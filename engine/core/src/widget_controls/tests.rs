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
    paths: Vec<Vec<u8>>,
    exists: bool,
    existing_checks: usize,
    create_success: bool,
    split_count: i32,
    parent_path: Vec<u8>,
    window_flags: Vec<(u32, bool)>,
    selections: Vec<(u32, file_browser::SelectionDescriptor, file_browser::Selection)>,
    editor_texts: Vec<Vec<u8>>,
    grid_texts: std::collections::BTreeMap<i32, Vec<u8>>,
    grid_last: u32,
    grid_reads: usize,
    removed: Vec<(Handle, Handle)>,
    button_updates: Vec<(Handle, Option<Handle>)>,
    fit_counts: std::collections::VecDeque<usize>,
    fit_widths: Vec<i32>,
    marker: Option<Vec<u8>>,
    marker_width: i32,
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

impl file_browser::BrowserForeign for Recording {
    fn directory_exists(&mut self, _: &[u8]) -> bool {
        self.existing_checks += 1;
        self.exists
    }
    fn create_directory(&mut self, path: &[u8]) -> bool {
        self.paths.push(path.to_vec());
        self.create_success
    }
    fn split_directory(&mut self, _: &[u8], capacity: usize, bytes: usize) -> (i32, Vec<u8>) {
        assert_eq!((capacity, bytes), (2, 260));
        (self.split_count, self.parent_path.clone())
    }
    fn split_file(&mut self, path: &[u8]) -> [Vec<u8>; 4] {
        let split = path.iter().rposition(|&b| b == b'\\').map_or(0, |i| i + 1);
        [Vec::new(), path[..split].to_vec(), path[split..].to_vec(), Vec::new()]
    }
    fn path_separator(&mut self, byte: u8) -> bool {
        matches!(byte, b'\\' | b'/')
    }
    fn path_extension(&mut self, path: &[u8]) -> Vec<u8> {
        path.iter()
            .rposition(|&b| b == b'.')
            .map_or_else(Vec::new, |i| path[i..].to_vec())
    }
    fn lowercase(&mut self, bytes: &mut [u8]) {
        bytes.make_ascii_lowercase();
    }
    fn right_trim(&mut self, bytes: &mut Vec<u8>) {
        while bytes.last().is_some_and(u8::is_ascii_whitespace) {
            bytes.pop();
        }
    }
    fn window_id(&mut self, _: Handle) -> u32 {
        17
    }
    fn set_window_flag(&mut self, _: Handle, mask: u32, value: bool) {
        self.window_flags.push((mask, value));
    }
    fn notify_path(&mut self, id: u32, win: u32, path: &[u8], fourth: u32) {
        assert_eq!((id, win, fourth), (0x7abe_7205, 17, 0));
        self.paths.push(path.to_vec());
    }
    fn notify_selection(
        &mut self,
        id: u32,
        win: u32,
        descriptor: file_browser::SelectionDescriptor,
        selection: &file_browser::Selection,
        fourth: u32,
    ) {
        assert_eq!((win, fourth), (17, 0));
        self.selections.push((id, descriptor, selection.clone()));
    }
    fn set_editor_text(&mut self, _: Handle, text: &[u8]) {
        self.editor_texts.push(text.to_vec());
    }
    fn grid_last_row(&mut self, _: Handle) -> u32 {
        self.grid_last
    }
    fn grid_text(&mut self, _: Handle, _: i32, row: i32) -> Option<Vec<u8>> {
        self.grid_reads += 1;
        self.grid_texts.get(&row).cloned()
    }
    fn grid_scroll_to(&mut self, _: Handle, row: i32) {
        self.bar = row;
    }
    fn grid_selected(&mut self, _: Handle, _: i32) -> bool {
        false
    }
    fn grid_select(&mut self, _: Handle, row: i32) {
        self.bar = row;
    }
    fn child_window(&mut self, control: Handle, accessor: u32) -> Handle {
        control + u64::from(accessor)
    }
    fn remove_child(&mut self, parent: Handle, child: Handle) {
        self.removed.push((parent, child));
    }
    fn set_button_buffer(&mut self, button: Handle, buffer: Option<Handle>) {
        self.button_updates.push((button, buffer));
    }
    fn font_fit(&mut self, _: Handle, _: &[u8], width: i32, option: u32) -> usize {
        assert_eq!(option, 0);
        self.fit_widths.push(width);
        self.fit_counts.pop_front().expect("fit count fixture")
    }
    fn text_resource(&mut self, id: u32) -> Option<Vec<u8>> {
        assert_eq!(id, 0x5377_be31);
        self.marker.clone()
    }
    fn font_rect(&mut self, _: Handle, _: &[u8]) -> Rect {
        [0, 0, self.marker_width, 10]
    }
}

