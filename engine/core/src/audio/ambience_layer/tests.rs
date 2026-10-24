// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Default)]
struct Tuning;
impl LayerTuningSource for Tuning {
    fn text(&mut self, _: u32) -> Option<String> {
        None
    }
    fn words(&mut self, _: u32) -> Option<Vec<u32>> {
        None
    }
    fn word(&mut self, key: u32) -> Option<u32> {
        (key == 0x6A3F_BDDA).then_some(0x1234)
    }
    fn float(&mut self, key: u32) -> Option<f32> {
        (key == 0x0A41_68E1).then_some(30.0)
    }
    fn curve(&mut self, _: u32) -> Option<ResponseCurve> {
        None
    }
}

#[test]
fn aa10_09_missing_resource_keeps_constructor_loop_and_present_clears_bytes() {
    let (missing, found) = LayerTuning::read(7, 99, 1, None);
    assert!(!found);
    assert_eq!((missing.loop_id, missing.bytes), (99, [31; 2]));
    assert_eq!((missing.vector_rate, missing.scalar_rate), (0.1, 4.0));
    let (present, found) = LayerTuning::read(7, 99, 1, Some(&mut Tuning));
    assert!(found);
    assert_eq!((present.loop_id, present.bytes), (0x1234, [0; 2]));
    assert_eq!((present.occupant_radius, present.cell_radius), (30.0, 30.0));
}

