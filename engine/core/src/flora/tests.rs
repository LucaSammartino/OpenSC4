// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests for the flora simulator. The behaviour of §16 is covered by
//! the conformance vectors `fo16_*`; these tests cover the pure helpers,
//! the registry, the generator and the bounds on hostile input.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use super::*;
use crate::rng::FixedSeeds;
use crate::stream::{ByteReader, ByteWriter, SaveObject};
use crate::variant::{mem, Elem, Variant};

// ---- a small host ---------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct Occ {
    pos: Vec3,
    flora: FloraOccupant,
    in_manager: bool,
}

struct Host {
    occs: RefCell<BTreeMap<u64, Occ>>,
    next: Cell<u64>,
    removed: RefCell<Vec<u64>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    exemplars: RefCell<Vec<(ResKey, Rc<PropertyHolder>)>>,
    ground: Cell<f32>,
}

impl Host {
    fn new() -> Rc<Host> {
        Rc::new(Host {
            occs: RefCell::new(BTreeMap::new()),
            next: Cell::new(100),
            removed: RefCell::new(Vec::new()),
            refs: RefCell::new(BTreeMap::new()),
            tuning: RefCell::new(None),
            exemplars: RefCell::new(Vec::new()),
            ground: Cell::new(260.0),
        })
    }

    fn add_flora(&self, id: u64, x: f32, z: f32, t: u32, state: i8) {
        self.occs.borrow_mut().insert(
            id,
            Occ {
                pos: [x, 260.0, z],
                flora: FloraOccupant {
                    flora_type: t,
                    state,
                    ..FloraOccupant::default()
                },
                in_manager: true,
            },
        );
        self.refs.borrow_mut().insert(id, 1);
    }

    fn refs_of(&self, id: u64) -> i64 {
        self.refs.borrow().get(&id).copied().unwrap_or(0)
    }
}

