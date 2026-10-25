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
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Seed.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"

#include "FC4Checked.h"
#include "FC4CoreProps.h"
#include "FC4DevelopmentInterfaces.h"  // cFC4ISC4BuildingDevelopmentSimulator
#include "FC4GameList.h"               // fc4::GameHashSetU32

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZString.h"
#include "cRZBaseString.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZPersistResourceManager.h"
#include "cGZPersistResourceKey.h"
#include "cIGZPersistResourceKeyFilter.h"
#include "cIGZPersistResourceKeyList.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4FloraSimulator.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4PropManager.h"
#include "cISCResExemplar.h"
#include "SC4Rect.h"
#include "cRZCOMDllDirector.h"

#include <vector>

namespace {

// §3.2: the 6 messages the lot manager subscribes to (fc4_netlot_init).
constexpr uint32_t kMsgZoningChanged = 0x07699455;
constexpr uint32_t kMsgLotStateChanged = 0xE8BDA2E7;
constexpr uint32_t kMsgRoadDamageChanged = 0x69247DC8;

// cIGZMessageTarget2 IID: the SDK header declares no constant (network-manager.md 1).
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;

// §2.4 / §3.2 step 4: the exemplars.
constexpr uint32_t kExemplarType = 0x6534284A;
constexpr uint32_t kPropExemplarType = 0x10;
constexpr uint32_t kNetworkLotExemplarType = 0x21;
// The lot-object properties (§2.4 objects row; lot-developer.md §17.1).
constexpr uint32_t kObjectPropBase = 0x88EDC900;
constexpr uint32_t kObjectPropLimit = 0x88EDCE00;  // the scan stops here (§B3.3)

// The host's user is the manager's owner block (FC4NetMgrHost.h NetMgrOwner:
// the shared read callbacks read the user as the NetMgrContext* at its offset
// 0, the write callbacks reach the manager through `mgr`).
cFC4NetworkLotManager* Self(void* u) {
    return static_cast<cFC4NetworkLotManager*>(static_cast<fc4::NetMgrOwner*>(u)->mgr);
}

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
// occupant insert / remove, the pothole effect props (§8.8), the first
// occupant at a cell, and the street props and flora of §8.6 (DoNetwork-
// OccupantRemoved steps 2-3, CreateProp, CreateLotObjects): the prop manager
// (city slot ?; the SDK's cISC4PropManager via cISC4City::GetPropManager) and
// the flora simulator (cISC4City::GetFloraSimulator), with the lot tags of
// occupant slots 28 (SetLotTag) and 29 (SetFlag 0x80). NOT wired (NULL, so the
// core reads 0 / false / none and says so at Init): update_intersection (the
// traffic map's slot 36 needs cFC4ISC4TrafficNetworkMap extended; the core
// counts it [choice]).

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

cISC4PropManager* PropManager(void* u) {
    cISC4City* c = Self(u)->City();
    return c ? c->GetPropManager() : nullptr;  // borrowed
}

cISC4FloraSimulator* Flora(void* u) {
    cISC4City* c = Self(u)->City();
    return c ? c->GetFloraSimulator() : nullptr;  // borrowed
}

// §8.5 DoNetworkOccupantRemoved step 2: prop manager slot 33, x & 0xFF.
void RemoveStreetProps(void* u, int32_t x, int32_t z) {
    if (!AcquireCity(u)) return;
    if (cISC4PropManager* pm = PropManager(u)) pm->RemoveStreetPropsFromTile(x & 0xFF, z & 0xFF);
}

// §8.6 CreateProp step 2: prop manager slot 32 AddStreetProp(id, pos, or),
// borrowed (the occupant manager holds the added prop).
uint64_t AddStreetProp(void* u, uint32_t id, const float* pos, uint32_t orientation) {
    if (!AcquireCity(u) || pos == nullptr) return 0;
    cISC4PropManager* pm = PropManager(u);
    if (pm == nullptr) return 0;
    const cS3DVector3 p(pos[0], pos[1], pos[2]);
    cISC4Occupant* o = pm->AddStreetProp(id, p, static_cast<int32_t>(orientation));
    FC4_X87_CHECK("netlot AddStreetProp");
    return o != nullptr ? fc4::TokenOf(o) : 0;
}

// §8.6 CreateProp step 3: the prop's slot 28 = the occupant's SetLotTag
// (compiled slot 28 = the SDK's SetLotTag, lot-developer.md §6.5).
void StreetPropSlot28(void*, uint64_t prop, uint32_t value) {
    if (prop == 0) return;
    OccOf(prop)->SetLotTag(value);
}

// §8.6 CreateProp step 4: the flipped lot's prop gets slot 29 (0x80) = the
// occupant's SetFlag (compiled slot 29).
void StreetPropSlot29(void*, uint64_t prop, uint32_t value) {
    if (prop == 0) return;
    OccOf(prop)->SetFlag(value);
}

// §8.5 DoNetworkOccupantRemoved step 3: flora slot 20
// DemolishFloraInCellArea({x, z, x, z}, 0).
void DemolishFlora(void* u, int32_t x, int32_t z) {
    if (!AcquireCity(u)) return;
    cISC4FloraSimulator* f = Flora(u);
    if (f == nullptr) return;
    f->DemolishFloraInCellArea(SC4Rect<int32_t>(x, z, x, z), 0);
    FC4_X87_CHECK("netlot DemolishFloraInCellArea");
}

// §8.6 CreateLotObjects flora arm: flora slot 17 AddNewFloraOccupant
// (type, pos, true), borrowed; the position float[3] is rewritten by the
// call (flora.md §6.5).
uint64_t AddFlora(void* u, uint32_t id, const float* pos) {
    if (!AcquireCity(u) || pos == nullptr) return 0;
    cISC4FloraSimulator* f = Flora(u);
    if (f == nullptr) return 0;
    float p[3] = {pos[0], pos[1], pos[2]};
    cISC4Occupant* o = f->AddNewFloraOccupant(id, p, true);
    FC4_X87_CHECK("netlot AddNewFloraOccupant");
    return o != nullptr ? fc4::TokenOf(o) : 0;
}

// §8.6 CreateLotObjects flora arm: the flora's slot 28 = SetLotTag(i).
void FloraSlot28(void*, uint64_t flora, uint32_t index) {
    if (flora == 0) return;
    OccOf(flora)->SetLotTag(index);
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

// ---- the object records and the ID lists (§8.6) ----------------------------------------------------------
//
// object_ids(config, index): the candidate IDs of object `index` of the
// configuration, expanded the way the lot configuration manager's slot 27
// GetObjectIDArray expands them (prop-lotconfig.md §B5.6): a family ID of a
// prop becomes the prop manager's slot 9 list (family load order), of flora
// the flora simulator's slot 14 set in its bucket order; a type ID is its own
// entry. The managers are the city's.

// The bucket-ascending, chain-oldest-first walk of a hash set we created
// (FC4LotDevHost.cpp's HashSetValues, §B5.6's order).
void HashSetValues(const fc4::GameHashSetU32<>& set, std::vector<uint32_t>& out) {
    std::vector<uint32_t> chain;
    for (size_t i = 0; i < set.bucket_count(); ++i) {
        chain.clear();
        for (auto* n = set.bucket(i); n != nullptr; n = n->next) chain.push_back(n->value);
        for (auto it = chain.rbegin(); it != chain.rend(); ++it) out.push_back(*it);
    }
}

void ObjectIds(void* u, uint64_t config, uint32_t index, fc4_u32_callback cb, void* ctx) {
    cFC4NetworkLotManager* s = Self(u);
    cFC4NetworkLotConfiguration* c = s->ConfigByToken(config);
    if (c == nullptr || cb == nullptr) return;
    const std::vector<uint32_t>& raw = c->ObjectIds(index);
    if (raw.empty()) return;
    // The expansion of prop-lotconfig.md §B5.6 (the lot configuration
    // manager's slot 27 GetObjectIDArray, which §8.6 names): per ID of the
    // object's ID list, by the object's type. A type ID is appended; a family
    // ID is expanded through the managers, in their documented orders. The
    // building arm needs the BDS's slot 18 (which assigns its vector) and the
    // prop arm the prop manager's slot 9 into a temporary vector; both walk
    // rare in the shipped data (the network lot configurations' ID lists hold
    // one value each), so the family arms log once when they run.
    const uint8_t kind = c->ObjectType(index);
    cISC4PropManager* pm = PropManager(u);
    cISC4FloraSimulator* f = Flora(u);
    std::vector<uint32_t> ids;
    for (uint32_t id : raw) {
        switch (kind) {
            case 4: {  // flora: flora simulator slots 11 (type) and 14 (family)
                if (f != nullptr) {
                    if (f->IsFloraTypeValid(id)) {  // slot 11
                        ids.push_back(id);
                    } else {
                        // Slot 14 into a hash set (hint 100), appended in its
                        // bucket order (§B5.6).
                        fc4::GameHashSetU32<> set(100);
                        f->GetFloraTypesByFamily(reinterpret_cast<intptr_t&>(set), id);
                        HashSetValues(set, ids);
                    }
                }
                break;
            }
            case 1: {  // prop: prop manager slots 8 (type) and 9 (family)
                if (pm != nullptr) {
                    if (pm->IsPropTypeValid(id)) {
                        ids.push_back(id);
                    } else {
                        SC4Vector<uint32_t> v;
                        pm->GetPropTypesByFamily(v, id);
                        for (const uint32_t t : v) ids.push_back(t);
                    }
                }
                break;
            }
            case 0: {  // building: BDS slots 15 (type) and 19 (family)
                cISC4City* city = Self(u)->City();
                cISC4BuildingDevelopmentSimulator* bds = city ? city->GetBuildingDevelopmentSimulator() : nullptr;
                cFC4ISC4BuildingDevelopmentSimulator* b = bds ? fc4::AsGame(bds) : nullptr;
                if (b != nullptr) {
                    if (b->IsBuildingTypeValid(id)) {  // slot 15
                        ids.push_back(id);
                    } else {
                        // Slot 19 GetBuildingTypeSetByFamily into a 193-bucket
                        // hash set (B), walked bucket ascending, each chain
                        // oldest first (the lot developer's order, §B5.6).
                        fc4::GameHashSetU32<> set(193);
                        b->GetBuildingTypeSetByFamily(reinterpret_cast<fc4::GameHashSetU32<>&>(set), id);
                        HashSetValues(set, ids);
                    }
                }
                break;
            }
            default:
                break;  // other types: nothing (§B5.6)
        }
    }
    for (uint32_t v : ids) cb(ctx, v);
}

// ---- host assembly -----------------------------------------------------------------------------------

fc4_netlot_host MakeHost(cFC4NetworkLotManager* self, fc4::NetMgrOwner* owner) {
    owner->mgr = self;  // the write callbacks reach the manager through it
    fc4_netlot_host h = fc4::MakeNetLotReadOnlyHost(&owner->ctx);
    h.user = owner;
    h.subscribe = &SubscribeCb;
    h.unsubscribe = &UnsubscribeCb;
    h.set_network_flag = &SetNetworkFlag;
    h.set_under_texture = &SetUnderTexture;
    h.set_variation = &SetVariation;
    h.insert_occupant = &InsertOccupant;
    h.remove_occupant = &RemoveOccupant;
    h.first_network_occupant_at = &FirstNetworkOccupantAt;
    h.remove_street_props = &RemoveStreetProps;
    h.add_street_prop = &AddStreetProp;
    h.street_prop_slot28 = &StreetPropSlot28;
    h.street_prop_slot29 = &StreetPropSlot29;
    h.demolish_flora = &DemolishFlora;
    h.add_flora = &AddFlora;
    h.flora_slot28 = &FloraSlot28;
    h.object_ids = &ObjectIds;
    h.create_effect_prop = &CreateEffectProp;
    h.effect_set_name = &EffectSetName;
    h.effect_start = &EffectStart;
    h.effect_stop = &EffectStop;
    h.set_position = &SetPosition;
    // NOT wired (NULL): update_intersection (the traffic map's slot 36; the
    // core counts it [choice], question NM11).
    return h;
}

}  // namespace

// ---- cFC4NetworkLotConfiguration ------------------------------------------------------------------------

bool cFC4NetworkLotConfiguration::QueryInterface(uint32_t riid, void** out) {
    if (out == nullptr) return false;
    *out = nullptr;
    // SPEC-GAP: the configuration IID is unspecified; expose only the known unknown identity.
    if (riid != GZIID_cIGZUnknown) return false;
    *out = static_cast<cISC4NetworkLotConfiguration*>(this);
    AddRef();
    return true;
}

uint32_t cFC4NetworkLotConfiguration::AddRef() { return ++refs_; }

uint32_t cFC4NetworkLotConfiguration::Release() {
    const uint32_t remaining = --refs_;
    if (remaining != 0) return remaining;
    delete this;
    return 0;
}

void cFC4NetworkLotConfiguration::GetName(cIGZString& name) const {
    name.FromChar(name_.data(), static_cast<uint32_t>(name_.size()));
}

const std::vector<uint32_t>& cFC4NetworkLotConfiguration::ObjectIds(size_t index) const {
    static const std::vector<uint32_t> kEmpty;
    return index < ids_.size() ? ids_[index] : kEmpty;
}

// ---- cFC4NetworkLot -----------------------------------------------------------------------------------

cFC4NetworkLot::cFC4NetworkLot(cISC4NetworkOccupant* occ,
                             std::shared_ptr<cFC4NetworkLotConfiguration> config)
    : occ_(occ), config_(std::move(config)) {
    if (occ_ != nullptr) FC4_ADDREF(occ_);
}

cFC4NetworkLot::~cFC4NetworkLot() {
    if (occ_ != nullptr) FC4_RELEASE(occ_);
}

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
    const uint32_t remaining = --refs_;
    if (remaining != 0) return remaining;
    delete this;
    return 0;
}

