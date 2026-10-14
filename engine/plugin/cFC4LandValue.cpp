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

#include "cFC4LandValue.h"

#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "SC4Rect.h"
#include "cIGZApp.h"
#include "cIGZCOM.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4City.h"
#include "cISC4GrowthDeveloper.h"
#include "cISC4HistoryWarehouse.h"
#include "cISC4LotDeveloper.h"
#include "cISC4SimGrid.h"
#include "cISC4Simulator.h"
#include "cISC4ZoneManager.h"
#include "cISTETerrain.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"

#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;

// Messages (§5).
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgSimIdle = 0x6695681A;
constexpr uint32_t kChangeMessages[] = {0x99EF1142, 0x99EF1143, 0x678E6E3A, 0x278E6003, 0x07699455};

// Agents (§4.1 step 8): one-shot, idle, day, month; flags 2 (established cities).
constexpr uint32_t kAgentLevels[] = {0, 10, 4, 6};
constexpr uint32_t kAgentFlags = 2;
const char kAgentName[] = "cSC4LandValueSimulator";

// Tuning exemplar (§8).
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0xE7E2C8D8;
constexpr uint32_t kPropTractSize = 0x47E2C200;
constexpr uint32_t kPropIntrinsic = 0x47E2C300;
constexpr uint32_t kPropWealth = 0x47E2C301;
constexpr uint32_t kPropWater = 0x47E2C310;
constexpr uint32_t kPropAltitudeCurve = 0x47E2C320;
constexpr uint32_t kPropDeveloperIds = 0x47E2C330;
constexpr uint32_t kPropDeveloperWeights = 0x47E2C331;
constexpr uint32_t kPropDesirabilityCurve = 0x47E2C332;

using GridU8 = cISC4SimGrid<uint8_t>;
using GridI16 = cISC4SimGrid<int16_t>;
using GridF32 = cISC4SimGrid<float>;

cFC4LandValueSimulator* Self(void* u) { return static_cast<cFC4LandValueSimulator*>(u); }
GridU8* U8(uint64_t t) { return reinterpret_cast<GridU8*>(FromToken(t)); }
GridI16* I16(uint64_t t) { return reinterpret_cast<GridI16*>(FromToken(t)); }
GridF32* F32(uint64_t t) { return reinterpret_cast<GridF32*>(FromToken(t)); }

cISC4BuildingDevelopmentSimulator* Bds(void* u) {
    cISC4City* c = Self(u)->City();
    return c ? c->GetBuildingDevelopmentSimulator() : nullptr;
}

cISC4HistoryWarehouse* History(void* u) {
    cISC4City* c = Self(u)->City();
    return c ? c->GetHistoryWarehouse() : nullptr;
}

// ---- host callbacks (fc4_landvalue_host, landvalue.md §12) -------------------------------------

void TerrainCells(void* u, int32_t* x, int32_t* z) {
    cISTETerrain* t = Self(u)->Terrain();
    *x = t ? static_cast<int32_t>(t->CellCountX()) : 0;
    *z = t ? static_cast<int32_t>(t->CellCountZ()) : 0;
}

bool IsLandVertex(void* u, uint32_t x, uint32_t z) {
    cISTETerrain* t = Self(u)->Terrain();
    return t != nullptr && t->IsLandVertex(x, z);
}

float Altitude(void* u, float x, float z) {
    cISTETerrain* t = Self(u)->Terrain();
    return t ? t->GetAltitude(x, z) : 0.0f;  // compiled slot 13 (foundations §16)
}

float MinAllowedHeight(void* u) {
    cISTETerrain* t = Self(u)->Terrain();
    return t ? t->MinAllowedHeight() : 0.0f;
}

float MaxAllowedHeight(void* u) {
    cISTETerrain* t = Self(u)->Terrain();
    return t ? t->MaxAllowedHeight() : 0.0f;
}

uint32_t ZoneType(void* u, int32_t x, int32_t z) {
    cISC4ZoneManager* zm = Self(u)->ZoneManager();
    return zm ? static_cast<uint32_t>(zm->GetZoneType(x, z)) : 0;
}

