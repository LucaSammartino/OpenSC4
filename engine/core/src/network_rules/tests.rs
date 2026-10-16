// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! network-rules.md Ã‚Â§15 (tests 1-24, named `t15_NN_Ã¢â‚¬Â¦`) and further checks.
//! Every rule file is synthetic.

use super::*;
use crate::stl::StlHashSet;

fn grid() -> CellGrid {
    CellGrid::new(64, 64)
}

fn road(x: i32, z: i32, e: u32) -> CellInfo {
    CellInfo::new(x, z).with_type(ROAD, e)
}

fn set_of(keys: &[u32]) -> StlHashSet {
    let mut s = StlHashSet::with_hint(100);
    for &k in keys {
        s.insert(k);
    }
    s
}

fn eval_at(ev: &Evaluator, g: &CellGrid, x: i32, z: i32, t: u8) -> Option<Vec<Output>> {
    ev.evaluate(&neighbourhood(g, cell_key(x, z), t))
        .map(<[Output]>::to_vec)
}

fn rules_of(ev: &Evaluator) -> Vec<(u32, Vec<(u32, u8)>)> {
    ev.rules()
        .map(|r| (r.key, r.outputs.iter().map(|o| (o.id, o.orientation)).collect()))
        .collect()
}

// ---- Ã‚Â§15 tests 1-9: the reader and the evaluator ---------------------------------------------------------

#[test]
fn t15_01_edge_tokens() {
    assert_eq!(edge_token(2), 0x02);
    assert_eq!(edge_token(13), 0x0B);
    assert_eq!(edge_token(27), 0x17);
    assert_eq!(edge_token(260), 0x04);
    assert_eq!(edge_token(0), 0);
}

#[test]
fn t15_02_one_rule() {
    let ev = Evaluator::from_bytes(b"#TestRules#\n1,0,2,0,2\n3,0,0x100,0,0");
    assert_eq!(ev.status, LoadStatus::Complete);
    let r: Vec<&TileRule> = ev.rules().collect();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].key, 0x0200_0200);
    assert!(r[0].conditions.is_empty());
    assert_eq!(
        r[0].outputs,
        vec![Output {
            id: 0x100,
            m: 0,
            orientation: 0
        }]
    );
    assert_eq!(r[0].transform, 0xFF);
}

#[test]
fn t15_03_rejected_header() {
    let ev = Evaluator::from_bytes(b"TestRules\n1,0,2,0,2\n3,0,0x100,0,0");
    assert_eq!(ev.status, LoadStatus::Rejected);
    assert_eq!(ev.rule_count(), 0);
    let mut g = grid();
    g.insert(road(10, 10, 0x0200_0200));
    assert!(eval_at(&ev, &g, 10, 10, ROAD).is_none());
    // The evaluator still exists in the database (Ã‚Â§2.3).
    let mut r = NetworkRules::new();
    r.add_tile_file(ROAD, Some(b"TestRules\n1,0,2,0,2\n3,0,0x100,0,0"));
    r.add_tile_file(ROAD, None);
    assert_eq!(r.evaluators[ROAD as usize].len(), 2);
}

#[test]
fn t15_04_bad_centre() {
    let ev = Evaluator::from_bytes(b"#T#\n2,0,0,0,0\n3,0,0x100,0,0");
    assert_eq!(ev.status, LoadStatus::BadCentre);
    assert_eq!(ev.rule_count(), 0);
    // Rules read before a bad centre stay (Ã‚Â§3.3 [inferred]).
    let ev = Evaluator::from_bytes(b"#T#\n1,0,2,0,2\n3,0,0x100,0,0\n1,2,0,2,0\n3,0,0x101,0,0\n7,0,0,0,0\n1,1,1,1,1");
    assert_eq!(ev.status, LoadStatus::BadCentre);
    assert_eq!(ev.rule_count(), 2);
}

#[test]
fn t15_05_transmogrify_straight() {
    let ev = Evaluator::from_bytes(b"#Transmogrify\n1,0,2,0,2\n3,0,0x100,0,0");
    assert!(ev.transmogrify);
    assert_eq!(
        rules_of(&ev),
        vec![(0x0002_0002, vec![(0x100, 0x01)]), (0x0200_0200, vec![(0x100, 0x00)])]
    );
    assert_eq!(ev.dropped_variants, 6);
    assert!(ev.rules().all(|r| r.transform == 0x01));
}

#[test]
fn t15_06_transmogrify_diagonal() {
    let want = vec![
        (0x0000_0301, vec![(0x200, 0)]),
        (0x0003_0100, vec![(0x200, 1)]),
        (0x0100_0003, vec![(0x200, 3)]),
        (0x0301_0000, vec![(0x200, 2)]),
    ];
    let ev = Evaluator::from_bytes(b"#Transmogrify\n1,1,3,0,0\n3,0,0x200,0,0");
    assert_eq!(rules_of(&ev), want);
    let ev = Evaluator::from_bytes(b"#Transmogrify\nnoflip\n1,1,3,0,0\n3,0,0x200,0,0");
    assert_eq!(rules_of(&ev), want);
    // The mirror of 0x00000301 is turn 1's pattern.
    assert_eq!(mirror_edges(0x0000_0301), 0x0003_0100);
    // notrans keeps the rule alone.
    let ev = Evaluator::from_bytes(b"#Transmogrify\nnotrans\n1,1,3,0,0\n3,0,0x200,0,0");
    assert_eq!(rules_of(&ev), vec![(0x0000_0301, vec![(0x200, 0)])]);
}

#[test]
fn t15_07_mirror_of_a_subtype() {
    assert_eq!(mirror_edge_byte(edge_token(22)), edge_token(42));
    assert_eq!(edge_token(22), 0x12);
    assert_eq!(mirror_edge_byte(0x12), 0x22);
    assert_eq!(edge_token(232), 0xBA);
    assert_eq!(mirror_edge_byte(0xBA), 0x3A);
    assert_eq!(mirror_edge_byte(edge_token(13)), edge_token(11));
    assert_eq!(mirror_edge_byte(0x0B), 0x09);
    // Kinds 0 and 4-7 unchanged.
    for b in [0x00u8, 0x04, 0x0D, 0x16, 0xFF, 0x78] {
        assert_eq!(mirror_edge_byte(b), b);
    }
}

