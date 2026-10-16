// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The UI window layer over the C ABI (docs/specs/ui-windows.md).
//!
//! The host implements [`fc4_uiwin_host`]; a glue lane's `cSC4View3DWin` and
//! `cIGZWin` shim forwards its slots here. The window tree, the catalog menus
//! and the dialogs stay host objects, named by tokens (0 = none).
//!
//! Threads: the core is single-threaded. The window tree is main-thread UI
//! state driven from the message loop (HANDOFF §5.1); the plugin serialises
//! every call on one object (fc4.h).
//!
//! # What the exports pin
//!
//! The layout facts of §3.2 are exported as functions rather than left in a
//! header, so a conformance vector can assert them against this crate and a
//! glue lane cannot drift from them:
//!
//! - [`fc4_uiwin_as_igzwin`] is the measurement — `this + 0x0C`, or 0 for a
//!   null `this` (conformance test 1);
//! - [`fc4_uiwin_vtable_primary`] and [`fc4_uiwin_vtable_window`] are the two
//!   vtables of one allocation (test 2);
//! - [`fc4_uiwin_slot_ret`] is the return channel of every `cISC4View3DWin`
//!   slot, so slots 59 and 60 answer "none" (test 3).

use std::ffi::c_void;

use fc4_core::ui_windows::catalog::{CatalogHost, InvokeResult};
use fc4_core::ui_windows::dialog::{popup_win_proc, AutoGamePause, HiddenPause, PopupAction};
use fc4_core::ui_windows::message::GzMessage;
use fc4_core::ui_windows::slots::{
    as_igzwin, RetChannel, IID_CATALOG_STATE, VIEW3D_OFF_WIN, VIEW3D_SLOT_COUNT, VIEW3D_SLOTS,
    VIEW3D_VTABLE, WIN_GEN_VTABLE,
};
use fc4_core::ui_windows::view3d::{CatalogState, View3DWin, View3DSave};
use fc4_core::ui_windows::win::{WinStore, WinToken};

use crate::guard;

type U = *mut c_void;

#[repr(C)]
pub struct fc4_uiwin {
    _private: [u8; 0],
}

/// What the window layer needs from the game (ui-windows.md §4.3, §5.1).
///
/// A missing callback reads as 0 / false / none. Nothing here interprets a UI
/// script: §13 says the only thing the window layer needs is "create a window
/// whose parent is X and whose group/instance are G/I".
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_uiwin_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// `SC4CreateWindowFromScript` (§5.1) — the 3-argument thunk to
    /// `cGZUIScriptService` **slot 4**. Returns the window, or 0 when the
    /// script service is missing.
    ///
    /// Quirk 10: do **not** route this to the method named
    /// `CreateWindowFromScript` at `0x0094B08B` — that is a 5-argument QI
    /// wrapper returning `AL` that releases the window it just made.
    pub create_window_from_script:
        Option<unsafe extern "C" fn(U, u32, u64, u32, *mut u64) -> bool>,
    /// `QueryInterface(0x8A2CAD8B, &p)` (§4.1): the **2-argument** form on
    /// slot 0. Writes the interface handle to `out`.
    pub query_catalog_iface: Option<unsafe extern "C" fn(U, u64, u32, *mut u64) -> bool>,
    /// The catalog interface's slot 17 (§4.1) — written to `state+0`.
    pub catalog_iface_slot17: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// Its slot 9 — written to `state+4`.
    pub catalog_iface_slot9: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// Its `Release` (slot 2), after the two reads.
    pub catalog_iface_release: Option<unsafe extern "C" fn(U, u64)>,
    /// `cIGZWinGen::SetWinGenFlag(flag, value)` (gen slot 6, §4.3).
    pub set_win_gen_flag: Option<unsafe extern "C" fn(U, u64, u32, bool)>,
    /// `nSC4UI::AlignWindowToWindow(winID, anchor, targetWinID, flags)` at
    /// `0x0078DB20` (§4.3).
    pub align_window_to_window: Option<unsafe extern "C" fn(U, u32, u32, u32, u32)>,
}

