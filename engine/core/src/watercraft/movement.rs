// SPDX-License-Identifier: GPL-3.0-or-later
use crate::automata::{State, Vec3};

use super::math;
use super::{state, BoatId, Cell, ContractGap, DockManager, Fleet, Result, Terrain, Watercraft};

pub(super) fn movement_state_allows(mut top: impl FnMut() -> Option<u32>) -> bool {
    top() != Some(state::RUN_AGROUND) && top() != Some(state::SINK)
}

impl Fleet {
    pub fn movement_possible(
        &mut self,
        id: BoatId,
        rectangle: [Cell; 4],
        terrain: &dyn Terrain,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<bool> {
        let boat = self.boat(id)?;
        if boat.record.base.automata_flags & 1 != 0 {
            return Ok(false);
        }
        if Watercraft::outside_city(rectangle, terrain) {
            self.push_state(id, State::new(state::IMMEDIATE_REMOVE), manager)?;
            return Ok(false);
        }
        let boat = self.boat(id)?;
        // §3.3 tests the top state twice, short-circuiting on 70, not FindState.
        Ok(movement_state_allows(|| boat.top_state()))
    }

    pub fn dock_requests(&mut self, id: BoatId, manager: &mut Option<&mut dyn DockManager>) -> Result<()> {
        let boat = self.boat(id)?;
        let Some(station) = boat.other_station() else {
            return Ok(());
        };
        if math::length(math::sub(boat.inherited_position, station.lot_position)) >= boat.drive.0[36] {
            return Ok(());
        }
        let queued = boat.top_state() == Some(state::QUEUE);
        let Some(service) = manager.as_deref_mut() else {
            return Ok(());
        };
        let next = if service.take_berth(&station.lot, id) {
            state::DOCK
        } else if queued {
            // WC7: literal queued branch has no fallback transition.
            return Ok(());
        } else if service.take_waiting_spot(&station.lot, id) {
            state::QUEUE
        } else {
            state::SLOW_REMOVE
        };
        if next != state::SLOW_REMOVE {
            let boat = self.boat_mut(id)?;
            for i in 0..2 {
                if boat.stations[i].as_ref().is_some_and(|s| s.lot == station.lot) {
                    boat.dock_requests[i] = true;
                }
            }
        }
        if queued && next == state::DOCK {
            self.pop_state(id, manager)?;
        }
        self.push_state(id, State::new(next), manager)
    }

    pub fn clear_dock_requests(&mut self, id: BoatId, manager: &mut Option<&mut dyn DockManager>) -> Result<()> {
        let boat = self.boat(id)?;
        if boat.held_berth
            && boat.current_station.as_ref().is_some_and(|station| {
                math::length(math::sub(boat.inherited_position, station.lot_position)) > boat.drive.0[36]
            })
        {
            let boat = self.boat_mut(id)?;
            boat.docking_in_progress = false;
            if let Some(service) = manager.as_deref_mut() {
                service.done_docking(&boat.current_station.as_ref().unwrap().lot, id);
                boat.held_berth = false;
                boat.record.base.automata_flags &= !0x1000_0000;
            }
        }
        Ok(())
    }

    /// Final approach only. Far-approach AI motion is explicitly WC4.
    /// A false return is consumed by the state dispatcher as PopState.
    pub fn docking_tick(
        &mut self,
        id: BoatId,
        dt: f32,
        terrain: &dyn Terrain,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<bool> {
        if self.boat(id)?.on_land(terrain) {
            self.push_state(id, State::new(state::IMMEDIATE_REMOVE), manager)?;
        }
        let boat = self.boat_mut(id)?;
        let station = boat.other_station().ok_or(ContractGap::MissingStationWc7)?;
        let distance = math::length(math::sub(station.position, boat.inherited_position));
        if boat.inherited_heading[1] != 0.0 {
            boat.inherited_heading = math::normalize(math::flat(boat.inherited_heading));
        }
        if distance > boat.drive.0[37] {
            return Err(ContractGap::AiMotionWc4);
        }
        let heading = math::normalize(boat.inherited_heading);
        let target = math::normalize(station.direction);
        let angle = math::dot(heading, target).acos().to_degrees();
        let angle = if angle.is_nan() { 0.0 } else { angle };
        if angle < 1.0 && distance < 1.0 {
            return Ok(false);
        }
        let amount = angle.min(dt * boat.drive.0[38]);
        let sign = super::sign(target[0] * heading[2] - target[2] * heading[0]);
        let heading = math::yaw(heading, sign * amount * 0.017_453_292);
        let position = if distance == 0.0 {
            boat.inherited_position
        } else {
            math::add(
                boat.inherited_position,
                math::scale(
                    math::sub(station.position, boat.inherited_position),
                    distance.min(dt * boat.drive.0[39]) / distance,
                ),
            )
        };
        boat.events
            .push(super::Event::Publish(super::Publication::MoveTo(position, heading)));
        boat.previous_position = boat.position;
        boat.previous_heading = boat.heading;
        boat.heading = heading;
        boat.position = position;
        boat.inherited_position = position;
        boat.inherited_heading = heading;
        Ok(true)
    }

    pub fn undocking_tick(
        &mut self,
        id: BoatId,
        dt: f32,
        rectangle: [Cell; 4],
        terrain: &dyn Terrain,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<bool> {
        if self.boat(id)?.on_land(terrain) {
            self.push_state(id, State::new(state::IMMEDIATE_REMOVE), manager)?;
        }
        if !self.movement_possible(id, rectangle, terrain, manager)? {
            return Ok(false);
        }
        let boat = self.boat_mut(id)?;
        let Some(route) = boat.neighbor.as_ref() else {
            return Ok(false);
        };
        if route.occupant.is_none() {
            return Ok(false);
        }
        if !terrain.navigable(terrain.cell_at(boat.position), 2, route.occupant.as_deref()) {
            return Ok(false);
        }
        let away = math::flat(math::sub(boat.position, route.position));
        let mut origin = route.position;
        origin[1] = terrain.water_height(origin);
        let distance = math::length(math::sub(boat.position, origin));
        if distance >= 2.0 * boat.radius {
            return Ok(false);
        }
        let direction = math::normalize(away);
        let amount = dt * boat.drive.0[39];
        let target = math::add(origin, math::scale(direction, 2.0 * boat.radius));
        let delta = math::flat(math::sub(target, boat.position));
        let length = math::length(delta);
        if length < 1.0 {
            return Ok(false);
        }
        let mut position = math::add(boat.position, math::scale(delta, amount.min(length) / length));
        position[1] = terrain.water_height(position);
        boat.previous_position = boat.position;
        boat.position = position;
        boat.inherited_position = position;
        boat.events.push(super::Event::Publish(super::Publication::MoveTo(
            position,
            boat.heading,
        )));
        Ok(true)
    }
}

impl Watercraft {
    pub fn clamped_frame_time(&self, dt: f32) -> f32 {
        // Literal min/max instead of f32::clamp: a negative configured upper
        // bound must not panic. The public contract assumes a valid limit.
        dt.max(0.0).min(self.drive.0[35])
    }

    /// Player drive's exact unpowered coasting substep (§6.1), using the
    /// unclamped first frame time; Movement_PositionNew clamps separately.
    pub fn coast(&mut self, dt: f32, throttle: f32, reverse: f32) {
        if throttle == -1.0 && reverse == -1.0 {
            let delta = dt * 0.001 * self.drive.0[3];
            if self.speed > 0.0 {
                self.speed = (self.speed - delta).max(0.0);
            } else if self.speed < 0.0 {
                self.speed = (self.speed + delta).min(0.0);
            }
        }
    }

    /// Gear selection is specified independently of the summarized speed and
    /// steering response. No AEMS set means the entire command is ignored.
    pub fn select_gear(&self, throttle: f32, reverse: f32) -> Option<(Gear, f32)> {
        self.sound_value()?;
        Some(if throttle >= self.drive.0[33] {
            (Gear::Forward, throttle)
        } else if reverse >= self.drive.0[32] {
            (Gear::Reverse, -reverse)
        } else {
            (Gear::Neutral, 0.0)
        })
    }

    /// State- and timer-only wake determination from §5.1. Effect execution
    /// remains a foreign resource operation, represented by ordered events.
    pub fn update_wakes(&mut self, dt: f32, city_cell_size: f32) {
        let moving = self.speed != 0.0
            || math::length(math::sub(self.position, self.previous_position)) >= 0.2 * city_cell_size
            || math::length(math::sub(self.heading, self.previous_heading)) >= 0.01;
        if moving {
            self.events.extend(super::WAKES.map(super::Event::StartWake));
            self.still_elapsed = 0.0;
        } else {
            self.still_elapsed += dt;
            if self.still_elapsed > 250.0 {
                for key in super::WAKES {
                    self.events.push(super::Event::StopWake(key));
                    self.events.push(super::Event::EffectUpdate);
                }
            }
        }
    }

    pub fn boat_crash_threshold_exceeded(&self, other_speed: f32) -> bool {
        let threshold = if self.top_state() == Some(state::PLAYER_DRIVE) {
            self.drive.0[27]
        } else {
            self.drive.0[15]
        };
        (self.speed - other_speed).abs() / self.drive.0[7] > threshold
    }

    pub fn shore_crash_threshold_exceeded(&self) -> bool {
        let threshold = if self.top_state() == Some(state::PLAYER_DRIVE) {
            self.drive.0[28]
        } else {
            self.drive.0[16]
        };
        self.speed.abs() / self.drive.0[7] > threshold
    }

    pub fn rudder_angle(&self) -> f32 {
        -0.125 * self.turn
    }

    /// Heading and the behind-point are pure geometry; the undocumented
    /// following translation rate remains WC4, not a native host input.
    pub fn following_target(&self, tug_position: Vec3, tug_heading: Vec3) -> Vec3 {
        math::sub(tug_position, math::scale(tug_heading, self.radius + 10.0))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gear {
    #[default]
    Neutral,
    Reverse,
    Forward,
}
