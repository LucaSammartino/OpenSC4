// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The effect directory resource (effects.md §3, §8.1, §9.1): the brush,
//! attractor and game ("scrubber") sections, the visual effect descriptions
//! with their `0x6C`-byte component records, and the name map.
//!
//! The spec gives the order of every section but the layout only of the
//! sections above. The render sections between them (particles, decals and
//! two more before the brushes; sequences, sounds, cameras and the dynamic
//! particles before the visual effects) are skipped by **locating** the next
//! known section [choice, EF6]:
//!
//! - the brush section is the first offset from which the brush, attractor
//!   and game sections all read plausibly (at least one brush; brush
//!   version 0, a non-zero brush ID, a one-shot byte of 0 or 1, a level of
//!   0-4; attractor version 0, a printable name and a value of 0 or 1; game
//!   version 0 or 1, flags within 7 bits, at least one game record; floats
//!   zero or normal), tried first at
//!   the shipped offset of §3 (records at `0x98CA6`, so the count at
//!   `0x98CA2`);
//! - the visual effect section is the first offset after the game section
//!   from which the section (version 1 or 2, flags within 9 bits,
//!   priorities up to 255, printable strings, floats zero or normal) and the name
//!   map behind it read plausibly, the map ending with the name `"end"` and
//!   every other index naming a description.
//!
//! [`EffectDirectory::parse_at`] takes both offsets when a host knows them.
//! Everything after the name map's `"end"` entry is ignored (it is not in
//! the spec). Strings are a `u32` length and the characters, read as UTF-8
//! (lossy) [choice].

use std::collections::BTreeMap;
use std::fmt;

/// The shipped offset of the brush section's count (§3: the records start at
/// `0x98CA6`).
pub const SHIPPED_BRUSH_SECTION: usize = 0x98CA2;

/// Size of a brush record in the file (§8.1).
pub const BRUSH_RECORD_BYTES: usize = 39;
/// Size of a version-1 game record in the file (§9.1).
pub const GAME_RECORD_BYTES: usize = 70;
/// The size of the placement block of a component record (§3).
pub const PLACEMENT_BYTES: usize = 56;

/// Plausibility limits of the section locator [choice, EF6].
const MAX_COUNT: u32 = 1 << 16;
const MAX_STRING: u32 = 4096;
const MAX_COMPONENTS: u32 = 4096;
const MAX_BINDINGS: u32 = 1024;
const MAX_COMPONENT_TYPE: u8 = 0x10;

/// A parse failure: what and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub what: String,
    pub offset: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at 0x{:X}", self.what, self.offset)
    }
}

impl std::error::Error for ParseError {}

/// `cSC4BrushDescription` (§8.1), as stored.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushRecord {
    pub version: u16,
    /// `+0x0C`: the terrain brush (terrain.md §9.1).
    pub brush_id: u32,
    /// `+0x10`: applications per effect-second, or the one-shot count.
    pub rate: f32,
    /// `+0x14`: the stroke duration in effect-seconds; 0 = no limit.
    pub length: f32,
    /// `+0x18`.
    pub one_shot: bool,
    /// `+0x1C`: `tBrushParameters.level`.
    pub level: u32,
    /// `+0x20`, `+0x24`.
    pub strength: (f32, f32),
    /// `+0x28`, `+0x2C`: the radius-scale range.
    pub radius: (f32, f32),
    /// `+0x30`.
    pub target_height: f32,
}

/// An attractor description (§3 item 4): the automata layer's, named only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttractorRecord {
    pub version: u16,
    pub name: String,
    pub value: u8,
}

/// Game record flags (§9.1, `+0x0C`).
pub mod game_flag {
    pub const NO_NETWORKS: u32 = 1;
    pub const NO_FLORA: u32 = 2;
    pub const DEZONE: u32 = 4;
    pub const SINGLE: u32 = 8;
    pub const PAUSE_SIM: u32 = 0x10;
    pub const PAUSE_SIM_HIDDEN: u32 = 0x20;
    pub const PAUSE_CLOCK: u32 = 0x40;
    /// The binary record keeps 7 bits.
    pub const MASK: u32 = 0x7F;
}

