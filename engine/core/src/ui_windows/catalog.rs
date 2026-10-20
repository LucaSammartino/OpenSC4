// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The catalog menus: secondary and tertiary (ui-windows.md §4).
//!
//! The 3D view opens two levels of catalog popup over the city — the
//! tool/build catalog (secondary) and a sub-catalog (tertiary). They are
//! ordinary scripted child windows of the view's `cIGZWin` sub-object, tracked
//! in the layout fields of [`View3DWin`], and the tutorial's `endTutorial`
//! closes them through slots 59 and 60 (tutorial.md TU2).
//!
//! # What this module gets right that a naive teardown gets wrong
//!
//! 1. **Closing the secondary menu closes the tertiary one first** (§4.2,
//!    quirk 1). Slot 59 calls slot 60 before doing anything else, because a
//!    tertiary catalog is only reachable from a secondary one — tear the
//!    secondary down alone and the leftover is unreachable, since
//!    `ChildDelete` destroys it.
//! 2. **`Invoke*` drops the current tool before it decides anything** (§4.3,
//!    quirk 3). Merely *trying* to open a catalog menu that is already open
//!    still costs the player their tool.
//! 3. **Teardown is Hide → Release → `ChildDelete`, then the fields** (§4.1,
//!    conformance test 7). `ChildDelete` is the tail forward of §2.3, so the
//!    child is destroyed, not merely detached.
//! 4. **`CloseSecondaryMenu` does not write the catalog state** (§4.2). Only
//!    the tertiary path writes through `+0x0208`; a replacement that "tidies
//!    up" by also writing it on the secondary path differs.
//! 5. **The parent of a new catalog window is `AsIGZWin()`** (§4.3) — the
//!    view's own window sub-object — which is what makes `ChildDelete` on
//!    that sub-object the right teardown.

use super::slots::{IID_CATALOG_STATE, IID_IGZ_WIN_GEN, UI_SCRIPT_GROUP};
use super::view3d::View3DWin;
use super::win::{WinOp, WinStore, WinToken, WIN_GEN_FLAG_1, WIN_GEN_FLAG_4};

/// The anchor code `nSC4UI::AlignWindowToWindow` is passed (§4.3).
///
/// *[inferred]* `0xAAAA` is an anchor/alignment code (ui-windows.md UIW-5).
pub const ALIGN_ANCHOR: u32 = 0xAAAA;

/// The fourth argument of `AlignWindowToWindow` (§4.3).
pub const ALIGN_FLAGS: u32 = 0;

/// `AlignWindowToWindow`'s third argument is a **target window id**, not a
/// coordinate (§4.4): at the two known call sites it is `1` and a
/// resource-shaped id `0xAB95_37B7`. *[inferred]* the signature is
/// `nSC4UI::AlignWindowToWindow(uint32 winID, uint32 anchor, uint32
/// targetWinID, uint32 flags)`, with `1` a special "screen"/"parent" id.
pub const ALIGN_TARGET_SCREEN: u32 = 1;

/// A catalog menu's identity: which of the two levels it is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuLevel {
    /// The tool/build catalog, `view+0x01F8` / `view+0x0200`, closed by slot 59.
    Secondary,
    /// The sub-catalog, `view+0x01FC` / `view+0x0204`, closed by slot 60.
    Tertiary,
}

impl MenuLevel {
    /// The slot that closes this level.
    pub fn close_slot(self) -> u32 {
        match self {
            MenuLevel::Secondary => super::slots::view3d_slot::CLOSE_SECONDARY_MENU,
            MenuLevel::Tertiary => super::slots::view3d_slot::CLOSE_TERTIARY_MENU,
        }
    }
}

/// Everything a catalog menu does outside the view and the window store.
///
/// Split out so the menu logic is testable without a script interpreter —
/// the UI script grammar is the unmade decision of INVENTORY-M6 §4.1, and
/// §13 says nothing in §2–§4 depends on interpreting the shipped scripts. The
/// only thing the window layer needs is "create a window whose parent is X
/// and whose group/instance are G/I" (§5.1).
pub trait CatalogHost {
    /// `SC4CreateWindowFromScript` (§5.1): a 3-argument thunk to
    /// `cGZUIScriptService` **slot 4**, returning the window in EAX or 0 when
    /// the script service is missing.
    ///
    /// Quirk 10: do **not** use the method named `CreateWindowFromScript`
    /// (`0x0094_B08B`) — that is a 5-argument QI wrapper returning `AL` that
    /// releases the window it just made.
    ///
    /// The argument **order** is not fully settled (§4.3, *[inferred]*
    /// `(out_ptr, parent, instance)` from a push sequence that interleaves
    /// register saves). We take them named so the order cannot leak into a
    /// caller; see question 3 in the module docs.
    fn create_window_from_script(&mut self, group: u32, parent: WinToken, instance: u32) -> Option<WinToken>;

