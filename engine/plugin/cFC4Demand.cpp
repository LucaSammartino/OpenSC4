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

#include "cFC4Demand.h"

#include "FC4Checked.h"
#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "cFC4Date.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "cGZPersistResourceKey.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZPersistResourceKeyFilter.h"
#include "cIGZPersistResourceKeyList.h"
#include "cIGZPersistResourceManager.h"
#include "cISC4AirportDeveloper.h"
#include "cISC4App.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4Demand.h"
#include "cISC4GrowthDeveloper.h"
#include "cISC4HistoryWarehouse.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4Nation.h"
#include "cISC4NeighborConnection.h"
#include "cISC4NeighborsSimulator.h"
#include "cISC4OrdinanceSimulator.h"
#include "cISC4RegionalCity.h"
#include "cISC4ResidentialSimulator.h"
#include "cISC4SeaportDeveloper.h"
#include "cISC4SimGrid.h"
#include "cISC4Simulator.h"
#include "cISC4TrafficSimulator.h"
#include "cISCPropertyHolder.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cmath>
#include <cstring>
#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kSimulatorType = 0x279681D7;  // §5 GetSimulatorType [compat]

// Messages (§10.2, §3.4).
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgStateChanged = 0xE8BDA2E7;
constexpr uint32_t kMsgActiveDemandChanged = 0x426840A0;
// §13 step 8: the notifications (the four agent levels bring the rest).
constexpr uint32_t kSubscriptions[] = {0xE8BDA2E7, 0x8A0D15DA, 0xEA0D163E, 0x26C63343, 0xCA1B151C};

// §10.1: SimBegin, Day, Month, Idle; flags 2 (established cities only).
constexpr uint32_t kAgentLevels[] = {0, 4, 6, 10};
constexpr uint32_t kAgentFlags = 2;
const char kAgentName[] = "cSC4DemandSimulator";

// Resources (§4.1, §7.1).
constexpr uint32_t kExemplarType = 0x6534284A;
constexpr uint32_t kPropExemplarType = 0x00000010;
constexpr uint32_t kDemandExemplarType = 3;
constexpr uint32_t kPropNoDemandEffects = 0x27812844;
constexpr uint32_t kBuildingDemandProps[] = {0x27812840, 0x27812841, 0x27812842, 0x27812843, 0x27812844};

// §8.3: GetTrafficArrived(holder, 3) [inferred, §15 Q6].
constexpr uint32_t kFreightTripClass = 3;

// §9.2: the growth developers, in developer-type (map) order. [choice] Read
// through GetDeveloper(type): the SDK's GetDevelopers() returns the BDS's map
// as an opaque pointer whose layout is not specified.
constexpr uint32_t kGrowthDeveloperTypes[] = {0x1010, 0x1020, 0x1030, 0x3110, 0x3120, 0x3130,
                                              0x3320, 0x3330, 0x4100, 0x4200, 0x4300, 0x4400};

using DeveloperType = cISC4BuildingDevelopmentSimulator::DeveloperType;

// The message server the nodes post through (set while a simulator is
// initialized; nodes fall back to the service pointer otherwise).
cIGZMessageServer2* g_msgServer = nullptr;

cFC4DemandSimulator* Self(void* u) { return static_cast<cFC4DemandSimulator*>(u); }
cISC4City* CityOf(void* u) { return u ? Self(u)->City() : nullptr; }

cISC4BuildingDevelopmentSimulator* Bds(void* u) {
    cISC4City* c = CityOf(u);
    return c ? c->GetBuildingDevelopmentSimulator() : nullptr;
}

cISC4GrowthDeveloper* Developer(void* u, uint32_t devType) {
    cISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b ? b->GetDeveloper(static_cast<DeveloperType>(devType)) : nullptr;  // borrowed
}

cFC4ISC4ResidentialSimulator* Residential(void* u) {
    cISC4City* c = CityOf(u);
    cISC4ResidentialSimulator* r = c ? c->GetResidentialSimulator() : nullptr;
    return r ? fc4::AsGame(r) : nullptr;
}

cFC4ISC4NeighborsSimulator* Neighbors(void* u) {
    cISC4City* c = CityOf(u);
    cISC4NeighborsSimulator* n = c ? c->GetNeighborsSimulator() : nullptr;
    return n ? fc4::AsGame(n) : nullptr;
}

cFC4ISC4Lot* LotAt(void* u, int32_t x, int32_t z) {
    cISC4City* c = CityOf(u);
    cISC4LotManager* lm = c ? c->GetLotManager() : nullptr;
    if (lm == nullptr) return nullptr;
    cISC4Lot* lot = fc4::AsGame(lm)->GetLot(x, z, true);  // slot 6, borrowed
    return lot ? fc4::AsGame(lot) : nullptr;
}

cFC4ISC4Lot* LotOf(uint64_t token) { return token ? fc4::AsGame(reinterpret_cast<cISC4Lot*>(FromToken(token))) : nullptr; }

cFC4ISC4RegionalCity* RegionalCityOf(uint64_t token) {
    return token ? fc4::AsGame(reinterpret_cast<cISC4RegionalCity*>(FromToken(token))) : nullptr;
}

// The application's cISC4App (one reference, released by the caller's scope).
template <class F>
auto WithApp(F f) -> decltype(f(static_cast<cISC4App*>(nullptr))) {
    using R = decltype(f(static_cast<cISC4App*>(nullptr)));
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    FC4_OUT(cISC4App*, sc4, "cISC4App* (QueryInterface out pointer)");
    const bool ok = app != nullptr && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(sc4.ptr()));
    sc4.Check();
    if (!ok || sc4.get() == nullptr) return R{};
    FC4_ACQUIRED(sc4.get(), "QueryInterface");
    R r = f(sc4.get());
    FC4_RELEASE(sc4.get());
    return r;
}

