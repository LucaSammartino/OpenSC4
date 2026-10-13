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

#include "cFC4Pollution.h"

#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "SC4Rect.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4HistoryWarehouse.h"
#include "cISC4LandfillDeveloper.h"
#include "cISC4LineItem.h"
#include "cISC4LotManager.h"
#include "cISC4NeighborDeal.h"
#include "cISC4Occupant.h"
#include "cISC4OrdinanceSimulator.h"
#include "cISC4PowerSimulator.h"
#include "cISC4PropDeveloper.h"
#include "cISC4ResidentialSimulator.h"
#include "cISC4SimGrid.h"
#include "cISC4TrafficSimulator.h"
#include "cISC4ZoneManager.h"
#include "cISCPropertyHolder.h"
#include "cISTETerrain.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstring>

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
constexpr uint32_t kMsgPreViewLevelChange = 0xC6CDBF1A;

// Tuning exemplar (§7.1, "Utilities").
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0xC911E35B;

using GridU8 = cISC4SimGrid<uint8_t>;
using GridI16 = cISC4SimGrid<int16_t>;

// `user` is the FC4PollutionGame (ours: the simulator's base; the shadow's: its own).
FC4PollutionGame* G(void* u) { return static_cast<FC4PollutionGame*>(u); }
// Write callbacks are only in our table, so `user` is the simulator.
cFC4PollutionSimulator* Sim(void* u) { return static_cast<cFC4PollutionSimulator*>(G(u)); }

cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cISC4NeighborDeal* DealOf(uint64_t t) { return reinterpret_cast<cISC4NeighborDeal*>(FromToken(t)); }
cFC4ISC4DepartmentBudget* DeptOf(uint64_t t) {
    return t ? fc4::AsGame(reinterpret_cast<cISC4DepartmentBudget*>(FromToken(t))) : nullptr;
}
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

// ---- city, tuning, terrain (reads) --------------------------------------------------------------

void CellCounts(void* u, int32_t* x, int32_t* z) {
    cISC4City* c = G(u)->city;
    if (x != nullptr) *x = c ? static_cast<int32_t>(c->CellCountX()) : 0;  // slot 93
    if (z != nullptr) *z = c ? static_cast<int32_t>(c->CellCountZ()) : 0;  // slot 94
}

void CellWidths(void* u, float* x, float* z) {
    cISC4City* c = G(u)->city;
    if (c == nullptr) return;  // the core's 16 m stay
    if (x != nullptr) *x = c->CellWidthX();  // slot 91, ST0
    if (z != nullptr) *z = c->CellWidthZ();  // slot 92, ST0
}

void PositionToCell(void* u, float x, float z, int32_t* cx, int32_t* cz) {
    int a = 0, b = 0;
    if (cISC4City* c = G(u)->city) c->PositionToCell(x, z, a, b);  // slot 95
    if (cx != nullptr) *cx = a;
    if (cz != nullptr) *cz = b;
}

fc4_props* Tuning(void* u) {
    if (G(u)->gated && !fc4::On(fc4::Sw::ExemplarLoads)) {
        fc4::NoteSkipped(fc4::Sw::ExemplarLoads, "pollution tuning exemplar (none)");
        return nullptr;
    }
    fc4::props::Exemplar ex;
    if (!ex.Load(kTuningType, kTuningGroup, kTuningInstance)) {
        if (G(u)->gated) fc4::Log("pollution: tuning exemplar unavailable; the defaults stay");
        return nullptr;
    }
    return fc4::props::ToCoreAll(ex.Holder());
}

float Altitude(void* u, float x, float z) {
    if (!Gate(u, fc4::Sw::PollutionTerrain, "pollution terrain altitude (0)")) return 0.0f;
    cISTETerrain* t = G(u)->terrain;
    return t ? t->GetAltitude(x, z) : 0.0f;  // compiled slot 13, ST0 (foundations §16)
}

// ---- other simulators (reads) -------------------------------------------------------------------

uint32_t ZoneType(void* u, int32_t x, int32_t z) {
    if (!Gate(u, fc4::Sw::PollutionZoning, "pollution GetZoneType (unzoned)")) return 0;
    cISC4ZoneManager* zm = G(u)->zones;
    return zm ? static_cast<uint32_t>(zm->GetZoneType(x, z)) : 0;  // slot 12
}

uint8_t AirTraffic(void* u, int32_t x, int32_t z) {
    if (!Gate(u, fc4::Sw::PollutionTraffic, "pollution air-polluting traffic (0)")) return 0;
    cISC4TrafficSimulator* t = G(u)->traffic;
    GridU8* g = t ? t->GetAirPollutingTrafficMap() : nullptr;  // slot 8, borrowed
    return g ? g->GetCellValue(x, z) : 0;
}

// [choice] Without an ordinance simulator the factor is 1.0, the neutral
// multiplier (spec question).
float CumulativeFactor(void* u, uint32_t id, uint32_t index) {
    if (!Gate(u, fc4::Sw::PollutionOrdinance, "pollution GetCumulativeFactor (1.0)")) return 1.0f;
    cISC4OrdinanceSimulator* o = G(u)->ordinance;
    return o ? o->GetCumulativeFactor(id, index) : 1.0f;  // slot 25, ST0
}

// FindDeals(type, list&) (neighbours slot 30). [abi] A list, cleared first,
// one reference each: an SC4List, held until the outermost call into the core
// returns (ours) or the shadowed delivery ends.
void FindDeals(void* u, uint32_t type, fc4_token_callback cb, void* ctx) {
    if (cb == nullptr || !Gate(u, fc4::Sw::PollutionNeighbors, "pollution FindDeals (none)")) return;
    cISC4NeighborsSimulator* n = G(u)->neighbors;
    if (n == nullptr) return;
    auto list = std::make_shared<SC4List<cISC4NeighborDeal*>>();
    fc4::AsGame(n)->FindDealsOfType(static_cast<int32_t>(type), *list);
    for (cISC4NeighborDeal* d : *list) {
        if (d != nullptr) cb(ctx, TokenOf(d));
    }
    G(u)->Hold(std::move(list));
}

int32_t DealAmount(void*, uint64_t deal) { return deal ? DealOf(deal)->GetAmount() : 0; }  // slot 10

