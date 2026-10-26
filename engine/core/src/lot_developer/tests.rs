// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests for the lot developer: the pure formulas of §11.4 / §7.3, the
//! tuning reader, the save record (§13), the desirability tolerance (§5.1),
//! and the behaviour of §4–§9 on a scripted host. The observable details of
//! every case are pinned by the conformance vectors (§16); these tests check
//! the core directly and guard against panics on hostile input.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use super::*;
use crate::property::PropertyHolder;
use crate::rng::{derive_seed, file_seed, FixedSeeds};
use crate::stream::{ByteReader, ByteWriter};

const SEEDS: FixedSeeds = FixedSeeds {
    wall_clock: 1,
    process_clock: 1,
};

/// The spec's file-RNG seed for the conformance vectors (§16).
const TEST_SEED: u32 = 12345;

// ---- pure helpers -------------------------------------------------------------------------------

#[test]
fn geometry_is_total_and_matches_spec() {
    // §16 test 10: 1x1 lot, altitude 100 except (1, 0) = 104: slope 4.0.
    let mut alt = |x: i32, z: i32| if x == 1 && z == 0 { 104.0f32 } else { 100.0 };
    let slope = area_slope_of(CellRect::new(0, 0, 0, 0), &mut alt);
    assert!((f64::from(slope) - 4.0).abs() < 1e-5, "slope {slope}");
    // A flat rectangle has no slope (lo = hi, distance 0).
    let mut flat = |_: i32, _: i32| 50.0f32;
    assert_eq!(area_slope_of(CellRect::new(0, 0, 3, 3), &mut flat), 0.0);
    // Hostile rectangles never panic.
    for r in [
        CellRect::new(i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        CellRect::new(i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        CellRect::new(-5, -5, -5, -5),
    ] {
        let mut a = |_: i32, _: i32| f32::NAN;
        let _ = area_slope_of(r, &mut a);
    }
}

#[test]
fn vc29_slope_keeps_register_precision_until_float_sink() {
    use crate::x87::Precision::{Bits24, Bits53};
    let mut altitude = |x, z| match (x, z) {
        (0, 0) => 0.0,
        (3, 4) => 13.0,
        _ => 6.0,
    };
    for (pc, bits) in [(Bits24, 0x40ED_B6DB), (Bits53, 0x40ED_B6DC)] {
        assert_eq!(
            super::geometry::area_slope_with_precision(CellRect::new(0, 0, 2, 3), &mut altitude, pc).to_bits(),
            bits
        );
    }
}

#[test]
fn development_cost_matches_spec() {
    let shipped = Tuning::shipped_exemplar();
    let mut t = Tuning::default();
    let mut acc = 0u32;
    let mut batch = 0u32;
    t.read(Some(&shipped), &mut acc, &mut batch);
    // §16 test 10: 1x1, slope 4.0 -> s = 0.25 -> curve 4.499996 -> cost 4.
    let cost = development_cost(CellRect::new(0, 0, 0, 0), 4.0, &t.cost_curve);
    assert_eq!(cost, 4, "cost {cost}");
    // A flat rectangle: slope 0 -> curve 0 -> cost 0.
    assert_eq!(development_cost(CellRect::new(0, 0, 1, 1), 0.0, &t.cost_curve), 0);
    // An empty curve evaluates to 0 (§3.5 [choice]).
    let empty = Tuning::default();
    assert_eq!(development_cost(CellRect::new(0, 0, 4, 4), 100.0, &empty.cost_curve), 0);
}

#[test]
fn rubble_count_matches_spec() {
    // §16 test 11: 11x11 -> area 121, count trunc(0.9453125) + 4 = 4, not
    // small, not large.
    let s = rubble_count([0.0, 0.0, 0.0], [11.0, 0.0, 11.0]);
    assert_eq!(s.count, 4);
    assert!(!s.small);
    assert!(!s.large);
    // 8x8 -> small, no draws.
    let s = rubble_count([0.0, 0.0, 0.0], [8.0, 0.0, 8.0]);
    assert!(s.small);
    // 16x16 -> count 6 (two interior props), large.
    let s = rubble_count([0.0, 0.0, 0.0], [16.0, 0.0, 16.0]);
    assert_eq!(s.count, 6);
    assert!(s.large);
    assert!(!s.small);
    // A hostile box never allocates billions (MAX_RUBBLE_PROPS).
    let s = rubble_count([0.0, 0.0, 0.0], [1e9, 0.0, 1e9]);
    assert!(s.count <= crate::lot_developer::geometry::MAX_RUBBLE_PROPS);
    // NaN -> not small, not large (comparisons false), count 4.
    let s = rubble_count([f32::NAN, 0.0, 0.0], [11.0, 0.0, 11.0]);
    assert_eq!(s.count, 4);
    assert!(!s.small);
    assert!(!s.large);
}

#[test]
fn empty_lot_zot_pattern() {
    // §16 test 8: zone 1 at (2, 2) yes, (1, 2) no; zone 8 at (4, 8) yes,
    // (4, 6) no; zone 10 no.
    assert!(is_empty_lot_suitable_for_zot(1, 2, 2));
    assert!(!is_empty_lot_suitable_for_zot(1, 1, 2));
    assert!(is_empty_lot_suitable_for_zot(8, 4, 8));
    assert!(!is_empty_lot_suitable_for_zot(8, 4, 6));
    assert!(!is_empty_lot_suitable_for_zot(10, 0, 0));
    assert!(!is_empty_lot_suitable_for_zot(0, 0, 0));
    // Negative coordinates use the C remainder (truncating).
    assert!(is_empty_lot_suitable_for_zot(1, -4, 0));
}

#[test]
fn query_interface_answers() {
    assert!(answers_iid(IID_UNKNOWN));
    assert!(answers_iid(IID_LOT_DEVELOPER));
    assert!(answers_iid(IID_MESSAGE_TARGET2));
    assert!(answers_iid(IID_SERIALIZABLE));
    assert!(!answers_iid(CLSID_LOT_DEVELOPER));
    assert!(!answers_iid(0x1234_5678));
    assert_eq!(SIMULATOR_TYPE, 0x6818_E5C9);
}

// ---- tuning, seeding ----------------------------------------------------------------------------

#[test]
fn tuning_defaults_and_shipped() {
    let mut t = Tuning::default();
    assert_eq!(t.stay_new_time, 90);
    assert_eq!(t.thresholds, [0.2, 0.4, 0.6]);
    assert_eq!(t.default_lot_foundation_type, DEFAULT_FOUNDATION_TYPE);
    // No exemplar: nothing is read.
    let mut acc = 200u32;
    let mut batch = 8u32;
    t.read(None, &mut acc, &mut batch);
    assert_eq!(acc, 200);
    assert_eq!(batch, 8);
    // The shipped exemplar.
    let shipped = Tuning::shipped_exemplar();
    t.read(Some(&shipped), &mut acc, &mut batch);
    assert_eq!(t.stay_new_time, 90);
    assert_eq!(t.thresholds, [0.3, 0.45, 0.5]);
    assert_eq!(acc, 100);
    assert_eq!(batch, 10);
    assert!(t.cost_curve.eval(0.25) > 4.0 && t.cost_curve.eval(0.25) < 5.0);
    // The thresholds are re-set to the defaults before every read.
    let mut empty = PropertyHolder::new();
    empty.add(
        prop::STAY_NEW_TIME,
        crate::variant::Variant::Scalar(crate::variant::Elem::U16(40)),
    );
    let mut acc = 0u32;
    let mut batch = 0u32;
    t.read(Some(&empty), &mut acc, &mut batch);
    assert_eq!(t.stay_new_time, 40);
    assert_eq!(t.thresholds, [0.2, 0.4, 0.6]);
}

#[test]
fn seeding_derives_from_session() {
    // foundations §6.3: derive_seed(session, 0x6C09C0F4).
    let from_session = LotDeveloperStatics::from_session(0x1234_5678, SEEDS);
    let expected = derive_seed(0x1234_5678, file_seed::LOT_DEVELOPER);
    let explicit = LotDeveloperStatics::new(expected, SEEDS);
    assert_eq!(from_session.borrow().rng.lcg_state(), explicit.borrow().rng.lcg_state());
    // The two session seeds that would collide with the clock fallbacks are
    // moved by XOR 1, so the state is never 0 or 0xFFFFFFFF.
    for s in [0u32, file_seed::LOT_DEVELOPER, file_seed::LOT_DEVELOPER ^ 0xFFFF_FFFF] {
        let st = LotDeveloperStatics::from_session(s, SEEDS);
        let lcg = st.borrow().rng.lcg_state();
        assert_ne!(lcg, 0);
        assert_ne!(lcg, 0xFFFF_FFFF);
    }
}

#[test]
fn desirability_tolerance_table() {
    // §16 test 1: the fixed tolerance per lot location.
    assert_eq!(LotDeveloper::desirability_tolerance(0, 0, &SEEDS), 2);
    assert_eq!(LotDeveloper::desirability_tolerance(1, 0, &SEEDS), 3);
    assert_eq!(LotDeveloper::desirability_tolerance(17, 3, &SEEDS), 4);
    assert_eq!(LotDeveloper::desirability_tolerance(10, 20, &SEEDS), 5);
    assert_eq!(LotDeveloper::desirability_tolerance(64, 37, &SEEDS), 5);
    assert_eq!(LotDeveloper::desirability_tolerance(255, 255, &SEEDS), 5);
}

// ---- a scripted host ----------------------------------------------------------------------------

/// One lot in the scripted world.
#[derive(Clone, Debug)]
struct Lot {
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    zone: i32,
    state: u32,
    livable: bool,
    condition: u32,
    days: u32,
    building_type: u32,
    building: Option<u64>,
    cfg: Option<u64>,
    wealth: u32,
    occupancy: f32,
    needs_power: bool,
    has_power: bool,
    road: bool,
    job: bool,
    organic: bool,
    flags: u32,
    historical: bool,
    travel_jobs: [f32; 4],
    designate: Option<u64>,
    unit: Option<u64>,
    initialized: bool,
    props: BTreeMap<u32, u32>,
    altitude: f32,
    facing: i32,
}

impl Default for Lot {
    fn default() -> Lot {
        Lot {
            x: 0,
            z: 0,
            w: 1,
            h: 1,
            zone: 1,
            state: lot_state::READY,
            livable: true,
            condition: 0,
            days: 0,
            building_type: 0,
            building: None,
            cfg: None,
            wealth: 1,
            occupancy: 1.0,
            needs_power: true,
            has_power: false,
            road: true,
            job: true,
            organic: true,
            flags: 0,
            historical: false,
            travel_jobs: [0.0, 1.0, 0.0, 0.0],
            designate: None,
            unit: None,
            initialized: true,
            props: BTreeMap::new(),
            altitude: 0.0,
            facing: 0,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Cfg {
    id: u32,
    w: u32,
    h: u32,
    stage: u8,
    water: bool,
    max_slope: f32,
    objects: Vec<CfgObject>,
    primary_building: Option<Vec<u32>>,
    primary_foundation: Option<Vec<u32>>,
    water_objects: bool,
}

#[derive(Clone, Debug, Default)]
struct Occ {
    bbox: (Vec3, Vec3),
    cells: CellRect,
    group: u32,
    lit: bool,
    props: BTreeMap<u32, u32>,
    position: Vec3,
    orientation: u32,
    is_building: bool,
    /// `Some(kind)` for a foundation occupant (§17.5).
    foundation_kind: Option<i32>,
    foundation_type: u32,
}

#[derive(Clone, Debug, Default)]
struct Profile {
    purpose: u8,
    wealth: u8,
    capacity: u16,
}

#[derive(Clone, Debug, Default)]
struct Dev {
    /// The developer type key `(purpose << 8) | wealth`.
    key: u32,
    purpose: u32,
    desirability: i16,
    water_cap: u8,
}

/// The scripted world: lots, configurations, occupants, units, cell grids,
/// an event log of the calls that change something, and the services the
/// core reaches.
struct Script {
    lots: RefCell<BTreeMap<u64, Lot>>,
    order: RefCell<Vec<u64>>,
    cfgs: RefCell<BTreeMap<u64, Cfg>>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    /// Lot slot 49 per lot, and lot slot 11 per occupant (LD17).
    lot_occs: RefCell<BTreeMap<u64, Vec<u64>>>,
    occ_objects: RefCell<BTreeMap<u64, CfgObject>>,
    /// Occupant types other than building (LD17).
    occ_types: RefCell<BTreeMap<u64, u32>>,
    profiles: RefCell<BTreeMap<u32, Profile>>,
    families: RefCell<BTreeMap<u32, Vec<u32>>>,
    valid_buildings: RefCell<BTreeMap<u32, bool>>,
    devs: RefCell<BTreeMap<u64, Dev>>,
    units: RefCell<BTreeMap<u64, Vec<u64>>>,
    textures: RefCell<Vec<u32>>,
    events: RefCell<Vec<String>>,
    next: Cell<u64>,
    // services and world state
    no_services: Cell<bool>,
    no_simulator: Cell<bool>,
    no_message_server: Cell<bool>,
    city_cells: Cell<(i32, i32)>,
    cell_widths: Cell<(f32, f32)>,
    terrain_cells: Cell<(i32, i32)>,
    terrain_width: Cell<f32>,
    sim_date: Cell<u32>,
    time_mode: Cell<u32>,
    view_rotation: Cell<i32>,
    powered_cells: RefCell<BTreeMap<(i32, i32), bool>>,
    watered_cells: RefCell<BTreeMap<(i32, i32), bool>>,
    trip_cells: RefCell<BTreeMap<(i32, i32), u8>>,
    terrain_alts: RefCell<BTreeMap<i32, f32>>,
    terrain_water: RefCell<BTreeMap<i32, f32>>,
    /// `GetAltitude(x, z)` at any position.
    pos_alt: Cell<f32>,
    jobs_demand: Cell<Option<f32>>,
    cheats: Cell<[bool; 3]>,
    resources: RefCell<BTreeMap<u32, bool>>,
    surrounding: RefCell<BTreeMap<u64, Vec<u64>>>,
    surrounding_rect: RefCell<Vec<u64>>,
    lots_in_rect: RefCell<Vec<u64>>,
    /// The rectangles `occupants_in_cells` was asked for.
    cell_queries: RefCell<Vec<CellRect>>,
    claim_resources: Cell<bool>,
    add_building_fails: Cell<bool>,
    // tuning
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    // iteration
    iters: RefCell<BTreeMap<u64, Cell<usize>>>,
    iter_next: Cell<u64>,
}

impl Default for Script {
    fn default() -> Script {
        Script {
            lots: RefCell::new(BTreeMap::new()),
            order: RefCell::new(Vec::new()),
            cfgs: RefCell::new(BTreeMap::new()),
            occs: RefCell::new(BTreeMap::new()),
            lot_occs: RefCell::new(BTreeMap::new()),
            occ_objects: RefCell::new(BTreeMap::new()),
            occ_types: RefCell::new(BTreeMap::new()),
            profiles: RefCell::new(BTreeMap::new()),
            families: RefCell::new(BTreeMap::new()),
            valid_buildings: RefCell::new(BTreeMap::new()),
            devs: RefCell::new(BTreeMap::new()),
            units: RefCell::new(BTreeMap::new()),
            textures: RefCell::new(Vec::new()),
            events: RefCell::new(Vec::new()),
            next: Cell::new(1),
            no_services: Cell::new(false),
            no_simulator: Cell::new(false),
            no_message_server: Cell::new(false),
            city_cells: Cell::new((64, 64)),
            cell_widths: Cell::new((16.0, 16.0)),
            terrain_cells: Cell::new((64, 64)),
            terrain_width: Cell::new(16.0),
            sim_date: Cell::new(0),
            time_mode: Cell::new(0),
            view_rotation: Cell::new(0),
            powered_cells: RefCell::new(BTreeMap::new()),
            watered_cells: RefCell::new(BTreeMap::new()),
            trip_cells: RefCell::new(BTreeMap::new()),
            terrain_alts: RefCell::new(BTreeMap::new()),
            terrain_water: RefCell::new(BTreeMap::new()),
            pos_alt: Cell::new(100.0),
            jobs_demand: Cell::new(None),
            cheats: Cell::new([false; 3]),
            resources: RefCell::new(BTreeMap::new()),
            surrounding: RefCell::new(BTreeMap::new()),
            surrounding_rect: RefCell::new(Vec::new()),
            lots_in_rect: RefCell::new(Vec::new()),
            cell_queries: RefCell::new(Vec::new()),
            claim_resources: Cell::new(true),
            add_building_fails: Cell::new(false),
            tuning: RefCell::new(Some(Rc::new(Tuning::shipped_exemplar()))),
            iters: RefCell::new(BTreeMap::new()),
            iter_next: Cell::new(0x9000),
        }
    }
}

impl Script {
    fn say(&self, s: String) {
        self.events.borrow_mut().push(s);
    }
    fn token(&self) -> u64 {
        let t = self.next.get();
        self.next.set(t + 1);
        t
    }
    /// Adds a lot with a builder; returns its token.
    fn add_lot(&self, f: impl FnOnce(&mut Lot)) -> u64 {
        let mut l = Lot::default();
        f(&mut l);
        let t = self.token();
        self.lots.borrow_mut().insert(t, l);
        self.order.borrow_mut().push(t);
        t
    }
    fn lot<T>(&self, l: u64, f: impl FnOnce(&Lot) -> T, def: T) -> T {
        self.lots.borrow().get(&l).map_or(def, f)
    }
    fn lot_mut<T>(&self, l: u64, f: impl FnOnce(&mut Lot) -> T, def: T) -> T {
        self.lots.borrow_mut().get_mut(&l).map_or(def, f)
    }
    fn add_occ(&self, f: impl FnOnce(&mut Occ)) -> u64 {
        let mut o = Occ::default();
        f(&mut o);
        let t = self.token();
        self.occs.borrow_mut().insert(t, o);
        t
    }
    fn events(&self) -> Vec<String> {
        self.events.borrow().clone()
    }
    fn clear_events(&self) {
        self.events.borrow_mut().clear();
    }
}

impl LotDeveloperHost for Script {
    fn acquire_services(&self) -> bool {
        !self.no_services.get()
    }
    fn release_services(&self) {
        self.say("release_services".into());
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn has_simulator(&self) -> bool {
        !self.no_simulator.get()
    }
    fn add_agent(&self, level: u32, message: u32) {
        self.say(format!("add_agent {level} {message:#x}"));
    }
    fn remove_agent(&self) {
        self.say("remove_agent".into());
    }
    fn acquire_message_server(&self) -> bool {
        !self.no_message_server.get()
    }
    fn release_message_server(&self) {
        self.say("release_message_server".into());
    }
    fn subscribe(&self, message: u32) {
        self.say(format!("subscribe {message:#x}"));
    }
    fn unsubscribe(&self, message: u32) {
        self.say(format!("unsubscribe {message:#x}"));
    }
    fn register_animation_tick(&self) {
        self.say("register_tick".into());
    }
    fn unregister_animation_tick(&self) {
        self.say("unregister_tick".into());
    }
    fn post_message(&self, msg_type: u32, data1: u32, lot: Option<u64>) {
        self.say(format!("post {msg_type:#x} d1={data1} lot={lot:?}"));
    }

    fn city_cell_counts(&self) -> (i32, i32) {
        self.city_cells.get()
    }
    fn city_cell_widths(&self) -> (f32, f32) {
        self.cell_widths.get()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let (cx, _cz) = self.cell_widths.get();
        ((x / cx).floor() as i32, (z / cx).floor() as i32)
    }
    fn cell_corner_to_position(&self, x: i32, z: i32) -> (f32, f32) {
        let (cx, cz) = self.cell_widths.get();
        (x as f32 * cx, z as f32 * cz)
    }
    fn cell_center_to_position(&self, x: i32, z: i32) -> (f32, f32) {
        let (cx, cz) = self.cell_widths.get();
        ((x as f32 + 0.5) * cx, (z as f32 + 0.5) * cz)
    }
    fn sim_date(&self) -> u32 {
        self.sim_date.get()
    }
    fn terrain_cell_counts(&self) -> (i32, i32) {
        self.terrain_cells.get()
    }
    fn terrain_cell_width(&self) -> f32 {
        self.terrain_width.get()
    }
    fn terrain_altitude(&self, index: i32) -> f32 {
        self.terrain_alts.borrow().get(&index).copied().unwrap_or(100.0)
    }
    fn terrain_water_level(&self, index: i32) -> f32 {
        self.terrain_water.borrow().get(&index).copied().unwrap_or(0.0)
    }
    fn terrain_altitude_at(&self, x: f32, z: f32) -> f32 {
        self.say(format!("altitude_at {x} {z}"));
        self.pos_alt.get()
    }
    fn terrain_water_level_at(&self, _x: f32, _z: f32) -> f32 {
        0.0
    }
    fn terrain_is_water_vertex(&self, _x: i32, _z: i32) -> bool {
        false
    }
    fn terrain_cell_has_water(&self, _x: i32, _z: i32) -> bool {
        false
    }
    fn zone_drag_color(&self, _zone: i32) -> u32 {
        0xFFB2_3300
    }
    fn zone_display_alpha(&self) -> u8 {
        0xAA
    }
    fn view_rotation(&self) -> i32 {
        self.view_rotation.get()
    }
    fn visual_time_mode(&self) -> u32 {
        self.time_mode.get()
    }
    fn resource_exists(&self, _t: u32, _g: u32, instance: u32) -> bool {
        self.resources.borrow().get(&instance).copied().unwrap_or(false)
    }

    fn lot_count(&self) -> i32 {
        self.order.borrow().len() as i32
    }
    fn safe_lot_iterator(&self) -> u64 {
        let t = self.iter_next.get();
        self.iter_next.set(t + 1);
        self.iters.borrow_mut().insert(t, Cell::new(0));
        t
    }
    fn safe_lot_iterator_rewind(&self, it: u64) {
        if let Some(c) = self.iters.borrow().get(&it) {
            c.set(0);
        }
    }
    fn safe_lot_iterator_next(&self, it: u64) -> Option<u64> {
        let idx = self.iters.borrow().get(&it)?.get();
        let order = self.order.borrow();
        let lot = order.get(idx)?;
        self.iters.borrow().get(&it)?.set(idx + 1);
        Some(*lot)
    }
    fn safe_lot_iterator_free(&self, it: u64) {
        self.iters.borrow_mut().remove(&it);
    }
    fn get_lot(&self, x: i32, z: i32, _origin: bool) -> Option<u64> {
        self.lots
            .borrow()
            .iter()
            .find(|(_, l)| l.x == x && l.z == z)
            .map(|(&t, _)| t)
    }
    fn lots_in_cell_rect(&self, _rect: CellRect, _origin: bool) -> Vec<u64> {
        self.lots_in_rect.borrow().clone()
    }
    fn lots_surrounding_lot(&self, lot: u64, list: &mut Vec<u64>, _flags: u32) {
        if let Some(v) = self.surrounding.borrow().get(&lot) {
            list.extend_from_slice(v);
        }
    }
    fn lots_surrounding_rect(&self, _rect: CellRect, list: &mut Vec<u64>, _flags: u32) {
        list.extend_from_slice(&self.surrounding_rect.borrow());
    }
    fn delete_lot(&self, lot: u64) -> bool {
        self.say(format!("delete_lot {lot}"));
        self.order.borrow_mut().retain(|&l| l != lot);
        self.lots.borrow_mut().remove(&lot);
        true
    }

    fn lot_is_initialized(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.initialized, true)
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        self.lot(lot, |l| (l.x, l.z), (0, 0))
    }
    fn lot_size(&self, lot: u64) -> (u32, u32) {
        self.lot(lot, |l| (l.w, l.h), (0, 0))
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.lot(
            lot,
            |l| CellRect::new(l.x, l.z, l.x + l.w as i32 - 1, l.z + l.h as i32 - 1),
            CellRect::new(0, 0, 0, 0),
        )
    }
    fn lot_center(&self, lot: u64) -> Vec3 {
        self.lot(lot, |l| [l.x as f32 * 16.0, 0.0, l.z as f32 * 16.0], [0.0, 0.0, 0.0])
    }
    fn lot_zone_type(&self, lot: u64) -> i32 {
        self.lot(lot, |l| l.zone, 0)
    }
    fn lot_altitude(&self, lot: u64) -> f32 {
        self.lot(lot, |l| l.altitude, 0.0)
    }
    fn lot_set_altitude(&self, lot: u64, altitude: f32) {
        self.lot_mut(lot, |l| l.altitude = altitude, ());
    }
    fn lot_facing(&self, lot: u64) -> i32 {
        self.lot(lot, |l| l.facing, 0)
    }
    fn lot_flags(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.flags, 0)
    }
    fn lot_set_flags(&self, lot: u64, flags: u32) {
        self.lot_mut(lot, |l| l.flags = flags, ());
    }
    fn lot_set_vertices_locked(&self, lot: u64, locked: bool) {
        self.say(format!("vertices_locked {lot} {locked}"));
    }
    fn lot_update_slope(&self, lot: u64) {
        self.say(format!("update_slope {lot}"));
    }
    fn lot_state(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.state, 0)
    }
    fn lot_set_state(&self, lot: u64, state: u32, _wealth: u32, why: u32) -> bool {
        self.say(format!("set_state {lot} {state} reason={why}"));
        self.lot_mut(lot, |l| l.state = state, ());
        true
    }
    fn lot_condition(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.condition, 0)
    }
    fn lot_set_condition(&self, lot: u64, condition: u32) {
        self.lot_mut(lot, |l| l.condition = condition, ());
    }
    fn lot_is_livable(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.livable, false)
    }
    fn lot_days_since_state_change(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.days, 0)
    }
    fn lot_building_type(&self, lot: u64, _flag: bool) -> u32 {
        self.lot(lot, |l| l.building_type, 0)
    }
    fn lot_set_building_type(&self, lot: u64, building: u32) {
        self.lot_mut(lot, |l| l.building_type = building, ());
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        self.lot(lot, |l| l.building, None)
    }
    fn lot_configuration(&self, lot: u64) -> Option<u64> {
        self.lot(lot, |l| l.cfg, None)
    }
    fn lot_set_configuration(&self, lot: u64, cfg: Option<u64>) {
        self.lot_mut(lot, |l| l.cfg = cfg, ());
    }
    fn lot_occupant_wealth(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.wealth, 0)
    }
    fn lot_occupancy(&self, lot: u64) -> f32 {
        self.lot(lot, |l| l.occupancy, 0.0)
    }
    fn lot_needs_power(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.needs_power, false)
    }
    fn lot_has_power(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.has_power, false)
    }
    fn lot_set_has_power(&self, lot: u64, powered: bool) {
        self.lot_mut(lot, |l| l.has_power = powered, ());
    }
    fn lot_has_job_access(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.job, false)
    }
    fn lot_has_road_access(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.road, false)
    }
    fn lot_has_organic_road_access(&self, lot: u64, _purpose: u32) -> bool {
        self.lot(lot, |l| l.organic, false)
    }
    fn lot_update_road_access(&self, lot: u64) {
        self.say(format!("update_road_access {lot}"));
    }
    fn lot_travel_jobs(&self, lot: u64, wealth: u32) -> f32 {
        self.lot(lot, |l| l.travel_jobs[wealth.min(3) as usize], 0.0)
    }
    fn lot_travel_designate(&self, lot: u64) -> Option<u64> {
        self.lot(lot, |l| l.designate, None)
    }
    fn lot_set_travel_designate(&self, lot: u64, designate: Option<u64>) {
        self.lot_mut(lot, |l| l.designate = designate, ());
    }
    fn lot_unit(&self, lot: u64) -> Option<u64> {
        self.lot(lot, |l| l.unit, None)
    }
    fn lot_reset_travel_population(&self, lot: u64) {
        self.say(format!("reset_travel_pop {lot}"));
    }
    fn lot_add_travel_population(&self, lot: u64, from: u64) {
        self.say(format!("add_travel_pop {lot} <- {from}"));
    }
    fn lot_is_historical(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.historical, false)
    }
    fn lot_set_historical(&self, lot: u64, historical: bool) {
        self.lot_mut(lot, |l| l.historical = historical, ());
    }
    fn lot_available_capacity(&self, _lot: u64, _dev: u32, _flag: bool) -> u16 {
        10
    }
    fn lot_set_population(&self, lot: u64, dev: u32, population: u16) -> bool {
        self.say(format!("set_population {lot} dev={dev} n={population}"));
        true
    }
    fn lot_property(&self, lot: u64, id: u32) -> Option<u32> {
        self.lot(lot, |l| l.props.get(&id).copied(), None)
    }
    fn lot_set_property(&self, lot: u64, id: u32, value: u32) {
        self.lot_mut(lot, |l| l.props.insert(id, value), None);
    }

    fn cfg_id(&self, cfg: u64) -> u32 {
        self.cfgs.borrow().get(&cfg).map_or(0, |c| c.id)
    }
    fn cfg_size(&self, cfg: u64) -> (u32, u32) {
        self.cfgs.borrow().get(&cfg).map_or((0, 0), |c| (c.w, c.h))
    }
    fn cfg_max_slope_before_foundation(&self, cfg: u64) -> f32 {
        self.cfgs.borrow().get(&cfg).map_or(0.0, |c| c.max_slope)
    }
    fn cfg_growth_stage(&self, cfg: u64) -> u8 {
        self.cfgs.borrow().get(&cfg).map_or(0, |c| c.stage)
    }
    fn cfg_is_compatible_with_water(&self, cfg: u64) -> bool {
        self.cfgs.borrow().get(&cfg).is_some_and(|c| c.water)
    }
    fn cfg_objects(&self, cfg: u64) -> Vec<CfgObject> {
        self.cfgs.borrow().get(&cfg).map_or(Vec::new(), |c| c.objects.clone())
    }
    fn cfg_primary_building_object(&self, cfg: u64) -> Option<CfgObject> {
        self.cfgs
            .borrow()
            .get(&cfg)
            .and_then(|c| c.primary_building.clone())
            .map(|ids| CfgObject {
                ids,
                ..CfgObject::default()
            })
    }
    fn cfg_primary_foundation_object(&self, cfg: u64) -> Option<CfgObject> {
        self.cfgs
            .borrow()
            .get(&cfg)
            .and_then(|c| c.primary_foundation.clone())
            .map(|ids| CfgObject {
                ids,
                ..CfgObject::default()
            })
    }
    fn cfg_has_objects_of_type(&self, cfg: u64, object_type: u32) -> bool {
        if object_type == object_type::WATER {
            return self.cfgs.borrow().get(&cfg).is_some_and(|c| c.water_objects);
        }
        self.cfgs
            .borrow()
            .get(&cfg)
            .is_some_and(|c| c.objects.iter().any(|o| o.object_type == object_type))
    }
    fn get_lot_configuration(&self, id: u32) -> Option<u64> {
        self.cfgs.borrow().iter().find(|(_, c)| c.id == id).map(|(&t, _)| t)
    }
    fn object_id_array(&self, cfg: u64, index: usize) -> Vec<u32> {
        self.cfgs
            .borrow()
            .get(&cfg)
            .and_then(|c| c.objects.get(index))
            .map_or(Vec::new(), |o| o.ids.clone())
    }
    fn first_building(&self, lot: u64) -> Option<u64> {
        self.lot(lot, |l| l.building, None)
    }
    fn update_lot_retaining_walls(&self, lot: u64) -> bool {
        self.say(format!("update_retaining_walls {lot}"));
        true
    }
    fn lot_occupants(&self, lot: u64) -> Vec<u64> {
        self.say(format!("lot_occupants {lot}"));
        self.lot_occs.borrow().get(&lot).cloned().unwrap_or_default()
    }
    fn lot_configuration_object(&self, _lot: u64, occ: u64) -> Option<CfgObject> {
        self.occ_objects.borrow().get(&occ).cloned()
    }
    fn update_lot_base_texture_occupant(&self, lot: u64, had: bool) -> bool {
        self.say(format!("update_base_texture_occupant {lot} had={had}"));
        true
    }
    fn base_texture_occupant(&self, _lot: u64, _create: bool) -> Option<u64> {
        Some(0xB0)
    }
    fn base_texture_set(&self, _occ: u64, records: &[TextureRecord]) {
        self.say(format!("set_textures n={}", records.len()));
        for r in records {
            self.say(format!("set_texture {:#x} @{},{}", r.texture, r.x, r.z));
        }
        self.textures.borrow_mut().extend(records.iter().map(|r| r.texture));
    }
    fn add_retaining_wall_occupant(&self, lot: u64, flag: u32) {
        self.say(format!("retaining_wall {lot} {flag}"));
    }
    fn update_surrounding_retaining_walls(&self, lot: u64) {
        self.say(format!("update_surrounding_walls {lot}"));
    }
    fn add_network_connection_nodes(&self, lot: u64, nodes: &[ConnectionNode]) {
        self.say(format!("add_network_nodes {lot} n={}", nodes.len()));
        for n in nodes {
            self.say(format!(
                "node {:#x} @{},{} {:#x} {:#x} {:#x} o={}",
                n.node_type, n.x, n.z, n.b, n.c, n.d, n.orientation
            ));
        }
    }
    fn remove_network_connection_nodes(&self, lot: u64) {
        self.say(format!("remove_network_nodes {lot}"));
    }
    fn start_construction_occupant(&self, lot: u64) {
        self.say(format!("construction_occupant {lot}"));
    }

    fn occupant_as_building(&self, occ: u64) -> Option<u64> {
        self.occs.borrow().get(&occ).and_then(|o| o.is_building.then_some(occ))
    }
    fn building_add_ref(&self, _b: u64) {}
    fn building_release(&self, _b: u64) {}
    fn building_set_lit(&self, b: u64, lit: bool) {
        self.say(format!("set_lit {b} {lit}"));
        if let Some(o) = self.occs.borrow_mut().get_mut(&b) {
            o.lit = lit;
        }
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        if let Some(&t) = self.occ_types.borrow().get(&occ) {
            return t;
        }
        self.occs
            .borrow()
            .get(&occ)
            .map_or(0, |o| if o.is_building { OCCUPANT_TYPE_BUILDING } else { 0 })
    }
    fn occupant_position(&self, occ: u64) -> Vec3 {
        self.occs.borrow().get(&occ).map_or([0.0; 3], |o| o.position)
    }
    fn occupant_bounding_box(&self, occ: u64) -> (Vec3, Vec3) {
        self.occs.borrow().get(&occ).map_or(([0.0; 3], [0.0; 3]), |o| o.bbox)
    }
    fn occupant_city_cells(&self, occ: u64) -> CellRect {
        self.occs
            .borrow()
            .get(&occ)
            .map_or(CellRect::new(0, 0, 0, 0), |o| o.cells)
    }
    fn occupant_in_group(&self, occ: u64, group: u32) -> bool {
        self.occs.borrow().get(&occ).is_some_and(|o| o.group == group)
    }
    fn occupant_post_message(&self, occ: u64, message: u32) {
        self.say(format!("occ_post {occ} {message:#x}"));
    }
    fn occupant_set_removal_flags(&self, occ: u64, flags: u32) {
        self.say(format!("removal_flags {occ} {flags}"));
    }
    fn occupant_set_lot_tag(&self, occ: u64, tag: u32) {
        self.say(format!("set_lot_tag {occ} {tag:#x}"));
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<u32> {
        self.occs.borrow().get(&occ).and_then(|o| o.props.get(&id).copied())
    }
    fn occupant_set_property(&self, occ: u64, id: u32, value: u32) {
        self.occs.borrow_mut().get_mut(&occ).map(|o| o.props.insert(id, value));
    }
    fn occupant_init(&self, _occ: u64) {}
    fn occupant_set_flag(&self, _occ: u64, _flag: u32) {}
    fn occupant_is_flag_set(&self, _occ: u64, _flag: u32) -> bool {
        true
    }
    fn occupant_shutdown(&self, occ: u64) {
        self.say(format!("shutdown {occ}"));
    }
    fn occupant_set_orientation(&self, occ: u64, orientation: u32) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.orientation = orientation;
        }
    }
    fn occupant_set_position(&self, occ: u64, pos: Vec3) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.position = pos;
        }
    }
    fn occupant_insert(&self, occ: u64) {
        self.say(format!("insert_occupant {occ}"));
    }
    fn occupant_remove(&self, occ: u64, a: u32, flags: u32) {
        self.say(format!("remove_occupant {occ} {a} {flags}"));
        self.occs.borrow_mut().remove(&occ);
    }
    fn occupant_changed(&self, occ: u64) {
        self.say(format!("occupant_changed {occ}"));
    }
    fn occupant_moved(&self, occ: u64) {
        let y = self.occupant_position(occ)[1];
        self.say(format!("occupant_moved {occ} y={y}"));
    }
    fn occupants_in_bbox(&self, _min: Vec3, _max: Vec3) -> Vec<u64> {
        Vec::new()
    }
    fn occupants_in_cells(&self, rect: CellRect, t: u32) -> Vec<u64> {
        self.cell_queries.borrow_mut().push(rect);
        if t != OCCUPANT_TYPE_FOUNDATION {
            return Vec::new();
        }
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| {
                o.foundation_kind.is_some()
                    && o.cells.x1 <= rect.x2
                    && rect.x1 <= o.cells.x2
                    && o.cells.z1 <= rect.z2
                    && rect.z1 <= o.cells.z2
            })
            .map(|(&t, _)| t)
            .collect()
    }
    fn occupants_in_box(&self, _min: Vec3, _max: Vec3, _t: u32) -> Vec<u64> {
        Vec::new()
    }
    fn building_orientation(&self, b: u64) -> i32 {
        self.occs.borrow().get(&b).map_or(0, |o| o.orientation as i32)
    }
    fn foundation_kind(&self, occ: u64) -> Option<i32> {
        self.occs.borrow().get(&occ).and_then(|o| o.foundation_kind)
    }
    fn foundation_type(&self, occ: u64) -> u32 {
        self.occs.borrow().get(&occ).map_or(0, |o| o.foundation_type)
    }
    fn add_new_foundation_occupant(
        &self,
        foundation_type: u32,
        kind: i32,
        pos: &mut Vec3,
        _dx: f32,
        _dz: f32,
        _orient: i32,
        _bottom: f32,
    ) -> bool {
        let name = if kind == foundation_kind::LOT {
            "add_lot_foundation"
        } else {
            "add_building_foundation"
        };
        self.say(format!("{name} {foundation_type:#x} y={}", pos[1]));
        true
    }

    fn prop_create(&self, prop_type: u32) -> Option<u64> {
        self.say(format!("create_prop {prop_type:#x}"));
        Some(self.add_occ(|_| {}))
    }
    fn lot_props(&self, _lot: u64) -> Vec<u64> {
        // Every prop carrying a zot number, so RemoveLotProp can find them.
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| o.props.contains_key(&PROP_ZOT_NUMBER))
            .map(|(&t, _)| t)
            .collect()
    }
    fn prop_add_to_lot(&self, lot: u64, occ: u64) -> bool {
        self.say(format!("add_lot_prop {lot} {occ}"));
        true
    }
    fn prop_add(&self, prop_type: u32, _pos: Vec3, _o: i32) -> Option<u64> {
        self.say(format!("prop_add {prop_type:#x}"));
        Some(self.add_occ(|_| {}))
    }
    fn remove_lot_props(&self, lot: u64) {
        self.say(format!("remove_lot_props {lot}"));
    }
    fn prop_remove(&self, occ: u64) -> bool {
        self.say(format!("prop_remove {occ}"));
        self.occs.borrow_mut().remove(&occ);
        true
    }
    fn remove_building_props(&self, b: u64) {
        self.say(format!("remove_building_props {b}"));
    }
    fn make_burnt_building(&self, building: u64) -> Option<BurntBuilding> {
        let (min, max) = self.occupant_bounding_box(building);
        Some(BurntBuilding {
            occupant: building,
            min,
            max,
        })
    }
    fn add_rubble_decal(&self, _lot: u64, _b: &BurntBuilding, instance: u32, size: f32) {
        self.say(format!("rubble_decal {instance:#x} {size}"));
    }

    fn building_profile(&self, building: u32) -> BuildingProfile {
        self.profiles
            .borrow()
            .get(&building)
            .map_or(BuildingProfile::default(), |p| BuildingProfile {
                purpose: p.purpose,
                wealth: p.wealth,
                capacity: p.capacity,
                ..BuildingProfile::default()
            })
    }
    fn developer_type(&self, purpose: u32, wealth: u32) -> u32 {
        (purpose << 8) | wealth
    }
    fn developer_of_type(&self, dev_type: u32) -> Option<u64> {
        self.devs
            .borrow()
            .iter()
            .find(|(_, d)| d.key == dev_type)
            .map(|(&t, _)| t)
    }
    fn wealth_of_type(&self, dev_type: u32) -> u32 {
        dev_type & 0xFF
    }
    fn developer_for(&self, purpose: u32, wealth: u32) -> Option<u64> {
        let key = (purpose << 8) | wealth;
        self.devs.borrow().iter().find(|(_, d)| d.key == key).map(|(&t, _)| t)
    }
    fn is_building_type(&self, id: u32) -> bool {
        self.valid_buildings.borrow().get(&id).copied().unwrap_or(false)
    }
    fn building_types_by_family(&self, family: u32) -> Vec<u32> {
        self.families.borrow().get(&family).cloned().unwrap_or_default()
    }
    fn add_new_building_occupant(&self, building: u32, _pos: Vec3, _o: i32) -> Option<u64> {
        if self.add_building_fails.get() {
            return None;
        }
        self.say(format!("add_building {building:#x}"));
        Some(self.add_occ(|o| o.is_building = true))
    }
    fn claim_construction_resources(&self, _amount: u16) -> bool {
        self.claim_resources.get()
    }
    fn cheat(&self, cheat: Cheat) -> bool {
        let i = match cheat {
            Cheat::IgnorePlumbing => 0,
            Cheat::IgnorePower => 1,
            Cheat::CoffeeBar => 2,
        };
        self.cheats.get()[i]
    }

    fn dev_purpose(&self, dev: u64) -> u32 {
        self.devs.borrow().get(&dev).map_or(0, |d| d.purpose)
    }
    fn dev_desirability(&self, dev: u64, _x: i32, _z: i32) -> i16 {
        self.devs.borrow().get(&dev).map_or(0, |d| d.desirability)
    }
    fn dev_water_supply_cap(&self, dev: u64) -> u8 {
        self.devs.borrow().get(&dev).map_or(0, |d| d.water_cap)
    }

    fn is_powered(&self, x: i32, z: i32) -> bool {
        self.powered_cells.borrow().get(&(x, z)).copied().unwrap_or(false)
    }
    fn is_cell_watered(&self, x: i32, z: i32) -> bool {
        self.watered_cells.borrow().get(&(x, z)).copied().unwrap_or(false)
    }
    fn trip_length(&self, x: i32, z: i32) -> u8 {
        self.trip_cells.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn jobs_demand(&self) -> Option<f32> {
        self.jobs_demand.get()
    }

    fn flora_type_valid(&self, id: u32) -> bool {
        self.valid_buildings.borrow().get(&id).copied().unwrap_or(false)
    }
    fn flora_types_by_family(&self, id: u32) -> Option<Vec<u32>> {
        self.families.borrow().get(&id).cloned()
    }
    fn flora_add(&self, flora_type: u32, _pos: Vec3) -> Option<u64> {
        self.say(format!("flora_add {flora_type:#x}"));
        Some(self.add_occ(|_| {}))
    }
    fn flora_demolish_in_cell_area(&self, _rect: CellRect) {
        self.say("flora_demolish".into());
    }

    fn demolish_occupant(&self, occ: u64) -> bool {
        self.say(format!("demolish_occupant {occ}"));
        true
    }
    fn modify_terrain_height(
        &self,
        per_vertex: bool,
        x0: i32,
        z0: i32,
        x1: i32,
        z1: i32,
        h: f32,
        exempt: Option<u64>,
    ) -> i32 {
        self.say(format!(
            "modify_terrain {per_vertex} {x0},{z0},{x1},{z1} {h} {exempt:?}"
        ));
        1
    }
    fn demolish_lot(&self, lot: u64) -> bool {
        self.say(format!("demolish_lot {lot}"));
        true
    }

    fn unit_members(&self, unit: u64) -> Vec<u64> {
        self.units.borrow().get(&unit).cloned().unwrap_or_default()
    }
    fn unit_create(&self) -> u64 {
        let t = self.token();
        self.units.borrow_mut().insert(t, Vec::new());
        t
    }
    /// As `cSC4BusinessUnit::Add` (§5.7): no duplicate member; the lot's
    /// unit is set either way.
    fn unit_add(&self, unit: u64, lot: u64) -> bool {
        let mut units = self.units.borrow_mut();
        let v = units.entry(unit).or_default();
        if !v.contains(&lot) {
            v.push(lot);
        }
        drop(units);
        self.lot_mut(lot, |l| l.unit = Some(unit), ());
        true
    }
    fn unit_remove(&self, unit: u64, lot: u64) -> bool {
        let removed = self
            .units
            .borrow_mut()
            .get_mut(&unit)
            .is_some_and(|v| v.contains(&lot).then(|| v.retain(|&l| l != lot)).is_some());
        if removed {
            // `Remove` also clears the lot's unit and travel designate (§5.7).
            self.lot_mut(
                lot,
                |l| {
                    l.unit = None;
                    l.designate = None;
                },
                (),
            );
        }
        removed
    }

    fn building_ref(&self, building: u64) -> RefValue {
        RefValue::Host {
            token: building,
            class_id: IID_BUILDING_OCCUPANT,
        }
    }
    fn resolve_building(&self, r: &RefValue) -> Option<u64> {
        match r {
            RefValue::Host { token, class_id } if *class_id == IID_BUILDING_OCCUPANT => Some(*token),
            _ => None,
        }
    }
}

