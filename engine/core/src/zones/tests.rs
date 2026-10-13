// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests for the pure zone helpers: the depth rule (conformance test
//! 27), the block depth and zone density (§7.2.1 step 8), the capacity value
//! and depth-range adjustment of `DetermineLotSize` (test 30g), the region helpers
//! (§7.2, §11.4) and the seed derivation (§9, foundations §6.3). The
//! parcellization, `PlaceZone`, the lot manager and the save records are
//! exercised against a scripted world by the conformance vectors
//! (`engine/conformance`, `zone_…`).

use super::parcel::DEFAULT_BLOCK_DEPTH;
use super::*;
use crate::grid::CellRect;
use crate::rng::{derive_seed, FixedSeeds, RzRandom};

// ---- depth rule (test 27) ----------------------------------------------------------------------

/// One row of test 27: results for rem = 1..12.
fn rule(ld: i32, min: i32, max: i32, ps: i32, front: bool, far: bool) -> Vec<i32> {
    (1..=12)
        .map(|rem| depth_rule(rem, front, far, min, max, ld, ps))
        .collect()
}

#[test]
fn depth_rule_test_27() {
    // Ld 2, min 1, max 3, ps 1.
    let a = rule(2, 1, 3, 1, true, true); // (a) far/front
    let b = rule(2, 1, 3, 1, false, true); // (b) far/back
    let b2 = rule(2, 1, 3, 1, true, false); // (b) near/front (same)
    let c = rule(2, 1, 3, 1, false, false); // (c) near/back
    let want_a = vec![1, 2, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2];
    let want_c = vec![2, 1, 2, 3, 2, 2, 2, 2, 2, 2, 2, 2];
    assert_eq!(a, want_a);
    assert_eq!(b, want_a);
    assert_eq!(b2, want_a);
    assert_eq!(c, want_c);
    assert_eq!(rule(2, 1, 3, 0, false, false), vec![2, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2]);

    // Ld 3, min 2, max 4, ps 1.
    assert_eq!(rule(3, 2, 4, 1, true, true), vec![1, 2, 3, 4, 3, 3, 2, 3, 3, 3, 3, 3]);
    assert_eq!(rule(3, 2, 4, 1, false, true), vec![1, 2, 3, 4, 2, 3, 3, 3, 3, 3, 3, 3]);
    assert_eq!(rule(3, 2, 4, 1, true, false), vec![1, 2, 3, 4, 2, 3, 3, 3, 3, 3, 3, 3]);
    assert_eq!(rule(3, 2, 4, 1, false, false), vec![3, 1, 2, 3, 4, 3, 3, 3, 3, 3, 3, 3]);

    // Ld 4, min 3, max 5, ps 1.
    assert_eq!(rule(4, 3, 5, 1, true, true), vec![1, 2, 3, 4, 5, 3, 4, 4, 5, 3, 4, 4]);
    assert_eq!(rule(4, 3, 5, 1, false, true), vec![1, 2, 3, 4, 5, 6, 3, 4, 4, 4, 4, 4]);
    assert_eq!(rule(4, 3, 5, 1, true, false), vec![1, 2, 3, 4, 5, 6, 3, 4, 4, 4, 4, 4]);
    assert_eq!(rule(4, 3, 5, 1, false, false), vec![4, 1, 2, 3, 4, 5, 3, 4, 4, 4, 4, 4]);
    // ps 0.
    assert_eq!(rule(4, 3, 5, 0, true, true), vec![1, 2, 3, 4, 5, 3, 4, 4, 3, 3, 4, 4]);
    assert_eq!(rule(4, 3, 5, 0, false, true), vec![1, 2, 3, 4, 5, 3, 3, 4, 4, 4, 4, 4]);
    assert_eq!(rule(4, 3, 5, 0, false, false), vec![4, 1, 2, 3, 4, 5, 3, 4, 4, 4, 4, 4]);
}

// ---- zone density and block depth (§7.2.1 step 8) ----------------------------------------------

#[test]
fn zone_density_table() {
    for (z, d) in [
        (1, 1),
        (4, 1),
        (2, 2),
        (5, 2),
        (3, 3),
        (6, 3),
        (7, 3),
        (8, 3),
        (9, 3),
        (0, 0),
        (10, 0),
        (16, 0),
    ] {
        assert_eq!(zone_density(z), d, "zone {z}");
    }
}

