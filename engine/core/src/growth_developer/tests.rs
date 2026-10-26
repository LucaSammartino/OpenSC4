// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests of the growth developer against a minimal fake host (the
//! conformance vectors `gd14_…` / `gd00_…` cover the spec in depth).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use super::*;
use crate::development::{GrowthItem, GrowthProfile};
use crate::grid::{CityDims, CoreGrids, GridElem, GridProvider, SimGrid, TractGrid};
use crate::property::PropertyHolder;
use crate::rng::FixedSeeds;
use crate::stream::{ByteReader, ByteWriter, RefValue, SaveObject};
use crate::variant::{mem, Elem, Variant};

struct Fake {
    grids: CoreGrids,
    park: RefCell<Option<Rc<SimGrid<i16>>>>,
    made_u8: RefCell<Vec<Rc<SimGrid<u8>>>>,
    made_i16: RefCell<Vec<Rc<SimGrid<i16>>>>,
    made_u16: RefCell<Vec<Rc<SimGrid<u16>>>>,
    calls: RefCell<Vec<String>>,
    demand_added: RefCell<Vec<f32>>,
    nodes: RefCell<HashMap<(u32, u32), u64>>,
    date: Cell<u32>,
}

impl Fake {
    fn new(cells: i32) -> Rc<Fake> {
        Rc::new(Fake {
            grids: CoreGrids {
                city: CityDims::square(cells),
            },
            park: RefCell::new(None),
            made_u8: RefCell::new(Vec::new()),
            made_i16: RefCell::new(Vec::new()),
            made_u16: RefCell::new(Vec::new()),
            calls: RefCell::new(Vec::new()),
            demand_added: RefCell::new(Vec::new()),
            nodes: RefCell::new(HashMap::new()),
            date: Cell::new(0),
        })
    }

    fn log(&self, s: &str) {
        self.calls.borrow_mut().push(s.to_owned());
    }
}

macro_rules! provider {
    ($t:ty, $list:ident) => {
        impl GridProvider<$t> for Fake {
            fn create_grid(&self, i: u32, s: i32) -> Option<Rc<dyn TractGrid<$t>>> {
                let g = SimGrid::<$t>::create(&self.grids.city, i, s);
                self.$list.borrow_mut().push(g.clone());
                Some(g as Rc<dyn TractGrid<$t>>)
            }
        }
    };
}

provider!(u8, made_u8);
provider!(i16, made_i16);
provider!(u16, made_u16);

fn resize<T: GridElem>(city: &CityDims, made: &[Rc<SimGrid<T>>], g: &Rc<dyn TractGrid<T>>, s: i32) {
    let p = Rc::as_ptr(g) as *const ();
    if let Some(sg) = made.iter().find(|m| Rc::as_ptr(m) as *const () == p) {
        sg.set_tract_size(city, s);
    }
}

