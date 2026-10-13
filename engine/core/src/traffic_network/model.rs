// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network model state (§2) and its accessors and queries (§4.5, §4.13,
//! §7.1). The rebuild (§4.2) is in `rebuild.rs`, the transit switches in
//! `switches.rs`, the tunnels in `tunnels.rs`, the ferries in `ferries.rs`,
//! the subnetworks in `subnetworks.rs`, the neighbour edges in
//! `neighbours.rs` and the water pathfinder in `water.rs`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::grid::{CellRect, SimGrid};
use crate::stl::{StlHashMap, StlHashSet};
use crate::x87;

use super::tuning::Tuning;
use super::{
    array_index, cell_key, net, net_type, side, travel, GridU32, GridU8, NetworkHost, OPPOSITE_SIDE,
};
use super::switches::TransitSwitch;
use super::tunnels::TunnelInfo;
use super::water::WaterPathFinder;

/// The cell connection array side (§1: a fixed 256 × 256).
pub const ARRAY_SIDE: usize = 256;
/// The cell connection array size.
pub const ARRAY_LEN: usize = ARRAY_SIDE * ARRAY_SIDE;

/// One cell connection record (§2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellConnection {
    /// `conn[t]`, t = 0…8: the connection word of travel type `t`. Nibble `e`
    /// belongs to the entry side `e`; bit `x` of that nibble is set when a
    /// type-`t` traveller that entered from side `e` may leave through side
    /// `x` (bit index `x + 4e`).
    pub conn: [u16; 9],
    /// `switch[t]`, t = 0…8: nibble `x` = the travel type a type-`t`
    /// traveller becomes when it leaves through side `x`; `0xF` = no change.
    pub switch: [u16; 9],
    /// `priorCar[e]`, e = 0…3: car lane continuity (§2.1, §4.4). Bit
    /// `p + 4x` set = a car that entered this cell from side `e` may leave
    /// through side `x` when it entered the previous cell from side `p`.
    pub prior_car: [u16; 4],
    /// The wormhole byte (§2.1): bit 1 subway (never set), bit 2 tunnel,
    /// bit 4 ferry.
    pub wormhole: u8,
}

impl Default for CellConnection {
    /// The Init values (§1 step 4): `conn` 0, `switch` `0xFFFF`, `wormhole`
    /// 0; FreeCity4 also sets `priorCar` to `0xFFFF` *[choice]*.
    fn default() -> Self {
        CellConnection {
            conn: [0; 9],
            switch: [0xFFFF; 9],
            prior_car: [0xFFFF; 4],
            wormhole: 0,
        }
    }
}

impl CellConnection {
    /// The reset of §4.2 step 2 (the same values as `default`).
    pub fn reset(&mut self) {
        *self = CellConnection::default();
    }

    /// Whether entry side `e` may leave through side `x` for travel type `t`
    /// (bit `x + 4e` of `conn[t]`).
    pub fn connected(&self, t: usize, e: usize, x: usize) -> bool {
        self.conn[t] & (1 << (x + 4 * e)) != 0
    }

    /// Nibble `e` of `conn[t]`: the exit mask for entry side `e`.
    pub fn exits(&self, t: usize, e: usize) -> u16 {
        (self.conn[t] >> (4 * e)) & 0xF
    }

    /// Whether entry side `e` of `conn[t]` has any exit.
    pub fn has_exit(&self, t: usize, e: usize) -> bool {
        self.exits(t, e) != 0
    }

    /// Nibble `x` of `switch[t]`: the type a type-`t` traveller becomes when
    /// it leaves through side `x` (`0xF` = no change).
    pub fn switch_to(&self, t: usize, x: usize) -> u32 {
        u32::from((self.switch[t] >> (4 * x)) & 0xF)
    }
}

/// One network-info record (§2.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NetworkInfo {
    /// Inverse capacity: `invCapTable[type]`, possibly reduced near
    /// intersections (§4.5, §4.6).
    pub inv_cap: f32,
    /// The number of connections counted at the last rebuild, truncated to 8
    /// bits (§4.2).
    pub conn_count: u8,
    /// The network type 0…12 (§2.2).
    pub net_type: u8,
    /// The transit network 0…5 (§2.2).
    pub net: u8,
    /// Bit 0 = an occupant here has network flag `0x40000` (§2.2).
    pub flags: u8,
    /// The extra step cost: the damaged-road extra step cost on a damaged car
    /// cell, else 0.0 (§2.2).
    pub extra_cost: f32,
    /// Subnetwork label per entry side (§4.11); 0 = unlabelled; car network
    /// only.
    pub subnet: [u16; 4],
}