/// Every field of §3.2 plus the menu state a vector reads (`fc4_uiwin_get_state`).
///
/// Laid out so a 32-bit build has no padding: the four `u64`s come first after
/// the two leading `u32`s, then the `u32` fields. Size 72.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_uiwin_state {
    pub struct_size: u32,
    /// The object's synthetic address, so `AsIGZWin` is checkable.
    pub this_addr: u32,
    /// `AsIGZWin()` = the `cIGZWin` sub-object token at `+0x0C`.
    pub win: u64,
    /// `+0x1F8`.
    pub secondary_win: u64,
    /// `+0x1FC`.
    pub tertiary_win: u64,
    /// `+0x300`.
    pub view_manager: u64,
    /// `+0x200`.
    pub secondary_kind: u32,
    /// `+0x204`.
    pub tertiary_kind: u32,
    /// `+0x208` as a raw pointer: 0 when the field is null.
    pub catalog_state_ptr: u32,
    /// `state+0`, the slot-17 result.
    pub catalog_field0: u32,
    /// `state+4`, the slot-9 result.
    pub catalog_field1: u32,
    /// The current tool (slot 24), 0 = none.
    pub input_control: u32,
    pub refs: i32,
    /// The hidden-pause nesting count (§6.4).
    pub hidden_pause_count: u32,
}

/// One `cISC4View3DWin` slot's ABI facts (ui-windows.md §3.3).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_uiwin_slot {
    pub struct_size: u32,
    pub slot: u32,
    pub offset: u32,
    pub target: u32,
    pub pops: u32,
    pub args: u32,
    /// [`RetChannel`] as a code: 0 = `al`, 1 = `eax`, 2 = `eax-or-void`,
    /// 3 = `none`, 4 = `x87`.
    pub ret: u32,
    /// 1 when a caller may read a value out of this slot (test 3).
    pub produces_value: u32,
}

/// The View3D save record (§3.3a): the only thing the window layer persists.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_uiwin_save {
    pub struct_size: u32,
    pub group: u32,
    pub type_id: u32,
    pub instance: u32,
    /// The further integer pushed alongside. *[inferred]* a record version.
    pub version: u32,
    /// `this+0xF0`'s class id, 0 when the reference is null.
    pub ref_f0_class: u32,
    pub ref_f0_present: u32,
    /// `this+0xF4`'s class id, written as `obj + 8` when non-null else null.
    pub ref_f4_class: u32,
    pub ref_f4_present: u32,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_uiwin_state>() == 72);
    assert!(std::mem::size_of::<fc4_uiwin_slot>() == 32);
    assert!(std::mem::size_of::<fc4_uiwin_save>() == 36);
    assert!(
        std::mem::size_of::<fc4_uiwin_host>()
            == if cfg!(target_pointer_width = "32") { 36 } else { 68 }
    );
};

// ---- host adapter --------------------------------------------------------------------------

macro_rules! cb {
    ($t:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $t.$f {
            Some(f) => unsafe { f($t.user $(, $arg)*) },
            None => $default,
        }
    };
}

struct Handle {
    view: View3DWin,
    store: WinStore,
    pause: HiddenPause,
    h: fc4_uiwin_host,
}

/// The host table on its own, so a call can borrow it and the view/store
/// fields of a [`Handle`] at the same time.
struct HostRef<'a>(&'a mut fc4_uiwin_host);

