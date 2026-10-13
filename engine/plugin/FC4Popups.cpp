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

#include "FC4Popups.h"

#include "FC4Checked.h"
#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"

#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cISC4App.h"
#include "cRZCOMDllDirector.h"

#include <cctype>
#include <climits>
#include <string>

namespace fc4 {
namespace {

enum class Want { Leave, Disable, Enable };  // unset or invalid / 0 / 1
// Idle: nothing to do. Pending: the next call is due (= 1: at PostAppInit or a
// city load; = 0: at ConnectionsReady). Off (= 0 only): our false is in force
// until the next city load. Done: = 1 made its call, or a call failed; nothing
// more is called in this process.
enum class State { Idle, Pending, Off, Done };

bool g_read = false;
Want g_want = Want::Leave;
State g_state = State::Idle;
cISC4App* g_testApp = nullptr;  // test seam (borrowed)

std::string Trimmed(const std::string& s) {
    size_t a = 0, b = s.size();
    while (a < b && std::isspace(static_cast<unsigned char>(s[a]))) ++a;
    while (b > a && std::isspace(static_cast<unsigned char>(s[b - 1]))) --b;
    return s.substr(a, b - a);
}

const char* Bool(bool v) { return v ? "true" : "false"; }

// The framework's application as cISC4App, with the one reference
// QueryInterface gave us (the caller releases it), or nullptr and `why`.
cISC4App* AcquireApp(const char** why) {
    cIGZFrameWork* fw = RZGetFrameWork();
    if (fw == nullptr) {
        *why = "no framework";
        return nullptr;
    }
    cIGZApp* app = fw->Application();  // borrowed from the framework: never released
    if (app == nullptr) {
        *why = "the framework has no application";
        return nullptr;
    }
    FC4_OUT(cISC4App*, sc4, "cISC4App* (QueryInterface out pointer)");
    const bool ok = app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(sc4.ptr()));
    sc4.Check();
    if (!ok || sc4.get() == nullptr) {
        *why = "QueryInterface(cISC4App 0x26CE01C0) failed";
        return nullptr;
    }
    FC4_ACQUIRED(sc4.get(), "QueryInterface");
    return sc4.get();
}

struct PopupCall {
    cISC4App* app;
    bool value;
    bool before;
    bool after;
};

// Runs under SehGuard.
void PopupStep(void* p) {
    auto* c = static_cast<PopupCall*>(p);
    c->before = c->app->GetPopupDialogsEnabled();  // slot 23 (+0x5C)
    c->app->SetPopupDialogsEnabled(c->value);      // slot 24 (+0x60), thiscall, one bool, pops 4
    c->after = c->app->GetPopupDialogsEnabled();
}

}  // namespace

void PopupDialogsReload() {
    g_read = true;
    const char* source = "";
    const std::string raw = Trimmed(Setting("popup_dialogs", "FC4_POPUP_DIALOGS", &source));
    g_want = Want::Leave;
    if (raw.empty()) {
        FC4_LOGV("popup dialogs: popup_dialogs not set; the game's popup dialogs are left alone");
    } else {
        // SettingInt reads the same value: 0/1 and the usual words; anything else is the sentinel.
        const long v = SettingInt("popup_dialogs", "FC4_POPUP_DIALOGS", LONG_MIN);
        if (v == 0 || v == 1) {
            g_want = v == 0 ? Want::Disable : Want::Enable;
            if (v == 0) {
                Log("popup dialogs: popup_dialogs=0 (%s): cISC4App::SetPopupDialogsEnabled(false) at each "
                    "ConnectionsReady 0x6AC284F3, true again at each later city load (with popups off the terrain "
                    "reconciliation never posts ConnectionsReady, foundations.md 11.6) (advisor popups are dropped "
                    "without pausing; generic dialogs answer without asking; city-situation popups still show)",
                    source);
            } else {
                Log("popup dialogs: popup_dialogs=1 (%s): cISC4App::SetPopupDialogsEnabled(true) at PostAppInit, or "
                    "at the first city load if the app is not reachable there",
                    source);
            }
        } else {
            Log("config: WARNING popup_dialogs='%s' (%s) is not 0/1; the game's popup dialogs are left alone",
                raw.c_str(), source);
        }
    }
    g_state = g_want == Want::Leave ? State::Idle : State::Pending;
}

namespace {

// One Get/Set/Get round on the app; true when the flag now reads `value`.
// When the app is not reachable the state is left alone (`retry` names the
// next chance); any other failure ends the switch (Done).
bool Call(bool value, const char* when, const char* retry) {
    const char* why = "";
    cISC4App* app = g_testApp;
    const bool counted = app == nullptr;  // our own QueryInterface reference (the test's app is borrowed)
    if (app == nullptr) app = AcquireApp(&why);
    if (app == nullptr) {
        Log("popup dialogs: SetPopupDialogsEnabled(%s) not called at %s: no cISC4App (%s); retried at the next %s",
            Bool(value), when, why, retry);
        return false;
    }
    PopupCall c{app, value, false, false};
    const bool clean = SehGuard("cISC4App::Get/SetPopupDialogsEnabled", &PopupStep, &c);
    if (counted) FC4_RELEASE(app);
    if (!clean) {
        Log("popup dialogs: WARNING structured exception in cISC4App::Get/SetPopupDialogsEnabled at %s; not retried",
            when);
    } else if (c.after != value) {
        Log("popup dialogs: WARNING SetPopupDialogsEnabled(%s) did not take effect (was enabled=%d, now=%d) at %s",
            Bool(value), c.before ? 1 : 0, c.after ? 1 : 0, when);
    } else {
        Log("popup dialogs: %s (cISC4App::SetPopupDialogsEnabled(%s); was enabled=%d, now=%d) at %s",
            value ? "enabled" : "disabled", Bool(value), c.before ? 1 : 0, c.after ? 1 : 0, when);
        return true;
    }
    g_state = State::Done;  // never retried after a failed call
    return false;
}

// = 1: the one call per process (PostAppInit, else the first city load that
// reaches the app).
void ApplyEnable(const char* when) {
    if (g_want != Want::Enable || g_state != State::Pending) return;
    if (Call(true, when, "city load")) g_state = State::Done;
}

}  // namespace

void PopupDialogsApply(const char* when) {
    if (!g_read) PopupDialogsReload();
    ApplyEnable(when);  // = 0 waits for ConnectionsReady
}

void PopupDialogsOnCityLoad() {
    if (!g_read) PopupDialogsReload();
    if (g_want == Want::Enable) {
        ApplyEnable("city load (PostCityInit)");  // only if PostAppInit could not make the call
    } else if (g_want == Want::Disable && g_state == State::Off) {
        // Our false from an earlier load would skip this load's terrain
        // reconciliation, and with it ConnectionsReady (foundations.md 11.6).
        if (Call(true, "city load (PostCityInit), until this load's ConnectionsReady", "city load")) {
            g_state = State::Pending;
        }
    }
}

void PopupDialogsOnConnectionsReady() {
    if (!g_read) PopupDialogsReload();
    if (g_want != Want::Disable || g_state != State::Pending) return;
    if (Call(false, "ConnectionsReady", "ConnectionsReady")) g_state = State::Off;
}

void PopupDialogsTestSetApp(cISC4App* app) { g_testApp = app; }

}  // namespace fc4