int32_t DealAmountUsed(void*, uint64_t deal) { return deal ? DealOf(deal)->GetAmountUsed() : 0; }  // slot 20

bool IsOccupantPowered(void* u, uint64_t occ) {
    if (occ == 0 || !Gate(u, fc4::Sw::PollutionPower, "pollution IsOccupantPowered (false)")) return false;
    cISC4PowerSimulator* p = G(u)->power;
    return p != nullptr && p->IsOccupantPowered(OccOf(occ));  // slot 31
}

int32_t Population(void* u) {
    if (!Gate(u, fc4::Sw::PollutionPopulation, "pollution GetPopulation (0)")) return 0;
    cISC4ResidentialSimulator* r = G(u)->residential;
    return r ? fc4::AsGame(r)->GetPopulation() : 0;  // slot 16
}

void LandfillUsage(void* u, uint32_t* usage, uint32_t* capacity) {
    uint32_t a = 0, b = 0;
    if (Gate(u, fc4::Sw::PollutionLandfill, "pollution GetLandfillUsage (0)")) {
        if (cISC4LandfillDeveloper* l = G(u)->landfill) l->GetLandfillUsage(a, b);  // slot 10
    }
    if (usage != nullptr) *usage = a;
    if (capacity != nullptr) *capacity = b;
}

uint32_t LandfillTileCount(void* u) {
    if (!Gate(u, fc4::Sw::PollutionLandfill, "pollution GetLandfillTileCount (0)")) return 0;
    cISC4LandfillDeveloper* l = G(u)->landfill;
    return l ? static_cast<uint32_t>(l->GetLandfillTileCount()) : 0;  // slot 11
}

// ---- occupants (reads) --------------------------------------------------------------------------

bool OccupantProperty(void* u, uint64_t occ, uint32_t id, fc4_props* out) {
    if (occ == 0 || !Gate(u, fc4::Sw::PollutionOccupants, "pollution occupant property read (absent)")) return false;
    cISCPropertyHolder* h = HolderOf(occ);  // borrowed
    return h != nullptr && fc4::props::CopyToCore(h, id, out);
}

void OccupantPosition(void* u, uint64_t occ, float* xyz) {
    if (xyz == nullptr || occ == 0 || !Gate(u, fc4::Sw::PollutionOccupants, "pollution occupant GetPosition")) {
        return;
    }
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);  // slot 8
    xyz[0] = v.fX;
    xyz[1] = v.fY;
    xyz[2] = v.fZ;
}

void OccupantBoundingBox(void* u, uint64_t occ, float* mn, float* mx) {
    if (mn == nullptr || mx == nullptr || occ == 0 ||
        !Gate(u, fc4::Sw::PollutionOccupants, "pollution occupant GetBoundingBox")) {
        return;
    }
    cS3DVector3 a, b;
    OccOf(occ)->GetBoundingBox(&a, &b);  // slot 10
    mn[0] = a.fX;
    mn[1] = a.fY;
    mn[2] = a.fZ;
    mx[0] = b.fX;
    mx[1] = b.fY;
    mx[2] = b.fZ;
}

// ---- budget (reads) -----------------------------------------------------------------------------

float DepartmentFunding(void* u, uint64_t dept) {
    if (!Gate(u, fc4::Sw::PollutionBudget, "pollution GetFundingPercentage (1.0)")) return 1.0f;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    SC4Percentage* p = d ? d->GetFundingPercentage() : nullptr;  // slot 10, a pointer
    return p ? p->percentage : 1.0f;
}

bool BudgetItemForPurpose(void* u, uint64_t occ, uint32_t purpose, fc4_pollution_budget_item* out) {
    if (!Gate(u, fc4::Sw::PollutionBudget, "pollution GetBudgetItemForPurpose (none)")) return false;
    cISC4BudgetSimulator* b = G(u)->budget;
    cISCPropertyHolder* h = HolderOf(occ);
    if (b == nullptr || h == nullptr) return false;
    cFC4ISC4BudgetSimulator::BudgetItem item{};
    if (!fc4::AsGame(b)->GetBudgetItemForPurpose(h, purpose, item)) return false;  // slot 51
    if (out != nullptr) {
        out->department = item.departmentID;
        out->line = item.lineItem;
        out->purpose = item.purpose;
        out->index = static_cast<uint32_t>(item.index);
        out->cost = item.cost;
        out->local_funding = item.localFundingPercentage;
        out->reserved = 0;
    }
    return true;
}

// ---- framework (ours only) ----------------------------------------------------------------------

void AddAgent(void* u, uint32_t level, const char* name, size_t len, uint32_t flags) {
    cISC4City* c = G(u)->city;
    cFC4ISC4Simulator* sim = c ? fc4::AsGame(c->GetSimulator()) : nullptr;
    if (sim == nullptr) {
        fc4::Log("pollution: no simulator; agent level %u not registered", level);
        return;
    }
    const cRZBaseString n(name, len);
    sim->AddAgent(Sim(u)->Target(), level, n, flags);  // slot 19
}

void RemoveAgent(void* u) {
    cISC4City* c = G(u)->city;
    cFC4ISC4Simulator* sim = c ? fc4::AsGame(c->GetSimulator()) : nullptr;
    if (sim != nullptr) sim->RemoveAgentAllLevels(Sim(u)->Target());  // slot 20
}

void Subscribe(void* u, const uint32_t* ids, size_t n) { Sim(u)->Subscribe(ids, n, true); }
void Unsubscribe(void* u, const uint32_t* ids, size_t n) { Sim(u)->Subscribe(ids, n, false); }

// §5.2: a posted (queued) standard message, data1 as given, data2..4 = 0.
void Post(void* u, uint32_t msgType, uint32_t data1) {
    if (!Gate(u, fc4::Sw::PollutionPosts, "pollution message post")) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* msg = FC4MakeMessage(msgType, static_cast<intptr_t>(data1), 0, 0);
    ms->MessagePost(msg, false);
    msg->Release();
}

void RecordHistory(void* u, uint32_t series, int32_t value) {
    if (!Gate(u, fc4::Sw::PollutionHistory, "pollution RecordHistoryData")) return;
    if (cISC4HistoryWarehouse* h = G(u)->history) h->RecordHistoryData(series, value);  // slot 12
}

