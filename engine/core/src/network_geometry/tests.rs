// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! network-geometry.md §18, tests 1–21, plus the details they rest on.

use std::collections::BTreeSet;

use super::extras::{self, Removal};
use super::params::{self, AVENUE, HIGHWAY, PIPE, ROAD, STREET};
use super::smoothing::{terrain_vertex_constraint, TerrainVertex};
use super::*;
use crate::terrain::{msg, Terrain, TerrainTuning};

#[path = "tests_nt34.rs"]
mod nt34;
#[path = "tests_nt36.rs"]
mod nt36;

fn close(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

/// A fresh flat, dry 64 × 64 city (altitude 300, sea 250).
fn flat() -> Terrain {
    Terrain::create(64, 64, TerrainTuning::SHIPPED).unwrap()
}

/// A 64 × 64 city whose vertex altitudes are `f(vx, vz)`, sea at 250.
fn shaped(f: impl Fn(i32, i32) -> f32) -> Terrain {
    let mut a = Vec::new();
    for z in 0..=64 {
        for x in 0..=64 {
            a.push(f(x, z));
        }
    }
    Terrain::from_altitudes(64, 64, &a, 250.0, TerrainTuning::SHIPPED).unwrap()
}

fn geo() -> NetworkGeometry {
    NetworkGeometry::new(TypeTable::shipped())
}

// ---- 1-5: the drag ------------------------------------------------------------------------------

#[test]
fn t01_straight_drag() {
    let t = flat();
    let mut g = geo();
    assert!(g.draw_network_line(ROAD, (10, 10), (20, 10), true, &t, &NoHost));
    let r = &g.run;
    assert_eq!((r.dragged_count, r.straight_count), (11, 11));
    assert_eq!((r.pen_direction, r.end_direction), (4, 3));
    assert_eq!(r.steps.len(), 11);
    assert!(r.steps.iter().all(|s| s.first == s.last));
    assert_eq!(r.cells.first(), Some(&(10, 10)));
    assert_eq!(r.cells.last(), Some(&(20, 10)));
}

#[test]
fn t02_near_straight_drag() {
    let s = break_into_segments((10, 10), (20, 11), false, false, 1, 1);
    assert_eq!(s.end, (20, 10));
    assert_eq!((s.dragged_count, s.straight_count, s.pen_direction), (11, 11, 4));
    // The end direction uses the end as passed in (§3.2 step 6): z1 < 11.
    assert_eq!(s.end_direction, 5);
}

#[test]
fn t03_diagonal_snap_width_1() {
    let s = break_into_segments((0, 0), (6, 4), true, false, 1, 1);
    assert_eq!(s.end, (5, 4));
    assert_eq!((s.dragged_count, s.straight_count), (6, 0));
    assert_eq!((s.pen_direction, s.end_direction), (4, 5));
    assert_eq!(s.axis, (0, 1));
    let (cells, steps) = compute_dragged_cells((0, 0), s.end, s.pen_direction, 1);
    assert_eq!(steps.len(), 6);
    let expected = [
        (0, 0),
        (1, 0),
        (1, 1),
        (2, 1),
        (2, 2),
        (3, 2),
        (3, 3),
        (4, 3),
        (4, 4),
        (5, 4),
    ];
    assert_eq!(cells, expected);
}

#[test]
fn t04_diagonal_snap_width_2() {
    let (w1, w2) = params::widths(AVENUE);
    let s = break_into_segments((0, 0), (5, 4), true, false, w1, w2);
    assert_eq!(s.end, (4, 4));
    assert_eq!((s.dragged_count, s.straight_count), (5, 0));
    assert_eq!(s.axis, (1, 0));
}

#[test]
fn nt28_ordered_diagonal_and_wide_footprints() {
    let t = flat();
    let mut g = geo();
    for (start, end, expected) in [((20, 4), (28, 12), 17), ((14, 4), (18, 8), 9)] {
        assert!(g.draw_network_line(ROAD, start, end, false, &t, &NoHost));
        assert_eq!(g.run.cells.len(), expected);
        let mut cells = vec![start];
        for n in 1..=(end.0 - start.0) {
            cells.extend([(start.0 + n - 1, start.1 + n), (start.0 + n, start.1 + n)]);
        }
        assert_eq!(g.run.cells, cells);
        assert_eq!(
            g.run.steps[0],
            Step {
                first: 0,
                last: 0,
                kind: StepKind::Regular
            }
        );
        for (i, step) in g.run.steps.iter().enumerate().skip(1) {
            assert_eq!((step.first, step.last), (2 * i as u32 - 1, 2 * i as u32));
        }
    }
    for (start, end, rows) in [
        ((4, 28), (16, 28), (4..=16).collect::<Vec<_>>()),
        ((16, 28), (4, 28), (4..=16).rev().collect()),
    ] {
        assert!(g.draw_network_line(AVENUE, start, end, false, &t, &NoHost));
        assert_eq!(
            g.run.cells,
            rows.into_iter().flat_map(|x| [(x, 27), (x, 28)]).collect::<Vec<_>>()
        );
    }
    assert!(g.draw_network_line(AVENUE, (4, 28), (4, 30), false, &t, &NoHost));
    assert_eq!(g.run.cells, vec![(3, 28), (4, 28), (3, 29), (4, 29), (3, 30), (4, 30)]);
}

#[test]
fn nt29_retained_corners_live_terrain_and_intersection_marker() {
    let mut g = geo();
    g.run.type_id = ROAD;
    g.types.infos[ROAD as usize].params.earthwork_cost = 1.0;
    g.insert_cell_info(CellInfo {
        x: 0,
        z: 0,
        in_run: true,
        flags: 1 << ROAD,
        corners: [4.0; 4],
        vertices: [0, 65, 66, 1],
        ..CellInfo::default()
    });
    g.solved.push((0, 0));
    // A later satisfier target cannot substitute for retained corners.
    g.network.insert_fixed(0, 99.0);
    let price = |g: &NetworkGeometry, height| extras::cost_of_solution(&g.cost_inputs(&shaped(|_, _| height), 0));
    assert_eq!(price(&g, 4.0), 10);
    assert_eq!(price(&g, 3.0), 14);
    g.cells.get_mut(&cell_key(0, 0)).unwrap().placed = true;
    assert_eq!(price(&g, 3.0), 14);
    g.cells.get_mut(&cell_key(0, 0)).unwrap().intersection = true;
    assert_eq!(price(&g, 3.0), 4);
    let ci = g.cells.get_mut(&cell_key(0, 0)).unwrap();
    ci.intersection = false;
    ci.in_run = false;
    assert_eq!(price(&g, 3.0), 4);
}

#[test]
fn nt29_solved_inventory_shared_vertices_and_repeated_charge() {
    let t = shaped(|_, _| 0.0);
    let mut g = geo();
    g.run.type_id = ROAD;
    g.types.infos[ROAD as usize].params.earthwork_cost = 1.0;
    for (x, vertices) in [(0, [0, 65, 66, 1]), (1, [1, 66, 67, 2]), (2, [2, 67, 68, 3])] {
        g.insert_cell_info(CellInfo {
            x,
            z: 0,
            corners: [1.0; 4],
            vertices,
            ..CellInfo::default()
        });
    }
    g.solved = vec![(0, 0), (1, 0)];
    let price = |g: &NetworkGeometry| extras::cost_of_solution(&g.cost_inputs(&t, 0));
    assert_eq!(price(&g), 8); // shared vertices charged twice; cache-only third cell excluded
    let ci = g.cells.get_mut(&cell_key(0, 0)).unwrap();
    ci.in_run = true;
    ci.flags = (1 << ROAD) | (1 << STREET);
    assert_eq!(price(&g), 18); // maximum type charge once
    g.cells.get_mut(&cell_key(2, 0)).unwrap().corners = [2.0; 4];
    g.solved.push((2, 0));
    assert_eq!(price(&g), 26); // appended non-run neighbour adds eight earthwork

    let t = flat();
    let mut g = geo();
    g.draw_network_line(ROAD, (4, 4), (14, 4), false, &t, &NoHost);
    assert_eq!(g.place(&t, &mut NoHost), 11);
    for ci in g.cells.values_mut() {
        ci.placed = true;
    }
    assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), 110);
}

