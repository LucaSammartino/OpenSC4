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

// The lot developer's host callbacks (fc4.h fc4_lotdev_host) over the game's
// interfaces (docs/specs/lot-developer.md §14), in the table's order. Every
// token handed to the core is BORROWED (fc4.h): references the game's lists
// give, objects made for the core and message objects are kept in the
// context until the outermost call into the core returns (LotDevReleaseHeld).
//
// Tokens. The core passes building tokens where it expects occupants (the
// first building to occupant_remove and remove_building_props, §7.1), so
// every occupant callback reaches the occupant through QueryInterface
// (cISC4Occupant 0x26BB767E), every building callback through
// cISC4BuildingOccupant (0x87DDA3A9); the tokens this file hands out are the
// cISC4Occupant (occupants) and cISC4BuildingOccupant (buildings) pointers.
//
// Main thread, holding the lot developer's lock; the shadow
// (FC4LotDevShadow.cpp) runs on the main thread.

#include "cFC4LotDeveloper.h"
#include "cFC4ZoneDeveloper.h"

#include "FC4Config.h"
#include "FC4ConstructionSetup.h"
#include "FC4CoreProps.h"
#include "FC4DevelopmentInterfaces.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Pacing.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4SafeLotIterator.h"
#include "FC4Streams.h"
#include "FC4TractInterfaces.h"
#include "FC4WinInterfaces.h"
#include "cFC4Message2Standard.h"
#include "compat/FC4Compat.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "SC4Point.h"
#include "SC4Percentage.h"
#include "SC4Rect.h"
#include "cGZPersistResourceKey.h"
#include "cIGZApp.h"
#include "cIGZCOM.h"
#include "cIGZFrameWork.h"
#include "cIGZMessageServer2.h"
#include "cIGZPersistResourceManager.h"
#include "cISC424HourClock.h"
#include "cISC4App.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4BusinessUnit.h"
#include "cISC4City.h"
#include "cISC4Demand.h"
#include "cISC4DemandSimulator.h"
#include "cISC4Demolition.h"
#include "cISC4FloraSimulator.h"
#include "cISC4FoundationOccupant.h"
#include "cISC4GrowthDeveloper.h"
#include "cISC4Lot.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4LotBaseTextureOccupant.h"
#include "cISC4LotConfiguration.h"
#include "cISC4LotConfigurationManager.h"
#include "cISC4LotManager.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4PlumbingSimulator.h"
#include "cISC4PowerSimulator.h"
#include "cISC4PropManager.h"
#include "cISC4PropOccupant.h"
#include "cISC4PropOccupantTerrainDecal.h"
#include "cISC4SimGrid.h"
#include "cISC4TrafficSimulator.h"
#include "cISC4View3DWin.h"
#include "cISC4ZoneManager.h"
#include "cISCPropertyHolder.h"
#include "cISTETerrain.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"
#include "cSC4LotConfigurationObject.h"

#include <algorithm>
#include <cstring>
#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

// §3.1: the agent's name and flags.
constexpr const char* kAgentName = "cSC4LotDeveloper";
constexpr uint32_t kAgentFlags = 2;
// §1.3 step 10: the animation-tick manager and the priority (every frame).
constexpr uint32_t kTickPriority = 0xC0;
// IIDs.
constexpr uint32_t kIID_cIGZUnknown = 0x00000001;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4BuildingOccupant = 0x87DDA3A9;
constexpr uint32_t kIID_cISC4PropOccupant = 0xE9793A65;
constexpr uint32_t kIID_cISC4FoundationOccupant = 0xC8FCE101;
constexpr uint32_t kIID_cISC4LotBaseTextureOccupant = 0x097F6D98;
constexpr uint32_t kCLSID_cSC4LotBaseTextureOccupant = 0xC97F987C;
constexpr uint32_t kCLSID_cSC4ConstructionOccupant = 0xA97F909E;
constexpr uint32_t kIID_cSC4ConstructionOccupant = 0x496E7636;
constexpr uint32_t kCLSID_cSC4BusinessUnit = 0x4A232DA8;
constexpr uint32_t kIID_cISC4BusinessUnit = 0x6A233069;
// Occupant types.
constexpr uint32_t kOccupantTypeBuilding = 0x278128A0;
constexpr uint32_t kOccupantTypeFlora = 0x74758926;
constexpr uint32_t kOccupantTypeBaseTexture = 0x497F6D9D;
// A lot's base texture has usage 2 (the landfill's 1, landfill.md §4.8).
constexpr int32_t kLotTextureUsage = 2;
constexpr uint32_t kMsgNightLight = 0x09D8E0B6;
constexpr uint32_t kMsgNightUpdated = 0xA6B79621;
// §3.5 the "Building Development" exemplar.
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0xE8DA7677;
// §5.6: the lot's zot property (the lotdev_lot_log line reads it).
constexpr uint32_t kPropZot = 0xC999C45E;
// §7.2 / tract §4.15.1: DemolishLots(true, [lot], 0, 0x800, false, 0, 0, 0).
constexpr uint32_t kDemolishFlags = 0x800;
// §6.4: hash sets built with bucket hint 100 (193 buckets).
constexpr uint32_t kSetHint = 100;
// Registered safe iterators are few; a longer ring is not trusted.
constexpr uint32_t kMaxRegisteredIterators = 4096;
// The cIGZVariant type of a uint32 property.
constexpr uint16_t kVariantUint32 = 0x06;

fc4::LotDevContext* Ctx(void* u) { return static_cast<fc4::LotDevContext*>(u); }
cISC4City* City(void* u) { return Ctx(u)->city; }

// The lotdev_* switches; the shadow's context reads the game as it is.
bool Gate(void* u, fc4::Sw s, const char* what) {
    if (!Ctx(u)->gated || fc4::On(s)) return true;
    fc4::NoteSkipped(s, what);
    return false;
}

// Writes reach the game only from the replacement (the shadow's table has
// them NULL anyway).
bool Writes(void* u) { return !Ctx(u)->readOnly; }

// Test seam op 52: a callback of the test table answers instead of the game.
#define LOTDEV_TEST(name, ...)                                                                 \
    do {                                                                                       \
        const fc4_lotdev_host* t_ = Ctx(u)->test;                                              \
        if (t_ != nullptr && t_->name != nullptr) return t_->name(t_->user, __VA_ARGS__);      \
    } while (0)
#define LOTDEV_TEST0(name)                                                                     \
    do {                                                                                       \
        const fc4_lotdev_host* t_ = Ctx(u)->test;                                              \
        if (t_ != nullptr && t_->name != nullptr) return t_->name(t_->user);                   \
    } while (0)

void NoteOnce(bool& done, const char* text) {
    if (done) return;
    done = true;
    fc4::Log("%s", text);
}
bool g_noteNoAllocator = false;
bool g_noteRetainingWalls = false;
bool g_noteBurnt = false;
bool g_noteRubble = false;
bool g_noteConstruction = false;
bool g_noteTexture = false;
bool g_noteUnit = false;
bool g_noteFamilySet = false;
bool g_noteIteratorFallback = false;
bool g_noteIteratorLeak = false;
bool g_noteTick = false;
bool g_noteZotView = false;

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

// A service still null takes `value` with a reference of its own.
template <class T>
void TakeIfNull(T*& slot, T* value) {
    if (slot != nullptr || value == nullptr) return;
    slot = value;
    FC4_ADDREF(slot);
}

template <class T>
void ReleaseSlot(T*& slot) {
    if (slot != nullptr) FC4_RELEASE(slot);
    slot = nullptr;
}

// The interface `iid` of `t` (borrowed: the QueryInterface's reference is
// dropped at once; the token's holder keeps the object alive).
template <class T>
T* As(uint64_t t, uint32_t iid) {
    if (t == 0) return nullptr;
    void* p = nullptr;
    if (!FromToken(t)->QueryInterface(iid, &p) || p == nullptr) return nullptr;
    T* r = static_cast<T*>(p);
    FC4_ACQUIRED(reinterpret_cast<cIGZUnknown*>(r), "QueryInterface");
    FC4_RELEASE(reinterpret_cast<cIGZUnknown*>(r));
    return r;
}

cISC4Occupant* Occ(uint64_t t) { return As<cISC4Occupant>(t, kIID_cISC4Occupant); }
cISC4BuildingOccupant* Building(uint64_t t) { return As<cISC4BuildingOccupant>(t, kIID_cISC4BuildingOccupant); }
cFC4ISC4Lot* LotOf(uint64_t t) { return fc4::AsGame(reinterpret_cast<cISC4Lot*>(FromToken(t))); }
cISC4Lot* SdkLotOf(uint64_t t) { return reinterpret_cast<cISC4Lot*>(FromToken(t)); }
cISC4LotConfiguration* CfgOf(uint64_t t) { return reinterpret_cast<cISC4LotConfiguration*>(FromToken(t)); }
cFC4ISC4GrowthDeveloper* DevOf(uint64_t t) {
    return fc4::AsGame(reinterpret_cast<cISC4GrowthDeveloper*>(FromToken(t)));
}
cISC4BusinessUnit* UnitOf(uint64_t t) { return reinterpret_cast<cISC4BusinessUnit*>(FromToken(t)); }

SC4Rect<int32_t> RectOf(const fc4_cell_rect& r) { return SC4Rect<int32_t>(r.x1, r.z1, r.x2, r.z2); }

// The services (the context's, else the city's for the shadow).
cISC4LotManager* Lots(void* u) {
    fc4::LotDevContext* c = Ctx(u);
    if (c->lots == nullptr && c->readOnly && c->city != nullptr) return c->city->GetLotManager();
    return c->lots;
}
cISC4OccupantManager* Occupants(void* u) {
    fc4::LotDevContext* c = Ctx(u);
    if (c->occupants == nullptr && c->readOnly && c->city != nullptr) return c->city->GetOccupantManager();
    return c->occupants;
}
cISC4PropManager* Props(void* u) {
    fc4::LotDevContext* c = Ctx(u);
    if (c->props == nullptr && c->readOnly && c->city != nullptr) return c->city->GetPropManager();
    return c->props;
}
cFC4ISC4BuildingDevelopmentSimulator* Bds(void* u) {
    fc4::LotDevContext* c = Ctx(u);
    cISC4BuildingDevelopmentSimulator* b = c->bds;
    if (b == nullptr && c->readOnly && c->city != nullptr) b = c->city->GetBuildingDevelopmentSimulator();
    return b ? fc4::AsGame(b) : nullptr;
}
cISTETerrain* Terrain(void* u) {
    fc4::LotDevContext* c = Ctx(u);
    if (c->terrain == nullptr && c->readOnly && c->city != nullptr) return c->city->GetTerrain();
    return c->terrain;
}
cISC4FloraSimulator* Flora(void* u) {
    fc4::LotDevContext* c = Ctx(u);
    if (c->flora == nullptr && c->readOnly && c->city != nullptr) return c->city->GetFloraSimulator();
    return c->flora;
}
cFC4ISC4LotConfigurationManager* Configs(void* u) {
    cISC4City* c = City(u);
    cISC4LotConfigurationManager* m = c ? c->GetLotConfigurationManager() : nullptr;  // borrowed
    return m ? fc4::AsGame(m) : nullptr;
}

bool ListsAvailable(void* u) {
    if (fc4::TractNodeAllocator::Available()) return true;
    ++Ctx(u)->listsRefused;
    NoteOnce(g_noteNoAllocator, "lotdev: the game's STL allocator is not available; game sets and lists are not made "
                                "(no lots, building or flora types listed)");
    return false;
}

using LotList = fc4::GameList<cISC4Lot*, fc4::TractNodeAllocator>;

// The entries of an SDK occupant list, one reference each kept for the call
// (the list's destructor then only frees its nodes).
std::vector<uint64_t> TakeOccupants(void* u, SC4List<cISC4Occupant*>& list, const char* how) {
    std::vector<uint64_t> tokens;
    for (cISC4Occupant*& o : list) {
        if (o == nullptr) continue;
        FC4_ACQUIRED(o, how);
        fc4::LotDevHold(Ctx(u), o);
        tokens.push_back(TokenOf(o));
        o = nullptr;
    }
    return tokens;
}

// The lots of a game list, as TakeOccupants.
std::vector<uint64_t> TakeLots(void* u, LotList& list, const char* how) {
    std::vector<uint64_t> tokens;
    for (cISC4Lot*& lot : list) {
        if (lot == nullptr) continue;
        FC4_ACQUIRED(lot, how);
        fc4::LotDevHold(Ctx(u), reinterpret_cast<cIGZUnknown*>(lot));
        tokens.push_back(TokenOf(lot));
        lot = nullptr;
    }
    return tokens;
}

void Each(const std::vector<uint64_t>& tokens, fc4_token_callback cb, void* cbCtx) {
    if (cb == nullptr) return;
    for (uint64_t t : tokens) cb(cbCtx, t);
}

// A uint32 property of a holder (first element), false when absent.
bool U32Property(const cISCPropertyHolder* h, uint32_t id, uint32_t* out) {
    if (out != nullptr) *out = 0;
    if (h == nullptr) return false;
    std::vector<uint32_t> v;
    if (!fc4::props::GetU32s(h, id, v) || v.empty()) return false;
    if (out != nullptr) *out = v[0];
    return true;
}

// Sets a uint32 property: replaced in place when present, else added.
void SetU32Property(cISCPropertyHolder* h, uint32_t id, uint32_t value) {
    if (h == nullptr) return;
    if (fc4::props::Has(h, id)) {
        fc4::props::SetTyped(h, id, kVariantUint32, false, &value, 1);
    } else {
        fc4::AsGame(h)->AddPropertyU32(id, value, false);  // holder slot 13
    }
}

// Inserted-order walk of a 193-bucket STLport hash_set (B): buckets
// ascending, each chain from its oldest node to its newest (the order
// inserting them into an empty set rebuilds it; FC4TractHost.cpp).
void HashSetValues(const fc4::GameHashSetU32<fc4::TractNodeAllocator>& set, std::vector<uint32_t>& out) {
    std::vector<uint32_t> chain;
    for (size_t i = 0; i < set.bucket_count(); ++i) {
        chain.clear();
        for (auto* n = set.bucket(i); n != nullptr; n = n->next) chain.push_back(n->value);
        for (auto it = chain.rbegin(); it != chain.rend(); ++it) out.push_back(*it);
    }
}

// ---- lifecycle (§1.3, §1.4) --------------------------------------------------------------------------

// §1.3 step 3: the city and ten services, each with a reference; the
// demolition utility without a check. The shadow's context has them
// borrowed already.
bool AcquireServices(void* u) {
    LOTDEV_TEST0(acquire_services);
    fc4::LotDevContext* c = Ctx(u);
    if (!c->readOnly) {
        TakeIfNull(c->city, AppCity());
        if (c->city == nullptr) {
            fc4::Log("lotdev: no city; services not acquired");
            return false;
        }
        TakeIfNull(c->lots, c->city->GetLotManager());
        TakeIfNull(c->occupants, c->city->GetOccupantManager());
        TakeIfNull(c->props, c->city->GetPropManager());
        TakeIfNull(c->bds, c->city->GetBuildingDevelopmentSimulator());
        TakeIfNull(c->power, c->city->GetPowerSimulator());
        TakeIfNull(c->plumbing, c->city->GetPlumbingSimulator());
        TakeIfNull(c->flora, c->city->GetFloraSimulator());
        TakeIfNull(c->terrain, c->city->GetTerrain());
        TakeIfNull(c->traffic, c->city->GetTrafficSimulator());
        TakeIfNull(c->demolition, c->city->GetDemolitionUtility());
    }
    const bool ok = c->city && c->lots && c->occupants && c->props && c->bds && c->power && c->plumbing && c->flora &&
                    c->terrain && c->traffic;
    if (!ok && !c->readOnly) {
        fc4::Log("lotdev: a service is missing (city=%p lots=%p occupants=%p props=%p bds=%p power=%p plumbing=%p "
                 "flora=%p terrain=%p traffic=%p)",
                 static_cast<void*>(c->city), static_cast<void*>(c->lots), static_cast<void*>(c->occupants),
                 static_cast<void*>(c->props), static_cast<void*>(c->bds), static_cast<void*>(c->power),
                 static_cast<void*>(c->plumbing), static_cast<void*>(c->flora), static_cast<void*>(c->terrain),
                 static_cast<void*>(c->traffic));
    }
    return ok;
}

void ReleaseServicesCb(void* u) {
    if (Ctx(u)->test != nullptr && Ctx(u)->test->release_services != nullptr) {
        Ctx(u)->test->release_services(Ctx(u)->test->user);
        return;
    }
    fc4::LotDevReleaseServices(Ctx(u));
}

// §3.5: the "Building Development" exemplar (NEW props for the core).
fc4_props* Tuning(void* u) {
    LOTDEV_TEST0(tuning);
    if (!Gate(u, fc4::Sw::ExemplarLoads, "lotdev tuning exemplar load (none)")) return nullptr;
    fc4::props::Exemplar ex;
    if (!ex.Load(kTuningType, kTuningGroup, kTuningInstance)) {
        if (!Ctx(u)->readOnly) fc4::Log("lotdev: tuning exemplar 0x%08X unavailable", kTuningInstance);
        return nullptr;
    }
    return fc4::props::ToCoreAll(ex.Holder());
}

// §1.3 step 8: the simulator (the shadow answers false: its Init stops there).
bool HasSimulator(void* u) {
    LOTDEV_TEST0(has_simulator);
    if (Ctx(u)->readOnly) return false;
    return City(u) != nullptr && City(u)->GetSimulator() != nullptr;
}

// §3.1: simulator slot 19 AddAgent(this + 4, level, "cSC4LotDeveloper", 2).
// [choice] lotdev_agents off: not registered (no day message then reaches
// the daily pass).
void AddAgent(void* u, uint32_t level, uint32_t message) {
    LOTDEV_TEST(add_agent, level, message);
    fc4::LotDevContext* c = Ctx(u);
    if (!Writes(u) || !Gate(u, fc4::Sw::LotdevAgents, "lotdev AddAgent (not registered)")) return;
    cFC4ISC4Simulator* sim = c->city ? fc4::AsGame(c->city->GetSimulator()) : nullptr;
    if (sim == nullptr || c->target == nullptr) {
        fc4::Log("lotdev: no simulator; agent level %u not registered", level);
        return;
    }
    const cRZBaseString name(kAgentName);
    if (!sim->AddAgent(c->target, level, name, kAgentFlags)) {  // slot 19
        fc4::Log("lotdev: AddAgent(level %u, message 0x%08X) returned false", level, message);
    }
    ++c->agentLevels;
}

