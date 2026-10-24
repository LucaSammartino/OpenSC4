// SPDX-License-Identifier: GPL-3.0-or-later
//! Window/resource/timer owner paths, text-edit §§1,5,10 and adapters.
use super::*;
use crate::widget_controls::{Message, Rect};

pub(crate) fn normalized_flags(mut flags: u32) -> u32 {
    if flags & 0x2000 != 0 {
        flags |= 0x200;
    }
    if flags & 0x200 != 0 {
        flags &= !6;
    }
    if flags & 2 != 0 {
        flags &= !4;
    }
    flags
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Timer {
    Caret,
    Scroll,
}

/// Calls are individual foreign interface operations, never editor algorithms.
pub trait TextHost: TextForeign {
    fn retain(&mut self, object: Handle);
    fn relinquish(&mut self, object: Handle);
    fn base_query(&mut self, window: Handle, iid: u32, out: &mut Option<Handle>) -> bool;
    fn base_init(&mut self, window: Handle);
    fn base_shutdown(&mut self, window: Handle);
    fn base_area(&mut self, window: Handle, area: Rect);
    fn base_height(&mut self, window: Handle, height: u32);
    fn key_state(&mut self, key: u32) -> bool;
    fn base_message(&mut self, window: Handle, message: Message) -> bool;
    fn base_show(&mut self, window: Handle, show: bool);
    fn base_focus(&mut self, window: Handle, gained: bool) -> bool;
    fn base_flag(&mut self, window: Handle, flag: u32, value: bool);
    fn base_fill(&mut self, window: Handle, color: u32) -> bool;
    fn focused(&mut self, window: Handle) -> bool;
    fn enabled(&mut self, window: Handle) -> bool;
    fn capturing(&mut self, window: Handle) -> bool;
    fn capture(&mut self, window: Handle, take: bool);
    fn cursor(&mut self, window: Handle, cursor: Option<u32>);
    fn cursor_position(&mut self, window: Handle) -> (i32, i32);
    fn click_elapsed(&mut self) -> Option<u32>;
    fn click_timer(&mut self, restart: bool);
    fn font_lookup(&mut self, style: u32) -> Option<Handle>;
    fn clipboard_service(&mut self) -> Option<Handle>;
    fn code_page(&mut self) -> encoding::Encoding;
    fn timer(&mut self, timer: Timer, start: bool, interval: u32, argument: Handle);
    fn timer_destroy(&mut self, timer: Timer);
    fn callback_dispatch(&mut self, argument: Handle, kind: u32);
    fn convert_color(&mut self, value: u32, to_native: bool) -> u32;
    fn font_height(&mut self, font: Handle) -> u32;
    fn font_fit(&mut self, font: Handle, bytes: &[u8], width: u32) -> u32;
    fn font_measure(&mut self, font: Handle, bytes: &[u8]) -> u32;
    fn bar_create(&mut self, horizontal: bool, arguments: [u32; 5]) -> Option<Handle>;
    fn bar_remove_child(&mut self, window: Handle, bar: Handle);
    fn bar_add_child(&mut self, window: Handle, bar: Handle);
    fn bar_dimensions(&mut self, bar: Handle) -> (u32, u32);
    fn bar_place(&mut self, bar: Handle, rect: Rect);
    fn bar_range(&mut self, bar: Handle, maximum: i32);
    fn bar_pair(&mut self, bar: Handle, maximum: i32, step: u32);
    fn bar_value(&mut self, bar: Handle, value: Option<u32>) -> u32;
    fn image_rect(&mut self, image: Handle) -> Rect;
    fn bar_image(&mut self, bar: Handle, image: Handle, rect: Rect);
    fn clipboard_copy(&mut self, service: Handle, bytes: &[u8], format: u32, option: u32);
    fn clipboard_paste(&mut self, service: Handle, option: u32) -> Option<Vec<u8>>;
    fn composed_previous(&mut self, prefix: &[u8], selector: i32) -> u32;
    fn send(&mut self, window: Handle, kind: u32, words: [u64; 3]) -> bool;
    fn draw(&mut self, operation: Paint);
    fn search(&mut self, text: &[u8], pattern: &[u8], position: u32, forward: bool) -> u32;
    fn sound_acquire(&mut self, id: u32, arguments: [u32; 6]) -> (bool, Option<Handle>);
    fn sound_play(&mut self, sound: Handle) -> bool;
}

#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    Fill(Rect, u32),
    Image(Handle, Rect),
    EdgeImage { image: Handle, area: Rect, thirds: [i32; 2] },
    Text {
        font: Handle,
        bytes: Vec<u8>,
        x: i32,
        y: i32,
        clip: Rect,
        foreground: u32,
        background: u32,
    },
    LineWidth(f32),
    Caret {
        x: i32,
        top: i32,
        bottom: i32,
        color: u32,
    },
    Outline(Rect, u32),
}

