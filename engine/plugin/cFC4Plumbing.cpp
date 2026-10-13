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

#include "cFC4Plumbing.h"

#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"
#include "compat/FC4Compat.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "SC4Point.h"
#include "SC4Rect.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4DisasterLayer.h"
#include "cISC4DisasterManager.h"
#include "cISC4HistoryWarehouse.h"
#include "cISC4LineItem.h"
#include "cISC4NeighborConnection.h"
#include "cISC4NeighborDeal.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4OrdinanceSimulator.h"
#include "cISC4PollutionSimulator.h"
#include "cISC4SimGrid.h"
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

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4BuildingOccupant = 0x87DDA3A9;
constexpr uint32_t kIID_cISC4NeighborDeal = 0x0773FEF1;
/// The pipe's class ID (docs/abi/direct-calls.md): only `cSC4PipeOccupant`
/// answers it, so a successful QueryInterface both identifies a pipe and
/// yields its base pointer, which is the `cISC4NetworkOccupant` sub-object
/// (at +0) and the `this` the non-virtual `SetWateredState` takes.
constexpr uint32_t kIID_cSC4PipeOccupant = 0x49C05B9F;

// Messages (§5.1, §6.7).
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimIdle = 0x6695681A;
constexpr uint32_t kMsgInsertOccupant = 0x99EF1142;
constexpr uint32_t kMsgRemoveOccupant = 0x99EF1143;
constexpr uint32_t kMsgDealStatus = 0xEA418D8F;
/// Posted at the end of a pass with void1 = IGZUnknown = this simulator
/// (§6.7 step 4): the water map view refreshes on it (§12.1).
constexpr uint32_t kMsgWaterMapChanged = 0x6946DC34;

// Tuning exemplar (§8, "Utilities"; plumbing tuning.rs TUNING_KEY).
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0xC911E35B;

// The pipe-burst disaster manager (§7.1).
constexpr uint32_t kPipeBurstManager = 0x4A1332E8;

// The building occupant type of the one-cell query (§5.3).
constexpr uint32_t kOccupantTypeBuilding = 0x278128A0;

using GridU8 = cISC4SimGrid<uint8_t>;

cFC4PlumbingSimulator* Self(void* u) { return static_cast<cFC4PlumbingSimulator*>(u); }

cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cFC4ISC4Lot* LotOf(uint64_t t) { return t ? fc4::AsGame(reinterpret_cast<cISC4Lot*>(FromToken(t))) : nullptr; }
cISC4LineItem* ItemOf(uint64_t t) { return reinterpret_cast<cISC4LineItem*>(FromToken(t)); }
cISC4NeighborDeal* DealOf(uint64_t t) { return reinterpret_cast<cISC4NeighborDeal*>(FromToken(t)); }
cISC4NeighborConnection* ConnOf(uint64_t t) { return reinterpret_cast<cISC4NeighborConnection*>(FromToken(t)); }
cFC4ISC4DepartmentBudget* DeptOf(uint64_t t) {
    return t ? fc4::AsGame(reinterpret_cast<cISC4DepartmentBudget*>(FromToken(t))) : nullptr;
}
GridU8* GridOf(uint64_t t) { return reinterpret_cast<GridU8*>(FromToken(t)); }

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

// Runs f(T*) on the occupant QI'd to `iid` (released afterwards); false if the
// QI fails.
template <class T, class F>
bool WithQi(uint64_t occ, uint32_t iid, F f) {
    T* p = Query<T>(FromToken(occ), iid);
    if (p == nullptr) return false;
    f(p);
    Drop(p);
    return true;
}

// ---- city, tuning, ordinance, pollution ---------------------------------------------------------

void CellCounts(void* u, int32_t* x, int32_t* z) {
    cISC4City* c = Self(u)->City();
    if (x != nullptr) *x = c ? static_cast<int32_t>(c->CellCountX()) : 0;
    if (z != nullptr) *z = c ? static_cast<int32_t>(c->CellCountZ()) : 0;
}

void PositionToCell(void* u, float x, float z, int32_t* cx, int32_t* cz) {
    int32_t a = 0, b = 0;
    if (cISC4City* c = Self(u)->City()) c->PositionToCell(x, z, a, b);  // slot 95
    if (cx != nullptr) *cx = a;
    if (cz != nullptr) *cz = b;
}

fc4_props* Tuning(void*) {
    if (!Gate(fc4::Sw::ExemplarLoads, "plumbing tuning exemplar (none)")) return nullptr;
    fc4::props::Exemplar ex;
    if (!ex.Load(kTuningType, kTuningGroup, kTuningInstance)) {
        fc4::Log("plumbing: tuning exemplar unavailable; the defaults stay");
        return nullptr;
    }
    return fc4::props::ToCoreAll(ex.Holder());
}

// The core seeds its own cRZRandom with this at Init (§4.1 step 2,
// foundations §6.1).
uint32_t WallClockSeed(void*) { return static_cast<uint32_t>(GetTickCount64() ^ GetCurrentProcessId()); }

bool CumulativeFactor(void* u, uint32_t id, uint32_t index, float* out) {
    if (!Gate(fc4::Sw::PlumbingOrdinance, "plumbing GetCumulativeFactor (absent)")) return false;
    cISC4City* c = Self(u)->City();
    cISC4OrdinanceSimulator* o = c ? c->GetOrdinanceSimulator() : nullptr;
    if (o == nullptr) return false;
    if (out != nullptr) *out = o->GetCumulativeFactor(id, index);  // slot 25, ST0
    return true;
}

int32_t WaterValue(void* u, int32_t x, int32_t z) {
    if (!Gate(fc4::Sw::PlumbingPollution, "plumbing GetWaterValue (0)")) return 0;
    cISC4PollutionSimulator* p = Self(u)->Pollution();
    if (p == nullptr) return 0;
    int32_t v = 0;
    p->GetWaterValue(static_cast<uint32_t>(x), static_cast<uint32_t>(z), v);  // slot 8
    return v;
}

// ---- framework ----------------------------------------------------------------------------------

void AddAgent(void* u, uint32_t level, const char* name, size_t len, uint32_t flags) {
    cISC4City* c = Self(u)->City();
    cFC4ISC4Simulator* sim = c ? fc4::AsGame(c->GetSimulator()) : nullptr;
    if (sim == nullptr) {
        fc4::Log("plumbing: no simulator; agent level %u not registered", level);
        return;
    }
    const cRZBaseString n(name, len);
    sim->AddAgent(Self(u)->Target(), level, n, flags);  // slot 19
}

void RemoveAgent(void* u) {
    cISC4City* c = Self(u)->City();
    cFC4ISC4Simulator* sim = c ? fc4::AsGame(c->GetSimulator()) : nullptr;
    if (sim != nullptr) sim->RemoveAgentAllLevels(Self(u)->Target());  // slot 20
}

