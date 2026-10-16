// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The bridge type INI sections (§2.1), one entry (§2.2) and the water and
//! land lists (§2.5).

use crate::network_rules::{find_section, ini_sections};

use super::types::{
    footprint_variants, ramp_heights, BridgeType, Footprint, PieceDef, DEFAULT_MAX_SPAN,
};
use super::BridgeHost;

/// An exemplar property value (§2.3). The shapes the record reads.
#[derive(Clone, Debug, PartialEq)]
pub enum PropValue {
    /// A `u32` (the integer properties).
    U32(u32),
    /// A `float`.
    F32(f32),
    /// A bool.
    Bool(bool),
    /// A list of `u32` (the piece instances, the cable and bitmap ids).
    U32s(Vec<u32>),
    /// A list of `float` (the depths, the ramp pairs, the footprints).
    F32s(Vec<f32>),
}

impl PropValue {
    fn u32(&self) -> Option<u32> {
        match self {
            PropValue::U32(v) => Some(*v),
            PropValue::Bool(b) => Some(*b as u32),
            _ => None,
        }
    }
    fn f32(&self) -> Option<f32> {
        match self {
            PropValue::F32(v) => Some(*v),
            PropValue::U32(v) => Some(*v as f32),
            _ => None,
        }
    }
    fn bool(&self) -> Option<bool> {
        match self {
            PropValue::Bool(b) => Some(*b),
            PropValue::U32(v) => Some(*v != 0),
            _ => None,
        }
    }
    fn u32s(&self) -> Option<&[u32]> {
        match self {
            PropValue::U32s(v) => Some(v),
            _ => None,
        }
    }
    fn f32s(&self) -> Option<&[f32]> {
        match self {
            PropValue::F32s(v) => Some(v),
            _ => None,
        }
    }
}

