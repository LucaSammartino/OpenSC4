// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent normative-map and arithmetic-boundary checks; foreign services are test doubles.
use super::*;
use crate::fonts::advance::{self, Precision, Rounding, X87Control};
use std::sync::Mutex;

fn normative_names() -> Vec<(u16, &'static str)> {
    include_str!("../../../../docs/specs/data/font-name-map-fn15.tsv")
        .lines()
        .skip(1)
        .map(|line| {
            let (code, name) = line.split_once('\t').unwrap();
            (u16::from_str_radix(code, 16).unwrap(), name)
        })
        .collect()
}

#[test]
fn fn15_all_477_names_and_every_absent_code() {
    let expected: HashMap<_, _> = normative_names().into_iter().collect();
    assert_eq!(expected.len(), 477);
    for code in 0..=u16::MAX {
        assert_eq!(
            crate::fonts::names::glyph_name(code),
            expected.get(&code).copied(),
            "U+{code:04X}"
        );
    }
    for (code, name) in [
        (0xB7, "periodcentered"),
        (0x3A5, "Upsilon"),
        (0x2126, "Omega"),
        (0xFB01, "fi"),
        (0xFB02, "fl"),
    ] {
        assert_eq!(crate::fonts::names::glyph_name(code), Some(name));
    }
}

struct NameFace {
    accepted: HashMap<String, u32>,
    calls: Arc<Mutex<Vec<String>>>,
}
impl FontFace for NameFace {
    fn family_name(&self) -> String {
        "Names".into()
    }
    fn style_flags(&self) -> u32 {
        0
    }
    fn is_fixed_size(&self) -> bool {
        false
    }
    fn fixed_advance(&self) -> i32 {
        0
    }
    fn char_index(&self, code: u32) -> Option<u32> {
        Some(if code == 0xFFFF { 0 } else { code })
    }
    fn glyph_name_to_code(&self, name: &str) -> Option<u32> {
        self.calls.lock().unwrap().push(name.into());
        self.accepted.get(name).copied()
    }
    fn set_char_size(&mut self, _: u32, _: u32, _: u32, _: f32, _: f32) -> Option<SizeMetrics> {
        None
    }
    fn load_glyph(&mut self, _: u32, _: GreyLevels, _: u16) -> Option<GlyphRaster> {
        None
    }
}

#[test]
fn fn15_full_lookup_success_cache_failure_retry_and_discarded_aliases() {
    let rows = normative_names();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let accepted: HashMap<String, u32> = rows
        .iter()
        .enumerate()
        .map(|(i, (_, name))| (name.to_string(), i as u32 + 1))
        .collect();
    let mut f = font();
    f.face = Some(Box::new(NameFace {
        accepted: accepted.clone(),
        calls: calls.clone(),
    }));
    for &(code, name) in &rows {
        // Several distinct codes intentionally share a PostScript name and hence glyph index.
        assert_eq!(f.name_map_index(code as u32), accepted[name]);
        assert_eq!(f.name_map_index(code as u32), accepted[name]);
    }
    assert_eq!(
        *calls.lock().unwrap(),
        rows.iter().map(|(_, n)| n.to_string()).collect::<Vec<_>>()
    );
    f.name_map.clear();
    f.face = Some(Box::new(NameFace {
        accepted: [
            ("middot".into(), 1),
            ("Upsilon1".into(), 2),
            ("uniFFFF".into(), 0xFFFF),
            ("uni4E00".into(), 42),
        ]
        .into(),
        calls: calls.clone(),
    }));
    calls.lock().unwrap().clear();
    for _ in 0..2 {
        assert_eq!(f.name_map_index(0xB7), 0);
        assert_eq!(f.name_map_index(0x3A5), 0);
        assert_eq!(f.name_map_index(0xFFFF), 0);
    }
    assert_eq!(
        *calls.lock().unwrap(),
        ["periodcentered", "uni00B7", "Upsilon", "uni03A5", "uniFFFF"].repeat(2)
    );
    assert!(f.name_map.is_empty());
    assert_eq!(f.name_map_index(0x4E00), 42);
    assert_eq!(f.name_map_index(0x4E00), 42);
    assert_eq!(calls.lock().unwrap().last().unwrap(), "uni4E00");
}

fn control(precision: Precision, rounding: Rounding) -> X87Control {
    X87Control { precision, rounding }
}