#[test]
fn t15_08_rotation_tables() {
    assert_eq!(rotate_meta(1, 1), 3);
    assert_eq!(rotate_meta(9, 1), 13);
    assert_eq!(rotate_meta(12, 3), 24);
    assert_eq!(rotate_meta(0, 3), 0);
    assert_eq!(rotate_orientation(0x03, 2), 0x01);
    assert_eq!(rotate_orientation(0x81, 1), 0x80);
    // Turning a metaposition turns its offset: (dx, dz) -> (-dz, dx).
    for m in 0..25 {
        let (dx, dz) = meta_offset(m);
        assert_eq!(meta_offset(rotate_meta(m, 1)), (-dz, dx), "m {m}");
        assert_eq!(meta_offset(mirror_meta(m)), (-dx, dz), "m {m}");
    }
    // The flipped table rows.
    for rot in 0..4u8 {
        for r in 0..4u8 {
            assert_eq!(rotate_orientation(0x80 | rot, r), 0x80 | (rot.wrapping_sub(r) & 3));
        }
    }
}

#[test]
fn t15_09_conditions_and_wildcards() {
    let strict = Evaluator::from_bytes(b"#T#\n1,0,2,0,0\n2,3,0,0,0,2\n3,0,0x200,0,0");
    let wild = Evaluator::from_bytes(b"#T#\n1,0,2,0,0\n2,3,?,?,?,2\n3,0,0x200,0,0");
    let c = &wild.rules().next().unwrap().conditions[0];
    assert_eq!((c.m, c.value, c.mask), (3, 0x0200_0000, 0xFF00_0000));
    let mut g = grid();
    g.insert(road(10, 10, 0x0000_0200));
    g.insert(road(10, 9, 0x0200_0000));
    let out = eval_at(&strict, &g, 10, 10, ROAD).unwrap();
    assert_eq!(out[0].id, 0x200);
    assert!(eval_at(&wild, &g, 10, 10, ROAD).is_some());
    g.insert(road(10, 9, 0x0200_0200));
    assert!(eval_at(&strict, &g, 10, 10, ROAD).is_none());
    assert!(eval_at(&wild, &g, 10, 10, ROAD).is_some());
}

// ---- tests 10-11: valid positions and claims -------------------------------------------------------------

#[test]
fn t15_10_valid_positions() {
    let ev = Evaluator::from_bytes(b"#T#\n1,0,0,2,0\n3,0,0x300,0,0\n3,5,0x301,0,0");
    assert_eq!(ev.rules().next().unwrap().key, 0x0002_0000);
    let mut g = grid();
    g.insert(road(10, 10, 0x0002_0000));
    assert!(eval_at(&ev, &g, 10, 10, ROAD).is_some());
    g.insert(CellInfo::new(11, 10).with_type(ROAD, 0));
    assert!(eval_at(&ev, &g, 10, 10, ROAD).is_some());
    g.insert(CellInfo::new(11, 10).with_type(RAIL, 0));
    assert!(eval_at(&ev, &g, 10, 10, ROAD).is_none());
    g.insert(CellInfo::new(11, 10).with_type(ROAD, 0).with_type(RAIL, 0));
    assert!(eval_at(&ev, &g, 10, 10, ROAD).is_none());
}

#[test]
fn t15_11_claims() {
    let ev = Evaluator::from_bytes(b"#T#\n1,0,2,0,2\n3,0,0x100,0,0\n3,7,0x101,0,0");
    let mut g = grid();
    g.insert(road(5, 5, 0x0200_0200));
    g.insert(road(5, 6, 0x0200_0200));
    let (a, b) = (cell_key(5, 5), cell_key(5, 6));
    assert_eq!((a, b), (0x0005_0005, 0x0006_0005));
    let mut st = SolveState::new();
    st.s = set_of(&[a, b]);
    let mut p = set_of(&[a, b]);
    assert_eq!(p.bucket_count(), 193);
    assert_eq!(p.iter_order(), vec![b, a]);
    let mut w = StlHashSet::with_hint(100);
    apply_network_rules(&mut g, &mut st, &mut p, &mut w, &ev, ROAD);
    assert_eq!(
        st.solved,
        vec![
            SolvedRecord {
                id: 0x100,
                orientation: 0,
                key: b
            },
            SolvedRecord {
                id: 0x100,
                orientation: 0,
                key: a
            }
        ]
    );
    assert!(st.s.is_empty());
    assert_eq!(g.get(b).unwrap().solved, 0);
    assert_eq!(g.get(a).unwrap().solved, 1);
}

// ---- tests 12-14, 21: multi-type intersections -------------------------------------------------------------

fn multi(text: &str) -> NetworkRules {
    let mut r = NetworkRules::new();
    r.load_multi_type(text.as_bytes());
    r
}

#[test]
fn t15_12_two_types_crossing() {
    let cell = || road(10, 10, 0x0200_0200).with_type(RAIL, 0x0002_0002);
    let key = cell_key(10, 10);
    let rules = multi("[RoadRailIntersectionSolutions]\nk=0x02000200,0x00020002,0x1234,1,0\n");
    let mut g = grid();
    g.insert(cell());
    let mut st = SolveState::new();
    st.s = set_of(&[key]);
    let mut out = Vec::new();
    solve_multi_type_intersections(&rules, &mut g, &mut st, &mut out, ROAD);
    assert_eq!(out, vec![key]);
    assert_eq!(
        st.solved,
        vec![SolvedRecord {
            id: 0x1234,
            orientation: 0x01,
            key
        }]
    );
    assert_eq!(st.potential, vec![(10, 10, 0x3)]);
    // No arm was cleared.
    assert_eq!(g.get(key).unwrap().edges[ROAD as usize], 0x0200_0200);

    // Through the whole solve: solved, S empty.
    let mut g = grid();
    g.insert(CellInfo { in_run: true, ..cell() });
    let mut st = SolveState::new();
    st.s = set_of(&[key]);
    st.valid_count = 1;
    assert_eq!(solve_network(&rules, &mut g, &mut st, ROAD, false), 1);
    assert!(st.s.is_empty());

    // Without the entry: it stays in S and no evaluator takes it.
    let mut rules = NetworkRules::new();
    rules.add_tile_file(ROAD, Some(b"#T#\n1,0,2,0,2\n3,0,0x100,0,0"));
    let mut g = grid();
    g.insert(CellInfo {
        in_run: true,
        step: 0,
        ..cell()
    });
    let mut st = SolveState::new();
    st.s = set_of(&[key]);
    st.valid_count = 1;
    assert_eq!(solve_network(&rules, &mut g, &mut st, ROAD, false), 0);
    assert!(st.s.contains(key));
    assert!(st.solved.is_empty());
}

