// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests of the commercial and industrial simulators against a fake
//! host: the §13 tests of `docs/specs/commercial-industrial.md` (named
//! `ci13_01_…` to `ci13_09_…`), the lifecycle and the records.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use super::*;
use crate::grid::{CityDims, SimGrid, TractGrid};
use crate::stream::{ByteReader, ByteWriter, RefResolver, RefValue, RegisteredObject, SaveObject, SaveRegistry};

/// One call the simulator made on the host.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Ev {
    AddAgent(u32, String, u32),
    RemoveAgent,
    AcquireMessageServer,
    Subscribe(u32),
    Unsubscribe(u32),
    ReleaseMessageServer,
    AcquireCity,
    ReleaseCity,
    AcquireBds,
    ReleaseBds,
    DemandNode(u32),
    ReleaseDemandNode(u64),
    CreateGrid(u32, i32),
    ShutdownGrid(u32),
    BuildingMap,
    MapKey(u32, u32),
    Purpose(u64),
}

struct Host {
    city: CityDims,
    events: RefCell<Vec<Ev>>,
    /// The demand node IDs the demand simulator answers.
    nodes: RefCell<BTreeSet<u32>>,
    has_demand: RefCell<bool>,
    map: RefCell<Option<Rc<SimGrid<u8>>>>,
    /// Occupant token → building purpose (absent: not a building).
    purposes: RefCell<Vec<(u64, u32)>>,
    created: RefCell<Vec<Rc<SimGrid<u8>>>>,
}

impl Host {
    fn new(cells: i32) -> Rc<Host> {
        Rc::new(Host {
            city: CityDims::square(cells),
            events: RefCell::new(Vec::new()),
            nodes: RefCell::new([0x3000, 0x3800].into_iter().collect()),
            has_demand: RefCell::new(true),
            map: RefCell::new(None),
            purposes: RefCell::new(Vec::new()),
            created: RefCell::new(Vec::new()),
        })
    }

    fn ev(&self, e: Ev) {
        self.events.borrow_mut().push(e);
    }

    fn take(&self) -> Vec<Ev> {
        std::mem::take(&mut *self.events.borrow_mut())
    }

    /// An all-zero building map the size of the city, with `key` at `cells`.
    fn set_map(&self, key: u8, cells: &[(i32, i32)]) {
        let m = SimGrid::<u8>::create(&self.city, 0x1234, 1);
        for &(x, z) in cells {
            m.set(x, z, key);
        }
        *self.map.borrow_mut() = Some(m);
    }

    fn instance_of(&self, g: &Rc<dyn TractGrid<u8>>) -> u32 {
        let p = Rc::as_ptr(g) as *const ();
        self.created
            .borrow()
            .iter()
            .find(|c| Rc::as_ptr(c) as *const () == p)
            .map_or(0, |c| c.instance_id())
    }
}

impl GridProvider<u8> for Host {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        self.ev(Ev::CreateGrid(instance, tract_size));
        let g = SimGrid::<u8>::create(&self.city, instance, tract_size);
        self.created.borrow_mut().push(g.clone());
        Some(g)
    }

    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        let g = r.object::<SimGrid<u8>>()?;
        self.created.borrow_mut().push(g.clone());
        Some(g)
    }
}

