// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4AudioSoundManager`, CLSID `0x4A47DC0D` (`docs/specs/audio.md` §8).
//! Section references (`§n`) point at that spec.
//!
//! Object size `0x80` (128 bytes). It owns the running sound instances.
//!
//! **The manager never sees `0x8A385DD8` or `0xAA396960`** *[compat]* (§8,
//! §14.11, conformance test 21): its `DoMessage` has neither in its switch and
//! neither in its subscribe list. It is reached **indirectly**, through
//! sound-system events (`0xEA451903` / `0x6A451908`, §8.3).
//!
//! **Nothing is saved** (§8.5, §12): no `Read`/`Write`/`GetGZCLSID`, and
//! `QueryInterface` answers only `1` and the CLSID.

use std::collections::BTreeMap;

pub mod execution;
/// Settled foreign call stages; full native helper execution: AU46/AU49.
pub mod owner;

use crate::audio::data::{
    SoundManagerTuning, TUNING_CURVE_KEY, TUNING_ONE_SHOT_KEY, TUNING_UPDATE_KEY, TUNING_VOICE_KEY,
};
use crate::audio::instance_id::InstanceIdCounter;
use crate::audio::{
    EVT_SET_POSITION, EVT_START_SOUND, EVT_STOP_SOUND, IID_SND_DATA_SOURCE_HOLDER, MSG_APP_FOCUS, MSG_EXEMPLAR_CHANGED,
    MSG_LISTENER_POS_CHANGED, MSG_POST_APP_INIT, MSG_PRE_APP_SHUTDOWN, MSG_PRE_CITY_SHUTDOWN, MSG_SIM_PAUSE_CHANGE,
    MSG_SIM_SPEED_CHANGE, SOUND_MANAGER_SIZE,
};

/// The object size of `cSC4AudioSoundManager` (§8): 128 bytes.
pub const SOUND_MANAGER_OBJECT_SIZE: usize = SOUND_MANAGER_SIZE;

/// `tSound` — one running sound instance (§8.3, §10.6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TSound {
    /// `+0x04` — the sound id.
    pub sound_id: u32,
    /// `+0x08` — the instance id, the key of the `+0x38` hashtable.
    pub instance_id: u32,
    /// `+0x34` — the pause-for-distance radius, `(param & 0xFF) * 0.01 * 100.0`.
    pub pause_radius: f32,
    /// `+0x38` — `pause_radius * 4.0`.
    pub resume_radius: f32,
    /// `+0x20` / `+0x24` — the soft and hard stop paths' handle fields (§8.3).
    pub soft: u32,
    pub hard: u32,
}

impl TSound {
    /// Allocate a `tSound` as `_StartSound` does (§8.3, §10.6 step 4).
    pub fn new(sound_id: u32, instance_id: u32, param: u32) -> Self {
        // `+0x34 = (param & 0xFF) * 0.01 * 100.0` — the 0.01 and the 100.0
        // cancel, so the field is the low byte of the parameter.
        let pause = f32::from((param & 0xFF) as u8) * 0.01 * 100.0;
        TSound {
            sound_id,
            instance_id,
            pause_radius: pause,
            resume_radius: pause * 4.0,
            soft: 0,
            hard: 0,
        }
    }
}

/// The eight types `DoMessage` handles (§8.3) — and **exactly** these
/// (conformance test 21).
pub fn handled_message_types() -> [u32; 8] {
    [
        MSG_POST_APP_INIT,
        MSG_PRE_APP_SHUTDOWN,
        MSG_PRE_CITY_SHUTDOWN,
        MSG_APP_FOCUS,
        MSG_SIM_SPEED_CHANGE,
        MSG_LISTENER_POS_CHANGED,
        MSG_SIM_PAUSE_CHANGE,
        MSG_EXEMPLAR_CHANGED,
    ]
}

/// What `_StopSound` reports (§8.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopResult {
    /// The instance was found and stopped.
    Stopped,
    /// The instance id is not present: **returns false and does nothing** —
    /// no side effects on the hashtable or the list (§8.3, §14.12).
    Unknown,
}

