// SPDX-License-Identifier: GPL-3.0-or-later
//! Framework sound services (§7). Scheduling and devices remain foreign.
use super::directors::ForeignRef;
use super::{InterfaceView, NativeFault, Unsupported};
use crate::audio::snd_event::SndEventStd;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

pub trait EventHandler: Send + Sync {
    fn do_event(&self, system: &mut SndSys, event: &SndEventStd) -> bool;
}
pub type HandlerRef = Arc<dyn EventHandler>;
pub trait UpdateTarget: Send + Sync {
    fn update(&self, system: &mut SndSys) -> bool;
    /// Adapter QI for 0x206077D2; a successful query owns one reference.
    fn query_shutdown_sound(&self) -> Option<Arc<dyn ShutdownSound>> {
        None
    }
}
pub trait ShutdownSound: Send + Sync {
    fn stop(&self);
}
pub type TargetRef = Arc<dyn UpdateTarget>;
pub trait Renderer: Send + Sync {
    fn master_volume(&self, volume: i32);
    fn listener(&self) -> Option<Arc<dyn Listener>> {
        None
    }
    fn listener_ex(&self) -> Option<Arc<dyn ListenerEx>> {
        None
    }
}
pub trait Listener: Send + Sync {
    fn position(&self, position: [f32; 3]);
    fn orientation(&self, orientation: [f32; 6]);
}
pub trait ListenerEx: Send + Sync {
    fn velocity(&self, velocity: [f32; 3]);
}
pub type RendererRef = Arc<dyn Renderer>;
pub trait MessageService {
    fn post(&mut self, event: Arc<SndEventStd>);
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetClass {
    Ordinary,
    Slow,
}
/// Required native services/timers stay foreign; calls preserve §7.1.3 order.
pub trait SystemHost {
    fn data_directory(&mut self) -> Option<String>;
    fn message_handler(&mut self, message: u32, register: bool);
    fn create_queue(&mut self) -> Option<ForeignRef>;
    fn create_cache(&mut self, class: u32, iid: u32) -> Option<ForeignRef>;
    fn timer(&mut self, operation: TimerOperation) -> bool;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerOperation {
    StartElapsed,
    StartSlow,
    CreateCallback { period: u32 },
    InstallCallback,
    StartCallback,
    Priority(u32),
    StopCallback,
    StopElapsed,
    DeleteCallback,
}

pub struct SndSys {
    pub service_id: u32,
    pub data_directory: Option<String>,
    /// Not inferred from EnableUpdates; this is a separate control.
    pub enabled_flag: Option<bool>,
    updates_enabled: bool,
    master_volume: i32,
    handlers: Vec<HandlerRef>,
    dispatch_depth: usize,
    pending_handlers: Vec<HandlerRef>,
    removed_handlers: Vec<HandlerRef>,
    ordinary: Vec<TargetRef>,
    slow: Vec<TargetRef>,
    enumeration_depth: usize,
    removed_targets: Vec<TargetRef>,
    pending_ordinary: Vec<TargetRef>,
    pending_slow: Vec<TargetRef>,
    renderers: Vec<RendererRef>,
    slow_started: u32,
    running: bool,
    shutting_down: bool,
    test_mode: bool,
    extensions: BTreeMap<String, u32>,
    position: Option<[f32; 3]>,
    orientation: Option<[f32; 6]>,
    velocity: Option<[f32; 3]>,
    property_cache: Option<ForeignRef>,
    instance_cache: Option<ForeignRef>,
    message_queue: Option<ForeignRef>,
    mainline_releases: Vec<ForeignRef>,
}
impl Default for SndSys {
    fn default() -> Self {
        Self::new(None)
    }
}
impl SndSys {
    pub const CLSID: u32 = 0xE41C_E9FD;
    pub fn new(filesystem_directory: Option<String>) -> Self {
        Self {
            service_id: 0x23CE_CBA0,
            data_directory: filesystem_directory,
            enabled_flag: None,
            updates_enabled: true,
            master_volume: 0x400,
            handlers: Vec::new(),
            dispatch_depth: 0,
            pending_handlers: Vec::new(),
            removed_handlers: Vec::new(),
            ordinary: Vec::new(),
            slow: Vec::new(),
            enumeration_depth: 0,
            removed_targets: Vec::new(),
            pending_ordinary: Vec::new(),
            pending_slow: Vec::new(),
            renderers: Vec::new(),
            slow_started: 0,
            running: false,
            shutting_down: false,
            test_mode: false,
            extensions: BTreeMap::new(),
            // SPEC-GAP: constructor listener coordinates are not specified.
            position: None,
            orientation: None,
            velocity: None,
            property_cache: None,
            instance_cache: None,
            message_queue: None,
            mainline_releases: Vec::new(),
        }
    }
    pub fn register_handler(&mut self, handler: HandlerRef) {
        if self.dispatch_depth != 0 {
            self.pending_handlers.insert(0, handler);
        } else {
            self.unregister_handler(Arc::clone(&handler));
            self.handlers.insert(0, handler);
        }
    }
    pub fn unregister_handler(&mut self, handler: HandlerRef) {
        if self.dispatch_depth != 0 {
            self.removed_handlers.push(handler);
        } else if let Some(index) = self.handlers.iter().position(|item| Arc::ptr_eq(item, &handler)) {
            self.handlers.remove(index);
        }
    }
    pub fn do_event(&mut self, event: &SndEventStd) -> bool {
        self.dispatch_depth += 1;
        let mut consumed = false;
        // No mutations reach the active list until the outermost dispatch exits.
        for index in 0..self.handlers.len() {
            let handler = Arc::clone(&self.handlers[index]);
            if handler.do_event(self, event) {
                consumed = true;
                break;
            }
        }
        self.dispatch_depth -= 1;
        if self.dispatch_depth == 0 {
            for added in std::mem::take(&mut self.pending_handlers) {
                self.register_handler(added);
            }
            for removed in std::mem::take(&mut self.removed_handlers) {
                self.unregister_handler(removed);
            }
        }
        consumed
    }
    /// Foreign allocation may fail. Reuse the existing audio payload object.
    pub fn do_payload(
        &mut self,
        event_id: u32,
        data: [u32; 4],
        allocate: impl FnOnce() -> Option<SndEventStd>,
    ) -> bool {
        let Some(event) = Self::event_create(event_id, data, allocate) else {
            return false;
        };
        self.do_event(&event)
    }
    pub fn event_create(
        event_id: u32,
        data: [u32; 4],
        allocate: impl FnOnce() -> Option<SndEventStd>,
    ) -> Option<SndEventStd> {
        let mut event = allocate()?;
        event.payload = event_id;
        event.flag = 1;
        event.base.set_data1(data[0]);
        event.base.set_data2(data[1]);
        event.base.set_data3(data[2]);
        event.base.set_data4(data[3]);
        Some(event)
    }
    pub fn post_payload(
        &self,
        event_id: u32,
        data: [u32; 4],
        allocate: impl FnOnce() -> Option<SndEventStd>,
        messages: &mut dyn MessageService,
    ) -> bool {
        let Some(event) = Self::event_create(event_id, data, allocate) else {
            return false;
        };
        self.post_event(Arc::new(event), messages)
    }
    pub fn post_event(&self, event: Arc<SndEventStd>, messages: &mut dyn MessageService) -> bool {
        messages.post(event);
        true
    }
    pub fn disable_updates(&mut self) {
        self.updates_enabled = false;
    }
    pub fn enable_updates(&mut self) {
        self.updates_enabled = true;
    }
    pub fn register_target(&mut self, target: TargetRef, period: u32) -> Result<(), Unsupported> {
        let class = if period == 20 {
            TargetClass::Ordinary
        } else {
            TargetClass::Slow
        };
        self.register_classified_target(target, class);
        Ok(())
    }
    /// Confirmed list operation once a class has been established by an adapter.
    pub fn register_classified_target(&mut self, target: TargetRef, class: TargetClass) {
        if self.enumeration_depth != 0 {
            match class {
                TargetClass::Ordinary => self.pending_ordinary.push(target),
                TargetClass::Slow => self.pending_slow.push(target),
            }
        } else {
            self.unregister_target(Arc::clone(&target));
            self.target_list(class).push(target);
        }
    }
    fn target_list(&mut self, class: TargetClass) -> &mut Vec<TargetRef> {
        match class {
            TargetClass::Ordinary => &mut self.ordinary,
            TargetClass::Slow => &mut self.slow,
        }
    }
    fn remove_target(&mut self, target: &TargetRef) {
        for list in [&mut self.ordinary, &mut self.slow] {
            if let Some(index) = list.iter().position(|item| Arc::ptr_eq(item, target)) {
                list.remove(index);
            }
        }
    }
    pub fn unregister_target(&mut self, target: TargetRef) {
        if self.shutting_down {
            return;
        }
        if self.enumeration_depth != 0 {
            self.removed_targets.push(target);
        } else {
            self.remove_target(&target);
        }
    }
    /// Low-level enumeration; UpdateTargets supplies the elapsed-time gate.
    pub fn enumerate_targets(&mut self, class: TargetClass) {
        if !self.updates_enabled {
            return;
        }
        self.enumeration_depth += 1;
        let count = self.target_list(class).len();
        for index in 0..count {
            let target = Arc::clone(&self.target_list(class)[index]);
            if !target.update(self) {
                break;
            }
        }
        self.enumeration_depth -= 1;
        self.drain_targets();
    }
    fn drain_targets(&mut self) {
        if self.enumeration_depth != 0 {
            return;
        }
        for target in std::mem::take(&mut self.removed_targets) {
            self.unregister_target(target);
        }
        for target in std::mem::take(&mut self.pending_ordinary) {
            self.register_classified_target(target, TargetClass::Ordinary);
        }
        for target in std::mem::take(&mut self.pending_slow) {
            self.register_classified_target(target, TargetClass::Slow);
        }
    }
    pub fn update_targets(&mut self, now_ms: u32) {
        if !self.updates_enabled {
            return;
        }
        self.enumeration_depth += 1;
        self.enumerate_targets(TargetClass::Ordinary);
        if now_ms.wrapping_sub(self.slow_started) > 250 {
            self.slow_started = now_ms;
            self.enumerate_targets(TargetClass::Slow);
        }
        self.enumeration_depth -= 1;
        self.drain_targets();
    }
    pub fn register_renderer(&mut self, renderer: RendererRef) -> Result<(), Unsupported> {
        if self.renderers.iter().any(|item| Arc::ptr_eq(item, &renderer)) {
            return Ok(());
        }
        self.renderers.push(renderer);
        Ok(())
    }
    pub fn master_volume(&self) -> i32 {
        self.master_volume
    }
    pub fn set_master_volume(&mut self, volume: i32) {
        self.master_volume = volume;
        for renderer in &self.renderers {
            renderer.master_volume(self.master_volume);
        }
    }
    pub fn unregister_renderer(&mut self, renderer: &RendererRef) {
        self.renderers.retain(|item| !Arc::ptr_eq(item, renderer));
    }
    pub fn is_registered_renderer(&self, renderer: &RendererRef) -> bool {
        self.renderers.iter().any(|item| Arc::ptr_eq(item, renderer))
    }
    pub fn num_renderers(&self) -> usize {
        self.renderers.len()
    }
    fn extension(text: &str) -> String {
        let text = text.to_lowercase();
        text.trim().strip_prefix('.').unwrap_or(text.trim()).to_owned()
    }
    pub fn set_extension(&mut self, text: &str, class: u32) -> bool {
        let text = Self::extension(text);
        if text.is_empty() {
            return false;
        }
        self.extensions.insert(text, class);
        true
    }
    pub fn extension_class(&self, text: &str, output: &mut u32) -> bool {
        let Some(class) = self.extensions.get(&Self::extension(text)) else {
            return false;
        };
        *output = *class;
        true
    }
    pub fn extension_count(&self) -> usize {
        self.extensions.len()
    }
    pub fn extension_class_at(&self, index: usize, output: &mut u32) -> bool {
        let Some(class) = self.extensions.values().nth(index) else {
            return false;
        };
        *output = *class;
        true
    }
    pub fn extension_text_at(&self, index: usize, output: &mut String) -> bool {
        if !self.extensions.contains_key(output) {
            return false;
        }
        let Some((text, _)) = self.extensions.range(output.clone()..).nth(index) else {
            return false;
        };
        *output = text.clone();
        true
    }
    pub fn position(&self) -> Option<[f32; 3]> {
        self.position
    }
    pub fn orientation(&self) -> Option<[f32; 6]> {
        self.orientation
    }
    pub fn velocity(&self) -> Option<[f32; 3]> {
        self.velocity
    }
    pub fn set_position(&mut self, value: [f32; 3]) {
        self.position = Some(value);
        for renderer in &self.renderers {
            if let Some(listener) = renderer.listener() {
                listener.position(value);
            }
        }
    }
    pub fn set_orientation(&mut self, value: [f32; 6]) {
        self.orientation = Some(value);
        for renderer in &self.renderers {
            if let Some(listener) = renderer.listener() {
                listener.orientation(value);
            }
        }
    }
    pub fn set_velocity(&mut self, value: [f32; 3]) {
        self.velocity = Some(value);
        for renderer in &self.renderers {
            if let Some(listener) = renderer.listener_ex() {
                listener.velocity(value);
            }
        }
    }
    pub fn set_listener(&mut self, velocity: [f32; 3], orientation: [f32; 6], position: [f32; 3]) {
        self.set_velocity(velocity);
        self.set_orientation(orientation);
        self.set_position(position);
    }
    pub fn running(&self) -> bool {
        self.running
    }
    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }
    pub fn set_test_mode(&mut self, value: bool) {
        self.test_mode = value;
    }
    pub fn test_mode(&self) -> bool {
        self.test_mode
    }
    pub fn property_cache(&self) -> Option<&ForeignRef> {
        self.property_cache.as_ref()
    }
    pub fn instance_cache(&self) -> Option<&ForeignRef> {
        self.instance_cache.as_ref()
    }
    pub fn acquire_property_cache(&self, output: &mut Option<ForeignRef>) -> bool {
        *output = self.property_cache.clone();
        output.is_some()
    }
    pub fn acquire_instance_cache(&self, output: &mut Option<ForeignRef>) -> bool {
        *output = self.instance_cache.clone();
        output.is_some()
    }
    pub fn add_mainline_release(&mut self, reference: ForeignRef) {
        self.mainline_releases.push(reference);
    }
    pub fn init(&mut self, host: &mut dyn SystemHost, now_ms: u32) -> bool {
        if let Some(directory) = host.data_directory() {
            self.data_directory = Some(directory);
        }
        host.message_handler(0xDB53_383B, true);
        if self.enabled_flag == Some(true) && !self.running {
            self.message_queue = host.create_queue();
            self.property_cache = host.create_cache(0x9D4C_6B95, 0x5D4C_6B90);
            self.instance_cache = host.create_cache(0xBDF6_13B4, 0x3DF6_13A1);
            host.timer(TimerOperation::StartElapsed);
            host.timer(TimerOperation::StartSlow);
            self.slow_started = now_ms;
            host.timer(TimerOperation::CreateCallback { period: 20 });
            host.timer(TimerOperation::InstallCallback);
            let started = host.timer(TimerOperation::StartCallback);
            host.timer(TimerOperation::Priority(4));
            self.running = started;
        }
        true
    }
    pub fn shutdown(&mut self, host: &mut dyn SystemHost) -> bool {
        self.shutting_down = true;
        host.timer(TimerOperation::StopCallback);
        host.timer(TimerOperation::StopElapsed);
        host.message_handler(0xDB53_383B, false);
        host.timer(TimerOperation::StopCallback);
        host.timer(TimerOperation::DeleteCallback);
        self.clear_handlers();
        for target in self.ordinary.iter().chain(&self.slow) {
            if let Some(sound) = target.query_shutdown_sound() {
                sound.stop();
            }
        }
        self.ordinary.clear();
        self.slow.clear();
        // SPEC-GAP: §7.1.3 does not name the queued-list offsets; use their §7.1.1 replay order.
        self.removed_targets.clear();
        self.pending_ordinary.clear();
        self.pending_slow.clear();
        self.set_running(false);
        self.renderers.clear();
        self.clear_handlers();
        self.extensions.clear();
        self.mainline_releases.clear();
        self.property_cache = None;
        self.instance_cache = None;
        self.message_queue = None;
        true
    }
    fn clear_handlers(&mut self) {
        self.handlers.clear();
        self.pending_handlers.clear();
        self.removed_handlers.clear();
    }
}

pub trait ManagerServices {
    fn acquire_sound_system(&mut self) -> bool;
    fn register_manager_update(&mut self, interface_offset: u32, period: u32);
    fn retain_persist_manager(&mut self);
    fn sound_system_reference(&mut self) -> Option<Arc<dyn ManagerSystem>> {
        None
    }
    fn persist_manager_reference(&mut self) -> Option<Arc<dyn ManagerResources>> {
        None
    }
}
pub trait ManagerSystem: Send + Sync {
    fn num_renderers(&self) -> usize;
}
pub trait ManagerResources: Send + Sync {
    fn get_resource(&self, key: [u32; 3], iid: u32, output: &mut Option<ObjectRef>, a: u32, b: u32) -> bool;
}
/// Foreign QI owns its output reference; a failed QI may still change output.
pub trait ManagerObject: Send + Sync {
    fn query(&self, iid: u32, output: &mut Option<ObjectRef>) -> bool;
    fn play(&self, volume: i32, a: u32, b: u32) -> bool;
    fn is_playing(&self) -> bool;
}
pub type ObjectRef = Arc<dyn ManagerObject>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lookup {
    pub id: u32,
    pub a: u32,
    pub b: u32,
    pub iid: u32,
}
pub trait ManagerCreator: Send + Sync {
    /// Native QI eligibility check; temporary queried reference is released.
    fn supports(&self, iid: u32) -> bool;
    fn create(&self, manager: &mut SndMgr, args: Lookup, output: &mut Option<ObjectRef>) -> bool;
    fn query(&self, iid: u32, output: &mut Option<ObjectRef>) -> bool;
}
pub type CreatorRef = Arc<dyn ManagerCreator>;
pub struct SndMgr {
    pub(super) initialized: bool,
    pub sound_service_acquired: bool,
    /// Constructor stores zero; INI can replace it without fallback classes.
    pub default_2d: Option<u32>,
    pub default_3d: Option<u32>,
    pub(super) system_sounds: BTreeMap<u32, u32>,
    system_volume: i32,
    sound_system: Option<Arc<dyn ManagerSystem>>,
    resources: Option<Arc<dyn ManagerResources>>,
    sounds: BTreeMap<u32, Option<ObjectRef>>,
    sources: BTreeMap<u32, Option<ObjectRef>>,
    sound_creators: Vec<CreatorRef>,
    source_creators: BTreeMap<u32, CreatorRef>,
    autoplay: std::collections::VecDeque<ObjectRef>,
    ids: std::collections::BTreeSet<u32>,
}
impl Default for SndMgr {
    fn default() -> Self {
        Self {
            initialized: false,
            sound_service_acquired: false,
            default_2d: Some(0),
            default_3d: Some(0),
            system_sounds: BTreeMap::new(),
            system_volume: 0x400,
            sound_system: None,
            resources: None,
            sounds: BTreeMap::new(),
            sources: BTreeMap::new(),
            sound_creators: Vec::new(),
            source_creators: BTreeMap::new(),
            autoplay: std::collections::VecDeque::new(),
            ids: std::collections::BTreeSet::new(),
        }
    }
}
impl SndMgr {
    pub const CLSID: u32 = 0x7AF8_5B98;
    pub const SERVICE_ID: u32 = 0x1AF8_5BFB;
    pub const PRIORITY: u32 = 0x7A11E;
    pub fn query_manager_interface(owner: &Arc<Mutex<Self>>, iid: u32) -> Option<InterfaceView<Self>> {
        if iid == 0x3AF8_5BB7 {
            Some(InterfaceView {
                owner: Arc::clone(owner),
                offset: 0x1C,
            })
        } else {
            None
        } // Primary framework service QI belongs to the adapter.
    }
    pub fn init(&mut self, services: &mut dyn ManagerServices) -> bool {
        if self.initialized {
            return true;
        }
        self.initialized = true;
        if !services.acquire_sound_system() {
            return false;
        }
        self.sound_service_acquired = true;
        self.sound_system = services.sound_system_reference();
        services.register_manager_update(0x20, 250);
        services.retain_persist_manager();
        self.resources = services.persist_manager_reference();
        true
    }
    pub fn system_sound_id(&self, id: u32) -> u32 {
        self.system_sounds.get(&id).copied().unwrap_or(0)
    }
    pub fn set_system_sound_id(&mut self, id: u32, sound: u32) {
        self.system_sounds.insert(id, sound);
    }
    /// Uses the existing audio sound-ID transformation, never a duplicate cache.
    pub fn tagged_sound_id(&self, id: u32) -> u32 {
        crate::audio::sound_manager::tagged_sound_id(id)
    }
    pub fn set_default_2d(&mut self, class: u32) -> bool {
        self.default_2d = Some(class);
        true
    }
    pub fn set_default_3d(&mut self, class: u32) -> bool {
        self.default_3d = Some(class);
        true
    }
    pub fn system_volume(&self) -> i32 {
        self.system_volume
    }
    pub fn set_system_volume(&mut self, volume: i32) {
        self.system_volume = volume;
    }
    pub fn get_resource(&self, key: [u32; 3], iid: u32, output: &mut Option<ObjectRef>) -> bool {
        self.resources
            .as_ref()
            .is_some_and(|resource| resource.get_resource(key, iid, output, 0, 0))
    }
    pub fn register_sound(&mut self, id: u32, sound: Option<ObjectRef>) {
        self.sounds.entry(id).or_insert(sound);
    }
    pub fn register_source(&mut self, id: u32, source: Option<ObjectRef>) {
        self.sources.entry(id).or_insert(source);
    }
    pub fn unregister_sound(&mut self, id: u32) {
        self.sounds.remove(&id);
    }
    pub fn unregister_source(&mut self, id: u32) {
        self.sources.remove(&id);
    }
    pub fn unregister_sound_pointer(&mut self, sound: &ObjectRef) {
        // SPEC-GAP: pointer-overload match multiplicity is unspecified; remove matching IDs.
        self.sounds
            .retain(|_, item| !item.as_ref().is_some_and(|item| Arc::ptr_eq(item, sound)));
    }
    pub fn unregister_source_pointer(&mut self, source: &ObjectRef) {
        // SPEC-GAP: pointer-overload match multiplicity is unspecified; remove matching IDs.
        self.sources
            .retain(|_, item| !item.as_ref().is_some_and(|item| Arc::ptr_eq(item, source)));
    }
    pub fn register_sound_creator(&mut self, creator: Option<CreatorRef>) -> bool {
        let Some(creator) = creator.filter(|creator| creator.supports(0x3AF9_866F)) else {
            return false;
        };
        self.remove_sound_creator(&creator);
        self.sound_creators.insert(0, creator);
        true
    }
    pub fn remove_sound_creator(&mut self, creator: &CreatorRef) {
        if let Some(index) = self.sound_creators.iter().position(|item| Arc::ptr_eq(item, creator)) {
            self.sound_creators.remove(index);
        }
    }
    pub fn register_source_creator(&mut self, id: u32, creator: Option<CreatorRef>) -> bool {
        let Some(creator) = creator.filter(|creator| creator.supports(0x9B75_709E)) else {
            return false;
        };
        self.source_creators.insert(id, creator);
        true
    }
    pub fn remove_source_creator(&mut self, id: u32) {
        self.source_creators.remove(&id);
    }
    pub fn remove_source_creator_pointer(&mut self, creator: &CreatorRef) {
        // SPEC-GAP: pointer-overload match multiplicity is unspecified; remove matching IDs.
        self.source_creators.retain(|_, item| !Arc::ptr_eq(item, creator));
    }
    pub fn get_source_creator(&self, id: u32, iid: u32, output: &mut Option<ObjectRef>) -> bool {
        self.source_creators
            .get(&id)
            .is_some_and(|creator| creator.query(iid, output))
    }
    pub fn get_sound(&mut self, args: Lookup, output: &mut Option<ObjectRef>) -> Result<bool, NativeFault> {
        let system = self.sound_system.as_ref().ok_or(NativeFault::MissingSoundSystem)?;
        if system.num_renderers() == 0 {
            return Ok(false);
        }
        if let Some(sound) = self.sounds.get(&args.id) {
            return Ok(sound.as_ref().is_some_and(|sound| sound.query(args.iid, output)));
        }
        for creator in self.sound_creators.clone() {
            if creator.create(self, args, output) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn get_source(&mut self, args: Lookup, output: &mut Option<ObjectRef>) -> bool {
        if let Some(source) = self.sources.get(&args.id) {
            return source.as_ref().is_some_and(|source| source.query(args.iid, output));
        }
        for creator in self.source_creators.values().cloned().collect::<Vec<_>>() {
            if creator.create(self, args, output) {
                return true;
            }
        }
        false
    }
    pub fn add_autoplay(&mut self, sound: ObjectRef) -> bool {
        self.autoplay.push_back(sound);
        true
    }
    pub fn autoplay(&mut self, id: u32, volume: i32) -> Result<bool, NativeFault> {
        let sound = if let Some(sound) = self.sounds.get(&id) {
            sound.clone().ok_or(NativeFault::NullSound)?
        } else {
            let mut sound = None;
            if !self.get_sound(
                Lookup {
                    id,
                    a: 0,
                    b: 0,
                    iid: 0x2060_77D2,
                },
                &mut sound,
            )? {
                return Ok(false);
            }
            sound.ok_or(NativeFault::NullSound)?
        };
        self.add_autoplay(Arc::clone(&sound));
        // Release the temporary lookup reference before the native Play call.
        drop(sound);
        Ok(self.autoplay.back().expect("just appended").play(volume, 0, 1))
    }
    pub fn autoplay_default(&mut self, id: u32) -> Result<bool, NativeFault> {
        self.autoplay(id, self.system_volume)
    }
    pub fn system_autoplay(&mut self, index: u32, volume: i32) -> Result<bool, NativeFault> {
        self.autoplay(self.system_sound_id(index), volume)
    }
    pub fn update(&mut self) {
        if self.autoplay.front().is_some_and(|sound| !sound.is_playing()) {
            self.autoplay.pop_front();
        }
    }
    pub fn autoplay_count(&self) -> usize {
        self.autoplay.len()
    }
    pub fn shutdown(&mut self) -> bool {
        self.autoplay.clear();
        self.system_sounds.clear();
        self.sounds.clear();
        self.sound_creators.clear();
        self.source_creators.clear();
        self.ids.clear();
        self.initialized = false;
        true
    }
}
impl Drop for SndMgr {
    fn drop(&mut self) {
        self.shutdown();
        self.sources.clear();
        self.resources = None;
        self.sound_system = None;
    }
}
