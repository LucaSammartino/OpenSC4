// SPDX-License-Identifier: GPL-3.0-or-later
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

//! C ABI of the 565 span renderer (`docs/specs/span-render-565.md`), for the
//! plugin's bridge of the native vtable `0x00AC1844` (§1.1). The five entry
//! points take the native records as the original methods receive them and
//! run `fc4_core::span_render565` on a typed memory window: the exact address
//! hull of every destination pixel and source texel the call touches
//! (`dest_extent`, `blt_source_extent`), so the core's bounds checks never
//! fire on input the original would draw. Bytes inside the hull that the call
//! does not address are never read or written.
//!
//! Return value: `FC4_SPAN565_OK` (0) when ours drew the call, otherwise a
//! decline code and nothing was written. The bridge forwards a declined call
//! to the native method, which is what the original does with that input
//! (§1: `n = 0` walks 2³² records; an invalid span, an index outside the
//! table or a tiled W/H of 0 has no defined result in the spec).

use fc4_core::span_render565::{self as sr, BltParams, Dest, Gradient, Source, Span, SpanError};

pub const FC4_SPAN565_OK: i32 = 0;
pub const FC4_SPAN565_EMPTY_LIST: i32 = 1;
pub const FC4_SPAN565_INVALID_SPAN: i32 = 2;
pub const FC4_SPAN565_SLOT_OUTSIDE_TABLE: i32 = 3;
pub const FC4_SPAN565_TILED_ZERO_SIZE: i32 = 4;
/// The hull is outside the address space or larger than `FC4_SPAN565_MAX_WINDOW`.
pub const FC4_SPAN565_WINDOW: i32 = 5;
/// A null record pointer.
pub const FC4_SPAN565_NULL: i32 = 6;
/// More than `FC4_SPAN565_MAX_SPANS` records.
pub const FC4_SPAN565_TOO_MANY: i32 = 7;

/// Largest window ours draws into (256 MiB); a larger hull is declined.
pub const FC4_SPAN565_MAX_WINDOW: i64 = 1 << 28;
/// Largest span count ours copies (the list is copied before drawing).
pub const FC4_SPAN565_MAX_SPANS: u32 = 1 << 20;

/// Destination record (§1, 24 bytes on x86): only `pixels` and `stride` are read.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct fc4_span565_dest {
    pub pixels: *mut u8,
    pub stride: i32,
    pub width: u32,
    pub height: u32,
    pub color_type: u32,
    pub bytes_per_pixel: u32,
}

/// RenderBlt `src` record (§6, 24 bytes on x86): texel (0, 0), stride, W, H;
/// the last two dwords are not read by this class.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct fc4_span565_source {
    pub texels: *const u8,
    pub stride: i32,
    pub width: u32,
    pub height: u32,
    pub unread: [u32; 2],
}

/// RenderBlt `b` record (§6, 32 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct fc4_span565_blt {
    pub u0: u32,
    pub v0: u32,
    pub du: u32,
    pub dv: u32,
    pub format: u32,
    pub argb: u32,
    pub key: u32,
    pub key_flag: u8,
    pub pad: [u8; 3],
}

fn code(e: SpanError) -> i32 {
    match e {
        SpanError::EmptySpanList => FC4_SPAN565_EMPTY_LIST,
        SpanError::InvalidSpan { .. } => FC4_SPAN565_INVALID_SPAN,
        SpanError::SlotOutsideTable { .. } => FC4_SPAN565_SLOT_OUTSIDE_TABLE,
        SpanError::TiledZeroSize => FC4_SPAN565_TILED_ZERO_SIZE,
        // Cannot happen inside the hull; reported as a window failure.
        SpanError::OutOfBounds { .. } => FC4_SPAN565_WINDOW,
    }
}

/// The span list as the core's records (`+0` start, `+4` end).
unsafe fn read_spans(spans: *const u32, n: u32) -> Result<Vec<Span>, i32> {
    if n == 0 {
        return Err(FC4_SPAN565_EMPTY_LIST);
    }
    if spans.is_null() {
        return Err(FC4_SPAN565_NULL);
    }
    if n > FC4_SPAN565_MAX_SPANS {
        return Err(FC4_SPAN565_TOO_MANY);
    }
    let raw = std::slice::from_raw_parts(spans, 2 * n as usize);
    Ok(raw.chunks_exact(2).map(|p| Span { start: p[0], end: p[1] }).collect())
}

/// Union of absolute `[lo, hi)` ranges.
fn hull(a: Option<(i64, i64)>, b: Option<(i64, i64)>) -> Option<(i64, i64)> {
    match (a, b) {
        (Some((l0, h0)), Some((l1, h1))) => Some((l0.min(l1), h0.max(h1))),
        (x, None) | (None, x) => x,
    }
}

fn shift(e: Option<(i64, i64)>, base: usize) -> Option<(i64, i64)> {
    e.map(|(lo, hi)| (lo + base as i64, hi + base as i64))
}

/// Runs `f` on the window `[lo, hi)` (absolute addresses) with the window's
/// start as offset 0. `None` hull: nothing to touch, `f` gets an empty slice.
unsafe fn with_window(win: Option<(i64, i64)>, f: impl FnOnce(&mut [u8], i64) -> Result<(), SpanError>) -> i32 {
    let (lo, hi) = win.unwrap_or((0, 0));
    if lo < 0 || hi < lo || hi - lo > FC4_SPAN565_MAX_WINDOW || usize::try_from(hi).is_err() {
        return FC4_SPAN565_WINDOW;
    }
    let mem: &mut [u8] = if hi == lo {
        &mut []
    } else {
        // SAFETY: the hull of the bytes the original method addresses for
        // this input; the caller (the native bridge) passes live records.
        std::slice::from_raw_parts_mut(lo as usize as *mut u8, (hi - lo) as usize)
    };
    match f(mem, lo) {
        Ok(()) => FC4_SPAN565_OK,
        Err(e) => code(e),
    }
}

