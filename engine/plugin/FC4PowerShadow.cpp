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

#include "FC4PowerShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"
#include "cFC4Power.h"

#include "GZServPtrs.h"
#include "cGZPersistResourceKey.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cIGZString.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4NeighborConnection.h"
#include "cISC4PowerSimulator.h"
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
constexpr uint32_t kMsgPostCityInit = 0x26D31EC1;
// The §5 messages forwarded to the shadow (fc4_power_shadow_message): the
// removal and the zoning change move the dirty rectangle (§5.3, PW5), the
// tuning reload reads the factor (§8, PW7).
constexpr uint32_t kMsgOccupantRemoved = 0x99EF1143;
constexpr uint32_t kMsgZoningChanged = 0x07699455;
constexpr uint32_t kMsgTuningReload = 0xCA1B151C;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
// The interfaces the §9.1 reader asks a reference for (fc4_power_capture_ref).
constexpr uint32_t kIID_cISC4SimGrid = 0xC9BD939A;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4PowerPoleOccupant = 0xA8C86142;
constexpr uint32_t kIID_cISC4DepartmentBudget = 0x223E2B67;
constexpr const char* kAgentName = "cSC4PowerSimulator";  // power.md §4.1
// The original's subscriptions (power.md §4.1 step 2): any of them may ask
// for a recompute, so a SimIdle after one is wrapped even when the original
// was idle.
constexpr uint32_t kEvents[] = {0x99EF1142, 0x99EF1143, 0x07699455, 0xCA1B151C, 0x278E6003,
                                0xC2C09554, 0xE8BDA2E7, 0xEA418D8F, 0x6AC284F3, kMsgPostCityInit};
constexpr long kDefaultCells = 8;
constexpr long kMaxCells = 64;
constexpr long kDefaultLines = 32;
// fc4::Log formats into a fixed buffer: longer shadow lines are split.
constexpr size_t kLogChunk = 2800;

// ---- the capture stream ------------------------------------------------------------------------------
//
// The original's Write goes into memory: primitives little-endian (as the
// game's streams write them), strings as a uint32 length then the bytes, and
// references (WriteGZSerializable) as uint32 objectID (index into the
// reference table + 1, 0 = null) then uint32 classID, each object listed once
// with its tokens for the interfaces the reader asks for. With `hold`, the
// occupant identities keep their QueryInterface reference until Release():
// the shadow reads plant properties after the original's update, which may
// have removed a plant. Process-lifetime objects (the reference count is not
// kept).
class CaptureStream final : public cFC4ISC4DBSegmentOStream {
public:
    explicit CaptureStream(bool hold) : hold_(hold) {}

    void Reset() {
        ReleaseHeld();
        bytes_.clear();
        refs_.clear();
        ids_.clear();
        good_ = true;
    }

    void ReleaseHeld() {
        for (cIGZUnknown* u : held_) FC4_RELEASE(u);
        held_.clear();
    }

