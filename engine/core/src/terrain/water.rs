// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The surface water's cell queries, navigability and bodies of water
//! (terrain.md §10).

use std::collections::VecDeque;

use crate::grid::CellRect;
use crate::landfill::spiral_cells;
use crate::x87::r24;

use super::field::Terrain;
use super::{water_type, BRIDGE_CLEARANCE, DRAFT_CLEARANCE};

/// A bridge on a cell (`IsNavigable` step 4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bridge {
    /// No network tile with flag `0x40000` (bridge) on the cell.
    None,
    /// A bridge tile; its deck height (tile slot 67) is unknown.
    UnknownDeck,
    /// A bridge tile with this deck height.
    Deck(f32),
}

/// What `IsNavigable` reads from other subsystems (§10.4).
pub trait NavigationHost {
    /// The lot at the cell centre (city slot 97 at y = sea level, lot
    /// manager slot 5), 0 for none.
    fn lot_at_cell(&self, x: i32, z: i32) -> u64;
    /// The traffic network tile at the cell (`FindNetworkTile(x, z, 0x1F4F,
    /// 1)`) if it is a bridge.
    fn bridge_at(&self, x: i32, z: i32) -> Bridge;
    /// Whether the occupant manager has a power pole (type `0x2890D4DE`)
    /// whose cell is `(x, z)`.
    fn has_power_pole(&self, x: i32, z: i32) -> bool;
}

/// A host with no lots, bridges or poles.
pub struct OpenWater;

impl NavigationHost for OpenWater {
    fn lot_at_cell(&self, _x: i32, _z: i32) -> u64 {
        0
    }
    fn bridge_at(&self, _x: i32, _z: i32) -> Bridge {
        Bridge::None
    }
    fn has_power_pole(&self, _x: i32, _z: i32) -> bool {
        false
    }
}

impl Terrain {
    /// `GetWaterDepthForCell(x, z)` (surface water slot 6, §10.2): `i :=
    /// GetVertIndex(x, z)` with no bounds check; the minimum of the depths at
    /// `i`, `i + 1`, `i + 1 + vertsX`, `i + vertsX` (in that order, a NaN
    /// candidate never replacing it); that minimum if `> 0`, else 0.0.
    /// [choice] An index outside the arrays reads as NaN.
    pub fn water_depth_for_cell(&self, x: i32, z: i32) -> f32 {
        let i = i64::from(x) + i64::from(z) * i64::from(self.verts_x);
        let vx = i64::from(self.verts_x);
        let d = |j: i64| {
            (0..self.water.len() as i64)
                .contains(&j)
                .then(|| self.water[j as usize].depth)
                .unwrap_or(f32::NAN)
        };
        let mut m = d(i);
        for j in [i + 1, i + 1 + vx, i + vx] {
            let c = d(j);
            if c < m {
                m = c;
            }
        }
        if m > 0.0 {
            m
        } else {
            0.0
        }
    }

    /// `HasDraftClearance(x, z)` (slot 13): `GetWaterDepthForCell > 5.0`.
    pub fn has_draft_clearance(&self, x: i32, z: i32) -> bool {
        self.water_depth_for_cell(x, z) > DRAFT_CLEARANCE
    }

    /// `GetWaterType(x, z)` (slot 12): at the vertex `GetVertIndex(x, z)`,
    /// `depth ≤ 0` → 0; else `level > sea` (or NaN) → 2, otherwise 3.
    /// [choice] An index outside the arrays gives 0.
    pub fn water_type(&self, x: i32, z: i32) -> u32 {
        let Some(i) = self.raw_index(i64::from(x), i64::from(z)) else {
            return water_type::LAND;
        };
        let w = self.water[i];
        if w.depth <= 0.0 {
            return water_type::LAND;
        }
        let level = r24(f64::from(w.depth) + f64::from(self.alt[i])) as f32;
        if level <= self.sea_level {
            water_type::SEA
        } else {
            water_type::FRESH
        }
    }

    /// `IsNavigable(x, z, r, lot)` (slot 8, §10.4): every cell of the square
    /// `[x − r, x + r] × [z − r, z + r]`, visited with the spiral iterator
    /// from `(x, z)` outward, must be in the city, have draft clearance, hold
    /// no lot other than `lot` (when `lot ≠ 0`) and no power pole. A bridge
    /// cell decides at once: true if its deck clears the water by 20 m,
    /// false otherwise *[quirk: the rest of the square is not checked]*.
    ///
    /// [choice] "Out of the city" (city slot 99) is the terrain's cell
    /// bounds; the spiral is the clockwise one of plumbing §7.2.
    pub fn is_navigable(&self, x: i32, z: i32, r: i32, lot: u64, host: &dyn NavigationHost) -> bool {
        let (x0, x1) = (x.wrapping_sub(r), x.wrapping_add(r));
        let (z0, z1) = (z.wrapping_sub(r), z.wrapping_add(r));
        let rect = CellRect::new(x0, z0, x1.max(x0), z1.max(z0));
        for (cx, cz) in spiral_cells(rect, (x, z)) {
            if !self.cell_in_bounds(cx, cz) || !self.has_draft_clearance(cx, cz) {
                return false;
            }
            if lot != 0 {
                let l = host.lot_at_cell(cx, cz);
                if l != 0 && l != lot {
                    return false;
                }
            }
            match host.bridge_at(cx, cz) {
                Bridge::None => {}
                Bridge::UnknownDeck => return false,
                Bridge::Deck(deck) => {
                    let i = cx as usize + cz as usize * self.verts_x as usize;
                    let surface = r24(f64::from(self.alt[i]) + f64::from(self.water[i].depth));
                    return r24(f64::from(deck) - surface) >= f64::from(BRIDGE_CLEARANCE);
                }
            }
            if host.has_power_pole(cx, cz) {
                return false;
            }
        }
        true
    }
}