float AreaSlope(void* u, int32_t x1, int32_t z1, int32_t x2, int32_t z2) {
    cISC4BuildingDevelopmentSimulator* bds = Bds(u);
    cISC4LotDeveloper* ld = bds ? bds->GetLotDeveloper() : nullptr;  // borrowed
    if (ld == nullptr) return 0.0f;
    const SC4Rect<int32_t> r(x1, z1, x2, z2);
    return ld->GetAreaSlope(r, true);
}

uint64_t Developer(void* u, uint32_t type) {
    cISC4BuildingDevelopmentSimulator* bds = Bds(u);
    if (bds == nullptr) return 0;
    return TokenOf(bds->GetDeveloper(static_cast<cISC4BuildingDevelopmentSimulator::DeveloperType>(type)));
}

uint64_t DesirabilityMap(void*, uint64_t dev) {
    if (dev == 0) return 0;
    return TokenOf(reinterpret_cast<cISC4GrowthDeveloper*>(FromToken(dev))->GetDesirabilityMap());
}

// §4.1 step 3 through GZCOM (simgrid.md §6.3): the game's own grid class. The
// class factory's object comes with the one reference the core then owns.
uint64_t CreateGrid(void*, uint32_t kind, int32_t tractSize, uint32_t instanceId) {
    const uint32_t clsid = kind == FC4_GRID_U8 ? kFC4_CLSID_SimGridUint8 : kFC4_CLSID_SimGridFloat;
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (com == nullptr || !com->GetClassObject(clsid, kFC4_IID_cISC4SimGrid, &p) || p == nullptr) {
        fc4::Log("landvalue: WARNING could not create grid 0x%08X (class 0x%08X) through GZCOM", instanceId, clsid);
        return 0;
    }
    if (kind == FC4_GRID_U8) {
        GridU8* g = static_cast<GridU8*>(p);
        FC4_ACQUIRED(g, "GetClassObject");
        g->Init();
        g->SetTractSize(tractSize);
        g->SetInstanceID(instanceId);
    } else {
        GridF32* g = static_cast<GridF32*>(p);
        FC4_ACQUIRED(g, "GetClassObject");
        g->Init();
        g->SetTractSize(tractSize);
        g->SetInstanceID(instanceId);
    }
    FC4_LOGV("landvalue: grid 0x%08X created (class 0x%08X, tract size %d) at %p", instanceId, clsid, tractSize, p);
    return TokenOf(p);
}

// Slots 10-17 do not depend on the element type.
void GridGeometry(void*, uint64_t t, fc4_grid_geometry* out) {
    GridU8* g = U8(t);
    if (g == nullptr || out == nullptr) return;
    out->tract_size = g->GetTractSize();
    out->shift = g->GetTractShift();
    out->count_x = g->GetTractCountX();
    out->count_z = g->GetTractCountZ();
    out->width_x = g->GetTractWidthX();
    out->width_z = g->GetTractWidthZ();
    out->inv_width_x = g->GetOneOverTractWidthX();
    out->inv_width_z = g->GetOneOverTractWidthZ();
}

void GridSetTractSize(void*, uint64_t t, int32_t s) {
    if (GridU8* g = U8(t)) {
        fc4::Log("landvalue: restored grid %p has tract size %d; resized to %d [choice]", static_cast<void*>(g),
                 g->GetTractSize(), s);
        g->SetTractSize(s);
    }
}

// Values cross the ABI x outer, z inner; read and written through the
// interface (GetTractValue / SetTractValue), not the grid's memory.
template <class G, class T>
void ReadGrid(G* g, T* out, size_t n) {
    if (g == nullptr || out == nullptr) return;
    const int32_t cx = g->GetTractCountX(), cz = g->GetTractCountZ();
    size_t i = 0;
    for (int32_t x = 0; x < cx; ++x) {
        for (int32_t z = 0; z < cz && i < n; ++z) out[i++] = g->GetTractValue(x, z);
    }
}

