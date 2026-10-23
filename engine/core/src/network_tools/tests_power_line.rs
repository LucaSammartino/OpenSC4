// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance tests for the power-line tool (network-tools.md §9,
//! network-geometry.md §11).

use super::power_line::*;

fn line(n: i32) -> Vec<(i32, i32)> {
    (0..n).map(|x| (x, 7)).collect()
}

fn poles(n: i32) -> Vec<PoleInfo> {
    (0..n)
        .map(|i| PoleInfo {
            x: i * 4,
            z: 7,
            ..PoleInfo::default()
        })
        .collect()
}

fn reserved(kinds: &[u32], uninit: u32) -> (u32, u32) {
    let cells = line(kinds.len() as i32);
    let mut f = 0;
    let v = check_for_reserved_tiles(&cells, |x, _| kinds[x as usize], uninit, &mut f);
    (v, f)
}

#[test]
fn reserved_tiles_clear_run() {
    assert_eq!(reserved(&[0, 0, 0, 0], 0), (4, 0));
    assert_eq!(reserved(&[], 0), (0, 0));
}

#[test]
fn reserved_tiles_building_cuts_before_cell() {
    assert_eq!(reserved(&[0, 0, 2, 0, 0], 0), (2, FAIL_BUILDING));
    assert_eq!(reserved(&[2, 0, 0], 0), (0, FAIL_BUILDING));
}

#[test]
fn reserved_tiles_zoned_does_not_stop() {
    assert_eq!(reserved(&[0, 1, 1, 0], 0), (4, FAIL_ZONED));
    // A zoned last cell leaves `last` at the previous clear cell.
    assert_eq!(reserved(&[0, 0, 1], 0), (2, FAIL_ZONED));
    // All zoned: `last` keeps its start value count − 1.
    assert_eq!(reserved(&[1, 1, 1], 0), (3, FAIL_ZONED));
}

#[test]
fn reserved_tiles_road_crossed_in_the_middle() {
    assert_eq!(reserved(&[0, 3, 0], 0), (3, FAIL_ROAD));
    // Road last after clear: stops; `last` is the clear cell before it.
    assert_eq!(reserved(&[0, 0, 3], 0), (2, FAIL_ROAD));
    // Road last after a zoned cell: no stop, `last` unchanged.
    assert_eq!(reserved(&[0, 1, 3], 0), (1, FAIL_ROAD));
    // Road after road: no stop.
    assert_eq!(reserved(&[3, 3], 0), (2, FAIL_ROAD));
}

#[test]
fn reserved_tiles_one_cell_road_depends_on_uninitialised_local() {
    // NT20: only the previous=clear path explicitly truncates to current-1.
    assert_eq!(reserved(&[3], 0), (0, FAIL_ROAD));
    assert_eq!(reserved(&[3], 7), (1, FAIL_ROAD));
}

#[test]
fn reserved_tiles_building_after_zoned_keeps_later_code() {
    assert_eq!(reserved(&[1, 0, 2], 0), (2, FAIL_BUILDING));
}

#[test]
fn pl_r3_reserved_unknown_result_updates_previous_without_updating_last() {
    assert_eq!(reserved(&[0, 4, 3], 0), (1, FAIL_ROAD));
}

#[test]
fn cost_of_solution_shipped_values() {
    let p = PowerLineParams::load(
        |id| match id {
            PROP_3A4 => Some(10),
            PROP_COST_PER_POLE => Some(15),
            _ => None,
        },
        |id| match id {
            PROP_3A8 => Some(7.0),
            PROP_COST_PER_WIRE => Some(2.0),
            _ => None,
        },
    );
    assert_eq!(
        p,
        PowerLineParams {
            v_3a4: 10,
            v_3a8: 7.0,
            cost_per_pole: 15,
            cost_per_wire: 2.0
        }
    );
    let mut t = PowerLineTool::new(p);
    t.wire_cells = 12;
    t.poles = poles(1);
    assert_eq!(t.cost_of_solution(), 0);
    t.poles = poles(4);
    assert_eq!(t.cost_of_solution(), 24 + 60);
    // network-geometry §18.18: exactly the public conformance vector.
    t.wire_cells = 20;
    t.poles = poles(3);
    assert_eq!(t.cost_of_solution(), 85);
    assert_eq!(t.counted_poles, 3);
    assert_eq!(PowerLineParams::default().v_3a4, 20);
}

#[test]
fn defaults_are_float_clearance_and_zero_costs() {
    assert_eq!(
        PowerLineParams::load(|_| None, |_| None),
        PowerLineParams {
            v_3a4: 20,
            v_3a8: 7.0,
            cost_per_pole: 0,
            cost_per_wire: 0.0
        }
    );
}

struct World {
    existing: Vec<(i32, i32, u32)>,
    made: u32,
}

impl PoleWorld for World {
    type Pole = (i32, i32, u32);
    fn find_pole(&self, x: i32, z: i32) -> Option<Self::Pole> {
        self.existing.iter().copied().find(|p| p.0 == x && p.1 == z)
    }
    fn new_pole(&mut self, x: i32, z: i32, _exemplar: u32, index: u32) -> Self::Pole {
        self.made += 1;
        (x, z, index)
    }
}

#[test]
fn create_power_pole_reuses_existing() {
    let mut w = World {
        existing: vec![(4, 4, 9)],
        made: 0,
    };
    assert_eq!(create_power_pole(&mut w, 4, 4, 0x1234, 2), (4, 4, 9));
    assert_eq!(w.made, 0);
    assert_eq!(create_power_pole(&mut w, 5, 4, 0x1234, 2), (5, 4, 2));
    assert_eq!(w.made, 1);
}
