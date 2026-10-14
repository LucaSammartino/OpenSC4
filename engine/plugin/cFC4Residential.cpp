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

#include "cFC4Residential.h"

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
#include "SC4String.h"
#include "SC4Vector.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4AutomataControllerManager.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4Demand.h"
#include "cISC4DemandSimulator.h"
#include "cISC4GrowthDeveloper.h"
#include "cISC4HistoryWarehouse.h"
#include "cISC4LineItem.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4Ordinance.h"
#include "cISC4OrdinanceSimulator.h"
#include "cISC4PollutionSimulator.h"
#include "cISC4SimGrid.h"
#include "cISC4Simulator.h"
#include "cISCPropertyHolder.h"
#include "cISTETerrain.h"
#include "cRZBaseString.h"
#include "cS3DVector3.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdlib>
#include <cstring>
#include <ctime>
#include <string>
#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cIGZSerializable = 0xE4FDA3D4;
constexpr uint32_t kIID_cISC4BuildingOccupant = 0x87DDA3A9;

// Messages (§4.3, §8, REH §5).
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgHistoryRecord = 0x89EFA536;

// Tuning exemplar (§7).
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0x69189C84;

// Departments (REH §4.2): budget group, LTEXT group of the names, and the two
// secondary departments that get a local line item 1.
constexpr uint32_t kDepartmentGroup = 0x6A357B7F;
constexpr uint32_t kDepartmentNameGroup = 0x6A3FF01C;
constexpr uint32_t kSecondaryHealth = 0xAA538CB3;
constexpr uint32_t kSecondaryEducation = 0x4A538CC6;

// The growth developers (§4.1 step 1).
constexpr uint32_t kDeveloperTypes[3] = {0x1010, 0x1020, 0x1030};
// The demand node Init holds (§4.1 step 1).
constexpr uint32_t kDemandNode = 0x1000;

// The state the month line calls "idle" (§5.1).
constexpr uint32_t kStateIdle = 10;

using GridU8 = cISC4SimGrid<uint8_t>;
using GridU16 = cISC4SimGrid<uint16_t>;
using GridF32 = cISC4SimGrid<float>;

fc4::ResidentialContext* Ctx(void* u) { return static_cast<fc4::ResidentialContext*>(u); }

cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cISCPropertyHolder* HolderOf(uint64_t occ) { return occ ? OccOf(occ)->AsPropertyHolder() : nullptr; }  // slot 6
cFC4ISC4DepartmentBudget* DeptOf(uint64_t t) {
    return t ? fc4::AsGame(reinterpret_cast<cISC4DepartmentBudget*>(FromToken(t))) : nullptr;
}
cISC4LineItem* LineOf(uint64_t t) { return reinterpret_cast<cISC4LineItem*>(FromToken(t)); }

// The residential_* switches; the shadow's context reads the game as it is.
bool Gate(void* u, fc4::Sw s, const char* what) {
    if (!Ctx(u)->gated || fc4::On(s)) return true;
    fc4::NoteSkipped(s, what);
    return false;
}

// An input that cannot be read: logged once per process and kind; the
// caller answers the spec's neutral value. Main thread (every host callback
// runs inside a core call or the shadow's delivery).
enum Missing : uint32_t {
    kNoCity = 1u << 0,
    kNoBds = 1u << 1,
    kNoBuildingMap = 1u << 2,
    kNoDeveloper = 1u << 3,
    kNoPopulationMap = 1u << 4,
    kNoPollution = 1u << 5,
    kNoSimulator = 1u << 6,
    kNoTerrain = 1u << 7,
    kNoOrdinances = 1u << 8,
    kNoBudget = 1u << 9,
    kNoLotManager = 1u << 10,
    kNoTuning = 1u << 11,
    kNoHistory = 1u << 12,
    kNoAutomata = 1u << 13,
    kNoMessageServer = 1u << 14,
    kNoEffects = 1u << 15,
    kNoDemandNode = 1u << 16,
};
uint32_t g_missingLogged = 0;

// The shadow's reads stay quiet: what it cannot read is logged once at
// start-up (fc4_residential_shadow_describe).
void NoteMissing(void* u, uint32_t kind, const char* text) {
    if (Ctx(u)->readOnly || (g_missingLogged & kind) != 0) return;
    g_missingLogged |= kind;
    fc4::Log("residential: %s (logged once)", text);
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

// AddRef / Release of a held object: plain on one of ours (no audit), audited
// on a game object.
void HoldObject(cIGZUnknown* o) {
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->AddRef();
    } else {
        FC4_ADDREF(o);
    }
}

void DropObject(cIGZUnknown* o) {
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->Release();
    } else {
        FC4_RELEASE(o);
    }
}

template <class T>
void Hold(T*& member, T* value) {
    if (member != nullptr || value == nullptr) return;
    member = value;
    HoldObject(reinterpret_cast<cIGZUnknown*>(member));
}

template <class T>
void Unhold(T*& member) {
    DropObject(reinterpret_cast<cIGZUnknown*>(member));
    member = nullptr;
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

cISC4City* City(void* u) {
    cISC4City* c = Ctx(u)->city;
    if (c == nullptr) NoteMissing(u, kNoCity, "no city: the city's services read as absent");
    return c;
}

// The services: the held one (simulator), else the city's (shadow).
cISC4BuildingDevelopmentSimulator* Bds(void* u) {
    fc4::ResidentialContext* c = Ctx(u);
    if (c->bds != nullptr) return c->bds;
    cISC4City* city = City(u);
    cISC4BuildingDevelopmentSimulator* b = city ? city->GetBuildingDevelopmentSimulator() : nullptr;  // borrowed
    if (b == nullptr) NoteMissing(u, kNoBds, "no building development simulator: no building map, no population maps");
    return b;
}

cFC4ISC4BudgetSimulator* Budget(void* u) {
    fc4::ResidentialContext* c = Ctx(u);
    cISC4BudgetSimulator* b = c->budget;
    if (b == nullptr) {
        cISC4City* city = City(u);
        b = city ? city->GetBudgetSimulator() : nullptr;  // city slot 52, borrowed
    }
    if (b == nullptr) NoteMissing(u, kNoBudget, "no budget simulator: no departments, budget items or line items");
    return b ? fc4::AsGame(b) : nullptr;
}

cFC4ISC4Simulator* Simulator(void* u) {
    fc4::ResidentialContext* c = Ctx(u);
    cISC4Simulator* s = c->agentSim;
    if (s == nullptr) {
        cISC4City* city = City(u);
        s = city ? city->GetSimulator() : nullptr;  // city slot 50, borrowed
    }
    if (s == nullptr) NoteMissing(u, kNoSimulator, "no simulator: the date reads as 0");
    return s ? fc4::AsGame(s) : nullptr;
}

// ---- lifecycle (§4.1, §4.2) -------------------------------------------------------------------------

// §4.1 step 1: the city, the BDS and the occupant manager, each only if not
// held. The city comes from the application (the simulator is a city
// component created by the city).
void AcquireServices(void* u) {
    fc4::ResidentialContext* c = Ctx(u);
    Hold(c->city, AppCity());
    if (c->city == nullptr) {
        fc4::Log("residential: no city; services not acquired");
        return;
    }
    Hold(c->bds, c->city->GetBuildingDevelopmentSimulator());  // city slot 53
    Hold(c->occupantManager, c->city->GetOccupantManager());   // city slot 34
}

// §4.1 step 1: BDS.GetDeveloper(0x1010 / 0x1020 / 0x1030), each only if not held.
void AcquireDevelopers(void* u) {
    fc4::ResidentialContext* c = Ctx(u);
    if (c->bds == nullptr) return;
    for (int i = 0; i < 3; ++i) {
        cISC4GrowthDeveloper* d =
            c->bds->GetDeveloper(static_cast<cISC4BuildingDevelopmentSimulator::DeveloperType>(kDeveloperTypes[i]));
        Hold(c->developers[i], d);
    }
}

// §4.1 step 1: GetDemand(0x1000, 0) of the demand simulator (city slot 56),
// only held. Init fails without it.
bool AcquireDemandNode(void* u) {
    fc4::ResidentialContext* c = Ctx(u);
    if (c->demandNode == nullptr && c->city != nullptr) {
        cISC4DemandSimulator* ds = c->city->GetDemandSimulator();  // borrowed
        cISC4Demand* node = ds ? fc4::AsGame(ds)->GetDemand(kDemandNode, 0) : nullptr;  // slot 6, borrowed
        Hold(c->demandNode, node);
    }
    if (c->demandNode == nullptr) NoteMissing(u, kNoDemandNode, "no demand node 0x1000: Init fails (§4.1 step 1)");
    return c->demandNode != nullptr;
}

// §4.1 step 2: the message server (the core subscribes right after).
bool AcquireMessageServer(void* u) {
    fc4::ResidentialContext* c = Ctx(u);
    if (c->msgServer == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            c->msgServer = ms;
            FC4_ADDREF(c->msgServer);
        }
    }
    if (c->msgServer == nullptr) NoteMissing(u, kNoMessageServer, "no message server: Init fails (§4.1 step 2)");
    return c->msgServer != nullptr;
}

