// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network lot configuration (§2.4) and the pure rules of the network
//! lot manager and the terrain handler: `IsLotCompatible` (§8.4), the zone
//! and wealth choice (§8.2, §8.3), the road style tables (§8.7), the lot
//! object position (§8.6), the pothole offsets (§8.8), the terrain rectangle
//! and the corner rule (§7.6).

use crate::grid::CellRect;
use crate::property::PropertyHolder;

use super::Position;

/// Configuration properties (§2.4).
pub mod prop {
    /// ExemplarType (property `0x10`) of a network lot configuration.
    pub const EXEMPLAR_TYPE_VALUE: u32 = 0x21;
    pub const PIECE_ID: u32 = 0xC9A5_A1BE;
    pub const ID: u32 = 0x21;
    pub const NAME: u32 = 0x20;
    pub const ZONES: u32 = 0x88ED_C793;
    pub const WEALTHS: u32 = 0x88ED_C795;
    pub const MASK: u32 = 0x49D5_5951;
    pub const PERIOD: u32 = 0xCA81_B8D4;
    pub const MIN_SLOPE: u32 = 0xAA12_0972;
    pub const MAX_SLOPE: u32 = 0xAA12_0973;
    pub const ROTATIONS: u32 = 0xEC3B_D470;
    pub const FLIP_RULE: u32 = 0xCC3E_4755;
    /// Below 2 the original writes the configuration back (tooling only;
    /// FreeCity4 skips it, the spec's *[choice]*).
    pub const VERSION: u32 = 0x88ED_C789;
}

/// Object types of a configuration's object array (§8.6).
pub const OBJECT_PROP: u8 = 1;
pub const OBJECT_FLORA: u8 = 4;

/// One 52-byte lot configuration object as the network lot manager reads it
/// (§8.6). The host builds them (`ConvertPropertiesToObjectArray` and
/// `CorrectObjectPositions` for a 16 × 16 m lot, city-detail.md).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetworkLotObject {
    /// `+0`: the object type (1 prop, 4 flora).
    pub kind: u8,
    /// `+1`: the flags byte (bit 0 set: no prop is created).
    pub flags: u8,
    /// `+2`: the orientation `o`.
    pub orientation: u8,
    /// `+4`, `+8`, `+0xC`: the offsets `ox`, `oy`, `oz`.
    pub offset: [f32; 3],
    /// `+0x24`: passed to a created prop's slot 28.
    pub field_24: u32,
}

/// A network lot configuration (`cSC4NetworkLotConfiguration`, §2.4).
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkLotConfig {
    /// The host's identity of the configuration (for
    /// [`super::NetworkLotHost::object_ids`]).
    pub token: u64,
    /// The decorated network piece (required).
    pub piece_id: u32,
    /// The configuration ID (saved in network lot records).
    pub id: u32,
    pub name: Vec<u8>,
    /// Allowed zone types (16 × false by default).
    pub zones: [bool; 16],
    /// Allowed wealths (4 × false by default).
    pub wealths: [bool; 4],
    /// Position pattern rows (`0F 0F 0F 0F` by default).
    pub mask: [u8; 4],
    /// The pattern's period `n` (4 by default).
    pub period: u8,
    pub min_slope: f32,
    pub max_slope: f32,
    /// Bit r allows piece rotation r (`0x0F` by default).
    pub rotations: u8,
    /// 0 any; 1 the piece must be flipped; 2 it must not be; others any.
    pub flip_rule: u8,
    pub objects: Vec<NetworkLotObject>,
}

impl Default for NetworkLotConfig {
    /// The constructor's values (`0x0060FA90`).
    fn default() -> NetworkLotConfig {
        NetworkLotConfig {
            token: 0,
            piece_id: 0,
            id: 0,
            name: Vec::new(),
            zones: [false; 16],
            wealths: [false; 4],
            mask: [0x0F; 4],
            period: 4,
            min_slope: 0.0,
            max_slope: 64.0,
            rotations: 0x0F,
            flip_rule: 0,
            objects: Vec::new(),
        }
    }
}

