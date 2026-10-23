// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenSetting { pub width: u32, pub height: u32, pub bits: u32 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenMode { pub setting: ScreenSetting, pub discriminator: u32 }
pub trait Driver: Send + Sync {
    fn id(&self) -> u32;
    fn modes(&self) -> Vec<ScreenMode>;
    fn enumerate_setup(&self);
}
/// Observable lifecycle calls; platform-specific discovery and rollback policy
/// belong to GZG3. The owner orders the settled successful path itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsCall {
    LoadDrivers, ConfigureScreen, CreateCache, Register, Subscribe(u32), AcquireCanvas,
    ReleaseCanvas, Unsubscribe(u32), RemoveCanvasHandler(u32), ShutdownCache,
    ReleaseCache, FreeDrivers, Unregister, ScreenDriverChanged, Tick,
    Listener(u32),
}
pub trait GraphicsHost: Send {
    fn framework_available(&self) -> bool;
    fn call(&mut self, call: GraphicsCall) -> bool;
    fn draw_service(&self) -> Option<Arc<dyn DrawService>>;
}
pub struct GraphicSystem {
    pub initialized: bool,
    pub drivers: Vec<Arc<dyn Driver>>,
    driver: Option<Arc<dyn Driver>>,
    pub driver_flags: u32,
    pub discriminator: u32,
    pub default_driver_class: u32,
    pub driver_ini: String,
    pub screen_object_present: bool,
    pub no_translucency: u8,
    pub translucency_word: u32,
    pub flush_pending: bool,
    pub screen_buffer_enabled: bool,
    pub cursor_emulation: bool,
    pub page_flipping: bool,
    pub listener_present: bool,
    pub construction_words: [u32;1],
    pub construction_bytes: [u8;5],
    host: Box<dyn GraphicsHost>,
}
impl GraphicSystem {
    pub fn new(host: Box<dyn GraphicsHost>) -> Self {
        // Preserve raw observed defaults without attaching invented meanings.
        Self { initialized: false, drivers: Vec::new(), driver: None, driver_flags: 0, discriminator: 0, default_driver_class: 0, driver_ini: String::new(), screen_object_present: false, no_translucency: 0, translucency_word: 0, flush_pending: false, screen_buffer_enabled: false, cursor_emulation: false, page_flipping: false, listener_present: false, construction_words: [1], construction_bytes: [0,1,0,1,1], host }
    }
    pub fn init(&mut self) -> bool {
        if self.initialized { return true; }
        if !self.host.framework_available() { return false; }
        for call in [GraphicsCall::LoadDrivers, GraphicsCall::ConfigureScreen, GraphicsCall::CreateCache, GraphicsCall::Register, GraphicsCall::Subscribe(0x8acaa95f), GraphicsCall::AcquireCanvas] {
            // Intermediate rollback is GZG3; host reports failure and owns any
            // provisional resources. No undocumented native rollback is claimed.
            if !self.host.call(call) { return false; }
        }
        self.initialized = true; true
    }
    pub fn shutdown(&mut self) -> bool {
        if self.initialized {
            self.initialized = false;
            for call in [GraphicsCall::ReleaseCanvas, GraphicsCall::Unsubscribe(0x8acaa95f), GraphicsCall::RemoveCanvasHandler(0x1a10630a), GraphicsCall::ShutdownCache, GraphicsCall::ReleaseCache, GraphicsCall::FreeDrivers, GraphicsCall::Unregister] { self.host.call(call); }
            self.driver = None; self.drivers.clear();
        } true
    }
    pub fn do_message(&self, kind: u32) -> bool {
        if kind == 0x8acaa95f { if let Some(service) = self.host.draw_service() { service.dispatch_slot9(); } } true
    }
    pub fn canvas_event(&mut self, iid: u32, subtype: u32) {
        if iid == 0xda1062fa && self.listener_present {
            let slot = match subtype { 0 => 0x108, 1 => 0x10c, 7 => 0x110, _ => return };
            self.host.call(GraphicsCall::Listener(slot));
        }
    }
    pub fn on_tick(&mut self) { self.host.call(GraphicsCall::Tick); }
    pub fn driver(&self) -> Option<&Arc<dyn Driver>> { self.driver.as_ref() }
    pub fn driver_count(&self) -> usize { self.drivers.len() }
    pub fn driver_by_index(&self, index: usize) -> Option<&Arc<dyn Driver>> { self.drivers.get(index) }
    pub fn driver_by_id(&self, id: u32) -> Option<&Arc<dyn Driver>> { self.drivers.iter().find(|d| d.id() == id) }
    pub fn set_driver(&mut self, driver: Option<&Arc<dyn Driver>>) {
        if match (&self.driver, driver) { (None,None) => true, (Some(a),Some(b)) => Arc::ptr_eq(a,b), _ => false } { return; }
        self.driver = None; self.driver_flags = 0; self.driver = driver.cloned();
        if let Some(d) = &self.driver { if d.modes().is_empty() { d.enumerate_setup(); } }
        if self.screen_object_present { self.host.call(GraphicsCall::ScreenDriverChanged); }
    }
    pub fn available(&self, driver_id: u32, setting: ScreenSetting) -> bool {
        // A nonzero ID must resolve, but enumeration deliberately uses active.
        if driver_id != 0 && self.driver_by_id(driver_id).is_none() { return false; }
        self.driver.as_ref().is_some_and(|d| d.modes().iter().any(|m| m.setting == setting && m.discriminator == self.discriminator))
    }
    pub fn downgrade(&self, driver_id: u32, setting: &mut ScreenSetting) -> bool {
        const LADDER: [(u32,u32);7] = [(2048,1536),(1600,1200),(1280,1024),(1024,768),(800,600),(640,480),(320,240)];
        loop {
            if setting.bits > 16 { setting.bits = 16; } else {
                let area = u64::from(setting.width)*u64::from(setting.height);
                let Some(&(w,h)) = LADDER.iter().find(|&&(w,h)| u64::from(w)*u64::from(h) < area) else { return false; };
                setting.width = w; setting.height = h;
            }
            if self.available(driver_id, *setting) { return true; }
        }
    }
    pub fn set_force_no_translucency(&mut self, byte: u8, word: u32) { self.no_translucency = byte; self.translucency_word = word; }
    pub fn translucency_enabled(&self) -> bool { self.no_translucency == 0 }
    pub fn lazy_flush(&mut self) { self.flush_pending = true; }
    pub fn disable_screen_buffer(&mut self) { self.screen_buffer_enabled = false; }
    pub fn set_ui_rendering_mode(&mut self) -> Result<()> { Err(Error::Unsupported("GZG3")) }
    pub fn render_buffer_to_screen(&mut self) -> Result<()> { Err(Error::Unsupported("GZG3")) }
    pub fn screen_pixel_format(&self) -> Result<u32> { Err(Error::Unsupported("GZG3")) }
    pub fn unidentified_operation(&mut self) -> Result<()> { Err(Error::Unsupported("GZG3")) }
}

