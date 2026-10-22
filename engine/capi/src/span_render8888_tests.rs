// SPDX-License-Identifier: GPL-3.0-or-later
// Tests of the fc4_span8888_* C ABI (glue-span8888-impl4), from the worked
// examples of docs/specs/span-render-8888.md.

use super::*;

fn dw(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

#[test]
fn solid_spec_example_and_guard_bytes() {
    // §1.1: spans [{0x00010002, 0x00010005}], surface {B, 16}, colour 0x12345678:
    // the dwords at B+24, B+28, B+32 change, nothing else.
    let spans = [0x0001_0002u32, 0x0001_0005];
    let mut buf = vec![0xA5u8; 64];
    let r = unsafe { fc4_span8888_solid(spans.as_ptr(), 1, buf.as_mut_ptr(), buf.len(), 0, 16, 0x1234_5678) };
    assert_eq!(r, OK);
    for off in (0..64).step_by(4) {
        let want = if (24..=32).contains(&off) {
            0x1234_5678
        } else {
            0xA5A5_A5A5
        };
        assert_eq!(dw(&buf, off), want, "dword at {off}");
    }
}

#[test]
fn overlapping_source_is_rejected_before_borrowing_or_writing() {
    let spans = [0u32, 1];
    let mut buf = [0xA5u8; 32];
    let mut desc = [0u8; 32];
    desc[16..20].copy_from_slice(&4u32.to_le_bytes());
    let p = buf.as_mut_ptr();
    for offset in [0, 4] {
        let r = unsafe {
            fc4_span8888_blt(
                spans.as_ptr(),
                1,
                p,
                16,
                0,
                16,
                p.add(offset),
                16,
                0,
                16,
                4,
                1,
                desc.as_ptr(),
                0,
            )
        };
        assert_eq!(r, REJECTED);
        assert_eq!(buf, [0xA5; 32]);
    }
}

#[test]
fn origin_and_negative_stride_address_upward() {
    // §2: the stride is a signed multiplier of y; pixel (0, 0) at byte `origin`.
    let spans = [0x0001_0000u32, 0x0001_0002];
    let mut buf = vec![0u8; 48];
    let r = unsafe { fc4_span8888_solid(spans.as_ptr(), 1, buf.as_mut_ptr(), buf.len(), 32, -16, 0xDEAD_BEEF) };
    assert_eq!(r, OK);
    assert_eq!(dw(&buf, 16), 0xDEAD_BEEF);
    assert_eq!(dw(&buf, 20), 0xDEAD_BEEF);
    assert_eq!(buf.iter().filter(|&&b| b != 0).count(), 8);
}

#[test]
fn out_of_window_is_rejected_and_nothing_written() {
    // A span that runs one pixel past the window: rejected, every byte kept.
    let spans = [0x0000_0000u32, 0x0000_0002, 0x0001_0000, 0x0001_0005];
    let mut buf = vec![0x11u8; 32]; // 2 rows of 16 bytes = 4 pixels each
    let r = unsafe { fc4_span8888_solid(spans.as_ptr(), 2, buf.as_mut_ptr(), buf.len(), 0, 16, 0) };
    assert_eq!(r, REJECTED);
    assert!(buf.iter().all(|&b| b == 0x11));
}

#[test]
fn null_and_zero_count_are_bad_args() {
    let spans = [0u32, 1];
    let mut buf = vec![0u8; 16];
    unsafe {
        assert_eq!(
            fc4_span8888_solid(spans.as_ptr(), 0, buf.as_mut_ptr(), 16, 0, 16, 1),
            BAD_ARGS
        );
        assert_eq!(
            fc4_span8888_solid(std::ptr::null(), 1, buf.as_mut_ptr(), 16, 0, 16, 1),
            BAD_ARGS
        );
        assert_eq!(
            fc4_span8888_solid(spans.as_ptr(), 1, std::ptr::null_mut(), 16, 0, 16, 1),
            BAD_ARGS
        );
        assert_eq!(
            fc4_span8888_gradient(spans.as_ptr(), 1, buf.as_mut_ptr(), 16, 0, 16, std::ptr::null(), false),
            BAD_ARGS
        );
        let desc = [0u8; 32];
        assert_eq!(
            fc4_span8888_blt(
                spans.as_ptr(),
                1,
                buf.as_mut_ptr(),
                16,
                0,
                16,
                std::ptr::null(),
                4,
                0,
                4,
                1,
                1,
                desc.as_ptr(),
                0
            ),
            BAD_ARGS
        );
    }
    assert!(buf.iter().all(|&b| b == 0));
}

#[test]
fn alpha_weights_full_and_zero() {
    // §4: A = 0xFF reproduces colour's RGB (w = 256); A = 0 keeps RGB, alpha byte 0xFF.
    let spans = [0u32, 1];
    let mut buf = 0x1234_5678u32.to_le_bytes().to_vec();
    assert_eq!(
        unsafe { fc4_span8888_alpha(spans.as_ptr(), 1, buf.as_mut_ptr(), 4, 0, 4, 0x00FF_0000, 0xFF00_0000,) },
        OK
    );
    assert_eq!(dw(&buf, 0), 0xFFFF_0000);
    let mut buf = 0x1234_5678u32.to_le_bytes().to_vec();
    assert_eq!(
        unsafe { fc4_span8888_alpha(spans.as_ptr(), 1, buf.as_mut_ptr(), 4, 0, 4, 0x00FF_0000, 0x00FF_FFFF,) },
        OK
    );
    assert_eq!(dw(&buf, 0), 0xFF34_5678);
}

#[test]
fn gradient_spec_example() {
    // §5: span at x = 2, y = 1, B0 = 0x00100000, dB/dx = 0x10000, dB/dy = 0x20000.
    let spans = [0x0001_0002u32, 0x0001_0005];
    let mut grad = [0u32; 12];
    grad[0] = 0x0010_0000;
    grad[4] = 0x0001_0000;
    grad[8] = 0x0002_0000;
    let mut buf = vec![0u8; 2 * 20];
    let r = unsafe {
        fc4_span8888_gradient(
            spans.as_ptr(),
            1,
            buf.as_mut_ptr(),
            buf.len(),
            0,
            20,
            grad.as_ptr(),
            false,
        )
    };
    assert_eq!(r, OK);
    assert_eq!(dw(&buf, 28), 0xFF00_0014);
    assert_eq!(dw(&buf, 32), 0xFF00_0015);
    assert_eq!(dw(&buf, 36), 0xFF00_0016);
    assert_eq!(dw(&buf, 24), 0);
}

#[test]
fn alpha_gradient_full_weight_writes_colour() {
    // §6: W = 256 << 16 gives B(d, s, 256) = s's RGB with alpha 0xFF.
    let spans = [0u32, 2];
    let mut grad = [0u32; 12];
    grad[2] = 0x0080_0000; // R0 = 0x80
    grad[3] = 0x0100_0000; // W0 = 256
    let mut buf = vec![0x55u8; 8];
    let r = unsafe { fc4_span8888_gradient(spans.as_ptr(), 1, buf.as_mut_ptr(), 8, 0, 8, grad.as_ptr(), true) };
    assert_eq!(r, OK);
    assert_eq!(dw(&buf, 0), 0xFF80_0000);
    assert_eq!(dw(&buf, 4), 0xFF80_0000);
}

#[test]
fn blt_copy_formats_and_key() {
    // §8/§9.2: format 3 (565) direct copy widens; keyed row skips the key texel.
    let spans = [0u32, 2];
    let src: Vec<u8> = [0xF800u16, 0x001F].iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut desc = [0u8; 32];
    desc[16] = 3; // format 565
    let mut buf = vec![0u8; 8];
    let r = unsafe {
        fc4_span8888_blt(
            spans.as_ptr(),
            1,
            buf.as_mut_ptr(),
            8,
            0,
            8,
            src.as_ptr(),
            4,
            0,
            4,
            2,
            1,
            desc.as_ptr(),
            0,
        )
    };
    assert_eq!(r, OK);
    assert_eq!(dw(&buf, 0), 0xFFFF_0000);
    assert_eq!(dw(&buf, 4), 0xFF00_00FF);
    // keyed: key = 0x001F skips texel 1.
    desc[24] = 0x1F;
    desc[28] = 1;
    let mut buf = vec![0x77u8; 8];
    let r = unsafe {
        fc4_span8888_blt(
            spans.as_ptr(),
            1,
            buf.as_mut_ptr(),
            8,
            0,
            8,
            src.as_ptr(),
            4,
            0,
            4,
            2,
            1,
            desc.as_ptr(),
            0,
        )
    };
    assert_eq!(r, OK);
    assert_eq!(dw(&buf, 0), 0xFFFF_0000);
    assert_eq!(dw(&buf, 4), 0x7777_7777);
}

#[test]
fn blt_bad_index_and_short_source_rejected() {
    let spans = [0u32, 2];
    let src = [0u8; 8];
    let mut desc = [0u8; 32];
    desc[16] = 7; // format past the table (U2)
    let mut buf = vec![0x33u8; 8];
    unsafe {
        assert_eq!(
            fc4_span8888_blt(
                spans.as_ptr(),
                1,
                buf.as_mut_ptr(),
                8,
                0,
                8,
                src.as_ptr(),
                8,
                0,
                8,
                2,
                1,
                desc.as_ptr(),
                0
            ),
            REJECTED
        );
        desc[16] = 4;
        assert_eq!(
            fc4_span8888_blt(
                spans.as_ptr(),
                1,
                buf.as_mut_ptr(),
                8,
                0,
                8,
                src.as_ptr(),
                8,
                0,
                8,
                2,
                1,
                desc.as_ptr(),
                12
            ),
            REJECTED
        );
        // A8R8G8B8 needs 8 bytes for 2 texels; give 7.
        assert_eq!(
            fc4_span8888_blt(
                spans.as_ptr(),
                1,
                buf.as_mut_ptr(),
                8,
                0,
                8,
                src.as_ptr(),
                7,
                0,
                8,
                2,
                1,
                desc.as_ptr(),
                0
            ),
            REJECTED
        );
    }
    assert!(buf.iter().all(|&b| b == 0x33));
}
