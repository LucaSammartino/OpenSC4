// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The process-wide building catalogue (§4.7): building types, families,
//! names, profiles, foundations, retaining walls and the maximum density
//! table, with the hash order of the original's STLport `hash_map`s.

use std::collections::{BTreeMap, BTreeSet};

use crate::development::{building_map_key, developer_type};
use crate::property::PropertyHolder;
use crate::props;
use crate::stream::ResKey;
use crate::x87::ftol32;

use super::prop;

/// The bucket counts of the STLport / SGI hash table (`__stl_prime_list`).
pub const STL_PRIMES: [u64; 28] = [
    53, 97, 193, 389, 769, 1543, 3079, 6151, 12289, 24593, 49157, 98317, 196613, 393241, 786433, 1572869, 3145739,
    6291469, 12582917, 25165843, 50331653, 100663319, 201326611, 402653189, 805306457, 1610612741, 3221225473,
    4294967291,
];

/// The size hint of a default-constructed `hash_map` (100 → 193 buckets).
pub const STL_DEFAULT_HINT: u64 = 100;

/// The smallest listed prime ≥ `n` (the last one if none is).
fn next_size(n: u64) -> u64 {
    STL_PRIMES
        .iter()
        .copied()
        .find(|&p| p >= n)
        .unwrap_or(STL_PRIMES[STL_PRIMES.len() - 1])
}

/// A model of the STLport (SGI) `hash_map<uint32, V>` that keeps its
/// iteration order (§4.7.1) *[inferred: SGI/STLport 4 growth policy]*:
/// identity hash, bucket = key mod bucket count; 193 buckets at
/// construction; before every insertion attempt the table grows to the
/// next listed prime ≥ size + 1 when size + 1 exceeds the bucket count,
/// moving each old bucket's nodes (head first) to the head of their new
/// bucket; a new node goes to the head of its bucket. Iteration: buckets
/// in ascending order, each from its head.
#[derive(Clone, Debug)]
pub struct StlHashMap<V> {
    /// Each bucket stores its nodes tail first (the head is the last
    /// element), so a head insertion is a push.
    buckets: Vec<Vec<(u32, V)>>,
    len: usize,
}

impl<V> Default for StlHashMap<V> {
    fn default() -> Self {
        StlHashMap::new()
    }
}