void NoteOnce(bool& done, const char* text) {
    if (done) return;
    done = true;
    fc4::Log("%s", text);
}

// ---- host callbacks (fc4_demand_host, demand.md §14) ------------------------------------------------

// A key filter for the resource manager: exemplars only. Lives on the stack
// of the enumeration; references are counted but never delete it.
class ExemplarKeyFilter final : public cIGZPersistResourceKeyFilter {
public:
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid == GZIID_cIGZPersistResourceKeyFilter || riid == GZIID_cIGZUnknown) {
            *ppvObj = this;
            AddRef();
            return true;
        }
        *ppvObj = nullptr;
        return false;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { return refs_ > 0 ? --refs_ : 0; }
    bool IsKeyIncluded(cGZPersistResourceKey const& key) override { return key.type == kExemplarType; }

private:
    uint32_t refs_ = 1;
};

// §4.1: every exemplar resource whose ExemplarType is 3, in the resource
// manager's order. [choice] The key list comes from
// GetAvailableResourceList(&list, filter) with a null list pointer on entry
// (the manager returns a new list with one reference).
void ForEachExemplarKey(void*, fc4_key_visitor visit, void* ctx) {
    if (visit == nullptr) return;
    if (!fc4::On(fc4::Sw::ExemplarLoads)) {
        fc4::NoteSkipped(fc4::Sw::ExemplarLoads, "demand exemplar enumeration (no demand nodes)");
        return;
    }
    cIGZPersistResourceManagerPtr rm;
    if (!rm) {
        fc4::Log("demand: no resource manager; no demand exemplars");
        return;
    }
    ExemplarKeyFilter filter;
    FC4_OUT(cIGZPersistResourceKeyList*, list, "cIGZPersistResourceKeyList* (GetAvailableResourceList out pointer)");
    rm->GetAvailableResourceList(list.ptr(), &filter);
    list.Check();
    cIGZPersistResourceKeyList* l = list.get();
    if (l == nullptr) {
        fc4::Log("demand: WARNING the resource manager returned no key list; no demand exemplars");
        return;
    }
    FC4_ACQUIRED(l, "GetAvailableResourceList");
    std::vector<cGZPersistResourceKey> keys;
    const uint32_t n = l->Size();
    keys.reserve(n);
    for (uint32_t i = 0; i < n; ++i) keys.push_back(l->GetKey(i));
    FC4_RELEASE(l);
    uint32_t found = 0;
    for (const cGZPersistResourceKey& k : keys) {
        if (k.type != kExemplarType) continue;
        fc4::props::Exemplar ex;
        if (!ex.Load(k.type, k.group, k.instance)) continue;
        std::vector<uint32_t> t;
        if (!fc4::props::GetU32s(ex.Holder(), kPropExemplarType, t) || t.empty() || t[0] != kDemandExemplarType) continue;
        visit(ctx, k.type, k.group, k.instance);
        ++found;
    }
    fc4::Log("demand: %u exemplar keys enumerated, %u demand exemplars", n, found);
}

fc4_props* ExemplarProps(void*, uint32_t type, uint32_t group, uint32_t instance) {
    fc4::props::Exemplar ex;
    if (!ex.Load(type, group, instance)) return nullptr;
    return fc4::props::ToCoreAll(ex.Holder());
}

void Warn(void*, const uint8_t* text, size_t len) {
    fc4::Log("demand: %.*s", static_cast<int>(len), reinterpret_cast<const char*>(text));
}

// [choice] Our nodes are created directly (not through the GZCOM class
// factory, §1): the core only keeps nodes of its own class.
fc4_demand_node* CreateNode(void*, uint64_t* token) {
    if (token == nullptr) return nullptr;
    cFC4Demand* n = new cFC4Demand();
    if (n->Core() == nullptr) {
        delete n;
        return nullptr;
    }
    n->AddRef();  // the core's reference
    *token = n->Token();
    return n->Core();
}

void AddRefToken(void*, uint64_t t) {
    cIGZUnknown* u = FromToken(t);
    if (u == nullptr) return;
    if (fc4::IsOurObject(u)) {
        u->AddRef();
    } else {
        FC4_ADDREF(u);
    }
}

void ReleaseToken(void*, uint64_t t) {
    cIGZUnknown* u = FromToken(t);
    if (u == nullptr) return;
    if (fc4::IsOurObject(u)) {
        u->Release();
    } else {
        FC4_RELEASE(u);
    }
}

// §3.4: a standard message with data1 = key and data2 = N, posted (queued).
// [choice] No IGZUnknown is attached (the spec names only data1 / data2).
void PostActiveDemand(void*, uint64_t, uint32_t key, int32_t data2) {
    if (!fc4::On(fc4::Sw::MessagePosting)) {
        fc4::NoteSkipped(fc4::Sw::MessagePosting, "demand ActiveDemandChanged post");
        return;
    }
    auto post = [&](cIGZMessageServer2* ms) {
        cFC4Message2Standard* msg = cFC4Message2Standard::New();
        msg->AddRef();
        msg->SetType(kMsgActiveDemandChanged);
        msg->SetData1(static_cast<intptr_t>(key));
        msg->SetData2(static_cast<intptr_t>(data2));
        ms->MessagePost(msg, false);
        msg->Release();
    };
    if (g_msgServer != nullptr) {
        post(g_msgServer);
        return;
    }
    cIGZMessageServer2Ptr ms;
    if (ms) post(ms);
}

