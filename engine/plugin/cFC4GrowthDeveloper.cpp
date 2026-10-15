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

#include "cFC4GrowthDeveloper.h"

#include "FC4Checked.h"
#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4CrimeInterfaces.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4GameMapBuild.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Seed.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Development.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "SC4Rect.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4AuraSimulator.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4City.h"
#include "cISC4CommercialSimulator.h"
#include "cISC4CrimeSimulator.h"
#include "cISC4Demand.h"
#include "cISC4DemandSimulator.h"
#include "cISC4GrowthDeveloper.h"
#include "cISC4LandValueSimulator.h"
#include "cISC4Lot.h"
#include "cISC4NeighborsSimulator.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4PollutionSimulator.h"
#include "cISC4RegionalCity.h"
#include "cISC4ResidentialSimulator.h"
#include "cISC4SimGrid.h"
#include "cISC4Simulator.h"
#include "cISC4TractDeveloper.h"
#include "cISC4TrafficSimulator.h"
#include "cISC4ZoneManager.h"
#include "cISCPropertyHolder.h"
#include "cRZBaseString.h"
#include "cS3DVector3.h"

#include <algorithm>
#include <cstdio>
#include <cstring>
#include <string>
#include <utility>
#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

// §1.2 step 8, §3.1.
constexpr uint32_t kMsgStateChanged = 0xE8BDA2E7;
constexpr uint32_t kMsgZoningChange = 0x07699455;

// Services (fc4.h: fc4_growth_host acquire_service).
enum : uint32_t { kCity = 0, kBds = 1, kOccupants = 2, kZones = 3, kLandValue = 4, kTraffic = 5, kServiceCount = 6 };

fc4::GrowthContext* G(void* u) { return static_cast<fc4::GrowthContext*>(u); }

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

// A reference on one of our objects or the game's (audited).
void AddRefAny(cIGZUnknown* o) {
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->AddRef();
    } else {
        FC4_ADDREF(o);
    }
}

