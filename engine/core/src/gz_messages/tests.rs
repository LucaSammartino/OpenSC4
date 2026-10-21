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
    fn add_ref(&mut self, t: Token) {
        *self.refs.entry(t).or_default() += 1;
    }
    fn release(&mut self, t: Token) {
        *self.refs.entry(t).or_default() -= 1;
    }
    fn wait_signal(&mut self, queue: &mut MessageQueue2, timeout: u32) {
        assert_eq!(timeout, BLOCKING_WAIT_TIMEOUT);
        self.waits += 1;
        if let Some(m) = self.pushes_on_wait.pop() {
            queue.push_back(self, m);
        }
    }
}

impl Server2Host for Host {
    fn do_message(&mut self, s: &mut MessageServer2, target: Token, msg: Token) {
        self.log.push((target, msg));
        if let Some(acts) = self.on_deliver.remove(&target) {
            self.run(s, acts);
        }
    }
    fn release_target(&mut self, s: &mut MessageServer2, target: Token) {
        self.release(target);
        if let Some(acts) = self.on_release.remove(&target) {
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
    assert!(!q.signal());
    q.push_back(&mut h, m1);
    assert!(q.signal());
    q.reset_signal();
    q.push_front(&mut h, m2);
    assert!(!q.signal(), "signal only on the empty to non-empty transition");
    assert_eq!(q.count(), 2);
    assert_eq!((q.front_type(&mut h), q.back_type(&mut h)), (7, 5));
    assert!(q.contains_type(&mut h, 5) && !q.contains_type(&mut h, 9));
    assert_eq!(q.peek_back(&mut h), Some(m1));
    assert_eq!(h.r(m1), 2, "peek AddRefs for the caller and keeps the message");
    assert_eq!(q.peek_equal(&mut h, m2), Some(m2));
    assert_eq!(q.peek_equal(&mut h, 3), None);
    assert_eq!(q.get_front(), Some(m2));
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
    let mut h = Host::default();
    h.pushes_on_wait = vec![2, 1];
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
    let (log, left) = worked_example(false);
    assert_eq!(log, vec![(W, 2), (A, 1), (W, 1)]);
    assert_eq!(left, 1, "m3 waits: taken reached the limit after m1");
    let (log, left) = worked_example(true);
    assert_eq!(log, vec![(W, 2), (A, 1), (W, 1), (W, 3)]);
    assert_eq!(left, 0);
}

#[test]
fn server2_send_order_wildcard_and_no_duplicates() {
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
    let mut h = Host::default();
    let mut s = MessageServer2::new();
    s.post_directed(&mut h, 1, A);
    s.post_directed(&mut h, 2, B);
    s.post_directed(&mut h, 3, A);
    assert_eq!(s.cancel_directed(&mut h, A), 2);
    assert_eq!((h.r(A), h.r(1), h.r(3)), (0, 0, 0));
    s.post_directed(&mut h, 4, C);
    h.on_deliver.insert(B, vec![Act::Directed(5, A)]);
    s.on_tick(&mut h);
    assert_eq!(h.log, vec![(B, 2), (C, 4)]);
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
}

impl Server1Host for Host1 {
    fn add_ref_target(&mut self, t: Token) {
        *self.refs.entry(t).or_default() += 1;
    }
    fn release_target(&mut self, _s: &mut MessageServer1, t: Token) {
        *self.refs.entry(t).or_default() -= 1;
    }
    fn do_message(&mut self, s: &mut MessageServer1, t: Token, msg: &Record) {
        let held = self.refs[&t];
        self.log.push((t, *msg, held));
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
