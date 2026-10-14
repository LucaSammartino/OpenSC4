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

#include "cFC4Aura.h"

#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4CrimeInterfaces.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "SC4Point.h"
#include "SC4Rect.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4CrimeSimulator.h"
#include "cISC4DisasterLayer.h"
#include "cISC4DisasterManager.h"
#include "cISC4HistoryWarehouse.h"
#include "cISC4LandValueSimulator.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4Occupant.h"
#include "cISC4OrdinanceSimulator.h"
#include "cISC4SimGrid.h"
#include "cISC4TrafficSimulator.h"
#include "cISCPropertyHolder.h"
#include "cRZBaseString.h"
#include "cS3DVector3.h"

#include <cstring>
#include <unordered_set>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;

// Messages (§5).
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgInsertOccupant = 0x99EF1142;
constexpr uint32_t kMsgRemoveOccupant = 0x99EF1143;
/// Posted at the end of every complete cycle (§11.1).
constexpr uint32_t kMsgAuraCycleCompleted = 0x2A09B21A;

// Tuning exemplar (§10, "Aura Simulator").
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0xAA023079;

/// The developer types a lot's populations are asked for (§6.5.1, [choice]):
/// the demand nodes' IDs, which are the building development simulator's
/// developer types (docs/abi/layouts.md, "The capacity maps").
constexpr uint32_t kDeveloperTypes[] = {0x1010, 0x1020, 0x1030, 0x3110, 0x3120, 0x3130,
                                        0x3320, 0x3330, 0x4100, 0x4200, 0x4300, 0x4400};

const char* const kInputName[FC4_AURA_INPUT_COUNT] = {
    "air pollution", "water pollution", "garbage",    "radiation",     "school grades", "hospital grades",
    "crime",         "trip length",     "congestion", "population R$", "population R$$", "population R$$$"};

using GridI8 = cISC4SimGrid<int8_t>;
using GridI16 = cISC4SimGrid<int16_t>;
using GridU16 = cISC4SimGrid<uint16_t>;

// `user` is the FC4AuraGame (ours: the simulator's base; the shadow's: its own).
FC4AuraGame* G(void* u) { return static_cast<FC4AuraGame*>(u); }
// Write callbacks are only in our table, so `user` is the simulator.
cFC4AuraSimulator* Sim(void* u) { return static_cast<cFC4AuraSimulator*>(G(u)); }
cISC4City* City(void* u) { return G(u)->city; }

cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cISCPropertyHolder* HolderOf(uint64_t occ) { return occ ? OccOf(occ)->AsPropertyHolder() : nullptr; }  // slot 6

bool Gate(void* u, fc4::Sw s, const char* what) {
    if (!G(u)->gated || fc4::On(s)) return true;
    fc4::NoteSkipped(s, what);
    return false;
}

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

cFC4ISC4BudgetSimulator* Budget(void* u) {
    cISC4City* c = City(u);
    cISC4BudgetSimulator* b = c ? c->GetBudgetSimulator() : nullptr;  // borrowed
    return b ? fc4::AsGame(b) : nullptr;
}

cFC4ISC4LotManager* LotMgr(void* u) {
    cISC4City* c = City(u);
    cISC4LotManager* lm = c ? c->GetLotManager() : nullptr;  // borrowed
    return lm ? fc4::AsGame(lm) : nullptr;
}

// ---- city, simulator, tuning ----------------------------------------------------------------------

void CellWidth(void* u, float* x, float* z) {
    cISC4City* c = City(u);
    if (c == nullptr) return;  // the core's defaults stay
    if (x != nullptr) *x = c->CellWidthX();  // slot 91, ST0
    if (z != nullptr) *z = c->CellWidthZ();  // slot 92, ST0
}

void PositionToCell(void* u, float x, float z, int32_t* cx, int32_t* cz) {
    int a = 0, b = 0;
    if (cISC4City* c = City(u)) c->PositionToCell(x, z, a, b);  // slot 95
    if (cx != nullptr) *cx = a;
    if (cz != nullptr) *cz = b;
}

uint32_t SimDateNumber(void* u) {
    cISC4City* c = City(u);
    cFC4ISC4Simulator* sim = c ? fc4::AsGame(c->GetSimulator()) : nullptr;  // city slot 50
    return sim ? static_cast<uint32_t>(sim->GetSimDateNumber()) : 0;  // slot 8
}

fc4_props* Tuning(void* u) {
    if (G(u)->gated && !fc4::On(fc4::Sw::ExemplarLoads)) {
        fc4::NoteSkipped(fc4::Sw::ExemplarLoads, "aura tuning exemplar (none)");
        return nullptr;
    }
    fc4::props::Exemplar ex;
    if (!ex.Load(kTuningType, kTuningGroup, kTuningInstance)) {
        if (G(u)->gated) fc4::Log("aura: tuning exemplar unavailable; nothing changes (§10)");
        return nullptr;
    }
    return fc4::props::ToCoreAll(ex.Holder());
}