// NT30 controlled replay inputs, not snapshots of the live city.
fn nt30_fixture(type_id: u32, entries: Vec<(i32, i32)>) -> (NetworkGeometry, Terrain) {
    let t = Terrain::from_altitudes(128, 128, &vec![100.0; 129 * 129], 0.0, TerrainTuning::SHIPPED).unwrap();
    let mut g = geo();
    g.run.type_id = type_id;
    g.types.infos[type_id as usize].params.earthwork_cost = 0.1;
    for &(x, z) in &entries {
        let vertices = smoothing::corner_vertices(x, z, 129);
        g.insert_cell_info(CellInfo {
            x,
            z,
            in_run: true,
            flags: 1 << type_id,
            vertices,
            corners: [100.0; 4],
            ..CellInfo::default()
        });
        for v in vertices {
            g.network.insert_fixed(v, 100.0);
        }
    }
    assert!(g.network.solve());
    g.solved = entries;
    (g, t)
}

#[test]
fn nt30_controlled_price_vectors() {
    let diagonal = |x, z, n| {
        std::iter::once((x, z))
            .chain((1..=n).flat_map(|j| [(x - 1 + j, z + j), (x + j, z + j)]))
            .collect()
    };
    for (kind, entries, expected) in [
        (ROAD, (4..=14).map(|x| (x, 4)).collect(), 110),
        (ROAD, diagonal(20, 4, 8), 170),
        (ROAD, diagonal(14, 4, 4), 90),
        (STREET, (64..=74).map(|x| (x, 8)).collect(), 55),
    ] {
        let (mut g, t) = nt30_fixture(kind, entries);
        assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), expected);
        for ci in g.cells.values_mut() {
            ci.placed = true;
        }
        assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), expected);
    }
}

#[test]
fn nt30_refresh_solved_and_incident_missing_network_height() {
    let (mut g, mut t) = nt30_fixture(ROAD, (4..=14).map(|x| (x, 4)).collect());
    g.cells.get_mut(&cell_key(4, 4)).unwrap().corners = [110.0; 4];
    g.smoothing.cells = vec![cell_key(4, 4)];
    assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), 114);
    // Refreshes an already solved, unplaced cell even with no grading changes.
    g.update_terrain_heights(&mut t);
    assert_eq!(g.solved.len(), 11);
    assert_eq!(g.cells[&cell_key(4, 4)].corners, [100.0; 4]);
    assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), 110);

    g.insert_cell_info(CellInfo {
        x: 3,
        z: 4,
        placed: true,
        vertices: [519, 648, 649, 520],
        corners: [102.0, 103.0, 104.0, 104.0],
        ..CellInfo::default()
    });
    // Rebuild so that 520 is absent, including from the existing run entries.
    g.network = VertexHtConstraintSatisfier::default();
    for v in [519, 648, 649] {
        g.network.insert_fixed(v, 100.0);
    }
    assert!(g.network.solve());
    t.set_altitude_at_vertex(520, 99.0);
    g.terrain.insert_fixed(520, 100.0);
    assert!(g.terrain.solve());
    assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), 110);
    let grading = g.update_terrain_heights(&mut t);
    assert_eq!(grading.changed, vec![520]);
    assert_eq!(g.solved.last(), Some(&(3, 4)));
    assert_eq!(g.cells[&cell_key(3, 4)].corners, [100.0, 100.0, 100.0, 104.0]);
    assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), 110);
    g.types.infos[ROAD as usize].params.earthwork_cost = 1.0;
    assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), 114);
    g.refresh_committed_corners(&t, &[520, 520]);
    assert_eq!(g.solved.len(), 12);
}

