// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests of the host-free parts: the pure helpers, the catalogue and
//! the process statics. The simulator itself is
//! exercised through the conformance vectors (`bd11_*`).

use super::*;
use crate::rng::FixedSeeds;
use crate::variant::{mem, Elem, Variant};

fn seeds() -> FixedSeeds {
    FixedSeeds {
        wall_clock: 1,
        process_clock: 1,
    }
}

fn key(group: u32, instance: u32) -> ResKey {
    ResKey {
        type_id: 0x6534_284A,
        group,
        instance,
    }
}

fn building(
    purpose: u8,
    wealth: u8,
    capacity: Option<u32>,
    families: &[u32],
    size: Option<[f32; 3]>,
) -> PropertyHolder {
    let mut h = PropertyHolder::new();
    h.add(prop::EXEMPLAR_TYPE, Variant::Scalar(Elem::U32(2)));
    h.add(0x2781_2833, Variant::Scalar(Elem::U8(purpose)));
    h.add(0x2781_2832, Variant::Scalar(Elem::U8(wealth)));
    if let Some(c) = capacity {
        let dt = crate::development::developer_type(u32::from(purpose), u32::from(wealth));
        h.add(
            0x2781_2834,
            Variant::array(mem::UINT32, vec![Elem::U32(dt), Elem::U32(c)]),
        );
    }
    if !families.is_empty() {
        h.add(
            0x2781_2870,
            Variant::array(mem::UINT32, families.iter().map(|&f| Elem::U32(f)).collect()),
        );
    }
    if let Some(s) = size {
        h.add(
            0x2781_2810,
            Variant::array(mem::FLOAT32, s.iter().map(|&x| Elem::F32(x)).collect()),
        );
    }
    h
}

fn config(size: (u32, u32), zones: &[u32], ids: &[u32], orientation: u32, pos: Option<(f32, f32)>) -> LotConfigInfo {
    let (x, z) = pos.unwrap_or((size.0 as f32 * 8.0, size.1 as f32 * 8.0));
    LotConfigInfo {
        size,
        zones: zones.iter().fold(0u16, |m, &z| m | (1 << z)),
        primary: Some(BuildingConfigObject {
            ids: ids.to_vec(),
            orientation,
            x,
            z,
        }),
    }
}

// ---- pure helpers -------------------------------------------------------------------------------------

#[test]
fn terrain_rect_unpacking_clamps() {
    assert_eq!(unpack_terrain_rect(0x0003_0005, 0x0000_0040, (64, 64)), (2, 0, 5, 63));
    assert_eq!(unpack_terrain_rect(0, 0x0001_FFFF, (64, 64)), (0, 0, 0, 63));
    assert_eq!(
        unpack_terrain_rect(0xFFFF_FFFF, 0xFFFF_FFFF, (0, 0)),
        (0xFFFE, 0xFFFE, -1, -1)
    );
}

#[test]
fn monthly_supply_matches_test_4() {
    // §11 test 4: x = 25500.375 -> clamped to 20480.
    assert_eq!(monthly_supply(30000.0, 12000.5, 9000.25, 0.5, 8192, 20480), 20480.0);
    // Below the minimum.
    assert_eq!(monthly_supply(100.0, 0.0, 0.0, 0.5, 8192, 20480), 8192.0);
    // Inside: trunc(0.5 * 20001) = 10000.
    assert_eq!(monthly_supply(20001.0, 0.0, 0.0, 0.5, 8192, 20480), 10000.0);
    // NaN -> 0 -> crMin; negative -> huge unsigned -> crMax (unsigned clamps).
    assert_eq!(monthly_supply(f32::NAN, 0.0, 0.0, 0.5, 8192, 20480), 8192.0);
    assert_eq!(monthly_supply(-100.0, 0.0, 0.0, 0.5, 8192, 20480), 20480.0);
}

#[test]
fn update_lot_elements_flag_is_the_low_byte() {
    assert!(!update_lot_elements_flag(0));
    assert!(update_lot_elements_flag(1));
    assert!(update_lot_elements_flag(3));
    assert!(update_lot_elements_flag(7));
    assert!(!update_lot_elements_flag(0x100));
    assert!(update_lot_elements_flag(0x101));
    assert!(!update_lot_elements_flag(0xFFFF_FF00));
}

