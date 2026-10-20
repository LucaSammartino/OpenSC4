// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests for the tract developer: the save record, the tuning reader,
//! the styles, the pure helpers, a small Grow on a scripted host, and
//! hostile hosts and records (no panic).

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use super::*;
use crate::development::{ConstructionProfile, GrowthProfile, Outcome};
use crate::property::PropertyHolder;
use crate::rng::FixedSeeds;
use crate::stream::{ByteReader, ByteWriter};

const SEEDS: FixedSeeds = FixedSeeds {
    wall_clock: 1,
    process_clock: 1,
};

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
    fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next() % n
        }
    }
    fn pick<T: Copy>(&mut self, v: &[T]) -> T {
        v[self.below(v.len() as u32) as usize]
    }
}

// ---- pure helpers -------------------------------------------------------------------------------

#[test]
fn desired_handles_hostile_demand() {
    assert_eq!(desired_of(Some(7.9), 1.5), (7, 10));
    assert_eq!(desired_of(Some(f32::NAN), 1.5), (1, 1));
    assert_eq!(desired_of(None, 1.5), (1, 1));
    for d in [f32::INFINITY, f32::NEG_INFINITY, f32::MAX, -f32::MAX, 3e9] {
        for o in [1.5f32, f32::NAN, f32::INFINITY, -1.0, 0.0] {
            let (a, _) = desired_of(Some(d), o);
            assert!(a >= 1);
        }
    }
}

#[test]
fn capacity_window_never_overflows() {
    let mut c = Candidate {
        kind: 2,
        existing: i32::MAX,
        ..Candidate::default()
    };
    // lo stays `existing` when `existing + 1` overflows; hi wraps like the
    // original's int32 arithmetic.
    let (lo, _) = capacity_window(&c, i32::MAX, i32::MIN);
    assert_eq!(lo, i32::MAX);
    c.kind = 4;
    for n in [0, -1, 1, 3, i32::MIN] {
        c.n = n;
        c.existing = i32::MIN;
        let _ = capacity_window(&c, i32::MIN, i32::MAX);
    }
}

#[test]
fn styles_points_and_groups() {
    assert_eq!(style_points(0.3), 30);
    assert_eq!(style_points(0.125), 12);
    assert_eq!(style_points(0.005), 0);
    for p in [f32::NAN, -1.0, 1e30, f32::INFINITY] {
        let _ = style_points(p);
    }
    let map = [(0x2000, 0.3f32), (0x2001, 0.7)];
    assert_eq!(style_group(&map, 4), 0x2000);
    assert_eq!(style_group(&map, 30), 0x2001);
    assert_eq!(style_group(&map, 99), 0x2001);
    assert_eq!(style_group(&[], 5), 0);
    // Hostile fractions never panic (the points saturate like `rhe`).
    let _ = style_group(&[(1, f32::NAN), (2, 1e30), (3, -5.0)], 50);
}

#[test]
fn geometry_is_total() {
    for f in [-7, -1, 0, 1, 2, 3, 4, 5, i32::MIN, i32::MAX] {
        let _ = candidate_rect(i32::MAX, i32::MIN, i32::MAX, i32::MIN, f);
    }
    assert_eq!(zone_family(2), 1);
    assert_eq!(zone_family(5), 2);
    assert_eq!(zone_family(9), 3);
    assert_eq!(zone_family(0), 0);
    assert_eq!(zone_family(i32::MIN), 0);
    // Test 25 (§4.4 ConvertZoneToType): zones 0-10 and 15.
    let zones = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15];
    let types = [0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 0, 0];
    for (z, t) in zones.iter().zip(types) {
        assert_eq!(zone_family(*z), t, "zone {z}");
    }
    assert!(answers_iid(IID_TRACT_DEVELOPER));
    assert!(!answers_iid(CLSID_TRACT_DEVELOPER));
}

// ---- tuning, styles, save -----------------------------------------------------------------------

#[test]
fn tuning_defaults_and_years() {
    let mut t = Tuning::constructor();
    assert_eq!(t.min_vacancy, [0.1, 0.1]);
    let mut years = 0;
    t.read(None, &mut years);
    assert_eq!(years, 40);
    assert_eq!(t.new_lot, [10.0, 5.0]);
    let shipped = Tuning::shipped_exemplar();
    let mut years = 0;
    t.read(Some(&shipped), &mut years);
    assert_eq!(years, 5);
    assert_eq!(t.available_styles, vec![0x2000, 0x2001, 0x2002, 0x2003]);
    assert_eq!(t.occupancy_variation, 0.0);
    // A nonzero field is not read again.
    let mut years = 9;
    t.read(Some(&shipped), &mut years);
    assert_eq!(years, 9);
}

fn dev_with(host: Rc<dyn TractDeveloperHost>) -> TractDeveloper {
    TractDeveloper::new(host, TractDeveloperStatics::new(RNG_SEED_XOR, &SEEDS))
}

