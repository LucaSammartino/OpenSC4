// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The STLport containers and algorithms whose iteration orders the game's
//! simulators depend on: `std::sort` (growth developer §5.4, tract
//! developer §4.14) and the `hash_map<uint32, V>` / `hash_set<uint32>` with
//! the identity hash (building development §4.7.1, tract developer §4.16).
//!
//! Both are the SGI STL algorithms as published in the SGI / STLport
//! sources (general literature). They are written here without any index
//! that can go out of bounds: the "unguarded" loops of the SGI code stop at
//! the slice ends, which changes nothing for a strict weak ordering and
//! keeps a comparator that is not one (NaN scores compare false both ways,
//! a hostile host) from reading outside the slice *[choice]*.

/// The SGI / STLport prime table of hash-table bucket counts
/// (`__stl_prime_list`).
pub const STL_PRIMES: [u32; 28] = [
    53, 97, 193, 389, 769, 1543, 3079, 6151, 12289, 24593, 49157, 98317, 196613, 393241, 786433, 1572869, 3145739,
    6291469, 12582917, 25165843, 50331653, 100663319, 201326611, 402653189, 805306457, 1610612741, 3221225473,
    4294967291,
];

/// The size hint of a default-constructed `hash_map` / `hash_set`
/// (100 → 193 buckets).
pub const STL_DEFAULT_HINT: usize = 100;

/// The first prime of the table not below `n` (the last one when `n` is
/// beyond the table).
pub fn next_prime(n: usize) -> u32 {
    STL_PRIMES
        .iter()
        .copied()
        .find(|&p| p as usize >= n)
        .unwrap_or(STL_PRIMES[STL_PRIMES.len() - 1])
}

// ---- std::sort ----------------------------------------------------------------------------------

/// `__stl_threshold`: the insertion-sort threshold of `std::sort`.
const THRESHOLD: usize = 16;

/// `std::sort(first, last, less)`: introsort with depth limit
/// `2·⌊log₂ n⌋`; pivot the median of the first, middle and last − 1
/// elements (copied by value); Hoare ("unguarded") partition; heap sort
/// past the limit (`partial_sort(first, last, last)` = `make_heap` +
/// `sort_heap`); then the final insertion sort (guarded on the first 16
/// elements, unguarded after).
///
/// The order of equal elements is exactly the one this algorithm produces
/// ([compat]). `less` should be a strict weak ordering; any other
/// comparator still leaves a permutation of the input.
pub fn stl_sort<T: Copy>(a: &mut [T], less: &mut dyn FnMut(&T, &T) -> bool) {
    let n = a.len();
    if n < 2 {
        return;
    }
    introsort_loop(a, 0, n, 2 * lg(n), less);
    final_insertion_sort(a, less);
}

/// `__lg(n)`: ⌊log₂ n⌋ for n ≥ 1.
fn lg(mut n: usize) -> usize {
    let mut k = 0;
    while n > 1 {
        n >>= 1;
        k += 1;
    }
    k
}

/// `__median(a, b, c, less)`.
fn median<T: Copy>(a: T, b: T, c: T, less: &mut dyn FnMut(&T, &T) -> bool) -> T {
    if less(&a, &b) {
        if less(&b, &c) {
            b
        } else if less(&a, &c) {
            c
        } else {
            a
        }
    } else if less(&a, &c) {
        a
    } else if less(&b, &c) {
        c
    } else {
        b
    }
}

fn introsort_loop<T: Copy>(
    a: &mut [T],
    first: usize,
    mut last: usize,
    mut depth: usize,
    less: &mut dyn FnMut(&T, &T) -> bool,
) {
    while last - first > THRESHOLD {
        if depth == 0 {
            heap_sort(&mut a[first..last], less);
            return;
        }
        depth -= 1;
        let pivot = median(a[first], a[first + (last - first) / 2], a[last - 1], less);
        let cut = unguarded_partition(a, first, last, pivot, less);
        introsort_loop(a, cut, last, depth, less);
        last = cut;
    }
}

/// `__unguarded_partition(first, last, pivot)`.
fn unguarded_partition<T: Copy>(
    a: &mut [T],
    mut first: usize,
    mut last: usize,
    pivot: T,
    less: &mut dyn FnMut(&T, &T) -> bool,
) -> usize {
    let lo = first;
    let hi = last;
    loop {
        while first < hi && less(&a[first], &pivot) {
            first += 1;
        }
        if last == lo {
            return first.min(hi);
        }
        last -= 1;
        while last > lo && less(&pivot, &a[last]) {
            last -= 1;
        }
        if first >= last {
            // A partition point that leaves both sides non-empty keeps the
            // recursion finite for any comparator; for an irreflexive one
            // the SGI cut is already inside this range.
            return first.clamp(lo + 1, hi - 1);
        }
        a.swap(first, last);
        first += 1;
    }
}

