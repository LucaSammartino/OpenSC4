// SPDX-License-Identifier: GPL-3.0-or-later
use crate::automata::State;

use super::{state, BoatId, ContractGap, DockManager, Event, Fleet, Result, Watercraft};

impl Fleet {
    /// Owner enter hooks only; inherited state handling is not replaced here.
    pub fn push_state(&mut self, id: BoatId, state: State, manager: &mut Option<&mut dyn DockManager>) -> Result<()> {
        let kind = state.state_type;
        self.push_raw(id, state)?;
        match kind {
            state::PLAYER_DRIVE => {
                let boat = self.boat_mut(id)?;
                boat.events.push(Event::ActiveSound(false));
                boat.player.field_30 = 0.0;
                boat.player.turn_rate = boat.drive.0[20];
                boat.player.aems_value = boat.sound_value().unwrap_or(1.0);
                boat.player.field_144 = 15;
                boat.player.shots = 0;
                boat.player.field_1ac = 2000.0;
                self.end_towing(id, manager)?;
                self.end_docking(id, manager)?;
            }
            state::RUN_AGROUND | state::SINK => {
                let boat = self.boat_mut(id)?;
                boat.record.base.automata_flags |= 0x1000_0000 | 0x0800_0000;
                // WC5: literal watercraft §4 says (0xB,0); automata §6.6
                // contradicts it with duration. No reconciled coverage claim.
                boat.events.push(Event::Notify(0xB, 0));
                boat.crash_elapsed = 0.0;
                self.end_towing(id, manager)?;
                self.end_docking(id, manager)?;
            }
            state::IMMEDIATE_REMOVE | state::SLOW_REMOVE => {
                let boat = self.boat_mut(id)?;
                boat.record.base.automata_flags |= 0x1000_0000 | 0x0800_0000;
                boat.events.push(Event::Remove(kind == state::IMMEDIATE_REMOVE));
                self.end_towing(id, manager)?;
                self.end_docking(id, manager)?;
            }
            state::DOCK => {
                let boat = self.boat_mut(id)?;
                boat.record.base.automata_flags |= 0x1000_0000;
                boat.docking = true;
                boat.docking_in_progress = true;
                self.end_towing(id, manager)?;
            }
            state::QUEUE => {
                let boat = self.boat_mut(id)?;
                boat.record.base.automata_flags |= 0x1000_0000;
                boat.queued = true;
                boat.speed = 0.0;
                boat.turn = 0.0;
                self.end_towing(id, manager)?;
            }
            state::PARKED => {
                let boat = self.boat_mut(id)?;
                boat.speed = 0.0;
                boat.turn = 0.0;
                self.end_towing(id, manager)?;
                self.end_docking(id, manager)?;
            }
            state::TOWED => {
                let boat = self.boat_mut(id)?;
                boat.being_towed = true;
                boat.record.base.automata_flags |= 0x1000_0000;
                self.end_docking(id, manager)?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn pop_state(&mut self, id: BoatId, manager: &mut Option<&mut dyn DockManager>) -> Result<bool> {
        let Some(top) = self.boat(id)?.record.base.states.last() else {
            return Ok(false);
        };
        // Reuse the base record/state representation and preserve the base
        // final-state rule implemented by automata::Automaton::pop_state.
        if top.base.is_final && top.state_type != crate::automata::state_type::RIOT_END {
            self.push_raw(id, State::new(crate::automata::state_type::RIOT_END))?;
            return Ok(true);
        }
        let state = self.boat_mut(id)?.record.base.states.pop().unwrap();
        match state.state_type {
            state::PLAYER_DRIVE => {
                self.boat_mut(id)?.events.push(Event::ActiveSound(true));
                self.end_towing(id, manager)?;
            }
            state::DOCK => {
                let boat = self.boat_mut(id)?;
                boat.docking = false;
                if boat.dock_requests.iter().all(|request| *request) {
                    if let Some(station) = boat.other_station() {
                        boat.initialize_current_route(station);
                    }
                } else {
                    boat.events.push(Event::Remove(true));
                }
            }
            state::QUEUE => {
                let boat = self.boat_mut(id)?;
                boat.record.base.automata_flags &= !0x1000_0000;
                boat.queued = false;
            }
            state::TOWED => {
                let tug = self.boat_mut(id)?.tug.take();
                self.boat_mut(id)?.record.base.automata_flags &= !0x1000_0000;
                if let Some(tug) = tug.and_then(|id| self.boats.get_mut(&id)) {
                    tug.towed = None;
                }
                // WC7: §4 does not say to clear being_towed on exit.
            }
            _ => {}
        }
        Ok(true)
    }

    pub fn end_towing(&mut self, id: BoatId, manager: &mut Option<&mut dyn DockManager>) -> Result<()> {
        if let Some(towed) = self.boat_mut(id)?.towed.take() {
            self.pop_state(towed, manager)?;
            self.boat_mut(towed)?.tug = None;
        }
        Ok(())
    }

    pub fn end_docking(&mut self, id: BoatId, manager: &mut Option<&mut dyn DockManager>) -> Result<()> {
        let boat = self.boat_mut(id)?;
        if boat.held_berth {
            if let Some(station) = boat.current_station.as_ref() {
                boat.docking_in_progress = false;
                if let Some(manager) = manager.as_deref_mut() {
                    manager.done_docking(&station.lot, id);
                    boat.held_berth = false;
                    boat.record.base.automata_flags &= !0x1000_0000;
                }
            }
        }
        Ok(())
    }

    pub fn set_state_internal(
        &mut self,
        id: BoatId,
        case: CrashCase,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<()> {
        let kind = match case {
            CrashCase::ImmediateRemove => state::IMMEDIATE_REMOVE,
            CrashCase::SlowRemove => state::SLOW_REMOVE,
            CrashCase::RunAground => state::RUN_AGROUND,
            CrashCase::Sink | CrashCase::CollisionSink => state::SINK,
        };
        let boat = self.boat_mut(id)?;
        let mut next = State::new(kind);
        if kind == state::RUN_AGROUND || kind == state::SINK {
            boat.events.push(Event::Notify(0xB, 0));
            next.base.countdown_ms = boat.drive.0[21];
            next.base.is_final = true;
        }
        if case == CrashCase::CollisionSink {
            boat.sunk_by_collision = true;
        }
        self.push_state(id, next, manager)
    }

    /// Owner-only state dispatch. Crash animation is not published; its
    /// accumulator still follows the specified order, then returns WC5.
    pub fn state_tick(&mut self, id: BoatId, dt: f32, manager: &mut Option<&mut dyn DockManager>) -> Result<()> {
        match self.boat(id)?.top_state() {
            Some(state::IDLE) => {
                self.boat_mut(id)?.idle_elapsed += dt;
                if self.boat(id)?.idle_elapsed > 60_000.0 {
                    self.push_state(id, State::new(state::SLOW_REMOVE), manager)?;
                    self.boat_mut(id)?.crash_elapsed += dt;
                    return Err(ContractGap::CrashSequenceWc5);
                }
            }
            Some(state::RUN_AGROUND | state::SINK) => {
                self.boat_mut(id)?.crash_elapsed += dt;
                return Err(ContractGap::CrashSequenceWc5);
            }
            Some(state::IMMEDIATE_REMOVE | state::SLOW_REMOVE) => {
                let immediate = self.boat(id)?.top_state() == Some(state::IMMEDIATE_REMOVE);
                self.boat_mut(id)?.events.push(Event::Remove(immediate));
            }
            Some(state::QUEUE) => self.dock_requests(id, manager)?,
            Some(state::DOCK | state::TOWED | state::UNDOCK) => {
                // Geometry services are required; use the corresponding
                // docking/following/undocking method, not a fake success.
                return Err(ContractGap::AiMotionWc4);
            }
            _ => {}
        }
        Ok(())
    }

    /// One-shot owner Shutdown, with all owned resource drops in §3 order.
    pub fn shutdown(&mut self, id: BoatId, manager: &mut Option<&mut dyn DockManager>) -> Result<bool> {
        if self.boat(id)?.shut_down {
            return Ok(false);
        }
        if let Some(towed) = self.boat_mut(id)?.towed.take() {
            self.push_state(towed, State::new(state::SLOW_REMOVE), manager)?;
            self.boat_mut(towed)?.tug = None;
        }
        if let Some(tug) = self.boat_mut(id)?.tug.take() {
            self.boat_mut(tug)?.towed = None;
        }
        let boat = self.boat_mut(id)?;
        for i in 0..2 {
            if boat.dock_requests[i] {
                if let (Some(station), Some(manager)) = (boat.stations[i].as_ref(), manager.as_deref_mut()) {
                    manager.done_docking(&station.lot, id);
                }
                boat.dock_requests[i] = false;
            }
        }
        if boat.initialized {
            boat.events.push(Event::Unregister(super::TOW_MESSAGE));
        }
        boat.stations = [None, None];
        boat.current_station = None;
        boat.neighbor = None;
        for effect in &mut boat.effects {
            if let Some(effect) = effect.take() {
                boat.events.push(Event::StopEffect(effect));
            }
        }
        boat.events.push(Event::BaseShutdown);
        boat.shut_down = true;
        Ok(true)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrashCase {
    ImmediateRemove,
    SlowRemove,
    RunAground,
    Sink,
    CollisionSink,
}

impl Watercraft {
    /// Exact shore-case decision from automata §6.6; dry-corner queries and
    /// terrain normals come from foreign terrain, not original boat code.
    pub fn determine_crash_case(&self, dry_corners: [bool; 4], terrain_normal: [f32; 3]) -> Result<CrashCase> {
        if self.top_state() != Some(state::PLAYER_DRIVE) {
            return Ok(CrashCase::SlowRemove);
        }
        if dry_corners.iter().any(|dry| *dry)
            && super::math::angle(super::math::scale(self.heading, -1.0), terrain_normal)
                .ok_or(ContractGap::DegenerateDirection)?
                > self.drive.0[25]
        {
            Ok(CrashCase::RunAground)
        } else {
            Ok(CrashCase::Sink)
        }
    }
}
