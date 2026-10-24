// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable cSC4Region, specified by region-model.md §§1–8 and region.md.
//! Filesystem, image decoders, resources and regional-city owners are explicit host services.
//! See README.md for owner closure and the deliberate safe treatment of dangling boxes.

mod codec;
pub use codec::{decode_bmp, encode_bmp, Config, Image, Location, RecordError};

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};

pub const REGION_IID: u32 = 0x89D2_BE26;
pub const PICTURE_TYPE: u32 = 0x856D_DBAC;
pub const PICTURE_GROUP: u32 = 0x6A1E_ED2C;
pub const DEFAULT_PICTURE: u32 = 0x2A1F_0276;
pub const FIRST_TEMPLATE: u32 = 0x6C35_68C4;
pub const CITY_RECORD: (u32, u32, u32) = (0xCA02_7EDB, 0xCA02_7EE1, 0);
pub const APP_VERSION_RECORD: (u32, u32, u32) = (0xEA3F_F4F5, 0xEA3F_F506, 0xEA3F_F4F5);
pub const TUTORIAL_TYPE: u32 = 0x6A5B_7BF5;
pub const TUTORIAL_GROUP: u32 = 0x6A5B_7B57;

/// A counted regional-city reference; an adapter can retain the actual owner's object in its
/// host. This projection contains only the properties consumed by cSC4Region, not a city codec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct City {
    pub position: (i32, i32),
    pub size: (i32, i32),
    pub serial: u32,
    pub path: String,
    pub name: String,
    pub tutorial_id: i32,
    /// SetPosition(..., true) requests cached connection recomputation from the city owner.
    pub rearrange: bool,
}

pub type CityRef = Arc<RwLock<City>>;

