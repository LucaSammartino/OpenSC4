//! lot-developer §16 tests 44–47, §6.7.16 visual vectors and owner boundaries.
use fc4_core::lot_records::construction::*;
use fc4_core::lot_records::Bounds;
use fc4_core::rng::{CrtRand, FixedSeeds, RzRandom};
use fc4_core::stream::RefValue;

include!("construction_owner/visual_vectors.rs");

fn reference(token: u64) -> RefValue {
    RefValue::Host {
        token,
        class_id: IID_PROP,
    }
}
fn bounds(x0: f32, z0: f32, x1: f32, z1: f32) -> Bounds {
    ([x0, 0.0, z0], [x1, 1.0, z1])
}
fn statics() -> ConstructionStatics {
    ConstructionStatics {
        tuning: ConstructionTuning::default(),
        rng: RzRandom::new(
            12345,
            &FixedSeeds {
                wall_clock: 1,
                process_clock: 1,
            },
        ),
        crt: CrtRand::new(1),
        cranes: [CraneSize::default(); 3],
        ground: vec![],
        roof: vec![],
        girder_plank: None,
        crane_post_half: None,
    }
}

#[test]
fn test44_divide_lot_and_nan_and_inverted() {
    let a = divide_lot(bounds(0.0, 0.0, 48.0, 32.0), bounds(10.0, 8.0, 30.0, 20.0), 2.0);
    let expected = [
        bounds(8.0, 0.0, 32.0, 6.0),
        bounds(32.0, 6.0, 48.0, 22.0),
        bounds(8.0, 22.0, 32.0, 32.0),
        bounds(0.0, 6.0, 8.0, 22.0),
        bounds(0.0, 0.0, 8.0, 6.0),
        bounds(32.0, 0.0, 48.0, 6.0),
        bounds(32.0, 22.0, 48.0, 32.0),
        bounds(0.0, 22.0, 8.0, 32.0),
    ];
    assert_eq!(a, expected);
    let a = divide_lot(bounds(0.0, 0.0, 48.0, 32.0), bounds(f32::NAN, 40.0, 60.0, 50.0), 2.0);
    assert!(a[0].0[0].is_nan());
    assert!(a[1].1[2] < a[1].0[2]);
}

#[test]
fn test45_crane_swing_and_fit_draws() {
    use std::f32::consts::{PI, TAU};
    let world = bounds(0.0, 0.0, 48.0, 32.0);
    assert_eq!(swing_limits(world, [24.0, 0.0, 5.0], 0, 20.0), (PI, TAU));
    let (lo, hi) = swing_limits(world, [10.0, 0.0, 5.0], 0, 20.0);
    assert!((lo - (1.5 * PI - PI / 6.0)).abs() < 0.000001);
    assert!((hi - (1.5 * PI + PI / 6.0)).abs() < 0.000001);
    let mut st = statics();
    let mut a = divide_lot(world, bounds(10.0, 8.0, 30.0, 20.0), 2.0);
    let mut expected = st.rng.clone();
    assert!(outer_crane(&mut a, 20.0, &mut st).is_none());
    assert_eq!(st.rng.next_raw(), expected.next_raw());
    st.cranes[1] = CraneSize {
        width: 6.0,
        reach: 12.0,
        back: 2.0,
    };
    let before = st.rng.clone();
    let fit = outer_crane(&mut a, 20.0, &mut st).unwrap();
    let mut expected = before;
    expected.uniform_u32(4);
    assert_eq!(fit.crane_type, 1);
    assert_eq!(st.rng.next_raw(), expected.next_raw());
    let mut rng = before_inner();
    let mut expected = rng.clone();
    let raw = expected.next_raw();
    let (side, size) = inner_crane_choice(bounds(0.0, 0.0, 30.0, 10.0), &st.cranes, &mut rng);
    assert_eq!(side, if raw < 0x7FFF_FFFF { 0 } else { 2 });
    assert_eq!(size, Some(1));
    assert_eq!(rng.next_raw(), expected.next_raw());
}

fn before_inner() -> RzRandom {
    statics().rng
}

