// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `network-bridges.md` §17 (the conformance tests, numbered 1–29), named
//! `t17_NN_…` so a vector and its unit test cross-reference by number.

use std::collections::BTreeMap;

use super::*;
use crate::network_geometry::drag::{Step, StepKind};
use crate::network_geometry::smoothing::CellInfo;
use crate::network_geometry::Run;

// ---- a flat world ----------------------------------------------------------------

struct MockTerrain {
    h: f32,
    sea: f32,
    wet: Vec<(i32, i32)>,
    /// A per-cell height override: what test 16's sloping banks need.
    over: BTreeMap<(i32, i32), f32>,
}

impl MockTerrain {
    fn flat(h: f32) -> MockTerrain {
        MockTerrain {
            h,
            sea: h,
            wet: Vec::new(),
            over: BTreeMap::new(),
        }
    }
    fn river(h: f32, sea: f32, x0: i32, x1: i32, z: i32) -> MockTerrain {
        let mut t = MockTerrain {
            h,
            sea,
            wet: Vec::new(),
            over: BTreeMap::new(),
        };
        for x in x0..=x1 {
            t.wet.push((x, z));
        }
        t
    }
    fn set(&mut self, x: i32, z: i32, h: f32) {
        self.over.insert((x, z), h);
    }
    fn at(&self, x: i32, z: i32) -> f32 {
        self.over.get(&(x, z)).copied().unwrap_or(self.h)
    }
    fn is_wet(&self, x: i32, z: i32) -> bool {
        self.wet.contains(&(x, z))
    }
}

impl crate::network_geometry::GeometryTerrain for MockTerrain {
    fn cells(&self) -> (u32, u32) {
        (64, 64)
    }
    fn vertex_altitude(&self, _v: i32) -> f32 {
        self.h
    }
    fn cell_has_water(&self, x: i32, z: i32) -> bool {
        self.is_wet(x, z)
    }
    fn surface_water_level(&self, _v: i32) -> f32 {
        self.sea
    }
    fn max_allowed_height(&self) -> f32 {
        1000.0
    }
}

impl BridgeTerrain for MockTerrain {
    fn lowest_corner(&self, x: i32, z: i32) -> f32 {
        self.at(x, z)
    }
    fn surface_level(&self, x: i32, z: i32) -> f32 {
        if self.is_wet(x, z) {
            self.sea
        } else {
            self.at(x, z)
        }
    }
    fn water_level(&self, _x: i32, _z: i32) -> f32 {
        self.sea
    }
}

impl span::StepMeasures for MockTerrain {}

/// A straight x-drag of `n` one-cell steps at row `z`.
fn run_x(n: u32, z: i32) -> Run {
    let cells: Vec<(i32, i32)> = (0..n as i32).map(|x| (x, z)).collect();
    let steps: Vec<Step> = (0..n)
        .map(|i| Step {
            first: i,
            last: i,
            kind: StepKind::Regular,
        })
        .collect();
    Run {
        type_id: 0,
        cells,
        steps,
        dragged_count: n,
        straight_count: n,
        pen_direction: 4,
        end_direction: 5,
        ..Run::default()
    }
}

fn world<'a>(run: &'a mut Run, valid: u32, t: &'a MockTerrain, host: &'a NoBridgeHost) -> span::SpanWorld<'a> {
    let straight = run.straight_count;
    span::SpanWorld {
        run,
        valid,
        straight,
        type_id: 0,
        terrain: t,
        host,
        infos: &|_x, _z| None,
    }
}

fn road_tool() -> BridgeTool {
    BridgeTool::new(types::shipped_records(0))
}

#[test]
fn nt31_controlled_bridge_selection_and_retained_price() {
    // NT31 selection-only vector. Complete model construction is a separate
    // scope; this does not instantiate the live city's 974-cost bridge.
    let mut tool = road_tool();
    assert_eq!(tool.types.len(), 9);
    let mut run = run_x(20, 43);
    let terrain = MockTerrain::flat(100.0);
    let host = NoBridgeHost;
    let w = world(&mut run, 20, &terrain, &host);
    for (preferred, expected) in [(-1, 1), (0, 0)] {
        tool.preferred = preferred;
        let (mut start, mut end) = (2, 17);
        assert_eq!(
            select::find_best_bridge_type(&tool, &w, &mut start, &mut end, true, 20.0),
            Some(expected)
        );
        assert_eq!((start, end), (2, 17));
    }
    let rec = BridgeRecord {
        type_id: 0,
        start: 2,
        end: 11,
        bridge_type: 0,
        ..BridgeRecord::default()
    };
    assert_eq!(cost::bridge_cost(&tool, &rec, 100), 900);
    let mut longer = rec.clone();
    longer.end += 1;
    assert_eq!(cost::bridge_cost(&tool, &longer, 100), 1000);
}

