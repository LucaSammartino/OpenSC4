// SPDX-License-Identifier: GPL-3.0-or-later
//! HIT manager/channel and initialization resources (§§4–5).
use super::directors::ForeignRef;
pub use super::hit_config::{unsigned_number, IniAction};
pub use super::hit_resource::LEGACY_FIELDS;
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
    pub blocks: Vec<Arc<TrackDataBlock>>,
    pub semantics: BTreeMap<u32, ForeignRef>,
    pub groups: BTreeMap<u32, ForeignRef>,
    pub sequence_sounds: BTreeMap<u32, SoundRef>,
    pub duck_priorities: BTreeMap<u32, i32>,
    pub registers: Properties,
    pub view_updates: usize,
    pub source_handler: Option<ForeignRef>,
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
            blocks: Vec::new(),
            semantics: BTreeMap::new(),
            groups: BTreeMap::new(),
            sequence_sounds: BTreeMap::new(),
            duck_priorities: BTreeMap::new(),
            registers: Properties::new(),
            view_updates: 0,
            source_handler: None,
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
    pub fn register_block(&mut self, block: Arc<TrackDataBlock>) {
        self.blocks.retain(|old| !Arc::ptr_eq(old, &block));
        self.blocks.insert(0, block);
    }
    pub fn remove_block(&mut self, block: &Arc<TrackDataBlock>) {
        self.blocks.retain(|old| !Arc::ptr_eq(old, block));
    }
    /// Borrowed result: the manager does not acquire a reference for lookup.
    pub fn entry(&self, id: u32) -> Option<(&TrackDataBlock, u32)> {
        self.blocks
            .iter()
            .find_map(|block| block.entries.get(&id).map(|offset| (block.as_ref(), *offset)))
    }
    pub fn register_semantic(&mut self, id: u32, object: ForeignRef) -> bool {
        self.semantics.insert(id, object);
        id != 0
    }
    pub fn control_group(&mut self, id: u32, create: impl FnOnce(u32) -> Option<ForeignRef>) -> Option<&ForeignRef> {
        if !self.groups.contains_key(&id) {
            self.groups.insert(id, create(id)?);
        }
        self.groups.get(&id)
    }
    pub fn set_sequence_sound(&mut self, id: u32, sound: SoundRef) {
        self.sequence_sounds.insert(id, sound);
    }
    pub fn duck_max(&self) -> i32 {
        self.duck_priorities.values().copied().fold(0, i32::max)
    }
    pub fn set_duck_priority(&mut self, sound: u32, priority: i32) {
        if !self.initialized || self.duck_priorities.get(&sound).copied().unwrap_or(0) == priority {
            return;
        }
        if priority == 0 {
            self.duck_priorities.remove(&sound);
        } else {
            self.duck_priorities.insert(sound, priority);
        }
        self.registers.insert(0x1B, self.duck_max());
        self.view_updates += 1;
    }
    pub fn clear_duck_map(&mut self) {
        self.duck_priorities.clear();
        self.registers.insert(0x1B, 0);
        self.view_updates += 1;
    }
    pub fn pause(&mut self) -> bool {
        self.clear_duck_map();
        true
    }
    pub fn unpause(&self) -> bool {
        true
    }
    pub fn random(&mut self, upper: u32, lower: u32) -> u32 {
        let span = upper.wrapping_sub(lower);
        if span == 0 {
            return lower;
        }
        let product = u64::from(self.random_seed) * 0x41C6_4E6D + 0x3039;
        self.random_seed = product as u32;
        lower.wrapping_add(((u64::from((product >> 16) as u32) * u64::from(span)) >> 32) as u32)
    }
    pub fn register_source_handler(&mut self, handler: ForeignRef) {
        self.source_handler = Some(handler);
    }
    pub fn source_field_words(&self, words: &[u32], call: impl FnOnce(&ForeignRef, &[u32]) -> u64) -> Option<u64> {
        self.source_handler.as_ref().map(|handler| call(handler, words))
    }
    /// Semantic payload and control-object policy require the §4.1 recording experiment.
    pub fn unresolved_operation(&self) -> Result<(), Unsupported> {
        Err(Unsupported("SS2"))
    }
}

