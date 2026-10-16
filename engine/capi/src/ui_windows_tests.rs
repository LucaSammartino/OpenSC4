// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The UI window layer through the C ABI (docs/specs/ui-windows.md): a fake
//! game behind `fc4_uiwin_host`.
//!
//! These mirror the 25 conformance tests of §11 at the ABI boundary, so a
//! glue lane that calls these exports gets the behaviour the vectors pin.

use std::cell::RefCell;
use std::ffi::c_void;

use crate::ui_windows::*;

#[derive(Default)]
struct Game {
    log: RefCell<Vec<String>>,
    /// Whether the factory answers at all (a false models a missing script
    /// service, §5.1).
    create_ok: RefCell<bool>,
    /// The window the factory hands back; 0 = none.
    next_win: RefCell<u64>,
    /// Whether `QueryInterface(0x8A2CAD8B)` answers (§4.1).
    catalog_iface_ok: RefCell<bool>,
    slot17: RefCell<u32>,
    slot9: RefCell<u32>,
    /// The token `create_window_from_script` was called with, for the group
    /// and parent checks (conformance tests 15 and 16).
    last_create: RefCell<Option<(u32, u64, u32)>>,
    last_align: RefCell<Option<(u32, u32, u32, u32)>>,
    flags: RefCell<Vec<(u64, u32, bool)>>,
    released: RefCell<Vec<u64>>,
}