#[test]
fn block_depth_table() {
    // Non-residential: density 1 → 7, 2 → 6, 3 → 5.
    assert_eq!(block_depth(1, 3, false), 7);
    assert_eq!(block_depth(2, 3, false), 6);
    assert_eq!(block_depth(3, 3, false), 5);
    // Residential: 1 → 3·depth; 2 → ftol(r24(depth × 2.5)); 3 → 2·depth.
    assert_eq!(block_depth(1, 2, true), 6);
    assert_eq!(block_depth(3, 2, true), 4);
    assert_eq!(block_depth(2, 2, true), 5);
    assert_eq!(block_depth(2, 3, true), 7);
    assert_eq!(block_depth(2, 4, true), 10);
    // Density 0 leaves the default unchanged.
    assert_eq!(block_depth(0, 99, true), DEFAULT_BLOCK_DEPTH);
}

// ---- DetermineLotSize pure functions (test 30g) ------------------------------------------------

#[test]
fn capacity_value_test_30g() {
    // v(k, n, avg) = ftol(r24(r24(k / n) × avg)).
    assert_eq!(capacity_value(16, 4, 50), 200);
    assert_eq!(capacity_value(16, 6, 150), 400);
    assert_eq!(capacity_value(16, 9, 335), 595);
    // One above the exact floor (the x87 chain, not ⌊k · avg / n⌋).
    assert_eq!(capacity_value(256, 3, 49154), 4_194_475);
    assert_eq!(capacity_value(4096, 3, 3074), 4_197_035);
    assert_eq!(capacity_value(4096, 6, 6145), 4_194_987);
    assert_eq!(capacity_value(4096, 9, 4616), 2_100_793);
}

#[test]
fn adjust_depth_range_test_30g() {
    for (lo, hi) in [(2, 2), (3, 3), (1, 2), (2, 3), (1, 1)] {
        assert_eq!(adjust_depth_range(lo, hi), (1, 3), "[{lo}, {hi}]");
    }
    assert_eq!(adjust_depth_range(3, 4), (2, 4));
    assert_eq!(adjust_depth_range(4, 4), (2, 4));
    assert_eq!(adjust_depth_range(5, 6), (4, 6));
    assert_eq!(adjust_depth_range(1, 3), (1, 3));
    assert_eq!(adjust_depth_range(1, 4), (1, 4));
    // Residential density-2 block: depth 1, 2, 3, 4 → 2, 5, 7, 10.
    let blocks: Vec<i32> = (1..=4).map(|d| block_depth(2, d, true)).collect();
    assert_eq!(blocks, vec![2, 5, 7, 10]);
}

// ---- middle row (test 29) ----------------------------------------------------------------------

#[test]
fn middle_row_test_29() {
    // o0 = 10, row step +2 (low side Q ∈ {0,3}, so = +lw = +2): range 10..15
    // → mid 10; 10..17 → 12; 10..21 → 14; 10..11 → 10.
    assert_eq!(middle_row(10, 2, 10, 15, 2), 10);
    assert_eq!(middle_row(10, 2, 10, 17, 2), 12);
    assert_eq!(middle_row(10, 2, 10, 21, 2), 14);
    assert_eq!(middle_row(10, 2, 10, 11, 2), 10);
}

// ---- region helpers (§7.2, §11.4) --------------------------------------------------------------

#[test]
fn region_compress_and_components() {
    // A 5 × 5 region with cells only in a 2 × 2 corner: compress crops to the
    // bounding box.
    let rect = CellRect::new(0, 0, 4, 4);
    let r = CellRegion::from_cells(rect, &[(2, 3), (3, 3), (2, 4), (3, 4)]);
    assert_eq!(r.count(), 4);
    let c = r.compress();
    assert_eq!(c.rect, CellRect::new(2, 3, 3, 4));
    assert_eq!(c.count(), 4);

    // Two 4-connected components, seeded x-outer, z-inner.
    let two = CellRegion::from_cells(rect, &[(0, 0), (0, 1), (4, 4)]);
    let comps = two.components();
    assert_eq!(comps.len(), 2);
    assert_eq!(comps[0].rect, CellRect::new(0, 0, 0, 1));
    assert_eq!(comps[0].count(), 2);
    assert_eq!(comps[1].rect, CellRect::new(4, 4, 4, 4));

    // An empty region has no components (the caller adds the empty region).
    assert!(CellRegion::new(rect, false).components().is_empty());
    assert_eq!(CellRegion::empty().count(), 0);
}

