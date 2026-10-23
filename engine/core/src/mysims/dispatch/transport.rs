// SPDX-License-Identifier: GPL-3.0-or-later
//! Verified §4.6.2 operations. These are deliberately separate from the still
//! unsettled route-branch/mask selection (MS18), not a replacement branch matrix.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransitStop {
    pub occupant: u64,
    pub purpose: u32,
    pub world: (f32, f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoardingAutomaton {
    pub occupant: u64,
    pub group: u32,
    pub has_automaton_interface: bool,
}

/// Selected effects, not a route-selection policy. MS18 still owns the missing
/// branch/mask matrix. No foreign service is asked to invent these selections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransportTail {
    pub travel_type: u32,
    pub state: i32,
    pub stuck: bool,
    pub idle: bool,
    pub clear_flags: u32,
    pub set_flags: u32,
}

fn ordinary(mode: u32) -> bool {
    matches!(mode, 0 | 1 | 2 | 3 | 6 | 7 | 8)
}

fn stop_class(mode: u32) -> Option<(u32, u32)> {
    match mode {
        2 => Some((0x1301, 0x4200)),
        3 => Some((0x1300, 0x4215)),
        6 => Some((0x1302, 0x4217)),
        7 => Some((0x1303, 0x422C)),
        8 => Some((0x1307, 0x422E)),
        _ => None,
    }
}

impl MySimDispatch {
    fn geometry_rect(&self, location: DispatchLocation) -> CellRect {
        let rect = match location.occupant {
            Some(o) => self.host.occupant_lot_rect(o),
            None => self.host.lot_rect_at(location.cell.0, location.cell.1),
        };
        rect.unwrap_or_else(|| CellRect::new(location.cell.0, location.cell.1, location.cell.0, location.cell.1))
    }

    /// Minimum of float32 corner-to-rectangle distances, then signed truncation.
    /// A contains B is intentionally not equivalent to B contains A (§4.6.2).
    pub fn location_distance(&self, a: DispatchLocation, b: DispatchLocation) -> i32 {
        let a = self.geometry_rect(a);
        let b = self.geometry_rect(b);
        let (x1, z1, x2, z2) = (a.x1 as f32, a.z1 as f32, a.x2 as f32, a.z2 as f32);
        let mut distance = f32::INFINITY;
        for (x, z) in [(b.x1, b.z1), (b.x1, b.z2), (b.x2, b.z1), (b.x2, b.z2)] {
            let (x, z) = (x as f32, z as f32);
            let dx = (x1 - x).max(0.0).max(x - x2);
            let dz = (z1 - z).max(0.0).max(z - z2);
            distance = distance.min((dx * dx + dz * dz).sqrt());
        }
        // _ftol2's indefinite value for signed overflow; Rust casts saturate.
        if distance >= 2_147_483_648.0 {
            i32::MIN
        } else {
            distance as i32
        }
    }

    pub fn destination_proximity(&self, sim: i32, final_marker: bool) -> bool {
        let st = self.get_state(sim);
        self.within_proximity(sim, self.location_distance(st.destination, st.current), final_marker)
    }

    fn clear_held_automaton(&self, sim: i32, services: &dyn TripServices) {
        let held = self.edit(sim, |st| st.automata_occupant.take());
        if let Some(o) = held {
            services.release_automaton(o);
        }
    }

    /// Only ordinary, different-mode waits are specified here. Accepted means
    /// this wait setup ran, not that an entire handoff or trip completed.
    pub fn ordinary_switch_wait(&self, sim: i32, target: u32, services: &dyn TripServices) -> TripOutcome {
        let st = self.get_state(sim);
        if !ordinary(st.travel_type) || !ordinary(target) || st.travel_type == target {
            return TripOutcome::UnsupportedMs18;
        }
        self.edit(sim, |st| st.current.occupant = None);
        self.clear_held_automaton(sim, services);
        self.edit(sim, |st| {
            let wait = if target <= 1 {
                st.tuning.idle_minimum
            } else {
                st.tuning.switch_minimum
            };
            st.current.set_deadline(st.minutes.wrapping_add(wait));
        });
        TripOutcome::Accepted
    }

