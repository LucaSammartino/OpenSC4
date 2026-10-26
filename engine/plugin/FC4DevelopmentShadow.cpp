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

#include "FC4DevelopmentShadow.h"

#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4DevelopmentInterfaces.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4GameMap.h"
#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4WriteCapture.h"
#include "cFC4Development.h"
#include "cFC4GrowthDeveloper.h"
#include "fc4.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4App.h"
#include "cGZPersistResourceKey.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4City.h"
#include "cISC4GrowthDeveloper.h"
#include "cISC4SimGrid.h"
#include "cISC4Simulator.h"

#include <algorithm>
#include <climits>
#include <cstdarg>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <iterator>
#include <memory>
#include <string>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgSimIdle = 0x6695681A;
constexpr uint32_t kMsgZoningChange = 0x07699455;     // growth-developer.md §3.1
constexpr uint32_t kMsgStateChanged = 0xE8BDA2E7;     // growth-developer.md §3.1
constexpr uint32_t kMsgPreCityShutdown = 0x26D31EC2;  // the city is about to shut down
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_GrowthDeveloper = 0xE7CDD662;  // growth-developer.md §1
constexpr const char* kGrowthAgent = "cSC4GrowthDeveloper";            // growth-developer.md §1.2 step 9
constexpr const char* kBdsAgent = "cSC4BuildingDevelopmentSimulator";  // building-development.md §3.1
// The developer exemplar (growth-developer.md §1: instance = the developer type).
constexpr uint32_t kExemplarType = 0x6534284A;
constexpr uint32_t kDeveloperGroup = 0x67CD5FA1;
// Grid classes (FC4HostGrid.h).
constexpr uint32_t kClsBool = 0x49B9E601, kClsU8 = 0x49B9E602, kClsI8 = 0x49B9E603, kClsU16 = 0x49B9E604,
                   kClsI16 = 0x49B9E605, kClsF32 = 0x49B9E60A;
// fc4_growth_grid `which` (fc4.h).
constexpr uint32_t kGridD = 0, kGridCapacity = 1, kGridPopulation = 2, kGridMaxCapacity = 3, kGridZoned = 4;
// fc4::Log formats into a fixed buffer: longer lines are split.
constexpr size_t kMaxListed = 24;
// [choice] At most this many EstimateDesirability cells per developer update
// (one per tract; a larger city is sampled with a stride).
constexpr uint32_t kMaxEstimates = 4096;
// [choice] Our update is run to its end with at most this many extra calls
// (a SimIdle of kCatchUpMs each) when the original finished first.
constexpr uint32_t kCatchUpCalls = 4096;
constexpr uint32_t kCatchUpMs = 1000;

const char* const kReasonNames[6] = {"zone", "landValueRange", "landValueType", "lot", "lotState", "power"};

// ---- formatting ------------------------------------------------------------------------------------

std::string Fmt(const char* fmt, ...) {
    char b[512];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(b, sizeof(b), fmt, ap);
    va_end(ap);
    return b;
}

std::string DateText(uint32_t julian) {
    fc4_date d{};
    fc4_date_from_julian(static_cast<int32_t>(julian), &d);
    return Fmt("%04d-%02d-%02d", d.year, d.month, d.day_of_month);
}

void LogLine(const std::string& line) {
    LogShadowLine("development shadow (cont.)", reinterpret_cast<const uint8_t*>(line.data()), line.size());
}

struct Diffs {
    uint32_t count = 0;
    std::string listed;
    size_t shown = 0;

    void Add(const std::string& what) {
        ++count;
        if (shown >= kMaxListed) return;
        ++shown;
        if (!listed.empty()) listed += ", ";
        listed += what;
    }
    // "; mismatches=N[; differences (original/ours): ...]"
    std::string Tail() const {
        std::string s = Fmt("; mismatches=%u", count);
        if (count != 0) {
            s += "; differences (original/ours): " + listed;
            if (count > shown) s += ", ...";
        }
        return s;
    }
};

// ---- the lists (§5.4) ------------------------------------------------------------------------------

struct Item {
    int32_t x, z, stage;
    bool operator<(const Item& o) const {
        if (x != o.x) return x < o.x;
        if (z != o.z) return z < o.z;
        return stage < o.stage;
    }
    bool operator==(const Item& o) const { return x == o.x && z == o.z && stage == o.stage; }
};

// In order: the counts and every index; as a multiset (the industrial decline
// order, §5.4 step 3): what one side has and the other lacks.
void CompareLists(const std::vector<Item>& o, const std::vector<Item>& s, bool asSet, const char* name, Diffs& df) {
    if (o.size() != s.size()) {
        df.Add(Fmt("%s count %u/%u", name, static_cast<unsigned>(o.size()), static_cast<unsigned>(s.size())));
    }
    if (!asSet) {
        const size_t n = std::min(o.size(), s.size());
        for (size_t i = 0; i < n; ++i) {
            if (o[i] == s[i]) continue;
            df.Add(Fmt("%s[%u] (%d,%d) s%d/(%d,%d) s%d", name, static_cast<unsigned>(i), o[i].x, o[i].z, o[i].stage,
                       s[i].x, s[i].z, s[i].stage));
        }
        return;
    }
    std::vector<Item> a = o, b = s;
    std::sort(a.begin(), a.end());
    std::sort(b.begin(), b.end());
    size_t i = 0, j = 0;
    while (i < a.size() || j < b.size()) {
        if (j >= b.size() || (i < a.size() && a[i] < b[j])) {
            df.Add(Fmt("%s (%d,%d) s%d only in the original", name, a[i].x, a[i].z, a[i].stage));
            ++i;
        } else if (i >= a.size() || b[j] < a[i]) {
            df.Add(Fmt("%s (%d,%d) s%d only in ours", name, b[j].x, b[j].z, b[j].stage));
            ++j;
        } else {
            ++i;
            ++j;
        }
    }
}

// ---- grids -------------------------------------------------------------------------------------------

using Grid8 = cISC4SimGrid<uint8_t>;  // the slots that do not depend on the element type

void* GridOf(uint64_t token) { return reinterpret_cast<void*>(static_cast<uintptr_t>(token)); }

template <class T>
bool CopyValues(void* src, void* dst) {
    auto* s = static_cast<cISC4SimGrid<T>*>(src);
    auto* d = static_cast<cISC4SimGrid<T>*>(dst);
    const int32_t cx = s->GetTractCountX(), cz = s->GetTractCountZ();
    if (cx != d->GetTractCountX() || cz != d->GetTractCountZ()) return false;
    for (int32_t x = 0; x < cx; ++x) {
        for (int32_t z = 0; z < cz; ++z) d->SetTractValue(x, z, s->GetTractValue(x, z));
    }
    return true;
}

// A copy of one of the original's grids: a new game grid of the same class,
// instance and tract size (CreateGameGrid: one reference), with its values.
// 0 when the class is not one of the five the record holds or the geometry
// differs.
uint64_t CopyGrid(void* src, uint32_t cls) {
    if (src == nullptr) return 0;
    auto* g = static_cast<Grid8*>(src);
    const uint64_t t = CreateGameGrid("development shadow", cls, g->GetInstanceID(), g->GetTractSize());
    if (t == 0) return 0;
    bool ok = false;
    switch (cls) {
        case kClsI16: ok = CopyValues<int16_t>(src, GridOf(t)); break;
        case kClsU16: ok = CopyValues<uint16_t>(src, GridOf(t)); break;
        case kClsI8: ok = CopyValues<int8_t>(src, GridOf(t)); break;
        case kClsF32: ok = CopyValues<float>(src, GridOf(t)); break;
        case kClsU8:
        case kClsBool: ok = CopyValues<uint8_t>(src, GridOf(t)); break;
        default: break;
    }
    if (!ok) {
        ReleaseGameGrid(t);
        return 0;
    }
    return t;
}

struct GridResult {
    uint32_t tracts = 0, differing = 0;
    int64_t maxAbs = 0;
};

// Tract by tract (x outer, z inner) through GetTractValue on both sides.
template <class T>
GridResult CompareGrid(void* orig, void* ours, const char* name, Diffs& df) {
    GridResult r;
    if (orig == nullptr || ours == nullptr) {
        if (orig != ours) df.Add(Fmt("%s grid %s/%s", name, orig ? "present" : "null", ours ? "present" : "null"));
        return r;
    }
    auto* o = static_cast<cISC4SimGrid<T>*>(orig);
    auto* s = static_cast<cISC4SimGrid<T>*>(ours);
    const int32_t cx = o->GetTractCountX(), cz = o->GetTractCountZ();
    if (cx != s->GetTractCountX() || cz != s->GetTractCountZ() || o->GetTractShift() != s->GetTractShift()) {
        df.Add(Fmt("%s geometry %dx%d s%d/%dx%d s%d", name, cx, cz, o->GetTractShift(), s->GetTractCountX(),
                   s->GetTractCountZ(), s->GetTractShift()));
        return r;
    }
    for (int32_t x = 0; x < cx; ++x) {
        for (int32_t z = 0; z < cz; ++z) {
            ++r.tracts;
            const T a = o->GetTractValue(x, z), b = s->GetTractValue(x, z);
            if (a == b) continue;
            ++r.differing;
            const int64_t d = std::llabs(static_cast<int64_t>(a) - static_cast<int64_t>(b));
            r.maxAbs = std::max(r.maxAbs, d);
            df.Add(Fmt("%s(%d,%d) %lld/%lld", name, x, z, static_cast<long long>(a), static_cast<long long>(b)));
        }
    }
    return r;
}

