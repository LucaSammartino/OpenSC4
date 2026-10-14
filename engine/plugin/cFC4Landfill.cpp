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

#include "cFC4Landfill.h"

#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "SC4CellRegion.h"
#include "SC4List.h"
#include "SC4Point.h"
#include "SC4Rect.h"
#include "SC4Vector.h"
#include "cIGZApp.h"
#include "cIGZCOM.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4City.h"
#include "cISC4LandfillOccupant.h"
#include "cISC4LineItem.h"
#include "cISC4LotBaseTextureOccupant.h"
#include "cISC4LotDeveloper.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4PollutionSimulator.h"
#include "cISC4PropManager.h"
#include "cISC4Simulator.h"
#include "cISC4TrafficSimulator.h"
#include "cISC4ZoneManager.h"
#include "cISCExemplarPropertyHolder.h"
#include "cISCPropertyHolder.h"
#include "cISCResExemplar.h"
#include "cISTETerrain.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"

#include <cstddef>
#include <cstring>
#include <memory>
#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
/// cISC4LandfillOccupant (§4.5.2, §7.3).
constexpr uint32_t kIID_cISC4LandfillOccupant = 0x2A15F262;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cIGZSerializable = 0xE4FDA3D4;

// Messages (§3.2, §5).
constexpr uint32_t kMsgZoningChange = 0x07699455;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgHistoryRecord = 0x89EFA536;

/// GetSimulatorType (slot 6, §1).
constexpr uint32_t kSimulatorType = 0x2A398ED6;

// The tuning exemplar "LandfillDeveloper" (§4.1).
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0x67CD5FA1;
constexpr uint32_t kTuningInstance = 0x6A2806AF;

// Occupant types (§4.4.5): flora (RemoveIncompatibleOccupants), base texture.
constexpr uint32_t kOccupantTypeFlora = 0x74758926;
constexpr uint32_t kOccupantTypeBaseTexture = 0x497F6D9D;
/// RemoveOccupant's `msgData` of §4.8.3 `(occupant, 1, 3)`.
constexpr uint32_t kRemoveMsgData = 3;

// Landfill tiles (§4.8.1 step 3, LG3): the class, its exemplar holder and
// the tile exemplars' type and group (the instance is the tile ID).
constexpr uint32_t kCLSID_cSC4LandfillOccupant = 0x8A22BEBE;
constexpr uint32_t kIID_cISCExemplarPropertyHolder = 0x0AC2B5F7;
constexpr uint32_t kTileExemplarType = 0x6534284A;
constexpr uint32_t kTileExemplarGroup = 0x07BDDF1C;
constexpr uint32_t kTileFlag = 4;          // SetFlag(4)
constexpr uint32_t kTileRemovalFlags = 2;  // SetRemovalFlags(2)

// The landfill ground texture (§4.8.5, LG2): class, interface, usage 1,
// texture instance 0x1005, the shipped zone display alpha.
constexpr uint32_t kCLSID_cSC4LotBaseTextureOccupant = 0xC97F987C;
constexpr uint32_t kIID_cISC4LotBaseTextureOccupant = 0x097F6D98;
constexpr int32_t kLandfillTextureUsage = 1;
constexpr uint32_t kLandfillTexture = 0x1005;
constexpr uint8_t kShippedZoneDisplayAlpha = 0xAA;
/// Occupant flag 1: the occupant is in the occupant manager (§4.8.4).
constexpr uint32_t kOccupantFlagInserted = 1;
/// PostOccupantMessage after an inserted occupant changed (§4.8.5 step 6).
constexpr uint32_t kMsgOccupantChanged = 0x99EF1145;

// fc4.h: a region of more cells is refused by the core.
constexpr int64_t kMaxRegionCells = int64_t{1} << 20;

fc4::LandfillContext* Ctx(void* u) { return static_cast<fc4::LandfillContext*>(u); }
cISC4City* City(void* u) { return Ctx(u)->city; }

cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cISCPropertyHolder* HolderOf(uint64_t occ) { return occ ? OccOf(occ)->AsPropertyHolder() : nullptr; }  // slot 6
cFC4ISC4DepartmentBudget* DeptOf(uint64_t t) {
    return t ? fc4::AsGame(reinterpret_cast<cISC4DepartmentBudget*>(FromToken(t))) : nullptr;
}

// The landfill_* switches; the shadow's context reads the game as it is.
bool Gate(void* u, fc4::Sw s, const char* what) {
    if (!Ctx(u)->gated || fc4::On(s)) return true;
    fc4::NoteSkipped(s, what);
    return false;
}

// Main thread only (every host callback runs under the developer's lock or
// in the shadow's delivery).
void NoteOnce(bool& done, const char* text) {
    if (done) return;
    done = true;
    fc4::Log("%s", text);
}
bool g_noteTexture = false;
bool g_noteTile = false;
bool g_noteTileInsert = false;
bool g_noteEffects = false;
bool g_noteZot = false;
bool g_noteView = false;
bool g_noteZoneFromCell = false;
bool g_noteRegion = false;
bool g_noteVector = false;

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

float FromBits(uint32_t b) {
    float f = 0.0f;
    std::memcpy(&f, &b, sizeof(f));
    return f;
}
uint32_t ToBits(float f) {
    uint32_t b = 0;
    std::memcpy(&b, &f, sizeof(b));
    return b;
}

// ---- cell regions ---------------------------------------------------------------------------------
//
// The game's SC4CellRegion<long> (docs/specs/zones.md, "SC4CellRegion<long>",
// 0x24 bytes; the SDK's SC4CellRegion.h agrees): int32 x1, z1, x2, z2
// (inclusive), then a cRZCellMap<bool>: vtable, width (x count), height
// (z count), words per row (ceil(height / 32)), uint32** rows. Bit (dx, dz)
// is bit dz % 32 of rows[dx][dz / 32].

struct GameCellMapView {
    const void* vtbl;
    uint32_t width;
    uint32_t height;
    uint32_t words;
    uint32_t* const* rows;
};
struct GameCellRegionView {
    int32_t x1, z1, x2, z2;
    GameCellMapView map;
};
static_assert(sizeof(void*) != 4 || sizeof(GameCellRegionView) == 0x24, "SC4CellRegion<long> layout");

// A game region (ZoningChange's GetVoid2, GetZoningCost's argument) as the
// core's fc4_landfill_region over `mask`. False (logged once) when the
// rectangle and the cell map disagree: the layout is not what zones.md says,
// so nothing is read.
bool ReadGameRegion(const void* p, std::vector<uint8_t>& mask, fc4_landfill_region* out) {
    if (p == nullptr || out == nullptr) return false;
    const GameCellRegionView* g = static_cast<const GameCellRegionView*>(p);
    const int64_t w = int64_t{g->x2} - g->x1 + 1;
    const int64_t h = int64_t{g->z2} - g->z1 + 1;
    const bool shape = w > 0 && h > 0 && w * h <= kMaxRegionCells;
    if (!shape || g->map.width != static_cast<uint64_t>(w) || g->map.height != static_cast<uint64_t>(h) ||
        g->map.words < static_cast<uint32_t>((h + 31) / 32) || g->map.rows == nullptr) {
        if (!g_noteRegion) {
            g_noteRegion = true;
            fc4::Log("landfill: WARNING a cell region does not have the documented layout (rect %d,%d..%d,%d, map "
                     "%ux%u, %u words per row); it is ignored",
                     g->x1, g->z1, g->x2, g->z2, g->map.width, g->map.height, g->map.words);
        }
        return false;
    }
    mask.assign(static_cast<size_t>(w * h), 0);
    for (int64_t dx = 0; dx < w; ++dx) {
        const uint32_t* row = g->map.rows[dx];
        if (row == nullptr) continue;
        for (int64_t dz = 0; dz < h; ++dz) {
            if ((row[dz / 32] >> (dz % 32)) & 1u) mask[static_cast<size_t>(dz * w + dx)] = 1;
        }
    }
    *out = fc4_landfill_region{};
    out->x0 = g->x1;
    out->z0 = g->z1;
    out->x1 = g->x2;
    out->z1 = g->z2;
    out->mask = mask.data();
    out->len = mask.size();
    return true;
}

// The cRZCellMap<bool> layout with a vtable of our own (slot 0, the
// destructor, is the only virtual; nothing in the calls below destroys it).
class CellMapStorage {
public:
    virtual ~CellMapStorage() = default;
    uint32_t width = 0;
    uint32_t height = 0;
    uint32_t words = 0;
    uint32_t** rows = nullptr;
};

