// SPDX-License-Identifier: GPL-3.0-or-later
//! Conformance tests for drawing-fonts.md, mostly its worked examples.

use std::collections::HashMap;
use std::sync::Arc;

use super::font::{FontObj, GzFont};
use super::layout::Rect;
use super::*;

#[path = "contract_tests.rs"]
mod contract_tests;

#[derive(Clone)]
struct MockFace {
    family: String,
    style: u32,
    fixed: bool,
    adv: HashMap<u32, i32>,
    metrics: SizeMetrics,
}

impl FontFace for MockFace {
    fn family_name(&self) -> String {
        self.family.clone()
    }
    fn style_flags(&self) -> u32 {
        self.style
    }
    fn is_fixed_size(&self) -> bool {
        self.fixed
    }
    fn char_index(&self, ch: u32) -> Option<u32> {
        Some(if self.adv.contains_key(&ch) { ch } else { 0 })
    }
    fn set_char_size(&mut self, _: u32, _: u32, _: u32, _: f32, _: f32) -> Option<SizeMetrics> {
        Some(self.metrics)
    }
    fn fixed_advance(&self) -> i32 {
        9 << 16
    }
    fn glyph_name_to_code(&self, name: &str) -> Option<u32> {
        (name == "uni4E00").then_some(0x41)
    }
    fn load_glyph(&mut self, index: u32, levels: GreyLevels, _flags: u16) -> Option<GlyphRaster> {
        let a = *self.adv.get(&index)?;
        let w = a.max(0) as u32;
        let (pitch, fill) = match levels {
            GreyLevels::Two => (w.div_ceil(8), 0xAA),
            GreyLevels::Levels256 => (w, 0x80),
        };
        Some(GlyphRaster {
            width: w,
            rows: 10,
            pitch,
            buffer: vec![fill; (pitch * 10) as usize],
            bearing_x: 64,
            bearing_y: 10 * 64,
            advance: a << 16,
            done_error: false,
        })
    }
}

struct MockLib {
    faces: HashMap<String, MockFace>,
}

impl FontLibrary for MockLib {
    fn file_exists(&self, path: &str) -> bool {
        self.faces.contains_key(path)
    }
    fn open(&self, path: &str) -> Option<Box<dyn FontFace>> {
        self.faces.get(path).map(|f| Box::new(f.clone()) as Box<dyn FontFace>)
    }
    fn read_head(&self, _: &str, n: usize) -> Vec<u8> {
        // An sfnt with one table directory record, `loca` (source type 1).
        let mut v = b"\x00\x01\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00loca".to_vec();
        v.resize(28, 0);
        v.truncate(n);
        v
    }
    fn screen_format(&self) -> ScreenFormat {
        ScreenFormat::Screen(9)
    }
    fn system_fonts_dir(&self) -> String {
        "C:\\Windows\\Fonts\\".into()
    }
    fn list_files(&self, _: &str, pat: &str) -> Vec<String> {
        if pat == "*.tt?" {
            let mut v: Vec<String> = self.faces.keys().cloned().collect();
            v.sort();
            v
        } else {
            Vec::new()
        }
    }
}

fn face(family: &str, style: u32) -> MockFace {
    let mut adv = HashMap::new();
    for c in 0x20u32..0x7F {
        adv.insert(c, 7);
    }
    adv.insert(' ' as u32, 3);
    adv.insert('.' as u32, 3);
    adv.insert('a' as u32, 8);
    MockFace {
        family: family.into(),
        style,
        fixed: false,
        adv,
        metrics: SizeMetrics {
            a: 0x000A_0000,
            b: 0x0003_0000,
            c: 0xFFFE_0000u32 as i32,
        },
    }
}

fn lib_with(fonts: &[(&str, &str, u32)]) -> Arc<MockLib> {
    Arc::new(MockLib {
        faces: fonts.iter().map(|(p, f, s)| (p.to_string(), face(f, *s))).collect(),
    })
}

fn font() -> GzFont {
    let lib = lib_with(&[("a.ttf", "Arial", 0)]);
    let mut f = GzFont::new(lib, Arc::new(Latin1Converter));
    f.set_file_path("a.ttf");
    f.set_size(12);
    assert!(f.init());
    f
}

#[test]
fn size_metrics_worked_example() {
    let mut f = font();
    f.set_spacing_adjustment(0, 2);
    assert_eq!((f.char_height, f.ascent, f.descent_raw), (15, 13, 10));
    assert_eq!(f.get_line_height(), 17);
    assert_eq!(f.size_above_baseline(), 15);
    assert_eq!(f.size_below_baseline(), 2);
    assert_eq!(f.face_kind, 1);
    // SetSize fails for 0 and while a cache holds glyphs.
    assert!(!f.set_size(0));
    f.get_char_data('x' as u32);
    assert!(!f.set_size(20));
}