cISC4NetworkOccupant* cFC4NetworkLot::GetNetworkOccupant() const { return occ_; }

// A stable empty array, never null: a caller that takes GetLotConfiguration-
// ObjectArray() and calls size() on it would dereference null (this lane's
// run glue24-rep-2).
SC4Vector<cSC4LotConfigurationObject>& kEmptyObjects() {
    static SC4Vector<cSC4LotConfigurationObject> empty;
    return empty;
}

uint32_t cFC4NetworkLot::GetLotConfigurationObjectCount() {
    // §2.3 slot 4: the configuration's objects.
    SC4Vector<cSC4LotConfigurationObject>* o = config_ != nullptr ? config_->Objects() : &kEmptyObjects();
    return static_cast<uint32_t>(o->size());
}

// §2.3 slot 5: the configuration's shared object array (never null: a caller
// that takes the pointer and calls size() on it would dereference null, this
// lane's run glue24-rep-2).
SC4Vector<cSC4LotConfigurationObject>* cFC4NetworkLot::GetLotConfigurationObjectArray() {
    return config_ != nullptr ? config_->Objects() : &kEmptyObjects();
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
    host_ = MakeHost(this, &owner_);
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
    for (fc4_props* p : configProps_) fc4_props_destroy(p);
    configProps_.clear();
    configs_.clear();
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
    owner_.ctx.city = c;
}