// A region built from the core's fc4_landfill_region in the game's layout,
// for PlaceZone (the zone manager clears the bits of rejected cells; it never
// reallocates or frees the region: zones.md) and GetSubnetworksInRegion
// (const). The memory is ours.
class RegionBuffer {
public:
    bool Build(const fc4_landfill_region* in) {
        if (in == nullptr) return false;
        const int64_t w = int64_t{in->x1} - in->x0 + 1;
        const int64_t h = int64_t{in->z1} - in->z0 + 1;
        if (w <= 0 || h <= 0 || w * h > kMaxRegionCells) return false;
        layout_.x1 = in->x0;
        layout_.z1 = in->z0;
        layout_.x2 = in->x1;
        layout_.z2 = in->z1;
        const uint32_t words = static_cast<uint32_t>((h + 31) / 32);
        bits_.assign(static_cast<size_t>(w) * words, 0);
        rows_.resize(static_cast<size_t>(w));
        for (int64_t dx = 0; dx < w; ++dx) rows_[static_cast<size_t>(dx)] = &bits_[static_cast<size_t>(dx) * words];
        for (int64_t dz = 0; dz < h; ++dz) {
            for (int64_t dx = 0; dx < w; ++dx) {
                const size_t i = static_cast<size_t>(dz * w + dx);
                const bool set = in->mask == nullptr || (i < in->len && in->mask[i] != 0);
                if (set) rows_[static_cast<size_t>(dx)][dz / 32] |= 1u << (dz % 32);
            }
        }
        layout_.map.width = static_cast<uint32_t>(w);
        layout_.map.height = static_cast<uint32_t>(h);
        layout_.map.words = words;
        layout_.map.rows = rows_.data();
        return true;
    }
    SC4CellRegion<int32_t>& Zoning() { return *reinterpret_cast<SC4CellRegion<int32_t>*>(&layout_); }
    SC4CellRegion<long> const& Traffic() const { return *reinterpret_cast<const SC4CellRegion<long>*>(&layout_); }

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

// ---- lifecycle (§1.1, §1.2) -------------------------------------------------------------------------

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

// Holds `p` in `field` with AddRef, only if the field is still null (LG15).
template <class T>
void HoldIfNull(T*& field, T* p) {
    if (field != nullptr || p == nullptr) return;
    field = p;
    FC4_ADDREF(field);
}

// §1.1 step 3 (LG15 [compat]): the city, the terrain, the pollution and budget
// simulators and the message server, each held with AddRef only if its field
// is still null. The original reads the global service pointers the city
// fills; the getters of the app's current city return the same objects. The
// original crashes on a missing service;
// we tolerate one (it is then fetched from the city when used).
void AcquireServices(void* u) {
    fc4::LandfillContext* c = Ctx(u);
    HoldIfNull(c->city, AppCity());
    if (c->city == nullptr) {
        fc4::Log("landfill: no city; services not acquired");
    } else {
        HoldIfNull(c->terrain, c->city->GetTerrain());
        HoldIfNull(c->pollution, c->city->GetPollutionSimulator());
        HoldIfNull(c->budget, c->city->GetBudgetSimulator());
    }
    if (c->msgServer == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) HoldIfNull(c->msgServer, static_cast<cIGZMessageServer2*>(ms));
    }
}

template <class T>
void ReleaseHeld(T*& field) {
    if (field != nullptr) FC4_RELEASE(field);
    field = nullptr;
}

// §1.2 steps 2 and 4: the message server, then the budget, pollution,
// terrain and city, each released only if held and then nulled (one call
// after the zones' Shutdown, which §1.2 shows is not observable).
void ReleaseServices(void* u) {
    fc4::LandfillContext* c = Ctx(u);
    ReleaseHeld(c->msgServer);
    ReleaseHeld(c->budget);
    ReleaseHeld(c->pollution);
    ReleaseHeld(c->terrain);
    ReleaseHeld(c->city);
}

// An agent's cIGZMessageTarget2 pointer (the value the original's simulator
// orders a level by, foundations §13.5), 0 when it has none.
uintptr_t TargetAddress(cIGZUnknown* agent) {
    cIGZUnknown* t = Query<cIGZUnknown>(agent, kIID_cIGZMessageTarget2);
    Drop(t);
    return reinterpret_cast<uintptr_t>(t);
}

// LG6 (§3.3 [compat]): the month agent must run after pollution's and the
// budget's. FreeCity4's simulator ranks it by name (foundations §13.4). The
// game's own simulator walks a level in ascending target-pointer order: the
// line logs where this object's target falls and warns when it is not above
// both (the order cannot be changed from here).
void LogMonthOrder(fc4::LandfillContext* c, cISC4City* city, cISC4Simulator* sim) {
    if (fc4::IsOurObject(sim)) {
        fc4::Log("landfill: month agent registered with FreeCity4's simulator: ranked after the unranked month "
                 "agents (pollution, budget) and before neighbours and ordinance (§3.3, foundations §13.4)");
        return;
    }
    const uintptr_t me = reinterpret_cast<uintptr_t>(c->target);
    const uintptr_t pol = TargetAddress(city->GetPollutionSimulator());
    const uintptr_t bud = TargetAddress(city->GetBudgetSimulator());
    const bool ok = (pol == 0 || me > pol) && (bud == 0 || me > bud);
    fc4::Log("landfill: %smonth agent registered with the game's simulator (address order): landfill target %p, "
             "pollution %p, budget %p; %s",
             ok ? "" : "WARNING ", reinterpret_cast<void*>(me), reinterpret_cast<void*>(pol),
             reinterpret_cast<void*>(bud),
             ok ? "landfill runs after both (§3.3)"
                : "landfill runs before one of them: its intake and maintenance reach pollution and the budget "
                  "one month early (§3.3)");
}

// §1.1 step 2: simulator slot 19 AddAgent(target, level, name, flags). Step 2
// runs before step 3 acquires the city, so the simulator comes from the app's
// city; it is held until RemoveAgent (§1.2 removes the agent after the
// services are released; LG18 [choice], the header).
void AddAgent(void* u, uint32_t level, const char* name, size_t len, uint32_t flags) {
    fc4::LandfillContext* c = Ctx(u);
    if (c->target == nullptr || name == nullptr) return;
    cISC4City* city = c->city ? c->city : AppCity();
    cISC4Simulator* raw = city ? city->GetSimulator() : nullptr;  // city slot 50, borrowed
    if (raw == nullptr) {
        fc4::Log("landfill: no simulator; agent not registered");
        return;
    }
    if (c->agentSim == nullptr) {
        c->agentSim = raw;
        FC4_ADDREF(c->agentSim);
    }
    const cRZBaseString n(name, len);
    fc4::AsGame(raw)->AddAgent(c->target, level, n, flags);  // slot 19
    c->agentsRegistered = true;
    if (level == 6) LogMonthOrder(c, city, raw);
}

// §1.2: simulator slot 20 RemoveAgent(target), on the simulator AddAgent used.
void RemoveAgent(void* u) {
    fc4::LandfillContext* c = Ctx(u);
    if (c->agentSim == nullptr) return;
    if (c->agentsRegistered) {
        fc4::AsGame(c->agentSim)->RemoveAgentAllLevels(c->target);
    }
    c->agentsRegistered = false;
    FC4_RELEASE(c->agentSim);
    c->agentSim = nullptr;
}

void Subscribe(void* u, uint32_t type) {
    fc4::LandfillContext* c = Ctx(u);
    if (c->msgServer != nullptr && c->target != nullptr) c->msgServer->AddNotification(c->target, type);
}

void Unsubscribe(void* u, uint32_t type) {
    fc4::LandfillContext* c = Ctx(u);
    if (c->msgServer != nullptr && c->target != nullptr) c->msgServer->RemoveNotification(c->target, type);
}

// ---- references -----------------------------------------------------------------------------------

// The department (borrowed from the budget simulator, §3.2) and the zots
// (borrowed from AddLotProp, or read back from a save with one reference).
void AddRefToken(void*, uint64_t t) {
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->AddRef();
    } else {
        FC4_ADDREF(o);
    }
}

void ReleaseToken(void*, uint64_t t) {
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->Release();
    } else {
        FC4_RELEASE(o);
    }
}

// ---- resources ------------------------------------------------------------------------------------