void ReleaseAny(cIGZUnknown* o) {
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

std::string DateText(uint32_t julian) {
    fc4_date d{};
    fc4_date_from_julian(static_cast<int32_t>(julian), &d);
    char buf[32];
    std::snprintf(buf, sizeof(buf), "%04d-%02d-%02d", static_cast<int>(d.year), static_cast<int>(d.month),
                  static_cast<int>(d.day_of_month));
    return buf;
}

// The development_* switches gate the developer's game calls; the shadow's
// context reads the game as it is.
bool Gate(void* u, fc4::Sw s, const char* what) {
    if (G(u)->readOnly || fc4::On(s)) return true;
    fc4::NoteSkipped(s, what);
    return false;
}

cISC4City* CityOf(void* u) { return static_cast<cISC4City*>(G(u)->services[kCity]); }

cFC4ISC4BuildingDevelopmentSimulator* Bds(void* u) {
    cIGZUnknown* b = G(u)->services[kBds];
    return b ? fc4::AsGame(static_cast<cISC4BuildingDevelopmentSimulator*>(b)) : nullptr;
}

cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cFC4ISC4Lot* LotOf(uint64_t t) { return t ? fc4::AsGame(reinterpret_cast<cISC4Lot*>(FromToken(t))) : nullptr; }
cFC4ISC4Demand* NodeOf(uint64_t t) {
    return t ? fc4::AsGame(reinterpret_cast<cISC4Demand*>(FromToken(t))) : nullptr;
}

// ---- services (§1.2 step 2, §1.3) -----------------------------------------------------------------

// The object of service `s` (borrowed): the city (the app's), else one of
// the held city's simulators.
cIGZUnknown* ServiceObject(void* u, uint32_t s) {
    if (s == kCity) return AppCity();
    cISC4City* city = CityOf(u);
    if (city == nullptr) return nullptr;
    switch (s) {
        case kBds: return city->GetBuildingDevelopmentSimulator();
        case kOccupants: return city->GetOccupantManager();  // slot 34
        case kZones: return city->GetZoneManager();
        case kLandValue: return city->GetLandValueSimulator();
        case kTraffic: return city->GetTrafficSimulator();
        default: return nullptr;
    }
}

// §1.2 step 2: a reference on a service still null (false when it is null).
bool AcquireService(void* u, uint32_t s) {
    if (s >= kServiceCount) return false;
    fc4::GrowthContext* c = G(u);
    if (c->services[s] != nullptr) return true;
    cIGZUnknown* p = ServiceObject(u, s);
    if (p == nullptr) return false;
    AddRefAny(p);
    c->services[s] = p;
    return true;
}

void ReleaseService(void* u, uint32_t s) {
    if (s >= kServiceCount) return;
    fc4::GrowthContext* c = G(u);
    ReleaseAny(c->services[s]);
    c->services[s] = nullptr;
}

// §1.2 step 8.
bool AcquireMessageServer(void* u) {
    fc4::GrowthContext* c = G(u);
    if (c->msgServer != nullptr) return true;
    cIGZMessageServer2Ptr ms;
    if (!ms) return false;
    c->msgServer = ms;
    FC4_ADDREF(c->msgServer);
    return true;
}

void ReleaseMessageServer(void* u) {
    fc4::GrowthContext* c = G(u);
    if (c->msgServer != nullptr) FC4_RELEASE(c->msgServer);
    c->msgServer = nullptr;
}

void Subscribe(void* u, const uint32_t* types, size_t n) {
    fc4::GrowthContext* c = G(u);
    if (c->readOnly || c->msgServer == nullptr || c->target == nullptr || types == nullptr) return;
    for (size_t i = 0; i < n; ++i) c->msgServer->AddNotification(c->target, types[i]);
}

void Unsubscribe(void* u, const uint32_t* types, size_t n) {
    fc4::GrowthContext* c = G(u);
    if (c->readOnly || c->msgServer == nullptr || c->target == nullptr || types == nullptr) return;
    for (size_t i = 0; i < n; ++i) c->msgServer->RemoveNotification(c->target, types[i]);
}

// City slot 50 (borrowed).
cISC4Simulator* SimulatorOf(void* u) {
    cISC4City* city = CityOf(u);
    return city ? city->GetSimulator() : nullptr;
}

bool SimulatorPresent(void* u) { return SimulatorOf(u) != nullptr; }

// §1.2 step 9: simulator slot 19 AddAgent(target, level, name, flags); the
// simulator is held until RemoveAgent.
void AddAgent(void* u, uint32_t level, const char* name, size_t len, uint32_t flags) {
    fc4::GrowthContext* c = G(u);
    if (c->readOnly || c->target == nullptr || name == nullptr) return;
    cISC4Simulator* raw = SimulatorOf(u);
    if (raw == nullptr) return;
    if (c->simulator == nullptr) {
        c->simulator = raw;
        FC4_ADDREF(c->simulator);
    }
    const cRZBaseString n(name, len);
    fc4::AsGame(static_cast<cISC4Simulator*>(c->simulator))->AddAgent(c->target, level, n, flags);
    c->agentsRegistered = true;
}

// §1.3: simulator slot 20 (all levels) on the simulator AddAgent used.
void RemoveAgent(void* u) {
    fc4::GrowthContext* c = G(u);
    if (c->simulator == nullptr) return;
    if (c->agentsRegistered) {
        fc4::AsGame(static_cast<cISC4Simulator*>(c->simulator))->RemoveAgentAllLevels(c->target);
    }
    c->agentsRegistered = false;
    FC4_RELEASE(c->simulator);
    c->simulator = nullptr;
}

// §11: simulator slot 8.
uint32_t SimDateNumber(void* u) {
    cISC4Simulator* s = SimulatorOf(u);
    return s ? static_cast<uint32_t>(fc4::AsGame(s)->GetSimDateNumber()) : 0;
}

// Grid slot 9 (the same slot for every element type).
void GridSetTractSize(void*, uint64_t grid, int32_t size) {
    if (grid == 0) return;
    reinterpret_cast<cISC4SimGrid<uint8_t>*>(FromToken(grid))->SetTractSize(size);
}

void AddRefToken(void*, uint64_t t) { AddRefAny(FromToken(t)); }
// [choice] (DG28) In the destructor (§1.3: the special-building copy stays
// until then) a game object's release is audited as the destructor's: a
// developer loaded from a save is also held by the load's object table,
// which an unsaved city frees after the occupants' other holders, so its
// copy can hold their last references, as the original's does.
// [spec question] DG28's lead (an extra AddRef after CreateInstance) does not
// explain them here: our BDS takes no extra AddRef (CreateBuildingOccupant,
// DG5), and without this workaround run a10a-after-dev (2026-10-15) still
// gave the five warnings on class 0xA9BD882D, all from this release in the
// destructor at shutdown. The workaround stays.
void ReleaseToken(void* u, uint64_t t) {
    cIGZUnknown* o = FromToken(t);
    if (o != nullptr && G(u)->destroying && !fc4::IsOurObject(o)) {
        FC4_RELEASE_AT_DESTRUCTION(o);
        return;
    }
    ReleaseAny(o);
}

// ---- demand and neighbours (§4.8, §5.3, §5.5) -------------------------------------------------------

// Demand simulator slot 6 (borrowed).
uint64_t GetDemand(void* u, uint32_t id, uint32_t index) {
    cISC4City* c = CityOf(u);
    cISC4DemandSimulator* ds = c ? c->GetDemandSimulator() : nullptr;
    return ds ? TokenOf(fc4::AsGame(ds)->GetDemand(id, index)) : 0;
}

float QuerySupply(void*, uint64_t n) {
    cFC4ISC4Demand* d = NodeOf(n);
    return d ? d->QuerySupplyValue() : 0.0f;  // slot 8
}

// Slot 17: a bool in AL with nothing on the x87 stack (docs/abi; FC4Interfaces.h).
void AddToDemandValue(void* u, uint64_t n, float v) {
    if (G(u)->readOnly) return;
    if (cFC4ISC4Demand* d = NodeOf(n)) d->AddToDemandValue(v);
}

float ActiveDemand(void*, uint64_t n) {
    cFC4ISC4Demand* d = NodeOf(n);
    return d ? d->QueryActiveDemandValue() : 0.0f;  // slot 12
}

// Neighbours simulator slots 21 / 22 (list, false): the count. [abi] An A
// list (allocators.md); the cities are borrowed.
uint32_t CountCities(void* u, bool all) {
    cISC4City* c = CityOf(u);
    cISC4NeighborsSimulator* raw = c ? c->GetNeighborsSimulator() : nullptr;
    if (raw == nullptr) return 0;
    cIGZAllocatorServicePtr alloc;
    if (!alloc) return 0;
    cFC4ISC4NeighborsSimulator* n = fc4::AsGame(raw);
    FC4_OUT(SC4List<cISC4RegionalCity*>, list, "SC4List<cISC4RegionalCity*> (growth connected cities)");
    if (all) {
        n->GetAllConnectedCities(list.get(), false);
    } else {
        n->GetConnectedCities(list.get(), false);
    }
    list.Check();
    uint32_t count = 0;
    for (cISC4RegionalCity* city : list.get()) {
        (void)city;
        ++count;
    }
    return count;
}

uint32_t ConnectedCities(void* u) { return CountCities(u, false); }
uint32_t AllConnectedCities(void* u) { return CountCities(u, true); }

// ---- the BDS (§11) -----------------------------------------------------------------------------------

float StagePopulationMultiplier(void* u) {
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b ? b->GetStagePopulationMultiplier() : 1.0f;  // slot 53, ST0
}

uint64_t TractDeveloperToken(void* u) {
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b ? TokenOf(b->GetTractDeveloper()) : 0;  // slot 35, borrowed
}

// Slot 44: the BDS's list, borrowed, in list order.
void SpecialBuildings(void* u, fc4_token_callback cb, void* ctx) {
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr || cb == nullptr) return;
    SC4List<cISC4Occupant*>* list = b->GetSpecialBuildings();
    if (list == nullptr) return;
    std::vector<uint64_t> tokens;
    for (cISC4Occupant* o : *list) {
        if (o != nullptr) tokens.push_back(TokenOf(o));
    }
    for (uint64_t t : tokens) cb(ctx, t);
}

