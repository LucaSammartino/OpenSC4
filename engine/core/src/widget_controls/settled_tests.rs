// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn wc_l_residual_outputs_storage_and_bounded_bar_coordinates() {
    let (mut l, mut f) = list(true);
    l.drop_resize = [false; 2];
    l.area = [0, 0, 80, 42];
    l.bar_width = 12;
    l.bar = Some(90);
    l.captured = true;
    l.mouse_move(80, 1, true, &mut f).unwrap();
    assert!(f.bar_events.is_empty()); // half-open list bound
    l.mouse_move(67, 1, true, &mut f).unwrap();
    assert!(f.bar_events.is_empty()); // inside list, outside bar
    f.dispatch_capture = Some(90);
    l.mouse_move(70, 1, false, &mut f).unwrap();
    assert_eq!(f.bar_events, [(90, false, 2, 1)]);
    assert!(l.bar_tracking);
    l.mouse_up_at(-10, -2, &mut f).unwrap();
    assert_eq!(f.bar_events[1], (90, true, -78, -2));
    assert!(!l.bar_tracking);
    let mut output = b"untouched".to_vec();
    assert!(!l.copy_text(-1, &mut output));
    assert_eq!(output, b"untouched");
    assert!(l.copy_text(0, &mut output));
    assert_eq!(output, b"row7");
    let mut index = 99;
    assert!(l.search_with(&mut index, |text, flag| {
        assert_eq!(flag, 1);
        text == b"row9"
    }));
    assert_eq!(index, 2);
    assert!(!l.search_with(&mut index, |_, _| false));
    assert_eq!(index, 2);
    let mut colors = [42; 2];
    assert!(!l.outline_colors(&mut colors, &mut f));
    assert_eq!(colors, [42; 2]);
    l.set_outline_colors([7, 8], &mut f);
    assert!(l.outline_colors(&mut colors, &mut f));
    assert_eq!(colors, [7, 8]);
    let old = l.entries.clone();
    let count = l.stored_count;
    assert!(l.insert_storage(-3, Entry { text: vec![], id: 99 }));
    assert_eq!(l.entries, old);
    assert_eq!(l.stored_count, count + 1);
    assert!(l.remove_storage(-3));
    assert_eq!(l.stored_count, count);
    assert_eq!(l.entries, old);
}

#[test]
fn wc_l36_timer_is_destroyed_before_retained_resources() {
    let (mut l, mut f) = list(false);
    let timer = l.timer.unwrap();
    l.border = Some(40);
    l.background = Some(41);
    l.services = [Some(42), Some(43)];
    f.refs.clear();
    l.destroy(&mut f);
    assert_eq!(
        f.refs,
        [
            (false, timer),
            (false, 40),
            (false, 41),
            (false, 100),
            (false, 42),
            (false, 43)
        ]
    );
    assert!(l.entries.is_empty());
}

#[test]
fn wc_t18_centered_images_and_packed_drag_preserve_hit_geometry() {
    let (mut t, mut f, a, b) = tree();
    t.flags |= 4;
    t.image_width = 20;
    t.set_image(0, Some(80), [2, 3, 8, 11], &mut f);
    t.nodes.get_mut(&a).unwrap().images = [0, -1, 0, -1];
    t.draw_item(a, [30, 16, 60, 32], [10, 16, 30, 32], [0; 4], &mut f)
        .unwrap();
    assert_eq!(t.nodes[&a].image_rect, [17, 20, 23, 28]);
    f.fail_mode = 0x8010;
    t.draw_item(a, [30, 16, 60, 32], [10, 16, 30, 32], [0; 4], &mut f)
        .unwrap();
    assert_eq!(t.nodes[&a].image_rect, [0; 4]);
    assert_eq!(t.nodes[&a].text_rect, [30, 16, 60, 32]);
    f.fail_mode = 0;
    t.select(a, true, &mut f);
    let saved = t.nodes[&a].clone();
    f.drag_return = true;
    f.drag_accept = 1;
    f.drag_target = Some(b);
    t.mouse_move(-1, -1, 0x10, &mut f).unwrap();
    assert_eq!(t.nodes[&a].text_rect, saved.text_rect);
    assert_eq!(t.nodes[&a].image_rect, saved.image_rect);
    assert_eq!(t.nodes[&a].expander_rect, saved.expander_rect);
    assert_eq!(f.drag_runs, 1);
}