fc4_props* LoadExemplar(void* u, uint32_t type, uint32_t group, uint32_t instance) {
    if (!Gate(u, fc4::Sw::ExemplarLoads, "landfill exemplar load (none)")) return nullptr;
    fc4::props::Exemplar ex;
    if (!ex.Load(type, group, instance)) return nullptr;
    return fc4::props::ToCoreAll(ex.Holder());
}

// §4.1: a missing exemplar leaves the code defaults.
fc4_props* Tuning(void* u) {
    fc4_props* p = LoadExemplar(u, kTuningType, kTuningGroup, kTuningInstance);
    if (p == nullptr && !Ctx(u)->readOnly) {
        fc4::Log("landfill: tuning exemplar unavailable; the code defaults stay");
    }
    return p;
}

fc4_props* Exemplar(void* u, uint32_t type, uint32_t group, uint32_t instance) {
    return LoadExemplar(u, type, group, instance);
}

// ---- city -----------------------------------------------------------------------------------------

void CellCounts(void* u, int32_t* x, int32_t* z) {
    cISC4City* c = City(u);
    if (x != nullptr) *x = c ? static_cast<int32_t>(c->CellCountX()) : 0;  // slot 93
    if (z != nullptr) *z = c ? static_cast<int32_t>(c->CellCountZ()) : 0;  // slot 94
}

// City slot 97 CellCenterToPosition (pop 16). docs/abi (corrected
// 2026-10-14): returns nothing, the results come through the out references,
// as the SDK declares it. Calling it through a float declaration popped an
// empty x87 register on every call (stack underflow). §8: return "—".
void CellCenterToPosition(void* u, int32_t cx, int32_t cz, float* x, float* z) {
    float a = 0.0f, b = 0.0f;
    if (cISC4City* c = City(u)) c->CellCenterToPosition(cx, cz, a, b);
    if (x != nullptr) *x = a;
    if (z != nullptr) *z = b;
}

// ---- terrain (slots 13, 19, 29, 49, 68) -----------------------------------------------------------

// The held terrain (LG15); the shadow's context reads it from the city.
cISTETerrain* Terrain(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(u, fc4::Sw::LandfillTerrain, what)) return nullptr;
    return Ctx(u)->terrain ? Ctx(u)->terrain : c->GetTerrain();
}

float Altitude(void* u, float px, float pz) {
    cISTETerrain* t = Terrain(u, "landfill terrain GetAltitude (0)");
    return t ? t->GetAltitude(px, pz) : 0.0f;  // compiled slot 13 (foundations §16), ST0
}

float AltitudeAtNearestGrid(void* u, float px, float pz) {
    cISTETerrain* t = Terrain(u, "landfill terrain GetAltitudeAtNearestGrid (0)");
    return t ? t->GetAltitudeAtNearestGrid(px, pz) : 0.0f;  // slot 19, ST0
}

float SeaLevel(void* u) {
    cISTETerrain* t = Terrain(u, "landfill terrain GetSeaLevel (0)");
    return t ? t->GetSeaLevel() : 0.0f;  // slot 29, ST0
}

// The direct path of §4.5.3 (never taken by the shipped code).
void SetAltitude(void* u, int32_t x1, int32_t z1, int32_t x2, int32_t z2, float altitude) {
    cISTETerrain* t = Terrain(u, "landfill terrain SetAltitude");
    if (t != nullptr) t->SetAltitude(SC4Rect<int32_t>(x1, z1, x2, z2), altitude, true, 5);  // slot 49
}

// §4.4.2 step 3: RedisplayTerrain(1, 1, &rect, 5) (display only; it posts
// 0x278E6003 with data4 5, which the core ignores, §3.2).
void RedisplayTerrain(void* u, int32_t x1, int32_t z1, int32_t x2, int32_t z2) {
    cISTETerrain* t = Terrain(u, "landfill terrain RedisplayTerrain");
    if (t != nullptr) t->RedisplayTerrain(true, true, SC4Rect<int32_t>(x1, z1, x2, z2), 5);  // slot 68
}

// ---- pollution simulator (slots 20, 21, 22, 24, 29, 30, 49, 50, 51) -------------------------------

// The held pollution simulator (LG15); the shadow's context reads it from the
// city.
cFC4ISC4PollutionSimulator* Pollution(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(u, fc4::Sw::LandfillPollution, what)) return nullptr;
    cISC4PollutionSimulator* p = Ctx(u)->pollution ? Ctx(u)->pollution : c->GetPollutionSimulator();
    return p ? fc4::AsGame(p) : nullptr;
}

uint32_t GarbageSupply(void* u) {
    cFC4ISC4PollutionSimulator* p = Pollution(u, "landfill GetGarbageSupply (0)");
    return p ? p->GetGarbageSupply() : 0;  // slot 20
}

uint32_t LandfillTileCapacity(void* u) {
    cFC4ISC4PollutionSimulator* p = Pollution(u, "landfill GetLandfillTileCapacity (0)");
    return p ? static_cast<uint32_t>(p->GetLandfillTileCapacity()) : 0;  // slot 21
}

int32_t TotalGarbageUncollected(void* u) {
    cFC4ISC4PollutionSimulator* p = Pollution(u, "landfill GetTotalGarbageUncollected (0)");
    return p ? p->GetTotalGarbageUncollected() : 0;  // slot 24
}

void AddToGarbageSupply(void* u, int32_t delta) {
    cFC4ISC4PollutionSimulator* p = Pollution(u, "landfill AddToGarbageSupply");
    if (p != nullptr) p->AddToGarbageSupply(delta);  // slot 29
}

void SetGarbageCapacity(void* u, int32_t available) {
    cFC4ISC4PollutionSimulator* p = Pollution(u, "landfill SetGarbageCapacity");
    if (p != nullptr) p->SetGarbageCapacity(available);  // slot 30
}

void SetIsActiveLandfill(void* u, int32_t x, int32_t z, bool on) {
    if (x < 0 || z < 0) return;
    cFC4ISC4PollutionSimulator* p = Pollution(u, "landfill SetIsActiveLandfill");
    if (p != nullptr) p->SetIsActiveLandfill(static_cast<uint32_t>(x), static_cast<uint32_t>(z), on);  // slot 50
}

// Slot 51 (pops 4); the result is ignored (§4.7.2). [abi] The two
// declarations disagree on its return channel (the SDK and FreeCity4's
// pollution say int64 in EDX:EAX, docs/abi says EAX; both are waived in
// abi_conformance_test for as long as no plugin source calls the method by
// name): called through the vtable with the result discarded, which is right
// for either.
using PutInLandfillFn = void(__fastcall*)(void* self, void* edx, int32_t n);

void GarbagePutInLandfill(void* u, int32_t n) {
    cFC4ISC4PollutionSimulator* p = Pollution(u, "landfill GarbagePutInLandfill");
    if (p == nullptr) return;
    void** vtbl = *reinterpret_cast<void***>(p);
    reinterpret_cast<PutInLandfillFn>(vtbl[51])(p, nullptr, n);
}

int32_t GarbageCapacity(void* u) {
    cFC4ISC4PollutionSimulator* p = Pollution(u, "landfill GetGarbageCapacity (0)");
    return p ? p->GetGarbageCapacity() : 0;  // slot 22
}

bool IsActiveLandfill(void* u, int32_t x, int32_t z) {
    if (x < 0 || z < 0) return false;
    cFC4ISC4PollutionSimulator* p = Pollution(u, "landfill IsActiveLandfill (false)");
    return p != nullptr && p->IsActiveLandfill(static_cast<uint32_t>(x), static_cast<uint32_t>(z));  // slot 49
}

// ---- budget (§3.2, §4.9) --------------------------------------------------------------------------

// The held budget simulator (LG15); the shadow's context reads it from the
// city (slot 52).
cFC4ISC4BudgetSimulator* Budget(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(u, fc4::Sw::LandfillBudget, what)) return nullptr;
    cISC4BudgetSimulator* b = Ctx(u)->budget ? Ctx(u)->budget : c->GetBudgetSimulator();
    return b ? fc4::AsGame(b) : nullptr;
}

// Slot 28 GetDepartmentBudget(id): borrowed (the core AddRefs it).
uint64_t GetDepartmentBudget(void* u, uint32_t id) {
    cFC4ISC4BudgetSimulator* b = Budget(u, "landfill GetDepartmentBudget (none)");
    return b ? TokenOf(b->GetDepartmentBudget(id)) : 0;
}

