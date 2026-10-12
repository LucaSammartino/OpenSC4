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

#include "FC4Observer.h"

#include "FC4Autotest.h"
#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Subsystems.h"
#include "FC4Watchdog.h"
#include "cFC4Message2Standard.h"
#include "fc4.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSystemService.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4Demand.h"
#include "cISC4DemandSimulator.h"
#include "cISC4ResidentialSimulator.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdarg>
#include <cstdio>
#include <cstring>

namespace fc4 {
namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kMsgPostCityInit = 0x26D31EC1;
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kObserverServiceID = 0x5C7A43E2;  // random, FreeCity4's
constexpr int32_t kObserverPriority = -1000000;      // [choice] tooling; any value works

// Demand IDs (public SC4 modding documentation: the occupant-group / demand
// IDs of the RCI groups), demand index 0 = this city. [choice]
struct DemandId {
    uint32_t id;
    const char* name;
    char group;  // 'R', 'C', 'I'
};
const DemandId kDemands[] = {
    {0x1010, "R$", 'R'},    {0x1020, "R$$", 'R'},   {0x1030, "R$$$", 'R'},  {0x3110, "Cs$", 'C'},
    {0x3120, "Cs$$", 'C'},  {0x3130, "Cs$$$", 'C'}, {0x3320, "Co$$", 'C'},  {0x3330, "Co$$$", 'C'},
    {0x4100, "I-R", 'I'},   {0x4200, "I-D", 'I'},   {0x4300, "I-M", 'I'},   {0x4400, "I-HT", 'I'},
};
constexpr size_t kDemandCount = sizeof(kDemands) / sizeof(kDemands[0]);

cISC4City* g_testCity = nullptr;  // test seam

cISC4City* CurrentCity() {
    if (g_testCity != nullptr) return g_testCity;
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

int64_t DataOf(cIGZMessage2* msg, bool* ok) {
    *ok = false;
    cIGZMessage2Standard* m = nullptr;
    if (!msg->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) || m == nullptr) return 0;
    const int64_t d1 = static_cast<int64_t>(m->GetData1());
    m->Release();
    *ok = true;
    return d1;
}

// ---- the digest ------------------------------------------------------------------------------------

struct DigestCall {
    int32_t julian;
    char line[1024];
};

void AppendF(char* out, size_t cap, const char* fmt, ...) {
    const size_t len = strlen(out);
    if (len + 1 >= cap) return;
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(out + len, cap - len, _TRUNCATE, fmt, ap);
    va_end(ap);
}

// Reads the digest through public interfaces only. Runs under SehGuard.
void DigestStep(void* p) {
    auto* c = static_cast<DigestCall*>(p);
    char* out = c->line;
    const size_t cap = sizeof(c->line);
    fc4_date d{};
    fc4_date_from_julian(c->julian, &d);
    _snprintf_s(out, cap, _TRUNCATE, "DIGEST julian=%d (%04d-%02d-%02d)", c->julian, d.year, d.month, d.day_of_month);
    cISC4City* city = CurrentCity();
    if (city == nullptr) {
        AppendF(out, cap, " no city");
        return;
    }
    cFC4ISC4BudgetSimulator* budget = AsGame(city->GetBudgetSimulator());
    if (budget != nullptr) {
        AppendF(out, cap, " funds=%lld", static_cast<long long>(budget->GetTotalFunds()));
    } else {
        AppendF(out, cap, " funds=-");
    }

    cISC4ResidentialSimulator* res = city->GetResidentialSimulator();
    cISC4DemandSimulator* demandSim = city->GetDemandSimulator();
    float value[kDemandCount] = {};
    bool have[kDemandCount] = {};
    for (size_t i = 0; i < kDemandCount && demandSim != nullptr; ++i) {
        cISC4Demand* dm = demandSim->GetDemand(kDemands[i].id, 0);  // [choice] borrowed pointer (no AddRef)
        if (dm == nullptr) continue;
        have[i] = true;
        value[i] = dm->QueryDemandValue();
    }
    // R = the residential simulator's population; C and I = the supply of
    // the census sum nodes 0x3001 / 0x4001 (demand §10, "population versus
    // jobs"), as the original's advisors read them.
    if (res != nullptr) {
        AppendF(out, cap, " population R/C/I=%ld", static_cast<long>(res->GetPopulation()));
    } else {
        AppendF(out, cap, " population R/C/I=-");
    }
    for (uint32_t census : {0x3001u, 0x4001u}) {
        cISC4Demand* dm = demandSim ? demandSim->GetDemand(census, 0) : nullptr;  // borrowed
        if (dm != nullptr) {
            AppendF(out, cap, "/%.0f", static_cast<double>(dm->QuerySupplyValue()));
        } else {
            AppendF(out, cap, "/-");
        }
    }
    AppendF(out, cap, " demand ");
    for (size_t i = 0; i < kDemandCount; ++i) AppendF(out, cap, "%s%s", i ? "/" : "", kDemands[i].name);
    AppendF(out, cap, "=");
    for (size_t i = 0; i < kDemandCount; ++i) {
        if (have[i]) {
            AppendF(out, cap, "%s%.1f", i ? "/" : "", static_cast<double>(value[i]));
        } else {
            AppendF(out, cap, "%s-", i ? "/" : "");
        }
    }
    if (budget != nullptr) {
        AppendF(out, cap, " budget income/expense=%lld/%lld", static_cast<long long>(budget->GetTotalMonthlyIncome()),
                static_cast<long long>(budget->GetTotalMonthlyExpense()));
    } else {
        AppendF(out, cap, " budget income/expense=-/-");
    }
}

void WriteDigest(int32_t julian) {
    DigestCall c{};
    c.julian = julian;
    const bool clean = SehGuard("monthly digest", &DigestStep, &c);
    Log("%s%s", c.line, clean ? "" : " [INCOMPLETE: structured exception while reading]");
}

// ---- the observer object ----------------------------------------------------------------------------

class cFC4Observer final : public cIGZSystemService, public cIGZMessageTarget2 {
public:
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        switch (riid) {
            case GZIID_cIGZSystemService: *ppvObj = static_cast<cIGZSystemService*>(this); break;
            case kIID_cIGZMessageTarget2: *ppvObj = static_cast<cIGZMessageTarget2*>(this); break;
            case GZIID_cIGZUnknown: *ppvObj = static_cast<cIGZUnknown*>(static_cast<cIGZSystemService*>(this)); break;
            default: *ppvObj = nullptr; return false;
        }
        AddRef();
        return true;
    }
    // A static object: counted, never deleted.
    uint32_t AddRef() override { return static_cast<uint32_t>(InterlockedIncrement(&refs_)); }
    uint32_t Release() override {
        const LONG r = InterlockedDecrement(&refs_);
        return r > 0 ? static_cast<uint32_t>(r) : 0u;
    }

