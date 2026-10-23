// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4AudioCOMDirector` and its registration order (`docs/specs/audio.md`
//! §1.1). Section references (`§n`) point at that spec.
//!
//! `cSC4AudioCOMDirector` is the one `cSC4App::AddCOMDirectors` (`0x004493B0`)
//! adds; it adds the other four from its `OnStart`.
//!
//! The five directors are boundaries in this spec (§2): Hit, radio and
//! freshness are named so an implementer can see the edge and are left to a
//! later spec (**AU8**). This module records the identities and the
//! registration order only.

/// The director id `GetDirectorID` (`0x004EB1A0`) returns.
pub const DIRECTOR_ID: u32 = 0xFDB5_CA17;

/// One of the five audio directors (§1.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Director {
    /// The director's id (§1.1).
    pub id: u32,
    /// The name.
    pub name: &'static str,
    /// How many classes its constructor registers (§1.1).
    pub class_count: u32,
    /// Whether this spec details it (§2).
    pub detailed: bool,
}

/// `cSC4AudioCOMDirector` (§1.1): 92 classes, the one the app adds.
pub const AUDIO_DIRECTOR: Director = Director {
    id: DIRECTOR_ID,
    name: "cSC4AudioCOMDirector",
    class_count: 92,
    detailed: true,
};

/// `cGZSoundCOMDirector` (§1.1): 8 classes.
pub const GZ_SOUND_DIRECTOR: Director = Director {
    id: 0xE9D3_A40B,
    name: "cGZSoundCOMDirector",
    class_count: 8,
    detailed: true,
};

/// `cHitManCOMDirector` (§1.1): 10 classes; HIT owners in `sound_system`.
pub const HIT_MAN_DIRECTOR: Director = Director {
    id: 0x9B2D_3D2F,
    name: "cHitManCOMDirector",
    class_count: 10,
    detailed: true,
};

/// `cFreshnessCOMDirector` (§1.1): 5 classes; owners in `sound_system`.
pub const FRESHNESS_DIRECTOR: Director = Director {
    id: 0xFD53_FF17,
    name: "cFreshnessCOMDirector",
    class_count: 5,
    detailed: true,
};

/// `cGZSndSysMacCOMDirector` (§1.1): added by `cSC4AudioCOMDirector::OnStart`.
pub const GZ_SND_SYS_MAC_DIRECTOR: Director = Director {
    id: 0xE9D3_A016,
    name: "cGZSndSysMacCOMDirector",
    class_count: 5,
    detailed: true,
};

/// `cSC4AudioCOMDirector::OnStart` (`0x004EB1B0`) registers the
/// sub-directors **in this order**, calling each one's `OnStart` (slot 4,
/// `+0x10`) immediately after adding it *[compat]* (§1.1, conformance
/// test 16):
///
/// 1. `GetGZSoundCOMDirector()`
/// 2. `GetGZSndSysMacCOMDirector()`
/// 3. `cGZToolsCOMDirector::Get()`
/// 4. `GetHitManCOMDirector()`
/// 5. `cFreshnessCOMDirector::GetDirector()`
///
/// `cGZToolsCOMDirector` is a framework director, not one of §1.1's five; it
/// is in the order because the measured call sequence has it there.
pub const ON_START_ORDER: [&str; 5] = [
    "GetGZSoundCOMDirector",
    "GetGZSndSysMacCOMDirector",
    "cGZToolsCOMDirector::Get",
    "GetHitManCOMDirector",
    "cFreshnessCOMDirector::GetDirector",
];

/// The five directors of §1.1, in the table's order.
pub const DIRECTORS: [Director; 4] = [AUDIO_DIRECTOR, GZ_SOUND_DIRECTOR, HIT_MAN_DIRECTOR, FRESHNESS_DIRECTOR];

/// `cSC4AudioCOMDirector::PostAppInit` (`0x004EBF40`) brings up the SC4 audio
/// services: it loads the AEMS banks under group `0x8A5971C5` (with a second
/// key `0xEA8A1115`) and creates the system services of §1.3 (§1.1).
///
/// Those two keys are exactly the audio configuration record's group and
/// instance ([`crate::audio::data`]).
pub fn post_app_init_keys() -> (u32, u32) {
    (crate::audio::data::CONFIG_GROUP, crate::audio::data::CONFIG_INSTANCE)
}

/// The service IIDs of §1.3 that `PostAppInit` registers.
pub const SERVICE_IDS: [(u32, &str); 4] = [
    (0xAA12_F911, "kGZGUID_AudioAmbienceManagerSystemServiceId"),
    (0x49E5_0BA1, "kAudioScapeManagerSystemServiceID"),
    (0xAC1F_AE31, "GZGUID_AEMSEventsSystemServiceId"),
    (0x1D07_B39F, "kRZRadioDefaultServiceId"),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Conformance test 16: the `OnStart` order of §1.1.
    #[test]
    fn on_start_order() {
        assert_eq!(ON_START_ORDER.len(), 5);
        assert_eq!(ON_START_ORDER[0], "GetGZSoundCOMDirector");
        assert_eq!(ON_START_ORDER[1], "GetGZSndSysMacCOMDirector");
        assert_eq!(ON_START_ORDER[2], "cGZToolsCOMDirector::Get");
        assert_eq!(ON_START_ORDER[3], "GetHitManCOMDirector");
        assert_eq!(ON_START_ORDER[4], "cFreshnessCOMDirector::GetDirector");
    }

    /// §1.1: the director ids.
    #[test]
    fn director_ids() {
        assert_eq!(AUDIO_DIRECTOR.id, 0xFDB5_CA17);
        assert_eq!(GZ_SOUND_DIRECTOR.id, 0xE9D3_A40B);
        assert_eq!(HIT_MAN_DIRECTOR.id, 0x9B2D_3D2F);
        assert_eq!(FRESHNESS_DIRECTOR.id, 0xFD53_FF17);
    }

    /// §1.1: the class counts: 92 + 8 + 10 + 5.
    #[test]
    fn class_counts() {
        assert_eq!(AUDIO_DIRECTOR.class_count, 92);
        assert_eq!(
            AUDIO_DIRECTOR.class_count,
            9 + crate::audio::leaf::LEAF_REGISTRATIONS.len() as u32,
            "nine named managers plus the 83 leaves (§11)"
        );
    }

    /// §1.1: `PostAppInit` loads the AEMS banks under the configuration
    /// record's keys.
    #[test]
    fn post_app_init_keys_match_the_config_record() {
        assert_eq!(post_app_init_keys(), (0x8A59_71C5, 0xEA8A_1115));
    }
}