/// Slot identities are separate even where names resemble one another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AlphaOperation {
    GenericBufferClip = 3, InitAlphaSystem = 4,
    AlphaBlt = 8, AlphaBltNoClip, AlphaBltConstAlpha, AlphaBltConstAlphaNoClip,
    AlphaBltConstSource, AlphaBltConstSourceNoClip, ShadeBltConstAlpha, ShadeBltConstAlphaNoClip,
    ShadowBlitConstAlpha, ShadowBlitConstAlphaNoClip, BrightBlitConstAlpha, BrightBlitConstAlphaNoClip,
    ShadeBlt, ShadeBltNoClip, ShadeBltStretch, ShadeBltStretchNoClip,
    ShadeBltWithAdditionalAlpha, ShadeBltWithAdditionalAlphaNoClip,
    ShadeBltStretchWithAdditionalAlpha, ShadeBltStretchWithAdditionalAlphaNoClip,
    AlphaBltSelfAlpha, AlphaBltSelfAlphaNoClip,
    AlphaBltHalfAlphaVariableSource1ConstSource2, AlphaBltHalfAlphaVariableSource1ConstSource2NoClip,
}
/// The GZG5 argument widths/return channels are unresolved. A typed envelope
/// preserves caller tokens without falsely declaring a native signature.
pub trait AlphaDelegate {
    type Arguments;
    type Output;
    fn invoke(&mut self, op: AlphaOperation, args: &mut Self::Arguments) -> Self::Output;
}
pub struct AlphaBlt<D> { pub delegate: D }
impl<D: AlphaDelegate> AlphaBlt<D> {
    pub fn alpha_level_count(&self) -> u32 { 32 }
    pub fn call(&mut self, op: AlphaOperation, args: &mut D::Arguments) -> D::Output { self.delegate.invoke(op, args) }
    pub fn surface_from_file(&mut self, path: &str, mask: bool, io: &mut dyn AlphaFileIo) -> Result<Option<Arc<dyn Buffer>>> {
        if !io.open(path) { return Ok(None); }
        // Row reversal, type-2 output and successful-lock Unlock are known;
        // their input quantization and malformed-file contract remain GZG5.
        // Do not emit incorrectly quantified pixels just because open succeeded.
        io.close(); let _ = mask;
        Err(Error::Unsupported("GZG5"))
    }
}
pub trait AlphaFileIo { fn open(&mut self, path: &str) -> bool; fn close(&mut self); }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanvasCall {
    Position { x:i32, y:i32, width:i32, height:i32, flags:u32 }, Title(Vec<u8>),
    Show(u32), Focus, Post([u32;3]), Event(u32), Input2Disabled(bool), Cleanup, ReleaseServices,
}
pub trait WindowHost: Send {
    fn call(&mut self, handle: Option<usize>, call: CanvasCall) -> bool;
    fn locale_ansi_code_page(&self) -> u32;
    fn services_available(&self) -> bool;
    fn realize(&mut self, setting: ScreenSetting, canvas_type: u32) -> Result<usize>;
    fn register_wheel_message(&mut self) -> Option<u32>;
}
pub struct CanvasW32 {
    pub handle: Option<usize>,
    pub initialized: bool,
    pub title: Vec<u8>,
    pub bounds: Rect,
    pub wheel_message: u32,
    pub canvas_type: u32,
    pub code_page: u32,
    pub coordinates: [u32;2],
    counters: [u32;2],
    host: Box<dyn WindowHost>,
}
impl CanvasW32 {
    pub fn new(host: Box<dyn WindowHost>) -> Self { Self { handle: None, initialized: false, title: Vec::new(), bounds: Rect::default(), wheel_message: u32::MAX, canvas_type: 0x1b, code_page: 0, coordinates: [0;2], counters: [0;2], host } }
    pub fn configure_bounds(&mut self, canvas_type: u32, width: i32, height: i32) { self.canvas_type = canvas_type; self.set_bounds(Rect(0,0,width,height)); }
    pub fn init(&mut self, setting: ScreenSetting) -> bool {
        let cp = self.host.locale_ansi_code_page(); self.code_page = if cp == 0 { 0x10 } else { cp };
        if !self.host.services_available() { return false; }
        self.configure_bounds(self.canvas_type, setting.width as i32, setting.height as i32);
        let Ok(handle) = self.host.realize(setting, self.canvas_type) else { return false; };
        self.handle = Some(handle); self.wheel_message = self.host.register_wheel_message().unwrap_or(u32::MAX);
        self.initialized = true; self.host.call(self.handle, CanvasCall::Event(0)); true
    }
    pub fn shutdown(&mut self) -> bool {
        if self.initialized {
            self.host.call(self.handle, CanvasCall::Event(1));
            self.host.call(self.handle, CanvasCall::Cleanup);
            self.host.call(self.handle, CanvasCall::ReleaseServices);
            // Equivalent to balancing all nested disables; only channel 2's
            // final transition is observable, so avoid a count-sized loop.
            if self.counters[1] > 0 { self.host.call(self.handle, CanvasCall::Input2Disabled(false)); }
            self.counters = [0;2]; self.initialized = false; self.handle = None;
        } true
    }
    pub fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
        if self.handle.is_some() { self.host.call(self.handle, CanvasCall::Position { x:bounds.0, y:bounds.1, width:bounds.width(), height:bounds.height(), flags:0x14 }); }
    }
    pub fn width(&self) -> i32 { self.bounds.width() }
    pub fn height(&self) -> i32 { self.bounds.height() }
    pub fn set_title(&mut self, bytes: &[u8]) {
        // Model the zero-terminated input, including an interior terminator.
        let n = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        self.title = bytes[..n].to_vec();
        if n > 0 { self.title.push(0); if self.handle.is_some() { self.host.call(self.handle, CanvasCall::Title(self.title.clone())); } }
    }
    pub fn show(&mut self, focus: bool) { self.host.call(self.handle, CanvasCall::Show(1)); if focus { self.host.call(self.handle, CanvasCall::Focus); } }
    pub fn post_package(&mut self, package: Option<(u32, &[u32])>) -> bool {
        match package { Some((0x8006, words)) if words.len() > 2 => self.host.call(self.handle, CanvasCall::Post([words[0],words[1],words[2]])), _ => false }
    }
    pub fn input_enabled(&self, selector: u32) -> bool { match selector { 1 => self.counters[0] == 0, 2 => self.counters[1] == 0, _ => false } }
    pub fn input_counters(&self) -> [u32;2] { self.counters }
    pub fn enable_input(&mut self, mask: u32, enable: bool) -> Result<()> {
        if !enable && (0..2).any(|i| mask & (1<<i) != 0 && self.counters[i] == u32::MAX) { return Err(Error::InvalidInput); }
        for i in 0..2 {
            if mask & (1<<i) == 0 { continue; }
            let was = self.counters[i];
            if enable { self.counters[i] = was.saturating_sub(1); } else { self.counters[i] += 1; }
            if i == 1 && (was == 0) != (self.counters[i] == 0) { self.host.call(self.handle, CanvasCall::Input2Disabled(self.counters[i] != 0)); }
        } Ok(())
    }
    /// A foreign message query provides a scoped, acquired interface payload;
    /// that scope owns Release, never a borrowed raw pointer in this owner.
    pub fn do_message(&mut self, kind: u32, queried: Option<(u32,u32,[u32;2])>) -> bool {
        if kind == 0xfa0c6390 { if let Some((0x1a0c615f,13,xy)) = queried { self.coordinates = xy; } } true
    }
    pub fn host_event_operation(&mut self) -> Result<()> { Err(Error::Unsupported("GZG4")) }
}
