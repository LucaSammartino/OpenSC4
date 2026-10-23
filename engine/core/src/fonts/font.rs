// SPDX-License-Identifier: GPL-3.0-or-later
//! `cGZFont` (drawing-fonts.md §2-§5, §7, §9, §10). Measuring and layout (§8) are in `layout.rs`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use super::cache::{CachedBitmapData, GlyphRecord};
use super::encoding::{self, Converted, CP_UTF16};
use super::{
    aa, advance, names, CodePageConverter, DrawColour, FontFace, FontLibrary, FontRenderer, FontUse, GlyphRaster,
    GreyLevels, ScreenFormat, SizeMetrics, SurfaceId,
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

/// The font-file content probe (`0x008A1BD8`, §5 FN10): an sfnt header (00 01 00 00, 00 02 00 00
/// or "OTTO") with a `DSIG` table → 4, with a `loca` or `sloc` table → 1; a file starting "%!" → 2;
/// else 0. The table directory is the sfnt one (u16 big-endian table count at byte 4, 16-byte
/// records from byte 12, tag first). An sfnt with neither table falls through to the "%!" test.
pub fn probe_source_type(lib: &dyn FontLibrary, path: &str) -> u32 {
    let head = lib.read_head(path, 12);
    let sfnt = head.len() >= 12 && (head[..4] == [0, 1, 0, 0] || head[..4] == [0, 2, 0, 0] || &head[..4] == b"OTTO");
    if sfnt {
        let n = u16::from_be_bytes([head[4], head[5]]) as usize;
        let dir = lib.read_head(path, 12 + 16 * n);
        let tags: Vec<&[u8]> = (0..n).filter_map(|i| dir.get(12 + 16 * i..16 + 16 * i)).collect();
        if tags.iter().any(|t| *t == b"DSIG") {
            return 4;
        }
        if tags.iter().any(|t| *t == b"loca" || *t == b"sloc") {
            return 1;
        }
    }
    if head.starts_with(b"%!") {
        2
    } else {
        0
    }
}

/// Underline and strike-through thickness from pointSize only (§7 FN8, `0x00896618`/`0x00896636`).
pub fn line_thickness(point_size: u32) -> (i32, i32) {
    let u = match point_size {
        0..=15 => 1,
        16..=25 => 2,
        26..=35 => 3,
        _ => 4,
    };
    let s = match point_size {
        0..=23 => 1,
        24..=47 => 2,
        48..=71 => 3,
        _ => 4,
    };
    (u, s)
}

static NEXT_FONT_ID: AtomicUsize = AtomicUsize::new(1);

/// A 128-entry ARGB palette for its (fg, bg) pair (§9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    pub fg: u32,
    pub bg: u32,
    pub entries: Vec<u32>,
}