// §4.1 step 3: city.GetBudgetSimulator() if not held.
bool AcquireBudgetSimulator(void* u) {
    fc4::ResidentialContext* c = Ctx(u);
    if (c->budget == nullptr && c->city != nullptr) Hold(c->budget, c->city->GetBudgetSimulator());
    return c->budget != nullptr;
}

// §4.1 step 4: the simulator (city slot 50), held until RemoveAgent.
// [choice] The original takes it without a matching release in §4.2; it is
// released after RemoveAgent here so the reference audit balances.
bool HasSimulator(void* u) {
    fc4::ResidentialContext* c = Ctx(u);
    if (c->agentSim == nullptr) {
        cISC4City* city = c->city != nullptr ? c->city : AppCity();
        Hold(c->agentSim, city != nullptr ? city->GetSimulator() : nullptr);
    }
    if (c->agentSim == nullptr) fc4::Log("residential: no simulator; Init fails (§4.1 step 4)");
    return c->agentSim != nullptr;
}

// §4.3: AddAgent(target, level, name, flags) (simulator slot 19).
void AddAgent(void* u, uint32_t level, const char* name, uint32_t flags) {
    fc4::ResidentialContext* c = Ctx(u);
    if (c->agentSim == nullptr || c->target == nullptr || name == nullptr) return;
    const cRZBaseString n(name);
    fc4::AsGame(c->agentSim)->AddAgent(c->target, level, n, flags);
    c->agentsRegistered = true;
}

// §4.2: RemoveAgent(target) (simulator slot 20), then the simulator goes.
void RemoveAgent(void* u) {
    fc4::ResidentialContext* c = Ctx(u);
    if (c->agentSim == nullptr) return;
    if (c->agentsRegistered) fc4::AsGame(c->agentSim)->RemoveAgentAllLevels(c->target);
    c->agentsRegistered = false;
    Unhold(c->agentSim);
}

void Subscribe(void* u, uint32_t type) {
    fc4::ResidentialContext* c = Ctx(u);
    if (c->msgServer != nullptr && c->target != nullptr) c->msgServer->AddNotification(c->target, type);
}

void Unsubscribe(void* u, uint32_t type) {
    fc4::ResidentialContext* c = Ctx(u);
    if (c->msgServer != nullptr && c->target != nullptr) c->msgServer->RemoveNotification(c->target, type);
}

void ReleaseMessageServerAndBudget(void* u) {
    fc4::ResidentialContext* c = Ctx(u);
    if (c->msgServer != nullptr) FC4_RELEASE(c->msgServer);
    c->msgServer = nullptr;
    Unhold(c->budget);
}

// §4.2: the demand node, the city, the BDS, the occupant manager and the developers.
void ReleaseServices(void* u) {
    fc4::ResidentialContext* c = Ctx(u);
    Unhold(c->demandNode);
    Unhold(c->city);
    Unhold(c->bds);
    Unhold(c->occupantManager);
    for (auto& d : c->developers) Unhold(d);
}

// §4.1 step 5.
void SetHistoryQueueLength(void* u, uint32_t series, uint32_t length) {
    if (!Gate(u, fc4::Sw::ResidentialHistory, "residential SetHistoryDataQueueLength")) return;
    cISC4City* c = City(u);
    cISC4HistoryWarehouse* h = c ? c->GetHistoryWarehouse() : nullptr;  // borrowed
    if (h == nullptr) {
        NoteMissing(u, kNoHistory, "no history warehouse: queue lengths not set");
        return;
    }
    h->SetHistoryDataQueueLength(series, length);
}

// ---- references -------------------------------------------------------------------------------------

void AddRefToken(void*, uint64_t t) { HoldObject(FromToken(t)); }
void ReleaseToken(void*, uint64_t t) { DropObject(FromToken(t)); }

// A reference the game will own (slots 21, 22, 47, 49, 61).
void HandOut(void*, uint64_t t) {
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->AddRef();
    } else {
        FC4_ADDREF_FOR_CALLER(o);
    }
}

// A reference read from a save (§9.2): the stream already queried it for
// `iid` (occupants as cISC4Occupant, departments as cISC4DepartmentBudget). A
// game object arrives with the reference the core now owns; one of ours (our
// department when the budget subsystem is on) arrives without one
// (FC4Streams.h), so it is taken here.
uint64_t ResolveRef(void*, uint64_t t, uint32_t) {
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return 0;
    if (fc4::IsOurObject(o)) o->AddRef();
    return t;
}

// ---- resources (§7) ---------------------------------------------------------------------------------

fc4_props* Tuning(void* u) {
    if (!Gate(u, fc4::Sw::ExemplarLoads, "residential tuning exemplar (none)")) return nullptr;
    fc4::props::Exemplar ex;
    if (!ex.Load(kTuningType, kTuningGroup, kTuningInstance)) {
        NoteMissing(u, kNoTuning, "tuning exemplar 0x6534284A/0xE7E2C2DB/0x69189C84 unavailable; the code defaults stay");
        return nullptr;
    }
    return fc4::props::ToCoreAll(ex.Holder());
}

// ---- the monthly pass's inputs (§5.3, §5.5) ---------------------------------------------------------

// BDS slot 41 GetBuildingMap: a borrowed cISC4SimGrid<uint8>.
uint64_t BuildingMap(void* u) {
    if (!Gate(u, fc4::Sw::ResidentialDevelopers, "residential GetBuildingMap (none)")) return 0;
    cISC4BuildingDevelopmentSimulator* b = Bds(u);
    const intptr_t g = b ? b->GetBuildingMap() : 0;
    if (b != nullptr && g == 0) NoteMissing(u, kNoBuildingMap, "the BDS has no building map: no proximity sources");
    return static_cast<uint64_t>(static_cast<uintptr_t>(g));
}

// BDS slot 42 GetBuildingMapKey(purpose, wealth) (AL).
uint8_t BuildingMapKey(void* u, uint32_t purpose, uint32_t wealth) {
    if (!Gate(u, fc4::Sw::ResidentialDevelopers, "residential GetBuildingMapKey (0)")) return 0;
    cISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr) return 0;
    const intptr_t k = b->GetBuildingMapKey(static_cast<cISC4BuildingOccupant::PurposeType>(purpose),
                                            static_cast<cISC4BuildingOccupant::WealthType>(wealth));
    return static_cast<uint8_t>(k);
}

// Growth developer slot 15 GetPopulationMap of 0x1010 / 0x1020 / 0x1030: the
// held developer (simulator), else the BDS's (shadow). Borrowed.
uint64_t DeveloperPopulationMap(void* u, uint32_t type) {
    if (!Gate(u, fc4::Sw::ResidentialDevelopers, "residential GetPopulationMap (none)")) return 0;
    fc4::ResidentialContext* c = Ctx(u);
    cISC4GrowthDeveloper* d = nullptr;
    for (int i = 0; i < 3; ++i) {
        if (kDeveloperTypes[i] == type) d = c->developers[i];
    }
    if (d == nullptr) {
        cISC4BuildingDevelopmentSimulator* b = Bds(u);
        d = b ? b->GetDeveloper(static_cast<cISC4BuildingDevelopmentSimulator::DeveloperType>(type)) : nullptr;
    }
    if (d == nullptr) {
        NoteMissing(u, kNoDeveloper, "a growth developer (0x1010 / 0x1020 / 0x1030) is missing: its population reads 0");
        return 0;
    }
    cISC4SimGrid<int16_t>* g = d->GetPopulationMap();
    if (g == nullptr) NoteMissing(u, kNoPopulationMap, "a growth developer has no population map: its population reads 0");
    return TokenOf(g);
}

// Pollution slot 6 GetPollutionValue(type, x, z) (AX).
uint16_t PollutionValue(void* u, uint32_t kind, int32_t x, int32_t z) {
    if (!Gate(u, fc4::Sw::ResidentialPollution, "residential GetPollutionValue (0)")) return 0;
    cISC4City* c = City(u);
    cISC4PollutionSimulator* p = c ? c->GetPollutionSimulator() : nullptr;  // borrowed
    if (p == nullptr) {
        NoteMissing(u, kNoPollution, "no pollution simulator: pollution reads 0");
        return 0;
    }
    return fc4::AsGame(p)->GetPollutionValue(kind, static_cast<uint32_t>(x), static_cast<uint32_t>(z));
}

// Simulator slot 6 GetSimDate(year*, month*, day*, dayOfYear*, weekDay*): the month.
uint32_t SimMonth(void* u) {
    cFC4ISC4Simulator* s = Simulator(u);
    if (s == nullptr) return 0;
    uint32_t y = 0, m = 0, d = 0, doy = 0, wd = 0;
    s->GetSimDateParts(&y, &m, &d, &doy, &wd);
    return m;
}

// city.GetSimulator().GetSimDate().DayNumber() (REH §4.9).
uint32_t SimDayNumber(void* u) {
    cFC4ISC4Simulator* s = Simulator(u);
    cFC4IGZDate* d = s ? s->GetSimDate() : nullptr;  // simulator slot 7, borrowed
    return d ? d->DayNumber() : 0;                  // date slot 26
}