void SetHistoryQueueLength(void* u, uint32_t series, uint32_t length) {
    if (!Gate(u, fc4::Sw::PollutionHistory, "pollution SetHistoryDataQueueLength")) return;
    if (cISC4HistoryWarehouse* h = G(u)->history) h->SetHistoryDataQueueLength(series, length);  // slot 13
}

// §4.1 step 2.6 / Shutdown: the prop developer (compiled slots 5 / 6) with our
// cISC4PropRequester.
void AddPropRequester(void* u) {
    if (!Gate(u, fc4::Sw::PollutionProps, "pollution AddPropRequester")) return;
    cISC4PropDeveloper* p = G(u)->props;
    if (p != nullptr) p->AddPropRequester(reinterpret_cast<cISC4PropRequester*>(Sim(u)->Requester()));
}

void RemovePropRequester(void* u) {
    if (!Gate(u, fc4::Sw::PollutionProps, "pollution RemovePropRequester")) return;
    cISC4PropDeveloper* p = G(u)->props;
    if (p != nullptr) p->RemovePropRequester(reinterpret_cast<cISC4PropRequester*>(Sim(u)->Requester()));
}

void SetDealAmountUsed(void* u, uint64_t deal, int32_t used) {
    if (deal != 0 && Gate(u, fc4::Sw::PollutionNeighbors, "pollution SetAmountUsed")) {
        DealOf(deal)->SetAmountUsed(used);  // slot 21
    }
}

void MarkDealsDirty(void* u) {
    if (!Gate(u, fc4::Sw::PollutionNeighbors, "pollution MarkDealsDirty")) return;
    if (cISC4NeighborsSimulator* n = G(u)->neighbors) fc4::AsGame(n)->MarkDealsDirty();  // slot 31
}

// GetLot(cellX, cellZ, false) (compiled slot 6, borrowed), then SetCondition (slot 36).
void SetLotCondition(void* u, int32_t x, int32_t z, uint32_t condition) {
    if (!Gate(u, fc4::Sw::PollutionLots, "pollution lot SetCondition")) return;
    cISC4LotManager* lm = G(u)->lotManager;
    cISC4Lot* lot = lm ? fc4::AsGame(lm)->GetLot(x, z, false) : nullptr;
    if (lot != nullptr) fc4::AsGame(lot)->SetCondition(condition);
}

void SetOccupantProperty(void* u, uint64_t occ, uint32_t id, uint16_t memType, bool isArray, const void* data,
                         size_t count) {
    if (occ == 0 || !Gate(u, fc4::Sw::PollutionOccupants, "pollution occupant property write")) return;
    if (!fc4::On(fc4::Sw::PropertyWrites)) {
        fc4::NoteSkipped(fc4::Sw::PropertyWrites, "pollution occupant property write");
        return;
    }
    cISCPropertyHolder* h = HolderOf(occ);
    if (h != nullptr && !fc4::props::SetTyped(h, id, memType, isArray, data, count)) {
        FC4_LOGV("pollution: property 0x%08X (type 0x%X) not written to occupant %p", id, memType, FromToken(occ));
    }
}

void RemoveOccupantProperty(void* u, uint64_t occ, uint32_t id) {
    if (occ == 0 || !Gate(u, fc4::Sw::PollutionOccupants, "pollution occupant property removal")) return;
    if (!fc4::On(fc4::Sw::PropertyWrites)) {
        fc4::NoteSkipped(fc4::Sw::PropertyWrites, "pollution occupant property removal");
        return;
    }
    cISCPropertyHolder* h = HolderOf(occ);
    if (h != nullptr) fc4::AsGame(h)->RemoveProperty(id, false);  // slot 17
}

// ---- references (ours only) ---------------------------------------------------------------------

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

// §6 Read: grids with IID 0xC9BD939A (resolved by the grid table), occupants
// with 0x26BB767E, the department with 0x223E2B67. The stream adapter already
// queried the interface the core asked for (FC4Streams.h), so a host object
// arrives as that interface with one reference, which the core keeps. One of
// ours (our department, when the budget subsystem is on) arrives without one
// (OurObjectRef), so it is taken here.
uint64_t ResolveRef(void*, uint64_t t, uint32_t) {
    cIGZUnknown* u = FromToken(t);
    if (u == nullptr) return 0;
    if (fc4::IsOurObject(u)) u->AddRef();
    return t;
}

// ---- budget (ours only) -------------------------------------------------------------------------

uint64_t CreateDepartment(void* u, uint32_t id, uint32_t group) {
    if (!Gate(u, fc4::Sw::PollutionBudget, "pollution CreateDepartmentBudget (none)")) return 0;
    cISC4BudgetSimulator* b = G(u)->budget;
    return b ? TokenOf(fc4::AsGame(b)->CreateDepartmentBudget(id, group)) : 0;  // slot 25, borrowed
}

void SetDepartmentName(void*, uint64_t dept, uint32_t group, uint32_t instance) {
    if (cFC4ISC4DepartmentBudget* d = DeptOf(dept)) d->SetDepartmentName(group, instance);  // slot 5
}

void SetDepartmentMaxFunding(void*, uint64_t dept, float fraction) {
    if (cFC4ISC4DepartmentBudget* d = DeptOf(dept)) {
        SC4Percentage p;
        p.percentage = fraction;
        d->SetMaxAllowedFundingPercentage(p);  // slot 13
    }
}

void CreateLineItem(void*, uint64_t dept, uint32_t line, bool locallyFunded, uint32_t group, uint32_t instance) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    cISC4LineItem* item = d ? d->CreateLineItem(line, locallyFunded) : nullptr;  // slot 17, borrowed
    if (item != nullptr) item->SetName(group, instance);                         // slot 7
}

void SetLineItemFullExpenses(void* u, uint64_t dept, uint32_t line, int64_t value) {
    if (!Gate(u, fc4::Sw::PollutionBudget, "pollution line item SetFullExpenses")) return;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    cISC4LineItem* item = d ? d->GetLineItem(line) : nullptr;  // slot 20, borrowed
    if (item != nullptr) item->SetFullExpenses(value);         // slot 12
}