std::string GridText(const char* name, const GridResult& r) {
    if (r.differing == 0) return Fmt("%s=ok", name);
    return Fmt("%s=%u differ (max |d| %lld)", name, r.differing, static_cast<long long>(r.maxAbs));
}

// ---- the original's record (§9) as an fc4_istream --------------------------------------------------

struct Record {
    bool ok = false;
    uint16_t version = 0;
    int32_t phantom = 0;
    uint32_t total = 0;
    uint32_t refs[5] = {};  // zoned, D, population, capacity, max capacity (object numbers)
};

Record ParseRecord(const WriteCapture& cap) {
    Record r;
    CaptureReader rd(cap.Bytes());
    if (!rd.U16(r.version) || r.version != 1) return r;
    if (!rd.I32(r.phantom) || !rd.U32(r.total)) return r;
    for (uint32_t& id : r.refs) {
        if (!rd.Ref(id)) return r;
    }
    r.ok = true;
    return r;
}

// The record's bytes for fc4_growth_read; each reference becomes a copy of
// the original's grid (one reference for the core).
struct RecordStream {
    const WriteCapture* cap = nullptr;
    size_t pos = 0;
    bool bad = false;
    uint32_t copies = 0, copyFailures = 0;

    bool Get(void* out, size_t n) {
        const std::vector<uint8_t>& b = cap->Bytes();
        if (bad || n > b.size() - pos) {
            bad = true;
            return false;
        }
        if (out != nullptr && n != 0) std::memcpy(out, b.data() + pos, n);
        pos += n;
        return true;
    }
};

RecordStream* Rs(void* u) { return static_cast<RecordStream*>(u); }
bool RsU8(void* u, uint8_t* o) { return Rs(u)->Get(o, 1); }
bool RsU16(void* u, uint16_t* o) { return Rs(u)->Get(o, 2); }
bool RsU32(void* u, uint32_t* o) { return Rs(u)->Get(o, 4); }
bool RsU64(void* u, uint64_t* o) { return Rs(u)->Get(o, 8); }
bool RsF32(void* u, float* o) { return Rs(u)->Get(o, 4); }
bool RsF64(void* u, double* o) { return Rs(u)->Get(o, 8); }
bool RsBytes(void* u, uint8_t* o, size_t n) { return Rs(u)->Get(o, n); }
bool RsSkip(void* u, size_t n) { return Rs(u)->Get(nullptr, n); }
bool RsStr(void* u, fc4_bytes_sink, void*) {  // the growth record holds no string
    Rs(u)->bad = true;
    return false;
}
int32_t RsError(void* u) { return Rs(u)->bad ? 1 : 0; }

bool RsRef(void* u, uint32_t, fc4_ref* out) {
    RecordStream* s = Rs(u);
    uint32_t id = 0;
    if (!s->Get(&id, 4)) return false;
    *out = fc4_ref{};
    if (id == 0) return true;
    uint32_t cls = 0;
    if (!s->Get(&cls, 4)) return false;
    out->class_id = cls;
    out->token = CopyGrid(s->cap->Grid(id), cls);
    if (out->token != 0) {
        ++s->copies;
    } else {
        ++s->copyFailures;
    }
    return true;
}

fc4_istream RecordIStream(RecordStream* rs) {
    fc4_istream in{};
    in.struct_size = sizeof(in);
    in.user = rs;
    in.get_u8 = &RsU8;
    in.get_u16 = &RsU16;
    in.get_u32 = &RsU32;
    in.get_u64 = &RsU64;
    in.get_f32 = &RsF32;
    in.get_f64 = &RsF64;
    in.get_bytes = &RsBytes;
    in.get_str = &RsStr;
    in.get_ref = &RsRef;
    in.skip = &RsSkip;
    in.error = &RsError;
    return in;
}

// ---- process statics of the shadow's own cores ----------------------------------------------------

fc4_growth_statics* g_statics = nullptr;
fc4_development_process* g_process = nullptr;

// [choice] The shadow's cores have process statics of their own (not the ones
// FreeCity4's development classes use), seeded like them.
fc4_growth_statics* ShadowStatics() {
    if (g_statics == nullptr) g_statics = fc4_growth_statics_create(DevelopmentSessionSeed(nullptr));
    return g_statics;
}

fc4_development_process* ShadowProcess() {
    if (g_process == nullptr) g_process = fc4_development_process_create(DevelopmentSessionSeed(nullptr));
    return g_process;
}

// ---- one growth developer's shadow core -----------------------------------------------------------

struct GrowthShadow {
    GrowthContext ctx;
    HostGridContext grids{"development shadow", Sw::DevelopmentMapWrites};
    fc4_growth_host host{};
    fc4_growth* core = nullptr;
    uint32_t depth = 0;

    GrowthShadow() {
        ctx.readOnly = true;
        host = MakeGrowthHost(&ctx, &grids);
    }
    ~GrowthShadow() {
        if (core != nullptr) fc4_growth_destroy(core);
        ReleaseGrowthContext(&ctx);
    }
    GrowthShadow(const GrowthShadow&) = delete;
    GrowthShadow& operator=(const GrowthShadow&) = delete;
    uint64_t Token() const { return TokenOf(this); }
};

// What a message hands the core (as cFC4GrowthDeveloper::DoMessage); the lot
// of a StateChanged is held by the shadow until the outermost call returns.
void MessageOf(cIGZMessage2* msg, fc4_growth_message* m, GrowthShadow* gs) {
    *m = fc4_growth_message{};
    m->msg_type = msg->GetType();
    cIGZMessage2Standard* s = nullptr;
    if (!msg->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&s)) || s == nullptr) return;
    FC4_ACQUIRED(reinterpret_cast<cIGZUnknown*>(s), "QueryInterface");
    m->data1 = static_cast<uint32_t>(s->GetData1());
    m->data2 = static_cast<uint32_t>(s->GetData2());
    m->data3 = static_cast<uint32_t>(s->GetData3());
    m->data4 = static_cast<uint32_t>(s->GetData4());
    if (m->msg_type == kMsgZoningChange) {
        if (const int32_t* r = static_cast<const int32_t*>(s->GetVoid2())) std::memcpy(m->rect, r, sizeof(m->rect));
    } else if (m->msg_type == kMsgStateChanged) {
        cIGZUnknown* lot = s->GetIGZUnknown();
        if (lot != nullptr && gs != nullptr) {
            if (IsOurObject(lot)) {
                lot->AddRef();
            } else {
                FC4_ADDREF(lot);
            }
            gs->ctx.held.push_back(lot);
        }
        m->unknown = TokenOf(lot);
    }
    FC4_RELEASE(reinterpret_cast<cIGZUnknown*>(s));
}

// One message to a shadow core (the held lots go when the outermost call returns).
bool CoreDeliver(GrowthShadow* gs, const fc4_growth_message& m) {
    if (gs == nullptr || gs->core == nullptr) return true;
    ++gs->depth;
    const bool r = fc4_growth_do_message(gs->core, &m);
    if (--gs->depth == 0) DropGrowthHeld(&gs->ctx);
    return r;
}

// ---- per original developer ------------------------------------------------------------------------

struct Dev {
    cIGZMessageTarget2* target = nullptr;  // the original's agent (identity only)
    cFC4ISC4GrowthDeveloper* orig = nullptr;  // one reference (QueryInterface); null: not shadowed
    int32_t type = 0;
    bool seenFinished = false;
    bool following = false;    // our core follows the original's update
    std::string failed;        // why this update is not compared (empty: compared)
    uint32_t startJulian = 0;
    std::unique_ptr<GrowthShadow> gs;
    // The current update.
    uint32_t calls = 0, messages = 0;
    std::string tuning;  // the tuning comparison at the baseline
    Diffs tuningDiffs;
    // The month bookkeeping compared at the baseline (empty: no previous core).
    std::string book;
    Diffs bookDiffs;
    uint32_t updates = 0;
};