#[test]
fn setter_clamps() {
    let mut f = font();
    f.set_aa_gamma(20.0);
    assert_eq!(f.aa_gamma, 10.0);
    assert_eq!(f.aa_table.len(), 256);
    f.set_aa_sharpness(-5.0);
    assert_eq!(f.aa_sharpness, -1.0);
    f.set_scaling(0.0, 5000.0);
    assert_eq!(f.get_scaling(), (0.01, 1000.0));
    f.set_advance_scaling(100.0);
    assert_eq!(f.get_advance_scaling(), (10.0, 1.0));
    f.set_antialiasing(2);
    f.set_antialiasing(0);
    assert_eq!(f.aa_mode, 2);
    f.set_style(0xFF, true);
    assert_eq!(f.style_flags & 0xFF, 0x3C);
    assert!(f.get_style(0x0C));
    assert!(GzFont::answers_interface(1) && GzFont::answers_interface(0x10673960));
    assert!(!GzFont::answers_interface(0x20673960));
}

#[test]
fn aa_table_rules() {
    assert!(aa::build_table(1.0, 0.0).is_empty());
    assert!((aa::sharpness_exponent(0.0) - 1.0).abs() < 1e-6);
    assert!((aa::sharpness_exponent(1.0) - 10.0).abs() < 1e-4);
    assert_eq!(aa::sharpness_exponent(-1.0), 0.0);
    let t = aa::build_table(2.0, 0.0);
    assert_eq!((t[0], t[255]), (0, 255));
    assert_eq!(t[128], ((128.0f32 / 255.0).powf(2.0) as f64 * 255.0) as u8);
}

#[test]
fn thai_worked_examples() {
    use thai::make_substituted_string as m;
    assert_eq!(m(&[0x0E1B, 0x0E48]).0, vec![0x0E1B, 0xF705]);
    assert_eq!(m(&[0x0E01, 0x0E48]).0, vec![0x0E01, 0xF70A]);
    assert_eq!(m(&[0x0E1B, 0x0E34, 0x0E48]).0, vec![0x0E1B, 0xF701, 0xF713]);
    assert_eq!(m(&[0x0E1B, 0x0E38, 0x0E48]).0, vec![0x0E1B, 0x0E38, 0xF705]);
    let (s, map) = m(&[0x0E01, 0x0E33]);
    assert_eq!(s, vec![0x0E01, 0x0E4D, 0x0E32]);
    assert_eq!(map, vec![0, 1, 1]);
    assert_eq!(m(&[0x0E0D, 0x0E38]).0, vec![0xF70F, 0x0E38]);
    assert_eq!(thai::thai_char_type(0x0E5B), 17);
    assert_eq!(thai::thai_char_type(0x0E7F), 0);
}

#[test]
fn encodings_and_probes() {
    let c = Latin1Converter;
    assert_eq!(encoding::convert_char_to_unicode(&c, 0x10, &[0x01, 0x0E]), (0x0E01, 2));
    assert_eq!(encoding::convert_char_to_unicode(&c, 0, &[0xE9]), (0xE9, 1));
    assert_eq!(encoding::convert_char_to_unicode(&c, 0, &[]), (0, 0));
    assert_eq!(encoding::code_page_probes(874), &[0x0E01, 0x0E2C]);
    assert!(encoding::code_page_probes(437).is_empty());
    let f = font();
    assert!(f.is_code_page_available(1252));
    assert!(!f.is_code_page_available(1251));
    assert!(f.is_code_page_available(8));
}

#[test]
fn glyph_cache_and_widths() {
    let mut f = font();
    f.set_antialiasing(3); // grey (the constructor default is monochrome, FN11)
    assert_eq!(f.char_width('H' as u32), 7);
    assert_eq!(f.get_x_offset('H' as u32), 1);
    f.set_spacing_adjustment(1, 0);
    assert_eq!(f.get_advance_width('H' as u32), 8);
    let g = f.get_char_data('H' as u32).unwrap();
    assert_eq!(g.y_offset, 13 - 10);
    assert_eq!(f.grey_cache.glyph_count(), 1);
    // ReallocateSourceData sized cellWidth*32 + x.
    assert_eq!(f.grey_cache.pitch, 7 * 32);
    assert_eq!(f.grey_cache.block.len(), ((10 + 4) * 7 * 32) as usize);
    // Zero-width characters: no glyph in the face -> empty box, advance 0.
    let z = f.get_char_data(0x200B).unwrap();
    assert_eq!(z.advance, 0);
    // Monochrome: MSB-first unpacking into 0x00/0xFF.
    f.set_antialiasing(1);
    let m = f.get_char_data('H' as u32).unwrap();
    let o = f.mono_cache.offset(m.src_left as u32, 0);
    assert_eq!(&f.mono_cache.block[o..o + 4], &[0xFF, 0x00, 0xFF, 0x00]);
}

