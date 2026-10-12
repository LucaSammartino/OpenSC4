// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! One-bit-per-cell bitmaps and the selection-grow flood fill (power.md
//! §2.1; plumbing's visited bitmap, plumbing.md §2).
//!
//! Layout *[compat]*: bit index `i = width · z + x`, stored in word `i >> 5`
//! at bit `x & 31` (LSB first; for a width that is not a multiple of 32 this
//! is not `i & 31`, as in the original). The word count is
//! `(width · height) / 32` (signed division), so it is also the persisted
//! size. "clear" and "xor" process `width²` bits (the original assumes a
//! square city).
//!
//! [choice]s: a bit whose word lies beyond the stored words reads 0 and
//! ignores writes; negative coordinates are outside; whole-bitmap helpers
//! stop at the stored words.

/// The default iteration cap of [`Bitmap::selection_grow`] (power.md §2.1).
pub const GROW_MAX_ITERATIONS: u32 = 600;

/// An inclusive cell rectangle `{x1, z1, x2, z2}`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellRect {
    pub x1: i32,
    pub z1: i32,
    pub x2: i32,
    pub z2: i32,
}

impl CellRect {
    pub fn new(x1: i32, z1: i32, x2: i32, z2: i32) -> CellRect {
        CellRect { x1, z1, x2, z2 }
    }
}

/// A bitmap of `width × height` cells.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bitmap {
    width: i32,
    height: i32,
    words: Vec<u32>,
}

/// The largest bitmap side allocated ([choice]: hostile sizes cannot make
/// the core allocate more).
pub const MAX_BITMAP_SIDE: i32 = 4096;

impl Bitmap {
    /// A cleared bitmap; sides are clamped to `0 ..= MAX_BITMAP_SIDE`.
    pub fn new(width: i32, height: i32) -> Bitmap {
        let (w, h) = (width.clamp(0, MAX_BITMAP_SIDE), height.clamp(0, MAX_BITMAP_SIDE));
        Bitmap {
            width: w,
            height: h,
            words: vec![0; Bitmap::word_count(w, h)],
        }
    }

    /// `(width · height) / 32` (signed division, power.md §2.1).
    pub fn word_count(width: i32, height: i32) -> usize {
        (i64::from(width.max(0)) * i64::from(height.max(0)) / 32) as usize
    }

    /// A bitmap over stored words (a save); missing words are 0 and extra
    /// ones dropped.
    pub fn from_words(width: i32, height: i32, words: &[u32]) -> Bitmap {
        let mut b = Bitmap::new(width, height);
        for (d, &s) in b.words.iter_mut().zip(words) {
            *d = s;
        }
        b
    }

    /// A bitmap over exactly the saved words (power.md §9.1: `Read`
    /// allocates `words × 4` bytes, whatever the city's size). The sides are
    /// only the view: the original takes the width from the city at each use,
    /// so a load before the cell counts are known keeps every word, and
    /// [`Bitmap::set_size`] supplies the sides later.
    pub fn from_saved(width: i32, height: i32, words: Vec<u32>) -> Bitmap {
        Bitmap {
            width: width.clamp(0, MAX_BITMAP_SIDE),
            height: height.clamp(0, MAX_BITMAP_SIDE),
            words,
        }
    }

    /// Changes the sides the bits are addressed with. The stored words are
    /// kept; [choice] if fewer than the new sides need, cleared words are
    /// appended (a bitmap made while the sides were unknown).
    pub fn set_size(&mut self, width: i32, height: i32) {
        self.width = width.clamp(0, MAX_BITMAP_SIDE);
        self.height = height.clamp(0, MAX_BITMAP_SIDE);
        let need = Bitmap::word_count(self.width, self.height);
        if self.words.len() < need {
            self.words.resize(need, 0);
        }
    }