struct State {
    bool enabled = false;
    int32_t settingsJulian = INT_MIN;
    bool subscribed = false;
    bool loggedOurs = false, loggedOursBds = false, loggedFault = false;
    bool disabledByFault = false;
    cISC4City* city = nullptr;  // identity of the city the entries belong to
    std::vector<std::unique_ptr<Dev>> devs;
    // The BDS shadow.
    std::unique_ptr<DevelopmentContext> bdsCtx;
    std::unique_ptr<HostGridContext> bdsGrids;
    fc4_development_host bdsHost{};
    fc4_development* bds = nullptr;
    bool bdsInit = false;
    bool catalogueCompared = false;
    uint32_t forwarded = 0;  // notifications handed to the cores (status)
    DWORD mainTid = 0;       // the thread of the first delivery (the simulator's)
    LotFlow lotFlow;         // the original's StateChanged flow since the last lot line (tooling)
};
State g;

void ReadSettings() { g.enabled = SettingInt("development_shadow", "FC4_DEVELOPMENT_SHADOW", 0) != 0; }

cISC4City* AppCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    FC4_ACQUIRED(sc4, "QueryInterface");
    cISC4City* city = sc4->GetCity();
    FC4_RELEASE(sc4);
    return city;
}

uint32_t Today(cISC4City* city) {
    cISC4Simulator* sim = city ? city->GetSimulator() : nullptr;  // city slot 50, borrowed
    return sim ? static_cast<uint32_t>(AsGame(sim)->GetSimDateNumber()) : 0;  // slot 8
}

void DropBds() {
    if (g.bds != nullptr) fc4_development_destroy(g.bds);
    g.bds = nullptr;
    if (g.bdsCtx) ReleaseDevelopmentContext(g.bdsCtx.get());
    g.bdsCtx.reset();
    g.bdsGrids.reset();
    g.bdsInit = false;
}

void DropDev(Dev& d) {
    d.gs.reset();
    if (d.orig != nullptr) FC4_RELEASE(reinterpret_cast<cIGZUnknown*>(d.orig));
    d.orig = nullptr;
}

// Every core and reference (city shutdown, a different city, the shadow off).
void DropAll() {
    for (auto& d : g.devs) DropDev(*d);
    g.devs.clear();
    DropBds();
    g.city = nullptr;
    g.catalogueCompared = false;
}

void DropAllGuarded(void*) { DropAll(); }

// ---- the notifications (ZoningChange, StateChanged, PreCityShutdown) -------------------------------

void ForwardStep(void* p) {
    cIGZMessage2* msg = static_cast<cIGZMessage2*>(p);
    for (auto& d : g.devs) {
        if (!d->gs || d->gs->core == nullptr) continue;
        fc4_growth_message m{};
        MessageOf(msg, &m, d->gs.get());
        CoreDeliver(d->gs.get(), m);
        ++d->messages;
    }
    ++g.forwarded;
}

class Target final : public cIGZMessageTarget2 {
public:
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid != kIID_cIGZMessageTarget2 && riid != GZIID_cIGZUnknown) return false;
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return static_cast<uint32_t>(InterlockedIncrement(&refs_)); }
    uint32_t Release() override {
        const LONG r = InterlockedDecrement(&refs_);
        return r > 0 ? static_cast<uint32_t>(r) : 0u;
    }
    bool DoMessage(cIGZMessage2* msg) override {
        if (msg == nullptr) return true;
        FC4_X87_SCOPE("development shadow DoMessage", msg->GetType());  // x87 leftover guard (FC4Fpu.h)
        if (g.mainTid == 0 || GetCurrentThreadId() != g.mainTid) return true;  // the cores are the main thread's
        const uint32_t type = msg->GetType();
        if (type == kMsgPreCityShutdown) {
            SehGuard("development shadow (city shutdown)", &DropAllGuarded, nullptr);
            return true;
        }
        if (type == kMsgStateChanged) {  // tooling: the month's lot line (data1 old state, data2 new)
            cIGZMessage2Standard* s = nullptr;
            if (msg->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&s)) && s != nullptr) {
                FC4_ACQUIRED(reinterpret_cast<cIGZUnknown*>(s), "QueryInterface");
                g.lotFlow.Note(static_cast<uint32_t>(s->GetData1()), static_cast<uint32_t>(s->GetData2()));
                FC4_RELEASE(reinterpret_cast<cIGZUnknown*>(s));
            }
        }
        if ((type == kMsgZoningChange || type == kMsgStateChanged) && !g.devs.empty()) {
            if (!SehGuard("development shadow (a notification)", &ForwardStep, msg)) g.disabledByFault = true;
        }
        return true;
    }

private:
    volatile LONG refs_ = 0;
};

Target& TheTarget() {
    static Target t;
    return t;
}

// [choice] Subscribed once, when the shadow first runs, and kept for the
// process (removing a notification inside a dispatch is avoided); without
// cores the messages do nothing.
void Subscribe() {
    if (g.subscribed) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    TheTarget().AddRef();  // our own reference (a static object)
    bool ok = true;
    for (const uint32_t t : {kMsgZoningChange, kMsgStateChanged, kMsgPreCityShutdown}) {
        ok = ms->AddNotification(static_cast<cIGZMessageTarget2*>(&TheTarget()), t) && ok;
    }
    g.subscribed = true;
    Log("development shadow: subscribed to ZoningChange, StateChanged and PreCityShutdown%s",
        ok ? "" : " (WARNING an AddNotification returned false)");
}

// ---- the growth developer steps -------------------------------------------------------------------

// The original developer behind an agent (a new entry on first sight), or
// nullptr when it is not one to shadow.
Dev* DevFor(cIGZMessageTarget2* target) {
    for (auto& d : g.devs) {
        if (d->target == target) return d->orig ? d.get() : nullptr;
    }
    auto d = std::make_unique<Dev>();
    d->target = target;
    void* p = nullptr;
    if (target->QueryInterface(kIID_GrowthDeveloper, &p) && p != nullptr) {
        cIGZUnknown* u = static_cast<cIGZUnknown*>(p);
        FC4_ACQUIRED(u, "QueryInterface");
        if (IsOurObject(u)) {
            if (!g.loggedOurs) {
                g.loggedOurs = true;
                Log("development shadow: inactive for the growth developers: they are FreeCity4's own (the shadow "
                    "compares against the ORIGINAL development unit: do not enable development)");
            }
            FC4_RELEASE(u);
        } else {
            d->orig = reinterpret_cast<cFC4ISC4GrowthDeveloper*>(u);
            // The developer type (0x1010 ... 0x4400) is slot 6, GetSubType;
            // slot 5, GetType, is the target zone (1, 2, 3), growth §10.1.
            d->type = static_cast<int32_t>(d->orig->GetSubType());  // slot 6
        }
    }
    g.devs.push_back(std::move(d));
    return g.devs.back()->orig ? g.devs.back().get() : nullptr;
}

std::vector<Item> OriginalList(cFC4ISC4GrowthDeveloper* o, bool decline) {
    fc4::GameVector<FC4GrowthItem>& v = decline ? o->GetDeclineList() : o->GetGrowthList();  // slots 12 / 11
    std::vector<Item> out;
    for (const FC4GrowthItem* it = v.begin(); it != v.end(); ++it) out.push_back(Item{it->x, it->z, it->maxStage});
    return out;
}

std::vector<Item> OurList(fc4_growth* core, uint32_t which) {
    const size_t n = fc4_growth_list(core, which, nullptr, 0);
    std::vector<fc4_growth_item> items(n);
    if (n != 0) fc4_growth_list(core, which, items.data(), n);
    std::vector<Item> out;
    out.reserve(n);
    for (const fc4_growth_item& it : items) out.push_back(Item{it.x, it.z, it.max_stage});
    return out;
}

