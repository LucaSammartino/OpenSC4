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

// The cmglue lane's plugin tests (included by plugin_load_test.cpp, which
// provides CHECK, the segment stream fakes and the helpers): the commercial
// and industrial simulators (commercial-industrial.md), the civic building
// simulator (civic-buildings.md) and the park manager (parks.md), without a
// city: the QI tables (§1), the slots' return channels, the records' round
// trips, and (through seam op 57) the pieces that need a stub host.

#include "cFC4Commercial.h"

namespace cmglue_test {

constexpr uint32_t kCLSID_Commercial = 0x8990C080;
constexpr uint32_t kIID_Commercial = 0x077ABEB2;
constexpr uint32_t kCLSID_Industrial = 0xA990C022;
constexpr uint32_t kIID_Industrial = 0x477A9E7D;
constexpr uint32_t kIID_MessageTarget2 = 0x452294AA;
constexpr uint32_t kCmMsgSimIdle = 0x6695681A;
constexpr uint32_t kCmMsgSimNewDay = 0x66956814;

// §1 for one object: the primary for 1 and `iid`, the message target (+4),
// the serializable (+8) whose GetGZCLSID is `clsid`; anything else false with
// the output untouched. Each successful query AddRefs.
void CheckQiTable(cIGZUnknown* primary, uint32_t iid, uint32_t clsid) {
    void* p = nullptr;
    CHECK(primary->QueryInterface(GZIID_cIGZUnknown, &p) && p == static_cast<void*>(primary));
    if (p != nullptr) static_cast<cIGZUnknown*>(p)->Release();
    p = nullptr;
    CHECK(primary->QueryInterface(iid, &p) && p == static_cast<void*>(primary));
    if (p != nullptr) static_cast<cIGZUnknown*>(p)->Release();
    cIGZMessageTarget2* mt = nullptr;
    CHECK(primary->QueryInterface(kIID_MessageTarget2, reinterpret_cast<void**>(&mt)) && mt != nullptr);
    CHECK(reinterpret_cast<uintptr_t>(mt) == reinterpret_cast<uintptr_t>(primary) + 4);
    if (mt != nullptr) mt->Release();
    cIGZSerializable* ser = nullptr;
    CHECK(primary->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&ser)) && ser != nullptr);
    CHECK(reinterpret_cast<uintptr_t>(ser) == reinterpret_cast<uintptr_t>(primary) + 8);
    if (ser != nullptr) {
        CHECK(ser->GetGZCLSID() == clsid);
        ser->Release();
    }
    p = reinterpret_cast<void*>(0x1234);
    CHECK(!primary->QueryInterface(0x12345678, &p) && p == reinterpret_cast<void*>(0x1234));
}

template <class T>
T* Create(cIGZCOMDirector* director, uint32_t clsid, uint32_t iid) {
    void* obj = nullptr;
    CHECK(director->GetClassObject(clsid, iid, &obj));
    return static_cast<T*>(obj);
}

template <class T>
cIGZSerializable* SerOf(T* o) {
    cIGZSerializable* s = nullptr;
    if (o == nullptr || !o->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&s))) return nullptr;
    return s;
}

template <class T>
cIGZMessageTarget2* TargetOf(T* o) {
    cIGZMessageTarget2* s = nullptr;
    if (o == nullptr || !o->QueryInterface(kIID_MessageTarget2, reinterpret_cast<void**>(&s))) return nullptr;
    return s;
}

// The record of `o` (its own Write into the null-reference stream).
template <class T>
std::vector<uint8_t> RecordOf(T* o, bool* ok) {
    std::vector<uint8_t> r;
    *ok = false;
    cIGZSerializable* s = SerOf(o);
    if (s == nullptr) return r;
    ResidentialSegOStream out;
    *ok = s->Write(out);
    r = out.bytes;
    s->Release();
    return r;
}

// A fresh object reads `bytes` (Read runs before Init); on success its own
// Write gives `again`.
template <class T>
bool ReadInto(cIGZCOMDirector* director, uint32_t clsid, uint32_t iid, const std::vector<uint8_t>& bytes,
              std::vector<uint8_t>* again) {
    T* o = Create<T>(director, clsid, iid);
    if (o == nullptr) return false;
    bool ok = false;
    if (cIGZSerializable* s = SerOf(o)) {
        ResidentialSegIStream in(bytes);
        ok = s->Read(in);
        if (ok && again != nullptr) {
            ResidentialSegOStream out;
            CHECK(s->Write(out));
            *again = out.bytes;
        }
        s->Release();
    }
    CHECK(o->Release() == 0);  // the final Release shuts a never-initialised one down
    return ok;
}

}  // namespace cmglue_test