// §10: the base auras go to a process-wide table keyed by developer type that
// other code reads [inferred]. Its layout and readers are not specified (§16
// question 7), so [choice] it is not written; logged once.
void PublishBaseAura(void* u, const int32_t* base) {
    if (G(u)->loggedBaseAura || base == nullptr) return;
    G(u)->loggedBaseAura = true;
    fc4::Log("aura: base auras R$/R$$/R$$$ = %d/%d/%d not copied to the process-wide developer-type table (its "
             "layout and readers are not specified: aura.md §10, §16 question 7) [choice]",
             base[0], base[1], base[2]);
}

// ---- lots (§6.4, §6.5, §6.13 rule R, §7) ------------------------------------------------------------

// [choice] The safe lot iterator's layout is not in docs/abi, so state 4's
// walk is a list of the lots found by GetLot(x, z, false) (lot manager slot
// 6, borrowed) over every cell, each lot once, with the cell it was found
// at. A lot is only reached again if GetLot still returns it at that cell
// (a lot removed between state 4 and state 5's later calls is skipped; the
// pointer is compared, never followed). The stamp (max) does not depend on
// the order (§6.5).
struct LotWalk {
    struct Entry {
        cISC4Lot* lot;
        int32_t x, z;
    };
    std::vector<Entry> lots;
    size_t next = 0;
};

LotWalk* WalkOf(uint64_t t) { return reinterpret_cast<LotWalk*>(static_cast<uintptr_t>(t)); }

uint32_t LotCount(void* u) {
    if (!Gate(u, fc4::Sw::AuraLots, "aura GetLotCount (0: the cycle ends at state 4)")) return 0;
    cFC4ISC4LotManager* lm = LotMgr(u);
    const int32_t n = lm ? lm->GetLotCount() : 0;  // slot 16
    return n > 0 ? static_cast<uint32_t>(n) : 0;
}

uint64_t LotIterator(void* u) {
    if (!Gate(u, fc4::Sw::AuraLots, "aura lot walk (none)")) return 0;
    cISC4City* c = City(u);
    cFC4ISC4LotManager* lm = LotMgr(u);
    if (c == nullptr || lm == nullptr) return 0;
    const int32_t nx = static_cast<int32_t>(c->CellCountX());  // slot 93
    const int32_t nz = static_cast<int32_t>(c->CellCountZ());  // slot 94
    LotWalk* w = new LotWalk();
    std::unordered_set<cISC4Lot*> seen;
    for (int32_t x = 0; x < nx; ++x) {
        for (int32_t z = 0; z < nz; ++z) {
            cISC4Lot* lot = lm->GetLot(x, z, false);  // slot 6, borrowed
            if (lot != nullptr && seen.insert(lot).second) w->lots.push_back({lot, x, z});
        }
    }
    ++G(u)->lotWalks;
    return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(w));
}

void LotIteratorRelease(void*, uint64_t t) { delete WalkOf(t); }

void FillRect(cFC4ISC4Lot* l, fc4_aura_lot* out) {
    SC4Rect<int32_t> r;
    l->GetBoundingRect(r);  // slot 14, inclusive cells
    out->x1 = r.topLeftX;
    out->z1 = r.topLeftY;
    out->x2 = r.bottomRightX;
    out->z2 = r.bottomRightY;
    out->zone_type = static_cast<uint32_t>(l->GetZoneType());  // slot 31
}

// [choice] §6.5.1's GetPopulations (lot slot 54) returns a container that is
// not in docs/abi; the (developer type, count) pairs are read with
// GetPopulation(type) (slot 53) for each developer type instead, and only
// non-zero counts are passed on. A lot whose only residential entry holds 0
// next to a non-residential one therefore reads -128 instead of 0 (spec
// question).
void Populations(void* u, cFC4ISC4Lot* l, fc4_aura_population_sink sink, void* ctx) {
    if (sink == nullptr) return;
    if (!Gate(u, fc4::Sw::AuraPopulations, "aura lot GetPopulation (none: every lot -128)")) return;
    if (!G(u)->loggedPopulations && G(u)->gated) {
        G(u)->loggedPopulations = true;
        fc4::Log("aura: lot populations read with GetPopulation(type) (lot slot 53) for the 12 developer types; "
                 "GetPopulations (slot 54) is not called: its container is not in docs/abi [choice]");
    }
    for (uint32_t type : kDeveloperTypes) {
        const uint16_t n = l->GetPopulation(static_cast<cFC4ISC4Lot::DeveloperType>(type));
        if (n != 0) sink(ctx, type, static_cast<int32_t>(n));
    }
}

bool LotNext(void* u, uint64_t t, fc4_aura_lot* out, fc4_aura_population_sink sink, void* ctx) {
    LotWalk* w = WalkOf(t);
    if (w == nullptr || out == nullptr) return false;
    if (!Gate(u, fc4::Sw::AuraLots, "aura lot walk (none)")) return false;
    cFC4ISC4LotManager* lm = LotMgr(u);
    while (lm != nullptr && w->next < w->lots.size()) {
        const LotWalk::Entry e = w->lots[w->next++];
        if (lm->GetLot(e.x, e.z, false) != e.lot) continue;  // removed since state 4
        cFC4ISC4Lot* l = fc4::AsGame(e.lot);
        *out = fc4_aura_lot{};
        FillRect(l, out);
        out->occupant_wealth = static_cast<int32_t>(l->GetOccupantWealth());  // slot 41
        cISC4BuildingOccupant* b = l->GetBuilding();                           // slot 48, borrowed
        out->building = b ? TokenOf(b->AsOccupant()) : 0;                      // building occupant slot 3
        Populations(u, l, sink, ctx);
        ++G(u)->lotsWalked;
        return true;
    }
    return false;
}

