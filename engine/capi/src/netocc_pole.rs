// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link transport for the power-pole occupant factory
//! (network-occupants.md §4). The plugin serializes access to one pole and
//! owns the host side of every reference: tokens are interface pointers and
//! each token the core keeps holds one reference, returned through the host's
//! `release` callback.
use crate::guard;
use crate::props::{fc4_props, props_ref};
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};
use fc4_core::automata::OccupantBase;
use fc4_core::network_occupants::pylon::{References, RenderStateArgs};
use fc4_core::network_occupants::power_pole::{
    AttachmentTables, PoleHost, PoleRecordHost, PoleStaticHost, PoleStatics, PowerPole,
};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{RefValue, ResKey};
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::Mutex;

/// The live fields the plugin reads and writes directly (52 bytes).
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct PoleState {
    removal: u16,
    highlight: u8,
    status: u8,
    flags: u8,
    rect: [u8; 4],
    distressed: u8,
    preview: u8,
    subscribed: u8,
    kind: u32,
    position: [f32; 3],
    cell: [i32; 2],
    /// The exemplar parent key: type, group, instance; `has_parent` 0 = none.
    parent: [u32; 3],
    has_parent: u32,
}

/// Host callbacks. Every token is an interface pointer of the host.
#[repr(C)]
pub struct PoleHostVt {
    user: *mut c_void,
    add_ref: Option<unsafe extern "C" fn(*mut c_void, u64)>,
    release: Option<unsafe extern "C" fn(*mut c_void, u64)>,
    insert_line: Option<unsafe extern "C" fn(*mut c_void, u64)>,
    remove_line: Option<unsafe extern "C" fn(*mut c_void, u64)>,
    /// `other.RemoveConnection(this, quiet)` on the other pole.
    reciprocal_remove: Option<unsafe extern "C" fn(*mut c_void, u64, u64, bool)>,
    post_message: Option<unsafe extern "C" fn(*mut c_void, u32, u32)>,
    subscription: Option<unsafe extern "C" fn(*mut c_void, u32, bool)>,
}

/// The control exemplar's values (§4.3), read once by the plugin. `present`
/// bit i says field i was found: bits 0-5 the six controls, 6 the widths,
/// 7 the demolition quote.
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct PoleTuning {
    present: u32,
    floats: [f32; 5],
    curve_points: u32,
    widths: [f32; 5],
    quote: i64,
}

/// Shared statics: the attachment tables the record reader overwrites and
/// the one-time StaticInit state (§4.3, §4.6). RefValues in here are only
/// Null or Host tokens, and every access holds the mutex.
struct Shared {
    tables: AttachmentTables,
    statics: PoleStatics,
    tuning: Option<PoleTuning>,
}
struct SendShared(Option<Shared>);
// SAFETY: see `Shared`; no Rc-backed RefValue::Object is ever stored.
unsafe impl Send for SendShared {}
static SHARED: Mutex<SendShared> = Mutex::new(SendShared(None));

fn with_shared<R>(f: impl FnOnce(&mut Shared) -> R) -> R {
    let mut g = SHARED.lock().unwrap_or_else(|e| e.into_inner());
    let shared = g.0.get_or_insert_with(|| Shared {
        tables: AttachmentTables::default(),
        statics: PoleStatics::new(0, 0),
        tuning: None,
    });
    f(shared)
}

fn token(r: &RefValue) -> u64 {
    match r {
        RefValue::Host { token, .. } => *token,
        _ => 0,
    }
}
fn host_ref(token: u64, class_id: u32) -> RefValue {
    if token == 0 {
        RefValue::Null
    } else {
        RefValue::Host { token, class_id }
    }
}

