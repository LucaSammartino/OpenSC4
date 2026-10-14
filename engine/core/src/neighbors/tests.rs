// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The conformance tests of neighbors.md §18 (1–12) as unit tests, plus the
//! lifecycle (§14) and message handling (§11.1), against a mock region.
//!
//! The mock region: the current city `R` is a 1×1 city at region cell (3, 3)
//! in a 64×64-cell city; [`World::region`] adds four 2×2 neighbours that do
//! not overlap: `W` at (1, 3), `E` at (4, 2), `N` at (2, 1) and `S` at (3, 4).
//! The host *sends* status reports synchronously to the simulator (as the
//! message server does for `SetAmount`, §5.3) and only records the posted
//! ones; tests deliver posted reports by hand when they need to.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use super::simulator::marker_orientation;
use super::*;
use crate::curve::ResponseCurve;
use crate::stream::{ByteReader, ByteWriter, InStream, OutStream, RefResolver, RefValue, SaveRegistry};

const R: CityId = 1;
const W: CityId = 2;
const E: CityId = 3;
const N: CityId = 4;
const S: CityId = 5;
/// The current city object (`cISC4City`, §8.2 NB12): not a regional city.
const CITY: CityHandle = 0xC17;

const E0: u32 = deal_type::BUY_POWER;
const E1: u32 = deal_type::SELL_POWER;
const E2: u32 = deal_type::BUY_WATER;
const E3: u32 = deal_type::SELL_WATER;
const E4: u32 = deal_type::BUY_GARBAGE;
const E5: u32 = deal_type::SELL_GARBAGE;

const TODAY: u32 = 1000;

#[derive(Clone, Debug, PartialEq)]
enum Ev {
    Status {
        deal: Option<usize>,
        status: u32,
        flags: u32,
        sent: bool,
    },
    Conn {
        msg: u32,
        t: u32,
        x: i32,
        z: i32,
    },
    PropAdd {
        pos: [f32; 3],
        orientation: i32,
        props: Vec<u32>,
    },
    PropRemove(u64),
    PropMove(u64, [f32; 3]),
    Agent(u32, String, u32),
    Subscribe(u32),
    Unsubscribe(u32),
    RemoveAgents,
    Symmetric(CityId, bool),
    Reload(CityId),
    UpdateLocal(CityId),
    SetLocal(CityId, usize),
    UpdateImportExport(CityId),
    History(u32, i32),
}

#[derive(Clone, Default)]
struct RCity {
    pos: (i32, i32),
    size: (i32, i32),
    established: bool,
    budget: f64,
    income: f64,
    stats: HashMap<(Stat, i32), f64>,
    /// The city's own connections: (type, cell in that city, active).
    conns: Vec<(u32, (i32, i32), bool)>,
    /// `GetAllConnections` as (type, other-side region cell).
    links: Vec<(u32, (i32, i32))>,
    local: Vec<DealData>,
}

#[derive(Clone)]
struct Occ {
    kind: u32,
    networks: Vec<u32>,
    cell: (i32, i32),
    pole_flag: bool,
}

struct World {
    cities: RefCell<HashMap<CityId, RCity>>,
    current: Cell<Option<CityId>>,
    city_object: Cell<Option<CityHandle>>,
    counts: Cell<(i32, i32)>,
    today: Cell<u32>,
    figures: RefCell<UtilityFigures>,
    landfill: Cell<(u32, u32)>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    events: RefCell<Vec<Ev>>,
    props: RefCell<Vec<(u64, [f32; 3])>>,
    next_prop: Cell<u64>,
    occupants: RefCell<HashMap<u64, Occ>>,
    sim: RefCell<Weak<NeighborsSimulator>>,
    deliver_sent: Cell<bool>,
    altitude: Cell<f32>,
    /// Per world position altitudes (`GetAltitude(px, pz)`, §6.3); other
    /// positions read `altitude`.
    altitude_at: RefCell<Vec<((f32, f32), f32)>>,
    surface: Cell<f32>,
    /// Foreign connections (§15.2) by token, the repairs and releases asked
    /// for, and the next own-connection object token (0 = no objects).
    foreign: RefCell<HashMap<u64, ConnectionState>>,
    repaired: RefCell<Vec<(u64, CityId)>>,
    released: RefCell<Vec<u64>>,
    next_object: Cell<u64>,
    /// `ReloadCity` replaces the city object: the city moves to id + 0x100
    /// (§14.1 step 5 *[compat]*).
    reload_replaces: Cell<bool>,
}

fn ptr(d: &Rc<NeighborDeal>) -> usize {
    Rc::as_ptr(d) as usize
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            cities: RefCell::new(HashMap::new()),
            current: Cell::new(Some(R)),
            city_object: Cell::new(Some(CITY)),
            counts: Cell::new((64, 64)),
            today: Cell::new(TODAY),
            figures: RefCell::new(UtilityFigures::default()),
            landfill: Cell::new((0, 0)),
            tuning: RefCell::new(Some(Rc::new(shipped_tuning()))),
            events: RefCell::new(Vec::new()),
            props: RefCell::new(Vec::new()),
            next_prop: Cell::new(0x100),
            occupants: RefCell::new(HashMap::new()),
            sim: RefCell::new(Weak::new()),
            deliver_sent: Cell::new(true),
            altitude: Cell::new(270.0),
            altitude_at: RefCell::new(Vec::new()),
            surface: Cell::new(280.0),
            foreign: RefCell::new(HashMap::new()),
            repaired: RefCell::new(Vec::new()),
            released: RefCell::new(Vec::new()),
            next_object: Cell::new(0),
            reload_replaces: Cell::new(false),
        });
        w.add_city(R, (3, 3), (1, 1));
        w
    }

    fn add_city(&self, id: CityId, pos: (i32, i32), size: (i32, i32)) {
        self.cities.borrow_mut().insert(
            id,
            RCity {
                pos,
                size,
                established: true,
                ..RCity::default()
            },
        );
    }

    /// The four 2×2 neighbours of the module docs.
    fn region(&self) {
        self.add_city(W, (1, 3), (2, 2));
        self.add_city(E, (4, 2), (2, 2));
        self.add_city(N, (2, 1), (2, 2));
        self.add_city(S, (3, 4), (2, 2));
    }

    fn city<T>(&self, id: CityId, f: impl FnOnce(&mut RCity) -> T) -> T {
        f(self.cities.borrow_mut().get_mut(&id).expect("city"))
    }

    fn pos(&self, id: CityId) -> (i32, i32) {
        self.city(id, |c| c.pos)
    }

    fn set_stat(&self, id: CityId, stat: Stat, c: i32, v: f64) {
        self.city(id, |city| city.stats.insert((stat, c), v));
    }

    fn stat(&self, id: CityId, stat: Stat, c: i32) -> f64 {
        self.city(id, |city| city.stats.get(&(stat, c)).copied().unwrap_or(0.0))
    }

    fn set_budget(&self, id: CityId, v: f64) {
        self.city(id, |c| c.budget = v);
    }

    fn clear(&self) {
        self.events.borrow_mut().clear();
    }

    fn events(&self) -> Vec<Ev> {
        self.events.borrow().clone()
    }

    fn push(&self, e: Ev) {
        self.events.borrow_mut().push(e);
    }

    /// The statuses reported for `d`, in order (posted and sent).
    fn statuses(&self, d: &Rc<NeighborDeal>) -> Vec<u32> {
        self.events()
            .into_iter()
            .filter_map(|e| match e {
                Ev::Status { deal, status, .. } if deal == Some(ptr(d)) => Some(status),
                _ => None,
            })
            .collect()
    }

    fn count(&self, pred: impl Fn(&Ev) -> bool) -> usize {
        self.events().iter().filter(|e| pred(e)).count()
    }

    fn cell_of(pos: [f32; 3]) -> (i32, i32) {
        ((pos[0] / 16.0).floor() as i32, (pos[2] / 16.0).floor() as i32)
    }
}

