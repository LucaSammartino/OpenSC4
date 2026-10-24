// SPDX-License-Identifier: GPL-3.0-or-later
//! Key, mouse, dispatch and composed-step owner paths.
use super::{layout::Vertical, runtime::TextHost, *};
use crate::widget_controls::Message;

impl TextEdit {
    pub fn step(&mut self, delta: i32, composed: bool, f: &mut impl TextHost) -> Result<()> {
        let special = if composed {
            self.encoding.accumulates()
        } else {
            matches!(self.encoding.selector, 8 | 100)
        };
        let mut at = self.position.index;
        if !special {
            at = at.wrapping_add(delta as u32);
        } else {
            for _ in 0..delta.unsigned_abs() {
                let index = at as usize;
                let step = if delta > 0 {
                    let tail = self
                        .text
                        .get(index..)
                        .ok_or(Unsupported::Precondition("TextEdit step readable tail"))?;
                    if composed {
                        f.composed_length(tail, self.encoding.selector) as u32
                    } else {
                        self.encoding.char_length(tail, tail.len())? as u32
                    }
                } else {
                    let prefix = self
                        .text
                        .get(..index)
                        .ok_or(Unsupported::Precondition("TextEdit step readable prefix"))?;
                    if composed {
                        f.composed_previous(prefix, self.encoding.selector)
                    } else {
                        self.encoding.previous_length(prefix) as u32
                    }
                };
                at = if delta > 0 {
                    at.wrapping_add(step)
                } else {
                    at.wrapping_sub(step)
                };
            }
        }
        self.position.index = if (at as i32) < 0 {
            0
        } else {
            at.min(self.text.len() as u32)
        };
        let (line, column) = self.line_column(self.position.index)?;
        self.position.line = line;
        self.position.column = column;
        Ok(())
    }
    fn finish_navigation(&mut self, shift: bool, reset_x: bool, f: &mut impl TextHost) -> Result<()> {
        self.position.active = self.position.index;
        if !shift {
            self.position.anchor = self.position.index;
        }
        self.insertion_in_view(f)?;
        self.show_caret(f);
        f.invalidate(self.window);
        if reset_x {
            self.preferred_x = u32::MAX;
        }
        Ok(())
    }
    pub fn delete_key(&mut self, backspace: bool, f: &mut impl TextHost) -> Result<()> {
        if self.flags & 1 == 0 {
            return Ok(());
        }
        let before = self.position;
        let selected = before.anchor != before.active;
        let mut removed = Vec::new();
        if selected {
            self.position.index = before.anchor.min(before.active);
            removed = self.remove_raw(self.position.index, before.anchor.abs_diff(before.active), f)?;
        } else if backspace && before.index > 0 {
            let prefix = self.span(0, before.index)?;
            let length = self.encoding.previous_length(prefix) as u32;
            self.position.index = before.index.wrapping_sub(length);
            removed = self.remove_char_raw(self.position.index, true, f)?;
        } else if !backspace && before.index < self.text.len() as u32 {
            removed = self.remove_char_raw(before.index, false, f)?;
        }
        if removed.is_empty() {
            f.correction(1);
        } else {
            self.record_deletion(self.position.index, removed, before);
        }
        self.fix_insertion()?;
        self.collapse();
        self.execute_repairs(f)?;
        if self.flags & 0x800 != 0 {
            f.notify(self.text_view, TEXT_CHANGED);
        }
        Ok(())
    }
    fn copy(&self, shift: bool, control: bool, alt: bool, f: &mut impl TextHost) -> Result<()> {
        if control
            && !shift
            && !alt
            && self.flags & 0x1000 != 0
            && self.flags & 0x2000 == 0
            && !f.capturing(self.window)
        {
            if let Some(service) = self.runtime.clipboard {
                let low = self.position.anchor.min(self.position.active);
                let count = self.position.anchor.abs_diff(self.position.active);
                if count != 0 {
                    f.clipboard_copy(service, self.span(low, count)?, 1, 0);
                }
            }
        }
        Ok(())
    }
    pub fn key_down(&mut self, key: u32, modifiers: u32, f: &mut impl TextHost) -> Result<bool> {
        let (shift, control, alt) = (modifiers & 1 != 0, modifiers & 2 != 0, modifiers & 4 != 0);
        match key {
            0x43 | 0x58 => {
                self.copy(shift, control, alt, f)?;
                if key == 0x58
                    && control
                    && !shift
                    && !alt
                    && self.flags & 0x1000 != 0
                    && self.flags & 0x2000 == 0
                    && !f.capturing(self.window)
                {
                    self.delete_key(false, f)?;
                }
            }
            0x56 if control && !shift && !alt && self.flags & 0x1000 != 0 && !f.capturing(self.window) => {
                if let Some(service) = self.runtime.clipboard {
                    if let Some(bytes) = f.clipboard_paste(service, 0) {
                        self.insert_text(&bytes, f)?;
                        self.execute_repairs(f)?;
                    }
                }
            }
            0x5a | 0x59 if control && !alt => {
                let present = if key == 0x5a {
                    !self.undo.is_empty()
                } else {
                    !self.redo.is_empty()
                };
                if !present {
                    f.correction(3);
                } else {
                    if key == 0x5a {
                        self.undo(f)?;
                    } else {
                        self.redo(f)?;
                    }
                    self.execute_repairs(f)?;
                }
            }
            0x2d if self.flags & 0x81 == 0x81 => self.flags ^= 0x4000,
            0x2e => self.delete_key(false, f)?,
            8 => self.delete_key(true, f)?,
            0x1b if self.flags & 1 != 0 => {
                if self.position.anchor != self.position.active {
                    self.collapse();
                    f.invalidate(self.window);
                }
                return Ok(f.send(self.window, 5, [0x1b, modifiers as u64, 0]));
            }
            0x1b => self.copy(shift, control, alt, f)?,
            13 => {
                if self.flags & 0x400 != 0 {
                    f.send(self.window, 0x9b15_52d5, [self.text_view, 13, 0]);
                }
            }
            0x25 | 0x27 | 0x24 | 0x23 if self.flags & 1 != 0 => {
                if key == 0x25 || key == 0x27 {
                    if alt {
                        self.set_insertion_line_column(
                            self.position.line,
                            if key == 0x25 {
                                0
                            } else {
                                self.line_byte_count(self.position.line)?
                            },
                        )?;
                    }
                    if !(alt && key == 0x25) {
                        let delta = if key == 0x25 { -1 } else { 1 };
                        if control {
                            self.word_step(delta)?;
                        } else {
                            self.step(delta, true, f)?;
                        }
                    }
                } else if control {
                    self.position.index = if key == 0x24 { 0 } else { self.text.len() as u32 };
                    let (line, column) = self.line_column(self.position.index)?;
                    self.position.line = line;
                    self.position.column = column;
                } else {
                    self.set_insertion_line_column(
                        self.position.line,
                        if key == 0x24 {
                            0
                        } else {
                            self.line_byte_count(self.position.line)?
                        },
                    )?;
                }
                if key == 0x23 {
                    self.correct_lf();
                }
                self.finish_navigation(shift, true, f)?;
            }
            0x21 | 0x22 | 0x26 | 0x28 => {
                let page = key == 0x21 || key == 0x22;
                if self.flags & 1 == 0 || self.flags & 0x200 != 0 {
                    if !page && self.flags & 0x800 != 0 {
                        f.send(self.window, 0x9b15_52d5, [self.text_view, key as u64, 0]);
                    }
                    return Ok(true);
                }
                let old = self.position.line;
                let last = self.lines.len().saturating_sub(1) as u32;
                let down = key == 0x22 || key == 0x28;
                if (down && old == last || !down && old == 0) && self.position.anchor == self.position.active {
                    f.correction(2);
                    return Ok(true);
                }
                let (x, _) = self.position_from_line_column(old, self.position.column, f)?;
                if self.preferred_x == u32::MAX {
                    self.preferred_x = x as u32;
                }
                let count = if page { self.runtime.rows } else { 1 };
                let line = if down {
                    if old < (self.lines.len() as u32).wrapping_sub(count) {
                        old.wrapping_add(count)
                    } else {
                        last
                    }
                } else {
                    old.saturating_sub(count)
                };
                let column = self.column_from_position(line, if page { x } else { self.preferred_x as i32 }, f)?;
                self.set_insertion_line_column(line, column)?;
                self.correct_lf();
                self.finish_navigation(shift, false, f)?;
            }
            _ => {}
        }
        Ok(true)
    }
    pub fn key_up(&self, key: u32, modifiers: u32, f: &mut impl TextHost) -> bool {
        key != 0x1b || f.send(self.window, 6, [key as u64, modifiers as u64, 0])
    }
    pub fn wheel(&mut self, delta: i32, f: &mut impl TextHost) -> Result<bool> {
        if !f.capturing(self.window) {
            for _ in 0..(delta / 100).unsigned_abs() {
                self.scroll_vertical(if delta > 0 { Vertical::Up } else { Vertical::Down }, f)?;
            }
        }
        Ok(true)
    }
    pub fn select_word(&mut self, f: &mut impl TextHost) -> Result<bool> {
        if self.text.is_empty() {
            return Ok(false);
        }
        let at = self.position.index as usize;
        let byte = *self.text.get(at).ok_or(Unsupported::Precondition(
            "TextEdit word selection readable insertion byte",
        ))?;
        if separator(byte) {
            if at == 0 || separator(self.text[at - 1]) {
                self.insertion_in_view(f)?;
                return Ok(true);
            }
            self.position.index -= 1;
            self.collapse();
        }
        let mut low = self.position.anchor.min(self.position.active);
        let mut high = self.position.anchor.max(self.position.active);
        while low > 0 && !separator(*self.span(low - 1, 1)?.first().unwrap()) {
            low -= 1;
        }
        while high < self.text.len() as u32 && !separator(self.text[high as usize]) {
            high += 1;
            self.position.index = high;
        }
        self.position.anchor = low;
        self.position.active = high;
        self.fix_insertion()?;
        self.insertion_in_view(f)?;
        Ok(true)
    }
    pub fn do_message(&mut self, message: Message, f: &mut impl TextHost) -> Result<bool> {
        if message.kind == 3 && message.words[0] as u32 == 0x8871_13a3 {
            let axis = message.words[1];
            if axis == 0 || axis == 1 {
                if let Some(bar) = self.runtime.bars[axis as usize] {
                    let value = f.bar_value(bar, None);
                    let slot = if axis == 0 {
                        &mut self.first_visible_line
                    } else {
                        &mut self.runtime.horizontal_pixel
                    };
                    if *slot != value {
                        *slot = value;
                        f.invalidate(self.window);
                    }
                }
                return Ok(true);
            }
        }
        match message.kind {
            0x631b_56cc => {
                self.toggle_caret(f);
                Ok(true)
            }
            0x631b_56cd => {
                self.cursor_scroll(f)?;
                Ok(true)
            }
            _ => Ok(f.base_message(self.window, message)),
        }
    }
    pub fn cursor_scroll(&mut self, f: &mut impl TextHost) -> Result<()> {
        if !f.capturing(self.window) {
            return Ok(());
        }
        let (x, y) = f.cursor_position(self.window);
        let [w, h] = self.runtime.size;
        let half = (self.runtime.line_height >> 1) as i32;
        let position = if y < -2 {
            self.scroll_vertical(Vertical::Up, f)?.then_some((x, half))
        } else if y >= (h as i32).wrapping_add(2) {
            self.scroll_vertical(Vertical::Down, f)?
                .then_some((x, (h as i32).wrapping_sub(half)))
        } else if x < -2 {
            self.scroll_horizontal(false, f)?.then_some((0, y))
        } else if x >= (w as i32).wrapping_add(2) {
            self.scroll_horizontal(true, f)?.then_some((w as i32, y))
        } else {
            None
        };
        if let Some((x, y)) = position {
            self.mouse_position(x, y, false, f)?;
        }
        Ok(())
    }
    pub fn mouse_position(&mut self, x: i32, y: i32, shift: bool, f: &mut impl TextHost) -> Result<()> {
        let (line, column) = self.line_column_from_position(x, y, f)?;
        self.set_insertion_line_column(line, column)?;
        self.correct_lf();
        self.position.active = self.position.index;
        if !shift {
            self.position.anchor = self.position.index;
        }
        Ok(())
    }
    pub fn left_down(&mut self, x: i32, y: i32, modifiers: u32, f: &mut impl TextHost) -> Result<bool> {
        if self.flags & 1 == 0 {
            f.send(
                self.window,
                0x9b15_52d6,
                [self.text_view, x as u32 as u64, y as u32 as u64],
            );
            return Ok(true);
        }
        let double = f
            .click_elapsed()
            .is_some_and(|elapsed| elapsed < self.separate_interval);
        f.click_timer(!double);
        f.capture(self.window, true);
        self.mouse_position(x, y, modifiers & 1 != 0, f)?;
        if double {
            self.select_word(f)?;
        }
        f.invalidate(self.window);
        self.preferred_x = u32::MAX;
        Ok(true)
    }
    pub fn mouse_move(&mut self, x: i32, y: i32, modifiers: u32, f: &mut impl TextHost) -> Result<bool> {
        if self.flags & 1 != 0 {
            if f.capturing(self.window) {
                self.scroll_timer(true, f);
                let old = self.position.active;
                self.mouse_position(x, y, true, f)?;
                if old != self.position.active {
                    f.invalidate(self.window);
                }
            } else {
                self.scroll_timer(false, f);
            }
        }
        let _ = modifiers;
        Ok(true)
    }
    pub fn right_down(&self, x: i32, y: i32, f: &mut impl TextHost) -> bool {
        if !f.capturing(self.window) {
            f.send(
                self.window,
                0x9b15_52d7,
                [self.text_view, x as u32 as u64, y as u32 as u64],
            );
        }
        true
    }
    pub fn left_up(&mut self, f: &mut impl TextHost) -> bool {
        if self.flags & 1 != 0 && f.capturing(self.window) {
            f.capture(self.window, false);
            self.scroll_timer(false, f);
        }
        true
    }
}