/// `cSC4ScrubberDescription` (§9.1), as stored (binary version 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameRecord {
    pub version: u16,
    /// `+0x0C` ([`game_flag`]), masked to 7 bits.
    pub flags: u32,
    /// `+0x10` (not described).
    pub field_10: u32,
    /// `+0x14`: cells.
    pub demolish_radius: i32,
    /// `+0x18`: demolition.md §4.9.
    pub demolish_flags: u32,
    /// `+0x1C`, `+0x20`: metres; 0 = no limit.
    pub min_demolish_size: f32,
    pub max_demolish_size: f32,
    /// `+0x24`.
    pub burn_radius: i32,
    /// `+0x30`, `+0x34`.
    pub message_type: u32,
    pub message_data: u32,
    /// `+0x38`..`+0x48`: the effect-map shape (render).
    pub map_kind: u32,
    pub map_3c: f32,
    pub map_40: [u8; 8],
    pub map_48: f32,
    /// `+0x4C`: effect-seconds; 0 = none.
    pub duration: f32,
    /// `+0x28` (version > 0).
    pub toxic: i32,
    /// `+0x2C` (version > 0).
    pub extinguish: i32,
}

impl Default for GameRecord {
    /// The constructor's defaults (§9.1): demolish flags 1, `+0x3C` 16.0,
    /// version 1, everything else 0.
    fn default() -> Self {
        GameRecord {
            version: 1,
            flags: 0,
            field_10: 0,
            demolish_radius: 0,
            demolish_flags: 1,
            min_demolish_size: 0.0,
            max_demolish_size: 0.0,
            burn_radius: 0,
            message_type: 0,
            message_data: 0,
            map_kind: 0,
            map_3c: 16.0,
            map_40: [0; 8],
            map_48: 0.0,
            duration: 0.0,
            toxic: 0,
            extinguish: 0,
        }
    }
}

/// A component record (§3, `0x6C` bytes in memory).
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentRecord {
    /// The child effect's name (meta), the asset name (particles, decals),
    /// else empty.
    pub name: String,
    /// `+0x0C` ([`super::component_type`]).
    pub kind: u8,
    /// `+0x10`, masked to 2 bits: bit 0 = ignored by the finish test (§7).
    pub flags: u32,
    /// `+0x14`: all 56 bytes of the local transform, kept as stored.
    /// Decoding and native numeric order remain open in EF19. The manager's
    /// identity-placement limitation does not alter these resource bytes.
    pub placement: [u8; PLACEMENT_BYTES],
    /// `+0x4C`, `+0x4D`: the level-of-detail range `[min, max)`.
    pub lod_min: u8,
    pub lod_max: u8,
    /// `+0x4E`, `+0x50` (render).
    pub copies: u16,
    pub spacing: u16,
    /// `+0x54`..`+0x60` (render).
    pub render: [f32; 4],
    /// `+0x64`: 0 = always; `g > 0` = one member of group `g` (§5.3).
    pub select_group: u16,
    /// `+0x66`.
    pub select_weight: u16,
    /// `+0x68`: the index into the section of the component's type.
    pub index: u32,
}

impl ComponentRecord {
    /// A component of `kind` with `index`, range `[1, 6)`, an identity
    /// placement and no select group (the shipped defaults of §5.2).
    pub fn new(kind: u8, index: u32) -> ComponentRecord {
        ComponentRecord {
            name: String::new(),
            kind,
            flags: 0,
            placement: IDENTITY_PLACEMENT,
            lod_min: 1,
            lod_max: 6,
            copies: 1,
            spacing: 16,
            render: [1.0; 4],
            select_group: 0,
            select_weight: 0,
            index,
        }
    }

    /// A meta component naming `child`.
    pub fn meta(child: &str) -> ComponentRecord {
        ComponentRecord {
            name: child.to_owned(),
            ..ComponentRecord::new(super::component_type::META, 0)
        }
    }

