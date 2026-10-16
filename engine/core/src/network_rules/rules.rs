// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The rule database and its resources (§2): the tile evaluators of every
//! type, the multi-type intersection solutions and the override matrix
//! (§8.1), the `[RuleOverrides]` patches (§9.1), the tile verifiers (§9.2)
//! and the highway intersection rules (§10).

use std::collections::{BTreeMap, HashMap};

use super::intrule::IntRuleMap;
use super::text::{atol, find_section, ini_sections, rz_atou32_hex, rz_convert_u32, scan};
use super::tile::{Evaluator, LoadStatus};
use super::{network_name_to_id, orientation, tile_instance_owner, tile_instances, TYPE_COUNT, TYPE_NAMES};

/// The type of every rule resource.
pub const RULE_TYPE: u32 = 0x0A5B_CF4B;
/// The group of every rule resource.
pub const RULE_GROUP: u32 = 0xAA5B_CF57;
/// The highway intersection rules (§10.1).
pub const INTERSECTION_INSTANCE: u32 = 0x1000_0000;
/// The multi-type intersection solutions and `[AllowedOverrides]` (§8.1).
pub const MULTI_TYPE_INSTANCE: u32 = 0x1000_0001;
/// The `[RuleOverrides]` tile conflict rules (§9.1).
pub const OVERRIDES_INSTANCE: u32 = 0x1000_0002;
/// The network INI (`0x8A5971C5 / 0x8A5993B9`, §2.1 step 1), read here for
/// its `[TileVerifiers]`: taken as group / instance with the rule type
/// *[inferred]*.
pub const NETWORK_INI_GROUP: u32 = 0x8A59_71C5;
pub const NETWORK_INI_INSTANCE: u32 = 0x8A59_93B9;

/// Which rule resource a byte buffer is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceId {
    /// Type `0x0A5BCF4B`, group `0xAA5BCF57`, this instance.
    Rule(u32),
    /// The network INI (only its `[TileVerifiers]` are read here).
    NetworkIni,
}

/// One `[RuleOverrides]` entry (§9.1): the pattern (A, then B east of A) and
/// its replacement, each `(id, orientation)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverridePair {
    pub pattern: [(u32, u8); 2],
    pub replacement: [(u32, u8); 2],
}

/// One tile verifier requirement `{id, orientation, m}` (§9.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Requirement {
    pub id: u32,
    pub orientation: u8,
    pub m: i32,
}

/// What one resource load did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadReport {
    /// A tile file: how it loaded.
    pub status: Option<LoadStatus>,
    /// A tile file: `#Transmogrify`.
    pub transmogrify: bool,
    /// A tile file: the rules read, the rules in the map after the
    /// expansion, the variants dropped.
    pub source_rules: usize,
    pub rules: usize,
    pub dropped_variants: usize,
    /// An INI: the entries taken and the entries ignored (wrong field
    /// count, unknown names, a failed parse).
    pub entries: usize,
    pub ignored: usize,
    /// An INI: the sections seen.
    pub sections: usize,
}

/// Every loaded rule (the process-wide tables of `ReloadRules`).
#[derive(Clone, Debug)]
pub struct NetworkRules {
    /// Each type's tile evaluators in load order (record `+0x24`).
    pub evaluators: Vec<Vec<Evaluator>>,
    /// The override matrix rows (record `+0xFC`): bit `u` of row `t` set when
    /// `t` may override `u`.
    pub override_rows: [u16; TYPE_COUNT],
    /// The multi-type solutions of each pair `(a, b)`, `a < b`:
    /// `(edgesA, edgesB) -> (piece id, orientation)`.
    pub solutions: BTreeMap<(u8, u8), HashMap<(u32, u32), (u32, u8)>>,
    /// `[RuleOverrides]` keyed by the pattern's first id, several per id in
    /// file order (`0x00B466D0`; NR5).
    pub rule_overrides: BTreeMap<u32, Vec<OverridePair>>,
    /// `[TileVerifiers]`: the verifiers of each piece id, any one of which
    /// may pass.
    pub verifiers: HashMap<u32, Vec<Vec<Requirement>>>,
    /// The highway intersection rules (`0x00B452B0`).
    pub int_rules: IntRuleMap,
}