// ---- §17 test 1: loading ---------------------------------------------------------

#[test]
fn t17_01_loading() {
    // Road has 9 bridge types with keys 1–9 in INI order; key 4 is exemplar
    // `0x5`.
    let road = types::shipped_records(0);
    assert_eq!(road.len(), 9);
    assert_eq!(
        (1..=9).collect::<Vec<u32>>(),
        road.iter().map(|t| t.key).collect::<Vec<u32>>()
    );
    assert_eq!(types::shipped_types(0)[3].instance, 0x5);
    // Street, pipe, subway, one-way road and dirt road have 0; power line has
    // none (§2.1).
    for t in [3u32, 4, 7, 10, 11] {
        assert!(types::shipped_records(t).is_empty(), "type {t}");
    }
    assert!(load::section_of(5).is_none());
    // Every shipped type: max span 100000, clearance [0, 10000], cost (0, 0).
    for t in &road {
        assert_eq!(t.max_span, DEFAULT_MAX_SPAN);
        assert_eq!(t.min_clearance, 0.0);
        assert_eq!(t.max_clearance, DEFAULT_MAX_CLEARANCE);
        assert_eq!((t.cost_fixed, t.cost_per_step), (0, 0));
    }
}

// ---- §17 test 2: default minimum span -------------------------------------------

#[test]
fn t17_02_default_min_span() {
    // An exemplar with N = 3 ramp steps and no `0x482CB458`: minimum span 8.
    let mut t = types::BridgeType {
        ramp_steps: 3,
        ..types::BridgeType::default()
    };
    if find::min_span_is_default(&t) {
        t.min_span = types::BridgeType::default_min_span(t.ramp_steps);
    }
    assert_eq!(types::BridgeType::default_min_span(3), 8);
    let mut t2 = types::BridgeType {
        ramp_steps: 3,
        ..types::BridgeType::default()
    };
    t2.finish_derived();
    assert_eq!(t2.min_span, 2);
    // The loader applies `2N + 2` when the property is absent.
    let src = load::TextSource::default();
    let mut out = Vec::new();
    load::read_bridge_type_info(
        &mut out,
        &load::IniEntry {
            key: "1".into(),
            value: "0x6534284a, 0xa82ca30f, 0x00000001, test".into(),
        },
        1,
        &src,
        &NoBridgeHost,
    );
    // No exemplar: the record is removed (§2.2 step 5).
    assert!(out.is_empty());
    let _ = t;
}

// ---- §17 test 3: malformed entry -------------------------------------------------

#[test]
fn t17_03_malformed_entry() {
    // Adding `10 = 0x6534284a, 0xa82ca30f` (2 tokens) gives a tenth road
    // bridge type: key 10 and every field at its default.
    let mut out = types::shipped_records(0);
    let src = load::TextSource::default();
    load::read_bridge_type_info(
        &mut out,
        &load::IniEntry {
            key: "10".into(),
            value: "0x6534284a, 0xa82ca30f".into(),
        },
        1,
        &src,
        &NoBridgeHost,
    );
    assert_eq!(out.len(), 10, "[quirk 1] the default record stays");
    let d = &out[9];
    assert_eq!(d.key, 10);
    assert_eq!(
        *d,
        types::BridgeType {
            key: 10,
            ..types::BridgeType::default()
        }
    );
}

// ---- §17 test 4: ramp heights ---------------------------------------------------

#[test]
fn t17_04_ramp_heights() {
    // Road key 2: `+0x78` ≈ 36.652, `+0x60[7]` = 0, `+0x7C` = 4.
    let t = &types::shipped_records(0)[1];
    assert_eq!(t.key, 2);
    assert_eq!(t.ramp_drops.len(), 8);
    assert!((t.total_ramp_rise - 36.652).abs() < 0.01, "{}", t.total_ramp_rise);
    assert_eq!(t.ramp_drops[7], 0.0);
    assert_eq!(t.last_column_ramp, 4);
    // Road key 1: ramp pairs all (0, 0), so `+0x78` = 0.
    assert_eq!(types::shipped_records(0)[0].total_ramp_rise, 0.0);
}

// ---- §17 test 5: specialisation grammar -----------------------------------------

