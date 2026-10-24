// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::VecDeque;

#[path = "runtime_tests.rs"]
mod runtime_tests;

#[path = "closure_tests.rs"]
mod closure_tests;

#[derive(Default)]
pub(super) struct Services {
    reasons: Vec<u32>,
    invalidations: Vec<Handle>,
    notifications: Vec<(Handle, u32)>,
    contexts: Vec<(Vec<u8>, u32, i32)>,
    decoded: VecDeque<(u32, usize)>,
    levels: VecDeque<u32>,
    comparisons: Vec<(u16, u16, u32)>,
    composed: usize,
    events: Vec<String>,
    paints: Vec<runtime::Paint>,
    focused: bool,
    captured: bool,
    clipboard: Option<Vec<u8>>,
    height: u32,
    bar_success: bool,
    cursor: (i32, i32),
    click: Option<u32>,
    image_source: Option<crate::widget_controls::Rect>,
    bar_size: Option<(u32, u32)>,
}

impl TextForeign for Services {
    fn previous_utf16(&mut self, context: &[u8], index: u32, selector: i32) -> u32 {
        self.contexts.push((context.to_vec(), index, selector));
        0x1234
    }
    fn decode_utf16(&mut self, bytes: &[u8], _: i32) -> (u32, usize) {
        self.decoded.pop_front().unwrap_or((bytes[0] as u32, 1))
    }
    fn thai_level(&mut self, current: u16, previous: u16, argument: u32) -> u32 {
        self.comparisons.push((current, previous, argument));
        self.levels.pop_front().unwrap_or(1)
    }
    fn composed_length(&mut self, _: &[u8], _: i32) -> usize {
        self.composed
    }
    fn correction(&mut self, reason: u32) {
        self.reasons.push(reason);
    }
    fn invalidate(&mut self, window: Handle) {
        self.invalidations.push(window);
    }
    fn notify(&mut self, view: Handle, kind: u32) {
        self.notifications.push((view, kind));
    }
}

impl conversion::CodePages for Services {
    fn system_code_page(&mut self) -> i32 {
        932
    }
    fn multi_byte_to_wide(&mut self, _: i32, _: u32, _: &[u8], _: &mut [u16]) -> usize {
        panic!("unexpected code-page call")
    }
    fn wide_to_multi_byte(&mut self, _: i32, _: u32, _: &[u16], _: &mut [u8], _: u8) -> usize {
        panic!("unexpected code-page call")
    }
    fn composition_tail(&mut self) -> u16 {
        0
    }
}

fn encoding(selector: i32) -> Encoding {
    Encoding {
        selector,
        system_selector: 932,
    }
}

fn editor(selector: i32, flags: u32) -> TextEdit {
    TextEdit::with_resolved_defaults(1, 2, flags, encoding(selector), 8, u32::MAX)
}

/// Already-resolved line starts, not a replacement font layout algorithm.
fn text(editor: &mut TextEdit, bytes: &[u8]) {
    editor.text = bytes.to_vec();
    editor.lines = if bytes.is_empty() { vec![] } else { vec![0] };
    editor.lines.extend(
        bytes
            .iter()
            .enumerate()
            .filter_map(|(i, &b)| (b == b'\n').then_some(i as u32 + 1)),
    );
}

#[test]
fn te_encoding_forward_domains_and_malformed_vectors_45_47() {
    let utf = encoding(8);
    assert_eq!(utf.char_length(&[0xe0, 0x80], 1), Ok(3));
    assert!(utf.char_length(&[0xe0], 1).is_err());
    assert_eq!(utf.char_length(&[0xe0, b'A', b'B'], 3), Ok(1));
    assert_eq!(utf.char_length(&[0xc0, 0x80], 2), Ok(2));
    assert_eq!(utf.char_length(&[0xfd, 0x80], 2), Ok(6));
    assert_eq!(utf.char_length(&[], 0), Ok(0));
    assert_eq!(utf.previous_length(&[0x80, 0xbf]), 3);
    assert_eq!(utf.glyph_count(&[0x80, 0xbf]), 0);
    assert_eq!(encoding(100).char_length(&[0x81, b'A'], 2), Ok(1));
    assert_eq!(encoding(100).glyph_count(&[0x81, b'A']), 2);
    assert_eq!(encoding(-2).char_length(&[0xc3, 0xa9], 2), Ok(2));
    assert_eq!(encoding(-1).char_length(&[0x81, b'A'], 2), Ok(2));
    for selector in [0, 16] {
        assert_eq!(encoding(selector).char_length(&[], 1), Ok(1));
        assert_eq!(encoding(selector).char_length(&[0xff], 1), Ok(1));
        assert_eq!(encoding(selector).glyph_count(&[0x80, 0x80]), 2);
    }
}