// Department slot 10: a pointer to the funding percentage (§3.2 FundsChanged).
float DepartmentFunding(void* u, uint64_t dept) {
    if (!Gate(u, fc4::Sw::LandfillBudget, "landfill GetFundingPercentage (1.0)")) return 1.0f;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    SC4Percentage* p = d ? d->GetFundingPercentage() : nullptr;
    return p ? p->percentage : 1.0f;
}

// Department slot 20 GetLineItem (borrowed), line item slot 11 (§4.9.2).
bool LineItemCurrentExpenses(void* u, uint64_t dept, uint32_t line, int64_t* out) {
    if (!Gate(u, fc4::Sw::LandfillBudget, "landfill GetCurrentExpenses (none)")) return false;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    cISC4LineItem* li = d ? d->GetLineItem(line) : nullptr;
    if (li == nullptr) return false;
    if (out != nullptr) *out = li->GetCurrentExpenses();
    return true;
}

// §4.9.3: GetLineItem(30), AddRef, SetFullExpenses (line item slot 12), Release.
void SetLineItemFullExpenses(void* u, uint64_t dept, uint32_t line, int64_t value) {
    if (!Gate(u, fc4::Sw::LandfillBudget, "landfill SetFullExpenses")) return;
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    cISC4LineItem* li = d ? d->GetLineItem(line) : nullptr;
    if (li == nullptr) return;
    FC4_ADDREF(li);
    li->SetFullExpenses(value);
    FC4_RELEASE(li);
}

// The first value of `property` of a game occupant's property holder
// (occupant slot 6, borrowed): a landfill tile (§4.9.1). LG11 [compat]: the
// holder has no cost of its own, so GetProperty finds the default exemplar's,
// the tile exemplar InsertLandfillTile attached (foundations §10.3).
bool OccupantItemCost(void* u, uint64_t occ, uint32_t property, int64_t* out) {
    if (occ == 0 || !Gate(u, fc4::Sw::LandfillOccupants, "landfill occupant item cost (none)")) return false;
    std::vector<int64_t> v;
    if (!fc4::props::GetI64s(HolderOf(occ), property, v) || v.empty()) return false;
    if (out != nullptr) *out = v[0];
    return true;
}

// ---- zone manager (slots 6, 11, 12) ---------------------------------------------------------------

cISC4ZoneManager* Zones(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(u, fc4::Sw::LandfillZoning, what)) return nullptr;
    return c->GetZoneManager();  // city slot 36
}

int32_t ZoneCount(void* u, int32_t type) {
    cISC4ZoneManager* zm = Zones(u, "landfill GetZoneCount (0)");
    return zm ? zm->GetZoneCount(static_cast<cISC4ZoneManager::ZoneType>(type)) : 0;  // slot 6
}

int32_t ZoneTypeAt(void* u, int32_t x, int32_t z) {
    cISC4City* c = City(u);
    if (c == nullptr || x < 0 || z < 0) return 0;
    if (static_cast<uint32_t>(x) >= c->CellCountX() || static_cast<uint32_t>(z) >= c->CellCountZ()) return 0;
    cISC4ZoneManager* zm = Zones(u, "landfill GetZoneType (unzoned)");
    return zm ? static_cast<int32_t>(zm->GetZoneType(x, z)) : 0;  // slot 12
}

// Slot 11 PlaceZone(region, type, placeZone = true, skipFundsCheck = true,
// checkAreaSlope = false, skipZoneSizeCheck = true, checkLotSize = false,
// null, null, 0) (§4.4.8 step 4). The zone manager posts 0x07699455 for the
// cell; it reaches DoMessage later.
bool PlaceZone(void* u, const fc4_landfill_region* region, int32_t type) {
    cISC4ZoneManager* zm = Zones(u, "landfill PlaceZone (refused)");
    if (zm == nullptr) return false;
    RegionBuffer rb;
    if (!rb.Build(region)) return false;
    ++Ctx(u)->placeZones;
    return zm->PlaceZone(rb.Zoning(), static_cast<cISC4ZoneManager::ZoneType>(type), true, true, false, true, false,
                         nullptr, nullptr, 0);
}

// ---- occupants ------------------------------------------------------------------------------------

cISC4OccupantManager* Occupants(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(u, fc4::Sw::LandfillOccupants, what)) return nullptr;
    return c->GetOccupantManager();  // city slot 34
}

// The occupants of `type` in one cell: GetOccupantsByStandardCityCells(list,
// &cell, &cell, type, 0xFFFFFFFF). [abi] The typed overload (compiled slot 41)
// fills an SC4List (A) with one reference each (allocators.md), kept until
// the outermost call into the core returns; the list's destructor releases
// each entry, as slot 48 does in the original (§4.8.4). LG4: the original's
// presence and cost lookups use slot 33 (the first match, AddRef'd); §4.8.6
// confirms slot 41 is equivalent for them while a cell holds at most one
// landfill occupant, which CreateLandfillOccupant ensures.
std::shared_ptr<SC4List<cISC4Occupant*>> OccupantsInCell(void* u, cISC4OccupantManager* om, int32_t x, int32_t z,
                                                        uint32_t type) {
    auto list = std::make_shared<SC4List<cISC4Occupant*>>();
    const SC4Point<int> cell(x, z);
    om->GetOccupantsByStandardCityCells(*list, &cell, &cell, type, 0xFFFFFFFFu);  // slot 41
    Ctx(u)->held.push_back(list);
    return list;
}

uint64_t OccupantAt(void* u, int32_t x, int32_t z, uint32_t type) {
    cISC4OccupantManager* om = Occupants(u, "landfill occupant lookup (none)");
    if (om == nullptr || x < 0 || z < 0) return 0;
    for (cISC4Occupant* o : *OccupantsInCell(u, om, x, z, type)) {
        if (o != nullptr) return TokenOf(o);
    }
    return 0;
}

// §4.8.3: slot 21 RemoveOccupant(occupant, true, 3).
bool OccupantRemove(void* u, uint64_t occ) {
    cISC4OccupantManager* om = Occupants(u, "landfill RemoveOccupant");
    if (om == nullptr || occ == 0) return false;
    ++Ctx(u)->occupantsRemoved;
    return om->RemoveOccupant(OccOf(occ), true, kRemoveMsgData);
}

// §4.5.2: slot 24 (occupant, true) after a move [inferred].
void OccupantChanged(void* u, uint64_t occ) {
    cISC4OccupantManager* om = Occupants(u, "landfill MoveOccupant");
    if (om != nullptr && occ != 0) om->MoveOccupant(OccOf(occ), true);
}

void OccupantPosition(void* u, uint64_t occ, float* xyz) {
    if (xyz == nullptr) return;
    xyz[0] = xyz[1] = xyz[2] = 0.0f;
    if (occ == 0 || !Gate(u, fc4::Sw::LandfillOccupants, "landfill occupant GetPosition (origin)")) return;
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);  // slot 8
    xyz[0] = v.fX;
    xyz[1] = v.fY;
    xyz[2] = v.fZ;
}

void OccupantSetPosition(void* u, uint64_t occ, const float* xyz) {
    if (xyz == nullptr || occ == 0 || !Gate(u, fc4::Sw::LandfillOccupants, "landfill occupant SetPosition")) return;
    const cS3DVector3 v(xyz[0], xyz[1], xyz[2]);
    OccOf(occ)->SetPosition(&v);  // slot 9
}

void OccupantSetRemovalFlags(void* u, uint64_t occ, uint32_t flags) {
    if (occ == 0 || !Gate(u, fc4::Sw::LandfillOccupants, "landfill occupant SetRemovalFlags")) return;
    OccOf(occ)->SetRemovalFlags(flags);  // slot 12
}

// §4.5.2: slot 15 (0x99EF1145, 0), the display notification [inferred].
void OccupantPostMessage(void* u, uint64_t occ, uint32_t message) {
    if (occ == 0 || !Gate(u, fc4::Sw::LandfillOccupants, "landfill occupant PostOccupantMessage")) return;
    OccOf(occ)->PostOccupantMessage(message, 0);
}

bool IsLandfillOccupant(void* u, uint64_t occ) {
    if (occ == 0 || !Gate(u, fc4::Sw::LandfillOccupants, "landfill occupant QueryInterface (not a tile)")) {
        return false;
    }
    cIGZUnknown* p = Query<cIGZUnknown>(FromToken(occ), kIID_cISC4LandfillOccupant);
    Drop(p);
    return p != nullptr;
}

