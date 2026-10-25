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

#include "FC4DisasterHost.h"
#include "FC4DisasterFire.h"
#include "FC4AutomataInterfaces.h"
#include "FC4HarnessLogic.h"

#include "FC4CoreProps.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "cIGZCOM.h"
#include "SC4List.h"
#include "SC4Point.h"
#include "SC4Rect.h"
#include "cIGZMessageServer2.h"
#include "cISC4City.h"
#include "cISC4FireProtectionSimulator.h"
#include "cISC4FlammabilitySimulator.h"
#include "cISC4HistoryWarehouse.h"
#include "cISC4LotManager.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4PoliceSimulator.h"
#include "cISC4SimGrid.h"
#include "cISC4TutorialSystem.h"
#include "cFC4Tutorial.h"  // the glue15 lane: fc4::TutorialIsTutorial
#include "cISC4WeatherSimulator.h"
#include "cISC4AutomataControllerManager.h"
#include "cISC4AutomataManager.h"
#include "cISC4Automaton.h"
#include "cISC4FireOccupant.h"
#include "cISCPropertyHolder.h"
#include "cS3DVector2.h"
#include "cS3DVector3.h"
#include "cISTETerrain.h"
#include "SC4String.h"
#include "cGZPersistResourceKey.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <algorithm>
#include <cstring>

