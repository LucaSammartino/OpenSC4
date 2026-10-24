// SPDX-License-Identifier: GPL-3.0-or-later
//! Buttons and the two owner-managed popup levels (§6i, ui-windows §4).
use super::{ActionHost, CommandHost, View3DWin};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CatalogQuery {
    pub catalog: u32,
    pub kind: u32,
    pub groups: Vec<u32>,
    pub networks: Vec<u32>,
    pub zone: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatalogItem {
    pub kind: u32,
    pub item: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopupLayout {
    pub kind: u32,
    pub background: u32,
    pub items: Vec<CatalogItem>,
    pub cell: [u32; 2],
    pub spacing: u32,
    pub rows: u32,
    pub callout: [u32; 2],
    pub frame_id: u32,
    pub remembered: [u32; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PopupCall {
    Script {
        parent: u64,
        group: u32,
        instance: u32,
        data: u32,
    },
    Catalog {
        parent: u64,
        layout: PopupLayout,
    },
    Query(u32, u32),
    Filter(u32, bool),
    GenFlag(u32, u32, bool),
    Align {
        window: u32,
        anchor: u32,
        target: u32,
        flags: u32,
        margin: i32,
    },
    Front(u32),
    Show(u32),
    Hide(u32),
    Release(u32),
    ChildDelete {
        parent: u64,
        window: u32,
    },
    Execute(u32),
    DayNightNotifications,
    DayNightPreference(u32),
    DayNightButton(u32, bool),
    ShowGraphs,
    MissionButton(u32),
    NextDisaster,
    VehicleRequest(u32),
    SituationVisibility(bool),
    TerrainSound,
    TerrainLevel(bool),
    TerrainPost(u32, u32),
    TerrainBrush(u32, u32),
    ReconcileEdges,
}

pub trait PopupHost {
    /// Script/Catalog create one owned reference. Query performs QI/AddRef.
    /// Other calls return None. Window construction and drawing stay foreign.
    fn popup(&mut self, call: PopupCall) -> Option<u32>;
    fn catalog_items(&mut self, query: &CatalogQuery) -> Vec<CatalogItem>;
    /// Slot 17 then slot 9 on queried catalog state (ui-windows §4.1).
    fn popup_state(&mut self, queried: u32) -> [u32; 2];
    /// Read the selected catalog item after restoration. The two stored words
    /// remain opaque; their interpretation is owned by the foreign catalog.
    fn popup_selected(&mut self, window: u32) -> Option<CatalogItem>;
    fn popup_height(&self) -> i32;
    fn tutorial_running(&self) -> bool;
    fn utility_extra_enabled(&self, id: u32) -> bool;
    fn situation_visible(&self) -> bool;
}

#[derive(Clone, Copy, Debug)]
pub struct OwnedPopup {
    pub token: u32,
    pub memory: Option<u32>,
    pub catalog: bool,
}

impl View3DWin {
    fn close_popup(&mut self, host: &mut impl CommandHost, secondary: bool) {
        if (secondary && self.secondary_win().is_none()) || (!secondary && self.tertiary_win().is_none()) {
            return;
        }
        if secondary {
            self.close_popup(host, false);
        }
        let index = usize::from(!secondary);
        if let Some(popup) = self.owner.popups[index].take() {
            // Only tertiary writes remembered state; secondary never does.
            if !secondary && popup.catalog {
                if let Some(query) = host.popup(PopupCall::Query(popup.token, 0x8A2C_AD8B)) {
                    let state = host.popup_state(query);
                    if let Some(memory) = popup.memory {
                        self.owner.lifecycle.remembered_catalogs.insert(memory, state);
                    }
                    host.popup(PopupCall::Release(query));
                }
            }
            host.popup(PopupCall::Hide(popup.token));
            host.popup(PopupCall::Release(popup.token));
            host.popup(PopupCall::ChildDelete {
                parent: self.as_igzwin(),
                window: popup.token,
            });
            if secondary {
                self.set_secondary(None, 0);
            } else {
                self.set_tertiary(None, 0);
                self.set_catalog_state(None);
            }
        } else if secondary {
            host.close_secondary();
            self.set_secondary(None, 0);
        } else {
            host.close_tertiary();
            self.set_tertiary(None, 0);
            self.set_catalog_state(None);
        }
    }

    pub fn close_owner_secondary(&mut self, host: &mut impl CommandHost) {
        self.close_popup(host, true);
    }

    pub fn close_owner_tertiary(&mut self, host: &mut impl CommandHost) {
        self.close_popup(host, false);
    }

    fn begin_popup(&mut self, host: &mut impl CommandHost, secondary: bool, kind: u32) -> bool {
        self.remove_all_controls(host, false);
        let current = if secondary {
            self.secondary_kind()
        } else {
            self.tertiary_kind()
        };
        if current != 0 {
            self.close_popup(host, secondary);
            if current == kind {
                return false;
            }
        }
        true
    }

    pub fn invoke_script_menu(
        &mut self,
        host: &mut impl CommandHost,
        secondary: bool,
        instance: u32,
        kind: u32,
        target: u32,
        margin: i32,
    ) {
        if !self.begin_popup(host, secondary, kind) {
            return;
        }
        if let Some(token) = host.popup(PopupCall::Script {
            parent: self.as_igzwin(),
            group: 0x96A0_06B0,
            instance,
            data: target,
        }) {
            if let Some(gen) = host.popup(PopupCall::Query(token, 0x5386_D516)) {
                host.popup(PopupCall::Filter(token, true));
                host.popup(PopupCall::GenFlag(gen, 4, false));
                host.popup(PopupCall::GenFlag(gen, 1, false));
                host.popup(PopupCall::Release(gen));
            }
            host.popup(PopupCall::Align {
                window: token,
                anchor: 0xAAAA,
                target,
                flags: 0,
                margin,
            });
            self.finish_popup(
                host,
                secondary,
                kind,
                OwnedPopup {
                    token,
                    memory: None,
                    catalog: false,
                },
            );
        }
    }

    fn finish_popup(&mut self, host: &mut impl CommandHost, secondary: bool, kind: u32, popup: OwnedPopup) {
        host.popup(PopupCall::Front(popup.token));
        host.popup(PopupCall::Show(popup.token));
        if secondary {
            self.set_secondary(Some(u64::from(popup.token)), kind);
        } else {
            self.set_tertiary(Some(u64::from(popup.token)), kind);
        }
        self.owner.popups[usize::from(!secondary)] = Some(popup);
    }

    pub fn invoke_catalog_popup(
        &mut self,
        host: &mut impl ActionHost,
        secondary: bool,
        kind: u32,
        background: u32,
        memory: impl Into<Option<u32>>,
        query: CatalogQuery,
    ) {
        let memory = memory.into();
        if !self.begin_popup(host, secondary, kind) {
            return;
        }
        let items = host.catalog_items(&query);
        let max_rows = if secondary || host.popup_height() < 601 { 6 } else { 8 };
        let remembered = memory
            .and_then(|key| self.owner.lifecycle.remembered_catalogs.get(&key))
            .copied()
            .unwrap_or_default();
        let layout = PopupLayout {
            kind,
            background,
            rows: items.len().clamp(1, max_rows) as u32,
            items,
            cell: [44, 44],
            spacing: 5,
            callout: [0xA85F_1A83, 0xE9C8_6C9D],
            frame_id: 0x8A6E_61E0,
            remembered,
        };
        if let Some(token) = host.popup(PopupCall::Catalog {
            parent: self.as_igzwin(),
            layout,
        }) {
            self.finish_popup(
                host,
                secondary,
                kind,
                OwnedPopup {
                    token,
                    memory,
                    catalog: true,
                },
            );
            if !host.tutorial_running() {
                if let Some(selected) = host.popup_selected(token) {
                    let level = match selected.item {
                        0x2992_37BF => 2,
                        0x2992_F72C => 3,
                        _ => 1,
                    };
                    if self.owner.view.level == level {
                        self.invoke_catalog_item(host, selected.kind, selected.item, false);
                    }
                }
            }
        }
    }

    fn handle_menu_button(&mut self, host: &mut impl ActionHost, button: u32) -> bool {
        let script = match button {
            0x6991_EE42 => Some((true, 0x8993_02FC, 0x0992_FD17, 1, 0)),
            0x0991_EE39 => Some((true, 0xC993_0681, 0x6993_06ED, 1, 0)),
            0xA994_824D => Some((true, 0xE992_37FF, 0xC992_37A0, 1, 0)),
            0xA9ED_5617 => Some((true, 0xAA35_6502, 0xCA35_CBED, 1, 0)),
            0xE991_EE2F => Some((true, 0x4992_F764, 0xE992_F711, 1, 0)),
            0x0991_EE13 => Some((true, 0xE994_9936, 0x6992_3479, 1, 0)),
            0xABB2_7A7A => Some((true, 0x6BB2_7447, 0x8BB2_7C12, 1, 0)),
            0x36 | 0x8A32_DDDB => Some((true, 0xAAA4_4448, 0xCA35_CBED, button, 0)),
            0x49E9_5D2B => Some((true, 0xE992_3283, 0x4992_3239, button, 0)),
            0x8991_EE08 => Some((true, 0x0992_3283, 0x4992_3239, button, 0)),
            1 => Some((false, 0x09DE_3002, 0x09DE_8798, 1, -25)),
            0xAB95_37B7 => Some((false, 0xCB95_403E, 0xAB95_4023, 0xAB95_37B7, -75)),
            _ => None,
        };
        if let Some((secondary, script, kind, _target, margin)) = script {
            self.invoke_script_menu(host, secondary, script, kind, button, margin);
            if button == 0xABB2_7A7A && self.secondary_kind() == kind {
                host.popup(PopupCall::SituationVisibility(host.situation_visible()));
            }
            return true;
        }
        let mut query = CatalogQuery::default();
        let mut secondary = false;
        let mut background = 0;
        if matches!(button, 0x2B79_E398 | 0xEA6E_379D) {
            query.catalog = 0x07BD_DF1C;
            query.kind = 0x4A2B_2467;
            self.invoke_catalog_popup(host, false, button, 0x1421_5ED2, None, query);
            return true;
        }
        // Memory keys without a specified field offset are logical per-menu
        // keys, not invented layout offsets. They are never serialized.
        let mut memory = button;
        match button {
            3 | 0x34 | 0x37 | 0x38 | 0x42 | 0x0993_0709 | 0x89DD_5405 => {
                query.catalog = 0xC8DB_CCBA;
                query.kind = 0x6A2B_245F;
                query.groups = match button {
                    3 => vec![0x1006],
                    0x34 => vec![0x150B],
                    0x37 => vec![0x1500, 0x1501],
                    0x38 => vec![0x1502],
                    0x42 => vec![0x1503, 0x1504, 0x1505, 0x1506],
                    0x0993_0709 => vec![0x150A],
                    _ => vec![0x1507],
                };
                background = 0x1421_5ED4;
            }
            0x35 | 0x39 | 0x40 => {
                query.kind = 0x6A2B_245F;
                match button {
                    0x35 => {
                        query.groups = vec![0x1400, 0x1402];
                        if host.utility_extra_enabled(0xE0D0_722E) {
                            query.groups.push(0x6A65_0FFE);
                        }
                        query.networks = vec![0x4003];
                    }
                    0x39 => {
                        query.groups = vec![0x1401];
                        query.networks = vec![0x4004];
                    }
                    _ => {
                        query.groups = vec![0x1402, 0x1401];
                        query.zone = Some(0x2A49_D7CA);
                    }
                }
                background = 0x1421_5ED3;
            }
            0x29 | 0x31 | 0x2992_37BF | 0x6999_BF56 | 0x2B73_0C5B | 0xA992_34A6 | 0xE992_34B3 => {
                query.catalog = 0x07BD_DF1C;
                query.kind = 0x4A2B_2467;
                match button {
                    0x29 => {
                        query.groups = vec![0x1300];
                        query.networks = vec![0x4001];
                        memory = 0x288;
                    }
                    0x31 => {
                        query.networks = vec![0x4005];
                        memory = 0x280;
                    }
                    0x2992_37BF => {
                        query.groups = vec![0x130A];
                        query.networks = vec![0x4002];
                        memory = 0x290;
                    }
                    0x6999_BF56 | 0x2B73_0C5B => {
                        query.networks = vec![0x4000];
                        memory = 0x278;
                    }
                    0xA992_34A6 => query.groups = vec![0x1519],
                    _ => query.groups = vec![0x1508],
                }
                background = 0x1421_5ED2;
            }
            0x2992_0899 | 0xA998_AF42 | 0xC998_AF00 => {
                query.catalog = 0x2A38_58E4;
                query.kind = 0x4A2B_2466;
                let (group, key) = match button {
                    0x2992_0899 => (0x8A3F_1505, 0x20C),
                    0xA998_AF42 => (0x8A3F_14DC, 0x214),
                    _ => (0x8A3F_1502, 0x21C),
                };
                query.groups = vec![group];
                memory = key;
            }
            0x2992_3215 | 0x1522_31F3 | 0xC992_31DA | 0x6A3D_638C => {
                query.catalog = 0x6A01_FC2A;
                query.kind = 0x4A2B_2462;
                let (key, group) = match button {
                    0xC992_31DA => (0x2A0, 0x6A33_D682),
                    0x1522_31F3 => (0x2A8, 0xEA33_D694),
                    0x6A3D_638C => (0x2B0, 0x8A3F_131B),
                    _ => (0x2B8, 0x8A31_43B7),
                };
                memory = key;
                query.groups = vec![group];
                background = 0x1421_5ED0;
            }
            0x4A22_EA06 => {
                query.catalog = 0x6A01_FC2A;
                query.kind = 0xEA5A_D08D;
                query.groups = vec![0x8A26_02BB];
                memory = 0x2C8;
                background = 0x1421_5ED0;
            }
            0xEBFA_C717..=0xEBFA_C719 => {
                query.catalog = match button {
                    0xEBFA_C717 => 0x00B4_3D44,
                    0xEBFA_C718 => 0x00B4_3D4C,
                    _ => 0x00B4_3D50,
                };
                query.kind = 0x0BB2_E43E;
            }
            0x69B9_324A => {
                query.catalog = 0x29B6_C670;
                query.kind = 0x4A2B_2464;
                secondary = true;
            }
            _ => return false,
        }
        self.invoke_catalog_popup(host, secondary, button, background, memory, query);
        true
    }

    pub fn update_day_night_buttons(&self, host: &mut impl PopupHost, mode: u32) {
        for (i, id) in [0xCA35_CB74, 0xCA35_CB76, 0xCA35_CB78].into_iter().enumerate() {
            host.popup(PopupCall::DayNightButton(id, i as u32 == mode));
        }
    }

    pub fn handle_button_activated(&mut self, host: &mut impl ActionHost, button: u32) -> bool {
        let command = match button {
            0x6A4F_BB31 => Some(0x6A93_5CB0),
            0x4A4F_BB60 => Some(0x6A93_5CB2),
            0x8998_BBDF => Some(0x6A93_5CB5),
            0xC998_BB81 => Some(0x6A93_5CAD),
            0x4992_046A => Some(0x6A93_5CB7),
            0xE992_0494 => Some(0x6A93_5CBA),
            0x2A4F_BB08 => Some(0x6A93_5CC9),
            0x8A4F_BAEA => Some(0x6A93_5CCC),
            0x9988_7766 => Some(0x6A93_5CF3),
            0x8B96_B73E => Some(0x6A93_5CF4),
            0xE999_C820 => Some(0x6A93_5D36),
            0xE992_37B4 => Some(0x6A93_5DC9),
            0x2992_F72C => Some(0x6A93_5DEE),
            0xCA58_EBB7 => Some(0x6A93_5E2A),
            0x098F_4F6C => Some(0x6A93_5E2D),
            0xA98F_4F88 => Some(0x6A93_5E34),
            0x0B95_4123 => Some(0x0B98_0581),
            0xCB95_412C => Some(0x0B98_0587),
            0xEB95_41EB => Some(0x6B98_058A),
            0xABA7_ED0A => Some(0xABCD_EE11),
            0xAA55_43A1 => Some(0x2A94_A5B6),
            0x44 => Some(0x6A93_5E3F),
            _ => None,
        };
        if let Some(command) = command {
            host.popup(PopupCall::Execute(command));
            return true;
        }
        match button {
            0x30 => self.invoke_zoning_tool(host, 0),
            0x32 => self.invoke_network_tool(host, 5),
            0xABB2_7B91 | 0x09DE_87AE | 0xE9DE_87A7 => {}
            0x1520_0002 | 0x1520_0003 | 0x49ED_F9B7 | 0x9988_7755 | 0xABC5_4125 => {
                self.show_mayor_subpanel(host, button)
            }
            0x09D2_7EB0 | 0x29D2_7EC0 | 0x49D2_7ED0 | 0xAA92_11B3 => {
                host.popup(PopupCall::ShowGraphs);
            }
            0x4BB9_2C1F..=0x4BB9_2C21 => {
                host.popup(PopupCall::MissionButton(button));
            }
            0xCA35_CB74 | 0xCA35_CB76 | 0xCA35_CB78 => {
                let mode = (button - 0xCA35_CB74) / 2;
                host.popup(PopupCall::DayNightNotifications);
                host.action(super::ActionCall::VisualTimeMode(mode));
                host.popup(PopupCall::DayNightPreference(mode));
                self.update_day_night_buttons(host, mode);
            }
            0x0992_FD36 => {
                host.popup(PopupCall::NextDisaster);
            }
            0x33 | 0x69A3_0679 | 0x4BFC_8C2F | 0x6BFC_8C55 => {
                let tool = match button {
                    0x33 => 0,
                    0x69A3_0679 => 1,
                    0x4BFC_8C2F => 6,
                    _ => 5,
                };
                if host.city_established() {
                    self.install_tool(host, super::ToolSpec::Dispatch(tool), false);
                }
            }
            0xEBFA_C720 => {
                host.popup(PopupCall::SituationVisibility(!host.situation_visible()));
            }
            0xEBF9_E342 => {
                host.popup(PopupCall::VehicleRequest(0x1E));
            }
            0x0AA4_4504 | 0x0AA4_4505 => {
                host.popup(PopupCall::TerrainSound);
                host.popup(PopupCall::TerrainLevel(button == 0x0AA4_4504));
            }
            0x0AA4_4502 | 0x0AA4_4503 => {
                host.popup(PopupCall::TerrainSound);
                host.popup(PopupCall::TerrainPost(0x470B_D214, button - 0x0AA4_4502));
            }
            0x0A32_DDD2 | 0x8992_31E6 => {
                host.popup(PopupCall::TerrainBrush(6, u32::from(button == 0x8992_31E6)));
            }
            0xEA32_DDE5 | 0xEA32_DDBB | 0x4A32_DDC7 => {
                let brush = match button {
                    0xEA32_DDE5 => 0x4A52_4342,
                    0xEA32_DDBB => 0xEA33_E2FD,
                    _ => 0x0A33_E37B,
                };
                host.popup(PopupCall::TerrainBrush(brush, 0));
            }
            0x4A55_1A6B => {
                host.popup(PopupCall::ReconcileEdges);
            }
            _ => return self.handle_menu_button(host, button),
        }
        true
    }
}
