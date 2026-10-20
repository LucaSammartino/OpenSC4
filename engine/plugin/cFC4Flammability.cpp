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

#include "cFC4Flammability.h"

#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Seed.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"

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
#include "cISC4DisasterManagerFire.h"
#include "cISC4HistoryWarehouse.h"
#include "cISC4LotManager.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4OrdinanceSimulator.h"
#include "cISC4PlumbingSimulator.h"
#include "cISC4SimGrid.h"
#include "cISC4TutorialSystem.h"
#include "cFC4Tutorial.h"  // the glue15 lane: fc4::TutorialIsTutorial
#include "cISC4WeatherSimulator.h"
#include "cISC4ZoneManager.h"
#include "cISCPropertyHolder.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstring>
#include <string>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4BuildingOccupant = 0x87DDA3A9;
constexpr uint32_t kIID_cISC4Lot = 0xE818FCED;
/// The fire disaster manager's interface (§6.8, §12).
constexpr uint32_t kIID_cISC4DisasterManagerFire = 0x49B43F57;
/// The fire disaster manager's type (§6.8).
constexpr uint32_t kFireDisasterManager = 0x621CDA33;

// Agent registration (§4.1 step 2): level 4 (SimNewDay) only, flags 2.
constexpr uint32_t kAgentLevel = 4;
constexpr uint32_t kAgentFlags = 2;
const char kAgentName[] = "cSC4FlammabilitySimulator";

// The six notifications of §4.1 step 3, in order, and exactly these ([compat]:
// in particular NOT the exemplar-changed message 0xC53D10AA, §5).
constexpr uint32_t kNotifications[] = {0x99EF1142, 0x99EF1143, 0x99EF1145, 0x6716A4E5, 0xC2C09554, 0xE99B3D4D};

// Messages (§5).
constexpr uint32_t kMsgInsertOccupant = 0x99EF1142;
constexpr uint32_t kMsgRemoveOccupant = 0x99EF1143;
constexpr uint32_t kMsgChangeOccupant = 0x99EF1145;
constexpr uint32_t kMsgConditionChanged = 0xE99B3D4D;
/// The agent level's message; data1 is the julian day.
constexpr uint32_t kMsgSimNewDay = 0x66956814;
/// Posted at every EndOfCycle with void1 = IGZUnknown = this simulator (§9).
constexpr uint32_t kMsgSimCycleComplete = 0x08F415FE;

// Tuning exemplar (§8, "Flammability Simulator").
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0xA8F41EE3;

// The three accepted occupant types (§6.1, §6.2).
constexpr uint32_t kOccupantTypes[] = {0x278128A0, 0x74758926, 0xA823821E};

using GridU8 = cISC4SimGrid<uint8_t>;

cFC4FlammabilitySimulator* Self(void* u) { return static_cast<cFC4FlammabilitySimulator*>(u); }

cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
GridU8* GridOf(uint64_t t) { return reinterpret_cast<GridU8*>(FromToken(t)); }

// Lots and the lot manager travel as SDK pointers but are called through the
// verified slot order (FC4Interfaces.h), as power does: when the lot developer
// or another subsystem replaces those classes, it implements cFC4ISC4Lot /
// cFC4ISC4LotManager, not the SDK declarations.
cFC4ISC4Lot* LotOf(uint64_t t) { return t ? fc4::AsGame(reinterpret_cast<cISC4Lot*>(FromToken(t))) : nullptr; }

cFC4ISC4LotManager* LotMgr(void* u) {
    cISC4LotManager* lm = Self(u)->LotManager();
    return lm ? fc4::AsGame(lm) : nullptr;
}

