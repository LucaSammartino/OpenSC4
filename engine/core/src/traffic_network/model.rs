// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network model state (§2), Init (§1), the capacity and connectivity
//! queries (§4.5, §4.13, §7.1) and the intersection capacity reduction
//! (§4.6).

use std::cell::{Ref, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::curve::ResponseCurve;
use crate::stl::{StlHashMap, StlHashSet, STL_DEFAULT_HINT};
use crate::variant::Variant;
use crate::x87;

use super::ferries::{FerryPass, FerryTerminal};
use super::neighbours::NeighbourEdge;
use super::switches::TransitSwitch;
use super::tuning::Tuning;
use super::tunnels::{InvMaxSpeed, TunnelInfo};
use super::water::WaterPathFinder;
use super::{
    array_index, cell_key, net, net_type, prop, side, travel, wormhole, CellRect, CellRegion, GridU32, GridU8,
    NetworkHost, DX, DZ, NET_OF_TRAVEL, OPPOSITE_SIDE,
};

/// The cell connection array side (§1: a fixed 256 × 256).
pub const ARRAY_SIDE: usize = 256;
/// The cell connection array size.
pub const ARRAY_LEN: usize = ARRAY_SIDE * ARRAY_SIDE;

/// One cell connection record (§2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellConnection {
    /// `conn[t]`, t = 0…8: nibble `e` belongs to the entry side `e`; bit `x`
    /// of that nibble is set when a type-`t` traveller that entered from side
    /// `e` may leave through side `x` (bit index `x + 4e`).
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
    /// The reset of §4.2 step 2: `conn[t] = 0`, `switch[t] = 0xFFFF`,
    /// `priorCar = 0xFFFF`, `wormhole = 0` (the same values as `default`).
    pub fn reset(&mut self) {
        *self = CellConnection::default();
    }

    /// Whether entry side `e` may leave through side `x` for travel type `t`.
    pub fn connected(&self, t: usize, e: usize, x: usize) -> bool {
        self.conn[t] & (1 << (x + 4 * e)) != 0
    }

    /// Nibble `e` of `conn[t]`: the exit mask for entry side `e`.
    pub fn exits(&self, t: usize, e: usize) -> u16 {
        (self.conn[t] >> (4 * e)) & 0xF
    }

    /// Whether `conn[t]` has an exit through side `x` from some entry side
    /// (bit `x` of any nibble).
    pub fn has_exit_through(&self, t: usize, x: usize) -> bool {
        self.conn[t] & (0x1111 << x) != 0
    }

    /// The union of the exits of every entry side of `conn[t]`.
    pub fn exit_union(&self, t: usize) -> u16 {
        (0..side::COUNT).fold(0, |m, e| m | self.exits(t, e))
    }

    /// Nibble `x` of `switch[t]`: the type a type-`t` traveller becomes when
    /// it leaves through side `x` (`0xF` = no change).
    pub fn switch_to(&self, t: usize, x: usize) -> u32 {
        u32::from((self.switch[t] >> (4 * x)) & 0xF)
    }
}

/// One network-info record (§2.2).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

/// The edge-density record (traffic-simulator.md §4.8): 33 u16 in the SDK's
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

/// The subnetwork info of one label (§4.12, `0x1C` bytes): a population map
/// and four destination capacities.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SubnetworkInfo {
    /// `map<developerType, count>`.
    pub population: BTreeMap<u32, u32>,
    /// `capacity[4]`.
    pub capacity: [u32; 4],
}

/// What `GetTransitSwitchQueryData` (slot 35) found at a cell (§7.1): a
/// switch by the ID grid, else a ferry terminal (its list index). The data
/// fields belong to traffic-simulator.md.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchQueryTarget {
    Switch(u32),
    Ferry(usize),
}