#[test]
fn t15_13_covered_with_override() {
    let key = cell_key(10, 10);
    let make = || {
        let mut g = grid();
        g.insert(road(10, 10, 0x0002_0002).with_type(STREET, 0x0002_0002));
        let mut st = SolveState::new();
        st.s = set_of(&[key]);
        (g, st)
    };
    let mut rules = multi("[AllowedOverrides]\nk=Road,Street\n");
    assert_eq!(rules.override_rows[ROAD as usize], 1 << STREET);
    assert_eq!(rules.override_rows[STREET as usize], 1 << ROAD);
    let (mut g, mut st) = make();
    let mut out = Vec::new();
    solve_multi_type_intersections(&rules, &mut g, &mut st, &mut out, ROAD);
    let c = g.get(key).unwrap();
    assert_eq!((c.flags, c.edges[STREET as usize], c.union), (1, 0, 0x0002_0002));
    assert!(out.is_empty() && st.s.contains(key));
    // The Road evaluators then solve it.
    rules.add_tile_file(ROAD, Some(b"#T#\n1,2,0,2,0\n3,0,0x500,0,0"));
    let (mut g, mut st) = make();
    st.valid_count = 1;
    assert_eq!(solve_network(&rules, &mut g, &mut st, ROAD, false), 1);
    assert_eq!(st.solved[0].id, 0x500);

    // Without the override nothing changes.
    let rules = NetworkRules::new();
    let (mut g, mut st) = make();
    solve_multi_type_intersections(&rules, &mut g, &mut st, &mut out, ROAD);
    assert_eq!(
        *g.get(key).unwrap(),
        road(10, 10, 0x0002_0002).with_type(STREET, 0x0002_0002)
    );
}

#[test]
fn t15_14_three_types() {
    let key = cell_key(10, 10);
    let rules = multi("[RoadRailIntersectionSolutions]\nk=0x00020002,0x02000200,0x777,0,1\n");
    let mut g = grid();
    g.insert(
        road(10, 10, 0x0002_0002)
            .with_type(RAIL, 0x0200_0200)
            .with_type(STREET, 0x0002_0002),
    );
    let mut st = SolveState::new();
    st.s = set_of(&[key]);
    let mut out = Vec::new();
    solve_multi_type_intersections(&rules, &mut g, &mut st, &mut out, ROAD);
    let c = g.get(key).unwrap();
    assert_eq!(c.flags, 0b11);
    assert_eq!(c.edges[STREET as usize], 0);
    assert_eq!(
        st.solved,
        vec![SolvedRecord {
            id: 0x777,
            orientation: 0x80,
            key
        }]
    );
    assert_eq!(st.potential, vec![(10, 10, 0b11)]);
}

#[test]
fn t15_21_bridge_cell_as_two_types() {
    let key = cell_key(10, 10);
    let cell = CellInfo {
        flags: 0x1 | FLAG_BRIDGE,
        in_run: true,
        ..road(10, 10, 0x0002_0002)
    };
    assert_eq!(type_of(FLAG_BRIDGE), GROUND_HIGHWAY);
    let mut rules = multi("[RoadGroundHighwayIntersectionSolutions]\nk=0x00020002,0x00000000,0x4321,0,0\n");
    rules.add_tile_file(ROAD, Some(b"#T#\n1,2,0,2,0\n3,0,0x500,0,0"));
    let mut g = grid();
    g.insert(cell);
    let mut st = SolveState::new();
    st.s = set_of(&[key]);
    st.valid_count = 1;
    assert_eq!(solve_network(&rules, &mut g, &mut st, ROAD, false), 1);
    assert_eq!(
        st.solved,
        vec![SolvedRecord {
            id: 0x4321,
            orientation: 0,
            key
        }]
    );
    // Without the entry, the Road evaluators solve it as usual.
    let mut rules = NetworkRules::new();
    rules.add_tile_file(ROAD, Some(b"#T#\n1,2,0,2,0\n3,0,0x500,0,0"));
    let mut g = grid();
    g.insert(cell);
    let mut st = SolveState::new();
    st.s = set_of(&[key]);
    st.valid_count = 1;
    assert_eq!(solve_network(&rules, &mut g, &mut st, ROAD, false), 1);
    assert_eq!(st.solved[0].id, 0x500);
}

// ---- tests 15-16: edge subtypes ------------------------------------------------------------------------------

#[test]
fn t15_15_scheme_2() {
    let (c, n) = (cell_key(10, 10), cell_key(10, 9));
    for north_in_p in [true, false] {
        let mut g = grid();
        g.insert(CellInfo::new(10, 10).with_type(RAIL, 0x0000_0201));
        g.insert(CellInfo::new(10, 9).with_type(RAIL, 0x0200_0000));
        let mut p = if north_in_p { set_of(&[c, n]) } else { set_of(&[c]) };
        let mut r = StlHashSet::with_hint(100);
        assign_edge_subtypes(&mut g, &mut p, &mut r, true);
        assert!(r.is_empty());
        assert_eq!(g.get(c).unwrap().edges[RAIL as usize], 0x0000_1201);
        let want_n = if north_in_p { 0x1200_0000 } else { 0x0200_0000 };
        assert_eq!(g.get(n).unwrap().edges[RAIL as usize], want_n);
    }
    // A fixed cell in P cannot take a subtype.
    let mut g = grid();
    g.insert(CellInfo {
        fixed: true,
        ..CellInfo::new(10, 10).with_type(RAIL, 0x0000_0201)
    });
    let mut p = set_of(&[c]);
    let mut r = StlHashSet::with_hint(100);
    assign_edge_subtypes(&mut g, &mut p, &mut r, true);
    assert_eq!(r.iter_order(), vec![c]);
    assert!(!p.contains(c));
}

#[test]
fn t15_16_scheme_1() {
    let (c, n) = (cell_key(10, 10), cell_key(9, 10));
    let mut g = grid();
    g.insert(road(10, 10, 0x0002_0001));
    g.insert(road(9, 10, 0x0003_0100));
    let mut p = set_of(&[c, n]);
    let mut r = StlHashSet::with_hint(100);
    assign_edge_subtypes(&mut g, &mut p, &mut r, true);
    assert_eq!(g.get(c).unwrap().edges[0], 0x0002_0009);
    assert_eq!(g.get(n).unwrap().edges[0], 0x000B_0100);
    // (9, 10) not in P: only (10, 10) changes.
    let mut g = grid();
    g.insert(road(10, 10, 0x0002_0001));
    g.insert(road(9, 10, 0x0003_0100));
    let mut p = set_of(&[c]);
    assign_edge_subtypes(&mut g, &mut p, &mut r, true);
    assert_eq!(g.get(c).unwrap().edges[0], 0x0002_0009);
    // Q pads P by one cell, so (9, 10) is visited but not in P: unchanged.
    assert_eq!(g.get(n).unwrap().edges[0], 0x0003_0100);
    // Not purely diagonal: the stub becomes orthogonal.
    let mut g = grid();
    g.insert(road(10, 10, 0x0002_0001));
    g.insert(road(9, 10, 0x0002_0000));
    let mut p = set_of(&[c, n]);
    assign_edge_subtypes(&mut g, &mut p, &mut r, true);
    assert_eq!(g.get(c).unwrap().edges[0], 0x0002_0002);
    // The reset clears old subtypes first (cells of P only).
    let mut g = grid();
    g.insert(road(10, 10, 0x0002_0019));
    let mut p = set_of(&[c]);
    assign_edge_subtypes(&mut g, &mut p, &mut r, true);
    assert_eq!(g.get(c).unwrap().edges[0], 0x0002_0001);
    // A fixed cell next to a purely diagonal neighbour fails.
    let mut g = grid();
    g.insert(CellInfo {
        fixed: true,
        ..road(10, 10, 0x0002_0001)
    });
    g.insert(road(9, 10, 0x0003_0100));
    let mut p = set_of(&[c]);
    let mut r = StlHashSet::with_hint(100);
    assign_edge_subtypes(&mut g, &mut p, &mut r, true);
    assert_eq!(r.iter_order(), vec![c]);
}

