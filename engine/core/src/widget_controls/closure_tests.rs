// SPDX-License-Identifier: GPL-3.0-or-later
//! Literal cases absent from the earlier grouped owner scenarios.
use super::*;

fn selection_messages(f: &Recording, id: u32) -> usize {
    f.messages.iter().filter(|(_, m)| m.words[0] as u32 == id).count()
}

#[test]
fn wc_l19_l22_literal_removal_and_page_notifications() {
    for (selected, expected, messages) in [(1, 1, 0), (2, 1, 1)] {
        let (mut l, mut f) = list(false);
        l.entries
            .iter_mut()
            .zip([b"A", b"B", b"C"])
            .for_each(|(e, text)| e.text = text.to_vec());
        l.select(selected, false, &mut f);
        f.messages.clear();
        l.remove(selected, &mut f).unwrap();
        assert_eq!(l.selected, expected);
        assert_eq!(
            l.text(expected as usize),
            Some(if selected == 1 { b"C".as_slice() } else { b"B" })
        );
        assert_eq!(selection_messages(&f, list_box::SELECTION), messages);
    }
    for (count, first, expected) in [(20, 0, (7, 4)), (10, 5, (9, 6))] {
        let (mut l, mut f) = list(false);
        l.clear(&mut f);
        for id in 0..count {
            l.insert(-2, vec![], id, &mut f);
        }
        l.whole_rows = 4;
        l.first = first;
        f.messages.clear();
        assert_eq!(l.next_page(&mut f), Ok(true));
        assert_eq!((l.selected, l.first), expected);
        assert_eq!(selection_messages(&f, list_box::SELECTION), 0);
        if count == 10 {
            assert_eq!(l.next_page(&mut f), Ok(true));
            assert_eq!((l.selected, l.first), expected);
        }
    }
}

#[test]
fn wc_l27_l31_literal_sort_storage_and_highlight() {
    let (mut l, mut f) = list(false);
    l.entries[0].text = b"Z".to_vec();
    l.entries[1].text = b"A".to_vec();
    l.entries[2].text = b"A".to_vec();
    l.selected = 2;
    l.enable_sorting(&mut f);
    assert!(f.compare_calls > 0);
    assert_eq!(l.selected, 0); // duplicate text resolves to first sorted match
    let comparisons = f.compare_calls;
    l.enable_sorting(&mut f);
    assert_eq!(f.compare_calls, comparisons);
    let physical = l.entries.clone();
    let invalidations = f.invalidations;
    assert_eq!(l.remove(-3, &mut f), Ok(true));
    assert_eq!(l.entries, physical);
    assert_eq!(l.stored_count, 2);
    assert!(f.invalidations > invalidations);
    let (mut l, mut f) = list(false);
    assert_eq!(l.remove(-2, &mut f), Ok(true));
    assert_eq!((l.entries.len(), l.stored_count), (2, 2));
    assert_eq!(l.entries.last().unwrap().id, 7);
    l.select(0, false, &mut f);
    l.selection_color = 0x112233;
    for (value, expected) in [
        (f32::NEG_INFINITY, 0.0),
        (0.5, 0.5),
        (f32::INFINITY, 1.0),
        (f32::NAN, f32::NAN),
    ] {
        l.set_intensity(value, &mut f);
        assert!(if expected.is_nan() {
            l.intensity.is_nan()
        } else {
            l.intensity == expected
        });
        f.draws.clear();
        l.paint(&mut f).unwrap();
        assert_eq!(
            f.draws.iter().any(|d| matches!(d, Draw::Fill { color: 0x112233, .. })),
            expected == 1.0
        );
    }
    l.set_intensity(-0.0, &mut f);
    assert_eq!(l.intensity.to_bits(), (-0.0f32).to_bits());
}

#[test]
fn wc_t17_t24_literal_geometry_and_descriptor_children() {
    let (mut t, mut f, a, _) = tree();
    t.next_handle = 8;
    let child8 = t.insert_at(descriptor(a, b"one"), 0, &mut f);
    t.next_handle = 3;
    let child3 = t.insert_at(descriptor(a, b"two"), 1, &mut f);
    t.next_handle = 9;
    t.insert_at(descriptor(child8, b"grandchild"), 0, &mut f);
    t.set_expanded(a, true, &mut f);
    let refs = f.refs.len();
    let value = t.descriptor(a).unwrap();
    assert_eq!(value.children, [8, 3]);
    assert_eq!(f.refs.len(), refs);
    assert_eq!(value.font, t.nodes[&a].font);
    assert!(value.visible);
    assert_eq!((child8, child3), (8, 3));
    // WC-T17's literal textWidth=20 comes from the font service.
    t.flags &= !4;
    t.image_width = 12;
    t.first = 1;
    f.measure_override = Some(20);
    t.paint([0, 0, 200, 100], &mut f).unwrap();
    assert_eq!(t.nodes[&child8].text_rect, [39, 16, 62, 32]);
    assert!(f.draws.iter().any(|d| matches!(
        d,
        Draw::Text {
            rect: [42, 17, 62, 32],
            ..
        }
    )));
}

#[test]
fn wc_t26_restart_is_not_clean_initialization() {
    let (mut t, mut f, _, _) = tree();
    let root = t.root;
    let next = t.next_handle;
    let defaults = f.defaults.clone();
    t.shutdown(&mut f);
    let refs = f.refs.clone();
    t.shutdown(&mut f);
    assert_eq!(f.refs, refs);
    assert!(t.internal_initialized);
    assert_eq!((t.root, t.next_handle, t.default_font), (root, next, None));
    assert!(t.nodes.is_empty());
    assert_eq!(
        t.checked_init(&mut f),
        Err(Unsupported::Precondition(
            "live TreeView root (Shutdown is not restart)"
        ))
    );
    t.create_root();
    assert!(t.nodes.is_empty()); // retained stale pointer suppresses root creation
    assert_eq!(f.defaults, defaults);
}

