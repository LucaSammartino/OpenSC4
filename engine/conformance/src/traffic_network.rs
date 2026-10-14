// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/traffic-network.md`: a scripted city
//! implementing `NetworkHost` (network, subway-access, prop and building
//! occupants with their path connections; lots with populations and trip
//! capacities; surface water; potholes; neighbour connections) and ops that
//! drive `fc4_core::traffic_network::NetworkModel`. See README.md ("Traffic
//! network ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use fc4_core::curve::ResponseCurve;
use fc4_core::grid::{CellRect, CityDims, CoreGrids, GridProvider, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::{FixedSeeds, RzRandom};
use fc4_core::traffic_network::{
    depth_byte, encode_route, heuristic, inverse_max_speed, net_type, read_rules, route_cost, rule_bytes, rule_entries,
    side_of, step_cost, travel, tunnel_distance, CellRegion, LotActivation, NeighbourEdge, NetworkHost, NetworkInfo,
    NetworkModel, PathInfo, Rule, SwitchQueryTarget, TransitSwitch, Tuning, PATH_TYPE_CAR,
};
use fc4_core::variant::Variant;

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::HostRef;
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, f32_of, holder_of, i64_of, ji, matches, opt, show, str_of, u32_of,
    variant_of, R,
};

// ---- the scripted world ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Kind {
    #[default]
    Network,
    Subway,
    Prop,
    Building,
}

#[derive(Clone, Default)]
struct Occ {
    kind: Kind,
    pos: [f32; 3],
    types: Vec<u32>,
    /// (path type, entry, exit).
    conns: Vec<(u32, u32, u32)>,
    /// (path type, entry).
    turns: Vec<(u32, u32)>,
    flags: u32,
    tunnel: bool,
    paired: Option<u64>,
    props: PropertyHolder,
    prop_type: u32,
    bounding: Option<CellRect>,
    /// Lane continuity: car paths (entry, exit byte, first vertex).
    paths: Vec<(u32, u8, u64)>,
    /// Lane continuity: (vertex, exit, entries) of the paths near a vertex.
    near: Vec<(u64, u32, Vec<u32>)>,
    /// Removed from the occupant manager; still readable.
    gone: bool,
}

#[derive(Clone)]
struct Lot {
    rect: CellRect,
    facing: u32,
    building: Option<u64>,
    holder: u64,
    pops: Vec<Vec<(u32, u32)>>,
}

/// A neighbour connection: (token, type, cell).
type Connection = (u64, u32, (i32, i32));

struct World {
    cells: Cell<(i32, i32)>,
    tuning: RefCell<Option<Tuning>>,
    speeds: RefCell<[[f32; net_type::COUNT]; travel::COUNT]>,
    cell_width: Cell<f32>,
    occs: RefCell<Vec<(u64, Occ)>>,
    lots: RefCell<Vec<(u64, Lot)>>,
    holders: RefCell<HashMap<u64, PropertyHolder>>,
    trip_caps: RefCell<HashMap<u64, [u32; 4]>>,
    /// Navigable cells: (depth, body of water).
    water: RefCell<HashMap<(i32, i32), (f32, u16)>>,
    under_water: RefCell<HashSet<(i32, i32)>>,
    damaged: RefCell<HashSet<(i32, i32)>>,
    connections: RefCell<Vec<Connection>>,
    pools: RefCell<HashMap<(i32, i32), u64>>,
    posted: RefCell<Vec<u32>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    model: RefCell<Option<Rc<NetworkModel>>>,
    rng: RefCell<Option<(u32, RzRandom)>>,
}

impl World {
    fn new() -> World {
        World {
            cells: Cell::new((4, 4)),
            tuning: RefCell::new(Some(Tuning::shipped())),
            speeds: RefCell::new([[0.0; net_type::COUNT]; travel::COUNT]),
            cell_width: Cell::new(16.0),
            occs: RefCell::new(Vec::new()),
            lots: RefCell::new(Vec::new()),
            holders: RefCell::new(HashMap::new()),
            trip_caps: RefCell::new(HashMap::new()),
            water: RefCell::new(HashMap::new()),
            under_water: RefCell::new(HashSet::new()),
            damaged: RefCell::new(HashSet::new()),
            connections: RefCell::new(Vec::new()),
            pools: RefCell::new(HashMap::new()),
            posted: RefCell::new(Vec::new()),
            refs: RefCell::new(BTreeMap::new()),
            model: RefCell::new(None),
            rng: RefCell::new(None),
        }
    }

    fn model(&self) -> R<Rc<NetworkModel>> {
        self.model
            .borrow()
            .clone()
            .ok_or_else(|| "no model (tn_init first)".into())
    }

    fn occ(&self, id: u64) -> Occ {
        self.occs
            .borrow()
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, o)| o.clone())
            .unwrap_or_default()
    }

    fn lot(&self, id: u64) -> Option<Lot> {
        self.lots
            .borrow()
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, l)| l.clone())
    }

    fn occupants(&self, kind: Kind, rect: CellRect) -> Vec<u64> {
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| o.kind == kind && !o.gone && inside(rect, self.position_to_cell(o.pos[0], o.pos[2])))
            .map(|(id, _)| *id)
            .collect()
    }

    fn dims(&self) -> CityDims {
        let (x, z) = self.cells.get();
        CityDims {
            cells_x: x,
            cells_z: z,
            ..CityDims::square(1)
        }
    }
}

fn inside(r: CellRect, (x, z): (i32, i32)) -> bool {
    x >= r.x1 && x <= r.x2 && z >= r.z1 && z <= r.z2
}

/// The world position of a cell's centre (16 m cells).
fn centre(x: i32, z: i32) -> [f32; 3] {
    [x as f32 * 16.0 + 8.0, 0.0, z as f32 * 16.0 + 8.0]
}

impl GridProvider<u32> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u32>>> {
        GridProvider::<u32>::create_grid(&CoreGrids { city: self.dims() }, instance, tract_size)
    }
}

impl GridProvider<u8> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        GridProvider::<u8>::create_grid(&CoreGrids { city: self.dims() }, instance, tract_size)
    }
}