impl<V> StlHashMap<V> {
    pub fn new() -> StlHashMap<V> {
        let n = next_size(STL_DEFAULT_HINT) as usize;
        let mut buckets = Vec::with_capacity(n);
        buckets.resize_with(n, Vec::new);
        StlHashMap { buckets, len: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The current bucket count.
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    fn bucket_of(&self, k: u32) -> usize {
        (u64::from(k) % self.buckets.len().max(1) as u64) as usize
    }

    /// `resize(hint)`.
    fn resize(&mut self, hint: u64) {
        let old_n = self.buckets.len() as u64;
        if hint <= old_n {
            return;
        }
        let n = next_size(hint);
        if n <= old_n {
            return;
        }
        let mut fresh: Vec<Vec<(u32, V)>> = Vec::with_capacity(n as usize);
        fresh.resize_with(n as usize, Vec::new);
        for bucket in std::mem::take(&mut self.buckets) {
            // Old head first; each moves to the head of its new bucket.
            for (k, v) in bucket.into_iter().rev() {
                fresh[(u64::from(k) % n) as usize].push((k, v));
            }
        }
        self.buckets = fresh;
    }

    pub fn get(&self, k: u32) -> Option<&V> {
        self.buckets
            .get(self.bucket_of(k))
            .and_then(|b| b.iter().find(|(key, _)| *key == k))
            .map(|(_, v)| v)
    }

    pub fn get_mut(&mut self, k: u32) -> Option<&mut V> {
        let b = self.bucket_of(k);
        self.buckets
            .get_mut(b)
            .and_then(|b| b.iter_mut().find(|(key, _)| *key == k))
            .map(|(_, v)| v)
    }

    pub fn contains(&self, k: u32) -> bool {
        self.get(k).is_some()
    }

    /// `operator[]` (`find_or_insert`): the value for `k`, inserting
    /// `make()` when absent. The table may grow even when `k` exists.
    pub fn entry(&mut self, k: u32, make: impl FnOnce() -> V) -> &mut V {
        self.resize(self.len as u64 + 1);
        let b = self.bucket_of(k);
        let pos = self.buckets[b].iter().position(|(key, _)| *key == k);
        let pos = match pos {
            Some(p) => p,
            None => {
                self.buckets[b].push((k, make()));
                self.len += 1;
                self.buckets[b].len() - 1
            }
        };
        &mut self.buckets[b][pos].1
    }

    /// Keys in iteration order.
    pub fn keys(&self) -> Vec<u32> {
        self.iter().map(|(k, _)| k).collect()
    }

    /// `(key, value)` in iteration order.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &V)> + '_ {
        self.buckets.iter().flat_map(|b| b.iter().rev().map(|(k, v)| (*k, v)))
    }
}

/// A building's profile (`BuildingProfile`, §4.7.1). The fields after
/// `capacity` are not used by growth *[inferred: field mapping]*.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildingProfile {
    /// 0–3 (values outside become 0).
    pub wealth: u8,
    pub purpose: u8,
    /// Capacities satisfied: developer type → amount (property `0x27812834`).
    pub capacities: BTreeMap<u32, u16>,
    /// The amount for `GetDeveloperType(purpose, wealth)`, 0 if absent.
    pub capacity: u16,
    /// Property `0x27812836`.
    pub flag_27812836: bool,
    /// Property `0x499AFA38` (default 30 when loaded from an exemplar).
    pub value_499afa38: u8,
    /// The values of property `0xAA1DD396`, kept raw: the 31-entry table
    /// that turns them into category flags is not in the spec (§4.7.1).
    pub category_values: Vec<u32>,
    /// Occupant size (property `0x27812810`), metres.
    pub occupant_size: [f32; 3],
    /// Plop cost (property `0x49CAC341`).
    pub plop_cost: i64,
}

impl BuildingProfile {
    /// The profile `LoadBuildingProperties` fills from a building exemplar
    /// (§4.7.1).
    pub fn from_exemplar(h: &PropertyHolder) -> BuildingProfile {
        let raw_wealth = props::u32_of(h, prop::WEALTH).unwrap_or(0) as u8;
        let wealth = if raw_wealth <= 3 { raw_wealth } else { 0 };
        let purpose = props::u32_of(h, prop::PURPOSE).unwrap_or(0) as u8;
        let mut capacities = BTreeMap::new();
        for (t, amount) in props::u32_pairs(h, prop::CAPACITY_SATISFIED) {
            capacities.insert(t, amount as u16);
        }
        let capacity = capacities
            .get(&developer_type(u32::from(purpose), u32::from(wealth)))
            .copied()
            .unwrap_or(0);
        let mut occupant_size = [0.0f32; 3];
        if let Some(v) = props::f32s(h, prop::OCCUPANT_SIZE) {
            for (slot, x) in occupant_size.iter_mut().zip(v) {
                *slot = x;
            }
        }
        BuildingProfile {
            wealth,
            purpose,
            capacities,
            capacity,
            flag_27812836: props::bool_of(h, prop::FLAG_27812836).unwrap_or(false),
            value_499afa38: props::u32_of(h, prop::VALUE_499AFA38).map_or(30, |v| v as u8),
            category_values: props::u32s(h, prop::CATEGORIES).unwrap_or_default(),
            occupant_size,
            plop_cost: props::i64_of(h, prop::PLOP_COST).unwrap_or(0),
        }
    }

    /// `GetBuildingMapKey(type)` of this profile: `uint8(4p + w + 1)`.
    pub fn map_key(&self) -> u8 {
        building_map_key(u32::from(self.purpose), u32::from(self.wealth))
    }
}