int CommercialEnableTest(const std::string& dll, const std::string& ini, const std::string& log) {
    using namespace cmglue_test;
    // Off by default; `enable = commercial, industrial` registers both. No
    // city: the QI tables (§1), slot 5 (§4.6), the records (§8), Init
    // without services (§4.1 step 5 fails), DoMessage (§3).
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, "[subsystems]\nenable = commercial, industrial\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (hd == nullptr) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    SetGamePrecision();
    if (director != nullptr) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(classes.size() == 6);  // simulator (1) + budget (3) + commercial (1) + industrial (1)
        CHECK(std::count(classes.begin(), classes.end(), kCLSID_Commercial) == 1);
        CHECK(std::count(classes.begin(), classes.end(), kCLSID_Industrial) == 1);

        // ---- commercial ----
        std::vector<uint8_t> record;
        if (auto* c = Create<cISC4CommercialSimulator>(director, kCLSID_Commercial, kIID_Commercial)) {
            CheckQiTable(c, kIID_Commercial, kCLSID_Commercial);
            // §4.6: no grids before Init; other types null.
            using DT = cISC4BuildingDevelopmentSimulator::DeveloperType;
            CHECK(c->GetProximityMap(static_cast<DT>(0x3320)) == nullptr);
            CHECK(c->GetProximityMap(static_cast<DT>(0x3330)) == nullptr);
            CHECK(c->GetProximityMap(static_cast<DT>(0x3110)) == nullptr);
            // §8.1: version 1, flag 1 (dirty = the constructor's 1), two null references.
            bool ok = false;
            record = RecordOf(c, &ok);
            CHECK(ok);
            const std::vector<uint8_t> expect = {1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0};
            CHECK(record == expect);
            // §4.1 step 5: no city, no demand simulator: Init fails.
            CHECK(!c->Init());
            // §3: DoMessage is true for every message; one idle step (dirty: state 3 -> 0).
            if (cIGZMessageTarget2* mt = TargetOf(c)) {
                ResidentialDayMessage idle(kCmMsgSimIdle, 0);
                CHECK(mt->DoMessage(&idle));
                ResidentialDayMessage day(kCmMsgSimNewDay, kJulian20000201);
                CHECK(mt->DoMessage(&day));
                ResidentialDayMessage other(0x12345678, 0);
                CHECK(mt->DoMessage(&other));
                mt->Release();
            }
            // After the two steps the record's flag is still 1 (state 2, §8.1).
            std::vector<uint8_t> after = RecordOf(c, &ok);
            CHECK(ok && after.size() == 11 && after[2] == 1);
            CHECK(c->Shutdown());
            CHECK(c->Release() == 0);
        }
        // §8.1: the record reads into a new object and writes back the same;
        // version 2 and a truncated record are refused.
        if (!record.empty()) {
            std::vector<uint8_t> again;
            CHECK(ReadInto<cISC4CommercialSimulator>(director, kCLSID_Commercial, kIID_Commercial, record, &again));
            CHECK(again == record);
            std::vector<uint8_t> bad = record;
            bad[0] = 2;
            CHECK(!ReadInto<cISC4CommercialSimulator>(director, kCLSID_Commercial, kIID_Commercial, bad, nullptr));
            bad = record;
            bad.resize(bad.size() - 2);
            CHECK(!ReadInto<cISC4CommercialSimulator>(director, kCLSID_Commercial, kIID_Commercial, bad, nullptr));
            // flag 0 reads back as 0.
            bad = record;
            bad[2] = 0;
            again.clear();
            CHECK(ReadInto<cISC4CommercialSimulator>(director, kCLSID_Commercial, kIID_Commercial, bad, &again));
            CHECK(again == bad);
        }

        // ---- industrial ----
        if (auto* in = Create<cISC4IndustrialSimulator>(director, kCLSID_Industrial, kIID_Industrial)) {
            CheckQiTable(in, kIID_Industrial, kCLSID_Industrial);
            bool ok = false;
            const std::vector<uint8_t> r = RecordOf(in, &ok);
            CHECK(ok && r == std::vector<uint8_t>({1, 0}));  // §8.2
            CHECK(in->Init());
            CHECK(in->Init());
            if (cIGZMessageTarget2* mt = TargetOf(in)) {
                ResidentialDayMessage other(0x12345678, 0);
                CHECK(mt->DoMessage(&other));
                mt->Release();
            }
            CHECK(in->Shutdown());
            CHECK(in->Release() == 0);
            std::vector<uint8_t> again;
            CHECK(ReadInto<cISC4IndustrialSimulator>(director, kCLSID_Industrial, kIID_Industrial, {1, 0}, &again));
            CHECK(again == std::vector<uint8_t>({1, 0}));
            CHECK(!ReadInto<cISC4IndustrialSimulator>(director, kCLSID_Industrial, kIID_Industrial, {2, 0}, nullptr));
        }
        CheckCw("commercial");
    }
    FreeLibrary(hd);
    DeleteFileA(ini.c_str());
    const std::string text = ReadFile(log);
    const auto npos = std::string::npos;
    CHECK(text.find("subsystem 'commercial': enabled (1 class replaced) by FreeCity4.ini (enable=)") != npos);
    CHECK(text.find("subsystem 'industrial': enabled (1 class replaced) by FreeCity4.ini (enable=)") != npos);
    CHECK(CountOf(text, "cFC4CommercialSimulator(") == 5);
    CHECK(CountOf(text, "cFC4CommercialSimulator destroyed") == 5);
    CHECK(text.find("commercial Init FAILED: ok=0 init=1") != npos);
    CHECK(text.find("commercial: no simulator; agent at level 0 not registered") != npos);
    CHECK(CountOf(text, "commercial Read: ok=1") == 2);
    CHECK(CountOf(text, "commercial Read: ok=0") == 2);
    CHECK(text.find("commercial Write -> 1") != npos);
    CHECK(text.find("industrial Init: initialized=1") != npos);
    CHECK(CountOf(text, "industrial Read -> 1") == 1);
    CHECK(CountOf(text, "industrial Read -> 0") == 1);
    CHECK(text.find("structured exception") == npos);
    CHECK(text.find("config: WARNING unknown key") == npos);
    CHECK(text.find("WARNING: released last reference") == npos);
    CHECK(text.find("WARNING: over-release") == npos);
    CHECK(text.find("streams: WARNING") == npos);
    if (g_failures == 0) std::printf("plugin commercial enable test: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}
