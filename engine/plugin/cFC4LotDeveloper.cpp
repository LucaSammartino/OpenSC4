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

#include "cFC4LotDeveloper.h"

#include "FC4LotDevShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4TractInterfaces.h"

#include "SC4List.h"
#include "SC4Point.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cISC4City.h"
#include "cISC4Lot.h"
#include "cISC4LotBaseTextureOccupant.h"
#include "cISC4LotConfiguration.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantFilter.h"
#include "cISC4OccupantManager.h"
#include "cISC4PropManager.h"
#include "cS3DVector3.h"
#include "cSC4LotConfigurationObject.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <mutex>
#include <string>
#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4Lot = 0xE818FCED;
constexpr uint32_t kIID_cISC4LotBaseTextureOccupant = 0x097F6D98;

// §3.2.
constexpr uint32_t kMsgOccupantInserted = 0x99EF1142;
constexpr uint32_t kMsgOccupantRemoved = 0x99EF1143;
constexpr uint32_t kMsgLotStateChanged = 0xE8BDA2E7;
constexpr uint32_t kMsgLotConditionChanged = 0xE99B3D4D;
constexpr uint32_t kMsgSimNewDay = 0x66956814;

// The lot developer the city runs (Init to Shutdown), for the month line.
std::atomic<cFC4LotDeveloper*> g_live{nullptr};

// Live instances, for the test seam (op 52).
std::mutex g_instancesLock;
std::vector<cFC4LotDeveloper*> g_instances;

// §10.1 [choice]: the session seed S, as the tract developer's: `lotdev_seed`
// / FC4_LOTDEV_SEED, else the cRZTime clock (seconds since 1901-01-01).
uint32_t SessionSeed(bool* fixed) {
    const std::string s = fc4::Setting("lotdev_seed", "FC4_LOTDEV_SEED");
    if (!s.empty()) {
        char* end = nullptr;
        const unsigned long v = std::strtoul(s.c_str(), &end, 0);
        if (end != s.c_str()) {
            *fixed = true;
            return static_cast<uint32_t>(v);
        }
    }
    *fixed = false;
    constexpr int64_t kRzTimeUnixOffset = 2177452800;  // 1901-01-01 to 1970-01-01
    return static_cast<uint32_t>(static_cast<int64_t>(std::time(nullptr)) + kRzTimeUnixOffset);
}

struct StaticsHolder {
    fc4_lotdev_statics* statics = nullptr;
    uint32_t seed = 0;
    bool fixed = false;
};

StaticsHolder& Statics() {
    static StaticsHolder h;
    static std::once_flag once;
    std::call_once(once, [] {
        h.seed = SessionSeed(&h.fixed);
        h.statics = fc4_lotdev_statics_create(h.seed);  // never destroyed: shared by every core of the process
    });
    return h;
}

// Write's diagnostics: the record goes through the game stream unchanged;
// the first put that returned false (which, at which byte) is noted.
struct TracedOut {
    const fc4_ostream* inner = nullptr;
    uint32_t bytes = 0;
    const char* firstFailed = nullptr;
    uint32_t failedAt = 0;
    uint32_t failures = 0;
    bool Note(bool ok, const char* what, uint32_t n) {
        if (!ok) {
            if (firstFailed == nullptr) {
                firstFailed = what;
                failedAt = bytes;
            }
            ++failures;
        }
        bytes += n;
        return ok;
    }
};
TracedOut& T(void* u) { return *static_cast<TracedOut*>(u); }
bool TU8(void* u, uint8_t v) { return T(u).Note(T(u).inner->put_u8(T(u).inner->user, v), "u8", 1); }
bool TU16(void* u, uint16_t v) { return T(u).Note(T(u).inner->put_u16(T(u).inner->user, v), "u16", 2); }
bool TU32(void* u, uint32_t v) { return T(u).Note(T(u).inner->put_u32(T(u).inner->user, v), "u32", 4); }
bool TU64(void* u, uint64_t v) { return T(u).Note(T(u).inner->put_u64(T(u).inner->user, v), "u64", 8); }
bool TF32(void* u, float v) { return T(u).Note(T(u).inner->put_f32(T(u).inner->user, v), "f32", 4); }
bool TF64(void* u, double v) { return T(u).Note(T(u).inner->put_f64(T(u).inner->user, v), "f64", 8); }
bool TBytes(void* u, const uint8_t* d, size_t n) {
    return T(u).Note(T(u).inner->put_bytes(T(u).inner->user, d, n), "bytes", static_cast<uint32_t>(n));
}
bool TStr(void* u, const uint8_t* d, size_t n) {
    return T(u).Note(T(u).inner->put_str(T(u).inner->user, d, n), "str", static_cast<uint32_t>(4 + n));
}
bool TRef(void* u, const fc4_ref* r) { return T(u).Note(T(u).inner->put_ref(T(u).inner->user, r), "ref", 4); }
int32_t TError(void* u) { return T(u).inner->error(T(u).inner->user); }

fc4_ostream MakeTraced(TracedOut* t) {
    fc4_ostream s{};
    s.struct_size = sizeof(s);
    s.user = t;
    s.put_u8 = &TU8;
    s.put_u16 = &TU16;
    s.put_u32 = &TU32;
    s.put_u64 = &TU64;
    s.put_f32 = &TF32;
    s.put_f64 = &TF64;
    s.put_bytes = &TBytes;
    s.put_str = &TStr;
    s.put_ref = &TRef;
    s.error = &TError;
    return s;
}

template <class T>
T* Query(cIGZUnknown* u, uint32_t iid) {
    if (u == nullptr) return nullptr;
    void* p = nullptr;
    if (!u->QueryInterface(iid, &p) || p == nullptr) return nullptr;
    T* t = static_cast<T*>(p);
    FC4_ACQUIRED(reinterpret_cast<cIGZUnknown*>(t), "QueryInterface");
    return t;
}

