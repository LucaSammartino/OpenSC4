// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The highway intersection rules (`nSC4Networks::cIntRule`, §10): the file
//! (instance `0x10000000`), the check types, the ASCII layouts, the
//! transforms, `IsWithinBounds`, `Test`, `DoAutoComplete`, `GetPrimaryType`
//! and `IdentifyHighwayIntersections`.
//!
//! Not here: the tile grid `+0xB8` and its map `+0xD4`, `AutoTileBase` and
//! `AutoPathBase` beyond storing them, and the constraint layout's consumer
//! (NR2); `InsertIsolatedHighwayIntersection` (§10.7), which is the
//! intersection tool's pipeline (geometry and commit), not a rule stage.

use std::collections::BTreeMap;

use crate::stl::{StlHashMap, STL_DEFAULT_HINT};

use super::grid::{CellGrid, ExtraCell, IntersectionRecord, SolveState};
use super::text::{atol, rz_atou32_hex, rz_convert_u32, scan, IniSection};
use super::{d_mask, network_name_to_id, type_bit, HIGHWAY, ONE_WAY_ROAD};

/// The one-way pattern of each `OneWayDir` (`0x00AA752C`).
pub const ONE_WAY_PATTERNS: [u8; 8] = [0x22, 0xAA, 0x88, 0x99, 0x11, 0x55, 0x44, 0x66];

/// The occupant flag of an intersection piece (network occupant slot 8).
pub const INTERSECTION_FLAG: u32 = 0x10_0000;

/// One check type (24 bytes, §10.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckType {
    /// `+0`, `+1`: the types, -1 when absent.
    pub type1: i8,
    pub type2: i8,
    pub edges1: u32,
    pub mask1: u32,
    pub edges2: u32,
    pub mask2: u32,
    pub multi_ok: bool,
    pub occupies: bool,
    pub optional: bool,
}

impl Default for CheckType {
    fn default() -> Self {
        CheckType {
            type1: -1,
            type2: -1,
            edges1: 0,
            mask1: 0xFFFF_FFFF,
            edges2: 0,
            mask2: 0xFFFF_FFFF,
            multi_ok: false,
            occupies: true,
            optional: false,
        }
    }
}

impl CheckType {
    /// The network checks `(type, edges, mask)` with a type.
    pub fn checks(&self) -> impl Iterator<Item = (u8, u32, u32)> {
        [
            (self.type1, self.edges1, self.mask1),
            (self.type2, self.edges2, self.mask2),
        ]
        .into_iter()
        .filter(|c| c.0 >= 0)
        .map(|(t, e, m)| (t as u8, e, m))
    }

    /// The OR of the type bits named.
    pub fn type_bits(&self) -> u32 {
        self.checks().fold(0, |b, (t, _, _)| b | type_bit(t))
    }

    /// Parses a `CheckType` value (`cParser`, `0x0063DDF0`, `0x0063F1F0`):
    /// `<tag> - <Network>:<edges>[,<mask>] [...] [check] [multiok]
    /// [optional]`. `#` starts a comment, `;` ends the value. A word that
    /// is neither a network name nor a keyword fails the definition
    /// *[choice]*; keywords and names compare case-insensitively.
    pub fn parse(value: &str) -> Result<(u8, CheckType), String> {
        let toks = check_tokens(value);
        let mut it = toks.iter().map(String::as_str).peekable();
        let tag = it.next().ok_or("empty check type")?;
        if tag.len() != 1 {
            return Err("Bad map tag".into());
        }
        if it.next() != Some("-") {
            return Err("missing '-' after the tag".into());
        }
        let mut ct = CheckType::default();
        let mut n = 0;
        while let Some(w) = it.next() {
            if w.eq_ignore_ascii_case("check") {
                ct.occupies = false;
            } else if w.eq_ignore_ascii_case("optional") {
                ct.optional = true;
                ct.occupies = false;
            } else if w.eq_ignore_ascii_case("multiok") {
                ct.multi_ok = true;
            } else if let Some(t) = network_name_to_id(w) {
                if n == 2 {
                    return Err("Too many network checks in one rule".into());
                }
                if it.next() != Some(":") {
                    return Err(format!("missing ':' after {w}"));
                }
                let edges = rz_atou32_hex(it.next().ok_or("missing edges")?);
                let mut mask = 0xFFFF_FFFF;
                if it.peek() == Some(&",") {
                    it.next();
                    mask = rz_atou32_hex(it.next().ok_or("missing mask")?);
                }
                if n == 0 {
                    (ct.type1, ct.edges1, ct.mask1) = (t as i8, edges, mask);
                } else {
                    (ct.type2, ct.edges2, ct.mask2) = (t as i8, edges, mask);
                }
                n += 1;
            } else {
                return Err(format!("unexpected '{w}'"));
            }
        }
        Ok((tag.as_bytes()[0], ct))
    }
}