namespace fc4 {
namespace {

// disasters.md §1, §5.3, §5.4, §5.11, §6.3; flora.md §6.4.1; network-manager.md §10.
constexpr uint32_t kTypeFlora = 0x74758926;
constexpr uint32_t kTypeBuilding = 0x278128A0;
constexpr uint32_t kTypeProp = 0xA823821E;
constexpr uint32_t kTypeNetwork = 0xC772BF98;
constexpr uint32_t kIID_cISC4NetworkOccupant = 0xA821EF94;
constexpr uint32_t kPropBurning = 0x69DBEBE1;
constexpr uint32_t kPropFlammability = 0x29244DB5;
// The fire occupant (§5.11, DS26): class 0x29D25F1D answering the
// fire-occupant interface 0x69D25C60 (the primary vtable) itself.
constexpr uint32_t kCLSID_cSC4FireOccupant = 0x29D25F1D;
constexpr uint32_t kIID_cISC4FireOccupant = 0x69D25C60;
// The riot leader (§6.4, automata.md §8.1, DS27): the pedestrian
// automata group, its RiotLeader state and the state the end sets.
constexpr uint32_t kRiotAutomataGroup = 0x4107;
constexpr uint32_t kRiotLeaderState = 8;
constexpr uint32_t kRiotIdleState = 1;
constexpr uint32_t kTrafficNetworkFlags = 0x1F4F;  // road .. ground highway (flora §6.4.1)
// Road, street and avenue (network types 0, 3, 6 as flag bits; disasters.md §6.4 step 1 [inferred]).
constexpr uint32_t kRiotRoadFlags = (1u << 0) | (1u << 3) | (1u << 6);
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0x29B6C670;
constexpr uint32_t kTuningFlame = 0xC9B6B73D;
constexpr uint32_t kTuningRiot = 0x4A38206D;
constexpr uint32_t kMsgIgnitions = 0x49D15B73;
constexpr uint32_t kMsgFireStartedEnded = 0x49E3DB1D;

DisasterHostContext* C(void* u) { return static_cast<DisasterHostContext*>(u); }
cISC4Occupant* Occ(uint64_t t) {
    return (t == 0 || IsSynthetic(t)) ? nullptr : reinterpret_cast<cISC4Occupant*>(static_cast<uintptr_t>(t));
}

// A write the mode does not make: shadow = recorded; live = recorded and the
// first one of each kind logged (the spec leaves the call open).
void Wrote(void* u, const char* what) {
    DisasterHostContext* c = C(u);
    ++c->writes[what];
    if (!c->shadow && c->notWired.insert(what).second) {
        Log("disaster host: %s is not wired (the interface it needs is not specified; the write is not made)", what);
    }
}

uint64_t NowRealMs() {
    static LARGE_INTEGER freq{};
    if (freq.QuadPart == 0) QueryPerformanceFrequency(&freq);
    LARGE_INTEGER now{};
    QueryPerformanceCounter(&now);
    return static_cast<uint64_t>(now.QuadPart / (freq.QuadPart / 1000));
}

cFC4ISC4Simulator* Sim(void* u) {
    cISC4City* c = C(u)->city;
    return c ? AsGame(c->GetSimulator()) : nullptr;  // city slot 50, borrowed
}

int32_t Clamp(int32_t v, int32_t lo, int32_t hi) { return v < lo ? lo : (v > hi ? hi : v); }

// ---- clock, pause, save ------------------------------------------------------------------------

uint64_t NowMs(void* u) { return C(u)->shadow ? C(u)->clockMs : NowRealMs(); }

void EmergencyPause(void* u) {
    if (C(u)->shadow) {
        ++C(u)->pauses;
        return;
    }
    if (cFC4ISC4Simulator* s = Sim(u)) s->EmergencyPause();  // slot 11
}
void EmergencyResume(void* u) {
    if (C(u)->shadow) {
        --C(u)->pauses;
        return;
    }
    if (cFC4ISC4Simulator* s = Sim(u)) s->EmergencyResume();  // slot 14
}
void DisableSave(void* u) {
    if (C(u)->shadow) {
        ++C(u)->saveOff;
        return;
    }
    if (C(u)->city) C(u)->city->DisableSave();  // city slot 104
}
void EnableSave(void* u) {
    if (C(u)->shadow) {
        --C(u)->saveOff;
        return;
    }
    if (C(u)->city) C(u)->city->EnableSave();  // city slot 103
}
bool SimPaused(void* u) {
    if (C(u)->shadow) return C(u)->pausedSample;
    cFC4ISC4Simulator* s = Sim(u);
    return s != nullptr && s->IsPaused();  // slot 15
}
bool SimEmergencyPaused(void* u) {
    if (C(u)->shadow) return C(u)->emergencySample;
    cFC4ISC4Simulator* s = Sim(u);
    return s != nullptr && s->IsEmergencyPaused();  // slot 17
}
bool CityEstablished(void* u) { return C(u)->city != nullptr && C(u)->city->GetEstablished(); }  // slot 22
void CityCells(void* u, uint32_t* x, uint32_t* z) {
    cISC4City* c = C(u)->city;
    if (x) *x = c ? c->CellCountX() : 0;  // slots 93, 94
    if (z) *z = c ? c->CellCountZ() : 0;
}
float CellWidth(void*) { return 16.0f; }  // city slot 91 (§5.6)

// ---- messages, ticks, history, references ------------------------------------------------------

void Post(void* u, const fc4_disaster_post* p) {
    if (p == nullptr) return;
    DisasterHostContext* c = C(u);
    if (c->postOverride) { c->postOverride(*p); return; }
    ++c->posts[p->msg_type];
    if (p->msg_type == kMsgIgnitions) {
        c->postData1Sums[p->msg_type] += static_cast<uint64_t>(p->data1);
        c->postData2Sums[p->msg_type] += p->data2;
    } else if (p->msg_type == kMsgFireStartedEnded) {
        c->postData2Sums[p->msg_type] += p->data2;
    }
    if (c->shadow) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    // [question Q30] data1 carries a token for 0x49E3DB1D (the fire
    // occupant); in live mode our tokens are the game's pointers.
    cFC4Message2Standard* m = FC4MakeMessage(p->msg_type, static_cast<intptr_t>(p->data1),
                                             static_cast<intptr_t>(p->data2), static_cast<intptr_t>(p->data3));
    m->SetData4(static_cast<intptr_t>(p->data4));
    ms->MessagePost(m, false);
    m->Release();
}
void Subscribe(void* u, uint32_t kind, uint32_t id, uint32_t type) {
    C(u)->subscriptions.insert(std::make_tuple(kind, id, type));
}
void Unsubscribe(void* u, uint32_t kind, uint32_t id, uint32_t type) {
    C(u)->subscriptions.erase(std::make_tuple(kind, id, type));
}
void RegisterTick(void* u, uint32_t instance, uint32_t) { C(u)->ticks.insert(instance); }
void UnregisterTick(void* u, uint32_t instance) { C(u)->ticks.erase(instance); }
void RecordHistory(void* u, uint32_t series, int32_t value) {
    C(u)->history[series] += value;
    if (C(u)->shadow || C(u)->city == nullptr) return;
    if (cISC4HistoryWarehouse* h = C(u)->city->GetHistoryWarehouse()) h->RecordHistoryData(series, value);  // slot 12
}
void AddRef(void* u, uint64_t t) {
    cISC4Occupant* o = Occ(t);
    if (o == nullptr) return;
    FC4_ADDREF(o);
    ++C(u)->refs[t];
}
void Release(void* u, uint64_t t) {
    cISC4Occupant* o = Occ(t);
    if (o == nullptr) return;
    auto it = C(u)->refs.find(t);
    if (it == C(u)->refs.end() || it->second <= 0) return;  // never one the core did not take
    if (--it->second == 0) C(u)->refs.erase(it);
    FC4_RELEASE(o);
}

// ---- camera, dispatch-unit counts --------------------------------------------------------------

bool FollowDisasters(void*) { return false; }  // [question Q31] the preference byte (§2.7) is not reachable
void GoToLocation(void* u, int32_t, int32_t, uint32_t) { Wrote(u, "go_to_location"); }
// The dispatch effectiveness divisor (§2.5): GetTotalDispatchCount, a read in
// either mode.
uint32_t FireDispatchCount(void* u) {
    cISC4City* c = C(u)->city;
    cISC4FireProtectionSimulator* f = c ? c->GetFireProtectionSimulator() : nullptr;  // city slot 57
    const int32_t n = f ? AsGame(f)->GetTotalDispatchCount() : 0;                    // slot 25
    return n > 0 ? static_cast<uint32_t>(n) : 0u;
}
uint32_t PoliceDispatchCount(void* u) {
    cISC4City* c = C(u)->city;
    cISC4PoliceSimulator* p = c ? c->GetPoliceSimulator() : nullptr;  // city slot 65
    const int32_t n = p ? AsGame(p)->GetTotalDispatchCount() : 0;     // slot 26
    return n > 0 ? static_cast<uint32_t>(n) : 0u;
}

// ---- occupants ---------------------------------------------------------------------------------

uint32_t BuildingTypeFlags(void*, uint64_t) { return 0; }  // [question Q31] the BDS flag read
// The mirror's fire occupants are synthetic tokens: their cells and box are
// the burning target's (FC4DisasterHost.h, `burning`).
uint64_t FireTargetOf(void* u, uint64_t t) {
    if (!IsSynthetic(t)) return t;
    for (const auto& [target, fire] : C(u)->burning) {
        if (fire == t) return target;
    }
    return t;
}
uint32_t OccupantType(void*, uint64_t t) {
    cISC4Occupant* o = Occ(t);
    return o ? static_cast<uint32_t>(o->GetType()) : 0;  // slot 7
}
bool InManager(cISC4Occupant* o) { return o != nullptr && o->IsFlagSet(1); }  // slot 32 (§5.10 step 1)
bool OccupantInManager(void*, uint64_t t) { return InManager(Occ(t)); }
bool OccupantCell(void* u, uint64_t t, int32_t* x, int32_t* z) {
    int32_t a = 0, b = 0;
    if (!DisasterOccupantCell(C(u)->city, Occ(t), &a, &b)) return false;
    if (x) *x = a;
    if (z) *z = b;
    return true;
}
void OccupantCellRect(void* u, uint64_t t, fc4_cell_rect* out) {
    cISC4Occupant* o = Occ(FireTargetOf(u, t));
    if (out == nullptr || o == nullptr) return;
    SC4Rect<long> r;
    o->GetBoundingCityCells(r);  // slot 11
    out->x1 = static_cast<int32_t>(r.topLeftX);
    out->z1 = static_cast<int32_t>(r.topLeftY);
    out->x2 = static_cast<int32_t>(r.bottomRightX);
    out->z2 = static_cast<int32_t>(r.bottomRightY);
}
void OccupantPosition(void*, uint64_t t, float* x, float* z) {
    cISC4Occupant* o = Occ(t);
    cS3DVector3 v;
    if (o) o->GetPosition(&v);  // slot 8
    if (x) *x = v.fX;
    if (z) *z = v.fZ;
}
void Box(cISC4Occupant* o, cS3DVector3* a, cS3DVector3* b) {
    if (o) o->GetBoundingBox(a, b);  // slot 10
}
void OccupantBBox(void* u, uint64_t t, fc4_world_box* out) {
    if (out == nullptr) return;
    cS3DVector3 a, b;
    Box(Occ(FireTargetOf(u, t)), &a, &b);
    out->x0 = a.fX;
    out->z0 = a.fZ;
    out->x1 = b.fX;
    out->z1 = b.fZ;
}
// [choice] top = the box's x by z extent; side = its height by (x + z) extent
// (§5.11 names the two areas only; the flame counts are not compared).
void OccupantBBoxAreas(void*, uint64_t t, float* top, float* side) {
    cS3DVector3 a, b;
    Box(Occ(t), &a, &b);
    const float dx = b.fX - a.fX, dy = b.fY - a.fY, dz = b.fZ - a.fZ;
    if (top) *top = dx * dz;
    if (side) *side = dy * (dx + dz);
}
cISCPropertyHolder* Holder(uint64_t t) {
    cISC4Occupant* o = Occ(t);
    return o ? o->AsPropertyHolder() : nullptr;  // slot 6, borrowed
}
bool PropU32(void*, uint64_t t, uint32_t id, uint32_t* out) {
    std::vector<uint32_t> v;
    cISCPropertyHolder* h = Holder(t);
    if (h == nullptr || !props::GetU32s(h, id, v) || v.empty()) return false;
    if (out) *out = v[0];
    return true;
}
bool PropI64(void*, uint64_t t, uint32_t id, int64_t* out) {
    std::vector<int64_t> v;
    cISCPropertyHolder* h = Holder(t);
    if (h == nullptr || !props::GetI64s(h, id, v) || v.empty()) return false;
    if (out) *out = v[0];
    return true;
}
bool PropF32(void*, uint64_t t, uint32_t id, float* out) {
    std::vector<float> v;
    cISCPropertyHolder* h = Holder(t);
    if (h == nullptr || !props::GetF32s(h, id, v) || v.empty()) return false;
    if (out) *out = v[0];
    return true;
}
bool PropBool(void* u, uint64_t t, uint32_t id, bool* out) {
    if (id == kPropBurning && C(u)->shadow) {
        // The mirror's own fires (FC4DisasterHost.h): present, true or false.
        if (out) *out = C(u)->burning.count(t) != 0;
        return true;
    }
    uint32_t v = 0;
    if (!PropU32(u, t, id, &v)) return false;
    if (out) *out = v != 0;
    return true;
}
void OccupantsInRect(void* u, const fc4_cell_rect* r, fc4_token_callback cb, void* ctx) {
    if (r == nullptr || cb == nullptr) return;
    for (uint32_t type : {kTypeBuilding, kTypeFlora, kTypeProp}) {
        auto list = DisasterOccupantsIn(C(u)->city, r->x1, r->z1, r->x2, r->z2, type, C(u));
        if (!list) continue;
        for (cISC4Occupant* o : *list) {
            if (o != nullptr) cb(ctx, TokenOf(o));
        }
    }
}
void RemoveOccupant(void* u, uint64_t t) {
    if (C(u)->shadow) {
        Wrote(u, "remove_occupant");
        return;
    }
    cISC4OccupantManager* om = C(u)->city ? C(u)->city->GetOccupantManager() : nullptr;
    if (om && Occ(t)) om->RemoveOccupant(Occ(t), true, 0);  // slot 21 (§5.10 ToastOccupant step 3)
}
uint64_t OccupantLot(void* u, uint64_t t) {
    cISC4LotManager* lm = C(u)->city ? C(u)->city->GetLotManager() : nullptr;
    return (lm && Occ(t)) ? TokenOf(AsGame(lm)->GetOccupantLot(Occ(t))) : 0;  // slot 14, borrowed
}
void SetLotCondition(void* u, uint64_t lot, uint32_t condition) {
    C(u)->lotConditions[lot] = condition;
    if (C(u)->shadow || lot == 0) return;
    AsGame(reinterpret_cast<cISC4Lot*>(static_cast<uintptr_t>(lot)))->SetCondition(condition);  // lot slot 36
}

// ---- fire --------------------------------------------------------------------------------------

uint8_t FireCoverage(void* u, uint64_t t) { return DisasterFireCoverage(C(u)->city, Occ(t)); }
uint8_t FlammabilityAt(void* u, int32_t x, int32_t z) {
    cISC4City* c = C(u)->city;
    cISC4FlammabilitySimulator* f = c ? c->GetFlammabilitySimulator() : nullptr;  // slot 58
    cISC4SimGrid<uint8_t>* g = f ? f->GetFlammabilityGrid() : nullptr;
    return g ? g->GetCellValue(x, z) : uint8_t{0};
}
// §5.4 step 7 (DS26): the fire occupant is created through GZCOM, Init'ed,
// given its target and its fire ID, then inserted in the occupant manager.
// `SetTarget` (§5.11) does the rest itself: max stage, the flame counts from
// the static D7/D8 curves, the target's BURNING property, `SetRemovalFlags(2)`
// and, for a building, the "fire_occupant" controllers (automata.md §8.2).
// The flame counts the core passes are the same curves' values; the class
// computes them from the target, so they are not written here [compat].
uint64_t CreateFireOccupant(void* u, uint32_t, uint64_t target, uint32_t fire_id, uint32_t, uint32_t) {
    DisasterHostContext* c = C(u);
    if (c->shadow) {
        const uint64_t fire = kSyntheticToken | c->nextSynthetic++;
        c->burning[target] = fire;
        ++c->writes["create_fire_occupant"];
        return fire;
    }
    cISC4City* city = c->city;
    cISC4Occupant* t = Occ(target);
    if (city == nullptr || t == nullptr) return 0;
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (com == nullptr || !com->GetClassObject(kCLSID_cSC4FireOccupant, kIID_cISC4FireOccupant, &p) ||
        p == nullptr) {
        Wrote(u, "create_fire_occupant (GZCOM has no class 0x29D25F1D)");
        return 0;
    }
    auto* storage = static_cast<cISC4FireOccupant*>(p);
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(storage), "GetClassObject");
    auto* fo = new DisasterFire(storage, [c](cISC4Occupant* target, bool burning) {
        // disasters.md 5.11: retain the target, set BURNING and the removal
        // lock, and attach controllers; undo all four at Shutdown.
        if (burning) FC4_ADDREF(target);
        const uint8_t value = burning ? 1 : 0;
        props::SetTyped(target->AsPropertyHolder(), kPropBurning, 0x01, false, &value, 1);
        if (burning) target->SetRemovalFlags(2);
        else target->UnsetRemovalFlags(2);
        auto* cm = c->city ? c->city->GetAutomataControllerManager() : nullptr;
        if (cm && static_cast<uint32_t>(target->GetType()) == kTypeBuilding) {
            if (burning) AsGame(cm)->AddControllersFromOccupantGroupName(SC4String("fire_occupant"), target, nullptr);
            else AsGame(cm)->RemoveControllers(target);
        }
        if (!burning) FC4_RELEASE_LAST_HOLDER(target);
    }, [storage] { FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(storage)); });
    cISC4Occupant* occ = fo->AsOccupant();
    const uint64_t token = TokenOf(occ);
    fo->hose = [c, token](int32_t delta) { return c->hoseFire && c->hoseFire(token, delta); };
    fo->post = [](cISC4Occupant* occupant, uint32_t type, uint32_t data) {
        // flora.md 9.1: the counted object and void1 must have the same
        // identity as the pointer inserted into the occupant manager.
        cIGZMessageServer2Ptr ms;
        if (!ms) return;
        auto* message = FC4MakeMessage(type, 0, occupant->IsFlagSet(4) ? 1 : 0, occupant->GetType());
        message->SetVoid1(occupant);
        message->SetIGZUnknown(occupant);
        message->SetData4(data);
        ms->MessagePost(message, false);
        message->Release();
    };
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(static_cast<cISC4FireOccupant*>(fo)), "create fire adapter");
    occ->Init();                            // §5.4 step 7.1
    fo->SetTarget(t);                       // §5.4 step 7.2 (§5.11)
    fo->SetFamily(fire_id);                 // slot 14 SetFireID (§5.11)
    if (cISC4OccupantManager* om = city->GetOccupantManager()) {
        om->InsertOccupant(occ, 0);  // slot 20 (§5.4 step 7.3); the manager takes its own reference
    }
    c->fireOccupants[token] = fo;  // our create reference, dropped by remove
    Log("disasters: Rust fire occupant created target=%p fire_id=%u (native static curves bypassed)",
        static_cast<void*>(t), fire_id);
    return token;
}
void RemoveFireOccupant(void* u, uint64_t fire, uint64_t target) {
    DisasterHostContext* c = C(u);
    if (c->shadow) {
        c->burning.erase(target);
        Wrote(u, "remove_fire_occupant");
        return;
    }
    // §5.10 KillFire step 4: RemoveOccupant with shutdown, so the fire
    // occupant's Shutdown runs (§5.11: clears BURNING, UnsetRemovalFlags(2),
    // RemoveControllers and releases the target).
    auto it = c->fireOccupants.find(fire);
    if (it == c->fireOccupants.end()) return;
    cISC4FireOccupant* fo = it->second;
    c->fireOccupants.erase(it);
    static_cast<DisasterFire*>(fo)->hose = nullptr;
    if (cISC4OccupantManager* om = c->city ? c->city->GetOccupantManager() : nullptr) {
        om->RemoveOccupant(fo->AsOccupant(), true, 0);  // slot 21
    }
    FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(fo));
}
void AddFireAura(void* u, uint64_t) { Wrote(u, "add_fire_aura"); }
bool TutorialNoBurnDown(void* u) {
    cISC4City* c = C(u)->city;
    cISC4TutorialSystem* t = c ? c->GetTutorialSystem() : nullptr;  // city slot 82
    // Slot 8, or fc4_tutorial_is_tutorial when the system is ours (enable = tutorial).
    return fc4::TutorialIsTutorial(t);
}
void Wind(void* u, float* x, float* z) {
    cISC4City* c = C(u)->city;
    cISC4WeatherSimulator* w = c ? c->GetWeatherSimulator() : nullptr;
    cS3DVector2 v;
    if (w != nullptr) AsGame(w)->GetAmbientWind(v);  // slot 22: the vector (§5.10 step 9)
    if (x) *x = w ? v.fX : 0.0f;
    if (z) *z = w ? v.fY : 0.0f;
}
void PlayEffect(void* u, const char*, uint64_t) { Wrote(u, "play_effect"); }
void PlayEffectAt(void* u, const char*, float, float) { Wrote(u, "play_effect_at"); }
void BurnFlora(void* u, uint64_t) { Wrote(u, "burn_flora"); }
void DemolishLot(void* u, uint64_t, uint64_t, uint32_t) { Wrote(u, "demolish_lot"); }
void SpewRadiation(void* u, uint64_t, float, float) { Wrote(u, "spew_radiation"); }