    fc4_power_capture View(bool writeOk) const {
        fc4_power_capture c{};
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
        cIGZSerializable* obj = const_cast<cIGZSerializable*>(p);
        size_t i = 0;
        while (i < ids_.size() && ids_[i] != obj) ++i;
        if (i == ids_.size()) {
            fc4_power_capture_ref r{};
            r.grid = Borrowed(obj, kIID_cISC4SimGrid, false);
            r.occupant = Borrowed(obj, kIID_cISC4Occupant, hold_);
            r.department = Borrowed(obj, kIID_cISC4DepartmentBudget, false);
            if (Borrowed(obj, kIID_cISC4PowerPoleOccupant, false) != 0) r.flags |= FC4_POWER_CAPTURE_POLE;
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
        good_ = false;  // not part of the power record (§9.1)
        return false;
    }

private:
    bool Put(const void* p, size_t n) {
        const uint8_t* b = static_cast<const uint8_t*>(p);
        bytes_.insert(bytes_.end(), b, b + n);  // x86: already little-endian
        return good_;
    }
    bool PutStr(const char* s, size_t n) {
        if (n > 0x00FFFFFF || (n != 0 && s == nullptr)) {
            good_ = false;
            return false;
        }
        SetUint32(static_cast<uint32_t>(n));
        return Put(s, n);
    }
    // The object's identity for `iid` (0 if it does not answer): queried, and
    // the reference dropped at once, or with `keep` held until ReleaseHeld.
    uint64_t Borrowed(cIGZSerializable* obj, uint32_t iid, bool keep) {
        void* q = nullptr;
        if (!obj->QueryInterface(iid, &q) || q == nullptr) return 0;
        cIGZUnknown* u = static_cast<cIGZUnknown*>(q);
        FC4_ACQUIRED(u, "QueryInterface");
        const uint64_t t = TokenOf(u);
        if (keep) {
            held_.push_back(u);
        } else {
            FC4_RELEASE(u);
        }
        return t;
    }

    bool hold_;
    std::vector<uint8_t> bytes_;
    std::vector<fc4_power_capture_ref> refs_;
    std::vector<const void*> ids_;
    std::vector<cIGZUnknown*> held_;
    bool good_ = true;
};

CaptureStream g_before{true};
CaptureStream g_after{false};

// ---- settings and run state (main thread only) ---------------------------------------------------

bool g_enabled = false;
long g_cells = kDefaultCells;
long g_lines = kDefaultLines;
int32_t g_settingsJulian = INT_MIN;

// Test seam (op 21).
bool g_testMode = false;
const FC4PowerShadowTestInput* g_test = nullptr;

// The original simulator's agent (its cIGZMessageTarget2), per simulator.
const void* g_agentSim = nullptr;
cIGZMessageTarget2* g_agent = nullptr;
bool g_loggedOurs = false;
bool g_loggedNoSerializable = false;

// One of the original's §5 messages arrived since the last wrapped delivery.
// A city start (PowerShadowCityInit) starts a new generation (a new shadow).
bool g_event = false;
uint32_t g_generation = 0;
bool g_listening = false;

// The shadow core, per city, original simulator and generation. Its reads go
// to g_game, filled with the city's borrowed services for one delivery.
FC4PowerGame g_game;
const HostGridContext g_gridCtx{"power shadow", Sw::PowerGridWrites};
fc4_power_shadow* g_shadow = nullptr;
const void* g_shadowCity = nullptr;
const void* g_shadowSim = nullptr;
uint32_t g_shadowGeneration = 0;
long g_shadowCells = 0;
long g_shadowLines = 0;
const void* g_shadowFailedFor = nullptr;

// The delivery being wrapped.
struct Delivery {
    cISC4PowerSimulator* sim = nullptr;
    uint32_t type = 0;
    int32_t julian = 0;
    bool begun = false;
    bool finished = false;
    int32_t result = -1;
};
Delivery g_d;
int32_t g_lastResult = -1;
bool g_lastPassed = false;

void ForwardToShadow(cIGZMessage2* msg);

// Counts the original's messages and forwards those the shadow mirrors (a
// static object: counted, never deleted).
class EventTarget final : public cIGZMessageTarget2 {
public:
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid != kIID_cIGZMessageTarget2 && riid != GZIID_cIGZUnknown) return false;
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { return refs_ > 0 ? --refs_ : 0u; }
    bool DoMessage(cIGZMessage2* msg) override {
        if (msg == nullptr) return true;
        FC4_X87_SCOPE("power shadow events DoMessage", msg->GetType());  // x87 leftover guard (FC4Fpu.h)
        g_event = true;
        ForwardToShadow(msg);
        return true;
    }

private:
    uint32_t refs_ = 0;
};

EventTarget& Events() {
    static EventTarget t;
    return t;
}

// Registered once, at the first delivery the shadow looks at (the message
// server exists by then), and left registered for the process.
void Listen() {
    if (g_listening || g_testMode) return;
    g_listening = true;
    cIGZMessageServer2Ptr ms;
    if (!ms) {
        Log("power shadow: no message server; every SimIdle is wrapped");
        return;
    }
    Events().AddRef();  // our own reference (static object)
    bool ok = true;
    for (uint32_t id : kEvents) ok = ms->AddNotification(static_cast<cIGZMessageTarget2*>(&Events()), id) && ok;
    g_event = true;  // whatever came before
    if (!ok) Log("power shadow: some notifications were refused; the idle pass-through may miss a recompute");
}

