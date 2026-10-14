// SPDX-License-Identifier: GPL-3.0-or-later
//! FSH (`SHPI`) textures, type [`TYPE_FSH`], as the original loads them (`docs/specs/graphics-formats.md` §2).
//!
//! # Layout (§2.2–§2.5)
//!
//! ```text
//! 0x00  "SHPI"            compared exactly
//! 0x04  u32 size          bytes the loader reads, header included; bytes after it are ignored
//! 0x08  u32 count         directory entries (frames)
//! 0x0C  char[4]           directory ID ("G264", "G354", ...): stored, never checked
//! 0x10  count × (char[4] tag, u32 offset)    offset from the record start to the entry header
//!
//! entry header (16 bytes), then the pixels:
//! +0x00 u8  code          §2.6
//! +0x01 u24 next          offset from this header to the next attached block, 0 = none
//! +0x04 s16 width, s16 height (sign-extended)
//! +0x08 u16 centre x, u16 centre y (ignored)
//! +0x0C u32 misc          bits 0-11: signed 12-bit A (DXT1 RGB/RGBA selector); bits 28-31: mip count M
//! ```
//!
//! # What the factory accepts (§2.6, §2.7)
//!
//! Frame 0's code must map to a renderer (format, type) pair ([`format_and_type`]; `0x7B` also needs a 256 × 1
//! palette found through the attachment chain, §2.5). Frame 0's **height** must be a power of two (0 passes);
//! the width is not checked. Every other frame must equal frame 0 in height, code and mip count (not width), and
//! all frames are laid out with frame 0's dimensions. Levels follow §2.8 ([`level_table`]): each level starts
//! 16-byte aligned from the entry's pixel start and DXT levels stop shrinking at 4 × 4. The (format, type) pair
//! is decided once, from frame 0, and every frame uses it (§2.10, FS5).
//!
//! # FreeCity4 choices
//!
//! The original checks no offset or length against the record (§2.2, Q2). FreeCity4 does *\[choice\]*:
//! - `size` below `16 + 8 × count` is rejected, as is a record shorter than `size`; a record with no entry
//!   (`count` 0) is rejected, as the original rejects it (§2.7, FS3).
//! - Every entry header, attachment header and frame 0's palette must lie inside the first `size` bytes.
//! - Each entry's own bytes end at the first of: its attached block (`next`), another entry's header, or `size`
//!   (only positions at or after its pixel start count; the original has no entry end, §2.8 FS1). The levels must
//!   fit there, except in the **mip-padding case** of §2.8 (FS2): when the levels packed back to back would fit
//!   but the 16-byte-aligned layout does not, the entry is accepted ([`Frame::zero_filled`]).
//! - Levels are read at the aligned offsets from the record, as the game reads them, whatever lies there: bytes
//!   past an entry's own data are the record's following bytes (the next attachment), and only bytes past `size`
//!   read as zero (§2.8, GF9; [`Fsh::level_data`]). A DXT level whose width is not a multiple of 4 reads its whole
//!   blocks the same way (FS7).
//! - `0x7B`: only frame 0 must have a 256 × 1 palette whose 256 entries fit in the record (the original checks
//!   frame 0 only, and reads uninitialised data when there is none; FS4). A later frame with no usable palette
//!   is accepted and decodes as grey, (i, i, i, 255), the original's luminance upload read unsigned.
//! - Pixels decode to RGBA8 as §2.11 says ([`decode_pixels`]); see the crate README for the decoding choices.

use std::borrow::Cow;

use crate::error::{corrupt, Error, Result};
use crate::tgi::{TYPE_FSH, TYPE_FSH_UNCREATED};

/// The container signature.
pub const MAGIC: [u8; 4] = *b"SHPI";
/// Length of the container header (before the directory).
pub const HEADER_LEN: usize = 16;
/// Length of one directory entry.
pub const DIR_ENTRY_LEN: usize = 8;
/// Length of an entry (or attached block) header.
pub const ENTRY_HEADER_LEN: usize = 16;

/// Bitmap and palette codes (§2.5, §2.6).
pub mod code {
    /// DXT1, 8 bytes per 4 × 4 block.
    pub const DXT1: u8 = 0x60;
    /// DXT3, 16 bytes per block.
    pub const DXT3: u8 = 0x61;
    /// DXT5, 16 bytes per block.
    pub const DXT5: u8 = 0x62;
    /// 8-bit alpha.
    pub const A8: u8 = 0x64;
    /// 8-bit luminance, 8-bit alpha.
    pub const L8A8: u8 = 0x65;
    /// 16-bit 4:4:4:4.
    pub const ARGB4444: u8 = 0x6D;
    /// 16-bit 5:6:5.
    pub const RGB565: u8 = 0x78;
    /// 4-bit index: rejected by the factory.
    pub const INDEX4: u8 = 0x7A;
    /// 8-bit index with a 256 × 1 palette.
    pub const INDEX8: u8 = 0x7B;
    /// 32-bit, bytes B, G, R, A.
    pub const ARGB8888: u8 = 0x7D;
    /// 16-bit 1:5:5:5.
    pub const ARGB1555: u8 = 0x7E;
    /// 24-bit, bytes B, G, R.
    pub const RGB888: u8 = 0x7F;
    /// Palette, B, G, R, A bytes per entry.
    pub const PALETTE_8888: u8 = 0x2A;
    /// Palette, B, G, R bytes per entry.
    pub const PALETTE_888: u8 = 0x24;
    /// Palette, `u16` A1R5G5B5 per entry.
    pub const PALETTE_1555: u8 = 0x2D;
    /// Palette, `u16` R5G6B5 per entry.
    pub const PALETTE_565: u8 = 0x29;
    /// Codes the palette search stops at (§2.5).
    pub const PALETTES: [u8; 4] = [PALETTE_8888, PALETTE_888, PALETTE_1555, PALETTE_565];
}