float Altitude(void* u, float x, float z) {
    if (!Gate(u, fc4::Sw::ResidentialTerrain, "residential GetAltitude (0)")) return 0.0f;
    cISC4City* c = City(u);
    cISTETerrain* t = c ? c->GetTerrain() : nullptr;
    if (t == nullptr) {
        NoteMissing(u, kNoTerrain, "no terrain: altitude reads 0");
        return 0.0f;
    }
    return t->GetAltitude(x, z);  // compiled slot 13 (foundations §16), ST0
}

// §5.5 step 2: [choice, spec question] no visual effect (buzzards) is
// created: the effects manager is not part of the verified interfaces. The
// core then skips the effect (CreateVisualEffect returned none).
uint64_t CreateVisualEffect(void* u, const char*) {
    ++Ctx(u)->effects;
    NoteMissing(u, kNoEffects, "visual effects (buzzards, §5.5 step 2) are not created");
    return 0;
}

// ---- messages (§8, REH §5) --------------------------------------------------------------------------

cIGZMessageServer2* Server(void* u) {
    fc4::ResidentialContext* c = Ctx(u);
    return c->msgServer;
}

// §8: MessageSend (synchronous) of a new standard message 0x89EFA536 with
// data1 = series, data2 = value; nothing without a message server.
void RecordHistory(void* u, uint32_t series, int32_t value) {
    if (!Gate(u, fc4::Sw::ResidentialHistory, "residential history record")) return;
    cIGZMessageServer2* ms = Server(u);
    if (ms == nullptr) return;
    cFC4Message2Standard* msg =
        FC4MakeMessage(kMsgHistoryRecord, static_cast<intptr_t>(series), static_cast<intptr_t>(value), 0);
    ms->MessageSend(msg);
    msg->Release();
    ++Ctx(u)->history;
}

// REH §5: MessagePost(msg, false) of a new standard message: a strike start
// carries void1 = IGZUnknown = the building's occupant and data2 = 1; an end
// data2 = 0 and data3 = its length in days.
void PostStrikeMessage(void* u, uint32_t type, uint64_t occ, uint32_t data2, uint32_t data3) {
    if (!Gate(u, fc4::Sw::ResidentialPosts, "residential strike message post")) return;
    cIGZMessageServer2* ms = Server(u);
    cIGZMessageServer2Ptr fallback;
    if (ms == nullptr) ms = fallback;
    if (ms == nullptr) return;
    cFC4Message2Standard* msg =
        FC4MakeMessage(type, 0, static_cast<intptr_t>(data2), static_cast<intptr_t>(data3));
    if (occ != 0) {
        msg->SetVoid1(FromToken(occ));
        msg->SetIGZUnknown(FromToken(occ));
    }
    ms->MessagePost(msg, false);
    msg->Release();
    ++Ctx(u)->posts;
}

// ---- occupants (§4.3, §6.6, REH §2, §4.1) -----------------------------------------------------------

// §4.3: void1 queried for cISC4BuildingOccupant (0x87DDA3A9). [choice, spec
// question] The core knows a building by its cISC4Occupant identity
// (AsOccupant), the identity the public slots take (19, 20, 26, 27, 45, 56,
// 57, 58), the lists hand out (47, 49, 61) and the record reads (§9.2,
// IID 0x26BB767E).
uint64_t OccupantAsBuilding(void* u, uint64_t occ) {
    if (occ == 0 || !Gate(u, fc4::Sw::ResidentialOccupants, "residential building QueryInterface (not a building)")) {
        return 0;
    }
    cISC4BuildingOccupant* b = Query<cISC4BuildingOccupant>(FromToken(occ), kIID_cISC4BuildingOccupant);
    if (b == nullptr) return 0;
    cISC4Occupant* o = b->AsOccupant();  // building slot 3, borrowed
    Drop(b);
    return TokenOf(o);
}

bool IsOccupantGroup(void* u, uint64_t occ, uint32_t group) {
    if (occ == 0 || !Gate(u, fc4::Sw::ResidentialOccupants, "residential IsOccupantGroup (false)")) return false;
    return OccOf(occ)->IsOccupantGroup(group);  // occupant slot 22
}

bool OccupantBoolProperty(void* u, uint64_t occ, uint32_t id, bool* out) {
    if (occ == 0 || !Gate(u, fc4::Sw::ResidentialOccupants, "residential occupant bool property (absent)")) {
        return false;
    }
    std::vector<uint32_t> v;
    if (!fc4::props::GetU32s(HolderOf(occ), id, v) || v.empty()) return false;
    if (out != nullptr) *out = v[0] != 0;
    return true;
}

// [choice, spec question] Written as a scalar bool (variant type 0x01).
bool SetOccupantBoolProperty(void* u, uint64_t occ, uint32_t id, bool value) {
    if (occ == 0 || !Gate(u, fc4::Sw::ResidentialOccupants, "residential occupant bool property write")) return false;
    if (!Gate(u, fc4::Sw::PropertyWrites, "residential occupant bool property write")) return false;
    const uint8_t b = value ? 1 : 0;
    cISCPropertyHolder* h = HolderOf(occ);
    return h != nullptr && fc4::props::SetTyped(h, id, 0x01, false, &b, 1);
}

bool OccupantProperty(void* u, uint64_t occ, uint32_t id, fc4_props* out) {
    if (occ == 0 || !Gate(u, fc4::Sw::ResidentialOccupants, "residential occupant property read (absent)")) {
        return false;
    }
    cISCPropertyHolder* h = HolderOf(occ);
    return h != nullptr && fc4::props::CopyToCore(h, id, out);
}

// [choice, spec question] REH §2's per-building values are written as
// scalars of their type (uint32 0x06, float32 0x0A).
void SetOccupantTyped(void* u, uint64_t occ, uint32_t id, uint16_t type, const void* data) {
    if (occ == 0 || !Gate(u, fc4::Sw::ResidentialOccupants, "residential occupant property write")) return;
    if (!Gate(u, fc4::Sw::PropertyWrites, "residential occupant property write")) return;
    cISCPropertyHolder* h = HolderOf(occ);
    if (h != nullptr && !fc4::props::SetTyped(h, id, type, false, data, 1)) {
        fc4::Log("residential: WARNING property 0x%08X (type 0x%02X) not written", id, type);
    }
}
void SetOccupantU32(void* u, uint64_t occ, uint32_t id, uint32_t v) { SetOccupantTyped(u, occ, id, 0x06, &v); }
void SetOccupantF32(void* u, uint64_t occ, uint32_t id, float v) { SetOccupantTyped(u, occ, id, 0x0A, &v); }

// The building interface of a building's occupant (one reference) or nullptr.
cISC4BuildingOccupant* BuildingOf(void* u, uint64_t occ, const char* what) {
    if (occ == 0 || !Gate(u, fc4::Sw::ResidentialOccupants, what)) return nullptr;
    return Query<cISC4BuildingOccupant>(FromToken(occ), kIID_cISC4BuildingOccupant);
}

// Building slot 4 GetBuildingType.
uint32_t BuildingType(void* u, uint64_t occ) {
    cISC4BuildingOccupant* b = BuildingOf(u, occ, "residential GetBuildingType (0)");
    if (b == nullptr) return 0;
    const uint32_t t = b->GetBuildingType();
    Drop(b);
    return t;
}

// Building slot 11 (GetBuildingProfile): its first u32 (REH §4.1 step 4).
uint32_t BuildingSlot11(void* u, uint64_t occ) {
    cISC4BuildingOccupant* b = BuildingOf(u, occ, "residential building slot 11 (0)");
    if (b == nullptr) return 0;
    const cISC4BuildingOccupant::BuildingProfile& p = b->GetBuildingProfile();
    uint32_t v = 0;
    std::memcpy(&v, &p, sizeof(v));
    Drop(b);
    return v;
}

// Lot slot 15 GetCenter(float* out3) through a thunk that reads the out
// parameter (crime.md §13, CQ3: the x87 stack is empty on return; a value
// left there is popped and counted). Returns whether ST0 held a value.
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

// GetOccupantLot (lot manager slot 14, borrowed), then its centre (x, z).
bool OccupantLotCenter(void* u, uint64_t occ, float* xz) {
    if (occ == 0 || !Gate(u, fc4::Sw::ResidentialLots, "residential GetOccupantLot (no lot)")) return false;
    cISC4City* c = City(u);
    cISC4LotManager* lm = c ? c->GetLotManager() : nullptr;  // city slot 33
    if (lm == nullptr) {
        NoteMissing(u, kNoLotManager, "no lot manager: buildings use their own position");
        return false;
    }
    cISC4Lot* lot = fc4::AsGame(lm)->GetOccupantLot(OccOf(occ));
    if (lot == nullptr) return false;
    float v[4] = {0.0f, 0.0f, 0.0f, 0.0f};
    if (LotCenterRaw(lot, v)) {
        static bool warned = false;  // main thread only
        if (!warned) {
            warned = true;
            fc4::Log("residential: WARNING lot GetCenter (slot 15) left a value in ST0; popped (logged once)");
        }
    }
    if (xz != nullptr) {
        xz[0] = v[0];
        xz[1] = v[2];
    }
    return true;
}

