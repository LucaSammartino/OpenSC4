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

#include "FC4ResidentialShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"
#include "cFC4Residential.h"

#include "cGZPersistResourceKey.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cIGZString.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4ResidentialSimulator.h"
#include "cRZCOMDllDirector.h"

#include <algorithm>
#include <climits>
#include <cstring>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
// The interfaces the §9.2 reader asks a reference for.
constexpr uint32_t kIID_cISC4SimGrid = 0xC9BD939A;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4DepartmentBudget = 0x223E2B67;
constexpr const char* kAgentName = "cSC4ResidentialSimulator";  // residential.md §4.1 step 4
constexpr long kDefaultCells = 4;
constexpr long kMaxCells = 64;
// fc4::Log formats into a fixed buffer: longer shadow lines are split.
constexpr size_t kLogChunk = 2800;

// ---- the capture stream ------------------------------------------------------------------------------
//
// The original's Write goes into memory (the format of
// fc4_residential_capture): primitives little-endian, strings as a uint32
// length then the bytes, and references (WriteGZSerializable) as uint32
// objectID (index into the reference table + 1, 0 = null) then uint32
// classID, each object listed once with its grid, occupant and department
// identities. The identities are borrowed: the original keeps its grids,
// buildings and departments alive across the delivery. Process-lifetime
// objects (the reference count is not kept).
class CaptureStream final : public cFC4ISC4DBSegmentOStream {
public:
    void Reset() {
        bytes_.clear();
        refs_.clear();
        ids_.clear();
        good_ = true;
    }

    fc4_residential_capture View(bool writeOk) const {
        fc4_residential_capture c{};
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
            fc4_residential_capture_ref r{};
            r.grid = Borrowed(obj, kIID_cISC4SimGrid);
            r.occupant = Borrowed(obj, kIID_cISC4Occupant);
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
        good_ = false;  // not part of the residential record (§9)
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
    std::vector<fc4_residential_capture_ref> refs_;
    std::vector<const void*> ids_;
    bool good_ = true;
};

CaptureStream g_before;
CaptureStream g_after;

// ---- settings and run state (main thread only) ---------------------------------------------------

bool g_enabled = false;
long g_cells = kDefaultCells;
int32_t g_settingsJulian = INT_MIN;

// Test seam (op 25).
bool g_testMode = false;
const FC4ResidentialShadowTestInput* g_test = nullptr;

// The original simulator's agent (its cIGZMessageTarget2), per simulator.
const void* g_agentSim = nullptr;
cIGZMessageTarget2* g_agent = nullptr;
bool g_loggedOurs = false;
bool g_loggedNoSerializable = false;

// The shadow core, per city and original simulator. Its reads go to g_ctx,
// which holds the city (borrowed) for one delivery.
ResidentialContext g_ctx;
const HostGridContext g_gridCtx{"residential shadow", Sw::ResidentialGridWrites};
fc4_residential_shadow* g_shadow = nullptr;
const void* g_shadowCity = nullptr;
const void* g_shadowSim = nullptr;
long g_shadowCells = 0;
const void* g_shadowFailedFor = nullptr;

// The delivery being wrapped.
struct Delivery {
    cISC4ResidentialSimulator* sim = nullptr;
    int32_t julian = 0;
    int32_t begin = -1;  // fc4_residential_shadow_begin's answer
    bool begun = false;
    bool finished = false;
    int32_t result = -1;
};
Delivery g_d;
int32_t g_lastResult = -1;

void ReadSettings() {
    g_enabled = SettingInt("residential_shadow", "FC4_RESIDENTIAL_SHADOW", 0) != 0;
    long n = SettingInt("residential_shadow_cells", "FC4_RESIDENTIAL_SHADOW_CELLS", kDefaultCells);
    if (n <= 0) n = kDefaultCells;
    if (n > kMaxCells) n = kMaxCells;
    g_cells = n;
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

// The original residential simulator if `target` is its agent: its
// cIGZMessageTarget2 interface, or failing that (logged) an agent registered
// under its name.
cISC4ResidentialSimulator* OriginalFor(cISC4City* city, cIGZMessageTarget2* target, const char* agentName) {
    if (city == nullptr || target == nullptr) return nullptr;
    cISC4ResidentialSimulator* raw = city->GetResidentialSimulator();  // compiled slot 68, borrowed
    if (raw == nullptr) return nullptr;
    if (IsOurObject(raw)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("residential shadow: inactive: the city's residential simulator is FreeCity4's own (the shadow "
                "compares against the ORIGINAL residential simulator: do not enable residential)");
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
        Log("residential shadow: the original residential simulator %p has agent %p (%s)", static_cast<void*>(raw),
            static_cast<void*>(g_agent), qi ? "its cIGZMessageTarget2 interface" : "no interface: matched by name");
    }
    const bool same = g_agent != nullptr && target == g_agent;
    const bool byName = g_agent == nullptr && agentName != nullptr && strcmp(agentName, kAgentName) == 0;
    return (same || byName) ? raw : nullptr;
}

// The city the read callbacks use, borrowed for one delivery (the test
// seam's city answers GetResidentialSimulator only: the reads see no city).
void FillContext(cISC4City* city) {
    g_ctx.readOnly = true;
    g_ctx.gated = false;  // the original's inputs as they are
    g_ctx.city = g_testMode ? nullptr : city;
}

void ClearContext() { g_ctx.city = nullptr; }

void DropShadow() {
    fc4_residential_shadow_destroy(g_shadow);
    g_shadow = nullptr;
    g_shadowCity = nullptr;
    g_shadowSim = nullptr;
}

bool EnsureShadow(const void* city, const void* sim) {
    if (g_shadow != nullptr && g_shadowCity == city && g_shadowSim == sim && g_shadowCells == g_cells) return true;
    DropShadow();
    const fc4_residential_host host = MakeResidentialReadOnlyHost(&g_ctx, &g_gridCtx);  // copied by the core
    g_shadow = fc4_residential_shadow_create(&host, static_cast<uint32_t>(g_cells));
    if (g_shadow == nullptr) {
        if (g_shadowFailedFor != sim) {
            g_shadowFailedFor = sim;
            Log("residential shadow: the shadow core could not be created; deliveries pass through uncompared");
        }
        return false;
    }
    g_shadowCity = city;
    g_shadowSim = sim;
    g_shadowCells = g_cells;
    Log("residential shadow: comparing with the original residential simulator %p (city %p); differing cells "
        "listed per grid: %ld",
        sim, city, g_cells);
    return true;
}

// The original's record through its own Write (cIGZSerializable).
fc4_residential_capture Capture(CaptureStream& s) {
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
        Log("residential shadow: the original residential simulator does not answer cIGZSerializable; its record "
            "(and with it the grids) cannot be read: nothing is compared");
    }
    return s.View(ok);
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
            Log("residential shadow (cont.) %.*s", static_cast<int>(n), p + off);
        }
        off += n;
        first = false;
    }
}