#[test]
fn ellipsis_worked_example() {
    let mut f = font();
    let r = Rect {
        left: 0,
        top: 0,
        right: 50,
        bottom: 10,
    };
    let mut t = b"Hello world".to_vec();
    assert_eq!(f.chars_to_fit(&t, 50).0, 7);
    assert_eq!(f.format_for_ellipsis(&mut t, b"...", r, false), 11);
    assert_eq!(t, b"Hello...");
    let mut t = b"Hello world".to_vec();
    f.format_for_ellipsis(&mut t, b"...", Rect { right: 8, ..r }, false);
    assert!(t.is_empty());
    let mut t = b"Hello world".to_vec();
    f.format_for_ellipsis(&mut t, b"...", Rect { right: 8, ..r }, true);
    assert_eq!(t, b"Hello world");
}

#[test]
fn text_area_rules() {
    let mut f = font(); // line height 15, 'x' 7, ' ' 3
    let mut r = Rect {
        left: 10,
        top: 20,
        right: 110,
        bottom: 70,
    };
    f.calculate_text_area(Some(b"xx"), &mut r, false, false);
    assert_eq!((r.right, r.bottom), (24, 35));
    let mut r = Rect {
        left: 10,
        top: 20,
        right: 110,
        bottom: 70,
    };
    f.calculate_text_area(None, &mut r, true, true);
    assert_eq!((r.right, r.bottom), (10, 35));
    // 30 words of "xxx " (24 px each); at width 100 four words per line.
    let text = b"xxx ".repeat(12);
    assert_eq!(f.number_of_lines(&text, 100), 3);
    let mut r = Rect {
        left: 10,
        top: 20,
        right: 110,
        bottom: 70,
    };
    f.calculate_text_area(Some(&text), &mut r, true, true);
    assert_eq!(r.bottom, 20 + 45);
    // Width of lines: narrowest multiple of 10 that gives <= 3 lines.
    let w = f.width_of_lines(&text, 45);
    assert_eq!(f.number_of_lines(&text, w as i32), 3);
    assert!(f.number_of_lines(&text, w as i32 - 10) > 3);
    assert_eq!(f.width_of_lines(&text, 10), 0);
    // Box: aw 8 -> minW 96, maxW 256.
    let mut r = Rect::default();
    assert!(f.calculate_text_area_box(b"xxxxxxxx", &mut r, -1, -1, 100));
    assert_eq!((r.right, r.bottom), (56, 15));
    let mut r = Rect::default();
    assert!(!f.calculate_text_area_box(b"xxx", &mut r, 1, 2, 10));
    assert_eq!((r.right, r.bottom), (0, 0));
}

#[test]
fn words_to_fit_modes() {
    let mut f = font();
    assert_eq!(f.words_to_fit(b"ab cd\nef", 1000, 2).0, 6);
    assert_eq!(f.words_to_fit(b"abcdef gh", 20, 1).0, 9); // mode 1 takes the rest of the line (§8.1)
    assert_eq!(f.words_to_fit(b"abcdef gh", 20, 2).0, 2);
    let (w, len) = f.get_string_visible_width(b"ab  ");
    assert_eq!((w, len), (15, 2));
}

