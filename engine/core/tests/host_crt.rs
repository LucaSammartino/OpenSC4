// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The host's `rand()` state (foundations §12.6): a process-wide switch, so it
//! has a test binary of its own.

use fc4_core::rng::{set_host_crt, CrtRand};
use std::sync::atomic::{AtomicU32, Ordering};

static GAME: AtomicU32 = AtomicU32::new(12345);

fn game(set: Option<u32>) -> u32 {
    if let Some(s) = set {
        GAME.store(s, Ordering::Relaxed);
    }
    GAME.load(Ordering::Relaxed)
}

#[test]
fn cores_share_the_host_state() {
    set_host_crt(Some(game));
    let mut a = CrtRand::default();
    let mut b = CrtRand::new(1);
    // After srand(12345): 7584, 19164, 25795 (§12.1), whichever core draws.
    assert_eq!([a.rand(), b.rand(), a.rand()], [7584, 19164, 25795]);
    assert_eq!(a.state(), GAME.load(Ordering::Relaxed));
    b.srand(1);
    assert_eq!(a.rand(), 41);
    set_host_crt(None);
    assert_eq!(CrtRand::default().rand(), 41);
    assert_eq!(GAME.load(Ordering::Relaxed), 1u32.wrapping_mul(214_013).wrapping_add(2_531_011));
}