/// All of the network model's state (§2.3). Plain data, so tests and the
/// future traffic simulator can inspect, save and restore it.
#[derive(Clone)]
pub struct NetworkState {
    /// The city size in cells (§1 step 1).
    pub count_x: i32,
    pub count_z: i32,
    /// The cell connection array: 65 536 records indexed `(x << 8) | z`
    /// (§2.1). Empty before Init.
    pub conn: Vec<CellConnection>,
    /// The network-info grid: per cell (`x · Z + z`), at most one record per
    /// transit network, in creation order (§2.2).
    pub info: Vec<Vec<NetworkInfo>>,
    /// The transit-switch ID grid (`uint32`, saved, §2.3).
    pub switch_id: Option<GridU32>,
    /// The transit-switch mode-mask grid (`uint8`, saved, §2.3).
    pub switch_mode: Option<GridU8>,
    /// The next switch ID (saved, §2.3); starts at 1.
    pub next_switch_id: u32,
    /// The transit-switch table (saved): `hash_map<u32, TransitSwitch>`,
    /// bucket hint 100.
    pub switches: StlHashMap<TransitSwitch>,
    /// The tunnel table (saved): `hash_map<u16 portalKey, TunnelInfo>`.
    pub tunnels: StlHashMap<TunnelInfo>,
    /// The ferry terminal list (§2.6).
    pub ferries: Vec<FerryTerminal>,
    /// The ferry departure table: `hash_map<u16 cellKey, list position>`.
    pub ferry_departures: StlHashMap<u32>,
    /// The ferry arrival table.
    pub ferry_arrivals: StlHashMap<u32>,
    /// The ferry terminal count per ferry type (§2.3): at most 16 per type.
    pub ferry_counts: [u32; 3],
    /// The ferry pass state (§4.9.3).
    pub ferry_pass: FerryPass,
    /// The subnetwork counter: the next label to assign (§2.3); starts at 1.
    pub subnet_counter: u16,
    /// The working subnetwork-info vector (§4.12); index 0 is unused.
    pub subnet_working: Vec<SubnetworkInfo>,
    /// The published subnetwork-info vector (§4.12); all queries read it.
    pub subnet_published: Vec<SubnetworkInfo>,
    /// The scan position of a sliced `UpdateSubnetworkInfo` (§4.12): the next
    /// cell index in x-inner, z-outer order; `None` when no pass is running.
    pub subnet_info_cursor: Option<usize>,
    /// The edge-density map (traffic-simulator.md §4.8): `hash_map<u32 key,
    /// TrafficEdgeDensity>`, hint 100.
    pub edge_density: StlHashMap<TrafficEdgeDensity>,
    /// The neighbour connection edge records (§2.7), one vector per map edge,
    /// in [`super::Edge`] order.
    pub neighbour_edges: [Vec<NeighbourEdge>; 4],
    /// The capacity tables and intersection-effect values (§4.5, §4.6).
    pub tuning: Tuning,
    /// The inverse max-speed table the tunnel cost reads (§4.8 step 5; a
    /// class static the simulator fills from its tuning).
    pub inv_max_speed: InvMaxSpeed,
    /// The intersection dirty rectangle and flag (§4.6).
    pub intersection_dirty: bool,
    pub intersection_rect: CellRect,
    /// The "subnetworks stale" flag (§2.3): set by every rebuild; the work
    /// cycle consumes it (traffic-simulator.md).
    pub subnetworks_stale: bool,
    /// The "network changed" flag (§2.3): set at Init and by every rebuild.
    pub network_changed: bool,
    /// The simulator's "occupants changed" byte (§3): set by every network
    /// occupant message.
    pub occupants_changed: bool,
    /// The water pathfinder (§4.10).
    pub water: WaterPathFinder,
    /// The congestion → inverse-speed curve (`0x00B4B2F0`) the simulator
    /// sets; `TransitSwitchEntryCost` evaluates it (§4.7.3).
    pub congestion_curve: ResponseCurve,
    /// The transit multiplier the simulator sets (§4.7.3); 1.0 at
    /// construction.
    pub transit_multiplier: f32,
}

impl Default for NetworkState {
    /// The constructor values (§2.3).
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
            ferry_pass: FerryPass::default(),
            subnet_counter: 1,
            subnet_working: Vec::new(),
            subnet_published: Vec::new(),
            subnet_info_cursor: None,
            edge_density: StlHashMap::new(),
            neighbour_edges: Default::default(),
            tuning: Tuning::default(),
            inv_max_speed: [[f32::MAX; net_type::COUNT]; travel::COUNT],
            intersection_dirty: false,
            intersection_rect: CellRect::new(0, 0, -1, -1),
            subnetworks_stale: false,
            network_changed: false,
            occupants_changed: false,
            water: WaterPathFinder::default(),
            congestion_curve: ResponseCurve::default(),
            transit_multiplier: 1.0,
        }
    }
}

impl NetworkState {
    /// The number of city cells.
    pub fn cell_count(&self) -> usize {
        (self.count_x.max(0) as usize) * (self.count_z.max(0) as usize)
    }

    /// Whether (x, z) is inside `[0, X−1] × [0, Z−1]`.
    pub fn in_city(&self, x: i32, z: i32) -> bool {
        x >= 0 && z >= 0 && x < self.count_x && z < self.count_z
    }

