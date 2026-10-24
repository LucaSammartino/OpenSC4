// SPDX-License-Identifier: GPL-3.0-or-later
//! widget-toolkit.md section 10.1 and WT5, WT10-WT12 closure vectors.
use super::*;

#[derive(Default)]
struct Boundary {
    events: Vec<NativeOp>,
    seen: Vec<(Object, u32)>,
    action: u8,
    consumed: bool,
    delete_byte: Option<u8>,
    deleting: Vec<(Window, u32, u32)>,
    destroy_requests: usize,
}
impl Boundary {
    fn balance(&self, object: Object) -> i32 {
        self.events
            .iter()
            .map(|e| match *e {
                NativeOp::AddRef(o) if o == object => 1,
                NativeOp::Release(o) if o == object => -1,
                _ => 0,
            })
            .sum()
    }
}
impl Host for Boundary {
    fn event(&mut self, e: NativeOp) {
        self.events.push(e);
    }
    fn filter(&mut self, wins: &mut Windows, f: Object, w: Window, msg: Message) -> bool {
        self.seen.push((f, msg.0[0]));
        if f == 100 && self.action != 0 {
            let action = std::mem::take(&mut self.action);
            match action {
                1 => {
                    wins.add_filter(self, w, 300);
                }
                2 => {
                    assert!(wins.remove_filter(self, w, 200));
                    wins.do_message(self, w, Message::new(99, 0, 0, 0, 0));
                }
                3 => {
                    assert!(wins.remove_filter(self, w, 100));
                }
                4 => {
                    assert!(wins.remove_filter(self, w, 200));
                    assert!(wins.remove_filter(self, w, 200));
                }
                5 => {
                    assert!(wins.remove_filter(self, w, 200));
                    wins.plot_composite(self, w);
                    // Nested paint restores the saved true Boolean and cannot drain.
                    assert!(wins.get(w).filtering);
                    assert_eq!(wins.get(w).removed_filters, [200]);
                }
                6 => {
                    wins.add_filter(self, w, 300);
                    wins.do_message(self, w, Message::new(99, 0, 0, 0, 0));
                }
                7 => {
                    assert!(wins.remove_filter(self, w, 100));
                    assert!(wins.remove_filter(self, w, 200));
                }
                _ => unreachable!(),
            }
        }
        self.consumed
    }
    fn child_delete_byte(&mut self, wins: &mut Windows, _: Window, c: Window) -> u8 {
        self.delete_byte.unwrap_or_else(|| u8::from(wins.destroy(self, c)))
    }
    fn deleting_destructor(&mut self, wins: &mut Windows, w: Window, flags: u32) {
        self.deleting.push((w, flags, wins.get(w).references));
        wins.deleting_destructor(self, w, flags);
    }
    fn destroy_window(&mut self, wins: &mut Windows, w: Window) -> bool {
        self.destroy_requests += 1;
        if self.destroy_requests == 3 {
            // Let a manager that was busy for two requests become available.
            // Without this change ChildDeleteAll repeats the same head forever.
            wins.busy = 0;
        }
        wins.destroy(self, w)
    }
}
fn window() -> (Windows, Boundary, Window) {
    let mut wins = Windows::new(5);
    let mut host = Boundary::default();
    let w = wins.create(&mut host, 1, None);
    (wins, host, w)
}
fn msg() -> Message {
    Message::new(98, 0, 0, 0, 0)
}

#[test]
fn wt10_live_append_including_original_tail_and_nested_pass() {
    for tail in [false, true] {
        let (mut s, mut h, w) = window();
        s.add_filter(&mut h, w, 100);
        if !tail {
            s.add_filter(&mut h, w, 200);
        }
        h.action = 1;
        s.do_message(&mut h, w, msg());
        let expected = if tail {
            vec![(100, 98), (300, 98)]
        } else {
            vec![(100, 98), (200, 98), (300, 98)]
        };
        assert_eq!(h.seen, expected);
        assert_eq!(h.balance(300), 1);
        h.seen.clear();
        s.do_message(&mut h, w, msg());
        assert_eq!(h.seen, expected);
    }
    let (mut s, mut h, w) = window();
    s.add_filter(&mut h, w, 100);
    s.add_filter(&mut h, w, 200);
    h.action = 6;
    s.do_message(&mut h, w, msg());
    assert_eq!(
        h.seen,
        [(100, 98), (100, 99), (200, 99), (300, 99), (200, 98), (300, 98)]
    );
}