impl FloraHost for Host {
    fn city_exists(&self) -> bool {
        true
    }
    fn city_size(&self) -> (f32, f32) {
        (1024.0, 1024.0)
    }
    fn city_cell_counts(&self) -> (i32, i32) {
        (64, 64)
    }
    fn city_established(&self) -> bool {
        false
    }
    fn sim_date_number(&self) -> u32 {
        1000
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn flora_exemplars(&self) -> Vec<(ResKey, Rc<PropertyHolder>)> {
        self.exemplars.borrow().clone()
    }
    fn obliterated_segment(&self) -> Option<SegmentId> {
        None
    }
    fn register_app_shutdown_callback(&self) {}
    fn subscribe(&self, _: &[u32]) {}
    fn unsubscribe(&self, _: &[u32]) {}
    fn register_command(&self, _: u32, _: &str) {}
    fn unregister_command(&self, _: u32) {}
    fn add_agent(&self, _: u32, _: &str, _: u32) {}
    fn remove_agent(&self) {}
    fn acquire_services(&self) {}
    fn release_services(&self) {}
    fn register_cheat(&self, _: u32, _: &str) {}
    fn unregister_cheat(&self, _: u32) {}
    fn post(&self, _: u32, _: u32) {}
    fn terrain_initialized(&self) -> bool {
        true
    }
    fn altitude(&self, _: f32, _: f32) -> f32 {
        self.ground.get()
    }
    fn sea_level(&self) -> f32 {
        250.0
    }
    fn face_normal(&self, _: f32, _: f32) -> Vec3 {
        [0.0, 1.0, 0.0]
    }
    fn lot_savvy_altitude(&self, _: f32, _: f32) -> (f32, bool) {
        (self.ground.get(), false)
    }
    fn moisture_type(&self, _: f32, _: f32) -> u32 {
        8
    }
    fn occupant_type(&self, occ: OccupantId) -> u32 {
        if self.occs.borrow().contains_key(&occ) {
            occupant_type::FLORA
        } else {
            0
        }
    }
    fn occupant_has_property(&self, _: OccupantId, _: u32) -> bool {
        false
    }
    fn occupant_position(&self, occ: OccupantId) -> Vec3 {
        self.occs.borrow().get(&occ).map_or([0.0; 3], |o| o.pos)
    }
    fn set_occupant_position(&self, occ: OccupantId, pos: Vec3) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.pos = pos;
        }
    }
    fn occupant_bounding_box(&self, occ: OccupantId) -> (Vec3, Vec3) {
        let p = self.occupant_position(occ);
        ([p[0] - 1.0, p[1], p[2] - 1.0], [p[0] + 1.0, p[1] + 3.0, p[2] + 1.0])
    }
    fn occupant_in_manager(&self, occ: OccupantId) -> bool {
        self.occs.borrow().get(&occ).is_some_and(|o| o.in_manager)
    }
    fn set_occupant_in_manager(&self, occ: OccupantId, on: bool) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.in_manager = on;
        }
    }
    fn network_flags(&self, _: OccupantId) -> Option<u32> {
        None
    }
    fn flora_occupant(&self, occ: OccupantId) -> Option<FloraOccupant> {
        self.occs.borrow().get(&occ).map(|o| o.flora)
    }
    fn set_flora_type(&self, occ: OccupantId, t: u32) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.flora.flora_type = t;
        }
    }
    fn set_birth_date(&self, occ: OccupantId, day: u32) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.flora.birth_date = day;
        }
    }
    fn set_last_seeding_date(&self, occ: OccupantId, day: u32) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.flora.last_seeding_date = day;
        }
    }
    fn set_flora_state(&self, occ: OccupantId, s: i8) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.flora.state = s;
        }
    }
    fn set_flora_orientation(&self, occ: OccupantId, v: u32) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.flora.orientation = v as u8;
        }
    }
    fn set_flora_record(&self, _: OccupantId, _: Option<Rc<FloraRecord>>) {}
    fn post_occupant_change(&self, _: OccupantId) {}
    fn create_flora_occupant(&self) -> Option<OccupantId> {
        let id = self.next.get();
        self.next.set(id + 1);
        self.occs.borrow_mut().insert(id, Occ::default());
        self.refs.borrow_mut().insert(id, 1);
        Some(id)
    }
    fn init_occupant(&self, _: OccupantId) {}
    fn add_ref(&self, occ: OccupantId) {
        *self.refs.borrow_mut().entry(occ).or_insert(0) += 1;
    }
    fn release(&self, occ: OccupantId) {
        *self.refs.borrow_mut().entry(occ).or_insert(0) -= 1;
    }
    fn object_ref(&self, occ: OccupantId) -> RefValue {
        RefValue::Host {
            token: occ,
            class_id: 0x0CC0_0001,
        }
    }
    fn resolve_ref(&self, r: &RefValue, _: u32) -> Option<OccupantId> {
        match r {
            RefValue::Host { token, .. } => Some(*token),
            _ => None,
        }
    }
    fn has_occupant_manager(&self) -> bool {
        true
    }
    fn insert_occupant(&self, occ: OccupantId) -> bool {
        self.set_occupant_in_manager(occ, true);
        self.add_ref(occ);
        true
    }
    fn remove_occupant(&self, occ: OccupantId) -> bool {
        if self.occupant_in_manager(occ) {
            self.set_occupant_in_manager(occ, false);
            self.release(occ);
        }
        self.removed.borrow_mut().push(occ);
        true
    }
    fn move_occupant(&self, _: OccupantId) {}
    fn occupants_in_bbox(&self, min: Vec3, max: Vec3, filter: OccupantFilter) -> Vec<OccupantId> {
        let ids: Vec<u64> = self.occs.borrow().keys().copied().collect();
        ids.into_iter()
            .filter(|&id| {
                self.occupant_in_manager(id)
                    && match filter {
                        OccupantFilter::Type(t) => t == occupant_type::FLORA,
                        OccupantFilter::Blocking => true,
                    }
                    && boxes_intersect(self.occupant_bounding_box(id), (min, max))
            })
            .collect()
    }
    fn occupants_in_cells(&self, rect: Option<CellRect>, t: u32) -> Vec<OccupantId> {
        if t != occupant_type::FLORA {
            return Vec::new();
        }
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| {
                let (cx, cz) = ((o.pos[0] / 16.0) as i32, (o.pos[2] / 16.0) as i32);
                o.in_manager && rect.map_or(true, |(x0, z0, x1, z1)| x0 <= cx && cx <= x1 && z0 <= cz && cz <= z1)
            })
            .map(|(&k, _)| k)
            .collect()
    }
    fn occupants_in_manager_cells(&self, t: u32) -> Vec<OccupantId> {
        self.occupants_in_cells(None, t)
    }
    fn segment_write_u32(&self, _: SegmentId, _: ResKey, _: u32) -> bool {
        true
    }
    fn segment_read_u32(&self, _: SegmentId, _: ResKey) -> Option<u32> {
        Some(u32::MAX)
    }
    fn segment_write_occupant(&self, _: SegmentId, _: ResKey, _: OccupantId) -> bool {
        true
    }
    fn segment_read_flora_occupant(&self, _: SegmentId, _: ResKey) -> Option<OccupantId> {
        None
    }
}

