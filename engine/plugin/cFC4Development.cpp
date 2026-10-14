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

#include "cFC4Development.h"

#include "FC4Checked.h"
#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4GameList.h"
#include "FC4GameMap.h"
#include "FC4GameMapBuild.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "SC4CellRegion.h"
#include "SC4HashSet.h"
#include "SC4List.h"
#include "SC4Rect.h"
#include "StringResourceKey.h"
#include "StringResourceManager.h"
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
#include "cISC4Advisor.h"
#include "cISC4AdvisorSystem.h"
#include "cISC4App.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4Demand.h"
#include "cISC4DemandSimulator.h"
#include "cISC4Demolition.h"
#include "cISC4FoundationOccupant.h"
#include "cISC4GrowthDeveloper.h"
#include "cISC4LandValueSimulator.h"
#include "cISC4Lot.h"
#include "cISC4LotConfiguration.h"
#include "cISC4LotConfigurationFilter.h"
#include "cISC4LotConfigurationManager.h"
#include "cISC4LotDeveloper.h"
#include "cISC4LotManager.h"
#include "cISC4LotRetainingWallOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4PowerSimulator.h"
#include "cISC4SimGrid.h"
#include "cISC4Simulator.h"
#include "cISC4TractDeveloper.h"
#include "cISC4ZoneManager.h"
#include "cISCExemplarPropertyHolder.h"
#include "cISCPropertyHolder.h"
#include "cISCResExemplar.h"
#include "cISTETerrain.h"
#include "cRZAutoRefCount.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"
#include "cSC4LotConfigurationObject.h"

#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <map>
#include <string>
#include <unordered_map>
#include <unordered_set>
#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

// Interfaces and classes (§1.1, §1.3, §4.9, §6).
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4BuildingOccupant = 0x87DDA3A9;
constexpr uint32_t kIID_cISC4Lot = 0xE818FCED;
constexpr uint32_t kIID_cISC4Demand = 0x0797A4E8;
constexpr uint32_t kIID_cISCExemplarPropertyHolder = 0x0AC2B5F7;
constexpr uint32_t kIID_cIGZCheatCodeManager = 0xA1085722;  // as used by the SDK examples
constexpr uint32_t kCLSID_cSC4BuildingOccupant = 0xA9BD882D;
constexpr uint32_t kCLSID_cSC4Demand = 0x8990C12D;

// Messages (§3.1, §3.2, §5).
constexpr uint32_t kMsgPlaceBuilding = 0x0B99AE81;
constexpr uint32_t kMsgOccupantInserted = 0x99EF1142;
constexpr uint32_t kMsgOccupantRemoved = 0x99EF1143;
constexpr uint32_t kMsgStateChanged = 0xE8BDA2E7;
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgBuildingTypesUpdated = 0xA783EBC8;

// Exemplars (§1.3, §4.7.1, §4.13).
constexpr uint32_t kExemplarResourceType = 0x6534284A;
constexpr uint32_t kPropExemplarType = 0x10;
constexpr uint32_t kPropLocalizedName = 0x8A416A99;
constexpr uint32_t kExemplarTypeBuilding = 2;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0xE8DA7677;

/// §4.9 slot 5: the occupant type of buildings.
constexpr uint32_t kOccupantTypeBuilding = 0x278128A0;

// A cell region of more cells is refused (as the landfill's).
constexpr int64_t kMaxRegionCells = int64_t{1} << 20;

// ---- the process lock (fc4.h: one lock for the BDS, its statics and every developer) ----------

struct ProcessLock {
    CRITICAL_SECTION cs;
    ProcessLock() { InitializeCriticalSection(&cs); }
};

CRITICAL_SECTION* TheLock() {
    static ProcessLock lock;  // never destroyed before the process ends (it is in use until then)
    return &lock.cs;
}

std::atomic<DWORD> g_mainTid{0};

// The live simulator (main thread; for the growth developers' host).
cFC4BuildingDevelopmentSimulator* g_live = nullptr;

void NoteOnce(bool& done, const char* text) {
    if (done) return;
    done = true;
    fc4::Log("%s", text);
}

// Main thread only (host callbacks run under the process lock).
bool g_noteCleanup = false;
bool g_noteFoundationAt = false;
bool g_noteFoundationProps = false;
bool g_noteWallProps = false;
bool g_noteNoAllocator = false;
bool g_noteRegion = false;

fc4::DevelopmentContext* Ctx(void* u) { return static_cast<fc4::DevelopmentContext*>(u); }

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

// The city the callbacks work on: the one held from acquire_services, else
// (the catalogue load and the developers run before §1.3 step 5) the app's.
cISC4City* CityOf(void* u) {
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->city != nullptr) return c->city;
    return c->readOnly ? nullptr : AppCity();
}

// The development_* switches; the shadow's context reads the game as it is.
bool Gate(void* u, fc4::Sw s, const char* what) {
    if (Ctx(u)->readOnly || fc4::On(s)) return true;
    fc4::NoteSkipped(s, what);
    return false;
}

// The shadow's context (FC4DevelopmentShadow.h) never changes the game: its
// posts, registrations, creations and every write callback do nothing.
bool Writes(void* u) { return !Ctx(u)->readOnly; }

cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cISC4Lot* LotOf(uint64_t t) { return reinterpret_cast<cISC4Lot*>(FromToken(t)); }
cFC4ISC4Demand* NodeOf(uint64_t t) {
    return t ? fc4::AsGame(reinterpret_cast<cISC4Demand*>(FromToken(t))) : nullptr;
}
cFC4ISC4GrowthDeveloper* DevOf(uint64_t t) {
    return t ? fc4::AsGame(reinterpret_cast<cISC4GrowthDeveloper*>(FromToken(t))) : nullptr;
}

std::string Text(const cIGZString* s) {
    if (s == nullptr || s->ToChar() == nullptr) return std::string();
    return std::string(s->ToChar(), s->Strlen());
}

std::string DateText(uint32_t julian) {
    fc4_date d{};
    fc4_date_from_julian(static_cast<int32_t>(julian), &d);
    char buf[32];
    std::snprintf(buf, sizeof(buf), "%04d-%02d-%02d", static_cast<int>(d.year), static_cast<int>(d.month),
                  static_cast<int>(d.day_of_month));
    return buf;
}

// ---- the services (§1.3 step 5, §1.4) ---------------------------------------------------------------

void Hold(cIGZUnknown*& slot, cIGZUnknown* p) {
    if (slot != nullptr || p == nullptr) return;
    slot = p;
    FC4_ADDREF(p);
}

void Unhold(cIGZUnknown*& slot) {
    if (slot != nullptr) FC4_RELEASE(slot);
    slot = nullptr;
}

// §1.3 step 5: the city and the services, each AddRef'd; the terrain's cell
// counts cached. [choice] The resource manager is not held (fetched when
// used, as the other glue does).
void AcquireServices(void* u) {
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->city == nullptr) {
        cISC4City* city = AppCity();
        if (city != nullptr) {
            c->city = city;
            FC4_ADDREF(city);
        }
    }
    cISC4City* city = c->city;
    if (city == nullptr) {
        fc4::Log("development: no city; services not acquired");
        return;
    }
    Hold(c->occupantManager, city->GetOccupantManager());  // slot 34
    Hold(c->zoneManager, city->GetZoneManager());
    Hold(c->lotManager, city->GetLotManager());
    Hold(c->landValue, city->GetLandValueSimulator());
    Hold(c->power, city->GetPowerSimulator());
    Hold(c->terrain, city->GetTerrain());
    if (c->terrain != nullptr) {
        cISTETerrain* t = static_cast<cISTETerrain*>(c->terrain);
        c->cellCountX = static_cast<int32_t>(t->CellCountX());  // slot 7
        c->cellCountZ = static_cast<int32_t>(t->CellCountZ());  // slot 8
    }
}

// §1.4: the cached services and the message server.
void ReleaseServices(void* u) {
    fc4::DevelopmentContext* c = Ctx(u);
    Unhold(c->terrain);
    Unhold(c->power);
    Unhold(c->landValue);
    Unhold(c->lotManager);
    Unhold(c->zoneManager);
    Unhold(c->occupantManager);
    if (c->msgServer != nullptr) FC4_RELEASE(c->msgServer);
    c->msgServer = nullptr;
    if (c->city != nullptr) FC4_RELEASE(c->city);
    c->city = nullptr;
}

cISC4OccupantManager* Occupants(void* u, const char* what) {
    if (!Gate(u, fc4::Sw::DevelopmentOccupants, what)) return nullptr;
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->occupantManager != nullptr) return static_cast<cISC4OccupantManager*>(c->occupantManager);
    cISC4City* city = CityOf(u);
    return city ? city->GetOccupantManager() : nullptr;
}

cISC4ZoneManager* Zones(void* u, const char* what) {
    if (!Gate(u, fc4::Sw::DevelopmentZoning, what)) return nullptr;
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->zoneManager != nullptr) return static_cast<cISC4ZoneManager*>(c->zoneManager);
    cISC4City* city = CityOf(u);
    return city ? city->GetZoneManager() : nullptr;
}

cFC4ISC4LotManager* Lots(void* u, const char* what) {
    if (!Gate(u, fc4::Sw::DevelopmentLots, what)) return nullptr;
    fc4::DevelopmentContext* c = Ctx(u);
    cISC4LotManager* lm = static_cast<cISC4LotManager*>(c->lotManager);
    if (lm == nullptr) {
        cISC4City* city = CityOf(u);
        lm = city ? city->GetLotManager() : nullptr;
    }
    return lm ? fc4::AsGame(lm) : nullptr;
}

cISC4LotDeveloper* LotDeveloper(void* u, const char* what) {
    if (!Gate(u, fc4::Sw::DevelopmentLotDeveloper, what)) return nullptr;
    cISC4City* city = CityOf(u);
    return city ? city->GetLotDeveloper() : nullptr;  // borrowed (§7.1 slot 36: cached, no AddRef)
}

cISTETerrain* Terrain(void* u) {
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->terrain != nullptr) return static_cast<cISTETerrain*>(c->terrain);
    cISC4City* city = CityOf(u);
    return city ? city->GetTerrain() : nullptr;
}

// ---- clock, city, terrain -----------------------------------------------------------------------------

// §8: simulator slot 7 GetSimDate() (borrowed), cIGZDate slot 26 DayNumber.
uint32_t TodayOf(cISC4City* c) {
    cISC4Simulator* s = c ? c->GetSimulator() : nullptr;
    cFC4IGZDate* d = s ? fc4::AsGame(s)->GetSimDate() : nullptr;
    return d ? d->DayNumber() : 0;
}

uint32_t SimDay(void* u) { return TodayOf(CityOf(u)); }

uint32_t Difficulty(void* u) {
    cISC4City* c = CityOf(u);
    return c ? static_cast<uint32_t>(c->GetDifficultyLevel()) : 0;  // city slot 24
}

void CellWidths(void* u, float* out2) {
    if (out2 == nullptr) return;
    cISC4City* c = CityOf(u);
    if (c == nullptr) return;  // the core's default stays
    out2[0] = c->CellWidthX();  // slot 91, ST0
    out2[1] = c->CellWidthZ();  // slot 92, ST0
}

void TerrainCellCounts(void* u, int32_t* out2) {
    if (out2 == nullptr) return;
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->terrain != nullptr) {
        out2[0] = c->cellCountX;
        out2[1] = c->cellCountZ;
        return;
    }
    cISTETerrain* t = Terrain(u);
    out2[0] = t ? static_cast<int32_t>(t->CellCountX()) : 0;
    out2[1] = t ? static_cast<int32_t>(t->CellCountZ()) : 0;
}

void PositionToCell(void* u, float x, float z, int32_t* out2) {
    if (out2 == nullptr) return;
    int cx = 0, cz = 0;
    if (cISC4City* c = CityOf(u)) c->PositionToCell(x, z, cx, cz);  // slot 95
    out2[0] = cx;
    out2[1] = cz;
}

// City slot 97 CellCenterToPosition: the results through the out references
// (it returns nothing). The core reads two floats, (x, z).
void CellCenter(void* u, int32_t x, int32_t z, float* out2) {
    if (out2 == nullptr) return;
    float fx = 0.0f, fz = 0.0f;
    if (cISC4City* c = CityOf(u)) c->CellCenterToPosition(x, z, fx, fz);
    out2[0] = fx;
    out2[1] = fz;
}

float Altitude(void* u, float x, float z) {
    if (!Gate(u, fc4::Sw::DevelopmentCity, "development terrain GetAltitude (0)")) return 0.0f;
    cISTETerrain* t = Terrain(u);
    return t ? t->GetAltitude(x, z) : 0.0f;  // compiled slot 13 (foundations §16), ST0
}

// ---- resources (§1.3 steps 1-3, §4.13) ---------------------------------------------------------------

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
    bool IsKeyIncluded(cGZPersistResourceKey const& key) override { return key.type == kExemplarResourceType; }

private:
    uint32_t refs_ = 1;
};

// Every exemplar key, in the resource manager's order (§1.3 step 1: "the key
// list's enumerate"). [choice] As the demand glue: GetAvailableResourceList
// with our filter returns a new list with one reference.
bool ExemplarKeys(std::vector<cGZPersistResourceKey>& keys) {
    cIGZPersistResourceManagerPtr rm;
    if (!rm) {
        fc4::Log("development: no resource manager; no exemplars");
        return false;
    }
    ExemplarKeyFilter filter;
    FC4_OUT(cIGZPersistResourceKeyList*, list, "cIGZPersistResourceKeyList* (GetAvailableResourceList out pointer)");
    rm->GetAvailableResourceList(list.ptr(), &filter);
    list.Check();
    cIGZPersistResourceKeyList* l = list.get();
    if (l == nullptr) {
        fc4::Log("development: WARNING the resource manager returned no key list; no exemplars");
        return false;
    }
    FC4_ACQUIRED(l, "GetAvailableResourceList");
    const uint32_t n = l->Size();
    keys.reserve(n);
    for (uint32_t i = 0; i < n; ++i) keys.push_back(l->GetKey(i));
    FC4_RELEASE_CREATED(l);
    return true;
}