    // cIGZSystemService
    uint32_t GetServiceID() override { return serviceId_; }
    cIGZSystemService* SetServiceID(uint32_t id) override {
        serviceId_ = id;
        return this;
    }
    int32_t GetServicePriority() override { return kObserverPriority; }
    bool IsServiceRunning() override { return running_; }
    cIGZSystemService* SetServiceRunning(bool running) override {
        running_ = running;
        return this;
    }
    bool Init() override { return true; }
    bool Shutdown() override { return true; }
    bool OnTick(uint32_t) override {
        WatchdogStart();  // once; this is the main thread
        NoteFrameworkTick();
        const int speed = AutotestOnTick();
        if (speed != 0) Unpause(speed);
        return true;
    }
    bool OnIdle(uint32_t) override { return true; }
    int32_t GetServiceTickPriority() override { return kObserverPriority; }

    // cIGZMessageTarget2
    bool DoMessage(cIGZMessage2* msg) override {
        if (msg == nullptr) return true;
        switch (msg->GetType()) {
            case kMsgPostCityInit:
                Log("observer: PostCityInit (%s)", SimulatorReplaced() ? "replacement" : "passthrough");
                if (!SimulatorReplaced()) SetCityRunning(true);  // ours does it itself
                AutotestOnCityLoaded();
                break;
            case kMsgSimNewDay: {
                bool ok = false;
                const int64_t j = DataOf(msg, &ok);
                if (ok) PublishJulian(static_cast<int32_t>(j));
                break;
            }
            case kMsgSimNewMonth: {
                bool ok = false;
                const int64_t j = DataOf(msg, &ok);
                if (ok) PublishJulian(static_cast<int32_t>(j));
                if (digest_) WriteDigest(ok ? static_cast<int32_t>(j) : static_cast<int32_t>(PublishedJulian()));
                break;
            }
            default:
                break;
        }
        return true;
    }

