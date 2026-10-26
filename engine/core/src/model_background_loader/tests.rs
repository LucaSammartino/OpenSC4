// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Default)]
struct Fake {
    events: Vec<Event>,
    released: Vec<Handle>,
    loaded: Vec<Handle>,
    keys: Vec<ResourceKey>,
    cells: VecDeque<Cell>,
    tuning: Option<Tuning>,
    ring: i32,
    expire: bool,
    query_count: usize,
    speed: u32,
    filter: [u32; 2],
    spiral: Option<(Rect, Cell)>,
    all_loaded: bool,
}
impl Host for Fake {
    fn force_speed_three(&mut self) -> bool {
        false
    }
    fn loader_speed_option(&mut self) -> u32 {
        self.speed
    }
    fn tuning(&mut self, _: u32) -> Option<Tuning> {
        self.tuning.clone()
    }
    fn acquire(&mut self, b: Binding) -> Handle {
        100 + b as u64
    }
    fn release(&mut self, h: Handle) {
        self.released.push(h);
    }
    fn city_cell_counts(&mut self) -> [i32; 2] {
        [64, 32]
    }
    fn thread_running(&mut self) -> bool {
        false
    }
    fn event(&mut self, e: Event) {
        self.events.push(e);
    }
    fn memory_mib(&mut self) -> Option<u32> {
        Some(2048)
    }
    fn screen_width(&mut self) -> Option<u32> {
        None
    }
    fn view_exists(&mut self) -> bool {
        true
    }
    fn set_filter_view(&mut self, z: u32, r: u32) {
        self.filter = [z, r];
    }
    fn reset_spiral(&mut self, r: Rect, c: Cell) {
        self.spiral = Some((r, c));
    }
    fn next_spiral_cell(&mut self) -> Option<Cell> {
        self.cells.pop_front()
    }
    fn spiral_ring(&mut self) -> i32 {
        self.ring
    }
    fn occupants(&mut self, _: Cell, z: u32, r: u32) -> Vec<Handle> {
        assert_eq!([z, r], self.filter);
        self.query_count += 1;
        vec![1, 2]
    }
    fn resource_occupant(&mut self, _: Handle) -> ResourceOccupant {
        ResourceOccupant {
            zoom_mask: if self.all_loaded { 31 } else { 0 },
            rotation_mask: 15,
            ..Default::default()
        }
    }
    fn model_ids(&mut self, _: Handle, q: ModelQuery, ids: &mut Vec<u32>, key: &mut ResourceKey) {
        assert_eq!(ids, &[0; 4]);
        assert_eq!(q.zoom, self.filter[0]);
        assert_eq!(*key, ResourceKey::default());
        *ids = vec![0, 7, 8];
    }
    fn load_resource(&mut self, key: ResourceKey) -> Option<Handle> {
        self.keys.push(key);
        Some(500)
    }
    fn load_occupant_models(&mut self, h: Handle, _: u32, _: u32) {
        self.loaded.push(h);
    }
    fn restart_timer(&mut self, _: Timer) {}
    fn timer_passed(&mut self, _: Timer, _: f32) -> bool {
        self.expire
    }
}

#[test]
fn m24_radius_m25_pause_dedup_and_lifetime() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<ModelBackgroundLoader>();
    let loader = ModelBackgroundLoader::default();
    let mut tuning = Tuning::default();
    tuning.minimum_radius[4] = 5;
    tuning.radius_factor[4] = 0.0125;
    let mut host = Fake {
        tuning: Some(tuning),
        ..Default::default()
    };
    assert!(loader.init(&mut host));
    assert!(loader.init(&mut host));
    assert_eq!(loader.snapshot().city_max, [63, 31]);
    let mut rect = Rect::new(100, 100, 110, 110);
    loader.expand_load_rect(&mut rect, 4, &mut host).unwrap();
    assert_eq!(rect, Rect::new(75, 75, 135, 135));
    host.events.clear();
    for c in [3, 3, 4] {
        loader.post_command(c);
    }
    assert!(loader.process_communications(&mut host).unwrap());
    assert_eq!(host.events, [Event::HiddenPause, Event::HiddenResume]);
    assert!(!loader.snapshot().simulator_paused);
    loader.post_command(2);
    loader.process_communications(&mut host).unwrap();
    host.events.clear();
    loader.shutdown(&mut host);
    assert_eq!(
        host.events,
        [Event::WakeThread, Event::WaitThread { timeout: 2_000_000 }]
    );
    assert_eq!(host.released, [102, 103, 104, 101, 100]);
    loader.shutdown(&mut host);
    assert_eq!(host.released.len(), 5);
}

