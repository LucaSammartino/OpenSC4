// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The automaton's SFX slots and the messages they post (`docs/specs/audio.md`
//! §3.3, §6). Section references (`§n`) point at that spec.
//!
//! The state base record's `+0x18` is the **SFX id**; id 0 means "this state
//! makes no sound" and neither slot is called *[compat]* (§6.1).
//!
//! Both messages are posted **queued, at normal priority** through the message
//! server global `DAT_00B43CCC`, slot 4: `MessagePost(m, /*highPriority*/
//! false)`. Delivery is therefore asynchronous — `StartSFX` and `StopSFX`
//! return before any sound is started or stopped *[compat]* (§6.4, conformance
//! test 10).

use crate::audio::instance_id::InstanceIdCounter;
use crate::audio::message::StandardMessage;
use crate::audio::{
    AUTOMATON_FLAGS_OFFSET, AUTOMATON_INSTANCE_OFFSET, MSG_AUTOMATA_PLAY_SFX, MSG_AUTOMATA_STOP_SFX, SLOT_START_SFX,
    SLOT_STOP_SFX, STATE_SFX_OFFSET,
};

/// `cSC4Automaton+0x8C` — the sound instance id (§3.3). **Unsaved** (§12).
///
/// `StartSFX` writes it; `StopSFX` reads it and **never clears it** (§14.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutomatonSfx {
    /// `+0x8C` — the sound instance id.
    pub instance_id: u32,
    /// `+0x84` — the automaton's flag word, copied into `state+0x20` on state
    /// entry (§3.3).
    pub flags: u32,
}

impl AutomatonSfx {
    /// An automaton with no sound playing.
    pub const fn new() -> Self {
        AutomatonSfx {
            instance_id: 0,
            flags: 0,
        }
    }
}

/// The message `StartSFX` and `StopSFX` build (§6.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SfxMessage {
    /// `+0x0C` — the type: `0x8A385DD8` play, `0xAA396960` stop.
    pub msg_type: u32,
    /// `+0x10` — the SFX id.
    pub data1: u32,
    /// `+0x14` — the instance id (`automaton+0x8C`).
    pub data2: u32,
    /// `+0x24` — the automaton's occupant (automaton slot 5, `AsOccupant`).
    pub void1: u64,
    /// The message-server `highPriority` argument: always 0 (§6.4).
    pub high_priority: u32,
    /// The presence flags the four setters leave: `0x87` (§6.4).
    pub flags: u8,
}

impl SfxMessage {
    /// Build the message of §6.4 from the four values the automaton writes.
    fn build(msg_type: u32, sfx_id: u32, instance_id: u32, occupant: u64) -> Self {
        let mut m = StandardMessage::new();
        m.add_ref();
        m.set_type(msg_type);
        m.set_void1(occupant);
        m.set_data1(sfx_id);
        m.set_data2(instance_id);
        SfxMessage {
            msg_type: m.msg_type,
            data1: m.data1,
            data2: m.data2,
            void1: m.void1,
            high_priority: 0,
            flags: m.flags,
        }
    }
}

/// Where a posted message went: the message server's `MessagePost` (queued),
/// never `MessageSend` (§6.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    /// The message was posted queued at normal priority (§6.4).
    Queued,
    /// Nothing was posted.
    Nothing,
}

/// The automaton side of §6: `StartSFX` (slot 61) and `StopSFX` (slot 62).
#[derive(Clone, Debug, Default)]
pub struct AutomatonSfxHost {
    /// `+0x8C` / `+0x84`.
    pub state: AutomatonSfx,
    /// The messages posted so far, in order.
    pub posted: Vec<SfxMessage>,
    /// The process-global instance-id counter (§8.4).
    pub ids: InstanceIdCounter,
}

impl AutomatonSfxHost {
    /// A host with a fresh counter.
    pub fn new() -> Self {
        AutomatonSfxHost {
            state: AutomatonSfx::new(),
            posted: Vec::new(),
            ids: InstanceIdCounter::new(),
        }
    }