void OccupantPosition(void* u, uint64_t occ, float* xz) {
    if (xz == nullptr) return;
    xz[0] = xz[1] = 0.0f;
    if (occ == 0 || !Gate(u, fc4::Sw::ResidentialOccupants, "residential occupant GetPosition (0, 0)")) return;
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);  // slot 8
    xz[0] = v.fX;
    xz[1] = v.fZ;
}

// ---- budget (REH §4.1, §4.2) ------------------------------------------------------------------------

// CreateDepartmentBudget(id, 0x6A357B7F) (slot 25, borrowed), AddRef (the
// core's reference), SetDepartmentName(0x6A3FF01C, name),
// SetMaxAllowedFundingPercentage, SetFundingPercentage(initial, 0); the two
// secondary departments also CreateLineItem(1, true) and clear display flag 1
// of line item 1.
uint64_t CreateDepartment(void* u, uint32_t id, uint32_t name, float maxFunding, float initialFunding) {
    if (!Gate(u, fc4::Sw::ResidentialBudget, "residential CreateDepartmentBudget (none)")) return 0;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    cISC4DepartmentBudget* d = b ? b->CreateDepartmentBudget(id, kDepartmentGroup) : nullptr;
    if (d == nullptr) {
        fc4::Log("residential: no department 0x%08X (no budget simulator, or it refused)", id);
        return 0;
    }
    const uint64_t t = TokenOf(d);
    AddRefToken(u, t);
    cFC4ISC4DepartmentBudget* g = fc4::AsGame(d);
    g->SetDepartmentName(kDepartmentNameGroup, name);  // slot 5
    SC4Percentage max;
    max.percentage = maxFunding;
    g->SetMaxAllowedFundingPercentage(max);  // slot 13
    SC4Percentage initial;
    initial.percentage = initialFunding;
    g->SetFundingPercentage(initial, 0);  // slot 11
    if (id == kSecondaryHealth || id == kSecondaryEducation) {
        g->CreateLineItem(1, true);  // slot 17
        if (cISC4LineItem* li = g->GetLineItem(1)) {
            li->SetDisplayFlag(static_cast<cISC4LineItem::DisplayFlag>(1), false);  // line item slot 18
        }
    }
    ++Ctx(u)->departments;
    return t;
}

// GetBudgetItemForPurpose (budget slot 51).
bool BudgetItem(void* u, uint64_t occ, uint32_t purpose, fc4_residential_budget_item* out) {
    if (occ == 0 || !Gate(u, fc4::Sw::ResidentialBudget, "residential GetBudgetItemForPurpose (none)")) return false;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    cISCPropertyHolder* h = HolderOf(occ);
    if (b == nullptr || h == nullptr) return false;
    cFC4ISC4BudgetSimulator::BudgetItem item{};
    if (!b->GetBudgetItemForPurpose(h, purpose, item)) return false;
    if (out != nullptr) {
        *out = fc4_residential_budget_item{};
        out->department = item.departmentID;
        out->line = item.lineItem;
        out->purpose = item.purpose;
        out->index = static_cast<uint32_t>(item.index);
        out->cost = item.cost;
        out->local_funding = item.localFundingPercentage;
    }
    return true;
}

// ChangeBudgetItemLine (budget slot 52).
void ChangeBudgetItemLine(void* u, uint64_t occ, uint32_t index, uint32_t line) {
    if (occ == 0 || !Gate(u, fc4::Sw::ResidentialBudget, "residential ChangeBudgetItemLine")) return;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    cISCPropertyHolder* h = HolderOf(occ);
    if (b != nullptr && h != nullptr) b->ChangeBudgetItemLine(h, index, line);
}

// GetDepartmentBudget (budget slot 28), borrowed.
uint64_t DepartmentBudget(void* u, uint32_t id) {
    if (!Gate(u, fc4::Sw::ResidentialBudget, "residential GetDepartmentBudget (none)")) return 0;
    cFC4ISC4BudgetSimulator* b = Budget(u);
    return b ? TokenOf(b->GetDepartmentBudget(id)) : 0;
}

float DepartmentFunding(void*, uint64_t dept) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    SC4Percentage* p = d ? d->GetFundingPercentage() : nullptr;  // slot 10
    return p ? p->percentage : 1.0f;
}

bool IsLocallyFunded(void* u, uint64_t dept, uint64_t occ, uint32_t purpose) {
    if (!Gate(u, fc4::Sw::ResidentialBudget, "residential IsLocallyFundedObjectInDepartment (false)")) return false;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    cISCPropertyHolder* h = HolderOf(occ);
    return d != nullptr && h != nullptr && d->IsLocallyFundedObjectInDepartment(h, purpose);  // slot 28
}

void AddLocallyFunded(void* u, uint64_t dept, uint64_t occ, uint32_t purpose) {
    if (!Gate(u, fc4::Sw::ResidentialBudget, "residential AddLocallyFundedObject")) return;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    cISCPropertyHolder* h = HolderOf(occ);
    if (d != nullptr && h != nullptr) d->AddLocallyFundedObject(h, purpose);  // slot 22
}

void RemoveLocallyFunded(void* u, uint64_t dept, uint64_t occ, uint32_t purpose) {
    if (!Gate(u, fc4::Sw::ResidentialBudget, "residential RemoveLocallyFundedObject")) return;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    cISCPropertyHolder* h = HolderOf(occ);
    if (d != nullptr && h != nullptr) d->RemoveLocallyFundedObject(h, purpose);  // slot 23
}

void SetLocalFunding(void* u, uint64_t dept, uint64_t occ, float fraction, uint32_t purpose) {
    if (!Gate(u, fc4::Sw::ResidentialBudget, "residential SetLocalFundingPercent")) return;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    cISCPropertyHolder* h = HolderOf(occ);
    if (d == nullptr || h == nullptr) return;
    SC4Percentage p;
    p.percentage = fraction;
    d->SetLocalFundingPercent(h, &p, purpose);  // slot 24 (the percentage by reference)
}

uint64_t LineItem(void* u, uint64_t dept, uint32_t line) {
    if (!Gate(u, fc4::Sw::ResidentialBudget, "residential GetLineItem (none)")) return 0;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    return d ? TokenOf(d->GetLineItem(line)) : 0;  // slot 20, borrowed
}

uint64_t CreateLineItemForBuildingType(void* u, uint64_t dept, uint32_t line, bool local) {
    if (!Gate(u, fc4::Sw::ResidentialBudget, "residential CreateLineItemForBuildingType (none)")) return 0;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    return d ? TokenOf(d->CreateLineItemForBuildingType(line, local)) : 0;  // slot 18, borrowed
}

void RemoveLineItem(void* u, uint64_t dept, uint32_t line) {
    if (!Gate(u, fc4::Sw::ResidentialBudget, "residential RemoveLineItem")) return;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    if (d != nullptr) d->RemoveLineItem(line);  // slot 19
}

int64_t LineItemCount(void*, uint64_t li) {
    cISC4LineItem* l = LineOf(li);
    return l ? l->GetSecondaryInfoField() : 0;  // line item slot 8
}

void SetLineItemCount(void* u, uint64_t li, int64_t count) {
    if (!Gate(u, fc4::Sw::ResidentialBudget, "residential line item SetSecondaryInfoField")) return;
    if (cISC4LineItem* l = LineOf(li)) l->SetSecondaryInfoField(count);  // line item slot 9
}

void SetLineItemDisplayFlag(void* u, uint64_t li, uint32_t flag, bool on) {
    if (!Gate(u, fc4::Sw::ResidentialBudget, "residential line item SetDisplayFlag")) return;
    if (cISC4LineItem* l = LineOf(li)) l->SetDisplayFlag(static_cast<cISC4LineItem::DisplayFlag>(flag), on);  // 18
}

// ---- ordinances (REH §4.3) --------------------------------------------------------------------------

cFC4ISC4OrdinanceSimulator* Ordinances(void* u) {
    cISC4City* c = City(u);
    cISC4OrdinanceSimulator* o = c ? c->GetOrdinanceSimulator() : nullptr;  // borrowed
    if (o == nullptr) NoteMissing(u, kNoOrdinances, "no ordinance simulator: no ordinance effects");
    return o ? fc4::AsGame(o) : nullptr;
}

// GetOrdinanceOnCount (slot 14), then GetOrdinanceOnArray (slot 15) with a
// buffer of that size.
void OrdinancesOn(void* u, fc4_u32_callback cb, void* ctx) {
    if (cb == nullptr || !Gate(u, fc4::Sw::ResidentialOrdinance, "residential ordinances on (none)")) return;
    cFC4ISC4OrdinanceSimulator* o = Ordinances(u);
    if (o == nullptr) return;
    const uint32_t count = o->GetOrdinanceOnCount();
    if (count == 0) return;
    std::vector<uint32_t> ids(count);
    uint32_t n = count;
    o->GetOrdinanceOnArray(ids.data(), n);
    if (n > count) n = count;
    for (uint32_t i = 0; i < n; ++i) cb(ctx, ids[i]);
}