float MaxCapacity(void* u, int32_t zone, uint32_t purpose, uint32_t wealth) {
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b ? b->GetMaxCapacity(zone, purpose, wealth) : 0.0f;  // slot 45 (ST0), abi: via cFC4ISC4BuildingDevelopmentSimulator
}

// §6.3: slot 39 GetBuildingProfile(type).purpose (inserts a zero profile for
// an unknown type [compat]). [choice] The shadow reads slot 40 (no insert)
// so it leaves the original's catalogue as it is; DG25 (§4.7.2): the same
// value, since a null read as purpose 0 equals the zero profile's.
uint32_t BuildingPurpose(void* u, uint32_t type) {
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr) return 0;
    if (G(u)->readOnly) {
        const FC4BuildingProfile* p = b->GetBuildingProfilePtr(type);
        return p ? p->purpose : 0;
    }
    return b->GetBuildingProfile(type).purpose;
}

// ---- the tract developer (§5.2) -----------------------------------------------------------------------

// Slot 7 UpdatePopulation(profile): the developer, the tract and the stage's low byte.
void UpdatePopulation(void* u, uint64_t td, const fc4_growth_profile* profile) {
    if (td == 0 || profile == nullptr || G(u)->readOnly) return;
    if (!Gate(u, fc4::Sw::DevelopmentTract, "growth tract UpdatePopulation")) return;
    FC4GrowthProfile g{};
    g.developer = reinterpret_cast<cISC4GrowthDeveloper*>(FromToken(profile->developer));
    g.x = profile->x;
    g.z = profile->z;
    g.stage = static_cast<uint8_t>(profile->stage);
    fc4::AsGame(reinterpret_cast<cISC4TractDeveloper*>(FromToken(td)))->UpdatePopulation(g);
    ++G(u)->populationUpdates;
}

// ---- zones, lots, occupants, city (§4.4, §6) ------------------------------------------------------------

// Zone manager slot 12 (int8 sign-extended in EAX). Outside the city: 0.
int8_t ZoneType(void* u, int32_t x, int32_t z) {
    cISC4City* c = CityOf(u);
    if (c == nullptr || x < 0 || z < 0) return 0;
    if (static_cast<uint32_t>(x) >= c->CellCountX() || static_cast<uint32_t>(z) >= c->CellCountZ()) return 0;
    if (!Gate(u, fc4::Sw::DevelopmentZoning, "growth GetZoneType (unzoned)")) return 0;
    cISC4ZoneManager* zm = static_cast<cISC4ZoneManager*>(G(u)->services[kZones]);
    return zm ? static_cast<int8_t>(static_cast<int32_t>(zm->GetZoneType(x, z))) : 0;
}

bool LotsOn(void* u, uint64_t lot, const char* what) { return lot != 0 && Gate(u, fc4::Sw::DevelopmentLots, what); }

uint32_t LotBuildingType(void* u, uint64_t lot, bool flag) {
    if (!LotsOn(u, lot, "growth lot GetBuildingType (0)")) return 0;
    return LotOf(lot)->GetBuildingType(flag);  // slot 46
}

uint16_t LotCapacity(void* u, uint64_t lot, uint32_t devType, bool flag) {
    if (!LotsOn(u, lot, "growth lot GetCapacity (0)")) return 0;
    return LotOf(lot)->GetCapacityOfDeveloper(static_cast<cFC4ISC4Lot::DeveloperType>(devType), flag);  // slot 50
}

void LotSize(void* u, uint64_t lot, uint8_t* out2) {
    if (out2 == nullptr) return;
    out2[0] = out2[1] = 0;
    if (!LotsOn(u, lot, "growth lot GetSize (0 x 0)")) return;
    uint8_t w = 0, d = 0;
    LotOf(lot)->GetSize(w, d);  // slot 22
    out2[0] = w;
    out2[1] = d;
}

void LotBoundingRect(void* u, uint64_t lot, fc4_cell_rect* out) {
    if (out == nullptr) return;
    *out = fc4_cell_rect{};
    if (!LotsOn(u, lot, "growth lot GetBoundingRect (empty)")) return;
    SC4Rect<int32_t> r;
    LotOf(lot)->GetBoundingRect(r);  // slot 14
    out->x1 = r.topLeftX;
    out->z1 = r.topLeftY;
    out->x2 = r.bottomRightX;
    out->z2 = r.bottomRightY;
}

void LotLocation(void* u, uint64_t lot, int32_t* out2) {
    if (out2 == nullptr) return;
    out2[0] = out2[1] = 0;
    if (!LotsOn(u, lot, "growth lot GetLocation (origin)")) return;
    int32_t x = 0, z = 0;
    LotOf(lot)->GetLocation(x, z);  // slot 21
    out2[0] = x;
    out2[1] = z;
}

// §4.4: occupant slot 6 AsPropertyHolder, the property copied as it is.
bool OccupantProperty(void* u, uint64_t occ, uint32_t id, fc4_props* scratch) {
    if (occ == 0 || !Gate(u, fc4::Sw::DevelopmentOccupants, "growth occupant property (absent)")) return false;
    cISCPropertyHolder* h = OccOf(occ)->AsPropertyHolder();  // borrowed
    return h != nullptr && fc4::props::CopyToCore(h, id, scratch);
}

void OccupantPosition(void* u, uint64_t occ, float* out3) {
    if (out3 == nullptr) return;
    out3[0] = out3[1] = out3[2] = 0.0f;
    if (occ == 0 || !Gate(u, fc4::Sw::DevelopmentOccupants, "growth occupant GetPosition (origin)")) return;
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);  // slot 8
    out3[0] = v.fX;
    out3[1] = v.fY;
    out3[2] = v.fZ;
}

