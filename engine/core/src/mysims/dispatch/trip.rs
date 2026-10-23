// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled dispatch transitions. MS18 services never imply successful arrival.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DispatchTuning {
    pub approach_distance: i32,
    pub proximity: i32,
    pub idle_minimum: i32,
    pub location_minimum: i32,
    pub switch_minimum: i32,
}

impl Default for DispatchTuning {
    fn default() -> Self {
        Self {
            approach_distance: 10,
            proximity: 10,
            idle_minimum: 1,
            location_minimum: 1,
            switch_minimum: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TripOutcome {
    Accepted,
    Rejected,
    UnsupportedMs18,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TripSearch {
    Suspended,
    Failed {
        neighbor_retries_exhausted: bool,
    },
    /// Route construction/transport selection remains MS18.
    UnsupportedMs18,
}

/// Foreign transport, path search and mission services. No core traffic writes.
/// None/Unsupported means the spec does not yet settle this handoff, not success.
pub trait TripServices {
    fn cancel_automaton(&self, _occupant: Option<u64>) -> Option<bool> {
        None
    }
    fn is_driveable(&self, _occupant: u64) -> bool {
        false
    }
    fn attach_player(&self, _occupant: u64) -> TripOutcome {
        TripOutcome::UnsupportedMs18
    }
    fn detach_player(&self, _occupant: Option<u64>) {}
    fn detach_location_automaton(&self, _location: DispatchLocation) {}
    fn release_automaton(&self, _occupant: u64) {}
    fn search_trip(&self, _sim: i32, _state: &DispatchState) -> TripSearch {
        TripSearch::UnsupportedMs18
    }
    /// Occupant-manager order, with purpose and position read through foreign
    /// interfaces. None means unavailable; Some(empty) means no stop exists.
    fn transit_stops(&self, _rect: CellRect) -> Option<Vec<TransitStop>> {
        None
    }
    fn boarding_automata(&self, _cell: (i32, i32)) -> Option<Vec<BoardingAutomaton>> {
        None
    }
    /// Attempt a real next leg, optionally with a selected boarding automaton.
    /// Returns its travel type (FF is failure); never a synthetic arrival.
    fn dispatch_next_leg(&self, _sim: i32, _automaton: Option<u64>) -> Option<u32> {
        None
    }
    fn register_deferred_setup(&self, _sim: i32, _register: bool) {}
}

impl Default for DispatchState {
    fn default() -> Self {
        Self {
            search: JobSearchState::default(),
            current: DispatchLocation::default(),
            destination: DispatchLocation::default(),
            queue: VecDeque::new(),
            tuning: DispatchTuning::default(),
            auxiliary_flag: false,
            opaque_28: 0,
            path_indices: (0, 0),
            change_counter: 0,
            previous_change_counter: 0,
            tracking_value: 0,
            minutes: 0,
            travel_type: 0xFF,
            wait_timestamp: 0,
            neighbor_retries: 0,
            auxiliary_occupant: None,
            auxiliary_value: 0,
            auxiliary_string: Vec::new(),
            automata_occupant: None,
            owner: None,
            simulation_service: None,
            models: BTreeMap::new(),
            route_marker: 0,
            read_pending: false,
            read_succeeded: false,
        }
    }
}

impl DispatchLocation {
    pub fn duration(&self) -> i32 {
        self.reserved.0 as i32
    }
    pub fn deadline(&self) -> i32 {
        self.reserved.1 as i32
    }
    pub fn set_duration(&mut self, duration: i32) {
        self.reserved.0 = duration as u32;
    }
    pub fn set_deadline(&mut self, deadline: i32) {
        self.reserved.1 = deadline as u32;
    }

    /// Exact stored identity (§4.7.1, MS19), independent of validity,
    /// world position and timing. Neither occupant nor locality overrides cells.
    pub fn same_location(&self, other: &Self) -> bool {
        (
            self.in_city,
            self.occupant,
            self.by_commute,
            self.cell,
            self.neighbor_city,
        ) == (
            other.in_city,
            other.occupant,
            other.by_commute,
            other.cell,
            other.neighbor_city,
        )
    }
}

impl MySimDispatch {
    /// Small state mutations never retain a lock across foreign/owner calls.
    pub(super) fn edit<T>(&self, sim: i32, f: impl FnOnce(&mut DispatchState) -> T) -> T {
        f(self.sims.write().unwrap().entry(sim).or_default())
    }

    pub(super) fn change_flags(&self, sim: i32, f: impl FnOnce(u32) -> u32) {
        let (old, new) = self.edit(sim, |st| {
            let old = st.search.flags;
            st.search.flags = f(old);
            if old != st.search.flags {
                st.change_counter = st.change_counter.wrapping_add(1);
            }
            (old, st.search.flags)
        });
        if old != new {
            self.notify_owner(sim, super::super::dispatch_note::TRIP_STATE_A, old, new);
        }
    }

    pub fn set_tuning(&self, sim: i32, tuning: DispatchTuning) {
        self.edit(sim, |st| st.tuning = tuning);
    }

    /// Copies without duration clamping or deadline expiration (§4.6.1).
    pub fn add_destination_location(&self, sim: i32, location: DispatchLocation, options: u32) -> bool {
        self.edit(sim, |st| {
            if options & 4 != 0 {
                st.queue.clear();
            }
            if options & 1 != 0 {
                st.queue.push_front(location);
            } else {
                st.queue.push_back(location);
            }
        });
        true
    }

    /// Message 0x44778231: signed wrapping minutes, no state update.
    pub fn minute_message(&self, sim: i32) {
        if self.pathfinders.borrow().contains_key(&sim) {
            self.edit(sim, |st| st.minutes = st.minutes.wrapping_add(5));
        }
    }

    /// The intentionally mutating timestamp getter (§4.6.1).
    pub fn wait_value(&self, sim: i32) -> i32 {
        self.edit(sim, |st| {
            if st.search.flags & 0x10 == 0 && st.minutes.wrapping_sub(st.wait_timestamp) > 15 {
                st.wait_timestamp = 0;
            }
            st.wait_timestamp
        })
    }

    /// Called only after the foreign service has located a transit switch.
    pub fn begin_located_switch(&self, sim: i32) {
        self.edit(sim, |st| {
            st.wait_timestamp = st.minutes;
            st.current
                .set_deadline(st.minutes.wrapping_add(st.tuning.switch_minimum));
        });
        self.change_flags(sim, |f| f | 0x10);
    }

    pub fn begin_idle(&self, sim: i32) {
        self.edit(sim, |st| {
            st.current.set_deadline(st.minutes.wrapping_add(st.tuning.idle_minimum))
        });
    }

    /// §4.6.2 compares truncated *linear* distance to squared tuning.
    pub fn within_proximity(&self, sim: i32, distance: i32, final_marker: bool) -> bool {
        let p = self.get_state(sim).tuning.proximity;
        let square = p.wrapping_mul(p);
        if final_marker {
            distance <= square
        } else {
            distance < square
        }
    }

    /// Automaton position message, not the minute clock (§4.11).
    pub fn automaton_cell_changed(&self, sim: i32, cell: (i32, i32)) {
        let st = self.get_state(sim);
        if st.current.cell == cell {
            return;
        }
        self.edit(sim, |st| st.current.set_cell_position(cell.0, cell.1));
        if st.search.flags & 0x10000 != 0 {
            return;
        }
        let dx = cell.0.wrapping_sub(st.destination.cell.0);
        let dz = cell.1.wrapping_sub(st.destination.cell.1);
        let near = dx.wrapping_mul(dx).wrapping_add(dz.wrapping_mul(dz))
            < st.tuning.approach_distance.wrapping_mul(st.tuning.approach_distance);
        self.change_flags(sim, |f| if near { f | 0x8000 } else { f & !0x8000 });
    }

    pub fn go_to(&self, sim: i32, mut location: DispatchLocation, services: &dyn TripServices) -> TripOutcome {
        let st = self.get_state(sim);
        if location.occupant.is_some() {
            location.set_duration(location.duration().max(st.tuning.location_minimum));
        }
        self.edit(sim, |st| st.queue.clear());
        if st.search.state != 2 {
            self.edit(sim, |st| {
                st.current.set_deadline(st.minutes.wrapping_sub(1));
                st.queue.push_back(location);
            });
            return TripOutcome::Accepted;
        }
        self.edit(sim, |st| st.destination = location);
        let cancel = services.cancel_automaton(st.automata_occupant);
        self.change_flags(sim, |f| f & !0x5800);
        match cancel {
            Some(true) => {
                self.change_flags(sim, |f| f & !0x4000);
                TripOutcome::Accepted
            }
            Some(false) => {
                self.change_flags(sim, |f| f | 0x4000);
                if self.animation_message(sim, services) == TripOutcome::UnsupportedMs18 {
                    return TripOutcome::UnsupportedMs18;
                }
                let st = self.get_state(sim);
                if st.search.route.is_empty() || st.path_indices.0 == st.path_indices.1 {
                    self.change_flags(sim, |f| f | 0x40);
                    TripOutcome::Rejected
                } else {
                    TripOutcome::Accepted
                }
            }
            None => TripOutcome::UnsupportedMs18,
        }
    }

    pub fn can_u_drive_it(&self, sim: i32, services: &dyn TripServices) -> bool {
        let st = self.get_state(sim);
        match st.search.state {
            2 => st.travel_type == 1 && st.automata_occupant.is_some_and(|o| services.is_driveable(o)),
            3 => st.current.same_location(&st.search.home) || st.current.same_location(&st.search.commute_destination),
            _ => false,
        }
    }

    pub fn enter_u_drive_it(&self, sim: i32, services: &dyn TripServices) -> TripOutcome {
        if !self.can_u_drive_it(sim, services) {
            return TripOutcome::Rejected;
        }
        let st = self.get_state(sim);
        if st.search.state == 3 {
            self.change_flags(sim, |f| f | 0x20000);
            let target = if st.current.same_location(&st.search.home) {
                st.search.commute_destination
            } else {
                st.search.home
            };
            self.go_to(sim, target, services);
            return TripOutcome::Accepted;
        }
        let result = services.attach_player(st.automata_occupant.unwrap());
        if result == TripOutcome::Accepted {
            self.change_flags(sim, |f| f | 0x10000);
        }
        result
    }

    /// Settled held-automaton pending-player branch; service details are MS18.
    pub fn held_automaton_player_update(&self, sim: i32, services: &dyn TripServices) -> TripOutcome {
        if !self.check_flags(sim, 0x20000) {
            return TripOutcome::Rejected;
        }
        self.change_flags(sim, |f| f & !0x20000);
        self.enter_u_drive_it(sim, services)
    }

    pub fn exit_u_drive_it(&self, sim: i32, services: &dyn TripServices) -> bool {
        let st = self.get_state(sim);
        if st.search.flags & 0x10000 != 0 {
            services.detach_player(st.automata_occupant);
            self.edit(sim, |st| {
                st.queue.clear();
                st.current = st.search.home;
                st.destination = st.search.home;
                st.travel_type = 0xFF;
            });
            self.set_state(sim, -1);
            self.reset_trip_path(sim);
            if let Some(o) = st.automata_occupant {
                services.release_automaton(o);
            }
            self.edit(sim, |st| st.automata_occupant = None);
        }
        self.change_flags(sim, |f| f & !0x30000);
        true
    }

    fn reset_trip_path(&self, sim: i32) {
        self.edit(sim, |st| {
            st.search.route.clear();
            st.path_indices = (0, 0);
        });
        if let Some(pf) = self.pathfinders.borrow_mut().get_mut(&sim) {
            pf.reset();
            pf.clear_all_travel_type_flags();
            pf.set_route_info(Some(VecDeque::new()));
        }
        self.change_flags(sim, |f| f & !RESET_PATH_FLAGS);
    }

    fn valid(&self, sim: i32, location: DispatchLocation) -> bool {
        location.is_valid(&SimWorld { host: &*self.host, sim })
    }

    fn resolve(&self, sim: i32, mut location: DispatchLocation) -> Option<DispatchLocation> {
        if self.valid(sim, location) {
            return Some(location);
        }
        if !location.by_commute {
            return None;
        }
        // Reuse the persistent query-only job pathfinder, never the trip service.
        let mut search = self.get_state(sim).search;
        let ok = {
            let mut pfs = self.pathfinders.borrow_mut();
            let pf = pfs.get_mut(&sim)?;
            let mut pf = CoreJobPathfinder {
                pf,
                net: &self.net,
                host: &*self.host,
            };
            validate_commute_location(
                &mut search,
                &mut pf,
                &SimWorld { host: &*self.host, sim },
                &mut location,
            )
        };
        self.edit(sim, |st| {
            st.search.route = search.route;
            st.path_indices = (0, 0);
        });
        self.change_flags(sim, |f| f & !RESET_PATH_FLAGS);
        if !ok {
            return None;
        }
        self.edit(sim, |st| st.search.commute_destination = location);
        self.host.notify(sim, super::super::dispatch_note::JOB_FOUND, 0, 0);
        Some(location)
    }

    fn location_notice(&self, sim: i32) {
        let st = self.get_state(sim);
        self.report_arrival(sim, st.current.same_location(&st.search.home));
    }

    /// Animation message 0x86AD10EE. Only the published transition rows run.
    pub fn animation_message(&self, sim: i32, services: &dyn TripServices) -> TripOutcome {
        let st = self.get_state(sim);
        match st.search.state {
            -1 => {
                if st.search.flags & FLAG_HAS_HOME != 0 {
                    self.change_flags(sim, |f| f & 0xFFFF01B2);
                    if !st.current.same_location(&st.search.home) {
                        self.change_flags(sim, |f| f & !0x32);
                        self.edit(sim, |st| {
                            st.current = st.search.home;
                            st.travel_type = 0xFE;
                            st.current.set_deadline(st.minutes.wrapping_add(st.current.duration()));
                        });
                        self.location_notice(sim);
                    }
                    self.set_state(sim, 3);
                } else if !st.queue.is_empty() {
                    self.set_state(sim, 3);
                }
                TripOutcome::Accepted
            }
            3 => self.select_destination(sim, services),
            2 if st.search.flags & 0x1000 != 0 => {
                if st.search.flags & 0x10000 != 0 {
                    return TripOutcome::Accepted;
                }
                if st.search.flags & 0x40 != 0 {
                    self.set_state(sim, -1);
                    return TripOutcome::Accepted;
                }
                TripOutcome::UnsupportedMs18
            }
            2 => match services.search_trip(sim, &st) {
                TripSearch::Suspended => TripOutcome::Accepted,
                TripSearch::Failed {
                    neighbor_retries_exhausted: true,
                } => {
                    self.set_state(sim, -1);
                    self.location_notice(sim);
                    TripOutcome::Rejected
                }
                _ => TripOutcome::UnsupportedMs18,
            },
            _ => TripOutcome::UnsupportedMs18,
        }
    }

    fn select_destination(&self, sim: i32, services: &dyn TripServices) -> TripOutcome {
        self.edit(sim, |st| st.destination = DispatchLocation::default());
        let st = self.get_state(sim);
        services.detach_location_automaton(st.current);
        if self.valid(sim, st.current) && st.current.occupant.is_some() && st.minutes <= st.current.deadline() {
            return TripOutcome::Accepted;
        }
        if !self.valid(sim, st.current) {
            if let Some(location) = self.resolve(sim, st.current) {
                self.edit(sim, |st| st.current = location);
            } else {
                let st = self.get_state(sim);
                if st.search.flags & FLAG_HAS_HOME != 0 && self.valid(sim, st.search.home) {
                    self.edit(sim, |st| st.current = st.search.home);
                    self.location_notice(sim);
                }
            }
        }
        let mut chosen = None;
        loop {
            let next = self.edit(sim, |st| st.queue.pop_front());
            let Some(next) = next else {
                break;
            };
            if let Some(location) = self.resolve(sim, next) {
                chosen = Some(location);
                break;
            }
        }
        let st = self.get_state(sim);
        self.edit(sim, |st| st.neighbor_retries = 0);
        if chosen.is_none()
            && st.search.flags & FLAG_HAS_HOME != 0
            && !st.current.same_location(&st.search.home)
            && self.valid(sim, st.search.home)
        {
            chosen = Some(st.search.home);
        }
        if let Some(mut location) = chosen {
            if !st.current.same_location(&location) {
                location.set_deadline(st.minutes.wrapping_add(location.duration()));
                self.edit(sim, |st| st.destination = location);
                self.set_state(sim, 2);
                self.change_flags(sim, |f| f & !0x5800);
                return TripOutcome::Accepted;
            }
        }
        if st.search.flags & FLAG_HAS_HOME == 0 || !st.current.same_location(&st.search.home) {
            self.set_state(sim, -1);
        }
        TripOutcome::Accepted
    }

    /// MySim-owned LeaveForWork/ReturnHome arithmetic is exposed as a request
    /// helper only; the MySim's refusal sound/state callbacks stay with that owner.
    pub fn dispatch_commute(&self, sim: i32, has_residence: bool, mysim_state: u8, hour: i32) -> TripOutcome {
        if !has_residence {
            return TripOutcome::Rejected;
        }
        let st = self.get_state(sim);
        match mysim_state {
            2 => {
                let mut location = st.search.commute_destination;
                if !location.by_commute || !self.valid(sim, location) {
                    location = DispatchLocation {
                        by_commute: true,
                        ..DispatchLocation::default()
                    };
                }
                location.set_duration(if !(8..=17).contains(&hour) {
                    5
                } else {
                    17i32.wrapping_sub(hour).wrapping_mul(60)
                });
                self.add_destination_location(sim, location, 4);
            }
            3 => {
                self.add_destination_location(sim, st.search.home, 4);
            }
            1 => {
                self.teleport_to_commute_location(sim);
                self.add_destination_location(sim, st.search.home, 4);
            }
            _ => {}
        }
        TripOutcome::Accepted
    }

    /// §7.2.1: unconditional copy, even for invalid commute locations. Complete
    /// every state write before synchronous owner callbacks; no arrival/reset.
    pub fn teleport_to_commute_location(&self, sim: i32) {
        let notes = self.edit(sim, |st| {
            if st.travel_type != 0xFE {
                st.change_counter = st.change_counter.wrapping_add(1);
            }
            st.travel_type = 0xFE;
            st.current = st.search.commute_destination;
            st.current.set_deadline(st.minutes.wrapping_add(st.current.duration()));
            st.change_counter = st.change_counter.wrapping_add(1);
            set_state(&mut st.search, 3)
        });
        for note in notes {
            match note {
                StateNote::Flags { old, new } => {
                    self.notify_owner(sim, super::super::dispatch_note::TRIP_STATE_A, old, new)
                }
                StateNote::TripState { old, new } => {
                    self.notify_owner(sim, super::super::dispatch_note::TRIP_STATE_B, old as u32, new as u32)
                }
            }
            // MS19 specifies this increment after each notification attempt,
            // including when no owner is available.
            self.edit(sim, |st| st.change_counter = st.change_counter.wrapping_add(1));
        }
    }
}
