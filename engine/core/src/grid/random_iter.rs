// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The random cell iterator (simgrid §7.4, weather §6.7, foundations
//! §12.4): "prime-step" mode (mode 1) and "table" mode (mode 0).
//!
//! Table mode *[compat]* (weather §6.7, G2): entries hold the **absolute**
//! coordinates `(x mod 256) << 8 | (z mod 256)`; the `dirty` flag, set by
//! `SetClipRect` ([`CellIterator::set_rect`]) and by a table resize, is never
//! cleared, so every table-mode reset refills the table in natural order (x
//! outer, z inner) before the optional shuffle. A table-mode reset leaves the
//! prime-step `current` and `step` unchanged, and `next` walks the table by
//! its length.
//!
//! [choice]s: the table is at most [`MAX_TABLE`] entries (a larger
//! rectangle keeps its count but only the first entries exist, so `next`
//! stops at the table's end); the reader fails on a stream error (the
//! original resizes the table to the stream object's address, weather §10).

use crate::rng::CrtRand;
use crate::stream::{InStream, OutStream};

/// The step table `P` (§6.7); the last entry is table padding.
pub const PRIME_TABLE: [i16; 71] = [
    1, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97, 101, 103, 107, 109,
    113, 179, 211, 257, 307, 373, 479, 577, 641, 719, 857, 977, 1063, 1237, 1471, 1663, 1993, 2221, 2539, 2909, 3259,
    3733, 4507, 5281, 5861, 6481, 6947, 7573, 9013, 10103, 12227, 15683, 17431, 19801, 21569, 23633, 26003, 27529,
    29683, 32051, 32611, 20192,
];

/// The `mode` byte of the table mode.
pub const MODE_TABLE: i8 = 0;
/// The `mode` byte of the prime-step mode.
pub const MODE_PRIME: i8 = 1;

/// The largest count the prime-step mode walks.
const MAX_COUNT: i32 = 130_444;

/// The largest table the table mode allocates ([choice]: 1024 × 1024 cells).
pub const MAX_TABLE: usize = 1 << 20;

/// The iterator's run state (weather §10): the saved byte is 1 = running
/// (also a freshly constructed, never-reset iterator) or 2 = exhausted; the
/// original never writes 0. A read byte other than 2 counts as running.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IteratorState {
    #[default]
    Running,
    Exhausted,
}

/// The saved state byte of a running iterator.
pub const STATE_RUNNING: i8 = 1;
/// The saved state byte of an exhausted iterator.
pub const STATE_EXHAUSTED: i8 = 2;

impl IteratorState {
    fn to_i8(self) -> i8 {
        match self {
            IteratorState::Running => STATE_RUNNING,
            IteratorState::Exhausted => STATE_EXHAUSTED,
        }
    }

    /// The state a saved byte means.
    pub fn from_i8(b: i8) -> IteratorState {
        if b == STATE_EXHAUSTED {
            IteratorState::Exhausted
        } else {
            IteratorState::Running
        }
    }
}

/// Visits every cell of `[x0, x1] × [z0, z1]` once in a scrambled order.
#[derive(Clone, Debug, PartialEq)]
pub struct CellIterator {
    pub state: IteratorState,
    /// The raw state byte (1 or 2 when set by the iterator; kept unchanged
    /// as read, for byte-exact saves).
    pub state_raw: i8,
    /// 1 = prime step (weather, flammability), 0 = table (pollution).
    pub mode: i8,
    pub dirty: u8,
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
    pub count: i32,
    /// The table mode's packed entries (saved in both modes).
    pub table: Vec<u16>,
    pub index: i32,
    pub current: i32,
    pub step: i32,
}

impl Default for CellIterator {
    /// A running (never reset) prime-step iterator with an empty rectangle
    /// at 0 and count 0.
    fn default() -> CellIterator {
        CellIterator {
            state: IteratorState::Running,
            state_raw: STATE_RUNNING,
            mode: MODE_PRIME,
            dirty: 0,
            x0: 0,
            z0: 0,
            x1: 0,
            z1: 0,
            count: 0,
            table: Vec::new(),
            index: 0,
            current: 0,
            step: 0,
        }
    }
}

