// SPDX-License-Identifier: GPL-3.0-or-later
use super::{tests::Host, *};

fn request(mode: i32, rect: Rect, model: Option<Handle>) -> DirtyRequest {
    DirtyRequest {
        mode,
        rect,
        model,
        delay: 0,
    }
}

#[test]
fn merge_d1_overlap_and_d2_distant_rectangles() {
    let mut h = Host::default();
    let mut queue = DirtyQueue::default();
    let policy = DirtyPolicy::default();
    queue.add(&mut h, request(1, Rect::new(0, 0, 100, 100), None), [0, 7], policy);
    queue.add(&mut h, request(1, Rect::new(50, 0, 150, 100), None), [0, 8], policy);
    assert_eq!(queue.accumulated_area(), 15000);
    assert_eq!(
        queue.nodes(),
        &[DirtyNode {
            stamp: 7,
            mode: 6,
            rect: Rect::new(0, 0, 150, 100),
            model: None
        }]
    );
    let mut queue = DirtyQueue::default();
    queue.add(&mut h, request(1, Rect::new(0, 0, 10, 10), None), [0, 0], policy);
    queue.add(&mut h, request(1, Rect::new(500, 500, 510, 510), None), [0, 0], policy);
    assert_eq!(queue.nodes().len(), 2);
    assert_eq!(queue.accumulated_area(), 200);
}

#[test]
fn merge_d3_releases_node_model_and_does_not_retain_incoming_model() {
    let mut h = Host::default();
    let mut queue = DirtyQueue::default();
    let policy = DirtyPolicy::default();
    queue.add(
        &mut h,
        request(0, Rect::new(0, 0, 100, 100), Some(Handle(40))),
        [0, 0],
        policy,
    );
    queue.add(
        &mut h,
        request(0, Rect::new(0, 0, 100, 100), Some(Handle(41))),
        [0, 0],
        policy,
    );
    assert_eq!(h.events, ["retain 40", "release 40"]);
    assert_eq!(queue.nodes()[0].mode, 6);
    assert_eq!(queue.nodes()[0].model, None);
    queue.clear(&mut h);
    assert_eq!(h.events.len(), 2);
}

#[test]
fn merge_d4_first_tie_recursive_merge_overcounts_area() {
    let mut h = Host::default();
    let mut queue = DirtyQueue::default();
    let policy = DirtyPolicy::default();
    for rect in [Rect::new(0, 0, 50, 100), Rect::new(100, 0, 200, 100)] {
        queue.add(&mut h, request(1, rect, None), [100, 9], policy);
    }
    assert_eq!(queue.accumulated_area(), 15000);
    assert_eq!(queue.nodes().len(), 2);
    queue.add(&mut h, request(1, Rect::new(40, 0, 110, 100), None), [100, 10], policy);
    assert_eq!(queue.nodes().len(), 1);
    assert_eq!(queue.nodes()[0].rect, Rect::new(0, 0, 200, 100));
    assert_eq!(queue.nodes()[0].stamp, 9);
    assert_eq!(queue.accumulated_area(), 31000);
}

#[test]
fn merge_d5_sixteen_nodes_force_a_merge_below_score_threshold() {
    let mut h = Host::default();
    let mut queue = DirtyQueue::default();
    let policy = DirtyPolicy::default();
    for x in (0..16).map(|i| i * 10000) {
        queue.add(
            &mut h,
            request(1, Rect::new(x, 0, x + 1, 1), None),
            [0, 0],
            DirtyPolicy { merge: false, ..policy },
        );
    }
    assert_eq!(queue.nodes().len(), 16);
    queue.add(
        &mut h,
        request(1, Rect::new(160000, 0, 160001, 1), None),
        [0, 0],
        policy,
    );
    assert_eq!(queue.nodes().len(), 16);
    assert!(queue
        .nodes()
        .iter()
        .any(|node| node.mode == 6 && node.rect == Rect::new(150000, 0, 160001, 1)));
}

#[test]
fn filter_d6_clamping_edge_rejection_and_inverted_rect() {
    let mut rect = Rect::new(-10, 590, 20, 700);
    assert!(rect.filter_to_viewport(800, 600));
    assert_eq!(rect, Rect::new(0, 590, 20, 600));
    let mut rect = Rect::new(800, 0, 900, 10);
    assert!(!rect.filter_to_viewport(800, 600));
    assert_eq!(rect, Rect::new(800, 0, 900, 10));
    let mut rect = Rect::new(5, 0, 3, 10);
    assert!(rect.filter_to_viewport(800, 600));
    assert_eq!(rect.area(), -20);
}

#[test]
fn deadline_moves_earlier_only_for_merge_enabled_nonempty_queue() {
    let mut h = Host::default();
    let mut queue = DirtyQueue::default();
    let policy = DirtyPolicy::default();
    for (rect, delay) in [(Rect::new(0, 0, 10, 10), 2), (Rect::new(500, 500, 510, 510), 3)] {
        let mut add = request(1, rect, None);
        add.delay = delay;
        queue.add(&mut h, add, [100, 0], policy);
    }
    assert_eq!(queue.deadline_base(), 95);
    let mut add = request(1, Rect::new(1000, 1000, 1010, 1010), None);
    add.delay = 10;
    queue.add(&mut h, add, [100, 0], DirtyPolicy { merge: false, ..policy });
    assert_eq!(queue.deadline_base(), 95);
}

#[test]
fn full_redraw_case_11_releases_queued_models_then_ignores_additions() {
    let mut h = Host::default();
    let mut r = Render3d::new(&mut h);
    r.set_viewport_size(800, 600);
    r.state.dirty_area_limit = 10000;
    r.add_dirty_rectangle(&mut h, request(0, Rect::new(0, 0, 10, 10), Some(Handle(20))));
    r.add_dirty_rectangle(&mut h, request(1, Rect::new(0, 0, 100, 100), Some(Handle(21))));
    assert!(r.state.full_redraw);
    assert_eq!(r.state.draw_reason, 4);
    assert!(r.dirty_queue().nodes().is_empty());
    assert_eq!(r.dirty_queue().accumulated_area(), 0);
    assert_eq!(h.events, ["retain 20", "release 20"]);
    r.add_dirty_rectangle(&mut h, request(0, Rect::new(0, 0, 10, 10), Some(Handle(22))));
    assert!(r.dirty_queue().nodes().is_empty());
    assert_eq!(h.events.len(), 2);
    assert_eq!(h.frustums.len(), 1);
}

#[test]
fn clear_queue_balances_only_owned_models_and_keeps_deadline_base() {
    let mut h = Host::default();
    let mut queue = DirtyQueue::default();
    let policy = DirtyPolicy {
        merge: false,
        ..DirtyPolicy::default()
    };
    for (mode, model) in [(0, Handle(20)), (1, Handle(21)), (0, Handle(22))] {
        queue.add(
            &mut h,
            request(mode, Rect::new(0, 0, 5, 5), Some(model)),
            [50, 0],
            policy,
        );
    }
    assert_eq!(queue.nodes().len(), 3);
    queue.clear(&mut h);
    queue.clear(&mut h);
    assert_eq!(h.events, ["retain 20", "retain 22", "release 20", "release 22"]);
    assert_eq!(queue.accumulated_area(), 0);
    assert_eq!(queue.deadline_base(), 50);
}
