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

struct Identity(Trace);
impl ConstructionIdentity for Identity {
    fn query(&self, _: u32, _: &mut Option<ForeignRef>) -> bool {
        note(&self.0, "QI");
        false
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
    fn convert(&mut self, _: &mut Vec<u8>, source: i32, destination: u32) -> bool {
        assert_eq!((source, destination), (-1, 8));
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
}