impl NeighborsHost for World {
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.push(Ev::Agent(level, name.to_string(), flags));
    }
    fn remove_agents(&self) {
        self.push(Ev::RemoveAgents);
    }
    fn subscribe(&self, msg_type: u32) {
        self.push(Ev::Subscribe(msg_type));
    }
    fn unsubscribe(&self, msg_type: u32) {
        self.push(Ev::Unsubscribe(msg_type));
    }
    fn cell_counts(&self) -> (i32, i32) {
        self.counts.get()
    }
    fn today(&self) -> u32 {
        self.today.get()
    }
    fn utility_figures(&self) -> UtilityFigures {
        *self.figures.borrow()
    }
    fn landfill_usage(&self) -> (u32, u32) {
        self.landfill.get()
    }
    fn send_history(&self, series: u32, value: i32) {
        self.push(Ev::History(series, value));
    }
    fn current_city(&self) -> Option<CityId> {
        self.current.get()
    }
    fn current_city_object(&self) -> Option<CityHandle> {
        self.city_object.get()
    }
    fn city_at(&self, rx: i32, rz: i32) -> Option<CityId> {
        self.cities
            .borrow()
            .iter()
            .find(|(_, c)| rx >= c.pos.0 && rx < c.pos.0 + c.size.0 && rz >= c.pos.1 && rz < c.pos.1 + c.size.1)
            .map(|(&id, _)| id)
    }
    fn city_position(&self, city: CityId) -> (i32, i32) {
        self.city(city, |c| c.pos)
    }
    fn city_size(&self, city: CityId) -> (i32, i32) {
        self.city(city, |c| c.size)
    }
    fn city_established(&self, city: CityId) -> bool {
        self.city(city, |c| c.established)
    }
    fn city_stat(&self, city: CityId, stat: Stat, commodity: i32) -> f64 {
        self.stat(city, stat, commodity)
    }
    fn set_city_stat(&self, city: CityId, stat: Stat, commodity: i32, value: f64) {
        self.set_stat(city, stat, commodity, value);
    }
    fn city_budget(&self, city: CityId) -> f64 {
        self.city(city, |c| c.budget)
    }
    fn set_city_budget(&self, city: CityId, value: f64) {
        self.city(city, |c| c.budget = value);
    }
    fn set_city_income(&self, city: CityId, value: f64) {
        self.city(city, |c| c.income = value);
    }
    fn city_find_connection(&self, city: CityId, conn_type: u32, x: i32, z: i32) -> Option<ForeignConnection> {
        self.city(city, |c| {
            c.conns
                .iter()
                .position(|&(t, cell, _)| t == conn_type && cell == (x, z))
                .map(|i| ForeignConnection {
                    token: 0xF000 + i as u64,
                    active: c.conns[i].2,
                })
        })
    }
    fn city_connections(&self, city: CityId) -> Vec<(u32, (i32, i32))> {
        self.city(city, |c| c.links.clone())
    }
    fn change_symmetric_connection(&self, city: CityId, _conn: &Rc<NeighborConnection>, add: bool) {
        self.push(Ev::Symmetric(city, add));
    }
    fn reload_city(&self, city: CityId) {
        self.push(Ev::Reload(city));
        if self.reload_replaces.get() {
            let mut cities = self.cities.borrow_mut();
            if let Some(c) = cities.remove(&city) {
                cities.insert(city + 0x100, c);
            }
        }
    }
    fn update_local_deals(&self, city: CityId) {
        self.push(Ev::UpdateLocal(city));
    }
    fn local_deals(&self, city: CityId) -> Vec<DealData> {
        self.city(city, |c| c.local.clone())
    }
    fn set_local_deals(&self, city: CityId, deals: &mut Vec<Rc<NeighborDeal>>) {
        // foundations §11.3 [quirk]: deals with a missing region city leave
        // the caller's list.
        deals.retain(|d| {
            let (ox, oz) = d.other_side();
            let (cx, cz) = d.city_side();
            self.city_at(ox, oz).is_some() && self.city_at(cx, cz).is_some()
        });
        self.push(Ev::SetLocal(city, deals.len()));
    }
    fn update_import_export(&self, city: CityId) {
        self.push(Ev::UpdateImportExport(city));
    }
    fn post_status(&self, deal: Option<&Rc<NeighborDeal>>, status: u32) {
        self.push(Ev::Status {
            deal: deal.map(ptr),
            status,
            flags: deal.map_or(0, |d| d.flags()),
            sent: false,
        });
    }
    fn send_status(&self, deal: &Rc<NeighborDeal>, status: u32) {
        self.push(Ev::Status {
            deal: Some(ptr(deal)),
            status,
            flags: deal.flags(),
            sent: true,
        });
        if !self.deliver_sent.get() {
            return;
        }
        let sim = self.sim.borrow().upgrade();
        if let Some(sim) = sim {
            sim.do_message(&NeighborsMessage {
                msg_type: msg::DEAL_STATUS,
                data1: status,
                ..NeighborsMessage::default()
            });
        }
    }
    fn post_connection_message(&self, msg_type: u32, conn_type: u32, x: i32, z: i32) {
        self.push(Ev::Conn {
            msg: msg_type,
            t: conn_type,
            x,
            z,
        });
    }
    fn cell_center(&self, x: i32, z: i32) -> (f32, f32) {
        (x as f32 * 16.0 + 8.0, z as f32 * 16.0 + 8.0)
    }
    fn marker_altitude(&self, px: f32, pz: f32) -> f32 {
        let at = self.altitude_at.borrow();
        at.iter()
            .find(|(p, _)| *p == (px, pz))
            .map_or(self.altitude.get(), |&(_, y)| y)
    }
    fn max_surface_level(&self, _x: i32, _z: i32) -> f32 {
        self.surface.get()
    }
    fn add_marker_prop(&self, position: [f32; 3], orientation: i32, bool_props: &[u32]) -> Option<u64> {
        let token = self.next_prop.get();
        self.next_prop.set(token + 1);
        self.props.borrow_mut().push((token, position));
        self.push(Ev::PropAdd {
            pos: position,
            orientation,
            props: bool_props.to_vec(),
        });
        Some(token)
    }
    fn marker_props(&self, x: i32, z: i32) -> Vec<u64> {
        self.props
            .borrow()
            .iter()
            .filter(|(_, p)| World::cell_of(*p) == (x, z))
            .map(|&(t, _)| t)
            .collect()
    }
    fn remove_prop(&self, prop: u64) {
        self.props.borrow_mut().retain(|&(t, _)| t != prop);
        self.push(Ev::PropRemove(prop));
    }
    fn move_prop(&self, prop: u64, position: [f32; 3]) {
        for p in self.props.borrow_mut().iter_mut() {
            if p.0 == prop {
                p.1 = position;
            }
        }
        self.push(Ev::PropMove(prop, position));
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        self.occupants.borrow().get(&occupant).map_or(0, |o| o.kind)
    }
    fn occupant_networks(&self, occupant: u64) -> Vec<u32> {
        self.occupants
            .borrow()
            .get(&occupant)
            .map_or(Vec::new(), |o| o.networks.clone())
    }
    fn occupant_cell(&self, occupant: u64) -> (i32, i32) {
        self.occupants.borrow().get(&occupant).map_or((0, 0), |o| o.cell)
    }
    fn power_pole_flag(&self, occupant: u64) -> bool {
        self.occupants.borrow().get(&occupant).is_some_and(|o| o.pole_flag)
    }
    fn connection_object(&self, _conn: &Rc<NeighborConnection>) -> u64 {
        let t = self.next_object.get();
        if t != 0 {
            self.next_object.set(t + 1);
        }
        t
    }
    fn foreign_connection(&self, token: u64) -> Option<ConnectionState> {
        self.foreign.borrow().get(&token).copied()
    }
    fn set_foreign_connection_flags(&self, token: u64, visited: Option<bool>, active: Option<bool>) {
        if let Some(s) = self.foreign.borrow_mut().get_mut(&token) {
            if let Some(v) = visited {
                s.visited = v;
            }
            if let Some(a) = active {
                s.active = a;
            }
        }
    }
    fn repair_foreign_connection(&self, token: u64, city: CityId) {
        self.repaired.borrow_mut().push((token, city));
    }
    fn release_connection(&self, token: u64) {
        self.released.borrow_mut().push(token);
    }
}

/// A world (with the four neighbours when `region`) and its simulator.
fn setup(region: bool) -> (Rc<World>, Rc<NeighborsSimulator>) {
    let w = World::new();
    if region {
        w.region();
    }
    let sim = NeighborsSimulator::new(w.clone(), 0x51, 0x1234_5678);
    *w.sim.borrow_mut() = Rc::downgrade(&sim);
    (w, sim)
}

/// `setup(true)`, `Init`, ConnectionsReady; the events cleared.
fn running() -> (Rc<World>, Rc<NeighborsSimulator>) {
    let (w, sim) = setup(true);
    assert!(sim.init());
    sim.do_message(&NeighborsMessage::of_type(msg::CONNECTIONS_READY));
    w.clear();
    (w, sim)
}

/// A deal of type `t` with `other`, seen from `R`, in the simulator's list.
fn deal(w: &World, sim: &Rc<NeighborsSimulator>, t: u32, other: CityId, flags: u32) -> Rc<NeighborDeal> {
    let d = NeighborDeal::from_data(DealData {
        deal_type: t,
        other_side: w.pos(other),
        city_side: w.pos(R),
        flags,
        start_date: TODAY - 100,
        current_date: TODAY - 1,
        expiration_date: TODAY + 1000,
        ..DealData::default()
    });
    sim.adopt_deal(&d);
    d
}

// ---- 1. Geometry -------------------------------------------------------------------------------------

#[test]
fn test_01_geometry() {
    // Side classification in a 64×64 city.
    assert_eq!(side_of(-1, 5, 64, 64), Side::West);
    assert_eq!(side_of(64, 5, 64, 64), Side::East);
    assert_eq!(side_of(5, -1, 64, 64), Side::North);
    assert_eq!(side_of(5, 64, 64, 64), Side::South);
    assert_eq!(side_of(5, 5, 64, 64), Side::Interior);

    // A 1×1 city at (3, 3) with 2×2 neighbours.
    let (pos, size) = ((3, 3), (1, 1));
    let cases = [
        // (cell, side, region cell, neighbour position, neighbour cell)
        ((-1, 5), Side::West, (2, 3), (1, 3), (128, 5)),
        ((64, 5), Side::East, (4, 3), (4, 2), (-1, 69)),
        ((5, -1), Side::North, (3, 2), (2, 1), (69, 128)),
        ((5, 64), Side::South, (3, 4), (3, 4), (5, -1)),
    ];
    for ((x, z), side, region_cell, npos, ncell) in cases {
        assert_eq!(side_of(x, z, 64, 64), side);
        assert_eq!(neighbouring_region_cell(side, x, z, pos, size), region_cell);
        assert_eq!(neighbour_connection_cell(side, x, z, region_cell, npos, (2, 2)), ncell);
        assert!(is_valid_connection_cell(ncell.0, ncell.1, (2, 2)));
        assert!(is_valid_connection_cell(x, z, size));
    }
    // Corners and interior cells are invalid.
    for (x, z) in [(-1, -1), (64, -1), (-1, 64), (64, 64), (5, 5)] {
        assert!(!is_valid_connection_cell(x, z, size), "({x}, {z})");
    }
    assert_eq!(connection_inner_cell(-1, -1, 64, 64), None);
    assert_eq!(connection_inner_cell(5, 5, 64, 64), None);
    assert_eq!(connection_inner_cell(-1, 5, 64, 64), Some((0, 5)));
    assert_eq!(connection_inner_cell(5, 64, 64, 64), Some((5, 63)));

    // The same through the simulator and the region.
    let (w, sim) = setup(true);
    w.city(W, |c| c.conns.push((conn_type::ROAD, (128, 5), true)));
    w.city(E, |c| c.conns.push((conn_type::ROAD, (-1, 69), false)));
    assert_eq!(sim.find_city_on_other_side_of_connection(-1, 5), Some(W));
    assert_eq!(sim.find_city_on_other_side_of_connection(64, 5), Some(E));
    assert_eq!(sim.find_city_on_other_side_of_connection(5, -1), Some(N));
    assert_eq!(sim.find_city_on_other_side_of_connection(5, 64), Some(S));
    assert_eq!(sim.neighbour_cell(5, -1), Some((N, (69, 128))));
    assert_eq!(
        sim.find_connection_on_other_side(conn_type::ROAD, -1, 5),
        Some(ForeignConnection {
            token: 0xF000,
            active: true
        })
    );
    assert_eq!(
        sim.find_connection_on_other_side(conn_type::ROAD, 64, 5)
            .map(|c| c.active),
        Some(false)
    );
    assert_eq!(sim.find_connection_on_other_side(conn_type::RAIL, -1, 5), None);

    // NB6 (§2.2): an interior cell keeps our own region position, finds our
    // own regional city there and gives (0, 0); no connection lies there, so
    // FindConnectionOnOtherSide still ends with null.
    assert_eq!(
        neighbour_connection_cell(Side::Interior, 10, 20, (3, 3), (3, 3), (1, 1)),
        (0, 0)
    );
    assert_eq!(sim.find_city_on_other_side_of_connection(10, 20), Some(R));
    assert_eq!(sim.neighbour_cell(10, 20), Some((R, (0, 0))));
    assert_eq!(sim.find_connection_on_other_side(conn_type::ROAD, 10, 20), None);
}

// ---- 2. Connections ----------------------------------------------------------------------------------

#[test]
fn test_02_create_connection_once_with_one_marker() {
    let (w, sim) = setup(true);
    let c = sim.create_connection(conn_type::ROAD, -1, 5, false).expect("created");
    assert!(sim.create_connection(conn_type::ROAD, -1, 5, false).is_none());
    assert_eq!(sim.all_connections().len(), 1);
    assert!(Rc::ptr_eq(&sim.find_connection(conn_type::ROAD, -1, 5).unwrap(), &c));
    assert_eq!(c.other_region_cell(), (2, 3));
    assert_eq!(
        w.events(),
        vec![
            Ev::Symmetric(W, true),
            Ev::PropAdd {
                pos: [8.0, 270.0, 88.0],
                orientation: 3,
                props: vec![occupant::MARKER_FLAG],
            },
            Ev::Conn {
                msg: msg::CONNECTION_CREATED,
                t: conn_type::ROAD,
                x: -1,
                z: 5
            },
        ]
    );
}

/// §6.3 (NB3): the marker height is `GetAltitude(px, pz)` at the inner
/// cell's centre, the point the prop is placed at; the connection cell is
/// never used. Road at (−1, 5) → inner (0, 5), centre (8, 88); Highway at
/// (5, −1) → inner (5, 0), centre (88, 8), 15.5 higher.
#[test]
fn test_02_marker_height_at_the_inner_cell_centre() {
    let (w, sim) = setup(true);
    *w.altitude_at.borrow_mut() = vec![
        ((8.0, 88.0), 300.0),
        ((-8.0, 88.0), 250.0),
        ((88.0, 8.0), 310.0),
        ((88.0, -8.0), 0.0),
    ];
    let pos_of = |w: &World| {
        w.events()
            .into_iter()
            .find_map(|e| match e {
                Ev::PropAdd { pos, .. } => Some(pos),
                _ => None,
            })
            .unwrap()
    };
    w.clear();
    sim.create_connection(conn_type::ROAD, -1, 5, false).unwrap();
    assert_eq!(pos_of(&w), [8.0, 300.0, 88.0]);
    w.clear();
    sim.create_connection(conn_type::HIGHWAY, 5, -1, false).unwrap();
    assert_eq!(pos_of(&w), [88.0, 325.5, 8.0]);
}