// §1.4: simulator slot 20 RemoveAgent (all levels).
void RemoveAgent(void* u) {
    LOTDEV_TEST0(remove_agent);
    fc4::LotDevContext* c = Ctx(u);
    if (!Writes(u) || c->agentLevels == 0) return;
    cFC4ISC4Simulator* sim = c->city ? fc4::AsGame(c->city->GetSimulator()) : nullptr;
    if (sim != nullptr && c->target != nullptr) sim->RemoveAgentAllLevels(c->target);  // slot 20
    c->agentLevels = 0;
}

// §1.3 step 9.
bool AcquireMessageServer(void* u) {
    LOTDEV_TEST0(acquire_message_server);
    fc4::LotDevContext* c = Ctx(u);
    if (!Writes(u)) return false;
    if (c->msgServer == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            c->msgServer = ms;
            FC4_ADDREF(c->msgServer);
        }
    }
    return c->msgServer != nullptr;
}

void ReleaseMessageServer(void* u) {
    LOTDEV_TEST0(release_message_server);
    if (!Writes(u)) return;
    ReleaseSlot(Ctx(u)->msgServer);
}

void Subscribe(void* u, uint32_t message) {
    LOTDEV_TEST(subscribe, message);
    fc4::LotDevContext* c = Ctx(u);
    if (!Writes(u) || c->msgServer == nullptr || c->target == nullptr) return;
    c->msgServer->AddNotification(c->target, message);  // slot 5
}

void Unsubscribe(void* u, uint32_t message) {
    LOTDEV_TEST(unsubscribe, message);
    fc4::LotDevContext* c = Ctx(u);
    if (!Writes(u) || c->msgServer == nullptr || c->target == nullptr) return;
    c->msgServer->RemoveNotification(c->target, message);  // slot 6
}

// The animation-tick manager (framework service 0x86AD10EC, IID 0x86AD10ED).
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

// §1.3 step 10: slot 5 AddTickableObject(this + 4, 0xC0).
void RegisterAnimationTick(void* u) {
    LOTDEV_TEST0(register_animation_tick);
    fc4::LotDevContext* c = Ctx(u);
    if (!Writes(u) || c->target == nullptr) return;
    if (!Gate(u, fc4::Sw::LotdevAgents, "lotdev AddTickableObject (not registered)")) return;
    cFC4ISC4AnimationTickManager* m = TickManager();
    if (m == nullptr) {
        NoteOnce(g_noteTick, "lotdev: no animation-tick manager; the night lights are not batched");
        return;
    }
    m->AddTickableObject(c->target, kTickPriority);  // slot 5
    c->tickRegistered = true;
    FC4_RELEASE(static_cast<cIGZUnknown*>(m));
}

// §1.4: slot 6 RemoveTickableObject(this + 4, 0xC0).
void UnregisterAnimationTick(void* u) {
    LOTDEV_TEST0(unregister_animation_tick);
    fc4::LotDevContext* c = Ctx(u);
    if (!Writes(u) || !c->tickRegistered || c->target == nullptr) return;
    cFC4ISC4AnimationTickManager* m = TickManager();
    if (m != nullptr) {
        m->RemoveTickableObject(c->target, kTickPriority);  // slot 6
        FC4_RELEASE(static_cast<cIGZUnknown*>(m));
    }
    c->tickRegistered = false;
}

// §3.3: a new standard message, MessagePost(msg, false) (queued); the lot
// (if any) as its IGZUnknown.
void PostMessage(void* u, uint32_t type, uint32_t data1, uint64_t lot) {
    LOTDEV_TEST(post_message, type, data1, lot);
    if (!Writes(u)) return;
    const fc4::Sw sw = type == kMsgNightUpdated ? fc4::Sw::LotdevNight : fc4::Sw::LotdevProps;
    if (!Gate(u, sw, "lotdev message post (not sent)")) return;
    cIGZMessageServer2* ms = Ctx(u)->msgServer;
    if (ms == nullptr) return;
    cFC4Message2Standard* msg = FC4MakeMessage(type, static_cast<intptr_t>(data1), 0, 0);
    if (msg == nullptr) return;
    if (lot != 0) msg->SetIGZUnknown(FromToken(lot));
    // Deterministic pacing: a lot StateChanged is sent at the day's settle
    // point instead (FC4Pacing.h; simulator.md §5.7 [choice]).
    if (!fc4::PacingDefer(msg)) ms->MessagePost(msg, false);
    msg->Release();
    ++Ctx(u)->posts;
}

// ---- city, simulator, terrain ---------------------------------------------------------------------------

void CityCellCounts(void* u, int32_t* out2) {
    LOTDEV_TEST(city_cell_counts, out2);
    if (out2 == nullptr) return;
    cISC4City* c = City(u);
    out2[0] = c ? static_cast<int32_t>(c->CellCountX()) : 0;  // slot 93
    out2[1] = c ? static_cast<int32_t>(c->CellCountZ()) : 0;  // slot 94
}

void CityCellWidths(void* u, float* out2) {
    LOTDEV_TEST(city_cell_widths, out2);
    if (out2 == nullptr) return;
    cISC4City* c = City(u);
    out2[0] = c ? c->CellWidthX() : 16.0f;  // slot 91, ST0
    out2[1] = c ? c->CellWidthZ() : 16.0f;  // slot 92, ST0
}

// City slot 95 PositionToCell returns nothing (lot-developer.md §17.2,
// LD18): its AL is the low byte of the x87 status word, so it is called as
// the SDK declares it (void) and no result is read.
void PositionToCell(void* u, float x, float z, int32_t* out2) {
    LOTDEV_TEST(position_to_cell, x, z, out2);
    int cx = 0, cz = 0;
    if (cISC4City* c = City(u)) c->PositionToCell(x, z, cx, cz);  // slot 95
    if (out2 != nullptr) {
        out2[0] = cx;
        out2[1] = cz;
    }
}

void CellCornerToPosition(void* u, int32_t x, int32_t z, float* out2) {
    LOTDEV_TEST(cell_corner_to_position, x, z, out2);
    float fx = 0.0f, fz = 0.0f;
    if (cISC4City* c = City(u)) c->CellCornerToPosition(x, z, fx, fz);  // slot 96
    if (out2 != nullptr) {
        out2[0] = fx;
        out2[1] = fz;
    }
}

void CellCenterToPosition(void* u, int32_t x, int32_t z, float* out2) {
    LOTDEV_TEST(cell_center_to_position, x, z, out2);
    float fx = 0.0f, fz = 0.0f;
    if (cISC4City* c = City(u)) c->CellCenterToPosition(x, z, fx, fz);  // slot 97
    if (out2 != nullptr) {
        out2[0] = fx;
        out2[1] = fz;
    }
}

// Simulator slot 8 GetSimDateNumber.
uint32_t SimDate(void* u) {
    LOTDEV_TEST0(sim_date);
    cISC4City* c = City(u);
    cFC4ISC4Simulator* s = c ? fc4::AsGame(c->GetSimulator()) : nullptr;
    return s ? static_cast<uint32_t>(s->GetSimDateNumber()) : 0;
}

void TerrainCellCounts(void* u, int32_t* out2) {
    LOTDEV_TEST(terrain_cell_counts, out2);
    if (out2 == nullptr) return;
    cISTETerrain* t = Terrain(u);
    out2[0] = t ? static_cast<int32_t>(t->CellCountX()) : 0;  // slot 7
    out2[1] = t ? static_cast<int32_t>(t->CellCountZ()) : 0;  // slot 8
}

// Terrain slot 6 CellWidth: ST0 (the SDK declares a uint32): called raw.
float TerrainCellWidth(void* u) {
    LOTDEV_TEST0(terrain_cell_width);
    cISTETerrain* t = Terrain(u);
    if (t == nullptr) return 0.0f;
    using Fn = float(__fastcall*)(void*, void*);
    void** vt = *reinterpret_cast<void***>(t);
    return reinterpret_cast<Fn>(vt[6])(t, nullptr);
}

float TerrainAltitude(void* u, int32_t index) {
    LOTDEV_TEST(terrain_altitude, index);
    cISTETerrain* t = Terrain(u);
    return t ? t->GetAltitude(index) : 0.0f;  // slot 12
}

float TerrainWaterLevel(void* u, int32_t index) {
    LOTDEV_TEST(terrain_water_level, index);
    cISTETerrain* t = Terrain(u);
    return t ? t->GetWaterLevel(index) : 0.0f;  // slot 31
}

float TerrainAltitudeAt(void* u, float x, float z) {
    LOTDEV_TEST(terrain_altitude_at, x, z);
    cISTETerrain* t = Terrain(u);
    return t ? t->GetAltitude(x, z) : 0.0f;  // slot 13
}

float TerrainWaterLevelAt(void* u, float x, float z) {
    LOTDEV_TEST(terrain_water_level_at, x, z);
    cISTETerrain* t = Terrain(u);
    return t ? t->GetWaterLevel(x, z) : 0.0f;  // slot 30
}

bool TerrainIsWaterVertex(void* u, int32_t x, int32_t z) {
    LOTDEV_TEST(terrain_is_water_vertex, x, z);
    cISTETerrain* t = Terrain(u);
    return t != nullptr && t->IsWaterVertex(static_cast<uint32_t>(x), static_cast<uint32_t>(z));  // slot 37
}

bool TerrainCellHasWater(void* u, int32_t x, int32_t z) {
    LOTDEV_TEST(terrain_cell_has_water, x, z);
    cISTETerrain* t = Terrain(u);
    return t != nullptr && t->CellHasWater(static_cast<uint32_t>(x), static_cast<uint32_t>(z));  // slot 34
}

cFC4ISC4ZoneManager* Zones(void* u) {
    cISC4City* c = City(u);
    cISC4ZoneManager* z = c ? c->GetZoneManager() : nullptr;
    return z ? fc4::AsGame(z) : nullptr;
}

uint32_t ZoneDragColor(void* u, int32_t zone) {
    LOTDEV_TEST(zone_drag_color, zone);
    cFC4ISC4ZoneManager* z = Zones(u);
    return z ? z->GetZoneDragColor(zone) : 0;  // slot 15
}

uint8_t ZoneDisplayAlpha(void* u) {
    LOTDEV_TEST0(zone_display_alpha);
    cFC4ISC4ZoneManager* z = Zones(u);
    return z ? z->GetZoneDisplayAlpha() : uint8_t{0xAA};  // slot 16 (shipped 0xAA)
}

// §5.6 step 5 (LD12): the zot prop's orientation is field +0x10 of the
// global SC4ViewUtilities (0x00B43DD8), the current view rotation 0..3; the
// only use of the view in the lot developer. Read through the interfaces
// (as the landfill's zot, landfill.md §4.7.5): the app's main window, its
// child 0x6104489A, that child's cISC4View3DWin (0x9A47B417 / 0xFA47B3F9),
// its renderer (slot 4) and the renderer's view utilities (slot 8, not
// GZCOM). 0 without a view (logged once).
const uint32_t kWinSC4App = 0x6104489A;
const uint32_t kWinSC4View3DWin = 0x9A47B417;
const uint32_t kIID_cISC4View3DWin = 0xFA47B3F9;
const uintptr_t kViewUtilitiesRotation = 0x10;

int32_t ViewRotation(void* u) {
    LOTDEV_TEST0(view_rotation);
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        NoteOnce(g_noteZotView, "lotdev: no cISC4App; the zot's orientation is 0 (§5.6, LD12)");
        return 0;
    }
    FC4_ACQUIRED(sc4, "QueryInterface");
    int32_t rotation = 0;
    bool read = false;
    cISC4View3DWin* view = nullptr;
    auto* main = reinterpret_cast<cFC4IGZWin*>(sc4->GetMainWindow());                // borrowed
    cFC4IGZWin* parent = main ? main->GetChildWindowFromID(kWinSC4App) : nullptr;  // slot 34, borrowed
    if (parent != nullptr &&
        parent->GetChildAs(kWinSC4View3DWin, kIID_cISC4View3DWin, reinterpret_cast<void**>(&view)) &&  // slot 36
        view != nullptr) {
        FC4_ACQUIRED(view, "GetChildAs");
        auto* render = reinterpret_cast<cFC4ISC43DRender*>(view->GetRenderer());  // slot 4, borrowed
        void* utilities = render ? render->GetViewUtilities() : nullptr;           // slot 8, borrowed, not GZCOM
        if (utilities != nullptr) {
            rotation = *reinterpret_cast<const int32_t*>(static_cast<const uint8_t*>(utilities) +
                                                         kViewUtilitiesRotation);
            read = true;
        }
        FC4_RELEASE(view);
    }
    if (!read) NoteOnce(g_noteZotView, "lotdev: no city view; the zot's orientation is 0 (§5.6, LD12)");
    FC4_RELEASE(sc4);
    return rotation;
}

// §8.2: the 24-hour clock's slot 6 GetVisualTimeMode.
uint32_t VisualTimeMode(void* u) {
    LOTDEV_TEST0(visual_time_mode);
    cISC4City* c = City(u);
    cISC424HourClock* clock = c ? c->Get24HourClock() : nullptr;
    return clock ? static_cast<uint32_t>(clock->GetVisualTimeMode()) : 0;
}

// §6.6: resource manager slot 14 (TestForKey).
bool ResourceExists(void* u, uint32_t type, uint32_t group, uint32_t instance) {
    LOTDEV_TEST(resource_exists, type, group, instance);
    cIGZPersistResourceManagerPtr rm;
    return rm && rm->TestForKey(cGZPersistResourceKey(type, group, instance));
}

// ---- lot manager -------------------------------------------------------------------------------------

cFC4ISC4LotManager* LotManager(void* u) {
    cISC4LotManager* lm = Lots(u);
    return lm ? fc4::AsGame(lm) : nullptr;
}

int32_t LotCount(void* u) {
    LOTDEV_TEST0(lot_count);
    cFC4ISC4LotManager* lm = LotManager(u);
    return lm ? lm->GetLotCount() : 0;  // slot 16
}

// ---- the safe lot iterator (§4.1, §15.1) ----

struct RawStep {
    fc4::safelots::Block* block;
    const void* expectedList;
    void* lot;
    bool end;
    bool bad;
};

// The original's walk (§4.1): if the current node is the head the walk is
// over, else the lot is the node's value and current := next. Plain reads
// of the game's list: under SehGuard.
void RawNextFn(void* p) {
    auto* s = static_cast<RawStep*>(p);
    fc4::safelots::Block* b = s->block;
    if (b->listObject != s->expectedList) {
        s->bad = true;
        return;
    }
    fc4::safelots::Node* head = fc4::safelots::MasterHead(b);
    fc4::safelots::Node* cur = b->current;
    if (head == nullptr || cur == nullptr) {
        s->bad = true;
        return;
    }
    if (cur == head) {
        s->end = true;
        return;
    }
    if (cur->next == nullptr || cur->next->prev != cur) {
        s->bad = true;
        return;
    }
    s->lot = cur->value;
    b->current = cur->next;
}

// §4.1: iterator.cur := the first node of the lot list.
void RawRewindFn(void* p) {
    auto* s = static_cast<RawStep*>(p);
    if (s->block->listObject != s->expectedList) {
        s->bad = true;
        return;
    }
    fc4::safelots::Node* head = fc4::safelots::MasterHead(s->block);
    if (head == nullptr || head->next == nullptr) {
        s->bad = true;
        return;
    }
    s->block->current = head->next;
}

struct RawRelease {
    fc4::safelots::Block* block;
    fc4::safelots::FreeFn freeNode;
    fc4::safelots::FreeFn freeBlock;
    int removed;
};

void RawReleaseFn(void* p) {
    auto* m = static_cast<RawRelease*>(p);
    m->removed = fc4::safelots::Release(m->block, m->freeNode, m->freeBlock, kMaxRegisteredIterators);
}

// The allocators a block's release needs: B (its iterator list node) and C
// (the block), on the identified executable only.
bool RawAllocators(fc4::compat::StlNodeFreeFn* freeNode, uintptr_t* freeBlock) {
    *freeBlock = fc4::compat::DirectCall(fc4::compat::Direct::GlobalHeapFree);
    return fc4::compat::IdentifiedVersion() != nullptr && *freeBlock != 0 &&
           fc4::compat::StlNodeAllocator(nullptr, freeNode);
}

// The list object of the game's iterator blocks, when they can be walked as
// the original walks them: the identified executable (allocators B and C)
// and the original lot manager (its vtable in the image: lm + 0x5C) or
// FreeCity4's (enable = zonedev: its raw master list mirror,
// cFC4ZoneDeveloper::MasterListObject()); null otherwise, *why saying why.
const void* RawListObject(cFC4ISC4LotManager* lm, const char** why) {
    fc4::compat::StlNodeFreeFn freeNode = nullptr;
    uintptr_t freeBlock = 0;
    if (!RawAllocators(&freeNode, &freeBlock)) {
        *why = "executable not identified, or allocator B / C unavailable";
        return nullptr;
    }
    return cFC4ZoneDeveloper::SafeIteratorListObject(lm, why);
}

// The fallback: every lot of the city (GetLotsInCellRect over the whole
// city, one reference each, kept by the iterator).
void Snapshot(void* u, fc4::LotDevIterator* it) {
    for (cIGZUnknown* l : it->lots) FC4_RELEASE_CREATED(l);  // a lot deleted since: ours was the last
    it->lots.clear();
    it->next = 0;
    cFC4ISC4LotManager* lm = LotManager(u);
    cISC4City* c = City(u);
    if (lm == nullptr || c == nullptr || !ListsAvailable(u)) return;
    LotList list;
    if (!list.valid()) return;
    lm->GetLotsInCellRect(reinterpret_cast<fc4::GameList<cISC4Lot*>&>(list), 0, 0,
                          static_cast<int32_t>(c->CellCountX()) - 1, static_cast<int32_t>(c->CellCountZ()) - 1,
                          false);
    for (cISC4Lot*& lot : list) {
        if (lot == nullptr) continue;
        FC4_ACQUIRED(lot, "GetLotsInCellRect list entry");
        it->lots.push_back(reinterpret_cast<cIGZUnknown*>(lot));
        lot = nullptr;
    }
}

