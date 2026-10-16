// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Tile rule files (§3), the `#Transmogrify` expansion (§4) and matching a
//! cell (`Evaluate`, §5).

use std::collections::BTreeMap;

use super::text::{atol, rz_convert_u32};
use super::{meta_of, meta_offset, orientation};

/// One condition `{m, value, mask}` (§3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Condition {
    pub m: i32,
    pub value: u32,
    pub mask: u32,
}

/// One output `{id, m, orientation}` (§3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Output {
    pub id: u32,
    pub m: u8,
    pub orientation: u8,
}

/// One rule (`cSC4TileRule`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileRule {
    /// The centre edge word (the map key).
    pub key: u32,
    /// The transform mask: `0xFF`, `0x0F` (`noflip`) or `0x01` (`notrans`).
    pub transform: u8,
    pub conditions: Vec<Condition>,
    pub outputs: Vec<Output>,
}

/// How a file loaded into an evaluator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoadStatus {
    /// No file was given (the evaluator still exists, §2.3).
    #[default]
    Missing,
    /// Read to the end of the file.
    Complete,
    /// The header was neither `#Transmogrify` nor another `#` token (§3.2):
    /// no rules.
    Rejected,
    /// A rule's centre token was not 1 (§3.3): loading stopped there; the
    /// rules read before stay.
    BadCentre,
    /// The file ended inside a centre line *[choice: as a bad centre]*.
    Truncated,
}

/// A tile evaluator (`cSC4TileEvaluator`) with its rule map (§3.4).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Evaluator {
    /// The ordered multimap: key ascending, each key's rules in insertion
    /// order.
    rules: BTreeMap<u32, Vec<TileRule>>,
    pub status: LoadStatus,
    /// The file started with `#Transmogrify`.
    pub transmogrify: bool,
    /// Rules read from the file (before the expansion).
    pub source_rules: usize,
    /// Variants dropped as duplicates by the expansion.
    pub dropped_variants: usize,
}

/// The edge token (`0x00622800`, §3.3): `w = v mod 256`, the byte
/// `((w / 10) << 3) | (w mod 10)`.
pub fn edge_token(v: i32) -> u8 {
    let w = v as u8;
    ((w / 10) << 3) | (w % 10)
}

/// The tokenizer (`0x006210B0`, `0x00622620`, §3.1): comment lines (first
/// character `;`) are skipped, tokens are separated by commas and line ends,
/// empty tokens are skipped. A `\r` before a line end is dropped *[choice]*;
/// tokens are not otherwise trimmed.
pub(crate) fn tokens(bytes: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = Vec::new();
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.starts_with(';') {
            continue;
        }
        for t in line.split(',') {
            if !t.is_empty() {
                out.push(t.to_string());
            }
        }
    }
    out
}

fn starts_with_digit(t: &str) -> bool {
    t.as_bytes().first().is_some_and(|c| c.is_ascii_digit())
}

fn edges_of(t: &[String]) -> u32 {
    t.iter()
        .enumerate()
        .fold(0, |e, (i, s)| e | (u32::from(edge_token(atol(s))) << (8 * i)))
}

impl Evaluator {
    /// An evaluator with no rules (a missing file).
    pub fn new() -> Evaluator {
        Evaluator::default()
    }

    /// `AddRulesFromResource` (`0x00623020`): reads one file into a fresh
    /// evaluator, then expands it when its header is `#Transmogrify`.
    pub fn from_bytes(bytes: &[u8]) -> Evaluator {
        let mut ev = Evaluator::new();
        ev.add_rules(bytes);
        ev
    }