void Subscribe(void* u, const uint32_t* ids, size_t n) { Self(u)->Subscribe(ids, n, true); }
void Unsubscribe(void* u, const uint32_t* ids, size_t n) { Self(u)->Subscribe(ids, n, false); }

// §6.7 step 4 / step 9. [choice] Both go through MessagePost(msg, false): the
// original uses SC4PostMessage2 for the deal status, which is not in the SDK,
// and the observable result is the same posted standard message.
void Post(void* u, uint32_t msgType, uint32_t data1) {
    if (!Gate(fc4::Sw::PlumbingPosts, "plumbing message post")) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* msg = FC4MakeMessage(msgType, 0, 0, 0);
    if (msgType == kMsgWaterMapChanged) {
        // void1 = IGZUnknown = the simulator (§6.7 step 4).
        msg->SetVoid1(FromToken(Self(u)->SelfToken()));
        msg->SetIGZUnknown(FromToken(Self(u)->SelfToken()));
    } else {
        msg->SetData1(static_cast<intptr_t>(data1));
    }
    ms->MessagePost(msg, false);
    msg->Release();
}

void RecordHistory(void* u, uint32_t id, uint32_t value) {
    if (!Gate(fc4::Sw::PlumbingHistory, "plumbing RecordHistoryData")) return;
    cISC4City* c = Self(u)->City();
    cISC4HistoryWarehouse* h = c ? c->GetHistoryWarehouse() : nullptr;
    if (h != nullptr) h->RecordHistoryData(id, value);  // slot 12
}

// ---- references ---------------------------------------------------------------------------------

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

// A saved pipe comes back as its cISC4NetworkOccupant (§9 reads pipes with IID
// 0x49C05B9F) and a producer as its cISC4Occupant; the core knows occupants by
// their cISC4Occupant identity. The core owns one reference per resolved token
// (released at Shutdown, or when a later Read replaces it). A host object
// arrives from the stream with that reference; one of ours (our department,
// when the budget subsystem is on) arrives without one (FC4Streams.h,
// OurObjectRef), so it is taken here.
uint64_t ResolveRef(void*, uint64_t t, uint32_t iid) {
    cIGZUnknown* u = FromToken(t);
    if (u == nullptr) return 0;
    if (fc4::IsOurObject(u)) {
        u->AddRef();
        return t;
    }
    if (iid != kIID_cSC4PipeOccupant && iid != kIID_cISC4Occupant) return t;
    cISC4Occupant* occ = Query<cISC4Occupant>(u, kIID_cISC4Occupant);
    FC4_RELEASE(u);
    return TokenOf(occ);
}

// ---- occupants ----------------------------------------------------------------------------------

uint32_t OccupantType(void*, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::PlumbingOccupants, "plumbing occupant GetType (0)")) return 0;
    return static_cast<uint32_t>(OccOf(occ)->GetType());  // slot 7
}

bool OccupantProperty(void*, uint64_t occ, uint32_t id, fc4_props* out) {
    if (occ == 0 || !Gate(fc4::Sw::PlumbingOccupants, "plumbing occupant property read (absent)")) return false;
    cISCPropertyHolder* h = OccOf(occ)->AsPropertyHolder();  // borrowed, slot 6
    return h != nullptr && fc4::props::CopyToCore(h, id, out);
}

void SetOccupantProperty(void*, uint64_t occ, uint32_t id, uint16_t memType, bool isArray, const void* data,
                         size_t count) {
    if (occ == 0 || !Gate(fc4::Sw::PlumbingOccupants, "plumbing occupant property write")) return;
    if (!Gate(fc4::Sw::PropertyWrites, "plumbing occupant property write")) return;
    cISCPropertyHolder* h = OccOf(occ)->AsPropertyHolder();
    if (h != nullptr && !fc4::props::SetTyped(h, id, memType, isArray, data, count)) {
        FC4_LOGV("plumbing: property 0x%08X (type 0x%X) not written to occupant %p", id, memType, FromToken(occ));
    }
}

void OccupantPosition(void*, uint64_t occ, float* xyz) {
    if (xyz == nullptr || occ == 0 || !Gate(fc4::Sw::PlumbingOccupants, "plumbing occupant GetPosition")) return;
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);  // slot 8
    xyz[0] = v.fX;
    xyz[1] = v.fY;
    xyz[2] = v.fZ;
}

void OccupantBoundingBox(void*, uint64_t occ, float* mn, float* mx) {
    if (mn == nullptr || mx == nullptr || occ == 0 ||
        !Gate(fc4::Sw::PlumbingOccupants, "plumbing occupant GetBoundingBox")) {
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

void OccupantBoundingCells(void*, uint64_t occ, fc4_cell_rect* out) {
    if (out == nullptr || occ == 0 ||
        !Gate(fc4::Sw::PlumbingOccupants, "plumbing occupant GetBoundingCityCells")) {
        return;
    }
    SC4Rect<long> r;
    OccOf(occ)->GetBoundingCityCells(r);  // slot 11
    out->x1 = static_cast<int32_t>(r.topLeftX);
    out->z1 = static_cast<int32_t>(r.topLeftY);
    out->x2 = static_cast<int32_t>(r.bottomRightX);
    out->z2 = static_cast<int32_t>(r.bottomRightY);
}

bool BuildingType(void*, uint64_t occ, uint32_t* out) {
    if (!Gate(fc4::Sw::PlumbingOccupants, "plumbing building type (none)")) return false;
    return WithQi<cISC4BuildingOccupant>(occ, kIID_cISC4BuildingOccupant, [&](cISC4BuildingOccupant* b) {
        if (out != nullptr) *out = static_cast<uint32_t>(b->GetBuildingType());  // slot 4
    });
}

// GetOccupantsByStandardCityCells(list, &cell, &cell, 0x278128A0, −1) (§5.3,
// occupant manager compiled slot 41: the buildings in one cell). [abi] It
// fills an SC4List (A) with one reference each, kept until the outermost call
// into the core returns.
void BuildingsInCell(void* u, int32_t x, int32_t z, fc4_token_callback cb, void* ctx) {
    if (cb == nullptr || !Gate(fc4::Sw::PlumbingOccupants, "plumbing buildings in a cell (none)")) return;
    cISC4OccupantManager* om = Self(u)->OccupantManager();
    if (om == nullptr) return;
    auto list = std::make_shared<SC4List<cISC4Occupant*>>();
    const SC4Point<int> cell(x, z);
    om->GetOccupantsByStandardCityCells(*list, &cell, &cell, kOccupantTypeBuilding, 0xFFFFFFFFu);
    for (cISC4Occupant* o : *list) {
        if (o != nullptr) cb(ctx, TokenOf(o));
    }
    Self(u)->Hold(std::move(list));
}

// ---- network occupants (pipes) ------------------------------------------------------------------
//
// Every call goes through QueryInterface for the pipe's class ID, which both
// identifies a pipe and yields its base: the cISC4NetworkOccupant sub-object
// is at +0 (docs/abi/direct-calls.md). §12.2 item 4: a replacement QIs rather
// than applying the original's +4 adjustment.

bool IsNetworkOccupant(void*, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::PlumbingPipes, "plumbing pipe QueryInterface (not a pipe)")) return false;
    cISC4NetworkOccupant* n = Query<cISC4NetworkOccupant>(FromToken(occ), kIID_cSC4PipeOccupant);
    const bool ok = n != nullptr;
    Drop(n);
    return ok;
}