impl NetworkLotConfig {
    /// `ReadLotConfiguration` (`0x0060F5F0`, §2.4) of an exemplar's
    /// properties: `None` unless the exemplar type (property `0x10`) is
    /// `0x21` and the piece ID is present. The objects are left empty (the
    /// host converts them).
    pub fn from_exemplar(h: &PropertyHolder, token: u64) -> Option<NetworkLotConfig> {
        use crate::props::{f32_of, string_of, u32_of, u32s};
        if !crate::props::has_exemplar_type(h, prop::EXEMPLAR_TYPE_VALUE) {
            return None;
        }
        let mut c = NetworkLotConfig {
            token,
            piece_id: u32_of(h, prop::PIECE_ID)?,
            ..NetworkLotConfig::default()
        };
        if let Some(v) = u32_of(h, prop::ID) {
            c.id = v;
        }
        if let Some(v) = string_of(h, prop::NAME) {
            c.name = v;
        }
        // Values ≥ 16 (zones) and ≥ 4 (wealths) are ignored; a scalar reads
        // as an empty list (EX39), which allows nothing.
        for z in u32s(h, prop::ZONES).unwrap_or_default() {
            if let Some(b) = c.zones.get_mut((z & 0xFF) as usize) {
                *b = true;
            }
        }
        for w in u32s(h, prop::WEALTHS).unwrap_or_default() {
            if let Some(b) = c.wealths.get_mut((w & 0xFF) as usize) {
                *b = true;
            }
        }
        if let Some(m) = u32s(h, prop::MASK) {
            if m.len() == 4 {
                for (d, s) in c.mask.iter_mut().zip(&m) {
                    *d = *s as u8;
                }
            }
        }
        if let Some(v) = u32_of(h, prop::PERIOD) {
            c.period = v as u8;
        }
        if let Some(v) = f32_of(h, prop::MIN_SLOPE) {
            c.min_slope = v;
        }
        if let Some(v) = f32_of(h, prop::MAX_SLOPE) {
            c.max_slope = v;
        }
        if let Some(v) = u32_of(h, prop::ROTATIONS) {
            c.rotations = v as u8;
        }
        if let Some(v) = u32_of(h, prop::FLIP_RULE) {
            c.flip_rule = v as u8;
        }
        Some(c)
    }

    /// The pattern row byte `row` (§8.4 step 1): rows 0–3 are `mask`, row 4
    /// reads the period's byte and rows 5–7 read 0 *[compat]*; other rows
    /// read 0 [choice].
    pub fn pattern_row(&self, row: i32) -> u8 {
        match row {
            0..=3 => self.mask[row as usize],
            4 => self.period,
            _ => 0,
        }
    }
}

/// `IsLotCompatible` steps 1–5 (§8.4): the position pattern, the zone, the
/// wealth, the rotation and the flip rule. The slope (step 6) is the
/// caller's, tested last. A period of 0 is never compatible [choice] (the
/// original divides by zero); zones outside 0..15, wealths outside 0..3,
/// rotations outside 0..7 and columns outside 0..7 never match.
pub fn is_lot_compatible_pattern(
    cfg: &NetworkLotConfig,
    x: i32,
    z: i32,
    zone: i32,
    wealth: i32,
    rotation: i32,
    flip: i32,
) -> bool {
    // 1. Position (signed division).
    let n = i32::from(cfg.period);
    if n == 0 {
        return false;
    }
    let row = z.wrapping_rem(n);
    let col = x.wrapping_rem(n);
    if !(0..8).contains(&col) || cfg.pattern_row(row) & (1u8 << col) == 0 {
        return false;
    }
    // 2., 3.
    if !usize::try_from(zone)
        .ok()
        .and_then(|i| cfg.zones.get(i))
        .copied()
        .unwrap_or(false)
    {
        return false;
    }
    if !usize::try_from(wealth)
        .ok()
        .and_then(|i| cfg.wealths.get(i))
        .copied()
        .unwrap_or(false)
    {
        return false;
    }
    // 4.
    if !(0..8).contains(&rotation) || cfg.rotations & (1u8 << rotation) == 0 {
        return false;
    }
    // 5.
    match cfg.flip_rule {
        1 => flip != 0,
        2 => flip == 0,
        _ => true,
    }
}

