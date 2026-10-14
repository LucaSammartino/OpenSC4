// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/pathfinder.md`: a scripted traffic
//! simulator and city implementing `PathfinderHost` (travel tables, volumes,
//! the destination bitmask map and free counts, lots with trip capacities,
//! neighbour pools and connections, the income accumulator, the carpool
//! ordinance) over a `NetworkState` set up cell by cell, and ops that drive
//! `fc4_core::pathfinder::Pathfinder`. See README.md ("Pathfinder ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};

use fc4_core::curve::ResponseCurve;
use fc4_core::grid::{CellRect, CityDims, CoreGrids, GridProvider};
use fc4_core::pathfinder::{
    distance_to_rect, encode_route, heap_order, node_key, rect_contains, shared_work_check, start_ring_cells,
    NodeStore, PathCx, Pathfinder, PathfinderHost, RouteNode, SharedWorkCheck, TravelTables, WorkCheck,
    CARPOOL_ORDINANCE,
};
use fc4_core::traffic_network::{
    boundary_cells, cell_key, grow_clamp, net_type, travel, wormhole, CellConnection, FerryTerminal, NeighborLink,
    NeighbourEdge, NetworkState, TerminalLink, TransitSwitch, TunnelInfo, ARRAY_LEN, SWITCH_ID_GRID_ID,
};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::HostRef;
use crate::scalar::{self, arg, arr, bool_of, check_expect, f32_of, i64_of, ji, matches, opt, show, str_of, u32_of, R};

// ---- the scripted world ---------------------------------------------------------------------------

/// The trip capacities of a destination holder: (max[4], used[4]).
type Capacity = ([u32; 4], [u32; 4]);

/// A lot: (token, bounding rect, property holder).
type Lot = (u64, CellRect, u64);

/// The §11 preamble's city size.
const PREAMBLE_CELLS: (i32, i32) = (8, 8);

struct World {
    cells: Cell<(i32, i32)>,
    tables: Cell<TravelTables>,
    max_distance: Cell<u32>,
    attractiveness: Cell<f32>,
    strike: Cell<u8>,
    trip_scale: Cell<f32>,
    carpool: Cell<bool>,
    /// (network, x, z) → volume.
    volumes: RefCell<BTreeMap<(u32, i32, i32), f32>>,
    /// The destination bitmask map.
    dest: RefCell<BTreeMap<(i32, i32), u8>>,
    free: Cell<[u32; 4]>,
    caps: RefCell<BTreeMap<u64, Capacity>>,
    lots: RefCell<Vec<Lot>>,
    pools: RefCell<BTreeMap<(i32, i32), u64>>,
    income: Cell<f64>,
    deadline_reached: Cell<bool>,
    net: RefCell<NetworkState>,
    shared: SharedWorkCheck,
    pfs: RefCell<BTreeMap<u32, Pathfinder>>,
}

/// The §11 preamble's travel tables: `affectedByTraffic[Car] =
/// canReachDestination[Car] = generatesTraffic[Car] = true`, everything else
/// false or 0.
fn preamble_tables() -> TravelTables {
    let mut t = TravelTables::default();
    t.affected_by_traffic[travel::CAR] = true;
    t.can_reach_destination[travel::CAR] = true;
    t.generates_traffic[travel::CAR] = true;
    t
}

/// An empty network of `w × h` cells (every cell record at the Init
/// values, no network info, no switches), with the preamble's
/// `invMaxSpeed[Car][Street] = 1.5` and `Curve(0.0) = 1.0`.
fn empty_net(w: i32, h: i32) -> NetworkState {
    let mut s = NetworkState {
        count_x: w,
        count_z: h,
        conn: vec![CellConnection::default(); ARRAY_LEN],
        info: vec![Vec::new(); (w.max(0) * h.max(0)) as usize],
        congestion_curve: ResponseCurve::from_points(&[(0.0, 1.0)]),
        ..NetworkState::default()
    };
    s.inv_max_speed[travel::CAR][net_type::STREET as usize] = 1.5;
    s
}

impl World {
    fn new() -> World {
        let (w, h) = PREAMBLE_CELLS;
        World {
            cells: Cell::new((w, h)),
            tables: Cell::new(preamble_tables()),
            max_distance: Cell::new(1),
            attractiveness: Cell::new(1.0),
            strike: Cell::new(0),
            trip_scale: Cell::new(1.0),
            carpool: Cell::new(false),
            volumes: RefCell::new(BTreeMap::new()),
            dest: RefCell::new(BTreeMap::new()),
            free: Cell::new([0; 4]),
            caps: RefCell::new(BTreeMap::new()),
            lots: RefCell::new(Vec::new()),
            pools: RefCell::new(BTreeMap::new()),
            income: Cell::new(0.0),
            deadline_reached: Cell::new(false),
            net: RefCell::new(empty_net(w, h)),
            shared: shared_work_check(),
            pfs: RefCell::new(BTreeMap::new()),
        }
    }

    fn lot(&self, token: u64) -> Option<Lot> {
        self.lots.borrow().iter().find(|l| l.0 == token).copied()
    }
}

