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

#include "cFC4NetworkLotManager.h"

#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4HistClock.h"
#include "FC4NetMgrHost.h"
#include "FC4RefAudit.h"
#include "FC4Seed.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "SC4Vector.h"
#include "cSC4LotConfigurationObject.h"
#include "cRZCOMDllDirector.h"

#include <vector>

namespace {

// §3.2: the 6 messages the lot manager subscribes to (fc4_netlot_init).
constexpr uint32_t kMsgZoningChanged = 0x07699455;
constexpr uint32_t kMsgLotStateChanged = 0xE8BDA2E7;
constexpr uint32_t kMsgRoadDamageChanged = 0x69247DC8;

// cIGZMessageTarget2 IID: the SDK header declares no constant (network-manager.md 1).
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;

cFC4NetworkLotManager* Self(void* u) { return static_cast<cFC4NetworkLotManager*>(u); }

cISC4App* App() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app != nullptr && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) && sc4 != nullptr) {
        FC4_ACQUIRED(sc4, "QueryInterface");
        return sc4;  // the caller releases it
    }
    return nullptr;
}

cISC4NetworkOccupant* NetOf(uint64_t t) {
    return reinterpret_cast<cISC4NetworkOccupant*>(fc4::FromToken(t));
}
cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(fc4::FromToken(t)); }

bool AcquireCity(void* u) {
    cFC4NetworkLotManager* s = Self(u);
    if (s->City() != nullptr) return true;
    if (cISC4App* app = App()) {
        s->SetCity(app->GetCity());
        FC4_RELEASE(app);
    }
    return s->City() != nullptr;
}

void SubscribeCb(void* u, uint32_t t) {
    cFC4NetworkLotManager* s = Self(u);
    if (cIGZMessageServer2* ms = s->Server()) ms->AddNotification(s->Target(), t);
}
void UnsubscribeCb(void* u, uint32_t t) {
    cFC4NetworkLotManager* s = Self(u);
    if (cIGZMessageServer2* ms = s->Server()) ms->RemoveNotification(s->Target(), t);
}

// ---- the write side (the read side is FC4NetMgrHost.cpp) -----------------------------------------------
//
// Wired here (§13's simulation-visible list): the subscriptions, the city, the
// road and avenue style writes (§8.7: flags, under texture, variation), the
// occupant insert / remove, the pothole effect props (§8.8) and the first
// occupant at a cell. NOT wired (NULL, so the core reads 0 / false / none and
// says so at Init): the street props and flora of §8.6 (the prop manager and
// the flora simulator are other subsystems'), `object_ids` (the lot
// configuration manager's slot 27), `update_intersection` and the traffic
// map's `find_network_tile` is the read host's.

void SetNetworkFlag(void*, uint64_t n, uint32_t bits) {
    if (n != 0) NetOf(n)->SetNetworkFlag(bits);  // slot 10
}

void SetUnderTexture(void*, uint64_t n, uint32_t texture) {
    if (n != 0) NetOf(n)->SetUnderTexture(texture);  // slot 29
}

void SetVariation(void*, uint64_t n, uint32_t variation) {
    if (n != 0) NetOf(n)->SetVariation(static_cast<uint8_t>(variation));  // slot 27
}

void InsertOccupant(void* u, uint64_t occupant) {
    if (occupant == 0 || !AcquireCity(u)) return;
    cISC4OccupantManager* om = Self(u)->City()->GetOccupantManager();
    if (om == nullptr) return;
    om->InsertOccupant(OccOf(occupant), 0);  // slot 20
    FC4_X87_CHECK("netlot InsertOccupant");
}

void RemoveOccupant(void* u, uint64_t occupant) {
    if (occupant == 0 || !AcquireCity(u)) return;
    cISC4OccupantManager* om = Self(u)->City()->GetOccupantManager();
    if (om == nullptr) return;
    om->RemoveOccupant(OccOf(occupant), false, 0);  // slot 21
    FC4_X87_CHECK("netlot RemoveOccupant");
}