struct Host<'a> {
    vt: &'a PoleHostVt,
    tuning: Option<PoleTuning>,
}
impl References for Host<'_> {
    fn add_ref(&mut self, value: &RefValue) {
        if let (Some(f), t) = (self.vt.add_ref, token(value)) {
            if t != 0 {
                unsafe { f(self.vt.user, t) }
            }
        }
    }
    fn release(&mut self, value: &RefValue) {
        if let (Some(f), t) = (self.vt.release, token(value)) {
            if t != 0 {
                unsafe { f(self.vt.user, t) }
            }
        }
    }
}
impl PoleHost for Host<'_> {
    fn insert_line(&mut self, line: &RefValue, _: u32) {
        if let (Some(f), t) = (self.vt.insert_line, token(line)) {
            if t != 0 {
                unsafe { f(self.vt.user, t) }
            }
        }
    }
    fn remove_line(&mut self, line: &RefValue, _: bool, _: u32) {
        if let (Some(f), t) = (self.vt.remove_line, token(line)) {
            if t != 0 {
                unsafe { f(self.vt.user, t) }
            }
        }
    }
    fn reciprocal_remove(&mut self, other: &RefValue, this: &RefValue, quiet: bool) {
        if let Some(f) = self.vt.reciprocal_remove {
            unsafe { f(self.vt.user, token(other), token(this), quiet) }
        }
    }
    fn post_message(&mut self, message: u32, data: u32) {
        if let Some(f) = self.vt.post_message {
            unsafe { f(self.vt.user, message, data) }
        }
    }
    // SPEC-GAP: NO33/NO43 leave the owner cell walk open; no cells reported.
    fn lines_to(&mut self, _: &RefValue, _: &RefValue, _: &mut Vec<(i32, i32)>) -> bool {
        false
    }
    // SPEC-GAP: no effects host is wired for the plugin pole; the
    // "powerpolezap" effect is not shown (visual only, §4.7).
    fn create_effect(&mut self, _: &str) -> RefValue {
        RefValue::Null
    }
    fn place_effect(&mut self, _: &RefValue, _: [f32; 3], _: [f32; 9], _: f32) {}
    fn start_effect(&mut self, _: &RefValue) {}
    fn stop_effect(&mut self, _: &RefValue) {}
    fn finish_effect(&mut self, _: &RefValue) {}
    fn subscription(&mut self, message: u32, subscribe: bool) {
        if let Some(f) = self.vt.subscription {
            unsafe { f(self.vt.user, message, subscribe) }
        }
    }
    /// occupant-manager §13.1 slot 4: clear initialised, post Destroy.
    fn occupant_shutdown(&mut self, base: &mut OccupantBase) {
        if base.status & 1 != 0 {
            base.status &= !1;
            self.post_message(0x99EF_1141, 0);
        }
    }
}
impl PoleStaticHost for Host<'_> {
    // The resource and texture managers serve only drawing (§4.7), which
    // the plugin pole does not do; no reference is taken.
    fn acquire_resource_manager(&mut self) -> RefValue {
        RefValue::Null
    }
    fn acquire_texture_manager(&mut self, _: u32, _: u32) -> RefValue {
        RefValue::Null
    }
    fn register_shutdown(&mut self) {}
    fn control_exemplar(&mut self, _: ResKey) -> bool {
        self.tuning.is_some()
    }
    fn float_property(&self, property: u32) -> Option<f32> {
        let t = self.tuning?;
        use fc4_core::network_occupants::power_pole::*;
        let i = [PROP_E9A18D3F, PROP_49A1E8C4, PROP_89A18FC4, PROP_E9A1E8D4, PROP_49A195C0]
            .iter()
            .position(|&p| p == property)?;
        (t.present & (1 << i) != 0).then_some(t.floats[i])
    }
    fn uint_property(&self, property: u32) -> Option<u32> {
        let t = self.tuning?;
        (property == fc4_core::network_occupants::power_pole::PROP_89A1E8FB && t.present & (1 << 5) != 0)
            .then_some(t.curve_points)
    }
    fn width_property(&self, property: u32) -> Option<[f32; 5]> {
        let t = self.tuning?;
        (property == 0xE9AC_43E7 && t.present & (1 << 6) != 0).then_some(t.widths)
    }
    fn sint64_property(&self, property: u32) -> Option<i64> {
        let t = self.tuning?;
        (property == 0x0A2E_CED8 && t.present & (1 << 7) != 0).then_some(t.quote)
    }
    fn pack(&mut self, _: RenderStateArgs) -> u64 {
        0
    }
}
impl PoleRecordHost for Host<'_> {
    fn exemplar_model_instance(&mut self, base: &OccupantBase) -> u32 {
        // §4.6 selects the table from the exemplar resource key's instance.
        base.holder.parent_key().map_or(0, |key| key.instance)
    }
}