#[test]
fn grow_budget_defaults_to_unlimited() {
    assert_eq!(DEFAULT_GROW_ATTEMPT_BUDGET, u32::MAX);
}

#[test]
fn process_seed_is_injected_and_reseedable() {
    let a = BdsProcess::new(12345, &seeds());
    let first: Vec<u32> = (0..3).map(|_| a.uniform(420)).collect();
    assert_eq!(first[0], 20);
    assert_eq!(a.steps(), 3);
    // Re-seeding restarts the same sequence; the step count is kept.
    a.reseed(12345, &seeds());
    assert_eq!(a.rng_state(), 12345);
    let again: Vec<u32> = (0..3).map(|_| a.uniform(420)).collect();
    assert_eq!(first, again);
    assert_eq!(a.steps(), 6);
    // A different injected seed gives a different stream (seed 1: 0 of 420).
    let b = BdsProcess::new(1, &seeds());
    assert_eq!(b.uniform(420), 0);
    b.reseed(7, &seeds());
    assert_eq!(b.uniform(66), 0);
    assert_eq!(b.uniform(50), 42);
}

#[test]
fn decline_days_add_in_float32() {
    assert_eq!(decline_day(5000, 2), 5002);
    // 2^24 + 1 is not a float32: the sum rounds to even.
    assert_eq!(decline_day(1 << 24, 1), 1 << 24);
    assert_eq!(decline_day((1 << 24) + 2, 1), (1 << 24) + 4);
}

#[test]
fn tuning_min_max_reads_raw_words() {
    let mut h = PropertyHolder::new();
    assert_eq!(tuning_min_max(&h), (None, None));
    h.add(
        prop::CONSTRUCTION_MIN_MAX,
        Variant::array(mem::UINT32, vec![Elem::U32(8192), Elem::U32(20480)]),
    );
    assert_eq!(tuning_min_max(&h), (Some(8192), Some(20480)));
    h.add(
        prop::CONSTRUCTION_MIN_MAX,
        Variant::array(mem::FLOAT32, vec![Elem::F32(1.0), Elem::F32(2.0)]),
    );
    assert_eq!(tuning_min_max(&h), (Some(0x3F80_0000), Some(0x4000_0000)));
    h.add(prop::CONSTRUCTION_MIN_MAX, Variant::Scalar(Elem::U32(7)));
    assert_eq!(tuning_min_max(&h), (Some(7), None));
    h.add(prop::CONSTRUCTION_MIN_MAX, Variant::Scalar(Elem::U8(7)));
    assert_eq!(tuning_min_max(&h), (None, None));
}

#[test]
fn identity() {
    assert_eq!(query_interface(1), Some(BdsInterface::Primary));
    assert_eq!(query_interface(IID_BDS), Some(BdsInterface::Primary));
    assert_eq!(query_interface(IID_MESSAGE_TARGET), Some(BdsInterface::MessageTarget));
    assert_eq!(query_interface(IID_MESSAGE_TARGET2), Some(BdsInterface::MessageTarget2));
    assert_eq!(query_interface(IID_SERIALIZABLE), Some(BdsInterface::Serializable));
    assert_eq!(query_interface(IID_SIM_GRID), None);
    assert_eq!(clock_seed(0), 0x0C09_C0B8);
    assert_eq!(CLSID_BDS, 0x8990_BE7A);
}

// ---- process statics ----------------------------------------------------------------------------------

#[test]
fn generator_draws_and_counts_steps() {
    let p = BdsProcess::new(12345, &seeds());
    p.enable_trace();
    assert_eq!(p.uniform(420), 20);
    assert_eq!(p.uniform(0), 0);
    assert_eq!(p.uniform(310), 308);
    assert_eq!(p.uniform(50), 21);
    assert_eq!(p.steps(), 3);
    assert_eq!(p.rng_state(), 0xD665_1C2C);
    let trace = p.take_trace();
    assert_eq!(trace.len(), 4);
    assert!(!trace[1].stepped);
    assert!(p.take_trace().is_empty());
}

