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

// The prop developer's host callbacks (fc4_prop_host, prop-developer.md §8.4):
// the game's services as the core asks for them. Tokens: lots
// (cISC4Lot*), network lots (cISC4NetworkLot*), props (cISC4PropOccupant*,
// the interface the record's references are read with), requesters
// (cISC4PropRequester*), managers (the interfaces of §7). A read-only
// context (the shadow) changes nothing in the game: writes are skipped or
// handed to its hooks.

#include "cFC4PropDeveloper.h"

#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4LotDevInterfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4WinInterfaces.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "SC4Rect.h"
#include "cIGZDate.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC424HourClock.h"
#include "cISC4City.h"
#include "cISC4LightingManager.h"
#include "cISC4Lot.h"
#include "cISC4LotConfiguration.h"
#include "cISC4LotDeveloper.h"
#include "cISC4LotManager.h"
#include "cISC4NetworkLot.h"
#include "cISC4NetworkLotManager.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4PowerSimulator.h"
#include "cISC4PropManager.h"
#include "cISC4PropOccupant.h"
#include "cISC4Simulator.h"
#include "cISC4View3DWin.h"
#include "cISC4ZoneManager.h"
#include "cISCPropertyHolder.h"
#include "cISTETerrain.h"
#include "cRZBaseString.h"
#include "cS3DVector3.h"
#include "cSC4LotConfigurationObject.h"

#include <cstring>
#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace fc4 {

const uint32_t kPropDevSubscriptions[13] = {0x99EF1142, 0x99EF1143, 0xC9DA96E8, 0xC9DA96E9, 0x44778231,
                                            0x44778234, 0x26D31EC1, 0xE8BDA2E7, 0x8A07AE00, 0xAA375FE8,
                                            0xCA5E5027, 0x278E6003, 0x678E6E3A};

}  // namespace fc4

