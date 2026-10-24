// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network rules through the C ABI (docs/specs/network-rules.md).

use crate::network_rules::*;

fn cell(x: i32, z: i32) -> fc4_network_rules_cell {
    fc4_network_rules_cell {
        struct_size: std::mem::size_of::<fc4_network_rules_cell>() as u32,
        x,
        z,
        ..Default::default()
    }
}

#[test]
fn detached_capture_distinguishes_missing_no_match_and_empty_match() {
    unsafe {
        let r = fc4_network_rules_create();
        load(r, 8, "#T#\n1,0,2,0,0\n2,3,0,0,0,2\n3,0,0x200,0,0\n");
        let mut capture = fc4_network_rules_capture {
            centre: 0x200,
            edges: [0; 256],
            edge_known: [0; 256],
            validity: [0; 256],
        };
        let mut out = [fc4_network_rules_output::default(); 1];
        assert_eq!(fc4_network_rules_replay(r, 0, 0, &capture, out.as_mut_ptr(), 1), -2);
        capture.edges[3] = 0x0200_0000;
        capture.edge_known[3] = 1;
        assert_eq!(fc4_network_rules_replay(r, 0, 0, &capture, out.as_mut_ptr(), 1), -2);
        capture.validity[0] = 2;
        assert_eq!(fc4_network_rules_replay(r, 0, 0, &capture, out.as_mut_ptr(), 1), 1);
        assert_eq!(out[0].id, 0x200);
        capture.validity[0] = 1;
        assert_eq!(fc4_network_rules_replay(r, 0, 0, &capture, out.as_mut_ptr(), 1), -1);
        // A failing condition short circuits before uncaptured validity.
        capture.edges[3] = 0;
        capture.validity[0] = 0;
        assert_eq!(fc4_network_rules_replay(r, 0, 0, &capture, out.as_mut_ptr(), 1), -1);
        load(r, 7, "#T#\n1,0,2,0,0\n1,0,0,0,0\n");
        assert_eq!(fc4_network_rules_replay(r, 0, 1, &capture, out.as_mut_ptr(), 1), 0);
        // NR16 snapshot excludes capacities/padding and retains duplicate keys.
        let size = fc4_network_rules_snapshot(r, 0, 0, std::ptr::null_mut(), 0);
        let mut words = vec![0; size as usize];
        assert_eq!(fc4_network_rules_snapshot(r, 0, 0, words.as_mut_ptr(), size), size);
        assert_eq!(words, [0x200, 0xFF, 1, 1, 3, 0x0200_0000, u32::MAX, 0x200, 0, 0]);
        assert_eq!(fc4_network_rules_replay(r, 99, 0, &capture, out.as_mut_ptr(), 1), -3);
        fc4_network_rules_destroy(r);
    }
}

fn road(x: i32, z: i32, e: u32) -> fc4_network_rules_cell {
    let mut c = cell(x, z);
    c.flags = 1;
    c.edges[0] = e;
    c
}

unsafe fn load(r: *mut fc4_network_rules, inst: u32, text: &str) -> fc4_network_rules_report {
    let mut rep = fc4_network_rules_report::default();
    assert!(fc4_network_rules_load(r, inst, text.as_ptr(), text.len(), &mut rep));
    rep
}

#[test]
fn null_arguments() {
    unsafe {
        assert!(!fc4_network_rules_load(
            std::ptr::null_mut(),
            8,
            std::ptr::null(),
            0,
            std::ptr::null_mut()
        ));
        assert_eq!(fc4_network_rules_evaluator_count(std::ptr::null(), 0), 0);
        assert_eq!(
            fc4_network_rules_evaluate(std::ptr::null(), 0, 0, std::ptr::null(), 0, 0, std::ptr::null_mut(), 0),
            -1
        );
        let mut v = 5;
        assert!(!fc4_network_rules_solve(
            std::ptr::null(),
            std::ptr::null_mut(),
            0,
            false,
            &mut v
        ));
        assert_eq!(v, 5);
        assert!(!fc4_network_rules_grid_set_cell(std::ptr::null_mut(), std::ptr::null()));
        fc4_network_rules_destroy(std::ptr::null_mut());
        fc4_network_rules_grid_destroy(std::ptr::null_mut());
        // A short cell is refused.
        let g = fc4_network_rules_grid_create(64, 64);
        let mut c = road(1, 1, 0);
        c.struct_size = 8;
        assert!(!fc4_network_rules_grid_set_cell(g, &c));
        fc4_network_rules_grid_destroy(g);
    }
}