    /// The whole city as a rectangle.
    pub fn city_rect(&self) -> CellRect {
        CellRect::new(0, 0, self.count_x - 1, self.count_z - 1)
    }

    /// The rectangle grown by `g` and clamped to the city.
    pub fn grow_clamp(&self, r: CellRect, g: i32) -> CellRect {
        grow_clamp(r, g, self.count_x, self.count_z)
    }

    fn info_index(&self, x: i32, z: i32) -> Option<usize> {
        self.in_city(x, z)
            .then(|| x as usize * self.count_z as usize + z as usize)
    }

    /// The cell connection record of (x, z), `None` before Init. The array is
    /// the fixed 256 × 256 indexed by the low 8 bits, so an off-map
    /// coordinate wraps as in the original (§10 Q4).
    pub fn conn_at(&self, x: i32, z: i32) -> Option<&CellConnection> {
        self.conn.get(array_index(x, z))
    }

    /// The mutable cell connection record of (x, z).
    pub fn conn_mut(&mut self, x: i32, z: i32) -> Option<&mut CellConnection> {
        self.conn.get_mut(array_index(x, z))
    }

    /// The wormhole byte of (x, z) (0 before Init).
    pub fn wormhole_at(&self, x: i32, z: i32) -> u8 {
        self.conn_at(x, z).map_or(0, |c| c.wormhole)
    }

    /// The network-info list of an in-city cell (empty off-map).
    pub fn info_at(&self, x: i32, z: i32) -> &[NetworkInfo] {
        match self.info_index(x, z) {
            Some(i) => self.info.get(i).map_or(&[], Vec::as_slice),
            None => &[],
        }
    }

    /// The mutable network-info list of an in-city cell.
    pub fn info_mut(&mut self, x: i32, z: i32) -> Option<&mut Vec<NetworkInfo>> {
        self.info_index(x, z).and_then(|i| self.info.get_mut(i))
    }

    /// `GetNetworkInfo(n, x, z)` (§2.2): the first record of transit network
    /// `n` in the cell's list, or `None`. The original has no bounds check;
    /// FreeCity4 returns `None` off-map [choice].
    pub fn network_info(&self, n: u32, x: i32, z: i32) -> Option<&NetworkInfo> {
        self.info_at(x, z).iter().find(|r| u32::from(r.net) == n)
    }

    /// The mutable first record of transit network `n` in a cell.
    pub fn network_info_mut(&mut self, n: u32, x: i32, z: i32) -> Option<&mut NetworkInfo> {
        self.info_mut(x, z)?.iter_mut().find(|r| u32::from(r.net) == n)
    }

    /// The first car record (`n = 1`) of a cell (§4.6, §4.11, §4.12).
    pub fn car_info(&self, x: i32, z: i32) -> Option<&NetworkInfo> {
        self.network_info(net::CAR, x, z)
    }

    /// Appends a new record of network `n` at the tail of the cell's list,
    /// with the creation quirk of §2.2: appending also clears the `flags`
    /// byte of the record that was the tail before it *[quirk]*.
    pub fn append_info(&mut self, n: u32, x: i32, z: i32) -> Option<&mut NetworkInfo> {
        let list = self.info_mut(x, z)?;
        if let Some(last) = list.last_mut() {
            last.flags = 0;
        }
        list.push(NetworkInfo {
            net: n as u8,
            ..NetworkInfo::default()
        });
        list.last_mut()
    }

    /// Gets or creates the record of network `n` in the cell (§4.2 step 3).
    pub fn get_or_create_info(&mut self, n: u32, x: i32, z: i32) -> Option<&mut NetworkInfo> {
        let pos = self.info_at(x, z).iter().position(|r| u32::from(r.net) == n);
        match pos {
            Some(i) => self.info_mut(x, z).map(|l| &mut l[i]),
            None => self.append_info(n, x, z),
        }
    }

    /// The switch ID at an in-city cell (0 = none); off-map reads 0 (§10 Q4).
    pub fn switch_id_at(&self, x: i32, z: i32) -> u32 {
        match (&self.switch_id, self.in_city(x, z)) {
            (Some(g), true) => g.get_cell_value(x, z),
            _ => 0,
        }
    }

    /// The switch mode mask at an in-city cell (0 off-map).
    pub fn switch_mode_at(&self, x: i32, z: i32) -> u8 {
        match (&self.switch_mode, self.in_city(x, z)) {
            (Some(g), true) => g.get_cell_value(x, z),
            _ => 0,
        }
    }