bool LotAt(void* u, int32_t x, int32_t z, fc4_aura_lot* out) {
    if (!Gate(u, fc4::Sw::AuraLots, "aura GetLot (none)")) return false;
    cFC4ISC4LotManager* lm = LotMgr(u);
    cISC4Lot* lot = lm ? lm->GetLot(x, z, false) : nullptr;  // slot 6, borrowed
    if (lot == nullptr) return false;
    if (out != nullptr) {
        *out = fc4_aura_lot{};
        FillRect(fc4::AsGame(lot), out);
    }
    return true;
}

// GetLandValueType (land value slot 5). [choice] Without a land value
// simulator the type is 0 (the original has one in every city; spec silent).
int32_t LandValueType(void* u, int32_t x, int32_t z) {
    if (!Gate(u, fc4::Sw::AuraLandValue, "aura GetLandValueType (0)")) return 0;
    cISC4City* c = City(u);
    cISC4LandValueSimulator* lv = c ? c->GetLandValueSimulator() : nullptr;  // borrowed
    return lv ? static_cast<int32_t>(lv->GetLandValueType(x, z)) : 0;
}

// ---- occupants (§5.2, §7) ---------------------------------------------------------------------------

bool OccupantHasProperty(void* u, uint64_t occ, uint32_t id) {
    if (occ == 0 || !Gate(u, fc4::Sw::AuraOccupants, "aura occupant HasProperty (false)")) return false;
    cISCPropertyHolder* h = HolderOf(occ);
    return h != nullptr && fc4::AsGame(h)->HasProperty(id);  // holder slot 3
}

bool OccupantProperty(void* u, uint64_t occ, uint32_t id, fc4_props* out) {
    if (occ == 0 || !Gate(u, fc4::Sw::AuraOccupants, "aura occupant property read (absent)")) return false;
    cISCPropertyHolder* h = HolderOf(occ);  // borrowed
    return h != nullptr && fc4::props::CopyToCore(h, id, out);
}

uint32_t OccupantType(void* u, uint64_t occ) {
    if (occ == 0 || !Gate(u, fc4::Sw::AuraOccupants, "aura occupant GetType (0)")) return 0;
    return static_cast<uint32_t>(OccOf(occ)->GetType());  // slot 7
}

void OccupantPosition(void* u, uint64_t occ, float* xyz) {
    if (xyz == nullptr || occ == 0 || !Gate(u, fc4::Sw::AuraOccupants, "aura occupant GetPosition")) return;
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);  // slot 8
    xyz[0] = v.fX;
    xyz[1] = v.fY;
    xyz[2] = v.fZ;
}

// §7.2: GetBudgetItemForPurpose(holder, purpose, item) (budget slot 51).
bool BudgetItemDepartment(void* u, uint64_t occ, uint32_t purpose, uint32_t* dept) {
    if (!Gate(u, fc4::Sw::AuraBudget, "aura GetBudgetItemForPurpose (none)")) return false;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    cISCPropertyHolder* h = HolderOf(occ);
    if (b == nullptr || h == nullptr) return false;
    cFC4ISC4BudgetSimulator::BudgetItem item{};
    if (!b->GetBudgetItemForPurpose(h, purpose, item)) return false;
    if (dept != nullptr) *dept = item.departmentID;
    return true;
}

// ---- budget, ordinances (§6.2, §6.3, §6.5.2, §6.14) -------------------------------------------------

// GetDepartmentBudget (slot 28, borrowed), *GetFundingPercentage() (department
// slot 10, a pointer). False without one: the core then uses 1.0 (§6.2, §6.3).
bool DepartmentFunding(void* u, uint32_t dept, float* out) {
    if (!Gate(u, fc4::Sw::AuraBudget, "aura GetFundingPercentage (none: 1.0)")) return false;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    cISC4DepartmentBudget* d = b ? b->GetDepartmentBudget(dept) : nullptr;
    SC4Percentage* p = d ? fc4::AsGame(d)->GetFundingPercentage() : nullptr;
    if (p == nullptr) return false;
    if (out != nullptr) *out = p->percentage;
    return true;
}

float TaxRate(void* u, int32_t index) {
    if (!Gate(u, fc4::Sw::AuraBudget, "aura GetTaxRate (0)")) return 0.0f;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    return b ? b->GetTaxRate(static_cast<uint32_t>(index)) : 0.0f;  // slot 36, ST0
}

// GetCumulativeOffset (ordinance slot 24): an int32 in EAX (the SDK says
// int64; called through the docs/abi declaration, FC4Interfaces.h).
int32_t CumulativeOffset(void* u, uint32_t prop, uint32_t index) {
    if (!Gate(u, fc4::Sw::AuraOrdinance, "aura GetCumulativeOffset (0)")) return 0;
    cISC4City* c = City(u);
    cISC4OrdinanceSimulator* o = c ? c->GetOrdinanceSimulator() : nullptr;  // borrowed
    return o ? fc4::AsGame(o)->GetCumulativeOffset32(prop, index) : 0;
}