impl GrowthDeveloperHost for Fake {
    fn acquire_service(&self, _s: Service) -> bool {
        true
    }
    fn release_service(&self, _s: Service) {}
    fn acquire_message_server(&self) -> bool {
        true
    }
    fn release_message_server(&self) {}
    fn subscribe(&self, _t: &[u32]) {
        self.log("subscribe");
    }
    fn unsubscribe(&self, _t: &[u32]) {
        self.log("unsubscribe");
    }
    fn simulator_present(&self) -> bool {
        true
    }
    fn add_agent(&self, level: u32, _name: &str, _flags: u32) {
        self.log(&format!("agent {level}"));
    }
    fn remove_agent(&self) {
        self.log("remove_agent");
    }
    fn sim_date_number(&self) -> u32 {
        self.date.get()
    }
    fn grid_set_tract_size(&self, _w: DevGrid, g: GridHandle<'_>, s: i32) {
        let city = self.grids.city;
        match g {
            GridHandle::U8(g) => resize(&city, &self.made_u8.borrow(), g, s),
            GridHandle::I16(g) => resize(&city, &self.made_i16.borrow(), g, s),
            GridHandle::U16(g) => resize(&city, &self.made_u16.borrow(), g, s),
        }
    }
    fn grid_shutdown(&self, w: DevGrid, _g: GridHandle<'_>) {
        self.log(&format!("grid_shutdown {w:?}"));
    }
    fn add_ref(&self, t: u64) {
        self.log(&format!("add_ref {t}"));
    }
    fn release(&self, t: u64) {
        self.log(&format!("release {t}"));
    }
    fn get_demand(&self, id: u32, index: u32) -> Option<u64> {
        self.nodes.borrow().get(&(id, index)).copied()
    }
    fn query_supply(&self, _n: u64) -> f32 {
        0.0
    }
    fn add_to_demand_value(&self, _n: u64, v: f32) {
        self.demand_added.borrow_mut().push(v);
    }
    fn active_demand(&self, n: u64) -> f32 {
        n as f32 * 0.5
    }
    fn connected_cities(&self) -> u32 {
        0
    }
    fn all_connected_cities(&self) -> u32 {
        0
    }
    fn stage_population_multiplier(&self) -> f32 {
        1.0
    }
    fn tract_developer(&self) -> Option<u64> {
        None
    }
    fn special_buildings(&self) -> Vec<u64> {
        Vec::new()
    }
    fn max_capacity(&self, _z: i32, _p: u32, _w: u32) -> f32 {
        7.3
    }
    fn building_purpose(&self, _bt: u32) -> u32 {
        1
    }
    fn update_population(&self, _td: u64, _p: &GrowthProfile) {}
    fn zone_type(&self, _x: i32, _z: i32) -> i8 {
        1
    }
    fn lot_building_type(&self, _l: u64, _f: bool) -> u32 {
        0
    }
    fn lot_capacity(&self, _l: u64, _d: u32, _f: bool) -> u16 {
        40
    }
    fn lot_size(&self, _l: u64) -> (u8, u8) {
        (2, 3)
    }
    fn lot_bounding_rect(&self, _l: u64) -> [i32; 4] {
        [3, 0, 4, 2]
    }
    fn lot_location(&self, _l: u64) -> (i32, i32) {
        (3, 0)
    }
    fn occupant_property(&self, _o: u64, _id: u32) -> Option<Variant> {
        None
    }
    fn occupant_position(&self, _o: u64) -> [f32; 3] {
        [0.0; 3]
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        ((x / 16.0) as i32, (z / 16.0) as i32)
    }
    fn source_u8(&self, _m: SourceMap) -> Option<Rc<dyn TractGrid<u8>>> {
        None
    }
    fn source_i8(&self, _m: SourceMap) -> Option<Rc<dyn TractGrid<i8>>> {
        None
    }
    fn source_i16(&self, m: SourceMap) -> Option<Rc<dyn TractGrid<i16>>> {
        match m {
            SourceMap::Park => self.park.borrow().clone().map(|g| g as Rc<dyn TractGrid<i16>>),
            _ => None,
        }
    }
    fn source_f32(&self, _m: SourceMap) -> Option<Rc<dyn TractGrid<f32>>> {
        None
    }
    fn limit_timer_start(&self, _ms: u32) {}
    fn limit_timer_expired(&self) -> bool {
        false
    }
    fn session_seed(&self) -> u32 {
        1
    }
}

fn seeds() -> FixedSeeds {
    FixedSeeds {
        wall_clock: 1,
        process_clock: 0,
    }
}

fn exemplar(dev: u32, zone: u8, purpose: u8) -> PropertyHolder {
    let mut h = PropertyHolder::new();
    h.add(prop::DEVELOPER_TYPE, Variant::Scalar(Elem::U32(dev)));
    h.add(prop::TARGET_ZONE, Variant::Scalar(Elem::U8(zone)));
    h.add(prop::PURPOSE, Variant::Scalar(Elem::U8(purpose)));
    h.add(prop::WEALTH, Variant::Scalar(Elem::U8(1)));
    h.add(prop::TRACT_SIZE, Variant::Scalar(Elem::U8(4)));
    h.add(prop::BASELINE, Variant::Scalar(Elem::I32(0)));
    h.add(prop::GROWTH_THRESHOLD, Variant::Scalar(Elem::I32(50)));
    h.add(
        Curve::Park.property(),
        Variant::array(
            mem::FLOAT32,
            [-1000.0f32, -1000.0, 1000.0, 1000.0].map(Elem::F32).to_vec(),
        ),
    );
    h
}