#[test]
fn marks_behave_like_a_map_of_sets() {
    let p = BdsProcess::new(1, &seeds());
    assert!(p.marks().is_empty());
    p.touch_marks(0x1010);
    assert_eq!(p.marks().len(), 1);
    assert!(p.marks_of(0x1010).is_empty());
    p.mark(0x1010, (2, 1));
    p.mark(0x1010, (-1, 5));
    p.mark(0x1010, (2, 1));
    assert!(p.is_marked(0x1010, (-1, 5)));
    assert!(!p.is_marked(0x3110, (-1, 5)));
    assert_eq!(
        p.marks_of(0x1010).into_iter().collect::<Vec<_>>(),
        vec![(-1, 5), (2, 1)]
    );
    p.clear_marks_of(0x1010);
    assert!(p.marks_of(0x1010).is_empty());
    assert_eq!(p.marks().len(), 1);
    p.clear_marks();
    assert!(p.marks().is_empty());
}

// ---- catalogue ------------------------------------------------------------------------------------------

#[test]
fn catalogue_family_queries() {
    let mut c = BuildingCatalogue::new();
    c.add_building(key(1, 0x100), &building(1, 1, None, &[0x7000], None), None);
    c.add_building(key(1, 0x200), &building(1, 2, None, &[0x7000], None), None);
    c.add_building(key(1, 0x2C1), &building(2, 1, None, &[0x70C1], None), None);
    assert_eq!(c.building_count(), 3);
    let mut v = vec![9, 9];
    assert_eq!(c.building_types_by_family(&mut v, 0x7000), 2);
    assert_eq!(v, vec![0x100, 0x200]);
    let mut v = vec![9];
    assert_eq!(c.building_types_by_family(&mut v, 0x100), 0);
    assert_eq!(v, vec![0x100]);
    let mut v = vec![9];
    assert_eq!(c.building_types_by_family(&mut v, 0x9999), 0);
    assert_eq!(v, vec![9]);
    let mut s: BTreeSet<u32> = [9].into_iter().collect();
    assert_eq!(c.building_types_by_family_set(&mut s, 0x7000), 2);
    assert_eq!(s.len(), 3);
    assert!(c.is_building_in_family(0x100, 0x100));
    assert!(c.is_building_in_family(0x200, 0x7000));
    assert!(!c.is_building_in_family(0x2C1, 0x7000));
    assert!(c.is_building_family_type_valid(0x100));
    assert!(!c.is_building_type_valid(0x7000));
    assert_eq!(c.all_building_types(), vec![0x100, 0x2C1, 0x200]);
    assert_eq!(c.all_family_types(), vec![0x70C1, 0x7000]);
    let only_a = |t: u32| t == 0x100;
    let mut s = BTreeSet::new();
    assert_eq!(c.building_types_by_filter(&mut s, Some(&only_a)), 1);
    assert_eq!(c.building_types_by_filter(&mut s, None), 3);
}

