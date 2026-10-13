// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Ferry terminals, links and neighbour links (§2.6): activation (§4.9.1),
//! the ferry phase of `DoConnectionsChanged` (§4.9.2), routing and the
//! periodic re-routing pass (§4.9.3), removal (§4.9.4) and the queries
//! (§4.9.5).

use crate::x87;

use super::model::{key32, Cx, NetworkModel, NetworkState};
use super::{ferry_prop, msg, net, prop, side, travel, wormhole, CellRect, DX, DZ};

/// The largest ferry-terminal count per type (§4.9.1 step 1).
pub const MAX_TERMINALS_PER_TYPE: u32 = 16;
/// The neighbour connection type the ferry routing walks (§4.9.1 step 5).
pub const FERRY_NEIGHBOUR_TYPE: u32 = 9;

/// A terminal link from A to B (§2.6).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TerminalLink {
    /// B's departure key.
    pub to_key: u16,
    /// B's arrival cell (x2, z2).
    pub arrival_x: u16,
    pub arrival_z: u16,
    /// `r24(cells × 0.01)`.
    pub cost: f32,
    /// The water route A → B (§4.10.3).
    pub route: Vec<u8>,
    /// Usage counters `[last1, current1, last2, current2]`: (last, current)
    /// for two counts.
    pub counters: [u16; 4],
}

/// A neighbour link (§2.6): a route to an off-map cell.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NeighborLink {
    /// The off-map cell.
    pub off_x: i16,
    pub off_z: i16,
    /// The in-city edge cell.
    pub edge_x: u16,
    pub edge_z: u16,
    /// `r24(cells × 0.01)`.
    pub cost: f32,
    /// The route to the edge.
    pub route_to: Vec<u8>,
    /// The route from the edge.
    pub route_from: Vec<u8>,
    /// Usage counters, as for [`TerminalLink::counters`].
    pub counters: [u16; 4],
}

/// A ferry terminal (§2.6).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FerryTerminal {
    /// The departure cell (marker `0x2C520000`): key of the departure table;
    /// gets wormhole bit 4.
    pub dep_x: u16,
    pub dep_z: u16,
    /// The arrival cell (marker `0x2C530000`): key of the arrival table.
    pub arr_x: u16,
    pub arr_z: u16,
    /// The water cell next to the marker `0x2C540000` (§4.9.1).
    pub water_x: u16,
    pub water_z: u16,
    /// The ferry type 0…2 (building property `0x0BD86FD3`).
    pub ferry_type: u8,
    /// The travel mask ([`ferry_mask`]).
    pub mask: u16,
    /// The terminal links, in list order.
    pub links: Vec<TerminalLink>,
    /// The neighbour links, in list order.
    pub neighbour_links: Vec<NeighborLink>,
    /// Riders (last period, current period).
    pub riders_last: u16,
    pub riders_current: u16,
    /// The lot's location.
    pub lot_x: u16,
    pub lot_z: u16,
    /// The re-route flag (§4.9.3).
    pub reroute: bool,
}

impl FerryTerminal {
    /// The departure cell key.
    pub fn departure_key(&self) -> u16 {
        super::cell_key(i32::from(self.dep_x), i32::from(self.dep_z))
    }
    /// The arrival cell key.
    pub fn arrival_key(&self) -> u16 {
        super::cell_key(i32::from(self.arr_x), i32::from(self.arr_z))
    }
}

/// The ferry travel mask of a ferry type (§2.6, §4.1 table `0x00AB32C8`):
/// type 0 = Walk, type 1 = Walk and Car; type 2 reads past the table and
/// gets `0xD70A` *[quirk]*.
pub fn ferry_mask(ferry_type: u8) -> u16 {
    match ferry_type {
        0 | 1 => super::FERRY_MASK[usize::from(ferry_type)],
        _ => super::FERRY_MASK_TYPE2,
    }
}

/// The periodic ferry-route pass (§4.9.3, work-cycle state 19).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FerryPass {
    /// Whether the pass is running.
    pub running: bool,
    /// The next terminal (list index).
    pub terminal: usize,
    /// The cells of the type-9 neighbour connections, rebuilt when the work
    /// cycle enters the state.
    pub neighbours: Vec<(i32, i32)>,
}

/// The link cost `r24(cells × 0.01)` (§4.9.3, §11 test 9).
pub fn route_cost(cells: u32) -> f32 {
    x87::r24(f64::from(cells) * f64::from(0.01f32)) as f32
}

