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

#include "FC4GrowthProbe.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4GrowthProbeHook.h"
#include "FC4GrowthProbeLogic.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "compat/FC4Compat.h"
#include "fc4.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4App.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4City.h"
#include "cISC4Demand.h"
#include "cISC4GrowthDeveloper.h"
#include "cISC4LotManager.h"
#include "cISC4ResidentialSimulator.h"
#include "cISC4TractDeveloper.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kMsgSimNewDay = 0x66956814;        // posted by the simulator (simulator.md §5.2)
constexpr uint32_t kMsgPostCityInit = 0x26D31EC1;     // the city finished its Init
constexpr uint32_t kMsgPreCityShutdown = 0x26D31EC2;  // the city is about to shut down
const uint32_t kMessages[] = {kMsgSimNewDay, kMsgPostCityInit, kMsgPreCityShutdown};

struct ProbeState {
    bool running = false;     // the hook is installed (and, in the game, the messages subscribed)
    bool subscribed = false;  // AddNotification done (not through the test seam)
    bool test = false;        // started through the test seam
    const void* testBdsVtable = nullptr;
    uint64_t frequency = 0;                // QPC ticks per second
    GrowthHookTotals last;                 // the totals at the previous day line or city load
    std::vector<GrowthDayRecord> days;     // day lines not yet summarised
    uint64_t lines = 0;                    // day lines in the session
    bool readFaultLogged = false;
    bool mapRefusedLogged = false;
};

ProbeState g;

cISC4City* CurrentCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    cISC4City* city = sc4->GetCity();
    sc4->Release();
    return city;  // borrowed
}

// The x87 leftover guard at the forwarder's exit (verbose mode only): MSVC
// code and the game's Grow keep nothing on the x87 stack across a call, so
// the stack must be empty there. FXSAVE only; nothing is popped.
void ForwarderExitCheck() {
    static X87Site site = {"growth probe Grow exit", 0, 0, 0, nullptr};
    X87Check(site);
}

