// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance tests for the power-line tool (network-tools.md §9,
//! network-geometry.md §11).

use super::host::ToolRun;
use super::power_line::*;
use super::state::ToolState;

const POWER: u32 = 5;

fn line(n: i32) -> Vec<(i32, i32)> {
    (0..n).map(|x| (x, 7)).collect()
}

/// Records the hook calls; poles and failure are scripted.
#[derive(Default)]
struct Host {
    calls: Vec<&'static str>,
    poles: Vec<PoleInfo>,
    wire: u32,
    fail: u32,
    lower_valid: Option<u32>,
    attempt_ok: bool,
}

impl PowerLineHost for Host {
    fn determine_pole_positions(&mut self, run: &mut ToolRun, tool: &mut PowerLineTool, failure: &mut u32) {
        self.calls.push("determine");
        tool.poles = self.poles.clone();
        tool.wire_cells = self.wire;
        if self.fail != 0 {
            *failure = self.fail;
        }
        if let Some(v) = self.lower_valid {
            run.valid_count = v;
        }
    }
    fn place_poles(&mut self, _run: &mut ToolRun, _tool: &mut PowerLineTool, arg: u32) {
        assert_eq!(arg, 1);
        self.calls.push("place_poles");
    }
    fn attempt_neighbor_connections(&mut self, _run: &mut ToolRun, _tool: &mut PowerLineTool) -> bool {
        self.calls.push("attempt");
        self.attempt_ok
    }
    fn place_neighbor_connections(&mut self, _run: &mut ToolRun, _tool: &mut PowerLineTool) {
        self.calls.push("place_neighbors");
    }
    fn refresh_preview(&mut self, arg: u32) {
        assert_eq!(arg, 0);
        self.calls.push("preview");
    }
    fn release_pole_infos(&mut self, tool: &mut PowerLineTool) {
        self.calls.push("release");
        tool.poles.clear();
    }
}

fn run(n: u32) -> ToolRun {
    let mut r = ToolRun::new(POWER, (0, 7), (n as i32 - 1, 7));
    r.dragged_count = n;
    r.cells = line(n as i32);
    r
}

fn poles(n: i32) -> Vec<PoleInfo> {
    (0..n).map(|i| PoleInfo { x: i * 4, z: 7 }).collect()
}

fn place(h: &mut Host, r: &mut ToolRun, nm: u32, real: bool) -> (bool, ToolState) {
    let mut st = ToolState::new(POWER);
    let mut tool = PowerLineTool::new(PowerLineParams::default());
    tool.poles = poles(9); // stale lists are cleared at step 1
    tool.anchors = vec![r.start, r.end];
    tool.lines = vec![(1, 1)];
    let ok = do_network_specific_placement(&mut st, &mut tool, h, r, nm, real);
    assert!(tool.lines.is_empty());
    (ok, st)
}

#[test]
fn real_drag_with_neighbours_places_both() {
    let mut h = Host {
        poles: poles(3),
        wire: 9,
        attempt_ok: true,
        ..Host::default()
    };
    let mut r = run(9);
    let (ok, _) = place(&mut h, &mut r, 1, true);
    assert!(ok);
    assert_eq!(r.valid_count, 9);
    assert_eq!(
        h.calls,
        ["release", "determine", "place_poles", "attempt", "place_neighbors", "release"]
    );
}

#[test]
fn failed_attempt_does_not_place_neighbors() {
    let mut h = Host {
        poles: poles(2),
        attempt_ok: false,
        ..Host::default()
    };
    let mut r = run(5);
    place(&mut h, &mut r, 1, true);
    assert_eq!(
        h.calls,
        ["release", "determine", "place_poles", "attempt", "release"]
    );
}

#[test]
fn draft_attempts_but_does_not_place_and_refreshes() {
    let mut h = Host {
        poles: poles(2),
        ..Host::default()
    };
    let mut r = run(5);
    place(&mut h, &mut r, 1, false);
    assert_eq!(h.calls, ["release", "determine", "place_poles", "attempt", "preview", "release"]);
}

#[test]
fn no_neighbour_mode_no_regional() {
    let mut h = Host {
        poles: poles(2),
        ..Host::default()
    };
    let mut r = run(5);
    place(&mut h, &mut r, 0, true);
    assert_eq!(h.calls, ["release", "determine", "place_poles", "release"]);
}

#[test]
fn one_step_drag_skips_poles_and_is_invalid() {
    let mut h = Host {
        poles: poles(2),
        ..Host::default()
    };
    let mut r = run(1);
    let (ok, _) = place(&mut h, &mut r, 1, false);
    assert!(ok);
    // No poles were determined: the span is under 48 units → valid 0.
    assert_eq!(r.valid_count, 0);
    assert_eq!(h.calls, ["release", "preview", "release"]);
}

#[test]
fn no_pole_record_means_valid_zero() {
    let mut h = Host {
        poles: vec![],
        ..Host::default()
    };
    let mut r = run(6);
    place(&mut h, &mut r, 1, true);
    assert_eq!(r.valid_count, 0);
    assert_eq!(h.calls, ["release", "determine", "place_poles", "release"]);
}

#[test]
fn single_pole_with_two_input_anchors_still_attempts_regional() {
    let mut h = Host {
        poles: poles(1),
        ..Host::default()
    };
    let mut r = run(6);
    place(&mut h, &mut r, 1, true);
    assert_eq!(r.valid_count, 6);
    assert_eq!(h.calls, ["release", "determine", "place_poles", "attempt", "release"]);
}

#[test]
fn failure_or_partial_drag_blocks_regional() {
    let mut h = Host {
        poles: poles(3),
        fail: FAIL_ZONED,
        ..Host::default()
    };
    let mut r = run(6);
    let (_, st) = place(&mut h, &mut r, 1, true);
    assert_eq!(st.failure, FAIL_ZONED);
    assert!(!h.calls.contains(&"attempt"));

    let mut h = Host {
        poles: poles(3),
        lower_valid: Some(4),
        ..Host::default()
    };
    let mut r = run(6);
    place(&mut h, &mut r, 1, true);
    assert_eq!(r.valid_count, 4);
    assert!(!h.calls.contains(&"attempt"));
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
    // [quirk] either way the valid count is 1 (last starts at 0); only the
    // code differs from nothing.
    assert_eq!(reserved(&[3], 0), (1, FAIL_ROAD));
    assert_eq!(reserved(&[3], 7), (1, FAIL_ROAD));
}

#[test]
fn reserved_tiles_building_after_zoned_keeps_later_code() {
    assert_eq!(reserved(&[1, 0, 2], 0), (2, FAIL_BUILDING));
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
    assert_eq!(PowerLineParams::default().v_3a4, 20);
}

#[test]
fn pole_position_and_guard() {
    assert_eq!(compute_pole_position(3, 10, |_, _| 270.5), Some([56.0, 270.5, 168.0]));
    assert_eq!(compute_pole_position(0, 0, |_, _| -f32::MAX), None);
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
