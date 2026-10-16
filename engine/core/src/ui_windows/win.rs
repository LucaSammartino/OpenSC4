// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The window object: what a `cIGZWin` *is* and the slot set the window layer
//! actually uses on it (ui-windows.md §2).
//!
//! Three interfaces, one object (§2.1): the framework window `cIGZWin` (IID
//! `0x22BA0121`), its generator interface `cIGZWinGen` (IID `0x5386D516`) and
//! the manager `cIGZWinMgr` that owns it. A game window such as
//! [`crate::ui_windows::view3d::View3DWin`] is **not** a `cGZWinGen`: it
//! *contains* one, reached through a second vtable on the same allocation.
//!
//! # `ChildDelete` destroys (§2.3)
//!
//! `cIGZWin::ChildDelete` is four instructions that tail-forward, through the
//! object held at the window's `+4`, to that object's slot 23 — the manager's
//! `DestroyWindow`. So it **destroys** the child, not merely detaches it, and
//! it is **not overridable** on the window (quirk 2). [`ZWin::child_delete`]
//! is therefore implemented as a manager call, never as a window method.
//!
//! # Threading (HANDOFF §5.1)
//!
//! The game calls getters from a second thread while the main thread is in
//! `OnTick`. The window tree is main-thread UI state: the original's window
//! layer is driven from the message loop only. Getters here are plain `&self`
//! reads of an immutable snapshot token; the tree itself is mutated only from
//! the message loop, which is the same thread the conformance runner drives.
//! *[choice]* we do not lock the tree: a caller that reads window state from a
//! second thread must copy what it needs on the main thread first.

use std::collections::BTreeMap;

/// A stable window identity within one [`WinStore`].
///
/// Not a `cIGZWin*`: the game's pointers are per-allocation and are invalidated
/// by `DestroyWindow`. Tokens survive a reorder of the store and are what a
/// conformance vector names a window by.
pub type WinToken = u64;

/// A window's user-visible id (`cIGZWin` slot 63 `GetID`).
pub type WinId = u32;

/// A message filter's identity inside a window's filter list.
pub type FilterToken = u64;

/// What `cIGZWin::GetParam` (slot 113) hands back for a key.
///
/// The advisor popup's advice is reached this way under key `0xEA9F13C0`
/// (§6.2); the window layer does not interpret the value.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WinParam {
    /// A scalar the caller reads as a dword.
    U32(u32),
    /// An opaque handle: the caller's own token, not a game pointer.
    Handle(u64),
}

impl WinParam {
    /// The dword view of this parameter, if it is a scalar.
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            WinParam::U32(v) => Some(*v),
            WinParam::Handle(_) => None,
        }
    }
}

/// One window, as `cIGZWin`'s used subset models it (§2.2).
#[derive(Clone, Debug)]
pub struct ZWin {
    token: WinToken,
    id: WinId,
    parent: Option<WinToken>,
    children: Vec<WinToken>,
    /// The `GetFlag`/`SetFlag` bitset (slots 67/68).
    flags: u32,
    params: BTreeMap<u32, WinParam>,
    /// Message filters, in attach order (slots 116/117).
    filters: Vec<FilterToken>,
    shown: bool,
    /// `AddRef`/`Release` count (slots 1/2).
    refs: i32,
    /// The object at `+4` that `ChildDelete` forwards through (§2.3). A
    /// replacement stores its manager handle here; the original's field
    /// identity is *[inferred]* to be the window manager (ui-windows.md
    /// UIW-7).
    manager: u64,
}

impl ZWin {
    fn new(token: WinToken, id: WinId) -> ZWin {
        ZWin {
            token,
            id,
            parent: None,
            children: Vec::new(),
            flags: 0,
            params: BTreeMap::new(),
            filters: Vec::new(),
            shown: false,
            // The store holds the creation reference, as the game does for a
            // window it has just built.
            refs: 1,
            manager: 0,
        }
    }

    pub fn token(&self) -> WinToken {
        self.token
    }

    /// `GetID` (slot 63).
    pub fn id(&self) -> WinId {
        self.id
    }

    /// `SetID` (slot 64).
    pub fn set_id(&mut self, id: WinId) {
        self.id = id;
    }

    /// `GetParentWin` (slot 11).
    pub fn parent(&self) -> Option<WinToken> {
        self.parent
    }