    /// Whether the level of detail `lod` is in `[lod_min, lod_max)` (§5.2).
    pub fn in_range(&self, lod: u32) -> bool {
        u32::from(self.lod_min) <= lod && lod < u32::from(self.lod_max)
    }
}

/// The placement bytes of an identity transform as the shipped components
/// store it (nine floats of a unit matrix, then zeros and a 1.0). Only used
/// to build records; the core does not read the placement (EF7).
pub const IDENTITY_PLACEMENT: [u8; PLACEMENT_BYTES] = {
    let mut p = [0u8; PLACEMENT_BYTES];
    // 1.0f32 little-endian (`to_le_bytes` is not const under the MSRV).
    let one = [0x00u8, 0x00, 0x80, 0x3F];
    let mut k = 0;
    while k < 4 {
        p[k] = one[k];
        p[16 + k] = one[k];
        p[32 + k] = one[k];
        p[48 + k] = one[k];
        k += 1;
    }
    p
};

/// A parameter binding of a visual effect (render): `{u32 & 0xF, string,
/// f32, u32}`.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamBinding {
    pub index: u32,
    pub name: String,
    pub value: f32,
    pub extra: u32,
}

/// A visual effect description (§3, `0x38` bytes in memory).
#[derive(Clone, Debug, PartialEq)]
pub struct EffectDescription {
    /// `+0x00`, masked to 9 bits ([`super::effect_flag`]).
    pub flags: u32,
    /// `+0x04`: 0 is never chosen.
    pub priority: u32,
    pub components: Vec<ComponentRecord>,
    pub bindings: Vec<ParamBinding>,
    /// `+0x2C`.
    pub text: String,
    /// `+0x20`..`+0x28`, only when the section version is above 1.
    pub extra: [u32; 3],
}

impl EffectDescription {
    /// A description with `priority` and `components`, flags 0.
    pub fn new(priority: u32, components: Vec<ComponentRecord>) -> EffectDescription {
        EffectDescription {
            flags: 0,
            priority,
            components,
            bindings: Vec::new(),
            text: String::new(),
            extra: [0, 0xCCCC_CCCC, 0xCCCC_CCCC],
        }
    }
}

/// Where the sections were found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SectionOffsets {
    pub brushes: usize,
    pub attractors: usize,
    pub games: usize,
    pub games_end: usize,
    pub effects: usize,
    pub names: usize,
    /// Just after the `"end"` entry.
    pub names_end: usize,
}

/// The effect directory (§3): the sections the simulation reads, and the
/// name map.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EffectDirectory {
    /// The `u16` root version (3 or 4).
    pub root_version: u16,
    pub brushes: Vec<BrushRecord>,
    pub attractors: Vec<AttractorRecord>,
    pub games: Vec<GameRecord>,
    /// The visual effect section's version (shipped 2).
    pub effects_version: u16,
    pub effects: Vec<EffectDescription>,
    /// The name map in file order: (name, description index). The `"end"`
    /// entry is not kept.
    pub names: Vec<(String, u32)>,
    /// The index of the `"end"` entry (shipped `0xFFFFFFFF`), or `None` when
    /// the map ended with a length of `0xFFFFFFFF`.
    pub end_index: Option<u32>,
    pub offsets: SectionOffsets,
    /// Lower-cased name → indices, in name-map order.
    by_name: BTreeMap<String, Vec<u32>>,
}

// ---- reading -------------------------------------------------------------------------------------

struct Cursor<'a> {
    d: &'a [u8],
    p: usize,
    /// The locator's plausibility tests (EF6): printable non-empty
    /// strings where names are expected, finite floats, small enums.
    strict: bool,
}

type R<T> = Result<T, ParseError>;

