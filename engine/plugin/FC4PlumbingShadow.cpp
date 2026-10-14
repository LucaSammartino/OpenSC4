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

#include "FC4PlumbingShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"
#include "cFC4Plumbing.h"

#include "SC4Percentage.h"
#include "cGZPersistResourceKey.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cIGZString.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4NeighborConnection.h"
#include "cISC4Occupant.h"
#include "cISC4PlumbingSimulator.h"
#include "cRZCOMDllDirector.h"

#include <algorithm>
#include <climits>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgSimIdle = 0x6695681A;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
// The interfaces the §9 reader asks a reference for (fc4_plumbing_capture_ref).
constexpr uint32_t kIID_cISC4SimGrid = 0xC9BD939A;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cSC4PipeOccupant = 0x49C05B9F;  // only a pipe answers it (docs/abi/direct-calls.md)
constexpr uint32_t kIID_cISC4DepartmentBudget = 0x223E2B67;
constexpr const char* kAgentName = "cSC4PlumbingSimulator";  // plumbing.md §4.1 step 3
constexpr long kDefaultCells = 8;
constexpr long kMaxCells = 64;
constexpr long kDefaultPasses = 4;
// fc4::Log formats into a fixed buffer: longer shadow lines are split.
constexpr size_t kLogChunk = 2800;

// ---- the capture stream ------------------------------------------------------------------------------
//
// The original's Write goes into memory: primitives little-endian (as the
// game's streams write them), strings as a uint32 length then the bytes, and
// references (WriteGZSerializable) as uint32 objectID (index into the
// reference table + 1, 0 = null) then uint32 classID, each object listed once
// with its tokens for the interfaces the reader asks for. The tokens are
// borrowed identities: the original keeps its grids, producers, pipes and
// department alive across the delivery. Process-lifetime object (the
// reference count is not kept).
//
// `headerOnly` (the passCounter read, fc4_plumbing_original.pass_counter,
// after every delivery): references are not looked at (no QueryInterface, no
// GetGZCLSID: objectID 1 and classID 0, or 0 for null), and the bytes after
// the passCounter are not kept.
class CaptureStream final : public cFC4ISC4DBSegmentOStream {
public:
    explicit CaptureStream(bool headerOnly = false) : headerOnly_(headerOnly) {}

    void Reset() {
        bytes_.clear();
        refs_.clear();
        ids_.clear();
        good_ = true;
        need_ = 0;
    }

    // The passCounter of the record written (plumbing.md §9: the version, 4;
    // the working and published grid references; countX; countZ; the flag
    // map, countX * countZ bytes; then the passCounter, field 7, written only
    // by state 0, §6.1 PS6). False when the record is not that.
    bool PassCounter(int32_t* out) const {
        size_t at = 0;
        if (CounterAt(&at) != 1 || out == nullptr) return false;
        std::memcpy(out, bytes_.data() + at, 4);
        return true;
    }