// ---- riot --------------------------------------------------------------------------------------

uint8_t PoliceCoverage(void* u, int32_t x, int32_t z) { return DisasterPoliceCoverage(C(u)->city, x, z); }
bool IsRoadCell(void* u, int32_t x, int32_t z) { return DisasterIsRoadCell(C(u)->city, x, z); }
void CellCorner(void* u, int32_t x, int32_t z, float* wx, float* wz) {
    float a = 0, b = 0;
    if (C(u)->city) C(u)->city->CellCornerToPosition(x, z, a, b);  // slot 96
    if (wx) *wx = a;
    if (wz) *wz = b;
}
// §6.4 step 3 and automata.md §8.1 (DS27): a pedestrian automaton of group
// 0x4107 at (x, z) on the terrain, AddRef'd for the leader record, then
// `SetState(8, null, -1.0, true, 0, 0, null)` (the RiotLeader state) and
// `SetVisible(true)`. The core has already found the road cell and drawn the
// in-cell offset; the height here is the terrain's at (x, z) (§6.4 step 3.1).
uint64_t CreateRiotLeader(void* u, float x, float z) {
    DisasterHostContext* c = C(u);
    if (c->shadow) {
        ++c->writes["create_riot_leader"];
        if (c->leaders.empty()) {
            ++c->leadersMissing;
            return kSyntheticToken | c->nextSynthetic++;
        }
        const uint64_t a = c->leaders.front();
        c->leaders.pop_front();
        return a;
    }
    cISC4City* city = c->city;
    if (city == nullptr) return 0;
    cISC4AutomataManager* pm = city->GetPedestrianManager();  // city slot 44
    if (pm == nullptr) return 0;
    const float y = city->GetTerrain() ? city->GetTerrain()->GetAltitude(x, z) : 0.0f;  // slot 13
    const cS3DVector3 pos(x, y, z);
    const cGZPersistResourceKey key;  // the zero model key (§7.2)
    cISC4Automaton* a = pm->CreateAutomaton(kRiotAutomataGroup, pos, 2, key, nullptr, 0xFF, nullptr);
    if (a == nullptr) {
        Wrote(u, "create_riot_leader (CreateAutomaton refused)");
        return 0;
    }
    FC4_ADDREF(a);
    a->SetState(kRiotLeaderState, nullptr, -1.0f, true, 0, 0, nullptr);  // slot 51
    a->SetVisible(true);                                                // slot 9
    const uint64_t token = TokenOf(a);
    c->riotLeaders[token] = a;
    return token;
}
uint64_t NetworkOccupantAt(void* u, int32_t x, int32_t z) { return DisasterNetworkOccupantAt(C(u)->city, x, z); }
void UnsetRemovalFlags(void* u, uint64_t t, uint32_t flags) {
    if (flags == 2) C(u)->locked.insert(t);
    if (C(u)->shadow) {
        ++C(u)->writes["unset_removal_flags"];
        return;
    }
    if (cISC4Occupant* o = Occ(t)) o->UnsetRemovalFlags(flags);  // slot 13
}
void SetRemovalFlags(void* u, uint64_t t, uint32_t flags) {
    if (flags == 2) C(u)->locked.erase(t);
    if (C(u)->shadow) {
        ++C(u)->writes["set_removal_flags"];
        return;
    }
    if (cISC4Occupant* o = Occ(t)) o->SetRemovalFlags(flags);  // slot 12
}
bool AutomatonCell(void* u, uint64_t t, int32_t* x, int32_t* z) {
    int32_t a = 0, b = 0;
    if (!DisasterOccupantCell(C(u)->city, Occ(t), &a, &b)) return false;
    if (x) *x = a;
    if (z) *z = b;
    return true;
}
// [question] §8.1's "Riot1".."RiotN" removal is automaton slot 43
// `RemoveRegEffect` (one name), but the add (slot 41 `AddRegEffect`) takes two
// names and §8.1 names only one ("Riot" + stage + 1). Not made; counted.
void SetRiotEffect(void* u, uint64_t, int32_t) { Wrote(u, "set_riot_effect"); }
// §6.5 step 4 and automata.md §8.1 "Stage rise": the template name
// `riot_leader_<stage>` through controller manager slot 5.
void SendRiotMessage(void* u, uint64_t a, int32_t stage) {
    DisasterHostContext* c = C(u);
    if (c->shadow) {
        Wrote(u, "send_riot_message");
        return;
    }
    auto it = c->riotLeaders.find(a);
    if (it == c->riotLeaders.end()) return;
    char name[32];
    std::snprintf(name, sizeof(name), "riot_leader_%d", stage);
    const SC4String tmpl(name);
    if (cISC4AutomataControllerManager* cm = c->city ? c->city->GetAutomataControllerManager() : nullptr) {
        cm->AddControllerFromTemplateName(it->second->AsOccupant(), tmpl, nullptr);  // slot 5
    }
}
// §6.5 step 5 and automata.md §8.1 "End": the SetState(1, ...) call is made,
// but it is a **no-op against a `final` leader** (the RiotLeader state was
// pushed final=1 at the start), so nothing here relies on it to end the
// leader: RemoveControllers and the release below are what end it.
void EndRiotLeader(void* u, uint64_t a) {
    DisasterHostContext* c = C(u);
    if (c->shadow) {
        Wrote(u, "end_riot_leader");
        return;
    }
    auto it = c->riotLeaders.find(a);
    if (it == c->riotLeaders.end()) return;
    cISC4Automaton* aut = it->second;
    c->riotLeaders.erase(it);
    aut->SetState(kRiotIdleState, nullptr, -1.0f, false, 0, 0, nullptr);  // slot 51 (no-op; see above)
    if (cISC4AutomataControllerManager* cm = c->city ? c->city->GetAutomataControllerManager() : nullptr) {
        cm->RemoveControllers(aut->AsOccupant());  // slot 14
    }
    FC4_RELEASE(aut);  // the leader record's reference (§6.4 step 3.2)
}
// A living leader walks the road under it (§6.5 step 6.2) from the pedestrian
// manager's own animation tick once it is in that manager; the riot's step
// does not drive it. Nothing to do here in live mode.
void RiotLeaderTick(void* u, uint64_t) {
    if (!C(u)->shadow) return;
    Wrote(u, "riot_leader_tick");
}

