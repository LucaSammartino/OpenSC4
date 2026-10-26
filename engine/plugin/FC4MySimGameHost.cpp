// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

// The MySims' read-only city services (FC4MySimGameHost.h).

#include "FC4MySimGameHost.h"

#include "FC4RefAudit.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "SC4List.h"
#include "SC4Percentage.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZString.h"
#include "cISC4App.h"
#include "cISC4AuraSimulator.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4CrimeSimulator.h"
#include "cISC4DisasterInstance.h"
#include "cISC4DisasterLayer.h"
#include "cISC4FireProtectionSimulator.h"
#include "cISC4GrowthDeveloper.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4PlumbingSimulator.h"
#include "cISC4PoliceSimulator.h"
#include "cISC4PollutionSimulator.h"
#include "cISC4PowerSimulator.h"
#include "cISC4ResidentialSimulator.h"
#include "cISC4SimGrid.h"
#include "cISC4TrafficSimulator.h"
#include "cS3DVector3.h"

#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kIIDBuildingOccupant = 0x87DDA3A9;
constexpr uint32_t kOccupantTypeBuilding = 0x278128A0;

cISC4City* AppCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    // Our QueryInterface added the reference we drop: record it so the audit pairs the release.
    FC4_ACQUIRED(reinterpret_cast<cIGZUnknown*>(sc4), "QueryInterface");
    cISC4City* city = sc4->GetCity();
    FC4_RELEASE(sc4);
    return city;
}

// QI with one reference for the caller; Drop releases it.
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

inline uint64_t TokenOf(const void* p) {
    return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(p));
}

// ---- mysim host callbacks (fc4_mysim_host, read-only to the game) ---------------------------------

void MCellCounts(void*, uint32_t* x, uint32_t* z) {
    cISC4City* city = AppCity();
    if (x) *x = city ? city->CellCountX() : 0;
    if (z) *z = city ? city->CellCountZ() : 0;
}
void MCellCenter(void*, int32_t cx, int32_t cz, float* x, float* z) {
    cISC4City* city = AppCity();
    if (x == nullptr || z == nullptr) return;
    if (city == nullptr) { *x = 0; *z = 0; return; }
    float px = 0, pz = 0;
    city->CellCenterToPosition(cx, cz, px, pz);  // slot 97
    *x = px; *z = pz;
}
bool MDisasterOngoing(void*) {
    cISC4City* city = AppCity();
    cISC4DisasterLayer* l = city ? city->GetDisasterLayer() : nullptr;
    return l ? l->IsADisasterOngoing() : false;  // slot 10
}
size_t MDisasterLocations(void*, fc4_mysim_disaster_location* out, size_t cap) {
    cISC4City* city = AppCity();
    cISC4DisasterLayer* l = city ? city->GetDisasterLayer() : nullptr;
    if (l == nullptr) return 0;
    const int32_t n = l->GetDisasterLocationCount();  // slot 16
    size_t w = 0;
    for (int32_t i = 0; i < n; ++i) {
        cISC4DisasterInstance* inst = nullptr;
        int x = 0, y = 0;
        // [abi-ret] slot 17 (interfaces.tsv eax-or-void): only whether a
        // location was returned; the shadow counts what came back.
        if (!l->GetDisasterLocationByIndex(i, &inst, x, y)) continue;
        if (out != nullptr && w < cap) {
            out[w].disaster_type = inst ? inst->GetID() : 0;  // GetID = the manager type (disasters.md §1)
            out[w].x = x;
            out[w].z = y;
        }
        ++w;
    }
    return w;
}

