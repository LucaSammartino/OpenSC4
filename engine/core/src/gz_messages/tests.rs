// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors. See LICENSE at the repository root.

//! Tests for §2-§4A of docs/specs/framework-gz-services.md.

use std::collections::HashMap;

use super::*;

#[derive(Clone, Debug)]
enum Act {
    Post(Token, bool),
    Add(Token, u32),
    Remove(Token, u32),
    Directed(Token, Token),
    Send(Token),
    Tick,
    Cancel(Token),
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Add(Token),
    Release(Token),
    Deliver(Token, Token),
    Window(Token, u32, usize),
    Type(u32),
    Pulse,
    Wait(u32),
    Close,
}

// The native flush guards are process-wide, so independent test fixtures
// must not concurrently suppress each other's flushes.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

/// Mock host: message types, reference counts, a delivery log and one-shot
/// re-entrant actions per target (on delivery and on release).
#[derive(Default)]
struct Host {
    types: HashMap<Token, u32>,
    refs: HashMap<Token, i64>,
    log: Vec<(Token, Token)>,
    on_deliver: HashMap<Token, Vec<Act>>,
    on_release: HashMap<Token, Vec<Act>>,
    pushes_on_wait: Vec<Token>,
    waits: u32,
    queues: Vec<MessageQueue2>,
    events: Vec<Event>,
    on_add: HashMap<Token, Vec<Act>>,
    on_type: Vec<Act>,
    equal: HashMap<(Token, Token), u32>,
    equal_calls: Vec<(Token, Token)>,
    other: Option<MessageServer2>,
    wait_results: Vec<u32>,
    skip_wait_push: u32,
}

impl Host {
    fn msg(&mut self, id: Token, ty: u32) -> Token {
        self.types.insert(id, ty);
        id
    }
    fn r(&self, t: Token) -> i64 {
        self.refs.get(&t).copied().unwrap_or(0)
    }
    fn all_zero(&self) -> bool {
        self.refs.values().all(|&v| v == 0)
    }
    fn run(&mut self, s: &mut MessageServer2, acts: Vec<Act>) {
        for a in acts {
            match a {
                Act::Post(m, f) => {
                    s.message_post(self, m, f);
                }
                Act::Add(t, ty) => {
                    s.add_notification(self, t, ty);
                }
                Act::Remove(t, ty) => {
                    s.remove_notification(self, t, ty);
                }
                Act::Directed(m, t) => {
                    s.post_directed(self, m, t);
                }
                Act::Send(m) => {
                    s.message_send(self, m);
                }
                Act::Tick => {
                    s.on_tick(self);
                }
                Act::Cancel(t) => {
                    s.cancel_directed(self, t);
                }
            }
        }
    }
}

impl MessageHost for Host {
    fn message_type(&mut self, msg: Token) -> u32 {
        self.types.get(&msg).copied().unwrap_or(0)
    }
    fn messages_equal(&mut self, a: Token, b: Token) -> bool {
        a == b
    }
    fn equality_result(&mut self, a: Token, b: Token) -> u32 {
        self.equal_calls.push((a, b));
        self.equal.get(&(a, b)).copied().unwrap_or(u32::from(a == b))
    }
    fn add_ref(&mut self, t: Token) {
        self.events.push(Event::Add(t));
        *self.refs.entry(t).or_default() += 1;
    }
    fn release(&mut self, t: Token) {
        self.events.push(Event::Release(t));
        *self.refs.entry(t).or_default() -= 1;
    }
    fn wait_signal(&mut self, queue: &mut MessageQueue2, timeout: u32) -> u32 {
        self.events.push(Event::Wait(timeout));
        assert_eq!(timeout, 1000);
        self.waits += 1;
        if self.skip_wait_push > 0 {
            self.skip_wait_push -= 1;
        } else if let Some(m) = self.pushes_on_wait.pop() {
            queue.push_back(self, m);
        }
        self.wait_results.pop().unwrap_or(0x102)
    }
    fn pulse_signal(&mut self) {
        self.events.push(Event::Pulse);
    }
    fn close_signal(&mut self) {
        self.events.push(Event::Close);
    }
}

impl Server2Host for Host {
    fn add_ref_target(&mut self, s: &mut MessageServer2, t: Token) {
        self.add_ref(t);
        if t == B && s.registry().targets(5).contains(&B) {
            if let Some(mut other) = self.other.take() {
                other.add_notification(self, W, 8);
                assert_eq!(other.registry().pending().count(), 1);
                self.other = Some(other);
            }
        }
        if self.on_add.contains_key(&t) {
            self.events
                .push(Event::Window(t, s.registry().depth(), s.registry().pending().count()));
            let acts = self.on_add.remove(&t).unwrap();
            self.run(s, acts);
        }
    }
    fn message_type_in_server(&mut self, s: &mut MessageServer2, m: Token) -> u32 {
        self.events.push(Event::Type(s.registry().depth()));
        let acts = std::mem::take(&mut self.on_type);
        self.run(s, acts);
        self.message_type(m)
    }
    fn do_message(&mut self, s: &mut MessageServer2, target: Token, msg: Token) {
        self.events.push(Event::Deliver(target, msg));
        self.log.push((target, msg));
        if let Some(acts) = self.on_deliver.remove(&target) {
            self.run(s, acts);
        }
    }
    fn release_target(&mut self, s: &mut MessageServer2, target: Token) {
        self.release(target);
        if let Some(acts) = self.on_release.remove(&target) {
            self.events.push(Event::Window(
                target,
                s.registry().depth(),
                s.registry().targets(5).len(),
            ));
            self.run(s, acts);
        }
    }
    fn create_standard_message(&mut self, iid: u32) -> Option<Token> {
        (iid == 7).then_some(99)
    }
}