    /// The children, in `ChildAdd` order.
    pub fn children(&self) -> &[WinToken] {
        &self.children
    }

    /// `GetFlag` (slot 67): `(flags & mask) != 0`, returned as 0/1 in EAX.
    pub fn get_flag(&self, mask: u32) -> u32 {
        u32::from(self.flags & mask != 0)
    }

    /// `SetFlag` (slot 68): sets or clears `mask`, returns 0/1 in EAX.
    pub fn set_flag(&mut self, mask: u32, on: bool) -> u32 {
        if on {
            self.flags |= mask;
        } else {
            self.flags &= !mask;
        }
        self.get_flag(mask)
    }

    /// `GetParam` (slot 113). Returns false in AL when the key is absent.
    ///
    /// The slot's return is `al` (§2.2), so a caller reads only the bool.
    pub fn get_param(&self, key: u32) -> Option<&WinParam> {
        self.params.get(&key)
    }

    /// The other half of slot 113's pair, for the store to fill in.
    pub fn set_param(&mut self, key: u32, value: WinParam) {
        self.params.insert(key, value);
    }

    /// The attached message filters, in attach order.
    pub fn filters(&self) -> &[FilterToken] {
        &self.filters
    }

    pub fn is_shown(&self) -> bool {
        self.shown
    }

    pub fn refs(&self) -> i32 {
        self.refs
    }

    /// The `+4` field `ChildDelete` forwards through (§2.3).
    pub fn manager_handle(&self) -> u64 {
        self.manager
    }

    /// Add a child (slot 14). Returns false in AL when it is already a child.
    ///
    /// *[choice]* the original's rejection cases were not read; we reject a
    /// duplicate and a self-parent, and accept everything else.
    fn child_add(&mut self, child: WinToken) -> bool {
        if child == self.token || self.children.contains(&child) {
            return false;
        }
        self.children.push(child);
        true
    }

    /// Remove a child (slot 15) — **detach only**, no destruction.
    fn child_remove(&mut self, child: WinToken) -> bool {
        match self.children.iter().position(|c| *c == child) {
            Some(i) => {
                self.children.remove(i);
                true
            }
            None => false,
        }
    }
}

/// One observable step the window layer took.
///
/// The conformance runner turns these into `expect_events` JSON; keeping them
/// as a first-class type is what makes teardown **order** checkable
/// (conformance tests 6, 7, 13, 14, 19).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WinOp {
    /// `ChildAdd` (slot 14).
    ChildAdd { parent: WinToken, child: WinToken, ok: bool },
    /// `ChildRemove` (slot 15): detach only.
    ChildRemove { parent: WinToken, child: WinToken, ok: bool },
    /// `ChildDelete` (slot 16): forwarded to the manager's `DestroyWindow`.
    ///
    /// Emitted once per `ChildDelete`, and the `DestroyWindow` it forwards to
    /// emits [`WinOp::DestroyWindow`] after it.
    ChildDelete { parent: WinToken, child: WinToken },
    /// The manager's `DestroyWindow` (slot 23).
    DestroyWindow(WinToken),
    /// `HideWindow` (slot 70).
    HideWindow(WinToken),
    /// `ShowWindow` (slot 69).
    ShowWindow(WinToken),
    /// `PullToFront` (slot 23 on `cIGZWin`).
    PullToFront(WinToken),
    /// `AddMessageFilter` (slot 116) — the filter is an inline sub-object, so
    /// the op records the field offset the game takes the address of (§3.2).
    AddMessageFilter { win: WinToken, offset: u32 },
    /// `RemoveMessageFilter` (slot 117).
    RemoveMessageFilter { win: WinToken, offset: u32 },
    /// `cISC4View3DWin` slot 27 `RemoveAllViewInputControls` — the tool drop
    /// §4.3 runs first.
    RemoveAllViewInputControls(u32),
    /// `SC4CreateWindowFromScript` was asked to build a window.
    CreateWindow { parent: WinToken, group: u32, instance: u32 },
    /// The factory returned this window (or 0).
    WindowCreated { token: WinToken, ok: bool },
    /// `nSC4UI::AlignWindowToWindow` (§4.3).
    AlignWindowToWindow { win: u32, anchor: u32, target: u32, flags: u32 },
    /// `cIGZWinGen::SetWinGenFlag(flag, value)` (§4.3).
    SetWinGenFlag { flag: u32, value: bool },
    /// `QueryInterface` on a window (slot 0).
    QueryInterface { win: WinToken, iid: u32, ok: bool },
    /// The interface handed back by a successful query was released.
    Release(WinToken),
    /// `cIGZWin` slot 3 `DoMessage` (§6.5). Returns bool in AL only.
    DoMessage { win: WinToken, msg: u32 },
    /// `cIGZWinProc::DoWinProcMessage(win, msg)` — **arg1 is the window**
    /// (§6.5, conformance test 21).
    DoWinProcMessage { win: WinToken, msg: u32 },
    /// `cIGZWin` slot 143 `GZOnCommand`.
    GZOnCommand { win: WinToken, command: u32, data: u32 },
    /// `cIGZWinMgr::IsWindowValid` (slot 24).
    IsWindowValid(WinToken),
    /// A catalog state dword was written through `view+0x208` (§4.1).
    CatalogStateWrite { offset: u32, value: u32 },
    /// `cIGZWin::GetID` was read (slot 63).
    GetID(WinToken),
    /// The window's `cIGZWinGen` interface was queried (IID `0x5386D516`).
    QueryWinGen(WinToken),
}

