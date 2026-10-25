// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::atomic::{AtomicI32, Ordering};

#[test]
fn te10_fresh_runtime_defaults() {
    let e = TextEdit::new(
        1,
        2,
        0,
        Encoding {
            selector: 0,
            system_selector: 932,
        },
    );
    let r = e.runtime;
    assert_eq!([r.caret_color, r.highlight_color, r.outline_color], [0xff00_0000; 3]);
    assert_eq!(
        (r.visible, r.rows, r.horizontal_byte, r.horizontal_pixel),
        ([0, 0], 0, 0, 0)
    );
    assert_eq!((r.line_height, r.background_option), (1, 1));
}

#[test]
fn te9_history_limit_and_preferred_x() {
    let e = TextEdit::new(
        1,
        2,
        0,
        Encoding {
            selector: 0,
            system_selector: 932,
        },
    );
    assert_eq!((e.history_limit, e.preferred_x), (0x4000, u32::MAX));
}

#[test]
fn te10_construction_preserves_sound_words() {
    let words = [0x1234, 9, 0xfeed, u32::MAX];
    assert_eq!(runtime::Runtime::with_sound_words(words).sound_ids, words);
}

#[test]
fn te11_nested_editor_still_lays_out_without_automatic_check() {
    let counter = layout::ReflowCounter(AtomicI32::new(0));
    let mut b = TextEdit::new(
        3,
        4,
        0,
        Encoding {
            selector: 0,
            system_selector: 932,
        },
    );
    b.initialized = true;
    b.runtime.bars[0] = Some(8); // An automatic check would remove this bar.
    b.runtime.size = [100, 50];
    let mut f = Services::default();
    counter
        .check(|| {
            assert_eq!(counter.0.load(Ordering::Relaxed), 1);
            b.recalculate_with_counter(&mut f, &counter)
        })
        .unwrap();
    assert_eq!(b.runtime.line_height, 16);
    assert_eq!(b.runtime.bars[0], Some(8));
    assert_eq!(counter.0.load(Ordering::Relaxed), 0);
}

#[test]
fn te11_signed_threshold_and_nonreturning_check() {
    for initial in [-1, 1] {
        let counter = layout::ReflowCounter(AtomicI32::new(initial));
        let mut called = false;
        counter
            .check(|| {
                called = true;
                assert_eq!(counter.0.load(Ordering::Relaxed), 0);
                Ok(())
            })
            .unwrap();
        assert_eq!(called, initial == -1);
        assert_eq!(counter.0.load(Ordering::Relaxed), initial);
    }
    let counter = layout::ReflowCounter(AtomicI32::new(0));
    assert!(counter
        .check(|| Err(Unsupported::Precondition("dependency did not return")))
        .is_err());
    assert_eq!(counter.0.load(Ordering::Relaxed), 1);
}

#[test]
fn te12_direct_and_edge_source_destination_mode() {
    let mut e = TextEdit::new(
        1,
        2,
        0x20,
        Encoding {
            selector: 0,
            system_selector: 932,
        },
    );
    e.runtime.font = Some(7);
    e.runtime.background = Some(20);
    e.runtime.bars = [Some(8), Some(9)];
    let mut f = Services {
        image_source: Some([0, 0, 30, 18]),
        bar_size: Some((7, 5)),
        ..Services::default()
    };
    e.runtime.background_option = 0;
    e.paint([0, 0, 100, 60], &mut f).unwrap();
    assert_eq!(
        f.paints[0],
        runtime::Paint::Image {
            image: 20,
            source: [0, 0, 30, 18],
            area: [0, 0, 100, 60],
            arguments: [0, 0]
        }
    );
    f.paints.clear();
    e.runtime.background_option = 1;
    e.paint([0, 0, 100, 60], &mut f).unwrap();
    assert_eq!(
        f.paints[0],
        runtime::Paint::EdgeImage {
            image: 20,
            source: [0, 0, 10, 6],
            area: [0, 0, 93, 55],
            mode: 1
        }
    );
    f.paints.clear();
    f.image_source = Some([5, 7, -31, -20]);
    e.paint([0, 0, 100, 60], &mut f).unwrap();
    assert_eq!(
        f.paints[0],
        runtime::Paint::EdgeImage {
            image: 20,
            source: [5, 7, -10, -6],
            area: [0, 0, 93, 55],
            mode: 1
        }
    );
}