void CellCounts(void* u, int32_t* x, int32_t* z) {
    cISC4City* c = CityOf(u);
    if (x != nullptr) *x = c ? static_cast<int32_t>(c->CellCountX()) : 0;
    if (z != nullptr) *z = c ? static_cast<int32_t>(c->CellCountZ()) : 0;
}

// The simulation date's day number (compiled slot 8, a Julian day).
uint32_t Today(void* u) {
    cISC4City* c = CityOf(u);
    cISC4Simulator* s = c ? c->GetSimulator() : nullptr;
    return s ? static_cast<uint32_t>(fc4::AsGame(s)->GetSimDateNumber()) : 0;
}

uint32_t NowMs(void*) { return GetTickCount(); }

// §6.3: GetLot(x, z, true), HasCapacity, GetBuildingType(true), GetOccupantWealth.
bool CensusLot(void* u, int32_t x, int32_t z, uint32_t* buildingType, uint32_t* wealth) {
    if (!fc4::On(fc4::Sw::LotScan)) {
        fc4::NoteSkipped(fc4::Sw::LotScan, "demand census lot (no lot)");
        return false;
    }
    cFC4ISC4Lot* lot = LotAt(u, x, z);
    if (lot == nullptr || !lot->HasCapacity()) return false;
    const uint32_t bt = lot->GetBuildingType(true);
    if (bt == 0) return false;
    if (buildingType != nullptr) *buildingType = bt;
    if (wealth != nullptr) *wealth = lot->GetOccupantWealth();
    return true;
}

bool g_capacityNoted = false;

// §6.3 / §7.2: the building profile's purpose. [choice] Its capacity list
// (a vector of maps in the game's STL) is not reported: its memory layout is
// not specified, so UpdateCapacity adds nothing (spec question).
bool BuildingProfile(void* u, uint32_t buildingType, uint32_t* purpose, fc4_capacity_visitor, void*) {
    cISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr || purpose == nullptr) return false;
    const cISC4BuildingOccupant::BuildingProfile& p = b->GetBuildingProfile(buildingType);
    *purpose = static_cast<uint32_t>(p.purpose);
    NoteOnce(g_capacityNoted,
             "demand: building profile capacity lists are not read (layout not specified); occupancy changes "
             "move no capacity (§7.2)");
    return true;
}

uint32_t DeveloperTypeOf(void* u, uint32_t purpose, uint32_t wealth) {
    cISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr) return 0;
    return static_cast<uint32_t>(b->GetDeveloperType(static_cast<cISC4BuildingOccupant::PurposeType>(purpose),
                                                     static_cast<cISC4BuildingOccupant::WealthType>(wealth)));
}

int16_t Desirability(void* u, uint32_t devType, int32_t x, int32_t z) {
    cISC4GrowthDeveloper* g = Developer(u, devType);
    cISC4SimGrid<int16_t>* m = g ? g->GetDesirabilityMap() : nullptr;
    return m ? m->GetCellValue(x, z) : 0;
}

uint16_t LotCapacity(void* u, int32_t x, int32_t z, uint32_t devType) {
    cFC4ISC4Lot* lot = LotAt(u, x, z);
    return lot ? lot->GetCapacityOfDeveloper(static_cast<DeveloperType>(devType), false) : 0;  // slot 50
}

float CellEq(void* u, int32_t x, int32_t z) {
    cFC4ISC4ResidentialSimulator* r = Residential(u);
    return r ? r->GetCellEQ(static_cast<uint32_t>(x), static_cast<uint32_t>(z)) : 0.0f;  // slot 35
}

float CellWorkforcePercent(void* u, int32_t x, int32_t z) {
    cFC4ISC4ResidentialSimulator* r = Residential(u);
    return r ? r->GetCellWorkforcePercent(static_cast<uint32_t>(x), static_cast<uint32_t>(z)).percentage : 0.0f;
}

uint32_t LotBuildingType(void*, uint64_t lot) {
    cFC4ISC4Lot* l = LotOf(lot);
    return l ? l->GetBuildingType(true) : 0;
}

// §7: the lot's building has bool 0x27812844 = true.
bool LotExcluded(void*, uint64_t lot) {
    cFC4ISC4Lot* l = LotOf(lot);
    cISC4BuildingOccupant* b = l ? l->GetBuilding() : nullptr;  // borrowed
    if (b == nullptr) return false;
    FC4_OUT(cISCPropertyHolder*, h, "cISCPropertyHolder* (QueryInterface out pointer)");
    const bool has = b->QueryInterface(GZIID_cISCPropertyHolder, reinterpret_cast<void**>(h.ptr()));
    h.Check();
    if (!has || h.get() == nullptr) return false;
    FC4_ACQUIRED(h.get(), "QueryInterface");
    std::vector<uint32_t> v;
    const bool excluded = fc4::props::GetU32s(h.get(), kPropNoDemandEffects, v) && !v.empty() && v[0] != 0;
    FC4_RELEASE(h.get());
    return excluded;
}

