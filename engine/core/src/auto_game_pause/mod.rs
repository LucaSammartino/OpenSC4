// SPDX-License-Identifier: GPL-3.0-or-later
//! Dedicated portable cSC4AutoGamePause and separate SC4GamePause normal algorithms.
//!
//! Contract: docs/specs/auto-game-pause.md §§2–4. No game binding or participant
//! implementation is supplied. In particular AGP1 is an unresolved raw interface,
//! not a sound/graphics abstraction. Host callbacks must return normally.
//! Allocation, copies/moves, duplicate destruction, reentry, unwind and arbitrary
//! storage mutation are outside the contract. No native lifetime guarantee follows
//! from Rust ownership. Explicit construction/destruction avoids retaining a host
//! borrow or secretly acquiring foreign services in Drop.

/// Opaque full 32-bit pointer identity; zero alone denotes null. Never dereferenced.
pub type Pointer = u32;

/// Individual foreign primitives, with exact byte-offset method tokens.
///
/// Lookup owns its output reference; all other pointer replies are borrowed.
/// Status and operation/Release reply bits have no control-flow meaning here.
/// The host supplies no whole-pause, final-state or reference-balance oracle.
pub trait Host {
    fn framework(&mut self) -> Pointer;
    fn lookup(&mut self, framework: Pointer, slot: u32, service: u32, iid: u32, output: &mut Pointer) -> u32;
    fn call(&mut self, participant: Pointer, slot: u32, arguments: &[u32]) -> u32;
    fn current_city(&mut self, application: Pointer, slot: u32) -> Pointer;
    fn clock(&mut self, city: Pointer, slot: u32) -> Pointer;
    fn release(&mut self, participant: Pointer, slot: u32) -> u32;
}

/// Four addressed, intact bytes, independent of object allocation or identity.
/// Implementations must make offsets 0..=3 readable/writable and preserve stores.
/// This boundary also permits externally owned storage without unsafe memory.
pub trait Storage {
    fn read_byte(&self, offset: usize) -> u8;
    fn write_byte(&mut self, offset: usize, value: u8);
}

impl Storage for [u8; 4] {
    fn read_byte(&self, offset: usize) -> u8 {
        self[offset]
    }

    fn write_byte(&mut self, offset: usize, value: u8) {
        self[offset] = value;
    }
}

/// The class owns only its four addressed bytes, not service pointers or state.
/// No Clone, Copy, default selection, active bit, Drop, or reference count.
pub struct AutoGamePause<S: Storage = [u8; 4]> {
    storage: S,
}

impl<S: Storage> AutoGamePause<S> {
    /// Supply storage, without entering or asserting a native allocation size.
    /// Call construct, then destruct once, with intact storage and serial callbacks.
    pub fn from_storage(storage: S) -> Self {
        Self { storage }
    }

    /// Store all four low bytes before any foreign callback; return this object.
    pub fn construct(&mut self, host: &mut impl Host, flags: [u32; 4]) -> &mut Self {
        let operands = self.store_entry(flags);
        sc4_game_pause(host, operands);
        self
    }

    /// Read current bytes in reverse address order, then request exit. No status,
    /// field modification, storage deallocation, or saved-binding compensation.
    pub fn destruct(&self, host: &mut impl Host) {
        sc4_game_pause(host, self.read_exit());
    }

    // Keep the storage/operand boundary explicit: entry forwards full original
    // words, while exit zero-extends the bytes read in order 3, 2, 1, 0 (§2).
    fn store_entry(&mut self, flags: [u32; 4]) -> [u32; 5] {
        self.storage.write_byte(0, flags[0] as u8);
        self.storage.write_byte(1, flags[1] as u8);
        self.storage.write_byte(2, flags[2] as u8);
        self.storage.write_byte(3, flags[3] as u8);
        [1, flags[0], flags[1], flags[2], flags[3]]
    }

    fn read_exit(&self) -> [u32; 5] {
        let f4 = u32::from(self.storage.read_byte(3));
        let f3 = u32::from(self.storage.read_byte(2));
        let f2 = u32::from(self.storage.read_byte(1));
        let f1 = u32::from(self.storage.read_byte(0));
        [0, f1, f2, f3, f4]
    }
}

/// Separate helper, taking the five original raw32 slots [D, F1, F2, F3, F4].
/// Only low bytes gate selection/direction. All foreign status/reply bits are
/// ignored; effects of earlier stages remain when a later participant is absent.
pub fn sc4_game_pause(host: &mut impl Host, operands: [u32; 5]) {
    let entering = operands[0] as u8 != 0;
    // Literal AGP1 key retained despite its unresolved typed identity (§3.1).
    const KEYS: [(u32, u32); 4] = [
        (0xc416_025c, 0x0073_283c),
        (0x86ad_10ec, 0x86ad_10ed),
        (0x4695_6659, 0x8695_664e),
        (0x66, 0x26ce_01c0),
    ];
    let mut temporary;
    for (stage, &(service, iid)) in KEYS.iter().enumerate() {
        if operands[stage + 1] as u8 == 0 {
            continue;
        }
        temporary = 0;
        let framework = host.framework();
        if framework != 0 {
            host.lookup(framework, 0x14, service, iid, &mut temporary);
        }
        if temporary == 0 {
            continue;
        }
        match stage {
            0 => {
                host.call(temporary, if entering { 0x48 } else { 0x4c }, &[]);
            }
            1 => {
                host.call(temporary, 0x30, &[u32::from(entering)]);
            }
            2 => {
                host.call(temporary, if entering { 0x28 } else { 0x34 }, &[]);
            }
            3 => {
                let city = host.current_city(temporary, 0x94);
                if city != 0 {
                    let clock = host.clock(city, 0x158);
                    if clock != 0 {
                        host.call(clock, 0x68, &[u32::from(entering)]);
                    }
                }
            }
            _ => unreachable!(),
        }
        // Both directions release the same lookup output, never city/clock (§3.2).
        host.release(temporary, 0x08);
    }
}

#[cfg(test)]
mod tests;