// cISC4LandfillOccupant slot 9 (§7.3).
bool IsEmptyLandfillTile(void* u, uint64_t occ) {
    if (occ == 0 || !Gate(u, fc4::Sw::LandfillOccupants, "landfill IsEmptyLandfillTile (false)")) return false;
    cISC4LandfillOccupant* t = Query<cISC4LandfillOccupant>(FromToken(occ), kIID_cISC4LandfillOccupant);
    if (t == nullptr) return false;
    const bool empty = t->IsEmptyLandfillTile();
    Drop(t);
    return empty;
}

// Every occupant of `type` in the cell, removed with (true, 3).
void RemoveInCell(void* u, int32_t x, int32_t z, uint32_t type, const char* what) {
    cISC4OccupantManager* om = Occupants(u, what);
    if (om == nullptr || x < 0 || z < 0) return;
    for (cISC4Occupant* o : *OccupantsInCell(u, om, x, z, type)) {
        if (o == nullptr) continue;
        om->RemoveOccupant(o, true, kRemoveMsgData);
        ++Ctx(u)->occupantsRemoved;
    }
}

// §4.8.4 (LG1 [compat]): RemoveIncompatibleOccupants(x, z) removes every
// flora occupant in the cell, in list order, with (true, 3).
void RemoveIncompatibleOccupants(void* u, int32_t x, int32_t z) {
    RemoveInCell(u, x, z, kOccupantTypeFlora, "landfill flora removal");
}

// The texture record SetTextureSpecification takes (§4.8.5, lot-records.md
// §6): the saved record's 14 bytes in their saved order [inferred: the
// in-memory record is laid out as saved; spec question]. With one record the
// stride does not matter; the two trailing bytes are zero so a 16-byte stride
// reads nothing past the record.
struct TextureRecord {
    uint32_t texture;
    uint8_t x;
    uint8_t z;
    uint8_t orientation;
    uint8_t objectByte3;
    uint8_t unnamed[5];
    uint8_t frame;
    uint8_t pad[2];
};
static_assert(offsetof(TextureRecord, x) == 4 && offsetof(TextureRecord, objectByte3) == 7 &&
                  offsetof(TextureRecord, unnamed) == 8 && offsetof(TextureRecord, frame) == 13,
              "texture record layout (lot-records.md §6)");

// §4.8.5 (LG2 [compat]): SetEmptyCellTexture(x, z) gives the cell the
// landfill ground texture: the record {0x1005, x, z, 0, 0, FF FF FF alpha 01,
// frame 0} (alpha: zone manager slot 16, shipped 0xAA), then
// CreateInstance(0xC97F987C, 0x097F6D98), AsOccupant()->Init(), SetUsage(1),
// SetTextureSpecification(&record, 1, null), and slot 24 (occ, false) plus
// PostOccupantMessage(0x99EF1145, 0) if the occupant is already inserted,
// slot 20 InsertOccupant(occ, 0) otherwise (result not checked); release.
// [choice] The original's direct construction when GZCOM fails is not
// reproduced: the cell keeps no texture (logged once). The callers check for
// an existing texture first (§4.4.5, §4.8.1).
void SetEmptyCellTexture(void* u, int32_t x, int32_t z) {
    cISC4OccupantManager* om = Occupants(u, "landfill SetEmptyCellTexture");
    if (om == nullptr) return;
    cISC4ZoneManager* zm = Zones(u, "landfill GetZoneDisplayAlpha (0xAA)");
    TextureRecord rec{};
    rec.texture = kLandfillTexture;
    rec.x = static_cast<uint8_t>(x);
    rec.z = static_cast<uint8_t>(z);
    rec.unnamed[0] = rec.unnamed[1] = rec.unnamed[2] = 0xFF;
    rec.unnamed[3] = zm ? zm->GetZoneDisplayAlpha() : kShippedZoneDisplayAlpha;  // slot 16
    rec.unnamed[4] = 0x01;
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (com == nullptr ||
        !com->GetClassObject(kCLSID_cSC4LotBaseTextureOccupant, kIID_cISC4LotBaseTextureOccupant, &p) || p == nullptr) {
        ++Ctx(u)->texturesFailed;
        NoteOnce(g_noteTexture, "landfill: WARNING could not create a base-texture occupant through GZCOM (class "
                                "0xC97F987C); landfill cells show no landfill ground texture");
        return;
    }
    cISC4LotBaseTextureOccupant* tex = static_cast<cISC4LotBaseTextureOccupant*>(p);
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(tex), "GetClassObject");
    if (cISC4Occupant* o = tex->AsOccupant()) o->Init();  // slot 3, occupant slot 3
    tex->SetUsage(kLandfillTextureUsage);                 // slot 5
    tex->SetTextureSpecification(
        reinterpret_cast<const cISC4LotBaseTextureOccupant::LotBaseTextureSpecification*>(&rec), 1, nullptr);  // 8
    if (cISC4Occupant* occ = tex->AsOccupant()) {
        if (occ->IsFlagSet(kOccupantFlagInserted)) {
            om->MoveOccupant(occ, false);  // slot 24
            occ->PostOccupantMessage(kMsgOccupantChanged, 0);
        } else {
            om->InsertOccupant(occ, 0);  // slot 20
        }
        ++Ctx(u)->texturesInserted;
    }
    FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(tex));
}

// §4.8.4 (LG1 [compat]): RemoveBaseTexture(x, z) lists the base textures in
// the cell and removes, through AsOccupant with (true, 3), only those whose
// cISC4LotBaseTextureOccupant (0x097F6D98) usage is 1, the landfill's; a
// lot's texture (usage 2) stays.
void RemoveBaseTexture(void* u, int32_t x, int32_t z) {
    cISC4OccupantManager* om = Occupants(u, "landfill base-texture removal");
    if (om == nullptr || x < 0 || z < 0) return;
    for (cISC4Occupant* o : *OccupantsInCell(u, om, x, z, kOccupantTypeBaseTexture)) {
        cISC4LotBaseTextureOccupant* tex =
            Query<cISC4LotBaseTextureOccupant>(o, kIID_cISC4LotBaseTextureOccupant);
        if (tex == nullptr) continue;
        if (tex->GetUsage() == kLandfillTextureUsage) {  // slot 4
            if (cISC4Occupant* occ = tex->AsOccupant()) {  // slot 3
                om->RemoveOccupant(occ, true, kRemoveMsgData);
                ++Ctx(u)->occupantsRemoved;
                ++Ctx(u)->texturesRemoved;
            }
        }
        Drop(tex);
    }
}

// §4.8.1 step 3 (LG3), the [choice] route through public interfaces: load the
// tile exemplar (0x6534284A, 0x07BDDF1C, tile); CreateInstance(0x8A22BEBE,
// 0x2A15F262); SetDefaultExemplar(ex) on its cISCExemplarPropertyHolder
// (0x0AC2B5F7, slot 3, result not defined); SetTileID(tile) (slot 6, now
// harmless: the key already has that instance; the previous ID it returns is
// ignored); SetOrientation (slot 8); SetFlag(4), SetRemovalFlags(2),
// SetPosition; Init; InsertOccupant(occ, 0) (slot 20) while this function
// still holds its reference (the quirk of step 7 is not reproduced); release.
// The core set the cell's active-landfill flag already. Returns the
// occupant's token (the occupant manager holds it), 0 when none was made: a
// missing exemplar (the original crashes) or class, or a refused insert. The
// cell (x, z) is not needed: the position places the occupant.
uint64_t InsertLandfillTile(void* u, int32_t /*x*/, int32_t /*z*/, uint32_t tile, int32_t orientation,
                            const float* xyz) {
    fc4::LandfillContext* c = Ctx(u);
    cISC4OccupantManager* om = Occupants(u, "landfill tile creation (kept in FreeCity4)");
    if (om == nullptr || xyz == nullptr) return 0;
    fc4::props::Exemplar ex;
    if (!ex.Load(kTileExemplarType, kTileExemplarGroup, tile) || ex.Resource() == nullptr) {
        ++c->tilesFailed;
        return 0;
    }
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (com == nullptr || !com->GetClassObject(kCLSID_cSC4LandfillOccupant, kIID_cISC4LandfillOccupant, &p) ||
        p == nullptr) {
        ++c->tilesFailed;
        NoteOnce(g_noteTile, "landfill: WARNING could not create a landfill occupant through GZCOM (class "
                             "0x8A22BEBE); the tiles are kept in FreeCity4 (no model, not saved by the game)");
        return 0;
    }
    cISC4LandfillOccupant* t = static_cast<cISC4LandfillOccupant*>(p);
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(t), "GetClassObject");
    uint64_t token = 0;
    cISCExemplarPropertyHolder* holder = Query<cISCExemplarPropertyHolder>(t, kIID_cISCExemplarPropertyHolder);
    cISC4Occupant* occ = t->AsOccupant();  // slot 5
    if (holder != nullptr && occ != nullptr) {
        holder->SetDefaultExemplar(ex.Resource());  // slot 3: takes its own reference
        t->SetTileID(tile);                         // slot 6
        t->SetOrientation(orientation);             // slot 8
        occ->SetFlag(kTileFlag);
        occ->SetRemovalFlags(kTileRemovalFlags);
        const cS3DVector3 v(xyz[0], xyz[1], xyz[2]);
        occ->SetPosition(&v);
        t->Init();  // slot 3
        if (om->InsertOccupant(occ, 0)) token = TokenOf(occ);  // slot 20
    }
    Drop(holder);
    if (token != 0) {
        ++c->tilesInserted;
    } else {
        ++c->tilesFailed;
        NoteOnce(g_noteTileInsert,
                 "landfill: WARNING a landfill occupant could not be set up or inserted; that tile is kept in FreeCity4");
    }
    FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(t));
    return token;
}