#[test]
fn nt30_first_refresh_fallback_and_connection_offset() {
    let (mut g, t) = nt30_fixture(ROAD, vec![(4, 4)]);
    g.solved.clear();
    let key = cell_key(4, 4);
    let ci = g.cells.get_mut(&key).unwrap();
    ci.placed = true;
    ci.connection_node = true;
    ci.corners = [110.0; 4];
    for v in ci.vertices {
        g.network.insert_fixed(v + smoothing::CONNECTION_NODE_OFFSET, 101.0);
    }
    assert!(g.network.solve());
    g.smoothing.cells = vec![key];
    g.refresh_committed_corners(&t, &[]);
    assert_eq!(g.cells[&key].corners, [101.0; 4]);
    assert_eq!(g.solved, vec![(4, 4)]);
    // Ordinary absent lookups in pass one use live terrain, not stale corners.
    g.cells.get_mut(&key).unwrap().connection_node = false;
    g.network = VertexHtConstraintSatisfier::default();
    g.refresh_committed_corners(&t, &[]);
    assert_eq!(g.cells[&key].corners, [100.0; 4]);
    assert_eq!(g.solved, vec![(4, 4)]);
}

#[test]
fn nt31_retained_powerline_price_precision() {
    assert_eq!(extras::power_line_cost(3, 10, 15, 2.0), 65);
    assert_eq!(extras::power_line_cost(2, 16_777_217, 0, 1.0), 16_777_217);
    // Unsigned input must also remain exact above the signed boundary.
    assert_eq!(extras::power_line_cost(2, 4_294_967_295, 0, 0.25), 1_073_741_823);
    // f64 rounds this product up to the next integer; x87's 64-bit
    // significand retains the fractional difference until truncation.
    let below_one = f32::from_bits(0x3f7f_ffff);
    assert_eq!(extras::power_line_cost(2, 2_147_483_649, 0, below_one), 2_147_483_520);
    assert_eq!(extras::power_line_cost(2, 2_147_483_649, 0, -below_one), -2_147_483_520);
    assert_eq!(extras::power_line_cost(2, 3, 0, f32::INFINITY), 0);
    assert_eq!(extras::power_line_cost(2, 3, 0, f32::NAN), 0);
    assert_eq!(extras::power_line_cost(2, u32::MAX, 0, f32::MAX), 0);
    assert_eq!(extras::power_line_cost(1, u32::MAX, 15, 2.0), 0);
}

#[test]
fn earthwork_product_is_not_rounded_before_the_magic_addition() {
    let price = extras::CostInputs {
        cells: vec![extras::CostCell {
            in_run: false,
            prebuilt: false,
            type_mask: 0,
            corners: [45.0, 0.0, 0.0, 0.0],
            terrain: [0.0; 4],
        }],
        earthwork_cost: 0.1,
        ..extras::CostInputs::default()
    };
    // float32(0.1) * 45 is just above 4.5. Rounding the product to
    // float32 first would create an exact tie and incorrectly charge 4.
    assert_eq!(extras::cost_of_solution(&price), 5);
}

#[test]
fn t05_street_is_straight_only() {
    let t = flat();
    let mut g = geo();
    assert!(g.draw_network_line(STREET, (0, 0), (5, 4), false, &t, &NoHost));
    assert_eq!(g.run.end, (5, 0));
    assert_eq!((g.run.dragged_count, g.run.straight_count), (6, 6));
}

#[test]
fn pre_adjust_rules() {
    // 1.1: dx = 1 and dz > 2.
    assert_eq!(break_into_segments((0, 0), (1, 5), false, false, 1, 1).end, (0, 5));
    // 1.4: dz = dx + 2 and dx ≠ 0: z2 moves one cell towards z1.
    let s = break_into_segments((0, 0), (-3, -5), false, false, 1, 1);
    assert_eq!(s.end, (-3, -4));
    assert_eq!((s.dragged_count, s.pen_direction, s.end_direction), (5, 2, 1));
    // A crooked drag with snap off keeps its end: straight count |dx − dz|.
    let s = break_into_segments((0, 10), (20, 16), false, false, 1, 1);
    assert_eq!((s.end, s.dragged_count, s.straight_count), ((20, 16), 21, 14));
}

#[test]
fn wide_types_stay_inside_the_city() {
    let t = flat();
    let mut g = geo();
    // An avenue widens toward negative minor coordinates (NT28).
    assert!(!g.draw_network_line(AVENUE, (0, 0), (5, 0), true, &t, &NoHost));
    assert!(g.run.cells.is_empty());
    assert!(g.draw_network_line(AVENUE, (0, 62), (5, 62), true, &t, &NoHost));
    assert_eq!(g.run.cells.len(), 12);
}

// ---- 6: the slope limits ------------------------------------------------------------------------

#[test]
fn t06_slope_limits() {
    let road = TypeInfo::shipped(ROAD);
    assert!(close(road.slope_rise, 11.2033, 1e-4));
    assert!(close(road.diagonal_slope_rise, 7.9219, 1e-4));
    assert!(close(TypeInfo::shipped(HIGHWAY).slope_rise, 5.8235, 1e-4));
    let rail = TypeInfo::shipped(params::RAIL);
    assert!(close(rail.slope_rise, 7.4609, 1e-4) && close(rail.diagonal_slope_rise, 5.2757, 1e-4));
    let street = TypeInfo::shipped(STREET);
    assert!(close(street.slope_rise, 13.4256, 1e-4) && close(street.diagonal_slope_rise, 9.4933, 1e-4));
    let pipe = TypeInfo::shipped(PIPE);
    assert!(close(pipe.slope_rise, 16.0, 1e-4) && close(pipe.diagonal_slope_rise, 11.3137, 1e-4));
    // The smoothness limits use p+0x08 (equal to p+0x00 in the shipped data).
    assert_eq!(road.smoothness, road.slope_rise);
}