impl CommercialHost for Host {
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.ev(Ev::AddAgent(level, name.to_string(), flags));
    }
    fn remove_agent(&self) {
        self.ev(Ev::RemoveAgent);
    }
    fn acquire_message_server(&self) -> bool {
        self.ev(Ev::AcquireMessageServer);
        true
    }
    fn subscribe(&self, msg_type: u32) {
        self.ev(Ev::Subscribe(msg_type));
    }
    fn unsubscribe(&self, msg_type: u32) {
        self.ev(Ev::Unsubscribe(msg_type));
    }
    fn release_message_server(&self) {
        self.ev(Ev::ReleaseMessageServer);
    }
    fn acquire_city(&self) -> bool {
        self.ev(Ev::AcquireCity);
        true
    }
    fn release_city(&self) {
        self.ev(Ev::ReleaseCity);
    }
    fn acquire_bds(&self) -> bool {
        self.ev(Ev::AcquireBds);
        true
    }
    fn release_bds(&self) {
        self.ev(Ev::ReleaseBds);
    }
    fn has_demand_simulator(&self) -> bool {
        *self.has_demand.borrow()
    }
    fn demand_node(&self, demand_id: u32) -> Option<u64> {
        self.ev(Ev::DemandNode(demand_id));
        self.nodes
            .borrow()
            .contains(&demand_id)
            .then_some(0xD000_0000 + u64::from(demand_id))
    }
    fn release_demand_node(&self, node: u64) {
        self.ev(Ev::ReleaseDemandNode(node));
    }
    fn shutdown_grid(&self, grid: &Rc<dyn TractGrid<u8>>) {
        self.ev(Ev::ShutdownGrid(self.instance_of(grid)));
    }
    fn building_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.ev(Ev::BuildingMap);
        self.map.borrow().clone().map(|m| m as Rc<dyn TractGrid<u8>>)
    }
    fn building_map_key(&self, purpose: u32, wealth: u32) -> u8 {
        self.ev(Ev::MapKey(purpose, wealth));
        ((purpose << 2) + wealth + 1) as u8
    }
    fn building_purpose(&self, occupant: u64) -> Option<u32> {
        self.ev(Ev::Purpose(occupant));
        self.purposes.borrow().iter().find(|p| p.0 == occupant).map(|p| p.1)
    }
}

fn sim(cells: i32) -> (Rc<Host>, Rc<CommercialSim>) {
    let h = Host::new(cells);
    let s = CommercialSim::new(h.clone(), 0xC0);
    (h, s)
}

/// An initialised simulator on an `cells`-square city, the event log cleared.
fn ready(cells: i32) -> (Rc<Host>, Rc<CommercialSim>) {
    let (h, s) = sim(cells);
    assert!(s.init());
    h.take();
    (h, s)
}

fn cells(g: &Rc<dyn TractGrid<u8>>) -> Vec<Vec<u8>> {
    (0..g.count_x())
        .map(|x| (0..g.count_z()).map(|z| g.get(x, z)).collect())
        .collect()
}

/// The expected §4.4 result: (x, 0) = 1 except at `zero` (0), every other
/// cell `rest`.
fn edge_pattern(n: i32, zero: &[i32]) -> Vec<Vec<u8>> {
    (0..n)
        .map(|x| {
            (0..n)
                .map(|z| match (z, zero.contains(&x)) {
                    (0, true) => 0,
                    (0, false) => 1,
                    _ => 0,
                })
                .collect()
        })
        .collect()
}

fn insert(occ: u64, data3: u32) -> CommercialMessage {
    CommercialMessage {
        msg_type: msg::OCCUPANT_INSERTED,
        data3,
        void1: Some(occ),
        ..CommercialMessage::default()
    }
}

// ================================================================ §13.1

#[test]
fn ci13_01_identity() {
    for iid in [
        IID_UNKNOWN,
        IID_COMMERCIAL_SIMULATOR,
        IID_MESSAGE_TARGET2,
        IID_SERIALIZABLE,
    ] {
        assert!(query_interface(iid), "commercial {iid:#X}");
    }
    for iid in [IID_INDUSTRIAL_SIMULATOR, IID_SIM_GRID, 0, 0x2E8A_9CB6] {
        assert!(!query_interface(iid), "commercial {iid:#X}");
    }
    for iid in [
        IID_UNKNOWN,
        IID_INDUSTRIAL_SIMULATOR,
        IID_MESSAGE_TARGET2,
        IID_SERIALIZABLE,
    ] {
        assert!(IndustrialSim::query_interface(iid), "industrial {iid:#X}");
    }
    for iid in [IID_COMMERCIAL_SIMULATOR, IID_SIM_GRID, 0] {
        assert!(!IndustrialSim::query_interface(iid), "industrial {iid:#X}");
    }
    let (_, s) = sim(8);
    assert_eq!(s.class_id(), 0x8990_C080);
    assert_eq!(IndustrialSim::new(1).class_id(), 0xA990_C022);
}

// ================================================================ §13.2

#[test]
fn ci13_02_init_fails_without_node_3000() {
    let (h, s) = sim(8);
    h.nodes.borrow_mut().remove(&0x3000);
    assert!(!s.init());
    let ev = h.take();
    assert!(!ev.iter().any(|e| matches!(e, Ev::CreateGrid(..))));
    // Short-circuit: 0x3800 is not asked for.
    assert!(ev.contains(&Ev::DemandNode(0x3000)));
    assert!(!ev.contains(&Ev::DemandNode(0x3800)));
    let st = s.snapshot();
    assert!(st.initialized, "step 1 ran");
    assert!(st.co2.is_none() && st.co3.is_none());
    assert_eq!(st.demand_nodes, [None, None]);
    // [question] A second Init returns true at once, still without grids.
    assert!(s.init());
    assert!(h.take().is_empty());
    assert!(s.proximity_map(developer::CO2).is_none());
}

