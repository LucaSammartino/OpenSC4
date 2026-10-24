// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! C ABI tests for the flora simulator (`fc4_flora_*`, `fc4_flora_registry_*`)
//! and its census shadow (`fc4_flora_shadow_*`). The behaviour itself is
//! covered by the core tests and the conformance vectors `fo16_*`.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::flora::{msg, occupant_type, prop, BURNT_TREE, FLORA_EXEMPLAR_TYPE};
use fc4_core::property::PropertyHolder;
use fc4_core::variant::{mem, Elem, Variant};

use crate::budget::fc4_token_callback;
use crate::flora::*;
use crate::flora_shadow::*;
use crate::hostgrid::fc4_cell_rect;
use crate::props::fc4_props;
use crate::test_streams::{istream, ostream, In, Out};

type U = *mut c_void;
/// A segment record's key: segment, type, group, instance.
type SegKey = (u64, u32, u32, u32);

// ---- empty tables (other tests use them) ------------------------------------------------------------

/// A host table with every callback NULL.
pub(crate) fn empty_host() -> fc4_flora_host {
    // SAFETY: every field is an Option of a function pointer, a pointer or
    // an integer: all-zero is valid (None / NULL / 0).
    let mut h: fc4_flora_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_flora_host>() as u32;
    h
}

/// A registry host table with every callback NULL.
pub(crate) fn empty_registry_host() -> fc4_flora_registry_host {
    fc4_flora_registry_host {
        struct_size: std::mem::size_of::<fc4_flora_registry_host>() as u32,
        user: std::ptr::null_mut(),
        create_block: None,
        set_block_seed: None,
        release_exemplar: None,
    }
}

// ---- a scripted game --------------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
struct Occ {
    t: u32,
    pos: [f32; 3],
    flora: fc4_flora_occupant,
    in_manager: bool,
    record: u64,
}

#[derive(Default)]
struct World {
    occs: RefCell<BTreeMap<u64, Occ>>,
    next: Cell<u64>,
    refs: RefCell<BTreeMap<u64, i64>>,
    date: Cell<u32>,
    /// (type, ExemplarType, exemplar token) in enumeration order.
    exemplars: RefCell<Vec<(u32, u32, u64)>>,
    blocks: RefCell<Vec<(u64, fc4_flora_record, u64)>>,
    seeds: RefCell<Vec<(u64, u32)>>,
    released_exemplars: RefCell<Vec<u64>>,
    posts: RefCell<Vec<(u32, u32)>>,
    log: RefCell<Vec<String>>,
    /// Run during `insert_occupant` (re-entrancy).
    on_insert: RefCell<Option<Box<dyn Fn()>>>,
    ground: Cell<f32>,
    moisture: Cell<u32>,
    /// Obliteration segments: (segment, type, group, instance) -> bytes.
    segments: RefCell<BTreeMap<SegKey, Vec<u8>>>,
    streams: RefCell<BTreeMap<u64, SegStream>>,
    next_stream: Cell<u64>,
    /// The new-city specification's obliterated segment (0 = none).
    obliterated: Cell<u64>,
}

/// One open record of an obliteration segment.
#[derive(Clone, Debug, Default)]
struct SegStream {
    key: SegKey,
    write: bool,
    bytes: Vec<u8>,
    pos: usize,
    error: bool,
}

fn w(u: U) -> &'static World {
    // SAFETY: the tests pass a live World as the user pointer.
    unsafe { &*(u as *const World) }
}

fn flora_props(t: u32, et: u32) -> PropertyHolder {
    let mut h = PropertyHolder::new();
    h.add(prop::EXEMPLAR_TYPE, Variant::Scalar(Elem::U32(et)));
    h.add(prop::TYPE_ID, Variant::Scalar(Elem::U32(t)));
    h.add(prop::TABLE, Variant::array(mem::FLOAT32, vec![Elem::F32(0.6); 256]));
    h
}

fn props_ptr(h: PropertyHolder) -> *mut fc4_props {
    Box::into_raw(Box::new(h)) as *mut fc4_props
}

impl World {
    fn new() -> Rc<World> {
        let w = World::default();
        w.next.set(1000);
        w.date.set(5000);
        w.ground.set(260.0);
        w.moisture.set(8);
        Rc::new(w)
    }
    fn user(&self) -> U {
        self as *const World as U
    }
    fn add_flora(&self, id: u64, x: f32, z: f32, t: u32, state: i8, birth: u32) {
        self.occs.borrow_mut().insert(
            id,
            Occ {
                t: occupant_type::FLORA,
                pos: [x, 260.0, z],
                flora: fc4_flora_occupant {
                    flora_type: t,
                    birth_date: birth,
                    last_seeding_date: birth,
                    state,
                    ..Default::default()
                },
                in_manager: true,
                record: 0,
            },
        );
        self.refs.borrow_mut().insert(id, 1);
    }
    fn occ(&self, id: u64) -> Occ {
        self.occs.borrow().get(&id).cloned().unwrap_or_default()
    }
    fn refs_of(&self, id: u64) -> i64 {
        self.refs.borrow().get(&id).copied().unwrap_or(0)
    }
}

// registry host
unsafe extern "C" fn create_block(u: U, r: *const fc4_flora_record, ex: u64) -> u64 {
    let w = w(u);
    let id = 0x8000_0000 + w.blocks.borrow().len() as u64 * 0x200;
    w.blocks.borrow_mut().push((id, *r, ex));
    id
}
unsafe extern "C" fn set_block_seed(u: U, b: u64, s: u32) {
    w(u).seeds.borrow_mut().push((b, s));
}
unsafe extern "C" fn release_exemplar(u: U, t: u64) {
    w(u).released_exemplars.borrow_mut().push(t);
}

fn registry_host(w: &World) -> fc4_flora_registry_host {
    fc4_flora_registry_host {
        struct_size: std::mem::size_of::<fc4_flora_registry_host>() as u32,
        user: w.user(),
        create_block: Some(create_block),
        set_block_seed: Some(set_block_seed),
        release_exemplar: Some(release_exemplar),
    }
}

