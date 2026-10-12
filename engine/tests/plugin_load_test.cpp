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

// Loads the real FreeCity4.dll the way a GZCOM host does (LoadLibrary +
// GZDllGetGZCOMDirector), without a framework, and drives the director and
// the simulator through their vtables. Then checks that the diagnostics log
// recorded the calls.
//
// Usage: fc4_plugin_load_test <path to FreeCity4.dll>

#include "cIGZCOMDirector.h"
#include "cIGZDate.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSystemService.h"
#include "cISC4BudgetSimulator.h"
#include "cISC4DepartmentBudget.h"
#include "cISC4LineItem.h"
#include "cISCPropertyHolder.h"
#include "cRZBaseVariant.h"
#include "cSCBaseProperty.h"
#include "cISC4Simulator.h"
#include "SC4Percentage.h"
#include "cRZBaseString.h"
#include "FC4Interfaces.h"
#include "game_abi.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdio>
#include <fstream>
#include <map>
#include <sstream>
#include <string>
#include <vector>

namespace {

int g_failures = 0;

#define CHECK(cond)                                                                       \
    do {                                                                                  \
        if (!(cond)) {                                                                    \
            std::fprintf(stderr, "%s:%d: CHECK failed: %s\n", __FILE__, __LINE__, #cond); \
            ++g_failures;                                                                 \
        }                                                                                 \
    } while (0)

constexpr uint32_t kCLSID_Simulator = 0x2990C1E5;
constexpr uint32_t kIID_cISC4Simulator = 0x8695664E;
constexpr uint32_t kDirectorID = 0x5C7A43E1;
constexpr uint32_t kCLSID_Budget = 0xE990BE01;
constexpr uint32_t kCLSID_Department = 0xE990BFFC;
constexpr uint32_t kCLSID_LineItem = 0xAA313C9F;
constexpr uint32_t kIID_cISC4BudgetSimulator = 0x6772BD23;
constexpr uint32_t kIID_cISC4DepartmentBudget = 0x223E2B67;

void CollectClass(uint32_t clsid, uint32_t, void* ctx) { static_cast<std::vector<uint32_t>*>(ctx)->push_back(clsid); }

// A handled access violation in the HOST's code (not the DLL's): the DLL's
// vectored handler must log it with the call ring.
int ForeignFault() {
    __try {
        *static_cast<volatile int*>(nullptr) = 1;
    } __except (EXCEPTION_EXECUTE_HANDLER) {
        return 1;
    }
    return 0;
}

// A building's property holder with just what the budget reads and writes
// (budget-item arrays), laid out like the game's (foundations §17.2): the
// plugin calls holders only through those slots.
class FakeHolder final : public cFC4ISCPropertyHolder {
public:
    ~FakeHolder() { RemoveAllProperties(false); }
    bool QueryInterface(uint32_t riid, void** pp) override {
        if (riid != GZIID_cISCPropertyHolder && riid != GZIID_cIGZUnknown) return false;
        *pp = this;
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return --refs; }  // stack object: never deleted

    bool HasProperty(uint32_t id) const override { return props.count(id) != 0; }
    bool GetPropertyList(cIGZUnknownList**) const override { return false; }
    bool GetPropertyData(uint32_t, void*, uint32_t&) const override { return false; }
    bool GetPropertyInterface(uint32_t, uint32_t, void**) const override { return false; }
    bool GetPropertyString(uint32_t, cIGZString&) const override { return false; }
    bool GetPropertyU32(uint32_t, uint32_t&) const override { return false; }
    cISCProperty* GetProperty(uint32_t id) const override {
        auto it = props.find(id);
        return it == props.end() ? nullptr : it->second;
    }
    bool AddPropertyData(uint32_t, void*, uint32_t, bool) override { return false; }
    bool AddPropertyObject(uint32_t, cIGZUnknown*, bool) override { return false; }
    bool AddPropertyString(uint32_t, cIGZString const&, bool) override { return false; }
    bool AddPropertyU32(uint32_t, uint32_t, bool) override { return false; }
    bool AddPropertyVariant(uint32_t id, cIGZVariant const* v, bool) override {
        RemoveProperty(id, false);
        cSCBaseProperty* p = new cSCBaseProperty(id, v);  // deep copy
        p->AddRef();
        props[id] = p;
        return true;
    }
    bool AddPropertyRecord(cISCProperty*, bool) override { return false; }
    bool CopyAddProperty(cISCProperty*, bool) override { return false; }
    bool RemoveProperty(uint32_t id, bool) override {
        auto it = props.find(id);
        if (it == props.end()) return false;
        it->second->Release();
        props.erase(it);
        return true;
    }
    bool RemoveAllProperties(bool) override {
        for (auto& kv : props) kv.second->Release();
        props.clear();
        return true;
    }
    bool EnumPropertiesFiltered(FilterFn, EnumFn, void*) const override { return false; }
    bool EnumProperties(EnumFn, void*) const override { return false; }
    bool CompactProperties() override { return true; }

    cISCPropertyHolder* AsSdk() { return reinterpret_cast<cISCPropertyHolder*>(static_cast<cFC4ISCPropertyHolder*>(this)); }

    template <class T>
    void Set(uint32_t id, T value) {
        cRZBaseVariant v;
        SetArray(v, &value);
        AddPropertyVariant(id, &v, false);
    }
    float Get32f(uint32_t id) const {
        const cISCProperty* p = GetProperty(id);
        return p != nullptr ? p->GetPropertyValue()->RefFloat32()[0] : -1.0f;
    }

    uint32_t refs = 1;

private:
    static void SetArray(cRZBaseVariant& v, uint32_t* x) { v.RefUint32(x, 1); }
    static void SetArray(cRZBaseVariant& v, int64_t* x) { v.RefSint64(x, 1); }
    static void SetArray(cRZBaseVariant& v, float* x) { v.RefFloat32(x, 1); }
    std::map<uint32_t, cSCBaseProperty*> props;
};

// A day agent that reports "not finished" for its first 150 calls.
class SlowAgent final : public cIGZMessageTarget2 {
public:
    uint32_t refs = 0;
    uint32_t calls = 0;
    uint32_t badMessages = 0;
    int32_t lastJulian = 0;
    bool QueryInterface(uint32_t riid, void** pp) override {
        if (riid == GZIID_cIGZUnknown) {
            *pp = static_cast<cIGZUnknown*>(this);
            AddRef();
            return true;
        }
        return false;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return refs > 0 ? --refs : 0; }
    bool DoMessage(cIGZMessage2* msg) override {
        ++calls;
        // Read the message the way game agents do: cast to the standard interface.
        auto* m = static_cast<cIGZMessage2Standard*>(msg);
        if (msg->GetType() != 0x66956814 || !m->GetHasData1() || m->GetData2() != 0 || m->GetData1() < 2451545) {
            ++badMessages;
        }
        lastJulian = static_cast<int32_t>(m->GetData1());
        return calls > 150;
    }
};

// A day agent whose first call blocks for 6.5 s (the watchdog's stall threshold is 5 s).
class BlockingAgent final : public cIGZMessageTarget2 {
public:
    uint32_t refs = 0;
    uint32_t calls = 0;
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return refs > 0 ? --refs : 0; }
    bool DoMessage(cIGZMessage2*) override {
        if (calls++ == 0) Sleep(6500);
        return true;
    }
};

size_t CountOf(const std::string& text, const std::string& what) {
    size_t n = 0;
    for (size_t at = text.find(what); at != std::string::npos; at = text.find(what, at + 1)) ++n;
    return n;
}

std::string ReadFile(const std::string& path) {
    std::ifstream f(path, std::ios::binary);
    std::stringstream ss;
    ss << f.rdbuf();
    return ss.str();
}

void WriteTextFile(const std::string& path, const std::string& text) {
    std::ofstream f(path, std::ios::binary | std::ios::trunc);
    f << text;
}

}  // namespace