#[test]
fn styles_rotate_and_survive_empty_lists() {
    let mut d = dev_with(Rc::new(Script::default()));
    // Empty list: NewYear and SetActiveStyles stay in range.
    d.set_years_between_style_changes(0);
    for _ in 0..5 {
        d.new_year();
    }
    assert_eq!(d.current_style_index(), 0);
    assert_eq!(d.current_style(), 0);
    d.set_active_styles(&[1, 2, 3]);
    assert_eq!(d.current_style_index(), 0);
    d.set_current_style_index(-5);
    assert_eq!(d.current_style(), 0);
    d.set_active_styles(&[3, 2, 3]);
    assert_eq!(d.current_style_index(), 0);
    d.set_current_style_index(1);
    d.set_active_styles(&[2, 2, 9]);
    assert_eq!(d.current_style_index(), 1);
    d.set_years_between_style_changes(i32::MIN);
    d.set_years_since_style_change(i32::MAX);
    d.new_year();
}

#[test]
fn save_round_trip_and_hostile_records() {
    let mut g = Lcg(3);
    for _ in 0..50 {
        let mut a = dev_with(Rc::new(Script::default()));
        let n = g.below(9) as usize;
        a.assign_active_styles((0..n).map(|_| g.next()).collect());
        a.set_current_style_index(g.next() as i32);
        a.set_years_since_style_change(g.next() as i32);
        a.set_years_between_style_changes(g.next() as i32);
        a.use_all_styles_at_once(g.next() & 1 == 1);
        let mut w = ByteWriter::new();
        assert!(a.write(&mut w));
        let bytes = w.into_bytes();
        assert_eq!(bytes.len(), 2 + 4 + 4 + 1 + 4 + 4 * n + 4);
        let mut b = dev_with(Rc::new(Script::default()));
        assert!(b.read(&mut ByteReader::new(&bytes)));
        assert_eq!(b.active_styles(), a.active_styles());
        assert_eq!(b.current_style_index(), a.current_style_index());
        assert_eq!(b.years_since_style_change(), a.years_since_style_change());
        assert_eq!(b.years_between_style_changes(), a.years_between_style_changes());
        assert_eq!(b.is_using_all_styles_at_once(), a.is_using_all_styles_at_once());
        // Every truncation fails cleanly.
        for cut in 0..bytes.len() {
            let mut c = dev_with(Rc::new(Script::default()));
            assert!(!c.read(&mut ByteReader::new(&bytes[..cut])), "cut {cut}");
        }
    }
    // Version > 3 changes nothing.
    let mut d = dev_with(Rc::new(Script::default()));
    d.set_years_between_style_changes(5);
    assert!(!d.read(&mut ByteReader::new(&[4, 0, 1, 0, 0, 0])));
    assert_eq!(d.years_between_style_changes(), 5);
    // A huge count is capped and zero-padded.
    let mut rec = vec![3u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    rec.extend_from_slice(&u32::MAX.to_le_bytes());
    rec.extend_from_slice(&7u32.to_le_bytes());
    assert!(!d.read(&mut ByteReader::new(&rec)));
    assert_eq!(d.active_styles().len(), MAX_SAVED_STYLES);
    assert_eq!(d.active_styles()[0], 7);
    // Random garbage never panics.
    for _ in 0..500 {
        let len = g.below(64) as usize;
        let mut junk: Vec<u8> = (0..len).map(|_| g.next() as u8).collect();
        if len >= 2 && g.next() & 1 == 0 {
            junk[0] = g.below(4) as u8;
            junk[1] = 0;
        }
        let _ = d.read(&mut ByteReader::new(&junk));
        let _ = d.current_style();
    }
}

// ---- a scripted host ----------------------------------------------------------------------------

#[derive(Clone)]
struct Lot {
    x: i32,
    z: i32,
    state: u32,
    capacity: u16,
    building: u32,
    wealth: u32,
}

/// A small world: residential developer 1, 1 x 1 lots, one configuration
/// (token 100, family 600 = {0x300}, capacity 2), unlimited resources.
struct Script {
    lots: RefCell<BTreeMap<u64, Lot>>,
    next: Cell<u64>,
    log: RefCell<Vec<String>>,
    demand: f32,
}

impl Default for Script {
    fn default() -> Script {
        Script {
            lots: RefCell::new(BTreeMap::new()),
            next: Cell::new(1000),
            log: RefCell::new(Vec::new()),
            demand: 3.0,
        }
    }
}

impl Script {
    fn with_lots(cells: &[(i32, i32)]) -> Script {
        let s = Script::default();
        for (i, &(x, z)) in cells.iter().enumerate() {
            s.lots.borrow_mut().insert(
                i as u64 + 1,
                Lot {
                    x,
                    z,
                    state: 1,
                    capacity: 0,
                    building: 0,
                    wealth: 1,
                },
            );
        }
        s
    }
    fn lot<T>(&self, l: u64, f: impl FnOnce(&Lot) -> T, def: T) -> T {
        self.lots.borrow().get(&l).map_or(def, f)
    }
    fn say(&self, s: String) {
        self.log.borrow_mut().push(s);
    }
}

impl TractDeveloperHost for Script {
    fn acquire_services(&self) -> bool {
        true
    }
    fn add_agent(&self, _: u32, _: u32) -> bool {
        true
    }
    fn acquire_message_server(&self) -> bool {
        true
    }
    fn release_message_server(&self) {}
    fn remove_agent(&self) {}
    fn release_services(&self) {}
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        Some(Rc::new(Tuning::shipped_exemplar()))
    }
    fn dev_target_zone(&self, _: u64) -> i32 {
        1
    }
    fn dev_subtype(&self, _: u64) -> u32 {
        0x1010
    }
    fn dev_purpose(&self, _: u64) -> u32 {
        1
    }
    fn dev_wealth(&self, _: u64) -> u32 {
        1
    }
    fn dev_construction_info(&self, _: u64) -> ConstructionProfile {
        ConstructionProfile {
            zone: 1,
            wealth: 1,
            purpose: 1,
            ..ConstructionProfile::default()
        }
    }
    fn dev_desirability_tract_size(&self, _: u64) -> i32 {
        4
    }
    fn dev_desirability_shift(&self, _: u64) -> u32 {
        2
    }
    fn dev_desirability_value(&self, _: u64, _: i32, _: i32) -> i16 {
        100
    }
    fn dev_capacity_value(&self, _: u64, _: i32, _: i32) -> u32 {
        0
    }
    fn dev_source_demand(&self, _: u64) -> Option<f32> {
        Some(self.demand)
    }
    fn dev_cap(&self, _: u64, _: Cap) -> u8 {
        255
    }
    fn dev_monopoly_use(&self, _: u64) -> i32 {
        -1
    }
    fn dev_style_percentages(&self, _: u64) -> Vec<(u32, f32)> {
        Vec::new()
    }
    fn building_profile(&self, _: u32) -> BuildingProfile {
        BuildingProfile::basic(1, 1, 2)
    }
    fn building_types_by_family(&self, family: u32) -> Vec<u32> {
        if family == 600 {
            vec![0x300]
        } else {
            Vec::new()
        }
    }
    fn building_in_occupant_group(&self, _: u32, _: u32) -> bool {
        true
    }
    fn building_is_monopoly(&self, _: u32) -> bool {
        false
    }
    fn building_field_ids(&self, _: u32) -> Option<Vec<u32>> {
        None
    }
    fn developer_for(&self, _: u32, _: u32) -> Option<u64> {
        Some(1)
    }
    fn claim_construction_resources(&self, amount: u16) -> bool {
        self.say(format!("claim {amount}"));
        true
    }
    fn release_construction_resources(&self, amount: u16) {
        self.say(format!("release {amount}"));
    }
    fn cheat(&self, _: Cheat) -> bool {
        false
    }
    fn needs_anchor_building(&self, _: CellRect, _: u32) -> bool {
        false
    }
    fn start_construction(&self, lot: u64, cfg: u64, building: u32, _: bool) -> bool {
        self.say(format!("start {lot} {cfg} {building:#x}"));
        if let Some(l) = self.lots.borrow_mut().get_mut(&lot) {
            l.state = lot_state::CONSTRUCTING;
            l.building = building;
            l.capacity = 2;
        }
        true
    }
    fn abandon_building(&self, lot: u64, reason: u32) -> bool {
        self.say(format!("abandon {lot} {reason}"));
        if let Some(l) = self.lots.borrow_mut().get_mut(&lot) {
            l.state = lot_state::ABANDONED;
        }
        true
    }
    fn reoccupy_building(&self, lot: u64, _: u32) -> bool {
        self.say(format!("reoccupy {lot}"));
        true
    }
    fn area_slope(&self, _: CellRect) -> f32 {
        0.0
    }
    fn get_lot(&self, x: i32, z: i32) -> Option<u64> {
        self.lots
            .borrow()
            .iter()
            .find(|(_, l)| l.x == x && l.z == z)
            .map(|(&k, _)| k)
    }
    fn lots_in_cell_rect(&self, r: CellRect, f: &mut dyn FnMut(u64)) {
        let hits: Vec<u64> = self
            .lots
            .borrow()
            .iter()
            .filter(|(_, l)| l.x >= r.x1 && l.x <= r.x2 && l.z >= r.z1 && l.z <= r.z2)
            .map(|(&k, _)| k)
            .collect();
        for k in hits {
            f(k);
        }
    }
    fn lots_surrounding_lot(&self, _: u64, _: u32) -> Vec<u64> {
        Vec::new()
    }
    fn lots_surrounding_rect(&self, _: CellRect, _: u32) -> Vec<u64> {
        Vec::new()
    }
    fn can_create_lot(&self, _: i32, _: i32, _: i32, _: i32) -> i32 {
        7
    }
    fn create_lot_if_possible(&self, x: i32, z: i32, _: i32, _: i32, _: i32) -> Option<u64> {
        let t = self.next.get();
        self.next.set(t + 1);
        self.lots.borrow_mut().insert(
            t,
            Lot {
                x,
                z,
                state: 1,
                capacity: 0,
                building: 0,
                wealth: 1,
            },
        );
        Some(t)
    }
    fn delete_lot(&self, lot: u64) -> bool {
        self.lots.borrow_mut().remove(&lot).is_some()
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.lot(lot, |l| CellRect::new(l.x, l.z, l.x, l.z), CellRect::new(0, 0, -1, -1))
    }
    fn lot_slope(&self, _: u64) -> f32 {
        0.0
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        self.lot(lot, |l| (l.x, l.z), (0, 0))
    }
    fn lot_size(&self, _: u64) -> (u32, u32) {
        (1, 1)
    }
    fn lot_facing(&self, _: u64) -> i32 {
        0
    }
    fn lot_set_facing(&self, _: u64, _: i32) {}
    fn lot_has_road_access(&self, _: u64) -> bool {
        true
    }
    fn lot_has_organic_road_access(&self, _: u64, _: u32) -> bool {
        true
    }
    fn lot_zone_type(&self, _: u64) -> i32 {
        1
    }
    fn lot_state(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.state, 0)
    }
    fn lot_set_state(&self, lot: u64, state: u32, _: u32, _: u32) {
        if let Some(l) = self.lots.borrow_mut().get_mut(&lot) {
            l.state = state;
        }
    }
    fn lot_is_livable(&self, _: u64) -> bool {
        false
    }
    fn lot_has_capacity(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.capacity > 0, false)
    }
    fn lot_days_since_state_change(&self, _: u64) -> u32 {
        100
    }
    fn lot_occupant_wealth(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.wealth, 0)
    }
    fn lot_is_historical(&self, _: u64) -> bool {
        false
    }
    fn lot_building_type(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.building, 0)
    }
    fn lot_capacity(&self, lot: u64, _: u32, _: bool) -> u16 {
        self.lot(lot, |l| l.capacity, 0)
    }
    fn lot_available_capacity(&self, lot: u64, _: u32, _: bool) -> u16 {
        self.lot(lot, |l| l.capacity, 0)
    }
    fn lot_set_population(&self, lot: u64, _: u32, population: u16) {
        self.say(format!("population {lot} {population}"));
    }
    fn lot_configuration_stage(&self, _: u64) -> Option<u8> {
        Some(1)
    }
    fn cfg_size(&self, _: u64) -> (u32, u32) {
        (1, 1)
    }
    fn cfg_min_slope(&self, _: u64) -> f32 {
        -100.0
    }
    fn cfg_max_slope(&self, _: u64) -> f32 {
        100.0
    }
    fn cfg_growth_stage(&self, _: u64) -> u8 {
        1
    }
    fn cfg_min_capacity(&self, _: u64) -> u16 {
        0
    }
    fn cfg_max_capacity(&self, _: u64) -> u16 {
        u16::MAX
    }
    fn cfg_compatible_zone(&self, _: u64, _: i32) -> bool {
        true
    }
    fn cfg_compatible_wealth(&self, _: u64, w: u32) -> bool {
        w == 1
    }
    fn cfg_compatible_purpose(&self, _: u64, p: u32) -> bool {
        p == 1
    }
    fn cfg_compatible_style(&self, _: u64, _: u32) -> bool {
        true
    }
    fn cfg_required_roads(&self, _: u64) -> Vec<bool> {
        Vec::new()
    }
    fn cfg_primary_building_ids(&self, _: u64) -> Option<Vec<u32>> {
        Some(vec![600])
    }
    fn get_lot_configuration(&self, _: u32) -> Option<u64> {
        None
    }
    fn register_configuration_filter(&self, _: u32, _: u32) -> u64 {
        50
    }
    fn unregister_configuration_filter(&self, _: u64) {}
    fn configurations_by_filter(&self, _: u64) -> Vec<u64> {
        vec![100]
    }
    fn zone_at(&self, _: i32, _: i32) -> i32 {
        1
    }
    fn land_value(&self, _: i32, _: i32) -> u8 {
        0
    }
    fn powered_value(&self, _: i32, _: i32) -> i32 {
        2
    }
    fn is_cell_watered(&self, _: i32, _: i32) -> bool {
        true
    }
    fn fire_coverage(&self, _: i32, _: i32) -> u8 {
        1
    }
    fn park_value(&self, _: i32, _: i32) -> i16 {
        1
    }
    fn road_access_value(&self, _: i32, _: i32) -> i32 {
        1
    }
    fn city_cell_counts(&self) -> (i32, i32) {
        (64, 64)
    }
    fn find_network_tile(&self, _: i32, _: i32) -> Option<u64> {
        None
    }
    fn network_is_of_type(&self, _: u64, _: u32) -> bool {
        false
    }
    fn demolish_lot(&self, lot: u64) -> bool {
        self.say(format!("demolish {lot}"));
        true
    }
    fn redisplay_terrain(&self, _: CellRect) {}
}

