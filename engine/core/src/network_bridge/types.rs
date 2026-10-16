// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The bridge type record (`0x178` bytes, §2.3), its defaults, the ramp
//! heights (§2.4) and the shipped columns (§2.6).
//!
//! The record is modelled as named fields; the offset of every field is in
//! its doc comment so the two views can be checked against each other.

use crate::x87::r24;

/// §2.3: the default maximum span (property `0x482CB467` is never set).
pub const DEFAULT_MAX_SPAN: u32 = 100_000;
/// §2.3: the default maximum clearance (`0x482CD464`).
pub const DEFAULT_MAX_CLEARANCE: f32 = 10_000.0;
/// §2.3: the default maximum pier count (`0x482CB45B`).
pub const DEFAULT_MAX_PIER_COUNT: u32 = 100;
/// §2.3: the default minimum clearance (`0x482CD463`).
pub const DEFAULT_MIN_CLEARANCE: f32 = 0.0;

/// One ramp or pier piece (§2.3 `+0x14`, `+0x44`): the instance and its
/// rotation. The rotation count is truncated to a byte when read (quirk 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PieceDef {
    /// The exemplar instance.
    pub instance: u32,
    /// The rotation, 0–3.
    pub rotation: u8,
}

/// One of a pillar footprint's four points (§2.3 `+0x80`): an (x, z) offset
/// around the occupant's position. The loader holds the four points of the
/// piece's own rotation and derives the other three by rotating about the
/// origin; [`BridgeType`] stores all sixteen.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Footprint {
    pub x: f32,
    pub z: f32,
}