#[test]
fn palette_rule() {
    // §9 FN12 vectors (entries 0, 8, 16, 24, 72, 112, 120), proved under unicorn.
    let cases: [(u32, u32, u32, [u32; 7]); 6] = [
        (
            7,
            0xFFFF,
            0xF000,
            [0xF000, 0xF000, 0xF000, 0xF111, 0xF777, 0xFCCC, 0xFFFF],
        ),
        (
            7,
            0xF000,
            0xFFFF,
            [0xFFFF, 0xFFFF, 0xFFFF, 0xFEEE, 0xF888, 0xF333, 0xF000],
        ),
        (7, 0xF84C, 0, [0, 0xF000, 0xF000, 0xF101, 0xF426, 0xF63A, 0xF84C]),
        (
            9,
            0xFFFF_FFFF,
            0xFF00_0000,
            [
                0xFF00_0000,
                0xFF00_0000,
                0xFF00_0F0F,
                0xFF00_1F1F,
                0xFF00_7F7F,
                0xFF00_CFCF,
                0xFFFF_FFFF,
            ],
        ),
        (
            9,
            0x00FF_0000,
            0,
            [
                0,
                0xFF00_0000,
                0xFF00_0000,
                0xFF00_0000,
                0xFF00_0000,
                0xFF00_0000,
                0x00FF_0000,
            ],
        ),
        (
            9,
            0xFF12_3456,
            0xFF00_0000,
            [
                0xFF00_0000,
                0xFF00_0000,
                0xFF00_0305,
                0xFF00_060A,
                0xFF00_1A2B,
                0xFF00_2A45,
                0xFF12_3456,
            ],
        ),
    ];
    for (t, fg, bg, want) in cases {
        let p = Palette::build(fg, bg, ScreenFormat::Screen(t));
        assert_eq!(p.entries.len(), 128);
        let got: Vec<u32> = [0, 8, 16, 24, 72, 112, 120].iter().map(|&i| p.entries[i]).collect();
        assert_eq!(got, want, "type {t} fg {fg:08X} bg {bg:08X}");
        assert!(p.entries[120..].iter().all(|&e| e == fg));
    }
    // Another colour type composes 0; no screen keeps the caller-named stale output.
    let p = Palette::build(0xFFFF, 0, ScreenFormat::Screen(0));
    assert!(p.entries[8..120].iter().all(|&e| e == 0));
    let p = Palette::build(0xFFFF, 0x1234, ScreenFormat::NoScreen { stale_out: 0xDEAD_BEEF });
    assert_eq!(p.entries[0], 0x1234);
    assert!(p.entries[8..120].iter().all(|&e| e == 0xDEAD_BEEF));
    assert_eq!(ScreenFormat::from_depth(16), ScreenFormat::Screen(7));
    assert_eq!(ScreenFormat::from_depth(32), ScreenFormat::Screen(9));
    // Split/compose rules of the §9 table for 1555 and 565.
    assert_eq!(
        super::font::split_colour(ScreenFormat::Screen(5), 0xFFFF),
        Some([255, 255, 255])
    );
    assert_eq!(
        super::font::compose_colour(ScreenFormat::Screen(6), 0x17, 0x88, 0xF9),
        0x8000 | (0x10 << 7) | (0x88 << 2) | (0xF9 >> 3)
    );
    assert_eq!(
        super::font::compose_colour(ScreenFormat::Screen(5), 0x107, 0x2FF, 0x380),
        (0xFC << 3) | 0x10
    );
    let mut f = font();
    for i in 0..5 {
        f.set_current_palette(i, 0);
    }
    assert_eq!(f.palettes.len(), 4);
    assert_eq!(f.palettes[0].fg, 4);
}

fn sys(fonts: &[(&str, &str, u32)]) -> GzFontSys {
    let mut s = GzFontSys::new(lib_with(fonts), Arc::new(Latin1Converter), 0);
    s.init();
    s
}

#[test]
fn catalogue_scoring_worked_example() {
    let mut s = sys(&[
        ("arial.ttf", "Arial", 0),
        ("tahoma.ttf", "Tahoma", 1),
        ("zz.ttf", "Other", 1),
    ]);
    assert_eq!(s.add_all_fonts_from_directory(""), 3);
    assert_eq!(s.font_names.get(&9).map(String::as_str), Some("Arial"));
    let f = s.font_acquire_family("Arial,Tahoma", 0, 12, 0x05, false).unwrap();
    assert_eq!(f.lock().file_path, "tahoma.ttf");
    // n = 0: kind scoring, ties keep the earlier path.
    let f = s.font_acquire_family("", 1, 12, 0, false).unwrap();
    assert_eq!(f.lock().file_path, "arial.ttf");
    assert!(GzFontSys::new(lib_with(&[]), Arc::new(Latin1Converter), 0)
        .font_acquire_family("x", 0, 1, 0, false)
        .is_none());
}

#[test]
fn cache_and_release() {
    let mut s = sys(&[("arial.ttf", "Arial", 0)]);
    s.add_font_path("arial.ttf");
    assert!(!s.add_font_path("arial.ttf"));
    let a = s.font_acquire_path("arial.ttf", 12, 0, true).unwrap();
    assert_eq!(a.ref_count(), 2);
    let b = s.font_acquire_family("arial", 0, 12, 0, true).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    assert_eq!(a.ref_count(), 3);
    let mut out = vec![None; 4];
    assert_eq!(s.get_cached_fonts(&mut []), 1);
    assert_eq!(s.get_cached_fonts(&mut out), 1);
    assert_eq!(s.font_release(&a), 2);
    assert_eq!(s.font_release(&a), 0);
    assert_eq!(s.get_cached_fonts(&mut []), 0);
    let mut recs = [FontInfoRecord::default(); 2];
    assert_eq!(s.get_font_info_list(&mut recs), 1);
    assert_eq!(&recs[0].family[..6], b"Arial\0");
    assert_eq!(recs[0].to_bytes().len(), 0x88);
    // AddFont(font): same size refused.
    let c = s.font_acquire_path("arial.ttf", 14, 0, false).unwrap();
    assert!(s.add_font(Some(&c)));
    assert!(!s.add_font(Some(&c)));
    assert!(!s.add_font(None));
    assert!(!s.remove_fonts_by_family("ARIAL"));
    assert!(s.catalogue.is_empty());
    assert!(s.families.contains("arial"));
    assert_eq!(c.ref_count(), 1);
    let solo = FontObj::new(GzFont::new(lib_with(&[]), Arc::new(Latin1Converter)));
    assert!(solo.query_interface(1));
    assert_eq!(solo.release(), 1);
    assert_eq!(solo.release(), 0);
}