namespace {

constexpr uint32_t kIID_cIGZSerializable = 0xE4FDA3D4;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4LotManager = 0x681905BD;
constexpr uint32_t kIID_cISC4PropManager = 0x28326F3E;
constexpr uint32_t kIID_cISC4NetworkLotManager = 0x89A5BA3F;
constexpr uint32_t kMsgLotStateProps = 0xCA5E5027;
constexpr uint32_t kPropRequesterProperty = 0x89A1C16C;
constexpr uint32_t kTypeProp = 0xA823821E;
constexpr uint32_t kTypeNetwork = 0xC772BF98;
constexpr uint32_t kMsgInsert = 0x99EF1142;
constexpr uint32_t kMsgRemove = 0x99EF1143;
constexpr uint32_t kMsgLotState = 0xE8BDA2E7;
// The animation-tick manager (lot-developer.md §6.7.8).
constexpr uint32_t kTickPriority = 0xC0;
// The renderer's slot 9 GetLightingManager (docs/abi/vtables.tsv
// cSC43DRender::vftable slot 9, medium confidence; question PV15).
class cFC4PropRender : public cIGZUnknown {
public:
    virtual bool Init() = 0;                         // 3
    virtual bool Shutdown() = 0;                     // 4
    virtual bool SetMapScaleFactor(int32_t f) = 0;   // 5
    virtual bool SetGraphicSystem(void* g) = 0;      // 6
    virtual bool SetGDriver(void* d) = 0;            // 7
    virtual void* GetViewUtilities() = 0;            // 8
    virtual cISC4LightingManager* GetLightingManager() = 0;  // 9 (borrowed)
};

fc4::PropDevContext* Ctx(void* u) { return static_cast<fc4::PropDevContext*>(u); }
bool Writes(void* u) { return !Ctx(u)->readOnly; }

cISC4City* City() { return fc4::propcity::AppCity(); }

cISC4Lot* SdkLot(uint64_t t) { return reinterpret_cast<cISC4Lot*>(FromToken(t)); }
cFC4ISC4Lot* LotOf(uint64_t t) { return t ? fc4::AsGame(SdkLot(t)) : nullptr; }
cISC4PropOccupant* PropOf(uint64_t t) { return reinterpret_cast<cISC4PropOccupant*>(FromToken(t)); }
cISC4Occupant* OccOfProp(uint64_t t) { return t ? PropOf(t)->AsOccupant() : nullptr; }  // prop slot 3
cFC4ISC4PropRequester* ReqOf(uint64_t t) { return reinterpret_cast<cFC4ISC4PropRequester*>(FromToken(t)); }
cISC4NetworkLot* NetLotOf(uint64_t t) { return reinterpret_cast<cISC4NetworkLot*>(FromToken(t)); }

cISC4LotManager* LotManager() {
    cISC4City* c = City();
    return c ? c->GetLotManager() : nullptr;
}
cISC4PropManager* PropManager() {
    cISC4City* c = City();
    return c ? c->GetPropManager() : nullptr;
}
cISC4NetworkLotManager* NetLotManager() {
    cISC4City* c = City();
    return c ? c->GetNetworkLotManager() : nullptr;  // city slot 42
}

void Sink(const std::vector<uint64_t>& v, fc4_u64s_sink sink, void* ctx) {
    if (sink != nullptr && !v.empty()) sink(ctx, v.data(), v.size());
}

// A game occupant list as prop tokens (cISC4PropOccupant); the list's
// references and the query results go to the pool.
std::vector<uint64_t> PropsOf(void* u, SC4List<cISC4Occupant*>& list, const char* how) {
    std::vector<uint64_t> out;
    for (cISC4Occupant*& o : list) {
        if (o == nullptr) continue;
        FC4_ACQUIRED(o, how);
        Ctx(u)->pool.Adopt(o);
        const uint64_t t = fc4::PropToken(Ctx(u)->pool, o);
        if (t != 0) out.push_back(t);
        o = nullptr;
    }
    return out;
}

// ---- city, simulator, messages --------------------------------------------------------------------

void CityCells(void*, int32_t* x, int32_t* z) {
    cISC4City* c = City();
    if (x != nullptr) *x = c ? static_cast<int32_t>(c->CellCountX()) : 0;  // slot 93
    if (z != nullptr) *z = c ? static_cast<int32_t>(c->CellCountZ()) : 0;  // slot 94
}

void CellWidth(void*, float* x, float* z) {
    cISC4City* c = City();
    if (x != nullptr) *x = c ? c->CellWidthX() : 16.0f;  // slot 91
    if (z != nullptr) *z = c ? c->CellWidthZ() : 16.0f;  // slot 92
}

// §3.1 step 1: the lot, prop and network lot managers as the record refers
// to them (§7 IIDs), one reference each kept by the context.
void Managers(void* u, fc4_prop_ref* out3) {
    fc4::PropDevContext* c = Ctx(u);
    cISC4City* city = City();
    cIGZUnknown* sdk[3] = {nullptr, nullptr, nullptr};
    if (city != nullptr) {
        sdk[0] = city->GetLotManager();
        sdk[1] = city->GetPropManager();
        sdk[2] = city->GetNetworkLotManager();
    }
    const uint32_t iids[3] = {kIID_cISC4LotManager, kIID_cISC4PropManager, kIID_cISC4NetworkLotManager};
    for (int i = 0; i < 3; ++i) {
        fc4::propcity::Drop(c->managers[i]);
        c->managers[i] = nullptr;
        cIGZUnknown* m = nullptr;
        void* p = nullptr;
        if (sdk[i] != nullptr && sdk[i]->QueryInterface(iids[i], &p) && p != nullptr) {
            m = static_cast<cIGZUnknown*>(p);
            if (!fc4::IsOurObject(m)) FC4_ACQUIRED(m, "QueryInterface");
        } else if (sdk[i] != nullptr) {
            m = sdk[i];  // [choice] the city's pointer when it does not answer the record's IID
            fc4::propcity::Hold(m);
        }
        c->managers[i] = m;
        uint32_t cls = 0;
        void* s = nullptr;
        if (m != nullptr && m->QueryInterface(kIID_cIGZSerializable, &s) && s != nullptr) {
            cIGZUnknown* su = static_cast<cIGZUnknown*>(s);
            if (!fc4::IsOurObject(su)) FC4_ACQUIRED(su, "QueryInterface");
            cls = static_cast<cIGZSerializable*>(s)->GetGZCLSID();
            fc4::propcity::Drop(su);
        }
        if (out3 != nullptr) out3[i] = fc4_prop_ref{TokenOf(m), cls, 0};
    }
}

uint64_t CreateWealthRequester(void* u) {
    fc4::PropDevContext* c = Ctx(u);
    if (c->readOnly) return 0;  // [choice] the shadow never makes one (its record has wealthAdded = 1)
    auto* w = new cFC4WealthRequester();
    w->SetOwner(c->owner);
    w->AddRef();  // the context's reference
    cIGZUnknown* primary = static_cast<cIGZUnknown*>(static_cast<cFC4ISC4PropRequester*>(w));
    c->wealth.push_back(primary);
    return TokenOf(static_cast<cFC4ISC4PropRequester*>(w));
}

void AddAgent(void* u, uint32_t level, const char* name, uint32_t flags) {
    fc4::PropDevContext* c = Ctx(u);
    if (!Writes(u) || c->target == nullptr || name == nullptr) return;
    cISC4City* city = City();
    cISC4Simulator* s = city ? city->GetSimulator() : nullptr;
    if (s == nullptr) {
        fc4::Log("propdev: no simulator; the agent is not registered");
        return;
    }
    const cRZBaseString n(name);
    fc4::AsGame(s)->AddAgent(c->target, level, n, flags);
    c->agentRegistered = true;
}

void RemoveAgents(void* u, const char*) {
    fc4::PropDevContext* c = Ctx(u);
    if (!c->agentRegistered) return;
    cISC4City* city = City();
    cISC4Simulator* s = city ? city->GetSimulator() : nullptr;
    if (s != nullptr) fc4::AsGame(s)->RemoveAgentAllLevels(c->target);  // slot 20
    c->agentRegistered = false;
}

void Subscribe(void* u, uint32_t type) {
    fc4::PropDevContext* c = Ctx(u);
    if (!Writes(u) || c->target == nullptr) return;
    if (c->messageServer == nullptr) {
        cIGZMessageServer2Ptr ms;
        cIGZMessageServer2* p = ms;
        if (p == nullptr) return;
        fc4::propcity::Hold(p);
        c->messageServer = p;
    }
    if (c->messageServer->AddNotification(c->target, type)) c->subscriptions.push_back(type);
}

void UnsubscribeAll(void* u) {
    fc4::PropDevContext* c = Ctx(u);
    if (c->messageServer != nullptr && c->target != nullptr) {
        for (uint32_t t : c->subscriptions) c->messageServer->RemoveNotification(c->target, t);
    }
    c->subscriptions.clear();
    fc4::propcity::Drop(c->messageServer);
    c->messageServer = nullptr;
}

cFC4ISC4AnimationTickManager* TickManager() {
    cIGZFrameWork* fw = RZGetFrameWork();
    void* p = nullptr;
    if (fw == nullptr || !fw->GetSystemService(kFC4_SID_AnimationTickManager, kFC4_IID_AnimationTickManager, &p) ||
        p == nullptr) {
        return nullptr;
    }
    auto* m = static_cast<cFC4ISC4AnimationTickManager*>(p);
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(m), "GetSystemService");
    return m;
}