/// The renderer's texture format (`eGDTextureFormat`, §2.6). [`TextureFormat::number`] is the enum value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum TextureFormat {
    Bgr = 2,
    Bgra = 3,
    Alpha = 4,
    Luminance = 5,
    LuminanceAlpha = 6,
    Dxt1Rgb = 7,
    Dxt1Rgba = 8,
    Dxt3 = 9,
    Dxt5 = 10,
}

impl TextureFormat {
    /// The `eGDTextureFormat` number.
    pub fn number(self) -> u32 {
        self as u32
    }

    /// True for the three block-compressed formats.
    pub fn is_dxt(self) -> bool {
        matches!(self, Self::Dxt1Rgb | Self::Dxt1Rgba | Self::Dxt3 | Self::Dxt5)
    }
}

/// The renderer's component type (`eGDType`, §2.6). [`ComponentType::number`] is the enum value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ComponentType {
    /// Signed byte (what the OpenGL table maps type 0 to; FreeCity4 reads such data unsigned *\[choice\]*).
    Byte = 0,
    UnsignedByte = 1,
    UnsignedShort4444 = 8,
    UnsignedShort5551 = 9,
    /// Packed 16-bit, used by code `0x78`; not mapped by the OpenGL table either.
    Packed16 = 11,
}

impl ComponentType {
    /// The `eGDType` number.
    pub fn number(self) -> u32 {
        self as u32
    }
}

/// The (format, type) pair the factory asks the renderer for (§2.6, `GetTypeAndFormat`), from the code and the
/// 12-bit field A of the entry header. `None` for a code the factory rejects (`0x7A` and every code not in the
/// table). The `0x7B` palette check is separate (§2.5, [`Fsh::parse`]).
pub fn format_and_type(code: u8, a: i16) -> Option<(TextureFormat, ComponentType)> {
    use ComponentType as T;
    use TextureFormat as F;
    Some(match code {
        code::DXT1 if a != 0 => (F::Dxt1Rgba, T::Byte),
        code::DXT1 => (F::Dxt1Rgb, T::Byte),
        code::DXT3 => (F::Dxt3, T::Byte),
        code::DXT5 => (F::Dxt5, T::Byte),
        code::A8 => (F::Alpha, T::Byte),
        code::L8A8 => (F::LuminanceAlpha, T::Byte),
        code::ARGB4444 => (F::Bgra, T::UnsignedShort4444),
        code::RGB565 => (F::Bgr, T::Packed16),
        code::INDEX8 => (F::Luminance, T::Byte),
        code::ARGB8888 => (F::Bgra, T::UnsignedByte),
        code::ARGB1555 => (F::Bgra, T::UnsignedShort5551),
        code::RGB888 => (F::Bgr, T::UnsignedByte),
        _ => return None,
    })
}

/// True if the default DX7 renderer can create a texture with this pair (§2.6, `TranslateGDFormatToSourceFormat`):
/// the three DXT formats and BGR/BGRA with unsigned bytes. The other accepted codes (`0x64 0x65 0x6D 0x78 0x7B
/// 0x7E`) load but get no surface there, so under DX7 a model using one is drawn untextured, in its diffuse
/// colour and alpha (GF1). FreeCity4 decodes and shows them *\[choice\]*, as the original's OpenGL renderer
/// does for all of them but `0x78`.
pub fn dx7_creatable(format: TextureFormat, ty: ComponentType) -> bool {
    format.is_dxt() || (matches!(format, TextureFormat::Bgr | TextureFormat::Bgra) && ty == ComponentType::UnsignedByte)
}

/// `PixelWidthMul2` (§2.6): twice the bytes per pixel of a code, for every code of the table (including `0x7A`).
pub fn pixel_width_mul2(code: u8) -> Option<u32> {
    Some(match code {
        code::DXT1 | code::INDEX4 => 1,
        code::DXT3 | code::DXT5 | code::A8 | code::INDEX8 => 2,
        code::L8A8 | code::ARGB4444 | code::RGB565 | code::ARGB1555 => 4,
        code::RGB888 => 6,
        code::ARGB8888 => 8,
        _ => return None,
    })
}

/// The minimum level dimension m (§2.7 rule 3): 4 for the DXT codes `0x60`–`0x62`, else 1.
pub fn min_level_dim(code: u8) -> u32 {
    if (code::DXT1..=code::DXT5).contains(&code) {
        4
    } else {
        1
    }
}

