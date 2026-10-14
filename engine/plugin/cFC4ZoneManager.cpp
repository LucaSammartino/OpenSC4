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

#include "cFC4ZoneManager.h"

#include "FC4ZonesShadow.h"

#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4GameList.h"
#include "FC4GameSet.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"

#include "GZServPtrs.h"
#include "SC4CellRegion.h"
#include "SC4List.h"
#include "SC4Rect.h"
#include "cGZPersistResourceKey.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4BudgetSimulator.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4City.h"
#include "cISC4Demolition.h"
#include "cISC4LandfillDeveloper.h"
#include "cISC4Lot.h"
#include "cISC4LotConfiguration.h"
#include "cISC4LotDeveloper.h"
#include "cISC4LotManager.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4PollutionSimulator.h"
#include "cISC4ZoneDeveloper.h"
#include "cISTETerrain.h"
#include "cRZCOMDllDirector.h"

#include <atomic>
#include <cstdio>
#include <cstring>
#include <mutex>
#include <type_traits>
#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cIGZSerializable = 0xE4FDA3D4;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4NetworkOccupant = 0xA821EF94;  // §5.1, §12

// Messages (§5).
constexpr uint32_t kMsgInsertOccupant = 0x99EF1142;
constexpr uint32_t kMsgZoningChange = 0x07699455;

// §6: the landfill occupant type of the occupant manager's slot 33 lookup.
constexpr uint32_t kOccupantTypeLandfill = 0x6A11BD2B;
// §6: FindNetworkTile(x, z, 0x1F4F, 1).
constexpr uint32_t kNetworkTileMask = 0x1F4F;

// fc4.h: the core refuses a region of more cells or a longer side.
constexpr int64_t kMaxRegionCells = int64_t{1} << 20;
constexpr int64_t kMaxRegionSide = 4096;
// The outLots set's consistency walk accepts at most this many nodes.
constexpr uint32_t kMaxOutLots = 1u << 20;

fc4::ZonesContext* Ctx(void* u) { return static_cast<fc4::ZonesContext*>(u); }
cISC4City* City(void* u) { return Ctx(u)->city; }

// The zones_* switches; the shadow's context reads the game as it is.
bool Gate(void* u, fc4::Sw s, const char* what) {
    if (!Ctx(u)->gated || fc4::On(s)) return true;
    fc4::NoteSkipped(s, what);
    return false;
}

// Test seam op 38: a callback of the test table answers instead of the game.
#define ZONES_TEST(name, ...)                                                                  \
    do {                                                                                       \
        const fc4_zones_host* t_ = Ctx(u)->test;                                               \
        if (t_ != nullptr && t_->name != nullptr) return t_->name(t_->user, __VA_ARGS__);      \
    } while (0)
#define ZONES_TEST0(name)                                                                      \
    do {                                                                                       \
        const fc4_zones_host* t_ = Ctx(u)->test;                                               \
        if (t_ != nullptr && t_->name != nullptr) return t_->name(t_->user);                   \
    } while (0)

// Main thread only (host callbacks run under the zone manager's lock or in
// the shadow's comparison).
void NoteOnce(bool& done, const char* text) {
    if (done) return;
    done = true;
    fc4::Log("%s", text);
}
bool g_noteNoAllocator = false;
bool g_noteBadSet = false;
bool g_noteList = false;

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

// The outLots set's node allocator: the game's (B), or the test seam's.
std::atomic<void* (*)(uint32_t)> g_testSetAllocate{nullptr};
std::atomic<void (*)(void*)> g_testSetFree{nullptr};

struct SetAllocator {
    static bool Available() { return g_testSetAllocate.load() != nullptr || fc4::StlNodeAllocator::Available(); }
    static void* Allocate(uint32_t size) {
        if (auto a = g_testSetAllocate.load()) return a(size);
        return fc4::StlNodeAllocator::Allocate(size);
    }
    static void Free(void* p) {
        if (auto f = g_testSetFree.load()) return f(p);
        fc4::StlNodeAllocator::Free(p);
    }
};

// The zone manager the city runs (Init to Shutdown), for the month line.
std::atomic<cFC4ZoneManager*> g_live{nullptr};

// Live instances, for the test seam (op 38).
std::mutex g_instancesLock;
std::vector<cFC4ZoneManager*> g_instances;

// ---- lifecycle and services (§1.1, §10.1) ---------------------------------------------------------

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

// A service still null takes `value` with a reference of its own.
template <class T>
void TakeIfNull(T*& slot, T* value) {
    if (slot != nullptr || value == nullptr) return;
    slot = value;
    FC4_ADDREF(slot);
}

template <class T>
void ReleaseSlot(T*& slot) {
    if (slot != nullptr) FC4_RELEASE(slot);
    slot = nullptr;
}

// §1.1 step 2: the city, occupant manager, terrain, pollution and budget
// simulators that are still null (a Read may have set them, §10.1), each
// with a reference, and the message server. The original takes them from
// globals the city fills; FreeCity4 takes them from the app's city.
void AcquireServices(void* u) {
    ZONES_TEST0(acquire_services);
    fc4::ZonesContext* c = Ctx(u);
    if (c->readOnly) return;
    TakeIfNull(c->city, AppCity());
    if (c->city != nullptr) {
        TakeIfNull(c->occupants, c->city->GetOccupantManager());  // city slot 34
        TakeIfNull(c->terrain, c->city->GetTerrain());
        TakeIfNull(c->pollution, c->city->GetPollutionSimulator());
        TakeIfNull(c->budget, c->city->GetBudgetSimulator());  // city slot 52
    } else {
        fc4::Log("zones: no city; services not acquired");
    }
    if (c->msgServer == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            c->msgServer = ms;
            FC4_ADDREF(c->msgServer);
        }
    }
}

// §1.1 Shutdown: the message server, budget, pollution, terrain, city,
// occupant manager, in that order.
void ReleaseServices(void* u) {
    ZONES_TEST0(release_services);
    fc4::ZonesContext* c = Ctx(u);
    if (c->readOnly) return;
    ReleaseSlot(c->msgServer);
    ReleaseSlot(c->budget);
    ReleaseSlot(c->pollution);
    ReleaseSlot(c->terrain);
    ReleaseSlot(c->city);
    ReleaseSlot(c->occupants);
}

void Subscribe(void* u, uint32_t msg) {
    ZONES_TEST(subscribe, msg);
    fc4::ZonesContext* c = Ctx(u);
    if (c->readOnly || !Gate(u, fc4::Sw::ZonesMessages, "zones InsertOccupant subscription")) return;
    if (c->msgServer != nullptr && c->target != nullptr) c->msgServer->AddNotification(c->target, msg);
}

void Unsubscribe(void* u, uint32_t msg) {
    ZONES_TEST(unsubscribe, msg);
    fc4::ZonesContext* c = Ctx(u);
    if (c->readOnly) return;
    if (c->msgServer != nullptr && c->target != nullptr) c->msgServer->RemoveNotification(c->target, msg);
}

// §4: the "Zone Manager" exemplar at every Init; a missing one leaves the
// arrays as they are (the core zeroes the costs first).
fc4_props* Tuning(void* u) {
    ZONES_TEST0(tuning);
    if (!Gate(u, fc4::Sw::ExemplarLoads, "zones tuning exemplar load (none)")) return nullptr;
    fc4::props::Exemplar ex;
    if (!ex.Load(FC4_ZONES_TUNING_TYPE, FC4_ZONES_TUNING_GROUP, FC4_ZONES_TUNING_INSTANCE)) {
        if (!Ctx(u)->readOnly) fc4::Log("zones: tuning exemplar 0x%08X unavailable", FC4_ZONES_TUNING_INSTANCE);
        return nullptr;
    }
    return fc4::props::ToCoreAll(ex.Holder());
}