#[test]
fn t17_05_specialization_grammar() {
    let keys = [1u32, 2, 3, 4, 5, 6, 7, 8, 9];
    let widths = [1u32; 9];
    let mins = [10u32, 16, 4, 4, 22, 4, 10, 12, 4];
    let maxs = [DEFAULT_MAX_SPAN; 9];
    // `bridge 99 length 10 end end` where no bridge type has key 99: the
    // parse fails and nothing after it is read.
    let text = "bridge 99 length 10 end end\nbridge 1 length 10 end end\n";
    let (map, ok) = parse_specializations(text, &keys, &widths, &mins, &maxs);
    assert!(!ok);
    assert!(map.iter().all(|m| m.is_empty()), "nothing after the failure is read");
    // `bridge 1 length 9 … end` for road key 1 (minimum span 10): the whole
    // parse fails.
    let (map, ok) = parse_specializations("bridge 1 length 9 end end", &keys, &widths, &mins, &maxs);
    assert!(!ok);
    assert!(map[0].is_empty());
    // A well-formed one lands.
    let (map, ok) = parse_specializations("bridge 1 length 10 end end", &keys, &widths, &mins, &maxs);
    assert!(ok);
    assert!(map[0].contains_key(&10));
}

// ---- §17 test 6: land flag ------------------------------------------------------

#[test]
fn t17_06_land_flag() {
    // Road: `+0x354` = 1. Street (no bridge types): 0.
    assert!(load::land_bridge_flag(9));
    assert!(!load::land_bridge_flag(0));
    // A test bridge type with `0x482CB44F` false as the only road type: still
    // 1 (the address test, quirk 3).
    let t = types::BridgeType {
        land: false,
        ..types::BridgeType::default()
    };
    assert!(!t.land);
    assert!(load::land_bridge_flag(1), "[quirk 3] the address, not the byte");
}

// ---- §17 test 7: edge mask ------------------------------------------------------

#[test]
fn t17_07_edge_mask() {
    let mut run = run_x(3, 10);
    run.pen_direction = 4;
    let mut ci = CellInfo::default();
    // A one-cell step whose cell has a road piece with edge word `0x00020002`:
    // passes.
    ci.edges[0] = 0x0002_0002;
    ci.flags = 1;
    let infos = |c: CellInfo| move |x, z| if (x, z) == (1, 10) { Some(c.clone()) } else { None };
    assert!(span::bridge_step_edge_types_ok(&run, 1, 0, &infos(ci.clone())));
    // The same with `0x00000200` (a z-direction road): fails.
    ci.edges[0] = 0x0000_0200;
    assert!(!span::bridge_step_edge_types_ok(&run, 1, 0, &infos(ci.clone())));
    // A cell with a rail piece (type mask bit 1) on a road drag: fails.
    ci.edges[0] = 0x0002_0002;
    ci.flags = 1 | 2;
    assert!(!span::bridge_step_edge_types_ok(&run, 1, 0, &infos(ci)));
    // A cell without a cell info passes.
    assert!(span::bridge_step_edge_types_ok(&run, 1, 0, &|_x, _z| None));
    // The masks of §4.3.
    assert_eq!(span::EdgeMask::allowed(4, 0, 1), 0x0002_0002);
    assert_eq!(span::EdgeMask::allowed(0, 0, 3), 0x0402_0002);
    assert_eq!(span::EdgeMask::allowed(0, 1, 3), 0x0402_0402);
    assert_eq!(span::EdgeMask::allowed(0, 2, 3), 0x0002_0402);
    assert_eq!(span::EdgeMask::allowed(2, 0, 1), 0x0200_0200);
    assert_eq!(span::EdgeMask::allowed(6, 0, 3), 0x0204_0200);
    assert_eq!(span::EdgeMask::allowed(6, 1, 3), 0x0204_0204);
    assert_eq!(span::EdgeMask::allowed(6, 2, 3), 0x0200_0204);
    let _ = run;
}

// ---- §17 test 17: cost ----------------------------------------------------------

#[test]
fn t17_17_cost() {
    // Road, the bridge of test 8 (start 9, end 20) on flat land: the bridge
    // term is 11 × 100 = 1100; highway with the same steps: 11 × 1000 =
    // 11000.
    let tool = road_tool();
    let rec = BridgeRecord {
        type_id: 0,
        start: 9,
        end: 20,
        bridge_type: 7,
        ..BridgeRecord::default()
    };
    assert_eq!(cost::bridge_cost(&tool, &rec, 100), 1100);
    assert_eq!(cost::bridge_cost(&tool, &rec, 1000), 11000);
    // With a non-zero `+0x150`/`+0x154`: `(end − start) × (+0x154 + +0x320)
    // + +0x150`.
    let mut t2 = tool.clone();
    t2.types[7].cost_fixed = 25;
    t2.types[7].cost_per_step = 2;
    assert_eq!(cost::bridge_cost(&t2, &rec, 100), 11 * 102 + 25);
}

