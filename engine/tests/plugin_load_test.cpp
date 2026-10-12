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
#include "cISC4Simulator.h"
#include "SC4Percentage.h"
#include "cRZBaseString.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdio>
#include <fstream>
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

std::string ReadFile(const std::string& path) {
    std::ifstream f(path, std::ios::binary);
    std::stringstream ss;
    ss << f.rdbuf();
    return ss.str();
}

}  // namespace

int main(int argc, char** argv) {
    if (argc < 2) {
        std::fprintf(stderr, "usage: %s <FreeCity4.dll>\n", argv[0]);
        return 2;
    }
    const std::string dll = argv[1];
    const std::string log = dll.substr(0, dll.find_last_of("\\/") + 1) + "FreeCity4.log";
    DeleteFileA(log.c_str());
    SetEnvironmentVariableA("FC4_LOG_LEVEL", "2");
    SetEnvironmentVariableA("FC4_DISABLE_SIMULATOR", nullptr);

    if (argc >= 3 && std::string(argv[2]) == "--disabled") {
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
    SetEnvironmentVariableA("FC4_STATUS_EVERY", "5");

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
            cIGZDate* date = sim->GetSimDate();
            CHECK(date != nullptr && date->Year() == 2000 && static_cast<int32_t>(date->DayNumber()) == day);
            CHECK(sim->RemoveAgent(&agent));
            CHECK(agent.refs == 0);

            uint32_t y = 0, m = 0, d = 0, doy = 0, wd = 0;
            sim->GetSimDate(&y, &m, &d, &doy, &wd);
            CHECK(y == 2000 && m == 1 && doy == static_cast<uint32_t>(day - 2451545 + 1));

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
                CHECK(budget->GetDepartmentBudget(0x28F55A9F) == dept);
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
                CHECK(budget->IssueBond(10000));
                CHECK(budget->GetTotalBorrowed() == 10000);
                CHECK(budget->GetTotalFunds() == 510100);
                CHECK(budget->GetTotalMonthlyExpense() == 500u + static_cast<uint32_t>(budget->GetMonthlyPaymentForLoan(10000)));
            }
            CHECK(budget->Shutdown());
            CHECK(budget->GetDepartmentBudget(0x28F55A9F) == nullptr);
            CHECK(budget->GetTotalFunds() == 510100);  // kept across Shutdown
            CHECK(budget->Release() == 0);
        }
        if (reader != INVALID_HANDLE_VALUE) CloseHandle(reader);
    }

    const std::string text = ReadFile(log);
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

    FreeLibrary(h);
    CHECK(ReadFile(log).find("DLL_PROCESS_DETACH (FreeLibrary)") != std::string::npos);

    if (g_failures == 0) std::printf("plugin load test: all checks passed (log: %s)\n", log.c_str());
    return g_failures == 0 ? 0 : 1;
}