using fc4::LotDevObjectAt;
using fc4::LotDevObjectOf;

}  // namespace

namespace fc4 {

const fc4_lotdev_statics* LotDevStatics(uint32_t* seed, bool* fixed) {
    StaticsHolder& h = Statics();
    if (seed != nullptr) *seed = h.seed;
    if (fixed != nullptr) *fixed = h.fixed;
    return h.statics;
}

void LotDevMonthLine(int32_t julian) {
    cFC4LotDeveloper* t = g_live.load();
    if (t == nullptr) return;
    t->LogMonth(julian);
}

}  // namespace fc4

// ---- object ---------------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4LotDeveloper() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4LotDeveloper*>(new cFC4LotDeveloper()));
}

cFC4LotDeveloper::cFC4LotDeveloper() {
    InitializeCriticalSection(&lock_);
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    ctx_.lotLog = fc4::SettingInt("lotdev_lot_log", "FC4_LOTDEV_LOT_LOG", 0) != 0;
    fc4::LotDevStatics(&seed_, &seedFixed_);
    CreateCore();
    {
        std::lock_guard<std::mutex> g(g_instancesLock);
        g_instances.push_back(this);
    }
    fc4::Log("cFC4LotDeveloper(%p) created, core %p, session seed 0x%08X (%s)%s", static_cast<void*>(Primary()),
             static_cast<void*>(core_), seed_, seedFixed_ ? "lotdev_seed" : "cRZTime",
             ctx_.lotLog ? ", lotdev_lot_log on" : "");
}

// The core over this object's table (the test seam's callbacks answer
// first, through the context), with the process's statics.
void cFC4LotDeveloper::CreateCore() {
    host_ = fc4::MakeLotDevHost(&ctx_);
    core_ = fc4_lotdev_create(&host_, fc4::LotDevStatics());
    if (core_ == nullptr) fc4::Log("cFC4LotDeveloper: core creation failed");
}

cFC4LotDeveloper::~cFC4LotDeveloper() {
    {
        std::lock_guard<std::mutex> g(g_instancesLock);
        for (size_t i = 0; i < g_instances.size(); ++i) {
            if (g_instances[i] == this) {
                g_instances.erase(g_instances.begin() + static_cast<std::ptrdiff_t>(i));
                break;
            }
        }
    }
    cFC4LotDeveloper* self = this;
    g_live.compare_exchange_strong(self, nullptr);
    // The night deque's references go with the core (fc4_lotdev_destroy
    // releases them through the table while the services are still held).
    fc4_lotdev_destroy(core_);
    core_ = nullptr;
    fc4::LotDevReleaseHeld(&ctx_);
    if (ctx_.agentLevels != 0 && ctx_.city != nullptr) {
        cFC4ISC4Simulator* sim = fc4::AsGame(ctx_.city->GetSimulator());
        if (sim != nullptr) sim->RemoveAgentAllLevels(ctx_.target);
        ctx_.agentLevels = 0;
    }
    fc4::LotDevReleaseServices(&ctx_);
    fc4::Log("cFC4LotDeveloper destroyed");
    DeleteCriticalSection(&lock_);
}

// §1.1: which sub-object answers is the core's decision; anything else is
// false and the out pointer is left untouched.
bool cFC4LotDeveloper::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    const uint32_t which = fc4_lotdev_query_interface(riid);
    if (which == 0) return false;
    if (which == 2) {
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
    } else if (which == 3) {
        *ppvObj = static_cast<cIGZSerializable*>(this);
    } else {
        *ppvObj = Primary();  // 0x00000001 and 0x4818E5D2
    }
    ++refCount_;
    return true;
}

uint32_t cFC4LotDeveloper::AddRef() { return ++refCount_; }

// §1.4 Release: count > 1 → decrement; otherwise Shutdown, then destroy.
// [quirk] 0 in both cases.
uint32_t cFC4LotDeveloper::Release() {
    if (refCount_ > 1) {
        --refCount_;
        return 0;
    }
    refCount_ = 0;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    Shutdown();
    delete this;
    return 0;
}

cFC4LotDeveloper* cFC4LotDeveloper::FromPrimary(const void* primary) {
    std::lock_guard<std::mutex> g(g_instancesLock);
    for (cFC4LotDeveloper* t : g_instances) {
        if (static_cast<const void*>(t->Primary()) == primary ||
            static_cast<const void*>(static_cast<cIGZMessageTarget2*>(t)) == primary ||
            static_cast<const void*>(static_cast<cIGZSerializable*>(t)) == primary) {
            return t;
        }
    }
    return nullptr;
}

void cFC4LotDeveloper::SetTestHooks(const fc4_lotdev_host* test) {
    Section sec(this);
    ctx_.test = test;
    fc4_lotdev_destroy(core_);
    CreateCore();
    fc4::Log("lotdev: test hooks %s (core %p)", test ? "installed" : "removed", static_cast<void*>(core_));
}

bool cFC4LotDeveloper::State(fc4_lotdev_state* out) const {
    if (out == nullptr || core_ == nullptr) return false;
    Section sec(this);
    out->struct_size = sizeof(*out);
    return fc4_lotdev_get_state(core_, out);
}

// ---- the lock ---------------------------------------------------------------------------------------------

cFC4LotDeveloper::Section::Section(const cFC4LotDeveloper* s) : s_(const_cast<cFC4LotDeveloper*>(s)) {
    EnterCriticalSection(&s_->lock_);
    ++s_->depth_;
}

cFC4LotDeveloper::Section::~Section() {
    // The outermost call: the references kept for it go.
    if (--s_->depth_ == 0) fc4::LotDevReleaseHeld(&s_->ctx_);
    LeaveCriticalSection(&s_->lock_);
}

fc4_cell_rect cFC4LotDeveloper::Cells(SC4Rect<int32_t> const& r) {
    return fc4_cell_rect{r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY};
}

// ---- status lines -----------------------------------------------------------------------------------------

