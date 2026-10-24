// SPDX-License-Identifier: GPL-3.0-or-later
//! cGZWin and cGZFadeTmpWin, from widget-toolkit.md §§1–17.
//! Native buffers, callbacks and service calls are explicit host boundaries.
//! This is the base owner; `win` is the older city-window trace facade.

use std::collections::{BTreeMap, VecDeque};

mod dispatch;
mod geometry;
mod paint;
#[cfg(test)]
mod tests;
mod tree;
pub use tree::ChildRead;

pub type Window = u64;
pub type Object = u64;
pub const IID_WIN: u32 = 0x22ba_0121;
pub const FADE_ALPHA: u32 = 0xe6e9_98fb;
pub const FADE_TIME: u32 = 0xe6e9_98fc;
pub const FADE_PERIOD: u32 = 0xe6e9_98fd;
pub const FADE_TEMP: u32 = 0xe6e9_98fe;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect(pub i32, pub i32, pub i32, pub i32);

impl Rect {
    pub fn width(self) -> i32 {
        self.2.wrapping_sub(self.0)
    }
    pub fn height(self) -> i32 {
        self.3.wrapping_sub(self.1)
    }
    pub fn offset(self, x: i32, y: i32) -> Self {
        Self(
            self.0.wrapping_add(x),
            self.1.wrapping_add(y),
            self.2.wrapping_add(x),
            self.3.wrapping_add(y),
        )
    }
    pub fn contains(self, x: i32, y: i32) -> bool {
        self.0 <= x && x < self.2 && self.1 <= y && y < self.3
    }
    pub fn overlaps(self, r: Self) -> bool {
        self.0 < r.2 && r.0 < self.2 && self.1 < r.3 && r.1 < self.3
    }
    fn local(self) -> Self {
        Self(0, 0, self.width(), self.height())
    }
}