#[test]
fn te_dbcs_lead_ranges_prefix_alignment_and_unmatched_leads() {
    for (selector, first, last) in [
        (932, 0x81, 0x9f),
        (936, 0xa1, 0xfc),
        (949, 0xb0, 0xc8),
        (950, 0x81, 0xfe),
    ] {
        let enc = encoding(selector);
        assert_eq!(enc.char_length(&[first], 1), Ok(2));
        assert_eq!(enc.char_length(&[last, 0], 2), Ok(2));
        assert_eq!(enc.char_length(&[first - 1], 1), Ok(1));
        assert_eq!(enc.previous_length(&[first]), 1);
        assert_eq!(enc.previous_length(&[first, first]), 2);
        assert_eq!(enc.previous_length(&[first, first, first]), 1);
        assert_eq!(enc.glyph_count(&[first, first, first]), 2);
    }
    assert_eq!(encoding(932).char_length(&[0xe0], 1), Ok(2));
    assert_eq!(encoding(932).char_length(&[0xfc], 1), Ok(2));
    assert_eq!(encoding(932).char_length(&[0xfd], 1), Ok(1));
}

#[test]
fn te_selection_reversed_unsigned_endpoints_and_insert_mode_vectors() {
    let mut e = editor(0, 0);
    let mut f = Services::default();
    assert_eq!(e.maximum, u32::MAX);
    assert_eq!(
        (e.gutters, e.mask, e.caret_period, e.separate_interval),
        ([5, 0], b'*', 1000, 500)
    );
    text(&mut e, b"abcde");
    assert_eq!(e.set_selection(4, 1, &mut f), Ok(true));
    assert_eq!(
        e.position,
        Position {
            line: 0,
            column: 1,
            index: 1,
            anchor: 4,
            active: 1
        }
    );
    e.set_selection(u32::MAX, 2, &mut f).unwrap();
    assert_eq!((e.position.anchor, e.position.active, e.position.index), (5, 2, 2));
    assert!(e.set_insert_mode(true));
    assert_eq!(e.flags & 0x4000, 0);
    e.set_insert_mode(false);
    assert!(!e.insert_mode());
    assert_eq!(e.text, b"abcde");
    assert_eq!(f.invalidations, vec![1; 2]);
    assert!(e.repairs.is_empty());
}

#[test]
fn te_adapter_copy_line_spans_insertion_and_asymmetric_repair() {
    let mut e = editor(0, 0);
    let mut f = Services::default();
    text(&mut e, b"a\nbc");
    let mut destination = [b'!'; 5];
    let mut count = 2;
    e.copy_bytes(&mut destination, &mut count).unwrap();
    assert_eq!((destination, count), (*b"a\n!!!", 2));
    count = 99;
    e.copy_bytes(&mut destination, &mut count).unwrap();
    assert_eq!((destination, count), (*b"a\nbc!", 4));
    assert_eq!(e.line_byte_count(0), Ok(2));
    assert_eq!(e.line_byte_count(u32::MAX), Ok(2));
    assert_eq!(e.line_column(2), Ok((1, 0)));
    assert_eq!(e.index_from_line_column(0, 3), (true, 3)); // whole-text bound
    assert_eq!(e.index_from_line_column(99, 3), (false, 4));
    assert_eq!(e.index_from_line_column(1, u32::MAX), (true, 1)); // wraps
    e.position.anchor = 99;
    e.position.active = 99;
    e.position.index = 99;
    e.fix_insertion().unwrap();
    assert_eq!((e.position.anchor, e.position.active, e.position.index), (99, 4, 4));
    e.preferred_x = 123;
    e.set_insertion_index(u32::MAX, &mut f).unwrap();
    assert_eq!((e.position.anchor, e.position.active, e.preferred_x), (99, 4, 123));
    e.set_insertion_line_column(0, 99).unwrap();
    assert_eq!((e.position.line, e.position.column, e.position.index), (0, 2, 2));
    assert_eq!(f.invalidations.len(), 1);
    text(&mut e, b"");
    e.fix_insertion().unwrap();
    assert_eq!(e.position, Position::default());
    assert_eq!(e.index_from_line_column(0, 0), (false, 0));
}

