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

//! C ABI over `fc4_core::span_render4444` for the plug-in's cSpanRender4444
//! bridge (glue-span4444-impl4; docs/specs/span-render-4444.md §1.1-§7).
//!
//! Every call takes the destination as a byte region (`dst`, `dst_len`) with
//! pixel (0, 0) at byte `origin` of it and a signed byte `stride`; the core
//! checks every touched byte against the region before it writes anything.
//! Return value: 0 rendered, 1 a null pointer or a zero span count,
//! 2 the core rejected the input (nothing written).

use fc4_core::span_render4444::{self as sr, BltDesc, Gradient, SourceImage, Span, Target};

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

unsafe fn target_of<'a>(dst: *mut u8, dst_len: usize, origin: usize, stride: i32) -> Option<Target<'a>> {
    if dst.is_null() || dst_len == 0 {
        return None;
    }
    Some(Target {
        data: std::slice::from_raw_parts_mut(dst, dst_len),
        origin,
        stride,
    })
}

/// The 12-dword gradient record (§5): starts, x steps, y steps.
unsafe fn gradient_of(grad: *const i32) -> Option<Gradient> {
    if grad.is_null() {
        return None;
    }
    let g = std::slice::from_raw_parts(grad, 12);
    let mut out = Gradient::default();
    out.start.copy_from_slice(&g[0..4]);
    out.dx.copy_from_slice(&g[4..8]);
    out.dy.copy_from_slice(&g[8..12]);
    Some(out)
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
pub unsafe extern "C" fn fc4_span4444_solid(
    spans: *const u32,
    count: u32,
    dst: *mut u8,
    dst_len: usize,
    origin: usize,
    stride: i32,
    colour: u32,
) -> i32 {
    match (spans_of(spans, count), target_of(dst, dst_len, origin, stride)) {
        (Some(s), Some(mut t)) => status(sr::render_solid(&s, &mut t, colour)),
        _ => BAD_ARGS,
    }
}

/// Slot 3 RenderAlpha (§4).
///
/// # Safety
/// As [`fc4_span4444_solid`].
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_span4444_alpha(
    spans: *const u32,
    count: u32,
    dst: *mut u8,
    dst_len: usize,
    origin: usize,
    stride: i32,
    colour: u32,
    alpha: u32,
) -> i32 {
    match (spans_of(spans, count), target_of(dst, dst_len, origin, stride)) {
        (Some(s), Some(mut t)) => status(sr::render_alpha(&s, &mut t, colour, alpha)),
        _ => BAD_ARGS,
    }
}

/// Slot 2 RenderGradient (§5); `alpha_ramp` selects slot 4
/// RenderAlphaGradient (§6). `grad` is the 12-dword record.
///
/// # Safety
/// As [`fc4_span4444_solid`]; `grad` holds 12 dwords.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_span4444_gradient(
    spans: *const u32,
    count: u32,
    dst: *mut u8,
    dst_len: usize,
    origin: usize,
    stride: i32,
    grad: *const i32,
    alpha_ramp: bool,
) -> i32 {
    match (
        spans_of(spans, count),
        target_of(dst, dst_len, origin, stride),
        gradient_of(grad),
    ) {
        (Some(s), Some(mut t), Some(g)) => status(if alpha_ramp {
            sr::render_alpha_gradient(&s, &mut t, &g)
        } else {
            sr::render_gradient(&s, &mut t, &g)
        }),
        _ => BAD_ARGS,
    }
}

