// SPDX-License-Identifier: GPL-3.0-or-later
//! All 19 static conformance vectors added for WG5–WG9, win-grid §6.
use super::*;
use grid::{Cell, Format, SizeEntry, Value};

fn text(value: &[u8]) -> Cell {
    Cell {
        value: Value::Text(value.to_vec()),
        ..Cell::default()
    }
}

#[test]
fn wg5_a_bounds_ignore_flags_and_allow_signed_unlimited() {
    let (mut g, _, _) = setup(0);
    g.limits = [0, 3];
    assert!(g.within((-9, 2)));
    assert!(!g.within((-9, 3)));
    for flags in [0, 0x6000] {
        g.flags = flags;
        g.limits[0] = 2;
        assert!(!g.within((-9, 2)));
    }
    g.limits = [-1, -1];
    assert!(g.within((-2, -2)));
    g.limits[0] = -2;
    assert!(!g.within((0, 0)));
}

#[test]
fn wg5_b_used_range_includes_format_only_records() {
    let (mut g, _, mut f) = setup(0);
    let mut range = [99; 4];
    assert!(!g.get_used_range(&mut range));
    assert_eq!(range, [0; 4]);
    g.set_cell_format_state((4, 7), &Format::default(), &mut f).unwrap();
    assert!(g.get_used_range(&mut range));
    assert_eq!(range, [4, 7, 4, 7]);
}

#[test]
fn wg5_c_selection_mode_membership() {
    let (mut g, _, _) = setup(0);
    g.selection = vec![(2, 3)];
    for (mode, expected) in [
        (0, [true, false, true]),
        (1, [false, true, true]),
        (2, [false, false, true]),
    ] {
        g.selection_mode = mode;
        assert_eq!([(2, 99), (99, 3), (2, 3)].map(|cell| g.selected(cell)), expected);
    }
}

#[test]
fn wg5_d_relative_scroll_fills_viewport() {
    let (mut g, _, mut f) = setup(0x10000);
    g.limits[1] = 10;
    g.cell_size[1] = 60;
    g.scroll_cells(1, 9, &mut f);
    assert_eq!(g.position[1], 7);
    g.scroll_to(1, 9);
    assert_eq!(g.position[1], 9);
    g.scroll_cells(1, -100, &mut f);
    assert_eq!(g.position[1], 0);
    g.cell_size[1] = 0;
    g.scroll_cells(1, 9, &mut f);
    assert_eq!(g.position[1], 9);
    g.limits[1] = -1;
    g.scroll_cells(1, 100, &mut f);
    assert_eq!(g.position[1], 109);
}

#[test]
fn wg5_e_format_merge_is_independent_per_field() {
    let (mut g, _, mut f) = setup(0);
    g.default_format.words[6] = 1;
    let mut row = Format {
        font: Some(40),
        ..Format::default()
    };
    row.words[0] = 0xff0000;
    let col = Format {
        font: Some(50),
        ..Format::default()
    };
    let mut cell = Format::default();
    cell.words[1] = 0x0000ff;
    cell.words[6] = 0;
    g.sizes[1].insert(
        3,
        SizeEntry {
            format: row,
            ..SizeEntry::default()
        },
    );
    g.sizes[0].insert(
        2,
        SizeEntry {
            format: col,
            ..SizeEntry::default()
        },
    );
    g.set_cell_format_state((2, 3), &cell, &mut f).unwrap();
    let refs = f.refs.len();
    let result = g.cell_format((2, 3));
    assert_eq!(result.font, Some(50));
    assert_eq!(&result.words[..2], &[0xff0000, 0xff]);
    assert_eq!(result.words[6] & 0xff, 0);
    assert_eq!(f.refs.len(), refs);
}