impl PathfinderHost for World {
    fn cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn work_deadline(&self, _ms: u32) -> u64 {
        0
    }
    fn work_deadline_reached(&self, _deadline: u64) -> bool {
        self.deadline_reached.get()
    }
    fn travel_tables(&self) -> TravelTables {
        self.tables.get()
    }
    fn max_distance_origin_to_network(&self) -> u32 {
        self.max_distance.get()
    }
    fn nearest_destination_attractiveness(&self) -> f32 {
        self.attractiveness.get()
    }
    fn strike_state(&self) -> u8 {
        self.strike.get()
    }
    fn trip_scale_for_display(&self) -> f32 {
        self.trip_scale.get()
    }
    fn volume(&self, n: u32, x: i32, z: i32) -> f32 {
        self.volumes.borrow().get(&(n, x, z)).copied().unwrap_or(0.0)
    }
    fn set_volume(&self, n: u32, x: i32, z: i32, v: f32) {
        self.volumes.borrow_mut().insert((n, x, z), v);
    }
    fn destination_byte(&self, x: i32, z: i32) -> u8 {
        self.dest.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn set_destination_byte(&self, x: i32, z: i32, bits: u8) {
        self.dest.borrow_mut().insert((x, z), bits);
    }
    fn free_destinations(&self, k: u32) -> u32 {
        self.free.get().get(k as usize).copied().unwrap_or(0)
    }
    fn set_free_destinations(&self, k: u32, n: u32) {
        let mut f = self.free.get();
        if let Some(slot) = f.get_mut(k as usize) {
            *slot = n;
        }
        self.free.set(f);
    }
    fn max_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        self.caps.borrow().get(&holder).map_or(0, |c| c.0[k as usize & 3])
    }
    fn used_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        self.caps.borrow().get(&holder).map_or(0, |c| c.1[k as usize & 3])
    }
    fn neighbor_pool_holder(&self, _t: u32, x: i32, z: i32) -> Option<u64> {
        self.pools.borrow().get(&(x, z)).copied()
    }
    fn income_accumulator(&self) -> f64 {
        self.income.get()
    }
    fn set_income_accumulator(&self, v: f64) {
        self.income.set(v);
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        self.lots
            .borrow()
            .iter()
            .find(|l| rect_contains(l.1, x, z))
            .map(|l| l.0)
    }
    fn lot_holder(&self, lot: u64) -> u64 {
        self.lot(lot).map_or(0, |l| l.2)
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.lot(lot).map_or(CellRect::new(0, 0, -1, -1), |l| l.1)
    }
    fn occupants_in_cells(&self, _rect: CellRect) -> Vec<u64> {
        Vec::new()
    }
    fn occupant_filter_includes(&self, _filter: u64, _occupant: u64) -> bool {
        false
    }
    fn is_ordinance_on(&self, id: u32) -> bool {
        id == CARPOOL_ORDINANCE && self.carpool.get()
    }
}

