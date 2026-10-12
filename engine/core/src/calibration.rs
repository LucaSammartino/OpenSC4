// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! CPU-cycle limit-timer calibration (spec §5.5).
//!
//! `calib` is the number of milliseconds per time-stamp-counter cycle. It is
//! refined from (wall clock, TSC) samples taken at least one second apart with
//! `calib := 0.9 × calib + 0.1 × sample`.

/// The calibration state machine. Pure: the caller supplies the readings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Calibration {
    calib: f32,
    q_last: u64,
    c_last: u64,
    samples: u64,
}

impl Calibration {
    /// Starts from the host's estimate (milliseconds per cycle) and the
    /// readings taken at construction. Returns `None` for an estimate that
    /// is not a positive, finite number.
    pub fn new(initial_ms_per_cycle: f32, q: u64, c: u64) -> Option<Self> {
        if !(initial_ms_per_cycle.is_finite() && initial_ms_per_cycle > 0.0) {
            return None;
        }
        Some(Calibration {
            calib: initial_ms_per_cycle,
            q_last: q,
            c_last: c,
            samples: 0,
        })
    }

    /// One `OnTick` step. Returns true if a sample was folded in.
    ///
    /// If strictly more than one second of wall-clock ticks (`qpc_frequency`)
    /// passed since the last sample point: when the TSC advanced (signed
    /// `c − c_last` greater than zero), fold in
    /// `sample = Δq × (1000 / qpcFrequency) / Δc`; in either case the sample
    /// point moves to (q, c).
    pub fn tick(&mut self, q: u64, c: u64, qpc_frequency: u64) -> bool {
        if qpc_frequency == 0 {
            return false;
        }
        let dq = q.wrapping_sub(self.q_last);
        if dq <= qpc_frequency {
            return false;
        }
        // Signed TSC difference: a counter that went backwards (e.g. after a
        // core migration on an old CPU) is not a valid sample.
        let dc = c.wrapping_sub(self.c_last) as i64;
        let mut updated = false;
        if dc > 0 {
            // [choice] computed in f64 and rounded once to f32.
            let sample = dq as f64 * (1000.0 / qpc_frequency as f64) / dc as f64;
            let next = 0.9 * f64::from(self.calib) + 0.1 * sample;
            if next.is_finite() && next > 0.0 {
                self.calib = next as f32;
                self.samples += 1;
                updated = true;
            }
        }
        self.q_last = q;
        self.c_last = c;
        updated
    }

    /// `CyclesToMillisecs`: milliseconds per cycle.
    pub fn cycles_to_ms(&self) -> f32 {
        self.calib
    }

    /// `MillisecsToCycles`: cycles per millisecond (`1 / calib`).
    pub fn ms_to_cycles(&self) -> f32 {
        1.0 / self.calib
    }

    /// Number of samples folded in so far.
    pub fn samples(&self) -> u64 {
        self.samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f64) -> bool {
        ((f64::from(a) - b) / b).abs() < 1e-6
    }

    #[test]
    fn rejects_bad_estimates() {
        assert!(Calibration::new(0.0, 0, 0).is_none());
        assert!(Calibration::new(-1.0, 0, 0).is_none());
        assert!(Calibration::new(f32::NAN, 0, 0).is_none());
        assert!(Calibration::new(f32::INFINITY, 0, 0).is_none());
    }

    #[test]
    fn follows_the_moving_average_once_per_second() {
        // 1 MHz wall clock; start at 1 GHz (1e-6 ms per cycle).
        let mut c = Calibration::new(1e-6, 0, 0).unwrap();
        assert!(close(c.cycles_to_ms(), 1e-6) && close(c.ms_to_cycles(), 1e6));
        // Half a second: no sample.
        assert!(!c.tick(500_000, 500_000_000, 1_000_000));
        // Exactly one second since the last sample point: still no sample (strictly more).
        assert!(!c.tick(1_000_000, 1_000_000_000, 1_000_000));
        // 1.1 s at 2 GHz: sample 5e-7 → 0.9e-6 + 0.5e-7 = 9.5e-7.
        assert!(c.tick(1_100_000, 2_200_000_000, 1_000_000));
        assert!(close(c.cycles_to_ms(), 9.5e-7));
        assert!(close(c.ms_to_cycles(), 1.0 / 9.5e-7));
        assert_eq!(c.samples(), 1);
        // TSC stood still: no sample, but the sample point moves.
        assert!(!c.tick(2_200_000, 2_200_000_000, 1_000_000));
        assert!(close(c.cycles_to_ms(), 9.5e-7));
        // From the moved point: 1.2 s at 1 GHz → sample 1e-6.
        assert!(c.tick(3_400_000, 3_400_000_000, 1_000_000));
        assert!(close(c.cycles_to_ms(), 0.9 * 9.5e-7 + 0.1 * 1e-6));
        // TSC went backwards: ignored.
        let before = c.cycles_to_ms();
        assert!(!c.tick(5_000_000, 3_000_000_000, 1_000_000));
        assert_eq!(c.cycles_to_ms(), before);
    }

    #[test]
    fn converges_to_the_true_rate() {
        let mut c = Calibration::new(1.0, 0, 0).unwrap(); // the "unmaintained" static value
        let (mut q, mut t) = (0u64, 0u64);
        for _ in 0..300 {
            q += 1_500_000; // 1.5 s at 1 MHz
            t += 4_500_000_000; // 3 GHz
            c.tick(q, t, 1_000_000);
        }
        assert!(close(c.cycles_to_ms(), 1.0 / 3e6));
    }
}
