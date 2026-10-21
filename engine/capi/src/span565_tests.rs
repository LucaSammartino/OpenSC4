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

use crate::span565::*;

fn dest(buf: &mut [u8], off: usize, stride: i32) -> fc4_span565_dest {
    fc4_span565_dest {
        pixels: unsafe { buf.as_mut_ptr().add(off) },
        stride,
        width: 5,
        height: 3,
        color_type: 5,
        bytes_per_pixel: 2,
    }
}

fn word(buf: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([buf[at], buf[at + 1]])
}

/// §1.1 conformance example (1): three words, nothing else changes.
#[test]
fn boundary_example_solid() {
    let mut buf = vec![0xA5u8; 64];
    let d = dest(&mut buf, 0, 12);
    let spans = [0x0001_0002u32, 0x0001_0005];
    let r = unsafe { fc4_span565_render_solid(spans.as_ptr(), 1, &d, 0x1234_F81F) };
    assert_eq!(r, FC4_SPAN565_OK);
    for (i, b) in buf.iter().enumerate() {
        if (16..22).contains(&i) {
            assert_eq!(word(&buf, i & !1), 0xF81F);
        } else {
            assert_eq!(*b, 0xA5, "byte {i}");
        }
    }
}

/// §1.1 conformance example (2): A8888 texel 0x00FF8040 -> 0xFC08.
#[test]
fn boundary_example_blt() {
    let mut buf = vec![0u8; 32];
    let texel = 0x00FF_8040u32.to_le_bytes();
    buf[16..20].copy_from_slice(&texel);
    let src = fc4_span565_source {
        texels: unsafe { buf.as_ptr().add(16) },
        stride: 16,
        width: 1,
        height: 1,
        unread: [0; 2],
    };
    let d = dest(&mut buf, 0, 12);
    let b = fc4_span565_blt {
        format: 4,
        ..Default::default()
    };
    let spans = [0u32, 1];
    let r = unsafe { fc4_span565_render_blt(spans.as_ptr(), 1, &d, &src, &b, 0) };
    assert_eq!(r, FC4_SPAN565_OK);
    assert_eq!(word(&buf, 0), 0xFC08);
    assert_eq!(&buf[2..16], &[0u8; 14]);
}

/// Declines write nothing: n = 0, invalid span, slot outside the table,
/// tiled W = 0, null records.
#[test]
fn declines_leave_memory_alone() {
    let mut buf = vec![0x11u8; 64];
    let d = dest(&mut buf, 0, 12);
    let ok = [0u32, 3];
    let bad = [5u32, 2];
    let g = [0u32; 12];
    unsafe {
        assert_eq!(fc4_span565_render_solid(ok.as_ptr(), 0, &d, 0), FC4_SPAN565_EMPTY_LIST);
        assert_eq!(
            fc4_span565_render_solid(bad.as_ptr(), 1, &d, 0),
            FC4_SPAN565_INVALID_SPAN
        );
        assert_eq!(
            fc4_span565_render_solid(ok.as_ptr(), 1, std::ptr::null(), 0),
            FC4_SPAN565_NULL
        );
        assert_eq!(
            fc4_span565_render_gradient(ok.as_ptr(), 1, &d, std::ptr::null()),
            FC4_SPAN565_NULL
        );
        assert_eq!(
            fc4_span565_render_alpha_gradient(bad.as_ptr(), 1, &d, g.as_ptr()),
            FC4_SPAN565_INVALID_SPAN
        );
        let src = fc4_span565_source {
            texels: buf.as_ptr(),
            stride: 8,
            width: 0,
            height: 2,
            unread: [0; 2],
        };
        let b = fc4_span565_blt {
            format: 7,
            ..Default::default()
        };
        assert_eq!(
            fc4_span565_render_blt(ok.as_ptr(), 1, &d, &src, &b, 0),
            FC4_SPAN565_SLOT_OUTSIDE_TABLE
        );
        let b = fc4_span565_blt {
            format: 3,
            ..Default::default()
        };
        assert_eq!(
            fc4_span565_render_blt(ok.as_ptr(), 1, &d, &src, &b, 12),
            FC4_SPAN565_SLOT_OUTSIDE_TABLE
        );
        assert_eq!(
            fc4_span565_render_blt(ok.as_ptr(), 1, &d, &src, &b, 8),
            FC4_SPAN565_TILED_ZERO_SIZE
        );
    }
    assert!(buf.iter().all(|&b| b == 0x11));
}