/// The bridge type record (§2.3). `0x178` bytes in the original.
#[derive(Clone, Debug, PartialEq)]
pub struct BridgeType {
    /// `+0x00`: the INI key.
    pub key: u32,
    /// `+0x04`, `0x482CB458`: the minimum span in steps (default `2N + 2`).
    pub min_span: u32,
    /// `+0x08`, `0x482CB467`: the maximum span in steps.
    pub max_span: u32,
    /// `+0x0C`, `0x482CB45F`: W, the pieces per step (lanes).
    pub width: u32,
    /// `+0x10`, `0x482CB442`: N, the ramp steps at each end.
    pub ramp_steps: u32,
    /// `+0x14`, `0x482CB443`/`0x482CB444`: the N × W ramp pieces.
    pub ramp_pieces: Vec<PieceDef>,
    /// `+0x20`, `0x482CB44D`: the N ramp model depths (−1 = never a column).
    pub ramp_depths: Vec<f32>,
    /// `+0x2C`, `0x482CB445`: the deck piece instance.
    pub deck_instance: u32,
    /// `+0x30`, `0x482CB446`: the deck piece rotation.
    pub deck_rotation: u8,
    /// `+0x34`, `0x482CB448`: the pier spacing in steps.
    pub pier_spacing: u32,
    /// `+0x38`, `0x482CB459`: the first-pier offset in steps (default `+0x34`).
    pub first_pier: u32,
    /// `+0x3C`, `0x482CD465`: the end-pier fraction (§10.2).
    pub end_pier_fraction: f32,
    /// `+0x40`, `0x482CB448`: M, the steps per pier group.
    pub pier_steps: u32,
    /// `+0x44`, `0x482CB449`/`0x482CB44A`: the M × W pier pieces.
    pub pier_pieces: Vec<PieceDef>,
    /// `+0x50`, `0x482CB44C`: the M pier model depths.
    pub pier_depths: Vec<f32>,
    /// `+0x5C`, `0x482CB441`: the deck height above **water**.
    pub deck_height_water: f32,
    /// `+0x60`, from `0x482CB44B`: the N + 1 cumulative ramp drops (§2.4).
    pub ramp_drops: Vec<f32>,
    /// `+0x6C`, from `0x482CB44B`: the N ramp model offsets (§2.4).
    pub ramp_offsets: Vec<f32>,
    /// `+0x78`: `+0x60[0]`, the total ramp rise, the deck height above land.
    pub total_ramp_rise: f32,
    /// `+0x7C`: the last ramp index i ≥ 1 whose model depth is ≥ 0, else 0.
    pub last_column_ramp: u32,
    /// `+0x80`, `0x482CB44E`: the pillar footprint of the ramp and deck
    /// pieces: 4 (x, z) points per rotation, 4 rotations.
    pub ramp_footprint: [[Footprint; 4]; 4],
    /// `+0x8C`, `0x482CB45E`: the same for the pier pieces (default
    /// `+0x80`'s points).
    pub pier_footprint: [[Footprint; 4]; 4],
    /// `+0x98`, `0x482CB44F`: a **land** bridge type.
    pub land: bool,
    /// `+0x99`, `0x482CB450`: a **water** bridge type.
    pub water: bool,
    /// `+0x9C`, `0x482CB455`: the suspension parameter between two pylons.
    pub suspension_pylon: f32,
    /// `+0xA0`, `0x482CB456`: the suspension parameter between a pylon and
    /// an end.
    pub suspension_end: f32,
    /// `+0xA4`, `0x482CB457`: copied into each supported piece.
    pub support_value: f32,
    /// `+0xA8`, `0x482CB45A`: the minimum pier count (§10.2).
    pub min_piers: u32,
    /// `+0xAC`, `0x482CB45B`: the maximum pier count (§10.2).
    pub max_piers: u32,
    /// `+0xB0`, `0x482CB45C`: the first cable model id.
    pub cable_a: u32,
    /// `+0xB4`, `0x482CB45D`: the second cable model id. Both non-zero make
    /// this a **cable** bridge (§10.4).
    pub cable_b: u32,
    /// `+0xB8`–`+0xC4`, `0x482CB471`, `0x482CD460`–`0x482CD462`: the cable
    /// floats. `+0xC4` is also added to a pier piece's model height.
    pub cable_params: [f32; 4],
    /// `+0xC8`, `0x482CD463`: the minimum clearance (§6).
    pub min_clearance: f32,
    /// `+0xCC`, `0x482CD464`: the maximum clearance.
    pub max_clearance: f32,
    /// `+0xD0`, `0x482CB468`: the model bound offset (§11.2 step 4).
    pub model_bound_offset: f32,
    /// `+0xD4`, `0x482CB472`: added to every piece's model height.
    pub model_height_offset: f32,
    /// `+0xD8`–`+0x144`, `0x482CB469`–`0x482CB46E`: the suspension tower
    /// data (visual, §11.4). `+0xD8` is a byte: all of it present.
    ///
    /// [choice] Held as an opaque blob: §11.4 says it is drawing data and
    /// nothing in the simulation reads it.
    pub tower_data: [u8; 0x148 - 0xD8],
    /// `+0x148`, `+0x14C`, `0x482CB46F`: the menu bitmap key (slot 40).
    pub bitmap_key: (u32, u32),
    /// `+0x150`, `0x482CB470` value 0: the fixed cost (§12).
    pub cost_fixed: u32,
    /// `+0x154`, `0x482CB470` value 1: the cost per step (§12).
    pub cost_per_step: u32,
    /// `+0x15C`: the name (§2.2 token 3, or the localised `0x482CD466`).
    pub name: String,
    /// `+0x16C`: the length map, step count → specialisation (§3).
    pub has_specialization: bool,
}