cFC4ISC4WeatherSimulator* Weather(void* u) {
    cISC4WeatherSimulator* w = Self(u)->Weather();
    return w ? fc4::AsGame(w) : nullptr;
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

// Runs f(T*) on the object QI'd to `iid` (released afterwards); false if the QI
// fails.
template <class T, class F>
bool WithQi(cIGZUnknown* u, uint32_t iid, F f) {
    T* p = Query<T>(u, iid);
    if (p == nullptr) return false;
    f(p);
    Drop(p);
    return true;
}

// ---- city, tuning, tutorial ---------------------------------------------------------------------

void CityCellCounts(void* u, int32_t* x, int32_t* z) {
    cISC4City* c = Self(u)->City();
    if (x != nullptr) *x = c ? static_cast<int32_t>(c->CellCountX()) : 0;  // slots 93, 94
    if (z != nullptr) *z = c ? static_cast<int32_t>(c->CellCountZ()) : 0;
}

void PositionToCell(void* u, float x, float z, int32_t* cx, int32_t* cz) {
    int32_t a = 0, b = 0;
    if (cISC4City* c = Self(u)->City()) c->PositionToCell(x, z, a, b);  // slot 95
    if (cx != nullptr) *cx = a;
    if (cz != nullptr) *cz = b;
}

// The core asks this only at the start of the fire roll (§6.8), so it also
// counts the rolls for the status line.
bool IsTutorial(void* u) {
    cISC4City* c = Self(u)->City();
    cISC4TutorialSystem* t = c ? c->GetTutorialSystem() : nullptr;  // city slot 82
    // Tutorial slot 8, or fc4_tutorial_is_tutorial when the system is ours
    // (enable = tutorial).
    const bool tutorial = fc4::TutorialIsTutorial(t);
    ++Self(u)->Probe().fireRolls;
    if (tutorial) ++Self(u)->Probe().tutorialSkips;
    return tutorial;
}

fc4_props* Tuning(void*) {
    if (!Gate(fc4::Sw::ExemplarLoads, "flammability tuning exemplar (none)")) return nullptr;
    fc4::props::Exemplar ex;
    if (!ex.Load(kTuningType, kTuningGroup, kTuningInstance)) {
        fc4::Log("flammability: tuning exemplar unavailable; the defaults stay");
        return nullptr;
    }
    return fc4::props::ToCoreAll(ex.Holder());
}

// ---- occupants ----------------------------------------------------------------------------------

uint32_t OccupantType(void*, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::FlammabilityOccupants, "flammability occupant GetType (0)")) return 0;
    return static_cast<uint32_t>(OccOf(occ)->GetType());  // slot 7
}

bool OccupantProperty(void*, uint64_t occ, uint32_t id, fc4_props* out) {
    if (occ == 0 || !Gate(fc4::Sw::FlammabilityOccupants, "flammability occupant property read (absent)")) {
        return false;
    }
    cISCPropertyHolder* h = OccOf(occ)->AsPropertyHolder();  // borrowed, slot 6
    return h != nullptr && fc4::props::CopyToCore(h, id, out);
}

void OccupantBounds(void*, uint64_t occ, fc4_cell_rect* out) {
    if (out == nullptr || occ == 0 || !Gate(fc4::Sw::FlammabilityOccupants, "flammability GetBoundingCityCells")) {
        return;
    }
    SC4Rect<long> r;
    OccOf(occ)->GetBoundingCityCells(r);  // slot 11
    out->x1 = static_cast<int32_t>(r.topLeftX);
    out->z1 = static_cast<int32_t>(r.topLeftY);
    out->x2 = static_cast<int32_t>(r.bottomRightX);
    out->z2 = static_cast<int32_t>(r.bottomRightY);
}

void OccupantPosition(void*, uint64_t occ, float* xyz) {
    if (xyz == nullptr || occ == 0 || !Gate(fc4::Sw::FlammabilityOccupants, "flammability occupant GetPosition")) {
        return;
    }
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);  // slot 8
    xyz[0] = v.fX;
    xyz[1] = v.fY;
    xyz[2] = v.fZ;
}

// GetOccupantsByStandardCityCells(list, &nw, &se, type, 0xFFFFFFFF) (§6.2 step
// 2). [abi] The typed overload (compiled slot 41) fills an SC4List (A) with one
// reference each, kept until the outermost call into the core returns.
//
// [choice] The spec asks for the FILTER overload (slot 40) with a type filter
// accepting building, flora and prop. cISC4OccupantFilter is not in
// docs/abi/interfaces.tsv, so its vtable is not verified; the typed overload is
// (slot 41 forwards to 40 with no filter, so the two are the same call). An
// occupant has exactly one type, so querying the three accepted types and
// merging gives the same set, in three list traversals instead of one.
void OccupantsInCells(void* u, const fc4_cell_rect* r, fc4_token_callback cb, void* ctx) {
    if (r == nullptr || cb == nullptr) return;
    if (!Gate(fc4::Sw::FlammabilityOccupants, "flammability occupant query (no occupants)")) return;
    cISC4OccupantManager* om = Self(u)->OccupantManager();
    if (om == nullptr) return;
    const SC4Point<int> mn(r->x1, r->z1);
    const SC4Point<int> mx(r->x2, r->z2);
    for (uint32_t type : kOccupantTypes) {
        auto list = std::make_shared<SC4List<cISC4Occupant*>>();
        om->GetOccupantsByStandardCityCells(*list, &mn, &mx, type, 0xFFFFFFFFu);  // slot 41
        for (cISC4Occupant* o : *list) {
            if (o != nullptr) cb(ctx, TokenOf(o));
        }
        Self(u)->Hold(std::move(list));
    }
}