// The tuning the update starts from (slots 9, 19-23 against the core's).
void CompareTuning(Dev& d) {
    Diffs& df = d.tuningDiffs;
    df = Diffs{};
    FC4ConstructionProfile op{};
    d.orig->GetConstructionInfo(op);  // slot 9: the first 18 bytes
    fc4_construction_profile sp{};
    fc4_growth_construction_info(d.gs->core, &sp);
    if (op.zone != sp.zone) df.Add(Fmt("zone %d/%d", op.zone, sp.zone));
    if (op.wealth != sp.wealth) df.Add(Fmt("wealth %u/%u", op.wealth, sp.wealth));
    if (op.purpose != sp.purpose) df.Add(Fmt("purpose %u/%u", op.purpose, sp.purpose));
    if (op.landValueMin != sp.land_value_min || op.landValueMax != sp.land_value_max) {
        df.Add(Fmt("land value %u-%u/%u-%u", op.landValueMin, op.landValueMax, sp.land_value_min, sp.land_value_max));
    }
    const int32_t ot = d.orig->GetDesirabilityThreshold(), st = fc4_growth_desirability_threshold(d.gs->core);
    if (ot != st) df.Add(Fmt("threshold %d/%d", ot, st));
    uint8_t caps[4] = {};
    fc4_growth_caps(d.gs->core, caps);
    const uint8_t oc[4] = {d.orig->GetPowerSupplyCap(), d.orig->GetWaterSupplyCap(), d.orig->GetFireCoverageCap(),
                           d.orig->GetParkCap()};
    for (int i = 0; i < 4; ++i) {
        if (oc[i] != caps[i]) df.Add(Fmt("cap %d %u/%u", 19 + i, oc[i], caps[i]));
    }
    // Slots 25 and 26, which the tract developer reads when it picks a
    // building (tract-developer.md §4.16 steps 3 and 8).
    // Slot 18, whose node the tract developer sizes a build by (§4.2): the
    // same game demand object.
    const void* osd = d.orig->GetSourceDemand();
    const void* ssd = FromToken(fc4_growth_source_demand(d.gs->core));
    if (osd != ssd) df.Add(Fmt("source demand %p/%p", osd, ssd));
    const int32_t om = d.orig->GetMonopolyUse();
    const uint32_t sm = fc4_growth_monopoly_use(d.gs->core);
    if (static_cast<uint32_t>(om) != sm) df.Add(Fmt("monopoly %d/%u", om, sm));
    std::vector<std::pair<uint32_t, uint32_t>> os, ss;  // (style, float bits)
    const GameMapStatus ms = WalkGameMap(d.orig->GetStylePercentages(), 64, [&](const GameMapNode& n) {
        os.emplace_back(n.key, n.value);
    });
    fc4_growth_styles(
        d.gs->core,
        [](void* c, uint32_t k, float v) {
            uint32_t bits = 0;
            std::memcpy(&bits, &v, sizeof(bits));
            static_cast<std::vector<std::pair<uint32_t, uint32_t>>*>(c)->emplace_back(k, bits);
        },
        &ss);
    if (ms != GameMapStatus::Ok) {
        df.Add(Fmt("styles unreadable (%s)", GameMapStatusName(ms)));
    } else if (os != ss) {
        df.Add(Fmt("styles %u entries/%u entries", static_cast<unsigned>(os.size()), static_cast<unsigned>(ss.size())));
    }
    d.tuning = df.count == 0 ? Fmt("tuning=ok (styles %u)", static_cast<unsigned>(ss.size()))
                             : Fmt("tuning=%u differ", df.count);
}

// The previous core against the record the original writes now (it saw the
// same ZoningChange and StateChanged messages since its baseline).
void CompareBookkeeping(Dev& d, const WriteCapture& cap, const Record& rec) {
    Diffs& df = d.bookDiffs;
    df = Diffs{};
    fc4_growth* c = d.gs->core;
    fc4_growth_state st{};
    st.struct_size = sizeof(st);
    fc4_growth_get_state(c, &st);
    const GridResult z =
        CompareGrid<uint8_t>(cap.Grid(rec.refs[0]), GridOf(fc4_growth_grid(c, kGridZoned)), "zoned", df);
    const GridResult k =
        CompareGrid<uint16_t>(cap.Grid(rec.refs[3]), GridOf(fc4_growth_grid(c, kGridCapacity)), "capacity", df);
    const GridResult m = CompareGrid<uint16_t>(cap.Grid(rec.refs[4]), GridOf(fc4_growth_grid(c, kGridMaxCapacity)),
                                               "maxCapacity", df);
    const int32_t total = fc4_growth_total_capacity(c);
    if (static_cast<int32_t>(rec.total) != total) df.Add(Fmt("total %d/%d", static_cast<int32_t>(rec.total), total));
    if (rec.phantom != st.last_phantom_sum) df.Add(Fmt("phantom %d/%d", rec.phantom, st.last_phantom_sum));
    d.book = Fmt("month bookkeeping (%u messages since %s): ", d.messages, DateText(d.startJulian).c_str()) +
             GridText("zoned", z) + " " + GridText("capacity", k) + " " + GridText("maxCapacity", m) +
             Fmt(" total=%d/%d phantom=%d/%d", static_cast<int32_t>(rec.total), total, rec.phantom,
                 st.last_phantom_sum);
}

// The original is at state 1 and has not run since StartMapUpdate: compare the
// previous core's month, then a new core from the original's record.
void Baseline(Dev& d, cISC4City* city) {
    d.failed.clear();
    d.calls = 0;
    d.book.clear();
    d.bookDiffs = Diffs{};
    d.tuning.clear();
    d.tuningDiffs = Diffs{};
    const uint32_t today = Today(city);
    WriteCapture cap;
    bool answered = false;
    if (!cap.Capture(reinterpret_cast<cIGZUnknown*>(d.orig), &answered)) {
        d.failed = answered ? "the original's Write failed" : "the original has no cIGZSerializable";
        d.gs.reset();
        d.startJulian = today;
        return;
    }
    const Record rec = ParseRecord(cap);
    if (!rec.ok) {
        d.failed = "the original's record is not version 1";
        d.gs.reset();
        d.startJulian = today;
        return;
    }
    if (d.gs && d.gs->core != nullptr) CompareBookkeeping(d, cap, rec);  // since the previous start
    d.gs.reset();
    d.messages = 0;
    d.startJulian = today;

    auto gs = std::make_unique<GrowthShadow>();
    gs->core = fc4_growth_create(&gs->host, ShadowStatics(), gs->Token());
    if (gs->core == nullptr) {
        d.failed = "the shadow core could not be created";
        return;
    }
    RecordStream rs;
    rs.cap = &cap;
    const fc4_istream in = RecordIStream(&rs);
    const bool read = fc4_growth_read(gs->core, &in);
    if (!read || rs.copyFailures != 0) {
        d.failed = read ? Fmt("%u of the original's grids could not be copied", rs.copyFailures)
                        : std::string("fc4_growth_read refused the original's record");
        return;  // the new core goes; the next month has no bookkeeping to compare
    }
    props::Exemplar ex;
    if (!ex.Load(kExemplarType, kDeveloperGroup, static_cast<uint32_t>(d.type))) {
        d.failed = Fmt("the developer exemplar 0x%08X/0x%08X/0x%08X does not load", kExemplarType, kDeveloperGroup,
                       static_cast<uint32_t>(d.type));
        return;
    }
    if (!fc4_growth_init(gs->core, props::ToCoreAll(ex))) {  // the props are consumed
        d.failed = "the shadow core's Init failed (a service is missing)";
        return;
    }
    d.gs = std::move(gs);
    CompareTuning(d);
    fc4_growth_start_map_update(d.gs->core);
    d.following = true;
}

