// SPDX-License-Identifier: GPL-3.0-or-later
//! Private graphics-rule transport. Hosts supply measurements and I/O, never rule results.
//! Native acquisition is deliberately separate: see graphics-rules GR9.
use fc4_core::graphics_rules::{
    GraphicsRuleManager, InformationProvider, InitContext, LogContext, LogFile, LogStamp, Lowercase, MatchError,
    Measurements, Properties, PropertyValue, ScriptFiles,
};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

/// Borrowed UTF-8 bytes, copied before the next host call. Empty permits a null pointer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Bytes {
    data: *const u8,
    len: usize,
}
impl Bytes {
    unsafe fn bytes(&self) -> Option<&[u8]> {
        if self.len == 0 {
            Some(&[])
        } else if self.data.is_null() || self.len > isize::MAX as usize {
            None
        } else {
            Some(std::slice::from_raw_parts(self.data, self.len))
        }
    }
    unsafe fn text(&self) -> Option<String> {
        std::str::from_utf8(self.bytes()?).ok().map(str::to_owned)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Measurement {
    cpu_features: u32,
    cpu_mhz: u32,
    physical_bytes: u32,
    available_bytes: u32,
    screen: [u32; 3],
    video_bytes: u32,
    texture_bytes: u32,
    os_strings: [Bytes; 6],
    driver_record: [u8; 0x34],
    driver_description: Bytes,
    sound_names: [Bytes; 2],
    has_sound: u8,
    backing_store: u8,
    forced_fsaa: u8,
}

/// Required callbacks; every callback/context remains valid through owner destruction.
/// Property callbacks take the retained renderer, all others take context. No callback
/// may reenter this owner or unwind. File bytes remain live until the next callback.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Host {
    context: *mut c_void,
    retain: unsafe extern "C" fn(*mut c_void),
    release: unsafe extern "C" fn(*mut c_void),
    find: unsafe extern "C" fn(*mut c_void, u32, Bytes) -> i32,
    get: unsafe extern "C" fn(*mut c_void, u32, u32) -> u32,
    set: unsafe extern "C" fn(*mut c_void, u32, u32, u32),
    count: unsafe extern "C" fn(*mut c_void, u32) -> u32,
    name: unsafe extern "C" fn(*mut c_void, u32, u32) -> Bytes,
    read: unsafe extern "C" fn(*mut c_void, Bytes, *mut Bytes) -> i32,
    measure: unsafe extern "C" fn(*mut c_void, *mut Measurement) -> bool,
    log_open: unsafe extern "C" fn(*mut c_void, Bytes) -> bool,
    log_is_open: unsafe extern "C" fn(*mut c_void) -> bool,
    log_write: unsafe extern "C" fn(*mut c_void, Bytes) -> bool,
    log_close: unsafe extern "C" fn(*mut c_void) -> bool,
    lowercase: unsafe extern "C" fn(*mut c_void, u8) -> u8,
}
// Host promises callbacks are callable on the game's getter thread. State and
// retained renderer access are serialized by Owner and the core's property mutex.
unsafe impl Send for Host {}
unsafe impl Send for Renderer {}

fn bytes(value: &[u8]) -> Bytes {
    Bytes {
        data: value.as_ptr(),
        len: value.len(),
    }
}

struct Renderer {
    pointer: *mut c_void,
    host: Host,
}
impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe { (self.host.release)(self.pointer) }
    }
}
impl Renderer {
    fn find(&self, kind: u32, name: &str) -> Option<u32> {
        u32::try_from(unsafe { (self.host.find)(self.pointer, kind, bytes(name.as_bytes())) }).ok()
    }
    fn set(&mut self, kind: u32, index: u32, value: u32) -> bool {
        unsafe { (self.host.set)(self.pointer, kind, index, value) };
        true // Native setters are void; no success-dependent control flow.
    }
}
impl Properties for Renderer {
    fn find_bool(&self, name: &str) -> Option<u32> {
        self.find(0, name)
    }
    fn find_int(&self, name: &str) -> Option<u32> {
        self.find(1, name)
    }
    fn find_float(&self, name: &str) -> Option<u32> {
        self.find(2, name)
    }
    fn get_bool(&self, index: u32) -> Option<bool> {
        Some(unsafe { (self.host.get)(self.pointer, 0, index) } != 0)
    }
    fn set_bool(&mut self, index: u32, value: bool) -> bool {
        self.set(0, index, u32::from(value))
    }
    fn set_int(&mut self, index: u32, value: i32) -> bool {
        self.set(1, index, value as u32)
    }
    fn set_float(&mut self, index: u32, value: f32) -> bool {
        self.set(2, index, value.to_bits())
    }
    fn dump(&self) -> Vec<(Vec<u8>, PropertyValue)> {
        let mut result = Vec::new();
        for kind in 0..3 {
            for index in 0..unsafe { (self.host.count)(self.pointer, kind) } {
                let name = unsafe { (self.host.name)(self.pointer, kind, index) };
                let name = unsafe { name.bytes() }.expect("valid renderer name bytes").to_vec();
                let value = unsafe { (self.host.get)(self.pointer, kind, index) };
                result.push((
                    name,
                    match kind {
                        0 => PropertyValue::Bool(value != 0),
                        1 => PropertyValue::Int(value as i32),
                        _ => PropertyValue::Float(f32::from_bits(value)),
                    },
                ));
            }
        }
        result
    }
}
impl ScriptFiles for Host {
    fn read(&mut self, path: &str) -> Result<Option<String>, String> {
        let mut output = bytes(&[]);
        match unsafe { (self.read)(self.context, bytes(path.as_bytes()), &mut output) } {
            0 => Ok(None),
            1 => unsafe { output.text() }
                .map(Some)
                .ok_or_else(|| "Unsupported non-UTF-8 rule file".into()),
            _ => Err("Host file acquisition failed".into()),
        }
    }
}
impl LogFile for Host {
    fn open(&mut self, path: &str) -> bool {
        unsafe { (self.log_open)(self.context, bytes(path.as_bytes())) }
    }
    fn is_open(&self) -> bool {
        unsafe { (self.log_is_open)(self.context) }
    }
    fn write_bytes(&mut self, value: &[u8]) -> bool {
        unsafe { (self.log_write)(self.context, bytes(value)) }
    }
    fn close(&mut self) -> bool {
        unsafe { (self.log_close)(self.context) }
    }
}
impl Lowercase for Host {
    fn lowercase(&self, value: &[u8]) -> Result<Vec<u8>, MatchError> {
        Ok(value
            .iter()
            .map(|b| unsafe { (self.lowercase)(self.context, *b) })
            .collect())
    }
}

