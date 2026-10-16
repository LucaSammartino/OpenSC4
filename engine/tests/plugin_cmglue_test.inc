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
    // One idle step per delivery below (the original's pacing; under the
    // default deterministic pacing one SimIdle runs the whole rebuild).
    SetEnvironmentVariableA("FC4_PACING", "original");
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

#include "cFC4Civic.h"

namespace cmglue_test {
constexpr uint32_t kCLSID_Civic = 0x0A413481;
constexpr uint32_t kIID_Civic = 0x8A41322F;

// Segment streams with strings (the civic record's tooltips, §7): GZStr as a
// uint32 length and the bytes; references as a uint32 0 (null only).
class TextSegOStream final : public cFC4ISC4DBSegmentOStream {
public:
    std::vector<uint8_t> bytes;
    bool QueryInterface(uint32_t riid, void** p) override {
        if (p == nullptr || (riid != kFC4_IID_cISC4DBSegmentOStream && riid != GZIID_cIGZUnknown)) return false;
        *p = static_cast<cFC4ISC4DBSegmentOStream*>(this);
        return true;
    }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    void Flush() override {}
    bool SetSint8(int8_t v) override { return Put(&v, 1); }
    bool SetUint8(uint8_t v) override { return Put(&v, 1); }
    bool SetSint16(int16_t v) override { return Put(&v, 2); }
    bool SetUint16(uint16_t v) override { return Put(&v, 2); }
    bool SetSint32(int32_t v) override { return Put(&v, 4); }
    bool SetUint32(uint32_t v) override { return Put(&v, 4); }
    bool SetSint64(int64_t v) override { return Put(&v, 8); }
    bool SetUint64(uint64_t v) override { return Put(&v, 8); }
    bool SetFloat32(float v) override { return Put(&v, 4); }
    bool SetFloat64(double v) override { return Put(&v, 8); }
    bool SetRZCharStr(char const* s) override {
        const uint32_t n = s ? static_cast<uint32_t>(std::strlen(s)) : 0u;
        return SetUint32(n) && Put(s, n);
    }
    bool SetGZStr(cIGZString const& s) override {
        const uint32_t n = s.Strlen();
        return SetUint32(n) && Put(s.Data(), n);
    }
    bool SetGZSerializable(cIGZSerializable const&) override { return false; }
    bool SetVoid(void const* p, uint32_t n) override { return Put(p, n); }
    int32_t GetError() override { return 0; }
    int32_t SetUserData(cIGZVariant*) override { return 0; }
    int32_t GetUserData() override { return 0; }
    bool Open(cISC4DBSegment*, cGZPersistResourceKey const&, bool) override { return false; }
    bool Close() override { return true; }
    bool IsOpen() override { return true; }
    int32_t GetRecord() override { return 0; }
    int32_t GetSegment() override { return 0; }
    bool WriteGZSerializable(cIGZSerializable const* p) override { return p == nullptr && SetUint32(0); }
    bool WriteResKey(cGZPersistResourceKey const&) override { return false; }
    bool WriteVariant(cIGZVariant const&) override { return false; }

private:
    bool Put(const void* p, size_t n) {
        if (n == 0) return true;
        const uint8_t* b = static_cast<const uint8_t*>(p);
        bytes.insert(bytes.end(), b, b + n);
        return true;
    }
};

class TextSegIStream final : public cFC4ISC4DBSegmentIStream {
public:
    explicit TextSegIStream(std::vector<uint8_t> b) : bytes(std::move(b)) {}
    std::vector<uint8_t> bytes;
    size_t at = 0;
    int32_t error = 0;
    bool QueryInterface(uint32_t riid, void** p) override {
        if (p == nullptr || (riid != kFC4_IID_cISC4DBSegmentIStream && riid != GZIID_cIGZUnknown)) return false;
        *p = static_cast<cFC4ISC4DBSegmentIStream*>(this);
        return true;
    }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    bool Skip(uint32_t n) override { return Get(nullptr, n); }
    bool GetSint8(int8_t& v) override { return Get(&v, 1); }
    bool GetUint8(uint8_t& v) override { return Get(&v, 1); }
    bool GetSint16(int16_t& v) override { return Get(&v, 2); }
    bool GetUint16(uint16_t& v) override { return Get(&v, 2); }
    bool GetSint32(int32_t& v) override { return Get(&v, 4); }
    bool GetUint32(uint32_t& v) override { return Get(&v, 4); }
    bool GetSint64(int64_t& v) override { return Get(&v, 8); }
    bool GetUint64(uint64_t& v) override { return Get(&v, 8); }
    bool GetFloat32(float& v) override { return Get(&v, 4); }
    bool GetFloat64(double& v) override { return Get(&v, 8); }
    bool GetRZCharStr(char*, uint32_t) override { return Fail(); }
    bool GetGZStr(cIGZString& s) override {
        uint32_t n = 0;
        if (!GetUint32(n) || bytes.size() - at < n) return Fail();
        const std::string t(reinterpret_cast<const char*>(bytes.data() + at), n);
        at += n;
        s.FromChar(t.c_str(), n);
        return true;
    }
    bool GetGZSerializable(cIGZSerializable&) override { return Fail(); }
    bool GetVoid(void* p, uint32_t n) override { return Get(p, n); }
    int32_t GetError() override { return error; }
    int32_t SetUserData(cIGZVariant*) override { return 0; }
    int32_t GetUserData() override { return 0; }
    bool Open(cISC4DBSegment*, cGZPersistResourceKey const&, bool) override { return false; }
    bool Close() override { return true; }
    bool IsOpen() override { return true; }
    int32_t GetRecord() override { return 0; }
    int32_t GetSegment() override { return 0; }
    bool ReadGZSerializable(cIGZSerializable** out, bool) override {
        uint32_t id = 1;
        if (out == nullptr || !GetUint32(id) || id != 0) return Fail();
        *out = nullptr;
        return true;
    }
    bool ReadResKey(cGZPersistResourceKey&) override { return Fail(); }
    bool ReadVariant(cIGZVariant&) override { return Fail(); }

private:
    bool Fail() {
        error = 1;
        return false;
    }
    bool Get(void* p, size_t n) {
        if (error != 0 || bytes.size() - at < n) return Fail();
        if (p != nullptr && n != 0) std::memcpy(p, bytes.data() + at, n);
        at += n;
        return true;
    }
};

template <class T>
std::vector<uint8_t> TextRecordOf(T* o, bool* ok) {
    std::vector<uint8_t> r;
    *ok = false;
    cIGZSerializable* s = SerOf(o);
    if (s == nullptr) return r;
    TextSegOStream out;
    *ok = s->Write(out);
    r = out.bytes;
    s->Release();
    return r;
}

template <class T>
bool TextReadInto(cIGZCOMDirector* director, uint32_t clsid, uint32_t iid, const std::vector<uint8_t>& bytes,
                  std::vector<uint8_t>* again) {
    T* o = Create<T>(director, clsid, iid);
    if (o == nullptr) return false;
    bool ok = false;
    if (cIGZSerializable* s = SerOf(o)) {
        TextSegIStream in(bytes);
        ok = s->Read(in);
        if (ok && again != nullptr) {
            TextSegOStream out;
            CHECK(s->Write(out));
            *again = out.bytes;
        }
        s->Release();
    }
    CHECK(o->Release() == 0);
    return ok;
}
}  // namespace cmglue_test