/// `cSC4AudioSoundManager` (§8).
#[derive(Clone, Debug)]
pub struct AudioSoundManager {
    /// `+0x10` — the refcount.
    pub refcount: u32,
    /// `+0x14` — master volume, default `0x400`.
    pub master_volume: u32,
    /// `+0x18` — cumulative elapsed time.
    pub elapsed: u32,
    /// `+0x1C` — the update counter.
    pub updates: u32,
    /// `+0x30` — the focus-restore countdown, set to 50.
    pub focus_countdown: u32,
    /// `+0x34`/`+0x38`/`+0x3C` — the instance-id hashtable.
    pub instances: BTreeMap<u32, TSound>,
    /// `+0x48`/`+0x4C` — the list of `tSound*` and its size.
    pub list: Vec<u32>,
    /// `+0x54` — the one-shot counter, compared against `+0x68`.
    pub one_shot_counter: u32,
    /// `+0x64`, `+0x68`, `+0x6C`, `+0x70` — the four tuning values of §10.8.
    pub tuning: SoundManagerTuning,
    /// The process-global instance-id counter (§8.4).
    pub ids: InstanceIdCounter,
    /// Whether a tuning read has failed (the original logs it, §10.8).
    pub tuning_log: Vec<String>,
}

impl Default for AudioSoundManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioSoundManager {
    /// A manager as the constructor leaves it (§8.1).
    pub fn new() -> Self {
        AudioSoundManager {
            refcount: 1,
            master_volume: crate::audio::MANAGER_MASTER_VOLUME_DEFAULT,
            elapsed: 0,
            updates: 0,
            focus_countdown: crate::audio::MANAGER_FOCUS_COUNTDOWN,
            instances: BTreeMap::new(),
            list: Vec::new(),
            one_shot_counter: 0,
            tuning: SoundManagerTuning::default(),
            ids: InstanceIdCounter::new(),
            tuning_log: Vec::new(),
        }
    }

    /// `QueryInterface` (`0x004F7B70`, §8.2) answers **`1` and `0x4A47DC0D`**
    /// only — the CLSID doubles as the primary IID — and returns the object
    /// base. Everything else is refused.
    pub fn query_interface(&self, iid: u32) -> bool {
        iid == 1 || iid == crate::audio::CLSID_SOUND_MANAGER
    }

    /// `DoMessage` (`0x004F9430`, §8.3): returns `true` when the type is one
    /// of the eight, `false` otherwise. In particular `0x8A385DD8` and
    /// `0xAA396960` are **not** handled (§14.11, conformance test 21).
    pub fn handles_message(msg_type: u32) -> bool {
        handled_message_types().contains(&msg_type)
    }

    /// `DoMessage` applied: records the side effect of each of the eight
    /// types (§8.3) and reports whether the type was handled.
    pub fn do_message(&mut self, msg_type: u32) -> bool {
        if !Self::handles_message(msg_type) {
            return false;
        }
        match msg_type {
            // `kSC4MessagePostAppInit`: return 1, no-op.
            MSG_POST_APP_INIT => {}
            // `kSC4MessagePreAppShutdown` and `kSC4MessagePreCityShutdown`.
            MSG_PRE_APP_SHUTDOWN | MSG_PRE_CITY_SHUTDOWN => self.stop_all_sounds(0),
            MSG_APP_FOCUS | MSG_SIM_SPEED_CHANGE | MSG_LISTENER_POS_CHANGED | MSG_SIM_PAUSE_CHANGE => {
                self.update_all_sounds();
            }
            MSG_EXEMPLAR_CHANGED => self.read_tuning_values(Some(SoundManagerTuning::default())),
            _ => {}
        }
        true
    }

    /// `DoEvent` (`0x004FA590`, §8.3): the sound-system event dispatcher.
    pub fn do_event(&mut self, event: u32, a: u32, b: u32, param: u32) -> Option<StopResult> {
        match event {
            EVT_START_SOUND => {
                self.start_sound(a, b, param);
                None
            }
            EVT_STOP_SOUND => Some(self.stop_sound(a, b != 0)),
            EVT_SET_POSITION => {
                // `FUN_004FA4C0` -> `tSound::SetPosition`; the receiver
                // converts the integer coordinates back with `(float)`
                // (§14.15).
                None
            }
            _ => None,
        }
    }

    /// `_StartSound` (`0x004F9D70`, `ret 0xC`, §8.3, §10.6).
    ///
    /// If the instance id is already live it is **stopped first**
    /// (`_StopSound(this, id, 1)`); if the caller passed id 0 a fresh one is
    /// taken from [`InstanceIdCounter`] (conformance test 22).
    pub fn start_sound(&mut self, sound_id: u32, instance_id: u32, param: u32) -> u32 {
        // Step 1: a live instance with the same id is stopped first.
        let mut key = instance_id;
        if self.instances.contains_key(&key) {
            self.stop_sound(key, true);
        }
        // Step 2: id 0 takes a fresh one (the counter is inlined here too,
        // §8.4).
        if key == 0 {
            key = self.ids.generate();
        }
        let sound = TSound::new(sound_id, key, param);
        self.instances.insert(key, sound);
        self.list.push(key);
        key
    }