// ---- test 17: tile subsets ------------------------------------------------------------------------------------

#[test]
fn t15_17_patch_tile_pair() {
    let mut rules = NetworkRules::new();
    rules.load_rule_overrides(b"[RuleOverrides]\n0x10,1,0,0x20,1,0=0x11,1,0,0x21,1,0\n");
    let mut a = TilePiece {
        id: 0x10,
        orientation: 0,
    };
    let mut b = TilePiece {
        id: 0x20,
        orientation: 0,
    };
    assert_eq!(patch_tile_pair(&rules, &mut a, &mut b, 1), 1);
    assert_eq!(
        (a, b),
        (
            TilePiece {
                id: 0x11,
                orientation: 0
            },
            TilePiece {
                id: 0x21,
                orientation: 0
            }
        )
    );
    // Another side does not match.
    let mut a = TilePiece {
        id: 0x10,
        orientation: 0,
    };
    let mut b = TilePiece {
        id: 0x20,
        orientation: 0,
    };
    assert_eq!(patch_tile_pair(&rules, &mut a, &mut b, 2), 0);

    // The hard conflict.
    let mut hard = NetworkRules::new();
    hard.load_rule_overrides(b"[RuleOverrides]\n0x10,1,0,0x20,1,0=0,0,0,0,0,0\n");
    let mut a = TilePiece {
        id: 0x10,
        orientation: 0,
    };
    let mut b = TilePiece {
        id: 0x20,
        orientation: 0,
    };
    assert_eq!(patch_tile_pair(&hard, &mut a, &mut b, 1), 2);

    let setup = || {
        let mut g = grid();
        g.insert(road(10, 10, 0x0200_0200));
        g.insert(road(10, 9, 0x0200_0200));
        let mut st = SolveState::new();
        st.append_solved(
            &mut g,
            SolvedRecord {
                id: 0x10,
                orientation: 0,
                key: cell_key(10, 10),
            },
        );
        st.append_solved(
            &mut g,
            SolvedRecord {
                id: 0x20,
                orientation: 0,
                key: cell_key(10, 9),
            },
        );
        (g, st)
    };
    let (mut g, mut st) = setup();
    assert!(!adjust_tile_subsets(&hard, &mut g, &mut st, false));
    // Resolve mode: A goes back into S, its +0xB8 stays (quirk 5).
    let (mut g, mut st) = setup();
    assert!(adjust_tile_subsets(&hard, &mut g, &mut st, true));
    assert_eq!(st.solved.len(), 1);
    assert_eq!(st.solved[0].key, cell_key(10, 9));
    assert!(st.s.contains(cell_key(10, 10)));
    assert_eq!(g.at(10, 10).unwrap().solved, 0);
    assert_eq!(g.at(10, 9).unwrap().solved, 0);
    // The patch through AdjustTileSubsets.
    let (mut g, mut st) = setup();
    assert!(adjust_tile_subsets(&rules, &mut g, &mut st, false));
    assert_eq!((st.solved[0].id, st.solved[1].id), (0x11, 0x21));
}

#[test]
fn patch_against_an_existing_piece_and_mirrored_patterns() {
    let mut rules = NetworkRules::new();
    // A pattern written flipped matches the mirrored pair.
    rules.load_rule_overrides(b"[RuleOverrides]\n0x10,3,1,0x20,3,1=0x11,3,1,0x21,1,1\n");
    let mut a = TilePiece {
        id: 0x10,
        orientation: 0,
    };
    let mut b = TilePiece {
        id: 0x20,
        orientation: 0,
    };
    // s = 1: nA = nB = 1; mirrored 1 ^ 0x82 = 0x83.
    assert_eq!(patch_tile_pair(&rules, &mut a, &mut b, 1), 1);
    // 0x83 ^ 0x82 = 1 -> denormalised (1 + 1 + 2) & 3 = 0; 0x81 ^ 0x82 = 3 -> 2.
    assert_eq!((a.id, a.orientation, b.id, b.orientation), (0x11, 0, 0x21, 2));

    // An existing neighbour (an occupant) is appended when patched.
    let mut rules = NetworkRules::new();
    rules.load_rule_overrides(b"[RuleOverrides]\n0x10,1,0,0x20,1,0=0x11,1,0,0x21,1,0\n");
    let mut g = grid();
    g.insert(road(10, 10, 0));
    g.insert(CellInfo {
        occupant: Some(OccupantView {
            piece_id: 0x20,
            ..OccupantView::default()
        }),
        ..road(10, 9, 0)
    });
    let mut st = SolveState::new();
    st.append_solved(
        &mut g,
        SolvedRecord {
            id: 0x10,
            orientation: 0,
            key: cell_key(10, 10),
        },
    );
    assert!(adjust_tile_subsets(&rules, &mut g, &mut st, false));
    assert_eq!(st.solved.len(), 2);
    assert_eq!(
        st.solved[1],
        SolvedRecord {
            id: 0x21,
            orientation: 0,
            key: cell_key(10, 9)
        }
    );
    assert_eq!(g.at(10, 9).unwrap().solved, 1);
}

// ---- tests 18-20, 22: cIntRule -----------------------------------------------------------------------------

fn int_rules(text: &str) -> IntRuleMap {
    let mut m = IntRuleMap::new();
    m.load(&ini_sections(text.as_bytes()));
    m
}

