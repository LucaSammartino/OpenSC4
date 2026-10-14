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

#include "cFC4Flora.h"

#include "FC4Checked.h"
#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4Watchdog.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "SC4NewCitySpecification.h"
#include "SC4Point.h"
#include "SC4Rect.h"
#include "cGZPersistResourceKey.h"
#include "cIGZApp.h"
#include "cIGZCOM.h"
#include "cIGZCheatCodeManager.h"
#include "cIGZCommandParameterSet.h"
#include "cIGZCommandServer.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZPersistResourceKeyFilter.h"
#include "cIGZPersistResourceKeyList.h"
#include "cIGZPersistResourceManager.h"
#include "cIGZString.h"
#include "cIGZVariant.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4FloraOccupant.h"
#include "cISC4LotManager.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4Simulator.h"
#include "cISC4WeatherSimulator.h"
#include "cISCPropertyHolder.h"
#include "cISCResExemplar.h"
#include "cISTETerrain.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <algorithm>
#include <cstdio>
#include <cstring>
#include <map>
#include <new>
#include <string>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4NetworkOccupant = 0xA821EF94;
constexpr uint32_t kIID_cIGZCheatCodeManager = 0xA1085722;  // as used by the SDK examples
/// cSC4FloraOccupant (§3.4, §13.2: create flora occupants through this class).
constexpr uint32_t kCLSID_cSC4FloraOccupant = 0xA9C05C85;

// Exemplars (§3.3, §4.1 step 2, §7).
constexpr uint32_t kExemplarType = 0x6534284A;
constexpr uint32_t kPropExemplarType = 0x10;
constexpr uint32_t kFloraExemplarType = 0x0F;
constexpr uint32_t kTuningGroup = 0x7A4A8458;
constexpr uint32_t kTuningInstance = 0x1A2FDB6A;

// Messages (§5).
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgInsertOccupant = 0x99EF1142;
constexpr uint32_t kMsgRemoveOccupant = 0x99EF1143;
constexpr uint32_t kMsgChangeOccupant = 0x99EF1145;
constexpr uint32_t kMsgCheat = 0x230E27AC;
constexpr uint32_t kMsgPlaceFlora = 0xAB99A64B;
const char kPlaceFloraName[] = "PlaceFlora";  // §4.1 step 3 (the command server keeps the pointer)

// Occupant types (§3.4, §6.4.1).
constexpr uint32_t kTypeFlora = 0x74758926;
constexpr uint32_t kTypeBuilding = 0x278128A0;
constexpr uint32_t kTypeLandfill = 0x6A11BD2B;
constexpr uint32_t kTypeProp = 0xA823821E;
constexpr uint32_t kTypeNetwork = 0xC772BF98;
constexpr uint32_t kBlockingNetworkFlags = 0x1F4F;

cFC4FloraSimulator* Self(void* u) { return static_cast<cFC4FloraSimulator*>(u); }
cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }

bool Gate(fc4::Sw s, const char* what) {
    if (fc4::On(s)) return true;
    fc4::NoteSkipped(s, what);
    return false;
}

void LogOnce(bool& done, const char* text) {
    if (done) return;
    done = true;
    fc4::Log("%s", text);
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

// The application's cISC4App (one reference, released after `f`).
template <class F>
void WithApp(F f) {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return;
    }
    FC4_ACQUIRED(sc4, "QueryInterface");
    f(sc4);
    FC4_RELEASE(sc4);
}

// ---- the record blocks (docs/abi/layouts.md) ------------------------------------------------------

uint32_t g_blocks = 0;  // blocks built (process lifetime)

// The registry's create_block (fc4_flora_registry_host): a block with the
// record's values holding `exemplar` (its reference passes to the block), one
// reference of ours for the life of the process (§13.2 [choice]: no game
// Release ever sees a count of 1). [choice] The family vector stays null:
// only the game's Release reads it, and Release never frees our blocks.
uint64_t CreateBlock(void*, const fc4_flora_record* rec, uint64_t exemplar) {
    if (rec == nullptr || !Gate(fc4::Sw::FloraRecords, "flora record block (none: GetStaticFloraData returns null)")) {
        if (exemplar != 0) FC4_RELEASE(FromToken(exemplar));  // create_block exists: ours to release
        return 0;
    }
    FC4FloraBlock* b = new (std::nothrow) FC4FloraBlock();
    if (b == nullptr) {
        if (exemplar != 0) FC4_RELEASE(FromToken(exemplar));
        return 0;
    }
    std::memset(static_cast<void*>(b), 0, sizeof(*b));
    b->type = rec->type_id;
    b->next = rec->next_type;
    b->seed = rec->seed_type;
    b->water = rec->water != 0 ? 1 : 0;
    std::memcpy(b->floats, rec->floats, sizeof(b->floats));
    b->clusterRadius = rec->cluster_radius;
    b->clusterHeight = rec->cluster_height;
    std::memcpy(b->table, rec->table, sizeof(b->table));
    b->unnamed = rec->unnamed;
    b->plopCost = rec->plop_cost;
    b->exemplar = reinterpret_cast<cISCResExemplar*>(FromToken(exemplar));
    b->flags = rec->flags;
    b->refCount = 1;
    ++g_blocks;
    return TokenOf(b);
}

// §6.11: a seed type computed after the record's block was built.
void SetBlockSeed(void*, uint64_t block, uint32_t seed) {
    if (block != 0) reinterpret_cast<FC4FloraBlock*>(FromToken(block))->seed = seed;
}

void ReleaseExemplar(void*, uint64_t exemplar) {
    if (exemplar != 0) FC4_RELEASE(FromToken(exemplar));
}

// ---- the exemplar enumeration (§4.1 step 2) --------------------------------------------------------

// A key filter for the resource manager: exemplars only. Lives on the stack
// of the enumeration; references are counted but never delete it.
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
};

// ---- city, clock, resources ------------------------------------------------------------------------

bool CityExists(void* u) { return Self(u)->City() != nullptr; }

void CitySize(void* u, float* x, float* z) {
    cISC4City* c = Self(u)->City();
    if (x != nullptr) *x = c ? c->SizeX() : 0.0f;  // slot 89, ST0
    if (z != nullptr) *z = c ? c->SizeZ() : 0.0f;  // slot 90
}

void CityCellCounts(void* u, int32_t* x, int32_t* z) {
    cISC4City* c = Self(u)->City();
    if (x != nullptr) *x = c ? static_cast<int32_t>(c->CellCountX()) : 0;  // slots 93, 94
    if (z != nullptr) *z = c ? static_cast<int32_t>(c->CellCountZ()) : 0;
}

bool CityEstablished(void* u) {
    cISC4City* c = Self(u)->City();
    return c != nullptr && c->GetEstablished();  // slot 22, AL
}

cFC4ISC4Simulator* SimOf(void* u) {
    cISC4Simulator* s = Self(u)->Simulator();
    if (s == nullptr && Self(u)->City() != nullptr) s = Self(u)->City()->GetSimulator();  // borrowed
    return s ? fc4::AsGame(s) : nullptr;
}

uint32_t SimDateNumber(void* u) {
    cFC4ISC4Simulator* s = SimOf(u);
    return s ? static_cast<uint32_t>(s->GetSimDateNumber()) : 0;  // simulator slot 8
}

fc4_props* Tuning(void*) {
    fc4::props::Exemplar ex;  // obeys exemplar_loads
    if (!ex.Load(kExemplarType, kTuningGroup, kTuningInstance)) {
        fc4::Log("flora: tuning exemplar unavailable; the defaults stay (§7)");
        return nullptr;
    }
    return fc4::props::ToCoreAll(ex.Holder());
}

void FloraExemplars(void*, fc4_flora_exemplar_visitor visit, void* ctx) {
    fc4::FloraForEachExemplar(true, true, visit, ctx);
}

