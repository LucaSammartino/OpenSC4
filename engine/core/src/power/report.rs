// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The recompute report (tooling, not part of the spec): what one recompute
//! cycle saw at each stage, for the plugin's log. Collected only while the
//! host has switched it on, so the host calls of the cycle are unchanged
//! otherwise (the report's own reads are getters: the plant's properties and
//! its budget item).

use std::fmt::Write as _;

use crate::grid::CellRect;

/// One plant as state 1 found it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlantReport {
    pub occupant: u64,
    /// `visited` when state 1 reached it (0: it seeded a flood).
    pub visited: u32,
    /// The position and the seed cell made from it.
    pub position: [f32; 3],
    pub seed: (i32, i32),
    /// The bounding-box cells tested against the mark (§6.3 step 2).
    pub rect: CellRect,
    pub capacity: Option<u32>,
    /// The budget item's local funding fraction (None: no item).
    pub funding: Option<f32>,
    pub condition: f32,
    pub efficiency: f32,
    pub produced: u32,
    /// Whether some flood reached its rectangle.
    pub connected: bool,
}

/// One `UpdatePowerGrid` call.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FloodReport {
    pub seed: (i32, i32),
    /// Whether the seed cell is wired.
    pub seed_wired: bool,
    /// Cells marked by the flood, and its row range.
    pub reached: u32,
    pub rows: (i32, i32),
    pub supply: u32,
    pub import: u32,
    pub available: u32,
    /// Marked cells with a consumption above 0, and their sum.
    pub consumers: u32,
    pub need: u32,
    /// Consumer cells of the blocks that were powered.
    pub powered: u32,
    pub shortfall: bool,
}

/// One recompute cycle (states 0–3).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    /// Recompute cycles since the report was switched on (state 0 counts).
    pub cycle: u32,
    pub cells: (i32, i32),
    pub cell_widths: (f32, f32),
    pub wire_size: (i32, i32),
    pub wire_words: usize,
    pub wire_bits: u32,
    pub consumption_cells: u32,
    pub plants: Vec<PlantReport>,
    pub floods: Vec<FloodReport>,
    /// The totals after state 3.
    pub production: u32,
    pub consumption: u32,
    pub demand: u32,
    pub powered: u32,
    pub unpowered: u32,
    pub complete: bool,
}

impl Report {
    /// The report as log lines (`\n`-separated, no trailing newline).
    pub fn text(&self) -> String {
        let mut t = String::new();
        let _ = write!(
            t,
            "power recompute {}: cells {}x{} widths {}x{}; wire {}x{} words {} bits set {}; consumption cells {}; \
             plants {}, floods {}; production {} consumption {} demand {} powered {} unpowered {}",
            self.cycle,
            self.cells.0,
            self.cells.1,
            self.cell_widths.0,
            self.cell_widths.1,
            self.wire_size.0,
            self.wire_size.1,
            self.wire_words,
            self.wire_bits,
            self.consumption_cells,
            self.plants.len(),
            self.floods.len(),
            self.production,
            self.consumption,
            self.demand,
            self.powered,
            self.unpowered
        );
        for p in &self.plants {
            let cap = p.capacity.map_or_else(|| "none".to_string(), |c| c.to_string());
            let fund = p.funding.map_or_else(|| "no item".to_string(), |f| format!("{f:.3}"));
            let _ = write!(
                t,
                "\n  plant {:#x}: visited {} position ({:.1}, {:.1}, {:.1}) seed ({}, {}) cells [{},{}..{},{}] \
                 capacity {} funding {} condition {:.3} efficiency {:.3} produced {} connected {}",
                p.occupant,
                p.visited,
                p.position[0],
                p.position[1],
                p.position[2],
                p.seed.0,
                p.seed.1,
                p.rect.x1,
                p.rect.z1,
                p.rect.x2,
                p.rect.z2,
                cap,
                fund,
                p.condition,
                p.efficiency,
                p.produced,
                u8::from(p.connected)
            );
        }
        for f in &self.floods {
            let _ = write!(
                t,
                "\n  flood from ({}, {}) wired {}: reached {} cells, rows {}..{}; supply {} import {} available {}; \
                 consumers {} need {}; powered {}{}",
                f.seed.0,
                f.seed.1,
                u8::from(f.seed_wired),
                f.reached,
                f.rows.0,
                f.rows.1,
                f.supply,
                f.import,
                f.available,
                f.consumers,
                f.need,
                f.powered,
                if f.shortfall { " (shortfall)" } else { "" }
            );
        }
        t
    }
}
