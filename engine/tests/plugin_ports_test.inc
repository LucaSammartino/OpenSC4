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

// The glue12b2 lane's plugin test (included by plugin_load_test.cpp after
// plugin_cmglue_test.inc, whose helpers it uses): the airport and seaport
// developers (port-developers.md) without a city: the QI tables of §1 (1 and
// cISC4PortDeveloper give +4), the slots' answers with no ports (§4.7, §12
// test 10), the port sub-object's slots 0-9, the empty records of §12 test 1
// and the version checks, Init / DoMessage / Shutdown and Release's 0.

#include "FC4PortInterfaces.h"

namespace ports_test {

constexpr uint32_t kIID_Port = 0x4A2C4E69;
constexpr uint32_t kIID_Ser = 0xE4FDA3D4;

// §1 for one developer.
void CheckPortQi(cIGZUnknown* p, uint32_t iid, uint32_t otherIid, uint32_t clsid) {
    const uintptr_t base = reinterpret_cast<uintptr_t>(p);
    auto at = [&](uint32_t riid) -> uintptr_t {
        void* o = nullptr;
        if (!p->QueryInterface(riid, &o) || o == nullptr) return 0;
        static_cast<cIGZUnknown*>(o)->Release();
        return reinterpret_cast<uintptr_t>(o);
    };
    CHECK(at(iid) == base);
    CHECK(at(GZIID_cIGZUnknown) == base + 4);  // [compat] QueryInterface(1) gives +4
    CHECK(at(kIID_Port) == base + 4);
    CHECK(at(cmglue_test::kIID_MessageTarget2) == base + 8);
    CHECK(at(kIID_Ser) == base + 0xC);
    cIGZSerializable* ser = nullptr;
    if (p->QueryInterface(kIID_Ser, reinterpret_cast<void**>(&ser)) && ser != nullptr) {
        CHECK(ser->GetGZCLSID() == clsid);
        ser->Release();
    }
    void* o = reinterpret_cast<void*>(0x1234);
    CHECK(!p->QueryInterface(otherIid, &o) && o == reinterpret_cast<void*>(0x1234));
    CHECK(!p->QueryInterface(0x12345678, &o) && o == reinterpret_cast<void*>(0x1234));
}

// Slots 3-9 with no ports, through the primary and the +4 sub-object.
template <class T>
void CheckNoPorts(T* p, uint32_t simulatorType) {
    CHECK(p->GetSimulatorType() == simulatorType);
    SC4Percentage pct;
    pct.percentage = -1.0f;
    CHECK(p->GetAvgEfficiency(&pct) == &pct);
    CHECK(pct.percentage == 1.0f);  // §4.3: 1.0 with no ports
    SC4Point<int32_t> pt(7, 8);
    CHECK(!p->GetLeastEfficientPort(pct, pt));
    CHECK(p->GetNumPorts(0x1F, 0x1F) == 0);
    CHECK(p->GetNumPorts(-1, -1) == 0);
    // The caller's vector as the game lays it out (begin, end, capacity end);
    // no fc4::GameVector here: its destructor needs the game's allocator.
    void* raw[3] = {nullptr, nullptr, nullptr};
    p->GetAllPortLots(*reinterpret_cast<fc4::GameVector<cISC4Lot*>*>(raw));
    CHECK(raw[0] == nullptr && raw[1] == nullptr);
    cFC4ISC4PortSubObject* sub = p->AsISC4PortDeveloper();
    CHECK(reinterpret_cast<uintptr_t>(sub) == reinterpret_cast<uintptr_t>(p) + 4);
    if (sub != nullptr) {
        CHECK(sub->GetSimulatorType() == simulatorType);
        CHECK(sub->GetNumPorts(0x1F, 0x1F) == 0);
        pct.percentage = -1.0f;
        CHECK(sub->GetAvgEfficiency(&pct) == &pct && pct.percentage == 1.0f);
    }
}

}  // namespace ports_test