fn dev_with(host: Rc<Script>) -> LotDeveloper {
    LotDeveloper::new(host, LotDeveloperStatics::new(TEST_SEED, SEEDS))
}

// ---- lifecycle (§1.3, §1.4) ---------------------------------------------------------------------

#[test]
fn init_and_shutdown() {
    let host = Rc::new(Script::default());
    let mut d = dev_with(host.clone());
    assert!(!d.is_initialised());
    assert!(d.init());
    assert!(d.is_initialised());
    // The agent is registered at levels 0, 4, 6; the nine messages are
    // subscribed; the extents come from the terrain.
    let ev = host.events();
    assert!(ev.contains(&"add_agent 0 0x66956810".to_string()));
    assert!(ev.contains(&"add_agent 4 0x66956814".to_string()));
    assert!(ev.contains(&"add_agent 6 0x66956816".to_string()));
    assert_eq!(d.verts_x(), 65);
    // A second Init returns true at once, with no further calls.
    host.clear_events();
    assert!(d.init());
    assert!(host.events().is_empty());
    assert!(d.shutdown());
    assert!(!d.is_initialised());
    // A second Shutdown returns true with no calls.
    host.clear_events();
    assert!(d.shutdown());
    assert!(host.events().is_empty());
}

#[test]
fn init_fails_without_services() {
    let host = Rc::new(Script::default());
    host.no_services.set(true);
    let mut d = dev_with(host);
    assert!(!d.init());
    assert!(!d.is_initialised());

    let host = Rc::new(Script::default());
    host.no_simulator.set(true);
    let mut d = dev_with(host);
    assert!(!d.init());

    let host = Rc::new(Script::default());
    host.no_message_server.set(true);
    let mut d = dev_with(host);
    assert!(!d.init());
}