#[test]
fn test_02_marker_orientation_props_and_height() {
    let (w, sim) = setup(true);
    let cells = [(-1, 5), (5, -1), (64, 5), (5, 64)];
    // flag = false: West 3, North 0, East 1, South 2; flag = true: 1, 2, 3, 0.
    for (flip, expected) in [(false, [3, 0, 1, 2]), (true, [1, 2, 3, 0])] {
        let t = if flip { conn_type::RAIL } else { conn_type::ROAD };
        for (&(x, z), &o) in cells.iter().zip(expected.iter()) {
            w.clear();
            sim.create_connection(t, x, z, flip).unwrap();
            let adds: Vec<i32> = w
                .events()
                .into_iter()
                .filter_map(|e| match e {
                    Ev::PropAdd { orientation, .. } => Some(orientation),
                    _ => None,
                })
                .collect();
            assert_eq!(adds, vec![o], "({x}, {z}) flag {flip}");
        }
    }
    assert_eq!(marker_orientation(Side::Interior, true), 0);

    // Ferry: no marker.
    w.clear();
    sim.create_connection(conn_type::FERRY, -1, 20, false).unwrap();
    assert_eq!(w.count(|e| matches!(e, Ev::PropAdd { .. })), 0);
    assert_eq!(w.count(|e| matches!(e, Ev::Conn { .. })), 1);

    // Pipe, subway and the elevated height.
    let props_of = |w: &World| {
        w.events()
            .into_iter()
            .find_map(|e| match e {
                Ev::PropAdd { pos, props, .. } => Some((pos[1], props)),
                _ => None,
            })
            .unwrap()
    };
    w.clear();
    sim.create_connection(conn_type::PIPE, -1, 30, false).unwrap();
    assert_eq!(
        props_of(&w),
        (
            270.0,
            vec![
                occupant::MARKER_FLAG,
                occupant::MARKER_UNDERGROUND,
                occupant::MARKER_PIPE
            ]
        )
    );
    w.clear();
    sim.create_connection(conn_type::SUBWAY, -1, 31, false).unwrap();
    assert_eq!(
        props_of(&w),
        (
            270.0,
            vec![
                occupant::MARKER_FLAG,
                occupant::MARKER_UNDERGROUND,
                occupant::MARKER_SUBWAY
            ]
        )
    );
    for t in [conn_type::HIGHWAY, conn_type::LIGHT_RAIL, conn_type::MONORAIL] {
        w.clear();
        sim.create_connection(t, -1, 40, false).unwrap();
        assert_eq!(props_of(&w).0, 285.5);
    }
}

#[test]
fn test_02_delete_occupant_connection() {
    let (w, sim) = setup(true);
    sim.create_connection(conn_type::ROAD, -1, 5, false).unwrap();
    sim.create_connection(conn_type::RAIL, -1, 5, false).unwrap();
    // A network occupant at (0, 5) carrying road (0) and rail (1): rail is
    // later in the list and wins.
    w.occupants.borrow_mut().insert(
        7,
        Occ {
            kind: occupant::NETWORK,
            networks: vec![0, 1],
            cell: (0, 5),
            pole_flag: false,
        },
    );
    w.clear();
    sim.delete_occupant_connection(7);
    assert!(sim.has_connection(conn_type::ROAD, -1, 5));
    assert!(!sim.has_connection(conn_type::RAIL, -1, 5));
    let conn_events: Vec<Ev> = w
        .events()
        .into_iter()
        .filter(|e| matches!(e, Ev::Conn { .. } | Ev::Symmetric(..)))
        .collect();
    assert_eq!(
        conn_events,
        vec![
            Ev::Symmetric(W, false),
            Ev::Conn {
                msg: msg::CONNECTION_DELETED,
                t: conn_type::RAIL,
                x: -1,
                z: 5
            }
        ]
    );
    // §6.4: every marker prop in the inner cell goes (the road's too).
    assert_eq!(w.count(|e| matches!(e, Ev::PropRemove(_))), 2);

    // A power pole in the corner cell (0, 0) deletes two connections.
    sim.create_connection(conn_type::POWER_POLE, -1, 0, false).unwrap();
    sim.create_connection(conn_type::POWER_POLE, 0, -1, false).unwrap();
    w.occupants.borrow_mut().insert(
        8,
        Occ {
            kind: occupant::POWER_POLE,
            networks: vec![],
            cell: (0, 0),
            pole_flag: false,
        },
    );
    w.clear();
    sim.delete_occupant_connection(8);
    assert!(sim.connections_of_type(conn_type::POWER_POLE).is_empty());
    assert_eq!(
        w.count(|e| matches!(e, Ev::Conn { msg, .. } if *msg == msg::CONNECTION_DELETED)),
        2
    );

    // Another occupant type, or a network with no listed network: nothing.
    w.occupants.borrow_mut().insert(
        9,
        Occ {
            kind: occupant::NETWORK,
            networks: vec![3],
            cell: (0, 5),
            pole_flag: false,
        },
    );
    w.clear();
    sim.delete_occupant_connection(9);
    sim.delete_occupant_connection(10);
    assert!(w.events().is_empty());
}

/// Resolves references by creating core connections, as the class store does.
#[derive(Default)]
struct Resolver {
    made: Vec<(u32, Rc<NeighborConnection>)>,
}

impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        if class_id != CLSID_NEIGHBOR_CONNECTION {
            return None;
        }
        if let Some((_, c)) = self.made.iter().find(|(id, _)| *id == object_id) {
            return Some(RefValue::Object(c.clone()));
        }
        let c = NeighborConnection::new();
        self.made.push((object_id, c.clone()));
        Some(RefValue::Object(c))
    }
}

/// The simulator record and every connection record it references.
fn save(sim: &NeighborsSimulator) -> (Vec<u8>, Vec<(u32, Vec<u8>)>) {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(reg.clone());
    assert!(sim.write(&mut out));
    let mut records = Vec::new();
    for (class, id, obj, _) in reg.borrow().entries.iter() {
        assert_eq!(*class, CLSID_NEIGHBOR_CONNECTION);
        let mut o = ByteWriter::new();
        assert!(obj.as_ref().unwrap().write(&mut o));
        records.push((*id, o.into_bytes()));
    }
    (out.into_bytes(), records)
}

#[test]
fn test_02_save_round_trip_and_version() {
    let (_w, sim) = setup(true);
    sim.create_connection(conn_type::ROAD, -1, 5, false).unwrap();
    let rail = sim.create_connection(conn_type::RAIL, 64, 5, false).unwrap();
    rail.set_active(false);
    rail.set_visited_this_cycle(true);
    let mut props = PropertyHolder::new();
    props.add(
        0x1234_5678,
        crate::variant::Variant::Scalar(crate::variant::Elem::F32(2.5)),
    );
    rail.set_properties(props);
    let (record, conns) = save(&sim);

    // u16 2, u32 count, count × (object id, class).
    let mut r = ByteReader::new(&record);
    assert_eq!(r.get_u16(), Some(2));
    assert_eq!(r.get_u32(), Some(2));
    assert_eq!(r.get_u32(), Some(1));
    assert_eq!(r.get_u32(), Some(CLSID_NEIGHBOR_CONNECTION));
    // Connection record: u16 1, type, cell, other region cell, flags byte.
    let rail_bytes = &conns[1].1;
    let mut c = ByteReader::new(rail_bytes);
    assert_eq!(c.get_u16(), Some(1));
    assert_eq!(c.get_u32(), Some(conn_type::RAIL));
    assert_eq!((c.get_i32(), c.get_i32()), (Some(64), Some(5)));
    assert_eq!((c.get_i32(), c.get_i32()), (Some(4), Some(3)));
    assert_eq!(c.get_u8(), Some(3));

    // Read back into a fresh simulator, then write again: identical bytes.
    let (_w2, sim2) = setup(true);
    let mut res = Resolver::default();
    {
        let mut inp = ByteReader::with_resolver(&record, &mut res);
        assert!(sim2.read(&mut inp));
    }
    for (id, bytes) in &conns {
        let (_, c) = res.made.iter().find(|(i, _)| i == id).unwrap();
        assert!(c.read(&mut ByteReader::new(bytes)));
    }
    assert_eq!(sim2.all_connections().len(), 2);
    let back = sim2.find_connection(conn_type::RAIL, 64, 5).unwrap();
    assert!(!back.is_active());
    assert!(back.visited_this_cycle());
    assert_eq!(save(&sim2), (record.clone(), conns.clone()));

    // The list is not cleared by Read: a second read appends.
    let mut res2 = Resolver::default();
    assert!(sim2.read(&mut ByteReader::with_resolver(&record, &mut res2)));
    assert_eq!(sim2.all_connections().len(), 4);

    // Version 3 is rejected by both records.
    let mut bad = record.clone();
    bad[0] = 3;
    let (_w3, sim3) = setup(true);
    assert!(!sim3.read(&mut ByteReader::new(&bad)));
    assert!(sim3.all_connections().is_empty());
    let mut bad_conn = conns[0].1.clone();
    bad_conn[0] = 3;
    assert!(!NeighborConnection::new().read(&mut ByteReader::new(&bad_conn)));
}

// ---- 3–5. Pricing ------------------------------------------------------------------------------------

/// Test 3's statistics: `R` buys power from `W`.
fn price_world() -> (Rc<World>, Rc<NeighborsSimulator>) {
    let (w, sim) = setup(true);
    assert!(sim.init());
    w.set_stat(R, Stat::Demanded, 1, 500.0);
    w.set_stat(R, Stat::Exported, 1, 0.0);
    w.set_stat(R, Stat::Produced, 1, 300.0);
    w.set_stat(R, Stat::Imported, 1, 0.0);
    w.set_stat(W, Stat::Produced, 1, 2000.0);
    w.set_stat(W, Stat::Imported, 1, 0.0);
    w.set_stat(W, Stat::Demanded, 1, 1000.0);
    w.set_stat(W, Stat::Exported, 1, 0.0);
    w.set_stat(W, Stat::CostPerUnit, 1, 2.0);
    (w, sim)
}

fn test3_inputs(existing: f64) -> (CommodityStats, CommodityStats, PriceInputs) {
    let buyer = CommodityStats {
        demanded: 500.0,
        exported: 0.0,
        produced: 300.0,
        imported: 0.0,
        cost_per_unit: 0.0,
    };
    let seller = CommodityStats {
        demanded: 1000.0,
        exported: 0.0,
        produced: 2000.0,
        imported: 0.0,
        cost_per_unit: 2.0,
    };
    let p = PriceInputs {
        base: 1000.0,
        scale: 2.5,
        min_markup: f64::from(0.1f32),
        max_markup: f64::from(0.5f32),
        existing,
    };
    (buyer, seller, p)
}

#[test]
fn test_03_price() {
    let scarcity = ResponseCurve::from_flat(&DEAL_PRICE_CURVE).unwrap();
    let garbage = ResponseCurve::from_flat(&GARBAGE_COST_CURVE).unwrap();
    let (buyer, seller, p) = test3_inputs(0.0);
    let c = deal_cost(&buyer, &seller, 100.0, &p, &|x| scarcity.eval(x));
    assert!((c / 100.0 - 2.631_428_6).abs() < 1e-6, "{c}");
    assert_eq!(c.trunc(), 263.0);
    let g = deal_cost(&buyer, &seller, 100.0, &p, &|x| garbage.eval(x));
    assert!((g / 100.0 - 2.215_041_8).abs() < 1e-6, "{g}");
    assert_eq!(g.trunc(), 221.0);
    // Below 1e-8: 0.
    assert_eq!(deal_cost(&buyer, &seller, 1e-9, &p, &|x| scarcity.eval(x)), 0.0);

    // Through the simulator (Init loaded the scarcity curve).
    let (_w, sim) = price_world();
    let s = sim.get_deal_cost(E0, 100.0, Some(W));
    assert!((s / 100.0 - 2.631_428_6).abs() < 1e-6, "{s}");
    // A deal's UpdateCost stores the truncation.
    let d = sim.propose_deal(E0, W);
    assert!(d.is_none(), "no power connection yet");
    // [choice] unknown types and a null city price at 0.
    assert_eq!(sim.get_deal_cost(0x1234, 100.0, Some(W)), 0.0);
    assert_eq!(sim.get_deal_cost(E0, 100.0, None), 0.0);
}

