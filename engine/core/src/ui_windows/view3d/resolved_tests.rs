// SPDX-License-Identifier: GPL-3.0-or-later
// Included in owner_tests to share the foreign-interface recorder.

#[test]
fn vw_t19_t20_deferred_tail_and_timer_boundaries() {
    let mut v = view();
    let mut h = Host {
        changed: false,
        ..Host::default()
    };
    v.owner.deferred = vec![
        Notification {
            kind: 1,
            ..Notification::default()
        },
        Notification {
            kind: 2,
            ..Notification::default()
        },
    ];
    assert!(v.paint(&mut h, 0, 0));
    assert_eq!(v.owner.deferred.len(), 2);
    h.changed = true;
    assert!(v.paint(&mut h, 0, 0));
    assert_eq!(v.owner.deferred.len(), 1);
    assert!(h.ops.contains(&Op::Repost(1, 0)));
    v.motion_mut().paint_enabled = true;
    v.motion_mut().paint_period_us = 1000.0;
    for elapsed in [999, 1000] {
        h.ops.clear();
        assert!(v.paint(&mut h, elapsed, 0));
        assert!(!h.ops.contains(&Op::Draw));
    }
    h.ops.clear();
    assert!(v.paint(&mut h, 1001, 0));
    assert!(h.ops.contains(&Op::Draw));
    v.motion_mut().paint_period_us = 0.0;
    h.ops.clear();
    assert!(v.paint(&mut h, 0, 0));
    assert!(h.ops.contains(&Op::Cost(0.0)));
}

#[test]
fn vw_t21_inverted_limits_minimum_first() {
    let mut v = view();
    let mut h = Host {
        changed: false,
        ..Host::default()
    };
    assert!(v.set_zoom_limits(&mut h, 4, 2, None));
    h.ops.clear();
    v.set_zoom(&mut h, 3);
    v.set_zoom(&mut h, 4);
    assert_eq!(h.ops, [Op::View(ViewChange::Zoom(4)), Op::View(ViewChange::Zoom(2))]);
}

#[test]
fn shutdown_releases_each_deferred_queue_reference_once() {
    let mut v = view();
    let mut h = Host {
        changed: false,
        ..Host::default()
    };
    for kind in [0xAA5C_F1E3, 0xAA5C_F1E3] {
        v.notification(
            &mut h,
            Notification {
                kind,
                ..Notification::default()
            },
        );
    }
    // Only BudgetFailure is enqueued by notification dispatch. Seed a second
    // owned queue entry to exercise shutdown independently of that producer.
    let second = Notification {
        kind: 0xAC08_3D7D,
        ..Notification::default()
    };
    h.deferred_reference(&second, true);
    v.owner.deferred.push(second);
    assert_eq!(v.owner.deferred.len(), 2);
    v.shutdown_tail(&mut h);
    v.shutdown_tail(&mut h);
    let references: Vec<_> = h
        .ops
        .into_iter()
        .filter(|op| matches!(op, Op::DeferredRef(..)))
        .collect();
    assert_eq!(
        references,
        [
            Op::DeferredRef(0xAA5C_F1E3, true),
            Op::DeferredRef(0xAC08_3D7D, true),
            Op::DeferredRef(0xAA5C_F1E3, false),
            Op::DeferredRef(0xAC08_3D7D, false),
        ]
    );
    assert!(v.owner.deferred.is_empty());
}

#[test]
fn vw_t19_safe_but_disabled_modal_drops_one_message_without_repost() {
    let mut v = view();
    let mut h = Host {
        changed: false,
        ..Host::default()
    };
    for kind in [0xAA5C_F1E3, 0xAC08_3D7D] {
        let message = Notification {
            kind,
            ..Notification::default()
        };
        h.deferred_reference(&message, true);
        v.owner.deferred.push(message);
    }
    h.changed = true; // Modal state is safe when the paint tail begins.
    h.deferred_enabled = Some(false); // Recheck after unlink forbids repost.
    h.ops.clear();
    assert!(v.paint(&mut h, 0, 0));
    assert_eq!(v.owner.deferred.len(), 1);
    assert_eq!(
        h.ops,
        [
            Op::DeferredRef(0xAA5C_F1E3, true),
            Op::DeferredRef(0xAA5C_F1E3, false),
            Op::DeferredRef(0xAA5C_F1E3, false),
            Op::Tail,
        ]
    );
    v.shutdown_tail(&mut h);
    assert_eq!(h.ops.last(), Some(&Op::DeferredRef(0xAC08_3D7D, false)));
}

