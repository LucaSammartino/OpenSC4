// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled crash scalars and sinking transform, watercraft §5.7.2.
use super::{math, physics::rotate, Event, Publication, Terrain, Watercraft};
use crate::automata::Vec3;

/// Required city/terrain calls, kept distinct to preserve outside short circuit.
pub trait GroundingTerrain: Terrain {
    fn normal(&self, position: Vec3) -> Vec3;
    fn world_inside(&self, position: Vec3) -> bool;
    fn cell_height(&self, cell: super::Cell) -> f32;
}

pub trait CrashEffects {
    /// A missing required effect is an invalid precondition, not false recovery.
    fn refresh(&mut self, kind: CrashEffect, transform: &super::Transform);
    fn splash_present(&self) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrashEffect {
    Mushroom,
    Splash,
    Grounding,
}

/// Primitive effect interface; refresh ordering is computed by the core.
pub trait CrashInstance {
    fn set_transform(&mut self, transform: &super::Transform);
    fn running(&self) -> bool;
    fn start(&mut self, control: bool);
}

pub struct CrashEffectSet<'a> {
    pub mushroom: Option<&'a mut dyn CrashInstance>,
    pub splash: Option<&'a mut dyn CrashInstance>,
    pub grounding: Option<&'a mut dyn CrashInstance>,
}

impl CrashEffects for CrashEffectSet<'_> {
    fn refresh(&mut self, kind: CrashEffect, transform: &super::Transform) {
        let instance = match kind {
            CrashEffect::Mushroom => &mut self.mushroom,
            CrashEffect::Splash => &mut self.splash,
            CrashEffect::Grounding => &mut self.grounding,
        }
        .as_deref_mut()
        .expect("watercraft crash effect is a required precondition");
        instance.set_transform(transform);
        if !instance.running() {
            instance.start(false);
        }
    }

    fn splash_present(&self) -> bool {
        self.splash.is_some()
    }
}

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
        self.request_effect_update();
        self.request_model_update();
        true
    }

    /// Scalar phase only; use heel_tick for contact and publication.
    /// `shore_contact` is the byte after the preceding slide/contact phase.
    pub fn heel_angle_step(&mut self) -> bool {
        let t = self.crash_elapsed;
        let duration = self.drive.0[21];
        if t >= duration {
            return false;
        }
        // WC12: p[24] is the heel endpoint; p[25] only gates contact.
        let b = self.drive.0[24];
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

    pub fn heel_tick(&mut self, dt: f32, terrain: &dyn GroundingTerrain) -> bool {
        if self.crash_elapsed >= self.drive.0[21] {
            return false;
        }
        if self.shore_contact && self.crash_elapsed < self.drive.0[21] * 0.5 {
            let n = math::normalize(terrain.normal(self.position));
            let back = math::normalize(math::scale(self.heading, -1.0));
            let angle = math::dot(n, back).acos();
            if angle > self.drive.0[25] {
                self.speed *= self.drive.0[23];
                let mut banked = super::Transform::default();
                // Banking only: do not apply normal bow pitch or lift here.
                if self.speed > 0.0 {
                    let p = &self.drive.0;
                    let bank = ((self.speed / p[7]) * p[19] * self.turn).clamp(-p[20], p[20]);
                    banked.basis = banked.basis.map(|v| rotate(v, math::normalize(self.heading), bank));
                }
                // WC12: sample again into the old -H workspace. Neither the
                // incoming owner rotation nor the first normal supplies V.
                let second_normal = terrain.normal(self.position);
                let v = math::add(
                    math::add(
                        math::scale(banked.basis[0], second_normal[0]),
                        math::scale(banked.basis[1], second_normal[1]),
                    ),
                    math::scale(banked.basis[2], second_normal[2]),
                );
                let proposed = [
                    self.position[0] + dt * self.drive.0[4] * self.speed * self.heading[0],
                    self.position[1],
                    self.position[2] + dt * self.drive.0[4] * self.speed * self.heading[2],
                ];
                if terrain.world_inside(proposed) {
                    let h = terrain.cell_height(terrain.cell_at(proposed));
                    if (h - self.position[1]).abs() < self.drive.0[26] {
                        let mut c = self.transform.clone();
                        let f = math::normalize(self.heading);
                        let r = math::normalize(math::cross(v, f));
                        c.basis = [r, math::cross(f, r), f];
                        c.translation = [proposed[0], h, proposed[2]];
                        c.changed(1);
                        c.changed(4);
                        self.transform = c;
                        self.events
                            .push(Event::Publish(Publication::Transform(self.transform.clone())));
                        self.previous_position = self.position;
                        self.position = self.transform.translation;
                        self.inherited_heading = self.heading;
                        self.crash_publication_tail();
                    } else {
                        self.shore_contact = false;
                    }
                } else {
                    self.shore_contact = false;
                }
            } else {
                self.shore_contact = false;
            }
        }
        let old_angle = self.crash_angle;
        if !self.heel_angle_step() {
            return false;
        }
        let increment = self.crash_angle - old_angle;
        let n = terrain.normal(self.position);
        // rotate() is the earlier sinking convention; §5.7.5 explicitly uses
        // negative half angles, so negate the angle at this boundary.
        let up = math::normalize(rotate(n, math::normalize(self.previous_heading), -self.crash_angle));
        let heading = math::normalize(rotate(self.heading, [0.0, 1.0, 0.0], -increment));
        let f = math::normalize(heading);
        let r = math::normalize(math::cross(up, f));
        self.transform.basis = [r, math::cross(f, r), f];
        self.transform.translation = self.position;
        self.transform.changed(1);
        self.transform.changed(4);
        self.events
            .push(Event::Publish(Publication::Transform(self.transform.clone())));
        self.heading = heading;
        self.inherited_heading = heading;
        self.crash_publication_tail();
        true
    }

    fn crash_publication_tail(&mut self) {
        self.events.push(Event::Publish(Publication::Update(true, true)));
        self.request_effect_update();
        self.request_model_update();
    }
}