/// The window store: the tree plus the manager bookkeeping `ChildDelete`
/// forwards into (§2.3).
#[derive(Debug)]
pub struct WinStore {
    wins: BTreeMap<WinToken, ZWin>,
    next: WinToken,
    /// The manager's busy count. `DestroyWindow` is deferred to the condemned
    /// list while this is non-zero (§2.3).
    busy: u32,
    /// Windows condemned while `busy > 0`, destroyed when it returns to 0.
    condemned: Vec<WinToken>,
    ops: Vec<WinOp>,
}

impl Default for WinStore {
    fn default() -> Self {
        Self::new()
    }
}

impl WinStore {
    pub fn new() -> WinStore {
        WinStore {
            wins: BTreeMap::new(),
            // Token 0 is never handed out: it reads as "null pointer".
            next: 1,
            busy: 0,
            condemned: Vec::new(),
            ops: Vec::new(),
        }
    }

    /// Every op taken since the last [`WinStore::clear_ops`], in order.
    pub fn ops(&self) -> &[WinOp] {
        &self.ops
    }

    pub fn clear_ops(&mut self) {
        self.ops.clear();
    }

    /// Record an observable step. `pub(crate)` so the sibling modules of this
    /// core can log through the store without a second channel.
    pub(crate) fn push(&mut self, op: WinOp) {
        self.ops.push(op);
    }

    /// Create a window with the next token and the given `GetID` value.
    pub fn create(&mut self, id: WinId) -> WinToken {
        let token = self.next;
        self.next += 1;
        self.wins.insert(token, ZWin::new(token, id));
        token
    }

    /// Create a window that carries the manager handle `ChildDelete` forwards
    /// through (§2.3).
    pub fn create_with_manager(&mut self, id: WinId, manager: u64) -> WinToken {
        let token = self.create(id);
        if let Some(w) = self.wins.get_mut(&token) {
            w.manager = manager;
        }
        token
    }

    /// Adopt a window the host made outside this store (§5.1's factory
    /// returns a `cIGZWin*` the store has never seen) and parent it.
    ///
    /// Returns false when the token is already taken or is 0.
    pub fn adopt(&mut self, token: WinToken, id: WinId, parent: WinToken) -> bool {
        if token == 0 || self.wins.contains_key(&token) {
            return false;
        }
        let mut w = ZWin::new(token, id);
        w.parent = if parent != 0 { Some(parent) } else { None };
        self.wins.insert(token, w);
        if parent != 0 {
            if let Some(p) = self.wins.get_mut(&parent) {
                p.child_add(token);
            }
        }
        true
    }

    pub fn get(&self, token: WinToken) -> Option<&ZWin> {
        self.wins.get(&token)
    }

    pub fn get_mut(&mut self, token: WinToken) -> Option<&mut ZWin> {
        self.wins.get_mut(&token)
    }

