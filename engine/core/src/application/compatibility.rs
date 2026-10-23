// SPDX-License-Identifier: GPL-3.0-or-later
//! Preferences, plugin policy, registry summary and bounded pumps (§13-15).
use super::{Application, Preferences};

pub const PREFERENCES_KEY: [u32; 3] = [0xA9DD_6E06, 0x49DD_6E08, 1];
pub const PREFERENCES_SIZE: usize = 0x1734;
pub const PREFERENCES_VERSION: u32 = 0x15;
pub const VIDEO_SETTINGS_WRITTEN: usize = 12;
pub const VIDEO_SETTINGS_READ: usize = 14;

/// Erasure is independent of named-switch lookup: even `axjunk` qualifies.
pub fn erase_one_shot_x(line: &mut crate::cmdline::CmdLine) -> bool {
    let index = line
        .args()
        .iter()
        .position(|arg| arg.iter().position(|b| *b == b'x') == Some(1));
    index.is_some_and(|index| line.erase_argument(index as i32).is_ok())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreferencesImage(pub [u8; PREFERENCES_SIZE]);

impl Default for PreferencesImage {
    fn default() -> Self {
        Self([0; PREFERENCES_SIZE])
    }
}

impl PreferencesImage {
    fn word(&self, offset: usize) -> u32 {
        u32::from_le_bytes(self.0[offset..offset + 4].try_into().expect("fixed preferences field"))
    }
    fn put_word(&mut self, offset: usize, value: u32) {
        self.0[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn overlay(&mut self, p: &Preferences) {
        for (offset, value) in [
            (4, p.audio_04),
            (0x69, p.audio_69),
            (0x498, p.rdd),
            (0x4A4, p.pause_on_focus_loss),
            (0x4EF, p.ime),
            (0xF17, !p.fullscreen),
        ] {
            // Raw-block persistence preserves noncanonical true bytes unless
            // the modeled boolean actually changes.
            if (self.0[offset] != 0) != value {
                self.0[offset] = u8::from(value);
            }
        }
        for (offset, value) in [
            (0x10, p.radio),
            (0x470, p.simulation.to_bits()),
            (0xF1C, p.width),
            (0xF20, p.height),
            (0xF24, p.depth),
        ] {
            self.put_word(offset, value);
        }
        self.0[0xF13] = p.driver_f13;
        self.0[0xF0C] = p.driver_f0c;
        self.0[0x4AC..0x4EC].fill(0);
        let len = p.language.len().min(63);
        self.0[0x4AC..0x4AC + len].copy_from_slice(&p.language[..len]);
    }
    fn apply(&self, p: &mut Preferences) {
        p.audio_04 = self.0[4] != 0;
        p.audio_69 = self.0[0x69] != 0;
        p.rdd = self.0[0x498] != 0;
        p.pause_on_focus_loss = self.0[0x4A4] != 0;
        p.ime = self.0[0x4EF] != 0;
        p.fullscreen = self.0[0xF17] == 0;
        p.driver_f13 = self.0[0xF13];
        p.driver_f0c = self.0[0xF0C];
        p.radio = self.word(0x10);
        p.simulation = f32::from_bits(self.word(0x470));
        p.width = self.word(0xF1C);
        p.height = self.word(0xF20);
        p.depth = self.word(0xF24);
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
    fn library_path(&mut self, library: &[u8]) -> Vec<u8>;
    fn library_exists(&mut self, library: &[u8]) -> bool;
    fn gzcom_available(&mut self) -> bool;
}

impl Application {
    fn preferences_path(&self) -> Vec<u8> {
        let mut name = Vec::new();
        self.app_file_name(b".cfg", &mut name);
        append_path(&self.directories.user_data_e0, &name)
    }
    pub fn read_preferences(&mut self, host: &mut impl CompatibilityHost) -> bool {
        let Some((version, bytes)) = host.read_preferences_record(&self.preferences_path(), PREFERENCES_KEY) else {
            return false;
        };
        if version != PREFERENCES_VERSION || bytes.len() != PREFERENCES_SIZE {
            return false;
        }
        self.preferences_image.0.copy_from_slice(&bytes);
        self.preferences_image.apply(&mut self.preferences);
        true
    }
    pub fn save_preferences(&mut self, host: &mut impl CompatibilityHost) -> bool {
        self.preferences.language.clear();
        self.preferences.ime = true;
        self.preferences_image.overlay(&self.preferences);
        self.preferences_image.0[0x4A8] = 1;
        self.preferences_image.0[0x4AC..0x4EC].fill(0);
        self.preferences_image.0[0x4EF] = 1;
        let path = self.preferences_path();
        host.delete_preferences_file(&path);
        host.write_preferences_record(&path, PREFERENCES_KEY, PREFERENCES_VERSION, &self.preferences_image.0)
    }
    pub fn add_dynamic_library(&self, library: &[u8], by_name: bool, host: &mut impl CompatibilityHost) -> bool {
        self.add_dynamic_library_with_bypass(library, by_name, false, host)
    }

    pub fn add_dynamic_library_with_bypass(
        &self,
        library: &[u8],
        by_name: bool,
        bypass: bool,
        host: &mut impl CompatibilityHost,
    ) -> bool {
        let path = if by_name {
            host.library_path(library)
        } else {
            library.to_vec()
        };
        if !host.gzcom_available() || !host.library_exists(&path) {
            return false;
        }
        if !bypass
            && host
                .library_admin_enabled(&path, false)
                .as_deref()
                .is_some_and(|v| v.eq_ignore_ascii_case(b"false") || v == b"0")
        {
            return false;
        }
        host.add_library(&path, false)
    }
    pub fn add_code_plugins(&self, directory: &[u8], host: &mut impl CompatibilityHost) {
        for library in host.dll_files(directory) {
            self.add_dynamic_library(&library, false, host);
        }
    }
}

pub fn append_path(base: &[u8], name: &[u8]) -> Vec<u8> {
    let mut out = base.to_vec();
    if !out.is_empty() && !matches!(out.last(), Some(b'/' | b'\\')) {
        out.push(b'\\');
    }
    out.extend_from_slice(name);
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryEntry {
    pub class: Vec<u8>,
    pub path: Vec<u8>,
}

/// Split only the first semicolon. A missing separator has class zero (empty
/// token) and leaves the entire value as the path; conversion is in resources.
pub fn registry_entry(entry: &[u8], data: &[u8], language: Option<&[u8]>) -> Option<RegistryEntry> {
    let split = entry.iter().position(|b| *b == b';');
    let mut path = entry[split.map_or(0, |index| index + 1)..].to_vec();
    for (key, value) in [
        (b"%DataDirectory%".as_slice(), data),
        (b"%Language%".as_slice(), language.unwrap_or(b"English")),
    ] {
        let mut out = Vec::new();
        let mut rest = path.as_slice();
        while let Some(index) = rest.windows(key.len()).position(|window| window == key) {
            out.extend_from_slice(&rest[..index]);
            out.extend_from_slice(value);
            rest = &rest[index + key.len()..];
        }
        out.extend_from_slice(rest);
        path = out;
    }
    Some(RegistryEntry {
        class: entry[..split.unwrap_or(0)].to_vec(),
        path,
    })
}

pub trait PumpHost {
    fn server_available(&mut self, generation: u8) -> bool;
    fn now_ms(&mut self) -> u64;
    fn message_waiting(&mut self, generation: u8) -> bool;
    fn deliver_one(&mut self, generation: u8);
}

/// Strictly more than min and ms; max is an unconditional message bound (§15).
pub fn pump_messages(generation: u8, [min, max, ms]: [u32; 3], host: &mut impl PumpHost) -> u32 {
    if !host.server_available(generation) {
        return 0;
    }
    let start = host.now_ms();
    let mut count = 0;
    while count < max && host.message_waiting(generation) {
        host.deliver_one(generation);
        count += 1;
        if count > min && host.now_ms().wrapping_sub(start) > u64::from(ms) {
            break;
        }
    }
    count
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Primary,
    Message2,
    WindowMessage,
    App,
    Service,
    StringHelper,
    Framework,
}
/// Native layout facts, not Rust pointer arithmetic or invented COM identities.
pub const FACES: [(Face, u32, u32); 7] = [
    (Face::Primary, 0, 15),
    (Face::Message2, 4, 4),
    (Face::WindowMessage, 8, 4),
    (Face::App, 12, 71),
    (Face::Service, 16, 13),
    (Face::StringHelper, 20, 6),
    (Face::Framework, 44, 15),
];

pub const V2_SUBSCRIPTIONS: [u32; 9] = [
    0xFA0C_6390,
    0x7A10_4750,
    0xBA44_EB51,
    0x49A7_80B2,
    0xEA65_224E,
    0xCBB5_BB45,
    0x8BB5_BB46,
    0xEA8A_E29A,
    0x26D3_1EC2,
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppHandles {
    pub debug_console: Option<u64>,
    pub gimex_factory: Option<u64>,
    pub window_location_saver: Option<u64>,
    pub unnamed_660: Option<u64>,
    pub lua_interpreter: Option<u64>,
    pub flag_668: u8,
    pub stored_4c: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Embedded {
    GlyphTextureManager,
    TutorialRegistry,
}

#[derive(Debug, PartialEq)]
pub enum AppValue<'a> {
    Bool(bool),
    Byte(u8),
    Word(u32),
    Token(Option<u64>),
    String(Vec<u8>),
    Preferences(&'a Preferences),
    NewCitySpecification(&'a [u8]),
    Embedded(Embedded),
}

impl Application {
    /// All published cISC4App getter slots. Unpublished folded slots return
    /// their slot number as an explicit unsupported result (AP14). Tokens are
    /// borrowed, and embedded variants preserve identity without inventing a
    /// native pointer. Lazy directories require externally serialized mutation.
    pub fn app_interface_getter(
        &mut self,
        slot: u32,
        language: Option<&[u8]>,
        host: &mut impl super::city::CityHost,
    ) -> Result<AppValue<'_>, u32> {
        self.app_interface_getter_language(slot, 0, language, host)
    }

    pub fn app_interface_getter_language(
        &mut self,
        slot: u32,
        language_index: i32,
        language: Option<&[u8]>,
        host: &mut impl super::city::CityHost,
    ) -> Result<AppValue<'_>, u32> {
        let value = match slot {
            21 => AppValue::Bool(self.debug_enabled),
            23 => AppValue::Bool(self.popup_dialogs_enabled),
            25 => AppValue::Word(self.mode),
            26 => AppValue::Word(self.handles.stored_4c),
            27 => AppValue::String(self.app_name.clone()),
            28 | 30 => {
                let mut name = Vec::new();
                self.app_file_name(if slot == 28 { b".ini" } else { b".cfg" }, &mut name);
                AppValue::String(name)
            }
            29 | 31 => {
                let mut name = Vec::new();
                self.app_file_name(if slot == 29 { b".ini" } else { b".cfg" }, &mut name);
                let candidate = [self.directories.data_7c.as_slice(), &name].concat();
                AppValue::String(
                    if slot == 29 && host.query(super::city::Query::Exists(candidate.clone())).flag {
                        candidate
                    } else {
                        [self.directories.user_base_11c.as_slice(), &name].concat()
                    },
                )
            }
            32 => AppValue::Token(self.field_314),
            33 => AppValue::Token(self.helper(super::Helper::CheatManager)),
            34 => AppValue::Token(self.city_state.region_owner),
            37 => AppValue::Token(self.city_state.city),
            35 => AppValue::Token(
                self.city_state
                    .region_owner
                    .and_then(|owner| host.query(super::city::Query::Region(owner)).token),
            ),
            36 => AppValue::Token(self.city_state.regional_city),
            38 => AppValue::Preferences(&self.preferences),
            39 => AppValue::NewCitySpecification(&self.city_state.new_city_specification),
            40 => AppValue::Token(self.handles.debug_console),
            41 => AppValue::Token(self.handles.gimex_factory),
            42 => AppValue::Token(self.helper(super::Helper::StringDetokenizer)),
            43 => AppValue::Token(self.handles.window_location_saver),
            44 => AppValue::Token(self.helper(super::Helper::RenderProperties)),
            45 | 48 => AppValue::Embedded(Embedded::GlyphTextureManager),
            46 => AppValue::Token(self.handles.unnamed_660),
            47 => AppValue::Token(self.handles.lua_interpreter),
            49 => AppValue::Embedded(Embedded::TutorialRegistry),
            50 => AppValue::Byte(self.handles.flag_668),
            51 => AppValue::String(self.directories.data_7c.clone()),
            52 => AppValue::String(self.directories.cd_data_90.clone()),
            53 => AppValue::String(self.directories.plugin_directory(language_index, language)),
            54 => AppValue::String(self.directories.language_directory(true, language_index, language)),
            55 => AppValue::String(self.directories.sku_cc.clone()),
            56 => AppValue::String(self.directories.user_data_e0.clone()),
            // Preserve AP3's offset names rather than adopting wrong export aliases.
            57 => AppValue::String(self.directories.slot_e4_108.clone()),
            58 => AppValue::String(self.directories.user_base_11c.clone()),
            59..=66 => {
                use super::UserFolder::*;
                let folder = [
                    Plugins,
                    Regions,
                    MySim,
                    Albums,
                    HttpCache,
                    Temp,
                    ExceptionReports,
                    Scripts,
                ][(slot - 59) as usize];
                AppValue::String(self.directories.user_folder(folder).to_vec())
            }
            _ => return Err(slot),
        };
        Ok(value)
    }
}

pub trait StartupHost {
    fn other_instance_running(&mut self) -> bool;
    fn restart_time(&mut self) -> u64;
    fn restart_sleep(&mut self, units: u32);
    fn driver_description_hardware(&mut self) -> bool;
}

/// Public-framework entry is foreign; the application owns this ordering (§3).
pub trait EntryHost {
    fn create_framework(&mut self, sc4: bool) -> u64;
    fn create_application(&mut self, name: &[u8]) -> u64;
    fn set_framework(&mut self, framework: u64);
    fn set_application_framework_face(&mut self, app: u64);
    fn framework_main(&mut self, framework: u64, args: &[Vec<u8>]);
    fn release_application(&mut self, app: u64);
    fn framework_slot40(&mut self, framework: u64, value: u32);
    fn resource_error(&mut self) -> Option<Vec<u8>>;
    fn display_resource_error(&mut self, text: &[u8]);
}

pub fn app_start(sc4_framework: bool, args: &[Vec<u8>], host: &mut impl EntryHost) {
    let framework = host.create_framework(sc4_framework);
    let app = host.create_application(b"SimCity 4");
    host.set_framework(framework);
    host.set_application_framework_face(app);
    host.framework_main(framework, args);
    host.release_application(app);
    host.framework_slot40(framework, 1);
    if let Some(error) = host.resource_error() {
        host.display_resource_error(&error);
    }
}

pub trait ResourceHost {
    fn ini_value(&mut self, file: &[u8], section: &[u8], key: &[u8]) -> Option<Vec<u8>>;
    fn ini_entries(&mut self, file: &[u8], section: &[u8]) -> Vec<Vec<u8>>;
    fn directory_exists(&mut self, path: &[u8]) -> bool;
    fn create_directory(&mut self, path: &[u8]);
    fn database_files(&mut self, path: &[u8], recursive: bool) -> Vec<Vec<u8>>;
    fn open_resource_segment(&mut self, class: &[u8], path: &[u8]);
    fn register_resource_factory(&mut self, resource_type: u32);
    fn packed_setting(&mut self, textures: bool, enabled: bool);
}

impl Application {
    /// §14.3 only accepts existing INI overrides. INI path resolution remains
    /// an explicit caller input until slot 29 is specified (AP13).
    pub fn find_and_possibly_fix_ini_files(&mut self, ini_path: &[u8], host: &mut impl ResourceHost) {
        for (key, plugin) in [(b"Data".as_slice(), false), (b"PlugIn".as_slice(), true)] {
            if let Some(path) = host.ini_value(ini_path, b"Directories", key) {
                if host.directory_exists(&path) {
                    if plugin {
                        self.directories.plugin_a4 = path;
                    } else {
                        self.directories.data_7c = path;
                    }
                }
            }
        }
    }

    /// Implements the published registry summary. Callback internals/class
    /// syntax are expressly open in §12; the foreign factory gets raw class bytes.
    pub fn load_registry_summary(
        &self,
        language: Option<&[u8]>,
        packed: [Option<&[u8]>; 2],
        host: &mut impl ResourceHost,
    ) {
        let file = append_path(&self.directories.data_7c, b"Sys\\Resources.ini");
        // AP13: the summary does not name the INI section for these options.
        // Their values are explicit inputs, rather than an invented section.
        for (value, textures) in [(packed[0], false), (packed[1], true)] {
            if let Some(value) = value {
                if let Some(enabled) = super::command_line::on_off(value) {
                    host.packed_setting(textures, enabled);
                }
            }
        }
        for section in [b"DBSegments".as_slice(), b"DBSegmentDirectoryFiles".as_slice()] {
            for entry in host.ini_entries(&file, section) {
                if let Some(entry) = registry_entry(&entry, &self.directories.data_7c, language) {
                    if section == b"DBSegments" {
                        host.open_resource_segment(&entry.class, &entry.path);
                    } else {
                        for file in host.database_files(&entry.path, false) {
                            host.open_resource_segment(&entry.class, &file);
                        }
                    }
                }
            }
        }
    }

    /// Enumeration/creation/order from §14.2. The complete factory list and
    /// database extensions are open (AP13), so foreign enumeration takes care
    /// of identifying database files; callers supply the published factory list.
    pub fn setup_resources_summary(
        &self,
        plugin_directories: &[Vec<u8>],
        factories: &[u32],
        host: &mut impl ResourceHost,
    ) {
        for file in host.database_files(&self.directories.data_7c, false) {
            if file.ends_with(b".dat") {
                host.open_resource_segment(b"", &file);
            }
        }
        for directory in plugin_directories {
            if !host.directory_exists(directory) {
                host.create_directory(directory);
            }
            for file in host.database_files(directory, true) {
                host.open_resource_segment(b"", &file);
            }
        }
        for factory in factories {
            host.register_resource_factory(*factory);
        }
    }
}

/// The published restart poll: no wall-clock assumption is made about timer
/// unit 5. The adapter exposes the same units as the original timer.
pub fn wait_for_instance(allow_multiple: bool, restart: bool, host: &mut impl StartupHost) -> bool {
    if !host.other_instance_running() || allow_multiple {
        return true;
    }
    if !restart {
        return false;
    }
    let start = host.restart_time();
    while host.other_instance_running() {
        if host.restart_time().wrapping_sub(start) >= 30 {
            return false;
        }
        host.restart_sleep(200_000);
    }
    true
}

impl Application {
    pub fn apply_driver_description(&mut self, hardware: bool, host: &mut impl super::Host) {
        if hardware {
            self.preferences.driver_f13 = 1;
        } else if self.preferences.driver_f13 != 0 {
            host.effect(super::Effect::Warning {
                title: "SimCity 4",
                text: "Could not initialize the hardware driver. Using the software driver instead.",
            });
            self.preferences.driver_f13 = 0;
            self.preferences.driver_f0c = 1;
        }
    }

    /// Drives the existing ordered runner with implementations of the newly
    /// specified app methods. Remaining boundaries are the spec's open methods,
    /// not an adapter's fabricated success for an app algorithm.
    pub fn run_owned(
        &mut self,
        host: &mut (impl super::Host + CompatibilityHost + super::city::CityHost + StartupHost + ResourceHost),
    ) -> super::Flow {
        loop {
            let flow = self.run(host);
            let super::Flow::NeedsImplementation(boundary) = flow else {
                return flow;
            };
            match &boundary {
                super::Boundary::SavePreferences => {
                    self.save_preferences(host);
                }
                super::Boundary::AddCodePlugins(path) => {
                    if path.is_empty() {
                        let Some(directories) = &self.startup_plugin_directories else {
                            return super::Flow::NeedsImplementation(boundary);
                        };
                        for directory in directories {
                            self.add_code_plugins(directory, host);
                        }
                    } else {
                        self.add_code_plugins(path, host);
                    }
                }
                super::Boundary::RestartWait => {
                    if !wait_for_instance(self.instance_policy.0, self.instance_policy.1, host) {
                        self.abort_pending_startup();
                        return super::Flow::Complete(false);
                    }
                }
                super::Boundary::FindAndPossiblyFixIniFiles => {
                    let Some(path) = self.resolved_ini_path.clone() else {
                        return super::Flow::NeedsImplementation(boundary);
                    };
                    self.find_and_possibly_fix_ini_files(&path, host);
                }
                super::Boundary::DriverConfiguration => {
                    let hardware = host.driver_description_hardware();
                    self.apply_driver_description(hardware, host);
                    // AP5 still blocks the unpublished config-rule target byte.
                    return super::Flow::NeedsImplementation(boundary);
                }
                super::Boundary::CloseCity => {
                    self.close_city(host);
                }
                super::Boundary::RequestQuit => {
                    if let Err(open) = self.request_quit(false, false, host) {
                        return super::Flow::NeedsImplementation(open);
                    }
                }
                super::Boundary::CityObliteration => {
                    if let Err(open) = self.do_city_obliteration(host) {
                        return super::Flow::NeedsImplementation(open);
                    }
                }
                // The exact identity/meaning is AP5, but the two foreign calls
                // are already represented by the ordered runner's next steps.
                super::Boundary::QuitService => {}
                super::Boundary::ShutdownMessageServer1 => self.unsubscribe_app_messages(host),
                _ => return super::Flow::NeedsImplementation(boundary),
            }
            self.resume_after_implemented(&boundary)
                .expect("runner's own pending boundary");
        }
    }

    pub fn subscribe_app_messages(&self, host: &mut impl super::Host) {
        host.effect(super::Effect::RegisterV2 {
            kinds: V2_SUBSCRIPTIONS,
            target: self.app_token,
        });
    }

    pub fn unsubscribe_app_messages(&self, host: &mut impl super::Host) {
        host.effect(super::Effect::UnregisterV2 {
            kinds: V2_SUBSCRIPTIONS,
            target: self.app_token,
        });
        host.effect(super::Effect::UnregisterV1 {
            kind: super::QUIT_REQUEST,
            target: self.app_token,
        });
    }

    pub fn unregister_app_cheats(&self, host: &mut impl super::Host) {
        if let Some(manager) = self.helper(super::Helper::CheatManager) {
            for id in [super::GP_CHEAT, super::EXCEPTION_CHEAT, super::FORCE_IDLE_CHEAT] {
                host.effect(super::Effect::UnregisterCheat { manager, id });
            }
        }
    }
}