#[test]
fn worker_retains_unique_resources_and_posts_completion_in_order() {
    let loader = ModelBackgroundLoader::default();
    let mut host = Fake {
        cells: [[4, 5], [5, 5]].into(),
        ring: 2,
        ..Default::default()
    };
    loader.init(&mut host);
    loader.start(4, 1, Rect::new(4, 5, 5, 5), [4, 5], &mut host).unwrap();
    assert_eq!(loader.snapshot().progress_width, 10);
    assert_eq!(loader.worker_slice(&mut host), Ok(Some(1_000_000)));
    assert_eq!(loader.snapshot().commands.values, [3, 0, 1, 4, 2]);
    assert_eq!(loader.snapshot().resource_count, 1);
    assert_eq!(host.keys.len(), 8);
    assert_eq!(host.released.iter().filter(|h| **h == 500).count(), 7);
    loader.process_communications(&mut host).unwrap();
    assert_eq!(host.loaded, [1, 2, 1, 2]);
    assert!(loader.snapshot().waiting.is_empty());
    loader.cancel(&mut host);
    assert_eq!(host.released.iter().filter(|h| **h == 500).count(), 8);
}

#[test]
fn m26_partial_main_pass_keeps_commands_and_refocus_keeps_running_state() {
    let loader = ModelBackgroundLoader::default();
    let tuning = Tuning {
        minimum_cells: Curve::Constant(0.0),
        maximum_waiting_cells: 3,
        ..Default::default()
    };
    let mut host = Fake {
        tuning: Some(tuning),
        cells: [[1, 1], [2, 1], [3, 1], [4, 1]].into(),
        ring: 2,
        expire: true,
        ..Default::default()
    };
    loader.init(&mut host);
    loader.start(3, 0, Rect::new(1, 1, 4, 1), [1, 1], &mut host).unwrap();
    loader.worker_slice(&mut host).unwrap();
    let commands = loader.snapshot().commands;
    assert!(!loader.process_communications(&mut host).unwrap());
    assert_eq!(loader.snapshot().commands, commands);
    assert_eq!(loader.snapshot().waiting.len(), 2);
    assert!(!loader.process_communications(&mut host).unwrap());
    assert!(loader.process_communications(&mut host).unwrap());
    assert_eq!(host.events.iter().filter(|e| **e == Event::HiddenPause).count(), 1);
    loader.refocus(Rect::new(10, 20, 9, 19), [10, 20], &mut host);
    let snapshot = loader.snapshot();
    assert_eq!(snapshot.tasks[0].state, TaskState::Running);
    assert_eq!(snapshot.tasks[0].progress, 0.0);
    assert_eq!(host.spiral, Some((Rect::new(10, 20, 10, 20), [10, 20])));
}

#[test]
fn query_disabled_costs_nothing_loaded_masks_skip_resources_and_pause_stops_walk() {
    let loader = ModelBackgroundLoader::default();
    let tuning = Tuning {
        query_occupants: false,
        budget: Curve::Constant(0.01),
        maximum_waiting_cells: 3,
        ..Default::default()
    };
    let mut host = Fake {
        tuning: Some(tuning),
        cells: [[1, 1], [2, 1], [3, 1], [4, 1]].into(),
        ring: 2,
        ..Default::default()
    };
    loader.init(&mut host);
    loader.start(0, 0, Rect::new(1, 1, 4, 1), [1, 1], &mut host).unwrap();
    loader.pause();
    loader.worker_slice(&mut host).unwrap();
    assert_eq!(host.cells.len(), 4);
    loader.resume();
    loader.worker_slice(&mut host).unwrap();
    assert_eq!(loader.snapshot().waiting.len(), 3);
    assert_eq!(host.query_count, 0);
    loader.cancel(&mut host);
    host.speed = 1;
    host.tuning = Some(Tuning::default());
    host.all_loaded = true;
    loader.start(0, 0, Rect::new(1, 1, 4, 1), [1, 1], &mut host).unwrap();
    loader.worker_slice(&mut host).unwrap();
    assert!(host.keys.is_empty());
    assert_eq!(loader.snapshot().speed, 1);
}

#[test]
fn mm1_is_explicit_and_vector_growth_copies_prefix_values_and_suffix() {
    assert_eq!(
        Curve::Samples(vec![(0.0, 10.0), (2.0, 20.0)]).value(1.0),
        Err(Unsupported::CurveInterpolationMm1)
    );
    assert_eq!(native_delay_duration(65536), Err(Unsupported::ClockUnitsMm1));
    let mut commands = Commands::default();
    for c in [3, 3, 4] {
        commands.push(c);
    }
    assert_eq!((commands.values.clone(), commands.capacity), (vec![3, 3, 4], 4));
    commands.grow_insert(1, 8, 3, false);
    assert_eq!(
        (commands.values.clone(), commands.capacity),
        (vec![3, 8, 8, 8, 3, 4], 6)
    );
    commands.grow_insert(2, 9, 1, true);
    assert_eq!((commands.values, commands.capacity), (vec![3, 8, 9], 12));
}

#[test]
fn screen_width_factors_preserve_all_five_spec_cases() {
    let loader = ModelBackgroundLoader::default();
    let mut host = Fake::default();
    loader.init(&mut host);
    for (zoom, expected) in [170, 91, 46, 20, 10].into_iter().enumerate() {
        loader
            .start(zoom as u32, 0, Rect::new(0, 0, 10, 10), [5, 5], &mut host)
            .unwrap();
        assert_eq!(loader.snapshot().progress_width, expected);
    }
    loader.destroy(&mut host);
}