void BeforeStep(void*) {
    const fc4_residential_capture c = Capture(g_before);
    // The reason, if not run, comes with finish.
    g_d.begin = fc4_residential_shadow_begin(g_shadow, g_d.julian, &c);
    g_d.begun = true;
}

void AfterStep(void*) {
    if (g_d.begin == -2) {
        // State 10: the original waited for SimNewMonth; nothing to compare
        // and nothing logged (the finish only closes the delivery).
        g_d.result = fc4_residential_shadow_finish(g_shadow, nullptr, nullptr, &Sink, nullptr);
        g_d.finished = true;
        return;
    }
    const fc4_residential_capture c = Capture(g_after);
    fc4_residential_shadow_scalars sc{};
    sc.struct_size = sizeof(sc);
    const bool haveScalars = c.ok != 0;
    if (haveScalars) {
        cFC4ISC4ResidentialSimulator* r = AsGame(g_d.sim);
        sc.population = r->GetPopulation();  // slot 16
        sc.global_eq = r->GetGlobalEQ();     // slot 32 (ST0)
        sc.global_hq = r->GetGlobalHQ();     // slot 33 (ST0)
        sc.global_le = r->GetGlobalLE();     // slot 34 (AL only; widened here)
    }
    g_d.result = fc4_residential_shadow_finish(g_shadow, &c, haveScalars ? &sc : nullptr, &Sink, nullptr);
    g_d.finished = true;
}

}  // namespace