#[test]
fn catalogue_profiles_and_names() {
    let mut c = BuildingCatalogue::new();
    let mut h = building(1, 2, Some(12), &[], Some([16.0, 5.0, 16.0]));
    h.add(0x20, Variant::string(b"R1"));
    c.add_building(key(1, 0x100), &h, Some(b"Small House".to_vec()));
    let mut h = building(2, 7, None, &[], None);
    h.add(0x899A_FBAD, Variant::string(b"Fallback"));
    c.add_building(key(1, 0x200), &h, None);
    let p = c.profile(0x100).cloned().expect("profile");
    assert_eq!((p.purpose, p.wealth, p.capacity), (1, 2, 12));
    assert_eq!(p.occupant_size, [16.0, 5.0, 16.0]);
    assert_eq!(p.map_key(), 7);
    assert_eq!(c.profile(0x200).map(|p| p.wealth), Some(0));
    assert_eq!(c.exemplar_name(0x100), b"R1".to_vec());
    assert_eq!(c.building_name(0x100), b"Small House".to_vec());
    assert_eq!(c.building_name(0x200), b"Fallback".to_vec());
    assert_eq!(c.exemplar_name(0x200), Vec::<u8>::new());
    // Unknown types: the profile reference inserts a zero profile.
    assert!(c.profile(0x999).is_none());
    assert_eq!(c.profile_entry(0x999).map_key(), 1);
    assert!(c.profile(0x999).is_some());
    assert!(!c.is_building_type_valid(0x999));
    // A later exemplar for the same type replaces the key.
    c.add_building(key(2, 0x100), &building(3, 3, None, &[], None), None);
    assert_eq!(c.building_key(0x100).map(|k| k.group), Some(2));
    assert_eq!(c.building_count(), 2);
    c.add_foundation(key(3, 0x500));
    c.add_retaining_wall(key(4, 0x600));
    assert_eq!(c.foundation_key(0x500).map(|k| k.group), Some(3));
    assert_eq!(c.retaining_wall_key(0x600).map(|k| k.group), Some(4));
    assert!(c.building_key(0x500).is_none());
    c.app_shutdown();
    assert_eq!(c.building_count(), 0);
}

#[test]
fn density_table_matches_test_10() {
    let widths = (16.0, 16.0);
    let run = |capacity: u32, size: (u32, u32), extra: Option<u32>| {
        let mut c = BuildingCatalogue::new();
        c.add_building(
            key(1, 0x1000),
            &building(1, 1, Some(capacity), &[], Some([8.0, 5.0, 8.0])),
            None,
        );
        let mut configs = vec![config(size, &[1], &[0x1000], 0, None)];
        if let Some(cap) = extra {
            c.add_building(
                key(1, 0x1001),
                &building(1, 1, Some(cap), &[], Some([8.0, 5.0, 8.0])),
                None,
            );
            configs.push(config(size, &[1], &[0x1001], 0, None));
        }
        c.build_density_table(&configs, widths);
        c
    };
    let c = run(20, (2, 3), None);
    assert_eq!(c.density(1, 6).map(f32::to_bits), Some(0x4075_5555));
    assert_eq!(c.max_capacity(1, 1, 1), 3.0);
    assert_eq!(c.max_capacity(2, 1, 1), 0.0);
    assert_eq!(run(2, (2, 3), None).density(1, 6), Some(1.0));
    assert_eq!(run(0, (2, 3), None).density(1, 6), Some(0.5));
    assert_eq!(run(0, (2, 3), None).max_capacity(1, 1, 1), 0.0);
    assert_eq!(run(5, (1, 2), None).density(1, 6), Some(3.0));
    assert_eq!(
        run(20, (2, 3), Some(9)).density(1, 6).map(f32::to_bits),
        Some(0x4075_5555)
    );
    assert_eq!(
        run(9, (2, 3), Some(20)).density(1, 6).map(f32::to_bits),
        Some(0x4075_5555)
    );
}

#[test]
fn density_fit_check_and_orientation() {
    let mut c = BuildingCatalogue::new();
    c.add_building(
        key(1, 0x3000),
        &building(2, 1, Some(30), &[], Some([16.0, 5.0, 40.0])),
        None,
    );
    c.add_building(
        key(1, 0x6000),
        &building(0, 0, Some(50), &[], Some([8.0, 5.0, 8.0])),
        None,
    );
    let configs = [
        config((3, 2), &[5], &[0x3000], 0, Some((24.0, 16.0))),
        config((3, 2), &[4], &[0x3000], 1, Some((24.0, 16.0))),
        config((2, 3), &[7], &[0x6000], 0, None),
        config((2, 3), &[11], &[0x9999], 0, None),
        LotConfigInfo {
            size: (2, 3),
            zones: 1 << 9,
            primary: None,
        },
    ];
    c.build_density_table(&configs, (16.0, 16.0));
    let table: Vec<_> = c.density_table().iter().map(|(&k, &v)| (k, v)).collect();
    assert_eq!(table, vec![((4, 10), 5.5)]);
    assert_eq!(c.max_capacity(4, 2, 1), 5.0);
}
