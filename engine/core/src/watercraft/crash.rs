// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled crash scalars and sinking transform, watercraft §5.7.2.
use super::{math, physics::rotate, Event, Publication, Terrain, Watercraft};

impl Watercraft {
    /// No frame-time factor: the ramp uses stored total crash elapsed.
    pub fn sinking_tick(&mut self, terrain: &dyn Terrain) -> bool {
        let t = self.crash_elapsed as f64;
        let duration = self.drive.0[21] as f64;
        let endpoint = self.drive.0[22];
        let old = self.crash_angle as f64;
        self.crash_angle = (((t / duration) * endpoint as f64 - old) + old) as f32;
        let angle = self.crash_angle;
        if angle >= endpoint {
            return false;
        }
        let previous = self.previous_heading;
        let y = [0.0, 1.0, 0.0];
        let roll = if self.crash_direction[1] == 0 { angle } else { -angle };
        let pitch = if self.crash_direction[0] == 0 { -angle } else { angle };
        let b = math::normalize(rotate(y, math::normalize(previous), roll));
        let axis = math::normalize(math::cross(previous, y));
        let f = math::normalize(math::normalize(rotate(previous, axis, pitch)));
        let r = math::normalize(math::cross(b, f));
        let u = math::cross(f, r);
        let h = self.heading;
        self.heading = math::normalize(math::add(
            math::add(math::scale(r, h[0]), math::scale(u, h[1])),
            math::scale(f, h[2]),
        ));
        self.transform.basis = [r, u, f];
        self.transform.changed(4);
        let threshold = 0.6_f32 * endpoint;
        if angle > threshold {
            let water = terrain.water_height(self.position);
            let height = terrain.terrain_height(self.position);
            let depth = ((water - height) * 0.5).min(0.0);
            let fraction = (angle - threshold) / threshold;
            self.transform.translation[1] = water - fraction * depth;
            self.transform.changed(1);
            self.previous_position = self.position;
            // Separate slot-29 sample and distinct runtime/transform heights.
            self.position[1] = terrain.water_height(self.position) - depth * (self.crash_elapsed / self.drive.0[21]);
        }
        self.events
            .push(Event::Publish(Publication::Transform(self.transform.clone())));
        self.inherited_heading = self.heading;
        self.events.push(Event::Publish(Publication::Update(true, true)));
        self.events.push(Event::EffectUpdate);
        self.events.push(Event::ModelUpdate);
        true
    }

    /// Scalar phase only. A true result still requires WC5's unresolved
    /// grounding/contact transform; callers must not treat it as publication.
    /// `shore_contact` is the byte after the preceding slide/contact phase.
    pub fn heel_angle_step(&mut self) -> bool {
        let t = self.crash_elapsed;
        let duration = self.drive.0[21];
        if t >= duration {
            return false;
        }
        let b = self.drive.0[25];
        let fraction = t as f64 / duration as f64;
        let target = if self.shore_contact && t < duration * 0.5 {
            fraction * b as f64
        } else {
            2.5 * b as f64 * fraction
        };
        let difference = (target - self.crash_angle as f64) as f32;
        self.crash_angle += difference;
        self.crash_angle < b
    }
}