// §7.1: GetAllBuildingTypes(array, count). [choice] The array overload with
// the GZ array convention (fill up to *count, set *count to the number
// written, return the total): one large buffer, retried if too small.
void AllBuildingTypes(void* u, fc4_u32_callback cb, void* ctx) {
    cISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr || cb == nullptr) return;
    std::vector<uint32_t> types(65536);
    for (int attempt = 0; attempt < 2; ++attempt) {
        uint32_t count = static_cast<uint32_t>(types.size());
        const size_t total = b->GetAllBuildingTypes(types.data(), count);
        if (total > types.size() && attempt == 0) {
            types.assign(total, 0);
            continue;
        }
        if (count > types.size()) count = static_cast<uint32_t>(types.size());
        for (uint32_t i = 0; i < count; ++i) cb(ctx, types[i]);
        fc4::Log("demand: %u building types", count);
        return;
    }
}

// §7.1: the building exemplar's demand properties only (exact types); NULL
// when it has neither effect list (nothing to read).
fc4_props* BuildingExemplar(void* u, uint32_t buildingType) {
    cISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr) return nullptr;
    cGZPersistResourceKey key;
    if (!b->GetBuildingKeyFromType(buildingType, key)) return nullptr;
    fc4::props::Exemplar ex;
    if (!ex.Load(key.type, key.group, key.instance)) return nullptr;
    const cISCPropertyHolder* h = ex.Holder();
    if (!fc4::props::Has(h, kBuildingDemandProps[0]) && !fc4::props::Has(h, kBuildingDemandProps[1])) return nullptr;
    fc4_props* p = fc4_props_create();
    if (p == nullptr) return nullptr;
    for (uint32_t id : kBuildingDemandProps) fc4::props::CopyToCore(h, id, p);
    return p;
}

// §8.1: the highest value of a series between two Julian days.
bool HistoryMax(void* u, uint32_t series, uint32_t fromDay, uint32_t toDay, float* max) {
    if (!fc4::On(fc4::Sw::History)) {
        fc4::NoteSkipped(fc4::Sw::History, "GetMinMaxAverage (no data)");
        return false;
    }
    cISC4City* c = CityOf(u);
    cISC4HistoryWarehouse* h = c ? c->GetHistoryWarehouse() : nullptr;
    if (h == nullptr || max == nullptr) return false;
    cFC4Date from(static_cast<int32_t>(fromDay), false);
    cFC4Date to(static_cast<int32_t>(toDay), false);
    FC4_OUT(float, mn, "float min (GetMinMaxAverage)");
    FC4_OUT(float, mx, "float max (GetMinMaxAverage)");
    FC4_OUT(float, avg, "float average (GetMinMaxAverage)");
    const bool ok = h->GetMinMaxAverage(series, *fc4::AsSdk(&from), *fc4::AsSdk(&to), mn.get(), mx.get(), avg.get());
    mn.Check();
    mx.Check();
    avg.Check();
    if (!ok) return false;
    *max = mx.get();
    return true;
}

float OrdinanceFactor(void* u, uint32_t prop, uint32_t index) {
    cISC4City* c = CityOf(u);
    cISC4OrdinanceSimulator* o = c ? c->GetOrdinanceSimulator() : nullptr;
    return o ? o->GetCumulativeFactor(prop, index) : 1.0f;
}

float NationEconomyModifier(void*, uint32_t demandId) {
    return WithApp([&](cISC4App* app) {
        cISC4Nation* n = app->GetNation();  // borrowed
        return n ? n->GetEconomyModifier(demandId) : 0.0f;
    });
}

// §8.3. [choice] The list entries are borrowed (no reference is released).
void Connections(void* u, fc4_connection_callback cb, void* ctx) {
    cFC4ISC4NeighborsSimulator* n = Neighbors(u);
    if (n == nullptr || cb == nullptr) return;
    FC4_OUT(SC4List<cISC4NeighborConnection*>, list, "SC4List<cISC4NeighborConnection*> (GetAllConnections)");
    n->GetAllConnections(list.get());  // slot 16
    list.Check();
    for (cISC4NeighborConnection* conn : list.get()) {
        if (conn == nullptr) continue;
        fc4_demand_connection dc{};
        dc.token = TokenOf(conn);
        dc.city = TokenOf(conn->GetCityOnOtherSideOfConnection());
        dc.conn_type = static_cast<uint32_t>(conn->GetType());
        cb(ctx, &dc);
    }
}

float TrafficArrived(void* u, uint64_t connection) {
    cISC4City* c = CityOf(u);
    cISC4TrafficSimulator* t = c ? c->GetTrafficSimulator() : nullptr;
    auto* conn = reinterpret_cast<cISC4NeighborConnection*>(FromToken(connection));
    if (t == nullptr || conn == nullptr) return 0.0f;
    return static_cast<float>(t->GetTrafficArrived(conn->AsISCPropertyHolder(), kFreightTripClass));  // slot 21
}

// §8.3 [inferred]: slot 13 of the port developers (no docs/abi row).
float AirportTrips(void* u) {
    cISC4City* c = CityOf(u);
    cISC4AirportDeveloper* a = c ? c->GetAirportDeveloper() : nullptr;
    return a ? static_cast<float>(a->GetNumAirportTrips()) : 0.0f;
}

float SeaportTrips(void* u) {
    cISC4City* c = CityOf(u);
    cISC4SeaportDeveloper* s = c ? c->GetSeaportDeveloper() : nullptr;
    return s ? static_cast<float>(s->GetNumSeaportTrips()) : 0.0f;
}

