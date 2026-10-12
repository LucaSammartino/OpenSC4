// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The scanning cell iterator (§7.4).

/// The scan state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScanState {
    #[default]
    Fresh,
    Running,
    Done,
}

/// Scans the inclusive rectangle `[x0, x1] × [z0, z1]`, x inner, z outer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanIterator {
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
    /// ≥ 1 ([choice]: a step below 1 is raised to 1 so a scan always ends).
    pub step: i32,
    pub x: i32,
    pub z: i32,
    pub state: ScanState,
}

impl Default for ScanIterator {
    /// The full int32 range, step 1, at (0, 0), fresh (§7.4).
    fn default() -> ScanIterator {
        ScanIterator {
            x0: i32::MIN,
            z0: i32::MIN,
            x1: i32::MAX,
            z1: i32::MAX,
            step: 1,
            x: 0,
            z: 0,
            state: ScanState::Fresh,
        }
    }
}

impl ScanIterator {
    pub fn new(x0: i32, z0: i32, x1: i32, z1: i32) -> ScanIterator {
        ScanIterator {
            x0,
            z0,
            x1,
            z1,
            ..ScanIterator::default()
        }
    }

    /// The rectangle of a grid's tracts `[0, countX − 1] × [0, countZ − 1]`,
    /// an empty range widened to one tract (§7.4).
    pub fn for_counts(count_x: i32, count_z: i32) -> ScanIterator {
        ScanIterator::new(0, 0, count_x.saturating_sub(1).max(0), count_z.saturating_sub(1).max(0))
    }

    pub fn set_step(&mut self, step: i32) {
        self.step = step.max(1);
    }

    /// `Reset(x, z)`: fresh at (x, z), or done if (x, z) is outside.
    pub fn reset(&mut self, x: i32, z: i32) {
        self.x = x;
        self.z = z;
        let inside = (self.x0..=self.x1).contains(&x) && (self.z0..=self.z1).contains(&z);
        self.state = if inside { ScanState::Fresh } else { ScanState::Done };
    }

    /// `GetNext(&x, &z)`.
    pub fn next_cell(&mut self) -> Option<(i32, i32)> {
        match self.state {
            ScanState::Done => return None,
            ScanState::Fresh => self.state = ScanState::Running,
            ScanState::Running => {
                let step = i64::from(self.step.max(1));
                let nx = i64::from(self.x) + step;
                if nx > i64::from(self.x1) {
                    self.x = self.x0;
                    let nz = i64::from(self.z) + step;
                    if nz > i64::from(self.z1) {
                        self.state = ScanState::Done;
                        return None;
                    }
                    self.z = nz as i32;
                } else {
                    self.x = nx as i32;
                }
            }
        }
        Some((self.x, self.z))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_order_and_reset_outside() {
        let mut it = ScanIterator::new(0, 0, 2, 1);
        it.reset(0, 0);
        let got: Vec<_> = std::iter::from_fn(|| it.next_cell()).collect();
        assert_eq!(got, vec![(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)]);
        assert_eq!(it.next_cell(), None);
        it.reset(5, 0);
        assert_eq!(it.next_cell(), None);
        let mut it = ScanIterator::new(i32::MAX - 1, i32::MAX - 1, i32::MAX, i32::MAX);
        it.reset(i32::MAX - 1, i32::MAX - 1);
        assert_eq!(std::iter::from_fn(|| it.next_cell()).count(), 4);
        assert_eq!(ScanIterator::for_counts(0, 3), ScanIterator::new(0, 0, 0, 2));
    }
}
