// SPDX-License-Identifier: GPL-3.0-or-later
//! View filters, levels, cheats and global triggers (view3d-window §§3.1, 6f, 6k).

use std::collections::BTreeMap;

use super::{ControlCall, InputHost, SaveRef, View3DWin};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FilterInfo {
    pub token: u32,
    /// Only populated for the realistic-filter interface and matching type.
    pub realistic_flags: Option<[bool; 3]>,
}

#[derive(Debug)]
pub struct ViewState {
    pub filter: Option<FilterInfo>,
    pub level: u32,
    pub terrain_query: bool,
    pub animation_debug: bool,
    pub all_rewards: bool,
    pub god_override: bool,
    pub not_loaded: bool,
    pub references: [SaveRef; 4],
    pub last_building: Option<u32>,
    pub last_lot: Option<u32>,
    pub contours_shown: bool,
    pub contour_layer: Option<i32>,
    pub(crate) triggers: BTreeMap<u32, u32>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            filter: None,
            level: 1,
            terrain_query: false,
            animation_debug: false,
            all_rewards: false,
            god_override: true,
            not_loaded: true,
            references: [SaveRef::null(); 4],
            last_building: None,
            last_lot: None,
            contours_shown: false,
            contour_layer: None,
            triggers: BTreeMap::new(),
        }
    }
}

/// Synchronous MessageServer2 sends, filter references and the terrain-view layer
/// list are foreign. A send can inspect the old filter before replacement.
pub trait StateHost {
    fn send(&mut self, message: u32, data1: u32, data2: u32);
    fn hold_filter(&mut self, token: u32);
    fn release_filter(&mut self, token: u32);
    fn add_contour_layer(&mut self, layer_type: u32) -> Option<i32>;
    fn remove_contour_layer(&mut self, id: i32);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheatTool {
    GameOfLife,
    PlaceLot { building: bool },
    TollBooth,
}

pub trait CheatHost: InputHost + StateHost {
    /// Each creation returns one owned reference, including filters.
    fn create_cheat_control(&mut self, tool: CheatTool) -> Option<u32>;
    fn create_realistic_filter(&mut self, flags: [bool; 3]) -> Option<FilterInfo>;
    fn camera_size(&mut self, size: f32);
    /// Foreign cSC4RenderProperties parses and sets the property. On success,
    /// the owner posts the render-property change notification.
    fn render_property(&mut self, name: &str, value: &str) -> bool;
    fn post_render_property_change(&mut self, message: u32);
    fn recorder_child_exists(&self) -> bool;
    /// Create the recorder child with ID A802B4EB and output directory
    /// UserDataDirectory/RecordedAnimations/. Allocation failure is tolerated.
    fn create_recorder_child(&mut self, id: u32, subdirectory: &str);
    fn remove_recorder_child(&mut self, id: u32);
}

impl View3DWin {
    pub fn get_view_level_info(&self, out: &mut u32) {
        *out = self.owner.view.level;
    }

    pub fn set_view_level_info(&mut self, host: &mut impl StateHost, level: &u32) {
        self.set_view_level(host, *level);
    }

    pub fn terrain_query_enabled(&self) -> bool {
        self.owner.view.terrain_query
    }

    pub fn last_selected_building(&self) -> Option<u32> {
        self.owner.view.last_building
    }

    pub fn set_last_selected_building(&mut self, id: u32) {
        self.owner.view.last_building = Some(id);
    }

    pub fn last_selected_lot(&self) -> Option<u32> {
        self.owner.view.last_lot
    }

    pub fn set_last_selected_lot(&mut self, id: u32) {
        self.owner.view.last_lot = Some(id);
    }

    pub fn view_state_mut(&mut self) -> &mut ViewState {
        &mut self.owner.view
    }

    pub fn get_filter(&self, host: &mut impl StateHost) -> Option<FilterInfo> {
        if let Some(filter) = self.owner.view.filter {
            host.hold_filter(filter.token);
        }
        self.owner.view.filter
    }

    pub fn set_filter(&mut self, host: &mut impl StateHost, new: Option<FilterInfo>) {
        let old = self.owner.view.filter;
        host.send(0x06CD_B65A, old.map_or(0, |f| f.token), new.map_or(0, |f| f.token));
        if let Some(f) = new {
            host.hold_filter(f.token);
        }
        self.owner.view.filter = new;
        if let Some(f) = old {
            host.release_filter(f.token);
        }
        host.send(0x06CD_B65B, 0, 0);
    }

    pub fn set_view_level(&mut self, host: &mut impl StateHost, level: u32) {
        if self.owner.view.level != level {
            self.owner.view.level = level;
            host.send(0xC6CD_BF1A, 0, 0);
            host.send(0xC6CD_BF1B, 0, 0);
        }
    }

    pub fn toggle_contours(&mut self, host: &mut impl StateHost) {
        let state = &mut self.owner.view;
        state.contours_shown = !state.contours_shown;
        if state.contours_shown {
            state.contour_layer = host.add_contour_layer(0x07F9_4205);
        } else if let Some(id) = state.contour_layer.take() {
            host.remove_contour_layer(id);
        }
    }