#[test]
fn fb_c01_c05_identity_failed_navigation_and_ordered_refresh_work() {
    use file_browser::*;
    let mut f = Recording::default();
    let mut browser = FileBrowser::new(1000);
    assert!(browser.children.iter().all(Option::is_none));
    assert_eq!(browser.flags, 0xd);
    assert_eq!((PRIMARY_SLOTS, WINDOW_SLOTS), (107, 151));
    let mut out = None;
    assert!(browser.query_interface(IID, &mut out, &mut f));
    assert_eq!(out, Some(1000));
    assert_eq!(f.refs, [(true, 1000)]);
    browser.current = b"old\\".to_vec();
    assert!(!browser.set_current_directory(b"missing\\", &mut f));
    assert_eq!(browser.current, b"old\\");
    assert!(browser.refresh_work.is_empty());
    f.split_count = 1;
    assert!(!browser.go_to_parent(&mut f));
    let n = f.existing_checks;
    f.split_count = 2;
    f.parent_path = b"parent\\".to_vec();
    assert!(browser.go_to_parent(&mut f));
    assert_eq!(f.existing_checks, n + 1);
    browser.fallback = b"also-missing\\".to_vec();
    let n = f.existing_checks;
    assert!(browser.update_path(&mut f));
    assert_eq!(f.existing_checks, n + 1);
    assert_eq!(f.paths.last().unwrap(), b"also-missing\\");
    assert_eq!(browser.refresh_work, [Refresh::DirectoryList, Refresh::FileList]);
}

#[test]
fn fb_c06_c09_grid_lookup_wraparound_and_ambiguous_zero() {
    use file_browser::*;
    let mut browser = FileBrowser::new(1);
    let mut f = Recording::default();
    assert_eq!(browser.get_row_of_entry_by_name(b"a", &mut f), 0);
    assert_eq!(f.grid_reads, 0);
    browser.children[5] = Some(500);
    assert_eq!(browser.get_row_of_entry_by_name(b"a", &mut f), 0);
    browser.name_column = Some(0);
    f.grid_last = 3;
    f.grid_texts.insert(1, b"a".to_vec());
    f.grid_texts.insert(3, b"a".to_vec());
    assert_eq!(browser.get_row_of_entry_by_name(b"a", &mut f), 1);
    f.grid_texts.insert(0, b"a".to_vec());
    assert_eq!(browser.get_row_of_entry_by_name(b"a", &mut f), 0);
    for last in [u32::MAX - 1, u32::MAX] {
        f.grid_last = last;
        f.grid_reads = 0;
        assert_eq!(browser.get_row_of_entry_by_name(b"absent", &mut f), 0);
        assert_eq!(f.grid_reads, 1024);
    }
    f.grid_last = 0x8000_0000;
    f.grid_reads = 0;
    assert_eq!(browser.get_row_of_entry_by_name(b"a", &mut f), 0);
    assert_eq!(f.grid_reads, 0);
}

