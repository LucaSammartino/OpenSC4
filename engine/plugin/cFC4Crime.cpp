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

#include "cFC4Crime.h"

#include "FC4Checked.h"
#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4Watchdog.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "SC4Rect.h"
#include "StringResourceKey.h"
#include "StringResourceManager.h"
#include "cGZPersistResourceKey.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZPersistResourceKeyFilter.h"
#include "cIGZPersistResourceKeyList.h"
#include "cIGZPersistResourceManager.h"
#include "cIGZString.h"
#include "cISC4App.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4Demand.h"
#include "cISC4DemandSimulator.h"
#include "cISC4DisasterLayer.h"
#include "cISC4DisasterManager.h"
#include "cISC4DisasterManagerFire.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4OrdinanceSimulator.h"
#include "cISC4PoliceSimulator.h"
#include "cISC4PropDeveloper.h"
#include "cISC4PropManager.h"
#include "cISC4PropOccupant.h"
#include "cISC4ResidentialSimulator.h"
#include "cISC4SimGrid.h"
#include "cISC4Simulator.h"
#include "cISC4ZoneManager.h"
#include "cISCPropertyHolder.h"
#include "cRZAutoRefCount.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cfloat>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <string>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
/// The fire disaster manager's interface and type (§6.8, §13).
constexpr uint32_t kIID_cISC4DisasterManagerFire = 0x49B43F57;
constexpr uint32_t kFireDisasterManager = 0x621CDA33;
/// §11.3: the interfaces the nearest-occupant queries ask for.
constexpr uint32_t kIID_CrimeSim = 0xA9B40F05;  // presumably cISC4Automaton (§16 Q3)
constexpr uint32_t kIID_cISC4PropOccupant = 0xE9793A65;
constexpr uint32_t kSimOccupantType = 0x496E5783;
constexpr uint32_t kPropOccupantType = 0xA823821E;
constexpr uint32_t kCrimeSimGroup = 0x4102;
constexpr uint32_t kPropCrimeIncident = 0x8AA864D5;

// Messages (§5, §10).
constexpr uint32_t kMsgInsertOccupant = 0x99EF1142;
constexpr uint32_t kMsgRemoveOccupant = 0x99EF1143;
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgCrimeCommitted = 0x0A159D95;
constexpr uint32_t kMsgCrimeMapUpdated = 0x88EA4DFF;

// Resources (§7, §8).
constexpr uint32_t kExemplarType = 0x6534284A;
constexpr uint32_t kCrimeTypeGroup = 0xEA12F32C;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0xC8E8F6C5;

/// §6.4 / §5: the ordinance property and index read as the crime factor.
constexpr uint32_t kOrdinanceCrimeFactor = 0x28ED0380;
/// §11.6 slot 4.
constexpr uint32_t kRequesterSatisfactionId = 0x49A1E996;

using GridU8 = cISC4SimGrid<uint8_t>;
using GridU16 = cISC4SimGrid<uint16_t>;
using GridF32 = cISC4SimGrid<float>;

cFC4CrimeSimulator* Self(void* u) { return static_cast<cFC4CrimeSimulator*>(u); }
cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cISC4Lot* RawLot(uint64_t t) { return reinterpret_cast<cISC4Lot*>(FromToken(t)); }
cFC4ISC4Lot* LotOf(uint64_t t) { return t ? fc4::AsGame(RawLot(t)) : nullptr; }
GridU8* GridOf(uint64_t t) { return reinterpret_cast<GridU8*>(FromToken(t)); }

cFC4ISC4LotManager* LotMgr(void* u) {
    cISC4LotManager* lm = Self(u)->LotManager();
    return lm ? fc4::AsGame(lm) : nullptr;
}

// Whatever police simulator the city has (the original or a replacement),
// through the verified slot order (FC4Interfaces.h, crime.md section 13).
cFC4ISC4PoliceSimulator* Police(void* u) {
    cISC4PoliceSimulator* p = Self(u)->Police();
    return p ? fc4::AsGame(p) : nullptr;
}

