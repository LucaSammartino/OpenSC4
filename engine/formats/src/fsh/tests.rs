// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests for the FSH reader: `docs/specs/graphics-formats.md` §8.1 (tests 1–20, numbered as there; 19 and 20 are
//! rebuilt synthetically here, the game-data sweep checks the shipped records), the §2.11 decoder, and hostile
//! input.

use super::*;
use crate::dbpf::{Compression, Package, PackageBuilder};
use crate::tgi::Tgi;

/// Bytes from hexadecimal text; spaces are ignored.
fn hex(s: &str) -> Vec<u8> {
    let digits: Vec<u8> = s.bytes().filter(u8::is_ascii_hexdigit).collect();
    assert_eq!(digits.len() % 2, 0, "odd hex string {s:?}");
    digits
        .chunks(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect()
}

/// §8: "FSH(entry)": `SHPI`, size (the record length), one entry, `G264`, directory entry `0000` → `0x18`, then
/// the entry.
fn fsh1(entry: &[u8]) -> Vec<u8> {
    let mut d = b"SHPI".to_vec();
    d.extend_from_slice(&(0x18 + entry.len() as u32).to_le_bytes());
    d.extend_from_slice(&1u32.to_le_bytes());
    d.extend_from_slice(b"G264");
    d.extend(hex("30 30 30 30 18 00 00 00"));
    d.extend_from_slice(entry);
    d
}

/// An entry: code, `next`, width, height, the `+0x0C` dword, then the pixels.
fn entry(code: u8, next: u32, w: u16, h: u16, misc: u32, pixels: &[u8]) -> Vec<u8> {
    let mut e = vec![code];
    e.extend_from_slice(&next.to_le_bytes()[..3]);
    e.extend_from_slice(&w.to_le_bytes());
    e.extend_from_slice(&h.to_le_bytes());
    e.extend_from_slice(&[0, 0, 0, 0]);
    e.extend_from_slice(&misc.to_le_bytes());
    e.extend_from_slice(pixels);
    e
}

fn px(img: &Image) -> Vec<[u8; 4]> {
    img.rgba.chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect()
}

fn test1() -> Vec<u8> {
    fsh1(&hex(
        "60 00 00 00 04 00 04 00 00 00 00 00 00 00 00 00  00 F8 1F 00 E4 E4 E4 E4",
    ))
}

fn test5() -> Vec<u8> {
    fsh1(&hex(
        "7D 00 00 00 02 00 02 00 00 00 00 00 00 00 00 00  FF 00 00 80 00 FF 00 FF 00 00 FF 00 11 22 33 44",
    ))
}

/// Test 7's record with a palette block of `code`, `width`, `bytes_per_entry`, and the entry's `next`.
fn test7(pal_code: u8, pal_width: u16, entry_size: usize, next: u32) -> Vec<u8> {
    let mut pixels = hex("00 01 80 FF");
    pixels.extend_from_slice(&[0; 12]);
    let mut e = entry(0x7B, next, 2, 2, 0, &pixels);
    let mut pal = vec![pal_code, 0, 0, 0];
    pal.extend_from_slice(&pal_width.to_le_bytes());
    pal.extend(hex("01 00 00 01 00 00 00 00 00 00"));
    e.extend(pal);
    for k in 0..256u32 {
        let k = k as u8;
        match entry_size {
            3 => e.extend_from_slice(&[k, k, k]),
            _ => e.extend_from_slice(&[k, k, k, 0xFF]),
        }
    }
    fsh1(&e)
}

fn rejected(data: &[u8]) -> bool {
    Fsh::parse_record(TYPE_FSH, data).is_err()
}

fn accepted(data: &[u8]) -> Fsh {
    Fsh::parse_record(TYPE_FSH, data).unwrap()
}

#[test]
fn t01_dxt1_four_colour() {
    let d = test1();
    assert_eq!(d.len(), 0x30);
    let f = accepted(&d);
    assert_eq!(f.format(), (TextureFormat::Dxt1Rgb, ComponentType::Byte));
    assert_eq!((f.format().0.number(), f.format().1.number()), (7, 0));
    assert_eq!(
        f.levels,
        vec![Level {
            offset: 0,
            size: 8,
            width: 4,
            height: 4
        }]
    );
    let img = f.decode(0, 0).unwrap();
    let row = [[255, 0, 0, 255], [0, 0, 255, 255], [170, 0, 85, 255], [85, 0, 170, 255]];
    for r in px(&img).chunks(4) {
        assert_eq!(r, row);
    }
}

#[test]
fn t02_dxt1_three_colour() {
    let pixels = hex("1F 00 00 F8 F4 F4 F4 F4");
    let row = [[0, 0, 255, 255], [255, 0, 0, 255], [0, 0, 0, 0], [0, 0, 0, 0]];
    for (misc, format) in [
        (1, TextureFormat::Dxt1Rgba),
        (0, TextureFormat::Dxt1Rgb),
        (0xFFF, TextureFormat::Dxt1Rgba),
    ] {
        let f = accepted(&fsh1(&entry(0x60, 0, 4, 4, misc, &pixels)));
        assert_eq!(f.format().0, format);
        // A = 0 or not, FreeCity4 decodes index 3 as transparent black (§2.11).
        for r in px(&f.decode(0, 0).unwrap()).chunks(4) {
            assert_eq!(r, row);
        }
    }
    assert_eq!(EntryHeader::parse(&entry(0x60, 0, 4, 4, 0xFFF, &[])).unwrap().a(), -1);
    assert_eq!(format_and_type(0x60, 8).unwrap().0.number(), 8);
}

#[test]
fn t03_dxt3() {
    let d = fsh1(&hex(
        "61 00 00 00 04 00 04 00 00 00 00 00 00 00 00 00  10 32 54 76 98 BA DC FE 00 F8 00 F8 00 00 00 00",
    ));
    assert_eq!(d.len(), 0x38);
    let f = accepted(&d);
    assert_eq!(f.format().0.number(), 9);
    for (i, p) in px(&f.decode(0, 0).unwrap()).into_iter().enumerate() {
        assert_eq!(p, [255, 0, 0, 17 * i as u8]);
    }
}

#[test]
fn t04_dxt5() {
    let d = fsh1(&entry(
        0x62,
        0,
        4,
        4,
        0,
        &hex("FF 00 08 82 20 08 82 20 00 F8 00 F8 00 00 00 00"),
    ));
    assert_eq!(d.len(), 0x38);
    let f = accepted(&d);
    assert_eq!(f.format().0.number(), 10);
    for (i, p) in px(&f.decode(0, 0).unwrap()).into_iter().enumerate() {
        assert_eq!(p, [255, 0, 0, if i % 2 == 0 { 255 } else { 0 }]);
    }
}

#[test]
fn t05_8888() {
    let d = test5();
    assert_eq!(d.len(), 0x38);
    let f = accepted(&d);
    assert_eq!((f.format().0.number(), f.format().1.number()), (3, 1));
    assert_eq!(f.levels[0].size, 16);
    assert_eq!(
        px(&f.decode(0, 0).unwrap()),
        [
            [0, 0, 255, 128],
            [0, 255, 0, 255],
            [255, 0, 0, 0],
            [0x33, 0x22, 0x11, 0x44]
        ]
    );
}

#[test]
fn t06_888() {
    let d = fsh1(&entry(0x7F, 0, 2, 2, 0, &hex("FF 00 00 00 FF 00 00 00 FF 10 20 30")));
    assert_eq!(d.len(), 0x34);
    let f = accepted(&d);
    assert_eq!((f.format().0.number(), f.format().1.number()), (2, 1));
    assert_eq!(f.levels[0].size, 12);
    assert_eq!(
        px(&f.decode(0, 0).unwrap()),
        [
            [0, 0, 255, 255],
            [0, 255, 0, 255],
            [255, 0, 0, 255],
            [0x30, 0x20, 0x10, 255]
        ]
    );
}

#[test]
fn t07_8bit_with_palette() {
    let d = test7(0x24, 256, 3, 0x20);
    assert_eq!(d.len(), 0x348);
    assert_eq!(
        &d[0x38..0x48],
        &hex("24 00 00 00 00 01 01 00 00 01 00 00 00 00 00 00")[..]
    );
    let f = accepted(&d);
    assert_eq!((f.format().0.number(), f.format().1.number()), (5, 0));
    let pal = f.frames[0].palette.unwrap();
    assert_eq!((pal.offset, pal.code, pal.width, pal.height), (0x38, 0x24, 256, 1));
    let grey = [
        [0, 0, 0, 255],
        [1, 1, 1, 255],
        [128, 128, 128, 255],
        [255, 255, 255, 255],
    ];
    assert_eq!(px(&f.decode(0, 0).unwrap()), grey);
    // 7a: palette width 255.
    assert!(rejected(&test7(0x24, 255, 3, 0x20)));
    // 7b: code 0x22 is not searched, and its next is 0: no palette.
    assert!(rejected(&test7(0x22, 256, 3, 0x20)));
    // 7c: an 8888 palette.
    let d = test7(0x2A, 256, 4, 0x20);
    assert_eq!(d.len(), 0x448);
    assert_eq!(px(&accepted(&d).decode(0, 0).unwrap()), grey);
    // 7d: next = 0, the palette is unreachable.
    assert!(rejected(&test7(0x24, 256, 3, 0)));
}

#[test]
fn t07e_palette_chain_and_bounds() {
    // The search walks past other attachments (a 0x70 name block) to the palette.
    let mut pixels = hex("00 01 80 FF");
    pixels.extend_from_slice(&[0; 12]);
    let mut e = entry(0x7B, 0x20, 2, 2, 0, &pixels);
    e.extend(entry(0x70, 0x10, 0, 0, 0, &[]));
    e.extend(entry(0x29, 0, 256, 1, 0, &[]));
    for k in 0..256u16 {
        e.extend_from_slice(&(k << 8).to_le_bytes()); // R5G6B5: k's top bits in red and green
    }
    let f = accepted(&fsh1(&e));
    assert_eq!(f.frames[0].palette.unwrap().code, 0x29);
    assert_eq!(
        f.attachments(0).iter().map(|a| a.code).collect::<Vec<_>>(),
        [0x70, 0x29]
    );
    assert_eq!(f.decode(0, 0).unwrap().pixel(0, 0), Some([0, 0, 0, 255]));
    // A palette whose 256 entries leave the record is rejected [choice].
    let mut short = fsh1(&e);
    short.truncate(short.len() - 2);
    let len = short.len() as u32;
    short[4..8].copy_from_slice(&len.to_le_bytes());
    assert!(rejected(&short));
    // A chain that leaves the record is rejected [choice].
    let mut out = test7(0x24, 256, 3, 0x20);
    out[0x18 + 1] = 0xF0;
    out[0x18 + 2] = 0xFF;
    assert!(rejected(&out));
}

#[test]
fn t08_4444() {
    let f = accepted(&fsh1(&entry(0x6D, 0, 2, 2, 0, &hex("00 8F F0 00 0F 00 FF FF"))));
    assert_eq!((f.format().0.number(), f.format().1.number()), (3, 8));
    assert_eq!(
        px(&f.decode(0, 0).unwrap()),
        [[255, 0, 0, 136], [0, 255, 0, 0], [0, 0, 255, 0], [255, 255, 255, 255]]
    );
}

fn u16s(v: &[u16]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

#[test]
fn t09_1555() {
    let f = accepted(&fsh1(&entry(
        0x7E,
        0,
        2,
        2,
        0,
        &u16s(&[0xFC00, 0x03E0, 0x801F, 0x7FFF]),
    )));
    assert_eq!((f.format().0.number(), f.format().1.number()), (3, 9));
    assert_eq!(
        px(&f.decode(0, 0).unwrap()),
        [[255, 0, 0, 255], [0, 255, 0, 0], [0, 0, 255, 255], [255, 255, 255, 0]]
    );
}

#[test]
fn t10_565() {
    let f = accepted(&fsh1(&entry(
        0x78,
        0,
        2,
        2,
        0,
        &u16s(&[0xF800, 0x07E0, 0x001F, 0x0000]),
    )));
    assert_eq!((f.format().0.number(), f.format().1.number()), (2, 11));
    assert_eq!(
        px(&f.decode(0, 0).unwrap()),
        [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [0, 0, 0, 255]]
    );
}

#[test]
fn t11_alpha_and_luminance_alpha() {
    let f = accepted(&fsh1(&entry(0x64, 0, 2, 2, 0, &hex("00 55 AA FF"))));
    assert_eq!(f.format().0.number(), 4);
    assert_eq!(
        px(&f.decode(0, 0).unwrap()),
        [
            [255, 255, 255, 0],
            [255, 255, 255, 85],
            [255, 255, 255, 170],
            [255, 255, 255, 255]
        ]
    );
    let f = accepted(&fsh1(&entry(0x65, 0, 2, 2, 0, &hex("10 20 30 40 50 60 70 80"))));
    assert_eq!(f.format().0.number(), 6);
    assert_eq!(
        px(&f.decode(0, 0).unwrap()),
        [
            [0x10, 0x10, 0x10, 0x20],
            [0x30, 0x30, 0x30, 0x40],
            [0x50, 0x50, 0x50, 0x60],
            [0x70, 0x70, 0x70, 0x80]
        ]
    );
}

#[test]
fn t12_rejected_codes() {
    for c in [0x7A, 0x63, 0x6E, 0x80, 0xE0] {
        let mut d = test1();
        d[0x18] = c;
        assert!(rejected(&d), "code {c:#04X}");
    }
    // Every code of the table is accepted.
    for c in [0x60, 0x61, 0x62, 0x64, 0x65, 0x6D, 0x78, 0x7D, 0x7E, 0x7F] {
        assert!(format_and_type(c, 0).is_some());
    }
    assert!(format_and_type(0x7B, 0).is_some());
}

#[test]
fn t13_height_and_width() {
    let mut d = test5();
    d[0x18 + 6] = 3;
    assert!(rejected(&d));
    d[0x18 + 6] = 0;
    let f = accepted(&d);
    assert_eq!((f.width, f.height), (2, 1));
    assert_eq!(f.levels[0].size, 8);
    assert_eq!(&f.level_data(0, 0).unwrap()[..], &hex("FF 00 00 80 00 FF 00 FF")[..]);
    // Width is not checked.
    let f = accepted(&fsh1(&entry(0x7D, 0, 3, 2, 0, &[0x40; 24])));
    assert_eq!(f.levels[0].size, 24);
    assert_eq!((f.width, f.height), (3, 2));
    // A negative (sign-extended) height fails the power-of-two test.
    let mut d = test5();
    d[0x18 + 6] = 0x00;
    d[0x18 + 7] = 0x80;
    assert!(rejected(&d));
}

#[test]
fn t14_level_layout() {
    let offsets = |l: &[Level]| l.iter().map(|l| (l.offset, l.size)).collect::<Vec<_>>();
    let t = level_table(0x60, 8, 8, 3).unwrap();
    assert_eq!(offsets(&t), [(0, 32), (32, 8), (48, 8), (64, 8)]);
    let t = level_table(0x7D, 4, 4, 2).unwrap();
    assert_eq!(offsets(&t), [(0, 64), (64, 16), (80, 4)]);
    assert_eq!(
        t.iter().map(|l| (l.width, l.height)).collect::<Vec<_>>(),
        [(4, 4), (2, 2), (1, 1)]
    );
    let t = level_table(0x61, 2, 2, 1).unwrap();
    assert_eq!(offsets(&t), [(0, 16), (16, 16)]);
    // The same tables through records with the padded layout: accepted, no zero-fill.
    let f = accepted(&fsh1(&entry(0x60, 0, 8, 8, 3 << 28, &[0x55; 72])));
    assert_eq!(offsets(&f.levels), [(0, 32), (32, 8), (48, 8), (64, 8)]);
    assert!(!f.frames[0].zero_filled);
    for level in 0..4 {
        f.decode(0, level).unwrap();
    }
    let f = accepted(&fsh1(&entry(0x7D, 0, 4, 4, 2 << 28, &[0x11; 84])));
    assert_eq!(f.decode(0, 2).unwrap().rgba, [0x11; 4]);
    assert_eq!((f.width, f.height), (4, 4));
    let f = accepted(&fsh1(&entry(0x61, 0, 2, 2, 1 << 28, &[0; 32])));
    assert_eq!((f.width, f.height), (4, 4));
    // Levels that do not fit even packed are rejected [choice].
    assert!(rejected(&fsh1(&entry(0x60, 0, 8, 8, 3 << 28, &[0x55; 40]))));
}

/// A record with two `0x7D` entries at `0x20` and `0x40`.
fn two_frames(second: &[u8]) -> Vec<u8> {
    let mut d = b"SHPI".to_vec();
    d.extend_from_slice(&(0x40 + second.len() as u32).to_le_bytes());
    d.extend_from_slice(&2u32.to_le_bytes());
    d.extend_from_slice(b"G264");
    d.extend(hex("30 30 30 30 20 00 00 00 30 30 30 31 40 00 00 00"));
    d.extend(entry(0x7D, 0, 2, 2, 0, &[0x01; 16]));
    d.extend_from_slice(second);
    d
}

#[test]
fn t15_frames() {
    let f = accepted(&two_frames(&entry(0x7D, 0, 2, 2, 0, &[0x02; 16])));
    assert_eq!(f.frames.len(), 2);
    assert_eq!(f.decode(1, 0).unwrap().rgba, [0x02; 16]);
    assert_eq!(f.frames[0].data_end, 0x40);
    assert!(rejected(&two_frames(&entry(0x7D, 0, 2, 4, 0, &[0x02; 32]))));
    assert!(rejected(&two_frames(&entry(0x7F, 0, 2, 2, 0, &[0x02; 12]))));
    assert!(rejected(&two_frames(&entry(0x7D, 0, 2, 2, 1 << 28, &[0x02; 20]))));
    // Width is not compared: the second frame is read with frame 0's 2 × 2 layout.
    let mut wide = vec![0x03; 16];
    wide.extend_from_slice(&[0x04; 16]);
    let f = accepted(&two_frames(&entry(0x7D, 0, 4, 2, 0, &wide)));
    assert_eq!(f.levels[0].size, 16);
    assert_eq!(f.decode(1, 0).unwrap().rgba, [0x03; 16]);
}

#[test]
fn t16_size_field() {
    let mut d = test5();
    d.extend_from_slice(&[0xAA; 4]);
    let f = accepted(&d);
    assert_eq!(f.data().len(), 0x38);
    let mut d = test5();
    d[4] = 0x3C;
    assert!(matches!(Fsh::parse(&d), Err(Error::Truncated { .. })));
    // [choice] size below the header and directory.
    let mut d = test5();
    d[4] = 0x17;
    assert!(rejected(&d));
    // [choice] no entry.
    let mut d = test5();
    d[8] = 0;
    assert!(rejected(&d));
    // [choice] a directory offset outside the record.
    let mut d = test5();
    d[0x14] = 0x30;
    assert!(matches!(Fsh::parse(&d), Err(Error::OutOfBounds { .. })));
}

#[test]
fn t17_magic_and_compression() {
    let mut d = test5();
    d[3] = 0x48;
    assert!(matches!(Fsh::parse(&d), Err(Error::BadMagic { .. })));
    let key = Tgi::new(TYPE_FSH, 0x1ABE_787D, 1);
    let unlisted = Tgi::new(TYPE_FSH, 0x1ABE_787D, 2);
    let compressed = crate::qfs::compress(&test5()).unwrap();
    let mut b = PackageBuilder::new();
    b.add(key, &test5(), Compression::Always).unwrap();
    b.add_stored(unlisted, compressed, None);
    let pkg = Package::from_bytes(b.build().unwrap()).unwrap();
    assert!(pkg.is_compressed(pkg.find(&key).unwrap()));
    assert_eq!(accepted(&pkg.read_tgi(&key).unwrap()).data(), &test5()[..]);
    assert!(matches!(
        Fsh::parse_record(TYPE_FSH, &pkg.read_tgi(&unlisted).unwrap()),
        Err(Error::BadMagic { .. })
    ));
}

#[test]
fn t18_type() {
    assert!(matches!(
        Fsh::parse_record(TYPE_FSH_UNCREATED, &test5()),
        Err(Error::Unsupported { .. })
    ));
    assert!(Fsh::parse_record(0x1234_5678, &test5()).is_err());
}

/// Test 19 rebuilt: DXT1 128 × 128, M = 7, A = 1, levels packed back to back (10,936 bytes) and padded to 10,944,
/// then a `0x70` attachment.
#[test]
fn t19_unpadded_mips() {
    let mut pixels: Vec<u8> = (0..10_936u32).map(|i| (i % 251) as u8 | 1).collect();
    pixels.resize(10_944, 0);
    let mut e = entry(0x60, 16 + 10_944, 128, 128, (7 << 28) | 1, &pixels);
    e.extend(entry(0x70, 0, 0, 0, 0, b"name"));
    let f = accepted(&fsh1(&e));
    assert_eq!(f.format().0, TextureFormat::Dxt1Rgba);
    assert_eq!(
        f.levels.iter().map(|l| l.offset).collect::<Vec<_>>(),
        [0, 8192, 10240, 10752, 10880, 10912, 10928, 10944]
    );
    assert_eq!(f.frames[0].data_end, f.frames[0].pixel_start() + 10_944);
    assert!(f.frames[0].zero_filled);
    // Level 6 is read at the aligned offset 10928 (inside the entry), level 7 entirely past it: zeros.
    assert_eq!(&f.level_data(0, 6).unwrap()[..], &pixels[10_928..10_936]);
    assert!(matches!(f.level_data(0, 6).unwrap(), Cow::Borrowed(_)));
    assert_eq!(&f.level_data(0, 7).unwrap()[..], &[0; 8]);
    // An all-zero DXT1 block: c0 = c1 = 0, 3-colour mode, index 0 = opaque black.
    assert_eq!(f.decode(0, 7).unwrap().rgba, [0, 0, 0, 255].repeat(16));
    for level in 0..8 {
        let img = f.decode(0, level).unwrap();
        assert_eq!(img.rgba.len() as u32, img.width * img.height * 4);
    }
    // Straddling the end: the level's head is copied, its tail reads as zero.
    let mut short = pixels.clone();
    short.truncate(10_932);
    let mut e = entry(0x60, 16 + 10_932, 128, 128, 6 << 28, &short);
    e.extend(entry(0x70, 0, 0, 0, 0, b"name"));
    let f = accepted(&fsh1(&e));
    let l6 = f.level_data(0, 6).unwrap();
    assert_eq!(&l6[..4], &pixels[10_928..10_932]);
    assert_eq!(&l6[4..], &[0; 4]);
}

/// Test 20 rebuilt: the fallback texture's shape (576 bytes, `G264`, one DXT1 32 × 32 entry, M = 0).
#[test]
fn t20_fallback_texture_shape() {
    let mut d = hex("53 48 50 49 40 02 00 00 01 00 00 00 47 32 36 34");
    d.extend(hex("30 30 30 30 20 00 00 00"));
    d.resize(0x20, 0);
    d.extend(entry(0x60, 0, 32, 32, 0, &[0x5A; 512]));
    d.resize(576, 0);
    let f = accepted(&d);
    assert_eq!(f.directory_id, *b"G264");
    assert_eq!((f.code(), f.width, f.height, f.mip_count()), (0x60, 32, 32, 0));
    assert_eq!(f.format().0.number(), 7);
}

#[test]
fn decoder_edges() {
    // DXT5 with a0 <= a1: indices 6 and 7 are 0 and 255, 2-5 interpolate by fifths.
    assert_eq!(dxt5_alphas(10, 60), [10, 60, 20, 30, 40, 50, 0, 255]);
    assert_eq!(dxt5_alphas(70, 0), [70, 0, 60, 50, 40, 30, 20, 10]);
    // DXT1 c0 == c1 is 3-colour mode; DXT3 ignores the mode.
    let t = colour_block(&hex("00 F8 00 F8 FF FF FF FF"), false);
    assert_eq!(t[0], [0, 0, 0, 0]);
    let t = colour_block(&hex("00 F8 00 F8 FF FF FF FF"), true);
    assert_eq!(t[0], [255, 0, 0, 255]);
    // Bit replication.
    assert_eq!(rgb565(0x8410), [132, 130, 132]);
    // DXT with a width that is not a multiple of 4: 6 × 4 reads 2 blocks, the second one zero-filled.
    let f = accepted(&fsh1(&entry(
        0x60,
        0,
        6,
        4,
        0,
        &hex("00 F8 00 00 00 00 00 00 00 00 00 00"),
    )));
    assert_eq!(f.levels[0].size, 12);
    let img = f.decode(0, 0).unwrap();
    assert_eq!((img.width, img.height), (6, 4));
    assert_eq!(img.pixel(0, 0), Some([255, 0, 0, 255]));
    assert_eq!(img.pixel(5, 3), Some([0, 0, 0, 255]));
    assert_eq!(img.pixel(6, 0), None);
    // The free decoder checks its input length and the palette.
    assert!(decode_pixels(0x7D, 2, 2, &[0; 15], None).is_err());
    assert!(decode_pixels(0x7B, 1, 1, &[0], None).is_err());
    assert!(decode_pixels(0x7A, 1, 1, &[0], None).is_err());
    let pal = [7u8; 6];
    assert!(decode_pixels(
        0x7B,
        1,
        1,
        &[3],
        Some(PaletteData {
            code: 0x2D,
            entries: &pal
        })
    )
    .is_err());
    // Out-of-range frame or level.
    let f = accepted(&test5());
    assert!(f.decode(1, 0).is_err());
    assert!(f.decode(0, 1).is_err());
    assert!(f.level_data(0, 1).is_none());
    assert!(dx7_creatable(TextureFormat::Bgra, ComponentType::UnsignedByte));
    assert!(!dx7_creatable(TextureFormat::Bgra, ComponentType::UnsignedShort4444));
    assert!(dx7_creatable(TextureFormat::Dxt1Rgba, ComponentType::Byte));
}

/// Truncations and single-byte corruptions of every fixture: parse and decode return, never panic.
#[test]
fn hostile_input() {
    let fixtures = [
        test1(),
        test5(),
        test7(0x24, 256, 3, 0x20),
        test7(0x2A, 256, 4, 0x20),
        two_frames(&entry(0x7D, 0, 2, 2, 0, &[0x02; 16])),
        fsh1(&entry(0x60, 0, 8, 8, 3 << 28, &[0x55; 72])),
        fsh1(&entry(0x6D, 0, 2, 2, 0, &hex("00 8F F0 00 0F 00 FF FF"))),
    ];
    let exercise = |d: &[u8]| {
        if let Ok(f) = Fsh::parse(d) {
            for frame in 0..f.frames.len() {
                for level in 0..f.levels.len() {
                    let _ = f.decode(frame, level);
                }
                let _ = f.attachments(frame);
            }
        }
    };
    for d in &fixtures {
        for n in 0..d.len() {
            exercise(&d[..n]);
        }
        for i in 0..d.len() {
            for v in [0x00, 0x01, 0x10, 0x7F, 0x80, 0xFF] {
                let mut m = d.clone();
                m[i] = v;
                exercise(&m);
            }
        }
    }
    // Huge dimensions with tiny data are rejected before anything large is allocated.
    assert!(rejected(&fsh1(&entry(0x7D, 0, 0x7FFF, 0x4000, 0xF << 28, &[0; 16]))));
}