// ---- the daily pass (§4) ------------------------------------------------------------------------

#[test]
fn daily_quota() {
    // §16 test 3: n = 100 -> 4, n = 27 -> 1, n = 28 -> 2, n = 0 -> none.
    for (n, want) in [(100, 4), (27, 1), (28, 2), (56, 3)] {
        let host = Rc::new(Script::default());
        for _ in 0..n {
            host.add_lot(|_| {});
        }
        let d = dev_with(host);
        assert_eq!(d.per_day(), want, "n {n}");
    }
    let host = Rc::new(Script::default());
    let d = dev_with(host);
    assert_eq!(d.per_day(), 0);
}

#[test]
fn daily_pass_visits_quota_then_finishes() {
    let host = Rc::new(Script::default());
    // 30 lots -> perDay = 2. Each empty lot logs one "reset_travel_pop" per
    // visit (the business-unit step), so the events count the lots visited.
    for _ in 0..30 {
        host.add_lot(|_| {});
    }
    let mut d = dev_with(host.clone());
    d.set_daily_state(3);
    host.clear_events();
    assert!(d.do_daily_simulation());
    assert_eq!(d.daily_state(), 1);
    assert_eq!(
        host.events()
            .iter()
            .filter(|e| e.starts_with("reset_travel_pop"))
            .count(),
        2
    );
    // The next day visits the next two lots; the cycle is still running.
    host.clear_events();
    assert!(d.do_daily_simulation());
    assert_eq!(d.daily_state(), 1);
    assert_eq!(
        host.events()
            .iter()
            .filter(|e| e.starts_with("reset_travel_pop"))
            .count(),
        2
    );
}