void AddBudgetItemToBudget(void* u, uint64_t occ, uint32_t index) {
    if (!Gate(u, fc4::Sw::PollutionBudget, "pollution AddBudgetItemToBudget")) return;
    cISC4BudgetSimulator* b = G(u)->budget;
    cISCPropertyHolder* h = HolderOf(occ);
    if (b != nullptr && h != nullptr) fc4::AsGame(b)->AddBudgetItemToBudget(h, index);  // slot 55
}

void RemoveBudgetItemFromBudget(void* u, uint64_t occ, uint32_t index) {
    if (!Gate(u, fc4::Sw::PollutionBudget, "pollution RemoveBudgetItemFromBudget")) return;
    cISC4BudgetSimulator* b = G(u)->budget;
    cISCPropertyHolder* h = HolderOf(occ);
    if (b != nullptr && h != nullptr) fc4::AsGame(b)->RemoveBudgetItemFromBudget(h, index);  // slot 56
}

void ChangeBudgetItemCost(void* u, uint64_t occ, uint32_t index, int64_t cost) {
    if (!Gate(u, fc4::Sw::PollutionBudget, "pollution ChangeBudgetItemCost")) return;
    cISC4BudgetSimulator* b = G(u)->budget;
    cISCPropertyHolder* h = HolderOf(occ);
    if (b != nullptr && h != nullptr) fc4::AsGame(b)->ChangeBudgetItemCost(h, index, cost);  // slot 53
}

void AddToBudgetItemLineExpenses(void* u, const fc4_pollution_budget_item* in, int64_t delta) {
    if (in == nullptr || !Gate(u, fc4::Sw::PollutionBudget, "pollution budget item AddToFullExpenses")) return;
    cISC4BudgetSimulator* b = G(u)->budget;
    if (b == nullptr) return;
    cFC4ISC4BudgetSimulator::BudgetItem item{};
    item.departmentID = in->department;
    item.lineItem = in->line;
    item.cost = in->cost;
    item.index = static_cast<int32_t>(in->index);
    item.purpose = in->purpose;
    item.localFundingPercentage = in->local_funding;
    cISC4LineItem* line = fc4::AsGame(b)->GetLineItemFromBudgetItem(item);  // slot 57, borrowed
    if (line != nullptr) line->AddToFullExpenses(delta);                     // slot 13
}

// ---- visual effects -----------------------------------------------------------------------------

// §4.10 is not implemented in the glue: the effects manager's
// CreateVisualEffect hands out a cISC4VisualEffect positioned through a
// cS3DTransform, and neither layout is in docs/abi (spec question). The core then
// keeps every effect slot empty; the worst tracts are still computed.
uint64_t CreateVisualEffect(void*, uint32_t id, uint32_t kind) {
    static bool logged = false;
    if (!logged) {
        logged = true;
        fc4::Log("pollution: visual effects are not implemented (effect 0x%08X kind %u and the others not shown)", id,
                 kind);
    }
    return 0;
}

void FillReads(fc4_pollution_host& h) {
    h.cell_counts = &CellCounts;
    h.cell_widths = &CellWidths;
    h.position_to_cell = &PositionToCell;
    h.tuning = &Tuning;
    h.altitude = &Altitude;
    h.zone_type = &ZoneType;
    h.air_traffic = &AirTraffic;
    h.cumulative_factor = &CumulativeFactor;
    h.find_deals = &FindDeals;
    h.deal_amount = &DealAmount;
    h.deal_amount_used = &DealAmountUsed;
    h.is_occupant_powered = &IsOccupantPowered;
    h.population = &Population;
    h.landfill_usage = &LandfillUsage;
    h.landfill_tile_count = &LandfillTileCount;
    h.occupant_property = &OccupantProperty;
    h.occupant_position = &OccupantPosition;
    h.occupant_bounding_box = &OccupantBoundingBox;
    h.department_funding = &DepartmentFunding;
    h.budget_item_for_purpose = &BudgetItemForPurpose;
}

fc4_pollution_host MakeHost(cFC4PollutionSimulator* self, const fc4::HostGridContext* grids) {
    fc4_pollution_host h{};
    h.struct_size = sizeof(h);
    h.user = static_cast<FC4PollutionGame*>(self);
    h.grids = fc4::MakeGridApi(grids);
    FillReads(h);
    // simulator, messages, history
    h.add_agent = &AddAgent;
    h.remove_agent = &RemoveAgent;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.post = &Post;
    h.record_history = &RecordHistory;
    h.set_history_queue_length = &SetHistoryQueueLength;
    h.add_prop_requester = &AddPropRequester;
    h.remove_prop_requester = &RemovePropRequester;
    // other simulators
    h.set_deal_amount_used = &SetDealAmountUsed;
    h.mark_deals_dirty = &MarkDealsDirty;
    h.set_lot_condition = &SetLotCondition;
    // occupants
    h.set_occupant_property = &SetOccupantProperty;
    h.remove_occupant_property = &RemoveOccupantProperty;
    // host objects
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.resolve_ref = &ResolveRef;
    // budget
    h.create_department = &CreateDepartment;
    h.set_department_name = &SetDepartmentName;
    h.set_department_max_funding = &SetDepartmentMaxFunding;
    h.create_line_item = &CreateLineItem;
    h.set_line_item_full_expenses = &SetLineItemFullExpenses;
    h.add_budget_item_to_budget = &AddBudgetItemToBudget;
    h.remove_budget_item_from_budget = &RemoveBudgetItemFromBudget;
    h.change_budget_item_cost = &ChangeBudgetItemCost;
    h.add_to_budget_item_line_expenses = &AddToBudgetItemLineExpenses;
    // visual effects: none created (the other effect callbacks are never reached)
    h.create_visual_effect = &CreateVisualEffect;
    return h;
}

// Slot 46's callback: append with one reference for the caller's list. The
// core hands out BORROWED tokens (its lists hold the references), so this
// AddRefs (§8.1).
void AppendGarbageBuilding(void* ctx, uint64_t t) {
    auto* list = static_cast<SC4List<cISC4Occupant*>*>(ctx);
    cISC4Occupant* o = OccOf(t);
    if (o == nullptr) return;
    FC4_ADDREF_FOR_CALLER(o);
    list->push_back(o);
}

