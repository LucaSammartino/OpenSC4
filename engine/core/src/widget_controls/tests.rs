// SPDX-License-Identifier: GPL-3.0-or-later
use super::list_box::*;
use super::*;

#[derive(Default)]
struct Recording {
    refs: Vec<(bool, Handle)>,
    messages: Vec<(Delivery, Message)>,
    invalidations: usize,
    draws: Vec<Draw>,
    font_height: i32,
    buffer_height: i32,
    elapsed: u32,
    interval: u32,
    bar: i32,
    defaults: Vec<u32>,
    base_messages: Vec<Message>,
}

impl Foreign for Recording {
    fn add_ref(&mut self, h: Handle) {
        self.refs.push((true, h));
    }
    fn release(&mut self, h: Handle) {
        self.refs.push((false, h));
    }
    fn base_init(&mut self, _: Handle) -> bool {
        false
    }
    fn base_shutdown(&mut self, _: Handle) {}
    fn base_query(&mut self, _: Handle, _: u32, _: &mut Option<Handle>) -> bool {
        false
    }
    fn base_message(&mut self, _: Handle, m: Message) -> bool {
        self.base_messages.push(m);
        false
    }
    fn invalidate(&mut self, _: Handle) {
        self.invalidations += 1;
    }
    fn show(&mut self, _: Handle, _: bool) {}
    fn capture(&mut self, _: Handle, _: bool) {}
    fn convert_payload(&mut self, _: Handle, value: i32) -> i32 {
        value
    }
    fn deliver(&mut self, _: Handle, d: Delivery, m: Message) -> bool {
        self.messages.push((d, m));
        false
    }
    fn drop_complete(&mut self, _: Handle, kind: u32, row: i32) {
        self.messages.push((
            Delivery::Send,
            Message {
                kind,
                words: [row, 0, 0, 0],
            },
        ));
    }
    fn default_interface(&mut self, key: u32) -> Option<Handle> {
        self.defaults.push(key);
        Some(100)
    }
    fn default_color(&mut self, key: u32) -> Option<u32> {
        self.defaults.push(key);
        Some(0)
    }
    fn create(&mut self, _: Resource) -> Option<Handle> {
        Some(200)
    }
    fn font_height(&mut self, _: Handle) -> i32 {
        self.font_height
    }
    fn buffer_height(&mut self, _: Handle) -> i32 {
        self.buffer_height
    }
    fn bar_value(&mut self, _: Handle) -> i32 {
        self.bar
    }
    fn bar_range(&mut self, _: Handle, _: i32, _: i32, value: i32) {
        self.bar = value;
    }
    fn place_bar(&mut self, _: Handle, _: Rect) {}
    fn create_bar(&mut self, _: Handle) -> Option<Handle> {
        Some(300)
    }
    fn destroy_bar(&mut self, _: Handle) {}
    fn timer_elapsed(&mut self, _: Handle) -> u32 {
        self.elapsed
    }
    fn restart_timer(&mut self, _: Handle) {
        self.elapsed = 0;
    }
    fn double_click_interval(&mut self) -> u32 {
        self.interval
    }
    fn begins_with(&mut self, text: &[u8], prefix: &[u8], arg: u32) -> bool {
        assert_eq!(arg, 1);
        text.starts_with(prefix)
    }
    fn text_equal(&mut self, left: &[u8], right: &[u8]) -> bool {
        left == right
    }
    fn draw(&mut self, draw: Draw) {
        self.draws.push(draw);
    }
}

fn list(drop_mode: bool) -> (ListBox, Recording) {
    let mut f = Recording {
        font_height: 10,
        interval: 500,
        ..Recording::default()
    };
    let mut list = ListBox::new(1000, drop_mode, &mut f);
    list.area = [10, 20, 110, 62];
    list.font = Some(100);
    list.recompute_metrics(&mut f);
    for id in [7, 7, 9] {
        list.insert(-2, format!("row{id}").into_bytes(), id, &mut f);
    }
    (list, f)
}

