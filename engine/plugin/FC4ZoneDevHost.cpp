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

// The zone developer's host callbacks (fc4.h fc4_zonedev_host) over the
// game's interfaces, in the order of docs/specs/zones.md §12, and the master
// lot list mirror in the original's raw layout (cFC4ZoneDeveloper.h).
// Tokens handed to the core are BORROWED unless fc4.h says otherwise:
// references the game's lists give are kept in the context until the
// outermost call into the core returns (ZoneDevReleaseHeld).
//
// Main thread, or another thread holding the zone developer's lock; the
// shadow (FC4ZoneDevShadow.cpp) runs on the main thread.

#include "cFC4ZoneDeveloper.h"

#include "FC4Config.h"
#include "FC4DevelopmentInterfaces.h"
#include "FC4Fpu.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"
#include "cFC4ZoneManager.h"
#include "compat/FC4Compat.h"

#include "GZServPtrs.h"
#include "SC4CellRegion.h"
#include "SC4HashSet.h"
#include "SC4List.h"
#include "SC4Point.h"
#include "SC4Rect.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4Demolition.h"
#include "cISC4GrowthDeveloper.h"
#include "cISC4Lot.h"
#include "cISC4LotBaseTextureOccupant.h"
#include "cISC4LotConfiguration.h"
#include "cISC4LotConfigurationFilter.h"
#include "cISC4LotConfigurationManager.h"
#include "cISC4LotDeveloper.h"
#include "cISC4NetworkManager.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4NetworkTool.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4SimGrid.h"
#include "cISC4TrafficSimulator.h"
#include "cISC4ZoneManager.h"
#include "cISTETerrain.h"
#include "SC4Vector.h"
#include "cS3DVector3.h"
#include "cSC4LotConfigurationObject.h"
#include "cRZCOMDllDirector.h"

#include <atomic>
#include <cfloat>
#include <cstring>
#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cISC4NetworkOccupant = 0xA821EF94;  // §12
// §5.3: DeferOccupantOperations, posted with data1 = 1 / 0.
constexpr uint32_t kMsgDeferOccupantOperations = 0x49D968C4;
// §7.10: PlaceNetworkOccupants(a, b, 0xC49, the tool, realMode, -1).
constexpr uint32_t kPlaceFlags = 0xC49;
// §5.2: the network tools of types 3 (street), 11 (dirt road) and 0 (road).
constexpr uint32_t kToolKinds[3] = {3, 11, 0};
// DetermineLotSize's local hash sets (§7.2.1): hint 100 (193 buckets).
constexpr uint32_t kSetHint = 100;
// Registered safe lot iterators are few; a longer ring is not trusted.
constexpr uint32_t kMaxIterators = 4096;

fc4::ZoneDevContext* Ctx(void* u) { return static_cast<fc4::ZoneDevContext*>(u); }
cISC4City* City(void* u) { return Ctx(u)->city; }

// The zonedev_* switches; the shadow's context reads the game as it is.
bool Gate(void* u, fc4::Sw s, const char* what) {
    if (!Ctx(u)->gated || fc4::On(s)) return true;
    fc4::NoteSkipped(s, what);
    return false;
}

// Writes reach the game only from the replacement (the shadow core swallows
// them before they reach the table, and its table has them NULL anyway).
bool Writes(void* u) { return !Ctx(u)->readOnly; }

// Test seam op 53: a callback of the test table answers instead of the game.
#define ZD_TEST(name, ...)                                                                     \
    do {                                                                                       \
        const fc4_zonedev_host* t_ = Ctx(u)->test;                                             \
        if (t_ != nullptr && t_->name != nullptr) return t_->name(t_->user, __VA_ARGS__);      \
    } while (0)
#define ZD_TEST0(name)                                                                         \
    do {                                                                                       \
        const fc4_zonedev_host* t_ = Ctx(u)->test;                                             \
        if (t_ != nullptr && t_->name != nullptr) return t_->name(t_->user);                   \
    } while (0)

// A few lines, then silence (main thread, or under the lock).
void NoteFew(volatile LONG* n, const char* text) {
    if (InterlockedIncrement(n) <= 3) fc4::Log("%s", text);
}
volatile LONG g_noteNoAllocator = 0;
volatile LONG g_noteJoining = 0;
volatile LONG g_noteForeignIterator = 0;
volatile LONG g_noteMissing = 0;
volatile LONG g_noteFamilySet = 0;
volatile LONG g_noteNewLot = 0;
volatile LONG g_noteJoin = 0;
volatile LONG g_noteDezone = 0;

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

cFC4ISC4Lot* LotOf(uint64_t t) { return fc4::AsGame(reinterpret_cast<cISC4Lot*>(FromToken(t))); }
cISC4Lot* SdkLotOf(uint64_t t) { return reinterpret_cast<cISC4Lot*>(FromToken(t)); }
cISC4LotConfiguration* CfgOf(uint64_t t) { return reinterpret_cast<cISC4LotConfiguration*>(FromToken(t)); }
cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cFC4ISC4GrowthDeveloper* DevOf(uint64_t t) {
    return fc4::AsGame(reinterpret_cast<cISC4GrowthDeveloper*>(FromToken(t)));
}

cFC4ISC4BuildingDevelopmentSimulator* Bds(void* u) {
    cISC4BuildingDevelopmentSimulator* b = Ctx(u)->bds;
    if (b == nullptr && Ctx(u)->readOnly && City(u) != nullptr) b = City(u)->GetBuildingDevelopmentSimulator();
    return b ? fc4::AsGame(b) : nullptr;
}

cISC4LotDeveloper* LotDeveloper(void* u) {
    fc4::ZoneDevContext* c = Ctx(u);
    if (c->lotDeveloper != nullptr) return c->lotDeveloper;
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b ? b->GetLotDeveloper() : nullptr;  // BDS slot 36, borrowed
}

// ---- allocators (B nodes, C blocks; or the test seam's) ---------------------------------------------

std::atomic<void* (*)(uint32_t)> g_testAllocate{nullptr};
std::atomic<void (*)(void*)> g_testFree{nullptr};

using BlockAllocateFn = void*(__cdecl*)(uint32_t);
using BlockFreeFn = void(__cdecl*)(void*);

// ---- lifecycle, services, messages (§1.2, §5.2) -------------------------------------------------------

// §1.2 steps 1-2 without the 3D view: the lot developer (re-pointed), BDS,
// city, occupant manager, terrain, traffic simulator, zone manager and
// message server, each still null taken with a reference. The original reads
// globals the city fills; FreeCity4 asks the app's city. False (logged) when
// any is missing.
bool AcquireServices(void* u) {
    ZD_TEST0(acquire_services);
    fc4::ZoneDevContext* c = Ctx(u);
    if (c->readOnly) return true;
    TakeIfNull(c->city, AppCity());
    if (c->city == nullptr) {
        fc4::Log("zonedev: no city; services not acquired");
        return false;
    }
    TakeIfNull(c->bds, c->city->GetBuildingDevelopmentSimulator());
    // Step 1: AddRef the new lot developer, release the old.
    cISC4LotDeveloper* ld = c->bds ? c->bds->GetLotDeveloper() : nullptr;  // BDS slot 36
    if (ld != c->lotDeveloper) {
        if (ld != nullptr) FC4_ADDREF(ld);
        ReleaseSlot(c->lotDeveloper);
        c->lotDeveloper = ld;
    }
    TakeIfNull(c->occupants, c->city->GetOccupantManager());
    TakeIfNull(c->terrain, c->city->GetTerrain());
    TakeIfNull(c->traffic, c->city->GetTrafficSimulator());
    TakeIfNull(c->zones, c->city->GetZoneManager());
    if (c->msgServer == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            c->msgServer = ms;
            FC4_ADDREF(c->msgServer);
        }
    }
    // Step 2's null check covers the services it takes, not step 1's lot
    // developer (zones.md §1.2): at a city load BDS slot 36 can still be null
    // here; LotDeveloper() asks BDS again at each use.
    const bool all = c->bds && c->occupants && c->terrain && c->traffic && c->zones && c->msgServer;
    if (!all) {
        char line[256];
        _snprintf_s(line, sizeof(line), _TRUNCATE,
                    "zonedev: a service is missing (lotDeveloper=%d bds=%d occupants=%d terrain=%d traffic=%d "
                    "zones=%d messages=%d); Init fails (§1.2 step 2)",
                    c->lotDeveloper != nullptr, c->bds != nullptr, c->occupants != nullptr, c->terrain != nullptr,
                    c->traffic != nullptr, c->zones != nullptr, c->msgServer != nullptr);
        NoteFew(&g_noteMissing, line);
    } else if (c->lotDeveloper == nullptr) {
        NoteFew(&g_noteMissing, "zonedev: no lot developer yet at Init (BDS slot 36 null); asked again at each use");
    }
    return all;
}

void ReleaseServices(void* u) {
    ZD_TEST0(release_services);
    if (Writes(u)) fc4::ZoneDevReleaseServices(Ctx(u));
}

void Subscribe(void* u, uint32_t msg) {
    ZD_TEST(subscribe, msg);
    fc4::ZoneDevContext* c = Ctx(u);
    if (!Writes(u) || !Gate(u, fc4::Sw::ZoneDevMessages, "zonedev message subscription")) return;
    if (c->msgServer != nullptr && c->target != nullptr) c->msgServer->AddNotification(c->target, msg);
}

void Unsubscribe(void* u, uint32_t msg) {
    ZD_TEST(unsubscribe, msg);
    fc4::ZoneDevContext* c = Ctx(u);
    if (!Writes(u)) return;
    if (c->msgServer != nullptr && c->target != nullptr) c->msgServer->RemoveNotification(c->target, msg);
}

