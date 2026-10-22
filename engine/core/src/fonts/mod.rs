// SPDX-License-Identifier: GPL-3.0-or-later
//! The font family: `cGZFont` (font object, glyph cache, measuring, layout, drawing records),
//! `cGZFontSys` (font catalogue) and `cFontCOMDirector` (class registration).
//!
//! Clean-room implementation of docs/specs/drawing-fonts.md. The font file / rasteriser library
//! (FreeType in the original) and the system code-page converter are external library contracts:
//! they are reached only through the typed provider traits below ([`FontLibrary`], [`FontFace`],
//! [`CodePageConverter`]), and the drawing back end (`cGZDrawFontRenderer`, out of scope) only
//! through [`FontRenderer`].

pub mod aa;
pub mod advance;
pub mod cache;
pub mod director;
pub mod encoding;
pub mod font;
pub mod fontsys;
pub mod layout;
mod names;
pub mod thai;
pub mod words;

#[cfg(test)]
mod tests;

use std::sync::Arc;

pub use cache::{CachedBitmapData, GlyphRecord};
pub use font::{DrawRecord, FontObj, FontRef, GzFont, Palette, PEN_DEFAULT_COLOUR};
pub use fontsys::{FontFixedInfo, FontInfoRecord, GzFontSys};

/// Vertical size metrics of the face after a char-size call, 16.16 fixed (spec §5):
/// `a` = face+0x7C, `b` = face+0x6C, `c` = face+0x74.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SizeMetrics {
    pub a: i32,
    pub b: i32,
    pub c: i32,
}

/// The screen buffer's colour format as the palette builder sees it through `RZ::GS()` +0x90/+0x8C
/// (§9, FN12). Explicit caller state: the core never assumes a display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenFormat {
    /// A screen buffer with colour type `t` (buffer +0xC): 4/6 = 1555, 5 = 565, 7 = 4444,
    /// 8/9 = 8888; any other type splits nothing and composes 0.
    Screen(u32),
    /// No screen buffer: split and compose write nothing, so entries 8..119 keep whatever the
    /// compose output dword held. The original leaves it uninitialised; the caller names it.
    NoScreen { stale_out: u32 },
}

impl ScreenFormat {
    /// The screen colour type for a display depth (§9): 7 (A4R4G4B4) at depth ≤ 16, else 9.
    pub fn from_depth(depth: u32) -> ScreenFormat {
        ScreenFormat::Screen(if depth <= 16 { 7 } else { 9 })
    }
}

/// Grey levels a glyph is loaded at (§7): 2 for aaMode 1, 256 otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GreyLevels {
    Two,
    Levels256,
}

/// A rasterised glyph as the external library returns it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlyphRaster {
    pub width: u32,
    pub rows: u32,
    /// Bytes per row of `buffer`. For [`GreyLevels::Two`] rows are packed 1 bit per pixel, MSB first.
    pub pitch: u32,
    pub buffer: Vec<u8>,
    /// Horizontal bearing, 26.6 fixed.
    pub bearing_x: i32,
    /// Vertical bearing, 26.6 fixed.
    pub bearing_y: i32,
    /// The loaded glyph's 16.16 advance (face+0xD0, §7 FN7); used unless the face is fixed-size.
    pub advance: i32,
    /// The glyph-done call (`0x0089E29E`) reported an error after a successful load (§7 edge).
    pub done_error: bool,
}

/// One open face (external library boundary).
pub trait FontFace: Send {
    fn family_name(&self) -> String;
    /// Style bits the face reports (the low bits of `styleFlags`, §2/§3).
    fn style_flags(&self) -> u32;
    /// The face's fixed-size flag (face+0x5C != 0).
    fn is_fixed_size(&self) -> bool;
    /// `FT_Get_Char_Index`: `None` when the lookup reports an error.
    fn char_index(&self, ch: u32) -> Option<u32>;
    /// The face's 16.16 advance at face+0x80, used for fixed-size faces (§7 FN7).
    fn fixed_advance(&self) -> i32;
    /// Resolve a PostScript glyph name to a character code (`0x0089F4F3`, §7 FN9); `None` when the
    /// face has no such glyph name.
    fn glyph_name_to_code(&self, _name: &str) -> Option<u32> {
        None
    }
    /// Set the char size to `size` points at `dpi_x` × `dpi_y` with the scaling transform.
    fn set_char_size(&mut self, size: u32, dpi_x: u32, dpi_y: u32, scale_x: f32, scale_y: f32) -> Option<SizeMetrics>;
    /// Load and render glyph `index` with the holder's u16 load flags (§5: 0x000A, plus 0x80 for
    /// source types 1/4, 0x01 for 2/3); `None` when the load reports an error.
    fn load_glyph(&mut self, index: u32, levels: GreyLevels, load_flags: u16) -> Option<GlyphRaster>;
    /// The probe lookup used by IsCodePageAvailable / IsCharAvailable (`0x0089F538`): glyph present.
    fn has_glyph(&self, unit: u16) -> bool {
        matches!(self.char_index(unit as u32), Some(i) if i != 0)
    }
}