#[test]
fn fn16_published_vectors_all_precisions_and_narrow_band() {
    for (adv, scale, ch, pc24, wide) in [
        (65535, 0x3F800080, 0x41, 1, 0),
        (-65535, 0x3F800080, 0x41, -1, 0),
        (524288, 0x3F7D70A4, 0x41, 8, 8),
        (524288, 0x3F7D70A4, 0x20, 7, 7),
        (360448, 0x3F7D70A4, 0x20, 4, 4),
    ] {
        for (pc, expected) in [
            (Precision::Pc24, pc24),
            (Precision::Pc53, wide),
            (Precision::Pc64, wide),
        ] {
            assert_eq!(
                advance::scaled_advance(adv, f32::from_bits(scale), ch, control(pc, Rounding::NearestEven)),
                Ok(expected)
            );
        }
    }
    assert_eq!(GzFont::scaled_advance(8 << 16, f32::from_bits(0x3F7AE148), 0x41), 7);
    assert_eq!(GzFont::scaled_advance(8 << 16, f32::from_bits(0x3F7AE149), 0x41), 8);
    assert_eq!(GzFont::scaled_advance(i32::MAX, 0.99999994, 0x20), 32766);
    assert_eq!(GzFont::scaled_advance(-2147418112, 0.99, 0x20), -32440);
    for ch in [0x200B, 0xFEFF] {
        assert_eq!(GzFont::scaled_advance(123 << 16, 0.5, ch), 0);
    }
}

#[test]
fn fn16_independent_float32_and_exact_float64_boundary_sweep() {
    // Integers below 2^24 load exactly even into f32; f32 multiplication is an independent
    // PC24 nearest oracle here. These products also fit f64 exactly, giving the PC53/64 oracle.
    for adv in [1, 3, 65535, 65536, 65537, 360448, 524288, 8388607, 16777215] {
        for sign in [1, -1] {
            let a = adv * sign;
            for bits in [
                0x3F000000u32,
                0x3F7AE147,
                0x3F7AE148,
                0x3F7AE149,
                0x3F800001,
                0x3F80007F,
                0x3F800080,
                0x3F800081,
                0x41200000,
            ] {
                let s = f32::from_bits(bits);
                let small = ((a as f32 * s) as f64 / 65536.0).trunc() as i64;
                let exact = (a as f64 * s as f64 / 65536.0).trunc() as i64;
                assert_eq!(
                    control(Precision::Pc24, Rounding::NearestEven).product(a, s, -16),
                    Ok(small)
                );
                for pc in [Precision::Pc53, Precision::Pc64] {
                    assert_eq!(control(pc, Rounding::NearestEven).product(a, s, -16), Ok(exact));
                }
            }
        }
    }
}

#[test]
fn fn16_rounding_directions_ties_extended_exponents_and_pc53_boundary() {
    let s = f32::from_bits(0x3F800080);
    for (r, pos, neg) in [
        (Rounding::NearestEven, 1, -1),
        (Rounding::Down, 0, -1),
        (Rounding::Up, 1, 0),
        (Rounding::TowardZero, 0, 0),
    ] {
        let c = control(Precision::Pc24, r);
        assert_eq!(c.product(65535, s, -16), Ok(pos));
        assert_eq!(c.product(-65535, s, -16), Ok(neg));
    }
    let c = X87Control::default();
    assert_eq!(c.product(16777217, 1.0, 0), Ok(16777216)); // even tie
    assert_eq!(c.product(16777219, 1.0, 0), Ok(16777220)); // odd tie
    assert_eq!(c.product(1, f32::from_bits(1), 149), Ok(1)); // extended exponent, no f32 underflow
    assert_eq!(c.product(2, f32::MAX, -128), Ok(1)); // first product exceeds f32 storage range
                                                     // (2^31-1)*(2-2^-23) = 4294967038 + 2^-23. Its significand has 55 bits:
                                                     // PC53 nearest drops the tiny residual, whereas PC64 retains it.
    let s = f32::from_bits(0x3FFFFFFF);
    assert_eq!(
        control(Precision::Pc53, Rounding::Up).product(i32::MAX, s, 0),
        Ok(4294967038)
    );
    assert_eq!(
        control(Precision::Pc64, Rounding::Up).product(i32::MAX, s, 0),
        Ok(4294967038)
    );
    // Just below an integer: PC53 rounds the product up before truncation; PC64 retains it.
    let s = f32::from_bits(0x3F800001);
    assert_eq!(
        control(Precision::Pc53, Rounding::NearestEven).product(i32::MAX, s, 0),
        Ok(2147483903)
    );
    assert_eq!(
        control(Precision::Pc64, Rounding::NearestEven).product(i32::MAX, s, 0),
        Ok(2147483902)
    );
    assert_eq!(c.product(1, f32::NAN, 0), Err(advance::ConversionError::NonFinite));
    assert_eq!(
        c.product(i32::MAX, f32::MAX, 0),
        Err(advance::ConversionError::IntegerOverflow)
    );
}

