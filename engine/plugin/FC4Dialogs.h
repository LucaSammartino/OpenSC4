// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

// The dialog closer (tooling for unattended runs; the harness lane; no
// simulation behaviour): docs/specs/modal-dialogs.md §3.2 [choice].
//
// FreeCity4.ini `close_dialogs = 1` (env FC4_CLOSE_DIALOGS; the ini wins)
// ends every game modal dialog through the game's window manager, from inside
// the game, instead of a screenshot and a click from outside:
//   - Track. At PostAppInit (before any city load: the two load-time dialogs
//     of §4 come before ConnectionsReady) a message target of ours subscribes
//     to the legacy message server's (service 0x64063387) modal begin / end
//     notices 0x62BA7B42 / 0x62BA7B43 (§2.5). A begin notice records the
//     window (data2), its ID (data1), its depth (data3, the nest count before
//     it) and the frame; an end notice drops the record of that window (keyed
//     by the window, not the ID: §7 question 1). Nothing is called inside a
//     notice.
//   - Close. On a later frame (the observer's OnTick, which the modal loop
//     keeps ticking, §2.3), for the innermost record only, once the grace of
//     kDialogsGraceFrames has passed:
//       1. checks, in this order: IsWindowValid(window) (manager slot 24;
//          safe on a stale pointer, so always first), the window is not
//          SC4GameDelay's (below), GetModalNestCount() (slot 43) == depth + 1,
//          and the window's GetFlag(0x1000) (cIGZWin slot 67);
//       2. presses button 0: PostMessageToTarget(window, 3, 0x287259F6, 0, 0)
//          (manager slot 14), what a click does;
//       3. if no end notice came kDialogsFallbackFrames frames later, the same
//          checks again, then EndModal(window, 0) (manager slot 54).
//     A check that fails stops the step: no call follows it. A window that is
//     no longer valid drops its record (a WARNING); one that is valid but not
//     the current modal window is checked again at the next frame (a WARNING
//     once after kDialogsFallbackFrames frames). If no end notice comes
//     kDialogsFallbackFrames frames after EndModal: a WARNING, and nothing
//     more is called on that window (its validity is still checked, so a
//     window that is gone without an end notice drops its record).
//   - Exclusions (§3.3, §3.4): only button 0 is ever sent (never 1, 6 or 7,
//     which do more than close the city-situation popup). SC4GameDelay's
//     invisible modal window (cSC4WinGameDelay, recognised by its primary
//     vtable from the compat table, docs/abi/vtables.tsv) is never touched:
//     it ends itself. Only the innermost dialog is ever acted on, and EndModal
//     only after the checks say the window is the current modal one (the
//     deferred-end map hazard of §3.4). When an inner dialog ends, the outer
//     one's pending fallback starts counting again, so an end the outer
//     dialog's handler deferred (§2.4) is not doubled by an EndModal.
//     Win32 message boxes are out of reach (§3.4: no frame runs).
//   - If the subscription could not be made at PostAppInit it is retried at
//     each frame; if a modal loop is already running when it succeeds, the
//     focused window and its parents are walked (GZGetFocus, manager slot 36;
//     GetParentWin, cIGZWin slot 11) to the first one whose GetFlag(0x1000)
//     is true, which is recorded at depth GetModalNestCount() - 1 (§3.2, "If
//     no begin notice was seen").
// Dialogs the harness asks for itself (RequestQuit(true), RequestCloseCity(
// true), RequestGoToRegionView(true)) are answered with button 0, which is
// "no" / cancel (§3.3, §4). The autotest's RequestQuit(false, false) opens no
// dialog.
//
// Log (always; the begin notice and the press also in the verbose log):
//   dialogs: closed 0x<id> (window 0x<ptr>, depth N) by button after K frame(s)
//   dialogs: closed 0x<id> (window 0x<ptr>, depth N) by EndModal fallback after K frame(s)
//   dialogs: 0x<id> (window 0x<ptr>, depth N) ended by itself after K frame(s)
//   dialogs: left 0x<id> (window 0x<ptr>, depth N) alone: SC4GameDelay's invisible modal ends itself
//   dialogs: WARNING could not close 0x<id> (window 0x<ptr>, depth N): <reason>
// K counts frames from the call that closed the dialog (the press, or
// EndModal) to its end notice; for "ended by itself", from its begin notice.
// The autotest verdict gains `, dialogs_closed=N` after `ref_warnings=`
// (close_dialogs = 1 only: the verdict line is unchanged otherwise).
//
// Off (`close_dialogs = 0`, unset, or not 0/1: a config warning): nothing is
// subscribed, nothing is called and nothing is logged; the verdict line is
// unchanged.
//
// ABI: FC4WinInterfaces.h (cIGZWinMgr, cIGZWin, the legacy cIGZMessageServer;
// docs/abi/interfaces.tsv, checked by abi_conformance_test); our target is the
// SDK's cIGZMessageTarget (IID 0xE98B2F57). References: the manager and the
// server are asked for (framework slot 5 GetSystemService) only when needed and
// released right after (the reference audit sees both); the window pointers of
// the notices are borrowed, never AddRef'd or released; the server AddRefs our
// target (a static object of ours) and releases it at RemoveNotification
// (PreAppShutdown).
#pragma once

#include <cstdint>

namespace fc4 {

// Frames (closer ticks) before the button press, from the begin notice
// ("a few frames, so the dialog has painted", §3.2) [choice].
constexpr uint32_t kDialogsGraceFrames = 5;
// Frames after the press before the EndModal fallback, and after EndModal
// before giving up ("some tens of frames", §3.2) [choice].
constexpr uint32_t kDialogsFallbackFrames = 30;

// Game start (FC4Subsystems.cpp): reads `close_dialogs`.
void DialogsReload();
// PostAppInit (cFC4Director.cpp): subscribes to the begin / end notices.
void DialogsStart();
// PreAppShutdown: removes the subscription.
void DialogsStop();
// The observer's OnTick (main thread; also inside a modal loop).
void DialogsOnTick();
// True when close_dialogs = 1 (any thread).
bool DialogsEnabled();
// Dialogs ended by our button press or EndModal so far (any thread).
long DialogsClosedCount();

}  // namespace fc4

// kFC4SeamDialogs (op 47)'s input: the closer's test hooks.
enum : uint32_t {
    kFC4DialogsTestServices = 1,  // use `winMgr` / `msgServer` / `gameDelayVtable` instead of the game's
    kFC4DialogsTestStart = 2,     // DialogsStart (as PostAppInit); returns 1 if subscribed
    kFC4DialogsTestStop = 3,      // DialogsStop (as PreAppShutdown); returns 1 if it was subscribed
    kFC4DialogsTestClosed = 4,    // returns DialogsClosedCount()
    kFC4DialogsTestOpen = 5,      // returns the number of recorded dialogs
};

struct FC4DialogsTestInput {
    uint32_t action;
    void* winMgr;                 // a cFC4IGZWinMgr* (borrowed; null: the framework's service)
    void* msgServer;              // a cFC4IGZMessageServer* (borrowed; null: the framework's service)
    const void* gameDelayVtable;  // the vtable that counts as cSC4WinGameDelay's (null: the compat table's)
};

// The seam's handler: -1 for a null input or an unknown action.
intptr_t DialogsTestSeam(const FC4DialogsTestInput* in);