impl Default for NetworkRules {
    fn default() -> Self {
        NetworkRules::new()
    }
}

impl NetworkRules {
    /// Empty tables: no evaluator, no rule (every consumer then finds that
    /// nothing applies, §2.1).
    pub fn new() -> NetworkRules {
        NetworkRules {
            evaluators: vec![Vec::new(); TYPE_COUNT],
            override_rows: [0; TYPE_COUNT],
            solutions: BTreeMap::new(),
            rule_overrides: BTreeMap::new(),
            verifiers: HashMap::new(),
            int_rules: IntRuleMap::new(),
        }
    }

    /// `ReloadRules` (`0x006253D0`, §2.1): every resource in the original's
    /// order, `fetch` answering the bytes of a resource or `None` when it is
    /// missing. Returns one report per resource asked for, in that order.
    pub fn reload(
        fetch: &mut dyn FnMut(ResourceId) -> Option<Vec<u8>>,
    ) -> (NetworkRules, Vec<(ResourceId, LoadReport)>) {
        let mut r = NetworkRules::new();
        let mut reports = Vec::new();
        for t in 0..TYPE_COUNT as u8 {
            for &inst in tile_instances(t) {
                let id = ResourceId::Rule(inst);
                let bytes = fetch(id);
                reports.push((id, r.add_tile_file(t, bytes.as_deref())));
            }
        }
        let order = [
            ResourceId::NetworkIni,
            ResourceId::Rule(MULTI_TYPE_INSTANCE),
            ResourceId::Rule(OVERRIDES_INSTANCE),
            ResourceId::Rule(INTERSECTION_INSTANCE),
        ];
        for id in order {
            let rep = match fetch(id) {
                Some(b) => r.load_resource(id, &b),
                None => LoadReport::default(),
            };
            reports.push((id, rep));
        }
        (r, reports)
    }

    /// Loads one resource by its id. A tile file is appended to its type's
    /// evaluators (the caller keeps the §2.3 order).
    pub fn load_resource(&mut self, id: ResourceId, bytes: &[u8]) -> LoadReport {
        match id {
            ResourceId::NetworkIni => self.load_tile_verifiers(bytes),
            ResourceId::Rule(MULTI_TYPE_INSTANCE) => self.load_multi_type(bytes),
            ResourceId::Rule(OVERRIDES_INSTANCE) => self.load_rule_overrides(bytes),
            ResourceId::Rule(INTERSECTION_INSTANCE) => self.load_intersections(bytes),
            ResourceId::Rule(inst) => match tile_instance_owner(inst) {
                Some((t, _)) => self.add_tile_file(t, Some(bytes)),
                None => LoadReport::default(),
            },
        }
    }

    /// Creates one tile evaluator for type `t` and pushes it (even when the
    /// file is missing or rejected, §2.3).
    pub fn add_tile_file(&mut self, t: u8, bytes: Option<&[u8]>) -> LoadReport {
        let ev = match bytes {
            Some(b) => Evaluator::from_bytes(b),
            None => Evaluator::new(),
        };
        let rep = LoadReport {
            status: Some(ev.status),
            transmogrify: ev.transmogrify,
            source_rules: ev.source_rules,
            rules: ev.rule_count(),
            dropped_variants: ev.dropped_variants,
            ..LoadReport::default()
        };
        if let Some(v) = self.evaluators.get_mut(t as usize) {
            v.push(ev);
        }
        rep
    }

