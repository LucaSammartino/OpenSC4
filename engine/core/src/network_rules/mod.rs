// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network rules (`docs/specs/network-rules.md`): the **piece choice**
//! half of the network tool's solve. Section references (`§n`) point at that
//! spec.
//!
//! # Structure
//!
//! - Vocabulary (this file, §1): sides, network types and their bits, edge
//!   words and their helpers (`connmask`, `covers`, `D`, `within`), the
//!   orientation byte and the metaposition table.
//! - Number readers and the INI reader (`text.rs`, §1.6).
//! - Tile rule files (`tile.rs`, §3-§5): the reader, the `#Transmogrify`
//!   expansion and `Evaluate`.
//! - The rule database (`rules.rs`, §2, §8.1, §9.1, §9.2): [`NetworkRules`]
//!   holds every evaluator, the multi-type solutions, the override matrix,
//!   the `[RuleOverrides]` patches, the tile verifiers and the highway
//!   intersection rules, and loads them from resource bytes.
//! - The solve input (`grid.rs`): [`CellGrid`] (the tool's cell infos as a
//!   plain snapshot) and [`SolveState`] (the tool's sets and lists).
//! - The rule stages of `SolveNetwork` (`solve.rs`, §6-§9).
//! - The highway intersection rules (`intrule.rs`, §10).
//!
//! # The interface for the network geometry core
//!
//! The geometry core (network-geometry.md) owns the drag, the cell infos and
//! the commit. It calls into this module with plain data:
//!
//! 1. Build a [`CellGrid`] (city size, one [`CellInfo`] per cell info, with an
//!    [`OccupantView`] for cells that hold an existing piece) and a
//!    [`SolveState`] whose `s` is the set of cells to be solved (`+0x84`) and
//!    whose `valid_count` is `+0x78`.
//! 2. Optionally [`identify_highway_intersections`] (§10.6) before building
//!    the set, as the original does.
//! 3. [`solve_network`] (§6.1): returns the valid count; the solved list,
//!    the edited cell infos (edge words, subtypes, flags, one-way bytes,
//!    `+0xB8`) and the potential-intersection list are left in the grid and
//!    the state.
//!
//! Nothing here calls back into the game, except the three cell questions
//! of `cIntRule::Test` for new cells (water, a network occupant, the
//! demolition region), which go through [`IntRuleHost`].
//!
//! # Choices made where the spec is open (`[choice]`)
//!
//! Each is tagged in the code and listed in the lane report as an NR
//! question: the INI reader's grammar (comments, blanks, duplicate sections),
//! the tokenizer's line endings, metapositions outside 0-24, the flipped
//! rotation table past row 3, the padded set's scan order and the bucket
//! hints of the sets the spec does not size, the through-mark walk's step
//! cap, the field trimming of `RZParseDelimitedText`, the intersection-rule
//! map's bucket hint, and what `CopyFrom` copies of the layout lines.

mod grid;
mod intrule;
mod rules;
mod solve;
mod text;
mod tile;

#[cfg(test)]
mod tests;

pub use grid::{CellGrid, CellInfo, ExtraCell, IntersectionRecord, OccupantView, SolveState, SolvedRecord};
pub use intrule::{
    identify_highway_intersections, Bounds, CheckType, DryHost, IntRule, IntRuleHost, IntRuleMap, Model, TaggedCell,
    INTERSECTION_FLAG, ONE_WAY_PATTERNS,
};
pub use rules::{
    LoadReport, NetworkRules, OverridePair, Requirement, ResourceId, MULTI_TYPE_INSTANCE, NETWORK_INI_GROUP,
    NETWORK_INI_INSTANCE, OVERRIDES_INSTANCE, INTERSECTION_INSTANCE, RULE_GROUP, RULE_TYPE,
};
pub use solve::{
    adjust_tile_subsets, apply_network_rules, assign_edge_subtypes, build_padded_network_set, neighbourhood,
    patch_tile_pair, solve_multi_type_intersections, solve_network, solve_using_evaluator, unsolved_cell_result,
    verify_tile_subsets, GridNeighbourhood, TilePiece,
};
pub use text::{atol, find_section, ini_sections, rz_atou32_hex, rz_convert_u32, scan, IniSection, ScanValue};
pub use tile::{
    edge_token, mirror_edge_byte, mirror_edges, mirror_meta, rotate_edges, rotate_meta, rotate_orientation, Condition, Evaluator,
    LoadStatus, Neighbourhood, Output, TileRule,
};

// ---- §1.1 cells and sides ---------------------------------------------------------------------------

