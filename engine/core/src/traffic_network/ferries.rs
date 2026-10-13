// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Ferry terminals, links and neighbour links (§2.6), the terminal activation
//! and removal (§4.9.1, §4.9.4), the ferry phase of `DoConnectionsChanged`
//! (§4.9.2), the routing (§4.9.3) and the queries (§4.9.5).

use crate::grid::CellRect;
use crate::x87;

use super::model::{NetworkModel, NetworkState};
use super::{cell_key, side, travel, NetworkHost};

/// The largest ferry-terminal count per type (§4.9.1 step 1).
pub const MAX_TERMINALS_PER_TYPE: u32 = 16;
/// The neighbour connection type the ferry routing walks (§4.9.1 step 5).
pub const FERRY_NEIGHBOUR_TYPE: u32 = 9;

/// A terminal link from A to B (§2.6).
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalLink {
    /// B's departure key.
    pub to_key: u16,
    /// B's arrival cell (x2, z2) as two u16.
    pub arrival_x: u16,
    pub arrival_z: u16,
    /// `cost = r24(cells × 0.01)` (§4.9.3).
    pub cost: f32,
    /// The water route A → B (§4.10.3).
    pub route: Vec<u8>,
    /// Four u16 usage counters: (last, current) for two counts.
    pub counters: [u16; 4],
}

impl Default for TerminalLink {
    fn default() -> Self {
        TerminalLink {
            to_key: 0,
            arrival_x: 0,
            arrival_z: 0,
            cost: 0.0,
            route: Vec::new(),
            counters: [0; 4],
        }
    }
}

/// A neighbour link (§2.6): a route to an off-map cell.
#[derive(Clone, Debug, PartialEq)]
pub struct NeighborLink {
    /// The off-map cell (x, z) as i16.
    pub off_x: i16,
    pub off_z: i16,
    /// The in-city edge cell as two u16.
    pub edge_x: u16,
    pub edge_z: u16,
    /// `cost = r24(cells × 0.01)`.
    pub cost: f32,
    /// The route to the edge.
    pub route_to: Vec<u8>,
    /// The route from the edge.
    pub route_from: Vec<u8>,
    /// Four u16 usage counters.
    pub counters: [u16; 4],
}

impl Default for NeighborLink {
    fn default() -> Self {
        NeighborLink {
            off_x: 0,
            off_z: 0,
            edge_x: 0,
            edge_z: 0,
            cost: 0.0,
            route_to: Vec::new(),
            route_from: Vec::new(),
            counters: [0; 4],
        }
    }
}

/// A ferry terminal (§2.6).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct FerryTerminal {
    /// The departure cell (x1, z1) (marker `0x2C520000`); key of the
    /// departure table; gets wormhole bit 4.
    pub dep_x: u16,
    pub dep_z: u16,
    /// The arrival cell (x2, z2) (marker `0x2C530000`); key of the arrival
    /// table.
    pub arr_x: u16,
    pub arr_z: u16,
    /// The water cell (x3, z3) (marker `0x2C540000`, §4.9.1).
    pub water_x: u16,
    pub water_z: u16,
    /// The ferry type 0…2 (building property `0x0BD86FD3`).
    pub ferry_type: u8,
    /// The travel mask from the table `{1, 3}` by type; type 2 reads `0xD70A`
    /// *[quirk]*.
    pub mask: u16,
    /// The terminal links (A → B), in list order.
    pub links: Vec<TerminalLink>,
    /// The neighbour links, in list order.
    pub neighbour_links: Vec<NeighborLink>,
    /// Riders (last period, current period), u16 each (§4.9.3).
    pub riders_last: u16,
    pub riders_current: u16,
    /// The lot's location (x, z).
    pub lot_x: u16,
    pub lot_z: u16,
    /// The re-route flag (§4.9.3).
    pub reroute: bool,
}