// City slots 93, 94.
void CellCounts(void* u, int32_t* x, int32_t* z) {
    ZONES_TEST(cell_counts, x, z);
    cISC4City* c = City(u);
    if (x != nullptr) *x = c ? static_cast<int32_t>(c->CellCountX()) : 0;
    if (z != nullptr) *z = c ? static_cast<int32_t>(c->CellCountZ()) : 0;
}

cIGZUnknown* ServiceOf(fc4::ZonesContext* c, uint32_t which) {
    switch (which) {
        case FC4_ZONES_SERVICE_CITY: return reinterpret_cast<cIGZUnknown*>(c->city);
        case FC4_ZONES_SERVICE_OCCUPANT_MANAGER: return reinterpret_cast<cIGZUnknown*>(c->occupants);
        case FC4_ZONES_SERVICE_TERRAIN: return reinterpret_cast<cIGZUnknown*>(c->terrain);
        case FC4_ZONES_SERVICE_POLLUTION: return reinterpret_cast<cIGZUnknown*>(c->pollution);
        case FC4_ZONES_SERVICE_BUDGET: return reinterpret_cast<cIGZUnknown*>(c->budget);
        default: return nullptr;
    }
}

// §10.1 field 13: the service's interface (the IID it is read back with).
uint64_t ServiceRef(void* u, uint32_t which) {
    ZONES_TEST(service_ref, which);
    return TokenOf(ServiceOf(Ctx(u), which));
}

// §10.1 Read: the token (its IID's interface) comes with one reference,
// which replaces the service held. ZG4 (§10.1): on a second Read the
// original nulls each field and reads the new reference without releasing
// the old one (each leaks once). [choice] (recommended) the old ones are
// released here.
void AdoptService(void* u, uint32_t which, uint64_t token) {
    ZONES_TEST(adopt_service, which, token);
    fc4::ZonesContext* c = Ctx(u);
    cIGZUnknown* const fresh = FromToken(token);
    auto put = [&](auto*& slot) {
        cIGZUnknown* const old = reinterpret_cast<cIGZUnknown*>(slot);
        slot = reinterpret_cast<std::remove_reference_t<decltype(slot)>>(fresh);
        if (old != nullptr) FC4_RELEASE(old);
    };
    if (c->readOnly) {
        return;  // the shadow borrows (the shadow core never adopts)
    }
    switch (which) {
        case FC4_ZONES_SERVICE_CITY: put(c->city); break;
        case FC4_ZONES_SERVICE_OCCUPANT_MANAGER: put(c->occupants); break;
        case FC4_ZONES_SERVICE_TERRAIN: put(c->terrain); break;
        case FC4_ZONES_SERVICE_POLLUTION: put(c->pollution); break;
        case FC4_ZONES_SERVICE_BUDGET: put(c->budget); break;
        default:
            if (fresh != nullptr) FC4_RELEASE(fresh);
            break;
    }
}

// §5.3: MessagePost of ZoningChange, void1 = IGZUnknown = the zone manager,
// void2 = the changed-cell region (owned by the message), data3 = the type,
// data4 = checkSlope.
void PostZoningChange(void* u, const fc4_zone_region* region, int32_t type, bool checkSlope) {
    ZONES_TEST(post_zoning_change, region, type, checkSlope);
    fc4::ZonesContext* c = Ctx(u);
    if (c->readOnly || region == nullptr || !Gate(u, fc4::Sw::ZonesPosts, "zones ZoningChange post")) return;
    cIGZMessageServer2* ms = c->msgServer;
    if (ms == nullptr && c->postSink == nullptr) return;
    cFC4ZoningChangeMessage* m = cFC4ZoningChangeMessage::New(*region, type, checkSlope, c->self);
    if (m == nullptr) return;
    m->AddRef();
    ++c->posts;
    if (c->postSink != nullptr) c->postSink(c->postSinkContext, m);
    if (ms != nullptr) ms->MessagePost(m, false);
    m->Release();
}

// ---- the zone developer / lot manager (§13.1) -----------------------------------------------------

cFC4ISC4LotManager* LotManager(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(u, fc4::Sw::ZonesLots, what)) return nullptr;
    cISC4LotManager* lm = c->GetLotManager();
    return lm ? fc4::AsGame(lm) : nullptr;
}

// Lot manager slot 6 GetLot(x, z, false): borrowed.
uint64_t GetLot(void* u, int32_t x, int32_t z) {
    ZONES_TEST(get_lot, x, z);
    cFC4ISC4LotManager* lm = LotManager(u, "zones GetLot (none)");
    return lm ? TokenOf(lm->GetLot(x, z, false)) : 0;
}

// Lot manager slot 7 into a list of the game's layout (B nodes, one
// reference per lot, allocators.md "GetLotsInCellRect"): each reference is
// handed to the core (it releases it), then the nodes are freed.
void LotsInCellRect(void* u, const fc4_cell_rect* rect, bool originOnly, fc4_token_callback cb, void* cbCtx) {
    ZONES_TEST(lots_in_cell_rect, rect, originOnly, cb, cbCtx);
    if (rect == nullptr || cb == nullptr) return;
    cFC4ISC4LotManager* lm = LotManager(u, "zones GetLotsInCellRect (none)");
    if (lm == nullptr) return;
    fc4::GameList<cISC4Lot*> list;
    if (!list.valid()) {
        NoteOnce(g_noteList, "zones: GetLotsInCellRect refused (no STL node allocator): no lots listed");
        return;
    }
    lm->GetLotsInCellRect(list, rect->x1, rect->z1, rect->x2, rect->z2, originOnly);
    for (cISC4Lot*& lot : list) {
        if (lot == nullptr) continue;
        FC4_ACQUIRED(lot, "GetLotsInCellRect list entry");
        cb(cbCtx, TokenOf(lot));  // the core owns that reference now
        lot = nullptr;            // the list's destructor then only frees its nodes
    }
}

// Zone developer slot 6 DoParcellization(region, type, flag) on a region of
// ours (the developer neither frees nor keeps it, §13.2).
void DoParcellization(void* u, const fc4_zone_region* region, int32_t type, bool flag) {
    ZONES_TEST(do_parcellization, region, type, flag);
    fc4::ZonesContext* c = Ctx(u);
    if (c->readOnly || region == nullptr || c->city == nullptr) return;
    if (!Gate(u, fc4::Sw::ZonesParcellization, "zones DoParcellization")) return;
    cISC4ZoneDeveloper* zd = c->city->GetZoneDeveloper();
    if (zd == nullptr) return;
    fc4::ZoneRegionBuffer rb;
    if (!rb.Build(*region)) return;
    ++c->parcellizations;
    zd->DoParcellization(rb.Game(), static_cast<cISC4ZoneManager::ZoneType>(type), flag);
    FC4_X87_CHECK("zones DoParcellization");
}

// ---- budget, demolition, landfill (§6) ------------------------------------------------------------

cFC4ISC4BudgetSimulator* Budget(void* u) {
    cISC4BudgetSimulator* b = Ctx(u)->budget;
    return b ? fc4::AsGame(b) : nullptr;
}

// Budget slot 6 (EDX:EAX). [choice] (ZG9: the original has no switches and
// calls the budget with no null test) zones_budget off: unlimited funds.
int64_t TotalFunds(void* u) {
    ZONES_TEST0(total_funds);
    if (!Gate(u, fc4::Sw::ZonesBudget, "zones GetTotalFunds (unlimited)")) return INT64_MAX;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    return b ? b->GetTotalFunds() : 0;
}