// Init, Read, Write, Shutdown. Inside a section.
void cFC4LotDeveloper::LogState(const char* what) {
    fc4_lotdev_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_lotdev_get_state(core_, &st)) {
        fc4::Log("lotdev %s: no core state", what);
        return;
    }
    fc4::Log("lotdev %s: init=%u daily=%u visited=%d vertsX=%d extent=%.3f,%.3f cells=%d,%d night=%u/%u mode=%u "
             "lit=%u acc=%u batch=%u forceEmpty=%u noZotUntil=%u commute=%u lots=%u abandoned=%u restored=%u "
             "conditions=%u zots=%u started=%u finished=%u demolitions=%u nightBatches=%u refused=%u queued=%u "
             "agents=%u tick=%u buildings=%u/%u props=%u flora=%u textures=%u/%u removed=%u lights=%u units=%u "
             "posts=%u iterators=%u fallbacks=%u faults=%u listsRefused=%u content: idArrays=%u/%u empty "
             "floraTypes=%u valid/%u not families=%u/%u empty floraAdds=%u/%u null propAdds=%u/%u null "
             "completions=%u cfgObjects props=%u+%u optional flora=%u+%u without IDs nodes=%u/%u failed "
             "autoconnects=%u nodesDestroyed=%u moved=%u wallCalls=%u burnt=%u decals=%u",
             what, st.initialized, st.daily_state, st.visited, st.verts_x, static_cast<double>(st.extent_x),
             static_cast<double>(st.extent_z), st.cell_count_x, st.cell_count_z, st.night_cursor, st.night_len,
             st.night_mode, st.lit_flag, st.night_acc, st.night_batch, st.force_empty, st.no_zot_until_day,
             st.can_check_commute, st.lots_visited, st.abandoned, st.restored, st.condition_changes, st.zot_changes,
             st.constructions_started, st.constructions_finished, st.demolitions, st.night_batches, st.refused_calls,
             st.queued_messages, ctx_.agentLevels, ctx_.tickRegistered ? 1u : 0u, ctx_.buildingsAdded,
             ctx_.buildingsFailed, ctx_.propsAdded, ctx_.floraAdded, ctx_.texturesMade, ctx_.texturesSet,
             ctx_.occupantsRemoved, ctx_.lightsSet, ctx_.unitsMade, ctx_.posts,
             static_cast<unsigned>(ctx_.iterators.size()), ctx_.iteratorFallbacks, ctx_.iteratorFaults,
             ctx_.listsRefused, ctx_.idArrays, ctx_.idArraysEmpty, ctx_.floraValid, ctx_.floraInvalid,
             ctx_.floraFamilies, ctx_.floraFamiliesEmpty, ctx_.floraAdds, ctx_.floraAddsNull, ctx_.propAdds,
             ctx_.propAddsNull, ctx_.completions, ctx_.cfgProps, ctx_.cfgPropsOptional, ctx_.cfgFlora,
             ctx_.cfgFloraNoIds, ctx_.nodesMade, ctx_.nodesFailed, ctx_.autoconnects, ctx_.nodesDestroyed,
             ctx_.objectsMoved, ctx_.wallCalls, ctx_.burntMade, ctx_.decalsAdded);
}

// One line per month: the daily pass's work since the previous line.
void cFC4LotDeveloper::LogMonth(int32_t julian) {
    Section sec(this);
    if (core_ == nullptr) return;
    fc4_lotdev_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_lotdev_get_state(core_, &st)) return;
    fc4_date d{};
    fc4_date_from_julian(julian, &d);
    fc4::Log("lotdev month %04d-%02d-%02d: days=%u redelivered=%u lots=%u abandoned=%u restored=%u conditions=%u "
             "zots=%u started=%u finished=%u demolitions=%u nightBatches=%u night=%u/%u mode=%u buildings=%u props=%u "
             "flora=%u textures=%u removed=%u lights=%u refused=%u queued=%u",
             d.year, d.month, d.day_of_month, days_ - month_.days, dayRedeliveries_ - month_.dayRedeliveries,
             st.lots_visited - month_.visited, st.abandoned - month_.abandoned, st.restored - month_.restored,
             st.condition_changes - month_.conditions, st.zot_changes - month_.zots,
             st.constructions_started - month_.started, st.constructions_finished - month_.finished,
             st.demolitions - month_.demolitions, st.night_batches - month_.nightBatches, st.night_cursor,
             st.night_len, st.night_mode, ctx_.buildingsAdded - month_.buildingsAdded,
             ctx_.propsAdded - month_.propsAdded, ctx_.floraAdded - month_.floraAdded,
             ctx_.texturesSet - month_.texturesSet, ctx_.occupantsRemoved - month_.removed,
             ctx_.lightsSet - month_.lights, st.refused_calls - month_.refused, st.queued_messages - month_.queued);
    month_.days = days_;
    month_.dayRedeliveries = dayRedeliveries_;
    month_.visited = st.lots_visited;
    month_.abandoned = st.abandoned;
    month_.restored = st.restored;
    month_.conditions = st.condition_changes;
    month_.zots = st.zot_changes;
    month_.started = st.constructions_started;
    month_.finished = st.constructions_finished;
    month_.demolitions = st.demolitions;
    month_.nightBatches = st.night_batches;
    month_.refused = st.refused_calls;
    month_.queued = st.queued_messages;
    month_.buildingsAdded = ctx_.buildingsAdded;
    month_.propsAdded = ctx_.propsAdded;
    month_.floraAdded = ctx_.floraAdded;
    month_.texturesSet = ctx_.texturesSet;
    month_.removed = ctx_.occupantsRemoved;
    month_.lights = ctx_.lightsSet;
}

// ---- lifecycle (§1.3, §1.4) -------------------------------------------------------------------------------

bool cFC4LotDeveloper::Init() {
    FC4_TRACE("LotDeveloper::Init");
    Section sec(this);
    const bool ok = fc4_lotdev_init(core_);  // §1.3, all of it (the core drives every host call)
    g_live = this;
    LogState(ok ? "Init" : "Init FAILED");
    return ok;
}