fn final_insertion_sort<T: Copy>(a: &mut [T], less: &mut dyn FnMut(&T, &T) -> bool) {
    let n = a.len();
    if n > THRESHOLD {
        insertion_sort(&mut a[..THRESHOLD], less);
        for i in THRESHOLD..n {
            let v = a[i];
            unguarded_linear_insert(a, i, v, less);
        }
    } else {
        insertion_sort(a, less);
    }
}

/// `__insertion_sort` with `__linear_insert`.
fn insertion_sort<T: Copy>(a: &mut [T], less: &mut dyn FnMut(&T, &T) -> bool) {
    for i in 1..a.len() {
        let v = a[i];
        if less(&v, &a[0]) {
            a.copy_within(0..i, 1);
            a[0] = v;
        } else {
            unguarded_linear_insert(a, i, v, less);
        }
    }
}

/// `__unguarded_linear_insert(last, value)`.
fn unguarded_linear_insert<T: Copy>(a: &mut [T], mut last: usize, v: T, less: &mut dyn FnMut(&T, &T) -> bool) {
    while last > 0 && less(&v, &a[last - 1]) {
        a[last] = a[last - 1];
        last -= 1;
    }
    a[last] = v;
}

/// `partial_sort(first, last, last)`: `make_heap` then `sort_heap`.
fn heap_sort<T: Copy>(a: &mut [T], less: &mut dyn FnMut(&T, &T) -> bool) {
    make_heap(a, less);
    let mut len = a.len();
    while len > 1 {
        // `pop_heap(first, end)`: the last element is re-inserted from the top.
        let v = a[len - 1];
        a[len - 1] = a[0];
        len -= 1;
        adjust_heap(&mut a[..len], 0, v, less);
    }
}

fn make_heap<T: Copy>(a: &mut [T], less: &mut dyn FnMut(&T, &T) -> bool) {
    let len = a.len();
    if len < 2 {
        return;
    }
    let mut parent = (len - 2) / 2;
    loop {
        let v = a[parent];
        adjust_heap(a, parent, v, less);
        if parent == 0 {
            return;
        }
        parent -= 1;
    }
}

/// `__adjust_heap(first, hole, len, value)` with `__push_heap`.
fn adjust_heap<T: Copy>(a: &mut [T], mut hole: usize, v: T, less: &mut dyn FnMut(&T, &T) -> bool) {
    let len = a.len();
    let top = hole;
    let mut second = 2 * hole + 2;
    while second < len {
        if less(&a[second], &a[second - 1]) {
            second -= 1;
        }
        a[hole] = a[second];
        hole = second;
        second = 2 * (second + 1);
    }
    if second == len {
        a[hole] = a[second - 1];
        hole = second - 1;
    }
    // `__push_heap(first, hole, top, value)`.
    while hole > top {
        let parent = (hole - 1) / 2;
        if !less(&a[parent], &v) {
            break;
        }
        a[hole] = a[parent];
        hole = parent;
    }
    a[hole] = v;
}

// ---- hash_map / hash_set ------------------------------------------------------------------------