/// A `FerryRouteInfo` (§4.9.5, slot 31). Lots are host tokens (0 = null);
/// the glue AddRefs them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FerryRouteInfo {
    pub lot_a: u64,
    pub lot_b: u64,
    pub ferry_type: u32,
    /// Counter 1: A→B, B→A.
    pub counter1: [u16; 2],
    /// Counter 2: A→B, B→A.
    pub counter2: [u16; 2],
    /// The "neighbour" flag (0 terminal link, 1 neighbour link).
    pub neighbour: u8,
    /// The neighbour cell; (−2, −2) for a terminal link.
    pub neighbour_x: i16,
    pub neighbour_z: i16,
    pub route_ab: Vec<u8>,
    pub route_ba: Vec<u8>,
}

impl NetworkState {
    /// The list index of the terminal with departure key `key`.
    pub fn terminal_by_departure(&self, key: u16) -> Option<usize> {
        self.ferry_departures.get(u32::from(key)).map(|&p| p as usize)
    }

    /// The ferry phase of `DoConnectionsChanged` (§4.9.2): every terminal in
    /// list order, against the grown rectangle R1.
    pub(crate) fn apply_ferries(&mut self, r1: CellRect) {
        for t in 0..self.ferries.len() {
            self.apply_ferry(t, r1);
        }
    }

    fn apply_ferry(&mut self, t: usize, r1: CellRect) {
        let term = &self.ferries[t];
        let (tx, tz) = (i32::from(term.dep_x), i32::from(term.dep_z));
        let (ax, az) = (i32::from(term.arr_x), i32::from(term.arr_z));
        let m = term.mask;
        let in_r1 = |x: i32, z: i32| x >= r1.x1 && x <= r1.x2 && z >= r1.z1 && z <= r1.z2;

        // Step 1: the departure cell T.
        if in_r1(tx, tz) {
            if let Some(c) = self.conn_mut(tx, tz) {
                c.wormhole |= wormhole::FERRY;
            }
            // Records for networks 0 and 1; a new one gets connCount 1, type
            // 0, invCapTable[0] and extraCost 0 (appended with the §2.2
            // quirk).
            let inv0 = self.tuning.inv_cap_table[0];
            for n in [net::PEDESTRIAN, net::CAR] {
                if self.network_info(n, tx, tz).is_none() {
                    if let Some(r) = self.append_info(n, tx, tz) {
                        r.conn_count = 1;
                        r.net_type = 0;
                        r.inv_cap = inv0;
                        r.extra_cost = 0.0;
                    }
                }
            }
            for d in 0..side::COUNT {
                let (nx, nz) = (tx + DX[d], tz + DZ[d]);
                if !self.in_city(nx, nz) {
                    continue;
                }
                let ns = super::side_of((tx, tz), (nx, nz));
                for tt in 0..travel::COUNT {
                    if m & (1 << tt) != 0 {
                        // Roads around the terminal lead into it.
                        let entries: Vec<usize> = self.conn_at(nx, nz).map_or_else(Vec::new, |c| {
                            (0..side::COUNT).filter(|&e| e != ns && c.exits(tt, e) != 0).collect()
                        });
                        for e in entries {
                            self.set_connection(nx, nz, e, ns, tt);
                        }
                    } else if let Some(c) = self.conn_mut(nx, nz) {
                        // Clear bit ns from every nibble.
                        c.conn[tt] &= !(0x1111u16 << ns);
                    }
                }
            }
        }

        // Step 2: the arrival cell A.
        if in_r1(ax, az) {
            for d in 0..side::COUNT {
                let (nx, nz) = (ax + DX[d], az + DZ[d]);
                if !self.in_city(nx, nz) {
                    continue;
                }
                let ns = super::side_of((ax, az), (nx, nz));
                for tt in 0..travel::COUNT {
                    if m & (1 << tt) != 0 {
                        let exits: Vec<usize> = self.conn_at(nx, nz).map_or_else(Vec::new, |c| {
                            (0..side::COUNT)
                                .filter(|&x| x != ns && c.connected(tt, ns, x))
                                .collect()
                        });
                        let any = !exits.is_empty();
                        for x in exits {
                            self.set_connection(nx, nz, ns, x, tt);
                        }
                        if any {
                            for s in 0..side::COUNT {
                                self.set_connection(ax, az, s, d, tt);
                            }
                        }
                    } else if let Some(c) = self.conn_mut(nx, nz) {
                        c.conn[tt] &= !(0xFu16 << (4 * ns));
                    }
                }
            }
        }
    }