impl<'a> Cursor<'a> {
    fn new(d: &'a [u8], p: usize) -> Self {
        Cursor { d, p, strict: false }
    }
    fn strict(d: &'a [u8], p: usize, strict: bool) -> Self {
        Cursor { d, p, strict }
    }
    /// In strict mode, fails unless `ok`.
    fn check(&self, ok: bool, what: &str) -> R<()> {
        if self.strict && !ok {
            return self.err(&format!("implausible {what}"));
        }
        Ok(())
    }
    /// A float, zero or normal (finite, not subnormal) in strict mode.
    fn real(&mut self, what: &str) -> R<f32> {
        let v = self.f32(what)?;
        self.check(v == 0.0 || v.is_normal(), what)?;
        Ok(v)
    }
    /// A string that must be printable ASCII in strict mode (empty allowed
    /// unless `name`).
    fn text(&mut self, what: &str, name: bool) -> R<String> {
        let s = self.string(what)?;
        self.check(
            (!name || !s.is_empty()) && s.bytes().all(|b| (0x20..0x7F).contains(&b)),
            what,
        )?;
        Ok(s)
    }
    fn err<T>(&self, what: &str) -> R<T> {
        Err(ParseError {
            what: what.to_owned(),
            offset: self.p,
        })
    }
    fn take(&mut self, n: usize, what: &str) -> R<&'a [u8]> {
        match self.p.checked_add(n) {
            Some(e) if e <= self.d.len() => {
                let s = &self.d[self.p..e];
                self.p = e;
                Ok(s)
            }
            _ => self.err(&format!("truncated {what}")),
        }
    }
    fn u8(&mut self, what: &str) -> R<u8> {
        Ok(self.take(1, what)?[0])
    }
    fn u16(&mut self, what: &str) -> R<u16> {
        let b = self.take(2, what)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&mut self, what: &str) -> R<u32> {
        let b = self.take(4, what)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn i32(&mut self, what: &str) -> R<i32> {
        Ok(self.u32(what)? as i32)
    }
    fn f32(&mut self, what: &str) -> R<f32> {
        Ok(f32::from_bits(self.u32(what)?))
    }
    fn count(&mut self, what: &str, max: u32) -> R<u32> {
        let at = self.p;
        let n = self.u32(what)?;
        if n > max {
            return Err(ParseError {
                what: format!("implausible {what} {n}"),
                offset: at,
            });
        }
        Ok(n)
    }
    fn string(&mut self, what: &str) -> R<String> {
        let n = self.count(what, MAX_STRING)?;
        let b = self.take(n as usize, what)?;
        Ok(String::from_utf8_lossy(b).into_owned())
    }
}

fn read_brush(c: &mut Cursor) -> R<BrushRecord> {
    let version = c.u16("brush version")?;
    c.check(version == 0, "brush version")?;
    let brush_id = c.u32("brush ID")?;
    c.check(brush_id != 0, "brush ID")?;
    let rate = c.real("brush rate")?;
    let length = c.real("brush length")?;
    let one_shot = c.u8("brush one-shot")?;
    c.check(one_shot <= 1, "brush one-shot")?;
    let level = c.u32("brush level")?;
    c.check(level <= 4, "brush level")?;
    Ok(BrushRecord {
        version,
        brush_id,
        rate,
        length,
        one_shot: one_shot != 0,
        level,
        strength: (c.real("brush strength")?, c.real("brush strength")?),
        radius: (c.real("brush radius")?, c.real("brush radius")?),
        target_height: c.real("brush target")?,
    })
}

fn read_game(c: &mut Cursor) -> R<GameRecord> {
    let version = c.u16("game version")?;
    c.check(version <= 1, "game version")?;
    let raw_flags = c.u32("game flags")?;
    c.check(raw_flags <= game_flag::MASK, "game flags")?;
    let flags = raw_flags & game_flag::MASK;
    let field_10 = c.u32("game +0x10")?;
    let demolish_radius = c.i32("game demolish radius")?;
    let demolish_flags = c.u32("game demolish flags")?;
    let min_demolish_size = c.real("game min size")?;
    let max_demolish_size = c.real("game max size")?;
    let burn_radius = c.i32("game burn")?;
    let message_type = c.u32("game message")?;
    let message_data = c.u32("game message data")?;
    let map_kind = c.u32("game map")?;
    let map_3c = c.real("game map +0x3C")?;
    let mut map_40 = [0u8; 8];
    map_40.copy_from_slice(c.take(8, "game map +0x40")?);
    let map_48 = c.real("game map +0x48")?;
    let duration = c.real("game duration")?;
    let (toxic, extinguish) = if version > 0 {
        (c.i32("game toxic")?, c.i32("game extinguish")?)
    } else {
        (0, 0)
    };
    Ok(GameRecord {
        version,
        flags,
        field_10,
        demolish_radius,
        demolish_flags,
        min_demolish_size,
        max_demolish_size,
        burn_radius,
        message_type,
        message_data,
        map_kind,
        map_3c,
        map_40,
        map_48,
        duration,
        toxic,
        extinguish,
    })
}

