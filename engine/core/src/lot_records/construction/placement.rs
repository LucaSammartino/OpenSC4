//! Construction-owned placement algorithms, lot-developer §6.7.7, .11–.13.
use super::*;

pub const CRANE_MODELS: [u32; 3] = [0x1C21_0000, 0x1C22_0000, 0x282A_0000];
pub const GROUND_TYPES: [u32; 5] = [0x0200_0000, 0x0201_0000, 0x0205_0000, 0x0304_0000, 0x0305_0000];
pub const ROOF_TYPES: [u32; 3] = [0x0303_0000, 0x0205_0000, 0x0206_0000];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CraneSize {
    pub width: f32,
    pub reach: f32,
    pub back: f32,
}

impl CraneSize {
    pub fn from_bounds(b: Bounds) -> Self {
        Self {
            width: b.1[0] - b.0[0],
            reach: b.0[0].abs().max(b.1[0].abs()),
            back: b.0[0].abs().min(b.1[0].abs()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropSize {
    pub type_id: u32,
    pub x: f32,
    pub z: f32,
}

/// Foreign resource/profile lookups, never placement decisions.
pub trait ConstructionSource {
    fn property(&mut self, id: u32) -> Option<f32>;
    /// Model type 0x5AD0E817, group 0xBADB57F1, interface 0x9AD0E7E3.
    fn crane_bounds(&mut self, instance: u32) -> Option<Bounds>;
    fn prop_dimensions(&mut self, type_id: u32) -> Option<[f32; 2]>;
}

impl ConstructionStatics {
    /// Call once for the process-owned statics, after base Init. `time` is
    /// plain cRZTime, without XOR; the caller owns the separate CRT RNG.
    pub fn load(source: &mut dyn ConstructionSource, time: u32, crt: CrtRand) -> Self {
        let tuning = ConstructionTuning::from_properties(|id| source.property(id));
        let cranes = CRANE_MODELS.map(|id| source.crane_bounds(id).map(CraneSize::from_bounds).unwrap_or_default());
        let mut props = |ids: &[u32]| {
            ids.iter()
                .filter_map(|&type_id| source.prop_dimensions(type_id).map(|[x, z]| PropSize { type_id, x, z }))
                .collect()
        };
        let seeds = crate::rng::FixedSeeds {
            wall_clock: time,
            process_clock: time,
        };
        Self {
            tuning,
            rng: RzRandom::new(time, &seeds),
            crt,
            cranes,
            ground: props(&GROUND_TYPES),
            roof: props(&ROOF_TYPES),
            girder_plank: None,
            crane_post_half: None,
        }
    }
}

fn area(x0: f32, z0: f32, x1: f32, z1: f32) -> Bounds {
    ([x0, 0.0, z0], [x1, 1.0, z1])
}

/// `world` is the city conversion of (x1,z1) and (x2+1,z2+1).
pub fn divide_lot(world: Bounds, building: Bounds, gap: f32) -> [Bounds; 8] {
    let (lo, hi) = world;
    let mut xa = building.0[0] - gap;
    let mut xb = building.1[0] + gap;
    let mut za = building.0[2] - gap;
    let mut zb = building.1[2] + gap;
    // Literal ordered comparisons preserve a NaN building side (§6.7.11).
    if xa < lo[0] {
        xa = lo[0];
    }
    if xb > hi[0] {
        xb = hi[0];
    }
    if za < lo[2] {
        za = lo[2];
    }
    if zb > hi[2] {
        zb = hi[2];
    }
    [
        area(xa, lo[2], xb, za),
        area(xb, za, hi[0], zb),
        area(xa, zb, xb, hi[2]),
        area(lo[0], za, xa, zb),
        area(lo[0], lo[2], xa, za),
        area(xb, lo[2], hi[0], za),
        area(xb, zb, hi[0], hi[2]),
        area(lo[0], zb, xa, hi[2]),
    ]
}

pub fn remove_rectangle(a: &mut Bounds, r: Bounds) {
    let candidates = [
        area(a.0[0], a.0[2], a.1[0], r.0[2]),
        area(r.1[0], a.0[2], a.1[0], a.1[2]),
        area(a.0[0], r.1[2], a.1[0], a.1[2]),
        area(a.0[0], a.0[2], r.0[0], a.1[2]),
    ];
    let mut best = 0.0;
    let mut result = ([0.0; 3], [0.0; 3]);
    for mut c in candidates {
        c.0[1] = a.0[1];
        c.1[1] = a.1[1];
        let product = (c.1[0] - c.0[0]) * (c.1[2] - c.0[2]);
        if product > best {
            best = product;
            result = c;
        }
    }
    *a = result;
}

pub fn swing_limits(world: Bounds, position: [f32; 3], side: usize, reach: f32) -> (f32, f32) {
    let axis = if side == 0 || side == 2 { 0 } else { 2 };
    let d = (world.1[axis] - position[axis]).min(position[axis] - world.0[axis]);
    use std::f32::consts::{FRAC_PI_2, PI, TAU};
    if d >= reach {
        return match side {
            0 => (PI, TAU),
            2 => (0.0, PI),
            3 => (3.0 * FRAC_PI_2, 5.0 * FRAC_PI_2),
            _ => (FRAC_PI_2, 3.0 * FRAC_PI_2),
        };
    }
    let centre = match side {
        0 => 1.5 * PI,
        2 => FRAC_PI_2,
        3 => TAU,
        _ => PI,
    };
    let half = (f64::from(d) / f64::from(reach)).asin() as f32;
    (centre - half, centre + half)
}

pub fn outer_crane(areas: &mut [Bounds; 8], local_height: f32, st: &mut ConstructionStatics) -> Option<CraneFit> {
    let world = (areas[4].0, areas[6].1);
    let (xa, za, xb, zb) = (areas[0].0[0], areas[0].1[2], areas[2].1[0], areas[2].0[2]);
    for (k, size) in st.cranes.iter().enumerate() {
        if size.width == 0.0 || !((0.8 * size.reach <= xb - xa || zb - za >= 12.0) && local_height >= 12.0) {
            continue;
        }
        let reach_room = [world.1[2] - za, xb - world.0[0], zb - world.0[2], world.1[0] - xa];
        let need = st.tuning.crane_post_gap + size.back + st.tuning.crane_post_width;
        let sides: Vec<_> = (0..4)
            .filter(|&s| {
                let axis = if s % 2 == 0 { 2 } else { 0 };
                reach_room[s] >= size.reach && areas[s].1[axis] - areas[s].0[axis] > need
            })
            .collect();
        if sides.is_empty() {
            continue;
        }
        let side = sides[st.rng.uniform_u32(sides.len() as u32) as usize];
        let f = st.tuning.crane_post_width * 0.5 + st.tuning.crane_post_gap;
        let p = match side {
            0 => [(xa + xb) * 0.5, 0.0, za - f],
            1 => [xb + f, 0.0, (za + zb) * 0.5],
            2 => [(xa + xb) * 0.5, 0.0, zb + f],
            _ => [xa - f, 0.0, (za + zb) * 0.5],
        };
        let (swing_min, swing_max) = swing_limits(world, p, side, size.reach);
        remove_rectangle(
            &mut areas[side],
            area(p[0] - need, p[2] - need, p[0] + need, p[2] + need),
        );
        return Some(CraneFit {
            crane_type: k as u8,
            position: p,
            swing_min,
            swing_max,
        });
    }
    None
}

/// Inner side draw and strict model fit (§6.7.15).
pub fn inner_crane_choice(b: Bounds, sizes: &[CraneSize; 3], rng: &mut RzRandom) -> (usize, Option<u8>) {
    let raw = rng.next_raw();
    let x_long = b.1[0] - b.0[0] >= b.1[2] - b.0[2];
    let side = if x_long { 0 } else { 1 } + if raw < 0x7FFF_FFFF { 0 } else { 2 };
    let axis = if x_long { 0 } else { 2 };
    (
        side,
        sizes
            .iter()
            .position(|s| s.width > 0.0 && s.back + s.reach < b.1[axis] - b.0[axis])
            .map(|k| k as u8),
    )
}

/// Absolute model rotation (§6.7.15); equality and negative angles retained.
pub fn crane_model_angle(angle: f32) -> f32 {
    let mut a = angle + std::f32::consts::PI;
    while a > std::f32::consts::TAU {
        a -= std::f32::consts::TAU;
    }
    a
}

pub fn inner_crane_pose(b: Bounds, size: CraneSize, side: usize) -> ([f32; 3], f32, f32) {
    use std::f32::consts::{FRAC_PI_2, PI, TAU};
    let offset = (f64::from(size.back) + f64::from(size.reach)) * 0.5 - f64::from(size.back);
    let mut p = [
        ((f64::from(b.0[0]) + f64::from(b.1[0])) * 0.5) as f32,
        0.,
        ((f64::from(b.0[2]) + f64::from(b.1[2])) * 0.5) as f32,
    ];
    let axis = if side == 0 || side == 2 { 0 } else { 2 };
    p[axis] = ((f64::from(b.0[axis]) + f64::from(b.1[axis])) * 0.5
        + if side == 0 || side == 3 { -offset } else { offset }) as f32;
    let q = ((f64::from(b.1[2 - axis]) - f64::from(b.0[2 - axis])) * 0.5) as f32;
    let (lo, hi) = if q >= size.reach {
        match side {
            0 => (3. * FRAC_PI_2, 5. * FRAC_PI_2),
            2 => (FRAC_PI_2, 3. * FRAC_PI_2),
            1 => (0., PI),
            _ => (PI, TAU),
        }
    } else {
        let c = f64::from(match side {
            0 => TAU,
            2 => PI,
            1 => FRAC_PI_2,
            _ => 3. * FRAC_PI_2,
        });
        let a = (f64::from(q) / f64::from(size.reach)).asin();
        ((c - a) as f32, (c + a) as f32)
    };
    (p, lo, hi)
}

/// Centre anchor and terrain fallback (§6.7.15). Corner heights use ordered comparisons.
pub fn prop_anchor(
    b: Bounds,
    footprint: [f32; 2],
    offsets: [f32; 2],
    building: Option<[f32; 3]>,
    mut altitude: impl FnMut(f32, f32) -> f32,
) -> ([f32; 3], Bounds) {
    let [fx, fz] = footprint;
    let [x, z] = offsets;
    let hx = (f64::from(fx) * 0.5) as f32;
    let hz = (f64::from(fz) * 0.5) as f32;
    let cx = (f64::from(b.0[0]) + f64::from(x) + f64::from(hx)) as f32;
    let cz = (f64::from(b.0[2]) + f64::from(z) + f64::from(hz)) as f32;
    let cut = ([cx - hx, b.0[1], cz - hz], [cx + hx, b.1[1], cz + hz]);
    let y = if let Some(p) = building {
        p[1]
    } else {
        let mut y = altitude(cut.0[0], cut.0[2]);
        for (x, z) in [(cut.1[0], cut.0[2]), (cut.0[0], cut.1[2]), (cut.1[0], cut.1[2])] {
            let next = altitude(x, z);
            if next > y {
                y = next;
            }
        }
        y
    };
    ([cx, y, cz], cut)
}

impl ConstructionOccupant {
    pub(super) fn attempt_crane(&mut self, host: &mut dyn ConstructionHost, st: &mut ConstructionStatics, b: Bounds) {
        let policy = host.crane_policy(&self.rec.building);
        let fit = if policy == 1 || policy == 3 {
            outer_crane(
                &mut self.areas,
                self.rec.local_bounds.1[1] - self.rec.local_bounds.0[1],
                st,
            )
        } else {
            None
        };
        if let Some(fit) = fit {
            self.rec.scalars.crane_type = fit.crane_type;
            self.rec.scalars.swing_min = fit.swing_min;
            self.rec.scalars.swing_max = fit.swing_max;
            self.create_crane(host, st, fit.position[0], fit.position[2], fit.crane_type);
        } else if policy == 2 || policy == 3 {
            let (side, size) = inner_crane_choice(b, &st.cranes, &mut st.rng);
            if let Some(kind) = size {
                let (p, lo, hi) = inner_crane_pose(b, st.cranes[usize::from(kind)], side);
                self.rec.scalars.crane_type = kind;
                self.rec.scalars.swing_min = lo;
                self.rec.scalars.swing_max = hi;
                self.create_crane(host, st, p[0], p[2], kind);
            }
        }
    }

    fn place_prop(
        &mut self,
        host: &mut dyn ConstructionHost,
        st: &mut ConstructionStatics,
        b: &mut Bounds,
        p: PropSize,
        roof: bool,
    ) {
        // §6.7.7: rejection precedes all three draws.
        let Some(prop) = host.create_prop(p.type_id) else {
            return;
        };
        let w = b.1[0] - b.0[0];
        let d = b.1[2] - b.0[2];
        let orientation = if w > p.z && d > p.x {
            [0, 2, 1, 3][st.rng.uniform_u32(4) as usize]
        } else {
            [0, 2][st.rng.uniform_u32(2) as usize]
        };
        let (fx, fz) = if orientation % 2 == 0 { (p.x, p.z) } else { (p.z, p.x) };
        let x = st.rng.range_f64(0.0, f64::from(w - fx)) as f32;
        let z = st.rng.range_f64(0.0, f64::from(d - fz)) as f32;
        let (position, footprint) = prop_anchor(
            *b,
            [fx, fz],
            [x, z],
            host.lot_developer_building_position(&self.rec.lot),
            |x, z| host.altitude(x, z),
        );
        host.place_prop(&prop, position, orientation);
        host.associate_prop(&prop, &self.rec.lot, roof);
        remove_rectangle(b, footprint);
        if roof {
            self.rec.roof_props.push(prop);
        } else {
            self.rec.ground_props.push(prop);
        }
    }

    pub(super) fn ground_props(&mut self, host: &mut dyn ConstructionHost, st: &mut ConstructionStatics) {
        if st.ground.is_empty() {
            return;
        }
        let mut count = st.rng.range_i32(1, 4);
        while count > 0 {
            let mut attempted = false;
            for a in reset_iterator(&mut st.crt, 8) {
                let b = self.areas[a];
                let w = b.1[0] - b.0[0];
                let d = b.1[2] - b.0[2];
                if !(w > 0.0 && d > 0.0) {
                    continue;
                }
                st.crt.random_shuffle(&mut st.ground);
                if let Some(p) = st.ground.iter().copied().find(|p| p.x < w && p.z < d) {
                    let mut remaining = b;
                    self.place_prop(host, st, &mut remaining, p, false);
                    self.areas[a] = remaining;
                    count -= 1;
                    attempted = true;
                    break;
                }
            }
            if !attempted {
                break;
            }
        }
    }

    pub(super) fn roof_props(&mut self, host: &mut dyn ConstructionHost, st: &mut ConstructionStatics) {
        let mut b = self.rec.local_bounds;
        let product = (b.1[0] - b.0[0]) * (b.1[2] - b.0[2]);
        let mut count = st.rng.range_i32(0, 1) + i32::from(product > 256.0) + 2 * i32::from(product > 1024.0);
        while count > 0 {
            st.crt.random_shuffle(&mut st.roof);
            let Some(p) = st
                .roof
                .iter()
                .copied()
                .find(|p| p.x < b.1[0] - b.0[0] && p.z < b.1[2] - b.0[2])
            else {
                break;
            };
            self.place_prop(host, st, &mut b, p, true);
            count -= 1;
        }
    }

    fn create_worker(
        &mut self,
        host: &mut dyn ConstructionHost,
        st: &mut ConstructionStatics,
        b: Bounds,
        roof: bool,
        type_id: u32,
    ) {
        let mut p = b.0;
        for axis in [0, 2] {
            let lo = b.0[axis] + 0.5;
            let hi = (b.1[axis] - 0.5) - 1.0;
            p[axis] = st.rng.range_f64(f64::from(lo), f64::from(hi)) as f32;
        }
        p[1] = host.altitude(p[0], p[2]);
        if let Some(automaton) = host.create_automaton(type_id, p, roof) {
            let x = st.rng.range_f64(-1.0, 1.0) as f32;
            let z = st.rng.range_f64(-1.0, 1.0) as f32;
            host.worker_direction(&automaton, normalized_direction(x, z));
            self.rec.crew.push(CrewEntry {
                first: false,
                second: roof,
                automaton,
            });
        }
    }

    pub(super) fn crew(&mut self, host: &mut dyn ConstructionHost, st: &mut ConstructionStatics) {
        if host.render_detail() <= 2 {
            return;
        }
        let count = st.rng.range_i32(2, 4);
        for _ in 0..count {
            let area = reset_iterator(&mut st.crt, 8)
                .into_iter()
                .map(|i| self.areas[i])
                .find(|b| b.1[0] - b.0[0] > 2.0 && b.1[2] - b.0[2] > 2.0);
            let Some(b) = area else {
                break;
            };
            let type_id = [0x0210_B000, 0x0210_C000, 0x0210_A000][st.rng.uniform_u32(3) as usize];
            self.create_worker(host, st, b, false, type_id);
        }
        let count = st.rng.uniform_u32(2);
        for _ in 0..=count {
            let b = self.rec.local_bounds;
            if b.1[0] - b.0[0] <= 2.0 || b.1[2] - b.0[2] <= 2.0 {
                break;
            }
            let type_id = [0x0210_9000, 0x0210_D000][st.rng.uniform_u32(2) as usize];
            self.create_worker(host, st, b, true, type_id);
        }
    }
}
