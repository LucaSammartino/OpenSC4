// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests for the traffic network model. Expected values are the ones
//! `docs/specs/traffic-network.md` states (section and §11 test numbers in
//! the comments), not values read back from the code.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use super::*;
use crate::curve::ResponseCurve;
use crate::grid::{CityDims, CoreGrids, GridProvider, TractGrid};
use crate::rng::{FixedSeeds, RzRandom};
use crate::variant::{mem, Elem, Variant};

// ---------------------------------------------------------------------------
// The stub host
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Kind {
    #[default]
    Network,
    Subway,
    Prop,
    /// A building: answers no occupant-rectangle query.
    Building,
}

/// A scripted occupant. Network occupants answer `PathConnection` only for
/// the listed (path type, entry, exit) triples and the turn-around test only
/// for the listed (path type, entry) pairs (§11 preamble).
#[derive(Clone, Debug, Default)]
struct Occ {
    kind: Kind,
    pos: [f32; 3],
    types: Vec<u32>,
    conns: Vec<(u32, u32, u32)>,
    turns: Vec<(u32, u32)>,
    flags: u32,
    tunnel: bool,
    paired: Option<u64>,
    props: HashMap<u32, Variant>,
    prop_type: u32,
    /// Lane continuity: car paths `(entry, exit byte, first vertex)`.
    paths: Vec<(u32, u8, u64)>,
    /// Lane continuity: `(vertex, exit) → entries` near the vertex.
    near: Vec<(u64, u32, Vec<u32>)>,
    /// Removed from the occupant manager (still readable, as a removed
    /// occupant is while its message is handled).
    gone: bool,
}

#[derive(Clone, Debug)]
struct Lot {
    rect: CellRect,
    facing: u32,
    building: Option<u64>,
    holder: u64,
    pops: Vec<Vec<(u32, u32)>>,
}

/// A neighbour connection: (token, connection type, cell).
type Connection = (u64, u32, (i32, i32));

#[derive(Default)]
struct World {
    cells: Cell<(i32, i32)>,
    occs: RefCell<Vec<(u64, Occ)>>,
    lots: RefCell<Vec<(u64, Lot)>>,
    holders: RefCell<HashMap<u64, BTreeMap<u32, Variant>>>,
    trip_caps: RefCell<HashMap<u64, [u32; 4]>>,
    /// Navigable cells: (depth, body of water).
    water: RefCell<HashMap<(i32, i32), (f32, u16)>>,
    under_water: RefCell<HashSet<(i32, i32)>>,
    damaged: RefCell<HashSet<(i32, i32)>>,
    /// Neighbour connections: (token, type, cell).
    connections: RefCell<Vec<Connection>>,
    pools: RefCell<HashMap<(i32, i32), u64>>,
    posted: RefCell<Vec<u32>>,
    refs: RefCell<HashMap<u64, i32>>,
}

/// The world position of a cell's centre (16 m cells).
fn centre(x: i32, z: i32) -> [f32; 3] {
    [x as f32 * 16.0 + 8.0, 0.0, z as f32 * 16.0 + 8.0]
}

fn inside(r: CellRect, (x, z): (i32, i32)) -> bool {
    x >= r.x1 && x <= r.x2 && z >= r.z1 && z <= r.z2
}

impl World {
    fn new(x: i32, z: i32) -> Rc<World> {
        let w = World::default();
        w.cells.set((x, z));
        Rc::new(w)
    }

    fn add(&self, id: u64, occ: Occ) {
        self.occs.borrow_mut().push((id, occ));
    }