#[test]
fn vw_t22_cursor_priorities_and_in_place_update() {
    let mut v = view();
    let mut h = Host::default();
    for (id, priority) in [(1, 2), (2, 9), (3, 9), (1, 100)] {
        v.set_cursor_text(
            &mut h,
            CursorText {
                id,
                priority,
                text1: id.to_string(),
                text2: String::new(),
                flag: false,
            },
        );
    }
    assert_eq!(
        v.owner
            .display
            .texts
            .iter()
            .map(|e| (e.id, e.priority))
            .collect::<Vec<_>>(),
        [(3, 9), (2, 9), (1, 2)]
    );
}

#[test]
fn vw_t23_fade_alpha_motion_and_hidden_reset() {
    let mut v = view();
    let mut h = Host::default();
    v.update_cursor_position(&mut h, 100, 200);
    v.set_cost_amount(&mut h, 1000);
    v.clear_cost_indicator(&mut h, true);
    v.update_cost_indicator(&mut h, 0.5);
    let cost = v.owner.display.cost;
    assert_eq!(cost.alpha, 170);
    assert_eq!(cost.fade_left_ms, 1000.0);
    assert_eq!(cost.position, (100, 133));
    v.update_cost_indicator(&mut h, 0.25);
    assert_eq!(v.owner.display.cost.alpha, 127);
    v.update_cost_indicator(&mut h, 0.75);
    assert_eq!(v.owner.display.cost.mode, CostMode::Hidden);
    v.clear_cost_indicator(&mut h, true);
    assert_eq!(v.owner.display.cost.fade_left_ms, 1500.0);
}

#[test]
fn vw_t25_precity_shutdown_preserves_city_objects_and_noop_command() {
    let mut v = view();
    let mut h = Host::default();
    v.owner.deferred.push(Notification::default());
    v.owner.motion.paint_enabled = true;
    v.owner.lifecycle.objects.insert(0xF4, 123);
    v.set_current_control(&mut h, Some(10), 2);
    v.add_persistent_control(&mut h, 11);
    v.pre_city_shutdown(&mut h);
    assert!(v.owner.deferred.is_empty());
    assert!(v.owner.stack.is_empty());
    assert!(v.owner.persistent.is_empty());
    assert_eq!(v.renderer(), Some(123));
    assert!(v.owner.motion.paint_enabled);
    assert!(!v.on_command(u32::MAX, u32::MAX));
}

#[test]
fn vw_t28_registry_is_literal_and_ordered() {
    let mut v = view();
    let mut h = Host::default();
    v.init(&mut h);
    assert_eq!(h.registry.len(), 104);
    assert_eq!(h.registry[0], (0x6A935C34, "ToggleGodPanel", 0));
    assert_eq!(h.registry[55], (0x6A935DC9, "PlaceBusStopTool", 0));
    assert_eq!(h.registry[64], (0x6A935DEE, "PipesTool", 0));
    assert_eq!(h.registry[103], (0xCAB22C64, "AnimationRecorder", 0));
    assert!(h.registry.iter().all(|r| r.2 == 0));
}

#[test]
fn vw_t30_trigger_wrapping_bucket_heads_and_growth() {
    let mut v = view();
    let mut h = Host::default();
    *v.owner.view.triggers.entry(1, || u32::MAX) = u32::MAX;
    v.assert_global_trigger(&mut h, 1);
    assert!(v.global_trigger_state(1));
    assert_eq!(v.save_record().triggers, [(1, 0)]);
    assert!(h.ops.is_empty());
    v.retract_global_trigger(&mut h, 1);
    assert_eq!(h.ops, [Op::Send(0xAA375FE8, 1, 0)]);
    v.assert_global_trigger(&mut h, 1);
    v.assert_global_trigger(&mut h, 194);
    assert_eq!(v.save_record().triggers, [(194, 1), (1, 1)]);
    for key in 2..194 {
        v.assert_global_trigger(&mut h, key);
    }
    assert_eq!(v.owner.view.triggers.bucket_count(), 389);
    assert_eq!(v.save_record().triggers.len(), 194);
}