// ---- the other simulators' grids (§6.7-§6.12, §6.15) ------------------------------------------------

// An input that is not there: its stage is skipped (the spec's neutral value,
// the core's None). Logged once per input (ours); listed on the cycle line.
uint64_t Missing(void* u, uint32_t which, const char* why) {
    FC4AuraGame* g = G(u);
    const uint32_t bit = 1u << which;
    g->cycleMissing |= bit;
    if (g->gated && (g->missingLogged & bit) == 0) {
        g->missingLogged |= bit;
        fc4::Log("aura: input %s not available (%s); its stage is skipped (the spec's neutral value)",
                 kInputName[which], why);
    }
    return 0;
}

// GetPopulationGrids(&r1, &r2, &r3) (residential slot 25, all three are out
// pointers, §13): once per outermost call; each grid AddRef'd for us and
// released when that call ends (§6.15: "released afterwards").
void FetchPopulationGrids(void* u) {
    FC4AuraGame* g = G(u);
    if (g->popFetched) return;
    g->popFetched = true;
    cISC4City* c = City(u);
    cISC4ResidentialSimulator* r = c ? c->GetResidentialSimulator() : nullptr;  // borrowed
    if (r == nullptr) return;
    GridU16* p[3] = {nullptr, nullptr, nullptr};
    fc4::AsGame(r)->GetPopulationGrids(p[0], p[1], p[2]);
    for (int i = 0; i < 3; ++i) {
        g->pop[i] = p[i];
        if (p[i] == nullptr) continue;
        FC4_ACQUIRED(p[i], "GetPopulationGrids");
        g->held.push_back(p[i]);
    }
}

// The input as a BORROWED token, valid until the outermost call returns.
uint64_t InputGrid(void* u, uint32_t which) {
    if (which >= FC4_AURA_INPUT_COUNT) return 0;
    if (!Gate(u, fc4::Sw::AuraInputs, "aura input grids (none: the stages are skipped)")) return 0;
    cISC4City* c = City(u);
    if (c == nullptr) return Missing(u, which, "no city");
    switch (which) {
        case FC4_AURA_INPUT_AIR:
        case FC4_AURA_INPUT_WATER:
        case FC4_AURA_INPUT_GARBAGE:
        case FC4_AURA_INPUT_RADIATION: {
            cISC4PollutionSimulator* p = c->GetPollutionSimulator();  // borrowed
            if (p == nullptr) return Missing(u, which, "no pollution simulator");
            GridI16* grid = fc4::AsGame(p)->GetPollutionGrid(which - FC4_AURA_INPUT_AIR);  // slot 5, no AddRef
            return grid ? TokenOf(grid) : Missing(u, which, "GetPollutionGrid returned null");
        }
        case FC4_AURA_INPUT_SCHOOL:
        case FC4_AURA_INPUT_HOSPITAL: {
            cISC4ResidentialSimulator* r = c->GetResidentialSimulator();  // borrowed
            if (r == nullptr) return Missing(u, which, "no residential simulator");
            // Slots 39 / 40 return a cISC4SimGrid<float>* (§13 SDK errata: the
            // SDK says int32; same register), not AddRef'd.
            cFC4ISC4ResidentialSimulator* rs = fc4::AsGame(r);
            const int32_t v =
                which == FC4_AURA_INPUT_SCHOOL ? rs->GetSchoolAverageGradeMap() : rs->GetHospitalAverageGradeMap();
            const uint64_t t = static_cast<uint64_t>(static_cast<uint32_t>(v));
            return t ? t : Missing(u, which, "the grade map getter returned null");
        }
        case FC4_AURA_INPUT_CRIME: {
            cISC4CrimeSimulator* cr = c->GetCrimeSimulator();  // borrowed
            if (cr == nullptr) return Missing(u, which, "no crime simulator");
            // The primary interface, called through the docs/abi-checked declaration.
            cISC4SimGrid<uint8_t>* grid = reinterpret_cast<cFC4ISC4CrimeSimulator*>(cr)->GetCrimeGrid();  // 5
            return grid ? TokenOf(grid) : Missing(u, which, "GetCrimeGrid returned null");
        }
        case FC4_AURA_INPUT_TRIP:
        case FC4_AURA_INPUT_CONGESTION: {
            cISC4TrafficSimulator* tr = c->GetTrafficSimulator();  // borrowed
            if (tr == nullptr) return Missing(u, which, "no traffic simulator");
            cISC4SimGrid<uint8_t>* grid =
                which == FC4_AURA_INPUT_TRIP ? tr->GetTripLengthMap() : tr->GetCongestionMap();  // 13 / 10
            return grid ? TokenOf(grid) : Missing(u, which, "the traffic map getter returned null");
        }
        default: {  // the three population grids
            FetchPopulationGrids(u);
            GridU16* grid = G(u)->pop[which - FC4_AURA_INPUT_POP1];
            return grid ? TokenOf(grid) : Missing(u, which, "no residential simulator or a null population grid");
        }
    }
}