impl QueueHost for Host {
    fn add_ref_queue(&mut self, q: Token) {
        self.queues[q as usize].add_ref();
    }
    fn release_queue(&mut self, q: Token) {
        let mut queue = std::mem::take(&mut self.queues[q as usize]);
        queue.release(self);
        self.queues[q as usize] = queue;
    }
    fn new_queue(&mut self) -> Token {
        self.queues.push(MessageQueue2::new());
        (self.queues.len() - 1) as Token
    }
}

const A: Token = 1000;
const B: Token = 1001;
const C: Token = 1002;
const W: Token = 1003;

#[test]
fn queue_push_peek_get_references_and_signal() {
    let mut h = Host::default();
    let m1 = h.msg(1, 5);
    let m2 = h.msg(2, 7);
    let mut q = MessageQueue2::new();
    assert_eq!(q.pulses(), 0);
    q.push_back(&mut h, m1);
    assert_eq!(q.pulses(), 1);
    q.push_front(&mut h, m2);
    assert_eq!(q.pulses(), 1, "pulse only on the empty to non-empty transition");
    assert_eq!(q.count(), 2);
    assert_eq!((q.front_type(&mut h), q.back_type(&mut h)), (7, 5));
    assert!(q.contains_type(&mut h, 5) && !q.contains_type(&mut h, 9));
    assert_eq!(q.peek_back(&mut h), Some(m1));
    assert_eq!(h.r(m1), 2, "peek AddRefs for the caller and keeps the message");
    assert_eq!(q.peek_equal(&mut h, m2), Some(m2));
    assert_eq!(q.peek_equal(&mut h, 3), None);
    assert_eq!(q.get_front(&mut h), Some(m2));
    assert_eq!(h.r(m2), 2, "get hands the queue's reference to the caller");
    assert_eq!(q.count(), 1);
    assert_eq!(q.remove_all(&mut h), 1);
    assert_eq!(h.r(m1), 1);
    assert_eq!(q.front_type(&mut h), 0);
    assert_eq!(q.peek_front(&mut h), None);
}

#[test]
fn queue_remove_variants_scan_in_the_specified_direction() {
    let mut h = Host::default();
    let ms: Vec<Token> = [(1, 5), (2, 6), (3, 5), (4, 5)]
        .iter()
        .map(|&(i, t)| h.msg(i, t))
        .collect();
    let mut q = MessageQueue2::new();
    for &m in &ms {
        q.push_back(&mut h, m);
    }
    assert!(q.remove_first_by_type(&mut h, 5));
    assert_eq!(q.messages().collect::<Vec<_>>(), vec![2, 3, 4]);
    assert!(q.remove_last_by_type(&mut h, 5));
    assert_eq!(q.messages().collect::<Vec<_>>(), vec![2, 3]);
    assert!(!q.remove_first_by_type(&mut h, 9));
    q.push_back(&mut h, 3);
    assert_eq!(q.remove_all_equal(&mut h, 3), 2);
    assert!(!q.remove_last_equal(&mut h, 3));
    assert!(q.remove_first_equal(&mut h, 2));
    for &m in &ms {
        q.push_back(&mut h, m);
    }
    assert_eq!(q.remove_all_by_type(&mut h, 5), 3);
    assert_eq!(q.get_equal(&mut h, 2), Some(2));
    assert_eq!(q.count(), 0);
    assert_eq!(h.r(1), 0);
    assert_eq!(h.r(2), 1, "the gotten message's reference is the caller's");
}

#[test]
fn queue_release_destroys_at_one_or_zero() {
    let mut h = Host::default();
    let mut q = MessageQueue2::new();
    q.push_back(&mut h, 1);
    assert_eq!(q.add_ref(), 1);
    assert_eq!(q.add_ref(), 2);
    assert_eq!(q.release(&mut h), 1);
    assert!(!q.is_destroyed());
    assert_eq!(q.release(&mut h), 0);
    assert!(q.is_destroyed());
    assert_eq!(h.r(1), 0, "destruction releases every queued message");
    let mut q0 = MessageQueue2::new();
    assert_eq!(q0.release(&mut h), 0);
    assert!(q0.is_destroyed(), "count 0 also destroys");
}

#[test]
fn queue_blocking_get_waits_until_filled() {
    let mut h = Host {
        pushes_on_wait: vec![2, 1],
        ..Host::default()
    };
    let mut q = MessageQueue2::new();
    assert_eq!(q.blocking_get_front(&mut h), 1);
    assert_eq!(h.waits, 1);
    assert_eq!(q.blocking_get_back(&mut h), 2);
    assert_eq!(h.waits, 2);
    q.push_back(&mut h, 3);
    assert_eq!(q.blocking_get_back(&mut h), 3);
    assert_eq!(h.waits, 2, "no wait when a message is there");
}

