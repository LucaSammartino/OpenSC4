// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Dialogs, the pause hook and the close path (ui-windows.md §6).
//!
//! This section of the spec is a cross-reference plus four corrections: the
//! behaviour itself is in modal-dialogs.md (the modal loop, and how a plugin
//! ends a dialog) and advisors.md §6.8.1 (the advisor popup's gates, queue and
//! pause). What this module models is where that behaviour sits in the window
//! tree, and the four corrections.
//!
//! # Why Esc and Enter do nothing (§6.2)
//!
//! `cSC4AdvisorPopupTracker::DoWinProcMessage` (`0x00762D30`) handles **no
//! keyboard messages**. Its first act on every message is to read the advice
//! the dialog is showing through the window property [`PARAM_ADVICE`]
//! (`0xEA9F13C0`, `cIGZWin` slot 113 `GetParam`) and then test that advice's
//! flags. A click on Close is a **command message** (data1 `0x287259F6`), so
//! the dialog ends. `docs/manual-checks.md` run 48 records the observation;
//! this is the code that makes it true.
//!
//! **Corrects the premise about coordinates.** The close path is
//! **message-based, not coordinate-based**. There is no pixel test anywhere in
//! `0x00762D30`. If a run clicked at a client point, what the game sees is the
//! command message, not the coordinates.
//!
//! # The pause hook is not a popup counter (§6.4)
//!
//! `cSC4AutoGamePause` is used by more than dialogs: `cSC4App::FullGamePause`,
//! `PartialCloseCity`, `cGZMiscCommandSystemSupport::DoMessage`,
//! `SC4WriteCityRegionViewThumbnail`, `cSC4View3DWin::DoTemporarySimHiddenPause`
//! (camera moves), city-situation popups and saves. The hidden count is a
//! **nesting count**, so a harness must not read it as "how many advisor
//! popups are open" (conformance test 23).

use super::message::{GzMessage, Handled, MsgKind};
use super::win::{WinToken, WinStore};

/// The window property the advice is reached under (§6.2).
pub use super::message::PARAM_ADVICE as ADVICE_PARAM;

/// The UI script instance an advisor popup is built from (§6.1 step 3).
///
/// The same group `0x96A006B0` the catalog menus use (§4.3).
pub const ADVISOR_POPUP_INSTANCE_LOW_MOOD: u32 = 0x4A5A_89D4;
/// The instance for mood >= 3 (§6.1).
pub const ADVISOR_POPUP_INSTANCE_HIGH_MOOD: u32 = 0x4A5A_89D5;
/// The mood boundary between the two instances (§6.1).
pub const ADVISOR_POPUP_MOOD_THRESHOLD: i32 = 3;

/// The advice flags the dialog handler tests (§6.2).
///
/// Meaning of the bits is advisors.md's; this layer only reproduces the two
/// the close path looks at.
pub mod advice_flag {
    /// Flag 2: the advice is still active.
    pub const ACTIVE: u32 = 2;
    /// Flag 8: expire requested.
    pub const EXPIRE: u32 = 8;
}

/// What the advisor popup's win proc decided to do with a message (§6.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PopupAction {
    /// Leave the dialog open.
    KeepOpen,
    /// End the dialog (`cSC4AdvisorPopupTracker::EndDialog`, `0x00762CD0`).
    End,
    /// Follow the link (`cSC4HTMLLinkHandler::DoURL`), then end it.
    FollowLinkThenEnd,
    /// Swallowed: the win-proc attach/detach notices `0xA2BF8AD5` /
    /// `0xA2BF8AD6`. They are **not** key down/up (modal-dialogs.md §6).
    Swallow,
    /// The message is one the procedure does not look at at all — a keyboard
    /// message. Nothing happens, which is why Esc and Enter do nothing.
    Ignored,
}