// simulator host
unsafe extern "C" fn yes(_: U) -> bool {
    true
}
unsafe extern "C" fn no(_: U) -> bool {
    false
}
unsafe extern "C" fn city_size(_: U, x: *mut f32, z: *mut f32) {
    *x = 1024.0;
    *z = 1024.0;
}
unsafe extern "C" fn city_cells(_: U, x: *mut i32, z: *mut i32) {
    *x = 64;
    *z = 64;
}
unsafe extern "C" fn date(u: U) -> u32 {
    w(u).date.get()
}
unsafe extern "C" fn exemplars(u: U, visit: fc4_flora_exemplar_visitor, ctx: *mut c_void) {
    let list = w(u).exemplars.borrow().clone();
    for (i, (t, et, token)) in list.into_iter().enumerate() {
        if let Some(f) = visit {
            f(ctx, 0x6534_284A, 1, i as u32, props_ptr(flora_props(t, et)), token);
        }
    }
}
unsafe extern "C" fn noop(_: U) {}
unsafe extern "C" fn noop_occ(_: U, _: u64) {}
unsafe extern "C" fn subscribe(u: U, ids: *const u32, n: usize) {
    w(u).log.borrow_mut().push(format!("subscribe {n} first {:#x}", *ids));
}
unsafe extern "C" fn unsubscribe(u: U, _: *const u32, n: usize) {
    w(u).log.borrow_mut().push(format!("unsubscribe {n}"));
}
unsafe extern "C" fn register_command(u: U, id: u32, p: *const u8, n: usize) {
    let name = String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned();
    w(u).log.borrow_mut().push(format!("command {id:#x} {name}"));
}
unsafe extern "C" fn unregister_id(u: U, id: u32) {
    w(u).log.borrow_mut().push(format!("unregister {id:#x}"));
}
unsafe extern "C" fn add_agent(u: U, level: u32, p: *const u8, n: usize, flags: u32) {
    let name = String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned();
    w(u).log.borrow_mut().push(format!("agent {level} {name} {flags}"));
}
unsafe extern "C" fn register_cheat(u: U, id: u32, p: *const u8, n: usize) {
    let name = String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned();
    w(u).log.borrow_mut().push(format!("cheat {id:#x} {name}"));
}
unsafe extern "C" fn post(u: U, t: u32, d2: u32) {
    w(u).posts.borrow_mut().push((t, d2));
}
unsafe extern "C" fn altitude(u: U, _: f32, _: f32) -> f32 {
    w(u).ground.get()
}
unsafe extern "C" fn sea(_: U) -> f32 {
    250.0
}
unsafe extern "C" fn normal(_: U, _: f32, _: f32, n: *mut f32) {
    *n.add(1) = 1.0;
}
unsafe extern "C" fn lot_alt(u: U, _: f32, _: f32, on: *mut bool) -> f32 {
    *on = false;
    w(u).ground.get()
}
unsafe extern "C" fn moisture(u: U, _: f32, _: f32) -> u32 {
    w(u).moisture.get()
}
unsafe extern "C" fn occ_type(u: U, o: u64) -> u32 {
    w(u).occ(o).t
}
unsafe extern "C" fn has_prop(_: U, _: u64, _: u32) -> bool {
    false
}
unsafe extern "C" fn position(u: U, o: u64, p: *mut f32) {
    let v = w(u).occ(o).pos;
    for (i, x) in v.iter().enumerate() {
        *p.add(i) = *x;
    }
}
unsafe extern "C" fn set_position(u: U, o: u64, p: *const f32) {
    if let Some(x) = w(u).occs.borrow_mut().get_mut(&o) {
        x.pos = [*p, *p.add(1), *p.add(2)];
    }
}
unsafe extern "C" fn bbox(u: U, o: u64, mn: *mut f32, mx: *mut f32) {
    let p = w(u).occ(o).pos;
    let (a, b) = ([p[0] - 1.0, p[1], p[2] - 1.0], [p[0] + 1.0, p[1] + 3.0, p[2] + 1.0]);
    for i in 0..3 {
        *mn.add(i) = a[i];
        *mx.add(i) = b[i];
    }
}
unsafe extern "C" fn in_manager(u: U, o: u64) -> bool {
    w(u).occ(o).in_manager
}
unsafe extern "C" fn set_in_manager(u: U, o: u64, on: bool) {
    if let Some(x) = w(u).occs.borrow_mut().get_mut(&o) {
        x.in_manager = on;
    }
}
unsafe extern "C" fn net_flags(_: U, _: u64, _: *mut u32) -> bool {
    false
}
unsafe extern "C" fn flora_occ(u: U, o: u64, out: *mut fc4_flora_occupant) -> bool {
    match w(u).occs.borrow().get(&o) {
        Some(x) if x.t == occupant_type::FLORA => {
            *out = x.flora;
            true
        }
        _ => false,
    }
}
fn with_flora(u: U, o: u64, f: impl FnOnce(&mut Occ)) {
    if let Some(x) = w(u).occs.borrow_mut().get_mut(&o) {
        f(x);
    }
}
unsafe extern "C" fn set_type(u: U, o: u64, v: u32) {
    with_flora(u, o, |x| x.flora.flora_type = v)
}
unsafe extern "C" fn set_birth(u: U, o: u64, v: u32) {
    with_flora(u, o, |x| x.flora.birth_date = v)
}
unsafe extern "C" fn set_last(u: U, o: u64, v: u32) {
    with_flora(u, o, |x| x.flora.last_seeding_date = v)
}
unsafe extern "C" fn set_state(u: U, o: u64, v: i8) {
    with_flora(u, o, |x| x.flora.state = v)
}
unsafe extern "C" fn set_orientation(u: U, o: u64, v: u32) {
    with_flora(u, o, |x| x.flora.orientation = v as u8)
}
unsafe extern "C" fn set_record(u: U, o: u64, b: u64) {
    with_flora(u, o, |x| x.record = b)
}
unsafe extern "C" fn post_change(u: U, o: u64) {
    w(u).log.borrow_mut().push(format!("change {o}"));
}
unsafe extern "C" fn create_occ(u: U) -> u64 {
    let w = w(u);
    let id = w.next.get();
    w.next.set(id + 1);
    w.occs.borrow_mut().insert(
        id,
        Occ {
            t: occupant_type::FLORA,
            ..Default::default()
        },
    );
    w.refs.borrow_mut().insert(id, 1);
    id
}
unsafe extern "C" fn add_ref(u: U, o: u64) {
    *w(u).refs.borrow_mut().entry(o).or_insert(0) += 1;
}
unsafe extern "C" fn release(u: U, o: u64) {
    *w(u).refs.borrow_mut().entry(o).or_insert(0) -= 1;
}
unsafe extern "C" fn resolve(u: U, token: u64, _iid: u32) -> u64 {
    if w(u).occs.borrow().contains_key(&token) {
        token
    } else {
        release(u, token);
        0
    }
}
unsafe extern "C" fn insert(u: U, o: u64) -> bool {
    let world = w(u);
    if let Some(f) = world.on_insert.borrow().as_ref() {
        f();
    }
    set_in_manager(u, o, true);
    add_ref(u, o);
    true
}
unsafe extern "C" fn remove(u: U, o: u64) -> bool {
    if in_manager(u, o) {
        set_in_manager(u, o, false);
        release(u, o);
    }
    w(u).log.borrow_mut().push(format!("remove {o}"));
    true
}
unsafe extern "C" fn move_occ(u: U, o: u64) {
    w(u).log.borrow_mut().push(format!("move {o}"));
}
unsafe extern "C" fn in_bbox(
    u: U,
    mn: *const f32,
    mx: *const f32,
    t: u32,
    blocking: bool,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) {
    let (a, b) = ([*mn, *mn.add(1), *mn.add(2)], [*mx, *mx.add(1), *mx.add(2)]);
    let ids: Vec<u64> = w(u)
        .occs
        .borrow()
        .iter()
        .filter(|(_, o)| {
            o.in_manager
                && (if blocking {
                    o.t == occupant_type::FLORA
                } else {
                    o.t == t
                })
        })
        .filter(|(_, o)| {
            let (p, q) = (
                [o.pos[0] - 1.0, o.pos[1], o.pos[2] - 1.0],
                [o.pos[0] + 1.0, o.pos[1] + 3.0, o.pos[2] + 1.0],
            );
            (0..3).all(|i| p[i] <= b[i] && a[i] <= q[i])
        })
        .map(|(k, _)| *k)
        .collect();
    for id in ids {
        if let Some(f) = cb {
            f(ctx, id);
        }
    }
}
unsafe extern "C" fn in_cells(u: U, r: *const fc4_cell_rect, t: u32, cb: fc4_token_callback, ctx: *mut c_void) {
    let r = r.as_ref().copied();
    let ids: Vec<u64> = w(u)
        .occs
        .borrow()
        .iter()
        .filter(|(_, o)| o.t == t)
        .filter(|(_, o)| {
            let (cx, cz) = ((o.pos[0] / 16.0) as i32, (o.pos[2] / 16.0) as i32);
            r.is_none_or(|r| r.x1 <= cx && cx <= r.x2 && r.z1 <= cz && cz <= r.z2)
        })
        .map(|(k, _)| *k)
        .collect();
    for id in ids {
        if let Some(f) = cb {
            f(ctx, id);
        }
    }
}
unsafe extern "C" fn in_manager_cells(u: U, t: u32, cb: fc4_token_callback, ctx: *mut c_void) {
    in_cells(u, std::ptr::null(), t, cb, ctx)
}