// The original finished its update: ours to its end, then the comparison.
void Finish(Dev& d, cISC4City* city) {
    const std::string head = Fmt("development shadow growth 0x%04X update %s: ", static_cast<unsigned>(d.type),
                                 DateText(d.startJulian).c_str());
    d.following = false;
    ++d.updates;
    if (!d.failed.empty() || !d.gs || d.gs->core == nullptr) {
        LogLine(head + "INCOMPLETE (" + (d.failed.empty() ? std::string("no shadow core") : d.failed) +
                "); not compared");
        return;
    }
    fc4_growth* c = d.gs->core;
    uint32_t extra = 0;
    while (!fc4_growth_map_update_finished(c) && extra < kCatchUpCalls) {
        fc4_growth_message m{};
        m.msg_type = kMsgSimIdle;
        m.data3 = kCatchUpMs;
        CoreDeliver(d.gs.get(), m);
        ++extra;
    }
    fc4_growth_state st{};
    st.struct_size = sizeof(st);
    fc4_growth_get_state(c, &st);
    if (!fc4_growth_map_update_finished(c)) {
        LogLine(head + Fmt("INCOMPLETE (ours did not finish: state %u after %u calls); not compared",
                           st.update_state, d.calls + extra));
        return;
    }
    Diffs df;
    cFC4ISC4GrowthDeveloper* o = d.orig;
    // The maps (slots 13-16) and the total (17).
    cISC4SimGrid<int16_t>* od = o->GetDesirabilityMap();
    const GridResult rd = CompareGrid<int16_t>(od, GridOf(fc4_growth_grid(c, kGridD)), "D", df);
    const GridResult rp =
        CompareGrid<int16_t>(o->GetPopulationMap(), GridOf(fc4_growth_grid(c, kGridPopulation)), "population", df);
    const GridResult rk =
        CompareGrid<uint16_t>(o->GetCapacityMap(), GridOf(fc4_growth_grid(c, kGridCapacity)), "capacity", df);
    const GridResult rm = CompareGrid<uint16_t>(o->GetMaxCapacityMap(), GridOf(fc4_growth_grid(c, kGridMaxCapacity)),
                                                "maxCapacity", df);
    const int32_t ot = o->GetTotalCapacity(), stt = fc4_growth_total_capacity(c);
    if (ot != stt) df.Add(Fmt("total %d/%d", ot, stt));
    // The lists (slots 11 / 12) after §5.4: the industrial decline order comes
    // from the original's wall-clock salt (§7), so purposes 5-8 compare it as a multiset.
    const bool industrial = st.purpose >= 5 && st.purpose <= 8;
    const std::vector<Item> og = OriginalList(o, false), sg = OurList(c, 0);
    const std::vector<Item> odl = OriginalList(o, true), sdl = OurList(c, 1);
    const uint32_t before = df.count;
    CompareLists(og, sg, false, "growth", df);
    const uint32_t growthDiffs = df.count - before;
    CompareLists(odl, sdl, industrial, "decline", df);
    const uint32_t declineDiffs = df.count - before - growthDiffs;
    // EstimateDesirability (slot 29, AX) at one cell per tract.
    uint32_t estimates = 0, estimateDiffs = 0;
    if (od != nullptr && city != nullptr) {
        const int32_t cx = od->GetTractCountX(), cz = od->GetTractCountZ(), s = od->GetTractShift();
        const int32_t cellsX = static_cast<int32_t>(city->CellCountX()), cellsZ = static_cast<int32_t>(city->CellCountZ());
        const uint64_t tracts = static_cast<uint64_t>(std::max(cx, 0)) * static_cast<uint64_t>(std::max(cz, 0));
        const uint64_t stride = tracts > kMaxEstimates ? (tracts + kMaxEstimates - 1) / kMaxEstimates : 1;
        const int32_t half = s > 0 ? (1 << s) / 2 : 0;
        for (uint64_t i = 0; i < tracts; i += stride) {
            const int32_t tx = static_cast<int32_t>(i / static_cast<uint64_t>(cz));
            const int32_t tz = static_cast<int32_t>(i % static_cast<uint64_t>(cz));
            const int32_t x = std::min((tx << s) + half, cellsX - 1), z = std::min((tz << s) + half, cellsZ - 1);
            const int16_t a = o->EstimateDesirability(x, z);
            const int16_t b = static_cast<int16_t>(fc4_growth_estimate_desirability(c, x, z));
            ++estimates;
            if (a != b) {
                ++estimateDiffs;
                df.Add(Fmt("estimate(%d,%d) %d/%d", x, z, a, b));
            }
        }
    }
    std::string line = head;
    line += Fmt("tracts=%dx%d ", od ? od->GetTractCountX() : 0, od ? od->GetTractCountZ() : 0);
    line += GridText("D", rd) + " " + GridText("population", rp) + " " + GridText("capacity", rk) + " " +
            GridText("maxCapacity", rm);
    line += Fmt(" total=%d/%d growth=%u/%u%s decline=%u/%u%s (%s) estimate=%u cells%s %s", ot, stt,
                static_cast<unsigned>(og.size()), static_cast<unsigned>(sg.size()), growthDiffs ? " DIFFER" : "",
                static_cast<unsigned>(odl.size()), static_cast<unsigned>(sdl.size()), declineDiffs ? " DIFFER" : "",
                industrial ? "as a multiset: industrial salt" : "in order", estimates,
                estimateDiffs ? Fmt(" (%u differ)", estimateDiffs).c_str() : "", d.tuning.c_str());
    if (!d.book.empty()) line += "; " + d.book;
    line += Fmt("; ours: calls=%u extra=%u messages=%u sourceMaps=%u missing=%u city size=%.1f", d.calls, extra,
                d.messages, d.gs->ctx.sourceMaps, d.gs->ctx.missingSourceMaps, static_cast<double>(st.last_city_size));
    // One count for the line: the update's, the tuning's and the month's.
    Diffs all = df;
    for (const Diffs* extraDiffs : {&d.tuningDiffs, &d.bookDiffs}) {
        all.count += extraDiffs->count;
        if (!extraDiffs->listed.empty()) all.listed += (all.listed.empty() ? "" : ", ") + extraDiffs->listed;
        all.shown += extraDiffs->shown;
    }
    LogLine(line + all.Tail());
}

struct GrowthDelivery {
    Dev* d;
    cISC4City* city;
    cIGZMessage2* msg;
};

void GrowthBefore(void* p) {
    GrowthDelivery& gd = *static_cast<GrowthDelivery*>(p);
    Dev& d = *gd.d;
    const bool finished = d.orig->IsMapUpdateFinished();  // slot 28
    if (finished) {
        d.seenFinished = true;
        return;
    }
    if (d.following || !d.seenFinished || !d.failed.empty()) return;
    // The update started since the last delivery (the BDS's StartMapUpdate);
    // a failure is reported when the original finishes.
    Baseline(d, gd.city);
}

void GrowthAfter(void* p) {
    // SPEC-GAP: The state-18 iterator's bool versus EDX:EAX native capture ABI needs GD-GC29-2.
    GrowthDelivery& gd = *static_cast<GrowthDelivery*>(p);
    Dev& d = *gd.d;
    if (d.following && d.gs && d.gs->core != nullptr) {
        fc4_growth_message m{};
        MessageOf(gd.msg, &m, d.gs.get());
        CoreDeliver(d.gs.get(), m);  // the same message: ours follows the original call by call
        ++d.calls;
    }
    if (!d.orig->IsMapUpdateFinished()) return;
    if (d.following || !d.failed.empty()) {
        Finish(d, gd.city);
        d.failed.clear();
    }
    d.seenFinished = true;
}

// ---- the BDS steps -------------------------------------------------------------------------------

bool EnsureBds(cISC4City* city) {
    if (g.bdsInit) return g.bds != nullptr;
    g.bdsInit = true;
    g.bdsCtx = std::make_unique<DevelopmentContext>();
    g.bdsCtx->readOnly = true;
    g.bdsCtx->city = city;
    FC4_ADDREF(city);  // released by ReleaseDevelopmentContext
    g.bdsGrids = std::make_unique<HostGridContext>(HostGridContext{"development shadow", Sw::DevelopmentMapWrites});
    g.bdsHost = MakeDevelopmentHost(g.bdsCtx.get(), g.bdsGrids.get());
    const bool first = !fc4_development_catalogue_loaded(ShadowProcess());
    g.bds = fc4_development_create(&g.bdsHost, ShadowProcess(), TokenOf(g.bdsCtx.get()));
    if (g.bds == nullptr || !fc4_development_init(g.bds)) {
        Log("development shadow: the shadow BDS could not be created; the catalogue and suitability are not compared");
        DropBds();
        g.bdsInit = true;
        return false;
    }
    if (first) {
        Log("development shadow: catalogue loaded for the shadow BDS: %u building types, %u families",
            static_cast<unsigned>(fc4_development_catalogue_types(ShadowProcess(), 0, nullptr, 0)),
            static_cast<unsigned>(fc4_development_catalogue_types(ShadowProcess(), 1, nullptr, 0)));
    }
    return true;
}

std::vector<uint32_t> OurTypes() {
    const size_t n = fc4_development_catalogue_types(ShadowProcess(), 0, nullptr, 0);
    std::vector<uint32_t> v(n);
    if (n != 0) fc4_development_catalogue_types(ShadowProcess(), 0, v.data(), n);
    return v;
}

struct CapEntry {
    uint32_t key;
    uint32_t value;
    bool operator==(const CapEntry& o) const { return key == o.key && value == o.value; }
};

void CollectCap(void* ctx, uint32_t key, uint32_t value) {
    static_cast<std::vector<CapEntry>*>(ctx)->push_back(CapEntry{key, value});
}

bool SameFloat(float a, float b) { return std::memcmp(&a, &b, sizeof(a)) == 0; }

