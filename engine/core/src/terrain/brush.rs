// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Brushes (terrain.md §9): the descriptions, `ApplyBrush`, the bitmap
//! footprint with `ComputeSpansForRectangle`, and the component kinds 0
//! (raise or lower), 2 (talus smoothing), 5 (level) and 7 (mark).

use std::f64::consts::{FRAC_PI_2, PI};

use crate::x87::{ftol32, r24};

use super::brush_id;
use super::field::{Terrain, VertRect};
use super::flag;

/// A description with a component kind above this is rejected (§9.1).
pub const MAX_COMPONENT_KIND: u32 = 11;
/// Up to four components per brush (§9.1).
pub const ADD_BRUSH_MAX_COMPONENTS: usize = 4;

/// The random-walk angle step of a description without the property
/// `0x8A020102`: 0.5° (§9.1), stored `× π/180` as a float. Every shipped
/// brush has it (quirk 12).
pub const DEFAULT_WALK_STEP: f32 = (0.5 * PI / 180.0) as f32;

/// `2π` as the float the walk-angle wrap compares and adds (`6.2831855`,
/// §9.3 step 2.1).
pub const TWO_PI_F32: f32 = (2.0 * PI) as f32;

/// The default talus angle of brush kind 2 (`ReadTerrainIniFile`, section
/// `WindErosion`, key `TalusAngle`; the code default 0.79 rad, §9.4.1). The
/// shipped ini value is open (TR1).
pub const DEFAULT_TALUS_ANGLE: f32 = 0.79;

/// A brush bitmap: a grey image, `pixels[v × width + u]` (§9.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrushBitmap {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl BrushBitmap {
    /// `None` unless `pixels` has `width × height` entries and both are
    /// non-zero.
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Option<BrushBitmap> {
        (width > 0 && height > 0 && pixels.len() as u64 == u64::from(width) * u64::from(height))
            .then_some(BrushBitmap { width, height, pixels })
    }

    /// A `width × height` bitmap with every pixel `value`.
    pub fn uniform(width: u32, height: u32, value: u8) -> BrushBitmap {
        BrushBitmap {
            width,
            height,
            pixels: vec![value; (width * height) as usize],
        }
    }

    fn pixel(&self, u: u32, v: u32) -> u8 {
        self.pixels[v as usize * self.width as usize + u as usize]
    }
}

/// One brush component (§9.1, properties `0x8A020202 + 0x100·i + k`).
#[derive(Clone, Debug, PartialEq)]
pub struct BrushComponent {
    /// `+3`: 0-11 (§9.4).
    pub kind: u32,
    /// `+0`: one radius per brush level.
    pub radius: [f32; 5],
    /// `+1`: one strength per brush level.
    pub strength: [f32; 5],
    /// `−1`: orient the bitmap along the stroke.
    pub oriented: bool,
    /// `+2`: the neutral grey value.
    pub neutral: u8,
    /// `−2`: the bitmap, as the host decoded it. [choice] A component with
    /// no bitmap is skipped.
    pub bitmap: Option<BrushBitmap>,
}

/// A brush description (§9.1).
///
/// The original keeps the descriptions in one process-wide map, so the two
/// angles are shared by every caller and every city of the process (TR8).
/// Here each [`Terrain`] holds its own map; a host that switches cities
/// carries the angles across with [`Terrain::brush`] /
/// [`Terrain::set_brush_angles`].
#[derive(Clone, Debug, PartialEq)]
pub struct BrushDescription {
    pub components: Vec<BrushComponent>,
    /// The random-walk angle step `0x8A020102`, radians (the property is in
    /// degrees, `× π/180`; [`DEFAULT_WALK_STEP`] when it is missing). A step
    /// that is not 0 (NaN counts as not 0) sets the walk flag (`+0x04`).
    pub random_walk_step: f32,
    /// The walk angle (`+0x68`). With the walk flag the original starts it at
    /// `RandomRange(0, 2π)` of the terrain's own generator when it reads the
    /// description; the host draws that and stores it here. Without the flag
    /// it is 0.
    pub walk_angle: f32,
    /// The stroke angle (`+0x6C`), used by the oriented components. It starts
    /// at 0 (TR8).
    pub stroke_angle: f32,
}