    /// `QueryInterface(0x8A2C_AD8B, &p)` on the window (§4.1).
    ///
    /// The **2-argument** form (`iid, &out`) on slot 0, not the SDK's
    /// 3-argument COM form: the callee writes the out pointer. Returns a
    /// handle to the interface, or `None` when the query fails or the callee
    /// wrote a null.
    fn query_catalog_iface(&mut self, win: WinToken, iid: u32) -> Option<u64>;

    /// The interface's slot 17 (§4.1). Written to `state+0`.
    fn catalog_iface_slot17(&mut self, iface: u64) -> u32;

    /// The interface's slot 9 (§4.1). Written to `state+4`.
    fn catalog_iface_slot9(&mut self, iface: u64) -> u32;

    /// The interface's `Release` (slot 2), after the two reads (§4.1).
    fn catalog_iface_release(&mut self, iface: u64);

    /// `cIGZWinGen::SetWinGenFlag(flag, value)` (gen slot 6, §4.3).
    ///
    /// *[inferred]* from the Mac name (`SetWinGenFlag(long, __bool8)`) that
    /// the slot pair is `(flag, value)`. The flag bit meanings are
    /// ui-windows.md UIW-4 and are distinct from `cIGZWin`'s
    /// `GetFlag`/`SetFlag` (slots 67/68).
    fn set_win_gen_flag(&mut self, win: WinToken, flag: u32, value: bool);

    /// `nSC4UI::AlignWindowToWindow(winID, anchor, targetWinID, flags)` at
    /// `0x0078_DB20` (§4.3).
    fn align_window_to_window(&mut self, win_id: u32, anchor: u32, target: u32, flags: u32);
}

/// The four arguments of `InvokeSecondaryMenu` (`0x007E_5C10`, `ret 0x0010`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SecondaryInvoke {
    /// arg1: a script-instance id (§4.4's table reads the same slot in the
    /// tertiary callers as `0x09DE_3002` / `0xCB95_403E`).
    pub instance: u32,
    /// arg2: the **menu kind**, compared against `+0x0200` and stored there on
    /// success.
    pub kind: u32,
    /// arg3: caller data.
    pub caller_data: u32,
    /// arg4: the target window id passed to `AlignWindowToWindow` (§4.4).
    pub target_win_id: u32,
}

/// The five arguments of `InvokeTertiaryMenu` (`0x007E_5D80`, `ret 0x0014`).
///
/// Quirk 12: the name `InvokeTertiaryMenu` also belongs to `0x007E_AEB0`,
/// which takes **six** dwords (`ret 0x0018`) and has no callers. A binding
/// written against the name rather than the address gets the wrong ABI.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TertiaryInvoke {
    /// arg1: a script-instance id.
    pub instance: u32,
    /// arg2: the **menu kind** (what `ebx` holds and what is stored to
    /// `+0x0204`).
    pub kind: u32,
    /// arg3: caller data.
    pub caller_data: u32,
    /// arg4: the target window id passed to `AlignWindowToWindow`.
    pub target_win_id: u32,
    /// arg5: a small negative integer (−25 at `0x007E_7179`, −75 at
    /// `0x007F_5093`), the only argument the secondary form lacks.
    ///
    /// It does **not** reach `AlignWindowToWindow`, which gets arg4; it is
    /// consumed earlier in the body or is unused. *[inferred]* a pixel
    /// margin/offset for the popup placement, which would explain −25 / −75
    /// (ui-windows.md UIW-2). Stored here so a vector can pin a call site's
    /// value without this module inventing what it means.
    pub margin: i32,
}

/// The outcome of an `Invoke*` call.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InvokeResult {
    /// The menu that was already open was toggled closed. No window created.
    Toggled,
    /// A menu was open with a different kind and is now closed, and the create
    /// then failed, so nothing is open.
    ReplacedThenFailed,
    /// A new window was created and shown.
    Opened(WinToken),
}