impl NetworkHost for World {
    fn cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        ((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
    }
    fn network_occupants_in_rect(&self, rect: CellRect) -> Vec<u64> {
        self.occupants(Kind::Network, rect)
    }
    fn subway_occupants_in_rect(&self, rect: CellRect) -> Vec<u64> {
        self.occupants(Kind::Subway, rect)
    }
    fn prop_occupants_in_rect(&self, rect: CellRect) -> Vec<u64> {
        self.occupants(Kind::Prop, rect)
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        self.occ(occ).pos
    }
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect {
        let o = self.occ(occ);
        o.bounding.unwrap_or_else(|| {
            let (x, z) = self.position_to_cell(o.pos[0], o.pos[2]);
            CellRect::new(x, z, x, z)
        })
    }
    fn prop_type(&self, occ: u64) -> u32 {
        self.occ(occ).prop_type
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        self.occ(occ).props.get(id).cloned()
    }
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool {
        self.occ(occ).flags & flag == flag
    }
    fn has_any_network_flag(&self, occ: u64, mask: u32) -> bool {
        self.occ(occ).flags & mask != 0
    }
    fn is_of_type(&self, occ: u64, network_type: u32) -> bool {
        self.occ(occ).types.contains(&network_type)
    }
    fn path_connection(&self, occ: u64, path_type: u32, entry: u32, exit: u32) -> bool {
        self.occ(occ).conns.contains(&(path_type, entry, exit))
    }
    fn path_info(&self, occ: u64) -> Option<PathInfo> {
        Some(occ)
    }
    fn path_info_turn_around(&self, pi: PathInfo, path_type: u32, entry: u32) -> bool {
        self.occ(pi).turns.contains(&(path_type, entry))
    }
    fn path_info_exits_from_entry(&self, pi: PathInfo, path_type: u32, entry: u32) -> Vec<(u8, u64)> {
        if path_type != PATH_TYPE_CAR {
            return Vec::new();
        }
        self.occ(pi)
            .paths
            .iter()
            .filter(|p| p.0 == entry)
            .map(|p| (p.1, p.2))
            .collect()
    }
    fn path_info_entries_near(&self, pi: PathInfo, path_type: u32, vertex: u64, exit: u32, _radius: f32) -> Vec<u32> {
        if path_type != PATH_TYPE_CAR {
            return Vec::new();
        }
        self.occ(pi)
            .near
            .iter()
            .filter(|n| n.0 == vertex && n.1 == exit)
            .flat_map(|n| n.2.clone())
            .collect()
    }
    fn is_tunnel(&self, occ: u64) -> bool {
        self.occ(occ).tunnel
    }
    fn tunnel_paired_portal(&self, occ: u64) -> Option<u64> {
        self.occ(occ).paired
    }
    fn lot_at(&self, x: i32, z: i32, anchor_only: bool) -> Option<u64> {
        self.lots
            .borrow()
            .iter()
            .find(|(_, l)| {
                if anchor_only {
                    (l.rect.x1, l.rect.z1) == (x, z)
                } else {
                    inside(l.rect, (x, z))
                }
            })
            .map(|(id, _)| *id)
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.lot(lot).map_or(CellRect::new(0, 0, -1, -1), |l| l.rect)
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        self.lot(lot).map_or((0, 0), |l| (l.rect.x1, l.rect.z1))
    }
    fn lot_facing(&self, lot: u64) -> u32 {
        self.lot(lot).map_or(0, |l| l.facing)
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        self.lot(lot).and_then(|l| l.building)
    }
    fn lot_populations(&self, lot: u64) -> Vec<Vec<(u32, u32)>> {
        self.lot(lot).map(|l| l.pops).unwrap_or_default()
    }
    fn lot_holder(&self, lot: u64) -> u64 {
        self.lot(lot).map_or(0, |l| l.holder)
    }
    fn holder_property(&self, holder: u64, id: u32) -> Option<Variant> {
        self.holders.borrow().get(&holder).and_then(|h| h.get(id).cloned())
    }
    fn set_holder_property(&self, holder: u64, id: u32, value: Variant) {
        self.holders.borrow_mut().entry(holder).or_default().add(id, value);
    }
    fn remove_holder_property(&self, holder: u64, id: u32) -> bool {
        self.holders.borrow_mut().get_mut(&holder).is_some_and(|h| h.remove(id))
    }
    fn max_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        self.trip_caps
            .borrow()
            .get(&holder)
            .map_or(0, |c| c.get(k as usize).copied().unwrap_or(0))
    }
    fn neighbor_pool_holder(&self, x: i32, z: i32) -> Option<u64> {
        self.pools.borrow().get(&(x, z)).copied()
    }
    fn is_road_damaged(&self, x: i32, z: i32) -> bool {
        self.damaged.borrow().contains(&(x, z))
    }
    fn cell_under_water(&self, x: i32, z: i32) -> bool {
        self.under_water.borrow().contains(&(x, z))
    }
    fn is_navigable(&self, x: i32, z: i32) -> bool {
        self.water.borrow().contains_key(&(x, z))
    }
    fn water_depth(&self, x: i32, z: i32) -> f32 {
        self.water.borrow().get(&(x, z)).map_or(0.0, |w| w.0)
    }
    fn body_of_water(&self, x: i32, z: i32) -> Option<u16> {
        Some(self.water.borrow().get(&(x, z)).map_or(0, |w| w.1))
    }
    fn connections_of_type(&self, connection_type: u32) -> Vec<u64> {
        self.connections
            .borrow()
            .iter()
            .filter(|c| c.1 == connection_type)
            .map(|c| c.0)
            .collect()
    }
    fn all_connections(&self) -> Vec<u64> {
        self.connections.borrow().iter().map(|c| c.0).collect()
    }
    fn connection_cell(&self, conn: u64) -> (i32, i32) {
        self.connections
            .borrow()
            .iter()
            .find(|c| c.0 == conn)
            .map_or((0, 0), |c| c.2)
    }
    fn post(&self, msg_type: u32) {
        self.posted.borrow_mut().push(msg_type);
    }
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
    }
}