impl BrushDescription {
    /// The walk flag (`+0x04`): the step is not 0 (NaN counts as not 0).
    pub fn walks(&self) -> bool {
        self.random_walk_step != 0.0
    }
}

/// `tBrushParameters` (§9.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushParams {
    pub from_x: f32,
    pub from_z: f32,
    pub to_x: f32,
    pub to_z: f32,
    /// 0-4; negative = the terrain's default level.
    pub level: i32,
    pub redisplay: bool,
    pub strength_scale: f32,
    pub radius_scale: f32,
    /// `data4` for `RedisplayTerrain`.
    pub data4: u32,
    /// Out: the touched vertex rectangle (`None` = empty).
    pub out: Option<VertRect>,
    /// The levelling components' target height.
    pub target_height: f32,
}

impl Default for BrushParams {
    /// The defaults of `0x004D8B90` (§9.2); the points are 0.
    fn default() -> Self {
        BrushParams {
            from_x: 0.0,
            from_z: 0.0,
            to_x: 0.0,
            to_z: 0.0,
            level: -1,
            redisplay: true,
            strength_scale: 1.0,
            radius_scale: 1.0,
            data4: super::kind::DISASTER,
            out: None,
            target_height: 100.0,
        }
    }
}

/// The shipped brushes the simulation applies (§9.1), with the bitmap the
/// host loaded. `None` for any other ID (the volcano brush's values are not
/// in the spec, TR2). None of them has the step property, so each walks with
/// the 0.5° default (quirk 12); the walk angle starts at 0 here and the host
/// sets its own draw with [`Terrain::set_brush_angles`].
pub fn shipped_brush(id: u32, bitmap: Option<BrushBitmap>) -> Option<BrushDescription> {
    let one = |radius: [f32; 5], strength: [f32; 5], neutral: u8| BrushDescription {
        components: vec![BrushComponent {
            kind: 0,
            radius,
            strength,
            oriented: true,
            neutral,
            bitmap: bitmap.clone(),
        }],
        random_walk_step: DEFAULT_WALK_STEP,
        walk_angle: 0.0,
        stroke_angle: 0.0,
    };
    match id {
        brush_id::EARTHQUAKE_FAULT | brush_id::EARTHQUAKE_FAULT_FLIPPED => Some(one([4.0; 5], [0.001; 5], 128)),
        brush_id::PIPE_SINK_HOLE => Some(one([16.0; 5], [0.01, 0.008, 0.006, 0.0045, 0.0035], 255)),
        _ => None,
    }
}

impl Terrain {
    /// Adds or replaces a brush description (`ReadBrushDescriptions`, §9.1).
    /// A description with more than four components or a component kind
    /// above 11 is rejected (false).
    pub fn add_brush(&mut self, id: u32, desc: BrushDescription) -> bool {
        if desc.components.len() > ADD_BRUSH_MAX_COMPONENTS
            || desc.components.iter().any(|c| c.kind > MAX_COMPONENT_KIND)
        {
            return false;
        }
        self.brushes.insert(id, desc);
        true
    }

    /// The description of a brush, if any.
    pub fn brush(&self, id: u32) -> Option<&BrushDescription> {
        self.brushes.get(&id)
    }

    /// Sets a description's walk angle (`+0x68`) and stroke angle (`+0x6C`):
    /// the host's `RandomRange(0, 2π)` start draw for the walk angle, or the
    /// values carried over from another terrain of the same process (the
    /// original's map is process-wide, TR8). False for an unknown brush.
    pub fn set_brush_angles(&mut self, id: u32, walk: f32, stroke: f32) -> bool {
        match self.brushes.get_mut(&id) {
            Some(d) => {
                d.walk_angle = walk;
                d.stroke_angle = stroke;
                true
            }
            None => false,
        }
    }