// City slot 95.
void PositionToCell(void* u, float x, float z, int32_t* out2) {
    if (out2 == nullptr) return;
    int cx = 0, cz = 0;
    if (cISC4City* c = CityOf(u)) c->PositionToCell(x, z, cx, cz);
    out2[0] = cx;
    out2[1] = cz;
}

// ---- source maps (§4.3, §4.7, §11) --------------------------------------------------------------------

cISC4LandValueSimulator* LandValueSim(void* u) {
    return static_cast<cISC4LandValueSimulator*>(G(u)->services[kLandValue]);
}

cISC4TrafficSimulator* TrafficSim(void* u) { return static_cast<cISC4TrafficSimulator*>(G(u)->services[kTraffic]); }

cIGZUnknown* SourceObject(void* u, uint32_t code, uint32_t arg) {
    cISC4City* c = CityOf(u);
    if (c == nullptr) return nullptr;
    switch (code) {
        case FC4_GROWTH_SOURCE_INTRINSIC_LAND_VALUE:
        case FC4_GROWTH_SOURCE_TOTAL_LAND_VALUE:
        case FC4_GROWTH_SOURCE_SLOPE: {
            cISC4LandValueSimulator* lv = LandValueSim(u);
            if (lv == nullptr) return nullptr;
            if (code == FC4_GROWTH_SOURCE_INTRINSIC_LAND_VALUE) return lv->GetIntrinsicLandValueMap();  // 7
            if (code == FC4_GROWTH_SOURCE_TOTAL_LAND_VALUE) return lv->GetTotalLandValueMap();          // 8
            return lv->GetSlopeMap();                                                                  // 9
        }
        case FC4_GROWTH_SOURCE_RESIDENTIAL_PROXIMITY:
        case FC4_GROWTH_SOURCE_SCHOOL_GRADE:
        case FC4_GROWTH_SOURCE_HOSPITAL_GRADE: {
            cISC4ResidentialSimulator* r = c->GetResidentialSimulator();
            if (r == nullptr) return nullptr;
            cFC4ISC4ResidentialSimulator* g = fc4::AsGame(r);
            if (code == FC4_GROWTH_SOURCE_RESIDENTIAL_PROXIMITY) {
                return g->GetProximityMap(static_cast<cFC4ISC4ResidentialSimulator::WealthType>(arg));  // 5
            }
            return code == FC4_GROWTH_SOURCE_SCHOOL_GRADE ? g->GetSchoolAverageGradeMap()  // 39
                                                          : g->GetHospitalAverageGradeMap();  // 40
        }
        case FC4_GROWTH_SOURCE_COMMERCIAL_PROXIMITY: {
            cISC4CommercialSimulator* cs = c->GetCommercialSimulator();
            return cs ? cs->GetProximityMap(static_cast<cISC4BuildingDevelopmentSimulator::DeveloperType>(arg))
                      : nullptr;  // 5
        }
        case FC4_GROWTH_SOURCE_PARK:
        case FC4_GROWTH_SOURCE_LANDMARK:
        case FC4_GROWTH_SOURCE_TRANSIENT_AURA: {
            cISC4AuraSimulator* a = c->GetAuraSimulator();
            if (a == nullptr) return nullptr;
            if (code == FC4_GROWTH_SOURCE_PARK) return a->GetParkMap();          // 14
            if (code == FC4_GROWTH_SOURCE_LANDMARK) return a->GetLandmarkMap();  // 15
            return a->GetTransientAuraGrid();                                    // 13
        }
        case FC4_GROWTH_SOURCE_POLLUTION: {
            cISC4PollutionSimulator* p = c->GetPollutionSimulator();
            return p ? p->GetPollutionGrid(arg) : nullptr;  // 5
        }
        case FC4_GROWTH_SOURCE_CRIME: {
            cISC4CrimeSimulator* cr = c->GetCrimeSimulator();
            return cr ? reinterpret_cast<cFC4ISC4CrimeSimulator*>(cr)->GetCrimeGrid() : nullptr;  // 5
        }
        case FC4_GROWTH_SOURCE_TRIP_LENGTH:
        case FC4_GROWTH_SOURCE_COMMERCIAL_TRAFFIC: {
            cISC4TrafficSimulator* t = TrafficSim(u);
            if (t == nullptr) return nullptr;
            return code == FC4_GROWTH_SOURCE_TRIP_LENGTH ? t->GetTripLengthMap()          // 13
                                                         : t->GetCommercialTrafficMap();  // 9
        }
        default: return nullptr;
    }
}

// A source map with one reference for the core (released through the grid
// table when the core drops it). Not gated by a switch: these are reads the
// update needs; development_city covers them like the other reads.
uint64_t SourceMap(void* u, uint32_t code, uint32_t arg) {
    if (!Gate(u, fc4::Sw::DevelopmentCity, "growth source map (none)")) return 0;
    cIGZUnknown* g = SourceObject(u, code, arg);
    if (g == nullptr) {
        ++G(u)->missingSourceMaps;
        return 0;
    }
    FC4_ADDREF(g);
    ++G(u)->sourceMaps;
    return TokenOf(g);
}

// ---- time slicing and seeds (§5.2, §7) ----------------------------------------------------------------

LONGLONG QpcNow() {
    LARGE_INTEGER q;
    QueryPerformanceCounter(&q);
    return q.QuadPart;
}

LONGLONG QpcFrequency() {
    static LONGLONG f = 0;
    if (f == 0) {
        LARGE_INTEGER q;
        QueryPerformanceFrequency(&q);
        f = q.QuadPart > 0 ? q.QuadPart : 1000;
    }
    return f;
}

// [choice] The state-18 deadline on the performance counter (the original
// times the slice with the simulator's helper, §11 "slicing helper"). DG2
// (growth-developer §5.2): the original reads the time-stamp counter; any
// monotonic real-time clock behaves the same.
void LimitTimerStart(void* u, uint32_t ms) {
    G(u)->deadline = QpcNow() + static_cast<LONGLONG>(ms) * QpcFrequency() / 1000;
}