bool Gate(fc4::Sw s, const char* what) {
    if (fc4::On(s)) return true;
    fc4::NoteSkipped(s, what);
    return false;
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

template <class T>
void Drop(T* p) {
    if (p != nullptr) FC4_RELEASE(reinterpret_cast<cIGZUnknown*>(p));
}

// A heap string for the crime-committed message (§10: "a new copy of the type
// name"). cRZBaseString never deletes itself; this one is freed by its last
// Release (the message's, once the server has delivered it).
class CrimeName final : public cRZBaseString {
public:
    CrimeName(const char* text, size_t len) : cRZBaseString(text, len) {}
    uint32_t AddRef() override { return static_cast<uint32_t>(InterlockedIncrement(&refs_)); }
    uint32_t Release() override {
        const LONG r = InterlockedDecrement(&refs_);
        if (r <= 0) {
            delete this;
            return 0;
        }
        return static_cast<uint32_t>(r);
    }

private:
    volatile LONG refs_ = 0;
};

// ---- resources (§7, §8): also the shadow's read-only host ---------------------------------------

fc4_props* Tuning(void*) {
    if (!Gate(fc4::Sw::ExemplarLoads, "crime tuning exemplar (none: code defaults)")) return nullptr;
    fc4::props::Exemplar ex;
    if (!ex.Load(kExemplarType, kTuningGroup, kTuningInstance)) {
        fc4::Log("crime: tuning exemplar unavailable; the code defaults apply (§8 [choice])");
        return nullptr;
    }
    return fc4::props::ToCoreAll(ex.Holder());
}

// Crime type exemplars only (type and group, §7). Lives on the stack of the
// enumeration; references are counted but never delete it.
class CrimeTypeKeyFilter final : public cIGZPersistResourceKeyFilter {
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
    bool IsKeyIncluded(cGZPersistResourceKey const& key) override {
        return key.type == kExemplarType && key.group == kCrimeTypeGroup;
    }

private:
    uint32_t refs_ = 1;
};

// §7: GetAvailableResourceList(&list, filter) with a null list pointer on
// entry; the manager returns a new list with one reference, ours to free
// (as demand's enumeration, run 42). The core orders the keys ([choice]
// ascending instance, §7).
void ForEachTypeKey(void*, fc4_key_visitor visit, void* ctx) {
    if (visit == nullptr) return;
    if (!Gate(fc4::Sw::ExemplarLoads, "crime type enumeration (no crime types)")) return;
    cIGZPersistResourceManagerPtr rm;
    if (!rm) {
        fc4::Log("crime: no resource manager; no crime types");
        return;
    }
    CrimeTypeKeyFilter filter;
    FC4_OUT(cIGZPersistResourceKeyList*, list, "cIGZPersistResourceKeyList* (GetAvailableResourceList out pointer)");
    rm->GetAvailableResourceList(list.ptr(), &filter);
    list.Check();
    cIGZPersistResourceKeyList* l = list.get();
    if (l == nullptr) {
        fc4::Log("crime: WARNING the resource manager returned no key list; no crime types");
        return;
    }
    FC4_ACQUIRED(l, "GetAvailableResourceList");
    std::vector<cGZPersistResourceKey> keys;
    const uint32_t n = l->Size();
    keys.reserve(n);
    for (uint32_t i = 0; i < n; ++i) keys.push_back(l->GetKey(i));
    FC4_RELEASE_CREATED(l);
    uint32_t found = 0;
    for (const cGZPersistResourceKey& k : keys) {
        if (k.type != kExemplarType || k.group != kCrimeTypeGroup) continue;
        visit(ctx, k.type, k.group, k.instance);
        ++found;
    }
    fc4::Log("crime: %u resource keys enumerated, %u crime type exemplars", n, found);
}

fc4_props* ExemplarProps(void*, uint32_t type, uint32_t group, uint32_t instance) {
    if (!Gate(fc4::Sw::ExemplarLoads, "crime type exemplar (none)")) return nullptr;
    fc4::props::Exemplar ex;
    if (!ex.Load(type, group, instance)) return nullptr;
    return fc4::props::ToCoreAll(ex.Holder());
}

// The display name (§7: SC4LoadExemplarString's LTEXT lookup).
bool Ltext(void*, uint32_t, uint32_t group, uint32_t instance, fc4_bytes_sink sink, void* ctx) {
    if (!Gate(fc4::Sw::Ltext, "crime type name LTEXT (not found)")) return false;
    cRZAutoRefCount<cIGZString> s;  // the SDK helper owns the reference
    if (!StringResourceManager::GetLocalizedString(StringResourceKey(group, instance), s)) return false;
    if (s && sink != nullptr) {
        const char* p = s->ToChar();
        if (p != nullptr) sink(ctx, reinterpret_cast<const uint8_t*>(p), s->Strlen());
    }
    return true;
}

// ---- lifecycle glue (§4) ------------------------------------------------------------------------

void AddAgentCb(void* u, const uint8_t* name, size_t len, const uint32_t* levels, uint32_t count, uint32_t flags) {
    const std::string n(reinterpret_cast<const char*>(name), len);
    Self(u)->AddAgent(n.c_str(), levels, count, flags);
}
void RemoveAgentCb(void* u) { Self(u)->RemoveAgent(); }
void SubscribeCb(void* u, const uint32_t* ids, uint32_t count) { Self(u)->Subscribe(ids, count, true); }
void UnsubscribeCb(void* u, const uint32_t* ids, uint32_t count) { Self(u)->Subscribe(ids, count, false); }
void AcquireServicesCb(void* u) { Self(u)->AcquireServices(); }
void ReleaseServicesCb(void* u) { Self(u)->ReleaseServices(); }

// §4.1 step 8: AddPropRequester (prop developer slot 5) on the prop-requester
// sub-object. §4.2: never unregistered.
void AddPropRequesterCb(void* u) {
    if (!Gate(fc4::Sw::CrimeProps, "crime AddPropRequester (not registered)")) return;
    cISC4City* c = Self(u)->City();
    cISC4PropDeveloper* pd = c ? c->GetPropDeveloper() : nullptr;  // city slot 41, borrowed
    if (pd == nullptr) {
        fc4::Log("crime: no prop developer; the prop requester is not registered");
        return;
    }
    pd->AddPropRequester(reinterpret_cast<cISC4PropRequester*>(Self(u)->Requester()));
    fc4::Log("crime: prop requester %p registered with the prop developer (new city)",
             static_cast<void*>(Self(u)->Requester()));
}

// ---- residential, demand (§6.1, §6.2) -------------------------------------------------------------

// GetPopulationGrids / GetEQGrids (residential slots 25 / 23): three out
// pointers, each AddRef'd for us (§13); the core releases them (grids.release).
template <class T, class F>
void ThreeGrids(void* u, uint64_t* out3, F call, const char* what) {
    if (out3 == nullptr) return;
    out3[0] = out3[1] = out3[2] = 0;
    if (!Gate(fc4::Sw::CrimeResidential, what)) return;
    cISC4City* c = Self(u)->City();
    cISC4ResidentialSimulator* r = c ? c->GetResidentialSimulator() : nullptr;  // city slot 68, borrowed
    if (r == nullptr) return;
    cISC4SimGrid<T>* g[3] = {nullptr, nullptr, nullptr};
    call(fc4::AsGame(r), g);
    for (int i = 0; i < 3; ++i) {
        if (g[i] == nullptr) continue;
        FC4_ACQUIRED(g[i], "GetPopulationGrids / GetEQGrids");
        out3[i] = TokenOf(g[i]);
    }
}

void PopulationGrids(void* u, uint64_t* out3) {
    ThreeGrids<uint16_t>(
        u, out3, [](cFC4ISC4ResidentialSimulator* r, GridU16** g) { r->GetPopulationGrids(g[0], g[1], g[2]); },
        "crime GetPopulationGrids (no population)");
}

void EqGrids(void* u, uint64_t* out3) {
    ThreeGrids<float>(
        u, out3, [](cFC4ISC4ResidentialSimulator* r, GridF32** g) { r->GetEQGrids(g[0], g[1], g[2]); },
        "crime GetEQGrids (no EQ)");
}

// GetDemand(id, 0) (demand simulator slot 6, borrowed), then the node's
// active value / max / min (slots 12, 14, 15).
bool Demand(void* u, uint32_t id, float* value, float* max, float* min) {
    if (!Gate(fc4::Sw::CrimeDemand, "crime GetDemand (absent)")) return false;
    cISC4City* c = Self(u)->City();
    cISC4DemandSimulator* ds = c ? c->GetDemandSimulator() : nullptr;  // city slot 56, borrowed
    if (ds == nullptr) return false;
    cISC4Demand* d = fc4::AsGame(ds)->GetDemand(id, 0);
    if (d == nullptr) return false;
    cFC4ISC4Demand* n = fc4::AsGame(d);
    if (value != nullptr) *value = n->QueryActiveDemandValue();
    if (max != nullptr) *max = n->QueryActiveDemandMax();
    if (min != nullptr) *min = n->QueryActiveDemandMin();
    return true;
}

// ---- police and ordinances (§6.4, §6.6) ----------------------------------------------------------

// Police slot 5: borrowed (no AddRef by the police simulator); one reference
// is taken here and handed to the core (fc4.h), which releases it.
uint64_t PoliceGrid(void* u) {
    if (!Gate(fc4::Sw::CrimePolice, "crime GetPolicePowerGrid (no coverage)")) return 0;
    cFC4ISC4PoliceSimulator* p = Police(u);
    GridU8* g = p ? p->GetPolicePowerGrid() : nullptr;
    Self(u)->Probe().policeGrid = g != nullptr ? 1u : 0u;
    if (g == nullptr) return 0;
    FC4_ADDREF(g);
    return TokenOf(g);
}

uint32_t JailPopulation(void* u) {
    if (!Gate(fc4::Sw::CrimePolice, "crime GetJailPopulation (0)")) return 0;
    cFC4ISC4PoliceSimulator* p = Police(u);
    const uint32_t r = p ? p->GetJailPopulation() : 0;  // slot 30
    Self(u)->Probe().jailed = r;
    return r;
}

int32_t ResetUnjailedCriminals(void* u) {
    if (!Gate(fc4::Sw::CrimePolice, "crime ResetUnjailedCriminals (0)")) return 0;
    cFC4ISC4PoliceSimulator* p = Police(u);
    const int32_t r = p ? p->ResetUnjailedCriminals() : 0;  // slot 27, int32 count (§13)
    Self(u)->Probe().released = r;
    return r;
}

void ProcessMonthlyCrimeStats(void* u) {
    if (!Gate(fc4::Sw::CrimePolice, "crime ProcessMonthlyCrimeStats")) return;
    if (cFC4ISC4PoliceSimulator* p = Police(u)) p->ProcessMonthlyCrimeStats();  // slot 38, result ignored
}

// §5: GetCumulativeFactor(0x28ED0380, 0); false = no ordinance simulator
// ([choice] the factor is left unchanged).
bool OrdinanceCrimeFactor(void* u, float* out) {
    if (!Gate(fc4::Sw::CrimeOrdinance, "crime GetCumulativeFactor (factor unchanged)")) return false;
    cISC4OrdinanceSimulator* o = Self(u)->Ordinance();
    if (o == nullptr) return false;
    const float f = o->GetCumulativeFactor(kOrdinanceCrimeFactor, 0);  // slot 25, ST0
    if (out != nullptr) *out = f;
    return true;
}

// ---- lots and zones (§6.6-§6.9, §11.4, §11.6): borrowed lot pointers ------------------------------

uint64_t RandomLot(void* u) {
    if (!Gate(fc4::Sw::CrimeLots, "crime GetRandomLot (none)")) return 0;
    cFC4ISC4LotManager* lm = LotMgr(u);
    cISC4Lot* lot = lm ? lm->GetRandomLot() : nullptr;  // lot manager slot 13, borrowed
    if (lot != nullptr) {
        ++Self(u)->Probe().lotsFound;
    } else {
        ++Self(u)->Probe().noLot;
    }
    return TokenOf(lot);
}

uint64_t LotAt(void* u, int32_t x, int32_t z) {
    if (!Gate(fc4::Sw::CrimeLots, "crime GetLot (none)")) return 0;
    cFC4ISC4LotManager* lm = LotMgr(u);
    return lm ? TokenOf(lm->GetLot(x, z, false)) : 0;  // lot manager slot 6, borrowed
}

void LotBoundingRect(void*, uint64_t lot, fc4_cell_rect* out) {
    if (out == nullptr || lot == 0 || !Gate(fc4::Sw::CrimeLots, "crime GetBoundingRect")) return;
    SC4Rect<int32_t> r{};
    LotOf(lot)->GetBoundingRect(r);  // lot slot 14
    out->x1 = r.topLeftX;
    out->z1 = r.topLeftY;
    out->x2 = r.bottomRightX;
    out->z2 = r.bottomRightY;
}

// Counted: the core asks for the location only in GetLotBits, i.e. once per
// attempt the police did not stop (§6.6 step 6, §6.7).
bool LotLocation(void* u, uint64_t lot, int32_t* x, int32_t* z) {
    ++Self(u)->Probe().lotBits;
    if (lot == 0 || !Gate(fc4::Sw::CrimeLots, "crime GetLocation (none)")) return false;
    int32_t a = 0, b = 0;
    const bool ok = LotOf(lot)->GetLocation(a, b);  // lot slot 21
    if (x != nullptr) *x = a;
    if (z != nullptr) *z = b;
    return ok;
}

uint8_t LotCondition(void*, uint64_t lot) {
    if (lot == 0 || !Gate(fc4::Sw::CrimeLots, "crime GetCondition (0)")) return 0;
    return static_cast<uint8_t>(LotOf(lot)->GetCondition());  // lot slot 35, AL
}

// lot.GetBuilding() (slot 48, not reference-counted) and its profile (§6.7:
// purpose at +0, wealth at +4; docs/abi/layouts.md). The token is the
// building occupant (cISC4BuildingOccupant*), as `ignite` expects.
bool LotBuilding(void*, uint64_t lot, uint64_t* building, uint32_t* purpose, uint32_t* wealth) {
    if (lot == 0 || !Gate(fc4::Sw::CrimeLots, "crime GetBuilding (none)")) return false;
    cISC4BuildingOccupant* b = LotOf(lot)->GetBuilding();
    if (b == nullptr) return false;
    const cISC4BuildingOccupant::BuildingProfile& p = b->GetBuildingProfile();  // slot 11, read only
    if (building != nullptr) *building = TokenOf(b);
    if (purpose != nullptr) *purpose = static_cast<uint32_t>(p.purpose);
    if (wealth != nullptr) *wealth = static_cast<uint32_t>(p.wealth);
    return true;
}

// Lot slot 15 (+0x3C), GetCenter(cS3DVector3&) const: thiscall, one 4-byte
// argument popped by the callee. §13 says the result is in the out parameter
// and the function leaves nothing on the x87 stack (it ends with FSTP);
// docs/abi tags its return as x87 (ST0). [choice] Both readings are
// survived: after the call ST0 is examined (FXAM) and popped unless it is
// empty, so the FPU stack stays balanced either way; the out parameter is the
// value used. Returns whether ST0 held a value (counted for the month line).
bool LotCenterRaw(cISC4Lot* lot, float* out3) {
    void* self = lot;
    void* fn = (*reinterpret_cast<void***>(lot))[15];
    uint16_t sw = 0;
    __asm {
        push out3
        mov ecx, self
        call fn
        fxam
        fnstsw ax
        mov sw, ax
        and ax, 0x4500
        cmp ax, 0x4100
        je center_st0_empty
        fstp st(0)
    center_st0_empty:
    }
    return (sw & 0x4500) != 0x4100;
}

// §11.4: city.PositionToCell(lot.GetCenter()).
void LotCenterCell(void* u, uint64_t lot, int32_t* x, int32_t* z) {
    int32_t a = 0, b = 0;
    cISC4City* c = Self(u)->City();
    if (lot != 0 && c != nullptr && Gate(fc4::Sw::CrimeLots, "crime GetCenter (cell 0, 0)")) {
        float v[4] = {0.0f, 0.0f, 0.0f, 0.0f};
        if (LotCenterRaw(RawLot(lot), v)) {
            ++Self(u)->Probe().centerSt0;
        } else {
            ++Self(u)->Probe().centerEmptySt0;
        }
        c->PositionToCell(v[0], v[2], a, b);  // city slot 95
    }
    if (x != nullptr) *x = a;
    if (z != nullptr) *z = b;
}

uint32_t ZoneType(void* u, int32_t x, int32_t z) {
    if (!Gate(fc4::Sw::CrimeZoning, "crime GetZoneType (unzoned)")) return 0;
    cISC4City* c = Self(u)->City();
    cISC4ZoneManager* zm = c ? c->GetZoneManager() : nullptr;  // city slot 36, borrowed
    return zm ? static_cast<uint32_t>(zm->GetZoneType(x, z)) : 0;  // zone manager slot 12
}

// Incident lots (§6.8 step 1: the incident holds a reference).
void AddRefToken(void*, uint64_t t) {
    if (cIGZUnknown* o = FromToken(t)) FC4_ADDREF(o);
}
void ReleaseToken(void*, uint64_t t) {
    if (cIGZUnknown* o = FromToken(t)) FC4_RELEASE(o);
}

// ---- occupants and props (§6.10, §11.3, §11.6) ----------------------------------------------------

bool OccupantHasProperty(void*, uint64_t occ, uint32_t id) {
    if (occ == 0 || !Gate(fc4::Sw::CrimeOccupants, "crime occupant HasProperty (absent)")) return false;
    cISCPropertyHolder* h = OccOf(occ)->AsPropertyHolder();  // slot 6, borrowed
    return h != nullptr && h->HasProperty(id);
}

// §6.10: the property as a byte array {m, r}. [choice] Read with the
// type-converting numeric reader (each element's low byte): exact for the
// u8 array the property holds; the original's RefUint8 on another variant
// type is not specified.
bool OccupantPropertyBytes(void*, uint64_t occ, uint32_t id, fc4_bytes_sink sink, void* ctx) {
    if (occ == 0 || !Gate(fc4::Sw::CrimeOccupants, "crime occupant property read (absent)")) return false;
    cISCPropertyHolder* h = OccOf(occ)->AsPropertyHolder();
    if (h == nullptr) return false;
    std::vector<uint32_t> v;
    if (!fc4::props::GetU32s(h, id, v)) return false;
    std::vector<uint8_t> bytes(v.size());
    for (size_t i = 0; i < v.size(); ++i) bytes[i] = static_cast<uint8_t>(v[i]);
    if (sink != nullptr && !bytes.empty()) sink(ctx, bytes.data(), bytes.size());
    return true;
}

void OccupantPosition(void*, uint64_t occ, float* xyz) {
    if (xyz == nullptr || occ == 0 || !Gate(fc4::Sw::CrimeOccupants, "crime occupant GetPosition")) return;
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);  // slot 8
    xyz[0] = v.fX;
    xyz[1] = v.fY;
    xyz[2] = v.fZ;
}