unsafe extern "C" fn seg_open(u: U, seg: u64, t: u32, g: u32, i: u32, write: bool) -> u64 {
    let w = w(u);
    let key = (seg, t, g, i);
    let bytes = if write {
        Vec::new()
    } else {
        match w.segments.borrow().get(&key) {
            Some(b) => b.clone(),
            None => return 0,
        }
    };
    let id = w.next_stream.get() + 1;
    w.next_stream.set(id);
    w.streams.borrow_mut().insert(
        id,
        SegStream {
            key,
            write,
            bytes,
            ..Default::default()
        },
    );
    w.log
        .borrow_mut()
        .push(format!("open {i} {}", if write { "w" } else { "r" }));
    id
}
unsafe extern "C" fn seg_close(u: U, s: u64) {
    let w = w(u);
    let st = w.streams.borrow_mut().remove(&s);
    if let Some(st) = st.filter(|st| st.write) {
        w.segments.borrow_mut().insert(st.key, st.bytes);
    }
    w.log.borrow_mut().push("close".into());
}
fn with_stream<T>(u: U, s: u64, f: impl FnOnce(&mut SegStream) -> T) -> Option<T> {
    w(u).streams.borrow_mut().get_mut(&s).map(f)
}
fn take(st: &mut SegStream, n: usize) -> Option<Vec<u8>> {
    if st.pos + n > st.bytes.len() {
        st.error = true;
        return None;
    }
    st.pos += n;
    Some(st.bytes[st.pos - n..st.pos].to_vec())
}
unsafe extern "C" fn put_u16(u: U, s: u64, v: u16) -> bool {
    with_stream(u, s, |st| st.bytes.extend(v.to_le_bytes())).is_some()
}
unsafe extern "C" fn put_u32(u: U, s: u64, v: u32) -> bool {
    with_stream(u, s, |st| st.bytes.extend(v.to_le_bytes())).is_some()
}
unsafe extern "C" fn get_u16(u: U, s: u64, out: *mut u16) -> bool {
    match with_stream(u, s, |st| take(st, 2)).flatten() {
        Some(b) => {
            *out = u16::from_le_bytes([b[0], b[1]]);
            true
        }
        None => false,
    }
}
unsafe extern "C" fn get_u32(u: U, s: u64, out: *mut u32) -> bool {
    match with_stream(u, s, |st| take(st, 4)).flatten() {
        Some(b) => {
            *out = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
            true
        }
        None => false,
    }
}
unsafe extern "C" fn stream_error(u: U, s: u64) -> bool {
    with_stream(u, s, |st| st.error).unwrap_or(true)
}
/// A stand-in occupant record: u16 3, the flora type, flag 1 as a byte.
unsafe extern "C" fn write_occ(u: U, o: u64, s: u64) -> bool {
    let o = w(u).occ(o);
    with_stream(u, s, |st| {
        st.bytes.extend(3u16.to_le_bytes());
        st.bytes.extend(o.flora.flora_type.to_le_bytes());
        st.bytes.push(u8::from(o.in_manager));
    })
    .is_some()
}
unsafe extern "C" fn read_occ(u: U, o: u64, s: u64) -> bool {
    let Some(b) = with_stream(u, s, |st| take(st, 7)).flatten() else {
        return false;
    };
    with_flora(u, o, |x| {
        x.flora.flora_type = u32::from_le_bytes([b[2], b[3], b[4], b[5]])
    });
    true
}
unsafe extern "C" fn obliterated(u: U) -> u64 {
    w(u).obliterated.get()
}

fn segment_host(h: &mut fc4_flora_host) {
    h.obliterated_segment = Some(obliterated);
    h.segment_open = Some(seg_open);
    h.segment_close = Some(seg_close);
    h.stream_put_u16 = Some(put_u16);
    h.stream_put_u32 = Some(put_u32);
    h.stream_get_u16 = Some(get_u16);
    h.stream_get_u32 = Some(get_u32);
    h.stream_error = Some(stream_error);
    h.write_occupant = Some(write_occ);
    h.read_occupant = Some(read_occ);
}

fn world_host(w: &World) -> fc4_flora_host {
    let mut h = empty_host();
    h.user = w.user();
    h.city_exists = Some(yes);
    h.city_size = Some(city_size);
    h.city_cell_counts = Some(city_cells);
    h.city_established = Some(no);
    h.sim_date_number = Some(date);
    h.flora_exemplars = Some(exemplars);
    h.register_app_shutdown_callback = Some(noop);
    h.subscribe = Some(subscribe);
    h.unsubscribe = Some(unsubscribe);
    h.register_command = Some(register_command);
    h.unregister_command = Some(unregister_id);
    h.add_agent = Some(add_agent);
    h.remove_agent = Some(noop);
    h.acquire_services = Some(noop);
    h.release_services = Some(noop);
    h.register_cheat = Some(register_cheat);
    h.unregister_cheat = Some(unregister_id);
    h.post = Some(post);
    h.terrain_initialized = Some(yes);
    h.altitude = Some(altitude);
    h.sea_level = Some(sea);
    h.face_normal = Some(normal);
    h.lot_savvy_altitude = Some(lot_alt);
    h.moisture_type = Some(moisture);
    h.occupant_type = Some(occ_type);
    h.occupant_has_property = Some(has_prop);
    h.occupant_position = Some(position);
    h.set_occupant_position = Some(set_position);
    h.occupant_bounding_box = Some(bbox);
    h.occupant_in_manager = Some(in_manager);
    h.set_occupant_in_manager = Some(set_in_manager);
    h.network_flags = Some(net_flags);
    h.flora_occupant = Some(flora_occ);
    h.set_flora_type = Some(set_type);
    h.set_birth_date = Some(set_birth);
    h.set_last_seeding_date = Some(set_last);
    h.set_flora_state = Some(set_state);
    h.set_flora_orientation = Some(set_orientation);
    h.set_flora_record = Some(set_record);
    h.post_occupant_change = Some(post_change);
    h.create_flora_occupant = Some(create_occ);
    h.init_occupant = Some(noop_occ);
    h.add_ref = Some(add_ref);
    h.release = Some(release);
    h.resolve_ref = Some(resolve);
    h.has_occupant_manager = Some(yes);
    h.insert_occupant = Some(insert);
    h.remove_occupant = Some(remove);
    h.move_occupant = Some(move_occ);
    h.occupants_in_bbox = Some(in_bbox);
    h.occupants_in_cells = Some(in_cells);
    h.occupants_in_manager_cells = Some(in_manager_cells);
    h
}

