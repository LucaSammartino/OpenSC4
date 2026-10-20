// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Tool state (network-tools.md §2): every field the tool paths read or
//! write, with the constructor defaults (`0x0062C5F0`).

use std::collections::BTreeMap;

/// The world cache (`+0x1C`, zones.md §8.8): a draft overlay of cell
/// records keyed by `(z << 16) + x`. Only `Reset` and a **real-mode** Place
/// empty it (§3.1 step 5, quirk 6); draft Places accumulate.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CacheRecord {
    /// `+0x51`: the draft byte — an existing network alone never answers
    /// `OccupantExists` true (zones.md §8.8).
    pub draft: bool,
    /// `+0x08`: the record's network flags.
    pub flags: u32,
    /// `+0x18 + 4·type`: the 4-byte edge mask of the tool's own type
    /// (`OccupantExists` returns it; byte i ≠ 0 = connection on edge i).
    pub edges: [u8; 4],
}

/// The per-type cost fields the tool copies from the placement exemplars
/// (§8.1, NT2: type info `+0xDC`/p+0x8C, and tool `+0x320`/`+0x324`/`+0x328`
/// = p+0x90/p+0x94/p+0x98).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolCosts {
    /// `+0x320`: bridge cost per cell (p+0x90, default 10).
    pub bridge_per_cell: i32,
    /// `+0x324`: tunnel cost per cell (p+0x94, default 20).
    pub tunnel_per_cell: i32,
    /// `+0x328`: earthwork cost per metre (p+0x98, default 0.1).
    pub earthwork_per_metre: f32,
}

impl Default for ToolCosts {
    fn default() -> Self {
        ToolCosts {
            bridge_per_cell: 10,
            tunnel_per_cell: 20,
            earthwork_per_metre: 0.1,
        }
    }
}

/// Every field of §2 the simulation sees. Field names cite the tool offset.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolState {
    /// `+0x35C`: the tool's network type (constructor argument).
    pub type_id: u32,
    /// `+0x50`: byte **real** — 1 while committing to the city, 0 while
    /// drafting (preview).
    pub real: bool,
    /// `+0x52`: byte **stop on failure** — when 1, a failed validation ends
    /// `PlaceNetwork` with false (default 1).
    pub stop_on_failure: bool,
    /// `+0x53`: byte set to 1 by `DestroyOccupantsAndResolve`.
    pub destroyed_flag: bool,
    /// `+0x208`: `data4` for occupant-manager inserts — 1 normally, 3 inside
    /// `PlaceNetworkOccupants` (§3.1 step 2; restored at §3.4, quirk: §6.2
    /// step 1's connection-node re-insert uses data4 **0**, quirk 10).
    pub data4: u32,
    /// `+0x20C`: **failure code** (§5.3); 0 = none. Stays until the next
    /// `PlaceNetworkOccupants` or `ClearFailureReason` (§5.2).
    pub failure: u32,
    /// `+0x234`: byte **preview update enabled** (slot 21; default 1). Gates
    /// the inserts in `PlaceNetwork` (§6.1 step 2).
    pub preview_update: bool,
    /// `+0x220`: demolition cost of what the drag would clear (computed by
    /// `UpdateOccupantHighlights`, §8.1).
    pub demolition_cost: i32,
    /// `+0x2C4`: float, the vertical tolerance of a single placed piece (§4).
    pub piece_tolerance: f32,
    /// `+0x320`/`+0x324`/`+0x328`: the cost trio (NT2).
    pub costs: ToolCosts,
    /// `+0x354`: byte — the type has land bridges (set by `Init`; quirk: set
    /// for every type with at least one bridge type, network-geometry §4.4).
    pub land_bridges: bool,
    /// `+0x360`: the layer mask — `0x1F4F` for the surface types, the type's
    /// own bit for pipe (4), power line (5) and subway (7) (§2).
    pub layer_mask: u32,
    /// `+0x364`: an OR occupant filter on `+0x360` (built in `Init`).
    pub layer_filter_mask: u32,
    /// `+0x36C`: underground tools — float **depth** below the terrain
    /// (property `0xA96E5641`; 0 for surface tools).
    pub depth: f32,
    /// `+0x370`/`+0x378`: the two floats whose sum is the **cover margin**
    /// (properties `0x496E564C`, `0x096E5653`; §11.1 step 5).
    pub cover_margin_a: f32,
    pub cover_margin_b: f32,
    /// `+0x240`/`+0x244`: city size in cells, X and Z (set in `Init`).
    pub city_x: u32,
    pub city_z: u32,
    /// `+0x248`: vertex row stride (X + 1).
    pub vertex_stride: u32,
    /// `+0x1C`: the world cache (zones.md §8.8).
    pub world_cache: BTreeMap<i32, CacheRecord>,
    /// `+0x98`: vector of solved cells (12 bytes each in the original; here
    /// as (x, z) pairs, in solve order).
    pub solved_cells: Vec<(i32, i32)>,
    /// `+0xA4`: hash set of solved cell keys (`(z << 16) + x`).
    pub solved_set: std::collections::BTreeSet<i32>,
}