#[test]
fn te_set_text_strlen_clear_history_limits_and_notify_vectors() {
    let mut e = editor(0, 0x100);
    let mut f = Services::default();
    text(&mut e, b"abcde");
    e.set_maximum(2);
    assert_eq!(e.text, b"abcde");
    e.record_deletion(0, b"old".to_vec(), Position::default());
    e.redo = e.undo.clone();
    e.set_text_bytes(Some(b"abc\0ignored"), 0, true, &mut f).unwrap();
    assert_eq!(e.text, b"ab");
    assert_eq!(f.reasons, [1]);
    assert_eq!(f.contexts[0], (vec![], 0, 0));
    assert!(e.undo.is_empty() && e.redo.is_empty());
    assert_eq!(f.notifications, [(2, TEXT_CHANGED)]);
    assert_eq!(
        e.repairs,
        [Repair::CalculateLines, Repair::ScrollValues, Repair::AutomaticBars]
    );
    e.set_maximum(u32::MAX);
    e.set_text_bytes(Some(b"a\0b"), 3, false, &mut f).unwrap();
    assert_eq!(e.text, b"a\0b");
    e.set_text_bytes(None, 0, false, &mut f).unwrap();
    assert!(e.text.is_empty());
    assert_eq!(e.repairs.last(), Some(&Repair::RecalculateAll));
}

#[test]
fn te_multibyte_truncation_uses_original_settext_pointer_and_filtered_insert_pointer() {
    let mut e = editor(8, 0);
    let mut f = Services::default();
    e.set_maximum(1);
    e.set_text_bytes(Some(&[0xc3, 0xa9]), 2, false, &mut f).unwrap();
    assert_eq!(e.text, [0xc3, 0xa9]);
    assert_eq!(f.reasons, [1]);
    e.flags |= 0x200;
    e.set_text_bytes(Some(&[b'\n', 0xc3, 0xa9, b'x']), 4, false, &mut f)
        .unwrap();
    // Original LF is one byte; candidate C3 A9 is not used for the length step.
    assert_eq!(e.text, [0xc3]);
    assert_eq!(f.reasons, [1, 0, 1]);
    text(&mut e, b"");
    assert_eq!(e.insert_bytes(&[b'\n', 0xc3, 0xa9, b'x'], 0, &mut f), Ok(2));
    assert_eq!(e.text, [0xc3, 0xa9]);
    assert_eq!(e.position.index, 1); // glyph count, not retained-byte count
    e.set_maximum(0);
    e.set_text_bytes(Some(&[0xc3, 0xa9]), 2, false, &mut f).unwrap();
    assert!(e.text.is_empty());
}

#[test]
fn te_composition_owner_erases_without_advancing_previous_and_zero_consumes_rest() {
    let e = editor(0, 0x200);
    let mut f = Services {
        decoded: [(0x10041, 1), (0x10042, 1), (0x10043, 1), (0, 0)].into(),
        levels: [1, 0, 1].into(),
        ..Services::default()
    };
    let mut candidate = b"abcde".to_vec();
    assert_eq!(e.fix_composition(&mut candidate, b"context", 3, &mut f), Ok(3));
    assert_eq!(candidate, b"ac");
    assert_eq!(f.comparisons, [(0x41, 0x1234, 0), (0x42, 0x41, 0), (0x43, 0x41, 0)]);
    assert_eq!(f.contexts, [(b"context".to_vec(), 3, 0)]);
    let mut empty = Vec::new();
    e.fix_composition(&mut empty, b"", 0, &mut f).unwrap();
    assert_eq!(f.contexts.len(), 2);
    let mut candidate = b"a\r\nb\t".to_vec();
    assert_eq!(e.remove_bad_characters(&mut candidate), Ok(1));
    assert_eq!(candidate, b"a\rb\t"); // CR and tab survive programmatic filtering
}