// ---- props (slots 26, 29) -------------------------------------------------------------------------

cISC4PropManager* Props(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(u, fc4::Sw::LandfillProps, what)) return nullptr;
    return c->GetPropManager();  // city slot 35
}

// §4.7.5: AddLotProp(propID, position, orientation) (slot 26, the
// (propID, pos, orientation) overload): borrowed (the core AddRefs it).
uint64_t AddLotProp(void* u, uint32_t prop, const float* xyz, int32_t orientation) {
    cISC4PropManager* pm = Props(u, "landfill AddLotProp (no zot)");
    if (pm == nullptr || xyz == nullptr) return 0;
    const cS3DVector3 v(xyz[0], xyz[1], xyz[2]);
    cISC4Occupant* o = pm->AddLotProp(prop, v, orientation);
    if (o == nullptr) {
        ++Ctx(u)->zotsFailed;
        return 0;
    }
    ++Ctx(u)->zotsPlaced;
    return TokenOf(o);
}

bool RemoveProp(void* u, uint64_t occ) {
    cISC4PropManager* pm = Props(u, "landfill RemovePropA");
    if (pm == nullptr || occ == 0) return false;
    ++Ctx(u)->zotsRemoved;
    return pm->RemovePropA(OccOf(occ));  // slot 29
}

// §4.7.5 (LG5 [compat]): the original uses the current view rotation, field
// +0x10 of the global SC4ViewUtilities object. [choice] 0, which matches the
// original at the view's first rotation (the spec's choice for a core with no
// view): that object is reached only through a global or cISC43DRender slot 8,
// and neither it nor its field is in docs/abi (spec question). The value is
// saved with the prop; nothing else reads it.
int32_t ZotOrientation(void*) {
    NoteOnce(g_noteZot, "landfill: the zot's orientation is 0 (the view rotation of §4.7.5 is not read)");
    return 0;
}

// ---- traffic ----------------------------------------------------------------------------------------

// §4.4.5: the traffic network map (city slot 40, landfill §8) slot 8
// FindNetworkTile(x, z, 0x1F4F, true). The tile is a pointer into the map,
// borrowed (LG7/LG17: only its presence is used, no reference is taken).
uint64_t FindNetworkTile(void* u, int32_t x, int32_t z, uint32_t mask, bool flag) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(u, fc4::Sw::LandfillTraffic, "landfill FindNetworkTile (none)")) return 0;
    auto* map = reinterpret_cast<cFC4ISC4TrafficNetworkMap*>(c->GetTrafficNetwork());
    if (map == nullptr) return 0;
    return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(map->FindNetworkTile(x, z, mask, flag)));
}

cISC4TrafficSimulator* Traffic(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(u, fc4::Sw::LandfillTraffic, what)) return nullptr;
    // [abi] The two slots fill a std::vector<uint32> with the game's default
    // allocator (B, allocators.md): without it (an unidentified executable)
    // nothing is asked.
    if (!fc4::StlNodeAllocator::Available()) {
        NoteOnce(g_noteVector,
                 "landfill: the game's vector allocator is not available; the traffic simulator is not asked (no "
                 "zone is connected)");
        return nullptr;
    }
    return c->GetTrafficSimulator();
}

// §4.7.2: traffic simulator slot 41 GetSubnetworksInRegion(region, &ids)
// (clears, then appends with B).
void SubnetworksInRegion(void* u, const fc4_landfill_region* region, fc4_token_callback cb, void* ctx) {
    if (cb == nullptr) return;
    cISC4TrafficSimulator* ts = Traffic(u, "landfill GetSubnetworksInRegion (none)");
    if (ts == nullptr) return;
    RegionBuffer rb;
    if (!rb.Build(region)) return;
    fc4::GameVector<uint32_t> ids;
    ts->GetSubnetworksInRegion(rb.Traffic(), *reinterpret_cast<SC4Vector<uint32_t>*>(&ids));
    for (uint32_t id : ids) cb(ctx, id);
}

// §4.7.2: slot 42 GetOccupantCountForAllSubnetworks(type, &counts)
// (assign(size, 0) with B, then counts).
void OccupantCountForAllSubnetworks(void* u, uint32_t type, fc4_token_callback cb, void* ctx) {
    if (cb == nullptr) return;
    cISC4TrafficSimulator* ts = Traffic(u, "landfill GetOccupantCountForAllSubnetworks (none)");
    if (ts == nullptr) return;
    fc4::GameVector<uint32_t> counts;
    ts->GetOccupantCountForAllSubnetworks(type, *reinterpret_cast<SC4Vector<uint32_t>*>(&counts));
    for (uint32_t n : counts) cb(ctx, n);
}

// ---- lot developer (§4.5.3) -----------------------------------------------------------------------

// buildingDevelopment.GetLotDeveloper() (BDS slot 36, borrowed)
// .AttemptAreaLevelling(rect, f, withWater, waterFlag, maxAreaSlope, null)
// (lot developer slot 21).
bool AttemptAreaLevelling(void* u, int32_t x1, int32_t z1, int32_t x2, int32_t z2, float f, bool withWater,
                          bool waterFlag, float maxAreaSlope) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(u, fc4::Sw::LandfillLevelling, "landfill AttemptAreaLevelling")) return false;
    cISC4BuildingDevelopmentSimulator* bds = c->GetBuildingDevelopmentSimulator();
    cISC4LotDeveloper* ld = bds ? bds->GetLotDeveloper() : nullptr;
    if (ld == nullptr) return false;
    ++Ctx(u)->levellings;
    return ld->AttemptAreaLevelling(SC4Rect<int32_t>(x1, z1, x2, z2), f, withWater, waterFlag, maxAreaSlope, nullptr);
}

// §3.2 PreViewLevelChange: only zones with a seagull effect read it, and none
// has one here (§4.10 is not implemented). LG16 gives a route through
// cISC4App::GetMainWindow and cIGZWin (SC4UI::GetView3DWin); cIGZWin is not
// in docs/abi, and with no seagull effect the level has no reader, so it is
// not taken.
uint32_t CurrentViewLevel(void*) {
    NoteOnce(g_noteView, "landfill: the 3D view level is not read (no zone has a seagull effect)");
    return 0;
}

// ---- zone effects (§4.10, rendering only) ---------------------------------------------------------

// [choice] Not implemented: the automata controllers and the seagull effect
// affect no simulation value (§4.10). Logged once, switch or not.
void InitZoneEffects(void* u, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, fc4_landfill_effects* out) {
    if (out != nullptr) *out = fc4_landfill_effects{};
    if (!Gate(u, fc4::Sw::LandfillEffects, "landfill zone effects")) return;
    NoteOnce(g_noteEffects,
             "landfill: the zone effects (garbage trucks, seagulls; §4.10, rendering only) are not created");
}

void ShutdownZoneEffects(void*, uint64_t, uint64_t, bool) {}
void SetTruckEnabled(void*, uint64_t, bool) {}
void StartSeagull(void*, uint64_t, uint32_t) {}
void StopSeagull(void*, uint64_t, uint32_t) {}

// ---- history (§5) ---------------------------------------------------------------------------------