fn message(t: u32, d1: u32) -> fc4_flora_message {
    fc4_flora_message {
        msg_type: t,
        data1: d1,
        ..Default::default()
    }
}

fn state(f: *const fc4_flora) -> fc4_flora_state {
    let mut s = fc4_flora_state::default();
    assert!(unsafe { fc4_flora_get_state(f, &mut s) });
    s
}

/// A world with types 0x1000 (token 11), a second exemplar of 0x1000 (token
/// 12, dropped), 0x2000 (token 13) and a non-flora exemplar (token 14), and
/// an initialised simulator over a fresh registry.
struct Fixture {
    world: Rc<World>,
    reg: *mut fc4_flora_registry,
    f: *mut fc4_flora,
}

impl Fixture {
    fn new() -> Fixture {
        let world = World::new();
        *world.exemplars.borrow_mut() = vec![
            (0x1000, FLORA_EXEMPLAR_TYPE, 11),
            (0x1000, FLORA_EXEMPLAR_TYPE, 12),
            (0x2000, FLORA_EXEMPLAR_TYPE, 13),
            (0x3000, 0x10, 14),
        ];
        let rh = registry_host(&world);
        let h = world_host(&world);
        unsafe {
            let reg = fc4_flora_registry_create(&rh);
            assert!(!reg.is_null());
            let f = fc4_flora_create(&h, reg, 7, 1);
            assert!(!f.is_null());
            assert!(fc4_flora_init(f));
            Fixture { world, reg, f }
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            fc4_flora_shutdown(self.f);
            fc4_flora_destroy(self.f);
            fc4_flora_registry_destroy(self.reg);
        }
    }
}

unsafe extern "C" fn push_u32(ctx: *mut c_void, v: u64) {
    (*(ctx as *mut Vec<u32>)).push(v as u32);
}

// ---- tests ------------------------------------------------------------------------------------------

#[test]
fn null_handles_and_short_tables_are_tolerated() {
    unsafe {
        assert!(fc4_flora_registry_create(std::ptr::null()).is_null());
        let mut short = empty_registry_host();
        short.struct_size = 4;
        assert!(fc4_flora_registry_create(&short).is_null());
        let rh = empty_registry_host();
        let reg = fc4_flora_registry_create(&rh);
        assert!(fc4_flora_create(std::ptr::null(), reg, 1, 1).is_null());
        let mut h = empty_host();
        h.struct_size = 8;
        assert!(fc4_flora_create(&h, reg, 1, 1).is_null());
        assert!(fc4_flora_create(&empty_host(), std::ptr::null(), 1, 1).is_null());
        let n = std::ptr::null_mut::<fc4_flora>();
        assert!(!fc4_flora_init(n));
        assert!(!fc4_flora_shutdown(n));
        assert!(fc4_flora_do_message(n, &message(msg::SIM_NEW_DAY, 1)));
        assert!(!fc4_flora_add_exemplar(n, props_ptr(flora_props(1, 0x0F)), 0));
        assert!(!fc4_flora_add_record(n, &fc4_flora_record::default(), 1));
        assert!(!fc4_flora_location_ok(n, 1, 0.0, 0.0, 0));
        let mut sc = 5.0f32;
        assert_eq!(fc4_flora_best_type(n, 0.0, 0.0, &mut sc), 0);
        assert_eq!(sc, 0.0);
        let mut pos = [1.0f32, 2.0, 3.0];
        assert_eq!(fc4_flora_add_new_occupant(n, 1, pos.as_mut_ptr(), false), 0);
        assert_eq!(fc4_flora_add_new_occupant(n, 1, std::ptr::null_mut(), false), 0);
        assert!(!fc4_flora_add_occupant(n, 1));
        assert!(!fc4_flora_demolish_occupant(n, 1, 0));
        assert!(!fc4_flora_demolish_cells(n, std::ptr::null(), 0));
        assert!(!fc4_flora_demolish_bbox(n, std::ptr::null(), std::ptr::null(), 0));
        assert!(!fc4_flora_kill(n, 1));
        assert!(!fc4_flora_save_obliterated(n, 1));
        let mut st = fc4_flora_state::default();
        assert!(!fc4_flora_get_state(n, &mut st));
        assert!(!fc4_flora_write(n, std::ptr::null()));
        assert!(!fc4_flora_read(n, std::ptr::null()));
        fc4_flora_destroy(n);
        let nr = std::ptr::null::<fc4_flora_registry>();
        assert_eq!(fc4_flora_registry_record(nr, 0), 0);
        assert!(!fc4_flora_registry_is_type_valid(nr, 0));
        assert!(!fc4_flora_registry_is_family_valid(nr, 0));
        let mut c = 3u32;
        assert_eq!(fc4_flora_registry_types_into(nr, std::ptr::null_mut(), &mut c), 0);
        assert_eq!(fc4_flora_registry_list(nr, 0, None, std::ptr::null_mut()), 0);
        assert_eq!(fc4_flora_registry_family(nr, 0, None, std::ptr::null_mut()), 0);
        assert!(!fc4_flora_registry_sizes(
            nr,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut()
        ));
        fc4_flora_registry_destroy(std::ptr::null_mut());
        // An empty host: every call answers neutrally.
        let f = fc4_flora_create(&empty_host(), reg, 1, 1);
        assert!(fc4_flora_init(f));
        assert!(fc4_flora_do_message(f, &message(msg::SIM_NEW_MONTH, 30)));
        assert!(fc4_flora_do_message(f, &message(msg::SIM_NEW_DAY, 31)));
        assert!(fc4_flora_shutdown(f));
        fc4_flora_destroy(f);
        fc4_flora_registry_destroy(reg);
    }
}