    /// `_StopSound` (`0x004F94C0`, `ret 8`, §8.3).
    ///
    /// On an **unknown instance id it returns false and does nothing**
    /// (`xor al,al; ret 8` at `0x004F94FF`) — no side effects *[quirk]*
    /// (§14.12). Otherwise it unregisters and takes the soft (`hard == false`)
    /// or hard path on the event's second argument; the effect component
    /// always sends 0 (soft).
    pub fn stop_sound(&mut self, instance_id: u32, hard: bool) -> StopResult {
        if !self.instances.contains_key(&instance_id) {
            return StopResult::Unknown;
        }
        self.instances.remove(&instance_id);
        self.list.retain(|&i| i != instance_id);
        let _ = hard; // §8.3: the two paths differ only in how the sample ends.
        StopResult::Stopped
    }

    /// `StopAllSounds` (`0x004F7E90`, §8.3).
    pub fn stop_all_sounds(&mut self, _flags: u32) {
        self.instances.clear();
        self.list.clear();
    }

    /// `UpdateAllSounds` (`0x004F7E20`, §8.3).
    pub fn update_all_sounds(&mut self) {
        self.updates = self.updates.wrapping_add(1);
    }

    /// `_ReadTuningValues` (`0x004F9250`, §10.8, conformance test 19): read
    /// the four keys of the audio settings exemplar and apply **no clamping**
    /// *[compat]*. A failed read keeps the defaults **and logs the failure**.
    ///
    /// `Some(t)` is the exemplar's values; `None` is a failed read.
    pub fn read_tuning_values(&mut self, read: Option<SoundManagerTuning>) {
        match read {
            Some(t) => self.tuning = t,
            None => {
                self.tuning = SoundManagerTuning::default();
                self.tuning_log.push(
                    "cSC4AudioSoundManager::_ReadTuningValues: read of 0x6534284A/0xCA25875D/0x6A359C5C failed"
                        .to_string(),
                );
            }
        }
    }

    /// The four keys §10.8 names, in the order the reader asks for them.
    pub fn tuning_keys() -> [u32; 4] {
        [
            TUNING_CURVE_KEY,
            TUNING_VOICE_KEY,
            TUNING_ONE_SHOT_KEY,
            TUNING_UPDATE_KEY,
        ]
    }
}

/// The sound instance lookup idiom of §10.6: hash bucket `id % bucketCount`,
/// chain on `+0x04`.
pub fn instance_bucket(instance_id: u32, bucket_count: u32) -> u32 {
    if bucket_count == 0 {
        0
    } else {
        instance_id % bucket_count
    }
}

/// `cGZSndMgr::GetSound` (`0x00968AE7`, §8.6): the `0xFF000000` OR is a
/// **sound-id tag** *[inferred]* (AU9). Applied before the lookup.
pub fn tagged_sound_id(sound_id: u32) -> u32 {
    sound_id | 0xFF00_0000
}