// Budget slot 8 (pop 8, AL). [choice] zones_budget off: nothing charged, true.
bool WithdrawFunds(void* u, int64_t amount) {
    ZONES_TEST(withdraw_funds, amount);
    fc4::ZonesContext* c = Ctx(u);
    if (c->readOnly) return true;
    if (!Gate(u, fc4::Sw::ZonesBudget, "zones WithdrawFunds (nothing charged)")) return true;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    if (b == nullptr) return false;
    ++c->withdrawals;
    return b->WithdrawFunds(amount);
}

// Demolition slot 7 DemolishLots(commit, lots, checkSlope, flags, 0, 0,
// &cost / 0, 0) (§6). The list is only walked by the game (allocators.md:
// any allocator works, no element is released); it borrows the core's lots.
int64_t DemolishLots(void* u, bool commit, const uint64_t* lots, size_t count, bool checkSlope, uint32_t flags) {
    ZONES_TEST(demolish_lots, commit, lots, count, checkSlope, flags);
    fc4::ZonesContext* c = Ctx(u);
    if (commit && c->readOnly) return 0;  // never: the shadow prices only
    if (c->city == nullptr || !Gate(u, fc4::Sw::ZonesDemolition, "zones DemolishLots (cost 0)")) return 0;
    cISC4Demolition* d = c->city->GetDemolitionUtility();
    if (d == nullptr) return 0;
    int64_t cost = 0;
    auto run = [&](auto& list) {
        for (size_t i = 0; i < count; ++i) {
            cISC4Lot* lot = reinterpret_cast<cISC4Lot*>(FromToken(lots[i]));
            if (lot != nullptr) list.push_back(lot);
        }
        d->DemolishLots(commit, reinterpret_cast<const SC4List<cISC4Lot*>&>(list), checkSlope ? 1 : 0, flags, false,
                        nullptr, commit ? nullptr : &cost, 0);
        for (cISC4Lot*& lot : list) lot = nullptr;  // borrowed: never released by the list
    };
    if (commit) {
        ++c->demolitions;
    } else {
        ++c->pricings;
    }
    fc4::GameList<cISC4Lot*> list;
    if (list.valid()) {
        run(list);
    } else {
        SC4List<cISC4Lot*> sdk;  // the allocator service (A): read only by the game
        run(sdk);
    }
    if (commit) FC4_X87_CHECK("zones DemolishLots");
    return cost;
}

// The landfill developer (city slot 78), slot 9 GetZoningCost of the region (EDX:EAX).
int64_t LandfillZoningCost(void* u, const fc4_zone_region* region) {
    ZONES_TEST(landfill_zoning_cost, region);
    cISC4City* c = City(u);
    if (c == nullptr || region == nullptr || !Gate(u, fc4::Sw::ZonesLandfillCost, "zones GetZoningCost (0)")) {
        return 0;
    }
    cISC4LandfillDeveloper* lf = c->GetLandfillDeveloper();  // city slot 78
    if (lf == nullptr) return 0;
    fc4::ZoneRegionBuffer rb;
    if (!rb.Build(*region)) return 0;
    // Through the verified declaration (EDX:EAX), spelled as a member pointer:
    // the SDK row's waiver for slot 9 (abi_conformance_test) is voided by any
    // plain arrow or dot call of that name in the plugin sources.
    constexpr auto kGetZoningCost = &cFC4ISC4LandfillDeveloper::GetZoningCost;
    return (fc4::AsGame(lf)->*kGetZoningCost)(reinterpret_cast<const SC4CellRegion<int32_t>*>(rb.Raw()));
}

// ---- network tiles (§6) -----------------------------------------------------------------------------

// The traffic network map (city slot 40) slot 8 FindNetworkTile(x, z,
// 0x1F4F, true): a cISC4NetworkOccupant (its slots 22, 48 and 63 are the
// tile methods of §12), borrowed.
uint64_t FindNetworkTile(void* u, int32_t x, int32_t z) {
    ZONES_TEST(find_network_tile, x, z);
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(u, fc4::Sw::ZonesTraffic, "zones FindNetworkTile (none)")) return 0;
    auto* map = reinterpret_cast<cFC4ISC4TrafficNetworkMap*>(c->GetTrafficNetwork());
    if (map == nullptr) return 0;
    return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(map->FindNetworkTile(x, z, kNetworkTileMask, true)));
}

cISC4NetworkOccupant* TileOf(uint64_t t) { return reinterpret_cast<cISC4NetworkOccupant*>(FromToken(t)); }

// tile.AsOccupant() (slot 48, borrowed).IsFlagSet(flag) (occupant slot 32).
bool TileOccupantFlagSet(void* u, uint64_t tile, uint32_t flag) {
    ZONES_TEST(tile_occupant_flag_set, tile, flag);
    if (tile == 0 || !Gate(u, fc4::Sw::ZonesTraffic, "zones tile IsFlagSet (false)")) return false;
    cISC4Occupant* o = TileOf(tile)->AsOccupant();
    return o != nullptr && o->IsFlagSet(flag);
}

// tile.IsOfType(type) (slot 22).
bool TileIsOfType(void* u, uint64_t tile, uint32_t type) {
    ZONES_TEST(tile_is_of_type, tile, type);
    if (tile == 0 || !Gate(u, fc4::Sw::ZonesTraffic, "zones tile IsOfType (false)")) return false;
    return TileOf(tile)->IsOfType(static_cast<cISC4NetworkOccupant::eNetworkType>(type));
}

// tile.GetCostOfDemolition() (slot 63, EDX:EAX).
int64_t TileDemolitionCost(void* u, uint64_t tile) {
    ZONES_TEST(tile_demolition_cost, tile);
    if (tile == 0 || !Gate(u, fc4::Sw::ZonesTraffic, "zones tile GetCostOfDemolition (0)")) return 0;
    return TileOf(tile)->GetCostOfDemolition();
}

// ---- terrain, lot developer ------------------------------------------------------------------------

// Terrain slot 34 CellHasWater (AL).
bool CellHasWater(void* u, int32_t x, int32_t z) {
    ZONES_TEST(cell_has_water, x, z);
    cISTETerrain* t = Ctx(u)->terrain;
    if (t == nullptr || x < 0 || z < 0 || !Gate(u, fc4::Sw::ZonesTerrain, "zones CellHasWater (dry)")) return false;
    return t->CellHasWater(static_cast<uint32_t>(x), static_cast<uint32_t>(z));
}

cISC4LotDeveloper* LotDeveloper(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(u, fc4::Sw::ZonesLotDeveloper, what)) return nullptr;
    cISC4BuildingDevelopmentSimulator* bds = c->GetBuildingDevelopmentSimulator();
    return bds ? bds->GetLotDeveloper() : nullptr;  // BDS slot 36, borrowed
}

// Lot developer slot 29 GetAreaSlope(rect, withWater): a float in ST0,
// declared float (the x87 rule) and used.
float AreaSlope(void* u, const fc4_cell_rect* rect, bool withWater) {
    ZONES_TEST(area_slope, rect, withWater);
    if (rect == nullptr) return 0.0f;
    cISC4LotDeveloper* ld = LotDeveloper(u, "zones GetAreaSlope (flat)");
    if (ld == nullptr) return 0.0f;
    const SC4Rect<int32_t> r(rect->x1, rect->z1, rect->x2, rect->z2);
    return ld->GetAreaSlope(r, withWater);
}

// Lot developer slot 43 SetEmptyLotBaseTexture(lot) (void).
void SetEmptyLotBaseTexture(void* u, uint64_t lot) {
    ZONES_TEST(set_empty_lot_base_texture, lot);
    fc4::ZonesContext* c = Ctx(u);
    if (c->readOnly || lot == 0) return;
    cISC4LotDeveloper* ld = LotDeveloper(u, "zones SetEmptyLotBaseTexture");
    if (ld == nullptr) return;
    ++c->textures;
    ld->SetEmptyLotBaseTexture(reinterpret_cast<cISC4Lot*>(FromToken(lot)));
}