#[test]
fn t15_18_rotate_bounds() {
    let m =
        int_rules("[HighwayIntersectionInfo_0x100]\nCheckType=a - Road:0x00020002\nCellLayout=aaaa<\nCellLayout=.^\n");
    let r = m.get(0x100).unwrap();
    assert_eq!(r.occupied, vec![(-1, 0), (0, 0), (1, 0), (2, 0)]);
    assert_eq!(
        r.bounds,
        Bounds {
            min_x: -1,
            max_x: 2,
            min_z: 0,
            max_z: 0
        }
    );
    let mut r = r.clone();
    r.rotate(1);
    assert_eq!(r.occupied, vec![(0, -1), (0, 0), (0, 1), (0, 2)]);
    assert_eq!(
        r.bounds,
        Bounds {
            min_x: 0,
            max_x: 1,
            min_z: -1,
            max_z: 2
        }
    );
    // A half turn is exact.
    let mut h = m.get(0x100).unwrap().clone();
    h.rotate(2);
    assert_eq!(
        h.bounds,
        Bounds {
            min_x: -2,
            max_x: 1,
            min_z: 0,
            max_z: 0
        }
    );
    // The check type's edges turn with it.
    assert_eq!(r.check_types[&b'a'].edges1, 0x0200_0200);
}

#[test]
fn t15_19_layout_and_centre() {
    let m =
        int_rules("[HighwayIntersectionInfo_0x5000]\nCheckType=a - Road:0x00020002\nCellLayout=.a+<\nCellLayout=..^\n");
    let r = m.get(0x5000).unwrap();
    assert_eq!(r.tagged, vec![TaggedCell { x: -1, z: 0, tag: b'a' }]);
    assert_eq!(r.new_cells, vec![(0, 0)]);
    assert_eq!(
        r.bounds,
        Bounds {
            min_x: -1,
            max_x: 0,
            min_z: 0,
            max_z: 0
        }
    );
    assert_eq!(r.type_mask, 1);
    // Without a centre mark the cells shift by +1 (quirk 6).
    let m = int_rules("[HighwayIntersectionInfo_0x5001]\nCheckType=a - Road:0x00020002\nCellLayout=a\n");
    assert_eq!(m.get(0x5001).unwrap().tagged[0], TaggedCell { x: 1, z: 1, tag: b'a' });
}

fn test_rule(extra: &str) -> IntRule {
    let m = int_rules(&format!(
        "[HighwayIntersectionInfo_0x7]\nCheckType=a - Road:0x00020002 {extra}\nCellLayout=a<\nCellLayout=^\n"
    ));
    m.get(7).unwrap().clone()
}

#[test]
fn t15_20_test() {
    let r = test_rule("");
    assert_eq!(r.tagged, vec![TaggedCell { x: 0, z: 0, tag: b'a' }]);
    let at = |c: Option<CellInfo>, r: &IntRule| {
        let mut g = grid();
        if let Some(c) = c {
            g.insert(c);
        }
        r.test(10, 10, &g, &DryHost)
    };
    assert_eq!(at(Some(road(10, 10, 0x0002_0002)), &r), 0);
    assert_eq!(at(Some(road(10, 10, 0x0200_0200)), &r), 2);
    assert_eq!(at(Some(CellInfo::new(10, 10).with_type(RAIL, 0x0002_0002)), &r), 1);
    let both = road(10, 10, 0x0002_0002).with_type(STREET, 0x0200_0200);
    assert_eq!(at(Some(both), &r), 1);
    assert_eq!(at(Some(both), &test_rule("multiok")), 0);
    assert_eq!(at(None, &r), 2);
    // An intersection piece without ReplacementIntersection.
    let inter = CellInfo {
        intersection: true,
        ..road(10, 10, 0x0002_0002)
    };
    assert_eq!(at(Some(inter), &r), 1);
    // A fixed cell.
    let fixed = CellInfo {
        fixed: true,
        ..road(10, 10, 0x0002_0002)
    };
    assert_eq!(at(Some(fixed), &r), 1);
    // `check` does not occupy: a fixed cell passes.
    assert_eq!(at(Some(fixed), &test_rule("check")), 0);
}

#[test]
fn test_new_cells_and_replacement() {
    struct Wet;
    impl IntRuleHost for Wet {
        fn is_water(&self, x: i32, _z: i32) -> bool {
            x == 11
        }
    }
    struct Refuse;
    impl IntRuleHost for Refuse {
        fn demolition_region_ok(&self, cells: &[(i32, i32)]) -> bool {
            cells.is_empty()
        }
    }
    let m = int_rules("[HighwayIntersectionInfo_0x8]\nCheckType=a - Road:0x00020002\nCellLayout=a+<\nCellLayout=^\nReplacementIntersection=0,0\n");
    let r = m.get(8).unwrap();
    assert!(r.replacement);
    let mut g = grid();
    g.insert(road(10, 10, 0x0002_0002));
    assert_eq!(r.test(10, 10, &g, &DryHost), 0);
    assert_eq!(r.test(10, 10, &g, &Wet), 1);
    assert_eq!(r.test(10, 10, &g, &Refuse), 1);
    // An intersection occupant whose anchor is elsewhere.
    g.insert(CellInfo {
        occupant: Some(OccupantView {
            network_flags: INTERSECTION_FLAG | 1,
            anchor: Some((3, 3)),
            ..OccupantView::default()
        }),
        ..road(10, 10, 0x0002_0002)
    });
    assert_eq!(r.test(10, 10, &g, &DryHost), 1);
    g.at_mut(10, 10).unwrap().occupant.as_mut().unwrap().anchor = Some((10, 10));
    assert_eq!(r.test(10, 10, &g, &DryHost), 0);
}

#[test]
fn t15_22_one_way_dir() {
    let base = int_rules("[HighwayIntersectionInfo_0x9]\nOneWayDir=1\n")
        .get(9)
        .unwrap()
        .clone();
    assert_eq!(base.one_way_dir, 1);
    let mut r = base.clone();
    r.rotate(1);
    assert_eq!(r.one_way_dir, 3);
    let mut t = base.clone();
    t.transpose();
    assert_eq!(t.one_way_dir, 5);
    // Through the file, in key order.
    let m = int_rules("[HighwayIntersectionInfo_0xA]\nOneWayDir=1\nRotate=1\n");
    assert_eq!(m.get(0xA).unwrap().one_way_dir, 3);
}

