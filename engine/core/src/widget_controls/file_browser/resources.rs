// SPDX-License-Identifier: GPL-3.0-or-later
//! Resource result/lock channels and the exact arrow recipe (§8.3).
use super::*;

impl FileBrowser {
    fn allocate_buffer(width: i32, height: i32, output: &mut Option<Handle>, f: &mut impl IconIo) -> Result<bool> {
        if output.is_some() {
            return Err(Unsupported::Precondition("initially null buffer output"));
        }
        *output = f
            .factory_buffer(0xc470_d325, 0x2073_2180)
            .or_else(|| f.graphics_buffer());
        let Some(buffer) = *output else {
            return Ok(false);
        };
        if !f.init_buffer(buffer, width, height, [3, 16]) {
            f.release(buffer);
            *output = None;
            return Ok(false);
        }
        Ok(true)
    }

    pub fn create_combo_arrow(&self, output: &mut Option<Handle>, f: &mut impl IconIo) -> Result<bool> {
        if !Self::allocate_buffer(64, 15, output, f)? {
            return Ok(false);
        }
        let buffer = output.expect("allocated buffer");
        let black = f.convert_rgb(self.window(), [0, 0, 0]);
        let magenta = f.convert_rgb(self.window(), [255, 0, 255]);
        if f.lock_buffer(buffer, 0x8001) {
            f.fill_buffer(buffer, [0, 0, 64, 15], magenta);
            f.unlock_buffer(buffer);
        }
        f.set_color_key(buffer, magenta);
        for x in [4, 20, 36, 52] {
            if f.lock_buffer(buffer, 0x8004) {
                for step in 0..4 {
                    f.line_buffer(buffer, [x + step, 5 + step], [x + 7 - step, 5 + step], black, 1);
                }
                f.unlock_buffer(buffer);
            }
        }
        Ok(true)
    }

    pub fn draw_default_icon(
        &self,
        selector: u32,
        buffer: Option<Handle>,
        area: Rect,
        f: &mut impl IconIo,
    ) -> Result<bool> {
        let Some(buffer) = buffer else {
            return Ok(true);
        };
        let mut colors = Vec::new();
        for rgb in [
            [255, 0, 255],
            [0, 0, 0],
            [255, 248, 144],
            [191, 184, 191],
            [127, 120, 127],
            [112, 112, 112],
            [175, 176, 175],
            [255, 255, 255],
        ] {
            colors.push(f.convert_rgb(self.window(), rgb));
        }
        if f.lock_buffer(buffer, 0x8001) {
            f.fill_buffer(buffer, area, colors[0]);
            f.unlock_buffer(buffer);
        }
        f.set_color_key(buffer, colors[0]);
        if selector <= 6 && f.lock_buffer(buffer, 0x8004) {
            f.unlock_buffer(buffer);
            // SPEC-GAP: §8.3.3 requires an ordered primitive trace for icon glyphs.
            return Err(Unsupported::Open("FBQ1 icon primitive trace, §8.3.3"));
        }
        Ok(true)
    }

    pub fn create_button_buffer(
        &self,
        selector: u32,
        output: &mut Option<Handle>,
        f: &mut impl IconIo,
    ) -> Result<bool> {
        if !Self::allocate_buffer(84, 22, output, f)? {
            return Ok(false);
        }
        let buffer = output.expect("allocated buffer");
        let mut colors = Vec::new();
        for rgb in [
            [125, 125, 125],
            [170, 170, 170],
            [255, 0, 255],
            [255, 255, 255],
            [100, 100, 100],
        ] {
            colors.push(f.convert_rgb(self.window(), rgb));
        }
        if f.lock_buffer(buffer, 0x8001) {
            f.fill_buffer(buffer, [0, 0, 84, 22], colors[2]);
            f.unlock_buffer(buffer);
        }
        f.set_color_key(buffer, colors[2]);
        let common = f.lock_buffer(buffer, 0x8004);
        if common {
            f.unlock_buffer(buffer);
        }
        if common || matches!(selector, 2 | 3) {
            // SPEC-GAP: §8.3.3 requires common-border and selector-glyph traces.
            return Err(Unsupported::Open("FBQ1 button primitive trace, §8.3.3"));
        }
        Ok(true)
    }
}
