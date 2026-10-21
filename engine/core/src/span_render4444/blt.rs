// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors. See LICENSE at the repository root.

//! RenderBlt (slot 5, §7) and its 14 x 12 kernel table (§8, §9).
//!
//! The 168 kernels are not separate functions here: §9.5 verified that every
//! table entry behaves as the model of §9.1-§9.4 (row = source format x
//! keyed, column = geometry x op), so one parameterised kernel covers all.

use super::{blend4444, offset, Span, SpanError, Target};

/// Source pixel formats, the `src+0x10` row index (§8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    A4R4G4B4 = 0,
    A1R5G5B5 = 1,
    X1R5G5B5 = 2,
    R5G6B5 = 3,
    A8R8G8B8 = 4,
    X8R8G8B8 = 5,
    A8 = 6,
}

impl SourceFormat {
    pub const ALL: [SourceFormat; 7] = [
        SourceFormat::A4R4G4B4,
        SourceFormat::A1R5G5B5,
        SourceFormat::X1R5G5B5,
        SourceFormat::R5G6B5,
        SourceFormat::A8R8G8B8,
        SourceFormat::X8R8G8B8,
        SourceFormat::A8,
    ];

    pub fn from_index(i: u32) -> Option<SourceFormat> {
        Self::ALL.get(i as usize).copied()
    }

    /// Bytes per source texel (§8).
    pub fn bytes_per_pixel(self) -> u32 {
        match self {
            SourceFormat::A8R8G8B8 | SourceFormat::X8R8G8B8 => 4,
            SourceFormat::A8 => 1,
            _ => 2,
        }
    }

    /// Colour-key mask (§9.4 "Keyed rows").
    pub fn key_mask(self) -> u32 {
        KEY_MASKS[self as usize]
    }
}

/// Per-format colour-key masks, indexed by `SourceFormat` (§9.4). A8 has an
/// empty mask, so its key test is "key == 0".
pub const KEY_MASKS: [u32; 7] = [0x0FFF, 0x7FFF, 0x7FFF, 0xFFFF, 0xFF_FFFF, 0xFF_FFFF, 0];

/// Column geometry, `mode / 4` (§8, §9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Geometry {
    Direct = 0,
    Stretch = 1,
    Tiled = 2,
}

/// Column operation, `mode % 4` (§8, §9.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Copy = 0,
    ConstantAlpha = 1,
    Modulate = 2,
    ModulateAlpha = 3,
}

/// The 32-byte source descriptor (§7 `src`). Only the fields the kernels
/// read are modelled (U6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BltDesc {
    /// `+0x00`: 16.16 start u (stretch) or integer x offset (tiled).
    pub u0: u32,
    /// `+0x04`: 16.16 start v (stretch) or integer y offset (tiled).
    pub v0: u32,
    /// `+0x08`: 16.16 du/dx (stretch only).
    pub dudx: u32,
    /// `+0x0C`: 16.16 dv/dy (stretch only).
    pub dvdy: u32,
    /// `+0x10`: source format (row selector, read only by RenderBlt).
    pub format: u32,
    /// `+0x14`: constant colour / alpha, ARGB 8888.
    pub colour: u32,
    /// `+0x18`: colour key.
    pub key: u32,
    /// `+0x1C`: keyed flag byte.
    pub keyed: u8,
}

/// The source image (§7 `rectB`): texel (x, y) at
/// `origin + y*stride + bpp*x` bytes into `data`, little-endian.
#[derive(Debug, Clone, Copy)]
pub struct SourceImage<'a> {
    pub data: &'a [u8],
    pub origin: usize,
    pub stride: i32,
    /// `+0x08`, read only by tiled kernels.
    pub width: u32,
    /// `+0x0C`, read only by tiled kernels.
    pub height: u32,
}

/// §9.2: texel to 4444 by truncation to the top 4 bits of each channel.
pub fn blt_conversion_4444(fmt: SourceFormat, t: u32) -> u16 {
    let n = |v: u32, shift: u32| ((v >> shift) & 0xF) as u16;
    let (a, r, g, b) = match fmt {
        SourceFormat::A4R4G4B4 => return t as u16,
        SourceFormat::A1R5G5B5 | SourceFormat::X1R5G5B5 => {
            let a = if fmt == SourceFormat::X1R5G5B5 || t & 0x8000 != 0 {
                0xF
            } else {
                0
            };
            (a, n(t, 11), n(t, 6), n(t, 1))
        }
        SourceFormat::R5G6B5 => (0xF, n(t, 12), n(t, 7), n(t, 1)),
        SourceFormat::A8R8G8B8 => (n(t, 28), n(t, 20), n(t, 12), n(t, 4)),
        SourceFormat::X8R8G8B8 => (0xF, n(t, 20), n(t, 12), n(t, 4)),
        SourceFormat::A8 => (n(t, 4), 0xF, 0xF, 0xF),
    };
    (a << 12) | (r << 8) | (g << 4) | b
}