#[derive(Debug)]
pub struct Runtime {
    pub font: Option<Handle>,
    pub font_style: Option<u32>,
    pub background: Option<Handle>,
    pub background_option: u8,
    /// Vertical then horizontal, as in owner creation/shutdown order.
    pub bars: [Option<Handle>; 2],
    pub images: [Option<Handle>; 2],
    pub image_rects: [Rect; 2],
    pub clipboard: Option<Handle>,
    pub caret_visible: bool,
    pub caret_running: bool,
    pub scroll_running: bool,
    pub size: [u32; 2],
    pub visible: [u32; 2],
    pub rows: u32,
    pub line_height: u32,
    pub horizontal_byte: u32,
    pub horizontal_pixel: u32,
    pub colors: [u32; 6],
    pub caret_color: u32,
    pub highlight_color: u32,
    pub outline_color: u32,
    pub fill_color: u32,
    pub sound_ids: [u32; 4],
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            font: None,
            font_style: None,
            background: None,
            background_option: 0,
            bars: [None; 2],
            images: [None; 2],
            image_rects: [[0; 4]; 2],
            clipboard: None,
            caret_visible: false,
            caret_running: false,
            scroll_running: false,
            size: [0; 2],
            visible: [0; 2],
            rows: 0,
            line_height: 0,
            horizontal_byte: 0,
            horizontal_pixel: 0,
            colors: [
                0xff00_0000,
                0x4000_0000,
                0xff00_0000,
                0x4000_0000,
                u32::MAX,
                0x4000_0000,
            ],
            // TE10: portable zero placeholders until callers resolve native
            // constructor colors/sound IDs; not a claim of native defaults.
            caret_color: 0,
            highlight_color: 0,
            outline_color: 0,
            fill_color: 0xff_ffff,
            sound_ids: [0; 4],
        }
    }
}