uint64_t FirstNetworkOccupantAt(void* u, int32_t x, int32_t z, uint32_t mask) {
    if (!AcquireCity(u)) return 0;
    cISC4OccupantManager* om = Self(u)->City()->GetOccupantManager();
    if (om == nullptr) return 0;
    cISC4Occupant* o = nullptr;
    if (!om->GetFirstOccupantByStandardCityCell(o, x, z, mask) || o == nullptr) return 0;  // slot 32
    return fc4::TokenOf(o);
}

// §8.8: the pothole is an effect prop (class 0x2977AA49) the traffic simulator
// drives. The core owns its lifetime; the game object is created here.
uint64_t CreateEffectProp(void*) {
    // The effect-prop class is not one the plugin creates on its own (it needs
    // the effects manager's factory, effects.md §5); until that is wired the
    // pothole is a stand-in the core keeps and the game never sees [choice].
    return fc4::NewNetStandIn(0x2977AA49);
}

void EffectSetName(void*, uint64_t prop, const char* name) {
    if (prop != 0 && fc4::IsNetStandIn(prop) && name != nullptr) {
        fc4::Log("netlot pothole name '%s' (stand-in prop)", name);
    }
}
void EffectStart(void*, uint64_t) {}
void EffectStop(void*, uint64_t) {}
void SetPosition(void*, uint64_t, const float*) {}

// ---- host assembly -----------------------------------------------------------------------------------

fc4_netlot_host MakeHost(cFC4NetworkLotManager* self, fc4::NetMgrContext* ctx) {
    fc4_netlot_host h = fc4::MakeNetLotReadOnlyHost(ctx);
    h.user = self;
    h.subscribe = &SubscribeCb;
    h.unsubscribe = &UnsubscribeCb;
    h.set_network_flag = &SetNetworkFlag;
    h.set_under_texture = &SetUnderTexture;
    h.set_variation = &SetVariation;
    h.insert_occupant = &InsertOccupant;
    h.remove_occupant = &RemoveOccupant;
    h.first_network_occupant_at = &FirstNetworkOccupantAt;
    h.create_effect_prop = &CreateEffectProp;
    h.effect_set_name = &EffectSetName;
    h.effect_start = &EffectStart;
    h.effect_stop = &EffectStop;
    h.set_position = &SetPosition;
    // NOT wired (NULL): remove_street_props, add_street_prop, street_prop_slot28
    // / _slot29, demolish_flora, add_flora, flora_slot28, object_ids,
    // update_intersection.
    return h;
}

}  // namespace

// ---- cFC4NetworkLot -----------------------------------------------------------------------------------

bool cFC4NetworkLot::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (riid != 1u && riid != kFC4_IID_cISC4NetworkLot && riid != GZIID_cIGZUnknown) {
        *ppvObj = nullptr;
        return false;
    }
    *ppvObj = static_cast<cISC4NetworkLot*>(this);
    AddRef();
    return true;
}

uint32_t cFC4NetworkLot::AddRef() { return ++refs_; }

uint32_t cFC4NetworkLot::Release() {
    if (refs_ > 1) return --refs_;
    delete this;
    return 0;
}

cISC4NetworkOccupant* cFC4NetworkLot::GetNetworkOccupant() const { return occ_; }

uint32_t cFC4NetworkLot::GetLotConfigurationObjectCount() {
    // §2.3: the configuration's objects. The core's copy is its own struct
    // (fc4_netlot_object); the bridge to the lot configuration manager's
    // cSC4LotConfigurationObject (lot-configuration.md 3.5) is not built, so
    // the count is 0 and the array below is empty [choice].
    return 0;
}

// A stable empty array, never null: a caller that takes `GetLotConfiguration-
// ObjectArray()` and calls size() on it would dereference null (this lane's
// run glue24-rep-2).
SC4Vector<cSC4LotConfigurationObject>* cFC4NetworkLot::GetLotConfigurationObjectArray() {
    static SC4Vector<cSC4LotConfigurationObject> kEmpty;
    return &kEmpty;
}

// ---- cFC4NetworkLotManager ----------------------------------------------------------------------------