void RegisterTick(void* u, uint32_t priority) {
    fc4::PropDevContext* c = Ctx(u);
    if (!Writes(u) || c->target == nullptr) return;
    cFC4ISC4AnimationTickManager* m = TickManager();
    if (m == nullptr) {
        fc4::Log("propdev: no animation-tick manager; the night batches do not run");
        return;
    }
    m->AddTickableObject(c->target, priority);  // slot 5
    c->tickRegistered = true;
    FC4_RELEASE(static_cast<cIGZUnknown*>(m));
}

void UnregisterTick(void* u) {
    fc4::PropDevContext* c = Ctx(u);
    if (!c->tickRegistered || c->target == nullptr) return;
    cFC4ISC4AnimationTickManager* m = TickManager();
    if (m != nullptr) {
        m->RemoveTickableObject(c->target, kTickPriority);  // slot 6
        FC4_RELEASE(static_cast<cIGZUnknown*>(m));
    }
    c->tickRegistered = false;
}

// §4.12: 0xCA5E5027, data1 x, data2 z, data3 = entered state 3; posted
// (queued) as a standard message.
void PostLotStateProps(void* u, int32_t x, int32_t z, bool entered3) {
    if (!Writes(u)) return;
    cIGZMessageServer2Ptr ms;
    cIGZMessageServer2* p = ms;
    if (p == nullptr) return;
    cFC4Message2Standard* msg = FC4MakeMessage(kMsgLotStateProps, x, z, entered3 ? 1 : 0);
    if (msg == nullptr) return;
    p->MessagePost(msg, false);
    msg->Release();
    ++Ctx(u)->posts;
}

void HoldToken(void* u, uint64_t t) {
    if (t == 0) return;
    fc4::propcity::Hold(FromToken(t));
    ++Ctx(u)->holds;
}

void ReleaseToken(void* u, uint64_t t) {
    if (t == 0) return;
    fc4::propcity::DropHeld(FromToken(t));
    if (Ctx(u)->holds > 0) --Ctx(u)->holds;
}

// ---- host requesters (crime, pollution; §4.1) -----------------------------------------------------

int32_t RequesterShare(void*, uint64_t r, const fc4_cell_rect* rect) {
    if (r == 0 || rect == nullptr) return 0;
    SC4Rect<int32_t> cells;
    cells.topLeftX = rect->x1;
    cells.topLeftY = rect->z1;
    cells.bottomRightX = rect->x2;
    cells.bottomRightY = rect->z2;
    return ReqOf(r)->GetRequestedShare(cells);  // slot 3
}

uint32_t RequesterSatisfaction(void*, uint64_t r) { return r ? ReqOf(r)->GetRequesterSatisfactionId() : 0; }  // 4

bool RequesterCustom(void*, uint64_t r) { return r != 0 && ReqOf(r)->UsesCustomPropSelector(); }  // slot 5

uint32_t RequesterSelect(void* u, uint64_t r, const uint32_t* ids, size_t count, int32_t x, int32_t z) {
    fc4::PropDevContext* c = Ctx(u);
    if (c->onSelect != nullptr) return c->onSelect(c->hookCtx, r, ids, count, x, z);
    if (c->readOnly || r == 0 || !fc4::StlNodeAllocator::Available()) return 0;
    fc4::GameVector<uint32_t> v;
    if (!v.reserve(count == 0 ? 1 : count)) return 0;
    for (size_t i = 0; i < count; ++i) v.push_back(ids[i]);
    return ReqOf(r)->SelectProp(v, x, z);  // slot 6
}

// ---- lots and network lots ------------------------------------------------------------------------

uint64_t LotAt(void*, int32_t x, int32_t z, bool originOnly) {
    cISC4LotManager* lm = LotManager();
    return lm ? TokenOf(lm->GetLot(x, z, originOnly)) : 0;  // slot 6, borrowed
}

bool LotLivable(void*, uint64_t lot) { return lot != 0 && LotOf(lot)->IsInLivableState(); }  // slot 37
uint32_t LotBuildingType(void*, uint64_t lot) { return lot ? LotOf(lot)->GetBuildingType(false) : 0; }  // slot 46
float LotOccupancy(void*, uint64_t lot) { return lot ? LotOf(lot)->GetOccupancy().percentage : 0.0f; }  // slot 61