/// `IsLotCompatible` step 6 (§8.4): `minSlope ≤ s ≤ maxSlope` as floats; a
/// NaN fails.
pub fn slope_fits(cfg: &NetworkLotConfig, slope: f32) -> bool {
    cfg.min_slope <= slope && slope <= cfg.max_slope
}

/// The neighbour offsets of §8.2 and §8.3, in visiting order (tables
/// `0x00AA7858` / `0x00AA7878`): W, N, E, S, then the diagonals.
pub const NEIGHBOUR_ORDER: [(i32, i32); 8] = [(-1, 0), (0, -1), (1, 0), (0, 1), (-1, -1), (-1, 1), (1, -1), (1, 1)];

/// The zone priority `P` (table `0x00AA76E8`, lower wins), `None` outside
/// 0..15 [choice].
pub fn zone_priority(zone: i32) -> Option<u8> {
    const P: [u8; 16] = [18, 4, 7, 6, 5, 4, 3, 8, 2, 1, 16, 16, 16, 16, 17, 16];
    usize::try_from(zone).ok().and_then(|i| P.get(i)).copied()
}

fn inside(x: i32, z: i32, size: (i32, i32)) -> bool {
    x >= 0 && z >= 0 && x < size.0 && z < size.1
}

/// `GetZoneTypeForNetwork(x, z)` (slot 9, §8.2): the cell's own zone when it
/// is not 0; otherwise the neighbour of lowest priority (ties keep the
/// earlier one), the diagonals only when the best after the four sides is
/// none, agriculture or landfill.
pub fn zone_choice(x: i32, z: i32, size: (i32, i32), zone_at: impl Fn(i32, i32) -> i32) -> i32 {
    let own = zone_at(x, z);
    if own != 0 {
        return own;
    }
    let mut best = 0;
    for (i, (dx, dz)) in NEIGHBOUR_ORDER.iter().enumerate() {
        if i == 4 && !matches!(best, 0 | 7 | 14) {
            break;
        }
        let (nx, nz) = (x + dx, z + dz);
        if !inside(nx, nz, size) {
            continue;
        }
        let n = zone_at(nx, nz);
        if let (Some(pn), Some(pb)) = (zone_priority(n), zone_priority(best)) {
            if pn < pb {
                best = n;
            }
        }
    }
    best
}

/// `GetWealthForNetwork(x, z)` (slot 10, §8.3). `lot_wealth(x, z)` is the
/// effective wealth of the lot at a cell (`None`: no lot); values outside
/// 0..3 count as 0. The diagonals are visited only when none of the four
/// sides had a lot. Ties go to the higher wealth.
pub fn wealth_choice(x: i32, z: i32, size: (i32, i32), lot_wealth: impl Fn(i32, i32) -> Option<i32>) -> i32 {
    let mut count = [0u32; 4];
    let mut any = false;
    for (i, (dx, dz)) in NEIGHBOUR_ORDER.iter().enumerate() {
        if i == 4 && any {
            break;
        }
        let (nx, nz) = (x + dx, z + dz);
        if !inside(nx, nz, size) {
            continue;
        }
        if let Some(w) = lot_wealth(nx, nz) {
            any = true;
            let w = if (0..=3).contains(&w) { w as usize } else { 0 };
            count[w] += 1;
        }
    }
    let mut r = 0usize;
    for w in 1..4 {
        if count[w] != 0 && count[r] <= count[w] {
            r = w;
        }
    }
    r as i32
}

/// The under-texture table `T[zone][wealth]` (table `0x00AA7758`, §8.7).
/// Wealths outside 0..3 are clamped and zones outside 0..15 take the "every
/// other zone" row [choice].
pub fn road_texture(zone: i32, wealth: i32) -> u32 {
    let w = wealth.clamp(0, 3) as usize;
    match zone {
        0 => [0, 0x0810_0000, 0x0820_0000, 0x0830_0000][w],
        7 | 14 => 0x0840_0000,
        8 | 9 => [0x0820_0000, 0x0810_0000, 0x0820_0000, 0x0820_0000][w],
        _ => [0x0820_0000, 0x0810_0000, 0x0820_0000, 0x0830_0000][w],
    }
}

