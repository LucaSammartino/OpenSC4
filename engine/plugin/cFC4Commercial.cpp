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

#include "cFC4Commercial.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4Demand.h"
#include "cISC4DemandSimulator.h"
#include "cISC4SimGrid.h"
#include "cISC4Simulator.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstring>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cIGZSerializable = 0xE4FDA3D4;
constexpr uint32_t kIID_cISC4BuildingOccupant = 0x87DDA3A9;

// Messages (§3).
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgSimIdle = 0x6695681A;
constexpr uint32_t kMsgOccupantInserted = 0x99EF1142;
constexpr uint32_t kMsgOccupantRemoved = 0x99EF1143;

// The developer types GetProximityMap answers (§4.6).
constexpr uint32_t kDeveloperCO2 = 0x3320;
constexpr uint32_t kDeveloperCO3 = 0x3330;
// The DoIdle states (§4.3).
constexpr int32_t kStateRebuildCO3 = 2;
constexpr int32_t kStateIdle = 3;

using GridU8 = cISC4SimGrid<uint8_t>;

fc4::CommercialContext* Ctx(void* u) { return static_cast<fc4::CommercialContext*>(u); }

// QueryInterface for `iid`; the result (one reference) or nullptr.
template <class T>
T* Query(cIGZUnknown* u, uint32_t iid) {
    if (u == nullptr) return nullptr;
    void* p = nullptr;
    if (!u->QueryInterface(iid, &p) || p == nullptr) return nullptr;
    T* t = static_cast<T*>(p);
    FC4_ACQUIRED(reinterpret_cast<cIGZUnknown*>(t), "QueryInterface");
    return t;
}

template <class T>
void Drop(T* p) {
    if (p != nullptr) FC4_RELEASE(reinterpret_cast<cIGZUnknown*>(p));
}

// AddRef / Release of a held object: plain on one of ours (no audit), audited
// on a game object.
void HoldObject(cIGZUnknown* o) {
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->AddRef();
    } else {
        FC4_ADDREF(o);
    }
}

void DropObject(cIGZUnknown* o) {
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->Release();
    } else {
        FC4_RELEASE(o);
    }
}

// The app's current city (borrowed).
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

// The building development simulator: the held one (simulator), else the
// city's (shadow), borrowed.
cISC4BuildingDevelopmentSimulator* Bds(void* u) {
    fc4::CommercialContext* c = Ctx(u);
    if (c->bds != nullptr) return c->bds;
    return c->city ? c->city->GetBuildingDevelopmentSimulator() : nullptr;  // city slot 53
}

// The city's simulator, borrowed (agents, §3).
cFC4ISC4Simulator* AgentSimulator(void* u) {
    fc4::CommercialContext* c = Ctx(u);
    cISC4City* city = c->city != nullptr ? c->city : AppCity();
    cISC4Simulator* s = city != nullptr ? city->GetSimulator() : nullptr;
    return s ? fc4::AsGame(s) : nullptr;
}

// ---- host callbacks (fc4_commercial_host) --------------------------------------------------------

// §4.1 step 2: AddAgent(+4, level, name, flags).
void AddAgent(void* u, uint32_t level, const char* name, uint32_t flags) {
    fc4::CommercialContext* c = Ctx(u);
    cFC4ISC4Simulator* s = AgentSimulator(u);
    if (s == nullptr || c->target == nullptr || name == nullptr) {
        fc4::Log("commercial: no simulator; agent at level %u not registered", level);
        return;
    }
    const cRZBaseString n(name);
    s->AddAgent(c->target, level, n, flags);
    c->agentsRegistered = true;
}

// §4.2: RemoveAgent(+4) on the city's simulator (every level).
void RemoveAgent(void* u) {
    fc4::CommercialContext* c = Ctx(u);
    if (!c->agentsRegistered) return;
    cFC4ISC4Simulator* s = AgentSimulator(u);
    if (s != nullptr) s->RemoveAgentAllLevels(c->target);
    c->agentsRegistered = false;
}

// §4.1 step 3.
bool AcquireMessageServer(void* u) {
    fc4::CommercialContext* c = Ctx(u);
    if (c->msgServer == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            c->msgServer = ms;
            FC4_ADDREF(c->msgServer);
        }
    }
    return c->msgServer != nullptr;
}

void Subscribe(void* u, uint32_t type) {
    fc4::CommercialContext* c = Ctx(u);
    if (c->msgServer != nullptr && c->target != nullptr) c->msgServer->AddNotification(c->target, type);
}