template <class G, class T>
void WriteGrid(G* g, const T* values, size_t n) {
    if (g == nullptr || values == nullptr) return;
    const int32_t cx = g->GetTractCountX(), cz = g->GetTractCountZ();
    size_t i = 0;
    for (int32_t x = 0; x < cx; ++x) {
        for (int32_t z = 0; z < cz && i < n; ++z) g->SetTractValue(x, z, values[i++]);
    }
}

void GridReadU8(void*, uint64_t t, uint8_t* out, size_t n) { ReadGrid(U8(t), out, n); }
void GridReadI16(void*, uint64_t t, int16_t* out, size_t n) { ReadGrid(I16(t), out, n); }
void GridWriteU8(void*, uint64_t t, const uint8_t* v, size_t n) { WriteGrid(U8(t), v, n); }
void GridWriteF32(void*, uint64_t t, const float* v, size_t n) { WriteGrid(F32(t), v, n); }

void GridShutdown(void*, uint64_t t) {
    if (GridU8* g = U8(t)) g->Shutdown();
}

void ReleaseGrid(void*, uint64_t t) {
    if (GridU8* g = U8(t)) FC4_RELEASE(g);
}

void SetHistoryQueueLength(void* u, uint32_t series, uint32_t length) {
    if (cISC4HistoryWarehouse* h = History(u)) h->SetHistoryDataQueueLength(series, length);
}

void RecordHistory(void* u, uint32_t series, int32_t value) {
    if (cISC4HistoryWarehouse* h = History(u)) h->RecordHistoryData(series, value);
}

// §9: type only, data fields 0, MessagePost(msg, false).
void Post(void*, uint32_t type) {
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* msg = FC4MakeMessage(type, 0, 0, 0);
    ms->MessagePost(msg, false);
    msg->Release();
}

fc4_landvalue_host MakeHost(cFC4LandValueSimulator* self) {
    fc4_landvalue_host h{};
    h.struct_size = sizeof(h);
    h.user = self;
    h.terrain_cells = &TerrainCells;
    h.is_land_vertex = &IsLandVertex;
    h.altitude = &Altitude;
    h.min_allowed_height = &MinAllowedHeight;
    h.max_allowed_height = &MaxAllowedHeight;
    h.zone_type = &ZoneType;
    h.area_slope = &AreaSlope;
    h.developer = &Developer;
    h.desirability_map = &DesirabilityMap;
    h.create_grid = &CreateGrid;
    h.grid_geometry = &GridGeometry;
    h.grid_set_tract_size = &GridSetTractSize;
    h.grid_read_u8 = &GridReadU8;
    h.grid_read_i16 = &GridReadI16;
    h.grid_write_u8 = &GridWriteU8;
    h.grid_write_f32 = &GridWriteF32;
    h.grid_shutdown = &GridShutdown;
    h.release = &ReleaseGrid;
    h.set_history_queue_length = &SetHistoryQueueLength;
    h.record_history = &RecordHistory;
    h.post = &Post;
    return h;
}

// ---- tuning (§8) ----------------------------------------------------------------------------------

struct Tuning {
    std::vector<int64_t> tractSize, intrinsic, wealth, water;
    std::vector<uint32_t> ids;
    std::vector<float> altitude, weights, desirability;
    bool hasIntrinsic = false, hasWealth = false, hasWater = false, hasAltitude = false, hasIds = false,
         hasWeights = false, hasDesirability = false;