/// The primary building configuration object of a lot configuration
/// (§4.7.4): its building or family IDs, orientation and position in the
/// lot (metres).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildingConfigObject {
    pub ids: Vec<u32>,
    pub orientation: u32,
    pub x: f32,
    pub z: f32,
}

/// What the maximum density table reads from one lot configuration
/// (§4.7.4).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LotConfigInfo {
    /// `GetSize()`: width and depth in cells.
    pub size: (u32, u32),
    /// Bit `z` set ⇔ `IsCompatibleWithZoneType(z)`, z in 0..16.
    pub zones: u16,
    /// `GetPrimaryBuildingConfigurationObject()`; [choice] a configuration
    /// without one adds nothing.
    pub primary: Option<BuildingConfigObject>,
}

/// One 24-bit rounding (foundations §18).
fn r(x: f64) -> f32 {
    x as f32
}

/// The building catalogue (§4.7).
#[derive(Clone, Debug, Default)]
pub struct BuildingCatalogue {
    /// Whether the static initialisation has run (§1.3 step 1).
    loaded: bool,
    /// Building type → exemplar key.
    keys: StlHashMap<ResKey>,
    profiles: StlHashMap<BuildingProfile>,
    exemplar_names: StlHashMap<Vec<u8>>,
    building_names: StlHashMap<Vec<u8>>,
    /// Family → member building types (appended in load order).
    families: StlHashMap<Vec<u32>>,
    foundations: StlHashMap<ResKey>,
    walls: StlHashMap<ResKey>,
    /// `(zone, map key)` → density (§4.7.4).
    density: BTreeMap<(u32, u8), f32>,
}