    /// `cSC4Automaton::StartSFX` (slot 61, `+0xF4`, `0x00509950`): `ret 4`,
    /// returns **1** on success and **0** when the id is 0 (§6.2).
    ///
    /// *[quirk]* The instance id is overwritten unconditionally, without
    /// stopping whatever was playing (§14.2). *[quirk]* The block at
    /// `0x00509987` is unreachable dead code (§14.6); the argument is a `u32`
    /// SFX id, not a refcounted object.
    pub fn start_sfx(&mut self, sfx_id: u32, occupant: u64) -> u32 {
        // Step 2: if the id is 0, return 0 immediately.
        if sfx_id == 0 {
            return 0;
        }
        // Step 4: GenerateNewInstanceId, stored at +0x8C.
        let instance = self.ids.generate();
        self.state.instance_id = instance;
        // Steps 5-11: build the message and post it queued at normal priority.
        let msg = SfxMessage::build(MSG_AUTOMATA_PLAY_SFX, sfx_id, instance, occupant);
        self.posted.push(msg);
        // Steps 12-13.
        1
    }

    /// `cSC4Automaton::StopSFX` (slot 62, `+0xF8`, `0x00505B50`): `ret 4`,
    /// **always returns 1** (§6.3).
    ///
    /// *[quirk]* With `+0x8C == 0` it posts nothing and still returns 1
    /// (§14.3). *[quirk]* It never clears `+0x8C`, so calling it twice posts
    /// two messages carrying the same instance id (§14.1).
    pub fn stop_sfx(&mut self, sfx_id: u32, occupant: u64) -> u32 {
        let instance = self.state.instance_id;
        // Step 1: if +0x8C is 0, skip to return 1.
        if instance == 0 {
            return 1;
        }
        let msg = SfxMessage::build(MSG_AUTOMATA_STOP_SFX, sfx_id, instance, occupant);
        self.posted.push(msg);
        1
    }

    /// `cSC4Automaton::HandleEnterState` (`0x00511450`): call slot 61 with
    /// `state+0x18` **only when that field is non-zero** (§6.1, conformance
    /// test 11). Returns whether the slot was called.
    pub fn handle_enter_state(&mut self, sfx_id: u32, occupant: u64) -> bool {
        if sfx_id == 0 {
            return false;
        }
        self.start_sfx(sfx_id, occupant);
        true
    }

    /// `cSC4Automaton::HandleExitState` (`0x0050E3F0`): call slot 62 with the
    /// same field under the same guard (§6.1, conformance test 11). Returns
    /// whether the slot was called.
    pub fn handle_exit_state(&mut self, sfx_id: u32, occupant: u64) -> bool {
        if sfx_id == 0 {
            return false;
        }
        self.stop_sfx(sfx_id, occupant);
        true
    }
}

/// The slot index of `StartSFX` (§6.1).
pub fn start_sfx_slot() -> u32 {
    SLOT_START_SFX
}

/// The slot index of `StopSFX` (§6.1).
pub fn stop_sfx_slot() -> u32 {
    SLOT_STOP_SFX
}

/// The SFX id field of the state base record (§6.1).
pub fn state_sfx_id(state_base: u32) -> u32 {
    state_base.wrapping_add(STATE_SFX_OFFSET)
}

/// The automaton's instance-id field (§3.3).
pub fn instance_offset() -> u32 {
    AUTOMATON_INSTANCE_OFFSET
}

