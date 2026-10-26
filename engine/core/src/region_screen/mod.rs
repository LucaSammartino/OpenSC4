// SPDX-License-Identifier: GPL-3.0-or-later
//! cSC4WinRegionScreen policies from region-screen.md; no native/ABI wiring.
mod commands;
#[cfg(test)]
mod tests;

use crate::region::{CityRef, Location, Region};
use crate::region_view::{self, CityId, CityRecord, RegionKey, RegionView, Unsupported};
use crate::regional_city;
use crate::ui_windows::base::{Window, Windows};
pub use region_view::Result;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialog {
    DeleteCity,
    NewRegion,
    LoadRegion,
    DeleteRegion,
    VideoPreferences,
    GamePreferences,
    SnapshotTaker,
    SnapshotAlbum,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tunable {
    Words(Vec<u32>),
    Floats(Vec<f32>),
}

/// Primitive service calls, reached through application/window/region interfaces.
pub trait Host: region_view::Host + regional_city::Lifecycle {
    fn default_difficulty(&mut self) -> u8;
    fn localized_string(&mut self, group: u32, instance: u32) -> String;
    /// Publish the created regional-city owner and return its region-model reference.
    fn publish_city(&mut self, record: CityRecord) -> CityRef;
    fn regional_city(&mut self, city: &CityRef) -> CityRecord;
    fn load_city(&mut self, path: &str, city: &CityRef) -> bool;
    fn clear_new_city_object(&mut self);
    fn request_new_city(&mut self, city: &CityRef) -> bool;
    fn region_list(&mut self) -> Vec<(RegionKey, String)>;
    fn current_region(&mut self) -> RegionKey;
    fn set_current_region(&mut self, key: RegionKey);
    fn request_region_view(&mut self, argument: bool) -> bool;
    fn preference_region(&mut self, name: &str);
    fn preference_position(&mut self, x: i32, y: i32);
    fn save_preferences(&mut self);
    fn request_quit(&mut self);
    fn double_click_time(&mut self) -> u32;
    fn play_sound(&mut self, id: u32);
    fn close_panels(&mut self);
    fn restore_signposts(&mut self);
    /// Request the specified presentation from script-created controls, without rebuilding them.
    fn selected_presentation(&mut self, city: CityId, established: bool, instance: u32, window_id: Option<u32>);
    fn show_dialog(&mut self, dialog: Dialog, selected: Option<CityId>);
    fn post_command(&mut self, command: u32);
    fn accelerator_command(&mut self, key: u32, code: u32) -> Option<u32>;
    fn preference_visibility(&mut self, post_view_init: bool);
    fn post_camera_view_init(&mut self);
    fn reset_snapshots(&mut self, city: CityId);
    fn read_thumbnails(&mut self, city: &CityRef);
    fn default_snapshots(&mut self, city: &CityRef);
    fn population_text(&mut self, id: u32, value: i64);
    fn tuning(&mut self, property: u32) -> Option<Tunable>;
    fn cloud_opacity(&mut self, minimum: u8, maximum: u8);
}

#[derive(Debug)]
pub struct RegionScreen {
    pub window: Window,
    pub held: Option<CityId>,
    pub held_since: u32,
    pub position: (f32, f32),
    pub scroll_speed: f32,
    pub velocity: (f32, f32),
    pub preference_f0a: bool,
    /// Preserve the unknown tuning values without inventing their meanings (QR1).
    pub tunables: BTreeMap<u32, Tunable>,
}

impl RegionScreen {
    /// Scroll speed is supplied by the owner until QR1 resolves the tuning fields.
    pub fn new(window: Window, scroll_speed: f32) -> Self {
        Self {
            window,
            held: None,
            held_since: 0,
            position: (0.0, 0.0),
            scroll_speed,
            velocity: (0.0, 0.0),
            preference_f0a: false,
            tunables: BTreeMap::new(),
        }
    }

    pub fn init(&mut self, windows: &mut Windows, host: &mut impl Host) -> Result<()> {
        windows.init(host, self.window);
        // SPEC-GAP: QR1 leaves application services, GUI construction and teardown open.
        Err(Unsupported::Open("QR1"))
    }

    pub fn shutdown(&mut self, windows: &mut Windows, host: &mut impl Host) -> Result<()> {
        windows.shutdown(host, self.window);
        Err(Unsupported::Open("QR1"))
    }

    pub fn read_tuning(&mut self, host: &mut impl Host, view: &mut RegionView) -> Result<()> {
        for property in 0xca38_3ca3..=0xca38_3cb1 {
            if let Some(value) = host.tuning(property) {
                self.tunables.insert(property, value);
            }
        }
        for (id, target) in [
            (0xca38_3cab, &mut view.grid_colour),
            (0xca38_3cac, &mut view.selection_colour),
        ] {
            if let Some(Tunable::Words(values)) = self.tunables.get(&id) {
                if let Some(value) = values.first() {
                    *target = *value;
                }
            }
        }
        let opacity = match self.tunables.get(&0xca38_3cb1) {
            Some(Tunable::Words(values)) if values.len() >= 2 => (values[0] as u8, values[1] as u8),
            _ => (32, 64),
        };
        host.cloud_opacity(opacity.0, opacity.1);
        // SPEC-GAP: QR1 does not assign meanings to the other tuning values.
        Err(Unsupported::Open("QR1"))
    }

    pub fn get_or_create_city(
        region: &mut Region,
        host: &mut impl Host,
        tile: Location,
        create: bool,
    ) -> Result<Option<CityRef>> {
        if let Some(city) = region.get_city(tile.x, tile.z).and_then(|id| region.boxed_city(id)) {
            return Ok(Some(city));
        }
        if !create {
            return Ok(None);
        }
        if !(0..=2).contains(&tile.size_exp) {
            return Err(Unsupported::Precondition("tile size code"));
        }
        let name = host.localized_string(0x6a23_1eaa, 0xaa73_8c4e);
        let difficulty = host.default_difficulty();
        let record = CityRecord::placeholder(
            difficulty,
            (tile.x, tile.z),
            tile.size_exp as u32,
            name.as_bytes(),
            host,
        );
        let city = host.publish_city(record);
        region.insert_city(city);
        Ok(region.get_city(tile.x, tile.z).and_then(|id| region.boxed_city(id)))
    }

    pub fn save_position(&self, host: &mut impl Host) {
        host.preference_position(self.position.0 as i32, self.position.1 as i32);
    }
    pub fn save_region_position(&self, region: &Region, host: &mut impl Host) {
        host.preference_region(region.name());
        self.save_position(host);
    }

    pub fn launch_regional_city(&self, host: &mut impl Host, city: &CityRef) -> bool {
        self.save_position(host);
        let path = city.read().expect("regional-city lock poisoned").path.clone();
        if !path.is_empty() && host.load_city(&path, city) {
            return true;
        }
        host.clear_new_city_object();
        host.request_new_city(city)
    }

    pub fn load_region(&self, host: &mut impl Host, name: &str) -> bool {
        for (key, region_name) in host.region_list() {
            if region_name == name {
                if key == host.current_region() {
                    return true;
                }
                host.set_current_region(key);
                return host.request_region_view(false);
            }
        }
        false
    }

    pub fn launch_city(
        &self,
        region: &mut Region,
        host: &mut impl Host,
        name: Option<&str>,
        size: i32,
        empty_only: bool,
    ) -> Result<bool> {
        if let Some(name) = name {
            let mut cities = Vec::new();
            region.get_all_cities(&mut cities);
            for city in cities {
                let record = host.regional_city(&city);
                let state = city.read().expect("regional-city lock poisoned");
                let area = state.size.0 * state.size.1;
                let size_matches = size == 3 || (0..=2).contains(&size) && area == 1 << (2 * size);
                let matches = record.established && state.name == name && size_matches;
                drop(state);
                if matches && !empty_only {
                    return Ok(self.launch_regional_city(host, &city));
                }
            }
            return Ok(false);
        }
        for tile in region.city_locations() {
            if size != 3 && size != tile.size_exp {
                continue;
            }
            if region.get_city(tile.x, tile.z).is_none() {
                let city = Self::get_or_create_city(region, host, tile, true)?
                    .ok_or(Unsupported::Precondition("created regional city"))?;
                return Ok(self.launch_regional_city(host, &city));
            }
        }
        Ok(false)
    }

    pub fn update_population(&self, region: &Region, host: &mut impl Host) -> i64 {
        let mut cities = Vec::new();
        region.get_all_cities(&mut cities);
        let total = cities
            .iter()
            .map(|city| i64::from(host.regional_city(city).population))
            .sum();
        host.population_text(0xc9e4_1918, total);
        total
    }

    pub fn put_cities_in_view(&self, region: &Region, host: &mut impl Host) -> Result<()> {
        let mut cities = Vec::new();
        region.get_all_cities(&mut cities);
        if let Some(city) = cities.into_iter().next() {
            if city.read().expect("regional-city lock poisoned").path.is_empty() {
                host.default_snapshots(&city);
            } else {
                host.read_thumbnails(&city);
            }
            // SPEC-GAP: RGV5 leaves the resampled image/hit/coverage builder open.
            return Err(Unsupported::Open("RGV5"));
        }
        // SPEC-GAP: QR5 requests ComputeBaseLimits' missing projection/limit formula.
        Err(Unsupported::Open("QR5"))
    }

    pub fn mouse_move(&mut self, view: &mut RegionView, host: &mut impl Host, x: i32, y: i32) -> Result<()> {
        if self.held.is_none() {
            view.select_city(host, x, y)?;
        }
        Ok(())
    }

    pub fn mouse_down(
        &mut self,
        region: &mut Region,
        view: &mut RegionView,
        host: &mut impl Host,
        point: (i32, i32),
        now: u32,
    ) -> Result<bool> {
        let (x, y) = point;
        let hit = view.city_at_point(host, x, y);
        // SPEC-GAP: equality at the double-click deadline is unspecified; use a strict interval.
        if self.held.is_some() && self.held == hit && now.wrapping_sub(self.held_since) < host.double_click_time() {
            return self.play_selected(region, view, host);
        }
        self.held = None;
        let Some(id) = view.select_city(host, x, y)? else {
            host.restore_signposts();
            return Ok(false);
        };
        host.play_sound(0x2a5c_322b);
        host.close_panels();
        let entry = view
            .cities
            .iter()
            .find(|c| c.id == id)
            .ok_or(Unsupported::Precondition("selected entry"))?;
        let established = region
            .get_city(entry.cell.0, entry.cell.1)
            .and_then(|id| region.boxed_city(id))
            .is_some_and(|city| host.regional_city(&city).established);
        let (instance, window_id) = if established {
            (0xca53_9340, None)
        } else {
            (0x0a8c_d184, Some(0x0a55_1c50))
        };
        host.selected_presentation(id, established, instance, window_id);
        self.held = Some(id);
        self.held_since = now;
        Ok(false)
    }

    pub fn play_selected(&self, region: &mut Region, view: &RegionView, host: &mut impl Host) -> Result<bool> {
        let Some(entry) = view.cities.iter().find(|c| Some(c.id) == view.selected) else {
            return Ok(false);
        };
        let tile = Location {
            x: entry.cell.0,
            z: entry.cell.1,
            size_exp: i32::from(entry.size_code),
        };
        let city = Self::get_or_create_city(region, host, tile, true)?
            .ok_or(Unsupported::Precondition("created regional city"))?;
        Ok(self.launch_regional_city(host, &city))
    }

    pub fn scroll(&mut self, view: &mut RegionView, host: &mut impl Host, x: i32, y: i32) {
        self.position.0 += x as f32 / 256.0;
        self.position.1 += y as f32 / 256.0;
        view.set_scroll_pos(host, self.position.0 as i32, self.position.1 as i32);
    }
}
