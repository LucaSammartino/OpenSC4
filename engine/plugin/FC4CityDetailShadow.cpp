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

#include "FC4CityDetailShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4HistClock.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4WriteCapture.h"
#include "cFC4CityDetail.h"

#include "GZServPtrs.h"
#include "cIGZCOM.h"
#include "cIGZSerializable.h"
#include "cISC4City.h"
#include "cISC4Simulator.h"
#include "cRZCOMDllDirector.h"

#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kIID_cIGZSerializable = 0xE4FDA3D4;
constexpr uint32_t kNone = 0xFFFFFFFFu;
constexpr size_t kMaxListed = 12;
// [choice] The shadow core's session seed (its dry runs only).
constexpr uint32_t kShadowSeed = 0x5EED0EADu;

bool g_enabled = false;
bool g_replaced = false;
uint32_t g_loads = 0, g_cleanLoads = 0;

// A record in memory as the game's segment stream (the original's Read).
class MemGameIStream final : public cFC4ISC4DBSegmentIStream {
public:
    explicit MemGameIStream(const std::vector<uint8_t>& b) : b_(b) {}
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid != GZIID_cIGZUnknown && riid != kFC4_IID_cISC4DBSegmentIStream) return false;
        *ppvObj = static_cast<cFC4ISC4DBSegmentIStream*>(this);
        return true;  // a stack object: no count
    }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    bool Skip(uint32_t n) override { return Take(nullptr, n); }
    bool GetSint8(int8_t& v) override { return Take(&v, 1); }
    bool GetUint8(uint8_t& v) override { return Take(&v, 1); }
    bool GetSint16(int16_t& v) override { return Take(&v, 2); }
    bool GetUint16(uint16_t& v) override { return Take(&v, 2); }
    bool GetSint32(int32_t& v) override { return Take(&v, 4); }
    bool GetUint32(uint32_t& v) override { return Take(&v, 4); }
    bool GetSint64(int64_t& v) override { return Take(&v, 8); }
    bool GetUint64(uint64_t& v) override { return Take(&v, 8); }
    bool GetFloat32(float& v) override { return Take(&v, 4); }
    bool GetFloat64(double& v) override { return Take(&v, 8); }
    bool GetRZCharStr(char*, uint32_t) override { return Fail(); }
    bool GetGZStr(cIGZString&) override { return Fail(); }
    bool GetGZSerializable(cIGZSerializable&) override { return Fail(); }
    bool GetVoid(void* p, uint32_t n) override { return Take(p, n); }
    int32_t GetError() override { return bad_ ? 1 : 0; }
    int32_t SetUserData(cIGZVariant*) override { return 0; }
    int32_t GetUserData() override { return 0; }
    bool Open(cISC4DBSegment*, cGZPersistResourceKey const&, bool) override { return false; }
    bool Close() override { return true; }
    bool IsOpen() override { return true; }
    int32_t GetRecord() override { return 0; }
    int32_t GetSegment() override { return 0; }
    bool ReadGZSerializable(cIGZSerializable** out, bool) override {
        if (out != nullptr) *out = nullptr;
        return Fail();
    }
    bool ReadResKey(cGZPersistResourceKey&) override { return Fail(); }
    bool ReadVariant(cIGZVariant&) override { return Fail(); }

private:
    bool Fail() {
        bad_ = true;
        return false;
    }
    bool Take(void* out, size_t n) {
        if (bad_ || n > b_.size() - pos_) return Fail();
        if (out != nullptr && n > 0) std::memcpy(out, b_.data() + pos_, n);
        pos_ += n;
        return true;
    }
    const std::vector<uint8_t>& b_;
    size_t pos_ = 0;
    bool bad_ = false;
};

void ReadSettings() { g_enabled = SettingInt("citydetail_shadow", "FC4_CITYDETAIL_SHADOW", 0) != 0; }

std::vector<uint8_t> Record(uint32_t version, uint32_t level) {
    std::vector<uint8_t> b(8);
    std::memcpy(b.data(), &version, 4);
    std::memcpy(b.data() + 4, &level, 4);
    return b;
}

std::string Hex(const std::vector<uint8_t>& b) {
    std::string s;
    char x[4];
    for (uint8_t v : b) {
        snprintf(x, sizeof(x), "%02X", v);
        s += x;
    }
    return s.empty() ? "-" : s;
}