void Unsubscribe(void* u, uint32_t type) {
    fc4::CommercialContext* c = Ctx(u);
    if (c->msgServer != nullptr && c->target != nullptr) c->msgServer->RemoveNotification(c->target, type);
}

void ReleaseMessageServer(void* u) {
    fc4::CommercialContext* c = Ctx(u);
    if (c->msgServer != nullptr) FC4_RELEASE(c->msgServer);
    c->msgServer = nullptr;
}

// §4.1 step 4: the city (from the application: the simulator is a city
// component) and its building development simulator (city slot 53).
bool AcquireCity(void* u) {
    fc4::CommercialContext* c = Ctx(u);
    if (c->city == nullptr) {
        cISC4City* city = AppCity();
        if (city != nullptr) {
            c->city = city;
            HoldObject(city);
        }
    }
    return c->city != nullptr;
}

void ReleaseCity(void* u) {
    fc4::CommercialContext* c = Ctx(u);
    DropObject(c->city);
    c->city = nullptr;
}

bool AcquireBds(void* u) {
    fc4::CommercialContext* c = Ctx(u);
    if (c->bds == nullptr && c->city != nullptr) {
        cISC4BuildingDevelopmentSimulator* b = c->city->GetBuildingDevelopmentSimulator();
        if (b != nullptr) {
            c->bds = b;
            HoldObject(b);
        }
    }
    return c->bds != nullptr;
}

void ReleaseBds(void* u) {
    fc4::CommercialContext* c = Ctx(u);
    DropObject(c->bds);
    c->bds = nullptr;
}

// §4.1 step 5: city slot 56.
bool HasDemandSimulator(void* u) {
    fc4::CommercialContext* c = Ctx(u);
    return c->city != nullptr && c->city->GetDemandSimulator() != nullptr;
}

// GetDemand(id, 0) (demand simulator slot 6), held for the core (§2.1).
uint64_t DemandNode(void* u, uint32_t id) {
    fc4::CommercialContext* c = Ctx(u);
    cISC4DemandSimulator* ds = c->city ? c->city->GetDemandSimulator() : nullptr;  // borrowed
    cISC4Demand* node = ds ? fc4::AsGame(ds)->GetDemand(id, 0) : nullptr;             // borrowed
    if (node == nullptr) {
        fc4::Log("commercial: no demand node 0x%04X: Init fails (§4.1 step 5)", id);
        return 0;
    }
    HoldObject(reinterpret_cast<cIGZUnknown*>(node));
    ++c->demandNodes;
    return TokenOf(node);
}

void ReleaseDemandNode(void* u, uint64_t node) {
    if (node == 0) return;
    DropObject(FromToken(node));
    if (Ctx(u)->demandNodes > 0) --Ctx(u)->demandNodes;
}

// §4.4: BDS slot 41 GetBuildingMap, a borrowed cISC4SimGrid<uint8>.
uint64_t BuildingMap(void* u) {
    cISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b ? static_cast<uint64_t>(static_cast<uintptr_t>(b->GetBuildingMap())) : 0;
}

// §4.4: BDS slot 42 GetBuildingMapKey(purpose, wealth); only AL (RG20).
uint8_t BuildingMapKey(void* u, uint32_t purpose, uint32_t wealth) {
    cISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr) return 0;
    const intptr_t k = b->GetBuildingMapKey(static_cast<cISC4BuildingOccupant::PurposeType>(purpose),
                                            static_cast<cISC4BuildingOccupant::WealthType>(wealth));
    return static_cast<uint8_t>(k);
}

// §3: void1.QueryInterface(0x87DDA3A9), the profile's first u32 (slot 11),
// then the building released.
bool BuildingPurpose(void* u, uint64_t occ, uint32_t* purpose) {
    cISC4BuildingOccupant* b = Query<cISC4BuildingOccupant>(FromToken(occ), kIID_cISC4BuildingOccupant);
    if (b == nullptr) return false;
    const cISC4BuildingOccupant::BuildingProfile& p = b->GetBuildingProfile();  // slot 11, read only
    const uint32_t v = static_cast<uint32_t>(p.purpose);
    Drop(b);
    if (purpose != nullptr) *purpose = v;
    fc4::CommercialContext* c = Ctx(u);
    if (!c->readOnly && v == 3) ++c->officeMessages;
    return true;
}

}  // namespace