#[test]
fn daily_pass_wraps_and_restarts() {
    // §16 test 4 (abridged): 29 lots -> perDay = 2. On the day the iterator
    // reaches the end the pass returns false with dailyState 3.
    let host = Rc::new(Script::default());
    for _ in 0..29 {
        host.add_lot(|_| {});
    }
    let mut d = dev_with(host.clone());
    d.set_daily_state(3);
    // 14 days of two lots each (28 lots), still in the cycle.
    for _ in 0..14 {
        assert!(d.do_daily_simulation());
        assert_eq!(d.daily_state(), 1);
    }
    // The 15th day reaches the last lot (L29), hits the end, sets state 3 and
    // returns false (visited 1 < perDay 2).
    assert!(!d.do_daily_simulation());
    assert_eq!(d.daily_state(), 3);
    // The re-delivery starts a new cycle from the front.
    assert!(d.do_daily_simulation());
    assert_eq!(d.daily_state(), 1);
}

#[test]
fn empty_city_pass() {
    let host = Rc::new(Script::default());
    let mut d = dev_with(host);
    assert!(d.do_daily_simulation());
    // n <= 0: no work, still starts a cycle the next time.
    assert_eq!(d.daily_state(), 3);
}

// ---- the tests (§5) -----------------------------------------------------------------------------