// ---- effects on the game (ours only) ----------------------------------------------------------------

void RecordHistory(void* u, uint32_t series, int32_t value) {
    if (!Gate(u, fc4::Sw::AuraHistory, "aura RecordHistoryData")) return;
    cISC4City* c = City(u);
    cISC4HistoryWarehouse* h = c ? c->GetHistoryWarehouse() : nullptr;  // borrowed
    if (h != nullptr) h->RecordHistoryData(series, value);  // slot 12
}

// §6.16: GetDisasterLayer() (city slot 72) -> GetDisasterManager(type) (layer
// slot 7) -> AttemptDisasterStart(cellX, cellZ) (manager slot 6, the MSVC-
// reversed overload as in the SDK). All borrowed; the result is ignored.
void AttemptDisasterStart(void* u, uint32_t manager, int32_t x, int32_t z) {
    if (!Gate(u, fc4::Sw::AuraDisaster, "aura AttemptDisasterStart (riot)")) return;
    cISC4City* c = City(u);
    cISC4DisasterLayer* layer = c ? c->GetDisasterLayer() : nullptr;
    cISC4DisasterManager* m = layer ? layer->GetDisasterManager(manager) : nullptr;
    fc4::Log("aura: riot roll won: AttemptDisasterStart(%d, %d) on manager 0x%08X (%s)", x, z, manager,
             m ? "called" : "no such manager");
    if (m != nullptr) m->AttemptDisasterStart(x, z);
}

// §8.4: 0 police EndStrike (slot 13), 1 fire EndStrike (slot 14), 2
// residential EndHealthStrike (9), 3 EndSchoolStrike (8). All borrowed.
void EndStrike(void* u, uint32_t kind) {
    if (!Gate(u, fc4::Sw::AuraStrikes, "aura EndStrike")) return;
    cISC4City* c = City(u);
    if (c == nullptr) return;
    switch (kind) {
        case 0:
            if (cISC4PoliceSimulator* p = c->GetPoliceSimulator()) fc4::AsGame(p)->EndStrike();
            break;
        case 1:
            if (cISC4FireProtectionSimulator* f = c->GetFireProtectionSimulator()) fc4::AsGame(f)->EndStrike();
            break;
        case 2:
            if (cISC4ResidentialSimulator* r = c->GetResidentialSimulator()) fc4::AsGame(r)->EndHealthStrike();
            break;
        case 3:
            if (cISC4ResidentialSimulator* r = c->GetResidentialSimulator()) fc4::AsGame(r)->EndSchoolStrike();
            break;
        default:
            break;
    }
}

// §11.1: MessagePost(msg, false) with void1 = IGZUnknown = this simulator.
void Post(void* u, uint32_t msgType) {
    if (msgType == kMsgAuraCycleCompleted) Sim(u)->NotePost();
    if (!Gate(u, fc4::Sw::AuraPosts, "aura message post")) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* msg = FC4MakeMessage(msgType, 0, 0, 0);
    void* self = FromToken(Sim(u)->SelfToken());
    msg->SetVoid1(self);
    msg->SetIGZUnknown(static_cast<cIGZUnknown*>(self));
    ms->MessagePost(msg, false);
    msg->Release();
}

// ---- the CPU-cycle limit timer (§6.5) ---------------------------------------------------------------

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

// [choice] The deadline is taken on the performance counter (the original
// reads the TSC with the simulator's cycles-per-millisecond; simulator §5.5).
void LimitTimerStart(void* u, uint32_t ms) {
    G(u)->deadline = QpcNow() + static_cast<LONGLONG>(ms) * QpcFrequency() / 1000;
}

bool LimitTimerExpired(void* u) { return QpcNow() >= G(u)->deadline; }

// ---- framework plumbing (ours only) -----------------------------------------------------------------

void Subscribe(void* u, const uint32_t* ids, uint32_t n) { Sim(u)->Subscribe(ids, n, true); }
void Unsubscribe(void* u, const uint32_t* ids, uint32_t n) { Sim(u)->Subscribe(ids, n, false); }

void AddAgent(void* u, uint32_t level, const char* name, uint32_t flags) {
    cISC4City* c = City(u);
    cFC4ISC4Simulator* sim = c ? fc4::AsGame(c->GetSimulator()) : nullptr;  // city slot 50
    if (sim == nullptr) {
        fc4::Log("aura: no simulator; agent level %u not registered", level);
        return;
    }
    const cRZBaseString n(name ? name : "");
    sim->AddAgent(Sim(u)->Target(), level, n, flags);  // slot 19
}

void RemoveAgent(void* u) {
    cISC4City* c = City(u);
    cFC4ISC4Simulator* sim = c ? fc4::AsGame(c->GetSimulator()) : nullptr;
    if (sim != nullptr) sim->RemoveAgentAllLevels(Sim(u)->Target());  // slot 20
}