bool MBuilding(void*, uint64_t t, fc4_mysim_building* out) {
    cISC4Occupant* occ = reinterpret_cast<cISC4Occupant*>(static_cast<uintptr_t>(t));
    if (occ == nullptr || out == nullptr) return false;
    cISC4BuildingOccupant* b = Query<cISC4BuildingOccupant>(occ, kIIDBuildingOccupant);
    if (b == nullptr) return false;
    out->building_type = b->GetBuildingType();       // slot 4
    out->purpose = static_cast<uint32_t>(b->GetBuildingProfile().purpose);  // slot 11
    const SC4Percentage* p = b->GetCompletionPercent();  // slot 9 (pointer)
    out->completion = p ? p->percentage : 0.0f;
    cS3DVector3 pos{};
    // [abi-ret] occupant slot 8 (interfaces.tsv eax-or-void): only the
    // presence bit is read; the coordinates come from the out struct.
    if (MySimBuildingPosition(b, &pos)) { out->position[0] = pos.fX; out->position[1] = pos.fY; out->position[2] = pos.fZ; }
    else { out->position[0] = out->position[1] = out->position[2] = 0.0f; }
    if (cIGZString* name = b->GetBuildingName()) {
        out->name = reinterpret_cast<const uint8_t*>(name->ToChar());
        out->name_len = name->Strlen();
    } else {
        out->name = nullptr; out->name_len = 0;
    }
    out->groups = nullptr; out->group_count = 0;  // groups not readable read-only (not compared)
    Drop(b);
    return true;
}

bool MLotOf(cISC4Lot* lot, uint64_t buildingToken, fc4_mysim_lot* out) {
    if (lot == nullptr || out == nullptr) return false;
    out->building = buildingToken;
    int lx = 0, lz = 0;
    if (lot->GetLocation(lx, lz)) { out->cell_x = lx; out->cell_z = lz; }  // slot 21
    else { out->cell_x = out->cell_z = 0; }
    SC4Rect<int32_t> r{};
    if (lot->GetBoundingRect(r)) {  // slot 14 of the lot geometry
        out->x0 = r.topLeftX; out->z0 = r.topLeftY;
        out->x1 = r.bottomRightX; out->z1 = r.bottomRightY;
    } else {
        out->x0 = out->x1 = out->cell_x; out->z0 = out->z1 = out->cell_z;
    }
    out->condition = static_cast<uint8_t>(lot->GetCondition());        // slot 35
    out->road_access = lot->HasRoadAccess() ? 1u : 0u;                 // slot 28
    out->wealth = static_cast<uint8_t>(lot->GetOccupantWealth());      // slot 41
    return true;
}

bool MLot(void*, uint64_t t, fc4_mysim_lot* out) {
    cISC4City* city = AppCity();
    cISC4LotManager* lm = city ? city->GetLotManager() : nullptr;  // slot 33
    if (lm == nullptr || t == 0 || out == nullptr) return false;
    auto* occ = Query<cISC4Occupant>(reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(t)), GZIID_cISC4Occupant);
    if (occ == nullptr) return false;
    cISC4Lot* lot = lm->GetOccupantLot(occ);  // slot 14, borrowed
    Drop(occ);
    if (lot == nullptr) return false;
    return MLotOf(lot, t, out);
}

bool MOccupantPosition(void*, uint64_t t, float* out) {
    if (t == 0 || out == nullptr) return false;
    auto* occ = Query<cISC4Occupant>(reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(t)), GZIID_cISC4Occupant);
    if (occ == nullptr) return false;
    cS3DVector3 p{};
    // [abi-ret] occupant slot 8 (interfaces.tsv eax-or-void): only the
    // presence bit feeds the callback's Option.
    const bool positioned = occ->GetPosition(&p);
    Drop(occ);
    if (!positioned) return false;
    out[0] = p.fX; out[1] = p.fY; out[2] = p.fZ;
    return true;
}