unsafe fn host<'a>(vt: *const PoleHostVt) -> Option<Host<'a>> {
    vt.as_ref().map(|vt| Host { vt, tuning: None })
}

/// Hands the control exemplar's values over before the first StaticInit.
#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_tuning(t: *const PoleTuning) {
    guard((), || {
        let t = t.as_ref().copied();
        with_shared(|s| {
            if s.tuning.is_none() {
                s.tuning = t;
            }
        });
    })
}

/// StaticInit (once per process, §4.3) through the given host.
fn static_init(host: &mut Host) {
    with_shared(|s| {
        host.tuning = s.tuning;
        s.statics.initialize(host);
    });
}

#[no_mangle]
pub extern "C" fn fc4_netpole_create() -> *mut PowerPole {
    guard(std::ptr::null_mut(), || {
        let mut p = PowerPole::fresh();
        // occupant-manager §13.1: a constructed occupant is not indexed.
        p.base.cell_rect = [0xFF, 0, 0, 0];
        Box::into_raw(Box::new(p))
    })
}

/// Releases every reference the pole holds, then frees it.
#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_destroy(p: *mut PowerPole, vt: *const PoleHostVt) {
    if p.is_null() {
        return;
    }
    guard((), || {
        let pole = Box::from_raw(p);
        if let Some(mut h) = host(vt) {
            for c in &pole.fields.connections {
                h.release(&c.pole);
                for line in &c.wire_points {
                    h.release(line);
                }
            }
            h.release(&pole.model);
            h.release(&pole.effect);
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_state(p: *mut PowerPole, s: *mut PoleState, put: bool) -> bool {
    guard(false, || {
        let (Some(p), Some(s)) = (p.as_mut(), s.as_mut()) else {
            return false;
        };
        if put {
            p.base.removal = s.removal;
            p.base.highlight = s.highlight;
            p.base.status = s.status;
            p.base.flags = s.flags;
            p.base.cell_rect = s.rect;
            p.set_preview(s.preview != 0);
            p.subscribed = s.subscribed != 0;
            p.wire_exemplar_index = s.kind;
            if p.fields.position != s.position {
                p.fields.position = s.position;
                p.stale = true;
            }
            p.fields.cell = (s.cell[0], s.cell[1]);
            let key = ResKey {
                type_id: s.parent[0],
                group: s.parent[1],
                instance: s.parent[2],
            };
            if s.has_parent == 0 {
                p.base.holder.parent = None;
            } else if p.base.holder.parent_key() != Some(key) {
                p.base.holder.parent = Some((key, Rc::new(PropertyHolder::new())));
            }
        } else {
            let key = p.base.holder.parent_key();
            *s = PoleState {
                removal: p.base.removal,
                highlight: p.base.highlight,
                status: p.base.status,
                flags: p.base.flags,
                rect: p.base.cell_rect,
                distressed: p.fields.distressed,
                preview: u8::from(p.is_preview()),
                subscribed: u8::from(p.subscribed),
                kind: p.wire_exemplar_index,
                position: p.fields.position,
                cell: [p.fields.cell.0, p.fields.cell.1],
                parent: key.map_or([0; 3], |k| [k.type_id, k.group, k.instance]),
                has_parent: u32::from(key.is_some()),
            };
        }
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_connection_count(p: *const PowerPole) -> u32 {
    guard(0, || p.as_ref().map_or(0, |p| p.connection_count() as u32))
}

/// Record `i`'s pole token, not counted (slot 7); 0 out of range.
#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_connected_pole(p: *const PowerPole, i: u32) -> u64 {
    guard(0, || {
        p.as_ref()
            .and_then(|p| p.connected_pole(i as usize))
            .map_or(0, |r| token(&r))
    })
}

/// Whether the pole holds a connection to `pole`.
#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_has_connection(p: *const PowerPole, pole: u64) -> bool {
    guard(false, || {
        p.as_ref().is_some_and(|p| {
            pole != 0 && p.fields.connections.iter().any(|c| token(&c.pole) == pole)
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_break_all(p: *mut PowerPole, this: u64, quiet: bool, vt: *const PoleHostVt) {
    guard((), || {
        if let (Some(p), Some(mut h)) = (p.as_mut(), host(vt)) {
            p.break_all_connections(&host_ref(this, 0x09C0_5C6A), quiet, &mut h);
        }
    })
}

/// The non-virtual `RemoveConnection(pole, quiet)` (§4.6).
#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_remove_connection(
    p: *mut PowerPole,
    pole: u64,
    quiet: bool,
    vt: *const PoleHostVt,
) {
    guard((), || {
        if let (Some(p), Some(mut h)) = (p.as_mut(), host(vt)) {
            if pole != 0 {
                p.remove_connection(&host_ref(pole, 0x09C0_5C6A), quiet, &mut h);
            }
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_insert_lines(p: *mut PowerPole, vt: *const PoleHostVt) {
    guard((), || {
        if let (Some(p), Some(mut h)) = (p.as_mut(), host(vt)) {
            p.insert_lines(&mut h);
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_set_distressed(p: *mut PowerPole, on: bool, vt: *const PoleHostVt) {
    guard((), || {
        if let (Some(p), Some(mut h)) = (p.as_mut(), host(vt)) {
            p.set_distressed(on, &mut h);
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_on_message(p: *mut PowerPole, message: u32, is_this: bool, vt: *const PoleHostVt) {
    guard((), || {
        if let (Some(p), Some(mut h)) = (p.as_mut(), host(vt)) {
            p.on_inserted_message(message, is_this, &mut h);
        }
    })
}

/// Occupant slot 4 (§4.3 `Shutdown`).
#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_shutdown(p: *mut PowerPole, this: u64, vt: *const PoleHostVt) {
    guard((), || {
        if let (Some(p), Some(mut h)) = (p.as_mut(), host(vt)) {
            p.shutdown(&host_ref(this, 0x09C0_5C6A), &mut h);
        }
    })
}

/// Main slot 15 after the occupant Init: StaticInit, the kind, the exemplar
/// key and the attachment table its instance selects (§4.3, §4.6).
/// SPEC-GAP: NO33 leaves the model load open; no model is made.
#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_init(p: *mut PowerPole, key: *const [u32; 3], kind: u32, vt: *const PoleHostVt) {
    guard((), || {
        let (Some(p), Some(mut h)) = (p.as_mut(), host(vt)) else {
            return;
        };
        static_init(&mut h);
        p.wire_exemplar_index = kind;
        if let Some(k) = key.as_ref() {
            let key = ResKey {
                type_id: k[0],
                group: k[1],
                instance: k[2],
            };
            if p.base.holder.parent_key() != Some(key) {
                p.base.holder.parent = Some((key, Rc::new(PropertyHolder::new())));
            }
            p.init_connection_points(key.instance);
        }
        for list in &mut p.geometry {
            list.clear();
        }
        p.bytes_54_55 = [0; 2];
        p.scale_64 = 1.0;
        if p.is_distressed() {
            p.set_distressed(true, &mut h);
        }
    })
}

/// Slot 10: the StaticInit quote (§4.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_demolition_cost(vt: *const PoleHostVt) -> i64 {
    guard(0, || {
        let Some(mut h) = host(vt) else { return 0 };
        static_init(&mut h);
        with_shared(|s| s.statics.demolition_cost())
    })
}

/// Occupant slot 17 (§4.2): the draw attributes only.
#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_set_highlight(p: *mut PowerPole, h: u8) {
    guard((), || {
        if let Some(p) = p.as_mut() {
            p.set_highlight(h);
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_write(p: *mut PowerPole, props: *const fc4_props, s: *const fc4_ostream) -> bool {
    guard(false, || {
        let (Some(p), Some(s)) = (p.as_mut(), s.as_ref()) else {
            return false;
        };
        if !ostream_ok(s) {
            return false;
        }
        if let Some(props) = props_ref(props) {
            p.base.holder.own = props.clone();
        }
        with_shared(|sh| p.write(&sh.tables, &mut COut(s)).unwrap_or(false))
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_read(p: *mut PowerPole, s: *const fc4_istream, vt: *const PoleHostVt) -> bool {
    guard(false, || {
        let (Some(p), Some(s), Some(mut h)) = (p.as_mut(), s.as_ref(), host(vt)) else {
            return false;
        };
        if !istream_ok(s) {
            return false;
        }
        let mut input = CIn {
            stream: s,
            decode: |_, _| None,
        };
        let mut g = SHARED.lock().unwrap_or_else(|e| e.into_inner());
        let shared = g.0.get_or_insert_with(|| Shared {
            tables: AttachmentTables::default(),
            statics: PoleStatics::new(0, 0),
            tuning: None,
        });
        h.tuning = shared.tuning;
        p.read_into(&mut shared.tables, &mut shared.statics, &mut input, &mut h)
            .unwrap_or(false)
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_netpole_props(
    p: *const PowerPole,
    cb: crate::neighbors::fc4_neighbor_property_callback,
    ctx: *mut c_void,
) {
    guard((), || {
        let (Some(p), Some(cb)) = (p.as_ref(), cb) else {
            return;
        };
        for id in p.base.holder.own.ids() {
            if let Some(v) = p.base.holder.own.get(id).and_then(crate::neighbors::connection_property) {
                cb(ctx, id, v.mem_type, v.is_array, v.bytes.as_ptr().cast(), v.count);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Log {
        refs: Vec<(u64, i32)>,
        messages: Vec<u32>,
        reciprocal: Vec<(u64, u64, bool)>,
    }
    unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
        (*(u as *const RefCell<Log>)).borrow_mut().refs.push((t, 1));
    }
    unsafe extern "C" fn release(u: *mut c_void, t: u64) {
        (*(u as *const RefCell<Log>)).borrow_mut().refs.push((t, -1));
    }
    unsafe extern "C" fn post(u: *mut c_void, m: u32, _: u32) {
        (*(u as *const RefCell<Log>)).borrow_mut().messages.push(m);
    }
    unsafe extern "C" fn reciprocal(u: *mut c_void, o: u64, t: u64, q: bool) {
        (*(u as *const RefCell<Log>)).borrow_mut().reciprocal.push((o, t, q));
    }
    fn vt(log: &RefCell<Log>) -> PoleHostVt {
        PoleHostVt {
            user: log as *const _ as *mut c_void,
            add_ref: Some(add_ref),
            release: Some(release),
            insert_line: None,
            remove_line: None,
            reciprocal_remove: Some(reciprocal),
            post_message: Some(post),
            subscription: None,
        }
    }

    #[test]
    fn state_layout_fresh_defaults_and_parent_key_round_trip() {
        assert_eq!(std::mem::size_of::<PoleState>(), 52);
        unsafe {
            let p = fc4_netpole_create();
            let mut s = PoleState::default();
            assert!(fc4_netpole_state(p, &mut s, false));
            assert_eq!(s.rect[0], 0xFF);
            assert_eq!(s.flags, 4);
            assert_eq!(s.has_parent, 0);
            assert_eq!(fc4_netpole_connection_count(p), 0);
            s.parent = [0x6534_284A, 1, 0x0000_0300];
            s.has_parent = 1;
            s.preview = 1;
            assert!(fc4_netpole_state(p, &mut s, true));
            let mut t = PoleState::default();
            assert!(fc4_netpole_state(p, &mut t, false));
            assert_eq!(t.parent, s.parent);
            assert_eq!(t.preview, 1);
            let log = RefCell::new(Log::default());
            let v = vt(&log);
            fc4_netpole_destroy(p, &v);
        }
    }

    #[test]
    fn init_selects_table_and_connections_release_and_reciprocate() {
        unsafe {
            let log = RefCell::new(Log::default());
            let v = vt(&log);
            let p = fc4_netpole_create();
            // Instance nibble 3 selects table B (§4.6).
            fc4_netpole_init(p, &[1, 2, 0x0000_0300], 7, &v);
            assert_eq!((*p).wire_exemplar_index, 7);
            assert!((*p).connection_point_table.is_some());
            (*p).fields.connections.push(fc4_core::network_occupants::power_pole::PoleConnection {
                pole: host_ref(0x50, 0x09C0_5C6A),
                wire_points: vec![host_ref(0x60, 0xC9C0_5C5D)],
                flags: [1, 1],
                ..Default::default()
            });
            assert!(fc4_netpole_has_connection(p, 0x50));
            assert_eq!(fc4_netpole_connected_pole(p, 0), 0x50);
            assert_eq!(fc4_netpole_connected_pole(p, 1), 0);
            fc4_netpole_break_all(p, 0x40, false, &v);
            // The records stay with no pole (quirk), the refs are released.
            assert_eq!(fc4_netpole_connection_count(p), 1);
            assert_eq!(log.borrow().reciprocal, vec![(0x50, 0x40, false)]);
            assert!(log.borrow().refs.contains(&(0x50, -1)));
            assert!(log.borrow().refs.contains(&(0x60, -1)));
            assert_eq!(log.borrow().messages, vec![0x99EF_1145]);
            fc4_netpole_destroy(p, &v);
        }
    }

    #[test]
    fn remove_connection_erases_the_record_and_destroy_releases_held_refs() {
        unsafe {
            let log = RefCell::new(Log::default());
            let v = vt(&log);
            let p = fc4_netpole_create();
            for t in [0x51u64, 0x52] {
                (*p).fields.connections.push(fc4_core::network_occupants::power_pole::PoleConnection {
                    pole: host_ref(t, 0x09C0_5C6A),
                    ..Default::default()
                });
            }
            fc4_netpole_remove_connection(p, 0x51, true, &v);
            assert_eq!(fc4_netpole_connection_count(p), 1);
            assert!(log.borrow().messages.is_empty());
            fc4_netpole_destroy(p, &v);
            let refs = &log.borrow().refs;
            assert_eq!(refs.iter().filter(|r| r.1 == -1).count(), 2);
        }
    }

    #[test]
    fn shutdown_posts_destroy_only_when_initialised() {
        unsafe {
            let log = RefCell::new(Log::default());
            let v = vt(&log);
            let p = fc4_netpole_create();
            fc4_netpole_shutdown(p, 0x40, &v);
            assert!(log.borrow().messages.is_empty());
            (*p).base.status |= 1;
            fc4_netpole_shutdown(p, 0x40, &v);
            assert_eq!(log.borrow().messages, vec![0x99EF_1141]);
            assert_eq!((*p).base.status & 1, 0);
            fc4_netpole_destroy(p, &v);
        }
    }
}