/// `CloseTertiaryMenu` (slot 60, `0x007E_5E90`): thiscall, pops 0, **returns
/// nothing**.
///
/// Conformance test 3: a caller must not read EAX.
///
/// Body order (§4.1): the `null` check is the **only** early return; on every
/// other path both `+0x01FC` and `+0x0204` are cleared at the end, even when the
/// `QueryInterface` fails (quirk 6).
pub fn close_tertiary_menu(host: &mut dyn CatalogHost, view: &mut View3DWin, store: &mut WinStore) {
    let Some(tertiary) = view.tertiary_win() else {
        // Conformance test 5: with +0x01FC == 0, return without touching
        // +0x0204 (or anything else).
        return;
    };

    // The catalog-state save, only when the pointer is non-null and only on
    // this path (§4.1, §4.2, conformance tests 9 and 10).
    if view.catalog_state().is_some() {
        if let Some(iface) = host.query_catalog_iface(tertiary, IID_CATALOG_STATE) {
            // Measured order: slot 17 first, then slot 9, then Release.
            let field0 = host.catalog_iface_slot17(iface);
            let field1 = host.catalog_iface_slot9(iface);
            // Quirk 5: written through a raw pointer with no lifetime
            // management and no validation. Quirk 6: a failed query skips the
            // write but not the teardown below.
            if view.write_catalog_state(field0, field1) {
                store.push(WinOp::CatalogStateWrite {
                    offset: 0,
                    value: field0,
                });
                store.push(WinOp::CatalogStateWrite {
                    offset: 4,
                    value: field1,
                });
            }
            host.catalog_iface_release(iface);
        }
    }

    // Teardown, fixed order: HideWindow → Release → ChildDelete (§4.1).
    store.hide_window(tertiary);
    store.release(tertiary);
    store.child_delete(view.as_igzwin(), tertiary);

    // Both fields cleared unconditionally at the end (§4.1).
    view.set_tertiary(None, 0);
}

/// `CloseSecondaryMenu` (slot 59, `0x007E_5D20`): thiscall, pops 0, **returns
/// nothing**.
///
/// Body order (§4.2): the `null` check is the only early return; then
/// **slot 60 first**, then the teardown, then the two fields.
///
/// Note the asymmetry (§4.2): this path does **not** touch `view+0x0208`.
pub fn close_secondary_menu(host: &mut dyn CatalogHost, view: &mut View3DWin, store: &mut WinStore) {
    let Some(secondary) = view.secondary_win() else {
        // Conformance test 4: with +0x01F8 == 0, must not call
        // CloseTertiaryMenu, HideWindow, Release or ChildDelete, and must
        // leave +0x0200 alone.
        return;
    };

    // The cascade. This is the whole reason slot 59 calls slot 60 (§4.2,
    // quirk 1): a tertiary catalog is only reachable from a secondary one.
    close_tertiary_menu(host, view, store);

    store.hide_window(secondary);
    store.release(secondary);
    store.child_delete(view.as_igzwin(), secondary);

    view.set_secondary(None, 0);
}

/// The shared body of `InvokeSecondaryMenu` and `InvokeTertiaryMenu` (§4.3,
/// §4.4).
///
/// §4.4: "Byte-for-byte the same shape as §4.3 with four differences" — the
/// kind field tested, the close slot, the callee's pop count and which fields
/// are stored. Everything else is identical, including the tool drop that
/// runs *before* the open-menu test.
fn invoke(
    host: &mut dyn CatalogHost,
    view: &mut View3DWin,
    store: &mut WinStore,
    level: MenuLevel,
    instance: u32,
    kind: u32,
    target_win_id: u32,
) -> InvokeResult {
    // (1) The tool drop runs FIRST, before any test on the current menu
    // (§4.3, quirk 3, conformance test 14). Opening a catalog menu that is
    // already open still costs the player their tool.
    let dropped = view.remove_all_view_input_controls(0);
    store.push(WinOp::RemoveAllViewInputControls(0));
    let _ = dropped;

    // (2) The toggle / replace test on the kind field.
    let (current_win, current_kind) = match level {
        MenuLevel::Secondary => (view.secondary_win(), view.secondary_kind()),
        MenuLevel::Tertiary => (view.tertiary_win(), view.tertiary_kind()),
    };
    let replaced = if current_kind != 0 {
        if current_kind == kind {
            // Toggle: close the same menu and return without creating.
            close_level(host, view, store, level);
            return InvokeResult::Toggled;
        }
        // Replace: close the open one first, then open the new one
        // (conformance test 13).
        close_level(host, view, store, level);
        true
    } else {
        false
    };
    let _ = current_win;

    // (3) Create. The parent is AsIGZWin() — the view's own window
    // sub-object (§4.3, conformance test 15) — and the group is the UI
    // script group 0x96A0_06B0 (§4.3, conformance test 16), not a window ID.
    let parent = view.as_igzwin();
    store.push(WinOp::CreateWindow {
        parent,
        group: UI_SCRIPT_GROUP,
        instance,
    });
    let Some(win) = host.create_window_from_script(UI_SCRIPT_GROUP, parent, instance) else {
        // Conformance test 17: a null create writes no field, calls no
        // AddMessageFilter, and leaves the previous menu closed.
        store.push(WinOp::WindowCreated { token: 0, ok: false });
        return if replaced {
            InvokeResult::ReplacedThenFailed
        } else {
            InvokeResult::Toggled
        };
    };
    // The factory produced a window the store has never seen (§5.1: it comes
    // back as a `cIGZWin*` in EAX). Adopt it and parent it to the view's own
    // window sub-object — conformance test 15.
    store.adopt(win, instance, parent);
    store.push(WinOp::WindowCreated { token: win, ok: true });

    // (4) The generator interface: QueryInterface(0x5386_D516) on the new
    // window (§4.3).
    if store.query_win_gen(win) {
        store.push(WinOp::QueryInterface {
            win,
            iid: IID_IGZ_WIN_GEN,
            ok: true,
        });
        // The filter is inline at +0x0008 on the view, so its **address** is
        // what AddMessageFilter takes (§3.2).
        store.add_message_filter(win, view.msg_filter_offset());
        // Both generator flags cleared (conformance test 18).
        host.set_win_gen_flag(win, WIN_GEN_FLAG_4, false);
        store.push(WinOp::SetWinGenFlag {
            flag: WIN_GEN_FLAG_4,
            value: false,
        });
        host.set_win_gen_flag(win, WIN_GEN_FLAG_1, false);
        store.push(WinOp::SetWinGenFlag {
            flag: WIN_GEN_FLAG_1,
            value: false,
        });
        // The interface is released afterwards (conformance test 18).
        store.release(win);
    }

    // (5) Align, then PullToFront, then ShowWindow, and only then the fields
    // (§4.3, conformance test 19).
    let win_id = store.get_id(win).unwrap_or(0);
    store.push(WinOp::GetID(win));
    host.align_window_to_window(win_id, ALIGN_ANCHOR, target_win_id, ALIGN_FLAGS);
    store.push(WinOp::AlignWindowToWindow {
        win: win_id,
        anchor: ALIGN_ANCHOR,
        target: target_win_id,
        flags: ALIGN_FLAGS,
    });
    store.pull_to_front(win);
    store.show_window(win);

    // (6) Now the fields, so a reader that sees the window in +0x01F8/+0x01FC
    // also sees it already shown (conformance test 19).
    match level {
        MenuLevel::Secondary => view.set_secondary(Some(win), kind),
        MenuLevel::Tertiary => view.set_tertiary(Some(win), kind),
    }

    InvokeResult::Opened(win)
}

