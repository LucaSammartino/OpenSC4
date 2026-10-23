// SPDX-License-Identifier: GPL-3.0-or-later
//! Regional vectors exercise the owner algorithm against service data, not an Attempt stub.
use super::*;

#[derive(Default)]
struct Host {
    rules: RegionalRules,
    cells: Option<RegionalCell>,
    border: Vec<(i32, i32)>,
    existing: Vec<(i32, i32)>,
    occupied: u32,
    border_mismatch: bool,
    writes: Vec<(bool, u32, (i32, i32), bool)>,
    adjusted: Option<[(i32, i32); 2]>,
}
impl RegionalWorld for Host {
    fn regional_rules(&self) -> RegionalRules {
        self.rules
    }
    fn draw_regional_line(
        &mut self,
        _: &mut ToolRun,
        _: &mut Vec<(usize, usize)>,
        anchors: &mut [(i32, i32); 2],
        _: u32,
        t: u32,
    ) {
        assert_eq!(t, 5);
        if let Some(adjusted) = self.adjusted {
            *anchors = adjusted;
        }
    }
    fn regional_border(&mut self, cell: (i32, i32)) -> u32 {
        self.border.push(cell);
        u32::from(self.border_mismatch && self.border.len() > 1)
    }
    fn regional_cell(&mut self, _: (i32, i32)) -> Option<RegionalCell> {
        self.cells
    }
    fn regional_existing(&mut self, kind: u32, cell: (i32, i32)) -> u32 {
        assert_eq!(kind, 4);
        self.existing.push(cell);
        self.occupied
    }
    fn regional_remove(&mut self, kind: u32, cell: (i32, i32)) {
        self.writes.push((false, kind, cell, false));
    }
    fn regional_create(&mut self, kind: u32, cell: (i32, i32), flag: bool) {
        self.writes.push((true, kind, cell, flag));
    }
}

fn fixture() -> (ToolRun, PowerLineTool, Host) {
    let mut run = ToolRun::new(5, (2, 1), (3, 1));
    run.cells = vec![(3, 0), (3, 1), (3, 2)];
    run.dragged_count = 1;
    run.straight_count = 1;
    let tool = PowerLineTool {
        anchors: vec![(2, 1), (3, 1)],
        steps: vec![(0, 2)],
        ..PowerLineTool::default()
    };
    let host = Host {
        rules: RegionalRules {
            dimensions: (4, 4),
            pen_direction: 4,
            available: true,
            multiple: 2,
            ..RegionalRules::default()
        },
        ..Host::default()
    };
    (run, tool, host)
}

#[test]
fn pl_ra2_append_both_vectors_preserve_entries_and_place_removes_before_creation() {
    let (mut run, mut tool, mut host) = fixture();
    tool.pending_neighbors.push(RegionalRecord::default());
    assert!(attempt_neighbor_connections(&mut run, &mut tool, &mut host, 2));
    assert_eq!(host.border, [(4, 0), (4, 2)]);
    assert_eq!(host.existing, [(4, 0), (4, 1), (4, 2)]);
    assert_eq!(tool.pending_neighbors[1..], tool.preview_neighbors);
    assert_eq!(
        tool.preview_neighbors.iter().map(|r| r.subsequent).collect::<Vec<_>>(),
        [false, true, true]
    );
    for (i, r) in tool.preview_neighbors.iter().enumerate() {
        assert_eq!(
            (r.key, r.edge_word, r.external, r.kind),
            (((i as u32) << 16) + 3, 0x0002_0000, (4, i as i32), 4)
        );
    }
    tool.pending_neighbors.remove(0);
    host.rules.tool_mask = (1 << 0) | (1 << 5) | (1 << 12);
    host.rules.connection_kinds[0] = 9;
    host.rules.connection_kinds[5] = 4;
    host.rules.connection_kinds[12] = 7;
    place_neighbor_connections(&tool, &mut host);
    for (i, writes) in host.writes.chunks_exact(3).enumerate() {
        assert_eq!(
            writes,
            [
                (false, 9, (4, i as i32), false),
                (false, 7, (4, i as i32), false),
                (true, 4, (4, i as i32), i != 0)
            ]
        );
    }
}