#[test]
fn test46_remove_rectangle_and_ties_and_empty() {
    let mut a = bounds(0.0, 0.0, 10.0, 10.0);
    remove_rectangle(&mut a, bounds(4.0, 7.0, 6.0, 10.0));
    assert_eq!(a, bounds(0.0, 0.0, 10.0, 7.0));
    let mut a = bounds(0.0, 0.0, 10.0, 10.0);
    remove_rectangle(&mut a, bounds(4.0, 4.0, 6.0, 6.0));
    assert_eq!(a, bounds(0.0, 0.0, 10.0, 4.0));
    let r = a;
    remove_rectangle(&mut a, r);
    assert_eq!(a, ([0.0; 3], [0.0; 3]));
}

#[derive(Default)]
struct Host {
    events: Vec<String>,
    props: Vec<u64>,
    workers: Vec<u64>,
    day: u32,
    detail: i32,
    reject: bool,
    completion: f32,
    policy: u8,
    crane_fail: bool,
    crane_requests: Vec<CraneRequest>,
    effects: Vec<EffectRequest>,
    positions: Vec<(u64, [f32; 3])>,
    moved_props: Vec<(u64, [f32; 3])>,
    moved_workers: Vec<(u64, [f32; 3])>,
    directions: Vec<[f32; 2]>,
    bindings: Option<[[TextureBinding; 5]; 6]>,
}
fn token(r: &RefValue) -> u64 {
    if let RefValue::Host { token, .. } = r {
        *token
    } else {
        0
    }
}