void ReadSettings() {
    g_enabled = SettingInt("power_shadow", "FC4_POWER_SHADOW", 0) != 0;
    long n = SettingInt("power_shadow_cells", "FC4_POWER_SHADOW_CELLS", kDefaultCells);
    if (n <= 0) n = kDefaultCells;
    if (n > kMaxCells) n = kMaxCells;
    g_cells = n;
    long l = SettingInt("power_shadow_lines", "FC4_POWER_SHADOW_LINES", kDefaultLines);
    if (l == 0) l = kDefaultLines;
    g_lines = l;  // negative: every cycle
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

// The original power simulator if `target` is its agent: its
// cIGZMessageTarget2 interface, or failing that (logged) an agent registered
// under its name.
cISC4PowerSimulator* OriginalFor(cISC4City* city, cIGZMessageTarget2* target, const char* agentName) {
    if (city == nullptr || target == nullptr) return nullptr;
    cISC4PowerSimulator* raw = city->GetPowerSimulator();  // compiled slot 67, borrowed
    if (raw == nullptr) return nullptr;
    if (IsOurObject(raw)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("power shadow: inactive: the city's power simulator is FreeCity4's own (the shadow compares against "
                "the ORIGINAL power simulator: do not enable power)");
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
        Log("power shadow: the original power simulator %p has agent %p (%s)", static_cast<void*>(raw),
            static_cast<void*>(g_agent), qi ? "its cIGZMessageTarget2 interface" : "no interface: matched by name");
    }
    const bool same = g_agent != nullptr && target == g_agent;
    const bool byName = g_agent == nullptr && agentName != nullptr && strcmp(agentName, kAgentName) == 0;
    return (same || byName) ? raw : nullptr;
}

// The services the read callbacks use, borrowed from the city for one
// delivery (the test seam's city has none).
void FillGame(cISC4City* city) {
    g_game.city = city;
    g_game.cityServices = !g_testMode;
    if (g_testMode || city == nullptr) return;
    g_game.occupantManager = city->GetOccupantManager();
    g_game.zoneManager = city->GetZoneManager();
    g_game.neighbors = city->GetNeighborsSimulator();
    g_game.budget = city->GetBudgetSimulator();
    g_game.demolition = nullptr;  // only written to by the simulator: never read here
}

void ClearGame() {
    g_game.held.clear();  // the occupant, deal and connection lists (and their references)
    g_game.city = nullptr;
    g_game.occupantManager = nullptr;
    g_game.zoneManager = nullptr;
    g_game.neighbors = nullptr;
    g_game.budget = nullptr;
    g_game.demolition = nullptr;
}

// cISC4NeighborConnection::GetVisitedThisCycle (compiled slot 11), for the
// shadow's FindUnvisitedConnection.
bool ConnectionVisited(void*, uint64_t c) {
    return c != 0 && reinterpret_cast<cISC4NeighborConnection*>(FromToken(c))->GetVisitedThisCycle();
}

void DropShadow() {
    fc4_power_shadow_destroy(g_shadow);
    g_shadow = nullptr;
    g_shadowCity = nullptr;
    g_shadowSim = nullptr;
}

// `cityStart`: created at PostCityInit (FC4_POWER_SHADOW_CITY_START), when
// the original's unsaved values are those of its Init after the load (the
// ordinance factor, read now through the city; pass import / export /
// demand and brokenFraction 0; no dirty rectangle). Otherwise the shadow
// starts mid-city and they are unknown until the original sets them again.
bool EnsureShadow(cISC4City* city, const void* sim, bool cityStart) {
    if (g_shadow != nullptr && g_shadowCity == city && g_shadowSim == sim && g_shadowGeneration == g_generation &&
        g_shadowCells == g_cells && g_shadowLines == g_lines) {
        return true;
    }
    DropShadow();
    const fc4_power_host host = PowerReadOnlyHost(&g_game, &g_gridCtx);  // copied by the core
    fc4_power_shadow_options o{};
    o.struct_size = sizeof(o);
    o.max_cells = static_cast<uint32_t>(g_cells);
    o.max_lines = g_lines < 0 ? UINT32_MAX : static_cast<uint32_t>(g_lines);
    o.connection_visited = &ConnectionVisited;
    o.flags = cityStart ? FC4_POWER_SHADOW_CITY_START : 0u;
    const bool fill = g_game.city == nullptr;  // the creation reads the factor through the city
    if (fill) FillGame(city);
    g_shadow = fc4_power_shadow_create(&host, &o);
    if (fill) ClearGame();
    if (g_shadow == nullptr) {
        if (g_shadowFailedFor != sim) {
            g_shadowFailedFor = sim;
            Log("power shadow: the shadow core could not be created; deliveries pass through uncompared");
        }
        return false;
    }
    g_shadowCity = city;
    g_shadowSim = sim;
    g_shadowGeneration = g_generation;
    g_shadowCells = g_cells;
    g_shadowLines = g_lines;
    Log("power shadow: comparing with the original power simulator %p (city %p) %s; differing cells listed per "
        "line: %ld; cycle lines with a difference per month: %ld",
        sim, static_cast<void*>(city),
        cityStart ? "from the city start (its unsaved values known: power.md §8, §7.6, §9.1, §5.3)"
                  : "from mid-city (not known: the ordinance factor until the first month, the pass import / export "
                    "/ demand until the first state 0, brokenFraction, the dirty rectangle until the first step)",
        g_cells, g_lines);
    return true;
}

// ---- forwarded messages (main thread: the message server delivers on the sender's thread) ---------------

struct Forwarded {
    fc4_power_message m{};
};

void ForwardStep(void* ctx) { fc4_power_shadow_message(g_shadow, &static_cast<Forwarded*>(ctx)->m); }

// The removal, zoning change and tuning reload the original handles (§5.2,
// §5.3, §8) go to the shadow as well, converted as cFC4PowerSimulator's
// DoMessage converts them: the dirty rectangle (PW5) and the factor (PW7)
// are then the original's from the city start on.
void ForwardToShadow(cIGZMessage2* msg) {
    const uint32_t type = msg->GetType();
    if (type != kMsgOccupantRemoved && type != kMsgZoningChanged && type != kMsgTuningReload) return;
    if (g_shadow == nullptr) return;
    cISC4City* city = CurrentCity();
    if (city == nullptr || city != g_shadowCity) return;
    Forwarded f;
    f.m.msg_type = type;
    cIGZMessage2Standard* s = nullptr;
    cIGZUnknown* occ = nullptr;
    void* p = nullptr;
    if (msg->QueryInterface(GZIID_cIGZMessage2Standard, &p) && p != nullptr) {
        s = static_cast<cIGZMessage2Standard*>(p);
        FC4_ACQUIRED(s, "QueryInterface");
        f.m.data1 = static_cast<uint32_t>(s->GetData1());
        f.m.data2 = static_cast<uint32_t>(s->GetData2());
        f.m.data3 = static_cast<uint32_t>(s->GetData3());
        f.m.data4 = static_cast<uint32_t>(s->GetData4());
        if (type == kMsgOccupantRemoved) {
            // §5.1: the occupant is void1, known by its cISC4Occupant identity.
            cIGZUnknown* v = static_cast<cIGZUnknown*>(s->GetVoid1());
            void* q = nullptr;
            if (v != nullptr && v->QueryInterface(kIID_cISC4Occupant, &q) && q != nullptr) {
                occ = static_cast<cIGZUnknown*>(q);
                FC4_ACQUIRED(occ, "QueryInterface");
                f.m.void1 = TokenOf(occ);
            }
        } else if (type == kMsgZoningChanged) {
            // [abi] void2 is the message's SC4CellRegion<long>* (as in
            // cFC4PowerSimulator::DoMessage): its bounds {x1, z1, x2, z2}.
            const int32_t* r = static_cast<const int32_t*>(s->GetVoid2());
            if (r != nullptr) {
                f.m.rect.x1 = r[0];
                f.m.rect.z1 = r[1];
                f.m.rect.x2 = r[2];
                f.m.rect.z2 = r[3];
            }
        }
    }
    // Inside a wrapped delivery the services are borrowed already.
    const bool fill = g_d.sim == nullptr;
    if (fill) FillGame(city);
    SehGuard("power shadow (a forwarded message)", &ForwardStep, &f);
    if (fill) ClearGame();
    if (occ != nullptr) FC4_RELEASE(occ);
    if (s != nullptr) FC4_RELEASE(s);
}

// The original's record through its own Write (cIGZSerializable).
fc4_power_capture Capture(CaptureStream& s) {
    s.Reset();
    bool ok = false;
    void* p = nullptr;
    if (g_d.sim->QueryInterface(GZIID_cIGZSerializable, &p) && p != nullptr) {
        cIGZSerializable* ser = static_cast<cIGZSerializable*>(p);
        FC4_ACQUIRED(ser, "QueryInterface");
        ok = ser->Write(s);
        FC4_RELEASE(ser);
    } else if (!g_loggedNoSerializable) {
        g_loggedNoSerializable = true;
        Log("power shadow: the original power simulator does not answer cIGZSerializable; its record cannot be "
            "read: nothing is compared");
    }
    return s.View(ok);
}

// Slots 27-29 of the original (getters, power.md §10.1).
fc4_power_shadow_counts Counts() {
    fc4_power_shadow_counts c{};
    c.struct_size = sizeof(c);
    cFC4ISC4PowerSimulator* p = AsGame(g_d.sim);
    c.overworked = static_cast<uint32_t>(p->GetOverworkedPowerPlantCount());
    c.aged = static_cast<uint32_t>(p->GetAgedPowerPlantCount());
    c.distressed = static_cast<uint32_t>(p->GetDistressedPowerPlantCount());
    c.ok = 1;
    return c;
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
            Log("power shadow (cont.) %.*s", static_cast<int>(n), p + off);
        }
        off += n;
        first = false;
    }
}