    fc4_plumbing_capture View(bool writeOk) const {
        fc4_plumbing_capture c{};
        c.struct_size = sizeof(c);
        c.ok = (writeOk && good_) ? 1u : 0u;
        c.bytes = bytes_.empty() ? nullptr : bytes_.data();
        c.len = bytes_.size();
        c.refs = refs_.empty() ? nullptr : refs_.data();
        c.ref_count = refs_.size();
        return c;
    }

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid == kFC4_IID_cISC4DBSegmentOStream || riid == GZIID_cIGZUnknown) {
            *ppvObj = static_cast<cFC4ISC4DBSegmentOStream*>(this);
            return true;
        }
        return false;
    }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }

    // cIGZOStream
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
    bool SetRZCharStr(char const* s) override { return PutStr(s, s ? strlen(s) : 0); }
    bool SetGZStr(cIGZString const& s) override { return PutStr(s.Data(), s.Strlen()); }
    bool SetGZSerializable(cIGZSerializable const& s) override {
        if (!const_cast<cIGZSerializable&>(s).Write(*this)) good_ = false;
        return good_;
    }
    bool SetVoid(void const* p, uint32_t n) override {
        if (n != 0 && p == nullptr) {
            good_ = false;
            return false;
        }
        return Put(p, n);
    }
    int32_t GetError() override { return good_ ? 0 : 1; }
    int32_t SetUserData(cIGZVariant*) override { return 0; }
    int32_t GetUserData() override { return 0; }

    // cISC4DBSegmentOStream
    bool Open(cISC4DBSegment*, cGZPersistResourceKey const&, bool) override { return false; }
    bool Close() override { return true; }
    bool IsOpen() override { return true; }
    int32_t GetRecord() override { return 0; }
    int32_t GetSegment() override { return 0; }
    bool WriteGZSerializable(cIGZSerializable const* p) override {
        if (p == nullptr) return SetUint32(0);
        if (headerOnly_) {
            SetUint32(1);
            return SetUint32(0);
        }
        cIGZSerializable* obj = const_cast<cIGZSerializable*>(p);
        size_t i = 0;
        while (i < ids_.size() && ids_[i] != obj) ++i;
        if (i == ids_.size()) {
            fc4_plumbing_capture_ref r{};
            r.grid = Borrowed(obj, kIID_cISC4SimGrid);
            r.occupant = Borrowed(obj, kIID_cISC4Occupant);
            r.network = Borrowed(obj, kIID_cSC4PipeOccupant);
            r.department = Borrowed(obj, kIID_cISC4DepartmentBudget);
            r.class_id = obj->GetGZCLSID();
            ids_.push_back(obj);
            refs_.push_back(r);
        }
        SetUint32(static_cast<uint32_t>(i + 1));
        return SetUint32(refs_[i].class_id);
    }
    bool WriteResKey(cGZPersistResourceKey const& k) override {
        SetUint32(k.type);
        SetUint32(k.group);
        return SetUint32(k.instance);
    }
    bool WriteVariant(cIGZVariant const&) override {
        good_ = false;  // not part of the plumbing record (§9)
        return false;
    }

private:
    bool Put(const void* p, size_t n) {
        if (headerOnly_ && need_ != 0 && bytes_.size() >= need_) return good_;  // past the passCounter
        const uint8_t* b = static_cast<const uint8_t*>(p);
        bytes_.insert(bytes_.end(), b, b + n);  // x86: already little-endian
        if (headerOnly_ && need_ == 0) {
            size_t at = 0;
            if (CounterAt(&at) != 0) need_ = at + 4;  // known (or not a version-4 record: keep it short)
        }
        return good_;
    }
    bool Get(size_t* off, void* v, size_t n) const {
        if (bytes_.size() < *off || bytes_.size() - *off < n) return false;
        std::memcpy(v, bytes_.data() + *off, n);
        *off += n;
        return true;
    }
    // Where the passCounter is: 1 (at `*at`, its four bytes written), 0 (not
    // yet written), -1 (not a version-4 record; `*at` = what was read).
    int CounterAt(size_t* at) const {
        size_t off = 0;
        uint16_t version = 0;
        if (!Get(&off, &version, 2)) return 0;
        if (version != 4) {
            *at = off;
            return -1;
        }
        for (int i = 0; i < 2; ++i) {  // the working and published grids
            uint32_t id = 0, cls = 0;
            if (!Get(&off, &id, 4)) return 0;
            if (id != 0 && !Get(&off, &cls, 4)) return 0;
        }
        uint32_t cx = 0, cz = 0;
        if (!Get(&off, &cx, 4) || !Get(&off, &cz, 4)) return 0;
        const uint64_t n = static_cast<uint64_t>(cx) * cz;
        if (n > (uint64_t{1} << 24)) {  // no city has that many tracts
            *at = off;
            return -1;
        }
        off += static_cast<size_t>(n);
        *at = off;
        return bytes_.size() >= off && bytes_.size() - off >= 4 ? 1 : 0;
    }
    bool PutStr(const char* s, size_t n) {
        if (n > 0x00FFFFFF || (n != 0 && s == nullptr)) {
            good_ = false;
            return false;
        }
        SetUint32(static_cast<uint32_t>(n));
        return Put(s, n);
    }
    // The object's identity for `iid` (0 if it does not answer): queried,
    // and the reference dropped at once.
    static uint64_t Borrowed(cIGZSerializable* obj, uint32_t iid) {
        void* q = nullptr;
        if (!obj->QueryInterface(iid, &q) || q == nullptr) return 0;
        cIGZUnknown* u = static_cast<cIGZUnknown*>(q);
        FC4_ACQUIRED(u, "QueryInterface");
        const uint64_t t = TokenOf(u);
        FC4_RELEASE(u);
        return t;
    }

    std::vector<uint8_t> bytes_;
    std::vector<fc4_plumbing_capture_ref> refs_;
    std::vector<const void*> ids_;
    bool good_ = true;
    bool headerOnly_ = false;
    size_t need_ = 0;  // headerOnly_: the bytes kept (0 = not known yet)
};

