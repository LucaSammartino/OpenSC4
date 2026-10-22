// SPDX-License-Identifier: GPL-3.0-or-later
//! Nonregional drag selection, placement, preview and repair orchestration.

use super::*;
use crate::x87::r24;

pub fn pivot(cells: &[(i32, i32)], range: (usize, usize), step: usize, previous: u32, anchor: Option<(i32, i32)>) -> Option<usize> {
    if previous != 0 && step < previous as usize {
        return Some(range.0);
    }
    anchor.and_then(|anchor| (range.0..=range.1).find(|&i| {
        let cell = cells[i];
        (i64::from(anchor.0) - i64::from(cell.0)).abs() == (i64::from(anchor.1) - i64::from(cell.1)).abs()
    }))
}

/// Interception transfers two endpoint references only on acceptance (§4.1).
pub fn intercept(tool: &PowerLineTool, world: &mut impl World, wire: Reference, step: u32, previous: u32, forced: bool, pending: &mut bool) -> SavedConnection {
    let mut descriptor = SavedConnection { endpoints: world.wire_endpoints(wire), enabled: [1; 2] };
    let cell = descriptor.endpoints.map(|p| p.map(|p| world.pole_data(p).cell).unwrap_or((-1, -1)));
    let diagonal = cell[0].0 != cell[1].0 && cell[0].1 != cell[1].1;
    let reject = if step >= previous || diagonal {
        true
    } else {
        *pending = true;
        let duplicate = tool.descriptor_present(descriptor)
            || tool.endpoint_aligned(world, descriptor.endpoints[0])
            || tool.endpoint_aligned(world, descriptor.endpoints[1]);
        if duplicate && !forced {
            *pending = false;
            true
        } else {
            false
        }
    };
    if reject {
        for endpoint in &mut descriptor.endpoints {
            if let Some(p) = endpoint.take() { world.release(p); }
        }
        descriptor.enabled = [1; 2];
    }
    world.release(wire);
    descriptor
}

fn ray_blocks(world: &mut impl World, a: Point, b: Point) -> bool {
    let delta = std::array::from_fn::<_, 3, _>(|i| r24(f64::from(b[i]) - f64::from(a[i])));
    let squared = r24(r24(r24(delta[2] * delta[2]) + r24(delta[1] * delta[1])) + r24(delta[0] * delta[0]));
    let stored_squared = squared as f32;
    let inverse = r24(1.0 / r24(squared.sqrt())) as f32;
    let direction = delta.map(|v| r24(v * f64::from(inverse)) as f32);
    if let Some(hit) = world.intersect(a, direction, 0, 0) {
        let d = std::array::from_fn::<_, 3, _>(|i| r24(f64::from(hit[i]) - f64::from(a[i])));
        let hit2 = r24(r24(r24(d[2] * d[2]) + r24(d[1] * d[1])) + r24(d[0] * d[0]));
        r24(hit2.sqrt()) <= r24(f64::from(stored_squared).sqrt())
    } else {
        false
    }
}

fn discard_solitary(tool: &mut PowerLineTool, world: &mut impl World, release: bool) {
    if tool.poles.last().is_some_and(|p| p.links.is_empty()) {
        let pole = tool.poles.pop().unwrap();
        if release {
            for endpoint in pole.saved.endpoints.into_iter().flatten() { world.release(endpoint); }
        }
    }
}

fn emit(tool: &mut PowerLineTool, cell: (i32, i32), index: usize, active: bool, accumulator: &mut Vec<WireCell>, descriptor: &mut SavedConnection) {
    let wire_index = tool.wire_vectors.len() as i32;
    tool.append_pole(cell, index, active, wire_index, *descriptor);
    tool.wire_vectors.push(accumulator.clone());
    accumulator.clear();
    *descriptor = SavedConnection::default();
}

