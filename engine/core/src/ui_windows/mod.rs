// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The UI window and dialog layer (`docs/specs/ui-windows.md`): the window
//! object, `cSC4View3DWin`, its catalog menus and the dialog close path.
//! Section references (`§n`) point at that spec.
//!
//! This is the layer the game's window tree is made of. The rest of the UI
//! (the panel, budget, query and region windows of §14) is **named and sized
//! only** by the spec and is out of scope here; what is modelled is what the
//! spec specifies in detail, so that a later panel core stands on something
//! real.
//!
//! # The load-bearing measurement
//!
//! **`cSC4View3DWin::AsIGZWin` at `0x005F_8400` returns `this + 0x000C`.** A
//! `cSC4View3DWin` carries its `cIGZWin` sub-object at `+0x000C`; the primary
//! vptr at `+0` is `cISC4View3DWin`; the two are distinct objects within one
//! allocation. Every catalog-menu teardown goes through that sub-object, never
//! through the game interface. See [`slots::as_igzwin`] and
//! [`view3d::View3DWin::as_igzwin`].
//!
//! # Structure
//!
//! - [`slots`]: the ABI surface as data — the complete 61-slot
//!   `cISC4View3DWin` table (§3.3) with argument lists and `eax` /
//!   `eax-or-void` / `al` / `none` channels, the `cIGZWin` subset the window
//!   layer calls (§2.2, §13), the manager slots, and the `cSC4View3DWin`
//!   object layout (§3.2).
//! - [`win`]: what a `cIGZWin` *is* (§2). **`ChildDelete` (slot 16) is a
//!   four-instruction tail forward** to the manager's `DestroyWindow`, so it
//!   *destroys* and is not overridable on the window.
//! - [`view3d`]: `cSC4View3DWin`'s layout, its interface and §3.3a save
//!   behaviour. Only the View3D reference record is persisted; no UI state.
//! - [`catalog`]: the secondary/tertiary catalog menus (§4) — the cascade
//!   (`CloseSecondaryMenu` calls slot 60 first), the toggle semantics of
//!   `Invoke*`, the tool drop that runs before the open test, the parent
//!   `AsIGZWin()` and the script group `0x96A0_06B0`.
//! - [`script`]: `SC4CreateWindowFromScript`, a 3-argument thunk to
//!   `cGZUIScriptService` **slot 4** (§5.1), and the name-collision trap.
//! - [`message`]: the message vocabulary and the `DoMessage` contracts (§6.5).
//!   `DoWinProcMessage` takes the **window** as arg1.
//! - [`dialog`]: the dialog close path, the hidden pause hook and why Esc and
//!   Enter do nothing (§6).
//!
//! # Choices made where the spec is open (`[choice]`)
//!
//! - **`SC4CreateWindowFromScript`'s argument order.** §4.3 infers
//!   `(out_ptr, parent, instance)` from a push sequence that interleaves
//!   register saves, while §5.1 establishes that one of the three is the UI
//!   **group** `0x96A0_06B0` and one an **instance**. We take them **named**
//!   ([`script::ScriptWindowRequest`]) so the order cannot leak into a caller
//!   and a later binding can fix the order in one place. **Question 3.**
//! - **The fifth argument of `InvokeTertiaryMenu`** (`−25` / `−75`) is
//!   recorded on [`catalog::TertiaryInvoke`] and not interpreted: it does not
//!   reach `AlignWindowToWindow` (§4.4) and *[inferred]* a pixel margin is not
//!   proven. **Question 2.**
//! - **`RemoveAllViewInputControls`'s argument** is passed through and
//!   ignored; *[inferred]* "force"/"silent", body not read (§4.3).
//! - **`ChildAdd`'s rejection cases.** The original's were not read; we reject
//!   a duplicate and a self-parent and accept everything else.
//! - **The message-delivery stop rule.** §6.5 gives the order
//!   (`DoWinProcMessage` → `DoWinMsg` → `DoMessage` → `GZOnCommand`) but not
//!   when the chain stops. We treat a `true` return as "handled" at each
//!   stage, which is what `bool` means at both entry points.
//! - **The window state is not locked.** The window tree is main-thread UI
//!   state driven from the message loop; getters are plain `&self` reads
//!   (HANDOFF §5.1 forbids `RefCell` for state a second thread reads, and no
//!   getter here is read off the main thread). A caller that needs window
//!   state from a second thread copies it on the main thread first.
//! - **`AsIGZWin` on a synthetic address.** A Rust model has no allocation
//!   with two vptrs, so [`view3d::View3DWin`] keeps a synthetic `this` and
//!   exposes the layout as data ([`view3d::VIEW3D_LAYOUT`]). The offset
//!   constant is the measurement.
//! - **Slots 144/145** of `cIGZWin` are named `SendMsg`'s two forms; the spec
//!   gives only the 144–147 range and §13 asks for 146–147, which
//!   `docs/abi/interfaces.tsv` names `PostMsg` / `PostMsg`. **Question 5.**
//! - **The catalog-state dwords' meaning** (UIW-1) is not invented: they are
//!   opaque dwords and only their write **order** is reproduced.
//!
//! # What is not here
//!
//! - The modal loop itself (modal-dialogs.md §2) — only what the window tree
//!   looks like underneath, and the dialog close-path rule of §6.3.
//! - The advisor popup's gates and queue (advisors.md §6.8.1) — only the
//!   window-side explanation and the property/flag detail of §6.1–§6.2.
//! - The UI script grammar and each control's vocabulary (INVENTORY-M6
//!   §4.1). §13: nothing in §2–§4 depends on interpreting the shipped
//!   scripts.
//! - The panel, budget, query and region windows (§14) — named and sized
//!   only.
//! - The MXF font format (§15 UIW-10).
//!
//! # Conformance
//!
//! The 25 conformance tests of §11 have one vector each in
//! `engine/conformance/vectors/ui11_NN_*.json`, driven by
//! `engine/conformance/src/ui_windows.rs`. Tests 20, 23 and 24 are marked
//! **[game]** in the spec — they need the original; their vectors pin the
//! replacement-side behaviour that makes the observation true and are marked
//! accordingly in their `description`.