bool LimitTimerExpired(void* u) { return QpcNow() >= G(u)->deadline; }

// §7: a key seed of 0xFFFFFFFF falls back to the clock in the original. Ours
// is derive(S, 0x0990C138 XOR n) of the development session seed
// (development_seed when set, else the session seed, FC4Seed.h), so a seeded
// run stays reproducible (foundations §6.3; DG23: the original derives it
// from time() and clock(), which cannot be matched; RN2).
uint32_t SessionSeedCb(void*) { return fc4::DevelopmentSessionSeed(nullptr); }

bool g_noteNoAllocator = false;

}  // namespace

// ---- fc4 helpers ----------------------------------------------------------------------------------

namespace fc4 {

fc4_growth_host MakeGrowthHost(GrowthContext* ctx, const HostGridContext* grids) {
    fc4_growth_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.grids = MakeGridApi(grids);
    h.acquire_service = &AcquireService;
    h.release_service = &ReleaseService;
    h.acquire_message_server = &AcquireMessageServer;
    h.release_message_server = &ReleaseMessageServer;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.simulator_present = &SimulatorPresent;
    h.add_agent = &AddAgent;
    h.remove_agent = &RemoveAgent;
    h.sim_date_number = &SimDateNumber;
    h.grid_set_tract_size = &GridSetTractSize;
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.get_demand = &GetDemand;
    h.query_supply = &QuerySupply;
    h.add_to_demand_value = &AddToDemandValue;
    h.active_demand = &ActiveDemand;
    h.connected_cities = &ConnectedCities;
    h.all_connected_cities = &AllConnectedCities;
    h.stage_population_multiplier = &StagePopulationMultiplier;
    h.tract_developer = &TractDeveloperToken;
    h.special_buildings = &SpecialBuildings;
    h.max_capacity = &MaxCapacity;
    h.building_purpose = &BuildingPurpose;
    h.update_population = &UpdatePopulation;
    h.zone_type = &ZoneType;
    h.lot_building_type = &LotBuildingType;
    h.lot_capacity = &LotCapacity;
    h.lot_size = &LotSize;
    h.lot_bounding_rect = &LotBoundingRect;
    h.lot_location = &LotLocation;
    h.occupant_property = &OccupantProperty;
    h.occupant_position = &OccupantPosition;
    h.position_to_cell = &PositionToCell;
    h.source_map = &SourceMap;
    h.limit_timer_start = &LimitTimerStart;
    h.limit_timer_expired = &LimitTimerExpired;
    h.session_seed = &SessionSeedCb;
    return h;
}

void DropGrowthHeld(GrowthContext* ctx) {
    std::vector<cIGZUnknown*> held;
    held.swap(ctx->held);
    for (cIGZUnknown* o : held) ReleaseAny(o);
}

void ReleaseGrowthContext(GrowthContext* ctx) {
    DropGrowthHeld(ctx);
    RemoveAgent(ctx);
    if (ctx->msgServer != nullptr && ctx->target != nullptr && !ctx->readOnly) {
        ctx->msgServer->RemoveNotification(ctx->target, kMsgStateChanged);
        ctx->msgServer->RemoveNotification(ctx->target, kMsgZoningChange);
    }
    ReleaseMessageServer(ctx);
    for (uint32_t s = kServiceCount; s-- > 0;) ReleaseService(ctx, s);
}

}  // namespace fc4

// ---- the object -----------------------------------------------------------------------------------

// Every call into the core: the process lock (the main thread waits; another
// thread waits at most 20 ms) and the call depth. When the outermost call
// returns, the game copies are refreshed and the held references dropped.
class cFC4GrowthDeveloper::Call {
public:
    explicit Call(const cFC4GrowthDeveloper* d) : d_(const_cast<cFC4GrowthDeveloper*>(d)) {
        if (lock_.held()) {
            ++d_->depth_;
        } else {
            ++d_->offMainCalls_;
        }
    }
    ~Call() {
        if (!lock_.held()) return;
        if (--d_->depth_ == 0) {
            d_->Publish();
            fc4::DropGrowthHeld(&d_->ctx_);
        }
    }
    Call(const Call&) = delete;
    Call& operator=(const Call&) = delete;
    bool ok() const { return lock_.held() && d_->core_ != nullptr; }

private:
    cFC4GrowthDeveloper* d_;
    fc4::DevelopmentLock lock_;
};

cIGZUnknown* CreateFC4GrowthDeveloper() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4GrowthDeveloper*>(new cFC4GrowthDeveloper()));
}

cFC4GrowthDeveloper::cFC4GrowthDeveloper() {
    fc4::DevelopmentLock lock;
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    gridCtx_.tag = "growth";
    gridCtx_.writes = fc4::Sw::DevelopmentMapWrites;
    host_ = fc4::MakeGrowthHost(&ctx_, &gridCtx_);
    core_ = fc4_growth_create(&host_, fc4::GrowthStatics(), SelfToken());
    if (core_ == nullptr) fc4::Log("cFC4GrowthDeveloper: core creation failed");
    RebuildStyles();  // an empty map, so the game never sees a null header
    fc4::Log("cFC4GrowthDeveloper(%p) created, core %p", static_cast<void*>(this), static_cast<void*>(core_));
}

cFC4GrowthDeveloper::~cFC4GrowthDeveloper() {
    fc4::DevelopmentLock lock;
    ctx_.destroying = true;
    fc4_growth_destroy(core_);  // the final Release: Shutdown if still initialised, every token released
    core_ = nullptr;
    fc4::ReleaseGrowthContext(&ctx_);
    fc4::FreeGameMap<fc4::StlNodeAllocator>(styles_);
    fc4::Log("cFC4GrowthDeveloper(%p) destroyed (%u updates, %u calls answered without the lock)",
             static_cast<void*>(this), updates_, offMainCalls_.load());
}