// ---- lots ---------------------------------------------------------------------------------------

uint64_t OccupantLot(void* u, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::FlammabilityLots, "flammability GetOccupantLot (none)")) return 0;
    cFC4ISC4LotManager* lm = LotMgr(u);
    return lm ? TokenOf(lm->GetOccupantLot(OccOf(occ))) : 0;  // lot manager slot 14, borrowed
}

uint8_t LotCondition(void*, uint64_t lot) {
    if (lot == 0 || !Gate(fc4::Sw::FlammabilityLots, "flammability GetCondition (0)")) return 0;
    return static_cast<uint8_t>(LotOf(lot)->GetCondition());  // lot slot 35
}

// lot.GetBuilding()->AsOccupant() (§6.3, §5); None without a building.
uint64_t LotBuilding(void*, uint64_t lot) {
    if (lot == 0 || !Gate(fc4::Sw::FlammabilityLots, "flammability GetBuilding (none)")) return 0;
    cISC4BuildingOccupant* b = LotOf(lot)->GetBuilding();  // lot slot 48, not ref-counted
    if (b == nullptr) return 0;
    cISC4Occupant* o = b->AsOccupant();  // building occupant slot 3
    return TokenOf(o);
}

uint64_t RandomLot(void* u) {
    ++Self(u)->Probe().randomLots;
    if (!Gate(fc4::Sw::FlammabilityLots, "flammability GetRandomLot (none)")) return 0;
    cFC4ISC4LotManager* lm = LotMgr(u);
    return lm ? TokenOf(lm->GetRandomLot()) : 0;  // lot manager slot 13, borrowed
}

// ---- other subsystems ---------------------------------------------------------------------------

// The ordinance factor; false = no ordinance simulator, and the core reads 1.0
// (§6.4, §4.1 step 5 [choice]).
bool CumulativeFactor(void* u, uint32_t prop, uint32_t index, float* out) {
    if (!Gate(fc4::Sw::FlammabilityOrdinance, "flammability GetCumulativeFactor (absent)")) return false;
    cISC4OrdinanceSimulator* o = Self(u)->Ordinance();
    if (o == nullptr) return false;
    if (out != nullptr) *out = o->GetCumulativeFactor(prop, index);  // slot 25, ST0
    return true;
}

bool CurrentSeason(void* u, int32_t* out) {
    if (!Gate(fc4::Sw::FlammabilitySeason, "flammability GetCurrentSeason (absent)")) return false;
    cFC4ISC4WeatherSimulator* w = Weather(u);
    if (w == nullptr) return false;
    if (out != nullptr) *out = w->GetCurrentSeason();  // slot 14
    return true;
}

// The plumbing simulator's IsCellWatered (§6.5). -1 = no plumbing simulator,
// which the core reads as "not watered" (§4.1 step 5 [choice]); 0 / 1 are the
// two answers. The answers are counted for the status line.
int32_t IsCellWateredCall(void* u, int32_t x, int32_t z) {
    if (!Gate(fc4::Sw::FlammabilityWater, "flammability IsCellWatered (no plumbing)")) return -1;
    // Called through the verified slot order (FC4Interfaces.h): this is our own
    // replacement when the plumbing subsystem is on, and the game's class
    // otherwise. Slot 11 is IsCellWatered in both.
    cISC4PlumbingSimulator* p = Self(u)->Plumbing();
    if (p == nullptr) return -1;
    return fc4::AsGame(p)->IsCellWatered(x, z) ? 1 : 0;
}

int32_t IsCellWatered(void* u, int32_t x, int32_t z) {
    const int32_t r = IsCellWateredCall(u, x, z);
    cFC4FlammabilitySimulator::Probes& p = Self(u)->Probe();
    ++p.wateredQueries;
    if (r > 0) ++p.watered;
    if (r < 0) ++p.noPlumbing;
    return r;
}

