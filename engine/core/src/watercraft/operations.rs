// SPDX-License-Identifier: GPL-3.0-or-later
use crate::automata::{State, Vec3};

use super::math;
use super::{state, BoatId, Cell, ContractGap, DockManager, Event, Fleet, Result, Watercraft};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Message {
    ReloadDriveParameters,
    /// The point is from foreign world picking, not original boat selection.
    TowLanding(Option<Vec3>),
    Other(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageDisposition {
    Handled,
    ForwardToAutomaton,
}

impl Fleet {
    pub fn get(&self, id: BoatId) -> Option<&Watercraft> {
        self.boats.get(&id)
    }

    pub fn get_mut(&mut self, id: BoatId) -> Option<&mut Watercraft> {
        self.boats.get_mut(&id)
    }

    /// Register a newly constructed Rust boat without replacing live storage.
    pub fn insert(&mut self, id: BoatId, boat: Watercraft) -> std::result::Result<(), Box<Watercraft>> {
        match self.boats.entry(id) {
            std::collections::btree_map::Entry::Occupied(_) => Err(Box::new(boat)),
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(boat);
                self.order.push(id);
                Ok(())
            }
        }
    }

    /// Exact prefix of DoAnimationTick, stopping before the inherited step.
    /// Returns false only for the specified low-speed-modifier early return.
    pub fn animation_budget(
        &mut self,
        id: BoatId,
        speed_modifier: f32,
        second_dt: f32,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<bool> {
        if speed_modifier < 0.00001 {
            return Ok(false);
        }
        if self.boat(id)?.count_idle_always {
            self.boat_mut(id)?.idle_elapsed += second_dt;
            if self.boat(id)?.idle_elapsed > 60_000.0 {
                self.push_state(id, State::new(state::SLOW_REMOVE), manager)?;
            }
        }
        Ok(true)
    }

    /// Lifetime substep (§5.1 steps 9-12) operates on owned state only.
    /// Returns whether AI movement is reached, not a native tick verdict.
    pub fn lifetime_tick(
        &mut self,
        id: BoatId,
        first_dt: f32,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<bool> {
        let boat = self.boat(id)?;
        if [
            state::PLAYER_DRIVE,
            state::DOCK,
            state::QUEUE,
            state::TOWED,
            state::UNDOCK,
        ]
        .iter()
        .any(|state| boat.has_state(*state))
            || boat.being_towed
            || boat.count_idle_always
        {
            return Ok(false);
        }
        if boat.lifetime_limit != 0.0 && boat.lifetime_elapsed > boat.lifetime_limit {
            let parked = boat.has_state(state::PARKED);
            // §5.1 explicitly asks the manager, then applies the parked
            // override; do not short-circuit away the foreign call.
            let retained = manager.as_deref_mut().is_some_and(|m| m.may_stay(id));
            if !retained || parked {
                self.push_state(id, State::new(state::SLOW_REMOVE), manager)?;
            } else {
                let boat = self.boat_mut(id)?;
                boat.lifetime_limit = 0.0;
                boat.record.base.automata_flags &= !0x10000;
            }
        }
        let boat = self.boat_mut(id)?;
        if boat.has_state(state::PARKED) || boat.top_state() == Some(state::IDLE) {
            return Ok(false);
        }
        boat.lifetime_elapsed += first_dt;
        Ok(true)
    }

    /// Message selection uses only the Rust fleet's current geometry/state.
    pub fn do_message(
        &mut self,
        id: BoatId,
        message: Message,
        manager: &mut Option<&mut dyn DockManager>,
    ) -> Result<MessageDisposition> {
        match message {
            Message::ReloadDriveParameters => {
                self.boat_mut(id)?.load_drive_parameters();
                Ok(MessageDisposition::Handled)
            }
            Message::TowLanding(point) if self.boat(id)?.tow_armed => {
                self.boat_mut(id)?.tow_armed = false;
                let Some(point) = point.filter(|_| manager.is_some()) else {
                    return Ok(MessageDisposition::Handled);
                };
                // Fleet's stable insertion order models manager-list order (§7).
                // Filter group/state only after the first geometric match.
                let picked = self.order.iter().copied().find(|other| {
                    *other != id
                        && self
                            .boats
                            .get(other)
                            .is_some_and(|boat| math::length(math::sub(boat.position, point)) < 2.0 * boat.radius)
                });
                if let Some(other) = picked {
                    let boat = self.boat(other)?;
                    if boat.group_id() == Some(0x4417)
                        && !matches!(
                            boat.top_state(),
                            Some(state::RUN_AGROUND | state::SINK | state::IMMEDIATE_REMOVE | state::SLOW_REMOVE)
                        )
                    {
                        if matches!(boat.top_state(), Some(state::DOCK | state::QUEUE | state::PARKED)) {
                            self.pop_state(other, manager)?;
                        }
                        self.boat_mut(other)?.tug = Some(id);
                        self.push_state(other, State::new(state::TOWED), manager)?;
                        self.boat_mut(id)?.towed = Some(other);
                        self.boat_mut(id)?.tug = None;
                    }
                }
                Ok(MessageDisposition::Handled)
            }
            _ => Ok(MessageDisposition::ForwardToAutomaton),
        }
    }

    /// §6.1 tow cancellation precedes coast and movement.
    pub fn release_tow_ahead(&mut self, id: BoatId, manager: &mut Option<&mut dyn DockManager>) -> Result<()> {
        let boat = self.boat(id)?;
        if let Some(towed) = boat.towed {
            let ahead = math::dot(math::sub(self.boat(towed)?.position, boat.position), boat.heading);
            if ahead > 0.0 {
                self.end_towing(id, manager)?;
            }
        }
        Ok(())
    }

    /// §6.2 stopped-tug button branch; sound-set gate applies first.
    pub fn tow_button(&mut self, id: BoatId, pressed: bool, manager: &mut Option<&mut dyn DockManager>) -> Result<()> {
        let boat = self.boat(id)?;
        if !pressed || boat.sound_value().is_none() || boat.group_id() != Some(0x4410) || boat.speed != 0.0 {
            return Ok(());
        }
        if boat.towed.is_some() {
            self.end_towing(id, manager)?;
        } else {
            let boat = self.boat_mut(id)?;
            boat.tow_armed = true;
            boat.events.push(Event::PlayEffect("tugropethrow"));
        }
        Ok(())
    }

    /// Advance the reader over already decoded network route cells. This is
    /// not Engine_AIPickLocation: direction encoding/backtracking remain WC7.
    pub fn next_route_cell(&mut self, id: BoatId, manager: &mut Option<&mut dyn DockManager>) -> Result<Option<Cell>> {
        let boat = self.boat_mut(id)?;
        if let Some(route) = boat.neighbor.as_mut() {
            if let Some(cell) = route.cells.get(route.next).copied() {
                route.next += 1;
                return Ok(Some(cell));
            }
            self.push_state(id, State::new(state::SLOW_REMOVE), manager)?;
            return Ok(None);
        }
        if boat.ferry {
            let station = boat.current_station.as_ref().ok_or(ContractGap::MissingStationWc7)?;
            if !boat.route_started {
                boat.route_next = boat.used_cells;
                boat.route_started = true;
            }
            let Some(cell) = station.route.get(boat.route_next).copied() else {
                return Ok(None);
            };
            boat.route_next += 1;
            boat.used_cells += 1;
            return Ok(Some(cell));
        }
        Err(ContractGap::MissingRouteWc7)
    }
}

impl Watercraft {
    /// Idle/no-motion rejection of DoAnimationTick, after wake/timer updates.
    pub fn still_time_allows_animation(&self) -> bool {
        self.still_elapsed <= 5000.0
            || [
                state::IDLE,
                state::PLAYER_DRIVE,
                state::QUEUE,
                state::PARKED,
                state::TOWED,
                state::UNDOCK,
            ]
            .iter()
            .any(|state| self.has_state(*state))
    }

    pub fn update_backwards_time(&mut self, first_dt: f32) {
        if self.speed < 0.0 {
            self.backwards_elapsed += first_dt;
        } else {
            self.backwards_elapsed = 0.0;
        }
    }
}

/// Assist_PickNewDir's exact priority and mask rule, independent of any
/// coordinate convention, navigability, collision or RNG provider.
pub fn candidate_directions(direction: u32, tried: u8) -> impl Iterator<Item = i32> {
    [0_i32, 1, -1, 2, -2, 3, -3, 4]
        .into_iter()
        .map(move |offset| (direction as i32).wrapping_add(offset) % 8)
        .filter(move |candidate| *candidate != (direction as i32).wrapping_add(4) % 8)
        .filter(move |candidate| tried & (1_u32.wrapping_shl((*candidate as u32) & 31) as u8) == 0)
}
