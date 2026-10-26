// SPDX-License-Identifier: GPL-3.0-or-later
//! Worked cases from sound-system.md §§7–9 (SS8–SS12).
use super::directors::*;
use super::framework::*;
use super::playlist::*;
use super::*;
use crate::audio::snd_event::SndEventStd;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

type Trace = Arc<Mutex<Vec<String>>>;
fn note(trace: &Trace, text: impl Into<String>) {
    trace.lock().unwrap().push(text.into());
}
struct DropNote(Trace, &'static str);
impl Drop for DropNote {
    fn drop(&mut self) {
        note(&self.0, self.1);
    }
}

struct CountTarget(Arc<AtomicUsize>, bool);
impl UpdateTarget for CountTarget {
    fn update(&self, _: &mut SndSys) -> bool {
        self.0.fetch_add(1, Ordering::SeqCst);
        self.1
    }
}
struct QueueTarget {
    target: TargetRef,
    once: AtomicBool,
}
impl UpdateTarget for QueueTarget {
    fn update(&self, system: &mut SndSys) -> bool {
        if !self.once.swap(true, Ordering::SeqCst) {
            system.register_target(self.target.clone(), 250).unwrap();
            system.register_target(self.target.clone(), 20).unwrap();
            system.unregister_target(self.target.clone());
        }
        false
    }
}
#[test]
fn ss8_period_boundary_and_ss12_category_replay() {
    let counts = [
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicUsize::new(0)),
    ];
    let mut system = SndSys::default();
    for (count, period) in counts.iter().zip([20, 19, 250]) {
        system
            .register_target(Arc::new(CountTarget(count.clone(), true)), period)
            .unwrap();
    }
    system.update_targets(250);
    assert_eq!(counts.each_ref().map(|count| count.load(Ordering::SeqCst)), [1, 0, 0]);
    system.update_targets(251);
    assert_eq!(counts.each_ref().map(|count| count.load(Ordering::SeqCst)), [2, 1, 1]);
    system.update_targets(501);
    assert_eq!(counts[1].load(Ordering::SeqCst), 1);
    system.update_targets(502);
    assert_eq!(counts[1].load(Ordering::SeqCst), 2);

    let count = Arc::new(AtomicUsize::new(0));
    let target: TargetRef = Arc::new(CountTarget(count.clone(), true));
    let mut system = SndSys::default();
    system
        .register_target(
            Arc::new(QueueTarget {
                target: target.clone(),
                once: AtomicBool::new(false),
            }),
            20,
        )
        .unwrap();
    system.update_targets(251); // false ordinary callback still checks slow list; queued adds wait until exit.
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(Arc::strong_count(&target), 3); // test, queueing owner, one slow registration
    system.update_targets(501);
    assert_eq!(count.load(Ordering::SeqCst), 0);
    system.update_targets(502);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    system.disable_updates();
    system.update_targets(1000);
    assert_eq!(count.load(Ordering::SeqCst), 1);
}
struct OwnedTarget(Trace, &'static str);
impl UpdateTarget for OwnedTarget {
    fn update(&self, _: &mut SndSys) -> bool {
        true
    }
}
impl Drop for OwnedTarget {
    fn drop(&mut self) {
        note(&self.0, self.1);
    }
}
struct QueueOwnedTargets(Trace);
impl UpdateTarget for QueueOwnedTargets {
    fn update(&self, system: &mut SndSys) -> bool {
        system.unregister_target(Arc::new(OwnedTarget(self.0.clone(), "remove1")));
        system
            .register_target(Arc::new(OwnedTarget(self.0.clone(), "ordinary")), 20)
            .unwrap();
        system.unregister_target(Arc::new(OwnedTarget(self.0.clone(), "remove2")));
        system
            .register_target(Arc::new(OwnedTarget(self.0.clone(), "slow")), 250)
            .unwrap();
        assert!(self.0.lock().unwrap().is_empty());
        note(&self.0, "callback");
        false
    }
}
#[test]
fn ss8_ss12_queues_retain_until_replay_and_shutdown_releases_active_categories() {
    let trace = Trace::default();
    let mut system = SndSys::default();
    system
        .register_target(Arc::new(QueueOwnedTargets(trace.clone())), 20)
        .unwrap();
    system.update_targets(251);
    assert_eq!(*trace.lock().unwrap(), ["callback", "remove1", "remove2"]);
    let mut host = SystemFixture {
        trace: Trace::default(),
        start: false,
        caches: false,
    };
    assert!(system.shutdown(&mut host));
    assert_eq!(
        *trace.lock().unwrap(),
        ["callback", "remove1", "remove2", "ordinary", "slow"]
    );
}

struct NamedHandler(&'static str, Trace);
impl EventHandler for NamedHandler {
    fn do_event(&self, _: &mut SndSys, _: &SndEventStd) -> bool {
        note(&self.1, self.0);
        false
    }
}
struct QueueHandler {
    x: HandlerRef,
    y: HandlerRef,
    once: AtomicBool,
}
impl EventHandler for QueueHandler {
    fn do_event(&self, system: &mut SndSys, _: &SndEventStd) -> bool {
        if !self.once.swap(true, Ordering::SeqCst) {
            system.register_handler(self.x.clone());
            system.register_handler(self.y.clone());
            system.unregister_handler(self.x.clone());
        }
        false
    }
}
#[test]
fn ss12_handler_replacement_and_deferred_front_order() {
    let trace = Trace::default();
    let a: HandlerRef = Arc::new(NamedHandler("A", trace.clone()));
    let b: HandlerRef = Arc::new(NamedHandler("B", trace.clone()));
    let x: HandlerRef = Arc::new(NamedHandler("X", trace.clone()));
    let y: HandlerRef = Arc::new(NamedHandler("Y", trace.clone()));
    let mut system = SndSys::default();
    for handler in [a.clone(), b, a.clone()] {
        system.register_handler(handler);
    }
    system.do_event(&SndEventStd::new());
    assert_eq!(*trace.lock().unwrap(), ["A", "B"]);
    assert_eq!(Arc::strong_count(&a), 2);
    system.register_handler(Arc::new(QueueHandler {
        x: x.clone(),
        y,
        once: AtomicBool::new(false),
    }));
    trace.lock().unwrap().clear();
    system.do_event(&SndEventStd::new());
    trace.lock().unwrap().clear();
    system.do_event(&SndEventStd::new());
    assert_eq!(*trace.lock().unwrap(), ["Y", "A", "B"]);
    assert_eq!(Arc::strong_count(&x), 2); // test and queueing handler only
}
struct SystemFixture {
    trace: Trace,
    start: bool,
    caches: bool,
}
impl SystemHost for SystemFixture {
    fn data_directory(&mut self) -> Option<String> {
        Some("Audio".into())
    }
    fn message_handler(&mut self, message: u32, register: bool) {
        assert_eq!(message, 0xDB53383B);
        note(&self.trace, format!("message:{register}"));
    }
    fn create_queue(&mut self) -> Option<ForeignRef> {
        note(&self.trace, "queue");
        Some(Arc::new(DropNote(self.trace.clone(), "drop:queue")))
    }
    fn create_cache(&mut self, class: u32, iid: u32) -> Option<ForeignRef> {
        let name = match (class, iid) {
            (0x9D4C6B95, 0x5D4C6B90) => "property",
            (0xBDF613B4, 0x3DF613A1) => "instance",
            _ => panic!("cache identity"),
        };
        note(&self.trace, name);
        self.caches.then(|| {
            Arc::new(DropNote(
                self.trace.clone(),
                if name == "property" {
                    "drop:property"
                } else {
                    "drop:instance"
                },
            )) as ForeignRef
        })
    }
    fn timer(&mut self, operation: TimerOperation) -> bool {
        note(&self.trace, format!("{operation:?}"));
        self.start
    }
}
struct Stoppable(Trace);
impl ShutdownSound for Stoppable {
    fn stop(&self) {
        note(&self.0, "stop");
    }
}
struct StopTarget(Trace);
impl UpdateTarget for StopTarget {
    fn update(&self, _: &mut SndSys) -> bool {
        true
    }
    fn query_shutdown_sound(&self) -> Option<Arc<dyn ShutdownSound>> {
        Some(Arc::new(Stoppable(self.0.clone())))
    }
}
#[test]
fn ss8_disabled_failed_and_running_init_cache_ownership_shutdown() {
    let trace = Trace::default();
    let mut host = SystemFixture {
        trace: trace.clone(),
        start: false,
        caches: false,
    };
    let mut system = SndSys::default();
    system.enabled_flag = Some(false);
    assert!(system.init(&mut host, 0));
    assert_eq!(*trace.lock().unwrap(), ["message:true"]);
    assert!(!system.running());
    let mut output: Option<ForeignRef> = Some(Arc::new(1u32));
    assert!(!system.acquire_property_cache(&mut output));
    assert!(output.is_none());
    system.enabled_flag = Some(true);
    trace.lock().unwrap().clear();
    assert!(system.init(&mut host, 100));
    assert!(!system.running());
    assert!(system.property_cache().is_none());
    assert_eq!(
        *trace.lock().unwrap(),
        [
            "message:true",
            "queue",
            "property",
            "instance",
            "StartElapsed",
            "StartSlow",
            "CreateCallback { period: 20 }",
            "InstallCallback",
            "StartCallback",
            "Priority(4)"
        ]
    );
    host.start = true;
    host.caches = true;
    assert!(system.init(&mut host, 200));
    assert!(system.running());
    let borrowed = system.property_cache().unwrap();
    assert_eq!(Arc::strong_count(borrowed), 1);
    assert!(system.acquire_property_cache(&mut output));
    assert_eq!(Arc::strong_count(output.as_ref().unwrap()), 2);
    drop(output.take());
    assert!(system.acquire_instance_cache(&mut output));
    drop(output.take());
    trace.lock().unwrap().clear();
    assert!(system.init(&mut host, 300));
    assert_eq!(*trace.lock().unwrap(), ["message:true"]);
    system.register_target(Arc::new(StopTarget(trace.clone())), 20).unwrap();
    system
        .register_target(Arc::new(StopTarget(trace.clone())), 250)
        .unwrap();
    trace.lock().unwrap().clear();
    assert!(system.shutdown(&mut host));
    assert!(!system.running());
    assert_eq!(
        *trace.lock().unwrap(),
        [
            "StopCallback",
            "StopElapsed",
            "message:false",
            "StopCallback",
            "DeleteCallback",
            "stop",
            "stop",
            "drop:property",
            "drop:instance",
            "drop:queue"
        ]
    );
}
#[derive(Clone)]
struct ListenerSpy(Trace);
impl Renderer for ListenerSpy {
    fn master_volume(&self, _: i32) {}
    fn listener(&self) -> Option<Arc<dyn Listener>> {
        Some(Arc::new(self.clone()))
    }
    fn listener_ex(&self) -> Option<Arc<dyn ListenerEx>> {
        Some(Arc::new(self.clone()))
    }
}
impl Listener for ListenerSpy {
    fn position(&self, value: [f32; 3]) {
        note(&self.0, format!("position:{:?}", value.map(f32::to_bits)));
    }
    fn orientation(&self, _: [f32; 6]) {
        note(&self.0, "orientation");
    }
}
impl ListenerEx for ListenerSpy {
    fn velocity(&self, _: [f32; 3]) {
        note(&self.0, "velocity");
    }
}
#[test]
fn ss8_extension_seed_rule_renderer_identity_and_listener_words() {
    let mut system = SndSys::default();
    assert!(system.set_extension(" .WAV ", 7));
    assert!(system.set_extension("wav", 9));
    assert_eq!(system.extension_count(), 1);
    let mut class = 123;
    assert!(system.extension_class(".WaV", &mut class));
    assert_eq!(class, 9);
    assert!(!system.extension_class("a.wav", &mut class));
    assert_eq!(class, 9);
    let mut text = String::new();
    assert!(!system.extension_text_at(0, &mut text));
    text = "wav".into();
    assert!(system.extension_text_at(0, &mut text));
    assert!(system.set_extension("a", 0));
    assert!(system.extension_class_at(0, &mut class));
    assert_eq!(class, 0);
    assert!(!system.extension_class_at(2, &mut class));
    assert_eq!(class, 0);
    assert!(!system.extension_text_at(1, &mut text));
    assert_eq!(text, "wav");
    assert!(!system.set_extension(" . ", 8));
    assert!(system.set_extension("..ogg", 5));
    assert!(system.extension_class("..ogg", &mut class));
    assert_eq!(class, 5);
    let trace = Trace::default();
    let renderer: RendererRef = Arc::new(ListenerSpy(trace.clone()));
    system.register_renderer(renderer.clone()).unwrap();
    system.register_renderer(renderer.clone()).unwrap();
    assert_eq!(system.num_renderers(), 1);
    assert_eq!(Arc::strong_count(&renderer), 2);
    let position = [1.5, -2.25, -0.0];
    system.set_listener([0.0; 3], [0.0; 6], position);
    assert_eq!(system.position().unwrap().map(f32::to_bits), position.map(f32::to_bits));
    assert_eq!(system.orientation(), Some([0.0; 6]));
    assert_eq!(system.velocity(), Some([0.0; 3]));
    assert_eq!(
        *trace.lock().unwrap(),
        [
            "velocity".to_owned(),
            "orientation".into(),
            format!("position:{:?}", position.map(f32::to_bits))
        ]
    );
}

struct RenderCount(Arc<AtomicUsize>);
impl ManagerSystem for RenderCount {
    fn num_renderers(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}
struct ManagerFixture(Arc<AtomicUsize>);
impl ManagerServices for ManagerFixture {
    fn acquire_sound_system(&mut self) -> bool {
        true
    }
    fn register_manager_update(&mut self, offset: u32, period: u32) {
        assert_eq!((offset, period), (0x20, 250));
    }
    fn retain_persist_manager(&mut self) {}
    fn sound_system_reference(&mut self) -> Option<Arc<dyn ManagerSystem>> {
        Some(Arc::new(RenderCount(self.0.clone())))
    }
}
#[derive(Clone)]
struct Object {
    name: &'static str,
    trace: Trace,
    query_ok: bool,
    play_ok: bool,
    playing: Arc<AtomicBool>,
}
impl ManagerObject for Object {
    fn query(&self, iid: u32, output: &mut Option<ObjectRef>) -> bool {
        note(&self.trace, format!("qi:{}:{iid}", self.name));
        if self.query_ok {
            *output = Some(Arc::new(self.clone()));
        }
        self.query_ok
    }
    fn play(&self, volume: i32, a: u32, b: u32) -> bool {
        note(&self.trace, format!("play:{}:{volume}:{a}:{b}", self.name));
        self.play_ok
    }
    fn is_playing(&self) -> bool {
        note(&self.trace, format!("playing:{}", self.name));
        self.playing.load(Ordering::SeqCst)
    }
}
fn object(name: &'static str, trace: &Trace, query_ok: bool, play_ok: bool) -> ObjectRef {
    Arc::new(Object {
        name,
        trace: trace.clone(),
        query_ok,
        play_ok,
        playing: Arc::new(AtomicBool::new(false)),
    })
}
struct Creator {
    name: u32,
    trace: Trace,
    success: bool,
    result: ObjectRef,
    register: bool,
}
impl ManagerCreator for Creator {
    fn supports(&self, iid: u32) -> bool {
        assert!(matches!(iid, 0x3AF9866F | 0x9B75709E));
        true
    }
    fn query(&self, iid: u32, output: &mut Option<ObjectRef>) -> bool {
        self.result.query(iid, output)
    }
    fn create(&self, manager: &mut SndMgr, args: Lookup, output: &mut Option<ObjectRef>) -> bool {
        note(&self.trace, format!("create:{}:{args:?}", self.name));
        if self.success {
            *output = Some(self.result.clone());
            if self.register {
                manager.register_source(args.id, Some(self.result.clone()));
            }
        }
        self.success
    }
}
#[test]
fn ss9_lookup_gate_cache_rejection_and_ordered_uncached_creators() {
    let trace = Trace::default();
    let count = Arc::new(AtomicUsize::new(0));
    let mut manager = SndMgr::default();
    assert_eq!(
        (manager.default_2d, manager.default_3d, manager.system_volume()),
        (Some(0), Some(0), 1024)
    );
    assert!(manager.init(&mut ManagerFixture(count.clone())));
    let sound = object("S", &trace, false, true);
    let sentinel = object("sentinel", &trace, true, true);
    let mut output = Some(sentinel.clone());
    manager.register_sound(7, Some(sound.clone()));
    manager.register_sound(7, Some(sentinel.clone())); // first pointer wins
    let args = Lookup {
        id: 7,
        a: 12,
        b: 34,
        iid: 88,
    };
    assert_eq!(manager.get_sound(args, &mut output), Ok(false));
    assert!(Arc::ptr_eq(output.as_ref().unwrap(), &sentinel));
    assert!(trace.lock().unwrap().is_empty());
    manager.register_sound_creator(Some(Arc::new(Creator {
        name: 1,
        trace: trace.clone(),
        success: true,
        result: sentinel.clone(),
        register: false,
    })));
    count.store(1, Ordering::SeqCst);
    assert_eq!(manager.get_sound(args, &mut output), Ok(false));
    assert_eq!(*trace.lock().unwrap(), ["qi:S:88"]);
    manager.register_source_creator(
        20,
        Some(Arc::new(Creator {
            name: 20,
            trace: trace.clone(),
            success: true,
            result: sentinel.clone(),
            register: false,
        })),
    );
    manager.register_source_creator(
        10,
        Some(Arc::new(Creator {
            name: 10,
            trace: trace.clone(),
            success: false,
            result: sentinel.clone(),
            register: false,
        })),
    );
    let args = Lookup {
        id: 99,
        a: 12,
        b: 34,
        iid: 88,
    };
    trace.lock().unwrap().clear();
    for _ in 0..2 {
        assert!(manager.get_source(args, &mut output));
    }
    let expected = [format!("create:10:{args:?}"), format!("create:20:{args:?}")];
    assert_eq!(
        *trace.lock().unwrap(),
        expected.iter().chain(&expected).cloned().collect::<Vec<_>>()
    );
    assert!(!manager.get_resource([1, 2, 3], 88, &mut output));
    assert!(Arc::ptr_eq(output.as_ref().unwrap(), &sentinel));
    manager.register_source(99, None);
    trace.lock().unwrap().clear();
    assert!(!manager.get_source(args, &mut output));
    assert!(trace.lock().unwrap().is_empty());
    assert!(Arc::ptr_eq(output.as_ref().unwrap(), &sentinel));
    manager.unregister_source(99);
    manager.register_source_creator(
        20,
        Some(Arc::new(Creator {
            name: 20,
            trace: trace.clone(),
            success: true,
            result: sentinel,
            register: true,
        })),
    );
    assert!(manager.get_source(args, &mut output));
    trace.lock().unwrap().clear();
    assert!(manager.get_source(args, &mut output));
    assert_eq!(*trace.lock().unwrap(), ["qi:sentinel:88"]);
}
#[test]
fn ss9_autoplay_failed_play_front_only_cleanup_and_source_survival() {
    let trace = Trace::default();
    let mut manager = SndMgr::default();
    let a = object("A", &trace, true, false);
    let b = object("B", &trace, true, true);
    let d = object("D", &trace, true, true);
    manager.register_sound(1, Some(a.clone()));
    manager.register_sound(2, Some(b.clone()));
    manager.register_source(42, Some(d.clone()));
    assert_eq!(manager.autoplay(1, 700), Ok(false)); // bypass renderer gate and QI
    assert_eq!(manager.autoplay(2, 800), Ok(true));
    assert_eq!(*trace.lock().unwrap(), ["play:A:700:0:1", "play:B:800:0:1"]);
    assert_eq!(manager.autoplay_count(), 2);
    assert_eq!(Arc::strong_count(&a), 3);
    assert_eq!(Arc::strong_count(&b), 3);
    manager.update();
    assert_eq!(manager.autoplay_count(), 1);
    assert_eq!(Arc::strong_count(&a), 2);
    manager.update();
    assert_eq!(manager.autoplay_count(), 0);
    assert_eq!(Arc::strong_count(&b), 2);
    manager.add_autoplay(b.clone());
    manager.add_autoplay(b.clone());
    assert_eq!(manager.autoplay_count(), 2);
    manager.set_default_2d(77);
    manager.set_system_volume(-12);
    manager.register_sound(0, Some(b.clone()));
    assert_eq!(manager.system_autoplay(123, -9), Ok(true)); // missing mapping still calls ID zero
    assert!(manager.shutdown());
    assert_eq!((manager.default_2d, manager.system_volume()), (Some(77), -12));
    assert_eq!(Arc::strong_count(&a), 1);
    assert_eq!(Arc::strong_count(&b), 1);
    let mut output = None;
    assert!(manager.get_source(
        Lookup {
            id: 42,
            a: 0,
            b: 0,
            iid: 88
        },
        &mut output
    ));
    assert_eq!(Arc::strong_count(&d), 2);
    drop(manager);
    assert_eq!(Arc::strong_count(&d), 1);
}

#[derive(Clone)]
struct Resources(Trace);
impl ManagerResources for Resources {
    fn get_resource(&self, key: [u32; 3], iid: u32, output: &mut Option<ObjectRef>, a: u32, b: u32) -> bool {
        assert_eq!((key, iid, a, b), ([1, 2, 3], 77, 0, 0));
        note(&self.0, "resource");
        *output = Some(object("resource", &self.0, true, true));
        true
    }
}
impl ManagerServices for Resources {
    fn acquire_sound_system(&mut self) -> bool {
        true
    }
    fn register_manager_update(&mut self, offset: u32, period: u32) {
        assert_eq!((offset, period), (0x20, 250));
    }
    fn retain_persist_manager(&mut self) {}
    fn persist_manager_reference(&mut self) -> Option<Arc<dyn ManagerResources>> {
        Some(Arc::new(self.clone()))
    }
}
#[test]
fn ss9_resource_arguments_and_playing_front_blocking() {
    let trace = Trace::default();
    let mut manager = SndMgr::default();
    assert!(manager.init(&mut Resources(trace.clone())));
    let mut output = None;
    assert!(manager.get_resource([1, 2, 3], 77, &mut output));
    assert!(output.is_some());
    assert_eq!(*trace.lock().unwrap(), ["resource"]);
    let playing = Arc::new(AtomicBool::new(true));
    let a: ObjectRef = Arc::new(Object {
        name: "A",
        trace: trace.clone(),
        query_ok: true,
        play_ok: true,
        playing: playing.clone(),
    });
    let b = object("B", &trace, true, true);
    manager.add_autoplay(a.clone());
    manager.add_autoplay(b.clone());
    trace.lock().unwrap().clear();
    manager.update();
    assert_eq!(manager.autoplay_count(), 2);
    assert_eq!(*trace.lock().unwrap(), ["playing:A"]);
    assert_eq!(Arc::strong_count(&b), 2);
    playing.store(false, Ordering::SeqCst);
    manager.update();
    assert_eq!(manager.autoplay_count(), 1);
    assert_eq!(Arc::strong_count(&a), 1);
    assert_eq!(Arc::strong_count(&b), 2);
    manager.update();
    assert_eq!(manager.autoplay_count(), 0);
    assert_eq!(Arc::strong_count(&b), 1);
}

struct Identity(Trace);
impl ConstructionIdentity for Identity {
    fn query(&self, iid: u32, output: &mut Option<ForeignRef>) -> bool {
        note(&self.0, "QI");
        if iid == 77 {
            *output = Some(Arc::new(77u32));
            true
        } else {
            false
        }
    }
    fn add_ref(&self) {
        note(&self.0, "AddRef");
    }
    fn release(&self) {
        note(&self.0, "Release");
    }
}
struct Constructors(Trace, bool);
impl ClassConstructor for Constructors {
    fn construct(&mut self, _: u32) -> Option<Arc<dyn ConstructionIdentity>> {
        self.1
            .then(|| Arc::new(Identity(self.0.clone())) as Arc<dyn ConstructionIdentity>)
    }
}
#[test]
fn ss10_all_directors_inherited_identity_and_factory_worked_case() {
    for classes in [&SOUND_CLASSES[..], &MAC_CLASSES[..], &FRESH_CLASSES[..]] {
        let trace = Trace::default();
        let mut state = DirectorState::with_classes(classes);
        assert_eq!(state.release(), 0);
        assert_eq!(state.add_ref(), 1);
        assert_eq!(state.ref_count(), 1);
        assert!(state.can_unload());
        assert_eq!(state.release(), 0);
        let mut offset = 77;
        assert!(!state.query(0, &mut offset));
        assert_eq!(offset, 77);
        for (iid, expected) in [(1, 0), (0xA21EE941, 0), (0x03FA40BF, 4)] {
            assert!(state.query(iid, &mut offset));
            assert_eq!(offset, expected);
        }
        assert_eq!(state.ref_count(), 3);
        assert!(!state.initialize(None, None, "wrong"));
        assert!(state.initialize(Some(Arc::new(1u32)), Some(Arc::new(2u32)), "plugin.dll"));
        let mut path = String::new();
        assert!(state.library_path(&mut path));
        assert_eq!(path, "plugin.dll");
        assert!(state.com().is_some());
        assert!(state.framework().is_some());
        assert_eq!(state.size_query(), 0);
        assert!(state.no_op());
        let sentinel: ForeignRef = Arc::new(99u32);
        let mut output = Some(sentinel.clone());
        let mut constructors = Constructors(trace.clone(), true);
        assert_eq!(state.get_class_object(0, 88, &mut output, &mut constructors), Ok(false));
        assert!(Arc::ptr_eq(output.as_ref().unwrap(), &sentinel));
        assert_eq!(
            state.get_class_object(classes[0], 88, &mut output, &mut constructors),
            Ok(false)
        );
        assert_eq!(*trace.lock().unwrap(), ["QI", "AddRef", "Release"]);
        constructors.1 = false;
        assert_eq!(
            state.get_class_object(classes[0], 88, &mut output, &mut constructors),
            Err(NativeFault::NullConstructionIdentity)
        );
        let mut enumerated = Vec::new();
        state.enum_classes(&mut |class| enumerated.push(class));
        assert_eq!(enumerated, classes);
    }
    assert_eq!(MAC_CONSTRUCTION_LAYOUTS.map(|(class, _, _)| class), MAC_CLASSES);
    assert_eq!(
        MAC_CONSTRUCTION_LAYOUTS.map(|(_, size, offset)| (size, offset)),
        [(0xD8, 0), (0x100, 0xD8), (0xA0, 8), (0x54, 4), (0x14, 0)]
    );
}
struct Child {
    trace: Trace,
    class: u32,
    unload: bool,
}
impl DirectorChild for Child {
    fn initialize(&self, _: &ForeignRef, path: &str) -> bool {
        assert_eq!(path, "sound.dll");
        note(&self.trace, format!("init:{}", self.class));
        false // InitializeCOM ignores child results.
    }
    fn enum_classes(&self, visit: &mut dyn FnMut(u32)) {
        visit(self.class);
    }
    fn get_class_object(&self, class: u32, _: u32, output: &mut Option<ForeignRef>) -> Result<bool, NativeFault> {
        note(&self.trace, format!("child:{}", self.class));
        if class == self.class {
            *output = Some(Arc::new(class));
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn can_unload(&self) -> bool {
        note(&self.trace, format!("unload:{}", self.class));
        self.unload
    }
}
#[test]
fn ss10_children_precede_local_factories_and_successful_qi_transfers_output() {
    for classes in [&SOUND_CLASSES[..], &MAC_CLASSES[..], &FRESH_CLASSES[..]] {
        let trace = Trace::default();
        let mut state = DirectorState::with_classes(classes);
        for (class, unload) in [(10, false), (20, true)] {
            state.children.push(Arc::new(Child {
                trace: trace.clone(),
                class,
                unload,
            }));
        }
        assert!(state.initialize(Some(Arc::new(1u32)), None, "sound.dll"));
        assert_eq!(*trace.lock().unwrap(), ["init:10", "init:20"]);
        let mut enumerated = Vec::new();
        state.enum_classes(&mut |class| enumerated.push(class));
        assert_eq!(enumerated, [vec![10, 20], classes.to_vec()].concat());
        trace.lock().unwrap().clear();
        assert!(!state.can_unload());
        assert_eq!(*trace.lock().unwrap(), ["unload:10"]);
        trace.lock().unwrap().clear();
        state.add_ref();
        assert!(state.can_unload());
        assert!(trace.lock().unwrap().is_empty());
        let mut constructors = Constructors(trace.clone(), true);
        let mut output = None;
        assert_eq!(state.get_class_object(20, 77, &mut output, &mut constructors), Ok(true));
        assert_eq!(*trace.lock().unwrap(), ["child:10", "child:20"]);
        assert_eq!(output.as_ref().unwrap().downcast_ref::<u32>(), Some(&20));
        trace.lock().unwrap().clear();
        assert_eq!(
            state.get_class_object(classes[0], 77, &mut output, &mut constructors),
            Ok(true)
        );
        assert_eq!(*trace.lock().unwrap(), ["child:10", "child:20", "QI"]);
        assert_eq!(output.as_ref().unwrap().downcast_ref::<u32>(), Some(&77));
    }
}
struct Cache(Trace);
impl PropertyCache for Cache {
    fn create_instance_set(&self, instance: u32) -> bool {
        assert_eq!(instance, 0xCC3014EE);
        note(&self.0, "CreateInstanceSet");
        false
    }
}
struct DirectorFixture {
    service: Option<ForeignRef>,
    cache: Option<Arc<dyn PropertyCache>>,
    trace: Trace,
    creates: usize,
}
impl DirectorHost for DirectorFixture {
    fn register_class(&mut self, _: u32) {}
    fn register_hook(&mut self, _: u32) {}
    fn create(&mut self, _: u32, _: u32) -> Option<ForeignRef> {
        self.creates += 1;
        if self.creates == 1 {
            self.service.clone()
        } else {
            None
        }
    }
    fn add_service(&mut self, _: ForeignRef) {
        note(&self.trace, "AddService");
    }
    fn register_handler(&mut self, _: &ForeignRef, _: ForeignRef) {}
    fn disable_updates(&mut self, _: &ForeignRef) {
        note(&self.trace, "DisableUpdates");
    }
    fn sound_system_service(&mut self) -> Option<ForeignRef> {
        self.service.clone()
    }
    fn acquire_property_cache(&mut self, _: &ForeignRef) -> Option<Arc<dyn PropertyCache>> {
        self.cache.clone()
    }
}
#[test]
fn ss10_post_init_retains_cache_ignores_false_and_distinguishes_nulls() {
    let trace = Trace::default();
    let cache: Arc<dyn PropertyCache> = Arc::new(Cache(trace.clone()));
    let service: ForeignRef = Arc::new(1u32);
    let mut host = DirectorFixture {
        service: Some(service.clone()),
        cache: Some(cache.clone()),
        trace: trace.clone(),
        creates: 0,
    };
    let mut director = SoundDirector::construct(&mut host);
    assert!(director.on_start(&mut host));
    assert!(director.manager.is_none());
    assert_eq!(*trace.lock().unwrap(), ["AddService"]);
    assert_eq!(director.post_app_init(&mut host), Ok(true));
    assert_eq!(Arc::strong_count(&cache), 2);
    assert_eq!(Arc::strong_count(&service), 3);
    assert_eq!(*trace.lock().unwrap(), ["AddService", "CreateInstanceSet"]);
    host.cache = None;
    assert_eq!(
        director.post_app_init(&mut host),
        Err(NativeFault::MissingPropertyCache)
    );
    host.service = None;
    assert_eq!(director.post_app_init(&mut host), Ok(false));
    assert!(director.pre_app_shutdown(&mut host));
    assert_eq!(trace.lock().unwrap().last().unwrap(), "DisableUpdates");
}

struct FileFixture {
    bytes: Vec<u8>,
    open_ok: bool,
    read_ok: bool,
    allocate_ok: bool,
    resolved: Option<String>,
    trace: Trace,
    conversions: Vec<Vec<u8>>,
}
impl FileFixture {
    fn text(text: &str) -> Self {
        Self {
            bytes: text.as_bytes().to_vec(),
            open_ok: true,
            read_ok: true,
            allocate_ok: true,
            resolved: None,
            trace: Trace::default(),
            conversions: Vec::new(),
        }
    }
}
impl PlaylistIo for FileFixture {
    fn resolve_path(&mut self, _: &str) -> Option<String> {
        self.resolved.clone()
    }
    fn open(&mut self, path: &str, a: u32, b: u32, c: u32) -> bool {
        assert_eq!((a, b, c), (1, 2, 1));
        note(&self.trace, format!("open:{path}"));
        self.open_ok
    }
    fn size(&mut self) -> usize {
        self.bytes.len()
    }
    fn allocate(&mut self, size: usize) -> Option<Vec<u8>> {
        self.allocate_ok.then(|| vec![0; size])
    }
    fn read(&mut self, buffer: &mut [u8]) -> bool {
        buffer.copy_from_slice(&self.bytes);
        self.read_ok
    }
    fn close(&mut self) {
        note(&self.trace, "close");
    }
    fn convert(&mut self, line: &mut Vec<u8>, source: i32, destination: u32) -> bool {
        assert_eq!((source, destination), (-1, 8));
        self.conversions.push(line.clone());
        false
    }
    fn decimal_long(&mut self, text: &str) -> i32 {
        // Fixture C decimal-prefix conversion; overflow is outside these worked cases.
        let text = text.trim_start();
        let end = text
            .char_indices()
            .take_while(|(index, ch)| ch.is_ascii_digit() || (*index == 0 && matches!(ch, '+' | '-')))
            .last()
            .map_or(0, |(index, ch)| index + ch.len_utf8());
        text[..end].parse().unwrap_or(0)
    }
}
#[test]
fn ss11_open_appends_plain_comments_urls_and_keeps_entries_on_io_failure() {
    let mut list = PlayList::default();
    let mut io = FileFixture::text("a.mp3\r\n#note\r\nhttp://host/b.mp3\r\n");
    for count in [3, 6] {
        assert_eq!(list.open("C:\\Music\\list.m3u", &mut io), Ok(true));
        assert_eq!(list.entries.len(), count);
    }
    assert_eq!(
        list.entries[0],
        Entry {
            title: "a".into(),
            path: "C:\\Music\\a.mp3".into(),
            duration: 0
        }
    );
    assert_eq!(list.entries[1].path, "C:\\Music\\#note");
    assert_eq!(list.entries[2].path, "C:\\Music\\http://host/b.mp3");
    io.open_ok = false;
    assert_eq!(list.open("missing.m3u", &mut io), Ok(false));
    assert_eq!(list.entries.len(), 6);
    assert_eq!(list.stored_path, "missing.m3u");
    assert_eq!(list.directory, "");
    io.open_ok = true;
    io.read_ok = false;
    io.resolved = Some("D:/resolved/list".into());
    assert_eq!(list.open("supplied", &mut io), Ok(false));
    assert_eq!(list.entries.len(), 6);
    assert_eq!(list.directory, "D:/resolved/");
    assert_eq!(io.trace.lock().unwrap().last().unwrap(), "close");
}
#[test]
fn ss11_plain_open_restores_raw_input_and_reconverts_first_line() {
    // §9.1's saved pre-header input includes whitespace removed by _GetLine.
    let mut list = PlayList::default();
    let mut io = FileFixture::text(" \t a.mp3 \r\nb.mp3\r\n");
    assert_eq!(list.open("C:\\Music\\list", &mut io), Ok(true));
    assert_eq!(
        io.conversions,
        [b"a.mp3".to_vec(), b"a.mp3".to_vec(), b"b.mp3".to_vec()]
    );
    assert_eq!(list.entries.len(), 2);
    assert_eq!(list.entries[0].path, "C:\\Music\\a.mp3");
    assert!(list.remaining_input.is_empty());
}
#[test]
fn ss11_extended_titles_partial_failure_and_pls_worked_cases() {
    let mut list = PlayList::default();
    let mut io = FileFixture::text("#EXTM3U\r\n#EXTINF:-1,Two, parts\r\na.mp3\r\n#comment\r\n");
    assert_eq!(list.open("C:\\Music\\list", &mut io), Ok(false));
    assert_eq!(
        list.entries,
        [Entry {
            title: "Two, parts".into(),
            path: "C:\\Music\\a.mp3".into(),
            duration: -1
        }]
    );
    let pls = "[playlist]\r\nFile2=b.mp3\r\nTitle99=Bee\r\nLength7=12\r\nFile1=a.mp3\r\nTitle1=Aye\r\nLength1=-1\r\nNumberOfEntries=2\r\nVersion=99\r\n";
    let mut list = PlayList::default();
    assert_eq!(list.open("C:\\Music\\list", &mut FileFixture::text(pls)), Ok(true));
    assert_eq!(
        list.entries,
        [
            Entry {
                title: "Bee".into(),
                path: "C:\\Music\\b.mp3".into(),
                duration: 12
            },
            Entry {
                title: "Aye".into(),
                path: "C:\\Music\\a.mp3".into(),
                duration: -1
            }
        ]
    );
    assert!(list.remaining_input.starts_with(b"\nVersion=99"));
    let mut early = PlayList::default();
    assert_eq!(
        early.open(
            "list",
            &mut FileFixture::text("[playlist]\r\nNumberOfEntries=2\r\nFile1=a.mp3\r\n")
        ),
        Ok(false)
    );
    assert!(early.entries.is_empty());
    let truncated = pls.split("NumberOfEntries").next().unwrap();
    assert_eq!(early.open("list", &mut FileFixture::text(truncated)), Ok(false));
    assert_eq!(early.entries.len(), 1);
    assert_eq!(early.entries[0].title, "Bee");
    assert_eq!(early.open("list", &mut FileFixture::text("[playlist]\r\n")), Ok(true));
    assert_eq!(early.entries.len(), 1);
    let mut missing_comma = PlayList::default();
    assert_eq!(
        missing_comma.open(
            "list",
            &mut FileFixture::text("#EXTM3U\r\n#EXTINF:Title only\r\na.mp3\r\n")
        ),
        Ok(true)
    );
    assert_eq!(
        (
            missing_comma.entries[0].duration,
            missing_comma.entries[0].title.as_str()
        ),
        (0, "Title only")
    );
}
#[test]
fn ss11_line_cursor_repeats_unterminated_tail_and_ignores_conversion_status() {
    let mut source = b"a.mp3".to_vec();
    for _ in 0..2 {
        assert_eq!(
            PlayList::get_line(&mut source, |_, source, destination| {
                assert_eq!((source, destination), (-1, 8));
                false
            }),
            Some(b"a.mp3".to_vec())
        );
        assert_eq!(source, b"a.mp3");
    }
    let mut source = b" \r\n\t a.mp3 \r\n".to_vec();
    assert_eq!(
        PlayList::get_line(&mut source, |line, _, _| {
            line.push(b'!');
            false
        }),
        Some(b"a.mp3!".to_vec())
    );
    assert_eq!(
        PlayList::get_line(&mut source, |_, _, _| panic!("no blank conversion")),
        None
    );
    // No BOM detection, and conversion failure is not an encoding error here.
    let mut source = vec![0xEF, 0xBB, 0xBF, 0x80, 0xE9, b'\r'];
    assert_eq!(
        PlayList::get_line(&mut source, |_, _, _| false),
        Some(vec![0xEF, 0xBB, 0xBF, 0x80, 0xE9])
    );
}
#[test]
fn ss11_literal_backslash_paths_and_ss12_no_separator_insertion() {
    for (path, expected) in [
        ("sub\\a.mp3", "C:\\Music\\Lists\\sub\\a.mp3"),
        (".\\a.mp3", "C:\\Music\\Lists\\a.mp3"),
        ("..\\a.mp3", "C:\\Music\\a.mp3"),
        ("...\\a.mp3", "C:\\a.mp3"),
        (".hidden\\a.mp3", "C:\\Music\\Lists\\a.mp3"),
        ("sub/../a.mp3", "C:\\Music\\Lists\\sub/../a.mp3"),
        ("http://host/a.mp3", "C:\\Music\\Lists\\http://host/a.mp3"),
        ("D:a.mp3", "D:a.mp3"),
        ("\\root\\a.mp3", "\\root\\a.mp3"),
        ("\\\\host\\share\\a.mp3", "\\\\host\\share\\a.mp3"),
    ] {
        assert_eq!(
            PlayList::make_absolute_path("C:\\Music\\Lists\\", path),
            Ok(expected.into()),
            "{path}"
        );
    }
    for (base, path, expected) in [
        ("C:/Music/", "sub\\a.mp3", "C:/Music/sub\\a.mp3"),
        ("C:\\", "..\\a.mp3", "a.mp3"),
        ("C:\\Music", "a.mp3", "C:\\Musica.mp3"),
    ] {
        assert_eq!(PlayList::make_absolute_path(base, path), Ok(expected.into()));
    }
}
#[test]
fn ss11_reserved_runtime_io_boundaries_remain_explicit() {
    let mut list = PlayList::default();
    assert_eq!(
        list.open("list", &mut FileFixture::text("")),
        Err(Unsupported("SS11 zero-byte observation"))
    );
    let mut io = FileFixture::text(" \r\n");
    io.allocate_ok = false;
    assert_eq!(
        list.open("list", &mut io),
        Err(Unsupported("SS11 allocation observation"))
    );
    io.allocate_ok = true;
    assert_eq!(list.open("list", &mut io), Ok(true));
    assert!(list.entries.is_empty());
    // Early NULs become spaces; the last byte is not touched.
    let mut io = FileFixture::text("a\0b.mp3\r\n");
    assert_eq!(list.open("list", &mut io), Ok(true));
    assert_eq!(list.entries[0].path, "a b.mp3");
//! Worked cases from sound-system.md, SS1–SS7.
use super::fresh::*;
use super::{hit::*, track::*, *};
use std::sync::atomic::{AtomicI32, Ordering};

#[derive(Default)]
struct Source {
    frequency: AtomicI32,
    calls: Mutex<Vec<String>>,
}
impl Source {
    fn record(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}
impl Sound for Source {
    fn load(&self) -> bool {
        self.record("load".into());
        true
    }
    fn unload(&self) -> bool {
        true
    }
    fn is_loaded(&self) -> bool {
        true
    }
    fn is_playing(&self) -> bool {
        true
    }
    fn pause(&self) {
        self.record("pause".into());
    }
    fn unpause(&self) {
        self.record("unpause".into());
    }
    fn play(&self, p: u32, looping: bool, flag: bool) -> bool {
        self.record(format!("play:{p}:{looping}:{flag}"));
        false
    }
    fn set_volume(&self, v: i32) -> bool {
        self.record(format!("vol:{v}"));
        true
    }
    fn frequency(&self) -> i32 {
        self.frequency.load(Ordering::Relaxed)
    }
    fn set_frequency(&self, v: i32) -> bool {
        self.record(format!("freq:{v}"));
        true
    }
    fn set_pan(&self, v: i32) -> bool {
        self.record(format!("pan:{v}"));
        true
    }
    fn spatial(&self, head: bool, min: f32, max: f32, pos: Option<[f32; 3]>) -> bool {
        self.record(format!("spatial:{head}:{min}:{max}:{pos:?}"));
        true
    }
    fn set_velocity(&self, v: [f32; 3]) -> bool {
        self.record(format!("velocity:{v:?}"));
        true
    }
    fn set_min_distance(&self, v: f32) -> bool {
        self.record(format!("min:{v}"));
        true
    }
    fn set_max_distance(&self, v: f32) -> bool {
        self.record(format!("max:{v}"));
        true
    }
    fn set_position(&self, v: [f32; 3]) -> bool {
        self.record(format!("position:{v:?}"));
        true
    }
}

#[derive(Default)]
struct TrackSpy {
    effects: Vec<TrackEffect>,
    errors: Vec<String>,
    error_result: i32,
    aliases: BTreeMap<u32, u32>,
}
impl TrackIo for TrackSpy {
    fn effect(&mut self, effect: TrackEffect) -> bool {
        self.effects.push(effect);
        true
    }
    fn report(&mut self, message: &str) -> i32 {
        self.errors.push(message.into());
        self.error_result
    }
    fn entry(&mut self, id: u32) -> Option<usize> {
        Some(id as usize)
    }
    fn alias(&mut self, id: u32) -> u32 {
        self.aliases.get(&id).copied().unwrap_or(0)
    }
}

#[test]
fn ss1_mode3_and_play_order_suppress_notifications() {
    let mut track = HitTrackPlayer {
        executing: true,
        ..Default::default()
    };
    track
        .properties
        .extend([(0xFF4D_CD0D, 3), (0xFF30_3B1B, 1), (0xFF59_FDD8, 23)]);
    let mut io = TrackSpy::default();
    assert!(track.play_with([9, 512, (-20i32) as u32, 77], &mut io));
    assert_eq!(
        io.effects,
        [
            TrackEffect::Kill,
            TrackEffect::Start,
            TrackEffect::StartDependent(23),
            TrackEffect::Unpause
        ]
    );
    assert_eq!(
        [
            track.properties[&0xFFB4_335C],
            track.properties[&0xFF90_AAC8],
            track.properties[&0xFF3E_C880]
        ],
        [9, 512, -20]
    );
    assert_eq!(track.properties[&PAYLOAD_PROPERTIES[3]], 77);
    assert_eq!(track.flags, [false, false, true]);
    assert!(track.property_notifications.is_empty());
}

#[test]
fn ss1_byte_width_wait_and_strict_ramp_worked_cases() {
    let mut track = HitTrackPlayer::default();
    let mut io = TrackSpy::default();
    track
        .execute_next(&[4, 13, 255, 5, 14, 0x78, 0x56, 0x34, 0x12], &mut io)
        .unwrap();
    assert_eq!((track.pc, track.properties[&13]), (3, 255));
    track
        .execute_next(&[4, 13, 255, 5, 14, 0x78, 0x56, 0x34, 0x12], &mut io)
        .unwrap();
    assert_eq!((track.pc, track.properties[&14]), (9, 0x1234_5678));
    track.pc = 0;
    track.executing = true;
    track.wait = 30;
    track.properties.extend([
        (13, 30),
        (0xFF0A_D592, 1),
        (0xFF47_F685, 14),
        (14, 0),
        (0xFF2D_16F2, 10),
        (0xFFD1_C82F, 100),
    ]);
    track.timer_callback(20, &[9, 13], &mut io).unwrap();
    assert_eq!(track.wait, 10);
    track.timer_callback(20, &[9, 13], &mut io).unwrap();
    assert_eq!(track.wait, 30); // excess ten milliseconds is discarded
    track.wait = 1000;
    track.properties.insert(14, 0);
    track.timer_callback(100, &[], &mut io).unwrap();
    assert_eq!((track.properties[&14], track.properties[&0xFF0A_D592]), (10, 1));
    track.timer_callback(100, &[], &mut io).unwrap();
    assert_eq!((track.properties[&14], track.properties[&0xFF0A_D592]), (10, 0));
    track.pc = 0;
    assert_eq!(track.execute_next(&[0x58], &mut io), Err(Unsupported("SS1")));
    track.pc = 0;
    track.execute_next(&[0], &mut io).unwrap();
    assert_eq!(io.errors, ["Illegal Instruction"]);
}

#[test]
fn ss1_alias_suppression_checked_register_and_callback_countdown() {
    let mut track = HitTrackPlayer::default();
    let mut io = TrackSpy::default();
    io.aliases.insert(13, 0xFF90_AAC8);
    assert!(track.set_register(13, -20, 1, &mut io));
    assert_eq!(track.properties[&0xFF90_AAC8], -20);
    assert_eq!(track.flags, [false, false, true]);
    assert!(io.effects.is_empty());
    // Only the low byte suppresses notification (§3.3).
    assert!(track.set_register(13, 255, 0x100, &mut io));
    assert_eq!(io.effects, [TrackEffect::VolPan]);
    assert_eq!(track.flags, [false, true, false]);
    track.execute_next(&[4, 75, 1, 4, 74, 2], &mut io).unwrap();
    assert_eq!(track.properties[&1], 1);
    track.execute_next(&[4, 75, 1, 4, 74, 2], &mut io).unwrap();
    assert_eq!(track.properties[&74], 2);
    assert_eq!(io.errors, ["Illegal Register Id"]);

    track.executing = true;
    track.wait = 1000;
    track.note_countdown = 2;
    io.effects.clear();
    track.timer_callback(20, &[], &mut io).unwrap();
    assert_eq!((track.wait, track.note_countdown), (980, 1));
    assert!(io.effects.is_empty());
    track.timer_callback(400, &[], &mut io).unwrap();
    assert_eq!((track.wait, track.note_countdown), (580, 0));
    assert_eq!(io.effects, [TrackEffect::NoteOff]);
}

#[test]
fn ss1_every_reserved_opcode_remains_explicitly_unsupported() {
    // The exact residual set in §3.3 must never become successful no-ops.
    let reserved = [0x0B, 0x0E]
        .into_iter()
        .chain(0x10..=0x19)
        .chain(0x1B..=0x1C)
        .chain(0x1F..=0x33)
        .chain(0x3E..=0x49)
        .chain(0x4B..=0x5C)
        .chain(0x5F..=0x61);
    for opcode in reserved {
        let mut track = HitTrackPlayer::default();
        assert_eq!(
            track.execute_next(&[opcode], &mut TrackSpy::default()),
            Err(Unsupported("SS1")),
            "opcode {opcode:02X}"
        );
    }
}

#[test]
fn ss2_whole_file_duplicate_registration_semantic_duck_and_random() {
    fn block(offsets: &[u32]) -> Arc<TrackDataBlock> {
        let mut bytes = b"prefix!ENTP".to_vec(); // deliberately unaligned; TRAX is not required
        for offset in offsets {
            bytes.extend(7u32.to_le_bytes());
            bytes.extend(offset.to_le_bytes());
        }
        bytes.extend(b"EENT");
        bytes.resize(100, 0);
        Arc::new(TrackDataBlock::parse("same.hit".into(), bytes).unwrap())
    }
    let a = block(&[32]);
    let b = block(&[48, 64]);
    let mut manager = HitMan::default();
    manager.register_block(a.clone());
    manager.register_block(b.clone());
    let (found, offset) = manager.entry(7).unwrap();
    assert!(std::ptr::eq(found, b.as_ref()));
    assert_eq!(offset, 64);
    assert!(std::ptr::eq(
        found.entry_bytes(offset).unwrap().as_ptr(),
        found.data[64..].as_ptr()
    ));
    manager.register_block(a.clone());
    assert_eq!(manager.entry(7).unwrap().1, 32);
    assert_eq!(manager.blocks.len(), 2);
    assert!(!manager.register_semantic(0, Arc::new(12u32)));
    assert_eq!(manager.semantics[&0].downcast_ref::<u32>(), Some(&12));
    manager.initialized = true;
    manager.set_duck_priority(1, 5);
    manager.set_duck_priority(2, -2);
    assert_eq!(manager.duck_max(), 5);
    manager.set_duck_priority(1, 0);
    assert_eq!(
        (manager.duck_max(), manager.registers[&27], manager.view_updates),
        (0, 0, 3)
    );
    assert!(manager.pause());
    assert!(!manager.paused);
    assert!(manager.unpause());
    assert_eq!(manager.random(20, 10), 13);
    assert_eq!(manager.random_seed, 0xBE39_E1CC);
    let seed = manager.random_seed;
    assert_eq!(manager.random(7, 7), 7);
    assert_eq!(manager.random_seed, seed);
}

#[derive(Default)]
struct Backend {
    binds: usize,
    forwards: Vec<(u32, u32, f32)>,
    resolved: BTreeMap<u32, Arc<Properties>>,
}
impl ChannelBackend for Backend {
    fn create(&mut self, _: u32, _: u32) -> Option<SoundRef> {
        None
    }
    fn initialize(&mut self, _: &SoundRef) {}
    fn query_source(&mut self, _: &SoundRef, _: u32) -> bool {
        false
    }
    fn configure_3d(&mut self, _: &SoundRef, _: u32, _: &Properties) -> bool {
        false
    }
    fn refresh_source_state(&mut self, _: &SoundRef) {}
    fn bind_source(&mut self, _: &SoundRef, _: Option<&directors::ForeignRef>, mode: u32) -> bool {
        assert_eq!(mode, 2);
        self.binds += 1;
        true
    }
    fn optional_float(&mut self, _: &SoundRef, iid: u32, slot: u32, value: f32) -> bool {
        self.forwards.push((iid, slot, value));
        false
    }
    fn resolve_properties(&mut self, key: ResourceKey, iid: u32) -> Option<Arc<Properties>> {
        assert_eq!((key.kind, key.group, iid), (0xEBFE_E345, 0xFC45_CAF7, 0xEBB1_0CDD));
        self.resolved.get(&key.instance).cloned()
    }
}

#[test]
fn ss3_notifications_preserve_return_asymmetry_and_retained_sets() {
    let child = Arc::new(Source::default());
    let mut channel = HitChannel {
        source: Some(child),
        ..Default::default()
    };
    channel.properties.insert(0xFF09_9DB2, 22051);
    channel
        .float_properties
        .extend([(0xFF30_71B9, 0.5), (0xFF01_50F2, 1.25), (0xFF86_FE79, -0.5)]);
    let mut backend = Backend::default();
    assert!(channel.notify(0xFF09_9DB2, &mut backend)); // inverted pitch return is discarded
    assert!(channel.notify(0xFF01_50F2, &mut backend));
    assert!(!channel.notify(0xFF86_FE79, &mut backend));
    assert_eq!(backend.forwards, [(0x6B04_46EB, 6, 1.25), (0x4B79_C1A9, 7, -0.5)]);
    let parent = Arc::new([(0xFF53_DBF3, 512)].into_iter().collect());
    backend.resolved.insert(7, Arc::clone(&parent));
    let notifying = Arc::new([(0xFF4C_97BE, 7), (0xFF90_AAC8, -3)].into_iter().collect());
    assert!(channel.notify_inherited(Arc::clone(&notifying), &mut backend));
    assert_eq!(Arc::strong_count(&notifying), 2);
    assert_eq!(Arc::strong_count(&parent), 3);
    assert_eq!(channel.play_value as i32, -1);
    channel.notify_inherited(Arc::new(Properties::new()), &mut backend);
    assert_eq!(Arc::strong_count(&notifying), 1);
    assert_eq!(Arc::strong_count(&parent), 2);
    let retained: directors::ForeignRef = Arc::new(7u32);
    channel.retained_source = Some(Arc::clone(&retained));
    assert!(channel.notify(0xFFAE_C905, &mut backend));
    assert_eq!(Arc::strong_count(&retained), 1);
}

#[test]
fn ss1_instruction_budget_reports_after_102_and_preserves_stack_offsets() {
    let mut track = HitTrackPlayer {
        executing: true,
        ..Default::default()
    };
    let mut io = TrackSpy {
        error_result: 1,
        ..Default::default()
    };
    track.timer_callback(0, &[15; 103], &mut io).unwrap();
    assert_eq!(track.pc, 102);
    assert_eq!(
        io.errors,
        ["Too many instructions executed without wait (infinite loop?)"]
    );
    track.pc = 0;
    let stream = [7, 6, 0, 0, 0, 12, 8];
    track.execute_next(&stream, &mut io).unwrap();
    assert_eq!((track.pc, track.return_stack.clone()), (6, vec![5]));
    track.execute_next(&stream, &mut io).unwrap();
    assert_eq!(track.pc, 5);
    assert!(track.return_stack.is_empty());
}

#[test]
fn ss6_path_init_ignores_load_boolean_and_preserves_directory() {
    struct Paths(Vec<String>);
    impl PathScoreInput for Paths {
        fn load_path(&mut self, path: &str) -> (ScoreData, bool) {
            self.0.push(path.to_owned());
            (
                ScoreData {
                    rows: 1,
                    columns: 1,
                    cells: vec![CellData::default()],
                    ..Default::default()
                },
                false,
            )
        }
    }
    let mut player = FreshPlayer::default();
    let mut paths = Paths(Vec::new());
    player.init_path("Music/score.txt", &mut paths).unwrap();
    assert!(player.initialized);
    assert_eq!(player.data_directory, "Music/");
    assert_eq!(paths.0, ["Music/score.txt"]);
}

#[test]
fn ss3_pitch_pan_volume_patch_and_interface_worked_cases() {
    let child = Arc::new(Source::default());
    child.frequency.store(22051, Ordering::Relaxed);
    let mut channel = HitChannel {
        source: Some(child.clone()),
        ..Default::default()
    };
    channel.float_properties.insert(0xFF30_71B9, 0.5);
    assert!(channel.update_pitch());
    assert_eq!(channel.properties[&0xFF09_9DB2], 11025);
    assert!(child.calls().is_empty());
    channel.properties.insert(0xFF09_9DB2, 22051);
    assert!(!channel.update_pitch());
    assert_eq!(child.calls(), ["freq:11025"]);
    channel
        .properties
        .extend([(0xFF90_AAC8, -3), (0xFF53_DBF3, 512), (0xFF3E_C880, -20)]);
    channel.update_vol_pan();
    assert_eq!(channel.play_value as i32, -1);
    channel.properties.extend([(0xFFCC_156F, 1), (0xFF90_AAC8, 1024)]);
    channel.update_vol_pan();
    assert_eq!(channel.play_value, 512);
    assert!(child.calls().iter().any(|c| c == "max:100000"));
    channel.properties.insert(0xFFA6_A614, -1);
    channel.update_vol_pan();
    assert!(child.calls().iter().any(|c| c == "max:-1"));
    let mut backend = Backend::default();
    assert!(channel.update_patch(&mut backend));
    assert_eq!(backend.binds, 1);
    let mut empty = HitChannel::default();
    assert!(!empty.update_patch(&mut backend));
    assert_eq!(backend.binds, 1);
    let owner = Arc::new(Mutex::new(channel));
    for (iid, offset) in [
        (0x2060_77D2, 0),
        (0x3060_77D2, 4),
        (0x5B34_F0AF, 8),
        (0xFD58_4467, 12),
        (0xAA11_A64F, 16),
        (0x09D0_02D2, 20),
    ] {
        let view = HitChannel::query_interface(&owner, iid).unwrap();
        assert_eq!(view.offset, offset);
        assert_eq!(Arc::strong_count(&owner), 2);
    }
    assert!(HitChannel::query_interface(&owner, 1).is_none());
}

#[derive(Default)]
struct Ini {
    sections: BTreeMap<String, Vec<(String, String)>>,
}
impl IniFile for Ini {
    fn set_filename(&mut self, _: &str) {}
    fn read_entry(&mut self, section: &str, key: &str) -> Option<String> {
        self.enum_entries(section)
            .into_iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }
    fn enum_entries(&mut self, section: &str) -> Vec<(String, String)> {
        self.sections.get(section).cloned().unwrap_or_default()
    }
    fn unsigned_number(&mut self, text: &str) -> Result<u32, Unsupported> {
        Ok(unsigned_number(text))
    }
}
#[derive(Default)]
struct InitHost {
    actions: Vec<IniAction>,
}
impl IniServices for InitHost {
    fn has_manager(&self) -> bool {
        false
    }
    fn has_sound_system(&self) -> bool {
        false
    }
    fn set_default_player(&mut self, _: bool, _: u32) {}
    fn map_extension(&mut self, _: &str, _: u32) {}
    fn data_directory(&self) -> &str {
        "data/"
    }
    fn apply(&mut self, action: &IniAction) -> bool {
        self.actions.push(action.clone());
        false
    }
}
#[test]
fn ss4_numeric_order_duplicates_and_failure_continuation() {
    for (text, expected) in [
        ("12tail", 12),
        ("12AB", 0x12AB),
        ("-1", u32::MAX),
        ("xyz", 0),
        (" AB", 0),
        ("10A?", 0x10A),
        ("4294967296", u32::MAX),
    ] {
        assert_eq!(unsigned_number(text), expected, "{text}");
    }
    let mut ini = Ini::default();
    ini.sections.insert(
        "TrackDataBlocks".into(),
        [("16", "a.hit"), ("2", "b.hit"), ("0x10", "c.hit")]
            .map(|(k, v)| (k.into(), v.into()))
            .to_vec(),
    );
    ini.sections.insert(
        "DBSegmentDirectoryFiles".into(),
        vec![("7".into(), "0x2026960B:wav,0x00000001:bin".into())],
    );
    let mut file = HitIniFile::new(ini);
    let mut host = InitHost::default();
    assert_eq!(file.enum_default_entries(&mut host), Ok(()));
    assert_eq!(
        host.actions,
        [
            IniAction::DirectoryBind {
                group: 7,
                kind: 0x2026_960B,
                extension: "wav".into()
            },
            IniAction::TypeGroup {
                kind: 0x2026_960B,
                group: 7
            },
            IniAction::DirectoryBind {
                group: 7,
                kind: 1,
                extension: "bin".into()
            },
            IniAction::TypeGroup { kind: 1, group: 7 },
            IniAction::LoadTrack("data/b.hit".into()),
            IniAction::LoadTrack("data/c.hit".into())
        ]
    );
    assert!(file.init_resources(&mut host));
}

#[test]
fn ss5_legacy_framing_defaults_self_reference_writer_and_private_copy() {
    let key = ResourceKey {
        kind: 0x0B8D_821A,
        group: 7,
        instance: 42,
    };
    let mut resource = HitPropertySetTrackResource::new(key);
    let mut text = String::from("TKDT,0,,");
    for field in 1..=21 {
        if field != 6 {
            text.push_str(if field == 7 { "1," } else { "," });
        }
    }
    text.push_str("ETKD");
    let mut record = 0u32.to_le_bytes().to_vec();
    record.extend(text.as_bytes());
    resource.properties.insert(LEGACY_FIELDS[5], 99);
    assert!(resource.read_legacy_record(&record));
    assert_eq!(resource.properties[&LEGACY_FIELDS[8]], 0x7DCD_ED3D);
    assert_eq!(resource.properties[&LEGACY_FIELDS[5]], 99);
    let mut self_record = 0u32.to_le_bytes().to_vec();
    self_record.extend(b"TKDT,2,,42,");
    assert!(!resource.read_legacy_record(&self_record));
    assert_eq!(resource.properties[&0xFF6F_9546], 42);
    resource.properties.clear();
    resource.properties.insert(LEGACY_FIELDS[11], -1);
    let text = resource.legacy_text();
    let tokens: Vec<_> = text.split(',').collect();
    assert_eq!(tokens[14], "-1");
    assert_eq!(tokens.len(), 31);
    assert!(resource.write_legacy(|bytes| {
        assert_eq!(bytes, text.as_bytes());
        false
    }));
    resource.reference_count = 5;
    resource.cursor_word = 123;
    resource.properties.insert(LEGACY_FIELDS[11], 512);
    let mut copy = resource.private_copy().unwrap();
    copy.properties.insert(LEGACY_FIELDS[11], 256);
    assert_eq!(resource.properties[&LEGACY_FIELDS[11]], 512);
    assert_eq!(copy.key, key);
    assert_eq!(
        (copy.reference_count, copy.cursor_word, copy.legacy_tables),
        (6, 123, true)
    );
}

#[test]
fn ss5_optional_length_magic_skip_and_empty_field_overlay() {
    let key = ResourceKey {
        kind: 0x0B8D_821A,
        group: 7,
        instance: 42,
    };
    let mut text = String::from("TKDT,2,,");
    for field in 1..=27 {
        text.push_str(match field {
            1 => "7,",
            9 => "0,",
            _ => ",",
        });
    }
    text.push_str("ETKD");
    let mut ordinary = 0u32.to_le_bytes().to_vec();
    ordinary.extend(text.as_bytes());
    let mut special = 0x544B_4432u32.to_le_bytes().to_vec();
    special.extend(b"ignored!"); // §5.2 skips these eight bytes after the header.
    special.extend(text.as_bytes());
    let mut records = vec![text.into_bytes(), ordinary.clone(), special.clone()];
    for inner in [ordinary, special] {
        let mut wrapped = (inner.len() as u32).to_le_bytes().to_vec();
        wrapped.extend(inner);
        records.push(wrapped);
    }
    for record in records {
        let mut resource = HitPropertySetTrackResource::new(key);
        resource
            .properties
            .extend([(LEGACY_FIELDS[6], 1), (LEGACY_FIELDS[8], 123), (LEGACY_FIELDS[11], 512)]);
        assert!(resource.read_legacy_record(&record));
        assert_eq!(resource.properties[&LEGACY_FIELDS[0]], 7);
        assert_eq!(resource.properties[&LEGACY_FIELDS[6]], 1);
        // An empty field 7 uses the nonspatial default despite its retained value.
        assert_eq!(resource.properties[&LEGACY_FIELDS[8]], 0x7DCD_ED2D);
        assert_eq!(resource.properties[&LEGACY_FIELDS[11]], 512);
        assert_eq!(resource.properties[&0xFF6F_9546], 42);
    }
}

#[test]
fn ss5_version_gates_overlay_and_unvalidated_text_tags() {
    let key = ResourceKey {
        kind: 0x0B8D_821A,
        group: 7,
        instance: 42,
    };
    for version in [0, 1, 2, u32::MAX] {
        let mut text = format!("unvalidated,{version},,");
        for field in 1..=27 {
            if (version == 0 && field == 6) || (version < 2 && field >= 22) {
                continue;
            }
            text.push_str(&format!("{field},"));
        }
        // §5.2 requires no ETKD marker or maximum-version check.
        let mut record = 0u32.to_le_bytes().to_vec();
        record.extend(text.as_bytes());
        let mut resource = HitPropertySetTrackResource::new(key);
        resource
            .properties
            .extend([(LEGACY_FIELDS[5], 600), (LEGACY_FIELDS[26], 2700)]);
        assert!(resource.read_legacy_record(&record));
        assert_eq!(
            resource.properties[&LEGACY_FIELDS[5]],
            if version == 0 { 600 } else { 6 }
        );
        assert_eq!(resource.properties[&LEGACY_FIELDS[8]], 9);
        assert_eq!(
            resource.properties[&LEGACY_FIELDS[26]],
            if version < 2 { 2700 } else { 27 }
        );
    }
}

#[derive(Default)]
struct FreshSpy {
    source: Option<SoundRef>,
    draw: u32,
    c_draws: usize,
    start_ok: bool,
    events: Vec<String>,
    seeds: Vec<u32>,
}
impl CellHost for FreshSpy {
    fn unregister_source_update_if_available(&mut self, _: &SoundRef) {
        self.events.push("source-unregister".into());
    }
    fn fade_to_zero(&mut self, _: &SoundRef, duration: u32) {
        self.events.push(format!("fade:{duration}"));
    }
    fn sound_system_available(&self) -> bool {
        true
    }
    fn sound_by_id(&mut self, id: u32) -> Option<SoundRef> {
        self.events.push(format!("id:{id:08x}"));
        self.source.clone()
    }
    fn sound_by_name(&mut self, name: &str) -> Option<SoundRef> {
        self.events.push(format!("name:{name}"));
        self.source.clone()
    }
    fn private_draw(&mut self) -> u32 {
        self.draw
    }
    fn crc24_filename(&mut self, _: &str) -> u32 {
        0x123456
    }
    fn pitch_ratio(&mut self, index: usize) -> Option<f64> {
        Some(if index == 0 { 1.0 } else { 2.0 })
    }
    fn register_source_update_if_available(&mut self, _: &SoundRef, period: u32) {
        self.events.push(format!("source-register:{period}"));
    }
    fn external_start(&mut self, cell: &CellData, id: u32, head: bool, looping: bool, fade: f32) -> bool {
        self.events
            .push(format!("external:{}:{id:08x}:{head}:{looping}:{fade}", cell.name));
        self.start_ok
    }
    fn external_position(&mut self, _: &CellData, _: [f32; 3]) {
        self.events.push("position".into());
    }
    fn external_distances(&mut self, _: &CellData, _: [f32; 2]) {
        self.events.push("distances".into());
    }
    fn external_volume(&mut self, _: &CellData, _: f32) {
        self.events.push("volume".into());
    }
    fn external_pitch(&mut self, _: &CellData, pitch: f32) {
        self.events.push(format!("pitch:{pitch}"));
    }
    fn external_stop(&mut self, _: &CellData, fade: f32) {
        self.events.push(format!("external-stop:{fade}"));
    }
    fn external_playing(&mut self, _: &CellData) -> bool {
        false
    }
    fn lifecycle(&mut self, event: FreshLifecycle) {
        self.events.push(format!("{event:?}"));
    }
}
impl FreshHost for FreshSpy {
    fn c_rand(&mut self) -> i32 {
        self.c_draws += 1;
        0
    }
    fn reseed_private(&mut self, seed: u32) {
        self.seeds.push(seed);
    }
    fn random_below(&mut self, upper: u32) -> u32 {
        self.draw % upper
    }
}
fn fresh_grid(rows: usize, columns: usize) -> FreshPlayer {
    let mut player = FreshPlayer::default();
    let score = ScoreData {
        cells: (0..rows * columns)
            .map(|id| CellData {
                id: id as u64,
                volume_word: 1024,
                class: 20,
                ..Default::default()
            })
            .collect(),
        volume_word: 1024,
        rows,
        columns,
        half: [1, 1],
        tempo: 120,
        beats_per_bar: 8,
        ..Default::default()
    };
    player.init_loaded_score(score).unwrap();
    player
}

#[test]
fn ss6_block_coordinate_selection_scores_and_separate_random_sources() {
    let mut player = fresh_grid(9, 9);
    player.block_mode = true;
    player.set_x_position(500);
    player.set_y_position(500);
    assert_eq!(player.coordinates, [4, 4]);
    assert_eq!((player.selectable_count, player.distinct_groups), (9, 9));
    let first = 3 * 9 + 3;
    assert_eq!(
        player.best_start(0, 0, || panic!("class20 must not draw C rand")),
        Some(first)
    );
    player.cells[first].cell.as_mut().unwrap().class = 1;
    player.selectable_count = 10;
    assert_eq!(player.start_score(first, 0, || 59), 1);
    assert_eq!(player.start_score(first, 0, || 60), 0);
    player.cells[first].cell.as_mut().unwrap().group = 7;
    player.selectable_count = 0;
    assert_eq!(
        player.start_score(first, 7, || panic!("matched group does not draw")),
        2
    );
    assert_eq!(player.selectable_count, 10);
    player.cells[first].cell.as_mut().unwrap().class = 0;
    assert_eq!(player.start_score(first, 0, || panic!()), 0);
    let mut host = FreshSpy::default();
    player.reseed(77, None, &mut host);
    assert_eq!(host.seeds, [77]);
    assert_eq!(host.c_draws, 0);
    assert_eq!(player.last_subtracted, Some(0));
    assert_eq!((player.beat_in_bar, player.next_beat, player.origin), (0, 1, 0));
    player.cells[0].state = 2;
    assert_eq!(player.cell_to_subtract(0, || panic!("outside chosen first")), Some(0));
}

#[test]
fn ss6_fade_overshoot_retarget_zero_step_and_clock_worked_cases() {
    let mut player = fresh_grid(1, 1);
    let mut host = FreshSpy::default();
    player.play(None);
    player.volume = 100;
    assert!(player.fade_volume(400, 145, true, None, &mut host));
    assert_eq!((player.fade.unwrap().step, player.fade.unwrap().period), (2, 20));
    for _ in 0..22 {
        player.on_fade(None, &mut host);
    }
    assert_eq!(player.volume, 144);
    assert!(player.fade.is_some());
    player.on_fade(None, &mut host);
    assert_eq!(player.volume, 146);
    assert!(player.fade.is_none());
    assert!(player.playing);
    player.volume = 100;
    player.fade_volume(400, 145, false, None, &mut host);
    player.fade_volume(1000, 40, false, None, &mut host);
    assert_eq!(player.fade.unwrap().period, 20);
    player.fade_volume(1, 110, false, None, &mut host);
    assert_eq!(player.volume, 110);
    assert!(player.fade.is_some());
    player.tempo = 0;
    assert_eq!(player.beat_due(5), -1);
    let next = player.next_beat;
    player.periodic_update(1000, &mut host);
    assert_eq!(player.next_beat, next);
    player.tempo = 120;
    player.origin = 0;
    player.next_beat = 1;
    player.beat_in_bar = 1;
    player.manual = true;
    player.periodic_update(500, &mut host);
    assert_eq!((player.next_beat, player.beat_in_bar), (2, 2));
    player.periodic_update(86_400_001, &mut host);
    assert_eq!((player.origin, player.next_beat, player.beat_in_bar), (8, 8, 8));
    assert!(host.events.contains(&"TimerRestart".into()));
    player.volume = 100;
    player.fade_volume(1, 110, true, None, &mut host);
    assert!(!player.playing);
}

struct ScheduleEvents(Arc<Mutex<Vec<String>>>);
impl Scheduler for ScheduleEvents {
    fn register(&mut self, period: u32) {
        self.0.lock().unwrap().push(format!("register:{period}"));
    }
    fn unregister(&mut self) {
        self.0.lock().unwrap().push("unregister".into());
    }
}
#[test]
fn ss6_stop_shutdown_and_repeated_init_lifetime_order() {
    let mut player = fresh_grid(1, 1);
    let source = Arc::new(Source::default());
    player.play(None);
    player.cells[0].source = Some(source);
    player.cells[0].state = 2;
    let mut host = FreshSpy::default();
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut schedule = ScheduleEvents(events.clone());
    player.stop(Some(&mut schedule), &mut host);
    assert_eq!(*events.lock().unwrap(), ["unregister"]);
    assert_eq!(&host.events[..3], ["FadeEnd", "TimerStop", "source-unregister"]);
    player.shutdown(None, &mut host);
    let score_delete = host.events.iter().position(|e| e == "ScoreDelete").unwrap();
    let player_delete = host.events.iter().position(|e| e == "PlayerDelete(0)").unwrap();
    assert!(score_delete < player_delete);
    assert!(player.score.is_none());
    assert!(player.cells.is_empty());
    assert!(!player.initialized);
    let mut player = fresh_grid(1, 1);
    player.init_loaded_score(ScoreData::default()).unwrap(); // initialized grids are not rebuilt
    assert_eq!(player.cells.len(), 1);
}

#[test]
fn ss7_guid_external_failure_countdown_pan_and_controller_worked_cases() {
    assert!(FreshCellPlayer::filename_has_guid("amb_1234ABCD.wav"));
    assert!(!FreshCellPlayer::filename_has_guid("1234-ABCD.wav"));
    let mut host = FreshSpy::default();
    let mut player = FreshCellPlayer::new(Arc::new(()));
    player.external_controller = true;
    player.set_cell(CellData {
        name: "amb_1234ABCD.wav".into(),
        volume_word: 1024,
        loop_length: 1,
        looping: true,
        ..Default::default()
    });
    assert!(!player.start(&mut host, 1024, 1024));
    assert!(player.is_busy());
    assert_eq!(player.cell.as_ref().unwrap().id, 0x1234_ABCD);
    assert!(!player.start(&mut host, 1024, 1024));
    assert_eq!(host.events.len(), 1);
    player.stop_flag = true;
    player.update(&mut host);
    assert_eq!((player.state, player.counter), (2, 0));
    player.update(&mut host);
    assert_eq!(player.state, 0);
    player.cell.as_mut().unwrap().pan = [768, 512];
    assert_eq!(player.auto_pan(&mut host, false, None), Some([50.0, 0.0, 0.0]));
    player.cell.as_mut().unwrap().pan = [512, 512];
    player.cell.as_mut().unwrap().random_pan = true;
    host.draw = 2;
    assert_eq!(player.auto_pan(&mut host, false, None), Some([-100.0, 0.0, 0.0]));
    player.state = 2;
    player.cell.as_mut().unwrap().looping = false;
    host.events.clear();
    player.update(&mut host);
    assert_eq!(player.state, 0);
    assert_eq!(host.events, ["external-stop:0"]);
}

#[test]
fn ss7_internal_start_ignored_play_failure_and_external_setter_order() {
    let source = Arc::new(Source::default());
    source.frequency.store(22051, Ordering::Relaxed);
    let mut host = FreshSpy {
        source: Some(source.clone()),
        start_ok: true,
        ..Default::default()
    };
    let mut player = FreshCellPlayer::new(Arc::new(()));
    player.set_cell(CellData {
        name: "amb_1234ABCD.wav".into(),
        volume_word: 1024,
        ..Default::default()
    });
    assert!(player.start(&mut host, 1024, 1024));
    assert!(player.is_playing());
    assert_eq!(host.events, ["id:1234abcd", "source-register:250"]);
    assert!(source.calls().iter().any(|e| e.starts_with("play:")));
    player.kill();
    player.external_controller = true;
    player.cell.as_mut().unwrap().pitch = [-1, -1];
    host.events.clear();
    assert!(player.start(&mut host, 1024, 1024));
    assert_eq!(&host.events[1..], ["position", "distances", "volume", "pitch:2"]);
    player.cell.as_mut().unwrap().fade_out = 5;
    player.stop(&mut host).unwrap();
    assert_eq!(player.state, 3);
    player.update(&mut host);
    assert_eq!(player.state, 0);
}
