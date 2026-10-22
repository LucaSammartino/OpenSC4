// SPDX-License-Identifier: GPL-3.0-or-later
//! Ordered records and explicit reference ownership (power-line-tool §§4–6).

use super::{control_points, direction, lesser, sample_cubic, CurveParams, Point, PowerLineTool, World};
use crate::x87::ftol32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Reference(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavedConnection {
    pub endpoints: [Option<Reference>; 2],
    pub enabled: [u8; 2],
}

impl Default for SavedConnection {
    fn default() -> Self {
        Self { endpoints: [None; 2], enabled: [1; 2] }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PoleInfo {
    pub index: usize,
    pub x: i32,
    pub z: i32,
    pub cell_index: usize,
    pub pole: Option<Reference>,
    pub created: bool,
    pub mask: u8,
    pub links: Vec<(usize, i32)>,
    pub saved: SavedConnection,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WireCell {
    pub x: i32,
    pub z: i32,
    pub wire: Option<Reference>,
    pub pivot: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Connection {
    pub other: Option<Reference>,
    pub wires: Vec<Reference>,
    pub byte8: u8,
    pub enabled: u8,
    pub cell: (i32, i32),
    pub geometry: Vec<Vec<Point>>,
    pub extra: Vec<Vec<Point>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PoleData {
    pub cell: (i32, i32),
    pub position: Point,
    /// Terrain slots 91/92, already stored as float32 by pole Init.
    pub divisors: [f32; 2],
    pub attachments: [[Point; 4]; 4],
    pub connections: Vec<Connection>,
    pub dirty: bool,
}

impl PowerLineTool {
    pub fn link(&mut self, a: usize, b: usize, wire: i32) {
        let bit = 1u8 << direction((self.poles[a].x, self.poles[a].z), (self.poles[b].x, self.poles[b].z));
        self.poles[a].links.push((b, wire));
        self.poles[b].links.push((a, wire));
        self.poles[a].mask |= bit;
        self.poles[b].mask |= bit;
    }

    pub fn append_pole(&mut self, cell: (i32, i32), cell_index: usize, previous: bool, wire: i32, saved: SavedConnection) {
        let index = self.poles.len();
        self.poles.push(PoleInfo { index, x: cell.0, z: cell.1, cell_index, saved, ..PoleInfo::default() });
        if previous && index != 0 {
            self.link(index - 1, index, wire);
        }
    }

    pub fn release_pole_infos(&mut self, world: &mut impl World) {
        for pole in &mut self.poles {
            for endpoint in &mut pole.saved.endpoints {
                if let Some(p) = endpoint.take() {
                    world.release(p);
                }
            }
        }
        // Structural clear never releases pole.pole or wire-cell payloads (§§1,6.5).
        self.poles.clear();
    }

    pub fn descriptor_present(&self, descriptor: SavedConnection) -> bool {
        self.poles.iter().any(|p| p.saved == descriptor)
    }

    pub fn endpoint_aligned(&self, world: &impl World, endpoint: Option<Reference>) -> bool {
        endpoint.is_some_and(|p| {
            let cell = world.pole_data(p).cell;
            self.poles.iter().any(|p| p.x == cell.0 || p.z == cell.1)
        })
    }
}

pub fn enable_connection(world: &mut impl World, pole: Reference, other: Reference, enabled: bool) {
    // The read happens even when the connection list is empty; enabling rereads it.
    let _cell = world.pole_data(other).cell;
    let index = world.pole_data(pole).connections.iter().position(|c| c.other == Some(other));
    if let Some(i) = index {
        let value = if enabled {
            lesser(world.pole_data(pole).cell, world.pole_data(other).cell) as u8
        } else {
            0
        };
        world.pole_data_mut(pole).connections[i].enabled = value;
    }
}

pub fn copy_wire_references(world: &mut impl World, destination: &mut Vec<Reference>, source: Option<&[WireCell]>) {
    if let Some(source) = source {
        for wire in source {
            // Native makes no null-interface test; absent payload is an unsupported caller input.
            let reference = wire.wire.expect("wire-reference copy requires a nonnull interface");
            world.add_ref(reference);
            destination.push(reference);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryBoundary {
    /// Preserve the resource result; do not repair N to an invented minimum.
    UnsupportedSampleCount(i32),
}

/// Actual connection owner body. Deep copies buffers/nodes, never duplicates payload AddRefs.
pub fn connect(world: &mut impl World, pole: Reference, other: Option<Reference>, wires: Option<&[WireCell]>) -> Result<(), GeometryBoundary> {
    let Some(other) = other else { return Ok(()) };
    let other_cell = world.pole_data(other).cell;
    let data = world.pole_data(pole);
    if data.connections.iter().any(|c| c.cell == other_cell) {
        return Ok(());
    }
    let self_cell = (
        ftol32(crate::x87::r24(f64::from(data.position[0]) / f64::from(data.divisors[0]))),
        ftol32(crate::x87::r24(f64::from(data.position[2]) / f64::from(data.divisors[1]))),
    );
    if self_cell == other_cell {
        return Ok(());
    }
    let orientation = direction(self_cell, other_cell) as usize;
    let enabled = lesser(self_cell, other_cell);
    world.add_ref(other);
    let mut record = Connection { other: Some(other), cell: other_cell, enabled: enabled as u8, ..Connection::default() };
    let stored_cell = world.pole_data(pole).cell;
    let extra = world.extra_geometry(stored_cell) || world.extra_geometry(other_cell);
    if enabled {
        let attachments = |data: &PoleData| data.attachments[orientation].map(|point| std::array::from_fn(|i| (f64::from(point[i]) + f64::from(data.position[i])) as f32));
        let first = attachments(world.pole_data(pole));
        let second = attachments(world.pole_data(other));
        let params: CurveParams = world.curve_params();
        for (a, b) in first.into_iter().zip(second) {
            let controls = control_points(a, b, params);
            if controls.count < 1 {
                // Native allocator failure for invalid N is not specified. No count clamp/geometry credit.
                world.release(other);
                return Err(GeometryBoundary::UnsupportedSampleCount(controls.count));
            }
            let mut main = Vec::with_capacity(controls.count as usize);
            main.push(a);
            main.extend(sample_cubic(a, controls.first, controls.second, b, controls.count as usize - 1));
            record.geometry.push(main);
            if extra {
                record.extra.push(sample_cubic(a, controls.first, controls.second, b, 3));
            }
        }
        copy_wire_references(world, &mut record.wires, wires);
        world.notify_pole(pole, 0x99EF_1145, 0);
    }
    // Clone is intentionally deep for all Vecs and raw for reference tokens, matching §5.3.
    world.pole_data_mut(pole).connections.push(record.clone());
    world.pole_data_mut(pole).dirty = true;
    Ok(())
}