fn profile(x: i32, z: i32, stage: i32) -> GrowthProfile {
    GrowthProfile {
        developer: 1,
        x,
        z,
        stage,
    }
}

#[test]
fn grow_builds_relaxed_passes_until_desired() {
    // Demand 3: desired 3; each lot takes a capacity-2 building. Pass 1
    // builds on one lot (built 2 < 3), pass 2 on another (built 4): stop.
    let host = Rc::new(Script::with_lots(&[(0, 0), (1, 0), (2, 0)]));
    let mut d = dev_with(host.clone());
    assert!(d.init());
    let mut out = Vec::new();
    let mut flag = false;
    let built = d.grow(&profile(0, 0, 1), &mut out, &mut flag);
    assert_eq!(built, 4);
    assert_eq!(out, vec![Outcome::Success, Outcome::Success]);
    assert_eq!(d.desired(), (3, 4));
    let log = host.log.borrow();
    assert_eq!(log.iter().filter(|s| s.starts_with("start")).count(), 2);
    assert_eq!(log.iter().filter(|s| s.starts_with("claim 2")).count(), 2);
    // Outcomes are appended to, never cleared.
    drop(log);
    let _ = d.grow(&profile(0, 0, 1), &mut out, &mut flag);
    assert!(out.len() > 2);
}