    /// `SetConnection(x, z, e, x', t)` (§4.3): `conn[t] |= 1 << (x' + 4e)` at
    /// (x, z); and, if `net(t) = 1` and the neighbour through `x'` is in the
    /// city, that neighbour's `priorCar[(x' + 2) mod 4] |= 0x1111 << e`.
    pub fn set_connection(&mut self, x: i32, z: i32, e: usize, exit: usize, t: usize) {
        if t >= travel::COUNT || e >= side::COUNT || exit >= side::COUNT {
            return;
        }
        if let Some(c) = self.conn_mut(x, z) {
            c.conn[t] |= 1 << (exit + 4 * e);
        }
        if NET_OF_TRAVEL[t] == net::CAR {
            let (nx, nz) = (x + DX[exit], z + DZ[exit]);
            if self.in_city(nx, nz) {
                if let Some(c) = self.conn_mut(nx, nz) {
                    c.prior_car[OPPOSITE_SIDE[exit]] |= 0x1111 << e;
                }
            }
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
        self.edge_density
            .get(Self::edge_key(x, z, t, evening))
            .unwrap_or(&EDGE_DENSITY_ZERO)
    }

    /// Creates the edge-density records of a cell and travel type for both
    /// evening values, if missing (§4.2 step 3).
    pub fn ensure_edge_density(&mut self, x: i32, z: i32, t: u32) {
        for evening in [false, true] {
            self.edge_density
                .entry(Self::edge_key(x, z, t, evening), TrafficEdgeDensity::default);
        }
    }

    /// Erases the 18 edge-density keys of a cell (§3, on occupant removal):
    /// both evening values for all 9 travel types.
    pub fn erase_cell_edge_densities(&mut self, x: i32, z: i32) {
        for evening in [false, true] {
            for t in 0..travel::COUNT as u32 {
                self.edge_density.remove(Self::edge_key(x, z, t, evening));
            }
        }
    }

    /// `GetNeighborConnectionInnerCell(x, z)` (§2.7, `0x0070DDD0`): the
    /// in-city cell and the side facing out for an off-map cell next to the
    /// map; `None` (the original's −1) unless exactly one of x and z is
    /// off-map.
    pub fn neighbor_connection_inner_cell(&self, x: i32, z: i32) -> Option<((i32, i32), usize)> {
        let x_off = x < 0 || x > self.count_x - 1;
        let z_off = z < 0 || z > self.count_z - 1;
        if x_off == z_off {
            return None;
        }
        Some(if x < 0 {
            ((0, z), side::WEST)
        } else if x > self.count_x - 1 {
            ((self.count_x - 1, z), side::EAST)
        } else if z < 0 {
            ((x, 0), side::NORTH)
        } else {
            ((x, self.count_z - 1), side::SOUTH)
        })
    }

    /// `HasMapExit(x, z)` (§2.7): for an off-map cell next to the map, true
    /// if any travel type's connection word at the in-city cell has an exit
    /// through the outward side (for any entry side); otherwise true if the
    /// edge record at the off-map cell has mask bit 9 set.
    pub fn has_map_exit(&self, x: i32, z: i32) -> bool {
        let Some(((ix, iz), d)) = self.neighbor_connection_inner_cell(x, z) else {
            return false;
        };
        if let Some(c) = self.conn_at(ix, iz) {
            if (0..travel::COUNT).any(|t| c.has_exit_through(t, d)) {
                return true;
            }
        }
        self.neighbour_edge_at(x, z).is_some_and(|e| e.mask & (1 << 9) != 0)
    }

    /// `TransitSwitchEntryCost(id)` (§4.7.3, x87 return).
    ///
    /// 1. Look the switch up with insert-if-missing semantics *[quirk]*.
    /// 2. `q = r24(load / capacity)`.
    /// 3. `q < 5.0` (ordered): `r24(r24(Curve(q) × transitMultiplier) ×
    ///    timeFactor)`.
    /// 4. Otherwise (q ≥ 5, NaN from 0/0, or +∞): `0x7EFFFFFF`.
    pub fn transit_switch_entry_cost(&mut self, id: u32) -> f32 {
        let (load, capacity, time_factor) = {
            let sw = self.switches.entry(id, TransitSwitch::default);
            (sw.load, sw.capacity, sw.time_factor)
        };
        let q = x87::r24(f64::from(load) / f64::from(capacity)) as f32;
        if q < 5.0 {
            let curve = self.congestion_curve.eval(q);
            let a = x87::r24(f64::from(curve) * f64::from(self.transit_multiplier)) as f32;
            x87::r24(f64::from(a) * f64::from(time_factor)) as f32
        } else {
            super::SWITCH_COST_BLOCKED
        }
    }

    /// `UpdateIntersectionData` (§4.6): runs over the dirty rectangle grown by
    /// N and clamped, then clears the flag. A no-op while the flag is clear.
    pub fn update_intersection_data(&mut self) {
        if !self.intersection_dirty {
            return;
        }
        let n = i32::from(self.tuning.intersection_count);
        let rect = self.grow_clamp(self.intersection_rect, n);
        for x in rect.x1..=rect.x2 {
            for z in rect.z1..=rect.z2 {
                self.reduce_cell_intersection(x, z, n);
            }
        }
        self.intersection_dirty = false;
    }

    /// §4.6 for one cell: a car record whose type is not Highway gets
    /// `invCap = invCapTable[type]`, times `values[best]` when `best < N`.
    fn reduce_cell_intersection(&mut self, x: i32, z: i32, n: i32) {
        let Some(info) = self.car_info(x, z).copied() else {
            return;
        };
        if u32::from(info.net_type) == net_type::HIGHWAY {
            return; // a Highway record is never reduced
        }
        let best = self.nearest_intersection(x, z, info.conn_count, n);
        let Some(&base) = self.tuning.inv_cap_table.get(usize::from(info.net_type)) else {
            return;
        };
        let inv = if best < n {
            let v = self.tuning.intersection_values[best as usize];
            x87::r24(f64::from(base) * f64::from(v)) as f32
        } else {
            base
        };
        if let Some(r) = self.network_info_mut(net::CAR, x, z) {
            r.inv_cap = inv;
        }
    }

    /// The distance to the nearest intersection (§4.6 step 1): 0 when the
    /// cell's own `connCount > 2`; otherwise a search along the car
    /// (`conn[1]`) exits, union over entry sides, that stops extending at
    /// depth `best` (which starts at N + 1).
    ///
    /// A reached cell is an intersection if its car record has
    /// `connCount > 2`, or if it has no car record but a transit switch whose
    /// "counts as intersection" flag is set. The search does not step through
    /// tunnel cells or cells without car info: such cells are reached (and
    /// may be intersections) but not extended *[inferred: "do not step
    /// through"]*. The bounds check compares both coordinates with X−1
    /// *[quirk]*.
    ///
    /// The original searches depth-first; the smallest depth at which an
    /// intersection is reached is the same for every visiting order, so this
    /// walks breadth-first, visiting each cell once at its smallest depth.
    fn nearest_intersection(&self, x: i32, z: i32, conn_count: u8, n: i32) -> i32 {
        if conn_count > 2 {
            return 0;
        }
        let mut best = n + 1;
        let max = self.count_x - 1; // the quirk: both coordinates against X−1
        let mut seen = std::collections::HashSet::new();
        seen.insert((x, z));
        let mut frontier = vec![(x, z)];
        let mut depth = 0;
        while !frontier.is_empty() && depth < best {
            let mut next = Vec::new();
            for &(cx, cz) in &frontier {
                let exits = self.conn_at(cx, cz).map_or(0, |c| c.exit_union(travel::CAR));
                for d in 0..side::COUNT {
                    if exits & (1 << d) == 0 {
                        continue;
                    }
                    let (nx, nz) = (cx + DX[d], cz + DZ[d]);
                    if nx < 0 || nz < 0 || nx > max || nz > max {
                        continue;
                    }
                    if self.is_intersection(nx, nz) {
                        best = best.min(depth + 1);
                        continue;
                    }
                    let tunnel = self.wormhole_at(nx, nz) & wormhole::TUNNEL != 0;
                    if tunnel || self.car_info(nx, nz).is_none() {
                        continue;
                    }
                    if seen.insert((nx, nz)) {
                        next.push((nx, nz));
                    }
                }
            }
            frontier = next;
            depth += 1;
        }
        best
    }

    /// Whether a reached cell is an intersection (§4.6 step 1).
    fn is_intersection(&self, x: i32, z: i32) -> bool {
        if let Some(info) = self.car_info(x, z) {
            return info.conn_count > 2;
        }
        let id = self.switch_id_at(x, z);
        id != 0 && self.switches.get(id).is_some_and(|sw| sw.counts_as_intersection)
    }

    /// The distinct non-zero `subnet[0…3]` of the first car record of the
    /// given cells, collected into an SGI `hash_set<u32>` (hint 100, identity
    /// hash) and returned in hash order (§4.13 slots 40, 41).
    fn subnetworks_of_cells(&self, cells: impl Iterator<Item = (i32, i32)>) -> Vec<u32> {
        let mut set = StlHashSet::with_hint(STL_DEFAULT_HINT);
        for (x, z) in cells {
            if let Some(r) = self.car_info(x, z) {
                for &id in &r.subnet {
                    if id != 0 {
                        set.insert(u32::from(id));
                    }
                }
            }
        }
        set.iter_order()
    }
}

/// The context of an operation that calls the host: the host and the state
/// taken out of the model (see the module documentation on re-entrancy).
pub(crate) struct Cx<'a> {
    pub(crate) h: &'a dyn NetworkHost,
    pub(crate) s: &'a mut NetworkState,
}