fn exemplar(t: u32, extra: &[(u32, Variant)]) -> Rc<PropertyHolder> {
    let mut h = PropertyHolder::new();
    h.add(prop::EXEMPLAR_TYPE, Variant::Scalar(Elem::U32(FLORA_EXEMPLAR_TYPE)));
    h.add(prop::TYPE_ID, Variant::Scalar(Elem::U32(t)));
    h.add(prop::TABLE, Variant::array(mem::FLOAT32, vec![Elem::F32(0.6); 256]));
    for (id, v) in extra {
        h.add(*id, v.clone());
    }
    Rc::new(h)
}

fn key(i: u32) -> ResKey {
    ResKey {
        type_id: EXEMPLAR_TYPE,
        group: 1,
        instance: i,
    }
}

fn rng() -> Box<dyn FloraRandom> {
    let seeds = FixedSeeds {
        wall_clock: 7,
        process_clock: 11,
    };
    Box::new(FloraRng::with_seed(42, &seeds))
}

fn sim_with(host: &Rc<Host>, types: &[Rc<PropertyHolder>]) -> Rc<FloraSim> {
    *host.exemplars.borrow_mut() = types
        .iter()
        .enumerate()
        .map(|(i, e)| (key(i as u32), e.clone()))
        .collect();
    let s = FloraSim::new(host.clone() as Rc<dyn FloraHost>, FloraRegistry::shared(), 1, rng());
    assert!(s.init());
    s
}

// ---- pure helpers -----------------------------------------------------------------------------------

#[test]
fn elevation_index_rounds_the_product_first() {
    assert_eq!(elevation_index(300.0, 250.0, false, 0.02), 1);
    assert_eq!(elevation_index(350.0, 250.0, false, 0.02), 2);
    assert_eq!(elevation_index(400.0, 250.0, false, 0.02), 3);
    assert_eq!(elevation_index(325.0, 250.0, false, 0.02), 1);
    assert_eq!(elevation_index(1250.0, 250.0, false, 0.02), 15);
    assert_eq!(elevation_index(220.0, 250.0, false, 0.02), 0);
    assert_eq!(elevation_index(130.0, 250.0, true, 0.02), 2);
    assert_eq!(elevation_index(320.0, 250.0, false, 0.08), 5);
    assert_eq!(elevation_index(f32::NAN, 250.0, false, 0.02), 0);
    assert_eq!(elevation_index(f32::INFINITY, 250.0, false, 0.02), 15);
}