#[test]
fn te8_reverse_search_keeps_overlapping_matches() {
    let mut e = TextEdit::new(
        1,
        2,
        0,
        Encoding {
            selector: 0,
            system_selector: 0,
        },
    );
    e.text = b"ababa".to_vec();
    assert_eq!(e.find_text(b"aba", 4, false, &mut Services::default()), 2);
}
#[test]
fn te8_forward_search_from_interior() {
    assert_eq!(adapters::search(b"ababa", b"aba", 1, true), 2);
}
#[test]
fn te8_empty_forward_search_owner_rejects_end() {
    let mut e = super::editor(0, 0);
    e.text = b"abc".to_vec();
    for (position, expected) in [(2, 2), (3, u32::MAX)] {
        assert_eq!(e.find_text(b"", position, true, &mut Services::default()), expected);
    }
}
#[test]
fn te8_empty_reverse_search_owner_rejects_end() {
    let mut e = super::editor(0, 0);
    e.text = b"abc".to_vec();
    for (position, expected) in [(1, 1), (u32::MAX, u32::MAX)] {
        assert_eq!(e.find_text(b"", position, false, &mut Services::default()), expected);
    }
    assert_eq!(adapters::search(b"a\0ba\0b", b"\0b", u32::MAX, false), 4);
}
#[test]
fn te8_filter_fixed_count_nul_padding() {
    let e = TextEdit::new(
        1,
        2,
        0x200,
        Encoding {
            selector: 0,
            system_selector: 0,
        },
    );
    let mut input = *b"a\0\nb";
    let mut count = 4;
    assert_eq!(e.filter_buffer(&mut input, &mut count).unwrap(), 1);
    assert_eq!((input, count), ([0x61, 0, 0, 0], 3));
    let mut input = *b"a\nb";
    let mut count = 3;
    assert_eq!(e.filter_buffer(&mut input, &mut count).unwrap(), 1);
    assert_eq!((input, count), ([0x61, 0x62, 0], 2));
}

#[derive(Default)]
struct Storage {
    events: Vec<String>,
    copies: usize,
    stop: Option<usize>,
    null: bool,
}
impl history::Storage for Storage {
    fn allocate(&mut self, count: u32, bytes: u32) -> u32 {
        self.events.push(format!("allocate:{count}:{bytes}"));
        if self.null {
            0
        } else {
            0x2000
        }
    }
    fn construct(&mut self, address: u32, source: &Action) -> std::result::Result<Action, history::Failure> {
        self.events.push(format!("copy:{address}:{}", source.at));
        self.copies += 1;
        if self.stop == Some(self.copies) {
            Err(history::Failure::DependencyStopped)
        } else {
            Ok(source.clone())
        }
    }
    fn destroy(&mut self, source: &Action) {
        self.events.push(format!("destroy:{}", source.at));
    }
    fn free(&mut self, address: u32) {
        self.events.push(format!("free:{address}"));
    }
}
fn action(at: u32) -> Action {
    Action {
        at,
        erase: at + 1,
        restore: vec![at as u8],
        position: Position {
            anchor: 9,
            active: 1,
            ..Position::default()
        },
    }
}

#[test]
fn te8_history_growth_capacity_copy_ownership_commit_order() {
    let mut h = history::History::default();
    h.push(action(1));
    assert_eq!(h.capacity(), 1);
    h.push(action(2));
    assert_eq!(h.capacity(), 2);
    let source = action(3);
    let mut storage = Storage::default();
    h.append_with(&source, &mut storage).unwrap();
    assert_eq!(h.capacity(), 4);
    assert_eq!(&**h, &[action(1), action(2), action(3)]);
    assert_ne!(h[2].restore.as_ptr(), source.restore.as_ptr());
    assert_eq!(
        storage.events,
        [
            "allocate:4:240",
            "copy:8192:1",
            "copy:8252:2",
            "copy:8312:3",
            "destroy:1",
            "destroy:2",
            "free:4096"
        ]
    );
}
#[test]
fn te8_history_stopped_second_copy_keeps_old_and_leaks_prefix() {
    let mut h = history::History::default();
    h.push(action(1));
    h.push(action(2));
    let pointer = h.as_ptr();
    let string = h[0].restore.as_ptr();
    let mut storage = Storage {
        stop: Some(2),
        ..Storage::default()
    };
    let preceding_edit = b"already edited".to_vec();
    assert_eq!(
        h.append_with(&action(3), &mut storage),
        Err(history::Failure::DependencyStopped)
    );
    assert_eq!(h.as_ptr(), pointer);
    assert_eq!(h[0].restore.as_ptr(), string);
    assert_eq!(h.len(), 2);
    assert_eq!(h.capacity(), 2);
    assert_eq!(preceding_edit, b"already edited");
    assert_eq!(storage.events, ["allocate:4:240", "copy:8192:1", "copy:8252:2"]);
}
#[test]
fn te8_spare_capacity_stop_and_null_allocation_are_not_empty_history() {
    let mut h = history::History::default();
    for at in 0..3 {
        h.push(action(at));
    }
    let mut storage = Storage {
        stop: Some(1),
        ..Storage::default()
    };
    assert!(h.append_with(&action(4), &mut storage).is_err());
    assert_eq!(h.len(), 3);
    assert_eq!(storage.events, ["copy:4276:4"]);
    let mut empty = history::History::default();
    let mut storage = Storage {
        null: true,
        ..Storage::default()
    };
    let failure = history::Failure::InvalidVector {
        begin: 0,
        end: 60,
        capacity: 60,
    };
    assert_eq!(empty.append_with(&action(1), &mut storage), Err(failure));
    assert_eq!(empty.invalid_state(), Some(failure));
    assert_eq!(storage.events, ["allocate:1:60"]);
}