void CompareCatalogue(cFC4ISC4BuildingDevelopmentSimulator* ob, uint32_t julian) {
    Diffs df;
    // The types (slot 12): in order (hash order, §4.7.1.2) and as a set.
    uint32_t n = 0;
    const uint32_t total = ob->GetAllBuildingTypesArray(nullptr, n);
    std::vector<uint32_t> ot(total);
    n = total;
    if (total != 0) ob->GetAllBuildingTypesArray(ot.data(), n);
    ot.resize(std::min<size_t>(n, total));
    const std::vector<uint32_t> st = OurTypes();
    bool orderSame = ot.size() == st.size();
    for (size_t i = 0; orderSame && i < ot.size(); ++i) orderSame = ot[i] == st[i];
    if (!orderSame) {
        size_t i = 0;
        while (i < ot.size() && i < st.size() && ot[i] == st[i]) ++i;
        df.Add(Fmt("type order differs from index %u", static_cast<unsigned>(i)));
    }
    std::vector<uint32_t> a = ot, b = st;
    std::sort(a.begin(), a.end());
    std::sort(b.begin(), b.end());
    std::vector<uint32_t> onlyO, onlyS;
    std::set_difference(a.begin(), a.end(), b.begin(), b.end(), std::back_inserter(onlyO));
    std::set_difference(b.begin(), b.end(), a.begin(), a.end(), std::back_inserter(onlyS));
    for (uint32_t t : onlyO) df.Add(Fmt("type 0x%08X only in the original", t));
    for (uint32_t t : onlyS) df.Add(Fmt("type 0x%08X only in ours", t));
    // The profiles (slot 40, no insert).
    uint32_t profiles = 0, profileDiffs = 0;
    for (uint32_t t : a) {
        if (!std::binary_search(b.begin(), b.end(), t)) continue;
        const FC4BuildingProfile* op = ob->GetBuildingProfilePtr(t);
        fc4_building_profile sp{};
        std::vector<CapEntry> sc;
        const bool have = fc4_development_catalogue_profile(ShadowProcess(), t, &sp, &CollectCap, &sc);
        ++profiles;
        const uint32_t before = df.count;
        if (op == nullptr || !have) {
            if ((op != nullptr) != have) {
                df.Add(Fmt("0x%08X profile %s/%s", t, op ? "present" : "none", have ? "present" : "none"));
                ++profileDiffs;
            }
            continue;
        }
        if (op->purpose != sp.purpose) df.Add(Fmt("0x%08X purpose %u/%u", t, op->purpose, sp.purpose));
        if (op->wealth != sp.wealth) df.Add(Fmt("0x%08X wealth %u/%u", t, op->wealth, sp.wealth));
        if (op->capacity != sp.capacity) df.Add(Fmt("0x%08X capacity %u/%u", t, op->capacity, sp.capacity));
        if (op->duration != sp.value_499afa38) {
            df.Add(Fmt("0x%08X 0x499AFA38 %u/%u", t, op->duration, sp.value_499afa38));
        }
        if (op->flag27812836 != sp.flag_27812836) {
            df.Add(Fmt("0x%08X 0x27812836 %u/%u", t, op->flag27812836, sp.flag_27812836));
        }
        if (op->categoryFlags != sp.category_flags) {
            df.Add(Fmt("0x%08X categories 0x%X/0x%X", t, op->categoryFlags, sp.category_flags));
        }
        for (int k = 0; k < 3; ++k) {
            if (!SameFloat(op->occupantSize[k], sp.occupant_size[k])) {
                df.Add(Fmt("0x%08X size[%d] %g/%g", t, k, static_cast<double>(op->occupantSize[k]),
                           static_cast<double>(sp.occupant_size[k])));
            }
        }
        if (op->plopCost != sp.plop_cost) {
            df.Add(Fmt("0x%08X plop cost %lld/%lld", t, static_cast<long long>(op->plopCost),
                       static_cast<long long>(sp.plop_cost)));
        }
        static GameCapacityRead read;  // large: not on the stack
        ReadProfileCapacities(op, &read);
        if (read.status != GameMapStatus::Ok) {
            df.Add(Fmt("0x%08X capacity maps unreadable", t));
        } else {
            std::vector<CapEntry> oc;
            for (uint32_t i = 0; i < read.count; ++i) {
                oc.push_back(CapEntry{read.entries[i].devType, read.entries[i].amount});
            }
            std::sort(sc.begin(), sc.end(), [](const CapEntry& x, const CapEntry& y) { return x.key < y.key; });
            const uint32_t sm = sp.n_capacities > 0 ? 1u : 0u;  // Q11: one map when there are entries
            if (read.maps != sm || !(oc == sc)) {
                df.Add(Fmt("0x%08X capacity maps %u (%u entries)/%u (%u entries)", t, read.maps,
                           static_cast<unsigned>(oc.size()), sm, static_cast<unsigned>(sc.size())));
            }
        }
        if (df.count != before) ++profileDiffs;
    }
    // The exemplar keys (slot 14) of the common types.
    uint32_t keyDiffs = 0;
    for (uint32_t t : a) {
        if (!std::binary_search(b.begin(), b.end(), t)) continue;
        cGZPersistResourceKey ok;
        fc4_res_key sk{};
        const bool oh = ob->GetBuildingKeyFromType(t, ok);
        const bool sh = fc4_development_catalogue_key(ShadowProcess(), 0, t, &sk);
        if (oh != sh || (oh && (ok.type != sk.type_id || ok.group != sk.group || ok.instance != sk.instance))) {
            ++keyDiffs;
            df.Add(Fmt("0x%08X key %08X/%08X/%08X / %08X/%08X/%08X", t, ok.type, ok.group, ok.instance, sk.type_id,
                       sk.group, sk.instance));
        }
    }
    // The families (slot 13, hash order), each family's members (slot 18, the
    // member list in load order) and the hash set the tract developer draws
    // from (slot 19 into a fresh 193-bucket set, §4.16 step 8: its iteration
    // order is part of the contract).
    uint32_t famO = 0, famS = 0, memberDiffs = 0, setDiffs = 0;
    bool famOrderSame = false;
    if (StlNodeAllocator::Available()) {
        GameVector<uint32_t> ofv;
        ob->GetAllBuildingFamilyTypes(ofv);  // EAX is garbage (§8 slot 13)
        const std::vector<uint32_t> of(ofv.begin(), ofv.end());
        const size_t nf = fc4_development_catalogue_types(ShadowProcess(), 1, nullptr, 0);
        std::vector<uint32_t> sf(nf);
        if (nf != 0) fc4_development_catalogue_types(ShadowProcess(), 1, sf.data(), nf);
        famO = static_cast<uint32_t>(of.size());
        famS = static_cast<uint32_t>(sf.size());
        famOrderSame = of == sf;
        std::vector<uint32_t> fa = of, fb = sf;
        std::sort(fa.begin(), fa.end());
        std::sort(fb.begin(), fb.end());
        std::vector<uint32_t> fOnlyO, fOnlyS;
        std::set_difference(fa.begin(), fa.end(), fb.begin(), fb.end(), std::back_inserter(fOnlyO));
        std::set_difference(fb.begin(), fb.end(), fa.begin(), fa.end(), std::back_inserter(fOnlyS));
        for (uint32_t f : fOnlyO) df.Add(Fmt("family 0x%08X only in the original", f));
        for (uint32_t f : fOnlyS) df.Add(Fmt("family 0x%08X only in ours", f));
        if (!famOrderSame && fOnlyO.empty() && fOnlyS.empty()) df.Add("family order differs");
        for (uint32_t f : fa) {
            if (!std::binary_search(fb.begin(), fb.end(), f)) continue;
            GameVector<uint32_t> omv;
            ob->GetBuildingTypesByFamily(omv, f);
            const std::vector<uint32_t> om(omv.begin(), omv.end());
            const size_t nm = fc4_development_catalogue_family(ShadowProcess(), f, nullptr, 0);
            std::vector<uint32_t> sm(nm);
            if (nm != 0) fc4_development_catalogue_family(ShadowProcess(), f, sm.data(), nm);
            if (om != sm) {
                ++memberDiffs;
                std::vector<uint32_t> ma = om, mb = sm;
                std::sort(ma.begin(), ma.end());
                std::sort(mb.begin(), mb.end());
                df.Add(Fmt("family 0x%08X members %u/%u (%s)", f, static_cast<unsigned>(om.size()),
                           static_cast<unsigned>(sm.size()), ma == mb ? "same set, order differs" : "sets differ"));
                continue;
            }
            GameHashSetU32<> oset(193), sset(193);
            ob->GetBuildingTypeSetByFamily(oset, f);
            for (uint32_t t : sm) sset.insert(t);
            std::vector<uint32_t> oi, si;
            for (size_t k = 0; k < oset.bucket_count(); ++k) {
                for (auto* n = oset.bucket(k); n != nullptr; n = n->next) oi.push_back(n->value);
            }
            for (size_t k = 0; k < sset.bucket_count(); ++k) {
                for (auto* n = sset.bucket(k); n != nullptr; n = n->next) si.push_back(n->value);
            }
            if (oi != si) {
                ++setDiffs;
                df.Add(Fmt("family 0x%08X hash set order (%u/%u buckets)", f,
                           static_cast<unsigned>(oset.bucket_count()), static_cast<unsigned>(sset.bucket_count())));
            }
        }
    } else {
        df.Add("families not compared (no STL allocator)");
    }
    // The maximum density table (slot 45, ST0).
    uint32_t density = 0, densityDiffs = 0;
    for (int32_t zone = 0; zone < 16; ++zone) {
        for (uint32_t purpose = 1; purpose <= 8; ++purpose) {
            for (uint32_t wealth = 0; wealth <= 3; ++wealth) {
                const float o = ob->GetMaxCapacity(zone, purpose, wealth);  // abi: via cFC4ISC4BuildingDevelopmentSimulator
                const float s = fc4_development_catalogue_max_capacity(ShadowProcess(), static_cast<uint32_t>(zone),
                                                                      purpose, wealth);
                ++density;
                if (!SameFloat(o, s)) {
                    ++densityDiffs;
                    df.Add(Fmt("maxCapacity(%d,%u,%u) %g/%g", zone, purpose, wealth, static_cast<double>(o),
                               static_cast<double>(s)));
                }
            }
        }
    }
    LogLine(Fmt("development shadow catalogue %s: types=%u/%u (%s) profiles=%u (%u differ) keys=%u differ "
                "families=%u/%u (%s) members=%u differ hashSets=%u differ density=%u (%u differ)",
                DateText(julian).c_str(), static_cast<unsigned>(ot.size()), static_cast<unsigned>(st.size()),
                orderSame ? "same order" : "order DIFFERS", profiles, profileDiffs, keyDiffs, famO, famS,
                famOrderSame ? "same order" : "order DIFFERS", memberDiffs, setDiffs, density, densityDiffs) +
            df.Tail());
}