// ---- occupants (§5.1, §6) ---------------------------------------------------------------------------

// Occupant manager slot 33 (&out, x, z, 0x6A11BD2B) (docs/abi: AL = found,
// *out AddRef'd on a hit): the reference is kept until the outermost call
// returns, so the token is borrowed for the core. [abi] The SDK declares the
// typed overload first and the filter one second; MSVC groups and reverses
// overloads, which gives docs/abi's order (32 = filter, 33 = typed), as the
// landfill glue's slot 41 relies on.
uint64_t LandfillAt(void* u, int32_t x, int32_t z) {
    ZONES_TEST(landfill_at, x, z);
    fc4::ZonesContext* c = Ctx(u);
    if (c->occupants == nullptr || !Gate(u, fc4::Sw::ZonesOccupants, "zones landfill occupant lookup (none)")) {
        return 0;
    }
    cISC4Occupant* o = nullptr;
    if (!c->occupants->GetFirstOccupantByStandardCityCell(o, x, z, kOccupantTypeLandfill) || o == nullptr) return 0;
    FC4_ACQUIRED(o, "GetOccupantAtCell (typed)");
    c->held.push_back(o);
    return TokenOf(o);
}

// Occupant slot 11 GetBoundingCityCells (through cISC4Occupant).
void OccupantBoundingCells(void* u, uint64_t occupant, fc4_cell_rect* out) {
    ZONES_TEST(occupant_bounding_cells, occupant, out);
    if (out == nullptr) return;
    *out = fc4_cell_rect{0, 0, -1, -1};
    if (!Gate(u, fc4::Sw::ZonesOccupants, "zones occupant GetBoundingCityCells (empty)")) return;
    cISC4Occupant* o = Query<cISC4Occupant>(FromToken(occupant), kIID_cISC4Occupant);
    if (o == nullptr) return;
    SC4Rect<long> r;
    if (o->GetBoundingCityCells(r)) {
        *out = fc4_cell_rect{static_cast<int32_t>(r.topLeftX), static_cast<int32_t>(r.topLeftY),
                             static_cast<int32_t>(r.bottomRightX), static_cast<int32_t>(r.bottomRightY)};
    }
    Drop(o);
}

// Occupant slot 14 CanRemove(flags) (through cISC4Occupant).
bool OccupantCanRemove(void* u, uint64_t occupant, uint32_t flags) {
    ZONES_TEST(occupant_can_remove, occupant, flags);
    if (!Gate(u, fc4::Sw::ZonesOccupants, "zones occupant CanRemove (false)")) return false;
    cISC4Occupant* o = Query<cISC4Occupant>(FromToken(occupant), kIID_cISC4Occupant);
    if (o == nullptr) return false;
    const bool can = o->CanRemove(flags);
    Drop(o);
    return can;
}

// QueryInterface(cISC4NetworkOccupant) then slot 9 HasAnyNetworkFlag: 1 / 0,
// -1 = not a network occupant.
int32_t NetworkHasAnyFlag(void* u, uint64_t occupant, uint32_t flags) {
    ZONES_TEST(network_has_any_flag, occupant, flags);
    if (!Gate(u, fc4::Sw::ZonesOccupants, "zones HasAnyNetworkFlag (not a network)")) return -1;
    cISC4NetworkOccupant* n = Query<cISC4NetworkOccupant>(FromToken(occupant), kIID_cISC4NetworkOccupant);
    if (n == nullptr) return -1;
    const bool any = n->HasAnyNetworkFlag(flags);
    Drop(n);
    return any ? 1 : 0;
}

// ---- references -------------------------------------------------------------------------------------

void AddRefToken(void* u, uint64_t t) {
    ZONES_TEST(add_ref, t);
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->AddRef();
    } else {
        FC4_ADDREF(o);
    }
}

void ReleaseToken(void* u, uint64_t t) {
    ZONES_TEST(release, t);
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->Release();
    } else {
        FC4_RELEASE(o);
    }
}

// ---- lots and lot configurations (cISC4Lot slots 9, 14, 31, 33, 48) -----------------------------------

cFC4ISC4Lot* LotOf(uint64_t t) { return fc4::AsGame(reinterpret_cast<cISC4Lot*>(FromToken(t))); }

bool LotsOn(void* u, const char* what) { return Gate(u, fc4::Sw::ZonesLots, what); }

uint64_t LotConfiguration(void* u, uint64_t lot) {
    ZONES_TEST(lot_configuration, lot);
    if (lot == 0 || !LotsOn(u, "zones lot GetLotConfiguration (none)")) return 0;
    return TokenOf(LotOf(lot)->GetLotConfiguration());
}

void LotBoundingRect(void* u, uint64_t lot, fc4_cell_rect* out) {
    ZONES_TEST(lot_bounding_rect, lot, out);
    if (out == nullptr) return;
    *out = fc4_cell_rect{0, 0, -1, -1};
    if (lot == 0 || !LotsOn(u, "zones lot GetBoundingRect (empty)")) return;
    SC4Rect<int32_t> r;
    if (LotOf(lot)->GetBoundingRect(r)) *out = fc4_cell_rect{r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY};
}

int32_t LotZoneType(void* u, uint64_t lot) {
    ZONES_TEST(lot_zone_type, lot);
    if (lot == 0 || !LotsOn(u, "zones lot GetZoneType (0)")) return 0;
    return static_cast<int32_t>(LotOf(lot)->GetZoneType());
}

uint32_t LotState(void* u, uint64_t lot) {
    ZONES_TEST(lot_state, lot);
    if (lot == 0 || !LotsOn(u, "zones lot GetState (0)")) return 0;
    return static_cast<uint32_t>(LotOf(lot)->GetState());
}

uint64_t LotBuilding(void* u, uint64_t lot) {
    ZONES_TEST(lot_building, lot);
    if (lot == 0 || !LotsOn(u, "zones lot GetBuilding (none)")) return 0;
    return TokenOf(LotOf(lot)->GetBuilding());
}

// Lot configuration slot 15 IsCompatibleWithZoneType(type) (AL).
bool CfgCompatibleWithZone(void* u, uint64_t cfg, int32_t type) {
    ZONES_TEST(cfg_compatible_with_zone, cfg, type);
    if (cfg == 0 || !LotsOn(u, "zones IsCompatibleWithZoneType (true)")) return true;
    auto* c = reinterpret_cast<cISC4LotConfiguration*>(FromToken(cfg));
    return c->IsCompatibleWithZoneType(static_cast<cISC4ZoneManager::ZoneType>(type));
}

// ---- PlaceZone's outLots (§6, §13.2) ------------------------------------------------------------------

struct OutLotsSink {
    fc4::ZonesContext* ctx;
    void* set;  // std::set<cRZAutoRefCount<cISC4Lot>>*: the STLport tree (allocators.md)
};

// The core hands every lot to demolish over at the pricing call (borrowed):
// a unique insert into the caller's set, AddRef only on a new insertion (the
// caller releases each element and frees the nodes with B).
void OutLotsInsert(void* p, uint64_t token) {
    OutLotsSink* s = static_cast<OutLotsSink*>(p);
    if (s == nullptr || s->set == nullptr || token == 0) return;
    if (!Gate(s->ctx, fc4::Sw::ZonesOutLots, "zones outLots insertion")) return;
    if (!SetAllocator::Available()) {
        ++s->ctx->outLotsRefused;
        NoteOnce(g_noteNoAllocator, "zones: WARNING outLots not filled (no STL node allocator)");
        return;
    }
    cIGZUnknown* lot = FromToken(token);
    fc4::GameMapStatus status = fc4::GameMapStatus::Ok;
    const fc4::GameSetResult r = fc4::GameSetInsertU32<SetAllocator>(
        static_cast<fc4::GameSetObject*>(s->set), static_cast<uint32_t>(reinterpret_cast<uintptr_t>(lot)),
        kMaxOutLots, &status);
    if (r == fc4::GameSetResult::Inserted) {
        ++s->ctx->outLotsInserted;
        FC4_ADDREF_FOR_CALLER(lot);
    } else if (r == fc4::GameSetResult::Refused) {
        ++s->ctx->outLotsRefused;
        if (!g_noteBadSet) {
            g_noteBadSet = true;
            fc4::Log("zones: WARNING outLots left alone (%s)", fc4::GameMapStatusName(status));
        }
    }
}

}  // namespace