bool LotConfigObjects(void*, uint64_t lot, fc4_lot_cfg_objects_sink sink, void* ctx) {
    cISC4LotConfiguration* cfg = lot ? LotOf(lot)->GetLotConfiguration() : nullptr;  // slot 9
    if (cfg == nullptr) return false;
    fc4::propcity::SinkCfgObjects(cfg->GetLotConfigurationObjectArray(), sink, ctx);  // cfg slot 23
    return true;
}

uint32_t LotWealth(void*, uint64_t lot) { return lot ? LotOf(lot)->GetOccupantWealth() : 0; }  // slot 41

void LotLocation(void*, uint64_t lot, int32_t* x, int32_t* z) {
    int32_t lx = 0, lz = 0;
    if (lot != 0) LotOf(lot)->GetLocation(lx, lz);  // slot 21
    if (x != nullptr) *x = lx;
    if (z != nullptr) *z = lz;
}

void LotSize(void*, uint64_t lot, int32_t* x, int32_t* z) {
    uint8_t sx = 0, sz = 0;
    if (lot != 0) LotOf(lot)->GetSize(sx, sz);  // slot 22
    if (x != nullptr) *x = sx;
    if (z != nullptr) *z = sz;
}

uint32_t LotState(void*, uint64_t lot) { return lot ? static_cast<uint32_t>(LotOf(lot)->GetState()) : 0; }  // 33

uint32_t ZoneType(void*, int32_t x, int32_t z) {
    cISC4City* c = City();
    cISC4ZoneManager* zm = c ? c->GetZoneManager() : nullptr;
    return zm ? static_cast<uint32_t>(fc4::AsGame(zm)->GetZoneType(x, z)) : 0;  // slot 12
}

uint64_t NetworkLotAt(void*, int32_t x, int32_t z) {
    cISC4NetworkLotManager* nlm = NetLotManager();
    return nlm ? TokenOf(nlm->GetNetworkLot(x, z)) : 0;  // slot 5, borrowed
}

bool NetworkLotObjects(void*, uint64_t nl, fc4_lot_cfg_objects_sink sink, void* ctx) {
    if (nl == 0) return false;
    SC4Vector<cSC4LotConfigurationObject>* v = NetLotOf(nl)->GetLotConfigurationObjectArray();  // network lot slot 5
    if (v == nullptr) return false;
    fc4::propcity::SinkCfgObjects(v, sink, ctx);
    return true;
}

uint32_t NetworkWealth(void*, int32_t x, int32_t z) {
    cISC4NetworkLotManager* nlm = NetLotManager();
    return nlm ? static_cast<uint32_t>(nlm->GetWealthForNetwork(x, z)) : 0;  // slot 10
}

// ---- the prop manager -----------------------------------------------------------------------------

void LotProps(void* u, uint64_t lot, fc4_u64s_sink sink, void* ctx) {
    cISC4PropManager* pm = PropManager();
    if (pm == nullptr || lot == 0) return;
    SC4List<cISC4Occupant*> list;
    pm->GetLotProps(list, SdkLot(lot));  // slot 24
    Sink(PropsOf(u, list, "GetLotProps list entry"), sink, ctx);
}

void StreetProps(void* u, int32_t x, int32_t z, fc4_u64s_sink sink, void* ctx) {
    cISC4PropManager* pm = PropManager();
    if (pm == nullptr) return;
    SC4List<cISC4Occupant*> list;
    pm->GetStreetProps(list, x, z);  // slot 31
    Sink(PropsOf(u, list, "GetStreetProps list entry"), sink, ctx);
}

void StreetPropsInRect(void* u, const fc4_cell_rect* rect, fc4_u64s_sink sink, void* ctx) {
    cISC4PropManager* pm = PropManager();
    if (pm == nullptr || rect == nullptr) return;
    SC4Rect<int32_t> r;
    r.topLeftX = rect->x1;
    r.topLeftY = rect->z1;
    r.bottomRightX = rect->x2;
    r.bottomRightY = rect->z2;
    SC4List<cISC4Occupant*> list;
    pm->GetStreetProps(list, r);  // slot 30
    Sink(PropsOf(u, list, "GetStreetProps list entry"), sink, ctx);
}

void RemoveProp(void* u, uint64_t prop) {
    fc4::PropDevContext* c = Ctx(u);
    ++c->removes;
    if (c->onRemove != nullptr) {
        c->onRemove(c->hookCtx, prop);
        return;
    }
    if (c->readOnly || prop == 0) return;
    cISC4PropManager* pm = PropManager();
    if (pm != nullptr) pm->RemovePropA(OccOfProp(prop));  // slot 29
}

bool PropTypeValid(void*, uint32_t id) {
    cISC4PropManager* pm = PropManager();
    return pm != nullptr && pm->IsPropTypeValid(id);  // slot 8
}

