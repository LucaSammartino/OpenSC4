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

#include "cFC4Protection.h"

#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"
#include "compat/FC4Compat.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "SC4String.h"
#include "cIGZAllocatorService.h"
#include "cIGZApp.h"
#include "cIGZCOM.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4AutomataControllerManager.h"
#include "cISC4City.h"
#include "cISC4DisasterLayer.h"
#include "cISC4HistoryWarehouse.h"
#include "cISC4LineItem.h"
#include "cISC4LotManager.h"
#include "cISC4Occupant.h"
#include "cISC4SimGrid.h"
#include "cISC4ZoneManager.h"
#include "cISCPropertyHolder.h"
#include "cISTETerrain.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"

#include <cstring>
#include <string>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4Lot = 0xE818FCED;
/// cISC4PropOccupant (§7.1: prop stations use the department funding only).
constexpr uint32_t kIID_cISC4PropOccupant = 0xE9793A65;

// Messages handled (§5).
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgInsertOccupant = 0x99EF1142;
constexpr uint32_t kMsgRemoveOccupant = 0x99EF1143;
constexpr uint32_t kMsgOccupantChanged = 0xC8DAC052;
/// Police only: a crime event; GetIGZUnknown() is the lot (§9.2).
constexpr uint32_t kMsgCrime = 0x0A159D95;

// Messages posted (§12).
constexpr uint32_t kMsgFireCycleEnd = 0x68F54CE0;
constexpr uint32_t kMsgFireStrikeBegin = 0x68F54CE1;
constexpr uint32_t kMsgPoliceCycleEnd = 0xA8DAF510;
constexpr uint32_t kMsgPoliceStrikeBegin = 0xA8DAF511;
constexpr uint32_t kMsgJailbreakBegin = 0x2A614891;
constexpr uint32_t kMsgJailbreakEnd = 0x2A614892;
constexpr uint32_t kMsgPoliceStationsChanged = 0xA949569A;

// Tuning exemplar (§6.7).
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance[2] = {0x68F5524A, 0x28DFCF89};

/// The air dispatch sources (§8.4, docs/abi/direct-calls.md): fire plane,
/// police helicopter.
constexpr uint32_t kAirSourceClsid[2] = {0x4C0AE114, 0x0C0B06E8};

/// The dispatch types (§1): fire engines 1, police cars 0.
constexpr uint32_t kDispatchType[2] = {1, 0};

const char* const kTag[2] = {"fire", "police"};

using GridU8 = cISC4SimGrid<uint8_t>;

fc4::ProtectionContext* Ctx(void* u) { return static_cast<fc4::ProtectionContext*>(u); }
cISC4City* City(void* u) { return Ctx(u)->city; }

cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cISCPropertyHolder* HolderOf(uint64_t occ) { return occ ? OccOf(occ)->AsPropertyHolder() : nullptr; }
cFC4ISC4DepartmentBudget* DeptOf(uint64_t t) {
    return t ? fc4::AsGame(reinterpret_cast<cISC4DepartmentBudget*>(FromToken(t))) : nullptr;
}

bool Gate(fc4::Sw s, const char* what) {
    if (fc4::On(s)) return true;
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

// The budget simulator held by acquire_services (§4.1), else the city's
// (the read-only host holds nothing).
cFC4ISC4BudgetSimulator* Budget(void* u) {
    cISC4BudgetSimulator* b = Ctx(u)->budget;
    if (b == nullptr) {
        cISC4City* c = City(u);
        b = c ? c->GetBudgetSimulator() : nullptr;  // city slot 52
    }
    return b ? fc4::AsGame(b) : nullptr;
}

cFC4ISC4DispatchManager* DispatchManager(void* u) {
    cISC4City* c = City(u);
    cISC4DispatchManager* d = c ? c->GetDispatchManager() : nullptr;  // city slot 39
    return d ? fc4::AsGame(d) : nullptr;
}

// ---- lifecycle (§4) -------------------------------------------------------------------------------

// §4.1 base init step 1: hold the city, the budget simulator (city slot 52)
// and the zone manager (city slot 36), each only if not held yet; step 4:
// hold the message server only if not held yet (PR12). [choice] The
// flammability simulator (fire) and the crime simulator (police), which the
// original holds but never calls, are not held, and neither is the simulator
// (the original reads a cache global each time): PR12, only reference counts
// differ. The other services are fetched from the city when used. The
// original crashes on a missing service; we tolerate one.
void AcquireServices(void* u, uint32_t) {
    fc4::ProtectionContext* c = Ctx(u);
    if (c->city == nullptr) {
        cIGZFrameWork* fw = RZGetFrameWork();
        cIGZApp* app = fw ? fw->Application() : nullptr;
        cISC4App* sc4 = nullptr;
        if (app != nullptr && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) && sc4 != nullptr) {
            FC4_ACQUIRED(sc4, "QueryInterface");
            c->city = sc4->GetCity();
            if (c->city != nullptr) FC4_ADDREF(c->city);
            FC4_RELEASE(sc4);
        }
    }
    if (c->city == nullptr) fc4::Log("protection (%s): no city; services not acquired", c->tag);
    if (c->city != nullptr && c->budget == nullptr) {
        c->budget = c->city->GetBudgetSimulator();  // city slot 52
        if (c->budget != nullptr) FC4_ADDREF(c->budget);
    }
    if (c->city != nullptr && c->zoneManager == nullptr) {
        c->zoneManager = c->city->GetZoneManager();  // city slot 36
        if (c->zoneManager != nullptr) FC4_ADDREF(c->zoneManager);
    }
    if (c->msgServer == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            c->msgServer = ms;
            FC4_ADDREF(c->msgServer);
        }
    }
}

// §4.2: the last step of the base Shutdown, after the grids (PR15, answered:
// the bracket matches). The original releases the zone manager, then the
// budget simulator, then the city, as here; the message server goes first
// (its release is never the last, so the timing cannot be observed). A
// read-only shadow host holds nothing and asks the city each time, which
// PR15 says cannot be told apart.
void ReleaseServices(void* u, uint32_t) {
    fc4::ProtectionContext* c = Ctx(u);
    if (c->msgServer != nullptr) FC4_RELEASE(c->msgServer);
    c->msgServer = nullptr;
    if (c->zoneManager != nullptr) FC4_RELEASE(c->zoneManager);
    c->zoneManager = nullptr;
    if (c->budget != nullptr) FC4_RELEASE(c->budget);
    c->budget = nullptr;
    if (c->city != nullptr) FC4_RELEASE(c->city);
    c->city = nullptr;
}

// §4.1 base init step 3: AddAgent(target, level, name, flags) per level.
void AddAgent(void* u, const char* name, size_t len, const uint32_t* levels, size_t count, uint32_t flags) {
    fc4::ProtectionContext* c = Ctx(u);
    cFC4ISC4Simulator* sim = c->city ? fc4::AsGame(c->city->GetSimulator()) : nullptr;  // city slot 50
    if (sim == nullptr || levels == nullptr) {
        fc4::Log("protection (%s): no simulator; agents not registered", c->tag);
        return;
    }
    const cRZBaseString n(name, len);
    for (size_t i = 0; i < count; ++i) sim->AddAgent(c->target, levels[i], n, flags);  // slot 19
    c->agentsRegistered = true;
}

void RemoveAgent(void* u) {
    fc4::ProtectionContext* c = Ctx(u);
    if (!c->agentsRegistered) return;
    cFC4ISC4Simulator* sim = c->city ? fc4::AsGame(c->city->GetSimulator()) : nullptr;
    if (sim != nullptr) sim->RemoveAgentAllLevels(c->target);  // slot 20
    c->agentsRegistered = false;
}

