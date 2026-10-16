// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The message and `DoMessage` contracts of the window layer
//! (ui-windows.md §6.5).
//!
//! # Entry points, in order
//!
//! Every message a window or a dialog sees goes through one of two entry
//! points (modal-dialogs.md §2.6). For a scripted window the order is:
//!
//! 1. the **window procedure** first — `DoWinProcMessage`, then `DoWinMsg`;
//! 2. then the window's own `DoMessage` (`cIGZWin` slot 3);
//! 3. and a type-3 message reaches `GZOnCommand` (`cIGZWin` slot 143) after
//!    both.
//!
//! Conformance test 22 pins that order.
//!
//! # `DoWinProcMessage`'s first argument is the window
//!
//! `DoWinProcMessage(cIGZWin* win, cGZMessage& msg)` — **arg1 is the window**,
//! not the message (§6.5). `0x0076_2D32` loads `arg1` and uses it as `this`
//! for `GetParam`, which is how the advisor tracker reaches the dialog's
//! advice property (`0xEA9F_13C0`). A shim that passes the message instead
//! fails conformance test 21.
//!
//! # There is no keyboard message in the advisor popup's procedure
//!
//! That is why Esc and Enter do nothing (§6.2): the procedure's first act on
//! every message is to read the advice and test its flags, and no keyboard
//! message is one of the kinds it looks at. A click on Close is a **command**
//! message, so the dialog ends.

use super::win::{WinOp, WinStore, WinToken};

/// The message vocabulary this layer switches on (§6.5).
///
/// `cGZMessage` is passed by reference; the three data words are what the
/// table below carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MsgKind {
    /// A command message (type 3), data1 `0x2872_59F6`. data2 is the **button
    /// ID** and data3 its toggle state.
    Command,
    /// The link message `0xE4A8_5D91`.
    Link,
    /// The win-proc attach/detach notices `0xA2BF_8AD5` / `0xA2BF_8AD6`.
    ///
    /// **Swallowed** by the advisor tracker. They are **not** key down/up
    /// (modal-dialogs.md §6).
    AttachNotice,
    DetachNotice,
    /// The legacy modal begin notice `0x62BA_7B42`: data1 the window ID, data2
    /// the window, data3 the nest count **before**.
    ModalBegin,
    /// The legacy modal end notice `0x62BA_7B43`: data3 the nest count
    /// **after**.
    ModalEnd,
    /// GZ type `0x000F` → Win32 `0x0700`, posted: data2 the window (wParam), data3
    /// the result (lParam). Ends the modal loop.
    ModalEndPosted,
    /// A key-down message.
    ///
    /// Present so a vector can *prove* the advisor popup ignores it
    /// (conformance test 20): the dialog handler switches on the kinds above
    /// and never on this one.
    KeyDown,
    /// A key-up message. Same standing as [`MsgKind::KeyDown`].
    KeyUp,
}

impl MsgKind {
    /// The message type word of this kind, as §6.5's table records it.
    pub fn msg_type(self) -> u32 {
        match self {
            MsgKind::Command => MSG_COMMAND,
            MsgKind::Link => MSG_LINK,
            MsgKind::AttachNotice => MSG_ATTACH,
            MsgKind::DetachNotice => MSG_DETACH,
            MsgKind::ModalBegin => MSG_MODAL_BEGIN,
            MsgKind::ModalEnd => MSG_MODAL_END,
            MsgKind::ModalEndPosted => MSG_MODAL_END_POSTED,
            MsgKind::KeyDown => MSG_KEY_DOWN,
            MsgKind::KeyUp => MSG_KEY_UP,
        }
    }

    /// True for the two kinds the advisor tracker swallows (§6.2).
    pub fn is_swallowed_by_advisor(self) -> bool {
        matches!(self, MsgKind::AttachNotice | MsgKind::DetachNotice)
    }

    /// True for a message the advisor popup's procedure looks at.
    ///
    /// §6.2: "handles **no keyboard messages**". A key message produces no
    /// end, which is what makes Esc and Enter do nothing.
    pub fn advisor_handles(self) -> bool {
        !matches!(self, MsgKind::KeyDown | MsgKind::KeyUp)
    }
}

/// `0x2872_59F6`: a button was pressed. data2 is the button ID (§6.5).
pub const MSG_COMMAND: u32 = 0x2872_59F6;
/// `0xE4A8_5D91`: a link (§6.5).
pub const MSG_LINK: u32 = 0xE4A8_5D91;
/// `0xA2BF_8AD5`: win-proc attached (§6.5).
pub const MSG_ATTACH: u32 = 0xA2BF_8AD5;
/// `0xA2BF_8AD6`: win-proc detached (§6.5).
pub const MSG_DETACH: u32 = 0xA2BF_8AD6;
/// `0x62BA_7B42`: modal begin notice, sent (§6.5).
pub const MSG_MODAL_BEGIN: u32 = 0x62BA_7B42;
/// `0x62BA_7B43`: modal end notice, sent (§6.5).
pub const MSG_MODAL_END: u32 = 0x62BA_7B43;
/// GZ type `0x000F`, which is Win32 `0x0700` once posted to the canvas window
/// (§6.5). Ends the modal loop.
pub const MSG_MODAL_END_POSTED: u32 = 0x000F;
/// A key-down. Named only so a vector can show the dialog handler ignores it.
pub const MSG_KEY_DOWN: u32 = 0x0100;
/// A key-up.
pub const MSG_KEY_UP: u32 = 0x0101;

