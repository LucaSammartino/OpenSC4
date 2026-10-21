// SPDX-License-Identifier: GPL-3.0-or-later
//! Measuring and layout of `cGZFont` (drawing-fonts.md §8) and DrawTextParagraph (§9).
//!
//! The spec leaves `UTF16GetWord`, the "break characters", Thai clusters and "composed characters"
//! unspecified (question FN13). Literal reading used here: a composed character is one converted
//! UTF-16 unit; a word is a run of units other than space (U+0020), tab, CR and LF, followed by its
//! run of spaces/tabs; the break characters are space and tab; a Thai cluster is one unit.

use super::encoding::Converted;
use super::font::GzFont;
use super::SurfaceId;

const CR: u16 = 0x0D;
const LF: u16 = 0x0A;

fn is_blank(u: u16) -> bool {
    u == 0x20 || u == 0x09
}
/// Whitespace dropped by GetStringVisibleWidth (FN13: the set is not listed).
fn is_space(u: u16) -> bool {
    u == 0x20 || (0x09..=0x0D).contains(&u)
}

/// Rectangle {left, top, right, bottom}.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl GzFont {
    /// Slot 36, CharWidth(ch).
    pub fn char_width(&mut self, ch: u32) -> i32 {
        if !self.initialised {
            return 0;
        }
        self.get_char_data(ch).map_or(0, |g| g.width())
    }
    /// Slot 58, GetXOffset(ch).
    pub fn get_x_offset(&mut self, ch: u32) -> i32 {
        self.get_char_data(ch).map_or(0, |g| g.x_offset as i32)
    }
    /// Slot 40, GetAdvanceWidth(ch): advance + charSpacing (0 without a glyph, FN13).
    pub fn get_advance_width(&mut self, ch: u32) -> i32 {
        match self.get_char_data(ch) {
            Some(g) => g.advance as i32 + self.char_spacing,
            None => 0,
        }
    }

    fn units_width(&mut self, units: &[u16]) -> i32 {
        units.iter().map(|&u| self.get_advance_width(u as u32)).sum()
    }

    /// Slot 48, GetStringWidth(s, n).
    pub fn get_string_width(&mut self, s: &[u8]) -> i32 {
        let c = self.convert(s);
        self.units_width(&c.units)
    }

    /// Slot 49, GetStringVisibleWidth(s, n, &len): (width, len).
    pub fn get_string_visible_width(&mut self, s: &[u8]) -> (i32, u32) {
        let c = self.convert(s);
        let mut k = c.units.len();
        while k > 0 && is_space(c.units[k - 1]) {
            k -= 1;
        }
        let w = self.units_width(&c.units[..k]);
        (w, c.src_len_of(k))
    }

    /// Slot 42, CalculateCharsToFitInWidth(s, n, w, &width): (source length, width).
    pub fn chars_to_fit(&mut self, s: &[u8], w: i32) -> (u32, i32) {
        let c = self.convert(s);
        let mut width = 0;
        let mut k = 0;
        while k < c.units.len() {
            let a = self.get_advance_width(c.units[k] as u32);
            if width + a > w {
                break;
            }
            width += a;
            k += 1;
        }
        (c.src_len_of(k), width)
    }

    /// End (exclusive) of the word starting at `pos`.
    fn word_end(units: &[u16], pos: usize) -> (usize, usize) {
        let mut e = pos;
        while e < units.len() && !is_blank(units[e]) && units[e] != CR && units[e] != LF {
            e += 1;
        }
        let vis_end = e;
        while e < units.len() && is_blank(units[e]) {
            e += 1;
        }
        (vis_end, e)
    }

    fn words_count(&mut self, c: &Converted, w: i32, mode: u32) -> usize {
        let u = &c.units;
        let mut pos = 0usize;
        let mut width = 0i32;
        while pos < u.len() {
            if u[pos] == CR || u[pos] == LF {
                pos += 1;
                break;
            }
            let (vis_end, end) = Self::word_end(u, pos);
            let vis = self.units_width(&u[pos..vis_end]);
            if width + vis > w {
                break;
            }
            width += self.units_width(&u[vis_end..end]) + vis;
            pos = end;
        }
        if pos == 0 && !u.is_empty() {
            if mode == 1 {
                while pos < u.len() && !matches!(u[pos], CR | LF | 0) && !is_blank(u[pos]) {
                    pos += 1;
                }
            } else {
                let mut k = 0usize;
                let mut pw = 0;
                while k < u.len() && pw < w {
                    pw += self.get_advance_width(u[k] as u32);
                    k += 1;
                }
                if pw >= w && k > 0 {
                    k -= 1;
                }
                // At least one unit, so line counting always advances (FN13).
                k = k.max(1);
                if k < u.len() && c.src[k] == c.src[k - 1] {
                    k += 1;
                }
                pos = k;
            }
        }
        if pos > 1 && pos < u.len() && !is_blank(u[pos - 1]) && u[pos] == 0x20 {
            pos += 1;
        }
        pos
    }

    /// Slot 41, CalculateWordsToFitInWidth(s, n, w, &width, mode): (source length, width).
    pub fn words_to_fit(&mut self, s: &[u8], w: i32, mode: u32) -> (u32, i32) {
        self.glyph_map.clear();
        let c = self.convert(s);
        self.glyph_map = c.src.clone();
        let k = self.words_count(&c, w, mode);
        let len = c.src_len_of(k);
        let width = self.get_string_width(&s[..len as usize]);
        (len, width)
    }

    /// Slot 43, CalculateNumberOfLines(s, n, w).
    pub fn number_of_lines(&mut self, s: &[u8], w: i32) -> u32 {
        let w = if w == 0 { 1 } else { w };
        let mut pos = 0usize;
        let mut lines = 0;
        while pos < s.len() {
            let (len, _) = self.words_to_fit(&s[pos..], w, 2);
            if len == 0 {
                break;
            }
            lines += 1;
            pos += len as usize;
        }
        lines
    }

    /// Slot 47, TextArea(s, n, rect).
    pub fn text_area(&mut self, s: &[u8]) -> Rect {
        Rect {
            left: 0,
            top: 0,
            right: self.get_string_width(s),
            bottom: self.char_height(),
        }
    }

    /// Slot 44, CalculateWidthOfLines(s, n, height).
    pub fn width_of_lines(&mut self, s: &[u8], height: i32) -> u32 {
        // lineHeight 0 is not guarded in the original (a division fault); 0 is returned here.
        let Some(max_lines) = (height as u32).checked_div(self.get_line_height() as u32) else {
            return 0;
        };
        let (mut lo, mut hi) = (2u32, 400u32);
        while lo <= hi {
            let mid = (lo + hi) >> 1;
            let lines = self.number_of_lines(s, (mid * 10) as i32);
            if lines > max_lines {
                lo = mid + 1;
            } else if lo == hi {
                return lo * 10;
            } else {
                hi = mid;
            }
        }
        0
    }

    /// Slot 46, CalculateTextArea(s, n, rect, wrap, fixWidth).
    pub fn calculate_text_area(&mut self, s: Option<&[u8]>, rect: &mut Rect, wrap: bool, fix_width: bool) {
        let lh = self.get_line_height();
        match s {
            None | Some([]) => {
                rect.right = rect.left;
                rect.bottom = rect.top + lh;
            }
            Some(s) if !wrap => {
                rect.right = rect.left + self.get_string_width(s);
                rect.bottom = rect.top + lh;
            }
            Some(s) if !fix_width => {
                rect.right = rect.left + self.width_of_lines(s, rect.bottom - rect.top) as i32;
            }
            Some(s) => {
                rect.bottom = rect.top + self.number_of_lines(s, rect.right - rect.left) as i32 * lh;
            }
        }
    }

    /// Slot 45, CalculateTextArea(s, n, rect, minW, maxW, maxH).
    pub fn calculate_text_area_box(
        &mut self,
        s: &[u8],
        rect: &mut Rect,
        mut min_w: i32,
        mut max_w: i32,
        max_h: i32,
    ) -> bool {
        let lh = self.get_line_height();
        let max_h = max_h.max(lh);
        if min_w == -1 || max_w == -1 {
            let aw = self.char_width('a' as u32);
            if min_w == -1 {
                min_w = 12 * aw;
            }
            if max_w == -1 {
                max_w = 32 * aw;
            }
            if max_w <= min_w {
                max_w = 2 * min_w;
            }
        }
        let mut w = 1i32;
        let has_nl = s.contains(&10);
        let set = |rect: &mut Rect, ww: i32, hh: i32| {
            rect.right = rect.left + ww;
            rect.bottom = rect.top + hh;
        };
        if !has_nl {
            w = self.get_string_width(s);
            if w < min_w {
                set(rect, w, lh);
                return true;
            }
            if w < 2 * min_w {
                let w2 = self.width_of_lines(s, 2 * lh);
                if w2 < max_w as u32 && 2 * lh <= max_h {
                    set(rect, w2 as i32, 2 * lh);
                    return true;
                }
            }
            if w < max_w {
                set(rect, w, lh);
                return true;
            }
        }
        let mut k = if has_nl { 1 } else { w / max_w + 1 };
        while k < max_h / lh {
            let h = k * lh;
            let ww = self.width_of_lines(s, h);
            if ww != 0 && ww < max_w as u32 && h <= max_h {
                set(rect, ww as i32, h);
                return true;
            }
            k += 1;
        }
        set(rect, 0, 0);
        false
    }

    /// Slot 50, FormatForEllipsis(text, ellipsis, rect, keepAll): edits `text`, returns its
    /// original length.
    pub fn format_for_ellipsis(&mut self, text: &mut Vec<u8>, ellipsis: &[u8], rect: Rect, keep_all: bool) -> u32 {
        let l = text.len() as u32;
        if keep_all {
            return l;
        }
        let w = rect.right - rect.left;
        let (fit, _) = self.chars_to_fit(text, w);
        if fit >= l {
            return l;
        }
        let e = self.get_string_width(ellipsis);
        if e < w {
            let (fit2, _) = self.chars_to_fit(text, w - e);
            text.truncate(fit2 as usize);
            while text.last() == Some(&b' ') {
                text.pop();
            }
            text.extend_from_slice(ellipsis);
        } else {
            text.clear();
        }
        l
    }

    /// Slot 52, DrawTextParagraph. Returns `initialised` (the spec gives no return rule).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text_paragraph(
        &mut self,
        target: Option<SurfaceId>,
        rect: Rect,
        text: &[u8],
        align: u32,
        fg: u32,
        bg: u32,
        clip: Option<(i32, i32, i32, i32)>,
    ) -> bool {
        let w = rect.right - rect.left;
        let lh = self.get_line_height();
        let mut y = rect.top;
        let mut pos = 0usize;
        while pos < text.len() && y < rect.bottom {
            let (len, line_w) = self.words_to_fit(&text[pos..], w, 2);
            if len == 0 {
                break;
            }
            let mut line = &text[pos..pos + len as usize];
            while let Some((&b, rest)) = line.split_last() {
                if b == b'\r' || b == b'\n' {
                    line = rest;
                } else {
                    break;
                }
            }
            let x = match align {
                1 => rect.left + w / 2 - line_w / 2,
                2 => rect.right - line_w,
                _ => rect.left,
            };
            if !self.draw_text_line(target, x, y, line, fg, bg, clip, None) {
                return false;
            }
            pos += len as usize;
            y += lh;
        }
        self.initialised
    }
}