    /// The number of brush descriptions.
    pub fn brush_count(&self) -> usize {
        self.brushes.len()
    }

    /// `ApplyBrush(brushID, params)` (slot 57, §9.2). `random_uniform(n)` is
    /// the terrain generator's `RandomUint32Uniform(n)`: one draw of
    /// `RandomUint32Uniform(5)` per in-bounds application of a description
    /// with the walk flag (every shipped brush, quirk 12). Returns false for
    /// an unknown brush (the out rectangle stays empty and nothing is
    /// posted).
    pub fn apply_brush(
        &mut self,
        id: u32,
        params: &mut BrushParams,
        random_uniform: &mut dyn FnMut(u32) -> u32,
    ) -> bool {
        params.out = None;
        let Some(mut desc) = self.brushes.remove(&id) else {
            return false;
        };
        self.apply_components(&mut desc, params, random_uniform);
        self.brushes.insert(id, desc);
        if params.redisplay {
            if let Some(out) = params.out {
                self.redisplay_terrain(true, true, out, params.data4);
            }
        }
        true
    }

    /// `ApplyBrushComponents` (`0x0074D730`, §9.3).
    fn apply_components(
        &mut self,
        desc: &mut BrushDescription,
        params: &mut BrushParams,
        random_uniform: &mut dyn FnMut(u32) -> u32,
    ) {
        // Step 1: the centre vertex, by truncation; out of the vertices
        // nothing is applied and nothing is drawn.
        let vx = ftol32(r24(f64::from(params.to_x) * 0.0625));
        let vz = ftol32(r24(f64::from(params.to_z) * 0.0625));
        if !self.vertex_in_bounds(vx, vz) {
            return;
        }
        // Step 2.1: the walk angle, once per call whatever the kinds.
        if desc.walks() {
            let step = f64::from(desc.random_walk_step);
            let walk = f64::from(desc.walk_angle);
            let two_pi = f64::from(TWO_PI_F32);
            let walk = if random_uniform(5) != 0 {
                let w = r24(step + walk);
                if w >= two_pi {
                    r24(w - two_pi)
                } else {
                    w
                }
            } else {
                let w = r24(walk - step);
                if w < 0.0 {
                    r24(w + two_pi)
                } else {
                    w
                }
            };
            desc.walk_angle = walk as f32;
        } else {
            desc.walk_angle = 0.0;
        }
        // Step 2.2: the stroke angle; `from = to` keeps the previous one
        // *[quirk 6]*. A NaN coordinate counts as different.
        if params.from_x != params.to_x || params.from_z != params.to_z {
            let dz = r24(f64::from(params.to_z) - f64::from(params.from_z));
            let dx = r24(f64::from(params.to_x) - f64::from(params.from_x));
            desc.stroke_angle = r24(r24(dz.atan2(dx)) + f64::from(FRAC_PI_2 as f32)) as f32;
        }
        // Step 3: each component. [choice] A level above 4 (or a default
        // level outside 0-4) is clamped to 0-4.
        let level = if params.level < 0 {
            self.default_brush_level
        } else {
            params.level
        }
        .clamp(0, 4) as usize;
        let target = params.target_height;
        for comp in &desc.components {
            let Some(bmp) = comp.bitmap.as_ref() else {
                continue;
            };
            // Step 2.3: an oriented component draws with the stroke angle,
            // any other with the walk angle.
            let angle = if comp.oriented {
                desc.stroke_angle
            } else {
                desc.walk_angle
            };
            let mut strength = r24(f64::from(comp.strength[level]) * f64::from(params.strength_scale));
            let radius = r24(f64::from(comp.radius[level]) * f64::from(params.radius_scale));
            if comp.kind == 2 && strength > 1.0 {
                // Kind 2: the caller caps the strength at 1.0 (§9.4).
                strength = 1.0;
            }
            let changed = self.footprint(vx, vz, angle, radius, bmp, |t, i, b| match comp.kind {
                0 => t.raise_or_lower(i, b, comp.neutral, strength),
                2 => t.talus_smooth(i, b, strength),
                5 => t.level_toward(i, b, strength, target),
                7 => t.brush_mark(i),
                // TR1: kinds 1, 3, 4, 6 and 8-11 are god-mode only and not
                // specified; they change nothing.
                _ => false,
            });
            let mut include = |x: i32, z: i32| match params.out.as_mut() {
                Some(r) => r.include(x, z),
                None => params.out = Some(VertRect::point(x, z)),
            };
            for &(x, z) in &changed {
                include(x, z);
            }
            if comp.kind == 2 && !changed.is_empty() {
                // Kind 2 moves material to the neighbours: "the out
                // rectangle grows by 1" (§9.4). [choice] The changed
                // vertices' rectangle grown by 1, clipped to the vertices,
                // joins the out rectangle (TR19).
                let mut g = VertRect::point(changed[0].0, changed[0].1);
                for &(x, z) in &changed[1..] {
                    g.include(x, z);
                }
                let (mx, mz) = (self.cells_x as i32, self.cells_z as i32);
                include((g.x0 - 1).max(0), (g.z0 - 1).max(0));
                include((g.x1 + 1).min(mx), (g.z1 + 1).min(mz));
            }
        }
    }