// City slots 93, 94.
void CellCounts(void* u, int32_t* x, int32_t* z) {
    ZD_TEST(cell_counts, x, z);
    cISC4City* c = City(u);
    if (x != nullptr) *x = c ? static_cast<int32_t>(c->CellCountX()) : 0;
    if (z != nullptr) *z = c ? static_cast<int32_t>(c->CellCountZ()) : 0;
}

cFC4ISC4NetworkTool* ToolOf(void* u, uint32_t kind) {
    for (int i = 0; i < 3; ++i) {
        if (kToolKinds[i] == kind) return reinterpret_cast<cFC4ISC4NetworkTool*>(Ctx(u)->tools[i]);
    }
    return nullptr;
}

// §5.2 PostCityInit: the network manager's GetNetworkTool(type, 1) (slot 5)
// for types 3, 11 and 0, each AddRef'd and its slot 3 (Init) called.
void AcquireNetworkTools(void* u) {
    ZD_TEST0(acquire_network_tools);
    fc4::ZoneDevContext* c = Ctx(u);
    if (!Writes(u) || c->city == nullptr) return;
    if (!Gate(u, fc4::Sw::ZoneDevNetworkTools, "zonedev network tools (none)")) return;
    cISC4NetworkManager* nm = c->city->GetNetworkManager();
    if (nm == nullptr) return;
    for (int i = 0; i < 3; ++i) {
        if (c->tools[i] != nullptr) continue;
        cISC4NetworkTool* t = nm->GetNetworkTool(static_cast<int32_t>(kToolKinds[i]), true);
        if (t == nullptr) continue;
        FC4_ADDREF(t);
        c->tools[i] = t;
        reinterpret_cast<cFC4ISC4NetworkTool*>(t)->Init();  // slot 3
    }
}

// §1.2 Shutdown: slot 4 then Release. GetNetworkTool(type, 1) hands out a
// tool made for the caller, whose only reference is the one AcquireNetworkTools
// took (the game runs showed the count reaching 0 here), so the free is the
// original's too: a release of a tool created for us.
void ReleaseNetworkTools(void* u) {
    ZD_TEST0(release_network_tools);
    fc4::ZoneDevContext* c = Ctx(u);
    if (!Writes(u)) return;
    for (cIGZUnknown*& t : c->tools) {
        if (t == nullptr) continue;
        reinterpret_cast<cFC4ISC4NetworkTool*>(t)->Shutdown();  // slot 4
        FC4_RELEASE_CREATED(t);
        t = nullptr;
    }
}

// §5.3: MessagePost of DeferOccupantOperations with data1 = 1 / 0.
void PostDefer(void* u, bool begin) {
    ZD_TEST(post_defer_occupant_operations, begin);
    fc4::ZoneDevContext* c = Ctx(u);
    if (!Writes(u) || c->msgServer == nullptr) return;
    if (!Gate(u, fc4::Sw::ZoneDevPosts, "zonedev DeferOccupantOperations post")) return;
    cFC4Message2Standard* m = cFC4Message2Standard::New();
    if (m == nullptr) return;
    m->AddRef();
    m->SetType(kMsgDeferOccupantOperations);
    m->SetData1(begin ? 1 : 0);
    ++c->posts;
    c->msgServer->MessagePost(m, false);
    m->Release();
}

// Zone manager slot 12 (sign-extended int8).
int32_t ZoneType(void* u, int32_t x, int32_t z) {
    ZD_TEST(zone_type, x, z);
    cISC4ZoneManager* zm = Ctx(u)->zones;
    if (zm == nullptr && Ctx(u)->readOnly && City(u) != nullptr) zm = City(u)->GetZoneManager();
    return zm ? fc4::AsGame(zm)->GetZoneType(x, z) : 0;
}

// §7.9 step 3: DemolishRegion(1, &region, 1, 1, 0, 0, 0, 0, 0, 0, 0)
// (demolition slot 6) on a region of ours in the §11.4 layout.
void DemolishRegion(void* u, const fc4_zone_region* region) {
    ZD_TEST(demolish_region, region);
    fc4::ZoneDevContext* c = Ctx(u);
    if (!Writes(u) || region == nullptr || c->city == nullptr) return;
    if (!Gate(u, fc4::Sw::ZoneDevDemolition, "zonedev DemolishRegion")) return;
    cISC4Demolition* d = c->city->GetDemolitionUtility();
    if (d == nullptr) return;
    fc4::ZoneRegionBuffer rb;
    if (!rb.Build(*region)) return;
    ++c->demolitions;
    d->DemolishRegion(true, *reinterpret_cast<const SC4CellRegion<int32_t>*>(rb.Raw()), 1, 1, false, nullptr,
                      nullptr, 0, nullptr, 0, 0);
    FC4_X87_CHECK("zonedev DemolishRegion");
}

// ---- networks (§7.10-§7.13, §8.8) ---------------------------------------------------------------------

// Network tool slot 7 PlaceNetworkOccupants(a, b, 0xC49, the tool, realMode,
// -1): AL only (§12: the upper EAX bytes are garbage).
bool StreetPlace(void* u, uint32_t kind, int32_t ax, int32_t az, int32_t bx, int32_t bz, bool realMode) {
    ZD_TEST(street_place, kind, ax, az, bx, bz, realMode);
    if (!Writes(u) || !Gate(u, fc4::Sw::ZoneDevStreets, "zonedev street Place (false)")) return false;
    cFC4ISC4NetworkTool* t = ToolOf(u, kind);
    if (t == nullptr) return false;
    ++Ctx(u)->streetsPlaced;
    const uint32_t r = t->ToolSlot07(static_cast<uint32_t>(ax), static_cast<uint32_t>(az), static_cast<uint32_t>(bx),
                                     static_cast<uint32_t>(bz), kPlaceFlags,
                                     static_cast<uint32_t>(reinterpret_cast<uintptr_t>(t)), realMode ? 1u : 0u,
                                     0xFFFFFFFFu);
    FC4_X87_CHECK("zonedev PlaceNetworkOccupants");
    return (r & 0xFFu) != 0;
}

// Network tool slot 5 Reset.
void ResetNetworkTool(void* u, uint32_t kind) {
    ZD_TEST(reset_network_tool, kind);
    if (!Writes(u) || !Gate(u, fc4::Sw::ZoneDevNetworkTools, "zonedev network tool Reset")) return;
    cFC4ISC4NetworkTool* t = ToolOf(u, kind);
    if (t != nullptr) t->ToolSlot05();
}

// Network occupant slot 47 ConnectsToNeighbor(edge, type) (AL).
bool ConnectsToNeighbor(void* u, uint64_t occupant, int32_t edge, uint32_t netType) {
    ZD_TEST(connects_to_neighbor, occupant, edge, netType);
    if (occupant == 0 || !Gate(u, fc4::Sw::ZoneDevOccupants, "zonedev ConnectsToNeighbor (false)")) return false;
    cISC4NetworkOccupant* n = Query<cISC4NetworkOccupant>(FromToken(occupant), kIID_cISC4NetworkOccupant);
    if (n == nullptr) return false;
    const bool r = n->ConnectsToNeighbor(edge, static_cast<cISC4NetworkOccupant::eNetworkType>(netType));
    Drop(n);
    return r;
}

// Network tool slot 9 OccupantExists(x, z, flag, &mask): AL only; the mask
// (4 bytes, one per network edge) is written only on a hit (§8.8).
bool NetworkToolOccupantExists(void* u, uint32_t kind, int32_t x, int32_t z, uint32_t flag, uint8_t* mask) {
    ZD_TEST(network_tool_occupant_exists, kind, x, z, flag, mask);
    if (!Gate(u, fc4::Sw::ZoneDevNetworkTools, "zonedev OccupantExists (false)")) return false;
    cFC4ISC4NetworkTool* t = ToolOf(u, kind);
    if (t == nullptr) return false;
    uint32_t m = 0;
    const uint32_t r = t->ToolSlot09(static_cast<uint32_t>(x), static_cast<uint32_t>(z), flag,
                                     static_cast<uint32_t>(reinterpret_cast<uintptr_t>(&m)));
    if ((r & 0xFFu) == 0) return false;
    if (mask != nullptr) std::memcpy(mask, &m, 4);
    return true;
}

// The ABI 51 street-joining hook: since re-specs11 the core runs
// JoinDeadEndStubs / JoinAcrossRail itself (§5.2.1, §5.2.2) and never calls
// it. Kept for the table; logs if it ever is.
void StreetJoiningHelper(void* u, uint64_t occupant, uint32_t data4) {
    ZD_TEST(street_joining_helper, occupant, data4);
    (void)occupant;
    char line[160];
    _snprintf_s(line, sizeof(line), _TRUNCATE, "zonedev: the old street-joining hook was called (data4=%u); ignored",
                data4);
    NoteFew(&g_noteJoining, line);
}

// The network occupant interface of an occupant token, with a reference
// (Drop it), or null when it is not a network occupant.
cISC4NetworkOccupant* Network(uint64_t occupant) {
    return occupant ? Query<cISC4NetworkOccupant>(FromToken(occupant), kIID_cISC4NetworkOccupant) : nullptr;
}

// Network occupant slot 12 GetNetworkFlags.
uint32_t NetworkFlags(void* u, uint64_t occupant) {
    ZD_TEST(network_flags, occupant);
    if (!Gate(u, fc4::Sw::ZoneDevOccupants, "zonedev GetNetworkFlags (0)")) return 0;
    cISC4NetworkOccupant* n = Network(occupant);
    if (n == nullptr) return 0;
    const uint32_t f = n->GetNetworkFlag();
    Drop(n);
    return f;
}

// Network occupant slot 8: every bit of `mask` set (§8.4.1).
bool NetworkHasAllFlags(void* u, uint64_t occupant, uint32_t mask) {
    ZD_TEST(network_has_all_flags, occupant, mask);
    if (!Gate(u, fc4::Sw::ZoneDevOccupants, "zonedev HasNetworkFlag (false)")) return false;
    cISC4NetworkOccupant* n = Network(occupant);
    if (n == nullptr) return false;
    const bool r = n->HasNetworkFlag(mask);
    Drop(n);
    return r;
}