/// The window property a dialog's advice is reached under (§6.2).
///
/// `cIGZWin` slot 113 `GetParam(0xEA9F_13C0, &out)` — the advisor tracker's
/// `DoWinProcMessage` reads it on **every** message before testing flags.
pub const PARAM_ADVICE: u32 = 0xEA9F_13C0;

/// One `cGZMessage`, as this layer reads it (§6.5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GzMessage {
    pub kind: MsgKind,
    /// The message type word. For [`MsgKind::Command`] this is `data1` and
    /// equals [`MSG_COMMAND`].
    pub msg_type: u32,
    /// data1. For a command, `0x2872_59F6`.
    pub data1: u32,
    /// data2. For a command, the **button ID**.
    pub data2: u32,
    /// data3. For a command, the toggle state.
    pub data3: u32,
}

impl GzMessage {
    /// A button command: data1 is `0x2872_59F6`, data2 the button ID.
    pub fn command(button_id: u32, toggle: u32) -> GzMessage {
        GzMessage {
            kind: MsgKind::Command,
            msg_type: MSG_COMMAND,
            data1: MSG_COMMAND,
            data2: button_id,
            data3: toggle,
        }
    }

    /// A link message.
    pub fn link() -> GzMessage {
        GzMessage {
            kind: MsgKind::Link,
            msg_type: MSG_LINK,
            data1: 0,
            data2: 0,
            data3: 0,
        }
    }

    /// A key-down. §6.2: the advisor popup's procedure does not handle it.
    pub fn key_down() -> GzMessage {
        GzMessage {
            kind: MsgKind::KeyDown,
            msg_type: MSG_KEY_DOWN,
            data1: 0,
            data2: 0,
            data3: 0,
        }
    }

    /// A key-up.
    pub fn key_up() -> GzMessage {
        GzMessage {
            kind: MsgKind::KeyUp,
            msg_type: MSG_KEY_UP,
            data1: 0,
            data2: 0,
            data3: 0,
        }
    }

    /// One of the win-proc attach/detach notices (§6.5).
    pub fn attach_notice(attached: bool) -> GzMessage {
        let (kind, msg_type) = if attached {
            (MsgKind::AttachNotice, MSG_ATTACH)
        } else {
            (MsgKind::DetachNotice, MSG_DETACH)
        };
        GzMessage {
            kind,
            msg_type,
            data1: 0,
            data2: 0,
            data3: 0,
        }
    }

    /// The modal begin/end legacy notices (§6.5).
    pub fn modal_notice(end: bool, win_id: u32, win: WinToken, nest: u32) -> GzMessage {
        let (kind, msg_type) = if end {
            (MsgKind::ModalEnd, MSG_MODAL_END)
        } else {
            (MsgKind::ModalBegin, MSG_MODAL_BEGIN)
        };
        GzMessage {
            kind,
            msg_type,
            data1: win_id,
            data2: win as u32,
            data3: nest,
        }
    }

    /// True when a type-3 message: the kind that reaches `GZOnCommand` after
    /// both earlier entry points (§6.5).
    pub fn is_type3(&self) -> bool {
        self.kind == MsgKind::Command
    }
}

/// What a window procedure or a window's `DoMessage` answered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Handled {
    /// The handler took the message; the chain stops.
    ///
    /// The slot returns bool in AL only (§2.2), so "handled" is an AL bool.
    Yes,
    /// Not handled; the chain continues.
    No,
}

impl Handled {
    /// The AL bool a handler returns.
    pub fn as_al(self) -> bool {
        matches!(self, Handled::Yes)
    }
}