fn read_component(c: &mut Cursor) -> R<ComponentRecord> {
    let name = c.text("component name", false)?;
    let at = c.p;
    let kind = c.u8("component type")?;
    if kind > MAX_COMPONENT_TYPE {
        return Err(ParseError {
            what: format!("component type {kind}"),
            offset: at,
        });
    }
    let flags = c.u32("component flags")? & 3;
    let mut placement = [0u8; PLACEMENT_BYTES];
    placement.copy_from_slice(c.take(PLACEMENT_BYTES, "component placement")?);
    let lod_min = c.u8("component lod")?;
    let lod_max = c.u8("component lod")?;
    let copies = c.u16("component copies")?;
    let spacing = c.u16("component spacing")?;
    let render = [
        c.real("component render")?,
        c.real("component render")?,
        c.real("component render")?,
        c.real("component render")?,
    ];
    Ok(ComponentRecord {
        name,
        kind,
        flags,
        placement,
        lod_min,
        lod_max,
        copies,
        spacing,
        render,
        select_group: c.u16("component group")?,
        select_weight: c.u16("component weight")?,
        index: c.u32("component index")?,
    })
}

/// The visual effect description's fields in file order (§3: flags,
/// priority, components, bindings, the string, then the three `u32` above
/// version 1).
fn read_effect(c: &mut Cursor, section_version: u16) -> R<EffectDescription> {
    let raw_flags = c.u32("effect flags")?;
    c.check(raw_flags <= 0x1FF, "effect flags")?;
    let flags = raw_flags & 0x1FF;
    let priority = c.u32("effect priority")?;
    c.check(priority <= 0xFF, "effect priority")?;
    let n = c.count("component count", MAX_COMPONENTS)?;
    let mut components = Vec::with_capacity(n.min(256) as usize);
    for _ in 0..n {
        components.push(read_component(c)?);
    }
    let nb = c.count("binding count", MAX_BINDINGS)?;
    let mut bindings = Vec::with_capacity(nb.min(64) as usize);
    for _ in 0..nb {
        bindings.push(ParamBinding {
            index: c.u32("binding index")? & 0xF,
            name: c.text("binding name", false)?,
            value: c.real("binding value")?,
            extra: c.u32("binding extra")?,
        });
    }
    let text = c.text("effect string", false)?;
    let extra = if section_version > 1 {
        [c.u32("effect extra")?, c.u32("effect extra")?, c.u32("effect extra")?]
    } else {
        [0; 3]
    };
    Ok(EffectDescription {
        flags,
        priority,
        components,
        bindings,
        text,
        extra,
    })
}

struct TableSections {
    brushes: Vec<BrushRecord>,
    attractors: Vec<AttractorRecord>,
    games: Vec<GameRecord>,
    offsets: SectionOffsets,
}

