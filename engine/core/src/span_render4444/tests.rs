// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors. See LICENSE at the repository root.

//! Tests from the worked examples and stated rules of
//! docs/specs/span-render-4444.md.

use super::*;

fn buf(w: usize, h: usize, fill: u16) -> Vec<u8> {
    let mut v = Vec::with_capacity(w * h * 2);
    for _ in 0..w * h {
        v.extend_from_slice(&fill.to_le_bytes());
    }
    v
}

fn px(data: &[u8], stride: usize, x: usize, y: usize) -> u16 {
    let o = y * stride + 2 * x;
    u16::from_le_bytes([data[o], data[o + 1]])
}

fn row(data: &[u8], stride: usize, y: usize, n: usize) -> Vec<u16> {
    (0..n).map(|x| px(data, stride, x, y)).collect()
}

fn src16(texels: &[u16]) -> Vec<u8> {
    texels.iter().flat_map(|t| t.to_le_bytes()).collect()
}

#[test]
fn span_packing() {
    let s = Span::new(3, 2, 5);
    assert_eq!(s.start, 0x0002_0003);
    assert_eq!(s.end, 0x0002_0008);
    assert_eq!((s.x(), s.y(), s.width()), (3, 2, 5));
    // end before start: width wraps (2^32 - 1), rejected as out of bounds.
    let bad = Span { start: 5, end: 4 };
    assert_eq!(bad.width(), u32::MAX);
}

#[test]
fn solid_fills_verbatim_and_leaves_neighbours() {
    let mut d = buf(8, 2, 0x1234);
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 16,
    };
    render_solid(&[Span::new(2, 1, 3), Span::new(0, 0, 0)], &mut t, 0xABCD_0F0F).unwrap();
    assert_eq!(row(&d, 16, 0, 8), vec![0x1234; 8]);
    assert_eq!(
        row(&d, 16, 1, 8),
        vec![0x1234, 0x1234, 0x0F0F, 0x0F0F, 0x0F0F, 0x1234, 0x1234, 0x1234]
    );
}

#[test]
fn solid_unaligned_origin_and_negative_stride() {
    // Origin at an odd byte, rows going up (stride -11).
    let mut d = vec![0u8; 40];
    let mut t = Target {
        data: &mut d,
        origin: 23,
        stride: -11,
    };
    render_solid(&[Span::new(1, 2, 2)], &mut t, 0xBEEF).unwrap();
    // pixel (1,2) at 23 - 22 + 2 = 3.
    assert_eq!(&d[3..7], &[0xEF, 0xBE, 0xEF, 0xBE]);
    assert!(d[..3].iter().chain(&d[7..]).all(|&b| b == 0));
}

#[test]
fn errors_leave_target_untouched() {
    let mut d = buf(4, 2, 0x1111);
    let before = d.clone();
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 8,
    };
    assert_eq!(render_solid(&[], &mut t, 0), Err(SpanError::EmptySpanList));
    let r = render_solid(&[Span::new(0, 0, 4), Span::new(2, 1, 3)], &mut t, 0xFFFF);
    assert_eq!(r, Err(SpanError::DestOutOfBounds { span: 1 }));
    let r = render_alpha(&[Span { start: 5, end: 4 }], &mut t, 0, 0);
    assert_eq!(r, Err(SpanError::DestOutOfBounds { span: 0 }));
    assert_eq!(d, before);
}

#[test]
fn alpha_weight_map() {
    assert_eq!(render_alpha_weight(0x07FF_FFFF), 0);
    assert_eq!(render_alpha_weight(0x0800_0000), 1);
    assert_eq!(render_alpha_weight(0x8000_0000), 8);
    assert_eq!(render_alpha_weight(0xF7FF_FFFF), 15);
    assert_eq!(render_alpha_weight(0xF800_0000), 16);
    assert_eq!(render_alpha_weight(0xFFFF_FFFF), 16);
}

#[test]
fn render_alpha_worked_example() {
    // §4: d = 0x8000, colour 0x0F84, alpha 0x80000000 -> 0xF842.
    let mut d = buf(3, 1, 0x8000);
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 6,
    };
    render_alpha(&[Span::new(1, 0, 1)], &mut t, 0x0F84, 0x8000_0000).unwrap();
    assert_eq!(row(&d, 6, 0, 3), vec![0x8000, 0xF842, 0x8000]);
}

