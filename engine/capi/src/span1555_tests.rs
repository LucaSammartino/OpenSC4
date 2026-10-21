//! The §1.1 boundary conformance examples of docs/specs/span-render-1555.md
//! through the C ABI.

use crate::span1555::*;

#[test]
fn solid_example_writes_three_words() {
    // Example (1): span (2,1)..(5,1), stride 8, colour 0x12347C1F.
    let spans = [0x0001_0002u32, 0x0001_0005];
    let mut buf = [0xAAu8; 24];
    let r = unsafe { fc4_span1555_solid(spans.as_ptr(), 1, buf.as_mut_ptr(), 24, 0, 8, 0x1234_7C1F) };
    assert_eq!(r, 0);
    for (i, b) in buf.iter().enumerate() {
        let want = match i {
            12 | 14 | 16 => 0x1F,
            13 | 15 | 17 => 0x7C,
            _ => 0xAA,
        };
        assert_eq!(*b, want, "byte {i}");
    }
}

#[test]
fn blt_example_converts_a4444() {
    // Example (2): format 0 (A4444), key off, kind 0, texel 0xF8C3 -> 0xC726.
    let spans = [0u32, 1];
    let mut dst = [0u8; 2];
    let src = [0xC3u8, 0xF8];
    let rec = [0u32, 2, 1, 1, 7, 2];
    let blit = [0u32; 8];
    let r = unsafe {
        fc4_span1555_blt(
            spans.as_ptr(),
            1,
            dst.as_mut_ptr(),
            2,
            0,
            2,
            src.as_ptr(),
            2,
            0,
            rec.as_ptr(),
            blit.as_ptr(),
            0,
        )
    };
    assert_eq!(r, 0);
    assert_eq!(u16::from_le_bytes(dst), 0xC726);
}

#[test]
fn window_errors_are_codes() {
    let spans = [0x0001_0002u32, 0x0001_0005];
    let mut buf = [0u8; 8];
    // n = 0, a destination past the window, a null gradient.
    assert_eq!(
        unsafe { fc4_span1555_solid(spans.as_ptr(), 0, buf.as_mut_ptr(), 8, 0, 8, 0) },
        1
    );
    assert_eq!(
        unsafe { fc4_span1555_solid(spans.as_ptr(), 1, buf.as_mut_ptr(), 8, 0, 8, 0) },
        3
    );
    assert_eq!(
        unsafe { fc4_span1555_gradient(spans.as_ptr(), 1, buf.as_mut_ptr(), 8, 0, 8, std::ptr::null()) },
        -1
    );
}