// §9.1. [choice] The cities are borrowed (no reference is released).
void Cities(void* u, bool all, fc4_token_callback cb, void* ctx) {
    cFC4ISC4NeighborsSimulator* n = Neighbors(u);
    if (n == nullptr || cb == nullptr) return;
    FC4_OUT(SC4List<cISC4RegionalCity*>, list, "SC4List<cISC4RegionalCity*> (GetConnectedCities)");
    if (all) {
        n->GetAllConnectedCities(list.get(), false);  // slot 22
    } else {
        n->GetConnectedCities(list.get(), false);  // slot 21
    }
    list.Check();
    for (cISC4RegionalCity* city : list.get()) {
        if (city != nullptr) cb(ctx, TokenOf(city));
    }
}

void AllConnectedCities(void* u, fc4_token_callback cb, void* ctx) { Cities(u, true, cb, ctx); }
void ConnectedCities(void* u, fc4_token_callback cb, void* ctx) { Cities(u, false, cb, ctx); }

uint64_t OwnRegionalCity(void*) {
    return WithApp([](cISC4App* app) { return TokenOf(app->GetRegionalCity()); });
}

int32_t CityPopulation(void*, uint64_t city, uint32_t type) {
    cFC4ISC4RegionalCity* c = RegionalCityOf(city);
    return c ? c->GetPopulationOfType(type) : 0;  // slot 9
}

int32_t CityExtrapolatedPopulation(void*, uint64_t city, uint32_t type) {
    cFC4ISC4RegionalCity* c = RegionalCityOf(city);
    return c ? c->GetExtrapolatedPopulation(type) : 0;  // slot 17
}

int32_t CityAllowableExtrapolation(void*, uint64_t city, uint32_t type) {
    cFC4ISC4RegionalCity* c = RegionalCityOf(city);
    return c ? c->GetAllowableExtrapolation(type) : 0;  // slot 18
}

float CityTaxRate(void*, uint64_t city, uint32_t type) {
    cFC4ISC4RegionalCity* c = RegionalCityOf(city);
    return c ? c->GetTaxRate(type) : 0.0f;  // slot 16
}

// Slot 13 returns 32 bits in EAX (docs/abi). [choice] They are taken as the
// f32 bits of the fraction, clamped to [0, 1] (NaN -> 0): spec question.
float CityWorkforcePercentage(void*, uint64_t city) {
    cFC4ISC4RegionalCity* c = RegionalCityOf(city);
    if (c == nullptr) return 0.0f;
    const uint32_t bits = c->GetWorkforcePercentage();
    float f = 0.0f;
    std::memcpy(&f, &bits, sizeof(f));
    if (!(f >= 0.0f)) return 0.0f;
    return f > 1.0f ? 1.0f : f;
}

void CityExtrapolateGrowth(void*, uint64_t city, uint32_t type, float amount) {
    if (cFC4ISC4RegionalCity* c = RegionalCityOf(city)) c->ExtrapolateGrowth(type, amount);  // slot 19
}

void GrowthDevelopers(void* u, fc4_growth_developer_callback cb, void* ctx) {
    if (cb == nullptr) return;
    for (uint32_t type : kGrowthDeveloperTypes) {
        cISC4GrowthDeveloper* g = Developer(u, type);
        if (g == nullptr) continue;
        fc4_demand_growth_developer d{};
        d.sub_type = g->GetSubType();
        d.kind = static_cast<uint32_t>(g->GetType());
        cISC4Demand* src = g->GetSourceDemand();  // borrowed; any node class (§1)
        if (src != nullptr) {
            d.has_source = 1;
            d.supply = src->QuerySupplyValue();
            d.demand = src->QueryDemandValue();
        }
        cb(ctx, &d);
    }
}

uint32_t DayOfLastSuccessfulBuild(void* u, uint32_t devType) {
    cISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b ? static_cast<uint32_t>(b->GetDayOfLastSuccessfulBuild(static_cast<DeveloperType>(devType))) : 0;
}

void RecordHistory(void* u, uint32_t series, int32_t value) {
    if (!fc4::On(fc4::Sw::History)) {
        fc4::NoteSkipped(fc4::Sw::History, "RecordHistoryData");
        return;
    }
    cISC4City* c = CityOf(u);
    if (cISC4HistoryWarehouse* h = c ? c->GetHistoryWarehouse() : nullptr) h->RecordHistoryData(series, value);
}

void SetHistoryQueueLength(void* u, uint32_t series, uint32_t length) {
    if (!fc4::On(fc4::Sw::History)) {
        fc4::NoteSkipped(fc4::Sw::History, "SetHistoryDataQueueLength");
        return;
    }
    cISC4City* c = CityOf(u);
    if (cISC4HistoryWarehouse* h = c ? c->GetHistoryWarehouse() : nullptr) h->SetHistoryDataQueueLength(series, length);
}