#[test]
fn wc_l16_l22_removal_and_forward_pages() {
    let mut f = Recording::default();
    let mut l = list_box::ListBox::new(1, false, &mut f);
    for n in 0..20 {
        l.insert(-2, vec![b'A' + n as u8], n, &mut f);
    }
    l.whole_rows = 4;
    l.selected = 7;
    l.first = 0;
    assert_eq!(l.next_page(&mut f), Ok(true));
    assert_eq!((l.selected, l.first), (7, 3));
    l.selected = 0;
    l.first = 0;
    l.next_page(&mut f).unwrap();
    assert_eq!((l.selected, l.first), (7, 4));
    l.selected = 5;
    l.remove(5, &mut f).unwrap();
    assert_eq!(l.selected, 5); // selected successor retained, no selection call
    l.remove(0, &mut f).unwrap();
    assert_eq!(l.selected, 4);
    l.clear(&mut f);
    for n in 0..10 {
        l.insert(-2, vec![], n, &mut f);
    }
    l.whole_rows = 4;
    l.first = 5;
    l.next_page(&mut f).unwrap();
    assert_eq!((l.selected, l.first), (9, 6));
    l.next_page(&mut f).unwrap();
    assert_eq!(l.first, 6);
}

#[test]
fn wc_l23_l30_sizing_sort_search_and_malformed_storage() {
    let mut f = Recording {
        parent_limit: Some(102),
        ..Recording::default()
    };
    let mut l = list_box::ListBox::new(1, true, &mut f);
    l.drop_resize = [false, false];
    l.area = [0, 0, 30, 22];
    l.row_height = 10;
    l.whole_rows = 2;
    l.y_gutter = 1;
    for n in 0..5 {
        l.insert(-2, vec![b'a' + n as u8], n, &mut f);
    }
    assert!(l.size_drop_height(&mut f));
    assert_eq!((l.height(), l.whole_rows), (52, 5));
    assert_eq!(l.first, 0);
    assert!(l.bar.is_none());
    assert_eq!(f.area_requests, [(1, [0, 0, 30, 52])]);
    l.clear(&mut f);
    l.x_gutter = 3;
    assert!(l.size_drop_width(&mut f));
    assert_eq!(l.width(), 7);
    assert_eq!(f.area_requests.last(), Some(&(1, [0, 0, 7, 52])));
    let old = l.height();
    assert!(!l.size_drop_height(&mut f));
    assert_eq!(l.height(), old);
    assert_eq!(l.drop_resize, [true, true]);
    l.drop_mode = false;
    for t in [b"Alpha".as_slice(), b"alpha", b"B"] {
        l.insert(-2, t.to_vec(), 0, &mut f);
    }
    assert_eq!(l.find_text(b"alpha", &mut f), Some(1));
    assert!(l.select_prefix(b"A", &mut f));
    assert_eq!(l.selected, 0);
    l.enable_sorting(&mut f);
    l.entries.reverse();
    let saved = l.entries.clone();
    l.enable_sorting(&mut f);
    assert_eq!(l.entries, saved);
    assert_eq!(l.remove(-3, &mut f), Ok(true));
    assert_eq!((l.entries.len(), l.stored_count), (3, 2));
    assert_eq!(l.remove(-2, &mut f), Ok(true));
    assert_eq!(l.entries.len(), 2);
    l.clear(&mut f);
    assert_eq!(l.remove(-1, &mut f), Ok(true));
}

