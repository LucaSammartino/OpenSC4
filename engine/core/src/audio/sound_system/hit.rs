// SPDX-License-Identifier: GPL-3.0-or-later
//! HIT manager/channel and initialization resources (§§4–5).
use super::{Properties, SoundRef, Unsupported};
use std::collections::BTreeMap;
use std::sync::Arc;

pub trait HitServices {
    fn install_default_semantics(&mut self);
    fn acquire_sound_and_persist(&mut self);
}

pub struct HitMan {
    pub service_id: u32,
    pub priority: u32,
    pub paused: bool,
    pub random_seed: u32,
    pub initialized: bool,
    channels: Vec<SoundRef>,
}
impl Default for HitMan {
    fn default() -> Self {
        Self {
            service_id: 0x1B2C_E671,
            priority: 0x7A116,
            paused: false,
            random_seed: u32::MAX,
            initialized: false,
            channels: Vec::new(),
        }
    }
}
impl HitMan {
    pub const CLSID: u32 = 0x7B1B_D993;
    pub fn init(&mut self, host: &mut dyn HitServices) {
        host.install_default_semantics();
        host.acquire_sound_and_persist();
        self.initialized = true;
    }
    /// The factory performs sound QI; None includes allocation/QI failure.
    /// Choice explicitly allowed by §4.1: reject instead of crashing.
    pub fn alloc_channel(&mut self, factory: impl FnOnce() -> Option<SoundRef>) -> Option<SoundRef> {
        let sound = factory()?;
        self.channels.push(Arc::clone(&sound));
        Some(sound)
    }
    pub fn free_channel(&mut self, sound: &SoundRef) {
        if let Some(index) = self.channels.iter().position(|s| Arc::ptr_eq(s, sound)) {
            self.channels.remove(index);
        }
    }
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }
    /// File grammar, registration, random/arbitration, pause and shutdown SS2.
    pub fn unresolved_operation(&self) -> Result<(), Unsupported> {
        Err(Unsupported("SS2"))
    }
}

/// Native backend creation/QI is foreign; configuration words remain raw
/// properties because their numeric/spatial interpretation is SS3.
pub trait ChannelBackend {
    fn create(&mut self, class_id: u32, sound_iid: u32) -> Option<SoundRef>;
    fn initialize(&mut self, sound: &SoundRef);
    fn query_source(&mut self, sound: &SoundRef, iid: u32) -> bool;
    fn configure_3d(&mut self, sound: &SoundRef, iid: u32, properties: &Properties) -> bool;
    fn refresh_source_state(&mut self, sound: &SoundRef);
}

pub struct HitChannel {
    pub play_value: u32,
    pub properties: Properties,
    pub source: Option<SoundRef>,
    pub manager: Option<Arc<super::framework::SndMgr>>,
}
impl Default for HitChannel {
    fn default() -> Self {
        Self {
            play_value: 0x400,
            properties: Properties::new(),
            source: None,
            manager: None,
        }
    }
}
impl HitChannel {
    pub const CLSID: u32 = 0x3B1E_A5BE;
    pub fn new(manager: Arc<super::framework::SndMgr>) -> Self {
        Self {
            manager: Some(manager),
            ..Self::default()
        }
    }
    pub fn create_sound(&mut self, backend: &mut dyn ChannelBackend) -> bool {
        let spatial = self.properties.get(&0xFFCC_156F).copied().unwrap_or(0) != 0;
        let Some(manager) = &self.manager else {
            return false;
        };
        let Some(class_id) = (if spatial {
            manager.default_3d
        } else {
            manager.default_2d
        }) else {
            return false;
        };
        let Some(sound) = backend.create(class_id, 0x2060_77D2) else {
            return false;
        };
        backend.initialize(&sound);
        if !backend.query_source(&sound, 0x09D0_02D2) {
            return false;
        }
        if spatial && !backend.configure_3d(&sound, 0x3060_77D2, &self.properties) {
            return false;
        }
        self.source = Some(sound);
        true
    }
    /// Full patch selection is SS3. A caller cannot claim a patch refresh
    /// succeeded merely because it supplied a sound. The continuation methods
    /// below accept the explicit result of that still-unsettled step.
    pub fn update_patch(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported("SS3"))
    }
    pub fn load_after_patch(
        &mut self,
        patch: Result<(), Unsupported>,
        backend: &mut dyn ChannelBackend,
    ) -> Result<bool, Unsupported> {
        patch?;
        Ok(match &self.source {
            None => true,
            Some(sound) => {
                let loaded = sound.load();
                backend.refresh_source_state(sound);
                loaded
            }
        })
    }
    pub fn note_on_after_patch(
        &mut self,
        payload: u32,
        patch: Result<(), Unsupported>,
        backend: &mut dyn ChannelBackend,
    ) -> Result<bool, Unsupported> {
        if !self.load_after_patch(patch, backend)? {
            return Ok(false);
        }
        Ok(self.source.as_ref().map_or(true, |sound| {
            sound.play(
                payload,
                self.properties.get(&0xFFF7_B8DF).copied().unwrap_or(0) != 0,
                true,
            )
        }))
    }
    pub fn load(&mut self, backend: &mut dyn ChannelBackend) -> Result<bool, Unsupported> {
        let patch = self.update_patch();
        self.load_after_patch(patch, backend)
    }
    pub fn note_on(&mut self, payload: u32, backend: &mut dyn ChannelBackend) -> Result<bool, Unsupported> {
        let patch = self.update_patch();
        self.note_on_after_patch(payload, patch, backend)
    }
    pub fn update_pitch_or_pan(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported("SS3"))
    }
}