/// §9.3: texel to ARGB 8888 by bit replication.
pub fn blt_widen_8888(fmt: SourceFormat, t: u32) -> u32 {
    let w4 = |v: u32, s: u32| ((v >> s) & 0xF) * 17;
    let w5 = |v: u32, s: u32| {
        let c = (v >> s) & 0x1F;
        (c << 3) | (c >> 2)
    };
    let (a, r, g, b) = match fmt {
        SourceFormat::A4R4G4B4 => (w4(t, 12), w4(t, 8), w4(t, 4), w4(t, 0)),
        SourceFormat::A1R5G5B5 | SourceFormat::X1R5G5B5 => {
            let a = if fmt == SourceFormat::X1R5G5B5 || t & 0x8000 != 0 {
                0xFF
            } else {
                0
            };
            (a, w5(t, 10), w5(t, 5), w5(t, 0))
        }
        SourceFormat::R5G6B5 => {
            let g = (t >> 5) & 0x3F;
            (0xFF, w5(t, 11), (g << 2) | (g >> 4), w5(t, 0))
        }
        SourceFormat::A8R8G8B8 => return t,
        SourceFormat::X8R8G8B8 => return t | 0xFF00_0000,
        SourceFormat::A8 => (t & 0xFF, 0xFF, 0xFF, 0xFF),
    };
    (a << 24) | (r << 16) | (g << 8) | b
}

/// §9.4 modulate: per byte `(C * K + 128) >> 8`.
fn modulate(c: u32, k: u32) -> u32 {
    let mut m = 0;
    for shift in [0u32, 8, 16, 24] {
        let cb = (c >> shift) & 0xFF;
        let kb = (k >> shift) & 0xFF;
        m |= (((cb * kb + 128) >> 8) & 0xFF) << shift;
    }
    m
}

/// Top nibble of each byte of an ARGB 8888 value, as a 4444 pixel.
fn top_nibbles(m: u32) -> u16 {
    (((m >> 16) & 0xF000) | ((m >> 12) & 0x0F00) | ((m >> 8) & 0x00F0) | ((m >> 4) & 0x000F)) as u16
}

/// One destination pixel from texel `t` and destination `d` (§9.4); `None`
/// leaves the destination untouched (keyed match).
fn kernel_pixel(fmt: SourceFormat, keyed: bool, op: Op, desc: &BltDesc, t: u32, d: u16) -> Option<u16> {
    if keyed && (t & fmt.key_mask()) == desc.key {
        return None;
    }
    Some(match op {
        Op::Copy => blt_conversion_4444(fmt, t),
        Op::ConstantAlpha => blend4444(blt_conversion_4444(fmt, t), d, desc.colour >> 28),
        Op::Modulate => top_nibbles(modulate(blt_widen_8888(fmt, t), desc.colour)),
        Op::ModulateAlpha => {
            let m = modulate(blt_widen_8888(fmt, t), desc.colour);
            blend4444(top_nibbles(m), d, m >> 28)
        }
    })
}