// ---- 7, 8: the satisfier ------------------------------------------------------------------------

#[test]
fn t07_satisfier_slope() {
    let s = TypeInfo::shipped(ROAD).slope_rise;
    let run = |c: f32| {
        let mut sat = VertexHtConstraintSatisfier::new();
        sat.init(0.0, 1000.0);
        sat.insert_fixed(1, 0.0);
        sat.insert_fixed(3, c);
        sat.add_vertex(2, 0.0);
        sat.insert_slope(1, 2, s);
        sat.insert_slope(2, 3, s);
        let ok = sat.solve();
        (ok, sat)
    };
    assert!(!run(30.0).0);
    let (ok, sat) = run(20.0);
    assert!(ok);
    let lo = (20.0f64 - f64::from(s)) as f32;
    assert_eq!(sat.height(2), Some(lo));
    assert!(close(lo, 8.7967, 1e-4));
    assert_eq!(sat.height(1), Some(0.0));
    assert_eq!(sat.height(3), Some(20.0));
}

#[test]
fn t08_satisfier_smoothness_and_preference() {
    let run = |pref: Option<f32>| {
        let mut sat = VertexHtConstraintSatisfier::new();
        sat.init(-f32::MAX, f32::MAX);
        sat.insert_fixed(1, 0.0);
        sat.insert_fixed(3, 0.0);
        sat.add_vertex(2, 0.0);
        sat.insert_smoothness(1, 2, 3, 11.2);
        if let Some(p) = pref {
            sat.set_preferred_value(2, p);
        }
        assert!(sat.solve());
        sat.height(2).unwrap()
    };
    assert_eq!(run(Some(3.0)), 3.0);
    assert_eq!(run(Some(9.0)), 0.0);
    // The range after the first propagation is [−5.6, 5.6].
    let mut sat = VertexHtConstraintSatisfier::new();
    sat.init(-f32::MAX, f32::MAX);
    sat.insert_fixed(1, 0.0);
    sat.insert_fixed(3, 0.0);
    sat.add_vertex(2, 7.0);
    sat.insert_smoothness(1, 2, 3, 11.2);
    assert!(sat.solve());
    assert_eq!(sat.height(2), Some(5.6));
}

#[test]
fn satisfier_details() {
    let mut sat = VertexHtConstraintSatisfier::new();
    sat.init(0.0, 100.0);
    sat.init(-5.0, 5.0); // ignored before Shutdown
    assert_eq!(sat.default_range(), (0.0, 100.0));
    assert!(sat.add_vertex(7, 50.0));
    assert!(!sat.add_vertex(7, 10.0)); // the first record stays
    assert_eq!(sat.record(7).unwrap().initial, 50.0);
    // An inversion by at most 0.01 collapses to the lower bound.
    sat.insert_range(7, 20.0, 30.0);
    sat.insert_range(7, 30.005, 40.0);
    assert_eq!(sat.range(7), Some((30.005, 30.005)));
    // A larger inversion stays and fails the solve.
    sat.add_vertex(8, 0.0);
    sat.insert_range(8, 10.0, 20.0);
    sat.insert_range(8, 21.0, 30.0);
    assert!(!sat.solve());
    // A second slope on the same pair keeps the smaller s.
    let mut sat = VertexHtConstraintSatisfier::new();
    sat.init(0.0, 100.0);
    sat.insert_slope(1, 2, 5.0);
    sat.insert_slope(2, 1, 3.0);
    sat.insert_slope(1, 2, 4.0);
    assert_eq!(sat.constraints(), &[Constraint::Slope(1, 2, 3.0)]);
    // A preferred range narrows the range before the clamp.
    sat.add_vertex(1, 0.0);
    sat.insert_fixed(2, 50.0);
    sat.set_preferred_range(1, 51.0, 60.0);
    assert!(sat.solve());
    assert_eq!(sat.height(1), Some(51.0));
    // Equalities pin through the propagation.
    let mut sat = VertexHtConstraintSatisfier::new();
    sat.init(0.0, 100.0);
    for v in 1..=4 {
        sat.add_vertex(v, 10.0 * v as f32);
    }
    sat.insert_equality(1, 2);
    sat.insert_equality(2, 3);
    sat.insert_equality(3, 4);
    sat.insert_fixed(4, 33.0);
    assert!(sat.solve());
    assert!((1..=4).all(|v| sat.height(v) == Some(33.0)));
    // Singleton visits append 4, then 1, 2, 3 on the next ordered wave;
    // the pending stack therefore pops 3 first (NT36).
    assert_eq!(sat.solution()[0], (3, 33.0));
}

// ---- 9, 10: tunnels -----------------------------------------------------------------------------

#[test]
fn t09_tunnel_trigger() {
    for (rise, searched) in [(11.0f32, false), (12.0, true)] {
        let t = shaped(|x, _| if x <= 12 { 300.0 } else { 300.0 + rise });
        let mut g = geo();
        assert!(g.draw_network_line(ROAD, (10, 10), (21, 10), true, &t, &NoHost));
        assert_eq!(g.run.dragged_count, 12);
        g.validate(&t, &mut NoHost);
        let want: Vec<u32> = if searched { vec![2] } else { vec![] };
        assert_eq!(g.run.tunnel_searches, want, "rise {rise}");
    }
}