bool cFC4LotDeveloper::Shutdown() {
    FC4_TRACE("LotDeveloper::Shutdown");
    // Keep this object alive while the simulator and the message server drop
    // their references (unless this already is the final Release).
    if (!destroying_) Primary()->AddRef();
    {
        Section sec(this);
        fc4::LotDevFlushLotLog(&ctx_);
        fc4_lotdev_shutdown(core_);  // §1.4: night deque, agent, tick, messages, services, iterator
        LogState("Shutdown");
    }
    cFC4LotDeveloper* self = this;
    g_live.compare_exchange_strong(self, nullptr);
    if (!destroying_) Primary()->Release();
    return true;  // §1.4: always
}

uint32_t cFC4LotDeveloper::GetSimulatorType() { return kFC4_LotDeveloperSimulatorType; }

// ---- slots 6-14 (§6, §7) ------------------------------------------------------------------------------------

bool cFC4LotDeveloper::StartConstruction(cISC4Lot* lot, cISC4LotConfiguration* cfg, uint32_t type, bool animate) {
    Section sec(this);
    const bool ok = fc4_lotdev_start_construction(core_, TokenOf(lot), TokenOf(cfg), type, animate);
    FC4_X87_CHECK("lotdev StartConstruction");
    return ok;
}

bool cFC4LotDeveloper::ConstructDefaultLot(cISC4Lot* lot, cISC4LotConfiguration* cfg, uint32_t type, bool animate) {
    Section sec(this);
    return fc4_lotdev_construct_default_lot(core_, TokenOf(lot), TokenOf(cfg), type, animate);
}

bool cFC4LotDeveloper::AbandonBuilding(cISC4Lot* lot, int32_t reason) {
    Section sec(this);
    return fc4_lotdev_abandon_building(core_, TokenOf(lot), static_cast<uint32_t>(reason));
}

bool cFC4LotDeveloper::ReoccupyBuilding(cISC4Lot* lot, int32_t devType) {
    Section sec(this);
    return fc4_lotdev_reoccupy_building(core_, TokenOf(lot), static_cast<uint32_t>(devType));
}

bool cFC4LotDeveloper::ChangeBuilding(cISC4Lot* lot, uint32_t type) {
    Section sec(this);
    return fc4_lotdev_change_building(core_, TokenOf(lot), type);
}

bool cFC4LotDeveloper::RemoveBuildingZotProp(cISC4Lot* lot) {
    Section sec(this);
    return fc4_lotdev_remove_building_zot_prop(core_, TokenOf(lot));
}

bool cFC4LotDeveloper::StartDemolishLot(cISC4Lot* lot) {
    Section sec(this);
    return fc4_lotdev_start_demolish_lot(core_, TokenOf(lot));
}

// Slot 13: true where the original leaves AL undefined (§11.1).
bool cFC4LotDeveloper::EndDemolishLot(cISC4Lot* lot) {
    Section sec(this);
    return fc4_lotdev_end_demolish_lot(core_, TokenOf(lot));
}

// Slot 14: true where the original leaves AL undefined (§11.1).
bool cFC4LotDeveloper::ExplodeLot(cISC4Lot* lot, bool burnt) {
    Section sec(this);
    return fc4_lotdev_explode_lot(core_, TokenOf(lot), burnt);
}

// ---- slots 15-32: the terrain snapshot (§17.2-§17.4, §11.4) -----------------------------------------------------

float cFC4LotDeveloper::GetTerrainAltitudeByVertex(int32_t x, int32_t z, bool withWater) {
    Section sec(this);
    return fc4_lotdev_terrain_altitude_by_vertex(core_, x, z, withWater);
}

float cFC4LotDeveloper::GetAreaAltitudeF(SC4Rect<float> const& rect, bool lowest, bool withWater) {
    Section sec(this);
    const float r[4] = {rect.topLeftX, rect.topLeftY, rect.bottomRightX, rect.bottomRightY};
    return fc4_lotdev_area_altitude_f(core_, r, lowest, withWater);
}

float cFC4LotDeveloper::GetAreaAltitudeI(SC4Rect<int32_t> const& rect, bool lowest, bool withWater) {
    Section sec(this);
    const fc4_cell_rect r = Cells(rect);
    return fc4_lotdev_area_altitude(core_, &r, lowest, withWater);
}

float cFC4LotDeveloper::GetAverageAltitude(SC4Rect<int32_t> const& rect, bool withWater) {
    Section sec(this);
    const fc4_cell_rect r = Cells(rect);
    return fc4_lotdev_average_area_altitude(core_, &r, withWater);
}

int32_t cFC4LotDeveloper::LevelInternalVertices(SC4Rect<int32_t> const& rect, float target, bool skipWater, bool force,
                                                cISC4Lot* lot) {
    Section sec(this);
    const fc4_cell_rect r = Cells(rect);
    return fc4_lotdev_level_area_internal_vertices(core_, &r, target, skipWater, force, TokenOf(lot));
}

// Slot 20: the third argument is not read (§17.4).
int32_t cFC4LotDeveloper::LevelBorderVertices(SC4Rect<int32_t> const& rect, float h, bool, cISC4Lot* lot) {
    Section sec(this);
    const fc4_cell_rect r = Cells(rect);
    return fc4_lotdev_level_bordering_vertices(core_, &r, h, TokenOf(lot));
}

// Slot 21: the second argument is not read (§17.4).
int32_t cFC4LotDeveloper::AttemptLevelling(SC4Rect<int32_t> const& rect, float, bool withWater, bool perVertex,
                                           float maxAreaSlope, cISC4Lot* lot) {
    Section sec(this);
    const fc4_cell_rect r = Cells(rect);
    return fc4_lotdev_attempt_area_levelling(core_, &r, withWater, perVertex, maxAreaSlope, TokenOf(lot));
}