/// An entry (or attached block) header (§2.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryHeader {
    pub code: u8,
    /// 24-bit offset from this header to the next attached block; 0 = none.
    pub next: u32,
    /// Sign-extended.
    pub width: i16,
    /// Sign-extended.
    pub height: i16,
    /// Ignored by the 3D path.
    pub centre_x: u16,
    /// Ignored by the 3D path.
    pub centre_y: u16,
    /// The dword at `+0x0C`: A in bits 0–11, M in bits 28–31, bits 12–27 ignored.
    pub misc: u32,
}

impl EntryHeader {
    /// Reads a header from the first 16 bytes of `b`; `None` if `b` is shorter.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let b = b.get(..ENTRY_HEADER_LEN)?;
        let u16_at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        Some(EntryHeader {
            code: b[0],
            next: u32::from_le_bytes([b[1], b[2], b[3], 0]),
            width: u16_at(4) as i16,
            height: u16_at(6) as i16,
            centre_x: u16_at(8),
            centre_y: u16_at(10),
            misc: u32::from_le_bytes([b[12], b[13], b[14], b[15]]),
        })
    }

    /// The signed 12-bit field A (bits 0–11, sign-extended): non-zero selects DXT1 with 1-bit alpha (§2.4).
    pub fn a(&self) -> i16 {
        (((self.misc & 0xFFF) as i32) << 20 >> 20) as i16
    }

    /// The mip count M (bits 28–31); the entry has M + 1 levels.
    pub fn mip_count(&self) -> u8 {
        (self.misc >> 28) as u8
    }
}

/// A directory entry (§2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirEntry {
    /// Never read by the game.
    pub tag: [u8; 4],
    /// Offset of the entry header from the record start.
    pub offset: u32,
}

/// A palette found by the §2.5 search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// Offset of the palette block's header from the record start; its data starts 16 bytes later.
    pub offset: u32,
    /// `0x2A`, `0x24`, `0x2D` or `0x29`.
    pub code: u8,
    /// `u16` at +4, zero-extended.
    pub width: u16,
    /// `u16` at +6, zero-extended.
    pub height: u16,
    /// The `u16` at +10 (kept, never checked).
    pub field_10: u16,
}

impl Palette {
    /// Bytes per palette entry for the code (§2.11).
    pub fn entry_size(&self) -> usize {
        palette_entry_size(self.code)
    }
}

fn palette_entry_size(code: u8) -> usize {
    match code {
        code::PALETTE_8888 => 4,
        code::PALETTE_888 => 3,
        _ => 2,
    }
}

/// One mipmap level (§2.8). Offsets are from the entry's pixel start (entry header + 16), the same for every
/// frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level {
    pub offset: u64,
    pub size: u64,
    /// Working dimensions (never below the code's minimum m).
    pub width: u32,
    pub height: u32,
}

/// The §2.8 level table for a code, frame-0 dimensions and mip count: level 0 at 0 with size
/// `PixelWidthMul2 × w × h / 2` (w, h at least m), each next level at the previous end rounded **up** to 16; going
/// down a level, w and h are each halved only while above m, and each halving halves the size. `None` for a code
/// without a pixel width.
///
/// A negative (sign-extended) width or height counts as below m, so it becomes m: the original's max is signed
/// (§2.7, FS6). A negative height never gets here: it fails the power-of-two test.
pub fn level_table(code: u8, width: i16, height: i16, mip_count: u8) -> Option<Vec<Level>> {
    let mul2 = u64::from(pixel_width_mul2(code)?);
    let m = min_level_dim(code);
    let clamp = |d: i16| (i32::from(d).max(m as i32)) as u32;
    let (mut w, mut h) = (clamp(width), clamp(height));
    let mut size = mul2 * u64::from(w) * u64::from(h) / 2;
    let mut offset = 0u64;
    let mut levels = Vec::with_capacity(usize::from(mip_count) + 1);
    for k in 0..=mip_count {
        if k > 0 {
            offset = (offset + size + 15) & !15;
            if w > m {
                w /= 2;
                size /= 2;
            }
            if h > m {
                h /= 2;
                size /= 2;
            }
        }
        levels.push(Level {
            offset,
            size,
            width: w,
            height: h,
        });
    }
    Some(levels)
}

/// Total bytes of a level table: aligned (the end of the last level) and packed back to back.
fn table_extent(levels: &[Level]) -> (u64, u64) {
    let aligned = levels.last().map_or(0, |l| l.offset + l.size);
    let packed = levels.iter().map(|l| l.size).sum();
    (aligned, packed)
}