#[test]
fn wc_l01_query_views_and_null_retention() {
    let (list, mut f) = list(false);
    for _view in [list.window, list.window + LIST_OFFSET] {
        let mut output = None;
        assert!(list.query_interface(IID, Some(&mut output), &mut f));
        assert_eq!(output, Some(list.window + LIST_OFFSET));
        f.release(list.window);
    }
    assert!(!list.query_interface(IID, None, &mut f));
    assert_eq!(f.refs, [(true, 1000), (false, 1000), (true, 1000), (false, 1000)]);
    assert_eq!((LIST_SLOTS, WINDOW_SLOTS), (66, 169));
}

#[test]
fn wc_l02_l06_selection_indices_ids_and_insertions() {
    for drop_mode in [false, true] {
        let (mut list, mut f) = list(drop_mode);
        list.select(1, false, &mut f);
        for _ in 0..2 {
            assert!(list.select(99, true, &mut f));
        }
        assert_eq!(list.selected, 2);
        assert_eq!(f.messages.len(), if drop_mode { 0 } else { 2 });
        for (_, m) in &f.messages {
            assert_eq!(
                *m,
                Message {
                    kind: 3,
                    words: [SELECTION as i32, 2, 0, 0]
                }
            );
        }
        assert!(list.select_id(7, false, &mut f));
        assert_eq!(list.selected, 0);
        let invalidations = f.invalidations;
        assert!(!list.select_id(99, true, &mut f));
        assert_eq!(f.invalidations, invalidations);
        assert_eq!(list.selected_id(), 7);
        list.select(1, false, &mut f);
        list.insert(0, b"new".to_vec(), 20, &mut f);
        assert_eq!(list.selected, 2);
        assert!(list.replace_text(-2, b"last".to_vec(), &mut f));
        assert_eq!(list.text(3), Some(b"last".as_slice()));
    }
    let mut f = Recording::default();
    let mut empty = ListBox::new(1, false, &mut f);
    empty.select(-1, true, &mut f);
    assert_eq!((empty.selected, empty.selected_id()), (-1, 0));
    assert!(empty.entries.is_empty());
}

#[test]
fn wc_l07_l09_signed_metrics_hit_geometry() {
    let (mut list, mut f) = list(false);
    assert_eq!((list.row_height, list.whole_rows), (10, 4));
    assert!(list.resize_whole_lines(&mut f));
    assert_eq!(list.height(), 42);
    for n in 3..8 {
        list.insert(-2, vec![], n, &mut f);
    }
    list.first = 2;
    assert_eq!(list.hit_test(-999, 10, false), Ok(2));
    assert_eq!(list.hit_test(999, 11, false), Ok(3));
    assert_eq!(list.hit_test(0, -30, false), Ok(-1));
    list.y_gutter = -1;
    list.recompute_metrics(&mut f);
    assert_eq!(list.whole_rows, 4);
    list.font = None;
    list.recompute_metrics(&mut f);
    assert_eq!((list.row_height, list.whole_rows), (0, 0));
    assert!(!list.resize_whole_lines(&mut f));
    assert!(matches!(list.hit_test(0, 1, false), Err(Unsupported::Precondition(_))));
}

#[test]
fn wc_l10_l13_color_and_reference_quirks() {
    let (mut list, mut f) = list(false);
    let n = f.invalidations;
    assert!(list.set_font_color(1, 17, &mut f));
    assert_eq!(f.invalidations, n);
    assert!(list.set_font_color(1, 17, &mut f));
    assert_eq!(f.invalidations, n + 1);
    assert!(!list.set_font_color(3, 17, &mut f));
    let mut output = 99;
    assert!(!list.font_color(0, &mut output));
    assert_eq!(output, 99);
    assert!(list.font_color(1, &mut output));
    assert_eq!(output, 17);
    list.background = Some(101);
    list.border = Some(102);
    list.set_font(Some(100), &mut f);
    list.set_background(Some(101), &mut f);
    list.set_border(Some(102), &mut f);
    list.set_font(None, &mut f);
    list.set_background(None, &mut f);
    list.set_border(None, &mut f);
    assert_eq!(
        f.refs,
        [
            (true, 100),
            (false, 100),
            (true, 101),
            (false, 101),
            (true, 102),
            (false, 102),
            (false, 100),
            (false, 101),
            (false, 102)
        ]
    );
    list.set_flat(true, &mut f);
    list.set_three_d(true, &mut f);
    assert!(!list.flat && list.three_d);
}