namespace fc4 {

// ---- regions (zones.md §11.4) -------------------------------------------------------------------------

namespace {

struct GameCellMapView {
    const void* vtbl;
    uint32_t width;
    uint32_t height;
    uint32_t words;
    uint32_t** rows;
};
struct GameCellRegionView {
    int32_t x1, z1, x2, z2;
    GameCellMapView map;
};
static_assert(sizeof(void*) != 4 || sizeof(GameCellRegionView) == 0x24, "SC4CellRegion<long> layout");

bool g_noteRegion = false;
uint8_t g_emptyMask = 0;

bool Fits(int64_t w, int64_t h) { return w <= kMaxRegionSide && h <= kMaxRegionSide && w * h <= kMaxRegionCells; }

}  // namespace

bool ReadZoneRegion(const void* game, std::vector<uint8_t>& mask, fc4_zone_region* out, const char* tag) {
    if (game == nullptr || out == nullptr) return false;
    const GameCellRegionView* g = static_cast<const GameCellRegionView*>(game);
    const int64_t w = int64_t{g->x2} - g->x1 + 1;
    const int64_t h = int64_t{g->z2} - g->z1 + 1;
    *out = fc4_zone_region{};
    out->x0 = g->x1;
    out->z0 = g->z1;
    out->x1 = g->x2;
    out->z1 = g->z2;
    mask.clear();
    if (w <= 0 || h <= 0) {
        // An empty rectangle: no cell, whatever the map says.
        out->mask = &g_emptyMask;
        out->len = 0;
        return true;
    }
    if (!Fits(w, h) || g->map.width != static_cast<uint64_t>(w) || g->map.height != static_cast<uint64_t>(h) ||
        g->map.words < static_cast<uint32_t>((h + 31) / 32) || g->map.rows == nullptr) {
        if (!g_noteRegion) {
            g_noteRegion = true;
            Log("%s: WARNING a cell region is not in the documented layout or too large (rect %d,%d..%d,%d, map "
                "%ux%u, %u words per row); refused",
                tag, g->x1, g->z1, g->x2, g->z2, g->map.width, g->map.height, g->map.words);
        }
        return false;
    }
    mask.assign(static_cast<size_t>(w * h), 0);
    for (int64_t dx = 0; dx < w; ++dx) {
        const uint32_t* row = g->map.rows[dx];
        if (row == nullptr) continue;
        for (int64_t dz = 0; dz < h; ++dz) {
            if ((row[dz / 32] >> (dz % 32)) & 1u) mask[static_cast<size_t>(dz * w + dx)] = 1;
        }
    }
    out->mask = mask.data();
    out->len = mask.size();
    return true;
}

uint32_t ClearRejectedBits(void* game, const fc4_zone_region& r) {
    if (game == nullptr || r.mask == nullptr || r.len == 0) return 0;
    GameCellRegionView* g = static_cast<GameCellRegionView*>(game);
    const int64_t w = int64_t{r.x1} - r.x0 + 1;
    const int64_t h = int64_t{r.z1} - r.z0 + 1;
    if (w <= 0 || h <= 0 || static_cast<size_t>(w * h) != r.len) return 0;
    uint32_t cleared = 0;
    for (int64_t dx = 0; dx < w; ++dx) {
        uint32_t* row = g->map.rows[dx];
        if (row == nullptr) continue;
        for (int64_t dz = 0; dz < h; ++dz) {
            const uint32_t bit = 1u << (dz % 32);
            if (r.mask[static_cast<size_t>(dz * w + dx)] == 0 && (row[dz / 32] & bit) != 0) {
                row[dz / 32] &= ~bit;
                ++cleared;
            }
        }
    }
    return cleared;
}

bool ZoneRegionBuffer::Build(const fc4_zone_region& in) {
    const int64_t w = int64_t{in.x1} - in.x0 + 1;
    const int64_t h = int64_t{in.z1} - in.z0 + 1;
    layout_.x1 = in.x0;
    layout_.z1 = in.z0;
    layout_.x2 = in.x1;
    layout_.z2 = in.z1;
    bits_.clear();
    rows_.clear();
    if (w <= 0 || h <= 0) {
        layout_.map.width = layout_.map.height = layout_.map.words = 0;
        layout_.map.rows = nullptr;
        return true;
    }
    if (!Fits(w, h)) return false;
    const uint32_t words = static_cast<uint32_t>((h + 31) / 32);
    bits_.assign(static_cast<size_t>(w) * words, 0);
    rows_.resize(static_cast<size_t>(w));
    for (int64_t dx = 0; dx < w; ++dx) rows_[static_cast<size_t>(dx)] = &bits_[static_cast<size_t>(dx) * words];
    for (int64_t dz = 0; dz < h; ++dz) {
        for (int64_t dx = 0; dx < w; ++dx) {
            const size_t i = static_cast<size_t>(dz * w + dx);
            const bool set = in.mask == nullptr || (i < in.len && in.mask[i] != 0);
            if (set) rows_[static_cast<size_t>(dx)][dz / 32] |= 1u << (dz % 32);
        }
    }
    layout_.map.width = static_cast<uint32_t>(w);
    layout_.map.height = static_cast<uint32_t>(h);
    layout_.map.words = words;
    layout_.map.rows = rows_.data();
    return true;
}

bool ZoneRegionBuffer::BuildFull(int32_t x1, int32_t z1, int32_t x2, int32_t z2) {
    fc4_zone_region r{};
    r.x0 = x1;
    r.z0 = z1;
    r.x1 = x2;
    r.z1 = z2;
    r.mask = nullptr;  // every cell set
    return Build(r);
}

bool ZoneRegionBuffer::Bit(int32_t x, int32_t z) const {
    const int64_t dx = int64_t{x} - layout_.x1;
    const int64_t dz = int64_t{z} - layout_.z1;
    if (dx < 0 || dz < 0 || dx >= layout_.map.width || dz >= layout_.map.height) return false;
    return ((layout_.map.rows[dx][dz / 32] >> (dz % 32)) & 1u) != 0;
}

// ---- the host tables ----------------------------------------------------------------------------------

fc4_zones_host MakeZonesHost(ZonesContext* ctx) {
    fc4_zones_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.grids = MakeGridApi(&ctx->grid);
    h.acquire_services = &AcquireServices;
    h.release_services = &ReleaseServices;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.tuning = &Tuning;
    h.cell_counts = &CellCounts;
    h.service_ref = &ServiceRef;
    h.adopt_service = &AdoptService;
    h.post_zoning_change = &PostZoningChange;
    h.get_lot = &GetLot;
    h.lots_in_cell_rect = &LotsInCellRect;
    h.do_parcellization = &DoParcellization;
    h.total_funds = &TotalFunds;
    h.withdraw_funds = &WithdrawFunds;
    h.demolish_lots = &DemolishLots;
    h.landfill_zoning_cost = &LandfillZoningCost;
    h.find_network_tile = &FindNetworkTile;
    h.tile_occupant_flag_set = &TileOccupantFlagSet;
    h.tile_is_of_type = &TileIsOfType;
    h.tile_demolition_cost = &TileDemolitionCost;
    h.cell_has_water = &CellHasWater;
    h.area_slope = &AreaSlope;
    h.set_empty_lot_base_texture = &SetEmptyLotBaseTexture;
    h.landfill_at = &LandfillAt;
    h.occupant_bounding_cells = &OccupantBoundingCells;
    h.occupant_can_remove = &OccupantCanRemove;
    h.network_has_any_flag = &NetworkHasAnyFlag;
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.lot_configuration = &LotConfiguration;
    h.lot_bounding_rect = &LotBoundingRect;
    h.lot_zone_type = &LotZoneType;
    h.lot_state = &LotState;
    h.lot_building = &LotBuilding;
    h.cfg_compatible_with_zone = &CfgCompatibleWithZone;
    return h;
}

fc4_zones_host MakeZonesReadOnlyHost(ZonesContext* ctx) {
    ctx->readOnly = true;
    ctx->gated = false;
    fc4_zones_host h = MakeZonesHost(ctx);
    // The grid is the original's, borrowed: read only.
    h.grids.create = nullptr;
    h.grids.set = nullptr;
    h.grids.fill = nullptr;
    h.grids.shutdown = nullptr;
    h.grids.release = nullptr;
    h.acquire_services = nullptr;
    h.release_services = nullptr;
    h.subscribe = nullptr;
    h.unsubscribe = nullptr;
    h.adopt_service = nullptr;
    h.post_zoning_change = nullptr;
    h.do_parcellization = nullptr;
    h.withdraw_funds = nullptr;
    h.set_empty_lot_base_texture = nullptr;
    return h;
}

void ZonesMonthLine(int32_t julian) {
    cFC4ZoneManager* z = g_live.load();
    if (z != nullptr) z->LogMonth(julian);
}

}  // namespace fc4