#[test]
fn monochrome_ranges() {
    let mut s = sys(&[("arial.ttf", "Arial", 0)]);
    s.set_default_antialiasing(3);
    s.set_monochrome_size_range(8, 10, None);
    assert_eq!(s.font_acquire_path("arial.ttf", 9, 0, false).unwrap().lock().aa_mode, 1);
    assert_eq!(
        s.font_acquire_path("arial.ttf", 12, 0, false).unwrap().lock().aa_mode,
        3
    );
    // Per-family ranges are looked up by the pre-Init (empty) name.
    s.set_monochrome_size_range(11, 13, Some("Arial"));
    assert_eq!(
        s.font_acquire_path("arial.ttf", 12, 0, false).unwrap().lock().aa_mode,
        3
    );
    s.set_monochrome_size_range(11, 13, Some(""));
    s.set_monochrome_size_range(1, 2, Some(""));
    assert_eq!(s.family_ranges.get(""), Some(&(11, 13)));
    assert_eq!(
        s.font_acquire_path("arial.ttf", 12, 0, false).unwrap().lock().aa_mode,
        1
    );
    s.set_monochrome_size_range(5, 5, Some(""));
    assert!(!s.family_ranges.contains_key(""));
}

#[test]
fn substitutions_worked_example() {
    let mut s = sys(&[("verdana.ttf", "Verdana", 0)]);
    s.add_font_path("verdana.ttf");
    s.add_substitution("Arial", "Tahoma");
    s.add_substitution("Arial", "Verdana");
    let mut out = vec![Some(String::new()), Some(String::new())];
    assert_eq!(s.get_substitutions("Arial", &mut out), 2);
    assert_eq!(out, vec![Some("Tahoma".into()), Some("Verdana".into())]);
    assert_eq!(s.get_substitutions("arial", &mut out), 0);
    let mut o = String::new();
    assert!(s.get_available_substitution("Arial", &mut o));
    assert_eq!(o, "Verdana");
    assert!(s.is_family_available("VERDANA", 7, 4));
    assert!(!s.is_family_available("VERDANA", 7, 1));
    assert_eq!(s.query_interface(0x287FB697), Some(true));
    assert_eq!(s.query_interface(0x00673960), Some(false));
    assert_eq!(s.query_interface(5), None);
    assert!(s.shutdown());
    assert!(s.catalogue.is_empty() && s.font_names.is_empty());
    assert_eq!(director::class_for(0x1AA404AC), Some(director::FontClass::Font));
}

// ---- FN7-FN14 answers ----------------------------------------------------------------------

struct HeadLib(Vec<u8>);

impl FontLibrary for HeadLib {
    fn file_exists(&self, _: &str) -> bool {
        true
    }
    fn open(&self, _: &str) -> Option<Box<dyn FontFace>> {
        None
    }
    fn read_head(&self, _: &str, n: usize) -> Vec<u8> {
        self.0[..n.min(self.0.len())].to_vec()
    }
    fn system_fonts_dir(&self) -> String {
        String::new()
    }
    fn list_files(&self, _: &str, _: &str) -> Vec<String> {
        Vec::new()
    }
    fn screen_format(&self) -> ScreenFormat {
        ScreenFormat::NoScreen { stale_out: 0 }
    }
}

fn sfnt(version: &[u8; 4], tags: &[&[u8; 4]]) -> Vec<u8> {
    let mut v = version.to_vec();
    v.extend_from_slice(&(tags.len() as u16).to_be_bytes());
    v.extend_from_slice(&[0; 6]);
    for t in tags {
        v.extend_from_slice(*t);
        v.extend_from_slice(&[0; 12]);
    }
    v
}

#[test]
fn fn10_content_probe() {
    use super::font::probe_source_type as probe;
    let p = |b: Vec<u8>| probe(&HeadLib(b), "x");
    assert_eq!(p(sfnt(&[0, 1, 0, 0], &[b"cmap", b"DSIG", b"loca"])), 4);
    assert_eq!(p(sfnt(&[0, 2, 0, 0], &[b"glyf", b"loca"])), 1);
    assert_eq!(p(sfnt(b"OTTO", &[b"sloc"])), 1);
    assert_eq!(p(sfnt(b"OTTO", &[b"CFF "])), 0);
    assert_eq!(p(b"%!PS-AdobeFont".to_vec()), 2);
    assert_eq!(p(b"ttcf\0\0\0\0\0\0\0\0".to_vec()), 0);
    // Init fails when the probe gives 0; the mock's sfnt with `loca` opens as type 1, flags 0x8A.
    let f = font();
    assert_eq!((f.source_type, f.load_flags), (1, 0x008A));
}