#[test]
fn wg6_a_default_initialization_uses_conversion_and_preserves_explicit_fields() {
    let (mut g, _, mut f) = setup(0);
    g.dirty = false;
    g.default_formatting(&mut f);
    assert_eq!(g.default_format.words[6], 0x0005_0201);
    assert_eq!(g.header_format.words[6], 0x0006_0201);
    assert_eq!(&g.default_format.words[2..4], &[0x7fc0c0c0, 0x7fffffff]);
    assert_eq!(&g.header_format.words[4..6], &[0x7f000000, 0x7fdcdcff]);
    assert_eq!(g.line_colors, [0xff000000; 2]);
    assert!(!g.dirty);
    assert_eq!(f.refs.iter().filter(|&&(add, h)| add && h == 100).count(), 2);
    g.default_format.words[0] = 123;
    g.header_format.words[6] = 0x0107_0300;
    g.default_formatting(&mut f);
    assert_eq!(g.default_format.words[0], 123);
    assert_eq!(g.header_format.words[6], 0x0107_0300);
    assert_eq!(f.refs.iter().filter(|&&(add, h)| add && h == 100).count(), 2);
}

#[test]
fn wg6_b_axis_copy_resets_both_size_words_and_balances_fonts() {
    let (mut g, _, mut f) = setup(0);
    for (axis, cell) in [(0, (4, -1)), (1, (-1, 4))] {
        g.sizes[axis].insert(
            4,
            SizeEntry {
                format: Format {
                    font: Some(20),
                    ..Format::default()
                },
                explicit: 12,
                cached: 34,
            },
        );
        let format = Format {
            font: Some(30),
            words: [7; 8],
        };
        g.dirty = false;
        assert_eq!(g.set_cell_format_state(cell, &format, &mut f), Ok(true));
        assert_eq!(
            g.sizes[axis][&4],
            SizeEntry {
                format,
                explicit: -1,
                cached: -1
            }
        );
        assert_eq!(&f.refs[f.refs.len() - 2..], &[(true, 30), (false, 20)]);
        assert!(!g.dirty);
    }
}

fn draw_text(g: &Grid, f: &mut Recording, horizontal: u8, vertical: u8) -> (Rect, u32) {
    let mut format = Format {
        font: Some(40),
        ..Format::default()
    };
    format.words[0] = 42;
    format.words[6] = u32::from_le_bytes([0, horizontal, vertical, 0]);
    let record = PaintRecord {
        rect: [10, 20, 110, 60],
        clip: [10, 20, 110, 60],
        cell_area: [10, 20, 110, 60],
        coordinate: (0, 0),
        cell: None,
        format,
        selected: false,
        enabled: true,
    };
    f.owner.paints.clear();
    g.paint_text(&record, record.rect, b"text", f).unwrap();
    let rect = f
        .owner
        .paints
        .iter()
        .find_map(|p| {
            if let PaintOp::Primitive(Draw::Text { rect, color, .. }) = p {
                assert_eq!(*color, 42);
                Some(*rect)
            } else {
                None
            }
        })
        .unwrap();
    let alignment = f
        .owner
        .paints
        .iter()
        .find_map(|p| {
            if let PaintOp::TextAlignment(a) = p {
                Some(*a)
            } else {
                None
            }
        })
        .unwrap();
    (rect, alignment)
}

#[test]
fn wg6_c_text_placement_cross_field_quirk() {
    let (g, _, mut f) = setup(0);
    f.font_height = 9;
    for (horizontal, top, flags) in [(3, 51, 1), (4, 51, 2), (5, 20, 0)] {
        assert_eq!(draw_text(&g, &mut f, horizontal, 7), ([10, top, 110, 60], flags));
    }
}

#[test]
fn wg6_d_center_halves_truncate_separately() {
    let (g, _, mut f) = setup(0);
    f.font_height = 9;
    for horizontal in [3, 4, 5] {
        assert_eq!(draw_text(&g, &mut f, horizontal, 6).0, [10, 36, 110, 45]);
    }
}

#[test]
fn wg7_a_failed_editor_fault_retains_cleared_state() {
    let (mut g, mut c, mut f) = setup(0);
    g.set_window((0, 0), Some(99), true, true, 1, &mut f);
    f.owner.fail_editor = true;
    assert_eq!(
        g.set_editable_text((0, 0), Some(b"new"), 3, &mut c, &mut f),
        Err(Unsupported::Precondition("Grid null editor dereference"))
    );
    assert!(!g.cells.contains_key(&(0, 0)));
    assert!(!g.windows.contains_key(&99));
    assert!(f.editor_texts.is_empty());
    assert!(f.refs.contains(&(false, 99)));
}