// ---- ZoningChange (§5.3, §13.1) ---------------------------------------------------------------------------

namespace {
std::atomic<long> g_liveRegions{0};
}

cFC4ZoningChangeMessage* cFC4ZoningChangeMessage::New(const fc4_zone_region& region, int32_t zoneType,
                                                      bool checkSlope, cIGZUnknown* zoneManager) {
    fc4::ZoneRegionBuffer* buf = new fc4::ZoneRegionBuffer();
    if (!buf->Build(region)) {
        delete buf;
        return nullptr;
    }
    cFC4ZoningChangeMessage* m = new cFC4ZoningChangeMessage();
    m->region_ = buf;
    ++g_liveRegions;
    m->SetType(kMsgZoningChange);
    m->SetVoid1(zoneManager);
    m->SetIGZUnknown(zoneManager);  // "IGZUnknown = the zone manager" (foundations §5.1 conventions)
    m->SetVoid2(buf->Raw());
    m->SetData3(static_cast<intptr_t>(zoneType));
    m->SetData4(checkSlope ? 1 : 0);
    return m;
}

long cFC4ZoningChangeMessage::LiveRegions() { return g_liveRegions.load(); }

cFC4ZoningChangeMessage::~cFC4ZoningChangeMessage() {
    delete region_;
    region_ = nullptr;
    --g_liveRegions;
}

// ---- object ---------------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4ZoneManager() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4ZoneManager*>(new cFC4ZoneManager()));
}

cFC4ZoneManager::cFC4ZoneManager() {
    InitializeCriticalSection(&lock_);
    mainTid_ = GetCurrentThreadId();  // the class store creates city components on the main thread
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    ctx_.self = Primary();
    host_ = fc4::MakeZonesHost(&ctx_);
    core_ = fc4_zones_create(&host_);
    if (core_ == nullptr) fc4::Log("cFC4ZoneManager: core creation failed");
    {
        std::lock_guard<std::mutex> g(g_instancesLock);
        g_instances.push_back(this);
    }
    RefreshSnapshot();
    fc4::Log("cFC4ZoneManager(%p) created, core %p", static_cast<void*>(Primary()), static_cast<void*>(core_));
}

cFC4ZoneManager::~cFC4ZoneManager() {
    {
        std::lock_guard<std::mutex> g(g_instancesLock);
        for (size_t i = 0; i < g_instances.size(); ++i) {
            if (g_instances[i] == this) {
                g_instances.erase(g_instances.begin() + static_cast<std::ptrdiff_t>(i));
                break;
            }
        }
    }
    cFC4ZoneManager* self = this;
    g_live.compare_exchange_strong(self, nullptr);
    fc4_zones_destroy(core_);  // shuts down and releases a grid a never-initialised object still holds
    core_ = nullptr;
    ReleaseHeld();
    // Services a Read handed over to an object that was never initialised.
    ReleaseServices(&ctx_);
    fc4::Log("cFC4ZoneManager destroyed (%u fallbacks)", fallbacks_.load());
    DeleteCriticalSection(&lock_);
}

// §1: which sub-object answers is the core's decision; anything else is
// false and the out pointer is left untouched.
bool cFC4ZoneManager::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr || !fc4_zones_query_interface(riid)) return false;
    if (riid == kIID_cIGZMessageTarget2) {
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
    } else if (riid == kIID_cIGZSerializable) {
        *ppvObj = static_cast<cIGZSerializable*>(this);
    } else {
        *ppvObj = Primary();  // 0x00000001 and 0x67652433
    }
    ++refCount_;
    return true;
}

uint32_t cFC4ZoneManager::AddRef() { return ++refCount_; }

// §1 (0x004DC430): count > 1 → decrement; otherwise Shutdown, then destroy.
uint32_t cFC4ZoneManager::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    Shutdown();
    delete this;
    return 0;
}

cFC4ZoneManager* cFC4ZoneManager::FromPrimary(const void* primary) {
    std::lock_guard<std::mutex> g(g_instancesLock);
    for (cFC4ZoneManager* z : g_instances) {
        if (static_cast<const void*>(z->Primary()) == primary ||
            static_cast<const void*>(static_cast<cIGZMessageTarget2*>(z)) == primary ||
            static_cast<const void*>(static_cast<cIGZSerializable*>(z)) == primary) {
            return z;
        }
    }
    return nullptr;
}

void cFC4ZoneManager::SetTestHooks(const fc4_zones_host* test, FC4PostSinkFn sink, void* sinkContext) {
    Section sec(this);
    ctx_.test = test;
    ctx_.postSink = sink;
    ctx_.postSinkContext = sinkContext;
    fc4_zones_destroy(core_);
    host_ = fc4::MakeZonesHost(&ctx_);
    if (test != nullptr && test->grids.create != nullptr) host_.grids = test->grids;
    core_ = fc4_zones_create(&host_);
    fc4::Log("zones: test hooks %s (core %p)", test ? "installed" : "removed", static_cast<void*>(core_));
}

// ---- the lock ---------------------------------------------------------------------------------------------

bool cFC4ZoneManager::Enter() {
    const DWORD me = GetCurrentThreadId();
    if (mainTid_ == 0 || me == mainTid_) {
        EnterCriticalSection(&lock_);
        return true;
    }
    if (TryEnterCriticalSection(&lock_)) return true;
    const ULONGLONG until = GetTickCount64() + kLockWaitMs;
    while (GetTickCount64() < until) {
        Sleep(1);
        if (TryEnterCriticalSection(&lock_)) return true;
    }
    ++fallbacks_;
    return false;
}

void cFC4ZoneManager::Leave() { LeaveCriticalSection(&lock_); }

cFC4ZoneManager::Section::Section(const cFC4ZoneManager* s) : s_(const_cast<cFC4ZoneManager*>(s)), ok_(s_->Enter()) {
    if (ok_) ++s_->depth_;
}