    /// True while the manager considers the window live (`IsWindowValid`,
    /// slot 24).
    pub fn is_window_valid(&self, token: WinToken) -> bool {
        self.wins.contains_key(&token)
    }

    /// `AddRef` (slot 1).
    pub fn add_ref(&mut self, token: WinToken) -> Option<u32> {
        let w = self.wins.get_mut(&token)?;
        w.refs += 1;
        Some(w.refs as u32)
    }

    /// `Release` (slot 2).
    pub fn release(&mut self, token: WinToken) -> Option<u32> {
        let w = self.wins.get_mut(&token)?;
        w.refs -= 1;
        let count = w.refs as u32;
        self.push(WinOp::Release(token));
        Some(count)
    }

    /// `ChildAdd` (slot 14). Returns false in AL on rejection.
    pub fn child_add(&mut self, parent: WinToken, child: WinToken) -> bool {
        let ok = match self.wins.get_mut(&parent) {
            Some(p) => p.child_add(child),
            None => false,
        };
        if ok {
            if let Some(c) = self.wins.get_mut(&child) {
                c.parent = Some(parent);
            }
        }
        self.push(WinOp::ChildAdd { parent, child, ok });
        ok
    }

    /// `ChildRemove` (slot 15): detach only. The child stays alive.
    pub fn child_remove(&mut self, parent: WinToken, child: WinToken) -> bool {
        let ok = match self.wins.get_mut(&parent) {
            Some(p) => p.child_remove(child),
            None => false,
        };
        if ok {
            if let Some(c) = self.wins.get_mut(&child) {
                c.parent = None;
            }
        }
        self.push(WinOp::ChildRemove { parent, child, ok });
        ok
    }

    /// `ChildDelete` (slot 16): the four-instruction tail forward of §2.3.
    ///
    /// It does **not** run on the window. It loads the manager out of the
    /// window's `+4` field and tail-calls the manager's `DestroyWindow`
    /// (slot 23). Quirk 2: not overridable on the window, so a subclass
    /// cannot intercept its own child's destruction.
    ///
    /// Returns nothing, like the slot: the body has no value to return.
    pub fn child_delete(&mut self, parent: WinToken, child: WinToken) {
        self.push(WinOp::ChildDelete { parent, child });
        // The tail forward. Emitting DestroyWindow separately is what makes
        // "ChildDelete destroys" observable rather than asserted.
        self.destroy_window(child);
    }

    /// `cIGZWinMgr::DestroyWindow` (slot 23): destroy now if the manager is
    /// idle, else condemn for later (§2.3).
    pub fn destroy_window(&mut self, token: WinToken) {
        if self.busy > 0 {
            if !self.condemned.contains(&token) {
                self.condemned.push(token);
            }
            return;
        }
        self.destroy_now(token);
    }

    fn destroy_now(&mut self, token: WinToken) {
        if self.wins.remove(&token).is_none() {
            return;
        }
        self.push(WinOp::DestroyWindow(token));
        // Orphans go with it: a destroyed parent's children are unreachable.
        let orphans: Vec<WinToken> = self
            .wins
            .values()
            .filter(|w| w.parent == Some(token))
            .map(|w| w.token)
            .collect();
        for o in orphans {
            self.destroy_now(o);
        }
    }

    /// Raise the manager's busy count (entering a modal loop or a tick).
    pub fn begin_busy(&mut self) {
        self.busy += 1;
    }

    /// Lower it; at zero, everything condemned is destroyed.
    pub fn end_busy(&mut self) {
        self.busy = self.busy.saturating_sub(1);
        if self.busy == 0 {
            let done = std::mem::take(&mut self.condemned);
            for t in done {
                self.destroy_now(t);
            }
        }
    }

    pub fn busy(&self) -> u32 {
        self.busy
    }

    /// `ShowWindow` (slot 69).
    pub fn show_window(&mut self, token: WinToken) {
        if let Some(w) = self.wins.get_mut(&token) {
            w.shown = true;
        }
        self.push(WinOp::ShowWindow(token));
    }

    /// `HideWindow` (slot 70).
    pub fn hide_window(&mut self, token: WinToken) {
        if let Some(w) = self.wins.get_mut(&token) {
            w.shown = false;
        }
        self.push(WinOp::HideWindow(token));
    }