#[test]
fn vw_t31_sizeof_crt_grammar() {
    let mut v = view();
    let mut h = Host::default();
    for text in ["nan", "inf", "0x1p4", "5junk", "1D1"] {
        v.process_cheat(&mut h, 0xCA7BC5B5, &["SizeOf", text]);
    }
    assert_eq!(
        h.ops,
        [
            Op::Size(0.9),
            Op::Size(0.9),
            Op::Size(0.9),
            Op::Size(5.0),
            Op::Size(10.0)
        ]
    );
}

#[test]
fn sizeof_uses_host_locale_decimal_separator() {
    let mut v = view();
    let mut h = Host {
        decimal_separator: b',',
        ..Host::default()
    };
    for text in ["1,5", "1.5", "  +2,5D1tail", "bad"] {
        v.process_cheat(&mut h, 0xCA7BC5B5, &["SizeOf", text]);
    }
    assert_eq!(h.ops, [Op::Size(1.5), Op::Size(1.0), Op::Size(25.0), Op::Size(0.9)]);
}

#[test]
fn vw_t24_date_formatter_year_boundaries() {
    let v = view();
    let mut h = Host::default();
    for (year, formatted_year) in [(-1, -1), (0, 0), (10000, 10000), (10001, 1)] {
        h.date = Some([2000 + year, 4, 5]);
        h.widgets.clear();
        v.update_date(&mut h);
        assert_eq!(h.widgets, [WidgetCall::Date([formatted_year, 4, 5])]);
    }
}

#[derive(Default)]
struct GraphRecorder {
    resource: Option<u32>,
    calls: Vec<(u32, i32, u32)>,
}

impl GraphHost for GraphRecorder {
    fn graph_resource(&mut self, instance: u32, group: u32) -> Option<u32> {
        assert_eq!((instance, group), (0x4203, 0xCA31_8385));
        self.resource
    }
    fn graph_reset(&mut self, resource: u32, index: i32, value: u32) {
        assert_eq!(resource, 77);
        self.calls.push((8, index, value));
    }
    fn graph_prepare(&mut self, resource: u32) {
        assert_eq!(resource, 77);
        self.calls.push((6, 0, 0));
    }
    fn graph_series(&mut self, resource: u32, series: u32, colour: u32) {
        assert_eq!(resource, 77);
        self.calls.push((7, series as i32, colour));
    }
    fn graph_release(&mut self, resource: u32) {
        assert_eq!(resource, 77);
        self.calls.push((0, 0, 0));
    }
}

#[test]
fn vw_t39_graph_resource_order_and_conditional_stop() {
    let v = view();
    let mut graph = GraphRecorder::default();
    v.setup_graph(&mut graph);
    assert!(graph.calls.is_empty());
    graph.resource = Some(77);
    v.setup_graph(&mut graph);
    assert_eq!(
        graph.calls,
        [
            (8, -1, 0),
            (6, 0, 0),
            (7, 1, 0x00FF00),
            (7, 2, 0x44CC22),
            (7, 3, 0x1E911E),
            (7, 4, 0x5B81D0),
            (7, 5, 0x3D6EB8),
            (7, 6, 0x3C42AD),
            (7, 7, 0xD0D030),
            (7, 8, 0xC6B733),
            (7, 9, 0xBC9F37),
            (0, 0, 0),
        ]
    );
    let mut h = Host {
        changed: false,
        ..Host::default()
    };
    let mut active = false;
    assert!(v.stop_auxiliary(&mut h, 42, &mut active));
    assert!(h.helper_calls.is_empty());
    active = true;
    assert!(v.stop_auxiliary(&mut h, 42, &mut active));
    assert!(!active);
    assert_eq!(h.helper_calls, [(42, 4)]);
    h.changed = true;
    h.helper_calls.clear();
    active = true;
    assert!(v.stop_auxiliary(&mut h, 42, &mut active));
    assert!(!active);
    assert_eq!(h.helper_calls, [(42, 4), (42, 0)]);
}