// The block came from a lot manager RawListObject accepted; its release reads
// only the block's own list object (null after FreeCity4's lot manager
// orphaned it at Shutdown: then nothing to unregister, BQ4).
void FreeRaw(fc4::LotDevIterator* it, cFC4ISC4LotManager* lm) {
    if (it->block == nullptr) return;
    fc4::compat::StlNodeFreeFn freeNode = nullptr;
    uintptr_t freeBlock = 0;
    bool released = false;
    if (lm != nullptr && RawAllocators(&freeNode, &freeBlock)) {
        RawRelease r{static_cast<fc4::safelots::Block*>(it->block), reinterpret_cast<fc4::safelots::FreeFn>(freeNode),
                     reinterpret_cast<fc4::safelots::FreeFn>(freeBlock), -1};
        released = fc4::SehGuard("lotdev safe lot iterator release", &RawReleaseFn, &r) && r.removed >= 0;
    }
    if (!released) {
        NoteOnce(g_noteIteratorLeak, "lotdev: WARNING a safe lot iterator could not be unregistered; its block is "
                                     "leaked, not freed");
    }
    it->block = nullptr;
    it->listObject = nullptr;
}

// Slot 15 GetSafeLotIterator: the game's block (walked raw) or the fallback.
uint64_t SafeLotIterator(void* u) {
    LOTDEV_TEST0(safe_lot_iterator);
    cFC4ISC4LotManager* lm = LotManager(u);
    if (lm == nullptr) return 0;
    auto* it = new fc4::LotDevIterator();
    const char* why = nullptr;
    const void* listObject = RawListObject(lm, &why);
    if (listObject != nullptr) {
        it->block = reinterpret_cast<void*>(lm->GetSafeLotIterator());  // slot 15
        it->listObject = listObject;
        if (it->block == nullptr) {
            why = "GetSafeLotIterator returned null";
        } else {
            RawStep s{static_cast<fc4::safelots::Block*>(it->block), listObject, nullptr, false, false};
            if (!fc4::SehGuard("lotdev safe lot iterator check", &RawRewindFn, &s) || s.bad) {
                why = "the new iterator failed its check (unexpected list object, or a broken list)";
                FreeRaw(it, lm);
            }
        }
    }
    if (it->block == nullptr) {
        it->listObject = nullptr;
        ++Ctx(u)->iteratorFallbacks;
        if (!g_noteIteratorFallback) {
            g_noteIteratorFallback = true;
            fc4::Log("lotdev: the lot manager's list is not walked raw (%s); the daily pass walks a "
                     "GetLotsInCellRect list instead (lot order: question)",
                     why != nullptr ? why : "?");
        }
        Snapshot(u, it);
    }
    Ctx(u)->iterators.push_back(it);
    return TokenOf(it);
}

fc4::LotDevIterator* IteratorOf(void* u, uint64_t t) {
    auto* it = reinterpret_cast<fc4::LotDevIterator*>(static_cast<uintptr_t>(t));
    const auto& v = Ctx(u)->iterators;
    return std::find(v.begin(), v.end(), it) != v.end() ? it : nullptr;
}

void SafeLotIteratorRewind(void* u, uint64_t t) {
    LOTDEV_TEST(safe_lot_iterator_rewind, t);
    fc4::LotDevIterator* it = IteratorOf(u, t);
    if (it == nullptr) return;
    it->faulted = false;
    if (it->block != nullptr) {
        cFC4ISC4LotManager* lm = LotManager(u);
        RawStep s{static_cast<fc4::safelots::Block*>(it->block), it->listObject, nullptr, false, false};
        if (lm == nullptr || it->listObject == nullptr ||
            !fc4::SehGuard("lotdev safe lot iterator rewind", &RawRewindFn, &s) || s.bad) {
            ++Ctx(u)->iteratorFaults;
            it->faulted = true;
        }
        return;
    }
    Snapshot(u, it);
}

// The lotdev_lot_log line of the previous lot, then the next lot's values.
void NoteLogLot(void* u, uint64_t lot);

uint64_t SafeLotIteratorNext(void* u, uint64_t t) {
    LOTDEV_TEST(safe_lot_iterator_next, t);
    fc4::LotDevIterator* it = IteratorOf(u, t);
    uint64_t lot = 0;
    if (it != nullptr && !it->faulted) {
        if (it->block != nullptr) {
            cFC4ISC4LotManager* lm = LotManager(u);
            RawStep s{static_cast<fc4::safelots::Block*>(it->block), it->listObject, nullptr, false, false};
            if (lm == nullptr || it->listObject == nullptr ||
                !fc4::SehGuard("lotdev safe lot iterator step", &RawNextFn, &s) || s.bad) {
                ++Ctx(u)->iteratorFaults;
                it->faulted = true;  // the end from now on (the next rewind tries again)
            } else if (!s.end) {
                lot = TokenOf(s.lot);
            }
        } else {
            // The fallback skips a lot deleted since the list was taken
            // (lot slot 6 IsInitialized false).
            while (it->next < it->lots.size()) {
                cIGZUnknown* l = it->lots[it->next++];
                if (l != nullptr && fc4::AsGame(reinterpret_cast<cISC4Lot*>(l))->IsInitialized()) {
                    lot = TokenOf(l);
                    break;
                }
            }
        }
    }
    NoteLogLot(u, lot);
    return lot;
}

void FreeIterator(void* u, fc4::LotDevIterator* it) {
    FreeRaw(it, LotManager(u));
    for (cIGZUnknown* l : it->lots) FC4_RELEASE_CREATED(l);
    it->lots.clear();
    delete it;
}

void SafeLotIteratorFree(void* u, uint64_t t) {
    LOTDEV_TEST(safe_lot_iterator_free, t);
    fc4::LotDevIterator* it = IteratorOf(u, t);
    if (it == nullptr) return;
    auto& v = Ctx(u)->iterators;
    v.erase(std::find(v.begin(), v.end(), it));
    FreeIterator(u, it);
}

// Slot 6 GetLot(x, z, originOnly): borrowed (the lot manager holds it).
uint64_t GetLot(void* u, int32_t x, int32_t z, bool originOnly) {
    LOTDEV_TEST(get_lot, x, z, originOnly);
    cFC4ISC4LotManager* lm = LotManager(u);
    return lm ? TokenOf(lm->GetLot(x, z, originOnly)) : 0;
}

// Slot 7 GetLotsInCellRect(list, x1, z1, x2, z2, originOnly).
void LotsInCellRect(void* u, const fc4_cell_rect* rect, bool originOnly, fc4_token_callback cb, void* cbCtx) {
    LOTDEV_TEST(lots_in_cell_rect, rect, originOnly, cb, cbCtx);
    cFC4ISC4LotManager* lm = LotManager(u);
    if (rect == nullptr || cb == nullptr || lm == nullptr || !ListsAvailable(u)) return;
    LotList list;
    if (!list.valid()) return;
    lm->GetLotsInCellRect(reinterpret_cast<fc4::GameList<cISC4Lot*>&>(list), rect->x1, rect->z1, rect->x2, rect->z2,
                          originOnly);
    Each(TakeLots(u, list, "GetLotsInCellRect list entry"), cb, cbCtx);
}

// Slots 9 / 10 on a fresh list each.
void LotsSurroundingLot(void* u, uint64_t lot, uint32_t flags, fc4_token_callback cb, void* cbCtx) {
    LOTDEV_TEST(lots_surrounding_lot, lot, flags, cb, cbCtx);
    cFC4ISC4LotManager* lm = LotManager(u);
    if (lot == 0 || cb == nullptr || lm == nullptr || !ListsAvailable(u)) return;
    LotList list;
    if (!list.valid()) return;
    lm->GetLotsSurroundingLot(SdkLotOf(lot), reinterpret_cast<fc4::GameList<cISC4Lot*>&>(list),
                              static_cast<int32_t>(flags));
    Each(TakeLots(u, list, "GetLotsSurroundingLot list entry"), cb, cbCtx);
}

void LotsSurroundingRect(void* u, const fc4_cell_rect* rect, uint32_t flags, fc4_token_callback cb, void* cbCtx) {
    LOTDEV_TEST(lots_surrounding_rect, rect, flags, cb, cbCtx);
    cFC4ISC4LotManager* lm = LotManager(u);
    if (rect == nullptr || cb == nullptr || lm == nullptr || !ListsAvailable(u)) return;
    LotList list;
    if (!list.valid()) return;
    const SC4Rect<long> r(rect->x1, rect->z1, rect->x2, rect->z2);
    lm->GetLotsSurroundingRect(r, reinterpret_cast<fc4::GameList<cISC4Lot*>&>(list), static_cast<int32_t>(flags));
    Each(TakeLots(u, list, "GetLotsSurroundingRect list entry"), cb, cbCtx);
}

// Slot 24 DeleteLot(lot). The lot stays alive (one reference of ours) until
// the outermost call returns: the core may still hold its token.
bool DeleteLot(void* u, uint64_t lot) {
    LOTDEV_TEST(delete_lot, lot);
    if (!Writes(u) || lot == 0 || !Gate(u, fc4::Sw::LotdevDemolition, "lotdev DeleteLot (refused)")) return false;
    cFC4ISC4LotManager* lm = LotManager(u);
    if (lm == nullptr) return false;
    cISC4Lot* l = SdkLotOf(lot);
    FC4_ADDREF(l);
    fc4::LotDevHold(Ctx(u), reinterpret_cast<cIGZUnknown*>(l), true);
    // [abi-ret] Slot 24 leaves EAX undefined (interfaces.tsv eax-or-void; zones.md §8.6 and the §11.3 row:
    // "no bool result"): the SDK's bool is garbage, so the call is the result.
    lm->DeleteLot(l);
    FC4_X87_CHECK("lotdev DeleteLot");
    ++Ctx(u)->lotsDeleted;
    return true;
}

// ---- lots (FC4Interfaces.h cFC4ISC4Lot) ----------------------------------------------------------------

bool LotWrites(void* u, uint64_t lot, const char* what) {
    return Writes(u) && lot != 0 && Gate(u, fc4::Sw::LotdevLotWrites, what);
}

bool LotIsInitialized(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_is_initialized, lot);
    return lot != 0 && LotOf(lot)->IsInitialized();  // slot 6
}
void LotLocation(void* u, uint64_t lot, int32_t* out2) {
    LOTDEV_TEST(lot_location, lot, out2);
    int32_t x = 0, z = 0;
    if (lot != 0) LotOf(lot)->GetLocation(x, z);  // slot 21
    if (out2 != nullptr) {
        out2[0] = x;
        out2[1] = z;
    }
}
void LotSize(void* u, uint64_t lot, uint32_t* out2) {
    LOTDEV_TEST(lot_size, lot, out2);
    uint8_t w = 0, h = 0;
    if (lot != 0) LotOf(lot)->GetSize(w, h);  // slot 22
    if (out2 != nullptr) {
        out2[0] = w;
        out2[1] = h;
    }
}
void LotBoundingRect(void* u, uint64_t lot, fc4_cell_rect* out) {
    LOTDEV_TEST(lot_bounding_rect, lot, out);
    if (out == nullptr) return;
    *out = fc4_cell_rect{0, 0, 0, 0};
    if (lot == 0) return;
    SC4Rect<int32_t> r;
    LotOf(lot)->GetBoundingRect(r);  // slot 14
    *out = fc4_cell_rect{r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY};
}
void LotCenter(void* u, uint64_t lot, float* out3) {
    LOTDEV_TEST(lot_center, lot, out3);
    if (out3 == nullptr) return;
    float c[3] = {0.0f, 0.0f, 0.0f};
    if (lot != 0) LotOf(lot)->GetCenter(c);  // slot 15
    std::memcpy(out3, c, sizeof(c));
}
int32_t LotZoneType(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_zone_type, lot);
    return lot ? static_cast<int32_t>(LotOf(lot)->GetZoneType()) : 0;  // slot 31
}
float LotAltitude(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_altitude, lot);
    return lot ? LotOf(lot)->GetAltitude() : 0.0f;  // slot 16
}
void LotSetAltitude(void* u, uint64_t lot, float altitude) {
    LOTDEV_TEST(lot_set_altitude, lot, altitude);
    if (LotWrites(u, lot, "lotdev lot SetAltitude")) LotOf(lot)->SetAltitude(altitude);  // slot 17
}
int32_t LotFacing(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_facing, lot);
    return lot ? LotOf(lot)->GetFacing() : 0;  // slot 23
}
uint32_t LotFlags(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_flags, lot);
    return lot ? LotOf(lot)->GetFlags() : 0;  // slot 12
}
void LotSetFlags(void* u, uint64_t lot, uint32_t flags) {
    LOTDEV_TEST(lot_set_flags, lot, flags);
    if (LotWrites(u, lot, "lotdev lot SetFlags")) LotOf(lot)->SetFlags(flags);  // slot 13
}
void LotSetVerticesLocked(void* u, uint64_t lot, bool locked) {
    LOTDEV_TEST(lot_set_vertices_locked, lot, locked);
    if (LotWrites(u, lot, "lotdev lot SetVerticesAreLocked")) LotOf(lot)->SetVerticesAreLocked(locked);  // slot 69
}
void LotUpdateSlope(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_update_slope, lot);
    if (LotWrites(u, lot, "lotdev lot UpdateSlope")) LotOf(lot)->UpdateSlope();  // slot 20
}
uint32_t LotState(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_state, lot);
    return lot ? static_cast<uint32_t>(LotOf(lot)->GetState()) : 0;  // slot 33
}
bool LotSetState(void* u, uint64_t lot, uint32_t state, uint32_t wealth, uint32_t reason) {
    LOTDEV_TEST(lot_set_state, lot, state, wealth, reason);
    if (!LotWrites(u, lot, "lotdev lot SetState")) return false;
    // [abi-ret] Lot slot 34 returns nothing defined (lot-records.md §13.2; interfaces.tsv eax-or-void). The value
    // is only passed through as our slot 8 AbandonBuilding's result, which lot-developer.md §11.1 gives as "AL of
    // SetState" (the original leaks the same register); no core decision reads it (question LD34).
    const bool ok = LotOf(lot)->SetState(static_cast<cFC4ISC4Lot::HabitationState>(state),
                                         static_cast<cFC4ISC4Lot::WealthType>(wealth),
                                         static_cast<cFC4ISC4Lot::VacateReason>(reason));  // slot 34
    FC4_X87_CHECK("lotdev lot SetState");
    return ok;
}
uint32_t LotCondition(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_condition, lot);
    return lot ? LotOf(lot)->GetCondition() : 0;  // slot 35
}
void LotSetCondition(void* u, uint64_t lot, uint32_t condition) {
    LOTDEV_TEST(lot_set_condition, lot, condition);
    if (LotWrites(u, lot, "lotdev lot SetCondition")) LotOf(lot)->SetCondition(condition);  // slot 36
}
bool LotIsLivable(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_is_livable, lot);
    return lot != 0 && LotOf(lot)->IsInLivableState();  // slot 37
}
uint32_t LotDaysSinceStateChange(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_days_since_state_change, lot);
    return lot ? LotOf(lot)->GetDaysSinceLastStateChange() : 0;  // slot 40
}
uint32_t LotBuildingType(void* u, uint64_t lot, bool flag) {
    LOTDEV_TEST(lot_building_type, lot, flag);
    return lot ? LotOf(lot)->GetBuildingType(flag) : 0;  // slot 46
}
void LotSetBuildingType(void* u, uint64_t lot, uint32_t building) {
    LOTDEV_TEST(lot_set_building_type, lot, building);
    if (LotWrites(u, lot, "lotdev lot SetBuildingType")) LotOf(lot)->SetBuildingType(building);  // slot 47
}
// Slot 48 GetBuilding: borrowed.
uint64_t LotBuilding(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_building, lot);
    return lot ? TokenOf(LotOf(lot)->GetBuilding()) : 0;
}
// Slot 9 GetLotConfiguration: no reference.
uint64_t LotConfiguration(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_configuration, lot);
    return lot ? TokenOf(LotOf(lot)->GetLotConfiguration()) : 0;
}
void LotSetConfiguration(void* u, uint64_t lot, uint64_t cfg) {
    LOTDEV_TEST(lot_set_configuration, lot, cfg);
    if (LotWrites(u, lot, "lotdev lot SetLotConfiguration")) LotOf(lot)->SetLotConfiguration(CfgOf(cfg));  // 10
}
uint32_t LotOccupantWealth(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_occupant_wealth, lot);
    return lot ? LotOf(lot)->GetOccupantWealth() : 0;  // slot 41
}
// Slot 61 GetOccupancy (§11.4, LD20): `void GetOccupancy(float& out)`, pop 4,
// EAX = the out pointer. Declared as an SC4Percentage returned by value,
// which MSVC passes the same way (a hidden result pointer, one stack dword,
// returned in EAX; vtable_layout_test checks it), so the float is read
// through the pointer, never from EAX.
float LotOccupancy(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_occupancy, lot);
    return lot ? LotOf(lot)->GetOccupancy().percentage : 0.0f;
}
bool LotNeedsPower(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_needs_power, lot);
    return lot != 0 && LotOf(lot)->GetNeedsPower();  // slot 65
}
bool LotHasPower(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_has_power, lot);
    return lot != 0 && LotOf(lot)->GetHasPower();  // slot 66
}
void LotSetHasPower(void* u, uint64_t lot, bool powered) {
    LOTDEV_TEST(lot_set_has_power, lot, powered);
    if (LotWrites(u, lot, "lotdev lot SetHasPower")) LotOf(lot)->SetHasPower(powered);  // slot 67
}
bool LotHasJobAccess(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_has_job_access, lot);
    return lot != 0 && LotOf(lot)->HasJobAccess();  // slot 27
}
bool LotHasRoadAccess(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_has_road_access, lot);
    return lot != 0 && LotOf(lot)->HasRoadAccess();  // slot 28
}
bool LotHasOrganicRoadAccess(void* u, uint64_t lot, uint32_t purpose) {
    LOTDEV_TEST(lot_has_organic_road_access, lot, purpose);
    return lot != 0 && LotOf(lot)->HasOrganicRoadAccess(purpose);  // slot 29
}
void LotUpdateRoadAccess(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_update_road_access, lot);
    if (LotWrites(u, lot, "lotdev lot UpdateRoadAccess")) LotOf(lot)->UpdateRoadAccess();  // slot 30
}
float LotTravelJobs(void* u, uint64_t lot, uint32_t wealth) {
    LOTDEV_TEST(lot_travel_jobs, lot, wealth);
    return lot ? LotOf(lot)->GetTravelJobs(static_cast<cFC4ISC4Lot::WealthType>(wealth)) : 0.0f;  // slot 63
}
uint64_t LotTravelDesignate(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_travel_designate, lot);
    return lot ? TokenOf(LotOf(lot)->GetTravelDesignate()) : 0;  // slot 59
}
void LotSetTravelDesignate(void* u, uint64_t lot, uint64_t designate) {
    LOTDEV_TEST(lot_set_travel_designate, lot, designate);
    if (LotWrites(u, lot, "lotdev lot SetTravelDesignate")) LotOf(lot)->SetTravelDesignate(SdkLotOf(designate));  // 60
}
uint64_t LotUnit(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_unit, lot);
    return lot ? TokenOf(LotOf(lot)->GetUnit()) : 0;  // slot 25
}
void LotResetTravelPopulation(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_reset_travel_population, lot);
    if (LotWrites(u, lot, "lotdev lot ResetTravelPopulation")) LotOf(lot)->ResetTravelPopulation();  // slot 57
}
void LotAddTravelPopulation(void* u, uint64_t lot, uint64_t from) {
    LOTDEV_TEST(lot_add_travel_population, lot, from);
    if (LotWrites(u, lot, "lotdev lot AddTravelPopulation")) LotOf(lot)->AddTravelPopulation(SdkLotOf(from));  // 58
}
bool LotIsHistorical(void* u, uint64_t lot) {
    LOTDEV_TEST(lot_is_historical, lot);
    return lot != 0 && LotOf(lot)->IsHistorical();  // slot 43
}
void LotSetHistorical(void* u, uint64_t lot, bool historical) {
    LOTDEV_TEST(lot_set_historical, lot, historical);
    if (LotWrites(u, lot, "lotdev lot SetHistorical")) LotOf(lot)->SetHistorical(historical);  // slot 44
}
uint16_t LotAvailableCapacity(void* u, uint64_t lot, uint32_t devType, bool flag) {
    LOTDEV_TEST(lot_available_capacity, lot, devType, flag);
    return lot ? LotOf(lot)->GetAvailableCapacity(static_cast<cFC4ISC4Lot::DeveloperType>(devType), flag)
               : uint16_t{0};  // slot 52
}
bool LotSetPopulation(void* u, uint64_t lot, uint32_t devType, uint16_t population) {
    LOTDEV_TEST(lot_set_population, lot, devType, population);
    if (!LotWrites(u, lot, "lotdev lot SetPopulation")) return false;
    return LotOf(lot)->SetPopulation(static_cast<cFC4ISC4Lot::DeveloperType>(devType), population);  // slot 55
}
bool LotProperty(void* u, uint64_t lot, uint32_t id, uint32_t* out) {
    LOTDEV_TEST(lot_property, lot, id, out);
    if (lot == 0) {
        if (out != nullptr) *out = 0;
        return false;
    }
    return U32Property(LotOf(lot)->AsPropertyHolder(), id, out);  // lot slot 3
}
void LotSetProperty(void* u, uint64_t lot, uint32_t id, uint32_t value) {
    LOTDEV_TEST(lot_set_property, lot, id, value);
    if (LotWrites(u, lot, "lotdev lot property")) SetU32Property(LotOf(lot)->AsPropertyHolder(), id, value);
}

