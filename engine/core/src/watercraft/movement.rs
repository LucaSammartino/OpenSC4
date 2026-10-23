// SPDX-License-Identifier: GPL-3.0-or-later
use crate::automata::{State, Vec3};

use super::math;
use super::{state, BoatId, Cell, ContractGap, DockManager, Fleet, Result, Terrain, Watercraft};

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
        if rectangle.into_iter().any(|cell| !terrain.contains(cell)) {
            self.push_state(id, State::new(state::IMMEDIATE_REMOVE), manager)?;
            return Ok(false);
        }
        let boat = self.boat(id)?;
        Ok(!matches!(boat.top_state(), Some(state::RUN_AGROUND | state::SINK)))
    }

    pub fn dock_requests(&mut self, id: BoatId, manager: &mut Option<&mut dyn DockManager>) -> Result<()> {
        let boat = self.boat(id)?;
        let Some(station) = boat.other_station() else {
            return Ok(());
        };
        if math::length(math::sub(boat.position, station.position)) > boat.drive.0[36] {
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
            && boat
                .current_station
                .as_ref()
                .is_some_and(|station| math::length(math::sub(boat.position, station.position)) > boat.drive.0[36])
        {
            // §5.3 clears only these two flags; unlike EndDocking, it does
            // not say to clear docking_in_progress.
            let boat = self.boat_mut(id)?;
            if let Some(service) = manager.as_deref_mut() {
                service.done_docking(&boat.current_station.as_ref().unwrap().lot, id);
            }
            boat.held_berth = false;
            boat.record.base.automata_flags &= !0x1000_0000;
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
            // WC7: the on-land return value is not published.
            return Err(ContractGap::DockingReturnWc7);
        }
        let boat = self.boat_mut(id)?;
        let station = boat.other_station().ok_or(ContractGap::MissingStationWc7)?;
        let distance = math::length(math::sub(station.position, boat.position));
        let heading = math::unit(math::flat(boat.heading)).ok_or(ContractGap::DegenerateDirection)?;
        if distance > boat.drive.0[37] {
            return Err(ContractGap::AiMotionWc4);
        }
        let target = math::unit(math::flat(station.direction)).ok_or(ContractGap::DegenerateDirection)?;
        let angle = math::angle(heading, target).ok_or(ContractGap::DegenerateDirection)?;
        if angle.to_degrees() < 1.0 && distance < 1.0 {
            boat.heading = heading;
            return Ok(false);
        }
        let heading = math::turn(heading, target, (dt * boat.drive.0[38]).to_radians())
            .ok_or(ContractGap::DegenerateDirection)?;
        let position = math::approach(boat.position, station.position, dt * boat.drive.0[39]);
        boat.heading = heading;
        boat.position = position;
        // WC4: literal "docked when" is evaluated after moving/turning too;
        // the specification does not give the native instruction order.
        let remaining_angle = math::angle(heading, target).ok_or(ContractGap::DegenerateDirection)?;
        let remaining_distance = math::length(math::sub(position, station.position));
        Ok(remaining_angle.to_degrees() >= 1.0 || remaining_distance >= 1.0)
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
            return Err(ContractGap::UndockBlockedWc7);
        }
        if !self.movement_possible(id, rectangle, terrain, manager)? {
            return Err(ContractGap::UndockBlockedWc7);
        }
        let boat = self.boat_mut(id)?;
        let route = boat.neighbor.as_ref().ok_or(ContractGap::MissingRouteWc7)?;
        if !terrain.navigable(terrain.cell_at(boat.position), 2, Some(&route.occupant)) {
            return Err(ContractGap::UndockBlockedWc7);
        }
        let away = math::flat(math::sub(boat.position, route.position));
        let distance = math::length(away);
        if distance >= 2.0 * boat.radius {
            return Ok(false);
        }
        let direction = math::unit(away).ok_or(ContractGap::DegenerateDirection)?;
        let amount = dt * boat.drive.0[39];
        if amount >= 1.0 {
            let mut target = math::add(route.position, math::scale(direction, 2.0 * boat.radius));
            target[1] = boat.position[1];
            let mut position = math::approach(boat.position, target, amount);
            position[1] = boat.position[1];
            boat.position = position;
        }
        Ok(math::length(math::flat(math::sub(boat.position, route.position))) < 2.0 * boat.radius)
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
        self.group.sound_value()?;
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
                self.events.extend(super::WAKES.map(super::Event::StopWake));
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