// §11.3: the box query. [choice] §13 names the FILTER overload (slot 36) with
// a type-and-group / type-and-property filter; cISC4OccupantFilter is not in
// docs/abi/interfaces.tsv, so the verified TYPED overload (slot 37, which
// forwards to slot 36 with a type filter, allocators.md) is called with the
// type, and the second condition is reported per occupant for the core's
// selection (fc4_crime_nearest_of). The last argument is 0xFFFFFFFF ("all
// flags", §16 Q5). The list (A, one reference each) is released when it goes.
template <class F>
void QueryBox(cISC4OccupantManager* om, const float* min3, const float* max3, uint32_t kind, F each) {
    const bool sim = kind == FC4_CRIME_NEAREST_SIM;
    SC4List<cISC4Occupant*> list;
    om->GetOccupantsByBBox(list, min3, max3, sim ? kSimOccupantType : kPropOccupantType, 0xFFFFFFFFu);  // slot 37
    for (cISC4Occupant* o : list) {
        if (o == nullptr) continue;
        fc4_crime_nearby n{};
        n.token = TokenOf(o);
        n.occupant_type = static_cast<uint32_t>(o->GetType());  // slot 7
        if (sim) {
            n.in_crime_group = o->IsOccupantGroup(kCrimeSimGroup) ? 1 : 0;  // slot 22
        } else {
            cISCPropertyHolder* h = o->AsPropertyHolder();
            n.has_incident_property = (h != nullptr && h->HasProperty(kPropCrimeIncident)) ? 1 : 0;
        }
        cS3DVector3 v;
        o->GetPosition(&v);
        n.position[0] = v.fX;
        n.position[1] = v.fY;
        n.position[2] = v.fZ;
        void* p = nullptr;
        if (o->QueryInterface(sim ? kIID_CrimeSim : kIID_cISC4PropOccupant, &p) && p != nullptr) {
            FC4_ACQUIRED(static_cast<cIGZUnknown*>(p), "QueryInterface");
            n.answers_iid = 1;
            if (!sim) n.prop_state = static_cast<cISC4PropOccupant*>(p)->GetState();  // slot 12
            FC4_RELEASE(static_cast<cIGZUnknown*>(p));
        }
        each(o, n);
    }
}