int CivicEnableTest(const std::string& dll, const std::string& ini, const std::string& log) {
    using namespace cmglue_test;
    // Off by default; `enable = civic` registers it. No city: the QI table
    // (§1), the slots (§8), the record (§7), DoMessage (§3). The test process
    // is not the game: the game's string allocator (compat table) is absent,
    // so slot 6 answers null while the core keeps every status (§4.7).
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, "[subsystems]\nenable = civic\ncivic_seed = 7\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (hd == nullptr) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    SetGamePrecision();
    // §7: version 6, no exclusion, no status, a null department, two empty lists.
    const std::vector<uint8_t> empty = {6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
    if (director != nullptr) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(classes.size() == 5);  // simulator (1) + budget (3) + civic (1)
        CHECK(std::count(classes.begin(), classes.end(), kCLSID_Civic) == 1);
        std::vector<uint8_t> record;
        if (auto* c = Create<cISC4CivicBuildingSimulator>(director, kCLSID_Civic, kIID_Civic)) {
            CheckQiTable(c, kIID_Civic, kCLSID_Civic);
            bool ok = false;
            CHECK(TextRecordOf(c, &ok) == empty && ok);
            // §4.1: no city, no budget: Init still answers true.
            CHECK(c->Init());
            CHECK(!c->IsBuildingExcluded(0x8A9AE664));
            CHECK(c->GetLandmarkCount() == 0);
            CHECK(c->GetConditionalBuildingStatus(0x032B0000) == nullptr);
            // Slot 7 with a caller's status in the game layout ({u8 status; u16
            // count; cRZString {vtable, begin, end, capacity end, refcount}}).
            struct GameStatus {
                uint8_t status, pad;
                uint16_t count;
                const void* vtable;
                char *begin, *end, *capacityEnd;
                uint32_t refs;
            };
            static_assert(sizeof(void*) != 4 || sizeof(GameStatus) == 0x18, "ConditionalBuildingStatus layout");
            char text[] = "x";
            GameStatus st{6, 0, 2, nullptr, text, text + 1, text + 2, 1};
            auto* gs = reinterpret_cast<cISC4CivicBuildingSimulator::ConditionalBuildingStatus*>(&st);
            CHECK(c->UpdateConditionalBuildingStatus(0x032B0000, gs));
            // Keys 194 and 1 share bucket 1 of 193 (§12 test 9): the newer first.
            st = GameStatus{1, 0, 0, nullptr, text, text, text + 2, 1};
            CHECK(c->UpdateConditionalBuildingStatus(1, gs));
            CHECK(c->UpdateConditionalBuildingStatus(194, gs));
            // No game allocator here: no mirror entry.
            CHECK(c->GetConditionalBuildingStatus(0x032B0000) == nullptr);
            record = TextRecordOf(c, &ok);
            CHECK(ok);
            // 2 + 4 + 4 + 3 statuses + 4 + 8.
            CHECK(record.size() == 2 + 4 + 4 + (4 + 1 + 2 + 4 + 1) + 2 * (4 + 1 + 2 + 4) + 4 + 4 + 4);
            // Hash order (§7): 194 and 1 in bucket 1 (194 newer, first), then 0x032B0000.
            if (record.size() >= 22) {
                uint32_t k1 = 0, k2 = 0;
                std::memcpy(&k1, &record[10], 4);
                std::memcpy(&k2, &record[10 + 11], 4);
                CHECK(k1 == 194 && k2 == 1);
            }
            if (cIGZMessageTarget2* mt = TargetOf(c)) {
                ResidentialDayMessage month(0x66956816, kJulian20000201);
                CHECK(mt->DoMessage(&month));
                ResidentialDayMessage other(0x12345678, 0);
                CHECK(mt->DoMessage(&other));
                mt->Release();
            }
            CHECK(c->Shutdown());
            CHECK(c->Release() == 0);
        }
        if (!record.empty()) {
            std::vector<uint8_t> again;
            CHECK(TextReadInto<cISC4CivicBuildingSimulator>(director, kCLSID_Civic, kIID_Civic, record, &again));
            // §7 / §12 test 9 (CB6, on main since the cores10b answers): a read
            // reverses each bucket chain of two or more entries, so only the
            // second round trip must give the record back; the bucket order and
            // the record's size hold after one. (The core still keeps the chain
            // order on the first round trip; question CIV-4.)
            CHECK(again.size() == record.size());
            std::vector<uint8_t> twice;
            CHECK(TextReadInto<cISC4CivicBuildingSimulator>(director, kCLSID_Civic, kIID_Civic, again, &twice));
            CHECK(twice == record);
            std::vector<uint8_t> bad = record;
            bad[0] = 5;  // §12 test 9: version 5 is refused
            CHECK(!TextReadInto<cISC4CivicBuildingSimulator>(director, kCLSID_Civic, kIID_Civic, bad, nullptr));
        }
        CheckCw("civic");
    }
    FreeLibrary(hd);
    DeleteFileA(ini.c_str());
    const std::string text = ReadFile(log);
    const auto npos = std::string::npos;
    CHECK(text.find("subsystem 'civic': enabled (1 class replaced) by FreeCity4.ini (enable=)") != npos);
    CHECK(text.find("session seed 7 (civic_seed)") != npos);
    CHECK(text.find("civic Init: ok=1 init=1") != npos);
    CHECK(text.find("civic: Lua civic.get_landmark_count() NOT published") != npos);
    CHECK(text.find("civic: the game's string allocator is not available") != npos);
    CHECK(text.find("civic month 2000-02-01") != npos);
    CHECK(CountOf(text, "civic Read: ok=1") == 2);
    CHECK(CountOf(text, "civic Read: ok=0") == 1);
    CHECK(text.find("structured exception") == npos);
    CHECK(text.find("config: WARNING unknown key") == npos);
    CHECK(text.find("WARNING: released last reference") == npos);
    CHECK(text.find("WARNING: over-release") == npos);
    CHECK(text.find("streams: WARNING") == npos);
    if (g_failures == 0) std::printf("plugin civic enable test: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}

// ---- the park manager (parks.md) -------------------------------------------------------------------

#include "cFC4Parks.h"
#include "cIGZMessageServer2.h"

namespace cmglue_test {
constexpr uint32_t kCLSID_Parks = 0xAA3FE6BC;
constexpr uint32_t kIID_Parks = 0xAA3FE6BC;  // §1: the CLSID's value

// A message server that records the posts (seam op 57).
class PostRecorder final : public cIGZMessageServer2 {
public:
    uint32_t posts = 0, type = 0;
    void* void1 = nullptr;
    cIGZUnknown* unknown = nullptr;
    bool highPriority = true;
    bool standard = false;
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    bool MessageSend(cIGZMessage2*) override { return false; }
    bool MessagePost(cIGZMessage2* m, bool high) override {
        ++posts;
        highPriority = high;
        if (m == nullptr) return false;
        type = m->GetType();
        cIGZMessage2Standard* s = nullptr;
        if (m->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&s)) && s != nullptr) {
            standard = true;
            void1 = s->GetVoid1();
            unknown = s->GetIGZUnknown();
            s->Release();
        }
        return true;
    }
    bool AddNotification(cIGZMessageTarget2*, uint32_t) override { return false; }
    bool RemoveNotification(cIGZMessageTarget2*, uint32_t) override { return false; }
    bool GeneralMessagePostToTarget(cIGZMessage2*, cIGZMessageTarget2*) override { return false; }
    bool CancelGeneralMessagePostsToTarget(cIGZMessageTarget2*) override { return false; }
    bool OnTick(uint32_t) override { return false; }
    uint32_t GetMessageQueueSize() override { return 0; }
    cIGZMessageServer2* SetAlwaysClearQueueOnTick(bool) override { return this; }
    uint32_t GetRefCount() override { return 1; }
    cIGZMessage2* CreateMessage(uint32_t, uint32_t, void**) override { return nullptr; }
};

// A zoo occupant stand-in (only its identity and reference count are used).
class FakeZoo final : public cIGZUnknown {
public:
    uint32_t refs = 1;
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return refs > 0 ? --refs : 0u; }
};
}  // namespace cmglue_test