// Network occupant slot 38 GetEdgeTypesFlag(type).
uint32_t NetworkEdgeMask(void* u, uint64_t occupant, uint32_t netType) {
    ZD_TEST(network_edge_mask, occupant, netType);
    if (!Gate(u, fc4::Sw::ZoneDevOccupants, "zonedev GetEdgeTypesFlag (0)")) return 0;
    cISC4NetworkOccupant* n = Network(occupant);
    if (n == nullptr) return 0;
    const uint32_t m = n->GetEdgeTypesFlag(static_cast<cISC4NetworkOccupant::eNetworkType>(netType));
    Drop(n);
    return m;
}

// Network occupant slot 22 IsOfType(type): 1 / 0, -1 when not a network.
int32_t NetworkIsOfType(void* u, uint64_t occupant, uint32_t netType) {
    ZD_TEST(network_is_of_type, occupant, netType);
    if (!Gate(u, fc4::Sw::ZoneDevOccupants, "zonedev IsOfType (not a network)")) return -1;
    cISC4NetworkOccupant* n = Network(occupant);
    if (n == nullptr) return -1;
    const bool r = n->IsOfType(static_cast<cISC4NetworkOccupant::eNetworkType>(netType));
    Drop(n);
    return r ? 1 : 0;
}

// Network occupant slot 17 (the flag 0x400000 test; the SDK's IsImmovable).
bool NetworkIsFlag400000(void* u, uint64_t occupant) {
    ZD_TEST(network_is_flag_0x400000, occupant);
    if (!Gate(u, fc4::Sw::ZoneDevOccupants, "zonedev slot 17 (false)")) return false;
    cISC4NetworkOccupant* n = Network(occupant);
    if (n == nullptr) return false;
    const bool r = n->IsImmovable();
    Drop(n);
    return r;
}

// Occupant slot 11 GetBoundingCityCells.
void OccupantBoundingCells(void* u, uint64_t occupant, fc4_cell_rect* out) {
    ZD_TEST(occupant_bounding_cells, occupant, out);
    if (out == nullptr) return;
    *out = fc4_cell_rect{0, 0, -1, -1};
    if (occupant == 0 || !Gate(u, fc4::Sw::ZoneDevOccupants, "zonedev GetBoundingCityCells (empty)")) return;
    SC4Rect<long> r;
    if (OccOf(occupant)->GetBoundingCityCells(r)) {
        *out = fc4_cell_rect{static_cast<int32_t>(r.topLeftX), static_cast<int32_t>(r.topLeftY),
                             static_cast<int32_t>(r.bottomRightX), static_cast<int32_t>(r.bottomRightY)};
    }
}

// Whether the network tool of `kind` (0, 3 or 11) is held (PostCityInit).
bool NetworkToolPresent(void* u, uint32_t kind) {
    ZD_TEST(network_tool_present, kind);
    return ToolOf(u, kind) != nullptr;
}

// §5.2.1 / §5.2.2: tool `kind`'s slot 7 PlaceNetworkOccupants(a, b, 0, null,
// 1, -1); AL not used.
void NetworkToolJoin(void* u, uint32_t kind, int32_t ax, int32_t az, int32_t bx, int32_t bz) {
    ZD_TEST(network_tool_join, kind, ax, az, bx, bz);
    if (!Writes(u) || !Gate(u, fc4::Sw::ZoneDevStreets, "zonedev street join")) return;
    cFC4ISC4NetworkTool* t = ToolOf(u, kind);
    if (t == nullptr) return;
    ++Ctx(u)->joins;
    char line[160];
    _snprintf_s(line, sizeof(line), _TRUNCATE, "zonedev: join with tool %u (%d,%d)-(%d,%d)", kind, ax, az, bx, bz);
    NoteFew(&g_noteJoin, line);
    t->ToolSlot07(static_cast<uint32_t>(ax), static_cast<uint32_t>(az), static_cast<uint32_t>(bx),
                  static_cast<uint32_t>(bz), 0u, 0u, 1u, 0xFFFFFFFFu);
    FC4_X87_CHECK("zonedev join PlaceNetworkOccupants");
}

// ---- terrain ------------------------------------------------------------------------------------------

bool CellHasWater(void* u, int32_t x, int32_t z) {
    ZD_TEST(cell_has_water, x, z);
    cISTETerrain* t = Ctx(u)->terrain;
    if (t == nullptr && Ctx(u)->readOnly && City(u) != nullptr) t = City(u)->GetTerrain();
    if (t == nullptr || x < 0 || z < 0 || !Gate(u, fc4::Sw::ZoneDevTerrain, "zonedev CellHasWater (dry)")) {
        return false;
    }
    return t->CellHasWater(static_cast<uint32_t>(x), static_cast<uint32_t>(z));  // slot 34
}

cISTETerrain* Terrain(void* u) {
    cISTETerrain* t = Ctx(u)->terrain;
    if (t == nullptr && Ctx(u)->readOnly && City(u) != nullptr) t = City(u)->GetTerrain();
    return t;
}

float TerrainAltitude(void* u, float x, float z) {
    ZD_TEST(terrain_altitude, x, z);
    cISTETerrain* t = Terrain(u);
    if (t == nullptr || !Gate(u, fc4::Sw::ZoneDevTerrain, "zonedev GetAltitude (0)")) return 0.0f;
    return t->GetAltitude(x, z);  // compiled slot 13 (foundations §16), ST0
}

float TerrainVertexAltitude(void* u, int32_t vx, int32_t vz) {
    ZD_TEST(terrain_vertex_altitude, vx, vz);
    cISTETerrain* t = Terrain(u);
    if (t == nullptr || !Gate(u, fc4::Sw::ZoneDevTerrain, "zonedev GetAltitudeAtVertex (0)")) return 0.0f;
    return t->GetAltitudeAtVertex(vx, vz);  // slot 14, ST0
}

// Terrain slot 35 CellHasLand. The core asks only for cells CellHasWater
// answered true for (§8.4.2); a negative cell answers land.
bool CellHasLand(void* u, int32_t x, int32_t z) {
    ZD_TEST(cell_has_land, x, z);
    cISTETerrain* t = Terrain(u);
    if (t == nullptr || x < 0 || z < 0 || !Gate(u, fc4::Sw::ZoneDevTerrain, "zonedev CellHasLand (land)")) {
        return true;
    }
    return t->CellHasLand(static_cast<uint32_t>(x), static_cast<uint32_t>(z));
}

// Surface water slot 8 IsNavigable(x, z, 0, null) (city slot 83, borrowed):
// called through the vtable (the SDK's overload order differs, as in
// cFC4TrafficSimulator.cpp). AL.
bool IsNavigable(void* u, int32_t x, int32_t z) {
    ZD_TEST(is_navigable, x, z);
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(u, fc4::Sw::ZoneDevTerrain, "zonedev IsNavigable (false)")) return false;
    void* sw = static_cast<void*>(c->GetSurfaceWater());
    if (sw == nullptr) return false;
    using Fn = uint8_t(__fastcall*)(void* self, void* edx, int32_t cx, int32_t cz, int32_t radius, void* lot);
    void** vt = *reinterpret_cast<void***>(sw);
    return reinterpret_cast<Fn>(vt[8])(sw, nullptr, x, z, 0, nullptr) != 0;  // pop 16
}

// ---- city and occupants -----------------------------------------------------------------------------------

// City slot 98 LocationIsInBounds(x, z) (AL).
bool LocationInBounds(void* u, const float* pos) {
    ZD_TEST(location_in_bounds, pos);
    cISC4City* c = City(u);
    return c != nullptr && pos != nullptr && c->LocationIsInBounds(pos[0], pos[2]);
}

// City slot 95 PositionToCell(x, z, &cx, &cz): AL per zones.md §12 (the SDK
// declares it void). [abi] Called through the vtable to read AL: __thiscall
// through __fastcall (ECX = this, EDX unused, the four arguments on the stack,
// callee pops 16).
// City slot 99 CellIsInBounds.
bool CellIsInBounds(void* u, int32_t x, int32_t z) {
    ZD_TEST(cell_is_in_bounds, x, z);
    cISC4City* c = City(u);
    return c != nullptr && c->CellIsInBounds(x, z);
}

bool PositionToCell(void* u, const float* pos, int32_t* x, int32_t* z) {
    ZD_TEST(position_to_cell, pos, x, z);
    cISC4City* c = City(u);
    if (c == nullptr || pos == nullptr || x == nullptr || z == nullptr) return false;
    using Fn = uint8_t(__fastcall*)(void* self, void* edx, float px, float pz, int32_t* cx, int32_t* cz);
    void** vt = *reinterpret_cast<void***>(c);
    const uint8_t al = reinterpret_cast<Fn>(vt[95])(c, nullptr, pos[0], pos[2], x, z);
    return al != 0;
}

// Occupant manager slot 41 GetOccupantsByStandardCityCells(list, &cell,
// &cell, type, -1) into an SC4List (A, one reference each; slot 48's release
// is the list's destructor), kept until the outermost call returns.
// [question] (zones.md §7.3 "all types") The argument the original passes for
// every type is not given: unfiltered, the filter overload (slot 40) with a
// null filter.
void CellOccupants(void* u, int32_t x, int32_t z, bool filtered, uint32_t type, fc4_token_callback cb, void* ctx) {
    ZD_TEST(cell_occupants, x, z, filtered, type, cb, ctx);
    fc4::ZoneDevContext* c = Ctx(u);
    cISC4OccupantManager* om = c->occupants;
    if (om == nullptr && c->readOnly && c->city != nullptr) om = c->city->GetOccupantManager();
    if (om == nullptr || cb == nullptr || !Gate(u, fc4::Sw::ZoneDevOccupants, "zonedev cell occupants (none)")) {
        return;
    }
    const SC4Point<int> cell(x, z);
    auto list = std::make_shared<SC4List<cISC4Occupant*>>();
    if (filtered) {
        om->GetOccupantsByStandardCityCells(*list, &cell, &cell, type, 0xFFFFFFFFu);  // slot 41
    } else {
        om->GetOccupantsByStandardCityCells(*list, &cell, &cell, static_cast<cISC4OccupantFilter*>(nullptr),
                                            0xFFFFFFFFu);  // slot 40
    }
    for (cISC4Occupant* o : *list) {
        if (o != nullptr) cb(ctx, TokenOf(o));
    }
    c->heldLists.push_back(std::move(list));
}