#[test]
fn blend_bounds() {
    // a = 16 reproduces the source colour, a = 0 the destination; alpha F.
    assert_eq!(blend4444(0x0ABC, 0x7123, 16), 0xFABC);
    assert_eq!(blend4444(0x0ABC, 0x7123, 0), 0xF123);
    // §9.4: a tops out at 15 for op 1: s = 15 over d = 0 gives 14.
    assert_eq!(blend4444(0x0FFF, 0x0000, 15), 0xFEEE);
    // Exhaustive: per-channel result stays in 0..15 for a in 0..16.
    for a in 0..=16 {
        for s in 0..16u16 {
            for d in 0..16u16 {
                let v = (16 * d as i32 + a * (s as i32 - d as i32) + 8).div_euclid(16);
                assert!((0..16).contains(&v));
                assert_eq!(blend4444(s << 4, d << 4, a as u32), 0xF000 | ((v as u16) << 4));
            }
        }
    }
}

#[test]
fn gradient_flat_and_truncation() {
    // B = 0x12.8, G = 0x34, R = 0xFF.FF, no steps.
    let g = Gradient {
        start: [0x0012_8000, 0x0034_0000, 0x00FF_FFFF, 0],
        ..Default::default()
    };
    let mut d = buf(4, 1, 0);
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 8,
    };
    render_gradient(&[Span::new(0, 0, 4)], &mut t, &g).unwrap();
    assert_eq!(row(&d, 8, 0, 4), vec![0xFF31; 4]);
}

#[test]
fn gradient_steps_and_b_drift() {
    // B step 0x10000F: the arithmetic >> 4 drops its low nibble, so B gains
    // exactly 1.0 per pixel (the 0xF/65536 part is lost: drift).
    let g = Gradient {
        start: [0x000F_0000, 0, 0, 0],
        dx: [0x0010_000F, 0x0010_0000, 0x0010_0000, 0],
        dy: [0; 4],
    };
    let mut d = buf(4, 1, 0);
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 8,
    };
    render_gradient(&[Span::new(0, 0, 4)], &mut t, &g).unwrap();
    // B = 0x0F, 0x1F, 0x2F, 0x3F -> nibble 0,1,2,3; G, R = 0,0x10,0x20,0x30.
    assert_eq!(row(&d, 8, 0, 4), vec![0xF000, 0xF111, 0xF222, 0xF333]);
}

#[test]
fn gradient_start_uses_absolute_xy() {
    let g = Gradient {
        start: [0, 0, 0, 0],
        dx: [0, 0, 0x0010_0000, 0],
        dy: [0, 0x0020_0000, 0, 0],
    };
    let mut d = buf(4, 3, 0);
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 8,
    };
    render_gradient(&[Span::new(2, 2, 1)], &mut t, &g).unwrap();
    // R = 0x20 -> 2, G = 0x40 -> 4.
    assert_eq!(px(&d, 8, 2, 2), 0xF240);
}

#[test]
fn gradient_r_overflow_reaches_alpha() {
    // U2: R from 0xF0 by +0x10 per pixel reaches 0x100 at pixel 1: the carry
    // lands in the alpha nibble (0xF + 1 wraps to 0x0).
    let g = Gradient {
        start: [0, 0, 0x00F0_0000, 0],
        dx: [0, 0, 0x0010_0000, 0],
        dy: [0; 4],
    };
    let mut d = buf(2, 1, 0);
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 4,
    };
    render_gradient(&[Span::new(0, 0, 2)], &mut t, &g).unwrap();
    assert_eq!(row(&d, 4, 0, 2), vec![0xFF00, 0x0000]);
}

#[test]
fn alpha_gradient_weights() {
    for (a, w) in [
        (0u32, 0u32),
        (15, 0),
        (16, 1),
        (128, 8),
        (240, 15),
        (241, 16),
        (255, 16),
    ] {
        assert_eq!(alpha_gradient_weight((a << 16).wrapping_mul(0x88)), w, "A={a}");
        assert_eq!(w, a * 17 / 256);
    }
}