// ---- lot configurations and their manager (SDK slots match the compiled ones) ---------------------------

uint32_t CfgId(void* u, uint64_t cfg) {
    LOTDEV_TEST(cfg_id, cfg);
    return cfg ? CfgOf(cfg)->GetID() : 0;  // slot 3
}
void CfgSize(void* u, uint64_t cfg, uint32_t* out2) {
    LOTDEV_TEST(cfg_size, cfg, out2);
    uint32_t w = 0, h = 0;
    if (cfg != 0) CfgOf(cfg)->GetSize(w, h);  // slot 7
    if (out2 != nullptr) {
        out2[0] = w;
        out2[1] = h;
    }
}
float CfgMaxSlopeBeforeFoundation(void* u, uint64_t cfg) {
    LOTDEV_TEST(cfg_max_slope_before_foundation, cfg);
    return cfg ? CfgOf(cfg)->GetMaximumSlopeBeforeFoundation() : 0.0f;  // slot 10, ST0
}
uint8_t CfgGrowthStage(void* u, uint64_t cfg) {
    LOTDEV_TEST(cfg_growth_stage, cfg);
    return cfg ? CfgOf(cfg)->GetGrowthStage() : uint8_t{0};  // slot 11
}
bool CfgIsCompatibleWithWater(void* u, uint64_t cfg) {
    LOTDEV_TEST(cfg_is_compatible_with_water, cfg);
    return cfg != 0 && CfgOf(cfg)->IsCompatibleWithWater();  // slot 14
}
// Slot 23: the objects in array order.
void CfgObjects(void* u, uint64_t cfg, fc4_lotdev_cfg_object_callback cb, void* cbCtx) {
    LOTDEV_TEST(cfg_objects, cfg, cb, cbCtx);
    if (cfg == 0 || cb == nullptr) return;
    SC4Vector<cSC4LotConfigurationObject>* v = CfgOf(cfg)->GetLotConfigurationObjectArray();
    if (v == nullptr) return;
    std::vector<uint32_t> ids;
    for (cSC4LotConfigurationObject* o = v->begin(); o != v->end(); ++o) {
        const fc4_lotdev_cfg_object c = fc4::LotDevObjectOf(o, ids);
        if (c.object_type == 1) ++((c.flags & 1) ? Ctx(u)->cfgPropsOptional : Ctx(u)->cfgProps);
        if (c.object_type == 4) ++(c.n_ids ? Ctx(u)->cfgFlora : Ctx(u)->cfgFloraNoIds);
        cb(cbCtx, &c);
    }
}
bool CfgPrimaryBuildingObject(void* u, uint64_t cfg, fc4_lotdev_cfg_object_callback cb, void* cbCtx) {
    LOTDEV_TEST(cfg_primary_building_object, cfg, cb, cbCtx);
    cSC4LotConfigurationObject* o = cfg ? CfgOf(cfg)->GetPrimaryBuildingConfigurationObject() : nullptr;  // 24
    if (o == nullptr) return false;
    std::vector<uint32_t> ids;
    const fc4_lotdev_cfg_object c = fc4::LotDevObjectOf(o, ids);
    if (cb != nullptr) cb(cbCtx, &c);
    return true;
}
bool CfgPrimaryFoundationObject(void* u, uint64_t cfg, fc4_lotdev_cfg_object_callback cb, void* cbCtx) {
    LOTDEV_TEST(cfg_primary_foundation_object, cfg, cb, cbCtx);
    cSC4LotConfigurationObject* o = cfg ? CfgOf(cfg)->GetPrimaryFoundationConfigurationObject() : nullptr;  // 26
    if (o == nullptr) return false;
    std::vector<uint32_t> ids;
    const fc4_lotdev_cfg_object c = fc4::LotDevObjectOf(o, ids);
    if (cb != nullptr) cb(cbCtx, &c);
    return true;
}
bool CfgHasObjectsOfType(void* u, uint64_t cfg, uint32_t type) {
    LOTDEV_TEST(cfg_has_objects_of_type, cfg, type);
    return cfg != 0 && CfgOf(cfg)->HasObjectsOfType(type);  // slot 27
}
// Manager slot 8: no reference.
uint64_t GetLotConfiguration(void* u, uint32_t id) {
    LOTDEV_TEST(get_lot_configuration, id);
    cFC4ISC4LotConfigurationManager* m = Configs(u);
    return m ? TokenOf(m->GetLotConfiguration(id)) : 0;
}
// Manager slot 27 GetObjectIDArray(obj, ids) into a vector of ours (B).
void ObjectIdArray(void* u, uint64_t cfg, uint32_t index, fc4_u32s_sink sink, void* sinkCtx) {
    LOTDEV_TEST(object_id_array, cfg, index, sink, sinkCtx);
    if (cfg == 0 || sink == nullptr) return;
    cFC4ISC4LotConfigurationManager* m = Configs(u);
    cSC4LotConfigurationObject* o = fc4::LotDevObjectAt(CfgOf(cfg), index);
    if (m == nullptr || o == nullptr || !ListsAvailable(u)) return;
    fc4::GameVector<uint32_t, fc4::TractNodeAllocator> ids;
    m->GetObjectIDArray(o, &ids);
    std::vector<uint32_t> copy(ids.begin(), ids.end());
    ++Ctx(u)->idArrays;
    if (copy.empty()) ++Ctx(u)->idArraysEmpty;
    if (!copy.empty()) sink(sinkCtx, copy.data(), copy.size());
}

// ---- slots and helpers §17 does not cover ----------------------------------------------------------------

uint64_t FirstBuilding(void* u, uint64_t lot) {
    LOTDEV_TEST(first_building, lot);
    return TokenOf(fc4::LotDevFirstBuilding(Ctx(u), SdkLotOf(lot)));
}

// The retaining-wall helpers (§7.2, LD3; docs/abi/direct-calls.md): static
// functions of cSC4LotRetainingWallOccupant, __cdecl (the caller pops),
// taking the cISC4Lot interface pointer. Called by address behind the
// executable check (fc4::compat::DirectCall) and the lotdev_terrain switch;
// when either refuses, no wall is made or updated (logged once).
using RetainingWallAddFn = bool(__cdecl*)(cISC4Lot* lot, uint32_t add);  // 0x005E3070, pops 8 (caller), AL
using RetainingWallLotFn = bool(__cdecl*)(cISC4Lot* lot);                // 0x005E37F0, pops 4 (caller), AL
using RetainingWallVoidFn = void(__cdecl*)(cISC4Lot* lot);               // 0x005E3960, pops 4 (caller)

uintptr_t RetainingWallCall(void* u, fc4::compat::Direct which, uint64_t lot, const char* what) {
    if (!Writes(u) || lot == 0 || !Gate(u, fc4::Sw::LotdevTerrain, what)) return 0;
    const uintptr_t fn = fc4::compat::DirectCall(which);
    if (fn == 0) {
        NoteOnce(g_noteRetainingWalls, "lotdev: the retaining-wall helpers are not available (executable not "
                                       "identified or address rejected): no walls made or updated (§7.2, LD3)");
    } else {
        ++Ctx(u)->wallCalls;
    }
    return fn;
}

// Slot 62 UpdateLotRetainingWalls(lot) (§7.2, LD3): UpdateRetainingWalls
// (0x005E37F0); AL is that call's leftover (no defined value). False when
// the call is refused.
bool UpdateLotRetainingWalls(void* u, uint64_t lot) {
    LOTDEV_TEST(update_lot_retaining_walls, lot);
    const uintptr_t fn =
        RetainingWallCall(u, fc4::compat::Direct::RetainingWallsUpdate, lot, "lotdev UpdateRetainingWalls");
    if (fn == 0) return false;
    const bool al = reinterpret_cast<RetainingWallLotFn>(fn)(SdkLotOf(lot));
    FC4_X87_CHECK("lotdev UpdateRetainingWalls");
    Ctx(u)->expireAll = true;  // the lot's walls may be removed and rebuilt
    return al;
}

// UpdateLotObjectAlitudes (§6.5, LD17) is the core's; these are its game
// calls. Lot slot 49 GetLotOccupants: the lot's occupants in list order, one
// reference each kept for the call (the original's A list holds referenced
// occupants and releases them one by one).
void LotOccupants(void* u, uint64_t lot, fc4_token_callback cb, void* cbCtx) {
    LOTDEV_TEST(lot_occupants, lot, cb, cbCtx);
    if (lot == 0 || cb == nullptr) return;
    SC4List<cISC4Occupant*> list;
    LotOf(lot)->GetLotOccupants(list);  // slot 49
    Each(TakeOccupants(u, list, "GetLotOccupants list entry"), cb, cbCtx);
}

// Lot slot 11 GetLotConfigurationObject(occ): the configuration object the
// occupant was made from (a pointer into the configuration's array), or
// none.
bool LotConfigurationObject(void* u, uint64_t lot, uint64_t occ, fc4_lotdev_cfg_object_callback cb, void* cbCtx) {
    LOTDEV_TEST(lot_configuration_object, lot, occ, cb, cbCtx);
    if (lot == 0 || occ == 0) return false;
    cISC4Occupant* o = Occ(occ);
    if (o == nullptr) return false;
    cSC4LotConfigurationObject* obj = LotOf(lot)->GetLotConfigurationObject(o);  // slot 11
    if (obj == nullptr) return false;
    std::vector<uint32_t> ids;
    const fc4_lotdev_cfg_object c = fc4::LotDevObjectOf(obj, ids);
    if (cb != nullptr) cb(cbCtx, &c);
    return true;
}

// Occupant manager slot 24 (occ, 1): the occupant moved (UpdateLotObjectAlitudes
// after SetPosition).
void OccupantMoved(void* u, uint64_t occ) {
    LOTDEV_TEST(occupant_moved, occ);
    cISC4OccupantManager* om = Occupants(u);
    if (!Writes(u) || om == nullptr) return;
    if (cISC4Occupant* o = Occ(occ)) {
        om->MoveOccupant(o, true);  // slot 24
        ++Ctx(u)->objectsMoved;
    }
    FC4_X87_CHECK("lotdev MoveOccupant");
}

// Slot 41's lookup (§6.6, LD16 / LD24): occupant manager slot 33
// GetFirstOccupantByStandardCityCell(&o, x, z, base texture) at the lot's
// location cell (lot slot 21): the first base-texture occupant in that cell's
// bucket order whose box overlaps it. The lot pointer and the usage are NOT
// checked: whichever base texture comes first at the cell is taken. The hit
// carries a reference (occupant-manager.md §7), kept for the call.
cISC4Occupant* FindBaseTexture(void* u, cISC4Lot* lot) {
    cISC4OccupantManager* om = Occupants(u);
    if (lot == nullptr || om == nullptr) return nullptr;
    int32_t x = 0, z = 0;
    fc4::AsGame(lot)->GetLocation(x, z);  // slot 21
    cISC4Occupant* o = nullptr;
    if (!om->GetFirstOccupantByStandardCityCell(o, x, z, kOccupantTypeBaseTexture) || o == nullptr) {  // slot 33
        return nullptr;
    }
    FC4_ACQUIRED(o, "GetFirstOccupantByStandardCityCell");
    fc4::LotDevHold(Ctx(u), o);
    return o;
}

// Slot 41 step 3 (§6.6): the found occupant as cISC4LotBaseTextureOccupant
// (borrowed: the query's reference is dropped, the hold keeps it), given
// SetUsage(2) (base-texture slot 5) when the context writes. Null when the
// query fails (then slot 41 answers false and does not create one).
cISC4LotBaseTextureOccupant* FoundBaseTexture(void* u, cISC4Occupant* o) {
    if (o == nullptr) return nullptr;
    cISC4LotBaseTextureOccupant* tex = As<cISC4LotBaseTextureOccupant>(TokenOf(o), kIID_cISC4LotBaseTextureOccupant);
    if (tex != nullptr && Writes(u) && Gate(u, fc4::Sw::LotdevTextures, "lotdev base-texture SetUsage")) {
        tex->SetUsage(kLotTextureUsage);  // slot 5
    }
    return tex;
}

// Slot 45 UpdateLotBaseTextureOccupant(lot, force) (§6.6, LD16): slot 41
// without create, then the occupant's UpdateVertexAltitudes(true, force,
// null) (base-texture slot 9), which rebounds it from the terrain and, when
// forced or its box changed, notifies the occupant manager and rebuilds its
// models. AL: what that call left; false when there is no occupant.
bool UpdateLotBaseTextureOccupant(void* u, uint64_t lot, bool force) {
    LOTDEV_TEST(update_lot_base_texture_occupant, lot, force);
    if (!Writes(u) || lot == 0 || !Gate(u, fc4::Sw::LotdevTextures, "lotdev UpdateLotBaseTextureOccupant")) {
        return false;
    }
    cISC4LotBaseTextureOccupant* tex = FoundBaseTexture(u, FindBaseTexture(u, SdkLotOf(lot)));
    if (tex == nullptr) return false;
    // [abi-ret] Base-texture slot 9 (vtables.tsv eax-or-void) has no spec'd result (question LD33); it is passed
    // through as slot 45's AL, which lot-developer.md §6.6 leaves undefined and the core does not read.
    const bool ok = tex->UpdateVertexAltitudes(true, force, nullptr);  // slot 9
    FC4_X87_CHECK("lotdev UpdateVertexAltitudes");
    return ok;
}

uint64_t BaseTextureOccupant(void* u, uint64_t lot, bool create) {
    LOTDEV_TEST(base_texture_occupant, lot, create);
    return TokenOf(fc4::LotDevBaseTextureOccupant(Ctx(u), SdkLotOf(lot), create));
}

// The record SetTextureSpecification takes (lot-records.md §6; landfill's
// TextureRecord): 14 bytes as saved, padded to 16.
struct TextureRecord {
    uint32_t texture;
    uint8_t x;
    uint8_t z;
    uint8_t orientation;
    uint8_t objectByte3;
    uint8_t extra[5];
    uint8_t frame;
    uint8_t pad[2];
};
static_assert(sizeof(TextureRecord) == 16, "texture record stride");

// The base-texture occupant's slot 8 SetTextureSpecification(records, n, 0).
void BaseTextureSet(void* u, uint64_t occ, const fc4_lotdev_texture* records, size_t n) {
    LOTDEV_TEST(base_texture_set, occ, records, n);
    if (!Writes(u) || occ == 0 || !Gate(u, fc4::Sw::LotdevTextures, "lotdev SetTextureSpecification")) return;
    cISC4LotBaseTextureOccupant* tex = As<cISC4LotBaseTextureOccupant>(occ, kIID_cISC4LotBaseTextureOccupant);
    if (tex == nullptr) return;
    std::vector<TextureRecord> recs(n);
    for (size_t i = 0; i < n; ++i) {
        TextureRecord& r = recs[i];
        r = TextureRecord{};
        r.texture = records[i].texture;
        r.x = static_cast<uint8_t>(records[i].x);
        r.z = static_cast<uint8_t>(records[i].z);
        r.orientation = records[i].orientation;
        r.objectByte3 = records[i].byte3;
        std::memcpy(r.extra, records[i].extra, sizeof(r.extra));
        r.frame = records[i].frame;
    }
    tex->SetTextureSpecification(
        reinterpret_cast<const cISC4LotBaseTextureOccupant::LotBaseTextureSpecification*>(recs.empty() ? nullptr
                                                                                                         : recs.data()),
        static_cast<uint32_t>(n), nullptr);  // slot 8
    ++Ctx(u)->texturesSet;
}