#[test]
fn te_raw_limits_invalid_domains_and_no_history_or_notify() {
    let mut f = Services::default();
    for selector in [8, 100] {
        let mut e = editor(selector, 0x100 | 0x800);
        text(&mut e, b"abcde");
        e.set_maximum(2);
        e.initialized = true;
        assert_eq!(e.insert_bytes(b"x", 3, &mut f), Ok(0));
        assert_eq!(e.text, b"abcde");
        assert_eq!(e.position.index, 3);
        assert_eq!(
            e.repairs,
            [Repair::CalculateLines, Repair::InsertionInView, Repair::AutomaticBars]
        );
        assert!(e.undo.is_empty());
    }
    assert_eq!(f.reasons, [1, 1]);
    assert!(f.notifications.is_empty());
    let mut e = editor(0, 0);
    text(&mut e, b"abcde");
    e.maximum = 2;
    assert_eq!(
        e.insert_bytes(b"x", 0, &mut f),
        Err(Unsupported::Precondition(
            "TE3 lowered ordinary limit wraps retained count"
        ))
    );
    assert_eq!(e.text, b"abcde");
    assert_eq!(e.insert_bytes(b"", 0, &mut f), Ok(0)); // empty returns before limit
    text(&mut e, b"");
    assert!(e.remove_character(0, true, &mut f).is_err());
    e.maximum = 8;
    assert_eq!(e.insert_bytes(b"abc", u32::MAX, &mut f), Ok(3));
    assert_eq!(e.text, b"abc");
    assert_eq!(e.position.index, 3);
}

#[test]
fn te_raw_remove_caps_unsigned_counts_preserves_endpoints_and_character_modes() {
    let mut f = Services {
        composed: 2,
        ..Services::default()
    };
    let mut e = editor(0, 0);
    text(&mut e, b"abcde");
    e.position.anchor = 5;
    e.position.active = 4;
    assert!(e.remove_bytes(u32::MAX, 1, &mut f).is_empty());
    assert_eq!(e.remove_bytes(3, u32::MAX, &mut f), b"de");
    assert_eq!((e.position.index, e.position.anchor, e.position.active), (3, 5, 4));
    assert_eq!(e.remove_character(0, false, &mut f), Ok(b"ab".to_vec()));
    assert_eq!(e.remove_character(u32::MAX, true, &mut f), Ok(b"c".to_vec()));
    assert!(e.text.is_empty());
    assert!(e.undo.is_empty() && f.notifications.is_empty());
}

#[test]
fn te_insertion_history_quarter_eviction_deletion_unbounded_and_zero_limit_vectors() {
    let mut e = editor(0, 0x100);
    for n in 0..8 {
        assert!(e.record_deletion(n, vec![n as u8], Position::default()));
    }
    let old = e.undo.clone();
    e.redo = old.clone();
    assert!(e.record_insertion(8, 1, vec![], Position::default()));
    assert_eq!(e.undo.len(), 7);
    assert_eq!(e.undo[..6], old[2..]);
    assert!(e.redo.is_empty());
    e.undo = old.clone();
    e.record_deletion(8, vec![], Position::default());
    assert_eq!(e.undo.len(), 9);
    assert_eq!(e.undo[..8], old[..]);
    e.undo.clear();
    e.set_history_limit(0);
    for (n, expected) in [1, 2, 3, 4, 4].into_iter().enumerate() {
        e.record_insertion(n as u32, 1, vec![], Position::default());
        assert_eq!(e.undo.len(), expected);
    }
    assert_eq!(e.undo[0].at, 1);
    e.undo.truncate(3);
    let old = e.undo.clone();
    e.set_history_limit(0);
    assert_eq!(e.undo, old);
    e.record_insertion(99, 1, vec![], Position::default());
    assert_eq!(e.undo.len(), 4);
    e.flags &= !0x100;
    assert!(!e.record_insertion(0, 1, vec![], Position::default()));
    assert!(!e.record_deletion(0, vec![], Position::default()));
}

#[test]
fn te_undo_redo_replays_bytes_and_all_five_state_words_without_filter_or_notification() {
    let mut e = editor(0, 0x100 | 0x800);
    let mut f = Services::default();
    text(&mut e, b"abcde");
    let before = Position {
        line: 2,
        column: 8,
        index: 1,
        anchor: 4,
        active: 1,
    };
    e.position = before;
    e.insert_high_level_bytes(b"XY", &mut f).unwrap();
    assert_eq!(e.text, b"aXYe");
    assert_eq!(e.undo.len(), 1);
    let after = e.position;
    let corrections = f.contexts.len();
    let notifications = f.notifications.len();
    e.flags |= 0x200;
    e.maximum = 0;
    assert_eq!(e.undo_bytes(&mut f), Ok(true));
    assert_eq!(e.text, b"abcde");
    assert_eq!(e.position, before);
    assert_eq!(e.redo_bytes(&mut f), Ok(true));
    assert_eq!(e.text, b"aXYe");
    assert_eq!(e.position, after);
    assert_eq!(f.contexts.len(), corrections);
    assert_eq!(f.notifications.len(), notifications);
    assert_eq!(e.redo_bytes(&mut f), Ok(false));
    e.flags &= !0x100;
    assert_eq!(e.undo_bytes(&mut f), Ok(false));
    assert_eq!(e.undo.len(), 1);
}