// Slot 9 GetPropTypesByFamily(vector, family). [abi inferred] (question
// PV14) a std::vector<uint32_t> with allocator B, as the game's vector
// out-parameters are (allocators.md: the rule of thumb; this method is not
// in its table).
void PropsByFamily(void*, uint32_t family, fc4_u32s_sink sink, void* ctx) {
    cISC4PropManager* pm = PropManager();
    if (pm == nullptr || sink == nullptr || !fc4::StlNodeAllocator::Available()) return;
    fc4::GameVector<uint32_t> v;
    pm->GetPropTypesByFamily(reinterpret_cast<SC4Vector<uint32_t>&>(v), family);
    std::vector<uint32_t> copy(v.begin(), v.end());
    if (!copy.empty()) sink(ctx, copy.data(), copy.size());
}

// The prop profile (slot 13): the SDK's PropProfile, u32 requester
// satisfaction at +0x10 and u8 wealth at +0x14 (crime.md §11.6 layout).
static_assert(sizeof(cISC4PropOccupant::PropProfile) == 28, "PropProfile layout");
uint32_t PropSatisfaction(void*, uint32_t id) {
    cISC4PropManager* pm = PropManager();
    if (pm == nullptr) return 0;
    const cISC4PropOccupant::PropProfile& p = pm->GetPropProfile(id);
    uint32_t v = 0;
    std::memcpy(&v, reinterpret_cast<const uint8_t*>(&p) + 16, sizeof(v));
    return v;
}

uint8_t PropWealth(void*, uint32_t id) {
    cISC4PropManager* pm = PropManager();
    if (pm == nullptr) return 0;
    const cISC4PropOccupant::PropProfile& p = pm->GetPropProfile(id);
    return reinterpret_cast<const uint8_t*>(&p)[20];
}

uint64_t CreateLotProp(void* u, uint64_t lot, uint32_t index, uint32_t id) {
    fc4::PropDevContext* c = Ctx(u);
    ++c->creates;
    if (c->onCreate != nullptr) return c->onCreate(c->hookCtx, lot, false, index, id);
    if (c->readOnly || lot == 0) return 0;
    cISC4City* city = City();
    cISC4LotDeveloper* ld = city ? city->GetLotDeveloper() : nullptr;
    if (ld == nullptr) return 0;
    cISC4Occupant* occ = fc4::AsGame(ld)->CreateLotProp(SdkLot(lot), index, id);  // slot 63, borrowed
    return fc4::PropToken(c->pool, occ);
}

uint64_t CreateNetworkProp(void* u, uint64_t nl, uint32_t index, uint32_t id) {
    fc4::PropDevContext* c = Ctx(u);
    ++c->creates;
    if (c->onCreate != nullptr) return c->onCreate(c->hookCtx, nl, true, index, id);
    if (c->readOnly || nl == 0) return 0;
    cISC4NetworkLotManager* nlm = NetLotManager();
    if (nlm == nullptr) return 0;
    cISC4Occupant* occ = nlm->CreateProp(NetLotOf(nl), index, id);  // slot 8, borrowed
    return fc4::PropToken(c->pool, occ);
}

// ---- prop occupants -------------------------------------------------------------------------------

uint32_t PropClass(void* u, uint64_t prop) {
    if (prop == 0) return 0;
    auto* s = static_cast<cIGZSerializable*>(fc4::propcity::QueryInto(Ctx(u)->pool, FromToken(prop),
                                                                       kIID_cIGZSerializable));
    return s ? s->GetGZCLSID() : 0;
}

fc4_props* PropProperties(void*, uint64_t prop) {
    cISC4Occupant* o = OccOfProp(prop);
    cISCPropertyHolder* h = o ? o->AsPropertyHolder() : nullptr;  // occupant slot 6
    return h ? fc4::props::ToCoreAll(h) : nullptr;
}

bool PropLotTag(void*, uint64_t prop, uint32_t* tag) {
    uint32_t t = 0;
    cISC4Occupant* o = OccOfProp(prop);
    const bool has = o != nullptr && o->GetLotTag(t);  // slot 27
    if (tag != nullptr) *tag = has ? t : 0;
    return has;
}

void SetPropRequester(void* u, uint64_t prop, uint32_t r) {
    if (!Writes(u)) return;
    cISC4Occupant* o = OccOfProp(prop);
    if (o != nullptr) fc4::propcity::SetU32Property(o->AsPropertyHolder(), kPropRequesterProperty, r);
}

uint32_t PropType(void*, uint64_t prop) { return prop ? PropOf(prop)->GetPropType() : 0; }  // slot 7
uint32_t PropState(void*, uint64_t prop) { return prop ? PropOf(prop)->GetState() : 0; }   // slot 12

void SetPropState(void* u, uint64_t prop, uint32_t state) {
    ++Ctx(u)->stateWrites;
    if (Writes(u) && prop != 0) PropOf(prop)->SetState(state);  // slot 11
}

void SetPropTimer(void* u, uint64_t prop, uint32_t mask, bool on) {
    ++Ctx(u)->timerWrites;
    if (Writes(u) && prop != 0) {
        PropOf(prop)->SetPropTimerState(static_cast<cISC4PropOccupant::tPropTimerMask>(mask), on);  // slot 21
    }
}

void SetRandomChance(void* u, uint64_t prop, uint8_t chance) {
    if (Writes(u) && prop != 0) PropOf(prop)->SetRandomChance(chance);  // slot 17
}

