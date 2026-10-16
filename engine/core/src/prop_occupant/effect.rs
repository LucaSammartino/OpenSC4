// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4PropOccupantVisualEffect` (§6.2), the class `0x2977AA49`.
//!
//! The visual effect is a **separate prop class**, not a member of the plain
//! prop. Its record is `lot-records.md` §10.2 and its behaviour is
//! `network-tools.md` §13.2 (the NM5 answer); both stand. What this module
//! adds is the relationship to the embedded occupant.
//!
//! # Layout (§6.2)
//!
//! The embedded prop occupant is at `+4` (`AsPropOccupant`) and the occupant at
//! `+0xC` (`AsOccupant`). The effect object lives at `+0x6C`; the stored name at
//! `+0x74` / `+0x78`; the stored effect ID at `+0x84`; `+0x88` / `+0x8C` are
//! zeroed on an effect-type change; `started` is `+0x90`; the constructor
//! (`0x005F0210`) sets `+0x91 := 1`. The class is `0x94` bytes.
//!
//! # Choices (`[choice]`) and literal readings
//!
//! - **The second effect handle.** The record quirk says the name is dropped
//!   "whenever the effect is live (**either of its two** effect handles is
//!   non-null)" (`lr` §10.2), but neither §6.2 nor `nt` §13.2 gives the second
//!   handle's offset. `[choice]` FreeCity4 models it as a second handle and
//!   treats "live" as "either is non-null". Question 2 below.
//! - **`StartEffect` with no effect object** sets `started := 1` and returns
//!   false (`nt` §13.2) — the flag is written even on the failure path.
//! - **The name after a reload** comes back from the exemplar
//!   *[inferred]* (§6.2, test 22): the core records that the name was dropped
//!   and leaves the lookup to the caller.

use std::cell::{Cell, RefCell};

use super::{PropOccupant, SharedPropTypeTable, CLSID_PROP_VISUAL_EFFECT, OBJECT_SIZE_VISUAL_EFFECT};

/// The IID `QueryInterface` answers with the object base, together with the
/// class ID (§6.2).
pub const IID_VISUAL_EFFECT: u32 = 0x0982_0792;

/// The byte offset of the embedded prop occupant (`AsPropOccupant`, `nt` §13.2).
pub const OFF_EFFECT_PROP: usize = 0x04;
/// The byte offset of the occupant sub-object (`AsOccupant`, `nt` §13.2).
pub const OFF_EFFECT_OCCUPANT: usize = 0x0C;
/// The effect object (§6.2).
pub const OFF_EFFECT_OBJECT: usize = 0x6C;
/// The stored name (§6.2).
pub const OFF_EFFECT_NAME: usize = 0x74;
/// The stored effect ID (§6.2).
pub const OFF_EFFECT_ID: usize = 0x84;
/// Zeroed on an effect-type change (§6.2).
pub const OFF_EFFECT_SCRATCH: usize = 0x88;
/// `started` (§6.2).
pub const OFF_EFFECT_STARTED: usize = 0x90;
/// Set to 1 by the constructor `0x005F0210` (§6.2).
pub const OFF_EFFECT_CTOR_FLAG: usize = 0x91;

/// What the effect prop needs from the game (§6.2, `nt` §13.2).
pub trait EffectHost {
    /// (Re)create the effect object at `+0x6C` (`0x005EE9C0`), which
    /// `SetEffectType` calls after storing a new type. Returns the handle, or
    /// `None` when no effect object could be made — the case `StartEffect`
    /// answers false for.
    fn restart_effect(&self, id: u32, name: &[u8]) -> Option<u64>;

    /// The effect's slot 3, called with `b = 0` from `StartEffect` when
    /// occupant flag 2 is set (`nt` §13.2 step 1).
    fn effect_slot3(&self, handle: u64);

    /// The effect's slot 4, called with `b = 0` from `StopEffect` when
    /// occupant flag 2 is set (`nt` §13.2 step 1).
    fn effect_slot4(&self, handle: u64);

    /// Occupant flag 2, which gates both inner calls and whose change starts or
    /// stops the effect (§6.2).
    fn flag2(&self) -> bool;
}