int main(int argc, char** argv) {
    if (argc < 2) {
        std::fprintf(stderr, "usage: %s <FreeCity4.dll>\n", argv[0]);
        return 2;
    }
    const std::string dll = argv[1];
    const std::string mode = argc >= 3 ? argv[2] : "--normal";
    const std::string log = dll.substr(0, dll.find_last_of("\\/") + 1) + "FreeCity4.log";
    const std::string ini = dll.substr(0, dll.find_last_of("\\/") + 1) + "FreeCity4.ini";
    DeleteFileA(log.c_str());
    DeleteFileA(ini.c_str());  // a leftover from an aborted run must not change this one
    SetEnvironmentVariableA("FC4_LOG_LEVEL", "2");
    SetEnvironmentVariableA("FC4_DISABLE_SIMULATOR", nullptr);

    if (mode == "--ini-disabled") {
        // FreeCity4.ini `disable =` wins over FC4_DISABLE; switches are read at
        // game start; unknown keys are reported.
        SetEnvironmentVariableA("FC4_DISABLE", "simulator");
        WriteTextFile(ini, "; bisection test\n[subsystems]\ndisable = Budget   # the original budget\nlot_scan = off\n"
                       "bogus_key = 1\n");
        HMODULE hd = LoadLibraryA(dll.c_str());
        CHECK(hd != nullptr);
        if (hd == nullptr) return 1;
        auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
        cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
        CHECK(director != nullptr);
        if (director != nullptr) {
            std::vector<uint32_t> classes;
            director->EnumClassObjects(&CollectClass, &classes);
            CHECK(classes.size() == 1 && classes[0] == kCLSID_Simulator);
            void* obj = nullptr;
            CHECK(!director->GetClassObject(kCLSID_Budget, kIID_cISC4BudgetSimulator, &obj));
        }
        FreeLibrary(hd);
        DeleteFileA(ini.c_str());
        const std::string text = ReadFile(log);
        CHECK(text.find("subsystem 'budget': DISABLED by FreeCity4.ini (disable=)") != std::string::npos);
        CHECK(text.find("subsystem 'simulator': enabled (1 class replaced)") != std::string::npos);
        CHECK(text.find("config (game start): ") != std::string::npos);
        CHECK(text.find("FreeCity4.ini found; budget_default=1; set by ini: lot_scan=0; set by env: -") !=
              std::string::npos);
        CHECK(text.find("config (game start): budget switches OFF (1 of 11): lot_scan") != std::string::npos);
        CHECK(text.find("config: WARNING unknown key 'bogus_key' in FreeCity4.ini (ignored)") != std::string::npos);
        if (g_failures == 0) std::printf("plugin ini disable test: all checks passed\n");
        return g_failures == 0 ? 0 : 1;
    }

    if (mode == "--disabled") {
        // FC4_DISABLE switches subsystems off: nothing is registered or created.
        SetEnvironmentVariableA("FC4_DISABLE", "Budget, Simulator");
        HMODULE hd = LoadLibraryA(dll.c_str());
        CHECK(hd != nullptr);
        if (hd == nullptr) return 1;
        auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
        cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
        CHECK(director != nullptr);
        if (director != nullptr) {
            std::vector<uint32_t> classes;
            director->EnumClassObjects(&CollectClass, &classes);
            CHECK(classes.empty());
            void* obj = nullptr;
            CHECK(!director->GetClassObject(kCLSID_Simulator, kIID_cISC4Simulator, &obj));
            CHECK(!director->GetClassObject(kCLSID_Budget, kIID_cISC4BudgetSimulator, &obj));
        }
        FreeLibrary(hd);
        const std::string text = ReadFile(log);
        CHECK(text.find("subsystem 'simulator': DISABLED by FC4_DISABLE") != std::string::npos);
        CHECK(text.find("subsystem 'budget': DISABLED by FC4_DISABLE") != std::string::npos);
        CHECK(text.find("object created") == std::string::npos);
        if (g_failures == 0) std::printf("plugin disable test: all checks passed\n");
        return g_failures == 0 ? 0 : 1;
    }
    SetEnvironmentVariableA("FC4_DIAG_SELFTEST", "1");
    SetEnvironmentVariableA("FC4_CHECKED_SELFTEST", "1");
    SetEnvironmentVariableA("FC4_STATUS_EVERY", "5");
    SetEnvironmentVariableA("FC4_HEARTBEAT_MS", "2000");

    HMODULE h = LoadLibraryA(dll.c_str());
    CHECK(h != nullptr);
    if (h == nullptr) return 1;

    using GetDirectorFn = cIGZCOMDirector* (*)();
    auto getDirector = reinterpret_cast<GetDirectorFn>(GetProcAddress(h, "GZDllGetGZCOMDirector"));
    CHECK(getDirector != nullptr);
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    if (director != nullptr) {
        CHECK(director->GetDirectorID() == kDirectorID);

        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(classes.size() == 4);
        CHECK(classes.size() == 4 && classes[0] == kCLSID_Simulator && classes[1] == kCLSID_Budget &&
              classes[2] == kCLSID_Department && classes[3] == kCLSID_LineItem);

        // Another process (an editor, a tail) holding the log open must not stop the logging.
        HANDLE reader = CreateFileA(log.c_str(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                                    nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
        CHECK(reader != INVALID_HANDLE_VALUE);

        // Our class with an interface it does not implement: refused, object destroyed.
        void* nope = reinterpret_cast<void*>(1);
        CHECK(!director->GetClassObject(kCLSID_Simulator, 0x0BADF00D, &nope));
        CHECK(nope == nullptr);

        cISC4Simulator* sim = nullptr;
        CHECK(director->GetClassObject(kCLSID_Simulator, kIID_cISC4Simulator, reinterpret_cast<void**>(&sim)));
        CHECK(sim != nullptr);
        CHECK(!director->GetClassObject(0x12345678, kIID_cISC4Simulator, reinterpret_cast<void**>(&sim)) ||
              sim != nullptr);

        if (sim != nullptr) {
            CHECK(sim->Init());
            CHECK(sim->IsHiddenPaused());
            CHECK(sim->GetSimDateNumber() == 2451545);
            CHECK(sim->SetSimSpeed(2));
            CHECK(sim->HiddenResume());
            CHECK(!sim->IsAnyPaused());

            SlowAgent agent;
            CHECK(sim->AddAgent(&agent, cISC4Simulator::AgentTypeSimNewDay, cRZBaseString("test day agent"),
                                static_cast<cISC4Simulator::eAgentFlags>(3)));
            CHECK(agent.refs == 1);

            cIGZSystemService* service = nullptr;
            CHECK(sim->QueryInterface(GZIID_cIGZSystemService, reinterpret_cast<void**>(&service)));
            if (service != nullptr) {
                CHECK(service->GetServiceID() == 0x46956659);
                CHECK(service->GetServicePriority() == -2000000);
                // Default tuning: 100 ms per day at speed 2. Tick for ~600 ms.
                const DWORD start = GetTickCount();
                while (GetTickCount() - start < 600) service->OnTick(0);
                service->Release();
            }
            const int32_t day = sim->GetSimDateNumber();
            std::printf("simulated %d day(s) in ~600 ms; agent called %u times\n", day - 2451545, agent.calls);
            CHECK(day > 2451545 + 2);
            CHECK(agent.calls > 150);
            CHECK(agent.badMessages == 0);
            CHECK(agent.lastJulian == day);
            // Overloaded methods are only ever called through the game's slots (foundations §16).
            cIGZDate* date = reinterpret_cast<cIGZDate*>(fc4test::CallSlot(sim, 7, {}).eax);  // GetSimDate()
            CHECK(date != nullptr && date->Year() == 2000 && static_cast<int32_t>(date->DayNumber()) == day);
            if (date != nullptr) {
                // cIGZDate slots 33-36 (foundations §17.4): a uint32 day number in
                // EAX, one stack argument, nothing on the x87 stack, and the date
                // itself unchanged.
                using fc4test::CallSlot;
                const uint32_t d = static_cast<uint32_t>(day);
                cIGZDate* other = nullptr;
                CHECK((CallSlot(date, 9, {reinterpret_cast<uint32_t>(&other)}).eax & 0xFF) == 1);  // Clone
                CHECK(other != nullptr);
                if (other != nullptr) {
                    CHECK((CallSlot(other, 4, {100}).eax & 0xFF) == 1);  // Set(uint32 dayNumber)
                    const fc4test::SlotCall m1 = CallSlot(date, 33, {7});  // operator-(int32)
                    const fc4test::SlotCall m2 = CallSlot(date, 34, {reinterpret_cast<uint32_t>(other)});  // operator-(date)
                    const fc4test::SlotCall p1 = CallSlot(date, 35, {7});  // operator+(int32)
                    const fc4test::SlotCall p2 = CallSlot(date, 36, {reinterpret_cast<uint32_t>(other)});  // operator+(date)
                    std::printf("date slots 33-36: %u %u %u %u (day %u)\n", m1.eax, m2.eax, p1.eax, p2.eax, d);
                    CHECK(m1.eax == d - 7 && m2.eax == d - 100 && p1.eax == d + 7 && p2.eax == d + 100);
                    for (const fc4test::SlotCall* c : {&m1, &m2, &p1, &p2}) {
                        CHECK(c->espDelta == 0 && c->x87Pushed == 0);
                    }
                    CHECK(date->DayNumber() == d && other->DayNumber() == 100u);  // neither date modified
                    // Slots 16/17: FirstDayOfMonth(month) / FirstDayOfMonth().
                    const fc4test::SlotCall f16 = CallSlot(date, 16, {3});
                    const fc4test::SlotCall f17 = CallSlot(date, 17, {});
                    CHECK(f16.espDelta == 0 && f17.espDelta == 0);
                    CHECK(f16.eax == 2451545u + 31u + 29u);  // 2000-03-01
                    CHECK(f17.eax == 2451545u);              // the sim date is in January 2000
                    other->Release();
                }
            }
            {
                // Overloads through the game's slots (simulator §5.6), with the
                // game's argument counts: the stack must balance.
                using fc4test::CallSlot;
                uint32_t gy = 0, gm = 0, gd = 0, gdoy = 0, gwd = 0;
                const fc4test::SlotCall s6 = CallSlot(sim, 6, {reinterpret_cast<uint32_t>(&gy), reinterpret_cast<uint32_t>(&gm),
                                                         reinterpret_cast<uint32_t>(&gd), reinterpret_cast<uint32_t>(&gdoy),
                                                         reinterpret_cast<uint32_t>(&gwd)});
                std::printf("slot 6: esp %d, year %u\n", s6.espDelta, gy);
                CHECK(s6.espDelta == 0 && gy == 2000);
                const fc4test::SlotCall s7 = CallSlot(sim, 7, {});
                std::printf("slot 7: esp %d, eax %08X\n", s7.espDelta, s7.eax);
                CHECK(s7.espDelta == 0 && s7.eax != 0);
                SlowAgent other;
                const fc4test::SlotCall s20 = CallSlot(sim, 20, {reinterpret_cast<uint32_t>(static_cast<cIGZMessageTarget2*>(&other))});
                const fc4test::SlotCall s21 = CallSlot(sim, 21, {reinterpret_cast<uint32_t>(static_cast<cIGZMessageTarget2*>(&other)), 4});
                const fc4test::SlotCall s22 = CallSlot(sim, 22, {9});
                std::printf("slots 20/21/22: esp %d %d %d\n", s20.espDelta, s21.espDelta, s22.espDelta);
                CHECK(s20.espDelta == 0 && s21.espDelta == 0 && s22.espDelta == 0);
            }
            {
                // RemoveAgent(agent, level) at slot 21 removes the day registration.
                const fc4test::SlotCall r = fc4test::CallSlot(
                    sim, 21, {reinterpret_cast<uint32_t>(static_cast<cIGZMessageTarget2*>(&agent)), 4});
                CHECK(r.espDelta == 0 && (r.eax & 0xFF) == 1);
            }
            CHECK(agent.refs == 0);
            {
                // RemoveAllAgents() at slot 23.
                const fc4test::SlotCall r = fc4test::CallSlot(sim, 23, {});
                CHECK(r.espDelta == 0);
            }

            uint32_t y = 0, m = 0, d = 0, doy = 0, wd = 0;
            const fc4test::SlotCall r6 = fc4test::CallSlot(
                sim, 6,
                {reinterpret_cast<uint32_t>(&y), reinterpret_cast<uint32_t>(&m), reinterpret_cast<uint32_t>(&d),
                 reinterpret_cast<uint32_t>(&doy), reinterpret_cast<uint32_t>(&wd)});
            CHECK(r6.espDelta == 0);
            CHECK(y == 2000 && m == 1 && doy == static_cast<uint32_t>(day - 2451545 + 1));

            {
                // Watchdog: a day agent that blocks for 6.5 s inside OnTick must
                // produce a stall report (activity, call ring, main-thread sample)
                // and an "over" line; heartbeats run meanwhile (FC4_HEARTBEAT_MS=2000).
                BlockingAgent blocker;
                CHECK(sim->AddAgent(&blocker, cISC4Simulator::AgentTypeSimNewDay, cRZBaseString("blocking test agent"),
                                    static_cast<cISC4Simulator::eAgentFlags>(3)));
                cIGZSystemService* svc = nullptr;
                CHECK(sim->QueryInterface(GZIID_cIGZSystemService, reinterpret_cast<void**>(&svc)));
                if (svc != nullptr) {
                    const DWORD start = GetTickCount();
                    while (blocker.calls == 0 && GetTickCount() - start < 3000) svc->OnTick(0);
                    const DWORD after = GetTickCount();
                    while (GetTickCount() - after < 2500) svc->OnTick(0);  // let the watchdog see the end
                    svc->Release();
                }
                CHECK(blocker.calls >= 1);
                fc4test::CallSlot(sim, 20, {reinterpret_cast<uint32_t>(static_cast<cIGZMessageTarget2*>(&blocker))});
            }

            CHECK(sim->Shutdown());
            CHECK(sim->GetSimDateNumber() == 2451545);
            CHECK(sim->Release() == 0);
        }

        // The budget without a game: no city, no message server, no exemplar
        // (code defaults; difficulty 0 without a city).
        cISC4BudgetSimulator* budget = nullptr;
        CHECK(director->GetClassObject(kCLSID_Budget, kIID_cISC4BudgetSimulator, reinterpret_cast<void**>(&budget)));
        if (budget != nullptr) {
            CHECK(budget->Init());
            CHECK(budget->GetTotalFunds() == 500000);
            CHECK(budget->DepositFunds(100));
            CHECK(!budget->WithdrawFunds(600000));
            CHECK(budget->GetTotalFunds() == 500100);
            CHECK(budget->GetTaxRate(0) == 9.0f);

            cISC4DepartmentBudget* dept = budget->CreateDepartmentBudget(0x28F55A9F, 7);
            CHECK(dept != nullptr);
            if (dept != nullptr) {
                CHECK(dept->GetDepartmentID() == 0x28F55A9F);
                CHECK(fc4test::CallSlot(budget, 28, {0x28F55A9F}).eax == reinterpret_cast<uint32_t>(dept));
                cISC4DepartmentBudget* viaClsid = nullptr;
                CHECK(dept->QueryInterface(kCLSID_Department, reinterpret_cast<void**>(&viaClsid)));
                CHECK(viaClsid == dept);
                if (viaClsid != nullptr) viaClsid->Release();
                cISC4DepartmentBudget* viaIid = nullptr;
                CHECK(dept->QueryInterface(kIID_cISC4DepartmentBudget, reinterpret_cast<void**>(&viaIid)));
                if (viaIid != nullptr) viaIid->Release();

                cISC4LineItem* line = dept->CreateLineItem(1, false);
                CHECK(line != nullptr);
                if (line != nullptr) {
                    cISC4LineItem* viaLineClsid = nullptr;
                    CHECK(line->QueryInterface(kCLSID_LineItem, reinterpret_cast<void**>(&viaLineClsid)));
                    if (viaLineClsid != nullptr) viaLineClsid->Release();
                    CHECK(line->GetID() == 1);
                    CHECK(line->SetFullExpenses(1000));
                    SC4Percentage half{0.5f};
                    CHECK(dept->SetFundingPercentage(half, 0));
                    CHECK(dept->GetFundingPercentage()->percentage == 0.5f);
                    CHECK(line->GetCurrentExpenses() == 500);
                    CHECK(dept->GetLineItem(1) == line);
                }
                CHECK(budget->GetTotalMonthlyExpense() == 500u);  // low 32 bits of the 64-bit value
                {
                    // Overloads through the game's slots (budget "Verified slot order").
                    using fc4test::CallSlot;
                    const fc4test::SlotCall s18 = CallSlot(budget, 18, {0});  // GetTaxIncome(int32)
                    const fc4test::SlotCall s19 = CallSlot(budget, 19, {});   // GetTaxIncome()
                    const fc4test::SlotCall s34 = CallSlot(budget, 34, {0x28F55A9F});  // GetFunding(uint32)
                    const fc4test::SlotCall s35 = CallSlot(budget, 35, {reinterpret_cast<uint32_t>(dept)});
                    CHECK(s34.espDelta == 0 && s35.espDelta == 0 && s34.eax == 0 && s35.eax == 0);
                    std::printf("budget slots 18/19: esp %d %d\n", s18.espDelta, s19.espDelta);
                    CHECK(s18.espDelta == 0 && s19.espDelta == 0);
                    cRZBaseString empty("");
                    const fc4test::SlotCall s27 = CallSlot(budget, 27, {reinterpret_cast<uint32_t>(static_cast<cIGZString*>(&empty))});
                    const fc4test::SlotCall s28 = CallSlot(budget, 28, {0x28F55A9F});
                    std::printf("budget slots 27/28: esp %d %d, %08X %08X (dept %p)\n", s27.espDelta, s28.espDelta, s27.eax,
                                s28.eax, static_cast<void*>(dept));
                    CHECK(s27.espDelta == 0 && s28.espDelta == 0);
                    CHECK(s27.eax == reinterpret_cast<uint32_t>(dept));  // "" is a prefix of every name
                    CHECK(s28.eax == reinterpret_cast<uint32_t>(dept));
                }
                CHECK(budget->IssueBond(10000));
                CHECK(budget->GetTotalBorrowed() == 10000);
                CHECK(budget->GetTotalFunds() == 510100);
                CHECK(budget->GetTotalMonthlyExpense() == 500u + static_cast<uint32_t>(budget->GetMonthlyPaymentForLoan(10000)));
                {
                    // Slot 26, GetLocalFundingPercent, returns a POINTER to a float
                    // in EAX (budget erratum): the game dereferences it at once.
                    FakeHolder holder;
                    holder.Set<uint32_t>(0xEA54D283, 0x28F55A9F);  // department
                    holder.Set<uint32_t>(0xEA54D284, 2);           // line item
                    holder.Set<uint32_t>(0xEA54D285, 7);           // purpose
                    holder.Set<int64_t>(0xEA54D286, 200);          // cost
                    CHECK(dept->CreateLineItem(2, true) != nullptr);
                    CHECK(dept->AddLocallyFundedObject(holder.AsSdk(), 7));
                    CHECK(holder.refs == 2);
                    const uint32_t h = reinterpret_cast<uint32_t>(holder.AsSdk());
                    using fc4test::CallSlot;
                    const fc4test::SlotCall found = CallSlot(dept, 26, {h, 7});
                    const fc4test::SlotCall otherPurpose = CallSlot(dept, 26, {h, 8});
                    const fc4test::SlotCall noHolder = CallSlot(dept, 26, {0, 7});
                    for (const fc4test::SlotCall* c : {&found, &otherPurpose, &noHolder}) {
                        for (int i = 0; i < c->x87Pushed; ++i) fc4test::PopX87();
                    }
                    std::printf("department slot 26: found %08X (%d x87), not found %08X / %08X\n", found.eax,
                                found.x87Pushed, otherPurpose.eax, noHolder.eax);
                    CHECK(found.espDelta == 0 && otherPurpose.espDelta == 0 && noHolder.espDelta == 0);
                    CHECK(found.x87Pushed == 0 && otherPurpose.x87Pushed == 0 && noHolder.x87Pushed == 0);
                    CHECK(found.eax != 0 && otherPurpose.eax != 0 && noHolder.eax != 0);
                    if (found.eax != 0) {
                        // The item's local-funding fraction: property value / 100.
                        const float q = reinterpret_cast<const float*>(found.eax)[0];
                        CHECK(q == holder.Get32f(0xEA54D287) / 100.0f);
                        CHECK(q == 0.5f);  // AddLocallyFundedObject wrote the department's 50 %
                    }
                    if (otherPurpose.eax != 0) CHECK(reinterpret_cast<const float*>(otherPurpose.eax)[0] == 0.0f);
                    CHECK(noHolder.eax == otherPurpose.eax);  // the shared constant 0.0
                    CHECK(found.eax != otherPurpose.eax);
                    CHECK(dept->RemoveLocallyFundedObject(holder.AsSdk(), 7));
                    CHECK(holder.refs == 1);
                    const fc4test::SlotCall gone = CallSlot(dept, 26, {h, 7});
                    CHECK(gone.eax == otherPurpose.eax && gone.espDelta == 0 && gone.x87Pushed == 0);
                }
            }
            CHECK(budget->Shutdown());
            CHECK(fc4test::CallSlot(budget, 28, {0x28F55A9F}).eax == 0);
            CHECK(budget->GetTotalFunds() == 510100);  // kept across Shutdown
            CHECK(budget->Release() == 0);
        }

        // Bisection switches: FreeCity4.ini is re-read at city load (budget
        // Init) and wins over the environment; a switch that is off makes its
        // behaviour a no-op.
        SetEnvironmentVariableA("FC4_BUDGET_CHEATS", "0");    // overridden by the ini
        SetEnvironmentVariableA("FC4_BUDGET_DISASTER", "0");  // not in the ini: applies
        WriteTextFile(ini, "[budget]\r\nholder_refs = 0\r\nproperty_writes=off ; no writes\r\ncheats = 1\r\n");
        cISC4BudgetSimulator* budget2 = nullptr;
        CHECK(director->GetClassObject(kCLSID_Budget, kIID_cISC4BudgetSimulator, reinterpret_cast<void**>(&budget2)));
        if (budget2 != nullptr) {
            CHECK(budget2->Init());
            cISC4DepartmentBudget* dept = budget2->CreateDepartmentBudget(0x28F55A9F, 7);
            CHECK(dept != nullptr);
            if (dept != nullptr) {
                CHECK(dept->CreateLineItem(2, true) != nullptr);
                FakeHolder holder;
                holder.Set<uint32_t>(0xEA54D283, 0x28F55A9F);
                holder.Set<uint32_t>(0xEA54D284, 2);
                holder.Set<uint32_t>(0xEA54D285, 7);
                holder.Set<int64_t>(0xEA54D286, 200);
                CHECK(dept->AddLocallyFundedObject(holder.AsSdk(), 7));
                CHECK(holder.refs == 1);                             // holder_refs off: no reference held
                CHECK(holder.Get32f(0xEA54D287) == -1.0f);           // property_writes off: nothing written
                CHECK(dept->RemoveLocallyFundedObject(holder.AsSdk(), 7));
                CHECK(holder.refs == 1);
            }
            CHECK(budget2->Shutdown());
            CHECK(budget2->Release() == 0);
        }
        DeleteFileA(ini.c_str());
        SetEnvironmentVariableA("FC4_BUDGET_CHEATS", nullptr);
        SetEnvironmentVariableA("FC4_BUDGET_DISASTER", nullptr);
        if (reader != INVALID_HANDLE_VALUE) CloseHandle(reader);
    }

    // The same faulting address three times: reported at most twice.
    CHECK(ForeignFault() == 1);
    CHECK(ForeignFault() == 1);
    CHECK(ForeignFault() == 1);
    const std::string text = ReadFile(log);
    CHECK(CountOf(text, "first-chance exception OUTSIDE FreeCity4.dll in fc4_plugin_load_test.exe+0x") == 2);
    CHECK(text.find("(report 2 of at most 2 for this address)") != std::string::npos);
    // Watchdog (hang diagnostics).
    CHECK(text.find("watchdog: started for main thread") != std::string::npos);
    CHECK(text.find("WATCHDOG: main thread (tid ") != std::string::npos);
    CHECK(text.find("stalled - inside FreeCity4 for ") != std::string::npos);
    CHECK(text.find("  activity: OnTick (") != std::string::npos);
    CHECK(text.find("> core fc4_sim_on_tick (") != std::string::npos);
    CHECK(text.find("| delivering msg 0x66956814 (level 4) to agent ") != std::string::npos);
    CHECK(text.find("'blocking test agent' for ") != std::string::npos);
    CHECK(text.find("  context: eip=0x") != std::string::npos);
    CHECK(text.find("] FreeCity4.dll+0x") != std::string::npos);  // a return address into our image
    CHECK(text.find("] fc4_plugin_load_test.exe+0x") != std::string::npos);  // ... and into the executable
    CHECK(text.find("  cpu: process ") != std::string::npos);
    CHECK(text.find("WATCHDOG: stall over after ") != std::string::npos);
    CHECK(text.find("heartbeat (periodic): 2000-01-") != std::string::npos);
    CHECK(text.find("watchdog: stopped") != std::string::npos);
    CHECK(text.find("SetSimSpeed(2) -> 1 (julian 2451545") != std::string::npos);
    CHECK(text.find("HiddenResume() -> 1 (pause=0 hidden=0 emergency=0; julian 2451545 speed 2)") != std::string::npos);
    CHECK(text.find("first-chance exception OUTSIDE FreeCity4.dll in fc4_plugin_load_test.exe+0x") != std::string::npos);
    CHECK(text.find("FreeCity4 interface calls (oldest first):") != std::string::npos);
    CHECK(text.find("] BudgetSimulator::Shutdown") != std::string::npos);
    CHECK(text.find("call BudgetSimulator::GetTaxIncomeOfType (#1)") != std::string::npos);
    CHECK(text.find("DLL_PROCESS_ATTACH") != std::string::npos);
    CHECK(text.find("cFC4Director constructed") != std::string::npos);
    CHECK(text.find("EnumClassObjects: reporting clsid 0x2990C1E5") != std::string::npos);
    CHECK(text.find("GetClassObject enter (clsid=0x2990C1E5, iid=0x8695664E") != std::string::npos);
    CHECK(text.find("GetClassObject exit (clsid=0x2990C1E5, iid=0x8695664E) -> 1") != std::string::npos);
    CHECK(text.find("GetClassObject exit (clsid=0x2990C1E5, iid=0x0BADF00D) -> 0") != std::string::npos);
    CHECK(text.find("GetClassObject exit (clsid=0x12345678, iid=0x8695664E) -> 0") != std::string::npos);
    CHECK(text.find("factory enter: cSC4Simulator (0x2990C1E5), factory call #1") != std::string::npos);
    CHECK(text.find("constructor enter") != std::string::npos);
    CHECK(text.find("calling fc4_sim_create") != std::string::npos);
    CHECK(text.find("fc4_sim_create returned") != std::string::npos);
    CHECK(text.find("cSC4Simulator object created") != std::string::npos);
    CHECK(text.find("CreateOurs: after QueryInterface -> 1 (clean=1)") != std::string::npos);
    CHECK(text.find("vectored handler installed") != std::string::npos);
    CHECK(text.find("first-chance exception in FreeCity4.dll: code 0xC0000005 ACCESS_VIOLATION (writing address "
                    "0x00000000) at") != std::string::npos);
    CHECK(text.find("STRUCTURED EXCEPTION caught in diagnostics self-test: code 0xC0000005") != std::string::npos);
    CHECK(text.find("diagnostics self-test: guard returned 0 (expected 0)") != std::string::npos);
    CHECK(text.find("could not be written") == std::string::npos);
    CHECK(text.find("AddAgent(") != std::string::npos);
    CHECK(text.find("deliver #1: level 4 type 0x66956814 d1=2451546 d2=0 d3=2 -> agent") != std::string::npos);
    CHECK(text.find("'test day agent' returned 0") != std::string::npos);
    CHECK(text.find("'test day agent' (level 4) has now returned false 100 times in a row") != std::string::npos);
    CHECK(text.find("status @tick 5:") != std::string::npos);
    CHECK(text.find("DAY_GATE_OPEN=") != std::string::npos);
    CHECK(text.find("  level 4: 1 agent(s), cursor") != std::string::npos);
    CHECK(text.find("post SimNewDay 0x66956814 d1=2451546 (2000-01-02) d2=0 d3=2 -> MessagePost skipped") !=
          std::string::npos);
    CHECK(text.find("post 0x4A7FB7E9 d1=2") != std::string::npos);  // SpeedChanged
    CHECK(text.find("GetSimDate() #1 -> date object") != std::string::npos);
    CHECK(text.find("::Year() -> 2000") != std::string::npos);
    CHECK(text.find("GetSimDateNumber() #1 -> 2451545") != std::string::npos);
    // Limit-timer calibration (spec 5.5): installed and published, but this test
    // process is not SimCity 4, so the compat guard must refuse to write.
    CHECK(text.find("calibration: QPC ") != std::string::npos);
    CHECK(text.find("installed=1") != std::string::npos);
    CHECK(text.find("compat: host executable ") != std::string::npos);
    CHECK(text.find("compat: WARNING unrecognised executable") != std::string::npos);
    CHECK(text.find("written to the game=yes") == std::string::npos);
    CHECK(text.find("calibration (Init): installed") != std::string::npos);
    CHECK(text.find("calibration (status): installed") != std::string::npos);
    CHECK(text.find("QueryInterface(0x8695664E) -> 1") != std::string::npos);
    CHECK(text.find("Init() called") != std::string::npos);
    CHECK(text.find("OnTick #1 ") != std::string::npos);
    CHECK(text.find("HiddenResume() -> 1 (pause=0 hidden=0") != std::string::npos);
    CHECK(text.find("cFC4Simulator destroyed") != std::string::npos);
    CHECK(text.find("subsystem 'budget': enabled (3 classes replaced)") != std::string::npos);
    CHECK(text.find("cSC4BudgetSimulator object created") != std::string::npos);
    CHECK(text.find("budget: tuning exemplar unavailable; using the code defaults") != std::string::npos);
    CHECK(text.find("budget: no simulator; agents not registered") != std::string::npos);
    CHECK(text.find("budget: game-data registry node 'budget' not published (stub)") != std::string::npos);
    CHECK(text.find("budget Init: ok=1") != std::string::npos);
    CHECK(text.find("budget post 0x0772FAD4") != std::string::npos);  // FundsChanged (skipped: no server)
    CHECK(text.find("budget IssueBond(10000) -> 1 (borrowed 10000)") != std::string::npos);
    CHECK(text.find("budget Shutdown") != std::string::npos);
    CHECK(text.find("cFC4BudgetSimulator destroyed") != std::string::npos);

    // Bisection configuration.
    CHECK(text.find("config (game start): all 11 budget switches ON") != std::string::npos);
    CHECK(text.find("config (city load: budget Init): all 11 budget switches ON") != std::string::npos);
    CHECK(text.find("set by ini: property_writes=0, cheats=1, holder_refs=0; set by env: disaster=0") !=
          std::string::npos);
    CHECK(text.find("config (city load: budget Init): budget switches OFF (3 of 11): property_writes, disaster, "
                    "holder_refs") != std::string::npos);
    CHECK(text.find("switch 'holder_refs' off: AddRef of a game object for the budget core skipped") !=
          std::string::npos);
    CHECK(text.find("switch 'property_writes' off: AddPropertyVariant (reported as written) skipped") !=
          std::string::npos);
    CHECK(text.find("switches (budget Shutdown): calls skipped since the last report: property_writes ") !=
          std::string::npos);
    CHECK(text.find("budget core (before budget Shutdown): holds 0 reference(s)") != std::string::npos);
    // Reference-count audit: the fake holder and the test agents are "game" objects.
    CHECK(text.find("refaudit: on (verbose line cap 2000") != std::string::npos);
    CHECK(text.find("ref +1 AddRef ") != std::string::npos);
    CHECK(text.find("ref -1 Release ") != std::string::npos);
    CHECK(text.find("[class ?, vtable fc4_plugin_load_test.exe+0x") != std::string::npos);
    CHECK(text.find("at fc4::`anonymous-namespace'::AddRefToken:") != std::string::npos);
    CHECK(text.find("refaudit (budget Shutdown): ") != std::string::npos);
    CHECK(text.find("refaudit (simulator Shutdown): ") != std::string::npos);
    CHECK(text.find("objects we still hold references to: 0") != std::string::npos);
    CHECK(text.find("WARNING: released last reference to game object ") != std::string::npos);  // test agents
    CHECK(text.find("WARNING: over-release") == std::string::npos);
    // Checked build: RTC failures and out-parameter overruns reach the log.
    if (mode == "--checked") {
        CHECK(text.find("build: CHECKED (/Od /RTC1") != std::string::npos);
        CHECK(text.find("RTC ERROR #1 (report 1 for this call site): ESP not preserved across a call") !=
              std::string::npos);
        CHECK(text.find("  call site: FreeCity4.dll+0x") != std::string::npos);
        CHECK(text.find("ProvokeEspMismatch") != std::string::npos);  // our function name (FreeCity4.pdb)
        CHECK(text.find("OUT-PARAM OVERRUN #1: the callee wrote outside the 1-byte self-test uint8 out-parameter") !=
              std::string::npos);
        CHECK(text.find("(canary C5: before C5C5C5C5C5C5C5C5C5C5C5C5C5C5C5C5 | after 332211C5") != std::string::npos);
    } else {
        CHECK(text.find("build: normal") != std::string::npos);
        CHECK(text.find("FC4_CHECKED_SELFTEST ignored: this is not a checked build") != std::string::npos);
        CHECK(text.find("RTC ERROR") == std::string::npos);
    }
    CHECK(text.find("OUT-PARAM OVERRUN #2") == std::string::npos);

    FreeLibrary(h);
    CHECK(ReadFile(log).find("DLL_PROCESS_DETACH (FreeLibrary)") != std::string::npos);

    if (g_failures == 0) std::printf("plugin load test: all checks passed (log: %s)\n", log.c_str());
    return g_failures == 0 ? 0 : 1;
}