// AddNotification / RemoveNotification, one call per type in order. The
// message server is held from acquire_services to release_services, which
// bracket every (un)subscription (§4).
void Notifications(void* u, const uint32_t* ids, size_t n, bool add) {
    fc4::ProtectionContext* c = Ctx(u);
    if (ids == nullptr || c->msgServer == nullptr) return;
    for (size_t i = 0; i < n; ++i) {
        if (add) {
            c->msgServer->AddNotification(c->target, ids[i]);
        } else {
            c->msgServer->RemoveNotification(c->target, ids[i]);
        }
    }
}
void Subscribe(void* u, const uint32_t* ids, size_t n) { Notifications(u, ids, n, true); }
void Unsubscribe(void* u, const uint32_t* ids, size_t n) { Notifications(u, ids, n, false); }

// §4.1 base init step 5 / base shutdown: the dispatch-source sub-object.
void RegisterDispatchSource(void* u) {
    fc4::ProtectionContext* c = Ctx(u);
    if (!Gate(fc4::Sw::ProtectionDispatch, "protection RegisterDispatchSource")) return;
    cFC4ISC4DispatchManager* dm = DispatchManager(u);
    if (dm == nullptr || c->source == nullptr) return;
    // [abi-ret] Slot 13 returns nothing (dispatch.md §8 row "none"; interfaces.tsv eax-or-void): the SDK's bool
    // is a leftover AL, and a false one would skip the unregister at shutdown while the manager still holds
    // the source. The call always registers (no duplicate check), so it is the result.
    dm->RegisterDispatchSource(fc4::AsSdk(c->source));  // slot 13
    c->dispatchRegistered = true;
}

void UnregisterDispatchSource(void* u) {
    fc4::ProtectionContext* c = Ctx(u);
    if (!c->dispatchRegistered) return;
    cFC4ISC4DispatchManager* dm = DispatchManager(u);
    if (dm != nullptr && c->source != nullptr) dm->UnregisterDispatchSource(fc4::AsSdk(c->source));  // slot 14
    c->dispatchRegistered = false;
}

// SetInstanceID (§2): the slot does not depend on the element type.
void SetGridInstance(void*, uint64_t grid, uint32_t instance) {
    if (grid != 0) reinterpret_cast<GridU8*>(FromToken(grid))->SetInstanceID(instance);
}

// ---- references -----------------------------------------------------------------------------------

void AddRefToken(void*, uint64_t t) {
    cIGZUnknown* u = FromToken(t);
    if (u == nullptr) return;
    if (fc4::IsOurObject(u)) {
        u->AddRef();
    } else {
        FC4_ADDREF(u);
    }
}

// The air source's reference is the one the GZCOM factory (or the save)
// handed us, so its last Release is expected (FC4_RELEASE_CREATED).
void ReleaseToken(void* u, uint64_t t) {
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->Release();
    } else if (t == Ctx(u)->airToken) {
        FC4_RELEASE_CREATED(o);
        Ctx(u)->airToken = 0;
    } else {
        FC4_RELEASE(o);
    }
}

// ---- resources ------------------------------------------------------------------------------------

fc4_props* Tuning(void* u, uint32_t kind) {
    if (!Gate(fc4::Sw::ExemplarLoads, "protection tuning exemplar (none)")) return nullptr;
    fc4::props::Exemplar ex;
    if (!ex.Load(kTuningType, kTuningGroup, kTuningInstance[kind & 1])) {
        fc4::Log("protection (%s): tuning exemplar unavailable; the code defaults stay", Ctx(u)->tag);
        return nullptr;
    }
    return fc4::props::ToCoreAll(ex.Holder());
}

// ---- budget (§4.1, §6.1, §6.6) --------------------------------------------------------------------

// CreateDepartmentBudget (slot 25, borrowed), AddRef (the core's reference),
// SetDepartmentName, SetMaxAllowedFundingPercentage, CreateLineItem(1, true),
// GetLineItem(1).SetDisplayFlag(1, false) (§4.1 fire step 4).
uint64_t CreateDepartment(void* u, uint32_t id, uint32_t group, uint32_t nameGroup, uint32_t nameInstance,
                          float maxFunding) {
    if (!Gate(fc4::Sw::ProtectionBudget, "protection CreateDepartmentBudget (none)")) return 0;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    cISC4DepartmentBudget* d = b ? b->CreateDepartmentBudget(id, group) : nullptr;
    if (d == nullptr) {
        fc4::Log("protection (%s): no department 0x%08X (no budget simulator, or it refused)", Ctx(u)->tag, id);
        return 0;
    }
    const uint64_t t = TokenOf(d);
    AddRefToken(u, t);
    cFC4ISC4DepartmentBudget* g = fc4::AsGame(d);
    g->SetDepartmentName(nameGroup, nameInstance);  // slot 5
    SC4Percentage max;
    max.percentage = maxFunding;
    g->SetMaxAllowedFundingPercentage(max);  // slot 13
    g->CreateLineItem(1, true);              // slot 17
    if (cISC4LineItem* li = g->GetLineItem(1)) {
        li->SetDisplayFlag(static_cast<cISC4LineItem::DisplayFlag>(1), false);  // line item slot 18
    }
    return t;
}

uint32_t DepartmentId(void*, uint64_t dept) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    return d ? d->GetDepartmentID() : 0;  // slot 3
}

float DepartmentFunding(void*, uint64_t dept) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    SC4Percentage* p = d ? d->GetFundingPercentage() : nullptr;  // slot 10
    return p ? p->percentage : 1.0f;
}

void AddLocallyFunded(void*, uint64_t dept, uint64_t occ, uint32_t purpose) {
    if (!Gate(fc4::Sw::ProtectionBudget, "protection AddLocallyFundedObject")) return;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    cISCPropertyHolder* h = HolderOf(occ);
    if (d != nullptr && h != nullptr) d->AddLocallyFundedObject(h, purpose);  // slot 22
}

void RemoveLocallyFunded(void*, uint64_t dept, uint64_t occ, uint32_t purpose) {
    if (!Gate(fc4::Sw::ProtectionBudget, "protection RemoveLocallyFundedObject")) return;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    cISCPropertyHolder* h = HolderOf(occ);
    if (d != nullptr && h != nullptr) d->RemoveLocallyFundedObject(h, purpose);  // slot 23
}

bool SetLocalFunding(void*, uint64_t dept, uint64_t occ, float pct, uint32_t purpose) {
    if (!Gate(fc4::Sw::ProtectionBudget, "protection SetLocalFundingPercent")) return false;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    cISCPropertyHolder* h = HolderOf(occ);
    if (d == nullptr || h == nullptr) return false;
    SC4Percentage p;
    p.percentage = pct;
    return d->SetLocalFundingPercent(h, &p, purpose);  // slot 24 (the percentage by reference)
}

// GetBudgetItemForPurpose (budget slot 51), then the item's department by its
// ID (slot 28, borrowed) and that department's IsLocallyFundedObjectInDepartment
// (§6.1: the department lookup comes first).
bool BudgetItem(void* u, uint64_t occ, uint32_t purpose, fc4_protection_budget_item* out) {
    if (!Gate(fc4::Sw::ProtectionBudget, "protection GetBudgetItemForPurpose (none)")) return false;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    cISCPropertyHolder* h = HolderOf(occ);
    if (b == nullptr || h == nullptr) return false;
    cFC4ISC4BudgetSimulator::BudgetItem item{};
    if (!b->GetBudgetItemForPurpose(h, purpose, item)) return false;
    if (out != nullptr) {
        *out = fc4_protection_budget_item{};
        cISC4DepartmentBudget* d = b->GetDepartmentBudget(item.departmentID);
        out->has_department = d != nullptr ? 1u : 0u;
        out->locally_funded = (d != nullptr && fc4::AsGame(d)->IsLocallyFundedObjectInDepartment(h, purpose)) ? 1u : 0u;
        out->local_funding = item.localFundingPercentage;
        out->cost = item.cost;
        out->department = TokenOf(d);
    }
    return true;
}

// ---- occupants ------------------------------------------------------------------------------------

uint32_t OccupantType(void*, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::ProtectionOccupants, "protection occupant GetType (0)")) return 0;
    return static_cast<uint32_t>(OccOf(occ)->GetType());  // slot 7
}