CaptureStream g_capture;
CaptureStream g_counter(true);

// ---- settings and run state (main thread only) ---------------------------------------------------

bool g_enabled = false;
long g_cells = kDefaultCells;
long g_passes = kDefaultPasses;
int32_t g_settingsJulian = INT_MIN;

// Test seam (op 22).
bool g_testMode = false;
const FC4PlumbingShadowTestInput* g_test = nullptr;

// The original simulator's agent (its cIGZMessageTarget2), per simulator.
const void* g_agentSim = nullptr;
cIGZMessageTarget2* g_agent = nullptr;
bool g_loggedOurs = false;
bool g_loggedNoSerializable = false;

// The shadow core, per city, original simulator and settings. Its reads go to
// g_game, filled with the city's borrowed services for one delivery.
FC4PlumbingGame g_game;
const HostGridContext g_gridCtx{"plumbing shadow", Sw::PlumbingGridWrites};
fc4_plumbing_shadow* g_shadow = nullptr;
const void* g_shadowCity = nullptr;
const void* g_shadowSim = nullptr;
long g_shadowCells = 0;
long g_shadowPasses = 0;
const void* g_shadowFailedFor = nullptr;
// A simulator whose shadowed delivery raised a structured exception: not
// shadowed again (its deliveries pass through).
const void* g_faultedSim = nullptr;

// The delivery being wrapped.
struct Delivery {
    uint32_t type = 0;
    int32_t julian = 0;
    int32_t result = -1;
};
Delivery g_d;
int32_t g_lastResult = -1;

void ReadSettings() {
    g_enabled = SettingInt("plumbing_shadow", "FC4_PLUMBING_SHADOW", 0) != 0;
    long n = SettingInt("plumbing_shadow_cells", "FC4_PLUMBING_SHADOW_CELLS", kDefaultCells);
    if (n <= 0) n = kDefaultCells;
    if (n > kMaxCells) n = kMaxCells;
    g_cells = n;
    long p = SettingInt("plumbing_shadow_passes", "FC4_PLUMBING_SHADOW_PASSES", kDefaultPasses);
    g_passes = p < 0 ? kDefaultPasses : p;
}

cISC4City* CurrentCity() {
    if (g_testMode) return g_test->city;
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    FC4_ACQUIRED(sc4, "QueryInterface");
    cISC4City* city = sc4->GetCity();  // borrowed
    FC4_RELEASE(sc4);
    return city;
}

// The original plumbing simulator if `target` is its agent: its
// cIGZMessageTarget2 interface, or failing that (logged) an agent registered
// under its name.
cISC4PlumbingSimulator* OriginalFor(cISC4City* city, cIGZMessageTarget2* target, const char* agentName) {
    if (city == nullptr || target == nullptr) return nullptr;
    cISC4PlumbingSimulator* raw = city->GetPlumbingSimulator();  // compiled slot 64, borrowed
    if (raw == nullptr) return nullptr;
    if (IsOurObject(raw)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("plumbing shadow: inactive: the city's plumbing simulator is FreeCity4's own (the shadow compares "
                "against the ORIGINAL plumbing simulator: do not enable plumbing)");
        }
        return nullptr;
    }
    if (raw != g_agentSim) {
        void* agent = nullptr;
        const bool qi = raw->QueryInterface(kIID_cIGZMessageTarget2, &agent) && agent != nullptr;
        g_agent = nullptr;
        if (qi) {
            cIGZUnknown* a = static_cast<cIGZUnknown*>(agent);
            FC4_ACQUIRED(a, "QueryInterface");
            g_agent = static_cast<cIGZMessageTarget2*>(agent);
            FC4_RELEASE(a);  // the simulator keeps it alive; only its identity is kept
        }
        g_agentSim = raw;
        Log("plumbing shadow: the original plumbing simulator %p has agent %p (%s)", static_cast<void*>(raw),
            static_cast<void*>(g_agent), qi ? "its cIGZMessageTarget2 interface" : "no interface: matched by name");
    }
    const bool same = g_agent != nullptr && target == g_agent;
    const bool byName = g_agent == nullptr && agentName != nullptr && strcmp(agentName, kAgentName) == 0;
    return (same || byName) ? raw : nullptr;
}