void OccupantsInBox(void* u, const float* min3, const float* max3, uint32_t kind, fc4_crime_nearby_callback cb,
                    void* ctx) {
    if (min3 == nullptr || max3 == nullptr || cb == nullptr) return;
    if (!Gate(fc4::Sw::CrimeOccupants, "crime occupant box query (none)")) return;
    cISC4OccupantManager* om = Self(u)->OccupantManager();
    if (om == nullptr) return;
    QueryBox(om, min3, max3, kind, [&](cISC4Occupant*, const fc4_crime_nearby& n) { cb(ctx, &n); });
}

// §11.6: PropManager.GetPropProfile(id) (slot 13) -> PropProfile.crimeIncidentID.
// [choice] The field is read at offset 24, the SDK's PropProfile layout
// (u32 class, cS3DVector3 size, u32 satisfaction, u8 wealth, u32 incident
// ID); docs/abi/layouts.md does not list PropProfile (spec question).
uint32_t PropCrimeIncidentId(void* u, uint32_t prop) {
    if (!Gate(fc4::Sw::CrimeProps, "crime GetPropProfile (generic)")) return 0;
    cISC4City* c = Self(u)->City();
    cISC4PropManager* pm = c ? c->GetPropManager() : nullptr;  // borrowed
    if (pm == nullptr) return 0;
    const cISC4PropOccupant::PropProfile& p = pm->GetPropProfile(prop);
    uint32_t id = 0;
    std::memcpy(&id, reinterpret_cast<const uint8_t*>(&p) + 24, sizeof(id));
    return id;
}