/// Slot 5 RenderBlt (§7) with the table dispatch of the vtable method:
/// row `format*2 + flag` from the 32-byte `desc` (§7 `src`), column `mode`.
/// The source image is `src_len` bytes with texel (0, 0) at `src_origin`
/// (`rectB` +0 base, +4 stride, +8 width, +0x0C height).
///
/// # Safety
/// As [`fc4_span4444_solid`]; `src` is `src_len` readable bytes, `desc` 32.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_span4444_blt(
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
    let d = std::slice::from_raw_parts(desc, 32);
    let dw = |i: usize| u32::from_le_bytes([d[4 * i], d[4 * i + 1], d[4 * i + 2], d[4 * i + 3]]);
    let bd = BltDesc {
        u0: dw(0),
        v0: dw(1),
        dudx: dw(2),
        dvdy: dw(3),
        format: dw(4),
        colour: dw(5),
        key: dw(6),
        keyed: d[0x1C],
    };
    let img = SourceImage {
        data: std::slice::from_raw_parts(src, src_len),
        origin: src_origin,
        stride: src_stride,
        width: src_width,
        height: src_height,
    };
    match (spans_of(spans, count), target_of(dst, dst_len, origin, stride)) {
        (Some(s), Some(mut t)) => status(sr::render_blt_raw(&s, &mut t, &img, &bd, mode)),
        _ => BAD_ARGS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_and_alpha_follow_the_spec_examples() {
        // §3: verbatim store; §4 worked example d=0x8000, colour 0x0F84, a=8 -> 0xF842.
        let spans = [0x0000_0001u32, 0x0000_0003];
        let mut buf = [0u8; 8];
        assert_eq!(
            unsafe { fc4_span4444_solid(spans.as_ptr(), 1, buf.as_mut_ptr(), 8, 0, 8, 0x1234_ABCD) },
            OK
        );
        assert_eq!(buf, [0, 0, 0xCD, 0xAB, 0xCD, 0xAB, 0, 0]);
        let mut d = [0x00u8, 0x80, 0, 0];
        let one = [0u32, 1];
        assert_eq!(
            unsafe { fc4_span4444_alpha(one.as_ptr(), 1, d.as_mut_ptr(), 4, 0, 4, 0x0F84, 0x8000_0000) },
            OK
        );
        assert_eq!(u16::from_le_bytes([d[0], d[1]]), 0xF842);
    }

    #[test]
    fn bad_input_is_rejected_without_writing() {
        let spans = [0x0000_0003u32, 0x0000_0006];
        let mut buf = [0u8; 8];
        assert_eq!(
            unsafe { fc4_span4444_solid(spans.as_ptr(), 1, buf.as_mut_ptr(), 8, 0, 8, 0xFFFF) },
            REJECTED
        );
        assert_eq!(buf, [0u8; 8]);
        assert_eq!(
            unsafe { fc4_span4444_solid(spans.as_ptr(), 0, buf.as_mut_ptr(), 8, 0, 8, 0) },
            BAD_ARGS
        );
        let g = [0i32; 12];
        assert_eq!(
            unsafe { fc4_span4444_gradient(std::ptr::null(), 1, buf.as_mut_ptr(), 8, 0, 8, g.as_ptr(), false) },
            BAD_ARGS
        );
    }

    #[test]
    fn blt_reads_the_descriptor_bytes() {
        // Row 0 (format 0, key off), column 0: a 4444 copy of one texel.
        let spans = [0u32, 1];
        let mut dst = [0u8; 2];
        let src = [0x34u8, 0x12];
        let desc = [0u8; 32];
        let r = unsafe {
            fc4_span4444_blt(
                spans.as_ptr(),
                1,
                dst.as_mut_ptr(),
                2,
                0,
                2,
                src.as_ptr(),
                2,
                0,
                2,
                1,
                1,
                desc.as_ptr(),
                0,
            )
        };
        let mut want = [0u8; 2];
        let mut t = Target {
            data: &mut want,
            origin: 0,
            stride: 2,
        };
        let img = SourceImage {
            data: &src,
            origin: 0,
            stride: 2,
            width: 1,
            height: 1,
        };
        let expect = sr::render_blt_raw(&[Span { start: 0, end: 1 }], &mut t, &img, &BltDesc::default(), 0);
        assert_eq!(r, status(expect));
        assert_eq!(dst, want);
    }
}
