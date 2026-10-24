// SPDX-License-Identifier: GPL-3.0-or-later
//! Command and catalog-item decisions (§§6g, 6i). Numeric zone/network/lot
//! command IDs and tool arguments follow the settled VW9 registry.
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
    LevelTerrain,
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
    ChangeTerrainType,
    CreateFlatAreas,
    CreateShoreCliffs,
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
    pub(super) fn install_tool(&mut self, host: &mut impl ActionHost, spec: ToolSpec, move_once: bool) {
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
                self.close_owner_secondary(host);
            } else {
                self.close_owner_tertiary(host);
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
            0x6A5A_C478 => self.run_global_terrain_tool(host, item),
            0x0BB2_E43E => host.action(ActionCall::StartAutomata(item)),
            _ => {}
        }
    }

    pub fn run_global_terrain_tool(&mut self, host: &mut impl ActionHost, item: u32) {
        host.popup(super::PopupCall::TerrainSound);
        match item {
            0 => host.action(ActionCall::ChangeTerrainType),
            1 => host.action(ActionCall::CreateFlatAreas),
            2 => host.action(ActionCall::CreateShoreCliffs),
            3 | 4 => {
                host.popup(super::PopupCall::TerrainLevel(item == 3));
            }
            5 | 6 => {
                host.popup(super::PopupCall::TerrainPost(0x470B_D214, item - 5));
            }
            7 => self.install_tool(host, ToolSpec::LevelTerrain, false),
            _ => {}
        }
    }

    /// Parameters are required: the original save/quit paths dereference their
    /// parameter set without a null test. Every command returns false.
    pub fn process_command(&mut self, host: &mut impl ActionHost, id: u32, parameters: [u32; 2]) -> bool {
        // §6g: registration includes unhandled commands; dispatch only the
        // identified zone/network/lot commands, preserving their city guards.
        let tool = match id {
            0x6A93_5CF7 => Some(ToolSpec::Zone(1)),
            0x6A93_5D0B => Some(ToolSpec::Zone(2)),
            0x6A93_5D0E => Some(ToolSpec::Zone(3)),
            0x6A93_5D11 => Some(ToolSpec::Zone(4)),
            0x6A93_5D17 => Some(ToolSpec::Zone(5)),
            0x6A93_5D1A => Some(ToolSpec::Zone(6)),
            0x6A93_5D1D => Some(ToolSpec::Zone(7)),
            0x6A93_5D25 => Some(ToolSpec::Zone(8)),
            0x6A93_5D2B => Some(ToolSpec::Zone(9)),
            0x6A93_5D28 => Some(ToolSpec::Zone(14)),
            0x6A93_5D2E => Some(ToolSpec::Zone(0)),
            0x6A93_5DC0 => Some(ToolSpec::Network(0)),
            0x6A93_5DDA => Some(ToolSpec::Network(1)),
            0x6A93_5DC5 => Some(ToolSpec::Network(2)),
            0x6A93_5DC2 => Some(ToolSpec::Network(3)),
            0x6A93_5DEE => Some(ToolSpec::Network(4)),
            0x6A93_5DEB => Some(ToolSpec::Network(5)),
            0x6B73_0F78 => Some(ToolSpec::Network(6)),
            0x6A93_5DE3 => Some(ToolSpec::Network(7)),
            0x2B79_E72B => Some(ToolSpec::Network(8)),
            0x6A93_5DC9 => Some(ToolSpec::Lot(0x0500_0001)),
            0x6A93_5DDD => Some(ToolSpec::Lot(0x6A8A_358F)),
            0x6A93_5DDF => Some(ToolSpec::Lot(0x6A38_55D4)),
            0x6A93_5DE6 => Some(ToolSpec::Lot(0x0500_0002)),
            0x6A93_5DF1 => Some(ToolSpec::Lot(0x69DE_426B)),
            0x6A93_5DF4 => Some(ToolSpec::Lot(0x0100_0005)),
            0x6A93_5DF7 => Some(ToolSpec::Lot(0x0100_0003)),
            0x6A93_5DFA => Some(ToolSpec::Lot(0xA9DD_1E4C)),
            0x6A93_5DFD => Some(ToolSpec::Lot(0x0100_0007)),
            0x6A93_5E0A => Some(ToolSpec::Lot(0x2A27_35A0)),
            0x6A93_5E0D => Some(ToolSpec::Lot(0xEA27_3867)),
            0x6A93_5E14 => Some(ToolSpec::Lot(0x89DD_094E)),
            0x6A93_5E11 => Some(ToolSpec::Lot(0x4A2A_F441)),
            0x6A93_5E17 => Some(ToolSpec::Lot(0x4A28_29D1)),
            0x2BFE_11C9 => Some(ToolSpec::Lot(0x4BAF_BA9F)),
            0x2BFE_1095 => Some(ToolSpec::Lot(0x0B96_AE64)),
            0x2BFE_0BEA => Some(ToolSpec::Lot(0x6BB9_42D7)),
            _ => None,
        };
        if let Some(tool) = tool {
            match tool {
                ToolSpec::Zone(zone) => self.invoke_zoning_tool(host, zone),
                ToolSpec::Network(network) => self.invoke_network_tool(host, network),
                ToolSpec::Lot(lot) => self.invoke_catalog_item(host, 0x6A2B_245F, lot, false),
                _ => unreachable!(),
            }
            return false;
        }
        // ToggleUIVisibility reads the main window's visibility once. Do not
        // also run the compatibility motion/panel toggle for this command.
        if id == 0x6A93_5E3F {
            let minimized = host.main_panel_visible();
            self.minimize_ui(host, minimized);
            return false;
        }
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
