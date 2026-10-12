// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The flora type registry (§3.2), the static flora records (§3.3) and the
//! cluster chains and seed types (§6.11).

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::property::PropertyHolder;
use crate::props;

use super::{flag, prop, table_byte};

/// The default propagation-table byte (§3.3): `0x99` where both indices
/// are in 5..12, else 0.
pub const DEFAULT_TABLE_BYTE: u8 = 0x99;

/// [choice] The flags byte of the key-0 default record (uninitialised in the
/// original, §3.2): the code-default bits 1 and 2 plus bit 0.
pub const DEFAULT_RECORD_FLAGS: u8 = flag::NEVER_AUTO | flag::SKIP_SCORE | flag::BIT2;

/// One static flora record (`cSC4StaticFloraData`, §3.3).
#[derive(Clone, Debug, PartialEq)]
pub struct FloraRecord {
    pub type_id: u32,
    pub next_type: u32,
    /// The type planted when this flora seeds (§6.11).
    pub seed_type: Cell<u32>,
    pub water: bool,
    /// Days.
    pub maturity_age: f32,
    pub seeding_age: f32,
    pub seeding_interval: f32,
    pub seeds_per_cycle: f32,
    /// Metres.
    pub seed_spread: f32,
    /// Days: the rebirth age.
    pub lifespan: f32,
    pub max_age: f32,
    pub slope_tolerance: f32,
    pub min_score: f32,
    pub seed_probability: f32,
    /// Read by the occupant (its bounding box), not the simulator.
    pub half_width: f32,
    pub unused_12: f32,
    pub clearance_radius: f32,
    pub height: f32,
    pub cluster_radius: f32,
    pub cluster_height: f32,
    /// `[elevation index][moisture type]`.
    pub table: [u8; 256],
    pub unnamed: u32,
    pub flags: u8,
    pub families: Vec<u32>,
    pub plop_cost: i64,
}

/// The code-default propagation table.
fn default_table() -> [u8; 256] {
    let mut t = [0u8; 256];
    for e in 5..=12 {
        for m in 5..=12 {
            t[e * 16 + m] = DEFAULT_TABLE_BYTE;
        }
    }
    t
}

impl FloraRecord {
    /// A record with the code defaults of §3.3 for `type_id` (seed type =
    /// own type).
    pub fn new(type_id: u32) -> FloraRecord {
        FloraRecord {
            type_id,
            next_type: 0,
            seed_type: Cell::new(type_id),
            water: false,
            maturity_age: 730.0,
            seeding_age: 730.0,
            seeding_interval: 365.0,
            seeds_per_cycle: 2.0,
            seed_spread: 128.0,
            lifespan: 1460.0,
            max_age: 1560.0,
            slope_tolerance: 0.5,
            min_score: 0.3,
            seed_probability: 0.5,
            half_width: 1.0,
            unused_12: 4.0,
            clearance_radius: 3.0,
            height: 3.0,
            cluster_radius: 0.0,
            cluster_height: 0.0,
            table: default_table(),
            unnamed: 0,
            flags: flag::SKIP_SCORE | flag::BIT2,
            families: Vec::new(),
            plop_cost: 0,
        }
    }

    /// Field `i` (0-based) of the 14 floats of property `0x083DC35B`.
    pub fn float_mut(&mut self, i: usize) -> Option<&mut f32> {
        Some(match i {
            0 => &mut self.maturity_age,
            1 => &mut self.seeding_age,
            2 => &mut self.seeding_interval,
            3 => &mut self.seeds_per_cycle,
            4 => &mut self.seed_spread,
            5 => &mut self.lifespan,
            6 => &mut self.max_age,
            7 => &mut self.slope_tolerance,
            8 => &mut self.min_score,
            9 => &mut self.seed_probability,
            10 => &mut self.half_width,
            11 => &mut self.unused_12,
            12 => &mut self.clearance_radius,
            13 => &mut self.height,
            _ => return None,
        })
    }