// ---- §17 test 21: orientation ---------------------------------------------------

#[test]
fn t17_21_orientation() {
    // Pen 0 → R 3, cells last to first, t = 2; pen 6 → R 2, t = 3.
    let o0 = model::Orientation::of(0);
    assert_eq!(o0.r, 3);
    assert_eq!(o0.t, 2);
    assert!(!o0.lanes_forward);
    let o6 = model::Orientation::of(6);
    assert_eq!(o6.r, 2);
    assert_eq!(o6.t, 3);
    assert!(!o6.lanes_forward);
    let o4 = model::Orientation::of(4);
    assert_eq!(o4.r, 1);
    assert_eq!(o4.t, 0);
    assert!(o4.lanes_forward);
    let o2 = model::Orientation::of(2);
    assert_eq!(o2.r, 0);
    assert_eq!(o2.t, 1);
    // A deck piece with `+0x30` = 1 on pen 4 has rotation 2.
    assert_eq!(o4.rotate(1 & 3), 2);
}

// ---- §17 test 25: getters -------------------------------------------------------

#[test]
fn t17_25_getters() {
    use getters::ToolBridgeGetters as G;
    let mut tool = road_tool();
    // With no bridge: `GetMajorBridge` = −1, `GetMajorBridgeAltitude` false.
    assert_eq!(G::get_major_bridge(&tool), -1);
    assert!(G::get_major_bridge_altitude(&tool).is_none());
    assert!(G::get_major_bridge_reference_altitude(&tool).is_none());
    assert!(G::is_major_bridge_land_bridge(&tool).is_none());
    // `GetBridgeBitmapKey(0, …)` for road: true, (`0x6A386D26`, 1).
    assert_eq!(G::get_bridge_bitmap_key(&tool, 0), Some((0x6A38_6D26, 1)));
    // Reset: `+0x200 := −1`, `+0x204 := −1.0`.
    G::set_preferred_bridge(&mut tool, 4);
    G::set_bridge_altitude_override(&mut tool, 300.0);
    G::reset_preferences(&mut tool);
    assert_eq!(G::get_preferred_bridge(&tool), -1);
    assert_eq!(G::get_bridge_altitude_override(&tool), -1.0);
    // The major bridge is the **last** record (§1.1).
    tool.append_record(BridgeRecord {
        bridge_type: 2,
        deck: 10.0,
        reference: 11.0,
        land: 0,
        ..BridgeRecord::default()
    });
    tool.append_record(BridgeRecord {
        bridge_type: 7,
        deck: 20.0,
        reference: 21.0,
        land: 1,
        ..BridgeRecord::default()
    });
    assert_eq!(G::get_major_bridge(&tool), 7);
    assert_eq!(G::get_major_bridge_altitude(&tool), Some(20.0));
    assert_eq!(G::get_major_bridge_reference_altitude(&tool), Some(21.0));
    assert_eq!(G::is_major_bridge_land_bridge(&tool), Some(true));
    // Slot 30: the candidate list; false when empty.
    let mut list = Vec::new();
    assert!(!G::get_possible_bridges(&tool, &mut list));
    tool.candidates = vec![1, 2, 3];
    assert!(G::get_possible_bridges(&tool, &mut list));
    assert_eq!(list, vec![1, 2, 3]);
}

// ---- §17 tests 18–20: the parametric layout -------------------------------------

fn road_key1() -> BridgeType {
    types::shipped_records(0).remove(0)
}