// ---- toxic spill, terrain, pipes, volcano (not mirrored; Q29) -----------------------------------

uint64_t CreateToxic(void* u, uint32_t, int32_t, int32_t, int32_t) {
    Wrote(u, "create_toxic_occupant");
    return 0;
}
void SetToxicAmount(void* u, uint64_t, int32_t) { Wrote(u, "set_toxic_amount"); }
void RemoveToxic(void* u, uint64_t) { Wrote(u, "remove_toxic_occupant"); }
void TerrainBrush(void* u, uint32_t, float, float, float, float) { Wrote(u, "apply_terrain_brush"); }
void RedrawTerrain(void* u, const fc4_cell_rect*) { Wrote(u, "redraw_terrain"); }
void EndLongTerrain(void* u) { Wrote(u, "end_long_terrain_operation"); }
void StopTerrainTextures(void* u) { Wrote(u, "stop_terrain_texture_updates"); }
void PlaceRepulsor(void* u, float, float) { Wrote(u, "place_repulsor"); }
uint64_t FindWateredPipe(void*, int32_t, int32_t) { return 0; }
void PipeCell(void*, uint64_t, int32_t* x, int32_t* z) {
    if (x) *x = 0;
    if (z) *z = 0;
}
uint32_t ViewLevel(void*) { return 0; }
bool CreateLeakEffect(void* u, uint64_t, uint32_t) {
    Wrote(u, "create_leak_effect");
    return false;
}
size_t ConnectedPipeCells(void*, uint64_t, int32_t*, size_t) { return 0; }
void DemolishPipe(void* u, uint64_t, uint32_t) { Wrote(u, "demolish_pipe"); }
void VolcanoStartDeformation(void* u, uint32_t, float, float) { Wrote(u, "volcano_start_deformation"); }
void VolcanoStartLava(void* u, uint32_t, float, float, float) { Wrote(u, "volcano_start_lava"); }
void VolcanoLavaRates(void* u, uint32_t, float, float) { Wrote(u, "volcano_lava_rates"); }
void VolcanoEndLava(void* u, uint32_t) { Wrote(u, "volcano_end_lava"); }

}  // namespace