// ---- dispatch ---------------------------------------------------------------------------------------

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("pf_")?;
    let w = scalar::world(host, "pathfinder", || std::rc::Rc::new(World::new()));
    Some(match rest {
        "env" => env(&w, op),
        "cell" => cell(&w, op),
        "street" => street(&w, op),
        "lot" => lot(&w, op),
        "capacity" => capacity(&w, op),
        "dest" => dest(&w, op),
        "volume" => volume(&w, op),
        "neighbor_cost" => neighbor_cost(&w, op),
        "pool" => pool(&w, op),
        "ferry" => ferry(&w, op),
        "tunnel" => tunnel(&w, op),
        "switch" => switch(&w, op),
        "create" => create(&w, op, true),
        "new" => create(&w, op, false),
        "call" => call(&w, op),
        "eval" => eval(&w, op),
        "work_check" => work_check(&w, op),
        "expect" => expect(&w, op),
        "expect_world" => expect_world(&w, op),
        "expect_edge" => expect_edge(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- parsing helpers ----------------------------------------------------------------------------------

fn i32_of(j: &Json, what: &str) -> R<i32> {
    Ok(i64_of(j, what)? as i32)
}

fn pair(j: &Json, what: &str) -> R<(i32, i32)> {
    match arr(j, what)? {
        [x, z] => Ok((i32_of(x, what)?, i32_of(z, what)?)),
        _ => Err(format!("'{what}' must be [x, z]")),
    }
}

fn rect_of(j: &Json, what: &str) -> R<CellRect> {
    match arr(j, what)? {
        [a, b, c, d] => Ok(CellRect::new(
            i32_of(a, what)?,
            i32_of(b, what)?,
            i32_of(c, what)?,
            i32_of(d, what)?,
        )),
        _ => Err(format!("'{what}' must be [x1, z1, x2, z2]")),
    }
}

/// A float: a number, or its exact bit pattern as a `"0x…"` string.
fn fv(j: &Json, what: &str) -> R<f32> {
    match j.as_str() {
        Some(s) if s.starts_with("0x") || s.starts_with("0X") => u32::from_str_radix(&s[2..], 16)
            .map(f32::from_bits)
            .map_err(|_| format!("'{what}' must be a float or \"0x…\" bits")),
        _ => f32_of(j, what),
    }
}

/// The cells an op names: `cell`, `cells` or `rect` (every cell of it).
fn cells_of(op: &Json) -> R<Vec<(i32, i32)>> {
    if let Some(c) = opt(op, "cells") {
        return arr(c, "cells")?.iter().map(|c| pair(c, "cells")).collect();
    }
    if let Some(r) = opt(op, "rect") {
        let r = rect_of(r, "rect")?;
        let mut out = Vec::new();
        for z in r.z1..=r.z2 {
            for x in r.x1..=r.x2 {
                out.push((x, z));
            }
        }
        return Ok(out);
    }
    Ok(vec![pair(
        opt(op, "cell").ok_or("missing 'cell', 'cells' or 'rect'")?,
        "cell",
    )?])
}

fn field<'a>(op: &'a Json, key: &str) -> R<&'a Json> {
    opt(op, key).ok_or_else(|| format!("missing field '{key}'"))
}

fn iarg(op: &Json, i: usize) -> R<i32> {
    i32_of(arg(op, i)?, "argument")
}

fn uarg(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}

fn farg(op: &Json, i: usize) -> R<f32> {
    fv(arg(op, i)?, "argument")
}

fn rect_args(op: &Json) -> R<CellRect> {
    Ok(CellRect::new(iarg(op, 0)?, iarg(op, 1)?, iarg(op, 2)?, iarg(op, 3)?))
}

fn u4(j: &Json, what: &str) -> R<[u32; 4]> {
    let v: Vec<u32> = arr(j, what)?.iter().map(|v| u32_of(v, what)).collect::<R<_>>()?;
    v.try_into().map_err(|_| format!("'{what}' must have 4 values"))
}

fn travel_type(j: &Json) -> R<usize> {
    let t = u32_of(j, "travel type")? as usize;
    if t >= travel::COUNT {
        return Err(format!("travel type {t} out of range"));
    }
    Ok(t)
}

/// A float as its exact bit pattern, `"0x3C23D70A"` (upper case).
fn fb(v: f32) -> Json {
    Json::Str(format!("0x{:08X}", v.to_bits()))
}

/// A 32-bit key as `"0x10010203"` (upper case).
fn hx(v: u32) -> Json {
    Json::Str(format!("0x{v:08X}"))
}

fn obj(fields: Vec<(&str, Json)>) -> Json {
    Json::Object(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn jcell((x, z): (i32, i32)) -> Json {
    Json::Array(vec![ji(x), ji(z)])
}

fn jrect(r: CellRect) -> Json {
    Json::Array(vec![ji(r.x1), ji(r.z1), ji(r.x2), ji(r.z2)])
}

fn jlist<T: Copy + Into<i64>>(v: &[T]) -> Json {
    Json::Array(v.iter().map(|&x| ji(x)).collect())
}

/// Compares every key of `want` with the same key of `got` (partial
/// objects).
fn check_fields(got: &Json, want: &Json, what: &str) -> R<()> {
    let want = want.as_object().ok_or_else(|| format!("'{what}' must be an object"))?;
    let mut errors = Vec::new();
    for (k, v) in want {
        match got.get(k) {
            None => errors.push(format!("unknown {what} field '{k}'")),
            Some(g) => {
                if !matches(g, v, true) {
                    errors.push(format!("{k}: got {}, expected {}", show(g), show(v)));
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

// ---- environment ------------------------------------------------------------------------------------

fn type_set(j: &Json, what: &str) -> R<[bool; travel::COUNT]> {
    let mut out = [false; travel::COUNT];
    for t in arr(j, what)? {
        out[travel_type(t)?] = true;
    }
    Ok(out)
}

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(c) = opt(op, "cells") {
        let (cx, cz) = pair(c, "cells")?;
        w.cells.set((cx, cz));
        *w.net.borrow_mut() = empty_net(cx, cz);
    }
    let mut t = w.tables.get();
    if let Some(v) = opt(op, "can_reach") {
        t.can_reach_destination = type_set(v, "can_reach")?;
    }
    if let Some(v) = opt(op, "generates") {
        t.generates_traffic = type_set(v, "generates")?;
    }
    if let Some(v) = opt(op, "affected") {
        t.affected_by_traffic = type_set(v, "affected")?;
    }
    if let Some(v) = opt(op, "income_per_tile") {
        for e in arr(v, "income_per_tile")? {
            match arr(e, "income_per_tile")? {
                [ty, x] => t.income_per_tile[travel_type(ty)?] = fv(x, "income")?,
                _ => return Err("each income_per_tile entry is [t, v]".into()),
            }
        }
    }
    if let Some(v) = opt(op, "default_start_cost") {
        let costs = arr(v, "default_start_cost")?;
        if costs.len() != travel::COUNT {
            return Err("'default_start_cost' must have 9 values".into());
        }
        for (slot, c) in t.default_start_cost.iter_mut().zip(costs) {
            *slot = fv(c, "default_start_cost")?;
        }
    }
    w.tables.set(t);
    {
        let mut net = w.net.borrow_mut();
        if let Some(v) = opt(op, "inv_max_speed") {
            for e in arr(v, "inv_max_speed")? {
                match arr(e, "inv_max_speed")? {
                    [ty, nt, x] => {
                        let nt = u32_of(nt, "network type")? as usize;
                        if nt >= net_type::COUNT {
                            return Err("network type out of range".into());
                        }
                        net.inv_max_speed[travel_type(ty)?][nt] = fv(x, "speed")?;
                    }
                    _ => return Err("each inv_max_speed entry is [t, type, v]".into()),
                }
            }
        }
        if let Some(v) = opt(op, "curve") {
            let mut points = Vec::new();
            for p in arr(v, "curve")? {
                match arr(p, "curve")? {
                    [x, y] => points.push((fv(x, "x")?, fv(y, "y")?)),
                    _ => return Err("each curve point is [x, y]".into()),
                }
            }
            net.congestion_curve = ResponseCurve::from_points(&points);
        }
        if let Some(v) = opt(op, "transit_multiplier") {
            net.transit_multiplier = fv(v, "transit_multiplier")?;
        }
    }
    if let Some(v) = opt(op, "attractiveness") {
        w.attractiveness.set(fv(v, "attractiveness")?);
    }
    if let Some(v) = opt(op, "max_distance") {
        w.max_distance.set(u32_of(v, "max_distance")?);
    }
    if let Some(v) = opt(op, "strike") {
        w.strike.set(u32_of(v, "strike")? as u8);
    }
    if let Some(v) = opt(op, "trip_scale") {
        w.trip_scale.set(fv(v, "trip_scale")?);
    }
    if let Some(v) = opt(op, "carpool") {
        w.carpool.set(bool_of(v, "carpool")?);
    }
    if let Some(v) = opt(op, "free") {
        w.free.set(u4(v, "free")?);
    }
    if let Some(v) = opt(op, "income") {
        w.income.set(scalar::num(v).ok_or("'income' must be a number")?);
    }
    if let Some(v) = opt(op, "deadline_reached") {
        w.deadline_reached.set(bool_of(v, "deadline_reached")?);
    }
    Ok(())
}

/// Pokes cell connection records and appends network-info records.
fn cell(w: &World, op: &Json) -> R<()> {
    let cells = cells_of(op)?;
    let mut net = w.net.borrow_mut();
    for (x, z) in cells {
        {
            let c = net
                .conn_mut(x, z)
                .ok_or_else(|| format!("({x},{z}) is outside the city"))?;
            for (key, words) in [("conn", &mut c.conn), ("switch", &mut c.switch)] {
                if let Some(v) = opt(op, key) {
                    for e in arr(v, key)? {
                        match arr(e, key)? {
                            [t, word] => words[travel_type(t)?] = u32_of(word, key)? as u16,
                            _ => return Err(format!("each '{key}' entry is [t, word]")),
                        }
                    }
                }
            }
            if let Some(v) = opt(op, "prior_car") {
                c.prior_car = u4(v, "prior_car")?.map(|p| p as u16);
            }
            if let Some(v) = opt(op, "wormhole") {
                c.wormhole = u32_of(v, "wormhole")? as u8;
            }
        }
        if let Some(v) = opt(op, "info") {
            for e in arr(v, "info")? {
                match arr(e, "info")? {
                    [n, ty, cap, extra] => {
                        let n = u32_of(n, "network")?;
                        let (ty, cap, extra) = (u32_of(ty, "type")?, fv(cap, "inv_cap")?, fv(extra, "extra_cost")?);
                        let r = net
                            .append_info(n, x, z)
                            .ok_or_else(|| format!("cannot add network {n} info at ({x},{z})"))?;
                        r.net_type = ty as u8;
                        r.inv_cap = cap;
                        r.extra_cost = extra;
                    }
                    _ => return Err("each info entry is [network, type, inv_cap, extra_cost]".into()),
                }
            }
        }
    }
    Ok(())
}

/// The §11 preamble's street cell.
fn street(w: &World, op: &Json) -> R<()> {
    let cells = cells_of(op)?;
    let mut net = w.net.borrow_mut();
    for (x, z) in cells {
        let c = net
            .conn_mut(x, z)
            .ok_or_else(|| format!("({x},{z}) is outside the city"))?;
        c.conn[travel::CAR] = 0x0104;
        c.switch[travel::CAR] = 0xFFFF;
        c.prior_car = [0xFFFF; 4];
        c.wormhole = 0;
        let r = net
            .append_info(1, x, z)
            .ok_or_else(|| format!("cannot add road info at ({x},{z})"))?;
        r.inv_cap = 0.0625;
        r.net_type = net_type::STREET as u8;
        r.extra_cost = 0.0;
    }
    Ok(())
}

fn lot(w: &World, op: &Json) -> R<()> {
    let id = i64_of(field(op, "id")?, "id")? as u64;
    let rect = rect_of(field(op, "rect")?, "rect")?;
    let holder = match opt(op, "holder") {
        Some(h) => i64_of(h, "holder")? as u64,
        None => id + 0x1_0000,
    };
    {
        let mut lots = w.lots.borrow_mut();
        lots.retain(|l| l.0 != id);
        lots.push((id, rect, holder));
    }
    if opt(op, "max").is_some() || opt(op, "used").is_some() {
        set_capacity(w, holder, op)?;
    }
    if let Some(b) = opt(op, "dest") {
        let bits = u32_of(b, "dest")? as u8;
        for x in rect.x1..=rect.x2 {
            for z in rect.z1..=rect.z2 {
                w.set_destination_byte(x, z, bits);
            }
        }
    }
    Ok(())
}

fn set_capacity(w: &World, holder: u64, op: &Json) -> R<()> {
    let mut caps = w.caps.borrow_mut();
    let c = caps.entry(holder).or_insert(([0; 4], [0; 4]));
    if let Some(m) = opt(op, "max") {
        c.0 = u4(m, "max")?;
    }
    if let Some(u) = opt(op, "used") {
        c.1 = u4(u, "used")?;
    }
    Ok(())
}

fn capacity(w: &World, op: &Json) -> R<()> {
    let holder = i64_of(field(op, "holder")?, "holder")? as u64;
    set_capacity(w, holder, op)
}

fn dest(w: &World, op: &Json) -> R<()> {
    let bits = u32_of(field(op, "bits")?, "bits")? as u8;
    for (x, z) in cells_of(op)? {
        w.set_destination_byte(x, z, bits);
    }
    Ok(())
}

fn volume(w: &World, op: &Json) -> R<()> {
    let n = opt(op, "n").map(|v| u32_of(v, "n")).transpose()?.unwrap_or(1);
    let v = fv(field(op, "value")?, "value")?;
    for (x, z) in cells_of(op)? {
        w.set_volume(n, x, z, v);
    }
    Ok(())
}

/// The neighbour edge record of an off-map cell (traffic-network.md §2.7.1),
/// read by `GetTrafficNeighborConnection` in §4.13 step 3.
fn neighbor_cost(w: &World, op: &Json) -> R<()> {
    let (x, z) = pair(field(op, "cell")?, "cell")?;
    let cost = fv(field(op, "cost")?, "cost")?;
    let mask = opt(op, "mask").map(|v| u32_of(v, "mask")).transpose()?.unwrap_or(0) as u16;
    let pool = opt(op, "pool").map(|v| u32_of(v, "pool")).transpose()?.unwrap_or(0) as u16;
    let mut net = w.net.borrow_mut();
    if net.neighbour_edge_index(x, z).is_none() {
        return Err(format!("({x},{z}) is not an off-map edge cell"));
    }
    net.set_neighbour_edge(x, z, NeighbourEdge { mask, pool, cost });
    Ok(())
}

fn pool(w: &World, op: &Json) -> R<()> {
    let holder = i64_of(field(op, "holder")?, "holder")? as u64;
    for c in cells_of(op)? {
        w.pools.borrow_mut().insert(c, holder);
    }
    if opt(op, "max").is_some() || opt(op, "used").is_some() {
        set_capacity(w, holder, op)?;
    }
    Ok(())
}

/// A ferry terminal departing at `cell`: wormhole bit 4, the terminal
/// list entry and the departure table entry; `arrival` (default (0, 0)) is
/// its arrival cell, where §4.6's off-map fallback puts the start node.
fn ferry(w: &World, op: &Json) -> R<()> {
    let (x, z) = pair(field(op, "cell")?, "cell")?;
    let (ax, az) = opt(op, "arrival")
        .map(|v| pair(v, "arrival"))
        .transpose()?
        .unwrap_or((0, 0));
    let mask = u32_of(field(op, "mask")?, "mask")? as u16;
    let repeat = opt(op, "repeat").map(|v| u32_of(v, "repeat")).transpose()?.unwrap_or(1);
    let mut links = Vec::new();
    if let Some(v) = opt(op, "links") {
        for l in arr(v, "links")? {
            match arr(l, "links")? {
                [ax, az, c] => links.push(TerminalLink {
                    arrival_x: u32_of(ax, "x")? as u16,
                    arrival_z: u32_of(az, "z")? as u16,
                    cost: fv(c, "cost")?,
                    ..TerminalLink::default()
                }),
                _ => return Err("each link is [x, z, cost]".into()),
            }
        }
    }
    let links: Vec<TerminalLink> = (0..repeat).flat_map(|_| links.iter().cloned()).collect();
    let mut neighbour_links = Vec::new();
    if let Some(v) = opt(op, "neighbour_links") {
        for l in arr(v, "neighbour_links")? {
            match arr(l, "neighbour_links")? {
                [ox, oz, ex, ez, c] => neighbour_links.push(NeighborLink {
                    off_x: i32_of(ox, "off x")? as i16,
                    off_z: i32_of(oz, "off z")? as i16,
                    edge_x: u32_of(ex, "edge x")? as u16,
                    edge_z: u32_of(ez, "edge z")? as u16,
                    cost: fv(c, "cost")?,
                    ..NeighborLink::default()
                }),
                _ => return Err("each neighbour link is [off x, off z, edge x, edge z, cost]".into()),
            }
        }
    }
    let mut net = w.net.borrow_mut();
    net.conn_mut(x, z)
        .ok_or_else(|| format!("({x},{z}) is outside the city"))?
        .wormhole |= wormhole::FERRY;
    let pos = net.ferries.len() as u32;
    net.ferries.push(FerryTerminal {
        dep_x: x as u16,
        dep_z: z as u16,
        arr_x: ax as u16,
        arr_z: az as u16,
        mask,
        links,
        neighbour_links,
        ..FerryTerminal::default()
    });
    *net.ferry_departures.entry(u32::from(cell_key(x, z)), || pos) = pos;
    Ok(())
}

/// A tunnel portal at `cell` leading to `other`: wormhole bit 2 and the
/// tunnel table entry.
fn tunnel(w: &World, op: &Json) -> R<()> {
    let (x, z) = pair(field(op, "cell")?, "cell")?;
    let other = pair(field(op, "other")?, "other")?;
    let mask = opt(op, "mask").map(|v| u32_of(v, "mask")).transpose()?.unwrap_or(0x1FE) as u16;
    let mut cost = [0.0f32; travel::COUNT];
    if let Some(v) = opt(op, "cost") {
        for e in arr(v, "cost")? {
            match arr(e, "cost")? {
                [t, c] => cost[travel_type(t)?] = fv(c, "cost")?,
                _ => return Err("each cost is [t, v]".into()),
            }
        }
    }
    let mut net = w.net.borrow_mut();
    net.conn_mut(x, z)
        .ok_or_else(|| format!("({x},{z}) is outside the city"))?
        .wormhole |= wormhole::TUNNEL;
    *net.tunnels.entry(u32::from(cell_key(x, z)), TunnelInfo::default) = TunnelInfo {
        other,
        mask,
        cost,
        net_type: 0,
    };
    Ok(())
}

/// A transit switch: the switch table entry and, for `cells`, the switch
/// ID grid.
fn switch(w: &World, op: &Json) -> R<()> {
    let id = u32_of(field(op, "id")?, "id")?;
    let get = |k: &str| opt(op, k).map(|v| fv(v, k)).transpose();
    let (load, capacity, tf) = (get("load")?, get("capacity")?, get("time_factor")?);
    let (cx, cz) = w.cells.get();
    let mut net = w.net.borrow_mut();
    {
        let sw = net.switches.entry(id, TransitSwitch::default);
        if let Some(v) = load {
            sw.load = v;
        }
        if let Some(v) = capacity {
            sw.capacity = v;
        }
        if let Some(v) = tf {
            sw.time_factor = v;
        }
    }
    if opt(op, "cells").is_some() || opt(op, "cell").is_some() || opt(op, "rect").is_some() {
        if net.switch_id.is_none() {
            let city = CityDims {
                cells_x: cx,
                cells_z: cz,
                ..CityDims::square(1)
            };
            net.switch_id = GridProvider::<u32>::create_grid(&CoreGrids { city }, SWITCH_ID_GRID_ID, 1);
        }
        let grid = net.switch_id.clone().ok_or("no switch ID grid")?;
        for (x, z) in cells_of(op)? {
            grid.set(x, z, id);
        }
    }
    Ok(())
}

// ---- the pathfinder objects -------------------------------------------------------------------------

fn pf_id(op: &Json) -> R<u32> {
    opt(op, "pf")
        .map(|v| u32_of(v, "pf"))
        .transpose()
        .map(|v| v.unwrap_or(0))
}

/// `CreatePathFinder` (constructor, AddRef, Init) or the bare constructor.
fn create(w: &World, op: &Json, full: bool) -> R<()> {
    let id = pf_id(op)?;
    let pf = if full {
        Pathfinder::create(w, w.shared.clone())
    } else {
        Pathfinder::new(w.shared.clone())
    };
    w.pfs.borrow_mut().insert(id, pf);
    Ok(())
}

fn with_pf<T>(w: &World, op: &Json, f: impl FnOnce(&mut Pathfinder) -> R<T>) -> R<T> {
    let id = pf_id(op)?;
    let mut pfs = w.pfs.borrow_mut();
    let pf = pfs
        .get_mut(&id)
        .ok_or_else(|| format!("no pathfinder {id} (pf_create first)"))?;
    f(pf)
}

fn call(w: &World, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;
    let got = with_pf(w, op, |pf| {
        Ok(match method {
            "query_interface" => Json::Bool(pf.query_interface(uarg(op, 0)?)),
            "add_ref" => ji(pf.add_ref()),
            "release" => ji(pf.release()),
            "init" => Json::Bool(pf.init(w)),
            "shutdown" => Json::Bool(pf.shutdown()),
            "set_flag" => {
                pf.set_flag(uarg(op, 0)?);
                Json::Null
            }
            "clear_flag" => {
                pf.clear_flag(uarg(op, 0)?);
                Json::Null
            }
            "clear_all_flags" => {
                pf.clear_all_flags();
                Json::Null
            }
            "set_route_info" => {
                let on = bool_of(arg(op, 0)?, "route")?;
                pf.set_route_info(on.then(VecDeque::new));
                Json::Null
            }
            "set_trip_destination_filter" => {
                pf.set_trip_destination_filter(uarg(op, 0)?);
                Json::Null
            }
            "set_destination_flag" => {
                pf.set_destination_flag(uarg(op, 0)? as u8);
                Json::Null
            }
            "clear_destination_flag" => {
                pf.clear_destination_flag(uarg(op, 0)? as u8);
                Json::Null
            }
            "clear_all_destination_flags" => {
                pf.clear_all_destination_flags();
                Json::Null
            }
            "set_travel_type_flag" => {
                pf.set_travel_type_flag(uarg(op, 0)? as u16, bool_of(arg(op, 1)?, "also starting")?);
                Json::Null
            }
            "clear_travel_type_flag" => {
                pf.clear_travel_type_flag(uarg(op, 0)? as u16);
                Json::Null
            }
            "clear_all_travel_type_flags" => {
                pf.clear_all_travel_type_flags();
                Json::Null
            }
            "set_starting_cost_array" => {
                let costs = match arg(op, 0)? {
                    Json::Null => None,
                    j => {
                        let v: Vec<f32> = arr(j, "costs")?.iter().map(|c| fv(c, "cost")).collect::<R<_>>()?;
                        Some(<[f32; 9]>::try_from(v).map_err(|_| "the starting cost array has 9 values")?)
                    }
                };
                pf.set_starting_cost_array(costs);
                Json::Null
            }
            "set_max_trip_length" => {
                pf.set_max_trip_length(farg(op, 0)?);
                Json::Null
            }
            "set_traffic_density" => {
                pf.set_traffic_density(uarg(op, 0)?);
                Json::Null
            }
            "set_start_rect" => {
                pf.set_start_rect(rect_args(op)?);
                Json::Null
            }
            "set_end_rect" => {
                pf.set_end_rect(rect_args(op)?);
                Json::Null
            }
            "set_max_work_time" => {
                pf.set_max_work_time(uarg(op, 0)?);
                Json::Null
            }
            "reset" => {
                pf.reset();
                Json::Null
            }
            "find_path" => {
                let mut net = w.net.borrow_mut();
                ji(pf.find_path(&mut PathCx { net: &mut net, host: w }))
            }
            "apply_traffic" => {
                let mut net = w.net.borrow_mut();
                pf.apply_traffic(&mut PathCx { net: &mut net, host: w }, uarg(op, 0)?);
                Json::Null
            }
            "release_path_nodes" => {
                pf.release_path_nodes();
                Json::Null
            }
            "get_traffic_arrived" => ji(pf.get_traffic_arrived()),
            "get_trip_length" => fb(pf.get_trip_length()),
            "get_trip_length_scaled_for_display" => fb(pf.get_trip_length_scaled_for_display(w)),
            "get_end_cell" => jcell(pf.get_end_cell()),
            "get_last_travel_type" => ji(pf.get_last_travel_type()),
            "get_compressed_route" => {
                let mut out = match opt(op, "args").and_then(|a| a.as_array()).and_then(|a| a.first()) {
                    Some(h) => hex_decode(str_of(h, "initial bytes")?)?,
                    None => Vec::new(),
                };
                let mask = pf.get_compressed_route(&w.net.borrow(), &mut out);
                obj(vec![("bytes", Json::Str(hex_encode(&out))), ("mask", hx(mask))])
            }
            "was_trip_reversible" => Json::Bool(pf.was_trip_reversible(&w.net.borrow())),
            "heuristic" => fb(pf.heuristic(iarg(op, 0)?, iarg(op, 1)?)),
            "score" => fb(pf.score(iarg(op, 0)?, iarg(op, 1)?, farg(op, 2)?)),
            other => return Err(format!("unknown method '{other}'")),
        })
    })?;
    check_expect(op, got)
}

/// The observable state of a pathfinder.
fn pf_state(pf: &Pathfinder) -> Json {
    let nodes = pf.nodes().nodes();
    let node_json = |i: usize| {
        let n = nodes[i];
        obj(vec![
            ("index", ji(i as i64)),
            ("key", hx(n.key)),
            ("cell", jcell((n.x, n.z))),
            ("t", ji(n.t)),
            ("side", ji(n.side)),
            ("g", fb(n.g)),
            ("f", fb(n.f)),
            ("parent", n.parent.map_or(Json::Null, ji)),
            ("open", Json::Bool(n.open)),
            ("closed", Json::Bool(n.closed)),
        ])
    };
    let result = pf
        .result_node()
        .and_then(|r| nodes.iter().position(|n| n.key == r.key))
        .map_or(Json::Null, |i| ji(i as i64));
    let route = pf.route().map_or(Json::Null, |r| {
        Json::Array(
            r.iter()
                .map(|d| Json::Array(vec![ji(d.x), ji(d.z), ji(d.travel_type)]))
                .collect(),
        )
    });
    let ((px, pz), pside) = pf.park_and_ride();
    let over_budget = pf.over_budget();
    obj(vec![
        ("ref_count", ji(pf.ref_count())),
        ("destroyed", Json::Bool(pf.is_destroyed())),
        ("initialised", Json::Bool(pf.is_initialised())),
        ("city_size", jcell(pf.city_size())),
        ("flags", ji(pf.flags())),
        ("filter", ji(pf.trip_destination_filter())),
        ("destination_flags", ji(pf.destination_flags())),
        ("allowed", ji(pf.allowed_travel_types())),
        ("starting", ji(pf.starting_travel_types())),
        (
            "start_costs",
            pf.starting_cost_array()
                .map_or(Json::Null, |c| Json::Array(c.iter().map(|&v| fb(v)).collect())),
        ),
        ("trip_length", fb(pf.get_trip_length())),
        ("density", ji(pf.traffic_density())),
        ("arrived", ji(pf.get_traffic_arrived())),
        ("start_rect", jrect(pf.start_rect())),
        ("end_rect", jrect(pf.end_rect())),
        ("search_rect", jrect(pf.search_rect())),
        ("excluded_rect", jrect(pf.excluded_rect())),
        ("route", route),
        ("max_work_time", ji(pf.max_work_time())),
        ("suspended", Json::Bool(pf.is_suspended())),
        ("max_cost", Json::Array(pf.max_cost().iter().map(|&v| fb(v)).collect())),
        ("weight", fb(pf.weight())),
        ("over_budget", jlist(&over_budget)),
        ("disabled", ji(pf.disabled_travel_types())),
        ("node_count", ji(nodes.len() as i64)),
        ("nodes", Json::Array((0..nodes.len()).map(node_json).collect())),
        ("open_set", jlist(pf.open_set().entries())),
        ("result", result),
        ("end_cell", jcell(pf.get_end_cell())),
        ("last_travel_type", ji(pf.get_last_travel_type())),
        ("destination_type_reached", ji(pf.destination_type_reached())),
        ("park_and_ride", Json::Array(vec![ji(px), ji(pz), ji(pside as i64)])),
        ("work_counter", ji(pf.work_counter())),
    ])
}

fn expect(w: &World, op: &Json) -> R<()> {
    let got = with_pf(w, op, |pf| Ok(pf_state(pf)))?;
    check_fields(&got, field(op, "state")?, "pathfinder")
}

/// The process-wide work-check statics (§4.11): `set` = `[m, a]` (marks
/// them initialised), `expect` = `[m, a]`.
fn work_check(w: &World, op: &Json) -> R<()> {
    if let Some(s) = opt(op, "set") {
        match arr(s, "set")? {
            [m, a] => w.shared.set(WorkCheck {
                initialised: true,
                multiplier: u32_of(m, "m")?,
                adaptation: i32_of(a, "a")?,
            }),
            _ => return Err("'set' must be [m, a]".into()),
        }
    }
    let wc = w.shared.get();
    check_expect(op, Json::Array(vec![ji(wc.multiplier), ji(wc.adaptation)]))
}

// ---- the simulator side ---------------------------------------------------------------------------------

fn expect_world(w: &World, op: &Json) -> R<()> {
    let net = w.net.borrow();
    let mut keys = net.edge_density.keys();
    keys.sort_unstable();
    let mut switch_ids = net.switches.keys();
    switch_ids.sort_unstable();
    let got = obj(vec![
        (
            "volumes",
            Json::Array(
                w.volumes
                    .borrow()
                    .iter()
                    .map(|(&(n, x, z), &v)| Json::Array(vec![ji(n), ji(x), ji(z), fb(v)]))
                    .collect(),
            ),
        ),
        (
            "dest",
            Json::Array(
                w.dest
                    .borrow()
                    .iter()
                    .map(|(&(x, z), &b)| Json::Array(vec![ji(x), ji(z), ji(b)]))
                    .collect(),
            ),
        ),
        ("free", jlist(&w.free.get())),
        ("income", Json::Str(format!("0x{:016X}", w.income.get().to_bits()))),
        ("edge_density_keys", Json::Array(keys.iter().map(|&k| hx(k)).collect())),
        (
            "switch_loads",
            Json::Array(
                switch_ids
                    .iter()
                    .map(|&id| Json::Array(vec![ji(id), fb(net.switches.get(id).map_or(0.0, |s| s.load))]))
                    .collect(),
            ),
        ),
    ]);
    check_fields(&got, field(op, "state")?, "world")
}

/// One edge-density record: `counts` = the non-zero `current` counters as
/// `[entry, exit, value]` (index `4·entry + exit`); every other counter must
/// be 0.
fn expect_edge(w: &World, op: &Json) -> R<()> {
    let (x, z) = pair(field(op, "cell")?, "cell")?;
    let t = opt(op, "t").map(|v| u32_of(v, "t")).transpose()?.unwrap_or(1);
    let evening = opt(op, "evening")
        .map(|v| bool_of(v, "evening"))
        .transpose()?
        .unwrap_or(false);
    let mut want = [0u16; 16];
    for e in arr(field(op, "counts")?, "counts")? {
        match arr(e, "counts")? {
            [entry, exit, v] => {
                let (entry, exit) = (u32_of(entry, "entry")? as usize, u32_of(exit, "exit")? as usize);
                if entry > 3 || exit > 3 {
                    return Err("entry and exit are sides 0…3".into());
                }
                want[4 * entry + exit] = u32_of(v, "count")? as u16;
            }
            _ => return Err("each count is [entry, exit, value]".into()),
        }
    }
    let net = w.net.borrow();
    let got = net.edge_density_at(x, z, t, evening).current;
    if got != want {
        return Err(format!(
            "({x},{z}) t {t} evening {evening}: current {got:?}, expected {want:?}"
        ));
    }
    Ok(())
}

// ---- pure functions ---------------------------------------------------------------------------------

fn chain_of(j: &Json) -> R<Vec<RouteNode>> {
    arr(j, "chain")?
        .iter()
        .map(|n| match arr(n, "chain")? {
            [x, z, t] => Ok(RouteNode {
                x: i32_of(x, "x")?,
                z: i32_of(z, "z")?,
                t: u32_of(t, "t")? as u8,
            }),
            _ => Err("each chain node is [x, z, t]".into()),
        })
        .collect()
}

fn route_json((bytes, mask): (Vec<u8>, u32)) -> Json {
    obj(vec![("bytes", Json::Str(hex_encode(&bytes))), ("mask", hx(mask))])
}

fn cells_json(v: &[(i32, i32)]) -> Json {
    Json::Array(v.iter().map(|&c| jcell(c)).collect())
}

fn eval(w: &World, op: &Json) -> R<()> {
    let name = str_of(field(op, "fn")?, "fn")?;
    let got = match name {
        "heap_order" => {
            let fs: Vec<f32> = arr(arg(op, 0)?, "f")?.iter().map(|v| fv(v, "f")).collect::<R<_>>()?;
            Json::Array(heap_order(&fs).into_iter().map(|i| ji(i as i64)).collect())
        }
        "boundary_cells" => cells_json(&boundary_cells(rect_of(arg(op, 0)?, "rect")?)),
        "start_ring_cells" => cells_json(&start_ring_cells(
            rect_of(arg(op, 0)?, "start")?,
            rect_of(arg(op, 1)?, "ring")?,
        )),
        "grow_clamp" => {
            let (cx, cz) = w.cells.get();
            jrect(grow_clamp(rect_of(arg(op, 0)?, "rect")?, iarg(op, 1)?, cx, cz))
        }
        "encode_route" => route_json(encode_route(&chain_of(arg(op, 0)?)?, &*w.net.borrow())),
        "encode_unit_steps" => {
            // [x0, z0, t, code, count]: `count` unit steps with one code.
            let (mut x, mut z, t, code, count) = (
                iarg(op, 0)?,
                iarg(op, 1)?,
                uarg(op, 2)? as u8,
                uarg(op, 3)?,
                uarg(op, 4)?,
            );
            let (dx, dz) = match code {
                0 => (-1, 0),
                1 => (0, -1),
                2 => (1, 0),
                3 => (0, 1),
                _ => return Err("the step code is 0…3".into()),
            };
            let mut chain = vec![RouteNode { x, z, t }];
            for _ in 0..count {
                x += dx;
                z += dz;
                chain.push(RouteNode { x, z, t });
            }
            route_json(encode_route(&chain, &*w.net.borrow()))
        }
        "distance_to_rect" => fb(distance_to_rect(
            iarg(op, 0)?,
            iarg(op, 1)?,
            rect_of(arg(op, 2)?, "rect")?,
        )),
        "node_key" => hx(node_key(uarg(op, 0)?, uarg(op, 1)?, iarg(op, 2)?, iarg(op, 3)?)),
        "bucket" => ji(NodeStore::bucket(uarg(op, 0)?) as i64),
        other => return Err(format!("unknown fn '{other}'")),
    };
    match op.get("expect") {
        Some(want) if !matches(&got, want, true) => Err(format!("returned {}, expected {}", show(&got), show(want))),
        _ => Ok(()),
    }
}