// §4.1 step 12, §10.3: the obliterated city's segment is
// SC4NewCitySpecification::obliteratedCityDBSegment (SDK, +0x204). Reading
// its records needs cIGZPersistDBSegment, which is not in docs/abi: the
// restore is not done [spec question], logged whenever a segment is there.
uint64_t ObliteratedSegment(void*) {
    cIGZPersistDBSegment* seg = nullptr;
    WithApp([&](cISC4App* app) {
        SC4NewCitySpecification* spec = app->GetNewCitySpecification();  // slot 39
        if (spec != nullptr) seg = spec->obliteratedCityDBSegment;
    });
    if (seg != nullptr) {
        fc4::Log("flora: the obliterated city's segment %p is NOT restored: cIGZPersistDBSegment's record reads are "
                 "not in docs/abi (flora.md §10.3; spec question); its trees are not re-created",
                 static_cast<void*>(seg));
    }
    return 0;
}

bool g_loggedShutdownCallback = false;

// §4.1 step 2: cISC4App slot 69 RegisterShutdownCallbackFunction(cb, 0).
// [choice, spec question] Not registered: the callback releases the records,
// and ours are kept for the life of the process (§13.2), so it would only
// empty the registry while the game may still read it; the call's timing
// against this DLL's unload is not documented.
void RegisterAppShutdownCallback(void*) {
    LogOnce(g_loggedShutdownCallback,
            "flora: the application shutdown callback (§4.1 step 2) is not registered [choice]: the records are kept "
            "for the life of the process (§13.2)");
}

void SubscribeIds(void* u, const uint32_t* ids, size_t count) { Self(u)->Subscribe(ids, count, true); }
void UnsubscribeIds(void* u, const uint32_t* ids, size_t count) { Self(u)->Subscribe(ids, count, false); }

void RegisterCommandCb(void* u, uint32_t id, const uint8_t* name, size_t len) {
    if (!Gate(fc4::Sw::FloraCheats, "flora PlaceFlora command registration")) return;
    const std::string n(reinterpret_cast<const char*>(name), len);
    Self(u)->RegisterCommand(id, n.c_str());
}

void UnregisterCommandCb(void* u, uint32_t id) { Self(u)->UnregisterCommand(id); }

void AddAgentCb(void* u, uint32_t level, const uint8_t* name, size_t len, uint32_t flags) {
    const std::string n(reinterpret_cast<const char*>(name), len);
    Self(u)->AddAgent(level, n.c_str(), flags);
}

void RemoveAgentCb(void* u) { Self(u)->RemoveAgent(); }
void AcquireServicesCb(void* u) { Self(u)->AcquireServices(); }
void ReleaseServicesCb(void* u) { Self(u)->ReleaseServices(); }

void RegisterCheatCb(void* u, uint32_t id, const uint8_t* name, size_t len) {
    if (!Gate(fc4::Sw::FloraCheats, "flora cheat registration")) return;
    const std::string n(reinterpret_cast<const char*>(name), len);
    Self(u)->RegisterCheat(id, n.c_str());
}

void UnregisterCheatCb(void* u, uint32_t id) { Self(u)->UnregisterCheat(id); }

// §9: MessagePost(msg, false) with void1 = IGZUnknown = this simulator's
// primary interface, data2 as given (FloraTypesUpdated: the map size).
void Post(void* u, uint32_t msgType, uint32_t data2) {
    if (!Gate(fc4::Sw::FloraPosts, "flora message post")) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* msg = FC4MakeMessage(msgType, 0, static_cast<intptr_t>(data2), 0);
    cISC4FloraSimulator* self = Self(u)->Primary();
    msg->SetVoid1(self);
    msg->SetIGZUnknown(self);
    ms->MessagePost(msg, false);
    msg->Release();
}

// ---- terrain, lots, weather ------------------------------------------------------------------------

bool TerrainInitialized(void* u) {
    if (!Gate(fc4::Sw::FloraTerrain, "flora terrain IsInitialized (false)")) return false;
    cISTETerrain* t = Self(u)->Terrain();
    return t != nullptr && t->IsInitialized();  // slot 5, AL
}

float Altitude(void* u, float x, float z) {
    if (!Gate(fc4::Sw::FloraTerrain, "flora terrain GetAltitude (0)")) return 0.0f;
    cISTETerrain* t = Self(u)->Terrain();
    return t ? t->GetAltitude(x, z) : 0.0f;  // compiled slot 13, ST0
}

float SeaLevel(void* u) {
    if (!Gate(fc4::Sw::FloraTerrain, "flora terrain GetSeaLevel (0)")) return 0.0f;
    cISTETerrain* t = Self(u)->Terrain();
    return t ? t->GetSeaLevel() : 0.0f;  // slot 29, ST0
}

void FaceNormal(void* u, float x, float z, float* out3) {
    if (out3 == nullptr || !Gate(fc4::Sw::FloraTerrain, "flora terrain GetFaceNormal (zero)")) return;
    cISTETerrain* t = Self(u)->Terrain();
    if (t == nullptr) return;
    cS3DVector3 n(0.0f, 0.0f, 0.0f);  // §6.4: the output is initialised to 0
    t->GetFaceNormal(x, z, n);       // slot 26
    out3[0] = n.fX;
    out3[1] = n.fY;
    out3[2] = n.fZ;
}

// Lot manager slot 18 (§12: a float in ST0, SDK error fixed in FC4Interfaces.h).
float LotSavvyAltitude(void* u, float x, float z, bool* onLot) {
    if (!Gate(fc4::Sw::FloraTerrain, "flora GetLotSavvyTerrainAltitude (0)")) return 0.0f;
    cISC4LotManager* lm = Self(u)->LotManager();
    bool on = false;
    const float h = lm ? fc4::AsGame(lm)->GetLotSavvyTerrainAltitude(x, z, &on) : 0.0f;
    if (onLot != nullptr) *onLot = on;
    return h;
}

// Weather slot 19 (§6.2). [choice, §14 quirk 15] No weather simulator: 0.
// [spec question] Read as the verified declaration returns it (AL, uint8_t);
// the moisture types are 0..15.
uint32_t MoistureType(void* u, float x, float z) {
    if (!Gate(fc4::Sw::FloraWeather, "flora GetMoistureType (0)")) return 0;
    cISC4WeatherSimulator* w = Self(u)->Weather();
    return w ? static_cast<uint32_t>(fc4::AsGame(w)->GetMoistureType(x, z)) : 0u;
}

// ---- occupants -------------------------------------------------------------------------------------

uint32_t OccupantType(void*, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::FloraOccupants, "flora occupant GetType (0)")) return 0;
    return static_cast<uint32_t>(OccOf(occ)->GetType());  // slot 7
}

bool OccupantHasProperty(void*, uint64_t occ, uint32_t id) {
    if (occ == 0 || !Gate(fc4::Sw::FloraOccupants, "flora occupant HasProperty (absent)")) return false;
    return fc4::props::Has(OccOf(occ)->AsPropertyHolder(), id);  // slot 6 (borrowed), then HasProperty
}

void OccupantPosition(void*, uint64_t occ, float* out3) {
    if (out3 == nullptr || occ == 0 || !Gate(fc4::Sw::FloraOccupants, "flora occupant GetPosition")) return;
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);  // slot 8
    out3[0] = v.fX;
    out3[1] = v.fY;
    out3[2] = v.fZ;
}

void SetOccupantPosition(void*, uint64_t occ, const float* pos3) {
    if (pos3 == nullptr || occ == 0 || !Gate(fc4::Sw::FloraOccupants, "flora occupant SetPosition")) return;
    const cS3DVector3 v(pos3[0], pos3[1], pos3[2]);
    OccOf(occ)->SetPosition(&v);  // slot 9
}