#[test]
fn decline_and_population_on_the_scripted_host() {
    let host = Rc::new(Script::with_lots(&[(0, 0)]));
    {
        let mut lots = host.lots.borrow_mut();
        let l = lots.get_mut(&1).unwrap();
        l.state = lot_state::OCCUPIED;
        l.building = 0x300;
        l.capacity = 40;
    }
    let mut d = dev_with(host.clone());
    assert!(d.init());
    // Shipped variation 0, d = 100: occ = 1 -> full -> 1 - U(0.01, 0.1).
    d.update_population(&profile(0, 0, 0));
    assert!(host.log.borrow().iter().any(|s| s.starts_with("population 1 ")));
    assert_eq!(d.decline(&profile(0, 0, 0)), 40);
    assert_eq!(host.lot(1, |l| l.state, 0), lot_state::ABANDONED);
    assert_eq!(d.decline(&profile(0, 0, 0)), 0);
}

#[test]
fn update_population_tract_zero_uses_seed_one() {
    // The same lot in tract (0, 0) gives the same population on every
    // call: the temporary generator is fixed (seed 1), not clock-seeded.
    let host = Rc::new(Script::with_lots(&[(0, 0)]));
    {
        let mut lots = host.lots.borrow_mut();
        let l = lots.get_mut(&1).unwrap();
        l.state = lot_state::OCCUPIED;
        l.capacity = 40;
    }
    let mut d = dev_with(host.clone());
    d.update_population(&profile(0, 0, 0));
    d.update_population(&profile(0, 0, 0));
    let log = host.log.borrow();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0], log[1]);
    // Constructor tuning (no Init): variation 0.2, minVacancy {0.1, 0.1}.
    // Tract (0, 0), d = 100: the first draw makes occ >= 1, the second is
    // U(0.1, 0.1) = 0.1, so occ = 0.9 and cap 40 -> 36.
    assert_eq!(log[0], "population 1 36");
}

