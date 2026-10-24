// SPDX-License-Identifier: GPL-3.0-or-later
//! Native byte layout/view arithmetic. Foreign metrics never choose policy.
use super::{runtime::TextHost, *};
use crate::widget_controls::Rect;
use std::sync::atomic::{AtomicBool, Ordering};

static AUTOMATIC_REFLOW: AtomicBool = AtomicBool::new(false);
struct ReflowGuard;
impl Drop for ReflowGuard {
    fn drop(&mut self) {
        AUTOMATIC_REFLOW.store(false, Ordering::Release);
    }
}

impl TextEdit {
    pub(crate) fn font(&self) -> Result<Handle> {
        self.runtime
            .font
            .ok_or(Unsupported::Precondition("TextEdit metric font"))
    }
    pub(crate) fn span(&self, start: u32, count: u32) -> Result<&[u8]> {
        self.text
            .get(start as usize..start as usize + count as usize)
            .ok_or(Unsupported::Precondition("TextEdit readable byte span"))
    }
    pub fn calculate_visible(&mut self, f: &mut impl TextHost) {
        let vertical = self.runtime.bars[0].map_or(0, |b| f.bar_dimensions(b).0);
        let horizontal = self.runtime.bars[1].map_or(0, |b| f.bar_dimensions(b).1);
        self.runtime.visible = [
            self.runtime.size[0].saturating_sub(self.gutters[0].wrapping_mul(2).wrapping_add(vertical)),
            self.runtime.size[1].saturating_sub(self.gutters[1].wrapping_mul(2).wrapping_add(horizontal)),
        ];
    }
    pub fn text_area(&self) -> Rect {
        [
            self.gutters[0] as i32,
            self.gutters[1] as i32,
            self.gutters[0].wrapping_add(self.runtime.visible[0]) as i32,
            self.gutters[1].wrapping_add(self.runtime.visible[1]) as i32,
        ]
    }
    pub fn calculate_lines(&mut self, f: &mut impl TextHost) -> Result<()> {
        let font = self.font()?;
        self.lines.clear();
        if self.text.is_empty() {
            self.position.line = 0;
            self.position.column = 0;
            self.position.index = 0;
            return Ok(());
        }
        self.lines.push(0);
        let mut at = 0;
        while at < self.text.len() {
            let remaining = &self.text[at..];
            let count = if self.runtime.visible[0] == 0 || self.flags & 0x200 != 0 {
                remaining.len()
            } else if self.flags & 2 == 0 {
                remaining
                    .iter()
                    .position(|&b| b == b'\n')
                    .map_or(remaining.len(), |n| n + 1)
            } else {
                f.font_fit(font, remaining, self.runtime.visible[0]) as usize
            };
            if count == 0 || count > remaining.len() {
                return Err(Unsupported::Precondition("TE8 font fitting progress/span"));
            }
            at += count;
            if at < self.text.len() || self.text[at - 1] == b'\n' {
                self.lines.push(at as u32);
            }
        }
        Ok(())
    }
    pub fn recalculate(&mut self, f: &mut impl TextHost) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }
        self.runtime.line_height = self.runtime.font.map_or(16, |font| f.font_height(font));
        self.calculate_visible(f);
        if self.runtime.line_height == 0 {
            return Err(Unsupported::Precondition("TextEdit nonzero line height for DIV"));
        }
        self.runtime.rows = self.runtime.visible[1] / self.runtime.line_height;
        if self.runtime.rows == 0 && (self.runtime.visible[1] as i32) > 0 {
            self.runtime.rows = 1;
        }
        // Init is allowed to complete without a font; font-dependent calls are not.
        if self.runtime.font.is_some() {
            self.calculate_lines(f)?;
        }
        self.bar_values(true, f)?;
        self.fix_insertion()?;
        self.first_visible_line = self.first_visible_line.min(self.lines.len().saturating_sub(1) as u32);
        // TE11: literal shared guard against nested automatic reflow. The spec
        // does not assign a larger permitted recursion threshold.
        if AUTOMATIC_REFLOW
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            let _guard = ReflowGuard;
            self.automatic_bars(f)?;
        }
        Ok(())
    }
    pub fn execute_repairs(&mut self, f: &mut impl TextHost) -> Result<()> {
        // Preserve order (and duplicated work) from the byte-stage call sites.
        while !self.repairs.is_empty() {
            match self.repairs[0] {
                Repair::RecalculateAll => self.recalculate(f)?,
                Repair::CalculateLines => self.calculate_lines(f)?,
                Repair::ScrollValues => self.bar_values(true, f)?,
                Repair::InsertionInView => self.insertion_in_view(f)?,
                Repair::AutomaticBars => self.automatic_bars(f)?,
            }
            self.repairs.remove(0);
        }
        Ok(())
    }
    pub fn longest_width(&self, all: bool, f: &mut impl TextHost) -> Result<u32> {
        let font = self.font()?;
        let range = if all {
            0..self.lines.len() as u32
        } else {
            self.first_visible_line..self.first_visible_line.wrapping_add(self.runtime.rows)
        };
        let mut maximum = 0;
        for line in range {
            let start = *self
                .lines
                .get(line as usize)
                .ok_or(Unsupported::Precondition("TextEdit coherent visible line range"))?;
            maximum = maximum.max(f.font_measure(font, self.span(start, self.line_byte_count(line)?)?));
        }
        Ok(maximum)
    }
    pub fn horizontal_pixels(&self, f: &mut impl TextHost) -> Result<u32> {
        if self.flags & 0x200 != 0 {
            Ok(f.font_measure(self.font()?, self.span(0, self.runtime.horizontal_byte)?))
        } else {
            Ok(self.runtime.horizontal_pixel)
        }
    }
    pub fn bar_sizes(&self, f: &mut impl TextHost) {
        let [width, height] = self.runtime.size;
        if let Some(horizontal) = self.runtime.bars[1] {
            let h = f.bar_dimensions(horizontal).1;
            let v = self.runtime.bars[0].map_or(0, |bar| f.bar_dimensions(bar).0);
            f.bar_place(
                horizontal,
                [
                    0,
                    height.wrapping_sub(h) as i32,
                    width.wrapping_sub(v) as i32,
                    height as i32,
                ],
            );
        }
        if let Some(vertical) = self.runtime.bars[0] {
            let v = f.bar_dimensions(vertical).0;
            let h = self.runtime.bars[1].map_or(0, |bar| f.bar_dimensions(bar).1);
            f.bar_place(
                vertical,
                [
                    width.wrapping_sub(v).wrapping_add(1) as i32,
                    0,
                    width as i32,
                    height.wrapping_sub(h) as i32,
                ],
            );
        }
    }
    pub fn bar_values(&self, recompute: bool, f: &mut impl TextHost) -> Result<()> {
        for axis in 0..2 {
            if let Some(bar) = self.runtime.bars[axis] {
                if recompute && self.runtime.rows != 0 {
                    let (maximum, step) = if axis == 0 {
                        let count = self.lines.len() as u32;
                        (
                            (count.wrapping_sub(self.runtime.rows) as i32).max(0),
                            count / self.runtime.rows + 1,
                        )
                    } else {
                        let width = self.longest_width(true, f)?;
                        let visible = self.runtime.visible[0] as i32;
                        if visible == 0 {
                            return Err(Unsupported::Precondition("TextEdit horizontal range DIV"));
                        }
                        let quotient = (width as i32)
                            .checked_div(visible)
                            .ok_or(Unsupported::Precondition("TextEdit signed range DIV"))?;
                        (
                            (width.wrapping_sub(visible as u32) as i32).max(0),
                            quotient.wrapping_add(1) as u32,
                        )
                    };
                    f.bar_range(bar, maximum);
                    f.bar_pair(bar, maximum, step);
                }
                f.bar_value(
                    bar,
                    Some(if axis == 0 {
                        self.first_visible_line
                    } else {
                        self.runtime.horizontal_pixel
                    }),
                );
            }
        }
        Ok(())
    }
    pub fn add_bars(&mut self, vertical: bool, horizontal: bool, f: &mut impl TextHost) -> Result<bool> {
        let mut attempted = false;
        for (axis, requested, bit, args) in [
            (0, vertical, 8, [0, 2, 0, 100, 0]),
            (1, horizontal, 4, [1, 1, 0, 100, 0]),
        ] {
            if requested && self.flags & bit != 0 && self.runtime.bars[axis].is_none() {
                attempted = true;
                self.runtime.bars[axis] = f.bar_create(axis == 1, args);
                if let Some(bar) = self.runtime.bars[axis] {
                    self.apply_bar_image(axis, f);
                    f.bar_add_child(self.window, bar);
                } else {
                    self.clear_flag(bit, f)?;
                }
            }
        }
        if attempted {
            self.recalculate(f)?;
            self.bar_sizes(f);
            self.bar_values(true, f)?;
        }
        Ok(true)
    }
    pub fn remove_bars(&mut self, vertical: bool, horizontal: bool, f: &mut impl TextHost) -> Result<()> {
        let mut removed = false;
        for (axis, requested) in [(0, vertical), (1, horizontal)] {
            if requested {
                if let Some(bar) = self.runtime.bars[axis] {
                    f.bar_remove_child(self.window, bar);
                    f.relinquish(bar);
                    self.runtime.bars[axis] = None;
                    removed = true;
                }
            }
        }
        if removed {
            self.recalculate(f)?;
        }
        Ok(())
    }
    pub fn automatic_bars(&mut self, f: &mut impl TextHost) -> Result<()> {
        let nonzero = self.runtime.visible.iter().all(|&v| v != 0);
        let vertical = self.flags & 8 != 0 && self.lines.len() as u32 > self.runtime.rows && nonzero;
        let horizontal = self.flags & 4 != 0
            && self.flags & 2 == 0
            && nonzero
            && self.runtime.font.is_some()
            && self.longest_width(true, f)? > self.runtime.visible[0];
        self.remove_bars(!vertical, !horizontal, f)?;
        self.add_bars(vertical, horizontal, f)?;
        Ok(())
    }
    pub fn scroll_vertical(&mut self, operation: Vertical, f: &mut impl TextHost) -> Result<bool> {
        let first = self.first_visible_line;
        let count = self.lines.len() as u32;
        let rows = self.runtime.rows;
        let remaining = count.wrapping_sub(rows).wrapping_sub(first) as i32;
        self.first_visible_line = match operation {
            Vertical::Up if first as i32 > 0 => first - 1,
            Vertical::Down if remaining > 0 => first.wrapping_add(1),
            Vertical::Start if first != 0 || self.runtime.horizontal_byte != 0 => {
                self.runtime.horizontal_byte = 0;
                self.runtime.horizontal_pixel = 0;
                0
            }
            Vertical::End if remaining > 0 => {
                self.runtime.horizontal_byte = 0;
                self.runtime.horizontal_pixel = 0;
                count.wrapping_sub(rows)
            }
            Vertical::PageUp if first != 0 => {
                if (first as i32) >= (rows as i32) {
                    first.wrapping_sub(rows)
                } else {
                    0
                }
            }
            Vertical::PageDown if remaining > 0 => {
                if first < count {
                    first.wrapping_add(rows)
                } else {
                    count.wrapping_sub(1)
                }
            }
            _ => return Ok(false),
        };
        self.bar_values(false, f)?;
        f.invalidate(self.window);
        Ok(true)
    }
    pub fn scroll_horizontal(&mut self, right: bool, f: &mut impl TextHost) -> Result<bool> {
        if self.flags & 0x200 != 0 {
            let at = self.runtime.horizontal_byte as usize;
            if right && at < self.text.len() {
                self.runtime.horizontal_byte = self
                    .runtime
                    .horizontal_byte
                    .wrapping_add(self.encoding.char_length(&self.text[at..], self.text.len() - at)? as u32);
            } else if !right && (at as i32) > 0 {
                let prefix = self
                    .text
                    .get(..at)
                    .ok_or(Unsupported::Precondition("TextEdit horizontal prefix"))?;
                self.runtime.horizontal_byte = self
                    .runtime
                    .horizontal_byte
                    .wrapping_sub(self.encoding.previous_length(prefix) as u32);
            }
        } else {
            if self.flags & 2 != 0 && self.runtime.bars[1].is_none() {
                return Ok(true);
            }
            let offset = self.runtime.horizontal_pixel;
            if right && offset <= self.runtime.visible[0] {
                self.runtime.horizontal_pixel = offset
                    .wrapping_add(self.runtime.line_height)
                    .min(self.runtime.visible[0]);
            } else if !right && offset as i32 > 0 {
                self.runtime.horizontal_pixel = offset.wrapping_sub(self.runtime.line_height);
            }
            self.bar_values(false, f)?;
        }
        f.invalidate(self.window);
        Ok(true)
    }
    pub fn position_from_line_column(&self, line: u32, column: u32, f: &mut impl TextHost) -> Result<(i32, i32)> {
        let advance = if self.lines.is_empty() {
            0
        } else {
            let start = self.lines[(line as usize).min(self.lines.len() - 1)];
            f.font_measure(
                self.font()?,
                self.span(start, column.min(self.text.len() as u32 - start))?,
            )
        };
        let x = advance
            .wrapping_add(self.gutters[0])
            .wrapping_sub(self.horizontal_pixels(f)?);
        let line = line.min(self.lines.len().saturating_sub(1) as u32);
        Ok((
            x as i32,
            line.wrapping_sub(self.first_visible_line)
                .wrapping_mul(self.runtime.line_height)
                .wrapping_add(self.gutters[1]) as i32,
        ))
    }
    pub fn column_from_position(&self, line: u32, x: i32, f: &mut impl TextHost) -> Result<u32> {
        if self.lines.is_empty() {
            return Ok(0);
        }
        let x = (x as u32)
            .wrapping_add(self.horizontal_pixels(f)?)
            .wrapping_sub(self.gutters[0]) as i32;
        if x <= 0 {
            return Ok(0);
        }
        let line = line.min(self.lines.len() as u32 - 1);
        let bytes = self.span(self.lines[line as usize], self.line_byte_count(line)?)?;
        let font = self.font()?;
        let mut at = 0;
        while at < bytes.len() {
            let next = at + self.encoding.char_length(&bytes[at..], bytes.len() - at)?;
            let prefix = bytes
                .get(..next)
                .ok_or(Unsupported::Precondition("TE1 metric prefix"))?;
            if x as u32 <= f.font_measure(font, prefix) {
                return Ok(at as u32);
            }
            at = next;
        }
        Ok(at as u32)
    }
    pub fn insertion_in_view(&mut self, f: &mut impl TextHost) -> Result<()> {
        let old = (self.first_visible_line, self.runtime.horizontal_pixel);
        let line = self.position.line;
        if line < self.first_visible_line {
            self.first_visible_line = line;
        } else if line >= self.first_visible_line.wrapping_add(self.runtime.rows) {
            self.first_visible_line = line.wrapping_sub(self.runtime.rows).wrapping_add(1);
        }
        if self.flags & 0x200 != 0 {
            let column = self.position.column;
            if column < self.runtime.horizontal_byte {
                self.runtime.horizontal_byte = column;
            } else {
                let advance = f.font_measure(self.font()?, self.span(0, column)?);
                if (advance.wrapping_sub(self.horizontal_pixels(f)?) as i32) > self.runtime.visible[0] as i32 {
                    let target = (self.runtime.visible[0].wrapping_mul(3) as i32) / 4;
                    while self.runtime.horizontal_byte < self.text.len() as u32 {
                        let at = self.runtime.horizontal_byte as usize;
                        let step = self.encoding.char_length(&self.text[at..], self.text.len() - at)? as u32;
                        if step == 0 {
                            return Err(Unsupported::Precondition("TE1 scroll progress"));
                        }
                        self.runtime.horizontal_byte = self.runtime.horizontal_byte.wrapping_add(step);
                        if (advance.wrapping_sub(self.horizontal_pixels(f)?) as i32) < target {
                            break;
                        }
                    }
                }
            }
            self.bar_values(false, f)?;
        } else {
            if self.runtime.bars[1].is_some() || self.flags & 2 == 0 {
                let start = self.lines.get(line as usize).copied().unwrap_or(0);
                let advance = f.font_measure(self.font()?, self.span(start, self.position.column)?);
                let relative = advance.wrapping_sub(self.horizontal_pixels(f)?) as i32;
                if relative < 0 {
                    self.runtime.horizontal_pixel = self.runtime.horizontal_pixel.wrapping_add(relative as u32);
                } else if relative as u32 >= self.runtime.visible[0] {
                    self.runtime.horizontal_pixel = self
                        .runtime
                        .horizontal_pixel
                        .wrapping_add(relative as u32)
                        .wrapping_sub(self.runtime.visible[0]);
                }
            }
            if old != (self.first_visible_line, self.runtime.horizontal_pixel) {
                self.bar_values(false, f)?;
            }
        }
        Ok(())
    }
    pub fn update_view(&mut self, f: &mut impl TextHost) -> Result<()> {
        if self.initialized {
            self.insertion_in_view(f)?;
            self.automatic_bars(f)?;
        }
        f.invalidate(self.window);
        Ok(())
    }
    pub fn show(&mut self, show: bool, f: &mut impl TextHost) -> Result<()> {
        self.update_view(f)?;
        f.base_show(self.window, show);
        Ok(())
    }
    pub fn exact_line_heights(&mut self, f: &mut impl TextHost) -> Result<bool> {
        let height = self.runtime.size[1].wrapping_sub(self.gutters[1].wrapping_mul(2));
        let line_height = self.runtime.line_height;
        if line_height == 0 {
            return Err(Unsupported::Precondition("TextEdit exact-height DIV"));
        }
        let quotient = height / line_height;
        let rows = if quotient as i32 <= 0 { 1 } else { quotient };
        let new_height = rows.wrapping_mul(line_height);
        if new_height != height {
            self.runtime.size[1] = new_height.wrapping_add(self.gutters[1].wrapping_mul(2));
            f.base_area(
                self.window,
                [0, 0, self.runtime.size[0] as i32, self.runtime.size[1] as i32],
            );
        }
        self.bar_sizes(f);
        self.bar_values(true, f)?;
        f.invalidate(self.window);
        Ok(true)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vertical {
    Up,
    Down,
    Start,
    End,
    PageUp,
    PageDown,
}