// §1: the core decides which sub-object answers; an unknown IID is false
// with *ppv unchanged.
bool cFC4GrowthDeveloper::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (fc4_growth_query_interface(riid)) {
        case 1: *ppvObj = static_cast<cFC4ISC4GrowthDeveloper*>(this); break;
        case 2: *ppvObj = static_cast<cIGZMessageTarget2*>(this); break;
        case 3: *ppvObj = static_cast<cIGZSerializable*>(this); break;
        default: return false;
    }
    ++refCount_;
    return true;
}

uint32_t cFC4GrowthDeveloper::AddRef() { return ++refCount_; }

// §1: count > 1 decrements; otherwise Shutdown, destroy, 0.
uint32_t cFC4GrowthDeveloper::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    Shutdown();
    delete this;
    return 0;
}

// ---- publishing -----------------------------------------------------------------------------------

namespace {
void CollectStyle(void* ctx, uint32_t key, float value) {
    static_cast<std::vector<std::pair<uint32_t, float>>*>(ctx)->emplace_back(key, value);
}
}  // namespace

void cFC4GrowthDeveloper::Publish() {
    if (core_ == nullptr) return;
    fc4_growth_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_growth_get_state(core_, &st)) return;
    type_ = st.target_zone;
    subType_ = st.developer_type;
    purpose_ = st.purpose;
    wealth_ = st.wealth;
    totalCapacity_ = st.total_capacity;
    sourceDemand_ = st.source_demand;
    grids_[0] = st.desirability_map;
    grids_[1] = st.capacity_map;
    grids_[2] = st.population_map;
    grids_[3] = st.max_capacity_map;
    threshold_ = fc4_growth_desirability_threshold(core_);
    monopoly_ = static_cast<int32_t>(fc4_growth_monopoly_use(core_));
    uint8_t caps[4] = {0, 0, 0, 0};
    fc4_growth_caps(core_, caps);
    caps_ = static_cast<uint32_t>(caps[0]) | (static_cast<uint32_t>(caps[1]) << 8) |
            (static_cast<uint32_t>(caps[2]) << 16) | (static_cast<uint32_t>(caps[3]) << 24);
    updateState_ = st.update_state;
    // The lists change in states 17 and 20 only (§5.1, §5.4).
    const bool moved = st.update_state != lastState_;
    if (listsDirty_ || (moved && (lastState_ >= 17 || st.update_state == 0))) RefreshLists();
    if (moved && lastState_ == 20 && st.update_state == 0) {
        ++updates_;
        LogUpdate(st);
    }
    lastState_ = st.update_state;
    if (stylesDirty_) RebuildStyles();
}

// Slots 11 / 12: the core's lists copied into the B vectors (the buffers are
// kept; §12: read-only for callers).
void cFC4GrowthDeveloper::RefreshLists() {
    listsDirty_ = false;
    for (uint32_t which = 0; which < 2; ++which) {
        fc4::GameVector<FC4GrowthItem>& v = which == 0 ? growth_ : decline_;
        size_t n = fc4_growth_list(core_, which, nullptr, 0);
        std::vector<fc4_growth_item> items(n);
        if (n != 0) n = std::min(n, fc4_growth_list(core_, which, items.data(), n));
        v.clear();
        if (n == 0) continue;
        if (!fc4::StlNodeAllocator::Available() || !v.reserve(n)) {
            if (!g_noteNoAllocator) {
                g_noteNoAllocator = true;
                fc4::Log("growth: WARNING the game's STL allocator is not available; the growth and decline lists "
                         "stay empty");
            }
            continue;
        }
        for (size_t i = 0; i < n; ++i) v.push_back(FC4GrowthItem{items[i].x, items[i].z, items[i].max_stage});
    }
}

// Slot 26's map: the core's styles (ascending) with B nodes, values as float bits.
void cFC4GrowthDeveloper::RebuildStyles() {
    if (!fc4::StlNodeAllocator::Available()) return;
    stylesDirty_ = false;
    std::vector<std::pair<uint32_t, float>> styles;
    if (core_ != nullptr) fc4_growth_styles(core_, &CollectStyle, &styles);
    std::vector<uint32_t> keys, values;
    for (const auto& kv : styles) {
        uint32_t bits = 0;
        std::memcpy(&bits, &kv.second, sizeof(bits));
        keys.push_back(kv.first);
        values.push_back(bits);
    }
    fc4::GameMapObject fresh{nullptr, 0, 0};
    if (!fc4::BuildGameMap<fc4::StlNodeAllocator>(fresh, keys.data(), values.data(), keys.size())) {
        fc4::Log("growth: WARNING the style map could not be rebuilt (%u styles)", static_cast<unsigned>(keys.size()));
        return;
    }
    fc4::GameMapObject old = styles_;
    styles_.header = fresh.header;
    styles_.count = fresh.count;
    fc4::FreeGameMap<fc4::StlNodeAllocator>(old);
}

// ---- status lines ---------------------------------------------------------------------------------

void cFC4GrowthDeveloper::LogState(const char* what) {
    fc4_growth_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_growth_get_state(core_, &st)) return;
    const uint64_t g[6] = {st.desirability_map, st.capacity_map,  st.population_map,
                           st.max_capacity_map, st.zoned_map, st.working_map};
    int grids = 0;
    for (uint64_t t : g) grids += t != 0 ? 1 : 0;
    fc4::Log("growth 0x%08X %s: init=%d state=%u zone=%d purpose=%u wealth=%u tract=%u capacity=%d phantom=%d "
             "services=%u server=%u grids=%d/6 source=%s",
             st.developer_type, what, st.initialized, st.update_state, st.target_zone, st.purpose, st.wealth,
             st.tract_size, st.total_capacity, st.last_phantom_sum, st.services, st.message_server, grids,
             st.source_demand != 0 ? "yes" : "none");
}