fc4_commercial_host fc4::MakeCommercialHost(CommercialContext* ctx, const HostGridContext* grids) {
    fc4_commercial_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.grids = MakeGridApi(grids);
    h.building_map = &BuildingMap;
    h.building_map_key = &BuildingMapKey;
    h.building_purpose = &BuildingPurpose;
    if (ctx->readOnly) return h;
    h.add_agent = &AddAgent;
    h.remove_agent = &RemoveAgent;
    h.acquire_message_server = &AcquireMessageServer;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.release_message_server = &ReleaseMessageServer;
    h.acquire_city = &AcquireCity;
    h.release_city = &ReleaseCity;
    h.acquire_bds = &AcquireBds;
    h.release_bds = &ReleaseBds;
    h.has_demand_simulator = &HasDemandSimulator;
    h.demand_node = &DemandNode;
    h.release_demand_node = &ReleaseDemandNode;
    return h;
}

// ---- the commercial simulator --------------------------------------------------------------------

cIGZUnknown* CreateFC4CommercialSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cISC4CommercialSimulator*>(new cFC4CommercialSimulator()));
}

cFC4CommercialSimulator::cFC4CommercialSimulator() : gridCtx_{"commercial", fc4::Sw::CommercialGridWrites} {
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    host_ = fc4::MakeCommercialHost(&ctx_, &gridCtx_);
    core_ = fc4_commercial_create(&host_);
    if (core_ == nullptr) fc4::Log("cFC4CommercialSimulator: core creation failed");
    fc4::Log("cFC4CommercialSimulator(%p) created, core %p", static_cast<void*>(this), static_cast<void*>(core_));
}

cFC4CommercialSimulator::~cFC4CommercialSimulator() {
    co2_.store(nullptr);
    co3_.store(nullptr);
    fc4_commercial_destroy(core_);  // releases the grids a loaded, never-initialised object still holds
    core_ = nullptr;
    // Whatever a never-initialised object still holds.
    ReleaseMessageServer(&ctx_);
    RemoveAgent(&ctx_);
    ReleaseBds(&ctx_);
    ReleaseCity(&ctx_);
    fc4::Log("cFC4CommercialSimulator destroyed (%u rebuilds, %u days, %u idles)", rebuilds_, days_, idles_);
}

// §1: 1 and 0x077ABEB2 give the primary interface, 0x452294AA the message
// target, 0xE4FDA3D4 the serializable; anything else is false, the output
// untouched.
bool cFC4CommercialSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case GZIID_cIGZUnknown:
        case kFC4_IID_cISC4CommercialSimulator:
            *ppvObj = static_cast<cISC4CommercialSimulator*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case kIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        default:
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4CommercialSimulator::AddRef() { return ++refCount_; }

// §2.3: above 1 decrement; at 1 or less Shutdown, destroy, return 0.
uint32_t cFC4CommercialSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

// The two grid pointers for GetProximityMap (any thread). Main thread.
void cFC4CommercialSimulator::Publish() {
    co2_.store(core_ ? reinterpret_cast<GridU8*>(FromToken(fc4_commercial_proximity_map(core_, kDeveloperCO2)))
                     : nullptr);
    co3_.store(core_ ? reinterpret_cast<GridU8*>(FromToken(fc4_commercial_proximity_map(core_, kDeveloperCO3)))
                     : nullptr);
}

void cFC4CommercialSimulator::LogState(const char* what, bool ok) {
    fc4_commercial_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_commercial_get_state(core_, &st)) {
        fc4::Log("commercial %s: ok=%d (no state)", what, ok ? 1 : 0);
        return;
    }
    fc4::Log("commercial %s: ok=%d init=%u dirty=%u state=%d flag=%u grids CO$$=%p CO$$$=%p demand=%llx/%llx city=%u "
             "bds=%u msgServer=%u",
             what, ok ? 1 : 0, st.initialized, st.dirty, st.state, st.flag,
             static_cast<void*>(FromToken(st.co2_grid)), static_cast<void*>(FromToken(st.co3_grid)),
             static_cast<unsigned long long>(st.demand_nodes[0]), static_cast<unsigned long long>(st.demand_nodes[1]),
             st.city_held, st.bds_held, st.message_server_held);
}

// ---- lifecycle (§4.1, §4.2) ----------------------------------------------------------------------