/// The brush, attractor and game sections from `at` (§3 items 3–5).
/// `strict` adds the locator's plausibility tests.
fn read_tables(d: &[u8], at: usize, strict: bool) -> R<TableSections> {
    let mut c = Cursor::strict(d, at, strict);
    let mut offsets = SectionOffsets {
        brushes: at,
        ..SectionOffsets::default()
    };
    let n = c.count("brush count", MAX_COUNT)?;
    if strict && n == 0 {
        return c.err("no brushes");
    }
    let mut brushes = Vec::with_capacity(n.min(256) as usize);
    for _ in 0..n {
        brushes.push(read_brush(&mut c)?);
    }
    offsets.attractors = c.p;
    let n = c.count("attractor count", MAX_COUNT)?;
    let mut attractors = Vec::with_capacity(n.min(256) as usize);
    for _ in 0..n {
        let version = c.u16("attractor version")?;
        c.check(version == 0, "attractor version")?;
        let name = c.text("attractor name", true)?;
        let value = c.u8("attractor value")?;
        c.check(value <= 1, "attractor value")?;
        attractors.push(AttractorRecord { version, name, value });
    }
    offsets.games = c.p;
    let n = c.count("game count", MAX_COUNT)?;
    c.check(n > 0, "game count")?;
    let mut games = Vec::with_capacity(n.min(256) as usize);
    for _ in 0..n {
        games.push(read_game(&mut c)?);
    }
    offsets.games_end = c.p;
    Ok(TableSections {
        brushes,
        attractors,
        games,
        offsets,
    })
}

struct EffectSections {
    version: u16,
    effects: Vec<EffectDescription>,
    names: Vec<(String, u32)>,
    end_index: Option<u32>,
    names_at: usize,
    names_end: usize,
}

/// The visual effect section and the name map from `at` (§3 items 7, 8).
fn read_effects(d: &[u8], at: usize, strict: bool) -> R<EffectSections> {
    let mut c = Cursor::strict(d, at, strict);
    let version = c.u16("effect section version")?;
    c.check((1..=2).contains(&version), "effect section version")?;
    let n = c.count("effect count", MAX_COUNT)?;
    let mut effects = Vec::with_capacity(n.min(256) as usize);
    for _ in 0..n {
        effects.push(read_effect(&mut c, version)?);
    }
    let names_at = c.p;
    let mut names = Vec::new();
    let end_index = loop {
        let at = c.p;
        let len = c.u32("name length")?;
        if len == 0xFFFF_FFFF {
            break None;
        }
        if len > MAX_STRING {
            return Err(ParseError {
                what: format!("implausible name length {len}"),
                offset: at,
            });
        }
        let name = String::from_utf8_lossy(c.take(len as usize, "name")?).into_owned();
        c.check(
            !name.is_empty() && name.bytes().all(|b| (0x20..0x7F).contains(&b)),
            "name",
        )?;
        let index = c.u32("name index")?;
        if name == "end" {
            break Some(index);
        }
        if index >= n {
            return Err(ParseError {
                what: format!("name '{name}' indexes description {index} of {n}"),
                offset: at,
            });
        }
        names.push((name, index));
    };
    Ok(EffectSections {
        version,
        effects,
        names,
        end_index,
        names_at,
        names_end: c.p,
    })
}

impl EffectDirectory {
    /// Parses a resource, locating the brush and visual effect sections
    /// (module docs, EF6).
    pub fn parse(data: &[u8]) -> R<EffectDirectory> {
        let root_version = Cursor::new(data, 0).u16("root version")?;
        let tables = Self::locate_tables(data)?;
        let effects_at = Self::locate_effects(data, tables.offsets.games_end)?;
        let eff = read_effects(data, effects_at, false)?;
        Ok(Self::assemble(root_version, tables, effects_at, eff))
    }

    /// Parses with known section offsets: the brush section's count at
    /// `brushes`, the visual effect section's version at `effects`.
    pub fn parse_at(data: &[u8], brushes: usize, effects: usize) -> R<EffectDirectory> {
        let root_version = Cursor::new(data, 0).u16("root version")?;
        let tables = read_tables(data, brushes, false)?;
        let eff = read_effects(data, effects, false)?;
        Ok(Self::assemble(root_version, tables, effects, eff))
    }

    fn locate_tables(data: &[u8]) -> R<TableSections> {
        if let Ok(t) = read_tables(data, SHIPPED_BRUSH_SECTION, true) {
            return Ok(t);
        }
        (2..data.len().saturating_sub(3))
            .find_map(|p| read_tables(data, p, true).ok())
            .ok_or(ParseError {
                what: "no brush section".into(),
                offset: 2,
            })
    }