#[test]
fn t10_no_tunnel_code() {
    // A hill from vertex 6 to 20: a tunnel from step 5 to step 20.
    let hill = shaped(|x, _| if (6..=20).contains(&x) { 340.0 } else { 300.0 });
    let mut g = geo();
    assert!(g.draw_network_line(ROAD, (0, 10), (30, 10), true, &hill, &NoHost));
    g.place(&hill, &mut NoHost);
    assert_eq!(g.run.tunnels, vec![Tunnel { start: 5, end: 20 }]);
    assert_eq!(g.run.steps[5].kind, StepKind::Portal);
    assert_eq!(g.run.steps[12].kind, StepKind::TunnelInterior);
    assert_eq!(g.run.steps[20].kind, StepKind::Portal);
    assert_ne!(g.failure, code::TUNNEL);
    // A crooked drag over the same hill gets no tunnel and no tunnel code.
    let mut g = geo();
    g.draw_network_line(ROAD, (0, 10), (30, 16), false, &hill, &NoHost);
    g.place(&hill, &mut NoHost);
    assert!(g.run.tunnels.iter().all(|t| t.end < 25));
    assert_ne!(g.failure, code::TUNNEL);
    for (a, b) in [((0, 0), (10, 0)), ((5, 5), (5, 30)), ((3, 3), (9, 8))] {
        let t = flat();
        let mut g = geo();
        g.draw_network_line(ROAD, a, b, true, &t, &NoHost);
        g.place(&t, &mut NoHost);
        assert_ne!(g.failure, code::TUNNEL);
    }
}

// ---- 11-13: bridges, water, reserved tiles ------------------------------------------------------

/// A river: cells x = 8..12 have water (vertices 9..12 below the sea).
fn river() -> Terrain {
    shaped(|x, _| if (9..=12).contains(&x) { 200.0 } else { 300.0 })
}

struct BridgeHost;
impl GeometryHost for BridgeHost {
    fn bridge_type_count(&self, _t: u32) -> u32 {
        1
    }
    fn find_best_bridge_type(&mut self, _r: &Run, _s: u32, _e: u32) -> Option<BridgeChoice> {
        Some(BridgeChoice {
            bridge_type: 0,
            min_span: 2,
        })
    }
}

#[test]
fn t11_bridge_needs_a_straight_drag() {
    let t = river();
    assert!(t.cell_has_water(8, 10) && t.cell_has_water(12, 10));
    assert!(!t.cell_has_water(7, 10) && !t.cell_has_water(13, 10));
    let mut g = geo();
    g.draw_network_line(ROAD, (0, 10), (20, 16), false, &t, &BridgeHost);
    let r = g.validate(&t, &mut BridgeHost);
    assert!(g.run.bridges.is_empty());
    assert_eq!(r.failure, Some(code::BAD_TERRAIN));
    assert_eq!((g.valid, g.failure), (0, code::BAD_TERRAIN));

    let mut g = geo();
    g.draw_network_line(ROAD, (0, 10), (20, 10), true, &t, &BridgeHost);
    let r = g.validate(&t, &mut BridgeHost);
    assert_eq!(r.failure, None);
    assert_eq!(
        g.run.bridges,
        vec![Bridge {
            start: 7,
            end: 13,
            bridge_type: 0
        }]
    );
    assert!((7..=13).all(|i| g.run.steps[i].kind == StepKind::Bridge));
    assert_eq!(g.run.steps[6].kind, StepKind::Regular);
    // No bridge type fits: the run is cut at the first wet step.
    let mut g = geo();
    g.draw_network_line(ROAD, (0, 10), (20, 10), true, &t, &NoHost);
    struct NoFit;
    impl GeometryHost for NoFit {
        fn bridge_type_count(&self, _t: u32) -> u32 {
            1
        }
    }
    let r = g.validate(&t, &mut NoFit);
    assert_eq!(r.failure, Some(code::BRIDGE));
    let mut g = geo();
    g.draw_network_line(ROAD, (0, 10), (20, 10), true, &t, &NoHost);
    let mut nofit = NoFit;
    let mut v = g.run.dragged_count;
    passes::identify_bridge_cells(&mut g.run, &ToolFlags::for_type(ROAD), &t, &mut nofit, &mut v);
    assert_eq!(v, 8);
}

#[test]
fn t12_starting_in_water() {
    let t = river();
    let mut g = geo();
    g.draw_network_line(ROAD, (10, 10), (20, 10), true, &t, &NoHost);
    assert_eq!(g.place(&t, &mut NoHost), 0);
    assert_eq!(g.failure, code::BAD_TERRAIN);
    let mut g = geo();
    g.draw_network_line(PIPE, (10, 10), (20, 10), true, &t, &NoHost);
    let r = g.validate(&t, &mut NoHost);
    assert_eq!(r.failure, None);
    assert_eq!(g.valid, 11);
}

struct Landfill((i32, i32));
impl GeometryHost for Landfill {
    fn cell_occupants(&self, x: i32, z: i32) -> Vec<CellOccupant> {
        if (x, z) == self.0 {
            vec![CellOccupant::Landfill]
        } else {
            Vec::new()
        }
    }
}

#[test]
fn t13_reserved_landfill() {
    let t = flat();
    let mut g = geo();
    let mut h = Landfill((12, 10));
    g.draw_network_line(ROAD, (10, 10), (15, 10), true, &t, &h);
    assert_eq!(g.place(&t, &mut h), 0);
    assert_eq!(g.failure, code::RESERVED);
    // The pass alone cuts at step 2.
    let mut v = g.run.dragged_count;
    passes::check_for_reserved_tiles(&g.run, &h, ROAD, false, &mut v);
    assert_eq!(v, 2);
    // Network occupants: immovable and (not a node or another type).
    let net = |immovable, connection_node, type_id| CellOccupant::Network {
        immovable,
        connection_node,
        type_id,
    };
    struct One(CellOccupant);
    impl GeometryHost for One {
        fn cell_occupants(&self, _x: i32, _z: i32) -> Vec<CellOccupant> {
            vec![self.0]
        }
    }
    assert!(passes::is_cell_reserved(&One(net(true, false, ROAD)), 0, 0, ROAD));
    assert!(passes::is_cell_reserved(&One(net(true, true, 1)), 0, 0, ROAD));
    assert!(!passes::is_cell_reserved(&One(net(true, true, ROAD)), 0, 0, ROAD));
    assert!(!passes::is_cell_reserved(&One(net(false, false, ROAD)), 0, 0, ROAD));
    assert!(!passes::is_cell_reserved(&One(CellOccupant::Building), 0, 0, ROAD));
}