/// `kGZIID_cIGZSndDataSourceHolder`, the IID the sample lookup asks for
/// (§8.6, §10.6).
pub fn data_source_holder_iid() -> u32 {
    IID_SND_DATA_SOURCE_HOLDER
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Conformance test 21: the manager does not see the automata messages.
    #[test]
    fn does_not_handle_the_automata_messages() {
        assert!(!AudioSoundManager::handles_message(crate::audio::MSG_AUTOMATA_PLAY_SFX));
        assert!(!AudioSoundManager::handles_message(crate::audio::MSG_AUTOMATA_STOP_SFX));
        let mut m = AudioSoundManager::new();
        assert!(!m.do_message(crate::audio::MSG_AUTOMATA_PLAY_SFX));
        assert!(!m.do_message(crate::audio::MSG_AUTOMATA_STOP_SFX));
        assert!(m.instances.is_empty());
    }

    /// Conformance test 21: `DoMessage` handles exactly the eight types of
    /// §8.3.
    #[test]
    fn handles_exactly_eight_types() {
        let t = handled_message_types();
        assert_eq!(t.len(), 8);
        for x in t {
            assert!(AudioSoundManager::handles_message(x), "0x{x:08X}");
        }
        for x in [0x8A38_5DD8u32, 0xAA39_6960, 0xFA0C_6390, 0x26D3_1EC1] {
            assert!(
                !AudioSoundManager::handles_message(x),
                "0x{x:08X} is not one of the eight"
            );
        }
    }

    /// Conformance test 22: `_StartSound` stops a live instance first and
    /// restarts under the same id.
    #[test]
    fn start_sound_replaces_a_live_instance() {
        let mut m = AudioSoundManager::new();
        let k = m.start_sound(1, 0x2A, 0);
        assert_eq!(k, 0x2A);
        assert_eq!(m.instances.len(), 1);
        assert_eq!(m.list, vec![0x2A]);

        // The same id again: the old one is stopped first, then a new one is
        // started under the same id.
        let k2 = m.start_sound(2, 0x2A, 0);
        assert_eq!(k2, 0x2A);
        assert_eq!(m.instances.len(), 1, "the old instance is gone");
        assert_eq!(m.instances[&0x2A].sound_id, 2, "the new one is under the same id");
        assert_eq!(m.list, vec![0x2A], "the list holds one entry");
    }

    /// Conformance test 22: `_StopSound` on a missing id returns false and
    /// leaves the hashtable and the list unchanged.
    #[test]
    fn stop_sound_on_a_missing_id_is_a_noop() {
        let mut m = AudioSoundManager::new();
        m.start_sound(1, 0x2A, 0);
        assert_eq!(m.stop_sound(0x99, false), StopResult::Unknown);
        assert_eq!(m.instances.len(), 1);
        assert_eq!(m.list, vec![0x2A]);
    }

    /// Conformance test 22: a real stop removes it from both.
    #[test]
    fn stop_sound_removes_the_instance() {
        let mut m = AudioSoundManager::new();
        m.start_sound(1, 0x2A, 0);
        assert_eq!(m.stop_sound(0x2A, false), StopResult::Stopped);
        assert!(m.instances.is_empty());
        assert!(m.list.is_empty());
    }

    /// Conformance test 20: `_StartSound` with id 0 takes a fresh global id.
    #[test]
    fn start_sound_with_zero_takes_a_fresh_id() {
        let mut m = AudioSoundManager::new();
        let a = m.start_sound(1, 0, 0);
        let b = m.start_sound(2, 0, 0);
        assert_eq!((a, b), (1, 2));
        assert_ne!(a, 0);
    }

    /// Conformance test 19: a failed read keeps the defaults and logs.
    #[test]
    fn tuning_defaults_and_the_failure_log() {
        let mut m = AudioSoundManager::new();
        assert_eq!(m.tuning.voice_gate, 3);
        assert_eq!(m.tuning.one_shot, 10);
        assert_eq!(m.tuning.update_period, 30);
        m.read_tuning_values(Some(SoundManagerTuning {
            voice_gate: 7,
            one_shot: 8,
            update_period: 9,
            ..SoundManagerTuning::default()
        }));
        assert_eq!(m.tuning.voice_gate, 7);
        m.read_tuning_values(None);
        assert_eq!(m.tuning.voice_gate, 3, "the defaults come back on a failed read");
        assert_eq!(m.tuning_log.len(), 1, "the failure is logged");
    }

    /// Conformance test 19: no clamping is applied (§10.8).
    #[test]
    fn tuning_is_unclamped() {
        let mut m = AudioSoundManager::new();
        m.read_tuning_values(Some(SoundManagerTuning {
            voice_gate: 0,
            one_shot: u32::MAX,
            update_period: 0,
            ..SoundManagerTuning::default()
        }));
        assert_eq!(m.tuning.voice_gate, 0);
        assert_eq!(m.tuning.one_shot, u32::MAX);
        assert_eq!(m.tuning.update_period, 0);
    }

    /// §8.2: QI answers only `1` and the CLSID.
    #[test]
    fn query_interface_answers_two_iids() {
        let m = AudioSoundManager::new();
        assert!(m.query_interface(1));
        assert!(m.query_interface(crate::audio::CLSID_SOUND_MANAGER));
        assert!(!m.query_interface(0x3AF8_5BB7));
    }

    /// §10.6: the instance-table hash idiom.
    #[test]
    fn instance_bucket_uses_modulo() {
        assert_eq!(instance_bucket(0x2A, 8), 2);
        assert_eq!(instance_bucket(0, 0), 0);
    }

    /// AU9: the `0xFF000000` tag of §8.6.
    #[test]
    fn sound_id_tag_sets_the_top_byte() {
        assert_eq!(tagged_sound_id(0x0012_3456), 0xFF12_3456);
        assert_eq!(tagged_sound_id(0xFF12_3456), 0xFF12_3456);
    }
}