impl TextEdit {
    pub fn query_interface(&self, iid: u32, out: &mut Option<Handle>, f: &mut impl TextHost) -> bool {
        if iid == IID {
            *out = Some(self.text_view);
            f.retain(self.window);
            true
        } else {
            f.base_query(self.window, iid, out)
        }
    }
    pub fn toggle_caret(&mut self, f: &mut impl TextHost) {
        if f.focused(self.window) {
            self.runtime.caret_visible = !self.runtime.caret_visible;
            f.invalidate(self.window);
        }
    }
    pub fn show_caret(&mut self, f: &mut impl TextHost) {
        if !self.runtime.caret_visible && f.focused(self.window) {
            self.toggle_caret(f);
        }
    }
    pub fn caret_timer(&mut self, start: bool, f: &mut impl TextHost) -> bool {
        if self.runtime.caret_running == start {
            return false;
        }
        f.timer(Timer::Caret, start, self.caret_period >> 1, self.text_view);
        self.runtime.caret_running = start;
        true
    }
    pub fn scroll_timer(&mut self, start: bool, f: &mut impl TextHost) {
        if self.runtime.scroll_running != start {
            f.timer(Timer::Scroll, start, 100, 0);
            self.runtime.scroll_running = start;
        }
    }
    pub fn timer_callback(timer: Timer, argument: Handle, f: &mut impl TextHost) {
        f.callback_dispatch(
            argument,
            if timer == Timer::Caret {
                0x631b_56cc
            } else {
                0x631b_56cd
            },
        );
    }
    pub fn set_caret_period(&mut self, period: u32, f: &mut impl TextHost) {
        self.caret_period = period.clamp(250, 10000);
        if self.initialized && self.flags & 0x40 != 0 {
            // The native setter dispatches the two flag operations, not a
            // timer restart directly. These masks have no reflow path.
            self.clear_flag(0x40, f).expect("caret-only flag cannot reflow");
            self.set_flags(self.flags | 0x40, f)
                .expect("caret-only flag cannot reflow");
        }
    }
    pub fn set_background(&mut self, image: Option<Handle>, option: u8, f: &mut impl TextHost) {
        let changed = image != self.runtime.background;
        if changed {
            if let Some(image) = image {
                f.retain(image);
            }
            if let Some(old) = self.runtime.background {
                f.relinquish(old);
            }
            self.runtime.background = image;
        }
        if changed || self.runtime.background_option != option {
            f.invalidate(self.window);
        }
        self.runtime.background_option = option;
    }
    pub fn set_bar_image(&mut self, axis: usize, image: Option<Handle>, rect: Rect, f: &mut impl TextHost) {
        if let Some(image) = image {
            f.retain(image);
        }
        if let Some(old) = self.runtime.images[axis] {
            f.relinquish(old);
        }
        self.runtime.images[axis] = image;
        self.runtime.image_rects[axis] = rect;
        self.apply_bar_image(axis, f);
    }
    pub(crate) fn apply_bar_image(&self, axis: usize, f: &mut impl TextHost) {
        if let (Some(bar), Some(image)) = (self.runtime.bars[axis], self.runtime.images[axis]) {
            let mut rect = self.runtime.image_rects[axis];
            if rect[2] <= rect[0] || rect[3] <= rect[1] {
                rect = f.image_rect(image);
            }
            f.bar_image(bar, image, rect);
        }
    }
    pub fn set_font(&mut self, font: Option<Handle>, f: &mut impl TextHost) -> Result<()> {
        if font != self.runtime.font {
            if let Some(font) = font {
                f.retain(font);
            }
            if let Some(old) = self.runtime.font {
                f.relinquish(old);
            }
            self.runtime.font = font;
            if self.initialized {
                self.recalculate(f)?;
                f.invalidate(self.window);
            }
        }
        Ok(())
    }
    pub fn set_font_style(&mut self, style: u32, f: &mut impl TextHost) -> Result<bool> {
        self.runtime.font_style = Some(style);
        if self.initialized {
            if let Some(font) = f.font_lookup(style) {
                self.set_font(Some(font), f)?;
            }
        }
        Ok(true)
    }
    pub fn init(&mut self, f: &mut impl TextHost) -> Result<bool> {
        if self.initialized {
            return Ok(true);
        }
        f.base_init(self.window);
        self.initialized = true;
        let font = f
            .font_lookup(self.runtime.font_style.unwrap_or(0x6896_3c4c))
            .or_else(|| f.font_lookup(0x6896_3c4c));
        if let Some(font) = font {
            self.set_font(Some(font), f)?;
        }
        if self.flags & 1 != 0 {
            f.cursor(self.window, Some(3));
        }
        if f.enabled(self.window) && self.flags & 0x41 == 0x41 {
            self.caret_timer(true, f);
        }
        self.encoding = f.code_page();
        if self.runtime.clipboard.is_none() {
            self.runtime.clipboard = f.clipboard_service();
            if let Some(service) = self.runtime.clipboard {
                f.retain(service);
            }
        }
        self.recalculate(f)?;
        Ok(true)
    }
    pub fn shutdown(&mut self, f: &mut impl TextHost) {
        if let Some(font) = self.runtime.font.take() {
            f.relinquish(font);
        }
        self.caret_timer(false, f);
        self.scroll_timer(false, f);
        for slot in &mut self.runtime.bars {
            if let Some(bar) = slot.take() {
                f.relinquish(bar);
            }
        }
        for slot in &mut self.runtime.images {
            if let Some(image) = slot.take() {
                f.relinquish(image);
            }
        }
        if let Some(service) = self.runtime.clipboard.take() {
            f.relinquish(service);
        }
        // TE7: native teardown deliberately has no background-image release.
        f.base_shutdown(self.window);
        self.initialized = false;
    }
    pub fn destroy(&mut self, f: &mut impl TextHost) {
        self.shutdown(f);
        f.timer_destroy(Timer::Scroll);
        f.timer_destroy(Timer::Caret);
        self.redo.clear();
        self.undo.clear();
        self.pending.clear();
        self.lines.clear();
        self.text.clear();
    }
    pub fn set_flags(&mut self, requested: u32, f: &mut impl TextHost) -> Result<()> {
        let changed = self.flags ^ requested;
        self.flags = requested;
        if self.flags & 0x2000 != 0 {
            self.flags |= 0x200;
        }
        if self.flags & 0x200 != 0 {
            self.flags &= !6;
        }
        if self.flags & 2 != 0 {
            self.flags &= !4;
        }
        if self.initialized {
            if changed & 0x41 != 0 {
                let start = f.enabled(self.window) && self.flags & 0x41 == 0x41;
                self.caret_timer(start, f);
            }
            if changed & 0xb != 0 {
                self.recalculate(f)?;
                f.invalidate(self.window);
                f.cursor(self.window, if self.flags & 1 != 0 { Some(3) } else { None });
            }
        }
        Ok(())
    }
    pub fn clear_flag(&mut self, mask: u32, f: &mut impl TextHost) -> Result<()> {
        if self.flags & mask != 0 {
            self.set_flags(self.flags & !mask, f)?;
        }
        Ok(())
    }
    pub fn is_flag_set(&self, mask: u32) -> bool {
        self.flags & mask != 0
    }
    pub fn set_base_flag(&mut self, flag: u32, value: bool, f: &mut impl TextHost) {
        if flag == 1 {
            self.caret_timer(value && self.flags & 0x41 == 0x41, f);
        }
        f.base_flag(self.window, flag, value);
    }
    pub fn set_outline(&mut self, enabled: bool, color: u32, f: &mut impl TextHost) {
        self.flags = (self.flags & !0x10) | if enabled { 0x10 } else { 0 };
        self.runtime.outline_color = f.convert_color(color, true);
    }
    pub fn set_caret_color(&mut self, color: u32, f: &mut impl TextHost) {
        self.runtime.caret_color = f.convert_color(color, true);
    }