/// `cSC4AdvisorPopupTracker::DoWinProcMessage` (`0x00762D30`) as a pure
/// decision over an advice's flags (§6.2).
///
/// The first act on every message is to read the advice; then:
///
/// | Advice state | Action |
/// |---|---|
/// | `CheckFlags(2)` false (no longer active) | end the dialog |
/// | `CheckFlags(2)` true **and** `CheckFlags(8)` true | end it |
/// | `CheckFlags(2)` true and `CheckFlags(8)` false | leave it open |
/// | a command message (type 3, data1 `0x287259F6`) | end it — **any** button ID |
/// | a link message `0xE4A85D91` | follow the link, then end it |
/// | types `0xA2BF8AD5` / `0xA2BF8AD6` | swallowed |
pub fn popup_win_proc(advice_flags: u32, msg: &GzMessage) -> PopupAction {
    // A keyboard message is never looked at (§6.2). This is the whole of
    // "Esc and Enter do nothing".
    if !msg.kind.advisor_handles() {
        return PopupAction::Ignored;
    }

    if msg.kind.is_swallowed_by_advisor() {
        return PopupAction::Swallow;
    }

    // The flag tests run first on every message the procedure does look at.
    let active = advice_flags & advice_flag::ACTIVE != 0;
    let expire = advice_flags & advice_flag::EXPIRE != 0;
    if !active {
        return PopupAction::End;
    }
    if expire {
        return PopupAction::End;
    }

    match msg.kind {
        // Quirk 7: **any** button ID ends an advisor popup, not a specific
        // Close control. A harness that sends button 0 is correct for them
        // and wrong for two other dialog families that ignore it (§6.3).
        MsgKind::Command => PopupAction::End,
        MsgKind::Link => PopupAction::FollowLinkThenEnd,
        // Still active, no expire: leave it open.
        _ => PopupAction::KeepOpen,
    }
}

/// Whether a message closes an advisor popup (§6.2, conformance test 20).
pub fn popup_closes_on(advice_flags: u32, msg: &GzMessage) -> bool {
    matches!(
        popup_win_proc(advice_flags, msg),
        PopupAction::End | PopupAction::FollowLinkThenEnd
    )
}

/// The scoped hidden pause an advisor popup takes (§6.1 step 4).
///
/// `cSC4AutoGamePause(sound no, animation no, simulator yes, clock yes)`
/// (`0x005F9320`/`0x005F9360`) → `SC4GamePause` (`0x005F91C0`) →
/// `cISC4Simulator` slot 10 `HiddenPause()` and the city clock's
/// `HiddenPause(1)`. `IsPaused()` stays **false**; `IsHiddenPaused()` becomes
/// **true**.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AutoGamePause {
    pub sound: bool,
    pub animation: bool,
    pub simulator: bool,
    pub clock: bool,
}

impl AutoGamePause {
    /// The pause an advisor popup takes (§6.1 step 4): simulator and clock
    /// only.
    pub fn advisor_popup() -> AutoGamePause {
        AutoGamePause { sound: false, animation: false, simulator: true, clock: true }
    }

    /// The pause `cSC4View3DWin::DoTemporarySimHiddenPause` (`0x007E5B40`)
    /// takes for a camera move (§6.4). It raises the **same** count with no
    /// dialog open — which is why a harness must not equate the two
    /// (conformance test 23).
    pub fn camera_move() -> AutoGamePause {
        AutoGamePause { sound: false, animation: false, simulator: true, clock: true }
    }
}

/// The hidden-pause nesting counter (§6.4).
///
/// `cISC4Simulator` slot 16 `IsHiddenPaused` (`0x00703D60`) is shared with the
/// clock and answers "count at `+0x34` > 0". The count is a **nesting count**,
/// so a second `hidden` pause while one is open leaves it at 2 and ending one
/// leaves it at 1 — never at "one popup open".
#[derive(Debug, Default)]
pub struct HiddenPause {
    count: u32,
    /// How many pauses were taken for dialogs, for a vector that shows the
    /// two are not the same set (§6.4).
    dialog_pauses: u32,
}

impl HiddenPause {
    pub fn new() -> HiddenPause {
        HiddenPause::default()
    }

    /// Take a scoped hidden pause (slot 10 `HiddenPause` + the clock's).
    pub fn enter(&mut self, kind: AutoGamePause, is_dialog: bool) {
        if kind.simulator {
            self.count += 1;
            if is_dialog {
                self.dialog_pauses += 1;
            }
        }
    }

    /// Release one level.
    pub fn leave(&mut self, kind: AutoGamePause, is_dialog: bool) {
        if kind.simulator {
            self.count = self.count.saturating_sub(1);
            if is_dialog {
                self.dialog_pauses = self.dialog_pauses.saturating_sub(1);
            }
        }
    }

    /// `IsHiddenPaused()`: count at `+0x34` > 0.
    pub fn is_hidden_paused(&self) -> bool {
        self.count > 0
    }

    /// `IsPaused()`: **false** while a hidden pause is in force (§6.1).
    ///
    /// A hidden pause is not a normal pause: the date keeps its own rule and
    /// the simulator is only held invisibly.
    pub fn is_paused(&self) -> bool {
        false
    }

    /// The nesting count itself.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// How many of the levels were taken for dialogs.
    pub fn dialog_pauses(&self) -> u32 {
        self.dialog_pauses
    }
}