#[test]
fn ci13_02_init_fails_without_node_3800_or_demand() {
    let (h, s) = sim(8);
    h.nodes.borrow_mut().remove(&0x3800);
    assert!(!s.init());
    let st = s.snapshot();
    assert_eq!(st.demand_nodes, [Some(0xD000_3000), None], "the node found is held");
    assert!(st.co2.is_none() && st.co3.is_none());
    // Shutdown releases the held node.
    s.shutdown();
    assert!(h.take().contains(&Ev::ReleaseDemandNode(0xD000_3000)));

    let (h, s) = sim(8);
    *h.has_demand.borrow_mut() = false;
    assert!(!s.init());
    assert!(!h
        .take()
        .iter()
        .any(|e| matches!(e, Ev::DemandNode(_) | Ev::CreateGrid(..))));
}

#[test]
fn ci13_02_init_registers_and_creates() {
    let (h, s) = sim(64);
    assert!(s.init());
    let name = "cSC4CommercialSimulator".to_string();
    assert_eq!(
        h.take(),
        vec![
            Ev::AddAgent(0, name.clone(), 2),
            Ev::AddAgent(10, name.clone(), 2),
            Ev::AddAgent(4, name.clone(), 2),
            Ev::AddAgent(6, name, 2),
            Ev::AcquireMessageServer,
            Ev::Subscribe(0x99EF_1142),
            Ev::Subscribe(0x99EF_1143),
            Ev::AcquireCity,
            Ev::AcquireBds,
            Ev::DemandNode(0x3000),
            Ev::DemandNode(0x3800),
            Ev::CreateGrid(0x89D5_B6EF, 1),
            Ev::CreateGrid(0x89D5_B788, 1),
        ]
    );
    let st = s.snapshot();
    assert!(st.initialized && st.static_flag);
    assert_eq!(st.tract_size, 1);
    let co2 = s.proximity_map(0x3320).unwrap();
    let co3 = s.proximity_map(0x3330).unwrap();
    assert_eq!(h.instance_of(&co2), 0x89D5_B788);
    assert_eq!(h.instance_of(&co3), 0x89D5_B6EF);
    for g in [&co2, &co3] {
        let geo = g.geometry();
        assert_eq!((geo.tract_size, geo.count_x, geo.count_z), (1, 64, 64));
        assert!(cells(g).iter().flatten().all(|&v| v == 0));
    }
    // A second Init does nothing.
    assert!(s.init());
    assert!(h.take().is_empty());
}

#[test]
fn ci13_02_init_keeps_loaded_grids() {
    let (h, s) = sim(8);
    let loaded = SimGrid::<u8>::create(&h.city, 0x89D5_B788, 1);
    s.with_state(|st| st.co2 = Some(loaded.clone()));
    assert!(s.init());
    let ev = h.take();
    assert!(ev.contains(&Ev::CreateGrid(0x89D5_B6EF, 1)));
    assert!(!ev.contains(&Ev::CreateGrid(0x89D5_B788, 1)));
    let co2 = s.proximity_map(developer::CO2).unwrap();
    assert_eq!(Rc::as_ptr(&co2) as *const (), Rc::as_ptr(&loaded) as *const ());
}

#[test]
fn shutdown_releases_in_order() {
    let (h, s) = ready(8);
    assert!(s.shutdown());
    assert_eq!(
        h.take(),
        vec![
            Ev::ShutdownGrid(0x89D5_B788),
            Ev::ShutdownGrid(0x89D5_B6EF),
            Ev::ReleaseCity,
            Ev::ReleaseBds,
            Ev::Unsubscribe(0x99EF_1142),
            Ev::Unsubscribe(0x99EF_1143),
            Ev::ReleaseMessageServer,
            Ev::ReleaseDemandNode(0xD000_3000),
            Ev::ReleaseDemandNode(0xD000_3800),
            Ev::RemoveAgent,
        ]
    );
    let st = s.snapshot();
    assert!(!st.initialized);
    assert!(st.co2.is_none() && st.co3.is_none());
    // Not initialised: nothing, still true.
    assert!(s.shutdown());
    assert!(h.take().is_empty());
}

