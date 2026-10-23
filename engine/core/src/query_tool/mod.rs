// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable cSC4ViewInputControlQuery, from city-tools-input.md §8.
//! Foreign tokens are identities, never native pointers. See README.md for
//! ownership, primitive host contracts, and explicitly unsettled spec portions.

mod dialog;
mod model;
mod tips;

pub use crate::view_input_controls::{ControlHost, Pixel, Vec3};
pub use dialog::*;
pub use model::*;
pub use tips::{load_ratio, rating_level, Tip};

pub const MESSAGES: [u32; 6] = [
    0x0A45_6D95,
    0x0A45_6D96,
    0x26D3_1EC2,
    0xA6B7_9602,
    0x6695_6814,
    0x6924_7DC7,
];
pub const CURSOR_TEXT: u32 = 0x2A45_6CB1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Query,
    Route,
}
impl Mode {
    pub fn id(self) -> u32 {
        match self {
            Self::Query => 0xC7AF_928E,
            Self::Route => 0xC7AF_928F,
        }
    }
    pub fn cursor(self) -> u32 {
        match self {
            Self::Query => 0x816D_7F74,
            Self::Route => 0xCB8D_5B54,
        }
    }
    pub fn from_command(command: u32, current: u32) -> Option<Self> {
        let mode = match command {
            0x6A93_5CF3 => Self::Query,
            0x6A93_5CF4 => Self::Route,
            _ => return None,
        };
        (mode.id() != current).then_some(mode)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    Hover,
    Route,
    Automaton,
    Stoplight,
    Describe,
}
pub fn accepts(filter: Filter, o: &Occupant) -> bool {
    match filter {
        Filter::Describe => accepts(Filter::Hover, o) && !o.properties.contains_key(&0xCAA9_AB92),
        Filter::Hover => {
            matches!(
                o.kind,
                Kind::Building | Kind::PowerPole | Kind::Landfill | Kind::Network
            ) || (matches!(o.kind, Kind::Flora | Kind::Prop) && o.properties.contains_key(&0x8A41_6A99))
        }
        Filter::Route => {
            o.kind == Kind::Building
                || (o.network_interface && o.network_types.iter().any(|t| matches!(t, 0..=3 | 6..=12)))
        }
        Filter::Stoplight => o.kind == Kind::Prop && (o.groups.contains(&0x5002) || o.groups.contains(&0x5003)),
        Filter::Automaton => o.vehicle || o.pedestrian,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Open {
    LegendCt7,
    ModifierActionCt9,
    CalloutInterfaceCt13,
    DebugWritersCt14,
    NeighborDialogCt14,
    MySimCt14,
    RouteCt15,
    FillerConflictCt18,
    BuildingValuesCt19,
    TrafficInfoCt20,
}

#[derive(Debug)]
pub struct QueryTool {
    pub mode: Mode,
    pub tip_delay: u32,
    pub initialized: bool,
    pub highlighted: Option<Token>,
    pub saved_highlight: u32,
    pub tip_allowed: bool,
    pub point: Vec3,
    pub queriable: bool,
    pub clicking: bool,
    pub terrain_query: bool,
    pub unused: u32,
    pub callout: Option<Token>,
    pub legend: [Option<Token>; 2],
    pub path_view: Option<Token>,
    pub path_rect: Option<Rect>,
    timer_running: bool,
    base_cursor: Option<u64>,
    base_view: bool,
}

impl QueryTool {
    /// The spec's global default vector is supplied by the embedding host; no
    /// numeric value for it is published. Both command makers pass delay 50.
    pub fn new(mode: Mode, tip_delay: u32, default_point: Vec3) -> Self {
        Self {
            mode,
            tip_delay,
            initialized: false,
            highlighted: None,
            saved_highlight: 0,
            tip_allowed: true,
            point: default_point,
            queriable: false,
            clicking: false,
            terrain_query: false,
            unused: 3,
            callout: None,
            legend: [None; 2],
            path_view: None,
            path_rect: None,
            timer_running: false,
            base_cursor: None,
            base_view: false,
        }
    }

    pub fn init(&mut self, host: &mut impl Host) -> bool {
        if self.initialized {
            return true;
        }
        // Reuse the shared control cursor primitive; no second input base.
        self.base_cursor = host.lookup_cursor(self.mode.cursor());
        if let Some(c) = self.base_cursor {
            host.effect(Effect::AcquireCursor(c));
            host.apply_window_cursor(c, true);
        }
        self.base_view = host.has_view();
        if self.base_view {
            host.effect(Effect::AcquireView);
        }
        host.effect(Effect::AcquireWindowManager);
        self.initialized = true;
        for m in MESSAGES {
            host.effect(Effect::Subscribe(m));
        }
        host.effect(Effect::BindTipCallback);
        self.tip_allowed = true;
        self.terrain_query = host.terrain_query_enabled();
        self.callout = host.create_callout();
        if let Some(w) = self.callout {
            host.effect(Effect::CalloutSetup(w, [0xA85F_1A83, 0x0A52_F5C6], [-40, -40, 40, 40]));
            host.effect(Effect::Hide(w));
            host.effect(Effect::AddChild(w));
        }
        if self.mode == Mode::Route {
            host.effect(Effect::Unsettled(Open::LegendCt7));
        }
        true
    }

    fn stop_timer(&mut self, host: &mut impl Host) {
        host.effect(Effect::StopTipTimer);
        self.timer_running = false;
    }
    fn hide_tip(&self, host: &mut impl Host) {
        host.effect(Effect::ClearCursorText(CURSOR_TEXT));
        if let Some(w) = self.callout {
            host.effect(Effect::Hide(w));
        }
    }
    pub fn set_highlight_state(&self, host: &mut impl Host, state: u32) {
        if let Some(id) = self.highlighted {
            if let Some(lot) = host.lot_for_occupant(id) {
                for id in host.lot(lot).occupants {
                    if host.occupant(id).bool_property(0x6A95_E503) {
                        host.effect(Effect::Highlight(id, state));
                    }
                }
                host.effect(Effect::ReleaseOccupantList);
            }
        }
    }
    fn unhighlight(&mut self, host: &mut impl Host) {
        if let Some(id) = self.highlighted {
            host.effect(Effect::Highlight(id, self.saved_highlight));
            self.set_highlight_state(host, 0);
            host.effect(Effect::Release(id));
        }
        self.highlighted = None;
        self.saved_highlight = 0;
    }
    pub fn shutdown(&mut self, host: &mut impl Host) -> bool {
        if !self.initialized {
            return true;
        }
        host.effect(Effect::ClearCursorText(CURSOR_TEXT));
        if let Some(w) = self.callout.take() {
            host.effect(Effect::RemoveChild(w));
            host.effect(Effect::Release(w));
        }
        for w in &mut self.legend {
            if let Some(w) = w.take() {
                host.effect(Effect::DetachProc(w));
                host.effect(Effect::RemoveChild(w));
                host.effect(Effect::Release(w));
            }
        }
        self.remove_paths(host);
        self.unhighlight(host);
        for m in MESSAGES {
            host.effect(Effect::Unsubscribe(m));
        }
        if self.timer_running {
            self.stop_timer(host);
            self.tip_allowed = false;
        }
        host.effect(Effect::ReleaseWindowManager);
        if self.base_view {
            host.effect(Effect::ReleaseView);
            self.base_view = false;
        }
        if let Some(c) = self.base_cursor.take() {
            host.effect(Effect::ReleaseCursor(c));
        }
        self.initialized = false;
        true
    }
    /// Explicit destruction because host effects cannot run from Rust Drop.
    pub fn destroy(&mut self, host: &mut impl Host) {
        if let Some(p) = self.path_view.take() {
            host.effect(Effect::Release(p));
        }
        host.effect(Effect::DestroyTipTimer);
        for w in self.legend.iter_mut().rev() {
            if let Some(w) = w.take() {
                host.effect(Effect::Release(w));
            }
        }
        if let Some(w) = self.callout.take() {
            host.effect(Effect::Release(w));
        }
    }
    fn filter(&self) -> Filter {
        if self.mode == Mode::Route {
            Filter::Route
        } else {
            Filter::Hover
        }
    }
    pub fn pick(host: &mut impl Host, pixel: Pixel, filter: Filter) -> Option<Token> {
        // Renderer owns traversal; predicate is portable and passed to the host.
        host.pick(pixel, filter, accepts)
    }
    pub fn mouse_move(&mut self, host: &mut impl Host, pixel: Pixel) -> bool {
        self.queriable = false;
        if !host.has_view() {
            return true;
        }
        let mut hit = Self::pick(host, pixel, self.filter());
        let mut stale = false;
        if let Some(id) = hit {
            self.queriable = true;
            if host.occupant(id).bool_property(0x6A95_E503) {
                if let Some(l) = host.lot_for_occupant(id) {
                    if let Some(b) = host.lot(l).building {
                        hit = Some(b);
                    }
                }
            }
            if self.terrain_query {
                if let Some(p) = host.pick_terrain(pixel) {
                    self.point = p;
                }
            }
        } else if let Some(p) = host.pick_terrain(pixel) {
            if let Some(l) = host.lot_at(host.position_to_cell(p)) {
                if let Some(b) = host.lot(l).building {
                    let o = host.occupant(b);
                    if (self.mode == Mode::Route && accepts(Filter::Route, &o))
                        || (self.mode == Mode::Query && o.groups.contains(&0x1006))
                    {
                        hit = Some(b);
                        if self.mode == Mode::Route {
                            self.queriable = true;
                        }
                    }
                }
            }
            if self.mode == Mode::Query {
                self.queriable = true;
            }
            if (p.x - self.point.x).abs() >= 0.0001
                || (p.y - self.point.y).abs() >= 0.0001
                || (p.z - self.point.z).abs() >= 0.0001
            {
                self.point = p;
                stale = hit.is_none();
            }
        }
        let changed = hit != self.highlighted;
        if changed {
            self.unhighlight(host);
            self.highlighted = hit;
            if let Some(id) = hit {
                self.saved_highlight = host.occupant(id).highlight;
                host.effect(Effect::Highlight(id, 7));
                self.set_highlight_state(host, 7);
                host.effect(Effect::Acquire(id));
            }
        }
        if changed || stale || hit.is_none() {
            self.hide_tip(host);
        }
        if !self.clicking {
            self.stop_timer(host);
            self.tip_allowed = true;
            host.effect(Effect::StartTipTimer(self.tip_delay));
            self.timer_running = true;
        }
        if let Some(w) = self.callout {
            if host.visible(w) {
                host.effect(Effect::MoveCallout(w, pixel));
            }
        }
        true
    }
    pub fn mouse_exit(&mut self, host: &mut impl Host) -> bool {
        self.stop_timer(host);
        self.hide_tip(host);
        self.unhighlight(host);
        self.queriable = false;
        true
    }
    pub fn mouse_down(&mut self, host: &mut impl Host, pixel: Pixel, modifiers: u32) -> bool {
        self.stop_timer(host);
        self.hide_tip(host);
        let mut automaton = false;
        if modifiers & 1 != 0 {
            if let Some(id) = Self::pick(host, pixel, Filter::Automaton) {
                automaton = true;
                host.effect(Effect::ModifierHit(id, Open::ModifierActionCt9));
            } else {
                host.effect(Effect::PostAt(0x2ACD_2CF3, pixel));
            }
            if let Some(id) = Self::pick(host, pixel, Filter::Stoplight) {
                host.effect(Effect::ModifierHit(id, Open::ModifierActionCt9));
            }
        }
        if modifiers & 2 != 0 {
            let p = host.pick_terrain(pixel).unwrap_or_else(|| host.default_point());
            let c = host.position_to_cell(p);
            host.effect(Effect::Residential59(Cell { x: c.x / 4, z: c.z / 4 }));
        }
        self.clicking = true;
        if self.mode == Mode::Route {
            self.get_occupant_paths(host);
            return true;
        }
        self.clicking = false;
        if !automaton {
            self.describe_pick(host, pixel, host.debug_query_enabled());
        }
        true
    }
    pub fn mouse_up(&mut self, host: &mut impl Host) -> bool {
        self.stop_timer(host);
        self.hide_tip(host);
        self.clicking = false;
        true
    }
    pub fn unhandled_input(&self) -> bool {
        false
    }
    pub fn should_stack(&self) -> bool {
        true
    }
    pub fn is_self_scrolling(&self) -> bool {
        false
    }
    pub fn message(&mut self, host: &mut impl Host, message: u32) -> bool {
        match message {
            0x0A45_6D95 if self.tip_allowed => {
                let tip = self.build_tip(host);
                self.display_tip(host, &tip);
            }
            0x0A45_6D96 => host.effect(Effect::RecalcCoverage),
            0x26D3_1EC2 => self.tip_allowed = false,
            0xA6B7_9602 => {
                let p = host.cursor_position(false);
                self.mouse_move(host, p);
            }
            0x6695_6814 if host.day() % 10 == 0 => {
                if let Some((w, s)) = host.active_dialog() {
                    walk(host, w, s, true);
                }
            }
            0x6924_7DC7 if self.path_view.is_some() => {
                host.effect(Effect::ClearPaths(self.path_view.unwrap()));
                host.effect(Effect::Unsettled(Open::RouteCt15));
            }
            _ => {}
        }
        false
    }
    pub fn tip_callback(host: &mut impl Host) {
        host.effect(Effect::SendTipMessage(MESSAGES[0]));
    }
    pub fn coverage_callback(host: &mut impl Host) {
        host.effect(Effect::PostMessage(MESSAGES[1]));
    }
    fn remove_paths(&mut self, host: &mut impl Host) {
        if let Some(p) = self.path_view.take() {
            host.effect(Effect::UnregisterPath(p));
            host.effect(Effect::ShutdownPath(p));
            host.effect(Effect::Release(p));
        }
    }
    pub fn get_occupant_paths(&mut self, host: &mut impl Host) {
        self.remove_paths(host);
        self.path_rect = self.highlighted.and_then(|id| {
            let o = host.occupant(id);
            if o.kind == Kind::Building {
                host.lot_for_occupant(id).map(|l| host.lot(l).rect)
            } else if o.network_interface {
                Some(Rect {
                    min: o.cell,
                    max: o.cell,
                })
            } else {
                None
            }
        });
        if self.path_rect.is_some() {
            host.effect(Effect::Unsettled(Open::RouteCt15));
        }
    }
}

#[cfg(test)]
mod tests;