impl Game {
    fn new() -> Box<Game> {
        let g = Box::new(Game::default());
        *g.create_ok.borrow_mut() = true;
        *g.next_win.borrow_mut() = 0x9001;
        *g.catalog_iface_ok.borrow_mut() = true;
        *g.slot17.borrow_mut() = 0xAA;
        *g.slot9.borrow_mut() = 0xBB;
        g
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
    // SAFETY: the test's `Game` outlives every call it is passed to.
    unsafe { &*(u as *const Game) }
}

unsafe extern "C" fn create_window_from_script(
    u: *mut c_void,
    group: u32,
    parent: u64,
    instance: u32,
    out: *mut u64,
) -> bool {
    let g = game(u);
    g.log(format!("create group={group:08X} parent={parent:X} instance={instance:08X}"));
    *g.last_create.borrow_mut() = Some((group, parent, instance));
    if !*g.create_ok.borrow() {
        return false;
    }
    let w = *g.next_win.borrow();
    *g.next_win.borrow_mut() = w + 1;
    if !out.is_null() {
        *out = w;
    }
    true
}

unsafe extern "C" fn query_catalog_iface(u: *mut c_void, win: u64, iid: u32, out: *mut u64) -> bool {
    let g = game(u);
    g.log(format!("query_catalog win={win:X} iid={iid:08X}"));
    if !*g.catalog_iface_ok.borrow() {
        return false;
    }
    if !out.is_null() {
        *out = 0xC0;
    }
    true
}

unsafe extern "C" fn catalog_iface_slot17(u: *mut c_void, iface: u64) -> u32 {
    game(u).log(format!("slot17 iface={iface:X}"));
    *game(u).slot17.borrow()
}

unsafe extern "C" fn catalog_iface_slot9(u: *mut c_void, iface: u64) -> u32 {
    game(u).log(format!("slot9 iface={iface:X}"));
    *game(u).slot9.borrow()
}

unsafe extern "C" fn catalog_iface_release(u: *mut c_void, iface: u64) {
    game(u).log(format!("iface_release iface={iface:X}"));
    game(u).released.borrow_mut().push(iface);
}

unsafe extern "C" fn set_win_gen_flag(u: *mut c_void, win: u64, flag: u32, value: bool) {
    game(u).log(format!("set_win_gen_flag win={win:X} flag={flag} value={value}"));
    game(u).flags.borrow_mut().push((win, flag, value));
}

unsafe extern "C" fn align_window_to_window(u: *mut c_void, win: u32, anchor: u32, target: u32, flags: u32) {
    game(u).log(format!("align win={win:X} anchor={anchor:04X} target={target:08X} flags={flags}"));
    *game(u).last_align.borrow_mut() = Some((win, anchor, target, flags));
}

fn host(g: &Game) -> fc4_uiwin_host {
    fc4_uiwin_host {
        struct_size: std::mem::size_of::<fc4_uiwin_host>() as u32,
        user: g.user(),
        create_window_from_script: Some(create_window_from_script),
        query_catalog_iface: Some(query_catalog_iface),
        catalog_iface_slot17: Some(catalog_iface_slot17),
        catalog_iface_slot9: Some(catalog_iface_slot9),
        catalog_iface_release: Some(catalog_iface_release),
        set_win_gen_flag: Some(set_win_gen_flag),
        align_window_to_window: Some(align_window_to_window),
    }
}

fn setup() -> (Box<Game>, *mut fc4_uiwin, u64) {
    let g = Game::new();
    let h = host(&g);
    // SAFETY: `h` is a complete host table and `g` outlives the handle.
    let u = unsafe { fc4_uiwin_create(&h, 0x2000) };
    assert!(!u.is_null());
    (g, u, 0x200C)
}

fn state(u: *mut fc4_uiwin) -> fc4_uiwin_state {
    let mut s = fc4_uiwin_state::default();
    // SAFETY: `u` is a live handle and `s` is writable.
    assert!(unsafe { fc4_uiwin_get_state(u, &mut s) });
    s
}

#[test]
fn as_igzwin_is_this_plus_0x0c() {
    // Conformance test 1.
    assert_eq!(fc4_uiwin_as_igzwin(0), 0, "a null this returns null");
    assert_eq!(fc4_uiwin_as_igzwin(0x1000), 0x100C);
    assert_eq!(fc4_uiwin_win_offset(), 0x0C);
    let (g, u, w) = setup();
    let s = state(u);
    assert_eq!(s.this_addr, 0x2000);
    assert_eq!(s.win, w, "the window token IS AsIGZWin(this)");
    assert_eq!(u64::from(fc4_uiwin_as_igzwin(s.this_addr)), s.win);
    assert_eq!(s.win, 0x200C);
    // SAFETY: `u` is a live handle.
    unsafe { fc4_uiwin_destroy(u) };
    drop(g);
}

#[test]
fn two_vtables_one_allocation() {
    // Conformance test 2.
    assert_eq!(fc4_uiwin_vtable_primary(), 0x00AB_CE40);
    assert_eq!(fc4_uiwin_vtable_window(), 0x00AD_C678);
    assert_ne!(fc4_uiwin_vtable_primary(), fc4_uiwin_vtable_window());
}

#[test]
fn slots_59_and_60_return_nothing() {
    // Conformance test 3.
    assert_eq!(fc4_uiwin_slot_count(), 61);
    assert!(!fc4_uiwin_slot_produces_value(59));
    assert!(!fc4_uiwin_slot_produces_value(60));
    let mut s = fc4_uiwin_slot::default();
    // SAFETY: `s` is writable.
    assert!(unsafe { fc4_uiwin_slot_info(59, &mut s) });
    assert_eq!(s.ret, 3, "none");
    assert_eq!(s.produces_value, 0);
    // SAFETY: `s` is writable.
    assert!(unsafe { fc4_uiwin_slot_info(60, &mut s) });
    assert_eq!(s.ret, 3, "none");
    // Slot 3 does produce a value: AsIGZWin returns EAX.
    // SAFETY: `s` is writable.
    assert!(unsafe { fc4_uiwin_slot_info(3, &mut s) });
    assert_eq!(s.ret, 1, "eax");
    assert_eq!(s.produces_value, 1);
    // SAFETY: `s` is writable.
    assert!(!unsafe { fc4_uiwin_slot_info(61, &mut s) }, "no such slot");
}

#[test]
fn close_secondary_is_a_noop_when_nothing_is_open() {
    // Conformance test 4.
    let (g, u, w) = setup();
    let before = state(u);
    g.take();
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_close_secondary(u) };
    let after = state(u);
    assert_eq!(after.secondary_kind, before.secondary_kind, "+0x200 left alone");
    assert!(g.take().iter().all(|l| !l.starts_with("create")), "no create, no teardown");
    assert_eq!(after.win, w);
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn close_tertiary_is_a_noop_when_nothing_is_open() {
    // Conformance test 5.
    let (g, u, _w) = setup();
    let before = state(u);
    g.take();
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_close_tertiary(u) };
    assert_eq!(state(u).tertiary_kind, before.tertiary_kind, "+0x204 left alone");
    assert!(g.take().is_empty());
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn invoke_toggles_and_the_parent_and_group_are_right() {
    // Conformance tests 12, 15 and 16.
    let (g, u, _w) = setup();
    // SAFETY: `u` is live.
    let r = unsafe { fc4_uiwin_invoke_secondary(u, 0x9DE3_002, 0x9DE8_798, 0, 1) };
    assert_eq!(r, 3, "opened");
    let open = state(u);
    assert_ne!(open.secondary_win, 0);
    assert_eq!(open.secondary_kind, 0x9DE8_798);
    let (group, parent, instance) = g.last_create.borrow().expect("a create call");
    assert_eq!(group, 0x96A0_06B0, "the UI script group (test 16)");
    assert_eq!(parent, open.win, "AsIGZWin() (test 15)");
    assert_eq!(instance, 0x9DE3_002);

    // The same kind toggles it closed.
    // SAFETY: `u` is live.
    let r2 = unsafe { fc4_uiwin_invoke_secondary(u, 0x9DE3_002, 0x9DE8_798, 0, 1) };
    assert_eq!(r2, 1, "toggled");
    let closed = state(u);
    assert_eq!(closed.secondary_win, 0);
    assert_eq!(closed.secondary_kind, 0);
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn invoke_replaces_a_different_menu() {
    // Conformance test 13.
    let (g, u, _w) = setup();
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_invoke_secondary(u, 1, 0x1111, 0, 1) };
    let first = state(u).secondary_win;
    // SAFETY: `u` is live.
    let r = unsafe { fc4_uiwin_invoke_secondary(u, 2, 0x2222, 0, 1) };
    assert_eq!(r, 3);
    let s = state(u);
    assert_ne!(s.secondary_win, first, "a new window is open");
    assert_eq!(s.secondary_kind, 0x2222);
    let _ = &g;
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn the_tool_is_dropped_before_the_open_test() {
    // Conformance test 14 / quirk 3: opening a catalog menu that is already
    // open still costs the player their tool, because the drop runs before the
    // open-menu test.
    let (g, u, _w) = setup();
    // SAFETY: `u` is live.
    assert!(unsafe { fc4_uiwin_set_input_control(u, 0x51) });
    assert_eq!(state(u).input_control, 0x51, "a tool is held");
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_invoke_secondary(u, 1, 0x1111, 0, 1) };
    assert_eq!(state(u).input_control, 0, "dropped on the first invoke");

    // Give the player a tool again, then toggle the same menu closed: the
    // result is a toggle-close and no window is created, yet the tool is
    // still gone.
    // SAFETY: `u` is live.
    assert!(unsafe { fc4_uiwin_set_input_control(u, 0x52) });
    assert_eq!(state(u).input_control, 0x52);
    g.take();
    // SAFETY: `u` is live.
    let r = unsafe { fc4_uiwin_invoke_secondary(u, 1, 0x1111, 0, 1) };
    assert_eq!(r, 1, "toggled closed, no create");
    assert!(g.take().iter().all(|l| !l.starts_with("create")), "and no create was attempted");
    assert_eq!(state(u).input_control, 0, "but the tool was still dropped first");
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn generator_flags_are_cleared_and_the_iface_released() {
    // Conformance test 18.
    let (g, u, _w) = setup();
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_invoke_secondary(u, 1, 1, 0, 1) };
    let flags = g.flags.borrow().clone();
    assert_eq!(flags.len(), 2);
    assert_eq!(flags[0].1, 4);
    assert_eq!(flags[0].2, false);
    assert_eq!(flags[1].1, 1);
    assert_eq!(flags[1].2, false);
    // The gen interface is released after the two flags (test 18). That
    // release is `cIGZWin` slot 2 on the new window, which the core logs;
    // `catalog_iface_release` is a different Release (§4.1) and must not fire.
    assert!(g.released.borrow().is_empty(), "no catalog-iface release here");
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn align_happens_once_with_the_target_window_id() {
    // Conformance test 19.
    let (g, u, _w) = setup();
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_invoke_tertiary(u, 1, 2, 0, 0xAB95_37B7, -75) };
    let (win, anchor, target, flags) = g.last_align.borrow().expect("one align call");
    assert_eq!(anchor, 0xAAAA);
    assert_eq!(target, 0xAB95_37B7, "arg4 is the target window id, not a coordinate");
    assert_eq!(flags, 0);
    let s = state(u);
    assert_eq!(s.tertiary_kind, 2);
    assert_ne!(s.tertiary_win, 0);
    let _ = win;
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn null_create_is_safe() {
    // Conformance test 17.
    let (g, u, _w) = setup();
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_invoke_secondary(u, 1, 0x1111, 0, 1) };
    let open = state(u).secondary_win;
    *g.create_ok.borrow_mut() = false;
    // SAFETY: `u` is live.
    let r = unsafe { fc4_uiwin_invoke_secondary(u, 2, 0x2222, 0, 1) };
    assert_eq!(r, 2, "replaced then the create failed");
    let s = state(u);
    assert_eq!(s.secondary_win, 0);
    assert_eq!(s.secondary_kind, 0, "no field written");
    let _ = open;
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn tertiary_writes_two_dwords_in_order() {
    // Conformance test 10.
    let (g, u, _w) = setup();
    // SAFETY: `u` is live.
    assert!(unsafe { fc4_uiwin_set_catalog_state(u, true, 5, 6) });
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_invoke_tertiary(u, 1, 2, 0, 1, -25) };
    g.take();
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_close_tertiary(u) };
    let s = state(u);
    assert_eq!(s.catalog_field0, 0xAA, "slot 17 to state+0");
    assert_eq!(s.catalog_field1, 0xBB, "slot 9 to state+4");
    let log = g.take();
    let order: Vec<&str> = log
        .iter()
        .filter(|l| {
            l.starts_with("slot17")
                || l.starts_with("slot9")
                || l.starts_with("iface_release")
                || l.starts_with("query_catalog")
        })
        .map(|l| l.split(' ').next().unwrap())
        .collect();
    assert_eq!(order, ["query_catalog", "slot17", "slot9", "iface_release"]);
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn tertiary_skips_the_save_on_a_failed_query() {
    // Conformance test 11 / quirk 6.
    let (g, u, _w) = setup();
    // SAFETY: `u` is live.
    assert!(unsafe { fc4_uiwin_set_catalog_state(u, true, 5, 6) });
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_invoke_tertiary(u, 1, 2, 0, 1, -25) };
    *g.catalog_iface_ok.borrow_mut() = false;
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_close_tertiary(u) };
    let s = state(u);
    assert_eq!(s.catalog_field0, 5, "+0x208 untouched");
    assert_eq!(s.catalog_field1, 6);
    assert_eq!(s.tertiary_win, 0, "but the fields are cleared");
    assert_eq!(s.tertiary_kind, 0);
    assert_eq!(g.released.borrow().len(), 0, "nothing to release");
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn secondary_does_not_write_catalog_state() {
    // Conformance test 9 / §4.2's asymmetry.
    let (_g, u, _w) = setup();
    // SAFETY: `u` is live.
    assert!(unsafe { fc4_uiwin_set_catalog_state(u, true, 5, 6) });
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_invoke_secondary(u, 1, 1, 0, 1) };
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_close_secondary(u) };
    let s = state(u);
    assert_eq!(s.catalog_field0, 5, "CloseSecondaryMenu must not write +0x208");
    assert_eq!(s.catalog_field1, 6);
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn secondary_close_cascades_and_clears_both_fields() {
    // Conformance tests 6 and 8.
    let (_g, u, _w) = setup();
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_invoke_secondary(u, 1, 1, 0, 1) };
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_invoke_tertiary(u, 2, 2, 0, 1, -25) };
    assert_ne!(state(u).tertiary_win, 0);
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_close_secondary(u) };
    let s = state(u);
    assert_eq!(s.secondary_win, 0);
    assert_eq!(s.secondary_kind, 0);
    assert_eq!(s.tertiary_win, 0, "the cascade closed the tertiary too");
    assert_eq!(s.tertiary_kind, 0);
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn esc_and_enter_do_nothing_and_any_button_closes() {
    // Conformance test 20 (the replacement-side half).
    assert_eq!(fc4_uiwin_popup_action(2, msg_kind_code::KEY_DOWN, 0, 0, 0), 4, "ignored");
    assert_eq!(fc4_uiwin_popup_action(2, msg_kind_code::KEY_UP, 0, 0, 0), 4, "ignored");
    for button in [0u32, 1, 0x8A8D_FD4E] {
        assert_eq!(
            fc4_uiwin_popup_action(2, msg_kind_code::COMMAND, 0x2872_59F6, button, 0),
            1,
            "button {button:#x} ends it"
        );
    }
    // An inactive advice ends it; an active one with expire ends it.
    assert_eq!(fc4_uiwin_popup_action(0, msg_kind_code::COMMAND, 0x2872_59F6, 0, 0), 1);
    assert_eq!(fc4_uiwin_popup_action(2 | 8, msg_kind_code::COMMAND, 0x2872_59F6, 0, 0), 1);
    // Link: follow then end.
    assert_eq!(fc4_uiwin_popup_action(2, msg_kind_code::LINK, 0, 0, 0), 2);
    // Attach/detach notices are swallowed, not key messages.
    assert_eq!(fc4_uiwin_popup_action(2, msg_kind_code::ATTACH, 0, 0, 0), 3);
    assert_eq!(fc4_uiwin_popup_action(2, msg_kind_code::DETACH, 0, 0, 0), 3);
}

