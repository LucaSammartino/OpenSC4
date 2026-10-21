//! C ABI over the 1555 span renderer (`fc4_core::span_render1555`,
//! docs/specs/span-render-1555.md). The plug-in's native bridge (glue-span1555-impl4)
//! calls these with the same arguments the original's slots 1-5 receive (§1.1), but
//! with every pointer replaced by an explicit memory window, so the core never
//! touches memory outside it.
//!
//! A window is `(ptr, len, base)`: `len` bytes from `ptr`, and `base` the signed byte
//! offset from `ptr` of the record's pixel (0, 0) (it may lie outside the window, as a
//! biased native pointer can). Spans are `n` pairs of dwords (`start`, `end`, §1).
//!
//! Return codes: 0 drawn; 1 `n = 0`; 2 invalid span; 3 destination outside the window;
//! 4 source outside the window; 5 row ≥ 14; 6 kind ≥ 12; 7 empty tile; -1 a null
//! pointer or a panic.

use fc4_core::span_render1555::{
    render_alpha, render_alpha_gradient, render_blt, render_gradient, render_solid, BlitRecord, BltSource, Error,
    GradientRecord, Span, Surface,
};

use crate::guard;

fn code(r: Result<(), Error>) -> i32 {
    match r {
        Ok(()) => 0,
        Err(Error::EmptySpanList) => 1,
        Err(Error::InvalidSpan { .. }) => 2,
        Err(Error::DestOutOfBounds { .. }) => 3,
        Err(Error::SourceOutOfBounds) => 4,
        Err(Error::RowOutOfRange { .. }) => 5,
        Err(Error::KindOutOfRange { .. }) => 6,
        Err(Error::EmptyTile) => 7,
    }
}

unsafe fn spans_of(spans: *const u32, n: u32) -> Option<Vec<Span>> {
    if n == 0 {
        return Some(Vec::new());
    }
    if spans.is_null() {
        return None;
    }
    let raw = std::slice::from_raw_parts(spans, 2 * n as usize);
    Some(raw.chunks_exact(2).map(|c| Span { start: c[0], end: c[1] }).collect())
}

/// Runs `f` over the decoded spans and the destination window; -1 on a null pointer.
unsafe fn with_dst(
    spans: *const u32,
    n: u32,
    dst: *mut u8,
    dst_len: u32,
    dst_base: i64,
    stride: i32,
    f: impl FnOnce(&[Span], &mut Surface) -> Result<(), Error>,
) -> i32 {
    guard(-1, || {
        let Some(list) = spans_of(spans, n) else { return -1 };
        if dst.is_null() && dst_len != 0 {
            return -1;
        }
        let buf: &mut [u8] = if dst_len == 0 {
            &mut []
        } else {
            std::slice::from_raw_parts_mut(dst, dst_len as usize)
        };
        let mut s = Surface {
            buf,
            base: dst_base,
            stride,
        };
        code(f(&list, &mut s))
    })
}

unsafe fn gradient_of(g: *const i32) -> Option<GradientRecord> {
    if g.is_null() {
        return None;
    }
    let v = std::slice::from_raw_parts(g, 12);
    let mut r = GradientRecord::default();
    r.c.copy_from_slice(&v[0..4]);
    r.dx.copy_from_slice(&v[4..8]);
    r.dy.copy_from_slice(&v[8..12]);
    Some(r)
}

/// Slot 1, RenderSolid (§2).
#[no_mangle]
pub unsafe extern "C" fn fc4_span1555_solid(
    spans: *const u32,
    n: u32,
    dst: *mut u8,
    dst_len: u32,
    dst_base: i64,
    stride: i32,
    colour: u32,
) -> i32 {
    with_dst(spans, n, dst, dst_len, dst_base, stride, |l, s| {
        render_solid(l, s, colour)
    })
}

/// Slot 3, RenderAlpha (§3).
#[no_mangle]
pub unsafe extern "C" fn fc4_span1555_alpha(
    spans: *const u32,
    n: u32,
    dst: *mut u8,
    dst_len: u32,
    dst_base: i64,
    stride: i32,
    src: u32,
    argb: u32,
) -> i32 {
    with_dst(spans, n, dst, dst_len, dst_base, stride, |l, s| {
        render_alpha(l, s, src, argb)
    })
}

/// Slot 2, RenderGradient (§4); `grad` is the 12-dword record of §4.1.
#[no_mangle]
pub unsafe extern "C" fn fc4_span1555_gradient(
    spans: *const u32,
    n: u32,
    dst: *mut u8,
    dst_len: u32,
    dst_base: i64,
    stride: i32,
    grad: *const i32,
) -> i32 {
    let Some(g) = gradient_of(grad) else { return -1 };
    with_dst(spans, n, dst, dst_len, dst_base, stride, |l, s| {
        render_gradient(l, s, &g)
    })
}

/// Slot 4, RenderAlphaGradient (§5).
#[no_mangle]
pub unsafe extern "C" fn fc4_span1555_alpha_gradient(
    spans: *const u32,
    n: u32,
    dst: *mut u8,
    dst_len: u32,
    dst_base: i64,
    stride: i32,
    grad: *const i32,
) -> i32 {
    let Some(g) = gradient_of(grad) else { return -1 };
    with_dst(spans, n, dst, dst_len, dst_base, stride, |l, s| {
        render_alpha_gradient(l, s, &g)
    })
}

/// Slot 5, RenderBlt (§6). `rec` is the 24-byte source record (its `+0` pointer is
/// replaced by the source window), `blit` the 32-byte blit record.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_span1555_blt(
    spans: *const u32,
    n: u32,
    dst: *mut u8,
    dst_len: u32,
    dst_base: i64,
    stride: i32,
    src: *const u8,
    src_len: u32,
    src_base: i64,
    rec: *const u32,
    blit: *const u32,
    kind: u32,
) -> i32 {
    if rec.is_null() || blit.is_null() || (src.is_null() && src_len != 0) {
        return -1;
    }
    let r = std::slice::from_raw_parts(rec, 6);
    let b = std::slice::from_raw_parts(blit, 8);
    let sbuf: &[u8] = if src_len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(src, src_len as usize)
    };
    let source = BltSource {
        buf: sbuf,
        base: src_base,
        stride: r[1] as i32,
        width: r[2],
        height: r[3],
        color_type: r[4],
        bpp: r[5],
    };
    let record = BlitRecord {
        u0: b[0],
        v0: b[1],
        du: b[2],
        dv: b[3],
        format: b[4],
        color: b[5],
        key: b[6],
        key_flag: b[7] as u8,
    };
    with_dst(spans, n, dst, dst_len, dst_base, stride, |l, s| {
        render_blt(l, s, &source, &record, kind)
    })
}