/// The window is the hull: a blit whose source lies before the destination,
/// with a negative destination stride, draws the same words as the core on
/// one arena, and the bytes between and around stay untouched.
#[test]
fn window_matches_core_on_one_arena() {
    use fc4_core::span_render565 as sr;
    let mut buf = vec![0u8; 256];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (i * 37 + 11) as u8;
    }
    let mut want = buf.clone();
    let spans = [0x0000_0001u32, 0x0000_0004, 0x0001_0000, 0x0001_0003];
    let list = [
        sr::Span {
            start: spans[0],
            end: spans[1],
        },
        sr::Span {
            start: spans[2],
            end: spans[3],
        },
    ];
    // dst row 0 at 200, stride -24 (row 1 at 176); src at 16, R565 (f = 3), stride 10.
    let core_dst = sr::Dest { base: 200, stride: -24 };
    let core_src = sr::Source {
        base: 16,
        stride: 10,
        width: 4,
        height: 2,
    };
    for kind in [0u32, 1, 2, 3, 4, 8, 9] {
        let p = sr::BltParams {
            u0: 0x8000,
            v0: 0x8000,
            du: 0xC000,
            dv: 0x10000,
            format: 3,
            argb: 0x80FF_8040,
            ..Default::default()
        };
        sr::render_blt(&mut want, &list, &core_dst, &core_src, &p, kind).unwrap();
        let d = dest(&mut buf, 200, -24);
        let s = fc4_span565_source {
            texels: unsafe { buf.as_ptr().add(16) },
            stride: 10,
            width: 4,
            height: 2,
            unread: [0; 2],
        };
        let b = fc4_span565_blt {
            u0: 0x8000,
            v0: 0x8000,
            du: 0xC000,
            dv: 0x10000,
            format: 3,
            argb: 0x80FF_8040,
            ..Default::default()
        };
        let r = unsafe { fc4_span565_render_blt(spans.as_ptr(), 2, &d, &s, &b, kind) };
        assert_eq!(r, FC4_SPAN565_OK, "kind {kind}");
        assert_eq!(buf, want, "kind {kind}");
    }
}

/// Every untextured entry equals the core on the same arena.
#[test]
fn untextured_entries_match_core() {
    use fc4_core::span_render565 as sr;
    let mut buf = vec![0u8; 96];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (i * 53 + 7) as u8;
    }
    let mut want = buf.clone();
    let spans = [0x0002_0000u32, 0x0002_0005, 0x0000_0001, 0x0000_0003];
    let list = [
        sr::Span {
            start: spans[0],
            end: spans[1],
        },
        sr::Span {
            start: spans[2],
            end: spans[3],
        },
    ];
    let g: [u32; 12] = [
        0x0010_0000,
        0x0020_0000,
        0x0030_0000,
        0x0080_0000,
        0x1234,
        0x5678,
        0x9ABC,
        0x4000,
        7,
        9,
        11,
        13,
    ];
    let gg = sr::Gradient {
        c: [g[0], g[1], g[2], g[3]],
        dx: [g[4], g[5], g[6], g[7]],
        dy: [g[8], g[9], g[10], g[11]],
    };
    let cd = sr::Dest { base: 4, stride: 12 };
    sr::render_solid(&mut want, &list, &cd, 0xBEEF).unwrap();
    sr::render_alpha(&mut want, &list, &cd, 0x1234, 0x80FF_0000).unwrap();
    sr::render_gradient(&mut want, &list, &cd, &gg).unwrap();
    sr::render_alpha_gradient(&mut want, &list, &cd, &gg).unwrap();
    let d = dest(&mut buf, 4, 12);
    unsafe {
        assert_eq!(fc4_span565_render_solid(spans.as_ptr(), 2, &d, 0xBEEF), FC4_SPAN565_OK);
        assert_eq!(
            fc4_span565_render_alpha(spans.as_ptr(), 2, &d, 0x1234, 0x80FF_0000),
            FC4_SPAN565_OK
        );
        assert_eq!(
            fc4_span565_render_gradient(spans.as_ptr(), 2, &d, g.as_ptr()),
            FC4_SPAN565_OK
        );
        assert_eq!(
            fc4_span565_render_alpha_gradient(spans.as_ptr(), 2, &d, g.as_ptr()),
            FC4_SPAN565_OK
        );
    }
    assert_eq!(buf, want);
}

#[test]
fn record_layouts() {
    let p = std::mem::size_of::<usize>();
    assert_eq!(std::mem::size_of::<fc4_span565_dest>(), if p == 4 { 24 } else { 32 });
    assert_eq!(std::mem::size_of::<fc4_span565_source>(), if p == 4 { 24 } else { 32 });
    assert_eq!(std::mem::size_of::<fc4_span565_blt>(), 32);
}