void SetTimeOfDayRange(void* u, uint64_t prop, float start, float end) {
    if (Writes(u) && prop != 0) PropOf(prop)->SetTimeOfDayRange(start, end);  // slot 13
}

void TimeOfDayRange(void*, uint64_t prop, float* start, float* end) {
    float s = 0.0f, e = 0.0f;
    if (prop != 0) PropOf(prop)->GetTimeOfDayRange(s, e);  // slot 14
    if (start != nullptr) *start = s;
    if (end != nullptr) *end = e;
}

void SetDateRange(void* u, uint64_t prop, uint32_t date, uint32_t duration, uint8_t flag) {
    if (Writes(u) && prop != 0) PropOf(prop)->SetSimulatorDateRange(date, duration, flag, 0);  // slot 15
}

// §3.3 Day: TickSimulatorDate(simulator.GetSimDate()) (prop slot 16): the
// simulator's own date object (the core's day number is that date's).
void TickDate(void* u, uint64_t prop, uint32_t) {
    if (!Writes(u) || prop == 0) return;
    cISC4City* c = City();
    cISC4Simulator* s = c ? c->GetSimulator() : nullptr;
    cFC4IGZDate* d = s ? fc4::AsGame(s)->GetSimDate() : nullptr;  // slot 7, borrowed
    if (d != nullptr) PropOf(prop)->TickSimulatorDate(*reinterpret_cast<cIGZDate*>(d));
}

void PropPosition(void*, uint64_t prop, float* out3) {
    cS3DVector3 v(0.0f, 0.0f, 0.0f);
    cISC4Occupant* o = OccOfProp(prop);
    if (o != nullptr) o->GetPosition(&v);  // slot 8
    if (out3 != nullptr) {
        out3[0] = v.fX;
        out3[1] = v.fY;
        out3[2] = v.fZ;
    }
}

// §4.11: SetPosition (slot 9), then occupant manager slot 24 (flag 1).
void SetPropPosition(void* u, uint64_t prop, const float* p3) {
    if (!Writes(u) || p3 == nullptr) return;
    cISC4Occupant* o = OccOfProp(prop);
    if (o == nullptr) return;
    const cS3DVector3 v(p3[0], p3[1], p3[2]);
    o->SetPosition(&v);
    cISC4City* c = City();
    cISC4OccupantManager* om = c ? c->GetOccupantManager() : nullptr;
    if (om != nullptr) om->MoveOccupant(o, true);  // slot 24
}

void SetPropFlag(void* u, uint64_t prop, uint32_t flag, bool on) {
    if (!Writes(u)) return;
    cISC4Occupant* o = OccOfProp(prop);
    if (o == nullptr) return;
    if (on) {
        o->SetFlag(flag);  // slot 29
    } else {
        o->ClearFlag(flag);  // slot 31
    }
}

void NetworkFirstCell(void*, uint64_t occ, int32_t* x, int32_t* z) {
    SC4Rect<long> r{};
    cISC4Occupant* o = reinterpret_cast<cISC4Occupant*>(FromToken(occ));
    if (o != nullptr) o->GetBoundingCityCells(r);  // slot 11
    if (x != nullptr) *x = static_cast<int32_t>(r.topLeftX);
    if (z != nullptr) *z = static_cast<int32_t>(r.topLeftY);
}

void PositionToCell(void*, float x, float z, int32_t* cx, int32_t* cz) {
    int ix = 0, iz = 0;
    if (cISC4City* c = City()) c->PositionToCell(x, z, ix, iz);  // slot 95
    if (cx != nullptr) *cx = ix;
    if (cz != nullptr) *cz = iz;
}

// ---- others ---------------------------------------------------------------------------------------

bool IsPowered(void*, int32_t x, int32_t z) {
    cISC4City* c = City();
    cISC4PowerSimulator* p = c ? c->GetPowerSimulator() : nullptr;
    return p != nullptr && fc4::AsGame(p)->IsPowered(x, z);  // slot 37
}

bool ViewAvailable(void*) {
    cISC4View3DWin* v = fc4::propcity::View3D();
    if (v == nullptr) return false;
    fc4::propcity::Drop(reinterpret_cast<cIGZUnknown*>(v));
    return true;
}

bool ViewTriggerState(void*, uint32_t trigger) {
    cISC4View3DWin* v = fc4::propcity::View3D();
    if (v == nullptr) return false;
    const bool on = v->GetGlobalEffectTriggerState(trigger) != 0;  // slot 46
    fc4::propcity::Drop(reinterpret_cast<cIGZUnknown*>(v));
    return on;
}

uint32_t SimDate(void*) {
    cISC4City* c = City();
    cISC4Simulator* s = c ? c->GetSimulator() : nullptr;
    return s ? static_cast<uint32_t>(fc4::AsGame(s)->GetSimDateNumber()) : 0;  // slot 8
}

bool ClockTime(void*, uint32_t* hour, uint32_t* minute) {
    cISC4City* c = City();
    cISC424HourClock* k = c ? c->Get24HourClock() : nullptr;
    if (k == nullptr) return false;
    if (hour != nullptr) *hour = k->GetHour();
    if (minute != nullptr) *minute = k->GetMinute();
    return true;
}

