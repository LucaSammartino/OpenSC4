// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.
//
// cFC4DisasterLayer (cFC4DisasterLayer.h; disasters.md §2, §2.8).
// DRAFT (glue-disasters-impl4 round 1): not yet in CMakeLists.txt; the
// includes and helper names follow FC4DisasterHost.cpp and must be checked
// at the first build.

#include "cFC4DisasterLayer.h"

#include "FC4DisasterHost.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "GZServPtrs.h"
#include "cIGZCOM.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4DisasterInstance.h"
#include "cISC4DisasterManager.h"
#include "cISC4Occupant.h"

#include <cstddef>

static_assert(offsetof(FC4DisasterCostRecord, totalCost) == 0x08, "record +0x08");
static_assert(offsetof(FC4DisasterCostRecord, kinds) == 0x10, "record +0x10");
#if defined(_M_IX86)
static_assert(offsetof(FC4DisasterCostRecord, fireEffectiveness) == 0x5C, "record +0x5C");
static_assert(offsetof(FC4DisasterCostRecord, policeEffectiveness) == 0x60, "record +0x60");
#endif

namespace {

// §1: the (type, class) table, in table order [compat].
const uint32_t kManagers[7][2] = {
    {0xC4C90997, 0xA9B5AB9F}, {0x621CDA33, 0x69C1718E}, {0x42963812, 0x0A086BD3}, {0x4A1332E8, 0x4A132FDF},
    {0xCA380B06, 0x4A38206C}, {0x6A7B4397, 0x6A7B3E9B}, {0xEBFB0404, 0xEBFB0402},
};
// §2.5: occupant removed and the four dispatch messages.
const uint32_t kMsgOccupantRemoved = 0x99EF1143;
const uint32_t kAssessment[5] = {kMsgOccupantRemoved, 0xE94671EC, 0xE94671EB, 0x894671F6, 0xE94671ED};
const uint32_t kMsgPeriodStarted = 0xCA1B1E64, kMsgPeriodEnded = 0xAA1B2002;
const uint32_t kOccBuilding = 0x278128A0, kOccNetwork = 0xC772BF98, kPropDemolitionCost = 0x8A1C3E72;

cFC4DisasterLayer* L(void* u) { return static_cast<cFC4DisasterLayer*>(u); }
uint64_t Tok(const void* p) { return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(p)); }
template <class T> T* Ptr(uint64_t t) { return reinterpret_cast<T*>(static_cast<uintptr_t>(t)); }

cFC4ISC4Simulator* Sim(void* u) {
    cISC4City* c = L(u)->City();
    return c ? AsGame(c->GetSimulator()) : nullptr;  // city slot 50, borrowed
}
uint64_t Now(void*) { return fc4::DisasterNowRealMs(); }  // TODO(build): FC4DisasterHost.cpp NowRealMs is file-local
void Pause(void* u) { if (auto* s = Sim(u)) s->EmergencyPause(); }    // slot 11
void Resume(void* u) { if (auto* s = Sim(u)) s->EmergencyResume(); }  // slot 14
void SaveOff(void* u) { if (cISC4City* c = L(u)->City()) c->DisableSave(); }  // city slot 104
void SaveOn(void* u) { if (cISC4City* c = L(u)->City()) c->EnableSave(); }    // city slot 103
void Post(uint32_t type, uint32_t d1, int32_t d2, int32_t d3, uint32_t d4, cIGZUnknown* obj) {
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* m = FC4MakeMessage(type, static_cast<intptr_t>(d1), d2, d3);
    m->SetData4(static_cast<intptr_t>(d4));
    if (obj) m->SetVoid1(obj);  // TODO(build): §2.4 the message's IGZUnknown (slot +0x84), not void1
    ms->MessagePost(m, false);
    m->Release();
}
void Started(void*, uint32_t n, int32_t x, int32_t z, uint64_t inst) {
    cISC4DisasterInstance* i = Ptr<cISC4DisasterInstance>(inst);
    Post(kMsgPeriodStarted, n, x, z, i ? i->GetID() : 0, i);
}
void Ended(void*) { Post(kMsgPeriodEnded, 0, 0, 0, 0, nullptr); }
void Subscribe(void* u, bool on) {
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    for (uint32_t id : kAssessment) {
        if (on) ms->AddNotification(static_cast<cIGZMessageTarget2*>(L(u)), id);
        else ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(L(u)), id);
    }
}
void AddRefOcc(void*, uint64_t t) { if (auto* o = Ptr<cISC4Occupant>(t)) FC4_ADDREF(o); }
void ReleaseOcc(void*, uint64_t t) { if (auto* o = Ptr<cISC4Occupant>(t)) FC4_RELEASE(o); }
bool Follow(void*) { return false; }  // [question Q31] as FC4DisasterHost: the preference byte is not reachable
void GoTo(void*, int32_t, int32_t, uint32_t) {}  // view-side only (§2.7); the camera move is not wired [Q31]
bool InMgr(void*, uint64_t t) { auto* o = Ptr<cISC4Occupant>(t); return o && o->IsFlagSet(1); }
bool OccCell(void* u, uint64_t t, int32_t* x, int32_t* z) {
    return fc4::DisasterOccupantCell(L(u)->City(), Ptr<cISC4Occupant>(t), x, z);
}
bool InstLoc(void*, uint64_t t, int32_t* x, int32_t* z) {
    auto* i = Ptr<cISC4DisasterInstance>(t);
    int a = 0, b = 0;
    if (!i || !i->GetLocation(a, b)) return false;  // instance slot 6 (§3.2)
    *x = a;
    *z = b;
    return true;
}
bool Counts(void* u, uint32_t* f, uint32_t* p) {
    cISC4City* c = L(u)->City();
    cISC4FireProtectionSimulator* fs = c ? c->GetFireProtectionSimulator() : nullptr;  // city slot 57
    cISC4PoliceSimulator* ps = c ? c->GetPoliceSimulator() : nullptr;                // city slot 65
    if (!fs || !ps) return false;
    const int32_t nf = AsGame(fs)->GetTotalDispatchCount(), np = AsGame(ps)->GetTotalDispatchCount();
    *f = nf > 0 ? static_cast<uint32_t>(nf) : 0u;
    *p = np > 0 ? static_cast<uint32_t>(np) : 0u;
    return true;
}

}  // namespace