// AddRetainingWallOccupant(lot, add) (0x005E3070; §7.2, LD3): `add` 0
// removes the lot's wall occupant, 1 builds and inserts one. Its AL (a wall
// was removed / inserted) is not read by the lot developer's callers.
void AddRetainingWallOccupant(void* u, uint64_t lot, uint32_t flag) {
    LOTDEV_TEST(add_retaining_wall_occupant, lot, flag);
    const uintptr_t fn =
        RetainingWallCall(u, fc4::compat::Direct::RetainingWallAdd, lot, "lotdev AddRetainingWallOccupant");
    if (fn == 0) return;
    reinterpret_cast<RetainingWallAddFn>(fn)(SdkLotOf(lot), flag != 0 ? 1u : 0u);
    FC4_X87_CHECK("lotdev AddRetainingWallOccupant");
    Ctx(u)->expireAll = true;
}

// UpdateSurroundingRetainingWalls(lot) (0x005E3960; §7.2, LD3).
void UpdateSurroundingRetainingWalls(void* u, uint64_t lot) {
    LOTDEV_TEST(update_surrounding_retaining_walls, lot);
    const uintptr_t fn = RetainingWallCall(u, fc4::compat::Direct::RetainingWallsUpdateSurrounding, lot,
                                           "lotdev UpdateSurroundingRetainingWalls");
    if (fn == 0) return;
    reinterpret_cast<RetainingWallVoidFn>(fn)(SdkLotOf(lot));
    FC4_X87_CHECK("lotdev UpdateSurroundingRetainingWalls");
    Ctx(u)->expireAll = true;
}

// cFC4ISC4NetworkManager: FC4LotDevInterfaces.h (the SDK header puts slots 14+ one late).
constexpr uint32_t kOccupantTypeNetwork = 0xC772BF98;
constexpr uint32_t kIID_cISC4NetworkOccupant = 0xA821EF94;
constexpr uint32_t kNetworkFlagConnectionNode = 0x200000;

// The city's network manager (borrowed), or null.
cFC4ISC4NetworkManager* NetworkManager(void* u) {
    cISC4City* c = City(u);
    return c ? reinterpret_cast<cFC4ISC4NetworkManager*>(c->GetNetworkManager()) : nullptr;
}

// AddNetworkConnectionNodes steps 2-3 (§6.5, LD17; the core gives the
// nodes): network manager slot 20 CreateConnectionNode for each node in
// order, each result appended to one list whatever the call returned, then
// slot 21 AutoconnectLotNodes(lot, list) (also with no node); then the list
// releases its entries (the original's occupant manager slot 48) and is
// freed. [choice, LD32] A failed call that left `out` null appends nothing:
// the original's list would carry a null that AutoconnectLotNodes reads.
void AddNetworkConnectionNodes(void* u, uint64_t lot, const fc4_lotdev_conn_node* nodes, size_t n) {
    LOTDEV_TEST(add_network_connection_nodes, lot, nodes, n);
    fc4::LotDevContext* ctx = Ctx(u);
    ++ctx->completions;
    cFC4ISC4NetworkManager* nm = NetworkManager(u);
    if (!Writes(u) || lot == 0 || nm == nullptr ||
        !Gate(u, fc4::Sw::LotdevBuildings, "lotdev AddNetworkConnectionNodes (no nodes)")) {
        return;
    }
    std::vector<cISC4Occupant*> made;
    for (size_t i = 0; i < n && nodes != nullptr; ++i) {
        const fc4_lotdev_conn_node& c = nodes[i];
        cISC4Occupant* o = nullptr;
        const SC4Point<int> cell(c.x, c.z);
        const int32_t type = static_cast<int32_t>(c.node_type);
        const bool ok = nm->CreateConnectionNode(o, type, cell, c.b, c.c, c.d, c.orientation);  // 20, abi: via cFC4I
        FC4_X87_CHECK("lotdev CreateConnectionNode");
        if (o == nullptr) {
            ++ctx->nodesFailed;
            continue;
        }
        FC4_ACQUIRED(o, "CreateConnectionNode");
        ++(ok ? ctx->nodesMade : ctx->nodesFailed);
        made.push_back(o);
    }
    {
        SC4List<cISC4Occupant*> list;
        for (cISC4Occupant*& o : made) list.push_back(o);
        nm->AutoconnectLotNodes(SdkLotOf(lot), list);  // slot 21, abi: via cFC4I
        FC4_X87_CHECK("lotdev AutoconnectLotNodes");
        ++ctx->autoconnects;
        // The list's references are ours (made): released here, not by the list.
        for (cISC4Occupant*& o : list) o = nullptr;
    }
    for (cISC4Occupant* o : made) FC4_RELEASE(o);
}

// RemoveNetworkConnectionNodes(lot) (§6.5, LD17): the network occupants
// (0xC772BF98) of the lot's bounding rectangle (occupant manager slot 41,
// maxCount -1; nothing more when it fails), those whose network occupant
// (QueryInterface 0xA821EF94) has every bit of 0x200000 (slot 8) go to
// network manager slot 6 DestroyNetworkOccupants; the rest are dropped.
void RemoveNetworkConnectionNodes(void* u, uint64_t lot) {
    LOTDEV_TEST(remove_network_connection_nodes, lot);
    cISC4OccupantManager* om = Occupants(u);
    cFC4ISC4NetworkManager* nm = NetworkManager(u);
    if (!Writes(u) || lot == 0 || om == nullptr || nm == nullptr ||
        !Gate(u, fc4::Sw::LotdevDemolition, "lotdev RemoveNetworkConnectionNodes (kept)")) {
        return;
    }
    SC4Rect<int32_t> r;
    LotOf(lot)->GetBoundingRect(r);  // slot 14
    SC4List<cISC4Occupant*> found;
    const SC4Point<int> a(r.topLeftX, r.topLeftY);
    const SC4Point<int> b(r.bottomRightX, r.bottomRightY);
    const bool ok = om->GetOccupantsByStandardCityCells(found, &a, &b, kOccupantTypeNetwork, 0xFFFFFFFFu);  // 41
    const std::vector<uint64_t> tokens = TakeOccupants(u, found, "GetOccupantsByStandardCityCells list entry");
    if (!ok) return;
    std::vector<cISC4Occupant*> nodes;
    for (uint64_t t : tokens) {
        cISC4NetworkOccupant* no = As<cISC4NetworkOccupant>(t, kIID_cISC4NetworkOccupant);
        if (no != nullptr && no->HasNetworkFlag(kNetworkFlagConnectionNode)) {  // slot 8
            nodes.push_back(reinterpret_cast<cISC4Occupant*>(FromToken(t)));
        }
    }
    SC4List<cISC4Occupant*> list;
    for (cISC4Occupant*& o : nodes) {
        list.push_back(o);
        fc4::LotDevExpire(Ctx(u), o);
    }
    nm->DestroyNetworkOccupants(list);  // slot 6 (also with none, as the original)
    FC4_X87_CHECK("lotdev DestroyNetworkOccupants");
    Ctx(u)->nodesDestroyed += static_cast<uint32_t>(nodes.size());
    for (cISC4Occupant*& o : list) o = nullptr;  // the held references are released after the call
}

// cSC4ConstructionOccupant (class 0xA97F909E, IID 0x496E7636) as §6.3 uses it.
class cFC4ConstructionOccupantView : public cIGZUnknown {
public:
    virtual cISC4Occupant* AsOccupant() = 0;  // 3
    virtual intptr_t SetLot(cISC4Lot* lot) = 0;  // 4
};

// §6.3 (LD25: the occupant comes from the COM directory, not the BDS):
// CreateInstance(0xA97F909E, 0x496E7636), SetLot(lot), InsertOccupant
// (occupant manager slot 20), release. The class is registered, so the
// original's direct construction when GZCOM fails is never reached; it is
// not reproduced (refused, logged once).
void StartConstructionOccupant(void* u, uint64_t lot) {
    LOTDEV_TEST(start_construction_occupant, lot);
    if (!Writes(u) || lot == 0 || !Gate(u, fc4::Sw::LotdevBuildings, "lotdev construction occupant (none)")) return;
    cISC4OccupantManager* om = Occupants(u);
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    if (om == nullptr || com == nullptr) return;
    auto* co = fc4::CreateConfiguredConstruction(LotOf(lot), [com]() -> cFC4ConstructionOccupantView* {
        void* p = nullptr;
        if (!com->GetClassObject(kCLSID_cSC4ConstructionOccupant, kIID_cSC4ConstructionOccupant, &p) || !p) {
            NoteOnce(g_noteConstruction, "lotdev: WARNING could not create a construction occupant through GZCOM (class "
                                         "0xA97F909E); animated construction shows no scene");
            return nullptr;
        }
        auto* created = static_cast<cFC4ConstructionOccupantView*>(p);
        FC4_ACQUIRED(static_cast<cIGZUnknown*>(created), "GetClassObject");
        return created;
    });
    if (!co) {
        fc4::Log("lotdev: construction scene not published for lot %p (building/factory unavailable)", SdkLotOf(lot));
        return;
    }
    if (cISC4Occupant* o = co->AsOccupant()) {
        fc4::Log("lotdev: configured construction occupant %p for lot %p", o, SdkLotOf(lot));
        om->InsertOccupant(o, 0);  // slot 20, only after SetLot completes
    }
    FC4_X87_CHECK("lotdev construction occupant");
    ++Ctx(u)->constructionOccupants;
    FC4_RELEASE(static_cast<cIGZUnknown*>(co));
}

// ---- buildings and other occupants -------------------------------------------------------------------

// QueryInterface for cISC4BuildingOccupant: the reference is kept for the call.
uint64_t OccupantAsBuilding(void* u, uint64_t occ) {
    LOTDEV_TEST(occupant_as_building, occ);
    if (occ == 0) return 0;
    void* p = nullptr;
    if (!FromToken(occ)->QueryInterface(kIID_cISC4BuildingOccupant, &p) || p == nullptr) return 0;
    auto* b = static_cast<cIGZUnknown*>(p);
    FC4_ACQUIRED(b, "QueryInterface");
    fc4::LotDevHold(Ctx(u), b);
    return TokenOf(p);
}

// §8.1: the night deque's own reference.
void BuildingAddRef(void* u, uint64_t building) {
    LOTDEV_TEST(building_add_ref, building);
    if (building != 0) FC4_ADDREF(FromToken(building));
}

// §8.1 / §1.4: dropped when the building leaves the deque or at Shutdown
// (its last reference may be this one: the original releases it there too).
void BuildingRelease(void* u, uint64_t building) {
    LOTDEV_TEST(building_release, building);
    if (building != 0) FC4_RELEASE_AT_DESTRUCTION(FromToken(building));
}

// §8.4 step 1: building slot 17 SetLit.
void BuildingSetLit(void* u, uint64_t building, bool lit) {
    LOTDEV_TEST(building_set_lit, building, lit);
    if (!Writes(u) || !Gate(u, fc4::Sw::LotdevNight, "lotdev building SetLit")) return;
    cISC4BuildingOccupant* b = Building(building);
    if (b == nullptr) return;
    b->SetLit(lit);
    ++Ctx(u)->lightsSet;
}

uint32_t OccupantType(void* u, uint64_t occ) {
    LOTDEV_TEST(occupant_type, occ);
    cISC4Occupant* o = Occ(occ);
    return o ? static_cast<uint32_t>(o->GetType()) : 0;  // slot 7
}

void OccupantPosition(void* u, uint64_t occ, float* out3) {
    LOTDEV_TEST(occupant_position, occ, out3);
    if (out3 == nullptr) return;
    cS3DVector3 v(0.0f, 0.0f, 0.0f);
    if (cISC4Occupant* o = Occ(occ)) o->GetPosition(&v);  // slot 8
    out3[0] = v.fX;
    out3[1] = v.fY;
    out3[2] = v.fZ;
}

void OccupantBoundingBox(void* u, uint64_t occ, float* out6) {
    LOTDEV_TEST(occupant_bounding_box, occ, out6);
    if (out6 == nullptr) return;
    cS3DVector3 a(0.0f, 0.0f, 0.0f), b(0.0f, 0.0f, 0.0f);
    if (cISC4Occupant* o = Occ(occ)) o->GetBoundingBox(&a, &b);  // slot 10
    out6[0] = a.fX;
    out6[1] = a.fY;
    out6[2] = a.fZ;
    out6[3] = b.fX;
    out6[4] = b.fY;
    out6[5] = b.fZ;
}

void OccupantCityCells(void* u, uint64_t occ, fc4_cell_rect* out) {
    LOTDEV_TEST(occupant_city_cells, occ, out);
    if (out == nullptr) return;
    SC4Rect<long> r;
    if (cISC4Occupant* o = Occ(occ)) o->GetBoundingCityCells(r);  // slot 11
    *out = fc4_cell_rect{static_cast<int32_t>(r.topLeftX), static_cast<int32_t>(r.topLeftY),
                         static_cast<int32_t>(r.bottomRightX), static_cast<int32_t>(r.bottomRightY)};
}

bool OccupantInGroup(void* u, uint64_t occ, uint32_t group) {
    LOTDEV_TEST(occupant_in_group, occ, group);
    cISC4Occupant* o = Occ(occ);
    return o != nullptr && o->IsOccupantGroup(group);  // slot 22
}

// Occupant slot 15 PostOccupantMessage(msg, 0): the night-light message
// (§8.4) under lotdev_night, the base-texture change (§6.6) under
// lotdev_textures.
void OccupantPostMessage(void* u, uint64_t occ, uint32_t message) {
    LOTDEV_TEST(occupant_post_message, occ, message);
    const fc4::Sw sw = message == kMsgNightLight ? fc4::Sw::LotdevNight : fc4::Sw::LotdevTextures;
    if (!Writes(u) || !Gate(u, sw, "lotdev PostOccupantMessage")) return;
    if (cISC4Occupant* o = Occ(occ)) o->PostOccupantMessage(message, 0);
}

void OccupantSetRemovalFlags(void* u, uint64_t occ, uint32_t flags) {
    LOTDEV_TEST(occupant_set_removal_flags, occ, flags);
    if (!Writes(u)) return;
    if (cISC4Occupant* o = Occ(occ)) o->SetRemovalFlags(flags);  // slot 12
}

void OccupantSetLotTag(void* u, uint64_t occ, uint32_t tag) {
    LOTDEV_TEST(occupant_set_lot_tag, occ, tag);
    if (!Writes(u)) return;
    if (cISC4Occupant* o = Occ(occ)) o->SetLotTag(tag);  // slot 28
}

bool OccupantProperty(void* u, uint64_t occ, uint32_t id, uint32_t* out) {
    LOTDEV_TEST(occupant_property, occ, id, out);
    cISC4Occupant* o = Occ(occ);
    if (o == nullptr) {
        if (out != nullptr) *out = 0;
        return false;
    }
    return U32Property(o->AsPropertyHolder(), id, out);  // slot 6
}

void OccupantSetProperty(void* u, uint64_t occ, uint32_t id, uint32_t value) {
    LOTDEV_TEST(occupant_set_property, occ, id, value);
    if (!Writes(u)) return;
    if (cISC4Occupant* o = Occ(occ)) SetU32Property(o->AsPropertyHolder(), id, value);
}

void OccupantInit(void* u, uint64_t occ) {
    LOTDEV_TEST(occupant_init, occ);
    if (!Writes(u)) return;
    if (cISC4Occupant* o = Occ(occ)) o->Init();  // slot 3
}

void OccupantSetFlag(void* u, uint64_t occ, uint32_t flag) {
    LOTDEV_TEST(occupant_set_flag, occ, flag);
    if (!Writes(u)) return;
    if (cISC4Occupant* o = Occ(occ)) o->SetFlag(flag);  // slot 29
}

bool OccupantIsFlagSet(void* u, uint64_t occ, uint32_t flag) {
    LOTDEV_TEST(occupant_is_flag_set, occ, flag);
    cISC4Occupant* o = Occ(occ);
    return o != nullptr && o->IsFlagSet(flag);  // slot 32
}

void OccupantShutdown(void* u, uint64_t occ) {
    LOTDEV_TEST(occupant_shutdown, occ);
    if (!Writes(u)) return;
    if (cISC4Occupant* o = Occ(occ)) o->Shutdown();  // slot 4
}

// The orientation of a prop (cISC4PropOccupant slot 10) or a building
// (slot 15); other occupants have none.
void OccupantSetOrientation(void* u, uint64_t occ, uint32_t orientation) {
    LOTDEV_TEST(occupant_set_orientation, occ, orientation);
    if (!Writes(u) || occ == 0) return;
    if (cISC4PropOccupant* p = As<cISC4PropOccupant>(occ, kIID_cISC4PropOccupant)) {
        p->SetOrientation(static_cast<int32_t>(orientation));
    } else if (cISC4BuildingOccupant* b = Building(occ)) {
        b->SetOrientation(static_cast<int32_t>(orientation));
    }
}

void OccupantSetPosition(void* u, uint64_t occ, const float* pos3) {
    LOTDEV_TEST(occupant_set_position, occ, pos3);
    if (!Writes(u) || pos3 == nullptr) return;
    const cS3DVector3 v(pos3[0], pos3[1], pos3[2]);
    if (cISC4Occupant* o = Occ(occ)) o->SetPosition(&v);  // slot 9
}

void OccupantInsert(void* u, uint64_t occ) {
    LOTDEV_TEST(occupant_insert, occ);
    cISC4OccupantManager* om = Occupants(u);
    if (!Writes(u) || om == nullptr) return;
    if (cISC4Occupant* o = Occ(occ)) om->InsertOccupant(o, 0);  // slot 20
    FC4_X87_CHECK("lotdev InsertOccupant");
}

// Occupant manager slot 21 RemoveOccupant(occ, a, flags).
void OccupantRemove(void* u, uint64_t occ, uint32_t a, uint32_t flags) {
    LOTDEV_TEST(occupant_remove, occ, a, flags);
    cISC4OccupantManager* om = Occupants(u);
    if (!Writes(u) || om == nullptr || !Gate(u, fc4::Sw::LotdevDemolition, "lotdev RemoveOccupant (kept)")) return;
    cISC4Occupant* o = Occ(occ);
    if (o == nullptr) return;
    fc4::LotDevExpire(Ctx(u), o);
    om->RemoveOccupant(o, a != 0, flags);
    FC4_X87_CHECK("lotdev RemoveOccupant");
    ++Ctx(u)->occupantsRemoved;
}