#[test]
fn wc_l26_l31_l34_paint_prefixes_intensity_and_fitting() {
    let mut f = Recording::default();
    let mut l = list_box::ListBox::new(1, false, &mut f);
    l.font = Some(7);
    l.area = [0, 0, 100, 200];
    l.row_height = 10;
    l.y_gutter = 0;
    let texts = [
        b"abc".as_slice(),
        b"##abc",
        b"###abc",
        b"#112233abc",
        b"#bad",
        b"##bad",
        b"#123456A",
        b"#12xxxxB",
    ];
    for t in texts {
        l.insert(-2, t.to_vec(), 0, &mut f);
    }
    let stored = l.entries.clone();
    l.paint(&mut f).unwrap();
    let painted: Vec<_> = f
        .draws
        .iter()
        .filter_map(|d| {
            if let Draw::FittedText { text, color, flags, .. } = d {
                assert_eq!(*flags, 0x08000000);
                Some((text.clone(), *color))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        painted.iter().map(|p| p.0.as_slice()).collect::<Vec<_>>(),
        [b"abc".as_slice(), b"#abc", b"##abc", b"abc", b"#bad", b"A", b"B"]
    );
    assert_eq!(painted[5].1, 0x123456);
    assert_eq!(painted[6].1, 0x123456);
    assert_eq!(l.entries, stored);
    for value in [f32::NEG_INFINITY, 0.5, f32::INFINITY, f32::NAN] {
        l.set_intensity(value, &mut f);
        if value.is_nan() {
            assert!(l.intensity.is_nan());
        } else {
            assert_eq!(l.intensity, value.clamp(0.0, 1.0));
        }
    }
    assert_eq!(l.entries[3].text, b"#112233abc");
}

#[test]
fn wc_l24_l34_l35_l36_clipping_fitting_capture_and_lifetime() {
    let mut f = Recording {
        parent_limit: Some(39),
        ..Recording::default()
    };
    let mut l = ListBox::new(1, true, &mut f);
    l.drop_resize = [false, false];
    l.area = [0, 0, 80, 22];
    l.bar_width = 12;
    l.whole_rows = 2;
    l.row_height = 10;
    l.y_gutter = 1;
    for i in 0..5 {
        l.insert(-2, b"abcdef".to_vec(), i, &mut f);
    }
    f.area_requests.clear();
    l.size_drop_height(&mut f);
    assert_eq!((l.height(), l.whole_rows), (32, 3));
    assert_eq!(f.area_requests, [(1, [0, 0, 80, 37]), (1, [0, 0, 80, 32])]);
    assert!(l.bar.is_some());
    l.font = Some(7);
    f.fit_counts.push_back(3);
    f.draws.clear();
    l.paint(&mut f).unwrap();
    assert!(f
        .draws
        .iter()
        .any(|d| matches!(d,Draw::FittedText{text,count:3,flags:0x08000000,..}if text==b"abcdef")));
    let bar = l.bar.unwrap();
    f.dispatch_capture = Some(bar);
    l.captured = true;
    assert_eq!(l.mouse_move(79, 1, true, &mut f), Ok(true));
    assert!(l.bar_tracking);
    assert_eq!(f.capture_owner, Some(1));
    assert_eq!(l.mouse_up(&mut f), Ok(true));
    assert!(!l.bar_tracking);
    l.background = Some(8);
    l.border = Some(9);
    l.services = [Some(10), Some(11)];
    f.refs.clear();
    l.shutdown(&mut f);
    let refs = f.refs.clone();
    l.shutdown(&mut f);
    assert_eq!(f.refs, refs);
    assert_eq!(l.entries.len(), 5);
    assert!(l.timer.is_some());
    l.destroy(&mut f);
    assert!(l.entries.is_empty());
    assert!(l.timer.is_none());
}

#[test]
fn wc_t13_t16_flatten_anchor_and_viewport() {
    let (mut t, mut f, a, b) = tree();
    let c = t.insert(descriptor(a, b"C"), &mut f).unwrap();
    t.set_expanded(a, true, &mut f);
    t.set_expanded(0, false, &mut f);
    t.flags &= !2;
    t.recalculate();
    assert_eq!(t.visible, [a, c, b]);
    assert!(!t.nodes[&0].flattened);
    assert_eq!((t.depth(a), t.depth(b)), (0, 0));
    t.flags |= 2;
    t.recalculate();
    assert_eq!(t.visible, [0]);
    t.set_expanded(0, true, &mut f);
    t.recalculate();
    assert_eq!(t.visible, [0, a, c, b]);
    assert_eq!((t.depth(a), t.depth(b)), (0, 0));
    t.flags |= 0x10;
    t.select(a, false, &mut f);
    t.select(b, false, &mut f);
    t.key_down(0x26, &mut f).unwrap();
    assert_eq!(t.selection, [c]);
    t.set_margins(0, 0, &mut f);
    t.layout_viewport([0, 0, 100, 33], [100, 100], [0, 0]).unwrap();
    assert_eq!((t.viewport_rows, t.page_count), (2, 3));
    assert_eq!(t.first + t.page_count as i32 - 1, 2); // WC-T16 last paint row
}

#[test]
fn wc_t19_t26_properties_order_descriptors_and_restart() {
    let (mut t, mut f, a, b) = tree();
    t.raw_style[4] = 1;
    let n = f.invalidations;
    t.set_style_bits(3, true, &mut f);
    assert_eq!(t.raw_style[4], 1);
    assert_eq!(f.invalidations, n);
    t.set_expander_size(14, &mut f);
    assert_eq!(t.raw_style[0], 11);
    t.set_text(a, b"B".to_vec(), &mut f);
    t.set_text(b, b"D".to_vec(), &mut f);
    let mut d = descriptor(0, b"C");
    d.reference = -4;
    let older = t.insert(d.clone(), &mut f).unwrap();
    let newest = t.insert(d, &mut f).unwrap();
    assert_eq!(t.nodes[&0].children, [a, newest, older, b]);
    let mut d = descriptor(a, b"foreign reference");
    d.reference = b;
    let foreign = t.insert(d, &mut f).unwrap();
    assert_eq!(t.nodes[&a].children, [foreign]);
    assert_eq!(t.nodes[&foreign].reference, b);
    let before = t.nodes.len();
    let next = t.next_handle;
    let invalidations = f.invalidations;
    t.layout_dirty = false;
    f.font_height = 80;
    let mut d = descriptor(0, b"leaked");
    d.reference = 999;
    assert_eq!(t.insert(d, &mut f), Ok(-1));
    assert_eq!(t.next_handle, next + 1);
    assert_eq!(t.nodes.len(), before);
    assert_eq!(t.row_height, 80);
    assert!(!t.layout_dirty);
    assert_eq!(f.invalidations, invalidations);
    assert_eq!(t.abandoned.len(), 1);
    let mut value = t.descriptor(a).unwrap();
    assert_eq!(value.children, [foreign]);
    value.force_branch = true;
    value.visible = false;
    value.children = vec![999];
    let topology = t.nodes[&a].children.clone();
    t.apply_descriptor(a, value, &mut f);
    assert!(!t.nodes[&a].force_branch);
    assert_eq!(t.nodes[&a].children, topology);
    assert!(t.get_visible(a)); // WC-T25: visible=false does not hide the node
    let root = t.root;
    let next = t.next_handle;
    t.shutdown(&mut f);
    let releases = f.refs.len();
    t.shutdown(&mut f);
    assert_eq!(f.refs.len(), releases);
    assert_eq!(t.root, root);
    assert_eq!(t.next_handle, next);
    assert!(t.internal_initialized);
    assert!(t.default_font.is_none());
}

#[test]
fn wc_t27_t29_t36_image_retention_metrics_and_failure() {
    let (mut t, mut f, _, _) = tree();
    f.refs.clear();
    let invalidations = f.invalidations;
    t.set_image(0, Some(7), [0, 0, 5, 12], &mut f);
    f.refs.clear();
    t.set_image(0, Some(7), [0, 0, 5, 12], &mut f);
    assert_eq!(f.refs, [(true, 7), (false, 7)]);
    assert_eq!(t.image(0), Some((Some(7), [0, 0, 5, 12])));
    assert_eq!(f.invalidations, invalidations);
    t.row_height = 12;
    for (height, want) in [(12, 12), (13, 15), (2, 15)] {
        t.append_image(Some(8), [0, 0, 5, height], &mut f);
        assert_eq!(t.row_height, want);
    }
    assert_eq!(t.images.len(), 7);
    f.refs.clear();
    let metrics = (t.row_height, t.image_width);
    t.truncate_images(&mut f);
    assert_eq!(f.refs, [(false, 8); 3]);
    assert_eq!(t.images.len(), 4);
    assert_eq!((t.row_height, t.image_width), metrics);
    let mut t = tree_view::TreeView::new(1);
    f.fail_image_slot = Some(0);
    f.image_attempts.clear();
    assert!(t.init(&mut f)); // WC-T36: no aggregate image-init failure
    assert_eq!(f.image_attempts, [0, 1, 2, 3]);
    assert!(t.images.iter().all(Option::is_some));
}

#[test]
fn wc_t30_t35_move_remove_and_adapters() {
    let (mut t, mut f, a, b) = tree();
    let c = t.insert(descriptor(0, b"C"), &mut f).unwrap();
    t.move_node(a, 0, b, &mut f);
    assert_eq!(t.nodes[&0].children, [c, b, a]); // C prepended by descriptor
    t.move_node(a, 0, 999, &mut f);
    assert_eq!(t.nodes[&0].children, [a, c, b]);
    assert_eq!(t.text(999), b"");
    let n = f.invalidations;
    assert_eq!(t.node_image(a, 4), -1);
    assert!(!t.set_node_image(a, 4, 1, &mut f));
    assert_eq!(f.invalidations, n);
    t.set_node_color(a, 1, 99, &mut f);
    assert_eq!(t.descriptor(a).unwrap().colors[2], 99);
    assert!(!t.children(a, None));
    let mut out = vec![999];
    assert!(t.children(a, Some(&mut out)));
    assert!(out.is_empty());
    t.flags |= 0x10;
    t.select(a, false, &mut f);
    t.select(b, false, &mut f);
    assert_eq!(t.first_selected(), a);
    assert_eq!(t.selected_handles(), [a, b]);
    let child = t.insert(descriptor(a, b"child"), &mut f).unwrap();
    t.select(child, false, &mut f);
    f.messages.clear();
    t.remove_node(0, &mut f);
    assert_eq!(t.nodes.len(), 1);
    assert_eq!(t.nodes[&0].children, []);
    assert!(t.selection.is_empty());
    assert_eq!(
        f.messages
            .iter()
            .filter(|(_, m)| m.words[0] as u32 == tree_view::SELECTION)
            .count(),
        3
    );
}

#[test]
fn wc_t17_t18_t48_t51_geometry_fills_and_guarded_strokes() {
    let (mut t, mut f, a, _) = tree();
    let child = t.insert(descriptor(a, b"xx"), &mut f).unwrap();
    t.flags &= !4;
    t.image_width = 12;
    t.set_expanded(a, true, &mut f);
    t.first = 1;
    t.paint([0, 0, 200, 100], &mut f).unwrap();
    assert_eq!(t.depth(child), 1);
    assert_eq!(t.nodes[&child].text_rect, [39, 16, 58, 32]);
    t.select(a, true, &mut f);
    t.colors[1] = 0;
    t.colors[3] = 0xffffff;
    t.insertion_side = true;
    f.draws.clear();
    t.draw_item(a, [10, 16, 40, 32], [0; 4], [0; 4], &mut f).unwrap();
    assert!(f.draws.contains(&Draw::Fill {
        rect: [10, 16, 40, 32],
        color: 0x7f7f7f
    }));
    assert!(f.draws.iter().any(|d| matches!(
        d,
        Draw::Fill {
            rect: [0, 16, 4096, 20],
            color: 0xffffff
        }
    )));
    f.draws.clear();
    f.fail_mode = 0x8001;
    t.draw_item(a, [10, 16, 40, 32], [0; 4], [0; 4], &mut f).unwrap();
    assert!(!f.draws.iter().any(|d| matches!(d, Draw::Fill { .. })));
    assert!(f.draws.iter().any(|d| matches!(d, Draw::Text { .. })));
    f.fail_mode = 0x8008;
    t.paint([0, 0, 200, 100], &mut f).unwrap();
    assert_eq!(t.nodes[&a].expander_rect, [0; 4]);
    assert!(f.draws.iter().any(|d| matches!(d, Draw::Line { .. })));
    f.fail_mode = 0;
    t.set_expanded(a, false, &mut f);
    f.draws.clear();
    t.paint([0, 0, 200, 100], &mut f).unwrap();
    let r = t.nodes[&a].expander_rect;
    let center = r[1] + 4;
    assert!(f.draws.contains(&Draw::Line {
        points: [r[0] + 2, center, r[0] + 6, center],
        color: 0,
        style: 1
    }));
    assert!(f.draws.contains(&Draw::Line {
        points: [r[0] + 4, r[1] + 2, r[0] + 4, r[1] + 6],
        color: 0,
        style: 1
    }));
}

#[test]
fn wc_t37_t38_cached_page_and_empty_navigation() {
    let (mut t, mut f, _, _) = tree();
    t.remove_node(0, &mut f);
    for n in 0..20 {
        let mut d = descriptor(0, &[n]);
        d.reference = -3;
        t.insert(d, &mut f).unwrap();
    }
    t.recalculate();
    t.page_count = 4;
    let anchor = t.visible[7];
    t.select(anchor, true, &mut f);
    t.key_down(0x22, &mut f).unwrap();
    assert_eq!(t.selection, [t.visible[11]]);
    t.key_down(0x21, &mut f).unwrap();
    assert_eq!(t.selection, [anchor]);
    let mut t = tree_view::TreeView::new(1);
    assert!(matches!(t.key_down(0x24, &mut f), Err(Unsupported::Precondition(_))));
    assert_eq!(t.key_down(13, &mut f), Ok(true));
    assert_eq!(t.key_down(27, &mut f), Ok(true));
}

#[test]
fn wc_t39_t43_mouse_protocol_shared_clicks_and_collapse_repair() {
    let (mut t, mut f, a, _) = tree();
    t.paint([0, 0, 200, 100], &mut f).unwrap();
    let mut shared = tree_view::ClickState::default();
    let r = t.nodes[&a].text_rect;
    let (x, y) = (r[0] + 3, r[1] + 4);
    f.interval = 500;
    f.control = true;
    t.flags |= 0x10;
    t.select(a, true, &mut f);
    t.mouse_button(x, y, false, &mut shared, &mut f).unwrap();
    assert!(t.selection.is_empty());
    t.flags &= !0x10;
    f.elapsed = 501;
    t.mouse_button(x, y, false, &mut shared, &mut f).unwrap();
    assert_eq!(t.selection, [a]);
    f.messages.clear();
    t.clear_selection(&mut f);
    f.messages.clear();
    t.mouse_button(x, y, true, &mut shared, &mut f).unwrap();
    assert_eq!(
        f.messages.iter().map(|(_, m)| m.words[0] as u32).collect::<Vec<_>>(),
        [tree_view::SELECTION, 0x1af5a13e, 0x1af5a13f]
    );
    let (mut other, mut g, other_a, _) = tree();
    other.paint([0, 0, 200, 100], &mut g).unwrap();
    g.interval = 500;
    g.elapsed = 502;
    other.mouse_button(x, y, false, &mut shared, &mut g).unwrap();
    assert!(g
        .messages
        .iter()
        .any(|(_, m)| m.words[0] as u32 == tree_view::ACTIVATION));
    assert_eq!(a, other_a);
    g.messages.clear();
    g.elapsed = 1002;
    other.mouse_button(x, y, false, &mut shared, &mut g).unwrap();
    assert!(!g
        .messages
        .iter()
        .any(|(_, m)| m.words[0] as u32 == tree_view::ACTIVATION));
    let c = t.insert(descriptor(a, b"child"), &mut f).unwrap();
    t.set_expanded(a, true, &mut f);
    t.select(c, true, &mut f);
    t.flags |= 0x10;
    t.select(a, false, &mut f);
    t.key_down(13, &mut f).unwrap();
    assert!(!t.nodes[&c].selected);
    assert!(t.selection.contains(&c));
    f.messages.clear();
    t.paint([0, 0, 200, 100], &mut f).unwrap();
    assert!(!t.selection.contains(&c));
    assert_eq!(
        f.messages
            .iter()
            .filter(|(_, m)| m.words[0] as u32 == tree_view::SELECTION)
            .count(),
        1
    );
}

#[test]
fn wc_t44_t47_drag_refusal_side_rejection_and_drop() {
    let (mut t, mut f, a, b) = tree();
    let c = t.insert(descriptor(a, b"child"), &mut f).unwrap();
    t.set_expanded(a, true, &mut f);
    t.paint([0, 0, 200, 100], &mut f).unwrap();
    t.select(a, true, &mut f);
    f.drag_return = true;
    f.drag_accept = 0;
    t.mouse_move(199, 90, 0x10, &mut f).unwrap();
    assert_eq!(f.drag_runs, 0);
    assert_eq!(t.selection, [a]);
    t.notify_drag(10, 3, &[a], &mut f).unwrap();
    assert!(t.insertion_side);
    t.notify_drag(10, 7, &[a], &mut f).unwrap();
    assert!(!t.insertion_side);
    let row = t.nodes[&c].visible_index.unwrap() as i32;
    let y = row * 16 + 3;
    t.notify_drag(10, y, &[a], &mut f).unwrap();
    assert!(t.selection.is_empty());
    t.select(a, true, &mut f);
    f.drag_accept = 1;
    f.drag_target = Some(b);
    f.messages.clear();
    let topology = t.nodes[&0].children.clone();
    t.mouse_move(199, 90, 0x10, &mut f).unwrap();
    assert_eq!(f.drag_runs, 1);
    assert_eq!(t.nodes[&0].children, topology);
    assert!(t.selection.contains(&a));
    assert!(f.messages.iter().any(|(_, m)| m.words[0] as u32 == 0x1af5a141));
    assert!(!t.insertion_side);
}