#[test]
fn region_includes_and_bounds() {
    let r = CellRegion::new(CellRect::new(1, 2, 3, 4), true);
    assert!(r.includes(1, 2) && r.includes(3, 4));
    assert!(!r.includes(0, 2) && !r.includes(4, 4) && !r.includes(1, 5));
    assert_eq!(r.count(), 9);
    // MAX_REGION_SIDE clamps a hostile rectangle.
    let big = CellRegion::new(CellRect::new(0, 0, i32::MAX, i32::MAX), true);
    assert!(big.width() <= MAX_REGION_SIDE as i32);
}

// ---- seed derivation (§9, foundations §6.3) ----------------------------------------------------

#[test]
fn session_seed_matches_the_tract_harness() {
    // Session 0 gives seed 0x8C09C68A (the same the tract harness uses for
    // the shared zone developer generator), and the seeded U(4) sequence is
    // 3, 0, 3, 0, … (§9 "Seeded test").
    let seeds = FixedSeeds {
        wall_clock: 0,
        process_clock: 0,
    };
    assert_eq!(derive_seed(0, RNG_SEED_XOR), 0x8C09_C68A);
    let st = ZoneDeveloperStatics::from_session(0, &seeds);
    assert_eq!(st.borrow().rng.lcg_state(), 0x8C09_C68A);
    let mut draws = Vec::new();
    for _ in 0..4 {
        let v = st.borrow_mut().uniform(DrawKind::Z2, 4);
        draws.push(v);
    }
    assert_eq!(draws, vec![3, 0, 3, 0]);
    // The first NextRaw of a cRZRandom with that seed is 0xF5DF3363 (§9).
    let mut probe = RzRandom::new(0x8C09_C68A, &seeds);
    assert_eq!(probe.next_raw(), 0xF5DF_3363);
}

#[test]
fn create_lot_if_possible_facing_minus_one_draws_once() {
    // §9 Z2c: a facing-−1 attempt makes exactly one U(4) draw on the shared
    // statics. Verified here at the statics level (the draw the core makes);
    // the full lot-creation path is a conformance vector.
    let seeds = FixedSeeds {
        wall_clock: 0,
        process_clock: 0,
    };
    let st = ZoneDeveloperStatics::from_session(0, &seeds);
    st.borrow_mut().trace = Some(Vec::new());
    let before = st.borrow().rng.lcg_state();
    let _ = st.borrow_mut().uniform(DrawKind::Z2, 4);
    let after = st.borrow().rng.lcg_state();
    // Exactly one advance.
    let mut probe = RzRandom::new(before, &seeds);
    probe.next_raw();
    assert_eq!(after, probe.lcg_state());
    let trace = st.borrow().trace.clone().unwrap();
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].kind, DrawKind::Z2);
    assert_eq!(trace[0].arg, 4);
}

// ---- tuning (§4) -------------------------------------------------------------------------------

#[test]
fn shipped_tuning_reads_the_spec_values() {
    let h = shipped_tuning();
    let mut t = Tuning::default();
    t.read(Some(&h));
    assert_eq!(t.min_size[7], 4);
    assert_eq!(t.max_size[0], 32768);
    assert_eq!(t.create_cost[1], 10);
    assert_eq!(t.destroy_cost[14], 1);
    assert_eq!(t.texture[0], 4121);
    assert_eq!(t.drag_color[1], 12845311);
    assert_eq!(t.display_alpha, 170);
    assert_eq!(t.max_slope[0], 256.0);
    assert_eq!(t.max_slope[1], 15.0);
}

#[test]
fn is_rci_zone_table() {
    assert!(!is_rci_zone(0));
    for t in 1..=9 {
        assert!(is_rci_zone(t), "zone {t}");
    }
    for t in [10, 11, 15, 16, -1] {
        assert!(!is_rci_zone(t), "zone {t}");
    }
}