impl CellIterator {
    /// A prime-step iterator over `[x0, x1] × [z0, z1]` (inclusive).
    pub fn new(x0: i32, z0: i32, x1: i32, z1: i32) -> CellIterator {
        CellIterator {
            mode: MODE_PRIME,
            x0,
            z0,
            x1,
            z1,
            ..CellIterator::default()
        }
    }

    /// A table-mode iterator over `[x0, x1] × [z0, z1]` (inclusive).
    pub fn new_table(x0: i32, z0: i32, x1: i32, z1: i32) -> CellIterator {
        CellIterator {
            mode: MODE_TABLE,
            ..CellIterator::new(x0, z0, x1, z1)
        }
    }

    /// The iterator as the original's constructor leaves it (crime §3,
    /// flammability §3): prime-step mode, state running, dirty, step 1, an
    /// empty rectangle at 0 and count 0, so the first `next` returns false.
    pub fn constructed() -> CellIterator {
        let mut it = CellIterator {
            mode: MODE_PRIME,
            dirty: 1,
            step: 1,
            ..CellIterator::default()
        };
        it.set_state(IteratorState::Running);
        it
    }

    /// `SetClipRect`: the rectangle; sets `dirty` (weather §6.7).
    pub fn set_rect(&mut self, x0: i32, z0: i32, x1: i32, z1: i32) {
        self.x0 = x0;
        self.z0 = z0;
        self.x1 = x1;
        self.z1 = z1;
        self.dirty = 1;
    }

    fn width(&self) -> i64 {
        i64::from(self.x1) - i64::from(self.x0) + 1
    }

    fn height(&self) -> i64 {
        i64::from(self.z1) - i64::from(self.z0) + 1
    }

    fn set_state(&mut self, s: IteratorState) {
        self.state = s;
        self.state_raw = s.to_i8();
    }

    /// `reset(shuffle)`: in prime-step mode exactly one `rand()` call when
    /// shuffling; in table mode the table is refilled (when `dirty`, which
    /// is always the case once sized) and `random_shuffle`d (n − 1 calls)
    /// when shuffling.
    pub fn reset(&mut self, shuffle: bool, crt: &mut CrtRand) {
        if self.mode == MODE_TABLE {
            self.reset_table(shuffle, crt);
            return;
        }
        let (w, h) = (self.width(), self.height());
        let mut count = if w <= 0 || h <= 0 {
            0
        } else {
            (w * h).min(i64::from(i32::MAX)) as i32
        };
        self.index = 0;
        self.set_state(IteratorState::Running);
        let mut k = match count {
            c if c < 29 => 0,
            c if c < 71 => 10,
            c if c < 113 => 20,
            c if c < 857 => 30,
            c if c < 3259 => 40,
            c if c < 12227 => 50,
            _ => {
                count = count.min(MAX_COUNT);
                60
            }
        };
        while k < 70 && i32::from(PRIME_TABLE[k]) < count / 4 {
            k += 1;
        }
        while k < 70 && count % i32::from(PRIME_TABLE[k]) == 0 {
            k += 1;
        }
        self.count = count;
        self.step = i32::from(PRIME_TABLE[k]);
        if shuffle {
            let r = crt.rand() as i32;
            self.current = if count > 0 { r % count } else { 0 };
        }
    }

    /// Table-mode reset (weather §6.7).
    fn reset_table(&mut self, shuffle: bool, crt: &mut CrtRand) {
        let (w, h) = (self.width(), self.height());
        let count = if w <= 0 || h <= 0 { 0 } else { w.saturating_mul(h) };
        self.count = count.min(i64::from(i32::MAX)) as i32;
        self.index = 0;
        self.set_state(IteratorState::Running);
        let n = (count.max(0) as u64).min(MAX_TABLE as u64) as usize;
        if self.table.len() != n {
            self.table.resize(n, 0);
            self.dirty = 1;
        }
        if self.dirty != 0 {
            let cells = (0..w.max(0)).flat_map(|dx| (0..h.max(0)).map(move |dz| (dx, dz)));
            for (slot, (dx, dz)) in self.table.iter_mut().zip(cells) {
                let x = (i64::from(self.x0) + dx) & 0xFF;
                let z = (i64::from(self.z0) + dz) & 0xFF;
                *slot = ((x << 8) | z) as u16;
            }
        }
        if shuffle {
            crt.random_shuffle(&mut self.table);
        }
    }