#[test]
fn te_high_level_overwrite_lf_gate_and_failed_insertion_keeps_prior_deletion() {
    let mut e = editor(0, 0x100 | 0x4000);
    let mut f = Services {
        composed: 1,
        ..Services::default()
    };
    text(&mut e, b"abc");
    e.position.index = 1;
    e.preferred_x = 12;
    e.insert_high_level_bytes(b"Z", &mut f).unwrap();
    assert_eq!(e.text, b"aZc");
    assert_eq!(e.undo[0].restore, b"b");
    assert_eq!(e.preferred_x, u32::MAX);
    e.flags |= 0x200;
    e.position.anchor = 0;
    e.position.active = 3;
    e.insert_high_level_bytes(b"\nrest", &mut f).unwrap();
    assert_eq!(e.text, b"aZc");
    assert_eq!((e.position.anchor, e.position.active), (2, 2));
    e.position.anchor = 0;
    e.position.active = 3;
    e.set_maximum(0);
    e.insert_high_level_bytes(b"x", &mut f).unwrap();
    assert!(e.text.is_empty()); // no speculative rollback
    assert_eq!(e.undo.len(), 1); // zero insertion does not record removed text
    assert_eq!(f.reasons, [1, 1]); // raw truncation, then high-level rejection
}

#[test]
fn te_pending_malformed_sequences_tab_capture_and_cr_vectors_48_49() {
    let mut e = editor(8, 1);
    let mut f = Services::default();
    assert_eq!(e.character_bytes(0xc0, false, &mut f), Ok(true));
    assert_eq!(e.pending, [0xc0]);
    assert_eq!(e.character_bytes(b'\t', false, &mut f), Ok(true));
    assert_eq!(e.pending, [0xc0]);
    assert_eq!(e.character_bytes(b'A', false, &mut f), Ok(false));
    assert_eq!(e.text, [0xc0, b'A']);
    assert!(e.pending.is_empty());
    assert_eq!(e.character_bytes(0xe0, false, &mut f), Ok(true));
    assert_eq!(e.character_bytes(b'A', false, &mut f), Ok(true));
    assert_eq!(e.character_bytes(b'B', false, &mut f), Ok(false));
    assert!(e.text.ends_with(&[0xe0, b'A', b'B']));
    let old = e.text.clone();
    assert_eq!(e.character_bytes(b'X', true, &mut f), Ok(false));
    assert_eq!(e.text, old);
    assert!(e.pending.is_empty());
    assert_eq!(e.character_bytes(b'\r', false, &mut f), Ok(false));
    assert!(e.text.ends_with(b"\n"));
    e.flags = 0;
    assert_eq!(e.character_bytes(b'Z', false, &mut f), Ok(false));
    assert!(!e.text.ends_with(b"Z"));
}

#[test]
fn te_word_navigation_uses_only_space_lf_and_lf_repair_keeps_line_selection() {
    let mut e = editor(0, 0);
    text(&mut e, b"one,two  three\tfour\n");
    e.word_step(1).unwrap();
    assert_eq!(e.position.index, 9);
    e.word_step(1).unwrap();
    assert_eq!(e.position.index, e.text.len() as u32);
    e.word_step(-1).unwrap();
    assert_eq!(e.position.index, 9);
    e.word_step(-99).unwrap();
    assert_eq!(e.position.index, 0);
    e.position = Position {
        line: 1,
        column: 5,
        index: e.text.len() as u32,
        anchor: 2,
        active: 9,
    };
    e.correct_lf();
    assert_eq!(
        (e.position.line, e.position.column, e.position.anchor, e.position.active),
        (1, 4, 2, 9)
    );
    assert!(!separator(b'\t') && !separator(b',') && separator(b'\n'));
}