#[test]
fn manager_add_create_get_remove_ownership() {
    let mut h = Host::default();
    let mut m = QueueManager2::new();
    assert!(QueueManager2::answers_iid(QUEUE_MANAGER2_IID));
    assert!(m.create_queue(&mut h, 10));
    assert!(!m.create_queue(&mut h, 10));
    assert_eq!(h.queues.len(), 1, "nothing allocated when the id exists");
    let q0 = 0;
    assert_eq!(h.queues[0].ref_count(), 1);
    let ext = h.new_queue();
    h.queues[ext as usize].add_ref();
    h.queues[ext as usize].push_back(&mut Host::default(), 5);
    assert!(m.add_queue(&mut h, 3, ext));
    assert!(!m.add_queue(&mut h, 3, q0));
    assert_eq!(h.queues[ext as usize].ref_count(), 2);
    assert_eq!(m.entries().collect::<Vec<_>>(), vec![(3, ext), (10, q0)]);
    assert_eq!(m.get_queue(&mut h, 10), Some(q0));
    assert_eq!(h.queues[0].ref_count(), 2);
    assert_eq!(m.get_queue(&mut h, 4), None);
    assert!(m.remove_queue(&mut h, ext));
    assert!(!m.remove_queue(&mut h, ext));
    assert_eq!(h.queues[ext as usize].ref_count(), 1);
    assert!(!h.queues[ext as usize].is_destroyed());
    assert!(!m.remove_queue_id(&mut h, 3));
    assert!(m.remove_all_queues(&mut h));
    assert!(m.is_empty());
    assert_eq!(h.queues[0].ref_count(), 1, "the GetQueue reference survives");
    h.release_queue(q0);
    assert!(h.queues[0].is_destroyed());
    assert!(m.create_queue(&mut h, 1) && m.remove_queue_id(&mut h, 1));
    assert!(h.queues[2].is_destroyed(), "a queue nobody else holds is destroyed");
}

fn worked_example(drain_all: bool) -> (Vec<(Token, Token)>, u32) {
    let mut h = Host::default();
    let m1 = h.msg(1, 5);
    let m2 = h.msg(2, 7);
    let m3 = h.msg(3, 9);
    let mut s = MessageServer2::new();
    assert!(s.drain_all());
    s.set_drain_all(drain_all);
    s.add_notification(&mut h, A, 5);
    s.add_notification(&mut h, W, WILDCARD_TYPE);
    s.message_post(&mut h, m1, false);
    s.message_post(&mut h, m2, true);
    assert_eq!(s.posted_count(), 2);
    assert!(h.log.is_empty(), "posting never delivers immediately");
    h.on_deliver.insert(W, vec![Act::Post(m3, false)]);
    s.on_tick(&mut h);
    (h.log, s.posted_count())
}

#[test]
fn server2_worked_example_section5() {
    let _serial = serial();
    let (log, left) = worked_example(false);
    assert_eq!(log, vec![(W, 2), (A, 1), (W, 1)]);
    assert_eq!(left, 1, "m3 waits: taken reached the limit after m1");
    let (log, left) = worked_example(true);
    assert_eq!(log, vec![(W, 2), (A, 1), (W, 1), (W, 3)]);
    assert_eq!(left, 0);
}

#[test]
fn server2_send_order_wildcard_and_no_duplicates() {
    let _serial = serial();
    let mut h = Host::default();
    let m = h.msg(1, 5);
    let z = h.msg(2, 0);
    let mut s = MessageServer2::new();
    for (t, ty) in [(B, 5), (W, 0), (A, 5), (B, 5), (A, 0)] {
        assert!(s.add_notification(&mut h, t, ty));
    }
    assert_eq!(s.registry().targets(5), &[B, A]);
    assert_eq!(h.r(B), 1, "a duplicate takes no reference");
    assert!(s.message_send(&mut h, m));
    assert_eq!(h.log, vec![(B, 1), (A, 1), (W, 1), (A, 1)], "A gets it twice");
    h.log.clear();
    s.message_send(&mut h, z);
    assert_eq!(h.log, vec![(W, 2), (A, 2)], "type 0 is sent once to the wildcard list");
}

#[test]
fn server2_registration_during_delivery_does_not_change_it() {
    let _serial = serial();
    let mut h = Host::default();
    let m = h.msg(1, 5);
    let mut s = MessageServer2::new();
    s.add_notification(&mut h, A, 5);
    s.add_notification(&mut h, B, 5);
    h.on_deliver.insert(A, vec![Act::Remove(B, 5), Act::Add(C, 5)]);
    s.message_send(&mut h, m);
    assert_eq!(h.log, vec![(A, 1), (B, 1)]);
    assert_eq!(s.registry().targets(5), &[A, C]);
    assert_eq!((h.r(B), h.r(C)), (0, 1));
}

#[test]
fn server2_remove_frees_empty_list() {
    let _serial = serial();
    let mut h = Host::default();
    let mut s = MessageServer2::new();
    assert!(s.remove_notification(&mut h, A, 5), "true when nothing was registered");
    s.add_notification(&mut h, A, 5);
    s.add_notification(&mut h, A, 6);
    s.remove_notification(&mut h, A, 5);
    assert_eq!(s.registry().types().collect::<Vec<_>>(), vec![6]);
    assert_eq!(h.r(A), 1);
}