// ---- arson (§6.8) ---------------------------------------------------------------------------------

bool DisasterLayerPresent(void* u) {
    if (!Gate(fc4::Sw::CrimeFire, "crime GetDisasterLayer (none: no arson fire)")) return false;
    cISC4City* c = Self(u)->City();
    return c != nullptr && c->GetDisasterLayer() != nullptr;  // city slot 72
}

// GetDisasterManager(0x621CDA33) queried for cISC4DisasterManagerFire, then
// IgniteOccupant(building as occupant) and SetCause(cause). Ownership: the
// layer and the manager are BORROWED (flammability run 37); only the
// QueryInterface result is ours, released once. [choice] A missing manager or
// interface (the original does not check, §6.8 [quirk]) skips the fire.
// SetCause follows IgniteOccupant whatever it returned (the literal §6.8).
void Ignite(void* u, uint64_t building, uint32_t cause) {
    ++Self(u)->Probe().igniteCalls;
    if (building == 0 || !Gate(fc4::Sw::CrimeFire, "crime IgniteOccupant (no fire)")) return;
    cISC4City* c = Self(u)->City();
    cISC4DisasterLayer* layer = c ? c->GetDisasterLayer() : nullptr;  // borrowed
    cISC4DisasterManager* m = layer ? layer->GetDisasterManager(kFireDisasterManager) : nullptr;  // slot 7, borrowed
    if (m == nullptr) {
        fc4::Log("crime: arson: no fire disaster manager; no fire");
        return;
    }
    cISC4DisasterManagerFire* fire = Query<cISC4DisasterManagerFire>(m, kIID_cISC4DisasterManagerFire);
    if (fire == nullptr) {
        fc4::Log("crime: arson: the disaster manager does not answer 0x49B43F57; no fire");
        return;
    }
    cISC4BuildingOccupant* b = reinterpret_cast<cISC4BuildingOccupant*>(FromToken(building));
    cISC4Occupant* occ = b->AsOccupant();  // building occupant slot 3
    const bool ok = occ != nullptr && fire->IgniteOccupant(occ);  // fire slot 4
    fire->SetCause(cause);                                         // fire slot 9
    Drop(fire);
    if (ok) ++Self(u)->Probe().ignited;
}

// ---- messages (§10) -------------------------------------------------------------------------------

void PostCrimeCommitted(void* u, uint32_t typeId, uint64_t lot, const uint8_t* name, size_t len) {
    ++Self(u)->Probe().posts;
    if (!Gate(fc4::Sw::CrimePosts, "crime 0x0A159D95 post")) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* msg = FC4MakeMessage(kMsgCrimeCommitted, static_cast<intptr_t>(typeId), 0, 0);
    msg->SetIGZUnknown(FromToken(lot));
    CrimeName* s = new CrimeName(reinterpret_cast<const char*>(name), len);
    s->AddRef();  // ours, until the message holds it
    msg->SetString(s);
    s->Release();
    ms->MessagePost(msg, false);
    msg->Release();
}

void PostCrimeMapUpdated(void* u) {
    ++Self(u)->Probe().mapPosts;
    if (!Gate(fc4::Sw::CrimePosts, "crime 0x88EA4DFF post")) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* msg = FC4MakeMessage(kMsgCrimeMapUpdated, 0, 0, 0);
    void* self = FromToken(Self(u)->SelfToken());  // the primary interface, no AddRef (§10)
    msg->SetVoid1(self);
    msg->SetIGZUnknown(static_cast<cIGZUnknown*>(self));
    ms->MessagePost(msg, false);
    msg->Release();
}

fc4_crime_host MakeHost(cFC4CrimeSimulator* self, const fc4::HostGridContext* grids) {
    fc4_crime_host h{};
    h.struct_size = sizeof(h);
    h.user = self;
    h.grids = fc4::MakeGridApi(grids);
    h.add_agent = &AddAgentCb;
    h.remove_agent = &RemoveAgentCb;
    h.subscribe = &SubscribeCb;
    h.unsubscribe = &UnsubscribeCb;
    h.acquire_services = &AcquireServicesCb;
    h.release_services = &ReleaseServicesCb;
    h.add_prop_requester = &AddPropRequesterCb;
    h.tuning = &Tuning;
    h.for_each_type_key = &ForEachTypeKey;
    h.exemplar = &ExemplarProps;
    h.ltext = &Ltext;
    h.population_grids = &PopulationGrids;
    h.eq_grids = &EqGrids;
    h.demand = &Demand;
    h.police_grid = &PoliceGrid;
    h.jail_population = &JailPopulation;
    h.reset_unjailed_criminals = &ResetUnjailedCriminals;
    h.process_monthly_crime_stats = &ProcessMonthlyCrimeStats;
    h.ordinance_crime_factor = &OrdinanceCrimeFactor;
    h.random_lot = &RandomLot;
    h.lot_at = &LotAt;
    h.lot_bounding_rect = &LotBoundingRect;
    h.lot_location = &LotLocation;
    h.lot_condition = &LotCondition;
    h.lot_building = &LotBuilding;
    h.lot_center_cell = &LotCenterCell;
    h.zone_type = &ZoneType;
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.occupant_has_property = &OccupantHasProperty;
    h.occupant_property_bytes = &OccupantPropertyBytes;
    h.occupant_position = &OccupantPosition;
    h.occupants_in_box = &OccupantsInBox;
    h.prop_crime_incident_id = &PropCrimeIncidentId;
    h.disaster_layer_present = &DisasterLayerPresent;
    h.ignite = &Ignite;
    h.post_crime_committed = &PostCrimeCommitted;
    h.post_crime_map_updated = &PostCrimeMapUpdated;
    return h;
}