#[test]
fn release_semantics() {
    let (h, s) = ready(8);
    assert_eq!(s.add_ref(), 1);
    assert_eq!(s.add_ref(), 2);
    assert_eq!(s.release(), 1);
    assert!(!s.snapshot().destroyed);
    assert_eq!(s.release(), 0);
    let st = s.snapshot();
    assert!(st.destroyed);
    assert_eq!(st.ref_count, 1, "not decremented first");
    assert!(h.take().contains(&Ev::RemoveAgent), "Release ran Shutdown");

    let i = IndustrialSim::new(2);
    assert!(i.init());
    i.add_ref();
    i.add_ref();
    assert_eq!(i.release(), 1);
    assert_eq!(i.release(), 0);
    assert!(i.snapshot().destroyed);
    assert!(!i.snapshot().initialized);
}

// ================================================================ §13.3

#[test]
fn ci13_03_dirty_marking() {
    let (h, s) = ready(8);
    h.purposes.borrow_mut().extend([(1, 3), (2, 1), (3, 3)]);
    s.with_state(|st| st.dirty = false);
    // Purpose 1 does not.
    assert!(s.do_message(&insert(2, OCCUPANT_TYPE_BUILDING)));
    assert!(!s.snapshot().dirty);
    // A network occupant does not, and its profile is not read.
    h.take();
    assert!(s.do_message(&insert(1, 0xC772_BF98)));
    assert!(!s.snapshot().dirty);
    assert!(h.take().is_empty());
    // Not a building occupant: nothing.
    assert!(s.do_message(&insert(99, OCCUPANT_TYPE_BUILDING)));
    assert!(!s.snapshot().dirty);
    // Purpose 3 does.
    assert!(s.do_message(&insert(1, OCCUPANT_TYPE_BUILDING)));
    assert!(s.snapshot().dirty);
    // The removal message too; a preview occupant (no test exists) too.
    s.with_state(|st| st.dirty = false);
    let mut m = insert(3, OCCUPANT_TYPE_BUILDING);
    m.msg_type = msg::OCCUPANT_REMOVED;
    assert!(s.do_message(&m));
    assert!(s.snapshot().dirty);
    // Other messages return true and change nothing.
    s.with_state(|st| st.dirty = false);
    for t in [msg::SIM_BEGIN, msg::SIM_NEW_MONTH, 0x1234_5678] {
        assert!(s.do_message(&CommercialMessage::of_type(t)));
    }
    let st = s.snapshot();
    assert!(!st.dirty);
    assert_eq!(st.state, state::IDLE);
}

// ================================================================ §13.4

#[test]
fn ci13_04_state_machine() {
    let (h, s) = ready(8);
    h.set_map(15, &[(3, 0)]);
    let co2 = s.proximity_map(developer::CO2).unwrap();
    let co3 = s.proximity_map(developer::CO3).unwrap();
    assert!(s.snapshot().dirty, "constructor dirty");
    // Call 1: state 3 → 0, maps unchanged.
    s.do_idle();
    let st = s.snapshot();
    assert_eq!((st.dirty, st.state), (false, 0));
    assert!(cells(&co2).iter().flatten().all(|&v| v == 0));
    assert!(!h.take().iter().any(|e| matches!(e, Ev::MapKey(..))));
    // Call 2: CO$$ rebuilt, state 2.
    s.do_idle();
    assert_eq!(s.snapshot().state, 2);
    assert_eq!(cells(&co2), edge_pattern(8, &[3]));
    assert!(cells(&co3).iter().flatten().all(|&v| v == 0));
    assert_eq!(h.take(), vec![Ev::MapKey(3, 2), Ev::BuildingMap]);
    // Call 3: CO$$$ rebuilt, state 3.
    s.do_idle();
    assert_eq!(s.snapshot().state, 3);
    assert_eq!(cells(&co3), edge_pattern(8, &[]));
    assert_eq!(h.take(), vec![Ev::MapKey(3, 3), Ev::BuildingMap]);
    // Not dirty: nothing.
    let before = (cells(&co2), cells(&co3));
    s.do_idle();
    let st = s.snapshot();
    assert_eq!((st.dirty, st.state), (false, 3));
    assert!(h.take().is_empty());
    assert_eq!((cells(&co2), cells(&co3)), before);
    // The day and idle messages run DoIdle; the month one does not.
    s.with_state(|st| st.dirty = true);
    s.do_message(&CommercialMessage::of_type(msg::SIM_NEW_MONTH));
    assert_eq!(s.snapshot().state, 3);
    s.do_message(&CommercialMessage::of_type(msg::SIM_NEW_DAY));
    assert_eq!(s.snapshot().state, 0);
    s.do_message(&CommercialMessage::of_type(msg::SIM_IDLE));
    assert_eq!(s.snapshot().state, 2);
}