bool ZoneType(void* u, int32_t x, int32_t z, uint32_t* out) {
    if (!Gate(fc4::Sw::FlammabilityZoning, "flammability GetZoneType (unzoned)")) return false;
    cISC4City* c = Self(u)->City();
    cISC4ZoneManager* zm = c ? c->GetZoneManager() : nullptr;  // city slot 36
    if (zm == nullptr) return false;
    if (out != nullptr) *out = static_cast<uint32_t>(zm->GetZoneType(x, z));  // zone manager slot 12
    return true;
}

void RecordHistory(void* u, uint32_t historyType, int32_t value) {
    if (!Gate(fc4::Sw::FlammabilityHistory, "flammability RecordHistoryData")) return;
    cISC4City* c = Self(u)->City();
    cISC4HistoryWarehouse* h = c ? c->GetHistoryWarehouse() : nullptr;
    if (h != nullptr) h->RecordHistoryData(historyType, value);  // slot 12
}

// ---- fire (§6.8) --------------------------------------------------------------------------------

// GetDisasterLayer()->GetDisasterManager(0x621CDA33) queried for
// cISC4DisasterManagerFire, then IgniteCell(...). False when the layer, the
// manager or the interface is missing, so the core skips set_fire_cause.
//
// Ownership (§6.8): the layer and the manager are BORROWED (no AddRef; the
// layer keeps owning its managers, as in plumbing's pipe-burst and the
// budget's earthquake paths); only the QueryInterface result is ours, and it
// is released once. Run 37 released the manager too, once here and once in
// SetFireCause: the game's fire manager lost two references per ignition.
bool IgniteCell(void* u, uint32_t x, uint32_t z, bool useProbability, bool teleport, uint32_t fireIid) {
    ++Self(u)->Probe().igniteCalls;
    if (!Gate(fc4::Sw::FlammabilityFire, "flammability IgniteCell (no fire manager)")) return false;
    cISC4City* c = Self(u)->City();
    cISC4DisasterLayer* layer = c ? c->GetDisasterLayer() : nullptr;  // city slot 72, borrowed
    cISC4DisasterManager* m = layer ? layer->GetDisasterManager(kFireDisasterManager) : nullptr;  // slot 7, borrowed
    if (m == nullptr) return false;
    cISC4DisasterManagerFire* fire = Query<cISC4DisasterManagerFire>(m, kIID_cISC4DisasterManagerFire);
    if (fire == nullptr) return false;
    const bool ok = fire->IgniteCell(x, z, useProbability, teleport, fireIid);  // fire slot 5
    Drop(fire);
    if (ok) ++Self(u)->Probe().ignited;
    return ok;
}

void SetFireCause(void* u, uint32_t cause) {
    if (!Gate(fc4::Sw::FlammabilityFire, "flammability SetCause")) return;
    cISC4City* c = Self(u)->City();
    cISC4DisasterLayer* layer = c ? c->GetDisasterLayer() : nullptr;
    cISC4DisasterManager* m = layer ? layer->GetDisasterManager(kFireDisasterManager) : nullptr;  // borrowed
    if (m == nullptr) return;
    // Only the QueryInterface result is ours (released by WithQi), as in IgniteCell.
    WithQi<cISC4DisasterManagerFire>(m, kIID_cISC4DisasterManagerFire,
                                     [&](cISC4DisasterManagerFire* fire) { fire->SetCause(cause); });  // slot 9
}

// ---- messages (§9) ------------------------------------------------------------------------------

// MessagePost(msg, false) of a standard message with void1 = IGZUnknown = this
// simulator (§9).
void Post(void* u, uint32_t msgType) {
    if (msgType == kMsgSimCycleComplete) ++Self(u)->Probe().endOfCycles;
    if (!Gate(fc4::Sw::FlammabilityPosts, "flammability message post")) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* msg = FC4MakeMessage(msgType, 0, 0, 0);
    if (msgType == kMsgSimCycleComplete) {
        void* self = FromToken(Self(u)->SelfToken());
        msg->SetVoid1(self);
        msg->SetIGZUnknown(static_cast<cIGZUnknown*>(self));
    }
    ms->MessagePost(msg, false);
    msg->Release();
}