    fc4_landvalue_tuning Get() const {
        // A present empty array still needs a non-NULL pointer.
        static const int64_t kNoInts[1] = {};
        static const uint32_t kNoIds[1] = {};
        static const float kNoFloats[1] = {};
        auto ints = [&](bool has, const std::vector<int64_t>& v) {
            return has ? (v.empty() ? kNoInts : v.data()) : nullptr;
        };
        auto floats = [&](bool has, const std::vector<float>& v) {
            return has ? (v.empty() ? kNoFloats : v.data()) : nullptr;
        };
        fc4_landvalue_tuning t{};
        t.struct_size = sizeof(t);
        t.has_tract_size = tractSize.empty() ? 0u : 1u;
        t.tract_size = tractSize.empty() ? 0 : tractSize[0];
        t.intrinsic = ints(hasIntrinsic, intrinsic);
        t.intrinsic_count = intrinsic.size();
        t.wealth = ints(hasWealth, wealth);
        t.wealth_count = wealth.size();
        t.water = ints(hasWater, water);
        t.water_count = water.size();
        t.altitude_curve = floats(hasAltitude, altitude);
        t.altitude_curve_count = altitude.size();
        t.developer_ids = hasIds ? (ids.empty() ? kNoIds : ids.data()) : nullptr;
        t.developer_id_count = ids.size();
        t.developer_weights = floats(hasWeights, weights);
        t.developer_weight_count = weights.size();
        t.desirability_curve = floats(hasDesirability, desirability);
        t.desirability_curve_count = desirability.size();
        return t;
    }
};

bool ReadTuning(Tuning& t) {
    fc4::props::Exemplar ex;
    if (!ex.Load(kTuningType, kTuningGroup, kTuningInstance)) return false;
    const cISCPropertyHolder* h = ex.Holder();
    namespace p = fc4::props;
    p::GetI64s(h, kPropTractSize, t.tractSize);
    t.hasIntrinsic = p::GetI64s(h, kPropIntrinsic, t.intrinsic);
    t.hasWealth = p::GetI64s(h, kPropWealth, t.wealth);
    t.hasWater = p::GetI64s(h, kPropWater, t.water);
    t.hasAltitude = p::GetF32s(h, kPropAltitudeCurve, t.altitude);
    t.hasIds = p::GetU32s(h, kPropDeveloperIds, t.ids);
    t.hasWeights = p::GetF32s(h, kPropDeveloperWeights, t.weights);
    t.hasDesirability = p::GetF32s(h, kPropDesirabilityCurve, t.desirability);
    return true;
}

}  // namespace

void FC4LandValueReadTuning(void (*f)(void* ctx, const fc4_landvalue_tuning* tuning), void* ctx) {
    if (f == nullptr) return;
    Tuning tuning;
    if (!ReadTuning(tuning)) {
        f(ctx, nullptr);
        return;
    }
    const fc4_landvalue_tuning t = tuning.Get();
    f(ctx, &t);
}

cIGZUnknown* CreateFC4LandValueSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cISC4LandValueSimulator*>(new cFC4LandValueSimulator()));
}

cFC4LandValueSimulator::cFC4LandValueSimulator() {
    host_ = MakeHost(this);
    core_ = fc4_landvalue_create(&host_);
    if (core_ == nullptr) fc4::Log("cFC4LandValueSimulator: core creation failed");
    fc4::Log("cFC4LandValueSimulator(%p) created, core %p", static_cast<void*>(this), static_cast<void*>(core_));
}

cFC4LandValueSimulator::~cFC4LandValueSimulator() {
    fc4_landvalue_destroy(core_);  // releases grids a never-initialised (loaded) object still holds
    core_ = nullptr;
    fc4::Log("cFC4LandValueSimulator destroyed");
}

bool cFC4LandValueSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_IID_cISC4LandValueSimulator:
            *ppvObj = static_cast<cISC4LandValueSimulator*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        case GZIID_cIGZUnknown:
            *ppvObj = static_cast<cIGZUnknown*>(static_cast<cISC4LandValueSimulator*>(this));
            break;
        default:
            *ppvObj = nullptr;
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4LandValueSimulator::AddRef() { return ++refCount_; }

// §4.3: at a count of 1 (or below) Shutdown, destroy, return 0.
uint32_t cFC4LandValueSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

// ---- lifecycle (§4) -------------------------------------------------------------------------------