    /// Reads one file into this evaluator's map (§3.2, §3.3).
    pub fn add_rules(&mut self, bytes: &[u8]) {
        let toks = tokens(bytes);
        let mut it = toks.iter().peekable();
        let Some(header) = it.next() else {
            self.status = LoadStatus::Rejected;
            return;
        };
        let transmogrify = header == "#Transmogrify";
        if !transmogrify && !header.starts_with('#') {
            self.status = LoadStatus::Rejected;
            return;
        }
        self.transmogrify = transmogrify;
        self.status = LoadStatus::Complete;
        'rules: while let Some(first) = it.next() {
            let mut transform = 0xFFu8;
            let mut tok = first;
            if tok == "noflip" || tok == "notrans" {
                transform = if tok == "noflip" { 0x0F } else { 0x01 };
                match it.next() {
                    Some(t) => tok = t,
                    None => {
                        self.status = LoadStatus::Truncated;
                        break;
                    }
                }
            }
            // The centre token is compared by value *[choice]*.
            if atol(tok) != 1 {
                self.status = LoadStatus::BadCentre;
                break;
            }
            let centre: Vec<String> = it.by_ref().take(4).cloned().collect();
            if centre.len() < 4 {
                self.status = LoadStatus::Truncated;
                break;
            }
            let mut rule = TileRule {
                key: edges_of(&centre),
                transform,
                conditions: Vec::new(),
                outputs: Vec::new(),
            };
            loop {
                let Some(next) = it.peek() else {
                    // The end of the file: kept only with an output.
                    if !rule.outputs.is_empty() {
                        self.insert(rule);
                    }
                    break 'rules;
                };
                let kind = atol(next);
                if !starts_with_digit(next) || (kind != 2 && kind != 3) {
                    // The next rule's token: kept even with no outputs.
                    self.insert(rule);
                    continue 'rules;
                }
                it.next();
                let want = if kind == 2 { 5 } else { 4 };
                let line: Vec<String> = it.by_ref().take(want).cloned().collect();
                if line.len() < want {
                    // A line cut by the end of the file is dropped and the
                    // rule ends as at the end of the file *[choice]*.
                    if !rule.outputs.is_empty() {
                        self.insert(rule);
                    }
                    break 'rules;
                }
                if kind == 2 {
                    let mut value = 0u32;
                    let mut mask = 0u32;
                    for (i, t) in line[1..].iter().enumerate() {
                        if t == "?" {
                            continue;
                        }
                        value |= u32::from(edge_token(atol(t))) << (8 * i);
                        mask |= 0xFF << (8 * i);
                    }
                    rule.conditions.push(Condition {
                        m: atol(&line[0]),
                        value,
                        mask,
                    });
                } else {
                    rule.outputs.push(Output {
                        m: atol(&line[0]) as u8,
                        id: rz_convert_u32(&line[1]),
                        orientation: orientation(atol(&line[2]), atol(&line[3])),
                    });
                }
            }
        }
        if self.transmogrify {
            self.expand();
        }
    }

    /// Inserts a rule after every existing rule with an equal key.
    pub fn insert(&mut self, rule: TileRule) {
        self.source_rules += 1;
        self.rules.entry(rule.key).or_default().push(rule);
    }

    /// The rules in map order.
    pub fn rules(&self) -> impl Iterator<Item = &TileRule> {
        self.rules.values().flatten()
    }

    /// The rules of one key, in insertion order.
    pub fn rules_for(&self, key: u32) -> &[TileRule] {
        self.rules.get(&key).map_or(&[], |v| v.as_slice())
    }

    pub fn rule_count(&self) -> usize {
        self.rules.values().map(Vec::len).sum()
    }

    /// The `#Transmogrify` expansion (`0x00622B50`, §4).
    fn expand(&mut self) {
        let source = std::mem::take(&mut self.rules);
        let mut out: BTreeMap<u32, Vec<TileRule>> = BTreeMap::new();
        let mut dropped = 0usize;
        for rule in source.values().flatten() {
            let mut kept: Vec<TileRule> = Vec::new();
            for t in 0..8u8 {
                if rule.transform & (1 << t) == 0 {
                    continue;
                }
                let mut v = if t & 4 != 0 { mirror_rule(rule) } else { rule.clone() };
                v = rotate_rule(&v, t & 3);
                if kept.iter().any(|k| same_pattern(k, &v)) {
                    dropped += 1;
                    continue;
                }
                v.transform = 0x01;
                kept.push(v);
            }
            for v in kept {
                out.entry(v.key).or_default().push(v);
            }
        }
        self.rules = out;
        self.dropped_variants = dropped;
    }

    /// `Evaluate` (`0x006220D0`, §5.2): the outputs of the first rule for the
    /// centre key whose conditions hold and whose outputs are all valid
    /// positions; `None` when no rule passes.
    pub fn evaluate(&self, n: &dyn Neighbourhood) -> Option<&[Output]> {
        let rules = self.rules.get(&n.centre())?;
        rules
            .iter()
            .find(|r| {
                r.conditions.iter().all(|c| c.mask & (c.value ^ n.edges(c.m)) == 0)
                    && r.outputs.iter().all(|o| n.valid(i32::from(o.m)))
            })
            .map(|r| r.outputs.as_slice())
    }
}

/// The neighbourhood of a centre cell (`0x00627080`, §5.1).
pub trait Neighbourhood {
    /// The centre key: the centre cell info's edge word for the type, or 0.
    fn centre(&self) -> u32;
    /// The edge word for the type at metaposition `m`, or 0 without a cell
    /// info.
    fn edges(&self, m: i32) -> u32;
    /// `IsValidSolutionPos` (`0x00623F40`) at metaposition `m`.
    fn valid(&self, m: i32) -> bool;
}

/// Same centre key and the same set of conditions as `(m, value)` pairs, in
/// both directions (`0x006214A0`); masks and outputs are not compared.
fn same_pattern(a: &TileRule, b: &TileRule) -> bool {
    let has = |x: &TileRule, c: &Condition| x.conditions.iter().any(|d| d.m == c.m && d.value == c.value);
    a.key == b.key && a.conditions.iter().all(|c| has(b, c)) && b.conditions.iter().all(|c| has(a, c))
}