/// One frame (directory entry) of an accepted FSH.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Offset of the entry header from the record start.
    pub offset: u32,
    pub header: EntryHeader,
    /// Frame 0's (format, type) pair (§2.6), the same in every frame: the original decides it once, from frame 0,
    /// and creates every frame with it, so a later DXT1 frame's own A is never read (§2.10, FS5).
    pub format: TextureFormat,
    pub component_type: ComponentType,
    /// The palette of a `0x7B` frame (§2.5); `None` for every other code. Always present in frame 0; `None` in a
    /// later frame with no usable palette (not found, not 256 × 1, or not inside the record), which decodes as
    /// grey (FS4).
    pub palette: Option<Palette>,
    /// End of this entry's own bytes (exclusive): its attached block, another entry's header, or `size`. Used
    /// only to accept the entry (§2.8, FS1); the levels are read from the record whatever lies past it.
    pub data_end: u32,
    /// True in the §2.8 mip-padding case: the 16-byte-aligned levels run past [`Frame::data_end`] (only the
    /// levels packed back to back fit). Those levels read the record's following bytes, as the game does (GF9);
    /// only bytes past `size` read as zero. (The name predates GF9.)
    pub zero_filled: bool,
}

impl Frame {
    /// Offset of the pixel data (the entry header + 16) from the record start.
    pub fn pixel_start(&self) -> u32 {
        self.offset + ENTRY_HEADER_LEN as u32
    }
}

/// An attached block found by walking an entry's `next` chain (for inspection; the game follows the chain only to
/// find a palette).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attachment {
    /// Offset of the block header from the record start.
    pub offset: u32,
    pub code: u8,
    pub next: u32,
}

/// An RGBA8 image, rows top to bottom, 4 bytes per pixel (R, G, B, A).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    /// The pixel at (x, y), or `None` outside the image.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = ((y as usize) * (self.width as usize) + x as usize) * 4;
        let p = self.rgba.get(i..i + 4)?;
        Some([p[0], p[1], p[2], p[3]])
    }
}

/// An accepted FSH texture: every frame passed the factory's checks (§2.7) and FreeCity4's bounds checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fsh {
    /// The record's first `size` bytes.
    data: Vec<u8>,
    /// The directory ID (`G264`, ...): stored, never checked.
    pub directory_id: [u8; 4],
    /// The directory, in file order (entry i is frame i).
    pub directory: Vec<DirEntry>,
    /// The frames, one per directory entry.
    pub frames: Vec<Frame>,
    /// The level table (§2.8), from frame 0's code, dimensions and mip count, shared by every frame.
    pub levels: Vec<Level>,
    /// The texture's dimensions: max(W, m) × max(H, m) of frame 0 (§2.7 rule 3).
    pub width: u32,
    pub height: u32,
}

impl Fsh {
    /// The factory read (§2.1): only [`TYPE_FSH`] creates an object. [`TYPE_FSH_UNCREATED`] is registered but never
    /// created, so it fails too.
    pub fn parse_record(type_id: u32, data: &[u8]) -> Result<Fsh> {
        match type_id {
            TYPE_FSH => Fsh::parse(data),
            TYPE_FSH_UNCREATED => Err(Error::Unsupported {
                what: "FSH",
                detail: format!("type {type_id:#010X} is registered but the factory never creates it"),
            }),
            _ => Err(Error::Unsupported {
                what: "FSH",
                detail: format!("type {type_id:#010X} is not an FSH type"),
            }),
        }
    }

    /// Parses and checks an FSH record (no type check; see [`Fsh::parse_record`]). See the module documentation.
    pub fn parse(data: &[u8]) -> Result<Fsh> {
        if data.len() < HEADER_LEN {
            return Err(Error::Truncated {
                what: "FSH header",
                needed: HEADER_LEN as u64,
                available: data.len() as u64,
            });
        }
        if data[..4] != MAGIC {
            return Err(Error::BadMagic {
                what: "FSH",
                found: data[..4].to_vec(),
            });
        }
        let word = |o: usize| u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
        let size = word(4);
        let count = word(8);
        let directory_id = [data[12], data[13], data[14], data[15]];
        // [choice] Q2: the directory must fit inside `size`.
        let dir_end = HEADER_LEN as u64 + DIR_ENTRY_LEN as u64 * u64::from(count);
        if u64::from(size) < dir_end {
            return Err(corrupt(
                "FSH",
                format!("size {size} is below the header and {count} directory entries"),
            ));
        }
        // §2.2: the loader reads `size` bytes; a shorter record fails, bytes after it are ignored.
        if (data.len() as u64) < u64::from(size) {
            return Err(Error::Truncated {
                what: "FSH",
                needed: u64::from(size),
                available: data.len() as u64,
            });
        }
        // §2.7 (FS3): a zero count fails the load before any entry is read.
        if count == 0 {
            return Err(corrupt("FSH", "no directory entry"));
        }
        let data = &data[..size as usize];
        let directory: Vec<DirEntry> = data[HEADER_LEN..dir_end as usize]
            .chunks_exact(DIR_ENTRY_LEN)
            .map(|c| DirEntry {
                tag: [c[0], c[1], c[2], c[3]],
                offset: u32::from_le_bytes([c[4], c[5], c[6], c[7]]),
            })
            .collect();
        let header_at = |off: u32, what: &'static str| -> Result<EntryHeader> {
            EntryHeader::parse(data.get(off as usize..).unwrap_or(&[])).ok_or(Error::OutOfBounds {
                what,
                offset: u64::from(off),
                size: ENTRY_HEADER_LEN as u64,
                limit: u64::from(size),
            })
        };
        let headers = directory
            .iter()
            .map(|d| header_at(d.offset, "FSH entry header"))
            .collect::<Result<Vec<_>>>()?;