// MessageSend (synchronous) of a new standard message 0x89EFA536 with
// data1 = series, data2 = value.
void SendHistory(void* u, uint32_t series, uint32_t value) {
    fc4::LandfillContext* c = Ctx(u);
    if (!Gate(u, fc4::Sw::LandfillHistory, "landfill history record")) return;
    cIGZMessageServer2* ms = c->msgServer;
    cIGZMessageServer2Ptr fallback;
    if (ms == nullptr) ms = fallback;
    if (ms == nullptr) return;
    cFC4Message2Standard* msg =
        FC4MakeMessage(kMsgHistoryRecord, static_cast<intptr_t>(series), static_cast<intptr_t>(value), 0);
    ms->MessageSend(msg);
    FC4_X87_CHECK("landfill MessageSend (after)");  // the game's handlers ran synchronously (FC4Fpu.h)
    msg->Release();
    ++c->historySends;
}

}  // namespace

namespace fc4 {

fc4_landfill_host MakeLandfillReadOnlyHost(LandfillContext* ctx) {
    fc4_landfill_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.tuning = &Tuning;
    h.exemplar = &Exemplar;
    h.cell_counts = &CellCounts;
    h.cell_center_to_position = &CellCenterToPosition;
    h.altitude = &Altitude;
    h.altitude_at_nearest_grid = &AltitudeAtNearestGrid;
    h.sea_level = &SeaLevel;
    h.garbage_supply = &GarbageSupply;
    h.landfill_tile_capacity = &LandfillTileCapacity;
    h.total_garbage_uncollected = &TotalGarbageUncollected;
    h.garbage_capacity = &GarbageCapacity;
    h.is_active_landfill = &IsActiveLandfill;
    h.get_department_budget = &GetDepartmentBudget;
    h.department_funding_percentage = &DepartmentFunding;
    h.line_item_current_expenses = &LineItemCurrentExpenses;
    h.occupant_item_cost = &OccupantItemCost;
    h.zone_count = &ZoneCount;
    h.zone_type = &ZoneTypeAt;
    h.occupant_at = &OccupantAt;
    h.occupant_position = &OccupantPosition;
    h.is_landfill_occupant = &IsLandfillOccupant;
    h.is_empty_landfill_tile = &IsEmptyLandfillTile;
    h.zot_orientation = &ZotOrientation;
    h.find_network_tile = &FindNetworkTile;
    h.subnetworks_in_region = &SubnetworksInRegion;
    h.occupant_count_for_all_subnetworks = &OccupantCountForAllSubnetworks;
    h.current_view_level = &CurrentViewLevel;
    return h;
}

fc4_landfill_host MakeLandfillHost(LandfillContext* ctx) {
    fc4_landfill_host h = MakeLandfillReadOnlyHost(ctx);
    // lifecycle
    h.acquire_services = &AcquireServices;
    h.release_services = &ReleaseServices;
    h.add_agent = &AddAgent;
    h.remove_agent = &RemoveAgent;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    // terrain, pollution, budget writes
    h.set_altitude = &SetAltitude;
    h.redisplay_terrain = &RedisplayTerrain;
    h.add_to_garbage_supply = &AddToGarbageSupply;
    h.set_garbage_capacity = &SetGarbageCapacity;
    h.set_is_active_landfill = &SetIsActiveLandfill;
    h.garbage_put_in_landfill = &GarbagePutInLandfill;
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.set_line_item_full_expenses = &SetLineItemFullExpenses;
    // zones, occupants, props, levelling
    h.place_zone = &PlaceZone;
    h.occupant_remove = &OccupantRemove;
    h.occupant_changed = &OccupantChanged;
    h.occupant_set_position = &OccupantSetPosition;
    h.occupant_set_removal_flags = &OccupantSetRemovalFlags;
    h.occupant_post_message = &OccupantPostMessage;
    h.remove_incompatible_occupants = &RemoveIncompatibleOccupants;
    h.set_empty_cell_texture = &SetEmptyCellTexture;
    h.remove_base_texture = &RemoveBaseTexture;
    h.insert_landfill_tile = &InsertLandfillTile;  // ABI 33 (LG3)
    h.add_lot_prop = &AddLotProp;
    h.remove_prop = &RemoveProp;
    h.attempt_area_levelling = &AttemptAreaLevelling;
    // effects, history
    h.init_zone_effects = &InitZoneEffects;
    h.shutdown_zone_effects = &ShutdownZoneEffects;
    h.set_truck_enabled = &SetTruckEnabled;
    h.start_seagull = &StartSeagull;
    h.stop_seagull = &StopSeagull;
    h.send_history = &SendHistory;
    return h;
}

}  // namespace fc4

// ---- object ---------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4LandfillDeveloper() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4LandfillDeveloper*>(new cFC4LandfillDeveloper()));
}

cFC4LandfillDeveloper::cFC4LandfillDeveloper() {
    InitializeCriticalSection(&lock_);
    mainTid_ = GetCurrentThreadId();  // the class store creates city components on the main thread
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    host_ = fc4::MakeLandfillHost(&ctx_);
    core_ = fc4_landfill_create(&host_, TokenOf(Primary()));
    if (core_ == nullptr) fc4::Log("cFC4LandfillDeveloper: core creation failed");
    fc4::Log("cFC4LandfillDeveloper(%p) created, core %p", static_cast<void*>(Primary()), static_cast<void*>(core_));
}

cFC4LandfillDeveloper::~cFC4LandfillDeveloper() {
    fc4_landfill_destroy(core_);  // releases what a loaded, never-initialised object still holds
    core_ = nullptr;
    ctx_.held.clear();
    fc4::Log("cFC4LandfillDeveloper destroyed (%u fallbacks)", fallbacks_.load());
    DeleteCriticalSection(&lock_);
}

// §1: which sub-object answers is the core's decision; anything else is false
// and the out pointer is left untouched.
bool cFC4LandfillDeveloper::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr || !fc4_landfill_query_interface(riid)) return false;
    if (riid == kIID_cIGZMessageTarget2) {
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
    } else if (riid == kIID_cIGZSerializable) {
        *ppvObj = static_cast<cIGZSerializable*>(this);
    } else {
        *ppvObj = Primary();  // 0x00000001 and 0x8A09D502
    }
    ++refCount_;
    return true;
}

uint32_t cFC4LandfillDeveloper::AddRef() { return ++refCount_; }

// §1: at a count of 1 (or below) Shutdown, destroy, return 0 (the count is
// not decremented first).
uint32_t cFC4LandfillDeveloper::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    Shutdown();
    delete this;
    return 0;
}

// ---- the lock -------------------------------------------------------------------------------------

bool cFC4LandfillDeveloper::Enter(bool wait) {
    const DWORD me = GetCurrentThreadId();
    if (wait || mainTid_ == 0 || me == mainTid_) {
        EnterCriticalSection(&lock_);
        return true;
    }
    if (TryEnterCriticalSection(&lock_)) return true;
    const ULONGLONG until = GetTickCount64() + kLockWaitMs;
    while (GetTickCount64() < until) {
        Sleep(1);
        if (TryEnterCriticalSection(&lock_)) return true;
    }
    ++fallbacks_;
    return false;
}

void cFC4LandfillDeveloper::Leave() { LeaveCriticalSection(&lock_); }

cFC4LandfillDeveloper::Section::Section(const cFC4LandfillDeveloper* s, bool wait)
    : s_(const_cast<cFC4LandfillDeveloper*>(s)), ok_(s_->Enter(wait)) {
    if (ok_) ++s_->depth_;
}

cFC4LandfillDeveloper::Section::~Section() {
    if (!ok_) return;
    if (--s_->depth_ == 0) {
        // The outermost call: the lists kept for it go, and the values other
        // threads fall back to are refreshed.
        s_->ctx_.held.clear();
        s_->RefreshSnapshot();
    }
    s_->Leave();
}

// Only the core's own state (no host calls): any thread holding the lock.
void cFC4LandfillDeveloper::RefreshSnapshot() {
    if (core_ == nullptr) return;
    snap_.maintenance = fc4_landfill_maintenance_costs(core_);
    uint32_t usage = 0, capacity = 0;
    fc4_landfill_usage(core_, &usage, &capacity);
    snap_.usage = usage;
    snap_.capacity = capacity;
}

