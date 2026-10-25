// SPDX-License-Identifier: GPL-3.0-or-later
//! Watercraft §§3.2–3.4 and 5.1.1. No native owner calculation is a provider.
use super::{math, Event, Terrain, Watercraft};
use crate::automata::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    /// Column vectors: right, up, forward.
    pub basis: [Vec3; 3],
    pub translation: Vec3,
    pub scale: f32,
    pub flags: u32,
    pub revision: u8,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: [0.0; 3],
            scale: 1.0,
            flags: 0,
            revision: 0,
        }
    }
}

impl Transform {
    pub(super) fn changed(&mut self, flag: u32) {
        self.flags |= flag;
        self.revision = self.revision.wrapping_add(1);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Publication {
    Transform(Transform),
    MoveTo(Vec3, Vec3),
    Update(bool, bool),
}

/// Quaternion convention from §5.1.1 (negative cross product).
pub(super) fn rotate(v: Vec3, axis: Vec3, angle: f32) -> Vec3 {
    let (s, c) = (angle * 0.5).sin_cos();
    let [x, y, z] = math::scale(axis, s);
    let rows = [
        [
            1.0 - 2.0 * y * y - 2.0 * z * z,
            2.0 * x * y + 2.0 * c * z,
            2.0 * x * z - 2.0 * c * y,
        ],
        [
            2.0 * x * y - 2.0 * c * z,
            1.0 - 2.0 * x * x - 2.0 * z * z,
            2.0 * y * z + 2.0 * c * x,
        ],
        [
            2.0 * x * z + 2.0 * c * y,
            2.0 * y * z - 2.0 * c * x,
            1.0 - 2.0 * x * x - 2.0 * y * y,
        ],
    ];
    rows.map(|r| math::dot(r, v))
}

impl Watercraft {
    pub fn rear_endpoint(&self) -> Vec3 {
        math::sub(self.position, math::scale(self.heading, self.radius))
    }
    pub fn front_endpoint(&self) -> Vec3 {
        math::add(self.position, math::scale(self.heading, self.radius))
    }
    pub fn maximum_extent(bounds: [Vec3; 2]) -> f32 {
        let d = math::sub(bounds[1], bounds[0]);
        d[0].max(d[1]).max(d[2])
    }
    pub fn angular_proximity(&mut self, input: Vec3) -> bool {
        self.inherited_heading = math::normalize(self.inherited_heading);
        let d = math::dot(math::normalize(input), self.inherited_heading).clamp(-1.0, 1.0);
        (f64::from(d.acos()) * 57.295780181884766) < 2.0
    }
    pub fn erase_travel_point(&mut self, index: usize) -> usize {
        self.travel.remove(index);
        index
    }
    pub fn slew_ai_targets(&mut self, dt: f32, speed: f32, turn: f32) {
        fn approach(x: f32, target: f32, step: f32) -> f32 {
            if (target - x).abs() <= step {
                target
            } else {
                x + super::sign(target - x) * step
            }
        }
        let p = &self.drive.0;
        self.speed = approach(self.speed, speed, dt * p[13]).max(p[6]).min(p[7]);
        self.turn = approach(self.turn, turn, dt * p[14]).max(p[8]).min(p[9]);
    }

    /// Standalone straight integrator writes supplied outputs, not owner fields.
    pub fn integrate_straight(&self, dt: f32, transform: &mut Transform) -> Vec3 {
        let n = math::normalize(math::flat(self.heading));
        let distance = self.drive.0[4] * self.speed * dt;
        let position = math::add(self.position, math::scale(n, distance));
        transform.translation = position;
        transform.changed(1);
        position
    }

    fn integrate_curve(&mut self, dt: f32, transform: &mut Transform, terrain: &dyn Terrain) -> Vec3 {
        let p = &self.drive.0;
        let radius = if self.speed > 0.0 {
            p[11] * ((self.speed / p[7] + 0.5) + (1.0 - self.turn.abs() / p[9]))
        } else {
            p[12] * ((self.speed.abs() / p[6].abs() + 0.5) + (1.0 - self.turn.abs() / p[8].abs()))
        };
        let up = [0.0, 1.0, 0.0];
        let side = math::normalize(if self.turn > 0.0 {
            math::cross(up, self.heading)
        } else {
            math::cross(self.heading, up)
        });
        let mut center = math::add(self.position, math::scale(side, radius));
        center[1] = terrain.water_height(center);
        let radial = math::normalize(math::sub(self.position, center));
        let distance = p[4] * self.speed * dt;
        let theta = distance / radius * if self.turn > 0.0 { 1.0 } else { -1.0 };
        let position = math::add(center, math::scale(math::normalize(math::yaw(radial, theta)), radius));
        self.previous_heading = self.heading;
        self.heading = math::normalize(math::yaw(self.heading, theta));
        transform.translation = math::add(math::sub(position, self.position), self.position);
        transform.changed(1);
        if (position[2] - center[2]).abs() > 0.00001 {
            let _ = terrain.water_height(position);
        }
        position
    }

    pub fn apply_visual_attitude(&self, transform: &mut Transform) {
        if self.speed == 0.0 {
            return;
        }
        let p = &self.drive.0;
        if self.speed > 0.0 {
            let bank = ((self.speed / p[7]) * p[19] * self.turn).max(-p[20]).min(p[20]);
            let axis = math::normalize(self.heading);
            transform.basis = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(|v| rotate(v, axis, bank));
            transform.changed(4);
        }
        let b = math::normalize(math::scale(transform.basis[1], transform.scale));
        let axis = math::normalize(math::cross(self.heading, [0.0, 1.0, 0.0]));
        let bow = (p[17] * self.speed).max(0.0).min(p[18]);
        let f = math::normalize(math::normalize(rotate(self.heading, axis, -bow)));
        let r = math::normalize(math::cross(b, f));
        transform.basis = [r, math::cross(f, r), f];
        transform.changed(4);
        transform.translation[1] += p[18] / 2.0;
        transform.changed(1);
    }

    pub fn movement_position_new(&mut self, dt: f32, terrain: &dyn Terrain) {
        let p = &self.drive.0;
        self.turn = self.turn.max(p[8]).min(p[9]);
        self.speed = self.speed.max(p[6]).min(p[7]);
        let mut transform = self.transform.clone();
        let next = if self.speed == 0.0 {
            self.events.push(Event::FindState(16));
            self.position
        } else if self.turn == 0.0 {
            self.integrate_straight(dt, &mut transform)
        } else {
            self.integrate_curve(dt, &mut transform, terrain)
        };
        self.previous_position = self.position;
        self.position = next;
        if self.speed != 0.0 || self.turn != 0.0 {
            self.apply_visual_attitude(&mut transform);
        }
        self.transform = transform;
        self.publish_transform();
    }

    pub(super) fn publish_transform(&mut self) {
        self.events
            .push(Event::Publish(Publication::Transform(self.transform.clone())));
        self.inherited_position = self.position;
        self.inherited_heading = self.heading;
        self.events.push(Event::Publish(Publication::Update(true, true)));
        self.request_effect_update();
        self.request_model_update();
    }
}
