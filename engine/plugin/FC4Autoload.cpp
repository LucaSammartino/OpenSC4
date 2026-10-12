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

#include "FC4Autoload.h"

#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"

#include "SC4String.h"
#include "SC4Vector.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cISC4App.h"
#include "cISC4Region.h"
#include "cISC4RegionalCity.h"
#include "cRZAutoRefCount.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"

#include <cctype>
#include <new>
#include <string>

namespace fc4 {
namespace {

enum class State { Unread, Off, Waiting, Done };

State g_state = State::Unread;
std::string g_path;
long g_delayTicks = 60;
long g_readyTicks = 0;  // consecutive ticks with a region
int g_emptyChecks = 0;  // region checks that listed no city
bool g_busy = false;    // LoadCity is running (it may tick the framework)

std::string Normal(std::string s) {
    for (char& c : s) {
        c = (c == '/') ? '\\' : static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    }
    return s;
}

// The app's cISC4App (one reference; the caller releases it), or nullptr.
cISC4App* App() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4))) return nullptr;
    return sc4;
}

// The region's city locations. [choice] The vector's storage is filled by
// the game and deliberately never freed here (a one-time leak of a few
// bytes), so this tool adds no cross-module deallocation to a bisection run.
using Locations = SC4Vector<cISC4Region::cLocation>;
Locations* NewLocations() {
    alignas(Locations) static unsigned char storage[sizeof(Locations)];
    return new (storage) Locations();  // never destroyed
}

// The region's entry for the city whose save is `want`, or nullptr. Logs the
// saves seen when there is no match.
cRZAutoRefCount<cISC4RegionalCity>* FindCity(cISC4Region* region, const Locations& locations,
                                             const std::string& want, int* cities) {
    std::string seen;
    *cities = 0;
    for (const cISC4Region::cLocation& at : locations) {
        cRZAutoRefCount<cISC4RegionalCity>* entry = region->GetCity(at.x, at.z);  // slot 11, region-owned
        cISC4RegionalCity* city = entry ? static_cast<cISC4RegionalCity*>(*entry) : nullptr;
        if (city == nullptr) continue;
        ++*cities;
        cRZBaseString path;
        AsGame(city)->GetCitySaveFilePath(path);  // slot 31
        const std::string p(path.ToChar() ? path.ToChar() : "", path.Strlen());
        if (Normal(p) == Normal(want)) return entry;
        if (seen.size() < 2048) seen += (seen.empty() ? "" : " | ") + p;
    }
    Log("autoload: the region's %d cities have these saves: %s", *cities, seen.empty() ? "-" : seen.c_str());
    return nullptr;
}

void Attempt(cISC4App* sc4, cISC4Region* region, const Locations& locations) {
    g_state = State::Done;  // one attempt, whatever happens
    int cities = 0;
    cRZAutoRefCount<cISC4RegionalCity>* entry = FindCity(region, locations, g_path, &cities);
    if (entry == nullptr) {
        // [choice] LoadCity is not called without the region's entry: what the
        // game does with a null second argument is not specified.
        Log("autoload: no city of the current region has the save %s; not loading (open that save's region once "
            "by hand so the game starts in it)",
            g_path.c_str());
        return;
    }
    Log("autoload: calling LoadCity(%s) with the region's city entry %p", g_path.c_str(), static_cast<void*>(entry));
    SC4String path(g_path.c_str(), static_cast<uint32_t>(g_path.size()));
    g_busy = true;
    const bool ok = sc4->LoadCity(path, entry);  // slot 13
    g_busy = false;
    Log("autoload: LoadCity(%s) -> %s", g_path.c_str(), ok ? "true" : "false");
}

}  // namespace

void AutoloadOnCityLoaded() {
    if (g_state == State::Waiting) {
        g_state = State::Done;
        Log("autoload: a city was loaded before the autoload; nothing to do");
    }
}

void AutoloadOnTick() {
    if (g_busy || g_state == State::Off || g_state == State::Done) return;
    if (g_state == State::Unread) {
        const char* source = "";
        g_path = Setting("autoload_city", "FC4_AUTOLOAD_CITY", &source);
        if (g_path.size() >= 2 && g_path.front() == '"' && g_path.back() == '"') g_path = g_path.substr(1, g_path.size() - 2);
        if (g_path.empty()) {
            g_state = State::Off;
            return;
        }
        g_delayTicks = SettingInt("autoload_delay_ticks", "FC4_AUTOLOAD_DELAY_TICKS", 60);
        if (g_delayTicks < 1) g_delayTicks = 1;
        g_state = State::Waiting;
        Log("autoload: %s (from %s); waiting for the region view, then %ld ticks", g_path.c_str(), source,
            g_delayTicks);
        return;
    }
    cISC4App* sc4 = App();
    if (sc4 == nullptr) return;
    cISC4Region* region = sc4->GetRegion();  // borrowed
    g_readyTicks = region != nullptr ? g_readyTicks + 1 : 0;
    if (g_readyTicks == 1) Log("autoload: the app has a region; loading in %ld ticks", g_delayTicks);
    if (region != nullptr && g_readyTicks >= g_delayTicks) {
        static Locations* locations = NewLocations();
        // Trivial elements: clear() only moves the end pointer.
        locations->clear();
        const bool listed = region->GetCityLocations(*locations);  // slot 23
        if (listed && !locations->empty()) {
            Attempt(sc4, region, *locations);
        } else if (++g_emptyChecks <= 10) {
            Log("autoload: the region lists no cities yet (%d); retrying in %ld ticks", g_emptyChecks, g_delayTicks);
            g_readyTicks = 0;
        } else {
            g_state = State::Done;
            Log("autoload: the region lists no cities; giving up");
        }
    }
    sc4->Release();
}

}  // namespace fc4