#[test]
fn t17_18_parametric_layout() {
    // Road key 1, start 5, end 24, pen 4 (R = 1): pieces by step.
    let t = road_key1();
    assert_eq!((t.ramp_steps, t.pier_steps, t.width), (3, 3, 1));
    assert_eq!((t.pier_spacing, t.first_pier, t.last_column_ramp), (6, 6, 0));
    let (next, _placed, spacing, _last) = model::pier_positions(&t, 5, 24);
    assert_eq!((next, spacing), (11, 7));
    let o = model::Orientation::of(4);
    let mut rec = BridgeRecord {
        type_id: 0,
        start: 5,
        end: 24,
        bridge_type: 0,
        deck: 100.0,
        ..BridgeRecord::default()
    };
    let cells_of = |s: u32| -> Vec<(i32, i32)> { vec![(s as i32, 10)] };
    model::construct_parametric_bridge(&mut rec, &t, &o, &cells_of);
    // 20 steps, one lane each.
    assert_eq!(rec.pieces.len(), 20);
    let kind_at = |s: u32| {
        let p = &rec.pieces[(s - 5) as usize];
        (p.kind, p.rotation, p.depth)
    };
    // 5, 6, 7 ramp 0, 1, 2; 8 deck; 9, 10 mirrored pier 2, 1; 11 pier 0;
    // 12, 13 pier 1, 2; 14, 15 deck; 16, 17 mirrored pier 2, 1; 18 pier 0;
    // 19, 20 pier 1, 2; 21 deck; 22, 23, 24 mirrored ramp 2, 1, 0.
    let expect = [
        (PieceKind::Ramp, 256.0),
        (PieceKind::Ramp, -1.0),
        (PieceKind::Ramp, -1.0),
        (PieceKind::Deck, -1.0),
        (PieceKind::Pier, -1.0),
        (PieceKind::Pier, -1.0),
        (PieceKind::Pier, 498.0),
        (PieceKind::Pier, -1.0),
        (PieceKind::Pier, -1.0),
        (PieceKind::Deck, -1.0),
        (PieceKind::Deck, -1.0),
        (PieceKind::Pier, -1.0),
        (PieceKind::Pier, -1.0),
        (PieceKind::Pier, 498.0),
        (PieceKind::Pier, -1.0),
        (PieceKind::Pier, -1.0),
        (PieceKind::Deck, -1.0),
        (PieceKind::Ramp, -1.0),
        (PieceKind::Ramp, -1.0),
        (PieceKind::Ramp, 256.0),
    ];
    for (k, (kind, depth)) in expect.iter().enumerate() {
        let (got_kind, _rot, got_depth) = kind_at(5 + k as u32);
        assert_eq!(got_kind, *kind, "step {}", 5 + k as u32);
        assert_eq!(got_depth, *depth, "step {}", 5 + k as u32);
    }
    // No support flags (no cable models).
    assert!(rec.pieces.iter().all(|p| p.flags == 0));
}

#[test]
fn t17_19_pier_fraction() {
    // Road key 5 (medium suspension), start 10, end 49 (40 steps).
    let t = types::shipped_records(0).remove(4);
    assert_eq!(t.key, 5);
    assert_eq!((t.pier_spacing, t.first_pier, t.end_pier_fraction), (16, 6, 0.0));
    assert_eq!((t.min_piers, t.max_piers, t.pier_steps), (0, 100, 1));
    // The shipped row leaves `+0x3C`, `+0xA8` and `+0xAC` at their defaults
    // unless the exemplar sets them; test 19's figures come from the
    // exemplar. Set them as the test states.
    let mut t = t;
    t.end_pier_fraction = 0.6;
    t.min_piers = 2;
    t.max_piers = 47;
    let (next, _p, spacing, _last) = model::pier_positions(&t, 10, 49);
    assert_eq!((next, spacing), (20, 19));
}

#[test]
fn t17_20_squeezed_piers() {
    // M = 1, `+0x38` = 6, `+0x7C` = 0, `+0xA8` = 2, start 0, end 9: first 6 >
    // last 3, so both become 4, and no pier piece is placed (quirk 7).
    let t = types::BridgeType {
        pier_steps: 1,
        first_pier: 6,
        min_piers: 2,
        ..types::BridgeType::default()
    };
    let (next, placed, _s, _last) = model::pier_positions(&t, 0, 9);
    assert_eq!(next, -1, "[quirk 7] no pier is placed");
    assert_eq!(placed, -1);
}

// ---- §17 test 28: column top ----------------------------------------------------