#[test]
fn te8_range_growth_inserts_owned_strings_before_copying_suffix() {
    let mut h = history::History::default();
    h.push(action(1));
    h.push(action(5));
    let mut source = [action(2), action(3), action(4)];
    let mut storage = Storage::default();
    h.grow_insert_with(1, &source, &mut storage).unwrap();
    assert_eq!(h.capacity(), 5); // n + max(n, k), where k > n.
    assert_eq!(&**h, &[action(1), action(2), action(3), action(4), action(5)]);
    for (copy, original) in h[1..4].iter().zip(&source) {
        assert_ne!(copy.restore.as_ptr(), original.restore.as_ptr());
    }
    source[0].restore[0] = 99;
    assert_eq!(h[1].restore, [2]);
    assert_eq!(
        storage.events,
        [
            "allocate:5:300",
            "copy:8192:1",
            "copy:8252:2",
            "copy:8312:3",
            "copy:8372:4",
            "copy:8432:5",
            "destroy:1",
            "destroy:5",
            "free:4096"
        ]
    );
}

#[test]
fn te8_range_growth_uses_size_instead_of_old_capacity() {
    let mut h = history::History::default();
    for at in 1..=3 {
        h.push(action(at));
    }
    assert_eq!(h.capacity(), 4);
    let mut storage = Storage::default();
    h.grow_insert_with(2, &[action(4), action(5)], &mut storage).unwrap();
    assert_eq!(h.capacity(), 6); // n + max(n, k), not twice old capacity.
    assert_eq!(&**h, &[action(1), action(2), action(4), action(5), action(3)]);
    assert_eq!(storage.events[0], "allocate:6:360");
}

#[test]
fn te8_range_growth_stops_during_insert_or_suffix_without_cleanup() {
    for stopped_copy in [2, 3] {
        let mut h = history::History::default();
        h.push(action(1));
        h.push(action(3));
        let pointer = h.as_ptr();
        let string = h[0].restore.as_ptr();
        let mut storage = Storage {
            stop: Some(stopped_copy),
            ..Storage::default()
        };
        assert_eq!(
            h.grow_insert_with(1, &[action(2)], &mut storage),
            Err(history::Failure::DependencyStopped)
        );
        assert_eq!(h.as_ptr(), pointer);
        assert_eq!(h[0].restore.as_ptr(), string);
        assert_eq!(h.capacity(), 2);
        assert_eq!(&**h, &[action(1), action(3)]);
        let expected = ["allocate:4:240", "copy:8192:1", "copy:8252:2", "copy:8312:3"];
        assert_eq!(storage.events, expected[..stopped_copy + 1]);
    }
}

#[test]
fn te8_null_range_growth_attempts_low_addresses_before_committing() {
    for stop in [Some(1), Some(2), None] {
        let mut h = history::History::default();
        h.push(action(1));
        h.push(action(3));
        let pointer = h.as_ptr();
        let string = h[0].restore.as_ptr();
        let mut storage = Storage {
            stop,
            null: true,
            ..Storage::default()
        };
        let result = h.grow_insert_with(1, &[action(2)], &mut storage);
        if let Some(stopped_copy) = stop {
            assert_eq!(result, Err(history::Failure::DependencyStopped));
            assert_eq!(h.as_ptr(), pointer);
            assert_eq!(h[0].restore.as_ptr(), string);
            assert_eq!(h.capacity(), 2);
            assert_eq!(&**h, &[action(1), action(3)]);
            assert_eq!(h.invalid_state(), None);
            let expected = ["allocate:4:240", "copy:60:2", "copy:120:3"];
            assert_eq!(storage.events, expected[..stopped_copy + 1]);
        } else {
            let failure = history::Failure::InvalidVector {
                begin: 0,
                end: 180,
                capacity: 240,
            };
            assert_eq!(result, Err(failure));
            assert_eq!(h.invalid_state(), Some(failure));
            assert_eq!(
                storage.events,
                [
                    "allocate:4:240",
                    "copy:60:2",
                    "copy:120:3",
                    "destroy:1",
                    "destroy:3",
                    "free:4096"
                ]
            );
            let events = storage.events.clone();
            assert_eq!(h.append_with(&action(4), &mut storage), Err(failure));
            assert_eq!(h.grow_insert_with(0, &[action(4)], &mut storage), Err(failure));
            assert_eq!(storage.events, events);
        }
    }
}