        // §2.7 rules 1-4, on frame 0. The (format, type) pair is decided here, once, from frame 0's code and A,
        // and every frame is created with it (§2.10, FS5).
        let h0 = headers[0];
        let (format, component_type) = format_and_type(h0.code, h0.a())
            .ok_or_else(|| corrupt("FSH", format!("bitmap code {:#04X} is not accepted", h0.code)))?;
        let height = i32::from(h0.height);
        if height & height.wrapping_neg() != height {
            return Err(corrupt("FSH", format!("height {height} is not a power of two")));
        }
        for (i, h) in headers.iter().enumerate().skip(1) {
            if h.height != h0.height || h.code != h0.code || h.mip_count() != h0.mip_count() {
                return Err(corrupt(
                    "FSH",
                    format!(
                        "frame {i} (code {:#04X}, height {}, M {}) differs from frame 0 (code {:#04X}, height {}, M {})",
                        h.code,
                        h.height,
                        h.mip_count(),
                        h0.code,
                        h0.height,
                        h0.mip_count()
                    ),
                ));
            }
        }
        let levels = level_table(h0.code, h0.width, h0.height, h0.mip_count())
            .ok_or_else(|| corrupt("FSH", format!("bitmap code {:#04X} has no pixel width", h0.code)))?;
        let (aligned, packed) = table_extent(&levels);

        let mut frames = Vec::with_capacity(headers.len());
        for (i, (d, h)) in directory.iter().zip(&headers).enumerate() {
            let pixel_start = u64::from(d.offset) + ENTRY_HEADER_LEN as u64;
            // [choice] the entry's own bytes end at its attached block, another entry's header, or `size`. The
            // original has no entry end (FS1); this end only decides acceptance.
            let mut data_end = u64::from(size);
            if h.next != 0 {
                let attached = u64::from(d.offset) + u64::from(h.next);
                if attached >= pixel_start {
                    data_end = data_end.min(attached);
                }
            }
            for other in &directory {
                let o = u64::from(other.offset);
                if o >= pixel_start {
                    data_end = data_end.min(o);
                }
            }
            let available = data_end.saturating_sub(pixel_start);
            // [choice] Q2, FS2: the levels must fit, except the §2.8 mip-padding case (packed fits, aligned does
            // not). The original accepts any layout.
            if packed > available {
                return Err(Error::OutOfBounds {
                    what: "FSH pixel data",
                    offset: pixel_start,
                    size: packed,
                    limit: data_end,
                });
            }
            // §2.6 (FS4): the original checks frame 0's palette only. [choice] A later frame keeps its own
            // palette when it is usable and otherwise has none (it decodes as grey); its chain is not an error.
            let palette = if h.code != code::INDEX8 {
                None
            } else if i == 0 {
                Some(index8_palette(data, d.offset, i)?)
            } else {
                index8_palette(data, d.offset, i).ok()
            };
            frames.push(Frame {
                offset: d.offset,
                header: *h,
                format,
                component_type,
                palette,
                data_end: data_end as u32,
                zero_filled: aligned > available,
            });
        }
        Ok(Fsh {
            data: data.to_vec(),
            directory_id,
            directory,
            frames,
            width: levels[0].width,
            height: levels[0].height,
            levels,
        })
    }

    /// The record's first `size` bytes (everything the loader reads).
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Frame 0's code (every frame has the same one).
    pub fn code(&self) -> u8 {
        self.frames[0].header.code
    }

    /// The mip count M (every frame has the same one); there are M + 1 levels.
    pub fn mip_count(&self) -> u8 {
        self.frames[0].header.mip_count()
    }

    /// Frame 0's (format, type) pair: what the factory asks the renderer for (§2.6).
    pub fn format(&self) -> (TextureFormat, ComponentType) {
        (self.frames[0].format, self.frames[0].component_type)
    }

    /// The bytes of one level of one frame, `size` bytes from the §2.8 aligned offset, read from the record as the
    /// game reads them: in the mip-padding case ([`Frame::zero_filled`]) the last levels take the record's
    /// following bytes (the next attachment's header, GF9), and only bytes past `size` read as zero *\[choice\]*.
    /// These are the raw pixels; for a `0x7B` frame the 8-bit indices, which the original's terrain code reads on
    /// the CPU (§2.13, GF4). `None` for a frame or level that does not exist.
    pub fn level_data(&self, frame: usize, level: usize) -> Option<Cow<'_, [u8]>> {
        let f = self.frames.get(frame)?;
        let l = self.levels.get(level)?;
        Some(self.record_bytes(u64::from(f.pixel_start()) + l.offset, l.size))
    }

    /// `len` bytes of the record from `start`; bytes past `size` read as zero *\[choice\]* (the original would read
    /// past its buffer; no shipped level does, §2.8).
    fn record_bytes(&self, start: u64, len: u64) -> Cow<'_, [u8]> {
        let end = start + len;
        let limit = self.data.len() as u64;
        if end <= limit {
            return Cow::Borrowed(&self.data[start as usize..end as usize]);
        }
        let mut out = vec![0u8; len as usize];
        if start < limit {
            let n = (limit - start) as usize;
            out[..n].copy_from_slice(&self.data[start as usize..]);
        }
        Cow::Owned(out)
    }

    /// Decodes one level of one frame to RGBA8 (§2.11), at the level's working dimensions (a DXT texture is at
    /// least 4 × 4). When the level's size does not cover every 4 × 4 block (a DXT width that is not a multiple of
    /// 4), the whole blocks are read from the level's start, so the missing bytes are the record's following
    /// bytes, as DX7 copies them (§2.8, FS7); only bytes past `size` read as zero. The image is cropped to the
    /// level's width. A `0x7B` frame without a palette (a later frame, FS4) decodes as grey, (i, i, i, 255).
    pub fn decode(&self, frame: usize, level: usize) -> Result<Image> {
        let missing = || corrupt("FSH", format!("no frame {frame} level {level}"));
        let f = self.frames.get(frame).ok_or_else(missing)?;
        let l = self.levels.get(level).ok_or_else(missing)?;
        let need = pixel_bytes(f.header.code, l.width, l.height).ok_or_else(missing)?;
        let bytes = self.record_bytes(u64::from(f.pixel_start()) + l.offset, need.max(l.size));
        let grey: Vec<u8>;
        let palette = match f.palette {
            Some(p) => {
                let start = p.offset as usize + ENTRY_HEADER_LEN;
                Some(PaletteData {
                    code: p.code,
                    entries: &self.data[start..start + 256 * p.entry_size()],
                })
            }
            None if f.header.code == code::INDEX8 => {
                // FS4: the original uploads the indices as luminance; read unsigned, that is a grey ramp.
                grey = (0..=255u8).flat_map(|i| [i, i, i]).collect();
                Some(PaletteData {
                    code: code::PALETTE_888,
                    entries: &grey,
                })
            }
            None => None,
        };
        let rgba = decode_pixels(f.header.code, l.width, l.height, &bytes, palette)?;
        Ok(Image {
            width: l.width,
            height: l.height,
            rgba,
        })
    }

    /// The blocks attached to a frame's entry, following `next` from block to block until `next` is 0 or a block
    /// header would leave the record (for inspection: the game follows the chain only for codes `0x7B`/`0x7A`).
    pub fn attachments(&self, frame: usize) -> Vec<Attachment> {
        let mut out = Vec::new();
        let Some(f) = self.frames.get(frame) else {
            return out;
        };
        let (mut at, mut next) = (u64::from(f.offset), f.header.next);
        while next != 0 {
            at += u64::from(next);
            let Some(h) = EntryHeader::parse(self.data.get(at as usize..).unwrap_or(&[])) else {
                break;
            };
            out.push(Attachment {
                offset: at as u32,
                code: h.code,
                next: h.next,
            });
            next = h.next;
        }
        out
    }
}