bool cFC4LotDeveloper::GetOccupantCellCoverage(cISC4Occupant* occupant, SC4Rect<int32_t>& rect) {
    Section sec(this);
    fc4_cell_rect r{rect.topLeftX, rect.topLeftY, rect.bottomRightX, rect.bottomRightY};
    fc4_lotdev_occupant_cell_coverage(core_, TokenOf(occupant), &r);
    rect.topLeftX = r.x1;
    rect.topLeftY = r.z1;
    rect.bottomRightX = r.x2;
    rect.bottomRightY = r.z2;
    // [choice] The original's AL is the x87 status flags PositionToCell
    // leaves (section 17.2, LD18 / LD27); no caller reads it. True.
    return true;
}

// Slots 23 and 24: AL 0 (§11.1: the original's stubs).
bool cFC4LotDeveloper::IsAreaDevelopedStub(SC4Rect<int32_t> const&) { return false; }
bool cFC4LotDeveloper::IsLotDeveloped(cISC4Lot*) { return false; }

bool cFC4LotDeveloper::IsAreaFlat(SC4Rect<int32_t> const& rect, bool withWater) {
    Section sec(this);
    const fc4_cell_rect r = Cells(rect);
    return fc4_lotdev_is_area_flat(core_, &r, withWater);
}

bool cFC4LotDeveloper::IsLotFlat(cISC4Lot* lot, bool withWater) {
    Section sec(this);
    return fc4_lotdev_is_lot_flat(core_, TokenOf(lot), withWater);
}

float cFC4LotDeveloper::GetAreaAltitudeRange(SC4Rect<int32_t> const& rect, bool withWater) {
    Section sec(this);
    const fc4_cell_rect r = Cells(rect);
    return fc4_lotdev_area_altitude_range(core_, &r, withWater);
}

float cFC4LotDeveloper::GetLotAltitudeRange(cISC4Lot* lot, bool withWater) {
    Section sec(this);
    return fc4_lotdev_lot_altitude_range(core_, TokenOf(lot), withWater);
}

float cFC4LotDeveloper::GetAreaSlope(SC4Rect<int32_t> const& rect, bool withWater) {
    Section sec(this);
    const fc4_cell_rect r = Cells(rect);
    return fc4_lotdev_area_slope(core_, &r, withWater);
}

float cFC4LotDeveloper::GetLotSlope(cISC4Lot* lot, bool withWater) {
    Section sec(this);
    return fc4_lotdev_lot_slope(core_, TokenOf(lot), withWater);
}

float cFC4LotDeveloper::GetOccupantSlope(cISC4Occupant* occupant, bool withWater) {
    Section sec(this);
    return fc4_lotdev_occupant_slope(core_, TokenOf(occupant), withWater);
}

int64_t cFC4LotDeveloper::GetLotDevelopmentCost(SC4Rect<int32_t> const& rect) {
    Section sec(this);
    const fc4_cell_rect r = Cells(rect);
    return fc4_lotdev_lot_development_cost(core_, &r);
}

// ---- slots 33-37 (§5.7, §6.4) -------------------------------------------------------------------------------------

bool cFC4LotDeveloper::RemoveLotFromItsBusinessUnit(cISC4Lot* lot) {
    Section sec(this);
    return fc4_lotdev_remove_lot_from_its_business_unit(core_, TokenOf(lot));
}

// Slot 34: the unit in and out; true where the original is undefined.
bool cFC4LotDeveloper::UpdateTravelDesignates(cISC4BusinessUnit*& unit) {
    Section sec(this);
    uint64_t t = TokenOf(unit);
    fc4_lotdev_update_travel_designates(core_, &t);
    unit = reinterpret_cast<cISC4BusinessUnit*>(FromToken(t));
    return true;
}

// Slot 35: true where the original is undefined.
bool cFC4LotDeveloper::UpdateTravelPopulations(cISC4BusinessUnit* unit) {
    Section sec(this);
    fc4_lotdev_update_travel_populations(core_, TokenOf(unit));
    return true;
}

bool cFC4LotDeveloper::NeedsAnchorBuilding(SC4Rect<int32_t> const& rect, int32_t purpose) {
    Section sec(this);
    const fc4_cell_rect r = Cells(rect);
    return fc4_lotdev_needs_anchor_building(core_, &r, purpose);
}

bool cFC4LotDeveloper::UpdateLotElements(cISC4Lot* lot, uint32_t kind) {
    Section sec(this);
    return fc4_lotdev_update_lot_elements(core_, TokenOf(lot), kind);
}

// ---- slots 38-47 (§6.6, §17.6, §17.7) ----------------------------------------------------------------------------

// Slot 38: no reference for the caller.
cISC4BuildingOccupant* cFC4LotDeveloper::GetFirstBuilding(cISC4Lot* lot) {
    Section sec(this);
    return fc4::LotDevFirstBuilding(&ctx_, lot);
}

// Slot 39: EAX = &orient (§11.1, §17.6).
int32_t* cFC4LotDeveloper::GetObjectPosition(cISC4Lot* lot, cISC4LotConfiguration* cfg,
                                              cSC4LotConfigurationObject* obj, float* pos, int32_t& orient) {
    Section sec(this);
    std::vector<uint32_t> ids;
    const fc4_lotdev_cfg_object o = LotDevObjectOf(obj, ids);
    float p[3] = {0.0f, 0.0f, 0.0f};
    if (pos != nullptr) std::memcpy(p, pos, sizeof(p));
    orient = fc4_lotdev_object_position(core_, TokenOf(lot), TokenOf(cfg), obj ? &o : nullptr, p);
    if (pos != nullptr) std::memcpy(pos, p, sizeof(p));
    return &orient;
}