uint32_t WallClockSeed() { return static_cast<uint32_t>(GetTickCount64() ^ GetCurrentProcessId()); }

}  // namespace

fc4_pollution_host fc4::PollutionReadOnlyHost(FC4PollutionGame* game, const HostGridContext* grids) {
    fc4_pollution_host h{};
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

// ---- object -------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4PollutionSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4PollutionSimulator*>(new cFC4PollutionSimulator()));
}

cFC4PollutionSimulator::cFC4PollutionSimulator() {
    host_ = MakeHost(this, &gridCtx_);
    // The visual-effect generator is seeded from the wall clock (foundations §6.2).
    core_ = fc4_pollution_create(&host_, SelfToken(), WallClockSeed());
    if (core_ == nullptr) fc4::Log("cFC4PollutionSimulator: core creation failed");
    fc4::Log("cFC4PollutionSimulator(%p) created, core %p", static_cast<void*>(this), static_cast<void*>(core_));
}

cFC4PollutionSimulator::~cFC4PollutionSimulator() {
    fc4_pollution_destroy(core_);
    core_ = nullptr;
    fc4::Log("cFC4PollutionSimulator destroyed");
}

bool cFC4PollutionSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_IID_cISC4PollutionSimulator:
            *ppvObj = static_cast<cFC4ISC4PollutionSimulator*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case kFC4_IID_cISC4PropRequester:
            *ppvObj = static_cast<cFC4ISC4PropRequester*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        case GZIID_cIGZUnknown:
            // §1: cIGZUnknown is answered with the primary interface.
            *ppvObj = static_cast<cIGZUnknown*>(static_cast<cFC4ISC4PollutionSimulator*>(this));
            break;
        default:
            // The out pointer is left untouched.
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4PollutionSimulator::AddRef() { return ++refCount_; }

// At a count of 1 (or below): Shutdown, destroy, return 0.
uint32_t cFC4PollutionSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

void cFC4PollutionSimulator::AcquireServices() {
    if (city == nullptr) {
        cIGZFrameWork* fw = RZGetFrameWork();
        cIGZApp* app = fw ? fw->Application() : nullptr;
        cISC4App* sc4 = nullptr;
        if (app != nullptr && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) && sc4 != nullptr) {
            FC4_ACQUIRED(sc4, "QueryInterface");
            city = sc4->GetCity();
            FC4_ADDREF(city);
            FC4_RELEASE(sc4);
        }
    }
    if (city == nullptr) return;
    auto take = [](auto*& member, auto* value) {
        if (member == nullptr && value != nullptr) {
            member = value;
            FC4_ADDREF(member);
        }
    };
    take(zones, city->GetZoneManager());
    take(traffic, city->GetTrafficSimulator());
    take(ordinance, city->GetOrdinanceSimulator());
    take(neighbors, city->GetNeighborsSimulator());
    take(power, city->GetPowerSimulator());
    take(residential, city->GetResidentialSimulator());
    take(landfill, city->GetLandfillDeveloper());
    take(budget, city->GetBudgetSimulator());
    take(terrain, city->GetTerrain());
    take(lotManager, city->GetLotManager());
    take(props, city->GetPropDeveloper());
    take(history, city->GetHistoryWarehouse());
}

void cFC4PollutionSimulator::ReleaseServices() {
    auto drop = [](auto*& member) {
        if (member != nullptr) FC4_RELEASE(member);
        member = nullptr;
    };
    drop(history);
    drop(props);
    drop(lotManager);
    drop(terrain);
    drop(budget);
    drop(landfill);
    drop(residential);
    drop(power);
    drop(neighbors);
    drop(ordinance);
    drop(traffic);
    drop(zones);
    drop(city);
}

void cFC4PollutionSimulator::Subscribe(const uint32_t* ids, size_t count, bool add) {
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

// The status line (Init, Read, every month from the first): the core's
// diagnostics; grids as nonzero/tracts sum [min..max] in save order.
void cFC4PollutionSimulator::LogState(const char* what) {
    fc4_pollution_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_pollution_get_state(core_, &st)) return;
    static const char* const kGrid[8] = {"airSrc", "waterSrc", "garbageSrc", "radSrc",
                                         "airVal", "waterVal", "garbageVal", "radVal"};
    char grids[640] = {};
    size_t n = 0;
    for (int i = 0; i < 8 && n < sizeof(grids); ++i) {
        const fc4_pollution_grid_stats& g = st.grids[i];
        const int w = _snprintf_s(grids + n, sizeof(grids) - n, _TRUNCATE, "%s%s %u/%u %lld [%d..%d]",
                                  i ? ", " : "", kGrid[i], g.nonzero, g.tracts, static_cast<long long>(g.sum), g.min,
                                  g.max);
        if (w < 0) break;
        n += static_cast<size_t>(w);
    }
    fc4::Log("pollution %s: init=%u loaded=%u cells=%ux%u tractsPerDay=%u days=%u months=%u resets=%u "
             "reentrant=%u lists wtp/inc/wte/rec=%u/%u/%u/%u avg air/water/garbage=%d/%d/%u uncollected=%u (%u) "
             "supply=%d capacity=%d landfillCells=%u developed=%u efficiency=%.3f recycling=%.4f "
             "last month produced/imported/exported/recycled/incinerated/wte/landfilled=%lld/%lld/%lld/%lld/%lld/"
             "%lld/%lld cost=%lld department=%p effectsVisible=%u; grids %s",
             what, st.initialized, st.loaded, st.cell_x, st.cell_z, st.tracts_per_day, days_, months_,
             st.cycle_resets, st.reentrant_calls, st.lists[0], st.lists[1], st.lists[2], st.lists[3], st.avg_air,
             st.avg_water, st.avg_garbage, st.garbage_uncollected, st.total_uncollected, st.garbage_supply,
             st.garbage_capacity, st.active_landfill_cells, st.developed_tracts, st.efficiency, st.recycling_effect,
             static_cast<long long>(st.last_month[0]), static_cast<long long>(st.last_month[1]),
             static_cast<long long>(st.last_month[2]), static_cast<long long>(st.last_month[3]),
             static_cast<long long>(st.last_month[4]), static_cast<long long>(st.last_month[5]),
             static_cast<long long>(st.last_month[6]), static_cast<long long>(st.last_month_cost),
             static_cast<void*>(FromToken(st.department)), st.effects_visible, grids);
}

