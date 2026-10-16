// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Creating a scripted window (ui-windows.md §5.1).
//!
//! # `SC4CreateWindowFromScript` is a 3-argument thunk
//!
//! Three overloads exist (`0x005F9390`, `0x005F93F0`, `0x005F9480`); the one
//! the view calls is the first. Decompiled whole:
//!
//! ```c
//! undefined4 FUN_005f9390(undefined4 param_1, undefined4 param_2, undefined4 param_3)
//! {
//!   int *svc = 0;
//!   int *com = FUN_008793ec();                    // the framework / cGZCOM accessor
//!   if (com != 0)
//!       com->slot5(0x5A356E15, 0xFA3562FA, &svc); // GetSystemService(service, iid, &out)
//!   if (svc != 0) {
//!       u = svc->slot4(param_1, param_2, param_3); // cGZUIScriptService::CreateWindowFromScript
//!       FUN_004495c0();
//!       return u;
//!   }
//!   FUN_004495c0();
//!   return 0;
//! }
//! ```
//!
//! The work is `cGZUIScriptService` **slot 4**, three arguments, forwarded
//! unchanged, returning the window in EAX. It returns **0** when the script
//! service is missing, and quirk 8: it still runs the cleanup on that path —
//! the cleanup is not conditional on success.
//!
//! # The name-collision trap (quirk 10) — read this before binding the name
//!
//! The name table attaches `cGZUIScriptService::CreateWindowFromScript` to
//! **two** different functions, and **neither of them is slot 4**:
//!
//! | Address | Signature | Returns | What it does |
//! |---|---|---|---|
//! | `0x0094B08B` | 5 dwords (`ret 0x14`), thiscall | **bool in `AL`** | `w = this->slot4(p1,p2,p3);` if `w`: `ok = w->QueryInterface(p4,p5); w->Release();` return `ok` |
//! | `0x0094B210` | one struct pointer (reads `+0`,`+4`,`+8`) | — | a struct-taking overload |
//! | **slot 4** (unnamed) | 3 dwords | **`cIGZWin*` in EAX** | the actual factory |
//!
//! So `0x0094B08B` is a **QI wrapper that consumes the window it created**: it
//! returns only whether the query worked, and releases the window before
//! returning. A caller that wants the window must call slot 4. Confusing the
//! two is the difference between getting a window and getting `AL` (and a
//! freed window).

use super::slots::{UI_SCRIPT_GROUP, UI_SCRIPT_IFACE, UI_SCRIPT_SERVICE};
use super::win::{WinToken, WinStore};

/// The three arguments of `SC4CreateWindowFromScript` (§5.1).
///
/// The argument **order** is not fully settled (§4.3: "*[inferred]* the order
/// is `(out_ptr, parent, instance)` from the push sequence (`push eax` /
/// `push edx` / `push ebx`, right-to-left)", while §5.1 establishes that one
/// of the three is the UI **group** `0x96A006B0` and one is an **instance**).
/// We take them named so the order cannot leak into a caller; see question 3.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ScriptWindowRequest {
    /// The UI script **group**: `0x96A006B0` for every scripted window
    /// (INVENTORY-M6 §4.1). Conformance test 16: a group, **not** a window ID.
    pub group: u32,
    /// The parent window. For a catalog menu this is `AsIGZWin()` — the view's
    /// own window sub-object (§4.3, conformance test 15).
    pub parent: WinToken,
    /// The script instance id. *[inferred]* it becomes the window's `GetID`,
    /// because `GetChildWindowFromID` (slot 34) must find scripted children
    /// and `cGZLegacyControl` names controls by instance (UIW-3).
    pub instance: u32,
}

impl ScriptWindowRequest {
    /// The catalog-menu request shape (§4.3).
    pub fn catalog(parent: WinToken, instance: u32) -> ScriptWindowRequest {
        ScriptWindowRequest { group: UI_SCRIPT_GROUP, parent, instance }
    }
}

/// The outcome of a create (§5.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ScriptWindowResult {
    /// The window, or 0 when the script service is missing.
    pub window: WinToken,
    /// Whether the cleanup helper ran. Quirk 8: **true on both paths** — the
    /// cleanup is not conditional on success.
    pub cleanup_ran: bool,
}

impl ScriptWindowResult {
    /// True when the factory produced a window.
    pub fn ok(&self) -> bool {
        self.window != 0
    }
}

/// What a script service must answer for the window layer.
///
/// §13: the UI script interpreter (`cGZUIScriptService`, `cGZLegacyControl`)
/// is the unmade decision of INVENTORY-M6 §4.1. **Nothing in §2–§4 depends on
/// interpreting the shipped scripts**: the only thing the window layer needs is
/// "create a window whose parent is X and whose group/instance are G/I", which
/// is this trait.
pub trait UIScriptService {
    /// `cGZUIScriptService` **slot 4** — the factory that returns the window
    /// in EAX. Three arguments forwarded unchanged.
    ///
    /// Do **not** call the method named `CreateWindowFromScript` at
    /// `0x0094B08B`: that is the 5-argument QI wrapper of quirk 10.
    fn create_window(&mut self, req: &ScriptWindowRequest) -> WinToken;
}