#[test]
fn fb_c10_c12_flag_precedence_and_any_bit_add() {
    let mut browser = file_browser::FileBrowser::new(1);
    let mut f = Recording::default();
    browser.flags = 1;
    browser.add_flags(5, &mut f);
    assert_eq!(browser.flags, 1);
    assert_eq!(f.existing_checks, 0);
    browser.add_flags(4, &mut f);
    assert_eq!(browser.flags, 5);
    assert_eq!(f.existing_checks, 1);
    browser.refresh_work.clear();
    browser.set_flags(3, &mut f);
    assert!(browser.refresh_work.is_empty());
    assert_eq!(f.window_flags, [(0x20000, false), (0x40000, false), (0x10000, true)]);
    f.window_flags.clear();
    browser.clear_flags(2, &mut f);
    assert_eq!(f.window_flags, [(0x20000, true), (0x40000, true)]);
    assert_eq!(browser.any_flag(5), 1);
}

#[test]
fn fb_c14_c18_creation_selection_and_double_click() {
    use file_browser::*;
    let mut browser = FileBrowser::new(1);
    let mut f = Recording {
        exists: true,
        ..Recording::default()
    };
    browser.current = b"C:\\".to_vec();
    assert!(!browser.create_new_directory(&mut f));
    assert_eq!(f.existing_checks, 256);
    assert_eq!(f.paths.last().unwrap(), b"C:\\New Folder (256)\\");
    assert!(browser.refresh_work.is_empty());
    browser.entries.push(Some(Entry {
        tag: CUSTOM_FILE,
        id: 42,
        name: b"old.txt".to_vec(),
    }));
    browser.children[6] = Some(400);
    assert_eq!(browser.selection_change(0, &mut f), Ok(true));
    let descriptor = browser.descriptor();
    assert_eq!(descriptor.id, 42);
    assert_eq!(f.selections[0].0, 0x7abe_7203);
    assert_eq!(f.editor_texts, [b"old.txt".to_vec()]);
    browser.selected_row = 7;
    assert_eq!(browser.selection_double_click(0, &mut f), Ok(true));
    assert_eq!(browser.selected_row, 7);
    assert_eq!(f.selections[1].0, 0x7abe_7204);
    assert!(browser.set_selection(b"C:\\missing.txt", &mut f));
    assert_eq!(browser.selection.id, 42);
    assert_eq!(browser.selection.kind, 2);
    assert_eq!(browser.borrowed_string(descriptor.strings[0]), b"C:\\missing.txt");
    assert_eq!(f.selections.len(), 2);
    assert!(browser.file_grid_double_click(0, &mut f));
    assert!(!browser.file_grid_double_click(u32::MAX, &mut f));
}

#[test]
fn fb_c23_c24_child_enable_and_reference_assignment() {
    use file_browser::*;
    let mut browser = FileBrowser::new(1);
    let mut f = Recording::default();
    browser.set_icon(0, Some(10), &mut f);
    browser.set_icon(0, Some(11), &mut f);
    browser.set_icon(0, Some(11), &mut f);
    browser.set_icon(7, Some(12), &mut f);
    assert_eq!(f.refs, [(true, 10), (true, 11), (false, 10)]);
    browser.children[9] = Some(100);
    browser.set_button_buffer(1, Some(20), &mut f);
    browser.set_button_buffer(1, None, &mut f);
    browser.set_button_buffer(1, None, &mut f);
    browser.set_button_buffer(u32::MAX, Some(99), &mut f);
    assert_eq!(f.button_updates, [(100, Some(20)), (100, None)]);
    browser.children[6] = Some(600);
    browser.children[5] = Some(500);
    browser.enable_child(FIRST_CHILD_ID + 6, 0, &mut f);
    assert_eq!(browser.children[6], None);
    assert_eq!(f.removed, [(5, 620)]);
    browser.enable_child(FIRST_CHILD_ID + 6, 2, &mut f);
    assert_eq!(browser.children[6], None);
    assert_eq!(browser.enabled[6], 2);
    browser.enable_child(FIRST_CHILD_ID + 5, 0, &mut f);
    assert_eq!(browser.children[5], Some(500));
}

