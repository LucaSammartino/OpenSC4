// SPDX-License-Identifier: GPL-3.0-or-later
//! Preferences, plugin policy, registry summary and bounded pumps (§13-15).
use super::{Application, Preferences};

pub const PREFERENCES_KEY: [u32; 3] = [0xA9DD_6E06, 0x49DD_6E08, 1];
pub const PREFERENCES_SIZE: usize = 0x1734;
pub const PREFERENCES_VERSION: u32 = 0x15;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreferencesImage(pub [u8; PREFERENCES_SIZE]);

impl Default for PreferencesImage {
    fn default() -> Self { Self([0; PREFERENCES_SIZE]) }
}

impl PreferencesImage {
    fn word(&self, offset: usize) -> u32 {
        u32::from_le_bytes(self.0[offset..offset + 4].try_into().expect("fixed preferences field"))
    }
    fn put_word(&mut self, offset: usize, value: u32) {
        self.0[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn overlay(&mut self, p: &Preferences) {
        for (offset, value) in [(4, p.audio_04), (0x69, p.audio_69), (0x498, p.rdd),
            (0x4A4, p.pause_on_focus_loss), (0x4EF, p.ime), (0xF17, p.fullscreen)] {
            self.0[offset] = u8::from(value);
        }
        for (offset, value) in [(0x10, p.radio), (0x470, p.simulation.to_bits()),
            (0xF1C, p.width), (0xF20, p.height), (0xF24, p.depth)] { self.put_word(offset, value); }
        self.0[0xF13] = p.driver_f13; self.0[0xF0C] = p.driver_f0c;
        self.0[0x4AC..0x4EC].fill(0);
        let len = p.language.len().min(63);
        self.0[0x4AC..0x4AC + len].copy_from_slice(&p.language[..len]);
    }
    fn apply(&self, p: &mut Preferences) {
        p.audio_04 = self.0[4] != 0; p.audio_69 = self.0[0x69] != 0;
        p.rdd = self.0[0x498] != 0; p.pause_on_focus_loss = self.0[0x4A4] != 0;
        p.ime = self.0[0x4EF] != 0; p.fullscreen = self.0[0xF17] != 0;
        p.driver_f13 = self.0[0xF13]; p.driver_f0c = self.0[0xF0C];
        p.radio = self.word(0x10); p.simulation = f32::from_bits(self.word(0x470));
        p.width = self.word(0xF1C); p.height = self.word(0xF20); p.depth = self.word(0xF24);
        let language = &self.0[0x4AC..0x4EC];
        p.language = language[..language.iter().position(|b| *b == 0).unwrap_or(64)].to_vec();
    }
}

/// Primitive filesystem/package/COM operations; preferences serialization and
/// the library-enable decision belong to the app, not this adapter.
pub trait CompatibilityHost {
    fn read_preferences_record(&mut self, path: &[u8], key: [u32; 3]) -> Option<(u32, Vec<u8>)>;
    fn delete_preferences_file(&mut self, path: &[u8]);
    fn write_preferences_record(&mut self, path: &[u8], key: [u32; 3], version: u32, bytes: &[u8]) -> bool;
    fn dll_files(&mut self, directory: &[u8]) -> Vec<Vec<u8>>;
    fn library_admin_enabled(&mut self, library: &[u8], by_name: bool) -> Option<Vec<u8>>;
    fn add_library(&mut self, library: &[u8], by_name: bool) -> bool;
}

impl Application {
    fn preferences_path(&self) -> Vec<u8> {
        let mut name = Vec::new(); self.app_file_name(b".cfg", &mut name);
        append_path(&self.directories.user_data_e0, &name)
    }
    pub fn read_preferences(&mut self, host: &mut impl CompatibilityHost) -> bool {
        let Some((version, bytes)) = host.read_preferences_record(&self.preferences_path(), PREFERENCES_KEY) else { return false; };
        if version != PREFERENCES_VERSION || bytes.len() != PREFERENCES_SIZE { return false; }
        self.preferences_image.0.copy_from_slice(&bytes);
        self.preferences_image.apply(&mut self.preferences);
        true
    }
    pub fn save_preferences(&mut self, host: &mut impl CompatibilityHost) -> bool {
        self.preferences.language.clear(); self.preferences.ime = true;
        self.preferences_image.overlay(&self.preferences);
        self.preferences_image.0[0x4A8] = 1;
        self.preferences_image.0[0x4AC..0x4EC].fill(0);
        let path = self.preferences_path();
        host.delete_preferences_file(&path);
        host.write_preferences_record(&path, PREFERENCES_KEY, PREFERENCES_VERSION, &self.preferences_image.0)
    }
    pub fn add_dynamic_library(&self, library: &[u8], by_name: bool, host: &mut impl CompatibilityHost) -> bool {
        let enabled = host.library_admin_enabled(library, by_name);
        // AP12: no case folding is specified for cRZString::IsEqual here.
        if enabled.as_deref().is_some_and(|v| v == b"false" || v == b"0") { return false; }
        host.add_library(library, by_name)
    }
    pub fn add_code_plugins(&self, directory: &[u8], host: &mut impl CompatibilityHost) {
        for library in host.dll_files(directory) { self.add_dynamic_library(&library, false, host); }
    }
}

pub fn append_path(base: &[u8], name: &[u8]) -> Vec<u8> {
    let mut out = base.to_vec();
    if !out.is_empty() && !matches!(out.last(), Some(b'/' | b'\\')) { out.push(b'\\'); }
    out.extend_from_slice(name); out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryEntry { pub class: Vec<u8>, pub path: Vec<u8> }

/// §14.2 publishes class;path but leaves class-ID syntax and malformed-entry
/// handling open (AP13). Preserve class bytes for the foreign class factory.
pub fn registry_entry(entry: &[u8], data: &[u8], language: Option<&[u8]>) -> Option<RegistryEntry> {
    let split = entry.iter().position(|b| *b == b';')?;
    let mut path = entry[split + 1..].to_vec();
    for (key, value) in [(b"%DataDirectory%".as_slice(), data), (b"%Language%".as_slice(), language.unwrap_or(b"English"))] {
        let mut out = Vec::new(); let mut rest = path.as_slice();
        while let Some(index) = rest.windows(key.len()).position(|window| window == key) {
            out.extend_from_slice(&rest[..index]); out.extend_from_slice(value); rest = &rest[index + key.len()..];
        }
        out.extend_from_slice(rest); path = out;
    }
    Some(RegistryEntry { class: entry[..split].to_vec(), path })
}

pub trait PumpHost {
    fn server_available(&mut self, generation: u8) -> bool;
    fn now_ms(&mut self) -> u64;
    fn message_waiting(&mut self, generation: u8) -> bool;
    fn deliver_one(&mut self, generation: u8);
}

/// Strictly more than min and ms; max is an unconditional message bound (§15).
pub fn pump_messages(generation: u8, [min, max, ms]: [u32; 3], host: &mut impl PumpHost) -> u32 {
    if !host.server_available(generation) { return 0; }
    let start = host.now_ms(); let mut count = 0;
    while count < max && host.message_waiting(generation) {
        host.deliver_one(generation); count += 1;
        if count > min && host.now_ms().wrapping_sub(start) > u64::from(ms) { break; }
    }
    count
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face { Primary, Message2, WindowMessage, App, Service, StringHelper, Framework }
/// Native layout facts, not Rust pointer arithmetic or invented COM identities.
pub const FACES: [(Face, u32, u32); 7] = [(Face::Primary, 0, 15), (Face::Message2, 4, 4),
    (Face::WindowMessage, 8, 4), (Face::App, 12, 71), (Face::Service, 16, 13),
    (Face::StringHelper, 20, 6), (Face::Framework, 44, 15)];

pub const V2_SUBSCRIPTIONS: [u32; 9] = [0xFA0C_6390, 0x7A10_4750, 0xBA44_EB51,
    0x49A7_80B2, 0xEA65_224E, 0xCBB5_BB45, 0x8BB5_BB46, 0xEA8A_E29A, 0x26D3_1EC2];

pub trait StartupHost {
    fn other_instance_running(&mut self) -> bool;
    fn restart_time(&mut self) -> u64;
    fn restart_sleep(&mut self, units: u32);
    fn driver_description_hardware(&mut self) -> bool;
}

/// The published restart poll: no wall-clock assumption is made about timer
/// unit 5. The adapter exposes the same units as the original timer.
pub fn wait_for_instance(allow_multiple: bool, restart: bool, host: &mut impl StartupHost) -> bool {
    if !host.other_instance_running() || allow_multiple { return true; }
    if !restart { return false; }
    let start = host.restart_time();
    while host.other_instance_running() {
        if host.restart_time().wrapping_sub(start) >= 30 { return false; }
        host.restart_sleep(200_000);
    }
    true
}

impl Application {
    pub fn apply_driver_description(&mut self, hardware: bool, host: &mut impl super::Host) {
        if hardware { self.preferences.driver_f13 = 1; }
        else if self.preferences.driver_f13 != 0 {
            host.effect(super::Effect::Warning { title: "SimCity 4",
                text: "Could not initialize the hardware driver. Using the software driver instead." });
            self.preferences.driver_f13 = 0; self.preferences.driver_f0c = 1;
        }
    }

    /// Drives the existing ordered runner with implementations of the newly
    /// specified app methods. Remaining boundaries are the spec's open methods,
    /// not an adapter's fabricated success for an app algorithm.
    pub fn run_owned(&mut self, host: &mut (impl super::Host + CompatibilityHost + super::city::CityHost)) -> super::Flow {
        loop {
            let flow = self.run(host);
            let super::Flow::NeedsImplementation(boundary) = flow else { return flow; };
            match &boundary {
                super::Boundary::SavePreferences => { self.save_preferences(host); }
                super::Boundary::AddCodePlugins(path) => self.add_code_plugins(path, host),
                super::Boundary::CloseCity => { self.close_city(host); }
                super::Boundary::RequestQuit => {
                    if let Err(open) = self.request_quit(false, false, host) { return super::Flow::NeedsImplementation(open); }
                }
                super::Boundary::CityObliteration => {
                    if let Err(open) = self.do_city_obliteration(host) { return super::Flow::NeedsImplementation(open); }
                }
                // The exact identity/meaning is AP5, but the two foreign calls
                // are already represented by the ordered runner's next steps.
                super::Boundary::QuitService => {},
                super::Boundary::ShutdownMessageServer1 => self.unsubscribe_app_messages(host),
                _ => return super::Flow::NeedsImplementation(boundary),
            }
            self.resume_after_implemented(&boundary).expect("runner's own pending boundary");
        }
    }

    pub fn subscribe_app_messages(&self, host: &mut impl super::Host) {
        host.effect(super::Effect::RegisterV2 { kinds: V2_SUBSCRIPTIONS, target: self.app_token });
    }

    pub fn unsubscribe_app_messages(&self, host: &mut impl super::Host) {
        host.effect(super::Effect::UnregisterV2 { kinds: V2_SUBSCRIPTIONS, target: self.app_token });
        host.effect(super::Effect::UnregisterV1 { kind: super::QUIT_REQUEST, target: self.app_token });
    }

    pub fn unregister_app_cheats(&self, host: &mut impl super::Host) {
        if let Some(manager) = self.helper(super::Helper::CheatManager) {
            for id in [super::GP_CHEAT, super::EXCEPTION_CHEAT, super::FORCE_IDLE_CHEAT] {
                host.effect(super::Effect::UnregisterCheat { manager, id });
            }
        }
    }
}