/// The traffic network model. The game keeps this inside
/// `cSC4TrafficSimulator`; FreeCity4 keeps it as a plain object the future
/// traffic simulator owns (§9).
pub struct NetworkModel {
    pub(super) host: Rc<dyn NetworkHost>,
    pub(super) s: RefCell<NetworkState>,
}

impl NetworkModel {
    /// A new model with the constructor values (§1, §2.3); `init` sizes it.
    pub fn new(host: Rc<dyn NetworkHost>) -> Rc<NetworkModel> {
        Rc::new(NetworkModel {
            host,
            s: RefCell::new(NetworkState::default()),
        })
    }

    /// The host.
    pub fn host(&self) -> &Rc<dyn NetworkHost> {
        &self.host
    }

    /// The state, borrowed.
    pub fn state(&self) -> Ref<'_, NetworkState> {
        self.s.borrow()
    }

    /// A copy of the state.
    pub fn snapshot(&self) -> NetworkState {
        self.s.borrow().clone()
    }

    /// Changes the state directly (hosts restoring state, tests).
    pub fn update_state<R>(&self, f: impl FnOnce(&mut NetworkState) -> R) -> R {
        f(&mut self.s.borrow_mut())
    }

    /// Runs `f` with the state taken out of the cell, so `f` may call the
    /// host without a borrow held (a re-entrant call sees an empty state).
    pub(crate) fn run<R>(&self, f: impl FnOnce(&mut Cx<'_>) -> R) -> R {
        let mut st = std::mem::take(&mut *self.s.borrow_mut());
        let r = {
            let mut cx = Cx {
                h: &*self.host,
                s: &mut st,
            };
            f(&mut cx)
        };
        *self.s.borrow_mut() = st;
        r
    }

    // ---- simulator-owned inputs ----

    /// Replaces the capacity tuning (§4.5; the simulator's
    /// `ReadSimulatorTunableValues` reads it).
    pub fn set_tuning(&self, tuning: Tuning) {
        self.s.borrow_mut().tuning = tuning;
    }

    /// Sets the inverse max-speed table the tunnel cost reads (§4.8 step 5).
    pub fn set_inv_max_speed(&self, inv: InvMaxSpeed) {
        self.s.borrow_mut().inv_max_speed = inv;
    }

    /// Sets the congestion curve and transit multiplier the simulator owns
    /// (§4.7.3, traffic-simulator.md §4.14).
    pub fn set_switch_cost_inputs(&self, curve: ResponseCurve, multiplier: f32) {
        let mut s = self.s.borrow_mut();
        s.congestion_curve = curve;
        s.transit_multiplier = multiplier;
    }

    // ---- Init (§1) ----

    /// The network part of the traffic simulator's `Init` (§1): the city
    /// size, the network-info grid, the cell connection array, the switch
    /// grids, the water pathfinder. The next switch ID is the constructor's
    /// (1); Init does not reset it.
    pub fn init(&self) {
        self.run(|cx| {
            let (x, z) = cx.h.cell_counts();
            let s = &mut *cx.s;
            // Step 1: the city size.
            s.count_x = x.clamp(0, super::MAX_CITY_SIDE);
            s.count_z = z.clamp(0, super::MAX_CITY_SIDE);
            // Step 2: X × Z empty network-info lists.
            s.info = vec![Vec::new(); s.cell_count()];
            // Step 4: every record of the cell connection array reset.
            s.conn = vec![CellConnection::default(); ARRAY_LEN];
            // Step 5: the switch grids, created through the host as the
            // game's grid classes (simgrid §6.4), tract size 1.
            s.switch_mode = super::GridProvider::<u8>::create_grid(cx.h, super::SWITCH_MODE_GRID_ID, 1);
            s.switch_id = super::GridProvider::<u32>::create_grid(cx.h, super::SWITCH_ID_GRID_ID, 1);
            // Step 6: the water pathfinder.
            s.water.init(s.count_x, s.count_z);
            // "network changed" is set at Init (§2.3).
            s.network_changed = true;
        });
    }

    // ---- capacity queries (§4.5) ----

    /// `GetCapacity(n, x, z)` (slot 23): `r24(1.0 / invCap)` of the first
    /// record of network `n` at (x, z), 0.0 when there is none.
    pub fn get_capacity(&self, n: u32, x: i32, z: i32) -> f32 {
        match self.s.borrow().network_info(n, x, z) {
            Some(r) => x87::r24(1.0 / f64::from(r.inv_cap)) as f32,
            None => 0.0,
        }
    }

    /// `GetConnectionCount(n, x, z)` (slot 25): the record's `connCount`, or
    /// 0, zero-extended.
    pub fn get_connection_count(&self, n: u32, x: i32, z: i32) -> u32 {
        self.s
            .borrow()
            .network_info(n, x, z)
            .map_or(0, |r| u32::from(r.conn_count))
    }

    /// `GetNetworkInfo(n, x, z)` (§2.2), copied.
    pub fn network_info(&self, n: u32, x: i32, z: i32) -> Option<NetworkInfo> {
        self.s.borrow().network_info(n, x, z).copied()
    }

    /// The cell connection record of (x, z), copied.
    pub fn cell_connection(&self, x: i32, z: i32) -> Option<CellConnection> {
        self.s.borrow().conn_at(x, z).copied()
    }

    // ---- connectivity queries (§4.13) ----

    /// The lot's subnetwork list (property `0x2A4F9C74`), `None` when the
    /// property is absent. Every element of a scalar or an array is read as a
    /// u32.
    pub fn lot_subnetworks(&self, lot: u64) -> Option<Vec<u32>> {
        let holder = self.host.lot_holder(lot);
        if holder == 0 {
            return None;
        }
        self.host
            .holder_property(holder, prop::LOT_SUBNETWORKS)
            .map(|v| subnetwork_ids(&v))
    }

    /// `AreLotsConnected(A, B)` (slot 36): true when both lots have a
    /// non-empty list and the lists share an ID.
    pub fn are_lots_connected(&self, a: u64, b: u64) -> bool {
        let la = self.lot_subnetworks(a).unwrap_or_default();
        let lb = self.lot_subnetworks(b).unwrap_or_default();
        !la.is_empty() && !lb.is_empty() && la.iter().any(|id| lb.contains(id))
    }

    /// `GetConnectedOccupantCount(lot, developerType)` (slot 37): the maximum
    /// over the lot's IDs (with `id < size`) of
    /// `published[id].population[type]` (0 when absent); 0 without a list.
    pub fn get_connected_occupant_count(&self, lot: u64, developer_type: u32) -> u32 {
        let ids = self.lot_subnetworks(lot).unwrap_or_default();
        let s = self.s.borrow();
        ids.iter()
            .filter_map(|&id| s.subnet_published.get(id as usize))
            .map(|info| info.population.get(&developer_type).copied().unwrap_or(0))
            .max()
            .unwrap_or(0)
    }

    /// `GetConnectedDestinationCount(lot, k)` (slot 38): `k > 3` (signed) →
    /// 0; otherwise the maximum over the lot's IDs (with `id < size`) of
    /// `published[id].capacity[k]`; 0 without a list. A negative `k` also
    /// gives 0 [choice] (the original indexes before the array).
    pub fn get_connected_destination_count(&self, lot: u64, k: i32) -> u32 {
        if !(0..=3).contains(&k) {
            return 0;
        }
        let ids = self.lot_subnetworks(lot).unwrap_or_default();
        let s = self.s.borrow();
        ids.iter()
            .filter_map(|&id| s.subnet_published.get(id as usize))
            .map(|info| info.capacity[k as usize])
            .max()
            .unwrap_or(0)
    }

    /// `GetSubnetworksForLot(lot, vector<u32>&)` (slot 39): the vector is
    /// cleared, then the property is read into it; the bool is whether the
    /// property exists.
    pub fn get_subnetworks_for_lot(&self, lot: u64) -> (Vec<u32>, bool) {
        match self.lot_subnetworks(lot) {
            Some(v) => (v, true),
            None => (Vec::new(), false),
        }
    }

    /// `GetSubnetworksInRectangle(rect, vector<u32>&)` (slot 40): the
    /// rectangle is inclusive; the distinct non-zero `subnet[0…3]` of the
    /// first car record of every in-city cell, in hash order (cells x outer,
    /// z inner).
    pub fn get_subnetworks_in_rectangle(&self, rect: CellRect) -> Vec<u32> {
        let s = self.s.borrow();
        let r = s.grow_clamp(rect, 0);
        let cells = (r.x1..=r.x2).flat_map(|x| (r.z1..=r.z2).map(move |z| (x, z)));
        s.subnetworks_of_cells(cells)
    }

    /// `GetSubnetworksInRegion(region, vector<u32>&)` (slot 41): as slot 40,
    /// over the in-city cells of the region's bounding rectangle whose bit is
    /// set.
    pub fn get_subnetworks_in_region(&self, region: &CellRegion) -> Vec<u32> {
        let s = self.s.borrow();
        let r = s.grow_clamp(region.rect, 0);
        let cells = (r.x1..=r.x2)
            .flat_map(|x| (r.z1..=r.z2).map(move |z| (x, z)))
            .filter(|&(x, z)| region.includes(x, z));
        s.subnetworks_of_cells(cells)
    }

    /// `GetOccupantCountForAllSubnetworks(developerType, vector<u32>&)`
    /// (slot 42): `v.assign(size, 0)`, then
    /// `v[id] = published[id].population[type]` (if present) for
    /// id = 1 … size−1.
    pub fn get_occupant_count_for_all_subnetworks(&self, developer_type: u32) -> Vec<u32> {
        let s = self.s.borrow();
        let mut v = vec![0u32; s.subnet_published.len()];
        for (id, info) in s.subnet_published.iter().enumerate().skip(1) {
            if let Some(&c) = info.population.get(&developer_type) {
                v[id] = c;
            }
        }
        v
    }

    // ---- pathfinder-facing helpers ----

    /// `TransitSwitchEntryCost(id)` (§4.7.3).
    pub fn transit_switch_entry_cost(&self, id: u32) -> f32 {
        self.s.borrow_mut().transit_switch_entry_cost(id)
    }

    /// `GetNeighborConnectionInnerCell(x, z)` (§2.7).
    pub fn neighbor_connection_inner_cell(&self, x: i32, z: i32) -> Option<((i32, i32), usize)> {
        self.s.borrow().neighbor_connection_inner_cell(x, z)
    }

    /// `HasMapExit(x, z)` (§2.7).
    pub fn has_map_exit(&self, x: i32, z: i32) -> bool {
        self.s.borrow().has_map_exit(x, z)
    }

    /// `UpdateIntersectionData` (§4.6), run by the simulator at the start of
    /// its next work cycle while the dirty flag is set.
    pub fn update_intersection_data(&self) {
        self.s.borrow_mut().update_intersection_data();
    }
}

