// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The riot's removal lock (§10; answers disasters.md Q4): the riot manager
//! interface's slot 9 `LockNetworkOccupant` contract, `MarkRiotNetworkOccupants`
//! and `ClearRiotNetworkOccupants`. The riot instances themselves are the
//! disasters subsystem's; this type keeps only the locked set and the count
//! of live instances that decides when the set is cleared.

use std::collections::BTreeSet;

/// What the lock needs from the game.
pub trait RiotHost {
    /// `trafficNetworkMap.SafeGetTileInfoAtCell(x, z)` (slot 14), then the
    /// tile's occupant: `None` when the tile or its occupant is null
    /// (borrowed).
    fn tile_occupant(&self, x: i32, z: i32) -> Option<u64>;
    fn add_ref(&self, token: u64);
    fn release(&self, token: u64);
    /// `n.AsOccupant().UnsetRemovalFlags(m)` (cISC4Occupant slot 13).
    fn unset_removal_flags(&self, occupant: u64, m: u32);
    /// `SetRemovalFlags(m)`.
    fn set_removal_flags(&self, occupant: u64, m: u32);
}

/// The player's removal privilege bit (§10 step 3).
pub const PLAYER_REMOVAL: u32 = 2;

/// The riot manager's locked set (§10).
#[derive(Clone, Debug, Default)]
pub struct RiotLock {
    /// The locked network occupants, each held. The original's set orders by
    /// pointer; here by token [choice] (only the release order depends on
    /// it).
    locked: BTreeSet<u64>,
    /// Live riot instances.
    instances: u32,
}

impl RiotLock {
    pub fn new() -> RiotLock {
        RiotLock::default()
    }

    /// The locked occupants, in set order.
    pub fn locked(&self) -> Vec<u64> {
        self.locked.iter().copied().collect()
    }

    pub fn instances(&self) -> u32 {
        self.instances
    }

    /// A riot instance was added to the riot manager's list.
    pub fn add_instance(&mut self) {
        self.instances = self.instances.saturating_add(1);
    }

    /// `RemoveInstanceFromList` (`0x0057D7E0`): when the last instance is
    /// removed, `ClearRiotNetworkOccupants`.
    pub fn remove_instance(&mut self, host: &dyn RiotHost) {
        self.instances = self.instances.saturating_sub(1);
        if self.instances == 0 {
            self.clear(host);
        }
    }

    /// The riot manager's `Shutdown` (`0x0057DA20`): clears the set.
    pub fn shutdown(&mut self, host: &dyn RiotHost) {
        self.clear(host);
        self.instances = 0;
    }

    /// `MarkRiotNetworkOccupants(x, z)` (`0x0057D300`): for `i` from `x − 3`
    /// to `x + 3` and `j` from `z − 3` to `z + 3` (x outer), lock the tile's
    /// occupant. *[compat]* The bounds are compared unsigned: with `x < 3`
    /// the outer loop runs no row, with `z < 3` every row is empty.
    pub fn mark(&mut self, host: &dyn RiotHost, x: i32, z: i32) {
        let (xl, xh) = (x.wrapping_sub(3) as u32, x.wrapping_add(3) as u32);
        let (zl, zh) = (z.wrapping_sub(3) as u32, z.wrapping_add(3) as u32);
        let mut i = u64::from(xl);
        while i <= u64::from(xh) {
            let mut j = u64::from(zl);
            while j <= u64::from(zh) {
                if let Some(n) = host.tile_occupant(i as u32 as i32, j as u32 as i32) {
                    self.lock(host, n);
                }
                j += 1;
            }
            i += 1;
        }
    }

    /// Slot 9 `LockNetworkOccupant(n)` (`0x0057DA40`): nothing when it is
    /// already in the set; otherwise AddRef, `UnsetRemovalFlags(2)` and
    /// insert.
    pub fn lock(&mut self, host: &dyn RiotHost, n: u64) {
        if n == 0 || self.locked.contains(&n) {
            return;
        }
        host.add_ref(n);
        host.unset_removal_flags(n, PLAYER_REMOVAL);
        self.locked.insert(n);
    }

    /// `ClearRiotNetworkOccupants` (`0x0057D720`): `SetRemovalFlags(2)` and
    /// Release for each locked occupant.
    pub fn clear(&mut self, host: &dyn RiotHost) {
        for n in std::mem::take(&mut self.locked) {
            host.set_removal_flags(n, PLAYER_REMOVAL);
            host.release(n);
        }
    }
}