/// The check-type tokenizer: blanks separate; `-`, `:` and `,` are tokens of
/// their own; `#` and `;` end the value.
fn check_tokens(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in value.chars() {
        if c == '#' || c == ';' {
            break;
        }
        if c.is_ascii_whitespace() || c == '-' || c == ':' || c == ',' {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            if !c.is_ascii_whitespace() {
                out.push(c.to_string());
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// One model of a rule (`Piece`, 16-byte entries at `+0x04`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Model {
    pub id: u32,
    pub rot: u8,
    pub flip: u8,
    pub x: f32,
    pub z: f32,
}

/// One tagged cell `(x, z, tag)` (`+0x58`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaggedCell {
    pub x: i32,
    pub z: i32,
    pub tag: u8,
}

/// The bounds (`+0x18` minX, `+0x1C` minZ, `+0x20` maxX, `+0x24` maxZ).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    pub min_x: i32,
    pub min_z: i32,
    pub max_x: i32,
    pub max_z: i32,
}

impl Default for Bounds {
    fn default() -> Self {
        Bounds {
            min_x: 1000,
            min_z: 1000,
            max_x: -1000,
            max_z: -1000,
        }
    }
}

impl Bounds {
    fn extend(&mut self, x: i32, z: i32) {
        self.min_x = self.min_x.min(x);
        self.max_x = self.max_x.max(x);
        self.min_z = self.min_z.min(z);
        self.max_z = self.max_z.max(z);
    }
}

/// One highway intersection rule.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IntRule {
    pub id: u32,
    pub models: Vec<Model>,
    /// `StepOffsets` (`+0x10`, `+0x14`).
    pub step_offsets: (i32, i32),
    pub bounds: Bounds,
    /// `+` cells (`+0x28`).
    pub new_cells: Vec<(i32, i32)>,
    /// `ReplacementIntersection` (`+0x48`) and its offset (`+0x40`, `+0x44`).
    pub replacement: bool,
    pub replacement_offset: (i32, i32),
    /// The occupied cells (`+0x4C`).
    pub occupied: Vec<(i32, i32)>,
    pub tagged: Vec<TaggedCell>,
    /// The tag map (`+0x64`).
    pub check_types: BTreeMap<u8, CheckType>,
    pub convert_query_id: u32,
    pub place_query_id: u32,
    pub preview_effect: String,
    /// `HandleOffset` (`+0x94`, `+0x98`).
    pub handle_offset: (i32, i32),
    /// The constraint layout's centre (`+0xAC` column, `+0xB0` row), -1 when
    /// not marked, and its cells `(x, z, code)` relative to that centre
    /// *[choice: the grid `+0x9C` is kept as a cell list]*.
    pub constraint_centre: (i32, i32),
    pub constraint_cells: Vec<(i32, i32, u8)>,
    /// `Costs` (`+0xB4`).
    pub costs: i32,
    pub auto_tile_base: u32,
    pub auto_path_base: u32,
    /// `+0xE4`: the OR of the type bits named by the check types.
    pub type_mask: u32,
    /// `+0xE8`.
    pub one_way_dir: u8,
    /// `+0xE9`.
    pub auto_place: bool,
    /// `RotationRing` links (`+0xEC` previous, `+0xF0` next).
    pub ring_prev: u32,
    pub ring_next: u32,
    /// `AddTypes` links (`+0xF4`, `+0xF8`) *[inferred, NR3]*.
    pub add_prev: u32,
    pub add_next: u32,
    pub cell_lines: Vec<String>,
    pub cons_lines: Vec<String>,
}