/// `SC4CreateWindowFromScript` (`0x005F9390`): the 3-argument thunk to
/// `cGZUIScriptService` slot 4.
///
/// Returns the window in EAX, or 0 when the script service is missing. Quirk
/// 8: the cleanup runs on **both** exits.
pub fn sc4_create_window_from_script(
    service: Option<&mut dyn UIScriptService>,
    req: &ScriptWindowRequest,
) -> ScriptWindowResult {
    let window = match service {
        Some(svc) => svc.create_window(req),
        None => 0,
    };
    // FUN_004495C0 runs on both exits (quirk 8).
    ScriptWindowResult { window, cleanup_ran: true }
}

/// The service lookup `SC4CreateWindowFromScript` performs (§5.1).
///
/// `cRZSysServPtr<cIGZUIScriptService, 4197802746ul, 1513451029ul>`
/// (`0x00449750`): `4197802746 = 0xFA3562FA` is the interface id and
/// `1513451029 = 0x5A356E15` the service id. The class is `cGZUIScriptService`,
/// class id `0xFA3562FB` — the interface id is one less than the class id.
pub fn service_lookup() -> (u32, u32) {
    (UI_SCRIPT_SERVICE, UI_SCRIPT_IFACE)
}

/// Whether the interface id is one less than the class id (§5.1's
/// cross-check).
pub fn iface_is_class_minus_one() -> bool {
    UI_SCRIPT_IFACE == 0xFA35_62FBu32.wrapping_sub(1)
}

/// A default script service that builds a window in a [`WinStore`], so the
/// catalog path can be exercised without a UI script interpreter.
#[derive(Debug, Default)]
pub struct StoreScriptService {
    /// The token the next window gets.
    next_id: WinToken,
    /// Whether the service answers at all (a false here models the "script
    /// service is missing" path of §5.1).
    pub present: bool,
}

impl StoreScriptService {
    pub fn new() -> StoreScriptService {
        StoreScriptService { next_id: 0x1000, present: true }
    }

    /// Build the window in `store`, parenting it to the request's parent.
    pub fn create_in(&mut self, store: &mut WinStore, req: &ScriptWindowRequest) -> WinToken {
        if !self.present {
            return 0;
        }
        // *[choice]* the instance becomes the window's `GetID` (UIW-3,
        // *[inferred]* in §5.1). `GetChildWindowFromID` (slot 34) has to find
        // scripted children, and `cGZLegacyControl` names controls by
        // instance.
        self.next_id = self.next_id.wrapping_add(1);
        let token = self.next_id;
        if store.adopt(token, req.instance, req.parent) {
            token
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeService {
        token: WinToken,
        calls: Vec<ScriptWindowRequest>,
    }

    impl UIScriptService for FakeService {
        fn create_window(&mut self, req: &ScriptWindowRequest) -> WinToken {
            self.calls.push(*req);
            self.token
        }
    }

    #[test]
    fn thunk_calls_slot_4_and_returns_the_window() {
        let mut svc = FakeService { token: 42, calls: Vec::new() };
        let req = ScriptWindowRequest::catalog(7, 0x9DE3_002);
        let r = sc4_create_window_from_script(Some(&mut svc), &req);
        assert_eq!(r.window, 42);
        assert!(r.ok());
        assert!(r.cleanup_ran);
        assert_eq!(svc.calls, vec![req]);
    }

    #[test]
    fn a_missing_service_returns_zero_and_still_cleans_up() {
        // Quirk 8.
        let req = ScriptWindowRequest::catalog(7, 1);
        let r = sc4_create_window_from_script(None, &req);
        assert_eq!(r.window, 0);
        assert!(!r.ok());
        assert!(r.cleanup_ran, "the cleanup is not conditional on success");
    }

    #[test]
    fn the_service_ids_match_the_spec() {
        assert_eq!(UI_SCRIPT_SERVICE, 0x5A35_6E15);
        assert_eq!(UI_SCRIPT_IFACE, 0xFA35_62FA);
        assert!(iface_is_class_minus_one(), "the iface id is the class id - 1");
        assert_eq!(service_lookup(), (UI_SCRIPT_SERVICE, UI_SCRIPT_IFACE));
    }

    #[test]
    fn the_catalog_request_carries_the_ui_script_group() {
        let r = ScriptWindowRequest::catalog(1, 2);
        assert_eq!(r.group, UI_SCRIPT_GROUP, "0x96A006B0, not a window ID");
    }

    #[test]
    fn store_service_parents_the_window_to_the_request() {
        let mut store = WinStore::new();
        let parent = store.create(1);
        let mut svc = StoreScriptService::new();
        let req = ScriptWindowRequest { group: UI_SCRIPT_GROUP, parent, instance: 0x9DE3_002 };
        let win = svc.create_in(&mut store, &req);
        assert!(store.is_window_valid(win));
        assert_eq!(store.get(win).unwrap().parent(), Some(parent));
        assert_eq!(store.get(win).unwrap().id(), 0x9DE3_002, "the instance becomes GetID");
    }

    #[test]
    fn an_absent_store_service_returns_zero() {
        let mut store = WinStore::new();
        let parent = store.create(1);
        let mut svc = StoreScriptService::new();
        svc.present = false;
        let req = ScriptWindowRequest::catalog(parent, 1);
        assert_eq!(svc.create_in(&mut store, &req), 0);
    }
}