bool OccupantProperty(void*, uint64_t occ, uint32_t id, fc4_props* out) {
    if (occ == 0 || !Gate(fc4::Sw::ProtectionOccupants, "protection occupant property read (absent)")) return false;
    cISCPropertyHolder* h = HolderOf(occ);  // borrowed, slot 6
    return h != nullptr && fc4::props::CopyToCore(h, id, out);
}

// The game's property helpers (§14): a u32 scalar or a u32 vector, as the
// core says (fc4.h set_occupant_property). PR9: 0x005FE3C0 writes a scalar
// Uint32 (strike start, availability masks, base call numbers, jail
// population), 0x005FE6B0 a Uint32 array of the vector's length (only the
// station statistics 0x8A70A0B9); both add a missing property. [choice] The
// original's lookup falls back to the occupant's exemplar; we always write
// onto the occupant's own holder (no shipped exemplar has these IDs).
void SetOccupantProperty(void*, uint64_t occ, uint32_t id, uint16_t memType, bool isArray, const void* data,
                         size_t count) {
    if (occ == 0 || !Gate(fc4::Sw::ProtectionOccupants, "protection occupant property write")) return;
    if (!Gate(fc4::Sw::PropertyWrites, "protection occupant property write")) return;
    cISCPropertyHolder* h = HolderOf(occ);
    if (h != nullptr && !fc4::props::SetTyped(h, id, memType, isArray, data, count)) {
        fc4::Log("protection: WARNING property 0x%08X (type 0x%02X, %u values) not written", id, memType,
                 static_cast<unsigned>(count));
    }
}

void OccupantPosition(void*, uint64_t occ, float* xyz) {
    if (xyz == nullptr || occ == 0 || !Gate(fc4::Sw::ProtectionOccupants, "protection occupant GetPosition")) {
        return;
    }
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);  // slot 8
    xyz[0] = v.fX;
    xyz[1] = v.fY;
    xyz[2] = v.fZ;
}

bool IsPropOccupant(void*, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::ProtectionOccupants, "protection prop QueryInterface (not a prop)")) return false;
    cIGZUnknown* p = Query<cIGZUnknown>(FromToken(occ), kIID_cISC4PropOccupant);
    Drop(p);
    return p != nullptr;
}

// GetOccupantLot (lot manager slot 14, borrowed): -1 without a lot, else
// HasRoadAccess (lot slot 28).
int32_t LotRoadAccess(void* u, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::ProtectionOccupants, "protection GetOccupantLot (no lot)")) return -1;
    cISC4City* c = City(u);
    cISC4LotManager* lm = c ? c->GetLotManager() : nullptr;  // city slot 33
    cISC4Lot* lot = lm ? fc4::AsGame(lm)->GetOccupantLot(OccOf(occ)) : nullptr;
    if (lot == nullptr) return -1;
    return fc4::AsGame(lot)->HasRoadAccess() ? 1 : 0;
}

// ---- city, simulator, zones, terrain, disasters ---------------------------------------------------

uint32_t SimDay(void* u) {
    cISC4City* c = City(u);
    cFC4ISC4Simulator* sim = c ? fc4::AsGame(c->GetSimulator()) : nullptr;
    cFC4IGZDate* d = sim ? sim->GetSimDate() : nullptr;  // simulator slot 7, borrowed
    return d ? d->DayNumber() : 0;                       // date slot 26
}

// The crime message's IGZUnknown queried for cISC4Lot, then GetLocation
// (lot slot 21) (§9.2).
bool LotLocation(void*, uint64_t unknown, int32_t* x, int32_t* z) {
    if (unknown == 0 || !Gate(fc4::Sw::ProtectionOccupants, "protection crime lot GetLocation (not a lot)")) {
        return false;
    }
    cISC4Lot* lot = Query<cISC4Lot>(FromToken(unknown), kIID_cISC4Lot);
    if (lot == nullptr) return false;
    int32_t a = 0, b = 0;
    fc4::AsGame(lot)->GetLocation(a, b);
    Drop(lot);
    if (x != nullptr) *x = a;
    if (z != nullptr) *z = b;
    return true;
}

// City slot 97 CellCenterToPosition (pop 16). docs/abi (corrected
// 2026-10-14): returns nothing, the results come through the out references,
// as the SDK declares it. Calling it through a float declaration popped an
// empty x87 register on every call (stack underflow). §14 lists only the
// outputs.
void CellCenter(void* u, int32_t cx, int32_t cz, float* x, float* z) {
    float a = 0.0f, b = 0.0f;
    if (cISC4City* c = City(u)) c->CellCenterToPosition(cx, cz, a, b);
    if (x != nullptr) *x = a;
    if (z != nullptr) *z = b;
}

// §5.3 reads the zone manager's zone grid (slot 5, a cSC4SimGrid<int8>) raw;
// GetZoneType (zone manager slot 12, as land value and flammability do)
// reads the same byte of the same grid, sign-extended, so "!= 0" agrees for
// every in-grid cell (PR10). [choice] Outside the city: 0 (unzoned; the
// original has no bounds check there, and no value).
uint32_t ZoneValue(void* u, int32_t x, int32_t z) {
    cISC4City* c = City(u);
    if (c == nullptr || x < 0 || z < 0) return 0;
    if (static_cast<uint32_t>(x) >= c->CellCountX() || static_cast<uint32_t>(z) >= c->CellCountZ()) return 0;
    cISC4ZoneManager* zm = Ctx(u)->zoneManager;  // held (§4.1); the read-only host asks the city
    if (zm == nullptr) zm = c->GetZoneManager();  // city slot 36
    return zm ? static_cast<uint32_t>(zm->GetZoneType(x, z)) : 0;
}

float SeaLevel(void* u) {
    cISC4City* c = City(u);
    cISTETerrain* t = c ? c->GetTerrain() : nullptr;
    return t ? t->GetSeaLevel() : 0.0f;  // terrain slot 29, ST0
}

float Altitude(void* u, float x, float z) {
    cISC4City* c = City(u);
    cISTETerrain* t = c ? c->GetTerrain() : nullptr;
    return t ? t->GetAltitude(x, z) : 0.0f;  // compiled slot 13 (foundations §16), ST0
}

bool DisasterOngoing(void* u) {
    cISC4City* c = City(u);
    cISC4DisasterLayer* l = c ? c->GetDisasterLayer() : nullptr;  // city slot 72
    return l != nullptr && l->IsADisasterOngoing();                // layer slot 10
}

// ---- dispatch manager (§5, §8) --------------------------------------------------------------------

uint32_t DispatchCount(void* u, uint32_t type) {
    if (!Gate(fc4::Sw::ProtectionDispatch, "protection GetDispatchCount (0)")) return 0;
    cFC4ISC4DispatchManager* dm = DispatchManager(u);
    return dm ? dm->GetDispatchCount(static_cast<int32_t>(type)) : 0;  // slot 11
}

// GetDispatchIDArray (slot 12): the count is the capacity (never 0 here:
// 0 means "no limit", docs/abi allocators.md); EAX = the number written.
uint32_t DispatchIds(void* u, uint32_t type, uint32_t* ids, uint32_t capacity) {
    if (ids == nullptr || capacity == 0) return 0;
    if (!Gate(fc4::Sw::ProtectionDispatch, "protection GetDispatchIDArray (none)")) return 0;
    cFC4ISC4DispatchManager* dm = DispatchManager(u);
    if (dm == nullptr) return 0;
    uint32_t cap = capacity;
    const uint32_t n = dm->GetDispatchIDArray(static_cast<int32_t>(type), ids, cap);
    return n < capacity ? n : capacity;
}