#[test]
fn tree_negative_property_aliases_preserve_raw_side_effects() {
    let (mut t, mut f, _, _) = tree();
    t.layout_dirty = false;
    let before = (f.invalidations, f.refs.len(), f.messages.len());
    for (index, value) in [
        (-1, 9),
        (-2, 0xabcdef),
        (-4, 7),
        (-7, 2),
        (-11, 3),
        (-14, 4),
        (-15, 5),
        (-16, 6),
        (-17, 8),
        (-18, 12),
        (-19, 16),
        (-20, 0x123456),
        (-23, 11),
        (-38, 99),
    ] {
        t.set_default_color(index, value).unwrap();
        assert_eq!(t.default_color(index), Ok(value));
    }
    assert_eq!(t.line_style, [0xabcdef, 9]);
    assert_eq!(&t.raw_style[..4], &[8, 6, 5, 4]);
    assert_eq!(t.next_handle, 99);
    assert_eq!(t.flags, 11); // (0x118 - 0x174) / 4 = -23
    assert_eq!(t.default_color((0x114 - 0x174) / 4), Ok(100));
    t.raw_property_words.insert(-25, 0x12340000); // raw root pointer, not a node handle
    t.set_default_color(-25, 0x56780000).unwrap();
    assert_eq!(t.default_color(-25), Ok(0x56780000));
    assert!(t.default_color(i32::MIN).is_err()); // host did not supply readable memory
    assert!(t.set_default_color(i32::MIN, 1).is_err());
    assert_eq!(t.default_color(4), Ok(0));
    t.set_default_color(4, 1).unwrap();
    assert!(!t.layout_dirty);
    assert_eq!((f.invalidations, f.refs.len(), f.messages.len()), before);
    t.set_default_color((0x11c - 0x174) / 4, 0xab_80_00_ff).unwrap();
    assert!(t.internal_initialized && !t.layout_dirty && t.selection_dirty);
    assert_eq!(t.default_color(-22), Ok(0xab_80_00_ff));
    t.set_stored_pair([10, 20]);
    assert_eq!((t.default_color(-2), t.default_color(-1)), (Ok(10), Ok(20)));
    assert_eq!(t.base_color(&mut f), 0x123456);
}

#[test]
fn tree_automatic_layout_sizes_buffers_and_updates_both_bar_requirements() {
    let (mut t, mut f, a, _) = tree();
    f.tree_thickness = [10, 12];
    t.set_text(a, vec![b'x'; 40], &mut f);
    t.paint([0, 0, 100, 40], &mut f).unwrap();
    assert!(t.horizontal_bar.is_some() && t.vertical_bar.is_some());
    assert_eq!(t.viewport, [3, 3, 82, 24]);
    assert_eq!((t.viewport_rows, t.page_count), (1, 2));
    assert_eq!(f.tree_resizes.last().unwrap()[1], 48);
    t.remove_node(0, &mut f);
    t.paint([0, 0, 100, 40], &mut f).unwrap();
    assert!(t.horizontal_bar.is_none() && t.vertical_bar.is_none());
}

#[test]
fn tree_drag_helper_borrows_session_and_balances_capture() {
    let (mut t, mut f, a, _) = tree();
    t.paint([0, 0, 200, 100], &mut f).unwrap();
    let session = tree_view::DragSession {
        nodes: vec![a],
        buffer: 500,
        size: [20, 16],
    };
    let mut helper = tree_view::DragHelper {
        source: 700,
        session: &session,
        position: [0; 2],
        captured: false,
    };
    let refs = f.refs.clone();
    assert!(!helper.init([10, 20], &mut f));
    assert!(!helper.captured);
    f.base_init_ok = true;
    assert!(helper.init([10, 20], &mut f));
    assert_eq!((helper.position, f.capture_owner), ([10, 20], Some(700)));
    helper.move_to(&mut t, [20, 30], [500, 500], &mut f).unwrap();
    assert_eq!(helper.position, [20, 30]);
    helper.paint(&mut f);
    assert!(f.draws.contains(&Draw::Blit {
        source: 500,
        rect: [0, 0, 20, 16]
    }));
    assert!(helper.mouse_up(&mut f));
    assert_eq!(
        f.messages.last().unwrap().1,
        Message {
            kind: 0xf,
            words: [0; 4]
        }
    );
    helper.shutdown(&mut f);
    assert!(!helper.captured);
    assert_eq!(f.capture_owner, None);
    assert_eq!(f.refs, refs);
}

#[test]
fn wc_t30_literal_move_order_and_t51_open_stroke() {
    let (mut t, mut f, a, b) = tree();
    let mut d = descriptor(0, b"C");
    d.reference = -3;
    let c = t.insert(d, &mut f).unwrap();
    t.move_node(a, 0, b, &mut f);
    assert_eq!(t.nodes[&0].children, [b, a, c]);
    t.move_node(a, 0, 999, &mut f);
    assert_eq!(t.nodes[&0].children, [a, b, c]);
    t.insert(descriptor(a, b"child"), &mut f).unwrap();
    t.set_expanded(a, true, &mut f);
    t.paint([0, 0, 200, 100], &mut f).unwrap();
    let r = t.nodes[&a].expander_rect;
    assert!(f.draws.contains(&Draw::Line {
        points: [r[0] + 2, r[1] + 4, r[0] + 6, r[1] + 4],
        color: 0,
        style: 1
    }));
    assert!(!f.draws.contains(&Draw::Line {
        points: [r[0] + 4, r[1] + 2, r[0] + 4, r[1] + 6],
        color: 0,
        style: 1
    }));
}