/// The neighbour offsets of sides W, N, E, S (`0x00AA89F8`, `0x00AA8A08`).
pub const SIDE_DX: [i32; 4] = [-1, 0, 1, 0];
pub const SIDE_DZ: [i32; 4] = [0, -1, 0, 1];

/// The cell key `(z << 16) + x` (unsigned 32-bit arithmetic).
pub fn cell_key(x: i32, z: i32) -> u32 {
    ((z as u32) << 16).wrapping_add(x as u32)
}

/// The cell of a key: x the low 16 bits, z the high 16 bits.
pub fn key_cell(key: u32) -> (i32, i32) {
    ((key & 0xFFFF) as i32, (key >> 16) as i32)
}

// ---- §1.2 network types -------------------------------------------------------------------------------

pub const ROAD: u8 = 0;
pub const RAIL: u8 = 1;
pub const HIGHWAY: u8 = 2;
pub const STREET: u8 = 3;
pub const PIPE: u8 = 4;
pub const POWER_LINE: u8 = 5;
pub const AVENUE: u8 = 6;
pub const SUBWAY: u8 = 7;
pub const LIGHT_RAIL: u8 = 8;
pub const MONORAIL: u8 = 9;
pub const ONE_WAY_ROAD: u8 = 10;
pub const DIRT_ROAD: u8 = 11;
pub const GROUND_HIGHWAY: u8 = 12;
/// The number of network types.
pub const TYPE_COUNT: usize = 13;

/// The mask of all type bits.
pub const ALL_TYPES: u32 = 0x1FFF;
/// Highway, LightRail, Monorail.
pub const ELEVATED_MASK: u32 = 0x304;
/// Road, Street, Avenue, DirtRoad.
pub const STREET_LEVEL_MASK: u32 = 0x849;
/// Cell flag: a bridge step.
pub const FLAG_BRIDGE: u32 = 0x40000;
/// Cell flag: a tunnel portal.
pub const FLAG_PORTAL: u32 = 0x80000;

/// The network names (`0x00AA845C`).
pub const TYPE_NAMES: [&str; TYPE_COUNT] = [
    "Road",
    "Rail",
    "Highway",
    "Street",
    "Pipe",
    "PowerLine",
    "Avenue",
    "Subway",
    "LightRail",
    "Monorail",
    "OneWayRoad",
    "DirtRoad",
    "GroundHighway",
];

/// The type bit of type `t`.
pub fn type_bit(t: u8) -> u32 {
    1u32 << (t as u32 & 31)
}

/// `NetworkNameToID` (`0x0063C7A0`): case-insensitive after trimming.
pub fn network_name_to_id(name: &str) -> Option<u8> {
    let n = name.trim();
    TYPE_NAMES
        .iter()
        .position(|t| t.eq_ignore_ascii_case(n))
        .map(|i| i as u8)
}

/// `typeOf(bits)` (`0x00623E60`, §8.2): the lowest type 0-11 whose bit is in
/// `bits`, and 12 when none is, so the GroundHighway bit and every non-type
/// flag bit read as type 12 *[quirk]*.
pub fn type_of(bits: u32) -> u8 {
    (0..12u8).find(|&t| bits & type_bit(t) != 0).unwrap_or(GROUND_HIGHWAY)
}

/// `elevated(x)` for a set of type bits (§8.2).
pub fn elevated(bits: u32) -> bool {
    bits & ELEVATED_MASK != 0
}

/// The multi-type priority table (`0x00AA8A18`, §8.2).
pub const PRIORITY: [u8; TYPE_COUNT] = [3, 5, 6, 2, 1, 0, 4, 1, 7, 7, 3, 0, 5];

/// The edge-subtype scheme of each type (record `+0x00`, §2.2, §7).
pub const SUBTYPE_SCHEME: [u8; TYPE_COUNT] = [1, 2, 0, 1, 1, 0, 0, 1, 2, 2, 1, 1, 0];

/// The exemplar group of each type's prebuilt-model pieces (record `+0x0C`,
/// §2.2, §11).
pub const PREBUILT_GROUP: [u32; TYPE_COUNT] = [
    0x2821_ED93,
    0xE834_7989,
    0xA843_4037,
    0xA92A_02EA,
    0xA951_171C,
    0x088E_1962,
    0xCB73_0FAC,
    0x8A15_F3F2,
    0x2B79_DFFB,
    0xEBE0_84C2,
    0xCBE0_84CB,
    0x6BE0_8658,
    0xEBE0_84D1,
];