    fn locate_effects(data: &[u8], from: usize) -> R<usize> {
        (from..data.len().saturating_sub(5))
            .find(|&p| {
                // Cheap pre-test: a section version of 0-2 (§3: shipped 2).
                (1..=2).contains(&u16::from_le_bytes([data[p], data[p + 1]])) && read_effects(data, p, true).is_ok()
            })
            .ok_or(ParseError {
                what: "no visual effect section".into(),
                offset: from,
            })
    }

    fn assemble(root_version: u16, t: TableSections, effects_at: usize, e: EffectSections) -> EffectDirectory {
        let mut offsets = t.offsets;
        offsets.effects = effects_at;
        offsets.names = e.names_at;
        offsets.names_end = e.names_end;
        let mut d = EffectDirectory {
            root_version,
            brushes: t.brushes,
            attractors: t.attractors,
            games: t.games,
            effects_version: e.version,
            effects: e.effects,
            names: e.names,
            end_index: e.end_index,
            offsets,
            by_name: BTreeMap::new(),
        };
        d.reindex();
        d
    }

    /// A directory from parts (tests, hosts with their own data). The name
    /// map is `names` in order.
    pub fn from_parts(
        brushes: Vec<BrushRecord>,
        games: Vec<GameRecord>,
        effects: Vec<EffectDescription>,
        names: Vec<(String, u32)>,
    ) -> EffectDirectory {
        let mut d = EffectDirectory {
            root_version: 4,
            brushes,
            games,
            effects_version: 2,
            effects,
            names,
            end_index: Some(0xFFFF_FFFF),
            ..EffectDirectory::default()
        };
        d.reindex();
        d
    }

    /// Rebuilds the name index after `names` changed.
    pub fn reindex(&mut self) {
        self.by_name.clear();
        for (n, i) in &self.names {
            self.by_name.entry(n.to_ascii_lowercase()).or_default().push(*i);
        }
    }

    /// The descriptions named `name` (case-insensitive, §4.1), in name-map
    /// order; `None` for an unknown name.
    pub fn indices_of(&self, name: &str) -> Option<&[u32]> {
        self.by_name.get(&name.to_ascii_lowercase()).map(Vec::as_slice)
    }

    /// `HasVisualEffect` (slot 5).
    pub fn has(&self, name: &str) -> bool {
        self.indices_of(name).is_some()
    }

    /// `FindEffectDesc` (§4.1 step 3): `None` for an unknown name; else the
    /// description with the highest priority `p`, `0 < p ≤ max_priority`,
    /// the first in name-map order on a tie, or −1 when none qualifies.
    pub fn find(&self, name: &str, max_priority: u32) -> Option<i32> {
        let ix = self.indices_of(name)?;
        let mut best: Option<(u32, u32)> = None;
        for &i in ix {
            let Some(p) = self.effects.get(i as usize).map(|e| e.priority) else {
                continue;
            };
            if p == 0 || p > max_priority {
                continue;
            }
            if best.map_or(true, |(bp, _)| p > bp) {
                best = Some((p, i));
            }
        }
        Some(best.map_or(-1, |(_, i)| i as i32))
    }

    /// The first name the map gives `index` (diagnostics).
    pub fn name_of(&self, index: u32) -> Option<&str> {
        self.names.iter().find(|(_, i)| *i == index).map(|(n, _)| n.as_str())
    }

