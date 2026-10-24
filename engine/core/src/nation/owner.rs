// SPDX-License-Identifier: GPL-3.0-or-later
use super::{EconomyDate, EconomyTuning, TuningProperties, RESET_TUTORIAL_CITIES, SUBSCRIPTIONS};
use std::sync::OnceLock;

pub const REGION_SCREEN: u32 = 0xEA65_9793;
pub const TUTORIAL_RESOURCE: (u32, u32) = (0x6A5B_7BF5, 0x6A5B_7B57);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NationError {
    InvalidRegionIndex(usize),
    InvalidDifficulty(usize),
    MissingCurve(u32),
    NotInitialized,
    LastRegion,
    RegionCreationFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageOutcome {
    Ignored,
    TutorialReset,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Directories {
    pub regions: String,
    pub downloads: String,
}

/// Shared process directory strings, not tuning. Even a no-app call consumes it.
#[derive(Debug, Default)]
pub struct ProcessDirectories(OnceLock<Directories>);

impl ProcessDirectories {
    pub fn global() -> &'static Self {
        static PROCESS: ProcessDirectories = ProcessDirectories(OnceLock::new());
        &PROCESS
    }

    pub fn static_init(&self, host: &mut impl NationHost) -> &Directories {
        self.0.get_or_init(|| {
            if !host.has_app() {
                return Directories::default();
            }
            let regions = host.user_regions_root();
            if !host.directory_exists(&regions) {
                host.create_directory(&regions);
            }
            let downloads = path(&regions, "Downloads\\");
            if !host.directory_exists(&downloads) {
                host.create_directory(&downloads);
            }
            Directories { regions, downloads }
        })
    }
}

/// Game/platform adapter operations. Paths and directory enumeration order are
/// supplied by this interface; no host filesystem or COM ownership is guessed.
pub trait NationHost {
    type Region: NationRegion;
    type City: TutorialCity;
    fn has_app(&self) -> bool;
    fn has_live_city(&self) -> bool;
    fn user_regions_root(&mut self) -> String;
    fn data_directory(&self) -> String;
    fn language_directory(&self) -> Option<String>;
    fn directory_exists(&mut self, path: &str) -> bool;
    fn create_directory(&mut self, path: &str);
    fn entries(&mut self, root: &str) -> Vec<DirectoryEntry>;
    /// Recursive source-relative file paths, in provider order (not directories).
    fn recursive_files(&mut self, source: &str) -> Vec<String>;
    fn copy_overwrite(&mut self, source: &str, destination: &str);
    fn make_writable(&mut self, path: &str);
    fn remove_directory(&mut self, path: &str, recursive: bool);
    fn has_pattern(&mut self, directory: &str, pattern: &str) -> bool;
    fn new_region(&mut self) -> Option<Self::Region>;
    /// Region §6 factory, including sanitization, layout and directory effects.
    fn create_region(&mut self, name: &str) -> Option<Self::Region>;
    fn read_tuning(&mut self) -> Option<TuningProperties>;
    fn preference_name(&self) -> [u8; 64];
    fn default_tutorial_directory(&self) -> String;
    fn should_reset_tutorials(&self) -> bool;
    /// None represents an absent message server. Results are not rollback signals.
    fn subscription(&mut self, message: u32, subscribe: bool);
    fn tutorial_regions(&self) -> Vec<String>;
    fn tutorial_entries(&self, directory: &str) -> Vec<u32>;
    fn screen_class_id(&self) -> Option<u32>;
    fn destroy_screen(&mut self);
    fn recreate_region_screen(&mut self);
    fn export_tutorial(&mut self, resource: (u32, u32, u32), path: &str) -> i32;
    fn read_tutorial_city(&mut self, path: &str) -> Option<Self::City>;
    fn set_tutorial_active(&mut self, id: u32, active: bool);
}

pub trait TutorialCity {
    fn set_save_path(&mut self, path: &str);
    fn position(&self) -> (u32, u32);
    fn shutdown(&mut self) -> bool;
}

/// Region calls always use its interface. Owned handles release on Drop; borrowed
/// getters do not take or release references. Callback bools are deliberately ignored.
pub trait NationRegion {
    fn directory_name(&self) -> &str;
    fn display_name(&self) -> &[u8];
    fn set_directory_name(&mut self, name: &str);
    fn set_display_name(&mut self, name: &[u8]);
    fn read_region_ini(&mut self) -> bool;
    fn init(&mut self) -> bool;
    fn shutdown(&mut self) -> bool;
    fn has_tutorial_city(&self) -> bool;
    fn delete(&mut self);
    fn move_city(&mut self, city: &mut impl TutorialCity, x: u32, z: u32) -> bool;
}

#[derive(Clone, Debug)]
pub struct DirectoryEntry {
    pub name: String,
    /// Provider entry kind 4 is a directory; other kinds are ignored.
    pub kind: u32,
}

fn path(root: &str, suffix: &str) -> String {
    if root.is_empty() || root.ends_with(['\\', '/']) {
        format!("{root}{suffix}")
    } else {
        format!("{root}\\{suffix}")
    }
}

pub struct Nation<R> {
    directories: Directories,
    regions: Vec<R>,
    active: Option<usize>,
    tuning: EconomyTuning,
    initialized: bool,
}

impl<R: NationRegion> Default for Nation<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: NationRegion> Nation<R> {
    pub fn new() -> Self {
        Self {
            directories: Directories::default(),
            regions: Vec::new(),
            active: Some(0),
            tuning: EconomyTuning {
                curves: Default::default(),
                difficulty: Vec::new(),
            },
            initialized: false,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    pub fn get_region_directory(&self) -> &str {
        &self.directories.regions
    }
    pub fn get_region_download_directory(&self) -> &str {
        &self.directories.downloads
    }
    pub fn get_region_count(&self) -> usize {
        self.regions.len()
    }
    pub fn get_active_region_index(&self) -> Option<usize> {
        self.active
    }
    pub fn get_region(&self, index: usize) -> Result<&R, NationError> {
        self.regions.get(index).ok_or(NationError::InvalidRegionIndex(index))
    }
    pub fn get_active_region(&self) -> Result<&R, NationError> {
        self.get_region(self.active.unwrap_or(usize::MAX))
    }
    pub fn get_economy_modifier(&self, id: u32, date: EconomyDate, city: Option<usize>) -> Result<f32, NationError> {
        if !self.initialized {
            return Err(NationError::NotInitialized);
        }
        self.tuning.get_economy_modifier(id, date, city)
    }

    pub fn set_active_region(&mut self, index: usize) -> Result<(), NationError> {
        self.get_region(index)?;
        if self.active == Some(index) {
            return Ok(());
        }
        let old = self.active.unwrap_or(usize::MAX);
        self.get_region(old)?;
        self.regions[old].shutdown();
        self.active = Some(index);
        self.regions[index].init();
        Ok(())
    }

    pub fn get_index_of_current_region_setting_in_preferences(&self, preference: &[u8; 64], out: &mut usize) -> bool {
        *out = 0;
        let length = preference.iter().position(|&b| b == 0).unwrap_or(64);
        for (i, region) in self.regions.iter().enumerate() {
            let name = region.display_name();
            if &name[..name.len().min(64)] == &preference[..length] {
                *out = i;
                return true;
            }
        }
        false
    }

    pub fn discover_regions(&mut self, host: &mut impl NationHost<Region = R>) -> Result<bool, NationError> {
        let mut accepted = false;
        for entry in host.entries(&self.directories.regions) {
            if entry.kind != 4 || entry.name.eq_ignore_ascii_case("Downloads") {
                continue;
            }
            let region = host.new_region().ok_or(NationError::RegionCreationFailed)?;
            self.regions.push(region);
            let region = self.regions.last_mut().unwrap();
            region.set_directory_name(&entry.name);
            region.set_display_name(entry.name.as_bytes());
            region.read_region_ini();
            accepted = true;
        }
        Ok(accepted)
    }

    pub fn copy_region_files(host: &mut impl NationHost<Region = R>, destination: &str, source: &str) {
        for suffix in host.recursive_files(source) {
            let destination = path(destination, &suffix);
            if let Some(end) = destination.rfind(['\\', '/']) {
                let parent = &destination[..end];
                if !host.directory_exists(parent) {
                    host.create_directory(parent);
                }
            }
            host.copy_overwrite(&path(source, &suffix), &destination);
            host.make_writable(&destination);
        }
    }

    pub fn copy_new_user_regions(&mut self, host: &mut impl NationHost<Region = R>) -> Result<bool, NationError> {
        let root = self.directories.regions.clone();
        if root.is_empty() || !host.directory_exists(&root) {
            return Ok(false);
        }
        let data = host.data_directory();
        Self::copy_region_files(host, &root, &path(&data, "Regions\\"));
        if let Some(language) = host.language_directory() {
            Self::copy_region_files(host, &root, &path(&path(&data, &language), "Regions\\"));
        }
        self.discover_regions(host)
    }

    /// Original bool Init is always true; Result carries only replacement safety
    /// errors (null factory etc.), leaving earlier effects intact.
    pub fn init(
        &mut self,
        process: &ProcessDirectories,
        host: &mut impl NationHost<Region = R>,
    ) -> Result<bool, NationError> {
        if self.initialized {
            return Ok(true);
        }
        self.initialized = true;
        self.directories = process.static_init(host).clone();
        self.tuning = EconomyTuning::reload(host.read_tuning().as_ref(), std::mem::take(&mut self.tuning.difficulty));
        self.regions.clear();
        if !self.discover_regions(host)? && !self.copy_new_user_regions(host)? {
            self.add_new_region(host, "")?;
        }
        self.active = None;
        let mut selected = 0;
        if !self.get_index_of_current_region_setting_in_preferences(&host.preference_name(), &mut selected) {
            let default = host.default_tutorial_directory();
            for (index, region) in self.regions.iter_mut().enumerate() {
                if region.init() {
                    let preferred =
                        region.has_tutorial_city() && region.directory_name().to_ascii_lowercase() == default;
                    region.shutdown();
                    if preferred {
                        selected = index;
                        break;
                    }
                }
            }
        }
        self.active = Some(selected);
        let activated = self.regions[selected].init();
        for message in SUBSCRIPTIONS {
            host.subscription(message, true);
        }
        let default = host.default_tutorial_directory();
        let has_default = self
            .regions
            .iter()
            .any(|r| r.directory_name().to_ascii_lowercase() == default);
        if !activated || (!has_default && host.should_reset_tutorials()) {
            self.reset_tutorial_cities(host)?;
        }
        Ok(true)
    }

    pub fn shutdown(&mut self, host: &mut impl NationHost<Region = R>) -> bool {
        if !self.initialized {
            return true;
        }
        self.initialized = false;
        for message in SUBSCRIPTIONS {
            host.subscription(message, false);
        }
        if let Some(region) = self.active.and_then(|i| self.regions.get_mut(i)) {
            region.shutdown();
        }
        self.regions.clear();
        self.tuning.curves.clear();
        true
    }

    pub fn add_new_region(&mut self, host: &mut impl NationHost<Region = R>, name: &str) -> Result<usize, NationError> {
        let region = host.create_region(name).ok_or(NationError::RegionCreationFailed)?;
        self.regions.push(region);
        Ok(self.regions.len() - 1)
    }

    pub fn delete_region(&mut self, host: &mut impl NationHost<Region = R>, index: usize) -> Result<(), NationError> {
        self.get_region(index)?;
        if self.regions.len() == 1 {
            return Err(NationError::LastRegion);
        }
        self.set_active_region(index)?;
        self.regions[index].delete();
        let neighbor = if index + 1 < self.regions.len() {
            index + 1
        } else {
            index - 1
        };
        self.set_active_region(neighbor)?;
        host.remove_directory(
            &path(&self.directories.regions, self.regions[index].directory_name()),
            false,
        );
        self.regions.remove(index);
        self.active = Some(if neighbor > index { neighbor - 1 } else { neighbor });
        Ok(())
    }

    pub fn do_message(
        &mut self,
        host: &mut impl NationHost<Region = R>,
        message: u32,
    ) -> Result<MessageOutcome, NationError> {
        if message == RESET_TUTORIAL_CITIES {
            self.reset_tutorial_cities(host)?;
            Ok(MessageOutcome::TutorialReset)
        } else {
            Ok(MessageOutcome::Ignored)
        }
    }

    pub fn reset_tutorial_cities(&mut self, host: &mut impl NationHost<Region = R>) -> Result<(), NationError> {
        if !host.has_app() || host.has_live_city() {
            return Ok(());
        }
        let mut names = host.tutorial_regions();
        let default = host.default_tutorial_directory();
        if let Some(i) = names.iter().position(|name| name == &default) {
            let name = names.remove(i);
            names.insert(0, name);
        }
        let recreate_screen = host.screen_class_id() == Some(REGION_SCREEN);
        if recreate_screen {
            host.destroy_screen();
        }
        for name in names {
            let directory = path(&self.directories.regions, &name);
            let existing = self
                .regions
                .iter()
                .position(|r| r.directory_name().eq_ignore_ascii_case(&name));
            let mut retained = existing;
            if let Some(index) = existing {
                if !host.directory_exists(&directory)
                    || !host.has_pattern(&directory, "*.sc4")
                    || !host.has_pattern(&directory, "region.ini")
                {
                    self.delete_region(host, index)?;
                    retained = None;
                }
            } else if host.directory_exists(&directory) {
                host.remove_directory(&directory, true);
            }
            let index = if let Some(index) = retained {
                index
            } else {
                let index = self.add_new_region(host, &name)?;
                let destination = path(&self.directories.regions, self.regions[index].directory_name());
                let source = path(&path(&host.data_directory(), "Regions\\"), &name);
                Self::copy_region_files(host, &destination, &source);
                index
            };
            self.set_active_region(index)?;
            let directory = path(&self.directories.regions, self.regions[index].directory_name());
            for id in host.tutorial_entries(&name) {
                let save = path(&directory, &format!("Tutorial-City-{id:08x}.sc4"));
                let resource = (TUTORIAL_RESOURCE.0, TUTORIAL_RESOURCE.1, id);
                let mut active = false;
                if host.export_tutorial(resource, &save) != -1 {
                    if let Some(mut city) = host.read_tutorial_city(&save) {
                        city.set_save_path(&save);
                        let (x, z) = city.position();
                        self.regions[index].move_city(&mut city, x, z);
                        city.shutdown();
                        active = true;
                    }
                }
                host.set_tutorial_active(id, active);
            }
        }
        if recreate_screen {
            host.recreate_region_screen();
        }
        Ok(())
    }
}