    fn occ(&self, id: u64) -> Occ {
        self.occs
            .borrow()
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, o)| o.clone())
            .unwrap_or_default()
    }

    /// A network piece of the given types at a cell, with car connections
    /// (entry, exit) and car turn-arounds at the given entries.
    fn piece(&self, id: u64, x: i32, z: i32, types: &[u32], conns: &[(u32, u32)], turns: &[u32]) {
        self.add(
            id,
            Occ {
                pos: centre(x, z),
                types: types.to_vec(),
                conns: conns.iter().map(|&(e, x)| (PATH_TYPE_CAR, e, x)).collect(),
                turns: turns.iter().map(|&e| (PATH_TYPE_CAR, e)).collect(),
                ..Occ::default()
            },
        );
    }

    fn add_lot(&self, id: u64, lot: Lot) {
        self.holders.borrow_mut().entry(lot.holder).or_default();
        self.lots.borrow_mut().push((id, lot));
    }

    fn lot(&self, id: u64) -> Option<Lot> {
        self.lots
            .borrow()
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, l)| l.clone())
    }

    fn holder_prop(&self, holder: u64, id: u32) -> Option<Variant> {
        self.holders.borrow().get(&holder).and_then(|m| m.get(&id).cloned())
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
        let p = self.occ(occ).pos;
        let (x, z) = self.position_to_cell(p[0], p[2]);
        CellRect::new(x, z, x, z)
    }
    fn prop_type(&self, occ: u64) -> u32 {
        self.occ(occ).prop_type
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        self.occ(occ).props.get(&id).cloned()
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
    fn path_info_entries_near(&self, pi: PathInfo, path_type: u32, vertex: u64, exit: u32, radius: f32) -> Vec<u32> {
        assert_eq!(path_type, PATH_TYPE_CAR, "§4.4: path type 1");
        assert_eq!(radius, 4.0, "§4.4: radius 4.0");
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
        self.holder_prop(holder, id)
    }
    fn set_holder_property(&self, holder: u64, id: u32, value: Variant) {
        self.holders.borrow_mut().entry(holder).or_default().insert(id, value);
    }
    fn remove_holder_property(&self, holder: u64, id: u32) -> bool {
        self.holders
            .borrow_mut()
            .get_mut(&holder)
            .is_some_and(|m| m.remove(&id).is_some())
    }
    fn max_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        self.trip_caps.borrow().get(&holder).map_or(0, |c| c[k as usize])
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

/// A model over the world with shipped tuning, after `Init` (§11 preamble).
fn model(w: &Rc<World>) -> Rc<NetworkModel> {
    let m = NetworkModel::new(w.clone() as Rc<dyn NetworkHost>);
    m.set_tuning(Tuning::shipped());
    m.init();
    m
}

fn conn(m: &NetworkModel, x: i32, z: i32) -> CellConnection {
    m.cell_connection(x, z).expect("in-city cell")
}

fn car(m: &NetworkModel, x: i32, z: i32) -> NetworkInfo {
    m.network_info(net::CAR, x, z).expect("car record")
}

fn bits(v: f32) -> u32 {
    v.to_bits()
}

const STREET: u32 = net_type::STREET;
const A: (i32, i32) = (1, 1);
const B: (i32, i32) = (2, 1);
const C: (i32, i32) = (3, 1);

/// §11 test 1's world: Street A, B, C inserted in that order, then
/// DoPostCityInit.
fn three_cell_street() -> (Rc<World>, Rc<NetworkModel>) {
    let w = World::new(4, 4);
    let m = model(&w);
    w.piece(1, A.0, A.1, &[STREET], &[], &[2]);
    w.piece(2, B.0, B.1, &[STREET], &[(0, 2), (2, 0)], &[]);
    w.piece(3, C.0, C.1, &[STREET], &[], &[0]);
    for occ in 1..=3 {
        m.occupant_changed(occ, false);
    }
    m.do_post_city_init();
    (w, m)
}

// ---------------------------------------------------------------------------
// Constants (§4.1, §4.7.3)
// ---------------------------------------------------------------------------

#[test]
fn switch_cost_blocked_bit_pattern() {
    // §4.7.3: 1.7014117e38 = 0x7EFFFFFF.
    assert_eq!(SWITCH_COST_BLOCKED.to_bits(), 0x7EFF_FFFF);
}

#[test]
fn side_of_rule() {
    // §4.1 SideOf(a → b): the side of b that faces a.
    assert_eq!(side_of((0, 0), (1, 0)), 0);
    assert_eq!(side_of((2, 0), (1, 0)), 2);
    assert_eq!(side_of((1, 1), (1, 0)), 3);
    assert_eq!(side_of((1, 0), (1, 1)), 1);
    assert_eq!(side_of((1, 1), (1, 1)), 3, "equal cells give 3");
}

#[test]
fn constant_tables() {
    // §4.1.
    assert_eq!(NET_OF_TRAVEL, [0, 1, 1, 2, 1, 2, 3, 4, 5]);
    assert_eq!(PATH_TYPE_OF_NET, [2, 1, 3, 4, 6, 7]);
    assert_eq!(REP_OF_NET, [0, 1, 3, 6, 7, 8]);
    assert_eq!(OPPOSITE_SIDE, [2, 3, 0, 1]);
    assert_eq!(ROT, [[2, 3, 0, 1], [1, 2, 3, 0], [0, 1, 2, 3], [3, 0, 1, 2]]);
    assert_eq!(FERRY_MASK, [1, 3]);
    assert_eq!(FERRY_MASK_TYPE2, 0xD70A);
    assert_eq!(TRANSIT_MASK, [3, 4, 2, 3, 0, 0, 3, 8, 0x10, 0x20, 3, 3, 2]);
    let takes: Vec<bool> = (0..13).map(|k| k != 4 && k != 5).collect();
    assert_eq!(TAKES_PART.to_vec(), takes);
}

#[test]
fn boundary_iterator_order() {
    // BoundaryIterator (zones.md slot 10, pathfinder.md §4.6): the first row
    // all x, the middle rows x1 then x2, the last row all x.
    let r = CellRect::new(0, 1, 2, 3);
    assert_eq!(
        boundary_cells(r),
        vec![(0, 1), (1, 1), (2, 1), (0, 2), (2, 2), (0, 3), (1, 3), (2, 3)]
    );
}

// ---------------------------------------------------------------------------
// §11 test 1: the three-cell street
// ---------------------------------------------------------------------------

#[test]
fn t01_three_cell_street() {
    let (_w, m) = three_cell_street();
    let expect = [(A, 0x0400u16, 1u8), (B, 0x0104, 2), (C, 0x0001, 1)];
    for &((x, z), word, count) in &expect {
        let c = conn(&m, x, z);
        for t in [1, 2, 4] {
            assert_eq!(c.conn[t], word, "conn[{t}] at ({x},{z})");
        }
        let info = car(&m, x, z);
        assert_eq!(info.conn_count, count);
        assert_eq!(u32::from(info.net_type), STREET);
        assert_eq!(bits(info.inv_cap), 0x3C23_D70A);
        assert_eq!(info.extra_cost, 0.0);
    }
    for x in 0..4 {
        for z in 0..4 {
            let c = conn(&m, x, z);
            let listed = expect.iter().any(|e| e.0 == (x, z));
            for t in 0..travel::COUNT {
                if !(listed && [1, 2, 4].contains(&t)) {
                    assert_eq!(c.conn[t], 0, "conn[{t}] at ({x},{z})");
                }
                assert_eq!(c.switch[t], 0xFFFF);
            }
            assert_eq!(c.prior_car, [0xFFFF; 4], "priorCar at ({x},{z})");
            // No records for networks other than 1.
            for n in 0..net::COUNT as u32 {
                if n != net::CAR {
                    assert!(m.network_info(n, x, z).is_none());
                }
            }
            if !listed {
                assert!(m.network_info(net::CAR, x, z).is_none());
            }
        }
    }
    // The intersection pass leaves invCap unchanged.
    m.update_intersection_data();
    for &((x, z), _, _) in &expect {
        assert_eq!(bits(car(&m, x, z).inv_cap), 0x3C23_D70A);
    }
    assert_eq!(bits(m.get_capacity(1, 2, 1)), 0x42C8_0000);
    assert_eq!(m.get_connection_count(1, 2, 1), 2);
    assert_eq!(m.get_capacity(0, 2, 1), 0.0);
}

// ---------------------------------------------------------------------------
// §11 test 2: subnetworks of test 1
// ---------------------------------------------------------------------------

#[test]
fn t02_subnetworks() {
    let (_w, m) = three_cell_street();
    assert_eq!(car(&m, A.0, A.1).subnet, [0, 0, 1, 0]);
    assert_eq!(car(&m, B.0, B.1).subnet, [1, 0, 1, 0]);
    assert_eq!(car(&m, C.0, C.1).subnet, [1, 0, 0, 0]);
    let s = m.state();
    assert_eq!(s.subnet_counter, 2);
    assert_eq!(s.subnet_working.len(), 2);
    assert_eq!(s.subnet_published.len(), 2);
}

#[test]
fn t02_seed_at_a_entry_0_labels_nothing() {
    let (_w, m) = three_cell_street();
    m.update_state(|s| {
        s.clear_subnetworks();
        s.fill(A.0, A.1, 0);
        assert_eq!(s.subnet_counter, 1, "the counter does not advance");
        for (x, z) in [A, B, C] {
            assert_eq!(s.car_info(x, z).unwrap().subnet, [0; 4]);
        }
    });
}

#[test]
fn t02_last_column_is_never_a_seed() {
    // C alone: x = 3 = X−1 is never a seed, so it stays unlabelled; a
    // direct Fill from it would label it.
    let w = World::new(4, 4);
    let m = model(&w);
    w.piece(3, C.0, C.1, &[STREET], &[], &[0]);
    m.do_post_city_init();
    assert_eq!(car(&m, C.0, C.1).subnet, [0; 4]);
    assert_eq!(m.state().subnet_counter, 1);
    m.update_state(|s| s.fill(C.0, C.1, 0));
    assert_eq!(car(&m, C.0, C.1).subnet, [1, 0, 0, 0]);
}

// ---------------------------------------------------------------------------
// §11 test 3: lot lists and queries
// ---------------------------------------------------------------------------

const L1: u64 = 101;
const L2: u64 = 102;
const L3: u64 = 103;
const DEV: u32 = 0x1010;

fn lot_world() -> (Rc<World>, Rc<NetworkModel>) {
    let w = World::new(4, 4);
    let m = model(&w);
    w.piece(1, A.0, A.1, &[STREET], &[], &[2]);
    w.piece(2, B.0, B.1, &[STREET], &[(0, 2), (2, 0)], &[]);
    w.piece(3, C.0, C.1, &[STREET], &[], &[0]);
    let lot = |x, z, holder, pops| Lot {
        rect: CellRect::new(x, z, x, z),
        facing: 0,
        building: None,
        holder,
        pops,
    };
    w.add_lot(L1, lot(1, 2, 1001, vec![vec![(DEV, 3)]]));
    w.add_lot(L2, lot(2, 0, 1002, Vec::new()));
    w.add_lot(L3, lot(0, 3, 1003, Vec::new()));
    w.trip_caps.borrow_mut().insert(1001, [5, 0, 0, 0]);
    w.trip_caps.borrow_mut().insert(1002, [0, 7, 0, 0]);
    // A stale list on the third lot, which the pass must remove.
    w.set_holder_property(1003, prop::LOT_SUBNETWORKS, Variant::Scalar(Elem::U32(9)));
    for occ in 1..=3 {
        m.occupant_changed(occ, false);
    }
    m.do_post_city_init();
    (w, m)
}

#[test]
fn t03_lot_ring_and_lists() {
    let (w, m) = lot_world();
    // L1's ring, corners excluded: (1,1), (1,3), (0,2), (2,2) as a set.
    let ring = m.state().grow_clamp(CellRect::new(1, 2, 1, 2), 1);
    let mut cells: Vec<(i32, i32)> = boundary_cells(ring)
        .into_iter()
        .filter(|&(x, z)| x == 1 || z == 2)
        .collect();
    cells.sort_unstable();
    assert_eq!(cells, vec![(0, 2), (1, 1), (1, 3), (2, 2)]);
    assert_eq!(m.lot_subnetworks(L1), Some(vec![1]));
    assert_eq!(m.lot_subnetworks(L2), Some(vec![1]));
    // The list is a u32 array property (§4.12 step 3).
    assert_eq!(
        w.holder_prop(1001, prop::LOT_SUBNETWORKS),
        Some(Variant::array(mem::UINT32, vec![Elem::U32(1)]))
    );
    assert_eq!(m.get_subnetworks_for_lot(L1), (vec![1], true));
}

#[test]
fn t03_queries() {
    let (_w, m) = lot_world();
    assert!(m.are_lots_connected(L1, L2));
    assert_eq!(m.get_connected_destination_count(L1, 1), 7);
    assert_eq!(m.get_connected_destination_count(L1, 0), 5);
    assert_eq!(m.get_connected_destination_count(L1, 4), 0);
    assert_eq!(m.get_connected_occupant_count(L2, DEV), 3);
    assert_eq!(m.get_occupant_count_for_all_subnetworks(DEV), vec![0, 3]);
    assert_eq!(m.get_subnetworks_in_rectangle(CellRect::new(0, 0, 3, 3)), vec![1]);
}

#[test]
fn t03_lot_without_list() {
    let (w, m) = lot_world();
    assert_eq!(m.lot_subnetworks(L3), None);
    assert_eq!(m.get_subnetworks_for_lot(L3), (Vec::new(), false));
    assert!(!m.are_lots_connected(L1, L3));
    assert!(
        w.holder_prop(1003, prop::LOT_SUBNETWORKS).is_none(),
        "0x2A4F9C74 removed"
    );
}

#[test]
fn t03_region_query() {
    // §4.13 slot 41: the in-city cells of the region whose bit is set.
    let (_w, m) = lot_world();
    let none = CellRegion::from_cells(CellRect::new(0, 0, 3, 3), &[(0, 0), (3, 3)]);
    assert!(m.get_subnetworks_in_region(&none).is_empty());
    let some = CellRegion::from_cells(CellRect::new(0, 0, 3, 3), &[(2, 1)]);
    assert_eq!(m.get_subnetworks_in_region(&some), vec![1]);
}

// ---------------------------------------------------------------------------
// §11 test 4: intersection reduction
// ---------------------------------------------------------------------------

/// A cross centred at (2, 2) of the given network type: straight arm pieces,
/// and a centre with all 12 (e → x ≠ e) connections.
fn cross(ty: u32) -> Rc<NetworkModel> {
    let w = World::new(4, 4);
    let m = model(&w);
    let all: Vec<(u32, u32)> = (0..4)
        .flat_map(|e| (0..4).filter(move |&x| x != e).map(move |x| (e, x)))
        .collect();
    w.piece(1, 2, 2, &[ty], &all, &[]);
    let mut id = 2;
    for x in [0, 1, 3] {
        w.piece(id, x, 2, &[ty], &[(0, 2), (2, 0)], &[]);
        id += 1;
    }
    for z in [0, 1, 3] {
        w.piece(id, 2, z, &[ty], &[(1, 3), (3, 1)], &[]);
        id += 1;
    }
    m.do_post_city_init();
    assert_eq!(car(&m, 2, 2).conn_count, 12);
    m.update_intersection_data();
    m
}

#[test]
fn t04_street_cross() {
    let m = cross(STREET);
    assert_eq!(bits(car(&m, 2, 2).inv_cap), 0x3C6A_0EA1);
    assert_eq!(m.get_capacity(1, 2, 2), 70.0);
    for (x, z) in [(1, 2), (3, 2), (2, 1), (2, 3)] {
        assert_eq!(bits(car(&m, x, z).inv_cap), 0x3C4C_CCCC, "({x},{z})");
        assert_eq!(m.get_capacity(1, x, z), 80.000_01);
    }
    for (x, z) in [(0, 2), (2, 0)] {
        assert_eq!(bits(car(&m, x, z).inv_cap), 0x3C36_0B61, "({x},{z})");
        assert_eq!(m.get_capacity(1, x, z), 90.0);
    }
    assert!(!m.state().intersection_dirty, "the pass clears the flag");
}

#[test]
fn t04_road_cross() {
    let m = cross(net_type::ROAD);
    assert_eq!(bits(car(&m, 2, 2).inv_cap), 0x3ABB_3EE8);
    for (x, z) in [(1, 2), (3, 2), (2, 1), (2, 3)] {
        assert_eq!(bits(car(&m, x, z).inv_cap), 0x3AA3_D70B);
    }
    for (x, z) in [(0, 2), (2, 0)] {
        assert_eq!(bits(car(&m, x, z).inv_cap), 0x3A91_A2B5);
    }
}

#[test]
fn t04_highway_is_never_reduced() {
    let m = cross(net_type::HIGHWAY);
    for (x, z) in [(2, 2), (1, 2), (0, 2), (2, 0)] {
        // invCapTable[Highway] for 4000 (§4.5 table).
        assert_eq!(bits(car(&m, x, z).inv_cap), 0x3983_126F);
    }
}

// ---------------------------------------------------------------------------
// §11 test 5: the capacity table
// ---------------------------------------------------------------------------

#[test]
fn t05_capacity_table() {
    // §4.5: shipped capacities 1000, 3000, 4000, 100, 0, 0, 2500, 3000,
    // 3000, 3000, 2000, 0, 4000 and their invCapTable bit patterns.
    let expect: [u32; 13] = [
        0x3A83_126F,
        0x39AE_C33E,
        0x3983_126F,
        0x3C23_D70A,
        0x3F80_0000,
        0x3F80_0000,
        0x39D1_B717,
        0x39AE_C33E,
        0x39AE_C33E,
        0x39AE_C33E,
        0x3A03_126F,
        0x3F80_0000,
        0x3983_126F,
    ];
    let t = Tuning::shipped();
    let got: Vec<u32> = t.inv_cap_table.iter().map(|v| v.to_bits()).collect();
    assert_eq!(got, expect.to_vec());

    let w = World::new(4, 4);
    let m = model(&w);
    let straight = [(0, 2), (2, 0)];
    w.piece(1, 0, 0, &[net_type::ROAD], &straight, &[]);
    w.piece(2, 2, 0, &[net_type::HIGHWAY], &straight, &[]);
    w.piece(3, 0, 2, &[net_type::ONE_WAY_ROAD], &straight, &[]);
    w.piece(4, 2, 2, &[net_type::DIRT_ROAD], &straight, &[]);
    m.do_post_city_init();
    assert_eq!(bits(m.get_capacity(1, 0, 0)), 0x4479_FFFF); // 999.99994
    assert_eq!(bits(m.get_capacity(1, 2, 0)), 0x4579_FFFF); // 3999.9998
    assert_eq!(bits(m.get_capacity(1, 0, 2)), 0x44F9_FFFF); // 1999.9999
    assert_eq!(car(&m, 2, 2).inv_cap, 1.0, "dirt: capacity 0 → invCap 1.0");
    assert_eq!(m.get_capacity(1, 2, 2), 1.0);
}

#[test]
fn t05_network_type_choice() {
    // §4.2 step 3: Street only while still "none"; Road does not replace
    // Highway; otherwise the last match wins.
    let w = World::new(4, 4);
    let m = model(&w);
    let straight = [(0, 2), (2, 0)];
    w.piece(1, 0, 0, &[net_type::ROAD, net_type::STREET], &straight, &[]);
    w.piece(2, 2, 0, &[net_type::HIGHWAY, net_type::ROAD], &straight, &[]);
    w.piece(3, 0, 2, &[net_type::ROAD, net_type::AVENUE], &straight, &[]);
    m.do_post_city_init();
    assert_eq!(u32::from(car(&m, 0, 0).net_type), net_type::ROAD);
    assert_eq!(u32::from(car(&m, 2, 0).net_type), net_type::HIGHWAY);
    assert_eq!(u32::from(car(&m, 0, 2).net_type), net_type::AVENUE);
}

// ---------------------------------------------------------------------------
// §11 test 6: turn-around via SetConnection
// ---------------------------------------------------------------------------

#[test]
fn t06_set_connection_turn_around() {
    let w = World::new(4, 4);
    let m = model(&w);
    // Clear every priorCar word so the ORs are visible.
    m.update_state(|s| {
        for x in 0..4 {
            for z in 0..4 {
                s.conn_mut(x, z).unwrap().prior_car = [0; 4];
            }
        }
    });
    m.set_connection(0, 0, 2, 2, travel::CAR);
    assert_eq!(conn(&m, 0, 0).conn[1], 0x0400);
    assert_eq!(conn(&m, 1, 0).prior_car, [0x4444, 0, 0, 0]);
    m.set_connection(0, 0, 0, 0, travel::CAR);
    assert_eq!(conn(&m, 0, 0).conn[1], 0x0401);
    for x in 0..4 {
        for z in 0..4 {
            let want = if (x, z) == (1, 0) { [0x4444, 0, 0, 0] } else { [0; 4] };
            assert_eq!(conn(&m, x, z).prior_car, want, "({x},{z})");
        }
    }
}

#[test]
fn set_connection_non_car_leaves_prior_car() {
    // §4.3 step 2 applies only when net(t) = 1.
    let w = World::new(4, 4);
    let m = model(&w);
    m.update_state(|s| s.conn_mut(1, 0).unwrap().prior_car = [0; 4]);
    m.set_connection(0, 0, 2, 2, travel::WALK);
    assert_eq!(conn(&m, 0, 0).conn[travel::WALK], 0x0400);
    assert_eq!(conn(&m, 1, 0).prior_car, [0; 4]);
}

// ---------------------------------------------------------------------------
// §11 test 7: tunnel cost
// ---------------------------------------------------------------------------

#[test]
fn t07_tunnel_cost() {
    let w = World::new(4, 4);
    let m = model(&w);
    let portal = |pos: [f32; 3], other: u64| Occ {
        pos,
        types: vec![net_type::ROAD],
        tunnel: true,
        paired: Some(other),
        ..Occ::default()
    };
    w.add(1, portal([8.0, 0.0, 24.0], 2));
    w.add(2, portal([72.0, 0.0, 24.0], 1));
    // Shipped speeds on Road: Car 31 (§4.8 example); Walk 3.5 *[inferred
    // from the stated Walk cost 0x3F8C6F2E]*; PassengerTrain none.
    let mut speeds = [[0.0f32; net_type::COUNT]; travel::COUNT];
    speeds[travel::CAR][net_type::ROAD as usize] = 31.0;
    speeds[travel::WALK][net_type::ROAD as usize] = 3.5;
    let inv = inverse_max_speed(&speeds, 16.0);
    assert_eq!(inv[travel::CAR][0].to_bits(), 0x3CFD_B011, "§4.8 example");
    assert_eq!(inv[travel::PASSENGER_TRAIN][0], f32::MAX, "v ≤ 0 → FLT_MAX");
    m.set_inv_max_speed(inv);
    assert!(m.tunnel_changed(1, false));
    assert!(m.tunnel_changed(2, false));
    for ((x, z), other) in [((0, 1), (4, 1)), ((4, 1), (0, 1))] {
        let t = m.tunnel_at(x, z).expect("record per portal");
        assert_eq!(t.other, other);
        assert_eq!(t.mask, 0x1FE);
        assert_eq!(u32::from(t.net_type), net_type::ROAD);
        assert_eq!(t.cost[travel::CAR].to_bits(), 0x3DFD_B011);
        assert_eq!(t.cost[travel::WALK].to_bits(), 0x3F8C_6F2E);
        assert_eq!(t.cost[travel::PASSENGER_TRAIN], f32::INFINITY);
    }
    // §4.2 step 5: a rebuild marks the in-city portal cell.
    m.do_connections_changed(CellRect::new(0, 0, 3, 3));
    assert_eq!(conn(&m, 0, 1).wormhole & wormhole::TUNNEL, wormhole::TUNNEL);
    // Remove erases the key; the next rebuild clears the bit.
    m.tunnel_changed(1, true);
    assert!(m.tunnel_at(0, 1).is_none());
    m.do_connections_changed(CellRect::new(0, 1, 0, 1));
    assert_eq!(conn(&m, 0, 1).wormhole & wormhole::TUNNEL, 0);
}

#[test]
fn t07_tunnel_type_order() {
    // §4.8 step 2: the first of 2, 6, 0, 1, 10, 12, 9; none → no record.
    let w = World::new(4, 4);
    let m = model(&w);
    let portal = |types: Vec<u32>| Occ {
        pos: centre(1, 1),
        types,
        tunnel: true,
        paired: Some(9),
        ..Occ::default()
    };
    w.add(
        9,
        Occ {
            pos: centre(3, 1),
            ..Occ::default()
        },
    );
    w.add(1, portal(vec![net_type::ROAD, net_type::AVENUE]));
    assert!(m.tunnel_changed(1, false));
    assert_eq!(u32::from(m.tunnel_at(1, 1).unwrap().net_type), net_type::AVENUE);
    w.occs.borrow_mut().retain(|(id, _)| *id != 1);
    w.add(1, portal(vec![net_type::STREET]));
    m.tunnel_changed(1, true);
    assert!(!m.tunnel_changed(1, false));
    assert!(m.tunnel_at(1, 1).is_none());
}

// ---------------------------------------------------------------------------
// §11 test 8: water route
// ---------------------------------------------------------------------------

#[test]
fn t08_depth_bytes() {
    assert_eq!(depth_byte(true, 3.3), 17);
    assert_eq!(depth_byte(true, 40.7), 200);
    assert_eq!(depth_byte(true, 0.1), 1);
    assert_eq!(depth_byte(true, 2.1), 11);
    // §4.10.1: not navigable, w ≤ 0 or NaN → 0.
    assert_eq!(depth_byte(false, 3.3), 0);
    assert_eq!(depth_byte(true, 0.0), 0);
    assert_eq!(depth_byte(true, f32::NAN), 0);
}

#[test]
fn t08_step_costs() {
    // §4.10.2 step 4 table.
    assert_eq!(step_cost(200, 0.0), 1.0);
    assert_eq!(step_cost(100, 0.0), 3.0);
    assert_eq!(step_cost(0, 0.0), 5.0);
}

fn water_world() -> (Rc<World>, Rc<NetworkModel>) {
    let w = World::new(8, 8);
    let m = model(&w);
    for x in 0..8 {
        w.water.borrow_mut().insert((x, 0), (40.7, 1));
    }
    m.do_post_city_init();
    (w, m)
}

#[test]
fn t08_water_route() {
    let (_w, m) = water_world();
    for x in 0..8 {
        assert_eq!(m.state().water.depth_at(x, 0), 200);
        assert_eq!(m.state().water.depth_at(x, 1), 0);
    }
    let p = m.find_water_path(0, 0, 5, 0).expect("a route");
    assert_eq!(p.cells, 6);
    assert_eq!(p.g, 5.0);
    assert_eq!(route_cost(p.cells).to_bits(), 0x3D75_C28F);
    assert_eq!(p.forward, vec![0x00, 0x00, 0x00, 0x40, 0x05, 0xAA, 0x02]);
    assert_eq!(p.backward, vec![0x00, 0x05, 0x00, 0x40, 0x05, 0x00, 0x00]);
    let mut route = Vec::new();
    assert!(m.get_water_route(0, 0, 5, 0, &mut route));
    assert_eq!(route, p.forward);
}

#[test]
fn t08_different_bodies_fail() {
    let (w, m) = water_world();
    w.water.borrow_mut().insert((7, 0), (40.7, 2));
    let mut route = vec![0xEE, 0xEE];
    assert!(!m.get_water_route(0, 0, 7, 0, &mut route));
    assert_eq!(route, vec![0xEE, 0xEE], "the vector is untouched");
    assert!(m.find_water_path(0, 0, 7, 0).is_none());
    // A land cell (body 0) at either end fails too (§4.10.2 step 2).
    assert!(m.find_water_path(0, 0, 0, 3).is_none());
}

#[test]
fn route_encoding_runs() {
    // §4.10.3: a run holds at most 255 steps; the 256th starts a new run.
    let path: Vec<(i32, i32)> = (0..=256).map(|x| (x, 0)).collect();
    let r = encode_route(&path);
    assert_eq!(&r[..5], &[0, 0, 0, 0x40, 255]);
    // 255 codes → 64 bytes, then the second run of one step.
    assert_eq!(r.len(), 3 + 2 + 64 + 2 + 1);
    assert_eq!(&r[3 + 2 + 64..], &[0x40, 1, 0x02]);
}

// ---------------------------------------------------------------------------
// §11 test 9: ferry cost encoding
// ---------------------------------------------------------------------------

#[test]
fn t09_ferry_cost() {
    let got: Vec<u32> = [1, 5, 37, 100, 250].iter().map(|&n| route_cost(n).to_bits()).collect();
    assert_eq!(
        got,
        vec![0x3C23_D70A, 0x3D4C_CCCC, 0x3EBD_70A4, 0x3F80_0000, 0x4020_0000]
    );
}

// ---------------------------------------------------------------------------
// §11 test 10: switch entry cost
// ---------------------------------------------------------------------------

#[test]
fn t10_switch_entry_cost() {
    let w = World::new(4, 4);
    let m = model(&w);
    let curve = ResponseCurve::from_points(&[(0.0, 2.0)]);
    let at0 = curve.eval(0.0);
    m.set_switch_cost_inputs(curve, 1.5);
    let set = |id: u32, load: f32, capacity: f32| {
        m.update_state(|s| {
            let sw = s.switches.entry(id, TransitSwitch::default);
            sw.load = load;
            sw.capacity = capacity;
            sw.time_factor = 0.5;
        })
    };
    set(1, 0.0, 100.0);
    set(2, 600.0, 100.0);
    set(3, 0.0, 0.0);
    // q = 0 → r24(r24(Curve(0) × multiplier) × timeFactor).
    assert_eq!(at0, 2.0);
    assert_eq!(m.transit_switch_entry_cost(1), 1.5);
    assert_eq!(m.transit_switch_entry_cost(2).to_bits(), 0x7EFF_FFFF);
    assert_eq!(m.transit_switch_entry_cost(3).to_bits(), 0x7EFF_FFFF);
    // An unknown ID inserts a default switch (capacity 0).
    assert!(!m.state().switches.contains(77));
    assert_eq!(m.transit_switch_entry_cost(77).to_bits(), 0x7EFF_FFFF);
    let s = m.state();
    let sw = s.switches.get(77).expect("inserted");
    assert_eq!(sw.capacity, 0.0);
}

// ---------------------------------------------------------------------------
// §11 test 11: rule rotation
// ---------------------------------------------------------------------------

#[test]
fn t11_rule_rotation() {
    let rs = read_rules(1, &[0x0101_8000]);
    assert_eq!(
        rs.rules,
        vec![Rule {
            kind: 0,
            side: [false, true, false, false],
            from: 1,
            to: 1,
        }]
    );
    assert_eq!(rs.to_mask, 0x0002);
    let sw = TransitSwitch {
        rules: rs.rules,
        ..TransitSwitch::default()
    };
    assert_eq!(sw.mode_mask(), 0b10);
}

#[test]
fn rule_kind_and_flags() {
    // §4.7.2: b0 = 0x82 → kind 1; §2.4: the flags from the "to" types and
    // 0x40 for a subway ↔ elevated-train rule.
    let rs = read_rules(2, &[0x0302_8082, 0x0607_0000]);
    assert_eq!(rs.rules[0].kind, 1);
    assert_eq!(rs.rules[0].side, [true, false, false, false]);
    assert_eq!(rs.to_mask, 1 << 6, "kind-1 rules add no 'to' bit");
    assert_eq!(rs.flags, 0x02 | 0x04 | 0x40);
}

#[test]
fn u32_entries_of_property() {
    let v = Variant::array(mem::UINT32, vec![Elem::U32(0x0101_8000), Elem::U32(7)]);
    assert_eq!(u32_entries(&v), Some(vec![0x0101_8000, 7]));
    let bytes = Variant::array(mem::UINT8, vec![Elem::U8(1), Elem::U8(2), Elem::U8(3)]);
    assert_eq!(u32_entries(&bytes), None, "a byte size not a multiple of 4");
}

// ---------------------------------------------------------------------------
// §11 test 12: no random draws
// ---------------------------------------------------------------------------

#[test]
fn t12_no_random_draws() {
    let seeds = FixedSeeds {
        wall_clock: 1,
        process_clock: 1,
    };
    let mut rng = RzRandom::new(12345, &seeds);
    // Tests 1–3 and 8, and a DoConnectionsChanged over the whole city. The
    // host interface has no generator, so the model cannot draw (§12).
    let (_w, m) = lot_world();
    m.do_connections_changed(CellRect::new(0, 0, 3, 3));
    let _ = m.get_connected_destination_count(L1, 1);
    let (_w8, m8) = water_world();
    let _ = m8.find_water_path(0, 0, 5, 0);
    let mut fresh = RzRandom::new(12345, &seeds);
    assert_eq!(rng.uniform_u32(100), fresh.uniform_u32(100));
}

// ---------------------------------------------------------------------------
// Other spec rules
// ---------------------------------------------------------------------------

#[test]
fn record_flags_damage_and_exclusion() {
    let w = World::new(4, 4);
    let m = model(&w);
    let straight = [(0, 2), (2, 0)];
    w.piece(1, 0, 0, &[STREET], &straight, &[]);
    w.piece(2, 2, 0, &[STREET], &straight, &[]);
    w.piece(3, 0, 2, &[STREET], &straight, &[]);
    w.piece(4, 2, 2, &[net_type::WATER_PIPE], &straight, &[]);
    w.occs.borrow_mut()[1].1.flags = network_flag::RECORD_FLAG;
    w.occs.borrow_mut()[2].1.flags = network_flag::EXCLUDED;
    w.damaged.borrow_mut().insert((0, 0));
    m.do_post_city_init();
    // §4.2 step 3: extraCost = the damaged-road cost (shipped 0.1, §4.5).
    assert_eq!(car(&m, 0, 0).extra_cost, 0.1);
    assert_eq!(car(&m, 0, 0).flags, 0);
    assert_eq!(car(&m, 2, 0).flags, 1, "network flag 0x40000 → flags bit 0");
    assert!(m.network_info(net::CAR, 0, 2).is_none(), "flag 0x200000 excludes");
    assert!(m.network_info(net::CAR, 2, 2).is_none(), "pipes do not take part");
    assert_eq!(conn(&m, 2, 2).conn, [0; 9]);
}

#[test]
fn append_clears_previous_tail_flags() {
    // §2.2 quirk: appending a record clears the old tail's flags.
    let w = World::new(4, 4);
    let m = model(&w);
    m.update_state(|s| {
        s.append_info(net::CAR, 1, 1).unwrap().flags = 1;
        s.append_info(net::PEDESTRIAN, 1, 1);
        assert_eq!(s.car_info(1, 1).unwrap().flags, 0);
    });
}

#[test]
fn edge_density_entries() {
    // §4.2 step 3: the edge-density entries of every t on n, both evening
    // values; §3: a removal erases the cell's 18 keys.
    let (w, m) = three_cell_street();
    {
        let s = m.state();
        for t in [1u32, 2, 4] {
            for evening in [false, true] {
                assert!(s.edge_density.contains(NetworkState::edge_key(2, 1, t, evening)));
            }
        }
        assert!(!s.edge_density.contains(NetworkState::edge_key(2, 1, 0, false)));
    }
    assert_eq!(NetworkState::edge_key(2, 1, 4, true), 0x0104_0201);
    w.occs.borrow_mut()[1].1.gone = true;
    m.occupant_changed(2, true);
    let s = m.state();
    for t in 0..9u32 {
        for evening in [false, true] {
            assert!(!s.edge_density.contains(NetworkState::edge_key(2, 1, t, evening)));
        }
    }
    assert!(s.network_info(net::CAR, 2, 1).is_none());
    assert!(s.occupants_changed);
}

#[test]
fn lane_continuity_last_flagged_occupant_decides() {
    // §4.4: car connection (0 → 2) at (1, 1) whose path starts at vertex 50;
    // the flagged occupants of P = (0, 1) answer entries near vertex 50
    // through exit (0 + 2) mod 4 = 2; the last one writes nibble 2.
    let w = World::new(4, 4);
    let m = model(&w);
    w.add(
        1,
        Occ {
            pos: centre(1, 1),
            types: vec![STREET],
            conns: vec![(1, 0, 2)],
            paths: vec![(0, 2, 50)],
            ..Occ::default()
        },
    );
    let neighbour = |entries: Vec<u32>| Occ {
        pos: centre(0, 1),
        types: vec![net_type::HIGHWAY],
        flags: 0x0000_0040,
        near: vec![(50, 2, entries)],
        ..Occ::default()
    };
    w.add(2, neighbour(vec![0, 3]));
    w.add(3, neighbour(vec![1]));
    m.do_connections_changed(CellRect::new(1, 1, 1, 1));
    // priorCar[0] nibble 2 := 1 << 1; the other nibbles keep 0xF.
    assert_eq!(conn(&m, 1, 1).prior_car[0], 0xF2FF);
    assert_eq!(conn(&m, 1, 1).prior_car[1..], [0xFFFF; 3]);
}

#[test]
fn lane_continuity_without_flagged_neighbour() {
    // With no flagged occupant in P the word keeps 0xFFFF (§4.4).
    let w = World::new(4, 4);
    let m = model(&w);
    w.add(
        1,
        Occ {
            pos: centre(1, 1),
            types: vec![STREET],
            conns: vec![(1, 0, 2)],
            paths: vec![(0, 2, 50)],
            ..Occ::default()
        },
    );
    w.add(
        2,
        Occ {
            pos: centre(0, 1),
            types: vec![STREET],
            near: vec![(50, 2, vec![1])],
            ..Occ::default()
        },
    );
    m.do_connections_changed(CellRect::new(1, 1, 1, 1));
    assert_eq!(conn(&m, 1, 1).prior_car, [0xFFFF; 4]);
}

#[test]
fn neighbour_inner_cell_and_map_exit() {
    // §2.7.
    let (_w, m) = three_cell_street();
    assert_eq!(m.neighbor_connection_inner_cell(-1, 2), Some(((0, 2), 0)));
    assert_eq!(m.neighbor_connection_inner_cell(4, 2), Some(((3, 2), 2)));
    assert_eq!(m.neighbor_connection_inner_cell(2, -1), Some(((2, 0), 1)));
    assert_eq!(m.neighbor_connection_inner_cell(2, 4), Some(((2, 3), 3)));
    assert_eq!(m.neighbor_connection_inner_cell(-1, -1), None);
    assert_eq!(m.neighbor_connection_inner_cell(1, 1), None);
    // C = (3, 1) exits only west; nothing leaves east at (4, 1).
    assert!(!m.has_map_exit(4, 1));
    m.set_connection(3, 1, 0, 2, travel::CAR);
    assert!(m.has_map_exit(4, 1));
    // The edge record's mask bit 9.
    assert!(!m.has_map_exit(-1, 3));
    m.set_neighbour_edge(
        -1,
        3,
        NeighbourEdge {
            mask: 1 << 9,
            pool: 0,
            cost: 0.0,
        },
    );
    assert!(m.has_map_exit(-1, 3));
}

#[test]
fn capacity_of_missing_record_and_counts() {
    // §4.5: 0.0 / 0 without a record.
    let w = World::new(4, 4);
    let m = model(&w);
    assert_eq!(m.get_capacity(1, 0, 0), 0.0);
    assert_eq!(m.get_connection_count(1, 0, 0), 0);
}

#[test]
fn init_sizes_and_flags() {
    // §1: the cell array is the fixed 256 × 256; network changed is set.
    let w = World::new(4, 4);
    let m = model(&w);
    let s = m.state();
    assert_eq!((s.count_x, s.count_z), (4, 4));
    assert_eq!(s.conn.len(), 65_536);
    assert!(s.network_changed);
    assert_eq!(s.next_switch_id, 1);
    assert_eq!(s.subnet_counter, 1);
    assert!(s.switch_id.is_some() && s.switch_mode.is_some());
}

#[test]
fn rebuild_sets_stale_and_dirty() {
    // §4.2 steps 1 and 7.
    let (_w, m) = three_cell_street();
    m.update_state(|s| {
        s.subnetworks_stale = false;
        s.network_changed = false;
        s.intersection_dirty = false;
    });
    m.do_connections_changed(CellRect::new(1, 1, 1, 1));
    let s = m.state();
    assert!(s.subnetworks_stale && s.network_changed && s.intersection_dirty);
    // R = the rectangle grown by 2, clamped.
    assert_eq!(s.intersection_rect, CellRect::new(0, 0, 3, 3));
}

#[test]
fn neighbour_capacity_per_labelled_entry_side() {
    // §4.12: a neighbour connection's pool capacity is added to the label of
    // each labelled entry side of its in-city cell, not deduplicated.
    // Pieces (2, 1) and (3, 1), both (0 → 2) and (2 → 0): the fill from
    // (2, 1) turns around at the map edge, so (3, 1) gets subnet [1, 0, 1, 0].
    let w = World::new(4, 4);
    let m = model(&w);
    let straight = [(0, 2), (2, 0)];
    w.piece(1, 2, 1, &[STREET], &straight, &[]);
    w.piece(2, 3, 1, &[STREET], &straight, &[]);
    // A connection at the off-map cell (4, 1) maps to (3, 1). The pool
    // holder answers at both cells (§4.12 leaves open which one is asked).
    w.connections.borrow_mut().push((500, 1, (4, 1)));
    w.pools.borrow_mut().insert((4, 1), 2000);
    w.pools.borrow_mut().insert((3, 1), 2000);
    w.trip_caps.borrow_mut().insert(2000, [0, 0, 4, 0]);
    m.do_post_city_init();
    assert_eq!(car(&m, 3, 1).subnet, [1, 0, 1, 0]);
    assert_eq!(car(&m, 2, 1).subnet, [1, 0, 1, 0]);
    let s = m.state();
    assert_eq!(s.subnet_published.len(), 2);
    assert_eq!(s.subnet_published[1].capacity, [0, 0, 8, 0]);
}

#[test]
fn subnetwork_info_slices_to_the_same_result() {
    // §4.12: slicing changes only when the result is published.
    let (_w, m) = lot_world();
    let whole = m.state().subnet_published.clone();
    m.begin_subnetwork_info();
    let mut steps = 0;
    while !m.work_on_subnetwork_info(3) {
        steps += 1;
        assert_eq!(m.state().subnet_published, whole, "not published early");
    }
    assert!(steps > 0);
    assert_eq!(m.state().subnet_published, whole);
}

#[test]
fn transit_switch_lot_activation_and_removal() {
    // §4.7.1 and §4.7.5: a switch lot gets the next ID in the ID grid and
    // the mode mask in the mode grid; deactivation releases the building and
    // zeroes the grids.
    let w = World::new(4, 4);
    let m = model(&w);
    // A street piece north of the switch lot.
    w.piece(1, 1, 0, &[STREET], &[(0, 2), (2, 0)], &[]);
    let mut props = HashMap::new();
    props.insert(
        prop::SWITCH_RULES,
        Variant::array(mem::UINT32, vec![Elem::U32(0x0101_8000)]),
    );
    props.insert(prop::SWITCH_CAPACITY, Variant::Scalar(Elem::F32(100.0)));
    props.insert(prop::SWITCH_TIME_FACTOR, Variant::Scalar(Elem::F32(0.5)));
    w.add(
        40,
        Occ {
            kind: Kind::Building,
            pos: centre(1, 1),
            props,
            ..Occ::default()
        },
    );
    w.add_lot(
        7,
        Lot {
            rect: CellRect::new(1, 1, 2, 1),
            facing: 1,
            building: Some(40),
            holder: 0,
            pops: Vec::new(),
        },
    );
    assert!(m.lot_activated(7));
    {
        let s = m.state();
        assert_eq!(s.switch_id_at(1, 1), 1);
        assert_eq!(s.switch_id_at(2, 1), 1);
        assert_eq!(s.switch_id_at(3, 1), 0);
        assert_eq!(s.switch_mode_at(1, 1), 0b10);
        assert_eq!(s.next_switch_id, 2);
        let sw = s.switches.get(1).expect("switch 1");
        assert_eq!(sw.building, 40);
        assert_eq!(sw.capacity, 100.0);
        assert_eq!(sw.time_factor, 0.5);
        assert_eq!(sw.to_mask, 0x0002);
    }
    assert_eq!(w.refs.borrow().get(&40), Some(&1));
    // §4.7.4 with the test-11 rule (kind 0, side north, Car → Car): the
    // switch cells' words are cleared (no same-switch rule for east/west);
    // N = (1, 0), ns = 3: entries 0 and 2 get exit 3 (0x0808), then entry 3
    // gets exits 0 and 2 (0x5000), on top of the piece's 0x0104.
    assert_eq!(conn(&m, 1, 1).conn, [0; 9]);
    assert_eq!(conn(&m, 1, 0).conn[travel::CAR], 0x590C);
    assert_eq!(conn(&m, 1, 0).conn[travel::BUS], 0x0104);
    assert_eq!(m.get_transit_switches(), vec![40]);
    assert_eq!(m.transit_switch_query_target(2, 1), Some(SwitchQueryTarget::Switch(1)));
    m.lot_deactivated(7);
    assert_eq!(conn(&m, 1, 0).conn[travel::CAR], 0x0104, "rebuilt");
    let s = m.state();
    assert!(!s.switches.contains(1));
    assert_eq!(s.switch_id_at(1, 1), 0);
    assert_eq!(s.switch_mode_at(1, 1), 0);
    assert_eq!(w.refs.borrow().get(&40), Some(&0));
}

#[test]
fn ferry_terminals_route_and_link() {
    // §4.9.1 and §4.9.3: two type-1 terminals on one water row are routed to
    // each other with cost r24(cells × 0.01); removal drops the links back.
    let w = World::new(8, 8);
    let m = model(&w);
    for x in 0..8 {
        w.water.borrow_mut().insert((x, 0), (40.7, 1));
        w.under_water.borrow_mut().insert((x, 0));
    }
    let mut next = 60;
    let mut terminal = |lot: u64, x: i32| {
        let mut props = HashMap::new();
        props.insert(prop::FERRY_TYPE, Variant::Scalar(Elem::U8(1)));
        let building = next;
        w.add(
            building,
            Occ {
                kind: Kind::Building,
                pos: centre(x, 2),
                props,
                ..Occ::default()
            },
        );
        for (z, ty) in [
            (2, ferry_prop::DEPARTURE),
            (2, ferry_prop::ARRIVAL),
            (1, ferry_prop::WATER),
        ] {
            next += 1;
            w.add(
                next,
                Occ {
                    kind: Kind::Prop,
                    pos: centre(x, z),
                    prop_type: ty,
                    ..Occ::default()
                },
            );
        }
        next += 1;
        w.add_lot(
            lot,
            Lot {
                rect: CellRect::new(x, 1, x, 2),
                facing: 0,
                building: Some(building),
                holder: 0,
                pops: Vec::new(),
            },
        );
    };
    terminal(8, 1);
    terminal(9, 5);
    m.do_post_city_init();
    assert!(!m.lot_activated(8));
    assert!(!m.lot_activated(9));
    assert_eq!(m.ferry_terminal_count(1), 2);
    {
        let s = m.state();
        let (a, b) = (&s.ferries[0], &s.ferries[1]);
        assert_eq!((a.water_x, a.water_z), (1, 0));
        assert_eq!(a.mask, 3, "type 1: Walk and Car");
        // B's link to A and A's link to B: 5 cells apart → 5 cells.
        let ab = a.links.iter().find(|l| l.to_key == b.departure_key()).expect("A → B");
        let ba = b.links.iter().find(|l| l.to_key == a.departure_key()).expect("B → A");
        assert_eq!(ab.cost.to_bits(), route_cost(5).to_bits());
        assert_eq!(ab.cost.to_bits(), 0x3D4C_CCCC);
        assert_eq!(ab.route, encode_route(&[(1, 0), (2, 0), (3, 0), (4, 0), (5, 0)]));
        assert_eq!(ba.route, encode_route(&[(5, 0), (4, 0), (3, 0), (2, 0), (1, 0)]));
        // §4.9.2: the departure cell is marked and has records for
        // networks 0 and 1.
        assert_eq!(s.wormhole_at(1, 2) & wormhole::FERRY, wormhole::FERRY);
        assert!(s.network_info(net::PEDESTRIAN, 1, 2).is_some());
        assert_eq!(s.car_info(1, 2).unwrap().conn_count, 1);
    }
    assert_eq!(m.all_ferry_routes().len(), 2);
    // §4.9.5 GetFerryRoutesInUse: a link with a non-zero last counter gives
    // one record (lots by their cells, neighbour 0, cell (−2, −2), both
    // routes); B's link back is skipped because A was visited.
    m.update_state(|s| {
        s.ferries[0].links[0].counters = [3, 0, 0, 0];
        s.ferries[1].links[0].counters = [0, 0, 5, 0];
    });
    let used = m.ferry_routes_in_use();
    assert_eq!(used.len(), 1);
    let r = &used[0];
    assert_eq!((r.lot_a, r.lot_b, r.ferry_type), (8, 9, 1));
    assert_eq!((r.neighbour, r.neighbour_x, r.neighbour_z), (0, -2, -2));
    assert_eq!(r.route_ab, encode_route(&[(1, 0), (2, 0), (3, 0), (4, 0), (5, 0)]));
    assert_eq!(r.route_ba, encode_route(&[(5, 0), (4, 0), (3, 0), (2, 0), (1, 0)]));
    // §4.9.4: removal of A deletes B's link back to A, no rebuild.
    m.lot_deactivated(8);
    let s = m.state();
    assert_eq!(s.ferries.len(), 1);
    assert!(s.ferries[0].links.is_empty());
    assert_eq!(s.ferry_counts[1], 1);
    assert_eq!(s.wormhole_at(1, 2) & wormhole::FERRY, wormhole::FERRY, "no rebuild");
}

#[test]
fn ferry_pass_posts_when_done() {
    // §4.9.3 / §5: the periodic pass posts 0x2C06BA82 when the list is
    // exhausted.
    let w = World::new(4, 4);
    let m = model(&w);
    m.begin_ferry_pass();
    assert!(m.work_on_ferry_routes(usize::MAX));
    assert_eq!(*w.posted.borrow(), vec![msg::FERRY_ROUTES_DONE]);
}
