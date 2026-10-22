// SPDX-License-Identifier: GPL-3.0-or-later
//! Supported zone paths. NT22's backward wire split is explicitly withheld.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rectangle {
    pub minimum: (i32, i32),
    pub maximum: (i32, i32),
}

impl Rectangle {
    fn contains_unsigned(self, cell: (i32, i32)) -> bool {
        (self.minimum.0 as u32..=self.maximum.0 as u32).contains(&(cell.0 as u32))
            && (self.minimum.1 as u32..=self.maximum.1 as u32).contains(&(cell.1 as u32))
    }
    fn contains_signed(self, cell: (i32, i32)) -> bool {
        (self.minimum.0..=self.maximum.0).contains(&cell.0) && (self.minimum.1..=self.maximum.1).contains(&cell.1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Relocation {
    pub start: i32,
    pub stop: i32,
    pub increment: i32,
    pub unused_ninth: Option<Reference>,
    pub other: Option<Reference>,
}

/// Caller must provide reachable stop and valid indices. No invented clamp or repaired stop.
pub fn find_new_pole_position(run: &mut ToolRun, tool: &mut PowerLineTool, world: &mut impl World, rectangle: Rectangle, cells: &[(i32, i32)], scan: Relocation, failure: &mut u32) -> Result<Option<(i32, i32)>, GeometryBoundary> {
    let mut index = scan.start;
    while index != scan.stop {
        let cell = cells[index as usize];
        if !rectangle.contains_unsigned(cell) && classify(world, cell) == 0 {
            let mask = scan.other.map(|other| 1u8 << direction(cell, world.pole_data(other).cell)).unwrap_or(0);
            let found = probe(world, cell, true);
            let pole = if let Some(pole) = found {
                pole
            } else {
                let position = compute_pole_position(cell.0, cell.1, world);
                world.create_pole(cell, mask, position)
            };
            if found.is_none() { world.insert_pole(pole, 0); }
            world.release(pole);
            if let Some(other) = scan.other {
                let anchors = [cell, world.pole_data(other).cell];
                tool.anchors = anchors.to_vec();
                tool.steps = world.draw_line(run, anchors, 0);
                run.valid_count = run.dragged_count;
                determine_pole_positions(run, tool, world, failure);
                place_poles(run, tool, world, failure, true)?;
            }
            return Ok(Some(cell));
        }
        index += scan.increment;
    }
    Ok(None)
}

pub trait ZoneWorld: World {
    fn query_rectangle(&mut self, rectangle: Rectangle, flags: u32, mask: i32) -> Vec<Reference>;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ZoneResult {
    /// Full split orchestration is withheld, before detaching the endpoints, under NT22.
    pub withheld_wire_splits: usize,
}

/// Query order, signed pole bounds, endpoint-overlap wire dedup and supported pole relocation.
pub fn update_on_zone_change(run: &mut ToolRun, tool: &mut PowerLineTool, world: &mut impl ZoneWorld, enabled: bool, rectangle: Rectangle, failure: &mut u32) -> Result<ZoneResult, GeometryBoundary> {
    let mut result = ZoneResult::default();
    if !enabled { return Ok(result); }
    tool.real = true;
    let mut poles = Vec::new();
    let mut wires = Vec::new();
    for occupant in world.query_rectangle(rectangle, 0, -1) {
        if let Some(pole) = world.query_interface(occupant, POLE_IID) {
            let position = world.occupant_position(occupant);
            let cell = world.world_to_cell(position);
            if rectangle.contains_signed(cell) {
                let connections = world.pole_data(pole).connections.clone();
                for connection in connections {
                    let Some(other) = connection.other else { continue };
                    let cells = world.connection_cells(pole, other);
                    if cells.len() > 1 {
                        let (start, stop, increment) = if cells[0] == cell { (1, cells.len() as i32, 1) } else { (cells.len() as i32 - 1, 0, -1) };
                        find_new_pole_position(run, tool, world, rectangle, &cells, Relocation { start, stop, increment, unused_ninth: Some(pole), other: Some(other) }, failure)?;
                    }
                }
                poles.push(pole);
            } else {
                world.release(pole);
            }
        } else if let Some(wire) = world.query_interface(occupant, WIRE_IID) {
            let endpoints = world.wire_endpoints(wire);
            let mut duplicate = false;
            for &old in &wires {
                let previous = world.wire_endpoints(old);
                duplicate = endpoints.iter().any(|p| previous.contains(p));
                for p in previous.into_iter().flatten() { world.release(p); }
                if duplicate { break; }
            }
            if duplicate { world.release(wire); } else { wires.push(wire); }
            for p in endpoints.into_iter().flatten() { world.release(p); }
        }
        world.release(occupant);
    }
    for wire in wires {
        let endpoints = world.wire_endpoints(wire);
        if let [Some(a), Some(b)] = endpoints {
            if !poles.contains(&a) && !poles.contains(&b) {
                let cells = world.connection_cells(a, b);
                if cells.len() > 1 {
                    let cell = world.wire_cell(wire);
                    if cells.contains(&cell) {
                        // NT22: (k,count,-1) has no proved safe exit. Withhold this entire split,
                        // including detachment and the first relocation; never fabricate a bound.
                        result.withheld_wire_splits += 1;
                    }
                }
            }
        }
        for endpoint in endpoints.into_iter().flatten() { world.release(endpoint); }
        world.release(wire);
    }
    for pole in poles {
        world.clear_connections(pole, true);
        world.remove_pole(pole, true, false);
        world.release(pole);
    }
    Ok(result)
}