/// The file system and font library (external library boundary).
pub trait FontLibrary: Send + Sync {
    fn file_exists(&self, path: &str) -> bool;
    /// Open a face; `None` when the library cannot open it.
    fn open(&self, path: &str) -> Option<Box<dyn FontFace>>;
    /// The first `n` bytes of a file (for the source-type sniff, §12).
    fn read_head(&self, path: &str, n: usize) -> Vec<u8>;
    /// `<Windows>\Fonts\` (with trailing separator).
    fn system_fonts_dir(&self) -> String;
    /// Files of `dir` matching `pattern` (full paths).
    fn list_files(&self, dir: &str, pattern: &str) -> Vec<String>;
    /// The default 8-byte renderer Init creates when none is set (§3). `cGZDrawFontRenderer` is out
    /// of scope: a host without one returns `None`.
    fn default_renderer(&self) -> Option<Arc<dyn FontRenderer>> {
        None
    }
    /// The screen colour format the palette builder splits and composes with (§9, FN12).
    fn screen_format(&self) -> ScreenFormat;
    /// The calling thread's x87 control (§7 FN16). Hosts without x87 use the application's
    /// startup default (PC24, nearest-even); hosts with ambient changes must report them here.
    fn x87_control(&self) -> advance::X87Control {
        advance::X87Control::default()
    }
}

/// The system code-page converter (external library boundary, §6).
pub trait CodePageConverter: Send + Sync {
    /// codePage 0: one byte ≥ 0x80 through the system ANSI converter.
    fn ansi_byte_to_unicode(&self, b: u8) -> u16;
    /// Length in bytes of the multibyte character at the start of `bytes` in code page `cp`.
    fn mb_char_len(&self, cp: u32, bytes: &[u8]) -> usize;
    /// Convert that one character to one UTF-16 unit.
    fn mb_to_unicode(&self, cp: u32, bytes: &[u8]) -> u16;
}

/// A trivial converter: every byte is its own Latin-1 code point. For tests and hosts without one.
#[derive(Clone, Copy, Debug, Default)]
pub struct Latin1Converter;

impl CodePageConverter for Latin1Converter {
    fn ansi_byte_to_unicode(&self, b: u8) -> u16 {
        b as u16
    }
    fn mb_char_len(&self, _cp: u32, _bytes: &[u8]) -> usize {
        1
    }
    fn mb_to_unicode(&self, _cp: u32, bytes: &[u8]) -> u16 {
        bytes.first().copied().unwrap_or(0) as u16
    }
}

/// Opaque drawing target handle.
pub type SurfaceId = u64;

/// Arguments supplied on the renderer's lock/unlock edges (§9 FN17).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontUse {
    pub font_id: usize,
    pub mode: u32,
}

/// The colour argument handed to the renderer with the records (§9 step 4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DrawColour {
    Palette(Palette),
    Colour(u32),
}

/// The renderer interface the font calls (renderer vtable +0x0C/+0x10/+0x14/+0x18/+0x1C, §9).
pub trait FontRenderer: Send + Sync {
    fn attach(&self, _font_id: usize) {}
    fn detach(&self, _font_id: usize) {}
    fn lock_surface(&self, target: SurfaceId, font: FontUse) -> bool;
    fn unlock_surface(&self, target: SurfaceId, font: FontUse) -> bool;
    /// `source` is the active cache's bitmap; every record's `src_offset` indexes it.
    fn draw(&self, target: SurfaceId, records: &[DrawRecord], source: &[u8], colour: &DrawColour);
}

/// ASCII lower-casing as the catalogue uses it (CRT `tolower` in the C locale).
pub(crate) fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}