struct Diff {
    uint32_t mismatches = 0;
    std::string listed;
    size_t n = 0;
    void Add(const char* fmt, ...) {
        ++mismatches;
        if (n++ >= kMaxListed) return;
        char b[200];
        va_list ap;
        va_start(ap, fmt);
        vsnprintf(b, sizeof(b), fmt, ap);
        va_end(ap);
        if (!listed.empty()) listed += ", ";
        listed += b;
    }
};

// The two objects of one case.
struct Pair {
    cFC4ISC4CityDetailManager* orig = nullptr;  // the original's interface (+0), one reference
    cIGZSerializable* origSer = nullptr;        // borrowed from orig
    CityDetailContext ctx;
    fc4_city_detail_host host{};
    fc4_city_detail* ours = nullptr;
};

bool MakeOriginal(Pair& p) {
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* obj = nullptr;
    if (com == nullptr || !com->GetClassObject(kFC4_CLSID_cSC4CityDetailManager, kFC4_IID_cISC4CityDetailManager, &obj) ||
        obj == nullptr) {
        return false;
    }
    cIGZUnknown* u = static_cast<cIGZUnknown*>(obj);
    if (IsOurObject(u)) {
        u->Release();
        return false;
    }
    FC4_ACQUIRED(u, "GetClassObject");
    p.orig = static_cast<cFC4ISC4CityDetailManager*>(obj);
    void* s = nullptr;
    if (!u->QueryInterface(kIID_cIGZSerializable, &s) || s == nullptr) {
        FC4_RELEASE_CREATED(u);
        p.orig = nullptr;
        return false;
    }
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(s), "QueryInterface");
    p.origSer = static_cast<cIGZSerializable*>(s);
    FC4_RELEASE(static_cast<cIGZUnknown*>(s));  // borrowed from orig
    return true;
}

bool MakeOurs(Pair& p) {
    p.ctx.readOnly = true;
    p.host = MakeCityDetailHost(&p.ctx);
    p.ours = fc4_city_detail_create(&p.host, kShadowSeed, 0);
    return p.ours != nullptr;
}

void Free(Pair& p) {
    if (p.orig != nullptr) {
        p.orig->Shutdown();
        FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(p.orig));  // GetClassObject's reference: the object goes
    }
    p.orig = nullptr;
    p.origSer = nullptr;
    if (p.ours != nullptr) {
        fc4_city_detail_shutdown(p.ours);
        fc4_city_detail_destroy(p.ours);
    }
    p.ours = nullptr;
    p.ctx.pool.Drain();
}

std::vector<uint8_t> OrigWrite(Pair& p, bool* ok) {
    WriteCapture cap;
    *ok = cap.Capture(static_cast<cIGZUnknown*>(p.orig));
    return cap.Bytes();
}

std::vector<uint8_t> OurWrite(Pair& p, bool* ok) {
    MemOStream out;
    *ok = fc4_city_detail_write(p.ours, out.Get());
    return out.bytes;
}

// One case: an optional record for both, then (when both read it, or with no
// record) Init on both; the results compared.
std::string Case(const char* name, const std::vector<uint8_t>* record, Diff& d) {
    Pair p;
    if (!MakeOriginal(p) || !MakeOurs(p)) {
        Free(p);
        d.Add("%s: the objects could not be made", name);
        return std::string(name) + "=?";
    }
    bool origRead = true, ourRead = true;
    if (record != nullptr) {
        MemGameIStream gin(*record);
        origRead = p.origSer->Read(gin);
        MemIStream min(*record);
        ourRead = fc4_city_detail_read(p.ours, min.Get());
        if (origRead != ourRead) d.Add("%s Read %d/%d", name, origRead ? 1 : 0, ourRead ? 1 : 0);
    }
    std::string out = std::string(name) + "=";
    if (origRead && ourRead) {
        p.orig->Init();
        fc4_city_detail_init(p.ours);
        p.ctx.pool.Drain();
        const uint32_t lo = p.orig->GetCityDetail();  // slot 5
        const uint32_t lu = fc4_city_detail_get_level(p.ours);
        if (lo != lu) d.Add("%s level %d/%d", name, static_cast<int>(lo), static_cast<int>(lu));
        bool wo = false, wu = false;
        const std::vector<uint8_t> bo = OrigWrite(p, &wo), bu = OurWrite(p, &wu);
        if (wo != wu || bo != bu) d.Add("%s record %s/%s", name, Hex(bo).c_str(), Hex(bu).c_str());
        fc4_city_detail_state st{};
        st.struct_size = sizeof(st);
        fc4_city_detail_get_state(p.ours, &st);
        if (st.last_pass != 0) d.Add("%s ours ran pass %u", name, st.last_pass);
        out += std::to_string(static_cast<int>(lo)) + "/" + std::to_string(static_cast<int>(lu)) + " record " + Hex(bo);
    } else {
        out += std::string("read ") + (origRead ? "1" : "0") + "/" + (ourRead ? "1" : "0");
    }
    Free(p);
    return out;
}