impl Default for NetworkInfo {
    fn default() -> Self {
        NetworkInfo {
            inv_cap: 0.0,
            conn_count: 0,
            net_type: 0,
            net: 0,
            flags: 0,
            extra_cost: 0.0,
            subnet: [0; 4],
        }
    }
}

/// The edge-density record the traffic simulator's maps and the pathfinder's
/// `ApplyTraffic` consume (traffic-simulator.md §4.8): 33 u16 in the SDK's
/// `0x42`-byte layout, `last[16]`, `usage`, `current[16]`, index
/// `4·entry + exit`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrafficEdgeDensity {
    pub last: [u16; 16],
    pub usage: u16,
    pub current: [u16; 16],
}

/// The shared all-zero record `GetTrafficEdgeDensity` returns for a missing
/// key (traffic-simulator.md §4.8, original `0x00AB31D0`).
pub const EDGE_DENSITY_ZERO: TrafficEdgeDensity = TrafficEdgeDensity {
    last: [0; 16],
    usage: 0,
    current: [0; 16],
};

/// One edge-density map record with its key, so the map can be saved in the
/// simulator's record (traffic-simulator.md §6.1 row 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeDensityRecord {
    pub key: u32,
    pub density: TrafficEdgeDensity,
}

/// The subnetwork info of one label (§4.12): a population map and four
/// destination capacities.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SubnetworkInfo {
    /// `map<developerType, count>`.
    pub population: BTreeMap<u32, u32>,
    /// `capacity[4]`.
    pub capacity: [u32; 4],
}

/// All of the network model's state (§2.3). Plain data, so tests and the
/// future traffic simulator can snapshot and restore it.
#[derive(Clone)]
pub struct NetworkState {
    /// The city size in cells (§1 step 1).
    pub count_x: i32,
    pub count_z: i32,
    /// The cell connection array: 65 536 records indexed `(x << 8) | z`
    /// (§2.1).
    pub conn: Vec<CellConnection>,
    /// The network-info grid: per cell, at most one record per transit
    /// network, in creation order (§2.2). City-sized, x outer.
    pub info: Vec<Vec<NetworkInfo>>,
    /// The transit-switch ID grid (`uint32`, saved, §2.3).
    pub switch_id: Option<GridU32>,
    /// The transit-switch mode-mask grid (`uint8`, saved, §2.3).
    pub switch_mode: Option<GridU8>,
    /// The next switch ID (saved, §2.3); starts at 1.
    pub next_switch_id: u32,
    /// The transit-switch table (saved, §2.3): `hash_map<u32, TransitSwitch>`,
    /// bucket hint 100.
    pub switches: StlHashMap<TransitSwitch>,
    /// The tunnel table (saved, §2.3): `hash_map<u16 portalKey, TunnelInfo>`,
    /// hint 100.
    pub tunnels: StlHashMap<TunnelInfo>,
    /// The ferry terminal list (§2.6), not saved by this module (the
    /// simulator's record saves it, traffic-simulator.md §6.1 row 19).
    pub ferries: Vec<super::ferries::FerryTerminal>,
    /// The ferry departure table: `hash_map<u16 cellKey, list position>`.
    pub ferry_departures: StlHashMap<u32>,
    /// The ferry arrival table.
    pub ferry_arrivals: StlHashMap<u32>,
    /// The ferry terminal count per ferry type [3] (§2.3): at most 16 per
    /// type.
    pub ferry_counts: [u32; 3],
    /// The ferry pass state (§4.9.3).
    pub ferry_pass: super::ferries::FerryPass,
    /// The subnetwork counter: the next label to assign (§2.3); starts at 1.
    pub subnet_counter: u16,
    /// The working subnetwork-info vector (§4.12); index 0 is unused.
    pub subnet_working: Vec<SubnetworkInfo>,
    /// The published subnetwork-info vector (§4.12); all queries read it.
    pub subnet_published: Vec<SubnetworkInfo>,
    /// The edge-density map (traffic-simulator.md §4.8): `hash_map<u32 key,
    /// TrafficEdgeDensity>`, hint 100.
    pub edge_density: StlHashMap<TrafficEdgeDensity>,
    /// The neighbour connection edge records (§2.7), one vector per map edge
    /// in the order: x < 0 (by z), x > X−1 (by z), z < 0 (by x), z > Z−1
    /// (by x).
    pub neighbour_edges: [Vec<super::neighbours::NeighbourEdge>; 4],
    /// The tuning: capacity tables and intersection-effect values (§4.5,
    /// §4.6).
    pub tuning: Tuning,
    /// The intersection dirty rectangle and flag (§4.6).
    pub intersection_dirty: bool,
    pub intersection_rect: CellRect,
    /// The "subnetworks stale" flag (§2.3): set by every rebuild.
    pub subnetworks_stale: bool,
    /// The "network changed" flag (§2.3): set at Init and by every rebuild.
    pub network_changed: bool,
    /// The water pathfinder (§4.10).
    pub water: WaterPathFinder,
    /// The congestion → inverse-speed curve the simulator sets (traffic-
    /// simulator.md §4.14); `TransitSwitchEntryCost` evaluates it (§4.7.3).
    pub congestion_curve: crate::curve::ResponseCurve,
    /// The transit multiplier the simulator sets (§4.7.3); 1.0 at
    /// construction.
    pub transit_multiplier: f32,
    /// Whether the model has run its Init (§1).
    pub initialized: bool,
}

