// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The prop developer through the C ABI (docs/specs/prop-developer.md): a
//! fake game behind `fc4_prop_host`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void, CStr};

use fc4_core::property::PropertyHolder;
use fc4_core::variant::{Elem, Variant};

use crate::prop_developer::*;
use crate::props::fc4_props;
use crate::sinks::{fc4_lot_cfg_object, fc4_lot_cfg_objects_sink, fc4_u64s_sink};
use crate::test_streams::{istream, ostream, In, Out};

const LOT: u64 = 0x10;
const WEALTH: u64 = 0x13B5_2FC0;

/// (lot tag, type, properties)
type Prop = (Option<u32>, u32, PropertyHolder);

#[derive(Default)]
struct Game {
    /// prop → (lot tag, type, properties)
    props: RefCell<BTreeMap<u64, Prop>>,
    next: RefCell<u64>,
    log: RefCell<Vec<String>>,
}

impl Game {
    fn new() -> Box<Game> {
        let g = Game::default();
        *g.next.borrow_mut() = 0x9000;
        Box::new(g)
    }
    fn user(&self) -> *mut c_void {
        self as *const Game as *mut c_void
    }
    fn log(&self, s: impl Into<String>) {
        self.log.borrow_mut().push(s.into());
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

unsafe extern "C" fn city_cells(_u: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = 64;
    *z = 64;
}
unsafe extern "C" fn managers(_u: *mut c_void, out: *mut fc4_prop_ref) {
    for (i, (t, c)) in [
        (0x1331_0038u64, 0x498F_9B01u32),
        (0x1137_876C, 0xA990_BD10),
        (0x13B4_35FC, 0xE9A6_CD3B),
    ]
    .into_iter()
    .enumerate()
    {
        *out.add(i) = fc4_prop_ref {
            token: t,
            class_id: c,
            reserved: 0,
        };
    }
}
unsafe extern "C" fn create_wealth_requester(_u: *mut c_void) -> u64 {
    WEALTH
}
unsafe extern "C" fn add_agent(u: *mut c_void, level: u32, name: *const c_char, flags: u32) {
    let n = CStr::from_ptr(name).to_string_lossy();
    game(u).log(format!("add_agent {level} {n} {flags}"));
}
unsafe extern "C" fn hold(u: *mut c_void, t: u64) {
    game(u).log(format!("hold {t:#x}"));
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    game(u).log(format!("release {t:#x}"));
}
unsafe extern "C" fn lot_at(_u: *mut c_void, x: i32, z: i32, _origin: bool) -> u64 {
    if (x, z) == (0, 0) {
        LOT
    } else {
        0
    }
}
unsafe extern "C" fn yes(_u: *mut c_void, _l: u64) -> bool {
    true
}
unsafe extern "C" fn building_type(_u: *mut c_void, _l: u64) -> u32 {
    0x100
}
unsafe extern "C" fn occupancy(_u: *mut c_void, _l: u64) -> f32 {
    0.5
}
unsafe extern "C" fn lot_wealth(_u: *mut c_void, _l: u64) -> u32 {
    2
}
unsafe extern "C" fn lot_config_objects(
    _u: *mut c_void,
    _l: u64,
    sink: fc4_lot_cfg_objects_sink,
    ctx: *mut c_void,
) -> bool {
    let ids = [0x500u32, 0x501];
    let objs: Vec<fc4_lot_cfg_object> = (0..4)
        .map(|i| fc4_lot_cfg_object {
            object_type: 1,
            tag: 0xA0 + i,
            flags: 1,
            reserved: [0; 3],
            id_count: 2,
            ids: ids.as_ptr(),
        })
        .collect();
    sink.unwrap()(ctx, objs.as_ptr(), objs.len());
    true
}
unsafe extern "C" fn lot_props(u: *mut c_void, lot: u64, sink: fc4_u64s_sink, ctx: *mut c_void) {
    let v: Vec<u64> = if lot == LOT {
        game(u).props.borrow().keys().copied().collect()
    } else {
        Vec::new()
    };
    sink.unwrap()(ctx, v.as_ptr(), v.len());
}
unsafe extern "C" fn prop_type_valid(_u: *mut c_void, id: u32) -> bool {
    id == 0x500 || id == 0x501
}
unsafe extern "C" fn prop_satisfaction(_u: *mut c_void, _id: u32) -> u32 {
    0x49A1_E995
}
unsafe extern "C" fn prop_wealth(_u: *mut c_void, id: u32) -> u8 {
    if id == 0x500 {
        2
    } else {
        3
    }
}
unsafe extern "C" fn create_lot_prop(u: *mut c_void, lot: u64, index: u32, id: u32) -> u64 {
    let g = game(u);
    let t = *g.next.borrow();
    *g.next.borrow_mut() = t + 1;
    g.props
        .borrow_mut()
        .insert(t, (Some(0xA0 + index), id, PropertyHolder::new()));
    g.log(format!("create {lot:#x} {index} {id:#x}"));
    t
}
unsafe extern "C" fn prop_class(_u: *mut c_void, _p: u64) -> u32 {
    0x2977_AA47
}
unsafe extern "C" fn prop_properties(u: *mut c_void, p: u64) -> *mut fc4_props {
    match game(u).props.borrow().get(&p) {
        Some(e) => Box::into_raw(Box::new(e.2.clone())) as *mut fc4_props,
        None => std::ptr::null_mut(),
    }
}
unsafe extern "C" fn prop_lot_tag(u: *mut c_void, p: u64, out: *mut u32) -> bool {
    match game(u).props.borrow().get(&p).and_then(|e| e.0) {
        Some(t) => {
            *out = t;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn set_prop_requester(u: *mut c_void, p: u64, r: u32) {
    if let Some(e) = game(u).props.borrow_mut().get_mut(&p) {
        e.2.add(0x89A1_C16C, Variant::Scalar(Elem::U32(r)));
    }
}
unsafe extern "C" fn prop_type(u: *mut c_void, p: u64) -> u32 {
    game(u).props.borrow().get(&p).map_or(0, |e| e.1)
}
unsafe extern "C" fn set_prop_state(u: *mut c_void, p: u64, s: u32) {
    game(u).log(format!("state {p:#x} {s}"));
}
unsafe extern "C" fn set_prop_timer(u: *mut c_void, p: u64, m: u32, on: bool) {
    game(u).log(format!("timer {p:#x} {m} {on}"));
}
unsafe extern "C" fn set_prop_flag(u: *mut c_void, p: u64, f: u32, on: bool) {
    game(u).log(format!("flag {p:#x} {f} {on}"));
}
unsafe extern "C" fn set_random_chance(_u: *mut c_void, _p: u64, _c: u8) {}
unsafe extern "C" fn view_available(_u: *mut c_void) -> bool {
    true
}

fn host(g: &Game) -> fc4_prop_host {
    // SAFETY: every field is a nullable pointer or an integer.
    let mut h: fc4_prop_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_prop_host>() as u32;
    h.user = g.user();
    h.city_cells = Some(city_cells);
    h.managers = Some(managers);
    h.create_wealth_requester = Some(create_wealth_requester);
    h.add_agent = Some(add_agent);
    h.hold = Some(hold);
    h.release = Some(release);
    h.lot_at = Some(lot_at);
    h.lot_livable = Some(yes);
    h.lot_building_type = Some(building_type);
    h.lot_occupancy = Some(occupancy);
    h.lot_config_objects = Some(lot_config_objects);
    h.lot_wealth = Some(lot_wealth);
    h.lot_props = Some(lot_props);
    h.prop_type_valid = Some(prop_type_valid);
    h.prop_satisfaction = Some(prop_satisfaction);
    h.prop_wealth = Some(prop_wealth);
    h.create_lot_prop = Some(create_lot_prop);
    h.prop_class = Some(prop_class);
    h.prop_properties = Some(prop_properties);
    h.prop_lot_tag = Some(prop_lot_tag);
    h.set_prop_requester = Some(set_prop_requester);
    h.prop_type = Some(prop_type);
    h.set_prop_state = Some(set_prop_state);
    h.set_prop_timer = Some(set_prop_timer);
    h.set_prop_flag = Some(set_prop_flag);
    h.set_random_chance = Some(set_random_chance);
    h.view_available = Some(view_available);
    h
}

fn write(p: *const fc4_prop_developer) -> Out {
    let mut o = Out::default();
    let os = ostream(&mut o);
    assert!(unsafe { fc4_prop_developer_write(p, &os) });
    o
}

fn read(p: *mut fc4_prop_developer, bytes: &[u8]) -> bool {
    let mut i = In::new(bytes);
    let is = istream(&mut i);
    unsafe { fc4_prop_developer_read(p, &is) }
}

fn msg(t: u32) -> fc4_prop_message {
    fc4_prop_message {
        msg_type: t,
        ..Default::default()
    }
}

#[test]
fn create_and_identity() {
    let g = Game::new();
    unsafe {
        assert!(fc4_prop_developer_create(std::ptr::null(), 0, 0).is_null());
        let mut h = host(&g);
        h.struct_size -= 1;
        assert!(fc4_prop_developer_create(&h, 0, 0).is_null());
        let mut off = 99;
        assert!(fc4_prop_developer_query_interface(0xE9A1_F4B6, &mut off));
        assert_eq!(off, 0);
        assert!(fc4_prop_developer_query_interface(0xE4FD_A3D4, &mut off));
        assert_eq!(off, 8);
        assert!(!fc4_prop_developer_query_interface(0x89C4_98BE, &mut off));
        assert!(fc4_wealth_requester_query_interface(0xE4FD_A3D4, &mut off));
        assert_eq!(off, 4);
        assert!(!fc4_prop_developer_signs_visible(std::ptr::null()));
        let p = fc4_prop_developer_create(&host(&g), 0x1234, 0xD0);
        assert_eq!(fc4_prop_developer_add_ref(p), 1);
        assert_eq!(fc4_prop_developer_release(p), 0);
        let mut s = fc4_prop_state::default();
        assert!(fc4_prop_developer_get_state(p, &mut s));
        assert_eq!((s.struct_size, s.destroyed, s.signs_visible), (104, 1, 1));
        assert!(!fc4_prop_developer_get_state(p, std::ptr::null_mut()));
        fc4_prop_developer_destroy(p);
    }
}

/// §12 tests 1, 3 and 5 through the ABI.
#[test]
fn init_block_and_record() {
    let g = Game::new();
    unsafe {
        let p = fc4_prop_developer_create(&host(&g), 0x1234, 0xD0);
        assert!(fc4_prop_developer_init(p));
        assert_eq!(
            g.take(),
            ["add_agent 10 cSC4PropDeveloper 2", "add_agent 4 cSC4PropDeveloper 2"]
        );
        fc4_prop_developer_add_prop_requester(p, 0x13B4_2BB8, 0x8990_C09A);
        fc4_prop_developer_add_prop_requester(p, 0x1134_7690, 0x8990_C065);
        assert_eq!(g.take(), ["hold 0x13b42bb8", "hold 0x11347690"]);
        let mut refs = [fc4_prop_ref::default(); 4];
        assert_eq!(
            fc4_prop_developer_list(p, FC4_PROP_LIST_REQUESTERS, refs.as_mut_ptr(), 4),
            3
        );
        assert_eq!((refs[0].token, refs[0].class_id), (WEALTH, 0x89C4_9986));
        assert_eq!(fc4_prop_developer_list(p, 9, refs.as_mut_ptr(), 4), 0);

        // First step sets up; the second updates block (0, 0): two wealth
        // props of wealth 2 (0x500) on the lot.
        assert!(fc4_prop_developer_do_message(p, &msg(0x6695_681A)));
        assert!(fc4_prop_developer_do_message(p, &msg(0x6695_681A)));
        let log = g.take();
        assert_eq!(log.len(), 2);
        assert!(log
            .iter()
            .all(|l| l.starts_with("create 0x10 ") && l.ends_with(" 0x500")));
        let mut b = fc4_prop_block::default();
        let (mut want, mut kept) = ([9u32; 3], [9u32; 3]);
        assert!(fc4_prop_developer_last_block(
            p,
            &mut b,
            want.as_mut_ptr(),
            kept.as_mut_ptr(),
            3
        ));
        assert_eq!(
            (b.struct_size, b.bx, b.bz, b.lots, b.slots, b.target),
            (52, 0, 0, 1, 4, 2)
        );
        assert_eq!((b.created, b.member_draws, b.requester_count), (2, 2, 3));
        assert_eq!((want, kept), ([2, 0, 0], [0, 0, 0]));
        assert_eq!(
            fc4_prop_developer_wealth_select(p, [0x501u32, 0x500].as_ptr(), 2, 0, 0),
            0x500
        );
        assert_eq!(fc4_prop_developer_wealth_select(p, [0x501u32].as_ptr(), 1, 0, 0), 0);

        // The record: version 5, scanning, three requesters, the managers.
        let o = write(p);
        assert_eq!(&o.bytes[..3], [5, 0, 2]);
        assert_eq!(
            o.refs[..6],
            [
                (WEALTH, 0x89C4_9986),
                (0x13B4_2BB8, 0x8990_C09A),
                (0x1134_7690, 0x8990_C065),
                (0x1331_0038, 0x498F_9B01),
                (0x1137_876C, 0xA990_BD10),
                (0x13B4_35FC, 0xE9A6_CD3B)
            ]
        );
        let q = fc4_prop_developer_create(&host(&g), 0x1234, 0xD1);
        assert!(!read(q, &o.bytes[..40]));
        assert!(read(q, &o.bytes));
        assert_eq!(write(q).bytes, o.bytes);
        let mut s = fc4_prop_state::default();
        assert!(fc4_prop_developer_get_state(q, &mut s));
        assert_eq!(
            (s.scan_flag, s.scanner_state, s.clip_x1, s.requester_count),
            (2, 1, 7, 3)
        );
        assert_eq!(s.wealth_added, 1);
        // A read requester of the wealth class is answered by the core.
        assert!(fc4_prop_developer_list(q, FC4_PROP_LIST_REQUESTERS, refs.as_mut_ptr(), 4) == 3);
        assert_eq!(refs[0].class_id, 0x89C4_9986);
        assert!(fc4_prop_developer_remove_prop_requester(q, 0x1134_7690));
        assert!(!fc4_prop_developer_remove_prop_requester(q, 0x1134_7690));
        assert_eq!(g.take(), ["release 0x11347690"]);
        fc4_prop_developer_destroy(q);
        fc4_prop_developer_destroy(p);
    }
}

/// §12 tests 12 and 13 through the ABI.
#[test]
fn registration_signs_and_triggers() {
    let g = Game::new();
    unsafe {
        let p = fc4_prop_developer_create(&host(&g), 7, 0xD0);
        assert!(fc4_prop_developer_init(p));
        let mut h = PropertyHolder::new();
        h.add(0x6A3F_CC37, Variant::Scalar(Elem::U32(0xCACB_DFC6)));
        g.props.borrow_mut().insert(0x400, (None, 0x600, h));
        g.props
            .borrow_mut()
            .insert(0x401, (None, 0xAB72_FBB3, PropertyHolder::new()));
        g.take();
        for prop in [0x400, 0x401] {
            let m = fc4_prop_message {
                msg_type: 0x99EF_1142,
                data3: 0xA823_821E,
                object: prop,
                ..Default::default()
            };
            assert!(fc4_prop_developer_do_message(p, &m));
        }
        assert!(g.take().contains(&"hold 0x400".to_string()));
        let mut trig = 0u32;
        let mut refs = [fc4_prop_ref::default(); 2];
        assert_eq!(
            fc4_prop_developer_trigger_entry(p, 0, &mut trig, refs.as_mut_ptr(), 2),
            1
        );
        assert_eq!(
            (trig, refs[0].token, refs[0].class_id),
            (0xCACB_DFC6, 0x400, 0x2977_AA47)
        );
        assert_eq!(
            fc4_prop_developer_trigger_entry(p, 1, &mut trig, refs.as_mut_ptr(), 2),
            usize::MAX
        );
        assert_eq!(fc4_prop_developer_list(p, FC4_PROP_LIST_SIGNS, refs.as_mut_ptr(), 2), 1);
        fc4_prop_developer_show_signs(p, false);
        fc4_prop_developer_show_signs(p, false);
        assert_eq!(g.take(), ["flag 0x401 2 false"]);
        assert!(!fc4_prop_developer_signs_visible(p));
        fc4_prop_developer_toggle_signs(p);
        assert!(fc4_prop_developer_signs_visible(p));
        // An animation tick in night mode 0 answers false.
        assert!(!fc4_prop_developer_do_message(p, &msg(0x86AD_10EE)));
        assert!(!fc4_prop_developer_do_message(p, std::ptr::null()));
        fc4_prop_developer_update_all(p);
        let mut b = fc4_prop_block::default();
        assert!(fc4_prop_developer_last_block(
            p,
            &mut b,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0
        ));
        assert_eq!((b.bx, b.bz), (7, 7));
        assert!(fc4_prop_developer_shutdown(p));
        let mut s = fc4_prop_state::default();
        assert!(fc4_prop_developer_get_state(p, &mut s));
        assert_eq!(
            (s.trigger_count, s.sign_count, s.requester_count, s.scan_flag),
            (0, 1, 0, 0)
        );
        fc4_prop_developer_destroy(p);
    }
}

/// The save sweep (`prop_city_sweep_tests.rs`): `rec` read into a fresh
/// developer (Read before Init, §7) and written again.
pub(super) fn sweep_round_trip(rec: &[u8]) -> Result<Vec<u8>, String> {
    let g = Game::new();
    unsafe {
        let p = fc4_prop_developer_create(&host(&g), 1, 0x7000);
        if p.is_null() {
            return Err("create failed".into());
        }
        let r = crate::save_sweep_tests::read_write(
            rec,
            |i| fc4_prop_developer_read(p, i),
            |o| fc4_prop_developer_write(p, o),
        );
        fc4_prop_developer_destroy(p);
        r
    }
}

#[test]
fn pv18_effect_transport_advances_only_owned_semantics_and_rng() {
    use fc4_core::rng::{FixedSeeds, RzRandomSuper};
    unsafe {
        let mut state = fc4_prop_effect_state {
            present: 1,
            interval: 10,
            duration: 2,
            start_day: 100,
            end_day: 102,
            timer: 13,
            chance: 100,
            state: 1,
        };
        let (mut words, next) = RzRandomSuper::new(
            1,
            &FixedSeeds {
                wall_clock: 1,
                process_clock: 1,
            },
        )
        .raw_state();
        let mut index = next as u32;
        let mut result = fc4_prop_effect_result::default();
        assert!(fc4_prop_developer_shadow_effect(
            &mut state,
            16,
            [0; 4].as_ptr(),
            100,
            words.as_mut_ptr(),
            &mut index,
            &mut result
        ));
        assert_eq!(
            (state.start_day, state.end_day, state.timer, state.state),
            (110, 102, 15, 0)
        );
        assert_eq!(
            (result.state_called, result.state_changed, result.requested_state),
            (1, 1, 0)
        );
        assert_eq!(
            (
                result.timer_called,
                result.timer_mask,
                result.timer_on,
                result.random_drawn
            ),
            (1, 2, 1, 1)
        );
        assert_eq!(index, 1);
        assert_eq!(std::mem::size_of::<fc4_prop_effect_state>(), 32);
        assert_eq!(std::mem::size_of::<fc4_prop_effect_result>(), 32);

        let initial = (state, words, index, result);
        for slot in [0, 13, u32::MAX] {
            assert!(!fc4_prop_developer_shadow_effect(
                &mut state,
                slot,
                [0; 4].as_ptr(),
                100,
                words.as_mut_ptr(),
                &mut index,
                &mut result
            ));
            assert_eq!((state, words, index, result), initial);
        }
        index = 625;
        assert!(!fc4_prop_developer_shadow_effect(
            &mut state,
            16,
            [0; 4].as_ptr(),
            100,
            words.as_mut_ptr(),
            &mut index,
            &mut result
        ));
        assert_eq!(state, initial.0);
        assert!(!fc4_prop_developer_shadow_effect(
            &mut state,
            16,
            std::ptr::null(),
            100,
            words.as_mut_ptr(),
            &mut index,
            &mut result
        ));
    }
}