/// The two tile rule instances of each type, in load order (§2.3; key table
/// `0x00B0B418`-`0x00B0B52C`). PowerLine loads none.
pub fn tile_instances(t: u8) -> &'static [u32] {
    const T: [&[u32]; TYPE_COUNT] = [
        &[0x8, 0x7],
        &[0x6, 0x5],
        &[0x2, 0x1],
        &[0xA, 0x9],
        &[0x4, 0x3],
        &[],
        &[0xE, 0xD],
        &[0xC, 0xB],
        &[0x10, 0xF],
        &[0x16, 0x15],
        &[0x12, 0x11],
        &[0x14, 0x13],
        &[0x18, 0x17],
    ];
    T.get(t as usize).copied().unwrap_or(&[])
}

/// The type and evaluator slot (0 first, 1 second) of a tile rule instance.
pub fn tile_instance_owner(instance: u32) -> Option<(u8, usize)> {
    (0..TYPE_COUNT as u8).find_map(|t| {
        tile_instances(t)
            .iter()
            .position(|&i| i == instance)
            .map(|slot| (t, slot))
    })
}

// ---- §1.3 edge words -----------------------------------------------------------------------------------

/// Byte `i` (side `i`) of an edge word.
pub fn edge_byte(e: u32, side: usize) -> u8 {
    (e >> (8 * (side & 3))) as u8
}

/// `e` with byte `side` replaced by `b`.
pub fn with_edge_byte(e: u32, side: usize, b: u8) -> u32 {
    let sh = 8 * (side & 3);
    (e & !(0xFFu32 << sh)) | (u32::from(b) << sh)
}

/// `connmask(e)`: `0x03` in each byte with bit 0 or bit 1 set.
pub fn connmask(e: u32) -> u32 {
    (0..4).fold(0, |m, i| if edge_byte(e, i) & 3 != 0 { m | (3 << (8 * i)) } else { m })
}

/// `covers(eW, eL)`: on every side where `eL` has a connection, `eW` has the
/// same low two bits.
pub fn covers(e_w: u32, e_l: u32) -> bool {
    (e_w ^ e_l) & connmask(e_l) == 0
}

/// `D(e)`: bits `2i` and `2i + 1` for each side `i` with a connection,
/// computed as the spec writes it.
pub fn d_mask(e: u32) -> u8 {
    let t = ((e & 0x0303_0303).wrapping_add(0x0303_0303) & 0x0404_0404).wrapping_mul(0x00C3_0C30);
    (t >> 24) as u8
}

/// `within(a, b)` (`0x00729EC0`), computed as the spec writes it *[compat]*.
pub fn within(a: u32, b: u32) -> bool {
    ((((a.wrapping_add(0x0F0F_0F0F)) >> 4) & 0x0101_0101).wrapping_mul(0xF) & (a ^ b)) == 0
}

/// Keeps bits 0-2 of each byte (`ResetEdgeMinorTypes`, `0x00621440`).
pub fn reset_minor_types(e: u32) -> u32 {
    e & 0x0707_0707
}

// ---- §1.4 orientation -----------------------------------------------------------------------------------

/// The orientation byte of `rot` and `flip`: `rot | (flip != 0 ? 0x80 : 0)`,
/// `rot` kept as a byte, not masked.
pub fn orientation(rot: i32, flip: i32) -> u8 {
    (rot as u8) | if flip != 0 { 0x80 } else { 0 }
}

// ---- §1.5 metapositions -------------------------------------------------------------------------------

/// The metaposition offsets (`0x00B0B3D8` dx, `0x00B0B3F4` dz).
pub const META_DX: [i32; 25] = [0, -1, -1, 0, 1, 1, 1, 0, -1, -2, -2, -2, -1, 0, 1, 2, 2, 2, 2, 2, 1, 0, -1, -2, -2];
pub const META_DZ: [i32; 25] = [0, 0, -1, -1, -1, 0, 1, 1, 1, 0, -1, -2, -2, -2, -2, -2, -1, 0, 1, 2, 2, 2, 2, 2, 1];

/// The offset of metaposition `m`. A metaposition outside 0-24 (the tables
/// have 25 entries) reads as the centre (0, 0) *[choice]*.
pub fn meta_offset(m: i32) -> (i32, i32) {
    if (0..25).contains(&m) {
        (META_DX[m as usize], META_DZ[m as usize])
    } else {
        (0, 0)
    }
}

/// The metaposition of an offset in the 5 x 5 square (the inverse grid
/// `0x00B0B3BC`).
pub fn meta_of(dx: i32, dz: i32) -> Option<i32> {
    (0..25).find(|&m| META_DX[m] == dx && META_DZ[m] == dz).map(|m| m as i32)
}