// ---- shared helpers ----------------------------------------------------------------------------

std::shared_ptr<SC4List<cISC4Occupant*>> DisasterOccupantsIn(cISC4City* city, int32_t x0, int32_t z0, int32_t x1,
                                                             int32_t z1, uint32_t type, DisasterHostContext* holdIn) {
    cISC4OccupantManager* om = city ? city->GetOccupantManager() : nullptr;  // city slot 34, borrowed
    if (om == nullptr) return nullptr;
    auto list = std::make_shared<SC4List<cISC4Occupant*>>();
    const SC4Point<int> mn(x0, z0);
    const SC4Point<int> mx(x1, z1);
    om->GetOccupantsByStandardCityCells(*list, &mn, &mx, type, 0xFFFFFFFFu);  // slot 41, one reference each
    if (holdIn != nullptr) holdIn->held.push_back(list);
    return list;
}

static SC4Rect<long> ClampedCityCells(cISC4City* city, cISC4Occupant* o);

// §5.4 step 4.1 over the clamped rectangle. simgrid.md §4.1 / §8 item 2: grid
// slot 8's last two parameters are a width and a height (the SDK's
// "nBottomRightX/Z" names are wrong), so the inclusive corners become
// w = x1 - x0 + 1, h = z1 - z0 + 1 (disasters.md §5.4: "rect=56,86..56,87 is a
// 1x2 cell rectangle"). Passing the corners averaged a far larger area.
static uint8_t FireCoverageOver(cISC4City* city, const SC4Rect<long>& r) {
    cISC4FireProtectionSimulator* f = city->GetFireProtectionSimulator();  // city slot 57
    cISC4SimGrid<uint8_t>* g = f ? AsGame(f)->GetFireProtectionGrid() : nullptr;  // slot 5
    const harness::GridCellRectArgs a =
        harness::CellRectArgs(static_cast<int32_t>(r.topLeftX), static_cast<int32_t>(r.topLeftY),
                              static_cast<int32_t>(r.bottomRightX), static_cast<int32_t>(r.bottomRightY));
    // An empty rectangle (no overlap with the city) reads 0 here: the spec does
    // not say what the original's average returns for w x h = 0 (question DS41).
    if (g == nullptr || a.empty) return 0;
    return g->GetAverageValueInCellRect(a.x, a.z, a.w, a.h);
}