/// The property ids of §2.3. A property the record does not read
/// (`0x482CB451`–`0x482CB454`) has no constant here.
pub mod prop {
    /// `+0x04`: the minimum span.
    pub const MIN_SPAN: u32 = 0x482C_B458;
    /// `+0x08`: the maximum span.
    pub const MAX_SPAN: u32 = 0x482C_B467;
    /// `+0x0C`: W, the pieces per step.
    pub const WIDTH: u32 = 0x482C_B45F;
    /// `+0x10`: N, the ramp steps.
    pub const RAMP_STEPS: u32 = 0x482C_B442;
    /// `+0x14`: the ramp piece instances / rotations.
    pub const RAMP_INSTANCES: u32 = 0x482C_B443;
    pub const RAMP_ROTATIONS: u32 = 0x482C_B444;
    /// `+0x20`: the ramp model depths.
    pub const RAMP_DEPTHS: u32 = 0x482C_B44D;
    /// `+0x2C`, `+0x30`: the deck piece.
    pub const DECK_INSTANCE: u32 = 0x482C_B445;
    pub const DECK_ROTATION: u32 = 0x482C_B446;
    /// `+0x34`: the pier spacing.
    ///
    /// [choice] §2.3's `+0x34` row says `0x482CB448`, which its `+0x40` row
    /// also claims for M. The property ids run in order around the piece
    /// lists (`B442` N, `B443`/`B444` ramp pieces, `B445`/`B446` deck,
    /// `B449`/`B44A` pier pieces, `B44C` pier depths, `B44D` ramp depths),
    /// so `B447` is the free id and this takes it for the spacing and
    /// `B448` for M; see question 3.
    pub const PIER_SPACING: u32 = 0x482C_B447;
    /// `+0x38`: the first-pier offset.
    pub const FIRST_PIER: u32 = 0x482C_B459;
    /// `+0x3C`: the end-pier fraction.
    pub const END_PIER_FRACTION: u32 = 0x482C_D465;
    /// `+0x40`: M, the steps per pier group.
    pub const PIER_STEPS: u32 = 0x482C_B448;
    /// `+0x44`: the pier piece instances / rotations.
    pub const PIER_INSTANCES: u32 = 0x482C_B449;
    pub const PIER_ROTATIONS: u32 = 0x482C_B44A;
    /// `+0x50`: the pier model depths.
    pub const PIER_DEPTHS: u32 = 0x482C_B44C;
    /// `+0x5C`: the deck height above water.
    pub const DECK_HEIGHT_WATER: u32 = 0x482C_B441;
    /// `+0x60`, `+0x6C`: the ramp pairs.
    pub const RAMP_PAIRS: u32 = 0x482C_B44B;
    /// `+0x80`, `+0x8C`: the pillar footprints.
    pub const RAMP_FOOTPRINT: u32 = 0x482C_B44E;
    pub const PIER_FOOTPRINT: u32 = 0x482C_B45E;
    /// `+0x98`, `+0x99`: the land and water flags.
    pub const LAND: u32 = 0x482C_B44F;
    pub const WATER: u32 = 0x482C_B450;
    /// `+0x9C`, `+0xA0`: the suspension parameters.
    pub const SUSPENSION_PYLON: u32 = 0x482C_B455;
    pub const SUSPENSION_END: u32 = 0x482C_B456;
    /// `+0xA4`: copied into each supported piece.
    pub const SUPPORT_VALUE: u32 = 0x482C_B457;
    /// `+0xA8`, `+0xAC`: the pier counts.
    pub const MIN_PIERS: u32 = 0x482C_B45A;
    pub const MAX_PIERS: u32 = 0x482C_B45B;
    /// `+0xB0`, `+0xB4`: the cable model ids.
    pub const CABLE_A: u32 = 0x482C_B45C;
    pub const CABLE_B: u32 = 0x482C_B45D;
    /// `+0xB8`–`+0xC4`: the cable floats.
    pub const CABLE_P0: u32 = 0x482C_B471;
    pub const CABLE_P1: u32 = 0x482C_D460;
    pub const CABLE_P2: u32 = 0x482C_D461;
    pub const CABLE_P3: u32 = 0x482C_D462;
    /// `+0xC8`, `+0xCC`: the clearance range.
    pub const MIN_CLEARANCE: u32 = 0x482C_D463;
    pub const MAX_CLEARANCE: u32 = 0x482C_D464;
    /// `+0xD0`: the model bound offset.
    pub const MODEL_BOUND_OFFSET: u32 = 0x482C_B468;
    /// `+0xD4`: added to every piece's model height.
    pub const MODEL_HEIGHT_OFFSET: u32 = 0x482C_B472;
    /// `+0xD8`–`+0x144`: the suspension tower data.
    pub const TOWER: [u32; 6] = [
        0x482C_B46B,
        0x482C_B46C,
        0x482C_B469,
        0x482C_B46A,
        0x482C_B46D,
        0x482C_B46E,
    ];
    /// `+0x148`, `+0x14C`: the menu bitmap key.
    pub const BITMAP_KEY: u32 = 0x482C_B46F;
    /// `+0x150`, `+0x154`: the cost (value 0 fixed, value 1 per step).
    pub const COST: u32 = 0x482C_B470;
    /// `+0x15C`: the localised name.
    pub const NAME: u32 = 0x482C_D466;
}

/// One INI entry (§2.2): the key text and the trimmed value text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IniEntry {
    pub key: String,
    pub value: String,
}

/// Where the bridge types and their exemplars come from (§2.1, §2.2 step 5).
pub trait BridgeSource {
    /// The INI entries of one section, in file order (§2.1).
    fn section_entries(&self, section: &str) -> Vec<IniEntry>;
    /// The properties of an exemplar (type, group, instance). `None` = the
    /// resource manager lookup failed and the record is removed (§2.2).
    fn exemplar_properties(&self, exemplar: (u32, u32, u32)) -> Option<Vec<(u32, PropValue)>>;
    /// The localised string `0x482CD466` names (§2.2 step 6).
    fn localized_string(&self, id: u32) -> Option<String>;
}

/// A source over raw INI bytes and a fixed property table: what the
/// conformance vectors and the unit tests drive.
#[derive(Clone, Debug, Default)]
pub struct TextSource {
    pub ini: Vec<u8>,
    /// The properties by exemplar, in the order the loader should see them.
    pub exemplars: Vec<((u32, u32, u32), Vec<(u32, PropValue)>)>,
    pub strings: Vec<(u32, String)>,
}

impl BridgeSource for TextSource {
    fn section_entries(&self, section: &str) -> Vec<IniEntry> {
        let secs = ini_sections(&self.ini);
        find_section(&secs, section)
            .into_iter()
            .map(|(k, v)| IniEntry {
                key: k.clone(),
                value: v.clone(),
            })
            .collect()
    }
    fn exemplar_properties(&self, exemplar: (u32, u32, u32)) -> Option<Vec<(u32, PropValue)>> {
        self.exemplars
            .iter()
            .find(|(e, _)| *e == exemplar)
            .map(|(_, p)| p.clone())
    }
    fn localized_string(&self, id: u32) -> Option<String> {
        self.strings
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, s)| s.clone())
    }
}