#[derive(Default)]
struct FailingRenderer(Mutex<Vec<(bool, SurfaceId, FontUse)>>);
impl FontRenderer for FailingRenderer {
    fn lock_surface(&self, target: SurfaceId, font: FontUse) -> bool {
        self.0.lock().unwrap().push((true, target, font));
        false
    }
    fn unlock_surface(&self, target: SurfaceId, font: FontUse) -> bool {
        self.0.lock().unwrap().push((false, target, font));
        false
    }
    fn draw(&self, _: SurfaceId, _: &[DrawRecord], _: &[u8], _: &DrawColour) {
        panic!("failed auto lock must not draw");
    }
}

#[test]
fn fn17_failed_callbacks_modes_wrapping_and_null_edges() {
    let mut f = font();
    let r = Arc::new(FailingRenderer::default());
    f.set_renderer(Some(r.clone()));
    for aa in [0, 1, 2, 3, 4, u32::MAX] {
        f.aa_mode = aa;
        assert!(f.lock_surface(123));
        assert_eq!(f.surface_lock_count, 1);
        assert!(f.lock_surface(456));
        assert!(f.unlock_surface(789));
        assert!(f.unlock_surface(987));
        assert_eq!(f.surface_lock_count, 0);
        let calls = r.0.lock().unwrap();
        assert_eq!(
            calls[calls.len() - 2],
            (
                true,
                123,
                FontUse {
                    font_id: f.id,
                    mode: if (1..=3).contains(&aa) { aa } else { 1 }
                }
            )
        );
        assert_eq!(calls.last().unwrap().1, 987);
    }
    assert!(!f.draw_text_line(Some(42), 0, 0, b"a", 1, 2, None, None));
    assert_eq!(f.surface_lock_count, 0);
    f.set_renderer(None);
    assert!(f.unlock_surface(0));
    assert_eq!(f.surface_lock_count, u32::MAX);
    assert!(f.lock_surface(0));
    assert_eq!(f.surface_lock_count, 0);
    for (old, lock) in [(0, true), (1, false)] {
        f.surface_lock_count = old;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if lock {
                f.lock_surface(0)
            } else {
                f.unlock_surface(0)
            }
        }));
        assert!(result.is_err());
        assert_eq!(f.surface_lock_count, old);
    }
    f.surface_lock_count = 1;
    assert!(f.lock_surface(0));
    assert!(f.unlock_surface(0));
    assert_eq!(f.surface_lock_count, 1);
}