    /// `PullToFront` (slot 23 on `cIGZWin`).
    pub fn pull_to_front(&mut self, token: WinToken) {
        self.push(WinOp::PullToFront(token));
    }

    /// `AddMessageFilter` (slot 116). The filter is an **inline sub-object**,
    /// so the caller passes the address of a field (§3.2, `+0x08`).
    pub fn add_message_filter(&mut self, token: WinToken, offset: u32) -> bool {
        let ok = match self.wins.get_mut(&token) {
            Some(w) => {
                if w.filters.contains(&u64::from(offset)) {
                    false
                } else {
                    w.filters.push(u64::from(offset));
                    true
                }
            }
            None => false,
        };
        self.push(WinOp::AddMessageFilter { win: token, offset });
        ok
    }

    /// `RemoveMessageFilter` (slot 117).
    pub fn remove_message_filter(&mut self, token: WinToken, offset: u32) -> bool {
        let ok = match self.wins.get_mut(&token) {
            Some(w) => match w.filters.iter().position(|f| *f == u64::from(offset)) {
                Some(i) => {
                    w.filters.remove(i);
                    true
                }
                None => false,
            },
            None => false,
        };
        self.push(WinOp::RemoveMessageFilter { win: token, offset });
        ok
    }

    /// `GetChildWindowFromID` (slot 34).
    pub fn child_from_id(&self, parent: WinToken, id: WinId) -> Option<WinToken> {
        let p = self.wins.get(&parent)?;
        p.children.iter().copied().find(|c| {
            self.wins.get(c).map(|w| w.id) == Some(id)
        })
    }

    /// `GetID` (slot 63).
    pub fn get_id(&self, token: WinToken) -> Option<WinId> {
        let w = self.wins.get(&token)?;
        Some(w.id)
    }

    /// `IsWindowValid` (slot 24).
    pub fn check_window_valid(&mut self, token: WinToken) -> bool {
        let ok = self.is_window_valid(token);
        self.push(WinOp::IsWindowValid(token));
        ok
    }

    /// `DoMessage` (slot 3): returns bool in AL only (§2.2, §6.5).
    pub fn do_message(&mut self, token: WinToken, msg: u32) -> bool {
        self.push(WinOp::DoMessage { win: token, msg });
        // *[choice]* a window with no win proc reports "unhandled", which is
        // what makes the delivery chain of §6.5 observable end to end.
        false
    }

    /// `GZOnCommand` (slot 143). Returns nothing useful (§2.2).
    pub fn gz_on_command(&mut self, token: WinToken, command: u32, data: u32) {
        self.push(WinOp::GZOnCommand { win: token, command, data });
    }

    /// `cIGZWinProc::DoWinProcMessage(win, msg)` — arg1 is the **window**,
    /// not the message (§6.5). Conformance test 21 fails a shim that swaps
    /// them.
    pub fn do_win_proc_message(&mut self, win: WinToken, msg: u32) -> bool {
        self.push(WinOp::DoWinProcMessage { win, msg });
        false
    }

    /// `QueryInterface` (slot 0) on a window. Returns bool in AL only (§2.2).
    ///
    /// §4.1's call is the **2-argument** form (`iid, &out`), not the SDK's
    /// 3-argument COM form: the callee writes the out pointer.
    pub fn query_interface(&mut self, token: WinToken, iid: u32) -> bool {
        let ok = self.wins.contains_key(&token)
            && matches!(iid, super::slots::IID_IGZ_WIN | super::slots::IID_IGZ_WIN_GEN);
        self.push(WinOp::QueryInterface { win: token, iid, ok });
        ok
    }

    /// `QueryInterface(0x5386D516)` for the generator interface (§4.3).
    pub fn query_win_gen(&mut self, token: WinToken) -> bool {
        self.push(WinOp::QueryWinGen(token));
        self.wins.contains_key(&token)
    }
}

/// The `cIGZWin::GetFlag`/`SetFlag` bit set the window layer names.
///
/// *[choice]* the flag **bit meanings** are not in the spec (ui-windows.md
/// UIW-4 covers the distinct `cIGZWinGen` generator flags). We model the
/// mask as an opaque bitset exactly as slot 67/68 do.
pub type WinFlags = u32;

