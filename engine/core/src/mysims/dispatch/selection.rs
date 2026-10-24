// SPDX-License-Identifier: GPL-3.0-or-later
//! Dispatch-owned half-open leg ranges and ordered presence probes (MS21).
use super::*;

impl MySimDispatch {
    /// The route search supplies the terminal cell explicitly. Append its FE
    /// marker before initializing/selecting the first range (§4.6.3).
    pub fn install_trip_route(
        &self,
        sim: i32,
        mut route: VecDeque<RouteData>,
        terminal_cell: (u8, u8),
        services: &dyn TripServices,
    ) -> Option<bool> {
        route.push_back(RouteData {
            x: terminal_cell.0,
            z: terminal_cell.1,
            travel_type: 0xFE,
        });
        self.edit(sim, |state| {
            state.search.route = route;
            state.path_indices = (0, 0);
        });
        self.select_next_leg(sim, services)
    }

    pub fn selected_route_marker(&self, sim: i32) -> Option<RouteData> {
        let st = self.get_state(sim);
        let (end, start) = st.path_indices;
        (start as i32 >= 0 && (start as i32) < end as i32)
            .then(|| st.search.route.get(start as usize).copied())
            .flatten()
    }

    /// None denotes missing foreign input or an out-of-range persisted index.
    /// Only valid initialized indices have specified selection behavior.
    pub fn select_next_leg(&self, sim: i32, services: &dyn TripServices) -> Option<bool> {
        let st = self.get_state(sim);
        let route = &st.search.route;
        let mut start = usize::try_from(st.path_indices.0 as i32).ok()?;
        if start > route.len() {
            return None;
        }
        let mut end = start;
        while start < route.len() {
            let mode = route[start].travel_type;
            end = start + 1;
            while end < route.len() && route[end].travel_type == mode {
                end += 1;
            }
            while start < end {
                let marker = route[start];
                if services.route_network_probe(u32::from(marker.x), u32::from(marker.z), 0x1FCF, 0)? != 0 {
                    break;
                }
                start += 1;
            }
            if start < end || mode == 0xFE || end == route.len() {
                break;
            }
        }
        self.edit(sim, |state| state.path_indices = (end as u32, start as u32));
        Some(start < end)
    }

    pub fn jump_to_next_switch(&self, sim: i32, services: &dyn TripServices) -> Option<bool> {
        let st = self.get_state(sim);
        if (st.path_indices.1 as i32) >= st.path_indices.0 as i32 {
            return Some(false);
        }
        self.select_next_leg(sim, services)?;
        Some(
            self.selected_route_marker(sim)
                .is_some_and(|marker| marker.travel_type != 0xFE),
        )
    }

    pub fn dispatch_selected_leg(&self, sim: i32, automaton: Option<u64>, services: &dyn TripServices) -> Option<u32> {
        let st = self.get_state(sim);
        let start = usize::try_from(st.path_indices.1 as i32).ok()?;
        let end = usize::try_from(st.path_indices.0 as i32).ok()?;
        if start > end || end > st.search.route.len() {
            return None;
        }
        if end - start < 2 {
            return Some(0xFF);
        }
        let route = st.search.route.range(start..end).copied().collect();
        services.basic_dispatch(sim, &route, automaton)
    }

    pub fn update_selected_route(&self, sim: i32, services: &dyn TripServices) -> TripOutcome {
        let Some(marker) = self.selected_route_marker(sim) else {
            return TripOutcome::UnsupportedMs18;
        };
        self.route_ready_update(sim, marker, services)
    }

    /// Relocation is the caller effect after a successful jump, never selection.
    pub(super) fn relocate_before_switch(&self, sim: i32) -> u32 {
        let st = self.get_state(sim);
        let marker = st.search.route[st.path_indices.1 as usize - 1];
        self.edit(sim, |state| {
            state
                .current
                .set_cell_position(i32::from(marker.x), i32::from(marker.z))
        });
        u32::from(marker.travel_type)
    }
}