#[test]
fn wt10_nested_message_retains_pending_until_outer_paint() {
    let (mut s, mut h, w) = window();
    s.add_filter(&mut h, w, 100);
    s.add_filter(&mut h, w, 200);
    h.action = 2;
    s.do_message(&mut h, w, msg());
    assert_eq!(h.seen, [(100, 98), (100, 99), (200, 99), (200, 98)]);
    assert_eq!(s.get(w).filters(), [100, 200]);
    assert_eq!(s.get(w).removed_filters, [200]);
    assert_eq!(h.balance(200), 2);
    h.seen.clear();
    s.get_mut(w).flags &= !1;
    assert!(!s.plot_composite(&mut h, w));
    assert!(h.seen.is_empty());
    assert_eq!(h.balance(200), 2); // early rejection never drains
    s.get_mut(w).flags |= 1;
    s.plot_composite(&mut h, w);
    assert_eq!(h.seen, [(100, 22), (200, 22)]);
    assert_eq!(h.balance(200), 0);
    assert_eq!(s.get(w).filters(), [100]);
    assert!(s.get(w).removed_filters.is_empty());
    let last_release = h.events.iter().rposition(|e| *e == NativeOp::Release(200)).unwrap();
    let paint = h.events.iter().position(|e| *e == NativeOp::Paint(w)).unwrap();
    assert!(last_release < paint);
}

#[test]
fn wt10_consumed_paint_still_drains_self_removal() {
    let (mut s, mut h, w) = window();
    s.add_filter(&mut h, w, 100);
    h.action = 3;
    h.consumed = true;
    assert!(s.plot_composite(&mut h, w));
    assert_eq!(h.balance(100), 0);
    assert!(s.get(w).filters().is_empty());
    assert!(!h.events.contains(&NativeOp::Paint(w)));
}

#[test]
fn wt10_duplicates_and_nested_paint_boolean_restore() {
    let (mut s, mut h, w) = window();
    assert!(!s.remove_filter(&mut h, w, 200));
    assert!(s.add_filter(&mut h, w, 100));
    assert!(s.add_filter(&mut h, w, 200));
    assert!(s.add_filter(&mut h, w, 200));
    assert!(s.remove_filter(&mut h, w, 200));
    assert_eq!(h.balance(200), 1);
    h.action = 5;
    s.do_message(&mut h, w, msg());
    assert_eq!(h.seen, [(100, 98), (100, 22), (200, 22), (200, 98)]);
    assert_eq!(h.balance(200), 2);
    assert!(!s.get(w).filtering);
    s.plot_composite(&mut h, w);
    assert_eq!(h.balance(200), 0);
}

#[test]
fn wt10_drain_releases_all_active_entries_before_pending_list() {
    let (mut s, mut h, w) = window();
    s.add_filter(&mut h, w, 100);
    s.add_filter(&mut h, w, 200);
    h.action = 7;
    s.do_message(&mut h, w, msg());
    h.events.clear();
    s.plot_composite(&mut h, w);
    let releases: Vec<_> = h
        .events
        .iter()
        .filter_map(|e| match *e {
            NativeOp::Release(o) => Some(o),
            _ => None,
        })
        .collect();
    assert_eq!(releases, [100, 200, 100, 200]);
}

#[test]
#[should_panic(expected = "WT10 active sentinel erasure")]
fn wt10_repeated_singleton_removal_reaches_corruption_boundary() {
    let (mut s, mut h, w) = window();
    s.add_filter(&mut h, w, 100);
    s.add_filter(&mut h, w, 200);
    h.action = 4;
    s.do_message(&mut h, w, msg());
    assert_eq!(s.get(w).removed_filters, [200, 200]);
    assert_eq!(h.balance(200), 3);
    s.plot_composite(&mut h, w);
}

#[test]
fn wt11_type_one_propagates_manager_and_override_bytes() {
    let (mut s, mut h, w) = window();
    let c = s.create(&mut h, 1, None);
    s.child_add(&mut h, w, c);
    s.begin_busy();
    assert_eq!(s.do_message_byte(&mut h, w, Message::new(1, c as u32, 0, 0, 0)), 1);
    assert_eq!(s.get(w).children(), [c]);
    assert_eq!(s.condemned, [c]);
    h.delete_byte = Some(0x7e);
    assert_eq!(s.do_message_byte(&mut h, w, Message::new(1, c as u32, 0, 0, 0)), 0x7e);
    assert_eq!(s.send_byte(&mut h, w, Message::new(1, c as u32, 0, 0, 0)), 0x7e);
    h.delete_byte = None;
    s.end_busy(&mut h);
    assert!(!s.valid(c));
    assert_eq!(s.do_message_byte(&mut h, w, Message::new(1, c as u32, 0, 0, 0)), 0);
}

#[test]
fn wt12_query_interface_outputs_and_zero_constructor_count() {
    for iid in [1, 0x22ba0121, 0xe98b2f57] {
        let (mut s, mut h, w) = window();
        assert_eq!(s.get(w).references, 0);
        let mut out = Some(999);
        assert!(s.query_interface(w, iid, &mut out));
        assert_eq!(out, Some(w));
        assert_eq!(s.get(w).references, 1);
        assert!(!s.query_interface(w, 0, &mut out));
        assert_eq!(out, Some(w));
        assert_eq!(s.get(w).references, 1);
        s.release(&mut h, w);
        assert!(!s.valid(w));
    }
}