impl ConstructionHost for Host {
    fn occupant_base_init(&mut self) {
        self.events.push("base+".into());
    }
    fn subscribe(&mut self, m: u32) {
        self.events.push(format!("sub {m:X}"));
    }
    fn unsubscribe(&mut self, m: u32) {
        self.events.push(format!("unsub {m:X}"));
    }
    fn add_tickable(&mut self, p: u32) {
        assert_eq!(p, 0x80);
    }
    fn remove_tickable(&mut self) {
        self.events.push("tick-".into());
    }
    fn post_occupant_group_added(&mut self, _: u32) {}
    fn post_construction_message(&mut self, m: u32, d: u8) {
        self.events.push(format!("post {m:X} {d}"));
    }
    fn sim_date(&mut self) -> u32 {
        self.day
    }
    fn lot_building(&mut self, _: &RefValue) -> Option<RefValue> {
        Some(reference(2))
    }
    fn building_bounds(&mut self, _: &RefValue) -> Bounds {
        ([10.0, 100.0, 8.0], [30.0, 120.0, 20.0])
    }
    fn altitude(&mut self, _: f32, _: f32) -> f32 {
        100.0
    }
    fn construction_days(&mut self, _: &RefValue) -> Option<u8> {
        Some(30)
    }
    fn building_completion(&mut self, _: &RefValue) -> f32 {
        self.completion
    }
    fn set_building_completion(&mut self, _: &RefValue, p: f32) {
        self.completion = p;
    }
    fn post_completion_changed(&mut self, _: &RefValue) {
        self.events.push("completion".into());
    }
    fn set_lot_state(&mut self, _: &RefValue, s: u32, a: u32, b: u32) {
        assert_eq!((s, a, b), (3, 0, 0));
        self.events.push("grown".into());
    }
    fn lot_cell_rect(&mut self, _: &RefValue) -> [i32; 4] {
        [0, 0, 2, 1]
    }
    fn cell_usable(&mut self, _: i32, _: i32) -> bool {
        false
    }
    fn set_lot_texture(&mut self, _: i32, _: i32, _: u32) {}
    fn cell_corner(&mut self, x: i32, z: i32) -> [f32; 3] {
        [x as f32 * 16.0, 0.0, z as f32 * 16.0]
    }
    fn crane_policy(&mut self, _: &RefValue) -> u8 {
        self.policy
    }
    fn create_prop(&mut self, _: u32) -> Option<RefValue> {
        if self.reject {
            return None;
        }
        let id = 10 + self.props.len() as u64;
        self.props.push(id);
        Some(reference(id))
    }
    fn place_prop(&mut self, prop: &RefValue, p: [f32; 3], o: u8) {
        assert!(p[0].is_finite() && p[2].is_finite() && o <= 3);
        self.positions.push((token(prop), p));
    }
    fn render_detail(&mut self) -> i32 {
        self.detail
    }
    fn create_automaton(&mut self, _: u32, p: [f32; 3], _: bool) -> Option<RefValue> {
        assert!(p[0].is_finite() && p[2].is_finite());
        let id = 100 + self.workers.len() as u64;
        self.workers.push(id);
        self.positions.push((id, p));
        Some(reference(id))
    }
    fn worker_direction(&mut self, _: &RefValue, d: [f32; 2]) {
        assert!(d.iter().all(|v| (-1.0..=1.0).contains(v)));
        self.directions.push(d);
    }
    fn create_effect(&mut self, lot: &RefValue, r: EffectRequest) -> Option<RefValue> {
        assert_eq!(token(lot), 1);
        self.effects.push(r);
        let id = 200 + self.props.len() as u64;
        self.props.push(id);
        Some(reference(id))
    }
    fn effect_slot8(&mut self, effect: &RefValue, enabled: bool) {
        assert!(enabled);
        self.events.push(format!("effect8 {}", token(effect)));
    }
    fn building_position(&mut self, _: &RefValue) -> [f32; 3] {
        [0., 100., 0.]
    }
    fn lot_developer_building_position(&mut self, _: &RefValue) -> Option<[f32; 3]> {
        Some([0., 100., 0.])
    }
    fn associate_prop(&mut self, _: &RefValue, _: &RefValue, roof: bool) {
        self.events.push(format!("associate {roof}"));
    }
    fn prop_position(&mut self, prop: &RefValue) -> [f32; 3] {
        self.positions
            .iter()
            .rev()
            .find(|(id, _)| *id == token(prop))
            .unwrap()
            .1
    }
    fn move_roof_prop(&mut self, prop: &RefValue, p: [f32; 3], manager: u32, property: u32, arg: u32) {
        assert_eq!((manager, property, arg), (0, 0x99EF1146, 0));
        self.moved_props.push((token(prop), p));
    }
    fn automaton_position(&mut self, automaton: &RefValue) -> [f32; 3] {
        self.prop_position(automaton)
    }
    fn move_automaton(&mut self, automaton: &RefValue, p: [f32; 3]) {
        self.moved_workers.push((token(automaton), p));
    }
    fn translate_crane(&mut self, p: [f32; 3]) {
        self.positions.push((999, p));
    }
    fn pose_crane(&mut self, _: f32) {}
    fn create_crane(&mut self, r: CraneRequest) -> bool {
        self.crane_requests.push(r);
        !self.crane_fail
    }
    fn spark_slot8(&mut self, _: usize, _: u32) {}
    fn place_spark(&mut self, _: usize, _: [f32; 3]) {}
    fn start_scaffold_removal(&mut self) {
        self.events.push("removal".into());
    }
    fn add_to_renderer(&mut self) {
        self.events.push("render+".into());
    }
    fn remove_from_renderer(&mut self) {
        self.events.push("render-".into());
    }
    fn remove_from_occupant_manager(&mut self) {
        self.events.push("occupant-".into());
    }
    fn occupant_base_shutdown(&mut self) {
        self.events.push("base-".into());
    }
    fn release_texture_bindings(&mut self) {
        self.events.push("textures-".into());
    }
    fn free_model_buffers(&mut self) {
        self.events.push("buffers-".into());
    }
    fn release_crane_model(&mut self) {
        self.events.push("crane-".into());
    }
    fn remove_prop(&mut self, r: &RefValue) {
        self.events.push(format!("prop- {}", token(r)));
        self.props.retain(|&p| p != token(r));
    }
    fn remove_automaton(&mut self, r: &RefValue) {
        self.events.push(format!("worker- {}", token(r)));
        self.workers.retain(|&p| p != token(r));
    }
    fn prepare_render_state(&mut self, bindings: [[TextureBinding; 5]; 6], state: [f32; 14]) {
        assert_eq!(state, INITIAL_RENDER_STATE);
        self.bindings = Some(bindings);
        self.events.push("render state".into());
    }
    fn visibility_result(&mut self) -> bool {
        false
    }
    fn model_message_keys(&mut self) -> (u32, u32) {
        (1, 2)
    }
}

