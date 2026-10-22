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
    let accepted = rows
        .iter()
        .enumerate()
        .map(|(i, (_, name))| (name.to_string(), i as u32 + 1))
        .collect();
    let mut f = font();
    f.face = Some(Box::new(NameFace {
        accepted,
        calls: calls.clone(),
    }));
    for (i, &(code, _)) in rows.iter().enumerate() {
        assert_eq!(f.name_map_index(code as u32), i as u32 + 1);
        assert_eq!(f.name_map_index(code as u32), i as u32 + 1);
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
    assert_eq!(c.product(1, f32::MAX, -128), Ok(0)); // no f32 overflow/storage
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