    /// The 14 floats in property order.
    pub fn floats(&self) -> [f32; 14] {
        [
            self.maturity_age,
            self.seeding_age,
            self.seeding_interval,
            self.seeds_per_cycle,
            self.seed_spread,
            self.lifespan,
            self.max_age,
            self.slope_tolerance,
            self.min_score,
            self.seed_probability,
            self.half_width,
            self.unused_12,
            self.clearance_radius,
            self.height,
        ]
    }

    /// A record built from a flora exemplar (§3.3). The caller checks the
    /// ExemplarType.
    pub fn from_exemplar(h: &PropertyHolder) -> FloraRecord {
        let type_id = props::u32_of(h, prop::TYPE_ID).unwrap_or(0);
        let mut r = FloraRecord::new(type_id);
        if let Some(v) = props::u32_of(h, prop::NEXT_CLUSTER_TYPE) {
            r.next_type = v;
        }
        r.water = h.has(prop::WATER);
        if let Some(v) = props::f32s(h, prop::FLOATS) {
            for (i, x) in v.iter().take(14).enumerate() {
                if let Some(f) = r.float_mut(i) {
                    *f = *x;
                }
            }
        }
        if let Some(v) = props::f32_of(h, prop::CLUSTER_RADIUS) {
            r.cluster_radius = v;
        }
        if let Some(v) = props::f32_of(h, prop::CLUSTER_HEIGHT) {
            r.cluster_height = v;
        }
        if let Some(v) = props::f32s(h, prop::TABLE) {
            for (i, x) in v.iter().take(256).enumerate() {
                r.table[i] = table_byte(*x);
            }
        }
        if let Some(v) = props::u32_of(h, prop::UNNAMED) {
            r.unnamed = v;
        }
        if let Some(b) = props::bool_of(h, prop::NEVER_AUTO) {
            r.flags = (r.flags & !flag::NEVER_AUTO) | u8::from(b);
        }
        if let Some(b) = props::bool_of(h, prop::FLAG_SWITCH) {
            r.flags &= if b { !flag::BIT2 } else { !flag::SKIP_SCORE };
        }
        if let Some(v) = props::u32s(h, prop::FAMILIES) {
            r.families = v;
        }
        if let Some(v) = props::i64_of(h, prop::PLOP_COST) {
            r.plop_cost = v;
        }
        r
    }

    pub fn has_flag(&self, bit: u8) -> bool {
        self.flags & bit != 0
    }
}

/// The process-wide registry (§3.2): the type map (ascending unsigned type
/// ID), the type array (registration order, without key 0) and the family
/// map.
#[derive(Debug)]
pub struct FloraRegistry {
    loaded: bool,
    map: BTreeMap<u32, Rc<FloraRecord>>,
    types: Vec<u32>,
    families: BTreeMap<u32, Vec<u32>>,
    default_flags: u8,
}

impl Default for FloraRegistry {
    fn default() -> Self {
        FloraRegistry {
            loaded: false,
            map: BTreeMap::new(),
            types: Vec::new(),
            families: BTreeMap::new(),
            default_flags: DEFAULT_RECORD_FLAGS,
        }
    }
}

/// The registry shared by every simulator of the process.
pub type SharedRegistry = Rc<RefCell<FloraRegistry>>;

impl FloraRegistry {
    pub fn new() -> FloraRegistry {
        FloraRegistry::default()
    }

    /// A fresh process-wide registry.
    pub fn shared() -> SharedRegistry {
        Rc::new(RefCell::new(FloraRegistry::new()))
    }

    /// Whether the first `Init` of the process ran (§4.1 step 2).
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn set_loaded(&mut self, loaded: bool) {
        self.loaded = loaded;
    }

    /// The flags byte given to the key-0 default record ([choice]).
    pub fn set_default_record_flags(&mut self, flags: u8) {
        self.default_flags = flags;
    }