// The session seed S (foundations §6.3): FreeCity4.ini `crime_seed` (env
// FC4_CRIME_SEED) when set, else the local wall-clock seconds like the
// original (§9). [choice] "Local seconds" = the local calendar time read as
// seconds since 1970-01-01 (the original's epoch is not specified).
uint32_t SessionSeed(bool* fixed) {
    const std::string s = fc4::Setting("crime_seed", "FC4_CRIME_SEED");
    if (!s.empty()) {
        char* end = nullptr;
        const unsigned long v = std::strtoul(s.c_str(), &end, 0);
        if (end != s.c_str()) {
            *fixed = true;
            return static_cast<uint32_t>(v);
        }
    }
    *fixed = false;
    const std::time_t now = std::time(nullptr);
    std::tm local{};
    if (localtime_s(&local, &now) != 0) return static_cast<uint32_t>(now);
    return static_cast<uint32_t>(_mkgmtime(&local));
}

uint32_t TruncAverage(uint32_t bits) {
    float f = 0.0f;
    std::memcpy(&f, &bits, sizeof(f));
    // _ftol (§6 notation): NaN and out-of-range give the int64 indefinite,
    // whose low half is 0.
    if (!(f > -9.2233720e18f && f < 9.2233720e18f)) return 0;
    return static_cast<uint32_t>(static_cast<int64_t>(f));
}

}  // namespace

namespace fc4 {
fc4_crime_host CrimeReadOnlyHost() {
    fc4_crime_host h{};
    h.struct_size = sizeof(h);
    h.tuning = &Tuning;
    h.for_each_type_key = &ForEachTypeKey;
    h.exemplar = &ExemplarProps;
    h.ltext = &Ltext;
    return h;
}
}  // namespace fc4

// ---- object -------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4CrimeSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4CrimeSimulator*>(new cFC4CrimeSimulator()));
}

cFC4CrimeSimulator::cFC4CrimeSimulator() {
    host_ = MakeHost(this, &gridCtx_);
    bool fixed = false;
    const uint32_t seed = SessionSeed(&fixed);
    // §9: one generator per process, seeded by the first object with
    // derive_seed(S, 0x6C09C3F6); later objects share it (fc4.h).
    core_ = fc4_crime_create(&host_, SelfToken(), seed);
    if (core_ == nullptr) fc4::Log("cFC4CrimeSimulator: core creation failed");
    fc4::Log("cFC4CrimeSimulator(%p) created, core %p, session seed 0x%08X (%s)", static_cast<void*>(this),
             static_cast<void*>(core_), seed, fixed ? "crime_seed" : "local wall clock");
}

cFC4CrimeSimulator::~cFC4CrimeSimulator() {
    fc4_crime_destroy(core_);
    core_ = nullptr;
    fc4::Log("cFC4CrimeSimulator destroyed");
}

uint32_t cFC4CrimeSimulator::GetSimulatorType() { return kFC4_CrimeSimulatorType; }

bool cFC4CrimeSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_IID_cISC4CrimeSimulator:
        case GZIID_cIGZUnknown:
            // §1: cIGZUnknown is answered with the primary interface.
            *ppvObj = static_cast<cFC4ISC4CrimeSimulator*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case kFC4_IID_cISC4PropRequesterCrime:
            *ppvObj = static_cast<cFC4CrimePropRequester*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        default:
            // §1: false and the out pointer null; in particular cIGZMessageTarget
            // (0xE98B2F57) is not answered.
            *ppvObj = nullptr;
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4CrimeSimulator::AddRef() { return ++refCount_; }

// §1: at a count of 1 (or below) Shutdown, destroy, return 0.
uint32_t cFC4CrimeSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

// §4.1 step 6 (and the services the callbacks need): the city from the
// application, then each service while its member is still null. [choice] A
// missing service is held as null (§4.1 step 6).
void cFC4CrimeSimulator::AcquireServices() {
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
    if (city_ == nullptr) return;
    auto take = [](auto*& member, auto* value) {
        if (member == nullptr && value != nullptr) {
            member = value;
            FC4_ADDREF(member);
        }
    };
    take(occupantManager_, city_->GetOccupantManager());
    take(lotManager_, city_->GetLotManager());
    take(ordinance_, city_->GetOrdinanceSimulator());
    take(police_, city_->GetPoliceSimulator());
}

// §4.2: police, ordinance, occupant manager, city (and our lot manager).
void cFC4CrimeSimulator::ReleaseServices() {
    auto drop = [](auto*& member) {
        if (member != nullptr) FC4_RELEASE(member);
        member = nullptr;
    };
    drop(police_);
    drop(ordinance_);
    drop(lotManager_);
    drop(occupantManager_);
    drop(city_);
}

void cFC4CrimeSimulator::Subscribe(const uint32_t* ids, size_t count, bool add) {
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

// §4.1 step 4: AddAgent (simulator slot 19) at each level, in order. The
// simulator is held until RemoveAgent (§4.2 runs it after the city is released).
void cFC4CrimeSimulator::AddAgent(const char* name, const uint32_t* levels, uint32_t count, uint32_t flags) {
    if (agentSim_ == nullptr) {
        AcquireServices();
        cISC4Simulator* sim = city_ ? city_->GetSimulator() : nullptr;  // city slot 50, borrowed
        if (sim == nullptr) {
            fc4::Log("crime: no simulator; agent not registered");
            return;
        }
        agentSim_ = sim;
        FC4_ADDREF(agentSim_);
    }
    const cRZBaseString n(name);
    for (uint32_t i = 0; i < count; ++i) fc4::AsGame(agentSim_)->AddAgent(Target(), levels[i], n, flags);
}

// §4.2: RemoveAgent(target), the all-levels overload (simulator slot 20).
void cFC4CrimeSimulator::RemoveAgent() {
    if (agentSim_ == nullptr) return;
    fc4::AsGame(agentSim_)->RemoveAgentAllLevels(Target());
    FC4_RELEASE(agentSim_);
    agentSim_ = nullptr;
}

void cFC4CrimeSimulator::LogState(const char* what) {
    fc4_crime_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_crime_get_state(core_, &st)) return;
    fc4::Log("crime %s: init=%u loaded=%u state=%u types=%u S=%u decay=%.3f reoffend=%.3f grid=%p effect=%p "
             "count=%u incidents=%u average=%u hotspot=(%d,%d)=%d ordinance=%.3f police=%p ordinanceSim=%p "
             "reentrant=%u",
             what, st.initialized, st.loaded_from_save, st.pipeline_state, st.types, st.tract_size, st.decay,
             st.reoffend, static_cast<void*>(FromToken(st.crime_grid)), static_cast<void*>(FromToken(st.effect_grid)),
             st.monthly_crime_count, st.incidents, TruncAverage(st.average_bits), st.hotspot_x, st.hotspot_z,
             st.hotspot_value, st.ordinance_factor, static_cast<void*>(police_), static_cast<void*>(ordinance_),
             st.reentrant_calls);
}