// Slot 40: EAX = &orient, 0 when nothing was written (§17.6).
int32_t* cFC4LotDeveloper::GetBuildingPosition(cISC4Lot* lot, float* pos, int32_t& orient) {
    Section sec(this);
    float p[3] = {0.0f, 0.0f, 0.0f};
    if (pos != nullptr) std::memcpy(p, pos, sizeof(p));
    int32_t o = orient;
    if (!fc4_lotdev_position_for_building(core_, TokenOf(lot), p, &o)) return nullptr;
    if (pos != nullptr) std::memcpy(pos, p, sizeof(p));
    orient = o;
    return &orient;
}

// Slot 41: the out pointer carries one reference for the caller (GZCOM
// out-parameter convention [question LD]).
bool cFC4LotDeveloper::GetLotBaseTextureOccupant(cISC4Lot* lot, cISC4LotBaseTextureOccupant*& occ, bool create) {
    Section sec(this);
    occ = nullptr;
    cISC4Occupant* o = fc4::LotDevBaseTextureOccupant(&ctx_, lot, create);
    if (o == nullptr) return false;
    cISC4LotBaseTextureOccupant* tex = Query<cISC4LotBaseTextureOccupant>(o, kIID_cISC4LotBaseTextureOccupant);
    if (tex == nullptr) return false;
    occ = tex;  // the QueryInterface's reference is the caller's
    return true;
}

// Slot 42: EAX nonzero when a boundary rule was found (§17.7).
intptr_t cFC4LotDeveloper::GetBoundaryTexture(int32_t* sides, uint32_t& texture, uint8_t& rotation, bool& flip) {
    Section sec(this);
    if (sides == nullptr) return 0;
    uint32_t t = texture;
    uint8_t r = rotation;
    bool f = flip;
    if (!fc4_lotdev_boundary_lot_texture(core_, sides, &t, &r, &f)) return 0;
    texture = t;
    rotation = r;
    flip = f;
    return 1;
}

void cFC4LotDeveloper::SetEmptyLotBaseTexture(cISC4Lot* lot) {
    Section sec(this);
    fc4_lotdev_set_empty_lot_base_texture(core_, TokenOf(lot));
}

// Slot 44: EAX = the lot (the original's is undefined, §11.1).
cISC4Lot* cFC4LotDeveloper::SetNormalLotBaseTexture(cISC4Lot* lot, cISC4LotConfiguration* cfg, bool flag) {
    Section sec(this);
    fc4_lotdev_set_normal_lot_base_texture(core_, TokenOf(lot), TokenOf(cfg), flag);
    return lot;
}

bool cFC4LotDeveloper::UpdateLotBaseTextureOccupant(cISC4Lot* lot, bool hadFoundation) {
    Section sec(this);
    return host_.update_lot_base_texture_occupant != nullptr &&
           host_.update_lot_base_texture_occupant(host_.user, TokenOf(lot), hadFoundation);
}

bool cFC4LotDeveloper::GetForceEmptyLotBaseTextures() {
    Section sec(this);
    return fc4_lotdev_get_force_empty(core_);
}

void cFC4LotDeveloper::SetForceEmptyLotBaseTextures(bool force) {
    Section sec(this);
    fc4_lotdev_set_force_empty(core_, force);
}

// ---- slots 48-62: foundations (§17.5) ------------------------------------------------------------------------

cISC4FoundationOccupant* cFC4LotDeveloper::GetFoundationByOccupant(cISC4Occupant* occupant) {
    Section sec(this);
    return reinterpret_cast<cISC4FoundationOccupant*>(
        FromToken(fc4_lotdev_foundation_by_occupant(core_, TokenOf(occupant))));
}

cISC4FoundationOccupant* cFC4LotDeveloper::GetFoundationByCell(int32_t kind, int32_t x, int32_t z) {
    Section sec(this);
    return reinterpret_cast<cISC4FoundationOccupant*>(FromToken(fc4_lotdev_foundation_by_cell(core_, kind, x, z)));
}

cISC4FoundationOccupant* cFC4LotDeveloper::GetFoundationByPosition(int32_t kind, float x, float z) {
    Section sec(this);
    return reinterpret_cast<cISC4FoundationOccupant*>(
        FromToken(fc4_lotdev_foundation_by_position(core_, kind, x, z)));
}

cISC4FoundationOccupant* cFC4LotDeveloper::GetLotFoundation(cISC4Lot* lot) {
    Section sec(this);
    return reinterpret_cast<cISC4FoundationOccupant*>(FromToken(fc4_lotdev_lot_foundation(core_, TokenOf(lot))));
}

bool cFC4LotDeveloper::RemoveFoundation(int32_t kind, float x, float z) {
    Section sec(this);
    return fc4_lotdev_remove_foundation(core_, kind, x, z);
}

// Slot 53: pop 8 (the SDK says 4).
bool cFC4LotDeveloper::RemoveLotFoundationAt(int32_t x, int32_t z) {
    Section sec(this);
    return fc4_lotdev_remove_lot_foundation(core_, x, z);
}

bool cFC4LotDeveloper::AddLotFoundationCfg(cISC4Lot* lot, cISC4LotConfiguration* cfg, bool useLotAltitude, bool level,
                                           bool walls) {
    Section sec(this);
    return fc4_lotdev_add_lot_foundation(core_, TokenOf(lot), TokenOf(cfg), useLotAltitude, level, walls);
}

bool cFC4LotDeveloper::AddLotFoundationType(cISC4Lot* lot, uint32_t type, bool useLotAltitude, bool level,
                                            bool walls) {
    Section sec(this);
    return fc4_lotdev_add_lot_foundation_type(core_, TokenOf(lot), type, useLotAltitude, level, walls);
}

bool cFC4LotDeveloper::IsOccupantFoundationRequired(cISC4Occupant* occupant) {
    Section sec(this);
    return fc4_lotdev_is_occupant_foundation_required(core_, TokenOf(occupant));
}

bool cFC4LotDeveloper::IsBoxFoundationRequired(float* min, float* max) {
    Section sec(this);
    if (min == nullptr || max == nullptr) return false;
    return fc4_lotdev_is_box_foundation_required(core_, min, max);
}