bool cFC4CommercialSimulator::Init() {
    FC4_TRACE("CommercialSimulator::Init");
    const bool ok = core_ != nullptr && fc4_commercial_init(core_);
    Publish();
    LogState(ok ? "Init" : "Init FAILED", ok);
    return ok;
}

bool cFC4CommercialSimulator::Shutdown() {
    FC4_TRACE("CommercialSimulator::Shutdown");
    // Keep this object alive while the simulator and the message server drop
    // their references (unless this already is the final Release).
    if (!destroying_) AddRef();
    co2_.store(nullptr);
    co3_.store(nullptr);
    if (core_ != nullptr) fc4_commercial_shutdown(core_);  // §4.2
    Publish();
    fc4::Log("commercial Shutdown: %u rebuilds, %u days, %u idles, occupant messages %u (offices %u)", rebuilds_,
             days_, idles_, ctx_.occupantMessages, ctx_.officeMessages);
    if (!destroying_) Release();
    return true;  // §4.2: always
}

// Slot 5 (§4.6): 0x3320 -> CO$$, 0x3330 -> CO$$$, else null. No AddRef.
cISC4SimGrid<uint8_t>* cFC4CommercialSimulator::GetProximityMap(
    cISC4BuildingDevelopmentSimulator::DeveloperType type) {
    const uint32_t t = static_cast<uint32_t>(type);
    if (t == kDeveloperCO2) return co2_.load();
    if (t == kDeveloperCO3) return co3_.load();
    return nullptr;
}

// ---- messages (§3) -------------------------------------------------------------------------------

void cFC4CommercialSimulator::NoteStep(uint32_t type, int32_t before) {
    fc4_commercial_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_commercial_get_state(core_, &st)) return;
    if (before == kStateRebuildCO3 && st.state == kStateIdle) {
        ++rebuilds_;
        fc4_date d{};
        if (lastJulian_ > 0) fc4_date_from_julian(lastJulian_, &d);
        // Edge (z = 0) and zero counts of both grids (§4.4: the edge is 1, 0
        // under an office of that wealth; everything else stays 0 on a fresh
        // grid).
        uint32_t edge[2] = {}, zeros[2] = {}, cells[2] = {};
        GridU8* grids[2] = {co2_.load(), co3_.load()};
        for (int i = 0; i < 2; ++i) {
            GridU8* g = grids[i];
            if (g == nullptr) continue;
            const int32_t cx = g->GetTractCountX(), cz = g->GetTractCountZ();
            for (int32_t x = 0; x < cx; ++x) {
                for (int32_t z = 0; z < cz; ++z) {
                    const uint8_t v = g->GetTractValue(x, z);
                    ++cells[i];
                    if (v == 0) ++zeros[i];
                    if (z == 0 && v == 1) ++edge[i];
                }
            }
        }
        fc4::Log("commercial rebuild %u %04d-%02d-%02d (msg 0x%08X): CO$$ cells=%u edge1=%u zeros=%u; CO$$$ cells=%u "
                 "edge1=%u zeros=%u",
                 rebuilds_, d.year, d.month, d.day_of_month, type, cells[0], edge[0], zeros[0], cells[1], edge[1],
                 zeros[1]);
    }
}

bool cFC4CommercialSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("commercial DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    const uint32_t type = pMessage->GetType();
    uint32_t d[3] = {};
    uint64_t void1 = 0;
    if (type == kMsgOccupantInserted || type == kMsgOccupantRemoved || type == kMsgSimNewDay ||
        type == kMsgSimNewMonth) {
        cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
        if (s != nullptr) {
            d[0] = static_cast<uint32_t>(s->GetData1());
            d[1] = static_cast<uint32_t>(s->GetData2());
            d[2] = static_cast<uint32_t>(s->GetData3());
            void1 = TokenOf(s->GetVoid1());  // the occupant of 0x99EF1142 / 0x99EF1143
        }
        Drop(s);
    }
    int32_t before = kStateIdle;
    if (type == kMsgSimIdle || type == kMsgSimNewDay) {
        fc4_commercial_state st{};
        st.struct_size = sizeof(st);
        if (fc4_commercial_get_state(core_, &st)) before = st.state;
    }
    if (type == kMsgOccupantInserted || type == kMsgOccupantRemoved) ++ctx_.occupantMessages;
    const bool r = fc4_commercial_do_message(core_, type, d[0], d[1], d[2], void1);
    if (type == kMsgSimNewDay) {
        lastJulian_ = static_cast<int32_t>(d[0]);
        ++days_;
    } else if (type == kMsgSimIdle) {
        ++idles_;
    }
    if (type == kMsgSimIdle || type == kMsgSimNewDay) NoteStep(type, before);
    if (type == kMsgSimNewMonth) {
        fc4_commercial_state st{};
        st.struct_size = sizeof(st);
        fc4_commercial_get_state(core_, &st);
        fc4_date dt{};
        if (d[0] > 0) fc4_date_from_julian(static_cast<int32_t>(d[0]), &dt);
        fc4::Log("commercial month %04d-%02d-%02d: dirty=%u state=%d rebuilds=%u occupants=%u offices=%u days=%u "
                 "idles=%u",
                 dt.year, dt.month, dt.day_of_month, st.dirty, st.state, rebuilds_, ctx_.occupantMessages,
                 ctx_.officeMessages, days_, idles_);
    }
    return r;  // §3: true
}

