// SPDX-License-Identifier: GPL-3.0-or-later
//! `cGZFont` (drawing-fonts.md §2-§5, §7, §9, §10). Measuring and layout (§8) are in `layout.rs`.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use super::cache::{CachedBitmapData, GlyphRecord};
use super::encoding::{self, Converted, CP_UTF16};
use super::{
    aa, fontsys, CodePageConverter, DrawColour, FontFace, FontLibrary, FontRenderer, GlyphRaster, GreyLevels,
    SizeMetrics, SurfaceId,
};

/// Class ID of `cGZFont` (§1).
pub const CLSID_GZ_FONT: u32 = 0x1AA4_04AC;
/// Interface ID of `cIGZFont` (§1).
pub const IID_GZ_FONT: u32 = 0x1067_3960;
/// The "use the font's colour / default background" sentinel (§9).
pub const PEN_DEFAULT_COLOUR: u32 = 0x0800_0000;
/// User style bits (§2).
pub const USER_STYLES: u32 = 0x3C;
pub const STYLE_UNDERLINE: u32 = 0x04;
pub const STYLE_STRIKE: u32 = 0x08;

static NEXT_FONT_ID: AtomicUsize = AtomicUsize::new(1);

/// A 128-entry ARGB palette for its (fg, bg) pair (§9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    pub fg: u32,
    pub bg: u32,
    pub entries: Vec<u32>,
}

impl Palette {
    /// The palette builder (§9). The spec has the graphic system split and compose the colours
    /// (`RZ::GS()` +0x90/+0x8C) without naming the pixel format: channels are taken as A8R8G8B8
    /// (question FN12).
    pub fn build(fg: u32, bg: u32) -> Palette {
        let ch = |c: u32, s: u32| ((c >> s) & 0xFF) as i32;
        let mut entries = vec![bg; 128];
        for k in 0..15i32 {
            let mut c = 0u32;
            for s in [24u32, 16, 8, 0] {
                let b = ch(bg, s);
                let f = ch(fg, s);
                let v = b + (f - b) * k / 16;
                c |= ((v as u32) & 0xFF) << s;
            }
            for j in 0..8 {
                entries[(8 + 8 * k + j) as usize] = c;
            }
        }
        for e in entries.iter_mut().skip(120) {
            *e = fg;
        }
        Palette { fg, bg, entries }
    }
}

/// A draw record (0x24 bytes in the original, §9).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrawRecord {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    /// Byte offset of the glyph's top-left in the active cache's block.
    pub src_offset: usize,
    pub src_pitch: u32,
    pub width: i32,
    pub height: i32,
    pub ch: u32,
}

/// The font object state (§2).
pub struct GzFont {
    pub(crate) lib: Arc<dyn FontLibrary>,
    pub(crate) conv: Arc<dyn CodePageConverter>,
    pub(crate) id: usize,
    pub initialised: bool,
    pub file_path: String,
    pub family_name: String,
    pub face_kind: u32,
    pub point_size: u32,
    pub char_height: i32,
    pub style_flags: u32,
    pub fg: u32,
    pub bg: u32,
    pub aa_mode: u32,
    /// +0x50, read by DrawTextLine (§9); no slot writes it.
    pub field_50: u32,
    pub aa_gamma: f32,
    pub aa_sharpness: f32,
    pub aa_table: Vec<u8>,
    pub scale_x: f32,
    pub scale_y: f32,
    pub advance_scale: f32,
    pub char_spacing: i32,
    pub line_spacing: i32,
    pub code_page: u32,
    pub surface_lock_count: u32,
    pub ascent: i32,
    pub descent_raw: i32,
    pub(crate) renderer: Option<Arc<dyn FontRenderer>>,
    pub glyph_map: Vec<u32>,
    pub default_char: u32,
    pub default_glyph: u32,
    pub(crate) face: Option<Box<dyn FontFace>>,
    pub records: Vec<DrawRecord>,
    pub current_palette: Option<Palette>,
    pub palettes: Vec<Palette>,
    pub mono_cache: CachedBitmapData,
    pub grey_cache: CachedBitmapData,
    /// Font source type (§12) of filePath, read at Init (FN10: the spec uses it in §5/§7 without
    /// saying where the font gets it).
    pub source_type: u32,
    pub(crate) metrics: SizeMetrics,
}