int ParksEnableTest(const std::string& dll, const std::string& ini, const std::string& log) {
    using namespace cmglue_test;
    // Off by default; `enable = parks` registers it. No city: the QI table
    // (§1), Init / Shutdown (§8), DoMessage (§3), the record (§7) and, through
    // seam op 57, the ZooEscape post (§4.5).
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, "[subsystems]\nenable = parks\nparks_seed = 9\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (hd == nullptr) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    auto seam = reinterpret_cast<FC4TestSeamFn>(GetProcAddress(hd, "FC4TestSeam"));
    CHECK(seam != nullptr);
    SetGamePrecision();
    // §7: version 2, two empty lists (the corpus's 413 records).
    const std::vector<uint8_t> empty = {2, 0, 0, 0, 0, 0, 0, 0, 0, 0};
    if (director != nullptr) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(classes.size() == 5);  // simulator (1) + budget (3) + parks (1)
        CHECK(std::count(classes.begin(), classes.end(), kCLSID_Parks) == 1);
        if (auto* p = Create<cISC4ParkManager>(director, kCLSID_Parks, kIID_Parks)) {
            CheckQiTable(p, kIID_Parks, kCLSID_Parks);
            bool ok = false;
            CHECK(RecordOf(p, &ok) == empty && ok);
            CHECK(p->Init());  // §4.1: no city, no budget: true
            CHECK(p->Init());  // already initialised: true
            if (cIGZMessageTarget2* mt = TargetOf(p)) {
                ResidentialDayMessage month(0x66956816, kJulian20000201);
                CHECK(mt->DoMessage(&month));
                ResidentialDayMessage inserted(0x99EF1142, 0);  // void1 null
                CHECK(mt->DoMessage(&inserted));
                ResidentialDayMessage other(0x12345678, 0);
                CHECK(mt->DoMessage(&other));
                mt->Release();
            }
            CHECK(RecordOf(p, &ok) == empty && ok);
            // §4.5 step 2 through the host (seam op 57).
            if (seam != nullptr) {
                PostRecorder server;
                FakeZoo zoo;
                FC4ParksTestInput in{&server, p, &zoo};
                CHECK(seam(kFC4SeamParks, &in, nullptr) == 1);
                CHECK(server.posts == 1);
                CHECK(server.type == 0xAA667CE2);
                CHECK(server.standard);
                CHECK(server.void1 == static_cast<void*>(p));
                CHECK(server.unknown == static_cast<cIGZUnknown*>(&zoo));
                CHECK(!server.highPriority);
                CHECK(zoo.refs == 1);  // the message's reference is gone with it
                CHECK(seam(kFC4SeamParks, nullptr, nullptr) == -1);
            }
            CHECK(p->Shutdown());
            CHECK(p->Release() == 0);
        }
        std::vector<uint8_t> again;
        CHECK(ReadInto<cISC4ParkManager>(director, kCLSID_Parks, kIID_Parks, empty, &again));
        CHECK(again == empty);
        for (uint8_t v : {1, 3}) {  // §12 test 8: only version 2
            std::vector<uint8_t> bad = empty;
            bad[0] = v;
            CHECK(!ReadInto<cISC4ParkManager>(director, kCLSID_Parks, kIID_Parks, bad, nullptr));
        }
        CheckCw("parks");
    }
    FreeLibrary(hd);
    DeleteFileA(ini.c_str());
    const std::string text = ReadFile(log);
    const auto npos = std::string::npos;
    CHECK(text.find("subsystem 'parks': enabled (1 class replaced) by FreeCity4.ini (enable=)") != npos);
    CHECK(text.find("session seed 9 (parks_seed)") != npos);
    CHECK(text.find("parks Init: ok=1 init=1 parks=0 zoos=0") != npos);
    CHECK(text.find("parks month 2000-02-01: parks=0 zoos=0") != npos);
    CHECK(text.find("parks: zoo escape at") != npos);
    CHECK(CountOf(text, "parks Read: ok=1") == 1);
    CHECK(CountOf(text, "parks Read: ok=0") == 2);
    CHECK(text.find("structured exception") == npos);
    CHECK(text.find("config: WARNING unknown key") == npos);
    CHECK(text.find("WARNING: released last reference") == npos);
    CHECK(text.find("WARNING: over-release") == npos);
    CHECK(text.find("streams: WARNING") == npos);
    if (g_failures == 0) std::printf("plugin parks enable test: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}