// Occupant manager slot 40 GetOccupantsByBBox (mysims.md §4.9; corners in
// world metres via CellCornerToPosition). Nodes from the game allocator go
// back through ReleaseOccupantList.
int64_t MBuildingsInBox(void*, int32_t x0, int32_t z0, int32_t x1, int32_t z1, uint64_t* out, size_t cap) {
    cISC4City* city = AppCity();
    cISC4OccupantManager* om = city ? city->GetOccupantManager() : nullptr;  // slot 34
    if (om == nullptr) return -1;
    SC4List<cISC4Occupant*> list;
    float ax = 0, az = 0, bx = 0, bz = 0;
    city->CellCornerToPosition(x0, z0, ax, az);
    city->CellCornerToPosition(x1 + 1, z1 + 1, bx, bz);
    const float fx[2] = {ax, bx}, fz[2] = {az, bz};
    om->GetOccupantsByBBox(list, fx, fz, kOccupantTypeBuilding, UINT32_MAX);  // maxCount, not flags
    int64_t n = 0;
    for (cISC4Occupant* o : list) {
        if (out != nullptr && static_cast<size_t>(n) < cap) out[n] = TokenOf(o);
        ++n;
    }
    om->ReleaseOccupantList(list);  // slot 48: the references go back
    return n;
}

int64_t MAllBuildings(void*, uint64_t* out, size_t cap) {
    cISC4City* city = AppCity();
    cISC4OccupantManager* om = city ? city->GetOccupantManager() : nullptr;
    if (om == nullptr) return -1;
    SC4List<cISC4Occupant*> list;
    om->GetOccupantsByBBox(list, nullptr, nullptr, kOccupantTypeBuilding, UINT32_MAX);
    int64_t n = 0;
    for (cISC4Occupant* o : list) {
        if (out != nullptr && static_cast<size_t>(n) < cap) out[n] = TokenOf(o);
        ++n;
    }
    om->ReleaseOccupantList(list);
    return n;
}

size_t MLotsInRect(void*, int32_t x0, int32_t z0, int32_t x1, int32_t z1, fc4_mysim_lot* out, size_t cap) {
    cISC4City* city = AppCity();
    cISC4LotManager* lm = city ? city->GetLotManager() : nullptr;
    if (lm == nullptr) return 0;
    GameList<cISC4Lot*> list;  // docs/abi/allocators.md: B nodes, one owned reference per lot.
    if (!list.valid()) return 0;
    AsGame(lm)->GetLotsInCellRect(list, x0, z0, x1, z1, false);
    size_t n = 0;
    for (cISC4Lot*& lot : list) {
        if (!lot) continue;
        FC4_ACQUIRED(lot, "GetLotsInCellRect");
        if (out != nullptr && n < cap) {
            uint64_t bt = 0;
            if (cISC4BuildingOccupant* b = lot->GetBuilding()) {  // slot 48, borrowed
                bt = TokenOf(b);
            }
            MLotOf(lot, bt, &out[n]);
        }
        ++n;
        FC4_RELEASE(lot);
        lot = nullptr;
    }
    return n;
}

size_t MSpecialBuildings(void*, uint64_t* out, size_t cap) {
    cISC4City* city = AppCity();
    cISC4BuildingDevelopmentSimulator* bds = city ? city->GetBuildingDevelopmentSimulator() : nullptr;  // slot 53
    if (bds == nullptr) return 0;
    const intptr_t raw = bds->GetSpecialBuildings();  // slot 44
    if (raw == 0) return 0;
    const auto* types = reinterpret_cast<const std::vector<uint32_t>*>(raw);
    size_t n = 0;
    for (uint32_t v : *types) {
        if (out != nullptr && n < cap) out[n] = v;  // building TYPE ids (not tokens)
        ++n;
    }
    return n;
}

size_t MEducationBuildings(void*, bool hospitals, uint64_t* out, size_t cap) {
    cISC4City* city = AppCity();
    cISC4ResidentialSimulator* res = city ? city->GetResidentialSimulator() : nullptr;
    if (res == nullptr) return 0;
    SC4List<cISC4Occupant*> list;
    if (hospitals) res->GetHospitalBuildings(list, nullptr);  // slot 49
    else res->GetSchoolBuildings(list, nullptr);              // slot 47
    size_t n = 0;
    for (cISC4Occupant* o : list) {
        if (out != nullptr && n < cap) out[n] = TokenOf(o);
        ++n;
    }
    return n;
}

