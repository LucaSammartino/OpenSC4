// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildingKind {
    ChangeHouse,
    ChangeJob,
    MoveIn,
}

/// Move-in holds the interface; the other tools hold the object. No native
/// pointer arithmetic is performed on a portable identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimView {
    Object(Handle),
    Interface(Handle),
}
impl SimView {
    pub fn handle(self) -> Handle {
        match self {
            Self::Object(h) | Self::Interface(h) => h,
        }
    }
}

pub trait BuildingServices: InputServices {
    fn sim_interface(&mut self, object: Handle) -> Option<Handle>;
    fn agent(&mut self) -> Option<Handle>;
    /// Created foreign reference. Register the corresponding portable windows
    /// in `windows`, including its parent and the not-allowed sibling.
    fn create_badge(&mut self, windows: &mut Windows, parent: Window, key: Key, child: u32, iid: u32)
        -> Option<Window>;
    fn bitmap_id(&mut self, sim: SimView) -> u32;
    fn bitmap_file(&mut self, sim: SimView, index: i32) -> String;
    fn application_directory(&mut self) -> String;
    /// Image methods return created references; image setters only borrow.
    fn resource_image(&mut self, key: Key) -> Option<Handle>;
    fn file_image(&mut self, path: &str, mode: u32) -> Option<Handle>;
    fn bitmap_image(&mut self, child: Window, iid: u32, image: Handle);
    fn wealth(&mut self, sim: SimView) -> u32;
    fn highlight(&mut self, building: Handle) -> u32;
    fn set_highlight(&mut self, building: Handle, level: u32, persistent: bool);
}

#[derive(Debug)]
pub struct BuildingTool {
    pub kind: BuildingKind,
    pub(super) initialized: bool,
    pub(super) sim: Option<SimView>,
    pub(super) highlighted: Option<(Handle, u32)>,
    pub(super) agent: Option<Handle>,
    pub(super) callout: Option<Window>,
    pub(super) allowed: Option<Window>,
    pub(super) denied: Option<Window>,
    pub(super) badge_height: i32,
    pub(super) badge_half_width: i32,
}