impl ToolState {
    /// The constructor defaults (`0x0062C5F0`).
    pub fn new(type_id: u32) -> ToolState {
        // §2: the layer mask is `0x1F4F` for the surface types and the type's
        // own bit for pipe (4), power line (5) and subway (7).
        let surface = !matches!(type_id, 4 | 5 | 7);
        let layer_mask = if surface { 0x1F4F } else { 1u32 << type_id };
        ToolState {
            type_id,
            real: false,
            stop_on_failure: true,
            destroyed_flag: false,
            data4: 1,
            failure: 0,
            preview_update: true,
            demolition_cost: 0,
            piece_tolerance: 0.0,
            costs: ToolCosts::default(),
            land_bridges: false,
            layer_mask,
            layer_filter_mask: layer_mask,
            depth: 0.0,
            cover_margin_a: 0.0,
            cover_margin_b: 0.0,
            city_x: 0,
            city_z: 0,
            vertex_stride: 0,
            world_cache: BTreeMap::new(),
            solved_cells: Vec::new(),
            solved_set: std::collections::BTreeSet::new(),
        }
    }

    /// The tool's cell key: `(z << 16) + x` (§2; NG7 — no swapped key).
    pub fn cell_key(x: i32, z: i32) -> i32 {
        (z << 16).wrapping_add(x)
    }

    /// `Init` (slot 3): sets the city size, the vertex stride and the
    /// underground depth fields the host read from the exemplar
    /// (`ReloadNetworkSpecificInfo`, `0x00657980`). `land_bridges` follows
    /// the bridge-type quirk (§4.4: set exactly when the type has at least
    /// one bridge type).
    #[allow(clippy::too_many_arguments)]
    pub fn init(
        &mut self,
        city_x: u32,
        city_z: u32,
        depth: f32,
        cover_a: f32,
        cover_b: f32,
        piece_tolerance: f32,
        land_bridges: bool,
    ) {
        self.city_x = city_x;
        self.city_z = city_z;
        self.vertex_stride = city_x + 1;
        self.depth = depth;
        self.cover_margin_a = cover_a;
        self.cover_margin_b = cover_b;
        self.piece_tolerance = piece_tolerance;
        self.land_bridges = land_bridges;
    }

    /// `Reset` (slot 5): empties the world cache and the step counts
    /// (zones.md §8.8: only `Reset` and a real-mode Place empty the cache).
    pub fn reset(&mut self) {
        self.world_cache.clear();
        self.solved_cells.clear();
        self.solved_set.clear();
        self.failure = 0;
    }

    /// `ClearFailureReason` (slot 13).
    pub fn clear_failure(&mut self) {
        self.failure = 0;
    }
}