// The localized building name (§4.7.1: SC4LoadExemplarString of property
// 0x8A416A99 = {type, group, instance} of an LTEXT): true and the text, or false.
bool LocalizedName(const cISCPropertyHolder* holder, std::string& out) {
    std::vector<uint32_t> v;
    if (!fc4::props::GetU32s(holder, kPropLocalizedName, v) || v.size() < 3) return false;
    if (!fc4::On(fc4::Sw::Ltext)) {
        fc4::NoteSkipped(fc4::Sw::Ltext, "development building name LTEXT (absent)");
        return false;
    }
    cRZAutoRefCount<cIGZString> s;  // the SDK helper owns the reference (not audited)
    if (!StringResourceManager::GetLocalizedString(StringResourceKey(v[1], v[2]), s)) return false;
    out = Text(s);
    return true;
}

// fc4.h exemplars: every exemplar whose ExemplarType is one of `types` (all
// when n is 0), with its properties (NEW, the core's) and, for buildings, the
// localized name.
void Exemplars(void* u, const uint32_t* types, size_t n, fc4_development_exemplar_visitor visit, void* ctx) {
    if (visit == nullptr) return;
    if (!fc4::On(fc4::Sw::ExemplarLoads)) {
        fc4::NoteSkipped(fc4::Sw::ExemplarLoads, "development exemplar enumeration (none)");
        return;
    }
    std::vector<cGZPersistResourceKey> keys;
    if (!ExemplarKeys(keys)) return;
    uint32_t visited = 0, withInherited = 0, inherited = 0;
    for (const cGZPersistResourceKey& k : keys) {
        if (k.type != kExemplarResourceType) continue;
        fc4::props::Exemplar ex;
        if (!ex.Load(k.type, k.group, k.instance)) continue;
        std::vector<uint32_t> t;
        if (!fc4::props::GetU32s(ex.Holder(), kPropExemplarType, t) || t.empty()) continue;
        if (n != 0 && std::find(types, types + n, t[0]) == types + n) continue;
        // The effective properties (data-formats §4.4): most buildings have
        // their ExemplarType, purpose, capacity and families in a cohort.
        fc4::props::ToCoreAllStats st;
        fc4_props* props = fc4::props::ToCoreAll(ex, &st);
        if (st.inherited != 0) ++withInherited;
        inherited += st.inherited;
        std::string name;
        const bool hasName = t[0] == kExemplarTypeBuilding && LocalizedName(ex.Holder(), name);
        const fc4_res_key key{k.type, k.group, k.instance};
        visit(ctx, &key, props, reinterpret_cast<const uint8_t*>(name.data()), name.size(), hasName);
        ++visited;
    }
    if (!Ctx(u)->readOnly) {
        fc4::Log("development: %u exemplar keys, %u exemplars of %u type(s) visited (%u with %u properties inherited "
                 "from cohorts)",
                 static_cast<unsigned>(keys.size()), visited, static_cast<unsigned>(n), withInherited, inherited);
    }
}

// The lot configuration filter of §4.7.4 (every configuration). Lives on the
// stack; the manager does not keep it.
class AllLotConfigurations final : public cISC4LotConfigurationFilter {
public:
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid == GZIID_cISC4LotConfigurationFilter || riid == GZIID_cIGZUnknown) {
            *ppvObj = this;
            AddRef();
            return true;
        }
        return false;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { return refs_ > 0 ? --refs_ : 0; }
    bool IsLotConfigurationIncluded(cISC4LotConfiguration*) override { return true; }

private:
    uint32_t refs_ = 1;
};

// §4.7.4: every lot configuration. [spec question, choice] The spec reads the
// lot configuration manager's slot 20 GetLotConfigurationMap, whose layout is
// not in docs/abi; the configurations come from slot 9
// GetLotConfigurationsByFilter with an accept-all filter (allocators.md: an
// STLport hash_set filled with B, one reference per configuration, released
// here). The density table keeps maxima, so the order does not matter.
void LotConfigurations(void* u, fc4_lot_config_visitor visit, void* ctx) {
    if (visit == nullptr) return;
    cISC4City* city = CityOf(u);
    cISC4LotConfigurationManager* lcm = city ? city->GetLotConfigurationManager() : nullptr;
    if (lcm == nullptr) {
        fc4::Log("development: no lot configuration manager; the maximum density table is empty");
        return;
    }
    if (!fc4::StlNodeAllocator::Available()) {
        NoteOnce(g_noteNoAllocator, "development: the game's STL allocator is not available; no lot configurations "
                                    "are read and the game containers are not built");
        return;
    }
    fc4::GameHashSetU32<> set(193);
    AllLotConfigurations filter;
    lcm->GetLotConfigurationsByFilter(*reinterpret_cast<SC4HashSet<cISC4LotConfiguration*>*>(&set), &filter);
    std::vector<cISC4LotConfiguration*> configs;
    configs.reserve(set.size());
    for (size_t b = 0; b < set.bucket_count(); ++b) {
        for (auto* node = set.bucket(b); node != nullptr; node = node->next) {
            auto* cfg = reinterpret_cast<cISC4LotConfiguration*>(static_cast<uintptr_t>(node->value));
            if (cfg == nullptr) continue;
            FC4_ACQUIRED(cfg, "GetLotConfigurationsByFilter");
            configs.push_back(cfg);
        }
    }
    std::vector<uint32_t> ids;
    for (cISC4LotConfiguration* cfg : configs) {
        fc4_lot_config c{};
        cfg->GetSize(c.size_x, c.size_z);  // slot 7
        for (uint32_t z = 0; z < 16; ++z) {
            if (cfg->IsCompatibleWithZoneType(static_cast<cISC4ZoneManager::ZoneType>(z))) c.zones |= 1u << z;  // slot 15
        }
        ids.clear();
        cSC4LotConfigurationObject* obj = cfg->GetPrimaryBuildingConfigurationObject();  // slot 24
        if (obj != nullptr) {
            c.has_primary = 1;
            c.orientation = static_cast<uint32_t>(obj->rotation);
            c.x = obj->x;
            c.z = obj->z;
            for (uint32_t id : obj->data) ids.push_back(id);
        }
        c.n_ids = static_cast<uint32_t>(ids.size());
        c.ids = ids.empty() ? nullptr : ids.data();
        visit(ctx, &c);
    }
    for (cISC4LotConfiguration* cfg : configs) FC4_RELEASE(cfg);
    if (!Ctx(u)->readOnly) {
        fc4::Log("development: %u lot configurations read for the maximum density table",
                 static_cast<unsigned>(configs.size()));
    }
}

// §4.13: the "simulators" exemplar; NULL leaves the code defaults.
fc4_props* Tuning(void* u) {
    fc4::props::Exemplar ex;
    if (!ex.Load(kExemplarResourceType, kTuningGroup, kTuningInstance)) {
        if (!Ctx(u)->readOnly) fc4::Log("development: tuning exemplar unavailable; the code defaults stay");
        return nullptr;
    }
    return fc4::props::ToCoreAll(ex);
}

// ---- framework (§1.3, §1.4, §5) ----------------------------------------------------------------------

// §1.3 step 1: app slot 69 RegisterStaticCleanup. [spec question, choice] The
// application's slot 69 is not in docs/abi: not registered (the catalogue
// lives until the process ends, which is what the cleanup ends anyway).
void RegisterStaticCleanup(void*) {
    NoteOnce(g_noteCleanup, "development: the static-data cleanup is not registered with the application (app slot "
                            "69 is not in docs/abi); the catalogue lives until the process ends");
}

// §5: kSC4MessageBuildingTypesUpdated, void1 null, data2 = the count, posted.
void PostBuildingTypesUpdated(void* u, uint32_t count) {
    if (!Writes(u)) return;
    if (!Gate(u, fc4::Sw::DevelopmentPosts, "development BuildingTypesUpdated post")) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* msg = FC4MakeMessage(kMsgBuildingTypesUpdated, 0, static_cast<intptr_t>(count), 0);
    ms->MessagePost(msg, false);
    msg->Release();
    fc4::Log("development: posted BuildingTypesUpdated (%u building types)", count);
}

cIGZMessageServer2* MessageServer(void* u) {
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->msgServer == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            c->msgServer = ms;
            FC4_ADDREF(c->msgServer);
        }
    }
    return c->msgServer;
}

void Subscribe(void* u, const uint32_t* types, size_t n) {
    fc4::DevelopmentContext* c = Ctx(u);
    cIGZMessageServer2* ms = MessageServer(u);
    if (ms == nullptr || c->target == nullptr || types == nullptr) return;
    for (size_t i = 0; i < n; ++i) ms->AddNotification(c->target, types[i]);
}

void Unsubscribe(void* u, const uint32_t* types, size_t n) {
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->msgServer == nullptr || c->target == nullptr || types == nullptr) return;
    for (size_t i = 0; i < n; ++i) c->msgServer->RemoveNotification(c->target, types[i]);
}

// §1.3 step 8: command server slot 6 RegisterCommand(id, name, null).
void RegisterCommand(void* u, uint32_t id, const char* name, size_t len) {
    if (!Writes(u)) return;
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->commandId != 0 || name == nullptr) return;
    if (!Gate(u, fc4::Sw::DevelopmentCheats, "development PlaceBuilding command registration")) return;
    cIGZCommandServerPtr cs;
    if (!cs) {
        fc4::Log("development: no command server; PlaceBuilding not registered");
        return;
    }
    const std::string n(name, len);
    if (cs->RegisterCommand(id, n.c_str(), nullptr)) c->commandId = id;
}

// §1.4: command server slot 9, only what was registered.
void UnregisterCommand(void* u, uint32_t id) {
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->commandId == 0 || c->commandId != id) return;
    cIGZCommandServerPtr cs;
    if (cs) cs->UnregisterCommand(id);
    c->commandId = 0;
}

// §1.3 step 9: simulator slot 19 AddAgent(target, level, name, flags); the
// simulator is held until RemoveAgent.
void AddAgent(void* u, uint32_t level, const char* name, size_t len, uint32_t flags) {
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->target == nullptr || name == nullptr) return;
    cISC4City* city = CityOf(u);
    cISC4Simulator* raw = city ? city->GetSimulator() : nullptr;  // city slot 50, borrowed
    if (raw == nullptr) {
        fc4::Log("development: no simulator; agent level %u not registered", level);
        return;
    }
    if (c->simulator == nullptr) {
        c->simulator = raw;
        FC4_ADDREF(c->simulator);
    }
    const cRZBaseString n(name, len);
    fc4::AsGame(static_cast<cISC4Simulator*>(c->simulator))->AddAgent(c->target, level, n, flags);
    c->agentsRegistered = true;
}

// §1.4: simulator slot 20 on the simulator AddAgent used.
void RemoveAgent(void* u) {
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->simulator == nullptr) return;
    if (c->agentsRegistered) {
        fc4::AsGame(static_cast<cISC4Simulator*>(c->simulator))->RemoveAgentAllLevels(c->target);
    }
    c->agentsRegistered = false;
    FC4_RELEASE(c->simulator);
    c->simulator = nullptr;
}

// §1.3 step 10: RegisterCheatCode (slot 3) and AddNotification2 (slot 7).
// [spec question, choice] The notification is added for the cheat's id (as
// the flora glue).
void RegisterCheat(void* u, uint32_t id, const char* name, size_t len) {
    if (!Writes(u)) return;
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->target == nullptr || name == nullptr) return;
    if (!Gate(u, fc4::Sw::DevelopmentCheats, "development cheat registration")) return;
    if (c->cheatMgr == nullptr) {
        cIGZFrameWork* fw = RZGetFrameWork();
        cIGZApp* app = fw ? fw->Application() : nullptr;
        cISC4App* sc4 = Query<cISC4App>(app, GZIID_cISC4App);
        cIGZCheatCodeManager* cm = sc4 ? sc4->GetCheatCodeManager() : nullptr;
        c->cheatMgr = Query<cIGZCheatCodeManager>(cm, kIID_cIGZCheatCodeManager);
        Drop(sc4);
        if (c->cheatMgr == nullptr) {
            fc4::Log("development: no cheat code manager; cheat 0x%08X not registered", id);
            return;
        }
    }
    const std::string n(name, len);
    c->cheatMgr->RegisterCheatCode(id, cRZBaseString(n.c_str()));
    c->cheatMgr->AddNotification2(c->target, id);
    c->cheats.push_back(id);
}

// §1.4: cheat manager slots 8 and 4; the manager goes with the last cheat.
void UnregisterCheat(void* u, uint32_t id) {
    fc4::DevelopmentContext* c = Ctx(u);
    auto it = std::find(c->cheats.begin(), c->cheats.end(), id);
    if (c->cheatMgr == nullptr || it == c->cheats.end()) return;
    c->cheatMgr->RemoveNotification2(c->target, id);
    c->cheatMgr->UnregisterCheatCode(id);
    c->cheats.erase(it);
    if (c->cheats.empty()) {
        FC4_RELEASE(c->cheatMgr);
        c->cheatMgr = nullptr;
    }
}

// ---- references -----------------------------------------------------------------------------------

uint64_t NoteCreated(void* u, void* p) {
    cIGZUnknown* o = static_cast<cIGZUnknown*>(p);
    if (o == nullptr) return 0;
    FC4_ACQUIRED(o, "GetClassObject");
    Ctx(u)->created[o] = 1;
    return TokenOf(o);
}

void AddRefToken(void* u, uint64_t t) {
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return;
    auto& created = Ctx(u)->created;
    auto it = created.find(o);
    if (it != created.end()) ++it->second;
    if (fc4::IsOurObject(o)) {
        o->AddRef();
    } else {
        FC4_ADDREF(o);
    }
}

void ReleaseToken(void* u, uint64_t t) {
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return;
    auto& created = Ctx(u)->created;
    auto it = created.find(o);
    bool lastCreated = false;
    if (it != created.end() && --it->second == 0) {
        created.erase(it);
        lastCreated = true;
    }
    if (fc4::IsOurObject(o)) {
        o->Release();
    } else if (lastCreated) {
        FC4_RELEASE_CREATED(o);  // ours since GetClassObject: freeing it may be expected
    } else {
        FC4_RELEASE(o);
    }
}

// ---- demand (§4.1, §4.6) ---------------------------------------------------------------------------

// §4.1: demandSim.GetDemand(id, 0).QuerySupplyValue(); false for a missing node.
bool CensusSupply(void* u, uint32_t id, float* out) {
    cISC4City* c = CityOf(u);
    cISC4DemandSimulator* ds = c ? c->GetDemandSimulator() : nullptr;
    cISC4Demand* d = ds ? fc4::AsGame(ds)->GetDemand(id, 0) : nullptr;  // slot 6, borrowed
    if (d == nullptr) return false;
    if (out != nullptr) *out = fc4::AsGame(d)->QuerySupplyValue();  // slot 8, ST0
    return true;
}