bool HasNetworkFlag(void*, uint64_t occ, uint32_t flag) {
    if (!Gate(fc4::Sw::PlumbingPipes, "plumbing HasNetworkFlag (false)")) return false;
    bool v = false;
    WithQi<cISC4NetworkOccupant>(occ, kIID_cSC4PipeOccupant,
                                 [&](cISC4NetworkOccupant* n) { v = n->HasNetworkFlag(flag); });  // slot 8
    return v;
}

uint32_t NetworkFlags(void*, uint64_t occ) {
    if (!Gate(fc4::Sw::PlumbingPipes, "plumbing GetNetworkFlag (0)")) return 0;
    uint32_t v = 0;
    WithQi<cISC4NetworkOccupant>(occ, kIID_cSC4PipeOccupant,
                                 [&](cISC4NetworkOccupant* n) { v = n->GetNetworkFlag(); });  // slot 12
    return v;
}

void SetNetworkFlag(void*, uint64_t occ, uint32_t flag) {
    if (!Gate(fc4::Sw::PlumbingPipes, "plumbing SetNetworkFlag")) return;
    WithQi<cISC4NetworkOccupant>(occ, kIID_cSC4PipeOccupant,
                                 [&](cISC4NetworkOccupant* n) { n->SetNetworkFlag(flag); });  // slot 10
}

void ClearNetworkFlag(void*, uint64_t occ, uint32_t flag) {
    if (!Gate(fc4::Sw::PlumbingPipes, "plumbing ClearNetworkFlag")) return;
    WithQi<cISC4NetworkOccupant>(occ, kIID_cSC4PipeOccupant,
                                 [&](cISC4NetworkOccupant* n) { n->ClearNetworkFlag(flag); });  // slot 11
}

bool ConnectsToNeighbor(void*, uint64_t occ, uint32_t dir, uint32_t networkType) {
    if (!Gate(fc4::Sw::PlumbingPipes, "plumbing ConnectsToNeighbor (false)")) return false;
    bool v = false;
    WithQi<cISC4NetworkOccupant>(
        occ, kIID_cSC4PipeOccupant,
        [&](cISC4NetworkOccupant* n) {
            v = n->ConnectsToNeighbor(static_cast<int32_t>(dir),
                                      static_cast<cISC4NetworkOccupant::eNetworkType>(networkType));
        });  // slot 47
    return v;
}

void OccupiedCell(void*, uint64_t occ, int32_t* x, int32_t* z) {
    uint32_t cx = 0, cz = 0;
    if (Gate(fc4::Sw::PlumbingPipes, "plumbing GetOccupiedCell")) {
        WithQi<cISC4NetworkOccupant>(occ, kIID_cSC4PipeOccupant,
                                     [&](cISC4NetworkOccupant* n) { n->GetOccupiedCell(cx, cz); });  // slot 59
    }
    if (x != nullptr) *x = static_cast<int32_t>(cx);
    if (z != nullptr) *z = static_cast<int32_t>(cz);
}

// cSC4PipeOccupant::SetWateredState(bool), 0x00648540 (§12.2 item 3: the only
// caller, non-virtual, so a plug-in shim or nothing). `this` is the pipe's
// base, which QueryInterface for the pipe's class ID returns.
using SetWateredStateFn = void(__fastcall*)(void* self, void* edx, uint32_t watered);

void SetPipeWatered(void*, uint64_t occ, bool watered) {
    if (!Gate(fc4::Sw::PlumbingDirectCalls, "plumbing pipe SetWateredState (direct call)")) return;
    const uintptr_t va = fc4::compat::DirectCall(fc4::compat::Direct::PipeSetWateredState);
    if (va == 0) return;  // logged once by compat
    // The QI result is the base; the pointer is kept only for the call.
    cISC4NetworkOccupant* base = Query<cISC4NetworkOccupant>(FromToken(occ), kIID_cSC4PipeOccupant);
    if (base == nullptr) {
        fc4::Log("plumbing: SetWateredState skipped: %p is not a game pipe", FromToken(occ));
        return;
    }
    // __thiscall through __fastcall: ECX = this, EDX unused, stack (watered),
    // callee pops 4. Treat the return as void (it tail-calls slot 100).
    reinterpret_cast<SetWateredStateFn>(va)(base, nullptr, watered ? 1u : 0u);
    Drop(base);
}

// ---- lots ---------------------------------------------------------------------------------------

uint64_t LotAt(void* u, int32_t x, int32_t z) {
    if (!Gate(fc4::Sw::PlumbingLots, "plumbing GetLot (none)")) return 0;
    cISC4LotManager* lm = Self(u)->LotManager();
    return lm ? TokenOf(lm->GetLot(x, z, false)) : 0;  // compiled slot 6, borrowed
}

uint64_t OccupantLot(void* u, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::PlumbingLots, "plumbing GetOccupantLot (none)")) return 0;
    cISC4LotManager* lm = Self(u)->LotManager();
    return lm ? TokenOf(lm->GetOccupantLot(OccOf(occ))) : 0;  // slot 14, borrowed
}

void LotRect(void*, uint64_t lot, fc4_cell_rect* out) {
    if (out == nullptr || lot == 0 || !Gate(fc4::Sw::PlumbingLots, "plumbing GetBoundingRect")) return;
    SC4Rect<int32_t> r;
    LotOf(lot)->GetBoundingRect(r);  // slot 14, inclusive cells
    out->x1 = r.topLeftX;
    out->z1 = r.topLeftY;
    out->x2 = r.bottomRightX;
    out->z2 = r.bottomRightY;
}

bool LotNeedsPower(void*, uint64_t lot) {
    if (lot == 0 || !Gate(fc4::Sw::PlumbingLots, "plumbing GetNeedsPower (false)")) return false;
    return LotOf(lot)->GetNeedsPower();  // slot 65
}

bool LotHasPower(void*, uint64_t lot) {
    if (lot == 0 || !Gate(fc4::Sw::PlumbingLots, "plumbing GetHasPower (false)")) return false;
    return LotOf(lot)->GetHasPower();  // slot 66
}