impl Default for NetworkState {
    fn default() -> Self {
        NetworkState {
            count_x: 0,
            count_z: 0,
            conn: Vec::new(),
            info: Vec::new(),
            switch_id: None,
            switch_mode: None,
            next_switch_id: 1,
            switches: StlHashMap::new(),
            tunnels: StlHashMap::new(),
            ferries: Vec::new(),
            ferry_departures: StlHashMap::new(),
            ferry_arrivals: StlHashMap::new(),
            ferry_counts: [0; 3],
            ferry_pass: super::ferries::FerryPass::default(),
            subnet_counter: 1,
            subnet_working: Vec::new(),
            subnet_published: Vec::new(),
            edge_density: StlHashMap::new(),
            neighbour_edges: Default::default(),
            tuning: Tuning::default(),
            intersection_dirty: false,
            intersection_rect: CellRect::new(0, 0, -1, -1),
            subnetworks_stale: false,
            network_changed: false,
            water: WaterPathFinder::default(),
            congestion_curve: crate::curve::ResponseCurve::default(),
            transit_multiplier: 1.0,
            initialized: false,
        }
    }
}

impl NetworkState {
    /// The number of city cells.
    pub fn cell_count(&self) -> usize {
        (self.count_x.max(0) as usize) * (self.count_z.max(0) as usize)
    }

    /// Whether (x, z) is inside `[0, X−1] × [0, Z−1]` (§2.7 "in city").
    pub fn in_city(&self, x: i32, z: i32) -> bool {
        x >= 0 && z >= 0 && x < self.count_x && z < self.count_z
    }

    /// The cell array index when (x, z) is in city.
    fn info_index(&self, x: i32, z: i32) -> Option<usize> {
        if self.in_city(x, z) {
            Some(x as usize * self.count_z as usize + z as usize)
        } else {
            None
        }
    }

    /// The cell connection record of (x, z), or `None` when the cell array is
    /// not allocated. The array is the fixed 256 × 256, indexed by the low 8
    /// bits, so an off-map coordinate wraps as in the original (§10 Q4).
    pub fn conn_at(&self, x: i32, z: i32) -> Option<&CellConnection> {
        self.conn.get(array_index(x, z))
    }

    /// The mutable cell connection record of (x, z).
    pub fn conn_mut(&mut self, x: i32, z: i32) -> Option<&mut CellConnection> {
        self.conn.get_mut(array_index(x, z))
    }

    /// The network-info list of an in-city cell (empty for an off-map cell).
    pub fn info_at(&self, x: i32, z: i32) -> &[NetworkInfo] {
        match self.info_index(x, z) {
            Some(i) => self.info.get(i).map_or(&[], Vec::as_slice),
            None => &[],
        }
    }

    /// The mutable network-info list of an in-city cell.
    fn info_mut(&mut self, x: i32, z: i32) -> Option<&mut Vec<NetworkInfo>> {
        self.info_index(x, z).and_then(|i| self.info.get_mut(i))
    }

    /// `GetNetworkInfo(n, x, z)` (§2.2): the first record of transit network
    /// `n` in the cell's list, or `None`. No bounds check in the original;
    /// FreeCity4 returns `None` off-map [choice].
    pub fn network_info(&self, n: u32, x: i32, z: i32) -> Option<&NetworkInfo> {
        self.info_at(x, z).iter().find(|r| u32::from(r.net) == n)
    }

    /// The first car record (`n = 1`) of a cell (§4.11, §4.12).
    pub fn car_info(&self, x: i32, z: i32) -> Option<&NetworkInfo> {
        self.network_info(net::CAR, x, z)
    }

    /// The mutable first record of transit network `n` in a cell.
    pub fn network_info_mut(&mut self, n: u32, x: i32, z: i32) -> Option<&mut NetworkInfo> {
        self.info_mut(x, z)?
            .iter_mut()
            .find(|r| u32::from(r.net) == n)
    }

