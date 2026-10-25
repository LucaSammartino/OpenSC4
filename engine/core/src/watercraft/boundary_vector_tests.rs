// SPDX-License-Identifier: GPL-3.0-or-later
//! Boundary vectors WC1-I04/I05, WC3-G01–G03, H01, H03, H04, I01–I03,
//! J01–J02, LC01–LC02, DBG01–DBG02, WC4-AN01–AN03, I02, J01–J03, WC2-P04.
use super::tests::{boat, Water};
use super::*;
use crate::automata::{State, Vec3};
use std::cell::RefCell;
use std::sync::Arc;

#[test]
fn wc1_i04_i05_idle_threshold_is_strict_and_independent_of_mode() {
    let mut f = Fleet::default();
    let mut b = boat();
    b.count_idle_always = true;
    b.idle_elapsed = 59_990.0;
    f.insert(1, b).unwrap();
    assert_eq!(f.animation_budget(1, 1.0, 10.0, &mut None), Ok(true));
    assert_eq!(f.get(1).unwrap().idle_elapsed, 60_000.0);
    assert_eq!(f.get(1).unwrap().top_state(), None);
    assert_eq!(f.animation_budget(1, 1.0, 0.5, &mut None), Ok(true));
    assert_eq!(f.get(1).unwrap().top_state(), Some(state::SLOW_REMOVE));
    // Mode false: no accumulation, but the threshold is still tested.
    let mut f = Fleet::default();
    let mut b = boat();
    b.idle_elapsed = 60_001.0;
    f.insert(1, b).unwrap();
    assert_eq!(f.animation_budget(1, 1.0, 10.0, &mut None), Ok(true));
    assert_eq!(f.get(1).unwrap().idle_elapsed, 60_001.0);
    assert_eq!(f.get(1).unwrap().top_state(), Some(state::SLOW_REMOVE));
    // Speed modifier below 1e-5 returns before accumulation or the test.
    let mut f = Fleet::default();
    let mut b = boat();
    b.count_idle_always = true;
    b.idle_elapsed = 60_001.0;
    f.insert(1, b).unwrap();
    assert_eq!(f.animation_budget(1, 0.000_009, 10.0, &mut None), Ok(false));
    assert_eq!(f.get(1).unwrap().idle_elapsed, 60_001.0);
    assert_eq!(f.get(1).unwrap().top_state(), None);
}

#[test]
fn wc3_g01_g02_endpoints_and_extent_use_runtime_pose_without_normalizing() {
    let mut b = boat();
    b.position = [10.0, 20.0, 30.0];
    b.heading = [2.0, 3.0, 4.0];
    b.radius = 5.0;
    assert_eq!(b.rear_endpoint(), [0.0, 5.0, 10.0]);
    assert_eq!(b.front_endpoint(), [20.0, 35.0, 50.0]);
    assert_eq!(b.heading, [2.0, 3.0, 4.0]);
    assert_eq!(Watercraft::maximum_extent([[1.0, 2.0, 3.0], [4.0, 12.0, 8.0]]), 10.0);
}

struct Recording {
    outside: Cell,
    queries: RefCell<Vec<Cell>>,
}
impl Terrain for Recording {
    fn contains(&self, cell: Cell) -> bool {
        self.queries.borrow_mut().push(cell);
        cell != self.outside
    }
    fn cell_at(&self, _: Vec3) -> Cell {
        panic!("no conversion")
    }
    fn terrain_height(&self, _: Vec3) -> f32 {
        panic!("no terrain")
    }
    fn terrain_height_at_cell(&self, _: Cell) -> f32 {
        panic!("no terrain")
    }
    fn sea_level(&self) -> f32 {
        0.0
    }
    fn water_height(&self, _: Vec3) -> f32 {
        panic!("no water")
    }
    fn navigable(&self, _: Cell, _: u32, _: Option<&Resource>) -> bool {
        true
    }
}

