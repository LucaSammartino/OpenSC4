// SPDX-License-Identifier: GPL-3.0-or-later
//! GUI lifetime and reference ownership (view3d-window §6b).
use super::{CommandHost, LifecycleHost, View3DWin};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelperKind {
    PanelList,
    NewsTicker,
    MySim,
    Graphs,
    BuildingSelect,
    Budget,
    Callout,
    Bmp,
    AdvisorTracker,
    Rotation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuiCall {
    Script {
        instance: u32,
        id: u32,
        iid: u32,
    },
    Create {
        class: u32,
        iid: u32,
    },
    Helper(HelperKind),
    Query(u32, u32),
    Child {
        parent: u64,
        id: u32,
        iid: u32,
    },
    RecursiveChild {
        id: u32,
        iid: u32,
    },
    Filter(u32, bool),
    Flag(u32, u32, bool),
    ClearFocus(u32),
    Move(u32, i32, i32),
    Align(u32, u32),
    AddChild(u32),
    RemoveChild(u32),
    Visible(u32, bool),
    SetId(u32, u32),
    NotifyTarget(u32, u32),
    SetWinProc(u32, u32),
    Init(u32, Option<u32>),
    Shutdown(u32),
    Release(u32),
    Destroy(u32),
    Image {
        type_id: u32,
        group: u32,
        instance: u32,
    },
    ButtonImage {
        button: u32,
        image: u32,
        mode: u32,
        colour_key: [u8; 3],
    },
    ButtonStyle(u32, u32, bool),
    Tooltip(u32, u32, u32),
    CalloutStyle(u32, u32, u32),
    CalloutImage(u32, u32, i32, i32),
    CreateBuffer,
    SurfaceSize(u32, u32, u32, [u32; 2]),
    BmpSurface(u32, u32),
    BmpArea(u32, i32, i32, u32, u32),
    Area(u32, i32, i32, i32, i32),
    EnableChild(u32, bool),
    ClearScrollRenderSettings,
}

/// Query, child lookup, script/create/image/buffer and helper calls return an
/// owned reference or None. Other operations return None. Width/height are
/// foreign window geometry; script keys always have type 0/group 96A006B0.
pub trait GuiHost: CommandHost + LifecycleHost + super::StateHost {
    fn gui(&mut self, call: GuiCall) -> Option<u32>;
    fn client_size(&self) -> (i32, i32);
    fn window_size(&self, window: u32) -> (i32, i32);
}

#[derive(Debug, Default)]
pub struct GuiState {
    /// One owned reference per occupied field, including queried interfaces.
    pub fields: BTreeMap<u32, u32>,
    /// One owned reference per entry. Per-panel lists borrow these tokens.
    pub windows: Vec<u32>,
    pub panels: [Vec<u32>; 5],
    /// Borrowed stable window handle: the parent owns its lifetime.
    pub overlay: Option<u32>,
}

impl View3DWin {
    fn gui_script(&self, host: &mut impl GuiHost, instance: u32, id: u32, iid: u32) -> Option<u32> {
        host.gui(GuiCall::Script { instance, id, iid })
    }

    fn gui_helper(&mut self, host: &mut impl GuiHost, offset: u32, kind: HelperKind, window: Option<u32>) {
        if let Some(object) = host.gui(GuiCall::Helper(kind)) {
            self.owner.gui.fields.insert(offset, object);
            host.gui(GuiCall::Init(object, window));
        }
    }

    fn gui_align(&self, host: &mut impl GuiHost, window: u32) {
        if let Some(&main) = self.owner.gui.fields.get(&0x18C) {
            host.gui(GuiCall::Align(window, main));
        }
    }

    fn gui_panel(&self, host: &mut impl GuiHost, window: u32) {
        host.gui(GuiCall::Filter(window, true));
        host.gui(GuiCall::Flag(window, 0x800, true));
        host.gui(GuiCall::ClearFocus(window));
    }

    pub fn gui_state(&self) -> &GuiState {
        &self.owner.gui
    }

    /// There is deliberately no second-run guard (§6b). Normal lifetime is one
    /// AddGUI/RemoveGUI pair; a foreign allocation failure leaves a null field.
    /// VW14: the spec does not settle all null-creation paths; skip dependent
    /// configuration instead of calling through a missing foreign object.
    pub fn add_gui(&mut self, host: &mut impl GuiHost) {
        let (width, height) = host.client_size();
        if let Some(main) = self.gui_script(host, 0xC973_B411, 0x0987_B48F, 0x22BA_0121) {
            self.owner.gui.fields.insert(0x18C, main);
            if let Some(generator) = host.gui(GuiCall::Query(main, 0x5386_D516)) {
                self.owner.gui.fields.insert(0x190, generator);
            }
            host.gui(GuiCall::Filter(main, true));
            let (_, panel_height) = host.window_size(main);
            let x = 5 + if width > height.saturating_mul(3) { width / 3 } else { 0 };
            host.gui(GuiCall::Move(main, x, height - panel_height + 11));
            host.gui(GuiCall::Flag(main, 0x800, true));
            host.gui(GuiCall::ClearFocus(main));
        }
        if let Some(restore) = host.gui(GuiCall::Create {
            class: 0x22EC_FC47,
            iid: 0x22BA_0121,
        }) {
            self.owner.gui.fields.insert(0x194, restore);
            if let Some(button) = host.gui(GuiCall::Query(restore, 0x8810)) {
                if let Some(image) = host.gui(GuiCall::Image {
                    type_id: 0x856D_DBAC,
                    group: 0x46A0_06B0,
                    instance: 0x5324_4588,
                }) {
                    host.gui(GuiCall::ButtonImage {
                        button,
                        image,
                        mode: 2,
                        colour_key: [255, 0, 255],
                    });
                    host.gui(GuiCall::Release(image));
                }
                host.gui(GuiCall::ButtonStyle(button, 0x20, true));
                host.gui(GuiCall::Tooltip(button, 0x6A23_1EAA, 0x8A51_545D));
                host.gui(GuiCall::Flag(restore, 4, true));
                if let Some(&main) = self.owner.gui.fields.get(&0x18C) {
                    host.gui(GuiCall::NotifyTarget(restore, main));
                }
                host.gui(GuiCall::SetId(restore, 0x43));
                host.gui(GuiCall::Move(restore, 12, height - 28));
                host.gui(GuiCall::AddChild(restore));
                host.gui(GuiCall::Visible(restore, false));
                host.gui(GuiCall::ClearFocus(restore));
                host.gui(GuiCall::Release(button));
            }
        }
        self.gui_helper(host, 0x2D8, HelperKind::PanelList, None);
        // NewsTicker::Init is delayed until its script window exists.
        if let Some(ticker) = host.gui(GuiCall::Helper(HelperKind::NewsTicker)) {
            self.owner.gui.fields.insert(0x2DC, ticker);
        }
        for (i, (script, id, extra)) in [
            (0x69E3_D347, 0xC991_EDA8, None),
            (0x2BC9_0671, 0x69E4_0A1F, Some(0xE988_9775)),
            (0xAA1F_1F57, 0xCA1F_1D9C, Some(0xABB2_6B0E)),
            (0x4988_9894, 0xC98F_49F1, None),
            (0xAA53_E3EA, 0x0A78_827A, None),
        ]
        .into_iter()
        .enumerate()
        {
            if let Some(window) = self.gui_script(host, script, id, 0x22BA_0121) {
                self.owner.gui.windows.push(window);
                self.owner.gui.panels[i].push(window);
                if let Some(generator) = host.gui(GuiCall::Query(window, 0x5386_D516)) {
                    self.owner.gui.fields.insert(0x1E0 + i as u32 * 4, generator);
                    if let Some(&proc) = self.owner.gui.fields.get(&0x2D8) {
                        host.gui(GuiCall::SetWinProc(generator, proc));
                    }
                }
                self.gui_panel(host, window);
                self.gui_align(host, window);
                if let Some(id) = extra {
                    if let Some(child) = host.gui(GuiCall::Child {
                        parent: u64::from(window),
                        id,
                        iid: 0x5386_D516,
                    }) {
                        self.owner.gui.windows.push(child);
                        self.owner.gui.panels[i].push(child);
                    }
                }
                if i == 3 {
                    self.gui_helper(host, 0x2E4, HelperKind::MySim, Some(window));
                }
            }
        }
        for (offset, parent, id) in [
            (0x310, self.owner.gui.fields.get(&0x18C).copied().map(u64::from), 1),
            (0x314, Some(self.as_igzwin()), 0x09E4_18FE),
            (0x318, Some(self.as_igzwin()), 0xC9E4_1918),
        ] {
            if let Some(parent) = parent {
                if let Some(text) = host.gui(GuiCall::Child {
                    parent,
                    id,
                    iid: 0x212C_DC1F,
                }) {
                    self.owner.gui.fields.insert(offset, text);
                }
            }
        }
        if let Some(ticker) = self.gui_script(host, 0x2A2A_ED99, 0xCA2A_EDC0, 0x5386_D516) {
            if let Some(&proc) = self.owner.gui.fields.get(&0x2DC) {
                host.gui(GuiCall::Init(proc, Some(ticker)));
            }
            self.gui_align(host, ticker);
            self.owner.gui.windows.push(ticker);
        }
        for (script, id, field, helper) in [
            (0x4BC9_06B5, 0x6A64_E3C0, 0x30C, None),
            (0x2BC9_060F, 0x4200, 0x300, None),
            (0x6BC9_065A, 0x0A4A_8176, 0x304, Some((0x2F8, HelperKind::Graphs))),
            (
                0x6BC6_1F19,
                0xABC6_19D2,
                0x308,
                Some((0x2FC, HelperKind::BuildingSelect)),
            ),
        ] {
            if let Some(window) = self.gui_script(host, script, id, 0x22BA_0121) {
                self.owner.gui.fields.insert(field, window);
                self.gui_align(host, window);
                if let Some((offset, kind)) = helper {
                    self.gui_helper(host, offset, kind, Some(window));
                }
            }
        }
        self.gui_helper(host, 0x2E8, HelperKind::Budget, None);
        self.add_gui_auxiliary(host, width, height);
        if !host.city_established() {
            self.owner.panels.current = Some(0);
            self.show_panel(host, 0);
            for id in [0x9988_7766, 0x8B96_B73E] {
                host.gui(GuiCall::EnableChild(id, false));
            }
        }
    }

    fn add_gui_auxiliary(&mut self, host: &mut impl GuiHost, width: i32, height: i32) {
        if let Some(callout) = host.gui(GuiCall::Helper(HelperKind::Callout)) {
            self.owner.gui.fields.insert(0x3D8, callout);
            host.gui(GuiCall::CalloutStyle(callout, 11, 0xA85F_1A83));
            host.gui(GuiCall::CalloutStyle(callout, 10, 0xE9C8_6C9D));
            host.gui(GuiCall::Visible(callout, false));
            if let Some(image) = host.gui(GuiCall::Image {
                type_id: 0x856D_DBAC,
                group: 0x46A0_06B0,
                instance: 0x1441_6190,
            }) {
                host.gui(GuiCall::CalloutImage(callout, image, -1, -1));
                host.gui(GuiCall::Release(image));
            }
            host.gui(GuiCall::AddChild(callout));
        }
        if let Some(image) = host.gui(GuiCall::Image {
            type_id: 0x856D_DBAC,
            group: 0x46A0_06B0,
            instance: 0x1441_6160,
        }) {
            self.owner.gui.fields.insert(0x3E0, image);
        }
        if let Some(bmp) = host.gui(GuiCall::Helper(HelperKind::Bmp)) {
            self.owner.gui.fields.insert(0x3D4, bmp);
            if let Some(surface) = host.gui(GuiCall::CreateBuffer) {
                self.owner.gui.fields.insert(0x3D0, surface);
                host.gui(GuiCall::SurfaceSize(surface, 128, 32, [1, 0]));
                host.gui(GuiCall::BmpSurface(bmp, surface));
                host.gui(GuiCall::BmpArea(bmp, 0, 0, 128, 32));
            }
            host.gui(GuiCall::AddChild(bmp));
        }
        self.gui_helper(host, 0x31C, HelperKind::AdvisorTracker, None);
        if let Some(overlay) = host.gui(GuiCall::Create {
            class: 0xCA5D_3294,
            iid: 0xCA5D_3290,
        }) {
            host.gui(GuiCall::Area(overlay, 0, 0, width, height));
            host.gui(GuiCall::SetId(overlay, 0x6A5E_44B6));
            for (flag, on) in [(0x10000, false), (0x200000, true), (4, true)] {
                host.gui(GuiCall::Flag(overlay, flag, on));
            }
            host.gui(GuiCall::AddChild(overlay));
            self.owner.gui.overlay = Some(overlay);
            host.gui(GuiCall::Release(overlay));
        }
        if let Some(window) = host.gui(GuiCall::RecursiveChild {
            id: 0xEA8C_AD14,
            iid: 0x5386_D516,
        }) {
            if let Some(proc) = host.gui(GuiCall::Helper(HelperKind::Rotation)) {
                host.gui(GuiCall::SetWinProc(window, proc));
                host.gui(GuiCall::Release(proc));
            }
            host.gui(GuiCall::Release(window));
        }
    }

    fn gui_release_field(&mut self, host: &mut impl GuiHost, field: u32, shutdown: bool, destroy: bool) {
        if let Some(token) = self.owner.gui.fields.remove(&field) {
            if destroy {
                host.gui(GuiCall::Filter(token, false));
                host.gui(GuiCall::Destroy(token));
            }
            if shutdown {
                host.gui(GuiCall::Shutdown(token));
            }
            host.gui(GuiCall::Release(token));
        }
    }

    pub fn remove_gui(&mut self, host: &mut impl GuiHost) {
        if self.owner.popups.iter().any(Option::is_some) {
            self.close_owner_secondary(host);
        }
        self.gui_release_field(host, 0x18C, true, true);
        self.gui_release_field(host, 0x190, false, false);
        self.gui_release_field(host, 0x194, true, true);
        for token in std::mem::take(&mut self.owner.gui.windows) {
            host.gui(GuiCall::Filter(token, false));
            host.gui(GuiCall::Destroy(token));
            host.gui(GuiCall::Shutdown(token));
            host.gui(GuiCall::Release(token));
        }
        for panel in &mut self.owner.gui.panels {
            panel.clear();
        }
        for field in 0..5 {
            self.gui_release_field(host, 0x1E0 + field * 4, false, false);
        }
        for (field, stop) in [
            (0x2DC, true),
            (0x2E4, true),
            (0x2D8, false),
            (0x2EC, true),
            (0x2F0, false),
        ] {
            // VW6: 2EC/2F0 creation is unsettled. Null fields are harmless.
            self.gui_release_field(host, field, stop, false);
        }
        for field in [0x30C, 0x300, 0x304, 0x308] {
            self.gui_release_field(host, field, true, true);
        }
        for field in [0x2F8, 0x2FC, 0x2E8] {
            self.gui_release_field(host, field, true, false);
        }
        for field in [0x310, 0x314, 0x318] {
            self.gui_release_field(host, field, true, true);
        }
        if let Some(callout) = self.owner.gui.fields.remove(&0x3D8) {
            host.gui(GuiCall::RemoveChild(callout));
            host.gui(GuiCall::Release(callout));
        }
        self.gui_release_field(host, 0x3D0, false, false);
        self.gui_release_field(host, 0x3D4, false, false);
        host.gui(GuiCall::ClearScrollRenderSettings);
        self.owner.motion.reduced_render_settings = false;
        self.gui_release_field(host, 0x3E0, false, false);
        self.gui_release_field(host, 0x31C, true, false);
        if let Some(overlay) = self.owner.gui.overlay.take() {
            // VW14: honor the section's no-live-created-window requirement;
            // the enumerated RemoveGUI steps omit this parent-owned overlay.
            host.gui(GuiCall::Destroy(overlay));
            host.gui(GuiCall::Shutdown(overlay));
            // Parent owns the only reference. Never release a borrowed handle.
        }
    }

    pub fn shutdown(&mut self, host: &mut impl GuiHost) {
        self.shutdown_helpers(host);
        if let Some(filter) = self.owner.view.filter.take() {
            host.release_filter(filter.token);
        }
        if let Some(layer) = self.owner.view.contour_layer.take() {
            host.remove_contour_layer(layer);
        }
        self.owner.view.contours_shown = false;
        self.remove_gui(host);
        self.shutdown_tail(host);
    }
}
