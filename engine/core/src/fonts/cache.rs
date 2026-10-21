// SPDX-License-Identifier: GPL-3.0-or-later
//! Glyph cache (`cGZFont::CachedBitmapData`, drawing-fonts.md §7).

use std::collections::HashMap;

/// Glyph record (§7): a rectangle in the cache's 8-bit source bitmap plus placement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlyphRecord {
    pub src_left: i32,
    pub src_top: i32,
    pub src_right: i32,
    pub src_bottom: i32,
    pub x_offset: i16,
    /// From the line top.
    pub y_offset: i16,
    pub advance: i16,
}

impl GlyphRecord {
    pub fn width(&self) -> i32 {
        self.src_right - self.src_left
    }
    pub fn height(&self) -> i32 {
        self.src_bottom - self.src_top
    }
}

/// One glyph cache: hash map of glyph records and the source bitmap they index.
#[derive(Clone, Debug, Default)]
pub struct CachedBitmapData {
    pub glyphs: HashMap<u32, GlyphRecord>,
    /// Whole block, `(height + 4) · pitch` bytes; row 0 starts at `2 · pitch`.
    pub block: Vec<u8>,
    pub pitch: u32,
    pub height: u32,
    /// Next free column.
    pub pen_x: u32,
}

impl CachedBitmapData {
    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    /// `CachedBitmapData::Clear` (`0x008982BA`): empties the map and the bitmap.
    pub fn clear(&mut self) {
        self.glyphs.clear();
        self.block.clear();
        self.pitch = 0;
        self.height = 0;
        self.pen_x = 0;
    }

    /// Byte offset of pixel (x, y) in `block`.
    pub fn offset(&self, x: u32, y: u32) -> usize {
        (2 * self.pitch + y * self.pitch + x) as usize
    }

    /// `ReallocateSourceData(cache, w, h)` (`0x0089632D`): only grows; new block `(h + 4)·w` bytes,
    /// zeroed, data at `2·w`, old rows copied, pitch = w.
    pub fn reallocate(&mut self, w: u32, h: u32) {
        let w = w.max(self.pitch);
        let h = h.max(self.height);
        if w == self.pitch && h == self.height && !self.block.is_empty() {
            return;
        }
        let mut block = vec![0u8; ((h + 4) * w) as usize];
        for y in 0..self.height {
            let from = (2 * self.pitch + y * self.pitch) as usize;
            let to = (2 * w + y * w) as usize;
            block[to..to + self.pitch as usize].copy_from_slice(&self.block[from..from + self.pitch as usize]);
        }
        self.block = block;
        self.pitch = w;
        self.height = h;
    }
}