    /// Finds or appends A's link keyed by `to_key` and sets its fields
    /// (§4.9.3); a new link starts with zero counters and keeps its place.
    fn set_terminal_link(&mut self, a: usize, to_key: u16, arrival: (u16, u16), cost: f32, route: Vec<u8>) {
        let links = &mut self.ferries[a].links;
        let i = match links.iter().position(|l| l.to_key == to_key) {
            Some(i) => i,
            None => {
                links.push(TerminalLink {
                    to_key,
                    ..TerminalLink::default()
                });
                links.len() - 1
            }
        };
        let l = &mut links[i];
        (l.arrival_x, l.arrival_z) = arrival;
        l.cost = cost;
        l.route = route;
    }

    fn remove_terminal_link(&mut self, a: usize, to_key: u16) {
        self.ferries[a].links.retain(|l| l.to_key != to_key);
    }
}

impl Cx<'_> {
    /// `DoFerryTerminalActivated` (§4.9.1). Returns whether a terminal was
    /// appended.
    pub(crate) fn ferry_activated(&mut self, lot: u64, building: u64, lot_rect: CellRect) -> bool {
        let h = self.h;
        // Step 1: the ferry type (u8); stop if missing, > 2, or 16 already.
        let Some(ty) = h
            .occupant_property(building, prop::FERRY_TYPE)
            .and_then(|v| v.items().first().map(|e| e.as_i64() as u8))
        else {
            return false;
        };
        if ty > 2 || self.s.ferry_counts[usize::from(ty)] >= MAX_TERMINALS_PER_TYPE {
            return false;
        }
        // Step 2: classify the prop occupants of the lot rectangle; later
        // props of a kind overwrite earlier ones.
        let (mut dep, mut arr, mut water) = (None, None, None);
        for occ in h.prop_occupants_in_rect(lot_rect) {
            let p = h.occupant_position(occ);
            let (cx, cz) = h.position_to_cell(p[0], p[2]);
            match h.prop_type(occ) {
                ferry_prop::DEPARTURE => dep = Some((cx, cz)),
                ferry_prop::ARRIVAL => arr = Some((cx, cz)),
                ferry_prop::WATER => {
                    // The first side whose neighbour is outside the lot
                    // rectangle and under water. A water prop with no such
                    // side leaves an earlier one [choice].
                    let found = (0..side::COUNT).map(|d| (cx + DX[d], cz + DZ[d])).find(|&(nx, nz)| {
                        let outside = nx < lot_rect.x1 || nx > lot_rect.x2 || nz < lot_rect.z1 || nz > lot_rect.z2;
                        outside && h.cell_under_water(nx, nz)
                    });
                    if found.is_some() {
                        water = found;
                    }
                }
                _ => {}
            }
        }
        let (Some(dep), Some(arr), Some(water)) = (dep, arr, water) else {
            return false;
        };
        let (lx, lz) = h.lot_location(lot);
        let terminal = FerryTerminal {
            dep_x: dep.0 as u16,
            dep_z: dep.1 as u16,
            arr_x: arr.0 as u16,
            arr_z: arr.1 as u16,
            water_x: water.0 as u16,
            water_z: water.1 as u16,
            ferry_type: ty,
            mask: ferry_mask(ty),
            lot_x: lx as u16,
            lot_z: lz as u16,
            ..FerryTerminal::default()
        };
        // Step 3: append, count, insert both keys (an existing key keeps its
        // terminal: `hash_map::insert`).
        let pos = self.s.ferries.len();
        let (dk, ak) = (u32::from(terminal.departure_key()), u32::from(terminal.arrival_key()));
        self.s.ferries.push(terminal);
        self.s.ferry_counts[usize::from(ty)] += 1;
        self.s.ferry_departures.entry(dk, || pos as u32);
        self.s.ferry_arrivals.entry(ak, || pos as u32);
        // Step 4: route to every other terminal of the same type.
        for other in 0..pos {
            if self.s.ferries[other].ferry_type == ty {
                self.route_and_link(pos, other);
            }
        }
        // Step 5: route to every type-9 neighbour connection.
        for conn in h.connections_of_type(FERRY_NEIGHBOUR_TYPE) {
            let (x, z) = h.connection_cell(conn);
            self.route_to_neighbour(pos, x, z);
        }
        true
    }

    /// `RouteAndLinkTerminals(A, B)` (§4.9.3).
    pub(crate) fn route_and_link(&mut self, a: usize, b: usize) {
        let (ta, tb) = (&self.s.ferries[a], &self.s.ferries[b]);
        let (a_key, b_key) = (ta.departure_key(), tb.departure_key());
        let (a_arr, b_arr) = ((ta.arr_x, ta.arr_z), (tb.arr_x, tb.arr_z));
        let (sx, sz) = (i32::from(ta.water_x), i32::from(ta.water_z));
        let (tx, tz) = (i32::from(tb.water_x), i32::from(tb.water_z));
        match self.s.water.find_path(self.h, sx, sz, tx, tz) {
            None => {
                self.s.remove_terminal_link(a, b_key);
                self.s.remove_terminal_link(b, a_key);
            }
            Some(p) => {
                let cost = route_cost(p.cells);
                self.s.set_terminal_link(a, b_key, b_arr, cost, p.forward);
                self.s.set_terminal_link(b, a_key, a_arr, cost, p.backward);
            }
        }
    }

    /// `RouteTerminalToNeighbor(T, x, z)` (§4.9.3).
    pub(crate) fn route_to_neighbour(&mut self, t: usize, x: i32, z: i32) {
        let Some(((ix, iz), _)) = self.s.neighbor_connection_inner_cell(x, z) else {
            return;
        };
        let term = &self.s.ferries[t];
        let (wx, wz) = (i32::from(term.water_x), i32::from(term.water_z));
        let (ox, oz) = (x as i16, z as i16);
        let found = self.s.water.find_path(self.h, wx, wz, ix, iz);
        let links = &mut self.s.ferries[t].neighbour_links;
        let pos = links.iter().position(|l| l.off_x == ox && l.off_z == oz);
        match (found, pos) {
            (None, Some(i)) => {
                links.remove(i);
            }
            (None, None) => {}
            (Some(p), pos) => {
                let i = pos.unwrap_or_else(|| {
                    links.push(NeighborLink {
                        off_x: ox,
                        off_z: oz,
                        ..NeighborLink::default()
                    });
                    links.len() - 1
                });
                let l = &mut links[i];
                (l.edge_x, l.edge_z) = (ix as u16, iz as u16);
                l.cost = route_cost(p.cells);
                l.route_to = p.forward;
                l.route_from = p.backward;
            }
        }
    }

    /// Terminal removal (§4.9.4): the first terminal (list order) whose lot
    /// cell lies in the lot rectangle; at most one per call. No
    /// `DoConnectionsChanged` is run *[quirk]*.
    pub(crate) fn ferry_lot_deactivated(&mut self, r: CellRect) {
        let s = &mut *self.s;
        let Some(pos) = s.ferries.iter().position(|t| {
            let (lx, lz) = (i32::from(t.lot_x), i32::from(t.lot_z));
            lx >= r.x1 && lx <= r.x2 && lz >= r.z1 && lz <= r.z2
        }) else {
            return;
        };
        // Step 1: its type count, not below 0.
        let ty = usize::from(s.ferries[pos].ferry_type).min(2);
        s.ferry_counts[ty] = s.ferry_counts[ty].saturating_sub(1);
        // Step 2: in every terminal reached through its links, delete the
        // link back to it.
        let key = s.ferries[pos].departure_key();
        let targets: Vec<u16> = s.ferries[pos].links.iter().map(|l| l.to_key).collect();
        for to_key in targets {
            if let Some(b) = s.terminal_by_departure(to_key) {
                if b != pos && b < s.ferries.len() {
                    s.remove_terminal_link(b, key);
                }
            }
        }
        // Step 3: a running pass that points at it moves on to the next
        // terminal (which takes its index); later indices shift down.
        if s.ferry_pass.running && s.ferry_pass.terminal > pos {
            s.ferry_pass.terminal -= 1;
        }
        // Step 4: erase its keys and free it; the surviving positions above
        // it shift down.
        let (dk, ak) = (s.ferries[pos].departure_key(), s.ferries[pos].arrival_key());
        s.ferry_departures.remove(u32::from(dk));
        s.ferry_arrivals.remove(u32::from(ak));
        s.ferries.remove(pos);
        for table in [&mut s.ferry_departures, &mut s.ferry_arrivals] {
            for k in table.keys() {
                if let Some(v) = table.get_mut(k) {
                    if *v as usize > pos {
                        *v -= 1;
                    }
                }
            }
        }
    }

    /// Entering the periodic ferry state (§4.9.3, `0x0072134C`): rotates the
    /// rider and link counters, sets every re-route flag, rebuilds the
    /// type-9 neighbour-cell vector and starts at the first terminal. Returns
    /// the sum of the riders that became "last" (the simulator adds it to a
    /// city total).
    pub(crate) fn begin_ferry_pass(&mut self) -> u32 {
        let mut total = 0u32;
        for t in &mut self.s.ferries {
            t.riders_last = t.riders_current;
            t.riders_current = 0;
            total += u32::from(t.riders_last);
            for c in t
                .links
                .iter_mut()
                .map(|l| &mut l.counters)
                .chain(t.neighbour_links.iter_mut().map(|l| &mut l.counters))
            {
                *c = [c[1], 0, c[3], 0];
            }
            t.reroute = true;
        }
        let h = self.h;
        self.s.ferry_pass = FerryPass {
            running: true,
            terminal: 0,
            neighbours: h
                .connections_of_type(FERRY_NEIGHBOUR_TYPE)
                .into_iter()
                .map(|c| h.connection_cell(c))
                .collect(),
        };
        total
    }

    /// One terminal of the periodic pass (§4.9.3). Returns true when the list
    /// is exhausted; the pass then stops and posts `0x2C06BA82` (§5).
    pub(crate) fn ferry_pass_step(&mut self) -> bool {
        if !self.s.ferry_pass.running {
            return true;
        }
        let a = self.s.ferry_pass.terminal;
        if a < self.s.ferries.len() {
            let ty = self.s.ferries[a].ferry_type;
            for b in 0..self.s.ferries.len() {
                if b != a && self.s.ferries[b].ferry_type == ty && self.s.ferries[b].reroute {
                    self.route_and_link(a, b);
                }
            }
            self.s.ferries[a].reroute = false;
            let cells = self.s.ferry_pass.neighbours.clone();
            for (x, z) in cells {
                self.route_to_neighbour(a, x, z);
            }
            self.s.ferry_pass.terminal = a + 1;
        }
        if self.s.ferry_pass.terminal >= self.s.ferries.len() {
            self.s.ferry_pass.running = false;
            self.h.post(msg::FERRY_ROUTES_DONE);
            return true;
        }
        false
    }
}