fc4_demand_host MakeHost(cFC4DemandSimulator* sim) {
    fc4_demand_host h{};
    h.struct_size = sizeof(h);
    h.user = sim;
    h.for_each_exemplar_key = &ForEachExemplarKey;
    h.exemplar = &ExemplarProps;
    h.warn = &Warn;
    h.create_node = &CreateNode;
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.post_active_demand = &PostActiveDemand;
    h.cell_counts = &CellCounts;
    h.today = &Today;
    h.now_ms = &NowMs;
    h.census_lot = &CensusLot;
    h.building_profile = &BuildingProfile;
    h.developer_type = &DeveloperTypeOf;
    h.desirability = &Desirability;
    h.lot_capacity = &LotCapacity;
    h.cell_eq = &CellEq;
    h.cell_workforce_percent = &CellWorkforcePercent;
    h.lot_building_type = &LotBuildingType;
    h.lot_excluded = &LotExcluded;
    h.all_building_types = &AllBuildingTypes;
    h.building_exemplar = &BuildingExemplar;
    h.history_max = &HistoryMax;
    h.ordinance_factor = &OrdinanceFactor;
    h.nation_economy_modifier = &NationEconomyModifier;
    h.connections = &Connections;
    h.traffic_arrived = &TrafficArrived;
    h.airport_trips = &AirportTrips;
    h.seaport_trips = &SeaportTrips;
    h.all_connected_cities = &AllConnectedCities;
    h.connected_cities = &ConnectedCities;
    h.own_regional_city = &OwnRegionalCity;
    h.city_population = &CityPopulation;
    h.city_extrapolated_population = &CityExtrapolatedPopulation;
    h.city_allowable_extrapolation = &CityAllowableExtrapolation;
    h.city_tax_rate = &CityTaxRate;
    h.city_workforce_percentage = &CityWorkforcePercentage;
    h.city_extrapolate_growth = &CityExtrapolateGrowth;
    h.growth_developers = &GrowthDevelopers;
    h.day_of_last_successful_build = &DayOfLastSuccessfulBuild;
    h.record_history = &RecordHistory;
    h.set_history_queue_length = &SetHistoryQueueLength;
    return h;
}

// The nodes' table: user = nullptr; the core of a node uses post_active_demand only.
const fc4_demand_host* NodeHost() {
    static const fc4_demand_host h = MakeHost(nullptr);
    return &h;
}

}  // namespace

// =====================================================================================================
// The demand simulator
// =====================================================================================================

cIGZUnknown* CreateFC4DemandSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4DemandSimulator*>(new cFC4DemandSimulator()));
}

cFC4DemandSimulator::cFC4DemandSimulator() {
    host_ = MakeHost(this);
    core_ = fc4_demand_sim_create(&host_, TokenOf(AsUnknown()));
    if (core_ == nullptr) fc4::Log("cFC4DemandSimulator: core creation failed");
    fc4::Log("cFC4DemandSimulator(%p) created, core %p", static_cast<void*>(this), static_cast<void*>(core_));
}

cFC4DemandSimulator::~cFC4DemandSimulator() {
    fc4_demand_sim_destroy(core_);  // releases every node still in the map
    core_ = nullptr;
    fc4::Log("cFC4DemandSimulator destroyed");
}

// §1: cISC4DemandSimulator, cIGZSerializable, cIGZMessageTarget2, cIGZUnknown.
// [choice] Not the fourth IID 0x452294AB (§15 Q8: an older message-target
// interface whose methods are not specified).
bool cFC4DemandSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_IID_cISC4DemandSimulator:
            *ppvObj = static_cast<cFC4ISC4DemandSimulator*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        case GZIID_cIGZUnknown:
            *ppvObj = AsUnknown();
            break;
        default:
            *ppvObj = nullptr;
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4DemandSimulator::AddRef() { return ++refCount_; }

// [choice] As the other simulators: at a count of 1 (or below) Shutdown,
// destroy, return 0 (the demand spec does not describe Release).
uint32_t cFC4DemandSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

bool cFC4DemandSimulator::Initialized() const {
    fc4_demand_sim_state st{};
    st.struct_size = sizeof(st);
    return fc4_demand_sim_get_state(core_, &st) && st.initialized != 0;
}

void cFC4DemandSimulator::LogState(const char* what) {
    fc4_demand_sim_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_demand_sim_get_state(core_, &st)) return;
    fc4::Log("demand %s: %u nodes, %u links, census state %u, loaded=%u, relief C %.1f I %.1f, %u export cities", what,
             st.node_count, st.link_count, st.census_state, st.loaded_from_save, st.commercial_relief,
             st.industrial_relief, st.export_city_count);
}

// ---- lifecycle (§13) ----------------------------------------------------------------------------------

bool cFC4DemandSimulator::Init() {
    FC4_TRACE("DemandSimulator::Init");
    if (Initialized()) return true;
    // Step 2 (and 4: the services are read from the city when needed).
    if (city_ == nullptr) {
        WithApp([&](cISC4App* app) {
            city_ = app->GetCity();
            FC4_ADDREF(city_);
            return 0;
        });
    }
    if (msgServer_ == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            msgServer_ = ms;
            FC4_ADDREF(msgServer_);
        }
    }
    g_msgServer = msgServer_;
    // Steps 1, 3, 5-7 and 10 (the core).
    const bool ok = fc4_demand_sim_init(core_);
    Subscribe(true);       // step 8
    RegisterAgents(true);  // step 9
    fc4::Log("demand Init: ok=%d city=%p agents=%d", ok ? 1 : 0, static_cast<void*>(city_), agentsRegistered_ ? 1 : 0);
    LogState("after Init");
    return true;
}

bool cFC4DemandSimulator::Shutdown() {
    FC4_TRACE("DemandSimulator::Shutdown");
    if (!Initialized()) return true;
    // Keep this object alive while the simulator and the message server let
    // go (unless this already is the final Release).
    if (!destroying_) AddRef();
    RegisterAgents(false);
    Subscribe(false);
    fc4_demand_sim_shutdown(core_);  // nodes: Shutdown, Release
    if (g_msgServer == msgServer_) g_msgServer = nullptr;
    FC4_RELEASE(msgServer_);
    msgServer_ = nullptr;
    FC4_RELEASE(city_);
    city_ = nullptr;
    fc4::Log("demand Shutdown");
    if (!destroying_) Release();
    return true;
}