    /// Appends a new record of network `n` at the tail of the cell's list,
    /// reproducing the creation quirk (§2.2): appending also clears the
    /// `flags` byte of the record that was the tail before it.
    fn append_info(&mut self, n: u32, x: i32, z: i32) -> Option<usize> {
        let list = self.info_mut(x, z)?;
        if let Some(last) = list.last_mut() {
            last.flags = 0;
        }
        list.push(NetworkInfo {
            net: n as u8,
            ..NetworkInfo::default()
        });
        Some(list.len() - 1)
    }

    /// Gets or creates the record of network `n` in the cell (§4.2 step 3:
    /// "get or create the record of network n in c"). Returns its index in
    /// the cell's list.
    fn get_or_create_info(&mut self, n: u32, x: i32, z: i32) -> Option<usize> {
        if let Some(i) = self.info_at(x, z).iter().position(|r| u32::from(r.net) == n) {
            return Some(i);
        }
        self.append_info(n, x, z)
    }

    /// The switch ID at an in-city cell (0 = none); off-map reads 0
    /// [choice] (§10 Q4).
    pub fn switch_id_at(&self, x: i32, z: i32) -> u32 {
        match (&self.switch_id, self.in_city(x, z)) {
            (Some(g), true) => g.get_cell_value(x, z),
            _ => 0,
        }
    }

    /// The edge-density key of a cell, travel type and evening flag
    /// (traffic-simulator.md §4.8): `evening << 24 | t << 16 | x << 8 | z`.
    pub fn edge_key(x: i32, z: i32, t: u32, evening: bool) -> u32 {
        (u32::from(evening) << 24) | (t << 16) | ((x as u32 & 0xFF) << 8) | (z as u32 & 0xFF)
    }

    /// `GetTrafficEdgeDensity(x, z, t, evening)` (slot 12): the record, or
    /// the shared all-zero record when missing.
    pub fn edge_density_at(&self, x: i32, z: i32, t: u32, evening: bool) -> &TrafficEdgeDensity {
        let key = Self::edge_key(x, z, t, evening);
        self.edge_density.get(key).unwrap_or(&EDGE_DENSITY_ZERO)
    }

    /// The mutable edge-density record, creating it if missing (§4.2 step 3:
    /// "create the edge-density entries of the cell for every t on n, if
    /// missing").
    pub fn edge_density_mut(&mut self, x: i32, z: i32, t: u32, evening: bool) -> &mut TrafficEdgeDensity {
        let key = Self::edge_key(x, z, t, evening);
        self.edge_density.entry(key, TrafficEdgeDensity::default)
    }

    /// Erases the 18 edge-density keys of a cell (§3: on occupant removal):
    /// both evening values for all 9 travel types.
    pub fn erase_cell_edge_densities(&mut self, x: i32, z: i32) {
        for evening in [false, true] {
            for t in 0..travel::COUNT as u32 {
                let key = Self::edge_key(x, z, t, evening);
                self.edge_density.remove(key);
            }
        }
    }
}

/// The traffic network model. The game keeps this inside
/// `cSC4TrafficSimulator`; FreeCity4 keeps it as a plain object the future
/// traffic simulator owns (§9).
pub struct NetworkModel {
    host: Rc<dyn NetworkHost>,
    s: RefCell<NetworkState>,
}

impl NetworkModel {
    /// A new model with the constructor values (§1): an empty state; `Init`
    /// sizes it.
    pub fn new(host: Rc<dyn NetworkHost>) -> Rc<NetworkModel> {
        Rc::new(NetworkModel {
            host,
            s: RefCell::new(NetworkState::default()),
        })
    }

    /// A copy of the state.
    pub fn snapshot(&self) -> NetworkState {
        self.s.borrow().clone()
    }

    /// Runs `f` on the state (hosts restoring state, tests).
    pub fn with_state<R>(&self, f: impl FnOnce(&mut NetworkState) -> R) -> R {
        self.with(f)
    }

    /// Runs `f` on the state without holding a borrow while `f` runs, so `f`
    /// may call the host (a re-entrant call sees an empty state).
    pub fn with<R>(&self, f: impl FnOnce(&mut NetworkState) -> R) -> R {
        let mut st = std::mem::take(&mut *self.s.borrow_mut());
        let r = f(&mut st);
        *self.s.borrow_mut() = st;
        r
    }

    /// Reads a value out of the state.
    pub fn read<R>(&self, f: impl FnOnce(&NetworkState) -> R) -> R {
        f(&self.s.borrow())
    }

    /// The host (tests, the owning simulator).
    pub fn host(&self) -> &Rc<dyn NetworkHost> {
        &self.host
    }

    /// Sets the congestion curve and transit multiplier the simulator owns
    /// (§4.7.3, traffic-simulator.md §4.14).
    pub fn set_switch_cost_inputs(&self, curve: crate::curve::ResponseCurve, multiplier: f32) {
        let mut s = self.s.borrow_mut();
        s.congestion_curve = curve;
        s.transit_multiplier = multiplier;
    }