impl FerryTerminal {
    /// The departure cell key.
    pub fn departure_key(&self) -> u16 {
        cell_key(i32::from(self.dep_x), i32::from(self.dep_z))
    }
    /// The arrival cell key.
    pub fn arrival_key(&self) -> u16 {
        cell_key(i32::from(self.arr_x), i32::from(self.arr_z))
    }
}

/// The ferry travel mask for a ferry type (§2.6, §4.1 table `0x00AB32C8`):
/// type 0 = Walk, type 1 = Walk and Car, type 2 reads past the table and gets
/// `0xD70A` *[quirk]*.
pub fn ferry_mask(ferry_type: u8) -> u16 {
    match ferry_type {
        0 | 1 => super::FERRY_MASK[ferry_type as usize],
        _ => super::FERRY_MASK_TYPE2,
    }
}

/// The periodic ferry-route pass state (§4.9.3, work-cycle state 19).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FerryPass {
    /// Whether the pass is running.
    pub running: bool,
    /// The current terminal index in the list.
    pub terminal: usize,
    /// The inner position (which other terminal / neighbour link).
    pub inner: usize,
    /// The neighbour-cell vector being walked.
    pub neighbours: Vec<(i32, i32)>,
    /// The phase within one terminal's work.
    pub phase: u32,
}

/// `cost = r24(cells × 0.01)` (§4.9.3, §11 test 9).
pub fn route_cost(cells: u32) -> f32 {
    x87::r24(f64::from(cells) * f64::from(0.01f32)) as f32
}