    /// The multi-type solutions and `[AllowedOverrides]` (instance
    /// `0x10000001`, §8.1).
    pub fn load_multi_type(&mut self, bytes: &[u8]) -> LoadReport {
        let secs = ini_sections(bytes);
        let mut rep = LoadReport {
            sections: secs.len(),
            ..LoadReport::default()
        };
        self.solutions.clear();
        for a in 0..TYPE_COUNT as u8 {
            for b in a + 1..TYPE_COUNT as u8 {
                let name = format!(
                    "{}{}IntersectionSolutions",
                    TYPE_NAMES[a as usize], TYPE_NAMES[b as usize]
                );
                for (_, value) in find_section(&secs, &name) {
                    // RZParseDelimitedText; each field trimmed *[choice]*.
                    let f: Vec<&str> = value.split(',').map(str::trim).collect();
                    if f.len() != 5 {
                        rep.ignored += 1;
                        continue;
                    }
                    let n: Vec<u32> = f.iter().map(|s| rz_convert_u32(s)).collect();
                    self.solutions
                        .entry((a, b))
                        .or_default()
                        .insert((n[0], n[1]), (n[2], orientation(n[3] as i32, n[4] as i32)));
                    rep.entries += 1;
                }
            }
        }
        self.override_rows = [0; TYPE_COUNT];
        for (_, value) in find_section(&secs, "AllowedOverrides") {
            let mut f = value.splitn(2, ',');
            let a = f.next().and_then(network_name_to_id);
            let b = f.next().and_then(network_name_to_id);
            match (a, b) {
                (Some(a), Some(b)) => {
                    self.override_rows[a as usize] |= 1 << b;
                    self.override_rows[b as usize] |= 1 << a;
                    rep.entries += 1;
                }
                // An unknown name sets nothing *[choice]*.
                _ => rep.ignored += 1,
            }
        }
        rep
    }

    /// The `[RuleOverrides]` patches (instance `0x10000002`, §9.1).
    pub fn load_rule_overrides(&mut self, bytes: &[u8]) -> LoadReport {
        let secs = ini_sections(bytes);
        let mut rep = LoadReport {
            sections: secs.len(),
            ..LoadReport::default()
        };
        self.rule_overrides.clear();
        const PATTERN: &str = " %i , %i , %d , %i , %i , %d";
        let six = |s: &str| -> Option<[(u32, u8); 2]> {
            let v = scan(PATTERN, s);
            (v.len() == 6).then(|| {
                let i: Vec<i32> = v.iter().map(|x| x.as_i32()).collect();
                [
                    (i[0] as u32, orientation(i[1], i[2])),
                    (i[3] as u32, orientation(i[4], i[5])),
                ]
            })
        };
        for (key, value) in find_section(&secs, "RuleOverrides") {
            match (six(key), six(value)) {
                (Some(pattern), Some(replacement)) => {
                    self.rule_overrides
                        .entry(pattern[0].0)
                        .or_default()
                        .push(OverridePair { pattern, replacement });
                    rep.entries += 1;
                }
                _ => rep.ignored += 1,
            }
        }
        rep
    }

    /// The `[TileVerifiers]` of the network INI (§9.2).
    pub fn load_tile_verifiers(&mut self, bytes: &[u8]) -> LoadReport {
        let secs = ini_sections(bytes);
        let mut rep = LoadReport {
            sections: secs.len(),
            ..LoadReport::default()
        };
        self.verifiers.clear();
        for (key, value) in find_section(&secs, "TileVerifiers") {
            let id = rz_atou32_hex(key);
            let toks: Vec<&str> = value.split(',').collect();
            // A trailing group of fewer than four tokens is dropped *[choice]*.
            let reqs: Vec<Requirement> = toks
                .chunks_exact(4)
                .map(|g| Requirement {
                    m: atol(g[0]),
                    id: rz_atou32_hex(g[1]),
                    orientation: orientation(atol(g[2]), atol(g[3])),
                })
                .collect();
            if toks.len() % 4 != 0 {
                rep.ignored += 1;
            }
            self.verifiers.entry(id).or_default().push(reqs);
            rep.entries += 1;
        }
        rep
    }

    /// The highway intersection rules (instance `0x10000000`, §10.1).
    pub fn load_intersections(&mut self, bytes: &[u8]) -> LoadReport {
        let secs = ini_sections(bytes);
        let mut rep = LoadReport {
            sections: secs.len(),
            ..LoadReport::default()
        };
        let (taken, ignored) = self.int_rules.load(&secs);
        rep.entries = taken;
        rep.ignored = ignored;
        rep
    }

    /// The multi-type solution of the pair `(lo, hi)` for its two edge words.
    pub fn solution(&self, lo: u8, hi: u8, e_lo: u32, e_hi: u32) -> Option<(u32, u8)> {
        self.solutions
            .get(&(lo, hi))
            .and_then(|t| t.get(&(e_lo, e_hi)))
            .copied()
    }

    /// The evaluator count of each type.
    pub fn evaluator_counts(&self) -> [usize; TYPE_COUNT] {
        std::array::from_fn(|t| self.evaluators[t].len())
    }
}