impl Default for BridgeType {
    /// The **default bridge type record** §2.2 step 1 appends before parsing.
    /// `min_span` is `2N + 2` with N = 0 here, so 2.
    fn default() -> BridgeType {
        BridgeType {
            key: 0,
            min_span: 2,
            max_span: DEFAULT_MAX_SPAN,
            width: 1,
            ramp_steps: 0,
            ramp_pieces: Vec::new(),
            ramp_depths: Vec::new(),
            deck_instance: 0,
            deck_rotation: 0,
            pier_spacing: 0,
            first_pier: 0,
            end_pier_fraction: 0.0,
            pier_steps: 0,
            pier_pieces: Vec::new(),
            pier_depths: Vec::new(),
            deck_height_water: 0.0,
            ramp_drops: vec![0.0],
            ramp_offsets: Vec::new(),
            total_ramp_rise: 0.0,
            last_column_ramp: 0,
            ramp_footprint: [[Footprint::default(); 4]; 4],
            pier_footprint: [[Footprint::default(); 4]; 4],
            land: false,
            water: false,
            suspension_pylon: 0.0,
            suspension_end: 0.0,
            support_value: 0.0,
            min_piers: 0,
            max_piers: DEFAULT_MAX_PIER_COUNT,
            cable_a: 0,
            cable_b: 0,
            cable_params: [0.0; 4],
            min_clearance: DEFAULT_MIN_CLEARANCE,
            max_clearance: DEFAULT_MAX_CLEARANCE,
            model_bound_offset: 0.0,
            model_height_offset: 0.0,
            tower_data: [0; 0x148 - 0xD8],
            bitmap_key: (0, 0),
            cost_fixed: 0,
            cost_per_step: 0,
            name: String::new(),
            has_specialization: false,
        }
    }
}

/// π/2, the rotation the pillar footprints turn by (§2.3, `0x00AA8D30`).
pub const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;

/// Rotates a footprint point by `k` quarter turns (§2.3): "the same rotated
/// by 90°, 180° and 270°".
pub fn rotate_footprint(p: Footprint, k: u32) -> Footprint {
    let (s, c) = (k as f64 * HALF_PI).sin_cos();
    let (x, z) = (f64::from(p.x), f64::from(p.z));
    Footprint {
        x: r24(x * c - z * s) as f32,
        z: r24(x * s + z * c) as f32,
    }
}

/// Builds the four rotation variants of a four-point footprint (§2.3).
/// `base[0]` is the footprint for rotation 0; variant `r` is `base` turned by
/// `r` quarter turns.
pub fn footprint_variants(base: &[Footprint; 4]) -> [[Footprint; 4]; 4] {
    let mut out = [[Footprint::default(); 4]; 4];
    for (r, slot) in out.iter_mut().enumerate() {
        for (k, p) in base.iter().enumerate() {
            slot[k] = rotate_footprint(*p, r as u32);
        }
    }
    out
}

/// §2.4: the ramp heights from `0x482CB44B`'s N pairs `(a_i, b_i)`, read
/// from the last ramp step back.
///
/// - `+0x60[N] := 0`;
/// - `+0x60[i] := r24((b_i − a_i) + +0x60[i + 1])`;
/// - `+0x6C[i] := r24(+0x60[i + 1] + b_i)`.
///
/// Returns `(drops, offsets)`: `+0x60` has N + 1 entries, `+0x6C` has N.
/// With N = 0 the drops are `[0.0]` and `+0x78` is 0.
pub fn ramp_heights(pairs: &[(f32, f32)]) -> (Vec<f32>, Vec<f32>) {
    let n = pairs.len();
    let mut drops = vec![0.0f32; n + 1];
    let mut offsets = vec![0.0f32; n];
    for i in (0..n).rev() {
        let (a, b) = pairs[i];
        let next = f64::from(drops[i + 1]);
        drops[i] = r24(r24(f64::from(b) - f64::from(a)) + next) as f32;
        offsets[i] = r24(next + f64::from(b)) as f32;
    }
    (drops, offsets)
}

/// §2.3 `+0x7C`: the last ramp index i ≥ 1 whose model depth is ≥ 0, else 0.
pub fn last_column_ramp(depths: &[f32]) -> u32 {
    let mut out = 0u32;
    for (i, &d) in depths.iter().enumerate().skip(1) {
        if d >= 0.0 {
            out = i as u32;
        }
    }
    out
}

impl BridgeType {
    /// The default minimum span (§2.3): `2N + 2`.
    pub fn default_min_span(ramp_steps: u32) -> u32 {
        2 * ramp_steps + 2
    }

    /// `+0x78`: the total ramp rise, the deck height above land (§2.4).
    pub fn total_ramp_rise(&self) -> f32 {
        self.ramp_drops.first().copied().unwrap_or(0.0)
    }

    /// Both cable models set (§10.4): a cable bridge.
    pub fn is_cable(&self) -> bool {
        self.cable_a != 0 && self.cable_b != 0
    }