    /// The flag-10 stop/boarding operation with a route marker supplied by the
    /// route controller. Route index selection remains MS18; this does not guess
    /// which stored index is the next marker.
    pub fn transit_stop_update(&self, sim: i32, marker: RouteData, services: &dyn TripServices) -> TripOutcome {
        let st = self.get_state(sim);
        if st.search.flags & 0x10 == 0 {
            return TripOutcome::UnsupportedMs18;
        }
        let Some((purpose, group)) = stop_class(u32::from(marker.travel_type)) else {
            return TripOutcome::UnsupportedMs18;
        };
        if st.current.occupant.is_none() {
            let (x, z) = (i32::from(marker.x), i32::from(marker.z));
            let rect = CellRect::new(
                st.current.cell.0.min(x).wrapping_sub(1),
                st.current.cell.1.min(z).wrapping_sub(1),
                st.current.cell.0.max(x).wrapping_add(1),
                st.current.cell.1.max(z).wrapping_add(1),
            );
            let Some(stops) = services.transit_stops(rect) else {
                return TripOutcome::UnsupportedMs18;
            };
            let Some(stop) = stops.into_iter().find(|stop| stop.purpose == purpose) else {
                self.change_flags(sim, |f| f | 0x40);
                return TripOutcome::Rejected;
            };
            self.edit(sim, |st| {
                // Literal §4.6.2 writes: preserve other location fields.
                st.current.occupant = Some(stop.occupant);
                st.current.world = stop.world;
                st.current.cell = (world_to_cell(stop.world.0), world_to_cell(stop.world.1));
                st.current
                    .set_deadline(st.minutes.wrapping_add(st.tuning.switch_minimum));
                st.wait_timestamp = st.minutes;
            });
            self.notify_owner(sim, 0xEBA0732E, purpose, 0);
            self.clear_held_automaton(sim, services);
        }
        let st = self.get_state(sim);
        let automaton = if st.current.deadline() >= st.minutes {
            let Some(automata) = services.boarding_automata(st.current.cell) else {
                return TripOutcome::UnsupportedMs18;
            };
            let Some(a) = automata
                .into_iter()
                .find(|a| a.group == group && a.has_automaton_interface)
            else {
                return TripOutcome::Accepted; // still waiting, including equality
            };
            Some(a.occupant)
        } else {
            None
        };
        let Some(travel) = services.dispatch_next_leg(sim, automaton) else {
            return TripOutcome::UnsupportedMs18;
        };
        if travel == 0xFF {
            self.change_flags(sim, |f| f | 0x40);
            return TripOutcome::Rejected;
        }
        self.install_travel_type(sim, travel);
        self.change_flags(sim, |f| f | 0x200);
        self.change_flags(sim, |f| f & !0x436);
        TripOutcome::Accepted
    }

    /// FE-marker observations only. Near does not promise arrival/state 3;
    /// the remaining near/idle mask selection is explicitly still MS18.
    /// The caller applies the separately selected common tail after this notice.
    pub fn final_marker_observation(&self, sim: i32, services: &dyn TripServices) -> bool {
        let st = self.get_state(sim);
        if !st.current.in_city || !st.destination.in_city {
            self.edit(sim, |st| st.current.occupant = None);
            self.clear_held_automaton(sim, services);
        }
        let near = self.destination_proximity(sim, true);
        if near {
            self.notify_owner(sim, 0xEBA06BD4, 0, 0);
        } else {
            self.begin_idle(sim);
            self.notify_owner(sim, 0x2BAC398F, 0, 0);
        }
        near
    }

    fn install_travel_type(&self, sim: i32, travel: u32) {
        self.edit(sim, |st| {
            if st.travel_type != travel {
                st.travel_type = travel;
                st.change_counter = st.change_counter.wrapping_add(1);
            }
        });
    }

    /// Execute the verified common tail without folding observable masks.
    /// Branch selection is intentionally not delegated to a foreign service.
    pub fn apply_transport_tail(&self, sim: i32, selected: TransportTail) {
        self.install_travel_type(sim, selected.travel_type);
        self.set_state(sim, selected.state);
        if selected.stuck {
            self.change_flags(sim, |f| f | 0x40);
        }
        if selected.idle {
            self.begin_idle(sim);
            self.change_flags(sim, |f| f | 0x2401);
        }
        self.change_flags(sim, |f| f & !selected.clear_flags);
        self.change_flags(sim, |f| f | selected.set_flags);
    }
}