void cFC4GrowthDeveloper::LogUpdate(const fc4_growth_state& st) {
    const uint32_t today = SimDateNumber(&ctx_);
    fc4::Log("growth 0x%08X update %s: growth=%u decline=%u capacity=%d phantom=%d size=%.1f specials=%u%s "
             "maps=%u missing=%u population updates=%u (update %u, %u off-main calls)",
             st.developer_type, DateText(today != 0 ? today : st.start_day).c_str(), st.growth_items,
             st.decline_items, st.total_capacity, st.last_phantom_sum, static_cast<double>(st.last_city_size),
             st.specials, st.has_salt ? " salt" : "", ctx_.sourceMaps, ctx_.missingSourceMaps,
             ctx_.populationUpdates, updates_, offMainCalls_.load());
}

uint64_t cFC4GrowthDeveloper::GridToken(uint32_t which) {
    uint64_t t = which < 4 ? grids_[which].load() : 0;
    Call call(this);
    if (call.ok()) t = fc4_growth_grid(core_, which);
    return t;
}

// ---- lifecycle (§1.2, §1.3) -----------------------------------------------------------------------

// Slot 3. The holder's properties go to the core as a copy (it keeps no
// game reference); a null holder is false [choice, §1.2 step 3].
bool cFC4GrowthDeveloper::Init(cISCPropertyHolder* holder) {
    FC4_TRACE("GrowthDeveloper::Init");
    bool ok = false;
    {
        Call call(this);
        if (!call.ok()) return false;
        ok = fc4_growth_init(core_, holder != nullptr ? fc4::props::ToCoreAll(holder) : nullptr);
        listsDirty_ = stylesDirty_ = true;
        LogState(ok ? "Init" : "Init FAILED");
    }
    return ok;
}

bool cFC4GrowthDeveloper::Shutdown() {
    FC4_TRACE("GrowthDeveloper::Shutdown");
    // Keep this object alive while the simulator and the message server drop
    // their references (unless this already is the final Release).
    if (!destroying_) AddRef();
    {
        Call call(this);
        if (call.ok()) {
            fc4_growth_state st{};
            st.struct_size = sizeof(st);
            const bool wasInit = fc4_growth_get_state(core_, &st) && st.initialized != 0;
            fc4_growth_shutdown(core_);  // §1.3
            if (wasInit) LogState("Shutdown");
        }
    }
    if (!destroying_) Release();
    return true;
}

// ---- primary slots (§10.1) ------------------------------------------------------------------------

// Slot 5 GetType: the target zone (+0xD0: 1 R, 2 C, 3 I); slot 6 GetSubType:
// the developer type (+0xCC), the key of the BDS developer map (docs/abi
// cISC4GrowthDeveloper rows 5 and 6, re-answers9).
int32_t cFC4GrowthDeveloper::GetType() {
    Call call(this);
    return call.ok() ? fc4_growth_target_zone(core_) : type_.load();
}

uint32_t cFC4GrowthDeveloper::GetSubType() {
    Call call(this);
    return call.ok() ? fc4_growth_sub_type(core_) : subType_.load();
}

uint32_t cFC4GrowthDeveloper::GetPurpose() {
    Call call(this);
    return call.ok() ? fc4_growth_purpose(core_) : purpose_.load();
}

uint32_t cFC4GrowthDeveloper::GetWealth() {
    Call call(this);
    return call.ok() ? fc4_growth_wealth(core_) : wealth_.load();
}

// §6.2: the info fields written, the rest of the profile untouched.
void cFC4GrowthDeveloper::GetConstructionInfo(FC4ConstructionProfile& profile) {
    Call call(this);
    if (!call.ok()) return;
    fc4_construction_profile p{};
    static_assert(sizeof(p) == sizeof(profile), "ConstructionProfile layouts");
    std::memcpy(&p, &profile, sizeof(p));
    fc4_growth_construction_info(core_, &p);
    std::memcpy(&profile, &p, sizeof(p));
}

namespace {
// The value of slots 10 / 28 (no object with a destructor survives this call).
uint32_t BoolOf(bool v) { return v ? 1u : 0u; }
}  // namespace

// Slots 10 and 28 return 0/1 in the whole of EAX (§10.1). The asm block
// leaves it there and the function has no return statement (MSVC: warning
// 4035 silenced); nothing with a destructor is alive at that point.
#pragma warning(push)
#pragma warning(disable : 4035)
// DG26 (growth-developer §10.1): slots 10 and 28 answer in the full EAX, 0 or
// 1 (slot 10 = state < 17 signed, slot 28 = state = 0).
bool cFC4GrowthDeveloper::GrowthAndDeclineListsAreValid() {
    const uint32_t v = BoolOf([this] {
        Call call(this);
        return call.ok() ? fc4_growth_lists_valid(core_) : updateState_.load() < 17;
    }());
    __asm mov eax, v
}
#pragma warning(pop)

fc4::GameVector<FC4GrowthItem>& cFC4GrowthDeveloper::GetGrowthList() { return growth_; }

fc4::GameVector<FC4GrowthItem>& cFC4GrowthDeveloper::GetDeclineList() { return decline_; }

// Slots 13-16: borrowed (no AddRef).
cISC4SimGrid<int16_t>* cFC4GrowthDeveloper::GetDesirabilityMap() {
    return reinterpret_cast<cISC4SimGrid<int16_t>*>(FromToken(GridToken(0)));
}

cISC4SimGrid<uint16_t>* cFC4GrowthDeveloper::GetCapacityMap() {
    return reinterpret_cast<cISC4SimGrid<uint16_t>*>(FromToken(GridToken(1)));
}

cISC4SimGrid<int16_t>* cFC4GrowthDeveloper::GetPopulationMap() {
    return reinterpret_cast<cISC4SimGrid<int16_t>*>(FromToken(GridToken(2)));
}

cISC4SimGrid<uint16_t>* cFC4GrowthDeveloper::GetMaxCapacityMap() {
    return reinterpret_cast<cISC4SimGrid<uint16_t>*>(FromToken(GridToken(3)));
}

