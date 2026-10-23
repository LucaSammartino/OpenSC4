// SPDX-License-Identifier: GPL-3.0-or-later
//! cHitTrackPlayer, sound-system §3. Bytecode execution remains SS1.
use super::{InterfaceView, Properties, Scheduler, SoundRef, Unsupported};
use std::sync::{Arc, Mutex};

pub trait TrackServices {
    fn sound_system(&mut self) -> Option<super::directors::ForeignRef>;
    fn hit_manager(&mut self) -> Option<super::directors::ForeignRef>;
    fn sound_manager(&mut self) -> Option<Arc<super::framework::SndMgr>>;
}

pub const PAYLOAD_PROPERTIES: [u32; 4] = [0xFF14_1C48, 0xFF92_50B3, 0xFF9E_C945, 0xFF18_85BE];

#[derive(Default)]
pub struct HitTrackPlayer {
    pub(super) load_count: u32,
    pub executing: bool,
    pub paused: bool,
    pub(super) current_paused: bool,
    pub error: i32,
    pub properties: Properties,
    pub current: Option<SoundRef>,
    pub dependents: Vec<SoundRef>,
    pub property_notifications: Vec<(u32, i32)>,
    pub embedded_channel: super::hit::HitChannel,
    pub services: Vec<super::directors::ForeignRef>,
}

/// Effects whose values are known before the unsettled interpreter is entered.
#[derive(Debug, PartialEq, Eq)]
pub enum PlayPreparation {
    AlreadyPlaying,
    Execute {
        kill_first: bool,
        routing_mode: Option<i32>,
        payload: [u32; 4],
    },
}

impl HitTrackPlayer {
    pub fn construct(host: &mut dyn TrackServices) -> Self {
        let mut track = Self::default();
        track.services.extend(host.sound_system());
        track.services.extend(host.hit_manager());
        track.embedded_channel.manager = host.sound_manager();
        track
    }
    pub fn query_interface(owner: &Arc<Mutex<Self>>, iid: u32) -> Option<InterfaceView<Self>> {
        let offset = match iid {
            1 | 0xBB1B_E367 => 0,
            0xFD58_4467 => 4,
            0x3060_77D2 => 8,
            0x2060_77D2 => 12,
            0xAA11_A64F => 16,
            0x5B34_F0AF => 20,
            0xDD58_3E2D => 24,
            0xE9D0_0A9A => 28,
            _ => return None,
        };
        Some(InterfaceView {
            owner: Arc::clone(owner),
            offset,
        })
    }

    pub fn load(&mut self) -> bool {
        self.load_count = self.load_count.wrapping_add(1);
        if self.load_count == 1 {
            if let Some(sound) = &self.current {
                sound.load();
            }
        }
        true
    }

    pub fn unload(&mut self) -> bool {
        self.load_count = self.load_count.wrapping_sub(1);
        if self.load_count == 0 {
            if let Some(sound) = &self.current {
                sound.unload();
            }
        }
        true
    }

    pub fn is_loaded(&self) -> bool {
        self.load_count != 0
    }
    pub fn is_playing(&self) -> bool {
        self.executing || self.dependents.iter().any(|sound| sound.is_playing())
    }

    pub fn pause(&mut self, pause_current: bool, scheduler: &mut dyn Scheduler) {
        let playing = self.is_playing();
        self.paused = true;
        for sound in &self.dependents {
            sound.pause();
        }
        if playing {
            scheduler.unregister();
        }
        if pause_current {
            if let Some(sound) = &self.current {
                sound.pause();
                self.current_paused = true;
            }
        }
    }

    pub fn unpause(&mut self, scheduler: &mut dyn Scheduler) -> bool {
        self.paused = false;
        for sound in &self.dependents {
            sound.unpause();
        }
        if self.current_paused {
            if let Some(sound) = &self.current {
                sound.unpause();
            }
            self.current_paused = false;
        }
        if self.is_playing() {
            scheduler.register(20);
        }
        self.error == 0
    }

    /// Only the signed upper-bound check is established (§3.3).
    pub fn checked_reg_id(id: i32, report: &mut dyn FnMut(&str)) -> i32 {
        if id > 74 {
            report("Illegal Register Id");
            1
        } else {
            id
        }
    }
    pub fn priority(&self) -> Option<i32> {
        self.properties.get(&0x13).copied()
    }
    pub fn volume(&self) -> Option<i32> {
        self.properties.get(&0x15).copied()
    }
    pub fn pan(&self) -> Option<i32> {
        self.properties.get(&0x16).copied()
    }
    pub fn original_frequency(&self) -> Option<i32> {
        self.properties.get(&0x49).copied()
    }
    /// Confirmed notification writes, separate from SS1's argument routing.
    pub fn write_play_payload(&mut self, payload: [u32; 4]) {
        for (property, value) in PAYLOAD_PROPERTIES.into_iter().zip(payload) {
            self.properties.insert(property, value as i32);
            self.property_notifications.push((property, value as i32));
        }
    }

    /// A plan, not playback. Kill/start and payload notifications belong to the
    /// unsettled execution protocol SS1; do not apply half a restart here.
    pub fn prepare_play(&self, payload: [u32; 4]) -> PlayPreparation {
        let playing = self.is_playing();
        if playing && self.properties.get(&0xFF30_3B1B).copied() == Some(0) {
            PlayPreparation::AlreadyPlaying
        } else {
            PlayPreparation::Execute {
                kill_first: playing,
                routing_mode: self.properties.get(&0xFF4D_CD0D).copied(),
                payload,
            }
        }
    }
    pub fn play_one(&mut self, payload: u32) -> Result<bool, Unsupported> {
        self.play([payload, 0, 0, 0])
    }
    pub fn play(&mut self, payload: [u32; 4]) -> Result<bool, Unsupported> {
        match self.prepare_play(payload) {
            PlayPreparation::AlreadyPlaying => Ok(true),
            PlayPreparation::Execute { kill_first: false, .. } => {
                self.write_play_payload(payload);
                Err(Unsupported("SS1"))
            }
            // The Kill algorithm itself is SS1, so don't apply new payloads
            // before that required operation has completed.
            PlayPreparation::Execute { kill_first: true, .. } => Err(Unsupported("SS1")),
        }
    }
    pub fn execute_command(&mut self, _data: &[u8]) -> Result<(), Unsupported> {
        Err(Unsupported("SS1"))
    }
}
