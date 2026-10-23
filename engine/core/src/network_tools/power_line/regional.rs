// SPDX-License-Identifier: GPL-3.0-or-later
//! Type-5 regional Attempt (§5.4) and ordered Place (§9). Services remain foreign.

use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RegionalRecord {
    pub key: u32,
    pub edge_word: u32,
    pub external: (i32, i32),
    pub kind: u32,
    pub subsequent: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RegionalRules {
    pub dimensions: (u32, u32),
    pub multiple: u32,
    pub diagonal: u32,
    pub pen_direction: i32,
    pub end_direction: i32,
    pub compatibility: u32,
    pub priorities: [i32; 13],
    pub connection_kinds: [u32; 13],
    pub tool_mask: u32,
    pub available: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RegionalCell {
    pub type_flags: u32,
    pub aggregate: [u8; 4],
    pub edges: [[u8; 4]; 13],
}

/// Only drawing and manager/resource queries cross this seam; no owner-result callbacks.
pub trait RegionalWorld {
    fn regional_rules(&self) -> RegionalRules;
    /// Base slot 18: may update the run and the two copied final anchors.
    fn draw_regional_line(
        &mut self,
        run: &mut ToolRun,
        steps: &mut Vec<(usize, usize)>,
        anchors: &mut [(i32, i32); 2],
        mode: u32,
        type_id: u32,
    );
    fn regional_border(&mut self, cell: (i32, i32)) -> u32;
    fn regional_cell(&mut self, cell: (i32, i32)) -> Option<RegionalCell>;
    fn regional_existing(&mut self, kind: u32, cell: (i32, i32)) -> u32;
    fn regional_remove(&mut self, kind: u32, cell: (i32, i32));
    fn regional_create(&mut self, kind: u32, cell: (i32, i32), subsequent: bool);
}

pub fn attempt_neighbor_connections(
    run: &mut ToolRun,
    tool: &mut PowerLineTool,
    host: &mut impl RegionalWorld,
    mode: u32,
) -> bool {
    let n = tool.anchors.len();
    if n < 2 {
        return false;
    }
    let mut anchors = [tool.anchors[n - 2], tool.anchors[n - 1]];
    host.draw_regional_line(run, &mut tool.steps, &mut anchors, mode, 5);
    if anchors[0] == anchors[1] {
        return false;
    }
    let rules = host.regional_rules();
    let Some(&(first, last)) = tool.steps.get(run.dragged_count.wrapping_sub(1) as usize) else {
        return false;
    };
    let straight = run.straight_count >= run.dragged_count;
    let mut d = rules.pen_direction / 2;
    if !straight {
        if rules.diagonal != 1 {
            return false;
        }
        if last <= first {
            d = rules.end_direction.wrapping_sub(rules.pen_direction).wrapping_add(d) & 3;
        }
    }
    // Valid pen directions are the four entries supplied by DrawNetworkLine.
    let (dx, dz) = [(-1i32, 0i32), (0, -1), (1, 0), (0, 1)][d as usize];
    let candidate = (anchors[1].0.wrapping_add(dx), anchors[1].1.wrapping_add(dz));
    let (width, height) = rules.dimensions;
    if (candidate.0 as u32) < width && (candidate.1 as u32) < height {
        return false;
    }
    let edge = if straight {
        2
    } else if (0..=4).contains(&rules.end_direction.wrapping_sub(2 * d)) {
        3
    } else {
        1
    };
    let edge_word = edge << (8 * d);
    let project = |cell: (i32, i32)| {
        if (!straight) || d & 1 != 0 {
            (cell.0, candidate.1)
        } else {
            (candidate.0, cell.1)
        }
    };
    if rules.multiple > 1
        && host.regional_border(project(run.cells[first])) != host.regional_border(project(run.cells[last]))
    {
        return false;
    }
    let low = edge_word & 0x0101_0101;
    if (low != 0 && mode > 1) || !rules.available {
        return false;
    }
    // The public contract fixes the type-5 regional mapping at 4, with no consent branch.
    for &cell in &run.cells[first..=last] {
        if let Some(info) = host.regional_cell(cell) {
            if info.type_flags & 0x20 == 0 {
                return false;
            }
            if info.aggregate[d as usize] != 0 {
                for (t, edges) in info.edges.iter().enumerate() {
                    let value = edges[d as usize];
                    if value != 0
                        && (u32::from(value) != edge
                            || (t != 5
                                && rules.priorities[t] > rules.priorities[5]
                                && rules.compatibility & (1 << t) == 0))
                    {
                        return false;
                    }
                }
            }
        }
        let external = if d & 1 != 0 {
            (cell.0, candidate.1)
        } else {
            (candidate.0, cell.1)
        };
        if host.regional_existing(4, external) != 0 {
            return false;
        }
    }
    let mut append = |cell: (i32, i32), external, subsequent| {
        let record = RegionalRecord {
            key: (cell.1 as u32).wrapping_shl(16).wrapping_add(cell.0 as u32),
            edge_word,
            external,
            kind: 4,
            subsequent,
        };
        tool.pending_neighbors.push(record);
        tool.preview_neighbors.push(record);
    };
    if low == 0 {
        for (i, &cell) in run.cells[first..=last].iter().enumerate() {
            let external = if d & 1 != 0 {
                (cell.0, candidate.1)
            } else {
                (candidate.0, cell.1)
            };
            append(cell, external, i != 0);
        }
    } else {
        if mode != 1 {
            return false;
        }
        let clamp = |v: i32, size: u32| {
            if v < 0 {
                0
            } else if v as u32 == size {
                size.wrapping_sub(1) as i32
            } else {
                v
            }
        };
        append(
            (clamp(candidate.0, width), clamp(candidate.1, height)),
            candidate,
            false,
        );
    }
    true
}

pub fn place_neighbor_connections(tool: &PowerLineTool, host: &mut impl RegionalWorld) {
    let rules = host.regional_rules();
    for record in &tool.pending_neighbors {
        for (t, &kind) in rules.connection_kinds.iter().enumerate() {
            if t != 5 && rules.tool_mask & (1 << t) != 0 && kind != 0 {
                host.regional_remove(kind, record.external);
            }
        }
        host.regional_create(record.kind, record.external, record.subsequent);
    }
}