// ---- lifecycle (§4.1) ---------------------------------------------------------------------------

bool cFC4PollutionSimulator::Init() {
    FC4_TRACE("PollutionSimulator::Init");
    AcquireServices();
    {
        CoreCall call(this);
        fc4_pollution_init(core_);
    }
    LogState("Init");
    return true;  // §4.1: always
}

bool cFC4PollutionSimulator::Shutdown() {
    FC4_TRACE("PollutionSimulator::Shutdown");
    // Keep this object alive while the simulator, the prop developer and the
    // message server drop their references (unless this is the final Release).
    if (!destroying_) AddRef();
    {
        CoreCall call(this);
        fc4_pollution_shutdown(core_);
    }
    held.clear();
    ReleaseServices();
    fc4::Log("pollution Shutdown (%u days, %u months)", days_, months_);
    if (!destroying_) Release();
    return true;  // §4.1: always
}

// ---- queries (§8.1) -----------------------------------------------------------------------------
//
// Any thread: only the core handle (fc4.h: "any thread") and, for slot 33,
// this object's own float.

cISC4SimGrid<int16_t>* cFC4PollutionSimulator::GetPollutionGrid(uint32_t type) {
    // 0..3 only; null for every other type (§8.1 slot 5 [choice]). No AddRef.
    return reinterpret_cast<GridI16*>(FromToken(fc4_pollution_grid(core_, type)));
}

uint16_t cFC4PollutionSimulator::GetPollutionValue(uint32_t type, uint32_t cellX, uint32_t cellZ) {
    return fc4_pollution_value(core_, type, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ));
}

bool cFC4PollutionSimulator::GetAirValue(uint32_t cellX, uint32_t cellZ, int32_t& value) {
    value = fc4_pollution_slot_cell(core_, 7, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ));
    return true;  // §8.1: no meaningful return; a replacement returns true
}

bool cFC4PollutionSimulator::GetWaterValue(uint32_t cellX, uint32_t cellZ, int32_t& value) {
    value = fc4_pollution_slot_cell(core_, 8, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ));
    return true;
}

bool cFC4PollutionSimulator::GetGarbageValue(uint32_t cellX, uint32_t cellZ, int32_t& value) {
    value = fc4_pollution_slot_cell(core_, 9, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ));
    return true;
}

int32_t cFC4PollutionSimulator::GetAverageAirValue() { return static_cast<int32_t>(fc4_pollution_slot_value(core_, 10)); }

int32_t cFC4PollutionSimulator::GetAverageWaterValue() {
    return static_cast<int32_t>(fc4_pollution_slot_value(core_, 11));
}

int32_t cFC4PollutionSimulator::GetAverageGarbageValue() {
    return static_cast<int32_t>(fc4_pollution_slot_value(core_, 12));
}

bool cFC4PollutionSimulator::IsAirPolluted(uint32_t cellX, uint32_t cellZ) {
    return fc4_pollution_slot_cell(core_, 13, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ)) != 0;
}

bool cFC4PollutionSimulator::IsWaterPolluted(uint32_t cellX, uint32_t cellZ) {
    return fc4_pollution_slot_cell(core_, 14, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ)) != 0;
}

bool cFC4PollutionSimulator::IsGarbageAccumulated(uint32_t cellX, uint32_t cellZ) {
    return fc4_pollution_slot_cell(core_, 15, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ)) != 0;
}

bool cFC4PollutionSimulator::IsRadioactive(uint32_t cellX, uint32_t cellZ) {
    return fc4_pollution_slot_cell(core_, 16, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ)) != 0;
}

bool cFC4PollutionSimulator::GetWorstAirTract(uint32_t& cellX, uint32_t& cellZ, int32_t& value) {
    fc4_pollution_worst_tract(core_, 0, &cellX, &cellZ, &value);
    return true;  // §8.1: no meaningful return; a replacement returns true
}

bool cFC4PollutionSimulator::GetWorstWaterTract(uint32_t& cellX, uint32_t& cellZ, int32_t& value) {
    fc4_pollution_worst_tract(core_, 1, &cellX, &cellZ, &value);
    return true;
}

bool cFC4PollutionSimulator::GetWorstGarbageTract(uint32_t& cellX, uint32_t& cellZ, int32_t& value) {
    fc4_pollution_worst_tract(core_, 2, &cellX, &cellZ, &value);
    return true;
}

uint32_t cFC4PollutionSimulator::GetGarbageSupply() { return static_cast<uint32_t>(fc4_pollution_slot_value(core_, 20)); }

int32_t cFC4PollutionSimulator::GetLandfillTileCapacity() {
    return static_cast<int32_t>(fc4_pollution_slot_value(core_, 21));
}

int32_t cFC4PollutionSimulator::GetGarbageCapacity() { return static_cast<int32_t>(fc4_pollution_slot_value(core_, 22)); }

bool cFC4PollutionSimulator::IsGarbageUncollected() { return fc4_pollution_slot_value(core_, 23) != 0; }

int32_t cFC4PollutionSimulator::GetTotalGarbageUncollected() {
    return static_cast<int32_t>(fc4_pollution_slot_value(core_, 24));
}

// 25-28: no meaningful return in the original; a replacement returns true.
bool cFC4PollutionSimulator::SetIsRadioactive(uint32_t cellX, uint32_t cellZ, bool radioactive) {
    fc4_pollution_set_cell(core_, 25, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ), radioactive ? 1 : 0);
    return true;
}

bool cFC4PollutionSimulator::SetAirValue(uint32_t cellX, uint32_t cellZ, int16_t value) {
    fc4_pollution_set_cell(core_, 26, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ), value);
    return true;
}

bool cFC4PollutionSimulator::SetWaterValue(uint32_t cellX, uint32_t cellZ, int16_t value) {
    fc4_pollution_set_cell(core_, 27, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ), value);
    return true;
}

bool cFC4PollutionSimulator::SetGarbageValue(uint32_t cellX, uint32_t cellZ, int16_t value) {
    fc4_pollution_set_cell(core_, 28, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ), value);
    return true;
}