void BeforeStep(void*) {
    const fc4_power_capture c = Capture(g_before);
    const fc4_power_shadow_counts n = Counts();
    fc4_power_shadow_begin(g_shadow, g_d.type, g_d.julian, &c, &n);  // the reason, if not run, comes with finish
    g_d.begun = true;
}

void AfterStep(void*) {
    const fc4_power_capture c = Capture(g_after);
    const fc4_power_shadow_counts n = Counts();
    g_d.result = fc4_power_shadow_finish(g_shadow, &c, &n, &Sink, nullptr);
    g_d.finished = true;
}

}  // namespace

void PowerShadowStartup(bool simulatorReplaced, bool powerReplaced) {
    ReadSettings();
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("power_shadow", "FC4_POWER_SHADOW", &src);
    if (simulatorReplaced && !powerReplaced) {
        Log("power shadow: ON (power_shadow from %s): at each SimNewMonth, SimNewDay and working SimIdle of the "
            "ORIGINAL power simulator, FreeCity4's DoIdle step or month on its record read just before, compared "
            "just after with its record (fields, consumption map, mark / wire / powered bitmaps), its powered grid "
            "cell by cell, its plant counts (slots 27-29) and the occupant properties, deal usage, connection flags "
            "and budget item costs FreeCity4 wrote; one line per recompute cycle (state 3: production, consumption, "
            "import, export, demand, requirement, powered / unpowered, plants, aged, distressed), one per month, a "
            "bookkeeping line (running counts against a from-scratch insertion) per month; differing cells listed "
            "per line: %ld. The record is read through the original's own Write into memory. Not compared (not "
            "readable read-only): posted messages, history values, budget line items, demolitions, lot "
            "conditions, the poles the month's shuffle picks (the broken count is compared), deal flags. The "
            "shadow made at PostCityInit knows the original's unsaved values (its ordinance factor, pass import / "
            "export / demand and brokenFraction as its Init leaves them; the dirty rectangle from the removal and "
            "zoning messages, forwarded); one switched on mid-city marks what depends on them not exact until the "
            "original sets them again, and reruns a wire rebuild with the changed bits' box",
            src, g_cells);
    } else {
        Log("power shadow: requested (power_shadow from %s) but INACTIVE: it needs FreeCity4's simulator and the "
            "original power simulator (do not enable power); here the simulator is %s and power is %s",
            src, simulatorReplaced ? "ours" : "the original", powerReplaced ? "ours" : "the original");
    }
}