void OccupantBoundingBox(void*, uint64_t occ, float* min3, float* max3) {
    if (min3 == nullptr || max3 == nullptr || occ == 0 ||
        !Gate(fc4::Sw::FloraOccupants, "flora occupant GetBoundingBox")) {
        return;
    }
    cS3DVector3 a, b;
    OccOf(occ)->GetBoundingBox(&a, &b);  // slot 10
    min3[0] = a.fX;
    min3[1] = a.fY;
    min3[2] = a.fZ;
    max3[0] = b.fX;
    max3[1] = b.fY;
    max3[2] = b.fZ;
}

bool OccupantInManager(void*, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::FloraOccupants, "flora occupant IsFlagSet(1) (false)")) return false;
    return OccOf(occ)->IsFlagSet(1);  // slot 32
}

void SetOccupantInManager(void*, uint64_t occ, bool on) {
    if (occ == 0 || !Gate(fc4::Sw::FloraOccupants, "flora occupant SetFlag / ClearFlag(1)")) return;
    if (on) {
        OccOf(occ)->SetFlag(1);  // slot 29
    } else {
        OccOf(occ)->ClearFlag(1);  // slot 31
    }
}

bool NetworkFlags(void*, uint64_t occ, uint32_t* out) {
    if (occ == 0 || !Gate(fc4::Sw::FloraOccupants, "flora GetNetworkFlag (not a network)")) return false;
    return WithQi<cISC4NetworkOccupant>(FromToken(occ), kIID_cISC4NetworkOccupant, [&](cISC4NetworkOccupant* n) {
        if (out != nullptr) *out = n->GetNetworkFlag();  // slot 12
    });
}

bool FloraOccupantValues(void*, uint64_t occ, fc4_flora_occupant* out) {
    if (occ == 0 || !Gate(fc4::Sw::FloraOccupants, "flora occupant values (not flora)")) return false;
    return WithQi<cISC4FloraOccupant>(FromToken(occ), GZIID_cISC4FloraOccupant, [&](cISC4FloraOccupant* f) {
        if (out == nullptr) return;
        out->flora_type = f->GetFloraType();                            // slot 4
        out->birth_date = f->GetBirthDate();                            // slot 6
        out->last_seeding_date = f->GetLastSeedingDate();               // slot 8
        out->state = static_cast<int8_t>(f->GetState());                // slot 10 (sign-extended byte)
        out->orientation = static_cast<uint8_t>(f->GetOrientation());  // slot 12
    });
}

template <class F>
void SetFlora(uint64_t occ, const char* what, F f) {
    if (occ == 0 || !Gate(fc4::Sw::FloraOccupants, what)) return;
    WithQi<cISC4FloraOccupant>(FromToken(occ), GZIID_cISC4FloraOccupant, f);
}

void SetFloraType(void*, uint64_t occ, uint32_t v) {
    SetFlora(occ, "flora SetFloraType", [&](cISC4FloraOccupant* f) { f->SetFloraType(v); });  // slot 5
}
void SetBirthDate(void*, uint64_t occ, uint32_t v) {
    SetFlora(occ, "flora SetBirthDate", [&](cISC4FloraOccupant* f) { f->SetBirthDate(v); });  // slot 7
}
void SetLastSeedingDate(void*, uint64_t occ, uint32_t v) {
    SetFlora(occ, "flora SetLastSeedingDate", [&](cISC4FloraOccupant* f) { f->SetLastSeedingDate(v); });  // 9
}
void SetFloraState(void*, uint64_t occ, int8_t v) {
    SetFlora(occ, "flora SetState", [&](cISC4FloraOccupant* f) { f->SetState(static_cast<int32_t>(v)); });  // 11
}
void SetFloraOrientation(void*, uint64_t occ, uint32_t v) {
    SetFlora(occ, "flora SetOrientation", [&](cISC4FloraOccupant* f) { f->SetOrientation(v); });  // slot 13
}

// Slot 14 SetStaticFloraData: the occupant takes its own reference on the
// block (+0x170, layouts.md).
void SetFloraRecord(void*, uint64_t occ, uint64_t block) {
    if (occ == 0 || !Gate(fc4::Sw::FloraRecords, "flora SetStaticFloraData (no record set)")) return;
    SetFlora(occ, "flora SetStaticFloraData", [&](cISC4FloraOccupant* f) {
        f->SetStaticFloraData(reinterpret_cast<cSC4StaticFloraData*>(FromToken(block)));
    });
}

// §9.1: the occupant's own PostOccupantMessage(0x99EF1145, 0) (slot 15).
void PostOccupantChange(void*, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::FloraOccupants, "flora ChangeOccupant post")) return;
    OccOf(occ)->PostOccupantMessage(kMsgChangeOccupant, 0);
}

// §6.5 step 3, §13.2: a cSC4FloraOccupant through GZCOM (class 0xA9C05C85)
// as its cISC4Occupant; one reference, ours (the core releases it).
uint64_t CreateFloraOccupant(void*) {
    if (!Gate(fc4::Sw::FloraCreate, "flora occupant creation (none: nothing planted)")) return 0;
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (com == nullptr || !com->GetClassObject(kCLSID_cSC4FloraOccupant, kIID_cISC4Occupant, &p) || p == nullptr) {
        static bool logged = false;
        LogOnce(logged, "flora: WARNING could not create a flora occupant through GZCOM (class 0xA9C05C85)");
        return 0;
    }
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(p), "GetClassObject");
    return TokenOf(p);
}

void InitOccupant(void*, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::FloraCreate, "flora occupant Init")) return;
    OccOf(occ)->Init();  // slot 3
}

// The burnt list's reference (§6.9).
void AddRefOccupant(void*, uint64_t occ) {
    if (cIGZUnknown* o = FromToken(occ)) FC4_ADDREF(o);
}

// Every reference the core releases was taken by an audited site: a created
// occupant (GetClassObject), the burnt list's AddRef, a reference resolved
// from a save. After RemoveOccupant (§6.8) ours may be the last one, which is
// the expected free of an occupant the game no longer holds.
void ReleaseOccupant(void*, uint64_t occ) {
    if (cIGZUnknown* o = FromToken(occ)) FC4_RELEASE_CREATED(o);
}

// A burnt tree read from a save (§10.1 field 17) comes back as its
// cISC4Occupant (IID 0x26BB767E). The stream handed over one reference; the
// core keeps one on the occupant's identity.
uint64_t ResolveRef(void*, uint64_t t, uint32_t iid) {
    cIGZUnknown* u = FromToken(t);
    if (u == nullptr) return 0;
    if (fc4::IsOurObject(u)) {
        u->AddRef();
        return t;
    }
    if (iid != kIID_cISC4Occupant) return t;
    cISC4Occupant* occ = Query<cISC4Occupant>(u, kIID_cISC4Occupant);
    FC4_RELEASE(u);
    return TokenOf(occ);
}

// ---- the occupant manager --------------------------------------------------------------------------

bool HasOccupantManager(void* u) { return Self(u)->OccupantManager() != nullptr; }

bool InsertOccupant(void* u, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::FloraManager, "flora InsertOccupant (refused)")) return false;
    cISC4OccupantManager* om = Self(u)->OccupantManager();
    return om != nullptr && om->InsertOccupant(OccOf(occ), 0);  // slot 20
}

bool RemoveOccupant(void* u, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::FloraManager, "flora RemoveOccupant (not removed)")) return false;
    cISC4OccupantManager* om = Self(u)->OccupantManager();
    return om != nullptr && om->RemoveOccupant(OccOf(occ), true, 0);  // slot 21
}

void MoveOccupant(void* u, uint64_t occ) {
    if (occ == 0 || !Gate(fc4::Sw::FloraManager, "flora MoveOccupant")) return;
    cISC4OccupantManager* om = Self(u)->OccupantManager();
    if (om != nullptr) om->MoveOccupant(OccOf(occ), true);  // slot 24
}

using OccList = SC4List<cISC4Occupant*>;

