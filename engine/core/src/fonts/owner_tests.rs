// SPDX-License-Identifier: GPL-3.0-or-later
//! Owner-boundary conformance cases from drawing-fonts.md §§2–11.
use super::*;

#[test]
fn destructor_shuts_down_before_releasing_foreign_renderer() {
    let r = Arc::new(LogRenderer::default());
    {
        let mut f = font();
        f.set_renderer(Some(r.clone()));
    }
    assert_eq!(*r.0.lock().unwrap(), ["attach", "detach"]);
    assert_eq!(Arc::strong_count(&r), 1);

    r.0.lock().unwrap().clear();
    {
        let mut f = font();
        f.set_renderer(Some(r.clone()));
        assert!(f.shutdown());
        assert!(f.shutdown());
    }
    assert_eq!(*r.0.lock().unwrap(), ["attach", "detach"]);

    r.0.lock().unwrap().clear();
    let o = FontObj::new(font());
    o.lock().set_renderer(Some(r.clone()));
    drop(o); // Portable ownership also runs the deleting destructor's Shutdown.
    assert_eq!(*r.0.lock().unwrap(), ["attach", "detach"]);
}

#[test]
fn init_guards_and_constructor_size_face_attributes() {
    let mut f = GzFont::new(lib_with(&[("a.ttf", "Family", 3)]), Arc::new(Latin1Converter));
    assert!(!f.init());
    f.set_file_path("missing.ttf");
    assert!(!f.init());
    f.set_file_path("a.ttf");
    f.set_style(0xFFFF, true);
    assert!(f.init());
    assert_eq!((f.point_size, f.char_height, f.ascent, f.descent_raw), (16, 15, 13, 10));
    assert_eq!((f.face_kind, f.style_flags, f.default_glyph), (1, 0x3F, 0x20));
    assert_eq!(f.family_name, "Family");
    assert_eq!(f.load_flags, 0x8A);
    f.set_file_path("missing.ttf");
    assert!(f.init()); // Already initialised: do not inspect the replacement path.
    assert_eq!(f.family_name, "Family");
    assert!(f.shutdown());
    assert!(!f.initialised && f.face.is_none());
    assert!(f.mono_cache.block.is_empty() && f.grey_cache.block.is_empty());
}

fn fill_both_caches(f: &mut GzFont) {
    f.set_antialiasing(1);
    f.get_char_data(0x61).unwrap();
    f.set_antialiasing(3);
    f.get_char_data(0x61).unwrap();
    assert_eq!((f.mono_cache.glyph_count(), f.grey_cache.glyph_count()), (1, 1));
}

#[test]
fn attribute_cache_invalidation_and_size_gate() {
    let mut f = font();
    fill_both_caches(&mut f);
    assert!(!f.set_size(0));
    assert!(!f.set_size(30));
    assert_eq!(f.point_size, 12);
    f.set_style(0xFFFF, true);
    assert_eq!(f.style_flags, 0x3C);
    f.set_spacing_adjustment(2, 3);
    f.set_font_color(123, 456);
    f.set_antialiasing(0);
    assert_eq!(f.aa_mode, 3);
    assert!(f.get_style(0));
    assert!(!f.get_style(0x40));
    assert_eq!((f.mono_cache.glyph_count(), f.grey_cache.glyph_count()), (1, 1));
    f.set_aa_gamma(1.0); // No change: preserve both caches.
    assert_eq!(f.grey_cache.glyph_count(), 1);
    f.set_aa_gamma(2.0);
    assert_eq!(f.aa_table.len(), 256);
    assert_eq!((f.mono_cache.glyph_count(), f.grey_cache.glyph_count()), (1, 0));
    f.get_char_data(0x61);
    f.set_aa_gamma(1.0); // Identity empties the table without clearing grey glyphs.
    assert!(f.aa_table.is_empty());
    assert_eq!(f.grey_cache.glyph_count(), 1);
    f.set_advance_scaling(1.0);
    assert_eq!(f.grey_cache.glyph_count(), 1);
    f.set_advance_scaling(2.0);
    assert_eq!((f.mono_cache.glyph_count(), f.grey_cache.glyph_count()), (0, 0));
    assert!(f.set_size(4096));
    assert_eq!(f.point_size, 2048);
    fill_both_caches(&mut f);
    let metrics = (f.char_height, f.ascent, f.descent_raw);
    f.set_scaling(1.0, 1.0);
    assert_eq!(f.grey_cache.glyph_count(), 1);
    f.set_scaling(2.0, 3.0);
    assert_eq!((f.mono_cache.glyph_count(), f.grey_cache.glyph_count()), (0, 0));
    assert_eq!((f.char_height, f.ascent, f.descent_raw), metrics);
}