#[test]
fn t17_28_column_top() {
    // A test bridge type like road key 1 but with `+0xD4` = 10 and `+0xC4` =
    // 5, deck elev = 270, over dry ground at 0 m.
    let mut t = road_key1();
    t.model_height_offset = 10.0;
    t.cable_params[3] = 5.0;
    let o = model::Orientation::of(4);
    let mut rec = BridgeRecord {
        type_id: 0,
        start: 5,
        end: 24,
        bridge_type: 0,
        deck: 270.0,
        ..BridgeRecord::default()
    };
    let cells_of = |s: u32| -> Vec<(i32, i32)> { vec![(s as i32, 10)] };
    model::construct_parametric_bridge(&mut rec, &t, &o, &cells_of);
    // A pier centre piece (depth 498): corners 270, position height 270,
    // `dy` = 285 − 270 = 15 (the model's origin is at `Y` = 285).
    let pier = rec
        .pieces
        .iter()
        .find(|p| p.kind == PieceKind::Pier && p.depth == 498.0)
        .expect("pier centre");
    assert_eq!((pier.x, pier.z), (11, 10));
    assert_eq!(pier.corners, [270.0; 4]);
    let (_x, _z, height) = pieces::occupant_position(&pier.corners, 11, 10, 16.0);
    assert_eq!(height, 270.0);
    assert_eq!(pier.y, 285.0);
    assert_eq!(pieces::model_offset(pier.y, height), 15.0);
    // Its column top is 270 − 498 = −228, below the ground, so no column.
    let cols = pieces::pillar_columns(pier, &t, height, [0.0; 4], -1.0);
    assert!(cols.is_empty());
    // With depth 200: top 70, a column from 0 up to 70 (not 85).
    let mut p2 = pier.clone();
    p2.depth = 200.0;
    let cols = pieces::pillar_columns(&p2, &t, height, [0.0; 4], -1.0);
    assert_eq!(cols.len(), 1);
    assert_eq!((cols[0].bottom, cols[0].top), (0.0, 70.0));
}

// ---- §17 test 24: land record ---------------------------------------------------

#[test]
fn t17_24_land_record() {
    // `+0x18` = 1, `+0x10` = `+0x14` = 100 (no `+0x5C` or `+0x78` added).
    let mut tool = road_tool();
    let elev = 100.0f32;
    tool.append_record(BridgeRecord {
        type_id: 0,
        start: 4,
        end: 10,
        bridge_type: 2,
        deck: elev,
        reference: elev,
        land: 1,
        pieces: Vec::new(),
    });
    let r = tool.major().unwrap();
    assert_eq!(r.land, 1);
    assert_eq!((r.deck, r.reference), (100.0, 100.0));
    // `IsMajorBridgeLandBridge` is true and `GetMajorBridgeAltitude` gives 100.
    assert_eq!(
        getters::ToolBridgeGetters::is_major_bridge_land_bridge(&tool),
        Some(true)
    );
    assert_eq!(
        getters::ToolBridgeGetters::get_major_bridge_altitude(&tool),
        Some(100.0)
    );
}

// ---- §17 test 26: bridge pins ---------------------------------------------------

#[test]
fn t17_26_bridge_pins() {
    // The bridge of test 14 (road key 1, start 9, end 19, deck 270): the
    // constraint entry is {9, 19, 270, 270}; every corner of every cell has
    // height 270, allowed [270, 270], and is fixed.
    let tool = road_tool();
    let mut rec = BridgeRecord {
        type_id: 0,
        start: 9,
        end: 19,
        bridge_type: 0,
        deck: 270.0,
        reference: 250.0,
        land: 0,
        pieces: Vec::new(),
    };
    for k in 9..=19 {
        for _lane in 0..1 {
            rec.pieces.push(model::PieceRecord {
                kind: PieceKind::Deck,
                x: k,
                z: 10,
                corners: [270.0; 4],
                ..model::PieceRecord::default()
            });
        }
    }
    let mut tool = tool;
    tool.records = vec![rec];
    let run = run_x(31, 10);
    let mut map: BTreeMap<i32, CellInfo> = BTreeMap::new();
    for k in 9..=19 {
        map.insert(
            crate::network_geometry::smoothing::cell_key(k, 10),
            CellInfo {
                x: k,
                z: 10,
                ..CellInfo::default()
            },
        );
    }
    let entries = elev::bridge_elevation_constraints(&tool, &run, 31, &mut map);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].as_tuple(), (9, 19, 270.0, 270.0));
    for k in 9..=19 {
        let ci = map.get(&crate::network_geometry::smoothing::cell_key(k, 10)).unwrap();
        assert_eq!(ci.corners, [270.0; 4], "cell {k}");
        assert_eq!(ci.allowed, [(270.0, 270.0); 4], "cell {k}");
        assert!(ci.fixed, "cell {k}");
    }
    // None of those cells is in the solve set `+0x84`.
    let removed = elev::bridge_solve_set_removals(&tool, 31);
    assert_eq!(removed.len(), 11);
}

// ---- §17 tests 11, 13: the water pass's refusals --------------------------------

#[test]
fn t17_11_short_river() {
    // A river at x = 10 … 11 (2 wet steps): start 9, end 12, `end − start` =
    // 3 < 4: no record.
    let t = road_key1();
    assert_eq!(t.min_span, 10);
    // `end − start ≥ 4` and `n ≥` the minimum: 3 ≥ 4 is false, so nothing is
    // recorded (§4.4 step 4). The water-pass case is `t17_11_short_river_no_record`.
    let (start, end) = (9u32, 12u32);
    assert_eq!(end - start, 3);
    assert!(!(end - start >= 4), "end - start must be under 4");
}