/// One `cSC4PropOccupantVisualEffect`. The embedded prop occupant is public so
/// a caller can drive the 24 slots on it as on any other prop.
pub struct VisualEffectProp {
    /// The embedded prop occupant at `+4`.
    pub prop: PropOccupant,
    /// The effect object at `+0x6C`; 0 = none.
    effect: Cell<u64>,
    /// The second effect handle named by the record quirk (`lr` §10.2)
    /// *[inferred]*; 0 = none.
    effect_alt: Cell<u64>,
    /// The stored name at `+0x74` / `+0x78`.
    name: RefCell<Vec<u8>>,
    /// The stored effect ID at `+0x84`.
    effect_id: Cell<u32>,
    /// `+0x88` / `+0x8C`, zeroed on an effect-type change.
    scratch: Cell<[u32; 2]>,
    /// `started` at `+0x90`.
    started: Cell<bool>,
    /// `+0x91`, set to 1 by the constructor.
    ctor_flag: Cell<u8>,
    /// Whether the last `Write` dropped the name and the trailing u32 (the
    /// record quirk, test 22).
    name_dropped_on_write: Cell<bool>,
}

impl Default for VisualEffectProp {
    fn default() -> Self {
        Self::new(super::shared_type_table())
    }
}

impl VisualEffectProp {
    /// A freshly constructed effect prop (§6.2, `0x005F0210`): `+0x91 := 1`.
    pub fn new(types: SharedPropTypeTable) -> Self {
        Self {
            prop: PropOccupant::new(types),
            effect: Cell::new(0),
            effect_alt: Cell::new(0),
            name: RefCell::new(Vec::new()),
            effect_id: Cell::new(0),
            scratch: Cell::new([0; 2]),
            started: Cell::new(false),
            ctor_flag: Cell::new(1),
            name_dropped_on_write: Cell::new(false),
        }
    }

    /// The class ID (§1.2).
    pub fn class_id(&self) -> u32 {
        CLSID_PROP_VISUAL_EFFECT
    }

    /// The object size in bytes (§1.2, test 3).
    pub fn object_size(&self) -> usize {
        OBJECT_SIZE_VISUAL_EFFECT
    }

    /// `QueryInterface` (`0x005ECC10`, §6.2): IIDs [`IID_VISUAL_EFFECT`] and
    /// the class ID answer the **object base**; anything else is answered by
    /// the embedded prop occupant's own `QueryInterface`, which is what a
    /// caller reaching through `AsPropOccupant` would use.
    pub fn query_interface(iid: u32) -> Option<usize> {
        if iid == IID_VISUAL_EFFECT || iid == CLSID_PROP_VISUAL_EFFECT {
            Some(0)
        } else {
            None
        }
    }

    /// `AsOccupant` (slot 3), channel **`eax`**: `this + 0xC` (`nt` §13.2).
    pub fn as_occupant_offset() -> usize {
        OFF_EFFECT_OCCUPANT
    }

    /// `AsPropOccupant` (slot 4), channel **`eax`**: `this + 4` (`nt` §13.2).
    pub fn as_prop_occupant_offset() -> usize {
        OFF_EFFECT_PROP
    }

    /// Whether the effect is live: either effect handle is non-null
    /// (`lr` §10.2). This is the condition under which `Write` drops the name.
    pub fn is_live(&self) -> bool {
        self.effect.get() != 0 || self.effect_alt.get() != 0
    }

    /// The effect object handle at `+0x6C`, 0 when there is none (test 21).
    pub fn effect_handle(&self) -> u64 {
        self.effect.get()
    }

    /// Install an effect object at `+0x6C` (what a host does when the effect
    /// manager makes one).
    pub fn set_effect_handle(&self, handle: u64) {
        self.effect.set(handle);
    }

    /// The second effect handle (`lr` §10.2) *[inferred]*.
    pub fn effect_alt_handle(&self) -> u64 {
        self.effect_alt.get()
    }

    /// Install the second effect handle.
    pub fn set_effect_alt_handle(&self, handle: u64) {
        self.effect_alt.set(handle);
    }

    /// The stored effect ID at `+0x84`.
    pub fn effect_id(&self) -> u32 {
        self.effect_id.get()
    }

    /// The stored name at `+0x74` / `+0x78`.
    pub fn effect_name(&self) -> Vec<u8> {
        self.name.borrow().clone()
    }

    /// `started` at `+0x90`.
    pub fn is_started(&self) -> bool {
        self.started.get()
    }