// building-development §4.7.3 BuildingFitsInZone(bz, cz).
bool FitsInZone(int32_t bz, int32_t cz) {
    if (bz == cz) return true;
    switch (cz) {
        case 2:
            return bz == 1;
        case 3:
            return bz == 1 || bz == 2;
        case 5:
            return bz == 4;
        case 6:
            return bz == 4 || bz == 5;
        case 9:
            return bz == 8;
        case 0x0B:
        case 0x0C:
        case 0x0F:
            return bz == 0;
        default:
            return false;
    }
}

// A cell the suitability comparison can use: zoned, or holding a lot.
struct SuitCell {
    int32_t x, z;
    int32_t zone;  // the zone manager's type, sign-extended (§4.8 check 1)
    bool lot;      // GetLot(x, z, true) is not null (check 2)
    bool state1;   // the lot's GetState() = 1 (check 5)
};

// Every zoned or lot-holding cell of the city, read through the shadow BDS's
// host (the calls the core's check makes), once per month.
std::vector<SuitCell> SuitabilityCells(int32_t cellsX, int32_t cellsZ) {
    std::vector<SuitCell> cells;
    const fc4_development_host& h = g.bdsHost;
    if (h.zone_type == nullptr || h.lot_at == nullptr || h.lot_state == nullptr) return cells;
    for (int32_t x = 0; x < cellsX; ++x) {
        for (int32_t z = 0; z < cellsZ; ++z) {
            const int32_t zone = h.zone_type(h.user, x, z);
            const uint64_t lot = h.lot_at(h.user, x, z);  // borrowed
            if (zone == 0 && lot == 0) continue;
            cells.push_back(SuitCell{x, z, zone, lot != 0, lot != 0 && h.lot_state(h.user, lot) == 1});
        }
    }
    return cells;
}

void CompareSuitability(cFC4ISC4BuildingDevelopmentSimulator* ob, cISC4City* city, uint32_t julian) {
    Diffs df;
    const std::vector<uint32_t> types = OurTypes();
    const int32_t cellsX = static_cast<int32_t>(city->CellCountX()), cellsZ = static_cast<int32_t>(city->CellCountZ());
    if (cellsX <= 0 || cellsZ <= 0) return;
    const std::vector<SuitCell> cells = SuitabilityCells(cellsX, cellsZ);
    uint32_t zoned = 0, lots = 0, state1 = 0;
    for (const SuitCell& c : cells) {
        zoned += c.zone != 0 ? 1 : 0;
        lots += c.lot ? 1 : 0;
        state1 += c.state1 ? 1 : 0;
    }
    uint32_t checked = 0, suitO = 0, suitS = 0, fitting = 0;
    uint32_t reasons[7] = {};  // the original's reasons 0..5, 6 = other
    std::vector<const SuitCell*> groups[3];
    for (size_t i = 0; i < types.size(); ++i) {
        const uint32_t t = types[i];
        fc4_building_profile bp{};
        if (!fc4_development_catalogue_profile(ShadowProcess(), t, &bp, nullptr, nullptr)) continue;
        // The profile a developer of the type's purpose and wealth gives
        // (slot 9; no developer: zone 0 and no land value range).
        FC4ConstructionProfile gp{};
        gp.wealth = bp.wealth;
        gp.purpose = bp.purpose;
        const uint32_t devType = fc4_development_developer_type(bp.purpose, bp.wealth);
        cISC4GrowthDeveloper* dev = devType ? ob->GetDeveloper(devType) : nullptr;  // slot 32, borrowed
        if (dev != nullptr) AsGame(dev)->GetConstructionInfo(gp);
        // [choice] (DG30) The cell: one whose zone fits the profile's zone
        // (check 1 passes), taken in turn from the cells with a lot in state
        // 1 (every check is reached), with a lot in another state (check 5
        // decides) and without a lot (check 2 decides), so that the later
        // checks are compared too; a type that fits no cell gets a cell
        // spread over the city by its index (check 1 decides).
        for (auto& gr : groups) gr.clear();
        for (const SuitCell& c : cells) {
            if (!FitsInZone(gp.zone, c.zone)) continue;
            groups[c.state1 ? 0 : (c.lot ? 1 : 2)].push_back(&c);
        }
        int32_t x = static_cast<int32_t>((i * 7) % static_cast<size_t>(cellsX));
        int32_t z = static_cast<int32_t>((i * 13) % static_cast<size_t>(cellsZ));
        for (size_t k = 0; k < 3; ++k) {
            const std::vector<const SuitCell*>& gr = groups[(i + k) % 3];
            if (gr.empty()) continue;
            const SuitCell* c = gr[(i / 3) % gr.size()];
            x = c->x;
            z = c->z;
            ++fitting;
            break;
        }
        gp.buildingType = t;
        gp.orientation = 0;
        gp.x = x;
        gp.z = z;
        fc4_construction_profile cp{};
        static_assert(sizeof(cp) == sizeof(gp), "ConstructionProfile layouts");
        std::memcpy(&cp, &gp, sizeof(cp));
        int32_t ro = -1, rs = -1;
        const bool okO = ob->IsSuitableForConstruction(gp, ro);  // slot 46
        const bool okS = fc4_development_is_suitable(g.bds, &cp, &rs);
        ++checked;
        suitO += okO ? 1 : 0;
        suitS += okS ? 1 : 0;
        if (!okO) ++reasons[(ro >= 0 && ro <= 5) ? ro : 6];
        if (okO != okS || (!okO && ro != rs)) {
            df.Add(Fmt("0x%08X (%d,%d) zone %d: %s/%s", t, x, z, gp.zone,
                       okO ? "suitable" : (ro >= 0 && ro <= 5 ? kReasonNames[ro] : "?"),
                       okS ? "suitable" : (rs >= 0 && rs <= 5 ? kReasonNames[rs] : "?")));
        }
    }
    std::string rtext;
    for (int r = 0; r < 7; ++r) {
        if (reasons[r] == 0) continue;
        rtext += Fmt("%s%s=%u", rtext.empty() ? "" : " ", r < 6 ? kReasonNames[r] : "other", reasons[r]);
    }
    LogLine(Fmt("development shadow suitability %s: profiles=%u at a fitting cell %u (cells: zoned %u, with a lot %u, "
                "lot state 1 %u) suitable=%u/%u (original's reasons: %s)%s",
                DateText(julian).c_str(), checked, fitting, zoned, lots, state1, suitO, suitS,
                rtext.empty() ? "-" : rtext.c_str(),
                ob->CheatIgnorePower() ? " (the original's ignore-power cheat is on; ours is off)" : "") +
            df.Tail());
}

struct BdsDelivery {
    cFC4ISC4BuildingDevelopmentSimulator* ob;
    cISC4City* city;
};

void BdsAfter(void* p) {
    BdsDelivery& bd = *static_cast<BdsDelivery*>(p);
    if (!EnsureBds(bd.city)) return;
    const uint32_t julian = Today(bd.city);
    if (!g.catalogueCompared) {
        g.catalogueCompared = true;
        CompareCatalogue(bd.ob, julian);
    }
    CompareSuitability(bd.ob, bd.city, julian);
    // Tooling: the original's lots and month, to set beside "development lots
    // ... (ours)" of a run with the development unit enabled.
    const std::string flow = g.lotFlow.Take();
    Log("development lots %s (the original's): %s; state changes %s; resources %d/%d stage %.2f",
        DateText(julian).c_str(), LotCensus(bd.city).c_str(), flow.c_str(), bd.ob->GetFreeConstructionResources(),
        bd.ob->GetTotalConstructionResources(), static_cast<double>(bd.ob->GetStagePopulationMultiplier()));
}

// The city the entries belong to: a different one drops them first.
cISC4City* CurrentCity() {
    cISC4City* city = AppCity();
    if (city != g.city) {
        DropAll();
        g.city = city;
    }
    return city;
}

}  // namespace