// §8.1 slot 29: EAX = the applied delta (SDK: bool). [choice] declared int32.
int32_t cFC4PollutionSimulator::AddToGarbageSupply(int32_t amount) {
    return fc4_pollution_add_to_garbage_supply(core_, amount);
}

// §8.1 slot 30: EAX = the argument (SDK: bool).
int32_t cFC4PollutionSimulator::SetGarbageCapacity(int32_t capacity) {
    return fc4_pollution_set_garbage_capacity(core_, capacity);
}

int32_t cFC4PollutionSimulator::GetMonthlyIncineratorCapacity() {
    return static_cast<int32_t>(fc4_pollution_slot_value(core_, 31));
}

int32_t cFC4PollutionSimulator::GetMonthlyWasteToEnergyCapacity() {
    return static_cast<int32_t>(fc4_pollution_slot_value(core_, 32));
}

// §8.1 slot 33: the address of the recycling effect (a float). The member is
// refreshed at every call; a caller that keeps the pointer sees the value of
// its last call (the original's member changes at each end of cycle).
SC4Percentage* cFC4PollutionSimulator::GetRecyclingCenterEffect() {
    recyclingEffect_.percentage = fc4_pollution_recycling_center_effect(core_);
    return &recyclingEffect_;
}

int64_t cFC4PollutionSimulator::GetGarbageAbductedPrevMonth() { return fc4_pollution_slot_value(core_, 34); }
int64_t cFC4PollutionSimulator::GetGarbageProducedLastMonth() { return fc4_pollution_slot_value(core_, 35); }
int64_t cFC4PollutionSimulator::GetGarbageImportedLastMonth() { return fc4_pollution_slot_value(core_, 36); }
int64_t cFC4PollutionSimulator::GetGarbageExportedLastMonth() { return fc4_pollution_slot_value(core_, 37); }
int64_t cFC4PollutionSimulator::GetGarbageRecycledLastMonth() { return fc4_pollution_slot_value(core_, 38); }
int64_t cFC4PollutionSimulator::GetGarbageIncineratedLastMonth() { return fc4_pollution_slot_value(core_, 39); }
int64_t cFC4PollutionSimulator::GetGarbageConvertedToEnergyLastMonth() { return fc4_pollution_slot_value(core_, 40); }
int64_t cFC4PollutionSimulator::GetGarbageLandfilledLastMonth() { return fc4_pollution_slot_value(core_, 41); }

uint32_t cFC4PollutionSimulator::GetIncineratorCount() {
    return static_cast<uint32_t>(fc4_pollution_slot_value(core_, 42));
}

uint32_t cFC4PollutionSimulator::GetWasteToEnergyBuildingCount() {
    return static_cast<uint32_t>(fc4_pollution_slot_value(core_, 43));
}

uint32_t cFC4PollutionSimulator::GetRecyclingCenterCount() {
    return static_cast<uint32_t>(fc4_pollution_slot_value(core_, 44));
}

uint32_t cFC4PollutionSimulator::GetWaterTreatmentPlantCount() {
    return static_cast<uint32_t>(fc4_pollution_slot_value(core_, 45));
}

// §8.1 slot 46: appends (one reference each) incinerators, waste-to-energy
// plants, then recycling centres. No meaningful return; a replacement returns true.
bool cFC4PollutionSimulator::GetGarbageBuildings(SC4List<cISC4Occupant*>& list) {
    fc4_pollution_garbage_buildings(core_, &AppendGarbageBuilding, &list);
    return true;
}

uint32_t cFC4PollutionSimulator::GetBuildingGarbageCapacity(cISC4Occupant* occupant) {
    return fc4_pollution_building_garbage_capacity(core_, TokenOf(occupant));
}

float cFC4PollutionSimulator::GetGarbageScalingFactor() { return fc4_pollution_garbage_scaling_factor(core_); }

bool cFC4PollutionSimulator::IsActiveLandfill(uint32_t cellX, uint32_t cellZ) {
    return fc4_pollution_slot_cell(core_, 49, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ)) != 0;
}

bool cFC4PollutionSimulator::SetIsActiveLandfill(uint32_t cellX, uint32_t cellZ, bool active) {
    fc4_pollution_set_cell(core_, 50, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ), active ? 1 : 0);
    return true;  // §8.1: no meaningful return; a replacement returns true
}

// §8.1 slot 51: the new total (the original's EDX:EAX is meaningless).
int64_t cFC4PollutionSimulator::GarbagePutInLandfill(int32_t amount) {
    return fc4_pollution_garbage_put_in_landfill(core_, amount);
}

bool cFC4PollutionSimulator::GetMaxCapacity(uint32_t property, cISC4Occupant* occupant, uint32_t& result) {
    return fc4_pollution_occupant_u32(core_, 52, TokenOf(occupant), property, &result);
}

bool cFC4PollutionSimulator::GetCurrentCapacity(uint32_t property, cISC4Occupant* occupant, uint32_t& result) {
    return fc4_pollution_occupant_u32(core_, 53, TokenOf(occupant), property, &result);
}

bool cFC4PollutionSimulator::GetRecyclingCenterMaxCapacity(cISC4Occupant* occupant, SC4Percentage& capacity) {
    return fc4_pollution_occupant_percent(core_, 54, TokenOf(occupant), &capacity.percentage);
}

bool cFC4PollutionSimulator::GetRecyclingCenterCurrentCapacity(cISC4Occupant* occupant, SC4Percentage& capacity) {
    return fc4_pollution_occupant_percent(core_, 55, TokenOf(occupant), &capacity.percentage);
}

bool cFC4PollutionSimulator::GetCurrentOccupantCondition(cISC4Occupant* occupant, SC4Percentage& condition) {
    return fc4_pollution_occupant_percent(core_, 56, TokenOf(occupant), &condition.percentage);
}

bool cFC4PollutionSimulator::GetCurrentOccupantEfficiency(cISC4Occupant* occupant, SC4Percentage& efficiency) {
    return fc4_pollution_occupant_percent(core_, 57, TokenOf(occupant), &efficiency.percentage);
}

bool cFC4PollutionSimulator::GetIncineratorUsedCapacity(cISC4Occupant* occupant, uint32_t& capacity) {
    return fc4_pollution_occupant_u32(core_, 58, TokenOf(occupant), 0, &capacity);
}

