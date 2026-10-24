// SPDX-License-Identifier: GPL-3.0-or-later
//! TextEdit §9: original byte coordinates, mask-only drawing, pixel selection.
use super::{
    runtime::{Paint, TextHost},
    *,
};
use crate::widget_controls::Rect;

impl TextEdit {
    pub fn horizontal_metrics(&self, line: u32, scroll: u32, f: &mut impl TextHost) -> Result<(u32, u32, i32)> {
        let start = *self
            .lines
            .get(line as usize)
            .ok_or(Unsupported::Precondition("TextEdit horizontal line"))?;
        let count = self.line_byte_count(line)?;
        let bytes = self.span(start, count)?;
        if bytes.is_empty() {
            return Ok((start, start, 0));
        }
        let font = self.font()?;
        let step = |at: usize| -> Result<usize> {
            let tail = bytes
                .get(at..)
                .ok_or(Unsupported::Precondition("TE1 visible metric step"))?;
            let length = self.encoding.char_length(tail, tail.len())?;
            if length == 0 {
                return Err(Unsupported::Precondition("TE1 visible metric progress"));
            }
            Ok(length)
        };
        let mut left = 0;
        let mut displacement = 0;
        if scroll > 0 {
            let mut last = step(0)?;
            let mut at = last;
            let mut width = 0;
            let mut previous = 0;
            while at < bytes.len() {
                previous = width;
                width = f.font_measure(font, &bytes[..at]);
                if width > scroll {
                    break;
                }
                last = step(at)?;
                at += last;
            }
            left = at - last;
            displacement = previous.wrapping_sub(scroll) as i32;
        }
        let mut end = left + step(left)?;
        while end < bytes.len() {
            if f.font_measure(font, &bytes[left..end]) >= self.runtime.visible[0].wrapping_sub(displacement as u32) {
                break;
            }
            end += step(end)?;
        }
        Ok((
            start.wrapping_add(left as u32),
            start.wrapping_add(end as u32),
            displacement,
        ))
    }
    pub fn line_column_from_position(&self, mut x: i32, mut y: i32, f: &mut impl TextHost) -> Result<(u32, u32)> {
        if self.runtime.line_height == 0 {
            return Err(Unsupported::Precondition("TextEdit pixel hit DIV"));
        }
        if y < 0 {
            x = 0;
            y = 0;
        }
        x = x.max(0);
        let line = self
            .first_visible_line
            .wrapping_add(y as u32 / self.runtime.line_height)
            .min(self.lines.len().saturating_sub(1) as u32);
        let mask;
        let password = self.flags & 0x2000 != 0;
        let chosen = if password {
            mask = vec![self.mask; self.encoding.glyph_count(&self.text)];
            &mask[..]
        } else {
            &self.text[..]
        };
        if chosen.is_empty() {
            return Ok((0, 0));
        }
        let start = *self
            .lines
            .get(line as usize)
            .ok_or(Unsupported::Precondition("TextEdit pixel hit line"))? as usize;
        let count = self.line_byte_count(line)? as usize;
        let bytes = chosen
            .get(start..start + count)
            .ok_or(Unsupported::Precondition("TE6 original byte span in mask"))?;
        let font = self.font()?;
        let adjusted = (x as u32)
            .wrapping_sub(self.gutters[0])
            .wrapping_add(self.horizontal_pixels(f)?) as i32;
        let mut at = 0;
        let mut previous = 0i32;
        while at < bytes.len() {
            let step = if password {
                1
            } else {
                self.encoding.char_length(&bytes[at..], bytes.len() - at)?
            };
            let next = at + step;
            let prefix = bytes.get(..next).ok_or(Unsupported::Precondition("TE6 pixel prefix"))?;
            let width = f.font_measure(font, prefix) as i32;
            if width > adjusted {
                let column = if adjusted.wrapping_sub(previous).wrapping_sub(1) < width.wrapping_sub(adjusted) {
                    at
                } else {
                    next
                };
                let column = if password {
                    let mut byte = 0;
                    for _ in 0..column {
                        let tail = self
                            .text
                            .get(byte..)
                            .ok_or(Unsupported::Precondition("TE6 whole-text glyph prefix"))?;
                        byte += self.encoding.char_length(tail, tail.len())?;
                    }
                    byte
                } else {
                    column
                };
                return Ok((line, column as u32));
            }
            previous = width;
            at = next;
        }
        Ok((line, count as u32))
    }
    pub fn paint(&self, area: Rect, f: &mut impl TextHost) -> Result<bool> {
        let font = self.font()?;
        let focused = f.focused(self.window);
        let clip = [
            area[0].wrapping_add(self.gutters[0] as i32),
            area[1].wrapping_add(self.gutters[1] as i32),
            area[0].wrapping_add(self.gutters[0].wrapping_add(self.runtime.visible[0]) as i32),
            area[1].wrapping_add(self.gutters[1].wrapping_add(self.runtime.visible[1]) as i32),
        ];
        if self.flags & 0x20 != 0 {
            if let Some(image) = self.runtime.background {
                f.draw(Paint::Image(image, clip, self.runtime.background_option != 0));
            } else {
                let color = f.convert_color(self.runtime.fill_color, true);
                f.draw(Paint::Fill(clip, color));
            }
        }
        let password = self.flags & 0x2000 != 0;
        let mask = if password {
            vec![self.mask; self.encoding.glyph_count(&self.text)]
        } else {
            Vec::new()
        };
        let chosen = if password { &mask } else { &self.text };
        let endpoint = |value: u32| -> Result<u32> {
            if password {
                Ok(self.encoding.glyph_count(self.span(0, value)?) as u32)
            } else {
                Ok(value)
            }
        };
        let a = endpoint(self.position.anchor)?;
        let b = endpoint(self.position.active)?;
        let selection = (a.min(b), a.max(b));
        let foreground = f.convert_color(self.runtime.colors[0], false);
        let selected_foreground = f.convert_color(self.runtime.colors[2], false);
        let background = f.convert_color(self.runtime.colors[1], false);
        let selected_background = f.convert_color(self.runtime.colors[3], false);
        let highlight = f.convert_color(self.runtime.highlight_color, false);
        let end = self
            .first_visible_line
            .wrapping_add(self.runtime.rows)
            .min(self.lines.len() as u32);
        if self.runtime.visible[0] != 0 {
            for line in self.first_visible_line..end {
                let (start, end, displacement) = if self.flags & 2 != 0 && self.runtime.bars[1].is_none() {
                    let start = self.lines[line as usize];
                    (start, start.wrapping_add(self.line_byte_count(line)?), 0)
                } else {
                    self.horizontal_metrics(line, self.horizontal_pixels(f)?, f)?
                };
                let bytes = chosen
                    .get(start as usize..end as usize)
                    .ok_or(Unsupported::Precondition("TE6 original draw span in mask"))?;
                let x = clip[0].wrapping_add(displacement);
                let y = clip[1].wrapping_add(
                    line.wrapping_sub(self.first_visible_line)
                        .wrapping_mul(self.runtime.line_height) as i32,
                );
                let low = selection.0.max(start);
                let high = selection.1.min(end);
                let selected = (focused || self.flags & 0x10000 != 0) && low < high;
                if !selected || low != start || high != end {
                    f.draw(Paint::Text {
                        font,
                        bytes: bytes.to_vec(),
                        x,
                        y,
                        clip,
                        foreground,
                        background,
                    });
                }
                if selected {
                    let offset = f.font_measure(font, &bytes[..(low - start) as usize]) as i32;
                    let selected_bytes = &bytes[(low - start) as usize..(high - start) as usize];
                    let width = f.font_measure(font, selected_bytes) as i32;
                    f.draw(Paint::Fill(
                        [
                            x.wrapping_add(offset),
                            y,
                            x.wrapping_add(offset).wrapping_add(width),
                            y.wrapping_add(self.runtime.line_height as i32),
                        ],
                        highlight,
                    ));
                    f.draw(Paint::Text {
                        font,
                        bytes: selected_bytes.to_vec(),
                        x: x.wrapping_add(offset),
                        y,
                        clip,
                        foreground: selected_foreground,
                        background: selected_background,
                    });
                }
            }
        }
        if focused
            && self.runtime.caret_visible
            && self.position.line >= self.first_visible_line
            && self.position.line < end
        {
            let start = self.lines[self.position.line as usize] as usize;
            let prefix = chosen
                .get(start..start + self.position.column as usize)
                .ok_or(Unsupported::Precondition("TE6 original caret column in mask"))?;
            let advance = f.font_measure(font, prefix);
            let mut x = (clip[0] as u32)
                .wrapping_add(advance)
                .wrapping_sub(self.horizontal_pixels(f)?) as i32;
            if self.flags & 2 != 0 && x >= clip[2] {
                x = clip[2].wrapping_sub(1);
            }
            let top = clip[1]
                .wrapping_add(
                    self.position
                        .line
                        .wrapping_sub(self.first_visible_line)
                        .wrapping_mul(self.runtime.line_height) as i32,
                )
                .max(area[1].wrapping_add(1));
            let bottom = top
                .wrapping_add(self.runtime.line_height as i32)
                .min(area[3].wrapping_sub(1));
            f.draw(Paint::LineWidth(if self.runtime.line_height < 24 { 1.0 } else { 2.0 }));
            let color = f.convert_color(self.runtime.caret_color, false);
            f.draw(Paint::Caret { x, top, bottom, color });
            f.draw(Paint::LineWidth(1.0));
        }
        if focused && self.flags & 0x10 != 0 {
            let color = f.convert_color(self.runtime.outline_color, false);
            f.draw(Paint::Outline(area, color));
        }
        Ok(true)
    }
}