bool MGrade(void*, uint64_t, bool, float*) {
    return false;  // query-data record layout not built read-only; grade not compared
}
void MJobTitle(void*, uint64_t t, fc4_bytes_sink sink, void* ctx) {
    cISC4Occupant* occ = reinterpret_cast<cISC4Occupant*>(static_cast<uintptr_t>(t));
    if (occ == nullptr || sink == nullptr) return;
    cISC4BuildingOccupant* b = Query<cISC4BuildingOccupant>(occ, kIIDBuildingOccupant);
    if (b == nullptr) return;
    if (cIGZString* n = b->GetBuildingName()) {
        if (const char* s = n->ToChar()) sink(ctx, reinterpret_cast<const uint8_t*>(s), n->Strlen());
    }
    Drop(b);
}
bool MLandmarkFunding(void*, uint64_t, float*) { return false; }  // trip fields not compared

bool MAverageAge(void*, int32_t x, int32_t z, int32_t* out) {
    cISC4City* city = AppCity();
    cISC4ResidentialSimulator* res = city ? city->GetResidentialSimulator() : nullptr;
    if (res == nullptr || out == nullptr) return false;
    const int32_t map = res->GetAverageAgeMap();  // residential.md slot 41: uint8 grid (SDK int32 error).
    if (map == 0) return false;
    *out = reinterpret_cast<cISC4SimGrid<uint8_t>*>(map)->GetCellValue(x, z);
    return true;
}
bool MEqByWealth(void*, int32_t x, int32_t z, uint8_t w, float* out) {
    cISC4City* city = AppCity();
    cISC4ResidentialSimulator* res = city ? city->GetResidentialSimulator() : nullptr;
    if (res == nullptr || out == nullptr) return false;
    *out = res->GetCellEQByWealth(x, z, static_cast<cISC4BuildingOccupant::WealthType>(w));  // slot 37
    return true;
}
bool MHqByWealth(void*, int32_t x, int32_t z, uint8_t w, float* out) {
    cISC4City* city = AppCity();
    cISC4ResidentialSimulator* res = city ? city->GetResidentialSimulator() : nullptr;
    if (res == nullptr || out == nullptr) return false;
    *out = res->GetCellHQByWealth(x, z, static_cast<cISC4BuildingOccupant::WealthType>(w));  // slot 38
    return true;
}
bool MLifeExpectancy(void*, int32_t x, int32_t z, uint8_t* out) {
    cISC4City* city = AppCity();
    cISC4ResidentialSimulator* res = city ? city->GetResidentialSimulator() : nullptr;
    if (res == nullptr || out == nullptr) return false;
    *out = static_cast<uint8_t>(res->GetCellLifeExpectancy(x, z));  // slot 29
    return true;
}
bool MNewAgeByWealth(void*, uint8_t w, uint8_t* out) {
    cISC4City* city = AppCity();
    cISC4ResidentialSimulator* res = city ? city->GetResidentialSimulator() : nullptr;
    if (res == nullptr || out == nullptr) return false;
    *out = static_cast<uint8_t>(res->GetAverageNewAgeByWealth(static_cast<cISC4BuildingOccupant::WealthType>(w)));
    return true;
}

bool MGrid(void*, uint32_t grid, uint32_t arg, int32_t x, int32_t z, int32_t* out) {
    cISC4City* city = AppCity();
    if (city == nullptr || out == nullptr) return false;
    switch (grid) {
        case FC4_MYSIM_GRID_CRIME: {
            cISC4CrimeSimulator* s = city->GetCrimeSimulator();
            if (s == nullptr) return false;
            *out = s->GetCrimeGrid()->GetCellValue(x, z);
            return true;
        }
        case FC4_MYSIM_GRID_POLLUTION: {
            cISC4PollutionSimulator* s = city->GetPollutionSimulator();
            if (s == nullptr) return false;
            *out = s->GetPollutionGrid(arg)->GetCellValue(x, z);
            return true;
        }
        case FC4_MYSIM_GRID_AURA: {
            cISC4AuraSimulator* s = city->GetAuraSimulator();
            if (s == nullptr) return false;
            *out = s->GetAuraGrid()->GetCellValue(x, z);
            return true;
        }
        case FC4_MYSIM_GRID_DESIRABILITY: {
            cISC4BuildingDevelopmentSimulator* bds = city->GetBuildingDevelopmentSimulator();
            if (bds == nullptr) return false;
            cISC4GrowthDeveloper* d =
                bds->GetDeveloper(static_cast<cISC4BuildingDevelopmentSimulator::DeveloperType>(arg));
            if (d == nullptr) return false;
            *out = d->GetDesirabilityMap()->GetCellValue(x, z);
            return true;
        }
        default: return false;
    }
}