#[test]
fn alpha_gradient_blend() {
    // Colour R=0xF0 G=0x80 B=0x40, alpha 255 (weight 16): source reproduced.
    let g = Gradient {
        start: [0x0040_0000, 0x0080_0000, 0x00F0_0000, 0x00FF_0000],
        ..Default::default()
    };
    let mut d = buf(2, 1, 0x0123);
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 4,
    };
    render_alpha_gradient(&[Span::new(0, 0, 1)], &mut t, &g).unwrap();
    assert_eq!(row(&d, 4, 0, 2), vec![0xFF84, 0x0123]);
    // R out of range does not reach alpha here (top nibble masked off).
    let g2 = Gradient {
        start: [0, 0, 0x01F0_0000, 0x00FF_0000],
        ..Default::default()
    };
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 4,
    };
    render_alpha_gradient(&[Span::new(1, 0, 1)], &mut t, &g2).unwrap();
    assert_eq!(px(&d, 4, 1, 0), 0xFF00);
}

#[test]
fn sp1_published_packed_vectors() {
    // Independent expected words from §6.1, including every reachable weight >16.
    let cases: [(i32, u16, u32, u16); 22] = [
        (0x0070, 0xF859, 256 << 16, 0xFF7F),
        (0x0070, 0xF455, 272 << 16, 0xFF7F),
        (0x0070, 0xF853, 287 << 16, 0xFE7F),
        (0x0070, 0xF253, 302 << 16, 0xFF8F),
        (0x0070, 0xF852, 317 << 16, 0xFD8F),
        (0x0070, 0xF452, 332 << 16, 0xFE8F),
        (0x0070, 0xF852, 347 << 16, 0xFC8F),
        (0x0070, 0xF152, 362 << 16, 0xFF8F),
        (0x0070, 0xF851, 377 << 16, 0xFB8F),
        (0x0070, 0xF451, 392 << 16, 0xFD8F),
        (0x0070, 0xF851, 407 << 16, 0xFA8F),
        (0x0070, 0xF251, 422 << 16, 0xFE9F),
        (0x0070, 0xF851, 437 << 16, 0xF99F),
        (0x0070, 0xF451, 452 << 16, 0xFC9F),
        (0x0070, 0xF851, 467 << 16, 0xF89F),
        (0x0070, 0x8859, 0, 0xF859),
        (0x0070, 0x8859, 0x00FF_0000, 0xF070),
        (0x0000, 0x880F, 0x0100_0000, 0xFF0F),
        (0x070F, 0x8000, 0x0100_0000, 0xF800),
        (0x0070, 0xF851, 0xFFFF_0000, 0xF89F),
        (0x0070, 0x8859, 0x01E2_0000, 0xF859),
        (0x0070, 0x8859, 0x8000_0000, 0xF859),
    ];
    for (s, d, alpha, expected) in cases {
        let g = Gradient {
            start: [
                ((s & 15) << 20),
                (((s >> 4) & 15) << 20),
                (((s >> 8) & 15) << 20),
                alpha as i32,
            ],
            ..Default::default()
        };
        let mut data = buf(3, 1, d);
        let mut target = Target {
            data: &mut data,
            origin: 2,
            stride: 6,
        };
        render_alpha_gradient(&[Span::new(0, 0, 1)], &mut target, &g).unwrap();
        assert_eq!(
            row(&data, 6, 0, 3),
            [d, expected, d],
            "source={s:04x} alpha={alpha:08x}"
        );
    }
}

#[test]
fn sp1_step_crosses_weight_and_wrap_boundaries() {
    for (alpha, d, expected) in [
        (0x00FF_0000, 0xF859, [0xF070, 0xFF7F]),
        (0x01E1_0000, 0xF851, [0xF89F, 0xF851]),
    ] {
        let g = Gradient {
            start: [0, 0x0070_0000, 0, alpha],
            dx: [0, 0, 0, 0x0001_0000],
            ..Default::default()
        };
        let mut data = buf(2, 1, d);
        let mut target = Target {
            data: &mut data,
            origin: 0,
            stride: 4,
        };
        render_alpha_gradient(&[Span::new(0, 0, 2)], &mut target, &g).unwrap();
        assert_eq!(row(&data, 4, 0, 2), expected);
    }
}