    /// Kind 0 (`0x00745C60`, §9.4). Reports a change whenever `b ≠ neutral`,
    /// also when the vertex is locked or already at the clamp (TR15).
    fn raise_or_lower(&mut self, i: u32, b: u8, neutral: u8, strength: f64) -> bool {
        match b.cmp(&neutral) {
            std::cmp::Ordering::Greater => {
                let d = r24(f64::from(b - neutral) * strength) as f32;
                self.raise_vertex(i, d);
                true
            }
            std::cmp::Ordering::Less => {
                let d = r24(f64::from(neutral - b) * strength) as f32;
                self.lower_vertex(i, d);
                true
            }
            std::cmp::Ordering::Equal => false,
        }
    }

    /// The step of kinds 2 and 5: `r24(r24(b × strength) × (1/256))`
    /// (§9.4.1).
    fn grey_step(b: u8, strength: f64) -> f64 {
        r24(r24(f64::from(b) * strength) * (1.0 / 256.0))
    }

    /// Kind 5, level toward the target height (`0x00745BA0`, §9.4.1). `b = 0`,
    /// `alt = target` or a NaN difference: no change (false). Every other
    /// branch reports a change, also for a locked vertex.
    fn level_toward(&mut self, i: u32, b: u8, strength: f64, target: f32) -> bool {
        if b == 0 {
            return false;
        }
        let Some(ix) = self.idx(i) else {
            return false;
        };
        let step = Self::grey_step(b, strength);
        let d = r24(f64::from(self.alt[ix]) - f64::from(target));
        if d > 0.0 {
            if d <= step {
                self.set_altitude_at_vertex(i, target);
            } else {
                self.lower_vertex(i, step as f32);
            }
            true
        } else if d < 0.0 {
            if d < -step {
                self.raise_vertex(i, step as f32);
            } else {
                self.set_altitude_at_vertex(i, target);
            }
            true
        } else {
            false
        }
    }

    /// The two thresholds of kind 2 (§9.4.1): `Tedge := f32(tan(angle) ×
    /// 16)` and `Tdiag := r24(Tedge × √2)`. [choice] `√2` is the float
    /// `f32(sqrt(2))`, as in the lava's neighbour search (§11.3).
    pub fn talus_thresholds(&self) -> (f32, f32) {
        let edge = r24(r24(f64::from(self.talus_angle).tan()) * 16.0);
        let diag = r24(edge * f64::from(std::f32::consts::SQRT_2));
        (edge as f32, diag as f32)
    }

