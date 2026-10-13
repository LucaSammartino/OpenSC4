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

// The popup-dialogs switch (tooling for unattended runs; no simulation
// behaviour). advisors.md §6.8.1, "Ways for a plugin to suppress or dismiss
// advisor popups", option 1, and its [choice].
//
// FreeCity4.ini `popup_dialogs = 0` (env FC4_POPUP_DIALOGS; the ini wins)
// makes FreeCity4 call cISC4App::SetPopupDialogsEnabled(false) once per city
// load, when ConnectionsReady arrives. With the app flag false (§6.8.1):
//   - every queued advisor popup fails gate a and is dropped, with no hidden
//     pause of the simulator;
//   - GenericNotificationDialog shows nothing, GenericConfirmationDialog
//     returns false and DoScriptedModalDialog returns 0 without asking (the
//     quit, go-to-region, obliterate and budget-failure prompts use the last
//     one; what those callers then do is not in the spec);
//   - city-situation popups are NOT affected (they still pause);
//   - the terrain edge reconciliation is skipped, and with it ConnectionsReady
//     0x6AC284F3 (foundations.md §11.6): power, plumbing and the neighbours
//     simulator would stay disabled for the session (runs 58-64).
// `popup_dialogs = 1` calls it with true. Not set (or empty): the game is left
// alone and nothing is called. Any other value: a config warning, and the game
// is left alone. Values are read like the other 0/1 keys (SettingInt: on/off,
// true/false, yes/no are accepted too).
//
// When: the key is read at game start (class enumeration).
//   - `= 0` (foundations.md §11.6 [choice], advisors.md §6.8.1): the flag is
//     set false only when ConnectionsReady arrives (the observer subscribes to
//     0x6AC284F3; the regional city posts it once per load, a few frames after
//     PostCityInit). At each later city load (the observer's PostCityInit) the
//     flag is set true again, so that load's reconciliation runs and posts
//     ConnectionsReady, then false again when it arrives. Between PostCityInit
//     and ConnectionsReady no simulation day runs [inferred], so no advisor
//     popup is due. If the app is not reachable at ConnectionsReady, the call
//     is retried at the next one.
//   - `= 1`: the call is made at PostAppInit: the app object exists from its
//     constructor on. If the app is not reachable there, it is retried at
//     every city load until it has been made once; never twice.
// A call that raises a structured exception or does not take effect is logged
// and never retried. Nothing else in the game writes the flag (§6.8.1).
//
// ABI: cISC4App (IID 0x26CE01C0; docs/abi/vtables.tsv cSC4App::vftable_3,
// checked by abi_conformance_test through the SDK declaration):
//   slot 23 (+0x5C) bool GetPopupDialogsEnabled()        pops 0
//   slot 24 (+0x60) void SetPopupDialogsEnabled(bool)    thiscall, pops 4
// The app is reached as the other tooling reaches it: the framework's
// application, QueryInterface for cISC4App (one reference, released after the
// call through the reference audit). The framework's cIGZApp pointer is
// borrowed and never released.
//
// Log lines (not verbose):
//   popup dialogs: popup_dialogs=0 (FreeCity4.ini): cISC4App::SetPopupDialogsEnabled(false) at each
//     ConnectionsReady ...
//   popup dialogs: disabled (cISC4App::SetPopupDialogsEnabled(false); was enabled=1, now=0) at ConnectionsReady
//   popup dialogs: enabled (cISC4App::SetPopupDialogsEnabled(true); was enabled=0, now=1) at city load
//     (PostCityInit), until this load's ConnectionsReady
//   popup dialogs: SetPopupDialogsEnabled(<v>) not called at <when>: no cISC4App (<why>); retried at the next
//     ConnectionsReady | city load
//   popup dialogs: WARNING SetPopupDialogsEnabled(false) did not take effect (was enabled=1, now=1) at <when>
//   config: WARNING popup_dialogs='<value>' (<source>) is not 0/1; the game's popup dialogs are left alone
#pragma once

class cISC4App;

namespace fc4 {

// Reads `popup_dialogs` / FC4_POPUP_DIALOGS and logs the result (game start).
void PopupDialogsReload();

// PostAppInit: `= 1` makes its call here (or at the first city load that
// reaches the app). `= 0` waits for ConnectionsReady. `when` names the point
// for the log.
void PopupDialogsApply(const char* when);

// The observer's PostCityInit: `= 1` retries a call PostAppInit could not
// make; `= 0` sets the flag true again if we set it false for an earlier load.
void PopupDialogsOnCityLoad();

// The observer's ConnectionsReady (0x6AC284F3): `= 0` sets the flag false.
void PopupDialogsOnConnectionsReady();

// Test seam (FC4TestSeam.h op 5, shared with the observer's save autotest): an
// app to call instead of the framework's (borrowed: never released; null: back
// to the framework's).
void PopupDialogsTestSetApp(cISC4App* app);

}  // namespace fc4