/// The u32 IDs of a subnetwork-list property value.
fn subnetwork_ids(v: &Variant) -> Vec<u32> {
    v.items().iter().map(|e| e.as_i64() as u32).collect()
}

/// Grows a rectangle by `g` on every side and clamps it to
/// `[0, X−1] × [0, Z−1]`.
pub fn grow_clamp(r: CellRect, g: i32, count_x: i32, count_z: i32) -> CellRect {
    CellRect {
        x1: r.x1.saturating_sub(g).max(0),
        z1: r.z1.saturating_sub(g).max(0),
        x2: r.x2.saturating_add(g).min(count_x - 1),
        z2: r.z2.saturating_add(g).min(count_z - 1),
    }
}

/// The union of two rectangles (the dirty rectangles of §4.6 and §4.10.1);
/// an empty (inverted) rectangle adds nothing.
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

/// The cells of a rectangle's boundary in `BoundaryIterator` order
/// (`0x0060CEE0`; the order is verified in zones.md §7 slot 10 and stated in
/// pathfinder.md §4.6): the first row (`z = z1`) from x1 to x2, each middle
/// row only at x1 and then x2, the last row (`z = z2`) from x1 to x2. A
/// one-row rectangle is visited once; a one-column rectangle visits each
/// middle row once.
pub fn boundary_cells(r: CellRect) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    if r.x1 > r.x2 || r.z1 > r.z2 {
        return out;
    }
    for z in r.z1..=r.z2 {
        if z == r.z1 || z == r.z2 {
            out.extend((r.x1..=r.x2).map(|x| (x, z)));
        } else {
            out.push((r.x1, z));
            if r.x2 != r.x1 {
                out.push((r.x2, z));
            }
        }
    }
    out
}

/// The cell key of a cell as a hash-map key.
pub(super) fn key32(x: i32, z: i32) -> u32 {
    u32::from(cell_key(x, z))
}