void OccupantChanged(void* u, uint64_t occ) {
    LOTDEV_TEST(occupant_changed, occ);
    cISC4OccupantManager* om = Occupants(u);
    if (!Writes(u) || om == nullptr) return;
    if (cISC4Occupant* o = Occ(occ)) om->MoveOccupant(o, false);  // slot 24
}

// Slot 36 with the blocking filter (flora 0x74758926 only, §1.3 step 6):
// [choice] the typed overload (slot 37, a GetType() == type filter) with the
// flora type, which lists the same occupants in the same order (as crime's
// box query, crime.md §11.3).
void OccupantsInBBox(void* u, const float* min3, const float* max3, fc4_token_callback cb, void* cbCtx) {
    LOTDEV_TEST(occupants_in_bbox, min3, max3, cb, cbCtx);
    cISC4OccupantManager* om = Occupants(u);
    if (min3 == nullptr || max3 == nullptr || cb == nullptr || om == nullptr) return;
    SC4List<cISC4Occupant*> list;
    om->GetOccupantsByBBox(list, min3, max3, kOccupantTypeFlora, 0xFFFFFFFFu);
    Each(TakeOccupants(u, list, "GetOccupantsByBBox list entry"), cb, cbCtx);
}

// Slot 41 GetOccupantsByStandardCityCells(list, min, max, type, -1).
void OccupantsInCells(void* u, const fc4_cell_rect* rect, uint32_t type, fc4_token_callback cb, void* cbCtx) {
    LOTDEV_TEST(occupants_in_cells, rect, type, cb, cbCtx);
    cISC4OccupantManager* om = Occupants(u);
    if (rect == nullptr || cb == nullptr || om == nullptr) return;
    SC4List<cISC4Occupant*> list;
    const SC4Point<int> a(rect->x1, rect->z1);
    const SC4Point<int> b(rect->x2, rect->z2);
    om->GetOccupantsByStandardCityCells(list, &a, &b, type, 0xFFFFFFFFu);
    Each(TakeOccupants(u, list, "GetOccupantsByStandardCityCells list entry"), cb, cbCtx);
}

// Slot 37 (the typed box query, flags -1), in list order.
void OccupantsInBox(void* u, const float* min3, const float* max3, uint32_t type, fc4_token_callback cb,
                    void* cbCtx) {
    LOTDEV_TEST(occupants_in_box, min3, max3, type, cb, cbCtx);
    cISC4OccupantManager* om = Occupants(u);
    if (min3 == nullptr || max3 == nullptr || cb == nullptr || om == nullptr) return;
    SC4List<cISC4Occupant*> list;
    om->GetOccupantsByBBox(list, min3, max3, type, 0xFFFFFFFFu);
    Each(TakeOccupants(u, list, "GetOccupantsByBBox list entry"), cb, cbCtx);
}

int32_t BuildingOrientation(void* u, uint64_t building) {
    LOTDEV_TEST(building_orientation, building);
    cISC4BuildingOccupant* b = Building(building);
    return b ? b->GetOrientation() : 0;  // slot 14
}

// ---- foundations (cISC4FoundationOccupant, §17.5) --------------------------------------------------------

bool FoundationKind(void* u, uint64_t occ, int32_t* out) {
    LOTDEV_TEST(foundation_kind, occ, out);
    if (out != nullptr) *out = 0;
    cISC4FoundationOccupant* f = As<cISC4FoundationOccupant>(occ, kIID_cISC4FoundationOccupant);
    if (f == nullptr) return false;
    if (out != nullptr) *out = f->GetUsage();  // slot 9
    return true;
}

uint32_t FoundationType(void* u, uint64_t occ) {
    LOTDEV_TEST(foundation_type, occ);
    cISC4FoundationOccupant* f = As<cISC4FoundationOccupant>(occ, kIID_cISC4FoundationOccupant);
    return f ? f->GetFoundationType() : 0;  // slot 4
}

// BDS slot 23 AddNewFoundationOccupant(type, kind, pos, dx, dz, orient, bottom).
bool AddNewFoundationOccupant(void* u, uint32_t type, int32_t kind, float* pos3, float dx, float dz, int32_t orient,
                              float bottom) {
    LOTDEV_TEST(add_new_foundation_occupant, type, kind, pos3, dx, dz, orient, bottom);
    if (!Writes(u) || pos3 == nullptr || !Gate(u, fc4::Sw::LotdevTerrain, "lotdev AddNewFoundationOccupant (none)")) {
        return false;
    }
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr) return false;
    const bool ok = b->AddNewFoundationOccupantAt(type, kind, pos3, dx, dz, orient, bottom);
    FC4_X87_CHECK("lotdev AddNewFoundationOccupant");
    if (ok) ++Ctx(u)->foundationsAdded;
    return ok;
}

// ---- prop manager ------------------------------------------------------------------------------------

bool PropWrites(void* u, const char* what) { return Writes(u) && Gate(u, fc4::Sw::LotdevProps, what); }

// Slot 15 CreateProp(type, &prop): the new prop's reference is kept for the
// call (the prop manager holds it once added); the occupant interface goes
// to the core.
uint64_t PropCreate(void* u, uint32_t type) {
    LOTDEV_TEST(prop_create, type);
    cISC4PropManager* pm = Props(u);
    if (pm == nullptr || !PropWrites(u, "lotdev CreateProp (none)")) return 0;
    cISC4PropOccupant* prop = nullptr;
    if (!pm->CreateProp(type, prop) || prop == nullptr) return 0;
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(prop), "CreateProp out-prop");
    fc4::LotDevHold(Ctx(u), prop, true);
    ++Ctx(u)->propsAdded;
    return TokenOf(prop->AsOccupant());  // slot 3
}

// Slot 24 GetLotProps(list, lot).
void LotProps(void* u, uint64_t lot, fc4_token_callback cb, void* cbCtx) {
    LOTDEV_TEST(lot_props, lot, cb, cbCtx);
    cISC4PropManager* pm = Props(u);
    if (lot == 0 || cb == nullptr || pm == nullptr) return;
    SC4List<cISC4Occupant*> list;
    pm->GetLotProps(list, SdkLotOf(lot));
    Each(TakeOccupants(u, list, "GetLotProps list entry"), cb, cbCtx);
}

// Slot 25 AddLotProp(lot, occ).
bool PropAddToLot(void* u, uint64_t lot, uint64_t occ) {
    LOTDEV_TEST(prop_add_to_lot, lot, occ);
    cISC4PropManager* pm = Props(u);
    if (pm == nullptr || lot == 0 || !PropWrites(u, "lotdev AddLotProp (refused)")) return false;
    cISC4Occupant* o = Occ(occ);
    return o != nullptr && pm->AddLotProp(SdkLotOf(lot), o);
}

// Slot 26 AddLotProp(type, pos, orientation) → the occupant (no reference).
uint64_t PropAdd(void* u, uint32_t type, const float* pos3, int32_t orientation) {
    LOTDEV_TEST(prop_add, type, pos3, orientation);
    cISC4PropManager* pm = Props(u);
    if (pm == nullptr || pos3 == nullptr || !PropWrites(u, "lotdev AddLotProp (none)")) return 0;
    const cS3DVector3 v(pos3[0], pos3[1], pos3[2]);
    cISC4Occupant* o = pm->AddLotProp(type, v, orientation);
    FC4_X87_CHECK("lotdev AddLotProp");
    ++Ctx(u)->propAdds;
    if (o == nullptr) ++Ctx(u)->propAddsNull;
    if (o != nullptr) ++Ctx(u)->propsAdded;
    return TokenOf(o);
}

// Slot 28 RemoveLotProps(lot).
void RemoveLotProps(void* u, uint64_t lot) {
    LOTDEV_TEST(remove_lot_props, lot);
    cISC4PropManager* pm = Props(u);
    if (pm == nullptr || lot == 0 || !PropWrites(u, "lotdev RemoveLotProps (kept)")) return;
    Ctx(u)->expireAll = true;
    pm->RemoveLotProps(SdkLotOf(lot));
}

// Slot 29 RemovePropA(occ).
bool PropRemove(void* u, uint64_t occ) {
    LOTDEV_TEST(prop_remove, occ);
    cISC4PropManager* pm = Props(u);
    if (pm == nullptr || !PropWrites(u, "lotdev RemovePropA (kept)")) return false;
    cISC4Occupant* o = Occ(occ);
    if (o == nullptr) return false;
    fc4::LotDevExpire(Ctx(u), o);
    const bool ok = pm->RemovePropA(o);
    if (ok) ++Ctx(u)->occupantsRemoved;
    return ok;
}

// Slot 38 RemoveBuildingProps(building occupant).
void RemoveBuildingProps(void* u, uint64_t building) {
    LOTDEV_TEST(remove_building_props, building);
    cISC4PropManager* pm = Props(u);
    if (pm == nullptr || !PropWrites(u, "lotdev RemoveBuildingProps (kept)")) return;
    cISC4Occupant* o = Occ(building);
    if (o == nullptr) return;
    Ctx(u)->expireAll = true;
    pm->RemoveBuildingProps(o);
}

// §7.2 ExplodeLot step 2 (LD14): cSC4PropOccupantBurntBuilding, class
// 0xC9EFB016, made with CreateInstance(0xC9EFB016, cISC4PropOccupant
// 0xE9793A65), which answers the object's base (the class is registered in
// the COM directory); SetBoundsFromOccupant(building occupant) by address
// (0x005EEB70, __thiscall on the base, pop 4: docs/abi/direct-calls.md);
// then on its occupant (AsOccupant, the sub-object at +8): SetFlag(0x40)
// (slot 29) and Init (slot 3). Its bounding box (occupant slot 10) goes to
// the core. The object's reference is kept for the call (released after
// AddLotProp, as the original's). [choice] The original's direct `new` when
// the class factory fails is not reproduced: no burnt prop (logged once).
// The building's position (occupant slot 8) is kept for the decal (§7.2
// step 4), read here, before ClearLotBuildings removes the building.
const uint32_t kCLSID_cSC4PropOccupantBurntBuilding = 0xC9EFB016;
const uint32_t kCLSID_cSC4PropOccupantTerrainDecal = 0x2977AA48;
const uint32_t kIID_cISC4PropOccupantTerrainDecal = 0xE982078A;
const uint32_t kOccupantFlagProp40 = 0x40;
const uint32_t kRubbleDecalType = 0x7AB50E44;
const uint32_t kRubbleDecalGroup = 0x1ABE787D;
const float kRubbleDecalOpacity = 0.7f;
// __thiscall through __fastcall: ECX = this, EDX unused, stack (source), callee pops 4.
using SetBoundsFromOccupantFn = void(__fastcall*)(void* self, void* edx, cISC4Occupant* source);

bool MakeBurntBuilding(void* u, uint64_t building, fc4_lotdev_burnt* out) {
    LOTDEV_TEST(make_burnt_building, building, out);
    if (out == nullptr || building == 0 || !PropWrites(u, "lotdev burnt building (none)")) return false;
    cISC4Occupant* source = Occ(building);
    const uintptr_t setBounds = fc4::compat::DirectCall(fc4::compat::Direct::BurntBuildingSetBounds);
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (source == nullptr || setBounds == 0 || com == nullptr ||
        !com->GetClassObject(kCLSID_cSC4PropOccupantBurntBuilding, kIID_cISC4PropOccupant, &p) || p == nullptr) {
        NoteOnce(g_noteBurnt, "lotdev: WARNING no burnt-building prop (class 0xC9EFB016 not created through GZCOM, "
                              "or SetBoundsFromOccupant not available): the explosion leaves no burnt shell and no "
                              "rubble (§7.2, LD14)");
        return false;
    }
    auto* prop = static_cast<cISC4PropOccupant*>(p);
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(prop), "GetClassObject");
    fc4::LotDevHold(Ctx(u), prop, true);
    reinterpret_cast<SetBoundsFromOccupantFn>(setBounds)(prop, nullptr, source);  // 0x005EEB70, on the base
    FC4_X87_CHECK("lotdev burnt SetBoundsFromOccupant");
    cISC4Occupant* o = prop->AsOccupant();  // slot 3, borrowed (+8)
    if (o == nullptr) return false;
    o->SetFlag(kOccupantFlagProp40);  // slot 29
    o->Init();                        // slot 3
    cS3DVector3 mn, mx;
    o->GetBoundingBox(&mn, &mx);  // slot 10
    cS3DVector3 at;
    source->GetPosition(&at);  // slot 8: the building's, for the decal
    FC4_X87_CHECK("lotdev burnt building");
    fc4::LotDevContext* ctx = Ctx(u);
    ctx->burntPos[0] = at.fX;
    ctx->burntPos[1] = at.fY;
    ctx->burntPos[2] = at.fZ;
    ctx->burntOccupant = TokenOf(o);
    out->occupant = TokenOf(o);
    out->min[0] = mn.fX;
    out->min[1] = mn.fY;
    out->min[2] = mn.fZ;
    out->max[0] = mx.fX;
    out->max[1] = mx.fY;
    out->max[2] = mx.fZ;
    ++ctx->burntMade;
    return true;
}

// §7.2 ExplodeLot step 4 (LD14): cSC4PropOccupantTerrainDecal, class
// 0x2977AA48, CreateInstance(0x2977AA48, 0xE982078A); on its occupant
// (QueryInterface cISC4Occupant, the sub-object at +0xC): Init; then decal
// slot 3 SetDecalTexture(&{0x7AB50E44, 0x1ABE787D, instance}, size) and slot 4
// SetOpacity(0.7); occupant SetPosition(the building's position) (slot 9),
// SetFlag(0x40) (slot 29); prop manager slot 25 AddLotProp(lot, occupant).
// The decal's reference is released after (kept for the call). [choice] No
// direct `new` when the factory fails: no decal (logged once).
void AddRubbleDecal(void* u, uint64_t lot, const fc4_lotdev_burnt* burnt, uint32_t instance, float size) {
    LOTDEV_TEST(add_rubble_decal, lot, burnt, instance, size);
    cISC4PropManager* pm = Props(u);
    fc4::LotDevContext* ctx = Ctx(u);
    if (pm == nullptr || lot == 0 || burnt == nullptr || !PropWrites(u, "lotdev rubble decal (none)")) return;
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (com == nullptr ||
        !com->GetClassObject(kCLSID_cSC4PropOccupantTerrainDecal, kIID_cISC4PropOccupantTerrainDecal, &p) ||
        p == nullptr) {
        NoteOnce(g_noteRubble, "lotdev: WARNING could not create a rubble decal through GZCOM (class 0x2977AA48): "
                               "none added (§7.2, LD14)");
        return;
    }
    auto* decal = static_cast<cISC4PropOccupantTerrainDecal*>(p);
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(decal), "GetClassObject");
    fc4::LotDevHold(ctx, decal, true);
    cISC4Occupant* o = As<cISC4Occupant>(TokenOf(decal), kIID_cISC4Occupant);  // +0xC, borrowed
    if (o == nullptr) return;
    o->Init();  // slot 3
    const cGZPersistResourceKey key(kRubbleDecalType, kRubbleDecalGroup, instance);
    decal->SetDecalTexture(key, size);       // slot 3
    decal->SetOpacity(kRubbleDecalOpacity);  // slot 4
    // The building's position from MakeBurntBuilding (the same ExplodeLot);
    // the burnt prop's own when it is not that one.
    cS3DVector3 at;
    if (ctx->burntOccupant != 0 && ctx->burntOccupant == burnt->occupant) {
        at = cS3DVector3(ctx->burntPos[0], ctx->burntPos[1], ctx->burntPos[2]);
    } else if (cISC4Occupant* b = Occ(burnt->occupant)) {
        b->GetPosition(&at);
    }
    o->SetPosition(&at);               // slot 9
    o->SetFlag(kOccupantFlagProp40);   // slot 29
    pm->AddLotProp(SdkLotOf(lot), o);  // slot 25
    FC4_X87_CHECK("lotdev rubble decal");
    ++ctx->decalsAdded;
}

// ---- BDS (building-development.md §7.1) ----------------------------------------------------------------

// Slot 39 GetBuildingProfile (inserts a zero profile for an unknown type).
void BuildingProfile(void* u, uint32_t building, fc4_lotdev_profile* out) {
    LOTDEV_TEST(building_profile, building, out);
    if (out == nullptr) return;
    *out = fc4_lotdev_profile{};
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr) return;
    const FC4BuildingProfile& p = b->GetBuildingProfile(building);
    out->purpose = p.purpose;
    out->wealth = p.wealth;
    out->capacity = p.capacity;
}

uint32_t DeveloperType(void* u, uint32_t purpose, uint32_t wealth) {
    LOTDEV_TEST(developer_type, purpose, wealth);
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b ? b->GetDeveloperType(purpose, wealth) : 0;  // slot 30
}

uint64_t DeveloperOfType(void* u, uint32_t devType) {
    LOTDEV_TEST(developer_of_type, devType);
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b ? TokenOf(b->GetDeveloper(devType)) : 0;  // slot 32, no reference
}

uint32_t WealthOfType(void* u, uint32_t devType) {
    LOTDEV_TEST(wealth_of_type, devType);
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b ? b->GetWealth(devType) : 0;  // slot 31
}

uint64_t DeveloperFor(void* u, uint32_t purpose, uint32_t wealth) {
    LOTDEV_TEST(developer_for, purpose, wealth);
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b ? TokenOf(b->GetDeveloperOf(purpose, wealth)) : 0;  // slot 33, no reference
}

bool IsBuildingType(void* u, uint32_t id) {
    LOTDEV_TEST(is_building_type, id);
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b != nullptr && b->IsBuildingTypeValid(id);  // slot 15
}

// Slot 19 GetBuildingTypeSetByFamily(set, family) into a 193-bucket hash_set (B).
void BuildingTypesByFamily(void* u, uint32_t family, fc4_u32s_sink sink, void* sinkCtx) {
    LOTDEV_TEST(building_types_by_family, family, sink, sinkCtx);
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (sink == nullptr || b == nullptr || !ListsAvailable(u)) return;
    fc4::GameHashSetU32<fc4::TractNodeAllocator> set(kSetHint);
    if (set.bucket_count() == 0) return;
    b->GetBuildingTypeSetByFamily(reinterpret_cast<fc4::GameHashSetU32<>&>(set), family);
    if (set.bucket_count() != 193) {
        NoteOnce(g_noteFamilySet, "lotdev: a building or flora family set grew past 193 buckets; its insertion order "
                                  "is not rebuilt exactly (question)");
    }
    std::vector<uint32_t> values;
    HashSetValues(set, values);
    if (!values.empty()) sink(sinkCtx, values.data(), values.size());
}