unsafe fn untextured(
    spans: *const u32,
    n: u32,
    dst: *const fc4_span565_dest,
    draw: impl FnOnce(&mut [u8], &[Span], &Dest) -> Result<(), SpanError>,
) -> i32 {
    let list = match read_spans(spans, n) {
        Ok(l) => l,
        Err(c) => return c,
    };
    if dst.is_null() {
        return FC4_SPAN565_NULL;
    }
    let d = &*dst;
    let rel = Dest {
        base: 0,
        stride: d.stride,
    };
    let ext = match sr::dest_extent(&list, &rel) {
        Ok(e) => e,
        Err(e) => return code(e),
    };
    let base = d.pixels as usize;
    with_window(shift(ext, base), |mem, lo| {
        draw(
            mem,
            &list,
            &Dest {
                base: base as i64 - lo,
                stride: d.stride,
            },
        )
    })
}

unsafe fn gradient(g: *const u32) -> Option<Gradient> {
    if g.is_null() {
        return None;
    }
    let w = std::slice::from_raw_parts(g, 12);
    let mut r = Gradient::default();
    r.c.copy_from_slice(&w[0..4]);
    r.dx.copy_from_slice(&w[4..8]);
    r.dy.copy_from_slice(&w[8..12]);
    Some(r)
}

/// Slot 1, RenderSolid (§2).
///
/// # Safety
/// `spans` holds `n` 8-byte records, `dst` a live record whose pixels cover
/// every span (as for the original method).
#[no_mangle]
pub unsafe extern "C" fn fc4_span565_render_solid(
    spans: *const u32,
    n: u32,
    dst: *const fc4_span565_dest,
    colour: u32,
) -> i32 {
    untextured(spans, n, dst, |m, l, d| sr::render_solid(m, l, d, colour))
}

/// Slot 2, RenderGradient (§4); `grad` = the 12-dword record (§4.1).
///
/// # Safety
/// As `fc4_span565_render_solid`; `grad` points at 12 dwords.
#[no_mangle]
pub unsafe extern "C" fn fc4_span565_render_gradient(
    spans: *const u32,
    n: u32,
    dst: *const fc4_span565_dest,
    grad: *const u32,
) -> i32 {
    let Some(g) = gradient(grad) else {
        return FC4_SPAN565_NULL;
    };
    untextured(spans, n, dst, |m, l, d| sr::render_gradient(m, l, d, &g))
}

/// Slot 3, RenderAlpha (§3).
///
/// # Safety
/// As `fc4_span565_render_solid`.
#[no_mangle]
pub unsafe extern "C" fn fc4_span565_render_alpha(
    spans: *const u32,
    n: u32,
    dst: *const fc4_span565_dest,
    colour: u32,
    argb: u32,
) -> i32 {
    untextured(spans, n, dst, |m, l, d| sr::render_alpha(m, l, d, colour, argb))
}

/// Slot 4, RenderAlphaGradient (§5).
///
/// # Safety
/// As `fc4_span565_render_gradient`.
#[no_mangle]
pub unsafe extern "C" fn fc4_span565_render_alpha_gradient(
    spans: *const u32,
    n: u32,
    dst: *const fc4_span565_dest,
    grad: *const u32,
) -> i32 {
    let Some(g) = gradient(grad) else {
        return FC4_SPAN565_NULL;
    };
    untextured(spans, n, dst, |m, l, d| sr::render_alpha_gradient(m, l, d, &g))
}

/// Slot 5, RenderBlt (§6): one window over the destination and source hulls.
///
/// # Safety
/// As `fc4_span565_render_solid`; `src` and `b` live records whose texels
/// cover every address the slot reads (as for the original method).
#[no_mangle]
pub unsafe extern "C" fn fc4_span565_render_blt(
    spans: *const u32,
    n: u32,
    dst: *const fc4_span565_dest,
    src: *const fc4_span565_source,
    b: *const fc4_span565_blt,
    kind: u32,
) -> i32 {
    let list = match read_spans(spans, n) {
        Ok(l) => l,
        Err(c) => return c,
    };
    if dst.is_null() || src.is_null() || b.is_null() {
        return FC4_SPAN565_NULL;
    }
    let (d, s, bb) = (&*dst, &*src, &*b);
    let p = BltParams {
        u0: bb.u0,
        v0: bb.v0,
        du: bb.du,
        dv: bb.dv,
        format: bb.format,
        argb: bb.argb,
        key: bb.key,
        key_flag: bb.key_flag,
    };
    let rel_src = Source {
        base: 0,
        stride: s.stride,
        width: s.width,
        height: s.height,
    };
    // The table index is checked before any extent (as render_blt does).
    let sext = match sr::blt_source_extent(&list, &rel_src, &p, kind) {
        Ok(e) => e,
        Err(e) => return code(e),
    };
    let dext = match sr::dest_extent(
        &list,
        &Dest {
            base: 0,
            stride: d.stride,
        },
    ) {
        Ok(e) => e,
        Err(e) => return code(e),
    };
    let (dbase, sbase) = (d.pixels as usize, s.texels as usize);
    let win = hull(shift(dext, dbase), shift(sext, sbase));
    with_window(win, |mem, lo| {
        let dd = Dest {
            base: dbase as i64 - lo,
            stride: d.stride,
        };
        let ss = Source {
            base: sbase as i64 - lo,
            ..rel_src
        };
        sr::render_blt(mem, &list, &dd, &ss, &p, kind)
    })
}