void SetLotCondition(void*, uint64_t lot, uint8_t condition) {
    if (lot != 0 && Gate(fc4::Sw::PlumbingLots, "plumbing SetCondition")) {
        LotOf(lot)->SetCondition(condition);  // slot 36
    }
}

// ---- terrain ------------------------------------------------------------------------------------

bool TerrainCellCounts(void* u, int32_t* x, int32_t* z) {
    cISTETerrain* t = Self(u)->Terrain();
    if (t == nullptr || !Gate(fc4::Sw::PlumbingTerrain, "plumbing terrain cell counts (no terrain)")) return false;
    if (x != nullptr) *x = static_cast<int32_t>(t->CellCountX());  // slot 7
    if (z != nullptr) *z = static_cast<int32_t>(t->CellCountZ());  // slot 8
    return true;
}

bool CellHasWater(void* u, int32_t x, int32_t z) {
    cISTETerrain* t = Self(u)->Terrain();
    if (t == nullptr || !Gate(fc4::Sw::PlumbingTerrain, "plumbing CellHasWater (false)")) return false;
    return t->CellHasWater(static_cast<uint32_t>(x), static_cast<uint32_t>(z));  // slot 34
}

void RedisplayTerrain(void* u, int32_t minX, int32_t minZ, int32_t maxX, int32_t maxZ) {
    cISTETerrain* t = Self(u)->Terrain();
    if (t == nullptr || !Gate(fc4::Sw::PlumbingTerrain, "plumbing RedisplayTerrain")) return;
    const SC4Point<int32_t> mn(minX, minZ), mx(maxX, maxZ);
    t->RedisplayTerrain(false, false, mn, mx, 0);  // slot 69
}

// ---- disasters ----------------------------------------------------------------------------------

// GetDisasterLayer()->GetDisasterManager(0x4A1332E8) and, if it exists,
// AttemptDisasterStart(x, z). Returns whether the manager existed (§7.1).
bool AttemptPipeBurst(void* u, int32_t x, int32_t z) {
    if (!Gate(fc4::Sw::PlumbingDisaster, "plumbing AttemptDisasterStart (no manager)")) return false;
    cISC4City* c = Self(u)->City();
    cISC4DisasterLayer* layer = c ? c->GetDisasterLayer() : nullptr;  // city slot 72
    cISC4DisasterManager* m = layer ? layer->GetDisasterManager(kPipeBurstManager) : nullptr;  // layer slot 7
    if (m == nullptr) return false;
    Drop(m->AttemptDisasterStart(x, z));  // slot 6
    return true;
}

// ---- neighbours ---------------------------------------------------------------------------------

cFC4ISC4NeighborsSimulator* Neighbors(void* u) {
    cISC4NeighborsSimulator* n = Self(u)->Neighbors();
    return n ? fc4::AsGame(n) : nullptr;
}

// FindDeals(type, list&) (slot 30). [abi] A list, cleared first, one reference
// each (allocators.md): an SC4List, held until the outermost call into the core
// returns.
void FindDeals(void* u, uint32_t type, fc4_token_callback cb, void* ctx) {
    if (cb == nullptr || !Gate(fc4::Sw::PlumbingNeighbors, "plumbing FindDeals (none)")) return;
    cFC4ISC4NeighborsSimulator* n = Neighbors(u);
    if (n == nullptr) return;
    auto list = std::make_shared<SC4List<cISC4NeighborDeal*>>();
    n->FindDealsOfType(static_cast<int32_t>(type), *list);
    for (cISC4NeighborDeal* d : *list) {
        if (d != nullptr) cb(ctx, TokenOf(d));
    }
    Self(u)->Hold(std::move(list));
}

uint32_t DealType(void*, uint64_t deal) { return deal ? static_cast<uint32_t>(DealOf(deal)->GetType()) : 0; }

uint64_t DealCity(void*, uint64_t deal) {
    // Slot 7, borrowed.
    return deal ? TokenOf(DealOf(deal)->GetCityOnOtherSideOfConnection()) : 0;
}

int32_t DealAmount(void*, uint64_t deal) { return deal ? DealOf(deal)->GetAmount() : 0; }

int32_t DealAmountUsed(void*, uint64_t deal) { return deal ? DealOf(deal)->GetAmountUsed() : 0; }

void SetDealAmountUsed(void*, uint64_t deal, int32_t used) {
    if (deal != 0 && Gate(fc4::Sw::PlumbingNeighbors, "plumbing SetAmountUsed")) DealOf(deal)->SetAmountUsed(used);
}

void SetDealFlags(void*, uint64_t deal, uint32_t flags, bool on) {
    if (deal != 0 && Gate(fc4::Sw::PlumbingNeighbors, "plumbing deal SetFlags")) {
        DealOf(deal)->SetFlags(static_cast<int32_t>(flags), on);
    }
}

template <class Fill>
void Connections(void* u, fc4_token_callback cb, void* ctx, Fill fill) {
    if (cb == nullptr || !Gate(fc4::Sw::PlumbingNeighbors, "plumbing neighbour connections (none)")) return;
    cFC4ISC4NeighborsSimulator* n = Neighbors(u);
    if (n == nullptr) return;
    auto list = std::make_shared<SC4List<cISC4NeighborConnection*>>();
    fill(n, *list);
    for (cISC4NeighborConnection* c : *list) {
        if (c != nullptr) cb(ctx, TokenOf(c));
    }
    Self(u)->Hold(std::move(list));
}

using ConnType = cISC4NeighborsSimulator::ConnectionType;

// [abi] Slots 17 / 18: A lists, cleared first, one reference each.
void ConnectionsOfType(void* u, uint32_t type, fc4_token_callback cb, void* ctx) {
    Connections(u, cb, ctx, [&](cFC4ISC4NeighborsSimulator* n, SC4List<cISC4NeighborConnection*>& l) {
        n->GetConnectionsOfType(static_cast<ConnType>(type), l);
    });
}

void ConnectionsOfTypeToCity(void* u, uint32_t type, uint64_t city, fc4_token_callback cb, void* ctx) {
    Connections(u, cb, ctx, [&](cFC4ISC4NeighborsSimulator* n, SC4List<cISC4NeighborConnection*>& l) {
        n->GetConnectionsOfTypeToCity(static_cast<ConnType>(type),
                                      reinterpret_cast<cISC4RegionalCity const*>(FromToken(city)), l);
    });
}

uint64_t FindUnvisitedConnection(void* u, uint32_t type, uint64_t city) {
    if (!Gate(fc4::Sw::PlumbingNeighbors, "plumbing FindUnvisitedConnection (none)")) return 0;
    cFC4ISC4NeighborsSimulator* n = Neighbors(u);
    if (n == nullptr) return 0;
    return TokenOf(n->FindUnvisitedConnection(static_cast<ConnType>(type),
                                              reinterpret_cast<cISC4RegionalCity const*>(FromToken(city))));
}