impl City {
    pub fn counted(self) -> CityRef {
        Arc::new(RwLock::new(self))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppVersion {
    Missing,
    Unreadable,
    Value(u32),
}

impl AppVersion {
    pub fn accepted(self) -> bool {
        matches!(self, Self::Missing | Self::Value(62..=65))
    }
}

/// Opened package. `city == None` represents a missing or failed regional-city record.
/// Host decodes foundations.md §11, including its version and collection-size limits.
pub struct CityPackage {
    pub version: AppVersion,
    pub city: Option<CityRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: String,
    pub regular_file: bool,
}

/// Calls on external owners, never direct OS or game-global access. Paths use the nation's
/// trailing-backslash directory strings. Failures must be returned, not replaced by empty data.
pub trait RegionHost {
    fn region_directory(&self) -> String;
    fn download_directory(&self) -> String;
    fn active_directory_name(&self) -> String;
    fn directory_exists(&mut self, path: &str) -> bool;
    fn create_directory(&mut self, path: &str) -> bool;
    fn remove_directory_recursive(&mut self, path: &str) -> bool;
    fn list_directory(&mut self, path: &str) -> Vec<DirectoryEntry>;
    fn file_exists(&mut self, path: &str) -> bool;
    fn read_file(&mut self, path: &str) -> Option<Vec<u8>>;
    fn write_file(&mut self, path: &str, bytes: &[u8]) -> bool;
    fn make_writable(&mut self, path: &str);
    fn delete_file(&mut self, path: &str) -> bool;
    fn rename_file(&mut self, source: &str, target: &str) -> bool;
    fn copy_file_overwriting(&mut self, source: &str, target: &str) -> bool;
    /// Decode the framework's supported image formats (BMP and PNG, RM4), top row first.
    fn load_image(&mut self, path: &str) -> Option<Image>;
    fn resource_image(&mut self, kind: u32, group: u32, instance: u32) -> Option<Image>;
    fn resource_exists(&mut self, kind: u32, group: u32, instance: u32) -> bool;
    /// Export raw resource bytes. On failure remove any partially created destination file.
    fn export_resource(&mut self, kind: u32, group: u32, instance: u32, path: &str) -> bool;
    fn localized_name(&mut self, group: u32, instance: u32) -> String;
    /// Opens/closes the package and decodes the regional-city record; does not change attributes.
    fn read_city_package(&mut self, path: &str) -> Option<CityPackage>;
    /// Update ONLY the regional-city record; preserve all unrelated package records and bytes.
    /// Called only after the core has checked the package's application version.
    fn write_city_record(&mut self, path: &str, city: &CityRef) -> bool;
    fn shutdown_city(&mut self, city: &CityRef);
    fn set_city_position(&mut self, city: &CityRef, x: i32, z: i32, rearrange: bool) {
        let mut state = city.write().expect("regional-city lock poisoned");
        state.position = (x, z);
        state.rearrange = rearrange;
    }
    fn activate_tutorial(&mut self, id: i32);
    /// Must return a uniform integer in 0..upper (the game RNG, not C rand).
    fn random_uniform(&mut self, upper: u32) -> u32;
    /// `time() - mktime(2000-01-01 00:00 local)`, in seconds.
    fn seconds_since_local_2000(&mut self) -> i64;
}

/// Process-wide region state belongs to the embedding process, not a particular region.
#[derive(Clone, Debug, Default)]
pub struct RegionGlobals {
    pub init_seen: bool,
    pub base_heights: [u32; 2],
}

impl RegionGlobals {
    /// The region screen loads the two unsigned-byte tuning properties.
    pub fn set_height_tuning(&mut self, normal: u8, alternate: u8) {
        self.base_heights = [u32::from(normal), u32::from(alternate)];
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoxId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// RM16: the original indexes arbitrary memory for a type outside the two-entry table.
    BaseTerrainType(i32),
}

#[derive(Debug)]
pub struct Region {
    initialized: bool,
    references: u32,
    destroyed: bool,
    pub unused_word: u32,
    name: String,
    directory_name: String,
    config: Config,
    locations: Vec<Location>,
    overflow: Vec<CityRef>,
    cells: BTreeMap<(i32, i32), BoxId>,
    boxes: BTreeMap<BoxId, CityRef>,
    next_box: u64,
}

impl Default for Region {
    fn default() -> Self {
        Self {
            initialized: false,
            references: 0,
            destroyed: false,
            unused_word: 0,
            name: String::new(),
            directory_name: String::new(),
            config: Config::default(),
            locations: Vec::new(),
            overflow: Vec::new(),
            cells: BTreeMap::new(),
            boxes: BTreeMap::new(),
            next_box: 0,
        }
    }
}

impl Region {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
    }
    pub fn directory_name(&self) -> &str {
        &self.directory_name
    }
    pub fn set_directory_name(&mut self, name: &str) {
        self.directory_name = name.to_owned();
    }
    pub fn initialized(&self) -> bool {
        self.initialized
    }
    pub fn destroyed(&self) -> bool {
        self.destroyed
    }
    pub fn reference_count(&self) -> u32 {
        self.references
    }
    pub fn add_ref(&mut self) -> u32 {
        self.references = self.references.wrapping_add(1);
        self.references
    }
    /// Portable identity token stands for `this`; unsupported IDs leave out untouched.
    pub fn query_interface(&mut self, iid: u32, out: &mut Option<u32>) -> bool {
        if iid == 1 || iid == REGION_IID {
            *out = Some(REGION_IID);
            self.add_ref();
            true
        } else {
            false
        }
    }
    pub fn release(&mut self, host: &mut dyn RegionHost) -> u32 {
        if self.references > 1 {
            self.references -= 1;
            self.references
        } else {
            self.shutdown(host);
            self.cells.clear();
            self.locations.clear();
            self.overflow.clear();
            self.name.clear();
            self.directory_name.clear();
            self.references = 0;
            self.destroyed = true;
            0
        }
    }
    pub fn folder(&self, host: &dyn RegionHost) -> String {
        format!("{}{}\\", host.region_directory(), self.directory_name)
    }
    pub fn base_terrain_type(&self) -> i32 {
        self.config.terrain_type
    }
    pub fn set_base_terrain_type(&mut self, value: i32) {
        self.config.terrain_type = value;
    }
    pub fn base_terrain_height(&self, globals: &RegionGlobals) -> Result<u32, Unsupported> {
        // Literal unchecked lookup would read arbitrary process memory. RM16 leaves that open.
        match self.config.terrain_type {
            0 => Ok(globals.base_heights[0]),
            1 => Ok(globals.base_heights[1]),
            t => Err(Unsupported::BaseTerrainType(t)),
        }
    }
    pub fn water_preferences(&self) -> (u8, u8) {
        (self.config.water_min as u8, self.config.water_max as u8)
    }
    pub fn load_config(&mut self, host: &mut dyn RegionHost) {
        if let Some(bytes) = host.read_file(&(self.folder(host) + "region.ini")) {
            let name = self.config.apply_ini(&bytes);
            if let Some(name) = name {
                if let Some(id) = codec::localization_id(&name) {
                    self.name = host.localized_name(0x6A23_1EAA, id);
                } else {
                    self.name = name;
                }
            }
        }
    }
    pub fn load_city_locations(&mut self, host: &mut dyn RegionHost) {
        let path = self.folder(host) + "config.bmp";
        if let Some(image) = host
            .load_image(&path)
            .or_else(|| host.resource_image(PICTURE_TYPE, PICTURE_GROUP, DEFAULT_PICTURE))
        {
            self.locations = image.locations();
        }
    }
    pub fn city_locations(&self) -> Vec<Location> {
        self.locations.clone()
    }
    pub fn bounding_rect(&self) -> [i32; 4] {
        let mut rect = [i32::MAX, i32::MAX, i32::MIN, i32::MIN];
        for location in &self.locations {
            rect[0] = rect[0].min(location.x);
            rect[1] = rect[1].min(location.z);
            rect[2] = rect[2].max(location.x);
            rect[3] = rect[3].max(location.z);
        }
        if rect[2] <= rect[0] || rect[3] <= rect[1] {
            [0; 4]
        } else {
            rect
        }
    }
    pub fn get_city(&self, x: i32, z: i32) -> Option<BoxId> {
        self.cells.get(&(x, z)).copied()
    }
    /// Stale/dangling IDs are inspectable as None, never dereferenced (RM17).
    pub fn boxed_city(&self, id: BoxId) -> Option<CityRef> {
        self.boxes.get(&id).cloned()
    }
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
    pub fn overflow(&self) -> &[CityRef] {
        &self.overflow
    }
    pub fn get_all_cities(&self, out: &mut Vec<CityRef>) {
        let mut seen = Vec::new();
        for id in self.cells.values().collect::<BTreeSet<_>>() {
            if let Some(city) = self.boxes.get(id) {
                if !seen.iter().any(|other| Arc::ptr_eq(other, city)) {
                    seen.push(city.clone());
                }
            }
        }
        // Spec choice: position order replaces heap address order. Equal positions retain box order.
        seen.sort_by_key(|city| city.read().expect("regional-city lock poisoned").position);
        out.extend(seen);
    }
    pub fn insert_city(&mut self, city: CityRef) -> BoxId {
        self.next_box += 1;
        let id = BoxId(self.next_box);
        let state = city.read().expect("regional-city lock poisoned").clone();
        self.boxes.insert(id, city);
        for_cell(&state, |x, z| {
            self.cells.insert((x, z), id);
        });
        // Replaced and zero-area boxes deliberately remain retained, matching §5.5.
        id
    }
    pub fn overlaps_occupied_cell(&self, city: &CityRef) -> bool {
        let state = city.read().expect("regional-city lock poisoned");
        let mut overlaps = false;
        for_cell(&state, |x, z| overlaps |= self.cells.contains_key(&(x, z)));
        overlaps
    }
    pub fn remove_city(&mut self, id: BoxId, host: &mut dyn RegionHost) {
        if let Some(city) = self.boxes.remove(&id) {
            let state = city.read().expect("regional-city lock poisoned").clone();
            for_cell(&state, |x, z| {
                self.cells.remove(&(x, z));
            });
            host.shutdown_city(&city);
        }
    }
    pub fn delete_city(&mut self, id: BoxId, host: &mut dyn RegionHost) {
        if let Some(city) = self.boxed_city(id) {
            let path = city.read().expect("regional-city lock poisoned").path.clone();
            self.remove_city(id, host);
            host.delete_file(&path);
        }
    }
    /// No bool result is invented for the game's undefined return channel. Failure leaves id
    /// stale; success substitutes the new holder (which can be absent for a zero-area city).
    pub fn reload_city(&mut self, id: &mut Option<BoxId>, host: &mut dyn RegionHost) {
        if let Some(city) = id.and_then(|id| self.boxed_city(id)) {
            let path = city.read().expect("regional-city lock poisoned").path.clone();
            self.remove_city(id.expect("present holder"), host);
            if let Some(city) = read_regional_city(host, &path) {
                let position = {
                    let mut state = city.write().expect("regional-city lock poisoned");
                    state.path = path;
                    state.position
                };
                self.insert_city(city);
                *id = self.get_city(position.0, position.1);
            }
        }
    }
    pub fn load_cities(&mut self, host: &mut dyn RegionHost) {
        let folder = self.folder(host);
        for city in load_cities_from_directory(host, &folder) {
            let state = city.read().expect("regional-city lock poisoned").clone();
            let mut fits = false;
            if !self.overlaps_occupied_cell(&city) {
                for location in &self.locations {
                    let size = 1u32 << location.size_exp;
                    let (x, z) = state.position;
                    if x.wrapping_sub(location.x) as u32 >= size || z.wrapping_sub(location.z) as u32 >= size {
                        continue;
                    }
                    fits = (x, z) == (location.x, location.z) && state.size.0 == size as i32;
                    break;
                }
            }
            if fits {
                self.insert_city(city);
            } else {
                self.overflow.push(city);
            }
        }
    }
    pub fn move_overflow_to_downloads(&mut self, host: &mut dyn RegionHost) {
        for city in std::mem::take(&mut self.overflow) {
            host.shutdown_city(&city);
            let state = city.read().expect("regional-city lock poisoned").clone();
            let downloads = host.download_directory();
            let target = save_path_for_city(host, &downloads, &state.name, state.serial);
            if host.copy_file_overwriting(&state.path, &target) {
                host.delete_file(&state.path);
            }
        }
    }
    pub fn init(&mut self, host: &mut dyn RegionHost, globals: &mut RegionGlobals) -> bool {
        if !self.initialized {
            self.initialized = true;
            globals.init_seen = true;
            self.load_config(host);
            self.load_city_locations(host);
            self.load_cities(host);
            self.move_overflow_to_downloads(host);
        }
        !self.cells.is_empty()
    }
    pub fn shutdown(&mut self, host: &mut dyn RegionHost) -> bool {
        if self.initialized {
            self.initialized = false;
            while let Some(id) = self.cells.values().next().copied() {
                self.remove_city(id, host);
            }
            for city in std::mem::take(&mut self.overflow) {
                host.shutdown_city(&city);
            }
            let path = self.folder(host) + "region.ini";
            host.make_writable(&path);
            for index in 0..4 {
                let exists = host.file_exists(&path);
                let previous = host.read_file(&path);
                if exists && previous.is_none() {
                    continue;
                }
                let next = self.config.write_ini_entry(previous.as_deref(), &self.name, index);
                host.write_file(&path, &next);
            }
        }
        true
    }
    pub fn delete(&mut self, host: &mut dyn RegionHost) {
        while let Some(id) = self.cells.values().next().copied() {
            self.delete_city(id, host);
        }
        let folder = self.folder(host);
        host.delete_file(&(folder.clone() + "region.ini"));
        host.delete_file(&(folder.clone() + "config.bmp"));
        host.remove_directory_recursive(&folder);
        self.shutdown(host);
    }
    pub fn create(host: &mut dyn RegionHost, name: &str) -> Self {
        let stem = sanitize_file_name(name);
        let mut candidate = stem.clone();
        let mut suffix = 1u64;
        while host.directory_exists(&format!("{}{}\\", host.region_directory(), candidate)) {
            suffix += 1;
            candidate = format!("{stem} ({suffix})");
        }
        let mut region = Self::default();
        region.set_name(name);
        region.set_directory_name(&candidate);
        let folder = region.folder(host);
        if host.create_directory(&folder) {
            let mut count = 0;
            while host.resource_exists(PICTURE_TYPE, PICTURE_GROUP, FIRST_TEMPLATE + count) {
                count += 1;
            }
            if count > 0 {
                let instance = FIRST_TEMPLATE + host.random_uniform(count);
                host.export_resource(PICTURE_TYPE, PICTURE_GROUP, instance, &(folder + "config.bmp"));
            }
        }
        region
    }
    pub fn create_starter(host: &mut dyn RegionHost) -> Self {
        Self::create(host, "")
    }
    /// A source equal to this is represented by None, avoiding two mutable aliases.
    pub fn move_city(
        &mut self,
        source: Option<&mut Region>,
        city: &CityRef,
        position: (i32, i32),
        host: &mut dyn RegionHost,
        globals: &mut RegionGlobals,
    ) -> bool {
        if let Some(occupant) = self.get_city(position.0, position.1) {
            if self.boxed_city(occupant).is_some_and(|other| Arc::ptr_eq(&other, city)) {
                return true;
            }
            self.delete_city(occupant, host);
        }
        let state = city.read().expect("regional-city lock poisoned").clone();
        let mut serial = state.serial;
        let mut fresh = create_city_serial_number(host);
        while self
            .cells
            .values()
            .any(|id| self.boxes[id].read().expect("regional-city lock poisoned").serial == serial)
        {
            serial = fresh;
            fresh = fresh.wrapping_sub(1);
        }
        if let Some(source) = source {
            source.shutdown(host);
        }
        self.shutdown(host);
        let target = active_save_path_for_city(host, &state.name, serial);
        let mut ok = host.rename_file(&state.path, &target);
        if ok {
            host.make_writable(&target);
            if let Some(reloaded) = read_regional_city(host, &target) {
                reloaded.write().expect("regional-city lock poisoned").serial = serial;
                host.set_city_position(&reloaded, position.0, position.1, true);
                // §7.3 ignores the write helper's result; RQ3 records older/newer spec conflicts.
                write_regional_city(host, &target, &reloaded);
                host.shutdown_city(&reloaded);
            } else {
                ok = false;
            }
        }
        self.init(host, globals);
        ok
    }
    pub fn reset_tutorial_city(&mut self, id: i32, host: &mut dyn RegionHost) -> bool {
        if id == -1 {
            return false;
        }
        let mut cities = Vec::new();
        self.get_all_cities(&mut cities);
        for city in cities {
            let state = city.read().expect("regional-city lock poisoned").clone();
            if state.tutorial_id != id {
                continue;
            }
            if !host.export_resource(TUTORIAL_TYPE, TUTORIAL_GROUP, id as u32, &state.path) {
                return true;
            }
            if let Some(found) = self.get_city(state.position.0, state.position.1) {
                let mut holder = Some(found);
                self.reload_city(&mut holder, host);
                if let Some(reloaded) = holder.and_then(|h| self.boxed_city(h)) {
                    reloaded.write().expect("regional-city lock poisoned").tutorial_id = id;
                }
            }
            host.activate_tutorial(id);
            return false;
        }
        false
    }
}

fn for_cell(city: &City, mut f: impl FnMut(i32, i32)) {
    for i in 0..city.size.0 {
        for j in 0..city.size.1 {
            f(city.position.0.wrapping_add(i), city.position.1.wrapping_add(j));
        }
    }
}

pub fn read_regional_city(host: &mut dyn RegionHost, path: &str) -> Option<CityRef> {
    let package = host.read_city_package(path)?;
    if package.version.accepted() {
        package.city
    } else {
        None
    }
}

pub fn write_regional_city(host: &mut dyn RegionHost, path: &str, city: &CityRef) -> bool {
    match host.read_city_package(path) {
        Some(package) if package.version.accepted() => host.write_city_record(path, city),
        _ => false,
    }
}

pub fn load_cities_from_directory(host: &mut dyn RegionHost, folder: &str) -> Vec<CityRef> {
    let mut cities = Vec::new();
    if host.directory_exists(folder) {
        for entry in host.list_directory(folder) {
            if !entry.regular_file || !entry.name.ends_with(".sc4") {
                continue;
            }
            let path = folder.to_owned() + &entry.name;
            if let Some(city) = read_regional_city(host, &path) {
                host.make_writable(&path);
                city.write().expect("regional-city lock poisoned").path = path;
                cities.push(city);
            }
        }
    }
    cities
}

pub fn sanitize_file_name(name: &str) -> String {
    // RM5: literal UTF-8 choice pending the platform code-page conversion contract.
    let mut result: String = name.chars().filter(|c| !"<>:\\/\"|*?".contains(*c)).collect();
    let mut end = result.len().min(256);
    while !result.is_char_boundary(end) {
        end -= 1;
    }
    result.truncate(end);
    result
}

pub fn save_path_for_city(host: &mut dyn RegionHost, directory: &str, name: &str, serial: u32) -> String {
    let base = sanitize_file_name(&format!("City - {name}.sc4"));
    let mut candidate = directory.to_owned() + &base;
    let mut n = 1u64;
    while host.file_exists(&candidate) {
        if serial != 0 {
            if let Some(city) = read_regional_city(host, &candidate) {
                let found = city.read().expect("regional-city lock poisoned").serial;
                if found == 0 || found == serial {
                    break;
                }
            }
        }
        n += 1;
        // RM18: truncation may have removed the extension; append the suffix in that case.
        let dot = base.rfind('.').unwrap_or(base.len());
        candidate = format!("{directory}{} ({n}){}", &base[..dot], &base[dot..]);
    }
    candidate
}

pub fn active_save_path_for_city(host: &mut dyn RegionHost, name: &str, serial: u32) -> String {
    let directory = format!("{}{}\\", host.region_directory(), host.active_directory_name());
    save_path_for_city(host, &directory, name, serial)
}

pub fn create_city_serial_number(host: &mut dyn RegionHost) -> u32 {
    let seconds = host.seconds_since_local_2000().wrapping_add(28_800) as u32;
    seconds.wrapping_shl(4) | host.random_uniform(16)
}

#[cfg(test)]
mod tests;