cFC4NetworkLotManager::cFC4NetworkLotManager() {
    {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            SetServer(ms);
            FC4_ADDREF(msgServer_);
        }
    }
    host_ = MakeHost(this, &ctx_);
    core_ = fc4_netlot_create(&host_, 0, fc4::SessionSeed());
    if (core_ == nullptr) fc4::Log("cFC4NetworkLotManager: core creation failed");
    fc4::Log("cFC4NetworkLotManager(%p) created, core %p", static_cast<void*>(Primary()),
             static_cast<void*>(core_));
}

cFC4NetworkLotManager::~cFC4NetworkLotManager() {
    fc4_netlot_destroy(core_);
    core_ = nullptr;
    for (auto& kv : lots_) kv.second->Release();
    lots_.clear();
    SetCity(nullptr);
    FC4_RELEASE(msgServer_);
    msgServer_ = nullptr;
    fc4::Log("cFC4NetworkLotManager destroyed");
}

bool cFC4NetworkLotManager::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (!fc4_netlot_query_interface(riid)) return false;
    if (riid == 1u || riid == kFC4_IID_cISC4NetworkLotManager) {
        *ppvObj = Primary();
    } else if (riid == kIID_cIGZMessageTarget2) {
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
    } else if (riid == GZIID_cIGZSerializable) {
        *ppvObj = static_cast<cIGZSerializable*>(this);
    } else {
        return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4NetworkLotManager::AddRef() { return ++refCount_; }

uint32_t cFC4NetworkLotManager::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    if (destroying_) return 0;
    destroying_ = true;
    if (core_ != nullptr) {
        fc4_netlot_state st{};
        st.struct_size = sizeof(st);
        if (fc4_netlot_get_state(core_, &st) && st.initialized != 0) Shutdown();
    }
    delete this;
    return 0;
}

void cFC4NetworkLotManager::SetCity(cISC4City* c) {
    if (c == city_) return;
    if (c != nullptr) FC4_ADDREF(c);
    if (city_ != nullptr) FC4_RELEASE(city_);
    city_ = c;
    ctx_.city = c;
}

cISC4NetworkLot* cFC4NetworkLotManager::KeepLot(int32_t x, int32_t z, cISC4NetworkOccupant* occ,
                                                uint32_t configId) {
    const uint64_t key = (static_cast<uint64_t>(static_cast<uint32_t>(x)) << 32) |
                         static_cast<uint32_t>(z);
    auto it = lots_.find(key);
    if (it != lots_.end()) {
        // The cell's lot object is reused; its reference is the caller's.
        it->second->AddRef();
        return it->second;
    }
    cFC4NetworkLot* lot = new cFC4NetworkLot(occ, configId);
    lots_.emplace(key, lot);
    lot->AddRef();  // one for the map, one for the caller
    return lot;
}

bool cFC4NetworkLotManager::Init() {
    FC4_TRACE("NetworkLotManager::Init");
    writable_ = true;
    AcquireCity(&ctx_);
    const bool ok = fc4_netlot_init(core_);
    fc4_netlot_state st{};
    st.struct_size = sizeof(st);
    fc4_netlot_get_state(core_, &st);
    fc4::Log("netlot Init: ok=%d lots=%u potholes=%u configs=%u pending=%u", ok ? 1 : 0, st.lots,
             st.potholes, st.configs, st.pending);
    return true;
}

bool cFC4NetworkLotManager::Shutdown() {
    FC4_TRACE("NetworkLotManager::Shutdown");
    if (!destroying_) AddRef();
    fc4_netlot_shutdown(core_);
    fc4_netlot_state st{};
    st.struct_size = sizeof(st);
    fc4_netlot_get_state(core_, &st);
    fc4::Log("netlot Shutdown: lots=%u potholes=%u", st.lots, st.potholes);
    writable_ = false;
    for (auto& kv : lots_) kv.second->Release();
    lots_.clear();
    fc4::NetMgrReleaseHeld(&ctx_);
    if (!destroying_) Release();
    return true;
}