cFC4NetworkLotConfiguration* cFC4NetworkLotManager::ConfigByToken(uint64_t token) const {
    auto it = configs_.find(token);
    return it != configs_.end() ? it->second.get() : nullptr;
}

cISC4NetworkLot* cFC4NetworkLotManager::KeepLot(int32_t x, int32_t z, cISC4NetworkOccupant* occ,
                                                uint64_t configToken) {
    const uint64_t key = (static_cast<uint64_t>(static_cast<uint32_t>(x)) << 32) |
                         static_cast<uint32_t>(z);
    // Use the live core identity: a forced choice can share its saved ID with
    // another configuration while owning a different object array.
    std::shared_ptr<cFC4NetworkLotConfiguration> config;
    const auto found = configs_.find(configToken);
    if (found != configs_.end()) config = found->second;
    auto it = lots_.find(key);
    if (it != lots_.end()) {
        if (it->second->GetNetworkOccupant() == occ && it->second->Config() == config.get())
            return it->second;  // §8.1: borrowed, no reference transferred to the caller
        // A rebuilt cell must not return its previous occupant/configuration.
        // Callers that AddRef'd the old view keep both dependencies alive.
        it->second->Release();
        lots_.erase(it);
    }
    cFC4NetworkLot* lot = new cFC4NetworkLot(occ, std::move(config));
    lots_.emplace(key, lot);
    return lot;
}