struct Provider {
    measurement: Measurements,
    forced_fsaa: bool,
}
impl InformationProvider for Provider {
    fn measure(&mut self) -> Measurements {
        self.measurement.clone()
    }
    fn probe_forced_fsaa(&mut self, _: &[u8; 0x34]) -> bool {
        self.forced_fsaa
    }
}
impl Host {
    unsafe fn provider(&self) -> Option<Provider> {
        let empty = bytes(&[]);
        let mut m = Measurement {
            cpu_features: 0,
            cpu_mhz: 0,
            physical_bytes: 0,
            available_bytes: 0,
            screen: [0; 3],
            video_bytes: 0,
            texture_bytes: 0,
            os_strings: [empty; 6],
            driver_record: [0; 0x34],
            driver_description: empty,
            sound_names: [empty; 2],
            has_sound: 0,
            backing_store: 0,
            forced_fsaa: 0,
        };
        if !(self.measure)(self.context, &mut m) {
            return None;
        }
        let strings = m.os_strings.iter().map(|s| s.text()).collect::<Option<Vec<_>>>()?;
        let sound_names = if m.has_sound != 0 {
            Some([m.sound_names[0].text()?, m.sound_names[1].text()?])
        } else {
            None
        };
        Some(Provider {
            measurement: Measurements {
                cpu_features: m.cpu_features,
                cpu_mhz: m.cpu_mhz,
                physical_bytes: m.physical_bytes,
                available_bytes: m.available_bytes,
                screen: m.screen,
                video_bytes: m.video_bytes,
                texture_bytes: m.texture_bytes,
                os_strings: strings.try_into().ok()?,
                driver_record: m.driver_record,
                driver_description: m.driver_description.text()?,
                sound_names,
                backing_store: m.backing_store != 0,
            },
            forced_fsaa: m.forced_fsaa != 0,
        })
    }
}

struct State {
    manager: GraphicsRuleManager<Renderer, Host>,
    renderer: Option<Arc<Mutex<Renderer>>>,
    host: Host,
}
pub struct Owner(Mutex<State>);

/// Private transport statuses: 0 success, 1 fallback installed after parse failure,
/// -1 invalid input/poisoned state, -2 unavailable provider, -3 unsupported core profile,
/// -100 contained panic. These are NOT native manager return values.
fn call(owner: *const Owner, action: impl FnOnce(&mut State) -> i32) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(owner) = (unsafe { owner.as_ref() }) else {
            return -1;
        };
        let Ok(mut state) = owner.0.lock() else {
            return -1;
        };
        action(&mut state)
    }))
    .unwrap_or(-100)
}