#[test]
fn server2_reentrant_release_goes_through_pending_and_flush() {
    let _serial = serial();
    let mut h = Host::default();
    let mut s = MessageServer2::new();
    s.add_notification(&mut h, A, 5);
    s.add_notification(&mut h, A, 6);
    // A's destructor-like release re-enters while the depth is 1.
    h.on_release.insert(A, vec![Act::Remove(A, 6), Act::Add(B, 7)]);
    s.remove_notification(&mut h, A, 5);
    assert_eq!(s.registry().pending().count(), 0, "flushed");
    assert_eq!(s.registry().types().collect::<Vec<_>>(), vec![7]);
    assert_eq!(s.registry().targets(7), &[B]);
    assert_eq!((h.r(A), h.r(B)), (0, 1));
    assert_eq!(s.registry().depth(), 0);
}

#[test]
fn server2_directed_delivery_and_cancel() {
    let _serial = serial();
    let mut h = Host::default();
    let mut s = MessageServer2::new();
    s.post_directed(&mut h, 1, A);
    s.post_directed(&mut h, 2, B);
    // Prefix erasures preserve the native iterator; cancelling a last/sole
    // match is deliberately outside the supported native preconditions.
    s.post_directed(&mut h, 3, C);
    s.post_directed(&mut h, 6, W);
    s.cancel_directed(&mut h, A);
    assert_eq!((h.r(A), h.r(1), h.r(3)), (0, 0, 1));
    s.post_directed(&mut h, 4, C);
    h.on_deliver.insert(B, vec![Act::Directed(5, A)]);
    s.on_tick(&mut h);
    assert_eq!(h.log, vec![(B, 2), (C, 3), (W, 6), (C, 4)]);
    assert_eq!(
        s.directed().collect::<Vec<_>>(),
        vec![(A, 5)],
        "added during delivery: next tick"
    );
    assert_eq!(s.countdown(), -1);
    s.on_tick(&mut h);
    assert_eq!(h.log.last(), Some(&(A, 5)));
    assert!(h.all_zero());
}

#[test]
fn server2_create_message_and_destroy() {
    let _serial = serial();
    let mut h = Host::default();
    let mut s = MessageServer2::new();
    assert!(MessageServer2::answers_iid(MESSAGE_SERVER2_IID));
    assert_eq!(s.create_message(&mut h, STANDARD_MESSAGE_CLSID, 7), Some(99));
    assert_eq!(s.create_message(&mut h, STANDARD_MESSAGE_CLSID, 8), None);
    assert_eq!(s.create_message(&mut h, 1, 7), None);
    s.add_notification(&mut h, A, 5);
    s.message_post(&mut h, 1, false);
    s.post_directed(&mut h, 2, B);
    s.destroy(&mut h);
    assert!(h.all_zero());
    assert_eq!(s.posted_count(), 0);
}

#[derive(Default)]
struct Host1 {
    refs: HashMap<Token, i64>,
    log: Vec<(Token, Record, i64)>,
    post_on: Option<(Token, Record)>,
    directed_action: Option<bool>,
}

impl Server1Host for Host1 {
    fn add_ref_target(&mut self, t: Token) {
        *self.refs.entry(t).or_default() += 1;
    }
    fn release_target(&mut self, _s: &mut MessageServer1, t: Token) {
        *self.refs.entry(t).or_default() -= 1;
    }
    fn do_message(&mut self, s: &mut MessageServer1, t: Token, msg: &Record) {
        let held = self.refs.get(&t).copied().unwrap_or(0);
        self.log.push((t, *msg, held));
        if t == A {
            if let Some(nested) = self.directed_action.take() {
                if nested {
                    s.on_tick(self);
                } else {
                    s.cancel_directed(self, B);
                    s.post_directed(self, &[6, 0, 0, 0], 1005);
                }
            }
        }
        if let Some((pt, rec)) = self.post_on {
            if pt == t {
                self.post_on = None;
                s.message_post(&rec, false);
            }
        }
    }
}

#[test]
fn server1_no_wildcard_type0_ignored_and_snapshot_refs() {
    let _serial = serial();
    let mut h = Host1::default();
    let mut s = MessageServer1::new();
    s.add_notification(&mut h, A, 5);
    s.add_notification(&mut h, W, 0);
    let rec = [5, 1, 2, 3];
    s.message_send(&mut h, &rec);
    assert_eq!(h.log, vec![(A, rec, 2)], "AddRef'd during its call; no wildcard pass");
    s.message_send(&mut h, &[0, 0, 0, 0]);
    assert_eq!(h.log.len(), 1, "type 0 delivers to nobody");
    assert_eq!(h.refs[&A], 1);
}

#[test]
fn server1_tick_drain_modes_and_limit() {
    let _serial = serial();
    let mut h = Host1::default();
    let mut s = MessageServer1::new();
    s.set_drain_all(false);
    s.add_notification(&mut h, A, 5);
    s.message_post(&[5, 1, 0, 0], false);
    s.message_post(&[5, 2, 0, 0], true);
    h.post_on = Some((A, [5, 3, 0, 0]));
    s.on_tick(&mut h);
    let seen: Vec<u32> = h.log.iter().map(|e| e.1[1]).collect();
    assert_eq!(seen, vec![2, 1]);
    assert_eq!(s.posted_count(), 1);
    for t in 0..400 {
        s.add_notification(&mut h, 10_000 + t, 9);
    }
    h.log.clear();
    s.message_send(&mut h, &[9, 0, 0, 0]);
    assert_eq!(h.log.len(), SERVER1_SNAPSHOT_LIMIT);
    s.post_directed(&mut h, &[1, 2, 3, 4], B);
    s.on_tick(&mut h);
    assert_eq!(h.log.last().map(|e| (e.0, e.1)), Some((B, [1, 2, 3, 4])));
    s.destroy(&mut h);
    assert!(h.refs.values().all(|&v| v == 0));
}