// §3.2 step 4: every exemplar of exemplar type 0x21, in the key list's order
// (the resource manager's slot 17 with a type filter), read once per process.
// Each exemplar becomes (a) a core property holder handed to
// fc4_netlot_add_configuration_props before Init, and (b) the game's object
// records and ID lists for §8.6 (the lot's object array and object_ids).
void cFC4NetworkLotManager::ReadConfigurations() {
    cIGZPersistResourceManagerPtr rm;
    if (!rm) {
        fc4::Log("netlot: no resource manager; no network lot configurations");
        return;
    }
    // The exemplar key filter (the shadow's ExemplarKeyFilter).
    class ExemplarKeyFilter final : public cIGZPersistResourceKeyFilter {
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
        bool IsKeyIncluded(cGZPersistResourceKey const& key) override { return key.type == kExemplarType; }

    private:
        uint32_t refs_ = 1;
    } filter;
    FC4_OUT(cIGZPersistResourceKeyList*, list, "cIGZPersistResourceKeyList* (GetAvailableResourceList out pointer)");
    rm->GetAvailableResourceList(list.ptr(), &filter);
    list.Check();
    cIGZPersistResourceKeyList* l = list.get();
    if (l == nullptr) {
        fc4::Log("netlot: WARNING the resource manager returned no key list; no network lot configurations");
        return;
    }
    FC4_ACQUIRED(l, "GetAvailableResourceList");
    std::vector<cGZPersistResourceKey> keys;
    const uint32_t n = l->Size();
    keys.reserve(n);
    for (uint32_t i = 0; i < n; ++i) keys.push_back(l->GetKey(i));
    FC4_RELEASE_CREATED(l);

    uint32_t added = 0;
    for (const cGZPersistResourceKey& k : keys) {
        if (k.type != kExemplarType) continue;
        FC4_OUT(cISCResExemplar*, out, "cISCResExemplar* (GetResource out pointer)");
        const bool ok = rm->GetResource(k, GZIID_cISCResExemplar, reinterpret_cast<void**>(out.ptr()), 0, nullptr);
        out.Check();
        cISCResExemplar* ex = out.get();
        if (!ok || ex == nullptr) continue;
        FC4_ACQUIRED(ex, "GetResource");
        const cISCPropertyHolder* h = ex->AsISCPropertyHolder();
        std::vector<uint32_t> t;
        if (h != nullptr && fc4::props::GetU32s(h, kPropExemplarType, t) && !t.empty() && t[0] == kNetworkLotExemplarType) {
            // The configuration ID (§2.4: property 0x21, default 0). Read on
            // the host because the core does not expose the id per token.
            std::vector<uint32_t> idProp;
            const uint32_t configId = fc4::props::GetU32s(h, 0x21, idProp) && !idProp.empty() ? idProp[0] : 0;
            // The object records and ID lists (lot-developer.md §17.1: the
            // 0x34-byte record from the property's u32 values v0, v1, ...;
            // prop-lotconfig.md §B3.3: the scan stops at the first missing
            // property or 0x88EDCE00, a property with fewer than 10 values is
            // skipped, and an object is kept when its type is 0 or its LOD
            // bits (v1 & 0x30) are below the network detail d = 3).
            auto record = cFC4NetworkLotConfiguration::Create(
                static_cast<uint64_t>(configs_.size()) + 1);
            record->SetConfigId(configId);
            std::vector<uint32_t> piece;
            std::vector<uint8_t> name;
            fc4::props::GetU32s(h, 0xC9A5A1BE, piece);
            fc4::props::GetRawArrayBytes(h, 0x20, name);
            record->SetMetadata(piece.empty() ? 0 : piece[0], std::string(name.begin(), name.end()));
            std::vector<fc4_netlot_object> objects;
            for (uint32_t id = 0;; ++id) {
                std::vector<uint32_t> v;
                if (kObjectPropBase + id > kObjectPropLimit || !fc4::props::GetU32s(h, kObjectPropBase + id, v)) break;
                if (v.size() < 10) continue;
                const uint32_t lod = (v[1] >> 4) & 3;
                if (v[0] != 0 && lod >= 3) continue;
                // The core's 24-byte object (fc4.h fc4_netlot_object).
                fc4_netlot_object o{};
                o.kind = static_cast<uint8_t>(v[0] & 0xFF);
                o.flags = static_cast<uint8_t>(v[1] & 0xFF);
                o.orientation = static_cast<uint8_t>(v[2] & 0xFF);
                // v3, v4, v5 as signed 16.16 fixed point (§17.1: fild, × 2^-16).
                o.offset[0] = static_cast<float>(static_cast<int32_t>(v[3])) * (1.0f / 65536.0f);
                o.offset[1] = static_cast<float>(static_cast<int32_t>(v[4])) * (1.0f / 65536.0f);
                o.offset[2] = static_cast<float>(static_cast<int32_t>(v[5])) * (1.0f / 65536.0f);
                o.field_24 = v.size() > 11 ? v[11] : 0;
                objects.push_back(o);
                record->AddIds(v.size() > 12 ? std::vector<uint32_t>(v.begin() + 12, v.end())
                                             : std::vector<uint32_t>());
                // The game's 0x34-byte record (§17.1; the mirror flag is the
                // low byte of v2's bit 31), built in place (AddObject: the
                // data values are copied one by one).
                record->AddObject(static_cast<uint8_t>(v[0] & 0xFF), static_cast<uint8_t>(v[1] & 0xFF),
                                  static_cast<uint8_t>(v[2] & 0xFF), (v[2] & 0x80000000u) != 0, o.offset[0],
                                  o.offset[1], o.offset[2],
                                  static_cast<float>(static_cast<int32_t>(v[8])) * (1.0f / 65536.0f),
                                  static_cast<float>(static_cast<int32_t>(v[9])) * (1.0f / 65536.0f),
                                  static_cast<float>(static_cast<int32_t>(v[6])) * (1.0f / 65536.0f),
                                  static_cast<float>(static_cast<int32_t>(v[7])) * (1.0f / 65536.0f),
                                  v.size() > 10 ? v[10] : 0, o.field_24,
                                  v.size() > 12 ? std::vector<uint32_t>(v.begin() + 12, v.end())
                                                : std::vector<uint32_t>());
            }
            // The core's copy: the objects travel as fc4_netlot_object values.
            if (fc4_props* p = fc4::props::ToCoreAll(h, ex)) {
                configProps_.push_back(p);
                if (fc4_netlot_add_configuration_props(core_, record->Token(), p, objects.data(),
                                                       static_cast<uint32_t>(objects.size()))) {
                    configs_.emplace(record->Token(), std::move(record));
                    ++added;
                } else {
                    fc4::Log("netlot: an exemplar of type 0x21 did not read (no piece ID property?); skipped");
                }
            }
        }
        FC4_RELEASE(ex);
    }
    fc4::Log("netlot: %u exemplar keys enumerated, %u network lot configurations added", n, added);
}