// GetDispatchInfo (slot 10) into the record of docs/abi layouts.md (no
// references taken). §5 (PR1): the call ID is the record's +0x08 field and
// "active" its +0x10 byte. The station (+0x20, already a cISC4Occupant*) is
// the occupant whose dispatch effect the core reads (§7.2, PR2), borrowed
// with no QueryInterface, as the original does; the vehicle automaton (+0x24)
// is not used.
bool DispatchInfo(void* u, uint32_t id, fc4_protection_dispatch_info* out) {
    if (!Gate(fc4::Sw::ProtectionDispatch, "protection GetDispatchInfo (none)")) return false;
    cFC4ISC4DispatchManager* dm = DispatchManager(u);
    if (dm == nullptr) return false;
    FC4DispatchInfo info{};
    if (!dm->GetDispatchInfo(id, info)) return false;
    if (out != nullptr) {
        *out = fc4_protection_dispatch_info{};
        out->station = TokenOf(info.station);
        out->call_id = info.callId;
        out->active = info.active ? 1u : 0u;
        out->x = info.x;
        out->z = info.z;
    }
    return true;
}

// ---- automata controllers (§6.3, §9.5) ------------------------------------------------------------

// [abi] AddControllersFromOccupantGroup(name, occ, 0) is compiled slot 10 and
// Remove... slot 12 (overloads in reverse order, foundations §16): the SDK
// declarations give the same slots (abi_conformance_test); the by-name
// overloads (PR3). The game's group lookup reads the name raw as its
// cRZString (begin pointer at +0x04, end at +0x08, no virtual call; docs/abi
// layouts.md "Game string"): the SDK SC4String has exactly that layout
// (0x14 bytes, always a heap buffer; game_container_test checks it).
void Controllers(void* u, const char* group, size_t len, uint64_t occ, bool add) {
    if (group == nullptr || occ == 0) return;
    if (!Gate(fc4::Sw::ProtectionAutomata, add ? "protection AddControllersFromOccupantGroup"
                                                : "protection RemoveControllersFromOccupantGroup")) {
        return;
    }
    cISC4City* c = City(u);
    cISC4AutomataControllerManager* m = c ? c->GetAutomataControllerManager() : nullptr;  // city slot 47
    if (m == nullptr) return;
    const SC4String name(group, static_cast<uint32_t>(len));
    if (add) {
        m->AddControllersFromOccupantGroup(name, OccOf(occ), nullptr);
    } else {
        m->RemoveControllersFromOccupantGroup(name, OccOf(occ));
    }
    ++Ctx(u)->controllers;
}
void AddControllers(void* u, const char* g, size_t n, uint64_t occ) { Controllers(u, g, n, occ, true); }
void RemoveControllers(void* u, const char* g, size_t n, uint64_t occ) { Controllers(u, g, n, occ, false); }

// ---- other simulators, history, messages ----------------------------------------------------------

// The fire end of cycle (§5.4): the city's police simulator (city slot 65),
// GetPercentCoverage (police slot 19, a pointer) through the verified slot
// order (ours when the protection subsystem is on).
bool PolicePercentCoverage(void* u, float* out) {
    cISC4City* c = City(u);
    cISC4PoliceSimulator* p = c ? c->GetPoliceSimulator() : nullptr;
    if (p == nullptr) return false;
    SC4Percentage const* v = fc4::AsGame(p)->GetPercentCoverage();
    if (out != nullptr) *out = v ? v->percentage : 0.0f;
    return true;
}

void RecordHistory(void* u, uint32_t id, int32_t value) {
    if (!Gate(fc4::Sw::ProtectionHistory, "protection RecordHistoryData")) return;
    cISC4City* c = City(u);
    cISC4HistoryWarehouse* h = c ? c->GetHistoryWarehouse() : nullptr;
    if (h != nullptr) h->RecordHistoryData(id, value);  // slot 12
}

// MessagePost(msg, false) of a standard message (§12):
//   cycle end            void1 = IGZUnknown = the simulator
//   strike begin         void1 = IGZUnknown = the station
//   strike end           nothing
//   jailbreak begin/end  void1 = the simulator, IGZUnknown = the jail
//   0xA949569A           IGZUnknown = the simulator
// "the simulator" is its primary interface (the identity QueryInterface hands
// out for cIGZUnknown; PR11 confirms it: the original posts the primary,
// vtables 0x00AB1160 / 0x00AB2490). void1 is stored raw, the IGZUnknown is a
// counted reference.
void Post(void* u, uint32_t msgType, uint64_t subject) {
    fc4::ProtectionContext* c = Ctx(u);
    if (!Gate(fc4::Sw::ProtectionPosts, "protection message post")) return;
    cIGZMessageServer2* ms = c->msgServer;
    cIGZMessageServer2Ptr fallback;
    if (ms == nullptr) ms = fallback;
    if (ms == nullptr) return;
    cFC4Message2Standard* msg = FC4MakeMessage(msgType, 0, 0, 0);
    cIGZUnknown* self = FromToken(c->selfToken);
    cIGZUnknown* subj = FromToken(subject);
    switch (msgType) {
        case kMsgFireCycleEnd:
        case kMsgPoliceCycleEnd:
            msg->SetVoid1(self);
            msg->SetIGZUnknown(self);
            break;
        case kMsgFireStrikeBegin:
        case kMsgPoliceStrikeBegin:
            msg->SetVoid1(subj);
            msg->SetIGZUnknown(subj);
            break;
        case kMsgJailbreakBegin:
        case kMsgJailbreakEnd:
            msg->SetVoid1(self);
            msg->SetIGZUnknown(subj);
            break;
        case kMsgPoliceStationsChanged:
            msg->SetIGZUnknown(self);
            break;
        default:  // the strike-end messages carry nothing
            break;
    }
    ms->MessagePost(msg, false);
    msg->Release();
    ++c->posts;
}

// ---- air dispatch sources (§8.4) ------------------------------------------------------------------

// §8.4 / §13.2 (PR13): the original classes are created through GZCOM (the
// game registers both factories; docs/abi/direct-calls.md), which build the
// same object the original's Init does (GlobalHeapAlloc(0x28) and the same
// constructor). We ask for the dispatch source (0xC9C00435), the object base
// that Init / Shutdown need in ECX, with one reference (the sources refuse
// IID 1); their non-virtual Init / Shutdown are called by address. [choice]
// Without an identified executable no source is created (a later Init could
// not be called).
uint64_t CreateAirSource(void* u, uint32_t kind, uint32_t* classId) {
    fc4::ProtectionContext* c = Ctx(u);
    if (!Gate(fc4::Sw::ProtectionAirSources, "protection air dispatch source (none)")) return 0;
    if (fc4::compat::DirectCall(fc4::compat::Direct::AirDispatchInit) == 0) {
        fc4::Log("protection (%s): no air dispatch source (its Init cannot be called on this executable)", c->tag);
        return 0;
    }
    const uint32_t clsid = kAirSourceClsid[kind & 1];
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (com == nullptr || !com->GetClassObject(clsid, kFC4_IID_cISC4DispatchSource, &p) || p == nullptr) {
        fc4::Log("protection (%s): WARNING could not create the air dispatch source 0x%08X through GZCOM", c->tag,
                 clsid);
        return 0;
    }
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(p), "GetClassObject");
    if (classId != nullptr) *classId = clsid;
    c->airToken = TokenOf(p);
    fc4::Log("protection (%s): air dispatch source 0x%08X created (%p)", c->tag, clsid, p);
    return c->airToken;
}

// Both functions: __thiscall on the object base, no arguments, pop 0, AL.
using AirSourceFn = bool(__fastcall*)(void* self, void* edx);

void AirSourceCall(void* u, uint64_t source, fc4::compat::Direct which, const char* what) {
    if (source == 0 || !Gate(fc4::Sw::ProtectionAirSources, what)) return;
    const uintptr_t va = fc4::compat::DirectCall(which);
    if (va == 0) return;  // logged once by DirectCall
    reinterpret_cast<AirSourceFn>(va)(FromToken(source), nullptr);
    if (which == fc4::compat::Direct::AirDispatchInit) ++Ctx(u)->airInits;
}

void AirSourceInit(void* u, uint64_t s) {
    Ctx(u)->airToken = s;  // also a source read back from a save
    AirSourceCall(u, s, fc4::compat::Direct::AirDispatchInit, "protection air source Init");
}
void AirSourceShutdown(void* u, uint64_t s) {
    AirSourceCall(u, s, fc4::compat::Direct::AirDispatchShutdown, "protection air source Shutdown");
}