// Hands every occupant of `list` to `cb`, if `keep(o)`, and keeps the list
// (and its references) until the outermost call into the core returns.
template <class K>
void Deliver(void* u, std::shared_ptr<OccList> list, fc4_token_callback cb, void* ctx, K keep) {
    for (cISC4Occupant* o : *list) {
        if (o != nullptr && keep(o)) cb(ctx, TokenOf(o));
    }
    Self(u)->Hold(std::move(list));
}

// GetOccupantsByBBox(list, min, max, type, -1) (typed overload, compiled slot
// 37; one reference per occupant in an SC4List).
//
// [choice, spec question] The blocking filter of §6.4.1 is an object passed to
// the FILTER overload (slot 36); cISC4OccupantFilter is not in
// docs/abi/interfaces.tsv, so its vtable is not verified. The typed overload is
// called once per accepted type (flora, building, landfill, prop, network)
// and network occupants are kept when GetNetworkFlag() & 0x1F4F is non-zero:
// an occupant has exactly one type, so the union is what the filter accepts.
void OccupantsInBbox(void* u, const float* min3, const float* max3, uint32_t type, bool blocking,
                     fc4_token_callback cb, void* ctx) {
    if (min3 == nullptr || max3 == nullptr || cb == nullptr) return;
    if (!Gate(fc4::Sw::FloraQueries, "flora occupant box query (no occupants)")) return;
    cISC4OccupantManager* om = Self(u)->OccupantManager();
    if (om == nullptr) return;
    auto query = [&](uint32_t t) {
        auto list = std::make_shared<OccList>();
        om->GetOccupantsByBBox(*list, min3, max3, t, 0xFFFFFFFFu);  // slot 37
        return list;
    };
    if (!blocking) {
        Deliver(u, query(type), cb, ctx, [](cISC4Occupant*) { return true; });
        return;
    }
    for (uint32_t t : {kTypeFlora, kTypeBuilding, kTypeLandfill, kTypeProp}) {
        Deliver(u, query(t), cb, ctx, [](cISC4Occupant*) { return true; });
    }
    Deliver(u, query(kTypeNetwork), cb, ctx, [&](cISC4Occupant* o) {
        uint32_t flags = 0;
        return NetworkFlags(u, TokenOf(o), &flags) && (flags & kBlockingNetworkFlags) != 0;
    });
}

// GetOccupantsByStandardCityCells(list, &min, &max, type, -1) (compiled slot
// 41); rect NULL = null corners, the whole city (§6.10.4).
void OccupantsInCells(void* u, const fc4_cell_rect* r, uint32_t type, fc4_token_callback cb, void* ctx) {
    if (cb == nullptr || !Gate(fc4::Sw::FloraQueries, "flora occupant cell query (no occupants)")) return;
    cISC4OccupantManager* om = Self(u)->OccupantManager();
    if (om == nullptr) return;
    SC4Point<int> mn, mx;
    if (r != nullptr) {
        mn = SC4Point<int>(r->x1, r->z1);
        mx = SC4Point<int>(r->x2, r->z2);
    }
    auto list = std::make_shared<OccList>();
    om->GetOccupantsByStandardCityCells(*list, r ? &mn : nullptr, r ? &mx : nullptr, type, 0xFFFFFFFFu);
    Deliver(u, std::move(list), cb, ctx, [](cISC4Occupant*) { return true; });
}

// GetOccupantsByOccupantManagerCells(list, null, null, type, -1) (compiled
// slot 39): the whole city (§6.10.4; the obliteration save, §10.3).
void OccupantsInManagerCells(void* u, uint32_t type, fc4_token_callback cb, void* ctx) {
    if (cb == nullptr || !Gate(fc4::Sw::FloraQueries, "flora occupant manager-cell query (no occupants)")) return;
    cISC4OccupantManager* om = Self(u)->OccupantManager();
    if (om == nullptr) return;
    auto list = std::make_shared<OccList>();
    om->GetOccupantsByOccupantManagerCells(*list, nullptr, nullptr, type, 0xFFFFFFFFu);
    Deliver(u, std::move(list), cb, ctx, [](cISC4Occupant*) { return true; });
}

fc4_flora_host MakeHost(cFC4FloraSimulator* self) {
    fc4_flora_host h{};
    h.struct_size = sizeof(h);
    h.user = self;
    // city, clock, resources
    h.city_exists = &CityExists;
    h.city_size = &CitySize;
    h.city_cell_counts = &CityCellCounts;
    h.city_established = &CityEstablished;
    h.sim_date_number = &SimDateNumber;
    h.tuning = &Tuning;
    h.flora_exemplars = &FloraExemplars;
    h.obliterated_segment = &ObliteratedSegment;
    // framework
    h.register_app_shutdown_callback = &RegisterAppShutdownCallback;
    h.subscribe = &SubscribeIds;
    h.unsubscribe = &UnsubscribeIds;
    h.register_command = &RegisterCommandCb;
    h.unregister_command = &UnregisterCommandCb;
    h.add_agent = &AddAgentCb;
    h.remove_agent = &RemoveAgentCb;
    h.acquire_services = &AcquireServicesCb;
    h.release_services = &ReleaseServicesCb;
    h.register_cheat = &RegisterCheatCb;
    h.unregister_cheat = &UnregisterCheatCb;
    h.post = &Post;
    // terrain, lots, weather
    h.terrain_initialized = &TerrainInitialized;
    h.altitude = &Altitude;
    h.sea_level = &SeaLevel;
    h.face_normal = &FaceNormal;
    h.lot_savvy_altitude = &LotSavvyAltitude;
    h.moisture_type = &MoistureType;
    // occupants
    h.occupant_type = &OccupantType;
    h.occupant_has_property = &OccupantHasProperty;
    h.occupant_position = &OccupantPosition;
    h.set_occupant_position = &SetOccupantPosition;
    h.occupant_bounding_box = &OccupantBoundingBox;
    h.occupant_in_manager = &OccupantInManager;
    h.set_occupant_in_manager = &SetOccupantInManager;
    h.network_flags = &NetworkFlags;
    h.flora_occupant = &FloraOccupantValues;
    h.set_flora_type = &SetFloraType;
    h.set_birth_date = &SetBirthDate;
    h.set_last_seeding_date = &SetLastSeedingDate;
    h.set_flora_state = &SetFloraState;
    h.set_flora_orientation = &SetFloraOrientation;
    h.set_flora_record = &SetFloraRecord;
    h.post_occupant_change = &PostOccupantChange;
    h.create_flora_occupant = &CreateFloraOccupant;
    h.init_occupant = &InitOccupant;
    h.add_ref = &AddRefOccupant;
    h.release = &ReleaseOccupant;
    h.resolve_ref = &ResolveRef;
    // occupant manager
    h.has_occupant_manager = &HasOccupantManager;
    h.insert_occupant = &InsertOccupant;
    h.remove_occupant = &RemoveOccupant;
    h.move_occupant = &MoveOccupant;
    h.occupants_in_bbox = &OccupantsInBbox;
    h.occupants_in_cells = &OccupantsInCells;
    h.occupants_in_manager_cells = &OccupantsInManagerCells;
    // The obliteration segment's record I/O (§10.3) is not in docs/abi: no
    // callbacks (SaveObliterated never calls the core, the restore never runs).
    h.segment_write_u32 = nullptr;
    h.segment_read_u32 = nullptr;
    h.segment_write_occupant = nullptr;
    h.segment_read_flora_occupant = nullptr;
    return h;
}