struct ServiceFace {
    fixed: bool,
    fail_size: bool,
    fail_load: bool,
    done_error: bool,
    drops: Arc<std::sync::atomic::AtomicU32>,
}
impl Drop for ServiceFace {
    fn drop(&mut self) {
        self.drops.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}
impl FontFace for ServiceFace {
    fn family_name(&self) -> String {
        "Service".into()
    }
    fn style_flags(&self) -> u32 {
        0
    }
    fn is_fixed_size(&self) -> bool {
        self.fixed
    }
    fn fixed_advance(&self) -> i32 {
        65535
    }
    fn char_index(&self, _: u32) -> Option<u32> {
        Some(1)
    }
    fn set_char_size(&mut self, _: u32, _: u32, _: u32, _: f32, _: f32) -> Option<SizeMetrics> {
        (!self.fail_size).then_some(SizeMetrics {
            a: 10 << 16,
            b: 3 << 16,
            c: -(2 << 16),
        })
    }
    fn load_glyph(&mut self, _: u32, _: GreyLevels, _: u16) -> Option<GlyphRaster> {
        (!self.fail_load).then_some(GlyphRaster {
            width: 1,
            rows: 1,
            pitch: 1,
            buffer: vec![0x80],
            bearing_x: -65,
            bearing_y: -65,
            advance: if self.fixed { 8 << 16 } else { 65535 },
            done_error: self.done_error,
        })
    }
}

struct ControlLib(X87Control);
impl FontLibrary for ControlLib {
    fn file_exists(&self, _: &str) -> bool {
        true
    }
    fn open(&self, _: &str) -> Option<Box<dyn FontFace>> {
        None
    }
    fn read_head(&self, _: &str, _: usize) -> Vec<u8> {
        b"%!PS".to_vec()
    }
    fn screen_format(&self) -> ScreenFormat {
        ScreenFormat::Screen(9)
    }
    fn system_fonts_dir(&self) -> String {
        String::new()
    }
    fn list_files(&self, _: &str, _: &str) -> Vec<String> {
        Vec::new()
    }
    fn x87_control(&self) -> X87Control {
        self.0
    }
}

fn service_face(
    fixed: bool,
    fail_size: bool,
    fail_load: bool,
    done_error: bool,
    drops: &Arc<std::sync::atomic::AtomicU32>,
) -> Box<ServiceFace> {
    Box::new(ServiceFace {
        fixed,
        fail_size,
        fail_load,
        done_error,
        drops: drops.clone(),
    })
}

#[test]
fn fn16_loaded_fixed_failure_done_error_and_insert_unique_paths() {
    let drops = Arc::new(std::sync::atomic::AtomicU32::new(0));
    for (pc, expected) in [(Precision::Pc24, 1), (Precision::Pc53, 0), (Precision::Pc64, 0)] {
        for fixed in [false, true] {
            let mut f = font();
            f.lib = Arc::new(ControlLib(control(pc, Rounding::NearestEven)));
            f.face = Some(service_face(fixed, false, false, false, &drops));
            f.advance_scale = f32::from_bits(0x3F800080);
            let rec = f.get_char_data(0x41).unwrap();
            assert_eq!(rec.advance, expected);
            assert_eq!((rec.x_offset, rec.y_offset), (-1, 14)); // separate signed divisions
            assert_eq!(f.get_char_data(0x41), Some(rec));
            let pen = f.appropriate_cache().pen_x;
            // Direct AddCharData appends a raster but does not replace the first map record.
            assert_eq!(f.add_char_data(0x41), Some(rec));
            assert!(f.appropriate_cache().pen_x > pen);
            f.face = Some(service_face(fixed, false, false, true, &drops));
            let pen = f.appropriate_cache().pen_x;
            let rec = f.get_char_data(0x42).unwrap();
            assert_eq!(rec.advance, expected);
            assert_eq!(f.appropriate_cache().pen_x, pen + 1 + 7); // loaded raster AND box appended
            f.face = Some(service_face(fixed, false, true, false, &drops));
            let missing = f.get_char_data(0x43).unwrap();
            assert_eq!((missing.width(), missing.height(), missing.advance), (7, 7, 7));
        }
    }
}

#[test]
fn borrowed_renderer_and_final_release_of_failed_initialisation() {
    let mut f = font();
    let r = Arc::new(LogRenderer::default());
    f.set_renderer(Some(r.clone()));
    assert_eq!(Arc::strong_count(&r), 2);
    let borrowed = f.get_renderer().unwrap();
    assert_eq!(Arc::strong_count(borrowed), 2);
    assert_eq!(Arc::strong_count(&r), 2);
    let o = FontObj::new(f);
    assert!(!o.query_interface(0xDEADBEEF));
    assert_eq!(o.ref_count(), 1);
    assert!(o.query_interface(1));
    assert_eq!(o.ref_count(), 2);
    assert_eq!(o.release(), 1);
    assert_eq!(o.release(), 0);
    assert_eq!(Arc::strong_count(&r), 1);
    assert_eq!(*r.0.lock().unwrap(), ["attach", "detach"]);

    let drops = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let mut failed = GzFont::new(lib_with(&[]), Arc::new(Latin1Converter));
    failed.face = Some(service_face(false, true, false, false, &drops));
    assert!(!failed.set_size(12));
    assert!(!failed.initialised);
    assert!(failed.shutdown()); // unpublished success cannot be inferred from a face alone
    assert_eq!(drops.load(std::sync::atomic::Ordering::SeqCst), 0);
    let o = FontObj::new(failed);
    assert_eq!(o.release(), 0); // destructor frees holder even though Shutdown was a no-op
    assert_eq!(drops.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(o.lock().face.is_none());
}

#[test]
fn unsigned_size_adjustment_and_zero_line_height_fault() {
    let mut f = font();
    let mut face = face("Negative", 0);
    face.metrics = SizeMetrics {
        a: -(5 << 16),
        b: 0,
        c: 0,
    };
    f.face = Some(Box::new(face));
    f.source_type = 2;
    assert!(f.set_size(16));
    assert_eq!(f.char_height, -3); // -5 + 1 is unsigned > 28, so add once more
    f.char_height = 0;
    f.line_spacing = 0;
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f.width_of_lines(b"a", 1))).is_err());
}

#[test]
fn layout_uses_characters_for_newline_space_and_paragraph_trimming() {
    let utf16 = |s: &str| s.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>();
    let mut f = font();
    f.code_page = 0x10;
    let mut rect = Rect::default();
    let lh = f.get_line_height();
    assert!(f.calculate_text_area_box(&utf16("\u{010A}"), &mut rect, 20, 100, lh));
    assert_eq!(rect.bottom, lh); // low byte 0x0A is not U+000A
    let mut text = utf16("ab cd");
    let l = text.len() as u32;
    let width = f.get_string_width(&utf16("ab ..."));
    assert_eq!(
        f.format_for_ellipsis(
            &mut text,
            &utf16("..."),
            Rect {
                right: width,
                ..Rect::default()
            },
            false
        ),
        l
    );
    assert_eq!(text, utf16("ab..."));
    assert!(f.draw_text_paragraph(
        None,
        Rect {
            right: 100,
            bottom: lh,
            ..Rect::default()
        },
        &utf16("A\r"),
        0,
        1,
        2,
        None
    ));
    assert_eq!(f.records.iter().map(|r| r.ch).collect::<Vec<_>>(), [0x41]);
}