#[test]
fn gz_v01_shared_count_is_separate_from_posted_and_taken() {
    let mut s = MessageServer2::new();
    let mut h = Host::default();
    assert_eq!(s.ref_count(), 0);
    assert_eq!(s.remove_ref(), 0);
    assert_eq!(s.add_ref(), 1);
    assert_eq!(s.add_ref(), 2); // both interface adapters forward to this count
    s.message_post(&mut h, 7, false);
    assert_eq!((s.ref_count(), s.posted_count(), s.taken()), (2, 1, 0));
    assert_eq!(s.remove_ref(), 1);
    assert_eq!(s.remove_ref(), 0);
    assert_eq!(s.remove_ref(), 0);
}

#[test]
fn gz_v02_unsigned_wait_units_and_os_results() {
    for (us, ms) in [
        (0, 0),
        (999, 0),
        (1_000, 1),
        (1_000_000, 1000),
        (u32::MAX - 1, 4_294_967),
        (u32::MAX, u32::MAX),
    ] {
        assert_eq!(signal_wait_millis(us), ms);
    }
    for (os, result) in [(u32::MAX, 0), (0x102, 2), (0, 1), (0x80, 1), (17, 1)] {
        assert_eq!(signal_wait_result(os), result);
    }
}

#[test]
fn gz_v03_lost_pulse_does_not_latch_and_queue_closes_before_releases() {
    let mut q = MessageQueue2::new();
    let mut h = Host::default();
    assert_eq!(q.get_front(&mut h), None);
    // Producer wins the get/wait gap, with nobody waiting for this pulse.
    q.push_back(&mut h, 1);
    assert_eq!(h.wait_signal(&mut q, 1000), 0x102);
    assert_eq!(q.get_front(&mut h), Some(1));
    assert_eq!(q.pulses(), 1);
    assert_eq!(
        h.events,
        vec![
            Event::Add(1),
            Event::Pulse,
            Event::Wait(1000),
            Event::Add(1),
            Event::Release(1)
        ]
    );
    h.release(1);
    q.push_back(&mut h, 2);
    h.events.clear();
    q.destroy(&mut h);
    assert_eq!(h.events, vec![Event::Close, Event::Release(2)]);
    assert!(h.all_zero());
}

#[test]
fn gz_v04_all_five_equal_operations_use_queued_receiver_and_low_byte() {
    for operation in 0..5 {
        let mut h = Host::default();
        let mut q = MessageQueue2::new();
        for m in 1..=3 {
            q.push_back(&mut h, m);
        }
        h.equal.insert((1, 9), 0x100); // high bits alone are false
        h.equal.insert((2, 9), 1);
        h.equal.insert((3, 9), 0x101);
        match operation {
            0 => {
                assert!(q.remove_first_equal(&mut h, 9));
                assert_eq!(q.messages().collect::<Vec<_>>(), vec![1, 3]);
            }
            1 => {
                assert!(q.remove_last_equal(&mut h, 9));
                assert_eq!(q.messages().collect::<Vec<_>>(), vec![1, 2]);
            }
            2 => {
                assert_eq!(q.remove_all_equal(&mut h, 9), 2);
                assert_eq!(q.messages().collect::<Vec<_>>(), vec![1]);
            }
            3 => {
                assert_eq!(q.peek_equal(&mut h, 9), Some(2));
                assert_eq!(h.r(2), 2);
                assert_eq!(q.count(), 3);
                h.release(2);
            }
            4 => {
                assert_eq!(q.get_equal(&mut h, 9), Some(2));
                assert_eq!(h.r(2), 1);
                assert_eq!(q.count(), 2);
                h.release(2);
            }
            _ => unreachable!(),
        }
        assert!(h.equal_calls.iter().all(|&(receiver, arg)| receiver != 9 && arg == 9));
        q.destroy(&mut h);
        assert!(h.all_zero());
    }
}

#[test]
fn gz_v05_snapshot_holds_last_through_every_delivery_then_release_forward() {
    let _serial = serial();
    let mut h = Host::default();
    let mut s = MessageServer2::new();
    h.msg(1, 5);
    for (t, ty) in [(A, 5), (B, 5), (A, 0)] {
        s.add_notification(&mut h, t, ty);
    }
    h.on_deliver.insert(A, vec![Act::Remove(B, 5)]);
    h.events.clear();
    s.message_send(&mut h, 1);
    assert_eq!(
        h.events,
        vec![
            Event::Type(1),
            Event::Add(A),
            Event::Add(B),
            Event::Add(A),
            Event::Deliver(A, 1),
            Event::Release(B),
            Event::Deliver(B, 1),
            Event::Deliver(A, 1),
            Event::Release(A),
            Event::Release(B),
            Event::Release(A)
        ]
    );
    assert_eq!((h.r(A), h.r(B)), (2, 0));
    s.destroy(&mut h);
    assert!(h.all_zero());
}