impl NetworkModel {
    /// `RouteAndLinkTerminals(A, B)` on two terminal list indices (§4.9.3).
    pub fn route_and_link_terminals(&self, a: usize, b: usize) {
        self.run(|cx| {
            if a < cx.s.ferries.len() && b < cx.s.ferries.len() {
                cx.route_and_link(a, b);
            }
        });
    }

    /// `RouteTerminalToNeighbor(T, x, z)` on a terminal list index (§4.9.3).
    pub fn route_terminal_to_neighbor(&self, t: usize, x: i32, z: i32) {
        self.run(|cx| {
            if t < cx.s.ferries.len() {
                cx.route_to_neighbour(t, x, z);
            }
        });
    }

    /// Enters the periodic ferry state (§4.9.3). Returns the riders total.
    pub fn begin_ferry_pass(&self) -> u32 {
        self.run(|cx| cx.begin_ferry_pass())
    }

    /// Works on the periodic ferry pass for at most `max_terminals`
    /// terminals (the original slices this by time; slicing changes only
    /// when the pass finishes). Returns true when the pass is finished.
    pub fn work_on_ferry_routes(&self, max_terminals: usize) -> bool {
        self.run(|cx| {
            for _ in 0..max_terminals.max(1) {
                if cx.ferry_pass_step() {
                    return true;
                }
            }
            !cx.s.ferry_pass.running
        })
    }