#[test]
fn tile_instances_and_load_reports() {
    unsafe {
        let mut inst = [0u32; 4];
        assert_eq!(fc4_network_rules_tile_instances(0, inst.as_mut_ptr(), 4), 2);
        assert_eq!(&inst[..2], &[0x8, 0x7]);
        assert_eq!(fc4_network_rules_tile_instances(5, inst.as_mut_ptr(), 4), 0);
        assert_eq!(fc4_network_rules_tile_instances(13, inst.as_mut_ptr(), 4), 0);

        let r = fc4_network_rules_create();
        let rep = load(r, 0x8, "#Transmogrify\n1,0,2,0,2\n3,0,0x100,0,0");
        assert_eq!(rep.struct_size, 36);
        assert_eq!(
            (
                rep.status,
                rep.transmogrify,
                rep.source_rules,
                rep.rules,
                rep.dropped_variants
            ),
            (1, 1, 1, 2, 6)
        );
        // A missing file: the evaluator still joins the list.
        let mut rep = fc4_network_rules_report::default();
        assert!(fc4_network_rules_load(r, 0x7, std::ptr::null(), 0, &mut rep));
        assert_eq!(rep.status, 0);
        assert_eq!(fc4_network_rules_evaluator_count(r, 0), 2);
        assert_eq!(fc4_network_rules_rule_count(r, 0, 0), 2);
        assert_eq!(fc4_network_rules_rule_count(r, 0, 1), 0);
        let rep = load(r, 0x6, "TestRules\n1,0,2,0,2\n3,0,0x100,0,0");
        assert_eq!(rep.status, 2);
        assert_eq!(fc4_network_rules_evaluator_count(r, 1), 1);
        // The INI resources.
        let rep = load(r, 0x1000_0001, "[AllowedOverrides]\nk=Road,Street\n");
        assert_eq!((rep.status, rep.entries), (-1, 1));
        let rep = load(
            r,
            0x1000_0000,
            "[HighwayIntersectionInfo_0x1]\nCheckType=a - Road:0x00020002\nCellLayout=a<\nCellLayout=^\n",
        );
        assert_eq!(rep.rules, 1);
        assert_eq!(fc4_network_rules_intersection_count(r), 1);
        let rep = load(r, 0x8A59_93B9, "[TileVerifiers]\n0x100=5,0x101,0,0\n");
        assert_eq!(rep.entries, 1);
        fc4_network_rules_destroy(r);
    }
}

#[test]
fn evaluate_through_the_abi() {
    unsafe {
        let r = fc4_network_rules_create();
        load(r, 0x8, "#T#\n1,0,2,0,0\n2,3,0,0,0,2\n3,0,0x200,0,0\n3,5,0x201,0,1");
        let g = fc4_network_rules_grid_create(64, 64);
        assert!(fc4_network_rules_grid_set_cell(g, &road(10, 10, 0x0000_0200)));
        assert!(fc4_network_rules_grid_set_cell(g, &road(10, 9, 0x0200_0000)));
        let mut out = [fc4_network_rules_output::default(); 4];
        assert_eq!(fc4_network_rules_evaluate(r, 0, 0, g, 10, 10, out.as_mut_ptr(), 4), 2);
        assert_eq!(
            out[1],
            fc4_network_rules_output {
                id: 0x201,
                m: 5,
                orientation: 0x80,
                reserved: [0; 2]
            }
        );
        // A small buffer: the count is still returned.
        let mut one = [fc4_network_rules_output::default(); 1];
        assert_eq!(fc4_network_rules_evaluate(r, 0, 0, g, 10, 10, one.as_mut_ptr(), 1), 2);
        assert_eq!(one[0].id, 0x200);
        assert!(fc4_network_rules_grid_set_cell(g, &road(10, 9, 0x0200_0200)));
        assert_eq!(fc4_network_rules_evaluate(r, 0, 0, g, 10, 10, out.as_mut_ptr(), 4), -1);
        assert_eq!(fc4_network_rules_evaluate(r, 0, 1, g, 10, 10, out.as_mut_ptr(), 4), -1);
        fc4_network_rules_grid_destroy(g);
        fc4_network_rules_destroy(r);
    }
}

