// SPDX-License-Identifier: GPL-3.0-or-later
//! Packed colors and the distinct unpack/copy alpha rules, gz-framework-core §6.3–6.4.
pub fn key_mask(kind: u32) -> u32 {
    match kind {
        4 | 6 => 0x7fff,
        5 => 0xffff,
        7 => 0x0fff,
        8 | 9 => 0x00ff_ffff,
        _ => u32::MAX,
    }
}
pub fn map_rgba(kind: u32, r: u32, g: u32, b: u32, a: u32) -> u32 {
    let (r, g, b, a) = (r & 255, g & 255, b & 255, a & 255);
    match kind {
        4 | 6 => (a >> 7) << 15 | (r >> 3) << 10 | (g >> 3) << 5 | b >> 3,
        5 => (r >> 3) << 11 | (g >> 2) << 5 | b >> 3,
        7 => (a >> 4) << 12 | (r >> 4) << 8 | (g >> 4) << 4 | b >> 4,
        8 | 9 => a << 24 | r << 16 | g << 8 | b,
        _ => 0,
    }
}
pub fn unmap_rgba(kind: u32, p: u32) -> Option<[u8; 4]> {
    let rgb = match kind {
        4 | 6 => [
            expand((p >> 10) & 31, 5, 8),
            expand((p >> 5) & 31, 5, 8),
            expand(p & 31, 5, 8),
        ],
        5 => [
            expand((p >> 11) & 31, 5, 8),
            expand((p >> 5) & 63, 6, 8),
            expand(p & 31, 5, 8),
        ],
        7 => [((p >> 8) & 15) * 17, ((p >> 4) & 15) * 17, (p & 15) * 17],
        8 | 9 => [(p >> 16) & 255, (p >> 8) & 255, p & 255],
        _ => return None,
    };
    let a = match kind {
        4 | 6 => {
            if p & 0x8000 != 0 {
                132
            } else {
                0
            }
        }
        5 => 255,
        7 => {
            if p & 0xf000 == 0xf000 {
                255
            } else {
                (p >> 8) & 0xf0
            }
        }
        _ => p >> 24,
    };
    Some([rgb[0] as u8, rgb[1] as u8, rgb[2] as u8, a as u8])
}
fn expand(v: u32, from: u32, to: u32) -> u32 {
    if from >= to {
        v >> (from - to)
    } else if from == 1 {
        if v == 0 {
            0
        } else {
            (1 << to) - 1
        }
    } else {
        (v << (to - from)) | (v >> (2 * from - to))
    }
}
pub fn convert_pixel(source: u32, destination: u32, p: u32) -> u32 {
    let source = if (4..=9).contains(&source) { source } else { 4 };
    let destination = if (4..=9).contains(&destination) { destination } else { 4 };
    let (r, g, b, rb, gb) = match source {
        4 | 6 => ((p >> 10) & 31, (p >> 5) & 31, p & 31, 5, 5),
        5 => ((p >> 11) & 31, (p >> 5) & 63, p & 31, 5, 6),
        7 => ((p >> 8) & 15, (p >> 4) & 15, p & 15, 4, 4),
        _ => ((p >> 16) & 255, (p >> 8) & 255, p & 255, 8, 8),
    };
    let (a, ab) = match source {
        6 => ((p >> 15) & 1, 1),
        7 => ((p >> 12) & 15, 4),
        9 => (p >> 24, 8),
        _ => (1, 1),
    };
    match destination {
        4 | 6 => {
            let a = if destination == 4 { 1 } else { expand(a, ab, 1) };
            a << 15 | expand(r, rb, 5) << 10 | expand(g, gb, 5) << 5 | expand(b, rb, 5)
        }
        5 => expand(r, rb, 5) << 11 | expand(g, gb, 6) << 5 | expand(b, rb, 5),
        7 => expand(a, ab, 4) << 12 | expand(r, rb, 4) << 8 | expand(g, gb, 4) << 4 | expand(b, rb, 4),
        _ => {
            let a = if destination == 8 { 255 } else { expand(a, ab, 8) };
            a << 24 | expand(r, rb, 8) << 16 | expand(g, gb, 8) << 8 | expand(b, rb, 8)
        }
    }
}
