// SPDX-License-Identifier: GPL-3.0-or-later
//! framework-gz-services §5.3, host-controlled quiescent fixtures only.
use super::*;

fn fixture() -> (Host, QueueManager2, Token, Token) {
    let mut h = Host::default();
    let q = h.new_queue();
    let r = h.new_queue();
    h.add_ref_queue(q); // Explicit external fixture holds.
    h.add_ref_queue(r);
    (h, QueueManager2::new(), q, r)
}

fn finish(h: &mut Host, m: &mut QueueManager2, q: Token, r: Token) {
    m.remove_all_queues(h);
    h.release_queue(q);
    h.release_queue(r);
    assert!(h.queues.iter().all(MessageQueue2::is_destroyed));
    assert!(h.all_zero());
}

#[test]
fn gz_v17_id_cardinality_counts_aliases() {
    let (mut h, mut m, q, r) = fixture();
    assert_eq!(m.len(), 0);
    assert!(m.add_queue(&mut h, 7, q));
    assert_eq!(m.len(), 1);
    assert!(m.add_queue(&mut h, 8, q));
    assert_eq!(m.len(), 2);
    assert!(!m.add_queue(&mut h, 7, r));
    assert_eq!(m.len(), 2);
    assert!(m.remove_queue_id(&mut h, 7));
    assert_eq!(m.len(), 1);
    assert!(m.remove_all_queues(&mut h));
    assert_eq!(m.len(), 0);
    finish(&mut h, &mut m, q, r);
}

fn probes(h: &mut Host, m: &QueueManager2) -> Vec<Option<Token>> {
    (0..256)
        .chain([0x64fa8434, 0x652294bc, u32::MAX])
        .map(|id| {
            let result = m.get_queue(h, id);
            if let Some(q) = result {
                h.release_queue(q);
            }
            result
        })
        .collect()
}

#[test]
fn gz_v18_hidden_key_is_not_empty_registry() {
    let (mut h, mut m, q, r) = fixture();
    assert!(m.add_queue(&mut h, 0x10000, q));
    assert!(probes(&mut h, &m).iter().all(Option::is_none));
    assert_eq!(m.len(), 1);
    finish(&mut h, &mut m, q, r);
}

#[test]
fn gz_v19_lookup_hold_survives_removal() {
    let (mut h, mut m, q, r) = fixture();
    m.add_queue(&mut h, 7, q);
    let held = m.get_queue(&mut h, 7).unwrap();
    assert_eq!(held, q);
    assert!(m.remove_queue_id(&mut h, 7));
    h.release_queue(q); // Leave only the lookup's hold.
    assert_eq!(m.get_queue(&mut h, 7), None);
    assert_eq!(m.len(), 0);
    assert!(!h.queues[q as usize].is_destroyed());
    h.release_queue(held);
    assert!(h.queues[q as usize].is_destroyed());
    h.release_queue(r);
    assert!(h.all_zero());
}

#[test]
fn gz_v20_equal_bounded_samples_do_not_prove_registry_equality() {
    let (mut h, mut m, q, r) = fixture();
    m.add_queue(&mut h, 0x10000, q);
    let before = (m.len(), probes(&mut h, &m));
    m.remove_queue_id(&mut h, 0x10000);
    m.add_queue(&mut h, 0x10001, q);
    assert_eq!(before, (m.len(), probes(&mut h, &m)));
    assert_eq!(m.get_queue(&mut h, 0x10000), None);
    assert_eq!(m.entries().collect::<Vec<_>>(), [(0x10001, q)]);
    finish(&mut h, &mut m, q, r);
}

#[test]
fn gz_v21_ordinary_count_excludes_notifications_and_directed_posts() {
    let _serial = serial();
    let mut h = Host::default();
    h.msg(A, 7);
    MessageHost::add_ref(&mut h, A);
    MessageHost::add_ref(&mut h, B);
    let mut s = MessageServer2::new();
    assert_eq!(s.posted_count(), 0);
    s.add_notification(&mut h, B, 7);
    assert_eq!(s.posted_count(), 0);
    s.post_directed(&mut h, A, B);
    assert_eq!(s.posted_count(), 0);
    s.message_post(&mut h, A, false);
    assert_eq!(s.posted_count(), 1);
    assert!(h.log.is_empty());
    s.destroy(&mut h);
    MessageHost::release(&mut h, A);
    MessageHost::release(&mut h, B);
    assert!(h.all_zero());
}

#[test]
fn gz_v22_endpoints_do_not_identify_contents() {
    let mut h = Host::default();
    for t in [A, B, C, W] {
        h.msg(t, 7);
        MessageHost::add_ref(&mut h, t);
    }
    let mut q = MessageQueue2::new();
    let mut r = MessageQueue2::new();
    for t in [A, B, C] {
        q.push_back(&mut h, t);
    }
    for t in [A, W, C] {
        r.push_back(&mut h, t);
    }
    for queue in [&q, &r] {
        assert_eq!(queue.count(), 3);
        assert_eq!(queue.front_type(&mut h), 7);
        assert_eq!(queue.back_type(&mut h), 7);
        assert_eq!(queue.peek_front(&mut h), Some(A));
        assert_eq!(queue.peek_back(&mut h), Some(C));
        MessageHost::release(&mut h, A);
        MessageHost::release(&mut h, C);
    }
    q.destroy(&mut h);
    r.destroy(&mut h);
    for t in [A, B, C, W] {
        MessageHost::release(&mut h, t);
    }
    assert!(h.all_zero());
}
