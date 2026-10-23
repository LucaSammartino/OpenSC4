// SPDX-License-Identifier: GPL-3.0-or-later
//! City-window control lifetime and input routing (view3d-window §§6c, 6e).

use super::{MotionHost, View3DWin};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    Character(u32),
    KeyDown(u32, u32),
    KeyUp(u32, u32),
    DownL(i32, i32, u32),
    DownR(i32, i32, u32),
    UpL(i32, i32, u32),
    UpR(i32, i32, u32),
    Move(i32, i32, u32),
    Wheel(i32, i32, u32, i32),
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlCall {
    AddRef,
    Release,
    SetWindow,
    Init,
    Activate,
    Deactivate,
    Shutdown,
    Cursor,
    SelfScrolling,
    Capturing,
    Event(InputEvent),
    SetMouseScroll(i32, i32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControlReply {
    pub taken: bool,
    pub cursor: Option<u32>,
}

/// Foreign controls, tutorial, renderer picking and window capture. Tokens are
/// opaque; every owned reference and temporary visit is explicit in `control`.
pub trait InputHost: MotionHost {
    fn control(&mut self, id: u32, call: ControlCall) -> ControlReply;
    fn control_changed(&mut self, old: Option<u32>, new: Option<u32>);
    fn set_cursor(&mut self, cursor: u32);
    fn has_capture(&self) -> bool;
    fn set_capture(&mut self, capture: bool);
    fn tutorial_mouse(&mut self, x: &mut i32, y: &mut i32, mode: i32) -> bool;
    fn cursor_position(&mut self, x: i32, y: i32);
    /// Returns an owned creation reference, if allocation succeeded.
    fn create_mouse_scroll(&mut self) -> Option<u32>;
    /// Occupant hit: query the box centre, perform the discarded terrain
    /// altitude/water queries, and release the picked reference (§6c).
    fn occupant_centre(&mut self, x: i32, y: i32) -> Option<[f32; 3]>;
    fn terrain_point(&mut self, x: i32, y: i32) -> Option<[f32; 3]>;
    fn camera_jump(&mut self, point: [f32; 3]);
}

#[derive(Clone, Copy, Debug)]
pub struct MouseContext {
    pub width: i32,
    pub height: i32,
    pub edge_preference: bool,
    pub tutorial_edge_allowed: bool,
    pub fullscreen: bool,
}

impl View3DWin {
    fn cursor_for(&self, host: &mut impl InputHost, control: Option<u32>) {
        let cursor = control
            .and_then(|id| host.control(id, ControlCall::Cursor).cursor)
            .unwrap_or(0xC16C_4E1B);
        host.set_cursor(cursor);
    }

    /// §6e. `control_changed` owns the message's additional new-control
    /// reference; the host retains that reference until message destruction.
    pub fn set_current_control(&mut self, host: &mut impl InputHost, control: Option<u32>, mode: u32) -> bool {
        if let Some(id) = control {
            host.control(id, ControlCall::SetWindow);
        }
        let old = self.input_control();
        if let Some(id) = old {
            host.control(id, ControlCall::AddRef);
        }
        host.control_changed(old, control);
        match mode {
            1 => {
                self.remove_current_control(host, false);
            }
            2 => self.remove_all_controls(host, false),
            _ => {}
        }
        if let Some(id) = self.input_control() {
            host.control(id, ControlCall::Event(InputEvent::Exit));
            host.control(id, ControlCall::Deactivate);
        }
        if let Some(id) = control {
            if host.control(id, ControlCall::Init).taken {
                host.control(id, ControlCall::AddRef);
                self.owner.stack.push(id);
                host.control(id, ControlCall::Activate);
                if let Some(cursor) = host.control(id, ControlCall::Cursor).cursor {
                    host.set_cursor(cursor);
                }
            }
        }
        if let Some(id) = old {
            host.control(id, ControlCall::Release);
        }
        true
    }

    pub fn remove_current_control(&mut self, host: &mut impl InputHost, silent: bool) -> bool {
        let Some(old) = self.owner.stack.pop() else {
            return false;
        };
        if host.has_capture() {
            host.set_capture(false);
        }
        host.control(old, ControlCall::Deactivate);
        host.control(old, ControlCall::Shutdown);
        host.control(old, ControlCall::Release);
        if !silent {
            let new = self.input_control();
            self.cursor_for(host, new);
            if new.is_some() {
                host.control_changed(Some(old), new);
            }
        }
        true
    }

    pub fn remove_all_controls(&mut self, host: &mut impl InputHost, silent: bool) {
        while self.remove_current_control(host, silent) {}
    }

    pub fn add_persistent_control(&mut self, host: &mut impl InputHost, id: u32) -> bool {
        if !host.control(id, ControlCall::Init).taken {
            return false;
        }
        host.control(id, ControlCall::SetWindow);
        host.control(id, ControlCall::AddRef);
        self.owner.persistent.push(id);
        host.control(id, ControlCall::Activate);
        true
    }

    pub fn remove_persistent_control(&mut self, host: &mut impl InputHost, id: u32) -> bool {
        let Some(index) = self.owner.persistent.iter().position(|&c| c == id) else {
            return false;
        };
        host.control(id, ControlCall::Deactivate);
        host.control(id, ControlCall::Shutdown);
        host.control(id, ControlCall::Release);
        self.owner.persistent.remove(index);
        true
    }

    pub fn remove_all_persistent_controls(&mut self, host: &mut impl InputHost) -> bool {
        while let Some(&id) = self.owner.persistent.last() {
            // Remove the last occurrence, including duplicate control tokens.
            host.control(id, ControlCall::Deactivate);
            host.control(id, ControlCall::Shutdown);
            host.control(id, ControlCall::Release);
            self.owner.persistent.pop();
        }
        true
    }

    fn visit(host: &mut impl InputHost, controls: &[u32], event: InputEvent, broadcast: bool) -> bool {
        let mut taken = false;
        for &id in controls.iter().rev() {
            host.control(id, ControlCall::AddRef);
            let result = host.control(id, ControlCall::Event(event)).taken;
            host.control(id, ControlCall::Release);
            taken |= result;
            if result && !broadcast {
                break;
            }
        }
        taken
    }

    fn buttons(&self, host: &mut impl InputHost, event: InputEvent) -> bool {
        Self::visit(host, &self.owner.persistent, event, false) || Self::visit(host, &self.owner.stack, event, false)
    }

    /// §6c: the persistent key pass always runs, even after a stack taker.
    pub fn process_key(&mut self, host: &mut impl InputHost, key: u32, mods: u32, down: bool) -> bool {
        self.owner.modifiers = mods;
        let event = if down {
            InputEvent::KeyDown(key, mods)
        } else {
            InputEvent::KeyUp(key, mods)
        };
        let stack = Self::visit(host, &self.owner.stack, event, false);
        Self::visit(host, &self.owner.persistent, event, true) || stack
    }

    pub fn on_character(&self, host: &mut impl InputHost, ch: u32) -> bool {
        let event = InputEvent::Character(ch);
        if !Self::visit(host, &self.owner.stack, event, false) && self.owner.persistent.len() <= self.owner.stack.len()
        {
            // Original bounds quirk: index is checked against stack size.
            Self::visit(host, &self.owner.persistent, event, false);
        }
        true
    }

    pub fn on_mouse(&mut self, host: &mut impl InputHost, event: InputEvent, context: MouseContext) -> bool {
        match event {
            InputEvent::DownL(mut x, mut y, mods) => {
                self.owner.modifiers = mods;
                if host.tutorial_mouse(&mut x, &mut y, -1) {
                    self.buttons(host, InputEvent::DownL(x, y, mods));
                }
            }
            InputEvent::DownR(x, y, mods) => {
                self.owner.modifiers = mods;
                host.set_capture(true);
                if !self.buttons(host, event) {
                    self.owner.right_press = (x, y);
                    self.owner.dragged = false;
                    if let Some(id) = host.create_mouse_scroll() {
                        self.set_current_control(host, Some(id), 0);
                        host.control(id, ControlCall::SetMouseScroll(x, y));
                        host.control(id, ControlCall::Release);
                    } else {
                        self.set_current_control(host, None, 0);
                    }
                }
            }
            InputEvent::UpL(mut x, mut y, mods) => {
                self.owner.modifiers = mods;
                if host.has_capture() {
                    if let Some(id) = self.input_control() {
                        if !host.control(id, ControlCall::SelfScrolling).taken
                            && host.control(id, ControlCall::Capturing).taken
                        {
                            self.recalc_scroll_velocity(host, context.edge_preference);
                        }
                    }
                }
                if host.tutorial_mouse(&mut x, &mut y, 1) {
                    self.buttons(host, InputEvent::UpL(x, y, mods));
                }
            }
            InputEvent::UpR(x, y, mods) => {
                self.owner.modifiers = mods;
                if host.has_capture() {
                    host.set_capture(false);
                }
                if !self.owner.dragged && mods & 7 == 0 {
                    let point = host.occupant_centre(x, y).or_else(|| host.terrain_point(x, y));
                    if let Some(point) = point {
                        host.camera_jump(point);
                    }
                }
                self.buttons(host, event);
            }
            InputEvent::Move(mut x, mut y, mods) => {
                self.owner.modifiers = mods;
                if !host.tutorial_mouse(&mut x, &mut y, 0) {
                    return true;
                }
                let (px, py) = self.owner.right_press;
                if (i64::from(px) - i64::from(x)).abs() > 3 || (i64::from(py) - i64::from(y)).abs() > 3 {
                    self.owner.dragged = true;
                }
                host.cursor_position(x, y);
                let event = InputEvent::Move(x, y, mods);
                Self::visit(host, &self.owner.persistent, event, true);
                Self::visit(host, &self.owner.stack, event, false);
                self.edge_scroll(host, x, y, context);
            }
            InputEvent::Wheel(_, _, mods, delta) => {
                self.owner.modifiers = mods;
                if !Self::visit(host, &self.owner.stack, event, false) {
                    Self::visit(host, &self.owner.persistent, event, true);
                    self.zoom_step(host, delta > 0);
                }
            }
            InputEvent::Exit => {
                self.owner.motion.edge = [0, 0];
                self.recalc_scroll_velocity(host, context.edge_preference);
                host.cursor_position(-10000, -10000);
                if let Some(id) = self.input_control() {
                    // No temporary reference on this one callback (§6c).
                    host.control(id, ControlCall::Event(event));
                }
                Self::visit(host, &self.owner.persistent, event, true);
            }
            _ => {}
        }
        true
    }

    fn edge_scroll(&mut self, host: &mut impl InputHost, x: i32, y: i32, c: MouseContext) {
        if !c.edge_preference || !c.tutorial_edge_allowed {
            self.owner.motion.edge = [0, 0];
            self.recalc_scroll_velocity(host, c.edge_preference);
            return;
        }
        let current = self.input_control();
        let self_scroll = current
            .map(|id| host.control(id, ControlCall::SelfScrolling).taken)
            .unwrap_or(false);
        let capturing = current
            .map(|id| host.control(id, ControlCall::Capturing).taken)
            .unwrap_or(false);
        if !self_scroll && (c.fullscreen || capturing) {
            let axis = |p, size| {
                if p <= 1 {
                    -1
                } else if p >= size - 1 {
                    1
                } else {
                    0
                }
            };
            self.owner.motion.edge = [axis(x, c.width), axis(y, c.height)];
            self.recalc_scroll_velocity(host, c.edge_preference);
        } else if !c.fullscreen && self.owner.motion.edge != [0, 0] {
            self.owner.motion.edge = [0, 0];
            self.recalc_scroll_velocity(host, c.edge_preference);
        }
    }
}