fn rectangle() -> [Cell; 4] {
    [
        Cell { x: 2, z: 3 },
        Cell { x: 2, z: 5 },
        Cell { x: 4, z: 3 },
        Cell { x: 4, z: 5 },
    ]
}

#[test]
fn wc3_g03_outside_city_short_circuits_in_specified_corner_order() {
    let terrain = Recording {
        outside: Cell { x: 2, z: 5 },
        queries: RefCell::new(Vec::new()),
    };
    assert!(Watercraft::outside_city(rectangle(), &terrain));
    assert_eq!(
        *terrain.queries.borrow(),
        [Cell { x: 2, z: 3 }, Cell { x: 4, z: 3 }, Cell { x: 2, z: 5 }]
    );
    let inside = Recording {
        outside: Cell { x: -9, z: -9 },
        queries: RefCell::new(Vec::new()),
    };
    assert!(!Watercraft::outside_city(rectangle(), &inside));
    assert_eq!(inside.queries.borrow().len(), 4);
}

#[test]
fn wc3_h01_flag_1_returns_false_before_city_or_state_queries() {
    let terrain = Recording {
        outside: Cell { x: 2, z: 3 },
        queries: RefCell::new(Vec::new()),
    };
    let mut f = Fleet::default();
    let mut b = boat();
    b.record.base.automata_flags |= 1;
    b.record.base.states.push(State::new(state::RUN_AGROUND));
    f.insert(1, b).unwrap();
    assert_eq!(f.movement_possible(1, rectangle(), &terrain, &mut None), Ok(false));
    assert!(terrain.queries.borrow().is_empty());
    assert_eq!(f.get(1).unwrap().record.base.states.len(), 1);
}