/// Five dwords; mouse shorts are decoded from the same bytes, never from pointers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Message(pub [u32; 5]);
impl Message {
    pub fn new(kind: u32, a: u32, b: u32, c: u32, d: u32) -> Self {
        Self([kind, a, b, c, d])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Param {
    Object(Object),
    Integer(i32),
}

/// State of the published +00..+D4 fields. Unknown WT2 bytes are retained as zero.
#[derive(Clone, Debug)]
pub struct BaseWindow {
    pub manager: Object,
    pub keyboard: Option<Object>,
    pub unnamed_0c: u32,
    pub id: u32,
    pub absolute: Rect,
    pub draw_area: Rect,
    pub draw_origin: Rect,
    pub parent: Option<Window>,
    pub notification: Option<Window>,
    pub caption: String,
    pub private_buffer: Option<Object>,
    pub draw_buffer: Option<Object>,
    pub draw_context: Option<Object>,
    pub dirty: bool,
    pub buffered: bool,
    pub cursor: Option<Object>,
    pub accelerator: Option<Object>,
    pub fade_periods: (i32, i32),
    pub key_equivalent: u32,
    pub unknown_a0_a4: [u32; 2],
    pub area: Rect,
    pub blit: Rect,
    pub flags: u32,
    pub instance_id: u32,
    pub shade: u32,
    pub fill: u32,
    pub references: u32,
    children: Vec<Window>,
    child_generation: u64,
    params: BTreeMap<u32, Param>,
    filters: Vec<Object>,
    removed_filters: Vec<Object>,
    filtering: bool,
}

impl BaseWindow {
    fn new(manager: Object, keyboard: Option<Object>, id: u32) -> Self {
        Self {
            manager,
            keyboard,
            unnamed_0c: 0,
            id: 0,
            absolute: Rect::default(),
            draw_area: Rect::default(),
            draw_origin: Rect::default(),
            parent: None,
            notification: None,
            caption: String::new(),
            private_buffer: None,
            draw_buffer: None,
            draw_context: None,
            dirty: true,
            buffered: false,
            cursor: None,
            accelerator: None,
            fade_periods: (500, 500),
            key_equivalent: 0,
            unknown_a0_a4: [0; 2],
            area: Rect::default(),
            blit: Rect::default(),
            flags: 0x8903,
            instance_id: id,
            shade: u32::MAX,
            fill: 0,
            references: 1,
            children: Vec::new(),
            child_generation: 0,
            params: BTreeMap::new(),
            filters: Vec::new(),
            removed_filters: Vec::new(),
            filtering: false,
        }
    }
    pub fn flag(&self, mask: u32) -> bool {
        self.flags & mask != 0
    }
    pub fn children(&self) -> &[Window] {
        &self.children
    }
    pub fn params(&self) -> &BTreeMap<u32, Param> {
        &self.params
    }
    pub fn filters(&self) -> &[Object] {
        &self.filters
    }
    pub fn fade_running(&self) -> bool {
        self.flags & 0x10020 == 0x10020 && self.integer(FADE_PERIOD) != 0
    }
    fn integer(&self, key: u32) -> i32 {
        match self.params.get(&key) {
            Some(Param::Integer(v)) => *v,
            _ => 0,
        }
    }
}

/// Calls whose external interface semantics are intentionally opaque in the spec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeOp {
    AddRef(Object),
    Release(Object),
    Register(Window),
    Unregister(Window),
    DropManagerReferences(Window),
    Focus(Option<Window>),
    CaptureReleased(Window),
    Paint(Window),
    Handler(Window, u32, Vec<u32>),
    Context(Object, u32, Vec<u32>),
    ContextRect(Object, u32, Rect),
    BufferLock(Object, u32),
    BufferUnlock(Object, u32),
    BufferReleaseSurface(Object),
    BufferColor(Object, u32),
    TransparentColor(Object, Option<u32>),
    Fill(Object, Rect, u32),
    Blit {
        destination: Object,
        source: Object,
        from: Rect,
        to: Rect,
        context: bool,
    },
    ReleasePresentInterface(Object),
    CursorLookup(Window),
    CursorActivate(Object),
    QueuePlot(Window),
    Post(Window, Message),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BufferInfo {
    pub area: Rect,
    pub usable: bool,
    pub content: bool,
    pub context: Option<Object>,
    pub transparent: Option<u32>,
    pub color: Option<u32>,
}

/// Host supplies services or subclass overrides, never native memory to core.
/// Buffer creation internals and native slot meanings stay at this boundary (§12).
pub trait Host {
    fn event(&mut self, _event: NativeOp) {}
    fn clock(&mut self) -> u32 {
        0
    }
    fn screen_buffer(&mut self) -> Option<Object> {
        None
    }
    fn draw_service_present(&mut self) -> bool {
        true
    }
    fn buffer_info(&mut self, _buffer: Object) -> BufferInfo {
        BufferInfo::default()
    }
    fn create_private_buffer(&mut self, _w: Window, _width: i32, _height: i32) -> Option<Object> {
        None
    }
    fn lock_buffer(&mut self, _buffer: Object, _flags: u32) -> bool {
        true
    }
    fn pixel_transparent(&mut self, _buffer: Object, _x: i32, _y: i32) -> bool {
        false
    }
    fn transparent_color(&mut self, _color_id: u32) -> Option<u32> {
        None
    }
    fn rgb_to_native(&mut self, _buffer: Option<Object>, _rgb: [u8; 3]) -> Option<u32> {
        None
    }
    fn native_to_rgb(&mut self, _buffer: Option<Object>, _native: u32) -> Option<[u8; 3]> {
        None
    }
    fn cursor(&mut self, _w: Window) -> Option<Object> {
        None
    }
    fn focus(&mut self, _w: Option<Window>) -> bool {
        true
    }
    fn query_interface(&mut self, _w: Window, _iid: u32) -> Option<Object> {
        None
    }
    fn filter(&mut self, _wins: &mut Windows, _filter: Object, _w: Window, _msg: Message) -> bool {
        false
    }
    fn handler(&mut self, _wins: &mut Windows, _w: Window, _slot: u32, _args: &[u32]) -> bool {
        false
    }
    fn accelerate(&mut self, _accelerator: Object, _msg: Message) -> bool {
        false
    }
    fn paint(&mut self, _wins: &mut Windows, _w: Window) {}
    fn init(&mut self, wins: &mut Windows, w: Window) -> bool
    where
        Self: Sized,
    {
        wins.init(self, w)
    }
    fn shutdown(&mut self, wins: &mut Windows, w: Window) -> bool
    where
        Self: Sized,
    {
        wins.shutdown(self, w)
    }
    fn set_area(&mut self, wins: &mut Windows, w: Window, area: Rect)
    where
        Self: Sized,
    {
        wins.set_area(self, w, area);
    }
    fn window_to_screen(&mut self, wins: &Windows, w: Window, point: (i32, i32)) -> (i32, i32) {
        let a = wins.get(w).absolute;
        (point.0.wrapping_add(a.0), point.1.wrapping_add(a.1))
    }
    fn make_key_equivalent(&mut self, key: u32, modifiers: u32) -> u32 {
        Windows::make_key_equivalent(key, modifiers)
    }
}

#[derive(Debug)]
pub struct Windows {
    windows: BTreeMap<Window, BaseWindow>,
    next_token: Window,
    next_instance: u32,
    pub focus: Option<Window>,
    pub capture: Option<Window>,
    pub mouse_window: Option<Window>,
    pub main_window: Option<Window>,
    pub queue_enabled: bool,
    plots: VecDeque<Window>,
    posts: VecDeque<(Window, Message)>,
    busy: u32,
    condemned: Vec<Window>,
    draw_service: Option<bool>,
    child_readers: BTreeMap<(Window, u64), usize>,
    retired_children: BTreeMap<(Window, u64), Vec<Window>>,
}

impl Windows {
    /// WT6: caller supplies the initial serial rather than guessing 0 or 1.
    pub fn new(first_instance: u32) -> Self {
        Self {
            windows: BTreeMap::new(),
            next_token: 1,
            next_instance: first_instance,
            focus: None,
            capture: None,
            mouse_window: None,
            main_window: None,
            queue_enabled: true,
            plots: VecDeque::new(),
            posts: VecDeque::new(),
            busy: 0,
            condemned: Vec::new(),
            draw_service: None,
            child_readers: BTreeMap::new(),
            retired_children: BTreeMap::new(),
        }
    }
    pub fn create(&mut self, host: &mut impl Host, manager: Object, keyboard: Option<Object>) -> Window {
        let w = self.next_token;
        self.next_token += 1;
        self.windows
            .insert(w, BaseWindow::new(manager, keyboard, self.next_instance));
        self.next_instance = self.next_instance.wrapping_add(1);
        host.event(NativeOp::Register(w));
        w
    }
    pub fn get(&self, w: Window) -> &BaseWindow {
        &self.windows[&w]
    }
    pub fn get_mut(&mut self, w: Window) -> &mut BaseWindow {
        self.windows.get_mut(&w).expect("live window")
    }
    pub fn valid(&self, w: Window) -> bool {
        self.windows.contains_key(&w)
    }
    pub fn add_ref(&mut self, w: Window) -> u32 {
        let s = self.get_mut(w);
        s.references = s.references.wrapping_add(1);
        s.references
    }
    pub fn release(&mut self, w: Window) -> u32 {
        let s = self.get_mut(w);
        s.references = s.references.wrapping_sub(1);
        s.references
    }
    pub fn set_param(&mut self, host: &mut impl Host, w: Window, key: u32, value: Option<Param>) -> bool {
        if let Some(Param::Object(old)) = self.get_mut(w).params.remove(&key) {
            host.event(NativeOp::Release(old));
        }
        if let Some(value) = value {
            if let Param::Object(obj) = &value {
                host.event(NativeOp::AddRef(*obj));
            }
            self.get_mut(w).params.insert(key, value);
        }
        true
    }
    pub fn get_param(&self, host: &mut impl Host, w: Window, key: u32, out: &mut Option<Param>) -> bool {
        if let Some(value) = self.get(w).params.get(&key) {
            if let Param::Object(obj) = value {
                host.event(NativeOp::AddRef(*obj));
            }
            *out = Some(value.clone());
            true
        } else {
            false
        }
    }
    pub fn enum_params(&mut self, w: Window, mut cb: impl FnMut(&mut Self, Window, u32, &Param) -> bool) -> bool {
        let values = self.get(w).params.clone();
        for (key, value) in values {
            if !cb(self, w, key, &value) {
                return false;
            }
        }
        true
    }
    pub fn set_accelerator(&mut self, host: &mut impl Host, w: Window, acc: Option<Object>) {
        if self.get(w).accelerator == acc {
            return;
        }
        if let Some(old) = self.get(w).accelerator {
            host.event(NativeOp::Release(old));
        }
        if let Some(new) = acc {
            host.event(NativeOp::AddRef(new));
        }
        self.get_mut(w).accelerator = acc;
    }
    pub fn init(&mut self, host: &mut impl Host, w: Window) -> bool {
        if !self.get(w).flag(0x4000) {
            self.get_mut(w).flags |= 0x4000;
            if self.get(w).flags & 0x10001 == 0x10001 {
                self.private_buffer(host, w, true);
            }
            self.set_buffer_recursive(host, w);
        }
        true
    }
    pub fn shutdown(&mut self, host: &mut impl Host, w: Window) -> bool {
        if self.get(w).flag(0x4000) {
            self.get_mut(w).flags &= !0x4000;
            self.set_accelerator(host, w, None);
            let filters = std::mem::take(&mut self.get_mut(w).filters);
            // Pending removals still own the filter reference until the outer dispatch ends.
            let pending = std::mem::take(&mut self.get_mut(w).removed_filters);
            for f in filters.into_iter().chain(pending) {
                host.event(NativeOp::Release(f));
            }
            self.hide(host, w);
            self.child_delete_all(host, w);
            self.private_buffer(host, w, false);
            let params = std::mem::take(&mut self.get_mut(w).params);
            for value in params.into_values() {
                if let Param::Object(obj) = value {
                    host.event(NativeOp::Release(obj));
                }
            }
        }
        true
    }
    pub fn set_focus_to(&mut self, host: &mut impl Host, w: Option<Window>) -> bool {
        if !w.is_some_and(|w| self.valid(w)) {
            return false;
        }
        self.set_focus(host, w);
        true
    }
    pub fn set_focus(&mut self, host: &mut impl Host, w: Option<Window>) -> bool {
        host.event(NativeOp::Focus(w));
        if host.focus(w) {
            self.focus = w;
            true
        } else {
            false
        }
    }
    pub fn set_flag(&mut self, host: &mut impl Host, w: Window, mask: u32, on: bool) -> u32 {
        if on {
            self.get_mut(w).flags |= mask;
        } else {
            self.get_mut(w).flags &= !mask;
        }
        match mask {
            1 => {
                if self.get(w).flag(0x10000) {
                    self.private_buffer(host, w, on);
                } else {
                    self.child_private_buffers(host, w, on);
                }
                if !on && self.focus == Some(w) {
                    self.set_focus_to(host, self.get(w).notification);
                }
                if let Some(p) = self.get(w).parent {
                    self.invalidate_self_and_parents(p);
                }
                self.get_mut(w).flags &= !0x400000;
            }
            2 if !on => {
                if self.capture == Some(w) {
                    self.capture = None;
                    host.event(NativeOp::CaptureReleased(w));
                }
                if self.focus == Some(w) {
                    self.set_focus_to(host, self.get(w).notification);
                }
            }
            0x20 => {
                let period = if on { self.get(w).fade_periods.0 } else { 0 };
                self.set_fade(host, w, if on { 0 } else { 255 }, period);
            }
            0x10000 => {
                self.private_buffer(host, w, on && self.get(w).flag(1));
            }
            0x20000 if self.get(w).flag(0x10000) => {
                self.set_transparency(host, w, on);
                self.invalidate_self_and_parents(w);
            }
            0x8000000 => {
                self.set_buffer_recursive(host, w);
                self.set_draw_area_recursive(host, w);
            }
            _ => {}
        }
        self.get(w).flags
    }
    pub fn show(&mut self, host: &mut impl Host, w: Window) {
        if self.get(w).flag(1) {
            return;
        }
        self.set_flag(host, w, 1, true);
        if self.get(w).flag(0x20) {
            self.set_fade(host, w, 0, self.get(w).fade_periods.0);
        }
    }
    pub fn hide(&mut self, host: &mut impl Host, w: Window) {
        if !self.get(w).flag(1) {
            return;
        }
        self.set_flag(host, w, 1, false);
        if self.get(w).flag(0x20) {
            self.set_fade(host, w, 255, 0);
        }
    }
}