cIGZUnknown* CreateFC4DisasterLayer() {
    // No reference: the director's QueryInterface adds the city's one (§2.8).
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4DisasterLayer*>(new cFC4DisasterLayer()));
}

cFC4DisasterLayer::cFC4DisasterLayer() {
    host_.struct_size = sizeof(host_);
    host_.user = this;
    host_.now_ms = &Now;
    host_.emergency_pause = &Pause;
    host_.emergency_resume = &Resume;
    host_.disable_save = &SaveOff;
    host_.enable_save = &SaveOn;
    host_.post_started = &Started;
    host_.post_ended = &Ended;
    host_.subscribe = &Subscribe;
    host_.add_ref = &AddRefOcc;
    host_.release = &ReleaseOcc;
    host_.follow_disasters = &Follow;
    host_.go_to = &GoTo;
    host_.occupant_in_manager = &InMgr;
    host_.occupant_cell = &OccCell;
    host_.instance_location = &InstLoc;
    host_.dispatch_counts = &Counts;
    core_ = fc4_dlayer_create(&host_);
    fc4::Log("cFC4DisasterLayer(%p) created, core %p", static_cast<void*>(this), static_cast<void*>(core_));
}

cFC4DisasterLayer::~cFC4DisasterLayer() {
    if (core_) fc4_dlayer_destroy(core_);
}

cISC4City* cFC4DisasterLayer::City() const {
    cISC4AppPtr app;
    return app ? app->GetCity() : nullptr;  // borrowed, no reference held (§2.8 contract 4)
}