impl Palette {
    /// The palette builder of `SetCurrentPalette` (`0x0089838C`, §9 FN12): 0..7 = bg raw; for
    /// k = 0..14 eight entries of compose(B + (F − B)·k / 16) per channel, F/B = split of the low
    /// 16 bits of fg/bg (signed division toward zero); then 120..127 = fg raw, overwriting k = 14.
    pub fn build(fg: u32, bg: u32, screen: ScreenFormat) -> Palette {
        let mut entries = vec![bg; 128];
        let fs = split_colour(screen, fg & 0xFFFF);
        let bs = split_colour(screen, bg & 0xFFFF);
        for k in 0..15i32 {
            let c = match (fs, bs) {
                (Some(f), Some(b)) => {
                    let ch = |i: usize| (b[i] as i32 + (f[i] as i32 - b[i] as i32) * k / 16) as u32;
                    compose_colour(screen, ch(0), ch(1), ch(2))
                }
                // The split wrote nothing: a screen of another type composes 0; without a screen
                // the output dword keeps the caller-named stale value.
                _ => match screen {
                    ScreenFormat::Screen(_) => 0,
                    ScreenFormat::NoScreen { stale_out } => stale_out,
                },
            };
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

/// The screen buffer's colour split (`0x00825F80`, §9 table); `None` = returned false (no screen
/// or another colour type), outputs untouched.
pub fn split_colour(screen: ScreenFormat, c: u32) -> Option<[u8; 3]> {
    let ScreenFormat::Screen(t) = screen else {
        return None;
    };
    let x5 = |v: u32| (((v & 31) * 33) >> 2) as u8;
    match t {
        4 | 6 => Some([x5(c >> 10), x5(c >> 5), x5(c)]),
        5 => Some([x5(c >> 11), ((((c >> 5) & 63) * 65) >> 4) as u8, x5(c)]),
        7 => Some([
            (((c >> 8) & 15) * 17) as u8,
            (((c >> 4) & 15) * 17) as u8,
            ((c & 15) * 17) as u8,
        ]),
        8 | 9 => Some([(c >> 16) as u8, (c >> 8) as u8, c as u8]),
        _ => None,
    }
}

/// The screen buffer's colour compose (`0x00825DA0`, §9 table); channels are taken as bytes.
/// Without a screen buffer the caller-named stale output is returned.
pub fn compose_colour(screen: ScreenFormat, r: u32, g: u32, b: u32) -> u32 {
    let t = match screen {
        ScreenFormat::Screen(t) => t,
        ScreenFormat::NoScreen { stale_out } => return stale_out,
    };
    let (r, g, b) = (r & 255, g & 255, b & 255);
    match t {
        4 | 6 => 0x8000 | (r & 0xF8) << 7 | (g & 0xF8) << 2 | b >> 3,
        5 => (r & 0xF8) << 8 | (g & 0xFC) << 3 | b >> 3,
        7 => 0xF000 | (r & 0xF0) << 4 | (g & 0xF0) | b >> 4,
        8 | 9 => 0xFF00_0000 | r << 16 | g << 8 | b,
        _ => 0,
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
    /// Holder +0x24: the source type probed from the file contents when the face opens (§5 FN10).
    pub source_type: u32,
    /// Holder +0x20: u16 glyph load flags (§5).
    pub load_flags: u16,
    /// Holder +0x28: character → glyph index cache of the name map (§7 FN9).
    pub name_map: HashMap<u16, u32>,
    pub(crate) metrics: SizeMetrics,
}

impl GzFont {
    /// A freshly created font with the constructor defaults of §2 (FN11, `0x00899AAB`).
    pub fn new(lib: Arc<dyn FontLibrary>, conv: Arc<dyn CodePageConverter>) -> GzFont {
        GzFont {
            lib,
            conv,
            id: NEXT_FONT_ID.fetch_add(1, Ordering::Relaxed),
            initialised: false,
            file_path: String::new(),
            family_name: String::new(),
            face_kind: 0,
            point_size: 16,
            char_height: 16,
            style_flags: 0,
            fg: 0,
            bg: 0,
            aa_mode: 1,
            field_50: 2,
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
            default_char: 0x20,
            default_glyph: 0,
            face: None,
            records: Vec::new(),
            current_palette: None,
            palettes: Vec::new(),
            mono_cache: CachedBitmapData::default(),
            grey_cache: CachedBitmapData::default(),
            source_type: 0,
            load_flags: 0x000A,
            name_map: HashMap::new(),
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
        // §5 FN10: the content probe runs when the face is opened; type 0 does not open it.
        let st = probe_source_type(&*self.lib, &self.file_path);
        if st == 0 {
            return false;
        }
        let Some(face) = self.lib.open(&self.file_path) else {
            return false;
        };
        self.source_type = st;
        self.load_flags |= if st == 2 || st == 3 { 0x01 } else { 0x80 };
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

    // ---- §4.1 family A plain getters and the §4 getter halves --------------------------------

    /// Slot 5, GetName(&out): out ← familyName.
    pub fn get_name(&self) -> String {
        self.family_name.clone()
    }
    /// Slot 7, GetStyles: styleFlags.
    pub fn get_styles(&self) -> u32 {
        self.style_flags
    }
    /// Slot 9, GetFaceKind: 1 scalable, 2 fixed-size (set by Init).
    pub fn get_face_kind(&self) -> u32 {
        self.face_kind
    }
    /// Slot 10, GetSize: pointSize.
    pub fn get_size(&self) -> u32 {
        self.point_size
    }
    /// Slot 12, GetFontColor: foreground.
    pub fn get_font_color(&self) -> u32 {
        self.fg
    }
    /// Slot 13, GetFontBackgroundColor: background.
    pub fn get_font_background_color(&self) -> u32 {
        self.bg
    }
    /// Slot 15, GetAntialiasing.
    pub fn get_antialiasing(&self) -> u32 {
        self.aa_mode
    }
    /// Slot 17, GetAntialiasingGammaCorrection.
    pub fn get_aa_gamma(&self) -> f32 {
        self.aa_gamma
    }
    /// Slot 19, GetAntialiasingSharpness.
    pub fn get_aa_sharpness(&self) -> f32 {
        self.aa_sharpness
    }
    /// Slot 21, GetFilePath(&out): string copy.
    pub fn get_file_path(&self) -> String {
        self.file_path.clone()
    }
    /// Slot 23, GetCodePage(&cp): always true.
    pub fn get_code_page(&self) -> (bool, u32) {
        (true, self.code_page)
    }
    /// Slot 33, GetSpacingAdjustment(&char, &line): plain read.
    pub fn get_spacing_adjustment(&self) -> (i32, i32) {
        (self.char_spacing, self.line_spacing)
    }
    /// Slot 37, CharHeight(a, b): both stack arguments are ignored.
    pub fn char_height_args(&self, _a: u32, _b: u32) -> i32 {
        self.char_height
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
                    ch = ch.wrapping_add(1);
                    if ch as u32 > 28 {
                        ch = ch.wrapping_add(1);
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

    /// The name map `0x00898EB2` (§7 FN9) for source types 2/3: holder map, then the PostScript
    /// glyph name of `ch`, then `uniXXXX`; a hit is cached, a miss is not.
    pub(crate) fn name_map_index(&mut self, ch: u32) -> u32 {
        let key = ch as u16;
        if let Some(&i) = self.name_map.get(&key) {
            return i;
        }
        let Some(face) = self.face.as_ref() else {
            return 0;
        };
        let fallback = format!("uni{:04X}", ch & 0xFFFF);
        for name in [names::glyph_name(key), Some(fallback.as_str())].into_iter().flatten() {
            if let Some(code) = face.glyph_name_to_code(name) {
                let idx = face.char_index(code).unwrap_or(0);
                if idx != 0 {
                    self.name_map.insert(key, idx);
                    return idx;
                }
            }
        }
        0
    }

    /// The scaled advance of a loaded glyph (§7 FN7). `adv` is 16.16.
    pub fn scaled_advance(adv: i32, s: f32, ch: u32) -> i32 {
        advance::scaled_advance(adv, s, ch, advance::X87Control::default())
            .expect("FN18: unsupported x87 integer conversion") as i32
    }

    /// Slot 61, AddCharData (`0x00898FDC`) into the active cache.
    pub fn add_char_data(&mut self, ch: u32) -> Option<GlyphRecord> {
        let st23 = self.source_type == 2 || self.source_type == 3;
        let levels = if self.aa_mode == 1 {
            GreyLevels::Two
        } else {
            GreyLevels::Levels256
        };
        let zero_width = ch == 0x200B || ch == 0xFEFF;
        let mut loaded = None;
        if self.face.is_some() {
            let mut idx = if ch < 0x80 || !st23 {
                self.face.as_ref().and_then(|f| f.char_index(ch)).unwrap_or(0)
            } else {
                0
            };
            if idx == 0 && st23 && ch >= 0x20 {
                idx = self.name_map_index(ch);
            }
            if idx == 0 {
                idx = self.default_glyph;
            }
            if idx != 0 {
                let flags = self.load_flags;
                let face = self.face.as_mut().expect("face");
                let fixed = face.is_fixed_size().then(|| face.fixed_advance());
                loaded = face.load_glyph(idx, levels, flags).map(|g| (g, fixed));
            }
        }
        let a_plus_b = self.metrics.a.wrapping_add(self.metrics.b) / 65536;
        if let Some((g, fixed)) = loaded {
            let x_off = g.bearing_x / 64;
            let y_off = a_plus_b - g.bearing_y / 64;
            let adv16 = fixed.unwrap_or(g.advance);
            let adv = if zero_width {
                0
            } else {
                advance::scaled_advance(adv16, self.advance_scale, ch, self.lib.x87_control())
                    .expect("FN18: unsupported x87 integer conversion") as i32
            };
            let rec = self.copy_glyph_to_source_data(&g, levels == GreyLevels::Two, x_off, y_off, adv);
            let rec = *self.active_cache_mut().glyphs.entry(ch).or_insert(rec);
            if !g.done_error {
                return Some(rec);
            }
        }
        // Missing or unloadable glyph (§7 FN7): a hollow w × w outline at the line top.
        let (raster, adv, bits) = if zero_width {
            (GlyphRaster::default(), 0, true)
        } else {
            let w = (self.char_height as u32) >> 1;
            let mut buf = vec![0u8; (w * w) as usize];
            for y in 0..w {
                for x in 0..w {
                    if y == 0 || y + 1 == w || x == 0 || x + 1 == w {
                        buf[(y * w + x) as usize] = 0xFF;
                    }
                }
            }
            let g = GlyphRaster {
                width: w,
                rows: w,
                pitch: w,
                buffer: buf,
                ..GlyphRaster::default()
            };
            let adv = self
                .lib
                .x87_control()
                .product(w as i32, self.advance_scale, 0)
                .expect("FN18: unsupported x87 integer conversion") as i16 as i32;
            (g, adv, false)
        };
        let rec = self.copy_glyph_to_source_data(&raster, bits, 0, 0, adv);
        // Insert-unique (`0x00897CB2`): after a glyph-done error the loaded record stays.
        Some(*self.active_cache_mut().glyphs.entry(ch).or_insert(rec))
    }

    fn active_cache_mut(&mut self) -> &mut CachedBitmapData {
        if self.aa_mode == 1 {
            &mut self.mono_cache
        } else {
            &mut self.grey_cache
        }
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
        // §7 FN8: line rows (cell rows), from pointSize-only thicknesses; underline first.
        let (t_u, t_s) = line_thickness(self.point_size);
        let mut line_rows: Vec<i32> = Vec::new();
        let ch_h = self.char_height;
        let mut push_rows = |r: i32, t: i32| {
            if r < 0 || r >= ch_h {
                return;
            }
            line_rows.extend((r..r + t).take_while(|&row| row < ch_h));
        };
        if lines & STYLE_UNDERLINE != 0 {
            push_rows(self.ascent - y_off + t_u, t_u);
        }
        if lines & STYLE_STRIKE != 0 {
            push_rows(self.ascent - (self.char_height >> 2) - y_off, t_s);
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

    /// Slot 53 returns a borrowed renderer, without acquiring a reference.
    pub fn get_renderer(&self) -> Option<&Arc<dyn FontRenderer>> {
        self.renderer.as_ref()
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

    fn font_use(&self) -> FontUse {
        FontUse {
            font_id: self.id,
            mode: match self.aa_mode {
                1..=3 => self.aa_mode,
                _ => 1,
            },
        }
    }

    /// Slot 55, FN17. The native null dereference is represented by a panic before mutation.
    /// Callback failure is ignored; the wrapping increment and true return still occur.
    pub fn lock_surface(&mut self, target: SurfaceId) -> bool {
        if self.surface_lock_count == 0 {
            self.renderer
                .as_ref()
                .expect("FN17: null renderer on lock edge")
                .lock_surface(target, self.font_use());
        }
        self.surface_lock_count = self.surface_lock_count.wrapping_add(1);
        true
    }

    /// Slot 56.
    pub fn unlock_surface(&mut self, target: SurfaceId) -> bool {
        if self.surface_lock_count == 1 {
            self.renderer
                .as_ref()
                .expect("FN17: null renderer on unlock edge")
                .unlock_surface(target, self.font_use());
        }
        self.surface_lock_count = self.surface_lock_count.wrapping_sub(1);
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
            self.palettes
                .insert(0, Palette::build(fg, bg, self.lib.screen_format()));
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
                // DrawTextLine's automatic path checks the callback (unlike explicit slot 55).
                if !self
                    .renderer
                    .as_ref()
                    .expect("null renderer on automatic lock edge")
                    .lock_surface(t, self.font_use())
                {
                    return false;
                }
                self.surface_lock_count = self.surface_lock_count.wrapping_add(1);
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
                        DrawColour::Palette(
                            self.current_palette
                                .clone()
                                .unwrap_or_else(|| Palette::build(fg, bg, self.lib.screen_format())),
                        )
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

impl Drop for GzFont {
    fn drop(&mut self) {
        // §3: destruction calls Shutdown before the owned fields (including the foreign
        // renderer reference and face) are released, also for ordinary Rust ownership.
        self.shutdown();
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
        self.refs.fetch_add(1, Ordering::SeqCst).wrapping_add(1)
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
        // The deleting destructor frees the holder even when Init failed before the flag
        // was set (Shutdown alone deliberately does nothing on that path).
        f.face = None;
        f.name_map.clear();
        f.mono_cache.clear();
        f.grey_cache.clear();
        f.current_palette = None;
        f.palettes.clear();
        f.records.clear();
        f.glyph_map.clear();
        f.aa_table.clear();
        f.file_path.clear();
        f.family_name.clear();
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