impl NetworkModel {
    /// `DoFerryTerminalActivated` (§4.9.1): builds a terminal from the lot's
    /// prop occupants and registers it. Returns whether it was appended. The
    /// trailing rebuild is driven by the caller.
    pub fn activate_ferry(&self, building: u64, lot_rect: CellRect) -> bool {
        let h = self.host.clone();
        self.with(|s| {
            // Step 1: the ferry type; stop if missing, > 2, or 16 already.
            let Some(ferry_type) = h
                .occupant_property(building, super::prop::FERRY_TYPE)
                .and_then(|v| v.items().first().map(|e| e.as_i64() as u32))
            else {
                return false;
            };
            if ferry_type > 2 || s.ferry_counts[ferry_type as usize] >= MAX_TERMINALS_PER_TYPE {
                return false;
            }
            // Step 2: classify the prop occupants.
            let mut dep = None;
            let mut arr = None;
            let mut water = None;
            for occ in h.prop_occupants_in_rect(lot_rect) {
                let p = h.occupant_position(occ);
                let (cx, cz) = h.position_to_cell(p[0], p[2]);
                match h.prop_type(occ) {
                    super::ferry_prop::DEPARTURE => dep = Some((cx, cz)),
                    super::ferry_prop::ARRIVAL => arr = Some((cx, cz)),
                    super::ferry_prop::WATER => {
                        // The first side d whose neighbour is outside the lot
                        // rectangle and under water.
                        for d in 0..side::COUNT {
                            let (nx, nz) = (cx + super::DX[d], cz + super::DZ[d]);
                            let outside = nx < lot_rect.x1 || nx > lot_rect.x2 || nz < lot_rect.z1 || nz > lot_rect.z2;
                            if outside && h.cell_under_water(nx, nz) {
                                water = Some((nx, nz));
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
            // All three must be found, otherwise stop. Later props of a kind
            // overwrite earlier ones (the loop above keeps the last).
            let (Some(dep), Some(arr), Some(water)) = (dep, arr, water) else {
                return false;
            };
            let (lot_x, lot_z) = h
                .lot_at(lot_rect.x1, lot_rect.z1, false)
                .map_or((lot_rect.x1, lot_rect.z1), |lot| h.lot_location(lot));
            let terminal = FerryTerminal {
                dep_x: (dep.0 & 0xFF) as u16,
                dep_z: (dep.1 & 0xFF) as u16,
                arr_x: (arr.0 & 0xFF) as u16,
                arr_z: (arr.1 & 0xFF) as u16,
                water_x: (water.0 & 0xFF) as u16,
                water_z: (water.1 & 0xFF) as u16,
                ferry_type: ferry_type as u8,
                mask: ferry_mask(ferry_type as u8),
                lot_x: (lot_x & 0xFF) as u16,
                lot_z: (lot_z & 0xFF) as u16,
                ..FerryTerminal::default()
            };
            // Step 3: append the terminal, count it, insert both keys.
            let pos = s.ferries.len() as u32;
            s.ferries.push(terminal);
            s.ferry_counts[ferry_type as usize] += 1;
            let dep_key = u32::from(cell_key(dep.0, dep.1));
            let arr_key = u32::from(cell_key(arr.0, arr.1));
            s.ferry_departures.entry(dep_key, || pos);
            s.ferry_arrivals.entry(arr_key, || pos);
            true
        })
    }

    /// `RouteAndLinkTerminals(A, B)` (§4.9.3): water FindPath between the two
    /// water cells, then find-or-append each terminal's link with the cost and
    /// route. On failure, removes both links if present.
    pub fn route_and_link(&self, a: usize, b: usize) {
        let h = self.host.clone();
        self.with(|s| {
            let (ok, cells, fwd, back) = water_path_between(s, &h, a, b);
            if !ok {
                // Failure: remove A's link to B and B's link to A, if present.
                remove_terminal_link(s, a, b);
                remove_terminal_link(s, b, a);
                return;
            }
            let b_key = s.ferries[b].departure_key();
            let a_key = s.ferries[a].departure_key();
            let cost = route_cost(cells);
            let (b_arr_x, b_arr_z) = (s.ferries[b].arr_x, s.ferries[b].arr_z);
            let (a_arr_x, a_arr_z) = (s.ferries[a].arr_x, s.ferries[a].arr_z);
            // A's link to B, route A → B.
            set_terminal_link(s, a, b_key, b_arr_x, b_arr_z, cost, fwd);
            // B's link to A, route B → A.
            set_terminal_link(s, b, a_key, a_arr_x, a_arr_z, cost, back);
        })
    }

    /// `RouteTerminalToNeighbor(T, x, z)` (§4.9.3): maps the off-map cell to
    /// its in-city cell, water FindPath from T's water cell, then find or
    /// append the neighbour link. On failure, removes it.
    pub fn route_to_neighbour(&self, t: usize, x: i32, z: i32) {
        let h = self.host.clone();
        self.with(|s| {
            // Step 1: the in-city cell; stop if −1.
            let Some(((ix, iz), _d)) = inner_cell(s, x, z) else {
                return;
            };
            let (wx, wz) = (i32::from(s.ferries[t].water_x), i32::from(s.ferries[t].water_z));
            let (ok, cells, fwd, back) = s.water.find_path(&*h, wx, wz, ix, iz);
            if !ok {
                remove_neighbour_link(s, t, x, z);
                return;
            }
            let cost = route_cost(cells);
            set_neighbour_link(s, t, x, z, ix, iz, cost, fwd, back);
        })
    }

    /// The ferry phase of `DoConnectionsChanged` (§4.9.2): processes each
    /// terminal in list order over the grown rectangle R1.
    pub(super) fn apply_ferries(&self, s: &mut NetworkState, r1: CellRect) {
        for t in 0..s.ferries.len() {
            self.apply_ferry(s, t, r1);
        }
    }

    fn apply_ferry(&self, s: &mut NetworkState, t: usize, r1: CellRect) {
        let (tx, tz) = (i32::from(s.ferries[t].dep_x), i32::from(s.ferries[t].dep_z));
        let (ax, az) = (i32::from(s.ferries[t].arr_x), i32::from(s.ferries[t].arr_z));
        let mask = s.ferries[t].mask;
        let in_r1 = |x: i32, z: i32| x >= r1.x1 && x <= r1.x2 && z >= r1.z1 && z <= r1.z2;

        // Step 1: the departure cell in R1.
        if in_r1(tx, tz) {
            if let Some(c) = s.conn_mut(tx, tz) {
                c.wormhole |= super::wormhole::FERRY;
            }
            // Make sure T has records for networks 0 and 1.
            let inv0 = s.tuning.inv_cap_table[0];
            for n in [super::net::PEDESTRIAN, super::net::CAR] {
                if s.network_info(n, tx, tz).is_none() {
                    if let Some(list) = s.info_mut(tx, tz) {
                        list.push(super::model::NetworkInfo {
                            inv_cap: inv0,
                            conn_count: 1,
                            net_type: 0,
                            net: n as u8,
                            flags: 0,
                            extra_cost: 0.0,
                            subnet: [0; 4],
                        });
                    }
                }
            }
            // For each side whose neighbour N is in the city.
            for d in 0..side::COUNT {
                let (nx, nz) = (tx + super::DX[d], tz + super::DZ[d]);
                if !s.in_city(nx, nz) {
                    continue;
                }
                let ns = super::side_of((tx, tz), (nx, nz));
                for tt in 0..travel::COUNT as u16 {
                    if mask & (1 << tt) != 0 {
                        // Roads around the terminal lead into it: for each
                        // entry e ≠ ns of N with any exit in conn[t],
                        // SetConnection(N, e, ns, t).
                        let exits: Vec<usize> = s
                            .conn_at(nx, nz)
                            .map(|c| (0..side::COUNT).filter(|&e| e != ns && c.has_exit(tt as usize, e)).collect())
                            .unwrap_or_default();
                        for e in exits {
                            super::switches::set_connection(s, nx, nz, e, ns, tt);
                        }
                    } else {
                        // Clear bit ns from every nibble of N's conn[t].
                        if let Some(c) = s.conn_mut(nx, nz) {
                            for e in 0..side::COUNT {
                                c.conn[tt as usize] &= !(1 << (ns + 4 * e));
                            }
                        }
                    }
                }
            }
        }

        // Step 2: the arrival cell in R1.
        if in_r1(ax, az) {
            for d in 0..side::COUNT {
                let (nx, nz) = (ax + super::DX[d], az + super::DZ[d]);
                if !s.in_city(nx, nz) {
                    continue;
                }
                let ns = super::side_of((ax, az), (nx, nz));
                for tt in 0..travel::COUNT as u16 {
                    if mask & (1 << tt) != 0 {
                        // For each pair (ns → x) present in N's conn[t] with
                        // x ≠ ns, SetConnection(N, ns, x, t). If any were set,
                        // also SetConnection(A, s, d, t) for s = 0…3.
                        let exits: Vec<usize> = s
                            .conn_at(nx, nz)
                            .map(|c| {
                                let nib = c.exits(tt as usize, ns);
                                (0..side::COUNT).filter(|&x| x != ns && nib & (1 << x) != 0).collect()
                            })
                            .unwrap_or_default();
                        let any = !exits.is_empty();
                        for x in exits {
                            super::switches::set_connection(s, nx, nz, ns, x, tt);
                        }
                        if any {
                            for ss in 0..side::COUNT {
                                super::switches::set_connection(s, ax, az, ss, d, tt);
                            }
                        }
                    } else {
                        // Clear N's conn[t] nibble ns.
                        if let Some(c) = s.conn_mut(nx, nz) {
                            c.conn[tt as usize] &= !(0xF << (4 * ns));
                        }
                    }
                }
            }
        }
    }

    /// Terminal removal (§4.9.4): the first terminal (list order) whose lot
    /// cell lies in the lot rectangle. At most one per call. No
    /// `DoConnectionsChanged` is run *[quirk]*.
    pub fn remove_ferry(&self, lot_rect: CellRect) {
        let h = self.host.clone();
        self.with(|s| {
            let pos = s.ferries.iter().position(|t| {
                let (lx, lz) = (i32::from(t.lot_x), i32::from(t.lot_z));
                lx >= lot_rect.x1 && lx <= lot_rect.x2 && lz >= lot_rect.z1 && lz <= lot_rect.z2
            });
            let Some(pos) = pos else { return };
            // Step 1: decrement its type count (not below 0).
            let ft = s.ferries[pos].ferry_type as usize;
            s.ferry_counts[ft] = s.ferry_counts[ft].saturating_sub(1);
            // Step 2: in every terminal reached through its links, delete the
            // link back to it.
            let key = s.ferries[pos].departure_key();
            for other in 0..s.ferries.len() {
                if other == pos {
                    continue;
                }
                if let Some(i) = s.ferries[other].links.iter().position(|l| l.to_key == key) {
                    s.ferries[other].links.remove(i);
                }
            }
            // Step 3: if the ferry pass points at it, advance the pass.
            if s.ferry_pass.running && s.ferry_pass.terminal >= pos {
                s.ferry_pass.terminal = s.ferry_pass.terminal.saturating_sub(1);
            }
            // Step 4: erase its departure and arrival keys and free it.
            s.ferry_departures.remove(u32::from(s.ferries[pos].departure_key()));
            s.ferry_arrivals.remove(u32::from(s.ferries[pos].arrival_key()));
            s.ferries.remove(pos);
            // Fix the positions stored in the two tables (they index the list).
            reindex_ferry_tables(s);
            let _ = &h;
        })
    }

    /// `GetFerryTerminalCount(type)` (slot 32): the count for `type`. No
    /// bounds check in the original; FreeCity4 returns 0 for a type > 2
    /// [choice].
    pub fn ferry_terminal_count(&self, ferry_type: u32) -> u32 {
        let s = self.s.borrow();
        if ferry_type > 2 {
            0
        } else {
            s.ferry_counts[ferry_type as usize]
        }
    }

    /// `GetFerryRouteBetweenTiles(x1, z1, x2, z2)` (slot 29): a pointer to a
    /// stored route vector, or none. Returns a copy of the route (§4.9.5).
    pub fn ferry_route_between(&self, x1: i32, z1: i32, x2: i32, z2: i32) -> Option<Vec<u8>> {
        let s = self.s.borrow();
        let in_city = |x: i32, z: i32| s.in_city(x, z);
        let k1 = u32::from(cell_key(x1, z1));
        let k2 = u32::from(cell_key(x2, z2));
        // Case 1: (x1, z1) off-map → terminal by departure key (x2, z2);
        // neighbour link with off-map cell (x1, z1); route from the edge.
        if !in_city(x1, z1) {
            let pos = *s.ferry_departures.get(k2)? as usize;
            let t = s.ferries.get(pos)?;
            let l = t.neighbour_links.iter().find(|l| l.off_x as i32 == x1 && l.off_z as i32 == z1)?;
            return Some(l.route_from.clone());
        }
        // Case 2: (x1, z1) in city, (x2, z2) off-map → terminal by departure
        // key (x1, z1); neighbour link (x2, z2); route to the edge.
        if !in_city(x2, z2) {
            let pos = *s.ferry_departures.get(k1)? as usize;
            let t = s.ferries.get(pos)?;
            let l = t.neighbour_links.iter().find(|l| l.off_x as i32 == x2 && l.off_z as i32 == z2)?;
            return Some(l.route_to.clone());
        }
        // Case 3 and 4: both in city.
        let pos = *s.ferry_departures.get(k1)? as usize;
        let t = s.ferries.get(pos)?;
        // Case 4: (x2, z2) is an arrival key of B → the link whose key is B's
        // departure key.
        if let Some(&bpos) = s.ferry_arrivals.get(k2) {
            if let Some(b) = s.ferries.get(bpos as usize) {
                let bkey = b.departure_key();
                if let Some(l) = t.links.iter().find(|l| l.to_key == bkey) {
                    return Some(l.route.clone());
                }
            }
        }
        // Case 3: (x2, z2) not an arrival key → the neighbour link whose
        // in-city edge cell is (x2, z2); route to the edge.
        let l = t
            .neighbour_links
            .iter()
            .find(|l| l.edge_x as i32 == x2 && l.edge_z as i32 == z2)?;
        Some(l.route_to.clone())
    }

    /// `GetAllFerryRoutes(list<vector<u8>>&)` (slot 30): for each terminal, a
    /// copy of every terminal link's route, then every neighbour link's route
    /// *to* the edge (§4.9.5).
    pub fn all_ferry_routes(&self) -> Vec<Vec<u8>> {
        let s = self.s.borrow();
        let mut out = Vec::new();
        for t in &s.ferries {
            for l in &t.links {
                out.push(l.route.clone());
            }
            for l in &t.neighbour_links {
                out.push(l.route_to.clone());
            }
        }
        out
    }
}

/// A `FerryRouteInfo` (§4.9.5, slot 31): the record `GetFerryRoutesInUse`
/// appends. Lots are host tokens (0 = null).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FerryRouteInfo {
    pub lot_a: u64,
    pub lot_b: u64,
    pub ferry_type: u32,
    /// A→B, B→A for counter 1.
    pub counter1: [u16; 2],
    /// A→B, B→A for counter 2.
    pub counter2: [u16; 2],
    /// The "neighbour" flag (0 terminal, 1 neighbour).
    pub neighbour: u8,
    /// The neighbour cell (−2, −2) for a terminal link.
    pub neighbour_x: i16,
    pub neighbour_z: i16,
    pub route_ab: Vec<u8>,
    pub route_ba: Vec<u8>,
}

impl NetworkModel {
    /// `GetFerryRoutesInUse(list<FerryRouteInfo>&)` (slot 31, §4.9.5).
    pub fn ferry_routes_in_use(&self) -> Vec<FerryRouteInfo> {
        let h = self.host.clone();
        self.with(|s| {
            let mut out = Vec::new();
            let mut visited: Vec<u16> = Vec::new();
            for a in 0..s.ferries.len() {
                let a_key = s.ferries[a].departure_key();
                let a_lot = lot_at_cell(&h, i32::from(s.ferries[a].lot_x), i32::from(s.ferries[a].lot_z));
                let ferry_type = u32::from(s.ferries[a].ferry_type);
                // Terminal links with a non-zero last counter.
                for li in 0..s.ferries[a].links.len() {
                    let l = s.ferries[a].links[li].clone();
                    let last_nonzero = l.counters[0] != 0 || l.counters[2] != 0;
                    if !last_nonzero {
                        continue;
                    }
                    // Find B by its departure key; skip if already visited.
                    let Some(&bpos) = s.ferry_departures.get(u32::from(l.to_key)) else {
                        continue;
                    };
                    let b_key = l.to_key;
                    if visited.contains(&b_key) {
                        continue;
                    }
                    let b_lot = lot_at_cell(&h, i32::from(s.ferries[bpos as usize].lot_x), i32::from(s.ferries[bpos as usize].lot_z));
                    // B→A counters from B's link to A (0 when absent).
                    let ba = s.ferries[bpos as usize]
                        .links
                        .iter()
                        .find(|bl| bl.to_key == a_key)
                        .map(|bl| bl.counters)
                        .unwrap_or([0; 4]);
                    out.push(FerryRouteInfo {
                        lot_a: a_lot,
                        lot_b: b_lot,
                        ferry_type,
                        counter1: [l.counters[0], ba[0]],
                        counter2: [l.counters[1], ba[1]],
                        neighbour: 0,
                        neighbour_x: -2,
                        neighbour_z: -2,
                        route_ab: l.route.clone(),
                        route_ba: Vec::new(),
                    });
                }
                // Neighbour links with a non-zero last counter.
                for l in &s.ferries[a].neighbour_links.clone() {
                    if l.counters[0] == 0 && l.counters[2] == 0 {
                        continue;
                    }
                    out.push(FerryRouteInfo {
                        lot_a: a_lot,
                        lot_b: 0,
                        ferry_type,
                        counter1: [l.counters[0], l.counters[0]],
                        counter2: [l.counters[1], l.counters[1]],
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
        })
    }
}

fn lot_at_cell(h: &dyn NetworkHost, x: i32, z: i32) -> u64 {
    h.lot_at(x, z, false).unwrap_or(0)
}

/// Water FindPath between two terminals' water cells (§4.9.3 step 1).
fn water_path_between(
    s: &mut NetworkState,
    h: &dyn NetworkHost,
    a: usize,
    b: usize,
) -> (bool, u32, Vec<u8>, Vec<u8>) {
    let (ax, az) = (i32::from(s.ferries[a].water_x), i32::from(s.ferries[a].water_z));
    let (bx, bz) = (i32::from(s.ferries[b].water_x), i32::from(s.ferries[b].water_z));
    s.water.find_path(h, ax, az, bx, bz)
}

fn set_terminal_link(s: &mut NetworkState, a: usize, to_key: u16, arr_x: u16, arr_z: u16, cost: f32, route: Vec<u8>) {
    let links = &mut s.ferries[a].links;
    match links.iter_mut().find(|l| l.to_key == to_key) {
        Some(l) => {
            l.arrival_x = arr_x;
            l.arrival_z = arr_z;
            l.cost = cost;
            l.route = route;
        }
        None => links.push(TerminalLink {
            to_key,
            arrival_x: arr_x,
            arrival_z: arr_z,
            cost,
            route,
            counters: [0; 4],
        }),
    }
}

fn remove_terminal_link(s: &mut NetworkState, a: usize, b: usize) {
    let key = s.ferries[b].departure_key();
    if let Some(i) = s.ferries[a].links.iter().position(|l| l.to_key == key) {
        s.ferries[a].links.remove(i);
    }
}

fn set_neighbour_link(
    s: &mut NetworkState,
    t: usize,
    x: i32,
    z: i32,
    ix: i32,
    iz: i32,
    cost: f32,
    to: Vec<u8>,
    from: Vec<u8>,
) {
    let links = &mut s.ferries[t].neighbour_links;
    match links.iter_mut().find(|l| l.off_x as i32 == x && l.off_z as i32 == z) {
        Some(l) => {
            l.edge_x = (ix & 0xFF) as u16;
            l.edge_z = (iz & 0xFF) as u16;
            l.cost = cost;
            l.route_to = to;
            l.route_from = from;
        }
        None => links.push(NeighborLink {
            off_x: x as i16,
            off_z: z as i16,
            edge_x: (ix & 0xFF) as u16,
            edge_z: (iz & 0xFF) as u16,
            cost,
            route_to: to,
            route_from: from,
            counters: [0; 4],
        }),
    }
}

fn remove_neighbour_link(s: &mut NetworkState, t: usize, x: i32, z: i32) {
    if let Some(i) = s.ferries[t].neighbour_links.iter().position(|l| l.off_x as i32 == x && l.off_z as i32 == z) {
        s.ferries[t].neighbour_links.remove(i);
    }
}

fn reindex_ferry_tables(s: &mut NetworkState) {
    s.ferry_departures = crate::stl::StlHashMap::new();
    s.ferry_arrivals = crate::stl::StlHashMap::new();
    for (i, t) in s.ferries.iter().enumerate() {
        s.ferry_departures.entry(u32::from(t.departure_key()), || i as u32);
        s.ferry_arrivals.entry(u32::from(t.arrival_key()), || i as u32);
    }
}

/// The in-city cell and side of an off-map cell (§2.7), on a borrowed state.
fn inner_cell(s: &NetworkState, x: i32, z: i32) -> Option<((i32, i32), usize)> {
    let x_off = x < 0 || x >= s.count_x;
    let z_off = z < 0 || z >= s.count_z;
    if x_off == z_off {
        return None;
    }
    if x < 0 {
        Some(((0, z), side::WEST))
    } else if x >= s.count_x {
        Some(((s.count_x - 1, z), side::EAST))
    } else if z < 0 {
        Some(((x, 0), side::NORTH))
    } else {
        Some(((x, s.count_z - 1), side::SOUTH))
    }
}
