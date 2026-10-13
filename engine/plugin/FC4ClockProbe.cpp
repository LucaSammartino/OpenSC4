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

#include "FC4ClockProbe.h"

#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Subsystems.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

namespace fc4 {
namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kMsgPostCityInit = 0x26D31EC1;
constexpr uint32_t kMsgLevelBase = 0x66956810;  // + level (simulator.md §2.2)
constexpr uint32_t kFirstLevel = 4;             // day
constexpr uint32_t kLastLevel = 7;              // year
constexpr uint32_t kWeekLevel = 5;
constexpr uint32_t kAgentFlags = 3;  // enabled for unestablished and established cities
constexpr char kAgentName[] = "FreeCity4 clock probe";

const char* const kLevelNames[] = {"day", "week", "month", "year"};  // levels 4..7

// The messages the simulator posts (§5.2, §6) and what the line calls them.
struct Posted {
    uint32_t type;
    const char* name;
    bool calendar;  // day/week/month/year: data1 = Julian day, data3 = the counter
};
const Posted kPosted[] = {
    {0x66956814, "day", true},       {0x66956815, "week", true},      {0x66956816, "month", true},
    {0x66956817, "year", true},      {0xAA7FB7E0, "pause", false},    {0x4A7FB7E2, "hidden", false},
    {0x4A7FB807, "emergency", false}, {0x4A7FB7E9, "speed", false},
};

bool Fields(cIGZMessage2* msg, intptr_t* d1, intptr_t* d3) {
    cIGZMessage2Standard* m = nullptr;
    if (msg == nullptr || !msg->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) ||
        m == nullptr) {
        return false;
    }
    *d1 = m->GetData1();
    *d3 = m->GetData3();
    m->Release();
    return true;
}

cISC4City* CurrentCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    cISC4City* city = sc4->GetCity();
    sc4->Release();
    return city;
}

// A static message target: counted, never deleted. One instance is the agent,
// the other hears the posted messages; the day message type is the same for
// both (0x66956814), so the instance tells them apart.
class Probe final : public cIGZMessageTarget2 {
public:
    explicit Probe(bool agent) : agent_(agent) {}

    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid != kIID_cIGZMessageTarget2 && riid != GZIID_cIGZUnknown) {
            *ppvObj = nullptr;
            return false;
        }
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return static_cast<uint32_t>(InterlockedIncrement(&refs_)); }
    uint32_t Release() override {
        const LONG r = InterlockedDecrement(&refs_);
        return r > 0 ? static_cast<uint32_t>(r) : 0u;
    }

    bool DoMessage(cIGZMessage2* msg) override {
        if (msg == nullptr) return true;
        return agent_ ? AgentMessage(msg) : PostedMessage(msg);
    }

    int mode = 0;

private:
    bool AgentMessage(cIGZMessage2* msg) {
        const uint32_t type = msg->GetType();
        if (type < kMsgLevelBase + kFirstLevel || type > kMsgLevelBase + kLastLevel) return true;
        const uint32_t level = type - kMsgLevelBase;
        intptr_t d1 = 0, d3 = 0;
        if (!Fields(msg, &d1, &d3)) {
            Log("CLOCK agent %s (no message fields)", kLevelNames[level - kFirstLevel]);
            return true;
        }
        // Mode 2: the first delivery of each week is "not finished" (§5.3).
        const bool hold = mode == 2 && level == kWeekLevel && !(heldWeek_ && heldJulian_ == d1);
        const bool again = mode == 2 && level == kWeekLevel && !hold;
        Log("CLOCK agent %s j=%ld c=%ld%s", kLevelNames[level - kFirstLevel], static_cast<long>(d1),
            static_cast<long>(d3), again ? " again" : "");
        if (level == kWeekLevel && mode == 2) {
            heldWeek_ = hold;
            heldJulian_ = d1;
        }
        return !hold;
    }

    bool PostedMessage(cIGZMessage2* msg) {
        const uint32_t type = msg->GetType();
        if (type == kMsgPostCityInit) {
            RegisterAgent();
            return true;
        }
        for (const Posted& p : kPosted) {
            if (p.type != type) continue;
            intptr_t d1 = 0, d3 = 0;
            if (!Fields(msg, &d1, &d3)) {
                Log("CLOCK post %s (no message fields)", p.name);
            } else if (p.calendar) {
                Log("CLOCK post %s j=%ld c=%ld", p.name, static_cast<long>(d1), static_cast<long>(d3));
            } else {
                Log("CLOCK post %s d1=%ld", p.name, static_cast<long>(d1));
            }
            break;
        }
        return true;
    }

    void RegisterAgent();

    bool agent_;
    bool heldWeek_ = false;
    intptr_t heldJulian_ = 0;
    volatile LONG refs_ = 0;
};

Probe& AgentProbe() {
    static Probe p(true);
    return p;
}

Probe& PostProbe() {
    static Probe p(false);
    return p;
}

void Probe::RegisterAgent() {
    cISC4City* city = CurrentCity();
    cFC4ISC4Simulator* sim = city ? AsGame(city->GetSimulator()) : nullptr;
    if (sim == nullptr) {
        Log("CLOCK load: no city simulator; probe agent not registered");
        return;
    }
    // A target appears at most once per level (§4), so a second load of the
    // same simulator adds nothing; a new simulator gets its own registration.
    const cRZBaseString name(kAgentName);
    bool ok = true;
    for (uint32_t level = kFirstLevel; level <= kLastLevel; ++level) {
        ok = sim->AddAgent(static_cast<cIGZMessageTarget2*>(&AgentProbe()), level, name, kAgentFlags) && ok;
    }
    Log("CLOCK load: probe agent registered at levels %u-%u (%s simulator, mode %d)%s", kFirstLevel, kLastLevel,
        SimulatorReplaced() ? "FreeCity4" : "original", AgentProbe().mode, ok ? "" : " [an AddAgent returned false]");
}

bool g_started = false;

}  // namespace

void ClockProbeStart() {
    if (g_started) return;
    const long mode = SettingInt("clock_probe", "FC4_CLOCK_PROBE", 0);
    if (mode != 1 && mode != 2) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) {
        Log("CLOCK probe: no message server; not started");
        return;
    }
    AgentProbe().mode = static_cast<int>(mode);
    PostProbe().mode = static_cast<int>(mode);
    PostProbe().AddRef();  // our own references (static objects)
    AgentProbe().AddRef();
    bool notes = ms->AddNotification(static_cast<cIGZMessageTarget2*>(&PostProbe()), kMsgPostCityInit);
    for (const Posted& p : kPosted) {
        notes = ms->AddNotification(static_cast<cIGZMessageTarget2*>(&PostProbe()), p.type) && notes;
    }
    g_started = true;
    Log("CLOCK probe: started (mode %ld, notifications %d)", mode, notes ? 1 : 0);
}

void ClockProbeStop() {
    if (!g_started) return;
    g_started = false;
    cIGZMessageServer2Ptr ms;
    if (ms) {
        ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(&PostProbe()), kMsgPostCityInit);
        for (const Posted& p : kPosted) ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(&PostProbe()), p.type);
    }
    PostProbe().Release();
    AgentProbe().Release();
    Log("CLOCK probe: stopped");
}

}  // namespace fc4