#[test]
fn fn11_constructor_defaults() {
    let f = GzFont::new(lib_with(&[]), Arc::new(Latin1Converter));
    assert_eq!((f.point_size, f.char_height, f.aa_mode, f.field_50), (16, 16, 1, 2));
    assert_eq!(
        (f.aa_gamma, f.aa_sharpness, f.scale_x, f.scale_y, f.advance_scale),
        (1.0, 0.0, 1.0, 1.0, 1.0)
    );
    assert_eq!(
        (f.default_char, f.default_glyph, f.code_page, f.fg, f.bg),
        (0x20, 0, 0, 0, 0)
    );
    assert_eq!((f.source_type, f.load_flags), (0, 0x000A));
}

#[test]
fn fn7_advance_and_missing_glyph() {
    let adv = 0x0005_8000;
    assert_eq!(GzFont::scaled_advance(adv, 0.99, 0x20), 4);
    assert_eq!(GzFont::scaled_advance(adv, 0.99, 'a' as u32), 5);
    assert_eq!(GzFont::scaled_advance(adv, 1.0, 0x20), 5);
    assert_eq!(GzFont::scaled_advance(adv, 0.98, 0x20), 5); // s ≤ 0.98: no shortening
    assert_eq!(GzFont::scaled_advance(adv, 2.0, 0x20), 11);
    assert_eq!(GzFont::scaled_advance(-adv, 1.0, 'a' as u32), -5);
    // charHeight 15 -> a 7x7 hollow square at the line top, advance 7.
    let mut f = font();
    f.default_glyph = 0;
    let g = f.get_char_data(0x4E00).unwrap();
    assert_eq!(
        (g.width(), g.src_bottom - g.src_top, g.advance, g.x_offset, g.y_offset),
        (7, 7, 7, 0, 0)
    );
    let row = |f: &GzFont, y: u32| {
        let o = f.mono_cache.offset(g.src_left as u32, y);
        f.mono_cache.block[o..o + 7].to_vec()
    };
    assert_eq!(row(&f, 0), vec![0xFF; 7]);
    assert_eq!(row(&f, 3), vec![0xFF, 0, 0, 0, 0, 0, 0xFF]);
    assert_eq!(row(&f, 6), vec![0xFF; 7]);
    let z = f.get_char_data(0xFEFF).unwrap();
    assert_eq!((z.width(), z.advance), (0, 0));
    f.advance_scale = 1.5;
    assert_eq!(f.get_char_data(0x4E01).unwrap().advance, 10);
}

#[test]
fn fn8_line_thickness() {
    use super::font::line_thickness;
    assert_eq!(line_thickness(12), (1, 1));
    assert_eq!(line_thickness(16), (2, 1));
    assert_eq!(line_thickness(30), (3, 2));
    assert_eq!(line_thickness(35), (3, 2));
    assert_eq!(line_thickness(36), (4, 2));
    assert_eq!(line_thickness(72), (4, 4));
    // pointSize 12, ascent 13, yOffset 3: underline row 13 - 3 + 1 = 11 (1 row); srcBottom 12.
    let mut f = font();
    f.set_style(0x04, true);
    let g = f.get_char_data('H' as u32).unwrap();
    assert_eq!(g.src_bottom, 12);
    let o = f.mono_cache.offset(g.src_left as u32, 11);
    assert_eq!(f.mono_cache.block[o], 0xFF);
}

#[test]
fn fn9_name_map() {
    let mut f = font();
    f.source_type = 2;
    f.default_glyph = 0;
    // The mock face resolves "uni4E00" to 'A' (0x41).
    let g = f.get_char_data(0x4E00).unwrap();
    assert_eq!(g.advance, 7);
    assert_eq!(f.name_map.get(&0x4E00), Some(&0x41));
    // No name: nothing cached, the box is used.
    f.get_char_data(0x4E02);
    assert!(!f.name_map.contains_key(&0x4E02));
}