fn developer(host: &Rc<Fake>, statics: &Rc<GrowthStatics>, ex: PropertyHolder) -> Rc<GrowthDeveloper> {
    let d = GrowthDeveloper::new(host.clone() as Rc<dyn GrowthDeveloperHost>, statics.clone(), 1);
    d.add_ref();
    assert!(d.init(Some(Rc::new(ex))));
    d
}

fn run_update(d: &GrowthDeveloper) -> u32 {
    d.start_map_update();
    let mut calls = 0;
    while !d.is_map_update_finished() {
        d.do_idle(5);
        calls += 1;
        assert!(calls < 1000);
    }
    calls
}

#[test]
fn stage_descent_stops_at_an_unordered_accumulator() {
    use crate::curve::ResponseCurve;

    // §5.5: NaN city size makes a multi-point stage curve return NaN.
    let interpolated = ResponseCurve::from_points(&[(0.0, 10.0), (100.0, 20.0)]);
    let mut items = vec![
        GrowthItem {
            x: 0,
            z: 0,
            max_stage: 1
        };
        3
    ];
    assign_stages(&mut items, &[interpolated.clone(), interpolated.clone()], f32::NAN);
    assert!(items.iter().all(|it| it.max_stage == 2));

    // A finite top stage can descend into NaN; it must then keep that stage.
    let top = ResponseCurve::from_points(&[(0.0, 25.0)]);
    assign_stages(&mut items, &[interpolated.clone(), interpolated, top], f32::NAN);
    assert_eq!(items.iter().map(|it| it.max_stage).collect::<Vec<_>>(), [3, 2, 2]);
}

fn park(host: &Fake, values: &[(i32, i32, i16)]) {
    let g = SimGrid::<i16>::create(&host.grids.city, 0, 4);
    for &(x, z, v) in values {
        g.set(x, z, v);
    }
    *host.park.borrow_mut() = Some(g);
}

#[test]
fn init_creates_tuned_grids_and_shutdown_releases() {
    let host = Fake::new(64);
    let st = GrowthStatics::new(0, &seeds());
    let d = developer(&host, &st, exemplar(0x1010, 1, 1));
    assert!(d.is_initialized());
    let w = d.working_map().expect("W");
    assert_eq!(w.geometry().tract_size, 4);
    assert_eq!((w.geometry().count_x, w.geometry().count_z), (16, 16));
    assert_eq!(d.data().iterator.as_ref().map(|i| (i.x1, i.z1)), Some((15, 15)));
    assert!(host.calls.borrow().iter().any(|c| c == "agent 10"));
    assert!(d.shutdown());
    assert!(d.desirability_map().is_none());
    assert_eq!(
        host.calls
            .borrow()
            .iter()
            .filter(|c| c.starts_with("grid_shutdown"))
            .count(),
        6
    );
    assert_eq!(d.release(), 0);
    assert!(d.data().destroyed);
}

#[test]
fn update_builds_sorted_lists() {
    // §14 test 8.
    let host = Fake::new(8);
    park(&host, &[(0, 0, 60), (0, 1, 70), (1, 0, 60), (1, 1, 40)]);
    let st = GrowthStatics::new(0, &seeds());
    let d = developer(&host, &st, exemplar(0x1010, 1, 1));
    d.update_max_capacity([0, 0, 7, 7]);
    assert_eq!(d.zoned_count_map().map(|g| g.get(1, 1)), Some(16));
    assert_eq!(d.max_capacity_map().map(|g| g.get(1, 1)), Some(117));
    assert_eq!(run_update(&d), 20);
    let item = |x, z| GrowthItem { x, z, max_stage: 1 };
    assert_eq!(*d.growth_list(), vec![item(0, 1), item(0, 0), item(1, 0)]);
    assert_eq!(*d.decline_list(), vec![item(1, 1)]);
    assert_eq!(st.draws(), 0);
}

#[test]
fn industrial_decline_order_draws_one_salt() {
    // §14 test 9.
    let host = Fake::new(64);
    park(&host, &[(3, 5, 10), (4, 5, 20), (10, 2, 30), (0, 0, 40)]);
    let st = GrowthStatics::new(12345, &seeds());
    let d = developer(&host, &st, exemplar(0x4100, 3, 5));
    let zoned = d.zoned_count_map().expect("zoned");
    for (x, z) in [(3, 5), (4, 5), (10, 2), (0, 0)] {
        zoned.set(x, z, 1);
    }
    run_update(&d);
    let order: Vec<(i32, i32)> = d.decline_list().iter().map(|i| (i.x, i.z)).collect();
    assert_eq!(order, vec![(0, 0), (3, 5), (10, 2), (4, 5)]);
    assert_eq!(d.data().last_salt, Some(0x0C63_D3DC));
    assert_eq!(st.draws(), 1);
    assert_eq!(st.draw_salt(), 0xFED6_A704);
}

