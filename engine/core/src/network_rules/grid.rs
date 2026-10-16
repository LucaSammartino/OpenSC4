// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The solve input: the tool's cell infos (network-geometry.md §8.1) as a
//! plain snapshot, and the tool's sets and lists that the rule stages read
//! and write.

use std::collections::HashMap;

use crate::stl::StlHashSet;

use super::{cell_key, key_cell, TYPE_COUNT};

/// What the rule stages read from a cell's existing network occupant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OccupantView {
    /// Network occupant slot 23 (`PieceId`).
    pub piece_id: u32,
    /// Slot 26 (`GetOrientation`, byte `+0x104`).
    pub orientation: u8,
    /// Slot 8 (`GetNetworkFlag`).
    pub network_flags: u32,
    /// Slot 13 (`GetConnectionNodeEdgeTypes`): `None` when it returns false.
    pub node_edge_types: Option<(u32, u32)>,
    /// Slot 71 (`GetIntersectionAnchorCell`): `None` when it returns false.
    pub anchor: Option<(i32, i32)>,
}

/// One cell info (`cSC4NetworkCellInfo`, network-geometry.md §8.1): the
/// fields the rule stages use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellInfo {
    pub x: i32,
    pub z: i32,
    /// `+0x08`: the type bits and the other flags (`0x40000`, `0x80000`).
    pub flags: u32,
    /// `+0x18 + 4t`: the edge word of each type.
    pub edges: [u32; TYPE_COUNT],
    /// `+0x4C`: the union of the edge words.
    pub union: u32,
    /// `+0x50`: the one-way direction byte (two bits per side).
    pub one_way: u8,
    /// `+0x10`: part of the run.
    pub in_run: bool,
    /// `+0x14`: the step index.
    pub step: i32,
    /// `+0x52`: an intersection piece.
    pub intersection: bool,
    /// `+0x53`: fixed.
    pub fixed: bool,
    /// `+0x56`: a connection node.
    pub node: bool,
    /// `+0xB8`: the index of the cell's solved record, or -1.
    pub solved: i32,
    /// `+0x0C`: the existing occupant.
    pub occupant: Option<OccupantView>,
}

impl CellInfo {
    /// A cell info at (x, z) with no flags, no edges and no solved record.
    pub fn new(x: i32, z: i32) -> CellInfo {
        CellInfo {
            x,
            z,
            solved: -1,
            ..CellInfo::default()
        }
    }

    /// One type's bit and edge word (the union is recomputed).
    pub fn with_type(mut self, t: u8, edges: u32) -> CellInfo {
        self.flags |= super::type_bit(t);
        self.edges[t as usize] = edges;
        self.recompute_union();
        self
    }

    pub fn key(&self) -> u32 {
        cell_key(self.x, self.z)
    }

    /// Recomputes `+0x4C`.
    pub fn recompute_union(&mut self) {
        self.union = self.edges.iter().fold(0, |u, e| u | e);
    }

    /// Sets one type's edge word and recomputes the union.
    pub fn set_edges(&mut self, t: u8, e: u32) {
        self.edges[t as usize % TYPE_COUNT] = e;
        self.recompute_union();
    }
}

/// The tool's cell infos and the city size.
#[derive(Clone, Debug, Default)]
pub struct CellGrid {
    pub width: u32,
    pub height: u32,
    cells: HashMap<u32, CellInfo>,
}

impl CellGrid {
    pub fn new(width: u32, height: u32) -> CellGrid {
        CellGrid {
            width,
            height,
            cells: HashMap::new(),
        }
    }

    /// Adds or replaces a cell info (keyed by its x and z).
    pub fn insert(&mut self, c: CellInfo) {
        self.cells.insert(c.key(), c);
    }

    pub fn remove(&mut self, key: u32) -> Option<CellInfo> {
        self.cells.remove(&key)
    }

    pub fn get(&self, key: u32) -> Option<&CellInfo> {
        self.cells.get(&key)
    }