#[test]
fn wc_l14_l15_delivery_and_key_protocols() {
    let (mut list, mut f) = list(false);
    assert!(list.emit(SELECTION, &mut f));
    list.delivery = Delivery::Post;
    assert!(list.emit(SELECTION, &mut f));
    assert_eq!(f.messages[0].0, Delivery::Send);
    assert_eq!(f.messages[1].0, Delivery::Post);
    assert_eq!(f.messages[0].1, f.messages[1].1);
    assert!(!list.key_up(42, 17, &mut f));
    assert_eq!(
        f.messages.last().unwrap().1,
        Message {
            kind: 6,
            words: [42, 17, 0, 0]
        }
    );
    let count = f.messages.len();
    list.drop_mode = true;
    assert!(list.key_up(42, 17, &mut f));
    assert_eq!(f.messages.len(), count);
    assert_eq!(list.key_down(13, 0, &mut f), Ok(true));
    assert_eq!(f.messages.last().unwrap().1.kind, 15);
    assert_eq!(list.key_down(27, 0, &mut f), Ok(true));
    assert_eq!(f.messages.last().unwrap().1.words[0], -1);
}

#[test]
fn wc_f01_storage_removal_is_not_selection() {
    let (mut list, mut f) = list(false);
    list.select(1, false, &mut f);
    assert_eq!(list.entries.len(), 3);
    assert!(list.remove_storage(1));
    assert_eq!(list.entries.len(), 2);
    assert_eq!(list.selected, 1);
    assert_eq!(list.selected_id(), 9);
}

#[test]
fn list_mouse_timer_strictness_and_no_font_background() {
    let (mut list, mut f) = list(false);
    list.captured = true;
    list.previous_click = 0;
    f.elapsed = 500;
    list.mouse_down(0, 1, &mut f).unwrap();
    assert!(!list.pending_double_click);
    f.elapsed = 499;
    list.mouse_down(0, 1, &mut f).unwrap();
    assert!(list.pending_double_click);
    list.mouse_up(&mut f).unwrap();
    assert_eq!(f.messages.last().unwrap().1.words[0], ACTIVATION as i32);
    assert_eq!(list.previous_click, -1);
    list.font = None;
    list.paint(&mut f).unwrap();
    assert_eq!(f.draws.len(), 1);
    assert!(matches!(f.draws[0], Draw::Fill { .. }));
}

impl tree_view::TreeForeign for Recording {
    fn convert_color(&mut self, _: Handle, color: u32) -> u32 {
        color
    }
    fn initialize_buffer(&mut self, _: Handle) -> bool {
        true
    }
    fn buffer_access(&mut self, _: Handle, _: bool) -> bool {
        true
    }
    fn measure_text(&mut self, _: Handle, text: &[u8]) -> i32 {
        text.len() as i32 * 8
    }
    fn draw_image(&mut self, image: Handle, rect: Rect) {
        self.draws.push(Draw::Blit { source: image, rect });
    }
    fn set_bar_value(&mut self, _: Handle, value: i32) {
        self.bar = value;
    }
}

fn descriptor(parent: i32, text: &[u8]) -> tree_view::Descriptor {
    tree_view::Descriptor {
        parent,
        reference: -2,
        text: text.to_vec(),
        data: 0,
        images: [-1; 4],
        font: None,
        colors: [0, 1, 2, 3],
        expanded: false,
    }
}

