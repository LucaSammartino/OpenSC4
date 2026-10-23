// SPDX-License-Identifier: GPL-3.0-or-later
//! Panel state and the fully identified motion/query commands (§§6g, 6h).
//! Exact registration table and grouped command IDs remain open as VW9.

use super::{ControlCall, InputHost, View3DWin, ViewChange};

#[derive(Debug)]
pub struct PanelState {
    pub current: Option<u32>,
    pub mayor_subpanel: u32,
    pub mayor_none_shown: bool,
    pub mayor_shown: bool,
    pub mysim_shown: bool,
    pub panel4_shown: bool,
    pub minimized: bool,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            current: None,
            mayor_subpanel: 0x1520_0003,
            mayor_none_shown: false,
            mayor_shown: false,
            mysim_shown: false,
            panel4_shown: false,
            minimized: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryTool {
    Query,
    TrafficQuery,
    Demolish,
}

impl QueryTool {
    fn id(self) -> u32 {
        match self {
            Self::Query => 0xC7AF_928E,
            Self::TrafficQuery => 0xC7AF_928F,
            Self::Demolish => 0x46DD_B5F1,
        }
    }
}

pub trait CommandHost: InputHost {
    fn city_established(&self) -> bool;
    fn edge_scrolling_preference(&self) -> bool;
    fn simulator_paused(&self) -> bool;
    fn pause_simulator(&mut self);
    fn resume_simulator(&mut self);
    fn set_sim_speed(&mut self, speed: u32);
    fn refresh_speed_buttons(&mut self);
    fn close_secondary(&mut self);
    fn close_tertiary(&mut self);
    fn tab_toggled(&self, id: u32) -> bool;
    fn toggle_tab(&mut self, id: u32, on: bool);
    /// Set flag 1 on each button in the panel's window set; when hiding, skip
    /// buttons with flag 0x1000 (§6h).
    fn panel_buttons(&mut self, panel: u32, visible: bool);
    fn mayor_subpanel(&mut self, id: u32, visible: bool);
    fn mysim_visible(&mut self, visible: bool);
    fn advisor_visible(&mut self, visible: bool);
    fn news_ticker_visible(&mut self, visible: bool);
    fn control_panel_visible(&mut self, visible: bool);
    fn restore_button_visible(&mut self, visible: bool);
    fn auxiliary_ui_visible(&mut self, visible: bool);
    fn rotation_ui_visible(&mut self, visible: bool);
    /// Returns an owned creation reference. Query constructor arguments are
    /// (traffic ? 1 : 0, 50); demolish uses its own control constructor.
    fn create_query_tool(&mut self, tool: QueryTool) -> Option<u32>;
}

impl View3DWin {
    fn ticker(&self, host: &mut impl CommandHost) {
        let p = &self.owner.panels;
        host.news_ticker_visible(p.mayor_shown || p.mysim_shown || p.panel4_shown);
    }

    pub fn show_mayor_subpanel(&mut self, host: &mut impl CommandHost, id: u32) {
        self.owner.panels.mayor_subpanel = id;
        if id == 0 {
            self.owner.panels.mayor_none_shown = true;
            return;
        }
        if !self.owner.panels.mayor_none_shown {
            for panel in [0x49ED_F9B7, 0x41, 0x9988_7755, 0x1520_0002, 0x1520_0003, 0xABC5_4125] {
                host.mayor_subpanel(panel, panel == id);
            }
        }
    }

    pub fn show_panel(&mut self, host: &mut impl CommandHost, panel: u32) {
        host.panel_buttons(panel, true);
        match panel {
            1 => {
                self.show_mayor_subpanel(host, self.owner.panels.mayor_subpanel);
                self.owner.panels.mayor_shown = true;
            }
            2 => {
                host.advisor_visible(false);
                host.mysim_visible(true);
                self.owner.panels.mysim_shown = true;
            }
            4 => {
                self.owner.panels.panel4_shown = true;
                self.owner.panels.mysim_shown = false;
                host.mysim_visible(false);
            }
            0 => {
                self.owner.panels.mysim_shown = false;
                host.mysim_visible(false);
            }
            _ => {}
        }
        self.ticker(host);
    }

    pub fn hide_panel(&mut self, host: &mut impl CommandHost, panel: u32) {
        host.close_secondary();
        host.panel_buttons(panel, false);
        match panel {
            1 => {
                for id in [0x49ED_F9B7, 0x41, 0x9988_7755, 0x1520_0002, 0x1520_0003, 0xABC5_4125] {
                    host.mayor_subpanel(id, false);
                }
                self.owner.panels.mayor_shown = false;
            }
            2 => {
                host.mysim_visible(false);
                self.owner.panels.mysim_shown = false;
            }
            4 => self.owner.panels.panel4_shown = false,
            _ => {}
        }
        self.ticker(host);
    }

    pub fn panel_command(&mut self, host: &mut impl CommandHost, command: u32) {
        let (tab, panel) = match command {
            0x6A93_5C34 => {
                let god = !host.city_established() || (self.owner.view.god_override && self.owner.modifiers & 7 == 7);
                (0x2988_BC85, if god { 0 } else { 4 })
            }
            0x6A93_5CA4 => (0xC988_BC79, 1),
            0x6A93_5CA8 => (0x4988_BC6A, 2),
            0x6A93_5CAB => (0x8988_BC94, 3),
            _ => return,
        };
        let on = !host.tab_toggled(tab);
        host.toggle_tab(tab, on);
        self.apply_panel_tab(host, tab, panel, on);
    }