// §1.3 step 7: a new cSC4Demand (GZCOM), one reference, not Inited.
uint64_t CreateConstructionNode(void* u) {
    if (!Writes(u)) return 0;
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (com == nullptr || !com->GetClassObject(kCLSID_cSC4Demand, kIID_cISC4Demand, &p) || p == nullptr) {
        fc4::Log("development: WARNING could not create the construction node (class 0x%08X)", kCLSID_cSC4Demand);
        return 0;
    }
    return NoteCreated(u, p);
}

void NodeInit(void*, uint64_t n) {
    if (cFC4ISC4Demand* d = NodeOf(n)) d->Init();  // slot 3
}
void NodeShutdown(void*, uint64_t n) {
    if (cFC4ISC4Demand* d = NodeOf(n)) d->Shutdown();  // slot 4
}
float NodeSupply(void*, uint64_t n) {
    cFC4ISC4Demand* d = NodeOf(n);
    return d ? d->QuerySupplyValue() : 0.0f;  // slot 8
}
float NodeDemand(void*, uint64_t n) {
    cFC4ISC4Demand* d = NodeOf(n);
    return d ? d->QueryDemandValue() : 0.0f;  // slot 9
}
void NodeSetSupply(void*, uint64_t n, float v) {
    if (cFC4ISC4Demand* d = NodeOf(n)) d->SetSupplyValue(v);  // slot 18
}
void NodeSetDemand(void*, uint64_t n, float v) {
    if (cFC4ISC4Demand* d = NodeOf(n)) d->SetDemandValue(v);  // slot 19
}
// Slot 17 AddToDemandValue. [spec question] §8 gives no return channel; the
// result is not used (declared bool in AL, as the other setters).
void NodeAddToDemand(void*, uint64_t n, float v) {
    if (cFC4ISC4Demand* d = NodeOf(n)) d->AddToDemandValue(v);
}
bool NodeEndOfCycle(void*, uint64_t n) {
    cFC4ISC4Demand* d = NodeOf(n);
    return d != nullptr && d->EndOfCycle();  // slot 30
}
float DemandActiveValue(void*, uint64_t n) {
    cFC4ISC4Demand* d = NodeOf(n);
    return d ? d->QueryActiveDemandValue() : 0.0f;  // slot 12
}

// ---- tract developer (§4.4, §4.5) ------------------------------------------------------------------

cFC4ISC4TractDeveloper* TractDeveloper(void* u, const char* what) {
    if (!Gate(u, fc4::Sw::DevelopmentTract, what)) return nullptr;
    cISC4City* c = CityOf(u);
    cISC4TractDeveloper* td = c ? c->GetTractDeveloper() : nullptr;  // borrowed
    return td ? fc4::AsGame(td) : nullptr;
}

FC4GrowthProfile GameProfile(const fc4_growth_profile* p) {
    FC4GrowthProfile g{};
    g.developer = reinterpret_cast<cISC4GrowthDeveloper*>(FromToken(p->developer));
    g.x = p->x;
    g.z = p->z;
    g.stage = static_cast<uint8_t>(p->stage);  // the tract developer reads the low byte
    return g;
}

// Slot 5 Grow(profile, outcomes, flag): the outcome list is a B list made and
// freed here (§9.2).
int32_t TractGrow(void* u, const fc4_growth_profile* profile, bool* flag) {
    if (profile == nullptr || !Writes(u)) return 0;
    cFC4ISC4TractDeveloper* td = TractDeveloper(u, "development tract Grow (nothing built)");
    if (td == nullptr) return 0;
    fc4::GameList<int32_t> outcomes;
    if (!outcomes.valid()) {
        NoteOnce(g_noteNoAllocator, "development: the game's STL allocator is not available; no lot configurations "
                                    "are read and the game containers are not built");
        return 0;
    }
    const FC4GrowthProfile g = GameProfile(profile);
    bool f = flag != nullptr && *flag;
    const int32_t built = td->Grow(g, outcomes, f);
    if (flag != nullptr) *flag = f;
    ++Ctx(u)->grows;
    return built;
}

int32_t TractDecline(void* u, const fc4_growth_profile* profile) {
    if (profile == nullptr || !Writes(u)) return 0;
    cFC4ISC4TractDeveloper* td = TractDeveloper(u, "development tract Decline (nothing declined)");
    if (td == nullptr) return 0;
    ++Ctx(u)->declines;
    return td->Decline(GameProfile(profile));  // slot 6
}

// ---- lots (§3.2, §4.8-§4.11) -----------------------------------------------------------------------

uint32_t LotBuildingType(void* u, uint64_t lot) {
    if (lot == 0 || !Gate(u, fc4::Sw::DevelopmentLots, "development lot GetBuildingType (0)")) return 0;
    return fc4::AsGame(LotOf(lot))->GetBuildingType(false);  // slot 46
}

void LotBoundingRect(void* u, uint64_t lot, fc4_cell_rect* out) {
    if (out == nullptr) return;
    *out = fc4_cell_rect{};
    if (lot == 0 || !Gate(u, fc4::Sw::DevelopmentLots, "development lot GetBoundingRect (empty)")) return;
    SC4Rect<int32_t> r;
    fc4::AsGame(LotOf(lot))->GetBoundingRect(r);  // slot 14
    out->x1 = r.topLeftX;
    out->z1 = r.topLeftY;
    out->x2 = r.bottomRightX;
    out->z2 = r.bottomRightY;
}

// §4.10: lot.GetLotConfiguration().GetGrowthStage() (slots 9, then 11 in AL).
bool LotGrowthStage(void* u, uint64_t lot, uint8_t* out) {
    if (lot == 0 || !Gate(u, fc4::Sw::DevelopmentLots, "development lot growth stage (none)")) return false;
    cISC4LotConfiguration* cfg = fc4::AsGame(LotOf(lot))->GetLotConfiguration();
    if (cfg == nullptr) return false;
    if (out != nullptr) *out = cfg->GetGrowthStage();
    return true;
}

// Lot slot 48 GetBuilding (not reference-counted), as its cISC4Occupant
// (slot 3 AsOccupant): the core's occupant tokens are cISC4Occupant*.
uint64_t LotBuilding(void* u, uint64_t lot) {
    if (lot == 0 || !Gate(u, fc4::Sw::DevelopmentLots, "development lot GetBuilding (none)")) return 0;
    cISC4BuildingOccupant* b = fc4::AsGame(LotOf(lot))->GetBuilding();
    return b ? TokenOf(b->AsOccupant()) : 0;
}

uint32_t LotState(void* u, uint64_t lot) {
    if (lot == 0 || !Gate(u, fc4::Sw::DevelopmentLots, "development lot GetState (0)")) return 0;
    return static_cast<uint32_t>(fc4::AsGame(LotOf(lot))->GetState());  // slot 33
}

// Slot 15 GetCenter: the result through the out pointer only (crime.md §13).
void LotCenter(void* u, uint64_t lot, float* out3) {
    if (out3 == nullptr) return;
    out3[0] = out3[1] = out3[2] = 0.0f;
    if (lot == 0 || !Gate(u, fc4::Sw::DevelopmentLots, "development lot GetCenter (origin)")) return;
    float v[4] = {0.0f, 0.0f, 0.0f, 0.0f};
    fc4::AsGame(LotOf(lot))->GetCenter(v);
    out3[0] = v[0];
    out3[1] = v[1];
    out3[2] = v[2];
}

// §4.11 "lot.GetSlope()": §8 gives lot slot 19 (+0x4C, ST0). [spec question]
// FreeCity4's cISC4Lot declaration names slot 19 GetSlopeAtTimeOfConstruction
// (slot 18 is GetSlope); the spec's slot number is followed.
float LotSlope(void* u, uint64_t lot) {
    if (lot == 0 || !Gate(u, fc4::Sw::DevelopmentLots, "development lot slope (0)")) return 0.0f;
    return fc4::AsGame(LotOf(lot))->GetSlopeAtTimeOfConstruction();  // slot 19
}

bool LotDemolishable(void* u, uint64_t lot) {
    if (lot == 0 || !Gate(u, fc4::Sw::DevelopmentLots, "development lot IsInDemolishableState (false)")) return false;
    return fc4::AsGame(LotOf(lot))->IsInDemolishableState();  // slot 39
}

// Lot manager slot 6 GetLot(x, z, true): borrowed.
uint64_t LotAt(void* u, int32_t x, int32_t z) {
    cFC4ISC4LotManager* lm = Lots(u, "development GetLot (none)");
    return lm ? TokenOf(lm->GetLot(x, z, true)) : 0;
}

// Lot manager slot 7 GetLotsInCellRect(list, x1, z1, x2, z2, false): a B list
// holding one reference per lot (allocators.md); the references move to the
// context and are released when the outermost call returns.
void LotsInCellRect(void* u, const fc4_cell_rect* r, fc4_token_callback cb, void* ctx) {
    if (r == nullptr || cb == nullptr) return;
    cFC4ISC4LotManager* lm = Lots(u, "development GetLotsInCellRect (none)");
    if (lm == nullptr) return;
    fc4::GameList<cISC4Lot*> list;
    if (!list.valid()) {
        NoteOnce(g_noteNoAllocator, "development: the game's STL allocator is not available; no lot configurations "
                                    "are read and the game containers are not built");
        return;
    }
    lm->GetLotsInCellRect(list, r->x1, r->z1, r->x2, r->z2, false);
    std::vector<uint64_t> tokens;
    for (auto* n = list.head()->next; n != list.head(); n = n->next) {
        cISC4Lot* l = n->value;
        n->value = nullptr;  // the reference is the context's now
        if (l == nullptr) continue;
        FC4_ACQUIRED(l, "GetLotsInCellRect");
        Ctx(u)->held.push_back(l);
        tokens.push_back(TokenOf(l));
    }
    for (uint64_t t : tokens) cb(ctx, t);
}

// Lot developer slot 30 GetLotSlope(lot, false), ST0.
float LotDeveloperSlope(void* u, uint64_t lot) {
    cISC4LotDeveloper* ld = LotDeveloper(u, "development GetLotSlope (0)");
    return (ld != nullptr && lot != 0) ? ld->GetLotSlope(LotOf(lot), false) : 0.0f;
}

// Lot developer slot 37 UpdateLotElements(lot, kind): the whole 32-bit kind (§4.11).
void UpdateLotElements(void* u, uint64_t lot, uint32_t kind) {
    if (!Writes(u)) return;
    cISC4LotDeveloper* ld = LotDeveloper(u, "development UpdateLotElements");
    if (ld == nullptr || lot == 0) return;
    ld->UpdateLotElements(LotOf(lot), kind);
    ++Ctx(u)->lotElementUpdates;
}

// Lot developer slot 16, the float overload of GetLowestOrHighestAreaAltitude
// (MSVC reverses the overloads: float at 16, int32 at 17), (rect, false, true).
float AreaAltitude(void* u, const float* r) {
    cISC4LotDeveloper* ld = LotDeveloper(u, "development GetLowestOrHighestAreaAltitude (0)");
    if (ld == nullptr || r == nullptr) return 0.0f;
    return ld->GetLowestOrHighestAreaAltitude(SC4Rect<float>(r[0], r[1], r[2], r[3]), false, true);
}

// ---- zones, land value, power, demolition, budget (§4.8, §4.11) ------------------------------------

// Zone manager slot 12 (EAX = int8 sign-extended). Outside the city: 0.
int8_t ZoneTypeAt(void* u, int32_t x, int32_t z) {
    cISC4City* c = CityOf(u);
    if (c == nullptr || x < 0 || z < 0) return 0;
    if (static_cast<uint32_t>(x) >= c->CellCountX() || static_cast<uint32_t>(z) >= c->CellCountZ()) return 0;
    cISC4ZoneManager* zm = Zones(u, "development GetZoneType (unzoned)");
    return zm ? static_cast<int8_t>(static_cast<int32_t>(zm->GetZoneType(x, z))) : 0;
}

// The cRZCellMap<bool> layout with a vtable of our own (slot 0, the
// destructor, is the only virtual; the zone manager never destroys it).
class CellMapStorage {
public:
    virtual ~CellMapStorage() = default;
    uint32_t width = 0;
    uint32_t height = 0;
    uint32_t words = 0;
    uint32_t** rows = nullptr;
};

// A full rectangle as the game's SC4CellRegion<long> (zones.md, 0x24 bytes:
// x1, z1, x2, z2, then the cell map; bit (dx, dz) is bit dz % 32 of
// rows[dx][dz / 32]). The memory is ours; PlaceZone only clears bits.
class RegionBuffer {
public:
    bool Build(const fc4_cell_rect& r) {
        const int64_t w = int64_t{r.x2} - r.x1 + 1;
        const int64_t h = int64_t{r.z2} - r.z1 + 1;
        if (w <= 0 || h <= 0 || w * h > kMaxRegionCells) return false;
        layout_.x1 = r.x1;
        layout_.z1 = r.z1;
        layout_.x2 = r.x2;
        layout_.z2 = r.z2;
        const uint32_t words = static_cast<uint32_t>((h + 31) / 32);
        bits_.assign(static_cast<size_t>(w) * words, 0);
        rows_.resize(static_cast<size_t>(w));
        for (int64_t dx = 0; dx < w; ++dx) {
            uint32_t* row = &bits_[static_cast<size_t>(dx) * words];
            rows_[static_cast<size_t>(dx)] = row;
            for (int64_t dz = 0; dz < h; ++dz) row[dz / 32] |= 1u << (dz % 32);
        }
        layout_.map.width = static_cast<uint32_t>(w);
        layout_.map.height = static_cast<uint32_t>(h);
        layout_.map.words = words;
        layout_.map.rows = rows_.data();
        return true;
    }
    SC4CellRegion<int32_t>& Zoning() { return *reinterpret_cast<SC4CellRegion<int32_t>*>(&layout_); }

private:
    struct Layout {
        int32_t x1 = 0, z1 = 0, x2 = 0, z2 = 0;
        CellMapStorage map;
    };
    static_assert(sizeof(void*) != 4 || sizeof(Layout) == 0x24, "SC4CellRegion<long> layout");
    Layout layout_;
    std::vector<uint32_t> bits_;
    std::vector<uint32_t*> rows_;
};