    /// Kind 2, talus smoothing (`0x00745DE0`, §9.4.1): for each neighbour in
    /// the offset order that the `GetNeighbors` mask allows, the drop beyond
    /// the threshold times the step moves from the vertex to the neighbour
    /// (each side skipped when locked; clamped to the allowed heights; the
    /// depth rewritten to `sea − alt`). Reports a change when any neighbour
    /// had an excess, also when both were locked.
    fn talus_smooth(&mut self, i: u32, b: u8, strength: f64) -> bool {
        if b == 0 {
            return false;
        }
        let Some(ix) = self.idx(i) else {
            return false;
        };
        let step = Self::grey_step(b, strength);
        let (edge, diag) = self.talus_thresholds();
        let (x, z) = self.vertex_of(i);
        let mask = self.neighbor_mask(x as i32, z as i32);
        let offsets = self.neighbor_offsets();
        let mut changed = false;
        for (k, off) in offsets.iter().enumerate() {
            if mask & (1 << k) == 0 {
                continue;
            }
            let Some(nx) = self.idx(i.wrapping_add(*off as u32)) else {
                continue;
            };
            let threshold = if k % 2 == 0 { edge } else { diag };
            let excess = r24(r24(f64::from(self.alt[ix]) - f64::from(self.alt[nx])) - f64::from(threshold));
            if !(excess > 0.0) {
                continue;
            }
            let mv = r24(excess * step);
            if self.flags[ix] & flag::LOCKED == 0 {
                let mut a = r24(f64::from(self.alt[ix]) - mv) as f32;
                if a < self.tuning.min_height {
                    a = self.tuning.min_height;
                }
                self.alt[ix] = a;
                self.water[ix].depth = r24(f64::from(self.sea_level) - f64::from(a)) as f32;
            }
            if self.flags[nx] & flag::LOCKED == 0 {
                let mut a = r24(f64::from(self.alt[nx]) + mv) as f32;
                if a > self.tuning.max_height {
                    a = self.tuning.max_height;
                }
                self.alt[nx] = a;
                self.water[nx].depth = r24(f64::from(self.sea_level) - f64::from(a)) as f32;
            }
            changed = true;
        }
        changed
    }

    /// Kind 7, mark (`0x00740820`, §9.4): sets flag `0x0400`. [choice] On
    /// every footprint vertex whatever its grey value, reported as a change
    /// (the spec gives no condition, TR20).
    fn brush_mark(&mut self, i: u32) -> bool {
        match self.idx(i) {
            Some(ix) => {
                self.flags[ix] |= flag::BRUSH_MARK;
                true
            }
            None => false,
        }
    }

    /// The four corners of the brush rectangle (`0x00740AB0`, §9.3): `w × h`
    /// metres centred on `(cx, cz)` and rotated by `(c, s) = (cos, sin)`, in
    /// the order `p0 = (a, e)`, `p1 = (a, −e)`, `p2 = (−a, −e)`, `p3 = (−a,
    /// e)` with `a = −w/2`, `e = −h/2`, each step rounded.
    pub fn brush_rectangle_corners(cx: f32, cz: f32, w: f32, h: f32, c: f32, s: f32) -> [(f32, f32); 4] {
        let (c, s) = (f64::from(c), f64::from(s));
        let a = r24(f64::from(w) * -0.5);
        let e = r24(f64::from(h) * -0.5);
        [(a, e), (a, -e), (-a, -e), (-a, e)].map(|(px, pz)| {
            let x = r24(r24(r24(c * px) - r24(s * pz)) + f64::from(cx));
            let z = r24(r24(r24(s * px) + r24(c * pz)) + f64::from(cz));
            (x as f32, z as f32)
        })
    }