bool cFC4NetworkLotManager::Init() {
    FC4_TRACE("NetworkLotManager::Init");
    writable_ = true;
    AcquireCity(&owner_);
    ReadConfigurations();
    const bool ok = fc4_netlot_init(core_);
    fc4_netlot_state st{};
    st.struct_size = sizeof(st);
    fc4_netlot_get_state(core_, &st);
    // The property holders are consumed by the adds (the core copied what it
    // needs); they are freed at destruction (configProps_).
    fc4::Log("netlot Init: ok=%d lots=%u potholes=%u configs=%u pending=%u", ok ? 1 : 0, st.lots, st.potholes,
             st.configs, st.pending);
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
    fc4::NetMgrReleaseHeld(&owner_.ctx);
    if (!destroying_) Release();
    return true;
}

// Slot 5 (§8.1): the cell's lot, borrowed. A lot whose occupant did not
// resolve is not handed out: GetNetworkOccupant() would answer null and the
// caller dereferences it (this lane's run glue24-rep-1).
cISC4NetworkLot* cFC4NetworkLotManager::GetNetworkLot(int32_t x, int32_t z) {
    fc4_netlot_lot l{};
    uint64_t configToken = 0;
    if (!fc4_netlot_get_lot_with_configuration(core_, x, z, &l, &configToken) || l.occupant == 0) return nullptr;
    return KeepLot(x, z, reinterpret_cast<cISC4NetworkOccupant*>(static_cast<uintptr_t>(l.occupant)),
                   configToken);
}