bool MGridAt(void*, uint32_t grid, uint32_t arg, float x, float z, int32_t* out) {
    cISC4City* city = AppCity();
    if (city == nullptr || out == nullptr) return false;
    if (grid == FC4_MYSIM_GRID_CONGESTION) {
        cISC4TrafficSimulator* s = city->GetTrafficSimulator();
        if (s == nullptr) return false;
        cISC4SimGrid<uint8_t>* g = s->GetCongestionMap();  // slot 10
        if (g == nullptr) return false;
        int32_t tx = 0, tz = 0;
        if (!g->PositionToTract(x, z, tx, tz)) return false;
        *out = g->GetTractValue(tx, tz);
        return true;
    }
    if (grid == FC4_MYSIM_GRID_AVERAGE_EQ || grid == FC4_MYSIM_GRID_AVERAGE_HQ) {
        cISC4ResidentialSimulator* res = city->GetResidentialSimulator();
        if (res == nullptr) return false;
        cISC4SimGrid<float>* g = nullptr;
        float mn = 0, mx = 0;
        // [abi-ret] residential slots 21/22 (interfaces.tsv eax-or-void):
        // only success feeds the Option; the grid travels via the out param.
        const bool ok = grid == FC4_MYSIM_GRID_AVERAGE_EQ ? res->GetAverageEQGrid(g, &mn, &mx)
                                                          : res->GetAverageHQGrid(g, &mn, &mx);
        if (!ok || g == nullptr) return false;
        int32_t tx = 0, tz = 0;
        if (!g->PositionToTract(x, z, tx, tz)) return false;
        *out = static_cast<int32_t>(g->GetTractValue(tx, tz));
        return true;
    }
    return MGrid(nullptr, grid, arg, static_cast<int32_t>(x), static_cast<int32_t>(z), out);
}

bool MCoverage(void*, bool police, int32_t x0, int32_t z0, int32_t w, int32_t h, int32_t* out) {
    cISC4City* city = AppCity();
    if (city == nullptr || out == nullptr) return false;
    if (police) {
        cISC4PoliceSimulator* s = city->GetPoliceSimulator();
        if (s == nullptr) return false;
        *out = s->GetPolicePowerGrid()->GetAverageValueInCellRect(x0, z0, x0 + w - 1, z0 + h - 1);
        return true;
    }
    cISC4FireProtectionSimulator* s = city->GetFireProtectionSimulator();
    if (s == nullptr) return false;
    *out = s->GetFireProtectionGrid()->GetAverageValueInCellRect(x0, z0, x0 + w - 1, z0 + h - 1);
    return true;
}

int32_t MPowered(void*, uint64_t t) {
    cISC4City* city = AppCity();
    cISC4PowerSimulator* s = city ? city->GetPowerSimulator() : nullptr;
    if (s == nullptr) return -1;
    auto* occ = Query<cISC4Occupant>(reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(t)), GZIID_cISC4Occupant);
    if (occ == nullptr) return -1;
    const bool powered = s->IsOccupantPowered(occ);  // slot 31
    Drop(occ);
    return powered ? 1 : 0;
}
int32_t MWatered(void*, uint64_t t) {
    cISC4City* city = AppCity();
    cISC4PlumbingSimulator* s = city ? city->GetPlumbingSimulator() : nullptr;
    if (s == nullptr) return -1;
    auto* occ = Query<cISC4Occupant>(reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(t)), GZIID_cISC4Occupant);
    if (occ == nullptr) return -1;
    // [abi-ret] plumbing slot 9 (interfaces.tsv via-call): read as bool.
    const bool watered = s->IsBuildingWatered(occ);
    Drop(occ);
    return watered ? 1 : 0;
}