    /// Table-mode `next` (weather §6.7): absolute coordinates.
    fn next_table(&mut self) -> Option<(i32, i32)> {
        if self.state == IteratorState::Exhausted {
            return None;
        }
        let len = self.table.len();
        let e = match usize::try_from(self.index).ok().filter(|&i| i < len) {
            Some(i) => self.table[i],
            None => {
                self.set_state(IteratorState::Exhausted);
                return None;
            }
        };
        self.index += 1;
        if self.index as usize == len {
            self.set_state(IteratorState::Exhausted);
        }
        Some((i32::from(e >> 8), i32::from(e & 0xFF)))
    }

    /// Whether the iterator ran out (simgrid §7.4 "exhausted").
    pub fn is_exhausted(&self) -> bool {
        self.state == IteratorState::Exhausted
    }

    /// `next(&x, &z)`.
    pub fn next_cell(&mut self) -> Option<(i32, i32)> {
        if self.mode == MODE_TABLE {
            return self.next_table();
        }
        if self.state == IteratorState::Exhausted || self.count <= 0 {
            self.set_state(IteratorState::Exhausted);
            return None;
        }
        self.index = self.index.saturating_add(1);
        if self.index > self.count {
            self.set_state(IteratorState::Exhausted);
            return None;
        }
        let count = i64::from(self.count);
        self.current = ((i64::from(self.current) + i64::from(self.step)).rem_euclid(count)) as i32;
        let w = self.width().max(1);
        let c = i64::from(self.current);
        let x = (i64::from(self.x0) + c % w) as i32;
        let z = (i64::from(self.z0) + c / w) as i32;
        if self.index == self.count {
            self.set_state(IteratorState::Exhausted);
        }
        Some((x, z))
    }

