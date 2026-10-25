// SPDX-License-Identifier: GPL-3.0-or-later
//! WC3-L02–L04, WC3-R01, WC5-C01 enter result, inherited Shutdown and
//! presentation execution (watercraft §§3, 3.1, 3.5, 5.1, 5.1.4–5.1.7).
use super::tests::{boat, Water};
use super::*;
use crate::automata::{State, Vec3};
use crate::model_reg_effects::{self as fx, EffectFactory, ModelInstance, VisualEffect};
use crate::stream::{ByteReader, ByteWriter, OutStream, RefValue};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Base {
    add_refs: Vec<BoatId>,
    queries: Vec<(BoatId, u32)>,
}
impl AutomatonInterfaces for Base {
    fn add_ref(&mut self, object: BoatId) -> u32 {
        self.add_refs.push(object);
        self.add_refs.len() as u32
    }
    fn query_interface(&mut self, object: BoatId, iid: u32, out: &mut Option<InterfacePointer>) -> bool {
        self.queries.push((object, iid));
        *out = None;
        false
    }
}

#[test]
fn wc3_l02_query_interface_through_every_adjustor_returns_complete_owner() {
    let mut f = Fleet::default();
    f.insert(7, boat()).unwrap();
    for offset in QUERY_INTERFACE_OFFSETS {
        let mut base = Base::default();
        let mut out = None;
        let pointer = InterfacePointer { object: 7, offset };
        assert_eq!(f.query_interface(pointer, CLASS_ID, &mut out, &mut base), Ok(true));
        assert_eq!(out, Some(InterfacePointer::complete(7)));
        assert_eq!(base.add_refs, [7]);
        assert!(base.queries.is_empty());
        // Other IDs delegate unchanged to the automaton QueryInterface.
        let mut other = Some(InterfacePointer::complete(1));
        assert_eq!(f.query_interface(pointer, 0x1234, &mut other, &mut base), Ok(false));
        assert_eq!(other, None);
        assert_eq!(base.queries, [(7, 0x1234)]);
        assert_eq!(base.add_refs.len(), 1);
    }
    let bad = InterfacePointer { object: 7, offset: 16 };
    assert_eq!(
        f.query_interface(bad, CLASS_ID, &mut None, &mut Base::default()),
        Err(ContractGap::UnknownInterface)
    );
    assert_eq!(f.get(7).unwrap().gzclsid(), 0x096E_75F5);
}