fn tree() -> (tree_view::TreeView, Recording, i32, i32) {
    let mut f = Recording {
        font_height: 10,
        ..Recording::default()
    };
    let mut tree = tree_view::TreeView::new(1000);
    assert!(tree.init(&mut f));
    tree.viewport_rows = 4;
    let a = tree.insert_at(descriptor(0, b"A"), 0, &mut f);
    let b = tree.insert_at(descriptor(0, b"B"), 1, &mut f);
    f.messages.clear();
    (tree, f, a, b)
}

#[test]
fn wc_t01_t03_identity_initialization_and_expansion() {
    use tree_view::{COLLAPSE, EXPAND, IID, PRIMARY_SLOTS, WINDOW_SLOTS};
    let (mut tree, mut f, a, _) = tree();
    assert_eq!(tree.window(), tree.primary + 4);
    assert_eq!((PRIMARY_SLOTS, WINDOW_SLOTS), (60, 151));
    assert_eq!(tree.nodes.len(), 3);
    let requests = f.defaults.len();
    assert!(tree.init(&mut f));
    assert_eq!(f.defaults.len(), requests);
    assert_eq!(tree.root, Some(0));
    assert!(tree.nodes[&0].expanded && tree.nodes[&0].force_branch);
    assert!(!tree.nodes[&0].selected);
    let mut output = None;
    assert_eq!(tree.query_interface(IID, Some(&mut output), &mut f), Ok(true));
    assert_eq!(output, Some(tree.primary));
    assert!(matches!(
        tree.query_interface(IID, None, &mut f),
        Err(Unsupported::Precondition(_))
    ));
    assert!(tree.node(999).is_none());
    let n = f.invalidations;
    assert!(!tree.set_expanded(999, true, &mut f));
    assert_eq!(f.invalidations, n);
    assert!(tree.set_expanded(a, true, &mut f));
    assert!(tree.set_expanded(a, true, &mut f));
    assert!(tree.set_expanded(a, false, &mut f));
    assert_eq!(
        f.messages.iter().map(|(_, m)| m.words[0] as u32).collect::<Vec<_>>(),
        [EXPAND, COLLAPSE]
    );
}

#[test]
fn wc_t04_t06_selection_and_ancestor_visibility() {
    let (mut tree, mut f, a, b) = tree();
    assert!(tree.select(a, false, &mut f));
    assert!(tree.select(b, true, &mut f));
    assert!(!tree.deselect(999, &mut f));
    assert_eq!(tree.selection, [b]);
    assert!(!tree.nodes[&a].selected && tree.nodes[&b].selected);
    assert_eq!(f.messages.len(), 2);
    for (_, m) in &f.messages {
        assert_eq!(m.words[..2], [tree_view::SELECTION as i32, 0]);
    }
    tree.clear_selection(&mut f);
    f.messages.clear();
    tree.flags |= 0x10;
    tree.select(a, false, &mut f);
    tree.select(b, false, &mut f);
    assert_eq!(tree.selection, [a, b]);
    assert!(tree.clear_selection(&mut f));
    let n = f.messages.len();
    assert!(tree.clear_selection(&mut f));
    assert_eq!(f.messages.len(), n);
    let child = tree.insert_at(descriptor(a, b"child"), 0, &mut f);
    assert!(!tree.get_visible(child));
    f.messages.clear();
    assert!(tree.make_visible(child, &mut f));
    assert!(tree.nodes[&a].expanded);
    assert!(tree.get_visible(child));
    assert!(tree.selection.is_empty());
    assert_eq!(f.messages[0].1.words[..2], [tree_view::EXPAND as i32, a]);
}

#[test]
fn wc_t07_t09_wheel_sign_zero_and_missing_bar() {
    let (mut tree, mut f, _, _) = tree();
    for i in 0..7 {
        tree.insert_at(descriptor(0, &[b'C' + i]), i as usize + 2, &mut f);
    }
    tree.recalculate();
    assert_eq!(tree.visible.len(), 10);
    tree.first = 2;
    tree.vertical_bar = Some(300);
    assert!(tree.wheel(-1, &mut f));
    assert_eq!(f.bar, 3);
    assert!(tree.wheel(-1000, &mut f));
    assert_eq!(f.bar, 4);
    tree.first = 2;
    assert!(tree.wheel(0, &mut f));
    assert_eq!(f.bar, 1);
    tree.vertical_bar = None;
    assert!(tree.wheel(-100, &mut f));
    assert!(tree.wheel(100, &mut f));
    assert_eq!(f.bar, 1);
}