// Slot 5 (§8.1): the cell's lot, borrowed. A lot whose occupant did not
// resolve is not handed out: GetNetworkOccupant() would answer null and the
// caller dereferences it (this lane's run glue24-rep-1).
cISC4NetworkLot* cFC4NetworkLotManager::GetNetworkLot(int32_t x, int32_t z) {
    fc4_netlot_lot l{};
    if (!fc4_netlot_get_lot(core_, x, z, &l) || l.occupant == 0) return nullptr;
    return KeepLot(x, z, reinterpret_cast<cISC4NetworkOccupant*>(static_cast<uintptr_t>(l.occupant)),
                   l.config_id);
}

// Slot 6 (§11.2): the configuration set. `set` is the caller's
// cIGZUnknown-style set of cISC4NetworkLotConfiguration; the bridge needs the
// lot configuration manager's object type, so nothing is inserted yet [choice].
void cFC4NetworkLotManager::GetNetworkLotConfigurationSet(intptr_t) {
    fc4::Log("netlot GetNetworkLotConfigurationSet: the configuration-object bridge is not wired");
}

// Slot 7 (§8.6): true when a loaded configuration has `id`. The original never
// writes `out` (quirk), so neither does this.
bool cFC4NetworkLotManager::GetNetworkLotConfigurationById(uint32_t id, void*) {
    return fc4_netlot_configuration_by_id(core_, id);
}

// Slot 8 (§8.6): the street prop for the cell's lot, borrowed.
cISC4Occupant* cFC4NetworkLotManager::CreateProp(cISC4NetworkLot* lot, uint32_t index, uint32_t propId) {
    cFC4NetworkLot* ours = static_cast<cFC4NetworkLot*>(lot);
    cISC4NetworkOccupant* occ = ours != nullptr ? ours->GetNetworkOccupant() : nullptr;
    if (occ == nullptr) return nullptr;
    uint32_t ux = 0, uz = 0;
    occ->GetOccupiedCell(ux, uz);  // slot 59
    return reinterpret_cast<cISC4Occupant*>(static_cast<uintptr_t>(
        fc4_netlot_create_prop(core_, static_cast<int32_t>(ux), static_cast<int32_t>(uz), index, propId)));
}

int32_t cFC4NetworkLotManager::GetZoneTypeForNetwork(int32_t x, int32_t z) {
    return fc4_netlot_zone_type_for_network(core_, x, z);
}

int32_t cFC4NetworkLotManager::GetWealthForNetwork(int32_t x, int32_t z) {
    return fc4_netlot_wealth_for_network(core_, x, z);
}

// Slot 11 (§11.2): stops at the first false.
namespace {
struct IterateCtx {
    cFC4NetworkLotManager* self;
    cFC4ISC4NetworkLotManager::LotVisitor cb;
    void* ctx;
};
bool IterateOne(void* p, int32_t x, int32_t z, const fc4_netlot_lot* lot) {
    IterateCtx* c = static_cast<IterateCtx*>(p);
    if (lot->occupant == 0) return true;  // as GetNetworkLot: never a null occupant
    cISC4NetworkLot* l = c->self->KeepLot(
        x, z, reinterpret_cast<cISC4NetworkOccupant*>(static_cast<uintptr_t>(lot->occupant)), lot->config_id);
    const bool go = c->cb(static_cast<uint32_t>(x), static_cast<uint32_t>(z), l, c->ctx);
    l->Release();
    return go;
}
}  // namespace

bool cFC4NetworkLotManager::IterateAllNetworkLots(LotVisitor cb, void* ctx) {
    if (cb == nullptr) return true;
    IterateCtx c{this, cb, ctx};
    return fc4_netlot_iterate_lots(core_, &IterateOne, &c);
}

// Slot 12 (§8.6): the cell's lot is rebuilt from its configuration.
void cFC4NetworkLotManager::RedevelopLot(uint32_t x, uint32_t z, cISC4NetworkLot*) {
    fc4_netlot_redevelop_lot(core_, static_cast<int32_t>(x), static_cast<int32_t>(z));
}

