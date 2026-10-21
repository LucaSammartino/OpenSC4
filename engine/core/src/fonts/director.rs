// SPDX-License-Identifier: GPL-3.0-or-later
//! `cFontCOMDirector` (drawing-fonts.md §1): the three classes it registers and its OnStart rule.

use super::font::CLSID_GZ_FONT;
use super::fontsys::{CLSID_GZ_FONT_SYS, IID_GZ_FONT_SYS2};

/// Director ID.
pub const DIRECTOR_ID: u32 = 0x5AA4_0BD3;
/// `cGZDrawFontRenderer` class (identity only; out of scope).
pub const CLSID_DRAW_FONT_RENDERER: u32 = 0x08CD_A30B;

/// What a registered class creates, with its object size in the original.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontClass {
    FontSys,
    Font,
    Renderer,
}

/// The classes in registration order: (class ID, class, object bytes).
pub const CLASSES: [(u32, FontClass, u32); 3] = [
    (CLSID_GZ_FONT_SYS, FontClass::FontSys, 0x70),
    (CLSID_GZ_FONT, FontClass::Font, 0x15C),
    (CLSID_DRAW_FONT_RENDERER, FontClass::Renderer, 8),
];

/// The factory lookup for a class ID.
pub fn class_for(clsid: u32) -> Option<FontClass> {
    CLASSES.iter().find(|c| c.0 == clsid).map(|c| c.1)
}

/// OnStart: create `cGZFontSys` asking for this interface; register on success, then release.
pub const ON_START_CLASS: u32 = CLSID_GZ_FONT_SYS;
pub const ON_START_INTERFACE: u32 = IID_GZ_FONT_SYS2;