// ---- 14-19: flags, supports, costs, destroy -----------------------------------------------------

#[test]
fn t14_support_level() {
    let th = TypeInfo::shipped(ROAD).params.support_threshold;
    assert_eq!(th, 45.0);
    let lvl = |d: f32| extras::support_level([100.0 + d; 4], [100.0; 4], th);
    assert_eq!(lvl(30.0), 1);
    assert_eq!(extras::support_flags(1), 0x4000_0000 | 0x0800_0000);
    assert_eq!(lvl(50.0), 2);
    assert_eq!(lvl(0.005), 0);
    assert_eq!(extras::support_flags(0), 0);
    // The maximum over the corners.
    assert_eq!(extras::support_level([100.0, 150.0, 100.0, 120.0], [100.0; 4], th), 2);
}

#[test]
fn t15_terrain_support() {
    let road = TypeInfo::shipped(ROAD);
    assert_eq!(
        terrain_vertex_constraint(100.0, 0.0, true, false, &road),
        TerrainVertex::Unsupported
    );
    assert_eq!(
        terrain_vertex_constraint(70.0, 0.0, true, false, &road),
        TerrainVertex::Range {
            lo: 15.0,
            hi: 25.0,
            preferred: 70.0
        }
    );
    assert_eq!(
        terrain_vertex_constraint(20.0, 0.0, false, false, &road),
        TerrainVertex::Fix(20.0)
    );
    // Without supports, more than p0C above the terrain fails.
    assert_eq!(
        terrain_vertex_constraint(26.0, 0.0, false, false, &road),
        TerrainVertex::Unsupported
    );
    // The solve of the n = 70 vertex: the preferred 70 is out of range, the
    // terrain 0 is clamped to 15.
    let mut sat = VertexHtConstraintSatisfier::new();
    sat.init(-f32::MAX, f32::MAX);
    sat.add_vertex(5, 0.0);
    sat.insert_range(5, 15.0, 25.0);
    sat.set_preferred_value(5, 70.0);
    assert!(sat.solve());
    assert_eq!(sat.height(5), Some(15.0));
}

#[test]
fn t16_draft_flag() {
    let cell = 1 | smoothing::FLAG_BRIDGE | 0x0010_0000;
    let draft = extras::occupant_flags(cell, false, false);
    assert_ne!(draft & extras::FLAG_DRAFT, 0);
    assert_eq!(
        draft,
        1 | smoothing::FLAG_BRIDGE | extras::FLAG_DRAFT | extras::FLAG_4000
    );
    let real = extras::occupant_flags(cell | extras::FLAG_DRAFT, true, false);
    assert_eq!(real & extras::FLAG_DRAFT, 0);
    assert_eq!(real, cell | extras::FLAG_4000);
    let tunnel = extras::occupant_flags(1 | extras::FLAG_4000, true, true);
    assert_eq!(tunnel, 1 | extras::FLAG_USABLE);
}

#[test]
fn t17_cell_cost() {
    let t = flat();
    let mut g = geo();
    g.draw_network_line(ROAD, (10, 10), (20, 10), true, &t, &NoHost);
    assert_eq!(g.place(&t, &mut NoHost), 11);
    assert_eq!(g.failure, 0);
    assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), 110);
    // Earthwork: Σ|corner − altitude| × 0.1, rounded half to even.
    let mut c = extras::CostInputs {
        earthwork_cost: 0.1,
        ..extras::CostInputs::default()
    };
    c.cells.push(extras::CostCell {
        corners: [305.0, 305.0, 300.0, 300.0],
        terrain: [300.0; 4],
        ..extras::CostCell::default()
    });
    assert_eq!(extras::cost_of_solution(&c), 1);
    c.cells[0].corners = [325.0, 300.0, 300.0, 300.0];
    assert_eq!(extras::cost_of_solution(&c), 2); // 2.5 → 2
}

#[test]
fn t18_power_line_cost() {
    assert_eq!(extras::power_line_cost(3, 20, 15, 2.0), 85);
    assert_eq!(extras::power_line_cost(1, 20, 15, 2.0), 0);
    assert_eq!(extras::power_line_cost(0, 0, 15, 2.0), 0);
}

#[test]
fn t19_destroy_test_set() {
    let destroyed: BTreeSet<i32> = [extras::destroy_key(20, 20)].into();
    let mut resolve = BTreeSet::new();
    let mut removed = Vec::new();
    for (x, z) in [(20, 20), (21, 20), (20, 21), (21, 21)] {
        let k = extras::destroy_key(x, z);
        let how = extras::removal_for(k, &destroyed);
        if how == Removal::RemoveOrReplace {
            // Here RemoveOrReplaceOccupant puts a plain piece back at (21, 20)
            // and (20, 21) only.
            extras::update_resolve_set(&mut resolve, k, (x, z) != (21, 21));
        }
        removed.push(((x, z), how));
    }
    assert_eq!(removed[0].1, Removal::Remove);
    assert!(removed[1..].iter().all(|r| r.1 == Removal::RemoveOrReplace));
    let want: BTreeSet<i32> = [extras::destroy_key(21, 20), extras::destroy_key(20, 21)].into();
    assert_eq!(resolve, want);
}

// ---- 20, 21: grading and the smoothing codes ----------------------------------------------------