// ---- §4.1 rotation --------------------------------------------------------------------------------------

/// A metaposition turned by `r` quarter turns: 0 stays; 1-8 move by `2r`
/// around the inner ring, 9-24 by `4r` around the outer ring. Others are
/// left unchanged *[choice]*.
pub fn rotate_meta(m: i32, r: u8) -> i32 {
    let r = i32::from(r & 3);
    match m {
        1..=8 => (m - 1 + 2 * r).rem_euclid(8) + 1,
        9..=24 => (m - 9 + 4 * r).rem_euclid(16) + 9,
        _ => m,
    }
}

/// The table `0x00AA8328`: rows `[0,3,2,1]`, `[1,0,3,2]`, `[2,1,0,3]`,
/// `[3,2,1,0]`, i.e. `(rot - r) & 3`.
const FLIPPED_ROTATION: [u8; 16] = [0, 3, 2, 1, 1, 0, 3, 2, 2, 1, 0, 3, 3, 2, 1, 0];

/// An output orientation turned by `r` quarter turns (§4.1). A flipped
/// orientation whose rotation is 4 or more reads past the table *[quirk 1]*;
/// here it uses the table's rule `(rot - r) & 3` *[choice]*.
pub fn rotate_orientation(o: u8, r: u8) -> u8 {
    let r = r & 3;
    if o & 0x80 == 0 {
        o.wrapping_add(r) & 3
    } else {
        let rot = o & 0x7F;
        let idx = usize::from(rot) * 4 + usize::from(r);
        let v = FLIPPED_ROTATION
            .get(idx)
            .copied()
            .unwrap_or(rot.wrapping_sub(r) & 3);
        0x80 | v
    }
}

/// An edge word turned by `r` quarter turns: byte `i` moves to side
/// `(i + r) & 3`.
pub fn rotate_edges(e: u32, r: u8) -> u32 {
    e.rotate_left(8 * u32::from(r & 3))
}

fn rotate_rule(rule: &TileRule, r: u8) -> TileRule {
    TileRule {
        key: rotate_edges(rule.key, r),
        transform: rule.transform,
        conditions: rule
            .conditions
            .iter()
            .map(|c| Condition {
                m: rotate_meta(c.m, r),
                value: rotate_edges(c.value, r),
                mask: rotate_edges(c.mask, r),
            })
            .collect(),
        outputs: rule
            .outputs
            .iter()
            .map(|o| Output {
                id: o.id,
                m: rotate_meta(i32::from(o.m), r) as u8,
                orientation: rotate_orientation(o.orientation, r),
            })
            .collect(),
    }
}

// ---- §4.2 mirror ------------------------------------------------------------------------------------------

/// A metaposition mirrored west-east: `(dx, dz) -> (-dx, dz)`. One outside
/// the table is left unchanged *[choice]*.
pub fn mirror_meta(m: i32) -> i32 {
    if !(0..25).contains(&m) {
        return m;
    }
    let (dx, dz) = meta_offset(m);
    meta_of(-dx, dz).unwrap_or(m)
}

/// One edge byte mirrored (`0x00621210`): kind 1 <-> 3 (subtype kept); kind
/// 2: bit 3 kept, bits 4 and 5 swapped, bits 6-7 dropped *[quirk 2]*; kinds
/// 0 and 4-7 unchanged (the kind is bits 0-2).
pub fn mirror_edge_byte(b: u8) -> u8 {
    match b & 7 {
        1 => (b & !7) | 3,
        3 => (b & !7) | 1,
        2 => (b & 0x0F) | ((b & 0x10) << 1) | ((b & 0x20) >> 1),
        _ => b,
    }
}

/// An edge word mirrored: the W and E bytes swap, then each byte goes
/// through [`mirror_edge_byte`].
pub fn mirror_edges(e: u32) -> u32 {
    let b = e.to_le_bytes();
    let swapped = [b[2], b[1], b[0], b[3]];
    u32::from_le_bytes(swapped.map(mirror_edge_byte))
}

fn mirror_rule(rule: &TileRule) -> TileRule {
    TileRule {
        key: mirror_edges(rule.key),
        transform: rule.transform,
        conditions: rule
            .conditions
            .iter()
            .map(|c| Condition {
                m: mirror_meta(c.m),
                value: mirror_edges(c.value),
                mask: mirror_edges(c.mask),
            })
            .collect(),
        outputs: rule
            .outputs
            .iter()
            .map(|o| Output {
                id: o.id,
                m: mirror_meta(i32::from(o.m)) as u8,
                orientation: o.orientation ^ 0x80,
            })
            .collect(),
    }
}