/// The dialog close-path rule for `close_dialogs = 1` (§6.3).
///
/// **Corrects** `docs/harness.md`'s "end each game modal dialog by its default
/// button" and `docs/PROGRESS.md`'s "presses each modal dialog's default
/// button": button ID 0 is **not** necessarily the default. For a
/// confirmation, 0 is "no" (yes is `0x8A8DFD4E`); for a scripted dialog 0 is
/// "cancel"; two dialog families **ignore** button 0 entirely and need the
/// `EndModal` fallback.
///
/// A spec-conformant description is: *the closer posts command `0x287259F6`
/// with button ID 0, and falls back to `EndModal(window, 0)` if no end notice
/// arrives*.
pub mod close_dialogs {
    use super::super::slots::mgr_slot;

    /// The button ID the closer posts.
    pub const BUTTON_ID: u32 = 0;
    /// The confirmation's "yes" button — **not** 0 (§6.3).
    pub const CONFIRM_YES: u32 = 0x8A8D_FD4E;
    /// What button 0 means for a confirmation.
    pub const CONFIRM_MEANING: &str = "no";
    /// What button 0 means for a scripted dialog.
    pub const SCRIPTED_MEANING: &str = "cancel";
    /// How many dialog families ignore button 0 (§6.3).
    pub const FAMILIES_IGNORING_BUTTON_ZERO: u32 = 2;
    /// The fallback when no end notice arrives: `EndModal(window, 0)`.
    pub const FALLBACK_SLOT: u32 = mgr_slot::END_MODAL;
    /// The fallback's result argument.
    pub const FALLBACK_RESULT: u32 = 0;
}

/// A modal dialog's place in the window tree (§5.2).
///
/// A dialog with no parent is attached to the **main window** by `DoModalWin`
/// (modal-dialogs.md §2.2 step 1). A catalog menu's parent is the **view's own
/// window sub-object** (§4.3). Those two are the measured facts; the rest of
/// the tree is composition inferred from them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttachPoint {
    /// Attached to the canvas / main window: what `DoModalWin` does with a
    /// parentless dialog.
    MainWindow,
    /// A child of the view's `cIGZWin` sub-object: what a catalog menu is.
    ViewWindow(WinToken),
}

/// The window tree of a city screen (§5.2), as the two measured facts
/// constrain it.
pub fn dialog_attach_point(has_parent: bool, view_win: WinToken) -> AttachPoint {
    if has_parent {
        AttachPoint::ViewWindow(view_win)
    } else {
        AttachPoint::MainWindow
    }
}

/// End a dialog the §6.3 way: post the command with button ID 0, and fall
/// back to `EndModal(win, 0)` when no end notice arrives.
///
/// Returns the sequence a vector can check.
pub fn close_dialog(store: &mut WinStore, win: WinToken, got_end_notice: bool) -> Vec<&'static str> {
    let mut steps = vec!["post_command_0"];
    store.push(super::win::WinOp::GZOnCommand {
        win,
        command: super::message::MSG_COMMAND,
        data: close_dialogs::BUTTON_ID,
    });
    if !got_end_notice {
        steps.push("EndModal(win, 0)");
    }
    steps
}

/// The attach/detach notice handling of §6.5, expressed as the win proc's
/// answer.
pub fn handle_notice(msg: &GzMessage) -> Handled {
    if msg.kind.is_swallowed_by_advisor() {
        Handled::Yes
    } else {
        Handled::No
    }
}