    // ---- Init (§1) ----

    /// The network part of the traffic simulator's `Init` (§1). Sizes the
    /// cell array, the network-info grid and the switch grids, constructs the
    /// water pathfinder and sets the next switch ID.
    pub fn init(&self) {
        let h = self.host.clone();
        self.with(|s| {
            let (x, z) = h.cell_counts();
            s.count_x = x.clamp(0, super::MAX_CITY_SIDE);
            s.count_z = z.clamp(0, super::MAX_CITY_SIDE);
            // Step 2: the network-info grid, X × Z empty lists.
            s.info = vec![Vec::new(); s.cell_count()];
            // Step 4: the cell connection array, every record reset.
            s.conn = vec![CellConnection::default(); super::ARRAY_LEN];
            // Step 5: the switch grids (created through the host as the game's
            // grid classes; simgrid §6.4).
            if s.switch_mode.is_none() {
                s.switch_mode = h.create_grid(super::CLSID_SWITCH_MODE_GRID, 1);
            }
            if s.switch_id.is_none() {
                s.switch_id = h.create_grid(super::CLSID_SWITCH_ID_GRID, 1);
            }
            // Step 6: the water pathfinder.
            s.water.init(s.count_x, s.count_z);
            // Step 7: the next switch ID.
            s.next_switch_id = 1;
            s.network_changed = true;
            s.initialized = true;
        });
    }

    // ---- capacity queries (§4.5) ----

    /// `GetCapacity(n, x, z)` (slot 23): `r24(1.0 / invCap)` of the first
    /// record of network `n` at (x, z), 0.0 when there is none. No bounds
    /// check in the original; FreeCity4 returns 0.0 off-map [choice].
    pub fn get_capacity(&self, n: u32, x: i32, z: i32) -> f32 {
        let s = self.s.borrow();
        match s.network_info(n, x, z) {
            // x87 float: 1.0 / invCap, rounded once to 24 bits.
            Some(r) => x87::r24(1.0 / f64::from(r.inv_cap)) as f32,
            None => 0.0,
        }
    }

    /// `GetConnectionCount(n, x, z)` (slot 25): the record's `connCount`, or
    /// 0. Returned zero-extended in EAX.
    pub fn get_connection_count(&self, n: u32, x: i32, z: i32) -> u32 {
        let s = self.s.borrow();
        s.network_info(n, x, z).map_or(0, |r| u32::from(r.conn_count))
    }

    // ---- connectivity queries (§4.13) ----

    /// `AreLotsConnected(A, B)` (slot 36): true when both lots have a
    /// non-empty subnetwork list and the lists share an ID.
    pub fn are_lots_connected(&self, a: u64, b: u64) -> bool {
        let la = self.lot_subnetworks(a);
        let lb = self.lot_subnetworks(b);
        !la.is_empty() && !lb.is_empty() && la.iter().any(|id| lb.contains(id))
    }

    /// `GetConnectedOccupantCount(lot, developerType)` (slot 37): the maximum
    /// over the lot's IDs (with `id < size`) of `published[id].population[type]`
    /// (0 when absent); 0 without a list.
    pub fn get_connected_occupant_count(&self, lot: u64, developer_type: u32) -> u32 {
        let s = self.s.borrow();
        let mut best = 0u32;
        for id in self.lot_subnetworks_locked(&s, lot) {
            if let Some(info) = s.subnet_published.get(id as usize) {
                best = best.max(*info.population.get(&developer_type).unwrap_or(&0));
            }
        }
        best
    }

    /// `GetConnectedDestinationCount(lot, k)` (slot 38): `k > 3` (signed) →
    /// 0; otherwise the maximum over the lot's IDs (with `id < size`) of
    /// `published[id].capacity[k]`; 0 without a list.
    pub fn get_connected_destination_count(&self, lot: u64, k: i32) -> u32 {
        if k < 0 || k > 3 {
            return 0;
        }
        let s = self.s.borrow();
        let mut best = 0u32;
        for id in self.lot_subnetworks_locked(&s, lot) {
            if let Some(info) = s.subnet_published.get(id as usize) {
                best = best.max(info.capacity[k as usize]);
            }
        }
        best
    }

    /// `GetSubnetworksForLot(lot, vector<u32>&)` (slot 39): the lot's
    /// subnetwork list (property `0x2A4F9C74`), and whether the property
    /// exists.
    pub fn get_subnetworks_for_lot(&self, lot: u64) -> (Vec<u32>, bool) {
        let v = self.lot_subnetworks(lot);
        let exists = !v.is_empty() || self.host.lot_holder(lot) != 0
            && self.host.holder_property(self.host.lot_holder(lot), super::prop::LOT_SUBNETWORKS).is_some();
        (v, exists)
    }