pub fn determine_pole_positions(run: &mut ToolRun, tool: &mut PowerLineTool, world: &mut impl World, failure: &mut u32) {
    tool.wire_cells = 0;
    tool.release_pole_infos(world);
    tool.wire_vectors.clear();
    let mut anchor = 0;
    let mut last_clear = None;
    let mut last_clear_step = 0;
    let mut last_pole_step: i64 = -1;
    let mut active = false;
    let mut pending = false;
    let mut count = 0;
    let mut accumulator = Vec::new();
    let mut descriptor = SavedConnection::default();
    let mut last_position = [0.0; 3];
    let previous = run.straight_count;
    let forced_point = if previous > 0 && previous < run.valid_count && !tool.anchors.is_empty() {
        let range = tool.steps[previous as usize];
        (range.0..=range.1).find(|&i| run.cells[i].0 == tool.anchors[0].0 || run.cells[i].1 == tool.anchors[0].1)
    } else { None };
    let final_cell = if run.valid_count == 0 { None } else { Some(tool.steps[run.valid_count as usize - 1].1) };
    'steps: for step in 0..run.valid_count as usize {
        let range = tool.steps[step];
        let pivot_index = pivot(&run.cells, range, step, previous, tool.anchors.get(anchor).copied());
        for i in range.0..=range.1 {
            let cell = run.cells[i];
            let is_pivot = pivot_index == Some(i);
            let mut forced = forced_point == Some(i);
            if anchor < tool.anchors.len() && (cell == tool.anchors[anchor] || Some(i) == final_cell) {
                anchor += 1;
                forced = true;
            }
            if is_pivot && active && (step as u32).wrapping_sub(last_pole_step as u32) >= tool.params.v_3a4 {
                forced = true;
            }
            pending |= forced;
            let kind = classify(world, cell);
            if kind == CELL_CLEAR {
                *failure = 0;
                last_clear = Some(i);
                last_clear_step = step;
                if let Some(pole) = probe(world, cell, true) {
                    pending |= is_pivot;
                    world.release(pole);
                } else if let Some(wire) = probe(world, cell, false) {
                    descriptor = intercept(tool, world, wire, step as u32, previous, forced, &mut pending);
                }
                if is_pivot && !pending && !tool.poles.is_empty() && i + 1 < run.cells.len() {
                    let mut b = compute_pole_position(run.cells[i + 1].0, run.cells[i + 1].1, world);
                    b[1] = r24(f64::from(b[1]) + f64::from(tool.params.v_3a8)) as f32;
                    if ray_blocks(world, last_position, b) { pending = true; }
                }
                if pending {
                    emit(tool, cell, i, active, &mut accumulator, &mut descriptor);
                    last_position = compute_pole_position(cell.0, cell.1, world);
                    last_position[1] = r24(f64::from(last_position[1]) + f64::from(tool.params.v_3a8)) as f32;
                    active = true;
                    count += 1;
                    last_pole_step = step as i64;
                    last_clear = None;
                } else if Some(i) != final_cell {
                    accumulator.push(WireCell { x: cell.0, z: cell.1, pivot: is_pivot, wire: None });
                    tool.wire_cells = tool.wire_cells.wrapping_add(1);
                }
                pending = false;
            } else if kind == CELL_ZONED {
                if count == 1 && last_clear.is_none() {
                    count = 0;
                    discard_solitary(tool, world, false);
                } else if let Some(last) = last_clear.filter(|&l| l > 0 && active) {
                    emit(tool, run.cells[last], last, true, &mut accumulator, &mut descriptor);
                    count = 0;
                    last_clear = None;
                    last_pole_step = step as i64;
                }
                active = false;
                pending = true;
            } else {
                let guarded = (active && last_pole_step >= 0) || forced;
                if guarded {
                    if kind == CELL_BUILDING { *failure = FAIL_BUILDING; }
                    if kind == CELL_ROAD { *failure = FAIL_ROAD; }
                    if kind == CELL_BUILDING || forced {
                        if count == 1 && last_clear.is_none() {
                            count = 0;
                            discard_solitary(tool, world, true);
                        }
                        break 'steps;
                    }
                }
                if Some(i) != final_cell {
                    accumulator.push(WireCell { x: cell.0, z: cell.1, pivot: is_pivot, wire: None });
                    tool.wire_cells = tool.wire_cells.wrapping_add(1);
                }
            }
        }
    }
    if count == 1 { discard_solitary(tool, world, true); }
    run.valid_count = last_clear_step as u32 + 1;
}

pub fn terrain_highlight(run: &ToolRun, tool: &PowerLineTool, world: &mut impl World, start: u32) {
    for step in start..run.valid_count {
        let range = tool.steps[step as usize];
        for &cell in &run.cells[range.0..=range.1] {
            if !tool.poles.iter().any(|p| (p.x, p.z) == cell) { world.select(cell, 0x8000_00FF, [0; 3]); }
        }
    }
    for step in run.valid_count..run.dragged_count {
        let range = tool.steps[step as usize];
        for &cell in &run.cells[range.0..=range.1] { world.select(cell, 0x80FF_0000, [0; 3]); }
    }
}

pub fn create_record_pole(tool: &mut PowerLineTool, index: usize, world: &mut impl World) {
    let cell = (tool.poles[index].x, tool.poles[index].z);
    let found = probe(world, cell, true);
    let pole = match found {
        Some(pole) => pole,
        None => {
            let position = compute_pole_position(cell.0, cell.1, world);
            world.create_pole(cell, tool.poles[index].mask, position)
        }
    };
    tool.poles[index].pole = Some(pole);
    tool.poles[index].created = found.is_none();
}