#[test]
fn table_bytes_and_scores() {
    assert_eq!(table_byte(0.6), 153);
    assert_eq!(table_byte(0.1), 25);
    assert_eq!(table_byte(0.9), 229);
    assert_eq!(table_byte(0.4), 102);
    assert_eq!(table_byte(1.0), 255);
    assert_eq!(table_byte(1.2), 50);
    assert_eq!(table_byte(f32::NAN), 0);
    let mut t = [0u8; 256];
    t[16 + 8] = 153;
    t[16 + 3] = 25;
    assert_eq!(score_from_table(&t, 1, 8), 0.6);
    assert!((score_from_table(&t, 1, 3) - 0.098_039_2).abs() < 1e-7);
    assert_eq!(score_from_table(&t, 1, 16), 0.0);
    assert_eq!(score_from_table(&t, 16, 8), 0.0);
}

#[test]
fn blocks_and_wild_seed_scaling() {
    assert_eq!(blocks_per_month(64, 64, 24), 42);
    assert_eq!(blocks_per_month(128, 128, 24), 170);
    assert_eq!(blocks_per_month(256, 256, 24), 682);
    assert_eq!(blocks_per_month(64, 64, 6), 170);
    assert_eq!(blocks_per_month(64, 64, 0), 0);
    assert_eq!(blocks_per_month(-1, 1, 1), u32::MAX >> 2);
    assert_eq!(scale_wild_seeds(5, 1024.0, 1024.0), 5);
    assert_eq!(scale_wild_seeds(5, 2048.0, 2048.0), 20);
    assert_eq!(scale_wild_seeds(5, 4096.0, 4096.0), 80);
    assert_eq!(scale_wild_seeds(0, 1024.0, 1024.0), 0);
    let _ = scale_wild_seeds(u32::MAX, f32::MAX, f32::NAN);
}

#[test]
fn ftol_matches_the_x87_indefinite() {
    assert_eq!(ftol(2.9), 2);
    assert_eq!(ftol(-2.9), -2);
    assert_eq!(ftol(f64::NAN), i64::MIN);
    assert_eq!(ftol(1e30), i64::MIN);
}

#[test]
fn blocking_filter_and_boxes() {
    assert!(blocks_flora(occupant_type::PROP, None));
    assert!(blocks_flora(occupant_type::NETWORK, Some(0x8)));
    assert!(!blocks_flora(occupant_type::NETWORK, Some(0x20)));
    assert!(!blocks_flora(occupant_type::NETWORK, None));
    assert!(!blocks_flora(0x1234, Some(0x8)));
    let a = ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    assert!(boxes_intersect(a, ([1.0, 1.0, 1.0], [2.0, 2.0, 2.0])));
    assert!(!boxes_intersect(a, ([1.5, 0.0, 0.0], [2.0, 2.0, 2.0])));
}

#[test]
fn interfaces() {
    assert_eq!(query_interface(IID_FLORA_SIMULATOR), Some(FloraInterface::Primary));
    assert_eq!(query_interface(IID_UNKNOWN), Some(FloraInterface::Primary));
    assert_eq!(
        query_interface(IID_MESSAGE_TARGET2),
        Some(FloraInterface::MessageTarget)
    );
    assert_eq!(query_interface(IID_SERIALIZABLE), Some(FloraInterface::Serializable));
    assert_eq!(query_interface(IID_OCCUPANT), None);
}

// ---- registry ---------------------------------------------------------------------------------------

fn record(t: u32, next: u32) -> Rc<FloraRecord> {
    let mut r = FloraRecord::new(t);
    r.next_type = next;
    Rc::new(r)
}