// The services the read callbacks use, borrowed from the city for one
// delivery (the test seam's city has none: neutral reads).
void FillGame(cISC4City* city) {
    g_game.gated = false;  // the original's inputs as they are
    if (g_testMode || city == nullptr) return;
    g_game.city = city;
    g_game.occupantManager = city->GetOccupantManager();
    g_game.lotManager = city->GetLotManager();
    g_game.pollution = city->GetPollutionSimulator();
    g_game.ordinance = city->GetOrdinanceSimulator();
    g_game.neighbors = city->GetNeighborsSimulator();
    g_game.budget = city->GetBudgetSimulator();
    g_game.terrain = city->GetTerrain();
}

void ClearGame() {
    g_game.held.clear();  // the occupant, deal and connection lists (and their references)
    g_game.city = nullptr;
    g_game.occupantManager = nullptr;
    g_game.lotManager = nullptr;
    g_game.pollution = nullptr;
    g_game.ordinance = nullptr;
    g_game.neighbors = nullptr;
    g_game.budget = nullptr;
    g_game.terrain = nullptr;
}

// ---- the original, read-only (fc4_plumbing_original; user = its cISC4PlumbingSimulator*) ---------------

cFC4ISC4PlumbingSimulator* Orig(void* u) { return AsGame(static_cast<cISC4PlumbingSimulator*>(u)); }

uint64_t OrigWateredGrid(void* u) { return TokenOf(Orig(u)->GetWateredGrid()); }  // slot 7, borrowed

int64_t OrigSlot(void* u, uint32_t slot) {
    cFC4ISC4PlumbingSimulator* p = Orig(u);
    switch (slot) {
        case 12: return p->GetNumWaterSources();
        case 13: return p->GetNumPollutedWaterSources();
        // 14-17 and 23: the low 32 bits in EAX, signed (§10.4).
        case 14: return p->GetWaterProductionCapacity();
        case 15: return p->GetWaterConsumed();
        case 16: return p->GetWaterImported();
        case 17: return p->GetWaterExported();
        case 18: return p->GetNumPipes();
        case 19: return p->GetNumDistressedPipes();
        case 20: return p->GetNumBurstPipes();
        case 21: return p->GetNumWateredBuildings();
        case 22: return p->GetNumUnwateredBuildings();
        case 23: return p->GetWaterDemand();
        default: return 0;
    }
}

bool OrigProducerInfo(void* u, uint64_t occ, fc4_plumbing_producer_info* out) {
    if (out == nullptr) return false;
    uint32_t cur = 0, max = 0, used = 0;
    SC4Percentage eff{};
    bool polluted = false;
    // Slot 24: AL = 0 for an occupant that is not a producer.
    if (!Orig(u)->GetWaterProducerInfo(reinterpret_cast<cISC4Occupant*>(FromToken(occ)), &cur, &max, &eff, &polluted,
                                       &used)) {
        return false;
    }
    out->production = cur;
    out->max_capacity = max;
    out->efficiency = eff.percentage;
    out->polluted = polluted;
    out->used = used;
    return true;
}

bool OrigConnectionVisited(void*, uint64_t conn) {
    return conn != 0 && reinterpret_cast<cISC4NeighborConnection*>(FromToken(conn))->GetVisitedThisCycle();
}

