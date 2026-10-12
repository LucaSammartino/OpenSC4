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
#include "cIGZSystemService.h"
#include "cISC4Simulator.h"

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

void CollectClass(uint32_t clsid, uint32_t, void* ctx) { static_cast<std::vector<uint32_t>*>(ctx)->push_back(clsid); }

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
    SetEnvironmentVariableA("FC4_DIAG_SELFTEST", "1");

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
        CHECK(classes.size() == 1 && classes[0] == kCLSID_Simulator);

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
            std::printf("simulated %d day(s) in ~600 ms\n", day - 2451545);
            CHECK(day > 2451545 + 2);

            uint32_t y = 0, m = 0, d = 0, doy = 0, wd = 0;
            sim->GetSimDate(&y, &m, &d, &doy, &wd);
            CHECK(y == 2000 && m == 1 && doy == static_cast<uint32_t>(day - 2451545 + 1));

            CHECK(sim->Shutdown());
            CHECK(sim->GetSimDateNumber() == 2451545);
            CHECK(sim->Release() == 0);
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
    CHECK(text.find("CreateSimulator enter (factory call #1") != std::string::npos);
    CHECK(text.find("constructor enter") != std::string::npos);
    CHECK(text.find("calling fc4_sim_create") != std::string::npos);
    CHECK(text.find("fc4_sim_create returned") != std::string::npos);
    CHECK(text.find("cFC4Simulator created") != std::string::npos);
    CHECK(text.find("CreateOurs: after QueryInterface -> 1 (clean=1)") != std::string::npos);
    CHECK(text.find("vectored handler installed") != std::string::npos);
    CHECK(text.find("first-chance exception in FreeCity4.dll: code 0xC0000005 ACCESS_VIOLATION (writing address "
                    "0x00000000) at") != std::string::npos);
    CHECK(text.find("STRUCTURED EXCEPTION caught in diagnostics self-test: code 0xC0000005") != std::string::npos);
    CHECK(text.find("diagnostics self-test: guard returned 0 (expected 0)") != std::string::npos);
    CHECK(text.find("could not be written") == std::string::npos);
    CHECK(text.find("QueryInterface(0x8695664E) -> 1") != std::string::npos);
    CHECK(text.find("Init() called") != std::string::npos);
    CHECK(text.find("OnTick #1 ") != std::string::npos);
    CHECK(text.find("HiddenResume() -> 1 (pause=0 hidden=0") != std::string::npos);
    CHECK(text.find("cFC4Simulator destroyed") != std::string::npos);

    FreeLibrary(h);
    CHECK(ReadFile(log).find("DLL_PROCESS_DETACH (FreeLibrary)") != std::string::npos);

    if (g_failures == 0) std::printf("plugin load test: all checks passed (log: %s)\n", log.c_str());
    return g_failures == 0 ? 0 : 1;
}