void FormatDate(int32_t julian, char* out, size_t cap) {
    if (julian <= 0) {
        _snprintf_s(out, cap, _TRUNCATE, "(no date)");
        return;
    }
    fc4_date d{};
    fc4_date_from_julian(julian, &d);
    _snprintf_s(out, cap, _TRUNCATE, "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
}

// ---- the city reads (under the structured-exception guard) ------------------------------------

struct ClassReads {
    cISC4City* city = nullptr;
    const void* tractDeveloper = nullptr;
    const void* tractVtable = nullptr;
    const void* bds = nullptr;
    const void* bdsVtable = nullptr;
};

void ReadClasses(void* p) {
    ClassReads* c = static_cast<ClassReads*>(p);
    cISC4TractDeveloper* td = c->city->GetTractDeveloper();  // city slot 80, borrowed
    c->tractDeveloper = td;
    if (td != nullptr) std::memcpy(&c->tractVtable, td, sizeof(c->tractVtable));
    cISC4BuildingDevelopmentSimulator* b = c->city->GetBuildingDevelopmentSimulator();  // city slot 53, borrowed
    c->bds = b;
    if (b != nullptr) std::memcpy(&c->bdsVtable, b, sizeof(c->bdsVtable));
}

struct DayReads {
    cISC4City* city = nullptr;
    bool haveBds = false;
    GameMapStatus mapStatus = GameMapStatus::Ok;
    uint32_t developers = 0;
    uint32_t noDemand = 0;
    int64_t qualifying = -1;
    int64_t lots = -1;
    int64_t population = -1;
};

// One developer's active demand: growth developer slot 18 GetSourceDemand
// (borrowed; growth-developer.md §10.1), then cISC4Demand slot 12
// QueryActiveDemandValue (ST0; building-development.md §4.3's ad(D)).
bool ActiveDemand(uint32_t value, float* active) {
    auto* dev = reinterpret_cast<cISC4GrowthDeveloper*>(static_cast<uintptr_t>(value));
    if (dev == nullptr) return false;
    cISC4Demand* d = dev->GetSourceDemand();
    if (d == nullptr) return false;
    *active = AsGame(d)->QueryActiveDemandValue();
    return true;
}

void ReadDay(void* p) {
    DayReads* r = static_cast<DayReads*>(p);
    cISC4City* c = r->city;
    cISC4BuildingDevelopmentSimulator* bds = c->GetBuildingDevelopmentSimulator();  // city slot 53, borrowed
    if (bds != nullptr) {
        r->haveBds = true;
        // Slot 34: the BDS's own developer map, the STLport map of
        // docs/abi/allocators.md (building-development.md §7.1, §9.1); read
        // in place, never written.
        const auto* map = reinterpret_cast<const GameMapObject*>(bds->GetDevelopers());
        const GrowthQualifyingCount q = CountQualifyingDevelopers(map, &ActiveDemand);
        r->mapStatus = q.status;
        r->developers = q.developers;
        r->noDemand = q.noDemand;
        if (q.status == GameMapStatus::Ok) r->qualifying = q.qualifying;
    }
    cISC4LotManager* lm = c->GetLotManager();  // city slot 33, borrowed
    if (lm != nullptr) r->lots = AsGame(lm)->GetLotCount();  // lot manager slot 16
    cISC4ResidentialSimulator* rs = c->GetResidentialSimulator();  // city slot 68, borrowed
    if (rs != nullptr) r->population = AsGame(rs)->GetPopulation();  // residential slot 16
}

// ---- the lines ----------------------------------------------------------------------------------

void Summary(const char* why) {
    Log("%s", FormatGrowthSummary(why, g.days, g.frequency).c_str());
    g.days.clear();
}

void OnCityLoad(cISC4City* city) {
    const GrowthHookTotals t = GrowthHookRead();
    const unsigned long long unattributed = t.calls - g.last.calls;
    g.last = t;
    GrowthHookTakeMaxTicks();
    if (!g.days.empty()) Summary("city load");  // a previous city's days without a shutdown line
    ClassReads c;
    c.city = city;
    if (city == nullptr) {
        Log("growth probe: city load: no city; %llu Grow call(s) since the previous boundary not attributed to a "
            "day", unattributed);
        return;
    }
    if (!SehGuard("growth probe city load", &ReadClasses, &c)) {
        Log("growth probe: city load: WARNING structured exception while reading the city's classes");
    }
    char td[192];
    const void* patched = GrowthHookVtable();
    if (c.tractDeveloper == nullptr) {
        _snprintf_s(td, sizeof(td), _TRUNCATE, "none (the city has no tract developer)");
    } else if (c.tractVtable == patched) {
        _snprintf_s(td, sizeof(td), _TRUNCATE, "%p is the original class (vtable %p, the patched one): its Grow calls "
                    "are counted", c.tractDeveloper, c.tractVtable);
    } else {
        _snprintf_s(td, sizeof(td), _TRUNCATE, "WARNING %p has vtable %p, not the patched %p: its Grow calls are NOT "
                    "counted (attempts stay 0)", c.tractDeveloper, c.tractVtable, patched);
    }
    char bds[192];
    const void* original = g.test ? g.testBdsVtable : compat::BuildingDevelopmentVtable();
    if (c.bds == nullptr) {
        _snprintf_s(bds, sizeof(bds), _TRUNCATE, "none (qualifying=-)");
    } else if (original == nullptr) {
        _snprintf_s(bds, sizeof(bds), _TRUNCATE, "%p (vtable %p; not checked: the original's vtable is not known)",
                    c.bds, c.bdsVtable);
    } else if (c.bdsVtable == original) {
        _snprintf_s(bds, sizeof(bds), _TRUNCATE, "%p is the original class (vtable %p)", c.bds, c.bdsVtable);
    } else {
        _snprintf_s(bds, sizeof(bds), _TRUNCATE, "WARNING %p is NOT the original class (vtable %p, the original's "
                    "%p): this run does not calibrate the original", c.bds, c.bdsVtable, original);
    }
    Log("growth probe: city load: tract developer %s; BDS %s; %llu Grow call(s) since the previous boundary not "
        "attributed to a day", td, bds, unattributed);
}

intptr_t OnDay(int32_t julian, cISC4City* city) {
    const GrowthHookTotals t = GrowthHookRead();
    char date[24];
    FormatDate(julian, date, sizeof(date));
    GrowthDayValues v;
    v.date = date;
    v.attempts = t.calls - g.last.calls;
    v.ticks = t.qpcTicks - g.last.qpcTicks;
    v.cycles = t.tscCycles - g.last.tscCycles;
    v.maxTicks = GrowthHookTakeMaxTicks();
    v.frequency = g.frequency;
    g.last = t;
    DayReads r;
    r.city = city;
    if (city != nullptr && !SehGuard("growth probe day line", &ReadDay, &r)) {
        r.qualifying = -1;
        r.lots = -1;
        r.population = -1;
        if (!g.readFaultLogged) {
            g.readFaultLogged = true;
            Log("growth probe %s: WARNING structured exception while reading the city; its values are '-' (logged "
                "once)", date);
        }
    }
    if (r.haveBds && r.mapStatus != GameMapStatus::Ok && !g.mapRefusedLogged) {
        g.mapRefusedLogged = true;
        Log("growth probe %s: WARNING the BDS developer map (slot 34) was refused: %s; qualifying=- while it is "
            "(logged once)", date, GameMapStatusName(r.mapStatus));
    }
    v.qualifying = r.qualifying;
    v.lots = r.lots;
    v.population = r.population;
    Log("%s", FormatGrowthDayLine(v).c_str());
    GrowthDayRecord rec;
    rec.attempts = static_cast<uint32_t>(v.attempts);
    rec.qualifying = static_cast<int32_t>(v.qualifying);
    rec.ticks = v.ticks;
    rec.cycles = v.cycles;
    g.days.push_back(rec);
    ++g.lines;
    return static_cast<intptr_t>(v.attempts);
}

intptr_t OnMessage(uint32_t type, intptr_t data1, cISC4City* city) {
    switch (type) {
        case kMsgSimNewDay: return OnDay(static_cast<int32_t>(data1), city);
        case kMsgPostCityInit: OnCityLoad(city); return 0;
        case kMsgPreCityShutdown:
            if (!g.days.empty()) Summary("city shutdown");
            return 0;
        default: return 0;
    }
}

// A static message target: counted, never deleted.
class Target final : public cIGZMessageTarget2 {
public:
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
        if (msg == nullptr || !g.running) return true;
        const uint32_t type = msg->GetType();
        intptr_t data1 = 0;
        cIGZMessage2Standard* m = nullptr;
        if (msg->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) && m != nullptr) {
            data1 = m->GetData1();
            m->Release();
        }
        OnMessage(type, data1, CurrentCity());
        return true;
    }