impl CatalogHost for HostRef<'_> {
    fn create_window_from_script(
        &mut self,
        group: u32,
        parent: WinToken,
        instance: u32,
    ) -> Option<WinToken> {
        let mut out: u64 = 0;
        let ok = cb!(self.0, create_window_from_script, false, group, parent, instance, &mut out);
        (ok && out != 0).then_some(out)
    }

    fn query_catalog_iface(&mut self, win: WinToken, iid: u32) -> Option<u64> {
        let mut out: u64 = 0;
        let ok = cb!(self.0, query_catalog_iface, false, win, iid, &mut out);
        (ok && out != 0).then_some(out)
    }

    fn catalog_iface_slot17(&mut self, iface: u64) -> u32 {
        cb!(self.0, catalog_iface_slot17, 0, iface)
    }

    fn catalog_iface_slot9(&mut self, iface: u64) -> u32 {
        cb!(self.0, catalog_iface_slot9, 0, iface)
    }

    fn catalog_iface_release(&mut self, iface: u64) {
        cb!(self.0, catalog_iface_release, (), iface);
    }

    fn set_win_gen_flag(&mut self, win: WinToken, flag: u32, value: bool) {
        cb!(self.0, set_win_gen_flag, (), win, flag, value);
    }

    fn align_window_to_window(&mut self, win_id: u32, anchor: u32, target: u32, flags: u32) {
        cb!(self.0, align_window_to_window, (), win_id, anchor, target, flags);
    }
}

impl Handle {
    fn close_secondary(&mut self) {
        let Handle { view, store, h: tbl, .. } = self;
        let mut host = HostRef(tbl);
        fc4_core::ui_windows::catalog::close_secondary_menu(&mut host, view, store);
    }

    fn close_tertiary(&mut self) {
        let Handle { view, store, h: tbl, .. } = self;
        let mut host = HostRef(tbl);
        fc4_core::ui_windows::catalog::close_tertiary_menu(&mut host, view, store);
    }

    fn invoke_secondary(
        &mut self,
        instance: u32,
        kind: u32,
        caller_data: u32,
        target_win_id: u32,
    ) -> InvokeResult {
        let Handle { view, store, h: tbl, .. } = self;
        let mut host = HostRef(tbl);
        let args = fc4_core::ui_windows::catalog::SecondaryInvoke {
            instance,
            kind,
            caller_data,
            target_win_id,
        };
        fc4_core::ui_windows::catalog::invoke_secondary_menu(&mut host, view, store, args)
    }

    fn invoke_tertiary(
        &mut self,
        instance: u32,
        kind: u32,
        caller_data: u32,
        target_win_id: u32,
        margin: i32,
    ) -> InvokeResult {
        let Handle { view, store, h: tbl, .. } = self;
        let mut host = HostRef(tbl);
        let args = fc4_core::ui_windows::catalog::TertiaryInvoke {
            instance,
            kind,
            caller_data,
            target_win_id,
            margin,
        };
        fc4_core::ui_windows::catalog::invoke_tertiary_menu(&mut host, view, store, args)
    }
}

/// SAFETY: `u` must be NULL or a live pointer from `fc4_uiwin_create`.
unsafe fn handle<'a>(u: *mut fc4_uiwin) -> Option<&'a mut Handle> {
    (u as *mut Handle).as_mut()
}

fn opt(t: u64) -> Option<WinToken> {
    (t != 0).then_some(t)
}

/// `guard` over a closure that may bail with `?`.
///
/// A null handle, a null out-pointer or a bad slot all answer `fallback`, and
/// so does a panic — which is the documented contract of every entry point.
fn try_guard<R: Copy>(fallback: R, f: impl FnOnce() -> Option<R>) -> R {
    guard(None, f).unwrap_or(fallback)
}

fn ret_code(r: RetChannel) -> u32 {
    match r {
        RetChannel::Al => 0,
        RetChannel::Eax => 1,
        RetChannel::EaxOrVoid => 2,
        RetChannel::None => 3,
        RetChannel::X87 => 4,
    }
}

// ---- layout (ui-windows.md §3.2, conformance tests 1 and 2) ---------------------------------

/// `cSC4View3DWin::AsIGZWin` (`0x005F8400`, slot 3): `this + 0x0C`, or 0 for a
/// null `this`.
///
/// Quirk 4: the only null check is on `this` itself. A non-null result is
/// **not** proof of a valid window — use `cIGZWinMgr::IsWindowValid` (slot 24).
#[no_mangle]
pub extern "C" fn fc4_uiwin_as_igzwin(this_addr: u32) -> u32 {
    as_igzwin(this_addr)
}

