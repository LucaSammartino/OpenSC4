// SPDX-License-Identifier: GPL-3.0-or-later
//! Dispatch-owned route decisions (mysims §4.6.3–§4.6.4).
use super::*;

/// Group/model type selection uses no dispatch-owned RNG (§4.6.4).
pub fn model_selection(mode: u32, idle: bool, male: bool, walking_key: bool) -> (u32, i32) {
    let group = match mode {
        0 | 0xFE if !walking_key => {
            if male {
                0x4149
            } else {
                0x414A
            }
        }
        0 | 0xFE => 0x4107,
        1 => 0x420B,
        2 => 0x4200,
        3 => 0x4215,
        6 => 0x4217,
        7 => 0x422C,
        8 => 0x422E,
        _ => 0,
    };
    let model = match mode {
        1 => 0,
        0 if !idle => 1,
        0 | 0xFE => 2,
        _ => -1,
    };
    (group, model)
}

impl MySimDispatch {
    /// §4.6.4: the native temporary list is never populated. Even a valid
    /// regional connection cannot make any retry index select a location.
    pub fn update_neighbor_location(&self, _index: u32, _location: &mut DispatchLocation) -> bool {
        false
    }

    pub fn model_offset(&self, sim: i32, model_type: i32) -> i32 {
        self.get_state(sim).models.get(&model_type).map_or(-1, |entry| entry.1)
    }

    pub fn set_stuck(&self, sim: i32) {
        let st = self.get_state(sim);
        if st.search.flags & 1 == 0 && (st.search.flags & 0x12 != 0 || matches!(st.travel_type, 0 | 1)) {
            self.change_flags(sim, |f| f | 0x2401);
        }
        self.change_flags(sim, |f| f | 0x440);
        self.notify_owner(sim, 0xEBA06C12, 0, 0);
    }

    /// The current route marker is a specified input, not reconstructed from
    /// a native output record. Uncaptured service/partition inputs explicitly
    /// return UnsupportedMs18; they never imply a successful trip.
    pub fn route_ready_update(&self, sim: i32, marker: RouteData, services: &dyn TripServices) -> TripOutcome {
        let st = self.get_state(sim);
        if st.search.flags & 0x10000 != 0 {
            return TripOutcome::Accepted;
        }
        let mut tail = TransportTail {
            travel_type: st.travel_type,
            state: st.search.state,
            stuck: false,
            idle: false,
            clear_flags: 0,
            set_flags: 0,
        };
        if st.search.flags & 0x41 == 0x40 {
            tail.state = -1;
        } else if st.search.flags & 1 != 0 {
            return self.idle_recovery(sim, marker, services);
        } else if st.travel_type == 0xFE
            && st.search.flags & 4 == 0
            && !st.current.same_location(&st.search.home)
            && !st.current.same_location(&st.search.commute_destination)
        {
            self.begin_idle(sim);
            tail.idle = true;
            tail.set_flags = 1;
        } else if st.search.flags & 0x10 != 0 {
            return self.transit_stop_update(sim, marker, services);
        } else if st.search.flags & 4 != 0 {
            if st.minutes > st.current.deadline() {
                let Some(travel) = services.dispatch_next_leg(sim, None) else {
                    return TripOutcome::UnsupportedMs18;
                };
                tail.travel_type = travel;
                tail.stuck = travel == 0xFF;
                if tail.stuck {
                    tail.set_flags = 0x40;
                } else {
                    tail.clear_flags = 0x436;
                    tail.set_flags = 0x200;
                }
            }
        } else if st.search.flags & 0x20 == 0 {
            let Some(travel) = services.dispatch_next_leg(sim, None) else {
                return TripOutcome::UnsupportedMs18;
            };
            if travel == 0xFF {
                // No invented skip selection without the partition inputs.
                return TripOutcome::UnsupportedMs18;
            }
            tail.travel_type = travel;
            tail.clear_flags = 0x436;
            tail.set_flags = 0x200;
        } else {
            tail.clear_flags = 0x20;
            let target = u32::from(marker.travel_type);
            if target == 0xFE {
                let local = st.current.in_city && st.destination.in_city;
                let near = self.final_marker_observation(sim, services);
                tail.travel_type = 0xFE;
                tail.clear_flags = 0x460;
                if !near {
                    tail.set_flags = if local { 0x423 } else { 0x427 };
                    tail.idle = true;
                } else if local
                    && st.search.flags & 5 == 0
                    && !st.current.same_location(&st.search.home)
                    && !st.current.same_location(&st.search.commute_destination)
                {
                    self.begin_idle(sim);
                    tail.set_flags = 0x423;
                    tail.idle = true;
                } else {
                    tail.state = 3;
                    tail.clear_flags = 0x8460;
                    tail.set_flags = if local { 2 } else { 6 };
                    let valid = self.valid(sim, st.destination);
                    self.edit(sim, |state| {
                        state
                            .current
                            .set_duration(state.current.duration().wrapping_add(state.minutes));
                        if valid {
                            state.current = state.destination;
                        }
                    });
                }
            } else if matches!(st.travel_type, 0 | 1 | 2 | 3 | 6 | 7 | 8) && matches!(target, 0 | 1 | 2 | 3 | 6 | 7 | 8)
            {
                if st.travel_type == target {
                    let Some(travel) = services.dispatch_next_leg(sim, st.automata_occupant) else {
                        return TripOutcome::UnsupportedMs18;
                    };
                    if travel == 0xFF {
                        self.set_stuck(sim);
                    } else {
                        self.change_flags(sim, |f| f | 0x200);
                        self.change_flags(sim, |f| f & !0x400);
                        tail.travel_type = travel;
                    }
                } else {
                    self.ordinary_switch_wait(sim, target, services);
                    tail.set_flags = if matches!(target, 0 | 1) { 1 } else { 0x10 };
                    tail.idle = tail.set_flags == 1;
                }
            } else if matches!(st.travel_type, 0xFE | 0xFF) && target != st.travel_type {
                tail.state = -1;
            } else if matches!(st.travel_type, 0 | 1 | 2 | 3 | 6 | 7 | 8) {
                self.clear_held_automaton(sim, services);
            }
            // Switch fallback also follows same-mode success or setStuck.
            if tail.set_flags & 0x53 == 0 {
                tail.set_flags |= 3;
                tail.travel_type = 0xFE;
                tail.idle = true;
                self.edit(sim, |state| state.queue.clear());
                self.begin_idle(sim);
            }
        }
        self.apply_transport_tail(sim, tail);
        TripOutcome::Accepted
    }