bool PowerShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                        int64_t data1, bool* finished) {
    if ((type != kMsgSimNewDay && type != kMsgSimIdle && type != kMsgSimNewMonth) || target == nullptr ||
        msg == nullptr || finished == nullptr) {
        return false;
    }
    // data1 is the simulator date's julian day for all three, SimIdle
    // included (power.md §4.3, PW2: read once per SimulateAgents /
    // SimulateIdleTime call).
    const int32_t julian = static_cast<int32_t>(data1);
    if (!g_testMode && type == kMsgSimNewDay && julian != g_settingsJulian) {
        g_settingsJulian = julian;
        ReadSettings();  // once per day (FreeCity4.ini may change between city loads)
    }
    if (!g_testMode && !g_enabled) return false;
    // Cheap rejection for every other agent: ours registers by name (AddAgent).
    if (!g_testMode && target != g_agent && (agentName == nullptr || strcmp(agentName, kAgentName) != 0)) return false;
    Listen();
    cISC4City* city = CurrentCity();
    cISC4PowerSimulator* sim = OriginalFor(city, target, agentName);
    // A shadow made here starts mid-city (PowerShadowCityInit makes the one
    // of a city start); a fresh test seam call is a city start unless it says
    // mid-city.
    const bool cityStart = g_testMode && g_test->fresh != 0 && g_test->midCity == 0;
    if (sim == nullptr || !EnsureShadow(city, sim, cityStart)) return false;
    if (!fc4_power_shadow_wants(g_shadow, type, g_event)) {
        g_lastPassed = true;
        return false;  // the original is idle: the caller delivers
    }
    g_event = false;  // a message during the delivery counts for the next one

    g_d = Delivery{};
    g_d.sim = sim;
    g_d.type = type;
    g_d.julian = julian;
    FillGame(city);
    const bool beforeClean = SehGuard("power shadow (before the original's update)", &BeforeStep, nullptr);
    if (!beforeClean && !g_d.begun) fc4_power_shadow_begin(g_shadow, type, julian, nullptr, nullptr);  // "no capture"
    *finished = target->DoMessage(msg);  // the original power simulator, unchanged
    const bool afterClean = SehGuard("power shadow (after the original's update)", &AfterStep, nullptr);
    if (!afterClean && !g_d.finished) {
        g_d.result = fc4_power_shadow_finish(g_shadow, nullptr, nullptr, &Sink, nullptr);
    }
    g_lastResult = g_d.result;
    g_before.ReleaseHeld();
    ClearGame();
    g_d = Delivery{};
    return true;  // delivered (compared or not)
}