#[test]
fn t20_grading_threshold() {
    let mut t = flat();
    t.set_view_ready(true);
    let s = 65;
    let (a, b) = (10 * s + 10, 20 * s + 30);
    let mut ter = VertexHtConstraintSatisfier::new();
    ter.init(-f32::MAX, f32::MAX);
    ter.insert_fixed(a, 300.0005);
    ter.insert_fixed(b, 300.002);
    assert!(ter.solve());
    let g = update_terrain_heights(&ter, &mut t);
    assert_eq!(g.changed, vec![b]);
    assert_eq!(g.rect, Some(VertRect::new(29, 19, 31, 21)));
    assert_eq!(t.altitude_at_index(a as u32), 300.0);
    assert_eq!(t.altitude_at_index(b as u32), 300.002);
    let posts = t.take_posts();
    let kinds: Vec<u32> = posts.iter().map(|p| p.msg).collect();
    assert_eq!(kinds.first(), Some(&msg::BEGIN_LONG_OPERATION));
    assert_eq!(kinds.last(), Some(&msg::END_LONG_OPERATION));
    let changed = posts.iter().find(|p| p.msg == msg::TERRAIN_CHANGED).unwrap();
    assert_eq!(changed.data4, 4);
    assert_eq!((changed.data1, changed.data2), ((29 << 16) + 31, (19 << 16) + 21));
    // Nothing changed: no messages.
    let mut ter = VertexHtConstraintSatisfier::new();
    ter.init(-f32::MAX, f32::MAX);
    ter.insert_fixed(a, 300.0005);
    assert!(ter.solve());
    let g = update_terrain_heights(&ter, &mut t);
    assert!(g.changed.is_empty() && g.rect.is_none());
    assert!(t.take_posts().is_empty());
    // The rectangle stays inside the city.
    let mut ter = VertexHtConstraintSatisfier::new();
    ter.init(-f32::MAX, f32::MAX);
    ter.insert_fixed(0, 310.0);
    ter.insert_fixed(64 * s + 64, 310.0);
    assert!(ter.solve());
    let g = update_terrain_heights(&ter, &mut t);
    assert_eq!(g.rect, Some(VertRect::new(0, 0, 64, 64)));
}

#[test]
fn t21_smoothing_codes() {
    // A road up a 30 m-per-cell ramp between two immovable pieces held at
    // the terrain: the height solve fails at every level.
    let ramp = shaped(|x, _| 300.0 + 30.0 * x as f32);
    struct Ends;
    impl GeometryHost for Ends {
        fn existing_piece(&self, x: i32, _z: i32) -> Option<ExistingPiece> {
            (x == 10 || x == 20).then(|| {
                let a = |vx: i32| 300.0 + 30.0 * vx as f32;
                ExistingPiece {
                    flags: 1,
                    immovable: true,
                    corners: [a(x), a(x), a(x + 1), a(x + 1)],
                    ..ExistingPiece::default()
                }
            })
        }
    }
    let mut g = geo();
    g.draw_network_line(ROAD, (10, 10), (20, 10), true, &ramp, &Ends);
    assert_eq!(g.place(&ramp, &mut Ends), 0);
    assert_eq!(g.failure, code::SMOOTHING);
    // Without the ends the ordered propagation reaches closure before the
    // first pick (NT36); the free run finds a supportable profile.
    let mut g = geo();
    g.draw_network_line(ROAD, (10, 10), (20, 10), true, &ramp, &NoHost);
    assert_eq!(g.place(&ramp, &mut NoHost), 11);
    assert_eq!(g.failure, 0);
    // A 40 m dip: level 0 cannot solve (the run is held at the terrain).
    // At level 1 the dip's vertices have the narrowest ranges (their minimum
    // is the sea level 250), so they are picked first at their own height
    // and the rest of the run follows them down: the terrain is cut, which
    // p10 = 10000 allows.
    let dip = shaped(|x, _| if x == 15 { 260.0 } else { 300.0 });
    let mut g = geo();
    g.draw_network_line(ROAD, (10, 10), (20, 10), true, &dip, &NoHost);
    g.validate(&dip, &mut NoHost);
    g.create_cell_infos(&dip, &NoHost);
    assert!(!g.smoothen(0, &dip, &mut NoHost));
    assert_eq!(g.place(&dip, &mut NoHost), 11);
    assert_eq!(g.level, Some(1));
    assert_eq!(g.network.height(10 * 65 + 15), Some(260.0));
    assert_eq!(g.network.solution()[0].1, 260.0);
    // The loop helper: the last level's code wins.
    let mut n = 0;
    let r = smoothing::smoothing_loop(false, |_| {
        n += 1;
        if n < 4 {
            (true, false)
        } else {
            (false, false)
        }
    });
    assert_eq!(r, Err(code::SMOOTHING));
    let mut levels = Vec::new();
    let r = smoothing::smoothing_loop(true, |l| {
        levels.push(l);
        (true, l == 2)
    });
    assert_eq!((r, levels), (Ok(2), vec![1, 2]));
}

#[test]
fn flat_road_solves_at_level_0_and_grades_nothing() {
    let mut t = flat();
    t.set_view_ready(true);
    let mut g = geo();
    g.draw_network_line(ROAD, (10, 10), (20, 10), true, &t, &NoHost);
    assert_eq!(g.place(&t, &mut NoHost), 11);
    assert_eq!(g.level, Some(0));
    let gr = g.update_terrain_heights(&mut t);
    assert!(gr.changed.is_empty());
}

#[test]
fn a_gentle_valley_is_graded() {
    // A 6 m dip at vertex column 15: the network bridges it within the
    // slope limits and the terrain is raised to meet it (no supports).
    let mut t = shaped(|x, _| if x == 15 { 294.0 } else { 300.0 });
    t.set_view_ready(true);
    let mut g = geo();
    g.draw_network_line(ROAD, (10, 10), (20, 10), true, &t, &NoHost);
    assert_eq!(g.place(&t, &mut NoHost), 11);
    let lvl = g.level.unwrap();
    assert!(lvl <= 3);
    let gr = g.update_terrain_heights(&mut t);
    // Every written vertex now holds the terrain satisfier's height.
    for v in &gr.changed {
        assert_eq!(Some(t.altitude_at_index(*v as u32)), g.terrain.height(*v));
    }
    // Without supports the graded terrain meets the network.
    for &(v, h) in g.terrain.solution() {
        if let Some(n) = g.network.height(v) {
            if !g.smoothing.boundary.contains(&v) {
                assert_eq!(h, n);
            }
        }
    }
}

