// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable place-lot input owner, city-tools-input.md section 3. Foreign services
//! supply data/primitive operations, never native place-lot tool methods.
mod model;
pub use crate::view_input_controls::{ControlHost, Pixel, Vec3};
pub use model::*;

pub const ID: u32 = 0x88F1_54FB;
pub const CURSOR: u32 = 0x88E7_0B76;
pub const TEXT_KEY: u32 = 0xE9C8_77FA;
pub const UNKNOWN_COST: i64 = -100000;
pub const MESSAGES: [u32; 6] = [
    0x277A_9364,
    0x99EF_1142,
    0x99EF_1143,
    0x49D5_B83F,
    0xA6B7_9602,
    0xC9F7_75BB,
];

#[derive(Debug)]
pub struct PlaceLot {
    pub mode: u32,
    pub configuration: u32,
    pub exemplar: u32,
    pub monopoly: bool,
    pub initialized: bool,
    pub rotation: u8,
    pub player_rotation: u8,
    pub shown_rotation: i32,
    pub mouse: Pixel,
    pub last_placed: [u32; 4],
    pub position: Vec3,
    pub placeable: bool,
    pub feedback_flag: bool,
    pub toll_booth: bool,
    pub cost: i64,
    pub plop_cost: i64,
    pub plops_left: i32,
    pub coverage_view: u32,
    pub coverage_raised: bool,
    pub cursor_text: String,
    pub candidates: Vec<u32>,
    cursor: Option<u64>,
    view: bool,
    pub cursor_id: u32,
    pub control_id: u32,
    pub preview_effects: [Option<u64>; 2],
    pub previews: Vec<Preview>,
    pub ring: Option<u64>,
    pub constraint_effects: [Vec<u64>; 2],
    pub radius: f32,
    pub radius_auxiliary: u32,
    pub coverage_names: Option<[String; 2]>,
    pub texture: Option<(u64, f32)>,
    feedback_valid: bool,
    building_preview: bool,
}

impl PlaceLot {
    pub fn new(mode: u32, configuration: u32, exemplar: u32, monopoly: bool) -> Self {
        Self {
            mode,
            configuration,
            exemplar,
            monopoly,
            initialized: false,
            rotation: 0,
            player_rotation: 0,
            shown_rotation: -1,
            mouse: Pixel { x: 0, y: 0 },
            last_placed: [0, 0, u32::MAX, u32::MAX],
            position: Vec3 {
                x: 0.0,
                y: -f32::MAX,
                z: 0.0,
            },
            placeable: false,
            feedback_flag: false,
            toll_booth: false,
            cost: UNKNOWN_COST,
            plop_cost: UNKNOWN_COST,
            plops_left: 1000,
            coverage_view: 0,
            coverage_raised: false,
            cursor_text: String::new(),
            candidates: Vec::new(),
            cursor: None,
            view: false,
            cursor_id: 0,
            control_id: 0,
            preview_effects: [None; 2],
            previews: Vec::new(),
            ring: None,
            constraint_effects: [Vec::new(), Vec::new()],
            radius: 0.0,
            radius_auxiliary: 0,
            coverage_names: None,
            texture: None,
            feedback_valid: false,
            building_preview: false,
        }
    }

    pub fn init(&mut self, h: &mut impl Host) -> bool {
        if self.initialized {
            return true;
        }
        self.control_id = ID;
        self.cursor_id = CURSOR;
        self.set_cursor_id(h, CURSOR);
        if let Some(c) = self.cursor {
            h.apply_window_cursor(c, true);
        }
        self.view = h.has_view();
        if self.view {
            h.effect(Effect::Acquire(Service::View));
        }
        h.effect(Effect::Acquire(Service::WindowManager));
        self.initialized = true;
        for m in MESSAGES {
            h.effect(Effect::Subscribe(m));
        }
        h.effect(Effect::Acquire(Service::Configurations));
        h.effect(Effect::Acquire(Service::Buildings));
        if self.mode == 0 {
            if self.configuration == 0 {
                self.candidates = h.configurations(Filter::Monopoly(self.monopoly));
                self.configuration = pick(h, &self.candidates, self.candidates.first().copied().unwrap_or(0));
            }
            self.toll_booth = self.configuration == 0xEBF9_A440;
        } else {
            if self.exemplar == 0 {
                let entries = h.building_types();
                let last = h.last_building();
                self.exemplar = pick(
                    h,
                    &entries,
                    if last != 0 {
                        last
                    } else {
                        entries.first().copied().unwrap_or(0)
                    },
                );
                h.effect(Effect::StoreLastBuilding(self.exemplar));
            }
            // Literal section 3.2: a null building choice ends here, without the
            // Shutdown explicitly required for an empty configuration set.
            if self.exemplar == 0 {
                return false;
            }
            self.candidates = h.configurations(Filter::Building(self.exemplar));
            if self.candidates.is_empty() {
                self.shutdown(h);
                return false;
            }
            let last = h.last_configuration();
            self.configuration = pick(h, &self.candidates, last);
            h.effect(Effect::StoreLastConfiguration(self.configuration));
        }
        let building = self.resolved_building(h);
        if let Some(cost) = h.plop_cost(building) {
            self.plop_cost = cost;
        }
        self.build_preview(h);
        h.effect(Effect::GridInit);
        if self.ring.is_none() {
            self.ring = h.create_ring();
        }
        if self.coverage_view != 0 {
            h.effect(Effect::RegisterView(self.coverage_view));
        }
        true
    }