/// A model of the STLport (SGI) `hash_map<uint32, V>` that keeps its
/// iteration order (building development §4.7.1) *[inferred: SGI/STLport 4
/// growth policy]*: identity hash, bucket = key mod bucket count; the
/// constructor's hint rounded up by [`next_prime`] (193 buckets by
/// default); before every insertion attempt the table grows to the next
/// listed prime ≥ size + 1 when size + 1 exceeds the bucket count, moving
/// each old bucket's nodes (head first) to the head of their new bucket; a
/// new node goes to the head of its bucket. Iteration: buckets in ascending
/// order, each from its head.
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
    /// A default-constructed map ([`STL_DEFAULT_HINT`]).
    pub fn new() -> StlHashMap<V> {
        StlHashMap::with_hint(STL_DEFAULT_HINT)
    }

    /// A map constructed with a bucket hint (`n` rounded up to a prime of
    /// the table).
    pub fn with_hint(n: usize) -> StlHashMap<V> {
        let n = next_prime(n) as usize;
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
    fn resize(&mut self, hint: usize) {
        let old_n = self.buckets.len();
        if hint <= old_n {
            return;
        }
        let n = next_prime(hint) as usize;
        if n <= old_n {
            return;
        }
        let mut fresh: Vec<Vec<(u32, V)>> = Vec::with_capacity(n);
        fresh.resize_with(n, Vec::new);
        for bucket in std::mem::take(&mut self.buckets) {
            // Old head first; each moves to the head of its new bucket.
            for (k, v) in bucket.into_iter().rev() {
                fresh[(u64::from(k) % n as u64) as usize].push((k, v));
            }
        }
        self.buckets = fresh;
    }

    /// `resize(n)` as a public call: grows the table to the listed prime
    /// ≥ `n` when `n` exceeds the bucket count (the size a table reaches by
    /// inserting `n` keys one by one). Readers that rebuild a saved
    /// iteration order use it so that no growth happens while they insert.
    pub fn reserve(&mut self, n: usize) {
        self.resize(n);
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

    /// `erase(key)`: removes the node holding `k` from its bucket chain,
    /// keeping the order of the remaining nodes. Returns the removed value.
    /// The bucket array is not shrunk (SGI `hash_map::erase` does not
    /// resize), so the iteration order of the survivors is unchanged.
    pub fn remove(&mut self, k: u32) -> Option<V> {
        let b = self.bucket_of(k);
        let bucket = self.buckets.get_mut(b)?;
        let pos = bucket.iter().position(|(key, _)| *key == k)?;
        self.len -= 1;
        Some(bucket.remove(pos).1)
    }

    /// `operator[]` (`find_or_insert`): the value for `k`, inserting
    /// `make()` when absent. The table may grow even when `k` exists.
    pub fn entry(&mut self, k: u32, make: impl FnOnce() -> V) -> &mut V {
        self.entry_inserted(k, make).0
    }

    /// [`entry`](Self::entry), also returning whether `k` was inserted.
    fn entry_inserted(&mut self, k: u32, make: impl FnOnce() -> V) -> (&mut V, bool) {
        self.resize(self.len + 1);
        let b = self.bucket_of(k);
        let pos = self.buckets[b].iter().position(|(key, _)| *key == k);
        let (pos, inserted) = match pos {
            Some(p) => (p, false),
            None => {
                self.buckets[b].push((k, make()));
                self.len += 1;
                (self.buckets[b].len() - 1, true)
            }
        };
        (&mut self.buckets[b][pos].1, inserted)
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

/// An STLport `hash_set<uint32>` with the identity hash: the
/// [`StlHashMap`] order and growth policy without values.
#[derive(Clone, Debug)]
pub struct StlHashSet {
    map: StlHashMap<()>,
}

impl Default for StlHashSet {
    fn default() -> Self {
        StlHashSet::with_hint(STL_DEFAULT_HINT)
    }
}

impl StlHashSet {
    /// A set constructed with a bucket hint (`n` rounded up to a prime of
    /// the table; the tract developer uses 100 → 193).
    pub fn with_hint(n: usize) -> StlHashSet {
        StlHashSet {
            map: StlHashMap::with_hint(n),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn bucket_count(&self) -> usize {
        self.map.bucket_count()
    }

    pub fn contains(&self, v: u32) -> bool {
        self.map.contains(v)
    }

    /// `insert`: resize for `len + 1`, then insert at the head of the
    /// chain unless present. Returns whether it was inserted.
    pub fn insert(&mut self, v: u32) -> bool {
        self.map.entry_inserted(v, || ()).1
    }

    /// [`StlHashMap::reserve`].
    pub fn reserve(&mut self, n: usize) {
        self.map.reserve(n);
    }

    /// The elements in iteration order.
    pub fn iter_order(&self) -> Vec<u32> {
        self.map.keys()
    }

    /// `erase(value)`: removes `v` from its chain, keeping the order of the
    /// others (the bucket array is not shrunk). Returns whether it was
    /// present.
    pub fn remove(&mut self, v: u32) -> bool {
        self.map.remove(v).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small deterministic generator for test data.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 33) as u32
        }
        fn below(&mut self, n: u32) -> u32 {
            if n == 0 {
                0
            } else {
                self.next() % n
            }
        }
    }

    #[test]
    fn sort_keeps_small_ties_in_insertion_order() {
        let mut v: Vec<(i32, usize)> = [5, 3, 5, 1, 3, 5].iter().copied().zip(0..).collect();
        stl_sort(&mut v, &mut |a, b| a.0 > b.0);
        assert_eq!(v, vec![(5, 0), (5, 2), (5, 5), (3, 1), (3, 4), (1, 3)]);
    }

    #[test]
    fn sort_sorts_and_permutes() {
        let mut g = Lcg(7);
        for len in [0usize, 1, 2, 3, 15, 16, 17, 31, 64, 200, 1000, 2000] {
            for range in [1u32, 3, 50, 1_000_000] {
                let orig: Vec<u32> = (0..len).map(|_| g.below(range)).collect();
                let mut a = orig.clone();
                stl_sort(&mut a, &mut |x: &u32, y: &u32| x < y);
                let mut want = orig.clone();
                want.sort_unstable();
                assert_eq!(a, want, "len {len} range {range}");
            }
        }
        // The heap-sort path (organ pipe defeats median-of-three).
        let mut organ: Vec<i32> = (0..300).chain((0..300).rev()).collect();
        stl_sort(&mut organ, &mut |a, b| a < b);
        assert!(organ.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn sort_survives_inconsistent_comparators() {
        let mut g = Lcg(11);
        for len in [2usize, 5, 16, 17, 40, 100, 300] {
            let mut a: Vec<u32> = (0..len as u32).collect();
            stl_sort(&mut a, &mut |_: &u32, _: &u32| true);
            let mut b: Vec<u32> = (0..len as u32).rev().collect();
            stl_sort(&mut b, &mut |_: &u32, _: &u32| g.next() & 1 == 0);
            let mut c: Vec<u32> = (0..len as u32).collect();
            stl_sort(&mut c, &mut |_: &u32, _: &u32| false);
            // The contents are a permutation whatever the comparator says.
            for v in [a, b, c] {
                let mut s = v.clone();
                s.sort_unstable();
                assert_eq!(s, (0..len as u32).collect::<Vec<_>>());
            }
        }
        // NaN scores: unordered pairs compare false.
        let mut f = [2.0f64, f64::NAN, 1.0, f64::NAN, 0.5];
        stl_sort(&mut f, &mut |x: &f64, y: &f64| x < y);
        assert_eq!(f.iter().filter(|v| v.is_nan()).count(), 2);
    }

    #[test]
    fn next_prime_walks_the_table() {
        assert_eq!(next_prime(0), 53);
        assert_eq!(next_prime(100), 193);
        assert_eq!(next_prime(193), 193);
        assert_eq!(next_prime(194), 389);
        assert!(next_prime(usize::MAX) >= 193);
    }

    #[test]
    fn hash_map_order_is_bucket_order_with_head_insertion() {
        let mut m = StlHashMap::<u32>::new();
        assert_eq!(m.bucket_count(), 193);
        for k in [5u32, 198, 1, 400] {
            *m.entry(k, || 0) = k;
        }
        assert_eq!(m.keys(), vec![1, 198, 5, 400]);
        assert_eq!(m.len(), 4);
        assert_eq!(m.get(198), Some(&198));
        assert!(!m.contains(2));
        // An existing key is not re-inserted.
        *m.entry(5, || 99) += 1;
        assert_eq!(m.get(5), Some(&6));
        assert_eq!(m.keys(), vec![1, 198, 5, 400]);
    }

    #[test]
    fn hash_map_resizes_before_the_insert_that_would_overflow() {
        let mut m = StlHashMap::<()>::new();
        m.entry(400, || ());
        for k in 1..=192 {
            m.entry(k, || ());
        }
        assert_eq!(m.bucket_count(), 193);
        let keys = m.keys();
        assert_eq!(&keys[12..16], &[13, 14, 400, 15]);
        m.entry(193, || ());
        assert_eq!(m.bucket_count(), 389);
        let keys = m.keys();
        assert_eq!(&keys[8..13], &[9, 10, 400, 11, 12]);
        assert_eq!(keys.len(), 194);
    }

    #[test]
    fn hash_set_bucket_order_and_growth() {
        let mut s = StlHashSet::with_hint(100);
        assert_eq!(s.bucket_count(), 193);
        assert!(s.insert(5));
        assert!(s.insert(198));
        assert!(!s.insert(5));
        assert!(s.contains(198));
        assert_eq!(s.iter_order(), vec![198, 5]);
        // Tract developer test 14's order.
        let mut s = StlHashSet::with_hint(100);
        for v in [0x100, 0x101, 0x102, 0x2C1, 0x3] {
            s.insert(v);
        }
        assert_eq!(s.iter_order(), vec![0x3, 0x100, 0x101, 0x102, 0x2C1]);
        // Growth past 193 elements resizes to the next prime and keeps every
        // element once.
        let mut s = StlHashSet::with_hint(100);
        for v in 0..500u32 {
            s.insert(v.wrapping_mul(2_654_435_761));
        }
        assert_eq!(s.len(), 500);
        assert!(s.bucket_count() >= 500);
        assert!(STL_PRIMES.contains(&(s.bucket_count() as u32)));
        let mut o = s.iter_order();
        o.sort_unstable();
        o.dedup();
        assert_eq!(o.len(), 500);
    }
}