void cFC4DemandSimulator::Subscribe(bool add) {
    if (msgServer_ == nullptr) {
        if (add) fc4::Log("demand: no message server; notifications not subscribed");
        return;
    }
    cIGZMessageTarget2* self = static_cast<cIGZMessageTarget2*>(this);
    for (uint32_t id : kSubscriptions) {
        if (add) {
            msgServer_->AddNotification(self, id);
        } else {
            msgServer_->RemoveNotification(self, id);
        }
    }
}

void cFC4DemandSimulator::RegisterAgents(bool add) {
    cFC4ISC4Simulator* sim = city_ ? fc4::AsGame(city_->GetSimulator()) : nullptr;
    if (sim == nullptr) {
        if (add) fc4::Log("demand: no simulator; agents not registered");
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

// ---- cISC4DemandSimulator (§5) --------------------------------------------------------------------------

uint32_t cFC4DemandSimulator::GetSimulatorType() { return kSimulatorType; }

// §5: initializes first if needed; no reference is added.
cISC4Demand* cFC4DemandSimulator::GetDemand(uint32_t demandID, uint32_t demandIndex) {
    if (!Initialized()) Init();
    const uint64_t t = fc4_demand_sim_get_demand(core_, demandID, demandIndex);
    return reinterpret_cast<cISC4Demand*>(static_cast<uintptr_t>(t));
}

void cFC4DemandSimulator::UpdateOccupantEffects(uint32_t buildingType, SC4Percentage const& oldPct,
                                                SC4Percentage const& newPct) {
    fc4_demand_sim_update_occupant_effects(core_, buildingType, oldPct.percentage, newPct.percentage);
}

// docs/abi: the game returns in ST0. [choice] The value is the sum of the
// three outputs (what it is in the original is not specified; spec question).
float cFC4DemandSimulator::CalculateJobsPerUnitOfDemand(float* jobs, uint32_t demandID, uint32_t demandIndex) {
    float j[3] = {0.0f, 0.0f, 0.0f};
    fc4_demand_sim_jobs_per_unit(core_, demandID, demandIndex, j);
    if (jobs != nullptr) std::memcpy(jobs, j, sizeof(j));
    return j[0] + j[1] + j[2];
}

uint32_t cFC4DemandSimulator::GetJobsBySensus(uint32_t type) { return fc4_demand_sim_jobs_by_census(core_, type); }

float cFC4DemandSimulator::GetNeutralTaxRate() { return fc4_demand_sim_neutral_tax_rate(core_); }

// [choice] The caller's map is the game's STL map, whose memory layout is not
// specified: it is left untouched (spec question). The summary is logged once.
void cFC4DemandSimulator::GetLocalPopulationSummary(std::map<uint32_t, int32_t>&) const {
    const size_t n = fc4_demand_sim_local_population_summary(core_, nullptr, nullptr);
    if (!summaryNoted_) {
        summaryNoted_ = true;
        fc4::Log("demand: GetLocalPopulationSummary: %u entries not copied (the game's map layout is not specified)",
                 static_cast<unsigned>(n));
    }
}

// ---- messages (§10.2) ----------------------------------------------------------------------------------------

bool cFC4DemandSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    const uint32_t type = pMessage->GetType();
    cIGZMessage2Standard* m = nullptr;
    if (pMessage->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) && m != nullptr) {
        FC4_ACQUIRED(m, "QueryInterface");
    } else {
        m = nullptr;
    }
    if (type == kMsgStateChanged) {
        // §7: the lot is the message's IGZUnknown, seen as its cISC4Lot.
        uint64_t lot = 0;
        cISC4Lot* l = nullptr;
        cIGZUnknown* unk = m ? m->GetIGZUnknown() : nullptr;
        if (unk != nullptr && unk->QueryInterface(GZIID_cISC4Lot, reinterpret_cast<void**>(&l)) && l != nullptr) {
            FC4_ACQUIRED(l, "QueryInterface");
            lot = TokenOf(l);
        }
        if (m != nullptr) {
            fc4_demand_sim_state_changed(core_, lot, static_cast<uint32_t>(m->GetData1()),
                                         static_cast<uint32_t>(m->GetData2()), static_cast<uint32_t>(m->GetData3()),
                                         static_cast<uint32_t>(m->GetData4()));
        }
        if (l != nullptr) FC4_RELEASE(l);
    } else {
        const uint32_t d1 = m ? static_cast<uint32_t>(m->GetData1()) : 0;
        const uint32_t d2 = m ? static_cast<uint32_t>(m->GetData2()) : 0;
        const uint32_t d3 = m ? static_cast<uint32_t>(m->GetData3()) : 0;
        fc4_demand_sim_handle_message(core_, type, d1, d2, d3);
        if (type == kMsgSimNewMonth) LogState("month");
    }
    if (m != nullptr) FC4_RELEASE(m);
    return true;  // §10.2: always
}

// ---- persistence (§12.1) -------------------------------------------------------------------------------------

bool cFC4DemandSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("DemandSimulator::Write");
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_demand_sim_write(core_, a.Get());
    fc4::Log("demand Write -> %d", ok ? 1 : 0);
    return ok;
}

bool cFC4DemandSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("DemandSimulator::Read");
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_demand_sim_read(core_, a.Get());
    fc4::Log("demand Read -> %d", ok ? 1 : 0);
    LogState("after Read");
    return ok;
}

uint32_t cFC4DemandSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4DemandSimulator; }

// =====================================================================================================
// Demand nodes
// =====================================================================================================

cIGZUnknown* CreateFC4Demand() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4Demand*>(new cFC4Demand()));
}