/// Parses one hex token the way §2.2 step 4 does (the exemplar's type, group
/// and instance). Leading `0x` is accepted; the C `strtoul` base is 16.
pub fn parse_hex(s: &str) -> u32 {
    let t = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    u32::from_str_radix(t, 16).unwrap_or(0)
}

/// Parses one unsigned decimal token (the INI key, §2.2 step 2).
pub fn parse_u32(s: &str) -> u32 {
    s.trim().parse::<u32>().unwrap_or(0)
}

/// Appends one default record to `out` and returns its index (§2.2 step 1).
fn push_default(out: &mut Vec<BridgeType>) -> usize {
    out.push(BridgeType::default());
    out.len() - 1
}

/// `ReadBridgeTypeInfo` (`0x00642220`, §2.2): one `KEY = TYPE, GROUP,
/// INSTANCE, NAME` entry into the network type's vector.
///
/// Returns whether the record survived. A malformed value returns `true` and
/// **leaves the default record in the vector** (quirk 1); a failed exemplar
/// lookup removes the record again.
pub fn read_bridge_type_info(
    out: &mut Vec<BridgeType>,
    entry: &IniEntry,
    default_width: u32,
    src: &dyn BridgeSource,
    host: &dyn BridgeHost,
) -> bool {
    let idx = push_default(out);
    // 2. The key as an unsigned number.
    out[idx].key = parse_u32(&entry.key);
    // 3. Split on ',' — exactly four tokens or leave the default in place.
    let tokens: Vec<&str> = entry.value.split(',').map(str::trim).collect();
    if tokens.len() != 4 {
        return true;
    }
    // 4. Tokens 0-2 are the exemplar; token 3 is the name at +0x15C.
    let exemplar = (parse_hex(tokens[0]), parse_hex(tokens[1]), parse_hex(tokens[2]));
    out[idx].name = tokens[3].to_string();
    // 5. The resource manager lookup; on failure the record is removed.
    let props = match src.exemplar_properties(exemplar) {
        Some(p) => p,
        None => {
            out.remove(idx);
            return false;
        }
    };
    // The host sees the same lookup (the plugin's filter path).
    let _ = host.exemplar_properties(exemplar);
    apply_properties(&mut out[idx], &props, default_width);
    // 6. The localised name.
    if let Some(id) = find_prop(&props, prop::NAME).and_then(|v| v.u32()) {
        if let Some(s) = src.localized_string(id).or_else(|| host.localized_string(id)) {
            out[idx].name = s;
        }
    }
    out[idx].finish_derived();
    true
}

fn find_prop<'a>(props: &'a [(u32, PropValue)], id: u32) -> Option<&'a PropValue> {
    props.iter().find(|(k, _)| *k == id).map(|(_, v)| v)
}