uint32_t OccupantType(void* u, uint64_t occupant) {
    ZD_TEST(occupant_type, occupant);
    if (occupant == 0 || !Gate(u, fc4::Sw::ZoneDevOccupants, "zonedev occupant GetType (0)")) return 0;
    return static_cast<uint32_t>(OccOf(occupant)->GetType());  // slot 7
}

void OccupantPosition(void* u, uint64_t occupant, float* pos) {
    ZD_TEST(occupant_position, occupant, pos);
    if (pos == nullptr) return;
    pos[0] = pos[1] = pos[2] = 0.0f;
    if (occupant == 0 || !Gate(u, fc4::Sw::ZoneDevOccupants, "zonedev occupant GetPosition (0)")) return;
    cS3DVector3 v;
    OccOf(occupant)->GetPosition(&v);  // slot 8
    pos[0] = v.fX;
    pos[1] = v.fY;
    pos[2] = v.fZ;
}

int32_t NetworkHasAnyFlag(void* u, uint64_t occupant, uint32_t flags) {
    ZD_TEST(network_has_any_flag, occupant, flags);
    if (!Gate(u, fc4::Sw::ZoneDevOccupants, "zonedev HasAnyNetworkFlag (not a network)")) return -1;
    cISC4NetworkOccupant* n = Query<cISC4NetworkOccupant>(FromToken(occupant), kIID_cISC4NetworkOccupant);
    if (n == nullptr) return -1;
    const bool any = n->HasAnyNetworkFlag(flags);  // slot 9
    Drop(n);
    return any ? 1 : 0;
}

// Occupant manager slot 21 RemoveOccupant(occupant, 1, 0) (§8.6 step 2).
void RemoveOccupant(void* u, uint64_t occupant) {
    ZD_TEST(remove_occupant, occupant);
    fc4::ZoneDevContext* c = Ctx(u);
    if (!Writes(u) || occupant == 0 || c->occupants == nullptr) return;
    if (!Gate(u, fc4::Sw::ZoneDevOccupants, "zonedev RemoveOccupant")) return;
    ++c->removedOccupants;
    c->occupants->RemoveOccupant(OccOf(occupant), true, 0);
}

// ---- references ---------------------------------------------------------------------------------------------

void AddRefToken(void* u, uint64_t t) {
    ZD_TEST(add_ref, t);
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return;
    fc4::ZoneDevContext* c = Ctx(u);
    if (c->freshLot != nullptr && static_cast<void*>(o) == c->freshLot) {
        // §8.5 step 2: a lot NewLot just built (count 0) gets the caller's
        // reference, then the manager's. The first is taken before the
        // audit's once-per-class lookup, whose QueryInterface / Release pair
        // would free a lot at count 0 (§8.5.1 Free). The caller's reference
        // is handed on in slots 22 / 23 (forCaller); the manager's is ours.
        const uint32_t n = ++c->freshAdds;
        if (n >= 2) c->freshLot = nullptr;
        if (n == 1) {
            const uint32_t r = o->AddRef();
            const fc4::RefTarget audit = fc4::RefBefore(o);
            fc4::RefNote(audit, +1, static_cast<long>(r), c->forCaller ? "AddRef for the caller (new lot)" : "AddRef (new lot)",
                         FC4_SITE, !c->forCaller);
        } else {
            FC4_ADDREF(o);
        }
        return;
    }
    if (fc4::IsOurObject(o)) {
        o->AddRef();
    } else if (Ctx(u)->forCaller) {
        FC4_ADDREF_FOR_CALLER(o);  // slots 7-10: the caller releases it
    } else {
        FC4_ADDREF(o);
    }
}

// [choice] (reference audit) The lot manager owns its lots: the reference
// it gives back at DeleteLot (§8.6 step 6), including the DeleteLot that
// Shutdown runs for every lot of the cell map (§1.2, ZG32), is the last one, where the original frees the lot too; so its releases are
// audited as releases at the owner's destruction (no warning when the audited
// sites acquired the reference: a Read, or the lot creation).
void ReleaseToken(void* u, uint64_t t) {
    ZD_TEST(release, t);
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->Release();
    } else {
        FC4_RELEASE_AT_DESTRUCTION(o);
    }
}

// ---- lots (cISC4Lot, §12) ------------------------------------------------------------------------------------

bool LotWrites(void* u, const char* what) { return Writes(u) && Gate(u, fc4::Sw::ZoneDevLotChanges, what); }

// §8.5 step 1, §8.5.1: a lot block from the game's lot pool (0x006BF430,
// cdecl, the size pushed and unread) built by the game's constructor
// (0x006C1E50, thiscall (x, z, w, h, facing), pop 20, EAX = this; called
// through __fastcall with EDX unused): docs/abi/direct-calls.md. Its
// reference count is 0; the core AddRefs it twice (AddRefToken). The block
// address is the cISC4Lot* (vtable at +0). [choice] (§8.5.1) The vacate
// reason at +0x83, which the constructor leaves as the block held it, is
// written 0. Without the two addresses (the executable not identified) or
// with zonedev_lot_changes off no lot is built and CreateLot fails cleanly.
using LotPoolAllocateFn = void*(__cdecl*)(uint32_t size);
using LotConstructFn = void*(__fastcall*)(void* self, void* edx, int32_t x, int32_t z, int32_t w, int32_t h,
                                          int32_t facing);
constexpr uint32_t kLotBlockSize = 0x8C;
constexpr uint32_t kLotVacateReason = 0x83;

uint64_t NewLot(void* u, int32_t x, int32_t z, int32_t w, int32_t h, int32_t facing) {
    ZD_TEST(new_lot, x, z, w, h, facing);
    fc4::ZoneDevContext* c = Ctx(u);
    const char* why = nullptr;
    uintptr_t allocate = 0, construct = 0;
    if (!LotWrites(u, "zonedev CreateLot (no lot)")) {
        why = "zonedev_lot_changes is off";
    } else {
        allocate = fc4::compat::DirectCall(fc4::compat::Direct::LotPoolAllocate);
        construct = fc4::compat::DirectCall(fc4::compat::Direct::LotConstruct);
        if (allocate == 0 || construct == 0) why = "the lot pool / constructor is not available";
    }
    void* block = why == nullptr ? reinterpret_cast<LotPoolAllocateFn>(allocate)(kLotBlockSize) : nullptr;
    if (why == nullptr && block == nullptr) why = "the lot pool returned null";
    if (why != nullptr) {
        ++c->newLotRefusals;
        char line[200];
        _snprintf_s(line, sizeof(line), _TRUNCATE, "zonedev: CreateLot(%d, %d, %d, %d, facing %d) refused: %s", x, z,
                    w, h, facing, why);
        NoteFew(&g_noteNewLot, line);
        return 0;
    }
    void* lot = reinterpret_cast<LotConstructFn>(construct)(block, nullptr, x, z, w, h, facing);
    FC4_X87_CHECK("zonedev lot constructor");
    if (lot == nullptr) lot = block;
    static_cast<uint8_t*>(lot)[kLotVacateReason] = 0;
    ++c->lotsBuilt;
    c->freshLot = lot;
    c->freshAdds = 0;
    return TokenOf(lot);
}

}  // namespace

bool fc4::ZoneDevLotsConstructible(const ZoneDevContext* ctx) {
    if (ctx == nullptr) return false;
    if (ctx->test != nullptr) return ctx->test->new_lot != nullptr;
    if (ctx->readOnly) return true;  // the shadow builds nothing and must not filter
    if (ctx->gated && !fc4::On(fc4::Sw::ZoneDevLotChanges)) return false;
    return fc4::compat::DirectCall(fc4::compat::Direct::LotPoolAllocate) != 0 &&
           fc4::compat::DirectCall(fc4::compat::Direct::LotConstruct) != 0;
}