#[test]
fn fn14_parse_and_new_entry() {
    use super::fontsys::parse_delimited as p;
    assert_eq!(p("arial,,tahoma", 32), ["arial", "", "tahoma"]);
    assert_eq!(p(",arial", 32), ["", "arial"]);
    assert_eq!(p("arial,", 32), ["arial"]);
    assert_eq!(p("arial , tahoma ", 32), ["arial", "tahoma "]);
    assert_eq!(p("  \"a,b\",c", 32), ["a,b", "c"]);
    assert!(p("", 32).is_empty() && p(" \t ", 32).is_empty());
    assert_eq!(p("a,b,c", 2), ["a", "b"]);
    let mut s = GzFontSys::new(lib_with(&[("a.ttf", "Arial", 1)]), Arc::new(Latin1Converter), 0);
    let f = s.font_acquire_path("a.ttf", 12, 0x04, true).unwrap();
    let e = &s.catalogue["a.ttf"];
    assert_eq!(
        (e.family.as_str(), e.style, e.kind, e.source_type),
        ("Arial", 0x05, 1, 0)
    );
    assert!(s.families.contains("arial"));
    assert_eq!(f.ref_count(), 2);
    let again = s.font_acquire_family("Arial", 0, 12, 0x04, true).unwrap();
    assert!(Arc::ptr_eq(&f, &again));
}

#[test]
fn fn13_words_and_lines() {
    use super::words::*;
    let u = |s: &str| s.encode_utf16().collect::<Vec<u16>>();
    assert_eq!(ns_count(), 223);
    assert_eq!(get_word(&u("foo  bar")), 5);
    assert_eq!(get_word(&u("  bar")), 2);
    assert_eq!(get_word(&u("well-known")), 5);
    assert_eq!(get_word(&u("a\tb")), 2);
    assert_eq!(get_word(&u("ab\r\n")), 2);
    assert_eq!(get_word(&u("\r\n")), 1);
    assert_eq!(get_word(&u("漢字。")), 1);
    assert_eq!(get_word(&u("字。")), 2);
    assert_eq!(get_word(&[]), 0);
    assert_eq!(thai_cluster_len(&[0x0E01, 0x0E33], 2), 2);
    assert_eq!(thai_cluster_len(&[0x0E01, 0x0E34, 0x0E48, 0x0E33], 3), 4);
    assert_eq!(thai_cluster_len(&[0x0E01, 0x0E34, 0x0E01], 3), 2);
    let mut f = font();
    assert_eq!(f.number_of_lines(b"ab\r\ncd", 1000), 3);
    assert_eq!(f.words_to_fit(b"\ncd", 1000, 2).0, 1);
    // The visible width keeps tab but drops trailing U+0020.
    assert_eq!(f.get_string_visible_width(b"ab \t").1, 4);
}

// ---- §4.1 / §12.1 slot audit: slots no earlier test reached --------------------------------

#[test]
fn slot_getters_round_trip() {
    let mut f = font();
    assert_eq!(f.get_name(), "Arial");
    assert_eq!(f.get_file_path(), "a.ttf");
    assert_eq!(f.get_face_kind(), 1);
    assert_eq!(f.get_size(), 12);
    f.set_style(0xFF, true);
    assert_eq!(f.get_styles(), 0x3C);
    f.set_font_color(0x1234, 0x5678);
    assert_eq!((f.get_font_color(), f.get_font_background_color()), (0x1234, 0x5678));
    f.set_antialiasing(0);
    assert_eq!(f.get_antialiasing(), 1); // Set ignores 0 (FN11 default 1)
    f.set_aa_gamma(20.0);
    f.set_aa_sharpness(-3.0);
    assert_eq!((f.get_aa_gamma(), f.get_aa_sharpness()), (10.0, -1.0));
    f.set_code_page(1252);
    assert_eq!(f.get_code_page(), (true, 1252));
    f.set_spacing_adjustment(2, -1);
    assert_eq!(f.get_spacing_adjustment(), (2, -1));
    assert_eq!(f.char_height_args(7, 9), f.char_height());
    assert!(GzFont::is_char_encoding_available(2));
    assert!(!GzFont::is_char_encoding_available(1));
    // Slot 47: (0, 0, GetStringWidth, CharHeight).
    let w = f.get_string_width(b"aa");
    assert_eq!(w, 2 * (8 + 2)); // advance 8 plus charSpacing 2, per char
    let r = f.text_area(b"aa");
    assert_eq!((r.left, r.top, r.right, r.bottom), (0, 0, w, f.char_height()));
}

#[test]
fn char_available_and_preload() {
    let mut f = font();
    assert!(f.is_char_available(b""));
    assert!(f.is_char_available(b"ab"));
    assert!(!f.is_char_available(b"a\x01"));
    assert_eq!(f.code_page, 0);
    assert!(f.preload_chars(b"abc"));
    assert_eq!(f.appropriate_cache().glyph_count(), 3);
    let mut u = GzFont::new(lib_with(&[]), Arc::new(Latin1Converter));
    assert!(!u.preload_chars(b"abc"));
    assert_eq!(u.appropriate_cache().glyph_count(), 0);
}

#[derive(Default)]
struct LogRenderer(std::sync::Mutex<Vec<String>>);