    fn resolved_building(&self, h: &mut impl Host) -> u32 {
        if self.exemplar != 0 {
            self.exemplar
        } else {
            h.configuration(self.configuration).map_or(0, |c| c.first_building)
        }
    }

    pub fn shutdown(&mut self, h: &mut impl Host) -> bool {
        if !self.initialized {
            return true;
        }
        if self.coverage_raised {
            self.coverage_raised = false;
            h.effect(Effect::ClearCoverageOverlay);
            h.effect(Effect::RecalculateCoverage);
        }
        if self.coverage_view != 0 {
            h.effect(Effect::UnregisterView(self.coverage_view));
        }
        h.effect(Effect::CursorText(TEXT_KEY, String::new()));
        h.effect(Effect::ClearCost(false));
        h.effect(Effect::CoverageCategories(0));
        h.effect(Effect::RecalculateCoverage);
        h.effect(Effect::GridShutdown);
        self.release_configuration_effects(h);
        if let Some(ring) = self.ring.take() {
            stop(h, ring);
            h.effect(Effect::ReleaseCreatedEffect(ring));
        }
        h.effect(Effect::Release(Service::Buildings));
        h.effect(Effect::Release(Service::Configurations));
        for m in MESSAGES {
            h.effect(Effect::Unsubscribe(m));
        }
        h.effect(Effect::Release(Service::WindowManager));
        if self.view {
            h.effect(Effect::Release(Service::View));
            self.view = false;
        }
        if let Some(c) = self.cursor.take() {
            h.effect(Effect::ReleaseCursor(c));
        }
        self.initialized = false;
        true
    }

    pub fn set_cursor_id(&mut self, h: &mut impl Host, id: u32) -> bool {
        let Some(c) = h.lookup_cursor(id) else {
            return false;
        };
        self.cursor_id = id;
        if self.cursor != Some(c) {
            h.effect(Effect::AcquireCursor(c));
            if let Some(old) = self.cursor.replace(c) {
                h.effect(Effect::ReleaseCursor(old));
            }
            if self.initialized {
                h.apply_window_cursor(c, true);
            }
        }
        true
    }

    fn stop_previews(&mut self, h: &mut impl Host) {
        for token in &mut self.preview_effects {
            if let Some(t) = token.take() {
                if h.effect_running(t) {
                    h.effect(Effect::StopEffect(t, 0));
                }
                h.effect(Effect::ReleaseCreatedEffect(t));
            }
        }
    }

    pub fn message(&mut self, h: &mut impl Host, message: u32) -> bool {
        match message {
            0x277A_9364 => {
                let show = !h.terrain_grid_hidden();
                h.effect(Effect::GridVisible(show));
            }
            0x49D5_B83F => {
                self.coverage_raised = false;
                if self.view {
                    h.effect(Effect::RecalculateCoverage);
                }
            }
            0x99EF_1142 | 0x99EF_1143 if !self.coverage_raised => h.effect(Effect::RecalculateCoverage),
            0xA6B7_9602 | 0xC9F7_75BB => {
                self.stop_previews(h);
                self.feedback(h, self.placeable, self.feedback_flag, self.position);
            }
            _ => {}
        }
        false
    }

    pub fn key_down(&mut self, h: &mut impl Host, key: u32, modifiers: u32) -> bool {
        if !self.view || !h.on_top() {
            return false;
        }
        let delta = match h.translate_key(key, modifiers, 1) {
            Some(0x6A93_5CD5) => 1,
            Some(0x6A93_5CD1) => 3,
            _ => return false,
        };
        self.player_rotation = (self.player_rotation + delta) % 4;
        self.rotation = self.player_rotation;
        self.mouse_move(h, self.mouse, 0x100);
        self.feedback(h, self.placeable, self.feedback_flag, self.position);
        true
    }

