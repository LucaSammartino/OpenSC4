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

//! C ABI over `fc4_core::span_render8888` for the plug-in's cSpanRender8888
//! bridge (glue-span8888-impl4; docs/specs/span-render-8888.md §1.1-§9).
//!
//! Every call takes the destination as a byte region (`dst`, `dst_len`) with
//! pixel (0, 0) at byte `origin` of it and a signed byte `stride`; the core
//! checks every touched byte against the region before it writes anything.
//! Return value: 0 rendered, 1 a null pointer or a zero span count,
//! 2 the core rejected the input (nothing written).

use fc4_core::span_render8888::{self as sr, BltDesc, Gradient, Mem, SourceImage, Span, Surface};

const OK: i32 = 0;
const BAD_ARGS: i32 = 1;
const REJECTED: i32 = 2;

/// `count` (start, end) dword pairs (§2 span record).
unsafe fn spans_of(spans: *const u32, count: u32) -> Option<Vec<Span>> {
    if spans.is_null() || count == 0 {
        return None;
    }
    let raw = std::slice::from_raw_parts(spans, 2 * count as usize);
    Some(raw.chunks_exact(2).map(|p| Span { start: p[0], end: p[1] }).collect())
}

unsafe fn dst_of<'a>(dst: *mut u8, dst_len: usize) -> Option<&'a mut [u8]> {
    if dst.is_null() || dst_len == 0 {
        return None;
    }
    Some(std::slice::from_raw_parts_mut(dst, dst_len))
}

fn status(r: Result<(), sr::SpanError>) -> i32 {
    if r.is_ok() {
        OK
    } else {
        REJECTED
    }
}

/// Slot 1 RenderSolid (§3).
///
/// # Safety
/// `spans` holds `2*count` dwords; `dst` is `dst_len` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn fc4_span8888_solid(
    spans: *const u32,
    count: u32,
    dst: *mut u8,
    dst_len: usize,
    origin: usize,
    stride: i32,
    colour: u32,
) -> i32 {
    match (spans_of(spans, count), dst_of(dst, dst_len)) {
        (Some(s), Some(buf)) => status(sr::render_solid(&s, buf, Surface { base: origin, stride }, colour)),
        _ => BAD_ARGS,
    }
}

/// Slot 3 RenderAlpha (§4).
///
/// # Safety
/// As [`fc4_span8888_solid`].
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_span8888_alpha(
    spans: *const u32,
    count: u32,
    dst: *mut u8,
    dst_len: usize,
    origin: usize,
    stride: i32,
    colour: u32,
    alpha: u32,
) -> i32 {
    match (spans_of(spans, count), dst_of(dst, dst_len)) {
        (Some(s), Some(buf)) => status(sr::render_alpha(&s, buf, Surface { base: origin, stride }, colour, alpha)),
        _ => BAD_ARGS,
    }
}

/// Slot 2 RenderGradient (§5); `alpha_ramp` selects slot 4
/// RenderAlphaGradient (§6). `grad` is the 12-dword record (§5, §6).
///
/// # Safety
/// As [`fc4_span8888_solid`]; `grad` holds 12 dwords.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_span8888_gradient(
    spans: *const u32,
    count: u32,
    dst: *mut u8,
    dst_len: usize,
    origin: usize,
    stride: i32,
    grad: *const u32,
    alpha_ramp: bool,
) -> i32 {
    if grad.is_null() {
        return BAD_ARGS;
    }
    let mut g = [0u32; 12];
    g.copy_from_slice(std::slice::from_raw_parts(grad, 12));
    let g = Gradient::from_dwords(&g);
    match (spans_of(spans, count), dst_of(dst, dst_len)) {
        (Some(s), Some(buf)) => {
            let d = Surface { base: origin, stride };
            status(if alpha_ramp {
                sr::render_alpha_gradient(&s, buf, d, &g)
            } else {
                sr::render_gradient(&s, buf, d, &g)
            })
        }
        _ => BAD_ARGS,
    }
}

/// Slot 5 RenderBlt (§7) with the table dispatch of the vtable method:
/// row `format*2 + keyed` from the 32-byte `desc` (§7 `src`), column `mode`.
/// The source image is `src_len` bytes with texel (0, 0) at `src_origin`
/// (`rectB` +0 base, +4 stride, +8 width, +0x0C height). Destination and
/// source are distinct regions (Mem::Split).
///
/// # Safety
/// As [`fc4_span8888_solid`]; `src` is `src_len` readable bytes, `desc` 32,
/// and `src` does not overlap `dst`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_span8888_blt(
    spans: *const u32,
    count: u32,
    dst: *mut u8,
    dst_len: usize,
    origin: usize,
    stride: i32,
    src: *const u8,
    src_len: usize,
    src_origin: usize,
    src_stride: i32,
    src_width: u32,
    src_height: u32,
    desc: *const u8,
    mode: u32,
) -> i32 {
    if src.is_null() || src_len == 0 || desc.is_null() {
        return BAD_ARGS;
    }
    let mut d = [0u8; 32];
    d.copy_from_slice(std::slice::from_raw_parts(desc, 32));
    let bd = BltDesc::from_bytes(&d);
    let img = SourceImage {
        base: src_origin,
        stride: src_stride,
        width: src_width,
        height: src_height,
    };
    match (spans_of(spans, count), dst_of(dst, dst_len)) {
        (Some(s), Some(buf)) => {
            let mut mem = Mem::Split {
                dst: buf,
                src: std::slice::from_raw_parts(src, src_len),
            };
            status(sr::render_blt(&s, &mut mem, Surface { base: origin, stride }, img, &bd, mode))
        }
        _ => BAD_ARGS,
    }
}

#[cfg(test)]
#[path = "span_render8888_tests.rs"]
mod tests;