// The original's record through its own Write (cIGZSerializable).
bool OrigCapture(void* u, fc4_plumbing_capture* out) {
    g_capture.Reset();
    bool ok = false;
    void* p = nullptr;
    cISC4PlumbingSimulator* sim = static_cast<cISC4PlumbingSimulator*>(u);
    if (sim->QueryInterface(GZIID_cIGZSerializable, &p) && p != nullptr) {
        cIGZSerializable* ser = static_cast<cIGZSerializable*>(p);
        FC4_ACQUIRED(ser, "QueryInterface");
        ok = ser->Write(g_capture);
        FC4_RELEASE(ser);
    } else if (!g_loggedNoSerializable) {
        g_loggedNoSerializable = true;
        Log("plumbing shadow: the original plumbing simulator does not answer cIGZSerializable; its record cannot be "
            "read: nothing is compared");
    }
    if (out != nullptr) *out = g_capture.View(ok);
    return ok;
}

// The original's passCounter (plumbing.md §6.1, PS6), through its own Write
// into the header-only stream (a stream of its own: a capture view stays
// valid).
bool OrigPassCounter(void* u, int32_t* out) {
    g_counter.Reset();
    void* p = nullptr;
    cISC4PlumbingSimulator* sim = static_cast<cISC4PlumbingSimulator*>(u);
    if (out == nullptr || !sim->QueryInterface(GZIID_cIGZSerializable, &p) || p == nullptr) return false;
    cIGZSerializable* ser = static_cast<cIGZSerializable*>(p);
    FC4_ACQUIRED(ser, "QueryInterface");
    const bool ok = ser->Write(g_counter);
    FC4_RELEASE(ser);
    return ok && g_counter.GetError() == 0 && g_counter.PassCounter(out);
}

void DropShadow() {
    fc4_plumbing_shadow_destroy(g_shadow);
    g_shadow = nullptr;
    g_shadowCity = nullptr;
    g_shadowSim = nullptr;
}

// `cityStart`: made at PostCityInit (PlumbingShadowCityInit), before the
// city's first delivery.
bool EnsureShadow(const void* city, cISC4PlumbingSimulator* sim, bool cityStart) {
    if (g_shadow != nullptr && g_shadowCity == city && g_shadowSim == sim && g_shadowCells == g_cells &&
        g_shadowPasses == g_passes) {
        return true;
    }
    DropShadow();
    const fc4_plumbing_host host = PlumbingReadOnlyHost(&g_game, &g_gridCtx);  // copied by the core
    fc4_plumbing_original orig{};
    orig.struct_size = sizeof(orig);
    orig.user = sim;
    orig.watered_grid = &OrigWateredGrid;
    orig.slot_value = &OrigSlot;
    orig.producer_info = &OrigProducerInfo;
    orig.connection_visited = &OrigConnectionVisited;
    orig.capture = &OrigCapture;
    orig.pass_counter = &OrigPassCounter;
    g_shadow = fc4_plumbing_shadow_create(&host, &orig, static_cast<uint32_t>(g_cells), static_cast<uint32_t>(g_passes));
    if (g_shadow == nullptr) {
        if (g_shadowFailedFor != sim) {
            g_shadowFailedFor = sim;
            Log("plumbing shadow: the shadow core could not be created; deliveries pass through uncompared");
        }
        return false;
    }
    // Made before the city's first delivery (PostCityInit): the original's
    // pipe condition and ageing flag are known (0, false; plumbing.md §7.1
    // PS3); otherwise the shadow starts mid-city.
    const bool known = cityStart && fc4_plumbing_shadow_city_start(g_shadow);
    g_shadowCity = city;
    g_shadowSim = sim;
    g_shadowCells = g_cells;
    g_shadowPasses = g_passes;
    Log("plumbing shadow: comparing with the original plumbing simulator %p (city %p); differing cells listed per "
        "line: %ld; passes replayed per day: %ld (0 = all); made %s",
        static_cast<void*>(sim), city, g_cells, g_passes,
        known ? "at the city start (the original's pipe condition is known: 0)"
              : "mid-city (the original's pipe condition is not known: the distressed count is not exact)");
    return true;
}