    pub fn mouse_move(&mut self, h: &mut impl Host, mouse: Pixel, modifiers: u32) -> bool {
        let old_text = std::mem::take(&mut self.cursor_text);
        let old_valid = self.placeable;
        let old_flag = self.feedback_flag;
        let mut point = self.position;
        self.mouse = mouse;
        // Literal 3.7 resets this and never publishes a hit-path true write.
        // CT25 asks how a successful pick enables the 3.14 visible branch.
        self.feedback_flag = false;
        if let Some(hit) = h.pick_terrain(mouse) {
            point = hit;
            let mut candidate = self.player_rotation;
            let coverage = if modifiers & 0x100 != 0 {
                self.coverage(h, point, candidate)
            } else if self.toll_booth {
                let picked = pick_toll(h, point);
                if picked.coverage.valid {
                    self.change_configuration(h, picked.configuration);
                    candidate = picked.rotation;
                    point = picked.coverage.point;
                }
                picked.coverage.valid.then_some(picked.coverage)
            } else {
                let result = self.automatic_facing(h, point);
                candidate = result.rotation;
                if result.any {
                    Some(result.coverage)
                } else {
                    None
                }
            };
            if let Some(c) = coverage {
                point = c.point;
                self.rotation = candidate;
            }
            // Literal detailed 3.7 calls this even when facing fails. The
            // previous-placeability gate in quirk 4.5 conflicts with it (CT26).
            let validation = self.can_place(h, point, candidate);
            self.cost = validation.cost;
            self.cursor_text = validation.text;
            self.placeable = coverage.is_some_and(|c| c.valid) && validation.allowed;
            if self.placeable {
                self.last_placed = [0, 0, u32::MAX, u32::MAX];
            } else {
                self.rotation = self.player_rotation;
            }
            if self.cost > 0 {
                h.effect(Effect::ShowCost(self.cost as u32));
            } else {
                h.effect(Effect::ClearCost(false));
            }
        } else {
            self.cursor_text = h.localize(0x0A41_9226, 0xCA71_1914);
            self.placeable = false;
            h.effect(Effect::ClearCost(false));
        }
        if old_text != self.cursor_text {
            h.effect(Effect::CursorText(TEXT_KEY, self.cursor_text.clone()));
        }
        if old_valid != self.placeable
            || old_flag != self.feedback_flag
            || (point.x - self.position.x).abs() > 1e-4
            || (point.y - self.position.y).abs() > 1e-4
            || (point.z - self.position.z).abs() > 1e-4
            || self.shown_rotation != i32::from(self.player_rotation)
        {
            self.feedback(h, self.placeable, self.feedback_flag, point);
            self.position = point;
            // Literal section 3.7 compares the player rotation, not auto-facing.
            self.shown_rotation = i32::from(self.player_rotation);
        }
        if h.has_window_manager() {
            h.effect(Effect::FlushInputQueue);
        }
        true
    }

    pub fn mouse_exit(&mut self, h: &mut impl Host) -> bool {
        self.feedback_flag = false;
        self.feedback(h, self.placeable, false, self.position);
        self.cursor_text.clear();
        h.effect(Effect::CursorText(TEXT_KEY, String::new()));
        h.effect(Effect::ClearCost(false));
        true
    }

    pub fn mouse_down(&mut self, h: &mut impl Host, mouse: Pixel) -> bool {
        let mut placed = false;
        if let Some(mut point) = h.pick_terrain(mouse) {
            if self.placeable {
                let result = self.place_lot(h, &mut point, self.rotation, true);
                placed = result.built;
                if placed {
                    let w = h.terrain().cell_width;
                    self.last_placed = world_rectangle(result.rect, w);
                    self.feedback(h, true, false, point);
                    self.stop_previews(h);
                    h.effect(Effect::ClearCost(true));
                    self.placeable = false;
                    self.plops_left -= 1;
                    if self.plops_left < 1 && self.view {
                        h.remove_top_control(false);
                    }
                }
            }
        }
        if !placed {
            h.post_sound(0xEA45_1903, 0xAA62_C9D5);
        }
        placed
    }

    pub const fn should_stack(&self) -> bool {
        true
    }
    /// All inherited right/up/wheel/key-up/character slots.
    pub const fn unhandled_input(&self) -> bool {
        false
    }
}

fn pick(h: &mut impl Host, values: &[u32], initial: u32) -> u32 {
    let rows: Vec<_> = values.iter().map(|&id| (id, format!("0x{id:08x}"))).collect();
    h.pick_list("Lot Plop", &rows, initial).unwrap_or(0)
}

fn world_rectangle(rect: Rect, width: f32) -> [u32; 4] {
    // Section 4.4: retain the extended product until integer truncation, then
    // store its low dword. A binary32 multiply or saturating float-to-u32 cast
    // loses this behaviour (e.g. cell 10 with width 0.7 stores 6, not 7).
    [rect.x0, rect.z0, rect.x1 + 1, rect.z1 + 1].map(|cell| (f64::from(cell) * f64::from(width)).trunc() as i64 as u32)
}

mod feedback;
mod placement;
mod toll;
use feedback::stop;
pub use toll::{pick_toll, seek_compatible};
#[cfg(test)]
mod tests;
