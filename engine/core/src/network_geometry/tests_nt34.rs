// SPDX-License-Identifier: GPL-3.0-or-later
//! NT34 constraint-boundary vectors, not a city piece-selection fixture.
use super::*;

fn fixture(fixed: bool, placed: bool, clearance: f32, raise: f32) -> (NetworkGeometry, Terrain) {
    let mut heights = vec![100.0; 129 * 129];
    for (v, h) in [(520, 100.0), (649, 101.0), (650, 102.0), (521, 101.0)] {
        heights[v] = h;
    }
    let t = Terrain::from_altitudes(128, 128, &heights, 0.0, TerrainTuning::SHIPPED).unwrap();
    let mut g = geo();
    g.run.type_id = ROAD;
    let p = &mut g.types.infos[ROAD as usize].params;
    p.max_raise = raise;
    p.max_cut = 10.0;
    p.max_support_height = clearance;
    p.earthwork_cost = 1.0;
    let n = [102.0, 104.0, 106.0, 104.0];
    let ci = CellInfo {
        x: 4,
        z: 4,
        fixed,
        placed,
        has_supports: fixed,
        vertices: [520, 649, 650, 521],
        corners: n,
        allowed: n.map(|h| (h, h)),
        preferred: n.map(|h| (h, h)),
        ..CellInfo::default()
    };
    let key = smoothing::cell_key(4, 4);
    g.cells.insert(key, ci);
    g.solved.push((4, 4));
    g.smoothing.cells.push(key);
    (g, t)
}

fn solve_network(g: &mut NetworkGeometry, t: &Terrain) -> bool {
    // NT34 starts after plane/range construction, with no additional slope
    // or equality constraints, including C (a regular unsupported piece).
    g.network.shutdown();
    g.network.init(0.0, 1000.0);
    for ci in g.cells.values() {
        smoothing::insert_range_constraints(&mut g.network, ci, &g.smoothing.boundary, t);
    }
    g.network.solve()
}

#[test]
fn nonflat_network_support_grading_and_refresh() {
    for (fixed, clearance, terrain, earthwork) in [
        (true, 10.0, [100.0, 101.0, 102.0, 101.0], 12),
        (true, 1.0, [101.0, 103.0, 105.0, 103.0], 4),
        (false, 0.0, [102.0, 104.0, 106.0, 104.0], 0),
    ] {
        let (mut g, mut t) = fixture(fixed, false, clearance, 10.0);
        assert!(solve_network(&mut g, &t));
        assert_eq!(
            g.network.solution(),
            &[(650, 106.0), (649, 104.0), (521, 104.0), (520, 102.0)]
        );
        assert!(g.can_tiles_be_supported(&t, &mut NoHost));
        let expected = if fixed {
            [
                (649, terrain[1]),
                (650, terrain[2]),
                (520, terrain[0]),
                (521, terrain[3]),
            ]
        } else {
            [
                (650, terrain[2]),
                (649, terrain[1]),
                (521, terrain[3]),
                (520, terrain[0]),
            ]
        };
        assert_eq!(g.terrain.solution(), &expected);
        let grading = g.update_terrain_heights(&mut t);
        let expected = if clearance == 10.0 {
            vec![]
        } else if fixed {
            vec![649, 650, 520, 521]
        } else {
            vec![650, 649, 521, 520]
        };
        assert_eq!(grading.changed, expected);
        assert_eq!(g.solved, [(4, 4)]);
        let ci = &g.cells[&smoothing::cell_key(4, 4)];
        assert_eq!(ci.corners, [102.0, 104.0, 106.0, 104.0]);
        assert_eq!(ci.vertices.map(|v| t.altitude_at_index(v as u32)), terrain);
        assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), earthwork);
    }
}

#[test]
fn placed_fixed_terrain_and_infeasible_support() {
    let (mut g, t) = fixture(true, true, 1.0, 10.0);
    assert!(solve_network(&mut g, &t));
    assert!(g.can_tiles_be_supported(&t, &mut NoHost));
    assert_eq!(
        g.terrain.solution(),
        &[(650, 102.0), (649, 101.0), (521, 101.0), (520, 100.0)]
    );
    let (mut g, t) = fixture(true, false, 1.0, 0.0);
    assert!(solve_network(&mut g, &t));
    assert!(!g.can_tiles_be_supported(&t, &mut NoHost));
    assert_eq!(t.altitude_at_index(650), 102.0); // Failed support never commits.
}

#[test]
fn sgi_collision_resize_and_shutdown_retention() {
    let mut sat = VertexHtConstraintSatisfier::new();
    sat.init(0.0, 1000.0);
    for v in [0, 193, 386] {
        sat.insert_fixed(v, 10.0);
    }
    assert!(sat.solve());
    assert_eq!(sat.solution(), &[(386, 10.0), (193, 10.0), (0, 10.0)]);
    sat.shutdown();
    sat.init(0.0, 1000.0);
    for v in 0..193 {
        assert!(sat.add_vertex(v, 10.0));
    }
    // Duplicate insertion still resizes 193 -> 389 before finding the key.
    assert!(!sat.add_vertex(0, 20.0));
    sat.shutdown();
    sat.init(0.0, 1000.0);
    sat.insert_fixed(0, 1.0);
    sat.insert_fixed(389, 2.0);
    assert!(sat.solve());
    assert_eq!(sat.solution(), &[(389, 2.0), (0, 1.0)]);
}
