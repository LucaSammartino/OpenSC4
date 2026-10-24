// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl ListBox {
    pub fn set_flag(&mut self, mask: u32, on: u8, f: &mut impl Foreign) -> Result<u32> {
        // SPEC-GAP: W68's extra owner policy is unspecified; delegate to the base window.
        f.base_set_flag(self.window, mask, on)
    }
    /// W155 uses a complete case-sensitive byte match and preserves the output
    /// on failure, unlike L50's caller-supplied comparison.
    pub fn find_exact(&self, text: &[u8], output: &mut i32) -> bool {
        let Some(index) = self.entries.iter().position(|entry| entry.text == text) else {
            return false;
        };
        *output = index as i32;
        true
    }
    pub(super) fn forward_bar(&self, x: i32, y: i32) -> Option<Handle> {
        self.bar.filter(|_| {
            self.bar_tracking
                || (x >= 0 && x < self.width() && y >= 0 && y < self.height() && x >= self.width() - self.bar_width)
        })
    }
    pub fn set_drop_policies(&mut self, height: bool, width: bool) -> bool {
        self.set_drop_policy_bytes([u8::from(height), u8::from(width)])
    }
    /// L17/18 and L35/36 preserve the stored bytes, including noncanonical true.
    pub fn set_drop_byte(&mut self, byte: u8) {
        self.raw_bytes[0] = byte;
        self.drop_mode = byte != 0;
    }
    pub fn drop_byte(&self) -> u8 {
        Self::stored_byte(self.raw_bytes[0], self.drop_mode)
    }
    pub fn set_drop_policy_bytes(&mut self, bytes: [u8; 2]) -> bool {
        self.raw_bytes[1..3].copy_from_slice(&bytes);
        self.drop_resize = bytes.map(|byte| byte != 0);
        true
    }
    pub fn drop_policy_bytes(&self) -> [u8; 2] {
        [
            Self::stored_byte(self.raw_bytes[1], self.drop_resize[0]),
            Self::stored_byte(self.raw_bytes[2], self.drop_resize[1]),
        ]
    }
    fn stored_byte(raw: u8, enabled: bool) -> u8 {
        if (raw != 0) == enabled {
            raw
        } else {
            u8::from(enabled)
        }
    }
    pub fn reset_font_color(&mut self, index: usize, f: &mut impl Foreign) -> bool {
        self.set_font_color(index, SENTINEL, f)
    }
    pub fn set_selection_color(&mut self, color: u32, f: &mut impl Foreign) {
        self.selection_color = f.color_to_storage(self.window, color);
        f.invalidate(self.window);
    }
    pub fn selection_color(&self, output: &mut u32, f: &mut impl Foreign) -> bool {
        if self.selection_color == SENTINEL {
            return false;
        }
        *output = f.color_from_storage(self.window, self.selection_color);
        true
    }
    pub fn set_outline_colors(&mut self, colors: [u32; 2], f: &mut impl Foreign) {
        self.outline_colors = colors.map(|c| f.color_to_storage(self.window, c));
        f.invalidate(self.window);
    }
    pub fn outline_colors(&self, output: &mut [u32; 2], f: &mut impl Foreign) -> bool {
        if self.outline_colors.contains(&SENTINEL) {
            return false;
        }
        *output = self.outline_colors.map(|c| f.color_from_storage(self.window, c));
        true
    }
    pub fn set_bar_gaps(&mut self, x: u8, y: u8, f: &mut impl Foreign) -> bool {
        self.bar_gap = i32::from(x as i8);
        self.bar_gap_y = y as i8;
        self.recompute_metrics(f);
        f.invalidate(self.window);
        true
    }
    pub fn set_text_gutters(&mut self, x: u8, y: u8, f: &mut impl Foreign) -> bool {
        self.x_gutter = x as i8;
        self.y_gutter = y as i8;
        self.recompute_metrics(f);
        f.invalidate(self.window);
        true
    }
    pub fn set_minimum_height(&mut self, height: u8, f: &mut impl Foreign) -> bool {
        self.minimum_height = height;
        self.recompute_metrics(f);
        f.invalidate(self.window);
        true
    }
    pub fn set_bar_skin(&mut self, buffer: Option<Handle>, rect: Rect, f: &mut impl Foreign) -> bool {
        replace_ref(&mut self.services[0], buffer, f);
        self.skin_rect = rect;
        if let (Some(bar), Some(buffer)) = (self.bar, buffer) {
            let skin = if rect[2] <= rect[0] || rect[3] <= rect[1] {
                [0, 0, f.buffer_width(buffer), f.buffer_height(buffer)]
            } else {
                rect
            };
            f.bar_skin(bar, buffer, skin);
        }
        true
    }
    pub fn copy_text(&self, index: i32, output: &mut Vec<u8>) -> bool {
        // W154 is a lookup, unlike the clamping text-replacement adapter.
        let Some(entry) = usize::try_from(index).ok().and_then(|i| self.entries.get(i)) else {
            return false;
        };
        output.clone_from(&entry.text);
        true
    }
    pub fn search_with(&self, output: &mut i32, mut compare: impl FnMut(&[u8], u32) -> bool) -> bool {
        for (index, entry) in self.entries.iter().enumerate() {
            if compare(&entry.text, 1) {
                *output = index as i32;
                return true;
            }
        }
        false
    }
    pub fn set_delivery(&mut self, byte: u8) -> bool {
        self.raw_bytes[3] = byte;
        self.delivery = if byte == 0 { Delivery::Send } else { Delivery::Post };
        true
    }
    pub fn delivery_byte(&self) -> u8 {
        Self::stored_byte(self.raw_bytes[3], self.delivery == Delivery::Post)
    }
    pub fn false_stub(&self, _a: u32, _b: u32) -> bool {
        false
    }
}