#[test]
fn wc_t10_activation_sequence_and_branch_toggle() {
    let (mut tree, mut f, a, b) = tree();
    tree.insert_at(descriptor(a, b"child"), 0, &mut f);
    tree.flags |= 0x10;
    tree.select(a, false, &mut f);
    tree.select(b, false, &mut f);
    f.messages.clear();
    assert_eq!(tree.key_down(13, &mut f), Ok(true));
    assert_eq!(
        f.messages
            .iter()
            .map(|(_, m)| (m.words[0] as u32, m.words[1]))
            .collect::<Vec<_>>(),
        [
            (tree_view::ACTIVATION, a),
            (tree_view::EXPAND, a),
            (tree_view::ACTIVATION, b),
        ]
    );
    assert_eq!(tree.selection, [a, b]);
    assert!(!tree.key_up(13, 0));
}

#[test]
fn wc_t11_bar_messages_always_delegate_to_base() {
    let (mut tree, mut f, _, _) = tree();
    tree.horizontal_bar = Some(301);
    tree.vertical_bar = Some(302);
    for (sub, value) in [(0x1756_bac7_u32, 17), (0x1756_bac6, 4), (0, 99)] {
        f.bar = value;
        let m = Message {
            kind: 3,
            words: [0x8871_13a3_u32 as i32, sub as i32, 0, 0],
        };
        assert_eq!(tree.do_message(m, &mut f), Ok(false));
        assert_eq!(*f.base_messages.last().unwrap(), m);
    }
    assert_eq!((tree.horizontal_position, tree.first), (17, 4));
    assert_eq!(f.base_messages.len(), 3);
}

#[test]
fn wc_t12_missing_image_does_not_suppress_text() {
    let (mut tree, mut f, a, _) = tree();
    assert_eq!(tree.nodes[&a].images, [-1; 4]);
    assert_eq!(
        tree.draw_item(a, [10, 0, 50, 16], [0, 0, 9, 16], [0; 4], &mut f),
        Ok(true)
    );
    assert!(!f.draws.iter().any(|draw| matches!(draw, Draw::Blit { .. })));
    assert!(f
        .draws
        .iter()
        .any(|draw| matches!(draw, Draw::Text { text, .. } if text == b"A")));
    assert_eq!(tree.nodes[&a].text_rect, [10, 0, 50, 16]);
}

#[test]
fn tree_font_growth_shutdown_and_open_ordering() {
    let (mut tree, mut f, a, _) = tree();
    f.font_height = 30;
    f.refs.clear();
    assert!(tree.set_font(a, Some(999), &mut f));
    assert_eq!(tree.row_height, 30);
    assert!(f.refs.is_empty());
    f.font_height = 5;
    tree.set_font(a, None, &mut f);
    assert_eq!(tree.row_height, 30);
    assert_eq!(tree.insert(descriptor(999, b"missing"), &mut f), Ok(-1));
    let count = tree.nodes.len();
    assert!(matches!(
        tree.insert(descriptor(0, b"open"), &mut f),
        Err(Unsupported::Open(_))
    ));
    assert_eq!(tree.nodes.len(), count);
    tree.images = vec![Some(400), None, Some(401)];
    tree.horizontal_bar = Some(301);
    tree.vertical_bar = Some(302);
    tree.shutdown(&mut f);
    assert!(tree.nodes.is_empty() && tree.visible.is_empty() && tree.selection.is_empty());
    assert_eq!(
        f.refs,
        [
            (false, 400),
            (false, 401),
            (false, 100),
            (false, 200),
            (false, 301),
            (false, 302)
        ]
    );
}