// ---- lists ----------------------------------------------------------------------------------------

void CollectToken(void* ctx, uint64_t t) { static_cast<std::vector<uint64_t>*>(ctx)->push_back(t); }

// Empties the caller's ilist (A) as the original does (docs/abi
// allocators.md, GetFireStations): the nodes are freed with A WITHOUT
// releasing the elements. An empty list needs no allocator. PR8 (§15.4, §16
// quirk 7): reproduced; every original caller passes a fresh empty list, so
// only a plug-in passing a non-empty list leaks its references, and
// releasing instead would over-release a list of borrowed pointers.
void ClearKeepingElements(SC4List<cISC4Occupant*>& list) {
    SC4ListNode<cISC4Occupant*>* root = list.end().pNode;
    if (root == nullptr || root->next == root) return;
    cIGZAllocatorServicePtr a;
    SC4ListNode<cISC4Occupant*>* n = root->next;
    while (n != root) {
        SC4ListNode<cISC4Occupant*>* next = n->next;
        if (a) a->Deallocate(n);
        n = next;
    }
    root->next = root;
    root->prev = root;
}

// Appends each (borrowed) token with one reference for the caller.
void AppendForCaller(SC4List<cISC4Occupant*>& list, const std::vector<uint64_t>& tokens) {
    for (uint64_t t : tokens) {
        cISC4Occupant* o = OccOf(t);
        if (o == nullptr) continue;
        FC4_ADDREF_FOR_CALLER(o);
        list.push_back(o);
    }
}

float FromBits(uint32_t b) {
    float f = 0.0f;
    std::memcpy(&f, &b, sizeof(f));
    return f;
}

uint32_t ToBits(float f) {
    uint32_t b = 0;
    std::memcpy(&b, &f, sizeof(b));
    return b;
}

}  // namespace

// ---- host tables ----------------------------------------------------------------------------------

namespace fc4 {

fc4_protection_host MakeProtectionReadOnlyHost(ProtectionContext* ctx) {
    fc4_protection_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.tuning = &Tuning;
    h.department_id = &DepartmentId;
    h.department_funding = &DepartmentFunding;
    h.budget_item = &BudgetItem;
    h.occupant_type = &OccupantType;
    h.occupant_property = &OccupantProperty;
    h.occupant_position = &OccupantPosition;
    h.is_prop_occupant = &IsPropOccupant;
    h.lot_road_access = &LotRoadAccess;
    h.sim_day = &SimDay;
    h.lot_location = &LotLocation;
    h.cell_center_to_position = &CellCenter;
    h.zone_value = &ZoneValue;
    h.sea_level = &SeaLevel;
    h.altitude = &Altitude;
    h.disaster_ongoing = &DisasterOngoing;
    h.dispatch_count = &DispatchCount;
    h.dispatch_ids = &DispatchIds;
    h.dispatch_info = &DispatchInfo;
    h.police_percent_coverage = &PolicePercentCoverage;
    return h;
}

fc4_protection_host MakeProtectionHost(ProtectionContext* ctx, const HostGridContext* grids) {
    fc4_protection_host h = MakeProtectionReadOnlyHost(ctx);
    h.grids = MakeGridApi(grids);
    // lifecycle
    h.acquire_services = &AcquireServices;
    h.release_services = &ReleaseServices;
    h.add_agent = &AddAgent;
    h.remove_agent = &RemoveAgent;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.register_dispatch_source = &RegisterDispatchSource;
    h.unregister_dispatch_source = &UnregisterDispatchSource;
    h.set_grid_instance = &SetGridInstance;
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    // budget writes
    h.create_department = &CreateDepartment;
    h.add_locally_funded = &AddLocallyFunded;
    h.remove_locally_funded = &RemoveLocallyFunded;
    h.set_local_funding = &SetLocalFunding;
    // occupants
    h.set_occupant_property = &SetOccupantProperty;
    // automata, history, messages, air sources
    h.add_controllers = &AddControllers;
    h.remove_controllers = &RemoveControllers;
    h.record_history = &RecordHistory;
    h.post = &Post;
    h.create_air_source = &CreateAirSource;
    h.air_source_init = &AirSourceInit;
    h.air_source_shutdown = &AirSourceShutdown;
    return h;
}

}  // namespace fc4

// ---- object ---------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4FireProtectionSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4FireProtectionSimulator*>(new cFC4FireProtectionSimulator()));
}

cIGZUnknown* CreateFC4PoliceSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4PoliceSimulator*>(new cFC4PoliceSimulator()));
}

cFC4FireProtectionSimulator::cFC4FireProtectionSimulator() : cFC4ProtectionBase(0) {
    Construct(static_cast<cIGZUnknown*>(static_cast<cFC4ISC4FireProtectionSimulator*>(this)));
}

cFC4PoliceSimulator::cFC4PoliceSimulator() : cFC4ProtectionBase(1) {
    Construct(static_cast<cIGZUnknown*>(static_cast<cFC4ISC4PoliceSimulator*>(this)));
}

// §1: at a count of 1 (or below) Shutdown, destroy, return 0 (the count is
// not decremented first).
uint32_t cFC4FireProtectionSimulator::Release() {
    uint32_t remaining = 0;
    if (!ReleaseIsFinal(&remaining)) return remaining;
    Shutdown();
    delete this;
    return 0;
}

uint32_t cFC4PoliceSimulator::Release() {
    uint32_t remaining = 0;
    if (!ReleaseIsFinal(&remaining)) return remaining;
    Shutdown();
    delete this;
    return 0;
}

cFC4ProtectionBase::cFC4ProtectionBase(uint32_t kind)
    : kind_(kind & 1), gridCtx_{kTag[kind & 1], fc4::Sw::ProtectionGridWrites} {
    InitializeCriticalSection(&lock_);
    mainTid_ = GetCurrentThreadId();  // the class store creates simulators on the main thread
    ctx_.kind = kind_;
    ctx_.tag = kTag[kind_];
}

void cFC4ProtectionBase::Construct(cIGZUnknown* primary) {
    primary_ = primary;
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    ctx_.source = static_cast<cFC4ISC4DispatchSource*>(this);
    ctx_.selfToken = TokenOf(primary);
    host_ = fc4::MakeProtectionHost(&ctx_, &gridCtx_);
    // The object's own cRZRandom is seeded from the clock at construction
    // (§10): the wall-clock and process-clock values.
    core_ = fc4_protection_create(&host_, kind_, ctx_.selfToken,
                                  static_cast<uint32_t>(GetTickCount64() ^ GetCurrentProcessId()),
                                  static_cast<uint32_t>(GetTickCount()));
    if (core_ == nullptr) fc4::Log("cFC4ProtectionBase (%s): core creation failed", ctx_.tag);
    fc4::Log("cFC4%sSimulator(%p) created, core %p", kind_ == 0 ? "FireProtection" : "Police",
             static_cast<void*>(primary), static_cast<void*>(core_));
}

cFC4ProtectionBase::~cFC4ProtectionBase() {
    fc4_protection_destroy(core_);  // releases what a loaded, never-initialised object still holds
    core_ = nullptr;
    ctx_.held.clear();
    fc4::Log("cFC4%sSimulator destroyed (%u fallbacks)", kind_ == 0 ? "FireProtection" : "Police",
             fallbacks_.load());
    DeleteCriticalSection(&lock_);
}

bool cFC4ProtectionBase::QueryImpl(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    // §1: which sub-object answers is the core's decision; anything else is
    // false, and the out pointer is left untouched.
    switch (fc4_protection_query_interface(kind_, riid)) {
        case 1: *ppvObj = primary_; break;
        case 2: *ppvObj = static_cast<cIGZMessageTarget2*>(this); break;
        case 3: *ppvObj = static_cast<cFC4ISC4DispatchSource*>(this); break;
        case 4: *ppvObj = static_cast<cIGZSerializable*>(this); break;
        default: return false;
    }
    ++refCount_;
    return true;
}