#[test]
fn conversions_4444() {
    use SourceFormat::*;
    assert_eq!(blt_conversion_4444(A4R4G4B4, 0x1234), 0x1234);
    // 1555: A bit 15, R bits 11-14, G 6-9, B 1-4.
    assert_eq!(blt_conversion_4444(A1R5G5B5, 0x7FFF), 0x0FFF);
    assert_eq!(
        blt_conversion_4444(A1R5G5B5, 0x8000 | (0x1E << 10) | (0x11 << 5) | 0x03),
        0xFF81
    );
    assert_eq!(blt_conversion_4444(X1R5G5B5, 0x0000), 0xF000);
    // 565: R bits 12-15, G 7-10, B 1-4.
    assert_eq!(blt_conversion_4444(R5G6B5, (0x1F << 11) | (0x21 << 5) | 0x01), 0xFF80);
    assert_eq!(blt_conversion_4444(A8R8G8B8, 0x7F12_34AB), 0x713A);
    assert_eq!(blt_conversion_4444(X8R8G8B8, 0x0012_34AB), 0xF13A);
    assert_eq!(blt_conversion_4444(A8, 0x9C), 0x9FFF);
}

#[test]
fn widening_8888() {
    use SourceFormat::*;
    assert_eq!(blt_widen_8888(A4R4G4B4, 0xF1A0), 0xFF11_AA00);
    assert_eq!(blt_widen_8888(A1R5G5B5, 0x7FFF), 0x00FF_FFFF);
    assert_eq!(
        blt_widen_8888(A1R5G5B5, 0x8000 | (0x10 << 10) | (0x01 << 5) | 0x1F),
        0xFF84_08FF
    );
    assert_eq!(blt_widen_8888(X1R5G5B5, 0), 0xFF00_0000);
    assert_eq!(blt_widen_8888(R5G6B5, (0x10 << 11) | (0x20 << 5) | 0x01), 0xFF84_8208);
    assert_eq!(blt_widen_8888(A8R8G8B8, 0x1234_5678), 0x1234_5678);
    assert_eq!(blt_widen_8888(X8R8G8B8, 0x1234_5678), 0xFF34_5678);
    assert_eq!(blt_widen_8888(A8, 0x5A), 0x5AFF_FFFF);
}

fn img(data: &[u8], stride: i32, w: u32, h: u32) -> SourceImage<'_> {
    SourceImage {
        data,
        origin: 0,
        stride,
        width: w,
        height: h,
    }
}

/// Runs `mode` (raw dispatch) on one span (x, 0, n) of a 1-row target filled
/// with `fill`; returns the row.
fn blt1(src: &SourceImage, desc: &BltDesc, mode: u32, x: u16, n: u16, fill: u16) -> Vec<u16> {
    let w = (x + n + 1) as usize;
    let mut d = buf(w, 1, fill);
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 2 * w as i32,
    };
    render_blt_raw(&[Span::new(x, 0, n)], &mut t, src, desc, mode).unwrap();
    row(&d, 2 * w, 0, w)
}

#[test]
fn blt_copy_direct_per_format() {
    let s16 = src16(&[0x8000 | (0x1F << 10), 0x001F]);
    let i = img(&s16, 4, 2, 1);
    let desc = BltDesc {
        format: 1,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 0, 0, 2, 0x5555), vec![0xFF00, 0x000F, 0x5555]);
    let s32: Vec<u8> = [0xFF10_2030u32, 0x80FF_FFFF]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let i = img(&s32, 8, 2, 1);
    let desc = BltDesc {
        format: 4,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 0, 0, 2, 0), vec![0xF123, 0x8FFF, 0]);
    let desc = BltDesc {
        format: 5,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 0, 0, 2, 0), vec![0xF123, 0xFFFF, 0]);
    let a8 = [0x00u8, 0xF0, 0x7F];
    let i = img(&a8, 3, 3, 1);
    let desc = BltDesc {
        format: 6,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 0, 1, 2, 0), vec![0, 0xFFFF, 0x7FFF, 0]);
}

#[test]
fn blt_constant_alpha() {
    let s = src16(&[0x0FFF, 0x0FFF]);
    let i = img(&s, 4, 2, 1);
    // colour alpha 0xFF -> a = 15: 15 over 0 gives 14; source alpha ignored.
    let desc = BltDesc {
        colour: 0xFF00_0000,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 1, 0, 2, 0x0000), vec![0xFEEE, 0xFEEE, 0]);
    // colour alpha 0x0F -> a = 0: destination RGB kept, alpha forced F.
    let desc = BltDesc {
        colour: 0x0FFF_FFFF,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 1, 0, 1, 0x0123), vec![0xF123, 0x0123]);
}