/// The cell questions `Test` asks for new cells (§10.5).
pub trait IntRuleHost {
    /// Terrain slot 34: the cell holds water.
    fn is_water(&self, _x: i32, _z: i32) -> bool {
        false
    }
    /// Occupant manager slot 32 with the network filter `0x1F4F`.
    fn has_network_occupant(&self, _x: i32, _z: i32) -> bool {
        false
    }
    /// `ComputeDemolitionRegionForPlacementCells` succeeds.
    fn demolition_region_ok(&self, _cells: &[(i32, i32)]) -> bool {
        true
    }
}

/// A flat dry empty city.
pub struct DryHost;

impl IntRuleHost for DryHost {}

fn turn((x, z): (i32, i32)) -> (i32, i32) {
    (-z, x)
}

impl IntRule {
    /// A new rule: its id, its ring links to itself, the default bounds.
    pub fn new(id: u32) -> IntRule {
        IntRule {
            id,
            ring_prev: id,
            ring_next: id,
            add_prev: id,
            add_next: id,
            constraint_centre: (-1, -1),
            ..IntRule::default()
        }
    }

    /// Adds a check type to the tag map and its bits to `+0xE4`.
    pub fn add_check_type(&mut self, tag: u8, ct: CheckType) {
        self.type_mask |= ct.type_bits();
        self.check_types.insert(tag, ct);
    }