bool cFC4DisasterLayer::QueryInterface(uint32_t riid, void** ppv) {
    if (!ppv) return false;
    *ppv = nullptr;
    if (riid == kFC4_IID_cISC4DisasterLayer || riid == GZIID_cIGZUnknown) {
        *ppv = static_cast<cFC4ISC4DisasterLayer*>(this);
    } else if (riid == GZIID_cIGZMessageTarget2) {
        *ppv = static_cast<cIGZMessageTarget2*>(this);
    } else {
        return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4DisasterLayer::AddRef() { return ++refCount_; }

uint32_t cFC4DisasterLayer::Release() {
    const uint32_t n = refCount_ > 0 ? --refCount_ : 0;
    if (n == 0) delete this;
    return n;
}

// §2.2: managers created in table order, Init in map (ascending type) order.
bool cFC4DisasterLayer::Init() {
    if (!core_ || !fc4_dlayer_init(core_)) return true;  // once
    cIGZCOM* com = GZCOM();
    for (const auto& tc : kManagers) {
        cISC4DisasterManager* m = nullptr;
        if (com && com->GetClassObject(tc[1], kFC4_IID_cISC4DisasterManager, reinterpret_cast<void**>(&m)) && m) {
            // TODO(build): §1 says CreateInstance(class, 0x021E9CBF); check the cIGZCOM slot name.
            managers_[m->GetID()] = m;  // the reference CreateInstance gave
        }
    }
    for (auto& [type, m] : managers_) m->Init();  // manager slot 3
    fc4_dlayer_reset_cost(core_);
    RefreshRecord();
    fc4::Log("cFC4DisasterLayer::Init: %u native managers", static_cast<unsigned>(managers_.size()));
    return true;
}

// §2.2 / §2.8: the managers' Shutdown re-enters slots 9 and 14 and Releases.
bool cFC4DisasterLayer::Shutdown() {
    for (auto& [type, m] : managers_) m->Shutdown();  // slot 4
    std::map<uint32_t, cISC4DisasterManager*> held;
    held.swap(managers_);
    for (auto& [type, m] : held) FC4_RELEASE(m);
    if (core_) fc4_dlayer_shutdown(core_);
    RefreshRecord();
    return true;
}

bool cFC4DisasterLayer::EnableDisasters(bool e) { return fc4_dlayer_enable(core_, e); }
bool cFC4DisasterLayer::DisastersEnabled() { return fc4_dlayer_enabled(core_); }

cISC4DisasterManager* cFC4DisasterLayer::GetDisasterManager(uint32_t type) {
    auto it = managers_.find(type);
    return it == managers_.end() ? nullptr : it->second;  // no AddRef [compat]
}

bool cFC4DisasterLayer::DisasterHasBegun(cISC4DisasterInstance* i) {
    const bool r = fc4_dlayer_begun(core_, Tok(i));
    RefreshRecord();
    return r;
}
bool cFC4DisasterLayer::DisasterHasEnded() {
    const bool r = fc4_dlayer_ended(core_);
    RefreshRecord();
    return r;
}
bool cFC4DisasterLayer::IsADisasterOngoing() { return fc4_dlayer_ongoing(core_) != 0; }
bool cFC4DisasterLayer::GoToNextDisaster() { return fc4_dlayer_go_to_next(core_); }

bool cFC4DisasterLayer::GetMostRecentDisasterLocation(cISC4DisasterInstance* i, int& x, int& z) {
    int32_t a = 0, b = 0;
    if (!fc4_dlayer_most_recent(core_, Tok(i), &a, &b)) return false;
    x = a;
    z = b;
    return true;
}
bool cFC4DisasterLayer::RegisterDisasterLocation(cISC4DisasterInstance* i, int x, int z, cISC4Occupant* o,
                                                 int32_t zoom, bool focus) {
    return fc4_dlayer_register(core_, Tok(i), x, z, Tok(o), static_cast<uint32_t>(zoom), focus);
}
bool cFC4DisasterLayer::UnregisterDisasterLocations(cISC4DisasterInstance* i) {
    return fc4_dlayer_unregister(core_, Tok(i));
}
bool cFC4DisasterLayer::UnregisterDisasterLocationByOccupant(cISC4DisasterInstance* i, cISC4Occupant* o) {
    return fc4_dlayer_unregister_by_occupant(core_, Tok(i), Tok(o));
}
int32_t cFC4DisasterLayer::GetDisasterLocationCount() { return fc4_dlayer_location_count(core_); }

// Slot 17: stored values, not resolved; the instance AddRef'd for the caller.
bool cFC4DisasterLayer::GetDisasterLocationByIndex(int32_t index, cISC4DisasterInstance** inst, int& x, int& z) {
    fc4_dlayer_location l{};
    if (!fc4_dlayer_location_at(core_, index, &l)) return false;
    cISC4DisasterInstance* i = Ptr<cISC4DisasterInstance>(l.instance);
    if (inst) {
        *inst = i;
        if (i) FC4_ADDREF(i);
    }
    x = l.x;
    z = l.z;
    return true;
}

const void* cFC4DisasterLayer::GetDisasterCost() { return &record_; }

void cFC4DisasterLayer::RefreshRecord() {
    fc4_dlayer_record r{};
    if (!core_ || !fc4_dlayer_record(core_, &r)) return;
    record_.buildingCount = r.building_count;
    record_.totalCost = r.total_cost;
    for (int k = 0; k < 4; ++k) {
        record_.kinds[k].count = r.kind_count[k];
        record_.kinds[k].cost = r.kind_cost[k];
    }
    record_.fireEffectiveness = r.fire_effectiveness;
    record_.policeEffectiveness = r.police_effectiveness;
}

// §2.5: occupant removed (only while the city is established) and dispatch.
bool cFC4DisasterLayer::DoMessage(cIGZMessage2* msg) {
    cIGZMessage2Standard* m = nullptr;
    if (!msg || !msg->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) || !m) return false;
    FC4_ACQUIRED(m, "QueryInterface");
    const uint32_t type = m->GetType();
    if (type == kMsgOccupantRemoved) {
        cISC4City* c = City();
        auto* o = static_cast<cISC4Occupant*>(m->GetVoid1());  // TODO(build): which field carries the occupant
        if (o && c && c->GetEstablished()) {                    // city slot 22
            const uint32_t t = o->GetType();
            int64_t cost = 0;
            uint64_t tok = Tok(o);
            const bool has = fc4::DisasterOccupantPropI64(tok, kPropDemolitionCost, &cost);  // TODO(build): helper
            const uint32_t kind = t == kOccBuilding ? FC4_DLAYER_OCC_BUILDING
                                  : t == kOccNetwork ? FC4_DLAYER_OCC_NETWORK
                                                     : FC4_DLAYER_OCC_OTHER;
            // Building flags: 0 [question Q31], as FC4DisasterHost's BuildingTypeFlags.
            fc4_dlayer_occupant_removed(core_, kind, has, cost, 0);
        }
    } else {
        fc4_dlayer_dispatch(core_, type, m->GetData1(), static_cast<uint32_t>(m->GetData2()));
    }
    RefreshRecord();
    FC4_RELEASE(m);
    return true;
}