#[test]
fn state_one_rebuilds_co2() {
    let (h, s) = ready(8);
    // A building map without an office: the rebuild gives the edge pattern.
    h.set_map(15, &[]);
    s.with_state(|st| {
        st.dirty = false;
        st.state = state::REBUILD_CO2;
    });
    s.do_idle();
    assert_eq!(s.snapshot().state, 2);
    assert_eq!(cells(&s.proximity_map(developer::CO2).unwrap()), edge_pattern(8, &[]));
    // [choice] An impossible state does nothing.
    s.with_state(|st| st.state = 7);
    s.do_idle();
    assert_eq!(s.snapshot().state, 7);
}

// ================================================================ §13.5

#[test]
fn ci13_05_map_content() {
    let (h, s) = ready(8);
    h.set_map(15, &[(3, 0), (5, 5)]);
    for _ in 0..3 {
        s.do_idle();
    }
    assert_eq!(cells(&s.proximity_map(developer::CO2).unwrap()), edge_pattern(8, &[3]));
    assert_eq!(cells(&s.proximity_map(developer::CO3).unwrap()), edge_pattern(8, &[]));
}

/// §4.5: the 64 × 64 worked example.
#[test]
fn worked_example_4_5() {
    let (h, s) = ready(64);
    h.set_map(15, &[(10, 30), (11, 31), (12, 32)]);
    h.purposes.borrow_mut().push((7, 3));
    s.with_state(|st| st.dirty = false);
    s.do_message(&insert(7, OCCUPANT_TYPE_BUILDING));
    for _ in 0..3 {
        s.do_idle();
    }
    let g = cells(&s.proximity_map(0x3320).unwrap());
    assert_eq!(g.iter().flatten().filter(|&&v| v == 1).count(), 64);
    assert_eq!(g.iter().flatten().filter(|&&v| v == 0).count(), 4032);
    // The tower at (5, 0).
    h.set_map(15, &[(5, 0)]);
    s.with_state(|st| st.dirty = true);
    for _ in 0..3 {
        s.do_idle();
    }
    assert_eq!(cells(&s.proximity_map(0x3320).unwrap()), edge_pattern(64, &[5]));
}

#[test]
fn rebuild_without_building_map_or_grid() {
    // [compat] No building map (§4.4, CM5): no seed and no passes, so a
    // fresh grid stays all 0.
    let (h, s) = ready(8);
    assert!(h.building_map().is_none());
    for _ in 0..3 {
        s.do_idle();
    }
    let zeros = vec![vec![0u8; 8]; 8];
    assert_eq!(cells(&s.proximity_map(developer::CO2).unwrap()), zeros);
    assert_eq!(cells(&s.proximity_map(developer::CO3).unwrap()), zeros);
    // CM5: no passes either, so a loaded grid keeps its values exactly.
    let co3 = s.proximity_map(developer::CO3).unwrap();
    co3.set(4, 4, 9);
    co3.set(2, 0, 200);
    s.rebuild(WEALTH_CO3);
    let mut want = zeros.clone();
    want[4][4] = 9;
    want[2][0] = 200;
    assert_eq!(cells(&co3), want);
    // [choice] An absent grid: the rebuild does nothing; the machine advances.
    let (h2, s2) = sim(8);
    h2.nodes.borrow_mut().clear();
    assert!(!s2.init());
    for _ in 0..3 {
        s2.do_idle();
    }
    assert_eq!(s2.snapshot().state, 3);
    drop(h);
}

// ================================================================ §13.6