bool cFC4LandValueSimulator::Init() {
    FC4_TRACE("LandValueSimulator::Init");
    if (initialized_) return true;
    // Step 1: city, zone manager, terrain (each only if not already held).
    if (city_ == nullptr) {
        cIGZFrameWork* fw = RZGetFrameWork();
        cIGZApp* app = fw ? fw->Application() : nullptr;
        cISC4App* sc4 = nullptr;
        if (app != nullptr && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) && sc4 != nullptr) {
            FC4_ACQUIRED(sc4, "QueryInterface");
            city_ = sc4->GetCity();
            FC4_ADDREF(city_);
            FC4_RELEASE(sc4);
        }
    }
    if (city_ != nullptr && zoneManager_ == nullptr) {
        zoneManager_ = city_->GetZoneManager();
        FC4_ADDREF(zoneManager_);
    }
    if (city_ != nullptr && terrain_ == nullptr) {
        terrain_ = city_->GetTerrain();
        FC4_ADDREF(terrain_);
    }

    // Steps 2-7 (core): tuning, grids, pipeline state 0, iterators, histogram, history.
    Tuning tuning;
    const bool haveTuning = ReadTuning(tuning);
    if (!haveTuning) fc4::Log("landvalue: tuning exemplar unavailable; the current values stay");
    const fc4_landvalue_tuning t = tuning.Get();
    const bool ok = fc4_landvalue_init(core_, haveTuning ? &t : nullptr);

    // Step 8: agents; step 9: notifications.
    RegisterAgents(true);
    if (msgServer_ == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            msgServer_ = ms;
            FC4_ADDREF(msgServer_);
            for (uint32_t id : kChangeMessages) {
                msgServer_->AddNotification(static_cast<cIGZMessageTarget2*>(this), id);
            }
        }
    }
    initialized_ = true;

    fc4_landvalue_state st{};
    st.struct_size = sizeof(st);
    fc4_landvalue_get_state(core_, &st);
    lastState_ = st.pipeline_state;
    fc4::Log("landvalue Init: ok=%d city=%p terrain=%p grids intrinsic=%p total=%p slope=%p developers=%u "
             "dirty=%d average=%u",
             ok ? 1 : 0, static_cast<void*>(city_), static_cast<void*>(terrain_),
             static_cast<void*>(FromToken(fc4_landvalue_grid(core_, 0))),
             static_cast<void*>(FromToken(fc4_landvalue_grid(core_, 1))),
             static_cast<void*>(FromToken(fc4_landvalue_grid(core_, 2))), st.developer_count, st.dirty,
             st.average_land_value);
    return true;
}

bool cFC4LandValueSimulator::Shutdown() {
    FC4_TRACE("LandValueSimulator::Shutdown");
    if (!initialized_) return true;
    initialized_ = false;
    // Keep this object alive while the simulator and the message server drop
    // their references (unless this already is the final Release).
    if (!destroying_) AddRef();
    RegisterAgents(false);
    if (msgServer_ != nullptr) {
        for (uint32_t id : kChangeMessages) {
            msgServer_->RemoveNotification(static_cast<cIGZMessageTarget2*>(this), id);
        }
        FC4_RELEASE(msgServer_);
        msgServer_ = nullptr;
    }
    fc4_landvalue_shutdown(core_);  // grids: Shutdown, Release
    FC4_RELEASE(terrain_);
    terrain_ = nullptr;
    FC4_RELEASE(zoneManager_);
    zoneManager_ = nullptr;
    FC4_RELEASE(city_);
    city_ = nullptr;
    fc4::Log("landvalue Shutdown (%u passes)", passes_);
    if (!destroying_) Release();
    return true;
}

void cFC4LandValueSimulator::RegisterAgents(bool add) {
    cFC4ISC4Simulator* sim = city_ ? fc4::AsGame(city_->GetSimulator()) : nullptr;
    if (sim == nullptr) {
        if (add) fc4::Log("landvalue: no simulator; agents not registered");
        return;
    }
    cIGZMessageTarget2* self = static_cast<cIGZMessageTarget2*>(this);
    if (add) {
        const cRZBaseString name(kAgentName);
        for (uint32_t level : kAgentLevels) sim->AddAgent(self, level, name, kAgentFlags);
        agentsRegistered_ = true;
    } else if (agentsRegistered_) {
        sim->RemoveAgentAllLevels(self);
        agentsRegistered_ = false;
    }
}