void DevelopmentShadowStartup(bool simulatorReplaced, bool developmentReplaced) {
    ReadSettings();
    if (!g.enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("development_shadow", "FC4_DEVELOPMENT_SHADOW", &src);
    if (simulatorReplaced && !developmentReplaced) {
        Log("development shadow: ON (development_shadow from %s): every ORIGINAL growth developer's monthly update "
            "is followed by a read-only FreeCity4 core started from the original's record (its five grids copied) "
            "and fed the same SimIdle / SimNewDay messages and every ZoningChange / StateChanged; when the original "
            "finishes: D, population, built and maximum capacity tract by tract, the total capacity, the growth "
            "list in order, the decline list (in order; industrial purposes 5-8 as a multiset), EstimateDesirability "
            "at one cell per tract, the construction info, threshold and caps, and the previous month's "
            "bookkeeping (zoned counts, capacities, total, phantom sum); one line per developer update. After the "
            "ORIGINAL BDS's SimNewMonth: a read-only BDS core (catalogue loaded once per process) compared once per "
            "city on the building types, every profile and the maximum density table, and every month on "
            "IsSuitableForConstruction for every catalogue type",
            src);
        Log("development shadow: not compared or assumed: the industrial decline order (a salt from the original's "
            "wall-clock generator, growth §7: compared as a multiset), the working desirability W (no getter; D "
            "after state 17 is compared), the order of inputs read within a state when the original's states 15 and "
            "18 take a different number of calls than ours, the tract developer's UpdatePopulation calls and the "
            "phantom demand's AddToDemandValue (ours makes neither), the style percentages, abandonment and "
            "subjective thresholds, the BDS's own month, growth, decline plans, resources and special buildings "
            "(the shadow BDS answers only IsSuitableForConstruction and its catalogue), the catalogue's families, "
            "names and budget item cost, the cheats (ours are off), and a developer whose update had started "
            "before the shadow saw it idle (its first update is skipped)");
    } else {
        Log("development shadow: requested (development_shadow from %s) but INACTIVE: it needs FreeCity4's "
            "simulator and the original development unit (do not enable development); here the simulator is %s "
            "and development is %s",
            src, simulatorReplaced ? "ours" : "the original", developmentReplaced ? "ours" : "the original");
    }
}

bool DevelopmentShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                              int64_t data1, bool* finished) {
    if (type == kMsgSimNewDay) {
        const int32_t day = static_cast<int32_t>(data1);
        if (day != g.settingsJulian) {
            g.settingsJulian = day;
            const bool was = g.enabled;
            ReadSettings();  // once per day (FreeCity4.ini may change between city loads)
            if (was && !g.enabled) SehGuard("development shadow (turned off)", &DropAllGuarded, nullptr);
        }
    }
    if (!g.enabled || g.disabledByFault || target == nullptr || msg == nullptr || finished == nullptr ||
        agentName == nullptr) {
        return false;
    }
    const bool growth = (type == kMsgSimIdle || type == kMsgSimNewDay) && strcmp(agentName, kGrowthAgent) == 0;
    const bool bdsMonth = type == kMsgSimNewMonth && strcmp(agentName, kBdsAgent) == 0;
    if (!growth && !bdsMonth) return false;
    if (g.mainTid == 0) g.mainTid = GetCurrentThreadId();
    if (GetCurrentThreadId() != g.mainTid) return false;
    cISC4City* city = CurrentCity();
    if (city == nullptr) return false;
    Subscribe();

    if (growth) {
        Dev* d = DevFor(target);
        if (d == nullptr) return false;
        GrowthDelivery gd{d, city, msg};
        bool ok = SehGuard("development shadow (before a growth developer delivery)", &GrowthBefore, &gd);
        *finished = target->DoMessage(msg);  // the original growth developer, unchanged
        ok = ok && SehGuard("development shadow (after a growth developer delivery)", &GrowthAfter, &gd);
        if (!ok) {
            g.disabledByFault = true;
            Log("development shadow: a fault in the shadow; it stops for this session");
            SehGuard("development shadow (dropping after a fault)", &DropAllGuarded, nullptr);
        }
        return true;
    }

    cISC4BuildingDevelopmentSimulator* raw = city->GetBuildingDevelopmentSimulator();  // borrowed
    if (raw == nullptr) return false;
    if (IsOurObject(raw)) {
        if (!g.loggedOursBds) {
            g.loggedOursBds = true;
            Log("development shadow: inactive for the BDS: it is FreeCity4's own (do not enable development)");
        }
        return false;
    }
    *finished = target->DoMessage(msg);  // the original BDS's month, unchanged
    BdsDelivery bd{AsGame(raw), city};
    if (!SehGuard("development shadow (after the BDS's month)", &BdsAfter, &bd)) {
        g.disabledByFault = true;
        Log("development shadow: a fault in the shadow; it stops for this session");
        SehGuard("development shadow (dropping after a fault)", &DropAllGuarded, nullptr);
    }
    return true;
}

}  // namespace fc4

// ---- test seam op 34 --------------------------------------------------------------------------------

intptr_t DevelopmentShadowTest(const FC4DevelopmentShadowTestInput* in) {
    using namespace fc4;
    if (in == nullptr) return -1;
    if (in->mode == 0) {
        if ((in->original == nullptr && in->originalCount != 0) || (in->ours == nullptr && in->oursCount != 0)) {
            return -1;
        }
        std::vector<Item> o, s;
        for (size_t i = 0; i < in->originalCount; ++i) {
            o.push_back(Item{in->original[3 * i], in->original[3 * i + 1], in->original[3 * i + 2]});
        }
        for (size_t i = 0; i < in->oursCount; ++i) {
            s.push_back(Item{in->ours[3 * i], in->ours[3 * i + 1], in->ours[3 * i + 2]});
        }
        Diffs df;
        CompareLists(o, s, in->asSet != 0, "list", df);
        Log("development shadow test: lists %u/%u %s%s", static_cast<unsigned>(o.size()), static_cast<unsigned>(s.size()),
            in->asSet ? "as a multiset" : "in order", df.Tail().c_str());
        return static_cast<intptr_t>(df.count);
    }
    if (in->mode == 1) {
        if (in->developer == nullptr) return -1;
        WriteCapture cap;
        if (!cap.Capture(in->developer)) return -2;
        const Record rec = ParseRecord(cap);
        if (!rec.ok) return -2;
        GrowthShadow gs;
        gs.core = fc4_growth_create(&gs.host, ShadowStatics(), gs.Token());
        if (gs.core == nullptr) return -2;
        RecordStream rs;
        rs.cap = &cap;
        const fc4_istream is = RecordIStream(&rs);
        const bool read = fc4_growth_read(gs.core, &is);
        fc4_growth_state st{};
        st.struct_size = sizeof(st);
        fc4_growth_get_state(gs.core, &st);
        Log("development shadow test: record v%u (%u bytes) phantom=%d total=%u -> read %d (copies %u, failed %u; "
            "core total=%d phantom=%d)",
            rec.version, static_cast<unsigned>(cap.Bytes().size()), rec.phantom, rec.total, read ? 1 : 0, rs.copies,
            rs.copyFailures, st.total_capacity, st.last_phantom_sum);
        return read ? 1 : 0;
    }
    if (in->mode == 2) {
        if (in->developer == nullptr) return -1;
        // Only slot 0 (QueryInterface) is called on the agent.
        auto* target = reinterpret_cast<cIGZMessageTarget2*>(in->developer);
        const Dev* d = DevFor(target);
        const intptr_t type = d != nullptr ? static_cast<intptr_t>(d->type) : -2;
        if (d != nullptr) {
            Log("development shadow test: developer 0x%04X (target zone %d), exemplar 0x%08X/0x%08X/0x%08X",
                static_cast<unsigned>(d->type), static_cast<int>(d->orig->GetType()), kExemplarType, kDeveloperGroup,
                static_cast<uint32_t>(d->type));
        }
        for (auto it = g.devs.begin(); it != g.devs.end(); ++it) {
            if ((*it)->target != target) continue;
            DropDev(**it);
            g.devs.erase(it);
            break;
        }
        return type;
    }
    if (in->mode == 3) {
        if (in->holder == nullptr) return -1;
        std::string ids;
        for (const props::EffectiveProperty& e : props::EffectiveProperties(in->holder)) {
            ids += Fmt(" 0x%08X%s", e.id, e.depth == 0 ? "" : Fmt("@%u", e.depth).c_str());
        }
        props::ToCoreAllStats st;
        props::CoreProps p(props::ToCoreAll(in->holder, nullptr, &st));
        const size_t n = p.Get() != nullptr ? fc4_props_count(p.Get()) : 0;
        Log("development shadow test: holder properties %u (own %u, inherited %u):%s", static_cast<unsigned>(n), st.own,
            st.inherited, ids.c_str());
        return static_cast<intptr_t>(n);
    }
    return -1;
}