#[test]
fn ci13_06_loaded_grids() {
    let (h, s) = ready(8);
    let co2 = s.proximity_map(developer::CO2).unwrap();
    co2.set(4, 4, 9);
    co2.set(2, 0, 0);
    h.set_map(15, &[]);
    s.with_state(|st| st.state = state::REBUILD_CO2);
    s.do_idle();
    assert_eq!(co2.get(4, 4), 1, "lowered to its neighbours + 1");
    assert_eq!(co2.get(2, 0), 1, "seeded 255, converged to 1");
    let mut want = edge_pattern(8, &[]);
    want[4][4] = 1;
    assert_eq!(cells(&co2), want);
    // Values off the edge are only ever lowered, never raised: on a grid of
    // all 9 the edge seeds 255 and settles at 10 (its neighbour + 1); every
    // other cell keeps its 9.
    let co3 = s.proximity_map(developer::CO3).unwrap();
    co3.fill(9);
    s.do_idle();
    let g = cells(&co3);
    for row in &g {
        assert_eq!(row[0], 10, "edge cell next to 9");
        assert!(row[1..].iter().all(|&v| v == 9));
    }
}

// ================================================================ §13.7

#[test]
fn ci13_07_proximity_map() {
    let (_, s) = ready(8);
    let a = s.proximity_map(0x3320).unwrap();
    let b = s.proximity_map(0x3330).unwrap();
    let st = s.snapshot();
    assert!(Rc::ptr_eq(&a, st.co2.as_ref().unwrap()));
    assert!(Rc::ptr_eq(&b, st.co3.as_ref().unwrap()));
    for dev in [0x3110, 0x3310, 0] {
        assert!(s.proximity_map(dev).is_none());
    }
    assert_eq!(s.snapshot().ref_count, 0, "no reference added");
}

// ================================================================ §13.8

/// Resolves references to the grids registered while writing.
struct Resolver(Vec<RegisteredObject>);

impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        let e = self.0.iter().find(|e| e.0 == class_id && e.1 == object_id)?;
        e.2.clone().map(RefValue::Object)
    }
}

fn record(s: &dyn SaveObject) -> (Vec<u8>, Vec<RegisteredObject>) {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg.clone());
    assert!(s.write(&mut w));
    let entries = reg.borrow().entries.clone();
    (w.into_bytes(), entries)
}

fn load(s: &dyn SaveObject, bytes: &[u8], entries: &[RegisteredObject]) -> bool {
    let mut res = Resolver(entries.to_vec());
    s.read(&mut ByteReader::with_resolver(bytes, &mut res))
}

#[test]
fn ci13_08_save_round_trip() {
    let (_, s) = ready(8);
    let (bytes, entries) = record(&*s);
    assert_eq!(bytes.len(), 19);
    assert_eq!(&bytes[..3], &[1, 0, 1], "version 1, flag 1 (constructor dirty)");
    // References: (id 1, class 0x49B9E602), (id 2, class 0x49B9E602).
    assert_eq!(&bytes[3..11], &[1, 0, 0, 0, 0x02, 0xE6, 0xB9, 0x49]);
    assert_eq!(&bytes[11..19], &[2, 0, 0, 0, 0x02, 0xE6, 0xB9, 0x49]);

    let (_, t) = sim(8);
    assert!(load(&*t, &bytes, &entries));
    assert_eq!(record(&*t).0, bytes, "write → read → write");
    let st = t.snapshot();
    assert!(st.dirty);
    assert_eq!(st.state, 3, "state is not read");
    assert!(Rc::ptr_eq(st.co2.as_ref().unwrap(), s.snapshot().co2.as_ref().unwrap()));
    assert!(Rc::ptr_eq(st.co3.as_ref().unwrap(), s.snapshot().co3.as_ref().unwrap()));
}

#[test]
fn ci13_08_flag() {
    let (_, s) = ready(8);
    let flag = |s: &CommercialSim| record(s).0[2];
    for (dirty, state, want) in [
        (true, 3, 1),
        (false, 3, 0),
        (false, 0, 1),
        (false, 1, 1),
        (false, 2, 1),
        (true, 2, 1),
    ] {
        s.with_state(|st| {
            st.dirty = dirty;
            st.state = state;
        });
        assert_eq!(flag(&s), want, "dirty {dirty} state {state}");
    }
}