// Slot 6 (§11.2 / §8.6): the configuration set. `set` is the caller's set of
// cISC4NetworkLotConfiguration objects.
// SPEC-GAP: the output container layout/allocator is unspecified; do not write an assumed set ABI.
void cFC4NetworkLotManager::GetNetworkLotConfigurationSet(intptr_t) {
    fc4::Log("netlot GetNetworkLotConfigurationSet: the output container contract is not specified");
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
bool IterateOne(void* p, int32_t x, int32_t z, const fc4_netlot_lot* lot, uint64_t configToken) {
    IterateCtx* c = static_cast<IterateCtx*>(p);
    if (lot->occupant == 0) return true;  // as GetNetworkLot: never a null occupant
    cISC4NetworkLot* l = c->self->KeepLot(
        x, z, reinterpret_cast<cISC4NetworkOccupant*>(static_cast<uintptr_t>(lot->occupant)), configToken);
    // KeepLot is borrowed. Pin it across the callback, which may replace the
    // cached cell, and release only this temporary reference afterwards.
    FC4_ADDREF(l);
    const bool go = c->cb(static_cast<uint32_t>(x), static_cast<uint32_t>(z), l, c->ctx);
    FC4_RELEASE(l);
    return go;
}
}  // namespace

bool cFC4NetworkLotManager::IterateAllNetworkLots(LotVisitor cb, void* ctx) {
    if (cb == nullptr) return true;
    IterateCtx c{this, cb, ctx};
    return fc4_netlot_iterate_lots_with_configuration(core_, &IterateOne, &c);
}