// PlaceFlora's parameters (§5): param 0 as u32, params 1 and 2 as floats,
// through the command parameter set's slot 5 (§12; SDK operator[]) and the
// variant's verified slots 24 / 36 (FC4Interfaces.h).
size_t ReadPlaceFloraParams(cIGZCommandParameterSet* ps, fc4_flora_param* out) {
    if (ps == nullptr || !Gate(fc4::Sw::FloraCheats, "flora PlaceFlora parameters (none)")) return 0;
    size_t n = 0;
    for (uint32_t i = 0; i < 3; ++i) {
        cIGZVariant* v = ps->operator[](i);  // slot 5
        if (v == nullptr) break;
        fc4_flora_param p{};
        if (i == 0) {
            p.as_u32 = fc4::AsGame(v)->GetValUint32();  // slot 24
        } else {
            p.as_f32 = fc4::AsGame(v)->GetValFloat32();  // slot 36, ST0
        }
        out[n++] = p;
    }
    return n;
}

// The month line's census: every flora occupant of the city (slot 41 with
// null corners, §6.10.4) by state, and the number of types.
struct Census {
    bool taken = false;
    uint32_t total = 0, notFlora = 0, other = 0, types = 0;
    uint32_t stage[5] = {};
};

Census TakeCensus(cISC4OccupantManager* om) {
    Census c;
    if (om == nullptr || !fc4::On(fc4::Sw::FloraQueries) || !fc4::On(fc4::Sw::FloraOccupants)) return c;
    OccList list;
    om->GetOccupantsByStandardCityCells(list, nullptr, nullptr, kTypeFlora, 0xFFFFFFFFu);
    std::map<uint32_t, uint32_t> types;
    for (cISC4Occupant* o : list) {
        if (o == nullptr) continue;
        ++c.total;
        const bool flora = WithQi<cISC4FloraOccupant>(o, GZIID_cISC4FloraOccupant, [&](cISC4FloraOccupant* f) {
            const int32_t s = static_cast<int8_t>(f->GetState());
            if (s >= 0 && s <= 4) {
                ++c.stage[s];
            } else {
                ++c.other;
            }
            ++types[f->GetFloraType()];
        });
        if (!flora) ++c.notFlora;
    }
    c.types = static_cast<uint32_t>(types.size());
    c.taken = true;
    return c;
}

uint32_t WallClockSeed() { return static_cast<uint32_t>(GetTickCount64() ^ GetCurrentProcessId()); }

}  // namespace

// ---- process-wide ----------------------------------------------------------------------------------

namespace fc4 {

fc4_flora_registry* FloraRegistry() {
    static fc4_flora_registry* const r = [] {
        fc4_flora_registry_host h{};
        h.struct_size = sizeof(h);
        h.user = nullptr;
        h.create_block = &CreateBlock;
        h.set_block_seed = &SetBlockSeed;
        h.release_exemplar = &ReleaseExemplar;
        fc4_flora_registry* p = fc4_flora_registry_create(&h);
        Log("flora: the process-wide flora registry %p created (§3.2)", static_cast<void*>(p));
        return p;
    }();
    return r;
}

std::recursive_mutex& FloraLock() {
    static std::recursive_mutex m;
    return m;
}

// §4.1 step 2. [choice, spec question] The key list comes from
// GetAvailableResourceList(&list, filter) with an exemplar-type filter, as the
// demand glue does (run 42), not slot 18 GetAvailableResourceListForType: the
// order is the same (built segment by segment), and this list is proven to be
// a temporary of ours. Keys are read by index (Size / GetKey), the list's
// order, instead of EnumKeys' callback.
void FloraForEachExemplar(bool keepRefs, bool gated, fc4_flora_exemplar_visitor visit, void* ctx) {
    if (visit == nullptr) return;
    if (gated && (!Gate(Sw::FloraExemplars, "flora exemplar enumeration (no flora types)") ||
                  !Gate(Sw::ExemplarLoads, "flora exemplar enumeration (no flora types)"))) {
        return;
    }
    cIGZPersistResourceManagerPtr rm;
    if (!rm) {
        Log("flora: no resource manager; no flora exemplars");
        return;
    }
    ExemplarKeyFilter filter;
    FC4_OUT(cIGZPersistResourceKeyList*, list, "cIGZPersistResourceKeyList* (GetAvailableResourceList out pointer)");
    rm->GetAvailableResourceList(list.ptr(), &filter);
    list.Check();
    cIGZPersistResourceKeyList* l = list.get();
    if (l == nullptr) {
        Log("flora: WARNING the resource manager returned no key list; no flora exemplars");
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
        if (k.type != kExemplarType) continue;
        FC4_OUT(cISCResExemplar*, out, "cISCResExemplar* (GetResource out pointer)");
        const bool ok = rm->GetResource(k, GZIID_cISCResExemplar, reinterpret_cast<void**>(out.ptr()), 0, nullptr);
        out.Check();
        cISCResExemplar* ex = out.get();
        if (!ok || ex == nullptr) continue;
        FC4_ACQUIRED(ex, "GetResource");
        const cISCPropertyHolder* h = ex->AsISCPropertyHolder();
        std::vector<uint32_t> t;
        if (h == nullptr || !props::GetU32s(h, kPropExemplarType, t) || t.empty() || t[0] != kFloraExemplarType) {
            FC4_RELEASE(ex);
            continue;
        }
        fc4_props* p = props::ToCoreAll(h);
        uint64_t token = 0;
        if (keepRefs) {
            token = TokenOf(ex);  // the visitor takes this reference
        } else {
            FC4_RELEASE(ex);
        }
        visit(ctx, k.type, k.group, k.instance, p, token);
        ++found;
    }
    Log("flora: %u exemplar keys enumerated, %u flora exemplars%s", n, found, keepRefs ? "" : " (read only)");
}

bool FloraBlockValues(const void* block, fc4_flora_record* out) {
    if (block == nullptr || out == nullptr) return false;
    const FC4FloraBlock* b = static_cast<const FC4FloraBlock*>(block);
    *out = fc4_flora_record{};
    out->type_id = b->type;
    out->next_type = b->next;
    out->seed_type = b->seed;
    out->water = b->water != 0 ? 1 : 0;
    out->flags = b->flags;
    std::memcpy(out->floats, b->floats, sizeof(out->floats));
    out->cluster_radius = b->clusterRadius;
    out->cluster_height = b->clusterHeight;
    std::memcpy(out->table, b->table, sizeof(out->table));
    out->unnamed = b->unnamed;
    out->plop_cost = b->plopCost;
    if (b->familyBegin != nullptr && b->familyEnd > b->familyBegin && b->familyEnd - b->familyBegin < 65536) {
        out->families = b->familyBegin;
        out->family_count = static_cast<uint32_t>(b->familyEnd - b->familyBegin);
    }
    return true;
}

}  // namespace fc4

// ---- object -------------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4FloraSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cISC4FloraSimulator*>(new cFC4FloraSimulator()));
}

cFC4FloraSimulator::CoreCall::CoreCall(cFC4FloraSimulator* s) : s_(s), lock_(fc4::FloraLock(), std::try_to_lock) {
    if (!lock_.owns_lock()) {
        lock_.lock();
        ++s_->probes_.lockWaits;
    }
    ++s_->depth_;
}

cFC4FloraSimulator::CoreCall::~CoreCall() {
    if (--s_->depth_ == 0) s_->held_.clear();
}

cFC4FloraSimulator::cFC4FloraSimulator() {
    host_ = MakeHost(this);
    last_.struct_size = sizeof(last_);
    fc4_flora_registry* reg = fc4::FloraRegistry();
    {
        std::lock_guard<std::recursive_mutex> g(fc4::FloraLock());
        // Seed(0xFFFFFFFF) (§3.1, §8, foundations §6.1): the wall-clock and
        // process-clock values.
        core_ = reg ? fc4_flora_create(&host_, reg, SelfToken(), WallClockSeed(), static_cast<uint32_t>(GetTickCount()))
                    : nullptr;
    }
    if (core_ == nullptr) fc4::Log("cFC4FloraSimulator: core creation failed");
    fc4::Log("cFC4FloraSimulator(%p) created, core %p, registry %p", static_cast<void*>(this),
             static_cast<void*>(core_), static_cast<void*>(reg));
}