#[test]
fn t15_23_identify_highway_intersections() {
    let rules = int_rules("[HighwayIntersectionInfo_0x77]\nCheckType=a - Road:0x00020002\nCellLayout=a<\nCellLayout=^\nAutoPlace=1\nStepOffsets=1,2\n");
    let occ = Some(OccupantView::default());
    let make = || {
        let mut g = grid();
        for x in 10..13 {
            g.insert(CellInfo {
                occupant: occ,
                ..road(x, 10, 0x0002_0002)
            });
        }
        g
    };
    let steps: Vec<Vec<(i32, i32)>> = (10..13).map(|x| vec![(x, 10)]).collect();
    let mut g = make();
    let mut st = SolveState::new();
    assert_eq!(
        identify_highway_intersections(&rules, &mut g, &mut st, &steps, 3, 0, &DryHost),
        3
    );
    assert_eq!(st.potential, vec![(10, 10, 4)]);
    assert_eq!(st.intersections.len(), 1);
    let rec = &st.intersections[0];
    assert_eq!(
        (rec.rule_id, rec.x, rec.z, rec.start_step, rec.step_a, rec.step_b),
        (0x77, 10, 10, 0, 1, 2)
    );
    assert!(g.at(10, 10).unwrap().intersection);
    // v is returned whatever happens.
    let mut st = SolveState::new();
    assert_eq!(
        identify_highway_intersections(&rules, &mut make(), &mut st, &steps, 7, 0, &DryHost),
        7
    );
    assert_eq!(
        identify_highway_intersections(&rules, &mut make(), &mut st, &steps, 0, 0, &DryHost),
        0
    );
    // A run ended by a step without a match carries the type.
    let mut g = make();
    g.at_mut(11, 10).unwrap().occupant = None;
    let mut st = SolveState::new();
    identify_highway_intersections(&rules, &mut g, &mut st, &steps, 3, 6, &DryHost);
    assert_eq!(st.potential, vec![(10, 10, 6), (12, 10, 4)]);
}

// ---- test 24: tile verifiers ------------------------------------------------------------------------------------

#[test]
fn t15_24_tile_verifiers() {
    let mut rules = NetworkRules::new();
    rules.load_tile_verifiers(b"[TileVerifiers]\n0x100=5,0x101,0,0\n");
    assert_eq!(
        rules.verifiers[&0x100],
        vec![vec![Requirement {
            id: 0x101,
            orientation: 0,
            m: 5
        }]]
    );
    let run = |neighbour: Option<(u32, u8)>| {
        let mut g = grid();
        g.insert(road(10, 10, 0));
        g.insert(road(10, 11, 0));
        let mut st = SolveState::new();
        st.append_solved(
            &mut g,
            SolvedRecord {
                id: 0x100,
                orientation: 0x01,
                key: cell_key(10, 10),
            },
        );
        if let Some((id, o)) = neighbour {
            st.append_solved(
                &mut g,
                SolvedRecord {
                    id,
                    orientation: o,
                    key: cell_key(10, 11),
                },
            );
        }
        let ok = verify_tile_subsets(&rules, &mut g, &mut st);
        (ok, st)
    };
    let (ok, st) = run(Some((0x101, 0x01)));
    assert!(ok && st.solved.len() == 2);
    let (ok, st) = run(Some((0x101, 0x00)));
    assert!(!ok);
    assert_eq!(st.solved.len(), 1);
    assert!(st.s.contains(cell_key(10, 10)));
    let (ok, _) = run(None);
    assert!(!ok);
}

// ---- other checks ---------------------------------------------------------------------------------------------

#[test]
fn vocabulary_helpers() {
    assert_eq!(cell_key(5, 6), 0x0006_0005);
    assert_eq!(key_cell(0x0006_0005), (5, 6));
    assert_eq!(network_name_to_id("  lightrail "), Some(LIGHT_RAIL));
    assert_eq!(network_name_to_id("Tram"), None);
    assert_eq!(type_of(0), 12);
    assert_eq!(type_of(0x8 | 0x2), 1);
    assert_eq!(connmask(0x0B00_0402), 0x0300_0003);
    assert!(covers(0x0002_0002, 0x0000_0002));
    assert!(!covers(0x0002_0002, 0x0200_0000));
    assert_eq!(d_mask(0x0000_0201), 0b1111);
    assert_eq!(d_mask(0x0200_0000), 0b1100_0000);
    assert_eq!(d_mask(0x0404_0404), 0);
    assert!(within(0x0000_0002, 0x0003_0002));
    assert!(!within(0x0000_0002, 0x0000_0003));
    assert!(within(0, 0xFFFF_FFFF));
    assert_eq!(orientation(3, 1), 0x83);
    assert_eq!(orientation(5, 0), 5);
    assert_eq!(meta_offset(25), (0, 0));
    assert_eq!(tile_instances(ROAD), &[0x8, 0x7]);
    assert!(tile_instances(POWER_LINE).is_empty());
    assert_eq!(tile_instance_owner(0x15), Some((MONORAIL, 1)));
    assert_eq!(rotate_edges(0x0200_0200, 1), 0x0002_0002);
}

#[test]
fn number_readers_and_scan() {
    assert_eq!(atol("  -12x"), -12);
    assert_eq!(atol("x"), 0);
    assert_eq!(rz_convert_u32("0x1A"), 0x1A);
    assert_eq!(rz_convert_u32("26"), 26);
    assert_eq!(rz_atou32_hex(" 0X123456789"), 0x1234_5678);
    assert_eq!(rz_atou32_hex("ab"), 0xAB);
    let v = scan(" %i , %i , %d , %i , %i , %d", "0x10, 010 ,3,4 ,5 , 6");
    assert_eq!(v.len(), 6);
    assert_eq!(v[0], ScanValue::Int(16));
    assert_eq!(v[1], ScanValue::Int(8));
    assert_eq!(scan("%d,%d", "1, 2").len(), 2);
    assert_eq!(scan("%d,%d", "1 ,2").len(), 1);
    let p = scan("%f , %f , %i , %i , 0x%x", "1.5, -2 , 1, 0, 0x5D4A0000");
    assert_eq!(p.len(), 5);
    assert_eq!(p[0], ScanValue::Float(1.5));
    assert_eq!(p[1].as_f32(), -2.0);
    assert_eq!(p[4].as_i32() as u32, 0x5D4A_0000);
}

#[test]
fn ini_reader_choices() {
    let s = ini_sections(b"; c\r\nk=0\r\n[One]\r\n a = 1 \r\nno equals\r\n[one]\r\nb=2=3\r\n");
    assert_eq!(s.len(), 2);
    assert_eq!(s[0].entries, vec![("a".to_string(), "1".to_string())]);
    let both = find_section(&s, "ONE");
    assert_eq!(both.len(), 2);
    assert_eq!(both[1], &("b".to_string(), "2=3".to_string()));
}