    /// `+0x14`/`+0x44` are N × W and M × W; this rebuilds the derived
    /// `total_ramp_rise` and `last_column_ramp` after the properties land.
    pub fn finish_derived(&mut self) {
        self.total_ramp_rise = self.ramp_drops.first().copied().unwrap_or(0.0);
        self.last_column_ramp = last_column_ramp(&self.ramp_depths);
    }
}

/// The columns §2.6 publishes for the tests: everything the shipped
/// exemplars set that the tests read. The fields the exemplars do not set
/// keep their §2.3 defaults (`max_span` 100000, the clearance range
/// `[0, 10000]` and the cost `(0, 0)`) *[compat]*.
#[derive(Clone, Debug, PartialEq)]
pub struct ShippedBridgeType {
    pub key: u32,
    /// The exemplar instance.
    pub instance: u32,
    pub name: &'static str,
    pub min_span: u32,
    /// `+0x5C`, the deck height above water.
    pub deck_height_water: f32,
    /// N, the ramp steps.
    pub ramp_steps: u32,
    /// M, the steps per pier group.
    pub pier_steps: u32,
    /// W, the pieces per step.
    pub width: u32,
    /// `+0x34`, the pier spacing.
    pub pier_spacing: u32,
    /// `+0x38`, the first-pier offset.
    pub first_pier: u32,
    /// `+0x7C`, the last ramp index with a column.
    pub last_column_ramp: u32,
    /// `+0xD4`, the model height offset (ground highway only).
    pub model_height_offset: f32,
    /// `+0x148`, `+0x14C`: the menu bitmap key (slot 40, §1.1).
    pub bitmap_key: (u32, u32),
}

/// The shipped bridge types of one network type (§2.6). Only the columns the
/// tests use are published; the pieces, depths and footprints come from the
/// exemplars and are defaults here.
pub fn shipped_types(type_id: u32) -> Vec<ShippedBridgeType> {
    use ShippedBridgeType as S;
    let s = |key, instance, name, min_span, deck, n, m, w, u34, u38, u7c, d4, bitmap| S {
        key,
        instance,
        name,
        min_span,
        deck_height_water: deck,
        ramp_steps: n,
        pier_steps: m,
        width: w,
        pier_spacing: u34,
        first_pier: u38,
        last_column_ramp: u7c,
        model_height_offset: d4,
        bitmap_key: bitmap,
    };
    match type_id {
        // 0 road (§2.6).
        0 => vec![
            s(1, 0x1, "level", 10, 20.0, 3, 3, 1, 6, 6, 0, 0.0, (0x6A38_6D26, 1)),
            s(2, 0x2, "raised", 16, 0.0, 7, 1, 1, 5, 5, 4, 0.0, (0, 0)),
            s(3, 0x3, "undertruss", 4, 20.0, 1, 1, 1, 4, 4, 0, 0.0, (0, 0)),
            s(4, 0x5, "wooden covered", 4, 20.0, 1, 1, 1, 2, 2, 0, 0.0, (0, 0)),
            s(5, 0x6, "medium suspension", 22, 35.0, 1, 1, 1, 16, 6, 0, 0.0, (0, 0)),
            s(6, 0x7, "stone arch", 4, 20.0, 1, 1, 1, 4, 4, 0, 0.0, (0, 0)),
            s(7, 0x8, "small steel arch", 10, 20.0, 1, 1, 1, 4, 4, 0, 0.0, (0, 0)),
            s(8, 0x9, "cantilever truss", 12, 20.0, 1, 1, 1, 4, 4, 0, 0.0, (0, 0)),
            s(9, 0xA, "overdeck truss", 4, 20.0, 1, 1, 1, 6, 6, 0, 0.0, (0, 0)),
        ],
        // 1 rail: one type published; the other four are instances
        // 0x03000002..0x03000005 (§2.6).
        1 => vec![s(1, 0x4, "overtruss", 4, 15.5, 1, 1, 1, 3, 3, 0, 0.0, (0, 0))],
        // 2 highway: three of the eleven published (§2.6).
        2 => vec![
            s(1, 0x0200_0001, "level", 4, 0.0, 0, 1, 2, 5, 5, 0, 0.0, (0, 0)),
            s(2, 0x0200_0002, "Golden Gate", 43, 35.119, 3, 1, 2, 35, 6, 0, 0.0, (0, 0)),
            s(5, 0x0200_0005, "cable stayed", 32, 45.0, 1, 1, 2, 15, 7, 0, 0.0, (0, 0)),
        ],
        // 8 light rail, 9 monorail: one each (§2.6).
        8 => vec![s(1, 0x0800_0001, "", 4, 0.0, 0, 1, 1, 3, 3, 0, 0.0, (0, 0))],
        9 => vec![s(1, 0x0D00_0001, "", 4, 0.0, 0, 1, 1, 3, 3, 0, 0.0, (0, 0))],
        // 12 ground highway repeats highway's with `+0xD4` = −15.5 (§2.6).
        12 => vec![
            s(1, 0x0A00_0001, "level", 4, 0.0, 0, 1, 2, 5, 5, 0, -15.5, (0, 0)),
            s(2, 0x0A00_0002, "Golden Gate", 43, 35.119, 3, 1, 2, 35, 6, 0, -15.5, (0, 0)),
            s(5, 0x0A00_0005, "cable stayed", 32, 45.0, 1, 1, 2, 15, 7, 0, -15.5, (0, 0)),
        ],
        // 3 street, 4 pipe, 5 power line, 6 avenue, 7 subway, 10 one-way
        // road, 11 dirt road: no published column set here. Avenue's nine
        // (0x04000001..0x04000009) and the empty types are §2.1.
        _ => Vec::new(),
    }
}