pub mod base;
pub mod catalog;
pub mod dialog;
pub mod message;
pub mod script;
pub mod slots;
pub mod view3d;
pub mod win;

pub use catalog::{
    close_secondary_menu, close_tertiary_menu, invoke_secondary_menu, invoke_tertiary_menu, CatalogHost, InvokeResult,
    MenuLevel, SecondaryInvoke, TertiaryInvoke, ALIGN_ANCHOR, ALIGN_FLAGS, ALIGN_TARGET_SCREEN,
};
pub use dialog::{
    advice_flag, advisor_popup_instance, close_dialog, dialog_attach_point, popup_closes_on, popup_win_proc,
    AttachPoint, AutoGamePause, HiddenPause, PopupAction, ADVISOR_POPUP_INSTANCE_HIGH_MOOD,
    ADVISOR_POPUP_INSTANCE_LOW_MOOD, ADVISOR_POPUP_MOOD_THRESHOLD,
};
pub use message::{
    deliver, GzMessage, Handled, MsgKind, MSG_ATTACH, MSG_COMMAND, MSG_DETACH, MSG_KEY_DOWN, MSG_KEY_UP, MSG_LINK,
    MSG_MODAL_BEGIN, MSG_MODAL_END, MSG_MODAL_END_POSTED, PARAM_ADVICE,
};
pub use script::{
    iface_is_class_minus_one, sc4_create_window_from_script, service_lookup, ScriptWindowRequest, ScriptWindowResult,
    StoreScriptService, UIScriptService,
};
pub use slots::{
    as_igzwin, RetChannel, SlotInfo, CHILD_DELETE_FORWARDS_TO, IID_CATALOG_STATE, IID_IGZ_WIN, IID_IGZ_WIN_GEN,
    IID_VIEW_MANAGER, SCRIPT_SERVICE_CREATE_SLOT, UI_SCRIPT_GROUP, UI_SCRIPT_IFACE, UI_SCRIPT_SERVICE, VIEW3D_CLASS_ID,
    VIEW3D_OFF_CATALOG_STATE, VIEW3D_OFF_MSG_FILTER, VIEW3D_OFF_PRIMARY_VPTR, VIEW3D_OFF_SECONDARY_KIND,
    VIEW3D_OFF_SECONDARY_WIN, VIEW3D_OFF_TERTIARY_KIND, VIEW3D_OFF_TERTIARY_WIN, VIEW3D_OFF_VIEW_MANAGER,
    VIEW3D_OFF_WIN, VIEW3D_SAVE_GROUP, VIEW3D_SAVE_INSTANCE, VIEW3D_SAVE_TYPE, VIEW3D_SAVE_VERSION, VIEW3D_SLOTS,
    VIEW3D_SLOT_COUNT, VIEW3D_VTABLE, WIN_GEN_VTABLE, WIN_SLOTS,
};
pub use view3d::{CatalogState, SaveRef, View3DSave, View3DWin, VIEW3D_LAYOUT};
pub use win::{
    FilterToken, WinFlags, WinId, WinOp, WinParam, WinStore, WinToken, ZWin, VIEW3D_MSG_FILTER_OFFSET, WIN_GEN_FLAG_1,
    WIN_GEN_FLAG_4,
};