impl BuildingTool {
    pub fn new(kind: BuildingKind) -> Self {
        Self {
            kind,
            initialized: false,
            sim: None,
            highlighted: None,
            agent: None,
            callout: None,
            allowed: None,
            denied: None,
            badge_height: 0,
            badge_half_width: 0,
        }
    }
    pub fn sim(&self) -> Option<SimView> {
        self.sim
    }
    pub fn highlighted(&self) -> Option<Handle> {
        self.highlighted.map(|(b, _)| b)
    }
    pub fn badges(&self) -> (Option<Window>, Option<Window>) {
        (self.allowed, self.denied)
    }
    pub fn init(
        &mut self,
        sim: Option<Handle>,
        windows: &mut Windows,
        host: &mut impl BuildingServices,
    ) -> Result<bool> {
        if self.initialized {
            // SPEC-GAP: repeated initialization is outside the published lifecycle.
            return Err(Unsupported::Precondition("new building-tool lifetime"));
        }
        host.set_cursor(0x4340_1c71);
        if !host.base_init() {
            return Ok(false);
        }
        self.initialized = true;
        self.sim = match (self.kind, sim) {
            (BuildingKind::MoveIn, Some(sim)) => host.sim_interface(sim).map(SimView::Interface),
            (_, Some(sim)) => Some(SimView::Object(sim)),
            (_, None) => None,
        };
        if let Some(sim) = self.sim {
            host.add_ref(sim.handle());
        }
        self.agent = host.agent();
        if let Some(agent) = self.agent {
            host.add_ref(agent);
        }
        host.focus_view();
        if self.allowed.is_none() {
            if let Some(parent) = host.view_window() {
                self.allowed =
                    host.create_badge(windows, parent, [0, 0x96a0_06b0, 0x6a94_55c9], 0x27df_05be, 0x5386_d516);
            }
            if let Some(allowed) = self.allowed {
                let rect = windows.get(allowed).area;
                self.badge_height = rect.height();
                self.badge_half_width = rect.width() / 2;
                windows.set_flag(host, allowed, 0x20_0000, true);
                if let Some(parent) = windows.get(allowed).parent {
                    self.denied = windows.child_from_id_recursive(parent, 0x27df_05bf);
                }
                if let Some(denied) = self.denied {
                    host.add_ref(denied);
                    windows.set_flag(host, denied, 0x20_0000, true);
                }
                if let Some(sim) = self.sim {
                    let id = host.bitmap_id(sim);
                    let image = if id != 0 {
                        host.resource_image([0, 0x46a0_06b0, id])
                    } else {
                        let mut path = host.application_directory();
                        path.push_str(&host.bitmap_file(sim, -1));
                        host.file_image(&path, 8)
                    };
                    if let Some(image) = image {
                        for (parent, id) in [(Some(allowed), 0xea94_57ba), (self.denied, 0xea94_57bb)] {
                            if let Some(parent) = parent {
                                if let Some(child) = windows.child_from_id_recursive(parent, id) {
                                    host.bitmap_image(child, 0xc12c_ea13, image);
                                }
                            }
                        }
                        host.release(image);
                    }
                }
            }
        }
        if self.kind == BuildingKind::ChangeHouse {
            if let (Some(sim), Some(badge)) = (self.sim, self.allowed) {
                let wealth = host.wealth(sim);
                for (id, hide) in [(0xcbf3_3d81, wealth == 1), (0xcbf3_3d8c, wealth == 1 || wealth == 2)] {
                    if hide {
                        if let Some(child) = windows.child_from_id_recursive(badge, id) {
                            windows.hide(host, child);
                        }
                    }
                }
            }
        }
        Ok(true)
    }
    pub fn on_key_down(&mut self, key: u32, host: &mut impl InputServices) -> bool {
        if key == 0x1b {
            host.end_input();
            true
        } else {
            false
        }
    }
    pub fn on_mouse_exit(&self, windows: &mut Windows, host: &mut impl InputServices) -> bool {
        for badge in [self.allowed, self.denied].into_iter().flatten() {
            windows.hide(host, badge);
        }
        // §1.2 quirk: leave both highlight and callout alone.
        true
    }
    pub(super) fn restore_highlight(&mut self, host: &mut impl BuildingServices) {
        if let Some((building, level)) = self.highlighted.take() {
            host.set_highlight(building, level, false);
            host.release(building);
        }
    }
    pub fn shutdown(&mut self, windows: &mut Windows, host: &mut impl BuildingServices) -> bool {
        if let Some(sim) = self.sim.take() {
            host.release(sim.handle());
        }
        self.restore_highlight(host);
        if let Some(agent) = self.agent.take() {
            host.release(agent);
        }
        if let Some(callout) = self.callout {
            windows.hide(host, callout);
        }
        for window in [&mut self.callout, &mut self.allowed, &mut self.denied] {
            if let Some(window) = window.take() {
                if let Some(parent) = host.view_window() {
                    windows.child_delete(host, parent, window);
                }
                host.release(window);
            }
        }
        host.base_shutdown();
        self.initialized = false;
        true
    }
    /// Destructor body, in the reverse field order of §1.2. Shutdown normally
    /// drains the fields first. No Release is sent for a borrowed-only handle.
    pub fn destroy(&mut self, host: &mut impl InputServices) {
        for window in [&mut self.denied, &mut self.allowed, &mut self.callout] {
            if let Some(window) = window.take() {
                host.release(window);
            }
        }
        if let Some(agent) = self.agent.take() {
            host.release(agent);
        }
        if let Some((building, _)) = self.highlighted.take() {
            host.release(building);
        }
        if let Some(sim) = self.sim.take() {
            host.release(sim.handle());
        }
        host.base_destroy();
        self.initialized = false;
    }
}