/// The variation `V[zone][max(wealth − 1, 0)]` (table `0x00AA7728`, §8.7).
/// Zones outside 0..15 take the row of zones 10–13 [choice].
pub fn road_variation(zone: i32, wealth: i32) -> u32 {
    let w = (wealth - 1).clamp(0, 2) as usize;
    match zone {
        0 => 0,
        1 | 4 => [1, 2, 3][w],
        2 | 3 | 5 | 6 => [4, 5, 6][w],
        7 | 14 => 7,
        8 | 9 => [1, 1, 2][w],
        _ => 1,
    }
}

/// `GetPositionForLotConfigurationObject` (`0x0060C9B0`, §8.6): the
/// position and orientation of an object with offsets `offset` and
/// orientation byte `o` on a piece at `cell` with rotation `rotation` (its
/// low byte) and `flip`. Every operation is single precision; `y` is the
/// altitude at the point before the mirror *[quirk]*.
pub fn object_position(
    cell: (i32, i32),
    rotation: i32,
    flip: bool,
    o: u8,
    offset: [f32; 3],
    altitude: impl Fn(f32, f32) -> f32,
) -> (Position, u32) {
    let x0 = cell.0.wrapping_mul(16) as f32;
    let z0 = cell.1.wrapping_mul(16) as f32;
    let r = (rotation & 0xFF) as u32;
    let [ox, oy, oz] = offset;
    let (mut x, z) = match r {
        0 => ((x0 + 16.0) - ox, (z0 + 16.0) - oz),
        1 => (x0 + oz, (z0 + 16.0) - ox),
        2 => (x0 + ox, z0 + oz),
        3 => ((x0 + 16.0) - oz, z0 + ox),
        _ => (0.0, 0.0),
    };
    let y = altitude(x, z) + oy;
    let orientation = if flip {
        x = (16.0 - 2.0 * (x - x0)) + x;
        (-(i64::from(o) + i64::from(r))).rem_euclid(4) as u32
    } else {
        (u32::from(o) + r) & 3
    };
    ([x, y, z], orientation)
}

/// A pothole offset `(dx, dz)` in cells.
pub type PotholeOffset = (i8, i8);

/// The pothole offset `(DXE[e], DZE[e])` (signed bytes, table `0x00AA7910`,
/// §8.8) for the low byte of `GetActiveEdges()`; values past the 16 entries
/// give (0, 0) [choice].
pub fn pothole_offset(e: u32) -> PotholeOffset {
    match e {
        1 => (-1, 0),
        2 => (0, -1),
        3 => (-1, -1),
        4 => (1, 0),
        6 => (1, -1),
        8 => (0, 1),
        9 => (-1, 1),
        12 => (1, 1),
        _ => (0, 0),
    }
}

/// §7.6 step 1: the rectangle of a terrain message, clamped to the city.
pub fn terrain_rect(data1: u32, data2: u32, size: (i32, i32)) -> CellRect {
    let x1 = (data1 >> 16) as i32 - 1;
    let z1 = (data2 >> 16) as i32 - 1;
    let x2 = (data1 & 0xFFFF) as i32;
    let z2 = (data2 & 0xFFFF) as i32;
    CellRect {
        x1: x1.max(0),
        z1: z1.max(0),
        x2: x2.min(size.0 - 1),
        z2: z2.min(size.1 - 1),
    }
}

/// The corner rule of §7.6 (`0x006105F0`) for one corner's
/// `d = r24(occupant altitude − terrain altitude)`: below the terrain
/// (`d < 0.0f`) or more than 0.01 above (the double `0x3F847AE147AE147B`)
/// mismatches; a NaN does not *[compat]*.
pub fn corner_mismatch(d: f32) -> bool {
    d < 0.0 || f64::from(d) > 0.01
}