// ---- queries (§7.6) -------------------------------------------------------------------------------

uint8_t cFC4LandValueSimulator::GetLandValue(int32_t x, int32_t z) {
    GridU8* g = GetTotalLandValueMap();
    return g ? g->GetCellValue(x, z) : 0;  // cell coordinates, no bounds check (as the game)
}

cISC4LandValueSimulator::LandValueType cFC4LandValueSimulator::GetLandValueType(int32_t x, int32_t z) {
    return static_cast<LandValueType>(fc4_landvalue_type_of(core_, GetLandValue(x, z)));
}

cISC4SimGrid<uint8_t>* cFC4LandValueSimulator::GetIntrinsicLandValueMap() {
    return U8(fc4_landvalue_grid(core_, 0));
}

cISC4SimGrid<uint8_t>* cFC4LandValueSimulator::GetTotalLandValueMap() { return U8(fc4_landvalue_grid(core_, 1)); }

cISC4SimGrid<float>* cFC4LandValueSimulator::GetSlopeMap() { return F32(fc4_landvalue_grid(core_, 2)); }

uint8_t cFC4LandValueSimulator::GetAverageLandValue() { return fc4_landvalue_average(core_); }

float cFC4LandValueSimulator::GetModeAltitude() { return fc4_landvalue_mode_altitude(core_); }

// ---- messages (§5) --------------------------------------------------------------------------------

bool cFC4LandValueSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("land value DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    const uint32_t type = pMessage->GetType();
    int64_t data3 = 0;
    if (type == kMsgSimIdle) {
        cIGZMessage2Standard* m = nullptr;
        if (pMessage->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) && m != nullptr) {
            FC4_ACQUIRED(m, "QueryInterface");
            data3 = static_cast<int64_t>(m->GetData3());
            FC4_RELEASE(m);
        }
    }
    fc4_landvalue_handle_message(core_, type, data3);
    fc4_landvalue_state st{};
    st.struct_size = sizeof(st);
    if (fc4_landvalue_get_state(core_, &st)) {
        if (st.pipeline_state == 0 && lastState_ == 9 && type == kMsgSimNewMonth) {
            FC4_LOGV("landvalue: pass restarted (something changed)");
        }
        if (st.pipeline_state == 9 && lastState_ != 9) {
            ++passes_;
            fc4::Log("landvalue pass %u done: average %u, mode altitude %.1f, mean %.1f, counted tracts %d%s", passes_,
                     st.average_land_value, st.mode_alt, st.mean_alt, st.count,
                     st.ignore_zoning ? " (no zoned land: all land counted)" : "");
        }
        lastState_ = st.pipeline_state;
    }
    return true;  // §5: always
}

// ---- persistence (§10) ----------------------------------------------------------------------------

bool cFC4LandValueSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("LandValueSimulator::Write");
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_landvalue_write(core_, a.Get());
    fc4::Log("landvalue Write -> %d", ok ? 1 : 0);
    return ok;
}

bool cFC4LandValueSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("LandValueSimulator::Read");
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_landvalue_read(core_, a.Get());
    fc4_landvalue_state st{};
    st.struct_size = sizeof(st);
    fc4_landvalue_get_state(core_, &st);
    fc4::Log("landvalue Read -> %d (dirty %d, average %u, mean altitude %.1f; grids %p %p %p)", ok ? 1 : 0, st.dirty,
             st.average_land_value, st.mean_alt, static_cast<void*>(FromToken(fc4_landvalue_grid(core_, 0))),
             static_cast<void*>(FromToken(fc4_landvalue_grid(core_, 1))),
             static_cast<void*>(FromToken(fc4_landvalue_grid(core_, 2))));
    return ok;
}

uint32_t cFC4LandValueSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4LandValueSimulator; }