namespace {

// Lot slot 32 UpdateZoneType (§7.2.2).
void LotUpdateZoneType(void* u, uint64_t lot) {
    ZD_TEST(lot_update_zone_type, lot);
    if (lot == 0 || !LotWrites(u, "zonedev lot UpdateZoneType")) return;
    ++Ctx(u)->zoneUpdates;
    LotOf(lot)->UpdateZoneType();
}

void LotInit(void* u, uint64_t lot) {
    ZD_TEST(lot_init, lot);
    if (lot != 0 && LotWrites(u, "zonedev lot Init")) LotOf(lot)->Init();  // slot 5
}
void LotShutdown(void* u, uint64_t lot) {
    ZD_TEST(lot_shutdown, lot);
    if (lot != 0 && LotWrites(u, "zonedev lot Shutdown")) LotOf(lot)->Shutdown();  // slot 7
}
void LotPostCityInit(void* u, uint64_t lot) {
    ZD_TEST(lot_post_city_init, lot);
    if (lot != 0 && LotWrites(u, "zonedev lot PostCityInit")) LotOf(lot)->PostCityInit();  // slot 8
}
uint64_t LotConfiguration(void* u, uint64_t lot) {
    ZD_TEST(lot_configuration, lot);
    return lot ? TokenOf(LotOf(lot)->GetLotConfiguration()) : 0;  // slot 9, borrowed
}
uint32_t LotFlags(void* u, uint64_t lot) {
    ZD_TEST(lot_flags, lot);
    return lot ? LotOf(lot)->GetFlags() : 0;  // slot 12
}
void LotBoundingRect(void* u, uint64_t lot, fc4_cell_rect* out) {
    ZD_TEST(lot_bounding_rect, lot, out);
    if (out == nullptr) return;
    *out = fc4_cell_rect{0, 0, -1, -1};
    if (lot == 0) return;
    SC4Rect<int32_t> r;
    // zones §8.3 / lot-records §2: slot 14 supplies the inclusive rectangle.
    // SPEC-GAP: slot 14 has no specified status result; consume its output unconditionally.
    LotOf(lot)->GetBoundingRect(r);
    *out = fc4_cell_rect{r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY};
}
float LotAltitude(void* u, uint64_t lot) {
    ZD_TEST(lot_altitude, lot);
    return lot ? LotOf(lot)->GetAltitude() : 0.0f;  // slot 16, ST0
}
void LotLocation(void* u, uint64_t lot, int32_t* x, int32_t* z) {
    ZD_TEST(lot_location, lot, x, z);
    int32_t lx = 0, lz = 0;
    if (lot != 0) LotOf(lot)->GetLocation(lx, lz);  // slot 21
    if (x != nullptr) *x = lx;
    if (z != nullptr) *z = lz;
}
int32_t LotFacing(void* u, uint64_t lot) {
    ZD_TEST(lot_facing, lot);
    return lot ? LotOf(lot)->GetFacing() : 0;  // slot 23
}
bool LotHasBusinessUnit(void* u, uint64_t lot) {
    ZD_TEST(lot_has_business_unit, lot);
    return lot != 0 && LotOf(lot)->GetUnit() != nullptr;  // slot 25
}
int32_t LotZoneType(void* u, uint64_t lot) {
    ZD_TEST(lot_zone_type, lot);
    return lot ? static_cast<int32_t>(LotOf(lot)->GetZoneType()) : 0;  // slot 31
}
uint32_t LotState(void* u, uint64_t lot) {
    ZD_TEST(lot_state, lot);
    return lot ? static_cast<uint32_t>(LotOf(lot)->GetState()) : 0;  // slot 33
}
void LotSetState(void* u, uint64_t lot, uint32_t state, uint32_t a, uint32_t b) {
    ZD_TEST(lot_set_state, lot, state, a, b);
    if (lot == 0 || !LotWrites(u, "zonedev lot SetState")) return;
    LotOf(lot)->SetState(static_cast<cFC4ISC4Lot::HabitationState>(state), static_cast<cFC4ISC4Lot::WealthType>(a),
                         static_cast<cFC4ISC4Lot::VacateReason>(b));  // slot 34
}
void LotSetVerticesLocked(void* u, uint64_t lot, bool locked) {
    ZD_TEST(lot_set_vertices_locked, lot, locked);
    if (lot != 0 && LotWrites(u, "zonedev lot SetVerticesAreLocked")) LotOf(lot)->SetVerticesAreLocked(locked);  // 69
}

// ---- lot configurations (SDK slots match the compiled ones) ---------------------------------------------------

bool CfgCompatibleWithWater(void* u, uint64_t cfg) {
    ZD_TEST(cfg_compatible_with_water, cfg);
    return cfg != 0 && CfgOf(cfg)->IsCompatibleWithWater();  // slot 14
}
bool CfgCompatibleWithZone(void* u, uint64_t cfg, int32_t zone) {
    ZD_TEST(cfg_compatible_with_zone, cfg, zone);
    return cfg != 0 && CfgOf(cfg)->IsCompatibleWithZoneType(static_cast<cISC4ZoneManager::ZoneType>(zone));  // 15
}
uint32_t CfgId(void* u, uint64_t cfg) {
    ZD_TEST(cfg_id, cfg);
    return cfg ? CfgOf(cfg)->GetID() : 0;  // slot 3
}
void CfgSize(void* u, uint64_t cfg, uint32_t* x, uint32_t* z) {
    ZD_TEST(cfg_size, cfg, x, z);
    uint32_t sx = 0, sz = 0;
    if (cfg != 0) CfgOf(cfg)->GetSize(sx, sz);  // slot 7
    if (x != nullptr) *x = sx;
    if (z != nullptr) *z = sz;
}
uint8_t CfgGrowthStage(void* u, uint64_t cfg) {
    ZD_TEST(cfg_growth_stage, cfg);
    return cfg ? CfgOf(cfg)->GetGrowthStage() : uint8_t{0};  // slot 11
}
bool CfgCompatibleWithWealth(void* u, uint64_t cfg, uint32_t wealth) {
    ZD_TEST(cfg_compatible_with_wealth, cfg, wealth);
    return cfg != 0 &&
           CfgOf(cfg)->IsCompatibleWithWealthType(static_cast<cISC4BuildingOccupant::WealthType>(wealth));  // 16
}
bool CfgCompatibleWithPurpose(void* u, uint64_t cfg, uint32_t purpose) {
    ZD_TEST(cfg_compatible_with_purpose, cfg, purpose);
    return cfg != 0 &&
           CfgOf(cfg)->IsCompatibleWithPurposeType(static_cast<cISC4BuildingOccupant::PurposeType>(purpose));  // 17
}
// Slot 24's object: its ID list in order; false for a null object.
bool CfgPrimaryBuildingIds(void* u, uint64_t cfg, fc4_u32_callback cb, void* ctx) {
    ZD_TEST(cfg_primary_building_ids, cfg, cb, ctx);
    if (cfg == 0) return false;
    cSC4LotConfigurationObject* obj = CfgOf(cfg)->GetPrimaryBuildingConfigurationObject();
    if (obj == nullptr) return false;
    if (cb != nullptr) {
        for (uint32_t id : obj->data) cb(ctx, id);
    }
    return true;
}

// Slot 8 GetMaximumSlope (ST0).
float CfgMaxSlope(void* u, uint64_t cfg) {
    ZD_TEST(cfg_max_slope, cfg);
    return cfg ? CfgOf(cfg)->GetMaximumSlope() : FLT_MAX;
}

// Slot 23 GetLotConfigurationObjectArray: each 0x34-byte record's type,
// rotation, footprint (+0x10 max, +0x18 min) and data vector, in order.
void CfgObjects(void* u, uint64_t cfg, fc4_zonedev_cfg_object_callback cb, void* ctx) {
    ZD_TEST(cfg_objects, cfg, cb, ctx);
    if (cfg == 0 || cb == nullptr) return;
    SC4Vector<cSC4LotConfigurationObject>* v = CfgOf(cfg)->GetLotConfigurationObjectArray();
    if (v == nullptr) return;
    std::vector<uint32_t> values;
    for (const cSC4LotConfigurationObject& o : *v) {
        values.assign(o.data.begin(), o.data.end());
        fc4_zonedev_cfg_object r{};
        r.type = static_cast<uint8_t>(o.type);
        r.orientation = static_cast<uint8_t>(o.rotation);
        r.min_x = o.rectMinX;
        r.min_z = o.rectMinZ;
        r.max_x = o.rectMaxX;
        r.max_z = o.rectMaxZ;
        r.values = values.empty() ? nullptr : values.data();
        r.n_values = values.size();
        cb(ctx, &r);
    }
}

// The lot configuration filter that accepts everything (§7.2.1 step 5: the
// core applies DetermineLotSize's own filter). Lives on the stack; the
// manager does not keep it.
class AcceptAllConfigurations final : public cISC4LotConfigurationFilter {
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

// The lot configuration manager's slot 9 GetLotConfigurationsByFilter with
// the accept-all filter, into an STLport hash_set (B, hint 100): every
// configuration with one reference, kept until the outermost call returns.
// [choice] Kept and given back without the reference audit's per-reference
// bookkeeping (thousands per parcellization, as the tract glue's); counted.
void LotConfigurations(void* u, fc4_token_callback cb, void* ctx) {
    ZD_TEST(lot_configurations, cb, ctx);
    fc4::ZoneDevContext* c = Ctx(u);
    if (cb == nullptr || c->city == nullptr) return;
    if (!Gate(u, fc4::Sw::ZoneDevGrowth, "zonedev GetLotConfigurationsByFilter (none)")) return;
    cISC4LotConfigurationManager* lcm = c->city->GetLotConfigurationManager();
    if (lcm == nullptr) return;
    if (!fc4::ZoneDevAllocators::NodesAvailable()) {
        NoteFew(&g_noteNoAllocator, "zonedev: the game's STL allocator is not available; no game container is made");
        return;
    }
    fc4::GameHashSetU32<fc4::ZoneDevAllocators::NodePolicy> set(kSetHint);
    if (set.bucket_count() == 0) return;
    AcceptAllConfigurations filter;
    lcm->GetLotConfigurationsByFilter(*reinterpret_cast<SC4HashSet<cISC4LotConfiguration*>*>(&set), &filter);
    std::vector<uint64_t> tokens;
    tokens.reserve(set.size());
    for (size_t i = 0; i < set.bucket_count(); ++i) {
        for (auto* n = set.bucket(i); n != nullptr; n = n->next) {
            auto* cfg = reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(n->value));
            if (cfg == nullptr) continue;
            c->heldConfigs.push_back(cfg);
            ++c->configRefs;
            tokens.push_back(TokenOf(cfg));
        }
    }
    for (uint64_t t : tokens) cb(ctx, t);
}

// ---- lot developer (SDK slots match the compiled ones) --------------------------------------------------------

// Slot 21 AttemptAreaLevelling(&rect, 0.0f, 1, 1, FLT_MAX, null) (§12).
void AttemptAreaLevelling(void* u, const fc4_cell_rect* rect) {
    ZD_TEST(attempt_area_levelling, rect);
    if (!Writes(u) || rect == nullptr || !Gate(u, fc4::Sw::ZoneDevLevelling, "zonedev AttemptAreaLevelling")) return;
    cISC4LotDeveloper* ld = LotDeveloper(u);
    if (ld == nullptr) return;
    ++Ctx(u)->levellings;
    ld->AttemptAreaLevelling(SC4Rect<int32_t>(rect->x1, rect->z1, rect->x2, rect->z2), 0.0f, true, true, FLT_MAX,
                             nullptr);
    FC4_X87_CHECK("zonedev AttemptAreaLevelling");
}

void RemoveBuildingZotProp(void* u, uint64_t lot) {
    ZD_TEST(remove_building_zot_prop, lot);
    if (!Writes(u) || lot == 0 || !Gate(u, fc4::Sw::ZoneDevLotDeveloper, "zonedev RemoveBuildingZotProp")) return;
    cISC4LotDeveloper* ld = LotDeveloper(u);
    if (ld != nullptr) ld->RemoveBuildingZotProp(SdkLotOf(lot));  // slot 11
}

// Slot 41 GetLotBaseTextureOccupant(lot, &occ, false): the occupant (its
// cISC4Occupant, AsOccupant) with the ONE reference the call gave, for the
// core's RemoveOccupant and release (§8.6 step 2).
uint64_t LotBaseTextureOccupant(void* u, uint64_t lot) {
    ZD_TEST(lot_base_texture_occupant, lot);
    if (!Writes(u) || lot == 0 || !Gate(u, fc4::Sw::ZoneDevLotDeveloper, "zonedev GetLotBaseTextureOccupant")) {
        return 0;
    }
    cISC4LotDeveloper* ld = LotDeveloper(u);
    if (ld == nullptr) return 0;
    cISC4LotBaseTextureOccupant* t = nullptr;
    if (!ld->GetLotBaseTextureOccupant(SdkLotOf(lot), t, false) || t == nullptr) return 0;
    cISC4Occupant* o = t->AsOccupant();
    if (o == nullptr) {
        FC4_ACQUIRED(t, "GetLotBaseTextureOccupant");
        FC4_RELEASE(t);
        return 0;
    }
    FC4_ACQUIRED(o, "GetLotBaseTextureOccupant (its occupant)");
    return TokenOf(o);
}

void RemoveLotFromBusinessUnit(void* u, uint64_t lot) {
    ZD_TEST(remove_lot_from_business_unit, lot);
    if (!Writes(u) || lot == 0) return;
    if (!Gate(u, fc4::Sw::ZoneDevLotDeveloper, "zonedev RemoveLotFromItsBusinessUnit")) return;
    cISC4LotDeveloper* ld = LotDeveloper(u);
    if (ld != nullptr) ld->RemoveLotFromItsBusinessUnit(SdkLotOf(lot));  // slot 33
}

// Slot 29 GetAreaSlope(rect, withWater) (ST0).
float AreaSlope(void* u, const fc4_cell_rect* rect, bool withWater) {
    ZD_TEST(area_slope, rect, withWater);
    if (rect == nullptr || !Gate(u, fc4::Sw::ZoneDevLotDeveloper, "zonedev GetAreaSlope (0)")) return 0.0f;
    cISC4LotDeveloper* ld = LotDeveloper(u);
    if (ld == nullptr) return 0.0f;
    const float s = ld->GetAreaSlope(SC4Rect<int32_t>(rect->x1, rect->z1, rect->x2, rect->z2), withWater);
    FC4_X87_CHECK("zonedev GetAreaSlope");
    return s;
}

// Slot 46 GetForceEmptyLotBaseTextures.
bool ForceEmptyLotBaseTextures(void* u) {
    ZD_TEST0(force_empty_lot_base_textures);
    cISC4LotDeveloper* ld = LotDeveloper(u);
    return ld != nullptr && ld->GetForceEmptyLotBaseTextures();
}

// §7.2.2 step 3: the active zone manager's slot 11 PlaceZone(D, 0, commit 1,
// skipFunds 1, checkSlope 0, skipSize 1, lotInside 0, null, null, null) on a
// region of ours in the §11.4 layout. It dezones the cells, posts its own
// ZoningChange and calls back into DoParcellization (nested, under the lock).
void ZoneManagerDezone(void* u, const fc4_zone_region* region) {
    ZD_TEST(zone_manager_dezone, region);
    fc4::ZoneDevContext* c = Ctx(u);
    if (!Writes(u) || region == nullptr || c->zones == nullptr) return;
    if (!Gate(u, fc4::Sw::ZoneDevDemolition, "zonedev UpdateZoneType dezone")) return;
    fc4::ZoneRegionBuffer rb;
    if (!rb.Build(*region)) return;
    ++c->dezoneCalls;
    char line[160];
    _snprintf_s(line, sizeof(line), _TRUNCATE, "zonedev: UpdateZoneType dezones cells of %d,%d..%d,%d", region->x0,
                region->z0, region->x1, region->z1);
    NoteFew(&g_noteDezone, line);
    fc4::AsGame(c->zones)->PlaceZone(rb.Game(), 0, true, true, false, true, false, nullptr, nullptr, nullptr);
    FC4_X87_CHECK("zonedev UpdateZoneType PlaceZone");
}

void SetEmptyLotBaseTexture(void* u, uint64_t lot) {
    ZD_TEST(set_empty_lot_base_texture, lot);
    if (!Writes(u) || lot == 0 || !Gate(u, fc4::Sw::ZoneDevLotDeveloper, "zonedev SetEmptyLotBaseTexture")) return;
    cISC4LotDeveloper* ld = LotDeveloper(u);
    if (ld != nullptr) ld->SetEmptyLotBaseTexture(SdkLotOf(lot));  // slot 43
}

// ---- traffic simulator (§8.5 step 4) ------------------------------------------------------------------------

// Slot 22 GetDesiredLotInsertionPoint: a safe lot iterator block or null. A
// block of our own list gives its node's lot (0 at the end); a block whose
// list object is null gives "no list" (§8.5: nothing is inserted).
// [question] A block of another list (the original lot manager's) has no
// meaning for this list: treated as null (appended), logged.
bool DesiredLotInsertionPoint(void* u, uint64_t* node, bool* list) {
    ZD_TEST(desired_lot_insertion_point, node, list);
    fc4::ZoneDevContext* c = Ctx(u);
    c->insertion = nullptr;
    if (!Writes(u) || c->traffic == nullptr) return false;
    if (!Gate(u, fc4::Sw::ZoneDevTraffic, "zonedev GetDesiredLotInsertionPoint (null)")) return false;
    auto* block = static_cast<fc4::safelots::Block*>(reinterpret_cast<cFC4ISC4TrafficSimulator*>(c->traffic)->GetDesiredLotInsertionPoint());
    if (block == nullptr) return false;
    if (block->listObject == nullptr) {
        if (node != nullptr) *node = 0;
        if (list != nullptr) *list = false;
        return true;
    }
    if (c->master == nullptr || !c->master->Owns(block)) {
        NoteFew(&g_noteForeignIterator, "zonedev: the traffic simulator's insertion point is not an iterator of this "
                                        "lot list; the lot is appended (question)");
        return false;
    }
    c->insertion = block;
    if (node != nullptr) *node = c->master->LotAt(block->current);
    if (list != nullptr) *list = true;
    return true;
}

void SetLotInsertionPoint(void* u, uint64_t node) {
    ZD_TEST(set_lot_insertion_point, node);
    fc4::ZoneDevContext* c = Ctx(u);
    if (!Writes(u) || c->insertion == nullptr || c->master == nullptr) return;
    c->master->MoveTo(c->insertion, node);
}

// ---- BDS and growth developers (§7.2.1) ------------------------------------------------------------------------

uint64_t BdsDeveloper(void* u, uint32_t type) {
    ZD_TEST(bds_developer, type);
    if (!Gate(u, fc4::Sw::ZoneDevGrowth, "zonedev BDS GetDeveloper (none)")) return 0;
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    return b ? TokenOf(b->GetDeveloper(type)) : 0;  // slot 32, borrowed
}

// BDS slot 19 GetBuildingTypeSetByFamily(set, family) into an STLport
// hash_set (B, hint 100). The core takes the minimum and maximum over it, so
// the order does not matter.
void BuildingTypesByFamily(void* u, uint32_t family, fc4_u32_callback cb, void* ctx) {
    ZD_TEST(building_types_by_family, family, cb, ctx);
    if (cb == nullptr || !Gate(u, fc4::Sw::ZoneDevGrowth, "zonedev GetBuildingTypeSetByFamily (none)")) return;
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr) return;
    if (!fc4::ZoneDevAllocators::NodesAvailable()) {
        NoteFew(&g_noteNoAllocator, "zonedev: the game's STL allocator is not available; no game container is made");
        return;
    }
    fc4::GameHashSetU32<fc4::ZoneDevAllocators::NodePolicy> set(kSetHint);
    if (set.bucket_count() == 0) {
        NoteFew(&g_noteFamilySet, "zonedev: a building family set could not be made");
        return;
    }
    b->GetBuildingTypeSetByFamily(reinterpret_cast<fc4::GameHashSetU32<>&>(set), family);
    for (size_t i = 0; i < set.bucket_count(); ++i) {
        for (auto* n = set.bucket(i); n != nullptr; n = n->next) cb(ctx, n->value);
    }
}