// Slot 13 (§8.5): the configuration for the next piece. Without the
// configuration-object bridge the forced configuration is cleared [choice].
void cFC4NetworkLotManager::SetLotConfigurationForNextPiece(void* cfg) {
    if (cfg == nullptr) {
        fc4_netlot_clear_next_configuration(core_);
        return;
    }
    fc4::Log("netlot SetLotConfigurationForNextPiece: the configuration-object bridge is not wired");
}

// Slot 14 (§11.2): tooling, configurations of version < 2.
bool cFC4NetworkLotManager::WriteLotConfiguration(void*, cIGZPersistDBSegment*) { return false; }

// §3.2 / §6.2: the 6 subscribed messages. `*data2` of the zoning message is a
// cell rect (§6.2).
bool cFC4NetworkLotManager::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("netlot DoMessage", pMessage->GetType());
    const uint32_t type = pMessage->GetType();
    uint32_t d1 = 0, d2 = 0, d3 = 0;
    uint64_t void1 = 0, unknown = 0;
    fc4_cell_rect rect{};
    const fc4_cell_rect* rectp = nullptr;
    cIGZMessage2Standard* m = nullptr;
    if (pMessage->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) && m != nullptr) {
        FC4_ACQUIRED(m, "QueryInterface");
        d1 = static_cast<uint32_t>(m->GetData1());
        d2 = static_cast<uint32_t>(m->GetData2());
        d3 = static_cast<uint32_t>(m->GetData3());
        if (type == kMsgLotStateChanged || type == kMsgRoadDamageChanged) void1 = fc4::TokenOf(m->GetVoid1());
        if (type == kMsgZoningChanged) unknown = fc4::TokenOf(m->GetIGZUnknown());
        if (type == kMsgZoningChanged) {
            if (const int32_t* r = static_cast<const int32_t*>(m->GetVoid2())) {
                rect = fc4_cell_rect{r[0], r[1], r[2], r[3]};
                rectp = &rect;
            }
        }
        FC4_RELEASE(m);
    }
    return fc4_netlot_do_message(core_, type, d1, d2, d3, void1, unknown, rectp);
}

bool cFC4NetworkLotManager::Write(cIGZOStream& s) {
    FC4_TRACE("NetworkLotManager::Write");
    fc4::OStreamAdapter a(s);
    fc4::MemOStream mem;
    const bool ok = fc4_netlot_write(core_, a.Get());
    fc4_netlot_write(core_, mem.Get());
    // §4.2: the record read back by a scratch core and written again.
    bool same = false;
    {
        fc4_netlot_host h{};
        h.struct_size = sizeof(h);
        fc4_netlot* scratch = fc4_netlot_create(&h, 0, 0);
        fc4::MemIStream in(mem.bytes);
        fc4::MemOStream again;
        if (scratch != nullptr && fc4_netlot_read(scratch, in.Get()) && in.Remaining() == 0 &&
            fc4_netlot_write(scratch, again.Get())) {
            same = again.bytes == mem.bytes;
        }
        fc4_netlot_destroy(scratch);
    }
    fc4::Log("netlot Write -> %d (%u bytes, record11 round trip %s)", ok ? 1 : 0,
             static_cast<unsigned>(mem.bytes.size()), same ? "same" : "DIFFERENT");
    return ok;
}

// §4.2: Read runs before Init; the configurations are attached in Init (§3.2
// step 5). Nothing here is sized from the city.
bool cFC4NetworkLotManager::Read(cIGZIStream& s) {
    FC4_TRACE("NetworkLotManager::Read");
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_netlot_read(core_, a.Get());
    fc4_netlot_state st{};
    st.struct_size = sizeof(st);
    fc4_netlot_get_state(core_, &st);
    fc4::Log("netlot Read -> %d (%u lots, %u potholes)", ok ? 1 : 0, st.lots, st.potholes);
    return ok;
}

uint32_t cFC4NetworkLotManager::GetGZCLSID() { return kFC4_CLSID_cSC4NetworkLotManager; }

cIGZUnknown* CreateFC4NetworkLotManager() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4NetworkLotManager*>(new cFC4NetworkLotManager()));
}