#[test]
fn test47_completion_removes_every_manager_entry_once() {
    let mut h = Host {
        detail: 3,
        ..Host::default()
    };
    let mut st = statics();
    st.ground.push(PropSize {
        type_id: GROUND_TYPES[0],
        x: 1.0,
        z: 1.0,
    });
    st.roof.push(PropSize {
        type_id: ROOF_TYPES[0],
        x: 1.0,
        z: 1.0,
    });
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, reference(1));
    assert!(!c.rec.ground_props.is_empty());
    assert!(!c.rec.roof_props.is_empty());
    assert!(!c.rec.crew.is_empty());
    assert!(!c.rec.current_effect.is_null());
    for day in 1..100 {
        h.day = day;
        c.do_daily_sim(&mut h, &mut st);
        if c.rec.scalars.state == STATE_DONE {
            break;
        }
    }
    assert_eq!(c.rec.scalars.state, STATE_DONE);
    assert!(h.props.is_empty() && h.workers.is_empty());
    assert!(!c.initialised && !c.rec.has_crane);
    let count = h.events.len();
    c.shutdown(&mut h);
    assert_eq!(h.events.len(), count);
    let at = |s: &str| h.events.iter().position(|e| e == s).unwrap();
    assert!(at("grown") < at("post C9E7B1E8 0"));
    assert!(at("post C9E7B1E8 0") < at("base-"));
    assert!(at("base-") < at("unsub 66956814"));
    assert!(at("textures-") < at("buffers-") && at("buffers-") < at("crane-"));
}

#[test]
fn world_bounds_and_visual_messages_and_uninitialised_shutdown() {
    let mut h = Host::default();
    let mut c = ConstructionOccupant::default();
    let mut st = statics();
    c.shutdown(&mut h);
    assert!(h.events.is_empty());
    c.set_lot(&mut h, &mut st, reference(1));
    assert_eq!(c.bounding_box(), ([8.0, 100.0, 6.0], [32.0, 120.0, 22.0]));
    assert_eq!(c.position(), [20.0, 110.0, 14.0]);
    c.visibility_test(&mut h, false);
    assert_eq!(c.rec.base.flags & 2, 0);
    let count = h.events.len();
    c.visibility_test(&mut h, false);
    assert_eq!(h.events.len(), count);
    c.visibility_test(&mut h, true);
    assert_eq!(c.rec.base.flags & 2, 2);
    c.model_available(1, 1);
    assert!(c.model_dirty); // RebuildModel already requested relighting.
    c.model_dirty = false;
    c.model_available(1, 1);
    assert!(!c.model_dirty);
    c.model_available(1, 2);
    assert!(c.model_dirty);
    assert!(c.meshes.posts.is_some());
}

#[test]
fn static_tables_filter_missing_profiles_and_reseed_plain_time() {
    struct Source;
    impl ConstructionSource for Source {
        fn property(&mut self, _: u32) -> Option<f32> {
            None
        }
        fn crane_bounds(&mut self, id: u32) -> Option<Bounds> {
            (id == CRANE_MODELS[1]).then_some(bounds(-2.0, 0.0, 8.0, 1.0))
        }
        fn prop_dimensions(&mut self, id: u32) -> Option<[f32; 2]> {
            (id == 0x0205_0000).then_some([2.0, 3.0])
        }
    }
    let mut st = ConstructionStatics::load(&mut Source, 12345, CrtRand::new(1));
    assert_eq!(
        st.cranes,
        [
            CraneSize::default(),
            CraneSize {
                width: 10.0,
                reach: 8.0,
                back: 2.0
            },
            CraneSize::default()
        ]
    );
    assert_eq!((st.ground.len(), st.roof.len()), (1, 1));
    assert_eq!(st.rng.next_raw(), statics().rng.next_raw());
}