/// Which instance an advisor popup is built from (§6.1 step 3).
pub fn advisor_popup_instance(mood: i32) -> u32 {
    if mood < ADVISOR_POPUP_MOOD_THRESHOLD {
        ADVISOR_POPUP_INSTANCE_LOW_MOOD
    } else {
        ADVISOR_POPUP_INSTANCE_HIGH_MOOD
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_windows::message::{GzMessage, MsgKind, MSG_COMMAND};

    const ACTIVE: u32 = advice_flag::ACTIVE;
    const EXPIRE: u32 = advice_flag::EXPIRE;

    #[test]
    fn esc_and_enter_do_nothing() {
        // Conformance test 20: "Esc and Enter must not close it."
        assert_eq!(popup_win_proc(ACTIVE, &GzMessage::key_down()), PopupAction::Ignored);
        assert_eq!(popup_win_proc(ACTIVE, &GzMessage::key_up()), PopupAction::Ignored);
        assert!(!popup_closes_on(ACTIVE, &GzMessage::key_down()));
        assert!(!popup_closes_on(ACTIVE, &GzMessage::key_up()));
    }

    #[test]
    fn any_button_closes_the_advisor_popup() {
        // Conformance test 20 / quirk 7: **any** button ID.
        for id in [0u32, 1, 0x8A8D_FD4E, 0xFFFF_FFFF] {
            assert!(popup_closes_on(ACTIVE, &GzMessage::command(id, 0)), "button {id:#x}");
        }
    }

    #[test]
    fn an_inactive_advice_ends_the_dialog() {
        assert_eq!(popup_win_proc(0, &GzMessage::command(0, 0)), PopupAction::End);
        assert!(popup_closes_on(0, &GzMessage::command(0, 0)));
    }

    #[test]
    fn expire_requested_ends_an_active_dialog() {
        assert_eq!(popup_win_proc(ACTIVE | EXPIRE, &GzMessage::command(0, 0)), PopupAction::End);
    }

    #[test]
    fn active_without_expire_stays_open_on_a_non_command() {
        let m = GzMessage { kind: MsgKind::ModalBegin, msg_type: 0x62BA_7B42, data1: 0, data2: 0, data3: 0 };
        assert_eq!(popup_win_proc(ACTIVE, &m), PopupAction::KeepOpen);
        assert!(!popup_closes_on(ACTIVE, &m));
    }

    #[test]
    fn link_follows_then_ends() {
        assert_eq!(popup_win_proc(ACTIVE, &GzMessage::link()), PopupAction::FollowLinkThenEnd);
    }

    #[test]
    fn attach_detach_notices_are_swallowed() {
        assert_eq!(popup_win_proc(ACTIVE, &GzMessage::attach_notice(true)), PopupAction::Swallow);
        assert_eq!(popup_win_proc(ACTIVE, &GzMessage::attach_notice(false)), PopupAction::Swallow);
    }

    #[test]
    fn the_hidden_pause_is_a_nesting_count() {
        // Conformance test 23 / §6.4: a camera move raises the same count with
        // no dialog open, so the two are not the same thing.
        let mut p = HiddenPause::new();
        assert!(!p.is_hidden_paused());
        assert!(!p.is_paused());

        p.enter(AutoGamePause::advisor_popup(), true);
        assert!(p.is_hidden_paused());
        assert!(!p.is_paused(), "IsPaused() stays false (§6.1)");
        assert_eq!(p.count(), 1);

        // A camera move while the dialog is open: the count nests, it does not
        // count popups.
        p.enter(AutoGamePause::camera_move(), false);
        assert_eq!(p.count(), 2);
        assert_eq!(p.dialog_pauses(), 1, "one dialog, two nested pauses");

        p.leave(AutoGamePause::camera_move(), false);
        assert_eq!(p.count(), 1, "back to the previous value");
        p.leave(AutoGamePause::advisor_popup(), true);
        assert!(!p.is_hidden_paused());
        assert_eq!(p.count(), 0);
    }

    #[test]
    fn a_camera_move_alone_raises_the_count() {
        let mut p = HiddenPause::new();
        p.enter(AutoGamePause::camera_move(), false);
        assert!(p.is_hidden_paused(), "same count as a dialog");
        assert_eq!(p.dialog_pauses(), 0, "but no dialog is open");
    }

    #[test]
    fn advisor_popup_instance_follows_the_mood_threshold() {
        assert_eq!(advisor_popup_instance(2), ADVISOR_POPUP_INSTANCE_LOW_MOOD);
        assert_eq!(advisor_popup_instance(3), ADVISOR_POPUP_INSTANCE_HIGH_MOOD);
    }

    #[test]
    fn button_zero_is_not_the_default() {
        // §6.3's correction.
        assert_ne!(close_dialogs::BUTTON_ID, close_dialogs::CONFIRM_YES);
        assert_eq!(close_dialogs::CONFIRM_MEANING, "no");
        assert_eq!(close_dialogs::SCRIPTED_MEANING, "cancel");
    }

    #[test]
    fn close_dialog_falls_back_to_end_modal() {
        let mut s = WinStore::new();
        let w = s.create(1);
        assert_eq!(close_dialog(&mut s, w, true), ["post_command_0"]);
        assert_eq!(
            close_dialog(&mut s, w, false),
            ["post_command_0", "EndModal(win, 0)"]
        );
    }

    #[test]
    fn a_parentless_dialog_attaches_to_the_main_window() {
        let view = 0x0C;
        assert_eq!(dialog_attach_point(false, view), AttachPoint::MainWindow);
        assert_eq!(dialog_attach_point(true, view), AttachPoint::ViewWindow(view));
    }

    #[test]
    fn handle_notice_swallows_only_the_notices() {
        assert_eq!(handle_notice(&GzMessage::attach_notice(true)), Handled::Yes);
        assert_eq!(handle_notice(&GzMessage::command(0, 0)), Handled::No);
    }

    #[test]
    fn advice_param_key_matches_the_spec() {
        assert_eq!(PARAM_ADVICE, 0xEA9F_13C0);
        assert_eq!(MSG_COMMAND, 0x2872_59F6);
    }
}