/// The offset `AsIGZWin` adds: `0x0C`.
#[no_mangle]
pub extern "C" fn fc4_uiwin_win_offset() -> u32 {
    VIEW3D_OFF_WIN
}

/// The `cISC4View3DWin` vtable of a `cSC4View3DWin` (`0x00ABCE40`).
#[no_mangle]
pub extern "C" fn fc4_uiwin_vtable_primary() -> u32 {
    VIEW3D_VTABLE
}

/// The `cGZWinGen` vtable of the window sub-object at `+0x0C` (`0x00ADC678`).
#[no_mangle]
pub extern "C" fn fc4_uiwin_vtable_window() -> u32 {
    WIN_GEN_VTABLE
}

// ---- the slot table (§3.3, conformance test 3) ----------------------------------------------

/// `cISC4View3DWin`'s slot count: 61, slots 0-60.
#[no_mangle]
pub extern "C" fn fc4_uiwin_slot_count() -> u32 {
    VIEW3D_SLOT_COUNT
}

/// One slot's ABI facts. Returns false for a slot that does not exist.
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_slot_info(slot: u32, out: *mut fc4_uiwin_slot) -> bool {
    try_guard(false, || {
        let s = VIEW3D_SLOTS.get(slot as usize)?;
        let o = out.as_mut()?;
        *o = fc4_uiwin_slot {
            struct_size: std::mem::size_of::<fc4_uiwin_slot>() as u32,
            slot: s.slot,
            offset: s.offset,
            target: s.target,
            pops: s.pops,
            args: s.args,
            ret: ret_code(s.ret),
            produces_value: u32::from(s.ret.produces_value()),
        };
        Some(true)
    })
}

/// Whether a caller may read a value out of a slot.
///
/// Conformance test 3: `CloseSecondaryMenu` (59) and `CloseTertiaryMenu` (60)
/// return **nothing**, so both answer false and a caller must not read EAX.
#[no_mangle]
pub extern "C" fn fc4_uiwin_slot_produces_value(slot: u32) -> bool {
    VIEW3D_SLOTS
        .get(slot as usize)
        .map(|s| s.ret.produces_value())
        .unwrap_or(false)
}

// ---- lifecycle ------------------------------------------------------------------------------