#[test]
fn browser_compare_full_tags_byte_order_secondary_once_and_fit_boundary() {
    use file_browser::*;
    let mut f = Recording::default();
    let entry = |tag, text: &[u8]| Entry {
        tag,
        id: 0,
        name: text.to_vec(),
    };
    let p = entry(PARENT, b"..");
    assert!(precedes(&p, &p, 0, 0, 0, &mut f));
    assert!(precedes(
        &entry(FILE, b"file10"),
        &entry(FILE, b"file2"),
        0,
        0,
        0,
        &mut f
    ));
    assert!(!precedes(&entry(FILE, b"A"), &entry(FILE, b"a"), 0, 0, 0, &mut f));
    assert!(!precedes(&entry(FILE, b"a"), &entry(FILE, b"b"), 99, 0, 0, &mut f));
    assert!(!precedes(
        &entry(0xaaa8_8503, b"a"),
        &entry(FILE, b"b"),
        0,
        0,
        0,
        &mut f
    ));
    assert!(precedes(&entry(DIRECTORY, b"z"), &entry(FILE, b"a"), 0, 0, 0, &mut f));
    assert!(!precedes(&entry(DIRECTORY, b"z"), &entry(FILE, b"a"), 0, 0, 2, &mut f));
    let mut text = b"some text".to_vec();
    fit_string(&mut text, -10, None, &mut f).unwrap();
    assert_eq!(text, b"some text");
    f.fit_counts.push_back(1);
    f.marker_width = 12;
    fit_string(&mut text, 12, Some(100), &mut f).unwrap();
    assert!(text.is_empty());
    text = b"ab   tail".to_vec();
    f.fit_counts.extend([3, 5]);
    fit_string(&mut text, 13, Some(100), &mut f).unwrap();
    assert_eq!(text, b"ab...");
    assert_eq!(*f.fit_widths.last().unwrap(), 1);
}

#[test]
fn browser_parent_directory_ids_and_shutdown_reference_order() {
    use file_browser::*;
    let mut browser = FileBrowser::new(1);
    let mut f = Recording::default();
    assert_eq!(browser.raw_words[&0x224], 20);
    assert_eq!(browser.raw_words[&0x1f8], u32::MAX);
    assert_eq!(browser.style_words[0][6], 0);
    assert_eq!(browser.raw_bytes[&0x1dc], 1);
    browser.entries = vec![Some(Entry {
        tag: DIRECTORY,
        id: 42,
        name: b"folder".to_vec(),
    })];
    browser.selection_change(0, &mut f).unwrap();
    assert_eq!(browser.selection.id, 0);
    assert_eq!(browser.selection.leaf, b"folder\\");
    browser.entries[0].as_mut().unwrap().tag = PARENT;
    browser.selection_change(0, &mut f).unwrap();
    assert_eq!(browser.selection.kind, 0);
    for (i, child) in browser.children.iter_mut().enumerate() {
        *child = Some(100 + i as u64);
    }
    browser.common_font = Some(200);
    for (i, icon) in browser.icons.iter_mut().enumerate() {
        *icon = Some(300 + i as u64);
    }
    for (i, buffer) in browser.button_buffers.iter_mut().enumerate() {
        *buffer = Some(400 + i as u64);
    }
    browser.extra_reference = Some(500);
    browser.style_fonts = [Some(600), Some(601), Some(602), Some(603)];
    browser.custom_groups = vec![vec![CustomEntry {
        interface: Some(700),
        text: vec![],
    }]];
    browser.filter_labels = vec![b"All files".to_vec()];
    browser.filter_index = 9;
    browser.shutdown(&mut f);
    let mut released = vec![700];
    released.extend(100..113);
    released.push(200);
    released.extend(300..307);
    released.extend(400..404);
    released.extend([500, 600, 601, 603]);
    assert_eq!(f.refs, released.into_iter().map(|h| (false, h)).collect::<Vec<_>>());
    assert_eq!(browser.style_fonts[2], Some(602));
    assert_eq!(browser.filter_index, 0);
    assert_eq!(browser.filter_labels, [b"All files".to_vec()]);
    assert!(browser.entries.is_empty());
    let n = f.refs.len();
    browser.shutdown(&mut f);
    assert_eq!(f.refs.len(), n);
}