    pub fn get_mut(&mut self, key: u32) -> Option<&mut CellInfo> {
        self.cells.get_mut(&key)
    }

    pub fn at(&self, x: i32, z: i32) -> Option<&CellInfo> {
        self.get(cell_key(x, z))
    }

    pub fn at_mut(&mut self, x: i32, z: i32) -> Option<&mut CellInfo> {
        self.get_mut(cell_key(x, z))
    }

    /// The keys of every cell info, ascending.
    pub fn keys(&self) -> Vec<u32> {
        let mut k: Vec<u32> = self.cells.keys().copied().collect();
        k.sort_unstable();
        k
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Inside the city: both coordinates below the city size, compared
    /// unsigned.
    pub fn inside(&self, x: i32, z: i32) -> bool {
        (x as u32) < self.width && (z as u32) < self.height
    }

    /// The cell info at a key's cell moved by (dx, dz).
    pub fn neighbour(&self, key: u32, dx: i32, dz: i32) -> Option<&CellInfo> {
        let (x, z) = key_cell(key);
        self.at(x + dx, z + dz)
    }
}

/// One solved record (`+0x98`, 12 bytes): `{piece id, orientation, cell key}`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SolvedRecord {
    pub id: u32,
    pub orientation: u8,
    pub key: u32,
}

/// One highway intersection record (`+0xCC`, 40 bytes, §10.6).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IntersectionRecord {
    pub rule_id: u32,
    pub x: i32,
    pub z: i32,
    pub start_step: i32,
    pub step_a: i32,
    pub step_b: i32,
    /// The empty vector *[inferred]*.
    pub cells: Vec<u32>,
}

/// One extra cell of `DoAutoComplete` (the tool's list `+0xF0`, §10.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExtraCell {
    pub x: i32,
    pub z: i32,
    pub edges: u32,
    pub network_type: u8,
    pub one_way: u8,
    pub one_way_mask: u8,
}

/// The tool's sets and lists the rule stages use.
#[derive(Clone, Debug)]
pub struct SolveState {
    /// `+0x84`: the cells to be solved (`BuildSetOfCellsToBeSolved`).
    pub s: StlHashSet,
    /// `+0x98`: the solved list.
    pub solved: Vec<SolvedRecord>,
    /// `+0x08`: the potential-intersection list `(x, z, value)`.
    pub potential: Vec<(i32, i32, u32)>,
    /// `+0xCC`: the highway intersection records.
    pub intersections: Vec<IntersectionRecord>,
    /// `+0xF0`: the extra cells.
    pub extra_cells: Vec<ExtraCell>,
    /// `+0x78`: the valid count.
    pub valid_count: i32,
}

impl Default for SolveState {
    fn default() -> Self {
        SolveState::new()
    }
}

impl SolveState {
    pub fn new() -> SolveState {
        SolveState {
            s: StlHashSet::default(),
            solved: Vec::new(),
            potential: Vec::new(),
            intersections: Vec::new(),
            extra_cells: Vec::new(),
            valid_count: 0,
        }
    }

    /// Appends a solved record and points the cell's `+0xB8` at it.
    pub fn append_solved(&mut self, grid: &mut CellGrid, rec: SolvedRecord) {
        let idx = self.solved.len() as i32;
        self.solved.push(rec);
        if let Some(c) = grid.get_mut(rec.key) {
            c.solved = idx;
        }
    }

    /// Removes record `i`: the cell goes back into `S`, every later record's
    /// cell has its `+0xB8` decremented; the removed cell's own `+0xB8` is
    /// left as it was (§9.1 quirk 5, §9.2).
    pub fn unsolve(&mut self, grid: &mut CellGrid, i: usize) {
        let rec = self.solved[i];
        self.s.insert(rec.key);
        for later in &self.solved[i + 1..] {
            if let Some(c) = grid.get_mut(later.key) {
                c.solved -= 1;
            }
        }
        self.solved.remove(i);
    }
}