#[test]
fn seed_types_depend_on_registration_order() {
    let mut reg = FloraRegistry::new();
    assert!(reg.add(record(0x120, 0)));
    assert!(reg.add(record(0x110, 0x120)));
    assert!(!reg.add(record(0x110, 0)));
    assert_eq!(reg.get(0x120).unwrap().seed_type.get(), 0x110);
    let mut reg = FloraRegistry::new();
    reg.add(record(0x110, 0x120));
    reg.add(record(0x120, 0));
    assert_eq!(reg.get(0x120).unwrap().seed_type.get(), 0x120);
    assert_eq!(reg.types(), &[0x110, 0x120]);
}

#[test]
fn advance_cluster_type_terminates() {
    let mut reg = FloraRegistry::new();
    reg.add(record(0x10, 0x20));
    reg.add(record(0x20, 0x30));
    reg.add(record(0x30, 0));
    reg.add(record(0x50, 0x60));
    reg.add(record(0x60, 0x50));
    reg.add(record(0x70, 0x70));
    reg.insert_default();
    assert_eq!(advance_cluster_type(&reg, 0x10, 0x10), 0x20);
    assert_eq!(advance_cluster_type(&reg, 0x10, 0x20), 0x30);
    assert_eq!(advance_cluster_type(&reg, 0x20, 0x10), 0x30);
    assert_eq!(advance_cluster_type(&reg, 0x30, 0x10), 0);
    assert_eq!(advance_cluster_type(&reg, 0x10, 0x55), 0);
    assert_eq!(advance_cluster_type(&reg, 0x50, 0x60), 0x50);
    // A cycle that never reaches the target ends.
    assert_eq!(advance_cluster_type(&reg, 0x50, 0x10), 0);
    assert_eq!(advance_cluster_type(&reg, 0x70, 0x10), 0);
    assert_eq!(advance_cluster_type(&reg, 0, 0), 0);
}

#[test]
fn exemplar_parsing_and_flags() {
    let h = exemplar(
        0x99,
        &[
            (prop::NEVER_AUTO, Variant::Scalar(Elem::Bool(true))),
            (prop::FLAG_SWITCH, Variant::Scalar(Elem::Bool(false))),
            (prop::FLOATS, Variant::array(mem::FLOAT32, vec![Elem::F32(1.0); 20])),
            (prop::TABLE, Variant::array(mem::FLOAT32, vec![Elem::F32(1.2); 3])),
        ],
    );
    let r = FloraRecord::from_exemplar(&h);
    assert_eq!(r.type_id, 0x99);
    assert_eq!(r.flags, flag::NEVER_AUTO | flag::BIT2);
    assert_eq!(r.floats(), [1.0; 14]);
    assert_eq!(&r.table[..3], &[50, 50, 50]);
    assert_eq!(r.table[5 * 16 + 5], DEFAULT_TABLE_BYTE);
    assert_eq!(r.table[3], 0);
}

#[test]
fn app_shutdown_keeps_the_families() {
    let mut reg = FloraRegistry::new();
    let mut r = FloraRecord::new(5);
    r.families = vec![9];
    reg.add(Rc::new(r));
    reg.insert_default();
    reg.app_shutdown();
    assert!(reg.keys().is_empty());
    assert!(reg.types().is_empty());
    assert_eq!(reg.family(9), Some(&[5][..]));
}

// ---- generator ----------------------------------------------------------------------------------------

#[test]
fn generator_is_deterministic_and_24_bit() {
    let mut a = rng();
    let mut b = rng();
    for _ in 0..200 {
        let u = a.uniform_f64();
        assert_eq!(u, b.uniform_f64());
        assert!((0.0..=1.0).contains(&u));
        assert_eq!(f64::from(u as f32), u);
        let r = a.range_f64(-118.0, 138.0);
        assert_eq!(r, b.range_f64(-118.0, 138.0));
        assert!((-118.0..=138.0).contains(&r));
        let g = a.gaussian();
        assert_eq!(g, b.gaussian());
        assert!(g.is_finite());
        let i = a.range_i32(0, 4);
        assert_eq!(i, b.range_i32(0, 4));
    }
    // The state (with the Gaussian cache) restores the sequence.
    a.gaussian();
    let s = a.state();
    let next: Vec<f64> = (0..4).map(|_| a.gaussian()).collect();
    let mut c = rng();
    c.set_state(s);
    let again: Vec<f64> = (0..4).map(|_| c.gaussian()).collect();
    assert_eq!(next, again);
}