#[test]
fn capacity_bookkeeping_and_messages() {
    // §14 tests 3 and 16.
    let host = Fake::new(16);
    let st = GrowthStatics::new(0, &seeds());
    let d = developer(&host, &st, exemplar(0x1010, 1, 1));
    let m = GrowthMessage {
        data1: 1,
        data2: 2,
        data3: 0x1234,
        data4: 1,
        unknown: Some(7),
        ..GrowthMessage::of_type(msg::STATE_CHANGED)
    };
    assert!(d.handle_message(&m));
    let cap = d.capacity_map().expect("capacity");
    assert_eq!((cap.get(0, 0), cap.get(1, 0)), (22, 18));
    assert_eq!(d.total_capacity(), 40);
    d.update_population_count(7, 0x1234, false);
    d.update_population_count(7, 0x1234, false);
    assert_eq!((cap.get(0, 0), cap.get(1, 0)), (0, 0));
    assert_eq!(d.total_capacity(), -40);
}

#[test]
fn phantom_demand_and_new_day_rule() {
    // §14 tests 10 and 11 (by the rules: the 20 × 0.01f tie rounds to even).
    let host = Fake::new(16);
    host.nodes.borrow_mut().insert((0x1010, 0x20000), 9);
    let st = GrowthStatics::new(0, &seeds());
    let mut ex = exemplar(0x1010, 1, 1);
    ex.add(prop::BASELINE, Variant::Scalar(Elem::I32(30)));
    ex.remove(Curve::Park.property());
    ex.add(prop::PHANTOM_DEMAND, Variant::Scalar(Elem::Bool(true)));
    ex.add(prop::SOURCE_DEMAND, Variant::Scalar(Elem::U32(0x1010)));
    let d = developer(&host, &st, ex);
    d.zoned_count_map().expect("zoned").set(0, 0, 1);
    d.capacity_map().expect("capacity").set(0, 0, 667);
    run_update(&d);
    assert_eq!(d.population_map().map(|g| g.get(0, 0)), Some(200));
    assert_eq!(*host.demand_added.borrow(), vec![39.0]);
    assert_eq!(d.data().last_phantom_sum, 39);

    d.data_mut().start_day = 100;
    host.date.set(103);
    for (state, want) in [(2, false), (3, true), (0, true)] {
        d.data_mut().update_state = state;
        assert_eq!(d.new_day_finished(), want, "state {state}");
    }
}

#[test]
fn record_versions() {
    // §14 test 14 (null references only; the vectors cover the store).
    let host = Fake::new(16);
    let st = GrowthStatics::new(0, &seeds());
    let d = GrowthDeveloper::new(host.clone() as Rc<dyn GrowthDeveloperHost>, st, 1);
    let mut w = ByteWriter::new();
    assert!(SaveObject::write(&*d, &mut w));
    assert_eq!(w.bytes().len(), 30);
    assert_eq!(&w.bytes()[..2], &[1, 0]);
    let mut bad = w.bytes().to_vec();
    bad[0] = 2;
    bad[2] = 9;
    assert!(!SaveObject::read(&*d, &mut ByteReader::new(&bad)));
    assert_eq!(d.data().last_phantom_sum, 0);
    let mut good = w.bytes().to_vec();
    good[2] = 9;
    assert!(SaveObject::read(&*d, &mut ByteReader::new(&good)));
    assert_eq!(d.data().last_phantom_sum, 9);
    assert!(!SaveObject::read(&*d, &mut ByteReader::new(&good[..8])));
}