fn header_setup() -> (Grid, LayoutContext, Recording) {
    let (mut g, c, mut f) = setup(0x18);
    f.grid_dimensions = [300, 200];
    g.gutters = [3, 5, 7, 11];
    g.header_size = [20, 40];
    g.default_size = [50, 20];
    (g, c, f)
}

#[test]
fn wg9_a_whole_header_bands() {
    let (mut g, _, mut f) = header_setup();
    assert_eq!(g.header_rect(0, -1, &mut f), Some([3, 5, 293, 25]));
    assert_eq!(g.header_rect(1, -1, &mut f), Some([3, 5, 23, 189]));
    g.flags = 0;
    assert_eq!(g.header_rect(0, -1, &mut f), None);
}

#[test]
fn wg9_b_individual_header_rectangles() {
    let (mut g, _, mut f) = header_setup();
    assert_eq!(g.cell_rect((0, 0), &mut f), [43, 25, 93, 45]);
    assert_eq!(g.header_rect(0, 0, &mut f), Some([43, 5, 93, 25]));
    assert_eq!(g.header_rect(1, 0, &mut f), Some([3, 25, 43, 45]));
    g.limits[0] = 1;
    assert_eq!(g.header_rect(0, 1, &mut f), None);
}

#[test]
fn wg9_c_whole_area_extends_clip_but_not_bitmap_alignment() {
    let (mut g, _, mut f) = header_setup();
    g.flags |= 0x20000;
    let mut format = Format {
        font: Some(40),
        ..Format::default()
    };
    format.words[0] = 42;
    format.words[6] = 4 << 8;
    let mut record = PaintRecord {
        rect: [43, 25, 93, 45],
        clip: [43, 25, 93, 45],
        cell_area: [43, 25, 293, 189],
        coordinate: (0, 0),
        cell: Some(text(b"text")),
        format,
        selected: false,
        enabled: true,
    };
    g.paint_content(&record, &mut f).unwrap();
    assert!(f.owner.paints.contains(&PaintOp::Clip([43, 25, 293, 45])));
    assert!(f
        .owner
        .paints
        .iter()
        .any(|p| matches!(p,PaintOp::Primitive(Draw::Text {rect,..}) if rect[2]==293)));
    f.owner.paints.clear();
    record.cell.as_mut().unwrap().value = Value::Bitmap(500);
    g.paint_content(&record, &mut f).unwrap();
    assert!(f.owner.paints.contains(&PaintOp::Clip([43, 25, 293, 45])));
    assert!(f
        .owner
        .paints
        .iter()
        .any(|p| matches!(p,PaintOp::Primitive(Draw::Blit {rect,..}) if rect[0]==84 && rect[2]==92)));
}

#[test]
fn wg8_a_partial_setters_preserve_inheritance_without_dirty_write() {
    let (mut g, mut c, mut f) = setup(0);
    g.dirty = false;
    assert!(g.set_cell_wrapping((2, 3), 0, &mut c, &mut f));
    assert!(g.set_cell_alignment((2, 3), 3, 7, &mut c, &mut f));
    assert!(g.set_cell_border((2, 3), 1, 0x12345678, &mut c, &mut f));
    assert_eq!(g.set_cell_colors((2, 3), 2, 0, &mut c, &mut f), Ok(true));
    let format = g.cells[&(2, 3)].format.as_ref().unwrap();
    assert_eq!(
        format.words,
        [0x40000000, 0x40000000, 0, 0x40000000, 0x40000000, 0x40000000, 0x01070300, 0x12345678]
    );
    assert_eq!(g.cells[&(2, 3)].value, Value::Empty);
    assert!(!g.dirty);
    assert_eq!(g.set_cell_colors((2, 3), 7, 9, &mut c, &mut f), Ok(true));
    assert_eq!(g.cells[&(2, 3)].format.as_ref().unwrap().words[7], 9);
    g.set_cell_extra((7, 7), 0);
    assert!(!g.cells.contains_key(&(7, 7)));
    g.limits = [1, 1];
    g.extra_mode = 2;
    assert!(!g.set_cell_extra((7, 7), 3));
    assert_eq!(g.extra_mode, 1);
}

