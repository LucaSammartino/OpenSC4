// SPDX-License-Identifier: GPL-3.0-or-later
use super::super::{
    layout::Vertical,
    runtime::{Paint, TextHost, Timer},
    *,
};
use super::{editor, Services};
use crate::widget_controls::{Message, Rect};

impl TextHost for Services {
    fn retain(&mut self, h: Handle) {
        self.events.push(format!("+{h}"));
    }
    fn relinquish(&mut self, h: Handle) {
        self.events.push(format!("-{h}"));
    }
    fn base_query(&mut self, _: Handle, _: u32, _: &mut Option<Handle>) -> bool {
        false
    }
    fn base_init(&mut self, _: Handle) {
        self.events.push("init".into());
    }
    fn base_shutdown(&mut self, _: Handle) {
        self.events.push("shutdown".into());
    }
    fn base_area(&mut self, _: Handle, r: Rect) {
        self.events.push(format!("area{r:?}"));
    }
    fn base_height(&mut self, _: Handle, height: u32) {
        self.events.push(format!("height{height}"));
    }
    fn key_state(&mut self, key: u32) -> bool {
        self.events.push(format!("key{key:x}"));
        false
    }
    fn base_message(&mut self, _: Handle, _: Message) -> bool {
        self.events.push("base-message".into());
        false
    }
    fn base_show(&mut self, _: Handle, _: bool) {
        self.events.push("show".into());
    }
    fn base_focus(&mut self, _: Handle, _: bool) -> bool {
        true
    }
    fn base_flag(&mut self, _: Handle, _: u32, _: bool) {}
    fn base_fill(&mut self, _: Handle, _: u32) -> bool {
        true
    }
    fn focused(&mut self, _: Handle) -> bool {
        self.focused
    }
    fn enabled(&mut self, _: Handle) -> bool {
        true
    }
    fn capturing(&mut self, _: Handle) -> bool {
        self.captured
    }
    fn capture(&mut self, _: Handle, take: bool) {
        self.captured = take;
    }
    fn cursor(&mut self, _: Handle, _: Option<u32>) {}
    fn cursor_position(&mut self, _: Handle) -> (i32, i32) {
        self.cursor
    }
    fn click_elapsed(&mut self) -> Option<u32> {
        self.click
    }
    fn click_timer(&mut self, restart: bool) {
        self.click = if restart { Some(0) } else { None };
    }
    fn font_lookup(&mut self, _: u32) -> Option<Handle> {
        Some(7)
    }
    fn clipboard_service(&mut self) -> Option<Handle> {
        None
    }
    fn code_page(&mut self) -> encoding::Encoding {
        encoding::Encoding {
            selector: 0,
            system_selector: 0,
        }
    }
    fn timer(&mut self, t: Timer, start: bool, interval: u32, argument: Handle) {
        self.events.push(format!("timer{t:?}:{start}:{interval}:{argument}"));
    }
    fn timer_destroy(&mut self, t: Timer) {
        self.events.push(format!("destroy{t:?}"));
    }
    fn callback_dispatch(&mut self, _: Handle, kind: u32) {
        self.events.push(format!("callback{kind:x}"));
    }
    fn convert_color(&mut self, c: u32, native: bool) -> u32 {
        self.events.push(format!("color{native}"));
        c
    }
    fn font_height(&mut self, _: Handle) -> u32 {
        self.height
    }
    fn font_fit(&mut self, _: Handle, bytes: &[u8], width: u32) -> u32 {
        (bytes.len() as u32).min(width / 10)
    }
    fn font_measure(&mut self, _: Handle, bytes: &[u8]) -> u32 {
        bytes.len() as u32 * 10
    }
    fn bar_create(&mut self, horizontal: bool, _: [u32; 5]) -> Option<Handle> {
        self.events.push(format!("create{horizontal}"));
        self.bar_success.then_some(if horizontal { 9 } else { 8 })
    }
    fn bar_remove_child(&mut self, _: Handle, bar: Handle) {
        self.events.push(format!("remove{bar}"));
    }
    fn bar_add_child(&mut self, _: Handle, bar: Handle) {
        self.events.push(format!("child{bar}"));
    }
    fn bar_dimensions(&mut self, _: Handle) -> (u32, u32) {
        (10, 8)
    }
    fn bar_place(&mut self, bar: Handle, r: Rect) {
        self.events.push(format!("bar{bar}:{r:?}"));
    }
    fn bar_range(&mut self, _: Handle, _: i32) {
        self.events.push("range".into());
    }
    fn bar_pair(&mut self, _: Handle, _: i32, _: u32) {
        self.events.push("pair".into());
    }
    fn bar_value(&mut self, _: Handle, _: Option<u32>) -> u32 {
        self.events.push("value".into());
        3
    }
    fn image_rect(&mut self, _: Handle) -> Rect {
        [1, 2, 3, 4]
    }
    fn bar_image(&mut self, _: Handle, _: Handle, r: Rect) {
        self.events.push(format!("image{r:?}"));
    }
    fn clipboard_copy(&mut self, _: Handle, bytes: &[u8], _: u32, _: u32) {
        self.clipboard = Some(bytes.to_vec());
    }
    fn clipboard_paste(&mut self, _: Handle, _: u32) -> Option<Vec<u8>> {
        self.clipboard.clone()
    }
    fn composed_previous(&mut self, _: &[u8], _: i32) -> u32 {
        self.composed as u32
    }
    fn send(&mut self, _: Handle, kind: u32, words: [u64; 3]) -> bool {
        self.events.push(format!("send{kind:x}:{words:?}"));
        true
    }
    fn draw(&mut self, p: Paint) {
        self.paints.push(p);
    }
    fn search(&mut self, _: &[u8], _: &[u8], _: u32, _: bool) -> u32 {
        u32::MAX
    }
    fn sound_acquire(&mut self, _: u32, _: [u32; 6]) -> (bool, Option<Handle>) {
        (false, None)
    }
    fn sound_play(&mut self, _: Handle) -> bool {
        false
    }
}