pub struct TrackDataBlock {
    pub name: String,
    pub data: Vec<u8>,
    pub entries: BTreeMap<u32, u32>,
}
impl TrackDataBlock {
    /// §4.1 permits rejecting truncated input rather than reading outside it.
    pub fn parse(name: String, data: Vec<u8>) -> Option<Self> {
        let mut entries = BTreeMap::new();
        if let Some(start) = data.windows(4).position(|word| word == b"ENTP") {
            let mut cursor = start + 4;
            while cursor < data.len() {
                let id = u32::from_le_bytes(data.get(cursor..cursor + 4)?.try_into().ok()?);
                if id == 0x544E_4545 {
                    break;
                }
                let offset = u32::from_le_bytes(data.get(cursor + 4..cursor + 8)?.try_into().ok()?);
                entries.insert(id, offset);
                cursor += 8;
            }
        }
        Some(Self { name, data, entries })
    }
    pub fn entry_bytes(&self, offset: u32) -> Option<&[u8]> {
        self.data.get(offset as usize..)
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
    fn bind_source(&mut self, _sound: &SoundRef, _source: Option<&ForeignRef>, _mode: u32) -> bool {
        false
    }
    fn optional_float(&mut self, _sound: &SoundRef, _iid: u32, _slot: u32, _value: f32) -> bool {
        false
    }
    fn optional_integer(&mut self, _sound: &SoundRef, _iid: u32, _slot: u32, _value: i32) -> bool {
        false
    }
    fn resolve_properties(&mut self, _key: ResourceKey, _iid: u32) -> Option<Arc<Properties>> {
        None
    }
}

pub struct HitChannel {
    pub play_value: u32,
    pub properties: Properties,
    pub source: Option<SoundRef>,
    pub manager: Option<Arc<super::framework::SndMgr>>,
    pub float_properties: BTreeMap<u32, f32>,
    pub retained_source: Option<ForeignRef>,
    pub inherited_properties: Vec<Arc<Properties>>,
}
impl Default for HitChannel {
    fn default() -> Self {
        Self {
            play_value: 0x400,
            properties: Properties::new(),
            source: None,
            manager: None,
            float_properties: BTreeMap::new(),
            retained_source: None,
            inherited_properties: Vec::new(),
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
    pub const VIEW_SLOTS: [usize; 6] = [54, 17, 4, 9, 3, 4];
    pub fn query_interface(owner: &Arc<std::sync::Mutex<Self>>, iid: u32) -> Option<super::InterfaceView<Self>> {
        let offset = match iid {
            0x2060_77D2 => 0,
            0x3060_77D2 => 4,
            0x5B34_F0AF => 8,
            0xFD58_4467 => 12,
            0xAA11_A64F => 16,
            0x09D0_02D2 => 20,
            _ => return None,
        };
        Some(super::InterfaceView {
            owner: Arc::clone(owner),
            offset,
        })
    }
    pub fn value(&self, id: u32) -> i32 {
        self.properties
            .get(&id)
            .copied()
            .or_else(|| self.inherited_properties.iter().find_map(|set| set.get(&id).copied()))
            .unwrap_or(0)
    }
    pub fn update_patch(&mut self, backend: &mut dyn ChannelBackend) -> bool {
        if self.source.is_none() && !self.create_sound(backend) {
            return false;
        }
        let Some(sound) = &self.source else {
            return false;
        };
        // Safe rejection at the unchecked source/QI boundary permitted by §4.2.
        if !backend.bind_source(sound, self.retained_source.as_ref(), 2) {
            return false;
        }
        self.update_vol_pan();
        self.update_pitch();
        true
    }
    pub fn update_pitch(&mut self) -> bool {
        let Some(sound) = &self.source else {
            return false;
        };
        if !sound.is_loaded() {
            return false;
        }
        let factor = f64::from(self.float_properties.get(&0xFF30_71B9).copied().unwrap_or(1.0));
        if self.properties.contains_key(&0xFF09_9DB2)
            || self
                .inherited_properties
                .iter()
                .any(|set| set.contains_key(&0xFF09_9DB2))
        {
            !sound.set_frequency((f64::from(self.value(0xFF09_9DB2)) * factor).trunc() as i64 as i32)
        } else {
            let frequency = (f64::from(sound.frequency()) * factor).trunc() as i64 as i32;
            self.properties.insert(0xFF09_9DB2, frequency);
            true
        }
    }
    pub fn update_vol_pan(&mut self) -> bool {
        let Some(sound) = &self.source else {
            return true;
        };
        if self.value(0xFFCC_156F) == 0 {
            sound.set_pan(self.value(0xFF3E_C880));
        } else {
            let maximum = self.value(0xFFA6_A614);
            sound.set_min_distance(self.value(0xFFAA_B623) as f32);
            sound.set_max_distance(if maximum == 0 { 100000.0 } else { maximum as f32 });
            sound.set_position([
                self.value(0xFFCC_4D2D) as f32,
                self.value(0xFF4A_01D6) as f32,
                self.value(0xFF46_9820) as f32,
            ]);
            sound.set_velocity([
                self.value(0xFF44_4C9A) as f32,
                self.value(0xFFC2_0061) as f32,
                self.value(0xFFCE_9997) as f32,
            ]);
        }
        let volume = self.value(0xFF90_AAC8).wrapping_mul(self.value(0xFF53_DBF3)) / 1024;
        self.play_value = volume as u32;
        sound.set_volume(volume);
        true
    }
    pub fn notify(&mut self, id: u32, backend: &mut dyn ChannelBackend) -> bool {
        match id {
            0xFF09_9DB2 | 0xFF30_71B9 => {
                self.update_pitch();
                true
            }
            0xFF4C_97BE => self.notify_inherited(Arc::new(self.properties.clone()), backend),
            0xFF3E_C880 | 0xFF44_4C9A | 0xFF46_9820 | 0xFF4A_01D6 | 0xFF53_DBF3 | 0xFF90_AAC8 | 0xFFC2_0061
            | 0xFFCC_4D2D | 0xFFCE_9997 | 0xFFA6_A614 | 0xFFAA_B623 => self.update_vol_pan(),
            0xFF0A_D592 => {
                if !self.source.as_ref().is_some_and(|s| s.is_playing()) {
                    self.properties.insert(id, 0);
                }
                true
            }
            0xFFB3_6659 => {
                if let Some(sound) = &self.source {
                    if self.value(id) != 0 {
                        sound.pause();
                    } else {
                        sound.unpause();
                    }
                }
                true
            }
            0xFFAE_C905 => {
                self.retained_source = None;
                true
            }
            0xFF01_50F2 | 0xFF86_FE79 | 0xFF73_4343 | 0xFFD3_D39B => {
                let (iid, slot) = match id {
                    0xFF01_50F2 => (0x6B04_46EB, 6),
                    0xFF86_FE79 => (0x4B79_C1A9, 7),
                    0xFF73_4343 => (0x4B79_C1A9, 3),
                    _ => (0x4B79_C1A9, 4),
                };
                let result = self.source.as_ref().is_some_and(|sound| {
                    backend.optional_float(sound, iid, slot, self.float_properties.get(&id).copied().unwrap_or(0.0))
                });
                id == 0xFF01_50F2 || result
            }
            0xFFA4_2951 | 0xFFBD_B196 => self.source.as_ref().is_some_and(|sound| {
                backend.optional_integer(
                    sound,
                    0x4B79_C1A9,
                    if id == 0xFFA4_2951 { 6 } else { 5 },
                    self.value(id),
                )
            }),
            _ => true,
        }
    }
    /// External notifications supply the acquired notifying set, not a value copy.
    pub fn notify_inherited(&mut self, mut set: Arc<Properties>, backend: &mut dyn ChannelBackend) -> bool {
        self.inherited_properties.clear();
        loop {
            let next = set.get(&0xFF4C_97BE).copied().unwrap_or(0);
            self.inherited_properties.push(set);
            if next == 0 {
                break;
            }
            let Some(resolved) = backend.resolve_properties(
                ResourceKey {
                    kind: 0xEBFE_E345,
                    group: 0xFC45_CAF7,
                    instance: next as u32,
                },
                0xEBB1_0CDD,
            ) else {
                break;
            };
            set = resolved;
        }
        self.update_vol_pan()
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
        if !self.update_patch(backend) {
            return Ok(false);
        }
        self.load_after_patch(Ok(()), backend)
    }
    pub fn note_on(&mut self, payload: u32, backend: &mut dyn ChannelBackend) -> Result<bool, Unsupported> {
        if !self.update_patch(backend) || !self.load(backend)? {
            return Ok(false);
        }
        Ok(self
            .source
            .as_ref()
            .map_or(true, |sound| sound.play(payload, self.value(0xFFF7_B8DF) != 0, true)))
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
    fn data_directory(&self) -> &str {
        ""
    }
    fn file_exists(&mut self, _path: &str) -> bool {
        false
    }
    /// Inherited locale comparison stays in the adapter (§5.1 runtime boundary).
    fn text_is_true(&mut self, text: &str) -> bool {
        text == "true"
    }
    /// Executes the named COM operation with the IIDs/order specified in §5.1.
    /// False safely rejects a missing dependency, leaving earlier work installed.
    fn apply(&mut self, _action: &super::hit_config::IniAction) -> bool {
        false
    }
}
pub struct HitIniFile<I: IniFile> {
    pub ini: I,
    pub exclusions: std::collections::BTreeSet<u32>,
    pub allow_text_as_key: bool,
    pub creator_sections: BTreeMap<u32, String>,
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
    pub(super) fn init_defaults(&mut self, host: &mut dyn IniServices) {
        if host.has_manager() {
            for (key, spatial) in [
                ("DefaultSndDataSourcePlayer2DClassId", false),
                ("DefaultSndDataSourcePlayer3DClassId", true),
            ] {
                if let Some(value) = self.ini.read_entry("Config", key) {
                    host.set_default_player(spatial, super::hit_config::unsigned_number(&value));
                }
            }
        }
        if host.has_sound_system() {
            for value in self.numeric_entries("SndFileDefaultDataSourceClassIds").into_values() {
                let mut extension = value;
                let class = super::hit_config::split_token(&mut extension, ',');
                host.map_extension(&extension, super::hit_config::unsigned_number(&class));
            }
        }
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
    pub reference_count: u32,
    pub cursor_word: u32,
    pub legacy_tables: bool,
}
impl HitPropertySetTrackResource {
    pub fn new(key: ResourceKey) -> Self {
        Self {
            key,
            properties: BTreeMap::new(),
            reference_count: 0,
            cursor_word: 0,
            legacy_tables: false,
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
        let mut copy = Self::new(self.key);
        copy.legacy_tables = true;
        copy.assign(self);
        copy.reference_count = copy.reference_count.wrapping_add(1);
        Ok(copy)
    }
    pub fn assign(&mut self, source: &Self) {
        self.key = source.key;
        self.properties.clone_from(&source.properties);
        self.reference_count = source.reference_count;
        self.cursor_word = source.cursor_word;
    }
}