#[test]
fn init_registers_blocks_once_per_type_and_releases_unused_exemplars() {
    let fx = Fixture::new();
    let w = &fx.world;
    unsafe {
        // Blocks: 0x1000 (with exemplar 11), 0x2000 (13), key 0 (none).
        let blocks = w.blocks.borrow().clone();
        let mut by_type: BTreeMap<u32, (u64, u64)> = BTreeMap::new();
        for (b, r, ex) in &blocks {
            assert!(by_type.insert(r.type_id, (*b, *ex)).is_none(), "one block per type");
        }
        assert_eq!(by_type.keys().copied().collect::<Vec<_>>(), vec![0, 0x1000, 0x2000]);
        assert_eq!(by_type[&0x1000].1, 11);
        assert_eq!(by_type[&0x2000].1, 13);
        assert_eq!(by_type[&0].1, 0);
        let mut rel = w.released_exemplars.borrow().clone();
        rel.sort_unstable();
        assert_eq!(rel, vec![12, 14], "the duplicate and the non-flora exemplar");
        // [choice] key 0 flags 0x01; exemplar records bits 1 and 2.
        let k0 = blocks.iter().find(|b| b.1.type_id == 0).unwrap().1;
        assert_eq!(k0.flags, 0x01);
        let a = blocks.iter().find(|b| b.1.type_id == 0x1000).unwrap().1;
        assert_eq!(a.flags, 0x06);
        assert_eq!(a.table[0], 153);
        // FloraTypesUpdated after the load, data2 = map entries before key 0.
        assert_eq!(w.posts.borrow().as_slice(), &[(msg::FLORA_TYPES_UPDATED, 2)]);
        // Getters.
        assert_eq!(fc4_flora_registry_record(fx.reg, 0x1000), by_type[&0x1000].0);
        assert_eq!(fc4_flora_registry_record(fx.reg, 0), by_type[&0].0);
        assert_eq!(fc4_flora_registry_record(fx.reg, 0x3000), 0);
        assert!(fc4_flora_registry_is_type_valid(fx.reg, 0));
        assert!(!fc4_flora_registry_is_type_valid(fx.reg, 0x3000));
        let mut n = 0u32;
        assert_eq!(fc4_flora_registry_types_into(fx.reg, std::ptr::null_mut(), &mut n), 2);
        assert_eq!(n, 2);
        let mut buf = [0u32; 1];
        let mut n1 = 1u32;
        assert_eq!(fc4_flora_registry_types_into(fx.reg, buf.as_mut_ptr(), &mut n1), 1);
        assert_eq!(buf, [0x1000]);
        let mut keys: Vec<u32> = Vec::new();
        assert_eq!(
            fc4_flora_registry_list(fx.reg, 1, Some(push_u32), &mut keys as *mut Vec<u32> as *mut c_void),
            3
        );
        assert_eq!(keys, vec![0, 0x1000, 0x2000]);
        let mut types: Vec<u32> = Vec::new();
        fc4_flora_registry_list(fx.reg, 0, Some(push_u32), &mut types as *mut Vec<u32> as *mut c_void);
        assert_eq!(types, vec![0x1000, 0x2000]);
        assert_eq!(fc4_flora_registry_list(fx.reg, 2, None, std::ptr::null_mut()), 0);
        assert_eq!(fc4_flora_registry_list(fx.reg, 9, None, std::ptr::null_mut()), 0);
        let (mut m, mut t, mut fam, mut b) = (0, 0, 0, 0);
        assert!(fc4_flora_registry_sizes(fx.reg, &mut m, &mut t, &mut fam, &mut b));
        assert_eq!((m, t, fam, b), (3, 2, 0, 3));
        // The status counters.
        let s = state(fx.f);
        assert_eq!(
            (s.initialized, s.map_len, s.type_count, s.block_count, s.registry_loaded),
            (1, 3, 2, 3, 1)
        );
        assert_eq!((s.cells_x, s.blocks_per_month, s.iterator_count), (64, 170, 1024));
        assert!(w.log.borrow().iter().any(|l| l == "agent 4 cSC4FloraSimulator 3"));
        assert!(w.log.borrow().iter().any(|l| l == "command 0xab99a64b PlaceFlora"));
        assert!(w.log.borrow().iter().any(|l| l == "cheat 0x95bdc9c Flora"));
    }
}

#[test]
fn a_second_simulator_shares_the_registry() {
    let fx = Fixture::new();
    let w = &fx.world;
    unsafe {
        let blocks = w.blocks.borrow().len();
        let h = world_host(w);
        let f2 = fc4_flora_create(&h, fx.reg, 8, 3);
        assert!(fc4_flora_init(f2));
        // Loaded once per process: no new types; key 0 is already present.
        assert_eq!(w.blocks.borrow().len(), blocks);
        assert_eq!(w.posts.borrow().len(), 1);
        fc4_flora_shutdown(f2);
        fc4_flora_destroy(f2);
    }
}

#[test]
fn slot_5_and_slot_6_register_records_with_their_blocks() {
    let fx = Fixture::new();
    let w = &fx.world;
    unsafe {
        // Slot 5: the exemplar reference goes to the new block.
        assert!(fc4_flora_add_exemplar(fx.f, props_ptr(flora_props(0x4000, 0x0F)), 21));
        let (b, r, ex) = *w.blocks.borrow().last().unwrap();
        assert_eq!((r.type_id, ex), (0x4000, 21));
        assert_eq!(fc4_flora_registry_record(fx.reg, 0x4000), b);
        // Already registered: false, the reference released.
        assert!(!fc4_flora_add_exemplar(fx.f, props_ptr(flora_props(0x4000, 0x0F)), 22));
        assert!(w.released_exemplars.borrow().contains(&22));
        // No props: false, released.
        assert!(!fc4_flora_add_exemplar(fx.f, std::ptr::null_mut(), 23));
        assert!(w.released_exemplars.borrow().contains(&23));
        // Slot 6: the caller's block; its next type captures seed types.
        let mut v = fc4_flora_record {
            type_id: 0x5000,
            next_type: 0x4000,
            seed_type: 0x5000,
            ..Default::default()
        };
        let fams = [0xF1u32, 0xF2];
        v.families = fams.as_ptr();
        v.family_count = 2;
        let before = w.blocks.borrow().len();
        assert!(fc4_flora_add_record(fx.f, &v, 0xB10C));
        assert_eq!(w.blocks.borrow().len(), before, "no block built for slot 6");
        assert_eq!(fc4_flora_registry_record(fx.reg, 0x5000), 0xB10C);
        assert!(!fc4_flora_add_record(fx.f, &v, 0xB10D));
        assert!(fc4_flora_registry_is_family_valid(fx.reg, 0xF1));
        assert!(fc4_flora_registry_is_family_valid(fx.reg, 0x5000));
        assert!(!fc4_flora_registry_is_family_valid(fx.reg, 0xF3));
        let mut fam: Vec<u32> = Vec::new();
        assert_eq!(
            fc4_flora_registry_family(fx.reg, 0xF2, Some(push_u32), &mut fam as *mut Vec<u32> as *mut c_void),
            1
        );
        assert_eq!(fam, vec![0x5000]);
        // §6.11: 0x4000's seed type (0x4000 = 0x5000's next) becomes 0x5000.
        assert!(w.seeds.borrow().contains(&(b, 0x5000)), "{:?}", w.seeds.borrow());
    }
}

#[test]
fn planting_counts_created_and_planted_and_hands_over_the_block() {
    let fx = Fixture::new();
    let w = &fx.world;
    unsafe {
        let mut pos = [100.0f32, 0.0, 100.0];
        let o = fc4_flora_add_new_occupant(fx.f, 0x1000, pos.as_mut_ptr(), false);
        assert_ne!(o, 0);
        assert_eq!(pos, [100.0, 260.0, 100.0]);
        let occ = w.occ(o);
        assert!(occ.in_manager);
        assert_eq!(occ.record, fc4_flora_registry_record(fx.reg, 0x1000));
        assert_eq!(occ.flora.birth_date, 5000);
        assert_eq!(
            w.refs_of(o),
            1,
            "the local reference was released; the manager keeps one"
        );
        // Too close: refused by the location test, nothing created.
        let mut p2 = [100.5f32, 0.0, 100.5];
        assert_eq!(
            fc4_flora_add_new_occupant(fx.f, 0x1000, p2.as_mut_ptr(), true),
            w.next.get() - 1
        );
        let s = state(fx.f);
        assert_eq!((s.created, s.planted, s.spread, s.refused), (2, 2, 0, 0));
        assert!(fc4_flora_location_ok(fx.f, 0x1000, 500.0, 500.0, 0));
        assert!(!fc4_flora_location_ok(fx.f, 0x1000, 100.0, 100.0, 0));
        let mut sc = 0.0f32;
        assert_eq!(fc4_flora_best_type(fx.f, 500.0, 500.0, &mut sc), 0x1000);
        assert!((sc - 0.6).abs() < 1e-6);
        // Demolition and slot 18.
        assert!(fc4_flora_demolish_occupant(fx.f, o, 0));
        assert!(!w.occ(o).in_manager);
        assert!(fc4_flora_add_occupant(fx.f, o));
        let r = fc4_cell_rect {
            x1: 6,
            z1: 6,
            x2: 6,
            z2: 6,
        };
        assert!(fc4_flora_demolish_cells(fx.f, &r, 0));
        assert!(!w.occ(o).in_manager);
        let (mn, mx) = ([0.0f32; 3], [1024.0f32, 1e30, 1024.0]);
        assert!(fc4_flora_demolish_bbox(fx.f, mn.as_ptr(), mx.as_ptr(), 0));
        let s = state(fx.f);
        assert_eq!(s.removed_other, 3);
    }
}