fn fixture(text: &[u8]) -> (TextEdit, Services) {
    let mut e = editor(0, 0);
    e.text = text.to_vec();
    e.lines = if text.is_empty() { vec![] } else { vec![0] };
    e.runtime.font = Some(7);
    e.runtime.visible = [100, 40];
    e.runtime.size = [100, 50];
    e.runtime.line_height = 10;
    e.runtime.rows = 4;
    e.gutters = [0, 0];
    (
        e,
        Services {
            height: 10,
            ..Services::default()
        },
    )
}

#[test]
fn te_live_character_and_output_adapters_vectors_9_48_49_54_55() {
    let (mut e, mut f) = fixture(b"abc");
    e.flags = 1;
    assert_eq!(e.character(b'\t', &mut f), Ok(true));
    assert_eq!(f.events, ["key11", "key12"]);
    assert_eq!(e.text, b"abc");
    e.encoding.selector = 8;
    e.position.index = 3;
    e.position.anchor = 3;
    e.position.active = 3;
    assert_eq!(e.character(0xc0, &mut f), Ok(true));
    assert_eq!(e.character(0x41, &mut f), Ok(false));
    assert!(e.pending.is_empty());
    assert_eq!(&e.text[3..], &[0xc0, 0x41]);
    let mut output = [0xfe; 4];
    let mut count = 2;
    e.get_text_buffer(&mut output, &mut count).unwrap();
    assert_eq!(output, [b'a', b'b', 0xfe, 0xfe]);
    assert_eq!(count, 2);
    struct Outputs(u32);
    impl adapters::WordOutputs for Outputs {
        fn write(&mut self, _: Handle, value: u32) { self.0 = value; }
    }
    let mut outputs = Outputs(0);
    e.position.anchor = 1;
    e.position.active = 4;
    assert!(e.get_selection(99, 99, &mut outputs));
    assert_eq!(outputs.0, 4);
    let mut style = 42;
    assert!(!e.get_font_style(&mut style));
    assert_eq!(style, 42);
    assert_eq!(e.copy_to_string(|s| s.to_vec()), e.text);
}