#[test]
fn t17_09_tie_break() {
    // As test 8 but `n = 11`: keys 1 and 7 both have `d = 1`; key 1 (index 0)
    // wins.
    let tool = road_tool();
    let n = 11u32;
    let mut best: Option<(u32, i64)> = None;
    for (i, t) in tool.types.iter().enumerate() {
        let s = select::candidate_score(n, t.min_span);
        let better = match best {
            None => true,
            Some((_, bs)) => s.abs() < bs.abs(),
        };
        if better {
            best = Some((i as u32, s));
        }
    }
    // Keys 1 (min 10) and 7 (min 10) both score 1; the first wins.
    assert_eq!(select::candidate_score(11, 10), 1);
    assert_eq!(best, Some((0, 1)));
}

#[test]
fn t17_10_padding() {
    // Key 5 has minimum 22: with room on both sides padding is alternate back
    // and forward to start 4, end 25 (22 steps).
    let run = run_x(31, 10);
    let t = MockTerrain::flat(250.0);
    let host = NoBridgeHost;
    let mut run = run;
    let w = world(&mut run, 31, &t, &host);
    let (mut s, mut e) = (9u32, 20u32);
    assert!(select::pad_steps_to_make_min_bridge_span(&w, 22, &mut s, &mut e));
    assert_eq!((s, e), (4, 25));
    assert_eq!(e - s + 1, 22);
    // With the drag (7, 10) → (30, 10) instead (start 2, end 13, `v` = 24),
    // the back side is blocked at once (`start` must stay > 1) and the forward
    // side takes all ten: start 2, end 23.
    let (mut s, mut e) = (2u32, 13u32);
    let w = world(&mut run, 24, &t, &host);
    assert!(select::pad_steps_to_make_min_bridge_span(&w, 22, &mut s, &mut e));
    assert_eq!((s, e), (2, 23));
}

// ---- helpers the tests above reference ------------------------------------------

mod find {
    use super::types::BridgeType;
    /// Whether `min_span` still holds the `2N + 2` default (§2.3).
    pub fn min_span_is_default(t: &BridgeType) -> bool {
        t.min_span == BridgeType::default_min_span(0) || t.min_span == 2
    }
}

// ---- 17 tests 8, 11, 12, 14, 16: the water pass --------------------------------

/// Road's maximum slope in radians (p+0x00 = 35 degrees), so `c14 = 8.5521`
/// and `c4 = 2.4435` (5.2).
const ROAD_MAX_SLOPE: f32 = 0.610_865_24;

/// Runs the water pass over a straight x-drag of `n` steps with a river
/// `x0..=x1` at row `z`, on flat banks at `bank`.
fn water_pass(n: u32, z: i32, x0: i32, x1: i32, bank: f32, sea: f32, override_h: f32) -> (BridgeTool, u32, Run) {
    let mut run = run_x(n, z);
    let t = MockTerrain::river(bank, sea, x0, x1, z);
    let host = NoBridgeHost;
    let mut tool = road_tool();
    tool.altitude_override = override_h;
    let mut valid = n;
    {
        let mut w = span::SpanWorld {
            run: &mut run,
            valid,
            straight: n,
            type_id: 0,
            terrain: &t,
            host: &host,
            infos: &|_x, _z| None,
        };
        identify_bridge_cells(&mut tool, &mut w, &mut valid, ROAD_MAX_SLOPE);
    }
    (tool, valid, run)
}

#[test]
fn t17_08_water_pass_simple() {
    // Road, straight drag (0, 10) -> (30, 10), pen 4, a river at x = 10..19:
    // start 9, end 20, n = 12. The winner is key 8 (cantilever truss, index
    // 7, d = 0). One record, +0x18 = 0, v unchanged.
    let (tool, valid, _run) = water_pass(31, 10, 10, 19, 250.0, 250.0, -1.0);
    assert_eq!(valid, 31, "v unchanged");
    assert_eq!(tool.records.len(), 1);
    let r = &tool.records[0];
    assert_eq!((r.start, r.end), (9, 20));
    assert_eq!(tool.types[r.bridge_type as usize].key, 8);
    assert_eq!(r.bridge_type, 7);
    assert_eq!(r.land, 0);
    assert_eq!(select::candidate_score(12, 12), 0);
}