    /// Writes the sections this module reads, as a resource whose render
    /// sections are empty: the root version, then the brush, attractor and
    /// game sections, the visual effect section and the name map ending in
    /// `"end"` (index `end_index`, default `0xFFFFFFFF`). [`Self::parse`]
    /// reads it back (with at least one brush) [choice: test fixtures].
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Vec::new();
        w.extend_from_slice(&self.root_version.to_le_bytes());
        put_u32(&mut w, self.brushes.len() as u32);
        for b in &self.brushes {
            write_brush(&mut w, b);
        }
        put_u32(&mut w, self.attractors.len() as u32);
        for a in &self.attractors {
            w.extend_from_slice(&a.version.to_le_bytes());
            put_str(&mut w, &a.name);
            w.push(a.value);
        }
        put_u32(&mut w, self.games.len() as u32);
        for g in &self.games {
            write_game(&mut w, g);
        }
        w.extend_from_slice(&self.effects_version.to_le_bytes());
        put_u32(&mut w, self.effects.len() as u32);
        for e in &self.effects {
            write_effect(&mut w, e, self.effects_version);
        }
        for (n, i) in &self.names {
            put_str(&mut w, n);
            put_u32(&mut w, *i);
        }
        match self.end_index {
            Some(i) => {
                put_str(&mut w, "end");
                put_u32(&mut w, i);
            }
            None => put_u32(&mut w, 0xFFFF_FFFF),
        }
        w
    }
}

fn put_u32(w: &mut Vec<u8>, v: u32) {
    w.extend_from_slice(&v.to_le_bytes());
}

fn put_f32(w: &mut Vec<u8>, v: f32) {
    w.extend_from_slice(&v.to_le_bytes());
}

fn put_str(w: &mut Vec<u8>, s: &str) {
    put_u32(w, s.len() as u32);
    w.extend_from_slice(s.as_bytes());
}

/// The binary brush record (§8.1, 39 bytes).
pub fn write_brush(w: &mut Vec<u8>, b: &BrushRecord) {
    w.extend_from_slice(&b.version.to_le_bytes());
    put_u32(w, b.brush_id);
    put_f32(w, b.rate);
    put_f32(w, b.length);
    w.push(u8::from(b.one_shot));
    put_u32(w, b.level);
    put_f32(w, b.strength.0);
    put_f32(w, b.strength.1);
    put_f32(w, b.radius.0);
    put_f32(w, b.radius.1);
    put_f32(w, b.target_height);
}

/// The binary game record (§9.1; 70 bytes at version 1).
pub fn write_game(w: &mut Vec<u8>, g: &GameRecord) {
    w.extend_from_slice(&g.version.to_le_bytes());
    put_u32(w, g.flags & game_flag::MASK);
    put_u32(w, g.field_10);
    put_u32(w, g.demolish_radius as u32);
    put_u32(w, g.demolish_flags);
    put_f32(w, g.min_demolish_size);
    put_f32(w, g.max_demolish_size);
    put_u32(w, g.burn_radius as u32);
    put_u32(w, g.message_type);
    put_u32(w, g.message_data);
    put_u32(w, g.map_kind);
    put_f32(w, g.map_3c);
    w.extend_from_slice(&g.map_40);
    put_f32(w, g.map_48);
    put_f32(w, g.duration);
    if g.version > 0 {
        put_u32(w, g.toxic as u32);
        put_u32(w, g.extinguish as u32);
    }
}

fn write_effect(w: &mut Vec<u8>, e: &EffectDescription, section_version: u16) {
    put_u32(w, e.flags & 0x1FF);
    put_u32(w, e.priority);
    put_u32(w, e.components.len() as u32);
    for c in &e.components {
        put_str(w, &c.name);
        w.push(c.kind);
        put_u32(w, c.flags & 3);
        w.extend_from_slice(&c.placement);
        w.push(c.lod_min);
        w.push(c.lod_max);
        w.extend_from_slice(&c.copies.to_le_bytes());
        w.extend_from_slice(&c.spacing.to_le_bytes());
        for f in c.render {
            put_f32(w, f);
        }
        w.extend_from_slice(&c.select_group.to_le_bytes());
        w.extend_from_slice(&c.select_weight.to_le_bytes());
        put_u32(w, c.index);
    }
    put_u32(w, e.bindings.len() as u32);
    for b in &e.bindings {
        put_u32(w, b.index & 0xF);
        put_str(w, &b.name);
        put_f32(w, b.value);
        put_u32(w, b.extra);
    }
    put_str(w, &e.text);
    if section_version > 1 {
        for v in e.extra {
            put_u32(w, v);
        }
    }
}

#[cfg(test)]
#[path = "directory_tests.rs"]
mod tests;