impl BuildingCatalogue {
    pub fn new() -> BuildingCatalogue {
        BuildingCatalogue::default()
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn set_loaded(&mut self, loaded: bool) {
        self.loaded = loaded;
    }

    /// `AddBuildingData(holder)` (§4.7.1): the building type is the
    /// exemplar's instance. `localized_name` is `SC4LoadExemplarString` of
    /// property `0x8A416A99`.
    pub fn add_building(&mut self, key: ResKey, h: &PropertyHolder, localized_name: Option<Vec<u8>>) {
        let t = key.instance;
        *self.keys.entry(t, ResKey::default) = key;
        let name = props::string_of(h, prop::EXEMPLAR_NAME).unwrap_or_default();
        *self.exemplar_names.entry(t, Vec::new) = name;
        let bname = localized_name
            .or_else(|| props::string_of(h, prop::BUILDING_NAME))
            .unwrap_or_default();
        *self.building_names.entry(t, Vec::new) = bname;
        *self.profiles.entry(t, BuildingProfile::default) = BuildingProfile::from_exemplar(h);
        for f in props::u32s(h, prop::FAMILIES).unwrap_or_default() {
            self.families.entry(f, Vec::new).push(t);
        }
    }

    /// A foundation exemplar (ExemplarType `0x11`) *[inferred: type =
    /// instance, stored as type → key]*.
    pub fn add_foundation(&mut self, key: ResKey) {
        *self.foundations.entry(key.instance, ResKey::default) = key;
    }

    /// A lot retaining wall exemplar (ExemplarType `0x15`) *[inferred]*.
    pub fn add_retaining_wall(&mut self, key: ResKey) {
        *self.walls.entry(key.instance, ResKey::default) = key;
    }

    /// The number of building types.
    pub fn building_count(&self) -> usize {
        self.keys.len()
    }

    // ---- queries (§4.7.2) ----

    /// `IsBuildingTypeValid(t)` (15).
    pub fn is_building_type_valid(&self, t: u32) -> bool {
        self.keys.contains(t)
    }

    /// `IsBuildingFamilyTypeValid(f)` (16): a family or a building type.
    pub fn is_building_family_type_valid(&self, f: u32) -> bool {
        self.families.contains(f) || self.keys.contains(f)
    }

    /// `IsBuildingInFamily(b, f)` (20).
    pub fn is_building_in_family(&self, b: u32, f: u32) -> bool {
        b == f || self.families.get(f).is_some_and(|m| m.contains(&b))
    }

    /// The members of family `f`, if it is one.
    pub fn family(&self, f: u32) -> Option<&[u32]> {
        self.families.get(f).map(Vec::as_slice)
    }

    /// `GetBuildingTypesByFamily(vector&, f)` (18): a family assigns its
    /// member list and returns the member count; a building type makes the
    /// vector `[f]` and returns 0 *[quirk]*; anything else leaves it and
    /// returns 0.
    pub fn building_types_by_family(&self, out: &mut Vec<u32>, f: u32) -> u32 {
        if let Some(m) = self.families.get(f) {
            out.clear();
            out.extend_from_slice(m);
            m.len() as u32
        } else if self.keys.contains(f) {
            out.clear();
            out.push(f);
            0
        } else {
            0
        }
    }

    /// `GetBuildingTypesByFamily(hash_set&, f)` (19): inserts (no clear).
    pub fn building_types_by_family_set(&self, out: &mut BTreeSet<u32>, f: u32) -> u32 {
        if let Some(m) = self.families.get(f) {
            out.extend(m.iter().copied());
            m.len() as u32
        } else {
            if self.keys.contains(f) {
                out.insert(f);
            }
            0
        }
    }

    /// `GetBuildingTypesByFilter(hash_set&, filter)` (17): every building
    /// type (hash order) the filter accepts (all without a filter) is
    /// inserted; returns how many passed.
    pub fn building_types_by_filter(&self, out: &mut BTreeSet<u32>, filter: Option<&dyn Fn(u32) -> bool>) -> u32 {
        let mut n = 0u32;
        for t in self.keys.keys() {
            if filter.map_or(true, |f| f(t)) {
                out.insert(t);
                n = n.wrapping_add(1);
            }
        }
        n
    }

    /// `GetBuildingProfile(t)` (39): inserts a zero profile for an unknown
    /// type *[compat]*.
    pub fn profile_entry(&mut self, t: u32) -> &mut BuildingProfile {
        self.profiles.entry(t, BuildingProfile::default)
    }

    /// `GetBuildingProfilePtr(t)` (40).
    pub fn profile(&self, t: u32) -> Option<&BuildingProfile> {
        self.profiles.get(t)
    }

    /// `GetBuildingKeyFromType(t, key&)` (14).
    pub fn building_key(&self, t: u32) -> Option<ResKey> {
        self.keys.get(t).copied()
    }

    /// `GetFoundationKeyFromType` (22).
    pub fn foundation_key(&self, t: u32) -> Option<ResKey> {
        self.foundations.get(t).copied()
    }

    /// `GetLotRetainingWallKeyFromType` (28).
    pub fn retaining_wall_key(&self, t: u32) -> Option<ResKey> {
        self.walls.get(t).copied()
    }

    /// `GetExemplarName(t)` (37): inserts "" for an unknown type.
    pub fn exemplar_name(&mut self, t: u32) -> Vec<u8> {
        self.exemplar_names.entry(t, Vec::new).clone()
    }

    /// `GetBuildingName(t)` (38): inserts "" for an unknown type.
    pub fn building_name(&mut self, t: u32) -> Vec<u8> {
        self.building_names.entry(t, Vec::new).clone()
    }

    /// Building types in hash order (`GetAllBuildingTypes`, 11).
    pub fn all_building_types(&self) -> Vec<u32> {
        self.keys.keys()
    }

    /// Family types in hash order (13).
    pub fn all_family_types(&self) -> Vec<u32> {
        self.families.keys()
    }

    /// Foundation types in hash order (21).
    pub fn all_foundation_types(&self) -> Vec<u32> {
        self.foundations.keys()
    }

    /// Retaining wall types in hash order (27).
    pub fn all_retaining_wall_types(&self) -> Vec<u32> {
        self.walls.keys()
    }

    /// The profile map's types in hash order (they include the zero profiles
    /// inserted for unknown types).
    pub fn profile_types(&self) -> Vec<u32> {
        self.profiles.keys()
    }

    // ---- maximum density table (§4.7.4) ----

    /// `CalculateMaxCapacity` over every lot configuration. `cell_width` is
    /// the city's `CellWidthX`, `CellWidthZ`.
    pub fn build_density_table(&mut self, configs: &[LotConfigInfo], cell_width: (f32, f32)) {
        for c in configs {
            self.add_density_config(c, cell_width);
        }
    }

    /// One configuration of [`Self::build_density_table`].
    pub fn add_density_config(&mut self, c: &LotConfigInfo, cell_width: (f32, f32)) {
        let (w, h) = c.size;
        let area = (w as i32).wrapping_mul(h as i32);
        let size_x = r(f64::from(w) * f64::from(cell_width.0));
        let size_z = r(f64::from(h) * f64::from(cell_width.1));
        let Some(obj) = &c.primary else { return };
        let half = f64::from(0.5f32);
        let eps = f64::from(0.001f32);
        for &id in &obj.ids {
            let types: Vec<u32> = match self.families.get(id) {
                Some(m) => m.clone(),
                None if self.keys.contains(id) => vec![id],
                None => Vec::new(),
            };
            for t in types {
                if self.keys.contains(t) {
                    let size = self.profiles.get(t).map_or([0.0; 3], |p| p.occupant_size);
                    let (mut dx, mut dz) = (size[0], size[2]);
                    if obj.orientation == 1 || obj.orientation == 3 {
                        std::mem::swap(&mut dx, &mut dz);
                    }
                    let hx = r(f64::from(dx) * half);
                    let hz = r(f64::from(dz) * half);
                    let left = r(f64::from(obj.x) - f64::from(hx));
                    let right = r(f64::from(hx) + f64::from(obj.x));
                    let top = r(f64::from(obj.z) - f64::from(hz));
                    let bottom = r(f64::from(hz) + f64::from(obj.z));
                    let fits = r(f64::from(left) + eps) >= 0.0
                        && r(f64::from(right) - eps) <= size_x
                        && r(f64::from(top) + eps) >= 0.0
                        && r(f64::from(bottom) - eps) <= size_z;
                    if !fits {
                        continue;
                    }
                }
                let prof = self.profile_entry(t).clone();
                if prof.purpose == 0 {
                    continue;
                }
                let mut d = r(f64::from(r(f64::from(prof.capacity) / f64::from(area))) + 0.5);
                if d < 1.0 && prof.capacity > 0 {
                    d = 1.0;
                }
                let key = prof.map_key();
                for z in 0..16u32 {
                    if c.zones & (1 << z) != 0 {
                        let e = self.density.entry((z, key)).or_insert(0.0);
                        if *e < d {
                            *e = d;
                        }
                    }
                }
            }
        }
    }

    /// The raw table entry for `(zone, map key)`.
    pub fn density(&self, zone: u32, key: u8) -> Option<f32> {
        self.density.get(&(zone, key)).copied()
    }

    /// Every table entry in key order.
    pub fn density_table(&self) -> &BTreeMap<(u32, u8), f32> {
        &self.density
    }

    /// `GetMaxCapacity(zone, p, w)` (45): `float(trunc(entry))`, 0.0 when
    /// absent.
    pub fn max_capacity(&self, zone: u32, purpose: u32, wealth: u32) -> f32 {
        match self.density(zone, building_map_key(purpose, wealth)) {
            Some(v) => ftol32(f64::from(v)) as f32,
            None => 0.0,
        }
    }

    /// Process exit: the static data cleanup callback (`0x0069B850`).
    pub fn app_shutdown(&mut self) {
        *self = BuildingCatalogue::default();
    }
}