#[test]
fn pl_ra1_rejection_is_atomic_and_missing_cell_info_still_probes_region() {
    for failure in 0..6 {
        let (mut run, mut tool, mut host) = fixture();
        let mut info = RegionalCell {
            type_flags: 0x20,
            ..RegionalCell::default()
        };
        match failure {
            0 => info.type_flags = 0,
            1 => {
                info.aggregate[2] = 2;
                info.edges[0][2] = 1;
            }
            2 => {
                info.aggregate[2] = 2;
                info.edges[0][2] = 2;
                host.rules.priorities[0] = 1;
            }
            3 => host.occupied = 1,
            4 => host.rules.available = false,
            _ => host.border_mismatch = true,
        }
        host.cells = Some(info);
        let before = tool.clone();
        assert!(!attempt_neighbor_connections(&mut run, &mut tool, &mut host, 1));
        assert_eq!(tool, before);
        assert!(host.writes.is_empty());
    }
    let (mut run, mut tool, mut host) = fixture();
    assert!(attempt_neighbor_connections(&mut run, &mut tool, &mut host, 1));
    assert_eq!(host.existing.len(), 3);
}

#[test]
fn pl_ra3_diagonal_low_mode_and_clamped_key_keep_external_coordinates() {
    let (mut run, mut tool, mut host) = fixture();
    run.straight_count = 0;
    host.rules.diagonal = 1;
    host.rules.end_direction = 4;
    assert!(!attempt_neighbor_connections(&mut run, &mut tool, &mut host, 2));
    assert!(tool.pending_neighbors.is_empty());
    assert!(attempt_neighbor_connections(&mut run, &mut tool, &mut host, 1));
    assert_eq!(
        tool.pending_neighbors,
        [RegionalRecord {
            key: 0x0001_0003,
            edge_word: 0x0003_0000,
            external: (4, 1),
            kind: 4,
            subsequent: false
        }]
    );
    // Draw's copies, rather than the original anchors, control equal-endpoint rejection.
    host.adjusted = Some([(0, 0); 2]);
    assert!(!attempt_neighbor_connections(&mut run, &mut tool, &mut host, 1));
    assert_eq!(tool.anchors, [(2, 1), (3, 1)]);
}

#[test]
fn regional_directions_diagonal_single_cell_and_compatibility() {
    for (pen, end, cells, external, word) in [
        (0, (0, 1), vec![(0, 1)], (-1, 1), 2),
        (2, (1, 0), vec![(1, 0)], (1, -1), 0x200),
        (6, (1, 3), vec![(1, 3)], (1, 4), 0x0200_0000),
    ] {
        let (mut run, mut tool, mut host) = fixture();
        run.cells = cells;
        tool.steps = vec![(0, 0)];
        tool.anchors[1] = end;
        host.rules.pen_direction = pen;
        assert!(attempt_neighbor_connections(&mut run, &mut tool, &mut host, 1));
        assert_eq!(
            (tool.pending_neighbors[0].external, tool.pending_neighbors[0].edge_word),
            (external, word)
        );
    }
    let (mut run, mut tool, mut host) = fixture();
    run.straight_count = 0;
    run.cells = vec![(1, 0)];
    tool.steps = vec![(0, 0)];
    tool.anchors[1] = (1, 0);
    host.rules.diagonal = 1;
    host.rules.pen_direction = 0;
    host.rules.end_direction = 1; // (1-0+0)&3 = north
    let mut info = RegionalCell {
        type_flags: 0x20,
        ..RegionalCell::default()
    };
    info.aggregate[1] = 1;
    info.edges[0][1] = 1;
    host.rules.priorities[0] = 1;
    host.rules.compatibility = 1;
    host.cells = Some(info);
    assert!(attempt_neighbor_connections(&mut run, &mut tool, &mut host, 1));
    assert_eq!(tool.pending_neighbors[0].edge_word, 0x100);
    assert_eq!(tool.pending_neighbors[0].key, 1);
    assert_eq!(tool.pending_neighbors[0].external, (1, -1));
}