int32_t cFC4GrowthDeveloper::GetTotalCapacity() {
    Call call(this);
    return call.ok() ? fc4_growth_total_capacity(core_) : totalCapacity_.load();
}

cISC4Demand* cFC4GrowthDeveloper::GetSourceDemand() {
    uint64_t t = sourceDemand_.load();
    {
        Call call(this);
        if (call.ok()) t = fc4_growth_source_demand(core_);
    }
    return reinterpret_cast<cISC4Demand*>(FromToken(t));  // borrowed
}

uint8_t cFC4GrowthDeveloper::CapAt(uint32_t i) {
    Call call(this);
    if (call.ok()) {
        uint8_t caps[4] = {0, 0, 0, 0};
        fc4_growth_caps(core_, caps);
        return caps[i & 3];
    }
    return static_cast<uint8_t>(caps_.load() >> (8 * (i & 3)));
}

// Slots 19-22 (AL).
uint8_t cFC4GrowthDeveloper::GetPowerSupplyCap() { return CapAt(0); }

uint8_t cFC4GrowthDeveloper::GetWaterSupplyCap() { return CapAt(1); }

uint8_t cFC4GrowthDeveloper::GetFireCoverageCap() { return CapAt(2); }

uint8_t cFC4GrowthDeveloper::GetParkCap() { return CapAt(3); }

int32_t cFC4GrowthDeveloper::GetDesirabilityThreshold() {
    Call call(this);
    return call.ok() ? fc4_growth_desirability_threshold(core_) : threshold_.load();
}

// Slot 24 (ST0): the inserting lookup (§10.1 error 4).
float cFC4GrowthDeveloper::GetAbandonmentThreshold(int32_t densityType) {
    Call call(this);
    return call.ok() ? fc4_growth_abandonment_threshold(core_, densityType) : 0.0f;
}

int32_t cFC4GrowthDeveloper::GetMonopolyUse() {
    Call call(this);
    return call.ok() ? static_cast<int32_t>(fc4_growth_monopoly_use(core_)) : monopoly_.load();
}

fc4::GameMapObject* cFC4GrowthDeveloper::GetStylePercentages() { return &styles_; }

void cFC4GrowthDeveloper::StartMapUpdate() {
    Call call(this);
    if (call.ok()) fc4_growth_start_map_update(core_);
}

#pragma warning(push)
#pragma warning(disable : 4035)
bool cFC4GrowthDeveloper::IsMapUpdateFinished() {
    const uint32_t v = BoolOf([this] {
        Call call(this);
        return call.ok() ? fc4_growth_map_update_finished(core_) : updateState_.load() == 0;
    }());
    __asm mov eax, v
}
#pragma warning(pop)

// Slot 29: the int16 in AX (§10.1 error 2; the upper half is the sign).
int16_t cFC4GrowthDeveloper::EstimateDesirability(int32_t cellX, int32_t cellZ) {
    Call call(this);
    return call.ok() ? static_cast<int16_t>(fc4_growth_estimate_desirability(core_, cellX, cellZ)) : 0;
}

// Slot 30: without the lock, the values of an index outside the vectors
// (DG27, glue-only: -4 / 20 are what the original's slot 30 gives for an
// index outside its vectors, growth-developer §10.1).
void cFC4GrowthDeveloper::GetSubjectiveThresholds(int32_t index, int32_t& min, int32_t& max) {
    Call call(this);
    if (call.ok()) {
        fc4_growth_subjective_thresholds(core_, index, &min, &max);
        return;
    }
    min = -4;
    max = 20;
}

// ---- messages (§3.1) ------------------------------------------------------------------------------

bool cFC4GrowthDeveloper::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("growth DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    Call call(this);
    if (!call.ok()) return true;  // [choice] (DG21, glue-only) a message from another thread without the lock is dropped
    fc4_growth_message m{};
    m.msg_type = pMessage->GetType();
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    if (s != nullptr) {
        m.data1 = static_cast<uint32_t>(s->GetData1());
        m.data2 = static_cast<uint32_t>(s->GetData2());
        m.data3 = static_cast<uint32_t>(s->GetData3());
        m.data4 = static_cast<uint32_t>(s->GetData4());
        if (m.msg_type == kMsgZoningChange) {
            // §6.1: void2 is the rectangle {x1, z1, x2, z2}.
            if (const int32_t* r = static_cast<const int32_t*>(s->GetVoid2())) std::memcpy(m.rect, r, sizeof(m.rect));
        } else if (m.msg_type == kMsgStateChanged) {
            // §6.3: the lot is GetIGZUnknown(), used as a cISC4Lot without
            // QueryInterface [compat]; held until the outermost call returns
            // (a queued message runs then).
            cIGZUnknown* lot = s->GetIGZUnknown();
            if (lot != nullptr) {
                AddRefAny(lot);
                ctx_.held.push_back(lot);
            }
            m.unknown = TokenOf(lot);
        }
    }
    Drop(s);
    return fc4_growth_do_message(core_, &m);  // NewDay: false while the update lags (§3.1)
}

// ---- persistence (§9) -----------------------------------------------------------------------------

bool cFC4GrowthDeveloper::Write(cIGZOStream& s) {
    FC4_TRACE("GrowthDeveloper::Write");
    Call call(this);
    if (!call.ok()) return false;
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_growth_write(core_, a.Get());
    fc4::Log("growth 0x%08X Write -> %d", fc4_growth_sub_type(core_), ok ? 1 : 0);
    return ok;
}

// §9: runs before Init.
bool cFC4GrowthDeveloper::Read(cIGZIStream& s) {
    FC4_TRACE("GrowthDeveloper::Read");
    bool ok = false;
    {
        Call call(this);
        if (!call.ok()) return false;
        fc4::IStreamAdapter a(s);
        ok = fc4_growth_read(core_, a.Get());
        listsDirty_ = true;
        LogState(ok ? "Read" : "Read FAILED");
    }
    return ok;
}

uint32_t cFC4GrowthDeveloper::GetGZCLSID() { return kFC4_CLSID_cSC4GrowthDeveloper; }