/// Reads the properties into a record (§2.3). `default_width` is the W
/// `LoadBridgeTypes` is called with (§2.1).
///
/// [choice] §2.3 lists `+0x0C`'s default as 1 while `LoadBridgeTypes` is
/// called with the network type's W (2 for highway and avenue). This reads
/// `+0x0C` from the property and falls back to the passed W, which is 1 for
/// every type the table does not give 2; see question 2.
fn apply_properties(t: &mut BridgeType, props: &[(u32, PropValue)], default_width: u32) {
    let u = |id| find_prop(props, id).and_then(PropValue::u32);
    let f = |id| find_prop(props, id).and_then(PropValue::f32);
    let b = |id| find_prop(props, id).and_then(PropValue::bool);

    if let Some(v) = u(prop::MIN_SPAN) {
        t.min_span = v;
    }
    if let Some(v) = u(prop::MAX_SPAN) {
        t.max_span = v;
    }
    t.width = u(prop::WIDTH).unwrap_or(default_width.max(1));
    let w = t.width.max(1) as usize;
    t.ramp_steps = u(prop::RAMP_STEPS).unwrap_or(0);
    if t.min_span == BridgeType::default_min_span(0) && t.ramp_steps > 0 {
        // The default is `2N + 2` with this record's N (§2.3).
        if find_prop(props, prop::MIN_SPAN).is_none() {
            t.min_span = BridgeType::default_min_span(t.ramp_steps);
        }
    }
    let n = t.ramp_steps as usize;
    let m = u(prop::PIER_STEPS).unwrap_or(0) as usize;
    t.pier_steps = m as u32;

    t.ramp_pieces = piece_list(props, prop::RAMP_INSTANCES, prop::RAMP_ROTATIONS, n * w);
    t.pier_pieces = piece_list(props, prop::PIER_INSTANCES, prop::PIER_ROTATIONS, m * w);
    t.ramp_depths = f32_list(props, prop::RAMP_DEPTHS, n);
    t.pier_depths = f32_list(props, prop::PIER_DEPTHS, m);
    t.deck_instance = u(prop::DECK_INSTANCE).unwrap_or(0);
    // [quirk 2] The rotation is stored as a byte.
    t.deck_rotation = u(prop::DECK_ROTATION).unwrap_or(0) as u8;
    t.pier_spacing = u(prop::PIER_SPACING).unwrap_or(0);
    t.first_pier = u(prop::FIRST_PIER).unwrap_or(t.pier_spacing);
    t.end_pier_fraction = f(prop::END_PIER_FRACTION).unwrap_or(0.0);
    t.deck_height_water = f(prop::DECK_HEIGHT_WATER).unwrap_or(0.0);

    // §2.4: the ramp pairs.
    let pairs_f = find_prop(props, prop::RAMP_PAIRS)
        .and_then(PropValue::f32s)
        .map(|v| v.to_vec())
        .unwrap_or_default();
    let pairs: Vec<(f32, f32)> = pairs_f
        .chunks_exact(2)
        .take(n)
        .map(|c| (c[0], c[1]))
        .collect();
    let (drops, offsets) = ramp_heights(&pairs);
    t.ramp_drops = drops;
    t.ramp_offsets = offsets;

    t.ramp_footprint = footprint_from(props, prop::RAMP_FOOTPRINT);
    t.pier_footprint = if find_prop(props, prop::PIER_FOOTPRINT).is_some() {
        footprint_from(props, prop::PIER_FOOTPRINT)
    } else {
        t.ramp_footprint
    };
    t.land = b(prop::LAND).unwrap_or(false);
    t.water = b(prop::WATER).unwrap_or(false);
    t.suspension_pylon = f(prop::SUSPENSION_PYLON).unwrap_or(0.0);
    t.suspension_end = f(prop::SUSPENSION_END).unwrap_or(0.0);
    t.support_value = f(prop::SUPPORT_VALUE).unwrap_or(0.0);
    t.min_piers = u(prop::MIN_PIERS).unwrap_or(0);
    t.max_piers = u(prop::MAX_PIERS).unwrap_or(super::types::DEFAULT_MAX_PIER_COUNT);
    t.cable_a = u(prop::CABLE_A).unwrap_or(0);
    t.cable_b = u(prop::CABLE_B).unwrap_or(0);
    for (i, id) in [
        prop::CABLE_P0,
        prop::CABLE_P1,
        prop::CABLE_P2,
        prop::CABLE_P3,
    ]
    .iter()
    .enumerate()
    {
        t.cable_params[i] = f(*id).unwrap_or(0.0);
    }
    t.min_clearance = f(prop::MIN_CLEARANCE).unwrap_or(super::types::DEFAULT_MIN_CLEARANCE);
    t.max_clearance = f(prop::MAX_CLEARANCE).unwrap_or(super::types::DEFAULT_MAX_CLEARANCE);
    t.model_bound_offset = f(prop::MODEL_BOUND_OFFSET).unwrap_or(0.0);
    t.model_height_offset = f(prop::MODEL_HEIGHT_OFFSET).unwrap_or(0.0);
    if let Some(bk) = find_prop(props, prop::BITMAP_KEY).and_then(PropValue::u32s) {
        t.bitmap_key = (bk.first().copied().unwrap_or(0), bk.get(1).copied().unwrap_or(0));
    }
    if let Some(c) = find_prop(props, prop::COST).and_then(PropValue::u32s) {
        t.cost_fixed = c.first().copied().unwrap_or(0);
        t.cost_per_step = c.get(1).copied().unwrap_or(0);
    }
    t.has_specialization = true;
}