// §4.3: the "aura" node of the game-data registry. Not published (budget's
// node is not either: creating registry entries with callbacks is not in
// docs/abi); §4.3 [choice]: "failure is not an error".
bool RegistryAdd(void* u) {
    if (!G(u)->loggedRegistry) {
        G(u)->loggedRegistry = true;
        fc4::Log("aura: game-data registry node 'aura' (add_transient_effect, end_strike) not published (stub, as "
                 "budget's)");
    }
    return false;
}

void RegistryRemove(void*) {}

void FillReads(fc4_aura_host& h) {
    h.cell_width = &CellWidth;
    h.position_to_cell = &PositionToCell;
    h.sim_date_number = &SimDateNumber;
    h.tuning = &Tuning;
    h.lot_count = &LotCount;
    h.lot_iterator = &LotIterator;
    h.lot_next = &LotNext;
    h.lot_iterator_release = &LotIteratorRelease;
    h.lot_at = &LotAt;
    h.land_value_type = &LandValueType;
    h.occupant_has_property = &OccupantHasProperty;
    h.occupant_property = &OccupantProperty;
    h.occupant_type = &OccupantType;
    h.occupant_position = &OccupantPosition;
    h.budget_item_department = &BudgetItemDepartment;
    h.department_funding = &DepartmentFunding;
    h.tax_rate = &TaxRate;
    h.cumulative_offset = &CumulativeOffset;
    h.input_grid = &InputGrid;
}

fc4_aura_host MakeHost(cFC4AuraSimulator* self, const fc4::HostGridContext* grids) {
    fc4_aura_host h{};
    h.struct_size = sizeof(h);
    h.user = static_cast<FC4AuraGame*>(self);
    h.grids = fc4::MakeGridApi(grids);
    FillReads(h);
    h.publish_base_aura = &PublishBaseAura;
    h.record_history = &RecordHistory;
    h.attempt_disaster_start = &AttemptDisasterStart;
    h.end_strike = &EndStrike;
    h.post = &Post;
    h.limit_timer_start = &LimitTimerStart;
    h.limit_timer_expired = &LimitTimerExpired;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.add_agent = &AddAgent;
    h.remove_agent = &RemoveAgent;
    h.registry_add = &RegistryAdd;
    h.registry_remove = &RegistryRemove;
    return h;
}

uint32_t WallClockSeed() { return static_cast<uint32_t>(GetTickCount64() ^ GetCurrentProcessId()); }

void AppendMissing(char* buf, size_t size, uint32_t mask, const char* none) {
    if (mask == 0) {
        strcpy_s(buf, size, none);
        return;
    }
    size_t n = 0;
    int w = _snprintf_s(buf, size, _TRUNCATE, "not available (neutral):");
    if (w > 0) n = static_cast<size_t>(w);
    for (uint32_t i = 0; i < FC4_AURA_INPUT_COUNT && n < size; ++i) {
        if ((mask & (1u << i)) == 0) continue;
        w = _snprintf_s(buf + n, size - n, _TRUNCATE, " %s;", kInputName[i]);
        if (w < 0) break;
        n += static_cast<size_t>(w);
    }
}

}  // namespace

void FC4AuraGame::DropHeld() {
    for (cIGZUnknown* p : held) FC4_RELEASE(p);
    held.clear();
    pop[0] = pop[1] = pop[2] = nullptr;
    popFetched = false;
}

fc4_aura_host fc4::AuraReadOnlyHost(FC4AuraGame* game, const HostGridContext* grids) {
    fc4_aura_host h{};
    h.struct_size = sizeof(h);
    h.user = game;
    h.grids = MakeGridApi(grids);
    // Geometry and reads only: the shadow copies the live grids and never
    // creates, writes, shuts down or releases one.
    h.grids.create = nullptr;
    h.grids.set = nullptr;
    h.grids.fill = nullptr;
    h.grids.shutdown = nullptr;
    h.grids.release = nullptr;
    FillReads(h);
    return h;
}

// ---- object -----------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4AuraSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cISC4AuraSimulator*>(new cFC4AuraSimulator()));
}

cFC4AuraSimulator::cFC4AuraSimulator() {
    host_ = MakeHost(this, &gridCtx_);
    // §9: the cRZRandom is seeded from the wall clock at construction.
    core_ = fc4_aura_create(&host_, SelfToken(), WallClockSeed(), static_cast<uint32_t>(GetTickCount()));
    if (core_ == nullptr) fc4::Log("cFC4AuraSimulator: core creation failed");
    fc4::Log("cFC4AuraSimulator(%p) created, core %p", static_cast<void*>(this), static_cast<void*>(core_));
}

cFC4AuraSimulator::~cFC4AuraSimulator() {
    fc4_aura_destroy(core_);
    core_ = nullptr;
    DropHeld();
    fc4::Log("cFC4AuraSimulator destroyed");
}

cFC4AuraSimulator::CoreCall::~CoreCall() {
    if (--s_->depth_ != 0) return;
    s_->DropHeld();
    if (s_->posts_ != s_->cyclesLogged_) s_->LogCycle();
}

bool cFC4AuraSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_IID_cISC4AuraSimulator:
        case GZIID_cIGZUnknown:
            // §1: 0xC773E880 and 1 -> the primary pointer.
            *ppvObj = static_cast<cISC4AuraSimulator*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        default:
            // The out pointer is left untouched.
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4AuraSimulator::AddRef() { return ++refCount_; }

