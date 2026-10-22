// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

//! Span renderer for A4R4G4B4 targets (`nGZDrawSoftRender::cSpanRender4444`).
//!
//! Written only from `docs/specs/span-render-4444.md`; `§n` references point at
//! that spec. The five render entry points (§3-§7) and the 14 x 12 blit kernel
//! table (§8, §9) are modelled as pure functions over byte buffers.
//!
//! Bounded API: the original works on raw pointers with no clipping and no
//! range checks. Here every buffer is a byte slice with an origin offset and a
//! signed byte stride, and every pixel address a call would touch (destination
//! and source) is checked **before anything is written**: a call either
//! renders every span or returns an error and leaves the target untouched.
//! Where the original's behaviour on bad input is undefined or pathological
//! (count 0, U1; unchecked table indices, U3; zero tile size, U7) this module
//! returns an error instead and says so on the error variant. Those error
//! paths are a choice of this API, not verified behaviour of the original.

mod blt;
mod methods;
#[cfg(test)]
mod tests;

pub use blt::{
    blt_conversion_4444, blt_widen_8888, render_blt, render_blt_raw, BltDesc, Geometry, Kernel, Op, SourceFormat,
    SourceImage, KEY_MASKS,
};
pub use methods::{
    alpha_gradient_weight, render_alpha, render_alpha_gradient, render_alpha_weight, render_gradient, render_solid,
    Gradient,
};

/// Errors of the bounded API. None of these exist in the original (§2, §7):
/// they mark inputs for which the original's behaviour is undefined, a fault
/// or a memory overrun.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanError {
    /// Empty span list. The original's decrement-then-test loop would run
    /// 2^32 iterations for count 0 (§2, U1); not modelled.
    EmptySpanList,
    /// A destination pixel of span `span` lies outside the target slice.
    DestOutOfBounds { span: usize },
    /// A source texel read by span `span` lies outside the source slice
    /// (stretch walks are not clamped by the original, U5).
    SourceOutOfBounds { span: usize },
    /// `format * 2 + flag` >= 14 or `mode` >= 12: the original indexes the
    /// kernel table without a check (U3).
    BadKernelIndex { row: u32, mode: u32 },
    /// Tiled kernel with source width or height 0: the original's `DIV`
    /// faults (U7).
    ZeroTileSize,
}

/// One span record (§2): two dwords packed `(y << 16) | x`. Width is
/// `end - start` on the whole dwords (wrapping), exclusive end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    /// Span of `width` pixels from `(x, y)`, packed as the original packs it.
    pub fn new(x: u16, y: u16, width: u16) -> Span {
        let start = ((y as u32) << 16) | x as u32;
        Span {
            start,
            end: start.wrapping_add(width as u32),
        }
    }
    pub fn x(self) -> u32 {
        self.start & 0xFFFF
    }
    pub fn y(self) -> u32 {
        self.start >> 16
    }
    /// `end - start` on the whole dwords (§2). A record whose `end` is on an
    /// earlier position than `start` yields a huge width, which the bounded
    /// API rejects as out of bounds.
    pub fn width(self) -> u32 {
        self.end.wrapping_sub(self.start)
    }
}

/// 16-bit render target (§2 "Target descriptor", §7 `rectA`): pixel (x, y)
/// at `origin + y*stride + 2*x` bytes into `data`. Pixels are little-endian.
pub struct Target<'a> {
    pub data: &'a mut [u8],
    /// Byte offset of pixel (0, 0) in `data` (the descriptor's base pointer).
    pub origin: usize,
    /// Row stride in bytes; may be negative or odd (unaligned rows are
    /// allowed: pixels are read and written as two bytes).
    pub stride: i32,
}

/// Byte offset of `origin + y*stride + bpp*x` if `len` bytes from it fit in
/// a buffer of `size` bytes. Address maths in i128 so nothing wraps.
pub(crate) fn offset(origin: usize, stride: i32, x: u64, y: u64, bpp: u64, len: u64, size: usize) -> Option<usize> {
    let off = origin as i128 + y as i128 * stride as i128 + (bpp as i128) * (x as i128);
    if off < 0 || off + len as i128 > size as i128 {
        None
    } else {
        Some(off as usize)
    }
}

impl Target<'_> {
    /// Byte offset of the first pixel of `span`, checking the whole span
    /// (`2*width` bytes, linear from the start pixel) fits.
    pub(crate) fn span_offset(&self, span: Span, index: usize) -> Result<usize, SpanError> {
        offset(
            self.origin,
            self.stride,
            span.x() as u64,
            span.y() as u64,
            2,
            2 * span.width() as u64,
            self.data.len(),
        )
        .ok_or(SpanError::DestOutOfBounds { span: index })
    }

    /// Checks every span of the list; returns the start offsets.
    pub(crate) fn check_spans(&self, spans: &[Span]) -> Result<Vec<usize>, SpanError> {
        if spans.is_empty() {
            return Err(SpanError::EmptySpanList);
        }
        spans.iter().enumerate().map(|(i, s)| self.span_offset(*s, i)).collect()
    }

    pub(crate) fn get(&self, off: usize) -> u16 {
        u16::from_le_bytes([self.data[off], self.data[off + 1]])
    }

    pub(crate) fn put(&mut self, off: usize, px: u16) {
        self.data[off..off + 2].copy_from_slice(&px.to_le_bytes());
    }
}

/// Packed 4444 blend (§6.1, SP1): red/blue share a wrapping dword;
/// green is separate. Logical shifts and final masks preserve carries/borrows
/// between blue and red at weights 17..31. Source/destination alpha is ignored.
pub fn blend4444(s: u16, d: u16, a: u32) -> u16 {
    let (srb, drb) = (u32::from(s & 0x0F0F), u32::from(d & 0x0F0F));
    let (sg, dg) = (u32::from(s & 0x00F0), u32::from(d & 0x00F0));
    let rb = srb
        .wrapping_sub(drb)
        .wrapping_mul(a)
        .wrapping_add(16 * drb)
        .wrapping_add(0x808);
    let g = sg.wrapping_sub(dg).wrapping_mul(a).wrapping_add(16 * (dg + 8));
    (0xF000 | ((rb >> 4) & 0x0F0F) | ((g >> 4) & 0x00F0)) as u16
}