    /// `GetFerryTerminalCount(type)` (slot 32). The original has no bounds
    /// check; a type above 2 gives 0 [choice].
    pub fn ferry_terminal_count(&self, ferry_type: u32) -> u32 {
        self.s
            .borrow()
            .ferry_counts
            .get(ferry_type as usize)
            .copied()
            .unwrap_or(0)
    }

    /// `GetFerryRouteBetweenTiles(x1, z1, x2, z2)` (slot 29): a copy of the
    /// stored route the original returns a pointer to, or `None` (§4.9.5).
    pub fn ferry_route_between_tiles(&self, x1: i32, z1: i32, x2: i32, z2: i32) -> Option<Vec<u8>> {
        let s = self.s.borrow();
        let (o1, o2) = (!s.in_city(x1, z1), !s.in_city(x2, z2));
        let find_nl = |t: &FerryTerminal, x: i32, z: i32| {
            t.neighbour_links
                .iter()
                .find(|l| l.off_x == x as i16 && l.off_z == z as i16)
                .cloned()
        };
        if o1 {
            // By departure key (x2, z2); its neighbour link to (x1, z1); the
            // route from the edge.
            let t = s.ferries.get(s.terminal_by_departure(super::cell_key(x2, z2))?)?;
            return find_nl(t, x1, z1).map(|l| l.route_from);
        }
        let t = s.ferries.get(s.terminal_by_departure(super::cell_key(x1, z1))?)?;
        if o2 {
            return find_nl(t, x2, z2).map(|l| l.route_to);
        }
        match s.ferry_arrivals.get(key32(x2, z2)) {
            // (x2, z2) is an arrival key of B: A's link keyed by B's
            // departure key.
            Some(&b) => {
                let bkey = s.ferries.get(b as usize)?.departure_key();
                t.links.iter().find(|l| l.to_key == bkey).map(|l| l.route.clone())
            }
            // Otherwise: the neighbour link whose in-city edge cell is
            // (x2, z2), its route to the edge.
            None => t
                .neighbour_links
                .iter()
                .find(|l| i32::from(l.edge_x) == x2 && i32::from(l.edge_z) == z2)
                .map(|l| l.route_to.clone()),
        }
    }