    /// The saved form (§10).
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_i8(self.state_raw);
        out.put_i8(self.mode);
        out.put_u8(self.dirty);
        out.put_i32(self.x0);
        out.put_i32(self.z0);
        out.put_i32(self.x1);
        out.put_i32(self.z1);
        out.put_i32(self.count);
        out.put_u32(self.table.len() as u32);
        for &t in &self.table {
            out.put_u16(t);
        }
        out.put_i32(self.index);
        out.put_i32(self.current);
        out.put_i32(self.step);
        out.error() == 0
    }

    pub fn read(inp: &mut dyn InStream) -> Option<CellIterator> {
        let state_raw = inp.get_i8()?;
        let mut it = CellIterator {
            state: IteratorState::from_i8(state_raw),
            state_raw,
            mode: inp.get_i8()?,
            dirty: inp.get_u8()?,
            x0: inp.get_i32()?,
            z0: inp.get_i32()?,
            x1: inp.get_i32()?,
            z1: inp.get_i32()?,
            count: inp.get_i32()?,
            ..CellIterator::default()
        };
        let n = inp.get_u32()?;
        for _ in 0..n {
            it.table.push(inp.get_u16()?);
        }
        it.index = inp.get_i32()?;
        it.current = inp.get_i32()?;
        it.step = inp.get_i32()?;
        Some(it)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_for_city_sizes() {
        let mut crt = CrtRand::default();
        for (n, step) in [(64, 3733), (128, 15683), (256, 17431)] {
            let mut it = CellIterator::new(0, 0, n - 1, n - 1);
            it.reset(false, &mut crt);
            assert_eq!(it.step, step);
        }
        assert_eq!(crt, CrtRand::default());
    }

    #[test]
    fn full_pass_visits_each_cell_once() {
        let mut crt = CrtRand::default();
        for (w, h) in [(1, 1), (3, 5), (16, 16), (7, 13), (64, 64)] {
            let mut it = CellIterator::new(2, 3, 2 + w - 1, 3 + h - 1);
            let before = crt.state();
            it.reset(true, &mut crt);
            let mut once = CrtRand::new(before);
            once.rand();
            assert_eq!(crt, once, "reset makes exactly one rand() call");
            let mut seen = vec![false; (w * h) as usize];
            while let Some((x, z)) = it.next_cell() {
                let i = ((z - 3) * w + (x - 2)) as usize;
                assert!(!seen[i]);
                seen[i] = true;
            }
            assert!(seen.iter().all(|&s| s));
            assert_eq!(it.next_cell(), None);
            assert_eq!(it.state, IteratorState::Exhausted);
        }
    }

    #[test]
    fn table_mode() {
        let mut crt = CrtRand::new(1);
        let mut it = CellIterator::new_table(0, 0, 3, 2);
        it.reset(true, &mut crt);
        let mut once = CrtRand::new(1);
        for _ in 0..11 {
            once.rand();
        }
        assert_eq!(crt, once, "n - 1 rand() calls");
        let mut seen = Vec::new();
        while let Some(c) = it.next_cell() {
            seen.push(c);
        }
        assert_eq!(seen.len(), 12);
        seen.sort();
        let mut want: Vec<_> = (0..4).flat_map(|x| (0..3).map(move |z| (x, z))).collect();
        want.sort();
        assert_eq!(seen, want);
        assert!(it.is_exhausted());
        // Unshuffled refill: natural order, x outer.
        let mut it = CellIterator::new_table(5, 6, 6, 7);
        it.reset(false, &mut crt);
        let got: Vec<_> = std::iter::from_fn(|| it.next_cell()).collect();
        assert_eq!(got, vec![(5, 6), (5, 7), (6, 6), (6, 7)]);
        assert_eq!(CellIterator::constructed().next_cell(), None);
    }

    #[test]
    fn state_byte() {
        use crate::stream::{ByteReader, ByteWriter};
        let save = |it: &CellIterator| {
            let mut w = ByteWriter::new();
            assert!(it.write(&mut w));
            w.into_bytes()
        };
        // Never reset: running (1); 0 is never written.
        for it in [
            CellIterator::new(0, 0, 3, 3),
            CellIterator::new_table(0, 0, 3, 3),
            CellIterator::constructed(),
        ] {
            assert_eq!(save(&it)[0], 1);
        }
        let mut it = CellIterator::new(0, 0, 0, 0);
        it.reset(false, &mut CrtRand::new(1));
        assert!(it.next_cell().is_some());
        assert_eq!(save(&it)[0], 2);
        // A read byte other than 2 counts as running and is kept unchanged.
        let mut it = CellIterator::new(0, 0, 1, 0);
        it.reset(false, &mut CrtRand::new(1));
        let mut bytes = save(&it);
        bytes[0] = 0;
        let mut r = CellIterator::read(&mut ByteReader::new(&bytes)).unwrap();
        assert_eq!((r.state, r.state_raw), (IteratorState::Running, 0));
        assert_eq!(save(&r), bytes);
        assert!(r.next_cell().is_some());
        bytes[0] = 2;
        let mut r = CellIterator::read(&mut ByteReader::new(&bytes)).unwrap();
        assert_eq!(r.state, IteratorState::Exhausted);
        assert_eq!(r.next_cell(), None);
    }

    #[test]
    fn degenerate_rectangles() {
        let mut crt = CrtRand::default();
        for (x0, z0, x1, z1) in [(0, 0, -1, -1), (5, 5, 0, 0), (i32::MIN, 0, i32::MAX, 0)] {
            for mut it in [
                CellIterator::new(x0, z0, x1, z1),
                CellIterator::new_table(x0, z0, x1, z1),
            ] {
                it.reset(true, &mut crt);
                for _ in 0..10 {
                    let _ = it.next_cell();
                }
                it.table.truncate(1);
                it.count = 5;
                it.state = IteratorState::Running;
                for _ in 0..10 {
                    let _ = it.next_cell();
                }
            }
        }
    }
}