#[test]
fn tile_reader_edges() {
    // CRLF, comments, a rule ended by the next one with no outputs.
    let ev = Evaluator::from_bytes(b"#R#\r\n; comment\r\n1,0,2,0,2\r\n1,2,0,2,0\r\n3,0,0x10,0,0\r\n");
    assert_eq!(ev.rule_count(), 2);
    let blocked: Vec<&TileRule> = ev.rules_for(0x0200_0200).iter().collect();
    assert!(blocked[0].outputs.is_empty());
    // A rule with no outputs at the end of the file is dropped.
    let ev = Evaluator::from_bytes(b"#R#\n1,0,2,0,2\n3,0,0x10,0,0\n1,2,0,2,0\n2,0,2,0,2,0\n");
    assert_eq!(ev.rule_count(), 1);
    // A blocking rule with no outputs matches first and produces nothing.
    let ev = Evaluator::from_bytes(b"#R#\n1,0,2,0,2\n1,0,2,0,2\n3,0,0x10,0,0\n");
    let mut g = grid();
    g.insert(road(1, 1, 0x0200_0200));
    assert_eq!(eval_at(&ev, &g, 1, 1, ROAD), Some(vec![]));
    // Rules of one key keep file order across files of one evaluator.
    let mut ev = Evaluator::from_bytes(b"#R#\n1,0,2,0,2\n3,0,0x1,0,0\n");
    ev.add_rules(b"#R#\n1,0,2,0,2\n3,0,0x2,0,0\n");
    assert_eq!(
        rules_of(&ev),
        vec![(0x0200_0200, vec![(1, 0)]), (0x0200_0200, vec![(2, 0)])]
    );
    // Edge tokens with subtypes in the centre.
    let ev = Evaluator::from_bytes(b"#R#\n1,13,0,27,0\n3,4,0x1,1,1\n");
    let r = ev.rules().next().unwrap();
    assert_eq!(r.key, 0x0017_000B);
    assert_eq!(
        r.outputs[0],
        Output {
            id: 1,
            m: 4,
            orientation: 0x81
        }
    );
}

#[test]
fn transmogrify_moves_conditions_and_outputs() {
    let ev = Evaluator::from_bytes(b"#Transmogrify\nnoflip\n1,2,0,0,0\n2,1,0,0,2,0\n3,1,0x9,0,0\n");
    let rules: Vec<&TileRule> = ev.rules().collect();
    assert_eq!(rules.len(), 4);
    let turned = ev.rules_for(0x0000_0200)[0].clone();
    assert_eq!(turned.conditions[0].m, 3);
    assert_eq!(turned.conditions[0].value, 0x0002_0000 << 8);
    assert_eq!(turned.outputs[0].m, 3);
    assert_eq!(turned.outputs[0].orientation, 1);
}

#[test]
fn connection_node_qualification() {
    let ev = Evaluator::from_bytes(b"#T#\n1,0,2,0,2\n3,0,0x100,0,0");
    let key = cell_key(5, 5);
    let run = |occ: Option<OccupantView>, flags: u32| {
        let mut g = grid();
        g.insert(CellInfo {
            node: true,
            flags,
            occupant: occ,
            ..road(5, 5, 0x0200_0200)
        });
        let mut st = SolveState::new();
        st.s = set_of(&[key]);
        let mut p = set_of(&[key]);
        let mut w = StlHashSet::with_hint(100);
        apply_network_rules(&mut g, &mut st, &mut p, &mut w, &ev, ROAD);
        st.solved.len()
    };
    let occ = OccupantView {
        network_flags: 1,
        ..OccupantView::default()
    };
    assert_eq!(run(Some(occ), 1), 1);
    // A non-type flag in the cell's word never qualifies (quirk 3).
    assert_eq!(run(Some(occ), 1 | FLAG_PORTAL), 0);
    // Slot 13's edge types bound the cell's edges.
    let narrow = OccupantView {
        node_edge_types: Some((0x0000_0200, 0x0200_0200)),
        ..occ
    };
    assert_eq!(run(Some(narrow), 1), 1);
    let wrong = OccupantView {
        node_edge_types: Some((0x0000_0300, 0x0200_0200)),
        ..occ
    };
    assert_eq!(run(Some(wrong), 1), 0);
    assert_eq!(run(None, 1), 0);
}

#[test]
fn padded_set_and_unsolved_rule() {
    let mut g = grid();
    g.insert(road(0, 0, 0));
    g.insert(CellInfo {
        fixed: true,
        ..road(1, 0, 0)
    });
    g.insert(road(2, 2, 0));
    g.insert(road(3, 3, 0));
    let p = build_padded_network_set(&g, &set_of(&[cell_key(0, 0)]), 2);
    let mut o = p.iter_order();
    o.sort_unstable();
    assert_eq!(o, vec![cell_key(0, 0), cell_key(2, 2)]);
    // The fixed source still goes in.
    let p = build_padded_network_set(&g, &set_of(&[cell_key(1, 0)]), 0);
    assert_eq!(p.iter_order(), vec![cell_key(1, 0)]);

    let mut st = SolveState::new();
    st.valid_count = 5;
    assert_eq!(unsolved_cell_result(&g, &st), 5);
    g.insert(CellInfo {
        in_run: true,
        step: 3,
        ..road(4, 4, 0)
    });
    st.s = set_of(&[cell_key(4, 4)]);
    assert_eq!(unsolved_cell_result(&g, &st), 3);
    st.s.insert(cell_key(2, 2));
    assert_eq!(unsolved_cell_result(&g, &st), 2);
}

#[test]
fn check_type_grammar() {
    let (t, ct) = CheckType::parse("b - Road:0x00020002,0x00FF00FF OneWayRoad:0x02000200 check multiok # c").unwrap();
    assert_eq!(t, b'b');
    assert_eq!((ct.type1, ct.edges1, ct.mask1), (0, 0x0002_0002, 0x00FF_00FF));
    assert_eq!((ct.type2, ct.edges2, ct.mask2), (10, 0x0200_0200, 0xFFFF_FFFF));
    assert!(ct.multi_ok && !ct.occupies && !ct.optional);
    let (_, o) = CheckType::parse("c - Highway:0x02000200 optional; Road:1").unwrap();
    assert!(o.optional && !o.occupies);
    assert_eq!(o.type2, -1);
    assert_eq!(CheckType::parse("ab - Road:2").unwrap_err(), "Bad map tag");
    assert_eq!(
        CheckType::parse("a - Road:2 Rail:2 Street:2").unwrap_err(),
        "Too many network checks in one rule"
    );
    let m = int_rules("[HighwayIntersectionInfo_0x1]\nCheckType=ab - Road:2\n");
    assert_eq!(m.failures.len(), 1);
}

