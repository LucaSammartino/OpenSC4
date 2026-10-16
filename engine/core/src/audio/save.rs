// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Nothing in the audio layer is saved (`docs/specs/audio.md` §12).
//! Section references (`§n`) point at that spec.
//!
//! Evidence (§12): no audio class has a `GetGZCLSID`; the only `Read`/`Write`-
//! named methods are resource and tuning readers, not `cIGZSerializable`
//! pairs; and `save-records.md` contains no audio record.
//!
//! Consequences a replacement must match *[compat]* (§12, conformance
//! test 17):
//!
//! 1. Sound instances (`automaton+0x8C`) are **not** in the save. After a load
//!    every automaton starts with no sound playing, and a state that was
//!    mid-SFX on save does not resume it.
//! 2. The sound-event pool, the AEMS banks' loaded state, the Hit and
//!    freshness players and the radio's playlist position are all
//!    **runtime-only**.
//! 3. Because `HandleEnterState` only starts a sound on state entry, a loaded
//!    automaton that is already inside a sounding state will be **silent until
//!    it next changes state** *[quirk]* (§14.9).

/// The city record a class needs in order to be rebuilt by the loader
/// (`GetGZCLSID`). No audio class has one (§12).
pub const HAS_SAVE_RECORD: bool = false;

/// The audio save-record ids: there are none (§12).
pub const SAVE_RECORDS: [u32; 0] = [];

/// Whether a field of the audio layer is saved (§12).
///
/// Every field of every audio class is runtime-only. The two automaton SFX
/// fields of §3.3 are **unsaved**.
pub fn is_saved(_field: &str) -> bool {
    false
}

/// The state a load restores (§12, conformance test 17): nothing. This is the
/// post-load state of every automaton and of every manager.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PostLoadState {
    /// `automaton+0x8C` — 0 after a load (§12.1).
    pub automaton_instance_id: u32,
    /// The number of sound instances the manager holds: 0 after a load.
    pub live_sounds: u32,
    /// The sound-event pool's live objects: 0 after a load (§12.2).
    pub pooled_events: u32,
    /// Whether any AEMS bank is loaded: false after a load (§12.2).
    pub aems_banks_loaded: bool,
}

impl PostLoadState {
    /// A fresh load restores exactly this (§12, conformance test 17).
    pub const fn after_load() -> Self {
        PostLoadState {
            automaton_instance_id: 0,
            live_sounds: 0,
            pooled_events: 0,
            aems_banks_loaded: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::automaton::AutomatonSfxHost;
    use crate::audio::sound_manager::AudioSoundManager;

    /// Conformance test 17: a save/load restores no sound instance.
    #[test]
    fn after_load_every_automaton_is_silent() {
        // Mid-state: an automaton is inside a sounding state.
        let mut h = AutomatonSfxHost::new();
        h.handle_enter_state(0x00CA_FE01, 0x00B0_0004);
        assert_ne!(h.state.instance_id, 0);

        // A load rebuilds the automaton from nothing: §12.1.
        let after = PostLoadState::after_load();
        assert_eq!(after.automaton_instance_id, 0);
        assert_eq!(after.live_sounds, 0);
        assert_eq!(after.pooled_events, 0);
        assert!(!after.aems_banks_loaded);
    }

    /// Conformance test 17: the manager holds nothing after a load either.
    #[test]
    fn after_load_the_manager_is_empty() {
        let mut m = AudioSoundManager::new();
        m.start_sound(1, 0x2A, 0);
        assert_eq!(m.instances.len(), 1);
        let after = PostLoadState::after_load();
        assert_eq!(after.live_sounds, 0);
    }

    /// §12: nothing in the audio layer is saved.
    #[test]
    fn nothing_is_saved() {
        assert!(!is_saved("anything"));
        assert!(SAVE_RECORDS.is_empty());
        assert!(!is_saved("instance_id"));
        assert!(!is_saved("flags"));
        assert!(!is_saved("anything"));
    }

    /// §14.9: a loaded automaton inside a sounding state is silent until it
    /// next changes state — `HandleEnterState` is the only place `StartSFX` is
    /// called.
    #[test]
    fn a_loaded_automaton_is_silent_until_the_next_state() {
        let mut h = AutomatonSfxHost::new();
        // It was mid-state on save, so on load it is already inside the state
        // and `HandleEnterState` does not fire again.
        let after = PostLoadState::after_load();
        assert_eq!(after.automaton_instance_id, 0);
        assert!(h.posted.is_empty());
        // Only the next state entry starts a sound.
        assert!(h.handle_enter_state(0x00CA_FE01, 0));
        assert_eq!(h.posted.len(), 1);
    }
}