#[test]
fn solve_two_types_crossing() {
    unsafe {
        let r = fc4_network_rules_create();
        load(
            r,
            0x1000_0001,
            "[RoadRailIntersectionSolutions]\nk=0x02000200,0x00020002,0x1234,1,0\n",
        );
        let g = fc4_network_rules_grid_create(64, 64);
        let mut c = road(10, 10, 0x0200_0200);
        c.flags |= 2;
        c.edges[1] = 0x0002_0002;
        c.in_run = 1;
        assert!(fc4_network_rules_grid_set_cell(g, &c));
        assert!(fc4_network_rules_grid_add_to_solve(g, 10, 10));
        assert!(!fc4_network_rules_grid_add_to_solve(g, 10, 10));
        assert!(fc4_network_rules_grid_set_valid_count(g, 1));
        let mut v = -9;
        assert!(fc4_network_rules_solve(r, g, 0, false, &mut v));
        assert_eq!(v, 1);
        let mut solved = [fc4_network_rules_solved::default(); 2];
        assert_eq!(fc4_network_rules_grid_solved(g, solved.as_mut_ptr(), 2), 1);
        assert_eq!(
            solved[0],
            fc4_network_rules_solved {
                id: 0x1234,
                orientation: 1,
                x: 10,
                z: 10
            }
        );
        let mut pot = [fc4_network_rules_potential::default(); 2];
        assert_eq!(fc4_network_rules_grid_potential(g, pot.as_mut_ptr(), 2), 1);
        assert_eq!(pot[0], fc4_network_rules_potential { x: 10, z: 10, types: 3 });
        assert_eq!(fc4_network_rules_grid_unsolved(g, std::ptr::null_mut(), 0), 0);
        let mut back = cell(0, 0);
        assert!(fc4_network_rules_grid_get_cell(g, 10, 10, &mut back));
        assert_eq!((back.solved, back.union_edges, back.in_run), (0, 0x0202_0202, 1));
        assert!(!fc4_network_rules_grid_get_cell(g, 11, 10, &mut back));

        // Unsolved without the entry.
        assert!(fc4_network_rules_grid_reset_solve(g));
        assert!(fc4_network_rules_grid_set_cell(g, &c));
        let r2 = fc4_network_rules_create();
        fc4_network_rules_grid_add_to_solve(g, 10, 10);
        fc4_network_rules_grid_set_valid_count(g, 1);
        assert!(fc4_network_rules_solve(r2, g, 0, false, &mut v));
        assert_eq!(v, 0);
        let mut xz = [0i32; 4];
        assert_eq!(fc4_network_rules_grid_unsolved(g, xz.as_mut_ptr(), 2), 1);
        assert_eq!(&xz[..2], &[10, 10]);
        assert!(fc4_network_rules_grid_remove_cell(g, 10, 10));
        assert!(!fc4_network_rules_grid_remove_cell(g, 10, 10));
        fc4_network_rules_destroy(r2);
        fc4_network_rules_grid_destroy(g);
        fc4_network_rules_destroy(r);
    }
}

#[test]
fn occupant_fields_round_trip() {
    unsafe {
        let g = fc4_network_rules_grid_create(8, 8);
        let mut c = road(1, 2, 0x0200_0200);
        c.node = 1;
        c.has_occupant = 1;
        c.occ_piece = 0x55;
        c.occ_orientation = 0x81;
        c.occ_network_flags = 1;
        c.occ_has_node_edges = 1;
        c.occ_node_edges = [0x0000_0200, 0x0200_0200];
        c.occ_has_anchor = 1;
        c.occ_anchor = [3, 4];
        c.step = 7;
        c.solved = -1;
        assert!(fc4_network_rules_grid_set_cell(g, &c));
        let mut back = cell(0, 0);
        assert!(fc4_network_rules_grid_get_cell(g, 1, 2, &mut back));
        c.union_edges = 0x0200_0200;
        assert_eq!(back, c);
        fc4_network_rules_grid_destroy(g);
    }
}