fn grown_lot(host: &Rc<Script>, purpose: u8, wealth: u8) -> (u64, u64) {
    // A developer of type (purpose << 8) | wealth and a matching profile,
    // with a high desirability so TestDesirability passes by default.
    let dev_key = (u32::from(purpose) << 8) | u32::from(wealth);
    let dev_token = host.token();
    host.devs.borrow_mut().insert(
        dev_token,
        Dev {
            key: dev_key,
            purpose: u32::from(purpose),
            desirability: 100,
            water_cap: 5,
        },
    );
    host.profiles.borrow_mut().insert(
        0x3000,
        Profile {
            purpose,
            wealth,
            capacity: 0,
        },
    );
    let building = host.add_occ(|o| {
        o.is_building = true;
        o.bbox = ([0.0, 0.0, 0.0], [16.0, 10.0, 16.0]);
    });
    let lot = host.add_lot(move |l| {
        l.state = lot_state::OCCUPIED;
        l.building_type = 0x3000;
        l.building = Some(building);
        l.wealth = u32::from(wealth);
        l.livable = true;
    });
    (lot, building)
}

#[test]
fn power_grace() {
    // §16 test 5: an unpowered building lot visited three times returns 1,
    // 1, 0; the zot property is 4 after the first; powering it resets and
    // returns 2.
    let host = Rc::new(Script::default());
    let (lot, _) = grown_lot(&host, 1, 1);
    let mut d = dev_with(host.clone());
    // Unpowered: needs_power true, no power cell set.
    assert_eq!(d.test_power(lot, 0, 0), 1);
    assert_eq!(
        host.lot(lot, |l| l.props.get(&lot_prop::ZOT).copied(), None),
        Some(zot::POWER)
    );
    assert_eq!(d.test_power(lot, 0, 0), 1);
    assert_eq!(d.test_power(lot, 0, 0), 0);
    // Power it: the counter resets and the zot is removed.
    host.powered_cells.borrow_mut().insert((0, 0), true);
    assert_eq!(d.test_power(lot, 0, 0), 2);
    assert_eq!(
        host.lot(lot, |l| l.props.get(&lot_prop::NO_POWER).copied(), None),
        Some(0)
    );
}

#[test]
fn travel_commute_and_road() {
    // §16 test 6: a residence in state 3 with road and job access but trip
    // length 0 returns true until the 7th visit, then false.
    let host = Rc::new(Script::default());
    let (lot, _) = grown_lot(&host, 1, 1);
    host.lot_mut(
        lot,
        |l| {
            l.state = lot_state::OCCUPIED;
            l.road = true;
            l.job = true;
        },
        (),
    );
    host.trip_cells.borrow_mut().insert((0, 0), 0);
    let mut d = dev_with(host.clone());
    let p = BuildingProfile::basic(1, 1, 0);
    let results: Vec<bool> = (0..7).map(|_| d.test_travel(lot, &p, 0, 0)).collect();
    assert_eq!(results, vec![true, true, true, true, true, true, false]);
    // The same lot without road access: true, true, false, with zot 6 from
    // the first visit and one 0x4C04DE69 data1 = 1 per visit.
    let host2 = Rc::new(Script::default());
    let (lot2, _) = grown_lot(&host2, 1, 1);
    host2.lot_mut(
        lot2,
        |l| {
            l.road = false;
        },
        (),
    );
    let mut d2 = dev_with(host2.clone());
    let r: Vec<bool> = (0..3).map(|_| d2.test_travel(lot2, &p, 0, 0)).collect();
    assert_eq!(r, vec![true, true, false]);
    assert_eq!(
        host2.lot(lot2, |l| l.props.get(&lot_prop::ZOT).copied(), None),
        Some(zot::ROAD)
    );
    let posts = host2
        .events()
        .iter()
        .filter(|e| e.starts_with("post 0x4c04de69 d1=1"))
        .count();
    assert_eq!(posts, 3);
}

#[test]
fn zot_priority() {
    // §16 test 7: current zot 4; request 6 -> still 4 (a data1=1 is posted);
    // request 7 -> still 4; current 7, request 5 -> removed, zot 5.
    let host = Rc::new(Script::default());
    let lot = host.add_lot(|l| {
        l.props.insert(lot_prop::ZOT, zot::POWER);
    });
    let mut d = dev_with(host.clone());
    d.add_building_zot_prop(lot, zot::ROAD_PROP, zot::ROAD);
    assert_eq!(d.current_zot(lot), zot::POWER);
    assert!(host.events().iter().any(|e| e.starts_with("post 0x4c04de69 d1=1")));
    host.clear_events();
    d.add_building_zot_prop(lot, zot::COMMUTE_PROP, zot::COMMUTE);
    assert_eq!(d.current_zot(lot), zot::POWER);
    // Now set 7 and request 5: lower wins.
    host.lot_set_property(lot, lot_prop::ZOT, zot::COMMUTE);
    d.add_building_zot_prop(lot, zot::WATER_PROP, zot::WATER);
    assert_eq!(d.current_zot(lot), zot::WATER);
}

#[test]
fn condition_matches_spec() {
    // §16 test 9 (a subset): state 3, building wealth 3, occupant wealth 1,
    // condition 0, days 10 -> 3; state 4 condition 0 days 0 -> 1.
    let host = Rc::new(Script::default());
    host.profiles.borrow_mut().insert(
        0x3000,
        Profile {
            purpose: 1,
            wealth: 3,
            capacity: 0,
        },
    );
    let lot = host.add_lot(|l| {
        l.state = lot_state::OCCUPIED;
        l.building_type = 0x3000;
        l.wealth = 1;
        l.condition = 0;
        l.days = 10;
        l.occupancy = 1.0;
    });
    let mut d = dev_with(host.clone());
    assert_eq!(d.test_condition(lot, 0, 0), 3);
    // state 4, condition 0, days 0 -> 1.
    host.lot_mut(
        lot,
        |l| {
            l.state = lot_state::ABANDONED;
            l.condition = 0;
            l.days = 0;
        },
        (),
    );
    assert_eq!(d.test_condition(lot, 0, 0), 1);
    // state 4, condition 2, days 91 -> 3.
    host.lot_mut(
        lot,
        |l| {
            l.condition = 2;
            l.days = 91;
        },
        (),
    );
    assert_eq!(d.test_condition(lot, 0, 0), 3);
    // state 4, condition 4, days 91 -> 4 (c > 3 keeps it).
    host.lot_mut(lot, |l| l.condition = 4, ());
    assert_eq!(d.test_condition(lot, 0, 0), 4);
}

#[test]
fn abandonment_order_last_failing_wins() {
    // §16 test 18: a lot failing desirability and water records reason 3
    // (the last failing test wins); SetState is called once. Both tests have
    // a grace period, so the counters are pre-loaded past it.
    let host = Rc::new(Script::default());
    let dev_key = (1u32 << 8) | 1;
    let dev_token = host.token();
    host.devs.borrow_mut().insert(
        dev_token,
        Dev {
            key: dev_key,
            purpose: 1,
            desirability: -100,
            water_cap: 0,
        },
    );
    host.profiles.borrow_mut().insert(
        0x3000,
        Profile {
            purpose: 1,
            wealth: 1,
            capacity: 0,
        },
    );
    let building = host.add_occ(|o| {
        o.is_building = true;
        o.bbox = ([0.0, 0.0, 0.0], [16.0, 10.0, 16.0]);
    });
    let lot = host.add_lot(move |l| {
        l.state = lot_state::OCCUPIED;
        l.building_type = 0x3000;
        l.building = Some(building);
        l.wealth = 1;
        l.livable = true;
        l.needs_power = true;
        l.road = true;
        l.job = true;
        // Past both grace periods: desirability (tolerance <= 8) and water
        // (tolerance 2).
        l.props.insert(lot_prop::LOW_DESIRABILITY, 100);
        l.props.insert(lot_prop::NO_WATER, 100);
    });
    // Powered and road+trip OK, so only desirability and water fail.
    host.powered_cells.borrow_mut().insert((0, 0), true);
    host.trip_cells.borrow_mut().insert((0, 0), 1);
    let cfg = host.token();
    host.cfgs.borrow_mut().insert(
        cfg,
        Cfg {
            id: cfg as u32,
            w: 1,
            h: 1,
            stage: 5, // above the developer's water cap 0 -> needs water
            ..Cfg::default()
        },
    );
    host.lot_mut(lot, |l| l.cfg = Some(cfg), ());
    let mut d = dev_with(host.clone());
    host.clear_events();
    d.update_lot(lot);
    let ev = host.events();
    let set_states: Vec<&String> = ev.iter().filter(|e| e.starts_with("set_state")).collect();
    assert_eq!(set_states.len(), 1, "{ev:?}");
    assert!(set_states[0].contains("reason=3"), "{ev:?}");
}

