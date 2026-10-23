// SPDX-License-Identifier: GPL-3.0-or-later
//! Application-owned preference orchestration, application.md §13.3.
use super::{compatibility::erase_one_shot_x, Application};
use crate::{cmdline::CmdLine, gz_messages::Token};

/// Primitive calls to foreign managers. A queried graphics interface is owned
/// until release_graphics_interface; other manager handles are borrowed.
pub trait PreferenceHost {
    fn language_manager_present(&mut self) -> bool;
    fn language_by_name(&mut self, name: &[u8], local: bool) -> Option<u32>;
    fn installed_language(&mut self, deluxe: bool) -> Option<u32>;
    fn system_language(&mut self) -> u32;
    fn can_switch_language(&mut self, id: u32) -> bool;
    fn select_language(&mut self, id: u32);
    fn graphics_mode(&mut self) -> Option<[u32; 3]>;
    fn request_mode(&mut self, dimensions: [u32; 3]);
    fn request_fullscreen(&mut self, fullscreen: bool);
    fn query_graphics_interface(&mut self) -> Option<Token>;
    fn select_driver(&mut self, interface: Token, driver: u32);
    fn release_graphics_interface(&mut self, interface: Token);
    fn simulator(&mut self, city: Token) -> Option<Token>;
    fn set_simulator_speed(&mut self, simulator: Token, speed: u32);
    fn framework_command_line(&mut self) -> CmdLine;
    fn default_cheat(&mut self, manager: Token, value: &[u8]);
}

/// Finite representation of the native copied-command-line loop. A caller can
/// keep stepping NonProgress to reproduce repetition, without hanging the engine.
pub struct CheatRequests {
    line: CmdLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheatStep {
    Done,
    Progress,
    NonProgress,
}

impl CheatRequests {
    pub fn new(line: &CmdLine) -> Self {
        Self {
            line: CmdLine::copy_of(line),
        }
    }
    pub fn step(&mut self, manager: Token, host: &mut impl PreferenceHost) -> CheatStep {
        let mut value = Vec::new();
        if !self.line.is_switch_present_value(b"x", &mut value, true) {
            return CheatStep::Done;
        }
        host.default_cheat(manager, &value);
        if erase_one_shot_x(&mut self.line) {
            CheatStep::Progress
        } else {
            CheatStep::NonProgress
        }
    }
}

pub fn simulator_speed(input: f32) -> u32 {
    // A product of two binary32 operands fits exactly in binary64. _ftol2
    // returns integer-indefinite on NaN/out-of-i64, whose low word is zero.
    let product = f64::from(input) * f64::from(f32::from_bits(0x3DA2_37C3));
    let converted = if product.is_finite() && (-9223372036854775808.0..9223372036854775808.0).contains(&product) {
        product.trunc() as i64
    } else {
        i64::MIN
    };
    [5, 10, 15, 20, 30, 45, 60, 80][(converted as i32).clamp(0, 7) as usize]
}

impl Application {
    pub fn do_preferences_changed(&mut self, full: bool, host: &mut impl PreferenceHost) -> CheatStep {
        self.enable_full_game_pause_on_focus_loss(self.preferences.pause_on_focus_loss);
        if full {
            self.preferences_image.0[0x4EB] = 0;
            if host.language_manager_present() {
                let named = if self.preferences.language.is_empty() {
                    None
                } else {
                    host.language_by_name(&self.preferences.language, false)
                        .or_else(|| host.language_by_name(&self.preferences.language, true))
                };
                let mut selected = named.filter(|id| host.can_switch_language(*id));
                if selected.is_none() {
                    let installed = host.installed_language(true).or_else(|| host.installed_language(false));
                    selected = installed.filter(|id| host.can_switch_language(*id));
                }
                let mut defective = None;
                if selected.is_none() {
                    let last = host.system_language();
                    if host.can_switch_language(last) {
                        selected = Some(last);
                    } else {
                        for ordinal in 1..=44 {
                            if host.can_switch_language(last) {
                                defective = Some(ordinal);
                                break;
                            }
                        }
                    }
                }
                if let Some(id) = selected {
                    host.select_language(id);
                }
                if let Some(id) = selected.or(defective) {
                    self.preferences_image.0[0x4A8..0x4AC].copy_from_slice(&id.to_le_bytes());
                }
            }
            if let Some([width, height, depth]) = host.graphics_mode() {
                if !matches!(depth, 16 | 32) || width < self.preferences.width || height < self.preferences.height {
                    self.preferences.fullscreen = false;
                }
                let preferred_depth = if self.preferences.depth == 0 || self.preferences.fullscreen {
                    depth
                } else {
                    self.preferences.depth
                };
                host.request_mode([self.preferences.width, self.preferences.height, preferred_depth]);
                host.request_fullscreen(self.preferences.fullscreen);
            }
            if let Some(interface) = host.query_graphics_interface() {
                let driver = if self.driver_id != 0 {
                    self.preferences.driver_f13 = match self.driver_id {
                        0x7ACA_35C6 => 0,
                        0xBADB_6906 => 1,
                        _ => 255,
                    };
                    Some(self.driver_id)
                } else {
                    match self.preferences.driver_f13 {
                        0 => Some(0x7ACA_35C6),
                        1 => Some(0xBADB_6906),
                        _ => None,
                    }
                };
                if let Some(driver) = driver {
                    host.select_driver(interface, driver);
                }
                host.release_graphics_interface(interface);
            }
        }
        if let Some(simulator) = self.city_state.city.and_then(|city| host.simulator(city)) {
            host.set_simulator_speed(simulator, simulator_speed(self.preferences.simulation));
        }
        self.apply_command_line_cheats(self.helper(super::Helper::CheatManager), host)
    }

    pub fn apply_command_line_cheats(&self, manager: Option<Token>, host: &mut impl PreferenceHost) -> CheatStep {
        let Some(manager) = manager else {
            return CheatStep::Done;
        };
        let mut requests = CheatRequests::new(&host.framework_command_line());
        loop {
            let step = requests.step(manager, host);
            if step != CheatStep::Progress {
                return step;
            }
        }
    }
}