cFC4FloraSimulator::~cFC4FloraSimulator() {
    {
        std::lock_guard<std::recursive_mutex> g(fc4::FloraLock());
        fc4_flora_destroy(core_);
        core_ = nullptr;
    }
    fc4::Log("cFC4FloraSimulator destroyed");
}

// §1: 0x06C66177 and 0x00000001 (cIGZUnknown) give the primary interface,
// 0x452294AA cIGZMessageTarget2, 0xE4FDA3D4 cIGZSerializable; anything else
// false with the out pointer untouched.
bool cFC4FloraSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_IID_cISC4FloraSimulator:
        case GZIID_cIGZUnknown:
            *ppvObj = Primary();
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = Target();
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        default:
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4FloraSimulator::AddRef() { return ++refCount_; }

// §1: at a count of 1 (or below) Shutdown, destroy, return 0.
uint32_t cFC4FloraSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

void cFC4FloraSimulator::NoteThread(const char* what) {
    if (fc4::MainThreadId() == 0 || fc4::OnMainThread()) return;
    ++probes_.offMainThread;
    fc4::NoteOffMainThreadCall(what);
}

cISC4City* cFC4FloraSimulator::EnsureCity() {
    if (city_ == nullptr) {
        WithApp([&](cISC4App* sc4) {
            city_ = sc4->GetCity();
            if (city_ != nullptr) FC4_ADDREF(city_);
        });
    }
    return city_;
}

// §4.1 step 5 (and step 6's city): each service only while its member is
// still null. The weather simulator may be missing (§6.2); [choice] so may the
// others (their reads are neutral then). The blocking filter of step 5 is the
// box query's type list (OccupantsInBbox), not an object.
void cFC4FloraSimulator::AcquireServices() {
    if (EnsureCity() == nullptr) {
        fc4::Log("flora: no city; services not acquired");
        return;
    }
    auto take = [](auto*& member, auto* value) {
        if (member == nullptr && value != nullptr) {
            member = value;
            FC4_ADDREF(member);
        }
    };
    take(occupantManager_, city_->GetOccupantManager());  // city slot 34
    take(weather_, city_->GetWeatherSimulator());         // slot 70
    take(terrain_, city_->GetTerrain());                  // slot 84
    take(lotManager_, city_->GetLotManager());            // slot 33
}

// §4.2: the lot manager, terrain, occupant manager and weather (then the city).
void cFC4FloraSimulator::ReleaseServices() {
    auto drop = [](auto*& member) {
        if (member != nullptr) FC4_RELEASE(member);
        member = nullptr;
    };
    drop(lotManager_);
    drop(terrain_);
    drop(occupantManager_);
    drop(weather_);
    if (simulator_ == nullptr) drop(city_);  // else after RemoveAgent
}