// Ours alone on a level the original must not see (it would run a pass).
std::string DryRun(uint32_t level) {
    Pair p;
    if (!MakeOurs(p)) return "?";
    const std::vector<uint8_t> rec = Record(0, level);
    MemIStream min(rec);
    std::string s = "level " + std::to_string(level) + " -> ";
    if (!fc4_city_detail_read(p.ours, min.Get())) {
        s += "read refused";
    } else {
        fc4_city_detail_init(p.ours);
        fc4_city_detail_state st{};
        st.struct_size = sizeof(st);
        fc4_city_detail_get_state(p.ours, &st);
        static const char* const kPass[] = {"none", "reduce", "increase"};
        char b[200];
        snprintf(b, sizeof(b), "%s removed=%u created=%u draws=%u network-lots=%u", st.last_pass < 3 ? kPass[st.last_pass] : "?",
                 st.last_removed, st.last_created, st.last_draws, st.last_network_lots);
        s += b;
    }
    Free(p);
    return s;
}

struct Run {
    cISC4City* city;
    std::string line;
};

void RunFn(void* ctx) {
    Run& r = *static_cast<Run*>(ctx);
    const uint32_t setting = CityDetailSetting();
    Diff d;
    const std::vector<uint8_t> same = Record(0, setting), none = Record(0, kNone), v1 = Record(1, setting);
    std::string cases = Case("fresh", nullptr, d);
    cases += " " + Case("read-same", &same, d);
    cases += " " + Case("read-none", &none, d);
    cases += " " + Case("read-v1", &v1, d);
    std::string dry;
    for (uint32_t level = 1; level <= 3; ++level) {
        if (level == setting) continue;
        if (!dry.empty()) dry += ", ";
        dry += DryRun(level);
    }
    int32_t julian = 0;
    cISC4Simulator* sim = r.city ? r.city->GetSimulator() : nullptr;
    if (sim != nullptr) julian = AsGame(sim)->GetSimDateNumber();  // slot 8
    char date[16];
    ++g_loads;
    if (d.mismatches == 0) ++g_cleanLoads;
    r.line = std::string("citydetail shadow load ") + propcity::DateText(julian, date) +
             ": setting=" + std::to_string(setting) + " " + cases + "; dry runs (ours only, not compared): " +
             (dry.empty() ? std::string("-") : dry) + "; mismatches=" + std::to_string(d.mismatches) +
             (d.listed.empty() ? std::string() : "; differences: " + d.listed);
}

}  // namespace

void CityDetailShadowStartup(bool cityDetailReplaced) {
    g_replaced = cityDetailReplaced;
    ReadSettings();
    if (!g_enabled) return;
    const char* src = "";
    Setting("citydetail_shadow", "FC4_CITYDETAIL_SHADOW", &src);
    if (cityDetailReplaced) {
        Log("citydetail shadow: requested (citydetail_shadow from %s) but INACTIVE: it compares against the ORIGINAL "
            "class, and citydetail is enabled (ours)",
            src);
        return;
    }
    Log("citydetail shadow: ON (citydetail_shadow from %s): at each city load a fresh ORIGINAL city detail manager and "
        "a fresh shadow core (read-only host) get the same inputs (fresh Init; records 0/setting, 0/none and "
        "version 1 then Init); Read's result, the level (slot 5) and the record are compared. Not compared: the "
        "reduce and increase passes (they would change the city; ours alone dry-runs them)",
        src);
}

void CityDetailShadowCityLoaded(cISC4City* city) {
    if (!g_enabled || g_replaced) return;
    Run r{city, std::string()};
    if (!SehGuard("citydetail shadow (load)", &RunFn, &r)) {
        Log("citydetail shadow load: INCOMPLETE (a fault); not compared (the objects of the faulted case leak)");
        return;
    }
    LogShadowLine("citydetail shadow (cont.)", reinterpret_cast<const uint8_t*>(r.line.data()), r.line.size());
    Log("citydetail shadow: %u load lines so far, %u with mismatches=0", g_loads, g_cleanLoads);
}

}  // namespace fc4