uint8_t DisasterFireCoverage(cISC4City* city, cISC4Occupant* o) {
    if (city == nullptr || o == nullptr) return 0;
    return FireCoverageOver(city, ClampedCityCells(city, o));
}

// §5.4 step 4.1's clamped cell rectangle, shared with the coverage average.
static SC4Rect<long> ClampedCityCells(cISC4City* city, cISC4Occupant* o) {
    SC4Rect<long> r;
    o->GetBoundingCityCells(r);
    const int32_t mx = static_cast<int32_t>(city->CellCountX()) - 1;
    const int32_t mz = static_cast<int32_t>(city->CellCountZ()) - 1;
    // no overlap with the city gives an empty rectangle (§5.4 step 4).
    if (static_cast<int32_t>(r.bottomRightX) < 0 || static_cast<int32_t>(r.bottomRightY) < 0 ||
        static_cast<int32_t>(r.topLeftX) > mx || static_cast<int32_t>(r.topLeftY) > mz) {
        return SC4Rect<long>(0, 0, -1, -1);
    }
    return SC4Rect<long>(Clamp(static_cast<int32_t>(r.topLeftX), 0, mx), Clamp(static_cast<int32_t>(r.topLeftY), 0, mz),
                         Clamp(static_cast<int32_t>(r.bottomRightX), 0, mx),
                         Clamp(static_cast<int32_t>(r.bottomRightY), 0, mz));
}