bool cFC4ProtectionBase::ReleaseIsFinal(uint32_t* remaining) {
    if (refCount_ > 1) {
        *remaining = --refCount_;
        return false;
    }
    *remaining = 0;
    if (destroying_) return false;  // a release from inside the final Shutdown
    destroying_ = true;
    refCount_ = 0;
    return true;
}

// ---- the lock -------------------------------------------------------------------------------------

bool cFC4ProtectionBase::Enter() {
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

void cFC4ProtectionBase::Leave() { LeaveCriticalSection(&lock_); }

cFC4ProtectionBase::Section::Section(cFC4ProtectionBase* s) : s_(s), ok_(s->Enter()) {
    if (ok_) ++s_->depth_;
}

cFC4ProtectionBase::Section::~Section() {
    if (!ok_) return;
    if (--s_->depth_ == 0) {
        // The outermost call: the objects kept for queued messages go, and
        // the values other threads fall back to are refreshed.
        s_->ctx_.held.clear();
        s_->RefreshSnapshot();
    }
    s_->Leave();
}

// Only the core's own state (no host calls): cheap, any thread holding the lock.
void cFC4ProtectionBase::RefreshSnapshot() {
    if (core_ == nullptr) return;
    snap_.grid = fc4_protection_grid(core_);
    snap_.department = fc4_protection_department(core_);
    snap_.avgBits = ToBits(fc4_protection_average_coverage(core_));
    const float pct = fc4_protection_percent_coverage(core_);
    snap_.pctBits = ToBits(pct);
    pct_.percentage = pct;
    snap_.stations = fc4_protection_station_count(core_);
    snap_.onStrike = fc4_protection_is_on_strike(core_) ? 1u : 0u;
    snap_.strikeDate = fc4_protection_strike_start_date(core_);
    if (kind_ == 1) {
        snap_.jailDepartment = fc4_protection_jail_department(core_);
        snap_.jailCapacity = fc4_protection_jail_capacity(core_);
        snap_.jailPopulation = fc4_protection_jail_population(core_);
        snap_.criminals = fc4_protection_criminal_count(core_);
        snap_.arrests = fc4_protection_arrest_count(core_);
        snap_.jails = fc4_protection_jail_count(core_);
        snap_.jailedPctBits = ToBits(fc4_protection_jailed_percentage(core_));
    }
}

void cFC4ProtectionBase::Hold(cIGZUnknown* object) {
    if (object == nullptr) return;
    FC4_ADDREF(object);  // a game object (the message's occupant or lot): audited both ways
    ctx_.held.push_back(std::shared_ptr<void>(object, [](void* p) { FC4_RELEASE(static_cast<cIGZUnknown*>(p)); }));
}

// The month status line (and Init / Read). Main thread, inside a section.
void cFC4ProtectionBase::LogState(const char* what) {
    fc4_protection_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_protection_get_state(core_, &st)) return;
    const uint32_t active = DispatchCount(&ctx_, kDispatchType[kind_]);
    const int32_t units = fc4_protection_total_dispatch_count(core_);
    if (st.air_source != 0) ctx_.airToken = st.air_source;
    fc4::Log("protection %s (%s): init=%u loaded=%u tracts=%dx%d stations=%u coveredCells=%u maxCoverage=%u "
             "coverageSum=%u avg=%.3f pct=%.4f zoned=%d sum=%d nonZero=%d tractsPerDay=%d dispatches=%u units=%d "
             "strike=%u strikeDate=%u nextCall=%u department=%p grid=%p unadjusted=%p air=%p reentrant=%u "
             "fallbacks=%u posts=%u airInits=%u controllers=%u months=%u days=%u",
             what, ctx_.tag, st.initialized, st.loaded, st.count_x, st.count_z, st.stations, st.covered_tracts,
             st.max_coverage, st.coverage_sum, st.avg, st.pct, st.zoned, st.sum, st.non_zero, st.tracts_per_day,
             active, units, st.strike_flag, st.strike_date, st.next_call, static_cast<void*>(FromToken(st.department)),
             static_cast<void*>(FromToken(st.adjusted_grid)), static_cast<void*>(FromToken(st.unadjusted_grid)),
             static_cast<void*>(FromToken(st.air_source)), st.reentrant_calls, fallbacks_.load(), ctx_.posts,
             ctx_.airInits, ctx_.controllers, months_, days_);
    // A month with a coverage grid that has no tracts: the daily cycle (§5.3)
    // cannot run, and the grid is never updated. `Init` takes the geometry
    // from the grid (§4.1 base init steps 2 and 6). At `Read` a grid can still
    // be 0 x 0, because the game reads its record later, but never at `Init`
    // (HG1, foundations §3.7): every saved object is read in ascending class
    // ID before cSC4City::Init, fire (0x2990C05A) before the grids
    // (0x49B9E602) and police (0x6990C04F) after them. So a 0 x 0 grid at a
    // month is empty in itself, such as a grid saved as 0 x 0 (it stays empty
    // after its Read).
    if (!emptyGridWarned_ && std::strcmp(what, "month") == 0 && st.adjusted_grid != 0 &&
        (st.count_x <= 0 || st.count_z <= 0)) {
        emptyGridWarned_ = true;
        fc4::Log("protection (%s): WARNING the coverage grid %p has %dx%d tracts at a month (init=%u "
                 "tractsPerDay=%d): no daily cycle runs (logged once)",
                 ctx_.tag, static_cast<void*>(FromToken(st.adjusted_grid)), st.count_x, st.count_z, st.initialized,
                 st.tracts_per_day);
    }
    if (kind_ == 1) {
        fc4::Log("protection %s (police jails): jails=%u funded=%u jailbreaks=%u capacity=%d population=%d jailed=%d "
                 "arrestsLast=%d crimesLast=%d jailDepartment=%p",
                 what, st.jails, st.funded_jails, st.jailbreaks, st.jail_capacity, st.jail_population, st.jailed,
                 st.arrests_last, st.crimes_last, static_cast<void*>(FromToken(st.jail_department)));
    }
}

// ---- lifecycle (§4) -------------------------------------------------------------------------------

bool cFC4ProtectionBase::InitImpl() {
    FC4_TRACE(kind_ == 0 ? "FireProtectionSimulator::Init" : "PoliceSimulator::Init");
    mainTid_ = GetCurrentThreadId();
    Section sec(this);
    fc4_protection_init(core_);  // §4.1, all of it (the core drives every host call)
    LogState("Init");
    return true;  // §4.1: always
}

bool cFC4ProtectionBase::ShutdownImpl() {
    FC4_TRACE(kind_ == 0 ? "FireProtectionSimulator::Shutdown" : "PoliceSimulator::Shutdown");
    // Keep this object alive while the simulator and the message server drop
    // their references (unless this already is the final Release).
    if (!destroying_) primary_->AddRef();
    {
        Section sec(this);
        fc4_protection_shutdown(core_);  // §4.2
    }
    fc4::Log("protection Shutdown (%s): %u months, %u days, %u fallbacks", ctx_.tag, months_, days_,
             fallbacks_.load());
    if (!destroying_) primary_->Release();
    return true;  // §4.2: always
}

// ---- primary slots (§15.1, §15.2) -----------------------------------------------------------------

cISC4SimGrid<uint8_t>* cFC4ProtectionBase::GridImpl() {
    Section sec(this);
    const uint64_t g = sec.ok() ? fc4_protection_grid(core_) : snap_.grid.load();
    return reinterpret_cast<GridU8*>(FromToken(g));  // slot 5, no AddRef
}

cISC4DepartmentBudget* cFC4ProtectionBase::DepartmentImpl() {
    Section sec(this);
    const uint64_t d = sec.ok() ? fc4_protection_department(core_) : snap_.department.load();
    return reinterpret_cast<cISC4DepartmentBudget*>(FromToken(d));  // slot 6, borrowed
}

bool cFC4ProtectionBase::IsStationImpl(cISC4Occupant* o) {
    Section sec(this);
    return sec.ok() && fc4_protection_is_station(core_, TokenOf(o));
}