fc4_flammability_host MakeHost(cFC4FlammabilitySimulator* self, const fc4::HostGridContext* grids) {
    fc4_flammability_host h{};
    h.struct_size = sizeof(h);
    h.user = self;
    h.grids = fc4::MakeGridApi(grids);
    // city
    h.city_cell_counts = &CityCellCounts;
    h.position_to_cell = &PositionToCell;
    h.is_tutorial = &IsTutorial;
    h.tuning = &Tuning;
    // occupants
    h.occupant_type = &OccupantType;
    h.occupant_property = &OccupantProperty;
    h.occupant_bounds = &OccupantBounds;
    h.occupant_position = &OccupantPosition;
    h.occupants_in_cells = &OccupantsInCells;
    // lots
    h.occupant_lot = &OccupantLot;
    h.lot_condition = &LotCondition;
    h.lot_building = &LotBuilding;
    h.random_lot = &RandomLot;
    // other subsystems
    h.cumulative_factor = &CumulativeFactor;
    h.current_season = &CurrentSeason;
    h.is_cell_watered = &IsCellWatered;
    h.zone_type = &ZoneType;
    h.record_history = &RecordHistory;
    // fire
    h.ignite_cell = &IgniteCell;
    h.set_fire_cause = &SetFireCause;
    // messages
    h.post = &Post;
    return h;
}

}  // namespace

// ---- object -------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4FlammabilitySimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cISC4FlammabilitySimulator*>(new cFC4FlammabilitySimulator()));
}

cFC4FlammabilitySimulator::cFC4FlammabilitySimulator() {
    host_ = MakeHost(this, &gridCtx_);
    // The generator is the object's own cRZRandom, seeded 0 (§7); its double
    // path's clock reseed takes derive(S, GZCLSID) of the session seed
    // (foundations §6.3, FC4Seed.h).
    core_ = fc4_flammability_create(&host_, SelfToken(), fc4::SessionSeed());
    if (core_ == nullptr) fc4::Log("cFC4FlammabilitySimulator: core creation failed");
    fc4::Log("cFC4FlammabilitySimulator(%p) created, core %p", static_cast<void*>(this), static_cast<void*>(core_));
}

cFC4FlammabilitySimulator::~cFC4FlammabilitySimulator() {
    fc4_flammability_destroy(core_);
    core_ = nullptr;
    fc4::Log("cFC4FlammabilitySimulator destroyed");
}

bool cFC4FlammabilitySimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_IID_cISC4FlammabilitySimulator:
            *ppvObj = static_cast<cISC4FlammabilitySimulator*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        case GZIID_cIGZUnknown:
            // §1: cIGZUnknown is answered with the primary interface.
            *ppvObj = static_cast<cIGZUnknown*>(static_cast<cISC4FlammabilitySimulator*>(this));
            break;
        default:
            // §1: false, and the out pointer is left untouched. In particular
            // cIGZMessageTarget (0xE98B2F57) is not answered.
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4FlammabilitySimulator::AddRef() { return ++refCount_; }

// §4.3: at a count of 1 (or below) Shutdown, destroy, return 0 (the count is
// not decremented first).
uint32_t cFC4FlammabilitySimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

uint64_t cFC4FlammabilitySimulator::Occ(cISC4Occupant* occupant) const { return TokenOf(occupant); }

// §4.1 steps 4-5: each service only while its member is still null. The
// original AddRefs the cached globals without a null check and crashes on a
// missing service; [choice] we tolerate a missing one (§4.1 step 5).
void cFC4FlammabilitySimulator::AcquireServices() {
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
        fc4::Log("flammability: no city; services not acquired");
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
    take(ordinance_, city_->GetOrdinanceSimulator());
    take(plumbing_, city_->GetPlumbingSimulator());
    take(weather_, city_->GetWeatherSimulator());
}

// §4.2.
void cFC4FlammabilitySimulator::ReleaseServices() {
    auto drop = [](auto*& member) {
        if (member != nullptr) FC4_RELEASE(member);
        member = nullptr;
    };
    drop(weather_);
    drop(plumbing_);
    drop(ordinance_);
    drop(lotManager_);
    drop(occupantManager_);
    drop(city_);
}