#[test]
fn blt_modulate_and_modulate_alpha() {
    let s = src16(&[0xFFFF, 0x8421]);
    let i = img(&s, 4, 2, 1);
    // White constant: (255*255 + 128) >> 8 = 254 -> nibble F.
    let desc = BltDesc {
        colour: 0xFFFF_FFFF,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 2, 0, 2, 0x3333), vec![0xFFFF, 0x8421, 0x3333]);
    // 0x80 constant: 0xFF*0x80 + 0x80 = 0x8000 -> 0x80 -> 8;
    // 0x88*0x80 = 0x4400, +0x80 >> 8 = 0x44 -> 4.
    let desc = BltDesc {
        colour: 0x8080_8080,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 2, 0, 1, 0), vec![0x8888, 0]);
    assert_eq!(blt1(&i, &desc, 2, 1, 1, 0), vec![0, 0x4210, 0]);
    // Modulate + alpha: M = 0x80808080, a = 8, s = 8 over d = 0:
    // floor((0 + 64 + 8) / 16) = 4.
    assert_eq!(blt1(&i, &desc, 3, 0, 1, 0x0000), vec![0xF444, 0]);
    // Zero constant: M = 0 everywhere, a = 0, destination RGB kept.
    let desc = BltDesc {
        colour: 0,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 3, 0, 1, 0x0ABC), vec![0xFABC, 0x0ABC]);
    assert_eq!(blt1(&i, &desc, 2, 0, 1, 0x0ABC), vec![0x0000, 0x0ABC]);
}

#[test]
fn keyed_masks() {
    // 4444 keyed: mask 0x0FFF ignores the alpha nibble.
    let s = src16(&[0xF123, 0x0123, 0x0124]);
    let i = img(&s, 6, 3, 1);
    let desc = BltDesc {
        key: 0x123,
        keyed: 1,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 0, 0, 3, 0x7777), vec![0x7777, 0x7777, 0x0124, 0x7777]);
    // A key with bits outside the mask never matches.
    let desc = BltDesc {
        key: 0x1123,
        keyed: 1,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 0, 0, 2, 0), vec![0xF123, 0x0123, 0]);
    // 565 key uses all 16 bits.
    let s = src16(&[0xFFFF, 0x7FFF]);
    let i = img(&s, 4, 2, 1);
    let desc = BltDesc {
        format: 3,
        key: 0xFFFF,
        keyed: 1,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 0, 0, 2, 0x1111), vec![0x1111, 0xF7FF, 0x1111]);
    // 1555 key ignores bit 15.
    let desc = BltDesc {
        format: 1,
        key: 0x7FFF,
        keyed: 1,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 0, 0, 2, 0x1111), vec![0x1111, 0x1111, 0x1111]);
    // 8888 key mask 0xFFFFFF.
    let s32: Vec<u8> = [0x12AB_CDEFu32].iter().flat_map(|v| v.to_le_bytes()).collect();
    let i = img(&s32, 4, 1, 1);
    let desc = BltDesc {
        format: 4,
        key: 0xAB_CDEF,
        keyed: 1,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 2, 0, 1, 0x1111), vec![0x1111, 0x1111]);
}

#[test]
fn keyed_a8_degenerates() {
    let a8 = [0x00u8, 0x55, 0xFF];
    let i = img(&a8, 3, 3, 1);
    // Key 0: nothing drawn, in every column.
    let desc = BltDesc {
        format: 6,
        key: 0,
        keyed: 1,
        colour: 0xFFFF_FFFF,
        ..Default::default()
    };
    for mode in 0..12 {
        assert_eq!(blt1(&i, &desc, mode, 0, 3, 0x1234), vec![0x1234; 4], "mode {mode}");
    }
    // Any other key: every texel drawn, even one equal to the key.
    let desc = BltDesc {
        format: 6,
        key: 0x55,
        keyed: 1,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 0, 0, 3, 0x1234), vec![0x0FFF, 0x5FFF, 0xFFFF, 0x1234]);
}

