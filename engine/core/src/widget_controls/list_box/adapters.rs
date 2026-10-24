// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl ListBox {
    pub(super) fn forward_bar(&self, x: i32, y: i32) -> Option<Handle> {
        self.bar.filter(|_| {
            self.bar_tracking
                || (x >= 0 && x < self.width() && y >= 0 && y < self.height() && x >= self.width() - self.bar_width)
        })
    }
    pub fn set_drop_policies(&mut self, height: bool, width: bool) -> bool {
        self.drop_resize = [height, width];
        true
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
        self.delivery = if byte == 0 { Delivery::Send } else { Delivery::Post };
        true
    }
    pub fn false_stub(&self, _a: u32, _b: u32) -> bool {
        false
    }
}
