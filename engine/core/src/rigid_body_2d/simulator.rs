// SPDX-License-Identifier: GPL-3.0-or-later
//! particles.md §17: stable value handles and the settled noncontact integrator.
//! Finite admission/integration (§§7.3–7.5); unresolved whole collision cycles
//! remain explicit. Raw collision/list helpers are in the parent/storage modules.
use crate::effects::particle_owners::Unsupported;
use std::collections::BTreeMap;

pub const SECTOR_WIDTH: f32 = 64.;
pub const SECTOR_RECIPROCAL: f32 = 1. / 64.;
pub type BodyId = u64;

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalDescription {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub height: f32,
    pub height_velocity: f32,
    pub angle: f32,
    pub angular_velocity: f32,
    pub acceleration: [f32; 2],
    pub friction: bool,
    pub half_extents: [f32; 2],
    pub half_height: f32,
    pub response: f32,
    pub friction_rates: [f32; 3],
    /// Description +48..+5C, retained without interpretation.
    pub tail: [u32; 6],
}
#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub description: PhysicalDescription,
    pub axes: Option<[f32; 2]>,
    pub live: bool,
    pub static_body: bool,
    pub low_response: bool,
    pub angular_response: f32,
    pub bounds: [f32; 2],
    pub key: u32,
    pub spans: [u32; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct Joint {
    pub bodies: [BodyId; 2],
    pub anchors: [[f32; 2]; 2],
    pub distance: f32,
    pub breaking: f32,
}
#[derive(Clone, Debug, Default)]
pub struct SectorMembers {
    pub ordinary: Vec<BodyId>,
    pub static_bodies: Vec<BodyId>,
    pub low_response: Vec<BodyId>,
    pub total: u32,
}
#[derive(Clone, Debug, Default)]
pub struct Simulator {
    next: BodyId,
    pub objects: BTreeMap<BodyId, Object>,
    pub joints: Vec<Joint>,
    pub sectors: BTreeMap<u32, SectorMembers>,
    pub sector_events: Vec<(bool, BodyId, u32)>,
}

/// Same helper as rigid-body-2d.md §3, corrected by PT12.
pub fn primary_collision_sector(current: u32, a: u32, b: u32) -> bool {
    super::primary_sector(current, a, b)
}
/// §7.3 native conversion shortcut, including its integral-boundary correction.
pub fn sector_integer(q: f64, ceiling: bool) -> u32 {
    let shifted = q + if ceiling { 0.5 } else { -0.5 };
    let n = ((shifted + 12_582_912.) as f32).to_bits().wrapping_sub(0x4b40_0000);
    let correction = if ceiling { n.wrapping_sub(1) } else { n.wrapping_add(1) };
    if q == f64::from(correction as i32) {
        correction
    } else {
        n
    }
}
fn packed_key(position: [f32; 2], bounds: [f64; 2]) -> u32 {
    let x = sector_integer(
        (f64::from(position[0]) - bounds[0]) * f64::from(SECTOR_RECIPROCAL),
        false,
    );
    let y = sector_integer(
        (f64::from(position[1]) - bounds[1]) * f64::from(SECTOR_RECIPROCAL),
        false,
    );
    x.wrapping_shl(16).wrapping_add(y).wrapping_add(0x4000_4000)
}
pub fn sector_keys(key: u32, spans: [u32; 2]) -> Vec<u32> {
    let x = (key >> 16).wrapping_sub(0x4000);
    let y = (key & 0xffff).wrapping_sub(0x4000);
    let initial = x.wrapping_shl(16).wrapping_add(y).wrapping_add(0x4000_4000);
    (0..spans[0])
        .flat_map(|row| (0..spans[1]).map(move |col| initial.wrapping_add(row << 16).wrapping_add(col)))
        .collect()
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionPhase {
    Pair(BodyId, BodyId),
    Static(BodyId),
    Low(BodyId),
}
fn raw_body(body: &Object) -> Result<super::Body, Unsupported> {
    Ok(super::Body {
        position: body.description.position,
        velocity: body.description.velocity,
        angular_velocity: body.description.angular_velocity,
        extents: body.description.half_extents,
        axis: body.axes.ok_or(Unsupported::PT11)?,
    })
}
pub fn local_random(external: u32, clock: u32) -> crate::rng::RzRandom {
    let seed = (clock.wrapping_shl(16) | clock) ^ external;
    let mut random = crate::rng::RzRandom::new(
        0,
        &crate::rng::FixedSeeds {
            wall_clock: 0,
            process_clock: clock,
        },
    );
    random.set_state(seed, seed, None);
    random
}
/// §7.5 RandomDirection2, conditional on individual double range samples.
pub fn random_direction(draw: &mut impl FnMut() -> f64) -> Result<[f32; 2], Unsupported> {
    loop {
        let x = draw() as f32;
        let y = draw();
        let q = f64::from(x) * f64::from(x) + y * y;
        if q > 1. {
            continue;
        }
        if q == 0. {
            return Err(Unsupported::PT11);
        }
        let inverse = 1. / q.sqrt();
        return Ok([(f64::from(x) * inverse) as f32, (y * inverse) as f32]);
    }
}
/// §6.2 impulse, then §7.5's conditional coincident correction. Prior stores
/// survive an unsupported exceptional direction; no rollback is promised.
pub fn resolve(
    a: &mut Object,
    b: &mut Object,
    n: [f32; 2],
    point: [f32; 2],
    extra: [f32; 2],
    draw: &mut impl FnMut() -> f64,
) -> Result<(), Unsupported> {
    let relative = super::relative_velocity(&raw_body(b)?, &raw_body(a)?, point).map_err(|_| Unsupported::PT11)?;
    let offsets = |body: &Object| {
        [
            (f64::from(point[0]) - f64::from(body.description.position[0])) as f32,
            (-(f64::from(point[1]) - f64::from(body.description.position[1]))) as f32,
        ]
    };
    let oa = offsets(a).map(f64::from);
    let ob = offsets(b).map(f64::from);
    let n = n.map(f64::from);
    let ka = oa[0] * n[1] + oa[1] * n[0];
    let kb = ob[0] * n[1] + ob[1] * n[0];
    let ma = f64::from(a.description.response);
    let mb = f64::from(b.description.response);
    let ia = f64::from(a.angular_response);
    let ib = f64::from(b.angular_response);
    let v = std::array::from_fn::<_, 2, _>(|i| f64::from(relative[i]) + f64::from(extra[i]));
    let denominator = ((ma + mb) * (n[0] * n[0] + n[1] * n[1]) + (kb * ib) * kb) + (ka * ia) * ka;
    if denominator <= 0. || !denominator.is_finite() {
        return Err(Unsupported::PT11);
    }
    let j0 = (((v[0] * n[0] + v[1] * n[1]) * f64::from(f32::from_bits(0xbfb3_3333))) / denominator) as f32;
    let epsilon = f64::from(f32::from_bits(0x38d1_b717));
    let j = f64::from(j0) + if j0 >= 0. { epsilon } else { -epsilon };
    for (body, sign, mass) in [(&mut *a, 1., ma), (&mut *b, -1., mb)] {
        let x = (j * mass) * n[0];
        let y = ((j * mass) * n[1]) as f32;
        body.description.velocity[0] = (f64::from(body.description.velocity[0]) + sign * x) as f32;
        body.description.velocity[1] = (f64::from(body.description.velocity[1]) + sign * f64::from(y)) as f32;
    }
    a.description.angular_velocity = (f64::from(a.description.angular_velocity) + (ka * ia) * j) as f32;
    b.description.angular_velocity = (f64::from(b.description.angular_velocity) - (kb * ib) * j) as f32;
    let dx = f64::from(a.description.position[0]) - f64::from(b.description.position[0]);
    let dy = (f64::from(a.description.position[1]) - f64::from(b.description.position[1])) as f32;
    let length = (dx * dx + f64::from(dy) * f64::from(dy)).sqrt();
    let stored = length as f32;
    if length >= 4. {
        return Ok(());
    }
    let difference = if stored == 0. {
        random_direction(draw)?.map(f64::from)
    } else {
        [dx, f64::from(dy)]
    };
    let inverse = 1. / (difference[0] * difference[0] + difference[1] * difference[1]).sqrt();
    let normal = [difference[0] * inverse, f64::from((difference[1] * inverse) as f32)];
    let separation = difference.map(|v| ((4. - f64::from(stored)) * inverse * v) as f32);
    for (body, sign) in [(&mut *a, 1.), (&mut *b, -1.)] {
        for (i, value) in normal.iter().enumerate() {
            body.description.velocity[i] = (f64::from(body.description.velocity[i]) + sign * value) as f32;
        }
        for (i, value) in separation.iter().enumerate() {
            body.description.position[i] = (f64::from(body.description.position[i]) + sign * f64::from(*value)) as f32;
        }
    }
    Ok(())
}
impl Simulator {
    /// Input is the host hash iterator order, never sorted keys from BTreeMap.
    pub fn collision_phases(&self, keys: &[u32]) -> Vec<CollisionPhase> {
        let mut phases = Vec::new();
        for key in keys {
            let sector = &self.sectors[key];
            for (i, &a) in sector.ordinary.iter().enumerate() {
                for &b in &sector.ordinary[i + 1..] {
                    phases.push(CollisionPhase::Pair(a, b));
                }
                phases.push(CollisionPhase::Static(a));
                phases.push(CollisionPhase::Low(a));
            }
        }
        phases
    }
    pub fn update_joints(&mut self, draw: &mut impl FnMut() -> f64) -> Result<(), Unsupported> {
        let mut at = 0;
        while at < self.joints.len() {
            let joint = &self.joints[at];
            let [aid, bid] = joint.bodies;
            let a = &self.objects[&aid];
            let b = &self.objects[&bid];
            let anchor = |body: &Object, local: [f32; 2]| -> Result<[f32; 2], Unsupported> {
                let [c, s] = body.axes.ok_or(Unsupported::PT11)?.map(f64::from);
                let [x, y] = local.map(f64::from);
                Ok([
                    (c * x - s * y + f64::from(body.description.position[0])) as f32,
                    (s * x + c * y + f64::from(body.description.position[1])) as f32,
                ])
            };
            let pa = anchor(a, joint.anchors[0])?;
            let pb = anchor(b, joint.anchors[1])?;
            let d = std::array::from_fn::<_, 2, _>(|i| (f64::from(pb[i]) - f64::from(pa[i])) as f32);
            let length = (f64::from(d[0]).powi(2) + f64::from(d[1]).powi(2)).sqrt();
            if length <= f64::from(joint.distance) {
                at += 1;
                continue;
            }
            let point = std::array::from_fn(|i| ((f64::from(pa[i]) + f64::from(pb[i])) * 0.5) as f32);
            let relative =
                super::relative_velocity(&raw_body(a)?, &raw_body(b)?, point).map_err(|_| Unsupported::PT11)?;
            if f64::from(d[0]) * f64::from(relative[0]) + f64::from(d[1]) * f64::from(relative[1]) < 0. {
                at += 1;
                continue;
            }
            let speed = (f64::from(relative[0]).powi(2) + f64::from(relative[1]).powi(2)).sqrt();
            if speed > f64::from(f32::from_bits(0x3727_c5ac)) {
                let mut a = self.objects[&aid].clone();
                let mut b = self.objects[&bid].clone();
                let result = resolve(&mut a, &mut b, d, point, [0.; 2], draw);
                self.objects.insert(aid, a);
                self.objects.insert(bid, b);
                result?;
            }
            let weighted = |body: &Object| -> Result<[f32; 2], Unsupported> {
                let m = f64::from(body.description.response);
                let inertia = f64::from(body.angular_response);
                if m == 0. || inertia == 0. {
                    return Err(Unsupported::PT11);
                }
                let w = f64::from(body.description.angular_velocity) / inertia;
                Ok([
                    (f64::from(body.description.velocity[0]) / m
                        - w * (f64::from(point[1]) - f64::from(body.description.position[1])))
                        as f32,
                    (f64::from(body.description.velocity[1]) / m
                        + w * (f64::from(point[0]) - f64::from(body.description.position[0])))
                        as f32,
                ])
            };
            let va = weighted(&self.objects[&aid])?;
            let vb = weighted(&self.objects[&bid])?;
            let difference = std::array::from_fn::<_, 2, _>(|i| (f64::from(vb[i]) - f64::from(va[i])) as f32);
            let magnitude = (f64::from(difference[0]).powi(2) + f64::from(difference[1]).powi(2)).sqrt();
            if magnitude > f64::from(self.joints[at].breaking) {
                self.joints.remove(at);
            } else {
                at += 1;
            }
        }
        Ok(())
    }
    pub fn map_object(&mut self, id: BodyId, insert: bool) {
        let body = &self.objects[&id];
        let keys = sector_keys(body.key, body.spans);
        let flag75 = body.static_body;
        let flag74 = body.low_response;
        for key in keys {
            if insert {
                self.sectors.entry(key).or_default();
            }
            let Some(sector) = self.sectors.get_mut(&key) else {
                continue;
            };
            // §7.5 selects +75 first, then +74. Labels follow its vector table.
            let ids = if flag75 {
                &mut sector.low_response
            } else if flag74 {
                &mut sector.static_bodies
            } else {
                &mut sector.ordinary
            };
            if insert {
                ids.push(id);
                sector.total += 1;
            } else {
                if let Some(at) = ids.iter().position(|&member| member == id) {
                    ids.remove(at);
                }
                sector.total -= 1;
                if sector.total == 0 {
                    self.sectors.remove(&key);
                }
            }
        }
        self.sector_events.push((insert, id, body.key));
    }
    pub fn link(&mut self, joint: Joint) {
        self.joints.push(joint);
    }
    pub fn add_object(&mut self, description: &PhysicalDescription, static_body: bool) -> BodyId {
        let [a, b] = description.half_extents.map(f64::from);
        let denominator = b * b + a * a;
        let angular_response = ((f64::from(description.response) * 48.) / denominator) as f32;
        let low_response = !static_body && description.response < f32::from_bits(0x3727c5ac);
        let mut bounds = [0.; 2];
        let mut key = 0;
        let axes = static_body.then(|| {
            [
                f64::from(description.angle).cos() as f32,
                f64::from(description.angle).sin() as f32,
            ]
        });
        let spans = if let Some([c, s]) = axes {
            let x = a * f64::from(c).abs() + b * f64::from(s).abs();
            let y = a * f64::from(s).abs() + b * f64::from(c).abs();
            bounds = [x as f32, y as f32];
            key = packed_key(description.position, [f64::from(bounds[0]), y]);
            [
                sector_integer(f64::from(bounds[0]) * f64::from(SECTOR_RECIPROCAL) * 2., true),
                sector_integer(y * f64::from(SECTOR_RECIPROCAL) * 2., true),
            ]
        } else {
            let count = sector_integer(denominator.sqrt() * f64::from(SECTOR_RECIPROCAL), true);
            [count; 2]
        };
        self.next += 1;
        self.objects.insert(
            self.next,
            Object {
                description: description.clone(),
                axes,
                live: true,
                static_body,
                low_response,
                angular_response,
                bounds,
                key,
                spans,
            },
        );
        if static_body {
            self.map_object(self.next, true);
        }
        self.next
    }
    /// Valid retained body ID is a precondition, as for native RemoveObject.
    pub fn remove_object(&mut self, id: BodyId) {
        self.joints.retain(|joint| !joint.bodies.contains(&id));
        if self.objects[&id].key != 0 {
            self.map_object(id, false);
        }
        self.objects.get_mut(&id).expect("valid body handle").live = false;
    }
    pub fn remove_dead_objects(&mut self) {
        self.objects.retain(|_, body| body.live);
    }
    pub fn integrate_list(&mut self, low_response: bool, dt: f32) -> Result<(), Unsupported> {
        let ids: Vec<_> = self
            .objects
            .iter()
            .filter(|(_, b)| b.live && !b.static_body && b.low_response == low_response)
            .map(|(&id, _)| id)
            .collect();
        for id in ids {
            let body = self.objects.get_mut(&id).expect("live body");
            let d = &mut body.description;
            for i in 0..2 {
                d.position[i] = (f64::from(d.position[i]) + f64::from(dt) * f64::from(d.velocity[i])) as f32;
            }
            d.height = (f64::from(d.height) + f64::from(dt) * f64::from(d.height_velocity)) as f32;
            d.angle = (f64::from(d.angle) + f64::from(dt) * f64::from(d.angular_velocity)) as f32;
            if f64::from(d.angle).abs() > std::f64::consts::PI {
                d.angle = (f64::from(d.angle) % std::f64::consts::TAU) as f32;
            }
            let c = f64::from(d.angle).cos() as f32;
            let s = f64::from(d.angle).sin() as f32;
            body.axes = Some([c, s]);
            for i in 0..2 {
                d.velocity[i] = (f64::from(d.velocity[i]) + f64::from(dt) * f64::from(d.acceleration[i])) as f32;
            }
            if d.friction {
                let [c, s] = [c, s].map(f64::from);
                let [vx, vy] = d.velocity.map(f64::from);
                let u = (s * vy + c * vx) - f64::from(dt) * f64::from(d.friction_rates[0]);
                let v = (c * vy + (-s) * vx) - f64::from(dt) * f64::from(d.friction_rates[1]);
                let u = if u < 0. { 0. } else { u };
                let v = if v < 0. { 0. } else { v };
                let products = [(c * u) as f32, (s * u) as f32, ((-s) * v) as f32, (c * v) as f32];
                d.velocity = [
                    (f64::from(products[0]) + f64::from(products[2])) as f32,
                    (f64::from(products[1]) + f64::from(products[3])) as f32,
                ];
                let w = f64::from(d.angular_velocity) - f64::from(dt) * f64::from(d.friction_rates[2]);
                d.angular_velocity = if w < 0. { 0. } else { w as f32 };
            }
            let [a, b] = d.half_extents.map(f64::from);
            body.bounds = [
                (f64::from(s).abs() * b + f64::from(c).abs() * a) as f32,
                (f64::from(c).abs() * b + f64::from(s).abs() * a) as f32,
            ];
            let key = packed_key(d.position, body.bounds.map(f64::from));
            if key != body.key {
                if body.key != 0 {
                    self.map_object(id, false);
                }
                self.objects.get_mut(&id).expect("body").key = key;
                self.map_object(id, true);
            }
        }
        Ok(())
    }
    /// Only the no-contact domain is settled. Collision dispatch always follows
    /// both lists. Returning Unsupported does not erase prior integration.
    pub fn advance_time(&mut self, dt: f32) -> Result<(), Unsupported> {
        self.integrate_list(false, dt)?;
        self.integrate_list(true, dt)?;
        if self.objects.values().filter(|b| b.live).count() > 1 || !self.joints.is_empty() {
            return Err(Unsupported::PT11);
        }
        Ok(())
    }
}