/// Close whichever level `level` names, through its slot.
///
/// Secondary cascades to tertiary inside [`close_secondary_menu`]; tertiary
/// does not touch the secondary.
fn close_level(host: &mut dyn CatalogHost, view: &mut View3DWin, store: &mut WinStore, level: MenuLevel) {
    match level {
        MenuLevel::Secondary => close_secondary_menu(host, view, store),
        MenuLevel::Tertiary => close_tertiary_menu(host, view, store),
    }
}

/// `InvokeSecondaryMenu` (`0x007E_5C10`): thiscall, **4 arguments** (`ret
/// 0x0010`), no useful return.
pub fn invoke_secondary_menu(
    host: &mut dyn CatalogHost,
    view: &mut View3DWin,
    store: &mut WinStore,
    args: SecondaryInvoke,
) -> InvokeResult {
    invoke(
        host,
        view,
        store,
        MenuLevel::Secondary,
        args.instance,
        args.kind,
        args.target_win_id,
    )
}

/// `InvokeTertiaryMenu` (`0x007E_5D80`): thiscall, **5 arguments** (`ret
/// 0x0014`), no useful return.
///
/// The fifth argument (`args.margin`) is the only one the secondary form
/// lacks. It does not reach `AlignWindowToWindow` (§4.4).
pub fn invoke_tertiary_menu(
    host: &mut dyn CatalogHost,
    view: &mut View3DWin,
    store: &mut WinStore,
    args: TertiaryInvoke,
) -> InvokeResult {
    // arg5 is consumed earlier in the body or is unused (§4.4, UIW-2). We do
    // not invent a use for it: `[choice]` record it and ignore it here.
    let _ = args.margin;
    let _ = args.caller_data;
    invoke(
        host,
        view,
        store,
        MenuLevel::Tertiary,
        args.instance,
        args.kind,
        args.target_win_id,
    )
}

