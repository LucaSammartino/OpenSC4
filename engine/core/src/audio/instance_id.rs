// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4AudioSoundManager::GenerateNewInstanceId` (`docs/specs/audio.md`
//! §8.4). Section references (`§n`) point at that spec.
//!
//! ```text
//! mov  ecx, dword ptr [0x00B098C0]
//! test ecx, ecx
//! jne  have
//! mov  ecx, 1
//! have:
//! mov  eax, ecx          ; return value = pre-increment
//! inc  ecx
//! mov  dword ptr [0x00B098C0], ecx
//! ret
//! ```
//!
//! - A single **process-global** counter at **`0x00B098C0`** — **not**
//!   per-manager (no `this` is read).
//! - Returns the **pre-increment** value, so ids are issued 1, 2, 3, ...
//! - **Skips 0**: a zero counter is forced to 1 first, so 0 is never issued,
//!   including after a 32-bit wrap.
//! - No saturation. `ret` with no stack args.
//!
//! The same sequence is inlined in `_StartSound` when the caller passes id 0
//! (§8.4); those four sites are the only references to `0x00B098C0` in the
//! image.
//!
//! *[compat]*: a replacement must keep a single global counter, must never
//! return 0, and must use the pre-increment value (§8.4, conformance test 20).

/// The counter's global address in the original (§8.4).
pub const INSTANCE_ID_ADDR: u32 = 0x00B0_98C0;

/// The process-wide instance-id counter.
///
/// A `Cell<u32>` rather than an `AtomicU32` because the original is a plain
/// global incremented without a lock on one thread; the capi layer holds it
/// per handle and publishes it the same way the other cores publish their
/// state (HANDOFF §5.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InstanceIdCounter {
    /// The raw counter word at `0x00B098C0`.
    state: u32,
}

impl InstanceIdCounter {
    /// A counter as the process starts: the global is zero until first use,
    /// and the first call therefore returns 1 (§8.4).
    pub const fn new() -> Self {
        InstanceIdCounter { state: 0 }
    }

    /// A counter already holding `state`, for a vector that wants to start at
    /// a known point or drive a wrap.
    pub const fn at(state: u32) -> Self {
        InstanceIdCounter { state }
    }

    /// The raw counter word (the value at `0x00B098C0`).
    pub const fn raw(&self) -> u32 {
        self.state
    }

    /// `GenerateNewInstanceId` (§8.4): the pre-increment value, never 0.
    pub fn generate(&mut self) -> u32 {
        // `test ecx, ecx` / `jne have` / `mov ecx, 1`
        if self.state == 0 {
            self.state = 1;
        }
        // `mov eax, ecx` — the pre-increment value is the return.
        let value = self.state;
        // `inc ecx` / `mov [0x00B098C0], ecx`
        self.state = self.state.wrapping_add(1);
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Conformance test 20: 1, 2, 3 from a single counter, pre-increment.
    #[test]
    fn issues_1_2_3() {
        let mut c = InstanceIdCounter::new();
        assert_eq!(c.generate(), 1);
        assert_eq!(c.generate(), 2);
        assert_eq!(c.generate(), 3);
    }

    /// Conformance test 20: never 0, including after a 32-bit wrap.
    #[test]
    fn never_returns_zero() {
        let mut c = InstanceIdCounter::new();
        for _ in 0..10 {
            assert_ne!(c.generate(), 0);
        }
        // Drive the counter to the wrap point: 0xFFFFFFFF, then 0, then 1.
        let mut w = InstanceIdCounter::at(0xFFFF_FFFF);
        assert_eq!(w.generate(), 0xFFFF_FFFF);
        // The counter word is now 0; the next call forces it to 1 and returns 1.
        assert_eq!(w.raw(), 0);
        assert_eq!(w.generate(), 1);
        assert_eq!(w.generate(), 2);
    }

    /// The counter is one global, so a second handle sees the same sequence.
    #[test]
    fn is_one_global_counter() {
        let mut a = InstanceIdCounter::new();
        assert_eq!(a.generate(), 1);
        assert_eq!(a.generate(), 2);
    }
}