// ---- simulator flows ------------------------------------------------------------------------------------

#[test]
fn init_reads_city_and_tuning() {
    let host = Host::new();
    let mut t = PropertyHolder::new();
    t.add(prop::PERIOD, Variant::Scalar(Elem::U32(24)));
    t.add(prop::WILD_SEEDS, Variant::Scalar(Elem::U32(0)));
    t.add(prop::ELEVATION_SCALE, Variant::Scalar(Elem::F32(800.0)));
    t.add(prop::BURN_RATE, Variant::Scalar(Elem::F32(1.0)));
    t.add(prop::BURNT_REMOVAL_STEP, Variant::Scalar(Elem::F32(1.0)));
    *host.tuning.borrow_mut() = Some(Rc::new(t));
    let s = sim_with(&host, &[exemplar(0x1000, &[])]);
    let d = s.snapshot();
    assert_eq!((d.period, d.blocks_per_month, d.wild_seeds_per_month), (24, 42, 0));
    assert_eq!(d.elevation_scale, 0.02);
    assert_eq!((d.iterator.x1, d.iterator.z1, d.iterator.table.len()), (31, 31, 1024));
    assert!(s.registry().borrow().contains(0));
    assert!(s.shutdown());
    assert!(!s.is_initialized());
}

#[test]
fn kill_and_daily_removal() {
    let host = Host::new();
    let s = sim_with(&host, &[exemplar(0x1000, &[])]);
    for id in 1..=5 {
        host.add_flora(id, 100.0 + id as f32 * 20.0, 100.0, 0x1000, 1);
        assert!(s.kill_flora_occupant(id));
    }
    assert!(s.kill_flora_occupant(1));
    assert!(!s.kill_flora_occupant(77));
    assert_eq!(s.snapshot().burnt, vec![1, 2, 3, 4, 5]);
    assert_eq!(host.refs_of(1), 2);
    s.daily(); // code defaults: four a day
    assert_eq!(*host.removed.borrow(), vec![1, 2, 3, 4]);
    assert_eq!(host.refs_of(1), 0);
    s.daily();
    assert!(s.snapshot().burnt.is_empty());
    assert!(s.shutdown());
}

#[test]
fn planting_raises_land_flora_to_sea_level() {
    let host = Host::new();
    host.ground.set(240.0);
    let s = sim_with(
        &host,
        &[
            exemplar(0x1000, &[]),
            exemplar(0x2000, &[(prop::WATER, Variant::Scalar(Elem::Bool(true)))]),
        ],
    );
    let mut pos = [500.0, 0.0, 500.0];
    let occ = s.add_new_flora_occupant(0x2000, &mut pos, false).expect("planted");
    assert_eq!(pos, [500.0, 240.0, 500.0]);
    assert!(host.occupant_in_manager(occ));
    // Land flora is refused under water off a lot, but forced planting works.
    let mut pos = [600.0, 0.0, 600.0];
    assert!(s.add_new_flora_occupant(0x1000, &mut pos, false).is_none());
    let mut pos = [600.0, 0.0, 600.0];
    s.add_new_flora_occupant(0x1000, &mut pos, true).expect("forced");
    assert_eq!(pos[1], 250.0);
}