// The month status line (one per SimNewMonth, from the first one on): the
// pipeline run since the previous month line (§5: it starts at a month
// message and takes the next six day/idle deliveries).
void cFC4CrimeSimulator::LogMonth(int32_t julian) {
    fc4_crime_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_crime_get_state(core_, &st)) return;
    uint64_t sum = 0;
    uint32_t max = 0, tracts = 0;
    if (GridU8* g = GridOf(st.crime_grid)) {
        const int32_t nx = g->GetTractCountX();
        const int32_t nz = g->GetTractCountZ();
        for (int32_t x = 0; x < nx; ++x) {
            for (int32_t z = 0; z < nz; ++z) {
                const uint32_t v = g->GetTractValue(x, z);
                sum += v;
                if (v > max) max = v;
            }
        }
        tracts = static_cast<uint32_t>(nx > 0 && nz > 0 ? nx * nz : 0);
    }
    fc4_date d{};
    if (julian > 0) fc4_date_from_julian(julian, &d);
    const Probes& p = probes_;
    const uint32_t prevented = p.lotsFound > p.lotBits ? p.lotsFound - p.lotBits : 0;
    ++months_;
    fc4::Log("crime month %04d-%02d: crimes=%u prevented=%u arson=%u/%u incidents=%u grid sum/max=%llu/%u "
             "(tracts=%u) average=%u hotspot=(%d,%d)=%d props requested=%u selected=%u/%u jailed=%u released=%d "
             "criminal base=%d to commit=%d attempts=%u (no lot %u) police grid=%s ordinance=%.3f posts=%u/%u "
             "state=%u types=%u days=%u months=%u reentrant=%u incident arrays=%u center st0=%u/%u off-main=%u",
             d.year, d.month, st.monthly_crime_count, prevented, p.ignited, p.igniteCalls, st.incidents,
             static_cast<unsigned long long>(sum), max, tracts, TruncAverage(st.average_bits), st.hotspot_x,
             st.hotspot_z, st.hotspot_value, shareCalls_.exchange(0), selected_.exchange(0), selectCalls_.exchange(0),
             p.jailed, p.released, st.criminal_base, st.crimes_to_commit, p.lotsFound, p.noLot,
             p.policeGrid ? "yes" : "no", st.ordinance_factor, p.posts, p.mapPosts, st.pipeline_state, st.types,
             days_, months_, st.reentrant_calls, incidentArrayCalls_, p.centerSt0, p.centerSt0 + p.centerEmptySt0,
             offMainCalls_.load());
    probes_ = Probes{};
    incidentArrayCalls_ = 0;
}

// ---- lifecycle (§4) -----------------------------------------------------------------------------

bool cFC4CrimeSimulator::Init() {
    FC4_TRACE("CrimeSimulator::Init");
    AcquireServices();  // the callbacks need the city from the first step on
    {
        CoreCall call(this);
        fc4_crime_init(core_);  // §4.1 steps 1-9 (the core drives the callbacks)
    }
    LogState("Init");
    return true;  // §4.1: always
}

bool cFC4CrimeSimulator::Shutdown() {
    FC4_TRACE("CrimeSimulator::Shutdown");
    // Keep this object alive while the simulator and the message server drop
    // their references (unless this already is the final Release).
    if (!destroying_) AddRef();
    {
        CoreCall call(this);
        fc4_crime_shutdown(core_);  // §4.2 (the core drives the callbacks)
    }
    held_.clear();
    // Whatever the core did not release (Init never ran): ours to drop.
    if (msgServer_ != nullptr) {
        FC4_RELEASE(msgServer_);
        msgServer_ = nullptr;
    }
    RemoveAgent();
    ReleaseServices();
    fc4::Log("crime Shutdown (%u days, %u months)", days_, months_);
    if (!destroying_) Release();
    return true;  // §4.2: always
}

// ---- queries (§11): the core's snapshot, any thread -----------------------------------------------

cISC4SimGrid<uint8_t>* cFC4CrimeSimulator::GetCrimeGrid() {
    return GridOf(fc4_crime_grid(core_));  // slot 5, no AddRef
}

uint32_t cFC4CrimeSimulator::GetAverageCrime() { return fc4_crime_average(core_); }

uint32_t cFC4CrimeSimulator::GetMonthlyCrimeCount() { return fc4_crime_monthly_count(core_); }

bool cFC4CrimeSimulator::GetCrimeHotspot(uint8_t* value, int32_t* x, int32_t* z) {
    return fc4_crime_hotspot(core_, value, x, z);  // §11.2: nothing written when false
}

// §11.3. The query runs on the calling thread like the original's; the
// selection is the core's pure function. The QI reference of the winner is
// released before returning: the pointer is borrowed.
cIGZUnknown* cFC4CrimeSimulator::Nearest(cS3DVector3 const& pos, float radius, uint32_t kind) {
    if (!Gate(fc4::Sw::CrimeOccupants, "crime nearest occupant (none)")) return nullptr;
    cISC4OccupantManager* om = occupantManager_;
    if (om == nullptr) return nullptr;
    // §11.3: float32 corners.
    const float min3[3] = {pos.fX - radius, -FLT_MAX, pos.fZ - radius};
    const float max3[3] = {pos.fX + radius, FLT_MAX, pos.fZ + radius};
    std::vector<fc4_crime_nearby> found;
    std::vector<cISC4Occupant*> occs;
    QueryBox(om, min3, max3, kind, [&](cISC4Occupant* o, const fc4_crime_nearby& n) {
        found.push_back(n);
        occs.push_back(o);
    });
    const float p3[3] = {pos.fX, pos.fY, pos.fZ};
    const int32_t i = fc4_crime_nearest_of(kind, p3, radius, found.data(), static_cast<uint32_t>(found.size()));
    if (i < 0 || static_cast<size_t>(i) >= occs.size()) return nullptr;
    // The occupant manager keeps the occupant alive (the list's references
    // are gone); the QI result is the interface the caller gets, borrowed.
    void* p = nullptr;
    const uint32_t iid = kind == FC4_CRIME_NEAREST_SIM ? kIID_CrimeSim : kIID_cISC4PropOccupant;
    if (!occs[static_cast<size_t>(i)]->QueryInterface(iid, &p) || p == nullptr) return nullptr;
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(p), "QueryInterface");
    FC4_RELEASE(static_cast<cIGZUnknown*>(p));
    return static_cast<cIGZUnknown*>(p);
}