// ---- persistence (§8.1) --------------------------------------------------------------------------

bool cFC4CommercialSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("CommercialSimulator::Write");
    fc4::OStreamAdapter a(s);
    const bool ok = core_ != nullptr && fc4_commercial_write(core_, a.Get());
    fc4::Log("commercial Write -> %d", ok ? 1 : 0);
    return ok;
}

bool cFC4CommercialSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("CommercialSimulator::Read");
    fc4::IStreamAdapter a(s);
    const bool ok = core_ != nullptr && fc4_commercial_read(core_, a.Get());
    Publish();
    LogState("Read", ok);
    return ok;
}

uint32_t cFC4CommercialSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4CommercialSimulator; }

// ---- the industrial stub -------------------------------------------------------------------------

cIGZUnknown* CreateFC4IndustrialSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cISC4IndustrialSimulator*>(new cFC4IndustrialSimulator()));
}

cFC4IndustrialSimulator::cFC4IndustrialSimulator() {
    core_ = fc4_industrial_create();
    if (core_ == nullptr) fc4::Log("cFC4IndustrialSimulator: core creation failed");
    fc4::Log("cFC4IndustrialSimulator(%p) created, core %p", static_cast<void*>(this), static_cast<void*>(core_));
}

cFC4IndustrialSimulator::~cFC4IndustrialSimulator() {
    fc4_industrial_destroy(core_);
    core_ = nullptr;
    fc4::Log("cFC4IndustrialSimulator destroyed");
}

bool cFC4IndustrialSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case GZIID_cIGZUnknown:
        case kFC4_IID_cISC4IndustrialSimulator:
            *ppvObj = static_cast<cISC4IndustrialSimulator*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case kIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        default:
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4IndustrialSimulator::AddRef() { return ++refCount_; }

// §2.3 (the same Release as the park manager's).
uint32_t cFC4IndustrialSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

// §2.2: initialized := true (and the write-only static); true.
bool cFC4IndustrialSimulator::Init() {
    FC4_TRACE("IndustrialSimulator::Init");
    if (core_ != nullptr) fc4_industrial_init(core_);
    fc4::Log("industrial Init: initialized=%d", core_ && fc4_industrial_is_initialized(core_) ? 1 : 0);
    return true;
}

bool cFC4IndustrialSimulator::Shutdown() {
    FC4_TRACE("IndustrialSimulator::Shutdown");
    if (core_ != nullptr) fc4_industrial_shutdown(core_);
    fc4::Log("industrial Shutdown");
    return true;
}

bool cFC4IndustrialSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr) return true;
    FC4_X87_SCOPE("industrial DoMessage", pMessage->GetType());
    return core_ == nullptr || fc4_industrial_do_message(core_, pMessage->GetType());
}

bool cFC4IndustrialSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("IndustrialSimulator::Write");
    fc4::OStreamAdapter a(s);
    const bool ok = core_ != nullptr && fc4_industrial_write(core_, a.Get());
    fc4::Log("industrial Write -> %d", ok ? 1 : 0);
    return ok;
}

bool cFC4IndustrialSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("IndustrialSimulator::Read");
    fc4::IStreamAdapter a(s);
    const bool ok = core_ != nullptr && fc4_industrial_read(core_, a.Get());
    fc4::Log("industrial Read -> %d", ok ? 1 : 0);
    return ok;
}

uint32_t cFC4IndustrialSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4IndustrialSimulator; }