pub fn place_poles(run: &mut ToolRun, tool: &mut PowerLineTool, world: &mut impl World, failure: &mut u32, allow_repair: bool) -> Result<(), GeometryBoundary> {
    for group in tool.groups.clone() {
        let indices: Vec<_> = group.iter().filter_map(|&cell_index| tool.poles.iter().position(|p| p.cell_index == cell_index && (tool.real || p.pole.is_none()))).collect();
        for &a in &indices {
            for &b in &indices {
                let pa = &tool.poles[a];
                let pb = &tool.poles[b];
                let distance = (f64::from(pa.x as u32) - f64::from(pb.x as u32)).abs() + (f64::from(pa.z as u32) - f64::from(pb.z as u32)).abs();
                if distance == 1.0 { tool.link(a, b, -1); }
            }
        }
    }
    tool.groups.clear();
    for i in 0..tool.poles.len() { create_record_pole(tool, i, world); }
    let mut repair = Vec::new();
    for i in 0..tool.poles.len() {
        let pole = tool.poles[i].pole.unwrap();
        let saved = tool.poles[i].saved;
        // Read endpoints in A,B order even when one or both flags are zero.
        for endpoint in saved.endpoints.into_iter().flatten() { let _ = world.pole_data(endpoint).cell; }
        if let [Some(a), Some(b)] = saved.endpoints {
            if tool.real {
                world.remove_connection(a, b, 0); world.remove_connection(b, a, 0);
            } else {
                enable_connection(world, a, b, false); enable_connection(world, b, a, false);
            }
        }
        for (endpoint, enabled) in saved.endpoints.into_iter().zip(saved.enabled) {
            if let Some(endpoint) = endpoint.filter(|_| enabled != 0) {
                world.add_ref(endpoint); world.add_ref(pole); repair.push((endpoint, pole));
            }
        }
        if !tool.real && saved.endpoints.iter().all(Option::is_some) {
            tool.edits.push(saved);
        } else {
            for endpoint in saved.endpoints.into_iter().flatten() { world.release(endpoint); }
        }
        tool.poles[i].saved.endpoints = [None; 2];
    }
    for i in 0..tool.poles.len() {
        let source = tool.poles[i].pole.unwrap();
        for (j, wire_index) in tool.poles[i].links.clone() {
            let destination = tool.poles[j].pole.unwrap();
            if world.pole_data(source).connections.iter().any(|c| c.other == Some(destination)) { continue; }
            let source_lesser = lesser((tool.poles[i].x, tool.poles[i].z), (tool.poles[j].x, tool.poles[j].z));
            let endpoints = if source_lesser { [source, destination] } else { [destination, source] };
            if tool.real && wire_index >= 0 {
                for wire in &mut tool.wire_vectors[wire_index as usize] {
                    if wire.wire.is_none() {
                        let position = compute_pole_position(wire.x, wire.z, world);
                        wire.wire = Some(world.create_wire((wire.x, wire.z), endpoints, wire.pivot, position, tool.real));
                    }
                }
            }
            // Draft geometry has no permanent wire interfaces to retain.
            let wires = if tool.real && wire_index >= 0 { Some(tool.wire_vectors[wire_index as usize].as_slice()) } else { None };
            connect(world, source, Some(destination), if source_lesser { wires } else { None })?;
            connect(world, destination, Some(source), if source_lesser { None } else { wires })?;
        }
    }
    if tool.real {
        for pole in &tool.poles {
            for &(_, wire_index) in &pole.links {
                if wire_index > 0 {
                    for wire in &mut tool.wire_vectors[wire_index as usize] {
                        if let Some(reference) = wire.wire.take() { world.release(reference); }
                    }
                }
            }
        }
    }
    for record in &mut tool.poles {
        let pole = record.pole.take().unwrap();
        if record.created { world.insert_pole(pole, 0); }
        if tool.real {
            world.refresh_pole(pole);
        } else {
            world.highlight_pole(pole, 3, 0);
            if record.created { tool.newly_created.push(pole); } else { tool.reused.push(pole); }
            world.add_ref(pole);
        }
        world.release(pole);
    }
    for (a, b) in repair {
        if !tool.real {
            connect(world, a, Some(b), None)?; connect(world, b, Some(a), None)?;
        } else if allow_repair {
            tool.release_pole_infos(world);
            tool.wire_vectors.clear();
            let anchors = [world.pole_data(a).cell, world.pole_data(b).cell];
            tool.anchors = anchors.to_vec();
            tool.steps = world.draw_line(run, anchors, 0);
            run.valid_count = run.dragged_count;
            determine_pole_positions(run, tool, world, failure);
            place_poles(run, tool, world, failure, false)?;
        }
        world.release(a); world.release(b);
    }
    Ok(())
}

pub fn undo_preview(tool: &mut PowerLineTool, world: &mut impl World) {
    for pole in tool.newly_created.drain(..) {
        world.highlight_pole(pole, 0, 0);
        if world.has_occupant_manager() { world.remove_pole(pole, true, false); world.release(pole); }
    }
    for pole in tool.reused.drain(..) {
        world.highlight_pole(pole, 0, 0); world.reset_pole_preview(pole, 0); world.release(pole);
    }
    for saved in tool.edits.drain(..) {
        if let Some(a) = saved.endpoints[0] {
            if let Some(b) = saved.endpoints[1] { enable_connection(world, a, b, true); }
            world.release(a);
        }
        if let Some(b) = saved.endpoints[1] {
            if let Some(a) = saved.endpoints[0] { enable_connection(world, b, a, true); }
            world.release(b);
        }
    }
    world.base_cleanup();
}