#[test]
fn gz_v06_unlink_release_defer_and_replay_before_pending_release() {
    let _serial = serial();
    let mut s = MessageServer2::new();
    let mut h = Host::default();
    s.add_notification(&mut h, A, 5);
    h.on_release.insert(A, vec![Act::Add(B, 5)]);
    h.events.clear();
    s.remove_notification(&mut h, A, 5);
    assert_eq!(
        h.events,
        vec![
            Event::Release(A),
            Event::Window(A, 1, 0),
            Event::Add(B),
            Event::Add(B),
            Event::Add(B),
            Event::Release(B),
            Event::Release(B),
            Event::Add(B),
            Event::Release(B)
        ]
    );
    assert_eq!(s.registry().targets(5), &[B]);
    assert_eq!(s.registry().pending().count(), 0);
    assert_eq!(h.r(B), 1);
    s.destroy(&mut h);
    assert!(h.all_zero());
}

#[test]
fn gz_v07_get_type_reentry_applies_on_next_send_before_depth_increment() {
    let _serial = serial();
    let mut s = MessageServer2::new();
    let mut h = Host::default();
    h.msg(1, 5);
    s.add_notification(&mut h, A, 5);
    h.on_type = vec![Act::Add(B, 5)];
    s.message_send(&mut h, 1);
    assert_eq!(h.log, vec![(A, 1)]);
    assert_eq!(s.registry().pending().count(), 1);
    h.events.clear();
    s.message_send(&mut h, 1);
    assert_eq!(h.log, vec![(A, 1), (A, 1), (B, 1)]);
    assert_eq!(&h.events[..3], &[Event::Add(B), Event::Release(B), Event::Type(1)]);
    assert_eq!((h.r(A), h.r(B)), (1, 1));
    s.destroy(&mut h);
    assert!(h.all_zero());
}

#[test]
fn gz_v08_generation1_directed_records_are_copied_and_targets_borrowed() {
    let mut s = MessageServer1::new();
    let mut h = Host1::default();
    let mut record = [1, 2, 3, 4];
    s.post_directed(&mut h, &record, A);
    record.fill(99);
    assert!(h.refs.is_empty());
    s.deliver_directed(&mut h);
    assert_eq!(h.log, vec![(A, [1, 2, 3, 4], 0)]);
    assert!(h.refs.is_empty());
    for t in [A, A, B, C, W] {
        s.post_directed(&mut h, &record, t);
    }
    s.cancel_directed(&mut h, A);
    s.destroy(&mut h);
    assert!(h.refs.is_empty());
}

#[test]
fn gz_v09_generation2_persistent_local_pair_reference_order() {
    let mut s = MessageServer2::new();
    let mut h = Host::default();
    s.post_directed(&mut h, 1, A);
    s.post_directed(&mut h, 2, B);
    h.events.clear();
    s.deliver_directed(&mut h);
    assert_eq!(
        h.events,
        vec![
            Event::Add(A),
            Event::Add(1),
            Event::Release(1),
            Event::Release(A),
            Event::Deliver(A, 1),
            Event::Add(B),
            Event::Release(A),
            Event::Add(2),
            Event::Release(1),
            Event::Release(2),
            Event::Release(B),
            Event::Deliver(B, 2),
            Event::Release(2),
            Event::Release(B)
        ]
    );
    assert_eq!(s.countdown(), -1);
    assert!(h.all_zero());
}

#[test]
fn gz_v10_prefix_and_v11_suffix_skip_in_both_generations() {
    for (targets, cancelled, remaining) in [
        (vec![A, A, B, C, W], 2, vec![B, C, W]),
        (vec![B, C, A, A], 1, vec![B, C, A]),
    ] {
        let mut s = MessageServer2::new();
        let mut h = Host::default();
        let mut raw = MessageServer1::new();
        let mut rh = Host1::default();
        for (i, &t) in targets.iter().enumerate() {
            s.post_directed(&mut h, i as Token + 1, t);
            raw.post_directed(&mut rh, &[i as u32, 7, 8, 9], t);
        }
        s.cancel_directed(&mut h, A);
        raw.cancel_directed(&mut rh, A);
        assert_eq!(targets.len() - s.directed().count(), cancelled as usize);
        assert_eq!(s.directed().map(|p| p.0).collect::<Vec<_>>(), remaining);
        assert_eq!(raw.directed().map(|p| p.0).collect::<Vec<_>>(), remaining);
        assert_eq!(h.r(A), i64::from(2 - cancelled));
        s.deliver_directed(&mut h);
        raw.deliver_directed(&mut rh);
        assert_eq!(h.log.iter().map(|p| p.0).collect::<Vec<_>>(), remaining);
        assert!(h.all_zero());
        assert!(rh.refs.is_empty());
    }
}