void cFC4FloraSimulator::Subscribe(const uint32_t* ids, size_t count, bool add) {
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

// §4.1 step 4: hold the simulator, then AddAgent(target, level, name, flags).
void cFC4FloraSimulator::AddAgent(uint32_t level, const char* name, uint32_t flags) {
    if (simulator_ == nullptr) {
        cISC4City* c = EnsureCity();
        cISC4Simulator* s = c ? c->GetSimulator() : nullptr;  // city slot 50
        if (s != nullptr) {
            simulator_ = s;
            FC4_ADDREF(simulator_);
        }
    }
    if (simulator_ == nullptr) {
        fc4::Log("flora: no simulator; agent not registered (level %u)", level);
        return;
    }
    const cRZBaseString n(name);
    fc4::AsGame(simulator_)->AddAgent(Target(), level, n, flags);  // slot 19
    agentsRegistered_ = true;
}

// §4.2: RemoveAgent (all levels), release the simulator.
void cFC4FloraSimulator::RemoveAgent() {
    if (simulator_ != nullptr) {
        if (agentsRegistered_) fc4::AsGame(simulator_)->RemoveAgentAllLevels(Target());  // slot 20
        FC4_RELEASE(simulator_);
        simulator_ = nullptr;
    }
    agentsRegistered_ = false;
    if (occupantManager_ == nullptr && lotManager_ == nullptr && city_ != nullptr) {
        FC4_RELEASE(city_);  // the services are gone: the city goes last
        city_ = nullptr;
    }
}

// §4.1 step 11: RegisterCheatCode(id, name) (slot 3) and AddNotification2
// (slot 7). [spec question] The notification is added for the cheat's id, as
// §4.1 step 11 says ("subscribe the message target to it"); the budget and the
// simulator glue pass 0.
void cFC4FloraSimulator::RegisterCheat(uint32_t id, const char* name) {
    if (cheatMgr_ != nullptr) return;
    WithApp([&](cISC4App* sc4) {
        cIGZCheatCodeManager* cm = sc4->GetCheatCodeManager();
        if (cm != nullptr && cm->QueryInterface(kIID_cIGZCheatCodeManager, reinterpret_cast<void**>(&cheatMgr_)) &&
            cheatMgr_ != nullptr) {
            FC4_ACQUIRED(cheatMgr_, "QueryInterface");
            cheatMgr_->RegisterCheatCode(id, cRZBaseString(name));
            cheatMgr_->AddNotification2(Target(), id);
            cheatId_ = id;
        } else {
            cheatMgr_ = nullptr;
        }
    });
}

// §4.2: RemoveNotification2 (slot 8), then UnregisterCheatCode (slot 4).
void cFC4FloraSimulator::UnregisterCheat(uint32_t id) {
    if (cheatMgr_ == nullptr) return;
    cheatMgr_->RemoveNotification2(Target(), id);
    cheatMgr_->UnregisterCheatCode(id);
    FC4_RELEASE(cheatMgr_);
    cheatMgr_ = nullptr;
    cheatId_ = 0;
}

// §4.1 step 3: RegisterCommand(id, "PlaceFlora", null) (command server slot 6).
void cFC4FloraSimulator::RegisterCommand(uint32_t id, const char* name) {
    if (commandId_ != 0 || name == nullptr) return;
    if (std::strcmp(name, kPlaceFloraName) != 0) {
        fc4::Log("flora: WARNING command '%s' not registered (only PlaceFlora is known)", name);
        return;
    }
    cIGZCommandServerPtr cs;
    if (!cs) {
        fc4::Log("flora: no command server; PlaceFlora not registered");
        return;
    }
    if (cs->RegisterCommand(id, kPlaceFloraName, nullptr)) commandId_ = id;  // parameter info null (§4.1)
}

// §4.2: UnregisterCommand (slot 9), only what was registered.
void cFC4FloraSimulator::UnregisterCommand(uint32_t id) {
    if (commandId_ == 0 || commandId_ != id) return;
    cIGZCommandServerPtr cs;
    if (cs) cs->UnregisterCommand(id);
    commandId_ = 0;
}

void cFC4FloraSimulator::LogState(const char* what, int32_t julian) {
    fc4_flora_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_flora_get_state(core_, &st)) return;
    const Census c = TakeCensus(occupantManager_);
    char date[16] = "-";
    if (julian > 0) {
        fc4_date d{};
        fc4_date_from_julian(julian, &d);
        std::snprintf(date, sizeof(date), "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
    }
    char census[200];
    if (c.taken) {
        std::snprintf(census, sizeof(census),
                      "flora=%u (young %u, maturing %u, mature %u, burnt %u, dying %u, other %u; types %u; not flora %u)",
                      c.total, c.stage[0], c.stage[1], c.stage[2], c.stage[3], c.stage[4], c.other, c.types,
                      c.notFlora);
    } else {
        std::snprintf(census, sizeof(census), "flora=- (no census: %s)",
                      occupantManager_ == nullptr ? "no occupant manager" : "flora_queries / flora_occupants off");
    }
    const fc4_flora_state& p = last_;
    fc4::Log("flora %s %s: %s; planted=%u(+%u) spread=%u(+%u) aged=%u(+%u) reborn=%u(+%u) dying=%u(+%u) "
             "burnt=%u(+%u) cleared=%u(+%u) refused=%u(+%u) removedMonth=%u removedOther=%u moved=%u created=%u "
             "createFailed=%u; burntList=%u enabled=%u propagation=%u forced=%u preserve=%u today=%u period=%u "
             "blocksPerMonth=%u wildSeeds=%u cells=%dx%d blocksVisited=%u occupantsVisited=%u months=%u days=%u; "
             "registry loaded=%u map=%u types=%u families=%u blocks=%u/%u recordsWithoutBlock=%u; reentrant=%u "
             "queued=%u lockWaits=%u/%u offMainThread=%u",
             what, date, census, st.planted, st.planted - p.planted, st.spread, st.spread - p.spread, st.aged,
             st.aged - p.aged, st.reborn, st.reborn - p.reborn, st.dying, st.dying - p.dying, st.burnt,
             st.burnt - p.burnt, st.cleared, st.cleared - p.cleared, st.refused, st.refused - p.refused,
             st.removed_month, st.removed_other, st.moved, st.created, st.create_failed, st.burnt_list,
             st.enabled, st.propagation, st.propagation_forced, st.preserve, st.today, st.period,
             st.blocks_per_month, st.wild_seeds_per_month, st.cells_x, st.cells_z, st.blocks_visited,
             st.occupants_visited, st.months, st.days, st.registry_loaded, st.map_len, st.type_count,
             st.family_count, st.block_count, g_blocks, st.records_without_block, st.reentrant_calls,
             st.queued_messages, st.lock_waits, probes_.lockWaits, probes_.offMainThread);
    last_ = st;
}

// ---- lifecycle (§4) -------------------------------------------------------------------------------------

// §4.1: every step is the core's, through the host callbacks (the load, the
// subscriptions, PlaceFlora, the agents, the services, the city values, the
// iterator, the key-0 record, the tuning, propagation, the cheat and the
// obliteration restore).
bool cFC4FloraSimulator::Init() {
    FC4_TRACE("FloraSimulator::Init");
    {
        CoreCall call(this);
        fc4_flora_init(core_);
    }
    LogState("Init", 0);
    return true;  // §4.1: always
}

bool cFC4FloraSimulator::Shutdown() {
    FC4_TRACE("FloraSimulator::Shutdown");
    // Keep this object alive while the simulator, the message server and the
    // cheat manager drop their references (unless this already is the final
    // Release).
    if (!destroying_) AddRef();
    {
        CoreCall call(this);
        fc4_flora_shutdown(core_);  // §4.2 (a no-op when not initialised)
    }
    // Anything an Init left behind (nothing after the core's Shutdown).
    if (msgServer_ != nullptr) {
        FC4_RELEASE(msgServer_);
        msgServer_ = nullptr;
    }
    UnregisterCheat(cheatId_);
    UnregisterCommand(commandId_);
    ReleaseServices();
    RemoveAgent();
    fc4::Log("flora Shutdown (%u days, %u months, %u messages)", probes_.days, probes_.months, probes_.messages);
    if (!destroying_) Release();
    return true;  // §4.2: always
}

// ---- the registry (§3.2, §11.1 slots 5-14) ---------------------------------------------------------

// Slot 5: the given exemplar (one reference of ours for the record) or the
// exemplar loaded by `key`; null → false.
bool cFC4FloraSimulator::AddStaticFloraData(cGZPersistResourceKey* key, cISCResExemplar* exemplar) {
    CoreCall call(this);
    NoteThread("FloraSimulator::AddStaticFloraData(key, exemplar)");
    cISCResExemplar* ex = exemplar;
    if (ex != nullptr) {
        FC4_ADDREF(ex);
    } else if (key != nullptr && fc4::On(fc4::Sw::ExemplarLoads)) {
        cIGZPersistResourceManagerPtr rm;
        if (rm) {
            FC4_OUT(cISCResExemplar*, out, "cISCResExemplar* (GetResource out pointer)");
            const bool ok = rm->GetResource(*key, GZIID_cISCResExemplar, reinterpret_cast<void**>(out.ptr()), 0,
                                            nullptr);
            out.Check();
            if (ok && out.get() != nullptr) {
                ex = out.get();
                FC4_ACQUIRED(ex, "GetResource");
            }
        }
    }
    if (ex == nullptr) return fc4_flora_add_exemplar(core_, nullptr, 0);
    fc4_props* props = fc4::props::ToCoreAll(ex->AsISCPropertyHolder());
    return fc4_flora_add_exemplar(core_, props, TokenOf(ex));  // both taken
}

// Slot 6: a record the caller built in the game layout. True: registered, and
// the registry's reference is taken on the block (+0x170, §11.1).
bool cFC4FloraSimulator::AddStaticFloraData(cSC4StaticFloraData* record) {
    CoreCall call(this);
    NoteThread("FloraSimulator::AddStaticFloraData(record)");
    fc4_flora_record v{};
    if (!fc4::FloraBlockValues(record, &v)) return false;
    if (!fc4_flora_add_record(core_, &v, TokenOf(record))) return false;
    ++reinterpret_cast<FC4FloraBlock*>(record)->refCount;
    return true;
}

// Slots 7-14 read the registry's snapshot (any thread, no lock).
cSC4StaticFloraData* cFC4FloraSimulator::GetStaticFloraData(uint32_t type) {
    return reinterpret_cast<cSC4StaticFloraData*>(FromToken(fc4_flora_registry_record(fc4::FloraRegistry(), type)));
}

uint32_t cFC4FloraSimulator::GetAllFloraTypes(uint32_t* buffer, uint32_t& count) {
    return fc4_flora_registry_types_into(fc4::FloraRegistry(), buffer, &count);  // slot 9
}

namespace {
void AppendU32(void* ctx, uint64_t v) {
    static_cast<fc4::GameVector<uint32_t>*>(ctx)->push_back(static_cast<uint32_t>(v));
}
void InsertU32(void* ctx, uint64_t v) {
    static_cast<fc4::GameHashSetU32<>*>(ctx)->insert(static_cast<uint32_t>(v));
}
}  // namespace

// Slots 8 and 10: the game's vector (B, allocators.md): cleared (buffer
// kept), then appended to.
void cFC4FloraSimulator::GetAllFloraTypes(SC4Vector<uint32_t>& vector) {
    auto& v = reinterpret_cast<fc4::GameVector<uint32_t>&>(vector);
    v.clear();
    fc4_flora_registry_list(fc4::FloraRegistry(), 0, &AppendU32, &v);
}

void cFC4FloraSimulator::GetAllFloraFamilyTypes(SC4Vector<uint32_t>& vector) {
    auto& v = reinterpret_cast<fc4::GameVector<uint32_t>&>(vector);
    v.clear();
    fc4_flora_registry_list(fc4::FloraRegistry(), 2, &AppendU32, &v);
}

bool cFC4FloraSimulator::IsFloraTypeValid(uint32_t type) {
    return fc4_flora_registry_is_type_valid(fc4::FloraRegistry(), type);
}

bool cFC4FloraSimulator::IsFloraFamilyTypeValid(uint32_t type) {
    return fc4_flora_registry_is_family_valid(fc4::FloraRegistry(), type);
}

// Slot 13: every map key (0 included) into the caller's hash_set (not
// cleared: a union), the number of insert attempts returned.
// [spec question] With a non-null filter: cISC4FloraTypeFilter (its slot 3)
// is not in docs/abi, so nothing is inserted and 0 is returned (logged once).
uint32_t cFC4FloraSimulator::GetFloraTypesByFilter(intptr_t& hashSet, cISC4FloraTypeFilter* filter) {
    if (filter != nullptr) {
        static bool logged = false;
        LogOnce(logged, "flora: GetFloraTypesByFilter with a filter: cISC4FloraTypeFilter is not in docs/abi; nothing "
                        "inserted, 0 returned (spec question)");
        return 0;
    }
    auto* set = reinterpret_cast<fc4::GameHashSetU32<>*>(&hashSet);
    return fc4_flora_registry_list(fc4::FloraRegistry(), 1, &InsertU32, set);
}

// Slot 14: the family's types into the caller's hash_set; their count.
uint32_t cFC4FloraSimulator::GetFloraTypesByFamily(intptr_t& hashSet, uint32_t family) {
    auto* set = reinterpret_cast<fc4::GameHashSetU32<>*>(&hashSet);
    return fc4_flora_registry_family(fc4::FloraRegistry(), family, &InsertU32, set);
}

// ---- planting and removal (§6, §11.1 slots 15-24) ----------------------------------------------------

bool cFC4FloraSimulator::LocationIsOKForNewFlora(uint32_t type, float x, float z, cISC4Occupant* exclude) {
    CoreCall call(this);
    NoteThread("FloraSimulator::LocationIsOKForNewFlora");
    return fc4_flora_location_ok(core_, type, x, z, TokenOf(exclude));
}

uint32_t cFC4FloraSimulator::GetBestFloraTypeForLocation(float x, float z, float& score) {
    CoreCall call(this);
    NoteThread("FloraSimulator::GetBestFloraTypeForLocation");
    return fc4_flora_best_type(core_, x, z, &score);
}

// Slot 17: the inserted occupant, borrowed (the occupant manager holds it);
// `pos` is rewritten (§14 quirk 4).
cISC4Occupant* cFC4FloraSimulator::AddNewFloraOccupant(uint32_t type, float* pos, bool force) {
    CoreCall call(this);
    NoteThread("FloraSimulator::AddNewFloraOccupant(type, pos, force)");
    return OccOf(fc4_flora_add_new_occupant(core_, type, pos, force));
}

bool cFC4FloraSimulator::AddNewFloraOccupant(cISC4Occupant* occupant) {
    CoreCall call(this);
    NoteThread("FloraSimulator::AddNewFloraOccupant(occupant)");
    return fc4_flora_add_occupant(core_, TokenOf(occupant));
}

bool cFC4FloraSimulator::DemolishFloraOccupant(cISC4Occupant* occupant, int32_t flags) {
    CoreCall call(this);
    NoteThread("FloraSimulator::DemolishFloraOccupant");
    return fc4_flora_demolish_occupant(core_, TokenOf(occupant), flags);
}

bool cFC4FloraSimulator::DemolishFloraInCellArea(SC4Rect<int32_t> const& rect, int32_t flags) {
    CoreCall call(this);
    NoteThread("FloraSimulator::DemolishFloraInCellArea");
    fc4_cell_rect r{};
    r.x1 = rect.topLeftX;
    r.z1 = rect.topLeftY;
    r.x2 = rect.bottomRightX;
    r.z2 = rect.bottomRightY;
    return fc4_flora_demolish_cells(core_, &r, flags);
}

bool cFC4FloraSimulator::DemolishFloraInBoundingBox(float const* min, float const* max, int32_t flags) {
    CoreCall call(this);
    NoteThread("FloraSimulator::DemolishFloraInBoundingBox");
    return fc4_flora_demolish_bbox(core_, min, max, flags);
}

// Slot 22: always 0 (EDX:EAX).
int64_t cFC4FloraSimulator::GetCostOfOccupantDemolition(cISC4Occupant*, int32_t) { return 0; }

bool cFC4FloraSimulator::KillFloraOccupant(cISC4Occupant* occupant) {
    CoreCall call(this);
    NoteThread("FloraSimulator::KillFloraOccupant");
    return fc4_flora_kill(core_, TokenOf(occupant));
}

// Slot 24 (§10.3). [spec question] cIGZPersistDBSegment's record writes are
// not in docs/abi: nothing is written (the core is not called, so no flag 1
// toggles), true returned as the original does; logged once.
bool cFC4FloraSimulator::SaveObliterated(cIGZPersistDBSegment* segment) {
    NoteThread("FloraSimulator::SaveObliterated");
    static bool logged = false;
    if (!logged) {
        logged = true;
        fc4::Log("flora: SaveObliterated(%p): the obliterated city's flora is NOT saved: cIGZPersistDBSegment's record "
                 "writes are not in docs/abi (flora.md §10.3; spec question)",
                 static_cast<void*>(segment));
    }
    return true;
}

// ---- messages (§5) ---------------------------------------------------------------------------------------

bool cFC4FloraSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    CoreCall call(this);
    ++probes_.messages;
    fc4_flora_message m{};
    m.msg_type = pMessage->GetType();
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    cISC4Occupant* occ = nullptr;
    std::string text;
    fc4_flora_param params[3] = {};
    if (s != nullptr) {
        m.data1 = static_cast<uint32_t>(s->GetData1());
        m.data2 = static_cast<uint32_t>(s->GetData2());
        m.data3 = static_cast<uint32_t>(s->GetData3());
        if (m.msg_type == kMsgInsertOccupant || m.msg_type == kMsgRemoveOccupant) {
            // §5: the occupant is void1, known by its cISC4Occupant identity.
            occ = Query<cISC4Occupant>(static_cast<cIGZUnknown*>(s->GetVoid1()), kIID_cISC4Occupant);
            m.occupant = TokenOf(occ);
        } else if (m.msg_type == kMsgCheat) {
            // The cheat's command line is a cIGZString in void2 (as the budget reads it).
            const cIGZString* t = static_cast<const cIGZString*>(s->GetVoid2());
            if (t != nullptr && t->Data() != nullptr) text.assign(t->Data(), t->Strlen());
            m.text = text.empty() ? nullptr : reinterpret_cast<const uint8_t*>(text.data());
            m.text_len = text.size();
        } else if (m.msg_type == kMsgPlaceFlora) {
            m.param_count = ReadPlaceFloraParams(static_cast<cIGZCommandParameterSet*>(s->GetVoid1()), params);
            m.params = m.param_count != 0 ? params : nullptr;
        }
    }
    // A message delivered while another call into the core is running is
    // queued by the core and handled when that call returns: keep its occupant
    // alive until then.
    if (depth_ > 1 && occ != nullptr) {
        cIGZUnknown* k = occ;
        FC4_ADDREF(k);
        Hold(std::shared_ptr<void>(k, [](void* p) { FC4_RELEASE(static_cast<cIGZUnknown*>(p)); }));
    }
    fc4_flora_do_message(core_, &m);
    Drop(occ);
    Drop(s);
    if (m.msg_type == kMsgSimNewDay) ++probes_.days;
    if (m.msg_type == kMsgSimNewMonth) {
        ++probes_.months;
        if (depth_ == 1) LogState("month", static_cast<int32_t>(m.data1));
    }
    return true;  // §5: always true
}

// ---- persistence (§10) -----------------------------------------------------------------------------------

bool cFC4FloraSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("FloraSimulator::Write");
    CoreCall call(this);
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_flora_write(core_, a.Get());
    fc4::Log("flora Write -> %d", ok ? 1 : 0);
    return ok;
}

// §10.2: Read runs before Init and sizes nothing from the city (Init
// recomputes the sizes, the cell counts and the iterator).
bool cFC4FloraSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("FloraSimulator::Read");
    bool ok = false;
    {
        CoreCall call(this);
        fc4::IStreamAdapter a(s);
        ok = fc4_flora_read(core_, a.Get());
    }
    fc4::Log("flora Read -> %d", ok ? 1 : 0);
    LogState("Read", 0);
    return ok;
}

uint32_t cFC4FloraSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4FloraSimulator; }