private:
    volatile LONG refs_ = 0;
};

Target& TheTarget() {
    static Target t;
    return t;
}

intptr_t StartWith(void** vtable, uintptr_t expected, bool test, const void* bdsVtable) {
    if (g.running) return 1;
    const char* source = "";
    const std::string raw = Setting("growth_probe", "FC4_GROWTH_PROBE", &source);
    const long mode = SettingInt("growth_probe", "FC4_GROWTH_PROBE", 0);
    if (mode == 0) return 0;
    if (mode != 1) {
        Log("growth probe: WARNING growth_probe='%s' (%s) is not 0/1; the probe stays off", raw.c_str(), source);
        return 0;
    }
    Log("growth probe: ON (growth_probe from %s): counts the tract developer's Grow calls (cISC4TractDeveloper slot "
        "5) per game day, for the growthAttemptsPerDay calibration (building-development.md §12); meant for the "
        "ORIGINAL building development simulator and tract developer",
        source);
    Log("growth probe: NOTE cut=1 (attempts < qualifying) is approximate: a pick that ends without a Grow call "
        "(invalid lists, an empty growth list) is not counted as an attempt, and qualifying is read when the posted "
        "SimNewDay arrives, not at the entry of the BDS's daily Grow");
    if (!test) {
        void** vt = nullptr;
        uintptr_t grow = 0;
        if (!compat::TractDeveloperGrow(&vt, &grow)) {
            Log("growth probe: NOT installed: the original tract developer vtable is not available (see the compat "
                "lines above); nothing patched");
            return -1;
        }
        vtable = vt;
        expected = grow;
    }
    GrowthHookReport r;
    const GrowthHookStatus s = InstallGrowthHook(vtable, expected, &r);
    if (s != GrowthHookStatus::Installed) {
        Log("growth probe: NOT installed: %s (vtable %p, entry 0x%08lX holds 0x%08lX, expected 0x%08lX, error %lu); "
            "nothing patched",
            GrowthHookStatusName(s), static_cast<void*>(vtable), static_cast<unsigned long>(r.entryAddress),
            static_cast<unsigned long>(r.found), static_cast<unsigned long>(expected), r.error);
        return -1;
    }
    LARGE_INTEGER f{};
    QueryPerformanceFrequency(&f);
    g.frequency = static_cast<uint64_t>(f.QuadPart);
    const bool verbose = LogLevel() >= kLogVerbose;
    Log("growth probe: hook installed at 0x%08lX (vtable %p slot 5): 0x%08lX -> 0x%08lX (the FreeCity4.dll "
        "forwarder); page protection 0x%lX %s; QPC frequency %llu Hz; x87 check at the forwarder's exit: %s",
        static_cast<unsigned long>(r.entryAddress), static_cast<void*>(vtable), static_cast<unsigned long>(r.found),
        static_cast<unsigned long>(r.written), r.protectBefore, r.protectRestored ? "restored" : "NOT restored",
        static_cast<unsigned long long>(g.frequency), verbose ? "on (verbose)" : "off (not verbose)");
    if (!test) {
        cIGZMessageServer2Ptr ms;
        if (!ms) {
            const GrowthHookStatus back = RemoveGrowthHook(&r);
            Log("growth probe: no message server; the hook is removed again (%s)", GrowthHookStatusName(back));
            return -1;
        }
        TheTarget().AddRef();  // our own reference (a static object)
        bool ok = true;
        for (const uint32_t type : kMessages) {
            ok = ms->AddNotification(static_cast<cIGZMessageTarget2*>(&TheTarget()), type) && ok;
        }
        g.subscribed = true;
        if (!ok) Log("growth probe: WARNING an AddNotification returned false");
    }
    if (verbose) SetGrowthHookExitCheck(&ForwarderExitCheck);
    g.running = true;
    g.test = test;
    g.testBdsVtable = bdsVtable;
    g.last = GrowthHookRead();
    GrowthHookTakeMaxTicks();
    return 1;
}