// §4.11: PlaceZone(region, zone 0, true, flag, false, true, true, null, null, 0).
void Dezone(void* u, const fc4_cell_rect* r, bool flag) {
    if (r == nullptr || !Writes(u)) return;
    cISC4ZoneManager* zm = Zones(u, "development dezoning PlaceZone");
    if (zm == nullptr) return;
    RegionBuffer rb;
    if (!rb.Build(*r)) {
        NoteOnce(g_noteRegion, "development: WARNING a dezoning rectangle is empty or too large; not dezoned");
        return;
    }
    zm->PlaceZone(rb.Zoning(), static_cast<cISC4ZoneManager::ZoneType>(0), true, flag, false, true, true, nullptr,
                  nullptr, 0);
    ++Ctx(u)->dezones;
}

// Power simulator slot 19 (AL).
bool Powered(void* u, int32_t x, int32_t z) {
    if (!Gate(u, fc4::Sw::DevelopmentCity, "development power GetSupplyValue (unpowered)")) return false;
    fc4::DevelopmentContext* c = Ctx(u);
    cISC4PowerSimulator* p = static_cast<cISC4PowerSimulator*>(c->power);
    if (p == nullptr) {
        cISC4City* city = CityOf(u);
        p = city ? city->GetPowerSimulator() : nullptr;
    }
    return p != nullptr && fc4::AsGame(p)->GetSupplyValue(x, z);
}

cISC4LandValueSimulator* LandValueSim(void* u) {
    fc4::DevelopmentContext* c = Ctx(u);
    if (c->landValue != nullptr) return static_cast<cISC4LandValueSimulator*>(c->landValue);
    cISC4City* city = CityOf(u);
    return city ? city->GetLandValueSimulator() : nullptr;
}

uint32_t LandValueType(void* u, int32_t x, int32_t z) {
    if (!Gate(u, fc4::Sw::DevelopmentCity, "development GetLandValueType (0)")) return 0;
    cISC4LandValueSimulator* lv = LandValueSim(u);
    return lv ? static_cast<uint32_t>(lv->GetLandValueType(x, z)) : 0;  // slot 5
}

uint8_t LandValue(void* u, int32_t x, int32_t z) {
    if (!Gate(u, fc4::Sw::DevelopmentCity, "development GetLandValue (0)")) return 0;
    cISC4LandValueSimulator* lv = LandValueSim(u);
    return lv ? lv->GetLandValue(x, z) : 0;  // slot 6, AL
}

// §4.11: DemolishLots(true, lots, 0, 1, true, null, &cost, 0) (demolition slot
// 7); the list holds one reference per lot, released by its destructor.
int64_t DemolishLots(void* u, const uint64_t* lots, size_t n) {
    if (!Writes(u)) return 0;
    if (!Gate(u, fc4::Sw::DevelopmentDemolition, "development DemolishLots (cost 0)")) return 0;
    cISC4City* c = CityOf(u);
    cISC4Demolition* d = c ? c->GetDemolitionUtility() : nullptr;
    if (d == nullptr) return 0;
    if (n == 0) return 0;  // §4.11: a replacement may skip the call for an empty list
    cIGZAllocatorServicePtr alloc;
    if (!alloc) return 0;
    SC4List<cISC4Lot*> list;
    for (size_t i = 0; i < n; ++i) {
        cISC4Lot* l = LotOf(lots[i]);
        if (l == nullptr) continue;
        l->AddRef();  // the list's reference, released by its destructor
        list.push_back(l);
    }
    int64_t cost = 0;
    d->DemolishLots(true, list, 0, 1, true, nullptr, &cost, 0);
    ++Ctx(u)->demolitions;
    return cost;
}

cFC4ISC4BudgetSimulator* Budget(void* u) {
    if (!Gate(u, fc4::Sw::DevelopmentDemolition, "development budget funds")) return nullptr;
    cISC4City* c = CityOf(u);
    cISC4BudgetSimulator* b = c ? c->GetBudgetSimulator() : nullptr;
    return b ? fc4::AsGame(b) : nullptr;
}

int64_t TotalFunds(void* u) {
    cFC4ISC4BudgetSimulator* b = Budget(u);
    return b ? b->GetTotalFunds() : 0;  // slot 6
}

void SetTotalFunds(void* u, int64_t v) {
    if (!Writes(u)) return;
    if (cFC4ISC4BudgetSimulator* b = Budget(u)) b->SetTotalFunds(v);  // slot 5
}

// ---- occupants (§3.2, §4.9) ------------------------------------------------------------------------

bool HasOccupantManager(void* u) { return Occupants(u, "development occupant manager (none)") != nullptr; }

// Slot 20 InsertOccupant(occupant, 0) (§4.9: AsOccupant, false).
bool InsertOccupant(void* u, uint64_t occ) {
    if (!Writes(u)) return false;
    cISC4OccupantManager* om = Occupants(u, "development InsertOccupant (refused)");
    if (om == nullptr || occ == 0) return false;
    ++Ctx(u)->inserts;
    return om->InsertOccupant(OccOf(occ), 0);
}

// Slot 21 RemoveOccupant(occupant, true, 0) (§4.9 slot 25).
bool RemoveOccupant(void* u, uint64_t occ) {
    if (!Writes(u)) return false;
    cISC4OccupantManager* om = Occupants(u, "development RemoveOccupant (refused)");
    if (om == nullptr || occ == 0) return false;
    ++Ctx(u)->removals;
    return om->RemoveOccupant(OccOf(occ), true, 0);
}

// §4.9 slot 5: occupant manager slot 33 (the typed occupant-at-cell query)
// for buildings. A found occupant comes with one reference, released here.
bool BuildingOccupantAtCell(void* u, int32_t x, int32_t z) {
    cISC4OccupantManager* om = Occupants(u, "development occupant at cell (none)");
    if (om == nullptr) return false;
    cISC4Occupant* found = nullptr;
    om->GetFirstOccupantByStandardCityCell(found, x, z, kOccupantTypeBuilding);
    if (found == nullptr) return false;
    FC4_ACQUIRED(found, "GetFirstOccupantByStandardCityCell");
    FC4_RELEASE(found);
    return true;
}

const cISCPropertyHolder* HolderOf(uint64_t occ) { return occ ? OccOf(occ)->AsPropertyHolder() : nullptr; }

bool OccupantHasProperty(void* u, uint64_t occ, uint32_t id) {
    if (occ == 0 || !Gate(u, fc4::Sw::DevelopmentOccupants, "development occupant HasProperty (false)")) return false;
    return fc4::props::Has(HolderOf(occ), id);
}

bool OccupantPropertyI64(void* u, uint64_t occ, uint32_t id, int64_t* out) {
    if (occ == 0 || !Gate(u, fc4::Sw::DevelopmentOccupants, "development occupant property (none)")) return false;
    std::vector<int64_t> v;
    if (!fc4::props::GetI64s(HolderOf(occ), id, v) || v.empty()) return false;
    if (out != nullptr) *out = v[0];
    return true;
}

bool OccupantInGroup(void* u, uint64_t occ, uint32_t group) {
    if (occ == 0 || !Gate(u, fc4::Sw::DevelopmentOccupants, "development IsOccupantGroup (false)")) return false;
    return OccOf(occ)->IsOccupantGroup(group);  // slot 22
}

void OccupantPosition(void* u, uint64_t occ, float* out3) {
    if (out3 == nullptr) return;
    out3[0] = out3[1] = out3[2] = 0.0f;
    if (occ == 0 || !Gate(u, fc4::Sw::DevelopmentOccupants, "development occupant GetPosition (origin)")) return;
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);
    out3[0] = v.fX;
    out3[1] = v.fY;
    out3[2] = v.fZ;
}

// §4.9: a new building occupant (GZCOM), one reference for the core.
uint64_t CreateBuildingOccupant(void* u) {
    if (!Writes(u)) return 0;
    if (!Gate(u, fc4::Sw::DevelopmentOccupants, "development building occupant creation (none)")) return 0;
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (com == nullptr || !com->GetClassObject(kCLSID_cSC4BuildingOccupant, kIID_cISC4Occupant, &p) || p == nullptr) {
        fc4::Log("development: WARNING could not create a building occupant (class 0x%08X)",
                 kCLSID_cSC4BuildingOccupant);
        return 0;
    }
    ++Ctx(u)->occupantsCreated;
    return NoteCreated(u, p);
}

// The building occupant interface of an occupant token (borrowed after the
// QueryInterface's reference is dropped: the token holds one).
template <class F>
void WithBuilding(uint64_t occ, F f) {
    cISC4BuildingOccupant* b = Query<cISC4BuildingOccupant>(FromToken(occ), kIID_cISC4BuildingOccupant);
    if (b == nullptr) return;
    f(b);
    Drop(b);
}

void SetBuildingType(void* u, uint64_t occ, uint32_t type) {
    if (occ == 0 || !Gate(u, fc4::Sw::DevelopmentOccupants, "development SetBuildingType")) return;
    WithBuilding(occ, [&](cISC4BuildingOccupant* b) { b->SetBuildingType(type); });
}

// The type's exemplar key: the catalogue mirror, else the process statics.
bool BuildingKey(uint32_t type, fc4_res_key* out);

// §4.9 0x00698490 (LoadBuildingOccupantProperties). [spec question, choice]
// The body is not specified: the type's exemplar (resource manager
// GetResource) becomes the occupant's default exemplar
// (cISCExemplarPropertyHolder 0x0AC2B5F7 slot 3 SetDefaultExemplar). False
// when the type has no key, the exemplar does not load or the occupant has
// no exemplar holder.
bool LoadBuildingOccupant(void* u, uint64_t occ, uint32_t type) {
    if (occ == 0 || !Gate(u, fc4::Sw::DevelopmentOccupants, "development building properties (failed)")) return false;
    fc4_res_key k{};
    if (!BuildingKey(type, &k)) {
        ++Ctx(u)->loadsFailed;
        return false;
    }
    cIGZPersistResourceManagerPtr rm;
    if (!rm) {
        ++Ctx(u)->loadsFailed;
        return false;
    }
    FC4_OUT(cISCResExemplar*, res, "cISCResExemplar* (GetResource out pointer)");
    const bool got = rm->GetResource(cGZPersistResourceKey(k.type_id, k.group, k.instance), GZIID_cISCResExemplar,
                                     reinterpret_cast<void**>(res.ptr()), 0, nullptr);
    res.Check();
    cISCResExemplar* ex = res.get();
    if (!got || ex == nullptr) {
        ++Ctx(u)->loadsFailed;
        return false;
    }
    FC4_ACQUIRED(ex, "GetResource");
    cISCExemplarPropertyHolder* h =
        Query<cISCExemplarPropertyHolder>(FromToken(occ), kIID_cISCExemplarPropertyHolder);
    const bool ok = h != nullptr && h->SetDefaultExemplar(ex);
    Drop(h);
    FC4_RELEASE(ex);
    if (!ok) ++Ctx(u)->loadsFailed;
    return ok;
}

// cISC4Occupant GetBoundingBox(min, max): min then max.
void OccupantBoundingBox(void* u, uint64_t occ, float* out6) {
    if (out6 == nullptr) return;
    for (int i = 0; i < 6; ++i) out6[i] = 0.0f;
    if (occ == 0 || !Gate(u, fc4::Sw::DevelopmentOccupants, "development GetBoundingBox (empty)")) return;
    cS3DVector3 mn, mx;
    OccOf(occ)->GetBoundingBox(&mn, &mx);
    out6[0] = mn.fX;
    out6[1] = mn.fY;
    out6[2] = mn.fZ;
    out6[3] = mx.fX;
    out6[4] = mx.fY;
    out6[5] = mx.fZ;
}

void SetOccupantBoundingBox(void* u, uint64_t occ, const float* box6) {
    if (occ == 0 || box6 == nullptr || !Gate(u, fc4::Sw::DevelopmentOccupants, "development SetBoundingBox")) return;
    WithBuilding(occ, [&](cISC4BuildingOccupant* b) { b->SetBoundingBox(box6, box6 + 3); });
}

void SetOccupantPosition(void* u, uint64_t occ, const float* pos3) {
    if (occ == 0 || pos3 == nullptr || !Gate(u, fc4::Sw::DevelopmentOccupants, "development SetPosition")) return;
    const cS3DVector3 v(pos3[0], pos3[1], pos3[2]);
    OccOf(occ)->SetPosition(&v);
}

void SetOccupantOrientation(void* u, uint64_t occ, int32_t orientation) {
    if (occ == 0 || !Gate(u, fc4::Sw::DevelopmentOccupants, "development SetOrientation")) return;
    WithBuilding(occ, [&](cISC4BuildingOccupant* b) { b->SetOrientation(orientation); });
}

void InitOccupant(void* u, uint64_t occ) {
    if (occ == 0 || !Gate(u, fc4::Sw::DevelopmentOccupants, "development occupant Init")) return;
    OccOf(occ)->Init();
}

// ---- advisors (§4.10) ------------------------------------------------------------------------------

// advisorSystem.GetAdvisor(0x17).PushEvent(event, null) (city slot 81, slots
// 7 and 18). [quirk, §4.10] The subject the core built is discarded: the
// original pushes a null subject, so none is created here.
void PushAdvisorEvent(void* u, uint32_t advisor, uint32_t event, const fc4_advice_subject*) {
    if (!Writes(u)) return;
    if (!Gate(u, fc4::Sw::DevelopmentAdvisor, "development advisor event")) return;
    cISC4City* c = CityOf(u);
    cISC4AdvisorSystem* as = c ? c->GetAdvisorSystem() : nullptr;
    cISC4Advisor* a = as ? as->GetAdvisor(static_cast<int32_t>(advisor)) : nullptr;
    if (a == nullptr) return;
    a->PushEvent(static_cast<int32_t>(event), nullptr);
    ++Ctx(u)->advisorEvents;
}

// ---- growth developers (§1.5, §8) ------------------------------------------------------------------

// §1.5: a cSC4GrowthDeveloper through GZCOM (class 0x0990C138), one reference.
uint64_t CreateDeveloper(void* u, const fc4_res_key*) {
    if (!Writes(u)) return 0;  // the shadow compares the original's developers
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (com == nullptr ||
        !com->GetClassObject(kFC4_CLSID_cSC4GrowthDeveloper, kFC4_IID_cISC4GrowthDeveloper, &p) || p == nullptr) {
        fc4::Log("development: WARNING could not create a growth developer (class 0x%08X)",
                 kFC4_CLSID_cSC4GrowthDeveloper);
        return 0;
    }
    return NoteCreated(u, p);
}