#[test]
fn te_exact_height_preserves_origin_and_background_excludes_bars() {
    let (mut e, mut f) = fixture(b"abc");
    e.runtime.size = [100, 25];
    e.gutters[1] = 2;
    e.exact_line_heights(&mut f).unwrap();
    assert_eq!(e.runtime.size[1], 24);
    assert!(f.events.iter().any(|event| event == "height24"));
    assert!(!f.events.iter().any(|event| event.starts_with("area")));
    e.runtime.bars = [Some(8), Some(9)];
    e.flags = 0x20;
    e.runtime.background = Some(20);
    e.paint([30, 40, 130, 90], &mut f).unwrap();
    assert_eq!(f.paints[0], Paint::Image(20, [30, 40, 120, 82]));
    e.runtime.background_option = 1;
    f.paints.clear();
    e.paint([30, 40, 130, 90], &mut f).unwrap();
    assert_eq!(f.paints[0], Paint::EdgeImage { image: 20, area: [30, 40, 120, 82], thirds: [0, 0] });
}

#[test]
fn te_single_line_threshold_bar_failure_and_preferred_x_vectors_50_51_53() {
    let (mut e, mut f) = fixture(b"abcdefgh\nabcdefgh\nabcdefgh");
    e.flags = 0x200;
    e.runtime.visible[0] = 40;
    e.position.index = 6;
    e.position.column = 6;
    e.insertion_in_view(&mut f).unwrap();
    assert_eq!(e.runtime.horizontal_byte, 4);
    e.flags = 12;
    f.bar_success = false;
    e.add_bars(true, true, &mut f).unwrap();
    assert_eq!(e.flags & 12, 0);
    assert_eq!(e.runtime.bars, [None; 2]);
    e.flags = 1;
    e.lines = vec![0, 9, 18];
    e.runtime.rows = 1;
    e.runtime.line_height = 10;
    e.position = Position { line: 1, column: 2, index: 11, anchor: 11, active: 11 };
    e.preferred_x = 70;
    e.key_down(0x28, 0, &mut f).unwrap();
    assert_eq!(e.position.column, 6);
    assert_eq!(e.preferred_x, 70);
    e.position = Position { line: 1, column: 2, index: 11, anchor: 11, active: 11 };
    e.key_down(0x22, 0, &mut f).unwrap();
    assert_eq!(e.position.column, 1);
    assert_eq!(e.preferred_x, 70);
}

#[test]
fn te_runtime_caret_resources_and_flag_vectors_7_8_31_34_57_63_64() {
    let (mut e, mut f) = fixture(b"abc");
    e.toggle_caret(&mut f);
    assert!(!e.runtime.caret_visible);
    assert!(f.invalidations.is_empty());
    e.caret_period = 1001;
    assert!(e.caret_timer(true, &mut f));
    assert!(!e.caret_timer(true, &mut f));
    assert!(f.events[0].contains(":500:"));
    f.events.clear();
    e.set_background(Some(20), 0, &mut f);
    f.events.clear();
    let invalidations = f.invalidations.len();
    e.set_background(Some(20), 0, &mut f);
    assert!(f.events.is_empty());
    assert_eq!(f.invalidations.len(), invalidations);
    e.set_background(Some(20), 1, &mut f);
    assert_eq!(f.invalidations.len(), invalidations + 1);
    assert!(f.events.is_empty());
    e.runtime.bars[0] = Some(8);
    e.runtime.images[0] = Some(21);
    e.set_bar_image(0, Some(21), [0; 4], &mut f);
    assert_eq!(&f.events[..3], ["+21", "-21", "image[1, 2, 3, 4]"]);
    f.events.clear();
    let n = f.invalidations.len();
    e.set_outline(true, 42, &mut f);
    assert_eq!(f.invalidations.len(), n);
    e.set_caret_period(0, &mut f);
    assert_eq!(e.caret_period, 250);
    e.set_caret_period(u32::MAX, &mut f);
    assert_eq!(e.caret_period, 10000);
    assert!(!f.events.iter().any(|v| v.starts_with("timer")));
    e.flags = 1;
    assert!(e.is_flag_set(3));
    f.events.clear();
    e.set_caret_color(4, &mut f);
    e.set_highlight_color(5, &mut f);
    assert_eq!(f.events, ["colortrue", "colorfalse"]);
    e.shutdown(&mut f);
    assert!(!f.events.iter().any(|v| v == "-20"));
}

