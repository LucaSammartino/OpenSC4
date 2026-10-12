// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The random cell iterator in "prime-step" mode (§6.7).

use crate::rng::CrtRand;
use crate::stream::{InStream, OutStream};

/// The step table `P` (§6.7); the last entry is table padding.
pub const PRIME_TABLE: [i16; 71] = [
    1, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97, 101, 103, 107, 109,
    113, 179, 211, 257, 307, 373, 479, 577, 641, 719, 857, 977, 1063, 1237, 1471, 1663, 1993, 2221, 2539, 2909, 3259,
    3733, 4507, 5281, 5861, 6481, 6947, 7573, 9013, 10103, 12227, 15683, 17431, 19801, 21569, 23633, 26003, 27529,
    29683, 32051, 32611, 20192,
];

/// The largest count the prime-step mode walks.
const MAX_COUNT: i32 = 130_444;

/// The iterator's run state ([choice] save encoding: 0, 1, 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IteratorState {
    #[default]
    NotStarted,
    Running,
    Exhausted,
}

impl IteratorState {
    fn to_i8(self) -> i8 {
        match self {
            IteratorState::NotStarted => 0,
            IteratorState::Running => 1,
            IteratorState::Exhausted => 2,
        }
    }
}

/// Visits every cell of `[x0, x1] × [z0, z1]` once in a scrambled order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CellIterator {
    pub state: IteratorState,
    /// The raw state byte as read (kept for byte-exact saves).
    pub state_raw: i8,
    /// 1 = prime step (the only mode the weather uses).
    pub mode: i8,
    pub dirty: u8,
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
    pub count: i32,
    /// The "table" mode's entries (never used by the weather; saved).
    pub table: Vec<u16>,
    pub index: i32,
    pub current: i32,
    pub step: i32,
}

impl CellIterator {
    /// A prime-step iterator over `[x0, x1] × [z0, z1]` (inclusive).
    pub fn new(x0: i32, z0: i32, x1: i32, z1: i32) -> CellIterator {
        CellIterator {
            mode: 1,
            x0,
            z0,
            x1,
            z1,
            ..CellIterator::default()
        }
    }

    pub fn set_rect(&mut self, x0: i32, z0: i32, x1: i32, z1: i32) {
        self.x0 = x0;
        self.z0 = z0;
        self.x1 = x1;
        self.z1 = z1;
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

    /// `reset(shuffle)`: makes exactly one `rand()` call when shuffling.
    pub fn reset(&mut self, shuffle: bool, crt: &mut CrtRand) {
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

    /// `next(&x, &z)`.
    pub fn next_cell(&mut self) -> Option<(i32, i32)> {
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
            state: match state_raw {
                1 => IteratorState::Running,
                2 => IteratorState::Exhausted,
                _ => IteratorState::NotStarted,
            },
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
    fn degenerate_rectangles() {
        let mut crt = CrtRand::default();
        for (x0, z0, x1, z1) in [(0, 0, -1, -1), (5, 5, 0, 0), (i32::MIN, 0, i32::MAX, 0)] {
            let mut it = CellIterator::new(x0, z0, x1, z1);
            it.reset(true, &mut crt);
            for _ in 0..10 {
                let _ = it.next_cell();
            }
        }
    }
}