bool cFC4ProtectionBase::GetStationFundingImpl(cISC4Occupant* o, int32_t* cost, SC4Percentage* funding) {
    Section sec(this);
    return sec.ok() &&
           fc4_protection_station_funding(core_, TokenOf(o), cost, funding ? &funding->percentage : nullptr);
}

bool cFC4ProtectionBase::SetStationFundingImpl(cISC4Occupant* o, SC4Percentage const& funding) {
    Section sec(this);
    return sec.ok() && fc4_protection_set_station_funding(core_, TokenOf(o), funding.percentage);
}

// By-value SC4Percentage through the hidden result pointer (§15).
SC4Percentage* cFC4ProtectionBase::StationEfficiencyImpl(SC4Percentage* result, cISC4Occupant* o) {
    Section sec(this);
    const float v = sec.ok() ? fc4_protection_station_efficiency(core_, TokenOf(o)) : 0.0f;
    if (result != nullptr) result->percentage = v;
    return result;
}

bool cFC4ProtectionBase::IsOnStrikeImpl() {
    Section sec(this);
    return sec.ok() ? fc4_protection_is_on_strike(core_) : snap_.onStrike.load() != 0;
}

SC4Percentage* cFC4ProtectionBase::ChanceOfStrikeImpl(SC4Percentage* result) {
    Section sec(this);
    const float v = sec.ok() ? fc4_protection_chance_of_strike(core_) : 0.0f;
    if (result != nullptr) result->percentage = v;
    return result;
}

bool cFC4ProtectionBase::BeginStrikeImpl(cISC4Occupant* s) {
    Section sec(this);
    return sec.ok() && fc4_protection_begin_strike(core_, TokenOf(s));
}

bool cFC4ProtectionBase::EndStrikeImpl() {
    Section sec(this);
    return sec.ok() && fc4_protection_end_strike(core_);
}

uint32_t cFC4ProtectionBase::StrikeStartDateImpl() {
    Section sec(this);
    return sec.ok() ? fc4_protection_strike_start_date(core_) : snap_.strikeDate.load();
}

// The fallback has no clock: 0xFFFFFFFF without a strike date (as the
// original), else 0.
uint32_t cFC4ProtectionBase::StrikeLengthImpl() {
    Section sec(this);
    if (sec.ok()) return fc4_protection_strike_length(core_);
    return snap_.strikeDate.load() == 0 ? 0xFFFFFFFFu : 0u;
}

// Slot 17 / 16: appends (no clear), one reference each; the number appended.
int32_t cFC4ProtectionBase::StrikingStationsImpl(SC4List<cISC4Occupant*>& list) {
    Section sec(this);
    if (!sec.ok()) return 0;
    std::vector<uint64_t> tokens;
    const int32_t n = fc4_protection_striking_stations(core_, &CollectToken, &tokens);
    AppendForCaller(list, tokens);
    return n;
}

bool cFC4ProtectionBase::IsStationOnStrikeImpl(cISC4Occupant* s) {
    Section sec(this);
    return sec.ok() && fc4_protection_is_station_on_strike(core_, TokenOf(s));
}

float cFC4ProtectionBase::AverageCoverageImpl() {
    Section sec(this);
    return sec.ok() ? fc4_protection_average_coverage(core_) : FromBits(snap_.avgBits.load());
}

// Slot 20 / 19: a pointer to a member that stays valid for the object's life.
SC4Percentage const* cFC4ProtectionBase::PercentCoverageImpl() {
    Section sec(this);
    if (sec.ok()) pct_.percentage = fc4_protection_percent_coverage(core_);
    return &pct_;
}

bool cFC4ProtectionBase::OccupantCoverageImpl(cISC4Occupant* s, SC4Percentage const& p, float& coverRadius,
                                              float& stationRadius) {
    Section sec(this);
    if (!sec.ok()) {
        if (kind_ == 0) coverRadius = stationRadius = 0.0f;  // fire zeroes both outputs first (§7.3)
        return false;
    }
    return fc4_protection_occupant_coverage(core_, TokenOf(s), p.percentage, &coverRadius, &stationRadius);
}

// Slot 22 / 21: empties the caller's list (nodes freed, elements NOT
// released), then appends one reference each; the count in EAX.
int32_t cFC4ProtectionBase::StationsImpl(SC4List<cISC4Occupant*>& list) {
    Section sec(this);
    if (!sec.ok()) return 0;  // the caller's list untouched
    std::vector<uint64_t> tokens;
    fc4_protection_stations(core_, &CollectToken, &tokens);
    ClearKeepingElements(list);
    AppendForCaller(list, tokens);
    return static_cast<int32_t>(tokens.size());
}

uint32_t cFC4ProtectionBase::StationCountImpl() {
    Section sec(this);
    return sec.ok() ? fc4_protection_station_count(core_) : snap_.stations.load();
}

float cFC4ProtectionBase::StationDispatchRadiusImpl(cISC4Occupant* s) {
    Section sec(this);
    return sec.ok() ? fc4_protection_station_dispatch_radius(core_, TokenOf(s)) : 0.0f;
}

int32_t cFC4ProtectionBase::TotalDispatchCountImpl() {
    Section sec(this);
    return sec.ok() ? fc4_protection_total_dispatch_count(core_) : 0;
}

// ---- police jails (§9, §15.2) ---------------------------------------------------------------------

cISC4DepartmentBudget* cFC4ProtectionBase::JailDepartmentImpl() {
    Section sec(this);
    const uint64_t d = sec.ok() ? fc4_protection_jail_department(core_) : snap_.jailDepartment.load();
    return reinterpret_cast<cISC4DepartmentBudget*>(FromToken(d));
}

int32_t cFC4ProtectionBase::ResetUnjailedCriminalsImpl() {
    Section sec(this);
    return sec.ok() ? fc4_protection_reset_unjailed_criminals(core_) : 0;
}

uint32_t cFC4ProtectionBase::JailCapacityOfImpl(cISC4Occupant* j) {
    Section sec(this);
    return sec.ok() ? fc4_protection_jail_capacity_of(core_, TokenOf(j)) : 0;
}

uint32_t cFC4ProtectionBase::JailCapacityImpl() {
    Section sec(this);
    return sec.ok() ? fc4_protection_jail_capacity(core_) : snap_.jailCapacity.load();
}

uint32_t cFC4ProtectionBase::JailPopulationImpl() {
    Section sec(this);
    return sec.ok() ? fc4_protection_jail_population(core_) : snap_.jailPopulation.load();
}

uint32_t cFC4ProtectionBase::CriminalCountImpl() {
    Section sec(this);
    return sec.ok() ? fc4_protection_criminal_count(core_) : snap_.criminals.load();
}

// Slot 32: as slot 21 (clear without releasing, then copy).
int32_t cFC4ProtectionBase::JailsImpl(SC4List<cISC4Occupant*>& list) {
    Section sec(this);
    if (!sec.ok()) return 0;
    std::vector<uint64_t> tokens;
    fc4_protection_jails(core_, &CollectToken, &tokens);
    ClearKeepingElements(list);
    AppendForCaller(list, tokens);
    return static_cast<int32_t>(tokens.size());
}

bool cFC4ProtectionBase::IsJailImpl(cISC4Occupant* o) {
    Section sec(this);
    return sec.ok() && fc4_protection_is_jail(core_, TokenOf(o));
}

uint32_t cFC4ProtectionBase::ArrestCountImpl() {
    Section sec(this);
    return sec.ok() ? fc4_protection_arrest_count(core_) : snap_.arrests.load();
}

uint32_t cFC4ProtectionBase::JailCountImpl() {
    Section sec(this);
    return sec.ok() ? fc4_protection_jail_count(core_) : snap_.jails.load();
}

SC4Percentage* cFC4ProtectionBase::JailedPercentageImpl(SC4Percentage* result) {
    Section sec(this);
    const float v = sec.ok() ? fc4_protection_jailed_percentage(core_) : FromBits(snap_.jailedPctBits.load());
    if (result != nullptr) result->percentage = v;
    return result;
}