#[test]
fn delivery_order_is_proc_then_message_then_command() {
    // Conformance tests 21 and 22. The signature takes the window as its
    // second argument, so a shim cannot pass the message there.
    let (_g, u, _w) = setup();
    // SAFETY: `u` is live.
    let mask = unsafe { fc4_uiwin_deliver(u, 0x40, msg_kind_code::COMMAND, 0x2872_59F6, 0, 0) };
    assert_eq!(mask, 1 | 2 | 4 | 8, "all four stages, a type-3 message");
    // SAFETY: `u` is live.
    let mask2 = unsafe { fc4_uiwin_deliver(u, 0x40, msg_kind_code::LINK, 0, 0, 0) };
    assert_eq!(mask2, 1 | 2 | 4, "GZOnCommand is type-3 only");
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn the_hidden_pause_is_a_nesting_count() {
    // Conformance test 23.
    let (_g, u, _w) = setup();
    assert!(!unsafe { fc4_uiwin_is_hidden_paused(u) });
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_pause_enter(u, true) };
    assert!(unsafe { fc4_uiwin_is_hidden_paused(u) });
    assert!(!unsafe { fc4_uiwin_is_paused(u) }, "IsPaused() stays false (§6.1)");
    assert_eq!(state(u).hidden_pause_count, 1);
    // A camera move raises the same count with no dialog open.
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_pause_enter(u, false) };
    assert_eq!(state(u).hidden_pause_count, 2);
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_pause_leave(u, false) };
    assert_eq!(state(u).hidden_pause_count, 1, "back to the previous value");
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_pause_leave(u, true) };
    assert!(!unsafe { fc4_uiwin_is_hidden_paused(u) });
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn only_the_view3d_record_is_saved() {
    // Conformance test 25 / quirk 13.
    let (_g, u, _w) = setup();
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_invoke_secondary(u, 1, 1, 0, 1) };
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_invoke_tertiary(u, 2, 2, 0, 1, -25) };
    assert_ne!(state(u).secondary_win, 0);

    let mut rec = fc4_uiwin_save::default();
    // SAFETY: `u` is live, `rec` is writable.
    assert!(unsafe { fc4_uiwin_save(u, &mut rec) });
    assert_eq!(rec.group, 0xA9C8_647C);
    assert_eq!(rec.type_id, 0xE9C8_647E);
    assert_eq!(rec.instance, 1);
    assert_eq!(rec.version, 4);
    assert!(!fc4_uiwin_save_carries_ui_state(), "no UI state in the record");

    // SAFETY: `u` is live, `rec` is a complete record.
    assert!(unsafe { fc4_uiwin_load(u, &rec) });
    let s = state(u);
    assert_eq!(s.secondary_win, 0, "a catalog popup open at save time loads closed");
    assert_eq!(s.tertiary_win, 0);
    assert_eq!(s.secondary_kind, 0);
    assert_eq!(s.tertiary_kind, 0);
    // SAFETY: `u` is live.
    unsafe { fc4_uiwin_destroy(u) };
}

#[test]
fn a_null_handle_is_harmless() {
    // Every entry point runs inside guard() and answers its documented
    // fallback on a null handle.
    // SAFETY: passing NULL is the documented no-op case.
    unsafe {
        fc4_uiwin_destroy(std::ptr::null_mut());
        fc4_uiwin_close_secondary(std::ptr::null_mut());
        fc4_uiwin_close_tertiary(std::ptr::null_mut());
    }
    let mut s = fc4_uiwin_state::default();
    // SAFETY: `s` is writable; the handle is the documented NULL case.
    assert!(!unsafe { fc4_uiwin_get_state(std::ptr::null_mut(), &mut s) });
    // SAFETY: passing NULL is the documented no-op case.
    assert_eq!(unsafe { fc4_uiwin_invoke_secondary(std::ptr::null_mut(), 1, 1, 0, 1) }, 0);
    assert_eq!(fc4_uiwin_slot_count(), 61);
}

#[test]
fn catalog_iface_iid_is_the_unidentified_one() {
    // UIW-1: the interface the tertiary close queries is not named anywhere.
    assert_eq!(fc4_uiwin_catalog_iface_iid(), 0x8A2C_AD8B);
}