// §1: above 1, decrement; otherwise Shutdown, destroy, return 0.
uint32_t cFC4AuraSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

// §4.1 step 2: the city (one reference of ours). [choice] The building
// development simulator, which the original holds but never uses, is not
// held; every other service is fetched from the city when it is used.
void cFC4AuraSimulator::AcquireServices() {
    if (city != nullptr) return;
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app != nullptr && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) && sc4 != nullptr) {
        FC4_ACQUIRED(sc4, "QueryInterface");
        city = sc4->GetCity();
        if (city != nullptr) FC4_ADDREF(city);
        FC4_RELEASE(sc4);
    }
}

void cFC4AuraSimulator::ReleaseServices() {
    if (city != nullptr) FC4_RELEASE(city);
    city = nullptr;
}

void cFC4AuraSimulator::Subscribe(const uint32_t* ids, size_t count, bool add) {
    if (ids == nullptr) return;
    if (add && msgServer_ == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            msgServer_ = ms;
            FC4_ADDREF(msgServer_);
        }
    }
    if (msgServer_ == nullptr) return;
    for (size_t i = 0; i < count; ++i) {
        if (add) {
            msgServer_->AddNotification(Target(), ids[i]);
        } else {
            msgServer_->RemoveNotification(Target(), ids[i]);
        }
    }
    if (!add) {
        FC4_RELEASE(msgServer_);
        msgServer_ = nullptr;
    }
}

// The status line (Init, Read, every month): the mayor rating, the aura grid's
// sum / min / max, the hotspots and the last riot roll, and the pipeline.
void cFC4AuraSimulator::LogState(const char* what) {
    fc4_aura_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_aura_get_state(core_, &st)) return;
    char missing[256];
    AppendMissing(missing, sizeof(missing), missingLogged, "none missing so far");
    fc4::Log("aura %s: init=%u state=%d mayor rating=%d max=%d@(%d,%d) min=%d@(%d,%d) aura tracts=%u sum=%d "
             "[%d..%d] base tracts with data=%u hotspots=%u riot roll=%.2f departments=%u global effect=%d "
             "base auras=%d/%d/%d decay long/short=%.2f/%.2f slice step/miss=%u/%d reentrant=%u days=%u months=%u "
             "cycles=%u lot walks=%u lots=%u; inputs %s",
             what, st.initialized, st.state, st.mayor_rating, st.max_rating, st.max_x, st.max_z, st.min_rating,
             st.min_x, st.min_z, st.aura_tracts, st.aura_sum, st.aura_min, st.aura_max, st.base_tracts, st.hotspots,
             st.riot_roll, st.departments, st.global_effect, st.effective_base_aura[0], st.effective_base_aura[1],
             st.effective_base_aura[2], st.long_decay, st.short_decay, st.slice_step, st.slice_miss,
             st.reentrant_calls, days_, months_, posts_, lotWalks, lotsWalked, missing);
}

// One line per completed cycle (the cycle-completed post, §6.17).
void cFC4AuraSimulator::LogCycle() {
    cyclesLogged_ = posts_;
    fc4_aura_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_aura_get_state(core_, &st)) return;
    char missing[256];
    AppendMissing(missing, sizeof(missing), cycleMissing, "all read");
    fc4::Log("aura cycle %u completed (day %u, started day %u): mayor rating=%d max=%d@(%d,%d) min=%d@(%d,%d) "
             "aura tracts=%u sum=%d [%d..%d] base tracts with data=%u hotspots=%u riot roll=%.2f lots=%u; inputs %s",
             posts_, SimDateNumber(static_cast<FC4AuraGame*>(this)), st.cycle_start_day, st.mayor_rating,
             st.max_rating, st.max_x, st.max_z, st.min_rating, st.min_x, st.min_z, st.aura_tracts, st.aura_sum,
             st.aura_min, st.aura_max, st.base_tracts, st.hotspots, st.riot_roll, lotsWalked, missing);
    cycleMissing = 0;
    lotsWalked = 0;
}

// ---- lifecycle (§4) ---------------------------------------------------------------------------------

bool cFC4AuraSimulator::Init() {
    FC4_TRACE("AuraSimulator::Init");
    AcquireServices();
    {
        CoreCall call(this);
        fc4_aura_init(core_);
    }
    LogState("Init");
    return true;  // §4.1: always
}

bool cFC4AuraSimulator::Shutdown() {
    FC4_TRACE("AuraSimulator::Shutdown");
    // Keep this object alive while the simulator and the message server drop
    // their references (unless this is the final Release).
    if (!destroying_) AddRef();
    {
        CoreCall call(this);
        fc4_aura_shutdown(core_);
    }
    DropHeld();
    ReleaseServices();
    fc4::Log("aura Shutdown (%u days, %u months, %u cycles)", days_, months_, posts_);
    if (!destroying_) Release();
    return true;  // §4.2: always
}

// ---- cISC4AuraSimulator (§8) ------------------------------------------------------------------------
//
// The getters run on any thread: they read the snapshot only.

