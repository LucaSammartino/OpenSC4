// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The game's `std::sort` (§5.4): the SGI / STLport introsort as published
//! in the SGI STL sources (general literature). Threshold 16; pivot the
//! median of the first, middle and last−1 elements (copied by value); Hoare
//! ("unguarded") partition; depth limit `2·⌊log₂ n⌋`, then heap sort
//! (`partial_sort(first, last, last)` = `make_heap` + `sort_heap`); a final
//! insertion sort (guarded over the first 16 elements, unguarded after).
//!
//! The order of equal elements is exactly the one this algorithm produces
//! (§5.4 ties, [compat]). `less` must be a strict weak ordering; the
//! "unguarded" loops are bounded by the slice ends here, which changes
//! nothing for such an ordering and keeps a hostile comparator from
//! indexing outside the slice.

/// `__stl_threshold`.
const THRESHOLD: usize = 16;

/// `std::sort(first, last, less)`.
pub fn stl_sort<T: Copy>(a: &mut [T], less: &mut impl FnMut(&T, &T) -> bool) {
    let n = a.len();
    if n == 0 {
        return;
    }
    introsort_loop(a, 0, n, lg(n) * 2, less);
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

fn median<T: Copy>(a: T, b: T, c: T, less: &mut impl FnMut(&T, &T) -> bool) -> T {
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
    less: &mut impl FnMut(&T, &T) -> bool,
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
    less: &mut impl FnMut(&T, &T) -> bool,
) -> usize {
    let lo = first;
    let hi = last;
    loop {
        while first + 1 < hi && less(&a[first], &pivot) {
            first += 1;
        }
        last -= 1;
        while last > lo && less(&pivot, &a[last]) {
            last -= 1;
        }
        if first >= last {
            return first;
        }
        a.swap(first, last);
        first += 1;
    }
}

fn final_insertion_sort<T: Copy>(a: &mut [T], less: &mut impl FnMut(&T, &T) -> bool) {
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
fn insertion_sort<T: Copy>(a: &mut [T], less: &mut impl FnMut(&T, &T) -> bool) {
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
fn unguarded_linear_insert<T: Copy>(a: &mut [T], mut last: usize, v: T, less: &mut impl FnMut(&T, &T) -> bool) {
    while last > 0 && less(&v, &a[last - 1]) {
        a[last] = a[last - 1];
        last -= 1;
    }
    a[last] = v;
}

/// `partial_sort(first, last, last)`: `make_heap` then `sort_heap`.
fn heap_sort<T: Copy>(a: &mut [T], less: &mut impl FnMut(&T, &T) -> bool) {
    let len = a.len();
    if len < 2 {
        return;
    }
    let mut parent = (len - 2) / 2;
    loop {
        let v = a[parent];
        adjust_heap(a, parent, len, v, less);
        if parent == 0 {
            break;
        }
        parent -= 1;
    }
    let mut end = len;
    while end > 1 {
        // `pop_heap(first, end)`: the last element is re-inserted from the top.
        let v = a[end - 1];
        a[end - 1] = a[0];
        end -= 1;
        adjust_heap(a, 0, end, v, less);
    }
}

/// `__adjust_heap(first, hole, len, value)` with `__push_heap`.
fn adjust_heap<T: Copy>(a: &mut [T], mut hole: usize, len: usize, v: T, less: &mut impl FnMut(&T, &T) -> bool) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_and_keeps_small_ties_in_order() {
        let mut v: Vec<(i32, usize)> = [5, 3, 5, 1, 3, 5].iter().copied().zip(0..).collect();
        stl_sort(&mut v, &mut |a, b| a.0 > b.0);
        assert_eq!(v, vec![(5, 0), (5, 2), (5, 5), (3, 1), (3, 4), (1, 3)]);
    }

    #[test]
    fn sorts_large_inputs_including_the_heap_path() {
        // A sequence that defeats median-of-three quickly still sorts.
        let mut x = 12345u32;
        let mut v: Vec<u32> = (0..2000)
            .map(|_| {
                x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
                (x >> 16) % 50
            })
            .collect();
        let mut want = v.clone();
        want.sort_unstable();
        stl_sort(&mut v, &mut |a, b| a < b);
        assert_eq!(v, want);
        let mut organ: Vec<i32> = (0..300).chain((0..300).rev()).collect();
        stl_sort(&mut organ, &mut |a, b| a < b);
        assert!(organ.windows(2).all(|w| w[0] <= w[1]));
        // A hostile comparator cannot index outside the slice.
        let mut h: Vec<u8> = (0..100).collect();
        stl_sort(&mut h, &mut |_, _| true);
        let mut h: Vec<u8> = (0..100).collect();
        stl_sort(&mut h, &mut |_, _| false);
    }
}