#[no_mangle]
pub unsafe extern "C" fn fc4_cfgrules_owner_create(host: *const Host) -> *mut Owner {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(host) = host.as_ref().copied() else {
            return std::ptr::null_mut();
        };
        Box::into_raw(Box::new(Owner(Mutex::new(State {
            manager: GraphicsRuleManager::new(host),
            renderer: None,
            host,
        }))))
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_cfgrules_owner_destroy(owner: *mut Owner) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !owner.is_null() {
            drop(Box::from_raw(owner));
        }
    }));
}

#[no_mangle]
pub unsafe extern "C" fn fc4_cfgrules_owner_init(
    owner: *const Owner,
    renderer: *mut c_void,
    data: Bytes,
    user: Bytes,
    prefs_first_time: bool,
) -> i32 {
    call(owner, |s| {
        let (Some(data), Some(user)) = (data.text(), user.text()) else {
            return -1;
        };
        if renderer.is_null() {
            return -1;
        }
        // SPEC-GAP: GR9 has no interface-only native probe/backing-store acquisition recipe.
        // Refuse unavailable acquisition before retaining/replacing properties or parsing.
        let Some(mut provider) = s.host.provider() else {
            return -2;
        };
        if !s
            .renderer
            .as_ref()
            .is_some_and(|p| p.lock().is_ok_and(|p| p.pointer == renderer))
        {
            (s.host.retain)(renderer);
            s.renderer = Some(Arc::new(Mutex::new(Renderer {
                pointer: renderer,
                host: s.host,
            })));
        }
        s.manager.init(
            s.renderer.as_ref().expect("renderer retained"),
            &mut provider,
            &mut s.host,
            &InitContext::from_directories(&data, &user, prefs_first_time),
        );
        if s.manager.last_init_error.is_some() {
            -3
        } else {
            i32::from(s.manager.last_parse_error.is_some())
        }
    })
}

#[repr(C)]
pub struct LogInput {
    app_directory: Bytes,
    write_log: Bytes,
    date: Bytes,
    hour: u8,
    minute: u8,
    preferences: *const u8, // null or twelve unsigned application bytes in SUMMARY order
}
impl LogInput {
    unsafe fn context(&self) -> Option<LogContext> {
        let preferences = if self.preferences.is_null() {
            None
        } else {
            Some(std::slice::from_raw_parts(self.preferences, 12).try_into().ok()?)
        };
        Some(LogContext {
            app_directory: self.app_directory.text()?,
            write_log: Some(self.write_log.text()?),
            stamp: LogStamp {
                date: self.date.text()?,
                hour: self.hour,
                minute: self.minute,
            },
            preferences,
        })
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_cfgrules_owner_evaluate(owner: *const Owner, input: *const LogInput) -> i32 {
    call(owner, |s| {
        let Some(context) = input.as_ref().and_then(|i| i.context()) else {
            return -1;
        };
        if s.manager.set_options_from_configuration(&context, &s.host).is_ok() {
            0
        } else {
            -3
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_cfgrules_owner_log(
    owner: *const Owner,
    path: Bytes,
    input: *const LogInput,
    opened: *mut bool,
) -> i32 {
    call(owner, |s| {
        let (Some(path), Some(input), Some(opened)) = (path.text(), input.as_ref(), opened.as_mut()) else {
            return -1;
        };
        *opened = false;
        let Some(context) = input.context() else {
            return -1;
        };
        let Some(mut provider) = s.host.provider() else {
            return -2;
        };
        match s
            .manager
            .log_configuration_to_file(&path, &context.stamp, &mut provider)
        {
            Ok(value) => {
                *opened = value;
                0
            }
            Err(_) => -3,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_cfgrules_owner_apply(owner: *const Owner, group: u32, option: u32) -> i32 {
    call(owner, |s| {
        if s.manager.apply_option_group_setting(group, option).is_ok() {
            0
        } else {
            -3
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_cfgrules_owner_auto(owner: *const Owner, group: u32, output: *mut u32) -> i32 {
    call(owner, |s| {
        let Some(output) = output.as_mut() else {
            return -1;
        };
        *output = s.manager.get_option_group_auto_setting(group);
        0
    })
}

/// Emit a copied diagnostic while the owner lock is held; the callback must not reenter.
#[no_mangle]
pub unsafe extern "C" fn fc4_cfgrules_owner_parse_error(
    owner: *const Owner,
    context: *mut c_void,
    emit: unsafe extern "C" fn(*mut c_void, Bytes),
) -> i32 {
    call(owner, |s| {
        if let Some(error) = &s.manager.last_parse_error {
            let message = error.to_string();
            emit(context, bytes(message.as_bytes()));
        }
        0
    })
}