#[test]
fn update_population_nan_occupancy_draws_once() {
    // A NaN variation makes occ NaN: it compares as "< 1", so there is no
    // second draw, and the population is 0 (§4.18).
    let host = Rc::new(Script::with_lots(&[(0, 0)]));
    {
        let mut lots = host.lots.borrow_mut();
        let l = lots.get_mut(&1).unwrap();
        l.state = lot_state::OCCUPIED;
        l.capacity = 40;
    }
    let statics = TractDeveloperStatics::new(RNG_SEED_XOR, &SEEDS);
    statics.borrow_mut().trace = Some(Vec::new());
    let mut d = TractDeveloper::new(host.clone(), statics.clone());
    d.set_tuning(Tuning {
        occupancy_variation: f32::NAN,
        ..Tuning::code_defaults()
    });
    d.update_population(&profile(0, 0, 0));
    assert_eq!(host.log.borrow().as_slice(), ["population 1 0".to_string()]);
    let trace = statics.borrow().trace.clone().unwrap();
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].kind, DrawKind::R7);
}

// ---- a hostile host -----------------------------------------------------------------------------

/// Answers are plausible most of the time and hostile otherwise (odd
/// sizes, NaN and infinite slopes, inverted or off-city rectangles, random
/// tokens and IDs, missing objects, failing calls), at a per-run rate.
/// Iteration counts stay bounded (small tract sizes and lot widths), so a
/// run ends.
struct Hostile {
    g: RefCell<Lcg>,
    /// Chance of a hostile answer, in percent.
    bad: u32,
    /// The purpose last reported, so plausible buildings match it.
    purpose: Cell<u32>,
}

impl Hostile {
    fn r(&self) -> u32 {
        self.g.borrow_mut().next()
    }
    fn hostile(&self) -> bool {
        self.g.borrow_mut().below(100) < self.bad
    }
    /// `good`, or one of `bad` when this answer is hostile.
    fn h<T: Copy>(&self, good: T, bad: &[T]) -> T {
        if self.hostile() {
            self.g.borrow_mut().pick(bad)
        } else {
            good
        }
    }
    fn pick<T: Copy>(&self, v: &[T]) -> T {
        self.g.borrow_mut().pick(v)
    }
    fn b(&self, good: bool) -> bool {
        self.h(good, &[!good])
    }
    fn f(&self, good: f32) -> f32 {
        self.h(good, &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e30, -1e30, -0.0])
    }
    fn small(&self) -> i32 {
        self.pick(&[-3, -1, 0, 1, 2, 3, 4, 7, 63, 64, 300])
    }
    fn tokens(&self, max: u32, range: u32) -> Vec<u64> {
        let n = self.g.borrow_mut().below(max + 1);
        (0..n).map(|_| u64::from(self.g.borrow_mut().below(range))).collect()
    }
    fn ids(&self, max: u32) -> Vec<u32> {
        let n = self.g.borrow_mut().below(max + 1);
        (0..n).map(|_| self.r()).collect()
    }
}