    pub fn global_trigger_state(&self, trigger: u32) -> bool {
        self.owner.view.triggers.contains_key(&trigger)
    }

    pub fn assert_global_trigger(&mut self, host: &mut impl StateHost, trigger: u32) {
        use std::collections::btree_map::Entry;
        match self.owner.view.triggers.entry(trigger) {
            Entry::Occupied(mut e) => *e.get_mut() = e.get().wrapping_add(1),
            Entry::Vacant(e) => {
                e.insert(1);
                host.send(0xAA37_5FE8, trigger, 1);
            }
        }
    }

    pub fn retract_global_trigger(&mut self, host: &mut impl StateHost, trigger: u32) {
        if let Some(count) = self.owner.view.triggers.get_mut(&trigger) {
            if *count > 1 {
                *count -= 1;
            } else {
                self.owner.view.triggers.remove(&trigger);
                host.send(0xAA37_5FE8, trigger, 0);
            }
        }
    }

    pub fn recorder(&mut self, host: &mut impl CheatHost, add: bool) -> bool {
        match (add, host.recorder_child_exists()) {
            (true, false) => host.create_recorder_child(0xA802_B4EB, "RecordedAnimations/"),
            (false, true) => host.remove_recorder_child(0xA802_B4EB),
            _ => {}
        }
        true
    }

    /// Args include the cheat word at index 0; splitting belongs to the foreign
    /// command-line interface (§3.1), not Rust whitespace tokenization.
    pub fn process_cheat(&mut self, host: &mut impl CheatHost, id: u32, args: &[&str]) -> bool {
        let tool = match id {
            0x8973_A69C => Some(CheatTool::GameOfLife),
            0x4AC0_96C6 => Some(CheatTool::PlaceLot { building: false }),
            0x4AC0_96CA => Some(CheatTool::PlaceLot { building: true }),
            0x0BFD_8064 => Some(CheatTool::TollBooth),
            _ => None,
        };
        if let Some(tool) = tool {
            let control = host.create_cheat_control(tool);
            self.set_current_control(host, control, 2);
            if let Some(id) = control {
                host.control(id, ControlCall::Release);
            }
            return true;
        }
        match id {
            0xCA7B_C5B5 => {
                let Some(text) = args.get(1) else {
                    return false;
                };
                // f32::clamp preserves a NaN input, as do the spec's comparisons.
                let size = atof(text).clamp(0.9, 64.0);
                host.camera_size(size);
                true
            }
            0xAA7A_2746 | 0x6A7A_276E => {
                let old = self.owner.view.filter.and_then(|f| f.realistic_flags);
                let mut flags = [false; 3];
                if let Some(old) = old {
                    flags = if id == 0xAA7A_2746 {
                        [old[0], !old[1], old[2]]
                    } else {
                        [!old[0], old[1], false]
                    };
                }
                if let Some(filter) = host.create_realistic_filter(flags) {
                    self.set_filter(host, Some(filter));
                    host.release_filter(filter.token);
                }
                // Allocation failure is explicitly allowed to skip this cheat.
                false
            }
            0x8AB7_30AE => {
                self.owner.view.terrain_query = !args.get(1).is_some_and(|s| s.eq_ignore_ascii_case("off"));
                true
            }
            0x88F3_D4BB | 0x88F3_D4BC => {
                let Some(name) = args.get(1) else {
                    return false;
                };
                self.set_render_property(host, name, args.get(2).copied().unwrap_or(""));
                true
            }
            0x6976_AFA0 => self.recorder(host, true),
            0x4A97_2F37 => {
                self.owner.view.animation_debug = true;
                true
            }
            0x4A82_253E => {
                self.owner.view.all_rewards = true;
                true
            }
            _ => false,
        }
    }

    pub fn set_render_property(&mut self, host: &mut impl CheatHost, name: &str, value: &str) -> bool {
        if host.render_property(name, value) {
            host.post_render_property_change(0xC9F7_75BB);
            true
        } else {
            false
        }
    }
}

/// Decimal C-atof prefix semantics, including trailing junk and incomplete
/// exponents. No locale is guessed: a foreign command-line adapter may supply
/// numeric arguments in the spec's dot-decimal form. Locale behavior is VW12.
fn atof(text: &str) -> f32 {
    let text = text.trim_start();
    let bytes = text.as_bytes();
    let mut i = usize::from(matches!(bytes.first(), Some(b'+') | Some(b'-')));
    let start = i;
    let special = text[start..].to_ascii_lowercase();
    if special.starts_with("infinity") || special.starts_with("inf") {
        return if bytes.first() == Some(&b'-') {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };
    }
    if special.starts_with("nan") {
        return f32::NAN;
    }
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
    }
    if i == start || (i == start + 1 && bytes.get(start) == Some(&b'.')) {
        return 0.0;
    }
    if matches!(bytes.get(i), Some(b'e') | Some(b'E')) {
        let exponent = i;
        i += 1;
        if matches!(bytes.get(i), Some(b'+') | Some(b'-')) {
            i += 1;
        }
        let digits = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == digits {
            i = exponent;
        }
    }
    text[..i].parse().unwrap_or(0.0)
}