uint64_t ConnectionOnOtherSide(void* u, uint64_t conn) {
    if (conn == 0 || !Gate(fc4::Sw::PlumbingNeighbors, "plumbing FindConnectionOnOtherSide (none)")) return 0;
    cFC4ISC4NeighborsSimulator* n = Neighbors(u);
    return n ? TokenOf(n->FindConnectionOnOtherSideOf(ConnOf(conn))) : 0;
}

void ConnectionCell(void*, uint64_t conn, int32_t* x, int32_t* z) {
    int32_t cx = 0, cz = 0;
    if (conn != 0) ConnOf(conn)->GetConnectionCell(cx, cz);  // slot 6
    if (x != nullptr) *x = cx;
    if (z != nullptr) *z = cz;
}

void SetConnectionVisited(void*, uint64_t conn, bool on) {
    if (conn != 0 && Gate(fc4::Sw::PlumbingNeighbors, "plumbing SetVisitedThisCycle")) {
        ConnOf(conn)->SetVisitedThisCycle(on);  // slot 12
    }
}

bool ConnectionIsActive(void*, uint64_t conn) { return conn != 0 && ConnOf(conn)->GetIsActive(); }

void SetConnectionActive(void*, uint64_t conn, bool on) {
    if (conn != 0 && Gate(fc4::Sw::PlumbingNeighbors, "plumbing SetIsActive")) ConnOf(conn)->SetIsActive(on);
}

void MarkDealsDirty(void* u) {
    if (!Gate(fc4::Sw::PlumbingNeighbors, "plumbing MarkDealsDirty")) return;
    if (cFC4ISC4NeighborsSimulator* n = Neighbors(u)) n->MarkDealsDirty();  // slot 31
}

// ---- budget -------------------------------------------------------------------------------------

cFC4ISC4BudgetSimulator* Budget(void* u) {
    cISC4BudgetSimulator* b = Self(u)->Budget();
    return b ? fc4::AsGame(b) : nullptr;
}

uint64_t CreateDepartment(void* u, uint32_t id, uint32_t group) {
    if (!Gate(fc4::Sw::PlumbingBudget, "plumbing CreateDepartmentBudget (none)")) return 0;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    return b ? TokenOf(b->CreateDepartmentBudget(id, group)) : 0;  // slot 25, borrowed
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

float DepartmentFunding(void*, uint64_t dept) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    SC4Percentage* p = d ? d->GetFundingPercentage() : nullptr;  // slot 10
    return p ? p->percentage : 1.0f;
}

uint64_t CreateLineItem(void*, uint64_t dept, uint32_t line, bool locallyFunded) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    return d ? TokenOf(d->CreateLineItem(line, locallyFunded)) : 0;  // slot 17
}

void SetLineItemName(void*, uint64_t item, uint32_t group, uint32_t instance) {
    if (item != 0) ItemOf(item)->SetName(group, instance);  // slot 7
}

void RemoveLineItem(void*, uint64_t dept, uint32_t line) {
    if (cFC4ISC4DepartmentBudget* d = DeptOf(dept)) d->RemoveLineItem(line);  // slot 19
}

uint64_t DepartmentById(void* u, uint32_t id) {
    if (!Gate(fc4::Sw::PlumbingBudget, "plumbing GetDepartmentBudget (none)")) return 0;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    return b ? TokenOf(b->GetDepartmentBudget(id)) : 0;  // slot 28, borrowed
}

uint64_t LineItem(void*, uint64_t dept, uint32_t line) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    return d ? TokenOf(d->GetLineItem(line)) : 0;  // slot 20, borrowed
}

uint64_t CreateLineItemForBuildingType(void*, uint64_t dept, uint32_t line, bool locallyFunded) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    return d ? TokenOf(d->CreateLineItemForBuildingType(line, locallyFunded)) : 0;  // slot 18
}

void SetLineItemDisplayFlag(void*, uint64_t item, uint32_t flag, bool on) {
    if (item != 0) ItemOf(item)->SetDisplayFlag(flag, on);  // slot 18
}

void AddToLineItemExpenses(void*, uint64_t item, int64_t delta) {
    if (item != 0) ItemOf(item)->AddToFullExpenses(delta);  // slot 13
}

void SetLineItemExpenses(void*, uint64_t item, int64_t value) {
    if (item != 0) ItemOf(item)->SetFullExpenses(value);  // slot 12
}

int64_t LineItemSecondaryInfo(void*, uint64_t item) {
    return item ? ItemOf(item)->GetSecondaryInfoField() : 0;  // slot 8, EDX:EAX
}

void SetLineItemSecondaryInfo(void*, uint64_t item, int64_t v) {
    if (item != 0) ItemOf(item)->SetSecondaryInfoField(v);  // slot 9
}

cISCPropertyHolder* HolderOf(uint64_t occ) { return occ ? OccOf(occ)->AsPropertyHolder() : nullptr; }

bool BudgetItemForPurpose(void* u, uint64_t occ, uint32_t purpose, fc4_plumbing_budget_item* out) {
    if (!Gate(fc4::Sw::PlumbingBudget, "plumbing GetBudgetItemForPurpose (none)")) return false;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    cISCPropertyHolder* h = HolderOf(occ);
    if (b == nullptr || h == nullptr) return false;
    cFC4ISC4BudgetSimulator::BudgetItem item{};
    if (!b->GetBudgetItemForPurpose(h, purpose, item)) return false;  // slot 51
    if (out != nullptr) {
        out->department = item.departmentID;
        out->line = item.lineItem;
        out->cost = item.cost;
        out->index = static_cast<uint32_t>(item.index);
        out->reserved = 0;
    }
    return true;
}

void ChangeBudgetItemLine(void* u, uint64_t occ, uint32_t index, uint32_t line) {
    cFC4ISC4BudgetSimulator* b = Budget(u);
    cISCPropertyHolder* h = HolderOf(occ);
    if (b != nullptr && h != nullptr) b->ChangeBudgetItemLine(h, index, line);  // slot 52
}

void ChangeBudgetItemCost(void* u, uint64_t occ, uint32_t index, int64_t cost) {
    cFC4ISC4BudgetSimulator* b = Budget(u);
    cISCPropertyHolder* h = HolderOf(occ);
    if (b != nullptr && h != nullptr) b->ChangeBudgetItemCost(h, index, cost);  // slot 53
}

uint64_t LineItemFromBudgetItem(void* u, const fc4_plumbing_budget_item* in) {
    cFC4ISC4BudgetSimulator* b = Budget(u);
    if (b == nullptr || in == nullptr) return 0;
    cFC4ISC4BudgetSimulator::BudgetItem item{};
    item.departmentID = in->department;
    item.lineItem = in->line;
    item.cost = in->cost;
    item.index = static_cast<int32_t>(in->index);
    return TokenOf(b->GetLineItemFromBudgetItem(item));  // slot 57, borrowed
}