#[test]
fn stretch_walk() {
    let s = src16(&[0x0000, 0x0001, 0x0002, 0x0003, 0x0010, 0x0011, 0x0012, 0x0013]);
    let i = img(&s, 8, 4, 2);
    // Half-texel steps from u0 = 0.5; x absolute: span starts at x = 1 so
    // u = 0.5*1 + 0.5 = 1.0 -> texels 1, 1, 2, 2 ... ; v = 0.5*0 + 1.0 = row 1.
    let desc = BltDesc {
        u0: 0x8000,
        v0: 0x1_0000,
        dudx: 0x8000,
        dvdy: 0x8000,
        ..Default::default()
    };
    assert_eq!(
        blt1(&i, &desc, 4, 1, 4, 0xFFFF),
        vec![0xFFFF, 0x11, 0x11, 0x12, 0x12, 0xFFFF]
    );
    // Out-of-range texel (no clamp, U5) is a bounded-API error.
    let desc = BltDesc { u0: 0x4_0000, ..desc };
    let mut d = buf(2, 1, 0);
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 4,
    };
    let r = render_blt_raw(&[Span::new(0, 0, 1)], &mut t, &i, &desc, 4);
    assert_eq!(r, Err(SpanError::SourceOutOfBounds { span: 0 }));
}

#[test]
fn tiled_walk() {
    let s = src16(&[0xA, 0xB, 0xC, 0xD, 0xE, 0xF]);
    let i = img(&s, 6, 3, 2);
    // tx = (1 + 4) mod 3 = 2, ty = (0 + 7) mod 2 = 1: D E F walk from F.
    let desc = BltDesc {
        u0: 4,
        v0: 7,
        dudx: 0xDEAD,
        dvdy: 0xBEEF,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 8, 1, 5, 0), vec![0, 0xF, 0xD, 0xE, 0xF, 0xD, 0]);
    // x + u0 wraps 2^32 before the modulo.
    let desc = BltDesc {
        u0: u32::MAX,
        ..Default::default()
    };
    assert_eq!(blt1(&i, &desc, 8, 1, 1, 0), vec![0, 0xA, 0]);
    let i0 = img(&s, 6, 0, 2);
    let mut d = buf(1, 1, 0);
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 2,
    };
    let r = render_blt_raw(&[Span::new(0, 0, 1)], &mut t, &i0, &desc, 8);
    assert_eq!(r, Err(SpanError::ZeroTileSize));
}

#[test]
fn dispatch_table() {
    // All 168 entries map one to one onto (format, keyed, geometry, op).
    let mut seen = std::collections::HashSet::new();
    for row in 0..14 {
        for mode in 0..12 {
            let k = Kernel::from_row_mode(row, mode).unwrap();
            assert_eq!((k.row(), k.mode()), (row, mode));
            assert!(seen.insert((k.fmt as u32, k.keyed, k.geom as u32, k.op as u32)));
        }
    }
    assert_eq!(seen.len(), 168);
    assert!(Kernel::from_row_mode(14, 0).is_none());
    assert!(Kernel::from_row_mode(0, 12).is_none());
    let s = src16(&[0x0123]);
    let i = img(&s, 2, 1, 1);
    let mut d = buf(1, 1, 0);
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 2,
    };
    let sp = [Span::new(0, 0, 1)];
    let bad = BltDesc {
        format: 7,
        ..Default::default()
    };
    assert_eq!(
        render_blt_raw(&sp, &mut t, &i, &bad, 0),
        Err(SpanError::BadKernelIndex { row: 14, mode: 0 })
    );
    assert_eq!(
        render_blt_raw(&sp, &mut t, &i, &BltDesc::default(), 12),
        Err(SpanError::BadKernelIndex { row: 0, mode: 12 })
    );
    // Flag byte 2 on format 0 names row 2 (A1R5G5B5 unkeyed), as the sum does.
    let odd = BltDesc {
        keyed: 2,
        ..Default::default()
    };
    render_blt_raw(&sp, &mut t, &i, &odd, 0).unwrap();
    assert_eq!(px(&d, 2, 0, 0), blt_conversion_4444(SourceFormat::A1R5G5B5, 0x0123));
}

#[test]
fn direct_ignores_uv_and_reads_same_xy() {
    let s = src16(&[1, 2, 3, 4, 5, 6]);
    let i = img(&s, 6, 3, 2);
    let desc = BltDesc {
        u0: 0x1234_5678,
        v0: 99,
        dudx: 7,
        dvdy: 9,
        ..Default::default()
    };
    let mut d = buf(3, 2, 0);
    let mut t = Target {
        data: &mut d,
        origin: 0,
        stride: 6,
    };
    render_blt_raw(&[Span::new(1, 1, 2)], &mut t, &i, &desc, 0).unwrap();
    assert_eq!(row(&d, 6, 1, 3), vec![0, 5, 6]);
}