bool cFC4LotDeveloper::AddBuildingFoundationCfg(cISC4BuildingOccupant* building, cISC4LotConfiguration* cfg) {
    Section sec(this);
    return fc4_lotdev_add_building_foundation(core_, TokenOf(building), TokenOf(cfg));
}

bool cFC4LotDeveloper::AddBuildingFoundationType(cISC4BuildingOccupant* building, uint32_t type) {
    Section sec(this);
    return fc4_lotdev_add_building_foundation_type(core_, TokenOf(building), type);
}

bool cFC4LotDeveloper::RemoveBuildingFoundation(cISC4Occupant* occupant) {
    Section sec(this);
    return fc4_lotdev_remove_building_foundation(core_, TokenOf(occupant));
}

bool cFC4LotDeveloper::UpdateLotFoundation(cISC4Lot* lot) {
    Section sec(this);
    return fc4_lotdev_update_lot_foundation(core_, TokenOf(lot));
}

// Slot 62: the retaining-wall helper (not specified: the host's answer).
bool cFC4LotDeveloper::UpdateLotRetainingWalls(cISC4Lot* lot) {
    Section sec(this);
    return host_.update_lot_retaining_walls != nullptr && host_.update_lot_retaining_walls(host_.user, TokenOf(lot));
}

// ---- slots 63 and 64 (§15.5) ----------------------------------------------------------------------------------

// Slot 63 CreateLotProp(lot, objectIndex, propID): the occupant, borrowed
// (the prop manager holds it), or 0. No draw.
cISC4Occupant* cFC4LotDeveloper::CreateLotProp(cISC4Lot* lot, uint32_t objectIndex, uint32_t propID) {
    Section sec(this);
    if (lot == nullptr) return nullptr;
    cISC4LotConfiguration* cfg = fc4::AsGame(lot)->GetLotConfiguration();  // lot slot 9
    cSC4LotConfigurationObject* obj = LotDevObjectAt(cfg, objectIndex);
    if (obj == nullptr) return nullptr;
    std::vector<uint32_t> ids;
    const fc4_lotdev_cfg_object o = LotDevObjectOf(obj, ids);
    float pos[3] = {0.0f, 0.0f, 0.0f};
    const int32_t orient = fc4_lotdev_object_position(core_, TokenOf(lot), TokenOf(cfg), &o, pos);  // slot 39
    if (propID == 0) return nullptr;
    if (!fc4::On(fc4::Sw::LotdevProps)) {
        fc4::NoteSkipped(fc4::Sw::LotdevProps, "lotdev CreateLotProp (none)");
        return nullptr;
    }
    cISC4City* city = ctx_.city;
    cISC4PropManager* pm = ctx_.props ? ctx_.props : (city ? city->GetPropManager() : nullptr);
    if (pm == nullptr) return nullptr;
    const cS3DVector3 v(pos[0], pos[1], pos[2]);
    cISC4Occupant* occ = pm->AddLotProp(propID, v, orient);  // prop manager slot 26, no reference
    if (occ == nullptr) return nullptr;
    occ->SetLotTag(obj->objectID);  // occupant slot 28
    ++ctx_.propsAdded;
    return occ;
}

// Slot 64 GetLotConfigurationObjectOccupant(lot, objectIndex, &out): the
// last occupant in the lot's cells, inside the lot's corners, whose lot tag
// is the object's; one reference for the caller. No draw.
bool cFC4LotDeveloper::GetLotConfigurationObjectOccupant(cISC4Lot* lot, uint32_t objectIndex, cISC4Occupant*& out) {
    Section sec(this);
    out = nullptr;
    if (lot == nullptr) return false;
    cFC4ISC4Lot* l = fc4::AsGame(lot);
    SC4Rect<int32_t> r;
    l->GetBoundingRect(r);  // lot slot 14
    cISC4City* city = ctx_.city;
    float minX = 0.0f, minZ = 0.0f, maxX = 0.0f, maxZ = 0.0f;
    if (city != nullptr) {
        city->CellCornerToPosition(r.topLeftX, r.topLeftY, minX, minZ);                  // slot 96
        city->CellCornerToPosition(r.bottomRightX + 1, r.bottomRightY + 1, maxX, maxZ);  // slot 96
    }
    cISC4LotConfiguration* cfg = l->GetLotConfiguration();  // lot slot 9
    cSC4LotConfigurationObject* obj = LotDevObjectAt(cfg, objectIndex);
    if (obj == nullptr) return false;
    const uint32_t tag = obj->objectID;
    cISC4OccupantManager* om = ctx_.occupants ? ctx_.occupants : (city ? city->GetOccupantManager() : nullptr);
    if (om == nullptr) return false;
    SC4List<cISC4Occupant*> list;
    const SC4Point<int> a(r.topLeftX, r.topLeftY);
    const SC4Point<int> b(r.bottomRightX, r.bottomRightY);
    // Slot 40: the filter overload with no filter.
    if (!om->GetOccupantsByStandardCityCells(list, &a, &b, static_cast<cISC4OccupantFilter*>(nullptr), 0xFFFFFFFFu)) {
        return false;
    }
    for (cISC4Occupant*& o : list) {
        if (o == nullptr) continue;
        FC4_ACQUIRED(o, "GetOccupantsByStandardCityCells list entry");
        cS3DVector3 p;
        o->GetPosition(&p);  // occupant slot 8
        uint32_t t = 0;
        if (minX <= p.fX && p.fX <= maxX && minZ <= p.fZ && p.fZ <= maxZ && o->GetLotTag(t) && t == tag) {
            if (out != nullptr) FC4_RELEASE(out);
            out = o;
            FC4_ADDREF_FOR_CALLER(out);
        }
        FC4_RELEASE(o);
        o = nullptr;  // the list's destructor then only frees its nodes
    }
    return out != nullptr;
}