cFC4ZoneManager::Section::~Section() {
    if (!ok_) return;
    if (--s_->depth_ == 0) {
        // The outermost call: the references kept for it go, and the values
        // other threads fall back to are refreshed.
        s_->ReleaseHeld();
        s_->RefreshSnapshot();
    }
    s_->Leave();
}

void cFC4ZoneManager::ReleaseHeld() {
    std::vector<cIGZUnknown*> held;
    held.swap(ctx_.held);
    for (cIGZUnknown* o : held) FC4_RELEASE(o);
}

// Only the core's own state (no host calls): any thread holding the lock.
void cFC4ZoneManager::RefreshSnapshot() {
    if (core_ == nullptr) return;
    fc4_zones_state st{};
    st.struct_size = sizeof(st);
    if (fc4_zones_get_state(core_, &st)) {
        for (int t = 0; t < 16; ++t) {
            snap_.zoneCount[t] = st.zone_count[t];
            snap_.undeveloped[t] = st.undeveloped[t];
        }
        snap_.undevelopedTotal = st.undeveloped_total;
        snap_.abandoned = st.abandoned_tile_count;
        snap_.alpha = st.display_alpha;
        snap_.defaultAlpha = st.default_alpha;
        snap_.grid = reinterpret_cast<cISC4SimGrid<int8_t>*>(FromToken(st.grid));
        snap_.gridX = st.grid_count_x;
        snap_.gridZ = st.grid_count_z;
    }
    fc4_zones_tuning tu{};
    tu.struct_size = sizeof(tu);
    if (fc4_zones_get_tuning(core_, &tu)) {
        for (int t = 0; t < 16; ++t) {
            snap_.minSize[t] = tu.min_size[t];
            snap_.maxSize[t] = tu.max_size[t];
            snap_.texture[t] = tu.texture[t];
            snap_.dragColor[t] = tu.drag_color[t];
        }
    }
}

namespace {

void Join16(char* out, size_t cap, const uint32_t* v) {
    size_t used = 0;
    out[0] = 0;
    for (int t = 0; t < 16 && used < cap; ++t) {
        const int w = _snprintf_s(out + used, cap - used, _TRUNCATE, "%s%u", t ? "," : "", v[t]);
        if (w < 0) break;
        used += static_cast<size_t>(w);
    }
}

}  // namespace

// The status lines (Init, Read, Write, Shutdown, month). Inside a section.
void cFC4ZoneManager::LogState(const char* what) {
    fc4_zones_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_zones_get_state(core_, &st)) return;
    char counts[200];
    char undeveloped[200];
    Join16(counts, sizeof(counts), st.zone_count);
    Join16(undeveloped, sizeof(undeveloped), st.undeveloped);
    fc4::Log("zones %s: version=1 init=%u loaded=%u grid=%p %dx%d counts=%s undeveloped=%s total=%u abandoned=%u "
             "failures=%u alpha=%u/%u place=%u dezones=%u reentrant=%u posts=%u parcels=%u pricings=%u "
             "demolitions=%u outLots=%u/%u withdrawals=%u textures=%u fallbacks=%u",
             what, st.initialized, st.loaded, static_cast<void*>(FromToken(st.grid)), st.grid_count_x,
             st.grid_count_z, counts, undeveloped, st.undeveloped_total, st.abandoned_tile_count,
             st.failure1_entries, st.display_alpha, st.default_alpha, st.place_calls, st.dezones,
             st.reentrant_calls, ctx_.posts, ctx_.parcellizations, ctx_.pricings, ctx_.demolitions,
             ctx_.outLotsInserted, ctx_.outLotsRefused, ctx_.withdrawals, ctx_.textures, fallbacks_.load());
}

void cFC4ZoneManager::LogMonth(int32_t julian) {
    Section sec(this);
    if (!sec.ok()) return;
    fc4_date d{};
    fc4_date_from_julian(julian, &d);
    char what[48];
    _snprintf_s(what, sizeof(what), _TRUNCATE, "month %04d-%02d-%02d", d.year, d.month, d.day_of_month);
    LogState(what);
}

// ---- lifecycle (§1.1) -------------------------------------------------------------------------------------

bool cFC4ZoneManager::Init() {
    FC4_TRACE("ZoneManager::Init");
    mainTid_ = GetCurrentThreadId();
    Section sec(this);
    const bool ok = fc4_zones_init(core_);  // §1.1, all of it (the core drives every host call)
    g_live = this;
    LogState("Init");
    return ok;
}

bool cFC4ZoneManager::Shutdown() {
    FC4_TRACE("ZoneManager::Shutdown");
    // Keep this object alive while the message server drops its reference
    // (unless this already is the final Release).
    if (!destroying_) Primary()->AddRef();
    {
        Section sec(this);
        fc4_zones_shutdown(core_);  // §1.1 Shutdown
        LogState("Shutdown");
    }
    cFC4ZoneManager* self = this;
    g_live.compare_exchange_strong(self, nullptr);
    if (!destroying_) Primary()->Release();
    return true;  // AL = 1
}

// ---- cISC4ZoneManager (§11.1) -----------------------------------------------------------------------------

cISC4SimGrid<int8_t>* cFC4ZoneManager::GetZoneGrid() {
    Section sec(this);
    if (!sec.ok()) return snap_.grid.load();
    return reinterpret_cast<cISC4SimGrid<int8_t>*>(FromToken(fc4_zones_grid(core_)));
}

uint32_t cFC4ZoneManager::GetZoneCount(int32_t type) {
    Section sec(this);
    if (!sec.ok()) return static_cast<uint32_t>(type) < 16 ? snap_.zoneCount[type].load() : 0;
    return fc4_zones_zone_count(core_, type);
}

uint32_t cFC4ZoneManager::GetUndevelopedTileCountOfType(int32_t type) {
    Section sec(this);
    if (!sec.ok()) return static_cast<uint32_t>(type) < 16 ? snap_.undeveloped[type].load() : 0;
    return fc4_zones_undeveloped_tile_count(core_, type);
}

uint32_t cFC4ZoneManager::GetUndevelopedTileCount() {
    Section sec(this);
    if (!sec.ok()) return snap_.undevelopedTotal.load();
    return fc4_zones_undeveloped_total(core_);
}

// Slot 9 inserts a zero entry into map 1 (§11.1 quirk): it needs the lock.
uint32_t cFC4ZoneManager::GetDevelopmentFailureCount(int32_t zone, int32_t result) {
    Section sec(this);
    if (!sec.ok()) return 0;
    return fc4_zones_development_failure_count(core_, zone, result);
}

uint32_t cFC4ZoneManager::GetAbandonedTileCount() {
    Section sec(this);
    if (!sec.ok()) return snap_.abandoned.load();
    return fc4_zones_abandoned_tile_count(core_);
}