    fn idle_recovery(&self, sim: i32, marker: RouteData, services: &dyn TripServices) -> TripOutcome {
        let entry = self.get_state(sim);
        let mut travel = entry.travel_type;
        let mut state = entry.search.state;
        if entry.search.flags & 0x2000 != 0 {
            self.notify_owner(sim, 0xEBA06BD0, 0, 0);
        }
        let Some(kind) = services.idle_model_kind(entry.automata_occupant) else {
            return TripOutcome::UnsupportedMs18;
        };
        let direct = entry.automata_occupant.is_some()
            && entry.route_marker == 2
            && kind == 7
            && matches!(entry.travel_type, 0 | 0xFE);
        if !direct {
            let mut snapshot = entry.current;
            if marker.travel_type != 0xFE {
                snapshot.set_cell_position(i32::from(marker.x), i32::from(marker.z));
            }
            let mut current = entry.current;
            let Some(first) = services.snap_to_road(&mut current) else {
                return TripOutcome::UnsupportedMs18;
            };
            let snapped = if first {
                true
            } else {
                let Some(second) = services.snap_to_road(&mut snapshot) else {
                    return TripOutcome::UnsupportedMs18;
                };
                second
            };
            if !snapped {
                self.notify_owner(sim, 0xEBA06C12, 0, 0);
                self.set_state(sim, -1);
                return TripOutcome::Accepted;
            }
            // Literal §4.6.3: current receives the snapshot after either snap
            // succeeds, including a successful first snap of current itself.
            self.edit(sim, |st| st.current = snapshot);
            self.notify_owner(sim, 0xEBA06BD0, 0, 0);
            let Some(result) = services.dispatch_idle_fallback(sim) else {
                return TripOutcome::UnsupportedMs18;
            };
            if result == 0xFF {
                self.notify_owner(sim, 0xEBA06C12, 0, 0);
                state = -1;
            } else if entry.travel_type != 0xFE {
                travel = result;
            }
        }
        let mut clear = 0x2002;
        let mut add = 0;
        if entry.search.flags & 0x2000 != 0 {
            let current = self.get_state(sim);
            if let Some(occupant) = current
                .automata_occupant
                .filter(|_| current.current.deadline() >= current.minutes)
            {
                let offset = self.model_offset(sim, current.route_marker);
                if offset > 0 {
                    services.apply_model_offset(occupant, offset);
                }
            } else {
                self.change_flags(sim, |flags| flags & !1);
                let near = self.destination_proximity(sim, false);
                if travel == 0xFE && self.check_flags(sim, 0x20) {
                    state = 3;
                    clear = 0xA403;
                    self.finish_idle_location(sim, near);
                } else {
                    let Some(result) = services.dispatch_next_leg(sim, None) else {
                        return TripOutcome::UnsupportedMs18;
                    };
                    if result == 0xFF {
                        // MS21: the route partition/skip boundary is not yet
                        // supplied by this adapter. Do not invent a skip.
                        return TripOutcome::UnsupportedMs18;
                    }
                    travel = result;
                    clear = 0x2437;
                    add = 0x200;
                }
            }
        }
        self.apply_transport_tail(
            sim,
            TransportTail {
                travel_type: travel,
                state,
                stuck: false,
                idle: false,
                clear_flags: clear,
                set_flags: add,
            },
        );
        TripOutcome::Accepted
    }

    fn finish_idle_location(&self, sim: i32, near: bool) {
        let st = self.get_state(sim);
        let copy = near && self.valid(sim, st.destination);
        self.edit(sim, |state| {
            state
                .current
                .set_deadline(state.minutes.wrapping_add(state.current.duration()));
            if copy {
                state.current = state.destination;
            }
        });
    }
}