// GetOrdinanceByID (slot 20, borrowed) -> GetMiscProperties (ordinance slot 16).
fc4_props* OrdinanceProperties(void* u, uint32_t id) {
    if (!Gate(u, fc4::Sw::ResidentialOrdinance, "residential ordinance properties (skipped)")) return nullptr;
    cFC4ISC4OrdinanceSimulator* o = Ordinances(u);
    cISC4Ordinance* ord = o ? o->GetOrdinanceByID(id) : nullptr;
    cISCPropertyHolder* h = ord ? fc4::AsGame(ord)->GetMiscProperties() : nullptr;
    return h ? fc4::props::ToCoreAll(h) : nullptr;
}

// ---- automata (REH §4.9) ----------------------------------------------------------------------------

// [abi] AddControllersFromOccupantGroup(name, occ, 0) and
// RemoveControllersFromOccupantGroup(name, occ) through the SDK declaration
// (overloads reversed as compiled; abi_conformance_test).
void Controllers(void* u, const char* group, uint64_t occ, bool add) {
    if (group == nullptr || occ == 0) return;
    if (!Gate(u, fc4::Sw::ResidentialAutomata, add ? "residential AddControllersFromOccupantGroup"
                                                   : "residential RemoveControllersFromOccupantGroup")) {
        return;
    }
    cISC4City* c = City(u);
    cISC4AutomataControllerManager* m = c ? c->GetAutomataControllerManager() : nullptr;  // city slot 47
    if (m == nullptr) {
        NoteMissing(u, kNoAutomata, "no automata controller manager: strike automata not changed");
        return;
    }
    const SC4String name(group, static_cast<uint32_t>(std::strlen(group)));
    if (add) {
        m->AddControllersFromOccupantGroup(name, OccOf(occ), nullptr);
    } else {
        m->RemoveControllersFromOccupantGroup(name, OccOf(occ));
    }
    ++Ctx(u)->controllers;
}
void AddControllers(void* u, const char* g, uint64_t occ) { Controllers(u, g, occ, true); }
void RemoveControllers(void* u, const char* g, uint64_t occ) { Controllers(u, g, occ, false); }

// ---- small helpers ----------------------------------------------------------------------------------

// The session seed S (foundations §6.3): FreeCity4.ini `residential_seed`
// (env FC4_RESIDENTIAL_SEED) when set, else the wall clock (§3: the original
// seeds its cRZRandom with 0xFFFFFFFF, the wall clock, at construction),
// taken as the cRZTime value for "now" like the crime glue (time() +
// 2177452800). The core seeds the class's generator with
// derive_seed(S, 0x4990C013) (fc4.h).
uint32_t SessionSeed(bool* fixed) {
    const std::string s = fc4::Setting("residential_seed", "FC4_RESIDENTIAL_SEED");
    if (!s.empty()) {
        char* end = nullptr;
        const unsigned long v = std::strtoul(s.c_str(), &end, 0);
        if (end != s.c_str()) {
            *fixed = true;
            return static_cast<uint32_t>(v);
        }
    }
    *fixed = false;
    constexpr int64_t kRzTimeUnixOffset = 2177452800;  // 1901-01-01 to 1970-01-01
    return static_cast<uint32_t>(static_cast<int64_t>(std::time(nullptr)) + kRzTimeUnixOffset);
}

// The game's query record <-> the core's (REH §4.11): the costs are 32-bit
// in the record. [choice, spec question] Widened as unsigned, written back
// as their low 32 bits.
fc4_residential_query_data ToCore(const FC4ResidentialQueryData& d) {
    fc4_residential_query_data q{};
    q.capacity = d.capacity;
    q.effective_capacity = d.effectiveCapacity;
    q.capacity_per_staff = d.capacityPerStaff;
    q.effective_per_staff = d.effectivePerStaff;
    q.served = d.served;
    q.grade = d.grade;
    q.staff_funding = d.staffFunding;
    q.staff_cost = static_cast<int64_t>(d.staffCost);
    q.vehicle_funding = d.vehicleFunding;
    q.vehicle_cost = static_cast<int64_t>(d.vehicleCost);
    return q;
}

void FromCore(const fc4_residential_query_data& q, FC4ResidentialQueryData& d) {
    d.capacity = q.capacity;
    d.effectiveCapacity = q.effective_capacity;
    d.capacityPerStaff = q.capacity_per_staff;
    d.effectivePerStaff = q.effective_per_staff;
    d.served = q.served;
    d.grade = q.grade;
    d.staffFunding = q.staff_funding;
    d.staffCost = static_cast<uint32_t>(static_cast<uint64_t>(q.staff_cost));
    d.vehicleFunding = q.vehicle_funding;
    d.vehicleCost = static_cast<uint32_t>(static_cast<uint64_t>(q.vehicle_cost));
}

// The caller's group vector (SC4Vector: mpStart, mpEnd, mpCapacity), read only.
struct Groups {
    const uint32_t* data = nullptr;
    uint32_t count = 0;
};
Groups GroupsOf(SC4Vector<uint32_t> const* v) {
    static const uint32_t kNone = 0;
    Groups g;
    if (v == nullptr) return g;
    const uint32_t* b = v->begin();
    const uint32_t* e = v->end();
    g.data = b != nullptr ? b : &kNone;  // non-null: "these groups" (possibly none)
    g.count = (b != nullptr && e > b) ? static_cast<uint32_t>(e - b) : 0;
    return g;
}

// Slots 47, 49, 61: each occupant arrives with one reference handed out for
// the caller's list (HandOut) and is appended (allocator A, docs/abi
// allocators.md).
void AppendToList(void* ctx, uint64_t t) {
    cISC4Occupant* o = OccOf(t);
    if (o != nullptr) static_cast<SC4List<cISC4Occupant*>*>(ctx)->push_back(o);
}

template <class G>
void HandOutGrid(G*& out, uint64_t t) {
    out = reinterpret_cast<G*>(FromToken(t));
    if (out != nullptr) HandOut(nullptr, t);
}

}  // namespace

// ---- host tables ------------------------------------------------------------------------------------

namespace fc4 {

fc4_residential_host MakeResidentialReadOnlyHost(ResidentialContext* ctx, const HostGridContext* grids) {
    fc4_residential_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    // Geometry and reads only: the shadow's grids are its own copies.
    h.grids = MakeGridApi(grids);
    h.grids.create = nullptr;
    h.grids.set = nullptr;
    h.grids.fill = nullptr;
    h.grids.shutdown = nullptr;
    h.grids.release = nullptr;
    // resources and the monthly pass's inputs
    h.tuning = &Tuning;
    h.building_map = &BuildingMap;
    h.building_map_key = &BuildingMapKey;
    h.developer_population_map = &DeveloperPopulationMap;
    h.pollution_value = &PollutionValue;
    h.sim_month = &SimMonth;
    h.sim_day_number = &SimDayNumber;
    h.altitude = &Altitude;
    // occupants
    h.occupant_as_building = &OccupantAsBuilding;
    h.is_occupant_group = &IsOccupantGroup;
    h.occupant_bool_property = &OccupantBoolProperty;
    h.occupant_property = &OccupantProperty;
    h.building_type = &BuildingType;
    h.building_slot11 = &BuildingSlot11;
    h.occupant_lot_center = &OccupantLotCenter;
    h.occupant_position = &OccupantPosition;
    // budget reads
    h.budget_item = &BudgetItem;
    h.department_budget = &DepartmentBudget;
    h.department_funding = &DepartmentFunding;
    h.is_locally_funded = &IsLocallyFunded;
    h.line_item = &LineItem;
    h.line_item_count = &LineItemCount;
    // ordinances
    h.ordinances_on = &OrdinancesOn;
    h.ordinance_properties = &OrdinanceProperties;
    return h;
}

fc4_residential_host MakeResidentialHost(ResidentialContext* ctx, const HostGridContext* grids) {
    fc4_residential_host h = MakeResidentialReadOnlyHost(ctx, grids);
    h.grids = MakeGridApi(grids);
    // lifecycle
    h.acquire_services = &AcquireServices;
    h.acquire_developers = &AcquireDevelopers;
    h.acquire_demand_node = &AcquireDemandNode;
    h.acquire_message_server = &AcquireMessageServer;
    h.acquire_budget_simulator = &AcquireBudgetSimulator;
    h.has_simulator = &HasSimulator;
    h.add_agent = &AddAgent;
    h.remove_agent = &RemoveAgent;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.release_message_server_and_budget = &ReleaseMessageServerAndBudget;
    h.release_services = &ReleaseServices;
    h.set_history_queue_length = &SetHistoryQueueLength;
    h.create_department = &CreateDepartment;
    h.release_department = &ReleaseToken;
    // references
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.hand_out = &HandOut;
    h.resolve_ref = &ResolveRef;
    // effects (none, §5.5 step 2), messages
    h.create_visual_effect = &CreateVisualEffect;
    h.record_history = &RecordHistory;
    h.post_strike_message = &PostStrikeMessage;
    // occupant writes
    h.set_occupant_bool_property = &SetOccupantBoolProperty;
    h.set_occupant_u32 = &SetOccupantU32;
    h.set_occupant_f32 = &SetOccupantF32;
    // budget writes
    h.change_budget_item_line = &ChangeBudgetItemLine;
    h.add_locally_funded = &AddLocallyFunded;
    h.remove_locally_funded = &RemoveLocallyFunded;
    h.set_local_funding = &SetLocalFunding;
    h.create_line_item_for_building_type = &CreateLineItemForBuildingType;
    h.remove_line_item = &RemoveLineItem;
    h.set_line_item_count = &SetLineItemCount;
    h.set_line_item_display_flag = &SetLineItemDisplayFlag;
    // automata
    h.add_controllers = &AddControllers;
    h.remove_controllers = &RemoveControllers;
    return h;
}

}  // namespace fc4