// Slot 11 (§6): the caller's region is read into the core's form; the cells
// the core rejected are cleared in it afterwards (the game only clears bits).
// outLots receives the lots to demolish at the pricing call.
bool cFC4ZoneManager::PlaceZone(SC4CellRegion<long>& region, int32_t type, bool commit, bool skipFunds,
                                bool checkSlope, bool skipSizeCheck, bool checkLotInside, int64_t* outCost,
                                int32_t* outError, void* outLots) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) {
        // [choice] Another thread that cannot get the lock: refused, error 0.
        if (outCost != nullptr) *outCost = 0;
        if (outError != nullptr) *outError = 0;
        return false;
    }
    std::vector<uint8_t> mask;
    fc4_zone_region r{};
    // [choice] (ZG2, recommended; the original dereferences a bad bitmap and
    // reads and writes the grid unchecked) A region the core cannot take
    // (wrong layout, above 1,048,576 cells or 4096 a side, a set cell outside
    // the city) fails with error 0 and touches nothing.
    const bool have = fc4::ReadZoneRegion(&region, mask, &r, "zones");
    OutLotsSink sink{&ctx_, outLots};
    fc4_zones_place_result res{};
    res.struct_size = sizeof(res);
    const bool ok = fc4_zones_place_zone(core_, have ? &r : nullptr, type, commit, skipFunds, checkSlope,
                                         skipSizeCheck, checkLotInside, outLots ? &OutLotsInsert : nullptr, &sink,
                                         &res);
    const uint32_t cleared = have ? fc4::ClearRejectedBits(&region, r) : 0;
    if (outCost != nullptr) *outCost = res.cost;
    if (outError != nullptr) *outError = res.error;
    if (commit) {
        fc4::Log("zones PlaceZone: type=%d rect=%d,%d..%d,%d cells=%u ok=%u error=%d cost=%lld accepted=%u "
                 "cleared=%u lots=%u%s",
                 type, r.x0, r.z0, r.x1, r.z1, static_cast<unsigned>(mask.size()), ok ? 1u : 0u, res.error,
                 static_cast<long long>(res.cost), res.accepted, cleared, res.lots,
                 have ? "" : " (region refused)");
    } else {
        FC4_LOGV("zones PlaceZone (dry run): type=%d rect=%d,%d..%d,%d ok=%u error=%d cost=%lld cleared=%u lots=%u",
                 type, r.x0, r.z0, r.x1, r.z1, ok ? 1u : 0u, res.error, static_cast<long long>(res.cost), cleared,
                 res.lots);
    }
    return ok;
}

// Slot 12: the int8 zone type, sign-extended. Another thread that cannot get
// the lock reads the game grid itself ([choice] 0 outside it).
int32_t cFC4ZoneManager::GetZoneType(int32_t x, int32_t z) {
    Section sec(this);
    if (sec.ok()) return fc4_zones_zone_type(core_, x, z);
    cISC4SimGrid<int8_t>* g = snap_.grid.load();
    if (g == nullptr || x < 0 || z < 0 || x >= snap_.gridX.load() || z >= snap_.gridZ.load()) return 0;
    return static_cast<int32_t>(g->GetTractValue(x, z));
}

bool cFC4ZoneManager::IsRCIZone(int32_t type) { return fc4_zones_is_rci_zone(type); }

bool cFC4ZoneManager::GetTextureForZone(int32_t type, cGZPersistResourceKey& key) {
    Section sec(this);
    uint32_t k[3] = {};
    if (sec.ok()) {
        if (!fc4_zones_texture_for_zone(core_, type, k)) return false;
    } else {
        if (static_cast<uint32_t>(type) >= 16) return false;
        // §11.1 slot 14: {0x7AB50E44, 0x1ABE787D, texture[type]}.
        k[0] = 0x7AB50E44;
        k[1] = 0x1ABE787D;
        k[2] = snap_.texture[type].load();
    }
    key.type = k[0];
    key.group = k[1];
    key.instance = k[2];
    return true;
}

uint32_t cFC4ZoneManager::GetZoneDragColor(int32_t type) {
    Section sec(this);
    if (!sec.ok()) return static_cast<uint32_t>(type) < 16 ? snap_.dragColor[type].load() : 0;
    return fc4_zones_drag_color(core_, type);
}

uint8_t cFC4ZoneManager::GetZoneDisplayAlpha() {
    Section sec(this);
    if (!sec.ok()) return static_cast<uint8_t>(snap_.alpha.load());
    return fc4_zones_display_alpha(core_);
}

uint8_t cFC4ZoneManager::GetDefaultZoneDisplayAlpha() {
    Section sec(this);
    if (!sec.ok()) return static_cast<uint8_t>(snap_.defaultAlpha.load());
    return fc4_zones_default_display_alpha(core_);
}

// Slot 18 (void): a changed alpha retextures every lot in state 1 (§11.1).
void cFC4ZoneManager::SetZoneDisplayAlpha(uint8_t alpha) {
    Section sec(this);
    if (!sec.ok()) return;  // [choice] another thread that cannot get the lock: not changed
    fc4_zones_set_display_alpha(core_, alpha);
}

int32_t cFC4ZoneManager::GetMinZoneSize(int32_t type) {
    Section sec(this);
    if (!sec.ok()) return static_cast<uint32_t>(type) < 16 ? snap_.minSize[type].load() : 0;
    return fc4_zones_min_size(core_, type);
}

int32_t cFC4ZoneManager::GetMaxZoneSize(int32_t type) {
    Section sec(this);
    if (!sec.ok()) return static_cast<uint32_t>(type) < 16 ? snap_.maxSize[type].load() : 0;
    return fc4_zones_max_size(core_, type);
}

// ---- messages (§5.1) --------------------------------------------------------------------------------------

bool cFC4ZoneManager::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("zones DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    Section sec(this);
    if (!sec.ok()) return true;  // [choice] a message from another thread that cannot get the lock is dropped
    const uint32_t type = pMessage->GetType();
    uint64_t occupant = 0;
    uint32_t d3 = 0, d4 = 0;
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    if (s != nullptr) {
        occupant = TokenOf(s->GetVoid1());
        d3 = static_cast<uint32_t>(s->GetData3());
        d4 = static_cast<uint32_t>(s->GetData4());
    }
    fc4_zones_dezone out{};
    out.struct_size = sizeof(out);
    fc4_zones_do_message(core_, type, occupant, d3, d4, &out);
    Drop(s);
    if (type == kMsgInsertOccupant && out.dezoned) {
        fc4::Log("zones dezone: occupant %p rect %d,%d..%d,%d changed=%u lots=%u", static_cast<void*>(FromToken(occupant)),
                 out.rect.x1, out.rect.z1, out.rect.x2, out.rect.z2, out.changed, out.lots);
    }
    return true;  // §5: always
}

// ---- persistence (§10.1) ----------------------------------------------------------------------------------

bool cFC4ZoneManager::Write(cIGZOStream& s) {
    FC4_TRACE("ZoneManager::Write");
    Section sec(this);
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_zones_write(core_, a.Get());
    fc4::Log("zones Write -> %d", ok ? 1 : 0);
    LogState("Write");
    return ok;
}

// Runs before Init: the core sizes nothing from the city here.
bool cFC4ZoneManager::Read(cIGZIStream& s) {
    FC4_TRACE("ZoneManager::Read");
    Section sec(this);
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_zones_read(core_, a.Get());
    fc4::Log("zones Read -> %d", ok ? 1 : 0);
    LogState("Read");
    return ok;
}

uint32_t cFC4ZoneManager::GetGZCLSID() { return kFC4_CLSID_cSC4ZoneManager; }

// ---- test seam op 38 ----------------------------------------------------------------------------------------

intptr_t ZonesTestSeam(const FC4ZonesTestInput* in) {
    if (in == nullptr) return -1;
    if (in->op == kFC4ZonesTestHooks) {
        if (in->zoneManager == nullptr) {
            // The outLots allocator only (null: back to the game's).
            g_testSetAllocate = in->setAllocate;
            g_testSetFree = in->setFree;
            return 0;
        }
        cFC4ZoneManager* z = cFC4ZoneManager::FromPrimary(in->zoneManager);
        if (z == nullptr) return -1;
        g_testSetAllocate = in->setAllocate;
        g_testSetFree = in->setFree;
        z->SetTestHooks(in->host, in->postSink, in->postSinkContext);
        return 1;
    }
    if (in->op == kFC4ZonesTestShadow) return fc4::ZonesShadowTestRun(in->city, in->julian, in->event, in->host);
    if (in->op == kFC4ZonesTestLiveRegions) return static_cast<intptr_t>(cFC4ZoningChangeMessage::LiveRegions());
    return -1;
}