// fc4::Log has a fixed buffer: a long line goes out in pieces, split at a
// space where there is one.
void Sink(void*, const uint8_t* data, size_t len) {
    const char* p = reinterpret_cast<const char*>(data);
    size_t off = 0;
    bool first = true;
    while (first || off < len) {
        size_t n = std::min(kLogChunk, len - off);
        if (off + n < len) {
            size_t cut = n;
            while (cut > kLogChunk / 2 && p[off + cut - 1] != ' ') --cut;
            if (cut > kLogChunk / 2) n = cut;
        }
        if (first) {
            Log("%.*s", static_cast<int>(n), p + off);
        } else {
            Log("plumbing shadow (cont.) %.*s", static_cast<int>(n), p + off);
        }
        off += n;
        first = false;
    }
}

void BeforeStep(void*) { fc4_plumbing_shadow_before(g_shadow, g_d.type, g_d.julian); }

void AfterStep(void*) { g_d.result = fc4_plumbing_shadow_after(g_shadow, &Sink, nullptr); }

}  // namespace

void PlumbingShadowStartup(bool simulatorReplaced, bool plumbingReplaced) {
    ReadSettings();
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("plumbing_shadow", "FC4_PLUMBING_SHADOW", &src);
    if (simulatorReplaced && !plumbingReplaced) {
        Log("plumbing shadow: ON (plumbing_shadow from %s): at each SimIdle and SimNewDay of the ORIGINAL plumbing "
            "simulator (one pass state each), FreeCity4's water pass replayed on the original's record from the "
            "start of its pass (its passCounter moved: state 0 ran; or from its swap, then not exact), state by "
            "state: each of our states 2 to 5 just before the original's, on its flag map of the moment, with no "
            "time limit (a pass is followed until its swap or its restart), and compared "
            "at its swap (the published grid cell by cell, producers' production, efficiency, pollution and water "
            "used, deal usage, water connections) and after its state 8 (produced, consumed, imported, exported, "
            "watered and unwatered cells, cost, demand, pipes, distressed, burst, sources, budget items, ageing); "
            "one line per replayed pass; at each SimNewMonth the pipe funding of the simulator's own department "
            "(distressed exactly when the shadow was made at the city start, else not exact; new bursts are "
            "random: by range, not exact); differing cells listed per line: %ld; passes replayed per game day: %ld "
            "(0 = all). The record is read through the original's own Write into memory. Not compared (not "
            "readable read-only): pipe watered states (a non-virtual call), redisplay, posts, history, line-item "
            "expenses, lot conditions, deal flags, the original's pipe condition (not saved; 0 at a city start) and "
            "pass state; demand (the original's running sum, moved by the occupant messages, never recomputed) is "
            "taken from the original's record at the comparison",
            src, g_cells, g_passes);
    } else {
        Log("plumbing shadow: requested (plumbing_shadow from %s) but INACTIVE: it needs FreeCity4's simulator and "
            "the original plumbing simulator (do not enable plumbing); here the simulator is %s and plumbing is %s",
            src, simulatorReplaced ? "ours" : "the original", plumbingReplaced ? "ours" : "the original");
    }
}