// ---- UpdateLotElements kind (§6.4, §16 test 27) -------------------------------------------------

#[test]
fn update_lot_elements_kind_is_full_32_bit() {
    // A lot in state 3 without a foundation, configuration whose max slope is
    // below the lot slope. kind 5 -> no AddLotFoundation; kind 0x105 and
    // kind 0x100 -> one AddLotFoundation each. The comparison is a full
    // 32-bit `kind != 5`, never `kind & 0xFF`.
    let make = |kind: u32| -> (Vec<String>, bool) {
        let host = Rc::new(Script::default());
        let cfg = host.token();
        host.cfgs.borrow_mut().insert(
            cfg,
            Cfg {
                id: cfg as u32,
                w: 1,
                h: 1,
                max_slope: 0.0, // below the lot slope
                primary_foundation: Some(vec![DEFAULT_FOUNDATION_TYPE]),
                ..Cfg::default()
            },
        );
        // A sloped lot: vertex (1, 0) at 104, the rest 100 -> slope 4.0.
        host.terrain_alts.borrow_mut().insert(1, 104.0);
        let lot = host.add_lot(move |l| {
            l.state = lot_state::OCCUPIED;
            l.flags = 0; // no foundation
            l.cfg = Some(cfg);
        });
        let mut d = dev_with(host.clone());
        host.clear_events();
        d.update_lot_elements(lot, kind);
        let ev = host.events();
        let had = ev
            .iter()
            .find(|e| e.starts_with("update_base_texture_occupant"))
            .map(|e| e.contains("had=true"))
            .unwrap_or(false);
        (ev, had)
    };
    // kind 5: no foundation, texture call gets false.
    let (ev, had) = make(5);
    assert!(!ev.iter().any(|e| e.starts_with("add_lot_foundation")), "{ev:?}");
    assert!(!had);
    // kind 0x105: foundation added, texture call gets true.
    let (ev, had) = make(0x105);
    assert_eq!(
        ev.iter().filter(|e| e.starts_with("add_lot_foundation")).count(),
        1,
        "{ev:?}"
    );
    assert!(had);
    // kind 0x100 (low byte 0): still added, proving the full-32-bit compare.
    let (ev, had) = make(0x100);
    assert_eq!(
        ev.iter().filter(|e| e.starts_with("add_lot_foundation")).count(),
        1,
        "{ev:?}"
    );
    assert!(had);
}

// ---- construction (§6) --------------------------------------------------------------------------

#[test]
fn start_construction_sets_states() {
    let host = Rc::new(Script::default());
    let cfg = host.token();
    host.cfgs.borrow_mut().insert(
        cfg,
        Cfg {
            id: cfg as u32,
            w: 1,
            h: 1,
            max_slope: 1e9,
            ..Cfg::default()
        },
    );
    host.profiles.borrow_mut().insert(
        0x3000,
        Profile {
            purpose: 1,
            wealth: 1,
            capacity: 0,
        },
    );
    let lot = host.add_lot(|_| {});
    let mut d = dev_with(host.clone());
    assert!(d.start_construction(lot, Some(cfg), 0x3000, false));
    // Without animation the state ends at 3 (occupied); condition 1 during.
    assert_eq!(host.lot_state(lot), lot_state::OCCUPIED);
    let ev = host.events();
    assert!(ev.iter().any(|e| e.contains("add_building 0x3000")), "{ev:?}");
}

#[test]
fn start_construction_animate_keeps_state_two() {
    let host = Rc::new(Script::default());
    let cfg = host.token();
    host.cfgs.borrow_mut().insert(
        cfg,
        Cfg {
            id: cfg as u32,
            w: 1,
            h: 1,
            max_slope: 1e9,
            ..Cfg::default()
        },
    );
    host.profiles.borrow_mut().insert(
        0x3000,
        Profile {
            purpose: 1,
            wealth: 1,
            capacity: 0,
        },
    );
    let lot = host.add_lot(|_| {});
    let mut d = dev_with(host.clone());
    assert!(d.start_construction(lot, Some(cfg), 0x3000, true));
    // With animation the lot stays in state 2 and the construction occupant
    // is handed off.
    assert_eq!(host.lot_state(lot), lot_state::CONSTRUCTING);
    assert!(host.events().iter().any(|e| e.starts_with("construction_occupant")));
}

#[test]
fn start_construction_type_zero_succeeds() {
    let host = Rc::new(Script::default());
    let lot = host.add_lot(|_| {});
    let mut d = dev_with(host.clone());
    assert!(d.start_construction(lot, None, 0, false));
    assert_eq!(host.lot_state(lot), lot_state::OCCUPIED);
    // No building occupant was added.
    assert!(!host.events().iter().any(|e| e.contains("add_building")));
}

#[test]
fn abandon_and_reoccupy() {
    let host = Rc::new(Script::default());
    host.profiles.borrow_mut().insert(
        0x3000,
        Profile {
            purpose: 1,
            wealth: 2,
            capacity: 5,
        },
    );
    let lot = host.add_lot(|l| {
        l.building_type = 0x3000;
        l.state = lot_state::OCCUPIED;
    });
    let mut d = dev_with(host.clone());
    assert!(d.abandon_building(lot, reason::NO_WATER));
    assert_eq!(host.lot_state(lot), lot_state::ABANDONED);
    // Reoccupy with devType whose wealth is 0: profile wealth 2 - 0 = 2 ->
    // condition 3.
    let dev_type = 0u32;
    assert!(d.reoccupy_building(lot, dev_type));
    assert_eq!(host.lot_state(lot), lot_state::OCCUPIED);
    assert_eq!(host.lot_condition(lot), 3);
    assert!(host.events().iter().any(|e| e.starts_with("set_population")));
}

// ---- night lights (§8) --------------------------------------------------------------------------

#[test]
fn night_batch_threshold() {
    // §16 test 15: shipped tuning (accumulator 100, batch 10). A tick of
    // 150000 µs -> 150 ms, acc 250 > 200 -> a batch runs.
    let host = Rc::new(Script::default());
    let mut d = dev_with(host.clone());
    d.read_tuning();
    assert_eq!(d.night_acc(), 100);
    assert_eq!(d.night_batch(), 10);
    d.set_night_mode(2);
    // No buildings: the batch immediately sets mode 0 (cursor at end).
    d.on_animation_tick(150);
    assert_eq!(d.night_acc(), 0);
    assert_eq!(d.night_mode(), 0);
    // Below the threshold: nothing runs and the accumulator keeps the ms.
    d.set_night_mode(2);
    d.on_animation_tick(99);
    assert_eq!(d.night_mode(), 2);
    assert_eq!(d.night_acc(), 99);
}

#[test]
fn animation_tick_message_false_when_idle() {
    // §16 test 15: with mode 0 the tick message returns false.
    let host = Rc::new(Script::default());
    let mut d = dev_with(host);
    d.set_night_mode(0);
    let m = LotDevMessage {
        msg_type: msg::ANIMATION_TICK,
        data2: 150_000,
        ..LotDevMessage::default()
    };
    assert!(!d.do_message(&m));
    d.set_night_mode(2);
    assert!(d.do_message(&m));
}

#[test]
fn night_deque_insert_remove() {
    let host = Rc::new(Script::default());
    let mut d = dev_with(host.clone());
    let b0 = host.add_occ(|o| o.is_building = true);
    let b1 = host.add_occ(|o| o.is_building = true);
    d.do_message(&LotDevMessage {
        msg_type: msg::OCCUPANT_INSERTED,
        data3: OCCUPANT_TYPE_BUILDING,
        void1: Some(b0),
        ..LotDevMessage::default()
    });
    d.do_message(&LotDevMessage {
        msg_type: msg::OCCUPANT_INSERTED,
        data3: OCCUPANT_TYPE_BUILDING,
        void1: Some(b1),
        ..LotDevMessage::default()
    });
    assert_eq!(d.night_len(), 2);
    // Removing the first shifts the deque.
    d.do_message(&LotDevMessage {
        msg_type: msg::OCCUPANT_REMOVED,
        data3: OCCUPANT_TYPE_BUILDING,
        void1: Some(b0),
        ..LotDevMessage::default()
    });
    assert_eq!(d.night_len(), 1);
    assert_eq!(d.night()[0], Some(b1));
}

#[test]
fn day_night_shuffle_uses_crt_rand() {
    // §16 test 16: the day/night message without an immediate update
    // shuffles the deque with C rand() (n-1 calls).
    let host = Rc::new(Script::default());
    let mut d = dev_with(host.clone());
    for _ in 0..4 {
        let b = host.add_occ(|o| o.is_building = true);
        d.do_message(&LotDevMessage {
            msg_type: msg::OCCUPANT_INSERTED,
            data3: OCCUPANT_TYPE_BUILDING,
            void1: Some(b),
            ..LotDevMessage::default()
        });
    }
    // time_mode 0 means neither immediate branch matches.
    host.time_mode.set(0);
    host.clear_events();
    d.do_message(&LotDevMessage {
        msg_type: msg::DAY_NIGHT_CHANGED,
        data2: 0, // night
        ..LotDevMessage::default()
    });
    assert_eq!(d.night_mode(), 2);
    // 3 rand() draws recorded as R15.
    let st = d.statics();
    st.borrow_mut().trace = Some(Vec::new());
    d.do_message(&LotDevMessage {
        msg_type: msg::DAY_NIGHT_CHANGED,
        data2: 1,
        ..LotDevMessage::default()
    });
    let draws = st.borrow().trace.as_ref().unwrap().clone();
    assert_eq!(draws.iter().filter(|x| x.kind == DrawKind::R15).count(), 3);
}

#[test]
fn immediate_day_night_update() {
    let host = Rc::new(Script::default());
    let mut d = dev_with(host.clone());
    // night (data2 = 0) and mode 2 -> immediate update, then post 0xA6B79621.
    host.time_mode.set(2);
    d.do_message(&LotDevMessage {
        msg_type: msg::DAY_NIGHT_CHANGED,
        data2: 0,
        ..LotDevMessage::default()
    });
    assert_eq!(d.night_mode(), 0);
    assert!(host.events().iter().any(|e| e.starts_with("post 0xa6b79621")));
}

#[test]
fn light_queries_the_lots_rectangle() {
    // §8.4 (answers Q8): the building's cells only find the lot; the props
    // are looked up in the lot's own rectangle.
    let host = Rc::new(Script::default());
    let lot = host.add_lot(|l| {
        l.x = 2;
        l.z = 3;
        l.w = 3;
        l.h = 2;
    });
    let b = host.add_occ(|o| {
        o.is_building = true;
        o.cells = CellRect::new(2, 3, 3, 3);
    });
    host.lot_mut(lot, |l| l.building = Some(b), ());
    let mut d = dev_with(host.clone());
    d.light(b, true);
    assert_eq!(*host.cell_queries.borrow(), vec![CellRect::new(2, 3, 4, 4)]);
    // No lot at the building's centre cell: no prop query.
    host.cell_queries.borrow_mut().clear();
    let stray = host.add_occ(|o| {
        o.is_building = true;
        o.cells = CellRect::new(40, 40, 40, 40);
    });
    d.light(stray, false);
    assert!(host.cell_queries.borrow().is_empty());
}

// ---- business units (§5.7) ----------------------------------------------------------------------