/// The palette of the `0x7B` entry at `entry` (frame `i`), checked as §2.6 checks frame 0's: found by the §2.5
/// search, exactly 256 × 1, and *\[choice\]* its 256 entries inside the record (`data` is the first `size` bytes).
fn index8_palette(data: &[u8], entry: u32, i: usize) -> Result<Palette> {
    let p =
        find_palette(data, entry)?.ok_or_else(|| corrupt("FSH", format!("frame {i}: code 0x7B without a palette")))?;
    if (p.width, p.height) != (256, 1) {
        return Err(corrupt(
            "FSH",
            format!("frame {i}: palette is {} × {}, not 256 × 1", p.width, p.height),
        ));
    }
    let end = u64::from(p.offset) + ENTRY_HEADER_LEN as u64 + 256 * p.entry_size() as u64;
    if end > data.len() as u64 {
        return Err(Error::OutOfBounds {
            what: "FSH palette",
            offset: u64::from(p.offset),
            size: end - u64::from(p.offset),
            limit: data.len() as u64,
        });
    }
    Ok(p)
}

/// The §2.5 palette search from the entry at `entry`: block to block along `next` (each relative to the current
/// block) until a palette code, or `next` = 0 (no palette). *\[choice\]* A block header outside the record fails
/// the read. The chain always moves forward (`next` > 0), so it ends.
fn find_palette(data: &[u8], entry: u32) -> Result<Option<Palette>> {
    let (mut at, mut next) = (
        u64::from(entry),
        EntryHeader::parse(&data[entry as usize..]).map_or(0, |h| h.next),
    );
    while next != 0 {
        at += u64::from(next);
        let b = data
            .get(at as usize..)
            .and_then(|b| b.get(..ENTRY_HEADER_LEN))
            .ok_or(Error::OutOfBounds {
                what: "FSH attached block",
                offset: at,
                size: ENTRY_HEADER_LEN as u64,
                limit: data.len() as u64,
            })?;
        if code::PALETTES.contains(&b[0]) {
            let u16_at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
            return Ok(Some(Palette {
                offset: at as u32,
                code: b[0],
                width: u16_at(4),
                height: u16_at(6),
                field_10: u16_at(10),
            }));
        }
        next = u32::from_le_bytes([b[1], b[2], b[3], 0]);
    }
    Ok(None)
}