    /// The lot's subnetwork IDs (property `0x2A4F9C74`), empty without one.
    fn lot_subnetworks(&self, lot: u64) -> Vec<u32> {
        let holder = self.host.lot_holder(lot);
        if holder == 0 {
            return Vec::new();
        }
        Self::subnetwork_property(&self.host, holder)
    }

    fn lot_subnetworks_locked(&self, _s: &NetworkState, lot: u64) -> Vec<u32> {
        self.lot_subnetworks(lot)
    }

    /// Reads the u32 array property `0x2A4F9C74` of a holder. A scalar or an
    /// array is taken as its values; a string or object reads as empty.
    fn subnetwork_property(host: &Rc<dyn NetworkHost>, holder: u64) -> Vec<u32> {
        match host.holder_property(holder, super::prop::LOT_SUBNETWORKS) {
            Some(v) => v.items().iter().map(|e| e.as_i64() as u32).collect(),
            None => Vec::new(),
        }
    }

    /// `GetSubnetworksInRectangle(rect, vector<u32>&)` (slot 40): the distinct
    /// non-zero `subnet[0…3]` of the first car record of every in-city cell
    /// of `rect`, collected into an SGI `hash_set<u32>` (hint 100 → 193
    /// buckets, identity hash) and appended in hash order. Returns true
    /// *[choice]* where the original returns garbage in AL.
    pub fn get_subnetworks_in_rectangle(&self, rect: CellRect) -> Vec<u32> {
        let s = self.s.borrow();
        let mut set = StlHashSet::with_hint(crate::stl::STL_DEFAULT_HINT);
        for x in rect.x1.max(0)..=rect.x2.min(s.count_x - 1) {
            for z in rect.z1.max(0)..=rect.z2.min(s.count_z - 1) {
                if let Some(r) = s.car_info(x, z) {
                    for &id in &r.subnet {
                        if id != 0 {
                            set.insert(u32::from(id));
                        }
                    }
                }
            }
        }
        set.iter_order()
    }

    /// `GetSubnetworksInRegion(region, vector<u32>&)` (slot 41): as slot 40,
    /// over the cells of an `SC4CellRegion`: its bounding rectangle filtered
    /// by its bitmap. Row `x − x1`, bit `(z − z1)`, 32 bits per word, least
    /// significant first; cells outside width × height are excluded.
    pub fn get_subnetworks_in_region(&self, region: &CellRegion) -> Vec<u32> {
        let s = self.s.borrow();
        let mut set = StlHashSet::with_hint(crate::stl::STL_DEFAULT_HINT);
        for x in region.x1..=region.x2 {
            for z in region.z1..=region.z2 {
                if !s.in_city(x, z) || !region.contains(x, z) {
                    continue;
                }
                if let Some(r) = s.car_info(x, z) {
                    for &id in &r.subnet {
                        if id != 0 {
                            set.insert(u32::from(id));
                        }
                    }
                }
            }
        }
        set.iter_order()
    }

    /// `GetOccupantCountForAllSubnetworks(developerType, vector<u32>&)`
    /// (slot 42): `v.assign(size, 0)`, then `v[id] = published[id].population[type]`
    /// (if present) for id = 1 … size−1.
    pub fn get_occupant_count_for_all_subnetworks(&self, developer_type: u32) -> Vec<u32> {
        let s = self.s.borrow();
        let mut v = vec![0u32; s.subnet_published.len()];
        for id in 1..s.subnet_published.len() {
            v[id] = *s.subnet_published[id].population.get(&developer_type).unwrap_or(&0);
        }
        v
    }

    // ---- tunnel / switch / ferry lookups used by the pathfinder ----

    /// The tunnel record keyed by a portal cell (§4.8), if any.
    pub fn tunnel_at(&self, x: i32, z: i32) -> Option<TunnelInfo> {
        self.s.borrow().tunnels.get(u32::from(cell_key(x, z))).copied()
    }

    /// `TransitSwitchEntryCost(id)` (§4.7.3): an x87 float. Looks the switch
    /// up with insert-if-missing semantics *[quirk]*: an unknown ID inserts a
    /// default switch (capacity 0 → NaN ratio → blocked).
    pub fn transit_switch_entry_cost(&self, id: u32) -> f32 {
        self.with(|s| {
            // Step 1: insert-if-missing.
            let sw = s.switches.entry(id, TransitSwitch::default).clone();
            // Step 2: q = r24(load / capacity).
            let q = x87::r24(f64::from(sw.load) / f64::from(sw.capacity)) as f32;
            // Step 3/4: q < 5.0 (ordered) → the curve; otherwise (q ≥ 5, NaN
            // from 0/0, or +∞) the blocked cost.
            if q < 5.0 {
                let curve = s.congestion_curve.eval(q);
                let a = x87::r24(f64::from(curve) * f64::from(s.transit_multiplier)) as f32;
                x87::r24(f64::from(a) * f64::from(sw.time_factor)) as f32
            } else {
                super::SWITCH_COST_BLOCKED
            }
        })
    }