intptr_t Stop() {
    if (!g.running) return 0;
    if (!g.days.empty()) Summary("app shutdown");
    if (g.subscribed) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            for (const uint32_t type : kMessages) {
                ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(&TheTarget()), type);
            }
        }
        TheTarget().Release();
        g.subscribed = false;
    }
    g.running = false;
    SetGrowthHookExitCheck(nullptr);
    GrowthHookReport r;
    const GrowthHookStatus s = RemoveGrowthHook(&r);
    const GrowthHookTotals t = GrowthHookRead();
    if (s == GrowthHookStatus::Removed) {
        Log("growth probe: hook removed: entry 0x%08lX holds the original 0x%08lX again (page protection 0x%lX %s); "
            "%llu Grow call(s) counted in all, %llu day line(s)",
            static_cast<unsigned long>(r.entryAddress), static_cast<unsigned long>(r.written), r.protectBefore,
            r.protectRestored ? "restored" : "NOT restored", static_cast<unsigned long long>(t.calls),
            static_cast<unsigned long long>(g.lines));
        return 1;
    }
    Log("growth probe: WARNING hook NOT removed: %s (entry 0x%08lX holds 0x%08lX, error %lu)", GrowthHookStatusName(s),
        static_cast<unsigned long>(r.entryAddress), static_cast<unsigned long>(r.found), r.error);
    return 0;
}

}  // namespace

void GrowthProbeStart() { StartWith(nullptr, 0, false, nullptr); }

void GrowthProbeStop() { Stop(); }

void GrowthProbeDetach() {
    if (!GrowthHookInstalled()) return;
    g.running = false;
    SetGrowthHookExitCheck(nullptr);
    GrowthHookReport r;
    const GrowthHookStatus s = RemoveGrowthHook(&r);
    Log("growth probe: DLL detach: %s (entry 0x%08lX)",
        s == GrowthHookStatus::Removed ? "hook removed, the original restored" : GrowthHookStatusName(s),
        static_cast<unsigned long>(r.entryAddress));
}

}  // namespace fc4

intptr_t GrowthProbeTest(const FC4GrowthProbeTestInput* in) {
    if (in == nullptr) return -1;
    switch (in->action) {
        case kFC4GrowthProbeTestStart:
            return fc4::StartWith(in->vtable, in->expected, true, in->bdsVtable);
        case kFC4GrowthProbeTestMessage:
            if (!fc4::g.running) return -1;
            return fc4::OnMessage(in->msgType, in->data1, in->city);
        case kFC4GrowthProbeTestStop:
            return fc4::Stop();
        default:
            return -1;
    }
}
