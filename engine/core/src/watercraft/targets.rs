// SPDX-License-Identifier: GPL-3.0-or-later
//! Watercraft §§5.2.1–5.2.2. Foreign inputs are cell centres and water level.
use super::{math, state, BoatId, DockManager, Event, Fleet, Publication, Result, Watercraft};
use crate::automata::{State, Vec3};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AiTargets {
    pub speed: f32,
    pub turn: f32,
    pub remove: bool,
}

impl Fleet {
    /// Complete picker-to-target path; no host-supplied target calculation.
    pub fn ai_motion_core(
        &mut self,
        id: BoatId,
        dt: f32,
        terrain: &dyn super::Terrain,
        mut centre: impl FnMut(super::Cell) -> Vec3,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<AiTargets> {
        if self.boat(id)?.travel.len() < 10 {
            let picked = self.pick_location(id, terrain, &mut centre, manager)?;
            let boat = self.boat_mut(id)?;
            boat.speed = 0.0;
            boat.turn = 0.0;
            if !picked {
                return Ok(AiTargets::default());
            }
        }
        // Refill has already executed in the owner. The shared midpoint path
        // only observes those points; its compatibility picker is a no-op.
        self.ai_motion(id, dt, terrain.sea_level(), |_| true, centre, manager)
    }

    /// The picker must be this core's route/search implementation; it is not a
    /// provider of a completed native motion result. Cell conversion is foreign.
    pub fn ai_motion(
        &mut self,
        id: BoatId,
        dt: f32,
        water: f32,
        mut picker: impl FnMut(&mut Watercraft) -> bool,
        mut centre: impl FnMut(super::Cell) -> Vec3,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<AiTargets> {
        let boat = self.boat_mut(id)?;
        if boat.travel.len() < 10 {
            let picked = picker(boat);
            boat.speed = 0.0;
            boat.turn = 0.0;
            if !picked {
                return Ok(AiTargets::default());
            }
        }
        if boat.travel.len() < 3 {
            return Ok(AiTargets::default());
        }
        let c = [
            centre(boat.travel[0].cell),
            centre(boat.travel[1].cell),
            centre(boat.travel[2].cell),
        ];
        let mid = [
            math::add(c[0], math::scale(math::sub(c[1], c[0]), 0.5)),
            math::add(c[1], math::scale(math::sub(c[2], c[1]), 0.5)),
        ];
        let targets = boat.ai_midpoints(dt, mid, water);
        if targets.remove {
            self.push_state(id, State::new(state::IMMEDIATE_REMOVE), manager)?;
        }
        Ok(targets)
    }
}

impl Watercraft {
    /// Selected targets are returned before slew, for observation of NaN and
    /// asymmetric turn signs. The midpoint scan never selects another segment.
    /// Requires at least three travel points, as checked by ai_motion.
    pub fn ai_midpoints(&mut self, dt: f32, mid: [Vec3; 2], water: f32) -> AiTargets {
        let delta = mid.map(|m| math::flat(math::sub(m, self.inherited_position)));
        let distance = delta.map(math::length);
        if distance.iter().any(|d| *d < 0.1) {
            self.travel.clear();
            return AiTargets {
                remove: true,
                ..AiTargets::default()
            };
        }
        let direction = delta.map(math::normalize);
        if distance[0] < 7.0 && self.angular_proximity(direction[1]) {
            self.ai_turn_latch = false;
            self.erase_travel_point(0);
            self.slew_ai_targets(dt, 0.0, 0.0);
            return AiTargets::default();
        }
        if self.inherited_heading[1].is_nan() {
            self.inherited_heading[1] = 0.0;
        }
        self.inherited_heading = math::normalize(self.inherited_heading);
        let mut target = AiTargets::default();
        let first_turn = !self.ai_turn_latch && !self.angular_proximity(direction[0]);
        let second_turn = !first_turn && distance[0] < 7.0 && !self.angular_proximity(direction[1]);
        if first_turn || second_turn {
            if second_turn {
                self.ai_turn_latch = true;
            }
            let u = direction[usize::from(second_turn)];
            let h = self.inherited_heading;
            let q = 0.1_f32 * dt;
            let angle = math::dot(u, h).acos() as f64 * 57.29578_f32 as f64;
            let limited = if angle < 0.0 {
                0.0
            } else if angle > q as f64 {
                q as f64
            } else {
                angle
            };
            let sign = if u[0] * h[2] - u[2] * h[0] < 0.0 { -1.0 } else { 1.0 };
            let signed = (limited * sign) as f32;
            let r = signed as f64 * 0.017_453_292_f32 as f64;
            let cosine = r.cos() as f32;
            self.inherited_heading = [
                (cosine as f64 * h[0] as f64 + r.sin() * h[2] as f64) as f32,
                h[1],
                (-r.sin() * h[0] as f64 + cosine as f64 * h[2] as f64) as f32,
            ];
            target.turn = ((if first_turn {
                signed.abs() as f64
            } else {
                limited * sign
            }) * self.drive.0[2] as f64
                / q as f64) as f32;
        } else if distance[0] >= 7.0 {
            let v = math::sub([mid[0][0], water, mid[0][2]], self.inherited_position);
            let length = math::length(v);
            let allowance = 0.0025_f32 * dt;
            let travel = if length < 0.0 {
                0.0
            } else if length > allowance {
                allowance
            } else {
                length
            };
            self.inherited_position = math::add(
                self.inherited_position,
                math::scale(math::scale(v, 1.0 / length), travel),
            );
            target.speed = (travel as f64 * self.drive.0[0] as f64 / allowance as f64) as f32;
        }
        self.inherited_heading = math::normalize(self.inherited_heading);
        self.events.push(Event::Publish(Publication::MoveTo(
            self.inherited_position,
            self.inherited_heading,
        )));
        self.previous_position = self.position;
        self.previous_heading = self.heading;
        self.position = self.inherited_position;
        self.heading = self.inherited_heading;
        self.slew_ai_targets(dt, target.speed, target.turn);
        target
    }
}