/// The catalog-state write order, for a vector that pins it (conformance test
/// 10): slot 17 to `state+0`, then slot 9 to `state+4`, then the release.
pub fn catalog_state_write_order() -> [(&'static str, u32); 3] {
    [("slot17", 0), ("slot9", 4), ("release", 0)]
}

/// `CloseSecondaryMenu`'s cascade: it calls slot 60 before anything else
/// (§4.2, conformance test 6).
pub fn secondary_close_cascades() -> bool {
    true
}

/// Whether `CloseSecondaryMenu` may write through `+0x0208` (§4.2,
/// conformance test 9).
pub fn secondary_close_writes_catalog_state() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_windows::slots::VIEW3D_OFF_WIN;
    use crate::ui_windows::view3d::CatalogState;

    /// A scripted factory that can be made to fail, and records nothing the
    /// store does not.
    struct TestHost {
        create_ok: bool,
        catalog_iface: Option<u64>,
        slot17: u32,
        slot9: u32,
        released: Vec<u64>,
        flags: Vec<(WinToken, u32, bool)>,
        aligns: Vec<(u32, u32, u32, u32)>,
        minted: u64,
    }

    impl TestHost {
        fn ok() -> TestHost {
            TestHost {
                create_ok: true,
                catalog_iface: Some(0x00C0),
                slot17: 17,
                slot9: 9,
                released: Vec::new(),
                flags: Vec::new(),
                aligns: Vec::new(),
                minted: 0,
            }
        }
    }

    impl CatalogHost for TestHost {
        fn create_window_from_script(&mut self, _group: u32, _parent: WinToken, _instance: u32) -> Option<WinToken> {
            if self.create_ok {
                // A window the store has never seen: `invoke` adopts it.
                self.minted += 1;
                Some(0x9000 + self.minted)
            } else {
                None
            }
        }

        fn query_catalog_iface(&mut self, _win: WinToken, iid: u32) -> Option<u64> {
            if iid == IID_CATALOG_STATE {
                self.catalog_iface
            } else {
                None
            }
        }

        fn catalog_iface_slot17(&mut self, _iface: u64) -> u32 {
            self.slot17
        }

        fn catalog_iface_slot9(&mut self, _iface: u64) -> u32 {
            self.slot9
        }

        fn catalog_iface_release(&mut self, iface: u64) {
            self.released.push(iface);
        }

        fn set_win_gen_flag(&mut self, win: WinToken, flag: u32, value: bool) {
            self.flags.push((win, flag, value));
        }

        fn align_window_to_window(&mut self, win_id: u32, anchor: u32, target: u32, flags: u32) {
            self.aligns.push((win_id, anchor, target, flags));
        }
    }

    fn setup() -> (TestHost, View3DWin, WinStore) {
        let mut store = WinStore::new();
        let win = store.create(1);
        let view = View3DWin::new(0x2000, win);
        (TestHost::ok(), view, store)
    }

    #[test]
    fn close_secondary_is_a_noop_when_nothing_is_open() {
        // Conformance test 4.
        let (mut h, mut v, mut s) = setup();
        v.set_secondary(None, 0x1234);
        s.clear_ops();
        close_secondary_menu(&mut h, &mut v, &mut s);
        assert!(s.ops().is_empty(), "must not call Hide/Release/ChildDelete");
        assert_eq!(v.secondary_kind(), 0x1234, "+0x0200 left alone");
    }

    #[test]
    fn close_tertiary_is_a_noop_when_nothing_is_open() {
        // Conformance test 5.
        let (mut h, mut v, mut s) = setup();
        v.set_tertiary(None, 0x1234);
        s.clear_ops();
        close_tertiary_menu(&mut h, &mut v, &mut s);
        assert!(s.ops().is_empty());
        assert_eq!(v.tertiary_kind(), 0x1234, "+0x0204 left alone");
    }

    #[test]
    fn secondary_close_cascades_to_tertiary_first() {
        // Conformance test 6: CloseTertiaryMenu before HideWindow on the
        // secondary window.
        let (mut h, mut v, mut s) = setup();
        let sec = s.create(2);
        let ter = s.create(3);
        s.child_add(v.as_igzwin(), sec);
        s.child_add(v.as_igzwin(), ter);
        v.set_secondary(Some(sec), 1);
        v.set_tertiary(Some(ter), 2);
        s.clear_ops();
        close_secondary_menu(&mut h, &mut v, &mut s);

        let hide_of = |t: WinToken| WinOp::HideWindow(t);
        let del_of = |t: WinToken| WinOp::ChildDelete {
            parent: v.as_igzwin(),
            child: t,
        };
        let ops = s.ops();
        let i_ter_hide = ops.iter().position(|o| *o == hide_of(ter)).unwrap();
        let i_sec_hide = ops.iter().position(|o| *o == hide_of(sec)).unwrap();
        assert!(i_ter_hide < i_sec_hide, "tertiary must close first");
        let i_ter_del = ops.iter().position(|o| *o == del_of(ter)).unwrap();
        assert!(i_ter_del < i_sec_hide);
    }

    #[test]
    fn teardown_order_is_hide_release_childdelete() {
        // Conformance test 7.
        let (mut h, mut v, mut s) = setup();
        let ter = s.create(3);
        s.child_add(v.as_igzwin(), ter);
        v.set_tertiary(Some(ter), 2);
        s.clear_ops();
        close_tertiary_menu(&mut h, &mut v, &mut s);
        let ops: Vec<&WinOp> = s
            .ops()
            .iter()
            .filter(|o| matches!(o, WinOp::HideWindow(_) | WinOp::Release(_) | WinOp::ChildDelete { .. }))
            .collect();
        assert_eq!(
            ops,
            vec![
                &WinOp::HideWindow(ter),
                &WinOp::Release(ter),
                &WinOp::ChildDelete {
                    parent: v.as_igzwin(),
                    child: ter
                },
            ]
        );
    }

    #[test]
    fn tertiary_close_clears_both_fields() {
        // Conformance test 8.
        let (mut h, mut v, mut s) = setup();
        let ter = s.create(3);
        s.child_add(v.as_igzwin(), ter);
        v.set_tertiary(Some(ter), 2);
        close_tertiary_menu(&mut h, &mut v, &mut s);
        assert_eq!(v.tertiary_win(), None);
        assert_eq!(v.tertiary_kind(), 0);
    }

    #[test]
    fn secondary_does_not_write_catalog_state() {
        // Conformance test 9 / §4.2's asymmetry. With only the secondary open
        // there is no tertiary path to write through +0x0208, so the state is
        // untouched. The secondary teardown itself never writes it.
        let (mut h, mut v, mut s) = setup();
        let sec = s.create(2);
        s.child_add(v.as_igzwin(), sec);
        v.set_secondary(Some(sec), 1);
        v.set_catalog_state(Some(CatalogState { field0: 5, field1: 6 }));
        s.clear_ops();
        close_secondary_menu(&mut h, &mut v, &mut s);
        assert_eq!(
            v.catalog_state(),
            Some(CatalogState { field0: 5, field1: 6 }),
            "CloseSecondaryMenu must not write through +0x0208"
        );
        assert!(
            !s.ops().iter().any(|o| matches!(o, WinOp::CatalogStateWrite { .. })),
            "and emits no CatalogStateWrite"
        );
    }

    #[test]
    fn the_cascade_writes_catalog_state_through_the_tertiary_path() {
        // The other half of §4.2: the catalog state is written back **by the
        // tertiary path**. Closing the secondary with a tertiary open
        // cascades, and that cascade is what writes +0x0208. A replacement that
        // "tidies up" by also writing it on the secondary path itself differs.
        let (mut h, mut v, mut s) = setup();
        h.slot17 = 0x0011;
        h.slot9 = 0x0022;
        let sec = s.create(2);
        let ter = s.create(3);
        s.child_add(v.as_igzwin(), sec);
        s.child_add(v.as_igzwin(), ter);
        v.set_secondary(Some(sec), 1);
        v.set_tertiary(Some(ter), 2);
        v.set_catalog_state(Some(CatalogState::default()));
        close_secondary_menu(&mut h, &mut v, &mut s);
        assert_eq!(
            v.catalog_state(),
            Some(CatalogState {
                field0: 0x0011,
                field1: 0x0022
            }),
            "written by the cascaded tertiary close"
        );
    }

    #[test]
    fn tertiary_writes_two_dwords_in_order() {
        // Conformance test 10: slot 17 to state+0, slot 9 to state+4, then
        // the release.
        let (mut h, mut v, mut s) = setup();
        h.slot17 = 0x00AA;
        h.slot9 = 0x00BB;
        let ter = s.create(3);
        s.child_add(v.as_igzwin(), ter);
        v.set_tertiary(Some(ter), 2);
        v.set_catalog_state(Some(CatalogState::default()));
        s.clear_ops();
        close_tertiary_menu(&mut h, &mut v, &mut s);
        assert_eq!(
            v.catalog_state(),
            Some(CatalogState {
                field0: 0x00AA,
                field1: 0x00BB
            })
        );
        assert_eq!(h.released, vec![0x00C0]);
        let writes: Vec<(u32, u32)> = s
            .ops()
            .iter()
            .filter_map(|o| match o {
                WinOp::CatalogStateWrite { offset, value } => Some((*offset, *value)),
                _ => None,
            })
            .collect();
        assert_eq!(writes, vec![(0, 0x00AA), (4, 0x00BB)]);
    }

    #[test]
    fn tertiary_skips_the_save_on_a_failed_query() {
        // Conformance test 11: +0x0208 is not written but the window is still
        // hidden, released and deleted. Quirk 6: the fields are still cleared.
        let (mut h, mut v, mut s) = setup();
        h.catalog_iface = None;
        let ter = s.create(3);
        s.child_add(v.as_igzwin(), ter);
        v.set_tertiary(Some(ter), 2);
        v.set_catalog_state(Some(CatalogState { field0: 5, field1: 6 }));
        s.clear_ops();
        close_tertiary_menu(&mut h, &mut v, &mut s);
        assert_eq!(
            v.catalog_state(),
            Some(CatalogState { field0: 5, field1: 6 }),
            "+0x0208 untouched on a failed query"
        );
        assert_eq!(v.tertiary_win(), None);
        assert_eq!(v.tertiary_kind(), 0);
        assert!(!s.is_window_valid(ter));
        assert!(h.released.is_empty(), "nothing to release");
    }

    #[test]
    fn invoke_toggles_the_same_menu() {
        // Conformance test 12.
        let (mut h, mut v, mut s) = setup();
        let mut hs = TestHost::ok();
        let first = invoke_secondary_menu(
            &mut hs,
            &mut v,
            &mut s,
            SecondaryInvoke {
                instance: 1,
                kind: 0x09DE_8798,
                caller_data: 0,
                target_win_id: 1,
            },
        );
        assert!(matches!(first, InvokeResult::Opened(_)), "{first:?}");
        let open = v.secondary_win().unwrap();

        let mut h2 = TestHost::ok();
        let second = invoke_secondary_menu(
            &mut h2,
            &mut v,
            &mut s,
            SecondaryInvoke {
                instance: 1,
                kind: 0x09DE_8798,
                caller_data: 0,
                target_win_id: 1,
            },
        );
        assert_eq!(second, InvokeResult::Toggled);
        assert_eq!(v.secondary_win(), None, "toggled closed");
        assert!(!s.is_window_valid(open), "the window was destroyed");
        let _ = &mut h;
    }

    #[test]
    fn invoke_replaces_a_different_menu() {
        // Conformance test 13: the open one is closed before the create.
        let (mut h, mut v, mut s) = setup();
        let mut hs = TestHost::ok();
        invoke_secondary_menu(
            &mut hs,
            &mut v,
            &mut s,
            SecondaryInvoke {
                instance: 1,
                kind: 0x1111,
                caller_data: 0,
                target_win_id: 1,
            },
        );
        let first = v.secondary_win().unwrap();
        s.clear_ops();
        invoke_secondary_menu(
            &mut hs,
            &mut v,
            &mut s,
            SecondaryInvoke {
                instance: 2,
                kind: 0x2222,
                caller_data: 0,
                target_win_id: 1,
            },
        );
        assert!(!s.is_window_valid(first), "the old menu is closed");
        assert!(v.secondary_win().is_some(), "a new one is open");
        let closed_at = s.ops().iter().position(|o| {
            *o == WinOp::ChildDelete {
                parent: v.as_igzwin(),
                child: first,
            }
        });
        let created_at = s.ops().iter().position(|o| matches!(o, WinOp::CreateWindow { .. }));
        assert!(closed_at.unwrap() < created_at.unwrap(), "close before create");
        let _ = &mut h;
    }

    #[test]
    fn invoke_drops_the_tool_first() {
        // Conformance test 14 / quirk 3: RemoveAllViewInputControls runs
        // before the open-menu test, even on a toggle-close.
        let (mut h, mut v, mut s) = setup();
        let mut hs = TestHost::ok();
        invoke_secondary_menu(
            &mut hs,
            &mut v,
            &mut s,
            SecondaryInvoke {
                instance: 1,
                kind: 0x1111,
                caller_data: 0,
                target_win_id: 1,
            },
        );
        // Open again with the same kind: a toggle-close, and the tool must
        // still have been dropped first.
        s.clear_ops();
        invoke_secondary_menu(
            &mut hs,
            &mut v,
            &mut s,
            SecondaryInvoke {
                instance: 1,
                kind: 0x1111,
                caller_data: 0,
                target_win_id: 1,
            },
        );
        assert_eq!(
            s.ops().first(),
            Some(&WinOp::RemoveAllViewInputControls(0)),
            "the tool drop is first even when the result is a toggle-close"
        );
        let _ = &mut h;
    }

    #[test]
    fn parent_is_the_views_window() {
        // Conformance test 15: the created window's parent is AsIGZWin().
        let (mut h, mut v, mut s) = setup();
        let mut hs = TestHost::ok();
        invoke_secondary_menu(
            &mut hs,
            &mut v,
            &mut s,
            SecondaryInvoke {
                instance: 1,
                kind: 1,
                caller_data: 0,
                target_win_id: 1,
            },
        );
        let created = s.ops().iter().find_map(|o| match o {
            WinOp::CreateWindow { parent, .. } => Some(*parent),
            _ => None,
        });
        assert_eq!(created, Some(v.as_igzwin()));
        assert_eq!(
            View3DWin::as_igzwin_addr(v.this_addr()),
            v.this_addr().wrapping_add(VIEW3D_OFF_WIN),
            "AsIGZWin is this + 0x000C"
        );
        let _ = &mut h;
    }

    #[test]
    fn create_passes_the_ui_script_group() {
        // Conformance test 16.
        let (mut h, mut v, mut s) = setup();
        let mut hs = TestHost::ok();
        invoke_tertiary_menu(
            &mut hs,
            &mut v,
            &mut s,
            TertiaryInvoke {
                instance: 0x09DE_3002,
                kind: 0x09DE_8798,
                caller_data: 7,
                target_win_id: 1,
                margin: -25,
            },
        );
        let g = s.ops().iter().find_map(|o| match o {
            WinOp::CreateWindow { group, .. } => Some(*group),
            _ => None,
        });
        assert_eq!(g, Some(UI_SCRIPT_GROUP), "the group, not a window ID");
        let _ = &mut h;
    }

    #[test]
    fn null_create_is_safe() {
        // Conformance test 17.
        let (mut h, mut v, mut s) = setup();
        let mut hs = TestHost::ok();
        invoke_secondary_menu(
            &mut hs,
            &mut v,
            &mut s,
            SecondaryInvoke {
                instance: 1,
                kind: 0x1111,
                caller_data: 0,
                target_win_id: 1,
            },
        );
        let open = v.secondary_win().unwrap();

        // Now with a failing factory and a different kind: the old menu is
        // closed first, and the create then leaves nothing written.
        hs.create_ok = false;
        s.clear_ops();
        let r = invoke_secondary_menu(
            &mut hs,
            &mut v,
            &mut s,
            SecondaryInvoke {
                instance: 2,
                kind: 0x2222,
                caller_data: 0,
                target_win_id: 1,
            },
        );
        assert_eq!(r, InvokeResult::ReplacedThenFailed);
        assert_eq!(v.secondary_win(), None);
        assert_eq!(v.secondary_kind(), 0, "no field written");
        assert!(!s.is_window_valid(open), "the previous menu is closed");
        assert!(
            !s.ops().iter().any(|o| matches!(o, WinOp::AddMessageFilter { .. })),
            "no AddMessageFilter on a null create"
        );
        let _ = &mut h;
    }

    #[test]
    fn generator_flags_are_cleared_and_the_iface_released() {
        // Conformance test 18.
        let (mut h, mut v, mut s) = setup();
        let mut hs = TestHost::ok();
        invoke_secondary_menu(
            &mut hs,
            &mut v,
            &mut s,
            SecondaryInvoke {
                instance: 1,
                kind: 1,
                caller_data: 0,
                target_win_id: 1,
            },
        );
        assert_eq!(
            hs.flags,
            vec![
                (v.secondary_win().unwrap(), 4, false),
                (v.secondary_win().unwrap(), 1, false)
            ]
        );
        assert!(s.ops().contains(&WinOp::SetWinGenFlag { flag: 4, value: false }));
        assert!(s.ops().contains(&WinOp::SetWinGenFlag { flag: 1, value: false }));
        let _ = &mut h;
    }

    #[test]
    fn align_then_show_then_fields() {
        // Conformance test 19: AlignWindowToWindow once, then PullToFront,
        // then ShowWindow, and only then the fields — so a reader of +0x01F8
        // sees the window already shown.
        let (_h, mut v, mut s) = setup();
        let mut hs = TestHost::ok();
        invoke_tertiary_menu(
            &mut hs,
            &mut v,
            &mut s,
            TertiaryInvoke {
                instance: 1,
                kind: 2,
                caller_data: 0,
                target_win_id: 0xAB95_37B7,
                margin: -75,
            },
        );
        assert_eq!(hs.aligns.len(), 1);
        assert_eq!(hs.aligns[0].1, ALIGN_ANCHOR);
        assert_eq!(hs.aligns[0].2, 0xAB95_37B7, "arg4 is the target window id");
        assert_eq!(hs.aligns[0].3, 0);

        let ops = s.ops();
        let i_align = ops
            .iter()
            .position(|o| matches!(o, WinOp::AlignWindowToWindow { .. }))
            .unwrap();
        let i_pull = ops.iter().position(|o| matches!(o, WinOp::PullToFront(_))).unwrap();
        let i_show = ops.iter().position(|o| matches!(o, WinOp::ShowWindow(_))).unwrap();
        assert!(i_align < i_pull && i_pull < i_show, "align, pull, show");

        // The window is shown before the field is set: after the call, both.
        let win = v.tertiary_win().unwrap();
        assert!(s.get(win).unwrap().is_shown());
        assert_eq!(v.tertiary_kind(), 2);
    }

    #[test]
    fn tertiary_invoke_records_its_fifth_argument() {
        // UIW-2: arg5 is a small negative integer that does not reach Align.
        // We record it and do not invent a meaning for it.
        let (mut _h, mut v, mut s) = setup();
        let mut hs = TestHost::ok();
        invoke_tertiary_menu(
            &mut hs,
            &mut v,
            &mut s,
            TertiaryInvoke {
                instance: 1,
                kind: 2,
                caller_data: 0,
                target_win_id: 1,
                margin: -25,
            },
        );
        assert_eq!(hs.aligns[0].3, 0, "arg5 is not the align flags");
        assert_eq!(hs.aligns[0].2, 1, "arg4 is");
    }

    #[test]
    fn menu_level_close_slots() {
        assert_eq!(MenuLevel::Secondary.close_slot(), 59);
        assert_eq!(MenuLevel::Tertiary.close_slot(), 60);
    }
}