#[test]
fn ci13_08_version_and_restart() {
    let (_, s) = ready(8);
    let (bytes, entries) = record(&*s);
    let mut v2 = bytes.clone();
    v2[0] = 2;
    let (_, t) = sim(8);
    assert!(!load(&*t, &v2, &entries));
    assert!(t.snapshot().co2.is_none());
    // A truncated record leaves the object unchanged ([choice] atomic).
    t.with_state(|st| st.dirty = false);
    assert!(!load(&*t, &bytes[..12], &entries));
    assert!(!t.snapshot().dirty);
    assert!(t.snapshot().co2.is_none());
    // After reading flag 1 the first DoIdle starts a rebuild.
    assert!(load(&*t, &bytes, &entries));
    t.do_idle();
    let st = t.snapshot();
    assert_eq!((st.dirty, st.state), (false, 0));
    // Flag 0 reads as not dirty: DoIdle does nothing.
    let mut f0 = bytes.clone();
    f0[2] = 0;
    let (_, u) = sim(8);
    assert!(load(&*u, &f0, &entries));
    u.do_idle();
    let st = u.snapshot();
    assert_eq!((st.dirty, st.state), (false, 3));
}

#[test]
fn ci13_08_null_grids_and_second_read() {
    let (_, s) = sim(8);
    s.with_state(|st| st.dirty = false);
    let (bytes, _) = record(&*s);
    assert_eq!(
        bytes,
        vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        "null references are 4 bytes"
    );
    // [choice] A second Read replaces (releases) the old grids.
    let (_, full) = ready(8);
    let (fb, fe) = record(&*full);
    let (_, t) = sim(8);
    assert!(load(&*t, &fb, &fe));
    let old = t.snapshot().co2.unwrap();
    let before = Rc::strong_count(&old);
    assert!(load(&*t, &bytes, &[]));
    assert!(t.snapshot().co2.is_none());
    assert_eq!(Rc::strong_count(&old), before - 1);
}

#[test]
fn ci13_08_industrial_record() {
    let i = IndustrialSim::new(5);
    let (bytes, _) = record(&*i);
    assert_eq!(bytes, vec![0x01, 0x00]);
    assert!(load(&*i, &bytes, &[]));
    assert!(!load(&*i, &[0x02, 0x00], &[]));
    assert!(!load(&*i, &[0x01], &[]));
    assert!(i.init());
    let st = i.snapshot();
    assert!(st.initialized && st.static_flag);
    assert!(i.do_message(0x6695_681A));
    assert!(i.shutdown());
}

// ================================================================ §13.9

/// §8.3, synthetic: the corpus' two commercial record shapes (flag 1 with
/// both grids all 0 at 64, 128 and 256 square; flag 0 with the 128 × 128
/// edge pattern) and the industrial `01 00` read and re-write
/// byte-identically. [not the corpus] The 421 saves live in the user's
/// documents folder, which this lane may not read.
#[test]
fn ci13_09_corpus_shapes() {
    for (n, flag) in [(64, 1u8), (128, 1), (256, 1), (128, 0)] {
        let city = CityDims::square(n);
        let co2 = SimGrid::<u8>::create(&city, instance::CO2, 1);
        let co3 = SimGrid::<u8>::create(&city, instance::CO3, 1);
        if flag == 0 {
            for x in 0..n {
                co2.set(x, 0, 1);
                co3.set(x, 0, 1);
            }
        }
        let mut bytes = vec![1, 0, flag];
        for id in [1u32, 2] {
            bytes.extend(id.to_le_bytes());
            bytes.extend(GRID_CLASS_U8.to_le_bytes());
        }
        let entries: Vec<RegisteredObject> = vec![
            (GRID_CLASS_U8, 1, Some(co2.clone() as Rc<dyn SaveObject>), 0),
            (GRID_CLASS_U8, 2, Some(co3.clone() as Rc<dyn SaveObject>), 0),
        ];
        let (_, s) = sim(n);
        assert!(load(&*s, &bytes, &entries));
        assert_eq!(s.snapshot().dirty, flag == 1);
        let (again, reg) = record(&*s);
        assert_eq!(again, bytes, "{n} flag {flag}");
        assert_eq!(reg.len(), 2);
        // A flag-0 city keeps its maps until something marks them dirty.
        assert!(s.init());
        s.do_idle();
        assert_eq!(s.snapshot().state, if flag == 1 { 0 } else { 3 });
    }
    let i = IndustrialSim::new(1);
    assert!(load(&*i, &[1, 0], &[]));
    assert_eq!(record(&*i).0, vec![1, 0]);
}