int64_t cFC4ProtectionBase::JailUpkeepCostImpl(cISC4Occupant* j) {
    Section sec(this);
    return sec.ok() ? fc4_protection_jail_upkeep_cost(core_, TokenOf(j)) : 0;
}

// Slot 38: the original's return value is meaningless; [choice] true (§15.2).
bool cFC4ProtectionBase::ProcessMonthlyCrimeStatsImpl() {
    Section sec(this);
    if (sec.ok()) fc4_protection_process_monthly_crime_stats(core_);
    return true;
}

bool cFC4ProtectionBase::IsInJailbreakImpl(cISC4Occupant* j) {
    Section sec(this);
    return sec.ok() && fc4_protection_is_in_jailbreak(core_, TokenOf(j));
}

bool cFC4ProtectionBase::BeginJailbreakImpl(cISC4Occupant* j) {
    Section sec(this);
    return sec.ok() && fc4_protection_begin_jailbreak(core_, TokenOf(j));
}

bool cFC4ProtectionBase::EndJailbreakImpl(cISC4Occupant* j) {
    Section sec(this);
    return sec.ok() && fc4_protection_end_jailbreak(core_, TokenOf(j));
}

// Slot 42: 0 and the caller's list untouched when the jailbreak list is
// empty; else as slot 21.
int32_t cFC4ProtectionBase::JailbreakBuildingsImpl(SC4List<cISC4Occupant*>& list) {
    Section sec(this);
    if (!sec.ok()) return 0;
    std::vector<uint64_t> tokens;
    fc4_protection_jailbreak_buildings(core_, &CollectToken, &tokens);
    if (tokens.empty()) return 0;
    ClearKeepingElements(list);
    AppendForCaller(list, tokens);
    return static_cast<int32_t>(tokens.size());
}

uint32_t cFC4ProtectionBase::StationArrestCountImpl(cISC4Occupant* s) {
    Section sec(this);
    return sec.ok() ? fc4_protection_station_arrest_count(core_, TokenOf(s)) : 0;
}

uint32_t cFC4ProtectionBase::StationCrimeCountImpl(cISC4Occupant* s) {
    Section sec(this);
    return sec.ok() ? fc4_protection_station_crime_count(core_, TokenOf(s)) : 0;
}

// ---- messages (§5) --------------------------------------------------------------------------------

bool cFC4ProtectionBase::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("protection DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    Section sec(this);
    if (!sec.ok()) return true;  // [choice] a message from another thread that cannot get the lock is dropped
    fc4_protection_message m{};
    m.msg_type = pMessage->GetType();
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    cISC4Occupant* occ = nullptr;
    cIGZUnknown* unknown = nullptr;  // borrowed from the message
    if (s != nullptr) {
        m.data1 = static_cast<uint32_t>(s->GetData1());
        m.data2 = static_cast<uint32_t>(s->GetData2());
        if (m.msg_type == kMsgInsertOccupant || m.msg_type == kMsgRemoveOccupant ||
            m.msg_type == kMsgOccupantChanged) {
            // §5: the occupant is void1, known by its cISC4Occupant identity.
            occ = Query<cISC4Occupant>(static_cast<cIGZUnknown*>(s->GetVoid1()), kIID_cISC4Occupant);
            m.occupant = TokenOf(occ);
        } else if (m.msg_type == kMsgCrime) {
            // §9.2: GetIGZUnknown() (the lot, queried by the core's lot_location).
            unknown = s->GetIGZUnknown();
            m.unknown = TokenOf(unknown);
        }
    }
    // A message delivered while another call into the core is running is
    // queued by the core and handled when that call returns: keep its objects
    // alive until then.
    if (depth_ > 1) {
        Hold(occ);
        Hold(unknown);
    }
    const bool r = fc4_protection_do_message(core_, &m);
    Drop(occ);
    Drop(s);
    if (m.msg_type == kMsgSimNewDay) ++days_;
    if (m.msg_type == kMsgSimNewMonth) {
        ++months_;
        if (depth_ == 1) LogState("month");
    }
    return r;  // §5: always true
}

// ---- persistence (§11) ----------------------------------------------------------------------------

bool cFC4ProtectionBase::Write(cIGZOStream& s) {
    FC4_TRACE(kind_ == 0 ? "FireProtectionSimulator::Write" : "PoliceSimulator::Write");
    Section sec(this);
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_protection_write(core_, a.Get());
    fc4::Log("protection Write (%s) -> %d", ctx_.tag, ok ? 1 : 0);
    return ok;
}

// Runs before Init: the core sizes nothing from the city here.
bool cFC4ProtectionBase::Read(cIGZIStream& s) {
    FC4_TRACE(kind_ == 0 ? "FireProtectionSimulator::Read" : "PoliceSimulator::Read");
    Section sec(this);
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_protection_read(core_, a.Get());
    fc4::Log("protection Read (%s) -> %d", ctx_.tag, ok ? 1 : 0);
    LogState("Read");
    return ok;
}

uint32_t cFC4ProtectionBase::GetGZCLSID() {
    return kind_ == 0 ? kFC4_CLSID_cSC4FireProtectionSimulator : kFC4_CLSID_cSC4PoliceSimulator;
}

// ---- the dispatch source (§8, §15.4) --------------------------------------------------------------

// Slot 3: the whole of EAX is 0 or 1.
uint32_t cFC4ProtectionBase::CanHandleDispatchType(uint32_t type) {
    Section sec(this);
    if (sec.ok()) return fc4_protection_can_handle_dispatch_type(core_, type) ? 1u : 0u;
    return type == kDispatchType[kind_] ? 1u : 0u;  // a constant (§15.4)
}

bool cFC4ProtectionBase::CanDispatch(float x, float z, bool* allBusy) {
    Section sec(this);
    return sec.ok() && fc4_protection_can_dispatch(core_, x, z, allBusy);
}

int32_t cFC4ProtectionBase::GetMaxPossibleDispatchCount() {
    Section sec(this);
    return sec.ok() ? fc4_protection_max_possible_dispatch_count(core_) : 0;
}

// Slot 6: empties and copies as slot 22 / 21; EAX = 0 / 1 (count != 0).
uint32_t cFC4ProtectionBase::GetStations(SC4List<cISC4Occupant*>& list) {
    Section sec(this);
    if (!sec.ok()) return 0;
    std::vector<uint64_t> tokens;
    fc4_protection_stations(core_, &CollectToken, &tokens);
    ClearKeepingElements(list);
    AppendForCaller(list, tokens);
    return tokens.empty() ? 0u : 1u;
}

bool cFC4ProtectionBase::GetDispatchesAtStation(cISC4Occupant* station, uint32_t* flags, uint32_t* base,
                                                uint32_t* count) {
    Section sec(this);
    return sec.ok() && fc4_protection_dispatches_at_station(core_, TokenOf(station), flags, base, count);
}

// Slot 8: the station (EAX, borrowed) or null; *id := 0 unless a unit went.
cISC4Occupant* cFC4ProtectionBase::DispatchFromNearestStation(float x, float z, uint32_t* id) {
    Section sec(this);
    if (!sec.ok()) {
        if (id != nullptr) *id = 0;
        return nullptr;
    }
    return OccOf(fc4_protection_dispatch_from_nearest_station(core_, x, z, id));
}

bool cFC4ProtectionBase::DispatchFromStation(cISC4Occupant* station, uint32_t* id) {
    Section sec(this);
    return sec.ok() && fc4_protection_dispatch_from_station(core_, TokenOf(station), id);
}

bool cFC4ProtectionBase::RevokeDispatchFromStation(cISC4Occupant* station, uint32_t id) {
    Section sec(this);
    return sec.ok() && fc4_protection_revoke_dispatch_from_station(core_, TokenOf(station), id);
}

// Slot 11 (pop 20): `a` and `e` are not used (§8.2).
bool cFC4ProtectionBase::GetDispatchAutomataProperties(uint32_t, uint32_t* b, float* c, uint32_t* d, uint32_t) {
    Section sec(this);
    return sec.ok() && fc4_protection_dispatch_automata_properties(core_, b, c, d);
}