#[test]
fn smoothing_set_and_boundary() {
    let t = flat();
    let mut g = geo();
    g.draw_network_line(ROAD, (10, 10), (12, 10), true, &t, &NoHost);
    g.validate(&t, &mut NoHost);
    g.create_cell_infos(&t, &NoHost);
    let s0 = smoothing::compute_initial_smoothing_set(&g.run, &g.solved, &g.cells, 0, (64, 64));
    assert_eq!(s0.cells, vec![cell_key(10, 10), cell_key(11, 10), cell_key(12, 10)]);
    // Level 0: every corner of a one-row box is on its border.
    assert_eq!(s0.boundary.len(), 8);
    let s1 = smoothing::compute_initial_smoothing_set(&g.run, &g.solved, &g.cells, 1, (64, 64));
    // Only the run's cells have infos; the border is 5 × 3 cells' outline.
    assert_eq!(s1.cells.len(), 3);
    assert_eq!(s1.boundary.len(), 2 * 6 + 2 * 2);
    assert!(!s1.boundary.contains(&(10 * 65 + 11)));
    // At the city's edge the border vertices are not held.
    let mut g = geo();
    g.draw_network_line(ROAD, (0, 0), (2, 0), true, &t, &NoHost);
    g.validate(&t, &mut NoHost);
    g.create_cell_infos(&t, &NoHost);
    let s = smoothing::compute_initial_smoothing_set(&g.run, &g.solved, &g.cells, 0, (64, 64));
    assert_eq!(
        s.boundary.iter().copied().collect::<Vec<_>>(),
        vec![65 + 1, 65 + 2, 65 + 3]
    );
}

#[test]
fn cell_infos_respect_the_water_surface() {
    // A cell beside a lake: its wet corners' minimum is the water level.
    let t = shaped(|x, _| if x >= 13 { 240.0 } else { 300.0 });
    let mut g = geo();
    g.draw_network_line(ROAD, (10, 10), (12, 10), true, &t, &NoHost);
    g.run.dragged_count = 3;
    g.valid = 3;
    g.create_cell_infos(&t, &NoHost);
    let ci = &g.cells[&cell_key(12, 10)];
    // Corners 2 and 3 are at x = 13 (altitude 240, water level 250).
    assert_eq!(ci.allowed[2].0, 250.0);
    assert_eq!(ci.preferred[2].0, 250.0);
    assert_eq!(ci.allowed[2].1, 240.0 + 500.0);
    // A dry vertex's level is the sea level too (its depth is sea − altitude
    // in the terrain core), so no corner goes below the sea.
    assert_eq!(ci.allowed[0], (250.0, 800.0));
    assert_eq!(ci.edges[0], 1); // the −x side joins (11, 10)
}

#[test]
fn flat_cells_and_cross_sections() {
    let road = TypeInfo::shipped(ROAD);
    // A cell with no edges is levelled.
    let mut sat = VertexHtConstraintSatisfier::new();
    let ci = CellInfo {
        x: 5,
        z: 5,
        vertices: smoothing::corner_vertices(5, 5, 65),
        ..CellInfo::default()
    };
    smoothing::insert_slope_and_smoothness_constraints(&mut sat, &ci, &road, false, (64, 64));
    assert_eq!(sat.constraint_count(), 3);
    // An E-W cell: 2 sides × (3 equalities + 2 slopes + 1 smoothness), the
    // shared slopes merged.
    let mut sat = VertexHtConstraintSatisfier::new();
    let mut ci = ci.clone();
    ci.edges[0] = 0x0001_0001;
    ci.edge_union = 0x0001_0001;
    smoothing::insert_slope_and_smoothness_constraints(&mut sat, &ci, &road, false, (64, 64));
    let slopes = sat
        .constraints()
        .iter()
        .filter(|c| matches!(c, Constraint::Slope(..)))
        .count();
    let smooth = sat
        .constraints()
        .iter()
        .filter(|c| matches!(c, Constraint::Smoothness(..)))
        .count();
    assert_eq!((slopes, smooth), (3, 2));
    // A four-sided single-type cell with a shipped pattern is not levelled.
    let mut sat = VertexHtConstraintSatisfier::new();
    ci.edges[0] = 0x0301_0301;
    ci.edge_union = 0x0301_0301;
    smoothing::insert_slope_and_smoothness_constraints(&mut sat, &ci, &road, false, (64, 64));
    assert!(sat.constraint_count() > 3);
    // An elevated type's diagonal two-sided cell is levelled.
    let mut sat = VertexHtConstraintSatisfier::new();
    ci.edges[0] = 0x0000_0108;
    ci.edge_union = 0x0000_0108;
    smoothing::insert_slope_and_smoothness_constraints(&mut sat, &ci, &road, true, (64, 64));
    assert_eq!(sat.constraint_count(), 3);
    // A cross-section outside the city is skipped.
    let edge = smoothing::cross_sections(0, 0, 0, 0, (64, 64));
    assert_eq!(edge[2], [-1, -1]);
}

#[test]
fn tunnel_offsets_and_means() {
    use super::passes::edge_offsets;
    let s = 65;
    let want_a = [s + 1, s, s, 0, 0, 1, 1, s + 1];
    let want_b = [1, 1, s + 1, s + 1, s, s, 0, 0];
    for d in 0..8u8 {
        assert_eq!(edge_offsets(s, d), (want_a[d as usize], want_b[d as usize]));
    }
}

#[test]
fn diagonal_end_extension() {
    struct Ext;
    impl GeometryHost for Ext {
        fn extends_diagonal_end(&self, x: i32, z: i32, _t: u32) -> bool {
            (x, z) == (6, 5)
        }
    }
    let t = flat();
    let mut g = geo();
    // (0, 0) → (5, 4): end direction 5 (+x +z), the next cell (6, 5).
    g.draw_network_line(ROAD, (0, 0), (6, 4), true, &t, &Ext);
    assert_eq!(g.run.dragged_count, 7);
    assert_eq!(g.run.cells.last(), Some(&(6, 5)));
}