    /// `GetNeighborConnectionInnerCell(x, z)` (§2.7): the in-city cell and
    /// the side facing out for an off-map cell next to the map; `None` (−1)
    /// unless exactly one of x and z is off-map.
    pub fn neighbour_inner_cell(&self, x: i32, z: i32) -> Option<((i32, i32), usize)> {
        let s = self.s.borrow();
        let x_off = x < 0 || x >= s.count_x;
        let z_off = z < 0 || z >= s.count_z;
        if x_off == z_off {
            return None; // both in city or both off-map
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

    /// `HasMapExit(x, z)` (§2.7): for an off-map cell next to the map, true
    /// if any travel type's connection word at the in-city cell has an exit
    /// through the outward side (for any entry side), or if the edge record at
    /// the off-map cell has mask bit 9 set (a water/ferry neighbour
    /// connection).
    pub fn has_map_exit(&self, x: i32, z: i32) -> bool {
        let Some(((ix, iz), d)) = self.neighbour_inner_cell(x, z) else {
            return false;
        };
        let s = self.s.borrow();
        if let Some(c) = s.conn_at(ix, iz) {
            // Any travel type's connection word with an exit through the
            // outward side d, for any entry side e (bit d + 4e).
            for t in 0..travel::COUNT {
                for e in 0..side::COUNT {
                    if c.conn[t] & (1 << (d + 4 * e)) != 0 {
                        return true;
                    }
                }
            }
        }
        // Otherwise the edge record's mask bit 9.
        s.neighbour_edge_at(x, z).is_some_and(|e| e.mask & (1 << 9) != 0)
    }

    // ---- intersection capacity reduction (§4.6) ----

    /// Marks the intersection dirty rectangle (§4.2 step 1): unions `rect`
    /// into it and sets the flag.
    pub fn mark_intersection_dirty(&self, rect: CellRect) {
        let mut s = self.s.borrow_mut();
        s.intersection_rect = if s.intersection_dirty {
            union_rect(s.intersection_rect, rect)
        } else {
            rect
        };
        s.intersection_dirty = true;
    }

    /// `UpdateIntersectionData` (§4.6): runs over the dirty rectangle grown by
    /// N and clamped, then clears the flag. Called at the start of the next
    /// work cycle while the dirty flag is set (traffic-simulator.md §3.2).
    pub fn update_intersection_data(&self) {
        self.with(|s| {
            if !s.intersection_dirty {
                return;
            }
            let n = s.tuning.intersection_count as i32;
            let rect = grow_clamp(s.intersection_rect, n, s.count_x, s.count_z);
            for x in rect.x1..=rect.x2 {
                for z in rect.z1..=rect.z2 {
                    reduce_cell_intersection(s, x, z);
                }
            }
            s.intersection_dirty = false;
        });
    }
}

/// The intersection reduction of one car cell (§4.6): for a car record whose
/// type is not Highway, find the distance to the nearest intersection and set
/// `invCap = invCapTable[type]`, times `values[best]` when `best < N`.
fn reduce_cell_intersection(s: &mut NetworkState, x: i32, z: i32) {
    let Some(info) = s.info_at(x, z).iter().find(|r| r.net == net::CAR as u8).copied() else {
        return;
    };
    if u32::from(info.net_type) == net_type::HIGHWAY {
        return; // a Highway record is never reduced
    }
    let n = s.tuning.intersection_count as i32;
    let best = nearest_intersection(s, x, z, n);
    let base = s.tuning.inv_cap_table[info.net_type as usize];
    let inv = if best < n {
        // invCap = r24(invCap × values[best]).
        x87::r24(f64::from(base) * f64::from(s.tuning.intersection_values[best as usize])) as f32
    } else {
        base
    };
    if let Some(r) = s.info_at(x, z).iter().position(|r| r.net == net::CAR as u8) {
        if let Some(list) = s.info.get_mut(x as usize * s.count_z as usize + z as usize) {
            list[r].inv_cap = inv;
        }
    }
}

/// The distance to the nearest intersection from (x, z) along car edges
/// (§4.6 step 1): 0 when `connCount > 2`; otherwise a depth-first search over
/// the car (`conn[1]`) exits, union over entry sides, stopping at depth
/// `best` (which starts at N + 1). A reached cell is an intersection if its
/// car record has `connCount > 2`, or if it has no car record but a transit
/// switch whose "counts as intersection" flag is set. Does not step through
/// tunnel cells (wormhole bit 2) or cells without car info.
fn nearest_intersection(s: &NetworkState, x: i32, z: i32, n: i32) -> i32 {
    let Some(c) = s.conn_at(x, z) else { return n + 1 };
    let Some(info) = s.car_info(x, z) else { return n + 1 };
    if info.conn_count > 2 {
        return 0;
    }
    let mut best = n + 1;
    // Depth-first with an explicit stack of (cell, depth).
    let mut stack: Vec<((i32, i32), i32)> = vec![((x, z), 0)];
    let mut visited = std::collections::HashSet::new();
    visited.insert((x, z));
    while let Some(((cx, cz), depth)) = stack.pop() {
        if depth >= best {
            continue; // stop extending at depth best
        }
        let Some(conn) = s.conn_at(cx, cz) else { continue };
        // Union over entry sides of the car exits.
        for e in 0..side::COUNT {
            let exits = conn.exits(travel::CAR, e);
            for d in 0..side::COUNT {
                if exits & (1 << d) == 0 {
                    continue;
                }
                let (nx, nz) = (cx + super::DX[d], cz + super::DZ[d]);
                // The bounds check compares both coordinates with X−1
                // *[quirk]*; it matters only on non-square maps.
                if (nx as u32) > (s.count_x - 1) as u32 || (nz as u32) > (s.count_x - 1) as u32 {
                    continue;
                }
                if !visited.insert((nx, nz)) {
                    continue;
                }
                // Do not step through tunnel cells.
                let worm = s.conn_at(nx, nz).map_or(0, |c| c.wormhole);
                if worm & super::wormhole::TUNNEL != 0 {
                    continue;
                }
                if is_intersection(s, nx, nz) {
                    best = best.min(depth + 1);
                    continue;
                }
                // Only continue through cells with car info.
                if s.car_info(nx, nz).is_some() {
                    stack.push(((nx, nz), depth + 1));
                }
            }
        }
    }
    best
}

/// Whether a reached cell is an intersection (§4.6 step 1): its car record
/// has `connCount > 2`, or it has no car record but a transit switch whose
/// "counts as intersection" flag is set.
fn is_intersection(s: &NetworkState, x: i32, z: i32) -> bool {
    if let Some(info) = s.car_info(x, z) {
        return info.conn_count > 2;
    }
    let id = s.switch_id_at(x, z);
    if id == 0 {
        return false;
    }
    s.switches.get(id).is_some_and(|sw| sw.counts_as_intersection)
}

/// Grows a rectangle by `g` on every side and clamps it to the city.
pub fn grow_clamp(r: CellRect, g: i32, count_x: i32, count_z: i32) -> CellRect {
    CellRect {
        x1: (r.x1 - g).max(0),
        z1: (r.z1 - g).max(0),
        x2: (r.x2 + g).min(count_x - 1),
        z2: (r.z2 + g).min(count_z - 1),
    }
}

/// The union of two rectangles (the intersection dirty rectangle, §4.6).
pub fn union_rect(a: CellRect, b: CellRect) -> CellRect {
    if a.x1 > a.x2 || a.z1 > a.z2 {
        return b;
    }
    if b.x1 > b.x2 || b.z1 > b.z2 {
        return a;
    }
    CellRect {
        x1: a.x1.min(b.x1),
        z1: a.z1.min(b.z1),
        x2: a.x2.max(b.x2),
        z2: a.z2.max(b.z2),
    }
}

/// An `SC4CellRegion<long>` (§4.13 slot 41): a bounding rectangle plus a
/// bitmap. Row `x − x1`, bit `(z − z1)`, 32 bits per word, least significant
/// first; cells outside width × height are excluded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CellRegion {
    pub x1: i32,
    pub z1: i32,
    pub x2: i32,
    pub z2: i32,
    /// One word per row (`x − x1`), bit `(z − z1)`.
    pub rows: Vec<u32>,
}

impl CellRegion {
    /// The width and height of the bounding rectangle.
    pub fn dims(&self) -> (i32, i32) {
        (self.x2 - self.x1 + 1, self.z2 - self.z1 + 1)
    }

    /// Whether (x, z) is inside the region: inside the rectangle, within
    /// width × height, and its bit set.
    pub fn contains(&self, x: i32, z: i32) -> bool {
        let (w, h) = self.dims();
        let rx = x - self.x1;
        let rz = z - self.z1;
        if rx < 0 || rz < 0 || rx >= w || rz >= h {
            return false;
        }
        if rz >= 32 {
            return false; // one word per row
        }
        self.rows
            .get(rx as usize)
            .is_some_and(|word| word & (1 << rz) != 0)
    }
}

/// A core `u8` grid, for the private water depth map (§4.10.1).
pub type CoreU8Grid = Rc<SimGrid<u8>>;