#[test]
fn wg8_b_resize_only_increases_cached_actual_width() {
    let (mut g, mut c, mut f) = setup(0);
    c.depth = 2;
    g.dirty = false;
    for (row, handle, width) in [(0, 40, 40), (1, 80, 80)] {
        f.owner.sizes.insert(handle, [width, 20]);
        g.cells.insert(
            (2, row),
            Cell {
                value: Value::Window {
                    handle,
                    keep_width: false,
                    keep_height: false,
                },
                ..Cell::default()
            },
        );
    }
    g.sizes[0].insert(
        2,
        SizeEntry {
            cached: 100,
            ..SizeEntry::default()
        },
    );
    g.sizes[1].insert(
        0,
        SizeEntry {
            cached: 55,
            ..SizeEntry::default()
        },
    );
    g.resize_columns_to_fit_windows(2, 1, &mut c, &mut f);
    assert_eq!(g.sizes[0][&2].cached, 100);
    assert!(!g.dirty);
    g.sizes[0].get_mut(&2).unwrap().cached = -1;
    g.resize_columns_to_fit_windows(2, 1, &mut c, &mut f);
    assert_eq!(g.sizes[0][&2].cached, 80);
    assert_eq!(g.sizes[1][&0].cached, -1);
    assert!(g.dirty);
}

fn sort_setup() -> (Grid, LayoutContext, Recording) {
    let (mut g, mut c, f) = setup(0);
    c.depth = 2;
    for (row, value) in [(4, b"b"), (9, b"A"), (12, b"a")] {
        g.cells.insert((0, row), text(value));
    }
    for row in [4, 6, 9, 12, 15] {
        g.cells.insert(
            (1, row),
            Cell {
                extra: row as u32,
                ..Cell::default()
            },
        );
    }
    (g, c, f)
}

#[test]
fn wg8_c_column_sort_stable_and_packs_to_zero() {
    let (mut g, mut c, mut f) = sort_setup();
    g.sort_column(0, 0, false, None, &mut c, &mut f);
    assert_eq!(g.cells[&(0, 0)].value, Value::Text(b"A".to_vec()));
    assert_eq!(g.cells[&(0, 1)].value, Value::Text(b"a".to_vec()));
    assert_eq!(g.cells[&(0, 2)].value, Value::Text(b"b".to_vec()));
    assert_eq!(g.cells[&(1, 9)].extra, 9);
}

#[test]
fn wg8_d_whole_sort_packs_unmatched_after_keys_per_column() {
    let (mut g, mut c, mut f) = sort_setup();
    g.headings[1].insert(9, b"heading".to_vec());
    g.sort_column(0, 0, true, Some(&mut grid::compare_c_locale), &mut c, &mut f);
    assert_eq!([2, 3, 0, 1, 4].map(|row| g.cells[&(1, row)].extra), [4, 6, 9, 12, 15]);
    assert_eq!(g.headings[1][&9], b"heading");
}

#[test]
fn wg8_e_row_sort_selection_looks_up_old_row() {
    let (mut g, mut c, mut f) = setup(0);
    c.depth = 2;
    g.cells.insert((2, 5), text(b"b"));
    g.cells.insert((8, 5), text(b"a"));
    g.selection = vec![(2, 5), (8, 5)];
    g.sort_row(5, 0, false, None, &mut c, &mut f);
    assert_eq!(g.cells[&(0, 5)].value, Value::Text(b"a".to_vec()));
    assert_eq!(g.cells[&(1, 5)].value, Value::Text(b"b".to_vec()));
    assert_eq!(g.selection, [(2, 5), (3, 5)]);
}

#[test]
fn wg8_f_c_locale_is_narrow_case_insensitive_not_numeric() {
    use std::cmp::Ordering::*;
    for (a, b, expected) in [
        (b"Z".as_slice(), b"a".as_slice(), Greater),
        (b"A", b"a", Equal),
        (b"10", b"2", Less),
        (b"A\0z", b"a", Equal),
    ] {
        assert_eq!(grid::compare_c_locale(&text(a), &text(b)), expected);
    }
    assert_eq!(
        grid::compare_c_locale(
            &text(b"a"),
            &Cell {
                value: Value::Bitmap(40),
                ..Cell::default()
            }
        ),
        Equal
    );
}