// The status lines (Init, Read, month). Inside a section.
void cFC4LandfillDeveloper::LogState(const char* what) {
    fc4_landfill_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_landfill_get_state(core_, &st)) return;
    fc4::Log("landfill %s: init=%u loaded=%u zones=%u connected=%u cells=%u heaps=%u valid=%u active=%u stored=%llu "
             "capacity=%u used=%u free=%d reported=%u moved=%lld supplyDelta=%lld maintenance=%lld activeCells=%u "
             "tiles=%u rebuilt=%u gameTiles=%u gameTileFailures=%u zots=%u department=%p reentrant=%u fallbacks=%u "
             "offThreadMessages=%u placeZones=%u zotsPlaced=%u zotsFailed=%u zotsRemoved=%u occupantsRemoved=%u "
             "tilesInserted=%u tilesFailed=%u textures=%u texturesFailed=%u texturesRemoved=%u levellings=%u "
             "history=%u months=%u",
             what, st.initialized, st.loaded, st.zones, st.connected_zones, st.cells, st.heaps, st.valid_heaps,
             st.active_heaps, static_cast<unsigned long long>(st.stored), st.capacity_total, st.used_total,
             st.free_capacity, st.capacity_reported, static_cast<long long>(st.moved),
             static_cast<long long>(st.supply_delta), static_cast<long long>(st.maintenance), st.active_cells,
             st.virtual_tiles, st.rebuilt_tiles, st.game_tiles, st.game_tile_failures, st.zots,
             static_cast<void*>(FromToken(st.department)), st.reentrant_calls, fallbacks_.load(),
             offThreadMessages_.load(), ctx_.placeZones, ctx_.zotsPlaced, ctx_.zotsFailed, ctx_.zotsRemoved,
             ctx_.occupantsRemoved, ctx_.tilesInserted, ctx_.tilesFailed, ctx_.texturesInserted,
             ctx_.texturesFailed, ctx_.texturesRemoved, ctx_.levellings, ctx_.historySends, months_);
}

// ---- lifecycle (§1.1, §1.2) -----------------------------------------------------------------------

bool cFC4LandfillDeveloper::Init() {
    FC4_TRACE("LandfillDeveloper::Init");
    mainTid_ = GetCurrentThreadId();
    Section sec(this);
    fc4_landfill_init(core_);  // §1.1, all of it (the core drives every host call)
    LogState("Init");
    return true;  // §1.1 step 5
}

bool cFC4LandfillDeveloper::Shutdown() {
    FC4_TRACE("LandfillDeveloper::Shutdown");
    // Keep this object alive while the simulator and the message server drop
    // their references (unless this already is the final Release).
    if (!destroying_) Primary()->AddRef();
    {
        Section sec(this);
        fc4_landfill_shutdown(core_);  // §1.2
    }
    fc4::Log("landfill Shutdown: %u months, %u fallbacks", months_, fallbacks_.load());
    if (!destroying_) Primary()->Release();
    return true;  // §1.2
}

// ---- primary slots (§7.1) -------------------------------------------------------------------------

// Slot 5: by-value SC4Percentage through the hidden result pointer (§7.4).
SC4Percentage* cFC4LandfillDeveloper::GetEfficiency(SC4Percentage* result) {
    Section sec(this);
    float v = FromBits(snap_.efficiencyBits.load());
    if (sec.ok()) {
        v = fc4_landfill_efficiency(core_);
        snap_.efficiencyBits = ToBits(v);
    }
    if (result != nullptr) result->percentage = v;
    return result;
}

uint32_t cFC4LandfillDeveloper::GetSimulatorType() { return kSimulatorType; }

int64_t cFC4LandfillDeveloper::GetMaintenanceCosts() {
    Section sec(this);
    return sec.ok() ? fc4_landfill_maintenance_costs(core_) : snap_.maintenance.load();
}

int64_t cFC4LandfillDeveloper::GetCurrentBudget() {
    Section sec(this);
    if (!sec.ok()) return snap_.budget.load();
    const int64_t v = fc4_landfill_current_budget(core_);
    snap_.budget = v;
    return v;
}

// Slot 9: the zone manager's PlaceZone prices a landfill with it (§9); an
// int64 in EDX:EAX. The region is the game's (same layout as ZoningChange's).
int64_t cFC4LandfillDeveloper::GetZoningCost(SC4CellRegion<int32_t> const* region) {
    Section sec(this);
    if (!sec.ok()) return 0;
    std::vector<uint8_t> mask;
    fc4_landfill_region r{};
    if (!ReadGameRegion(region, mask, &r)) return 0;
    return fc4_landfill_zoning_cost(core_, &r);
}

// Slot 10 (§4.6.5): the caller's references (the SDK order is right, §7.4).
void cFC4LandfillDeveloper::GetLandfillUsage(uint32_t& usage, uint32_t& capacity) {
    Section sec(this);
    if (sec.ok()) {
        fc4_landfill_usage(core_, &usage, &capacity);
    } else {
        usage = snap_.usage.load();
        capacity = snap_.capacity.load();
    }
}

// Slot 11: the zone manager's count of zone-14 cells (§4.6.5).
int32_t cFC4LandfillDeveloper::GetLandfillTileCount() const {
    Section sec(this);
    if (!sec.ok()) return snap_.tiles.load();
    const int32_t n = fc4_landfill_tile_count(core_);
    const_cast<Snapshot&>(snap_).tiles = n;
    return n;
}

// Slot 12: [choice] null, which §9 allows; the zone object whose memory the
// city situation manager reads is not reproduced. LG9 [compat]: it asks only
// for a target in the tiles' occupant group 0x1406 (now that the tiles are
// game occupants, it can), and handles null by falling back to the target's
// lot.
intptr_t cFC4LandfillDeveloper::GetLandfillZoneFromCell(int32_t, int32_t) {
    if (GetCurrentThreadId() == mainTid_) {
        NoteOnce(g_noteZoneFromCell, "landfill: GetLandfillZoneFromCell returns null (§9: the zone object is not "
                                     "reproduced)");
    }
    return 0;
}

// ---- messages (§3.2) ------------------------------------------------------------------------------

bool cFC4LandfillDeveloper::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("landfill DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    // LG20 [choice]: wait for the lock on every thread, so no message is
    // dropped (the original takes no lock and handles every message; all of
    // them come from the main thread, foundations §18.2). One from another
    // thread is counted (the status line) and the first few are logged.
    if (mainTid_ != 0 && GetCurrentThreadId() != mainTid_) {
        const uint32_t n = ++offThreadMessages_;
        if (n <= kOffThreadMessagesLogged) {
            fc4::Log("landfill: DoMessage 0x%08X on thread %lu (not the main thread); it waits for the lock",
                     pMessage->GetType(), static_cast<unsigned long>(GetCurrentThreadId()));
        }
    }
    Section sec(this, true);
    const uint32_t type = pMessage->GetType();
    uint32_t d[4] = {};
    std::vector<uint8_t> mask;
    fc4_landfill_region region{};
    bool haveRegion = false;
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    if (s != nullptr) {
        d[0] = static_cast<uint32_t>(s->GetData1());
        d[1] = static_cast<uint32_t>(s->GetData2());
        d[2] = static_cast<uint32_t>(s->GetData3());
        d[3] = static_cast<uint32_t>(s->GetData4());
        // §3.2: GetVoid2 is the zoning region (the message's; copied by the core).
        if (type == kMsgZoningChange) haveRegion = ReadGameRegion(s->GetVoid2(), mask, &region);
    }
    fc4_landfill_do_message(core_, type, d[0], d[1], d[2], d[3], haveRegion ? &region : nullptr);
    Drop(s);
    if (type == kMsgSimNewMonth) {
        ++months_;
        if (depth_ == 1) LogState("month");
    }
    return true;  // §3.1: always
}

// ---- persistence (§6.1) ---------------------------------------------------------------------------

bool cFC4LandfillDeveloper::Write(cIGZOStream& s) {
    FC4_TRACE("LandfillDeveloper::Write");
    Section sec(this);
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_landfill_write(core_, a.Get());
    fc4::Log("landfill Write -> %d", ok ? 1 : 0);
    return ok;
}

// Runs before Init: the core sizes nothing from the city here.
bool cFC4LandfillDeveloper::Read(cIGZIStream& s) {
    FC4_TRACE("LandfillDeveloper::Read");
    Section sec(this);
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_landfill_read(core_, a.Get());
    fc4::Log("landfill Read -> %d", ok ? 1 : 0);
    LogState("Read");
    return ok;
}

uint32_t cFC4LandfillDeveloper::GetGZCLSID() { return kFC4_CLSID_cSC4LandfillDeveloper; }