    /// `GetAllFerryRoutes(list<vector<u8>>&)` (slot 30): for each terminal,
    /// every terminal link's route, then every neighbour link's route to the
    /// edge.
    pub fn all_ferry_routes(&self) -> Vec<Vec<u8>> {
        let s = self.s.borrow();
        let mut out = Vec::new();
        for t in &s.ferries {
            out.extend(t.links.iter().map(|l| l.route.clone()));
            out.extend(t.neighbour_links.iter().map(|l| l.route_to.clone()));
        }
        out
    }

    /// `GetFerryRoutesInUse(list<FerryRouteInfo>&)` (slot 31, §4.9.5). A link
    /// is in use when either "last" counter (`counters[0]`, `counters[2]`) is
    /// non-zero; the reported counters are the "last" ones [inferred: the
    /// test uses them].
    pub fn ferry_routes_in_use(&self) -> Vec<FerryRouteInfo> {
        // The lots first (host calls), so no borrow is held across them.
        let lot_cells: Vec<(i32, i32)> = self
            .s
            .borrow()
            .ferries
            .iter()
            .map(|t| (i32::from(t.lot_x), i32::from(t.lot_z)))
            .collect();
        let lots: Vec<u64> = lot_cells
            .iter()
            .map(|&(x, z)| self.host.lot_at(x, z, false).unwrap_or(0))
            .collect();
        let s = self.s.borrow();
        let mut out = Vec::new();
        let mut visited: Vec<u16> = Vec::new();
        for (a, ta) in s.ferries.iter().enumerate() {
            let a_key = ta.departure_key();
            for l in &ta.links {
                if l.counters[0] == 0 && l.counters[2] == 0 {
                    continue;
                }
                if visited.contains(&l.to_key) {
                    continue;
                }
                let b = s.terminal_by_departure(l.to_key);
                let back = b.and_then(|b| s.ferries.get(b)?.links.iter().find(|bl| bl.to_key == a_key));
                let ba = back.map_or([0; 4], |bl| bl.counters);
                out.push(FerryRouteInfo {
                    lot_a: lots[a],
                    lot_b: b.and_then(|b| lots.get(b).copied()).unwrap_or(0),
                    ferry_type: u32::from(ta.ferry_type),
                    counter1: [l.counters[0], ba[0]],
                    counter2: [l.counters[2], ba[2]],
                    neighbour: 0,
                    neighbour_x: -2,
                    neighbour_z: -2,
                    route_ab: l.route.clone(),
                    route_ba: back.map_or_else(Vec::new, |bl| bl.route.clone()),
                });
            }
            for l in &ta.neighbour_links {
                if l.counters[0] == 0 && l.counters[2] == 0 {
                    continue;
                }
                out.push(FerryRouteInfo {
                    lot_a: lots[a],
                    lot_b: 0,
                    ferry_type: u32::from(ta.ferry_type),
                    counter1: [l.counters[0], l.counters[0]],
                    counter2: [l.counters[2], l.counters[2]],
                    neighbour: 1,
                    neighbour_x: l.off_x,
                    neighbour_z: l.off_z,
                    route_ab: l.route_to.clone(),
                    route_ba: l.route_from.clone(),
                });
            }
            visited.push(a_key);
        }
        out
    }
}