#[test]
fn the_monthly_pass_counts_aged_reborn_and_dying() {
    let fx = Fixture::new();
    let w = &fx.world;
    // 0x1000: maturity 730, lifespan 1460, max age 1560. At day 5000:
    //   1 born 4600 (age 400 >= 365): maturing (aged);
    //   2 born 3000 (age 2000 >= 1560): reborn;
    //   3 state 3 (burnt): reborn;
    //   4 type 0x2000, born 4990: young, score 0.6 >= 0.3: unchanged.
    w.add_flora(1, 40.0, 40.0, 0x1000, 0, 4600);
    w.add_flora(2, 80.0, 40.0, 0x1000, 2, 3000);
    w.add_flora(3, 120.0, 40.0, 0x1000, 3, 4990);
    w.add_flora(4, 160.0, 40.0, 0x2000, 0, 4990);
    unsafe {
        // 170 blocks a month (64 x 64 cells, period 6): seven months visit
        // all 1024 blocks.
        assert_eq!(state(fx.f).iterator_count, 1024);
        for _ in 0..7 {
            assert!(fc4_flora_do_message(fx.f, &message(msg::SIM_NEW_MONTH, 5000)));
        }
        let s = state(fx.f);
        assert_eq!(s.months, 7);
        assert_eq!(s.blocks_visited, 7 * 170);
        assert_eq!(w.occ(1).flora.state, 1);
        assert_eq!(w.occ(2).flora.birth_date, 5000);
        assert_eq!(w.occ(3).flora.state, 0);
        assert_eq!(w.occ(4).flora.state, 0);
        assert_eq!((s.aged, s.reborn, s.dying, s.removed_month), (1, 2, 0, 0));
        assert!(s.occupants_visited >= 4);
        assert!(w.log.borrow().iter().any(|l| l == "change 1"));
        // A low score (moisture type 16 reads table byte 0) kills young
        // flora: dying, removed during the month.
        w.moisture.set(16);
        for _ in 0..7 {
            fc4_flora_do_message(fx.f, &message(msg::SIM_NEW_MONTH, 5000));
        }
        let s = state(fx.f);
        assert!(s.dying >= 1 && s.removed_month == s.dying, "{s:?}");
    }
}

#[test]
fn kill_and_the_daily_clearing_are_counted() {
    let fx = Fixture::new();
    let w = &fx.world;
    w.add_flora(1, 40.0, 40.0, 0x1000, 0, 4600);
    w.add_flora(2, 80.0, 40.0, 0x1000, 0, 4600);
    unsafe {
        assert!(fc4_flora_kill(fx.f, 1));
        assert!(fc4_flora_kill(fx.f, 1), "already burnt: true");
        assert!(!fc4_flora_kill(fx.f, 99));
        assert_eq!(w.occ(1).flora.state, 3);
        assert_eq!(w.occ(1).flora.flora_type, BURNT_TREE);
        assert_eq!(w.occ(1).record, 0, "no Burnt Tree record registered");
        assert_eq!(w.refs_of(1), 2, "the burnt list holds one");
        let s = state(fx.f);
        assert_eq!((s.burnt, s.burnt_list, s.records_without_block), (1, 1, 0));
        // Code defaults: rate 1, step 4: the first day clears up to four.
        assert!(fc4_flora_do_message(fx.f, &message(msg::SIM_NEW_DAY, 5001)));
        let s = state(fx.f);
        assert_eq!((s.days, s.cleared, s.burnt_list), (1, 1, 0));
        assert!(!w.occ(1).in_manager);
        assert_eq!(w.refs_of(1), 0);
    }
}

#[test]
fn a_message_delivered_during_a_call_is_queued() {
    let fx = Fixture::new();
    let w = fx.world.clone();
    let f = fx.f as usize;
    *w.on_insert.borrow_mut() = Some(Box::new(move || unsafe {
        // Inside the planting call (depth 1 → 2).
        assert!(fc4_flora_do_message(
            f as *mut fc4_flora,
            &message(msg::SIM_NEW_DAY, 5001)
        ));
        let s = state(f as *const fc4_flora);
        assert_eq!(s.days, 0, "not handled yet");
    }));
    unsafe {
        let mut pos = [300.0f32, 0.0, 300.0];
        assert_ne!(fc4_flora_add_new_occupant(fx.f, 0x1000, pos.as_mut_ptr(), false), 0);
        *w.on_insert.borrow_mut() = None;
        let s = state(fx.f);
        assert_eq!((s.queued_messages, s.days), (1, 1), "handled when the call returned");
        assert!(s.reentrant_calls >= 1);
    }
}