    /// `ComputeSpansForRectangle` (`0x00740BB0`, §9.3, TR7): the scan
    /// conversion of the rotated rectangle. One `(z, start, end)` per row
    /// `zMin..=zMax`; the footprint visits `start..=end` (signed), so a row
    /// that no edge crosses (`start = i32::MAX`) is empty.
    pub fn compute_spans_for_rectangle(&self, corners: &[(f32, f32); 4]) -> Vec<(i32, i32, i32)> {
        let row = |z: f32| ftol32(r24(f64::from(z) * 0.0625));
        let mut z_min = corners.iter().map(|c| row(c.1)).min().unwrap_or(0);
        let mut z_max = corners.iter().map(|c| row(c.1)).max().unwrap_or(0);
        if z_min < 0 {
            z_min = 0;
        }
        // Unsigned: a negative maximum also becomes cellsZ.
        if (z_max as u32) > self.cells_z {
            z_max = self.cells_z as i32;
        }
        let mut out = Vec::new();
        for z in z_min..=z_max {
            let zz = f64::from(16 * z);
            let (mut start, mut end) = (i32::MAX, i32::MIN);
            for i in 0..4 {
                let (xi, zi) = (f64::from(corners[i].0), f64::from(corners[i].1));
                let (xj, zj) = (f64::from(corners[(i + 1) % 4].0), f64::from(corners[(i + 1) % 4].1));
                // Half-open: a horizontal edge or a NaN never crosses.
                if !((zi <= zz && zz < zj) || (zj <= zz && zz < zi)) {
                    continue;
                }
                let col = if xi == xj {
                    ftol32(r24(xi * 0.0625))
                } else {
                    let slope = r24(r24(xj - xi) / r24(zj - zi));
                    let x = r24(xi + r24(r24(zz - zi) * slope));
                    ftol32(r24(f64::from(ftol32(x)) * 0.0625))
                };
                start = start.min(col);
                end = end.max(col);
            }
            if start < 0 {
                start = 0;
            }
            end = end.wrapping_add(1);
            if (end as u32) > self.cells_x {
                end = self.cells_x as i32;
            }
            out.push((z, start, end));
        }
        out
    }

    /// The bitmap footprint (`0x00742180`, §9.3): runs `op(terrain, index,
    /// grey)` on each vertex of the spans whose bitmap pixel lies inside the
    /// bitmap and returns the vertices where it reported a change, in
    /// visiting order (rows ascending, x ascending). Truncation reaches half
    /// a pixel further on the low side *[quirk 7]*; `sin` and `cos` are not
    /// snapped (TR16).
    fn footprint(
        &mut self,
        vx: i32,
        vz: i32,
        angle: f32,
        radius: f64,
        bmp: &BrushBitmap,
        mut op: impl FnMut(&mut Terrain, u32, u8) -> bool,
    ) -> Vec<(i32, i32)> {
        let wpx = f64::from(bmp.width);
        let hpx = f64::from(bmp.height);
        let q = r24(radius / wpx);
        let k = r24(16.0 / q);
        let c = r24(f64::from(angle).cos());
        let s = r24(f64::from(angle).sin());
        let h = r24(hpx * q);
        let corners = Self::brush_rectangle_corners(
            r24(16.0 * f64::from(vx)) as f32,
            r24(16.0 * f64::from(vz)) as f32,
            radius as f32,
            h as f32,
            c as f32,
            s as f32,
        );
        let spans = self.compute_spans_for_rectangle(&corners);
        let ck = r24(c * k);
        let sk = r24(s * k);
        let mut out = Vec::new();
        for (z, start, end) in spans {
            if start > end {
                continue;
            }
            let dzk = r24(f64::from(z - vz) * k);
            let dxk = r24(f64::from(start - vx) * k);
            let mut u = r24(r24(r24(s * dzk) + r24(c * dxk)) + wpx / 2.0);
            let mut v = r24(r24(r24(c * dzk) - r24(s * dxk)) + hpx / 2.0);
            for x in start..=end {
                let pu = ftol32(u) as u32;
                let pv = ftol32(v) as u32;
                if pu < bmp.width && pv < bmp.height {
                    let b = bmp.pixel(pu, pv);
                    let i = self.vert_index(x, z);
                    if op(self, i, b) {
                        out.push((x, z));
                    }
                }
                u = r24(u + ck);
                v = r24(v - sk);
            }
        }
        out
    }
}