/// Fails only the first u16 write without making the stream sticky.
struct FailVersion {
    inner: ByteWriter,
    failed: bool,
    attempted: Vec<u16>,
}
impl OutStream for FailVersion {
    fn put_u8(&mut self, v: u8) -> bool {
        self.inner.put_u8(v)
    }
    fn put_u16(&mut self, v: u16) -> bool {
        if !self.failed {
            self.failed = true;
            self.attempted.push(v);
            return false;
        }
        self.inner.put_u16(v)
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.inner.put_u32(v)
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.inner.put_u64(v)
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.inner.put_f32(v)
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.inner.put_f64(v)
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.inner.put_bytes(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        self.inner.put_str(v)
    }
    fn put_ref(&mut self, r: &RefValue) -> bool {
        self.inner.put_ref(r)
    }
    fn error(&self) -> i32 {
        self.inner.error()
    }
}

#[test]
fn wc3_l03_write_failure_of_version_still_writes_base_and_returns_base_result() {
    let mut f = Fleet::default();
    f.insert(1, boat()).unwrap();
    let mut reference = ByteWriter::new();
    let base_result = f.get(1).unwrap().record.base.write(&mut reference);
    let mut stream = FailVersion {
        inner: ByteWriter::new(),
        failed: false,
        attempted: Vec::new(),
    };
    let pointer = InterfacePointer {
        object: 1,
        offset: LIFECYCLE_INTERFACE,
    };
    assert_eq!(f.write_through(pointer, &mut stream), Ok(base_result));
    assert_eq!(stream.attempted, [RECORD_VERSION]);
    assert_eq!(stream.inner.bytes(), reference.bytes());
}

#[test]
fn wc3_r01_read_accepts_only_version_two_and_propagates_base_result() {
    let mut source = boat();
    source.record.base.states.push(State::new(state::IDLE));
    let mut writer = ByteWriter::new();
    assert!(source.write_record(&mut writer));
    let bytes = writer.bytes().to_vec();
    let mut f = Fleet::default();
    f.insert(1, boat()).unwrap();
    let pointer = InterfacePointer {
        object: 1,
        offset: LIFECYCLE_INTERFACE,
    };
    for version in [1u16, 3] {
        let mut wrong = bytes.clone();
        wrong[..2].copy_from_slice(&version.to_le_bytes());
        assert_eq!(f.read_through(pointer, &mut ByteReader::new(&wrong)), Ok(false));
        assert!(f.get(1).unwrap().record.base.states.is_empty());
    }
    // Version 2 with a truncated base: base failure propagates.
    assert_eq!(f.read_through(pointer, &mut ByteReader::new(&bytes[..6])), Ok(false));
    assert_eq!(f.read_through(pointer, &mut ByteReader::new(&bytes)), Ok(true));
    assert_eq!(f.get(1).unwrap().record.base.states, source.record.base.states);
    assert_eq!(
        f.read_through(InterfacePointer { object: 1, offset: 8 }, &mut ByteReader::new(&bytes)),
        Err(ContractGap::UnknownInterface)
    );
}

fn station(id: u64) -> Arc<Station> {
    Arc::new(Station {
        lot: Arc::new(Resource(id)),
        occupant: Arc::new(Resource(id + 100)),
        lot_position: [0.0; 3],
        position: [0.0; 3],
        direction: [1.0, 0.0, 0.0],
        route: Arc::from(vec![Cell { x: 1, z: 1 }]),
    })
}

#[test]
fn wc3_l04_complete_destruction_releases_in_reverse_effect_then_route_tow_station_order() {
    let mut b = boat();
    let effects: Vec<Arc<Resource>> = (0..6).map(|i| Arc::new(Resource(i))).collect();
    b.effects = std::array::from_fn(|i| Some(effects[i].clone()));
    let neighbor = Arc::new(Resource(50));
    b.neighbor = Some(NeighborRoute {
        occupant: Some(neighbor.clone()),
        position: [0.0; 3],
        cells: Arc::from(vec![Cell { x: 0, z: 0 }]),
        next: 0,
    });
    b.tug = Some(2);
    b.towed = Some(3);
    let (a, bb, current) = (station(1), station(2), station(3));
    b.stations = [Some(a.clone()), Some(bb.clone())];
    b.current_station = Some(current.clone());
    b.held_berth = true;
    b.dock_requests = [true, true];
    b.initialized = true;
    let mut f = Fleet::default();
    f.insert(1, b).unwrap();
    let (result, log) = f
        .delete(
            InterfacePointer {
                object: 1,
                offset: DELETING_INTERFACE,
            },
            0,
        )
        .unwrap();
    assert_eq!(result, InterfacePointer::complete(1));
    let mut expected: Vec<Release> = (0..6).rev().map(Release::Effect).collect();
    expected.extend([
        Release::NeighborRouteStorage,
        Release::NeighborOccupant,
        Release::TravelStorage,
        Release::Tug,
        Release::Towed,
        Release::RouteStorage(None),
        Release::Station(None),
        Release::RouteStorage(Some(1)),
        Release::Station(Some(1)),
        Release::RouteStorage(Some(0)),
        Release::Station(Some(0)),
        Release::AutomatonBase,
    ]);
    assert_eq!(log, expected);
    for effect in &effects {
        assert_eq!(Arc::strong_count(effect), 1);
    }
    assert_eq!(Arc::strong_count(&neighbor), 1);
    assert_eq!(Arc::strong_count(&current), 1);
    assert_eq!(Arc::strong_count(&a), 1);
    // No Shutdown: no stop, berth return, unregister or base shutdown events.
    let boat = f.get(1).expect("flags without bit 0 keep the storage");
    assert!(boat.events.is_empty());
    assert_eq!(boat.dock_requests, [true, true]);
    assert!(boat.held_berth);
}

#[test]
fn deleting_destructor_skips_null_references_and_frees_only_with_low_flag_bit() {
    let mut f = Fleet::default();
    f.insert(1, boat()).unwrap();
    f.insert(2, boat()).unwrap();
    let (result, log) = f.delete(InterfacePointer::complete(1), 1).unwrap();
    assert_eq!(result, InterfacePointer::complete(1));
    assert_eq!(log, [Release::TravelStorage, Release::AutomatonBase]);
    assert!(f.get(1).is_none());
    assert_eq!(f.order, [2]);
    assert_eq!(
        f.delete(InterfacePointer { object: 2, offset: 4 }, 1),
        Err(ContractGap::UnknownInterface)
    );
    assert_eq!(
        f.delete(InterfacePointer::complete(9), 1),
        Err(ContractGap::UnknownBoat)
    );
}

#[test]
fn wc5_c01_enter_returns_base_result_without_suppressing_owner_entry_effects() {
    for kind in [state::RUN_AGROUND, state::SINK] {
        for base in [false, true] {
            let mut f = Fleet::default();
            let mut b = boat();
            b.crash_elapsed = 55.0;
            f.insert(1, b).unwrap();
            let mut entered = Vec::new();
            let mut next = State::new(kind);
            next.base.countdown_ms = 8000.0;
            let result = f
                .push_state_entered(1, next, &mut None, &mut |boat, state| {
                    // Base entry runs first, with the state already on the stack.
                    assert_eq!(boat.top_state(), Some(state.state_type));
                    assert!(boat.events.is_empty());
                    entered.push(state.state_type);
                    base
                })
                .unwrap();
            assert_eq!(result, base);
            assert_eq!(entered, [kind]);
            let b = f.get(1).unwrap();
            assert_eq!(b.events, [Event::Notify(0xB, 0)]);
            assert_eq!(b.crash_elapsed, 0.0);
            assert_eq!(b.record.base.automata_flags & 0x1800_0000, 0x1800_0000);
        }
    }
}

#[derive(Debug, PartialEq)]
enum Step {
    Slot69,
    Initialized,
    Clean,
    ReleaseModel(u64),
    B0,
    Registrations,
    Slot127,
    Slot98,
    EffectList,
    Zero98,
    Sound(u32, u32),
    Occupant,
}
struct Shutdowns {
    initialized: bool,
    sound: u32,
    steps: Arc<Mutex<Vec<Step>>>,
}
impl ShutdownServices for Shutdowns {
    fn initialized(&mut self) -> bool {
        self.steps.lock().unwrap().push(Step::Initialized);
        self.initialized
    }
    fn slot69(&mut self) {
        self.steps.lock().unwrap().push(Step::Slot69);
    }
    fn clean_current_state(&mut self) {
        self.steps.lock().unwrap().push(Step::Clean);
    }
    fn release_b0(&mut self) {
        self.steps.lock().unwrap().push(Step::B0);
    }
    fn disable_registrations(&mut self) {
        self.steps.lock().unwrap().push(Step::Registrations);
    }
    fn slot127(&mut self) {
        self.steps.lock().unwrap().push(Step::Slot127);
    }
    fn slot98(&mut self) {
        self.steps.lock().unwrap().push(Step::Slot98);
    }
    fn release_effect_list(&mut self) {
        self.steps.lock().unwrap().push(Step::EffectList);
    }
    fn release_98_and_zero(&mut self) {
        self.steps.lock().unwrap().push(Step::Zero98);
    }
    fn take_sound_identifier(&mut self) -> u32 {
        std::mem::take(&mut self.sound)
    }
    fn sound_event(&mut self, event: u32, identifier: u32) {
        self.steps.lock().unwrap().push(Step::Sound(event, identifier));
    }
    fn occupant_shutdown(&mut self) -> bool {
        self.steps.lock().unwrap().push(Step::Occupant);
        false
    }
}

/// Only model reference releases are observed; every other slot is unused.
struct Releases(Arc<Mutex<Vec<Step>>>);
impl ModelServices for Releases {
    fn sprite_lookup(&mut self, _: crate::stream::ResKey, _: u32, _: [u32; 2], _: &mut Option<u64>) -> bool {
        unreachable!()
    }
    fn sprite_create(&mut self, _: u64, _: u32, _: &mut Option<ModelHandle>) -> bool {
        unreachable!()
    }
    fn mesh_construct(&mut self, _: u32) -> ModelHandle {
        unreachable!()
    }
    fn mesh_load(&mut self, _: ModelHandle, _: crate::stream::ResKey) -> bool {
        unreachable!()
    }
    fn add_ref(&mut self, _: u64) {
        unreachable!()
    }
    fn release(&mut self, reference: u64) {
        self.0.lock().unwrap().push(Step::ReleaseModel(reference));
    }
    fn instance(&mut self, model: ModelHandle) -> u64 {
        model + 1000
    }
    fn bounds(&mut self, _: u64) -> [Vec3; 2] {
        [[1.0; 3], [2.0; 3]]
    }
    fn read_transform(&mut self, _: u64, _: &mut Transform) {
        unreachable!()
    }
    fn write_transform(&mut self, _: u64, _: &Transform) {
        unreachable!()
    }
    fn associate(&mut self, _: u64) {
        unreachable!()
    }
    fn instance_mode(&mut self, _: u64, _: u32) {
        unreachable!()
    }
    fn primary_mode(&mut self, _: ModelHandle, _: u32, _: u32) {
        unreachable!()
    }
    fn update_view(&mut self, _: ModelHandle) {
        unreachable!()
    }
    fn auxiliary_selectors(&mut self, _: u32) -> Option<Vec<u32>> {
        unreachable!()
    }
    fn property_key(&mut self, _: u32) -> Option<crate::stream::ResKey> {
        unreachable!()
    }
    fn visibility(&mut self, _: u32) -> bool {
        false
    }
    fn render_full(&mut self, _: u64) {
        unreachable!()
    }
    fn render_partial(&mut self, _: u64, _: u32) {
        unreachable!()
    }
    fn occupant_update(&mut self, _: u32, _: &mut u8) {
        unreachable!()
    }
    fn owner70(&mut self, _: bool, _: &mut u8) {
        unreachable!()
    }
    fn owner71(&mut self, _: bool, _: &mut u8) {
        unreachable!()
    }
    fn owner83(&mut self, _: f32) {
        unreachable!()
    }
    fn owner106(&mut self) {
        unreachable!()
    }
}

struct Instance;
impl ModelInstance for Instance {
    fn regions(&self) -> Option<&fx::Regions> {
        None
    }
    fn frame(&self) -> fx::InstanceFrame {
        fx::InstanceFrame {
            transform: fx::Transform::IDENTITY,
            flags: 0,
            animated_frame: None,
        }
    }
}

#[derive(Debug, PartialEq)]
enum Fx {
    Transform(String),
    Motion(String, [f32; 3]),
    Start(String),
    Stop(String),
}
struct Effect {
    name: String,
    started: bool,
    calls: Arc<Mutex<Vec<Fx>>>,
}
impl VisualEffect for Effect {
    fn is_started(&self) -> bool {
        self.started
    }
    fn start(&mut self, _: u32) {
        self.started = true;
        self.calls.lock().unwrap().push(Fx::Start(self.name.clone()));
    }
    fn stop(&mut self, _: u32) {
        self.started = false;
        self.calls.lock().unwrap().push(Fx::Stop(self.name.clone()));
    }
    fn set_transform(&mut self, _: fx::Transform, _: u8, _: u8) {
        self.calls.lock().unwrap().push(Fx::Transform(self.name.clone()));
    }
    fn set_parameter(&mut self, _: u32, value: [f32; 3], _: u32) {
        self.calls.lock().unwrap().push(Fx::Motion(self.name.clone(), value));
    }
}
#[derive(Default)]
struct Factory(Arc<Mutex<Vec<Fx>>>);
impl EffectFactory for Factory {
    fn acquire_manager(&mut self) -> bool {
        true
    }
    fn create(&mut self, name: &str) -> fx::Creation {
        fx::Creation {
            success: true,
            effect: Some(Box::new(Effect {
                name: name.into(),
                started: true,
                calls: self.0.clone(),
            })),
        }
    }
}

#[test]
fn uninitialized_complete_shutdown_runs_only_slot69_and_controller_shutdown() {
    let mut f = Fleet::default();
    let mut b = boat();
    b.towed = Some(2);
    b.effects[0] = Some(Arc::new(Resource(1)));
    b.models.entries.push(Some(5));
    f.insert(1, b).unwrap();
    let steps = Arc::new(Mutex::new(Vec::new()));
    let mut controller = fx::ModelRegEffects::new();
    controller.set_model_instance(Some(Arc::new(Instance)));
    let mut lifecycle = Shutdowns {
        initialized: false,
        sound: 9,
        steps: steps.clone(),
    };
    let mut models = Releases(steps.clone());
    let mut services = ShutdownStack {
        models: &mut models,
        lifecycle: &mut lifecycle,
        controller: &mut controller,
    };
    assert_eq!(f.shutdown_complete(1, &mut None, &mut services), Ok(true));
    assert!(controller.instance().is_none());
    assert_eq!(
        *steps.lock().unwrap(),
        [Step::Initialized, Step::Slot69, Step::Initialized]
    );
    let b = f.get(1).unwrap();
    // Owner cleanup is conditional: tow, effects and models survive.
    assert_eq!(b.towed, Some(2));
    assert!(b.effects[0].is_some());
    assert_eq!(b.models.entries.len(), 1);
    assert_eq!(b.events, [Event::BaseShutdown]);
}

#[test]
fn initialized_complete_shutdown_runs_owner_cleanup_then_inherited_order() {
    for sound in [0, 0x55] {
        let mut f = Fleet::default();
        let mut b = boat();
        b.effects[2] = Some(Arc::new(Resource(1)));
        b.models.entries = vec![Some(5), None, Some(6)];
        f.insert(1, b).unwrap();
        let steps = Arc::new(Mutex::new(Vec::new()));
        let mut controller = fx::ModelRegEffects::new();
        let mut lifecycle = Shutdowns {
            initialized: true,
            sound,
            steps: steps.clone(),
        };
        let mut models = Releases(steps.clone());
        let mut services = ShutdownStack {
            models: &mut models,
            lifecycle: &mut lifecycle,
            controller: &mut controller,
        };
        // Occupant Shutdown reporting false does not change the result.
        assert_eq!(f.shutdown_complete(1, &mut None, &mut services), Ok(true));
        let mut expected = vec![
            Step::Initialized,
            Step::Slot69,
            Step::Initialized,
            Step::Clean,
            Step::ReleaseModel(5),
            Step::ReleaseModel(6),
            Step::B0,
            Step::Registrations,
            Step::Slot127,
            Step::Slot98,
            Step::EffectList,
            Step::Zero98,
        ];
        if sound != 0 {
            expected.push(Step::Sound(SHUTDOWN_SOUND_EVENT, sound));
        }
        expected.push(Step::Occupant);
        assert_eq!(*steps.lock().unwrap(), expected);
        let b = f.get(1).unwrap();
        assert!(b.models.entries.is_empty());
        assert!(b.effects.iter().all(Option::is_none));
        assert_eq!(
            b.events,
            [
                Event::Unregister(TOW_MESSAGE),
                Event::StopEffect(Arc::new(Resource(1))),
                Event::BaseShutdown
            ]
        );
    }
}

#[test]
fn wc4_i04_presented_wake_stop_executes_six_stops_each_followed_by_slot_104() {
    let mut controller = fx::ModelRegEffects::new();
    controller.set_model_instance(Some(Arc::new(Instance)));
    let mut factory = Factory::default();
    controller.add_effect("A", "unrelated", None, &mut factory);
    let mut b = boat();
    b.inherited_heading = [1.0, 0.0, 0.0];
    b.inherited_measured_speed = 2.0;
    b.still_elapsed = 300.0;
    b.update_wakes(1.0, 16.0);
    let mut subsidiary = Subsidiary::default();
    let mut animation = AnimationServices {
        subsidiary: &mut subsidiary,
        models: &mut [],
        service: &mut NoAnimation,
        rope: None,
        lighting: None,
        processed_model: false,
        heading_threshold: [0.0; 3],
        distance_squared: 0.0,
    };
    let calls = factory.0.clone();
    let mut services = PresentationServices {
        effects: Some((&mut controller, &mut factory)),
        models: None,
    };
    b.execute_presentation(0, &mut animation, &mut services);
    let mut expected = Vec::new();
    for _ in 0..6 {
        expected.push(Fx::Transform("unrelated".into()));
        expected.push(Fx::Motion("unrelated".into(), [2.0, 0.0, 0.0]));
    }
    assert_eq!(*calls.lock().unwrap(), expected);
    assert_eq!(b.still_elapsed, 301.0);
}

struct NoAnimation;
impl AnimationService for NoAnimation {
    fn slot17(&mut self) -> bool {
        true
    }
    fn slot15(&mut self) -> bool {
        unreachable!()
    }
}

#[derive(Default)]
struct Lights(Vec<bool>);
impl Lighting for Lights {
    fn light_model(&mut self, control: bool) {
        self.0.push(control);
    }
}

#[test]
fn presentation_uses_inherited_motion_captured_at_each_request() {
    let mut controller = fx::ModelRegEffects::new();
    controller.set_model_instance(Some(Arc::new(Instance)));
    let mut factory = Factory::default();
    controller.add_effect("A", "wake", None, &mut factory);
    let mut b = boat();
    b.inherited_heading = [1.0, 0.0, 0.0];
    b.inherited_measured_speed = 1.0;
    b.inherited_position = [0.0; 3];
    b.request_effect_update();
    b.request_model_update();
    b.inherited_heading = [0.0, 0.0, 1.0];
    b.inherited_measured_speed = 3.0;
    b.inherited_position = [10.0, 0.0, 0.0];
    b.request_effect_update();
    b.request_model_update();
    let mut subsidiary = Subsidiary {
        last_lit_heading: [1.0, 0.0, 0.0],
        ..Subsidiary::default()
    };
    let mut lights = Lights::default();
    {
        let mut animation = AnimationServices {
            subsidiary: &mut subsidiary,
            models: &mut [],
            service: &mut NoAnimation,
            rope: None,
            lighting: Some(&mut lights),
            processed_model: true,
            heading_threshold: [0.5; 3],
            distance_squared: 4.0,
        };
        let mut services = PresentationServices {
            effects: Some((&mut controller, &mut factory)),
            models: None,
        };
        b.execute_presentation(0, &mut animation, &mut services);
    }
    assert_eq!(
        *factory.0.lock().unwrap(),
        [
            Fx::Transform("wake".into()),
            Fx::Motion("wake".into(), [1.0, 0.0, 0.0]),
            Fx::Transform("wake".into()),
            Fx::Motion("wake".into(), [0.0, 0.0, 3.0]),
        ]
    );
    // First slot 107 sees the unchanged cache; the second relights once.
    assert_eq!(lights.0, [false]);
    assert_eq!(subsidiary.last_lit_position, [10.0, 0.0, 0.0]);
    assert_eq!(subsidiary.last_lit_heading, [0.0, 0.0, 1.0]);
    assert!(b.presentation_snapshots.is_empty());
}

#[derive(Default)]
struct Publishing(Vec<String>);
impl ModelServices for Publishing {
    fn sprite_lookup(&mut self, _: crate::stream::ResKey, _: u32, _: [u32; 2], _: &mut Option<u64>) -> bool {
        unreachable!()
    }
    fn sprite_create(&mut self, _: u64, _: u32, _: &mut Option<ModelHandle>) -> bool {
        unreachable!()
    }
    fn mesh_construct(&mut self, _: u32) -> ModelHandle {
        unreachable!()
    }
    fn mesh_load(&mut self, _: ModelHandle, _: crate::stream::ResKey) -> bool {
        unreachable!()
    }
    fn add_ref(&mut self, _: u64) {
        unreachable!()
    }
    fn release(&mut self, _: u64) {
        unreachable!()
    }
    fn instance(&mut self, model: ModelHandle) -> u64 {
        model + 1000
    }
    fn bounds(&mut self, instance: u64) -> [Vec3; 2] {
        self.0.push(format!("bounds {instance}"));
        [[1.0; 3], [2.0; 3]]
    }
    fn read_transform(&mut self, _: u64, _: &mut Transform) {
        unreachable!()
    }
    fn write_transform(&mut self, _: u64, _: &Transform) {
        unreachable!()
    }
    fn associate(&mut self, _: u64) {
        unreachable!()
    }
    fn instance_mode(&mut self, _: u64, _: u32) {
        unreachable!()
    }
    fn primary_mode(&mut self, _: ModelHandle, _: u32, _: u32) {
        unreachable!()
    }
    fn update_view(&mut self, _: ModelHandle) {
        unreachable!()
    }
    fn auxiliary_selectors(&mut self, _: u32) -> Option<Vec<u32>> {
        unreachable!()
    }
    fn property_key(&mut self, _: u32) -> Option<crate::stream::ResKey> {
        unreachable!()
    }
    fn visibility(&mut self, flags: u32) -> bool {
        self.0.push(format!("visibility {flags}"));
        true
    }
    fn render_full(&mut self, instance: u64) {
        self.0.push(format!("full {instance}"));
    }
    fn render_partial(&mut self, _: u64, _: u32) {
        unreachable!()
    }
    fn occupant_update(&mut self, _: u32, _: &mut u8) {
        unreachable!()
    }
    fn owner70(&mut self, _: bool, _: &mut u8) {
        unreachable!()
    }
    fn owner71(&mut self, _: bool, _: &mut u8) {
        unreachable!()
    }
    fn owner83(&mut self, _: f32) {
        unreachable!()
    }
    fn owner106(&mut self) {
        unreachable!()
    }
}

#[test]
fn presented_movement_publication_executes_slot_100_then_104_then_107() {
    let mut controller = fx::ModelRegEffects::new();
    controller.set_model_instance(Some(Arc::new(Instance)));
    let mut factory = Factory::default();
    controller.add_effect("A", "wake", None, &mut factory);
    let mut b = boat();
    b.record.base.occupant.flags = 2;
    b.models.entries = vec![Some(1)];
    b.models.event_counter = 150.0;
    b.speed = 1.0;
    b.position = [5.0, 0.0, 0.0];
    b.movement_position_new(10.0, &Water);
    let mut subsidiary = Subsidiary::default();
    let mut lights = Lights::default();
    let mut publishing = Publishing::default();
    {
        let mut animation = AnimationServices {
            subsidiary: &mut subsidiary,
            models: &mut [],
            service: &mut NoAnimation,
            rope: None,
            lighting: Some(&mut lights),
            processed_model: true,
            heading_threshold: [10.0; 3],
            distance_squared: 1.0,
        };
        let mut services = PresentationServices {
            effects: Some((&mut controller, &mut factory)),
            models: Some(&mut publishing),
        };
        b.execute_presentation(0, &mut animation, &mut services);
    }
    assert_eq!(publishing.0, ["bounds 1001", "visibility 4", "full 1001"]);
    assert_eq!(b.occupant_bounds, [[1.0; 3], [2.0; 3]]);
    assert_eq!(b.models.event_counter, 0.0);
    assert_eq!(b.events.last(), Some(&Event::Notify(9, 0)));
    let calls = factory.0.lock().unwrap();
    assert_eq!(calls[0], Fx::Transform("wake".into()));
    assert_eq!(lights.0, [false]);
    assert_eq!(subsidiary.last_lit_position, b.inherited_position);
}