    pub fn default_record_flags(&self) -> u8 {
        self.default_flags
    }

    /// `AddStaticFloraData(record)` (slot 6): false when the type is already
    /// registered; otherwise insert, append to the type array, add to each
    /// family (creating it) and compute the seed types (§6.11).
    pub fn add(&mut self, record: Rc<FloraRecord>) -> bool {
        let t = record.type_id;
        if self.map.contains_key(&t) {
            return false;
        }
        self.map.insert(t, record.clone());
        self.types.push(t);
        for &f in &record.families {
            self.families.entry(f).or_default().push(t);
        }
        record.seed_type.set(t);
        for other in self.map.values() {
            if other.seed_type.get() == record.next_type {
                other.seed_type.set(t);
            }
        }
        true
    }

    /// Inserts the key-0 default record (§3.2, `Init` step 8), into the map
    /// only; an existing key 0 is kept.
    pub fn insert_default(&mut self) {
        if !self.map.contains_key(&0) {
            let mut r = FloraRecord::new(0);
            r.flags = self.default_flags;
            self.map.insert(0, Rc::new(r));
        }
    }

    /// `GetStaticFloraData(type)` (slot 7).
    pub fn get(&self, t: u32) -> Option<Rc<FloraRecord>> {
        self.map.get(&t).cloned()
    }

    /// `IsFloraTypeValid(type)` (slot 11).
    pub fn contains(&self, t: u32) -> bool {
        self.map.contains_key(&t)
    }

    /// The map's keys, ascending (including 0 when present).
    pub fn keys(&self) -> Vec<u32> {
        self.map.keys().copied().collect()
    }

    /// The map's records, ascending by type.
    pub fn records(&self) -> Vec<Rc<FloraRecord>> {
        self.map.values().cloned().collect()
    }

    pub fn map_len(&self) -> usize {
        self.map.len()
    }

    /// The type array (registration order, without key 0).
    pub fn types(&self) -> &[u32] {
        &self.types
    }

    /// The family IDs, ascending.
    pub fn family_ids(&self) -> Vec<u32> {
        self.families.keys().copied().collect()
    }

    /// A family's types, `None` for an unknown family.
    pub fn family(&self, id: u32) -> Option<&[u32]> {
        self.families.get(&id).map(Vec::as_slice)
    }

    /// The exemplar-changed reload (§5): empties the type array, the map and
    /// the family map.
    pub fn clear(&mut self) {
        self.types.clear();
        self.map.clear();
        self.families.clear();
    }

    /// The application-shutdown callback (§3.2): empties the map and the type
    /// array, not the family map.
    pub fn app_shutdown(&mut self) {
        self.map.clear();
        self.types.clear();
    }

    /// The next cluster type of `t` (0 for an unknown type).
    pub fn next_of(&self, t: u32) -> u32 {
        self.map.get(&t).map_or(0, |r| r.next_type)
    }

    /// Whether `to` is reachable from `from` by next links, `from = to`
    /// included. [choice] The walk stops at type 0, at an unknown type, and
    /// after `map_len + 1` steps.
    fn reaches(&self, from: u32, to: u32) -> bool {
        let mut cur = from;
        for _ in 0..=self.map.len() {
            if cur == to {
                return cur != 0;
            }
            if cur == 0 {
                return false;
            }
            match self.map.get(&cur) {
                Some(r) => cur = r.next_type,
                None => return false,
            }
        }
        false
    }
}

/// `AdvanceClusterType(a, b)` (§6.11): `next(b)` if `b` is reachable from
/// `a`, else `next(a)` if `a` is reachable from `b`, else 0.
pub fn advance_cluster_type(reg: &FloraRegistry, a: u32, b: u32) -> u32 {
    if reg.reaches(a, b) {
        reg.next_of(b)
    } else if reg.reaches(b, a) {
        reg.next_of(a)
    } else {
        0
    }
}