bool PlumbingShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                           int64_t data1, bool* finished) {
    if ((type != kMsgSimIdle && type != kMsgSimNewDay && type != kMsgSimNewMonth) || target == nullptr ||
        msg == nullptr || finished == nullptr) {
        return false;
    }
    const int32_t julian = static_cast<int32_t>(data1);
    if (!g_testMode && type == kMsgSimNewDay && julian != g_settingsJulian) {
        g_settingsJulian = julian;
        ReadSettings();  // once per day (FreeCity4.ini may change between city loads)
    }
    if (!g_testMode && !g_enabled) return false;
    // Cheap rejection for every other agent: ours registers by name (AddAgent).
    if (!g_testMode && target != g_agent && (agentName == nullptr || strcmp(agentName, kAgentName) != 0)) return false;
    cISC4City* city = CurrentCity();
    cISC4PlumbingSimulator* sim = OriginalFor(city, target, agentName);
    // A shadow made here starts mid-city (PlumbingShadowCityInit makes the
    // one of a city start); a test seam run is a city start unless it says
    // mid-city.
    const bool cityStart = g_testMode && g_test->midCity == 0;
    if (sim == nullptr || sim == g_faultedSim || !EnsureShadow(city, sim, cityStart)) return false;

    g_d = Delivery{};
    g_d.type = type;
    g_d.julian = julian;
    FillGame(city);
    const bool beforeClean = SehGuard("plumbing shadow (before the original's update)", &BeforeStep, nullptr);
    *finished = target->DoMessage(msg);  // the original plumbing simulator, unchanged
    const bool afterClean = beforeClean && SehGuard("plumbing shadow (after the original's update)", &AfterStep,
                                                    nullptr);
    ClearGame();
    if (!afterClean) {
        // The shadow's state is unknown after a structured exception: it is
        // dropped, and this simulator's deliveries pass through from now on.
        g_faultedSim = sim;
        DropShadow();
        g_d.result = -1;
        Log("plumbing shadow: a structured exception in the shadow: the original plumbing simulator %p is no "
            "longer shadowed",
            static_cast<void*>(sim));
    }
    g_lastResult = g_d.result;
    g_d = Delivery{};
    return true;  // delivered (compared or not)
}

namespace {
struct CityStart {
    cISC4City* city;
    cISC4PlumbingSimulator* sim;
};
void CityStartStep(void* ctx) {
    CityStart* c = static_cast<CityStart*>(ctx);
    FillGame(c->city);
    EnsureShadow(c->city, c->sim, true);
    ClearGame();
}
}  // namespace

void PlumbingShadowCityInit() {
    if (g_testMode) return;
    ReadSettings();
    if (!g_enabled) return;
    DropShadow();  // a new city: a new shadow
    cISC4City* city = CurrentCity();
    cISC4PlumbingSimulator* sim = city ? city->GetPlumbingSimulator() : nullptr;  // compiled slot 64, borrowed
    // Ours, or none: the first delivery says why it is inactive.
    if (sim == nullptr || IsOurObject(sim) || sim == g_faultedSim) return;
    CityStart c{city, sim};
    SehGuard("plumbing shadow (city start)", &CityStartStep, &c);
}

}  // namespace fc4

intptr_t PlumbingShadowTestRun(const FC4PlumbingShadowTestInput* in) {
    if (in == nullptr || (in->count != 0 && (in->types == nullptr || in->julians == nullptr))) return -1;
    fc4::g_testMode = true;
    fc4::g_test = in;
    fc4::g_agentSim = nullptr;  // each run is a fresh "game"
    fc4::g_agent = nullptr;
    fc4::g_shadowFailedFor = nullptr;
    fc4::g_faultedSim = nullptr;
    fc4::DropShadow();
    const long cells = fc4::g_cells, passes = fc4::g_passes;
    fc4::g_cells = fc4::kDefaultCells;
    fc4::g_passes = static_cast<long>(in->passesPerDay);
    bool taken = false;
    bool compared = false;
    intptr_t sum = 0;
    for (uint32_t i = 0; i < in->count; ++i) {
        const uint32_t type = in->types[i];
        const int32_t julian = in->julians[i];
        cFC4Message2Standard* msg = FC4MakeMessage(type, static_cast<intptr_t>(julian), 0, 5);
        bool finished = false;
        fc4::g_lastResult = -1;
        int32_t r = -3;
        if (fc4::PlumbingShadowDeliver(in->target, nullptr, msg, type, julian, &finished)) {
            taken = true;
            r = fc4::g_lastResult;
            if (r >= 0) {
                compared = true;
                sum += r;
            }
        } else if (in->target != nullptr) {
            in->target->DoMessage(msg);
        }
        msg->Release();
        if (in->results != nullptr) in->results[i] = r;
    }
    fc4::DropShadow();
    fc4::g_cells = cells;
    fc4::g_passes = passes;
    fc4::g_agentSim = nullptr;
    fc4::g_agent = nullptr;
    fc4::g_testMode = false;
    fc4::g_test = nullptr;
    if (!taken) return -1;
    return compared ? sum : -2;
}