#[test]
fn update_travel_designates_follows_the_table() {
    // §5.7 (answers Q9).
    let host = Rc::new(Script::default());
    let b = host.add_occ(|o| o.is_building = true);
    let anchor = host.add_lot(|l| {
        l.building = Some(b);
        l.road = true;
    });
    let null_d = host.add_lot(|l| l.road = false);
    let member_road = host.add_lot(|l| {
        l.road = false;
        l.designate = Some(anchor);
    });
    let road_less = host.add_lot(|l| l.road = false);
    let member_no_road = host.add_lot(|l| {
        l.road = false;
        l.designate = Some(road_less);
    });
    let outsider = host.add_lot(|l| l.road = true);
    let non_member = host.add_lot(|l| {
        l.road = false;
        l.designate = Some(outsider);
    });
    let unit = 900;
    for l in [anchor, null_d, member_road, road_less, member_no_road, non_member] {
        host.unit_add(unit, l);
    }
    let mut d = dev_with(host.clone());
    let mut u = Some(unit);
    d.update_travel_designates(&mut u);
    assert_eq!(u, Some(unit));
    let des = |l| host.lot(l, |x| x.designate, None);
    assert_eq!(des(anchor), None, "the anchor gets no designate");
    assert_eq!(des(null_d), Some(anchor));
    assert_eq!(des(member_road), Some(anchor), "kept (it already was the anchor)");
    assert_eq!(des(road_less), Some(anchor));
    assert_eq!(
        des(member_no_road),
        Some(anchor),
        "a member without road access is replaced"
    );
    assert_eq!(
        des(non_member),
        Some(anchor),
        "a non-member is cleared even with road access"
    );

    // The keep row leaves a member-with-road designate that is not the anchor.
    let host = Rc::new(Script::default());
    let b = host.add_occ(|o| o.is_building = true);
    let first = host.add_lot(|l| {
        l.building = Some(b);
        l.road = true;
    });
    let second = host.add_lot(|l| {
        l.building = Some(b);
        l.road = true;
        l.designate = Some(first);
    });
    let stale = host.add_lot(|l| {
        l.road = false;
        l.designate = Some(second);
    });
    host.unit_add(901, first);
    host.unit_add(901, second);
    host.unit_add(901, stale);
    // Make `second` the anchor's predecessor: move `first` to the back.
    host.units.borrow_mut().insert(901, vec![second, stale, first]);
    let mut d = dev_with(host.clone());
    let mut u = Some(901);
    d.update_travel_designates(&mut u);
    assert_eq!(
        host.lot(second, |x| x.designate, None),
        Some(first),
        "the anchor keeps a stale designate"
    );
    assert_eq!(
        host.lot(stale, |x| x.designate, None),
        Some(second),
        "kept: a member with road access"
    );
    assert_eq!(host.lot(first, |x| x.designate, None), Some(second));
}

#[test]
fn dissolution_moves_every_member_including_the_joiner() {
    // §5.7 (answers Q10): M = [a, b]; only b finds a candidate c, so
    // u' = [c, b], then a is added and b removed and re-added: [c, a, b].
    let host = Rc::new(Script::default());
    host.profiles.borrow_mut().insert(
        0x6000,
        Profile {
            purpose: 6,
            wealth: 1,
            capacity: 0,
        },
    );
    let mk = |road: bool| {
        host.add_lot(|l| {
            l.state = lot_state::OCCUPIED;
            l.building_type = 0x6000;
            l.road = road;
        })
    };
    let a = mk(false);
    let b = mk(false);
    let c = mk(true);
    host.unit_add(902, a);
    host.unit_add(902, b);
    host.surrounding.borrow_mut().insert(b, vec![c]);
    let mut d = dev_with(host.clone());
    let mut u = Some(902);
    d.update_travel_designates(&mut u);
    let joined = u.expect("b joined c's new unit");
    assert_ne!(joined, 902);
    assert_eq!(host.unit_members(joined), vec![c, a, b]);
    assert!(host.unit_members(902).is_empty());
    assert_eq!(host.lot(a, |x| x.unit, None), Some(joined));
    assert_eq!(host.lot(b, |x| x.unit, None), Some(joined));
}

// ---- power changed (§9) -------------------------------------------------------------------------

#[test]
fn power_changed_sets_zot_and_condition() {
    let host = Rc::new(Script::default());
    let building = host.add_occ(|o| {
        o.is_building = true;
        o.bbox = ([0.0, 0.0, 0.0], [16.0, 10.0, 16.0]);
    });
    let lot = host.add_lot(move |l| {
        l.building = Some(building);
        l.building_type = 0x3000;
        l.condition = 0;
        l.needs_power = true;
        l.has_power = true; // starts powered, so losing power changes it
    });
    host.lots_in_rect.borrow_mut().push(lot);
    let mut d = dev_with(host.clone());
    // Unpowered: sets has_power false, condition 1, adds a power zot.
    d.do_power_changed(CellRect::new(0, 0, 0, 0));
    assert_eq!(host.lot_condition(lot), 1);
    assert_eq!(d.current_zot(lot), zot::POWER);
    // Powered now: condition back to 0, zot removed.
    host.powered_cells.borrow_mut().insert((0, 0), true);
    host.clear_events();
    d.do_power_changed(CellRect::new(0, 0, 0, 0));
    assert_eq!(host.lot_condition(lot), 0);
    assert_eq!(d.current_zot(lot), zot::NONE);
}

// ---- city established / traffic (§3.2) ----------------------------------------------------------

#[test]
fn city_established_and_traffic() {
    let host = Rc::new(Script::default());
    let mut d = dev_with(host.clone());
    host.sim_date.set(100);
    d.do_message(&LotDevMessage::of_type(msg::CITY_ESTABLISHED));
    assert_eq!(d.no_zot_until_day(), 131);
    // Traffic map changed with a positive jobs demand sets canCheckCommute.
    host.jobs_demand.set(Some(1.5));
    d.do_message(&LotDevMessage::of_type(msg::TRAFFIC_MAP_CHANGED));
    assert!(d.can_check_commute());
    host.jobs_demand.set(Some(0.0));
    d.do_message(&LotDevMessage::of_type(msg::TRAFFIC_MAP_CHANGED));
    assert!(!d.can_check_commute());
    // NaN -> false.
    host.jobs_demand.set(Some(f32::NAN));
    d.do_message(&LotDevMessage::of_type(msg::TRAFFIC_MAP_CHANGED));
    assert!(!d.can_check_commute());
    // No demand node -> false.
    host.jobs_demand.set(None);
    d.do_message(&LotDevMessage::of_type(msg::TRAFFIC_MAP_CHANGED));
    assert!(!d.can_check_commute());
}

#[test]
fn unknown_and_terrain_messages_return_true() {
    let host = Rc::new(Script::default());
    let mut d = dev_with(host);
    assert!(d.do_message(&LotDevMessage::of_type(0x1234_5678)));
    // The terrain-changed message is subscribed but ignored (quirk 15).
    assert!(d.do_message(&LotDevMessage::of_type(msg::TERRAIN_CHANGED)));
    assert!(d.do_message(&LotDevMessage::of_type(msg::ONE_SHOT)));
    assert!(d.do_message(&LotDevMessage::of_type(msg::SIM_NEW_MONTH)));
}

// ---- persistence (§13) --------------------------------------------------------------------------

#[test]
fn save_record_matches_spec() {
    // §16 test 17: 64x64 city (cell width 16), empty deque, mode 0, unlit:
    // 24 bytes.
    let host = Rc::new(Script::default());
    let mut d = dev_with(host);
    assert!(d.init());
    let mut w = ByteWriter::new();
    assert!(d.write(&mut w));
    let bytes = w.into_bytes();
    let want = [
        0x01u8, 0x00, // version 1
        0x41, 0x00, 0x00, 0x00, // vertsX 65
        0x00, 0x00, 0x80, 0x44, // extentX 1024.0
        0x00, 0x00, 0x80, 0x44, // extentZ 1024.0
        0x00, 0x00, 0x00, 0x00, // n 0
        0x00, 0x00, 0x00, 0x00, // cursor 0
        0x00, // nightMode
        0x00, // litFlag
    ];
    assert_eq!(bytes, want, "{:02x?}", bytes);
}

#[test]
fn save_read_rejects_bad_version() {
    let host = Rc::new(Script::default());
    let mut d = dev_with(host);
    // A version-2 record is rejected after 2 bytes.
    let mut rec = vec![2u8, 0];
    rec.extend_from_slice(&65i32.to_le_bytes());
    assert!(!d.read(&mut ByteReader::new(&rec)));
    assert_eq!(d.verts_x(), 0);
}

#[test]
fn save_round_trip_with_buildings() {
    use crate::stream::{RefResolver, SaveRegistry};
    let host = Rc::new(Script::default());
    let b0 = host.add_occ(|o| o.is_building = true);
    let b1 = host.add_occ(|o| o.is_building = true);
    let mut d = dev_with(host.clone());
    assert!(d.init());
    d.do_message(&LotDevMessage {
        msg_type: msg::OCCUPANT_INSERTED,
        data3: OCCUPANT_TYPE_BUILDING,
        void1: Some(b0),
        ..LotDevMessage::default()
    });
    d.do_message(&LotDevMessage {
        msg_type: msg::OCCUPANT_INSERTED,
        data3: OCCUPANT_TYPE_BUILDING,
        void1: Some(b1),
        ..LotDevMessage::default()
    });
    d.set_night_mode(2);
    d.set_night_cursor(1);
    d.set_lit_flag(true);
    let registry = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(registry);
    assert!(d.write(&mut w));
    let bytes = w.into_bytes();
    // Read it back with a resolver that maps the host references.
    struct Resolver;
    impl RefResolver for Resolver {
        fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
            if class_id == IID_BUILDING_OCCUPANT {
                Some(RefValue::Host {
                    token: u64::from(object_id),
                    class_id,
                })
            } else {
                None
            }
        }
    }
    let mut resolver = Resolver;
    let mut d2 = dev_with(host);
    assert!(d2.read(&mut ByteReader::with_resolver(&bytes, &mut resolver)));
    assert_eq!(d2.night_len(), 2);
    assert_eq!(d2.night_mode(), 2);
    assert_eq!(d2.night_cursor(), 1);
    assert!(d2.lit_flag());
    // The accumulator is reset to 0 on read.
    assert_eq!(d2.night_acc(), 0);
}

// ---- hostile input ------------------------------------------------------------------------------

#[test]
fn hostile_records_and_hosts_do_not_panic() {
    let mut g = crate::lot_developer::tests::Lcg(7);
    for _ in 0..200 {
        let mut d = dev_with(Rc::new(Script::default()));
        let len = (g.next() % 64) as usize;
        let junk: Vec<u8> = (0..len).map(|_| g.next() as u8).collect();
        let _ = d.read(&mut ByteReader::new(&junk));
        let _ = d.do_daily_simulation();
        let _ = d.do_message(&LotDevMessage {
            msg_type: g.next(),
            data1: g.next(),
            data2: g.next(),
            data3: g.next(),
            data4: g.next(),
            ..LotDevMessage::default()
        });
        let _ = d.update_lot_elements(u64::from(g.next() % 8), g.next());
        let _ = d.start_construction(1, Some(2), g.next(), g.next() & 1 == 0);
        let _ = d.explode_lot(1, g.next() & 1 == 0);
        // Bounded boxes so the rubble loop and the vertex walk stay finite.
        let max = [g.next() % 40, g.next() % 40, g.next() % 40];
        d.add_lot_rubble_props(1, [0.0; 3], [max[0] as f32, 0.0, max[2] as f32], g.next() & 1 == 0);
        let r = CellRect::new(
            (g.next() % 16) as i32,
            (g.next() % 16) as i32,
            (g.next() % 16) as i32,
            (g.next() % 16) as i32,
        );
        let _ = d.get_lot_development_cost(r);
        let _ = d.get_area_slope(r, true);
        let _ = d.lowest_or_highest_area_altitude(r, true, true);
    }
}