/// FIXTURE: the 50 payload bytes of ONE class-store entry (object 0x0EC6381C)
/// of the growth developer class record `(0x0990C138, 0x299B2D1B, 0)` in the
/// test city save (FC4 Test, City - Konradshohe.sc4), and nothing else.
/// Its total capacity is `0x3DCF4E8D`: the original writes an uninitialised
/// word there (save-records §6, fill patterns), which run 102 logged as
/// `capacity=1036996237`.
const KONRADSHOHE_GROWTH_RECORD: [u8; 50] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8d, 0x4e, 0xcf, 0x3d, 0x18, 0x12, 0x94, 0x17, 0x02, 0xe6, 0xb9, 0x49, 0x98,
    0x12, 0x94, 0x17, 0x05, 0xe6, 0xb9, 0x49, 0x18, 0x13, 0x94, 0x17, 0x05, 0xe6, 0xb9, 0x49, 0x98, 0x13, 0x94, 0x17,
    0x04, 0xe6, 0xb9, 0x49, 0x18, 0x14, 0x94, 0x17, 0x04, 0xe6, 0xb9, 0x49,
];

/// Resolves the record's grid references to new core grids (the class store's
/// part, foundations §2.7) and remembers what was asked.
struct SavedGrids {
    city: CityDims,
    asked: Vec<(u32, u32)>,
}

impl crate::stream::RefResolver for SavedGrids {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        self.asked.push((class_id, object_id));
        let o: Rc<dyn SaveObject> = match class_id {
            0x49B9_E602 => SimGrid::<u8>::create(&self.city, object_id, 4),
            0x49B9_E605 => SimGrid::<i16>::create(&self.city, object_id, 4),
            0x49B9_E604 => SimGrid::<u16>::create(&self.city, object_id, 4),
            _ => return None,
        };
        Some(RefValue::Object(o))
    }
}

