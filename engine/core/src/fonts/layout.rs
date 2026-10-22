// SPDX-License-Identifier: GPL-3.0-or-later
//! Measuring and layout of `cGZFont` (drawing-fonts.md §8) and DrawTextParagraph (§9). Words,
//! clusters and the space class (§8.4, FN13) are in `words.rs`.

use super::encoding::Converted;
use super::font::GzFont;
use super::words;
use super::SurfaceId;

const CR: u16 = 0x0D;
const LF: u16 = 0x0A;

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
    /// Slot 40, GetAdvanceWidth(ch): advance + charSpacing; 0 (no charSpacing) when uninitialised
    /// or without a glyph record (§8.4).
    pub fn get_advance_width(&mut self, ch: u32) -> i32 {
        if !self.initialised {
            return 0;
        }
        match self.get_char_data(ch) {
            Some(g) => g.advance as i32 + self.char_spacing,
            None => 0,
        }
    }

    /// GetStringWidth over the first `n` units of `buf` (§8.4): the advance of the first unit of
    /// each composed character.
    fn units_width(&mut self, buf: &[u16], n: usize) -> i32 {
        let mut w = 0i32;
        let mut i = 0;
        while i < n {
            w = w.wrapping_add(self.get_advance_width(buf[i] as u32));
            i += words::composed_len(&buf[i..], n - i).max(1);
        }
        w
    }

    /// Units left after dropping trailing units of the space class (§8.4).
    fn visible_len(units: &[u16]) -> usize {
        let mut k = units.len();
        while k > 0 && words::is_space(units[k - 1]) {
            k -= 1;
        }
        k
    }

    /// Slot 48, GetStringWidth(s, n).
    pub fn get_string_width(&mut self, s: &[u8]) -> i32 {
        let c = self.convert(s);
        self.units_width(&c.units, c.units.len())
    }

    /// Slot 49, GetStringVisibleWidth(s, n, &len): (width, len).
    pub fn get_string_visible_width(&mut self, s: &[u8]) -> (i32, u32) {
        let c = self.convert(s);
        let k = Self::visible_len(&c.units);
        let w = self.units_width(&c.units, k);
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

    /// §8.1 on converted text: the unit count before the §11 map-back.
    fn words_count(&mut self, c: &Converted, w: i32, mode: u32) -> Option<usize> {
        let u = &c.units;
        let n = u.len();
        let mut i = 0usize;
        let mut sum = 0i32;
        while i < n {
            if u[i] == CR || u[i] == LF {
                i += 1;
                break;
            }
            let wl = words::get_word(&u[i..]);
            let word = &u[i..i + wl];
            let vw = self.units_width(word, Self::visible_len(word));
            if vw.wrapping_add(sum) as u32 > w as u32 {
                break;
            }
            sum = sum.wrapping_add(self.units_width(word, wl));
            i += wl;
        }
        if i == 0 {
            match mode {
                1 => {
                    while i < n && !matches!(u[i], 0 | LF | CR) && !words::is_space(u[i]) {
                        i += words::get_word(&u[i..]);
                    }
                }
                2 => {
                    let mut prev;
                    loop {
                        prev = i;
                        i += if words::is_thai(u[i]) {
                            words::thai_cluster_len(&u[i..], n - i)
                        } else {
                            1
                        };
                        // The original has no end bound here; it cannot be reached with
                        // non-negative advances (the first word is already wider than w). FN22
                        // tracks the core's safe bound when this caller precondition fails.
                        if i >= n || self.units_width(u, i) >= w {
                            break;
                        }
                    }
                    i = i.min(n);
                    if prev != 0 && self.units_width(u, i) >= w {
                        i = prev;
                    }
                    if i < n && c.substituted && c.src[i - 1] == c.src[i] {
                        i += 1;
                    }
                }
                _ => return None,
            }
        }
        if i > 1 && !words::is_space(u[i - 1]) && i < n && words::is_space(u[i]) {
            i += 1;
        }
        Some(i)
    }

    /// Slot 41, CalculateWordsToFitInWidth(s, n, w, &width, mode): (source length, width).
    pub fn words_to_fit(&mut self, s: &[u8], w: i32, mode: u32) -> (u32, i32) {
        if s.is_empty() || w == 0 {
            return (0, 0);
        }
        self.glyph_map.clear();
        let c = self.convert(s);
        if c.substituted {
            self.glyph_map = c.src.clone();
        }
        let Some(k) = self.words_count(&c, w, mode) else {
            return (0, 0);
        };
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
                // FN22: native trusts progress; preserve a bounded portable adapter for now.
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
        // Native division faults at lineHeight zero; preserve no-normal-return with a panic.
        let max_lines = (height as u32) / (self.get_line_height() as u32);
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
        // The published test is U+000A, not a byte 0x0A embedded in a multibyte unit.
        let has_nl = self.convert(s).units.contains(&10);
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
            let converted = self.convert(text);
            let keep = converted.units.iter().rposition(|&u| u != 0x20).map_or(0, |i| i + 1);
            text.truncate(converted.src_len_of(keep) as usize);
            text.extend_from_slice(ellipsis);
        } else {
            text.clear();
        }
        l
    }

    /// Slot 52, DrawTextParagraph. FN19: provisional `initialised` result, no pen contract yet.
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
            let converted = self.convert(line);
            let mut units = converted.units.len();
            while units != 0 && matches!(converted.units[units - 1], 13 | 10) {
                units -= 1;
            }
            line = &line[..converted.src_len_of(units) as usize];
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
