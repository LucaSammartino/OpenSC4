// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable control owners from `docs/specs/widget-controls.md`.
//!
//! The existing `legacy_control` interprets script properties; these owners
//! implement control state. OS windows, fonts, timers, drawing and IO are foreign.
//! Handles reuse the window layer's opaque identities, never native pointers.

pub mod list_box;
pub mod tree_view;

pub type Handle = crate::ui_windows::win::WinToken;
pub type Rect = [i32; 4];
pub const SENTINEL: u32 = 0xdead_beef;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    Open(&'static str),
    Precondition(&'static str),
}
pub type Result<T> = std::result::Result<T, Unsupported>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    Send,
    Post,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message {
    pub kind: u32,
    pub words: [i32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resource {
    Font,
    Buffer,
    Timer,
}

/// Individual foreign drawing primitives. The owner chooses order, clipping,
/// rows and selection; the adapter performs only the named operation.
#[derive(Clone, Debug, PartialEq)]
pub enum Draw {
    Fill {
        rect: Rect,
        color: u32,
    },
    Blit {
        source: Handle,
        rect: Rect,
    },
    Edge {
        source: Handle,
        rect: Rect,
    },
    Text {
        font: Handle,
        text: Vec<u8>,
        rect: Rect,
        color: u32,
    },
    Outline {
        rect: Rect,
        colors: [u32; 2],
        three_d: bool,
    },
}

/// The base window, COM and individual service operations. No owner algorithm
/// (selection, sorting, filtering or layout) is delegated to this interface.
pub trait Foreign {
    fn add_ref(&mut self, object: Handle);
    fn release(&mut self, object: Handle);
    fn base_init(&mut self, window: Handle) -> bool;
    fn base_shutdown(&mut self, window: Handle);
    fn base_message(&mut self, window: Handle, message: Message) -> bool;
    fn base_query(&mut self, window: Handle, iid: u32, output: &mut Option<Handle>) -> bool;
    fn invalidate(&mut self, window: Handle);
    fn show(&mut self, window: Handle, visible: bool);
    fn capture(&mut self, window: Handle, take: bool);
    fn convert_payload(&mut self, window: Handle, value: i32) -> i32;
    fn deliver(&mut self, window: Handle, delivery: Delivery, message: Message) -> bool;
    fn drop_complete(&mut self, window: Handle, kind: u32, row: i32);
    /// WCM lookups return borrowed interfaces. Owners explicitly retain them.
    fn default_interface(&mut self, resource: u32) -> Option<Handle>;
    fn default_color(&mut self, resource: u32) -> Option<u32>;
    /// Factory returns a created reference, released exactly once by the owner.
    fn create(&mut self, resource: Resource) -> Option<Handle>;
    fn font_height(&mut self, font: Handle) -> i32;
    fn buffer_height(&mut self, buffer: Handle) -> i32;
    fn bar_value(&mut self, bar: Handle) -> i32;
    fn bar_range(&mut self, bar: Handle, maximum: i32, page: i32, value: i32);
    fn place_bar(&mut self, bar: Handle, rect: Rect);
    fn create_bar(&mut self, window: Handle) -> Option<Handle>;
    fn destroy_bar(&mut self, bar: Handle);
    fn timer_elapsed(&mut self, timer: Handle) -> u32;
    fn restart_timer(&mut self, timer: Handle);
    fn double_click_interval(&mut self) -> u32;
    /// WC1 keeps byte/case/encoding policy in the actual comparison service.
    fn begins_with(&mut self, text: &[u8], prefix: &[u8], argument: u32) -> bool;
    fn text_equal(&mut self, left: &[u8], right: &[u8]) -> bool;
    fn draw(&mut self, operation: Draw);
}

pub(crate) fn replace_ref(slot: &mut Option<Handle>, value: Option<Handle>, f: &mut impl Foreign) {
    if let Some(h) = value {
        f.add_ref(h);
    }
    if let Some(h) = slot.take() {
        f.release(h);
    }
    *slot = value;
}

#[cfg(test)]
mod tests;