void cFC4FlammabilitySimulator::Subscribe(const uint32_t* ids, size_t count, bool add) {
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

void cFC4FlammabilitySimulator::RegisterAgents(bool add) {
    cFC4ISC4Simulator* sim = city_ ? fc4::AsGame(city_->GetSimulator()) : nullptr;
    if (sim == nullptr) {
        if (add) fc4::Log("flammability: no simulator; agent not registered");
        return;
    }
    if (add) {
        const cRZBaseString name(kAgentName);
        sim->AddAgent(Target(), kAgentLevel, name, kAgentFlags);  // slot 19
        agentsRegistered_ = true;
    } else if (agentsRegistered_) {
        sim->RemoveAgentAllLevels(Target());  // slot 20
        agentsRegistered_ = false;
    }
}

void cFC4FlammabilitySimulator::LogState(const char* what) {
    fc4_flammability_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_flammability_get_state(core_, &st)) return;
    fc4::Log("flammability %s: init=%u cells=%dx%d average=%u multiplier=%.3f zoned=%d/%d candidates=%u hotspots=%u "
             "S=%d period=%d tractsPerDay=%d adjusted=%p raw=%p reentrant=%u cycles=%u endOfCycles=%u "
             "fireRolls=%u tutorialSkips=%u randomLots=%u ignited=%u/%u wateredQueries=%u watered=%u noPlumbing=%u "
             "nested=%u/%u/%u/%u",
             what, st.initialized, st.count_x, st.count_z, st.average, st.global_multiplier, st.zoned_sum,
             st.zoned_count, st.candidates, st.hotspots, st.tract_size, st.cycle_period, st.tracts_per_day,
             static_cast<void*>(FromToken(st.adjusted_grid)), static_cast<void*>(FromToken(st.raw_grid)),
             st.reentrant_calls, cycles_, probes_.endOfCycles, probes_.fireRolls, probes_.tutorialSkips,
             probes_.randomLots, probes_.ignited, probes_.igniteCalls, probes_.wateredQueries, probes_.watered,
             probes_.noPlumbing, probes_.nestedGrid, probes_.nestedFlammable, probes_.nestedHotspot,
             probes_.nestedAverage);
}

// The status line's cadence: plumbing logs at SimNewMonth, which this agent
// (SimNewDay only, §4.1 step 2) never receives, so the month is read from the
// day's julian date (data1). True on the first day of a new month; without a
// date, every 30th day.
bool cFC4FlammabilitySimulator::IsNewMonth(int32_t julian) {
    if (julian <= 0) return cycles_ % 30 == 0;
    fc4_date d{};
    fc4_date_from_julian(julian, &d);
    const bool changed = lastMonth_ != 0 && d.month != lastMonth_;
    lastMonth_ = d.month;
    return changed;
}

// ---- lifecycle (§4) -----------------------------------------------------------------------------

bool cFC4FlammabilitySimulator::Init() {
    FC4_TRACE("FlammabilitySimulator::Init");
    AcquireServices();
    {
        CoreCall call(this);
        // Steps 1, 4, 6-9 (core): tuning, counts, grids, tractsPerDay, iterator.
        fc4_flammability_init(core_);
    }
    // Steps 2, 3 (plugin): the agent and the six notifications.
    RegisterAgents(true);
    if (msgServer_ == nullptr) {
        Subscribe(kNotifications, sizeof(kNotifications) / sizeof(kNotifications[0]), true);
    }
    LogState("Init");
    return true;  // §4.1: always
}

bool cFC4FlammabilitySimulator::Shutdown() {
    FC4_TRACE("FlammabilitySimulator::Shutdown");
    // Keep this object alive while the simulator and the message server drop
    // their references (unless this already is the final Release).
    if (!destroying_) AddRef();
    {
        CoreCall call(this);
        fc4_flammability_shutdown(core_);  // §4.2 core part: grids shut down
    }
    held_.clear();
    if (msgServer_ != nullptr) {
        Subscribe(kNotifications, sizeof(kNotifications) / sizeof(kNotifications[0]), false);
    }
    RegisterAgents(false);
    ReleaseServices();
    fc4::Log("flammability Shutdown (%u cycles)", cycles_);
    if (!destroying_) Release();
    return true;  // §4.2: always
}

// ---- queries (§11.1) ----------------------------------------------------------------------------

// A query made while a call into the core is running (the core counts it as
// reentrant_calls) is also counted here per slot, so the status line shows
// who re-entered (for example the fire manager during IgniteCell).
cISC4SimGrid<uint8_t>* cFC4FlammabilitySimulator::GetFlammabilityGrid() {
    if (depth_ > 0) ++probes_.nestedGrid;
    return GridOf(fc4_flammability_grid(core_));  // slot 5, no AddRef
}

bool cFC4FlammabilitySimulator::IsCellFlammable(int x, int z) {
    if (depth_ > 0) ++probes_.nestedFlammable;
    return fc4_flammability_is_cell_flammable(core_, x, z);  // slot 6
}