namespace {
struct CityStart {
    cISC4City* city;
    cISC4PowerSimulator* sim;
};
void CityStartStep(void* ctx) {
    CityStart* c = static_cast<CityStart*>(ctx);
    EnsureShadow(c->city, c->sim, true);
}
}  // namespace

void PowerShadowCityInit() {
    if (g_testMode) return;
    ReadSettings();
    if (!g_enabled) return;
    ++g_generation;  // a new city: a new shadow
    Listen();
    cISC4City* city = CurrentCity();
    cISC4PowerSimulator* sim = city ? city->GetPowerSimulator() : nullptr;  // compiled slot 67, borrowed
    if (sim == nullptr || IsOurObject(sim)) return;  // the first delivery says why it is inactive
    CityStart c{city, sim};
    SehGuard("power shadow (city start)", &CityStartStep, &c);
}

}  // namespace fc4

intptr_t PowerShadowTestDeliver(const FC4PowerShadowTestInput* in) {
    if (in == nullptr) return -1;
    fc4::g_testMode = true;
    fc4::g_test = in;
    if (in->fresh != 0) {
        fc4::g_agentSim = nullptr;  // a fresh "game"
        fc4::g_agent = nullptr;
        fc4::g_shadowFailedFor = nullptr;
        fc4::DropShadow();
    }
    fc4::g_lastResult = -1;
    fc4::g_lastPassed = false;
    cFC4Message2Standard* msg = FC4MakeMessage(in->msgType, static_cast<intptr_t>(in->julian), 0, 0);
    bool finished = false;
    const bool taken = fc4::PowerShadowDeliver(in->target, nullptr, msg, in->msgType, in->julian, &finished);
    msg->Release();
    const bool passed = fc4::g_lastPassed;
    if (!taken && !passed) fc4::DropShadow();
    fc4::g_testMode = false;
    fc4::g_test = nullptr;
    if (passed) {
        // Delivered by the "caller" here: the original's idle step.
        cFC4Message2Standard* m = FC4MakeMessage(in->msgType, static_cast<intptr_t>(in->julian), 0, 0);
        in->target->DoMessage(m);
        m->Release();
        return -3;
    }
    if (!taken) return -1;
    return fc4::g_lastResult == -1 ? -2 : fc4::g_lastResult;
}