/// A palette for [`decode_pixels`]: its code (`0x24`, `0x2A`, `0x2D`, `0x29`) and its entries.
#[derive(Clone, Copy, Debug)]
pub struct PaletteData<'a> {
    pub code: u8,
    pub entries: &'a [u8],
}

/// Bytes [`decode_pixels`] reads for a code at these dimensions: whole 4 × 4 blocks for DXT (8 or 16 bytes each),
/// else bytes per pixel × width × height. `None` for a code it does not decode.
pub fn pixel_bytes(code: u8, width: u32, height: u32) -> Option<u64> {
    let (w, h) = (u64::from(width), u64::from(height));
    let blocks = w.div_ceil(4) * h.div_ceil(4);
    Some(match code {
        code::DXT1 => blocks * 8,
        code::DXT3 | code::DXT5 => blocks * 16,
        code::A8 | code::INDEX8 => w * h,
        code::L8A8 | code::ARGB4444 | code::RGB565 | code::ARGB1555 => 2 * w * h,
        code::RGB888 => 3 * w * h,
        code::ARGB8888 => 4 * w * h,
        _ => return None,
    })
}

/// Decodes pixels of an accepted code to RGBA8, rows top to bottom (§2.11). `pixels` must hold at least
/// [`pixel_bytes`] bytes; code `0x7B` needs a palette of 256 entries.
///
/// - `0x60` DXT1: 5:6:5 colours expanded by bit replication; c0 > c1 (as `u16`) gives c2 = (2c0 + c1) / 3 and
///   c3 = (c0 + 2c1) / 3, otherwise c2 = (c0 + c1) / 2 and c3 transparent black, whatever A says: a Direct3D
///   DXT1 surface, which DX7 makes for both DXT1 formats, always decodes 3-colour index 3 as (0, 0, 0, 0) (GF3);
///   divisions on the 8-bit values, rounded down (FS8: the GPU does it in the original, nothing fixes it).
/// - `0x61` DXT3: 4-bit alpha (pixel 0 in the low nibble of byte 0) × 17, colour block always in 4-colour mode.
/// - `0x62` DXT5: interpolated alpha (6 or 4 interpolated values with 0 and 255), colour block in 4-colour mode.
/// - `0x7D` B, G, R, A; `0x7F` B, G, R and alpha 255; `0x6D` A4R4G4B4 × 17; `0x7E` A1R5G5B5; `0x78` R5G6B5
///   (the Direct3D layouts FSH tools write *\[choice\]*: no renderer of the original shows these three
///   correctly, GF2); `0x64` (255, 255, 255, a); `0x65` (L, L, L, a); `0x7B` through the palette (alpha 255
///   unless the palette has alpha).
pub fn decode_pixels(
    code: u8,
    width: u32,
    height: u32,
    pixels: &[u8],
    palette: Option<PaletteData<'_>>,
) -> Result<Vec<u8>> {
    let need = pixel_bytes(code, width, height)
        .ok_or_else(|| corrupt("FSH decode", format!("code {code:#04X} has no decoder")))?;
    if (pixels.len() as u64) < need {
        return Err(Error::Truncated {
            what: "FSH pixels",
            needed: need,
            available: pixels.len() as u64,
        });
    }
    let (w, h) = (width as usize, height as usize);
    let mut out = vec![0u8; w * h * 4];
    match code {
        code::DXT1 | code::DXT3 | code::DXT5 => {
            let block_len = if code == code::DXT1 { 8 } else { 16 };
            let bw = w.div_ceil(4);
            for (b, block) in pixels.chunks_exact(block_len).take(bw * h.div_ceil(4)).enumerate() {
                let texels = match code {
                    code::DXT1 => colour_block(block, false),
                    code::DXT3 => {
                        let mut t = colour_block(&block[8..], true);
                        for (i, px) in t.iter_mut().enumerate() {
                            px[3] = ((block[i / 2] >> (4 * (i % 2))) & 0xF) * 17;
                        }
                        t
                    }
                    _ => {
                        let mut t = colour_block(&block[8..], true);
                        let alphas = dxt5_alphas(block[0], block[1]);
                        let mut bits = 0u64;
                        for (k, &byte) in block[2..8].iter().enumerate() {
                            bits |= u64::from(byte) << (8 * k);
                        }
                        for (i, px) in t.iter_mut().enumerate() {
                            px[3] = alphas[((bits >> (3 * i)) & 7) as usize];
                        }
                        t
                    }
                };
                let (bx, by) = ((b % bw) * 4, (b / bw) * 4);
                for (i, px) in texels.iter().enumerate() {
                    let (x, y) = (bx + i % 4, by + i / 4);
                    if x < w && y < h {
                        let o = (y * w + x) * 4;
                        out[o..o + 4].copy_from_slice(px);
                    }
                }
            }
        }
        _ => {
            let entry = |i: usize| -> Result<[u8; 4]> {
                let p = palette.ok_or_else(|| corrupt("FSH decode", "code 0x7B without a palette"))?;
                let es = palette_entry_size(p.code);
                let e = p
                    .entries
                    .get(i * es..i * es + es)
                    .ok_or_else(|| corrupt("FSH decode", format!("palette has no entry {i}")))?;
                Ok(match p.code {
                    code::PALETTE_8888 => [e[2], e[1], e[0], e[3]],
                    code::PALETTE_888 => [e[2], e[1], e[0], 255],
                    code::PALETTE_1555 => argb1555(u16::from_le_bytes([e[0], e[1]])),
                    _ => rgb565_px(u16::from_le_bytes([e[0], e[1]])),
                })
            };
            for i in 0..w * h {
                let u16_at = |o: usize| u16::from_le_bytes([pixels[o], pixels[o + 1]]);
                let px = match code {
                    code::ARGB8888 => {
                        let p = &pixels[i * 4..i * 4 + 4];
                        [p[2], p[1], p[0], p[3]]
                    }
                    code::RGB888 => {
                        let p = &pixels[i * 3..i * 3 + 3];
                        [p[2], p[1], p[0], 255]
                    }
                    code::ARGB4444 => {
                        let v = u16_at(i * 2);
                        let n = |s: u16| ((v >> s) & 0xF) as u8 * 17;
                        [n(8), n(4), n(0), n(12)]
                    }
                    code::ARGB1555 => argb1555(u16_at(i * 2)),
                    code::RGB565 => rgb565_px(u16_at(i * 2)),
                    code::A8 => [255, 255, 255, pixels[i]],
                    code::L8A8 => {
                        let l = pixels[i * 2];
                        [l, l, l, pixels[i * 2 + 1]]
                    }
                    _ => entry(usize::from(pixels[i]))?,
                };
                out[i * 4..i * 4 + 4].copy_from_slice(&px);
            }
        }
    }
    Ok(out)
}