DisasterIgniteInputs DisasterIgniteInputsOf(cISC4City* city, cISC4Occupant* o, bool force, bool paused,
                                            bool emergencyPaused) {
    DisasterIgniteInputs v;
    if (city == nullptr || o == nullptr) return v;
    v.type = static_cast<uint32_t>(o->GetType());  // slot 7 (§5.4 step 2)
    {
        std::vector<uint32_t> f;
        cISCPropertyHolder* h = o->AsPropertyHolder();
        // §5.4 step 3 (DS38): read as an unsigned integer, a missing one is 0.
        v.flammability = (h != nullptr && props::GetU32s(h, kPropFlammability, f) && !f.empty()) ? f[0] : 0u;
    }
    v.burning = DisasterOccupantBurning(o);  // 0x69DBEBE1 (§5.4 step 5)
    const SC4Rect<long> r = ClampedCityCells(city, o);
    v.x0 = static_cast<int32_t>(r.topLeftX);
    v.z0 = static_cast<int32_t>(r.topLeftY);
    v.x1 = static_cast<int32_t>(r.bottomRightX);
    v.z1 = static_cast<int32_t>(r.bottomRightY);
    v.coverage = FireCoverageOver(city, r);  // the same average as the core reads
    // The exits these inputs admit (§5.4 "The refusals, exactly"). Exit 4 is
    // not computed here: it needs the D9 curve, so read it from `coverage`.
    const bool burnable = v.type == kTypeBuilding || v.type == kTypeFlora || v.type == kTypeProp;
    if (!force && !emergencyPaused && paused) v.exits |= 1u << 0;
    if (!burnable) v.exits |= 1u << 1;
    if (v.flammability == 0) v.exits |= 1u << 2;
    if (v.burning && !force) v.exits |= 1u << 4;
    return v;
}

uint8_t DisasterPoliceCoverage(cISC4City* city, int32_t x, int32_t z) {
    cISC4PoliceSimulator* p = city ? city->GetPoliceSimulator() : nullptr;  // city slot 65
    cISC4SimGrid<uint8_t>* g = p ? AsGame(p)->GetPolicePowerGrid() : nullptr;  // slot 5
    return g ? g->GetCellValue(x, z) : uint8_t{0};  // §6.4 step 2, GetCellValue
}

// [question Q32] The riot's road search (§6.4 step 1) reads the traffic
// network for network types 0, 3 and 6 [inferred]; here: FindNetworkTile
// (traffic network map slot 8) with those types' flag bits, the multi-cell
// index included (as landfill's road test).
bool DisasterIsRoadCell(cISC4City* city, int32_t x, int32_t z) {
    if (city == nullptr) return false;
    if (x < 0 || z < 0 || x >= static_cast<int32_t>(city->CellCountX()) ||
        z >= static_cast<int32_t>(city->CellCountZ())) {
        return false;
    }
    auto* map = reinterpret_cast<cFC4ISC4TrafficNetworkMap*>(city->GetTrafficNetwork());  // city slot 40, borrowed
    return map != nullptr && map->FindNetworkTile(x, z, kRiotRoadFlags, true) != nullptr;
}

// [question Q33] MarkRiotNetworkOccupants takes the occupant of the traffic
// map's tile (SafeGetTileInfoAtCell, slot 14; the tile record is not
// specified). Literal reading here: the first network occupant the occupant
// manager lists at the cell whose network flags have any bit of 0x1F4F (the
// traffic networks, flora §6.4.1); its cISC4Occupant.
uint64_t DisasterNetworkOccupantAt(cISC4City* city, int32_t x, int32_t z) {
    auto list = DisasterOccupantsIn(city, x, z, x, z, kTypeNetwork, nullptr);
    if (!list) return 0;
    uint64_t found = 0;
    for (cISC4Occupant* o : *list) {
        if (o == nullptr || found != 0) continue;
        void* p = nullptr;
        if (!o->QueryInterface(kIID_cISC4NetworkOccupant, &p) || p == nullptr) continue;
        cISC4NetworkOccupant* n = static_cast<cISC4NetworkOccupant*>(p);
        FC4_ACQUIRED(n, "QueryInterface");
        if (n->HasAnyNetworkFlag(kTrafficNetworkFlags)) found = TokenOf(o);  // slot 9
        FC4_RELEASE(n);
    }
    return found;  // the occupant stays alive in the occupant manager after the list goes
}

bool DisasterOccupantCell(cISC4City* city, cISC4Occupant* o, int32_t* x, int32_t* z) {
    if (city == nullptr || !InManager(o)) return false;
    cS3DVector3 v;
    o->GetPosition(&v);
    int a = 0, b = 0;
    city->PositionToCell(v.fX, v.fZ, a, b);  // slot 95
    *x = a;
    *z = b;
    return true;
}

bool DisasterOccupantBurning(cISC4Occupant* o) {
    if (o == nullptr) return false;
    std::vector<uint32_t> v;
    cISCPropertyHolder* h = o->AsPropertyHolder();
    return h != nullptr && props::GetU32s(h, kPropBurning, v) && !v.empty() && v[0] != 0;
}