#[test]
fn gz_v12_cancel_leaves_quota_for_new_post_and_v13_nested_countdown() {
    let _serial = serial();
    let mut s = MessageServer2::new();
    let mut h = Host::default();
    for (i, t) in [A, B, C, W, 1004].into_iter().enumerate() {
        s.post_directed(&mut h, i as Token + 1, t);
    }
    h.on_deliver.insert(A, vec![Act::Cancel(B), Act::Directed(6, 1005)]);
    s.on_tick(&mut h);
    assert_eq!(h.log, vec![(A, 1), (C, 3), (W, 4), (1004, 5), (1005, 6)]);
    assert_eq!(s.countdown(), -1);
    assert!(h.all_zero());
    h.log.clear();
    for (m, t) in [(1, A), (2, B), (3, C)] {
        s.post_directed(&mut h, m, t);
    }
    h.on_deliver.insert(A, vec![Act::Tick]);
    s.on_tick(&mut h);
    assert_eq!(h.log, vec![(A, 1), (B, 2), (C, 3)]);
    assert_eq!(s.countdown(), -2);
    assert!(h.all_zero());
    s.deliver_directed(&mut h);
    assert_eq!(
        s.countdown(),
        -1,
        "empty direct invocation resets nonpositive quota then decrements"
    );
}

#[derive(Default)]
struct Pending1 {
    refs: HashMap<Token, i64>,
    action: Option<bool>,
    seen: Vec<Token>,
    remove_pending: bool,
}
impl Server1Host for Pending1 {
    fn add_ref_target(&mut self, t: Token) {
        *self.refs.entry(t).or_default() += 1;
    }
    fn add_ref_target_in_server(&mut self, s: &mut MessageServer1, t: Token) {
        self.add_ref_target(t);
        if t == A {
            if std::mem::take(&mut self.remove_pending) {
                let before = self.refs.get(&B).copied().unwrap_or(0);
                s.remove_notification(self, B, 5);
                assert_eq!(self.refs.get(&B).copied().unwrap_or(0), before);
                assert_eq!(s.registry().pending().count(), 1);
            }
            if let Some(nested_send) = self.action.take() {
                assert_eq!(s.registry().depth(), 1);
                s.add_notification(self, B, 5);
                s.add_notification(self, B, 5);
                assert_eq!(s.registry().pending().count(), 2);
                if nested_send {
                    s.message_send(self, &[5, 0, 0, 0]);
                }
            }
        }
    }
    fn release_target(&mut self, _s: &mut MessageServer1, t: Token) {
        *self.refs.entry(t).or_default() -= 1;
    }
    fn do_message(&mut self, _s: &mut MessageServer1, t: Token, _m: &Record) {
        self.seen.push(t);
    }
}

#[test]
fn gz_v14_generation1_flush_vs_direct_apply_duplicate_hold_leak() {
    let _serial = serial();
    for nested in [false, true] {
        let mut h = Pending1 {
            action: Some(nested),
            ..Pending1::default()
        };
        let mut s = MessageServer1::new();
        s.add_notification(&mut h, A, 9);
        assert_eq!(s.registry().targets(5), &[B]);
        assert_eq!(s.registry().pending().count(), 0);
        assert_eq!(h.refs[&B], if nested { 2 } else { 1 });
        assert_eq!(h.seen, if nested { vec![B] } else { vec![] });
        s.destroy(&mut h);
        assert_eq!(
            h.refs[&B],
            i64::from(nested),
            "native duplicate pending-add leak is preserved"
        );
    }
}

#[test]
fn snapshot_addref_can_defer_and_nested_send_applies_pending() {
    let _serial = serial();
    let mut s = MessageServer2::new();
    let mut h = Host::default();
    h.msg(1, 5);
    s.add_notification(&mut h, A, 5);
    h.on_add.insert(A, vec![Act::Add(B, 8), Act::Send(1)]);
    s.message_send(&mut h, 1);
    assert_eq!(h.log, vec![(A, 1), (A, 1)]);
    assert_eq!(s.registry().targets(8), &[B]);
    assert_eq!(s.registry().pending().count(), 0);
    assert!(h.events.contains(&Event::Window(A, 1, 0)));
    s.destroy(&mut h);
    assert!(h.all_zero());
}

#[test]
fn wildcard_lookup_follows_typed_snapshot_addrefs_and_nested_pending_apply() {
    let _serial = serial();
    let mut s = MessageServer2::new();
    let mut h = Host::default();
    h.msg(1, 5);
    s.add_notification(&mut h, A, 5);
    s.add_notification(&mut h, W, 0);
    // Both lists already exist. Nested apply appends only to the wildcard
    // list; the enclosing typed-list iterator remains valid throughout.
    h.on_add.insert(A, vec![Act::Add(B, 0), Act::Send(1)]);
    s.message_send(&mut h, 1);
    assert_eq!(h.log, vec![(A, 1), (W, 1), (B, 1), (A, 1), (W, 1), (B, 1)]);
    assert_eq!((h.r(A), h.r(W), h.r(B)), (1, 1, 1));
    s.destroy(&mut h);
    assert!(h.all_zero());
}

#[test]
fn directed_equal_local_pointer_suppresses_redundant_assign_reference_calls() {
    let mut s = MessageServer2::new();
    let mut h = Host::default();
    s.post_directed(&mut h, 1, A);
    s.post_directed(&mut h, 1, A);
    h.events.clear();
    s.deliver_directed(&mut h);
    assert_eq!(
        h.events,
        vec![
            Event::Add(A),
            Event::Add(1),
            Event::Release(1),
            Event::Release(A),
            Event::Deliver(A, 1),
            Event::Release(1),
            Event::Release(A),
            Event::Deliver(A, 1),
            Event::Release(1),
            Event::Release(A)
        ]
    );
    assert!(h.all_zero());
}