/// The delivery chain of §6.5 for one scripted window.
///
/// Returns the stages the message reached, in order, so a conformance vector
/// can assert test 22 without the runner needing to know about windows.
///
/// A win proc's answer stops the chain: the window's own `DoMessage` is only
/// reached when the proc did not handle the message, and `GZOnCommand` is
/// reached only for a type-3 message that got that far. *[choice]* the spec
/// gives the order ("the window procedure first … then the window's own
/// `DoMessage`, and a type-3 message reaches `GZOnCommand` after both") and
/// not the stop rules; we treat a `true` return as "handled" at each stage,
/// which is what `bool` means at both entry points.
pub fn deliver(
    store: &mut WinStore,
    win: WinToken,
    msg: &GzMessage,
    win_proc: impl FnOnce(WinToken, &GzMessage) -> Handled,
    win_proc2: impl FnOnce(WinToken, &GzMessage) -> Handled,
    window: impl FnOnce(WinToken, &GzMessage) -> Handled,
) -> Vec<&'static str> {
    let mut stages = Vec::new();

    // (1) DoWinProcMessage(win, msg) — arg1 is the window (§6.5).
    store.push(WinOp::DoWinProcMessage { win, msg: msg.msg_type });
    stages.push("DoWinProcMessage");
    if win_proc(win, msg).as_al() {
        return stages;
    }

    // (2) DoWinMsg, the second half of the window-procedure entry (§6.5).
    stages.push("DoWinMsg");
    if win_proc2(win, msg).as_al() {
        return stages;
    }

    // (3) The window's own DoMessage (cIGZWin slot 3).
    store.push(WinOp::DoMessage { win, msg: msg.msg_type });
    stages.push("DoMessage");
    if window(win, msg).as_al() {
        return stages;
    }

    // (4) A type-3 message reaches GZOnCommand (slot 143) after both.
    if msg.is_type3() {
        store.push(WinOp::GZOnCommand {
            win,
            command: msg.data1,
            data: msg.data2,
        });
        stages.push("GZOnCommand");
    }
    stages
}

/// `PostMessageToTarget` / `SendMessageToTarget` (§6.5).
///
/// `PostMessageToTarget` (manager slots 14/15) queues under the manager's
/// lock and the manager delivers from its own tick (slot 16), skipping windows
/// that are no longer valid. `SendMessageToTarget` (slots 12/13) delivers
/// synchronously. **Neither returns a status in EAX.**
pub mod delivery {
    /// `SendMessageToTarget`: synchronous, manager slots 12/13.
    pub const SYNCHRONOUS: bool = true;
    /// `PostMessageToTarget`: queued under the manager's lock, delivered from
    /// the manager's own tick (slot 16).
    pub const QUEUED: bool = false;
    /// Neither returns a status in EAX (§6.5).
    pub const RETURNS_STATUS: bool = false;
    /// `PostMsg` (slot 147) returns the manager's slot 15 result, not a
    /// status (§6.5).
    pub const POST_MSG_RETURNS: &str = "manager slot 15 result";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_order_is_proc_then_message_then_command() {
        // Conformance test 22.
        let mut s = WinStore::new();
        let win = s.create(1);
        let msg = GzMessage::command(0, 0);
        s.clear_ops();
        let stages = deliver(
            &mut s,
            win,
            &msg,
            |_, _| Handled::No,
            |_, _| Handled::No,
            |_, _| Handled::No,
        );
        assert_eq!(stages, ["DoWinProcMessage", "DoWinMsg", "DoMessage", "GZOnCommand"]);
        let ops: Vec<&str> = s
            .ops()
            .iter()
            .map(|o| match o {
                WinOp::DoWinProcMessage { .. } => "DoWinProcMessage",
                WinOp::DoMessage { .. } => "DoMessage",
                WinOp::GZOnCommand { .. } => "GZOnCommand",
                _ => "other",
            })
            .collect();
        assert_eq!(ops, ["DoWinProcMessage", "DoMessage", "GZOnCommand"]);
    }

    #[test]
    fn a_non_command_never_reaches_gz_on_command() {
        let mut s = WinStore::new();
        let win = s.create(1);
        let msg = GzMessage::link();
        s.clear_ops();
        let stages = deliver(
            &mut s,
            win,
            &msg,
            |_, _| Handled::No,
            |_, _| Handled::No,
            |_, _| Handled::No,
        );
        assert!(!stages.contains(&"GZOnCommand"));
    }

    #[test]
    fn a_handled_message_stops_the_chain() {
        let mut s = WinStore::new();
        let win = s.create(1);
        let msg = GzMessage::command(0, 0);
        s.clear_ops();
        let stages = deliver(
            &mut s,
            win,
            &msg,
            |_, _| Handled::Yes,
            |_, _| Handled::No,
            |_, _| Handled::No,
        );
        assert_eq!(stages, ["DoWinProcMessage"]);
    }

    #[test]
    fn keyboard_messages_are_not_advisor_handled() {
        // Conformance test 20's premise (§6.2).
        assert!(!MsgKind::KeyDown.advisor_handles());
        assert!(!MsgKind::KeyUp.advisor_handles());
        assert!(MsgKind::Command.advisor_handles());
        assert!(MsgKind::Link.advisor_handles());
    }

    #[test]
    fn attach_detach_are_swallowed() {
        assert!(MsgKind::AttachNotice.is_swallowed_by_advisor());
        assert!(MsgKind::DetachNotice.is_swallowed_by_advisor());
        assert!(!MsgKind::Command.is_swallowed_by_advisor());
    }

    #[test]
    fn command_data1_is_the_message_type() {
        let m = GzMessage::command(0x8A8D_FD4E, 1);
        assert_eq!(m.data1, MSG_COMMAND);
        assert_eq!(m.data2, 0x8A8D_FD4E, "data2 is the button ID");
        assert_eq!(m.data3, 1);
        assert!(m.is_type3());
    }
}