/// The body-of-water map (surface water `+0x4C`, slot 9; §10.3): one uint16
/// per cell, 0 = none, and the dirty flag (`+0x48`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BodiesOfWater {
    cells_x: u32,
    cells_z: u32,
    map: Vec<u16>,
    dirty: bool,
}

impl BodiesOfWater {
    /// An empty map for a city of `cells_x × cells_z` cells.
    pub fn new(cells_x: u32, cells_z: u32) -> BodiesOfWater {
        BodiesOfWater {
            cells_x,
            cells_z,
            map: vec![0; (cells_x as usize) * (cells_z as usize)],
            dirty: false,
        }
    }

    pub fn cells_x(&self) -> u32 {
        self.cells_x
    }
    pub fn cells_z(&self) -> u32 {
        self.cells_z
    }

    /// The body ID of a cell (0 = none or out of bounds).
    pub fn get(&self, x: i32, z: i32) -> u16 {
        if (x as u32) < self.cells_x && (z as u32) < self.cells_z {
            self.map[x as usize + z as usize * self.cells_x as usize]
        } else {
            0
        }
    }

    /// The map, `x + z × cellsX`.
    pub fn map(&self) -> &[u16] {
        &self.map
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }

    /// `UpdateBodiesOfWater` (§10.3): clears the map, then scans the cells x
    /// outer, z inner; each unlabelled navigable cell (`IsNavigable(x, z, 0,
    /// 0)`) gets the next ID and a FIFO flood fill over the 4-neighbours
    /// (pushed west, east, north, south) labels every navigable cell it
    /// reaches. The map is resized to the terrain. [choice] IDs wrap past
    /// 65535 (0 is skipped); the dirty flag is cleared.
    pub fn update(&mut self, terrain: &Terrain, host: &dyn NavigationHost) {
        self.cells_x = terrain.cells_x();
        self.cells_z = terrain.cells_z();
        self.map = compute_bodies(terrain, host);
        self.dirty = false;
    }

    /// Installs a map computed elsewhere (see [`compute_bodies`]).
    pub fn set_map(&mut self, cells_x: u32, cells_z: u32, map: Vec<u16>) -> bool {
        if map.len() != (cells_x as usize) * (cells_z as usize) {
            return false;
        }
        self.cells_x = cells_x;
        self.cells_z = cells_z;
        self.map = map;
        self.dirty = false;
        true
    }

    /// SimNewDay (§10.3): rebuilds the map when the dirty flag is set.
    /// Returns whether it did.
    pub fn on_new_day(&mut self, terrain: &Terrain, host: &dyn NavigationHost) -> bool {
        if !self.dirty {
            return false;
        }
        self.update(terrain, host);
        true
    }

    /// A power pole inserted or removed (`0x99EF1142` / `0x99EF1143`, pole
    /// slot 13 false) on a cell with `depth > 0` sets the dirty flag (§10.3).
    /// [choice] "Depth > 0" is `GetWaterDepthForCell(x, z) > 0`.
    pub fn note_power_pole_changed(&mut self, terrain: &Terrain, x: i32, z: i32) {
        if terrain.water_depth_for_cell(x, z) > 0.0 {
            self.dirty = true;
        }
    }
}

/// The labelling of [`BodiesOfWater::update`], as a new map
/// (`x + z × cellsX`).
pub fn compute_bodies(terrain: &Terrain, host: &dyn NavigationHost) -> Vec<u16> {
    let (cx, cz) = (terrain.cells_x() as i32, terrain.cells_z() as i32);
    let at = |x: i32, z: i32| x as usize + z as usize * cx as usize;
    let mut map = vec![0u16; (cx as usize) * (cz as usize)];
    let mut next: u16 = 0;
    let mut queue = VecDeque::new();
    for x in 0..cx {
        for z in 0..cz {
            if map[at(x, z)] != 0 || !terrain.is_navigable(x, z, 0, 0, host) {
                continue;
            }
            next = next.wrapping_add(1);
            if next == 0 {
                next = 1;
            }
            map[at(x, z)] = next;
            queue.push_back((x, z));
            while let Some((px, pz)) = queue.pop_front() {
                for (nx, nz) in [(px - 1, pz), (px + 1, pz), (px, pz - 1), (px, pz + 1)] {
                    if terrain.cell_in_bounds(nx, nz)
                        && map[at(nx, nz)] == 0
                        && terrain.is_navigable(nx, nz, 0, 0, host)
                    {
                        map[at(nx, nz)] = next;
                        queue.push_back((nx, nz));
                    }
                }
            }
        }
    }
    map
}