// Slot 3 Init(holder): the game's holder of the developer exemplar.
bool DeveloperInit(void*, uint64_t dev, const fc4_res_key* key) {
    cFC4ISC4GrowthDeveloper* d = DevOf(dev);
    if (d == nullptr || key == nullptr) return false;
    fc4::props::Exemplar ex;
    if (!ex.Load(key->type_id, key->group, key->instance)) return false;
    return d->Init(const_cast<cISCPropertyHolder*>(ex.Holder()));
}

bool DeveloperShutdown(void*, uint64_t dev) {
    cFC4ISC4GrowthDeveloper* d = DevOf(dev);
    return d != nullptr && d->Shutdown();  // slot 4
}

uint32_t DeveloperSubType(void*, uint64_t dev) {
    cFC4ISC4GrowthDeveloper* d = DevOf(dev);
    return d ? d->GetSubType() : 0;  // slot 6
}

uint32_t DeveloperPurpose(void*, uint64_t dev) {
    cFC4ISC4GrowthDeveloper* d = DevOf(dev);
    return d ? d->GetPurpose() : 0;  // slot 7
}

bool DeveloperListsValid(void*, uint64_t dev) {
    cFC4ISC4GrowthDeveloper* d = DevOf(dev);
    return d != nullptr && d->GrowthAndDeclineListsAreValid();  // slot 10
}

// Slots 11 / 12: the 12-byte items. §8: the BDS reads the third field as a
// uint8 stage, so only its low byte is passed on.
void DeveloperList(void*, uint64_t dev, uint32_t which, fc4_growth_items_sink sink, void* ctx) {
    cFC4ISC4GrowthDeveloper* d = DevOf(dev);
    if (d == nullptr || sink == nullptr) return;
    fc4::GameVector<FC4GrowthItem>& v = which == 0 ? d->GetGrowthList() : d->GetDeclineList();
    std::vector<fc4_growth_item> items;
    items.reserve(v.size());
    for (FC4GrowthItem& i : v) items.push_back(fc4_growth_item{i.x, i.z, static_cast<int32_t>(i.maxStage & 0xFF)});
    sink(ctx, items.empty() ? nullptr : items.data(), items.size());
}

uint64_t DeveloperSourceDemand(void*, uint64_t dev) {
    cFC4ISC4GrowthDeveloper* d = DevOf(dev);
    return d ? TokenOf(d->GetSourceDemand()) : 0;  // slot 18, borrowed
}

void DeveloperStartMapUpdate(void*, uint64_t dev) {
    if (cFC4ISC4GrowthDeveloper* d = DevOf(dev)) d->StartMapUpdate();  // slot 27
}

bool DeveloperMapUpdateFinished(void*, uint64_t dev) {
    cFC4ISC4GrowthDeveloper* d = DevOf(dev);
    return d != nullptr && d->IsMapUpdateFinished();  // slot 28
}

// ---- the catalogue mirror (§4.7, §9.1) -------------------------------------------------------------
//
// Process-wide, like the catalogue: built once after the catalogue loaded,
// never freed. Names are game cRZStrings (or SDK strings without the game's
// allocator), profiles 0x40-byte BuildingProfiles with B capacity maps; both
// keep their addresses.

class MirrorLock {
public:
    MirrorLock();
    ~MirrorLock();
    MirrorLock(const MirrorLock&) = delete;
    MirrorLock& operator=(const MirrorLock&) = delete;
};

struct Mirror {
    CRITICAL_SECTION cs;
    bool built = false;
    std::vector<uint32_t> types, families, foundations, walls;
    std::unordered_set<uint32_t> typeSet, familySet;
    std::unordered_map<uint32_t, std::vector<uint32_t>> members;
    std::unordered_map<uint32_t, fc4_res_key> keys[4];  // 0 building, 2 foundation, 3 wall
    std::unordered_map<uint32_t, cIGZString*> names[2];  // 0 exemplar name, 1 building name
    std::unordered_map<uint32_t, FC4BuildingProfile*> profiles;
    std::unordered_map<uint32_t, float> maxCapacity;  // (zone << 8) | key
    bool gameStrings = false;
    Mirror() { InitializeCriticalSection(&cs); }
};

Mirror& TheMirror() {
    static Mirror* m = new Mirror();  // never destroyed: the game may hold its pointers until the process ends
    return *m;
}

MirrorLock::MirrorLock() { EnterCriticalSection(&TheMirror().cs); }
MirrorLock::~MirrorLock() { LeaveCriticalSection(&TheMirror().cs); }

// A stored name (§4.7.2: owned by the BDS; callers never release it).
cIGZString* NewName(const std::string& text) {
    if (fc4::StlNodeAllocator::Available()) {
        auto* s = new fc4::GameString();
        if (fc4::GameStringInit<fc4::StlNodeAllocator>(*s, text.data(), text.size())) {
            TheMirror().gameStrings = true;
            return reinterpret_cast<cIGZString*>(s);
        }
        delete s;
    }
    // [choice] Without the game's string class: an SDK string (same interface).
    return new cRZBaseString(text.c_str(), static_cast<uint32_t>(text.size()));
}

void SetCapacities(FC4BuildingProfile& p, const std::vector<std::pair<uint32_t, uint32_t>>& caps, bool present) {
    p.capacityBegin = p.capacityEnd = p.capacityCap = nullptr;
    // [spec question, choice] A profile with a capacity table has one map in
    // its vector; one without has an empty vector.
    if (!present || !fc4::StlNodeAllocator::Available()) return;
    auto* m = static_cast<fc4::GameMapObject*>(fc4::StlNodeAllocator::Allocate(sizeof(fc4::GameMapObject)));
    if (m == nullptr) return;
    m->header = nullptr;
    m->count = 0;
    m->comparator = 0;
    std::vector<uint32_t> keys, values;
    for (const auto& kv : caps) {
        keys.push_back(kv.first);
        values.push_back(kv.second & 0xFFFFu);
    }
    if (!fc4::BuildGameMap<fc4::StlNodeAllocator>(*m, keys.data(), values.data(), keys.size())) {
        fc4::StlNodeAllocator::Free(m);
        fc4::Log("development: WARNING a building profile's capacity map could not be built");
        return;
    }
    p.capacityBegin = m;
    p.capacityEnd = m + 1;
    p.capacityCap = m + 1;
}

void TypeValue(void* ctx, uint32_t key, uint32_t value) {
    static_cast<std::vector<std::pair<uint32_t, uint32_t>>*>(ctx)->emplace_back(key, value);
}

std::vector<uint32_t> CatalogueList(const fc4_development_process* pp, uint32_t which) {
    std::vector<uint32_t> v(fc4_development_catalogue_types(pp, which, nullptr, 0));
    if (!v.empty()) v.resize(fc4_development_catalogue_types(pp, which, v.data(), v.size()));
    return v;
}

void BytesToString(void* ctx, const uint8_t* data, size_t len) {
    if (data != nullptr && len != 0) static_cast<std::string*>(ctx)->append(reinterpret_cast<const char*>(data), len);
}

void DensityEntry(void* ctx, uint32_t zone, uint32_t key, float, float maxCapacity) {
    (*static_cast<std::unordered_map<uint32_t, float>*>(ctx))[(zone << 8) | (key & 0xFF)] = maxCapacity;
}

// Builds the mirror from the process statics (process lock held).
void BuildMirror(const fc4_development_process* pp) {
    if (pp == nullptr || !fc4_development_catalogue_loaded(pp)) return;
    MirrorLock lock;
    Mirror& m = TheMirror();
    if (m.built) return;
    m.types = CatalogueList(pp, 0);
    m.families = CatalogueList(pp, 1);
    m.foundations = CatalogueList(pp, 2);
    m.walls = CatalogueList(pp, 3);
    m.typeSet.insert(m.types.begin(), m.types.end());
    m.familySet.insert(m.families.begin(), m.families.end());
    for (uint32_t f : m.families) {
        std::vector<uint32_t> v(fc4_development_catalogue_family(pp, f, nullptr, 0));
        if (!v.empty()) v.resize(fc4_development_catalogue_family(pp, f, v.data(), v.size()));
        m.members[f] = std::move(v);
    }
    const std::pair<uint32_t, const std::vector<uint32_t>*> keyed[] = {{0, &m.types}, {2, &m.foundations},
                                                                       {3, &m.walls}};
    for (const auto& kv : keyed) {
        for (uint32_t t : *kv.second) {
            fc4_res_key k{};
            if (fc4_development_catalogue_key(pp, kv.first, t, &k)) m.keys[kv.first][t] = k;
        }
    }
    for (uint32_t t : m.types) {
        for (uint32_t which = 0; which < 2; ++which) {
            std::string text;
            fc4_development_catalogue_name(pp, which, t, &BytesToString, &text);
            auto it = m.names[which].find(t);
            if (it == m.names[which].end()) m.names[which][t] = NewName(text);
            // [choice] A name inserted for the type before the catalogue
            // loaded ("" for an unknown type) is kept as it is.
        }
    }
    uint32_t profiles = 0;
    for (uint32_t t : CatalogueList(pp, 4)) {
        fc4_building_profile bp{};
        std::vector<std::pair<uint32_t, uint32_t>> caps;
        if (!fc4_development_catalogue_profile(pp, t, &bp, &TypeValue, &caps)) continue;
        FC4BuildingProfile*& slot = m.profiles[t];
        if (slot == nullptr) {
            slot = new FC4BuildingProfile();
            std::memset(static_cast<void*>(slot), 0, sizeof(*slot));
        }
        // [spec question, choice] A zero profile inserted before the
        // catalogue loaded is replaced in place (its address is kept).
        FC4BuildingProfile& p = *slot;
        p.purpose = bp.purpose;
        p.wealth = bp.wealth;
        p.capacity = bp.capacity;
        p.duration = bp.value_499afa38;
        p.flag27812836 = bp.flag_27812836;
        p.categoryFlags = bp.category_flags;
        std::memcpy(p.occupantSize, bp.occupant_size, sizeof(p.occupantSize));
        p.plopCost = bp.plop_cost;
        // [spec question, choice] The budget item cost (+0x38) comes from the
        // budget simulator in the original (§4.7.1, not specified): 0.
        p.budgetItemCost = 0;
        SetCapacities(p, caps, bp.n_capacities != 0);
        ++profiles;
    }
    fc4_development_catalogue_density(pp, &DensityEntry, &m.maxCapacity);
    m.built = true;
    fc4::Log("development catalogue: %u building types, %u families, %u foundations, %u walls, %u profiles, %u "
             "density entries, names as %s",
             static_cast<unsigned>(m.types.size()), static_cast<unsigned>(m.families.size()),
             static_cast<unsigned>(m.foundations.size()), static_cast<unsigned>(m.walls.size()), profiles,
             static_cast<unsigned>(m.maxCapacity.size()), m.gameStrings ? "game strings" : "SDK strings");
}

bool BuildingKey(uint32_t type, fc4_res_key* out) {
    {
        MirrorLock lock;
        Mirror& m = TheMirror();
        if (m.built) {
            auto it = m.keys[0].find(type);
            if (it == m.keys[0].end()) return false;
            *out = it->second;
            return true;
        }
    }
    return fc4_development_catalogue_key(fc4::DevelopmentProcess(), 0, type, out);  // the process lock is held
}

// §4.7.2: slot 39 inserts an all-zero profile for an unknown type.
FC4BuildingProfile* MirrorProfile(uint32_t type, bool insert) {
    MirrorLock lock;
    Mirror& m = TheMirror();
    auto it = m.profiles.find(type);
    if (it != m.profiles.end()) return it->second;
    if (!insert) return nullptr;
    auto* p = new FC4BuildingProfile();
    std::memset(static_cast<void*>(p), 0, sizeof(*p));
    m.profiles[type] = p;
    return p;
}

cIGZString* MirrorName(uint32_t which, uint32_t type) {
    MirrorLock lock;
    Mirror& m = TheMirror();
    auto it = m.names[which].find(type);
    if (it != m.names[which].end()) return it->second;
    cIGZString* s = NewName(std::string());  // §4.7.2: "" inserted for an unknown type
    m.names[which][type] = s;
    return s;
}

// A copy of a mirror list (so no game code runs under the mirror lock).
std::vector<uint32_t> MirrorTypes(uint32_t which) {
    MirrorLock lock;
    Mirror& m = TheMirror();
    switch (which) {
    case 0:
        return m.types;
    case 1:
        return m.families;
    case 2:
        return m.foundations;
    default:
        return m.walls;
    }
}

bool MirrorFamily(uint32_t f, std::vector<uint32_t>* out) {
    MirrorLock lock;
    Mirror& m = TheMirror();
    auto it = m.members.find(f);
    if (it == m.members.end()) return false;
    if (out != nullptr) *out = it->second;
    return true;
}

bool MirrorIsType(uint32_t t) {
    MirrorLock lock;
    return TheMirror().typeSet.count(t) != 0;
}

bool MirrorKey(uint32_t which, uint32_t t, cGZPersistResourceKey& key) {
    MirrorLock lock;
    Mirror& m = TheMirror();
    auto it = m.keys[which].find(t);
    if (it == m.keys[which].end()) return false;
    key.type = it->second.type_id;
    key.group = it->second.group;
    key.instance = it->second.instance;
    return true;
}

uint32_t FillVector(fc4::GameVector<uint32_t>& v, const std::vector<uint32_t>& src) {
    v.clear();  // end = begin, the buffer kept (§4.7.2)
    for (uint32_t t : src) {
        if (!v.push_back(t)) break;
    }
    return static_cast<uint32_t>(v.size());
}

// The game's cISC4BuildingTypeFilter (only forward-declared by the SDK): §4.7.2
// calls its slot 3 with the type and tests the result. [spec question] The
// spec gives no signature; declared bool(uint32) in AL.
class FC4BuildingTypeFilter : public cIGZUnknown {
public:
    virtual bool IsBuildingTypeIncluded(uint32_t type) = 0;  // 3
};

// ---- the session seed ------------------------------------------------------------------------------