#[test]
fn reads_a_saved_record_byte_for_byte() {
    // §9 against a real record: version 1, last phantom sum 0, the total
    // capacity word carried unchanged, then the five grid references in
    // order (zoned u8, D i16, population i16, built u16, maximum u16).
    let host = Fake::new(128);
    let st = GrowthStatics::new(0, &seeds());
    let d = GrowthDeveloper::new(host.clone() as Rc<dyn GrowthDeveloperHost>, st, 1);
    let mut grids = SavedGrids {
        city: CityDims::square(128),
        asked: Vec::new(),
    };
    {
        let mut r = ByteReader::with_resolver(&KONRADSHOHE_GROWTH_RECORD, &mut grids);
        assert!(SaveObject::read(&*d, &mut r));
        assert_eq!(r.remaining(), 0);
    }
    assert_eq!(d.data().last_phantom_sum, 0);
    assert_eq!(d.data().total_capacity as u32, 0x3DCF_4E8D);
    assert_eq!(
        grids.asked,
        [
            (0x49B9_E602, 0x1794_1218),
            (0x49B9_E605, 0x1794_1298),
            (0x49B9_E605, 0x1794_1318),
            (0x49B9_E604, 0x1794_1398),
            (0x49B9_E604, 0x1794_1418),
        ]
    );
    assert!(d.zoned_count_map().is_some());
    assert!(d.desirability_map().is_some());
    assert!(d.population_map().is_some());
    assert!(d.capacity_map().is_some());
    assert!(d.max_capacity_map().is_some());
    // Written again: the same bytes, the object IDs renumbered by the store.
    let reg = Rc::new(RefCell::new(crate::stream::SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg);
    assert!(SaveObject::write(&*d, &mut w));
    let out = w.bytes();
    assert_eq!(out.len(), KONRADSHOHE_GROWTH_RECORD.len());
    assert_eq!(&out[..10], &KONRADSHOHE_GROWTH_RECORD[..10]);
    for (k, at) in (10..50).step_by(8).enumerate() {
        assert_eq!(u32::from_le_bytes(out[at..at + 4].try_into().unwrap()), k as u32 + 1);
        assert_eq!(&out[at + 4..at + 8], &KONRADSHOHE_GROWTH_RECORD[at + 4..at + 8]);
    }
}

#[test]
fn shipped_exemplars_read() {
    let host = Fake::new(16);
    let st = GrowthStatics::new(0, &seeds());
    for t in shipped_types() {
        let d = GrowthDeveloper::new(host.clone() as Rc<dyn GrowthDeveloperHost>, st.clone(), 1);
        assert!(d.init(shipped_exemplar(t).map(Rc::new)));
        let data = d.data();
        assert_eq!(data.tuning.developer_type, t);
        assert_eq!(data.tuning.tract_size, 4);
        assert!(Curve::ALL.iter().all(|c| data.tuning.curve(*c).is_some()));
        assert_eq!(data.tuning.stage_curves.len(), if t >= 0x4000 { 3 } else { 8 });
    }
    assert!(shipped_exemplar(0x1234).is_none());
}

#[test]
fn queries_and_slots() {
    assert_eq!(query_interface(IID_GROWTH_DEVELOPER), Some(GrowthInterface::Primary));
    assert_eq!(query_interface(7), None);
    assert!(SLOTS.iter().enumerate().all(|(i, r)| r.0 == i as u32));
    let mut s = SliceStep::default();
    for _ in 0..16 {
        s.adapt(21);
    }
    assert_eq!(s, SliceStep { step: 512, misses: 0 });
    for _ in 0..4 {
        s.adapt(1);
    }
    assert_eq!(s, SliceStep { step: 1024, misses: 0 });
}

#[test]
fn replacement_unit_methods() {
    // The BDS glue's one-to-one operations (module docs, "Replacement unit").
    let host = Fake::new(64);
    host.nodes.borrow_mut().insert((0x4100, SOURCE_DEMAND_INDEX), 42);
    let st = GrowthStatics::new(0, &seeds());
    let mut ex = exemplar(0x4100, 3, 6);
    ex.add(prop::SOURCE_DEMAND, Variant::Scalar(Elem::U32(0x4100)));
    let (d, ok) =
        GrowthDeveloper::create_from_exemplar(host.clone() as Rc<dyn GrowthDeveloperHost>, st.clone(), 9, Rc::new(ex));
    assert!(ok);
    assert_eq!(d.data().ref_count, 1);
    assert_eq!((d.sub_type(), d.purpose()), (0x4100, 6));
    // docs/abi (re-answers9): slot 5 GetType is the target zone (+0xD0), slot 6
    // GetSubType the developer type (+0xCC).
    assert_eq!(d.target_zone(), 3);
    assert_eq!(d.developer_type(), 0x4100);
    assert_eq!(d.source_demand(), Some(42));
    assert_eq!(d.source_active_demand(), Some(21.0));
    let zoned = d.zoned_count_map().expect("zoned");
    zoned.set(1, 1, 1);
    zoned.set(2, 1, 1);
    assert!(d.lists_valid());
    run_update(&d);
    assert_eq!(d.growth_items(), *d.growth_list());
    assert_eq!(d.decline_items(), *d.decline_list());
    let r = d.save_ref();
    let back = GrowthDeveloper::from_save_ref(&r).expect("resolves");
    assert!(Rc::ptr_eq(&back, &d));
    assert!(GrowthDeveloper::from_save_ref(&RefValue::Null).is_none());

    let bare = GrowthDeveloper::create(host.clone() as Rc<dyn GrowthDeveloperHost>, st, 10);
    assert_eq!(bare.data().ref_count, 1);
    assert_eq!(bare.source_active_demand(), None);
}

#[test]
fn deterministic_pacing_finishes_the_update_in_one_idle() {
    // simulator.md §5.7 [choice]: one settle SimIdle runs the whole map
    // update; the lists equal the step-by-step (original) update.
    let host = Fake::new(8);
    park(&host, &[(0, 0, 60), (0, 1, 70), (1, 0, 60), (1, 1, 40)]);
    let st = GrowthStatics::new(0, &seeds());
    let original = developer(&host, &st, exemplar(0x1010, 1, 1));
    original.update_max_capacity([0, 0, 7, 7]);
    run_update(&original);
    let d = developer(&host, &st, exemplar(0x1010, 1, 1));
    d.set_pacing(crate::pacing::Pacing::Deterministic);
    d.update_max_capacity([0, 0, 7, 7]);
    d.start_map_update();
    let idle = GrowthMessage {
        data3: 22,
        ..GrowthMessage::of_type(msg::SIM_IDLE)
    };
    assert!(d.handle_message(&idle));
    assert!(d.is_map_update_finished(), "the whole update in one delivery");
    assert_eq!(*d.growth_list(), *original.growth_list());
    assert_eq!(*d.decline_list(), *original.decline_list());
    // Original pacing: one state per idle.
    let o = developer(&host, &st, exemplar(0x1010, 1, 1));
    o.start_map_update();
    assert!(o.handle_message(&idle));
    assert!(!o.is_map_update_finished());
}