#[derive(Default)]
struct Manager(Vec<&'static str>);
impl DockManager for Manager {
    fn take_berth(&mut self, _: &Resource, _: BoatId) -> bool {
        unreachable!()
    }
    fn take_waiting_spot(&mut self, _: &Resource, _: BoatId) -> bool {
        unreachable!()
    }
    fn done_docking(&mut self, _: &Resource, _: BoatId) {
        self.0.push("done");
    }
    fn may_stay(&mut self, _: BoatId) -> bool {
        unreachable!()
    }
}

fn held(lot_position: Vec3) -> Fleet {
    let mut b = boat();
    b.drive.0[36] = 5.0;
    b.inherited_position = [0.0; 3];
    b.position = [100.0; 3];
    b.current_station = Some(Arc::new(Station {
        lot: Arc::new(Resource(1)),
        occupant: Arc::new(Resource(2)),
        lot_position,
        position: [100.0; 3],
        direction: [1.0, 0.0, 0.0],
        route: Arc::from(vec![]),
    }));
    b.held_berth = true;
    b.docking_in_progress = true;
    b.record.base.automata_flags |= 0x1000_0000;
    let mut f = Fleet::default();
    f.insert(1, b).unwrap();
    f
}

#[test]
fn wc3_h03_clear_dock_requests_uses_occupant_position_and_strict_radius() {
    let mut manager = Manager::default();
    let mut f = held([0.0, 5.0, 0.0]);
    let mut service: Option<&mut dyn DockManager> = Some(&mut manager);
    f.clear_dock_requests(1, &mut service).unwrap();
    assert!(manager.0.is_empty());
    assert!(f.get(1).unwrap().held_berth);
    let mut f = held([0.0, 6.0, 0.0]);
    let mut service: Option<&mut dyn DockManager> = Some(&mut manager);
    f.clear_dock_requests(1, &mut service).unwrap();
    assert_eq!(manager.0, ["done"]);
    assert!(!f.get(1).unwrap().held_berth);
}

#[test]
fn wc3_h04_end_docking_without_manager_clears_only_docking_in_progress() {
    let mut f = held([0.0; 3]);
    f.end_docking(1, &mut None).unwrap();
    let b = f.get(1).unwrap();
    assert!(!b.docking_in_progress);
    assert!(b.held_berth);
    assert_ne!(b.record.base.automata_flags & 0x1000_0000, 0);
    assert!(b.current_station.is_some());
}

struct Factory {
    names: Vec<&'static str>,
    fail: bool,
}
impl EffectResources for Factory {
    fn create(&mut self, name: &'static str) -> Option<Arc<Resource>> {
        self.names.push(name);
        (!self.fail).then(|| Arc::new(Resource(self.names.len() as u64)))
    }
}

#[test]
fn wc3_i01_through_i03_init_once_ignores_factory_failure_and_resets_timers() {
    let mut b = boat();
    b.lifetime_limit = 1000.0;
    b.lifetime_elapsed = 500.0;
    b.idle_elapsed = 1.0;
    b.crash_elapsed = 2.0;
    b.still_elapsed = 3.0;
    b.backwards_elapsed = 4.0;
    let mut factory = Factory {
        names: Vec::new(),
        fail: true,
    };
    assert_eq!(b.initialize_runtime([1.0; 3], [0.0, 0.0, 2.0], &mut factory), Ok(true));
    assert_eq!(factory.names, EFFECTS);
    assert!(b.effects.iter().all(Option::is_none));
    assert_eq!(b.record.base.automata_flags & 0x2_A004, 0x2_A004);
    assert_eq!(
        [
            b.lifetime_limit,
            b.lifetime_elapsed,
            b.idle_elapsed,
            b.crash_elapsed,
            b.still_elapsed,
            b.backwards_elapsed
        ],
        [0.0; 6]
    );
    assert_eq!(b.heading, [0.0, 0.0, 1.0]);
    // Already initialized: no reset or acquisition.
    b.lifetime_elapsed = 7.0;
    let mut again = Factory {
        names: Vec::new(),
        fail: false,
    };
    assert_eq!(b.initialize_runtime([0.0; 3], [1.0, 0.0, 0.0], &mut again), Ok(true));
    assert!(again.names.is_empty());
    assert_eq!(b.lifetime_elapsed, 7.0);
}

struct LandAt {
    inside: bool,
    terrain: f32,
    water: f32,
    calls: RefCell<Vec<&'static str>>,
}
impl Terrain for LandAt {
    fn contains(&self, _: Cell) -> bool {
        self.calls.borrow_mut().push("contains");
        self.inside
    }
    fn cell_at(&self, _: Vec3) -> Cell {
        self.calls.borrow_mut().push("cell");
        Cell { x: 1, z: 1 }
    }
    fn terrain_height(&self, _: Vec3) -> f32 {
        panic!("OnLand samples the cell height")
    }
    fn terrain_height_at_cell(&self, _: Cell) -> f32 {
        self.calls.borrow_mut().push("terrain");
        self.terrain
    }
    fn sea_level(&self) -> f32 {
        0.0
    }
    fn water_height(&self, _: Vec3) -> f32 {
        self.calls.borrow_mut().push("water");
        self.water
    }
    fn navigable(&self, _: Cell, _: u32, _: Option<&Resource>) -> bool {
        true
    }
}

#[test]
fn wc3_j01_j02_on_land_outside_short_circuit_and_strict_comparison() {
    let b = boat();
    let outside = LandAt {
        inside: false,
        terrain: 99.0,
        water: 0.0,
        calls: RefCell::new(Vec::new()),
    };
    assert!(!b.on_land(&outside));
    assert_eq!(*outside.calls.borrow(), ["cell", "contains"]);
    for (water, land) in [(10.0, false), (9.0, true)] {
        let t = LandAt {
            inside: true,
            terrain: 10.0,
            water,
            calls: RefCell::new(Vec::new()),
        };
        assert_eq!(b.on_land(&t), land);
        assert_eq!(*t.calls.borrow(), ["cell", "contains", "terrain", "water"]);
    }
}

#[test]
fn wc3_lc01_lc02_tuning_defaults_and_local_list_precedence() {
    let mut b = boat();
    b.drive.0 = [123.0; 40];
    b.group = None;
    b.load_drive_parameters();
    assert_eq!(b.drive, DriveParameters::default());
    let mut list = [0.0; 40];
    list[9] = 3.0;
    let parent = Arc::new(Group {
        id: 1,
        parameters: Some([7.0; 40]),
        aems_value: None,
        parent: None,
    });
    b.group = Some(Arc::new(Group {
        id: 2,
        parameters: Some(list),
        aems_value: None,
        parent: Some(parent.clone()),
    }));
    b.load_drive_parameters();
    assert_eq!(b.drive.0[9], 3.0);
    assert_eq!(b.drive.0[8], -3.0);
    b.group = Some(Arc::new(Group {
        id: 2,
        parameters: Some([0.0; 40]),
        aems_value: None,
        parent: Some(parent),
    }));
    b.load_drive_parameters();
    assert_eq!(b.drive, DriveParameters::load(Some(&[0.0; 40])));
    assert_eq!(b.drive.0[9], 2.0);
}

#[test]
fn wc2_p04_entry_34_is_retained_without_behavior() {
    let mut list = [0.0; 40];
    list[34] = 77.0;
    let tuned = DriveParameters::load(Some(&list));
    assert_eq!(tuned.0[34], 77.0);
    // Present-list dependent defaults still apply; only entry 34 differs.
    let mut expected = DriveParameters::load(Some(&[0.0; 40]));
    expected.0[34] = 77.0;
    assert_eq!(tuned, expected);
}

#[test]
fn wc3_dbg01_dbg02_shared_debug_text_and_formatting() {
    let mut a = boat();
    a.queued = true;
    a.docking = true;
    a.position = [1.0, 2.0, 3.0];
    a.heading = [0.0, 0.0, 1.0];
    let text = a.debug_extra_text([2, 3], Cell { x: -1, z: -1 });
    assert!(text.contains("(Q,D): (1,1)"), "{text}");
    assert!(text.contains("Cell( -1, -1)"), "{text}");
    assert!(text.contains("Pos(1.00,2.00,3.00)"), "{text}");
    assert!(text.contains("Ornt<0.00,0.00,1.00>"), "{text}");
    let first = {
        let guard = a.borrowed_debug_extra_text([0, 0], Cell { x: 0, z: 0 });
        &*guard as *const String
    };
    let mut b = boat();
    b.position = [9.0, 9.0, 9.0];
    let guard = b.borrowed_debug_extra_text([0, 0], Cell { x: 0, z: 0 });
    assert_eq!(&*guard as *const String, first);
    assert!(guard.contains("Pos(9.00,9.00,9.00)"));
}

#[derive(Default)]
struct Model(Vec<String>);
impl AnimatedModel for Model {
    fn visibility(&mut self, translucent: bool, value: f32) {
        self.0.push(format!("vis {translucent} {value}"));
    }
    fn advance(&mut self, time: f32) {
        self.0.push(format!("advance {time}"));
    }
    fn frame_count(&self) -> f32 {
        10.0
    }
    fn frame(&self) -> f32 {
        0.0
    }
}

#[test]
fn wc4_an01_visibility_requests_only_on_changed_samples() {
    let mut m1 = Model::default();
    let mut m2 = Model::default();
    let mut s = Subsidiary {
        visibility: SpeedTween {
            start: 0.5,
            target: 0.5,
            duration: 1.0,
            elapsed: 0.0,
        },
        ..Subsidiary::default()
    };
    {
        let mut models: [Option<&mut dyn AnimatedModel>; 3] = [Some(&mut m1), None, Some(&mut m2)];
        s.visibility_tick(100.0, &mut models);
        s.visibility = SpeedTween {
            start: 0.0,
            target: 1.0,
            duration: 1.0,
            elapsed: 0.0,
        };
        s.visibility_tick(1000.0, &mut models);
        s.visibility = SpeedTween {
            start: 0.0,
            target: 1.0,
            duration: 1.0,
            elapsed: 0.0,
        };
        s.visibility_tick(500.0, &mut models);
    }
    assert_eq!(m1.0, ["vis false 1", "vis true 0.5"]);
    assert_eq!(m2.0, m1.0);
}

struct Service {
    first: bool,
    second: bool,
    queried: RefCell<Vec<u32>>,
}
impl AnimationService for Service {
    fn slot17(&mut self) -> bool {
        self.queried.borrow_mut().push(17);
        self.first
    }
    fn slot15(&mut self) -> bool {
        self.queried.borrow_mut().push(15);
        self.second
    }
}

#[test]
fn wc4_an02_suppressed_animation_writes_nothing() {
    let mut b = boat();
    let mut s = Subsidiary {
        attachments_present: true,
        ..Subsidiary::default()
    };
    let mut m = Model::default();
    let mut service = Service {
        first: false,
        second: true,
        queried: RefCell::new(Vec::new()),
    };
    {
        let mut models: [Option<&mut dyn AnimatedModel>; 1] = [Some(&mut m)];
        assert!(s.animation_tick(10.0, &mut service, &mut models, &mut b));
    }
    assert!(m.0.is_empty());
    assert_eq!(b.models.event_counter, 0.0);
    assert!(b.events.is_empty());
    assert_eq!(*service.queried.borrow(), [17, 15]);
    let mut service = Service {
        first: true,
        second: true,
        queried: RefCell::new(Vec::new()),
    };
    {
        let mut models: [Option<&mut dyn AnimatedModel>; 1] = [Some(&mut m)];
        assert!(s.animation_tick(10.0, &mut service, &mut models, &mut b));
    }
    assert_eq!(*service.queried.borrow(), [17]);
    assert_eq!(m.0, ["advance 10"]);
    assert_eq!(b.models.event_counter, 10.0);
}

#[derive(Default)]
struct Lights(Vec<bool>);
impl Lighting for Lights {
    fn light_model(&mut self, control: bool) {
        self.0.push(control);
    }
}

#[test]
fn wc4_an03_lighting_thresholds_are_strict_and_flag_suppresses() {
    let mut s = Subsidiary::default();
    let mut lights = Lights::default();
    let mut b = boat();
    b.inherited_heading = [0.5, 0.25, 0.125];
    b.inherited_position = [2.0, 0.0, 0.0];
    s.lighting_tick(&b, true, [0.5, 0.25, 0.125], 4.0, Some(&mut lights));
    assert!(lights.0.is_empty());
    assert_eq!(s.last_lit_position, [0.0; 3]);
    b.inherited_position = [2.0, 0.0, 0.001];
    b.record.base.automata_flags |= 0x10_0000;
    s.lighting_tick(&b, true, [0.5, 0.25, 0.125], 4.0, Some(&mut lights));
    assert!(lights.0.is_empty());
    b.record.base.automata_flags &= !0x10_0000;
    s.lighting_tick(&b, true, [0.5, 0.25, 0.125], 4.0, Some(&mut lights));
    assert_eq!(lights.0, [false]);
    assert_eq!(s.last_lit_position, [2.0, 0.0, 0.001]);
    assert_eq!(s.last_lit_heading, [0.5, 0.25, 0.125]);
}

#[test]
fn wc4_i02_true_state_tick_suppresses_generic_seek() {
    let mut b = boat();
    b.record.base.automata_flags = 2;
    b.inherited_position = [1.0, 0.0, 0.0];
    b.inherited_destination = [50.0, 0.0, 0.0];
    let mut received = Vec::new();
    assert!(b.inherited_animation(
        5.0,
        7.0,
        |_, _, _| {},
        |_, a| {
            received.push(a);
            true
        },
        1.0,
        |_, _, _| panic!("no generic seek"),
    ));
    assert_eq!(received, [7.0]);
    assert_eq!(b.inherited_position, [1.0, 0.0, 0.0]);
    assert_eq!(b.events, [Event::Notify(0, 0)]);
}

struct Spatial;
impl SpatialServices for Spatial {
    fn centre(&self, c: Cell) -> Vec3 {
        [c.x as f32 + 0.5, 0.0, c.z as f32 + 0.5]
    }
    fn adjust_segment(&mut self, _: Vec3, _: &mut Vec3, _: u32) {}
}

#[derive(Default)]
struct Queries(usize);
impl CollisionQueries for Queries {
    fn cell_rectangle(&mut self, _: BoatId) -> [Cell; 4] {
        self.0 += 1;
        [Cell { x: 50, z: 50 }; 4]
    }
    fn occupied(&mut self, _: Cell) -> bool {
        false
    }
    fn is_wet(&mut self, _: Cell) -> bool {
        true
    }
    fn terrain_normal(&mut self, _: Vec3) -> Vec3 {
        [0.0, 1.0, 0.0]
    }
}

fn far(position: Vec3, berth: Vec3, heading: Vec3) -> Fleet {
    let mut b = boat();
    b.position = position;
    b.inherited_position = position;
    b.heading = heading;
    b.inherited_heading = heading;
    b.route_start = [-1.0; 3];
    b.docking_in_progress = true;
    let station = Arc::new(Station {
        lot: Arc::new(Resource(1)),
        occupant: Arc::new(Resource(2)),
        lot_position: berth,
        position: berth,
        direction: [1.0, 0.0, 0.0],
        route: Arc::from(vec![]),
    });
    b.stations = [Some(station.clone()), Some(station)];
    let mut f = Fleet::default();
    f.insert(1, b).unwrap();
    f
}

#[test]
fn wc4_j01_j02_far_approach_runs_ai_motion_and_collision_and_returns_true() {
    for dt in [150.0, -5.0] {
        let mut f = far([50.0, 0.0, 50.0], [50.0, 91.0, 50.0], [2.0, 0.0, 0.0]);
        let mut queries = Queries::default();
        let mut spatial = Spatial;
        let mut motion = MotionServices {
            terrain: &Water,
            queries: &mut queries,
            spatial: &mut spatial,
            ai_collisions: false,
        };
        assert_eq!(f.docking_tick_core(1, dt, &mut motion, &mut None), Ok(true));
        // Detection ran (its rectangle query), whatever the AI selected.
        assert!(queries.0 >= 1);
        // Final approach would have published MoveTo toward the berth.
        assert!(!f
            .get(1)
            .unwrap()
            .events
            .iter()
            .any(|e| matches!(e, Event::Publish(Publication::MoveTo(p, _)) if p[1] > 0.0)));
    }
}

#[test]
fn wc4_j03_on_land_at_range_equality_pushes_72_flattens_and_continues_final_approach() {
    let land = LandAt {
        inside: true,
        terrain: 10.0,
        water: 0.0,
        calls: RefCell::new(Vec::new()),
    };
    let mut f = far([0.0; 3], [90.0, 0.0, 0.0], [3.0, 4.0, 0.0]);
    struct NoQueries;
    impl CollisionQueries for NoQueries {
        fn cell_rectangle(&mut self, _: BoatId) -> [Cell; 4] {
            panic!("no far-branch collision")
        }
        fn occupied(&mut self, _: Cell) -> bool {
            unreachable!()
        }
        fn is_wet(&mut self, _: Cell) -> bool {
            unreachable!()
        }
        fn terrain_normal(&mut self, _: Vec3) -> Vec3 {
            unreachable!()
        }
    }
    let mut spatial = Spatial;
    let mut motion = MotionServices {
        terrain: &land,
        queries: &mut NoQueries,
        spatial: &mut spatial,
        ai_collisions: false,
    };
    assert_eq!(f.docking_tick_core(1, 1.0, &mut motion, &mut None), Ok(true));
    let b = f.get(1).unwrap();
    assert_eq!(b.top_state(), Some(state::IMMEDIATE_REMOVE));
    assert_eq!(b.inherited_heading, [1.0, 0.0, 0.0]);
    assert_eq!(b.inherited_position, [1.0, 0.0, 0.0]);
}