#[derive(Default)]
struct Host {
    created: Vec<u32>,
    released: Vec<u64>,
    tests: Vec<(u64, u32)>,
    cells: usize,
}
impl FilterHost for Host {
    fn create_filter(&mut self, clsid: u32, iid: u32) -> Option<u64> {
        assert_eq!(iid, 0x4A40_074C);
        self.created.push(clsid);
        Some(clsid as u64)
    }
    fn init_filter(&mut self, _: u64) -> bool {
        true
    }
    fn query(&mut self, filter: u64, iid: u32) -> Option<u64> {
        // A has global/occupant; B only cell.
        ((filter == 1 && matches!(iid, 0xEA40_074F | 0xCA40_0753)) || (filter == 2 && iid == 0xCA40_0757))
            .then_some(filter)
    }
    fn test(&mut self, interface: u64, stage: FilterStage) -> bool {
        self.tests.push((interface, stage.iid()));
        true
    }
    fn release(&mut self, reference: u64) {
        self.released.push(reference);
    }
    fn log(&mut self, _: &'static str) {}
    fn cell_center(&mut self, x: i32, z: i32) -> (f32, f32) {
        self.cells += 1;
        (x as f32, z as f32)
    }
    fn altitude(&mut self, _: f32, _: f32) -> f32 {
        7.0
    }
    fn occupant_position(&mut self, _: u64) -> Position {
        [1.0, 2.0, 3.0]
    }
}

#[test]
fn aa10_10_shared_filter_cache_counts_each_listing_and_balances_queries() {
    let mut globals = LayerGlobals::default();
    let mut host = Host::default();
    let mut a = LayerFilters::default();
    let mut b = LayerFilters::default();
    a.create(&[vec![1, 1], vec![], vec![]], &mut globals, &mut host);
    b.create(&[vec![1], vec![], vec![]], &mut globals, &mut host);
    assert_eq!(host.created, [1]);
    assert_eq!(a.interface_counts, [0, 2, 2]);
    assert_eq!(b.interface_counts, [0, 1, 1]);
    assert_eq!(host.released.len(), 6);
}

#[test]
fn aa10_11_global_kind_three_with_only_cell_interface_never_passes() {
    let mut globals = LayerGlobals::default();
    let mut host = Host::default();
    let mut filters = LayerFilters::default();
    filters.create(&[vec![1], vec![], vec![2]], &mut globals, &mut host);
    assert!(!filters.can_handle(true, &mut host));
    let count = host.tests.len();
    assert!(!filters.can_handle(true, &mut host));
    assert_eq!(host.tests.len(), count);
    filters.begin();
    assert!(!filters.can_handle(false, &mut host));
    assert_eq!(filters.cached_global, None);
}

#[test]
fn aa10_12_groups_have_strict_radius_and_keep_fixed_center_and_cycle_peaks() {
    let mut groups = GroupLayer::new(10.0);
    groups.insert([0.0; 3], 0.0);
    groups.insert([10.0, 0.0, 0.0], 0.0);
    assert_eq!(groups.groups.len(), 2);
    groups.insert([9.99, 0.0, 0.0], 0.0);
    assert_eq!((groups.groups[0].center, groups.groups[0].count), ([0.0; 3], 2));
    groups.begin();
    groups.insert([0.0; 3], 0.0);
    groups.end();
    assert_eq!(groups.groups.len(), 1);
    assert_eq!((groups.groups[0].count, groups.groups[0].peak), (1, 2));
}

#[test]
fn aa10_13_sample_reduction_keeps_first_in_each_occupied_grid_cell() {
    let positions: Vec<_> = (0..12).map(|index| [1.0, 0.0, index as f32]).collect();
    assert_eq!(distributed_positions(&positions, [0.0, 0.0, 30.0, 30.0]).len(), 2);
    assert_eq!(
        distributed_positions(&positions[..9], [0.0, 0.0, 30.0, 30.0]),
        positions[..9]
    );
    assert_eq!(distributed_positions(&positions, [0.0; 4]).len(), 1);
}

fn layer() -> AmbienceLayer {
    let tuning = LayerTuning::defaults(1, 0, 1);
    let ease = Ease {
        target_position: [0.0; 3],
        current_position: [0.0; 3],
        target_scalar: 0,
        current_scalar: 0,
        stepped: false,
        vector_rate: tuning.vector_rate,
        scalar_rate: tuning.scalar_rate,
    };
    AmbienceLayer {
        tuning,
        filters: LayerFilters::default(),
        occupant_groups: GroupLayer::new(50.0),
        cell_groups: GroupLayer::new(50.0),
        intersection: GroupLayer::new(50.0),
        rectangle: [0.0, 0.0, 30.0, 30.0],
        counters: [0; 4],
        can_handle_this_update: false,
        paused: false,
        group_count: 0,
        score: 0.0,
        samples: Vec::new(),
        listener_position: [0.0; 3],
        listener_vector: [0.0; 3],
        average: [0.0; 3],
        score_volume: 0,
        play_percent: 0,
        loop_volume: 0,
        scores: Vec::new(),
        loops: Vec::new(),
        ease,
    }
}

#[derive(Default)]
struct Events {
    ids: Vec<u32>,
    words: Vec<(usize, u32)>,
    failed: Option<u32>,
    released: usize,
    divisions: Vec<(u32, u32, bool)>,
    order: Vec<u32>,
    payloads: Vec<u64>,
}
impl LayerEventHost for Events {
    fn create_event(&mut self, event: u32, iid: u32) -> Option<u64> {
        assert_eq!(iid, 0x5B52_3BA4);
        self.ids.push(event);
        self.order.push(event);
        (self.failed != Some(event)).then_some(event as u64)
    }
    fn set_data(&mut self, _: u64, word: usize, value: u32) {
        self.words.push((word, value));
    }
    fn set_void1(&mut self, _: u64, player: u64) {
        self.payloads.push(player);
    }
    fn post_event(&mut self, _: u64) {}
    fn release_event(&mut self, _: u64) {
        self.released += 1;
    }
    fn division_position(&mut self, _: u64, _: u32, _: u32, _: [i32; 3]) {
        self.order.push(33);
    }
    fn division_enabled(&mut self, _: u64, col: u32, row: u32, enabled: bool) {
        self.divisions.push((col, row, enabled));
        self.order.push(32);
    }
}

#[test]
fn aa10_14_empty_group_mean_is_zero_and_no_loop_starts() {
    let mut layer = layer();
    let mut filters = Host::default();
    let mut events = Events::default();
    let mut globals = LayerGlobals::default();
    layer
        .filters
        .create(&[vec![1], vec![], vec![]], &mut globals, &mut filters);
    layer.loops.push(LoopEntry {
        sound: 9,
        instance: 0,
        last_position: [0.0; 3],
        last_volume: 0,
    });
    layer
        .update(
            33,
            [1.0; 3],
            [0.0; 3],
            LayerUpdateInput {
                sound_enabled: true,
                population: None,
                zoom: 0,
                zoom_height_five: 0.0,
            },
            &mut filters,
            &mut events,
            &mut crate::audio::instance_id::InstanceIdCounter::new(),
        )
        .unwrap();
    assert_eq!(layer.score, 0.0);
    assert!(events.ids.is_empty());
}

#[test]
fn aa10_15_freshness_unsigned_wrap_and_preference_scaling() {
    let mut layer = layer();
    layer.can_handle_this_update = true;
    layer.score = 20.0;
    layer.update_freshness_parameters();
    assert_eq!((layer.score_volume, layer.play_percent), (1024, 75));
    layer.tuning.curves[1] = ResponseCurve::from_points(&[(0.0, -1.0)]);
    layer.tuning.preference_mode = 0;
    layer.update_freshness_parameters();
    assert_eq!((layer.score_volume, layer.play_percent), (4_194_303, 50));
    layer.can_handle_this_update = false;
    layer.update_freshness_parameters();
    assert_eq!((layer.score_volume, layer.play_percent), (0, 0));
}

#[test]
fn aa10_16_score_start_updates_same_tick_and_failed_fade_falls_through() {
    let mut layer = layer();
    layer.can_handle_this_update = true;
    layer.score = 10.0;
    layer.group_count = 1;
    layer.samples = vec![[1.9, -2.9, 3.9]];
    layer.tuning.preference_mode = 0;
    layer.scores.push(ScoreEntry {
        playing: false,
        ready: true,
        last_percent: 99,
        last_volume: 99,
        player: 7,
    });
    let mut events = Events::default();
    layer.update_scores(1000.0, &mut events);
    assert_eq!(events.ids, [0xEA2D_827C, 0xEA2C_0390, 0x4A3D_A208, 0x0A41_4C5C]);
    assert_eq!(&events.words[..3], [(1, 1000), (2, 16000), (1, 0)]);
    assert_eq!(&events.order[..4], [0xEA2D_827C, 0xEA2C_0390, 33, 32]);
    assert_eq!(events.divisions.len(), 9);
    assert_eq!(events.released, 4);
    assert_eq!(events.payloads, [7; 4]);
    let mut events = Events::default();
    layer.update_scores(1000.0, &mut events);
    assert!(events.ids.is_empty());
    assert_eq!(events.divisions.len(), 9);
    layer.paused = true;
    let mut events = Events {
        failed: Some(0x2A2C_0393),
        ..Events::default()
    };
    layer.update_scores(1000.0, &mut events);
    assert!(layer.scores[0].playing);
    assert_eq!(events.divisions.len(), 9);
    events.failed = None;
    events.divisions.clear();
    layer.update_scores(1000.0, &mut events);
    assert!(!layer.scores[0].playing);
    assert!(events.divisions.is_empty());
}

#[test]
fn aa10_17_easing_scalar_and_vector_rounding_stages() {
    let mut ease = layer().ease;
    ease.stepped = true;
    ease.set_scalar(512, false);
    for value in [132, 264, 396, 512] {
        ease.step(33.0);
        assert_eq!(ease.current_scalar, value);
    }
    ease.set_position([3.0, 0.0, 4.0], false);
    ease.step(20.0);
    assert_eq!(ease.current_position, [1.2, 0.0, 1.6]);
    ease.set_scalar(0, true);
    assert_eq!(ease.current_scalar, 0);
}

#[test]
fn aa10_18_mode_six_is_explicit_native_fault() {
    let mut layer = layer();
    layer.tuning.score_mode = 6;
    let mut filters = Host::default();
    layer
        .filters
        .create(&[vec![1], vec![], vec![]], &mut LayerGlobals::default(), &mut filters);
    let mut events = Events::default();
    assert_eq!(
        layer.update(
            33,
            [0.0; 3],
            [0.0; 3],
            LayerUpdateInput {
                sound_enabled: true,
                population: None,
                zoom: 0,
                zoom_height_five: 0.0
            },
            &mut filters,
            &mut events,
            &mut crate::audio::instance_id::InstanceIdCounter::new()
        ),
        Err(LayerUpdateError::ModeSixNativeFault)
    );
    assert!(events.ids.is_empty());
}

#[test]
fn loop_start_volume_position_stop_and_release_order() {
    let mut layer = layer();
    layer.can_handle_this_update = true;
    layer.score = 2.0;
    layer.average = [1.9, 2.9, -3.9];
    layer.tuning.curves[2] = ResponseCurve::from_points(&[(0.0, 512.0)]);
    layer.loops.push(LoopEntry {
        sound: 99,
        instance: 0,
        last_position: [0.0; 3],
        last_volume: 0,
    });
    let mut events = Events::default();
    let mut ids = crate::audio::instance_id::InstanceIdCounter::at(42);
    layer.update_loops(33, &mut events, &mut ids);
    assert_eq!(events.ids, [0xEA45_1903, 0x4A45_1915, 0xCA45_190F]);
    assert_eq!(events.released, 3);
    assert_eq!(layer.loops[0].instance, 42);
    assert_eq!(&events.words[8..], [(1, 42), (2, 1), (3, 2), (4, (-3i32) as u32)]);
    layer.can_handle_this_update = false;
    events.ids.clear();
    layer.update_loops(33, &mut events, &mut ids);
    assert_eq!(events.ids, [0x6A45_1908]);
    assert_eq!(layer.loops[0].instance, 0);
}

#[test]
fn cell_cache_is_shared_and_group_intersection_keeps_inner_centers_once() {
    let mut globals = LayerGlobals::default();
    let mut host = Host::default();
    let mut layer = layer();
    layer
        .filters
        .create(&[vec![1, 2], vec![], vec![]], &mut globals, &mut host);
    layer.begin_update([0.0; 4]);
    layer.handle_cell(4, 5, true, &mut globals, &mut host);
    layer.begin_update([0.0; 4]);
    layer.handle_cell(4, 5, true, &mut globals, &mut host);
    assert_eq!(host.cells, 1);
    assert_eq!(layer.counters, [0, 1, 0, 0]);
    let mut a = GroupLayer::new(10.0);
    let mut b = GroupLayer::new(10.0);
    a.insert([0.0; 3], 0.0);
    b.insert([9.0, 0.0, 0.0], 0.0);
    let mut target = GroupLayer::new(0.0);
    target.intersect(&a, &b, false);
    assert_eq!(target.groups[0].center, [0.0; 3]);
    target.intersect(&a, &b, true);
    assert_eq!(target.groups[0].center, [9.0, 0.0, 0.0]);
}
