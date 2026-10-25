// SPDX-License-Identifier: GPL-3.0-or-later
//! NT36: network-geometry §9.8.1 and network-tools §8.1.5.
use super::*;

const VERTICES: [i32; 4] = [473, 602, 603, 474];

fn fixture(heights: [f32; 4], plane: [f32; 4]) -> (NetworkGeometry, Terrain) {
    let mut terrain = vec![300.0; 129 * 129];
    for (v, h) in VERTICES.into_iter().zip(heights) {
        terrain[v as usize] = h;
    }
    let t = Terrain::from_altitudes(128, 128, &terrain, 250.0, TerrainTuning::SHIPPED).unwrap();
    let mut g = geo();
    assert!(g.draw_network_line(ROAD, (86, 3), (86, 3), false, &t, &NoHost));
    // The admitted fixture starts after slot 77 constructs these plane
    // windows/preferences; it does not infer them from a native price.
    g.insert_cell_info(CellInfo {
        x: 86,
        z: 3,
        in_run: true,
        flags: 1 << ROAD,
        vertices: VERTICES,
        corners: heights,
        allowed: plane.map(|h| (h - 35.0, h + 35.0)),
        preferred: std::array::from_fn(|i| (heights[i].min(plane[i]), heights[i].max(plane[i]))),
        ..CellInfo::default()
    });
    g.solved.push((86, 3));
    (g, t)
}

#[test]
fn tempelhof_plane_equality_retry_and_cost() {
    let heights = [291.40268, 293.30893, 290.79285, 289.53262];
    let plane = [290.4685, 292.05176, 292.05176, 290.4685];
    let (mut g, t) = fixture(heights, plane);
    assert!(!g.smoothen(0, &t, &mut NoHost));
    assert_eq!(g.smooth(&t, &mut NoHost), Ok(1));
    let h = heights[1];
    assert_eq!(g.network.solution(), &[(602, h), (603, h), (474, h), (473, h)]);
    let ci = &g.cells[&cell_key(86, 3)];
    assert_eq!(ci.corners, [h; 4]);
    assert_eq!(ci.vertices.map(|v| t.altitude_at_index(v as u32)), heights);
    assert_eq!(g.solved, [(86, 3)]);
    assert_eq!(g.failure, 0);
    assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), 11);
}

#[test]
fn tempelhof_clear_land_replay_keeps_the_high_corner() {
    let heights = [291.40268, 293.30893, 290.79285, 289.53262];
    let (mut g, t) = fixture(heights, [300.0; 4]);
    // Exercise the live replay's existing new-cell ranges, independently of
    // the admitted post-plane fixture above. Pricing still uses the solve.
    g.valid = 1;
    g.create_cell_infos(&t, &NoHost);
    assert_eq!(g.smooth(&t, &mut NoHost), Ok(1));
    assert_eq!(g.cells[&cell_key(86, 3)].corners, [heights[1]; 4]);
    assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), 11);
}

#[test]
fn flat_and_gentle_single_cell_controls_keep_ten_cost() {
    for (heights, plane, level, retained) in [
        ([300.0; 4], [300.0; 4], 0, 300.0),
        (
            [300.0, 300.5, 300.25, 299.75],
            [299.875, 300.375, 300.375, 299.875],
            1,
            300.5,
        ),
    ] {
        let (mut g, t) = fixture(heights, plane);
        assert_eq!(g.smooth(&t, &mut NoHost), Ok(level));
        assert_eq!(g.cells[&cell_key(86, 3)].corners, [retained; 4]);
        assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), 10);
    }
}

#[test]
fn gentle_straight_road_control_keeps_terrain_and_cost() {
    let heights: Vec<_> = (0..129 * 129)
        .map(|v| 300.0 + 0.25 * ((v % 129) as f32 - 4.0))
        .collect();
    let t = Terrain::from_altitudes(128, 128, &heights, 250.0, TerrainTuning::SHIPPED).unwrap();
    let mut g = geo();
    assert!(g.draw_network_line(ROAD, (4, 4), (14, 4), false, &t, &NoHost));
    g.valid = 11;
    g.create_cell_infos(&t, &NoHost);
    assert_eq!(g.smooth(&t, &mut NoHost), Ok(0));
    assert_eq!(g.solved.len(), 11);
    for ci in g.cells.values() {
        assert_eq!(ci.corners, ci.vertices.map(|v| t.altitude_at_index(v as u32)));
    }
    assert_eq!(extras::cost_of_solution(&g.cost_inputs(&t, 0)), 110);
}