// BDS slot 39 GetBuildingProfile(type) (a zero profile for an unknown type).
bool BuildingProfile(void* u, uint32_t type, fc4_zonedev_profile* out) {
    ZD_TEST(building_profile, type, out);
    if (out == nullptr) return false;
    *out = fc4_zonedev_profile{};
    if (!Gate(u, fc4::Sw::ZoneDevGrowth, "zonedev GetBuildingProfile (none)")) return false;
    cFC4ISC4BuildingDevelopmentSimulator* b = Bds(u);
    if (b == nullptr) return false;
    const FC4BuildingProfile& p = b->GetBuildingProfile(type);
    out->purpose = p.purpose;
    out->wealth = p.wealth;
    out->capacity = p.capacity;
    return true;
}

int16_t DevEstimateDesirability(void* u, uint64_t dev, int32_t x, int32_t z) {
    ZD_TEST(dev_estimate_desirability, dev, x, z);
    return dev ? DevOf(dev)->EstimateDesirability(x, z) : int16_t{0};  // slot 29
}

// Slot 18 GetSourceDemand, then cISC4Demand slot 12 (ST0).
bool DevSourceDemand(void* u, uint64_t dev, float* out) {
    ZD_TEST(dev_source_demand, dev, out);
    if (out != nullptr) *out = 0.0f;
    cISC4Demand* d = dev ? DevOf(dev)->GetSourceDemand() : nullptr;
    if (d == nullptr || out == nullptr) return false;
    *out = fc4::AsGame(d)->QueryActiveDemandValue();
    return true;
}