/// Create a view at a given object address.
///
/// The `cIGZWin` sub-object's token is **derived**, not supplied: it is
/// `AsIGZWin(this_addr)` = `this_addr + 0x0C`, which is the measurement of
/// §3.2. The host names its own window by that token, so a caller cannot
/// disagree with the layout (conformance test 1).
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_create(
    host: *const fc4_uiwin_host,
    this_addr: u32,
) -> *mut fc4_uiwin {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_uiwin_host>() => {
            let mut store = WinStore::new();
            // The measured address arithmetic is 32-bit (it is `lea eax,
            // [ecx+0xc]`); a token is a `u64` so a host can name a window
            // without colliding with a raw pointer.
            let win = u64::from(as_igzwin(this_addr));
            store.adopt(win, 0, 0);
            let handle = Handle {
                view: View3DWin::new(this_addr, win),
                store,
                pause: HiddenPause::new(),
                h: *h,
            };
            Box::into_raw(Box::new(handle)) as *mut fc4_uiwin
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_destroy(u: *mut fc4_uiwin) {
    if !u.is_null() {
        guard((), || drop(Box::from_raw(u as *mut Handle)));
    }
}

/// Every field of §3.2 plus the menu state.
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_get_state(u: *mut fc4_uiwin, out: *mut fc4_uiwin_state) -> bool {
    try_guard(false, || {
        let h = handle(u)?;
        let o = out.as_mut()?;
        let cs = h.view.catalog_state();
        *o = fc4_uiwin_state {
            struct_size: std::mem::size_of::<fc4_uiwin_state>() as u32,
            this_addr: h.view.this_addr(),
            win: h.view.as_igzwin(),
            secondary_win: h.view.secondary_win().unwrap_or(0),
            tertiary_win: h.view.tertiary_win().unwrap_or(0),
            view_manager: h.view.view_manager().unwrap_or(0),
            secondary_kind: h.view.secondary_kind(),
            tertiary_kind: h.view.tertiary_kind(),
            catalog_state_ptr: h.view.catalog_state_ptr(),
            catalog_field0: cs.map(|c| c.field0).unwrap_or(0),
            catalog_field1: cs.map(|c| c.field1).unwrap_or(0),
            input_control: h.view.input_control().unwrap_or(0),
            refs: h.view.refs(),
            hidden_pause_count: h.pause.count(),
        };
        Some(true)
    })
}

/// Seed a window into the store so a vector can open a menu on it.
///
/// The host owns the real `cIGZWin`; this names it. Returns the token.
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_add_window(u: *mut fc4_uiwin, id: u32) -> u64 {
    try_guard(0, || {
        let h = handle(u)?;
        Some(h.store.create(id))
    })
}

/// Set `+0x208` to a catalog state, or clear it with `present = 0`.
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_set_catalog_state(
    u: *mut fc4_uiwin,
    present: bool,
    field0: u32,
    field1: u32,
) -> bool {
    try_guard(false, || {
        let h = handle(u)?;
        h.view
            .set_catalog_state(present.then_some(CatalogState { field0, field1 }));
        Some(true)
    })
}

/// Give the view a current tool (slot 24), or clear it with 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_set_input_control(u: *mut fc4_uiwin, control: u32) -> bool {
    try_guard(false, || {
        let h = handle(u)?;
        h.view.set_input_control(control);
        Some(true)
    })
}

// ---- the catalog menus (§4, conformance tests 4-19) -----------------------------------------

/// `CloseTertiaryMenu` (slot 60). Returns **nothing**, like the slot.
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_close_tertiary(u: *mut fc4_uiwin) {
    guard((), || {
        if let Some(h) = handle(u) {
            h.close_tertiary();
        }
    })
}

/// `CloseSecondaryMenu` (slot 59). Cascades to slot 60 first (§4.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_close_secondary(u: *mut fc4_uiwin) {
    guard((), || {
        if let Some(h) = handle(u) {
            h.close_secondary();
        }
    })
}

fn invoke_result_code(r: InvokeResult) -> u32 {
    match r {
        InvokeResult::Toggled => 1,
        InvokeResult::ReplacedThenFailed => 2,
        InvokeResult::Opened(_) => 3,
    }
}

/// `InvokeSecondaryMenu` (`0x007E5C10`): 4 arguments.
///
/// Returns 1 = toggled closed, 2 = replaced then the create failed, 3 =
/// opened. The tool is dropped **before** the open-menu test either way
/// (§4.3, quirk 3).
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_invoke_secondary(
    u: *mut fc4_uiwin,
    instance: u32,
    kind: u32,
    caller_data: u32,
    target_win_id: u32,
) -> u32 {
    try_guard(0, || {
        let h = handle(u)?;
        Some(invoke_result_code(h.invoke_secondary(instance, kind, caller_data, target_win_id)))
    })
}

/// `InvokeTertiaryMenu` (`0x007E5D80`): 5 arguments.
///
/// `margin` is arg5 — the small negative integer (−25 / −75) the secondary
/// form lacks. It does **not** reach `AlignWindowToWindow` (§4.4); this core
/// records it and does not invent a meaning for it (UIW-2).
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_invoke_tertiary(
    u: *mut fc4_uiwin,
    instance: u32,
    kind: u32,
    caller_data: u32,
    target_win_id: u32,
    margin: i32,
) -> u32 {
    try_guard(0, || {
        let h = handle(u)?;
        Some(invoke_result_code(h.invoke_tertiary(
            instance,
            kind,
            caller_data,
            target_win_id,
            margin,
        )))
    })
}