#[test]
fn test_04_max_amount() {
    let (buyer, seller, p) = test3_inputs(0.0);
    assert_eq!(deal_max_amount(&buyer, &seller, &p), 950.0);
    let (buyer, seller, p) = test3_inputs(100.0);
    assert_eq!(deal_max_amount(&buyer, &seller, &p), 1050.0);
    let (buyer, mut seller, p) = test3_inputs(0.0);
    seller.demanded = 1900.0;
    assert_eq!(deal_max_amount(&buyer, &seller, &p), 100.0);

    // Through the simulator: `existing` is the first E0 deal with W.
    let (w, sim) = price_world();
    assert_eq!(sim.get_deal_max_amount(E0, Some(W)), 950.0);
    let d = deal(&w, &sim, E0, W, flag::ACTIVE);
    d.set_data(DealData {
        amount: 100,
        ..d.data()
    });
    assert_eq!(sim.get_deal_max_amount(E0, Some(W)), 1050.0);
    // A deal with another city does not count.
    let other = deal(&w, &sim, E0, E, flag::ACTIVE);
    other.set_data(DealData {
        amount: 500,
        ..other.data()
    });
    assert_eq!(sim.get_deal_max_amount(E0, Some(W)), 1050.0);
    d.set_data(DealData { amount: 0, ..d.data() });
    w.set_stat(W, Stat::Demanded, 1, 1900.0);
    assert_eq!(sim.get_deal_max_amount(E0, Some(W)), 100.0);
    assert_eq!(sim.get_deal_max_amount(E0, None), 0.0);
}

/// `UpdateCommodityStats` figures that keep test 3's power statistics.
fn keep_power_figures(w: &World, sanitation: Option<f64>) {
    *w.figures.borrow_mut() = UtilityFigures {
        power_production: 300.0,
        power_requirement: 500.0,
        sanitation_department_expenses: sanitation,
        ..UtilityFigures::default()
    };
}

#[test]
fn test_05_curve_bug() {
    // With a sanitation department the garbage curve replaces the price curve.
    let (w, sim) = price_world();
    keep_power_figures(&w, Some(0.0));
    assert_eq!(sim.get_deal_cost(E0, 100.0, Some(W)).trunc(), 263.0);
    sim.update_commodity_stats(Some(CITY));
    assert_eq!(w.stat(R, Stat::Produced, 1), 300.0);
    assert_eq!(w.stat(R, Stat::Demanded, 1), 500.0);
    let g = sim.get_deal_cost(E0, 100.0, Some(W));
    assert!((g / 100.0 - 2.215_041_8).abs() < 1e-6, "{g}");
    assert_eq!(g.trunc(), 221.0);

    // Without the department: unchanged.
    let (w, sim) = price_world();
    keep_power_figures(&w, None);
    sim.update_commodity_stats(Some(CITY));
    assert_eq!(sim.get_deal_cost(E0, 100.0, Some(W)).trunc(), 263.0);
    assert_eq!(w.stat(R, Stat::CostPerUnit, 3), 1e-5);

    // Without the property: unchanged.
    let (w, sim) = setup(true);
    let mut h = shipped_tuning();
    h.remove(prop::GARBAGE_COST_CURVE);
    *w.tuning.borrow_mut() = Some(Rc::new(h));
    assert!(sim.init());
    for (k, v) in [(Stat::Demanded, 500.0), (Stat::Produced, 300.0)] {
        w.set_stat(R, k, 1, v);
    }
    for (k, v) in [
        (Stat::Produced, 2000.0),
        (Stat::Demanded, 1000.0),
        (Stat::CostPerUnit, 2.0),
    ] {
        w.set_stat(W, k, 1, v);
    }
    keep_power_figures(&w, Some(0.0));
    sim.update_commodity_stats(Some(CITY));
    assert_eq!(sim.get_deal_cost(E0, 100.0, Some(W)).trunc(), 263.0);

    // [choice] The two-curve switch keeps the scarcity curve.
    let (w, sim) = price_world();
    sim.set_two_curves(true);
    keep_power_figures(&w, Some(0.0));
    sim.update_commodity_stats(Some(CITY));
    assert_eq!(sim.get_deal_cost(E0, 100.0, Some(W)).trunc(), 263.0);

    // Only the current city object is updated (NB12): another regional
    // city, our own regional city, another city object or null do nothing.
    let (w, sim) = price_world();
    keep_power_figures(&w, Some(0.0));
    w.clear();
    for other in [Some(W), Some(R), Some(CITY + 1), None] {
        sim.update_commodity_stats(other);
        assert!(w.events().is_empty());
        assert_eq!(sim.get_deal_cost(E0, 100.0, Some(W)).trunc(), 263.0);
    }
    // [choice] With no current regional city it returns at once.
    w.current.set(None);
    sim.update_commodity_stats(Some(CITY));
    assert!(w.events().is_empty());
    w.current.set(Some(R));
    // No current city object: nothing matches.
    w.city_object.set(None);
    sim.update_commodity_stats(Some(CITY));
    assert!(w.events().is_empty());
    w.city_object.set(Some(CITY));
    sim.update_commodity_stats(Some(CITY));
    assert_eq!(sim.get_deal_cost(E0, 100.0, Some(W)).trunc(), 221.0);
}

#[test]
fn update_commodity_stats_writes_the_regional_figures_and_history() {
    let (w, sim) = setup(true);
    assert!(sim.init());
    *w.figures.borrow_mut() = UtilityFigures {
        total_funds: 12345.0,
        monthly_income: 678.0,
        power_production: 1000.0,
        power_requirement: 800.0,
        water_capacity: 2000.0,
        water_demand: 1500.0,
        garbage_recycled: 10.0,
        waste_to_energy_capacity: 20.0,
        incinerator_capacity: 30.0,
        garbage_abducted: 5.0,
        garbage_produced: 400.0,
        power_department_expenses: Some(99.0),
        water_department_expenses: None,
        sanitation_department_expenses: Some(1.0),
        waste_to_energy_expenses: 2.0,
    };
    w.landfill.set((100, 1300));
    w.set_stat(R, Stat::Imported, 1, 50.0);
    w.set_stat(R, Stat::Exported, 3, 7.9);
    let d = deal(&w, &sim, E0, W, flag::ACTIVE | flag::CANCELLED);
    d.set_data(DealData { cost: 51, ..d.data() });
    w.clear();
    sim.update_commodity_stats(Some(CITY));
    assert_eq!(w.city(R, |c| (c.budget, c.income)), (12345.0, 678.0));
    assert_eq!(w.stat(R, Stat::Produced, 1), 1000.0);
    assert_eq!(w.stat(R, Stat::Demanded, 1), 800.0);
    // (P + expenses) / (produced + imp + 1), with every deal's cost.
    assert_eq!(w.stat(R, Stat::CostPerUnit, 1), (51.0 + 99.0) / 1051.0);
    assert_eq!(w.stat(R, Stat::CostPerUnit, 2), 1e-5);
    // Garbage: free / (12 × 1 year) = 1200 / 12 = 100.
    let produced = 10.0 + 20.0 + 30.0 + 100.0 + 5.0;
    assert_eq!(w.stat(R, Stat::Produced, 3), produced);
    assert_eq!(w.stat(R, Stat::Demanded, 3), 400.0);
    let garbage = ResponseCurve::from_flat(&GARBAGE_COST_CURVE).unwrap();
    let x = (2.0 + 0.0 + 1.0) / (produced + 0.0 + 1.0);
    assert_eq!(w.stat(R, Stat::CostPerUnit, 3), f64::from(garbage.eval(x as f32)));
    let history: Vec<(u32, i32)> = w
        .events()
        .into_iter()
        .filter_map(|e| match e {
            Ev::History(s, v) => Some((s, v)),
            _ => None,
        })
        .collect();
    assert_eq!(
        history,
        vec![
            (history::POWER_IMPORTED, 50),
            (history::POWER_EXPORTED, 0),
            (history::WATER_IMPORTED, 0),
            (history::WATER_EXPORTED, 0),
            (history::GARBAGE_EXPORTED, 7),
            (history::GARBAGE_IMPORTED, 0),
            (history::GARBAGE_DEMANDED, 400),
            (history::GARBAGE_PRODUCED, 165),
        ]
    );
    // Step 3 pushes both flags before step 4.
    let first_history = w.events().iter().position(|e| matches!(e, Ev::History(..))).unwrap();
    let last_push = w
        .events()
        .iter()
        .rposition(|e| matches!(e, Ev::UpdateImportExport(_)))
        .unwrap();
    assert!(last_push < first_history);
}

// ---- 6. Life cycle -----------------------------------------------------------------------------------

/// A running simulator with a power-pole connection to `W` and a sell-power
/// deal using it.
fn life_cycle(
    flags: u32,
) -> (
    Rc<World>,
    Rc<NeighborsSimulator>,
    Rc<NeighborConnection>,
    Rc<NeighborDeal>,
) {
    let (w, sim) = running();
    let c = sim.create_connection(conn_type::POWER_POLE, -1, 5, false).unwrap();
    let d = deal(&w, &sim, E1, W, flags);
    w.clear();
    (w, sim, c, d)
}

fn run(w: &World, sim: &NeighborsSimulator, day: u32) {
    w.today.set(day);
    w.clear();
    sim.simulate_deals();
}

#[test]
fn test_06_life_cycle_grace_then_probation_then_removal() {
    let (w, sim, c, d) = life_cycle(flag::PROPOSED);
    // A proposed deal activates on the first run: status 0 (Active set),
    // 0 (Proposed cleared) from Activate's SetFlags calls (NB2), then 1.
    run(&w, &sim, TODAY);
    assert_eq!(d.flags(), flag::ACTIVE);
    assert_eq!(d.current_date(), TODAY);
    assert_eq!(
        w.statuses(&d),
        vec![status::CHANGED, status::CHANGED, status::ACTIVATED]
    );

    // Losing the connection: Grace, suspension = today, no removal.
    c.set_active(false);
    run(&w, &sim, TODAY + 10);
    assert_eq!(d.flags(), flag::ACTIVE | flag::GRACE);
    assert_eq!(d.suspension_date(), TODAY + 10);
    assert_eq!(sim.find_deals_all().len(), 1);
    assert!(w.statuses(&d).iter().all(|&s| s == status::CHANGED));

    // Failures during Grace are ignored.
    run(&w, &sim, TODAY + 99);
    assert_eq!(d.flags(), flag::ACTIVE | flag::GRACE);
    assert!(w.statuses(&d).is_empty());

    // After 90 days: Probation.
    run(&w, &sim, TODAY + 100);
    assert_eq!(d.flags(), flag::ACTIVE | flag::PROBATION);

    // A failure during Probation removes it with status 3.
    run(&w, &sim, TODAY + 101);
    assert!(sim.find_deals_all().is_empty());
    assert_eq!(w.statuses(&d), vec![status::CHANGED, status::NO_CONNECTION]);
    // SetAmount(0) is ignored in Probation (§16 quirk 3): no sent report.
    assert_eq!(w.count(|e| matches!(e, Ev::Status { sent: true, .. })), 0);
}

#[test]
fn test_06_life_cycle_probation_clears_after_180_days() {
    let (w, sim, _c, d) = life_cycle(flag::ACTIVE | flag::PROBATION);
    d.set_suspension_date(TODAY);
    run(&w, &sim, TODAY + 179);
    assert_eq!(d.flags(), flag::ACTIVE | flag::PROBATION);
    run(&w, &sim, TODAY + 180);
    assert_eq!(d.flags(), flag::ACTIVE);
    assert_eq!(w.statuses(&d), vec![status::CHANGED]);

    // A Grace deal: 90 days to Probation, 90 more to clear.
    let (w, sim, _c, d) = life_cycle(flag::ACTIVE | flag::GRACE);
    d.set_suspension_date(TODAY);
    run(&w, &sim, TODAY + 90);
    assert_eq!(d.flags(), flag::ACTIVE | flag::PROBATION);
    run(&w, &sim, TODAY + 180);
    assert_eq!(d.flags(), flag::ACTIVE);
}