int8_t cFC4AuraSimulator::GetAuraValue(int32_t x, int32_t z) {
    // §8.3: aura.GetCellValue(x, z) (cells). 0 without a grid [choice].
    GridI8* g = reinterpret_cast<GridI8*>(FromToken(fc4_aura_grid(core_, FC4_AURA_GRID_AURA)));
    return g ? g->GetCellValue(x, z) : 0;
}

void cFC4AuraSimulator::AddTransientEffect(float x, float z, float radius, float magnitude) {
    CoreCall call(this);
    fc4_aura_add_transient_effect(core_, x, z, radius, magnitude);
}

void cFC4AuraSimulator::AddTransientEffect(float x, float z, uint32_t type) {
    CoreCall call(this);
    fc4_aura_add_transient_effect_type(core_, x, z, type);
}

int8_t cFC4AuraSimulator::GetMayorRating() const {
    fc4_aura_ratings r{};
    r.struct_size = sizeof(r);
    return fc4_aura_get_ratings(core_, &r) ? r.mayor_rating : 0;
}

int8_t cFC4AuraSimulator::GetMaxMayorRating(SC4Point<long>& location) const {
    fc4_aura_ratings r{};
    r.struct_size = sizeof(r);
    if (!fc4_aura_get_ratings(core_, &r)) return 0;
    location.x = r.max_x;
    location.y = r.max_z;
    return r.max_rating;
}

int8_t cFC4AuraSimulator::GetMinMayorRating(SC4Point<long>& location) const {
    fc4_aura_ratings r{};
    r.struct_size = sizeof(r);
    if (!fc4_aura_get_ratings(core_, &r)) return 0;
    location.x = r.min_x;
    location.y = r.min_z;
    return r.min_rating;
}

void cFC4AuraSimulator::BustStrike(eStrikeBuster type) {
    CoreCall call(this);
    fc4_aura_bust_strike(core_, static_cast<uint32_t>(type));
}

// §8.3: the grid pointers, no AddRef.
cISC4SimGrid<int8_t>* cFC4AuraSimulator::GetAuraGrid() {
    return reinterpret_cast<GridI8*>(FromToken(fc4_aura_grid(core_, FC4_AURA_GRID_AURA)));
}

cISC4SimGrid<int8_t>* cFC4AuraSimulator::GetTransientAuraGrid() {
    return reinterpret_cast<GridI8*>(FromToken(fc4_aura_grid(core_, FC4_AURA_GRID_TRANSIENT)));
}

cISC4SimGrid<int16_t>* cFC4AuraSimulator::GetParkMap() const {
    return reinterpret_cast<GridI16*>(FromToken(fc4_aura_grid(core_, FC4_AURA_GRID_PARK)));
}

cISC4SimGrid<int16_t>* cFC4AuraSimulator::GetLandmarkMap() const {
    return reinterpret_cast<GridI16*>(FromToken(fc4_aura_grid(core_, FC4_AURA_GRID_LANDMARK)));
}

// ---- cIGZMessageTarget2 (§5) ------------------------------------------------------------------------

bool cFC4AuraSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    const uint32_t type = pMessage->GetType();
    const bool day = type == kMsgSimNewDay, month = type == kMsgSimNewMonth;
    CoreCall call(this);
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    uint32_t d1 = 0, d2 = 0, d3 = 0;
    cISC4Occupant* occ = nullptr;
    if (s != nullptr) {
        d1 = static_cast<uint32_t>(s->GetData1());
        d2 = static_cast<uint32_t>(s->GetData2());
        d3 = static_cast<uint32_t>(s->GetData3());
        if (type == kMsgInsertOccupant || type == kMsgRemoveOccupant) {
            // §5.2: the occupant is void1, known by its cISC4Occupant identity.
            occ = Query<cISC4Occupant>(static_cast<cIGZUnknown*>(s->GetVoid1()), kIID_cISC4Occupant);
        }
    }
    // A message delivered while another call into the core is running is
    // queued by the core and handled when that call returns: keep its
    // occupant alive until then.
    if (depth_ > 1 && occ != nullptr) {
        FC4_ADDREF(occ);
        held.push_back(occ);
    }
    const bool r = fc4_aura_do_message(core_, type, d1, d2, d3, TokenOf(occ));
    Drop(occ);
    Drop(s);
    if (day) ++days_;
    if (month) {
        ++months_;
        // Not inside another call into the core (its state is borrowed there).
        if (depth_ == 1) LogState("month");
    }
    return r;  // §5: true except a late SimNewDay
}

// ---- persistence (§11) ------------------------------------------------------------------------------

bool cFC4AuraSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("AuraSimulator::Write");
    CoreCall call(this);
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_aura_write(core_, a.Get());
    fc4::Log("aura Write -> %d", ok ? 1 : 0);
    return ok;
}

// Read runs before Init (§11.3): nothing here is sized from the city.
bool cFC4AuraSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("AuraSimulator::Read");
    bool ok = false;
    {
        CoreCall call(this);
        fc4::IStreamAdapter a(s);
        ok = fc4_aura_read(core_, a.Get());
    }
    fc4::Log("aura Read -> %d", ok ? 1 : 0);
    LogState("Read");
    return ok;
}

uint32_t cFC4AuraSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4AuraSimulator; }
