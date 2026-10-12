// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The iteration orders the tract developer depends on (§9.5): the game's
//! STLport `std::sort` (§4.14) and the STLport `hash_set<uint32>` with the
//! identity hash (§4.16).
//!
//! Both are the SGI STL algorithms (general literature). They are written
//! here without any index that can go out of bounds: the "unguarded" loops
//! of the SGI code stop at the slice ends, which changes nothing for a
//! strict weak ordering and keeps a comparator that is not one (NaN scores
//! compare false both ways) from reading outside the slice *[choice]*.

/// The SGI / STLport prime table of `hash_set` bucket counts.
pub const STL_PRIMES: [u32; 28] = [
    53, 97, 193, 389, 769, 1543, 3079, 6151, 12289, 24593, 49157, 98317, 196613, 393241, 786433, 1572869, 3145739,
    6291469, 12582917, 25165843, 50331653, 100663319, 201326611, 402653189, 805306457, 1610612741, 3221225473,
    4294967291,
];

/// The insertion-sort threshold of `std::sort`.
const THRESHOLD: usize = 16;

/// `std::sort(first, last, less)`: introsort with depth limit
/// `2·⌊log₂ n⌋` (median of first / middle / last − 1, Hoare partition),
/// heap sort past the limit, then the final insertion sort (guarded on the
/// first 16 elements, unguarded after).
pub fn stl_sort<T: Copy>(a: &mut [T], less: &mut dyn FnMut(&T, &T) -> bool) {
    let n = a.len();
    if n < 2 {
        return;
    }
    introsort_loop(a, 0, n, 2 * lg(n), less);
    final_insertion_sort(a, less);
}

fn lg(mut n: usize) -> usize {
    let mut k = 0;
    while n > 1 {
        n >>= 1;
        k += 1;
    }
    k
}

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
            // recursion finite for any comparator.
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

fn unguarded_linear_insert<T: Copy>(a: &mut [T], mut last: usize, v: T, less: &mut dyn FnMut(&T, &T) -> bool) {
    while last > 0 && less(&v, &a[last - 1]) {
        a[last] = a[last - 1];
        last -= 1;
    }
    a[last] = v;
}

fn heap_sort<T: Copy>(a: &mut [T], less: &mut dyn FnMut(&T, &T) -> bool) {
    make_heap(a, less);
    let mut len = a.len();
    while len > 1 {
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
    // push_heap
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

/// An STLport `hash_set<uint32>` with the identity hash: buckets in
/// ascending order of `value mod B`, each chain most recent insertion
/// first; growth to the next prime of [`STL_PRIMES`] walks the old buckets
/// in order and moves each chain's front node to the front of its new
/// bucket (§4.16).
#[derive(Clone, Debug)]
pub struct StlHashSet {
    buckets: Vec<Vec<u32>>,
    len: usize,
}

impl StlHashSet {
    /// A set constructed with a bucket hint (`n` rounded up to a prime of
    /// the table; the tract developer uses 100 → 193).
    pub fn with_hint(n: usize) -> StlHashSet {
        StlHashSet {
            buckets: vec![Vec::new(); next_prime(n) as usize],
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    fn bucket_of(v: u32, n: usize) -> usize {
        (v as usize) % n.max(1)
    }

    /// `insert`: resize for `len + 1`, then insert at the front of the
    /// chain unless present. Returns whether it was inserted.
    pub fn insert(&mut self, v: u32) -> bool {
        self.resize(self.len + 1);
        let b = StlHashSet::bucket_of(v, self.buckets.len());
        let Some(chain) = self.buckets.get_mut(b) else {
            return false;
        };
        if chain.contains(&v) {
            return false;
        }
        // Chains are stored front first.
        chain.insert(0, v);
        self.len += 1;
        true
    }

    fn resize(&mut self, hint: usize) {
        let old = self.buckets.len();
        if hint <= old {
            return;
        }
        let n = next_prime(hint) as usize;
        if n <= old {
            return;
        }
        let mut tmp: Vec<Vec<u32>> = vec![Vec::new(); n];
        for chain in self.buckets.iter_mut() {
            for v in chain.drain(..) {
                let b = StlHashSet::bucket_of(v, n);
                if let Some(c) = tmp.get_mut(b) {
                    c.insert(0, v);
                }
            }
        }
        self.buckets = tmp;
    }

    /// The elements in iteration order.
    pub fn iter_order(&self) -> Vec<u32> {
        self.buckets.iter().flatten().copied().collect()
    }
}

/// The first prime of the table not below `n` (the last one when `n` is
/// beyond the table).
pub fn next_prime(n: usize) -> u32 {
    STL_PRIMES
        .iter()
        .copied()
        .find(|&p| p as usize >= n)
        .unwrap_or(STL_PRIMES[STL_PRIMES.len() - 1])
}