// ---- object -----------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4ResidentialSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4ResidentialSimulator*>(new cFC4ResidentialSimulator()));
}

cFC4ResidentialSimulator::cFC4ResidentialSimulator() {
    mainTid_ = static_cast<uint32_t>(GetCurrentThreadId());  // the class store creates city components on the main thread
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    host_ = fc4::MakeResidentialHost(&ctx_, &gridCtx_);
    bool fixed = false;
    const uint32_t seed = SessionSeed(&fixed);
    // The calling thread (the class store's, on the main thread) becomes the
    // core's main thread (fc4.h).
    core_ = fc4_residential_create(&host_, SelfToken(), seed);
    if (core_ == nullptr) fc4::Log("cFC4ResidentialSimulator: core creation failed");
    fc4::Log("cFC4ResidentialSimulator(%p) created, core %p, session seed 0x%08X (%s)", static_cast<void*>(this),
             static_cast<void*>(core_), seed, fixed ? "residential_seed" : "cRZTime");
}

cFC4ResidentialSimulator::~cFC4ResidentialSimulator() {
    fc4_residential_destroy(core_);  // releases what a loaded, never-initialised object still holds
    core_ = nullptr;
    // Whatever a never-initialised object still holds (a Read without Init).
    ReleaseMessageServerAndBudget(&ctx_);
    RemoveAgent(&ctx_);
    ReleaseServices(&ctx_);
    fc4::Log("cFC4ResidentialSimulator destroyed (%u days, %u months, %u cycles, %u off-main messages)", days_,
             months_, cycles_, offMainMessages_.load());
}

// §1: 0x00000001 and 0x077AC1EE give the primary interface, 0x452294AA the
// message target, 0xE4FDA3D4 the serializable; anything else is false and
// the out pointer is left untouched.
bool cFC4ResidentialSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case GZIID_cIGZUnknown:
        case kFC4_IID_cISC4ResidentialSimulator:
            *ppvObj = static_cast<cFC4ISC4ResidentialSimulator*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case kIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        default:
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4ResidentialSimulator::AddRef() { return ++refCount_; }

// §1: at a count of 1 (or below) Shutdown, destroy, return 0.
uint32_t cFC4ResidentialSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

fc4_residential_snapshot cFC4ResidentialSimulator::Snapshot() const {
    fc4_residential_snapshot s{};
    s.struct_size = sizeof(s);
    if (core_ != nullptr) fc4_residential_get_snapshot(core_, &s);
    return s;
}

// ---- status lines -----------------------------------------------------------------------------------

// Init / Read / Shutdown. Main thread with no call running (get_state).
void cFC4ResidentialSimulator::LogState(const char* what) {
    fc4_residential_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_residential_get_state(core_, &st)) {
        fc4::Log("residential %s: (no state)", what);
        return;
    }
    fc4::Log("residential %s: init=%u started=%u dirty=%u tuning=%u state=%u pop=%d running=%d (R$ %u R$$ %u "
             "R$$$ %u) EQ=%.3f HQ=%.3f LE=%u sizes P=%d E=%d tracts=%dx%d departments=%u/4 grids=%u/25 schools=%d "
             "hospitals=%d pending=%u/%u healthAffecting=%u other=%u strike school=%u (start %d) health=%u (start %d) "
             "autoBudget=%u/%u",
             what, st.initialized, st.city_started, st.dirty, st.tuning_loaded, st.state, st.population,
             st.running_population, st.population_by_wealth[0], st.population_by_wealth[1],
             st.population_by_wealth[2], st.global_eq, st.global_hq, st.global_le, st.population_size, st.eq_size,
             st.tracts_x, st.tracts_z, st.departments, st.grids, st.school_buildings, st.hospital_buildings,
             st.pending_school, st.pending_hospital, st.health_affecting, st.other, st.school_strike,
             st.school_strike_start, st.health_strike, st.health_strike_start, st.school_auto_budget,
             st.hospital_auto_budget);
}

// One per SimNewMonth (and the first SimNewDay after Init).
void cFC4ResidentialSimulator::LogMonth() {
    fc4_residential_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_residential_get_state(core_, &st)) return;
    fc4_date d{};
    if (lastJulian_ > 0) fc4_date_from_julian(lastJulian_, &d);
    fc4::Log("residential month %04d-%02d-%02d: pop=%d (R$ %u R$$ %u R$$$ %u) EQ=%.3f HQ=%.3f LE=%u school "
             "covered=%d buildings=%d strike=%u hospital covered=%d buildings=%d strike=%u state=%u iterator=(%d,%d) "
             "tracts=%dx%d departments=%u/4 grids=%u/25 dirty=%u reentrant=%u deferred=%u views=%u offMain=%u "
             "posts=%u history=%u controllers=%u effects=%u days=%u months=%u cycles=%u",
             d.year, d.month, d.day_of_month, st.population, st.population_by_wealth[0], st.population_by_wealth[1],
             st.population_by_wealth[2], st.global_eq, st.global_hq, st.global_le, st.school_covered,
             st.school_buildings, st.school_strike, st.hospital_covered, st.hospital_buildings, st.health_strike,
             st.state, st.iterator_x, st.iterator_z, st.tracts_x, st.tracts_z, st.departments, st.grids, st.dirty,
             st.reentrant_calls, st.deferred_calls, st.view_calls, offMainMessages_.load(), ctx_.posts, ctx_.history,
             ctx_.controllers, ctx_.effects, days_, months_, cycles_);
}

// One per monthly cycle that reached state 10 (§5.1: state 9, history, done).
void cFC4ResidentialSimulator::LogCycle() {
    fc4_residential_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_residential_get_state(core_, &st)) return;
    fc4_date d{};
    if (lastJulian_ > 0) fc4_date_from_julian(lastJulian_, &d);
    fc4::Log("residential cycle %u done %04d-%02d-%02d: pop=%d (R$ %u R$$ %u R$$$ %u) EQ=%.3f HQ=%.3f LE=%u "
             "leSum=%llu school covered=%d of %d buildings hospital covered=%d of %d buildings healthAffecting=%u "
             "history=%u",
             cycles_, d.year, d.month, d.day_of_month, st.population, st.population_by_wealth[0],
             st.population_by_wealth[1], st.population_by_wealth[2], st.global_eq, st.global_hq, st.global_le,
             static_cast<unsigned long long>(st.le_sum), st.school_covered, st.school_buildings, st.hospital_covered,
             st.hospital_buildings, st.health_affecting, ctx_.history);
}

// ---- lifecycle (§4.1, §4.2) -------------------------------------------------------------------------

bool cFC4ResidentialSimulator::Init() {
    FC4_TRACE("ResidentialSimulator::Init");
    const bool ok = core_ != nullptr && fc4_residential_init(core_);  // §4.1, all of it
    lastState_ = 0xFFFFFFFFu;
    days_ = 0;
    LogState(ok ? "Init" : "Init FAILED");
    return ok;
}

bool cFC4ResidentialSimulator::Shutdown() {
    FC4_TRACE("ResidentialSimulator::Shutdown");
    // Keep this object alive while the simulator and the message server drop
    // their references (unless this already is the final Release).
    if (!destroying_) AddRef();
    if (core_ != nullptr) fc4_residential_shutdown(core_);  // §4.2
    fc4::Log("residential Shutdown: %u days, %u months, %u cycles, %u off-main messages, posts=%u history=%u",
             days_, months_, cycles_, offMainMessages_.load(), ctx_.posts, ctx_.history);
    if (!destroying_) Release();
    return true;  // §4.2: always
}

// ---- primary slots (§10.1) --------------------------------------------------------------------------

// Slot 5: w = 1 / 2 / 3 -> R$ / R$$ / R$$$; anything else null (§6.6). No AddRef.
cISC4SimGrid<uint8_t>* cFC4ResidentialSimulator::GetProximityMap(WealthType wealth) {
    const uint32_t w = static_cast<uint32_t>(wealth);
    if (w < 1 || w > 3) return nullptr;
    return reinterpret_cast<GridU8*>(FromToken(Snapshot().proximity[w - 1]));
}

bool cFC4ResidentialSimulator::SchoolIsOnStrike() { return Snapshot().school_strike != 0; }
bool cFC4ResidentialSimulator::HealthIsOnStrike() { return Snapshot().health_strike != 0; }
bool cFC4ResidentialSimulator::EndSchoolStrike() { return core_ && fc4_residential_end_strike(core_, true); }
bool cFC4ResidentialSimulator::EndHealthStrike() { return core_ && fc4_residential_end_strike(core_, false); }