uint32_t VisualTimeMode(void*) {
    cISC4City* c = City();
    cISC424HourClock* k = c ? c->Get24HourClock() : nullptr;
    return k ? static_cast<uint32_t>(k->GetVisualTimeMode()) : 0;  // slot 6
}

struct NightRead {
    cISC4View3DWin* view;
    bool night;
    bool read;
};

void ReadNight(void* p) {
    auto* r = static_cast<NightRead*>(p);
    auto* render = reinterpret_cast<cFC4PropRender*>(r->view->GetRenderer());  // view slot 4, borrowed
    cISC4LightingManager* lm = render ? render->GetLightingManager() : nullptr;  // renderer slot 9, borrowed
    if (lm == nullptr) return;
    r->night = lm->IsNight();  // slot 9
    r->read = true;
}

// §8.5 (PV15): both pointer-return getters are borrowed. Only View3D's
// acquired view reference is released; renderer and manager are not released.
bool LightingIsNight(void* u) {
    Ctx(u)->lightingAvailable = false;
    cISC4View3DWin* v = fc4::propcity::View3D();
    if (v == nullptr) return false;
    NightRead r{v, false, false};
    if (!fc4::SehGuard("propdev lighting manager IsNight", &ReadNight, &r)) r.read = false;
    fc4::propcity::Drop(reinterpret_cast<cIGZUnknown*>(v));
    Ctx(u)->lightingAvailable = r.read;
    static bool noted = false;
    if (!r.read && !noted) {
        noted = true;
        fc4::Log("propdev: lighting input unavailable (PV15); shadow cannot count it as exact");
    }
    return r.night;
}

void TerrainHeights(void*, float x, float z, float* altitude, float* water) {
    cISC4City* c = City();
    cISTETerrain* t = c ? c->GetTerrain() : nullptr;
    if (altitude != nullptr) *altitude = t ? t->GetAltitude(x, z) : 0.0f;  // slot 13
    if (water != nullptr) *water = t ? t->GetWaterLevel(x, z) : 0.0f;      // slot 30
}

}  // namespace

// ---- shared helpers ---------------------------------------------------------------------------------

const char* fc4::PropDevScriptLightingHour(int hour) {
    auto* view = propcity::View3D();
    if (!view) return "no 3D view";
    // prop-developer section 8.5 (PV15): borrowed renderer and manager.
    auto* render = reinterpret_cast<cFC4PropRender*>(view->GetRenderer());
    auto* lighting = render ? render->GetLightingManager() : nullptr;
    const char* result = "no lighting manager";
    if (lighting) {
        // lighting.md sections 2.1/2.2 and SDK slot 6: by-value float hours,
        // bool return. Let the genuine producer post dusk/dawn; never send a
        // synthetic propdev message or guess a function signature.
        const bool changed = lighting->SetTimeOfDay(static_cast<float>(hour));
        Log("script lighting producer: SetTimeOfDay hour=%d changed=%u night=%u", hour,
            changed ? 1u : 0u, lighting->IsNight() ? 1u : 0u);
        result = "native SetTimeOfDay delivered";
    }
    propcity::Drop(reinterpret_cast<cIGZUnknown*>(view));
    return result;
}

uint64_t fc4::PropToken(propcity::CallPool& pool, cIGZUnknown* u) {
    return TokenOf(propcity::QueryInto(pool, u, kFC4_IID_cISC4PropOccupant));
}

void fc4::AfterPropRead(PropDevContext* ctx, fc4_prop_developer* core, bool ok,
                        const std::vector<propcity::RecordingIStream::Taken>& refs) {
    if (!ok || core == nullptr) {
        propcity::DropRefs(refs);
        return;
    }
    // The managers (§7: re-acquired at Init).
    for (const propcity::RecordingIStream::Taken& t : refs) {
        if (t.iid == kIID_cISC4LotManager || t.iid == kIID_cISC4PropManager || t.iid == kIID_cISC4NetworkLotManager) {
            propcity::Drop(FromToken(t.token));
        }
    }
    // The sign list (§2: not held).
    const size_t n = fc4_prop_developer_list(core, FC4_PROP_LIST_SIGNS, nullptr, 0);
    std::vector<fc4_prop_ref> signs(n);
    if (n != 0) fc4_prop_developer_list(core, FC4_PROP_LIST_SIGNS, signs.data(), n);
    for (const fc4_prop_ref& s : signs) propcity::DropHeld(FromToken(s.token));
    // The wealth requesters: the host's from now on.
    const size_t r = fc4_prop_developer_list(core, FC4_PROP_LIST_REQUESTERS, nullptr, 0);
    std::vector<fc4_prop_ref> reqs(r);
    if (r != 0) fc4_prop_developer_list(core, FC4_PROP_LIST_REQUESTERS, reqs.data(), r);
    for (const fc4_prop_ref& q : reqs) {
        if (q.token != 0 && q.class_id == kFC4_CLSID_cSC4WealthRequester) ctx->wealth.push_back(FromToken(q.token));
    }
}