impl TractDeveloperHost for Hostile {
    fn acquire_services(&self) -> bool {
        self.b(true)
    }
    fn add_agent(&self, _: u32, _: u32) -> bool {
        self.b(true)
    }
    fn acquire_message_server(&self) -> bool {
        self.b(true)
    }
    fn release_message_server(&self) {}
    fn remove_agent(&self) {}
    fn release_services(&self) {}
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        if self.b(true) {
            Some(Rc::new(Tuning::shipped_exemplar()))
        } else {
            None
        }
    }
    fn dev_target_zone(&self, _: u64) -> i32 {
        let z = self.pick(&[1, 2, 3]);
        self.h(z, &[0, 7, -1, i32::MAX])
    }
    fn dev_subtype(&self, _: u64) -> u32 {
        self.r()
    }
    fn dev_purpose(&self, _: u64) -> u32 {
        let p = self.pick(&[1, 2, 5, 7]);
        let p = self.h(p, &[0, 9, u32::MAX]);
        self.purpose.set(p);
        p
    }
    fn dev_wealth(&self, _: u64) -> u32 {
        self.h(1, &[0, 2, 4, u32::MAX])
    }
    fn dev_construction_info(&self, _: u64) -> ConstructionProfile {
        let zone = self.h(1, &[0, 9, -1]);
        let land_value_min = self.h(0, &[10, 255]);
        let land_value_max = self.h(0, &[3, 255]);
        ConstructionProfile {
            zone,
            land_value_min,
            land_value_max,
            ..ConstructionProfile::default()
        }
    }
    fn dev_desirability_tract_size(&self, _: u64) -> i32 {
        self.h(4, &[-4, -1, 0, 1, 5])
    }
    fn dev_desirability_shift(&self, _: u64) -> u32 {
        self.h(2, &[0, 31, 32, 200, u32::MAX])
    }
    fn dev_desirability_value(&self, _: u64, _: i32, _: i32) -> i16 {
        self.r() as i16
    }
    fn dev_capacity_value(&self, _: u64, _: i32, _: i32) -> u32 {
        self.r()
    }
    fn dev_source_demand(&self, _: u64) -> Option<f32> {
        let d = self.g.borrow_mut().below(40) as f32;
        let d = self.f(d);
        self.h(Some(d), &[None])
    }
    fn dev_cap(&self, _: u64, _: Cap) -> u8 {
        let v = self.r() as u8;
        self.h(255, &[0, 1, v])
    }
    fn dev_monopoly_use(&self, _: u64) -> i32 {
        self.h(-1, &[0, 1, 2, 7])
    }
    fn dev_style_percentages(&self, _: u64) -> Vec<(u32, f32)> {
        if self.hostile() {
            let n = self.g.borrow_mut().below(4);
            (0..n).map(|_| (self.r(), self.f(f32::NAN))).collect()
        } else if self.r() & 1 == 0 {
            vec![(0x2000, 0.5), (0x2001, 0.5)]
        } else {
            Vec::new()
        }
    }
    fn building_profile(&self, _: u32) -> BuildingProfile {
        let purpose = self.h(self.purpose.get(), &[0, 5, 9]);
        let wealth = self.h(1, &[0, 2, 4]);
        let cap = self.pick(&[1, 2, 5]);
        let capacity = self.h(cap, &[0, u16::MAX]);
        BuildingProfile::basic(purpose as u8, wealth as u8, capacity)
    }
    fn building_types_by_family(&self, _: u32) -> Vec<u32> {
        if self.hostile() {
            self.ids(6)
        } else {
            vec![0x300, 0x301, 0x302]
        }
    }
    fn building_in_occupant_group(&self, _: u32, _: u32) -> bool {
        self.b(true)
    }
    fn building_is_monopoly(&self, _: u32) -> bool {
        self.b(false)
    }
    fn building_field_ids(&self, _: u32) -> Option<Vec<u32>> {
        if !self.hostile() {
            Some(vec![0x401, 0x402, 0x403])
        } else if self.r() & 1 == 0 {
            None
        } else {
            Some(self.ids(2))
        }
    }
    fn developer_for(&self, _: u32, _: u32) -> Option<u64> {
        self.h(Some(1), &[None])
    }
    fn claim_construction_resources(&self, _: u16) -> bool {
        self.b(true)
    }
    fn release_construction_resources(&self, _: u16) {}
    fn cheat(&self, _: Cheat) -> bool {
        self.b(false)
    }
    fn needs_anchor_building(&self, _: CellRect, _: u32) -> bool {
        self.b(false)
    }
    fn start_construction(&self, _: u64, _: u64, _: u32, _: bool) -> bool {
        self.b(true)
    }
    fn abandon_building(&self, _: u64, _: u32) -> bool {
        self.b(true)
    }
    fn reoccupy_building(&self, _: u64, _: u32) -> bool {
        self.b(true)
    }
    fn area_slope(&self, _: CellRect) -> f32 {
        self.f(0.0)
    }
    fn get_lot(&self, _: i32, _: i32) -> Option<u64> {
        let t = u64::from(self.g.borrow_mut().below(8));
        if self.r() % 3 == 0 {
            None
        } else {
            Some(t)
        }
    }
    fn lots_in_cell_rect(&self, _: CellRect, f: &mut dyn FnMut(u64)) {
        for t in self.tokens(5, 8) {
            f(t);
        }
    }
    fn lots_surrounding_lot(&self, _: u64, _: u32) -> Vec<u64> {
        self.tokens(6, 60)
    }
    fn lots_surrounding_rect(&self, _: CellRect, _: u32) -> Vec<u64> {
        self.tokens(8, 60)
    }
    fn can_create_lot(&self, _: i32, _: i32, _: i32, _: i32) -> i32 {
        let ok = self.pick(&[7, 8]);
        self.h(ok, &[0, 1, 2, 3, -1, 99])
    }
    fn create_lot_if_possible(&self, _: i32, _: i32, _: i32, _: i32, _: i32) -> Option<u64> {
        let t = u64::from(self.r());
        self.h(Some(t), &[None])
    }
    fn delete_lot(&self, _: u64) -> bool {
        self.b(true)
    }
    fn lot_bounding_rect(&self, _: u64) -> CellRect {
        let bad = CellRect::new(self.small(), self.small(), self.small(), self.small());
        self.h(CellRect::new(1, 1, 1, 1), &[bad, CellRect::new(5, 5, -5, -5)])
    }
    fn lot_slope(&self, _: u64) -> f32 {
        self.f(0.0)
    }
    fn lot_location(&self, _: u64) -> (i32, i32) {
        let good = (self.pick(&[0, 1, 2, 3]), self.pick(&[0, 1, 2, 3]));
        let bad = (self.small(), self.small());
        self.h(good, &[bad, (i32::MIN, i32::MAX)])
    }
    fn lot_size(&self, _: u64) -> (u32, u32) {
        let good = self.pick(&[(1, 1), (1, 1), (2, 1), (4, 1)]);
        self.h(good, &[(0, 0), (6, 3), (3, 0), (255, 255)])
    }
    fn lot_facing(&self, _: u64) -> i32 {
        let f = self.pick(&[0, 1, 2, 3]);
        self.h(f, &[-1, 4, i32::MIN])
    }
    fn lot_set_facing(&self, _: u64, _: i32) {}
    fn lot_has_road_access(&self, _: u64) -> bool {
        self.b(true)
    }
    fn lot_has_organic_road_access(&self, _: u64, _: u32) -> bool {
        self.b(true)
    }
    fn lot_zone_type(&self, _: u64) -> i32 {
        let z = self.pick(&[1, 4, 7]);
        self.h(z, &[0, 12, -1])
    }
    fn lot_state(&self, _: u64) -> u32 {
        let s = self.pick(&[1, 1, 3, 4]);
        self.h(s, &[0, 2, 5, 6, u32::MAX])
    }
    fn lot_set_state(&self, _: u64, _: u32, _: u32, _: u32) {}
    fn lot_is_livable(&self, _: u64) -> bool {
        self.b(false)
    }
    fn lot_has_capacity(&self, _: u64) -> bool {
        self.r() & 1 == 0
    }
    fn lot_days_since_state_change(&self, _: u64) -> u32 {
        let d = self.r();
        self.h(200, &[0, 59, d])
    }
    fn lot_occupant_wealth(&self, _: u64) -> u32 {
        self.h(1, &[0, 2, 3])
    }
    fn lot_is_historical(&self, _: u64) -> bool {
        self.b(false)
    }
    fn lot_building_type(&self, _: u64) -> u32 {
        let t = self.r();
        self.h(0x300, &[0, t])
    }
    fn lot_capacity(&self, _: u64, _: u32, _: bool) -> u16 {
        let c = self.pick(&[1, 5, 10]);
        self.h(c, &[0, u16::MAX])
    }
    fn lot_available_capacity(&self, _: u64, _: u32, _: bool) -> u16 {
        let c = self.pick(&[0, 1, 5, 10]);
        self.h(c, &[u16::MAX])
    }
    fn lot_set_population(&self, _: u64, _: u32, _: u16) {}
    fn lot_configuration_stage(&self, _: u64) -> Option<u8> {
        self.h(Some(0), &[None, Some(1), Some(255)])
    }
    fn cfg_size(&self, _: u64) -> (u32, u32) {
        let good = self.pick(&[(1, 1), (1, 1), (2, 1), (2, 2)]);
        self.h(good, &[(0, 0), (0, 1), (255, 255), (3, 1)])
    }
    fn cfg_min_slope(&self, _: u64) -> f32 {
        self.f(-100.0)
    }
    fn cfg_max_slope(&self, _: u64) -> f32 {
        self.f(100.0)
    }
    fn cfg_growth_stage(&self, _: u64) -> u8 {
        self.h(1, &[0, 3, 255])
    }
    fn cfg_min_capacity(&self, _: u64) -> u16 {
        self.h(0, &[1, u16::MAX])
    }
    fn cfg_max_capacity(&self, _: u64) -> u16 {
        self.h(u16::MAX, &[0, 10])
    }
    fn cfg_compatible_zone(&self, _: u64, _: i32) -> bool {
        self.b(true)
    }
    fn cfg_compatible_wealth(&self, _: u64, _: u32) -> bool {
        self.b(true)
    }
    fn cfg_compatible_purpose(&self, _: u64, _: u32) -> bool {
        self.b(true)
    }
    fn cfg_compatible_style(&self, _: u64, _: u32) -> bool {
        self.b(true)
    }
    fn cfg_required_roads(&self, _: u64) -> Vec<bool> {
        if self.hostile() {
            let n = self.g.borrow_mut().below(7);
            (0..n).map(|_| self.r() & 1 == 1).collect()
        } else {
            Vec::new()
        }
    }
    fn cfg_primary_building_ids(&self, _: u64) -> Option<Vec<u32>> {
        if !self.hostile() {
            Some(vec![600])
        } else if self.r() & 1 == 0 {
            None
        } else {
            Some(Vec::new())
        }
    }
    fn get_lot_configuration(&self, _: u32) -> Option<u64> {
        self.h(Some(100), &[None])
    }
    fn register_configuration_filter(&self, _: u32, _: u32) -> u64 {
        u64::from(self.r())
    }
    fn unregister_configuration_filter(&self, _: u64) {}
    fn configurations_by_filter(&self, _: u64) -> Vec<u64> {
        if self.hostile() {
            self.tokens(4, 8)
        } else {
            vec![100, 101, 102]
        }
    }
    fn zone_at(&self, _: i32, _: i32) -> i32 {
        self.h(1, &[0, 2, 9, -1])
    }
    fn land_value(&self, _: i32, _: i32) -> u8 {
        self.r() as u8
    }
    fn powered_value(&self, _: i32, _: i32) -> i32 {
        self.h(2, &[0, -1])
    }
    fn is_cell_watered(&self, _: i32, _: i32) -> bool {
        self.b(true)
    }
    fn fire_coverage(&self, _: i32, _: i32) -> u8 {
        self.h(1, &[0])
    }
    fn park_value(&self, _: i32, _: i32) -> i16 {
        self.h(1, &[0, -1])
    }
    fn road_access_value(&self, _: i32, _: i32) -> i32 {
        self.h(1, &[0])
    }
    fn city_cell_counts(&self) -> (i32, i32) {
        self.h((64, 64), &[(0, 0), (i32::MIN, i32::MAX), (1, 300)])
    }
    fn find_network_tile(&self, _: i32, _: i32) -> Option<u64> {
        let t = u64::from(self.r());
        if self.r() % 4 == 0 {
            Some(t)
        } else {
            None
        }
    }
    fn network_is_of_type(&self, _: u64, _: u32) -> bool {
        self.r() & 1 == 1
    }
    fn demolish_lot(&self, _: u64) -> bool {
        self.b(true)
    }
    fn redisplay_terrain(&self, _: CellRect) {}
}

