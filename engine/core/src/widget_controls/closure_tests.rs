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
fn wc_l03_l13_drop_selection_and_outline_setters_invalidate() {
    let (mut l, mut f) = list(true);
    l.select(1, false, &mut f);
    f.messages.clear();
    for _ in 0..2 {
        let before = f.invalidations;
        assert!(l.select(99, true, &mut f));
        assert_eq!(l.selected, 2);
        assert!(f.invalidations > before);
        assert!(f.messages.is_empty());
    }
    let before = f.invalidations;
    l.set_flat(true, &mut f);
    assert!(l.flat && !l.three_d);
    assert_eq!(f.invalidations, before + 1);
    l.set_three_d(true, &mut f);
    assert!(!l.flat && l.three_d);
    assert_eq!(f.invalidations, before + 2);
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
    let mut output = value.clone();
    output.children = vec![100, 200, 300];
    assert!(t.descriptor_into(a, &mut output));
    assert_eq!(output, value);
    let saved = output.clone();
    assert!(!t.descriptor_into(999, &mut output));
    assert_eq!(output, saved);
    assert_eq!(f.refs.len(), refs);
    assert_eq!((child8, child3), (8, 3));
    // WC-T17's literal textWidth=20 comes from the font service.
    t.flags &= !4;
    t.image_width = 12;
    t.first = 1;
    f.measure_override = Some(20);
    t.paint([0, 0, 200, 100], &mut f).unwrap();
    assert_eq!(t.nodes[&child8].expander_rect, [12, 19, 21, 28]);
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
fn wc_t25_descriptor_preserves_topology_and_false_visibility_but_dirties() {
    let (mut t, mut f, a, _) = tree();
    let child = t.insert(descriptor(a, b"child"), &mut f).unwrap();
    let mut value = t.descriptor(a).unwrap();
    value.force_branch = true;
    value.visible = false;
    value.children = vec![999];
    t.layout_dirty = false;
    let before = (f.invalidations, f.refs.len(), f.messages.len());
    assert!(t.apply_descriptor(a, value.clone(), &mut f));
    assert!(!t.nodes[&a].force_branch);
    assert_eq!(t.nodes[&a].children, [child]);
    assert!(t.layout_dirty);
    assert!(f.invalidations > before.0);
    assert_eq!((f.refs.len(), f.messages.len()), (before.1, before.2));
    assert!(t.get_visible(a));
    assert!(!t.get_visible(child)); // False visibility did not expand or hide its parent.
    t.layout_dirty = false;
    let before = (f.invalidations, f.refs.len(), f.messages.len());
    assert!(!t.apply_descriptor(999, value, &mut f));
    assert!(!t.layout_dirty);
    assert_eq!((f.invalidations, f.refs.len(), f.messages.len()), before);
}

#[test]
fn wc_t02_t05_silent_noops_and_exact_selection_payloads() {
    let (mut t, mut f, a, b) = tree();
    let before = (f.invalidations, f.messages.len());
    assert!(!t.set_expanded(999, true, &mut f));
    assert_eq!((f.invalidations, f.messages.len()), before);
    t.select(a, false, &mut f);
    t.select(b, true, &mut f);
    assert!(!t.deselect(999, &mut f));
    assert_eq!(t.selection, [b]);
    assert_eq!(f.messages.len(), 2);
    assert!(f
        .messages
        .iter()
        .all(|(_, m)| { m.kind == 3 && m.words == [tree_view::SELECTION as i32, 0, 0, 0] }));
    t.clear_selection(&mut f);
    t.flags |= 0x10;
    t.select(a, false, &mut f);
    t.select(b, false, &mut f);
    assert_eq!(t.selection, [a, b]);
    f.messages.clear();
    assert!(t.clear_selection(&mut f));
    assert!(t.selection.is_empty());
    assert_eq!(selection_messages(&f, tree_view::SELECTION), 1);
    assert!(t.clear_selection(&mut f));
    assert_eq!(f.messages.len(), 1);
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
fn tree_primary_insert_wrappers_copy_initialized_defaults_and_failure_data() {
    let mut t = tree_view::TreeView::new(1);
    let mut f = Recording {
        font_height: 10,
        ..Recording::default()
    };
    let mut text = b"image node".to_vec();
    let a = t.insert_images(0, -3, &text, [4, 5, 6, 7], &mut f).unwrap();
    text[0] = b'X';
    let n = t.node(a).unwrap();
    assert_eq!(n.text, b"image node");
    assert_eq!(
        (n.data, n.images, n.font, n.colors, n.expanded),
        (0, [4, 5, 6, 7], Some(100), [0, 0xffffff, 0xffffff, 0], false)
    );
    assert_eq!((n.parent, n.reference), (Some(0), -3));
    let refs = f.refs.clone();
    let b = t.insert_text(0, -2, b"data node", 0xabcdef, &mut f).unwrap();
    assert_eq!(t.node(b).unwrap().images, [0, 1, 2, 3]);
    assert_eq!(t.data(b), 0xabcdef);
    assert_eq!(t.nodes[&0].children, [b, a]);
    assert_eq!(f.refs, refs); // nodes borrow their default font
    t.layout_dirty = false;
    let before = (t.next_handle, t.nodes.len(), f.invalidations);
    assert_eq!(t.insert_text(0, 999, b"abandoned", 42, &mut f), Ok(-1));
    assert_eq!(t.next_handle, before.0 + 1);
    assert_eq!((t.nodes.len(), f.invalidations), (before.1, before.2));
    assert_eq!(t.nodes[&0].children, [b, a]); // WC-T23: no child-count commit.
    assert!(!t.layout_dirty);
    assert_eq!(t.abandoned.last().unwrap().data, 42);
    assert_eq!(t.abandoned.last().unwrap().text, b"abandoned");
    let next = t.next_handle;
    assert_eq!(t.insert_images(999, -3, b"bad parent", [0; 4], &mut f), Ok(-1));
    assert_eq!(t.next_handle, next);
    assert!(t.clear_root(&mut f));
    assert_eq!(t.root, Some(0));
    assert_eq!(t.nodes.len(), 1);
    assert!(t.nodes[&0].children.is_empty());
    // O24 is independently lazy, including the default color conversions.
    let mut lazy = tree_view::TreeView::new(2);
    let b = lazy.insert_text(0, -3, b"lazy", 7, &mut f).unwrap();
    assert_eq!(lazy.node(b).unwrap().colors, [0, 0xffffff, 0xffffff, 0]);
}

#[test]
fn wc_t46_drag_ending_on_descendant_cancels_without_restoring_selection() {
    let (mut t, mut f, a, _) = tree();
    let child = t.insert(descriptor(a, b"child"), &mut f).unwrap();
    t.set_expanded(a, true, &mut f);
    t.paint([0, 0, 200, 100], &mut f).unwrap();
    t.layout_viewport([0, 0, 200, 100], [200, 100], [0, 0]).unwrap();
    t.select(a, true, &mut f);
    f.drag_accept = 1;
    f.drag_return = true;
    let row = t.nodes[&child].visible_index.unwrap() as i32;
    f.drag_pointer = Some([10, row * 16 + 3]);
    assert_eq!(t.cursor_row(10, row * 16 + 7), Ok(Some(child)));
    let topology = t.nodes.clone();
    f.messages.clear();
    assert_eq!(t.mouse_move(199, 90, 0x10, &mut f), Ok(true));
    assert_eq!(f.drag_runs, 1);
    assert!(t.selection.is_empty());
    assert!(t.nodes.values().all(|node| !node.selected));
    assert!(!f.messages.iter().any(|(_, m)| m.words[0] as u32 == 0x1af5a141));
    for (handle, node) in topology {
        assert_eq!(t.nodes[&handle].children, node.children);
        assert_eq!(t.nodes[&handle].parent, node.parent);
    }
    assert!(!t.insertion_side);
}

#[test]
fn tree_child_array_adapter_preserves_unwritten_elements_and_missing_outputs() {
    let (mut t, mut f, a, _) = tree();
    let child = t.insert(descriptor(a, b"child"), &mut f).unwrap();
    let first = t.insert(descriptor(a, b"first"), &mut f).unwrap();
    t.insert(descriptor(child, b"grandchild"), &mut f).unwrap();
    let before = (f.refs.len(), f.invalidations, f.messages.len(), t.layout_dirty);
    let mut output = [999; 3];
    assert_eq!(t.children(a, Some(&mut output)), Ok(true));
    assert_eq!(output, [first, child, 999]); // Immediate children, in child-list order.
    assert_eq!(t.children(999, Some(&mut output)), Ok(false));
    assert_eq!(output, [first, child, 999]);
    assert_eq!(t.children(a, None), Ok(false));
    assert_eq!(t.children(first, Some(&mut [])), Ok(true));
    assert_eq!(t.children(first, Some(&mut output)), Ok(true));
    assert_eq!(output, [first, child, 999]); // An empty list writes no array elements.
    let mut short = [999];
    assert_eq!(
        t.children(a, Some(&mut short)),
        Err(Unsupported::Precondition("TreeView child output storage"))
    );
    assert_eq!(short, [999]);
    assert_eq!(
        (f.refs.len(), f.invalidations, f.messages.len(), t.layout_dirty),
        before
    );
}

#[test]
fn tree_selection_output_and_borrowed_adapter_inventory() {
    let (mut t, mut f, a, b) = tree();
    assert_eq!(t.copy_selection(None), Ok(true));
    t.flags |= 0x10;
    t.select(a, false, &mut f);
    t.select(b, false, &mut f);
    let before = (f.refs.len(), f.invalidations, f.messages.len());
    let mut output = [999; 3];
    assert_eq!(t.copy_selection(Some(&mut output)), Ok(true));
    assert_eq!(output, [a, b, 999]);
    assert_eq!(t.selected_handles().len(), 2); // O53
    assert_eq!(t.first_selected(), a); // O54, distinct from navigation anchor
    assert!(t.copy_selection(None).is_err());
    let mut short = [999];
    assert!(t.copy_selection(Some(&mut short)).is_err());
    assert_eq!(short, [999]);
    for h in [a, b] {
        assert_eq!(t.parent(h), 0);
        assert_eq!(t.insertion_reference(h), -2);
        assert!(t.is_selected(h));
        assert_eq!(t.node_font(h), Some(100));
        assert!(!t.expanded(h));
        assert_eq!(t.child_count(h), 0);
    }
    assert_eq!(t.child_count(0), 2);
    assert!(t.expanded(0));
    assert_eq!(
        (
            t.node_font(999),
            t.expanded(999),
            t.child_count(999),
            t.is_selected(999)
        ),
        (None, false, 0, false)
    );
    assert_eq!((t.parent(999), t.insertion_reference(999), t.data(999)), (-1, -1, 0));
    assert_eq!((f.refs.len(), f.invalidations, f.messages.len()), before);
}

#[test]
fn wc_t31_literal_selected_child_and_grandchild_removal_preserves_root() {
    let (mut t, mut f, a, _) = tree();
    let child = t.insert_text(a, -3, b"child", 0, &mut f).unwrap();
    let grandchild = t.insert_text(child, -3, b"grandchild", 0, &mut f).unwrap();
    t.flags |= 0x10;
    t.select(child, false, &mut f);
    t.select(grandchild, false, &mut f);
    f.messages.clear();
    let root = t.root;
    assert!(t.clear_root(&mut f));
    assert_eq!(t.root, root);
    assert_eq!(t.nodes.len(), 1);
    assert!(t.nodes[&0].children.is_empty());
    assert!(t.selection.is_empty());
    assert!(!t.nodes.contains_key(&child));
    assert!(!t.nodes.contains_key(&grandchild));
    assert_eq!(selection_messages(&f, tree_view::SELECTION), 2);
    assert!(f
        .messages
        .iter()
        .all(|(_, m)| m.words == [tree_view::SELECTION as i32, 0, 0, 0]));
}

#[test]
fn tree_property_setters_and_raw_resource_aliases_have_literal_side_effects() {
    let (mut t, mut f, a, _) = tree();
    // All changed layout properties invalidate once; identical writes do not.
    let start = f.invalidations;
    for _ in 0..2 {
        t.set_margins(8, 9, &mut f);
        t.set_spacing(7, &mut f);
        t.set_row_height(0, &mut f); // no lower-bound validation
        t.set_expander_size(u32::MAX, &mut f); // unsigned clamp to flags, not h
        t.set_base_color(42, &mut f);
    }
    assert_eq!(f.invalidations, start + 5);
    assert_eq!(
        (&t.raw_style[..4], t.row_height, t.base_color(&mut f)),
        (&[11, 7, 8, 9][..], 0, 42)
    );
    t.raw_style[4] = 3;
    let start = f.invalidations;
    t.set_style_bits(3, false, &mut f);
    t.set_style_bits(3, false, &mut f);
    assert!(!t.style_bits(3));
    assert_eq!(f.invalidations, start + 1);
    t.set_stored_pair([20, 30]);
    t.set_selection_color(50, &mut f);
    assert_eq!(f.invalidations, start + 1); // no selection
    t.set_row_height(16, &mut f);
    t.select(a, true, &mut f);
    let start = f.invalidations;
    t.set_selection_color(60, &mut f);
    t.set_selection_color(60, &mut f);
    assert_eq!(f.invalidations, start + 1);
    t.layout_dirty = false;
    let before = (f.invalidations, f.messages.len());
    f.refs.clear();
    t.set_default_font(Some(500), &mut f);
    t.set_default_font(Some(500), &mut f);
    assert_eq!(f.refs, [(true, 500), (false, 100)]);
    assert_eq!(t.nodes[&a].font, Some(100)); // existing borrowed font unchanged
    for (index, word) in [(-24, 600), (-10, 601), (-9, 602), (-8, 603)] {
        t.set_default_color(index, word).unwrap();
        assert_eq!(t.default_color(index), Ok(word));
    }
    assert_eq!(
        (t.default_font, t.working_buffer, t.horizontal_bar, t.vertical_bar),
        (Some(600), Some(601), Some(602), Some(603))
    );
    assert_eq!(f.refs, [(true, 500), (false, 100)]); // raw words never retain/release
    assert_eq!((f.invalidations, f.messages.len()), before);
    assert!(!t.layout_dirty);
    t.set_default_color(-22, 0xfa_80_02_03).unwrap();
    assert_eq!(t.default_color(-22), Ok(0xfa_80_02_03));
    t.layout_dirty = false;
    assert_eq!(t.default_color(-22), Ok(0xfa_80_00_03));
}

#[test]
fn tree_internal_init_failures_do_not_retry_resources() {
    for (no_graphics, fail_working_buffer, no_default_font) in
        [(false, true, false), (true, false, false), (false, false, true)]
    {
        let mut f = Recording {
            no_graphics,
            fail_working_buffer,
            no_default_font,
            ..Recording::default()
        };
        let mut t = tree_view::TreeView::new(1);
        assert!(t.init(&mut f)); // ignores base Init's false result
        assert!(t.internal_initialized);
        let before = (f.defaults.clone(), f.image_attempts.clone(), f.refs.clone());
        assert_eq!(t.working_buffer.is_none(), no_graphics || fail_working_buffer);
        assert_eq!(t.default_font.is_none(), no_default_font);
        f.no_graphics = false;
        f.fail_working_buffer = false;
        f.no_default_font = false;
        assert!(t.init(&mut f));
        assert_eq!((f.defaults.clone(), f.image_attempts.clone(), f.refs.clone()), before);
        assert_eq!(t.working_buffer.is_none(), no_graphics || fail_working_buffer);
    }
}

#[test]
fn list_primary_bytes_and_exact_search_preserve_outputs_without_effects() {
    let (mut l, f) = list(false);
    let before = (f.invalidations, f.refs.len(), f.messages.len(), l.area);
    l.set_drop_byte(0x80);
    assert_eq!(l.drop_byte(), 0x80);
    assert!(l.drop_mode);
    assert!(l.set_drop_policy_bytes([0xff, 0x80]));
    assert_eq!(l.drop_policy_bytes(), [0xff, 0x80]);
    assert!(l.set_delivery(0xfe));
    assert_eq!(l.delivery_byte(), 0xfe);
    assert_eq!(l.delivery, Delivery::Post);
    assert_eq!((f.invalidations, f.refs.len(), f.messages.len(), l.area), before);
    l.entries[0].text = b"Alpha".to_vec();
    l.entries[1].text = b"alpha".to_vec();
    let mut index = 99;
    assert!(l.find_exact(b"alpha", &mut index));
    assert_eq!(index, 1);
    assert!(!l.find_exact(b"alph", &mut index));
    assert_eq!(index, 1);
    assert!(!l.find_exact(b"ALPHA", &mut index));
    assert_eq!(index, 1);
    l.set_drop_byte(0);
    l.set_drop_policy_bytes([0, 0]);
    l.set_delivery(0);
    assert_eq!(
        (l.drop_byte(), l.drop_policy_bytes(), l.delivery_byte()),
        (0, [0, 0], 0)
    );
}

#[test]
fn list_residual_slot_adapters_colors_gutters_skin_and_base_flags() {
    let (mut l, mut f) = list(false);
    let mut color = 99;
    assert!(!l.selection_color(&mut color, &mut f));
    assert_eq!(color, 99);
    l.set_selection_color(0x123456, &mut f);
    assert!(l.selection_color(&mut color, &mut f));
    assert_eq!(color, 0x123456);
    let count = f.invalidations;
    assert!(l.reset_font_color(0, &mut f));
    assert_eq!(l.font_colors[0], SENTINEL);
    assert_eq!(f.invalidations, count + 1); // sentinel was already present
    assert!(!l.reset_font_color(3, &mut f));
    assert!(l.set_bar_gaps(0xfe, 0x80, &mut f));
    assert_eq!((l.bar_gap, l.bar_gap_y), (-2, -128));
    assert!(l.set_text_gutters(0xff, 2, &mut f));
    assert_eq!((l.x_gutter, l.y_gutter), (-1, 2));
    assert!(l.set_minimum_height(0xff, &mut f));
    assert_eq!((l.minimum_height, l.row_height), (255, 255));
    l.bar = Some(300);
    l.services[0] = Some(500);
    f.refs.clear();
    assert!(l.set_bar_skin(Some(500), [1, 2, 7, 9], &mut f));
    assert_eq!(f.refs, [(true, 500), (false, 500)]);
    assert_eq!(l.skin_rect, [1, 2, 7, 9]);
    assert_eq!(f.skins, [(300, 500, [1, 2, 7, 9])]);
    f.buffer_height = 20;
    assert!(l.set_bar_skin(Some(500), [0; 4], &mut f));
    assert_eq!(f.skins.last(), Some(&(300, 500, [0, 0, 0, 20])));
    assert!(l.set_bar_skin(None, [5; 4], &mut f));
    assert_eq!(l.services[0], None);
    assert!(!l.false_stub(1, 2));
    f.base_flags = 0x8903;
    assert_eq!(l.set_flag(2, 0, &mut f), Ok(0x8901));
    assert_eq!(l.set_flag(2, 0x80, &mut f), Ok(0x8903));
    assert_eq!(f.base_flag_calls, [(l.window, 2, 0), (l.window, 2, 0x80)]);
}

#[test]
fn wc_t40_literal_icon_click_and_t49_failed_access_has_no_release() {
    let (mut t, mut f, a, _) = tree();
    t.flags |= 4;
    t.set_image(0, Some(80), [0, 0, 6, 8], &mut f);
    t.set_node_image(a, 0, 0, &mut f);
    t.paint([0, 0, 200, 100], &mut f).unwrap();
    let r = t.nodes[&a].image_rect;
    assert_ne!(r, [0; 4]);
    f.messages.clear();
    t.mouse_button(r[0] + 3, r[1] + 3, true, &mut tree_view::ClickState::default(), &mut f)
        .unwrap();
    assert_eq!(t.selection, [a]);
    assert_eq!(
        f.messages.iter().map(|(_, m)| m.words).collect::<Vec<_>>(),
        [
            [tree_view::SELECTION as i32, 0, 0, 0],
            [0x1af5a13e, a, 0, 0],
            [0x1af5a13f, a, 0, 0]
        ]
    );
    f.access_calls.clear();
    f.draws.clear();
    f.fail_mode = 0x8001;
    t.draw_item(a, [10, 16, 40, 32], [0; 4], [0; 4], &mut f).unwrap();
    assert!(f.access_calls.contains(&(0x8001, true)));
    assert!(!f.access_calls.contains(&(0x8001, false)));
    assert!(!f.draws.iter().any(|d| matches!(d, Draw::Fill { .. })));
    assert!(f.draws.iter().any(|d| matches!(d, Draw::Text { .. })));
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

#[test]
fn tree_image_output_adapter_preserves_failed_outputs_and_borrows_null_slots() {
    let (mut t, mut f, _, _) = tree();
    assert!(t.set_image(0, Some(80), [2, 3, 8, 11], &mut f));
    assert!(t.set_image(3, None, [4, 5, 6, 7], &mut f));
    t.layout_dirty = false;
    let before = (f.refs.len(), f.invalidations, f.messages.len());
    let mut buffer = Some(999);
    let mut rect = [9; 4];
    for index in [4, u32::MAX] {
        assert!(!t.image_into(index, &mut buffer, &mut rect));
        assert_eq!((buffer, rect), (Some(999), [9; 4]));
    }
    assert!(t.image_into(0, &mut buffer, &mut rect));
    assert_eq!((buffer, rect), (Some(80), [2, 3, 8, 11]));
    assert!(t.image_into(3, &mut buffer, &mut rect));
    assert_eq!((buffer, rect), (None, [4, 5, 6, 7]));
    assert!(!t.layout_dirty);
    assert_eq!((f.refs.len(), f.invalidations, f.messages.len()), before);
}

#[test]
fn wc_t18_t50_successful_branch_geometry_is_cleared_after_failed_access() {
    let (mut t, mut f, a, _) = tree();
    t.flags |= 4;
    t.insert(descriptor(a, b"child"), &mut f).unwrap();
    t.set_image(0, Some(80), [0, 0, 6, 8], &mut f);
    t.set_node_image(a, 0, 0, &mut f);
    let area = [0, 0, 200, 100];
    t.paint(area, &mut f).unwrap();
    assert_ne!(t.nodes[&a].image_rect, [0; 4]);
    assert_ne!(t.nodes[&a].expander_rect, [0; 4]);
    let text = t.nodes[&a].text_rect;
    f.fail_mode = 0x8010;
    t.paint(area, &mut f).unwrap();
    assert_eq!(t.nodes[&a].image_rect, [0; 4]);
    assert_eq!(t.nodes[&a].text_rect, text);
    f.fail_mode = 0x8008;
    f.draws.clear();
    f.access_calls.clear();
    t.paint(area, &mut f).unwrap();
    assert_eq!(t.nodes[&a].expander_rect, [0; 4]);
    assert_eq!(t.nodes[&a].text_rect, text);
    assert!(!f.access_calls.contains(&(0x8008, false)));
    assert!(f.draws.iter().any(|draw| matches!(draw, Draw::Line { .. })));
}