/// The automaton's flag-word field (§3.3).
pub fn flags_offset() -> u32 {
    AUTOMATON_FLAGS_OFFSET
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Conformance test 7: `StartSFX(0)` returns 0, allocates no message and
    /// leaves `+0x8C` unchanged.
    #[test]
    fn start_sfx_with_id_zero_does_nothing() {
        let mut h = AutomatonSfxHost::new();
        h.state.instance_id = 0x99;
        assert_eq!(h.start_sfx(0, 0x00B0_0000), 0);
        assert!(h.posted.is_empty(), "no message is allocated");
        assert_eq!(h.state.instance_id, 0x99, "+0x8C is unchanged");
    }

    /// Conformance test 8: the play message shape and a fresh instance id.
    #[test]
    fn start_sfx_message_shape() {
        let mut h = AutomatonSfxHost::new();
        assert_eq!(h.start_sfx(0x00CA_FE01, 0x00B0_0004), 1);
        assert_eq!(h.posted.len(), 1);
        let m = &h.posted[0];
        assert_eq!(m.msg_type, MSG_AUTOMATA_PLAY_SFX);
        assert_eq!(m.data1, 0x00CA_FE01, "data1 = the SFX id");
        assert_eq!(m.data2, 1, "data2 = the new instance id");
        assert_eq!(m.void1, 0x00B0_0004, "void1 = the automaton occupant");
        assert_eq!(m.high_priority, 0, "MessagePost(..., 0): normal priority");
        assert_eq!(m.flags, 0x87);
        assert_eq!(h.state.instance_id, 1, "stored at +0x8C");
    }

    /// Conformance test 9: the stop guard, the stop shape, and that `+0x8C` is
    /// not cleared.
    #[test]
    fn stop_sfx_guard_and_shape() {
        let mut h = AutomatonSfxHost::new();
        assert_eq!(
            h.stop_sfx(0x00CA_FE02, 0x00B0_0004),
            1,
            "returns 1 even with nothing to do"
        );
        assert!(h.posted.is_empty(), "with +0x8C == 0 nothing is posted");

        let mut h = AutomatonSfxHost::new();
        h.start_sfx(0x00CA_FE01, 0x00B0_0004);
        h.posted.clear();
        assert_eq!(h.stop_sfx(0x00CA_FE02, 0x00B0_0004), 1);
        assert_eq!(h.posted.len(), 1);
        let m = &h.posted[0];
        assert_eq!(m.msg_type, MSG_AUTOMATA_STOP_SFX);
        assert_eq!(m.data1, 0x00CA_FE02, "data1 = the argument it was called with");
        assert_eq!(m.data2, 1, "data2 = automaton+0x8C");
        assert_eq!(m.void1, 0x00B0_0004);
        assert_eq!(m.high_priority, 0);
        assert_eq!(h.state.instance_id, 1, "+0x8C is not cleared (§14.1)");
    }

    /// §14.1: calling `StopSFX` twice posts two messages with the same
    /// instance id.
    #[test]
    fn stop_sfx_twice_repeats_the_same_instance() {
        let mut h = AutomatonSfxHost::new();
        h.start_sfx(1, 0);
        h.stop_sfx(1, 0);
        h.stop_sfx(1, 0);
        assert_eq!(h.posted.len(), 3);
        assert_eq!(h.posted[1].data2, h.posted[2].data2);
    }

    /// §14.2: a second `StartSFX` orphans the first instance id.
    #[test]
    fn start_sfx_overwrites_without_stopping() {
        let mut h = AutomatonSfxHost::new();
        h.start_sfx(1, 0);
        h.start_sfx(2, 0);
        assert_eq!(h.posted.len(), 2);
        assert_eq!(h.posted[0].data2, 1);
        assert_eq!(h.posted[1].data2, 2);
        assert_eq!(h.state.instance_id, 2, "only the second id remains in +0x8C");
    }

    /// Conformance test 11: the state entry / exit guards.
    #[test]
    fn state_entry_and_exit_are_guarded_on_non_zero() {
        let mut h = AutomatonSfxHost::new();
        assert!(!h.handle_enter_state(0, 0), "id 0 never reaches slot 61");
        assert!(!h.handle_exit_state(0, 0), "id 0 never reaches slot 62");
        assert!(h.posted.is_empty());

        assert!(h.handle_enter_state(7, 0x00B0_0008));
        assert!(h.handle_exit_state(7, 0x00B0_0008));
        assert_eq!(h.posted.len(), 2);
        assert_eq!(h.posted[0].msg_type, MSG_AUTOMATA_PLAY_SFX);
        assert_eq!(h.posted[1].msg_type, MSG_AUTOMATA_STOP_SFX);
        assert_eq!(h.posted[0].data1, 7);
        assert_eq!(h.posted[1].data1, 7);
    }

    /// Conformance test 10: both posts are queued at normal priority — the
    /// message-server argument is 0, so it is `MessagePost`, not `MessageSend`.
    #[test]
    fn both_posts_are_queued_at_normal_priority() {
        let mut h = AutomatonSfxHost::new();
        h.start_sfx(1, 0);
        h.stop_sfx(1, 0);
        for m in &h.posted {
            assert_eq!(m.high_priority, 0);
        }
    }

    /// The slot numbers of §6.1.
    #[test]
    fn slots_are_61_and_62() {
        assert_eq!(start_sfx_slot(), 61);
        assert_eq!(stop_sfx_slot(), 62);
    }
}