#[test]
fn cache_growth_preserves_pixels_padding_and_records() {
    let mut c = CachedBitmapData::default();
    c.reallocate(3, 2);
    let at = c.offset(2, 1);
    c.block[at] = 0xAB;
    c.glyphs.insert(
        0x61,
        GlyphRecord {
            src_right: 3,
            src_bottom: 2,
            ..GlyphRecord::default()
        },
    );
    c.reallocate(6, 4);
    assert_eq!(c.block.len(), 48);
    assert_eq!(c.block[c.offset(2, 1)], 0xAB);
    assert_eq!(c.block.iter().filter(|&&b| b != 0).count(), 1);
    assert_eq!(c.glyphs[&0x61].width(), 3);
    c.reallocate(1, 1);
    assert_eq!((c.pitch, c.height), (6, 4));
    c.clear();
    assert!(c.block.is_empty() && c.glyphs.is_empty());
    assert_eq!((c.pitch, c.height, c.pen_x), (0, 0, 0));
}

#[test]
fn glyph_unpack_remap_and_decorations() {
    let mut f = font();
    let g = GlyphRaster {
        width: 9,
        rows: 1,
        pitch: 2,
        buffer: vec![0x81, 0x80],
        ..GlyphRaster::default()
    };
    let r = f.copy_glyph_to_source_data(&g, true, -2, 3, 9);
    let row = f.mono_cache.offset(r.src_left as u32, 0);
    assert_eq!(&f.mono_cache.block[row..row + 9], &[255, 0, 0, 0, 0, 0, 0, 255, 255]);
    assert_eq!((r.x_offset, r.y_offset), (-2, 3));

    f.set_antialiasing(3);
    f.aa_table = (0..=255u8).rev().collect();
    let g = GlyphRaster {
        width: 2,
        rows: 1,
        pitch: 2,
        buffer: vec![0, 128],
        ..GlyphRaster::default()
    };
    f.set_style(0xC, true);
    let r = f.copy_glyph_to_source_data(&g, false, 2, 3, 6);
    assert_eq!((r.width(), r.height(), r.x_offset), (6, 12, 0));
    let c = &f.grey_cache;
    assert_eq!(&c.block[c.offset(0, 0)..c.offset(0, 0) + 6], &[0, 0, 255, 127, 0, 0]);
    // The underline begins at 13 - 3 + 1 = 11; strike at 13 - (15 >> 2) - 3 = 7.
    for y in [7, 11] {
        assert_eq!(&c.block[c.offset(0, y)..c.offset(0, y) + 6], &[255; 6]);
    }
}

#[test]
fn draw_preloads_before_records_and_strict_clip_still_advances_pen() {
    let mut f = font();
    let text: Vec<u8> = (0x21..0x7F).collect();
    assert!(f.draw_text_line(None, 0, 0, &text, 0, 0, None, None));
    assert_eq!(f.records.len(), text.len());
    assert!(f.mono_cache.pitch > 7 * 32); // Enough distinct glyphs to grow the bitmap.
    for r in &f.records {
        let g = f.mono_cache.glyphs[&r.ch];
        assert_eq!(r.src_pitch, f.mono_cache.pitch);
        assert_eq!(r.src_offset, f.mono_cache.offset(g.src_left as u32, 0));
        assert_eq!(f.mono_cache.block[r.src_offset], 255);
    }
    f.set_spacing_adjustment(2, 4);
    let mut pen = (10, 20);
    // 'a' box left=11, top=27, right=19, bottom=37. Touching right edge is not overlap.
    assert!(f.draw_text_line(None, 99, 99, b"a", 0, 0, Some((19, 27, 30, 37)), Some(&mut pen)));
    assert!(f.records.is_empty());
    assert_eq!(pen, (20, 20));
    assert!(f.draw_text_line(None, 10, 20, b"a", 0, 0, Some((18, 27, 30, 37)), None));
    assert_eq!(
        (
            f.records[0].left,
            f.records[0].top,
            f.records[0].right,
            f.records[0].bottom
        ),
        (11, 27, 19, 37)
    );
}

#[test]
fn palette_mru_keeps_four_and_matches_all_colour_bits() {
    let mut f = font();
    for fg in 1..=5 {
        f.set_current_palette(fg, 0);
    }
    assert_eq!(f.palettes.iter().map(|p| p.fg).collect::<Vec<_>>(), [5, 4, 3, 2]);
    f.set_current_palette(3, 0);
    assert_eq!(f.palettes.iter().map(|p| p.fg).collect::<Vec<_>>(), [3, 5, 4, 2]);
    f.set_current_palette(3, 0);
    assert_eq!(f.palettes.len(), 4);
    f.set_current_palette(0x01000003, 0);
    assert_eq!(
        f.palettes.iter().map(|p| p.fg).collect::<Vec<_>>(),
        [0x01000003, 3, 5, 4]
    );
    f.set_current_palette(0x01000003, 0xFF000000);
    assert_eq!(f.current_palette.as_ref().unwrap().entries[0], 0xFF000000);
    assert_eq!(f.current_palette.as_ref().unwrap().entries[127], 0x01000003);
}