// ---- messages (§6.5, conformance tests 21 and 22) --------------------------------------------

/// Deliver one message through the §6.5 chain and report the stages reached.
///
/// Returns a bitmask: 1 = `DoWinProcMessage`, 2 = `DoWinMsg`, 4 = `DoMessage`,
/// 8 = `GZOnCommand`. A type-3 message that nothing handled reaches all four,
/// in that order (conformance test 22).
///
/// The window-procedure entry takes the **window** as arg1 (§6.5); this
/// signature cannot get that wrong (conformance test 21).
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_deliver(
    u: *mut fc4_uiwin,
    win: u64,
    msg_kind: u32,
    data1: u32,
    data2: u32,
    data3: u32,
) -> u32 {
    try_guard(0, || {
        let h = handle(u)?;
        let msg = GzMessage {
            kind: msg_kind_of(msg_kind)?,
            msg_type: data1,
            data1,
            data2,
            data3,
        };
        let stages = fc4_core::ui_windows::message::deliver(
            &mut h.store,
            opt(win).unwrap_or(h.view.as_igzwin()),
            &msg,
            |_, _| fc4_core::ui_windows::message::Handled::No,
            |_, _| fc4_core::ui_windows::message::Handled::No,
            |_, _| fc4_core::ui_windows::message::Handled::No,
        );
        Some(stages.iter().fold(0u32, |acc, s| {
            acc | match *s {
                "DoWinProcMessage" => 1,
                "DoWinMsg" => 2,
                "DoMessage" => 4,
                "GZOnCommand" => 8,
                _ => 0,
            }
        }))
    })
}

/// The message kinds of §6.5, as a code for [`fc4_uiwin_deliver`].
pub mod msg_kind_code {
    pub const COMMAND: u32 = 0;
    pub const LINK: u32 = 1;
    pub const ATTACH: u32 = 2;
    pub const DETACH: u32 = 3;
    pub const MODAL_BEGIN: u32 = 4;
    pub const MODAL_END: u32 = 5;
    pub const MODAL_END_POSTED: u32 = 6;
    pub const KEY_DOWN: u32 = 7;
    pub const KEY_UP: u32 = 8;
}

fn msg_kind_of(code: u32) -> Option<fc4_core::ui_windows::message::MsgKind> {
    use fc4_core::ui_windows::message::MsgKind;
    Some(match code {
        msg_kind_code::COMMAND => MsgKind::Command,
        msg_kind_code::LINK => MsgKind::Link,
        msg_kind_code::ATTACH => MsgKind::AttachNotice,
        msg_kind_code::DETACH => MsgKind::DetachNotice,
        msg_kind_code::MODAL_BEGIN => MsgKind::ModalBegin,
        msg_kind_code::MODAL_END => MsgKind::ModalEnd,
        msg_kind_code::MODAL_END_POSTED => MsgKind::ModalEndPosted,
        msg_kind_code::KEY_DOWN => MsgKind::KeyDown,
        msg_kind_code::KEY_UP => MsgKind::KeyUp,
        _ => return None,
    })
}

// ---- dialogs (§6, conformance tests 20 and 23) -----------------------------------------------

/// What the advisor popup's win proc would do with this message (§6.2).
///
/// Returns [`PopupAction`] as a code: 0 = keep open, 1 = end, 2 = follow the
/// link then end, 3 = swallow, 4 = ignored.
///
/// Conformance test 20: a button command with **any** ID ends it, and a key
/// message is **ignored** — that is why Esc and Enter do nothing.
#[no_mangle]
pub extern "C" fn fc4_uiwin_popup_action(
    advice_flags: u32,
    msg_kind: u32,
    data1: u32,
    data2: u32,
    data3: u32,
) -> u32 {
    try_guard(4, || {
        let msg = GzMessage {
            kind: msg_kind_of(msg_kind)?,
            msg_type: data1,
            data1,
            data2,
            data3,
        };
        Some(match popup_win_proc(advice_flags, &msg) {
            PopupAction::KeepOpen => 0,
            PopupAction::End => 1,
            PopupAction::FollowLinkThenEnd => 2,
            PopupAction::Swallow => 3,
            PopupAction::Ignored => 4,
        })
    })
}

