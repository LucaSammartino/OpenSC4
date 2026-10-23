// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! A map from cell keys to values that iterates in insertion order (an
//! overwritten key keeps its place). It stands in for the original's hash
//! maps so that a record read and written again keeps its order [choice].

use std::collections::BTreeMap;

/// SGI hash_set<u16> processing order (network-manager §8.5, NM13).
/// Empty buckets are implicit; erase never shrinks the bucket count.
#[derive(Clone, Debug)]
pub struct PendingCells {
    buckets: BTreeMap<usize, Vec<u16>>,
    count: usize,
    bucket_count: usize,
}

impl Default for PendingCells {
    fn default() -> Self {
        Self {
            buckets: BTreeMap::new(),
            count: 0,
            bucket_count: 193,
        }
    }
}

impl PendingCells {
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn bucket_count(&self) -> usize {
        self.bucket_count
    }
    pub fn iter(&self) -> impl Iterator<Item = &u16> {
        self.buckets.values().flatten()
    }

    pub fn insert(&mut self, key: u16) {
        // Resize is requested even for a duplicate key. u16 keys cannot
        // require a prime beyond 98317.
        if self.count + 1 > self.bucket_count {
            let next = [389, 769, 1543, 3079, 6151, 12289, 24593, 49157, 98317]
                .into_iter()
                .find(|&n| n > self.count)
                .unwrap();
            let old = std::mem::take(&mut self.buckets);
            self.bucket_count = next;
            for key in old.into_values().flatten() {
                self.buckets.entry(usize::from(key) % next).or_default().insert(0, key);
            }
        }
        let bucket = self.buckets.entry(usize::from(key) % self.bucket_count).or_default();
        if !bucket.contains(&key) {
            bucket.insert(0, key);
            self.count += 1;
        }
    }

    pub fn remove(&mut self, key: &u16) {
        let b = usize::from(*key) % self.bucket_count;
        if let Some(bucket) = self.buckets.get_mut(&b) {
            if let Some(i) = bucket.iter().position(|k| k == key) {
                bucket.remove(i);
                self.count -= 1;
            }
            if bucket.is_empty() {
                self.buckets.remove(&b);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct OrderedMap<K: Ord + Copy, V> {
    slots: Vec<Option<(K, V)>>,
    index: BTreeMap<K, usize>,
}

impl<K: Ord + Copy, V> Default for OrderedMap<K, V> {
    fn default() -> Self {
        OrderedMap {
            slots: Vec::new(),
            index: BTreeMap::new(),
        }
    }
}

impl<K: Ord + Copy, V> OrderedMap<K, V> {
    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        let i = *self.index.get(k)?;
        self.slots[i].as_ref().map(|(_, v)| v)
    }

    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        let i = *self.index.get(k)?;
        self.slots[i].as_mut().map(|(_, v)| v)
    }

    pub fn contains_key(&self, k: &K) -> bool {
        self.index.contains_key(k)
    }

    /// Inserts or overwrites; returns the old value.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        if let Some(&i) = self.index.get(&k) {
            let old = self.slots[i].replace((k, v));
            return old.map(|(_, v)| v);
        }
        self.index.insert(k, self.slots.len());
        self.slots.push(Some((k, v)));
        None
    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        let i = self.index.remove(k)?;
        let old = self.slots[i].take().map(|(_, v)| v);
        if self.slots.len() > 64 && self.index.len() * 2 < self.slots.len() {
            self.compact();
        }
        old
    }

    fn compact(&mut self) {
        let slots = std::mem::take(&mut self.slots);
        self.index.clear();
        for (k, v) in slots.into_iter().flatten() {
            self.index.insert(k, self.slots.len());
            self.slots.push(Some((k, v)));
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.index.clear();
    }

    /// Entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.slots.iter().flatten().map(|(k, v)| (k, v))
    }

    pub fn keys(&self) -> Vec<K> {
        self.iter().map(|(k, _)| *k).collect()
    }

    /// Takes every entry out, in order.
    pub fn drain(&mut self) -> Vec<(K, V)> {
        self.index.clear();
        std::mem::take(&mut self.slots).into_iter().flatten().collect()
    }
}