void fc4::ReleasePropContext(PropDevContext* ctx) {
    ctx->pool.Drain();
    for (cIGZUnknown*& m : ctx->managers) {
        propcity::Drop(m);
        m = nullptr;
    }
    std::vector<cIGZUnknown*> w;
    w.swap(ctx->wealth);
    for (cIGZUnknown* o : w) propcity::Drop(o);
    propcity::Drop(ctx->messageServer);
    ctx->messageServer = nullptr;
}

bool fc4::PropMessageOf(cIGZMessage2* msg, propcity::CallPool& pool, fc4_prop_message* out) {
    if (msg == nullptr || out == nullptr) return false;
    *out = fc4_prop_message{};
    out->msg_type = msg->GetType();
    auto* s = static_cast<cIGZMessage2Standard*>(propcity::QueryInto(pool, msg, GZIID_cIGZMessage2Standard));
    if (s == nullptr) return true;  // the Idle / Day agent messages may carry nothing the core reads
    out->data1 = static_cast<uint32_t>(s->GetData1());
    out->data2 = static_cast<uint32_t>(s->GetData2());
    out->data3 = static_cast<uint32_t>(s->GetData3());
    out->data4 = static_cast<uint32_t>(s->GetData4());
    if (out->msg_type == kMsgInsert || out->msg_type == kMsgRemove) {
        cIGZUnknown* occ = static_cast<cIGZUnknown*>(s->GetVoid1());
        if (occ == nullptr) occ = s->GetIGZUnknown();
        if (out->data3 == kTypeProp) {
            out->object = PropToken(pool, occ);
        } else if (out->data3 == kTypeNetwork) {
            out->object = TokenOf(propcity::QueryInto(pool, occ, kIID_cISC4Occupant));
        }
    } else if (out->msg_type == kMsgLotState) {
        // §4.12 [quirk]: the message's IGZUnknown is used as the lot without a QueryInterface.
        cIGZUnknown* lot = s->GetIGZUnknown();
        if (lot != nullptr) {
            propcity::Hold(lot);
            pool.Adopt(lot);
        }
        out->object = TokenOf(lot);
    }
    return true;
}

fc4_prop_host fc4::MakePropHost(PropDevContext* ctx) {
    fc4_prop_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.city_cells = &CityCells;
    h.cell_width = &CellWidth;
    h.managers = &Managers;
    h.create_wealth_requester = &CreateWealthRequester;
    h.add_agent = &AddAgent;
    h.remove_agents = &RemoveAgents;
    h.subscribe = &Subscribe;
    h.unsubscribe_all = &UnsubscribeAll;
    h.register_animation_tick = &RegisterTick;
    h.unregister_animation_tick = &UnregisterTick;
    h.post_lot_state_props = &PostLotStateProps;
    h.hold = &HoldToken;
    h.release = &ReleaseToken;
    h.requester_share = &RequesterShare;
    h.requester_satisfaction = &RequesterSatisfaction;
    h.requester_custom = &RequesterCustom;
    h.requester_select = &RequesterSelect;
    h.lot_at = &LotAt;
    h.lot_livable = &LotLivable;
    h.lot_building_type = &LotBuildingType;
    h.lot_occupancy = &LotOccupancy;
    h.lot_config_objects = &LotConfigObjects;
    h.lot_wealth = &LotWealth;
    h.lot_location = &LotLocation;
    h.lot_size = &LotSize;
    h.lot_state = &LotState;
    h.zone_type = &ZoneType;
    h.network_lot_at = &NetworkLotAt;
    h.network_lot_objects = &NetworkLotObjects;
    h.network_wealth = &NetworkWealth;
    h.lot_props = &LotProps;
    h.street_props = &StreetProps;
    h.street_props_in_rect = &StreetPropsInRect;
    h.remove_prop = &RemoveProp;
    h.prop_type_valid = &PropTypeValid;
    h.props_by_family = &PropsByFamily;
    h.prop_satisfaction = &PropSatisfaction;
    h.prop_wealth = &PropWealth;
    h.create_lot_prop = &CreateLotProp;
    h.create_network_prop = &CreateNetworkProp;
    h.prop_class = &PropClass;
    h.prop_properties = &PropProperties;
    h.prop_lot_tag = &PropLotTag;
    h.set_prop_requester = &SetPropRequester;
    h.prop_type = &PropType;
    h.prop_state = &PropState;
    h.set_prop_state = &SetPropState;
    h.set_prop_timer = &SetPropTimer;
    h.set_random_chance = &SetRandomChance;
    h.set_time_of_day_range = &SetTimeOfDayRange;
    h.time_of_day_range = &TimeOfDayRange;
    h.set_date_range = &SetDateRange;
    h.tick_date = &TickDate;
    h.prop_position = &PropPosition;
    h.set_prop_position = &SetPropPosition;
    h.set_prop_flag = &SetPropFlag;
    h.network_first_cell = &NetworkFirstCell;
    h.position_to_cell = &PositionToCell;
    h.is_powered = &IsPowered;
    h.view_available = &ViewAvailable;
    h.view_trigger_state = &ViewTriggerState;
    h.sim_date = &SimDate;
    h.clock_time = &ClockTime;
    h.visual_time_mode = &VisualTimeMode;
    h.lighting_is_night = &LightingIsNight;
    h.terrain_heights = &TerrainHeights;
    return h;
}