// Slot 7 AddNewBuildingOccupantAt(type, pos, orientation): borrowed (the
// occupant manager holds it).
uint64_t AddNewBuildingOccupant(void* u, uint32_t building, const float* pos3, int32_t orientation) {
    LOTDEV_TEST(add_new_building_occupant, building, pos3, orientation);
    if (!Writes(u) || pos3 == nullptr || !Gate(u, fc4::Sw::LotdevBuildings, "lotdev AddNewBuildingOccupant (none)")) {
        return 0;
    }
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr) return 0;
    float pos[3] = {pos3[0], pos3[1], pos3[2]};
    cISC4BuildingOccupant* occ = b->AddNewBuildingOccupantAt(building, pos, orientation);
    FC4_X87_CHECK("lotdev AddNewBuildingOccupant");
    if (occ == nullptr) {
        ++Ctx(u)->buildingsFailed;
        return 0;
    }
    ++Ctx(u)->buildingsAdded;
    return TokenOf(occ);
}

bool ClaimConstructionResources(void* u, uint16_t amount) {
    LOTDEV_TEST(claim_construction_resources, amount);
    if (!Writes(u) || !Gate(u, fc4::Sw::LotdevBuildings, "lotdev ClaimConstructionResources (refused)")) return false;
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b != nullptr && b->ClaimConstructionResources(amount);  // slot 50
}

// Slots 54, 55, 58.
bool Cheat(void* u, uint32_t which) {
    LOTDEV_TEST(cheat, which);
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr) return false;
    switch (which) {
        case 0: return b->CheatIgnorePlumbing();
        case 1: return b->CheatIgnorePower();
        case 2: return b->CheatCoffeeBar();
        default: return false;
    }
}

// ---- growth developers ----------------------------------------------------------------------------------

uint32_t DevPurpose(void* u, uint64_t dev) {
    LOTDEV_TEST(dev_purpose, dev);
    return dev ? DevOf(dev)->GetPurpose() : 0;  // slot 7
}

int16_t DevDesirability(void* u, uint64_t dev, int32_t x, int32_t z) {
    LOTDEV_TEST(dev_desirability, dev, x, z);
    cISC4SimGrid<int16_t>* g = dev ? DevOf(dev)->GetDesirabilityMap() : nullptr;  // slot 13
    return g ? g->GetCellValue(x, z) : int16_t{0};
}

uint8_t DevWaterSupplyCap(void* u, uint64_t dev) {
    LOTDEV_TEST(dev_water_supply_cap, dev);
    return dev ? DevOf(dev)->GetWaterSupplyCap() : uint8_t{0};  // slot 20
}

// ---- power, plumbing, traffic, demand ----------------------------------------------------------------------

bool IsPowered(void* u, int32_t x, int32_t z) {
    LOTDEV_TEST(is_powered, x, z);
    fc4::LotDevContext* c = Ctx(u);
    cISC4PowerSimulator* p = c->power ? c->power : (c->city ? c->city->GetPowerSimulator() : nullptr);
    return p != nullptr && fc4::AsGame(p)->IsPowered(x, z);  // slot 37
}

bool IsCellWatered(void* u, int32_t x, int32_t z) {
    LOTDEV_TEST(is_cell_watered, x, z);
    fc4::LotDevContext* c = Ctx(u);
    cISC4PlumbingSimulator* p = c->plumbing ? c->plumbing : (c->city ? c->city->GetPlumbingSimulator() : nullptr);
    return p != nullptr && fc4::AsGame(p)->IsCellWatered(x, z);  // slot 11
}

uint8_t TripLength(void* u, int32_t x, int32_t z) {
    LOTDEV_TEST(trip_length, x, z);
    fc4::LotDevContext* c = Ctx(u);
    cISC4TrafficSimulator* t = c->traffic ? c->traffic : (c->city ? c->city->GetTrafficSimulator() : nullptr);
    cISC4SimGrid<uint8_t>* g = t ? t->GetTripLengthMap() : nullptr;  // slot 13
    return g ? g->GetCellValue(x, z) : uint8_t{0};
}

// §3.2 (0x69247DC7): demand simulator slot 6 GetDemand(0x2000, 0x20000), demand slot 9.
bool JobsDemand(void* u, float* out) {
    LOTDEV_TEST(jobs_demand, out);
    if (out != nullptr) *out = 0.0f;
    cISC4City* c = City(u);
    cISC4DemandSimulator* ds = c ? c->GetDemandSimulator() : nullptr;
    cISC4Demand* d = ds ? fc4::AsGame(ds)->GetDemand(0x2000, 0x20000) : nullptr;
    if (d == nullptr) return false;
    const float v = fc4::AsGame(d)->QueryDemandValue();
    if (out != nullptr) *out = v;
    return true;
}

// ---- flora ---------------------------------------------------------------------------------------------

bool FloraTypeValid(void* u, uint32_t id) {
    LOTDEV_TEST(flora_type_valid, id);
    cISC4FloraSimulator* f = Flora(u);
    const bool ok = f != nullptr && f->IsFloraTypeValid(id);  // slot 11
    ++(ok ? Ctx(u)->floraValid : Ctx(u)->floraInvalid);
    return ok;
}

// Slot 14 GetFloraTypesByFamily(set, id) into a 193-bucket hash_set (B);
// false when it returns 0.
bool FloraTypesByFamily(void* u, uint32_t id, fc4_u32s_sink sink, void* sinkCtx) {
    LOTDEV_TEST(flora_types_by_family, id, sink, sinkCtx);
    cISC4FloraSimulator* f = Flora(u);
    if (f == nullptr || !ListsAvailable(u)) return false;
    fc4::GameHashSetU32<fc4::TractNodeAllocator> set(kSetHint);
    if (set.bucket_count() == 0) return false;
    const uint32_t n = f->GetFloraTypesByFamily(*reinterpret_cast<intptr_t*>(&set), id);
    ++Ctx(u)->floraFamilies;
    if (n == 0) {
        ++Ctx(u)->floraFamiliesEmpty;
        return false;
    }
    std::vector<uint32_t> values;
    HashSetValues(set, values);
    if (sink != nullptr && !values.empty()) sink(sinkCtx, values.data(), values.size());
    return true;
}

// Slot 17 AddNewFloraOccupant(type, pos, false): borrowed.
uint64_t FloraAdd(void* u, uint32_t type, const float* pos3) {
    LOTDEV_TEST(flora_add, type, pos3);
    cISC4FloraSimulator* f = Flora(u);
    if (f == nullptr || pos3 == nullptr || !PropWrites(u, "lotdev AddNewFloraOccupant (none)")) return 0;
    float pos[3] = {pos3[0], pos3[1], pos3[2]};
    cISC4Occupant* o = f->AddNewFloraOccupant(type, pos, false);
    FC4_X87_CHECK("lotdev AddNewFloraOccupant");
    ++Ctx(u)->floraAdds;
    if (o == nullptr) ++Ctx(u)->floraAddsNull;
    if (o != nullptr) ++Ctx(u)->floraAdded;
    return TokenOf(o);
}

// Slot 20 DemolishFloraInCellArea(rect, 0).
void FloraDemolishInCellArea(void* u, const fc4_cell_rect* rect) {
    LOTDEV_TEST(flora_demolish_in_cell_area, rect);
    cISC4FloraSimulator* f = Flora(u);
    if (f == nullptr || rect == nullptr || !PropWrites(u, "lotdev DemolishFloraInCellArea (kept)")) return;
    Ctx(u)->expireAll = true;
    f->DemolishFloraInCellArea(RectOf(*rect), 0);
}

// ---- demolition ----------------------------------------------------------------------------------------

cISC4Demolition* Demolition(void* u) {
    fc4::LotDevContext* c = Ctx(u);
    return c->demolition ? c->demolition : (c->city ? c->city->GetDemolitionUtility() : nullptr);
}

// Slot 8 DemolishOccupant(true, occ, 0, 1, null, false, null).
bool DemolishOccupant(void* u, uint64_t occ) {
    LOTDEV_TEST(demolish_occupant, occ);
    if (!Writes(u) || !Gate(u, fc4::Sw::LotdevDemolition, "lotdev DemolishOccupant (refused)")) return false;
    cISC4Demolition* d = Demolition(u);
    cISC4Occupant* o = Occ(occ);
    if (d == nullptr || o == nullptr) return false;
    fc4::LotDevExpire(Ctx(u), o);
    const bool ok = d->DemolishOccupant(true, o, 0, 1, nullptr, false, nullptr);
    FC4_X87_CHECK("lotdev DemolishOccupant");
    if (ok) ++Ctx(u)->demolished;
    return ok;
}

// Slot 7 DemolishLots(true, [lot], 0, 0x800, false, 0, 0, 0). The list is
// only walked by the game: it borrows the lot.
bool DemolishLot(void* u, uint64_t lot) {
    LOTDEV_TEST(demolish_lot, lot);
    if (!Writes(u) || lot == 0 || !Gate(u, fc4::Sw::LotdevDemolition, "lotdev DemolishLots (refused)")) return false;
    cISC4Demolition* d = Demolition(u);
    if (d == nullptr) return false;
    Ctx(u)->expireAll = true;
    bool ok = false;
    auto run = [&](auto& list) {
        cISC4Lot* entry = SdkLotOf(lot);
        list.push_back(entry);
        ok = d->DemolishLots(true, reinterpret_cast<const SC4List<cISC4Lot*>&>(list), 0, kDemolishFlags, false,
                             nullptr, nullptr, 0);
        for (cISC4Lot*& l : list) l = nullptr;  // borrowed: never released by the list
    };
    LotList list;
    if (list.valid()) {
        run(list);
    } else {
        SC4List<cISC4Lot*> sdk;
        run(sdk);
    }
    FC4_X87_CHECK("lotdev DemolishLots");
    if (ok) ++Ctx(u)->demolished;
    return ok;
}

// Slot 9 ModifyTerrainHeight(true, perVertex, x0, z0, x1, z1, h, exempt, 5, null).
int32_t ModifyTerrainHeight(void* u, bool perVertex, int32_t x0, int32_t z0, int32_t x1, int32_t z1, float h,
                            uint64_t exempt) {
    LOTDEV_TEST(modify_terrain_height, perVertex, x0, z0, x1, z1, h, exempt);
    if (!Writes(u) || !Gate(u, fc4::Sw::LotdevTerrain, "lotdev ModifyTerrainHeight (0)")) return 0;
    cISC4Demolition* d = Demolition(u);
    if (d == nullptr) return 0;
    const int32_t n = d->ModifyTerrainHeight(true, perVertex, x0, z0, x1, z1, h, SdkLotOf(exempt), 5, nullptr);
    FC4_X87_CHECK("lotdev ModifyTerrainHeight");
    ++Ctx(u)->terrainWrites;
    return n;
}

// ---- business units (cISC4BusinessUnit) -------------------------------------------------------------------

// Slot 5 GetMembers (borrowed list): one reference per member kept for the call.
void UnitMembers(void* u, uint64_t unit, fc4_token_callback cb, void* cbCtx) {
    LOTDEV_TEST(unit_members, unit, cb, cbCtx);
    if (unit == 0 || cb == nullptr) return;
    SC4List<cISC4Lot*>* members = UnitOf(unit)->GetMembers();
    if (members == nullptr) return;
    std::vector<uint64_t> tokens;
    for (cISC4Lot* lot : *members) {
        if (lot == nullptr) continue;
        FC4_ADDREF(lot);
        fc4::LotDevHold(Ctx(u), reinterpret_cast<cIGZUnknown*>(lot));
        tokens.push_back(TokenOf(lot));
    }
    Each(tokens, cb, cbCtx);
}

// §5.7 step 4 (LD13): a new cSC4BusinessUnit, CreateInstance(0x4A232DA8,
// cISC4BusinessUnit 0x6A233069) (the class is registered; the original's
// direct construction holds no reference of its own: the member lots do).
// Our reference is kept for the call and released after the core has added
// the candidate and the lot (the created hold), so the members hold the unit.
uint64_t UnitCreate(void* u) {
    LOTDEV_TEST0(unit_create);
    if (!Writes(u) || !Gate(u, fc4::Sw::LotdevUnits, "lotdev business unit (none)")) return 0;
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (com == nullptr || !com->GetClassObject(kCLSID_cSC4BusinessUnit, kIID_cISC4BusinessUnit, &p) || p == nullptr) {
        NoteOnce(g_noteUnit, "lotdev: WARNING could not create a business unit through GZCOM (class 0x4A232DA8); no "
                             "unit is formed");
        return 0;
    }
    auto* unit = static_cast<cIGZUnknown*>(p);
    FC4_ACQUIRED(unit, "GetClassObject");
    fc4::LotDevHold(Ctx(u), unit, true);
    ++Ctx(u)->unitsMade;
    return TokenOf(p);
}

bool UnitAdd(void* u, uint64_t unit, uint64_t lot) {
    LOTDEV_TEST(unit_add, unit, lot);
    if (!Writes(u) || unit == 0 || lot == 0 || !Gate(u, fc4::Sw::LotdevUnits, "lotdev unit Add (refused)")) {
        return false;
    }
    return UnitOf(unit)->Add(SdkLotOf(lot));  // slot 3
}

bool UnitRemove(void* u, uint64_t unit, uint64_t lot) {
    LOTDEV_TEST(unit_remove, unit, lot);
    if (!Writes(u) || unit == 0 || lot == 0 || !Gate(u, fc4::Sw::LotdevUnits, "lotdev unit Remove (refused)")) {
        return false;
    }
    return UnitOf(unit)->Remove(SdkLotOf(lot));  // slot 4
}

// ---- lotdev_lot_log --------------------------------------------------------------------------------------

void NoteLogLot(void* u, uint64_t lot) {
    fc4::LotDevContext* c = Ctx(u);
    if (!c->lotLog) return;
    fc4::LotDevFlushLotLog(c);
    if (lot == 0) return;
    cFC4ISC4Lot* l = LotOf(lot);
    FC4_ADDREF(l);
    c->logLot = lot;
    c->logState = static_cast<uint32_t>(l->GetState());
    c->logCondition = l->GetCondition();
    U32Property(l->AsPropertyHolder(), kPropZot, &c->logZot);
}

}  // namespace

// ---- the class's helpers ------------------------------------------------------------------------------------

namespace fc4 {

// A configuration object (0x34 bytes, §17.1) as the core reads it. `ids`
// keeps the ID list (the object's own vector, not expanded).
fc4_lotdev_cfg_object LotDevObjectOf(cSC4LotConfigurationObject* o, std::vector<uint32_t>& ids) {
    fc4_lotdev_cfg_object c{};
    ids.clear();
    if (o == nullptr) return c;
    c.object_type = static_cast<uint8_t>(o->type);
    c.flags = o->flags;
    c.orientation = static_cast<uint8_t>(o->rotation);
    std::memcpy(&c.byte3, reinterpret_cast<const uint8_t*>(o) + 3, 1);  // the raw byte (+0x03), not a bool
    c.offset[0] = o->x;
    c.offset[1] = o->y;
    c.offset[2] = o->z;
    c.footprint_max[0] = o->rectMaxX;
    c.footprint_max[1] = o->rectMaxZ;
    c.footprint_min[0] = o->rectMinX;
    c.footprint_min[1] = o->rectMinZ;
    c.foundation = o->foundationIID;
    c.tag = o->objectID;
    for (uint32_t id : o->data) ids.push_back(id);
    c.n_ids = static_cast<uint32_t>(ids.size());
    c.ids = ids.empty() ? nullptr : ids.data();
    return c;
}

// The configuration object `index` of `cfg` (slot 23), or null when out of
// range ([choice] the original does not check, §15.5).
cSC4LotConfigurationObject* LotDevObjectAt(cISC4LotConfiguration* cfg, uint32_t index) {
    if (cfg == nullptr) return nullptr;
    SC4Vector<cSC4LotConfigurationObject>* v = cfg->GetLotConfigurationObjectArray();  // slot 23
    if (v == nullptr || index >= v->size()) return nullptr;
    return v->begin() + index;
}

// Slot 38 GetFirstBuilding(lot) (§7.1, LD15): the building occupants
// (0x278128A0) of the lot's bounding rectangle from occupant manager slot 41
// with maxCount 1; only the FIRST entry is queried for cISC4BuildingOccupant
// (null when it is not one); no reference for the caller (one is kept for
// the call, as the list's are).
cISC4BuildingOccupant* LotDevFirstBuilding(LotDevContext* ctx, cISC4Lot* lot) {
    void* u = ctx;
    if (ctx->test != nullptr && ctx->test->first_building != nullptr) {
        return reinterpret_cast<cISC4BuildingOccupant*>(
            FromToken(ctx->test->first_building(ctx->test->user, TokenOf(lot))));
    }
    cISC4OccupantManager* om = Occupants(u);
    if (lot == nullptr || om == nullptr) return nullptr;
    SC4Rect<int32_t> r;
    fc4::AsGame(lot)->GetBoundingRect(r);  // slot 14
    SC4List<cISC4Occupant*> list;
    const SC4Point<int> a(r.topLeftX, r.topLeftY);
    const SC4Point<int> b(r.bottomRightX, r.bottomRightY);
    const bool ok = om->GetOccupantsByStandardCityCells(list, &a, &b, kOccupantTypeBuilding, 1u);  // slot 41
    const std::vector<uint64_t> entries = TakeOccupants(u, list, "GetOccupantsByStandardCityCells list entry");
    return ok && !entries.empty() ? Building(entries.front()) : nullptr;
}

// Slot 41's occupant (§6.6, LD16 / LD24): the first base texture at the
// lot's location cell (FindBaseTexture; queried, SetUsage(2)), or, with
// `create`, a new one (CreateInstance(0xC97F987C, 0x097F6D98),
// AsOccupant()->Init(), SetUsage(2); not inserted: §6.6 inserts it after
// SetTextureSpecification). [choice] The original's direct construction when
// GZCOM fails is not reproduced (logged once). Borrowed (a reference is kept
// for the call).
cISC4Occupant* LotDevBaseTextureOccupant(LotDevContext* ctx, cISC4Lot* lot, bool create) {
    void* u = ctx;
    if (ctx->test != nullptr && ctx->test->base_texture_occupant != nullptr) {
        return reinterpret_cast<cISC4Occupant*>(
            FromToken(ctx->test->base_texture_occupant(ctx->test->user, TokenOf(lot), create)));
    }
    if (lot == nullptr) return nullptr;
    if (cISC4Occupant* o = FindBaseTexture(u, lot)) {
        // Found: a failed query answers false, and nothing is created.
        return FoundBaseTexture(u, o) != nullptr ? o : nullptr;
    }
    if (!create || ctx->readOnly || !Gate(u, fc4::Sw::LotdevTextures, "lotdev base-texture occupant (none)")) {
        return nullptr;
    }
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (com == nullptr ||
        !com->GetClassObject(kCLSID_cSC4LotBaseTextureOccupant, kIID_cISC4LotBaseTextureOccupant, &p) || p == nullptr) {
        NoteOnce(g_noteTexture, "lotdev: WARNING could not create a base-texture occupant through GZCOM (class "
                                "0xC97F987C); lots show no base texture");
        return nullptr;
    }
    auto* tex = static_cast<cISC4LotBaseTextureOccupant*>(p);
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(tex), "GetClassObject");
    LotDevHold(ctx, tex, true);
    cISC4Occupant* o = tex->AsOccupant();  // slot 3
    if (o != nullptr) o->Init();           // occupant slot 3
    tex->SetUsage(kLotTextureUsage);       // slot 5
    ++ctx->texturesMade;
    return o;
}