    /// `+0x91`, set to 1 by the constructor.
    pub fn ctor_flag(&self) -> u8 {
        self.ctor_flag.get()
    }

    /// The `+0x88` / `+0x8C` scratch pair, zeroed on an effect-type change.
    pub fn scratch(&self) -> [u32; 2] {
        self.scratch.get()
    }

    /// Whether the last `Write` dropped the name and the trailing u32 (the
    /// record quirk, test 22).
    pub fn name_dropped_on_write(&self) -> bool {
        self.name_dropped_on_write.get()
    }

    /// `SetEffectType(uint32 id)` (slot 5, `nt` §13.2), channel **`void`**:
    /// when `id` differs from `+0x84`, clear the stored name, store `id`, zero
    /// `+0x88` / `+0x8C`, and restart the effect.
    pub fn set_effect_type_id(&self, id: u32, host: &dyn EffectHost) -> bool {
        if id == self.effect_id.get() {
            return false;
        }
        self.name.borrow_mut().clear();
        self.effect_id.set(id);
        self.scratch.set([0; 2]);
        self.restart(host);
        true
    }

    /// `SetEffectType(char const* name)` (slot 6, `nt` §13.2), channel
    /// **`void`**: when `name` differs from the stored name, set the stored ID
    /// to 0, store `name`, and restart.
    pub fn set_effect_type_name(&self, name: &[u8], host: &dyn EffectHost) -> bool {
        if name == self.name.borrow().as_slice() {
            return false;
        }
        self.effect_id.set(0);
        *self.name.borrow_mut() = name.to_vec();
        self.restart(host);
        true
    }

    fn restart(&self, host: &dyn EffectHost) {
        let id = self.effect_id.get();
        let name = self.name.borrow().clone();
        self.effect.set(host.restart_effect(id, &name).unwrap_or(0));
    }

    /// `StartEffect(b)` (slot 7, `nt` §13.2), channel **`eax`**:
    ///
    /// - with an effect object at `+0x6C`: if occupant flag 2 is set, call the
    ///   effect's slot 3 with `b = 0`; `started := 1`; return **true**;
    /// - with none: `started := 1`; return **false**.
    ///
    /// So the call returns true only when an effect object exists at `+0x6C`
    /// (test 21).
    ///
    /// [abi-ret] Slot 7 is a real `eax` 0/1 (`nt` §13.2); the SDK bool reads
    /// AL only and the measured channel is not `al`, so a host must take the
    /// full EAX value here.
    pub fn start_effect(&self, _b: bool, host: &dyn EffectHost) -> bool {
        let handle = self.effect.get();
        if handle == 0 {
            self.started.set(true);
            return false;
        }
        if host.flag2() {
            host.effect_slot3(handle);
        }
        self.started.set(true);
        true
    }

    /// `StopEffect(b)` (slot 8, `nt` §13.2), channel **`eax`**: with an effect
    /// object **and** `started` set, call the effect's slot 4 when flag 2 is
    /// set, clear `started`, and return true; otherwise return false (test 21).
    ///
    /// [abi-ret] Slot 8 is a real `eax` 0/1 (`nt` §13.2), not an `al` bool.
    pub fn stop_effect(&self, _b: bool, host: &dyn EffectHost) -> bool {
        let handle = self.effect.get();
        if handle == 0 || !self.started.get() {
            return false;
        }
        if host.flag2() {
            host.effect_slot4(handle);
        }
        self.started.set(false);
        true
    }

    /// When occupant flag 2 changes (`SetAllFlags`, `0x005ED9D0`) the prop
    /// starts or stops its effect (§6.2).
    pub fn on_flag2_changed(&self, set: bool, host: &dyn EffectHost) -> Option<bool> {
        if set {
            Some(self.start_effect(true, host))
        } else {
            Some(self.stop_effect(true, host))
        }
    }

    /// The `Write` decision of the record quirk (`lr` §10.2): a live effect
    /// prop saves an **empty** name and 0 in place of the name and the trailing
    /// u32. Answers what the caller must emit and records the decision.
    ///
    /// [abi-ret] Not a game-method return; recorded here so the quirk has one
    /// owner. Returns `true` when the name was dropped.
    pub fn should_drop_name_on_write(&self) -> bool {
        let drop = self.is_live();
        self.name_dropped_on_write.set(drop);
        drop
    }
}