int PortsEnableTest(const std::string& dll, const std::string& ini, const std::string& log) {
    using namespace cmglue_test;
    using namespace ports_test;
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, "[subsystems]\nenable = ports\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (hd == nullptr) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    SetGamePrecision();
    // §12 test 1: the empty records.
    const std::vector<uint8_t> airport = {0x02, 0, 0x01, 0, 0x9D, 0x0E, 0x27, 0xCA, 0x0B, 0, 0, 0};
    const std::vector<uint8_t> seaport = {0x02, 0, 0x01, 0, 0x92, 0xAD, 0x29, 0xEA, 0x0C, 0, 0, 0};
    // §7's example: one small airport at stage 1 on (40, 52); without the
    // tuning (no resource manager here) its type index is out of range and the
    // record is refused ([choice], §7).
    const std::vector<uint8_t> example = {0x02, 0, 0x01, 0, 0x9D, 0x0E, 0x27, 0xCA, 0x0B, 0, 0x01, 0,
                                          0x01, 0, 0, 0, 0, 0, 0x28, 0, 0x34, 0};
    if (director != nullptr) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(classes.size() == 6);  // simulator (1) + budget (3) + ports (2)
        CHECK(std::count(classes.begin(), classes.end(), kFC4_CLSID_cSC4AirportDeveloper) == 1);
        CHECK(std::count(classes.begin(), classes.end(), kFC4_CLSID_cSC4SeaportDeveloper) == 1);
        if (auto* a = Create<cFC4ISC4AirportDeveloper>(director, kFC4_CLSID_cSC4AirportDeveloper,
                                                       kFC4_IID_cISC4AirportDeveloper)) {
            CheckPortQi(a, kFC4_IID_cISC4AirportDeveloper, kFC4_IID_cISC4SeaportDeveloper,
                        kFC4_CLSID_cSC4AirportDeveloper);
            bool ok = false;
            CHECK(RecordOf(a, &ok) == airport && ok);
            CheckNoPorts(a, 0xEA3955C4);
            CHECK(a->GetNumAirports(0) == 0);
            CHECK(a->GetNumAirports(1) == 0);
            CHECK(a->GetNumAirportTrips() == 0);
            uint32_t u = 9, c = 9;
            CHECK(!a->GetAirportUsageValues(nullptr, u, c) && u == 0 && c == 0);
            CHECK(a->GetMaintenanceCost(nullptr) == 0);
            CHECK(a->Init());  // no city: no department, no agent; true
            if (cIGZMessageTarget2* mt = TargetOf(a)) {
                ResidentialDayMessage month(0x66956816, kJulian20000201);
                CHECK(mt->DoMessage(&month));
                ResidentialDayMessage inserted(0x99EF1142, 0);  // void1 null
                CHECK(mt->DoMessage(&inserted));
                ResidentialDayMessage grow(0xEA394B74, 0);
                CHECK(mt->DoMessage(&grow));
                ResidentialDayMessage other(0x12345678, 0);
                CHECK(mt->DoMessage(&other));
                mt->Release();
            }
            CHECK(RecordOf(a, &ok) == airport && ok);
            CHECK(a->Shutdown());
            CHECK(a->Release() == 0);
        }
        if (auto* s = Create<cFC4ISC4SeaportDeveloper>(director, kFC4_CLSID_cSC4SeaportDeveloper,
                                                       kFC4_IID_cISC4SeaportDeveloper)) {
            CheckPortQi(s, kFC4_IID_cISC4SeaportDeveloper, kFC4_IID_cISC4AirportDeveloper,
                        kFC4_CLSID_cSC4SeaportDeveloper);
            bool ok = false;
            CHECK(RecordOf(s, &ok) == seaport && ok);
            CheckNoPorts(s, 0xEA3957AD);
            CHECK(s->GetNumSeaports() == 0);
            CHECK(s->GetNumSeaportTrips() == 0);
            CHECK(!s->CityHasSea());
            CHECK(s->GetCellSeaAccess(0, 0) == 0);
            CHECK(s->GetCellSeaAccess(-5, 1000) == 0);
            CHECK(s->Init());  // no terrain: an empty map
            CHECK(!s->CityHasSea());
            if (cIGZMessageTarget2* mt = TargetOf(s)) {
                ResidentialDayMessage month(0x66956816, kJulian20000201);
                CHECK(mt->DoMessage(&month));
                ResidentialDayMessage terrain(0x6A3B0AFD, 0);
                CHECK(mt->DoMessage(&terrain));
                mt->Release();
            }
            CHECK(s->Shutdown());
            CHECK(s->Release() == 0);
        }
        std::vector<uint8_t> again;
        CHECK(ReadInto<cFC4ISC4AirportDeveloper>(director, kFC4_CLSID_cSC4AirportDeveloper,
                                                 kFC4_IID_cISC4AirportDeveloper, airport, &again));
        CHECK(again == airport);
        again.clear();
        CHECK(ReadInto<cFC4ISC4SeaportDeveloper>(director, kFC4_CLSID_cSC4SeaportDeveloper,
                                                 kFC4_IID_cISC4SeaportDeveloper, seaport, &again));
        CHECK(again == seaport);
        CHECK(!ReadInto<cFC4ISC4AirportDeveloper>(director, kFC4_CLSID_cSC4AirportDeveloper,
                                                  kFC4_IID_cISC4AirportDeveloper, example, nullptr));
        for (uint8_t v : {1, 3}) {  // §12 test 1: only subclass version 2
            std::vector<uint8_t> bad = airport;
            bad[0] = v;
            CHECK(!ReadInto<cFC4ISC4AirportDeveloper>(director, kFC4_CLSID_cSC4AirportDeveloper,
                                                      kFC4_IID_cISC4AirportDeveloper, bad, nullptr));
        }
        CheckCw("ports");
    }
    FreeLibrary(hd);
    DeleteFileA(ini.c_str());
    const std::string text = ReadFile(log);
    const auto npos = std::string::npos;
    CHECK(text.find("subsystem 'ports': enabled (2 classes replaced) by FreeCity4.ini (enable=)") != npos);
    CHECK(text.find("ports airport Init: ok=1 init=1 tuning=0xCA270E9D") != npos);
    CHECK(text.find("ports seaport Init: ok=1 init=1 tuning=0xEA29AD92") != npos);
    CHECK(text.find("ports airport month 2000-02-01: ports=0 ran=0") != npos);
    CHECK(text.find("ports seaport month 2000-02-01: ports=0 ran=0") != npos);
    CHECK(text.find("ports airport Write -> 1 (ports=0)") != npos);
    CHECK(CountOf(text, "ports airport Read: ok=1") == 1);
    CHECK(CountOf(text, "ports airport Read: ok=0") == 3);
    CHECK(CountOf(text, "ports seaport Read: ok=1") == 1);
    CHECK(text.find("structured exception") == npos);
    CHECK(text.find("config: WARNING unknown key") == npos);
    CHECK(text.find("WARNING: released last reference") == npos);
    CHECK(text.find("WARNING: over-release") == npos);
    CHECK(text.find("streams: WARNING") == npos);
    if (g_failures == 0) std::printf("plugin ports enable test: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}