// IsLocallyFundedObjectInDepartment (department slot 28) and, when it is, the
// local funding (slot 26 returns a pointer): the fraction the department funds
// `occ` locally with (§10.1 FindWaterBuildings).
bool LocalFunding(void*, uint64_t dept, uint64_t occ, float* out) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    cISCPropertyHolder* h = HolderOf(occ);
    if (d == nullptr || h == nullptr) return false;
    if (!d->IsLocallyFundedObjectInDepartment(h, fc4::plumbing::kBudgetPurpose)) return false;
    SC4Percentage const* p = d->GetLocalFundingPercentPtr(h, fc4::plumbing::kBudgetPurpose);
    if (out != nullptr) *out = p ? p->percentage : 0.0f;
    return true;
}

fc4_plumbing_host MakeHost(cFC4PlumbingSimulator* self, const fc4::HostGridContext* grids) {
    fc4_plumbing_host h{};
    h.struct_size = sizeof(h);
    h.user = self;
    h.grids = fc4::MakeGridApi(grids);
    // city
    h.cell_counts = &CellCounts;
    h.position_to_cell = &PositionToCell;
    h.tuning = &Tuning;
    h.wall_clock_seed = &WallClockSeed;
    // simulator, messages, history
    h.add_agent = &AddAgent;
    h.remove_agent = &RemoveAgent;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.post = &Post;
    h.record_history = &RecordHistory;
    h.cumulative_factor = &CumulativeFactor;
    h.water_value = &WaterValue;
    // occupants
    h.occupant_type = &OccupantType;
    h.occupant_property = &OccupantProperty;
    h.set_occupant_property = &SetOccupantProperty;
    h.occupant_position = &OccupantPosition;
    h.occupant_bounding_box = &OccupantBoundingBox;
    h.occupant_bounding_cells = &OccupantBoundingCells;
    h.building_type = &BuildingType;
    h.buildings_in_cell = &BuildingsInCell;
    // network occupants (pipes)
    h.is_network_occupant = &IsNetworkOccupant;
    h.has_network_flag = &HasNetworkFlag;
    h.network_flags = &NetworkFlags;
    h.set_network_flag = &SetNetworkFlag;
    h.clear_network_flag = &ClearNetworkFlag;
    h.connects_to_neighbor = &ConnectsToNeighbor;
    h.occupied_cell = &OccupiedCell;
    h.set_pipe_watered = &SetPipeWatered;
    // lots
    h.lot_at = &LotAt;
    h.occupant_lot = &OccupantLot;
    h.lot_rect = &LotRect;
    h.lot_needs_power = &LotNeedsPower;
    h.lot_has_power = &LotHasPower;
    h.set_lot_condition = &SetLotCondition;
    // terrain
    h.terrain_cell_counts = &TerrainCellCounts;
    h.cell_has_water = &CellHasWater;
    h.redisplay_terrain = &RedisplayTerrain;
    // disasters
    h.attempt_pipe_burst = &AttemptPipeBurst;
    // neighbours
    h.find_deals = &FindDeals;
    h.deal_type = &DealType;
    h.deal_city = &DealCity;
    h.deal_amount = &DealAmount;
    h.deal_amount_used = &DealAmountUsed;
    h.set_deal_amount_used = &SetDealAmountUsed;
    h.set_deal_flags = &SetDealFlags;
    h.connections_of_type = &ConnectionsOfType;
    h.connections_of_type_to_city = &ConnectionsOfTypeToCity;
    h.find_unvisited_connection = &FindUnvisitedConnection;
    h.connection_on_other_side = &ConnectionOnOtherSide;
    h.connection_cell = &ConnectionCell;
    h.set_connection_visited = &SetConnectionVisited;
    h.connection_is_active = &ConnectionIsActive;
    h.set_connection_active = &SetConnectionActive;
    h.mark_deals_dirty = &MarkDealsDirty;
    // budget
    h.create_department = &CreateDepartment;
    h.set_department_name = &SetDepartmentName;
    h.set_department_max_funding = &SetDepartmentMaxFunding;
    h.create_line_item = &CreateLineItem;
    h.set_line_item_name = &SetLineItemName;
    h.remove_line_item = &RemoveLineItem;
    h.department_funding = &DepartmentFunding;
    h.department_by_id = &DepartmentById;
    h.line_item = &LineItem;
    h.create_line_item_for_building_type = &CreateLineItemForBuildingType;
    h.set_line_item_display_flag = &SetLineItemDisplayFlag;
    h.add_to_line_item_expenses = &AddToLineItemExpenses;
    h.set_line_item_expenses = &SetLineItemExpenses;
    h.line_item_secondary_info = &LineItemSecondaryInfo;
    h.set_line_item_secondary_info = &SetLineItemSecondaryInfo;
    h.budget_item_for_purpose = &BudgetItemForPurpose;
    h.change_budget_item_line = &ChangeBudgetItemLine;
    h.change_budget_item_cost = &ChangeBudgetItemCost;
    h.line_item_from_budget_item = &LineItemFromBudgetItem;
    h.local_funding = &LocalFunding;
    // references
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.resolve_ref = &ResolveRef;
    return h;
}

// Slot 28's callback: append with one reference for the caller's list. The
// core hands out BORROWED producer tokens, so this AddRefs (§10.1).
void AppendProducer(void* ctx, uint64_t t) {
    auto* list = static_cast<SC4List<cISC4Occupant*>*>(ctx);
    cISC4Occupant* o = OccOf(t);
    if (o == nullptr) return;
    FC4_ADDREF_FOR_CALLER(o);
    list->push_back(o);
}

}  // namespace

// ---- object -------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4PlumbingSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4PlumbingSimulator*>(new cFC4PlumbingSimulator()));
}

cFC4PlumbingSimulator::cFC4PlumbingSimulator() {
    host_ = MakeHost(this, &gridCtx_);
    core_ = fc4_plumbing_create(&host_, SelfToken());
    if (core_ == nullptr) fc4::Log("cFC4PlumbingSimulator: core creation failed");
    fc4::Log("cFC4PlumbingSimulator(%p) created, core %p", static_cast<void*>(this), static_cast<void*>(core_));
}

cFC4PlumbingSimulator::~cFC4PlumbingSimulator() {
    fc4_plumbing_destroy(core_);
    core_ = nullptr;
    fc4::Log("cFC4PlumbingSimulator destroyed");
}