    pub fn get_caret_color(&self, f: &mut impl TextHost) -> u32 {
        f.convert_color(self.runtime.caret_color, false)
    }
    pub fn get_highlight_color(&self, f: &mut impl TextHost) -> u32 {
        f.convert_color(self.runtime.highlight_color, false)
    }
    pub fn get_outline(&self, f: &mut impl TextHost) -> (bool, u32) {
        (
            self.flags & 0x10 != 0,
            f.convert_color(self.runtime.outline_color, false),
        )
    }
    pub fn set_fill(&mut self, color: u32, f: &mut impl TextHost) -> bool {
        let result = f.base_fill(self.window, color);
        if self.flags & 0x20 != 0 {
            f.invalidate(self.window);
        }
        result
    }
    pub fn set_color(&mut self, index: usize, color: u32) -> Result<bool> {
        *self
            .runtime
            .colors
            .get_mut(index)
            .ok_or(Unsupported::Precondition("TE7 unchecked color index"))? = color;
        Ok(true)
    }
    pub fn get_color(&self, index: usize) -> Result<u32> {
        self.runtime
            .colors
            .get(index)
            .copied()
            .ok_or(Unsupported::Precondition("TE7 unchecked color index"))
    }
    pub fn clear_font_color(&mut self, index: usize) -> Result<()> {
        self.set_color(
            index,
            match index {
                0 | 2 => 0xff00_0000,
                4 => u32::MAX,
                _ => 0x4000_0000,
            },
        )?;
        Ok(())
    }
    pub fn set_font_color(&mut self, index: usize, color: u32, f: &mut impl TextHost) -> Result<bool> {
        let color = f.convert_color(color, true);
        self.set_color(index, color)
    }
    pub fn get_font_color(&self, index: usize, f: &mut impl TextHost) -> Result<u32> {
        Ok(f.convert_color(self.get_color(index)?, false))
    }
    pub fn set_sound_id(&mut self, index: i32, id: u32) -> Result<bool> {
        if index >= 4 {
            return Ok(false);
        }
        if index < 0 {
            return Err(Unsupported::Precondition("TE7 negative sound index overwrites object"));
        }
        self.runtime.sound_ids[index as usize] = id;
        Ok(true)
    }
    pub fn play_sound(&self, reason: usize, f: &mut impl TextHost) -> Result<bool> {
        let id = *self
            .runtime
            .sound_ids
            .get(reason)
            .ok_or(Unsupported::Precondition("TextEdit sound reason"))?;
        if id == 0 {
            return Ok(false);
        }
        let (success, sound) = f.sound_acquire(id, [5, 0x5b52_3ba4, id, 0, 0, 0]);
        let result = if success {
            sound.is_some_and(|h| f.sound_play(h))
        } else {
            false
        };
        if let Some(sound) = sound {
            f.relinquish(sound);
        }
        Ok(result)
    }
    pub fn find_text(&self, pattern: &[u8], position: u32, forward: bool, f: &mut impl TextHost) -> u32 {
        let found = f.search(&self.text, pattern, position, forward);
        if found < self.text.len() as u32 {
            found
        } else {
            u32::MAX
        }
    }
    pub fn set_highlight_color(&mut self, color: u32, f: &mut impl TextHost) {
        self.runtime.highlight_color = f.convert_color(color, false);
    }
    pub fn set_area(&mut self, area: Rect, f: &mut impl TextHost) -> Result<()> {
        f.base_area(self.window, area);
        self.runtime.size = [
            area[2].wrapping_sub(area[0]) as u32,
            area[3].wrapping_sub(area[1]) as u32,
        ];
        self.calculate_visible(f);
        self.recalculate(f)?;
        self.bar_sizes(f);
        self.bar_values(true, f)?;
        f.invalidate(self.window);
        Ok(())
    }
    pub fn set_gutter(&mut self, horizontal: bool, value: u32, f: &mut impl TextHost) -> Result<()> {
        self.gutters[usize::from(!horizontal)] = value;
        if self.initialized {
            if horizontal {
                self.calculate_visible(f);
                self.recalculate(f)?;
            }
            f.invalidate(self.window);
        }
        Ok(())
    }
    pub fn focus(&mut self, gained: bool, f: &mut impl TextHost) -> bool {
        f.invalidate(self.window);
        if self.flags & if gained { 0x20000 } else { 0x8000 } != 0 {
            f.send(
                self.window,
                if gained { 0x9b15_52da } else { 0x9b15_52d9 },
                [self.text_view, 0, 0],
            );
        }
        if gained {
            f.base_focus(self.window, true)
        } else {
            true
        }
    }
}