#[test]
fn getters_do_not_wait_and_calls_from_another_thread_do() {
    let fx = Fixture::new();
    let w = fx.world.clone();
    let reg = fx.reg as usize;
    let f = fx.f as usize;
    let other: Rc<RefCell<Option<std::thread::JoinHandle<bool>>>> = Rc::new(RefCell::new(None));
    let o2 = other.clone();
    *w.on_insert.borrow_mut() = Some(Box::new(move || {
        // The main thread holds the lock here. A getter answers at once.
        let t =
            std::thread::spawn(move || unsafe { fc4_flora_registry_record(reg as *const fc4_flora_registry, 0x1000) });
        assert_ne!(t.join().unwrap(), 0);
        // A simulator call waits until the planting returns. The lock counts
        // a thread when it starts to wait (the owner re-enters without being
        // counted, so this thread can read the counter): wait until the
        // other thread is counted, however long a busy machine takes to
        // start it, instead of a fixed sleep.
        let before = state(f as *const fc4_flora).lock_waits;
        let h = std::thread::spawn(move || unsafe {
            let mut s = fc4_flora_state::default();
            fc4_flora_get_state(f as *const fc4_flora, &mut s)
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while state(f as *const fc4_flora).lock_waits == before {
            assert!(
                std::time::Instant::now() < deadline,
                "the other thread never waited for the lock"
            );
            assert!(!h.is_finished(), "it finished without waiting for the lock");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        // Counted: it is blocked until the planting returns.
        assert!(!h.is_finished(), "it waits for the lock");
        *o2.borrow_mut() = Some(h);
    }));
    unsafe {
        let mut pos = [600.0f32, 0.0, 600.0];
        assert_ne!(fc4_flora_add_new_occupant(fx.f, 0x1000, pos.as_mut_ptr(), false), 0);
    }
    *w.on_insert.borrow_mut() = None;
    let h = other.borrow_mut().take().unwrap();
    assert!(h.join().unwrap());
    assert!(state(fx.f).lock_waits >= 1);
}

#[test]
fn write_and_read_round_trip_with_the_burnt_list() {
    let fx = Fixture::new();
    let w = &fx.world;
    unsafe {
        let mut o = Out::default();
        assert!(fc4_flora_write(fx.f, &ostream(&mut o)));
        // §10.1: 2158 bytes for a new 64 x 64 city with no burnt flora.
        assert_eq!(o.bytes.len(), 2158);
        w.add_flora(1, 40.0, 40.0, 0x1000, 0, 4600);
        assert!(fc4_flora_kill(fx.f, 1));
        let mut o2 = Out::default();
        assert!(fc4_flora_write(fx.f, &ostream(&mut o2)));
        assert_eq!(o2.refs, vec![(1, 0)]);
        // Read into a fresh object (before its Init): one reference resolved.
        let h = world_host(w);
        let f2 = fc4_flora_create(&h, fx.reg, 9, 5);
        let mut i = In::new(&o2.bytes);
        assert!(fc4_flora_read(f2, &istream(&mut i)));
        assert_eq!(state(f2).burnt_list, 1);
        let refs = w.refs_of(1);
        // Destroyed without Init: the list's reference is released.
        fc4_flora_destroy(f2);
        assert_eq!(w.refs_of(1), refs - 1);
        // A bad version: false.
        let mut bad = o.bytes.clone();
        bad[0] = 3;
        let mut i = In::new(&bad);
        assert!(!fc4_flora_read(fx.f, &istream(&mut i)));
    }
}

#[test]
fn shadow_registry_and_month_compare_a_faithful_original() {
    // The "original" is our own simulator here: its registry and its
    // monthly pass must match the shadow's predictions exactly.
    let fx = Fixture::new();
    let w = &fx.world;
    unsafe {
        let shadow = fc4_flora_shadow_create();
        assert!(!shadow.is_null());
        assert_eq!(fc4_flora_shadow_set_tuning(shadow, std::ptr::null_mut()), 0.08);
        // Registry: the original's records as its blocks hold them.
        let blocks = w.blocks.borrow().clone();
        let rec = |t: u32| blocks.iter().find(|b| b.1.type_id == t).unwrap().1;
        let types = [0x1000u32, 0x2000];
        let recs = [rec(0x1000), rec(0x2000)];
        let k0 = rec(0);
        unsafe extern "C" fn enumerate(u: U, visit: fc4_flora_exemplar_visitor, ctx: *mut c_void) {
            let list = self::w(u).exemplars.borrow().clone();
            for (i, (t, et, _)) in list.into_iter().enumerate() {
                if let Some(f) = visit {
                    f(ctx, 0x6534_284A, 1, i as u32, props_ptr(flora_props(t, et)), 0);
                }
            }
        }
        let mut lines: Vec<String> = Vec::new();
        unsafe extern "C" fn sink(ctx: *mut c_void, p: *const u8, n: usize) {
            let v = &mut *(ctx as *mut Vec<String>);
            v.push(String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned());
        }
        let ctx = &mut lines as *mut Vec<String> as *mut c_void;
        let n = fc4_flora_shadow_registry(
            shadow,
            Some(enumerate),
            w.user(),
            types.as_ptr(),
            2,
            recs.as_ptr(),
            &k0,
            true,
            Some(sink),
            ctx,
        );
        assert_eq!(n, 0, "{lines:?}");
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("flora-static-shadow: compared=3 mismatches=0")),
            "{lines:?}"
        );
        // A wrong type order and a changed float are found.
        let swapped = [0x2000u32, 0x1000];
        let mut r2 = [rec(0x2000), rec(0x1000)];
        r2[1].floats[3] = 7.0;
        let n = fc4_flora_shadow_registry(
            shadow,
            Some(enumerate),
            w.user(),
            swapped.as_ptr(),
            2,
            r2.as_ptr(),
            &k0,
            false,
            Some(sink),
            ctx,
        );
        assert_eq!(n, 3, "{lines:?}");
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("flora-static-shadow: compared=3 mismatches=1")),
            "{lines:?}"
        );
        assert!(lines.last().unwrap().contains("float[3]"), "{lines:?}");

        // Month: census before, the pass, census after.
        w.add_flora(1, 40.0, 40.0, 0x1000, 0, 4600); // ages
        w.add_flora(2, 80.0, 40.0, 0x1000, 2, 3000); // reborn
        w.add_flora(3, 520.0, 520.0, 0x1000, 0, 4990); // stays young
        let census = |w: &World| -> Vec<fc4_flora_census_entry> {
            w.occs
                .borrow()
                .iter()
                .filter(|(_, o)| o.in_manager)
                .map(|(&k, o)| fc4_flora_census_entry {
                    token: k,
                    flora_type: o.flora.flora_type,
                    birth_date: o.flora.birth_date,
                    last_seeding_date: o.flora.last_seeding_date,
                    state: o.flora.state,
                    in_manager: 1,
                    is_flora: 1,
                    pos: o.pos,
                    bbox_min: [o.pos[0] - 1.0, o.pos[1], o.pos[2] - 1.0],
                    bbox_max: [o.pos[0] + 1.0, o.pos[1] + 3.0, o.pos[2] + 1.0],
                    moisture: 8,
                    altitude: 260.0,
                    ..Default::default()
                })
                .collect()
        };
        let mut total_visited = 0;
        for month in 0..7u32 {
            let julian = 5000 + month * 30;
            let cb = census(w);
            let mut b = Out::default();
            assert!(fc4_flora_write(fx.f, &ostream(&mut b)));
            assert!(fc4_flora_do_message(fx.f, &message(msg::SIM_NEW_MONTH, julian)));
            let mut a = Out::default();
            assert!(fc4_flora_write(fx.f, &ostream(&mut a)));
            let ca = census(w);
            let all = [rec(0x1000), rec(0x2000)];
            let m = fc4_flora_shadow_month_in {
                struct_size: std::mem::size_of::<fc4_flora_shadow_month_in>() as u32,
                julian,
                sim_date: w.date.get(),
                sea_level: 250.0,
                before: b.bytes.as_ptr(),
                before_len: b.bytes.len(),
                after: a.bytes.as_ptr(),
                after_len: a.bytes.len(),
                census_before: cb.as_ptr(),
                before_count: cb.len(),
                census_after: ca.as_ptr(),
                after_count: ca.len(),
                records: all.as_ptr(),
                record_count: 2,
            };
            let r = fc4_flora_shadow_month(shadow, &m, Some(sink), ctx);
            assert_eq!(r, 0, "month {month}: {}", lines.last().unwrap());
            assert!(
                lines[lines.len() - 2].starts_with("flora-occupant-shadow:"),
                "{lines:?}"
            );
            assert!(lines[lines.len() - 2].contains("mismatches=0"), "{lines:?}");
        }
        let mut st = fc4_flora_shadow_stats::default();
        assert!(fc4_flora_shadow_get_stats(shadow, &mut st));
        total_visited += st.visited;
        assert_eq!(st.months, 7);
        assert_eq!(st.mismatched, 0);
        assert!(total_visited >= 3, "{st:?}");
        assert_eq!(st.predicted_aged, 1, "{st:?}");
        assert_eq!(st.predicted_reborn, 1, "{st:?}");
        assert!(lines.iter().any(|l| l.contains("wrapped")), "{lines:?}");
        // A tampered census is found: occupant 3 (young) claims state 2.
        let cb = census(w);
        let mut b = Out::default();
        fc4_flora_write(fx.f, &ostream(&mut b));
        assert!(fc4_flora_do_message(fx.f, &message(msg::SIM_NEW_MONTH, 5210)));
        let mut a = Out::default();
        fc4_flora_write(fx.f, &ostream(&mut a));
        let mut ca = census(w);
        for e in &mut ca {
            if e.token == 3 {
                e.state = 2;
            }
        }
        let all = [rec(0x1000), rec(0x2000)];
        let mut m = fc4_flora_shadow_month_in {
            struct_size: std::mem::size_of::<fc4_flora_shadow_month_in>() as u32,
            julian: 5210,
            sim_date: w.date.get(),
            sea_level: 250.0,
            before: b.bytes.as_ptr(),
            before_len: b.bytes.len(),
            after: a.bytes.as_ptr(),
            after_len: a.bytes.len(),
            census_before: cb.as_ptr(),
            before_count: cb.len(),
            census_after: ca.as_ptr(),
            after_count: ca.len(),
            records: all.as_ptr(),
            record_count: 2,
        };
        assert_eq!(fc4_flora_shadow_month(shadow, &m, Some(sink), ctx), 1, "{lines:?}");
        assert!(lines[lines.len() - 2].contains("mismatches=1"), "{lines:?}");
        assert!(lines.last().unwrap().contains("predicted 0"), "{lines:?}");
        // A missing type record must not turn into a clean unchanged prediction.
        let mut missing_record = m;
        missing_record.record_count = 0;
        assert_eq!(fc4_flora_shadow_month(shadow, &missing_record, Some(sink), ctx), -2);
        assert!(
            lines.last().unwrap().contains("static record 0x1000 not captured"),
            "{lines:?}"
        );
        // The same record before and after: the iterator did not move (170
        // blocks expected) -> incomplete.
        m.after = b.bytes.as_ptr();
        m.after_len = b.bytes.len();
        assert_eq!(fc4_flora_shadow_month(shadow, &m, Some(sink), ctx), -2);
        assert!(lines.last().unwrap().contains("INCOMPLETE"), "{lines:?}");
        let mut no_capture = m;
        no_capture.before = std::ptr::null();
        assert_eq!(fc4_flora_shadow_month(shadow, &no_capture, Some(sink), ctx), -2);
        assert_eq!(
            fc4_flora_shadow_month(shadow, std::ptr::null(), None, std::ptr::null_mut()),
            -1
        );
        assert_eq!(
            fc4_flora_shadow_month(std::ptr::null_mut(), &m, None, std::ptr::null_mut()),
            -1
        );
        fc4_flora_shadow_destroy(shadow);
    }
}