#[test]
fn hostile_loops_are_bounded() {
    let host = Host::new();
    // Every type never-auto: wild seeding must not hang.
    let s = sim_with(
        &host,
        &[exemplar(
            0x1000,
            &[(prop::NEVER_AUTO, Variant::Scalar(Elem::Bool(true)))],
        )],
    );
    {
        let mut d = s.data_mut();
        d.wild_seeds_per_month = u32::MAX;
        d.blocks_per_month = u32::MAX;
        d.iterator.set_rect(5, 5, 0, 0);
        d.iterator.table.clear();
        d.iterator.count = 0;
    }
    s.monthly();
    // An empty type array.
    let s2 = FloraSim::new(host.clone() as Rc<dyn FloraHost>, FloraRegistry::shared(), 2, rng());
    s2.data_mut().wild_seeds_per_month = 10;
    s2.wild_seeds();
    // A huge blast is capped.
    s.blast(u32::MAX, 0, 1);
    // A huge seed count is capped and a NaN count gives no seed.
    let mut r = FloraRecord::new(0x3000);
    r.seeds_per_cycle = f32::NAN;
    s.add_static_flora_data(Rc::new(r));
    host.add_flora(9, 500.0, 500.0, 0x3000, 2);
    let fo = host.flora_occupant(9).unwrap();
    s.simulate_existing(9, fo);
    // The obliteration count is bounded and nothing is read.
    assert!(s.restore_obliterated(1));
}

#[test]
fn save_round_trip_and_hostile_reads() {
    let host = Host::new();
    let s = sim_with(&host, &[exemplar(0x1000, &[])]);
    {
        let mut d = s.data_mut();
        d.enabled = false;
        d.preserve = true;
        d.today = 1234;
        d.burn_accumulator = 0.25;
    }
    let mut w = ByteWriter::new();
    assert!(s.write(&mut w));
    let bytes = w.into_bytes();
    assert_eq!(bytes.len(), 2158);
    let t = FloraSim::new(host.clone() as Rc<dyn FloraHost>, FloraRegistry::shared(), 2, rng());
    let mut r = ByteReader::new(&bytes);
    assert!(t.read(&mut r));
    assert_eq!(r.position(), bytes.len());
    let (a, b) = (s.snapshot(), t.snapshot());
    assert_eq!(
        (b.enabled, b.preserve, b.today, b.burn_accumulator),
        (false, true, 1234, 0.25)
    );
    assert_eq!(a.iterator, b.iterator);
    assert_eq!(s.rng_state(), t.rng_state());
    assert_eq!(s.draw_uniform(), t.draw_uniform());
    // Version 3 is rejected after the version.
    let mut bad = bytes.clone();
    bad[0] = 3;
    let mut r = ByteReader::new(&bad);
    assert!(!t.read(&mut r));
    assert_eq!(r.position(), 2);
    // Every truncation fails cleanly; garbage never panics.
    for n in 0..bytes.len() {
        let mut r = ByteReader::new(&bytes[..n]);
        let _ = t.read(&mut r);
    }
    let mut x: u32 = 0x1234_5678;
    for _ in 0..64 {
        let mut junk = vec![2u8, 0];
        for _ in 0..300 {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
            junk.push((x >> 16) as u8);
        }
        let mut r = ByteReader::new(&junk);
        let _ = t.read(&mut r);
    }
}

#[test]
fn messages_always_finish() {
    let host = Host::new();
    let s = sim_with(&host, &[exemplar(0x1000, &[])]);
    for t in [
        msg::SIM_NEW_DAY,
        msg::SIM_NEW_MONTH,
        msg::INSERT_OCCUPANT,
        msg::REMOVE_OCCUPANT,
        msg::TERRAIN_HEIGHTS_CHANGED,
        msg::EXEMPLAR_CHANGED,
        msg::CITY_ESTABLISHED,
        msg::CITY_OBLITERATED,
        msg::PLACE_FLORA,
        msg::CHEAT,
        0x1234,
    ] {
        let m = Message {
            msg_type: t,
            data1: u32::MAX,
            data2: u32::MAX,
            data3: u32::MAX,
            occupant: Some(u64::MAX),
            text: Some("Flora blast 99999999999 x".into()),
            params: vec![CommandParam::default(); 3],
        };
        assert!(s.do_message(&m));
    }
}
