// SPDX-License-Identifier: GPL-3.0-or-later
//! Framework sound services (§7). Scheduling and devices remain foreign.
use super::{InterfaceView, Unsupported};
use crate::audio::snd_event::SndEventStd;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

pub trait EventHandler: Send + Sync {
    fn do_event(&self, system: &mut SndSys, event: &SndEventStd) -> bool;
}
pub type HandlerRef = Arc<dyn EventHandler>;
pub trait UpdateTarget: Send + Sync {
    fn update(&self, system: &mut SndSys) -> bool;
}
pub type TargetRef = Arc<dyn UpdateTarget>;
pub trait Renderer: Send + Sync {
    fn master_volume(&self, volume: i32);
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
enum TargetChange {
    Add(TargetClass, TargetRef),
    Remove(TargetRef),
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
    target_changes: Vec<TargetChange>,
    renderers: Vec<RendererRef>,
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
            target_changes: Vec::new(),
            renderers: Vec::new(),
        }
    }
    pub fn register_handler(&mut self, handler: HandlerRef) {
        if self.dispatch_depth != 0 {
            self.pending_handlers.push(handler);
        } else {
            self.handlers.push(handler);
        }
    }
    pub fn unregister_handler(&mut self, handler: HandlerRef) {
        if self.dispatch_depth != 0 {
            self.removed_handlers.push(handler);
        } else {
            self.handlers.retain(|item| !Arc::ptr_eq(item, &handler));
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
            self.handlers.append(&mut self.pending_handlers);
            for removed in self.removed_handlers.drain(..) {
                // Literal matching-pointer removal; duplicate semantics SS12.
                self.handlers.retain(|item| !Arc::ptr_eq(item, &removed));
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
    /// Exact period classifier is unsettled; do not silently classify 250ms.
    pub fn register_target(&mut self, _target: TargetRef, _period: u32) -> Result<(), Unsupported> {
        Err(Unsupported("SS8"))
    }
    /// Confirmed list operation once a class has been established by an adapter.
    pub fn register_classified_target(&mut self, target: TargetRef, class: TargetClass) {
        if self.enumeration_depth != 0 {
            self.target_changes.push(TargetChange::Add(class, target));
        } else {
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
        self.ordinary.retain(|item| !Arc::ptr_eq(item, target));
        self.slow.retain(|item| !Arc::ptr_eq(item, target));
    }
    pub fn unregister_target(&mut self, target: TargetRef) {
        if self.enumeration_depth != 0 {
            self.target_changes.push(TargetChange::Remove(target));
        } else {
            self.remove_target(&target);
        }
    }
    /// Enumerate one class; selection of when the slow class is due is SS8.
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
        if self.enumeration_depth == 0 {
            // Literal request order; target drain ordering is questioned in SS12.
            let changes = std::mem::take(&mut self.target_changes);
            for change in changes {
                match change {
                    TargetChange::Add(class, target) => self.target_list(class).push(target),
                    TargetChange::Remove(target) => self.remove_target(&target),
                }
            }
        }
    }
    pub fn register_renderer(&mut self, renderer: RendererRef) -> Result<(), Unsupported> {
        if self.renderers.iter().any(|item| Arc::ptr_eq(item, &renderer)) {
            return Err(Unsupported("SS8"));
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
    pub fn unresolved_lifecycle_listener_cache_or_mapping(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported("SS8"))
    }
}

pub trait ManagerServices {
    fn acquire_sound_system(&mut self) -> bool;
    fn register_manager_update(&mut self, interface_offset: u32, period: u32);
    fn retain_persist_manager(&mut self);
}
#[derive(Default)]
pub struct SndMgr {
    pub(super) initialized: bool,
    pub sound_service_acquired: bool,
    /// Defaults supplied by INI; constructor defaults are unsettled SS9.
    pub default_2d: Option<u32>,
    pub default_3d: Option<u32>,
    pub(super) system_sounds: BTreeMap<u32, u32>,
}
impl SndMgr {
    pub const CLSID: u32 = 0x7AF8_5B98;
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
        services.register_manager_update(0x20, 250);
        services.retain_persist_manager();
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
    pub fn resolve_sound_or_shutdown(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported("SS9"))
    }
}