// ---- dispatch ---------------------------------------------------------------------------------------

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("tn_")?;
    let w = scalar::world(host, "traffic_network", || Rc::new(World::new()));
    Some(match rest {
        "env" => env(&w, op),
        "init" => init(&w),
        "occupant" => occupant(&w, op),
        "remove_occupant" => remove_occupant(&w, op),
        "lot" => lot(&w, op),
        "water" => water(&w, op),
        "damaged" => cells_into(op, &w.damaged),
        "connection" => connection(&w, op),
        "curve" => curve(&w, op),
        "switch" => switch(&w, op),
        "set_cell" => set_cell(&w, op),
        "rng" => rng(&w, op),
        "call" => call(&w, op),
        "eval" => eval(op),
        "expect_cell" => expect_cell(&w, op),
        "expect_state" => expect_state(&w, op),
        "expect_tunnel" => expect_tunnel(&w, op),
        "expect_lot" => expect_lot(&w, op),
        "expect_switch" => expect_switch(&w, op),
        "expect_ferry" => expect_ferry(&w, op),
        "expect_rng" => expect_rng(&w, op),
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

fn u32_list(j: &Json, what: &str) -> R<Vec<u32>> {
    arr(j, what)?.iter().map(|v| u32_of(v, what)).collect()
}

fn cell_list(j: &Json, what: &str) -> R<Vec<(i32, i32)>> {
    arr(j, what)?.iter().map(|c| pair(c, what)).collect()
}

fn id_of(op: &Json, key: &str) -> R<u64> {
    Ok(i64_of(
        scalar::opt(op, key).ok_or_else(|| format!("missing field '{key}'"))?,
        key,
    )? as u64)
}

fn iarg(op: &Json, i: usize) -> R<i32> {
    i32_of(arg(op, i)?, "argument")
}

fn uarg(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}

/// A float as its exact bit pattern, `"0x3C23D70A"` (upper case).
fn fb(v: f32) -> Json {
    Json::Str(format!("0x{:08X}", v.to_bits()))
}

fn hex(b: &[u8]) -> Json {
    Json::Str(hex_encode(b))
}

fn obj(fields: Vec<(&str, Json)>) -> Json {
    Json::Object(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn jcell((x, z): (i32, i32)) -> Json {
    Json::Array(vec![ji(x), ji(z)])
}

fn jlist<T: Copy + Into<i64>>(v: &[T]) -> Json {
    Json::Array(v.iter().map(|&x| ji(x)).collect())
}

/// Compares every key of `want` with the same key of `got`, objects
/// partially (only the keys `want` gives).
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

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(c) = opt(op, "cells") {
        w.cells.set(pair(c, "cells")?);
    }
    if let Some(t) = op.get("tuning") {
        *w.tuning.borrow_mut() = match t {
            Json::Null => None,
            other if str_of(other, "tuning")? == "shipped" => Some(Tuning::shipped()),
            _ => return Err("'tuning' must be \"shipped\" or null".into()),
        };
    }
    if let Some(s) = opt(op, "speeds") {
        let mut speeds = w.speeds.borrow_mut();
        for e in arr(s, "speeds")? {
            match arr(e, "speeds")? {
                [t, ty, v] => {
                    let (t, ty) = (u32_of(t, "t")? as usize, u32_of(ty, "type")? as usize);
                    if t >= travel::COUNT || ty >= net_type::COUNT {
                        return Err("speed index out of range".into());
                    }
                    speeds[t][ty] = f32_of(v, "speed")?;
                }
                _ => return Err("each speed is [t, type, v]".into()),
            }
        }
    }
    if let Some(cw) = opt(op, "cell_width") {
        w.cell_width.set(f32_of(cw, "cell_width")?);
    }
    Ok(())
}

/// Creates the model, applies the simulator-owned inputs (tuning, the
/// inverse speed table) and runs `Init` (§1).
fn init(w: &Rc<World>) -> R<()> {
    let m = NetworkModel::new(w.clone() as Rc<dyn NetworkHost>);
    if let Some(t) = w.tuning.borrow().clone() {
        m.set_tuning(t);
    }
    m.set_inv_max_speed(inverse_max_speed(&w.speeds.borrow(), w.cell_width.get()));
    m.init();
    *w.model.borrow_mut() = Some(m);
    Ok(())
}

fn occupant(w: &World, op: &Json) -> R<()> {
    let id = id_of(op, "id")?;
    let kind = match opt(op, "kind").map(|k| str_of(k, "kind")).transpose()? {
        None | Some("network") => Kind::Network,
        Some("subway") => Kind::Subway,
        Some("prop") => Kind::Prop,
        Some("building") => Kind::Building,
        Some(other) => return Err(format!("unknown occupant kind '{other}'")),
    };
    let pos = if let Some(p) = opt(op, "pos") {
        match arr(p, "pos")? {
            [x, y, z] => [f32_of(x, "pos")?, f32_of(y, "pos")?, f32_of(z, "pos")?],
            _ => return Err("'pos' must be [x, y, z]".into()),
        }
    } else {
        let (x, z) = pair(opt(op, "cell").ok_or("an occupant needs 'cell' or 'pos'")?, "cell")?;
        centre(x, z)
    };
    let path_type = opt(op, "path_type").map(|p| u32_of(p, "path_type")).transpose()?;
    let pt = path_type.unwrap_or(PATH_TYPE_CAR);
    let mut o = Occ {
        kind,
        pos,
        ..Occ::default()
    };
    if let Some(t) = opt(op, "types") {
        o.types = u32_list(t, "types")?;
    }
    if let Some(c) = opt(op, "conns") {
        for e in arr(c, "conns")? {
            match u32_list(e, "conns")?.as_slice() {
                [entry, exit] => o.conns.push((pt, *entry, *exit)),
                [p, entry, exit] => o.conns.push((*p, *entry, *exit)),
                _ => return Err("each connection is [entry, exit] or [path type, entry, exit]".into()),
            }
        }
    }
    if let Some(t) = opt(op, "turns") {
        o.turns = u32_list(t, "turns")?.into_iter().map(|e| (pt, e)).collect();
    }
    if let Some(f) = opt(op, "flags") {
        o.flags = u32_of(f, "flags")?;
    }
    if let Some(p) = opt(op, "tunnel") {
        o.tunnel = true;
        o.paired = match p {
            Json::Bool(_) => None,
            other => Some(i64_of(other, "tunnel")? as u64),
        };
    }
    if let Some(p) = opt(op, "props") {
        o.props = holder_of(p)?;
    }
    if let Some(t) = opt(op, "prop_type") {
        o.prop_type = u32_of(t, "prop_type")?;
    }
    if let Some(b) = opt(op, "bounding") {
        o.bounding = Some(rect_of(b, "bounding")?);
    }
    if let Some(p) = opt(op, "paths") {
        for e in arr(p, "paths")? {
            match u32_list(e, "paths")?.as_slice() {
                [entry, exit, v] => o.paths.push((*entry, *exit as u8, u64::from(*v))),
                _ => return Err("each path is [entry, exit, vertex]".into()),
            }
        }
    }
    if let Some(n) = opt(op, "near") {
        for e in arr(n, "near")? {
            match arr(e, "near")? {
                [v, x, entries] => o.near.push((
                    u64::from(u32_of(v, "vertex")?),
                    u32_of(x, "exit")?,
                    u32_list(entries, "entries")?,
                )),
                _ => return Err("each near entry is [vertex, exit, [entries]]".into()),
            }
        }
    }
    let mut occs = w.occs.borrow_mut();
    match occs.iter_mut().find(|(i, _)| *i == id) {
        Some(slot) => slot.1 = o,
        None => occs.push((id, o)),
    }
    Ok(())
}

fn remove_occupant(w: &World, op: &Json) -> R<()> {
    let id = id_of(op, "id")?;
    let mut occs = w.occs.borrow_mut();
    let o = occs
        .iter_mut()
        .find(|(i, _)| *i == id)
        .ok_or_else(|| format!("no occupant {id}"))?;
    o.1.gone = true;
    Ok(())
}

fn lot(w: &World, op: &Json) -> R<()> {
    let id = id_of(op, "id")?;
    let rect = rect_of(scalar::opt(op, "rect").ok_or("a lot needs 'rect'")?, "rect")?;
    let holder = match opt(op, "holder") {
        Some(h) => i64_of(h, "holder")? as u64,
        None => id + 0x1_0000,
    };
    let mut pops = Vec::new();
    if let Some(p) = opt(op, "pops") {
        for map in arr(p, "pops")? {
            let mut m = Vec::new();
            for e in arr(map, "pops")? {
                match u32_list(e, "pops")?.as_slice() {
                    [ty, n] => m.push((*ty, *n)),
                    _ => return Err("each population is [type, count]".into()),
                }
            }
            pops.push(m);
        }
    }
    if let Some(c) = opt(op, "trip_caps") {
        let v = u32_list(c, "trip_caps")?;
        let caps: [u32; 4] = v.try_into().map_err(|_| "'trip_caps' must have 4 values")?;
        w.trip_caps.borrow_mut().insert(holder, caps);
    }
    let props = match opt(op, "props") {
        Some(p) => holder_of(p)?,
        None => PropertyHolder::new(),
    };
    w.holders.borrow_mut().insert(holder, props);
    let l = Lot {
        rect,
        facing: opt(op, "facing").map(|f| u32_of(f, "facing")).transpose()?.unwrap_or(0),
        building: opt(op, "building")
            .map(|b| i64_of(b, "building").map(|v| v as u64))
            .transpose()?,
        holder,
        pops,
    };
    let mut lots = w.lots.borrow_mut();
    match lots.iter_mut().find(|(i, _)| *i == id) {
        Some(slot) => slot.1 = l,
        None => lots.push((id, l)),
    }
    Ok(())
}

/// `cells` (`[[x, z]]`) or `rect`, with `depth`, `body` (1) and
/// `under_water` (true).
fn water(w: &World, op: &Json) -> R<()> {
    let mut cells = match opt(op, "cells") {
        Some(c) => cell_list(c, "cells")?,
        None => Vec::new(),
    };
    if let Some(r) = opt(op, "rect") {
        let r = rect_of(r, "rect")?;
        cells.extend((r.x1..=r.x2).flat_map(|x| (r.z1..=r.z2).map(move |z| (x, z))));
    }
    let depth = f32_of(opt(op, "depth").ok_or("water needs 'depth'")?, "depth")?;
    let body = opt(op, "body").map(|b| u32_of(b, "body")).transpose()?.unwrap_or(1) as u16;
    let under = opt(op, "under_water")
        .map(|u| bool_of(u, "under_water"))
        .transpose()?
        .unwrap_or(true);
    for c in cells {
        w.water.borrow_mut().insert(c, (depth, body));
        if under {
            w.under_water.borrow_mut().insert(c);
        }
    }
    Ok(())
}

fn cells_into(op: &Json, set: &RefCell<HashSet<(i32, i32)>>) -> R<()> {
    let cells = cell_list(scalar::opt(op, "cells").ok_or("missing 'cells'")?, "cells")?;
    set.borrow_mut().extend(cells);
    Ok(())
}

fn connection(w: &World, op: &Json) -> R<()> {
    let id = id_of(op, "id")?;
    let ty = u32_of(scalar::opt(op, "type").ok_or("missing 'type'")?, "type")?;
    let cell = pair(scalar::opt(op, "cell").ok_or("missing 'cell'")?, "cell")?;
    w.connections.borrow_mut().push((id, ty, cell));
    if let Some(p) = opt(op, "pool") {
        let holder = i64_of(p, "pool")? as u64;
        let at = match opt(op, "pool_cells") {
            Some(c) => cell_list(c, "pool_cells")?,
            None => vec![cell],
        };
        for c in at {
            w.pools.borrow_mut().insert(c, holder);
        }
        if let Some(c) = opt(op, "trip_caps") {
            let v = u32_list(c, "trip_caps")?;
            let caps: [u32; 4] = v.try_into().map_err(|_| "'trip_caps' must have 4 values")?;
            w.trip_caps.borrow_mut().insert(holder, caps);
        }
    }
    Ok(())
}

fn curve(w: &World, op: &Json) -> R<()> {
    let mut points = Vec::new();
    for p in arr(scalar::opt(op, "points").ok_or("missing 'points'")?, "points")? {
        match arr(p, "points")? {
            [x, y] => points.push((f32_of(x, "x")?, f32_of(y, "y")?)),
            _ => return Err("each point is [x, y]".into()),
        }
    }
    let mult = opt(op, "multiplier")
        .map(|m| f32_of(m, "multiplier"))
        .transpose()?
        .unwrap_or(1.0);
    w.model()?
        .set_switch_cost_inputs(ResponseCurve::from_points(&points), mult);
    Ok(())
}

/// A switch table entry set directly (test setup: §11 test 10, and a
/// switch as a load leaves it for the DoPostCityInit re-read, §4.7.2).
fn switch(w: &World, op: &Json) -> R<()> {
    let id = u32_of(scalar::opt(op, "id").ok_or("missing 'id'")?, "id")?;
    let get = |k: &str| opt(op, k).map(|v| f32_of(v, k)).transpose();
    let (load, capacity, tf) = (get("load")?, get("capacity")?, get("time_factor")?);
    let int = |k: &str| opt(op, k).map(|v| i64_of(v, k)).transpose();
    let (building, flags, to_mask) = (int("building")?, int("flags")?, int("to_mask")?);
    let has_access = opt(op, "has_access_point")
        .map(|v| bool_of(v, "has_access_point"))
        .transpose()?;
    let access = opt(op, "access").map(|v| pair(v, "access")).transpose()?;
    let rules = opt(op, "rules")
        .map(|v| arr(v, "rules")?.iter().map(rule_of).collect::<R<Vec<Rule>>>())
        .transpose()?;
    let cells = opt(op, "cells").map(|c| cell_list(c, "cells")).transpose()?;
    let mode = int("mode_mask")?.unwrap_or(0) as u8;
    w.model()?.update_state(|s| {
        let sw = s.switches.entry(id, TransitSwitch::default);
        if let Some(v) = load {
            sw.load = v;
        }
        if let Some(v) = capacity {
            sw.capacity = v;
        }
        if let Some(v) = tf {
            sw.time_factor = v;
        }
        if let Some(v) = building {
            sw.building = v as u64;
        }
        if let Some(v) = flags {
            sw.flags = v as u8;
        }
        if let Some(v) = to_mask {
            sw.to_mask = v as u16;
        }
        if let Some(v) = has_access {
            sw.has_access_point = v;
        }
        if let Some((x, z)) = access {
            (sw.access_x, sw.access_z) = (x as u8, z as u8);
        }
        if let Some(r) = rules {
            sw.rules = r;
        }
        for &(x, z) in cells.iter().flatten() {
            if let Some(g) = &s.switch_id {
                g.set(x, z, id);
            }
            if let Some(g) = &s.switch_mode {
                g.set(x, z, mode);
            }
        }
    });
    Ok(())
}

/// A rule as `{kind, side: [4 bools], from, to}` (all optional, default 0 /
/// false), the shape `tn_expect_switch` reports.
fn rule_of(j: &Json) -> R<Rule> {
    let int = |k: &str| {
        opt(j, k)
            .map(|v| u32_of(v, k))
            .transpose()
            .map(|v| v.unwrap_or(0) as u8)
    };
    let mut side = [false; 4];
    if let Some(sd) = opt(j, "side") {
        match arr(sd, "side")? {
            [a, b, c, d] => {
                side = [
                    bool_of(a, "side")?,
                    bool_of(b, "side")?,
                    bool_of(c, "side")?,
                    bool_of(d, "side")?,
                ]
            }
            _ => return Err("'side' must have 4 values".into()),
        }
    }
    Ok(Rule {
        kind: int("kind")?,
        side,
        from: int("from")?,
        to: int("to")?,
    })
}

/// Pokes a cell connection record (test setup, §11 test 6).
fn set_cell(w: &World, op: &Json) -> R<()> {
    let cells = match opt(op, "cells") {
        Some(c) => cell_list(c, "cells")?,
        None => vec![pair(scalar::opt(op, "cell").ok_or("missing 'cell'")?, "cell")?],
    };
    let prior = opt(op, "prior_car").map(|p| u32_list(p, "prior_car")).transpose()?;
    let m = w.model()?;
    m.update_state(|s| -> R<()> {
        for (x, z) in cells {
            let c = s.conn_mut(x, z).ok_or("set_cell outside the city")?;
            if let Some(p) = &prior {
                let p: [u32; 4] = p.clone().try_into().map_err(|_| "'prior_car' must have 4 values")?;
                c.prior_car = p.map(|v| v as u16);
            }
        }
        Ok(())
    })
}

fn rng(w: &World, op: &Json) -> R<()> {
    let seed = u32_of(scalar::opt(op, "seed").ok_or("missing 'seed'")?, "seed")?;
    *w.rng.borrow_mut() = Some((seed, RzRandom::new(seed, &SEEDS)));
    Ok(())
}

const SEEDS: FixedSeeds = FixedSeeds {
    wall_clock: 1,
    process_clock: 1,
};

// ---- calls ---------------------------------------------------------------------------------------

fn call(w: &World, op: &Json) -> R<()> {
    let m = w.model()?;
    let method = str_of(scalar::opt(op, "method").ok_or("missing 'method'")?, "method")?;
    let rect4 = |i: usize| -> R<CellRect> {
        Ok(CellRect::new(
            iarg(op, i)?,
            iarg(op, i + 1)?,
            iarg(op, i + 2)?,
            iarg(op, i + 3)?,
        ))
    };
    let lot = |i: usize| -> R<u64> { Ok(i64_of(arg(op, i)?, "lot")? as u64) };
    let got = match method {
        "occupant_changed" => {
            let removed = op
                .get("args")
                .and_then(|a| a.as_array())
                .and_then(|a| a.get(1))
                .map(|b| bool_of(b, "removed"))
                .transpose()?
                .unwrap_or(false);
            m.occupant_changed(i64_of(arg(op, 0)?, "occupant")? as u64, removed);
            Json::Null
        }
        "do_post_city_init" => {
            m.do_post_city_init();
            Json::Null
        }
        "post_city_init_rebuild" => {
            m.post_city_init_rebuild();
            Json::Null
        }
        "post_city_init_finish" => {
            m.post_city_init_finish();
            Json::Null
        }
        "do_connections_changed" => {
            m.do_connections_changed(rect4(0)?);
            Json::Null
        }
        "set_connection" => {
            m.set_connection(
                iarg(op, 0)?,
                iarg(op, 1)?,
                uarg(op, 2)? as usize,
                uarg(op, 3)? as usize,
                uarg(op, 4)? as usize,
            );
            Json::Null
        }
        "update_intersection_data" => {
            m.update_intersection_data();
            Json::Null
        }
        "get_capacity" => fb(m.get_capacity(uarg(op, 0)?, iarg(op, 1)?, iarg(op, 2)?)),
        "get_connection_count" => ji(m.get_connection_count(uarg(op, 0)?, iarg(op, 1)?, iarg(op, 2)?)),
        "are_lots_connected" => Json::Bool(m.are_lots_connected(lot(0)?, lot(1)?)),
        "get_connected_occupant_count" => ji(m.get_connected_occupant_count(lot(0)?, uarg(op, 1)?)),
        "get_connected_destination_count" => ji(m.get_connected_destination_count(lot(0)?, iarg(op, 1)?)),
        "get_subnetworks_for_lot" => {
            let (ids, ok) = m.get_subnetworks_for_lot(lot(0)?);
            obj(vec![("ids", jlist(&ids)), ("ok", Json::Bool(ok))])
        }
        "get_subnetworks_in_rectangle" => jlist(&m.get_subnetworks_in_rectangle(rect4(0)?)),
        "get_subnetworks_in_region" => {
            let region = CellRegion::from_cells(rect4(0)?, &cell_list(arg(op, 4)?, "cells")?);
            jlist(&m.get_subnetworks_in_region(&region))
        }
        "get_occupant_count_for_all_subnetworks" => jlist(&m.get_occupant_count_for_all_subnetworks(uarg(op, 0)?)),
        "transit_switch_entry_cost" => fb(m.transit_switch_entry_cost(uarg(op, 0)?)),
        "lot_activated" => Json::Bool(m.lot_activated(lot(0)?)),
        "lot_activated_outcome" => {
            let out = m.lot_activated_outcome(lot(0)?);
            let name = match out {
                LotActivation::NoBuilding => "no_building",
                LotActivation::Returned => "returned",
                LotActivation::Done { .. } => "done",
            };
            obj(vec![
                ("outcome", Json::Str(name.into())),
                ("switch_created", Json::Bool(out.switch_created())),
                ("continues", Json::Bool(out.continues())),
            ])
        }
        "lot_deactivated" => {
            m.lot_deactivated(lot(0)?);
            Json::Null
        }
        "get_transit_switches" => Json::Array(m.get_transit_switches().into_iter().map(|t| ji(t as i64)).collect()),
        "transit_switch_query_target" => match m.transit_switch_query_target(iarg(op, 0)?, iarg(op, 1)?) {
            None => Json::Null,
            Some(SwitchQueryTarget::Switch(id)) => obj(vec![("switch", ji(id))]),
            Some(SwitchQueryTarget::Ferry(i)) => obj(vec![("ferry", ji(i as i64))]),
        },
        "tunnel_changed" => {
            let removed = bool_of(arg(op, 1)?, "removed")?;
            Json::Bool(m.tunnel_changed(i64_of(arg(op, 0)?, "occupant")? as u64, removed))
        }
        "find_water_path" => match m.find_water_path(iarg(op, 0)?, iarg(op, 1)?, iarg(op, 2)?, iarg(op, 3)?) {
            None => Json::Null,
            Some(p) => obj(vec![
                ("cells", ji(p.cells)),
                ("g", fb(p.g)),
                ("forward", hex(&p.forward)),
                ("backward", hex(&p.backward)),
            ]),
        },
        "get_water_route" => {
            let mut route = match op.get("args").and_then(|a| a.as_array()).and_then(|a| a.get(4)) {
                Some(h) => hex_decode(str_of(h, "route")?)?,
                None => Vec::new(),
            };
            let ok = m.get_water_route(iarg(op, 0)?, iarg(op, 1)?, iarg(op, 2)?, iarg(op, 3)?, &mut route);
            obj(vec![("ok", Json::Bool(ok)), ("route", hex(&route))])
        }
        "water_terrain_changed" => {
            m.water_terrain_changed(uarg(op, 0)?, uarg(op, 1)?);
            Json::Null
        }
        "water_update" => {
            m.water_update();
            Json::Null
        }
        "has_map_exit" => Json::Bool(m.has_map_exit(iarg(op, 0)?, iarg(op, 1)?)),
        "neighbor_connection_inner_cell" => match m.neighbor_connection_inner_cell(iarg(op, 0)?, iarg(op, 1)?) {
            None => Json::Null,
            Some(((x, z), d)) => Json::Array(vec![ji(x), ji(z), ji(d as i64)]),
        },
        "set_neighbour_edge" => {
            let rec = NeighbourEdge {
                mask: uarg(op, 2)? as u16,
                pool: uarg(op, 3)? as u16,
                cost: f32_of(arg(op, 4)?, "cost")?,
            };
            m.set_neighbour_edge(iarg(op, 0)?, iarg(op, 1)?, rec);
            Json::Null
        }
        "begin_ferry_pass" => ji(m.begin_ferry_pass()),
        "work_on_ferry_routes" => Json::Bool(m.work_on_ferry_routes(uarg(op, 0)? as usize)),
        "ferry_terminal_count" => ji(m.ferry_terminal_count(uarg(op, 0)?)),
        "all_ferry_routes" => Json::Array(m.all_ferry_routes().iter().map(|r| hex(r)).collect()),
        "ferry_route_between_tiles" => {
            match m.ferry_route_between_tiles(iarg(op, 0)?, iarg(op, 1)?, iarg(op, 2)?, iarg(op, 3)?) {
                None => Json::Null,
                Some(r) => hex(&r),
            }
        }
        "ferry_routes_in_use" => Json::Array(
            m.ferry_routes_in_use()
                .iter()
                .map(|r| {
                    obj(vec![
                        ("lot_a", ji(r.lot_a as i64)),
                        ("lot_b", ji(r.lot_b as i64)),
                        ("type", ji(r.ferry_type)),
                        ("counter1", jlist(&r.counter1)),
                        ("counter2", jlist(&r.counter2)),
                        ("neighbour", ji(r.neighbour)),
                        ("cell", jcell((i32::from(r.neighbour_x), i32::from(r.neighbour_z)))),
                        ("route_ab", hex(&r.route_ab)),
                        ("route_ba", hex(&r.route_ba)),
                    ])
                })
                .collect(),
        ),
        "clear_subnetworks" => {
            m.clear_subnetworks();
            Json::Null
        }
        "update_subnetworks" => {
            m.update_subnetworks();
            Json::Null
        }
        "fill" => {
            let (x, z, e) = (iarg(op, 0)?, iarg(op, 1)?, uarg(op, 2)? as usize);
            m.update_state(|s| s.fill(x, z, e));
            Json::Null
        }
        "begin_subnetwork_info" => {
            m.begin_subnetwork_info();
            Json::Null
        }
        "work_on_subnetwork_info" => Json::Bool(m.work_on_subnetwork_info(uarg(op, 0)? as usize)),
        "update_subnetwork_info" => {
            m.update_subnetwork_info();
            Json::Null
        }
        other => return Err(format!("unknown method '{other}'")),
    };
    check_expect(op, got)
}

// ---- pure functions ----------------------------------------------------------------------------------

fn eval(op: &Json) -> R<()> {
    let f = str_of(scalar::opt(op, "fn").ok_or("missing 'fn'")?, "fn")?;
    let fa = |i: usize| -> R<f32> { f32_of(arg(op, i)?, "argument") };
    let got = match f {
        "depth_byte" => ji(depth_byte(bool_of(arg(op, 0)?, "navigable")?, fa(1)?)),
        "step_cost" => fb(step_cost(uarg(op, 0)? as u8, fa(1)?)),
        "heuristic" => fb(heuristic(iarg(op, 0)?, iarg(op, 1)?)),
        "route_cost" => fb(route_cost(uarg(op, 0)?)),
        "encode_route" => hex(&encode_route(&cell_list(arg(op, 0)?, "path")?)),
        "side_of" => ji(side_of((iarg(op, 0)?, iarg(op, 1)?), (iarg(op, 2)?, iarg(op, 3)?)) as i64),
        "tunnel_distance" => fb(tunnel_distance(fa(0)?, fa(1)?, fa(2)?)),
        "inverse_max_speed" => {
            let mut speeds = [[0.0f32; net_type::COUNT]; travel::COUNT];
            speeds[0][0] = fa(0)?;
            fb(inverse_max_speed(&speeds, fa(1)?)[0][0])
        }
        "read_rules" => rules_json(read_rules(uarg(op, 0)?, &u32_list(arg(op, 1)?, "entries")?), None),
        "rule_bytes" => jlist(&rule_bytes(&variant_of(arg(op, 0)?)?)),
        "read_rule_property" => {
            let bytes = rule_bytes(&variant_of(arg(op, 1)?)?);
            rules_json(read_rules(uarg(op, 0)?, &rule_entries(&bytes)), Some(bytes.len()))
        }
        other => return Err(format!("unknown fn '{other}'")),
    };
    // Objects are compared partially (only the keys `expect` gives).
    match op.get("expect") {
        Some(want) if !matches(&got, want, true) => Err(format!("returned {}, expected {}", show(&got), show(want))),
        _ => Ok(()),
    }
}

/// `{rules, to_mask, flags, mode_mask}`, plus `count` (the byte count) for
/// a property.
fn rules_json(rs: fc4_core::traffic_network::RuleSet, count: Option<usize>) -> Json {
    let sw = TransitSwitch {
        rules: rs.rules.clone(),
        ..TransitSwitch::default()
    };
    let mut fields = vec![
        ("rules", Json::Array(rs.rules.iter().map(rule_json).collect())),
        ("to_mask", ji(rs.to_mask)),
        ("flags", ji(rs.flags)),
        ("mode_mask", ji(sw.mode_mask())),
    ];
    if let Some(n) = count {
        fields.push(("count", ji(n as i64)));
    }
    obj(fields)
}

fn rule_json(r: &Rule) -> Json {
    obj(vec![
        ("kind", ji(r.kind)),
        ("side", Json::Array(r.side.iter().map(|&b| Json::Bool(b)).collect())),
        ("from", ji(r.from)),
        ("to", ji(r.to)),
    ])
}

// ---- checks ------------------------------------------------------------------------------------------

fn record_json(r: &NetworkInfo) -> Json {
    obj(vec![
        ("net", ji(r.net)),
        ("conn_count", ji(r.conn_count)),
        ("type", ji(r.net_type)),
        ("inv_cap", fb(r.inv_cap)),
        ("extra_cost", fb(r.extra_cost)),
        ("flags", ji(r.flags)),
        ("subnet", jlist(&r.subnet)),
    ])
}

fn expect_cell(w: &World, op: &Json) -> R<()> {
    let m = w.model()?;
    let cells = match opt(op, "cells") {
        Some(c) => cell_list(c, "cells")?,
        None => vec![pair(scalar::opt(op, "cell").ok_or("missing 'cell'")?, "cell")?],
    };
    let want = scalar::opt(op, "expect").ok_or("missing 'expect'")?;
    let s = m.state();
    for (x, z) in cells {
        let c = s
            .conn_at(x, z)
            .ok_or_else(|| format!("({x},{z}) is outside the cell array"))?;
        let got = obj(vec![
            ("conn", jlist(&c.conn)),
            ("switch", jlist(&c.switch)),
            ("prior_car", jlist(&c.prior_car)),
            ("wormhole", ji(c.wormhole)),
            ("switch_id", ji(s.switch_id_at(x, z))),
            ("switch_mode", ji(s.switch_mode_at(x, z))),
            (
                "records",
                Json::Array(s.info_at(x, z).iter().map(record_json).collect()),
            ),
            ("car", s.car_info(x, z).map_or(Json::Null, record_json)),
            ("water_depth", ji(s.water.depth_at(x, z))),
        ]);
        check_fields(&got, want, "cell").map_err(|e| format!("({x},{z}): {e}"))?;
    }
    Ok(())
}

fn expect_state(w: &World, op: &Json) -> R<()> {
    let m = w.model()?;
    let s = m.state();
    let r = s.intersection_rect;
    let mut keys: Vec<u32> = s.edge_density.keys();
    keys.sort_unstable();
    let mut switch_ids: Vec<u32> = s.switches.keys();
    switch_ids.sort_unstable();
    let info_json = |v: &[fc4_core::traffic_network::SubnetworkInfo]| {
        Json::Array(
            v.iter()
                .map(|i| {
                    obj(vec![
                        (
                            "population",
                            Json::Object(i.population.iter().map(|(k, v)| (k.to_string(), ji(*v))).collect()),
                        ),
                        ("capacity", jlist(&i.capacity)),
                    ])
                })
                .collect(),
        )
    };
    let got = obj(vec![
        ("count", jcell((s.count_x, s.count_z))),
        ("subnet_counter", ji(s.subnet_counter)),
        ("working_len", ji(s.subnet_working.len() as i64)),
        ("published_len", ji(s.subnet_published.len() as i64)),
        ("published", info_json(&s.subnet_published)),
        ("intersection_dirty", Json::Bool(s.intersection_dirty)),
        (
            "intersection_rect",
            Json::Array(vec![ji(r.x1), ji(r.z1), ji(r.x2), ji(r.z2)]),
        ),
        ("subnetworks_stale", Json::Bool(s.subnetworks_stale)),
        ("network_changed", Json::Bool(s.network_changed)),
        ("occupants_changed", Json::Bool(s.occupants_changed)),
        ("next_switch_id", ji(s.next_switch_id)),
        ("switch_ids", jlist(&switch_ids)),
        ("ferry_counts", jlist(&s.ferry_counts)),
        ("terminal_count", ji(s.ferries.len() as i64)),
        ("ferry_pass_running", Json::Bool(s.ferry_pass.running)),
        ("ferry_pass_terminal", ji(s.ferry_pass.terminal as i64)),
        ("water_dirty", Json::Bool(s.water.dirty)),
        (
            "water_dirty_rect",
            Json::Array(vec![
                ji(s.water.dirty_rect.x1),
                ji(s.water.dirty_rect.z1),
                ji(s.water.dirty_rect.x2),
                ji(s.water.dirty_rect.z2),
            ]),
        ),
        ("edge_density_keys", jlist(&keys)),
        (
            "inv_cap_table",
            Json::Array(s.tuning.inv_cap_table.iter().map(|&v| fb(v)).collect()),
        ),
        (
            "intersection_values",
            Json::Array(
                s.tuning.intersection_values[..usize::from(s.tuning.intersection_count)]
                    .iter()
                    .map(|&v| fb(v))
                    .collect(),
            ),
        ),
        ("damaged_road_cost", fb(s.tuning.damaged_road_cost)),
        ("posted", jlist(&w.posted.borrow())),
        (
            "refs",
            Json::Object(w.refs.borrow().iter().map(|(k, v)| (k.to_string(), ji(*v))).collect()),
        ),
    ]);
    check_fields(&got, scalar::opt(op, "state").ok_or("missing 'state'")?, "state")
}

fn expect_tunnel(w: &World, op: &Json) -> R<()> {
    let (x, z) = pair(scalar::opt(op, "cell").ok_or("missing 'cell'")?, "cell")?;
    let got = match w.model()?.tunnel_at(x, z) {
        None => Json::Null,
        Some(t) => obj(vec![
            ("other", jcell(t.other)),
            ("mask", ji(t.mask)),
            ("type", ji(t.net_type)),
            (
                "cost",
                Json::Object(
                    t.cost
                        .iter()
                        .enumerate()
                        .map(|(i, &c)| (i.to_string(), fb(c)))
                        .collect(),
                ),
            ),
        ]),
    };
    expect_maybe(&got, op.get("record").ok_or("missing 'record'")?, "tunnel")
}

/// `null` expects nothing; an object is compared field by field.
fn expect_maybe(got: &Json, want: &Json, what: &str) -> R<()> {
    match (got, want) {
        (Json::Null, Json::Null) => Ok(()),
        (Json::Null, _) => Err(format!("no {what}, expected {}", show(want))),
        (_, Json::Null) => Err(format!("{what} {}, expected none", show(got))),
        _ => check_fields(got, want, what),
    }
}

fn expect_lot(w: &World, op: &Json) -> R<()> {
    let lot = id_of(op, "lot")?;
    let got = match w.model()?.lot_subnetworks(lot) {
        None => Json::Null,
        Some(ids) => jlist(&ids),
    };
    let want = op.get("subnetworks").ok_or("missing 'subnetworks'")?;
    if matches(&got, want, false) {
        Ok(())
    } else {
        Err(format!(
            "lot {lot}: subnetworks {}, expected {}",
            show(&got),
            show(want)
        ))
    }
}

fn expect_switch(w: &World, op: &Json) -> R<()> {
    let id = u32_of(scalar::opt(op, "id").ok_or("missing 'id'")?, "id")?;
    let m = w.model()?;
    let s = m.state();
    let got = match s.switches.get(id) {
        None => Json::Null,
        Some(sw) => obj(vec![
            ("building", ji(sw.building as i64)),
            ("load", fb(sw.load)),
            ("time_factor", fb(sw.time_factor)),
            ("capacity", fb(sw.capacity)),
            ("flags", ji(sw.flags)),
            ("to_mask", ji(sw.to_mask)),
            ("mode_mask", ji(sw.mode_mask())),
            ("has_access_point", Json::Bool(sw.has_access_point)),
            ("access", jcell((i32::from(sw.access_x), i32::from(sw.access_z)))),
            ("simulator_value", ji(sw.simulator_value)),
            ("counts_as_intersection", Json::Bool(sw.counts_as_intersection)),
            ("rules", Json::Array(sw.rules.iter().map(rule_json).collect())),
        ]),
    };
    expect_maybe(&got, op.get("switch").ok_or("missing 'switch'")?, "switch")
}

fn expect_ferry(w: &World, op: &Json) -> R<()> {
    let i = u32_of(scalar::opt(op, "index").ok_or("missing 'index'")?, "index")? as usize;
    let m = w.model()?;
    let s = m.state();
    let got = match s.ferries.get(i) {
        None => Json::Null,
        Some(t) => {
            let c = |x: u16, z: u16| jcell((i32::from(x), i32::from(z)));
            obj(vec![
                ("departure", c(t.dep_x, t.dep_z)),
                ("arrival", c(t.arr_x, t.arr_z)),
                ("water", c(t.water_x, t.water_z)),
                ("lot", c(t.lot_x, t.lot_z)),
                ("type", ji(t.ferry_type)),
                ("mask", ji(t.mask)),
                ("reroute", Json::Bool(t.reroute)),
                (
                    "links",
                    Json::Array(
                        t.links
                            .iter()
                            .map(|l| {
                                obj(vec![
                                    ("to_key", ji(l.to_key)),
                                    ("arrival", c(l.arrival_x, l.arrival_z)),
                                    ("cost", fb(l.cost)),
                                    ("route", hex(&l.route)),
                                    ("counters", jlist(&l.counters)),
                                ])
                            })
                            .collect(),
                    ),
                ),
                (
                    "neighbour_links",
                    Json::Array(
                        t.neighbour_links
                            .iter()
                            .map(|l| {
                                obj(vec![
                                    ("off", jcell((i32::from(l.off_x), i32::from(l.off_z)))),
                                    ("edge", c(l.edge_x, l.edge_z)),
                                    ("cost", fb(l.cost)),
                                    ("route_to", hex(&l.route_to)),
                                    ("route_from", hex(&l.route_from)),
                                    ("counters", jlist(&l.counters)),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ])
        }
    };
    expect_maybe(&got, op.get("terminal").ok_or("missing 'terminal'")?, "terminal")
}

/// §11 test 12: the generator's next `RandomUint32Uniform(100)` equals that
/// of a fresh generator with the same seed.
fn expect_rng(w: &World, op: &Json) -> R<()> {
    let mut slot = w.rng.borrow_mut();
    let (seed, g) = slot.as_mut().ok_or("no generator (tn_rng first)")?;
    let mut fresh = RzRandom::new(*seed, &SEEDS);
    let (got, want) = (g.uniform_u32(100), fresh.uniform_u32(100));
    if got != want {
        return Err(format!("next U(100) = {got}, a fresh generator gives {want}"));
    }
    check_expect(op, ji(got))
}