/// A 5-bit channel expanded to 8 bits by bit replication.
fn expand5(v: u16) -> u8 {
    let v = (v & 31) as u8;
    (v << 3) | (v >> 2)
}

/// A 6-bit channel expanded to 8 bits by bit replication.
fn expand6(v: u16) -> u8 {
    let v = (v & 63) as u8;
    (v << 2) | (v >> 4)
}

fn rgb565(c: u16) -> [u8; 3] {
    [expand5(c >> 11), expand6(c >> 5), expand5(c)]
}

fn rgb565_px(c: u16) -> [u8; 4] {
    let [r, g, b] = rgb565(c);
    [r, g, b, 255]
}

fn argb1555(c: u16) -> [u8; 4] {
    [
        expand5(c >> 10),
        expand5(c >> 5),
        expand5(c),
        if c & 0x8000 != 0 { 255 } else { 0 },
    ]
}

/// The 16 texels of an 8-byte S3TC colour block, row-major. `always_four` is the DXT3/DXT5 rule; DXT1 uses the
/// 3-colour mode with transparent black when c0 ≤ c1.
fn colour_block(b: &[u8], always_four: bool) -> [[u8; 4]; 16] {
    let c0 = u16::from_le_bytes([b[0], b[1]]);
    let c1 = u16::from_le_bytes([b[2], b[3]]);
    let (p0, p1) = (rgb565(c0), rgb565(c1));
    let mix = |a: u16, b: u16, d: u16| -> [u8; 4] {
        let ch = |i: usize| ((a * u16::from(p0[i]) + b * u16::from(p1[i])) / d) as u8;
        [ch(0), ch(1), ch(2), 255]
    };
    let palette = if always_four || c0 > c1 {
        [
            [p0[0], p0[1], p0[2], 255],
            [p1[0], p1[1], p1[2], 255],
            mix(2, 1, 3),
            mix(1, 2, 3),
        ]
    } else {
        [
            [p0[0], p0[1], p0[2], 255],
            [p1[0], p1[1], p1[2], 255],
            mix(1, 1, 2),
            // GF3: what the Direct3D DXT1 surface gives, whatever A says.
            [0, 0, 0, 0],
        ]
    };
    let idx = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    let mut t = [[0u8; 4]; 16];
    for (i, px) in t.iter_mut().enumerate() {
        *px = palette[((idx >> (2 * i)) & 3) as usize];
    }
    t
}

/// The eight DXT5 alpha values for a0, a1 (§2.11), divisions rounded down *\[choice\]* (FS8; no DXT5 is shipped).
fn dxt5_alphas(a0: u8, a1: u8) -> [u8; 8] {
    let (x, y) = (u32::from(a0), u32::from(a1));
    let mut a = [a0, a1, 0, 0, 0, 0, 0, 0];
    if a0 > a1 {
        for (i, v) in a.iter_mut().enumerate().skip(2) {
            let i = i as u32;
            *v = (((8 - i) * x + (i - 1) * y) / 7) as u8;
        }
    } else {
        for (i, v) in a.iter_mut().enumerate().take(6).skip(2) {
            let i = i as u32;
            *v = (((6 - i) * x + (i - 1) * y) / 5) as u8;
        }
        a[6] = 0;
        a[7] = 255;
    }
    a
}

#[cfg(test)]
mod tests;