uint32_t SessionSeedValue(bool* fixed) {
    const std::string s = fc4::Setting("development_seed", "FC4_DEVELOPMENT_SEED");
    if (!s.empty()) {
        char* end = nullptr;
        const unsigned long v = std::strtoul(s.c_str(), &end, 0);
        if (end != s.c_str()) {
            if (fixed != nullptr) *fixed = true;
            return static_cast<uint32_t>(v);
        }
    }
    if (fixed != nullptr) *fixed = false;
    constexpr int64_t kRzTimeUnixOffset = 2177452800;  // 1901-01-01 to 1970-01-01 (cRZTime)
    return static_cast<uint32_t>(static_cast<int64_t>(std::time(nullptr)) + kRzTimeUnixOffset);
}

}  // namespace

// ---- fc4 helpers ----------------------------------------------------------------------------------

namespace fc4 {

DevelopmentLock::DevelopmentLock(uint32_t otherThreadMs) {
    CRITICAL_SECTION* cs = TheLock();
    if (OnDevelopmentMainThread()) {
        EnterCriticalSection(cs);
        held_ = true;
        return;
    }
    if (TryEnterCriticalSection(cs)) {
        held_ = true;
        return;
    }
    const ULONGLONG until = GetTickCount64() + otherThreadMs;
    while (GetTickCount64() < until) {
        Sleep(1);
        if (TryEnterCriticalSection(cs)) {
            held_ = true;
            return;
        }
    }
}

DevelopmentLock::~DevelopmentLock() {
    if (held_) LeaveCriticalSection(TheLock());
}

bool OnDevelopmentMainThread() {
    const DWORD main = g_mainTid.load();
    return main == 0 || main == GetCurrentThreadId();
}

uint32_t DevelopmentSessionSeed(bool* fixed) {
    static bool known = false;
    static bool wasFixed = false;
    static uint32_t seed = 0;
    if (!known) {
        seed = SessionSeedValue(&wasFixed);
        known = true;
    }
    if (fixed != nullptr) *fixed = wasFixed;
    return seed;
}

fc4_development_process* DevelopmentProcess() {
    static fc4_development_process* pp = nullptr;
    if (pp == nullptr) {
        bool fixed = false;
        const uint32_t seed = DevelopmentSessionSeed(&fixed);
        pp = fc4_development_process_create(seed);
        fc4::Log("development: process statics %p, session seed 0x%08X (%s)", static_cast<void*>(pp), seed,
                 fixed ? "development_seed" : "wall clock");
    }
    return pp;
}

fc4_growth_statics* GrowthStatics() {
    static fc4_growth_statics* gs = nullptr;
    if (gs == nullptr) {
        bool fixed = false;
        const uint32_t seed = DevelopmentSessionSeed(&fixed);
        gs = fc4_growth_statics_create(seed);
        fc4::Log("development: growth statics %p, session seed 0x%08X (%s)", static_cast<void*>(gs), seed,
                 fixed ? "development_seed" : "wall clock");
    }
    return gs;
}

uint64_t DevelopmentDeveloperOf(uint32_t developerType) {
    cFC4BuildingDevelopmentSimulator* b = g_live;
    if (b == nullptr) return 0;
    return TokenOf(b->GetDeveloper(developerType));
}

fc4_development_host MakeDevelopmentHost(DevelopmentContext* ctx, const HostGridContext* grids) {
    fc4_development_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.grids = MakeGridApi(grids);
    h.sim_day = &SimDay;
    h.difficulty = &Difficulty;
    h.cell_widths = &CellWidths;
    h.terrain_cell_counts = &TerrainCellCounts;
    h.position_to_cell = &PositionToCell;
    h.cell_center = &CellCenter;
    h.altitude = &Altitude;
    h.exemplars = &Exemplars;
    h.lot_configurations = &LotConfigurations;
    h.tuning = &Tuning;
    h.register_static_cleanup = &RegisterStaticCleanup;
    h.post_building_types_updated = &PostBuildingTypesUpdated;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.register_command = &RegisterCommand;
    h.unregister_command = &UnregisterCommand;
    h.add_agent = &AddAgent;
    h.remove_agent = &RemoveAgent;
    h.register_cheat = &RegisterCheat;
    h.unregister_cheat = &UnregisterCheat;
    h.acquire_services = &AcquireServices;
    h.release_services = &ReleaseServices;
    h.census_supply = &CensusSupply;
    h.create_construction_node = &CreateConstructionNode;
    h.node_init = &NodeInit;
    h.node_shutdown = &NodeShutdown;
    h.node_supply = &NodeSupply;
    h.node_demand = &NodeDemand;
    h.node_set_supply = &NodeSetSupply;
    h.node_set_demand = &NodeSetDemand;
    h.node_add_to_demand = &NodeAddToDemand;
    h.node_end_of_cycle = &NodeEndOfCycle;
    h.demand_active_value = &DemandActiveValue;
    h.tract_grow = &TractGrow;
    h.tract_decline = &TractDecline;
    h.lot_building_type = &LotBuildingType;
    h.lot_bounding_rect = &LotBoundingRect;
    h.lot_growth_stage = &LotGrowthStage;
    h.lot_building = &LotBuilding;
    h.lot_state = &LotState;
    h.lot_center = &LotCenter;
    h.lot_slope = &LotSlope;
    h.lot_demolishable = &LotDemolishable;
    h.lot_at = &LotAt;
    h.lots_in_cell_rect = &LotsInCellRect;
    h.lot_developer_slope = &LotDeveloperSlope;
    h.update_lot_elements = &UpdateLotElements;
    h.area_altitude = &AreaAltitude;
    h.zone_type = &ZoneTypeAt;
    h.dezone = &Dezone;
    h.powered = &Powered;
    h.land_value_type = &LandValueType;
    h.land_value = &LandValue;
    h.demolish_lots = &DemolishLots;
    h.total_funds = &TotalFunds;
    h.set_total_funds = &SetTotalFunds;
    h.has_occupant_manager = &HasOccupantManager;
    h.insert_occupant = &InsertOccupant;
    h.remove_occupant = &RemoveOccupant;
    h.building_occupant_at_cell = &BuildingOccupantAtCell;
    h.occupant_has_property = &OccupantHasProperty;
    h.occupant_property_i64 = &OccupantPropertyI64;
    h.occupant_in_group = &OccupantInGroup;
    h.occupant_position = &OccupantPosition;
    h.create_building_occupant = &CreateBuildingOccupant;
    h.set_building_type = &SetBuildingType;
    h.load_building_occupant = &LoadBuildingOccupant;
    h.occupant_bounding_box = &OccupantBoundingBox;
    h.set_occupant_bounding_box = &SetOccupantBoundingBox;
    h.set_occupant_position = &SetOccupantPosition;
    h.set_occupant_orientation = &SetOccupantOrientation;
    h.init_occupant = &InitOccupant;
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.push_advisor_event = &PushAdvisorEvent;
    h.create_developer = &CreateDeveloper;
    h.developer_init = &DeveloperInit;
    h.developer_shutdown = &DeveloperShutdown;
    h.developer_sub_type = &DeveloperSubType;
    h.developer_purpose = &DeveloperPurpose;
    h.developer_lists_valid = &DeveloperListsValid;
    h.developer_list = &DeveloperList;
    h.developer_source_demand = &DeveloperSourceDemand;
    h.developer_start_map_update = &DeveloperStartMapUpdate;
    h.developer_map_update_finished = &DeveloperMapUpdateFinished;
    return h;
}

void DropHeldTokens(DevelopmentContext* ctx) {
    std::vector<cIGZUnknown*> held;
    held.swap(ctx->held);
    for (cIGZUnknown* o : held) {
        if (o == nullptr) continue;
        if (fc4::IsOurObject(o)) {
            o->Release();
        } else {
            FC4_RELEASE(o);
        }
    }
}

void ReleaseDevelopmentContext(DevelopmentContext* ctx) {
    DropHeldTokens(ctx);
    for (uint32_t id : std::vector<uint32_t>(ctx->cheats)) UnregisterCheat(ctx, id);
    if (ctx->commandId != 0) UnregisterCommand(ctx, ctx->commandId);
    RemoveAgent(ctx);
    ReleaseServices(ctx);
}

}  // namespace fc4

// ---- the object -----------------------------------------------------------------------------------

// Every call into the core: the process lock (the main thread waits; another
// thread waits at most 20 ms) and the call depth. When the outermost call
// returns, the game copies are refreshed and the held tokens dropped.
class cFC4BuildingDevelopmentSimulator::Call {
public:
    explicit Call(const cFC4BuildingDevelopmentSimulator* s)
        : s_(const_cast<cFC4BuildingDevelopmentSimulator*>(s)) {
        if (lock_.held()) {
            ++s_->depth_;
        } else {
            ++s_->offMainCalls_;
        }
    }
    ~Call() {
        if (!lock_.held()) return;
        if (--s_->depth_ == 0) {
            s_->Publish();
            fc4::DropHeldTokens(&s_->ctx_);
        }
    }
    Call(const Call&) = delete;
    Call& operator=(const Call&) = delete;
    bool ok() const { return lock_.held() && s_->core_ != nullptr; }

private:
    cFC4BuildingDevelopmentSimulator* s_;
    fc4::DevelopmentLock lock_;
};

cIGZUnknown* CreateFC4BuildingDevelopmentSimulator() {
    return static_cast<cIGZUnknown*>(
        static_cast<cFC4ISC4BuildingDevelopmentSimulator*>(new cFC4BuildingDevelopmentSimulator()));
}

cFC4BuildingDevelopmentSimulator::cFC4BuildingDevelopmentSimulator() {
    DWORD expected = 0;
    g_mainTid.compare_exchange_strong(expected, GetCurrentThreadId());  // the class store's thread: the main thread
    fc4::DevelopmentLock lock;
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    gridCtx_.tag = "development";
    gridCtx_.writes = fc4::Sw::DevelopmentMapWrites;
    host_ = fc4::MakeDevelopmentHost(&ctx_, &gridCtx_);
    core_ = fc4_development_create(&host_, fc4::DevelopmentProcess(), SelfToken());
    if (core_ == nullptr) fc4::Log("cFC4BuildingDevelopmentSimulator: core creation failed");
    const long budget = fc4::SettingInt("development_growth_attempts_per_day", nullptr, 0);
    if (core_ != nullptr) fc4_development_set_grow_budget(core_, budget > 0 ? static_cast<uint32_t>(budget) : 0);
    RebuildDevelopers();  // an empty map, so the game never sees a null header
    fc4::Log("cFC4BuildingDevelopmentSimulator(%p) created, core %p, daily growth budget %s%ld", static_cast<void*>(this),
             static_cast<void*>(core_), budget > 0 ? "" : "unlimited ", budget > 0 ? budget : 0L);
}

cFC4BuildingDevelopmentSimulator::~cFC4BuildingDevelopmentSimulator() {
    fc4::DevelopmentLock lock;
    if (g_live == this) g_live = nullptr;
    fc4_development_destroy(core_);  // releases what a loaded, never-initialised object still holds
    core_ = nullptr;
    fc4::ReleaseDevelopmentContext(&ctx_);
    if (specials_ != nullptr) {
        delete specials_;  // releases the list's references
        specials_ = nullptr;
    }
    fc4::FreeGameMap<fc4::StlNodeAllocator>(developers_);
    fc4::Log("cFC4BuildingDevelopmentSimulator destroyed (%u calls answered without the lock)", offMainCalls_.load());
}

// §1.1: which sub-object answers is the core's decision; an unknown IID is
// false with *ppv unchanged.
bool cFC4BuildingDevelopmentSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (fc4_development_query_interface(riid)) {
    case 1:
        *ppvObj = static_cast<cFC4ISC4BuildingDevelopmentSimulator*>(this);
        break;
    case 2:
        *ppvObj = static_cast<cIGZMessageTarget*>(this);
        break;
    case 3:
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
        break;
    case 4:
        *ppvObj = static_cast<cIGZSerializable*>(this);
        break;
    default:
        return false;
    }
    ++refCount_;
    return true;
}

uint32_t cFC4BuildingDevelopmentSimulator::AddRef() { return ++refCount_; }

// §1.1: count > 1 decrements; at 1 Shutdown, destroy, 0.
uint32_t cFC4BuildingDevelopmentSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    Shutdown();
    delete this;
    return 0;
}

// ---- publishing -----------------------------------------------------------------------------------

namespace {
void CollectTypeToken(void* ctx, uint32_t key, uint64_t token) {
    static_cast<std::vector<std::pair<uint32_t, uint64_t>>*>(ctx)->emplace_back(key, token);
}
void CollectToken(void* ctx, uint64_t token) { static_cast<std::vector<uint64_t>*>(ctx)->push_back(token); }
}  // namespace

void cFC4BuildingDevelopmentSimulator::Publish() {
    if (core_ == nullptr) return;
    buildingMap_ = fc4_development_building_map(core_);
    freeResources_ = fc4_development_free_resources(core_);
    totalResources_ = fc4_development_total_resources(core_);
    uint32_t bits = 0;
    for (uint32_t i = 0; i < 5; ++i) {
        if (fc4_development_cheat(core_, i)) bits |= 1u << i;
    }
    cheats_ = bits;
    std::vector<std::pair<uint32_t, uint64_t>> devs;
    fc4_development_developers(core_, &CollectTypeToken, &devs);
    if (devs != developerList_) {
        AcquireSRWLockExclusive(&pubLock_);
        developerList_.swap(devs);
        ReleaseSRWLockExclusive(&pubLock_);
        RebuildDevelopers();
    }
    std::vector<uint64_t> specs;
    fc4_development_special_buildings(core_, &CollectToken, &specs);
    if (specs != specialTokens_) {
        specialTokens_.swap(specs);
        RefreshSpecials();
    }
    if (!TheMirror().built) BuildMirror(fc4::DevelopmentProcess());
}

// Slot 34's map: rebuilt from the core's developers (ascending type) with B.
void cFC4BuildingDevelopmentSimulator::RebuildDevelopers() {
    if (!fc4::StlNodeAllocator::Available()) return;
    std::vector<uint32_t> keys, values;
    for (const auto& kv : developerList_) {
        keys.push_back(kv.first);
        values.push_back(static_cast<uint32_t>(reinterpret_cast<uintptr_t>(FromToken(kv.second))));
    }
    fc4::GameMapObject fresh{nullptr, 0, 0};
    if (!fc4::BuildGameMap<fc4::StlNodeAllocator>(fresh, keys.data(), values.data(), keys.size())) {
        fc4::Log("development: WARNING the developer map could not be rebuilt (%u developers)",
                 static_cast<unsigned>(keys.size()));
        return;
    }
    fc4::GameMapObject old = developers_;
    developers_.header = fresh.header;
    developers_.count = fresh.count;
    fc4::FreeGameMap<fc4::StlNodeAllocator>(old);
}