// Police slot 21 (SC4List) / fire slot 22 (ilist): the same node layout; the
// SDK SC4List allocates its nodes with the framework allocator A and its
// destructor releases each element's reference — exactly what the getters
// hand over.
int64_t MStations(void*, bool police, fc4_mysim_station* out, size_t cap) {
    cISC4City* city = AppCity();
    if (city == nullptr) return -1;
    SC4List<cISC4Occupant*> list;
    int64_t n = 0;
    if (police) {
        cISC4PoliceSimulator* s = city->GetPoliceSimulator();
        if (s == nullptr) return -1;
        s->GetPoliceStations(list);  // slot 21
        for (cISC4Occupant* st : list) {
            SC4Percentage pct{0.0f};
            int32_t cost = 0;
            // [abi-ret] police slot 7 (interfaces.tsv eax-or-void): the
            // funding value comes from the out param; the bool only gates.
            if (s->GetStationFunding(st, cost, pct)) {
                if (out != nullptr && static_cast<size_t>(n) < cap) {
                    out[n].station = TokenOf(st);
                    out[n].funding = pct.percentage;
                }
                ++n;
            }
        }
    } else {
        cISC4FireProtectionSimulator* s = city->GetFireProtectionSimulator();
        if (s == nullptr) return -1;
        ilist<cISC4Occupant*> flist;  // slot 22: ilist = std::list with allocator A
        s->GetFireStations(flist);
        for (cISC4Occupant* st : flist) {
            SC4Percentage pct{0.0f};
            int32_t cost = 0;
            // [abi-ret] fire slot 8 (interfaces.tsv eax-or-void): the
            // funding value comes from the out param; the bool only gates.
            if (s->GetStationFunding(st, cost, pct)) {
                if (out != nullptr && static_cast<size_t>(n) < cap) {
                    out[n].station = TokenOf(st);
                    out[n].funding = pct.percentage;
                }
                ++n;
            }
        }
    }
    return n;
}

bool MTripSimulators(void*) {
    cISC4City* city = AppCity();
    return city != nullptr && city->GetTrafficSimulator() != nullptr &&
           city->GetPollutionSimulator() != nullptr && city->GetCrimeSimulator() != nullptr;
}

}  // namespace

void FillMySimGameServices(fc4_mysim_host* h) {
    if (h == nullptr) return;
    h->cell_counts = &MCellCounts;
    h->cell_center = &MCellCenter;
    h->disaster_ongoing = &MDisasterOngoing;
    h->disaster_locations = &MDisasterLocations;
    h->building = &MBuilding;
    h->lot = &MLot;
    h->occupant_position = &MOccupantPosition;
    h->buildings_in_box = &MBuildingsInBox;
    h->all_buildings = &MAllBuildings;
    h->lots_in_rect = &MLotsInRect;
    h->special_buildings = &MSpecialBuildings;
    h->education_buildings = &MEducationBuildings;
    h->grade = &MGrade;
    h->job_title = &MJobTitle;
    h->landmark_funding = &MLandmarkFunding;
    h->average_age = &MAverageAge;
    h->eq_by_wealth = &MEqByWealth;
    h->hq_by_wealth = &MHqByWealth;
    h->life_expectancy = &MLifeExpectancy;
    h->new_age_by_wealth = &MNewAgeByWealth;
    h->grid = &MGrid;
    h->grid_at = &MGridAt;
    h->coverage = &MCoverage;
    h->powered = &MPowered;
    h->watered = &MWatered;
    h->stations = &MStations;
    h->trip_simulators = &MTripSimulators;
}

}  // namespace fc4