#[test]
fn te_runtime_layout_and_scroll_vectors_12_15_29_30_39_44_61() {
    let (mut e, mut f) = fixture(b"abcd");
    e.runtime.visible[0] = 15;
    assert_eq!(e.horizontal_metrics(0, 9, &mut f), Ok((0, 3, -9)));
    assert_eq!(e.horizontal_metrics(0, 10, &mut f), Ok((1, 3, 0)));
    assert_eq!(e.column_from_position(0, 9, &mut f), Ok(0));
    assert_eq!(e.line_column_from_position(5, 0, &mut f), Ok((0, 0)));
    assert_eq!(e.line_column_from_position(6, 0, &mut f), Ok((0, 1)));
    e.runtime.horizontal_pixel = 5;
    assert_eq!(e.scroll_vertical(Vertical::Start, &mut f), Ok(false));
    assert_eq!(e.runtime.horizontal_pixel, 5);
    e.runtime.bars[1] = Some(9);
    e.runtime.line_height = 16;
    e.scroll_horizontal(false, &mut f).unwrap();
    assert_eq!(e.runtime.horizontal_pixel as i32, -11);
    e.runtime.visible[0] = 100;
    e.runtime.horizontal_pixel = 95;
    e.scroll_horizontal(true, &mut f).unwrap();
    assert_eq!(e.runtime.horizontal_pixel, 100);
    e.runtime.bars[0] = Some(8);
    f.events.clear();
    e.bar_sizes(&mut f);
    assert_eq!(f.events, ["bar9:[0, 42, 90, 50]", "bar8:[91, 0, 100, 42]"]);
    e.lines = (0..10).collect();
    e.first_visible_line = 5;
    e.runtime.rows = 4;
    e.scroll_vertical(Vertical::PageDown, &mut f).unwrap();
    assert_eq!(e.first_visible_line, 9);
    e.first_visible_line = 2;
    e.runtime.bars = [None; 2];
    e.runtime.horizontal_pixel = 0;
    f.captured = true;
    f.cursor = (-3, -3);
    e.lines = vec![0];
    e.cursor_scroll(&mut f).unwrap();
    assert_eq!(e.first_visible_line, 1);
    f.captured = false;
    e.first_visible_line = 2;
    e.wheel(99, &mut f).unwrap();
    e.wheel(99, &mut f).unwrap();
    assert_eq!(e.first_visible_line, 2);
}

#[test]
fn te_runtime_line_reflow_clipboard_keys_and_paint_vectors() {
    let (mut e, mut f) = fixture(b"a\n");
    e.initialized = true;
    e.runtime.visible = [100, 7];
    e.runtime.size = [100, 7];
    f.height = 16;
    e.recalculate(&mut f).unwrap();
    assert_eq!(e.runtime.rows, 1);
    assert_eq!(e.lines, [0, 2]);
    e.text = b"a".to_vec();
    e.recalculate(&mut f).unwrap();
    assert_eq!(e.lines, [0]);
    e.text = b"abcde".to_vec();
    e.flags = 0x1001;
    e.position.anchor = 1;
    e.position.active = 3;
    e.key_down(0x58, 2, &mut f).unwrap();
    assert_eq!(e.text, b"ade");
    e.runtime.clipboard = Some(30);
    e.flags = 0x3000;
    f.clipboard = Some(b"x".to_vec());
    e.key_down(0x43, 2, &mut f).unwrap();
    assert_eq!(f.clipboard, Some(b"x".to_vec()));
    e.key_down(0x56, 2, &mut f).unwrap();
    assert!(e.text.contains(&b'x'));
    e.flags = 0x400;
    f.events.clear();
    e.key_down(13, 0, &mut f).unwrap();
    assert!(f.events.iter().any(|s| s.starts_with("send9b1552d5:")));
    e.flags = 1;
    f.focused = true;
    e.runtime.caret_visible = true;
    e.position = Position::default();
    e.lines = vec![0];
    for (height, width) in [(23, 1.0), (24, 2.0)] {
        e.runtime.line_height = height;
        f.paints.clear();
        e.paint([0, 0, 100, 50], &mut f).unwrap();
        let widths: Vec<_> = f
            .paints
            .iter()
            .filter_map(|p| if let Paint::LineWidth(w) = p { Some(*w) } else { None })
            .collect();
        assert_eq!(widths, [width, 1.0]);
    }
    e.flags = 0;
    e.runtime.visible = [100, 50];
    e.text = b"abc def".to_vec();
    e.position = Position {
        index: 5,
        anchor: 0,
        active: 5,
        ..Position::default()
    };
    e.select_word(&mut f).unwrap();
    assert_eq!((e.position.anchor, e.position.active), (0, 7));
}