// Slot 44's list (A, one reference each, raw: the list releases raw).
void cFC4BuildingDevelopmentSimulator::RefreshSpecials() {
    cIGZAllocatorServicePtr alloc;
    if (!alloc) return;
    if (specials_ == nullptr) specials_ = new SC4List<cISC4Occupant*>();
    while (specials_->begin() != specials_->end()) specials_->erase(specials_->begin());
    for (uint64_t t : specialTokens_) {
        cISC4Occupant* o = OccOf(t);
        if (o == nullptr) continue;
        o->AddRef();
        specials_->push_back(o);
    }
}

uint64_t cFC4BuildingDevelopmentSimulator::PublishedDeveloper(uint32_t developerType) {
    uint64_t t = 0;
    AcquireSRWLockShared(&pubLock_);
    for (const auto& kv : developerList_) {
        if (kv.first == developerType) {
            t = kv.second;
            break;
        }
    }
    ReleaseSRWLockShared(&pubLock_);
    return t;
}

// ---- status lines ---------------------------------------------------------------------------------

void cFC4BuildingDevelopmentSimulator::LogState(const char* what) {
    fc4_development_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_development_get_state(core_, &st)) return;
    char budget[16];
    if (st.grow_budget == 0xFFFFFFFFu) {
        std::snprintf(budget, sizeof(budget), "unlimited");
    } else {
        std::snprintf(budget, sizeof(budget), "%u", st.grow_budget);
    }
    fc4::Log("development %s: init=%u loaded=%u state=%u cheats=0x%X developers=%u specials=%u stages=%u plan=%u "
             "marks=%u/%u types=%u budget=%s resources=%d/%d supply=%.1f demand=%.1f grows=%u built=%u declines=%u "
             "declined=%u reentrant=%u city=%dx%d rng=%llu map=%p node=%p offMain=%u occupants=%u loadsFailed=%u "
             "inserts=%u removals=%u advisor=%u demolitions=%u dezones=%u lotUpdates=%u",
             what, st.initialized, st.loaded, st.state, st.cheats, st.developers, st.special_buildings,
             st.highest_stages, st.decline_plan, st.marked_types, st.marked_cells, st.building_types, budget,
             st.free_resources, st.total_resources, st.supply, st.demand, st.grow_calls, st.built, st.decline_calls,
             st.declined, st.reentrant_calls, st.cell_count_x, st.cell_count_z,
             static_cast<unsigned long long>(st.rng_steps), static_cast<void*>(FromToken(st.building_map)),
             static_cast<void*>(FromToken(st.construction_node)), offMainCalls_.load(), ctx_.occupantsCreated,
             ctx_.loadsFailed, ctx_.inserts, ctx_.removals, ctx_.advisorEvents, ctx_.demolitions, ctx_.dezones,
             ctx_.lotElementUpdates);
}

// The status lines' date: the simulator's, else (no city) the last SimNewDay's data1.
uint32_t cFC4BuildingDevelopmentSimulator::StatusDay() {
    const uint32_t d = SimDay(&ctx_);
    return d != 0 ? d : lastNewDay_;
}

// One line per SimNewMonth: the month's growth and decline (differences from
// the previous line) and the state.
void cFC4BuildingDevelopmentSimulator::LogMonth() {
    fc4_development_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_development_get_state(core_, &st)) return;
    fc4_development_state p{};
    if (haveLastMonth_) p = lastMonth_;
    std::string stages;
    for (int i = 0; i < 16; ++i) {
        const uint32_t d = st.built_by_stage[i] - p.built_by_stage[i];
        if (d == 0) continue;
        char b[24];
        std::snprintf(b, sizeof(b), " %d:%u", i, d);
        stages += b;
    }
    if (stages.empty()) stages = " -";
    const float mult = fc4_development_stage_population_multiplier(core_);
    fc4::Log("development month %s: developers=%u built=%u (stages%s) (wealth %u/%u/%u/%u) declined=%u (wealth "
             "%u/%u/%u/%u) grows=%u declines=%u specials=%u city=%dx%d stage=%.2f resources=%d/%d supply=%.0f "
             "demand=%.0f state=%u plan=%u marks=%u/%u types=%u rng=%llu reentrant=%u offMain=%u occupants=%u "
             "loadsFailed=%u advisor=%u demolitions=%u dezones=%u lotUpdates=%u",
             DateText(StatusDay()).c_str(), st.developers, st.built - p.built, stages.c_str(),
             st.built_by_wealth[0] - p.built_by_wealth[0], st.built_by_wealth[1] - p.built_by_wealth[1],
             st.built_by_wealth[2] - p.built_by_wealth[2], st.built_by_wealth[3] - p.built_by_wealth[3],
             st.declined - p.declined, st.declined_by_wealth[0] - p.declined_by_wealth[0],
             st.declined_by_wealth[1] - p.declined_by_wealth[1], st.declined_by_wealth[2] - p.declined_by_wealth[2],
             st.declined_by_wealth[3] - p.declined_by_wealth[3], st.grow_calls - p.grow_calls,
             st.decline_calls - p.decline_calls, st.special_buildings, st.cell_count_x, st.cell_count_z, mult,
             st.free_resources, st.total_resources, st.supply, st.demand, st.state, st.decline_plan, st.marked_types,
             st.marked_cells, st.building_types, static_cast<unsigned long long>(st.rng_steps), st.reentrant_calls,
             offMainCalls_.load(), ctx_.occupantsCreated, ctx_.loadsFailed, ctx_.advisorEvents, ctx_.demolitions,
             ctx_.dezones, ctx_.lotElementUpdates);
    lastMonth_ = st;
    haveLastMonth_ = true;
}

namespace {
struct PlanText {
    std::string text;
    uint32_t n = 0;
};
void CollectPlan(void* ctx, const fc4_plan_entry* e) {
    auto* p = static_cast<PlanText*>(ctx);
    if (e == nullptr) return;
    if (p->n < 12) {
        char b[96];
        cFC4ISC4GrowthDeveloper* d = DevOf(e->developer);
        std::snprintf(b, sizeof(b), " [%u %d,%d %u 0x%X]", e->day, e->x, e->z, e->stage, d ? d->GetSubType() : 0u);
        p->text += b;
    } else if (p->n == 12) {
        p->text += " ...";
    }
    ++p->n;
}
}  // namespace

// One line per completed PlanDecline (the planning state 4 -> 0, §4.2).
void cFC4BuildingDevelopmentSimulator::LogDeclinePlan() {
    PlanText p;
    fc4_development_decline_plan(core_, &CollectPlan, &p);
    ++plans_;
    fc4::Log("development decline plan %s: %u entries%s", DateText(StatusDay()).c_str(), p.n,
             p.text.empty() ? "" : p.text.c_str());
}

// ---- lifecycle (§1.3, §1.4) -----------------------------------------------------------------------

bool cFC4BuildingDevelopmentSimulator::Init() {
    FC4_TRACE("BuildingDevelopmentSimulator::Init");
    g_mainTid = GetCurrentThreadId();
    Call call(this);
    if (!call.ok()) return true;
    fc4_development_init(core_);  // §1.3, all of it (the core drives every host call)
    g_live = this;
    LogState("Init");
    return true;
}

bool cFC4BuildingDevelopmentSimulator::Shutdown() {
    FC4_TRACE("BuildingDevelopmentSimulator::Shutdown");
    // Keep this object alive while the simulator and the message server drop
    // their references (unless this already is the final Release).
    if (!destroying_) AddRef();
    {
        Call call(this);
        if (call.ok()) {
            fc4_development_shutdown(core_);  // §1.4
            LogState("Shutdown");
        }
        if (g_live == this) g_live = nullptr;
    }
    if (!destroying_) Release();
    return true;
}

// ---- primary slots (§7.1) -------------------------------------------------------------------------

bool cFC4BuildingDevelopmentSimulator::LocationIsOKForNewBuilding(uint32_t type, float x, float z) {
    Call call(this);
    return call.ok() && fc4_development_location_is_ok(core_, type, x, z);
}

namespace {
// The building occupant interface of an occupant token the occupant manager
// holds (borrowed: the QueryInterface's reference is dropped).
cISC4BuildingOccupant* AsBuilding(uint64_t occ) {
    if (occ == 0) return nullptr;
    cISC4BuildingOccupant* b = Query<cISC4BuildingOccupant>(FromToken(occ), kIID_cISC4BuildingOccupant);
    Drop(b);
    return b;
}
uint64_t OccupantToken(cISC4BuildingOccupant* b) { return b ? TokenOf(b->AsOccupant()) : 0; }
uint64_t OccupantToken(cISC4FoundationOccupant* f) { return f ? TokenOf(f->AsOccupant()) : 0; }
}  // namespace

// Slots 6 / 7 / 47: the occupant or null (§7.1: a pointer, not bool).
cISC4BuildingOccupant* cFC4BuildingDevelopmentSimulator::AddNewBuildingOccupantAtCell(uint32_t type, int32_t cellX,
                                                                                      int32_t cellZ,
                                                                                      int32_t orientation) {
    Call call(this);
    if (!call.ok()) return nullptr;
    return AsBuilding(fc4_development_add_building_at_cell(core_, type, cellX, cellZ, orientation));
}

cISC4BuildingOccupant* cFC4BuildingDevelopmentSimulator::AddNewBuildingOccupantAt(uint32_t type, float* position,
                                                                                  int32_t orientation) {
    Call call(this);
    if (!call.ok() || position == nullptr) return nullptr;
    return AsBuilding(fc4_development_add_building(core_, type, position, orientation));
}

bool cFC4BuildingDevelopmentSimulator::AddNewBuildingOccupant(cISC4BuildingOccupant* occupant) {
    Call call(this);
    return call.ok() && fc4_development_insert_occupant(core_, OccupantToken(occupant));
}

// Slot 9: the foundation overload returns 0 (§4.9).
int64_t cFC4BuildingDevelopmentSimulator::GetCostOfFoundationDemolition(cISC4FoundationOccupant*, uint32_t) {
    return 0;
}

int64_t cFC4BuildingDevelopmentSimulator::GetCostOfOccupantDemolition(cISC4BuildingOccupant* occupant, uint32_t) {
    Call call(this);
    return call.ok() ? fc4_development_demolition_cost(core_, OccupantToken(occupant)) : 0;
}

// Slots 11-22, 27, 28: the catalogue mirror (§4.7.2).
uint32_t cFC4BuildingDevelopmentSimulator::GetAllBuildingTypes(fc4::GameVector<uint32_t>& types) {
    return FillVector(types, MirrorTypes(0));
}

uint32_t cFC4BuildingDevelopmentSimulator::GetAllBuildingTypesArray(uint32_t* types, uint32_t& count) {
    const std::vector<uint32_t> all = MirrorTypes(0);
    count = std::min<uint32_t>(count, static_cast<uint32_t>(all.size()));
    if (types != nullptr) {
        for (uint32_t i = 0; i < count; ++i) types[i] = all[i];
    }
    return static_cast<uint32_t>(all.size());
}

uint32_t cFC4BuildingDevelopmentSimulator::GetAllBuildingFamilyTypes(fc4::GameVector<uint32_t>& families) {
    return FillVector(families, MirrorTypes(1));
}

bool cFC4BuildingDevelopmentSimulator::GetBuildingKeyFromType(uint32_t type, cGZPersistResourceKey& key) {
    return MirrorKey(0, type, key);
}

bool cFC4BuildingDevelopmentSimulator::IsBuildingTypeValid(uint32_t type) { return MirrorIsType(type); }

bool cFC4BuildingDevelopmentSimulator::IsBuildingFamilyTypeValid(uint32_t family) {
    return MirrorFamily(family, nullptr) || MirrorIsType(family);
}

uint32_t cFC4BuildingDevelopmentSimulator::GetBuildingTypesByFilter(fc4::GameHashSetU32<>& types,
                                                                     cISC4BuildingTypeFilter* filter) {
    auto* f = reinterpret_cast<FC4BuildingTypeFilter*>(filter);
    uint32_t passed = 0;
    for (uint32_t t : MirrorTypes(0)) {
        if (f != nullptr && !f->IsBuildingTypeIncluded(t)) continue;
        types.insert(t);
        ++passed;
    }
    return passed;
}

uint32_t cFC4BuildingDevelopmentSimulator::GetBuildingTypesByFamily(fc4::GameVector<uint32_t>& types,
                                                                     uint32_t family) {
    std::vector<uint32_t> members;
    if (MirrorFamily(family, &members)) return FillVector(types, members);
    if (MirrorIsType(family)) {
        types.clear();
        types.push_back(family);
        return 0;  // §4.7.2 [quirk]: 0 for a single type
    }
    return 0;
}

uint32_t cFC4BuildingDevelopmentSimulator::GetBuildingTypeSetByFamily(fc4::GameHashSetU32<>& types, uint32_t family) {
    std::vector<uint32_t> members;
    if (MirrorFamily(family, &members)) {
        for (uint32_t t : members) types.insert(t);
        return static_cast<uint32_t>(members.size());
    }
    if (MirrorIsType(family)) types.insert(family);
    return 0;
}

bool cFC4BuildingDevelopmentSimulator::IsBuildingInFamily(uint32_t type, uint32_t family) {
    if (type == family) return true;
    std::vector<uint32_t> members;
    return MirrorFamily(family, &members) && std::find(members.begin(), members.end(), type) != members.end();
}

uint32_t cFC4BuildingDevelopmentSimulator::GetAllFoundationTypes(fc4::GameVector<uint32_t>& types) {
    return FillVector(types, MirrorTypes(2));
}

bool cFC4BuildingDevelopmentSimulator::GetFoundationKeyFromType(uint32_t type, cGZPersistResourceKey& key) {
    return MirrorKey(2, type, key);
}

// Slot 23: foundation placement (not used by growth). [spec question,
// choice] Its behaviour is not specified: false (logged once).
bool cFC4BuildingDevelopmentSimulator::AddNewFoundationOccupantAt(uint32_t, int32_t, float*, float, float, int32_t,
                                                                  float) {
    if (fc4::OnDevelopmentMainThread()) {
        NoteOnce(g_noteFoundationAt, "development: AddNewFoundationOccupant (slot 23, seven arguments) is not "
                                     "specified; it returns false");
    }
    return false;
}