/// Existing framework INI implementation, including its number converter.
/// No duplicate parser or guessed hexadecimal/duplicate-key rules.
pub trait IniFile {
    fn set_filename(&mut self, path: &str);
    fn read_entry(&mut self, section: &str, key: &str) -> Option<String>;
    fn enum_entries(&mut self, section: &str) -> Vec<(String, String)>;
    fn unsigned_number(&mut self, text: &str) -> Result<u32, Unsupported>;
}
pub trait IniServices {
    fn has_manager(&self) -> bool;
    fn has_sound_system(&self) -> bool;
    fn set_default_player(&mut self, spatial: bool, class_id: u32);
    fn map_extension(&mut self, extension: &str, class_id: u32);
}
pub struct HitIniFile<I: IniFile> {
    pub ini: I,
}
impl<I: IniFile> HitIniFile<I> {
    pub const CLSID: u32 = 0x3D80_2D67;
    pub fn set_filename(&mut self, filename: &str) {
        self.ini.set_filename(filename);
    }
    pub fn read_entry(&mut self, section: &str, key: &str) -> Option<String> {
        self.ini.read_entry(section, key)
    }
    pub fn enum_entries(&mut self, section: &str) -> Vec<(String, String)> {
        self.ini.enum_entries(section)
    }
    pub fn enum_default_entries(&mut self, host: &mut dyn IniServices) -> Result<(), Unsupported> {
        if host.has_manager() {
            for (key, spatial) in [
                ("DefaultSndDataSourcePlayer2DClassId", false),
                ("DefaultSndDataSourcePlayer3DClassId", true),
            ] {
                if let Some(value) = self.ini.read_entry("Config", key) {
                    host.set_default_player(spatial, self.ini.unsigned_number(&value)?);
                }
            }
        }
        if host.has_sound_system() {
            for (extension, value) in self.ini.enum_entries("SndFileDefaultDataSourceClassIds") {
                host.map_extension(&extension, self.ini.unsigned_number(&value)?);
            }
        }
        Ok(())
    }
    pub fn init_db_segments(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported("SS4"))
    }
    pub fn init_type_groups(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported("SS4"))
    }
    pub fn init_resources(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported("SS4"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceKey {
    pub kind: u32,
    pub group: u32,
    pub instance: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceFormat {
    Legacy,
    Properties,
}
/// Serialization is not inferred from the prefix; a foreign decoder can be
/// supplied once SS5 settles it. This trait only exposes the confirmed selector.
pub trait TrackResourceIo {
    fn read(&mut self, format: ResourceFormat, properties: &mut Properties) -> Result<bool, Unsupported>;
    fn write(&mut self, format: ResourceFormat, properties: &Properties) -> Result<bool, Unsupported>;
}
pub struct HitPropertySetTrackResource {
    pub key: ResourceKey,
    pub properties: Properties,
}
impl HitPropertySetTrackResource {
    pub fn new(key: ResourceKey) -> Self {
        Self {
            key,
            properties: BTreeMap::new(),
        }
    }
    pub fn set_key(&mut self, key: ResourceKey) {
        self.key = key;
    }
    pub fn set_type(&mut self, kind: u32) {
        self.key.kind = kind;
    }
    pub fn set_instance(&mut self, instance: u32) {
        self.key.instance = instance;
    }
    pub fn write(&self, io: &mut dyn TrackResourceIo) -> Result<bool, Unsupported> {
        let format = match self.key.kind {
            0x0B8D_821A => ResourceFormat::Legacy,
            0x0B9E_B87E => ResourceFormat::Properties,
            _ => return Ok(false),
        };
        io.write(format, &self.properties)
    }
    pub fn read(&mut self, io: &mut dyn TrackResourceIo) -> Result<bool, Unsupported> {
        self.properties.insert(0xFF6F_9546, self.key.instance as i32);
        io.read(
            if self.key.kind == 0x0B8D_821A {
                ResourceFormat::Legacy
            } else {
                ResourceFormat::Properties
            },
            &mut self.properties,
        )
    }
    pub const LEGACY_PREFIX: &'static str = "TKDT,2,";
    pub fn private_copy(&self) -> Result<Self, Unsupported> {
        Err(Unsupported("SS5"))
    }
}