impl GzFont {
    /// A freshly created font. The spec gives no constructor defaults (FN11): scales, advance scale
    /// and gamma start at 1.0 (the "no change" values of §4/§10), everything else at zero.
    pub fn new(lib: Arc<dyn FontLibrary>, conv: Arc<dyn CodePageConverter>) -> GzFont {
        GzFont {
            lib,
            conv,
            id: NEXT_FONT_ID.fetch_add(1, Ordering::Relaxed),
            initialised: false,
            file_path: String::new(),
            family_name: String::new(),
            face_kind: 0,
            point_size: 0,
            char_height: 0,
            style_flags: 0,
            fg: 0,
            bg: 0,
            aa_mode: 0,
            field_50: 0,
            aa_gamma: 1.0,
            aa_sharpness: 0.0,
            aa_table: Vec::new(),
            scale_x: 1.0,
            scale_y: 1.0,
            advance_scale: 1.0,
            char_spacing: 0,
            line_spacing: 0,
            code_page: 0,
            surface_lock_count: 0,
            ascent: 0,
            descent_raw: 0,
            renderer: None,
            glyph_map: Vec::new(),
            default_char: 0,
            default_glyph: 0,
            face: None,
            records: Vec::new(),
            current_palette: None,
            palettes: Vec::new(),
            mono_cache: CachedBitmapData::default(),
            grey_cache: CachedBitmapData::default(),
            source_type: 0,
            metrics: SizeMetrics::default(),
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    // ---- §3 lifetime -------------------------------------------------------------------------

    /// Slot 3, Init (`0x0089955C`).
    pub fn init(&mut self) -> bool {
        if self.initialised || self.file_path.is_empty() {
            return self.initialised;
        }
        if !self.lib.file_exists(&self.file_path) {
            return false;
        }
        let Some(face) = self.lib.open(&self.file_path) else {
            return false;
        };
        self.source_type = fontsys::source_type_from_file_name(&*self.lib, &self.file_path);
        self.face = Some(face);
        if self.point_size != 0 && !self.set_size(self.point_size) {
            return false;
        }
        let face = self.face.as_ref().expect("face");
        self.family_name = face.family_name();
        self.style_flags = face.style_flags() | (self.style_flags & USER_STYLES);
        self.face_kind = 1 + u32::from(face.is_fixed_size());
        self.default_glyph = face.char_index(self.default_char).unwrap_or(0);
        self.initialised = !self.family_name.is_empty();
        if self.initialised {
            if self.renderer.is_none() {
                if let Some(r) = self.lib.default_renderer() {
                    self.set_renderer(Some(r));
                }
            }
            self.rebuild_aa_table();
        }
        self.initialised
    }

    /// Slot 4, Shutdown (`0x008982E1`).
    pub fn shutdown(&mut self) -> bool {
        if self.initialised {
            self.set_renderer(None);
            self.mono_cache.clear();
            self.grey_cache.clear();
            self.current_palette = None;
            self.palettes.clear();
            self.face = None;
            self.initialised = false;
        }
        true
    }

    // ---- §4 attributes -----------------------------------------------------------------------

    /// Slot 0, QueryInterface: id `0x10673960` or 1 answer (the caller AddRefs, see [`FontObj`]).
    pub fn answers_interface(id: u32) -> bool {
        id == IID_GZ_FONT || id == 1
    }
    pub fn get_style(&self, mask: u32) -> bool {
        (self.style_flags & mask) == mask
    }
    pub fn set_style(&mut self, mask: u32, on: bool) -> bool {
        let m = mask & USER_STYLES;
        if on {
            self.style_flags |= m;
        } else {
            self.style_flags &= !m;
        }
        true
    }
    pub fn set_font_color(&mut self, fg: u32, bg: u32) -> bool {
        self.fg = fg;
        self.bg = bg;
        true
    }
    pub fn set_antialiasing(&mut self, mode: u32) -> bool {
        if mode != 0 {
            self.aa_mode = mode;
        }
        true
    }
    pub fn set_aa_gamma(&mut self, g: f32) -> bool {
        let g = g.clamp(0.0, 10.0);
        if g != self.aa_gamma {
            self.aa_gamma = g;
            self.rebuild_aa_table();
        }
        true
    }
    pub fn set_aa_sharpness(&mut self, s: f32) -> bool {
        let s = s.clamp(-1.0, 1.0);
        if s != self.aa_sharpness {
            self.aa_sharpness = s;
            self.rebuild_aa_table();
        }
        true
    }
    pub fn set_file_path(&mut self, p: &str) -> bool {
        self.file_path = p.to_owned();
        true
    }
    pub fn set_code_page(&mut self, cp: u32) -> bool {
        self.code_page = cp;
        true
    }
    pub fn is_char_encoding_available(e: u32) -> bool {
        e == 2
    }
    pub fn get_scaling(&self) -> (f32, f32) {
        (self.scale_x, self.scale_y)
    }
    pub fn set_scaling(&mut self, x: f32, y: f32) -> bool {
        let x = x.clamp(0.01, 1000.0);
        let y = y.clamp(0.01, 1000.0);
        if x != self.scale_x || y != self.scale_y {
            self.scale_x = x;
            self.scale_y = y;
            self.mono_cache.clear();
            self.grey_cache.clear();
            // Re-apply the char size only; metrics are not recomputed (literal §4).
            let (size, sx, sy) = (self.point_size, x, y);
            if let Some(f) = self.face.as_mut() {
                let _ = f.set_char_size(size, 72, 72, sx, sy);
            }
        }
        true
    }
    pub fn get_advance_scaling(&self) -> (f32, f32) {
        (self.advance_scale, 1.0)
    }
    pub fn set_advance_scaling(&mut self, s: f32) -> bool {
        let s = s.clamp(0.1, 10.0);
        if s != self.advance_scale {
            self.advance_scale = s;
            self.mono_cache.clear();
            self.grey_cache.clear();
        }
        true
    }
    pub fn set_spacing_adjustment(&mut self, ch: i32, line: i32) -> bool {
        self.char_spacing = ch;
        self.line_spacing = line;
        true
    }

    /// Slot 25, IsCodePageAvailable (§6).
    pub fn is_code_page_available(&self, cp: u32) -> bool {
        let probes = encoding::code_page_probes(cp);
        if probes.is_empty() {
            return true;
        }
        match self.face.as_ref() {
            Some(f) => probes.iter().all(|&p| f.has_glyph(p)),
            // The original dereferences the face handle; without a face nothing is present.
            None => false,
        }
    }

    /// Slot 27, IsCharAvailable(s, n).
    pub fn is_char_available(&mut self, s: &[u8]) -> bool {
        if s.is_empty() {
            return true;
        }
        let conv = self.convert(s);
        let saved = self.code_page;
        self.code_page = CP_UTF16;
        let r = match self.face.as_ref() {
            Some(f) => conv.units.iter().all(|&u| f.has_glyph(u)),
            None => false,
        };
        self.code_page = saved;
        r
    }

    /// Slot 28, PreloadChars(s, n).
    pub fn preload_chars(&mut self, s: &[u8]) -> bool {
        if self.initialised && !s.is_empty() {
            let conv = self.convert(s);
            for &u in &conv.units {
                self.get_char_data(u as u32);
            }
        }
        self.initialised
    }

    pub(crate) fn convert(&self, s: &[u8]) -> Converted {
        encoding::text_as_unicode(&*self.conv, self.code_page, s)
    }

    // ---- §5 size and metrics -----------------------------------------------------------------

    /// Slot 11, SetSize (`0x00897AB1`).
    pub fn set_size(&mut self, size: u32) -> bool {
        if size == 0 || self.mono_cache.glyph_count() != 0 || self.grey_cache.glyph_count() != 0 {
            return false;
        }
        let size = size.min(2048);
        self.point_size = size;
        let (sx, sy) = (self.scale_x, self.scale_y);
        let Some(face) = self.face.as_mut() else {
            return true;
        };
        match face.set_char_size(size, 72, 72, sx, sy) {
            Some(m) => {
                self.metrics = m;
                self.descent_raw = m.a / 65536;
                self.ascent = m.a.wrapping_add(m.b) / 65536;
                let mut ch = m.a.wrapping_sub(m.c).wrapping_add(m.b) / 65536;
                if ch == 0 {
                    ch = 1;
                }
                if self.source_type == 2 || self.source_type == 3 {
                    ch += 1;
                    if ch > 28 {
                        ch += 1;
                    }
                }
                self.char_height = ch;
                true
            }
            None => false,
        }
    }
    pub fn get_line_height(&self) -> i32 {
        self.char_height + self.line_spacing
    }
    pub fn char_height(&self) -> i32 {
        self.char_height
    }
    pub fn size_above_baseline(&self) -> i32 {
        self.ascent + self.line_spacing
    }
    pub fn size_below_baseline(&self) -> i32 {
        self.get_line_height() - self.size_above_baseline()
    }

    // ---- §10 -------------------------------------------------------------------------------

    pub(crate) fn rebuild_aa_table(&mut self) {
        self.aa_table = aa::build_table(self.aa_gamma, self.aa_sharpness);
        if !self.aa_table.is_empty() {
            self.grey_cache.clear();
        }
    }

    // ---- §7 glyph cache ----------------------------------------------------------------------

    /// Slot 59.
    pub fn appropriate_cache(&self) -> &CachedBitmapData {
        if self.aa_mode == 1 {
            &self.mono_cache
        } else {
            &self.grey_cache
        }
    }

    /// Slot 60, GetCharData(ch): record or `None`. Without an open face nothing can be added.
    pub fn get_char_data(&mut self, ch: u32) -> Option<GlyphRecord> {
        if let Some(r) = self.appropriate_cache().glyphs.get(&ch) {
            return Some(*r);
        }
        self.add_char_data(ch)
    }

    /// Slot 61, AddCharData (`0x00898FDC`) into the active cache.
    pub fn add_char_data(&mut self, ch: u32) -> Option<GlyphRecord> {
        let st23 = self.source_type == 2 || self.source_type == 3;
        let levels = if self.aa_mode == 1 {
            GreyLevels::Two
        } else {
            GreyLevels::Levels256
        };
        let default_glyph = self.default_glyph;
        let face = self.face.as_mut()?;
        let mut idx = if ch < 0x80 || !st23 {
            face.char_index(ch).unwrap_or(0)
        } else {
            0
        };
        if idx == 0 && st23 && ch >= 0x20 {
            idx = face.fallback_char_index(ch);
        }
        if idx == 0 && default_glyph != 0 {
            idx = default_glyph;
        }
        let zero_width = ch == 0x200B || ch == 0xFEFF;
        let loaded = face.load_glyph(idx, levels);
        let m = self.metrics;
        let (raster, x_off, y_off, adv, bits) = match loaded {
            Some(g) => {
                let x_off = g.bearing_x / 64;
                let y_off = m.a.wrapping_add(m.b) / 65536 - g.bearing_y / 64;
                let adv = if zero_width {
                    0
                } else if self.advance_scale == 1.0 {
                    g.advance / 65536
                } else {
                    // The spec gives the advance only for advanceScale == 1.0 (FN7): scaled and
                    // truncated here.
                    (g.advance as f64 / 65536.0 * self.advance_scale as f64) as i32
                };
                let bits = levels == GreyLevels::Two;
                (g, x_off, y_off, adv, bits)
            }
            None => {
                // An empty box of width charHeight/2 (FN7: box height and placement are not given;
                // charHeight rows at the line top here), drawn as a filled block; U+200B/U+FEFF an
                // empty box with advance 0.
                let w = (self.char_height / 2).max(0) as u32;
                let rows = self.char_height.max(0) as u32;
                let fill = if zero_width { 0 } else { 0xFF };
                let g = GlyphRaster {
                    width: w,
                    rows,
                    pitch: w,
                    buffer: vec![fill; (w * rows) as usize],
                    ..GlyphRaster::default()
                };
                (g, 0, 0, if zero_width { 0 } else { w as i32 }, false)
            }
        };
        let rec = self.copy_glyph_to_source_data(&raster, bits, x_off, y_off, adv);
        let cache = if self.aa_mode == 1 {
            &mut self.mono_cache
        } else {
            &mut self.grey_cache
        };
        cache.glyphs.insert(ch, rec);
        Some(rec)
    }

    /// Slot 62, CopyGlyphToSourceData (`0x00897748`) into the active cache.
    pub fn copy_glyph_to_source_data(
        &mut self,
        g: &GlyphRaster,
        packed_bits: bool,
        mut x_off: i32,
        y_off: i32,
        advance: i32,
    ) -> GlyphRecord {
        let lines = self.style_flags & (STYLE_UNDERLINE | STYLE_STRIKE);
        let mut cell_w = g.width as i32;
        let mut dx = 0i32;
        if lines != 0 {
            // Fold a positive xOffset into the cell (literal: the raster moves right by it and the
            // cell is widened to max(raster extent, advance)).
            if x_off > 0 {
                dx = x_off;
                x_off = 0;
            }
            cell_w = (g.width as i32 + dx).max(advance);
        }
        let cell_w = cell_w.max(0) as u32;
        // Line rows (relative to the glyph top), clipped to charHeight.
        let thick = self.metrics.line_thickness.max(0);
        let mut line_rows: Vec<i32> = Vec::new();
        let mut push_rows = |start: i32| {
            for r in start..start + thick {
                if r >= 0 && r + y_off < self.char_height {
                    line_rows.push(r);
                }
            }
        };
        if lines & STYLE_UNDERLINE != 0 {
            push_rows(self.ascent - y_off + self.metrics.underline_offset);
        }
        if lines & STYLE_STRIKE != 0 {
            push_rows(self.ascent - self.char_height / 4 - y_off);
        }
        let mut rows = g.rows;
        if let Some(&m) = line_rows.iter().max() {
            rows = rows.max(m as u32 + 1);
        }
        let mono = self.aa_mode == 1;
        let table = if !mono && !self.aa_table.is_empty() {
            Some(self.aa_table.clone())
        } else {
            None
        };
        let cache = if mono {
            &mut self.mono_cache
        } else {
            &mut self.grey_cache
        };
        let x = cache.pen_x;
        if x + cell_w > cache.pitch || rows > cache.height || cache.block.is_empty() {
            let new_w = if x + cell_w > cache.pitch {
                cell_w * 32 + x
            } else {
                cache.pitch
            };
            cache.reallocate(new_w.max(1), rows);
        }
        for y in 0..g.rows {
            for col in 0..g.width {
                let v = if packed_bits {
                    let byte = g.buffer.get((y * g.pitch + col / 8) as usize).copied().unwrap_or(0);
                    if byte & (0x80 >> (col % 8)) != 0 {
                        0xFF
                    } else {
                        0x00
                    }
                } else {
                    g.buffer.get((y * g.pitch + col) as usize).copied().unwrap_or(0)
                };
                let v = match &table {
                    Some(t) => t[v as usize],
                    None => v,
                };
                let o = cache.offset(x + col + dx as u32, y);
                cache.block[o] = v;
            }
        }
        for &r in &line_rows {
            for col in 0..cell_w {
                let o = cache.offset(x + col, r as u32);
                cache.block[o] = 0xFF;
            }
        }
        cache.pen_x = x + cell_w;
        GlyphRecord {
            src_left: x as i32,
            src_top: 0,
            src_right: (x + cell_w) as i32,
            src_bottom: rows as i32,
            x_offset: x_off as i16,
            y_offset: y_off as i16,
            advance: advance as i16,
        }
    }

    // ---- §9 drawing --------------------------------------------------------------------------

    pub fn get_renderer(&self) -> Option<Arc<dyn FontRenderer>> {
        self.renderer.clone()
    }

    /// Slot 54, SetRenderer.
    pub fn set_renderer(&mut self, r: Option<Arc<dyn FontRenderer>>) -> bool {
        let same = match (&self.renderer, &r) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !same {
            if let Some(n) = &r {
                n.attach(self.id);
            }
            if let Some(old) = self.renderer.take() {
                old.detach(self.id);
            }
            self.renderer = r;
        }
        true
    }

    /// Slot 55.
    pub fn lock_surface(&mut self, target: SurfaceId) -> bool {
        if self.surface_lock_count == 0 {
            let ok = self.renderer.as_ref().is_some_and(|r| r.lock_surface(target));
            if !ok {
                return false;
            }
        }
        self.surface_lock_count += 1;
        true
    }

    /// Slot 56.
    pub fn unlock_surface(&mut self, target: SurfaceId) -> bool {
        if self.surface_lock_count == 0 {
            return false;
        }
        self.surface_lock_count -= 1;
        if self.surface_lock_count == 0 {
            if let Some(r) = &self.renderer {
                r.unlock_surface(target);
            }
        }
        true
    }

    /// `SetCurrentPalette(fg, bg)` (`0x0089838C`).
    pub fn set_current_palette(&mut self, fg: u32, bg: u32) {
        if let Some(p) = &self.current_palette {
            if p.fg == fg && p.bg == bg {
                return;
            }
        }
        if let Some(i) = self.palettes.iter().position(|p| p.fg == fg && p.bg == bg) {
            let p = self.palettes.remove(i);
            self.palettes.insert(0, p);
        } else {
            if self.palettes.len() > 3 {
                self.palettes.pop();
            }
            self.palettes.insert(0, Palette::build(fg, bg));
        }
        self.current_palette = Some(self.palettes[0].clone());
    }

    /// Slot 51, DrawTextLine. `pen` overrides (x, y) and receives the final pen.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text_line(
        &mut self,
        target: Option<SurfaceId>,
        x: i32,
        y: i32,
        text: &[u8],
        fg: u32,
        bg: u32,
        clip: Option<(i32, i32, i32, i32)>,
        pen: Option<&mut (i32, i32)>,
    ) -> bool {
        let conv = self.convert(text);
        let mut fg = fg;
        let mut bg = bg;
        if self.aa_mode == 2 {
            if bg == PEN_DEFAULT_COLOUR && self.field_50 == 2 {
                bg = if fg != 0 { 0 } else { 0xFFFF_FFFF };
            }
            self.set_current_palette(fg, bg);
        }
        if fg == PEN_DEFAULT_COLOUR {
            fg = self.fg;
        }
        let _bg = bg & 0x00FF_FFFF;
        let mut locked_here = false;
        if let Some(t) = target {
            if self.surface_lock_count == 0 {
                if !self.lock_surface(t) {
                    return false;
                }
                locked_here = true;
            }
        }
        let (mut px, py) = match &pen {
            Some(p) => (p.0, p.1),
            None => (x, y),
        };
        for &u in &conv.units {
            self.get_char_data(u as u32);
        }
        self.records.clear();
        for &u in &conv.units {
            let (ch, g) = match self.get_char_data(u as u32) {
                Some(g) => (u as u32, g),
                None => match self.get_char_data(self.default_char) {
                    Some(g) => (self.default_char, g),
                    None => continue,
                },
            };
            let left = px + g.x_offset as i32;
            let top = py + g.y_offset as i32 + self.line_spacing;
            let (w, h) = (g.width(), g.height());
            let (right, bottom) = (left + w, top + h);
            let visible = match clip {
                None => true,
                Some((cl, ct, cr, cb)) => left < cr && right > cl && top < cb && bottom > ct,
            };
            if visible && w != 0 && h != 0 {
                let cache = self.appropriate_cache();
                let (src_offset, src_pitch) = (cache.offset(g.src_left as u32, g.src_top as u32), cache.pitch);
                self.records.push(DrawRecord {
                    left,
                    top,
                    right,
                    bottom,
                    src_offset,
                    src_pitch,
                    width: w,
                    height: h,
                    ch,
                });
            }
            px += g.advance as i32 + self.char_spacing;
        }
        if let Some(t) = target {
            if !self.records.is_empty() {
                if let Some(r) = &self.renderer {
                    let colour = if self.aa_mode == 2 {
                        DrawColour::Palette(self.current_palette.clone().unwrap_or_else(|| Palette::build(fg, bg)))
                    } else {
                        DrawColour::Colour(fg)
                    };
                    r.draw(t, &self.records, &self.appropriate_cache().block, &colour);
                }
            }
        }
        if let Some(p) = pen {
            *p = (px, py);
        }
        if locked_here {
            if let Some(t) = target {
                self.unlock_surface(t);
            }
        }
        self.initialised
    }
}

/// A reference-counted font (the COM object around [`GzFont`]). No RefCell: the state sits behind a
/// mutex so getters may run from another thread.
pub struct FontObj {
    refs: AtomicU32,
    font: Mutex<GzFont>,
}

pub type FontRef = Arc<FontObj>;

impl FontObj {
    /// A new object with one reference (the creator's).
    pub fn new(font: GzFont) -> FontRef {
        Arc::new(FontObj {
            refs: AtomicU32::new(1),
            font: Mutex::new(font),
        })
    }
    pub fn lock(&self) -> MutexGuard<'_, GzFont> {
        self.font.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn ref_count(&self) -> u32 {
        self.refs.load(Ordering::SeqCst)
    }
    /// Slot 1.
    pub fn add_ref(&self) -> u32 {
        self.refs.fetch_add(1, Ordering::SeqCst) + 1
    }
    /// Slot 2, Release: refCount > 1 → decrement and return it; otherwise FinalRelease (Shutdown,
    /// destroy) and 0.
    pub fn release(&self) -> u32 {
        let r = self.refs.load(Ordering::SeqCst);
        if r > 1 {
            self.refs.fetch_sub(1, Ordering::SeqCst);
            return r - 1;
        }
        self.refs.store(0, Ordering::SeqCst);
        let mut f = self.lock();
        f.shutdown();
        f.records.clear();
        f.glyph_map.clear();
        f.aa_table.clear();
        0
    }
    /// Slot 0, QueryInterface: AddRef and true for `0x10673960` or 1.
    pub fn query_interface(&self, id: u32) -> bool {
        if GzFont::answers_interface(id) {
            self.add_ref();
            true
        } else {
            false
        }
    }
}