bool cFC4PlumbingSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_IID_cISC4PlumbingSimulator:
            *ppvObj = static_cast<cFC4ISC4PlumbingSimulator*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        case GZIID_cIGZUnknown:
            // §1: cIGZUnknown is answered with the primary interface.
            *ppvObj = static_cast<cIGZUnknown*>(static_cast<cFC4ISC4PlumbingSimulator*>(this));
            break;
        default:
            // §1: false, and the out pointer is left untouched.
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4PlumbingSimulator::AddRef() { return ++refCount_; }

// §1: at a count of 1 (or below) Shutdown, destroy, return 0.
uint32_t cFC4PlumbingSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

uint64_t cFC4PlumbingSimulator::Occ(cISC4Occupant* occupant) const { return TokenOf(occupant); }

// §4.1 step 2: each service only while its member is still null.
void cFC4PlumbingSimulator::AcquireServices() {
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
    if (city_ == nullptr) {
        fc4::Log("plumbing: no city; services not acquired");
        return;
    }
    auto take = [](auto*& member, auto* value) {
        if (member == nullptr && value != nullptr) {
            member = value;
            FC4_ADDREF(member);
        }
    };
    take(occupantManager_, city_->GetOccupantManager());
    take(lotManager_, city_->GetLotManager());
    take(pollution_, city_->GetPollutionSimulator());
    take(ordinance_, city_->GetOrdinanceSimulator());
    take(neighbors_, city_->GetNeighborsSimulator());
    take(budget_, city_->GetBudgetSimulator());
    take(terrain_, city_->GetTerrain());
}

// §4.2 step 2.
void cFC4PlumbingSimulator::ReleaseServices() {
    auto drop = [](auto*& member) {
        if (member != nullptr) FC4_RELEASE(member);
        member = nullptr;
    };
    drop(terrain_);
    drop(budget_);
    drop(neighbors_);
    drop(ordinance_);
    drop(pollution_);
    drop(lotManager_);
    drop(occupantManager_);
    drop(city_);
}

void cFC4PlumbingSimulator::Subscribe(const uint32_t* ids, size_t count, bool add) {
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

void cFC4PlumbingSimulator::LogState(const char* what) {
    fc4_plumbing_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_plumbing_get_state(core_, &st)) return;
    fc4::Log("plumbing %s: init=%u grids=%u enabled=%u state=%u cells=%dx%d pass=%d producers=%u pipes=%u "
             "distressed=%u burst=%u polluted=%u averagePollution=%u wateredCells=%u buildingCells=%u "
             "pipeCondition=%.3f demand=%lld cost=%lld produced=%lld consumed=%lld imported=%lld exported=%lld "
             "department=%p published=%p reentrant=%u",
             what, st.initialized, st.grids_initialized, st.enabled, st.state, st.count_x, st.count_z, st.pass_counter,
             st.producers, st.num_pipes, st.num_distressed, st.num_burst, st.num_polluted,
             st.average_water_pollution, st.watered_cells, st.building_cells, st.pipe_condition,
             static_cast<long long>(st.demand), static_cast<long long>(st.total_water_cost),
             static_cast<long long>(st.last_produced), static_cast<long long>(st.last_consumed),
             static_cast<long long>(st.last_imported), static_cast<long long>(st.last_exported),
             static_cast<void*>(FromToken(st.department)), static_cast<void*>(FromToken(st.published_grid)),
             st.reentrant_calls);
    lastState_ = st.state;
}

// ---- lifecycle (§4) -----------------------------------------------------------------------------

bool cFC4PlumbingSimulator::Init() {
    FC4_TRACE("PlumbingSimulator::Init");
    AcquireServices();
    CoreCall call(this);
    fc4_plumbing_init(core_);
    LogState("Init");
    return true;  // §4.1: always
}

bool cFC4PlumbingSimulator::Shutdown() {
    FC4_TRACE("PlumbingSimulator::Shutdown");
    // Keep this object alive while the simulator and the message server drop
    // their references (unless this already is the final Release).
    if (!destroying_) AddRef();
    {
        CoreCall call(this);
        fc4_plumbing_shutdown(core_);
    }
    held_.clear();
    ReleaseServices();
    fc4::Log("plumbing Shutdown (%u passes)", passes_);
    if (!destroying_) Release();
    return true;  // §4.2: always
}

// ---- queries (§10.1) ----------------------------------------------------------------------------

uint8_t cFC4PlumbingSimulator::GetSupplyValue(int32_t x, int32_t z) {
    return fc4_plumbing_supply_value(core_, x, z);  // slot 5
}

bool cFC4PlumbingSimulator::GetSupplyValueMapPtr(cISC4SimGrid<uint8_t>*& grid) {
    grid = GetWateredGrid();  // no AddRef
    return true;              // §10.4: no meaningful return; a replacement returns true
}

cISC4SimGrid<uint8_t>* cFC4PlumbingSimulator::GetWateredGrid() {
    return GridOf(fc4_plumbing_watered_grid(core_));  // slot 7, borrowed
}

bool cFC4PlumbingSimulator::AreWateredBuildingsInCell(int32_t x, int32_t z) {
    return fc4_plumbing_are_watered_buildings_in_cell(core_, x, z);  // slot 8, full EAX
}

bool cFC4PlumbingSimulator::IsBuildingWatered(cISC4Occupant* occupant) {
    return fc4_plumbing_is_building_watered(core_, Occ(occupant));  // slot 9
}

uint32_t cFC4PlumbingSimulator::GetWaterProduction(cISC4Occupant* occupant) {
    return fc4_plumbing_water_production(core_, Occ(occupant));  // slot 10
}

bool cFC4PlumbingSimulator::IsCellWatered(int32_t x, int32_t z) {
    return fc4_plumbing_is_cell_watered(core_, x, z);  // slot 11
}

uint32_t cFC4PlumbingSimulator::GetNumWaterSources() {
    return static_cast<uint32_t>(fc4_plumbing_slot_value(core_, 12));
}

uint32_t cFC4PlumbingSimulator::GetNumPollutedWaterSources() {
    return static_cast<uint32_t>(fc4_plumbing_slot_value(core_, 13));
}

// Slots 14-17 and 23: the game returns only the low 32 bits in EAX (§10.4).
int32_t cFC4PlumbingSimulator::GetWaterProductionCapacity() {
    return static_cast<int32_t>(fc4_plumbing_slot_value(core_, 14));
}

int32_t cFC4PlumbingSimulator::GetWaterConsumed() {
    return static_cast<int32_t>(fc4_plumbing_slot_value(core_, 15));
}

int32_t cFC4PlumbingSimulator::GetWaterImported() { return static_cast<int32_t>(fc4_plumbing_slot_value(core_, 16)); }

int32_t cFC4PlumbingSimulator::GetWaterExported() { return static_cast<int32_t>(fc4_plumbing_slot_value(core_, 17)); }

uint32_t cFC4PlumbingSimulator::GetNumPipes() { return static_cast<uint32_t>(fc4_plumbing_slot_value(core_, 18)); }