#[test]
fn vw_t35_borrowed_getters_and_debug_no_effect() {
    let mut v = view();
    v.owner.lifecycle.objects.insert(0xF4, 1);
    v.owner.lifecycle.objects.insert(0xF8, 2);
    v.owner.lifecycle.objects.insert(0x120, 3);
    let refs = v.refs();
    for _ in 0..3 {
        assert_eq!(v.renderer(), Some(1));
        assert_eq!(v.model_maker(), Some(2));
        assert_eq!(v.key_accelerator(), Some(3));
        v.display_terrain_pick_debug_string(u32::MAX, 0);
    }
    assert_eq!(v.refs(), refs);
}

#[derive(Default)]
struct CoverageRecorder {
    listed: Vec<Category>,
    occupants: Vec<u32>,
    radius: Option<f32>,
    position: Option<[f32; 3]>,
    transforms: Vec<CoverageTransform>,
    created: Vec<&'static str>,
    released: Vec<u32>,
    stopped: Vec<u32>,
}

impl CoverageHost for CoverageRecorder {
    fn occupants(&mut self, category: Category) -> Vec<u32> {
        self.listed.push(category);
        self.occupants.clone()
    }
    fn radius(&mut self, c: Category, _: u32, factor: f32) -> Option<f32> {
        assert_eq!(factor, c.radius_factor());
        self.radius
    }
    fn position(&mut self, _: u32) -> Option<[f32; 3]> {
        self.position
    }
    fn release_occupant(&mut self, _: u32) {}
    fn create_effect(&mut self, name: &'static str) -> Option<u32> {
        self.created.push(name);
        Some(77)
    }
    fn transform(&mut self, _: u32, transform: CoverageTransform) {
        self.transforms.push(transform);
    }
    fn start(&mut self, _: u32, argument: u32) {
        assert_eq!(argument, 0);
    }
    fn running(&mut self, _: u32) -> bool {
        true
    }
    fn stop(&mut self, effect: u32, argument: u32) {
        assert_eq!(argument, 0);
        self.stopped.push(effect);
    }
    fn release_effect(&mut self, effect: u32) {
        self.released.push(effect);
    }
}

#[test]
fn vw_t37_t38_coverage_transform_fallback_and_stale_cache() {
    let mut coverage = Coverage::default();
    let mut h = CoverageRecorder {
        occupants: vec![1],
        radius: Some(12.0),
        position: Some([10.0, 20.0, 30.0]),
        ..CoverageRecorder::default()
    };
    coverage.set_categories(&mut h, 1);
    assert!(h.listed.is_empty());
    coverage.recalculate(&mut h);
    assert_eq!(h.listed, [Category::Police]);
    assert_eq!(h.created, ["PlopMode_Police_Existing"]);
    assert_eq!(h.transforms[0].scale, 24.0);
    h.position = Some([40.0; 3]);
    coverage.recalculate(&mut h);
    assert_eq!(h.transforms.len(), 1);
    h.radius = Some(13.0);
    coverage.recalculate(&mut h);
    coverage.recalculate(&mut h);
    assert_eq!(h.transforms.len(), 3);
    assert_eq!(h.transforms[2].scale, 26.0);
    h.occupants.clear();
    coverage.recalculate(&mut h);
    assert_eq!(h.stopped, [77]);
    assert_eq!(h.released, [77]);
    h.occupants.push(1);
    coverage.recalculate(&mut h);
    assert_eq!(h.created.len(), 1);
    coverage.set_categories(&mut h, 1);
    h.radius = None;
    coverage.recalculate(&mut h);
    assert_eq!(h.transforms.last().unwrap().scale, 64.0);
    coverage.clear(&mut h);
    h.position = None;
    coverage.recalculate(&mut h);
    assert_eq!(h.created.len(), 2);
}