fn piece_list(props: &[(u32, PropValue)], id_i: u32, id_r: u32, want: usize) -> Vec<PieceDef> {
    let inst = find_prop(props, id_i).and_then(PropValue::u32s).map(|v| v.to_vec());
    let rots = find_prop(props, id_r).and_then(PropValue::u32s).map(|v| v.to_vec());
    match (inst, rots) {
        (Some(i), Some(r)) => (0..want)
            .map(|k| PieceDef {
                instance: i.get(k).copied().unwrap_or(0),
                // [quirk 2] The rotation count is truncated to a byte
                // (`0x00642567`).
                rotation: r.get(k).copied().unwrap_or(0) as u8,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn f32_list(props: &[(u32, PropValue)], id: u32, want: usize) -> Vec<f32> {
    let v = find_prop(props, id).and_then(PropValue::f32s).map(|v| v.to_vec());
    match v {
        Some(v) => (0..want).map(|k| v.get(k).copied().unwrap_or(-1.0)).collect(),
        None => vec![-1.0; want],
    }
}

/// §2.3 `+0x80`: four (x, z) points, then the same rotated by 90°, 180° and
/// 270°.
fn footprint_from(props: &[(u32, PropValue)], id: u32) -> [[Footprint; 4]; 4] {
    let v = find_prop(props, id).and_then(PropValue::f32s).map(|v| v.to_vec());
    let mut base = [Footprint::default(); 4];
    if let Some(v) = v {
        for (k, p) in base.iter_mut().enumerate() {
            p.x = v.get(2 * k).copied().unwrap_or(0.0);
            p.z = v.get(2 * k + 1).copied().unwrap_or(0.0);
        }
    }
    footprint_variants(&base)
}

/// `LoadBridgeTypes` (`0x006437A0`, §2.1): the INI section's entries in file
/// order. `default_width` is the W of §2.1's table.
pub fn load_bridge_types(
    src: &dyn BridgeSource,
    section: &str,
    default_width: u32,
    host: &dyn BridgeHost,
) -> Vec<BridgeType> {
    let mut out = Vec::new();
    for e in src.section_entries(section) {
        read_bridge_type_info(&mut out, &e, default_width, src, host);
    }
    out
}

/// §2.5: the water (`+0x33C`) and land (`+0x348`) lists, in vector order.
pub fn bridge_type_lists(types: &[BridgeType]) -> (Vec<u32>, Vec<u32>) {
    let mut water = Vec::new();
    let mut land = Vec::new();
    for (i, t) in types.iter().enumerate() {
        if t.water {
            water.push(i as u32);
        }
        if t.land {
            land.push(i as u32);
        }
    }
    (water, land)
}

/// §2.5 `Init` (`0x00626B24`–`0x00626B5E`) sets the tool's land-bridge flag
/// `+0x354` by testing the **address** of each type's `+0x98` field, not the
/// byte stored there. The address is never null, so the flag is set exactly
/// when the network type has at least one bridge type (quirk 3).
///
/// `type_count` is the number of bridge types of the network type.
pub fn land_bridge_flag(type_count: usize) -> bool {
    type_count > 0
}

/// §2.1: the INI section of each network type, the specialisation instance
/// (`0x1000 + type`) and the W. Power line (5) has no section.
pub fn section_of(type_id: u32) -> Option<(&'static str, u32, u32)> {
    let (s, w) = match type_id {
        0 => ("RoadBridgeExemplars", 1),
        1 => ("RailBridgeExemplars", 1),
        2 => ("HighwayBridgeExemplars", 2),
        3 => ("StreetBridgeExemplars", 1),
        4 => ("PipeBridgeExemplars", 1),
        5 => return None,
        6 => ("AvenueBridgeExemplars", 2),
        7 => ("SubwayBridgeExemplars", 1),
        8 => ("LightRailBridgeExemplars", 1),
        9 => ("MonorailBridgeExemplars", 1),
        10 => ("OneWayRoadBridgeExemplars", 1),
        11 => ("DirtRoadBridgeExemplars", 1),
        12 => ("GroundHighwayBridgeExemplars", 2),
        _ => return None,
    };
    Some((s, 0x1000 + type_id, w))
}

/// The INI resource (§2.1): type 0, group `0x8A5971C5`, instance
/// `0x8A5993B9`.
pub const NETWORK_INI_RESOURCE: (u32, u32, u32) = (0, 0x8A59_71C5, 0x8A59_93B9);

/// The shipped maximum span every bridge type has, because the exemplars
/// never set `0x482CB467` (§2.3) *[compat]*.
pub const SHIPPED_MAX_SPAN: u32 = DEFAULT_MAX_SPAN;