// Slots 10-13: by value through the hidden result pointer (§10.1).
SC4Percentage cFC4ResidentialSimulator::ChanceOfSchoolStrike() {
    SC4Percentage r;
    r.percentage = core_ ? fc4_residential_strike_chance(core_, true) : 0.0f;
    return r;
}

SC4Percentage cFC4ResidentialSimulator::ChanceOfHealthStrike() {
    SC4Percentage r;
    r.percentage = core_ ? fc4_residential_strike_chance(core_, false) : 0.0f;
    return r;
}

SC4Percentage cFC4ResidentialSimulator::GetSchoolSystemRating() {
    SC4Percentage r;
    r.percentage = core_ ? fc4_residential_system_rating(core_, true) : 0.0f;
    return r;
}

SC4Percentage cFC4ResidentialSimulator::GetHealthSystemRating() {
    SC4Percentage r;
    r.percentage = core_ ? fc4_residential_system_rating(core_, false) : 0.0f;
    return r;
}

// Slots 14 / 15 (REH §4.11): the number of buildings counted. [choice] A
// null group vector (the original dereferences it) counts nothing.
uint32_t cFC4ResidentialSimulator::GetSchoolSystemTotals(FC4ResidentialQueryData* totals,
                                                        SC4Vector<uint32_t> const* groups) {
    if (totals == nullptr || core_ == nullptr) return 0;
    const Groups g = GroupsOf(groups);
    static const uint32_t kNone = 0;
    fc4_residential_query_data q = ToCore(*totals);
    const uint32_t n = fc4_residential_system_totals(core_, true, g.data ? g.data : &kNone, g.count, &q);
    FromCore(q, *totals);
    return n;
}

uint32_t cFC4ResidentialSimulator::GetHospitalSystemTotals(FC4ResidentialQueryData* totals,
                                                          SC4Vector<uint32_t> const* groups) {
    if (totals == nullptr || core_ == nullptr) return 0;
    const Groups g = GroupsOf(groups);
    static const uint32_t kNone = 0;
    fc4_residential_query_data q = ToCore(*totals);
    const uint32_t n = fc4_residential_system_totals(core_, false, g.data ? g.data : &kNone, g.count, &q);
    FromCore(q, *totals);
    return n;
}

int32_t cFC4ResidentialSimulator::GetPopulation() { return Snapshot().population; }
int32_t cFC4ResidentialSimulator::GetTotalCityEducationUpkeepCost() { return 0; }  // §10.1: always 0
int32_t cFC4ResidentialSimulator::GetTotalCityHealthUpkeepCost() { return 0; }

// Slot 19: [choice] true (§10.1).
bool cFC4ResidentialSimulator::SetOccupantFundingPercentages(cISC4Occupant* occupant, SC4Percentage const& staff,
                                                             SC4Percentage const& vehicle, bool isSchool) {
    if (core_ != nullptr) {
        fc4_residential_set_occupant_funding(core_, TokenOf(occupant), staff.percentage, vehicle.percentage,
                                             isSchool);
    }
    return true;
}

// Slot 20: staff / vehicle in/out (REH §4.2); [choice] true.
bool cFC4ResidentialSimulator::GetOccupantFundingPercentages(cISC4Occupant* occupant, SC4Percentage& staff,
                                                             SC4Percentage& vehicle, bool isSchool) {
    if (core_ != nullptr) {
        fc4_residential_get_occupant_funding(core_, TokenOf(occupant), &staff.percentage, &vehicle.percentage,
                                             isSchool);
    }
    return true;
}

// Slots 21 / 22 (§6.2): a NEW grid per call, owned by the caller; true.
bool cFC4ResidentialSimulator::GetAverageEQGrid(cISC4SimGrid<float>*& grid, float* min, float* max) {
    const uint64_t t = core_ ? fc4_residential_average_grid(core_, true, min, max) : 0;
    grid = reinterpret_cast<GridF32*>(FromToken(t));
    return true;
}

bool cFC4ResidentialSimulator::GetAverageHQGrid(cISC4SimGrid<float>*& grid, float* min, float* max) {
    const uint64_t t = core_ ? fc4_residential_average_grid(core_, false, min, max) : 0;
    grid = reinterpret_cast<GridF32*>(FromToken(t));
    return true;
}

// Slots 23-25 (§6.3): three out-references, each AddRef'd for the caller; 1.
bool cFC4ResidentialSimulator::GetEQGrids(cISC4SimGrid<float>*& g0, cISC4SimGrid<float>*& g1,
                                          cISC4SimGrid<float>*& g2) {
    const fc4_residential_snapshot s = Snapshot();
    HandOutGrid(g0, s.eq[0]);
    HandOutGrid(g1, s.eq[1]);
    HandOutGrid(g2, s.eq[2]);
    return true;
}

bool cFC4ResidentialSimulator::GetHQGrids(cISC4SimGrid<float>*& g0, cISC4SimGrid<float>*& g1,
                                          cISC4SimGrid<float>*& g2) {
    const fc4_residential_snapshot s = Snapshot();
    HandOutGrid(g0, s.hq[0]);
    HandOutGrid(g1, s.hq[1]);
    HandOutGrid(g2, s.hq[2]);
    return true;
}

bool cFC4ResidentialSimulator::GetPopulationGrids(cISC4SimGrid<uint16_t>*& g0, cISC4SimGrid<uint16_t>*& g1,
                                                  cISC4SimGrid<uint16_t>*& g2) {
    const fc4_residential_snapshot s = Snapshot();
    HandOutGrid(g0, s.population_grids[0]);
    HandOutGrid(g1, s.population_grids[1]);
    HandOutGrid(g2, s.population_grids[2]);
    return true;
}

// Slots 26 / 27 (REH §4.11): the record in/out; AL = found.
bool cFC4ResidentialSimulator::GetSchoolQueryData(cISC4Occupant* occupant, FC4ResidentialQueryData* data) {
    if (data == nullptr || core_ == nullptr) return false;
    fc4_residential_query_data q = ToCore(*data);
    const bool ok = fc4_residential_get_query_data(core_, true, TokenOf(occupant), &q);
    FromCore(q, *data);
    return ok;
}

bool cFC4ResidentialSimulator::GetHospitalQueryData(cISC4Occupant* occupant, FC4ResidentialQueryData* data) {
    if (data == nullptr || core_ == nullptr) return false;
    fc4_residential_query_data q = ToCore(*data);
    const bool ok = fc4_residential_get_query_data(core_, false, TokenOf(occupant), &q);
    FromCore(q, *data);
    return ok;
}

// Slot 28: [choice] true (§10.1).
bool cFC4ResidentialSimulator::EstimateCurrentOccupantCapacity(cISC4Occupant* occupant, uint32_t& capacity,
                                                               uint32_t& staff) {
    if (core_ != nullptr) fc4_residential_estimate_capacity(core_, TokenOf(occupant), &capacity, &staff);
    return true;
}

// Slots 29, 34, 42 return a uint8 in AL; a replacement gives the value
// zero-extended in EAX (§10.1). The asm block leaves it in EAX and the
// function has no return statement (MSVC: the value in EAX is the result;
// warning 4035 is expected and silenced).
#pragma warning(push)
#pragma warning(disable : 4035)
uint8_t cFC4ResidentialSimulator::GetCellLifeExpectancy(uint32_t cellX, uint32_t cellZ) {
    const uint32_t v = core_ ? fc4_residential_cell_life_expectancy(core_, static_cast<int32_t>(cellX),
                                                                   static_cast<int32_t>(cellZ))
                             : 0u;
    __asm mov eax, v
}

uint8_t cFC4ResidentialSimulator::GetGlobalLE() {
    const uint32_t v = core_ ? fc4_residential_global_le(core_) : 0u;
    __asm mov eax, v
}

uint8_t cFC4ResidentialSimulator::GetAverageNewAgeByWealth(WealthType wealth) {
    const uint32_t v = core_ ? fc4_residential_new_age_by_wealth(core_, static_cast<uint8_t>(wealth)) : 0u;
    __asm mov eax, v
}
#pragma warning(pop)

// Slots 30 / 31 (§6.4): by value through the hidden result pointer.
SC4Percentage cFC4ResidentialSimulator::GetCellWorkforcePercent(uint32_t cellX, uint32_t cellZ) {
    SC4Percentage r;
    r.percentage = core_ ? fc4_residential_cell_workforce(core_, static_cast<int32_t>(cellX),
                                                          static_cast<int32_t>(cellZ))
                         : 0.0f;
    return r;
}

SC4Percentage cFC4ResidentialSimulator::GetGlobalWorkforcePercent() {
    SC4Percentage r;
    r.percentage = core_ ? fc4_residential_global_workforce(core_) : 0.0f;
    return r;
}

float cFC4ResidentialSimulator::GetGlobalEQ() { return core_ ? fc4_residential_global_quotient(core_, true) : 0.0f; }
float cFC4ResidentialSimulator::GetGlobalHQ() { return core_ ? fc4_residential_global_quotient(core_, false) : 0.0f; }