cFC4Demand::cFC4Demand() {
    core_ = fc4_demand_node_create(NodeHost(), Token());
    if (core_ == nullptr) fc4::Log("cFC4Demand: core creation failed");
}

cFC4Demand::~cFC4Demand() {
    fc4_demand_node_destroy(core_);
    core_ = nullptr;
}

// §1: cISC4Demand, cIGZSerializable, cIGZUnknown (and ours).
bool cFC4Demand::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_IID_cISC4Demand:
            *ppvObj = static_cast<cFC4ISC4Demand*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        case kFC4_IID_CoreObject:
            *ppvObj = static_cast<cIFC4CoreObject*>(this);
            break;
        case GZIID_cIGZUnknown:
            *ppvObj = static_cast<cIGZUnknown*>(static_cast<cFC4ISC4Demand*>(this));
            break;
        default:
            *ppvObj = nullptr;
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4Demand::AddRef() { return ++refCount_; }

uint32_t cFC4Demand::Release() {
    if (refCount_ > 0) --refCount_;
    if (refCount_ == 0) {
        delete this;
        return 0;
    }
    return refCount_;
}

bool cFC4Demand::Init() { return fc4_demand_node_init(core_); }
bool cFC4Demand::Shutdown() { return fc4_demand_node_shutdown(core_); }
bool cFC4Demand::SimulationBegin() { return fc4_demand_node_simulation_begin(core_); }
uint32_t cFC4Demand::GetId() const { return fc4_demand_node_id(core_); }
bool cFC4Demand::SetId(uint32_t id) { return fc4_demand_node_set_id(core_, id); }
float cFC4Demand::QuerySupplyValue() const { return Get(FC4_DEMAND_SUPPLY); }
float cFC4Demand::QueryDemandValue() const { return Get(FC4_DEMAND_DEMAND); }
float cFC4Demand::QueryNewSupply() const { return Get(FC4_DEMAND_NEW_SUPPLY); }
float cFC4Demand::QueryNewDemand() const { return Get(FC4_DEMAND_NEW_DEMAND); }
float cFC4Demand::QueryActiveDemandValue() const { return Get(FC4_DEMAND_ACTIVE_DEMAND); }
float cFC4Demand::QueryEconomyModifier() const { return Get(FC4_DEMAND_ECONOMY_MODIFIER); }
float cFC4Demand::QueryActiveDemandMax() const { return Get(FC4_DEMAND_ACTIVE_MAX); }
float cFC4Demand::QueryActiveDemandMin() const { return Get(FC4_DEMAND_ACTIVE_MIN); }

// docs/abi: the game returns these in ST0. [choice] 1.0 for "true" (§3.2 says
// they return true; the value in ST0 is not specified: spec question).
float cFC4Demand::AddToSupplyValue(float v) { return fc4_demand_node_add(core_, FC4_DEMAND_SUPPLY, v) ? 1.0f : 0.0f; }
float cFC4Demand::AddToDemandValue(float v) { return fc4_demand_node_add(core_, FC4_DEMAND_DEMAND, v) ? 1.0f : 0.0f; }

bool cFC4Demand::SetSupplyValue(float v) { return fc4_demand_node_set(core_, FC4_DEMAND_SUPPLY, v); }
bool cFC4Demand::SetDemandValue(float v) { return fc4_demand_node_set(core_, FC4_DEMAND_DEMAND, v); }
bool cFC4Demand::SetActiveDemandMax(float v) { return fc4_demand_node_set(core_, FC4_DEMAND_ACTIVE_MAX, v); }
bool cFC4Demand::SetActiveDemandMin(float v) { return fc4_demand_node_set(core_, FC4_DEMAND_ACTIVE_MIN, v); }
float cFC4Demand::GetEconomyModifier() const { return Get(FC4_DEMAND_ECONOMY_MODIFIER); }
bool cFC4Demand::SetEconomyModifier(float v) { return fc4_demand_node_set(core_, FC4_DEMAND_ECONOMY_MODIFIER, v); }
float cFC4Demand::GetTaxModifier() const { return Get(FC4_DEMAND_TAX_MODIFIER); }
bool cFC4Demand::SetTaxModifier(float v) { return fc4_demand_node_set(core_, FC4_DEMAND_TAX_MODIFIER, v); }

SC4Percentage* cFC4Demand::GetDemandCap() const {
    cap_.percentage = Get(FC4_DEMAND_DEMAND_CAP);
    return &cap_;
}

bool cFC4Demand::SetDemandCap(SC4Percentage const& c) {
    return fc4_demand_node_set(core_, FC4_DEMAND_DEMAND_CAP, c.percentage);
}

uint32_t cFC4Demand::GetRegionUse() { return fc4_demand_node_region_use(core_); }
bool cFC4Demand::SetRegionUse(uint32_t v) { return fc4_demand_node_set_region_use(core_, v); }
bool cFC4Demand::EndOfCycle() { return fc4_demand_node_end_of_cycle(core_); }
void cFC4Demand::DebugLockValue(float) {}
void cFC4Demand::DebugUnlockValue() {}
bool cFC4Demand::DebugIsValueLocked() { return false; }

bool cFC4Demand::Write(cIGZOStream& s) {
    fc4::OStreamAdapter a(s);
    return fc4_demand_node_write(core_, a.Get());
}

bool cFC4Demand::Read(cIGZIStream& s) {
    fc4::IStreamAdapter a(s);
    return fc4_demand_node_read(core_, a.Get());
}

uint32_t cFC4Demand::GetGZCLSID() { return kFC4_CLSID_cSC4Demand; }