// Slot 12 (§8.6): the cell's lot is rebuilt from its configuration.
void cFC4NetworkLotManager::RedevelopLot(uint32_t x, uint32_t z, cISC4NetworkLot*) {
    fc4_netlot_redevelop_lot(core_, static_cast<int32_t>(x), static_cast<int32_t>(z));
}

// Slot 13 (§8.5): only manager-owned configurations have a core identity.
void cFC4NetworkLotManager::SetLotConfigurationForNextPiece(void* cfg) {
    if (cfg == nullptr) {
        fc4_netlot_clear_next_configuration(core_);
        return;
    }
    for (const auto& kv : configs_) {
        if (static_cast<cISC4NetworkLotConfiguration*>(kv.second.get()) == cfg) {
            fc4_netlot_set_next_configuration_token(core_, kv.second->Token());
            return;
        }
    }
    // SPEC-GAP: importing a foreign configuration is unspecified; leave the current choice intact.
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
    // §4.2: the record read back by a scratch core and written again — the
    // save's bytes must survive a load (this lane's round-trip check).
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

intptr_t NetworkLotOwnershipTest(cIGZUnknown* first, cIGZUnknown* second) {
    if (first == nullptr || second == nullptr) return -1;
    // Only the unknown prefix of these fake occupants is used (AddRef/Release).
    auto* a = reinterpret_cast<cISC4NetworkOccupant*>(first);
    auto* b = reinterpret_cast<cISC4NetworkOccupant*>(second);
    cISC4NetworkLot* retained = nullptr;
    {
        cFC4NetworkLotManager manager;
        retained = manager.KeepLot(1, 2, a, 0);
        if (manager.KeepLot(1, 2, a, 0) != retained) return 0;
        fc4_netlot_lot record{};
        record.occupant = static_cast<uint64_t>(reinterpret_cast<uintptr_t>(a));
        bool pinned = false;
        IterateCtx visit{&manager,
            [](uint32_t x, uint32_t z, cISC4NetworkLot* lot, void* ctx) {
                const uint32_t refs = lot->AddRef();
                lot->Release();
                *static_cast<bool*>(ctx) = x == 1 && z == 2 && refs == 3;
                return false;  // Early termination must also balance the pin.
            }, &pinned};
        if (IterateOne(&visit, 1, 2, &record, 0) || !pinned) return 0;
        // The cache still owns exactly one reference after iteration.
        if (retained->AddRef() != 2) {
            retained->Release();
            return 0;
        }
        retained->Release();
        retained->AddRef();
        auto* replacement = manager.KeepLot(1, 2, b, 0);
        if (replacement == retained || replacement->GetNetworkOccupant() != b) {
            retained->Release();
            return 0;
        }
    }
    const bool alive = retained->GetNetworkOccupant() == a;
    // Exactly one retained caller reference remains after manager destruction.
    if (retained->Release() != 0 || !alive) return 0;

    {
        cFC4NetworkLotManager manager;
        struct ReplaceDuringVisit {
            cFC4NetworkLotManager* manager;
            cISC4NetworkOccupant* before;
            cISC4NetworkOccupant* after;
            bool alive = false;
        } ctx{&manager, a, b};
        fc4_netlot_lot record{};
        record.occupant = static_cast<uint64_t>(reinterpret_cast<uintptr_t>(a));
        IterateCtx visit{&manager,
            [](uint32_t x, uint32_t z, cISC4NetworkLot* lot, void* opaque) {
                auto& ctx = *static_cast<ReplaceDuringVisit*>(opaque);
                auto* replacement = ctx.manager->KeepLot(x, z, ctx.after, 0);
                // Replacing the cell drops its cache reference while the callback
                // still uses the old borrowed view. Only the iterator pin remains.
                const uint32_t refs = lot->AddRef();
                ctx.alive = replacement != lot && replacement->GetNetworkOccupant() == ctx.after &&
                    lot->GetNetworkOccupant() == ctx.before && refs == 2;
                lot->Release();
                return false;
            }, &ctx};
        if (IterateOne(&visit, 1, 2, &record, 0) || !ctx.alive) return 0;
        auto* replacement = manager.KeepLot(1, 2, b, 0);
        const uint32_t refs = replacement->AddRef();
        replacement->Release();
        if (refs != 2) return 0;  // Cache owns one reference after early termination.
    }

    auto config = cFC4NetworkLotConfiguration::Create(1);
    config->SetConfigId(42);
    config->SetMetadata(0x1234, std::string("road\0lot", 8));
    cISC4NetworkLotConfiguration* iface = config.get();
    cRZBaseString name;
    iface->GetName(name);
    if (iface->GetID() != 42 || iface->GetNetworkPieceId() != 0x1234 || name.Strlen() != 8 ||
        std::string(name.ToChar(), name.Strlen()) != std::string("road\0lot", 8) ||
        iface->GetObjectArray() != config->Objects()) return 0;
    void* unknown = nullptr;
    if (!iface->QueryInterface(GZIID_cIGZUnknown, &unknown) || unknown != iface ||
        iface->QueryInterface(GZIID_cIGZUnknown, nullptr)) return 0;
    void* unsupported = iface;
    if (iface->QueryInterface(0xFFFFFFFF, &unsupported) || unsupported != nullptr) return 0;
    std::weak_ptr<cFC4NetworkLotConfiguration> weak = config;
    auto* lot = new cFC4NetworkLot(a, config);
    config.reset();
    const bool kept = !weak.expired() && lot->Config()->ConfigId() == 42 &&
        lot->GetLotConfigurationObjectArray() != nullptr;
    lot->Release();
    // The shared owner is gone, but the caller's QI reference keeps the COM object alive.
    const auto* objects = iface->GetObjectArray();
    // No game allocator exists in this DLL-only seam; nonempty exemplar arrays run in the city test.
    const bool comKept = weak.expired() && iface->GetID() == 42 && objects != nullptr && objects->empty();
    if (static_cast<cIGZUnknown*>(unknown)->Release() != 0 || !kept || !comKept) return 0;

    cFC4NetworkLotManager manager;
    auto forced = cFC4NetworkLotConfiguration::Create(7);
    forced->SetConfigId(99);
    manager.configs_.emplace(7, forced);
    auto duplicate = cFC4NetworkLotConfiguration::Create(8);
    duplicate->SetConfigId(99);
    manager.configs_.emplace(8, duplicate);
    auto* oldView = static_cast<cFC4NetworkLot*>(manager.KeepLot(4, 5, a, 8));
    if (oldView->Config() != duplicate.get() || manager.KeepLot(4, 5, a, 8) != oldView) return 0;
    oldView->AddRef();
    // Same occupant and saved ID, different selected object: rebuild the view.
    fc4_netlot_lot entry{};
    entry.occupant = static_cast<uint64_t>(reinterpret_cast<uintptr_t>(a));
    entry.config_id = 99;
    entry.has_config = 1;
    bool exactConfig = false;
    IterateCtx choice{&manager,
        [](uint32_t, uint32_t, cISC4NetworkLot* lot, void* ctx) {
            *static_cast<bool*>(ctx) = static_cast<cFC4NetworkLot*>(lot)->Config()->Token() == 7;
            return true;
        }, &exactConfig};
    if (!IterateOne(&choice, 4, 5, &entry, 7) || !exactConfig ||
        manager.KeepLot(4, 5, a, 7) == oldView || oldView->Config() != duplicate.get()) return 0;
    std::weak_ptr<cFC4NetworkLotConfiguration> duplicateLife = duplicate;
    manager.configs_.erase(8);
    duplicate.reset();
    if (duplicateLife.expired() || oldView->Config()->Token() != 8) return 0;
    oldView->Release();
    if (!duplicateLife.expired()) return 0;
    fc4_netlot_config c{};
    c.struct_size = sizeof(c);
    c.token = 7;
    c.id = 99;
    c.piece_id = 0x4321;
    if (!fc4_netlot_add_configuration(manager.core_, &c) || !fc4_netlot_init(manager.core_)) return 0;
    manager.SetLotConfigurationForNextPiece(static_cast<cISC4NetworkLotConfiguration*>(forced.get()));
    fc4_netlot_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_netlot_get_state(manager.core_, &st) || !st.has_forced || st.forced_id != 99) return 0;
    // An unowned pointer is never cast/dereferenced; null explicitly clears the choice.
    manager.SetLotConfigurationForNextPiece(first);
    if (!fc4_netlot_get_state(manager.core_, &st) || !st.has_forced || st.forced_id != 99) return 0;
    manager.SetLotConfigurationForNextPiece(nullptr);
    return fc4_netlot_get_state(manager.core_, &st) && !st.has_forced ? 1 : 0;
}