// ---- messages (§3) ----------------------------------------------------------------------------------------

bool cFC4LotDeveloper::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    const uint32_t type = pMessage->GetType();
    FC4_X87_SCOPE("lotdev DoMessage", type);  // x87 leftover guard (FC4Fpu.h)
    Section sec(this);
    fc4_lotdev_message m{};
    m.msg_type = type;
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    if (s != nullptr) {
        m.data1 = static_cast<uint32_t>(s->GetData1());
        m.data2 = static_cast<uint32_t>(s->GetData2());
        m.data3 = static_cast<uint32_t>(s->GetData3());
        m.data4 = static_cast<uint32_t>(s->GetData4());
        if (type == kMsgOccupantInserted || type == kMsgOccupantRemoved) {
            // §3.2: void1 is the occupant, known by its cISC4Occupant
            // identity. A queued message (another call into the core is
            // running) is handled when that call returns: the reference is
            // kept until then (released with the call's references; the
            // removed occupant's last one may be it).
            cISC4Occupant* occ = Query<cISC4Occupant>(static_cast<cIGZUnknown*>(s->GetVoid1()), kIID_cISC4Occupant);
            if (occ != nullptr) {
                m.occupant = TokenOf(occ);
                fc4::LotDevHold(&ctx_, occ, type == kMsgOccupantRemoved);
            }
        } else if (type == kMsgLotStateChanged || type == kMsgLotConditionChanged) {
            // §3.2: the IGZUnknown is a copy of the lot (cISC4Lot 0xE818FCED)
            // whose other reference the message holds: ours may be the last.
            cISC4Lot* lot = Query<cISC4Lot>(s->GetIGZUnknown(), kIID_cISC4Lot);
            if (lot != nullptr) {
                m.lot = TokenOf(lot);
                fc4::LotDevHold(&ctx_, reinterpret_cast<cIGZUnknown*>(lot), true);
            }
        }
        FC4_RELEASE(s);
    }
    if (type == kMsgSimNewDay) fc4::LotDevFlushLotLog(&ctx_);
    const bool r = fc4_lotdev_do_message(core_, &m);
    if (type == kMsgSimNewDay) {
        fc4::LotDevFlushLotLog(&ctx_);
        ++days_;
        if (!r) ++dayRedeliveries_;
    }
    return r;
}

// ---- persistence (§13) -------------------------------------------------------------------------------------

bool cFC4LotDeveloper::Write(cIGZOStream& s) {
    FC4_TRACE("LotDeveloper::Write");
    Section sec(this);
    fc4::OStreamAdapter a(s);
    const int32_t errorBefore = s.GetError();
    TracedOut t;
    t.inner = a.Get();
    const fc4_ostream traced = MakeTraced(&t);
    const bool written = fc4_lotdev_write(core_, &traced);
    const int32_t errorAfter = s.GetError();
    // §13: Write returns GetError() = 0.
    const bool ok = written || (core_ != nullptr && t.failures != 0 && errorAfter == 0);
    if (written && t.failures == 0) {
        fc4::Log("lotdev Write -> 1 (%u bytes)", t.bytes);
    } else {
        fc4::Log("lotdev Write -> %d (%u bytes; core %d; %u put(s) answered false, the first %s at byte %u; stream "
                 "error %d before, %d after)",
                 ok ? 1 : 0, t.bytes, written ? 1 : 0, t.failures, t.firstFailed ? t.firstFailed : "-", t.failedAt,
                 errorBefore, errorAfter);
    }
    LogState("Write");
    return ok;
}

// Runs before Init (§1.2).
bool cFC4LotDeveloper::Read(cIGZIStream& s) {
    FC4_TRACE("LotDeveloper::Read");
    Section sec(this);
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_lotdev_read(core_, a.Get());
    fc4::Log("lotdev Read -> %d", ok ? 1 : 0);
    LogState("Read");
    return ok;
}

uint32_t cFC4LotDeveloper::GetGZCLSID() { return kFC4_CLSID_cSC4LotDeveloper; }

// ---- test seam op 52 ----------------------------------------------------------------------------------------

intptr_t LotDevTestSeam(const FC4LotDevTestInput* in, void* out) {
    if (in == nullptr) return -1;
    if (in->op == kFC4LotDevTestHooks) {
        fc4::TractNodeAllocator::TestAllocate() = in->allocate;
        fc4::TractNodeAllocator::TestFree() = in->free;
        if (in->lotDeveloper == nullptr) return 0;
        cFC4LotDeveloper* t = cFC4LotDeveloper::FromPrimary(in->lotDeveloper);
        if (t == nullptr) return -1;
        t->SetTestHooks(in->host);
        return 1;
    }
    if (in->op == kFC4LotDevTestShadow) return fc4::LotDevShadowTestRun(in->city, in->julian, in->event, in->host);
    if (in->op == kFC4LotDevTestState) {
        auto* o = static_cast<FC4LotDevTestState*>(out);
        cFC4LotDeveloper* t = in->lotDeveloper ? cFC4LotDeveloper::FromPrimary(in->lotDeveloper) : nullptr;
        if (o == nullptr || t == nullptr) return -1;
        *o = FC4LotDevTestState{};
        o->core.struct_size = sizeof(o->core);
        t->State(&o->core);
        const fc4::LotDevContext& c = t->Context();
        o->agentLevels = c.agentLevels;
        o->tickRegistered = c.tickRegistered ? 1u : 0u;
        o->held = static_cast<uint32_t>(c.held.size());
        o->iterators = static_cast<uint32_t>(c.iterators.size());
        o->buildingsAdded = c.buildingsAdded;
        o->propsAdded = c.propsAdded;
        o->texturesSet = c.texturesSet;
        o->occupantsRemoved = c.occupantsRemoved;
        o->unitsMade = c.unitsMade;
        o->lightsSet = c.lightsSet;
        o->posts = c.posts;
        return 1;
    }
    return -1;
}