    /// The child filter receives an already toggled tab, unlike ProcessCommand.
    pub fn apply_panel_tab(&mut self, host: &mut impl CommandHost, _tab: u32, panel: u32, on: bool) {
        host.close_secondary();
        if on {
            if let Some(old) = self.owner.panels.current {
                self.hide_panel(host, old);
                let old_tab = match old {
                    0 | 4 => 0x2988_BC85,
                    1 => 0xC988_BC79,
                    2 => 0x4988_BC6A,
                    _ => 0x8988_BC94,
                };
                host.toggle_tab(old_tab, false);
            }
            self.owner.panels.current = Some(panel);
            self.show_panel(host, panel);
        } else if self.owner.panels.current == Some(panel) {
            self.owner.panels.current = None;
            self.hide_panel(host, panel);
        }
    }

    pub fn minimize_ui(&mut self, host: &mut impl CommandHost, minimized: bool) {
        self.owner.panels.minimized = minimized;
        host.control_panel_visible(!minimized);
        if let Some(panel) = self.owner.panels.current {
            if minimized {
                self.hide_panel(host, panel);
            } else {
                self.show_panel(host, panel);
            }
        }
        host.auxiliary_ui_visible(!minimized);
        host.news_ticker_visible(!minimized);
        host.close_secondary();
        host.restore_button_visible(minimized);
        host.rotation_ui_visible(!minimized);
    }

    pub fn query_tool_command(&mut self, host: &mut impl CommandHost, tool: QueryTool) {
        if !host.city_established() {
            return;
        }
        if let Some(id) = self.input_control() {
            if !host.control(id, ControlCall::CanStack).taken {
                self.remove_current_control(host, false);
            }
        }
        if let Some(id) = self.input_control() {
            if host.control(id, ControlCall::ToolId).tool_id == Some(tool.id()) {
                return;
            }
        }
        let control = host.create_query_tool(tool);
        self.set_current_control(host, control, 0);
        if let Some(id) = control {
            host.control(id, ControlCall::Release);
        }
    }

    /// Numeric IDs explicitly given in §6g. Unidentified grouped commands still
    /// need VW9; unknown/unhandled IDs return false, just like handled IDs.
    pub fn process_motion_panel_command(&mut self, host: &mut impl CommandHost, id: u32) -> bool {
        match id {
            0x6A93_5C34 | 0x6A93_5CA4 | 0x6A93_5CA8 | 0x6A93_5CAB => self.panel_command(host, id),
            0x6A93_5CAD => {
                if host.simulator_paused() {
                    host.resume_simulator();
                } else {
                    host.pause_simulator();
                }
            }
            0x6A93_5CB0 | 0x6A93_5CB2 | 0x6A93_5CB5 => {
                let speed = match id {
                    0x6A93_5CB0 => 0,
                    0x6A93_5CB2 => 1,
                    _ => 2,
                };
                host.set_sim_speed(speed);
                if host.simulator_paused() {
                    host.resume_simulator();
                }
                host.refresh_speed_buttons();
            }
            0x6A93_5CB7 | 0x6A93_5CBA => {
                self.zoom_step(host, id == 0x6A93_5CB7);
            }
            0x6A93_5CBD | 0x6A93_5CBF | 0x6A93_5CC2 | 0x6A93_5CC4 | 0x6A93_5CC7 | 0x2BCB_D5F1 => {
                let zoom = match id {
                    0x6A93_5CBD => 0,
                    0x6A93_5CBF => 1,
                    0x6A93_5CC2 => 2,
                    0x6A93_5CC4 => 3,
                    0x6A93_5CC7 => 4,
                    _ => 5,
                };
                self.set_zoom(host, zoom);
            }
            0x6A93_5CC9 => {
                self.change_view(host, ViewChange::RotateLeft);
            }
            0x6A93_5CCC => {
                self.change_view(host, ViewChange::RotateRight);
            }
            0x6A93_5CD8 | 0x6A93_5CDD | 0x6A93_5CE0 | 0x6A93_5CE2 => {
                let direction = match id {
                    0x6A93_5CD8 => [-1, 0],
                    0x6A93_5CDD => [0, -1],
                    0x6A93_5CE0 => [1, 0],
                    _ => [0, 1],
                };
                let edge = host.edge_scrolling_preference();
                self.keyboard_scroll(host, direction, edge);
            }
            0x2A94_8275 | 0x2A94_8272 | 0x2A94_826E | 0x2A94_826B => {
                let edge = host.edge_scrolling_preference();
                self.keyboard_scroll(host, [0, 0], edge);
            }
            0x2A94_84DA | 0x2A94_84F2 | 0x2A94_84FF | 0x2A94_850B => {
                if self.owner.motion.keyboard == [0, 0] {
                    let angle = match id {
                        0x2A94_84DA => std::f32::consts::PI,
                        0x2A94_84F2 => 3.0 * std::f32::consts::FRAC_PI_2,
                        0x2A94_84FF => 0.0,
                        _ => std::f32::consts::FRAC_PI_2,
                    };
                    host.scroll_camera(angle, 1.0);
                }
            }
            0x6A93_5CF1 => {
                self.remove_all_controls(host, false);
                host.close_tertiary();
                host.close_secondary();
            }
            0x6A93_5CF3 => self.query_tool_command(host, QueryTool::Query),
            0x6A93_5CF4 => self.query_tool_command(host, QueryTool::TrafficQuery),
            0x6A93_5D36 => self.query_tool_command(host, QueryTool::Demolish),
            0x6A93_5E3F => self.minimize_ui(host, !self.owner.panels.minimized),
            _ => {}
        }
        false
    }
}
