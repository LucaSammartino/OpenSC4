// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
#[test]
fn gzb4_alpha_unpack_is_distinct_from_blit_conversion() {
    assert_eq!(map_rgba(6, 18, 52, 86, 128), 0x88ca);
    assert_eq!(unmap_rgba(6, 0x88ca), Some([16, 49, 82, 132]));
    assert_eq!(map_rgba(7, 18, 52, 86, 128), 0x8135);
    assert_eq!(unmap_rgba(7, 0x8135), Some([17, 51, 85, 128]));
    assert_eq!(unmap_rgba(7, 0xf135), Some([17, 51, 85, 255]));
    assert_eq!(unmap_rgba(2, 0xffff), None);
    assert_eq!(convert_pixel(6, 9, 0x7fff), 0x00ff_ffff);
    assert_eq!(convert_pixel(4, 9, 0), 0xff00_0000);
    assert_eq!(convert_pixel(9, 6, 0xff44_5566), 0xa14c);
}
#[test]
fn all_36_converters_use_channel_precision_and_opaque_spare_bits() {
    for source in 4..=9 {
        for destination in 4..=9 {
            assert_eq!(
                convert_pixel(source, destination, 0xffff_ffff),
                match destination {
                    4..=6 => 0xffff,
                    7 => 0xffff,
                    _ => 0xffff_ffff,
                }
            );
        }
    }
    // 5-bit expansion repeats high bits, rather than normalized division.
    assert_eq!(convert_pixel(5, 9, 4 << 11), 0xff21_0000);
    assert_eq!(convert_pixel(7, 5, 0x0123), (2 << 11) | (8 << 5) | 6);
}