    bool Start() {
        if (started_) return true;
        digest_ = SettingInt("digest", "FC4_DIGEST", 1) != 0;
        cIGZFrameWork* fw = RZGetFrameWork();
        if (fw == nullptr) {
            Log("observer: no framework; not started");
            return false;
        }
        cIGZMessageServer2Ptr ms;
        if (!ms) {
            Log("observer: no message server; not started");
            return false;
        }
        AddRef();  // our own reference (static object)
        const bool service = fw->AddSystemService(static_cast<cIGZSystemService*>(this));
        const bool tick = fw->AddToTick(static_cast<cIGZSystemService*>(this));
        bool notes = true;
        for (uint32_t id : {kMsgPostCityInit, kMsgSimNewDay, kMsgSimNewMonth}) {
            notes = ms->AddNotification(static_cast<cIGZMessageTarget2*>(this), id) && notes;
        }
        started_ = true;
        Log("observer: started (service %d, tick %d, notifications %d; digest %s; %s mode)", service ? 1 : 0,
            tick ? 1 : 0, notes ? 1 : 0, digest_ ? "on" : "off (digest=0)",
            SimulatorReplaced() ? "replacement" : "passthrough");
        return true;
    }

    void Stop() {
        if (!started_) return;
        started_ = false;
        cIGZMessageServer2Ptr ms;
        if (ms) {
            for (uint32_t id : {kMsgPostCityInit, kMsgSimNewDay, kMsgSimNewMonth}) {
                ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(this), id);
            }
        }
        if (cIGZFrameWork* fw = RZGetFrameWork()) {
            fw->RemoveFromTick(static_cast<cIGZSystemService*>(this));
            fw->RemoveSystemService(static_cast<cIGZSystemService*>(this));
        }
        Release();
        Log("observer: stopped");
    }

private:
    // Runs the loaded city at `speed`, unpaused, through the simulator's
    // public interface (ours or the original).
    static void Unpause(int speed) {
        cISC4City* city = CurrentCity();
        cFC4ISC4Simulator* sim = city ? AsGame(city->GetSimulator()) : nullptr;
        if (sim == nullptr) {
            Log("autotest: unpause skipped: no city simulator");
            return;
        }
        int resumes = 0;
        while (sim->IsPaused() && resumes < 8) {
            if (!sim->Resume()) break;
            ++resumes;
        }
        const bool setSpeed = sim->GetSimSpeed() != speed;
        if (setSpeed) sim->SetSimSpeed(speed);
        Log("autotest: unpause: %d Resume call(s), speed %d%s; paused=%d hidden=%d emergency=%d", resumes,
            sim->GetSimSpeed(), setSpeed ? " (set)" : "", sim->IsPaused() ? 1 : 0, sim->IsHiddenPaused() ? 1 : 0,
            sim->IsEmergencyPaused() ? 1 : 0);
    }

    volatile LONG refs_ = 0;
    uint32_t serviceId_ = kObserverServiceID;
    bool running_ = true;
    bool started_ = false;
    bool digest_ = true;
};

cFC4Observer& Observer() {
    static cFC4Observer s;
    return s;
}

}  // namespace

void ObserverStart() { Observer().Start(); }
void ObserverStop() { Observer().Stop(); }

void ObserverTestSetCity(cISC4City* city) {
    g_testCity = city;
    Log("test seam: observer city %p", static_cast<void*>(city));
}
void ObserverTestTick() { Observer().OnTick(0); }
void ObserverTestMessage(uint32_t type, intptr_t data1) {
    cFC4Message2Standard* msg = FC4MakeMessage(type, data1, 0, 0);
    Observer().DoMessage(msg);
    msg->Release();
}

}  // namespace fc4