void LotDevHold(LotDevContext* ctx, cIGZUnknown* p, bool created) {
    if (p != nullptr) ctx->held.push_back(LotDevContext::Held{p, created});
}

namespace {
// The object's identity (its cIGZUnknown interface), or the pointer itself.
const void* Identity(cIGZUnknown* p) {
    void* q = nullptr;
    if (p == nullptr || !p->QueryInterface(kIID_cIGZUnknown, &q) || q == nullptr) return p;
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(q), "QueryInterface");
    FC4_RELEASE(static_cast<cIGZUnknown*>(q));
    return q;
}
}  // namespace

void LotDevExpire(LotDevContext* ctx, cIGZUnknown* p) {
    if (p != nullptr) ctx->expired.push_back(Identity(p));
}

void LotDevReleaseHeld(LotDevContext* ctx) {
    std::vector<LotDevContext::Held> held;
    std::vector<const void*> expired;
    held.swap(ctx->held);
    expired.swap(ctx->expired);
    const bool all = ctx->expireAll;
    ctx->expireAll = false;
    for (const LotDevContext::Held& h : held) {
        bool mayBeLast = h.created || all;
        if (!mayBeLast && !expired.empty()) {
            mayBeLast = std::find(expired.begin(), expired.end(), Identity(h.p)) != expired.end();
        }
        if (mayBeLast) {
            FC4_RELEASE_CREATED(h.p);
        } else {
            FC4_RELEASE(h.p);
        }
    }
    // The storage, for the next call.
    held.clear();
    if (ctx->held.empty()) held.swap(ctx->held);
}

void LotDevReleaseServices(LotDevContext* ctx) {
    std::vector<LotDevIterator*> iterators;
    iterators.swap(ctx->iterators);
    for (LotDevIterator* it : iterators) FreeIterator(ctx, it);
    if (ctx->logLot != 0) {
        FC4_RELEASE_CREATED(FromToken(ctx->logLot));
        ctx->logLot = 0;
    }
    if (ctx->readOnly) return;
    // §1.4.
    ReleaseSlot(ctx->msgServer);
    ReleaseSlot(ctx->lots);
    ReleaseSlot(ctx->occupants);
    ReleaseSlot(ctx->props);
    ReleaseSlot(ctx->bds);
    ReleaseSlot(ctx->power);
    ReleaseSlot(ctx->plumbing);
    ReleaseSlot(ctx->flora);
    ReleaseSlot(ctx->terrain);
    ReleaseSlot(ctx->traffic);
    ReleaseSlot(ctx->demolition);
    ReleaseSlot(ctx->city);
}

// "lotdev lot X,Z: state a->b condition c->d zot e->f" when one changed.
void LotDevFlushLotLog(LotDevContext* ctx) {
    if (ctx->logLot == 0) return;
    const uint64_t lot = ctx->logLot;
    ctx->logLot = 0;
    cFC4ISC4Lot* l = LotOf(lot);
    const uint32_t state = static_cast<uint32_t>(l->GetState());
    const uint32_t condition = l->GetCondition();
    uint32_t zot = 0;
    U32Property(l->AsPropertyHolder(), kPropZot, &zot);
    if (state != ctx->logState || condition != ctx->logCondition || zot != ctx->logZot) {
        int32_t x = 0, z = 0;
        l->GetLocation(x, z);
        fc4::Log("lotdev lot %d,%d: state %u->%u condition %u->%u zot %u->%u building=0x%08X", x, z, ctx->logState,
                 state, ctx->logCondition, condition, ctx->logZot, zot, l->GetBuildingType(false));
    }
    FC4_RELEASE_CREATED(l);  // a lot deleted since: ours was the last
}

fc4_lotdev_host MakeLotDevHost(LotDevContext* ctx) {
    fc4_lotdev_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.acquire_services = &AcquireServices;
    h.release_services = &ReleaseServicesCb;
    h.tuning = &Tuning;
    h.has_simulator = &HasSimulator;
    h.add_agent = &AddAgent;
    h.remove_agent = &RemoveAgent;
    h.acquire_message_server = &AcquireMessageServer;
    h.release_message_server = &ReleaseMessageServer;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.register_animation_tick = &RegisterAnimationTick;
    h.unregister_animation_tick = &UnregisterAnimationTick;
    h.post_message = &PostMessage;
    h.city_cell_counts = &CityCellCounts;
    h.city_cell_widths = &CityCellWidths;
    h.position_to_cell = &PositionToCell;
    h.cell_corner_to_position = &CellCornerToPosition;
    h.cell_center_to_position = &CellCenterToPosition;
    h.sim_date = &SimDate;
    h.terrain_cell_counts = &TerrainCellCounts;
    h.terrain_cell_width = &TerrainCellWidth;
    h.terrain_altitude = &TerrainAltitude;
    h.terrain_water_level = &TerrainWaterLevel;
    h.terrain_altitude_at = &TerrainAltitudeAt;
    h.terrain_water_level_at = &TerrainWaterLevelAt;
    h.terrain_is_water_vertex = &TerrainIsWaterVertex;
    h.terrain_cell_has_water = &TerrainCellHasWater;
    h.zone_drag_color = &ZoneDragColor;
    h.zone_display_alpha = &ZoneDisplayAlpha;
    h.view_rotation = &ViewRotation;
    h.visual_time_mode = &VisualTimeMode;
    h.resource_exists = &ResourceExists;
    h.lot_count = &LotCount;
    h.safe_lot_iterator = &SafeLotIterator;
    h.safe_lot_iterator_rewind = &SafeLotIteratorRewind;
    h.safe_lot_iterator_next = &SafeLotIteratorNext;
    h.safe_lot_iterator_free = &SafeLotIteratorFree;
    h.get_lot = &GetLot;
    h.lots_in_cell_rect = &LotsInCellRect;
    h.lots_surrounding_lot = &LotsSurroundingLot;
    h.lots_surrounding_rect = &LotsSurroundingRect;
    h.delete_lot = &DeleteLot;
    h.lot_is_initialized = &LotIsInitialized;
    h.lot_location = &LotLocation;
    h.lot_size = &LotSize;
    h.lot_bounding_rect = &LotBoundingRect;
    h.lot_center = &LotCenter;
    h.lot_zone_type = &LotZoneType;
    h.lot_altitude = &LotAltitude;
    h.lot_set_altitude = &LotSetAltitude;
    h.lot_facing = &LotFacing;
    h.lot_flags = &LotFlags;
    h.lot_set_flags = &LotSetFlags;
    h.lot_set_vertices_locked = &LotSetVerticesLocked;
    h.lot_update_slope = &LotUpdateSlope;
    h.lot_state = &LotState;
    h.lot_set_state = &LotSetState;
    h.lot_condition = &LotCondition;
    h.lot_set_condition = &LotSetCondition;
    h.lot_is_livable = &LotIsLivable;
    h.lot_days_since_state_change = &LotDaysSinceStateChange;
    h.lot_building_type = &LotBuildingType;
    h.lot_set_building_type = &LotSetBuildingType;
    h.lot_building = &LotBuilding;
    h.lot_configuration = &LotConfiguration;
    h.lot_set_configuration = &LotSetConfiguration;
    h.lot_occupant_wealth = &LotOccupantWealth;
    h.lot_occupancy = &LotOccupancy;
    h.lot_needs_power = &LotNeedsPower;
    h.lot_has_power = &LotHasPower;
    h.lot_set_has_power = &LotSetHasPower;
    h.lot_has_job_access = &LotHasJobAccess;
    h.lot_has_road_access = &LotHasRoadAccess;
    h.lot_has_organic_road_access = &LotHasOrganicRoadAccess;
    h.lot_update_road_access = &LotUpdateRoadAccess;
    h.lot_travel_jobs = &LotTravelJobs;
    h.lot_travel_designate = &LotTravelDesignate;
    h.lot_set_travel_designate = &LotSetTravelDesignate;
    h.lot_unit = &LotUnit;
    h.lot_reset_travel_population = &LotResetTravelPopulation;
    h.lot_add_travel_population = &LotAddTravelPopulation;
    h.lot_is_historical = &LotIsHistorical;
    h.lot_set_historical = &LotSetHistorical;
    h.lot_available_capacity = &LotAvailableCapacity;
    h.lot_set_population = &LotSetPopulation;
    h.lot_property = &LotProperty;
    h.lot_set_property = &LotSetProperty;
    h.cfg_id = &CfgId;
    h.cfg_size = &CfgSize;
    h.cfg_max_slope_before_foundation = &CfgMaxSlopeBeforeFoundation;
    h.cfg_growth_stage = &CfgGrowthStage;
    h.cfg_is_compatible_with_water = &CfgIsCompatibleWithWater;
    h.cfg_objects = &CfgObjects;
    h.cfg_primary_building_object = &CfgPrimaryBuildingObject;
    h.cfg_primary_foundation_object = &CfgPrimaryFoundationObject;
    h.cfg_has_objects_of_type = &CfgHasObjectsOfType;
    h.get_lot_configuration = &GetLotConfiguration;
    h.object_id_array = &ObjectIdArray;
    h.first_building = &FirstBuilding;
    h.update_lot_retaining_walls = &UpdateLotRetainingWalls;
    h.update_lot_base_texture_occupant = &UpdateLotBaseTextureOccupant;
    h.base_texture_occupant = &BaseTextureOccupant;
    h.base_texture_set = &BaseTextureSet;
    h.add_retaining_wall_occupant = &AddRetainingWallOccupant;
    h.update_surrounding_retaining_walls = &UpdateSurroundingRetainingWalls;
    h.add_network_connection_nodes = &AddNetworkConnectionNodes;
    h.remove_network_connection_nodes = &RemoveNetworkConnectionNodes;
    h.start_construction_occupant = &StartConstructionOccupant;
    h.occupant_as_building = &OccupantAsBuilding;
    h.building_add_ref = &BuildingAddRef;
    h.building_release = &BuildingRelease;
    h.building_set_lit = &BuildingSetLit;
    h.occupant_type = &OccupantType;
    h.occupant_position = &OccupantPosition;
    h.occupant_bounding_box = &OccupantBoundingBox;
    h.occupant_city_cells = &OccupantCityCells;
    h.occupant_in_group = &OccupantInGroup;
    h.occupant_post_message = &OccupantPostMessage;
    h.occupant_set_removal_flags = &OccupantSetRemovalFlags;
    h.occupant_set_lot_tag = &OccupantSetLotTag;
    h.occupant_property = &OccupantProperty;
    h.occupant_set_property = &OccupantSetProperty;
    h.occupant_init = &OccupantInit;
    h.occupant_set_flag = &OccupantSetFlag;
    h.occupant_is_flag_set = &OccupantIsFlagSet;
    h.occupant_shutdown = &OccupantShutdown;
    h.occupant_set_orientation = &OccupantSetOrientation;
    h.occupant_set_position = &OccupantSetPosition;
    h.occupant_insert = &OccupantInsert;
    h.occupant_remove = &OccupantRemove;
    h.occupant_changed = &OccupantChanged;
    h.occupants_in_bbox = &OccupantsInBBox;
    h.occupants_in_cells = &OccupantsInCells;
    h.occupants_in_box = &OccupantsInBox;
    h.building_orientation = &BuildingOrientation;
    h.foundation_kind = &FoundationKind;
    h.foundation_type = &FoundationType;
    h.add_new_foundation_occupant = &AddNewFoundationOccupant;
    h.prop_create = &PropCreate;
    h.lot_props = &LotProps;
    h.prop_add_to_lot = &PropAddToLot;
    h.prop_add = &PropAdd;
    h.remove_lot_props = &RemoveLotProps;
    h.prop_remove = &PropRemove;
    h.remove_building_props = &RemoveBuildingProps;
    h.make_burnt_building = &MakeBurntBuilding;
    h.add_rubble_decal = &AddRubbleDecal;
    h.building_profile = &BuildingProfile;
    h.developer_type = &DeveloperType;
    h.developer_of_type = &DeveloperOfType;
    h.wealth_of_type = &WealthOfType;
    h.developer_for = &DeveloperFor;
    h.is_building_type = &IsBuildingType;
    h.building_types_by_family = &BuildingTypesByFamily;
    h.add_new_building_occupant = &AddNewBuildingOccupant;
    h.claim_construction_resources = &ClaimConstructionResources;
    h.cheat = &Cheat;
    h.dev_purpose = &DevPurpose;
    h.dev_desirability = &DevDesirability;
    h.dev_water_supply_cap = &DevWaterSupplyCap;
    h.is_powered = &IsPowered;
    h.is_cell_watered = &IsCellWatered;
    h.trip_length = &TripLength;
    h.jobs_demand = &JobsDemand;
    h.flora_type_valid = &FloraTypeValid;
    h.flora_types_by_family = &FloraTypesByFamily;
    h.flora_add = &FloraAdd;
    h.flora_demolish_in_cell_area = &FloraDemolishInCellArea;
    h.demolish_occupant = &DemolishOccupant;
    h.demolish_lot = &DemolishLot;
    h.modify_terrain_height = &ModifyTerrainHeight;
    h.unit_members = &UnitMembers;
    h.unit_create = &UnitCreate;
    h.unit_add = &UnitAdd;
    h.unit_remove = &UnitRemove;
    h.lot_occupants = &LotOccupants;
    h.lot_configuration_object = &LotConfigurationObject;
    h.occupant_moved = &OccupantMoved;
    return h;
}

// Every callback that would change the game (or register anything) is
// NULL: the core answers its neutral value. has_simulator NULL = false.
fc4_lotdev_host MakeLotDevReadOnlyHost(LotDevContext* ctx) {
    ctx->readOnly = true;
    ctx->gated = false;
    ctx->lotLog = false;
    fc4_lotdev_host h = MakeLotDevHost(ctx);
    h.release_services = nullptr;
    h.has_simulator = nullptr;
    h.add_agent = nullptr;
    h.remove_agent = nullptr;
    h.acquire_message_server = nullptr;
    h.release_message_server = nullptr;
    h.subscribe = nullptr;
    h.unsubscribe = nullptr;
    h.register_animation_tick = nullptr;
    h.unregister_animation_tick = nullptr;
    h.post_message = nullptr;
    h.safe_lot_iterator = nullptr;
    h.safe_lot_iterator_rewind = nullptr;
    h.safe_lot_iterator_next = nullptr;
    h.safe_lot_iterator_free = nullptr;
    h.delete_lot = nullptr;
    h.lot_set_altitude = nullptr;
    h.lot_set_flags = nullptr;
    h.lot_set_vertices_locked = nullptr;
    h.lot_update_slope = nullptr;
    h.lot_set_state = nullptr;
    h.lot_set_condition = nullptr;
    h.lot_set_building_type = nullptr;
    h.lot_set_configuration = nullptr;
    h.lot_set_has_power = nullptr;
    h.lot_update_road_access = nullptr;
    h.lot_set_travel_designate = nullptr;
    h.lot_reset_travel_population = nullptr;
    h.lot_add_travel_population = nullptr;
    h.lot_set_historical = nullptr;
    h.lot_set_population = nullptr;
    h.lot_set_property = nullptr;
    h.update_lot_retaining_walls = nullptr;
    h.update_lot_base_texture_occupant = nullptr;
    h.base_texture_occupant = nullptr;
    h.base_texture_set = nullptr;
    h.add_retaining_wall_occupant = nullptr;
    h.update_surrounding_retaining_walls = nullptr;
    h.add_network_connection_nodes = nullptr;
    h.remove_network_connection_nodes = nullptr;
    h.start_construction_occupant = nullptr;
    h.building_add_ref = nullptr;
    h.building_release = nullptr;
    h.building_set_lit = nullptr;
    h.occupant_post_message = nullptr;
    h.occupant_set_removal_flags = nullptr;
    h.occupant_set_lot_tag = nullptr;
    h.occupant_set_property = nullptr;
    h.occupant_init = nullptr;
    h.occupant_set_flag = nullptr;
    h.occupant_shutdown = nullptr;
    h.occupant_set_orientation = nullptr;
    h.occupant_set_position = nullptr;
    h.occupant_insert = nullptr;
    h.occupant_remove = nullptr;
    h.occupant_changed = nullptr;
    h.add_new_foundation_occupant = nullptr;
    h.prop_create = nullptr;
    h.prop_add_to_lot = nullptr;
    h.prop_add = nullptr;
    h.remove_lot_props = nullptr;
    h.prop_remove = nullptr;
    h.remove_building_props = nullptr;
    h.make_burnt_building = nullptr;
    h.add_rubble_decal = nullptr;
    h.add_new_building_occupant = nullptr;
    h.claim_construction_resources = nullptr;
    h.flora_add = nullptr;
    h.flora_demolish_in_cell_area = nullptr;
    h.demolish_occupant = nullptr;
    h.demolish_lot = nullptr;
    h.modify_terrain_height = nullptr;
    h.unit_create = nullptr;
    h.unit_add = nullptr;
    h.unit_remove = nullptr;
    h.occupant_moved = nullptr;
    return h;
}

}  // namespace fc4