float cFC4ResidentialSimulator::GetCellEQ(uint32_t cellX, uint32_t cellZ) {
    return core_ ? fc4_residential_cell_quotient(core_, true, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ))
                 : 0.0f;
}

float cFC4ResidentialSimulator::GetCellHQ(uint32_t cellX, uint32_t cellZ) {
    return core_
               ? fc4_residential_cell_quotient(core_, false, static_cast<int32_t>(cellX), static_cast<int32_t>(cellZ))
               : 0.0f;
}

float cFC4ResidentialSimulator::GetCellEQByWealth(uint32_t cellX, uint32_t cellZ, WealthType wealth) {
    return core_ ? fc4_residential_cell_quotient_by_wealth(core_, true, static_cast<int32_t>(cellX),
                                                           static_cast<int32_t>(cellZ), static_cast<uint8_t>(wealth))
                 : 0.0f;
}

float cFC4ResidentialSimulator::GetCellHQByWealth(uint32_t cellX, uint32_t cellZ, WealthType wealth) {
    return core_ ? fc4_residential_cell_quotient_by_wealth(core_, false, static_cast<int32_t>(cellX),
                                                           static_cast<int32_t>(cellZ), static_cast<uint8_t>(wealth))
                 : 0.0f;
}

// Slots 39-41: borrowed (no AddRef).
cISC4SimGrid<float>* cFC4ResidentialSimulator::GetSchoolAverageGradeMap() {
    return reinterpret_cast<GridF32*>(FromToken(Snapshot().school_grade));
}

cISC4SimGrid<float>* cFC4ResidentialSimulator::GetHospitalAverageGradeMap() {
    return reinterpret_cast<GridF32*>(FromToken(Snapshot().hospital_grade));
}

cISC4SimGrid<uint8_t>* cFC4ResidentialSimulator::GetAverageAgeMap() {
    return reinterpret_cast<GridU8*>(FromToken(Snapshot().age));
}

// Slots 43 / 44: every output written (0 when the population is 0).
bool cFC4ResidentialSimulator::GetEQMinAndMaxCellCoords(uint32_t& minX, uint32_t& minZ, uint32_t& maxX,
                                                        uint32_t& maxZ, float& min, float& max) {
    fc4_residential_min_max m{};
    const bool ok = core_ && fc4_residential_quotient_min_max(core_, true, &m);
    minX = m.min_x;
    minZ = m.min_z;
    maxX = m.max_x;
    maxZ = m.max_z;
    min = m.min;
    max = m.max;
    return ok;
}

bool cFC4ResidentialSimulator::GetHQMinAndMaxCellCoords(uint32_t& minX, uint32_t& minZ, uint32_t& maxX,
                                                        uint32_t& maxZ, float& min, float& max) {
    fc4_residential_min_max m{};
    const bool ok = core_ && fc4_residential_quotient_min_max(core_, false, &m);
    minX = m.min_x;
    minZ = m.min_z;
    maxX = m.max_x;
    maxZ = m.max_z;
    min = m.min;
    max = m.max;
    return ok;
}

// Slot 45: outputs untouched when the building is neither a school nor a hospital.
bool cFC4ResidentialSimulator::GetOccupantCoverage(cISC4Occupant* occupant, SC4Percentage const& level,
                                                   float& distance, float& radius) {
    return core_ && fc4_residential_occupant_coverage(core_, TokenOf(occupant), level.percentage, &distance, &radius);
}

int32_t cFC4ResidentialSimulator::GetSchoolBuildingCount() { return Snapshot().school_building_count; }
int32_t cFC4ResidentialSimulator::GetHospitalBuildingCount() { return Snapshot().hospital_building_count; }

uint32_t cFC4ResidentialSimulator::Buildings(uint32_t which, SC4List<cISC4Occupant*>& list,
                                             SC4Vector<uint32_t> const* groups) {
    if (core_ == nullptr) return 0;
    const Groups g = GroupsOf(groups);
    return fc4_residential_buildings(core_, which, g.data, g.count, &AppendToList, &list);
}

// Slots 47 / 49: appended, one reference each; [choice] true (§10.1).
bool cFC4ResidentialSimulator::GetSchoolBuildings(SC4List<cISC4Occupant*>& buildings,
                                                  SC4Vector<uint32_t> const* groups) {
    Buildings(FC4_RESIDENTIAL_LIST_SCHOOL, buildings, groups);
    return true;
}

bool cFC4ResidentialSimulator::GetHospitalBuildings(SC4List<cISC4Occupant*>& buildings,
                                                    SC4Vector<uint32_t> const* groups) {
    Buildings(FC4_RESIDENTIAL_LIST_HOSPITAL, buildings, groups);
    return true;
}

// Slot 61: the health-affecting list, then the other list (§10.1).
void cFC4ResidentialSimulator::GetHealthAffectingBuildings(SC4List<cISC4Occupant*>& list) {
    Buildings(FC4_RESIDENTIAL_LIST_HEALTH_AFFECTING, list, nullptr);
}

int32_t cFC4ResidentialSimulator::GetMaxEQ() { return 200; }  // §10.1
int32_t cFC4ResidentialSimulator::GetMaxHQ() { return 200; }

bool cFC4ResidentialSimulator::GetGlobalAutoBudgetForSchools() { return Snapshot().school_auto_budget != 0; }
bool cFC4ResidentialSimulator::GetGlobalAutoBudgetForHospitals() { return Snapshot().hospital_auto_budget != 0; }

bool cFC4ResidentialSimulator::SetGlobalAutoBudgetForSchools(bool enable) {
    return core_ ? fc4_residential_set_global_auto_budget(core_, true, enable) : enable;
}

bool cFC4ResidentialSimulator::SetGlobalAutoBudgetForHospitals(bool enable) {
    return core_ ? fc4_residential_set_global_auto_budget(core_, false, enable) : enable;
}

bool cFC4ResidentialSimulator::GetAutoBudget(cISC4Occupant* occupant) {
    return core_ && fc4_residential_auto_budget(core_, TokenOf(occupant));
}

bool cFC4ResidentialSimulator::SetAutoBudget(cISC4Occupant* occupant, bool enable) {
    return core_ && fc4_residential_set_auto_budget(core_, TokenOf(occupant), enable);
}

bool cFC4ResidentialSimulator::EstimateIdealFunding(cISC4Occupant* occupant, SC4Percentage& funding) {
    return core_ && fc4_residential_ideal_funding(core_, TokenOf(occupant), &funding.percentage);
}

void cFC4ResidentialSimulator::ToggleTractTracking(int32_t, int32_t) {}  // slot 59: empty
void cFC4ResidentialSimulator::TrackTracts() {}                            // slot 62: empty
void cFC4ResidentialSimulator::OutputTractInfo(int32_t, int32_t) {}        // slot 63: empty

// ---- messages (§4.3) --------------------------------------------------------------------------------

bool cFC4ResidentialSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    const uint32_t type = pMessage->GetType();
    uint32_t d[3] = {};
    uint64_t void1 = 0;
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    if (s != nullptr) {
        d[0] = static_cast<uint32_t>(s->GetData1());
        d[1] = static_cast<uint32_t>(s->GetData2());
        d[2] = static_cast<uint32_t>(s->GetData3());
        void1 = TokenOf(s->GetVoid1());  // the occupant of 0x99EF1142 / 0x99EF1143 (§4.3)
    }
    // Off the main thread or during a call the core queues the message (and
    // holds its occupant) and answers true.
    const bool r = fc4_residential_do_message(core_, type, d[0], d[1], d[2], void1);
    Drop(s);
    if (GetCurrentThreadId() != mainTid_) {
        ++offMainMessages_;
        return r;
    }
    if (type == kMsgSimNewDay) {
        lastJulian_ = static_cast<int32_t>(d[0]);
        ++days_;
        if (days_ == 1 && months_ == 0) LogMonth();  // the status line from day one
        fc4_residential_snapshot snap = Snapshot();
        if (snap.state == kStateIdle && lastState_ != kStateIdle && lastState_ != 0xFFFFFFFFu) {
            ++cycles_;
            LogCycle();
        }
        lastState_ = snap.state;
    } else if (type == kMsgSimNewMonth) {
        ++months_;
        LogMonth();
        lastState_ = Snapshot().state;
    }
    return r;
}

// ---- persistence (§9) -------------------------------------------------------------------------------

bool cFC4ResidentialSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("ResidentialSimulator::Write");
    fc4::OStreamAdapter a(s);
    const bool ok = core_ != nullptr && fc4_residential_write(core_, a.Get());
    fc4::Log("residential Write -> %d", ok ? 1 : 0);
    return ok;
}

// Runs before Init: the core sizes nothing from the city here (§9.2).
bool cFC4ResidentialSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("ResidentialSimulator::Read");
    fc4::IStreamAdapter a(s);
    const bool ok = core_ != nullptr && fc4_residential_read(core_, a.Get());
    fc4::Log("residential Read -> %d", ok ? 1 : 0);
    LogState("Read");
    return ok;
}

uint32_t cFC4ResidentialSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4ResidentialSimulator; }