uint32_t cFC4PlumbingSimulator::GetNumDistressedPipes() {
    return static_cast<uint32_t>(fc4_plumbing_slot_value(core_, 19));
}

uint32_t cFC4PlumbingSimulator::GetNumBurstPipes() {
    return static_cast<uint32_t>(fc4_plumbing_slot_value(core_, 20));
}

uint32_t cFC4PlumbingSimulator::GetNumWateredBuildings() {
    return static_cast<uint32_t>(fc4_plumbing_slot_value(core_, 21));
}

uint32_t cFC4PlumbingSimulator::GetNumUnwateredBuildings() {
    return static_cast<uint32_t>(fc4_plumbing_slot_value(core_, 22));
}

int32_t cFC4PlumbingSimulator::GetWaterDemand() { return static_cast<int32_t>(fc4_plumbing_slot_value(core_, 23)); }

bool cFC4PlumbingSimulator::GetWaterProducerInfo(cISC4Occupant* producer, uint32_t* currentCapacity,
                                                 uint32_t* maxCapacity, SC4Percentage* efficiency,
                                                 bool* shutDownByPollution, uint32_t* waterUsed) {
    fc4_plumbing_producer_info info{};
    if (!fc4_plumbing_producer_info(core_, Occ(producer), &info)) return false;  // §10.1 slot 24: AL = 0
    // Each non-null pointer receives its value.
    if (currentCapacity != nullptr) *currentCapacity = info.production;
    if (maxCapacity != nullptr) *maxCapacity = info.max_capacity;
    if (efficiency != nullptr) efficiency->percentage = info.efficiency;
    if (shutDownByPollution != nullptr) *shutDownByPollution = info.polluted;
    if (waterUsed != nullptr) *waterUsed = info.used;
    return true;
}

bool cFC4PlumbingSimulator::GetWaterProducerConditionPercent(cISC4Occupant* producer, SC4Percentage& conditionPercent) {
    return fc4_plumbing_producer_condition_percent(core_, Occ(producer), &conditionPercent.percentage);
}

bool cFC4PlumbingSimulator::GetOccupantMonthlyUpkeepCost(cISC4Occupant* occupant, int64_t& monthlyUpkeepCost) {
    return fc4_plumbing_occupant_monthly_upkeep_cost(core_, Occ(occupant), &monthlyUpkeepCost);
}

bool cFC4PlumbingSimulator::FindWaterBuildings(cISC4Occupant** lowestFunded, SC4Percentage& funding,
                                               cISC4Occupant** worstCondition, long& conditionPercent) {
    fc4_plumbing_found f{};
    if (!fc4_plumbing_find_water_buildings(core_, &f)) return true;
    // Outputs are written only when found and when the pointer is non-null;
    // the occupants are AddRef'd for the caller (§10.1).
    if (f.has_lowest_funded != 0) {
        funding.percentage = f.lowest_funding;
        if (lowestFunded != nullptr) {
            cISC4Occupant* o = OccOf(f.lowest_funded);
            FC4_ADDREF_FOR_CALLER(o);
            *lowestFunded = o;
        }
    }
    if (f.has_worst_condition != 0) {
        conditionPercent = f.condition_percent;
        if (worstCondition != nullptr) {
            cISC4Occupant* o = OccOf(f.worst_condition);
            FC4_ADDREF_FOR_CALLER(o);
            *worstCondition = o;
        }
    }
    return true;  // §10.4: no meaningful return; a replacement returns true
}

bool cFC4PlumbingSimulator::GetWaterProducers(SC4List<cISC4Occupant*>& list) {
    CoreCall call(this);
    fc4_plumbing_water_producers(core_, &AppendProducer, &list);
    return true;  // §10.4: no meaningful return; a replacement returns true
}

// ---- messages (§5) ------------------------------------------------------------------------------

bool cFC4PlumbingSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    CoreCall call(this);
    fc4_plumbing_message m{};
    m.msg_type = pMessage->GetType();
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    cISC4Occupant* occ = nullptr;
    cISC4NeighborDeal* deal = nullptr;
    if (s != nullptr) {
        m.data1 = static_cast<uint32_t>(s->GetData1());
        m.data2 = static_cast<uint32_t>(s->GetData2());
        m.data3 = static_cast<uint32_t>(s->GetData3());
        if (m.msg_type == kMsgInsertOccupant || m.msg_type == kMsgRemoveOccupant) {
            // §5.1: the occupant is void1, known by its cISC4Occupant identity.
            occ = Query<cISC4Occupant>(static_cast<cIGZUnknown*>(s->GetVoid1()), kIID_cISC4Occupant);
            m.occupant = TokenOf(occ);
        } else if (m.msg_type == kMsgDealStatus && m.data1 == 0) {
            // §5.1: GetIGZUnknown() queried for cISC4NeighborDeal; released
            // after the call.
            deal = Query<cISC4NeighborDeal>(s->GetIGZUnknown(), kIID_cISC4NeighborDeal);
            m.deal = TokenOf(deal);
        }
    }
    // A message delivered while another call into the core is running is
    // queued by the core and handled when that call returns: keep its objects
    // alive until then.
    if (depth_ > 1) {
        cIGZUnknown* keep[] = {reinterpret_cast<cIGZUnknown*>(occ), reinterpret_cast<cIGZUnknown*>(deal)};
        for (cIGZUnknown* k : keep) {
            if (k == nullptr) continue;
            k->AddRef();
            Hold(std::shared_ptr<void>(k, [](void* p) { static_cast<cIGZUnknown*>(p)->Release(); }));
        }
    }
    const bool r = fc4_plumbing_do_message(core_, &m);
    Drop(deal);
    Drop(occ);
    Drop(s);
    if (m.msg_type == kMsgSimNewMonth) {
        ++passes_;
        LogState("month");
    } else {
        fc4_plumbing_state st{};
        st.struct_size = sizeof(st);
        if (fc4_plumbing_get_state(core_, &st) && st.state != lastState_) {
            FC4_LOGV("plumbing: state %u -> %u", lastState_, st.state);
            lastState_ = st.state;
        }
    }
    return r;  // §5.1: true, except the day and idle messages (DoIdle's result)
}

// ---- persistence (§9) ---------------------------------------------------------------------------

bool cFC4PlumbingSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("PlumbingSimulator::Write");
    CoreCall call(this);
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_plumbing_write(core_, a.Get());
    fc4::Log("plumbing Write -> %d", ok ? 1 : 0);
    return ok;
}

bool cFC4PlumbingSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("PlumbingSimulator::Read");
    CoreCall call(this);
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_plumbing_read(core_, a.Get());
    fc4::Log("plumbing Read -> %d", ok ? 1 : 0);
    LogState("Read");
    return ok;
}

uint32_t cFC4PlumbingSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4PlumbingSimulator; }