#[test]
fn wt12_reference_wrap_unsigned_release_and_deleting_argument() {
    for count in [0, 1, 0x80000000, u32::MAX] {
        let (mut s, mut h, w) = window();
        s.get_mut(w).references = count;
        let result = s.release(&mut h, w);
        if count <= 1 {
            assert_eq!(result, 0);
            assert_eq!(h.deleting, [(w, 1, count)]);
            assert!(!s.valid(w));
        } else {
            assert_eq!(result, count - 1);
            assert_eq!(s.get(w).references, count - 1);
            assert!(h.deleting.is_empty());
        }
    }
    let (mut s, mut h, w) = window();
    s.get_mut(w).references = u32::MAX;
    assert_eq!(s.add_ref(w), 0);
    assert_eq!(s.release(&mut h, w), 0);
    assert_eq!(h.deleting, [(w, 1, 0)]);
}

#[test]
fn wt12_manager_retained_cow_references_with_and_without_caller() {
    for caller in [false, true] {
        let (mut s, mut h, p) = window();
        let c = s.create(&mut h, 1, None);
        s.child_add(&mut h, p, c);
        if caller {
            s.add_ref(c);
        }
        let old = s.read_children(p);
        assert!(s.destroy(&mut h, c));
        assert!(s.get(p).children().is_empty());
        assert_eq!(&*old, &[c]);
        assert!(s.valid(c));
        assert_eq!(s.get(c).parent, None);
        assert!(!s.get(c).flag(0x4000));
        assert_eq!(s.get(c).references, 1 + u32::from(caller));
        s.unlock_children(&mut h, old);
        assert_eq!(s.valid(c), caller);
        if caller {
            s.release(&mut h, c);
        }
        assert!(!s.valid(c));
        assert_eq!(h.events.iter().filter(|e| **e == NativeOp::Unregister(c)).count(), 1);
    }
}

#[test]
fn wt12_raw_parent_storage_and_invalid_child_rejection() {
    let (mut s, mut h, w) = window();
    assert!(!s.child_add(&mut h, w, 0));
    assert!(!s.child_add(&mut h, w, 999));
    assert!(s.set_parent(&mut h, w, Some(w)));
    assert_eq!((s.get(w).parent, s.get(w).notification), (Some(w), Some(w)));
    assert_eq!(s.get(w).references, 0);
    // Do not traverse self/cyclic parents: native traversal can fail to terminate.
    s.set_parent(&mut h, w, None);
    s.get_mut(w).notification = None;
    assert!(s.set_parent(&mut h, w, Some(999)));
    assert_eq!(s.get(w).parent, Some(999));
    s.set_parent(&mut h, w, None);
    let c = s.create(&mut h, 1, None);
    assert!(s.child_add(&mut h, w, c));
    let count = s.get(c).references;
    assert!(!s.child_add(&mut h, w, c));
    assert_eq!(s.get(c).references, count);
}

#[test]
fn wt5_busy_requests_do_not_detach_and_delete_all_rereads_head() {
    let (mut s, mut h, p) = window();
    let c = s.create(&mut h, 1, None);
    s.child_add(&mut h, p, c);
    s.begin_busy();
    assert!(s.child_delete_all(&mut h, p));
    assert_eq!(h.destroy_requests, 3);
    assert!(s.get(p).children().is_empty());
    assert!(!s.valid(c));
    assert_eq!(s.condemned, [c]); // raw pointer, deduplicated, no queue reference
    s.end_busy(&mut h);
    assert!(s.condemned.is_empty());
}

#[test]
fn wt12_busy_ancestor_coverage_and_root_caller_ownership() {
    let (mut s, mut h, p) = window();
    s.init(&mut h, p);
    let c = s.create(&mut h, 1, None);
    s.child_add(&mut h, p, c);
    s.add_ref(p);
    s.begin_busy();
    s.destroy(&mut h, p);
    s.destroy(&mut h, c);
    assert_eq!(s.condemned, [p]);
    assert_eq!(s.get(c).parent, Some(p));
    assert_eq!(s.get(c).references, 1);
    s.end_busy(&mut h);
    assert!(s.valid(p));
    assert!(!s.valid(c));
    s.release(&mut h, p);
    assert!(!s.valid(p));
}

#[test]
fn wt12_deleting_destructor_flag_bit_zero_and_teardown_order() {
    for flags in [0, 2, 1, 3] {
        let (mut s, mut h, p) = window();
        s.init(&mut h, p);
        let c = s.create(&mut h, 1, None);
        s.child_add(&mut h, p, c);
        s.deleting_destructor(&mut h, p, flags);
        assert!(!s.valid(p));
        assert!(!s.valid(c));
        assert_eq!(s.windows.contains_key(&p), flags & 1 == 0);
        let child = h.events.iter().position(|e| *e == NativeOp::Unregister(c)).unwrap();
        let parent_drop = h
            .events
            .iter()
            .position(|e| *e == NativeOp::DropManagerReferences(p))
            .unwrap();
        let parent = h.events.iter().position(|e| *e == NativeOp::Unregister(p)).unwrap();
        assert!(child < parent_drop && parent_drop < parent);
    }
}