/// §10.3 (FQ3) through the C host: every record starts with u16 0; the
/// count record is u16 0, u32 count; the replacement city's Init reads them
/// back and inserts the occupants.
#[test]
fn obliteration_records_go_through_the_host_streams() {
    let fx = Fixture::new();
    let w = &fx.world;
    w.add_flora(1, 100.0, 100.0, 0x1000, 2, 5000);
    w.add_flora(2, 200.0, 100.0, 0x2000, 1, 5000);
    let mut h = world_host(w);
    segment_host(&mut h);
    unsafe {
        let f = fc4_flora_create(&h, fx.reg, 8, 1);
        assert!(fc4_flora_init(f));
        w.log.borrow_mut().clear();
        assert!(fc4_flora_save_obliterated(f, 77));
        {
            let segs = w.segments.borrow();
            assert_eq!(segs[&(77, 0xC9BB_3FC6, 0xC9BB_3FC7, 1)], [0, 0, 2, 0, 0, 0]);
            assert_eq!(segs[&(77, 0xC9BB_3FC6, 0x4A66_4F83, 0)], [0, 0, 3, 0, 0, 0x10, 0, 0, 0]);
            assert_eq!(segs[&(77, 0xC9BB_3FC6, 0x4A66_4F83, 1)], [0, 0, 3, 0, 0, 0x20, 0, 0, 0]);
        }
        assert!(w.occ(1).in_manager && w.occ(2).in_manager, "flag 1 set again");
        assert_eq!(
            w.log.borrow().as_slice(),
            ["open 1 w", "close", "open 0 w", "close", "open 1 w", "close"]
        );
        fc4_flora_shutdown(f);
        fc4_flora_destroy(f);
        // The replacement city (no flora of its own) re-creates them.
        w.occs.borrow_mut().clear();
        w.obliterated.set(77);
        // AQ5 (§10.3): each occupant's record is still open at its insert.
        let weak = Rc::downgrade(&fx.world);
        *w.on_insert.borrow_mut() = Some(Box::new(move || {
            if let Some(w) = weak.upgrade() {
                let open = w.streams.borrow().len();
                w.log.borrow_mut().push(format!("insert (open {open})"));
            }
        }));
        w.log.borrow_mut().clear();
        let first = w.next.get();
        let g = fc4_flora_create(&h, fx.reg, 9, 1);
        assert!(fc4_flora_init(g));
        *w.on_insert.borrow_mut() = None;
        let made: Vec<(u32, bool, i64)> = (first..w.next.get())
            .map(|id| (w.occ(id).flora.flora_type, w.occ(id).in_manager, w.refs_of(id)))
            .collect();
        assert_eq!(made, [(0x1000, true, 1), (0x2000, true, 1)]);
        let records: Vec<String> = w
            .log
            .borrow()
            .iter()
            .filter(|l| l.starts_with("open ") || l.starts_with("insert ") || *l == "close")
            .cloned()
            .collect();
        assert_eq!(
            records,
            [
                "open 1 r",
                "close",
                "open 0 r",
                "insert (open 1)",
                "close",
                "open 1 r",
                "insert (open 1)",
                "close"
            ],
            "the record is closed after Read, Init, InsertOccupant and Release"
        );
        fc4_flora_shutdown(g);
        fc4_flora_destroy(g);
        // A segment without the count record restores nothing.
        w.obliterated.set(78);
        let first = w.next.get();
        let g = fc4_flora_create(&h, fx.reg, 10, 1);
        assert!(fc4_flora_init(g));
        assert_eq!(w.next.get(), first);
        fc4_flora_shutdown(g);
        fc4_flora_destroy(g);
    }
}

/// FQ14: after an exemplar-change reload key 0 is gone from the map (and the
/// any-thread snapshot) until the next Init puts it back.
#[test]
fn a_reload_drops_key_0_until_the_next_init() {
    let fx = Fixture::new();
    unsafe {
        assert!(fc4_flora_registry_is_type_valid(fx.reg, 0));
        let mut m = message(msg::EXEMPLAR_CHANGED, 0);
        m.data2 = 0x1000;
        assert!(fc4_flora_do_message(fx.f, &m));
        assert!(fc4_flora_registry_is_type_valid(fx.reg, 0x1000));
        assert!(!fc4_flora_registry_is_type_valid(fx.reg, 0));
        assert_eq!(fc4_flora_registry_record(fx.reg, 0), 0);
        let h = world_host(&fx.world);
        let g = fc4_flora_create(&h, fx.reg, 11, 1);
        assert!(fc4_flora_init(g));
        assert!(fc4_flora_registry_is_type_valid(fx.reg, 0));
        fc4_flora_shutdown(g);
        fc4_flora_destroy(g);
    }
}