#[test]
fn test_06_life_cycle_cancelled_deal_is_removed() {
    let (w, sim, _c, d) = life_cycle(flag::ACTIVE | flag::CANCELLED);
    run(&w, &sim, TODAY);
    assert!(sim.find_deals_all().is_empty());
    let statuses = w.statuses(&d);
    assert_eq!(statuses[0], status::CANCELLED);
    // SetAmount(0) after the report: sent, then UpdateCost's posted report.
    assert_eq!(
        w.events()
            .into_iter()
            .filter(|e| matches!(e, Ev::Status { deal: Some(_), .. }))
            .map(|e| match e {
                Ev::Status { status, sent, .. } => (status, sent),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>(),
        vec![(status::CANCELLED, false), (0, true), (0, false)]
    );
}

#[test]
fn test_06_life_cycle_expiry_and_renewal() {
    let (w, sim, _c, d) = life_cycle(flag::ACTIVE);
    d.set_start_date(TODAY - 365);
    d.set_expiration_date(TODAY);
    w.set_budget(W, 1e9);
    // Expiry posts 6.
    run(&w, &sim, TODAY);
    assert_eq!(d.flags(), flag::ACTIVE | flag::EXPIRED);
    assert_eq!(w.statuses(&d), vec![status::CHANGED, status::EXPIRED]);
    // The next run renews with the same span (status 7).
    run(&w, &sim, TODAY + 3);
    assert_eq!(d.flags(), flag::ACTIVE);
    assert_eq!(
        (d.current_date(), d.start_date(), d.expiration_date()),
        (TODAY + 3, TODAY + 3, TODAY + 3 + 365)
    );
    assert_eq!(*w.statuses(&d).last().unwrap(), status::RENEWED);
    assert_eq!(d.cost(), 1);

    // Or flags CannotAfford when the payer (the neighbour, for a sell deal)
    // cannot pay the updated cost.
    let (w, sim, _c, d) = life_cycle(flag::ACTIVE | flag::EXPIRED);
    d.set_start_date(TODAY - 365);
    d.set_expiration_date(TODAY);
    w.set_budget(W, 0.0);
    run(&w, &sim, TODAY + 3);
    assert_eq!(d.flags(), flag::ACTIVE | flag::CANNOT_AFFORD);
    assert!(!w.statuses(&d).contains(&status::RENEWED));
    assert_eq!(d.expiration_date(), TODAY);
    // The run after that is the first strike.
    run(&w, &sim, TODAY + 4);
    assert_eq!(d.flags(), flag::ACTIVE | flag::GRACE);
}

#[test]
fn test_06_buy_deal_cannot_afford_and_neither_proposed_nor_active() {
    // A buy deal: R's budget below the cost sets CannotAfford (step 1).
    let (w, sim, _c, d) = life_cycle(flag::ACTIVE);
    d.set_data(DealData {
        deal_type: E0,
        cost: 10,
        ..d.data()
    });
    w.set_budget(R, 5.0);
    *w.figures.borrow_mut() = UtilityFigures {
        total_funds: 5.0,
        ..UtilityFigures::default()
    };
    run(&w, &sim, TODAY);
    assert_eq!(d.flags(), flag::ACTIVE | flag::GRACE);
    assert_eq!(d.suspension_date(), TODAY);

    // Neither Proposed nor Active: Cancel.
    let (w, sim, _c, d) = life_cycle(0);
    w.deliver_sent.set(false);
    run(&w, &sim, TODAY);
    assert_eq!(d.flags(), flag::CANCELLED);
    // The next run removes it with status 2.
    run(&w, &sim, TODAY + 1);
    assert!(sim.find_deals_all().is_empty());
    assert_eq!(w.statuses(&d)[0], status::CANCELLED);
}

/// `Activate` (§5.4, NB2): three calls through `SetFlags` — (Active, true),
/// (Proposed, false), (Cancelled, false) — each posting status 0 when it
/// changes a non-Silent deal's flags. Cancelled is not in `0x38`.
#[test]
fn activate_goes_through_set_flags() {
    let posted = |w: &World, d: &Rc<NeighborDeal>| -> Vec<(u32, u32, bool)> {
        w.events()
            .into_iter()
            .filter_map(|e| match e {
                Ev::Status {
                    deal,
                    status,
                    flags,
                    sent,
                } if deal == Some(ptr(d)) => Some((status, flags, sent)),
                _ => None,
            })
            .collect()
    };
    // A cancelled proposal: three changes, three posted status 0 reports.
    let (w, _sim, _c, d) = life_cycle(flag::PROPOSED | flag::CANCELLED);
    assert!(d.activate());
    assert_eq!(d.flags(), flag::ACTIVE);
    let p = flag::PROPOSED;
    let (a, c) = (flag::ACTIVE, flag::CANCELLED);
    assert_eq!(
        posted(&w, &d),
        [
            (status::CHANGED, a | p | c, false),
            (status::CHANGED, a | c, false),
            (status::CHANGED, a, false)
        ]
    );
    // Not cancelled: the third call changes nothing and posts nothing.
    let (w, _sim, _c, d) = life_cycle(flag::PROPOSED);
    assert!(d.activate());
    assert_eq!(
        posted(&w, &d),
        [(status::CHANGED, a | p, false), (status::CHANGED, a, false)]
    );
    // Silent: the flags change, nothing is posted.
    let (w, _sim, _c, d) = life_cycle(flag::PROPOSED | flag::SILENT);
    assert!(d.activate());
    assert_eq!(d.flags(), flag::ACTIVE | flag::SILENT);
    assert!(posted(&w, &d).is_empty());
    // A failure flag or no Proposed: false, nothing changes.
    for f in [flag::PROPOSED | flag::NO_CONNECTION, flag::ACTIVE] {
        let (w, _sim, _c, d) = life_cycle(f);
        assert!(!d.activate());
        assert_eq!(d.flags(), f);
        assert!(posted(&w, &d).is_empty());
    }
}

#[test]
fn silent_deals_are_not_simulated_and_ui_disable_skips_the_deals() {
    let (w, sim, _c, d) = life_cycle(flag::PROPOSED | flag::SILENT);
    run(&w, &sim, TODAY);
    assert_eq!(d.flags(), flag::PROPOSED | flag::SILENT);
    assert!(w.statuses(&d).is_empty());

    let (w, sim, _c, d) = life_cycle(flag::PROPOSED);
    sim.set_ui_disable_flag(true);
    run(&w, &sim, TODAY);
    assert_eq!(d.flags(), flag::PROPOSED);
    // Stats are still updated and pushed.
    assert!(w.count(|e| matches!(e, Ev::History(..))) > 0);
    sim.set_ui_disable_flag(false);
    run(&w, &sim, TODAY);
    assert_eq!(d.flags(), flag::ACTIVE);

    // Not ready: nothing at all.
    let (w, sim) = setup(true);
    let d = deal(&w, &sim, E1, W, flag::PROPOSED);
    w.clear();
    sim.simulate_deals();
    assert!(w.events().is_empty());
    assert_eq!(d.flags(), flag::PROPOSED);
}

// ---- 7. Garbage capacity -----------------------------------------------------------------------------

fn garbage_world(first_amount: i32) -> (Rc<World>, Rc<NeighborsSimulator>, Rc<NeighborDeal>, Rc<NeighborDeal>) {
    let (w, sim) = running();
    sim.create_connection(conn_type::ROAD, -1, 5, false).unwrap();
    // Produced(3) = 50 after UpdateCommodityStats; no free landfill.
    *w.figures.borrow_mut() = UtilityFigures {
        garbage_recycled: 50.0,
        ..UtilityFigures::default()
    };
    w.set_stat(R, Stat::Exported, 3, 100.0);
    let mk = |amount: i32| {
        let d = deal(&w, &sim, E5, W, flag::ACTIVE | flag::PROBATION);
        d.set_data(DealData {
            amount,
            suspension_date: TODAY,
            ..d.data()
        });
        d
    };
    let a = mk(first_amount);
    let b = mk(60);
    w.clear();
    (w, sim, a, b)
}

#[test]
fn test_07_garbage_capacity() {
    let (w, sim, a, b) = garbage_world(60);
    run(&w, &sim, TODAY);
    // The first gets NoLandfill (removed: it is in Probation) and reduces the
    // export budget to 40; the second is then unflagged.
    assert_eq!(w.statuses(&a), vec![status::CHANGED, status::NO_LANDFILL]);
    let flagged = w.events().iter().any(
        |e| matches!(e, Ev::Status { deal: Some(p), flags, .. } if *p == ptr(&a) && flags & flag::NO_LANDFILL != 0),
    );
    assert!(flagged);
    assert!(w.statuses(&b).is_empty());
    assert_eq!(b.flags(), flag::ACTIVE | flag::PROBATION);
    let left = sim.find_deals_all();
    assert_eq!(left.len(), 1);
    assert!(Rc::ptr_eq(&left[0], &b));

    // With a first deal of 40 the budget only falls to 60: both are flagged.
    let (w, sim, a, b) = garbage_world(40);
    run(&w, &sim, TODAY);
    assert_eq!(w.statuses(&a), vec![status::CHANGED, status::NO_LANDFILL]);
    assert_eq!(w.statuses(&b), vec![status::CHANGED, status::NO_LANDFILL]);
    assert!(sim.find_deals_all().is_empty());
}

// ---- 8. Totals ---------------------------------------------------------------------------------------

#[test]
fn test_08_totals() {
    let (w, sim) = setup(true);
    let mut deals = Vec::new();
    for (t, cost) in [(E0, 10), (E2, 20), (E4, 40), (E1, 100), (E3, 200), (E5, 400)] {
        let d = deal(&w, &sim, t, W, flag::ACTIVE);
        d.set_data(DealData { cost, ..d.data() });
        deals.push(d);
    }
    assert_eq!(sim.total_monthly_expense(), 70);
    assert_eq!(sim.total_monthly_income(), 700);
    for f in [
        flag::PROPOSED,
        flag::CANCELLED,
        flag::NO_CONNECTION,
        flag::CANNOT_AFFORD,
        flag::NO_LANDFILL,
    ] {
        deals[0].set_data(DealData {
            flags: flag::ACTIVE | f,
            ..deals[0].data()
        });
        deals[3].set_data(DealData {
            flags: flag::ACTIVE | f,
            ..deals[3].data()
        });
        assert_eq!(sim.total_monthly_expense(), 60, "flag {f:#x}");
        assert_eq!(sim.total_monthly_income(), 600, "flag {f:#x}");
    }
    for f in [flag::EXPIRED, flag::SILENT, flag::GRACE, flag::PROBATION] {
        deals[0].set_data(DealData {
            flags: flag::ACTIVE | f,
            ..deals[0].data()
        });
        deals[3].set_data(DealData {
            flags: flag::ACTIVE | f,
            ..deals[3].data()
        });
        assert_eq!(sim.total_monthly_expense(), 70, "flag {f:#x}");
        assert_eq!(sim.total_monthly_income(), 700, "flag {f:#x}");
    }
}

// ---- 9. Re-entrancy ----------------------------------------------------------------------------------

#[test]
fn test_09_reentrancy() {
    // A deal that is neither Proposed nor Active is cancelled during the run;
    // Cancel's SetAmount sends a status report, delivered at once.
    let (w, sim, _c, d) = life_cycle(0);
    run(&w, &sim, TODAY);
    let ev = w.events();
    let sent = ev
        .iter()
        .position(|e| matches!(e, Ev::Status { sent: true, .. }))
        .expect("a sent report");
    // The nested run posts the retry (data1 = 8, no deal) and nothing else.
    assert_eq!(
        ev[sent + 1],
        Ev::Status {
            deal: None,
            status: status::RETRY,
            flags: 0,
            sent: false
        }
    );
    assert_eq!(w.count(|e| matches!(e, Ev::Status { deal: None, .. })), 1);
    assert_eq!(
        ev[sent + 2],
        Ev::Status {
            deal: Some(ptr(&d)),
            status: status::CHANGED,
            flags: flag::CANCELLED,
            sent: false
        }
    );
    assert!(!sim.state().busy);

    // Status 0 / 8 set statsDirty, the others dealsDirty.
    let (_w, sim) = setup(true);
    for data1 in 0..=8 {
        sim.set_dirty(false, false);
        sim.do_message(&NeighborsMessage {
            msg_type: msg::DEAL_STATUS,
            data1,
            ..NeighborsMessage::default()
        });
        let s = sim.state();
        let stats = data1 == 0 || data1 == 8;
        assert_eq!((s.stats_dirty, s.deals_dirty), (stats, !stats), "data1 {data1}");
    }

    // A delivered status report runs the deal simulation.
    let (w, sim, _c, d) = life_cycle(flag::PROPOSED);
    sim.do_message(&NeighborsMessage {
        msg_type: msg::DEAL_STATUS,
        data1: 1,
        ..NeighborsMessage::default()
    });
    assert_eq!(d.flags(), flag::ACTIVE);
    assert_eq!(
        w.statuses(&d),
        vec![status::CHANGED, status::CHANGED, status::ACTIVATED]
    );
}

// ---- 10. Proposals -----------------------------------------------------------------------------------

/// A running simulator with an active power-pole connection to `W` (only E1
/// can be proposed) and figures that make our power cost per unit `cpu`.
fn proposal_world(cpu_expenses: f64) -> (Rc<World>, Rc<NeighborsSimulator>) {
    let (w, sim) = running();
    sim.create_connection(conn_type::POWER_POLE, -1, 5, false).unwrap();
    *w.figures.borrow_mut() = UtilityFigures {
        power_production: 5000.0,
        power_requirement: 1000.0,
        power_department_expenses: Some(cpu_expenses),
        ..UtilityFigures::default()
    };
    w.set_stat(W, Stat::Demanded, 1, 2000.0);
    w.clear();
    (w, sim)
}

#[test]
fn test_10_proposals() {
    // Cost 0 (our cost per unit 0): only the minimum budget matters.
    let (w, sim) = proposal_world(0.0);
    w.set_budget(W, 5000.0);
    assert!(sim.propose_deals(false).is_empty());
    w.set_budget(W, 5000.5);
    w.today.set(TODAY + 7);
    let list = sim.propose_deals(false);
    assert_eq!(list.len(), 1);
    let d = &list[0];
    assert_eq!(d.deal_type(), E1);
    assert_eq!(d.other_side(), (2, 3));
    assert_eq!(d.city_side(), (3, 3));
    assert_eq!((d.amount(), d.cost()), (0, 1));
    assert_eq!(
        (d.current_date(), d.start_date(), d.expiration_date()),
        (TODAY + 7, TODAY + 7, TODAY + 7 + 365)
    );
    assert_eq!(d.flags(), flag::PROPOSED | flag::SILENT);
    // Not added.
    assert!(sim.find_deals_all().is_empty());
    assert!(w.statuses(d).is_empty());

    // A real cost: the budget must exceed 60 × cost.
    let (w, sim) = proposal_world(5001.0);
    sim.update_commodity_stats(Some(CITY));
    let cost = sim.get_deal_cost(E1, 1000.0, Some(W));
    assert!(cost > 1000.0, "{cost}");
    w.set_budget(W, 60.0 * cost);
    assert!(sim.propose_deals(false).is_empty());
    w.set_budget(W, 60.0 * cost + 1.0);
    assert_eq!(sim.propose_deals(false).len(), 1);

    // Added: the deal joins the list; a second round proposes nothing
    // because a deal of that type exists.
    let (w, sim) = proposal_world(0.0);
    w.set_budget(W, 1e9);
    let list = sim.propose_deals(true);
    assert_eq!(list.len(), 1);
    let all = sim.find_deals(E1);
    assert_eq!(all.len(), 1);
    assert!(Rc::ptr_eq(&all[0], &list[0]));
    assert!(!all[0].is_silent());
    assert_eq!((all[0].amount(), all[0].cost()), (0, 1));
    assert!(sim.propose_deals(true).is_empty());

    // Not ready: nothing.
    let (w, sim) = setup(true);
    w.set_budget(W, 1e9);
    sim.create_connection(conn_type::POWER_POLE, -1, 5, false).unwrap();
    assert!(sim.propose_deals(false).is_empty());
}

#[test]
fn test_10_proposal_rng_and_term() {
    // Terms between 1 and 3 years: floor(uniform(1, 3.001)).
    let (w, sim) = proposal_world(0.0);
    let mut h = shipped_tuning();
    h.add(
        prop::MAX_TERM_YEARS,
        crate::variant::Variant::Scalar(crate::variant::Elem::F32(3.0)),
    );
    sim.set_tuning(Some(Rc::new(h)));
    w.set_budget(W, 1e9);
    assert_eq!(sim.proposal_rng_state(), None);
    let d = sim.propose_deals(false).remove(0);
    let seed = crate::rng::derive_seed(0x1234_5678, PROPOSAL_SEED_KEY);
    let mut rng = crate::rng::RzRandom::new(
        seed,
        &crate::rng::FixedSeeds {
            wall_clock: seed,
            process_clock: seed,
        },
    );
    let y = rng.range_f64(1.0, 3.0 + 0.001).floor();
    assert_eq!(d.expiration_date(), TODAY + 365 * y as u32);
    assert_eq!(sim.proposal_rng_state(), Some((rng.lcg_state(), rng.mul_state())));
}

// ---- 11. Clone ---------------------------------------------------------------------------------------

#[test]
fn test_11_clone() {
    let (w, sim) = setup(true);
    let src = DealData {
        deal_type: E0,
        city_side: w.pos(W),
        other_side: w.pos(R),
        amount: 77,
        cost: 123,
        flags: flag::ACTIVE | flag::SILENT,
        start_date: 5,
        expiration_date: 370,
        current_date: 6,
        suspension_date: 2,
        inc_amount: 3,
        inc_amount2: 9,
        last_amount: 4,
        last_inc_amount: 8,
        amount_used: 11,
        ..DealData::default()
    };
    // Seen from the neighbour: swapped, E0 ↔ E1.
    let e = sim.clone_deal_data(&src, false);
    assert_eq!(e.deal_type(), E1);
    assert_eq!(e.city_side(), w.pos(R));
    assert_eq!(e.other_side(), w.pos(W));
    assert_eq!(e.flags(), flag::ACTIVE);
    assert_eq!(
        (e.amount(), e.cost(), e.amount_used(), e.inc_amount(), e.last_amount()),
        (77, 123, 11, 3, 4)
    );
    // Copy leaves incAmount2 and lastIncAmount.
    assert_eq!((e.inc_amount2(), e.last_inc_amount()), (0, 0));
    assert_eq!(
        (
            e.start_date(),
            e.expiration_date(),
            e.current_date(),
            e.suspension_date()
        ),
        (5, 370, 6, 2)
    );
    assert_eq!(e.other_city(), Some(W));
    // Seen from us: unchanged.
    let mine = DealData {
        city_side: w.pos(R),
        other_side: w.pos(W),
        ..src.clone()
    };
    let e = sim.clone_deal_data(&mine, true);
    assert_eq!((e.deal_type(), e.city_side()), (E0, w.pos(R)));
    assert!(e.is_silent());
    // Missing region cells: Cancelled.
    for (cs, os) in [((50, 50), w.pos(R)), (w.pos(W), (-20, 9))] {
        let e = sim.clone_deal_data(
            &DealData {
                city_side: cs,
                other_side: os,
                ..src.clone()
            },
            false,
        );
        assert!(e.check_flags_and(flag::CANCELLED));
        assert_eq!((e.city_side(), e.other_side(), e.deal_type()), (cs, os, E0));
    }
    // The Silent flag follows the argument.
    let d = deal(&w, &sim, E3, W, flag::ACTIVE);
    assert!(d.clone_deal(true).is_silent());
    assert!(!d.clone_deal(false).is_silent());
    // A clone with silent = false reports its flag change (posted).
    w.clear();
    let e = d.clone_deal(false);
    assert!(w.statuses(&e).is_empty(), "the source was not Silent");
    let s = sim.clone_deal_data(&src, false);
    assert_eq!(w.statuses(&s), vec![status::CHANGED]);
}

// ---- 12. Deal record ---------------------------------------------------------------------------------

fn deal_bytes(d: &NeighborDeal) -> Vec<u8> {
    let mut out = ByteWriter::new();
    assert!(d.write(&mut out));
    out.into_bytes()
}

#[test]
fn test_12_deal_record() {
    let mut props = PropertyHolder::new();
    props.add(
        0x0BAD_F00D,
        crate::variant::Variant::Scalar(crate::variant::Elem::F32(1.5)),
    );
    let d = NeighborDeal::from_data(DealData {
        deal_type: E4,
        other_side: (2, 3),
        city_side: (3, 3),
        amount: -5,
        inc_amount: 6,
        inc_amount2: 7,
        last_amount: 8,
        last_inc_amount: 9,
        amount_used: 10,
        cost: -0x1_0000_0001,
        start_date: 2_451_545,
        expiration_date: 2_451_910,
        current_date: 2_451_600,
        suspension_date: 2_451_580,
        flags: 0x302,
        props,
    });
    let bytes = deal_bytes(&d);
    // u16 5, then the fields in record order.
    let mut r = ByteReader::new(&bytes);
    assert_eq!(r.get_u16(), Some(5));
    assert_eq!(r.get_u32(), Some(E4));
    assert_eq!((r.get_i32(), r.get_i32()), (Some(2), Some(3)));
    assert_eq!(r.get_i32(), Some(-5));
    assert_eq!(r.get_i32(), Some(10));
    assert_eq!(r.get_i64(), Some(-0x1_0000_0001));
    assert_eq!(r.get_i32(), Some(0x302));
    assert_eq!((r.get_i32(), r.get_i32()), (Some(3), Some(3)));
    assert_eq!(r.get_i32(), Some(6));
    assert_eq!(r.get_i32(), Some(8));
    assert_eq!(r.get_i32(), Some(10));
    assert_eq!(
        (r.get_u32(), r.get_u32(), r.get_u32()),
        (Some(2_451_600), Some(2_451_545), Some(2_451_910))
    );
    let mut p = PropertyHolder::new();
    assert!(p.read(&mut r));
    assert_eq!(r.get_u32(), Some(2_451_580));
    assert_eq!(r.remaining(), 0);

    // write → read → write is byte-identical.
    let back = NeighborDeal::new();
    assert!(back.read(&mut ByteReader::new(&bytes)));
    assert_eq!(deal_bytes(&back), bytes);
    // incAmount2 and lastIncAmount are not saved.
    assert_eq!((back.inc_amount2(), back.last_inc_amount()), (0, 0));

    // The second amountUsed is the one kept.
    let mut twice = bytes.clone();
    twice[10..14].copy_from_slice(&99i32.to_le_bytes());
    let t = NeighborDeal::new();
    assert!(t.read(&mut ByteReader::new(&twice)));
    assert_eq!(t.amount_used(), 10);

    // Version 4: no suspension date.
    let mut v4 = bytes[..bytes.len() - 4].to_vec();
    v4[0] = 4;
    let old = NeighborDeal::new();
    let mut r = ByteReader::new(&v4);
    assert!(old.read(&mut r));
    assert_eq!(r.remaining(), 0);
    assert_eq!(old.suspension_date(), DEFAULT_DATE);
    assert_eq!(old.current_date(), 2_451_600);
    assert_eq!(old.flags(), 0x302);
    // Any version is accepted (no check).
    let mut v9 = bytes.clone();
    v9[0] = 9;
    assert!(NeighborDeal::new().read(&mut ByteReader::new(&v9)));
}

// ---- lifecycle and messages (§14, §11.1) -------------------------------------------------------------

/// §14.1 step 5 *[compat]*: `ReloadCity` replaces each adjacent city's
/// object; steps 6-8 use only the cities asked for again after the reloads.
#[test]
fn init_uses_the_city_objects_made_by_reload_city() {
    let (w, sim) = setup(true);
    w.city(N, |c| c.established = false);
    w.city(W, |c| {
        c.local.push(DealData {
            deal_type: E1,
            city_side: (1, 3),
            other_side: (3, 3),
            amount: 40,
            flags: flag::ACTIVE | flag::SILENT,
            start_date: 800,
            expiration_date: 1165,
            current_date: 900,
            suspension_date: 850,
            ..DealData::default()
        });
    });
    w.reload_replaces.set(true);
    assert!(sim.init());
    let rest: Vec<Ev> = w
        .events()
        .into_iter()
        .filter(|e| matches!(e, Ev::Reload(_) | Ev::UpdateLocal(_)))
        .collect();
    let (w2, s2, e2) = (W + 0x100, S + 0x100, E + 0x100);
    assert_eq!(
        rest,
        vec![
            Ev::Reload(W),
            Ev::Reload(S),
            Ev::Reload(E),
            Ev::UpdateLocal(R),
            Ev::UpdateLocal(e2),
            Ev::UpdateLocal(s2),
            Ev::UpdateLocal(w2),
        ]
    );
    // W's deal came from the new W object.
    let deals = sim.find_deals_all();
    assert_eq!(deals.len(), 1);
    assert_eq!(deals[0].other_side(), (1, 3));
    // Step 8 reaches the new objects only.
    let old = [W, S, E];
    assert!(!w.events().iter().any(|e| match e {
        Ev::SetLocal(c, _) | Ev::UpdateImportExport(c) => old.contains(c),
        _ => false,
    }));
}

#[test]
fn init_registers_subscribes_and_rebuilds_the_deal_list() {
    let (w, sim) = setup(true);
    // An unestablished neighbour is skipped.
    w.city(N, |c| c.established = false);
    // W holds a deal it sells us, Silent as the region marks them; dates
    // 100 days behind today.
    w.city(W, |c| {
        c.local.push(DealData {
            deal_type: E1,
            city_side: (1, 3),
            other_side: (3, 3),
            amount: 40,
            flags: flag::ACTIVE | flag::SILENT,
            start_date: 800,
            expiration_date: 1165,
            current_date: 900,
            suspension_date: 850,
            ..DealData::default()
        });
        // A deal with a city that no longer exists: Cancelled clone, other
        // side missing, not added.
        c.local.push(DealData {
            deal_type: E2,
            city_side: (1, 3),
            other_side: (40, 40),
            ..DealData::default()
        });
    });
    let conn = NeighborConnection::with(conn_type::ROAD, 64, 70);
    // A stale other-side cell, repaired by Init (E side, z 70 → region z 4).
    conn.set_other_region_cell(0, 0);
    let mut res = Resolver::default();
    res.made.push((1, conn.clone()));
    let mut out = ByteWriter::new();
    out.put_u16(2);
    out.put_u32(1);
    out.put_u32(1);
    out.put_u32(CLSID_NEIGHBOR_CONNECTION);
    let rec = out.into_bytes();
    assert!(sim.read(&mut ByteReader::with_resolver(&rec, &mut res)));
    let stale = deal(&w, &sim, E0, W, flag::ACTIVE);

    assert!(sim.init());
    assert_eq!(conn.other_region_cell(), (4, 4));
    let ev = w.events();
    assert_eq!(
        &ev[..10],
        &[
            Ev::Agent(6, AGENT_NAME.to_string(), 2),
            Ev::Agent(4, AGENT_NAME.to_string(), 2),
            Ev::Agent(10, AGENT_NAME.to_string(), 2),
            Ev::Subscribe(0xAA7A_014D),
            Ev::Subscribe(0x278E_6003),
            Ev::Subscribe(0xEA41_8D8F),
            Ev::Subscribe(0x6AC2_84F3),
            Ev::Subscribe(0x99EF_1143),
            Ev::Subscribe(0x26D3_1EC5),
            Ev::Subscribe(0x26D3_1EC4),
        ]
    );
    // Adjacent established cities by position: W (1,3), S (3,4), E (4,2).
    assert_eq!(sim.adjacent_cities(false), vec![W, S, E]);
    assert_eq!(sim.adjacent_cities(true), vec![W, N, S, E]);
    let rest: Vec<Ev> = ev[10..]
        .iter()
        .filter(|e| matches!(e, Ev::Reload(_) | Ev::UpdateLocal(_)))
        .cloned()
        .collect();
    assert_eq!(
        rest,
        vec![
            Ev::Reload(W),
            Ev::Reload(S),
            Ev::Reload(E),
            Ev::UpdateLocal(R),
            // Reverse list order.
            Ev::UpdateLocal(E),
            Ev::UpdateLocal(S),
            Ev::UpdateLocal(W),
        ]
    );
    // The old list was cleared; W's deal came back seen from us, not Silent,
    // its dates shifted by today − currentDate = 100.
    let deals = sim.find_deals_all();
    assert_eq!(deals.len(), 1);
    assert!(!deals.iter().any(|d| Rc::ptr_eq(d, &stale)));
    let d = &deals[0];
    assert_eq!(d.deal_type(), E0);
    assert_eq!((d.city_side(), d.other_side()), ((3, 3), (1, 3)));
    assert_eq!(d.flags(), flag::ACTIVE);
    assert_eq!(
        (
            d.start_date(),
            d.expiration_date(),
            d.current_date(),
            d.suspension_date()
        ),
        (900, 1265, TODAY, 950)
    );
    // Step 8: the push (adjacent in reverse, then R), both flags.
    let pushes: Vec<Ev> = ev
        .iter()
        .filter(|e| matches!(e, Ev::SetLocal(..) | Ev::UpdateImportExport(_)))
        .cloned()
        .collect();
    assert_eq!(
        pushes,
        vec![
            Ev::SetLocal(E, 1),
            Ev::UpdateImportExport(E),
            Ev::SetLocal(S, 1),
            Ev::UpdateImportExport(S),
            Ev::SetLocal(W, 1),
            Ev::UpdateImportExport(W),
            Ev::SetLocal(R, 1),
            Ev::UpdateImportExport(R),
        ]
    );
    let s = sim.state();
    assert!(s.initialized && !s.deals_dirty && !s.stats_dirty && !s.ready);

    // A second Init does nothing.
    w.clear();
    assert!(sim.init());
    assert!(w.events().is_empty());
}

#[test]
fn shutdown_unsubscribes_removes_agents_and_clears() {
    let (w, sim) = running();
    sim.create_connection(conn_type::ROAD, -1, 5, false).unwrap();
    deal(&w, &sim, E1, W, flag::ACTIVE);
    w.clear();
    assert!(sim.shutdown());
    let mut expected: Vec<Ev> = msg::SUBSCRIPTIONS.iter().map(|&m| Ev::Unsubscribe(m)).collect();
    expected.push(Ev::RemoveAgents);
    assert_eq!(w.events(), expected);
    assert!(sim.all_connections().is_empty());
    assert!(sim.find_deals_all().is_empty());
    assert!(sim.tuning_exemplar().is_none());
    assert!(!sim.state().initialized);
    w.clear();
    assert!(sim.shutdown());
    assert!(w.events().is_empty());
}

#[test]
fn query_interface_answers_four_iids() {
    for iid in [
        IID_UNKNOWN,
        IID_NEIGHBORS_SIMULATOR,
        IID_MESSAGE_TARGET2,
        IID_SERIALIZABLE,
    ] {
        assert!(answers_iid(iid));
    }
    assert!(!answers_iid(IID_NEIGHBOR_DEAL));
    assert!(!answers_iid(IID_NEIGHBOR_CONNECTION));
}

#[test]
fn initial_state() {
    let (_w, sim) = setup(false);
    assert_eq!(
        sim.state(),
        SimState {
            initialized: false,
            ready: false,
            deals_dirty: true,
            stats_dirty: false,
            ui_disabled: false,
            busy: false,
            connections: 0,
            deals: 0,
        }
    );
    let d = NeighborDeal::new();
    assert_eq!(d.flags(), 0x81);
    assert_eq!(d.cost(), 0);
}

#[test]
fn messages_dispatch() {
    let (w, sim) = setup(true);
    assert!(sim.init());
    // ConnectionsReady.
    assert!(!sim.state().ready);
    sim.do_message(&NeighborsMessage::of_type(msg::CONNECTIONS_READY));
    assert!(sim.state().ready);

    // SimNewDay / SimIdle push with the current dirty flags.
    for m in [msg::SIM_NEW_DAY, msg::SIM_IDLE] {
        sim.set_dirty(false, false);
        w.clear();
        sim.do_message(&NeighborsMessage::of_type(m));
        assert!(w.events().is_empty());
        sim.set_dirty(false, true);
        sim.do_message(&NeighborsMessage::of_type(m));
        assert_eq!(w.count(|e| matches!(e, Ev::SetLocal(..))), 0);
        assert_eq!(w.count(|e| matches!(e, Ev::UpdateImportExport(_))), 5);
        assert!(!sim.state().stats_dirty);
    }

    // CityEstablished / CityObliterated clear the list and push.
    for m in [msg::CITY_ESTABLISHED, msg::CITY_OBLITERATED] {
        deal(&w, &sim, E1, W, flag::ACTIVE);
        w.clear();
        sim.do_message(&NeighborsMessage::of_type(m));
        assert!(sim.find_deals_all().is_empty());
        assert_eq!(w.count(|e| matches!(e, Ev::SetLocal(_, 0))), 5);
    }

    // PreDemolishNetworkOccupant and RemoveOccupant.
    sim.create_connection(conn_type::ROAD, 64, 5, false).unwrap();
    sim.create_connection(conn_type::POWER_POLE, 64, 6, false).unwrap();
    w.occupants.borrow_mut().insert(
        1,
        Occ {
            kind: occupant::NETWORK,
            networks: vec![0],
            cell: (63, 5),
            pole_flag: false,
        },
    );
    w.occupants.borrow_mut().insert(
        2,
        Occ {
            kind: occupant::POWER_POLE,
            networks: vec![],
            cell: (63, 6),
            pole_flag: true,
        },
    );
    sim.do_message(&NeighborsMessage {
        msg_type: msg::PRE_DEMOLISH_NETWORK_OCCUPANT,
        occupant: Some(1),
        ..NeighborsMessage::default()
    });
    assert!(!sim.has_connection(conn_type::ROAD, 64, 5));
    // RemoveOccupant: a pole whose flag is true is kept; false deletes.
    let remove = NeighborsMessage {
        msg_type: msg::REMOVE_OCCUPANT,
        occupant: Some(2),
        ..NeighborsMessage::default()
    };
    sim.do_message(&remove);
    assert!(sim.has_connection(conn_type::POWER_POLE, 64, 6));
    w.occupants.borrow_mut().get_mut(&2).unwrap().pole_flag = false;
    sim.do_message(&remove);
    assert!(!sim.has_connection(conn_type::POWER_POLE, 64, 6));
    // RemoveOccupant ignores network occupants.
    sim.create_connection(conn_type::ROAD, 64, 5, false).unwrap();
    sim.do_message(&NeighborsMessage {
        msg_type: msg::REMOVE_OCCUPANT,
        occupant: Some(1),
        ..NeighborsMessage::default()
    });
    assert!(sim.has_connection(conn_type::ROAD, 64, 5));

    // SimNewMonth runs the deal simulation.
    let d = deal(&w, &sim, E1, E, flag::PROPOSED);
    sim.create_connection(conn_type::POWER_POLE, 64, 7, false).unwrap();
    sim.do_message(&NeighborsMessage::of_type(msg::SIM_NEW_MONTH));
    assert_eq!(d.flags(), flag::ACTIVE);
}

#[test]
fn terrain_heights_changed_moves_the_markers() {
    let (w, sim) = setup(true);
    sim.create_connection(conn_type::ROAD, -1, 5, false).unwrap();
    sim.create_connection(conn_type::HIGHWAY, 64, 9, false).unwrap();
    sim.create_connection(conn_type::RAIL, 20, -1, false).unwrap();
    w.clear();
    // x 0..=63 (data1 = (1 << 16) | 63), z 5..=9 (data2 = (6 << 16) | 9).
    sim.do_message(&NeighborsMessage {
        msg_type: msg::TERRAIN_HEIGHTS_CHANGED,
        data1: (1 << 16) | 63,
        data2: (6 << 16) | 9,
        ..NeighborsMessage::default()
    });
    let moves: Vec<[f32; 3]> = w
        .events()
        .into_iter()
        .filter_map(|e| match e {
            Ev::PropMove(_, p) => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(
        moves,
        vec![[8.0, 280.0, 88.0], [63.0 * 16.0 + 8.0, 295.5, 9.0 * 16.0 + 8.0]]
    );
}

#[test]
fn connection_queries() {
    let (w, sim) = setup(true);
    w.city(N, |c| c.established = false);
    let road_w = sim.create_connection(conn_type::ROAD, -1, 5, false).unwrap();
    let pipe_w = sim.create_connection(conn_type::PIPE, -1, 6, false).unwrap();
    let rail_e = sim.create_connection(conn_type::RAIL, 64, 5, false).unwrap();
    let road_n = sim.create_connection(conn_type::ROAD, 5, -1, false).unwrap();
    assert_eq!(sim.connections_of_type(conn_type::ROAD).len(), 2);
    assert_eq!(sim.connections_to_city(W).len(), 2);
    assert!(Rc::ptr_eq(
        &sim.connections_of_type_to_city(conn_type::ROAD, N)[0],
        &road_n
    ));
    // Transport only; established unless asked.
    assert_eq!(sim.connected_cities(false), vec![W, E]);
    assert_eq!(sim.connected_cities(true), vec![W, E, N]);
    // FindUnvisitedConnection.
    assert!(Rc::ptr_eq(
        &sim.find_unvisited_connection(conn_type::ROAD, W).unwrap(),
        &road_w
    ));
    road_w.set_visited_this_cycle(true);
    assert!(sim.find_unvisited_connection(conn_type::ROAD, W).is_none());
    // GetAllConnectedCities: breadth first through the neighbours' links.
    w.add_city(9, (6, 2), (1, 1));
    w.city(E, |c| c.links.push((conn_type::ROAD, (6, 2))));
    w.city(9, |c| c.links.push((conn_type::PIPE, (8, 8))));
    assert_eq!(sim.all_connected_cities(false), vec![W, E, 9]);
    assert_eq!(sim.all_connected_cities(true), vec![W, E, N, 9]);
    // Suitable for deals: garbage takes the first transport type in order.
    assert!(Rc::ptr_eq(
        &sim.find_connection_suitable_for_deal(E4, W).unwrap(),
        &road_w
    ));
    assert!(
        sim.find_connection_suitable_for_deal(E4, E).is_none(),
        "rail is not searched"
    );
    // Water sell: our pipe must be active; buy: the other side's pipe.
    assert!(Rc::ptr_eq(
        &sim.find_connection_suitable_for_deal(E3, W).unwrap(),
        &pipe_w
    ));
    pipe_w.set_active(false);
    assert!(sim.find_connection_suitable_for_deal(E3, W).is_none());
    assert!(sim.find_connection_suitable_for_deal(E2, W).is_none());
    w.city(W, |c| c.conns.push((conn_type::PIPE, (128, 6), false)));
    assert!(sim.find_connection_suitable_for_deal(E2, W).is_none());
    w.city(W, |c| c.conns[0].2 = true);
    assert!(Rc::ptr_eq(
        &sim.find_connection_suitable_for_deal(E2, W).unwrap(),
        &pipe_w
    ));
    assert!(sim.find_connection_suitable_for_deal(0x1234, W).is_none());
    // DeleteAllConnectionsOfType.
    w.clear();
    sim.delete_all_connections_of_type(conn_type::ROAD);
    assert_eq!(sim.all_connections().len(), 2);
    assert_eq!(w.count(|e| matches!(e, Ev::Conn { .. })), 2);
    let _ = rail_e;
    // RequestCreateConnection.
    assert_eq!(
        sim.request_create_connection(conn_type::RAIL, 64, 5),
        ConnectionRequest::Exists
    );
    assert_eq!(
        sim.request_create_connection(conn_type::ROAD, 64, 5),
        ConnectionRequest::Confirm { neighbour: Some(E) }
    );
    assert_eq!(
        sim.request_create_connection(conn_type::ROAD, 5, -1),
        ConnectionRequest::Confirm { neighbour: None }
    );
}

#[test]
fn add_deal_unsilences_appends_and_pushes() {
    let (w, sim) = running();
    let d = NeighborDeal::from_data(DealData {
        deal_type: E1,
        other_side: (2, 3),
        city_side: (3, 3),
        ..DealData::default()
    });
    assert!(sim.add_deal(&d));
    assert!(!d.is_silent());
    assert_eq!(sim.find_deals_all().len(), 1);
    assert_eq!(w.statuses(&d), vec![status::CHANGED]);
    assert_eq!(w.count(|e| matches!(e, Ev::SetLocal(..))), 5);
    // The region prunes deals with a missing city from our list (foundations
    // §11.3 quirk).
    let lost = NeighborDeal::from_data(DealData {
        deal_type: E1,
        other_side: (30, 30),
        city_side: (3, 3),
        ..DealData::default()
    });
    assert!(sim.add_deal(&lost));
    assert_eq!(sim.find_deals_all().len(), 1);
    // FindDeals by type and city.
    assert_eq!(sim.find_deals(E1).len(), 1);
    assert_eq!(sim.find_deals_with(E1, W).len(), 1);
    assert!(sim.find_deals_with(E1, E).is_empty());
    sim.mark_deals_dirty();
    assert!(sim.state().deals_dirty);
}

#[test]
fn set_amount_rules() {
    let (w, sim) = running();
    let d = deal(&w, &sim, E1, W, flag::ACTIVE);
    w.deliver_sent.set(false);
    w.clear();
    d.set_amount(25);
    assert_eq!(d.amount(), 25);
    assert_eq!(d.cost(), 1);
    assert_eq!(
        w.events()
            .into_iter()
            .filter_map(|e| match e {
                Ev::Status { status, sent, .. } => Some((status, sent)),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![(0, true), (0, false)]
    );
    // Ignored in Grace and Probation, and without a current city.
    for f in [flag::GRACE, flag::PROBATION] {
        d.set_data(DealData {
            flags: flag::ACTIVE | f,
            ..d.data()
        });
        d.set_amount(3);
        assert_eq!(d.amount(), 25);
    }
    d.set_data(DealData {
        flags: flag::ACTIVE,
        ..d.data()
    });
    w.current.set(None);
    d.set_amount(3);
    assert_eq!(d.amount(), 25);
    // A deal with no simulator ignores SetAmount.
    let free = NeighborDeal::new();
    free.set_amount(3);
    assert_eq!(free.amount(), 0);
    // SetFlags: a Grace deal drops the failure flags.
    let g = deal(&w, &sim, E1, W, flag::ACTIVE | flag::GRACE);
    g.set_flags(flag::NO_CONNECTION | flag::EXPIRED, true);
    assert_eq!(g.flags(), flag::ACTIVE | flag::GRACE | flag::EXPIRED);
}

// ---- foreign connections (§15.2) ---------------------------------------------------------------------

/// Resolves every connection reference to a host object `0xA000 + id`.
struct HostResolver;

impl RefResolver for HostResolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: 0xA000 + u64::from(object_id),
            class_id,
        })
    }
}

#[test]
fn test_foreign_connections() {
    let (w, sim) = setup(true);
    // Two original objects, as the class store creates them on load.
    w.foreign.borrow_mut().insert(
        0xA001,
        ConnectionState {
            conn_type: conn_type::ROAD,
            cell: (-1, 5),
            other_region_cell: (0, 0),
            visited: false,
            active: true,
        },
    );
    w.foreign.borrow_mut().insert(
        0xA002,
        ConnectionState {
            conn_type: conn_type::POWER_POLE,
            cell: (64, 5),
            other_region_cell: (4, 3),
            visited: true,
            active: false,
        },
    );
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(reg.clone());
    out.put_u16(2);
    out.put_u32(2);
    for token in [0xA001u64, 0xA002] {
        out.put_ref(&RefValue::Host {
            token,
            class_id: CLSID_NEIGHBOR_CONNECTION,
        });
    }
    let record = out.into_bytes();

    // Read accepts the host objects; their state is read live.
    let mut res = HostResolver;
    assert!(sim.read(&mut ByteReader::with_resolver(&record, &mut res)));
    let all = sim.all_connections();
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|c| c.is_foreign()));
    assert_eq!((all[0].token(), all[1].token()), (0xA001, 0xA002));
    let road = sim.find_connection(conn_type::ROAD, -1, 5).expect("road");
    assert!(road.is_active() && !road.visited_this_cycle());
    let pole = sim.find_connection(conn_type::POWER_POLE, 64, 5).expect("pole");
    assert_eq!(pole.flags(), 3);
    assert_eq!(pole.other_region_cell(), (4, 3));
    // Writes go to the object.
    road.set_visited_this_cycle(true);
    road.set_active(false);
    assert!(w.foreign.borrow()[&0xA001].visited);
    assert!(!w.foreign.borrow()[&0xA001].active);
    assert_eq!(road.flags(), 3);
    // Another simulator changes the object: the core sees it at once.
    w.foreign.borrow_mut().get_mut(&0xA001).unwrap().active = true;
    assert!(road.is_active());

    // Init step 4 repairs each through its interface.
    assert!(sim.init());
    assert_eq!(*w.repaired.borrow(), vec![(0xA001, R), (0xA002, R)]);

    // Write: the same objects, as host references with the connection class.
    let reg2 = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out2 = ByteWriter::with_registry(reg2.clone());
    assert!(sim.write(&mut out2));
    assert_eq!(out2.into_bytes(), record);
    let entries: Vec<(u32, u64, bool)> = reg2
        .borrow()
        .entries
        .iter()
        .map(|(class, _, obj, token)| (*class, *token, obj.is_some()))
        .collect();
    assert_eq!(
        entries,
        vec![
            (CLSID_NEIGHBOR_CONNECTION, 0xA001, false),
            (CLSID_NEIGHBOR_CONNECTION, 0xA002, false)
        ]
    );

    // Deleting releases the list's reference; Shutdown releases the rest.
    sim.delete_connection(conn_type::ROAD, -1, 5);
    assert_eq!(*w.released.borrow(), vec![0xA001]);
    assert!(sim.shutdown());
    assert_eq!(*w.released.borrow(), vec![0xA001, 0xA002]);
    assert!(sim.all_connections().is_empty());
}

#[test]
fn test_own_connection_objects() {
    let (w, sim) = setup(true);
    w.next_object.set(0xB001);
    let c = sim.create_connection(conn_type::RAIL, 64, 5, false).unwrap();
    assert_eq!(c.token(), 0xB001);
    assert!(!c.is_foreign());
    let d = sim.create_connection(conn_type::RAIL, 64, 6, false).unwrap();
    assert_eq!(d.token(), 0xB002);
    sim.delete_all_connections_of_type(conn_type::RAIL);
    assert_eq!(*w.released.borrow(), vec![0xB001, 0xB002]);
    // Our objects are written as our own references (the host object's token).
    let e = sim.create_connection(conn_type::ROAD, -1, 5, false).unwrap();
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(reg.clone());
    assert!(sim.write(&mut out));
    let r = reg.borrow();
    assert_eq!(r.entries.len(), 1);
    assert_eq!(r.entries[0].2.as_ref().map(|o| o.host_token()), Some(e.token()));
}