/// Byte offsets of every texel span `index` reads (§9.1).
fn texel_offsets(
    span: Span,
    index: usize,
    img: &SourceImage,
    desc: &BltDesc,
    fmt: SourceFormat,
    geom: Geometry,
) -> Result<Vec<usize>, SpanError> {
    let bpp = fmt.bytes_per_pixel() as u64;
    let w = span.width();
    let (x, y) = (span.x(), span.y());
    let at = |col: u64, row: u64| {
        offset(img.origin, img.stride, col, row, bpp, bpp, img.data.len())
            .ok_or(SpanError::SourceOutOfBounds { span: index })
    };
    let mut out = Vec::with_capacity(w as usize);
    match geom {
        // Direct: texel (x, y), walking linearly along the row.
        Geometry::Direct => {
            for i in 0..w as u64 {
                out.push(at(x as u64 + i, y as u64)?);
            }
        }
        // Stretch: unsigned 16.16 walk, no clamp (U5, U7).
        Geometry::Stretch => {
            let v = desc.dvdy.wrapping_mul(y).wrapping_add(desc.v0) >> 16;
            let mut u = desc.dudx.wrapping_mul(x).wrapping_add(desc.u0);
            for _ in 0..w {
                out.push(at((u >> 16) as u64, v as u64)?);
                u = u.wrapping_add(desc.dudx);
            }
        }
        // Tiled: unsigned 32-bit modulo with integer offsets.
        Geometry::Tiled => {
            if img.width == 0 || img.height == 0 {
                return Err(SpanError::ZeroTileSize);
            }
            let mut tx = x.wrapping_add(desc.u0) % img.width;
            let ty = y.wrapping_add(desc.v0) % img.height;
            for _ in 0..w {
                out.push(at(tx as u64, ty as u64)?);
                tx += 1;
                if tx == img.width {
                    tx = 0;
                }
            }
        }
    }
    Ok(out)
}

fn read_texel(img: &SourceImage, off: usize, bpp: u32) -> u32 {
    let mut v = 0u32;
    for k in 0..bpp as usize {
        v |= (img.data[off + k] as u32) << (8 * k);
    }
    v
}

/// One entry of the 14 x 12 kernel table (§8): row `fmt * 2 + keyed`,
/// column `geom * 4 + op`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kernel {
    pub fmt: SourceFormat,
    pub keyed: bool,
    pub geom: Geometry,
    pub op: Op,
}

impl Kernel {
    /// Table entry at `row` (0..14), `mode` (0..12); `None` outside.
    pub fn from_row_mode(row: u32, mode: u32) -> Option<Kernel> {
        if mode >= 12 {
            return None;
        }
        Some(Kernel {
            fmt: SourceFormat::from_index(row / 2)?,
            keyed: row % 2 == 1,
            geom: [Geometry::Direct, Geometry::Stretch, Geometry::Tiled][(mode / 4) as usize],
            op: [Op::Copy, Op::ConstantAlpha, Op::Modulate, Op::ModulateAlpha][(mode % 4) as usize],
        })
    }
    pub fn row(self) -> u32 {
        self.fmt as u32 * 2 + self.keyed as u32
    }
    pub fn mode(self) -> u32 {
        self.geom as u32 * 4 + self.op as u32
    }
}

/// One kernel of the table (§8, §9). `desc.format` and `desc.keyed` are not
/// consulted (the kernel does not read them, U6).
pub fn render_blt(
    spans: &[Span],
    target: &mut Target,
    img: &SourceImage,
    desc: &BltDesc,
    kernel: Kernel,
) -> Result<(), SpanError> {
    let Kernel { fmt, keyed, geom, op } = kernel;
    let dst = target.check_spans(spans)?;
    let src: Vec<Vec<usize>> = spans
        .iter()
        .enumerate()
        .map(|(i, s)| texel_offsets(*s, i, img, desc, fmt, geom))
        .collect::<Result<_, _>>()?;
    let bpp = fmt.bytes_per_pixel();
    for (doff, soffs) in dst.into_iter().zip(src) {
        for (i, soff) in soffs.into_iter().enumerate() {
            let o = doff + 2 * i;
            let t = read_texel(img, soff, bpp);
            if let Some(px) = kernel_pixel(fmt, keyed, op, desc, t, target.get(o)) {
                target.put(o, px);
            }
        }
    }
    Ok(())
}

/// RenderBlt (slot 5, §7) as the vtable method dispatches: row
/// `desc.format * 2 + desc.keyed` (wrapping, the flag byte added as is),
/// column `mode`. The original does not range-check either index (U3); here
/// row >= 14 or mode >= 12 is `BadKernelIndex` ([choice]). A flag byte other
/// than 0/1 selects whatever row the sum names, as in the original.
pub fn render_blt_raw(
    spans: &[Span],
    target: &mut Target,
    img: &SourceImage,
    desc: &BltDesc,
    mode: u32,
) -> Result<(), SpanError> {
    let row = desc.format.wrapping_mul(2).wrapping_add(desc.keyed as u32);
    let kernel = Kernel::from_row_mode(row, mode).ok_or(SpanError::BadKernelIndex { row, mode })?;
    render_blt(spans, target, img, desc, kernel)
}
