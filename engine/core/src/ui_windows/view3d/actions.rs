// SPDX-License-Identifier: GPL-3.0-or-later
//! Command and catalog-item decisions (§§6g, 6i). Numeric zone/network/lot
//! command inventory and registry order remain open as VW9; typed entry points
//! implement those tools without inventing IDs.
use super::{CheatHost, CommandHost, ControlCall, InputEvent, View3DWin};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolSpec {
    Zone(u32),
    Network(u32),
    Intersection(u32),
    Lot(u32),
    Dispatch(u32),
    Sign,
    Label,
    DemolishSigns,
    TerrainScrubber { item: u32, mode: u32 },
    Flora(u32),
    Torch,
    EffectDisaster(DisasterProperties),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DisasterProperties {
    pub effect_name: Option<String>,
    pub effect_group: Option<u32>,
    pub manager: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionCall {
    GridMessage(u32),
    VisualTimeMode(u32),
    PlayOptions,
    GraphicOptions,
    AudioOptions,
    FocusView,
    PhotoAlbum,
    Snapshot,
    Save { prompt: bool, quick: bool },
    Region { prompt: bool },
    Quit { prompt: bool, parameter1: u32 },
    ObliterateDialog,
    ExpandedTooltips(bool),
    ToggleSigns,
    RendererRefresh,
    AnimationPause(bool),
    AnimationStep,
    GlobalTerrain(u32),
    StartAutomata(u32),
}

pub trait ActionHost: CheatHost + CommandHost {
    fn action(&mut self, call: ActionCall);
    /// Creation returns one owned reference, including on Init failure.
    fn create_tool(&mut self, spec: ToolSpec) -> Option<u32>;
    fn expanded_tooltips(&self) -> bool;
    fn signs_shown(&self) -> bool;
    fn animation_paused(&self) -> bool;
    fn main_panel_visible(&self) -> bool;
    /// Modifier-bit interpretation belongs to the foreign window system.
    fn alt_down(&self, modifiers: u32) -> bool;
    fn cursor_in_view(&self) -> (i32, i32);
    /// Exemplar (6534284A,29B6C670,id), properties 4A08C700/702/701.
    fn disaster_properties(&mut self, id: u32) -> DisasterProperties;
}

impl View3DWin {
    fn install_tool(&mut self, host: &mut impl ActionHost, spec: ToolSpec, move_once: bool) {
        let control = host.create_tool(spec);
        self.set_current_control(host, control, 2);
        if let Some(control) = control {
            if move_once && self.owner.mouse_inside {
                let (x, y) = host.cursor_in_view();
                host.control(
                    control,
                    ControlCall::Event(InputEvent::Move(x, y, self.owner.modifiers)),
                );
            }
            host.control(control, ControlCall::Release);
        }
    }

    pub fn invoke_zoning_tool(&mut self, host: &mut impl ActionHost, zone: u32) {
        if host.city_established() && zone < 16 {
            self.install_tool(host, ToolSpec::Zone(zone), false);
        }
    }

    pub fn invoke_network_tool(&mut self, host: &mut impl ActionHost, network: u32) {
        if host.city_established() {
            self.install_tool(host, ToolSpec::Network(network), false);
        }
    }

    pub fn invoke_disaster(&mut self, host: &mut impl ActionHost, id: u32) {
        if id == 0xA9DE_8784 {
            self.install_tool(host, ToolSpec::Torch, false);
        } else {
            let properties = host.disaster_properties(id);
            if properties.effect_name.is_some() || properties.effect_group.is_some() {
                self.install_tool(host, ToolSpec::EffectDisaster(properties), false);
            }
        }
    }

    pub fn invoke_catalog_item(&mut self, host: &mut impl ActionHost, kind: u32, item: u32, close: bool) {
        if close {
            if kind == 0x6A5A_C478 {
                host.close_secondary();
            } else {
                host.close_tertiary();
            }
        }
        match kind {
            0x4A2B_2467 if host.city_established() => {
                let network = match item {
                    0x8C32_9937 => 0,
                    0x29 => 1,
                    0x31 => 2,
                    0xEA6E_379D => 3,
                    0x2992_F72C => 4,
                    0x32 => 5,
                    0x2B73_0C5B => 6,
                    0x2992_37BF => 7,
                    0x2B79_E398 => 8,
                    0x4BE0_99DE => 9,
                    0x0BE0_99C1 => 10,
                    0x2BE0_99A9 => 11,
                    0x0BE0_998F => 12,
                    _ if item >> 16 == 0x6A47 => {
                        self.install_tool(host, ToolSpec::Intersection(item & 0xFFFF), false);
                        return;
                    }
                    _ => return,
                };
                self.invoke_network_tool(host, network);
            }
            0x4A2B_2466 => self.invoke_zoning_tool(host, item),
            0x6A2B_245F if host.city_established() => self.install_tool(host, ToolSpec::Lot(item), true),
            0x4A2B_2464 => self.invoke_disaster(host, item),
            0x4A2B_2462 | 0x8A31_3C52 => self.install_tool(
                host,
                ToolSpec::TerrainScrubber {
                    item,
                    mode: u32::from(kind == 0x4A2B_2462),
                },
                false,
            ),
            0xEA5A_D08D => self.install_tool(host, ToolSpec::Flora(item), false),
            0x6A5A_C478 => host.action(ActionCall::GlobalTerrain(item)),
            0x0BB2_E43E => host.action(ActionCall::StartAutomata(item)),
            _ => {}
        }
    }

    /// Parameters are required: the original save/quit paths dereference their
    /// parameter set without a null test. Every command returns false.
    pub fn process_command(&mut self, host: &mut impl ActionHost, id: u32, parameters: [u32; 2]) -> bool {
        self.process_motion_panel_command(host, id);
        match id {
            0x6A93_5CE5 => {
                let (x, y) = host.cursor_in_view();
                if let Some(point) = host.terrain_point(x, y) {
                    host.camera_jump(point);
                }
            }
            0x6A93_5CE8 => host.action(ActionCall::GridMessage(0x277A_9364)),
            0xEC3E_82F8 if parameters[0] != 0 => self.invoke_catalog_item(host, 0x6A2B_245F, parameters[0], false),
            0x8BE0_98F4 | 0x4BE0_98F7 | 0x6BE0_98FA | 0x4BE0_98FD => {
                let network = match id {
                    0x8BE0_98F4 => 9,
                    0x4BE0_98F7 => 10,
                    0x6BE0_98FA => 11,
                    _ => 12,
                };
                self.invoke_network_tool(host, network);
            }
            0x6A93_5D39 | 0xCA93_5DBD | 0x8C0B_0E46 | 0xEC0B_0E6A if host.city_established() => {
                let dispatch = match id {
                    0x6A93_5D39 => 1,
                    0xCA93_5DBD => 0,
                    0x8C0B_0E46 => 5,
                    _ => 6,
                };
                self.install_tool(host, ToolSpec::Dispatch(dispatch), false);
            }
            0x6A93_5E27 | 0x6A93_5E1E | 0x6A93_5E21 => {
                let mode = match id {
                    0x6A93_5E27 => 0,
                    0x6A93_5E1E => 1,
                    _ => 2,
                };
                host.action(ActionCall::VisualTimeMode(mode));
            }
            0x6A93_5E2A | 0x6A93_5E2D | 0x6A93_5E34 => {
                host.action(match id {
                    0x6A93_5E2A => ActionCall::PlayOptions,
                    0x6A93_5E2D => ActionCall::GraphicOptions,
                    _ => ActionCall::AudioOptions,
                });
                host.action(ActionCall::FocusView);
            }
            0x6A93_5E45 => host.action(ActionCall::PhotoAlbum),
            0x6A93_5E4B => host.action(ActionCall::Snapshot),
            0x6A93_5E36 | 0x6A93_5E37 => host.action(ActionCall::Save {
                prompt: parameters[0] == 0,
                quick: id == 0x6A93_5E37 || host.alt_down(self.owner.modifiers),
            }),
            0x6A93_5E39 => host.action(ActionCall::Region {
                prompt: parameters[0] == 0,
            }),
            0x6A93_5E3C => host.action(ActionCall::Quit {
                prompt: parameters[0] == 0,
                parameter1: parameters[1],
            }),
            0x6A93_5E3F => {
                let minimized = host.main_panel_visible();
                self.minimize_ui(host, minimized);
            }
            0x2A94_A04B => self.toggle_contours(host),
            0x2A94_A5B6 if host.city_established() => host.action(ActionCall::ObliterateDialog),
            0x6AA9_FE51 => {
                let enabled = !host.expanded_tooltips();
                host.action(ActionCall::ExpandedTooltips(enabled));
            }
            0x0B98_0581 => self.install_tool(host, ToolSpec::Sign, false),
            0x0B98_0587 => self.install_tool(host, ToolSpec::Label, false),
            0xABCD_EE11 => self.install_tool(host, ToolSpec::DemolishSigns, false),
            0x6B98_058A => {
                if host.signs_shown() {
                    if let Some(control) = self.input_control() {
                        if matches!(
                            host.control(control, ControlCall::ToolId).tool_id,
                            Some(0x8B3C_E416 | 0x8B4C_E5A0 | 0xCBCC_987E)
                        ) {
                            self.remove_current_control(host, false);
                        }
                    }
                }
                host.action(ActionCall::ToggleSigns);
            }
            0x6A93_91A0 => host.action(ActionCall::RendererRefresh),
            0x6A97_53EA if self.owner.view.animation_debug => {
                let paused = !host.animation_paused();
                host.action(ActionCall::AnimationPause(paused));
            }
            0x6A97_53ED if self.owner.view.animation_debug => host.action(ActionCall::AnimationStep),
            0xCAB2_2C64 => {
                self.recorder(host, true);
            }
            // Deliberately unhandled here: RotateLotCW/CCW, cheat dialog,
            // RegionBitmapLoad and unknown IDs (§6g).
            _ => {}
        }
        false
    }
}
