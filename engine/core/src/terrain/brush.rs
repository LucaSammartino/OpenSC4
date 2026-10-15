// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Brushes (terrain.md §9): the descriptions, `ApplyBrush` and the bitmap
//! footprint, with component kind 0 (raise or lower).

use std::f64::consts::{FRAC_PI_2, PI};

use crate::x87::{ftol32, r24};

use super::brush_id;
use super::field::{Terrain, VertRect};

/// A description with a component kind above this is rejected (§9.1).
pub const MAX_COMPONENT_KIND: u32 = 11;
/// Up to four components per brush (§9.1).
pub const ADD_BRUSH_MAX_COMPONENTS: usize = 4;

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
#[derive(Clone, Debug, PartialEq)]
pub struct BrushDescription {
    pub components: Vec<BrushComponent>,
    /// The random-walk angle step, radians (the property is in degrees,
    /// `× π/180`).
    pub random_walk_step: f32,
    /// The angle kept in the description (§9.3 step 2). When the step is not
    /// zero the original starts it at a random value in `[0, 2π)`; the host
    /// sets that start value here. [choice] Otherwise it starts at 0.
    pub angle: f32,
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
/// in the spec, TR2).
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
        random_walk_step: 0.0,
        angle: 0.0,
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

    /// `ApplyBrush(brushID, params)` (slot 57, §9.2). `random_uniform(n)` is
    /// `RandomUint32Uniform(n)`, used only by a brush with a random-walk step.
    /// Returns false for an unknown brush (the out rectangle stays empty and
    /// nothing is posted).
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

    /// `ApplyBrushComponents` (§9.3).
    fn apply_components(
        &mut self,
        desc: &mut BrushDescription,
        params: &mut BrushParams,
        random_uniform: &mut dyn FnMut(u32) -> u32,
    ) {
        // Step 1: the centre vertex, by truncation.
        let vx = ftol32(r24(f64::from(params.to_x) * 0.0625));
        let vz = ftol32(r24(f64::from(params.to_z) * 0.0625));
        if !self.vertex_in_bounds(vx, vz) {
            return;
        }
        // Step 2: the angle. [choice] It is decided once per application:
        // with any oriented component the stroke angle (kept in the
        // description; `from = to` reuses it *[quirk]*), else the random walk,
        // else 0; every component uses it.
        let angle = if desc.components.iter().any(|c| c.oriented) {
            if params.from_x != params.to_x || params.from_z != params.to_z {
                let dz = r24(f64::from(params.to_z) - f64::from(params.from_z));
                let dx = r24(f64::from(params.to_x) - f64::from(params.from_x));
                desc.angle = r24(dz.atan2(dx) + FRAC_PI_2) as f32;
            }
            desc.angle
        } else if desc.random_walk_step != 0.0 {
            let step = f64::from(desc.random_walk_step);
            let mut a = f64::from(desc.angle);
            a = if random_uniform(5) == 0 {
                r24(a - step)
            } else {
                r24(a + step)
            };
            let two_pi = r24(2.0 * PI);
            while a >= two_pi {
                a = r24(a - two_pi);
            }
            while a < 0.0 {
                a = r24(a + two_pi);
            }
            desc.angle = a as f32;
            desc.angle
        } else {
            0.0
        };
        // Step 3: each component. [choice] A level above 4 (or a default
        // level outside 0-4) is clamped to 0-4.
        let level = if params.level < 0 {
            self.default_brush_level
        } else {
            params.level
        }
        .clamp(0, 4) as usize;
        for comp in &desc.components {
            let Some(bmp) = comp.bitmap.as_ref() else {
                continue;
            };
            let strength = r24(f64::from(comp.strength[level]) * f64::from(params.strength_scale));
            let radius = r24(f64::from(comp.radius[level]) * f64::from(params.radius_scale));
            self.footprint(vx, vz, f64::from(angle), radius, bmp, |t, i, b| match comp.kind {
                0 => t.raise_or_lower(i, b, comp.neutral, strength),
                // TR1: kinds 1-11 are not specified; they change nothing.
                _ => false,
            })
            .into_iter()
            .for_each(|(x, z)| match params.out.as_mut() {
                Some(r) => r.include(x, z),
                None => params.out = Some(VertRect::point(x, z)),
            });
        }
    }

    /// Kind 0 (`0x00745C60`, §9.4). Reports a change whenever `b ≠ neutral`
    /// ([choice]: also when the vertex is locked or already at the clamp).
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

    /// The bitmap footprint (`0x00742180`, §9.3): runs `op(terrain, index,
    /// grey)` on each vertex whose bitmap pixel is inside the bitmap and
    /// returns the vertices where it reported a change, in visiting order.
    ///
    /// [choice] `ComputeSpansForRectangle` is not spelled out. The rows and
    /// spans here cover the axis-aligned bounding box of the rotated
    /// `radius × (H·radius / W)` rectangle, grown by one vertex plus one
    /// pixel on every side and clipped to the vertices (z outer from low to
    /// high, x from the low end of each row). The pixel test then decides,
    /// so the footprint is every vertex with `trunc(U) < W` and `trunc(V) <
    /// H` (unsigned), including the half pixel on the low side *[quirk]*.
    fn footprint(
        &mut self,
        vx: i32,
        vz: i32,
        angle: f64,
        radius: f64,
        bmp: &BrushBitmap,
        mut op: impl FnMut(&mut Terrain, u32, u8) -> bool,
    ) -> Vec<(i32, i32)> {
        let w = f64::from(bmp.width);
        let hgt = f64::from(bmp.height);
        let k = r24(16.0 / r24(radius / w));
        let mut out = Vec::new();
        if !k.is_finite() || k <= 0.0 {
            return out;
        }
        let c = r24(angle.cos());
        let s = r24(angle.sin());
        // The span box (vertex units).
        let hu = w / (2.0 * k);
        let hv = hgt / (2.0 * k);
        let margin = 1.0 + 1.0 / k;
        let ex = c.abs() * hu + s.abs() * hv + margin;
        let ez = s.abs() * hu + c.abs() * hv + margin;
        let lo = |centre: i32, e: f64| (f64::from(centre) - e).floor().max(0.0) as i32;
        let hi = |centre: i32, e: f64, n: u32| (f64::from(centre) + e).ceil().min(f64::from(n - 1)) as i32;
        let (x0, x1) = (lo(vx, ex), hi(vx, ex, self.verts_x));
        let (z0, z1) = (lo(vz, ez), hi(vz, ez, self.verts_z));
        let ck = r24(c * k);
        let sk = r24(s * k);
        for z in z0..=z1 {
            let dzk = r24(f64::from(z - vz) * k);
            let dxk = r24(f64::from(x0 - vx) * k);
            let mut u = r24(r24(r24(s * dzk) + r24(c * dxk)) + w / 2.0);
            let mut v = r24(r24(r24(c * dzk) - r24(s * dxk)) + hgt / 2.0);
            for x in x0..=x1 {
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