#[test]
fn copy_transpose_translate_and_rings() {
    let text = "[HighwayIntersectionInfo_0x1]\nCheckType=a - Road:0x00000301,0x0000FFFF\nCellLayout=a+<\nCellLayout=^\nPiece=1.5, 2.0, 1, 0, 0x55\nHandleOffset=1,2\n\
[HighwayIntersectionInfo_0x2]\nCopyFrom=0x1\nTranspose=1\nTranslate=3,4\n\
[HighwayIntersectionInfo_0x3]\nCopyFrom=0x1\nRotate=3\n\
[OrderingA]\nRotationRing=0x1,0x2,0x99,0x3\n\
[OrderingB]\nAddTypes=0x2,0x3\n";
    let m = int_rules(text);
    assert_eq!(m.len(), 3);
    let a = m.get(1).unwrap();
    assert_eq!(
        a.models,
        vec![Model {
            id: 0x55,
            rot: 1,
            flip: 0,
            x: 1.5,
            z: 2.0
        }]
    );
    let b = m.get(2).unwrap();
    assert_eq!(b.id, 2);
    // Transposed then translated.
    assert_eq!(b.tagged[0], TaggedCell { x: 3, z: 4, tag: b'a' });
    assert_eq!(b.new_cells, vec![(3, 5)]);
    assert_eq!(b.handle_offset, (5, 5));
    assert_eq!(
        b.models[0],
        Model {
            id: 0x55,
            rot: 2,
            flip: 1,
            x: 5.0,
            z: 5.5
        }
    );
    // W kind 1, N kind 3: swapped sides (0x0103), then 4 - value.
    assert_eq!(b.check_types[&b'a'].edges1, 0x0000_0301);
    assert_eq!(b.check_types[&b'a'].mask1, 0x0000_FFFF);
    let c = m.get(3).unwrap();
    assert_eq!(c.new_cells, vec![(0, -1)]);
    assert_eq!(c.models[0].rot, 0);
    // The rings.
    assert_eq!((a.ring_prev, a.ring_next), (3, 2));
    assert_eq!((b.ring_prev, b.ring_next), (1, 3));
    assert_eq!((c.ring_prev, c.ring_next), (2, 1));
    assert_eq!((b.add_prev, b.add_next, c.add_prev, c.add_next), (3, 3, 2, 2));
    assert_eq!((a.add_prev, a.add_next), (1, 1));
    // A transposed subtype byte is left as it is (quirk 7).
    let mut r = IntRule::new(9);
    r.add_check_type(b'x', CheckType::parse("x - Road:0x00000009").unwrap().1);
    r.transpose();
    assert_eq!(r.check_types[&b'x'].edges1, 0x0000_0900);
}

#[test]
fn auto_complete_and_primary_type() {
    let r = test_rule("");
    assert_eq!(r.primary_type(), ROAD);
    assert_eq!(IntRule::new(1).primary_type(), HIGHWAY);
    let mut g = grid();
    let mut st = SolveState::new();
    assert_eq!(r.do_auto_complete(10, 10, &g, &mut st), 1);
    assert_eq!(
        st.extra_cells,
        vec![ExtraCell {
            x: 10,
            z: 10,
            edges: 0x0002_0002,
            network_type: ROAD,
            one_way: 0,
            one_way_mask: 0
        }]
    );
    g.insert(road(10, 10, 0x0002_0002));
    let mut st = SolveState::new();
    r.do_auto_complete(10, 10, &g, &mut st);
    assert!(st.extra_cells.is_empty());
    // A one-way check: the pattern's bits on the arm's sides.
    let m = int_rules("[HighwayIntersectionInfo_0x4]\nCheckType=a - OneWayRoad:0x00020002\nCellLayout=a<\nCellLayout=^\nOneWayDir=1\n");
    let r = m.get(4).unwrap();
    let mut st = SolveState::new();
    r.do_auto_complete(0, 0, &grid(), &mut st);
    let d = d_mask(0x0002_0002);
    assert_eq!(
        (st.extra_cells[0].one_way, st.extra_cells[0].one_way_mask),
        (0xAA & d, d)
    );
}

#[test]
fn within_bounds() {
    let r = test_rule("");
    assert!(r.is_within_bounds(0, 0, 64, 64));
    assert!(r.is_within_bounds(63, 63, 64, 64));
    assert!(!r.is_within_bounds(64, 0, 64, 64));
    assert!(!r.is_within_bounds(-1, 0, 64, 64));
}

#[test]
fn reload_follows_the_load_order() {
    let mut asked = Vec::new();
    let (rules, reports) = NetworkRules::reload(&mut |id| {
        asked.push(id);
        match id {
            ResourceId::Rule(0x8) => Some(b"#R#\n1,0,2,0,2\n3,0,0x8,0,0".to_vec()),
            ResourceId::Rule(0x7) => Some(b"#R#\n1,0,2,0,2\n3,0,0x7,0,0".to_vec()),
            ResourceId::NetworkIni => Some(b"[TileVerifiers]\n0x8=0,0x8,0,0\n".to_vec()),
            _ => None,
        }
    });
    assert_eq!(asked.len(), 24 + 4);
    assert_eq!(asked[0], ResourceId::Rule(0x8));
    assert_eq!(asked[24], ResourceId::NetworkIni);
    assert_eq!(asked[27], ResourceId::Rule(INTERSECTION_INSTANCE));
    assert_eq!(reports.len(), 28);
    assert_eq!(rules.evaluator_counts(), [2, 2, 2, 2, 2, 0, 2, 2, 2, 2, 2, 2, 2]);
    let ev = &rules.evaluators[ROAD as usize];
    assert_eq!(ev[0].rules().next().unwrap().outputs[0].id, 0x8);
    assert_eq!(ev[1].rules().next().unwrap().outputs[0].id, 0x7);
    assert_eq!(rules.verifiers.len(), 1);
}

#[test]
fn solve_runs_every_type_and_keeps_claimed_padding() {
    // A Rail evaluator solves a Rail cell even when the tool type is Road
    // (Ã‚Â§6.1 step 5: all 13 types).
    let mut rules = NetworkRules::new();
    rules.add_tile_file(RAIL, Some(b"#R#\n1,0,2,0,2\n3,0,0x40,0,0\n3,7,0x41,0,0"));
    let mut g = grid();
    g.insert(CellInfo {
        in_run: true,
        ..CellInfo::new(5, 5).with_type(RAIL, 0x0200_0200)
    });
    // A padding cell below it: claimed (leaves P), but gets no record.
    g.insert(CellInfo::new(5, 6).with_type(RAIL, 0x0000_0200));
    let mut st = SolveState::new();
    st.s = set_of(&[cell_key(5, 5)]);
    st.valid_count = 1;
    assert_eq!(solve_network(&rules, &mut g, &mut st, ROAD, false), 1);
    assert_eq!(
        st.solved,
        vec![SolvedRecord {
            id: 0x40,
            orientation: 0,
            key: cell_key(5, 5)
        }]
    );
    assert_eq!(g.at(5, 6).unwrap().solved, -1);
}