    /// `ParseCellsFromASCIIArt` (`0x00609FC0`) on the cell layout lines, then
    /// `ParseConstraintsFromASCIIArt` (`0x0060A560`) on the constraint lines,
    /// each when its line list is not empty. The cell lists and the bounds are
    /// rebuilt *[choice]*.
    pub fn parse_layouts(&mut self) {
        if !self.cell_lines.is_empty() {
            self.tagged.clear();
            self.new_cells.clear();
            self.occupied.clear();
            self.bounds = Bounds::default();
            let (mut cx, mut cz) = (-1, -1);
            for (z, line) in self.cell_lines.iter().enumerate() {
                for (x, ch) in line.bytes().enumerate() {
                    let (x, z) = (x as i32, z as i32);
                    match ch {
                        b' ' | b'.' => {}
                        b'+' => {
                            self.new_cells.push((x, z));
                            self.occupied.push((x, z));
                            self.bounds.extend(x, z);
                        }
                        b'<' => cz = z,
                        b'^' => cx = x,
                        c if c.is_ascii_alphanumeric() => {
                            if let Some(ct) = self.check_types.get(&c) {
                                self.tagged.push(TaggedCell { x, z, tag: c });
                                if ct.occupies {
                                    self.occupied.push((x, z));
                                    self.bounds.extend(x, z);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            let shift = |p: &mut (i32, i32)| {
                p.0 -= cx;
                p.1 -= cz;
            };
            self.new_cells.iter_mut().for_each(shift);
            self.occupied.iter_mut().for_each(shift);
            for t in &mut self.tagged {
                t.x -= cx;
                t.z -= cz;
            }
            self.bounds.min_x -= cx;
            self.bounds.max_x -= cx;
            self.bounds.min_z -= cz;
            self.bounds.max_z -= cz;
        }
        if !self.cons_lines.is_empty() {
            let (mut cx, mut cz) = (-1, -1);
            let mut cells = Vec::new();
            for (z, line) in self.cons_lines.iter().enumerate() {
                for (x, ch) in line.bytes().enumerate() {
                    let code = match ch {
                        b'-' => 1,
                        b'|' => 2,
                        b'/' => 3,
                        b'\\' => 4,
                        b'F' => 5,
                        b'R' => 6,
                        b'#' => 7,
                        b'X' => 8,
                        b'<' => {
                            cz = z as i32;
                            0
                        }
                        b'^' => {
                            cx = x as i32;
                            0
                        }
                        _ => 0,
                    };
                    if code != 0 {
                        cells.push((x as i32, z as i32, code));
                    }
                }
            }
            self.constraint_centre = (cx, cz);
            self.constraint_cells = cells.into_iter().map(|(x, z, c)| (x - cx, z - cz, c)).collect();
        }
    }

    /// `Translate(a, b)` (`0x00608D20`, §10.4).
    pub fn translate(&mut self, a: i32, b: i32) {
        let mv = |p: &mut (i32, i32)| {
            p.0 += a;
            p.1 += b;
        };
        self.new_cells.iter_mut().for_each(mv);
        self.occupied.iter_mut().for_each(mv);
        for t in &mut self.tagged {
            t.x += a;
            t.z += b;
        }
        for m in &mut self.models {
            m.x += a as f32;
            m.z += b as f32;
        }
        self.handle_offset.0 += a;
        self.handle_offset.1 += b;
        self.bounds.min_x += a;
        self.bounds.max_x += a;
        self.bounds.min_z += b;
        self.bounds.max_z += b;
        self.constraint_centre.0 -= a;
        self.constraint_centre.1 -= b;
        // The cells are kept relative to the centre: moving the centre by
        // (-a, -b) moves them by (a, b).
        for c in &mut self.constraint_cells {
            c.0 += a;
            c.1 += b;
        }
    }

    /// `Transpose` (`0x006091F0`, §10.4): the mirror across x = z.
    pub fn transpose(&mut self) {
        let sw = |p: &mut (i32, i32)| *p = (p.1, p.0);
        self.new_cells.iter_mut().for_each(sw);
        self.occupied.iter_mut().for_each(sw);
        for t in &mut self.tagged {
            (t.x, t.z) = (t.z, t.x);
        }
        for m in &mut self.models {
            (m.x, m.z) = (m.z, m.x);
            m.flip = u8::from(m.flip == 0);
            m.rot = if m.flip != 0 {
                m.rot.wrapping_add(1) & 3
            } else {
                m.rot.wrapping_sub(1) & 3
            };
        }
        sw(&mut self.handle_offset);
        let b = self.bounds;
        self.bounds = Bounds {
            min_x: b.min_z,
            min_z: b.min_x,
            max_x: b.max_z,
            max_z: b.max_x,
        };
        for ct in self.check_types.values_mut() {
            ct.edges1 = transpose_edges(ct.edges1);
            ct.edges2 = transpose_edges(ct.edges2);
            ct.mask1 = swap_sides(ct.mask1);
            ct.mask2 = swap_sides(ct.mask2);
        }
        sw(&mut self.constraint_centre);
        for c in &mut self.constraint_cells {
            (c.0, c.1) = (c.1, c.0);
        }
        self.one_way_dir = (self.one_way_dir.wrapping_neg().wrapping_sub(2)) & 7;
    }

    /// `Rotate(n)` (`0x006094B0`, §10.4): `n & 3` quarter turns, one turn
    /// mapping (x, z) to (-z, x). The bounds keep the odd-turn quirk:
    /// `maxX := -(old minX)` *[quirk 8]*.
    pub fn rotate(&mut self, n: i32) {
        let n = (n & 3) as u8;
        if n == 0 {
            return;
        }
        for _ in 0..n {
            self.new_cells.iter_mut().for_each(|p| *p = turn(*p));
            self.occupied.iter_mut().for_each(|p| *p = turn(*p));
            for t in &mut self.tagged {
                (t.x, t.z) = turn((t.x, t.z));
            }
            self.handle_offset = turn(self.handle_offset);
            for m in &mut self.models {
                (m.x, m.z) = (-m.z, m.x);
            }
            for c in &mut self.constraint_cells {
                (c.0, c.1) = turn((c.0, c.1));
            }
        }
        for m in &mut self.models {
            m.rot = if m.flip == 0 {
                m.rot.wrapping_add(n) & 3
            } else {
                m.rot.wrapping_sub(n) & 3
            };
        }
        for ct in self.check_types.values_mut() {
            let r = 8 * u32::from(n);
            ct.edges1 = ct.edges1.rotate_left(r);
            ct.edges2 = ct.edges2.rotate_left(r);
            ct.mask1 = ct.mask1.rotate_left(r);
            ct.mask2 = ct.mask2.rotate_left(r);
        }
        let mut b = self.bounds;
        if n & 2 != 0 {
            b = Bounds {
                min_x: -b.max_x,
                max_x: -b.min_x,
                min_z: -b.max_z,
                max_z: -b.min_z,
            };
        }
        if n & 1 != 0 {
            let old = b;
            b.min_z = old.min_x;
            b.min_x = -old.max_z;
            b.max_z = old.max_x;
            b.max_x = -old.min_x;
        }
        self.bounds = b;
        self.one_way_dir = self.one_way_dir.wrapping_add(2 * n) & 7;
    }

    /// `IsWithinBounds(x, z, width, height)` (`0x00608CD0`).
    pub fn is_within_bounds(&self, x: i32, z: i32, width: u32, height: u32) -> bool {
        x.wrapping_add(self.bounds.min_x) >= 0
            && z.wrapping_add(self.bounds.min_z) >= 0
            && (x.wrapping_add(self.bounds.max_x) as u32) < width
            && (z.wrapping_add(self.bounds.max_z) as u32) < height
    }

    fn one_way_pattern(&self) -> u8 {
        ONE_WAY_PATTERNS[usize::from(self.one_way_dir & 7)]
    }

    /// `Test(x, z, tool)` (`0x00609A00`, §10.5): 0 match, 1 no match, 2 hard
    /// failure.
    pub fn test(&self, x: i32, z: i32, grid: &CellGrid, host: &dyn IntRuleHost) -> u8 {
        let ow = self.one_way_pattern();
        for tc in &self.tagged {
            let Some(ct) = self.check_types.get(&tc.tag) else {
                continue;
            };
            if ct.optional {
                continue;
            }
            let Some(info) = grid.at(x + tc.x, z + tc.z) else {
                return 2;
            };
            if ct.occupies {
                let occ_flag = info
                    .occupant
                    .is_some_and(|o| o.network_flags & INTERSECTION_FLAG == INTERSECTION_FLAG);
                if occ_flag || info.intersection {
                    if !self.replacement {
                        return 1;
                    }
                    if occ_flag {
                        let anchor = info.occupant.and_then(|o| o.anchor);
                        let want = (x + self.replacement_offset.0, z + self.replacement_offset.1);
                        if anchor != Some(want) {
                            return 1;
                        }
                    }
                }
                if info.fixed && !info.intersection {
                    return 1;
                }
            }
            let mut acc = 0u32;
            let mut one_way_edges = 0u32;
            for (t, edges, mask) in ct.checks() {
                let bit = type_bit(t);
                if info.flags & bit == 0 {
                    return 1;
                }
                acc |= bit;
                if t == ONE_WAY_ROAD {
                    one_way_edges = edges;
                }
                if mask & (info.edges[t as usize % 13] ^ edges) != 0 {
                    return 2;
                }
            }
            if !ct.multi_ok && info.flags & !acc & 0x1FFF != 0 {
                return 1;
            }
            if acc & type_bit(ONE_WAY_ROAD) != 0 && !ct.occupies && (info.one_way ^ ow) & d_mask(one_way_edges) != 0 {
                return 2;
            }
        }
        if self.new_cells.is_empty() {
            return 0;
        }
        let cells: Vec<(i32, i32)> = self.new_cells.iter().map(|&(rx, rz)| (x + rx, z + rz)).collect();
        for &(cx, cz) in &cells {
            if host.is_water(cx, cz) || host.has_network_occupant(cx, cz) {
                return 1;
            }
        }
        if host.demolition_region_ok(&cells) {
            0
        } else {
            1
        }
    }

    /// `DoAutoComplete(x, z, tool)` (`0x006097E0`, §10.5): appends the extra
    /// cells to `state.extra_cells`. Returns 1.
    pub fn do_auto_complete(&self, x: i32, z: i32, grid: &CellGrid, state: &mut SolveState) -> i32 {
        let ow = self.one_way_pattern();
        for tc in &self.tagged {
            let Some(ct) = self.check_types.get(&tc.tag) else {
                continue;
            };
            let (cx, cz) = (x + tc.x, z + tc.z);
            let info = grid.at(cx, cz);
            for (t, edges, mask) in ct.checks() {
                let mismatch = info.map_or(true, |i| mask & (i.edges[t as usize % 13] ^ edges) != 0);
                let (mut owbad, mut d) = (false, 0u8);
                if t == ONE_WAY_ROAD {
                    let cow = info.map_or(0, |i| i.one_way);
                    d = d_mask(edges);
                    owbad = d & (cow ^ ow) != 0;
                    if !ct.occupies {
                        let g = (0..4).fold(0u8, |g, i| {
                            if (cow >> (2 * i)) & 3 != 0 {
                                g | (3 << (2 * i))
                            } else {
                                g
                            }
                        });
                        if d & g & (cow ^ ow) != 0 {
                            continue;
                        }
                    }
                }
                if info.is_some() && !mismatch && !owbad {
                    continue;
                }
                state.extra_cells.push(ExtraCell {
                    x: cx,
                    z: cz,
                    edges,
                    network_type: t,
                    one_way: ow & d,
                    one_way_mask: d,
                });
            }
        }
        1
    }

    /// `GetPrimaryType()` (`0x006099A0`): the first type of the tagged cell
    /// at (0, 0), else 2 (Highway).
    pub fn primary_type(&self) -> u8 {
        self.tagged
            .iter()
            .find(|t| t.x == 0 && t.z == 0)
            .and_then(|t| self.check_types.get(&t.tag))
            .filter(|ct| ct.type1 >= 0)
            .map_or(HIGHWAY, |ct| ct.type1 as u8)
    }

    /// `MarkIntersection(run, true)` (`0x0063B130`): `+0x52` on the cell infos
    /// of the occupied cells.
    pub fn mark_intersection(&self, x: i32, z: i32, grid: &mut CellGrid) {
        for &(rx, rz) in &self.occupied {
            if let Some(c) = grid.at_mut(x + rx, z + rz) {
                c.intersection = true;
            }
        }
    }
}

/// Swaps bytes W <-> N and E <-> S.
fn swap_sides(e: u32) -> u32 {
    let b = e.to_le_bytes();
    u32::from_le_bytes([b[1], b[0], b[3], b[2]])
}

/// The transpose of a check type's edges (`0x00608C50`): the side swap, and
/// each byte whose whole value is 1, 2 or 3 becomes `4 - value` (a byte with
/// subtype bits is left as it is *[quirk 7]*).
fn transpose_edges(e: u32) -> u32 {
    u32::from_le_bytes(
        swap_sides(e)
            .to_le_bytes()
            .map(|b| if (1..=3).contains(&b) { 4 - b } else { b }),
    )
}

/// The global rule map (`0x00B452B0`): a hash map keyed by id. Its bucket
/// hint is the default 100 (193 buckets) *[choice]*.
#[derive(Clone, Debug)]
pub struct IntRuleMap {
    map: StlHashMap<IntRule>,
    /// Keys and values that did not parse: `(section, key, why)`.
    pub failures: Vec<(String, String, String)>,
}

impl Default for IntRuleMap {
    fn default() -> Self {
        IntRuleMap::new()
    }
}

fn two_ints(v: &str) -> Option<(i32, i32)> {
    let s = scan("%d,%d", v);
    (s.len() == 2).then(|| (s[0].as_i32(), s[1].as_i32()))
}

impl IntRuleMap {
    pub fn new() -> IntRuleMap {
        IntRuleMap {
            map: StlHashMap::with_hint(STL_DEFAULT_HINT),
            failures: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn get(&self, id: u32) -> Option<&IntRule> {
        self.map.get(id)
    }

    /// The rules in the map's hash order.
    pub fn iter(&self) -> impl Iterator<Item = &IntRule> {
        self.map.iter().map(|(_, r)| r)
    }

    /// Adds or replaces a rule (a replaced rule keeps its place in the
    /// order).
    pub fn insert(&mut self, rule: IntRule) {
        let id = rule.id;
        *self.map.entry(id, IntRule::default) = rule;
    }

    /// `ReadHighwayIntersectionRules` (`0x00641B60`, §10.1): every section in
    /// file order. Returns (rules read, keys or sections that failed).
    pub fn load(&mut self, sections: &[IniSection]) -> (usize, usize) {
        let mut rules = 0;
        let before = self.failures.len();
        for sec in sections {
            let lower = sec.name.to_ascii_lowercase();
            if lower.starts_with("ordering") {
                self.read_ordering(sec);
            } else if let Some(hex) = lower.strip_prefix("highwayintersectioninfo_") {
                let rule = self.read_rule(rz_atou32_hex(hex), sec);
                self.insert(rule);
                rules += 1;
            }
        }
        (rules, self.failures.len() - before)
    }

    /// `ReadIntersectionRule` (`0x00641A50`) and `ReadIntersectionEntries`
    /// (`0x00640320`).
    fn read_rule(&mut self, id: u32, sec: &IniSection) -> IntRule {
        let mut r = IntRule::new(id);
        for (key, value) in &sec.entries {
            let k = key.to_ascii_lowercase();
            let fail = |why: &str, failures: &mut Vec<(String, String, String)>| {
                failures.push((sec.name.clone(), key.clone(), why.to_string()));
            };
            match k.as_str() {
                "piece" => {
                    // Two floats, then " , %i , %i , 0x%x"; all five must be
                    // read *[choice]*.
                    let v = scan("%f , %f , %i , %i , 0x%x", value);
                    if v.len() == 5 {
                        r.models.push(Model {
                            x: v[0].as_f32(),
                            z: v[1].as_f32(),
                            rot: v[2].as_i32() as u8,
                            flip: v[3].as_i32() as u8,
                            id: v[4].as_i32() as u32,
                        });
                    } else {
                        fail("Piece needs 5 fields", &mut self.failures);
                    }
                }
                "stepoffsets" => match two_ints(value) {
                    Some(p) => r.step_offsets = p,
                    None => fail("StepOffsets needs 2 fields", &mut self.failures),
                },
                "celllayout" => r.cell_lines.push(value.clone()),
                "conslayout" => r.cons_lines.push(value.clone()),
                "autotilebase" => r.auto_tile_base = rz_convert_u32(value),
                "autopathbase" => r.auto_path_base = rz_convert_u32(value),
                "checktype" => match CheckType::parse(value) {
                    Ok((tag, ct)) => r.add_check_type(tag, ct),
                    Err(why) => fail(&why, &mut self.failures),
                },
                "copyfrom" => {
                    let from = rz_atou32_hex(value);
                    if let Some(src) = self.map.get(from) {
                        // The copy keeps this rule's own layout lines
                        // *[choice]*.
                        let (cells, cons) = (std::mem::take(&mut r.cell_lines), std::mem::take(&mut r.cons_lines));
                        r = src.clone();
                        r.id = id;
                        r.cell_lines = cells;
                        r.cons_lines = cons;
                        (r.ring_prev, r.ring_next, r.add_prev, r.add_next) = (id, id, id, id);
                    } else {
                        fail(&format!("CopyFrom: no rule {from:#x} (yet)"), &mut self.failures);
                    }
                }
                "transpose" => r.transpose(),
                "rotate" => r.rotate(atol(value)),
                // Applied only when both numbers are read *[choice]*.
                "translate" => match two_ints(value) {
                    Some((a, b)) => r.translate(a, b),
                    None => fail("Translate needs 2 fields", &mut self.failures),
                },
                "replacementintersection" => {
                    // The count is not checked; a missing number reads 0
                    // *[choice]*.
                    let s = scan("%d,%d", value);
                    r.replacement = true;
                    r.replacement_offset = (s.first().map_or(0, |v| v.as_i32()), s.get(1).map_or(0, |v| v.as_i32()));
                }
                "convertqueryid" => r.convert_query_id = rz_convert_u32(value),
                "placequeryid" => r.place_query_id = rz_convert_u32(value),
                "handleoffset" => match two_ints(value) {
                    Some(p) => r.handle_offset = p,
                    None => fail("HandleOffset needs 2 fields", &mut self.failures),
                },
                "previeweffect" => r.preview_effect = value.clone(),
                "costs" => {
                    if let Some(v) = scan(" %d ", value).first() {
                        r.costs = v.as_i32();
                    }
                }
                "autoplace" => r.auto_place = true,
                "onewaydir" => r.one_way_dir = atol(value) as u8,
                _ => {}
            }
        }
        r.parse_layouts();
        r
    }

    /// `ReadIntersectionOrderingEntries` (`0x006416B0`, §10.8).
    fn read_ordering(&mut self, sec: &IniSection) {
        let find = |name: &str| sec.entries.iter().find(|(k, _)| k.eq_ignore_ascii_case(name));
        let (ring, entry) = match (find("RotationRing"), find("AddTypes")) {
            (Some(e), _) => (true, e),
            (None, Some(e)) => (false, e),
            (None, None) => return,
        };
        let ids: Vec<u32> = entry
            .1
            .split(',')
            .map(rz_atou32_hex)
            .filter(|id| self.map.contains(*id))
            .collect();
        let n = ids.len();
        for (i, &id) in ids.iter().enumerate() {
            let prev = ids[(i + n - 1) % n];
            let next = ids[(i + 1) % n];
            if let Some(r) = self.map.get_mut(id) {
                if ring {
                    (r.ring_prev, r.ring_next) = (prev, next);
                } else {
                    (r.add_prev, r.add_next) = (prev, next);
                }
            }
        }
    }
}

/// `IdentifyHighwayIntersections(v, type)` (`0x0063B340`, §10.6) over the
/// dragged steps (`steps[s]` the cells of step `s`). Fills
/// `state.intersections` and appends to `state.potential`; returns `v` on
/// every path *[quirk 9]*.
pub fn identify_highway_intersections(
    rules: &IntRuleMap,
    grid: &mut CellGrid,
    state: &mut SolveState,
    steps: &[Vec<(i32, i32)>],
    v: i32,
    network_type: u32,
    host: &dyn IntRuleHost,
) -> i32 {
    state.intersections.clear();
    let mut run: Option<(u32, i32, i32, i32)> = None;
    let record = |state: &mut SolveState, (id, x, z, s): (u32, i32, i32, i32), value: u32| {
        let (a, b) = rules.get(id).map_or((0, 0), |r| r.step_offsets);
        state.intersections.push(IntersectionRecord {
            rule_id: id,
            x,
            z,
            start_step: s,
            step_a: s + a,
            step_b: s + b,
            cells: Vec::new(),
        });
        state.potential.push((x, z, value));
    };
    let last = v.clamp(0, steps.len() as i32);
    for s in 0..last {
        let mut matched = false;
        'cells: for &(x, z) in &steps[s as usize] {
            let Some(info) = grid.at(x, z) else { continue };
            if info.occupant.is_none() {
                continue;
            }
            let flags = info.flags;
            for rule in rules.iter() {
                if rule.auto_place
                    && rule.type_mask & flags != 0
                    && rule.is_within_bounds(x, z, grid.width, grid.height)
                    && rule.test(x, z, grid, host) == 0
                {
                    matched = true;
                    if run.is_none() {
                        run = Some((rule.id, x, z, s));
                        rule.mark_intersection(x, z, grid);
                    }
                    // Later matches of the step are ignored.
                    break 'cells;
                }
            }
        }
        if !matched {
            if let Some(r) = run.take() {
                record(state, r, network_type);
            }
        }
    }
    if let Some(r) = run.take() {
        record(state, r, 4);
    }
    v
}