#[test]
fn tests20_21_timing_and_queued_completion_day() {
    let mut st = statics();
    let g = rebuild_model(&([0.0, 100.0, 0.0], [10.0, 110.0, 10.0]), &st.tuning, |_, _| 100.0);
    assert_eq!(g.start_y.to_bits(), 0x42C8_3333);
    let t = timing(&g, &st.tuning, Some(30));
    assert_eq!((g.segments, t.a, t.b, t.c, t.e), (4, 0, 1, 1, 6));
    let mut c = ConstructionOccupant::default();
    let mut h = Host::default();
    c.init(&mut h);
    c.rec.lot = reference(1);
    c.rec.building = reference(2);
    c.rec.scalars.segments = 4;
    c.rec.scalars.b = 1;
    c.rec.scalars.c = 1;
    c.rec.scalars.e = 6;
    let mut starts = vec![];
    let mut removal = 0;
    let mut completed = 0;
    for day in 1..=30 {
        h.day = day;
        h.events.clear();
        c.do_message(&mut h, &mut st, MSG_SIM_NEW_DAY, u32::MAX);
        if h.events.iter().any(|e| e == "post 49E7B1E2 1") {
            starts.push(day);
        }
        if h.events.iter().any(|e| e == "removal") {
            removal = day;
        }
        if h.events.iter().any(|e| e == "grown") {
            completed = day;
        }
    }
    assert_eq!(starts, [1, 8, 15, 22]);
    assert_eq!((removal, completed), (29, 30));
}

#[test]
fn tests23_24_25_texture_sparks_and_crane_random_sequence() {
    let mut st = statics();
    assert_eq!(
        lot_texture(&mut st.rng, [1, 1, 2, 2], |_, _| true),
        [
            (1, 1, 0xA990_C100),
            (1, 2, 0xA990_C400),
            (2, 1, 0xA990_C200),
            (2, 2, 0xA990_C400)
        ]
    );
    let mut st = statics();
    let p = scatter_sparks(&mut st, &([0.0, 100.0, 0.0], [20.0, 110.0, 30.0]), 1);
    assert_eq!(p[0].map(f32::to_bits), [0x4138_7420, 0x42CC_C8DD, 0]);
    let mut crt = CrtRand::new(1);
    for _ in 0..3 {
        crt.rand();
    }
    assert_eq!(st.crt, crt);
    let mut st = statics();
    let mut expected = st.rng.clone();
    let mut s = ConstructionScalars {
        crane_state: 0,
        crane_duration: 0,
        ..ConstructionScalars::default()
    };
    let duration = 1000 + expected.uniform_u32(1000);
    animate_crane(&mut s, 0, &mut st.rng, 6000.0);
    assert_eq!((s.crane_state, s.crane_duration), (1, duration));
    animate_crane(&mut s, duration, &mut st.rng, 6000.0);
    assert_eq!(s.crane_state, 2);
    assert_eq!(st.rng.next_raw(), expected.next_raw());
}

#[test]
fn rejected_props_draw_nothing_and_failed_rounds_terminate() {
    let mut st = statics();
    let mut expected = st.rng.clone();
    let count = expected.range_i32(1, 4);
    expected.range_i32(0, 1);
    st.ground.push(PropSize {
        type_id: 1,
        x: 1.0,
        z: 1.0,
    });
    st.roof.push(PropSize {
        type_id: 2,
        x: 1.0,
        z: 1.0,
    });
    let mut h = Host {
        reject: true,
        ..Host::default()
    };
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, reference(1));
    assert!(c.rec.ground_props.is_empty() && c.rec.roof_props.is_empty());
    assert_eq!(st.rng.next_raw(), expected.next_raw());
    let mut crt = CrtRand::new(1);
    for _ in 0..(count * 7) {
        crt.rand();
    }
    assert_eq!(st.crt, crt);
    let mut st = statics();
    let mut expected = st.rng.clone();
    expected.range_i32(1, 4);
    expected.range_i32(0, 1);
    st.ground.push(PropSize {
        type_id: 1,
        x: 1000.0,
        z: 1000.0,
    });
    c.set_lot(&mut h, &mut st, reference(1));
    assert_eq!(st.rng.next_raw(), expected.next_raw());
    let mut crt = CrtRand::new(1);
    for _ in 0..7 {
        crt.rand();
    }
    assert_eq!(st.crt, crt);
}