impl FontRenderer for LogRenderer {
    fn attach(&self, _: usize) {
        self.0.lock().unwrap().push("attach".into());
    }
    fn detach(&self, _: usize) {
        self.0.lock().unwrap().push("detach".into());
    }
    fn lock_surface(&self, _: SurfaceId, _: FontUse) -> bool {
        self.0.lock().unwrap().push("lock".into());
        true
    }
    fn unlock_surface(&self, _: SurfaceId, _: FontUse) -> bool {
        self.0.lock().unwrap().push("unlock".into());
        true
    }
    fn draw(&self, _: SurfaceId, recs: &[DrawRecord], _: &[u8], c: &DrawColour) {
        self.0.lock().unwrap().push(format!("draw:{}:{:?}", recs.len(), c));
    }
}

#[test]
fn renderer_slots_and_shutdown() {
    let mut f = font();
    let r = Arc::new(LogRenderer::default());
    let dynr: Arc<dyn FontRenderer> = r.clone();
    f.set_renderer(Some(dynr.clone()));
    f.set_renderer(Some(dynr.clone())); // same: nothing
    assert!(f.get_renderer().is_some_and(|g| Arc::ptr_eq(g, &dynr)));
    // Only the 0→1 and 1→0 edges reach the renderer.
    assert!(f.lock_surface(1));
    assert!(f.lock_surface(1));
    f.unlock_surface(1);
    f.unlock_surface(1);
    // Shutdown: SetRenderer(null), caches cleared, face closed.
    f.preload_chars(b"a");
    assert!(f.shutdown());
    assert!(f.get_renderer().is_none() && !f.initialised);
    assert_eq!(f.appropriate_cache().glyph_count(), 0);
    assert_eq!(*r.0.lock().unwrap(), ["attach", "lock", "unlock", "detach"]);
    // FinalRelease shuts the font down.
    let o = FontObj::new(font());
    assert_eq!(o.add_ref(), 2);
    assert_eq!(o.release(), 1);
    assert_eq!(o.release(), 0);
    assert!(!o.lock().initialised);
}

#[test]
fn fontsys_remaining_slots() {
    let mut s = sys(&[("arial.ttf", "Arial", 0)]);
    assert_eq!(s.query_interface(super::fontsys::IID_GZ_FONT_SYS2), Some(true));
    assert_eq!(s.query_interface(super::fontsys::IID_GZ_FONT_SYS), Some(false));
    assert_eq!(s.query_interface(0x1234), None);
    assert!(!s.slot16(7));
    s.add_font_path("arial.ttf");
    // Slot 9: slot 11 with an empty list, then SetFontColor(colour, 0x08000000).
    let f = s.font_acquire_by_kind(1, 12, 0, 0xABCD, true).unwrap();
    assert_eq!(f.lock().get_font_color(), 0xABCD);
    assert_eq!(f.lock().get_font_background_color(), PEN_DEFAULT_COLOUR);
    assert_eq!(f.ref_count(), 2);
    // Release to zero runs Shutdown: the cache lets go of its reference, state is cleared.
    assert_eq!(s.ref_count, 2); // the two answered QueryInterface calls
    assert_eq!(s.release(), 1);
    assert_eq!(s.release(), 0);
    assert_eq!(f.ref_count(), 1);
    assert!(!s.initialised && s.catalogue.is_empty() && s.families.is_empty());
}

#[test]
fn draw_text_line_slot51_and_paragraph_slot52() {
    let mut f = font();
    let r = Arc::new(LogRenderer::default());
    f.set_renderer(Some(r.clone()));
    // No target: nothing reaches the renderer, the pen still advances; returns initialised.
    let mut pen = (10, 5);
    assert!(f.draw_text_line(None, 0, 0, b"aa", 0x123, 0, None, Some(&mut pen)));
    assert_eq!(pen, (10 + 2 * 8, 5));
    // With a target: lock 0→1, one draw with the fg colour (aaMode 1), unlock.
    assert!(f.draw_text_line(Some(1), 0, 0, b"aa", 0x123, 0, None, None));
    // Clip rect left of the text: no record overlaps, so no draw call.
    assert!(f.draw_text_line(Some(1), 100, 0, b"aa", 0x123, 0, Some((0, 0, 50, 50)), None));
    let rect = Rect {
        left: 0,
        top: 0,
        right: 1000,
        bottom: 100,
    };
    assert!(f.draw_text_paragraph(Some(1), rect, b"aa", 0, 0x123, 0, None));
    assert_eq!(
        *r.0.lock().unwrap(),
        [
            "attach",
            "lock",
            "draw:2:Colour(291)",
            "unlock",
            "lock",
            "unlock",
            "lock",
            "draw:2:Colour(291)",
            "unlock"
        ]
    );
}