#[test]
fn hostile_hosts_never_panic() {
    let mut kinds = std::collections::BTreeSet::new();
    let mut outcomes = std::collections::BTreeSet::new();
    for seed in 0..400u64 {
        let host = Rc::new(Hostile {
            g: RefCell::new(Lcg(seed)),
            bad: [2, 5, 15, 40][(seed % 4) as usize],
            purpose: Cell::new(1),
        });
        let statics = TractDeveloperStatics::new(seed as u32, &SEEDS);
        statics.borrow_mut().trace = Some(Vec::new());
        let mut d = TractDeveloper::new(host.clone(), statics.clone());
        d.set_histogram_enabled(seed % 2 == 0);
        let _ = d.init();
        let mut out = Vec::new();
        let mut flag = false;
        for i in 0..4 {
            let p = GrowthProfile {
                developer: u64::from(host.r()),
                x: host.h(0, &[-1, 7, i32::MIN, i32::MAX]),
                z: host.h(0, &[-1, 7, i32::MIN, i32::MAX]),
                stage: host.h(3 - i, &[0, -1, i32::MAX]),
            };
            let _ = d.grow(&p, &mut out, &mut flag);
            let _ = d.decline(&p);
            d.update_population(&p);
            let _ = d.do_message(MSG_YEAR);
        }
        let _ = d.shutdown();
        for draw in statics.borrow().trace.iter().flatten() {
            kinds.insert(draw.kind.name());
        }
        outcomes.extend(out.iter().map(|o| *o as u32));
    }
    // The runs reach deep into Grow: every core draw kind and a spread of
    // outcome codes occur.
    for k in ["R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8"] {
        assert!(kinds.contains(k), "{k} never drawn: {kinds:?}, outcomes {outcomes:?}");
    }
    assert!(outcomes.len() >= 8, "{outcomes:?}");
}
