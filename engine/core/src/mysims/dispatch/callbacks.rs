// SPDX-License-Identifier: GPL-3.0-or-later
//! Public callback semantics from mysims §4.6.5.
use super::*;

impl MySimDispatch {
    /// Message 09D92FC3. The sender identity is a captured input, not a
    /// guessed dispatch/automaton pointer. Unknown subtypes are inert.
    pub fn automaton_callback(
        &self,
        sim: i32,
        subtype: u32,
        sender: Option<u64>,
        services: &dyn TripServices,
    ) -> TripOutcome {
        let st = self.get_state(sim);
        let matching = st.automata_occupant.is_some() && sender == st.automata_occupant;
        match subtype {
            1 if matching && st.search.flags & 1 != 0 => {
                self.change_flags(sim, |flags| flags | 0x2000);
                self.animation_message(sim, services)
            }
            2 if matching => {
                if st.search.flags & 0x10000 != 0 {
                    self.exit_u_drive_it(sim, services);
                    TripOutcome::Accepted
                } else {
                    let outcome = self.go_to(sim, st.destination, services);
                    self.change_flags(sim, |flags| flags | 0x401);
                    outcome
                }
            }
            5 if st.search.state == 2 && st.search.flags & 0x10000 == 0 => {
                let Some(occupant) = st.automata_occupant else {
                    return TripOutcome::UnsupportedMs18;
                };
                let Some(mut route) = services.remaining_automaton_route(occupant) else {
                    return TripOutcome::UnsupportedMs18;
                };
                if route.len() > 1 {
                    route.pop_front();
                    return match services.submit_remaining_route(occupant, &route, true, false) {
                        Some(()) => TripOutcome::Accepted,
                        None => TripOutcome::UnsupportedMs18,
                    };
                }
                let Some(world) = services.automaton_position(occupant) else {
                    return TripOutcome::UnsupportedMs18;
                };
                self.edit(sim, |state| {
                    state.current.world = world;
                    state.current.cell = (world_to_cell(world.0), world_to_cell(world.1));
                });
                self.change_flags(sim, |flags| flags | 0x420);
                if self.select_next_leg(sim, services).is_none() {
                    return TripOutcome::UnsupportedMs18;
                }
                self.update_selected_route(sim, services)
            }
            9 if matching => {
                if let Some(occupant) = st.current.occupant {
                    services.release_location_occupant(occupant);
                    self.edit(sim, |state| {
                        state.current.occupant = None;
                        state.change_counter = state.change_counter.wrapping_add(1);
                    });
                }
                if self.check_flags(sim, 0x20000) {
                    self.held_automaton_player_update(sim, services);
                }
                let Some(occupant) = self.get_state(sim).automata_occupant else {
                    return TripOutcome::Rejected;
                };
                let Some(world) = services.automaton_position(occupant) else {
                    return TripOutcome::UnsupportedMs18;
                };
                let cell = (world_to_cell(world.0), world_to_cell(world.1));
                let current = self.get_state(sim);
                if cell != current.current.cell && current.search.flags & 0x10000 == 0 {
                    let dx = cell.0.wrapping_sub(current.destination.cell.0);
                    let dz = cell.1.wrapping_sub(current.destination.cell.1);
                    let near = dx.wrapping_mul(dx).wrapping_add(dz.wrapping_mul(dz))
                        < current
                            .tuning
                            .approach_distance
                            .wrapping_mul(current.tuning.approach_distance);
                    self.change_flags(sim, |flags| if near { flags | 0x8000 } else { flags & !0x8000 });
                }
                self.edit(sim, |state| {
                    state.current.world = world;
                    state.current.cell = cell;
                });
                TripOutcome::Accepted
            }
            _ => TripOutcome::Accepted,
        }
    }

    /// Message 0BB324BD; only data1=1 while player-driving exits control.
    pub fn mission_callback(&self, sim: i32, data1: u32, services: &dyn TripServices) {
        if data1 == 1 && self.check_flags(sim, 0x10000) {
            self.exit_u_drive_it(sim, services);
        }
    }
}