impl super::Fleet {
    /// DoCrashSequenceTick itself: state dispatch adds its first dt separately.
    pub fn crash_sequence_tick<T: GroundingTerrain>(
        &mut self,
        id: super::BoatId,
        dt: f32,
        rectangle: [super::Cell; 4],
        terrain: &T,
        effects: &mut dyn CrashEffects,
        manager: &mut Option<&mut dyn super::DockManager>,
    ) -> super::Result<bool> {
        if rectangle.into_iter().any(|cell| !terrain.contains(cell)) {
            self.push_state(id, crate::automata::State::new(super::state::IMMEDIATE_REMOVE), manager)?;
            return Ok(true);
        }
        let boat = self.boat_mut(id)?;
        let top = boat.top_state();
        if !matches!(top, Some(super::state::RUN_AGROUND | super::state::SINK)) {
            return Ok(true);
        }
        boat.crash_elapsed += dt;
        for key in ["SMLWake", "SMLWash", "MEDWake", "MEDWash", "LargeWake", "LargeWash"] {
            boat.events.push(Event::StopWake(key));
            boat.request_effect_update();
        }
        boat.player.speed_tween.retarget(0.0);
        let active = if top == Some(super::state::SINK) {
            if boat.sunk_by_collision {
                effects.refresh(CrashEffect::Mushroom, &boat.transform);
            }
            if effects.splash_present() {
                effects.refresh(CrashEffect::Splash, &boat.transform);
            }
            boat.sinking_tick(terrain)
        } else {
            effects.refresh(CrashEffect::Grounding, &boat.transform);
            boat.heel_tick(dt, terrain)
        };
        Ok(active)
    }

    pub fn crash_state_tick<T: GroundingTerrain>(
        &mut self,
        id: super::BoatId,
        dt: f32,
        rectangle: [super::Cell; 4],
        terrain: &T,
        effects: &mut dyn CrashEffects,
        manager: &mut Option<&mut dyn super::DockManager>,
    ) -> super::Result<bool> {
        self.boat_mut(id)?.crash_elapsed += dt;
        self.crash_sequence_tick(id, dt, rectangle, terrain, effects, manager)
    }
}