bool cFC4FlammabilitySimulator::GetFlammabilityHotspot(uint8_t* value, int* x, int* z) {
    if (depth_ > 0) ++probes_.nestedHotspot;
    // §11.1 slot 7: nothing is written when it returns false.
    return fc4_flammability_hotspot(core_, value, x, z);
}

uint8_t cFC4FlammabilitySimulator::GetAverageFlammability() {
    if (depth_ > 0) ++probes_.nestedAverage;
    return fc4_flammability_average(core_);  // slot 8
}

// ---- messages (§5) ------------------------------------------------------------------------------

bool cFC4FlammabilitySimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("flammability DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    CoreCall call(this);
    fc4_flammability_message m{};
    m.msg_type = pMessage->GetType();
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    cISC4Occupant* occ = nullptr;
    cISC4Lot* lot = nullptr;
    int32_t julian = 0;  // the status line's month (SimNewDay data1)
    if (s != nullptr) {
        if (m.msg_type == kMsgSimNewDay) julian = static_cast<int32_t>(s->GetData1());
        if (m.msg_type == kMsgInsertOccupant || m.msg_type == kMsgRemoveOccupant ||
            m.msg_type == kMsgChangeOccupant) {
            // §5: the occupant is void1, known by its cISC4Occupant identity.
            occ = Query<cISC4Occupant>(static_cast<cIGZUnknown*>(s->GetVoid1()), kIID_cISC4Occupant);
            m.occupant = TokenOf(occ);
        } else if (m.msg_type == kMsgConditionChanged) {
            // §5: GetIGZUnknown() queried for cISC4Lot; released after the call.
            lot = Query<cISC4Lot>(s->GetIGZUnknown(), kIID_cISC4Lot);
            m.lot = TokenOf(lot);
        }
    }
    // A message delivered while another call into the core is running is queued
    // by the core and handled when that call returns: keep its objects alive
    // until then.
    if (depth_ > 1) {
        cIGZUnknown* keep[] = {reinterpret_cast<cIGZUnknown*>(occ), reinterpret_cast<cIGZUnknown*>(lot)};
        for (cIGZUnknown* k : keep) {
            if (k == nullptr) continue;
            k->AddRef();
            Hold(std::shared_ptr<void>(k, [](void* p) { static_cast<cIGZUnknown*>(p)->Release(); }));
        }
    }
    const bool r = fc4_flammability_do_message(core_, &m);
    Drop(lot);
    Drop(occ);
    Drop(s);
    // SimNewDay drives the daily refresh; EndOfCycle posts the cycle complete
    // (§9). Count the days for the status and Shutdown lines; a status line
    // at each month start, as plumbing's.
    if (m.msg_type == kMsgSimNewDay) {
        ++cycles_;
        if (IsNewMonth(julian)) LogState("month");
    }
    return r;  // §5: always true
}

// ---- persistence (§10) --------------------------------------------------------------------------

bool cFC4FlammabilitySimulator::Write(cIGZOStream& s) {
    FC4_TRACE("FlammabilitySimulator::Write");
    CoreCall call(this);
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_flammability_write(core_, a.Get());
    fc4::Log("flammability Write -> %d", ok ? 1 : 0);
    return ok;
}

bool cFC4FlammabilitySimulator::Read(cIGZIStream& s) {
    FC4_TRACE("FlammabilitySimulator::Read");
    CoreCall call(this);
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_flammability_read(core_, a.Get());
    fc4::Log("flammability Read -> %d", ok ? 1 : 0);
    LogState("Read");
    return ok;
}

uint32_t cFC4FlammabilitySimulator::GetGZCLSID() { return kFC4_CLSID_cSC4FlammabilitySimulator; }

// ---- test seam ----------------------------------------------------------------------------------

intptr_t FlammabilityTestFirePath(cISC4City* city) {
    if (city == nullptr) return 1;
    cFC4FlammabilitySimulator* s = new cFC4FlammabilitySimulator();
    s->AddRef();
    s->TestSetCity(city);
    // As the core drives it: SetCause only after a successful IgniteCell.
    intptr_t r = 0;
    if (IgniteCell(s, 6, 12, true, false, 0)) {
        SetFireCause(s, 5);
    } else {
        r = 2;
    }
    s->TestSetCity(nullptr);  // borrowed: ReleaseServices must not release it
    s->Release();
    return r;
}