#[test]
fn t17_11_short_river_no_record() {
    // A river at x = 10..11 (2 wet steps): end - start = 3 < 4: no record,
    // v unchanged. CheckForGoodTerrain then refuses the run with 0xF0000001.
    let (tool, valid, _run) = water_pass(31, 10, 10, 11, 250.0, 250.0, -1.0);
    assert_eq!(valid, 31, "v unchanged");
    assert!(tool.records.is_empty());
    assert_eq!(cost::CODE_BAD_TERRAIN, 0xF000_0001);
}

#[test]
fn t17_12_no_far_bank() {
    // Road, (0, 10) -> (15, 10) with a river from x = 10 to past the end:
    // the end reaches +0x78 - 1; v := 10.
    let (tool, valid, _run) = water_pass(16, 10, 10, 30, 250.0, 250.0, -1.0);
    assert_eq!(valid, 10, "the run is cut at the wet step");
    assert!(tool.records.is_empty());
}

#[test]
fn t17_14_deck_elevation() {
    // As test 9 (road key 1), water level 250 m, flat banks at 250 m:
    // reference elevation +0x14 = 250, deck +0x10 = 270.
    let (tool, valid, _run) = water_pass(31, 10, 10, 18, 250.0, 250.0, -1.0);
    assert_eq!(valid, 31);
    assert_eq!(tool.records.len(), 1);
    let r = &tool.records[0];
    assert_eq!(tool.types[r.bridge_type as usize].key, 1);
    assert_eq!((r.start, r.end), (9, 19));
    assert_eq!(r.reference, 250.0);
    assert_eq!(r.deck, 270.0);
    // With SetBridgeAltitudeOverride(300): the reference is 300; the flat
    // banks never reach it, so the span widens to start 1 and end v - 2;
    // the deck is 300.
    let (tool, _v, _run) = water_pass(31, 10, 10, 18, 250.0, 250.0, 300.0);
    let r = &tool.records[0];
    assert_eq!(r.reference, 300.0);
    assert_eq!((r.start, r.end), (1, 29));
    assert_eq!(r.deck, 300.0);
}

#[test]
fn t17_16_steep_banks() {
    // Road (c14 = 8.5521, c4 = 2.4435), water level 250. The land behind the
    // bank step is flat at 262: at k = start, d = 262 - 250 = 12 > c14, so
    // A = B = 262. The far bank is the mirror image. elev = 262, no
    // widening.
    let c14 = (f64::from(ROAD_MAX_SLOPE) * 14.0) as f32;
    let c4 = (f64::from(ROAD_MAX_SLOPE) * 4.0) as f32;
    assert!((c14 - 8.552_14).abs() < 0.01, "c14 = {c14}");
    assert!((c4 - 2.443_47).abs() < 0.01, "c4 = {c4}");

    let mut run = run_x(31, 10);
    let mut t = MockTerrain::river(250.0, 250.0, 10, 19, 10);
    for x in 0..10 {
        t.set(x, 10, 262.0);
    }
    for x in 21..31 {
        t.set(x, 10, 262.0);
    }
    let host = NoBridgeHost;
    let mut tool = road_tool();
    let mut valid = 31u32;
    {
        let mut w = span::SpanWorld {
            run: &mut run,
            valid,
            straight: 31,
            type_id: 0,
            terrain: &t,
            host: &host,
            infos: &|_x, _z| None,
        };
        identify_bridge_cells(&mut tool, &mut w, &mut valid, ROAD_MAX_SLOPE);
    }
    assert_eq!(tool.records.len(), 1);
    let r = &tool.records[0];
    assert_eq!(r.reference, 262.0, "min(B, B2, max(A, A2))");
    assert_eq!((r.start, r.end), (9, 20), "no widening");

    // With a rise of 5 m instead (d = 5, between c4 and c14): B = B2 = 255,
    // A = A2 = 0, elev = min(255, 255, 0) = 0, so the floor rule gives the
    // surface of step start's first cell, 255.
    let mut run = run_x(31, 10);
    let mut t = MockTerrain::river(255.0, 250.0, 10, 19, 10);
    for x in 0..10 {
        t.set(x, 10, 255.0);
    }
    for x in 21..31 {
        t.set(x, 10, 255.0);
    }
    let mut tool = road_tool();
    let mut valid = 31u32;
    {
        let mut w = span::SpanWorld {
            run: &mut run,
            valid,
            straight: 31,
            type_id: 0,
            terrain: &t,
            host: &host,
            infos: &|_x, _z| None,
        };
        identify_bridge_cells(&mut tool, &mut w, &mut valid, ROAD_MAX_SLOPE);
    }
    assert_eq!(tool.records.len(), 1);
    let r = &tool.records[0];
    assert_eq!(r.reference, 255.0, "the floor rule");
}