static_assert(sizeof(FC4GrowthItem) == sizeof(fc4_growth_item), "growth list entries: {x, z, stage}");

// Slot 10 GrowthAndDeclineListsAreValid, then slot 11's vector as it is.
bool DevGrowthList(void* u, uint64_t dev, fc4_growth_items_sink sink, void* ctx) {
    ZD_TEST(dev_growth_list, dev, sink, ctx);
    if (dev == 0) return false;
    cFC4ISC4GrowthDeveloper* d = DevOf(dev);
    if (!d->GrowthAndDeclineListsAreValid()) return false;
    fc4::GameVector<FC4GrowthItem>& g = d->GetGrowthList();
    if (sink != nullptr && g.size() != 0) sink(ctx, reinterpret_cast<const fc4_growth_item*>(g.begin()), g.size());
    return true;
}

cISC4SimGrid<int16_t>* Desirability(uint64_t dev) { return dev ? DevOf(dev)->GetDesirabilityMap() : nullptr; }

int16_t DevDesirabilityValue(void* u, uint64_t dev, int32_t tx, int32_t tz) {
    ZD_TEST(dev_desirability_value, dev, tx, tz);
    cISC4SimGrid<int16_t>* g = Desirability(dev);  // slot 13
    return g ? g->GetTractValue(tx, tz) : int16_t{0};
}
uint32_t DevDesirabilityShift(void* u, uint64_t dev) {
    ZD_TEST(dev_desirability_shift, dev);
    cISC4SimGrid<int16_t>* g = Desirability(dev);
    return g ? static_cast<uint32_t>(g->GetTractShift()) : 0;  // grid slot 11
}
uint32_t DevPurpose(void* u, uint64_t dev) {
    ZD_TEST(dev_purpose, dev);
    return dev ? DevOf(dev)->GetPurpose() : 0;  // slot 7
}
uint32_t DevWealth(void* u, uint64_t dev) {
    ZD_TEST(dev_wealth, dev);
    return dev ? DevOf(dev)->GetWealth() : 0;  // slot 8
}

// ---- the master lot list mirror (§8.1) ------------------------------------------------------------------------

void MasterInsert(void* u, uint64_t lot, uint64_t before) {
    ZD_TEST(master_insert, lot, before);
    fc4::ZoneDevContext* c = Ctx(u);
    if (!Writes(u) || c->master == nullptr || lot == 0) return;
    c->master->Insert(FromToken(lot), FromToken(before));
}

void MasterRemove(void* u, uint64_t lot) {
    ZD_TEST(master_remove, lot);
    fc4::ZoneDevContext* c = Ctx(u);
    if (!Writes(u) || c->master == nullptr || lot == 0) return;
    c->master->Remove(FromToken(lot));
}

void MasterReset(void* u, const uint64_t* lots, size_t n) {
    ZD_TEST(master_reset, lots, n);
    fc4::ZoneDevContext* c = Ctx(u);
    if (!Writes(u) || c->master == nullptr) return;
    c->master->Reset(lots, n);
}

}  // namespace