    /// The number of set bits in the stored words (diagnostics).
    pub fn count_ones(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    /// The stored words (the persisted form).
    pub fn words(&self) -> &[u32] {
        &self.words
    }

    /// (word, mask) of a cell, if stored.
    fn locate(&self, x: i32, z: i32) -> Option<(usize, u32)> {
        if x < 0 || z < 0 {
            return None;
        }
        let i = i64::from(self.width) * i64::from(z) + i64::from(x);
        let w = (i >> 5) as usize;
        (w < self.words.len()).then(|| (w, 1u32 << (x & 31)))
    }

    /// Pixel test.
    pub fn test(&self, x: i32, z: i32) -> bool {
        self.locate(x, z).is_some_and(|(w, m)| self.words[w] & m != 0)
    }

    /// Pixel set (`on`) or clear.
    pub fn put(&mut self, x: i32, z: i32, on: bool) {
        if let Some((w, m)) = self.locate(x, z) {
            if on {
                self.words[w] |= m;
            } else {
                self.words[w] &= !m;
            }
        }
    }

    pub fn set(&mut self, x: i32, z: i32) {
        self.put(x, z, true);
    }

    /// The words covering the first `width²` bits (the square-city helpers).
    fn square_words(&self) -> usize {
        Bitmap::word_count(self.width, self.width).min(self.words.len())
    }

    /// "clear": `width²` bits to 0 *[compat]*.
    pub fn clear(&mut self) {
        let n = self.square_words();
        self.words[..n].fill(0);
    }

    /// "xor": `self ^= other` over `width²` bits *[compat]*.
    pub fn xor(&mut self, other: &Bitmap) {
        let n = self.square_words();
        for (d, s) in self.words[..n].iter_mut().zip(&other.words) {
            *d ^= s;
        }
    }

    /// Whether any bit is set (all stored words).
    pub fn any(&self) -> bool {
        self.words.iter().any(|&w| w != 0)
    }

    fn cells(r: CellRect) -> impl Iterator<Item = (i32, i32)> {
        (r.z1..=r.z2).flat_map(move |z| (r.x1..=r.x2).map(move |x| (x, z)))
    }

    /// The rectangle clipped to the stored cells.
    fn clip(&self, r: CellRect) -> CellRect {
        CellRect {
            x1: r.x1.max(0),
            z1: r.z1.max(0),
            x2: r.x2.min(self.width - 1),
            z2: r.z2.min(self.height.max(self.width) - 1),
        }
    }

    /// "rect set" (`on`) / "rect clear".
    pub fn rect_put(&mut self, r: CellRect, on: bool) {
        for (x, z) in Bitmap::cells(self.clip(r)) {
            self.put(x, z, on);
        }
    }

    /// "rect test any".
    pub fn rect_any(&self, r: CellRect) -> bool {
        Bitmap::cells(self.clip(r)).any(|(x, z)| self.test(x, z))
    }

    /// "rect or": `self |= src` inside the rectangle.
    pub fn rect_or(&mut self, src: &Bitmap, r: CellRect) {
        for (x, z) in Bitmap::cells(self.clip(r)) {
            if src.test(x, z) {
                self.set(x, z);
            }
        }
    }

    /// "selection grow" (power.md §2.1): grows the marked bits of `self`
    /// into the cells set in `mask`, 4-connected, at most `max_iterations`
    /// iterations of a horizontal then a vertical Jacobi pass over the
    /// active band of rows. Rows are bounded by `width` *[compat]*. Seed bits
    /// outside `mask` stay marked. Returns the widened `(row_min, row_max)`.
    pub fn selection_grow(&mut self, mask: &Bitmap, max_iterations: u32, row_min: i32, row_max: i32) -> (i32, i32) {
        let w = self.width;
        if w <= 0 {
            return (row_min, row_max);
        }
        let (mut row_min, mut row_max) = (row_min, row_max);
        let mut lo = if row_min > 0 { row_min - 1 } else { row_min };
        let mut hi = if row_max < w - 1 { row_max + 1 } else { row_max };
        for _ in 0..max_iterations {
            let (blo, bhi) = (lo.max(0), hi.min(w - 1));
            let mut changed: Option<(i32, i32)> = None;
            let mut note = |z: i32| {
                changed = Some(changed.map_or((z, z), |(a, b)| (a.min(z), b.max(z))));
            };
            // Horizontal pass: each row against its own pre-pass state.
            for z in blo..=bhi {
                let row: Vec<bool> = (0..w).map(|x| self.test(x, z)).collect();
                for x in 0..w {
                    let ux = x as usize;
                    let near = (x > 0 && row[ux - 1]) || (x + 1 < w && row[ux + 1]);
                    if near && !row[ux] && mask.test(x, z) {
                        self.set(x, z);
                        note(z);
                    }
                }
            }
            // Vertical pass: rows read as they were before this pass.
            let snap = |b: &Bitmap, z: i32| -> Vec<bool> {
                if z < 0 || z >= w {
                    vec![false; w as usize]
                } else {
                    (0..w).map(|x| b.test(x, z)).collect()
                }
            };
            let before: Vec<Vec<bool>> = (blo - 1..=bhi + 1).map(|z| snap(self, z)).collect();
            for z in blo..=bhi {
                let k = (z - blo + 1) as usize;
                for x in 0..w {
                    let ux = x as usize;
                    if !before[k][ux] && (before[k - 1][ux] || before[k + 1][ux]) && mask.test(x, z) {
                        self.set(x, z);
                        note(z);
                    }
                }
            }
            let Some((clo, chi)) = changed else { break };
            lo = if clo > 0 { clo - 1 } else { clo };
            hi = if chi + 1 < w { chi + 1 } else { chi };
            row_min = row_min.min(lo);
            row_max = row_max.max(hi);
        }
        (row_min, row_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_and_rects() {
        let mut b = Bitmap::new(40, 40);
        assert_eq!(b.words().len(), 50);
        b.set(33, 1);
        // i = 40 + 33 = 73 → word 2, bit 33 & 31 = 1.
        assert_eq!(b.words()[2], 1 << 1);
        assert!(b.test(33, 1) && !b.test(32, 1));
        b.rect_put(CellRect::new(0, 0, 2, 1), true);
        assert!(b.rect_any(CellRect::new(2, 1, 5, 5)));
        assert!(!b.rect_any(CellRect::new(3, 2, 5, 5)));
        let mut c = Bitmap::new(40, 40);
        c.rect_or(&b, CellRect::new(0, 0, 0, 0));
        assert!(c.test(0, 0) && !c.test(1, 0));
        c.xor(&b);
        assert!(!c.test(0, 0) && c.test(1, 0));
        c.clear();
        assert!(!c.any());
        // Out of range: reads false, writes ignored, no panic.
        c.put(-1, 0, true);
        c.put(i32::MAX, i32::MAX, true);
        assert!(!c.any() && !c.test(i32::MIN, 3));
        let odd = Bitmap::new(5, 5);
        assert_eq!(odd.words().len(), 0);
        assert!(!odd.test(0, 0));
    }

    #[test]
    fn grow_is_four_connected_and_banded() {
        let mut mask = Bitmap::new(32, 32);
        // An L-shaped wire: row 2 from x 1..6, then column 6 down to z 9.
        mask.rect_put(CellRect::new(1, 2, 6, 2), true);
        mask.rect_put(CellRect::new(6, 2, 6, 9), true);
        // A diagonal neighbour only: not connected.
        mask.set(7, 10);
        let mut mark = Bitmap::new(32, 32);
        mark.set(1, 2);
        let (lo, hi) = mark.selection_grow(&mask, GROW_MAX_ITERATIONS, 2, 3);
        for x in 1..=6 {
            assert!(mark.test(x, 2));
        }
        for z in 2..=9 {
            assert!(mark.test(6, z), "z {z}");
        }
        assert!(!mark.test(7, 10) && !mark.test(0, 2));
        assert!(lo <= 2 && hi >= 9, "{lo} {hi}");
        // One iteration grows one step each way per pass.
        let mut m1 = Bitmap::new(32, 32);
        m1.set(1, 2);
        m1.selection_grow(&mask, 1, 2, 2);
        assert!(m1.test(2, 2) && !m1.test(3, 2));
        // Empty width: nothing happens.
        assert_eq!(Bitmap::new(0, 0).selection_grow(&mask, 5, 1, 2), (1, 2));
    }
}