cISC4Automaton* cFC4CrimeSimulator::GetNearestCrimeSim(cS3DVector3 const& pos, float radius) {
    return reinterpret_cast<cISC4Automaton*>(Nearest(pos, radius, FC4_CRIME_NEAREST_SIM));
}

cISC4PropOccupant* cFC4CrimeSimulator::GetNearestCrimeProp(cS3DVector3 const& pos, float radius) {
    return reinterpret_cast<cISC4PropOccupant*>(Nearest(pos, radius, FC4_CRIME_NEAREST_PROP));
}

// §11.4, §14.2: the caller's vector is resized with the GAME's allocator (B,
// the STL node allocator; [choice] docs/abi/allocators.md does not list this
// method, the spec says "the same operator new / STLport allocator as the
// original"). Elements of incidents without a lot keep their previous
// content (zero when new). Without the allocator, nothing is touched.
bool cFC4CrimeSimulator::GetCrimeIncidentArray(fc4::GameVector<FC4CrimeEvent>& out) {
    if (fc4::OnMainThread()) ++incidentArrayCalls_;
    const uint32_t n = fc4_crime_incident_count(core_);
    std::vector<fc4_crime_event> ev(n);
    const uint32_t total = n != 0 ? fc4_crime_incidents(core_, ev.data(), n) : 0;
    const size_t count = total < n ? total : n;
    if (count > out.capacity() && !fc4::StlNodeAllocator::Available()) {
        static bool logged = false;
        if (!logged) {
            logged = true;
            fc4::Log("crime: WARNING GetCrimeIncidentArray: the game's allocator is unavailable; the caller's "
                     "vector is left as it was");
        }
        return false;
    }
    std::vector<FC4CrimeEvent> old(out.begin(), out.end());
    if (!out.reserve(count)) return false;
    out.clear();
    for (size_t i = 0; i < count; ++i) {
        FC4CrimeEvent e{};
        if (i < old.size()) e = old[i];
        const fc4_crime_event& c = ev[i];
        if (c.has_lot != 0) {
            e.cellX = c.cell_x;
            e.cellZ = c.cell_z;
            e.typeID = c.type_id;
            e.visualsCreated = c.visuals_created;
            e.iconType = c.icon_type;
            e.iconGroup = c.icon_group;
            e.iconInstance = c.icon_instance;
        }
        out.push_back(e);
    }
    return count > 0;
}

bool cFC4CrimeSimulator::SetVisualsCreated(uint32_t index, bool value) {
    return fc4_crime_set_visuals(core_, index, value);
}

bool cFC4CrimeSimulator::GetVisualsCreated(uint32_t index) { return fc4_crime_get_visuals(core_, index); }

// ---- messages (§5) ------------------------------------------------------------------------------

bool cFC4CrimeSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    CoreCall call(this);
    fc4_crime_message m{};
    m.msg_type = pMessage->GetType();
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    cISC4Occupant* occ = nullptr;
    int32_t julian = 0;
    if (s != nullptr) {
        m.data1 = static_cast<uint32_t>(s->GetData1());
        m.data2 = static_cast<uint32_t>(s->GetData2());
        if (m.msg_type == kMsgSimNewDay) julian = static_cast<int32_t>(s->GetData1());
        if (m.msg_type == kMsgInsertOccupant || m.msg_type == kMsgRemoveOccupant) {
            // §5: the occupant is void1, known by its cISC4Occupant identity.
            occ = Query<cISC4Occupant>(static_cast<cIGZUnknown*>(s->GetVoid1()), kIID_cISC4Occupant);
            m.occupant = TokenOf(occ);
        }
    }
    // A message delivered while another call into the core is running is
    // queued by the core: keep its occupant alive until that call returns.
    if (depth_ > 1 && occ != nullptr) {
        occ->AddRef();
        Hold(std::shared_ptr<void>(occ, [](void* p) { static_cast<cIGZUnknown*>(p)->Release(); }));
    }
    const bool r = fc4_crime_do_message(core_, &m);
    Drop(occ);
    Drop(s);
    if (m.msg_type == kMsgSimNewDay) {
        ++days_;
        if (julian > 0) lastJulian_ = julian;
    } else if (m.msg_type == kMsgSimNewMonth) {
        LogMonth(lastJulian_);
    }
    return r;  // §5: always true
}

// ---- persistence (§12) --------------------------------------------------------------------------

bool cFC4CrimeSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("CrimeSimulator::Write");
    CoreCall call(this);
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_crime_write(core_, a.Get());
    fc4::Log("crime Write -> %d", ok ? 1 : 0);
    return ok;
}

// §12: Read runs before Init (nothing here sizes state from the city).
bool cFC4CrimeSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("CrimeSimulator::Read");
    CoreCall call(this);
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_crime_read(core_, a.Get());
    fc4::Log("crime Read -> %d", ok ? 1 : 0);
    LogState("Read");
    return ok;
}

uint32_t cFC4CrimeSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4CrimeSimulator; }

// ---- prop requester (§11.6) -----------------------------------------------------------------------

bool cFC4CrimeSimulator::OffMain(const char* what) {
    if (fc4::MainThreadId() == 0 || fc4::OnMainThread()) return false;
    ++offMainCalls_;
    fc4::NoteOffMainThreadCall(what);
    if (!offMainLogged_.exchange(true)) {
        fc4::Log("crime: WARNING %s called off the main thread; answered 0 (the core is main-thread only)", what);
    }
    return true;
}

int32_t cFC4CrimeSimulator::GetRequestedShare(SC4Rect<int32_t> const& r) {
    if (OffMain("cSC4CrimeSimulator::GetRequestedShare")) return 0;
    ++shareCalls_;
    CoreCall call(this);
    return fc4_crime_requested_share(core_, r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY);
}

uint32_t cFC4CrimeSimulator::GetRequesterSatisfactionId() { return kRequesterSatisfactionId; }

bool cFC4CrimeSimulator::UsesCustomPropSelector() { return true; }

uint32_t cFC4CrimeSimulator::SelectProp(FC4U32VectorView const& props, int32_t cellX, int32_t cellZ) {
    if (OffMain("cSC4CrimeSimulator::SelectProp")) return 0;
    ++selectCalls_;
    const uint32_t* first = props.first;
    const size_t n = (first != nullptr && props.last > first) ? static_cast<size_t>(props.last - first) : 0;
    CoreCall call(this);
    const uint32_t r = fc4_crime_select_prop(core_, first, static_cast<uint32_t>(n), cellX, cellZ);
    if (r != 0) ++selected_;
    return r;
}