void ResidentialShadowStartup(bool simulatorReplaced, bool residentialReplaced) {
    ReadSettings();
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("residential_shadow", "FC4_RESIDENTIAL_SHADOW", &src);
    if (simulatorReplaced && !residentialReplaced) {
        Log("residential shadow: ON (residential_shadow from %s): at each SimNewDay of the ORIGINAL residential "
            "simulator (one state of its monthly cycle per day), FreeCity4's state on its record read just before "
            "(its own Write into memory; states 1, 2 and 4 run just after with the original's progress), compared "
            "just after with its 25 grids cell by cell, its saved fields, the occupant properties and local funding, "
            "and its population, global EQ / HQ and LE (slots 16, 32-34); one line per delivery outside state 10; "
            "differing cells listed per grid: %ld",
            src, g_cells);
        fc4_residential_shadow_describe(&Sink, nullptr);
    } else {
        Log("residential shadow: requested (residential_shadow from %s) but INACTIVE: it needs FreeCity4's "
            "simulator and the original residential simulator (do not enable residential); here the simulator is %s "
            "and residential is %s",
            src, simulatorReplaced ? "ours" : "the original", residentialReplaced ? "ours" : "the original");
    }
}

bool ResidentialShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                              int64_t data1, bool* finished) {
    if (type != kMsgSimNewDay || target == nullptr || msg == nullptr || finished == nullptr) return false;
    if (!g_testMode) {
        const int32_t day = static_cast<int32_t>(data1);
        if (day != g_settingsJulian) {
            g_settingsJulian = day;
            ReadSettings();  // once per day (FreeCity4.ini may change between city loads)
        }
        if (!g_enabled) return false;
        // Cheap rejection for every other agent: ours registers by name (AddAgent).
        if (target != g_agent && (agentName == nullptr || strcmp(agentName, kAgentName) != 0)) return false;
    }
    cISC4City* city = CurrentCity();
    cISC4ResidentialSimulator* sim = OriginalFor(city, target, agentName);
    if (sim == nullptr || !EnsureShadow(city, sim)) return false;

    g_d = Delivery{};
    g_d.sim = sim;
    g_d.julian = static_cast<int32_t>(data1);
    FillContext(city);
    const bool beforeClean = SehGuard("residential shadow (before the original's day)", &BeforeStep, nullptr);
    if (!beforeClean && !g_d.begun) {
        g_d.begin = fc4_residential_shadow_begin(g_shadow, g_d.julian, nullptr);  // "no capture"
    }
    ClearContext();
    const uint16_t nativeCw = X87ControlWord();
    *finished = target->DoMessage(msg);  // the original residential simulator, unchanged
    const uint16_t returnCw = X87ControlWord();
    FillContext(city);
    const bool afterClean = SehGuard("residential shadow (after the original's day)", &AfterStep, nullptr);
    if ((nativeCw & 0x0300u) != 0 || nativeCw != returnCw) {
        Log("views precision residential: day=%d native=0x%04X return=0x%04X replay=0x%04X",
            g_d.julian, nativeCw, returnCw, X87ControlWord());
    }
    if (!afterClean && !g_d.finished) {
        g_d.result = fc4_residential_shadow_finish(g_shadow, nullptr, nullptr, &Sink, nullptr);
    }
    g_lastResult = g_d.result;
    ClearContext();
    g_d = Delivery{};
    return true;  // delivered (compared or not)
}

}  // namespace fc4

intptr_t ResidentialShadowTestDeliver(const FC4ResidentialShadowTestInput* in) {
    if (in == nullptr) return -1;
    fc4::g_testMode = true;
    fc4::g_test = in;
    fc4::g_agentSim = nullptr;  // each call is a fresh "game"
    fc4::g_agent = nullptr;
    fc4::g_shadowFailedFor = nullptr;
    fc4::DropShadow();
    fc4::g_lastResult = -1;
    cFC4Message2Standard* msg = FC4MakeMessage(in->msgType, static_cast<intptr_t>(in->julian), 0, 0);
    bool finished = false;
    const bool taken = fc4::ResidentialShadowDeliver(in->target, nullptr, msg, in->msgType, in->julian, &finished);
    msg->Release();
    fc4::DropShadow();
    fc4::g_agentSim = nullptr;
    fc4::g_agent = nullptr;
    fc4::g_testMode = false;
    fc4::g_test = nullptr;
    if (!taken) return -1;
    return fc4::g_lastResult == -1 ? -2 : fc4::g_lastResult;
}