/// Which UI script instance an advisor popup is built from (§6.1 step 3).
#[no_mangle]
pub extern "C" fn fc4_uiwin_advisor_popup_instance(mood: i32) -> u32 {
    fc4_core::ui_windows::dialog::advisor_popup_instance(mood)
}

/// Take a scoped hidden pause. `is_dialog` marks a dialog's pause, so a
/// vector can show the count is not a popup count (§6.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_pause_enter(u: *mut fc4_uiwin, is_dialog: bool) {
    guard((), || {
        if let Some(h) = handle(u) {
            h.pause.enter(AutoGamePause::advisor_popup(), is_dialog);
        }
    })
}

/// Release one level of the hidden pause.
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_pause_leave(u: *mut fc4_uiwin, is_dialog: bool) {
    guard((), || {
        if let Some(h) = handle(u) {
            h.pause.leave(AutoGamePause::advisor_popup(), is_dialog);
        }
    })
}

/// `IsHiddenPaused()`: the nesting count at `+0x34` is > 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_is_hidden_paused(u: *mut fc4_uiwin) -> bool {
    try_guard(false, || Some(handle(u)?.pause.is_hidden_paused()))
}

/// `IsPaused()`: **false** while a hidden pause is in force (§6.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_is_paused(u: *mut fc4_uiwin) -> bool {
    try_guard(false, || Some(handle(u)?.pause.is_paused()))
}

// ---- save (§3.3a, conformance test 25) -------------------------------------------------------

/// `DoSave` (`0x007EA5B0`): the View3D record and **nothing else**.
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_save(u: *mut fc4_uiwin, out: *mut fc4_uiwin_save) -> bool {
    try_guard(false, || {
        let h = handle(u)?;
        let o = out.as_mut()?;
        let r: View3DSave = h.view.do_save();
        *o = fc4_uiwin_save {
            struct_size: std::mem::size_of::<fc4_uiwin_save>() as u32,
            group: r.group,
            type_id: r.type_id,
            instance: r.instance,
            version: r.version,
            ref_f0_class: r.ref_f0.class_id,
            ref_f0_present: u32::from(r.ref_f0.present),
            ref_f4_class: r.ref_f4.class_id,
            ref_f4_present: u32::from(r.ref_f4.present),
        };
        Some(true)
    })
}

/// `DoLoad` (`0x007EBFB0`): resolve the references by class id and leave every
/// UI field clear.
///
/// Quirk 13: a catalog popup open at save time is closed at load time. A
/// round trip must keep the camera-control / renderer / model-maker /
/// simulator references resolvable by class id.
#[no_mangle]
pub unsafe extern "C" fn fc4_uiwin_load(u: *mut fc4_uiwin, input: *const fc4_uiwin_save) -> bool {
    try_guard(false, || {
        let h = handle(u)?;
        let i = input.as_ref()?;
        let rec = View3DSave {
            group: i.group,
            type_id: i.type_id,
            instance: i.instance,
            version: i.version,
            ref_f0: fc4_core::ui_windows::view3d::SaveRef {
                class_id: i.ref_f0_class,
                present: i.ref_f0_present != 0,
            },
            ref_f4: fc4_core::ui_windows::view3d::SaveRef {
                class_id: i.ref_f4_class,
                present: i.ref_f4_present != 0,
            },
        };
        Some(h.view.do_load(&rec).is_ok())
    })
}

/// Whether the record carries any UI state — always false (§3.3a).
#[no_mangle]
pub extern "C" fn fc4_uiwin_save_carries_ui_state() -> bool {
    false
}

/// The catalog-state interface IID the tertiary close queries (`0x8A2CAD8B`).
#[no_mangle]
pub extern "C" fn fc4_uiwin_catalog_iface_iid() -> u32 {
    IID_CATALOG_STATE
}