#[test]
fn directed_append_constructs_copies_and_destroys_temporary_pair() {
    let mut s = MessageServer2::new();
    let mut h = Host::default();
    s.post_directed(&mut h, 1, A);
    assert_eq!(
        h.events,
        vec![
            Event::Add(A),
            Event::Add(1),
            Event::Add(A),
            Event::Add(1),
            Event::Release(1),
            Event::Release(A)
        ]
    );
    assert_eq!((h.r(A), h.r(1)), (1, 1));
    h.events.clear();
    s.destroy(&mut h);
    assert_eq!(h.events, vec![Event::Release(1), Event::Release(A), Event::Close]);
    assert!(h.all_zero());
}

#[test]
fn blocking_get_retries_failed_wait_and_timeout_without_resetting_signal() {
    let mut q = MessageQueue2::new();
    let mut h = Host {
        pushes_on_wait: vec![9],
        skip_wait_push: 1,
        wait_results: vec![0x102, u32::MAX],
        ..Host::default()
    };
    assert_eq!(q.blocking_get_back(&mut h), 9);
    assert_eq!(h.waits, 2);
    assert_eq!(
        h.events,
        vec![
            Event::Wait(1000),
            Event::Wait(1000),
            Event::Add(9),
            Event::Pulse,
            Event::Add(9),
            Event::Release(9)
        ]
    );
    h.release(9);
    assert!(h.all_zero());
}

#[test]
fn flush_guard_is_shared_across_server2_instances() {
    let _serial = serial();
    let mut h = Host::default();
    let mut other = MessageServer2::new();
    registry::HasRegistry::registry_mut(&mut other).depth = 1;
    other.add_notification(&mut h, C, 9);
    registry::HasRegistry::registry_mut(&mut other).depth = 0;
    h.other = Some(other);
    h.on_add.insert(A, vec![Act::Add(B, 5)]);
    let mut outer = MessageServer2::new();
    outer.add_notification(&mut h, A, 7);
    let mut other = h.other.take().unwrap();
    assert_eq!(other.registry().targets(9), &[]);
    assert_eq!(other.registry().targets(8), &[W]);
    assert_eq!(other.registry().pending().count(), 1);
    other.add_notification(&mut h, W, 8); // guard now clear; flush C
    assert_eq!(other.registry().targets(9), &[C]);
    outer.destroy(&mut h);
    other.destroy(&mut h);
    assert!(h.all_zero());
}

#[test]
fn directed_cancel_shift_reference_order_and_equal_target_suppression() {
    let mut h = Host::default();
    let mut s = MessageServer2::new();
    for (m, t) in [(1, B), (2, A), (3, C), (4, W), (5, 1004)] {
        s.post_directed(&mut h, m, t);
    }
    h.events.clear();
    s.cancel_directed(&mut h, A);
    assert_eq!(
        h.events,
        vec![
            Event::Add(B),
            Event::Release(A),
            Event::Add(1),
            Event::Release(2),
            Event::Release(1),
            Event::Release(B)
        ]
    );
    s.destroy(&mut h);
    assert!(h.all_zero());
    let mut s = MessageServer2::new();
    for (m, t) in [(1, B), (2, C), (3, A), (4, A)] {
        s.post_directed(&mut h, m, t);
    }
    h.events.clear();
    s.cancel_directed(&mut h, A);
    assert_eq!(
        h.events,
        vec![Event::Add(4), Event::Release(3), Event::Release(4), Event::Release(A)]
    );
    assert_eq!(s.directed().collect::<Vec<_>>(), vec![(B, 1), (C, 2), (A, 4)]);
    s.destroy(&mut h);
    assert!(h.all_zero());
}

#[test]
fn generation1_cancel_post_quota_nested_tick_and_raw_pending_remove() {
    let _serial = serial();
    let mut s = MessageServer1::new();
    let mut h = Host1::default();
    for (i, t) in [A, B, C, W, 1004].into_iter().enumerate() {
        s.post_directed(&mut h, &[i as u32 + 1, 0, 0, 0], t);
    }
    h.directed_action = Some(false);
    s.on_tick(&mut h);
    assert_eq!(h.log.iter().map(|p| p.0).collect::<Vec<_>>(), vec![A, C, W, 1004, 1005]);
    assert_eq!(s.countdown(), -1);
    assert!(h.refs.is_empty());
    h.log.clear();
    for (i, t) in [A, B, C].into_iter().enumerate() {
        s.post_directed(&mut h, &[i as u32 + 1, 0, 0, 0], t);
    }
    h.directed_action = Some(true);
    s.on_tick(&mut h);
    assert_eq!(h.log.iter().map(|p| p.0).collect::<Vec<_>>(), vec![A, B, C]);
    assert_eq!(s.countdown(), -2);
    assert!(h.refs.is_empty());

    let mut ph = Pending1 {
        remove_pending: true,
        ..Pending1::default()
    };
    ph.add_ref_target(B); // external hold, raw pending remove must not own it
    let mut pending = MessageServer1::new();
    pending.add_notification(&mut ph, A, 9);
    assert_eq!(pending.registry().pending().count(), 0);
    assert_eq!(ph.refs[&B], 1);
    pending.destroy(&mut ph);
    ph.release_target(&mut pending, B);
    assert!(ph.refs.values().all(|&v| v == 0));
}