namespace fc4 {

// ---- allocators ------------------------------------------------------------------------------------------------

void ZoneDevSetTestAllocators(void* (*allocate)(uint32_t), void (*free)(void*)) {
    g_testAllocate = allocate;
    g_testFree = free;
}

bool ZoneDevAllocators::NodesAvailable() { return g_testAllocate.load() != nullptr || StlNodeAllocator::Available(); }

void* ZoneDevAllocators::AllocateNode(uint32_t size) {
    if (auto a = g_testAllocate.load()) return a(size);
    return StlNodeAllocator::Allocate(size);
}

void ZoneDevAllocators::FreeNode(void* p) {
    if (p == nullptr) return;
    if (auto f = g_testFree.load()) return f(p);
    StlNodeAllocator::Free(p);
}

bool ZoneDevAllocators::BlocksAvailable() {
    if (g_testAllocate.load() != nullptr) return true;
    return compat::DirectCall(compat::Direct::GlobalHeapAllocate) != 0 &&
           compat::DirectCall(compat::Direct::GlobalHeapFree) != 0;
}

void* ZoneDevAllocators::AllocateBlock(uint32_t size) {
    if (auto a = g_testAllocate.load()) return a(size);
    const uintptr_t va = compat::DirectCall(compat::Direct::GlobalHeapAllocate);
    return va != 0 ? reinterpret_cast<BlockAllocateFn>(va)(size) : nullptr;
}

void ZoneDevAllocators::FreeBlock(void* p) {
    if (p == nullptr) return;
    if (auto f = g_testFree.load()) return f(p);
    const uintptr_t va = compat::DirectCall(compat::Direct::GlobalHeapFree);
    if (va != 0) reinterpret_cast<BlockFreeFn>(va)(p);
}

// ---- the master lot list mirror ---------------------------------------------------------------------------------

namespace {
safelots::Node* NewNode(void* value) {
    auto* n = static_cast<safelots::Node*>(ZoneDevAllocators::AllocateNode(sizeof(safelots::Node)));
    if (n == nullptr) return nullptr;
    n->next = n;
    n->prev = n;
    n->value = value;
    return n;
}
void LinkBefore(safelots::Node* n, safelots::Node* at) {
    n->next = at;
    n->prev = at->prev;
    at->prev->next = n;
    at->prev = n;
}
void Unlink(safelots::Node* n) {
    n->prev->next = n->next;
    n->next->prev = n->prev;
}
}  // namespace

ZoneDevMasterList::~ZoneDevMasterList() {
    OrphanIterators();
    FreeMaster();
    ZoneDevAllocators::FreeNode(obj_.masterHead);
    ZoneDevAllocators::FreeNode(obj_.iteratorHead);
    obj_.masterHead = obj_.iteratorHead = nullptr;
}

bool ZoneDevMasterList::Ensure() {
    if (Valid()) return true;
    if (!ZoneDevAllocators::NodesAvailable()) {
        NoteFew(&g_noteNoAllocator, "zonedev: the game's STL allocator is not available; no game container is made");
        return false;
    }
    if (obj_.masterHead == nullptr) obj_.masterHead = NewNode(nullptr);
    if (obj_.iteratorHead == nullptr) obj_.iteratorHead = NewNode(nullptr);
    return Valid();
}

bool ZoneDevMasterList::Insert(void* lot, void* before) {
    if (lot == nullptr || !Ensure() || nodes_.count(lot) != 0) return false;
    safelots::Node* n = NewNode(lot);
    if (n == nullptr) return false;
    safelots::Node* at = obj_.masterHead;
    if (before != nullptr) {
        auto it = nodes_.find(before);
        if (it != nodes_.end()) at = it->second;
    }
    LinkBefore(n, at);
    nodes_[lot] = n;
    return true;
}

bool ZoneDevMasterList::Remove(void* lot) {
    auto it = nodes_.find(lot);
    if (it == nodes_.end()) return false;
    safelots::Node* n = it->second;
    // Registered iterators on this node move on (§8.6 step 4).
    for (safelots::Node* r = obj_.iteratorHead->next; r != obj_.iteratorHead; r = r->next) {
        auto* b = static_cast<safelots::Block*>(r->value);
        if (b != nullptr && b->current == n) b->current = n->next;
    }
    Unlink(n);
    ZoneDevAllocators::FreeNode(n);
    nodes_.erase(it);
    return true;
}

void ZoneDevMasterList::FreeMaster() {
    if (obj_.masterHead == nullptr) return;
    for (safelots::Node* n = obj_.masterHead->next; n != obj_.masterHead;) {
        safelots::Node* next = n->next;
        ZoneDevAllocators::FreeNode(n);
        n = next;
    }
    obj_.masterHead->next = obj_.masterHead->prev = obj_.masterHead;
    nodes_.clear();
}

void ZoneDevMasterList::Reset(const uint64_t* lots, size_t n) {
    if (!Ensure()) return;
    for (safelots::Node* r = obj_.iteratorHead->next; r != obj_.iteratorHead; r = r->next) {
        auto* b = static_cast<safelots::Block*>(r->value);
        if (b != nullptr) b->current = obj_.masterHead;
    }
    FreeMaster();
    for (size_t i = 0; lots != nullptr && i < n; ++i) Insert(FromToken(lots[i]), nullptr);
}

safelots::Block* ZoneDevMasterList::NewIterator() {
    if (!Ensure() || !ZoneDevAllocators::BlocksAvailable()) return nullptr;
    auto* b = static_cast<safelots::Block*>(ZoneDevAllocators::AllocateBlock(sizeof(safelots::Block)));
    if (b == nullptr) return nullptr;
    b->current = obj_.masterHead->next;
    b->listObject = &obj_;
    safelots::Node* r = NewNode(b);
    if (r == nullptr) {
        ZoneDevAllocators::FreeBlock(b);
        return nullptr;
    }
    LinkBefore(r, obj_.iteratorHead);
    return b;
}

uint32_t ZoneDevMasterList::RegisteredIterators() const {
    if (obj_.iteratorHead == nullptr) return 0;
    uint32_t n = 0;
    for (safelots::Node* r = obj_.iteratorHead->next; r != obj_.iteratorHead && n <= kMaxIterators; r = r->next) ++n;
    return n;
}

uint32_t ZoneDevMasterList::OrphanIterators() {
    if (obj_.iteratorHead == nullptr) return 0;
    uint32_t n = 0;
    for (safelots::Node* r = obj_.iteratorHead->next; r != obj_.iteratorHead;) {
        safelots::Node* next = r->next;
        auto* b = static_cast<safelots::Block*>(r->value);
        if (b != nullptr) {
            b->listObject = nullptr;
            b->current = nullptr;
        }
        ZoneDevAllocators::FreeNode(r);
        ++n;
        r = next;
    }
    obj_.iteratorHead->next = obj_.iteratorHead->prev = obj_.iteratorHead;
    return n;
}

uint64_t ZoneDevMasterList::LotAt(const safelots::Node* node) const {
    if (node == nullptr || node == obj_.masterHead) return 0;
    return TokenOf(node->value);
}

void ZoneDevMasterList::MoveTo(safelots::Block* block, uint64_t lot) {
    if (block == nullptr || !Owns(block)) return;
    if (lot == 0) {
        block->current = obj_.masterHead;
        return;
    }
    auto it = nodes_.find(FromToken(lot));
    block->current = it != nodes_.end() ? it->second : obj_.masterHead;
}

void ZoneDevMasterList::Lots(std::vector<void*>& out) const {
    out.clear();
    if (obj_.masterHead == nullptr) return;
    for (safelots::Node* n = obj_.masterHead->next; n != obj_.masterHead; n = n->next) out.push_back(n->value);
}

// ---- the host tables ----------------------------------------------------------------------------------------------

fc4_zonedev_host MakeZoneDevHost(ZoneDevContext* ctx) {
    fc4_zonedev_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.grids = MakeGridApi(&ctx->grid);
    h.acquire_services = &AcquireServices;
    h.release_services = &ReleaseServices;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.cell_counts = &CellCounts;
    h.acquire_network_tools = &AcquireNetworkTools;
    h.release_network_tools = &ReleaseNetworkTools;
    h.post_defer_occupant_operations = &PostDefer;
    h.zone_type = &ZoneType;
    h.demolish_region = &DemolishRegion;
    h.street_place = &StreetPlace;
    h.reset_network_tool = &ResetNetworkTool;
    h.connects_to_neighbor = &ConnectsToNeighbor;
    h.network_tool_occupant_exists = &NetworkToolOccupantExists;
    h.street_joining_helper = &StreetJoiningHelper;
    h.cell_has_water = &CellHasWater;
    h.terrain_altitude = &TerrainAltitude;
    h.terrain_vertex_altitude = &TerrainVertexAltitude;
    h.location_in_bounds = &LocationInBounds;
    h.position_to_cell = &PositionToCell;
    h.cell_occupants = &CellOccupants;
    h.occupant_type = &OccupantType;
    h.occupant_position = &OccupantPosition;
    h.network_has_any_flag = &NetworkHasAnyFlag;
    h.remove_occupant = &RemoveOccupant;
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.new_lot = &NewLot;
    h.lot_init = &LotInit;
    h.lot_shutdown = &LotShutdown;
    h.lot_post_city_init = &LotPostCityInit;
    h.lot_configuration = &LotConfiguration;
    h.lot_flags = &LotFlags;
    h.lot_bounding_rect = &LotBoundingRect;
    h.lot_altitude = &LotAltitude;
    h.lot_location = &LotLocation;
    h.lot_facing = &LotFacing;
    h.lot_has_business_unit = &LotHasBusinessUnit;
    h.lot_zone_type = &LotZoneType;
    h.lot_state = &LotState;
    h.lot_set_state = &LotSetState;
    h.lot_set_vertices_locked = &LotSetVerticesLocked;
    h.cfg_compatible_with_water = &CfgCompatibleWithWater;
    h.cfg_compatible_with_zone = &CfgCompatibleWithZone;
    h.cfg_id = &CfgId;
    h.cfg_size = &CfgSize;
    h.cfg_growth_stage = &CfgGrowthStage;
    h.cfg_compatible_with_wealth = &CfgCompatibleWithWealth;
    h.cfg_compatible_with_purpose = &CfgCompatibleWithPurpose;
    h.cfg_primary_building_ids = &CfgPrimaryBuildingIds;
    h.lot_configurations = &LotConfigurations;
    h.attempt_area_levelling = &AttemptAreaLevelling;
    h.remove_building_zot_prop = &RemoveBuildingZotProp;
    h.lot_base_texture_occupant = &LotBaseTextureOccupant;
    h.remove_lot_from_business_unit = &RemoveLotFromBusinessUnit;
    h.set_empty_lot_base_texture = &SetEmptyLotBaseTexture;
    h.desired_lot_insertion_point = &DesiredLotInsertionPoint;
    h.set_lot_insertion_point = &SetLotInsertionPoint;
    h.bds_developer = &BdsDeveloper;
    h.building_types_by_family = &BuildingTypesByFamily;
    h.building_profile = &BuildingProfile;
    h.dev_estimate_desirability = &DevEstimateDesirability;
    h.dev_source_demand = &DevSourceDemand;
    h.dev_growth_list = &DevGrowthList;
    h.dev_desirability_value = &DevDesirabilityValue;
    h.dev_desirability_shift = &DevDesirabilityShift;
    h.dev_purpose = &DevPurpose;
    h.dev_wealth = &DevWealth;
    h.master_insert = &MasterInsert;
    h.master_remove = &MasterRemove;
    h.master_reset = &MasterReset;
    // ABI 55 (re-specs11)
    h.area_slope = &AreaSlope;
    h.cfg_max_slope = &CfgMaxSlope;
    h.cfg_objects = &CfgObjects;
    h.network_flags = &NetworkFlags;
    h.network_has_all_flags = &NetworkHasAllFlags;
    h.network_edge_mask = &NetworkEdgeMask;
    h.network_is_of_type = &NetworkIsOfType;
    h.network_is_flag_0x400000 = &NetworkIsFlag400000;
    h.occupant_bounding_cells = &OccupantBoundingCells;
    h.network_tool_present = &NetworkToolPresent;
    h.network_tool_join = &NetworkToolJoin;
    h.cell_has_land = &CellHasLand;
    h.cell_is_in_bounds = &CellIsInBounds;
    h.is_navigable = &IsNavigable;
    h.lot_update_zone_type = &LotUpdateZoneType;
    h.force_empty_lot_base_textures = &ForceEmptyLotBaseTextures;
    h.zone_manager_dezone = &ZoneManagerDezone;
    return h;
}

fc4_zonedev_host MakeZoneDevReadOnlyHost(ZoneDevContext* ctx) {
    ctx->readOnly = true;
    ctx->gated = false;
    fc4_zonedev_host h = MakeZoneDevHost(ctx);
    h.grids.create = nullptr;
    h.grids.set = nullptr;
    h.grids.fill = nullptr;
    h.grids.shutdown = nullptr;
    h.grids.release = nullptr;
    h.acquire_services = nullptr;
    h.release_services = nullptr;
    h.subscribe = nullptr;
    h.unsubscribe = nullptr;
    h.acquire_network_tools = nullptr;
    h.release_network_tools = nullptr;
    h.post_defer_occupant_operations = nullptr;
    h.demolish_region = nullptr;
    h.street_place = nullptr;
    h.reset_network_tool = nullptr;
    h.street_joining_helper = nullptr;
    h.remove_occupant = nullptr;
    h.new_lot = nullptr;
    h.lot_init = nullptr;
    h.lot_shutdown = nullptr;
    h.lot_post_city_init = nullptr;
    h.lot_set_state = nullptr;
    h.lot_set_vertices_locked = nullptr;
    h.attempt_area_levelling = nullptr;
    h.remove_building_zot_prop = nullptr;
    h.lot_base_texture_occupant = nullptr;
    h.remove_lot_from_business_unit = nullptr;
    h.set_empty_lot_base_texture = nullptr;
    h.desired_lot_insertion_point = nullptr;
    h.set_lot_insertion_point = nullptr;
    h.master_insert = nullptr;
    h.master_remove = nullptr;
    h.master_reset = nullptr;
    h.network_tool_join = nullptr;
    h.lot_update_zone_type = nullptr;
    h.zone_manager_dezone = nullptr;
    return h;
}

void ZoneDevReleaseHeld(ZoneDevContext* ctx) {
    std::vector<cIGZUnknown*> held;
    held.swap(ctx->held);
    for (cIGZUnknown* o : held) FC4_RELEASE(o);
    std::vector<cIGZUnknown*> configs;
    configs.swap(ctx->heldConfigs);
    for (cIGZUnknown* c : configs) c->Release();  // the set's references (not audited one by one, see above)
    ctx->heldLists.clear();                        // SC4List releases its elements
}

void ZoneDevReleaseServices(ZoneDevContext* ctx) {
    for (cIGZUnknown*& t : ctx->tools) {
        if (t == nullptr) continue;
        reinterpret_cast<cFC4ISC4NetworkTool*>(t)->Shutdown();
        FC4_RELEASE(t);
        t = nullptr;
    }
    ReleaseSlot(ctx->msgServer);
    ReleaseSlot(ctx->zones);
    ReleaseSlot(ctx->traffic);
    ReleaseSlot(ctx->terrain);
    ReleaseSlot(ctx->occupants);
    ReleaseSlot(ctx->city);
    ReleaseSlot(ctx->bds);
    ReleaseSlot(ctx->lotDeveloper);
}

}  // namespace fc4