/// `cIGZWinGen::SetWinGenFlag`'s two flags (§4.3), both cleared on a
/// successful catalog-menu create.
///
/// *[inferred]* from the Mac name (`SetWinGenFlag(long, __bool8)`) that the
/// slot pair is `(flag, value)`; the flag bit meanings are ui-windows.md
/// UIW-4.
pub const WIN_GEN_FLAG_1: u32 = 1;
pub const WIN_GEN_FLAG_4: u32 = 4;

/// The `cIGZWinMessageFilter` sub-object offset on a `cSC4View3DWin` (§3.2).
///
/// `InvokeSecondaryMenu` does `lea eax,[edi+8]; call [edx+0x1d0]` — it passes
/// the **address** of the field, so the filter is inline at `+8`, not a
/// pointer. Its vtable is not read (ui-windows.md UIW-9).
pub const VIEW3D_MSG_FILTER_OFFSET: u32 = 0x08;

/// The slot a `cIGZWinProc` entry point is reached through is not a vtable
/// slot: `DoWinProcMessage` is the object a window's `SetWinProc` attached
/// (§6.5). This names its argument convention so a binding cannot get it
/// wrong.
pub mod winproc {
    use super::WinToken;

    /// `DoWinProcMessage(win, msg)`. **Arg1 is the window.**
    ///
    /// §6.5: `0x00762D32` loads `arg1` and uses it as `this` for `GetParam`,
    /// which is how the advisor tracker reaches the dialog's advice property
    /// (`0xEA9F13C0`).
    pub const ARG1_IS_WINDOW: bool = true;
    /// Returns bool (§6.5 table).
    pub type Ret = bool;
    /// The window argument's type, for signatures.
    pub type Win = WinToken;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_remove_detaches_and_child_delete_destroys() {
        // Conformance test 7's foundation: ChildRemove only detaches,
        // ChildDelete destroys (§2.3).
        let mut s = WinStore::new();
        let parent = s.create(1);
        let a = s.create(2);
        let b = s.create(3);
        assert!(s.child_add(parent, a));
        assert!(s.child_add(parent, b));

        s.clear_ops();
        assert!(s.child_remove(parent, a));
        assert!(s.is_window_valid(a), "ChildRemove must not destroy");
        assert_eq!(s.get(parent).unwrap().children(), &[b]);

        s.clear_ops();
        s.child_delete(parent, b);
        assert!(!s.is_window_valid(b), "ChildDelete must destroy");
        assert!(s.is_window_valid(parent));
    }

    #[test]
    fn child_delete_logs_the_forward_to_destroy_window() {
        let mut s = WinStore::new();
        let parent = s.create(1);
        let child = s.create(2);
        s.child_add(parent, child);
        s.clear_ops();
        s.child_delete(parent, child);
        assert_eq!(
            s.ops(),
            &[
                WinOp::ChildDelete { parent, child },
                WinOp::DestroyWindow(child),
            ]
        );
    }

    #[test]
    fn destroy_is_deferred_while_busy() {
        let mut s = WinStore::new();
        let w = s.create(1);
        s.begin_busy();
        s.destroy_window(w);
        assert!(s.is_window_valid(w), "condemned, not yet destroyed");
        s.end_busy();
        assert!(!s.is_window_valid(w));
    }

    #[test]
    fn get_flag_set_flag_return_0_or_1() {
        let mut s = WinStore::new();
        let w = s.create(1);
        assert_eq!(s.get(w).unwrap().get_flag(0x4), 0);
        assert_eq!(s.get_mut(w).unwrap().set_flag(0x4, true), 1);
        assert_eq!(s.get(w).unwrap().get_flag(0x4), 1);
        assert_eq!(s.get_mut(w).unwrap().set_flag(0x4, false), 0);
    }

    #[test]
    fn as_igzwin_layout_matches_the_measurement() {
        // Two vtables, one allocation (conformance test 2): the view's
        // primary vptr and the window sub-object's are different, and the
        // sub-object sits at +0x0C.
        use super::super::slots::{as_igzwin, VIEW3D_OFF_WIN, VIEW3D_VTABLE, WIN_GEN_VTABLE};
        assert_ne!(VIEW3D_VTABLE, WIN_GEN_VTABLE);
        assert_eq!(VIEW3D_OFF_WIN, 0x0C);
        assert_eq!(as_igzwin(0x2000), 0x200C);
    }
}
