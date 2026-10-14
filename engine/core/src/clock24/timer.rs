// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The clock's timer (§4.3 "Timer model"), in microseconds of the host's
//! monotonic clock. *[choice]* No call-overhead correction and no minimum
//! tick (CK2).

/// A `cRZTimer` in unit 3 (microseconds).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Timer {
    pub running: bool,
    /// The host time when the running interval began.
    pub start_us: u64,
    /// The elapsed time of the finished intervals.
    pub elapsed_us: u64,
}

impl Timer {
    /// `Start`: starts a stopped timer and keeps the elapsed time it has.
    pub fn start(&mut self, now: u64) {
        if !self.running {
            self.running = true;
            self.start_us = now;
        }
    }

    /// `Stop`: adds the running interval to the elapsed time.
    pub fn stop(&mut self, now: u64) {
        if self.running {
            self.elapsed_us = self.elapsed_us.wrapping_add(now.wrapping_sub(self.start_us));
            self.running = false;
        }
    }

    /// `Restart`: the elapsed time is 0 and the timer runs ([choice]: a
    /// stopped timer is started).
    pub fn restart(&mut self, now: u64) {
        self.elapsed_us = 0;
        self.running = true;
        self.start_us = now;
    }

    /// `Reset`: stopped, elapsed 0.
    pub fn reset(&mut self) {
        self.running = false;
        self.elapsed_us = 0;
        self.start_us = 0;
    }

    /// The elapsed time including the running interval, in microseconds.
    pub fn elapsed_us(&self, now: u64) -> u64 {
        if self.running {
            self.elapsed_us.wrapping_add(now.wrapping_sub(self.start_us))
        } else {
            self.elapsed_us
        }
    }

    /// `GetElapsedTime`: truncated to 32 bits.
    pub fn elapsed(&self, now: u64) -> u32 {
        self.elapsed_us(now) as u32
    }
}