bool cFC4BuildingDevelopmentSimulator::AddNewFoundationOccupant(cISC4FoundationOccupant* occupant) {
    Call call(this);
    return call.ok() && fc4_development_insert_occupant(core_, OccupantToken(occupant));
}

bool cFC4BuildingDevelopmentSimulator::DemolishFoundationOccupant(cISC4FoundationOccupant* occupant, uint32_t) {
    Call call(this);
    return call.ok() && fc4_development_demolish_foundation(core_, OccupantToken(occupant));
}

// Slots 26 / 29: [spec question, choice] not specified: false (logged once).
bool cFC4BuildingDevelopmentSimulator::LoadFoundationProperties(cISC4FoundationOccupant*, uint32_t) {
    if (fc4::OnDevelopmentMainThread()) {
        NoteOnce(g_noteFoundationProps, "development: LoadFoundationProperties (slot 26) is not specified; it returns "
                                        "false");
    }
    return false;
}

uint32_t cFC4BuildingDevelopmentSimulator::GetAllLotRetainingWallTypes(fc4::GameVector<uint32_t>& types) {
    return FillVector(types, MirrorTypes(3));
}

bool cFC4BuildingDevelopmentSimulator::GetLotRetainingWallKeyFromType(uint32_t type, cGZPersistResourceKey& key) {
    return MirrorKey(3, type, key);
}

bool cFC4BuildingDevelopmentSimulator::LoadLotRetainingWallProperties(cISC4LotRetainingWallOccupant*, uint32_t) {
    if (fc4::OnDevelopmentMainThread()) {
        NoteOnce(g_noteWallProps, "development: LoadLotRetainingWallProperties (slot 29) is not specified; it returns "
                                  "false");
    }
    return false;
}

uint32_t cFC4BuildingDevelopmentSimulator::GetDeveloperType(uint32_t purpose, uint32_t wealth) {
    return fc4_development_developer_type(purpose, wealth);
}

uint32_t cFC4BuildingDevelopmentSimulator::GetWealth(uint32_t developerType) {
    return fc4_development_wealth_of(developerType);
}

// Slot 32: borrowed, null if absent. Another thread reads the published map.
cISC4GrowthDeveloper* cFC4BuildingDevelopmentSimulator::GetDeveloper(uint32_t developerType) const {
    auto* self = const_cast<cFC4BuildingDevelopmentSimulator*>(this);
    uint64_t t = 0;
    {
        Call call(this);
        t = call.ok() ? fc4_development_developer(core_, developerType) : self->PublishedDeveloper(developerType);
    }
    return reinterpret_cast<cISC4GrowthDeveloper*>(FromToken(t));
}

// Slot 33: slots 30 and 32, both through the vtable (§7.1).
cISC4GrowthDeveloper* cFC4BuildingDevelopmentSimulator::GetDeveloperOf(uint32_t purpose, uint32_t wealth) const {
    auto* self = const_cast<cFC4BuildingDevelopmentSimulator*>(this);
    cFC4ISC4BuildingDevelopmentSimulator* v = self;
    return v->GetDeveloper(v->GetDeveloperType(purpose, wealth));
}

fc4::GameMapObject* cFC4BuildingDevelopmentSimulator::GetDevelopers() { return &developers_; }

// Slots 35 / 36: the city's (§7.1: cached, no AddRef).
cISC4TractDeveloper* cFC4BuildingDevelopmentSimulator::GetTractDeveloper() {
    cISC4City* c = ctx_.city ? ctx_.city : AppCity();
    return c ? c->GetTractDeveloper() : nullptr;
}

cISC4LotDeveloper* cFC4BuildingDevelopmentSimulator::GetLotDeveloper() {
    cISC4City* c = ctx_.city ? ctx_.city : AppCity();
    return c ? c->GetLotDeveloper() : nullptr;
}

cIGZString* cFC4BuildingDevelopmentSimulator::GetExemplarName(uint32_t type) { return MirrorName(0, type); }

cIGZString* cFC4BuildingDevelopmentSimulator::GetBuildingName(uint32_t type) { return MirrorName(1, type); }

FC4BuildingProfile& cFC4BuildingDevelopmentSimulator::GetBuildingProfile(uint32_t type) const {
    return *MirrorProfile(type, true);
}

FC4BuildingProfile* cFC4BuildingDevelopmentSimulator::GetBuildingProfilePtr(uint32_t type) const {
    return MirrorProfile(type, false);
}

cISC4SimGrid<uint8_t>* cFC4BuildingDevelopmentSimulator::GetBuildingMap() {
    uint64_t t = buildingMap_.load();
    {
        Call call(this);
        if (call.ok()) t = fc4_development_building_map(core_);
    }
    return reinterpret_cast<cISC4SimGrid<uint8_t>*>(FromToken(t));
}

// Slots 42 / 43 return a uint8 in AL; a replacement gives it zero-extended in
// EAX (§7.1). The asm block leaves it in EAX and the function has no return
// statement (MSVC: warning 4035 silenced); nothing with a destructor is alive
// at that point.
namespace {
uint32_t KeyOfType(uint32_t type) {
    const FC4BuildingProfile* p = MirrorProfile(type, true);  // §4.7.3: an unknown type gets a zero profile, key 1
    return fc4_development_building_map_key(p->purpose, p->wealth) & 0xFFu;
}
}  // namespace

#pragma warning(push)
#pragma warning(disable : 4035)
uint8_t cFC4BuildingDevelopmentSimulator::GetBuildingMapKeyOf(uint32_t purpose, uint32_t wealth) {
    const uint32_t v = fc4_development_building_map_key(purpose, wealth) & 0xFFu;
    __asm mov eax, v
}

uint8_t cFC4BuildingDevelopmentSimulator::GetBuildingMapKey(uint32_t type) {
    const uint32_t v = KeyOfType(type);
    __asm mov eax, v
}
#pragma warning(pop)

SC4List<cISC4Occupant*>* cFC4BuildingDevelopmentSimulator::GetSpecialBuildings() {
    if (specials_ == nullptr && fc4::OnDevelopmentMainThread()) {
        cIGZAllocatorServicePtr alloc;
        if (alloc) specials_ = new SC4List<cISC4Occupant*>();
    }
    return specials_;
}

// Slot 45: the mirror's table (§4.7.4), ST0.
float cFC4BuildingDevelopmentSimulator::GetMaxCapacity(int32_t zone, uint32_t purpose, uint32_t wealth) {
    const uint32_t key = fc4_development_building_map_key(purpose, wealth) & 0xFFu;
    MirrorLock lock;
    Mirror& m = TheMirror();
    auto it = m.maxCapacity.find((static_cast<uint32_t>(zone) << 8) | key);
    return it != m.maxCapacity.end() ? it->second : 0.0f;
}

bool cFC4BuildingDevelopmentSimulator::IsSuitableForConstruction(FC4ConstructionProfile const& profile,
                                                                  int32_t& reason) {
    Call call(this);
    if (!call.ok()) return false;
    fc4_construction_profile p{};
    static_assert(sizeof(p) == sizeof(profile), "ConstructionProfile layouts");
    std::memcpy(&p, &profile, sizeof(p));
    return fc4_development_is_suitable(core_, &p, &reason);
}

cISC4BuildingOccupant* cFC4BuildingDevelopmentSimulator::PlopBuilding(FC4ConstructionProfile const& profile) {
    Call call(this);
    if (!call.ok()) return nullptr;
    fc4_construction_profile p{};
    std::memcpy(&p, &profile, sizeof(p));
    return AsBuilding(fc4_development_plop_building(core_, &p));
}

int32_t cFC4BuildingDevelopmentSimulator::GetFreeConstructionResources() {
    Call call(this);
    return call.ok() ? fc4_development_free_resources(core_) : freeResources_.load();
}

int32_t cFC4BuildingDevelopmentSimulator::GetTotalConstructionResources() {
    Call call(this);
    return call.ok() ? fc4_development_total_resources(core_) : totalResources_.load();
}

bool cFC4BuildingDevelopmentSimulator::ClaimConstructionResources(uint16_t amount) {
    Call call(this);
    return call.ok() && fc4_development_claim(core_, amount);
}

bool cFC4BuildingDevelopmentSimulator::ReleaseConstructionResources(uint16_t amount) {
    Call call(this);
    return call.ok() && fc4_development_release_resources(core_, amount);
}

int32_t cFC4BuildingDevelopmentSimulator::GetDayOfLastSuccessfulBuild(uint32_t developerType) {
    Call call(this);
    return call.ok() ? fc4_development_last_successful_build(core_, developerType) : 0;
}

float cFC4BuildingDevelopmentSimulator::GetStagePopulationMultiplier() {
    Call call(this);
    if (!call.ok()) return stageMultiplier_.load();
    const float v = fc4_development_stage_population_multiplier(core_);
    stageMultiplier_ = v;
    return v;
}

namespace {
bool CheatBit(const std::atomic<uint32_t>& bits, uint32_t which) { return ((bits.load() >> which) & 1u) != 0; }
}  // namespace

bool cFC4BuildingDevelopmentSimulator::CheatIgnorePlumbing() {
    Call call(this);
    return call.ok() ? fc4_development_cheat(core_, 0) : CheatBit(cheats_, 0);
}

bool cFC4BuildingDevelopmentSimulator::CheatIgnorePower() {
    Call call(this);
    return call.ok() ? fc4_development_cheat(core_, 1) : CheatBit(cheats_, 1);
}

bool cFC4BuildingDevelopmentSimulator::CheatIgnoreCaps() {
    Call call(this);
    return call.ok() ? fc4_development_cheat(core_, 2) : CheatBit(cheats_, 2);
}

bool cFC4BuildingDevelopmentSimulator::CheatDebugGrowth() {
    Call call(this);
    return call.ok() ? fc4_development_cheat(core_, 3) : CheatBit(cheats_, 3);
}

bool cFC4BuildingDevelopmentSimulator::CheatCoffeeBar() {
    Call call(this);
    return call.ok() ? fc4_development_cheat(core_, 4) : CheatBit(cheats_, 4);
}

// ---- messages (§3) --------------------------------------------------------------------------------

// §7.3: true, nothing done.
bool cFC4BuildingDevelopmentSimulator::DoMessage(cGZMessage&) {
    FC4_X87_SCOPE("development DoMessage (cIGZMessageTarget)", 0u);  // x87 leftover guard (FC4Fpu.h)
    return true;
}

namespace {
// §4.12 PlaceBuilding: parameter 0 as uint32 (variant slot 24), 1 and 2 as
// float (slot 36), from the command's parameter set (void1, slot 5).
size_t ReadPlaceBuildingParams(cIGZCommandParameterSet* ps, fc4_command_param* out) {
    if (ps == nullptr) return 0;
    // Slot 5 never returns null (docs/abi: an out-of-range index gives an
    // empty variant), so the count (slot 3) bounds the reads.
    const uint32_t count = std::min<uint32_t>(ps->GetParameterCount(), 3);
    size_t n = 0;
    for (uint32_t i = 0; i < count; ++i) {
        cIGZVariant* v = ps->operator[](i);  // slot 5
        if (v == nullptr) break;
        fc4_command_param p{};
        if (i == 0) {
            p.as_u32 = fc4::AsGame(v)->GetValUint32();
        } else {
            p.as_f32 = fc4::AsGame(v)->GetValFloat32();
        }
        out[n++] = p;
    }
    return n;
}
}  // namespace

bool cFC4BuildingDevelopmentSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("development DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    Call call(this);
    if (!call.ok()) return true;  // [choice] a message from another thread that cannot get the lock is dropped
    fc4_development_message m{};
    m.msg_type = pMessage->GetType();
    fc4_command_param params[3] = {};
    size_t nParams = 0;
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    if (s != nullptr) {
        m.data1 = static_cast<uint32_t>(s->GetData1());
        m.data2 = static_cast<uint32_t>(s->GetData2());
        m.data3 = static_cast<uint32_t>(s->GetData3());
        m.data4 = static_cast<uint32_t>(s->GetData4());
        if (m.msg_type == kMsgOccupantInserted || m.msg_type == kMsgOccupantRemoved) {
            // §3.2: the occupant is void1. Its reference is kept until the
            // outermost call returns (a queued message runs then).
            cISC4Occupant* o = Query<cISC4Occupant>(static_cast<cIGZUnknown*>(s->GetVoid1()), kIID_cISC4Occupant);
            if (o != nullptr) ctx_.held.push_back(o);
            m.occupant = TokenOf(o);
        } else if (m.msg_type == kMsgStateChanged) {
            // §3.2: the lot is GetIGZUnknown() (kept as the occupant).
            cISC4Lot* l = Query<cISC4Lot>(s->GetIGZUnknown(), kIID_cISC4Lot);
            if (l != nullptr) ctx_.held.push_back(l);
            m.lot = TokenOf(l);
        } else if (m.msg_type == kMsgPlaceBuilding) {
            nParams = ReadPlaceBuildingParams(static_cast<cIGZCommandParameterSet*>(s->GetVoid1()), params);
        }
    }
    Drop(s);
    fc4_development_do_message(core_, &m, nParams != 0 ? params : nullptr, nParams);
    if (m.msg_type == kMsgSimNewDay) {
        ++days_;
        lastNewDay_ = m.data1;
    }
    if (depth_ == 1) {
        if (m.msg_type == kMsgSimNewMonth) {
            ++months_;
            LogMonth();
        }
        fc4_development_state st{};
        st.struct_size = sizeof(st);
        if (fc4_development_get_state(core_, &st)) {
            if (lastPlanState_ == 4 && st.state == 0) LogDeclinePlan();
            lastPlanState_ = st.state;
        }
    }
    return true;  // §3.1: always
}

// ---- persistence (§6) -----------------------------------------------------------------------------

bool cFC4BuildingDevelopmentSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("BuildingDevelopmentSimulator::Write");
    Call call(this);
    if (!call.ok()) return false;
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_development_write(core_, a.Get());
    fc4::Log("development Write -> %d", ok ? 1 : 0);
    return ok;
}

// §6: runs before Init; nothing is sized from the city here.
bool cFC4BuildingDevelopmentSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("BuildingDevelopmentSimulator::Read");
    bool ok = false;
    {
        Call call(this);
        if (!call.ok()) return false;
        fc4::IStreamAdapter a(s);
        ok = fc4_development_read(core_, a.Get());
        fc4::Log("development Read -> %d", ok ? 1 : 0);
        LogState("Read");
    }
    return ok;
}

uint32_t cFC4BuildingDevelopmentSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4BuildingDevelopmentSimulator; }