#[test]
fn terrain_view_matches_the_core() {
    // The snapshot answers the terrain and cost slots exactly as the core.
    let host = Rc::new(Script::default());
    for (i, a) in [0.0f32, 3.5, 7.25, 1.0, 12.0, 2.0, 9.5, 4.0, 30.0].iter().enumerate() {
        let (x, z) = (i as i32 % 3, i as i32 / 3);
        host.terrain_alts.borrow_mut().insert(x + z * 65, *a);
        host.terrain_water.borrow_mut().insert(x + z * 65, 5.0);
    }
    let lot = host.add_lot(|l| {
        l.w = 2;
        l.h = 2;
    });
    let mut d = dev_with(host.clone());
    assert!(d.init());
    let v = d.terrain_view();
    assert_eq!(v.verts_x, 65);
    let rect = CellRect::new(0, 0, 1, 1);
    let h: &dyn LotDeveloperHost = &*host;
    for w in [false, true] {
        assert_eq!(
            v.altitude_by_vertex(h, 1, 1, w),
            d.get_terrain_altitude_by_vertex(1, 1, w)
        );
        assert_eq!(
            v.lowest_or_highest_area_altitude(h, rect, true, w),
            d.lowest_or_highest_area_altitude(rect, true, w)
        );
        assert_eq!(v.average_area_altitude(h, rect, w), d.average_area_altitude(rect, w));
        assert_eq!(v.area_altitude_range(h, rect, w), d.area_altitude_range(rect, w));
        assert_eq!(v.is_area_flat(h, rect, w), d.is_area_flat(rect, w));
        assert_eq!(v.lot_slope(h, lot, w).to_bits(), d.get_lot_slope(lot, w).to_bits());
    }
    assert_eq!(v.lot_development_cost(h, rect), d.get_lot_development_cost(rect));
    assert!(d.get_lot_development_cost(rect) > 0);
}

#[test]
fn is_area_flat_compares_with_the_first_vertex_with_water() {
    // §16 test 42 (LD29): every vertex against the first vertex (x1, z1),
    // read with water whatever `withWater` says; strict, so the range of a
    // flat rectangle can reach twice the tuning value.
    let host = Rc::new(Script::default());
    let set = |x: i32, z: i32, a: f32| {
        host.terrain_alts.borrow_mut().insert(x + z * 65, a);
    };
    set(0, 0, 100.5);
    set(1, 0, 100.0);
    set(0, 1, 101.4);
    set(1, 1, 100.9);
    let mut d = dev_with(host.clone());
    assert!(d.init());
    assert_eq!(d.terrain_view().tuning.max_cell_altitude_delta_for_flatness, 1.0);
    let rect = CellRect::new(0, 0, 0, 0);
    for w in [false, true] {
        // Range 1.4, but every difference from 100.5 is at most 0.9.
        assert!(d.is_area_flat(rect, w));
    }
    // Slot 27 on the same rectangle: 101.4 − 100 (one float32 rounding).
    assert_eq!(d.area_altitude_range(rect, false), 101.4f32 - 100.0);
    set(1, 0, 99.4);
    assert!(!d.is_area_flat(rect, false), "|99.4 - 100.5| = 1.1 > 1.0");
    set(1, 0, 99.5);
    assert!(d.is_area_flat(rect, false), "|99.5 - 100.5| = 1.0 is not above");
    set(1, 0, 100.0);
    // Water 101 at vertex (0, 0) only: the reference is 101 for either
    // `withWater`; the other vertices differ by 1.0, 0.4, 0.1. Dry, the
    // vertex (0, 0) itself reads 100.5 (difference 0.5).
    host.terrain_water.borrow_mut().insert(0, 101.0);
    assert!(d.is_area_flat(rect, false));
    assert!(d.is_area_flat(rect, true));
    // Water 102 there: (1, 0) = 100 differs by 2.0 for both.
    host.terrain_water.borrow_mut().insert(0, 102.0);
    assert!(!d.is_area_flat(rect, false));
    assert!(!d.is_area_flat(rect, true));
    // The view answers as the core.
    let v = d.terrain_view();
    let h: &dyn LotDeveloperHost = &*host;
    assert_eq!(v.is_area_flat(h, rect, true), d.is_area_flat(rect, true));
}

#[test]
fn is_area_flat_walks_x_outer_and_reads_the_reference_first() {
    // LD29: the reference read comes first (with water), then the walk;
    // a NaN difference is never above the tuning.
    let host = Rc::new(Script::default());
    host.terrain_alts.borrow_mut().insert(1, f32::NAN);
    let mut d = dev_with(host.clone());
    assert!(d.init());
    // Vertex (1, 0) is NaN: |NaN − 100| > 1.0 is false, so still flat.
    assert!(d.is_area_flat(CellRect::new(0, 0, 0, 0), false));
    // A NaN reference makes every difference NaN: flat.
    host.terrain_alts.borrow_mut().insert(0, f32::NAN);
    host.terrain_alts.borrow_mut().insert(1, 500.0);
    assert!(d.is_area_flat(CellRect::new(0, 0, 0, 0), false));
}

#[test]
fn network_connection_nodes_follow_the_type_7_objects() {
    // §6.5 AddNetworkConnectionNodes (LD17): type-7 objects with at least 3
    // IDs, in array order; the cell of slot 39's position; ids[1] and ids[2]
    // rotated left by 8·orientation, ids[3] or 0.
    let host = Rc::new(Script::default());
    let obj = |t: u32, ids: Vec<u32>, orientation: u8, off: Vec3| CfgObject {
        object_type: t,
        ids,
        orientation,
        offset: off,
        ..CfgObject::default()
    };
    let cfg = host.token();
    host.cfgs.borrow_mut().insert(
        cfg,
        Cfg {
            id: 0x6000_0001,
            w: 1,
            h: 1,
            objects: vec![
                obj(7, vec![0x0A, 0x1122_3344, 0x5566_7788, 0x99], 1, [8.0, 0.0, 4.0]),
                obj(7, vec![0x0B, 0x1], 0, [8.0, 0.0, 8.0]),
                obj(1, vec![0x0C, 0x1, 0x2], 0, [8.0, 0.0, 8.0]),
                obj(7, vec![0x0D, 0xAABB_CCDD, 0x0102_0304], 0, [15.0, 0.0, 1.0]),
                obj(7, vec![0x0E, 0x8000_0001, 0x1, 0x2, 0x3], 3, [1.0, 0.0, 15.0]),
            ],
            ..Cfg::default()
        },
    );
    let lot = host.add_lot(|l| {
        l.x = 2;
        l.z = 3;
        l.cfg = Some(cfg);
    });
    let mut d = dev_with(host.clone());
    assert!(d.init());
    // Facing 0: pos = c1 − offset with c1 = (48, 64).
    let nodes = d.connection_nodes(lot, cfg);
    assert_eq!(
        nodes,
        vec![
            ConnectionNode {
                node_type: 0x0A,
                x: 2,
                z: 3,
                b: 0x2233_4411,
                c: 0x6677_8855,
                d: 0x99,
                orientation: 1
            },
            ConnectionNode {
                node_type: 0x0D,
                x: 2,
                z: 3,
                b: 0xAABB_CCDD,
                c: 0x0102_0304,
                d: 0,
                orientation: 0
            },
            ConnectionNode {
                node_type: 0x0E,
                x: 2,
                z: 3,
                b: 0x0180_0000,
                c: 0x0100_0000,
                d: 0x2,
                orientation: 3
            },
        ]
    );
    // The host gets them all in one call, also with none.
    host.clear_events();
    d.add_network_connection_nodes(lot, cfg);
    let ev = host.events();
    assert!(ev.contains(&"add_network_nodes 2 n=3".to_string()) || ev.iter().any(|e| e.ends_with(" n=3")));
    let empty = host.token();
    host.cfgs.borrow_mut().insert(empty, Cfg::default());
    host.clear_events();
    d.add_network_connection_nodes(lot, empty);
    assert!(host
        .events()
        .iter()
        .any(|e| e.starts_with("add_network_nodes") && e.ends_with(" n=0")));
}

#[test]
fn update_lot_object_altitudes_moves_only_y_beyond_the_tolerance() {
    // §6.5 UpdateLotObjectAlitudes (LD17): buildings, props and flora of the
    // lot (lot slot 49, list order); the target is slot 39's y for the
    // occupant's configuration object, else the terrain's at its x, z;
    // |P.y − Q.y| > 0.1 (strict) moves y only and notifies (slot 24, 1).
    let host = Rc::new(Script::default());
    host.pos_alt.set(100.0);
    let cfg = host.token();
    host.cfgs.borrow_mut().insert(
        cfg,
        Cfg {
            w: 1,
            h: 1,
            ..Cfg::default()
        },
    );
    let lot = host.add_lot(|l| l.cfg = Some(cfg));
    let building = host.add_occ(|o| {
        o.is_building = true;
        o.position = [3.0, 105.0, 4.0];
    });
    let prop_near = host.add_occ(|o| o.position = [5.0, 100.05, 6.0]);
    let flora_far = host.add_occ(|o| o.position = [7.0, 99.8, 8.0]);
    let prop_cfg = host.add_occ(|o| o.position = [1.0, 100.0, 2.0]);
    let network = host.add_occ(|o| o.position = [9.0, 50.0, 9.0]);
    let other = host.add_occ(|o| o.position = [9.0, 50.0, 9.0]);
    host.occ_types.borrow_mut().insert(prop_near, OCCUPANT_TYPE_PROP);
    host.occ_types.borrow_mut().insert(flora_far, OCCUPANT_TYPE_FLORA);
    host.occ_types.borrow_mut().insert(prop_cfg, OCCUPANT_TYPE_PROP);
    host.occ_types.borrow_mut().insert(network, OCCUPANT_TYPE_NETWORK);
    host.occ_types.borrow_mut().insert(other, 0x1234_5678);
    // A prop object 2.0 above the ground at the lot's centre (type 1: the
    // terrain at its x, z): target 102.
    host.occ_objects.borrow_mut().insert(
        prop_cfg,
        CfgObject {
            object_type: 1,
            offset: [8.0, 2.0, 8.0],
            ..CfgObject::default()
        },
    );
    host.lot_occs
        .borrow_mut()
        .insert(lot, vec![building, prop_near, flora_far, prop_cfg, network, other]);
    let mut d = dev_with(host.clone());
    assert!(d.init());
    host.clear_events();
    d.update_lot_object_altitudes(lot);
    let moved: Vec<String> = host
        .events()
        .into_iter()
        .filter(|e| e.starts_with("occupant_moved"))
        .collect();
    assert_eq!(
        moved,
        vec![
            format!("occupant_moved {building} y=100"),
            format!("occupant_moved {flora_far} y=100"),
            format!("occupant_moved {prop_cfg} y=102"),
        ]
    );
    // Only y moved.
    assert_eq!(host.occupant_position(building), [3.0, 100.0, 4.0]);
    assert_eq!(host.occupant_position(prop_near), [5.0, 100.05, 6.0]);
    assert_eq!(host.occupant_position(network), [9.0, 50.0, 9.0]);
    assert_eq!(host.occupant_position(other), [9.0, 50.0, 9.0]);
    // The terrain was read at the occupants' own x, z (no object).
    let ev = host.events();
    assert!(ev.contains(&"altitude_at 3 4".to_string()));
    assert!(ev.contains(&"altitude_at 5 6".to_string()));
    assert!(!ev.contains(&"altitude_at 9 9".to_string()));
    // Exactly 0.1 away (in f32) is not moved.
    host.occupant_set_position(flora_far, [7.0, 100.1, 8.0]);
    host.clear_events();
    d.update_lot_object_altitudes(lot);
    assert!(!host.events().iter().any(|e| e.starts_with("occupant_moved")));
}

/// A small deterministic generator for test data.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }
}