fc4_disasters_host MakeDisasterHost(DisasterHostContext* ctx) {
    fc4_disasters_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.now_ms = &NowMs;
    h.emergency_pause = &EmergencyPause;
    h.emergency_resume = &EmergencyResume;
    h.disable_save = &DisableSave;
    h.enable_save = &EnableSave;
    h.sim_paused = &SimPaused;
    h.sim_emergency_paused = &SimEmergencyPaused;
    h.city_established = &CityEstablished;
    h.city_cells = &CityCells;
    h.cell_width = &CellWidth;
    h.post = &Post;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.register_tick = &RegisterTick;
    h.unregister_tick = &UnregisterTick;
    h.record_history = &RecordHistory;
    h.add_ref = &AddRef;
    h.release = &Release;
    h.follow_disasters = &FollowDisasters;
    h.go_to_location = &GoToLocation;
    h.fire_dispatch_count = &FireDispatchCount;
    h.police_dispatch_count = &PoliceDispatchCount;
    h.building_type_flags = &BuildingTypeFlags;
    h.occupant_type = &OccupantType;
    h.occupant_in_manager = &OccupantInManager;
    h.occupant_cell = &OccupantCell;
    h.occupant_cell_rect = &OccupantCellRect;
    h.occupant_position = &OccupantPosition;
    h.occupant_bbox = &OccupantBBox;
    h.occupant_bbox_areas = &OccupantBBoxAreas;
    h.occupant_prop_u32 = &PropU32;
    h.occupant_prop_i64 = &PropI64;
    h.occupant_prop_f32 = &PropF32;
    h.occupant_prop_bool = &PropBool;
    h.occupants_in_rect = &OccupantsInRect;
    h.remove_occupant = &RemoveOccupant;
    h.occupant_lot = &OccupantLot;
    h.set_lot_condition = &SetLotCondition;
    h.fire_coverage = &FireCoverage;
    h.flammability_at = &FlammabilityAt;
    h.create_fire_occupant = &CreateFireOccupant;
    h.remove_fire_occupant = &RemoveFireOccupant;
    h.add_fire_aura = &AddFireAura;
    h.tutorial_no_burn_down = &TutorialNoBurnDown;
    h.wind = &Wind;
    h.play_effect = &PlayEffect;
    h.play_effect_at = &PlayEffectAt;
    h.burn_flora = &BurnFlora;
    h.demolish_lot = &DemolishLot;
    h.spew_radiation = &SpewRadiation;
    h.police_coverage = &PoliceCoverage;
    h.is_road_cell = &IsRoadCell;
    h.cell_corner_position = &CellCorner;
    h.create_riot_leader = &CreateRiotLeader;
    h.network_occupant_at = &NetworkOccupantAt;
    h.unset_removal_flags = &UnsetRemovalFlags;
    h.set_removal_flags = &SetRemovalFlags;
    h.automaton_cell = &AutomatonCell;
    h.set_riot_effect = &SetRiotEffect;
    h.send_riot_message = &SendRiotMessage;
    h.end_riot_leader = &EndRiotLeader;
    h.riot_leader_tick = &RiotLeaderTick;
    h.create_toxic_occupant = &CreateToxic;
    h.set_toxic_amount = &SetToxicAmount;
    h.remove_toxic_occupant = &RemoveToxic;
    h.apply_terrain_brush = &TerrainBrush;
    h.redraw_terrain = &RedrawTerrain;
    h.end_long_terrain_operation = &EndLongTerrain;
    h.stop_terrain_texture_updates = &StopTerrainTextures;
    h.place_repulsor = &PlaceRepulsor;
    h.find_watered_pipe = &FindWateredPipe;
    h.pipe_cell = &PipeCell;
    h.view_level = &ViewLevel;
    h.create_leak_effect = &CreateLeakEffect;
    h.connected_pipe_cells = &ConnectedPipeCells;
    h.demolish_pipe = &DemolishPipe;
    h.volcano_start_deformation = &VolcanoStartDeformation;
    h.volcano_start_lava = &VolcanoStartLava;
    h.volcano_lava_rates = &VolcanoLavaRates;
    h.volcano_end_lava = &VolcanoEndLava;
    return h;
}

void DisasterHostReleaseHeld(DisasterHostContext* ctx) {
    if (ctx != nullptr) ctx->held.clear();
}

void DisasterHostSyncFires(DisasterHostContext* ctx, const fc4_disasters* core) {
    if (ctx->shadow) return;
    fc4_disaster_fire fire{};
    for (size_t n = 0; fc4_disasters_get_fire(core, n, &fire); ++n) {
        auto it = ctx->fireOccupants.find(fire.occupant);
        if (it != ctx->fireOccupants.end())
            static_cast<DisasterFire*>(it->second)->Sync(fire.stage, fire.max_stage, fire.counter, fire.hosed);
    }
}

void DisasterHostReleaseAll(DisasterHostContext* ctx) {
    if (ctx == nullptr) return;
    ctx->held.clear();
    for (auto& [t, n] : ctx->refs) {
        cISC4Occupant* o = Occ(t);
        for (int64_t i = 0; i < n && o != nullptr; ++i) FC4_RELEASE(o);
    }
    ctx->refs.clear();
    // What the live creates made and the matching remove / end did not drop
    // (a city closing mid-episode). The occupant manager is gone by now, so
    // the objects are only released.
    for (auto& [t, fo] : ctx->fireOccupants) {
        (void)t;
        static_cast<DisasterFire*>(fo)->hose = nullptr;
        fo->AsOccupant()->Shutdown();
        FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(fo));
    }
    ctx->fireOccupants.clear();
    for (auto& [t, a] : ctx->riotLeaders) {
        (void)t;
        FC4_RELEASE(a);
    }
    ctx->riotLeaders.clear();
}

int DisasterHostLoadTuning(fc4_disasters* core) {
    int found = 0;
    for (uint32_t inst : {kTuningFlame, kTuningRiot}) {
        props::Exemplar ex;
        if (!ex.Load(kTuningType, kTuningGroup, inst)) continue;
        fc4_props* p = props::ToCoreAll(ex.Holder());
        if (p == nullptr) continue;
        if (fc4_disasters_set_tuning(core, inst, p)) ++found;
        fc4_props_destroy(p);
    }
    return found;
}

}  // namespace fc4