bool cFC4PollutionSimulator::GetWasteToEnergyUsedCapacity(cISC4Occupant* occupant, uint32_t& capacity) {
    return fc4_pollution_occupant_u32(core_, 59, TokenOf(occupant), 0, &capacity);
}

bool cFC4PollutionSimulator::GetIncineratorCurrentCapacity(cISC4Occupant* occupant, uint32_t& capacity) {
    return fc4_pollution_occupant_u32(core_, 60, TokenOf(occupant), 0, &capacity);
}

bool cFC4PollutionSimulator::GetWasteToEnergyCurrentCapacity(cISC4Occupant* occupant, uint32_t& capacity) {
    return fc4_pollution_occupant_u32(core_, 61, TokenOf(occupant), 0, &capacity);
}

bool cFC4PollutionSimulator::GetWaterTreatmentMaxCapacity(cISC4Occupant* occupant, uint32_t& capacity) {
    return fc4_pollution_occupant_u32(core_, 62, TokenOf(occupant), 0, &capacity);
}

bool cFC4PollutionSimulator::GetWaterTreatmentCurrentCapacity(cISC4Occupant* occupant, uint32_t& capacity) {
    return fc4_pollution_occupant_u32(core_, 63, TokenOf(occupant), 0, &capacity);
}

bool cFC4PollutionSimulator::GetWaterTreatmentUsedCapacity(cISC4Occupant* occupant, uint32_t& capacity) {
    return fc4_pollution_occupant_u32(core_, 64, TokenOf(occupant), 0, &capacity);
}

bool cFC4PollutionSimulator::CalcOccupantMaintenanceCost(cISC4Occupant* occupant, int64_t& cost) {
    return fc4_pollution_maintenance_cost(core_, TokenOf(occupant), &cost);
}

// §4.3 (main thread: it may post and refresh the effects).
void cFC4PollutionSimulator::AddRadiationToCity(cS3DVector3 const& position, float strength, float radius) {
    CoreCall call(this);
    fc4_pollution_add_radiation(core_, position.fX, position.fY, position.fZ, strength, radius);
}

int32_t cFC4PollutionSimulator::GetMaxAirPollutionLevelForUI() {
    return static_cast<int32_t>(fc4_pollution_slot_value(core_, 67));
}

int32_t cFC4PollutionSimulator::GetMaxWaterPollutionLevelForUI() {
    return static_cast<int32_t>(fc4_pollution_slot_value(core_, 68));
}

int32_t cFC4PollutionSimulator::GetMaxGarbagePollutionLevelForUI() {
    return static_cast<int32_t>(fc4_pollution_slot_value(core_, 69));
}

int32_t cFC4PollutionSimulator::GetMaxRadiationPollutionLevelForUI() {
    return static_cast<int32_t>(fc4_pollution_slot_value(core_, 70));
}

// ---- cISC4PropRequester (§8.4, §4.11) -----------------------------------------------------------

int32_t cFC4PollutionSimulator::GetRequestedShare(SC4Rect<int32_t> const& cells) {
    return fc4_pollution_requested_share(core_, cells.topLeftX, cells.topLeftY, cells.bottomRightX,
                                         cells.bottomRightY);
}

uint32_t cFC4PollutionSimulator::GetRequesterSatisfactionId() { return fc4_pollution_prop_requester_slot(core_, 4); }

bool cFC4PollutionSimulator::UsesCustomPropSelector() { return fc4_pollution_prop_requester_slot(core_, 5) != 0; }

uint32_t cFC4PollutionSimulator::SelectProp(fc4::GameVector<uint32_t> const&, int32_t, int32_t) {
    return fc4_pollution_prop_requester_slot(core_, 6);
}

// ---- messages (§5) ------------------------------------------------------------------------------

bool cFC4PollutionSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    const uint32_t type = pMessage->GetType();
    const bool day = type == kMsgSimNewDay, month = type == kMsgSimNewMonth;
    // A service the city created after our Init (§4.1 step 4 runs once).
    if (day || month) AcquireServices();
    CoreCall call(this);
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    cISC4Occupant* occ = nullptr;
    uint32_t viewLevel = 0;
    if (s != nullptr) {
        if (type == kMsgInsertOccupant || type == kMsgRemoveOccupant) {
            // §4.2: the occupant is void1, known by its cISC4Occupant identity.
            occ = Query<cISC4Occupant>(static_cast<cIGZUnknown*>(s->GetVoid1()), kIID_cISC4Occupant);
        } else if (type == kMsgPreViewLevelChange) {
            // §5.3: void2 points at the new view level (valid during the delivery).
            const uint32_t* level = static_cast<const uint32_t*>(s->GetVoid2());
            if (level != nullptr) viewLevel = *level;
        }
    }
    // A message delivered while another call into the core is running is
    // queued by the core and handled when that call returns: keep its
    // occupant alive until then.
    if (depth_ > 1 && occ != nullptr) {
        cIGZUnknown* k = static_cast<cIGZUnknown*>(occ);
        k->AddRef();
        Hold(std::shared_ptr<void>(static_cast<void*>(k), [](void* p) { static_cast<cIGZUnknown*>(p)->Release(); }));
    }
    const bool r = fc4_pollution_do_message(core_, type, TokenOf(occ), viewLevel);
    Drop(occ);
    Drop(s);
    if (day) ++days_;
    if (month) {
        ++months_;
        LogState("month");
    }
    return r;  // §5.3: always true
}

// ---- persistence (§6) ---------------------------------------------------------------------------

bool cFC4PollutionSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("PollutionSimulator::Write");
    CoreCall call(this);
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_pollution_write(core_, a.Get());
    fc4::Log("pollution Write -> %d", ok ? 1 : 0);
    return ok;
}

// Read runs before Init (§6): nothing here is sized from the city.
bool cFC4PollutionSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("PollutionSimulator::Read");
    bool ok = false;
    {
        CoreCall call(this);
        fc4::IStreamAdapter a(s);
        ok = fc4_pollution_read(core_, a.Get());
    }
    fc4::Log("pollution Read -> %d", ok ? 1 : 0);
    LogState("Read");
    return ok;
}

uint32_t cFC4PollutionSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4PollutionSimulator; }