/// Turns a §2.6 row into a full record with the §2.3 defaults. The ramp
/// pairs of road key 1 and key 2 (§2.6) are filled in so the ramp heights
/// match the tests.
pub fn from_shipped(type_id: u32, s: &ShippedBridgeType) -> BridgeType {
    let mut t = BridgeType {
        key: s.key,
        min_span: s.min_span,
        width: s.width,
        ramp_steps: s.ramp_steps,
        pier_steps: s.pier_steps,
        deck_height_water: s.deck_height_water,
        pier_spacing: s.pier_spacing,
        first_pier: s.first_pier,
        model_height_offset: s.model_height_offset,
        bitmap_key: s.bitmap_key,
        name: s.name.to_string(),
        ..BridgeType::default()
    };
    // §2.6: the two ramp pair sets the tests read.
    let pairs: Vec<(f32, f32)> = match (type_id, s.key) {
        (0, 1) => vec![(0.0, 0.0); 3],
        (0, 2) => vec![
            (-1.747, 0.0),
            (-4.958, 0.0),
            (-7.686, 0.0),
            (-7.741, 0.0),
            (-6.789, 0.0),
            (-4.893, 0.0),
            (-2.838, 0.0),
        ],
        _ => vec![(0.0, 0.0); s.ramp_steps as usize],
    };
    let (drops, offsets) = ramp_heights(&pairs);
    t.ramp_drops = drops;
    t.ramp_offsets = offsets;
    // §2.6: road key 1's pier and ramp depths, and `+0xD0`.
    if type_id == 0 && s.key == 1 {
        t.pier_depths = vec![498.0, -1.0, -1.0];
        t.ramp_depths = vec![256.0, -1.0, -1.0];
        t.model_bound_offset = 15.0;
    } else if type_id == 0 && s.key == 2 {
        t.ramp_depths = vec![100.0, -1.0, -1.0, 100.0, 100.0, -1.0, -1.0];
    } else {
        t.pier_depths = vec![-1.0; s.pier_steps as usize];
        t.ramp_depths = vec![-1.0; s.ramp_steps as usize];
    }
    t.finish_derived();
    t
}

/// The shipped bridge types of a network type as full records (§2.6).
pub fn shipped_records(type_id: u32) -> Vec<BridgeType> {
    shipped_types(type_id)
        .iter()
        .map(|s| from_shipped(type_id, s))
        .collect()
}
