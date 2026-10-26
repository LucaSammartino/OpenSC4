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

// The terrain replacement over fc4_terrain (docs/specs/terrain.md). See
// cFC4Terrain.h for the shape, TR18 and TR9.

#include "cFC4Terrain.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4LotDevInterfaces.h"  // animation-tick manager slots 5/6
#include "FC4RefAudit.h"
#include "cFC4Message2Standard.h"
#include "compat/FC4Compat.h"

#include "fc4.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZIStream.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZOStream.h"
#include "cIGZWinMgr.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cIGZCOM.h"
#include "cRZCOMDllDirector.h"
#include "SC4List.h"

#include <atomic>
#include <algorithm>
#include <cstring>
#include <mutex>
#include <vector>

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

// Game calls (docs/abi/interfaces.tsv; a bool is only reliable in AL, a float
// return is x87). Reads of a game return value below carry their citation:
//   cISC4City 33 GetLotManager, 34 GetOccupantManager, 40 GetTrafficNetwork,
//   83 GetSurfaceWater, 84 GetTerrain (all borrowed pointers in EAX, the
//   table's `eax-or-void` class);
//   cISC4City 93/94 CellCountX / CellCountZ [abi-ret]: the export folds those
//   two names onto lighting/draw-context entries, but terrain.md §2.1 is
//   explicit that the terrain's Init reads the cell counts from the city
//   there, so they are counts in EAX and not a bool;
//   the traffic network map 8 FindNetworkTile (a pointer), network occupant 12
//   GetNetworkFlag and 67 GetPassageCeiling(float&) (a bool per TR19);
//   occupant manager 41 GetOccupantsByStandardCityCells (an A list);
//   lot manager 5 GetLot(cellX, cellZ, bool) (a borrowed pointer).

namespace fc4 {
namespace {

// ---- messages (terrain.md §8, §12.1) -------------------------------------------------------------

constexpr uint32_t kMsgPostCityInit = 0x26D31EC1;
constexpr uint32_t kMsgPreCityShutdown = 0x26D31EC2;
constexpr uint32_t kMsgViewUpdate = 0xA6B79602;      // §2.3, TR9
constexpr uint32_t kMsgAnimationTick = 0x86AD10EE;  // §12.1
constexpr uint32_t kMsgViewWindow = 0x470BD214;     // §12.1
constexpr uint32_t kMsgReconcile = 0x2A612643;      // §12.1
constexpr uint32_t kMsgSimNewDay = 0x66956814;      // §10.3

// What the original's terrain subscribes to (§12.1), minus the render-only
// notifications and the script commands we do not implement.
constexpr uint32_t kSubscribed[] = {kMsgPostCityInit, kMsgPreCityShutdown, kMsgViewUpdate,
                                    kMsgAnimationTick, kMsgViewWindow,      kMsgReconcile,
                                    kMsgSimNewDay};

// Vertex flags (§7).
constexpr uint16_t kFlagEvenHidden = 0x0020;
constexpr uint16_t kFlagOddHidden = 0x0040;
constexpr uint16_t kFlagCell4 = 0x0200;
constexpr uint16_t kFlagFlipInternal = 0x1000;
constexpr uint16_t kFlagFlipExternal = 0x2000;

// The one fc4_terrain of the process (one city at a time). The surface water
// object resolves through it, so it does not matter whether the city creates
// the water through the class factory or takes the terrain's own.
std::atomic<fc4_terrain*> g_activeCore{nullptr};
std::atomic<uint32_t> g_cellsX{0};
std::atomic<uint32_t> g_cellsZ{0};
std::atomic<TerrainDrawVertex*> g_drawVertices{nullptr};

// ---- the host context -----------------------------------------------------------------------------

// TR18 (terrain.md §15.4) is enforced here as well as in the core. `inside` is
// non-zero while one of our entry points is inside fc4_terrain, so a host
// callback that reaches back into the same handle is refused and counted. Only
// touched under the object's critical section.
struct HostContext {
    cISC4City* city = nullptr;  // borrowed for one call
    cIGZUnknown* self = nullptr;  // the terrain, for the messages' IGZUnknown
    void (*syncDraw)(void*) = nullptr;
    void* drawOwner = nullptr;
    int inside = 0;
    std::atomic<uint32_t> guardRefused{0};
    std::atomic<uint32_t> posts{0};
    std::atomic<uint32_t> randomDraws{0};
    std::atomic<uint32_t> fireCalls{0};
    bool testMode = false;
};

// The single TR18 gate every host callback goes through.
bool HostMayAct(HostContext* c, const char* what) {
    if (c == nullptr) return false;
    if (c->inside > 0) {
        const uint32_t n = c->guardRefused.fetch_add(1, std::memory_order_relaxed) + 1;
        Log("terrain: TR18 refused host callback '%s' from inside the terrain handle (guard count %u)", what, n);
        return false;
    }
    return true;
}

uint32_t HostRandomUniform(void* u, uint32_t n) {
    auto* c = static_cast<HostContext*>(u);
    if (!HostMayAct(c, "random_uniform") || n == 0) return 0;
    c->randomDraws.fetch_add(1, std::memory_order_relaxed);
    // §9.1: the walk angle draws from the terrain's own generator. [choice] a
    // process counter stands in for it; only the random-walk brushes consume
    // it, and every shipped brush is oriented so the walk angle is unused
    // (quirk 12), so this cannot change a height.
    static std::atomic<uint32_t> s_draw{0};
    return s_draw.fetch_add(1, std::memory_order_relaxed) % n;
}

// Navigability §10.4 steps 4 and 5: read the game only, never this handle.
int32_t HostBridgeAt(void* u, int32_t x, int32_t z, float* deck) {
    auto* c = static_cast<HostContext*>(u);
    if (!HostMayAct(c, "bridge_at") || c->city == nullptr) return FC4_TERRAIN_BRIDGE_NONE;
    auto* map = reinterpret_cast<cFC4ISC4TrafficNetworkMap*>(c->city->GetTrafficNetwork());
    if (map == nullptr) return FC4_TERRAIN_BRIDGE_NONE;
    void* tile = map->FindNetworkTile(x, z, 0x1F4F, true);
    if (tile == nullptr) return FC4_TERRAIN_BRIDGE_NONE;
    auto* n = static_cast<cISC4NetworkOccupant*>(tile);
    if ((n->GetNetworkFlag() & 0x40000u) == 0) return FC4_TERRAIN_BRIDGE_NONE;
    float h = 0.0f;
    if (!n->GetPassageCeiling(h)) return FC4_TERRAIN_BRIDGE_UNKNOWN_DECK;  // TR19
    if (deck != nullptr) *deck = h;
    return FC4_TERRAIN_BRIDGE_DECK;
}

bool HostPowerPole(void* u, int32_t x, int32_t z) {
    auto* c = static_cast<HostContext*>(u);
    if (!HostMayAct(c, "has_power_pole") || c->city == nullptr) return false;
    cISC4OccupantManager* om = c->city->GetOccupantManager();
    if (om == nullptr) return false;
    SC4List<cISC4Occupant*> list;
    const SC4Point<int> cell(x, z);
    om->GetOccupantsByStandardCityCells(list, &cell, &cell, 0x2890D4DEu, 0xFFFFFFFFu);
    bool any = false;
    for (cISC4Occupant*& o : list) {
        if (o == nullptr) continue;
        FC4_ACQUIRED(o, "GetOccupantsByStandardCityCells list entry");
        any = true;
        FC4_RELEASE(o);
        o = nullptr;
    }
    return any;
}

// §10.4 step 3: the lot at the cell (its centre, y = the sea level). The core
// compares the returned token with the one the caller passed in, so both are
// the lot pointer's bits. The y of the query is not part of the comparison.
uint64_t HostLotAtCell(void* u, int32_t x, int32_t z) {
    auto* c = static_cast<HostContext*>(u);
    if (!HostMayAct(c, "lot_at_cell") || c->city == nullptr) return 0;
    cISC4LotManager* lm = c->city->GetLotManager();
    if (lm == nullptr) return 0;
    cISC4Lot* lot = lm->GetLot(x, z, false);  // borrowed
    return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(lot));
}

// The lava's flammability and fire calls (§11.4, §11.5) drive the game's fire
// manager and flammability grid and must not come back here (§15.4). No
// flammability host is wired yet: nothing ignites. [choice]
bool HostCellFlammable(void* u, int32_t x, int32_t z) {
    auto* c = static_cast<HostContext*>(u);
    (void)x;
    (void)z;
    return HostMayAct(c, "is_cell_flammable") ? false : false;
}

void HostIgniteCell(void* u, int32_t x, int32_t z) {
    auto* c = static_cast<HostContext*>(u);
    (void)x;
    (void)z;
    if (!HostMayAct(c, "ignite_cell")) return;
    c->fireCalls.fetch_add(1, std::memory_order_relaxed);
}

void HostQuickBurnCell(void* u, int32_t x, int32_t z) {
    auto* c = static_cast<HostContext*>(u);
    (void)x;
    (void)z;
    if (!HostMayAct(c, "quick_burn_cell")) return;
    c->fireCalls.fetch_add(1, std::memory_order_relaxed);
}

void HostSetCause(void* u, uint32_t cause) {
    auto* c = static_cast<HostContext*>(u);
    (void)cause;
    if (!HostMayAct(c, "set_cause")) return;
    c->fireCalls.fetch_add(1, std::memory_order_relaxed);
}

bool HostNextBurnCell(void* u, int32_t* x, int32_t* z) {
    auto* c = static_cast<HostContext*>(u);
    (void)x;
    (void)z;
    return HostMayAct(c, "next_burn_cell") ? false : false;  // exhausted
}

cFC4Message2Standard* MakeTerrainPost(const fc4_terrain_post& p, cIGZUnknown* sender) {
    auto* m = FC4MakeMessage(p.msg_type, static_cast<intptr_t>(p.data1), static_cast<intptr_t>(p.data2),
                           static_cast<intptr_t>(p.data3));
    if (m == nullptr) return nullptr;
    m->SetData4(static_cast<intptr_t>(p.data4));
    // terrain.md section 8.1 / 15.5.12.1: data1 is the packed X rectangle.
    // foundations.md section 5.1: SetVoid1 aliases data1; the retained sender
    // belongs in the separate IGZUnknown field, leaving all four words intact.
    if (sender != nullptr) m->SetIGZUnknown(sender);
    return m;
}

// §8.1: MessagePost(msg, false) with four data words. The core hands these
// over after each entry point released the handle, so this is never re-entrant
// and never runs under the terrain's lock.
void HostPost(void* u, const fc4_terrain_post* p) {
    auto* c = static_cast<HostContext*>(u);
    if (c == nullptr || p == nullptr) return;
    c->posts.fetch_add(1, std::memory_order_relaxed);
    if ((p->msg_type == 0x278E6003u || p->msg_type == 0x678E6E3Au) && c->syncDraw != nullptr) {
        c->syncDraw(c->drawOwner);
    }
    if (c->testMode) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* m = MakeTerrainPost(*p, c->self);
    if (m == nullptr) return;
    ms->MessagePost(m, false);
    m->Release();
}

fc4_terrain_host MakeHost(HostContext* c) {
    fc4_terrain_host h{};
    h.struct_size = sizeof(h);
    h.user = c;
    h.post = &HostPost;
    h.random_uniform = &HostRandomUniform;
    h.lot_at_cell = &HostLotAtCell;
    h.bridge_at = &HostBridgeAt;
    h.has_power_pole = &HostPowerPole;
    h.is_cell_flammable = &HostCellFlammable;
    h.ignite_cell = &HostIgniteCell;
    h.quick_burn_cell = &HostQuickBurnCell;
    h.set_cause = &HostSetCause;
    h.next_burn_cell = &HostNextBurnCell;
    return h;
}

// ---- the city (borrowed) --------------------------------------------------------------------------

cISC4City* CurrentCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    FC4_ACQUIRED(sc4, "QueryInterface");
    cISC4City* city = sc4->GetCity();  // borrowed
    FC4_RELEASE(sc4);
    return city;
}

uint16_t VertexFlags(uint32_t x, uint32_t z) {
    auto* vertices = g_drawVertices.load(std::memory_order_acquire);
    const uint32_t cx = g_cellsX.load(std::memory_order_acquire);
    const uint32_t cz = g_cellsZ.load(std::memory_order_acquire);
    if (vertices != nullptr && x <= cx && z <= cz) return vertices[x + z * (cx + 1)].flags;
    fc4_terrain* core = g_activeCore.load(std::memory_order_acquire);
    return core != nullptr ? fc4_terrain_vertex_flags(core, static_cast<int32_t>(x), static_cast<int32_t>(z))
                           : uint16_t{0};
}

void SetFlagBit(uint32_t x, uint32_t z, uint16_t bit, bool on) {
    fc4_terrain* core = g_activeCore.load(std::memory_order_acquire);
    if (core == nullptr) return;
    uint16_t f = VertexFlags(x, z);
    f = on ? static_cast<uint16_t>(f | bit) : static_cast<uint16_t>(f & ~bit);
    fc4_terrain_set_vertex_flags(core, static_cast<int32_t>(x), static_cast<int32_t>(z), f);
    auto* vertices = g_drawVertices.load(std::memory_order_acquire);
    const uint32_t cx = g_cellsX.load(std::memory_order_acquire);
    if (vertices != nullptr && x <= cx && z <= g_cellsZ.load(std::memory_order_acquire)) {
        vertices[x + z * (cx + 1)].flags = f;
    }
}

}  // namespace


// foundations §2.7: a `ref` is `uint32 objectID` and, only when that is non-zero,
// `uint32 classID`. The referenced object's own data is NOT here. Read consumes
// exactly those bytes and leaves the pointer null: §13.2's terrain class record
// is "u16 version 2 and a reference to the terrain view", and §13.3's water
// record is "u16 1 and a reference (the terrain)". Under-reading either one
// desynchronises the component stream and every later Read gets garbage.
bool ReadRef(cIGZIStream& in) {
    uint32_t objectId = 0;
    if (!in.GetUint32(objectId)) return false;
    if (objectId == 0) return true;
    uint32_t classId = 0;
    return in.GetUint32(classId);
}

bool WriteNullRef(cIGZOStream& out) { return out.SetUint32(0); }

// ---- cFC4STESurfaceWater (§10) --------------------------------------------------------------------

class cFC4STESurfaceWater final : public cFC4ISTESurfaceWater, public cIGZSerializable {
public:
    // terrain.md §2.2, §15.5.5: the native view reads water +0x18.
    uint32_t nativeReserved_[4]{};
    WaterDrawVertex* vertices_ = nullptr;

    cFC4STESurfaceWater() {
        std::lock_guard<std::mutex> guard(InstanceMutex());
        vertices_ = SharedVertices();
        Instances().push_back(this);
    }

    // The city may create a distinct water interface before terrain Init.
    // Its native view reads that object's +0x18, not necessarily terrain +0x24.
    static void PublishDrawVertices(WaterDrawVertex* vertices) {
        std::lock_guard<std::mutex> guard(InstanceMutex());
        SharedVertices() = vertices;
        for (auto* water : Instances()) water->vertices_ = vertices;
    }

    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        // The surface water's own IID is 0xAB8DA759 (observed: the city QIs
        // for it when it builds the object; not named in terrain.md or the SDK
        // header). terrain.md §1 also lists the generic 0x00000001 among the
        // terrain's IIDs, so it is answered here too.
        if (riid == GZIID_cIGZUnknown || riid == kFC4_IID_cISTESurfaceWater || riid == 0x00000001u ||
            riid == 0xAB953253u) {
            *ppvObj = Primary();
        } else if (riid == GZIID_cIGZSerializable) {
            *ppvObj = static_cast<cIGZSerializable*>(this);
        } else {
            return false;
        }
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refCount_; }
    uint32_t Release() override {
        if (refCount_ > 1) return --refCount_;
        refCount_ = 0;
        if (destroying_) return 0;
        destroying_ = true;
        delete this;
        return 0;
    }

    bool Init() override { return true; }
    bool Shutdown() override { return true; }

    float GetWaterDepthAtVertex(int32_t index) const override;                  // 5
    float GetWaterDepthForCell(int32_t x, int32_t z) const override;            // 6
    bool IsNavigableAtPosition(uint32_t, uint32_t, uint32_t, uint32_t, uint32_t) const override;  // 7
    bool IsNavigable(int32_t x, int32_t z, int32_t r, cISC4Lot* lot) const override;  // 8
    void* GetBodyOfWaterMap() const override;                                   // 9
    float AltitudeAtWhichFloodingWillOccur(int32_t x, int32_t z) override;      // 10
    void* GetWaterVelocity(int32_t x, int32_t z) override;                      // 11
    int32_t GetWaterType(int32_t x, int32_t z) override;                        // 12
    bool HasDraftClearance(int32_t x, int32_t z) override;                      // 13

    // §13.3: u16 1 and a reference to the terrain. The water array is rebuilt
    // from the sea level on every load (§10.1) and lakes are never saved
    // (quirk 9), so there is nothing else to write.
    // §13.3: u16 1 and a reference (read with IID 0xC70BD37E, the terrain).
    bool Write(cIGZOStream& out) override {
        Log("terrain: water Write (§13.3)");
        return out.SetUint16(1) && WriteNullRef(out);
    }
    bool Read(cIGZIStream& in) override {
        uint16_t version = 0;
        const bool ok = in.GetUint16(version) && ReadRef(in);
        Log("terrain: water Read (version + ref, §13.3) -> ok=%d version=%u", ok ? 1 : 0, version);
        return ok;
    }
    uint32_t GetGZCLSID() override { return kFC4_CLSID_cSTESurfaceWater; }

    cIGZUnknown* Primary() {
        return static_cast<cIGZUnknown*>(static_cast<cFC4ISTESurfaceWater*>(this));
    }

private:
    ~cFC4STESurfaceWater() {
        std::lock_guard<std::mutex> guard(InstanceMutex());
        auto& instances = Instances();
        instances.erase(std::find(instances.begin(), instances.end(), this));
    }

    static std::mutex& InstanceMutex() { static std::mutex mutex; return mutex; }
    static std::vector<cFC4STESurfaceWater*>& Instances() {
        static std::vector<cFC4STESurfaceWater*> instances;
        return instances;
    }
    static WaterDrawVertex*& SharedVertices() { static WaterDrawVertex* vertices = nullptr; return vertices; }

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    mutable std::vector<uint16_t> bodyMap_;              // slot 9's backing grid
    mutable float velocity_[3] = {0.0f, 0.0f, 0.0f};     // slot 11's shared vector
};
static_assert(offsetof(cFC4STESurfaceWater, vertices_) == 0x18, "native view water array");

float cFC4STESurfaceWater::GetWaterDepthAtVertex(int32_t index) const {
    fc4_terrain* core = g_activeCore.load(std::memory_order_acquire);
    return core != nullptr ? fc4_terrain_water_depth_at_index(core, static_cast<uint32_t>(index)) : 0.0f;
}

float cFC4STESurfaceWater::GetWaterDepthForCell(int32_t x, int32_t z) const {
    fc4_terrain* core = g_activeCore.load(std::memory_order_acquire);
    return core != nullptr ? fc4_terrain_water_depth_for_cell(core, x, z) : 0.0f;
}

bool cFC4STESurfaceWater::IsNavigableAtPosition(uint32_t, uint32_t, uint32_t, uint32_t,
                                                uint32_t) const {
    // §10.2: converts the position to a cell (city slot 95) and calls slot 8.
    // The five argument dwords are not spelled out in the spec and the plugin
    // never calls this slot (FC4Interfaces.h), so it is refused rather than
    // guessed. See the lane report's numbered question.
    return false;
}

bool cFC4STESurfaceWater::IsNavigable(int32_t x, int32_t z, int32_t r, cISC4Lot* lot) const {
    fc4_terrain* core = g_activeCore.load(std::memory_order_acquire);
    if (core == nullptr) return false;
    return fc4_terrain_is_navigable(core, x, z, r, static_cast<uint64_t>(reinterpret_cast<uintptr_t>(lot)));
}

void* cFC4STESurfaceWater::GetBodyOfWaterMap() const {
    fc4_terrain* core = g_activeCore.load(std::memory_order_acquire);
    if (core == nullptr) return nullptr;
    const size_t n = static_cast<size_t>(g_cellsX.load(std::memory_order_acquire)) *
                     static_cast<size_t>(g_cellsZ.load(std::memory_order_acquire));
    bodyMap_.assign(n == 0 ? 1 : n, 0);
    if (n != 0) fc4_terrain_bodies_map(core, bodyMap_.data(), n);
    return bodyMap_.data();
}

float cFC4STESurfaceWater::AltitudeAtWhichFloodingWillOccur(int32_t x, int32_t z) {
    fc4_terrain* core = g_activeCore.load(std::memory_order_acquire);
    return core != nullptr ? fc4_terrain_flooding_altitude(core, x, z) : 0.0f;
}

void* cFC4STESurfaceWater::GetWaterVelocity(int32_t x, int32_t z) {
    fc4_terrain* core = g_activeCore.load(std::memory_order_acquire);
    if (core == nullptr || !fc4_terrain_water_velocity(core, x, z, velocity_)) {
        // §10.5: out of the array, a shared zero vector.
        velocity_[0] = velocity_[1] = velocity_[2] = 0.0f;
    }
    return velocity_;
}

int32_t cFC4STESurfaceWater::GetWaterType(int32_t x, int32_t z) {
    fc4_terrain* core = g_activeCore.load(std::memory_order_acquire);
    return core != nullptr ? static_cast<int32_t>(fc4_terrain_water_type(core, x, z)) : 0;
}

bool cFC4STESurfaceWater::HasDraftClearance(int32_t x, int32_t z) {
    fc4_terrain* core = g_activeCore.load(std::memory_order_acquire);
    return core != nullptr ? fc4_terrain_has_draft_clearance(core, x, z) : false;
}

// ---- cFC4STETerrain (§2-§9, §15.1) ---------------------------------------------------------------

class cFC4STETerrain final : public cFC4ISTETerrain, public cIGZMessageTarget2, public cIGZSerializable {
public:
    // MUST be the first data member: the game reads the original's fields at
    // the offsets of terrain.md §2.3 (FC4TerrainInterfaces.h TerrainFields).
    TerrainFields f_{};

    cFC4STETerrain();
    ~cFC4STETerrain();

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override { return ++refCount_; }
    uint32_t Release() override;

    bool Init() override;      // 3
    bool Shutdown() override;  // 4

    bool IsInitialized() override { return f_.initialized != 0; }               // 5
    float CellWidth() override { return 16.0f; }                               // 6
    uint32_t CellCountX() override;                                            // 7
    uint32_t CellCountZ() override;                                            // 8
    int32_t GetVertIndex(int32_t x, int32_t z) const override;                 // 9
    void GetVertex(int32_t index, int32_t& x, int32_t& z) const override;      // 10
    bool LocationIsInBounds(float x, float z) const override;                  // 11
    float GetAltitudeAtFlatIndex(int32_t index) override;                      // 12
    float GetAltitudeAt(float x, float z) override;                            // 13
    float GetAltitudeAtVertex(int32_t x, int32_t z) override;                  // 14
    void GetAltitudes(SC4Rect<int32_t> const& r, float* out, int32_t rowStride,
                      int32_t colStride) override;                            // 15
    void GetWaterDepths(SC4Rect<int32_t> const& r, float* out, int32_t rowStride,
                        int32_t colStride) override;                          // 16
    void GetCellCornerAltitudes(uint32_t x, uint32_t z, float* out) override;  // 17
    float GetMaxSurfaceLevel(uint32_t x, uint32_t z) override;                 // 18
    float GetAltitudeAtNearestGrid(float x, float z) override;                 // 19
    float GetAltitudeCheckSum(SC4Rect<int32_t> const& r) const override;       // 20
    float MinHeight() override;                                                // 21
    float MaxHeight() override;                                                // 22
    float MinAllowedHeight() override;                                         // 23
    float MaxAllowedHeight() override;                                         // 24
    int32_t GetTerrainGridIndex(float x, float z) override;                    // 25
    void GetFaceNormal(float x, float z, cS3DVector3& out) override;           // 26
    void GetLerpedNormal(float, float, cS3DVector3&) override {}               // 27  render
    void GetVertexNormal(uint32_t, uint32_t, cS3DVector3&) override {}         // 28  render
    float GetSeaLevel() const override;                                        // 29
    float GetWaterLevelAt(float x, float z) override;                          // 30
    float GetWaterLevelAtIndex(int32_t index) override;                        // 31
    float GetLerpedWaterLevel(float x, float z) override;                      // 32
    float GetWaterDepthAtVertexXZ(int32_t x, int32_t z) override;              // 33
    bool CellHasWater(uint32_t x, uint32_t z) const override;                  // 34
    bool CellHasLand(uint32_t x, uint32_t z) const override;                   // 35
    bool IsLandVertex(uint32_t x, uint32_t z) override;                        // 36
    bool IsWaterVertex(uint32_t x, uint32_t z) override;                       // 37
    bool IsSeaVertex(uint32_t x, uint32_t z) override;                         // 38
    bool AreSeaVerticesIn(SC4Rect<int32_t> const& r) const override;           // 39
    void DoPostCityInit() override;                                            // 40
    bool SetTerrainType(int32_t t) override;                                   // 41
    int32_t TerrainType() override;                                            // 42
    void CreateFlatAreas(bool b) override;                                     // 43
    bool CreateShoreCliffs(bool b) override;                                   // 44
    bool SetSeaLevel(float h) override;                                        // 45
    bool ChangeSeaLevel(bool up) override;                                     // 46
    float GetDefaultTerrainRaiseLowerAmount() const override { return 20.0f; }  // 47
    void ChangeTerrainLevel(bool up, float amount) override;                   // 48
    void SetAltitudeRect(SC4Rect<int32_t> const& r, float h, bool redisplay, uint32_t data4) override;  // 49
    void SetAltitudeAtVertex(int32_t index, float h) override;                 // 50
    void SetAltitudeXZ(int32_t x, int32_t z, float h) override;                // 51
    void LockAltitude(SC4Rect<int32_t> const& r, bool lock) override;          // 52
    uint32_t NumberOfWaterVerticesIn(SC4Rect<int32_t> const& r) override;      // 53
    cISTETerrainView* GetRenderObject() override {
        return static_cast<cISTETerrainView*>(f_.renderObject);
    }                                                                           // 54
    float GetSedimentValue(float x, float z) override;                         // 55
    void StartFlood() override;                                                // 56
    bool ApplyBrush(uint32_t brushId, TBrushParameters& p) override;           // 57
    bool GetBrushExemplarID(int32_t, uint32_t&) override { return false; }     // 58
    void BeginBrushScrub(uint32_t a, uint32_t b) override;                   // 59
    void EndBrushScrub(uint32_t a, uint32_t b) override;                       // 60
    float BrushRadius(uint32_t) const override { return 0.0f; }                // 61
    BrushCost GetCostPerBrushApplication(uint32_t) override { return 0; }      // 62
    bool GetUIElementsForBrush(uint32_t, uint32_t*) const override { return false; }  // 63
    void UpdateWaveProducers() override {}                                     // 64  render
    bool IsGoodWaveProducerLocation(float, float, float) const override { return false; }  // 65  render
    cISTETerrainView* GetView() override {  // 66
        // terrain.md §15.5.6: +0xE8 holds the object base, while callers
        // require IID 0x6771477D at base+0x0C (a different vtable).
        return f_.view == nullptr ? nullptr : reinterpret_cast<cISTETerrainView*>(
            static_cast<unsigned char*>(f_.view) + 0x0C);
    }
    void DoLighting(SC4Rect<int32_t> const&) override {}                       // 67  render
    void RedisplayTerrainRect(bool terrain, bool water, SC4Rect<int32_t> const& r,
                              uint32_t data4) override;                        // 68
    void RedisplayTerrainPoints(bool terrain, bool water, SC4Point<int32_t> const& a,
                                SC4Point<int32_t> const& b, uint32_t data4) override;  // 69
    bool IsTriangulationFlipped(uint32_t x, uint32_t z) override;              // 70
    void HideTerrainCell(uint32_t x, uint32_t z, bool hide) override;          // 71
    void HideEvenTriangle(uint32_t x, uint32_t z, bool hide) override;         // 72
    void HideOddTriangle(uint32_t x, uint32_t z, bool hide) override;          // 73
    bool IsEvenTriangleHidden(uint32_t x, uint32_t z) const override;          // 74
    bool IsOddTriangleHidden(uint32_t x, uint32_t z) const override;           // 75
    bool IsWholeCellHidden(uint32_t x, uint32_t z) const override;             // 76
    bool IsPointHidden(float x, float z) const override;                       // 77
    void StartAccumulatingTextureUpdates() override { f_.accumulateTexture = 1; }     // 78
    void StopAccumulatingTextureUpdates() override;                            // 79
    bool IsAccumulatingTextureUpdates() override { return f_.accumulateTexture != 0; }       // 80
    bool SetTerrainCellFlags(uint32_t x, uint32_t z, uint32_t flags) override; // 81
    uint32_t GetTerrainCellFlags(uint32_t x, uint32_t z) override;             // 82
    bool GetWaterSurfaceDisplayState() override;                               // 83
    void SetWaterSurfaceDisplayState(bool on) override;                        // 84
    bool GetPlumbingInfoDisplayState() override { return plumbingDisplay_; }   // 85
    void SetPlumbingInfoDisplayState(bool on) override { plumbingDisplay_ = on; }  // 86
    void BeginLongTerrainOperation() override;                                 // 87
    void EndLongTerrainOperation() override;                                   // 88
    void ReconcileEdgesWithNeighboringCities(bool, bool, bool) override;       // 89
    bool SaveObliterated(cIGZPersistDBSegment*) override { return false; }     // 90
    void DeletingDestructor(uint32_t flags) override;                          // 91
    float GetWaterDepthAtFlatIndex(int32_t index) override;                    // 92
    bool FlipInternalTriangulation(uint32_t x, uint32_t z, bool flip) override;  // 93
    bool FlipExternalTriangulation(uint32_t x, uint32_t z, bool flip) override;  // 94
    bool LoadObliterated(cIGZPersistDBSegment*) override { return false; }     // 95
    bool Load(cIGZPersistDBSegment*) override;                                 // 96
    bool Save(cIGZPersistDBSegment*) override;                                 // 97

    bool DoMessage(cIGZMessage2* msg) override;  // cIGZMessageTarget2 (§12.1)

    // §13.2: u16 version 2 and a reference to the terrain view (class
    // 0xC9B84E10). We hold no view, so the reference is read and dropped.
    bool Write(cIGZOStream& out) override {
        Log("terrain: Write (class record, §13.2)");
        return out.SetUint16(2) && WriteNullRef(out);
    }
    bool Read(cIGZIStream& in) override {
        uint16_t version = 0;
        const bool ok = in.GetUint16(version) && ReadRef(in);
        Log("terrain: Read (class record = version + ref, §13.2) -> ok=%d version=%u", ok ? 1 : 0, version);
        return ok;
    }
    uint32_t GetGZCLSID() override { return kFC4_CLSID_cSTETerrain; }

    cIGZUnknown* Primary() { return static_cast<cIGZUnknown*>(static_cast<cFC4ISTETerrain*>(this)); }
    cIGZMessageTarget2* Target() { return static_cast<cIGZMessageTarget2*>(this); }

    // For the test seam.
    fc4_terrain* Core() const { return core_; }
    HostContext* Host() { return &host_; }
    using ViewInit = bool(__thiscall*)(void*, cFC4ISTETerrain*);
    using ViewShutdown = void(__thiscall*)(void*);
    using TerrainStaticInit = bool(__cdecl*)();
    bool InitView(TerrainStaticInit staticInit, ViewInit init, ViewShutdown shutdown);

private:
    bool Enter() const;
    void Leave() const;
    bool CreateCore(uint32_t cellsX, uint32_t cellsZ);
    void DestroyCore();
    void SyncDrawVertices();
    void StopView();
    void Subscribe();
    void Unsubscribe();
    void OnViewUpdate(cIGZMessage2Standard* s);
    void OnAnimationTick();
    void PostReconcile();
    void SetAnimationTick(bool enable);

    cFC4STESurfaceWater* water_ = nullptr;  // the terrain's own (§1, +0x24)
    HostContext host_{};
    fc4_terrain_host hostTable_{};
    fc4_terrain* core_ = nullptr;
    uint32_t refCount_ = 0;
    bool destroying_ = false;
    bool subscribed_ = false;
    bool plumbingDisplay_ = false;  // §15.1: a global byte, not a field
    bool tickRegistered_ = false;
    bool viewInitialized_ = false;
    ViewShutdown viewShutdown_ = nullptr;
    std::vector<TerrainDrawVertex> drawVertices_;
    std::vector<WaterDrawVertex> waterVertices_;
    int32_t scrubbing_ = 0;

    mutable CRITICAL_SECTION lock_;
    mutable DWORD mainTid_ = 0;
    mutable int depth_ = 0;
};

// The three base sub-objects are three vptrs (§1: QueryInterface returns this,
// this + 4, this + 8), so the fields begin at +0x0C as terrain.md §2.3 says.
// offsetof on a polymorphic class is conditionally supported; MSVC accepts it,
// and this is exactly the property that has to hold.
static_assert(offsetof(cFC4STETerrain, f_) == 0x0C, "terrain fields start at +0x0C (§2.3)");

cFC4STETerrain::cFC4STETerrain() {
    InitializeCriticalSection(&lock_);
    // host_ default-constructs (its atomics make it non-assignable); only the
    // table has to be filled.
    hostTable_ = MakeHost(&host_);
    host_.self = Primary();
    host_.drawOwner = this;
    host_.syncDraw = [](void* owner) { static_cast<cFC4STETerrain*>(owner)->SyncDrawVertices(); };
    // §1: the terrain owns the surface water at +0x24. The city may also build
    // one through the class factory; both are views over the same core.
    water_ = new cFC4STESurfaceWater();
    water_->AddRef();  // ours
    f_.surfaceWater = water_->Primary();
    f_.saved78 = 308.0f;  // §2.4: tuning 0x48CD7B20, shipped 308
    f_.neighbourOffset[0] = -1;              // §11.3: W, NW, N, NE, E, SE, S, SW
    f_.neighbourOffset[1] = -1;              // filled with vertsX once known
    f_.neighbourOffset[2] = 0;
    f_.neighbourOffset[3] = 1;
    f_.neighbourOffset[4] = 1;
    f_.neighbourOffset[5] = 1;
    f_.neighbourOffset[6] = 0;
    f_.neighbourOffset[7] = -1;
}

cFC4STETerrain::~cFC4STETerrain() {
    StopView();
    DestroyCore();
    if (f_.view != nullptr) {
        // The view came from GZCOM()->GetClassObject: its QI took one reference.
        FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(f_.view));
        f_.view = nullptr;
        f_.renderObject = nullptr;
    }
    if (water_ != nullptr) {
        f_.surfaceWater = nullptr;
        water_->Release();
        water_ = nullptr;
    }
    DeleteCriticalSection(&lock_);
}

bool cFC4STETerrain::Enter() const {
    EnterCriticalSection(&lock_);
    if (mainTid_ == 0) mainTid_ = GetCurrentThreadId();
    ++depth_;
    return true;
}

void cFC4STETerrain::Leave() const {
    --depth_;
    LeaveCriticalSection(&lock_);
}

bool cFC4STETerrain::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (riid == GZIID_cIGZUnknown || riid == kFC4_IID_cISTETerrain || riid == 0x00000001u ||
        riid == 0xAB953253u) {
        *ppvObj = Primary();
    } else if (riid == kFC4_IID_cIGZMessageTarget2) {
        *ppvObj = Target();
    } else if (riid == kFC4_IID_cIGZSerializable) {
        *ppvObj = static_cast<cIGZSerializable*>(this);
    } else {
        return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4STETerrain::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    if (destroying_) return 0;
    destroying_ = true;
    if (f_.initialized != 0) Shutdown();
    delete this;
    return 0;
}

bool cFC4STETerrain::CreateCore(uint32_t cellsX, uint32_t cellsZ) {
    if (core_ != nullptr) return true;
    fc4_terrain_tuning tuning{};
    tuning.struct_size = sizeof(tuning);
    if (!fc4_terrain_tuning_shipped(&tuning)) {
        tuning = fc4_terrain_tuning{};
        tuning.struct_size = sizeof(tuning);
    }
    core_ = fc4_terrain_create(&hostTable_, &tuning, cellsX, cellsZ);
    if (core_ == nullptr) {
        Log("terrain: could not create the terrain core (%u x %u cells)", cellsX, cellsZ);
        return false;
    }
    fc4_terrain_set_default_brush_level(core_, f_.defaultBrushLevel);
    // §2.3: the grid fields the game reads directly.
    const uint32_t vx = cellsX + 1, vz = cellsZ + 1;
    f_.cellsX = cellsX;
    f_.cellsZ = cellsZ;
    f_.cellsXm1 = cellsX - 1;
    f_.cellsZm1 = cellsZ - 1;
    f_.vertsX = vx;
    f_.vertsZ = vz;
    f_.cityRect[0] = 0;
    f_.cityRect[1] = 0;
    f_.cityRect[2] = static_cast<int32_t>(cellsX);
    f_.cityRect[3] = static_cast<int32_t>(cellsZ);
    f_.sizeX = 16.0f * static_cast<float>(cellsX);
    f_.sizeZ = 16.0f * static_cast<float>(cellsZ);
    f_.vertexCount = vx * vz;
    drawVertices_.resize(f_.vertexCount);
    waterVertices_.resize(f_.vertexCount);
    f_.vertices = drawVertices_.data();
    cFC4STESurfaceWater::PublishDrawVertices(waterVertices_.data());
    f_.seaLevel = fc4_terrain_sea_level(core_);
    f_.cachedMin = fc4_terrain_cached_height(core_, 0);
    f_.cachedMax = fc4_terrain_cached_height(core_, 1);
    // §11.3: the 8 neighbour index offsets, W NW N NE E SE S SW.
    const int32_t vsx = static_cast<int32_t>(vx);
    f_.neighbourOffset[0] = -1;
    f_.neighbourOffset[1] = -1 - vsx;
    f_.neighbourOffset[2] = -vsx;
    f_.neighbourOffset[3] = 1 - vsx;
    f_.neighbourOffset[4] = 1;
    f_.neighbourOffset[5] = vsx + 1;
    f_.neighbourOffset[6] = vsx;
    f_.neighbourOffset[7] = vsx - 1;
    g_activeCore.store(core_, std::memory_order_release);
    g_cellsX.store(cellsX, std::memory_order_release);
    g_cellsZ.store(cellsZ, std::memory_order_release);
    SyncDrawVertices();
    g_drawVertices.store(drawVertices_.data(), std::memory_order_release);
    return true;
}

void cFC4STETerrain::SyncDrawVertices() {
    if (core_ == nullptr) return;
    for (uint32_t i = 0; i < f_.vertexCount; ++i) {
        const int32_t x = static_cast<int32_t>(i % f_.vertsX);
        const int32_t z = static_cast<int32_t>(i / f_.vertsX);
        auto& v = drawVertices_[i];
        v.altitude = fc4_terrain_altitude_at_index(core_, i);
        // SPEC-GAP: the core exposes face normals, not smoothed rendering normals; retain the face normal here.
        v.normal[1] = 1.0f;
        fc4_terrain_face_normal(core_, 16.0f * static_cast<float>(x < static_cast<int32_t>(f_.cellsX) ? x : x - 1),
                                16.0f * static_cast<float>(z < static_cast<int32_t>(f_.cellsZ) ? z : z - 1), v.normal);
        // LevelCell/RemoveCellLevelling own these two bits in the native array.
        v.flags = static_cast<uint16_t>((fc4_terrain_vertex_flags(core_, x, z) & ~0x60u) | (v.flags & 0x60u));
        auto& w = waterVertices_[i];
        w.depth = fc4_terrain_water_depth_at_index(core_, i);
        fc4_terrain_water_velocity(core_, x, z, w.velocity);
        w.sediment = fc4_terrain_sediment(core_, x, z);
    }
}

bool cFC4STETerrain::InitView(TerrainStaticInit staticInit, ViewInit init, ViewShutdown shutdown) {
    // NM20: native slot 15 borrows this view and reads its retained PRIMARY
    // terrain, including +0x6C. Publication/readiness must follow Init.
    if (viewInitialized_) return true;
    if (f_.view == nullptr || staticInit == nullptr || init == nullptr || shutdown == nullptr) return false;
    // terrain.md section 15.5.12.2: the native view's process-wide texture
    // tables depend on terrain StaticInit. It is idempotent, has no receiver,
    // and must run before the view's first Init allocates those tables.
    if (!staticInit() || !init(f_.view, this)) return false;
    viewShutdown_ = shutdown;
    viewInitialized_ = true;
    f_.viewReady = 1;
    return true;
}

void cFC4STETerrain::StopView() {
    if (!viewInitialized_) return;
    f_.viewReady = 0;
    viewInitialized_ = false;
    void* view = f_.view;
    f_.view = nullptr;
    f_.renderObject = nullptr;
    // Unregister borrowed draw-context users before releasing the terrain or
    // freeing its arrays (terrain-view.md §9.3). Shutdown drops the view's ref.
    viewShutdown_(view);
    FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(view));
}

void cFC4STETerrain::DestroyCore() {
    if (core_ == nullptr) return;
    g_activeCore.store(nullptr, std::memory_order_release);
    g_drawVertices.store(nullptr, std::memory_order_release);
    fc4_terrain_destroy(core_);
    core_ = nullptr;
    f_.vertices = nullptr;
    cFC4STESurfaceWater::PublishDrawVertices(nullptr);
    drawVertices_.clear();
    waterVertices_.clear();
    g_cellsX.store(0, std::memory_order_release);
    g_cellsZ.store(0, std::memory_order_release);
}

bool cFC4STETerrain::Init() {
    Enter();
    bool ok = false;
    do {
        if (f_.initialized != 0) {
            ok = true;
            break;
        }
        f_.city = CurrentCity();  // borrowed (§2.3 +0x1C)
        cISC4City* city = static_cast<cISC4City*>(f_.city);
        // Slot 54 (§15.1: `+0xE4`, exported as the folded GetSelectedCity)
        // is read as the city by the game: cSC4City::Init calls a method on
        // what it returns and faults when it is null. terrain.md calls it a
        // render object; the in-game evidence says city. [choice]
        f_.renderObject = city;
        uint32_t cx = 64, cz = 64;
        if (city != nullptr) {
            // [abi-ret] city slots 93/94 CellCountX / CellCountZ (terrain.md
            // §2.1: "the city has cellsX x cellsZ cells ... read in Init"; the
            // export folds those two names). A count in EAX, never a bool.
            cx = city->CellCountX();
            cz = city->CellCountZ();
            if (cx == 0 || cz == 0 || cx > 4096 || cz > 4096) {
                Log("terrain: implausible city cell counts %u x %u; using 64 x 64", cx, cz);
                cx = cz = 64;
            }
        }
        if (!CreateCore(cx, cz)) break;
        f_.initialized = 1;
        Subscribe();
        SetAnimationTick(true);
        Log("terrain: replacement ON (enable = terrain): %u x %u cells, sea=%.9g, default brush level=%d", cx,
            cz, fc4_terrain_sea_level(core_), f_.defaultBrushLevel);
        ok = true;
    } while (false);
    Leave();
    return ok;
}

bool cFC4STETerrain::Shutdown() {
    Enter();
    if (core_ != nullptr) {
        fc4_terrain_state st{};
        st.struct_size = sizeof(st);
        if (fc4_terrain_get_state(core_, &st)) {
            Log("terrain: shutdown: posts_pending=%u brushes=%u reentrant refused core=%u guard=%u "
                "(both expected 0 outside god-mode brush kinds 1 and 8-10)",
                st.posts_pending, st.brush_count, st.reentrant_refused,
                host_.guardRefused.load(std::memory_order_relaxed));
        }
    }
    SetAnimationTick(false);
    Unsubscribe();
    StopView();
    DestroyCore();
    f_.initialized = 0;
    f_.city = nullptr;
    Leave();
    return true;
}

void cFC4STETerrain::Subscribe() {
    if (subscribed_) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    for (uint32_t type : kSubscribed) {
        if (ms->AddNotification(Target(), type)) subscribed_ = true;
    }
}

void cFC4STETerrain::Unsubscribe() {
    if (!subscribed_) return;
    cIGZMessageServer2Ptr ms;
    if (ms) {
        for (uint32_t type : kSubscribed) ms->RemoveNotification(Target(), type);
    }
    subscribed_ = false;
}

void cFC4STETerrain::DoPostCityInit() {
    Enter();
    if (core_ != nullptr && f_.postCityInit == 0) {
        f_.postCityInit = 1;  // §2.3 / §15.5.1: mark before any collaborator call
        // TR9 (§2.3): the original copies the 3D camera's zoom (global
        // 0x00B43DD8, +0xC) into +0xBC here. The plugin has no camera
        // interface, so the level keeps its constructor value 0 until the
        // first view-update message 0xA6B79602 arrives - which is also what a
        // harness with no view sees (conformance test 20).
        fc4_terrain_set_default_brush_level(core_, f_.defaultBrushLevel);
        // §2.3 (+0xE8) / §15.1 slot 66: the 3D terrain view (class 0xC9B84E10),
        // which the original's DoPostCityInit creates. zones.md §5.2: the zone
        // developer's PostCityInit handler takes the terrain view from slot 66
        // and dereferences it, so a null one faults there. Drawing is out of the
        // terrain spec's scope, but the object itself is not optional: it is
        // created through the game's own class factory, never hand-built.
        if (f_.view == nullptr) {
            cIGZCOM* com = GZCOM();
            void* view = nullptr;
            // §15.5.1 step 3: the factory's requested interface is at
            // base+0x0C. Store the base at +0xE8, as the native layout requires.
            if (com != nullptr && com->GetClassObject(kFC4_CLSID_cSTETerrainView, 0x6771477Du, &view) &&
                view != nullptr) {
                f_.view = static_cast<unsigned char*>(view) - 0x0C;
                FC4_ACQUIRED(static_cast<cIGZUnknown*>(f_.view), "terrain view factory");
                f_.renderObject = f_.view;
                Log("terrain: created the terrain 3D view (class 0x%08X) at DoPostCityInit",
                    kFC4_CLSID_cSTETerrainView);
            } else {
                Log("terrain: WARNING the terrain 3D view (class 0x%08X) could not be created; the zone "
                    "developer's PostCityInit path reads it (zones.md §5.2)",
                    kFC4_CLSID_cSTETerrainView);
            }
        }
        if (InitView(reinterpret_cast<TerrainStaticInit>(compat::DirectCall(compat::Direct::TerrainStaticInit)),
                     reinterpret_cast<ViewInit>(compat::DirectCall(compat::Direct::TerrainViewInit)),
                     reinterpret_cast<ViewShutdown>(compat::DirectCall(compat::Direct::TerrainViewShutdown)))) {
            Log("terrain: NM20 view initialized: base=%p drawing=%p primary=%p vertices=%p ready=%u",
                f_.view, GetView(), Primary(), f_.vertices, f_.viewReady);
        }
        fc4_terrain_post_city_init(core_);
    }
    Leave();
}

// TR23 (§2.3): the zoom index is the u32 pointed to by GetVoid2, not data1.
void cFC4STETerrain::OnViewUpdate(cIGZMessage2Standard* s) {
    if (s == nullptr || core_ == nullptr) return;
    const void* zoom = s->GetVoid2();
    // SPEC-GAP: a missing zoom payload leaves the current level unchanged.
    if (zoom == nullptr) return;
    int32_t want = 0;
    std::memcpy(&want, zoom, sizeof(want));
    if (want != f_.defaultBrushLevel) {
        f_.defaultBrushLevel = want;
        fc4_terrain_set_default_brush_level(core_, f_.defaultBrushLevel);
        Log("terrain: TR9 default brush level -> %d (view update)", f_.defaultBrushLevel);
    }
}

bool cFC4STETerrain::DoMessage(cIGZMessage2* msg) {
    if (msg == nullptr) return true;
    FC4_X87_SCOPE("terrain DoMessage", msg->GetType());
    const uint32_t type = msg->GetType();
    cIGZMessage2Standard* s = nullptr;
    if (msg->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&s)) && s != nullptr) {
        FC4_ACQUIRED(s, "QueryInterface");
    }
    switch (type) {
        case kMsgPostCityInit: DoPostCityInit(); break;
        case kMsgPreCityShutdown: Shutdown(); break;
        case kMsgViewUpdate:
            Enter();
            OnViewUpdate(s);
            Leave();
            break;
        case kMsgViewWindow:
            // §12.1: refresh the textures, then RedisplayTerrain(1, 1, the whole
            // city, 1). Only the renderer reads the texture half.
            Enter();
            if (core_ != nullptr) {
                const int32_t cx = static_cast<int32_t>(fc4_terrain_cell_count(core_, 0));
                const int32_t cz = static_cast<int32_t>(fc4_terrain_cell_count(core_, 1));
                fc4_terrain_rect r{0, 0, cx, cz};
                fc4_terrain_redisplay(core_, true, true, r, 1);
            }
            Leave();
            break;
        case kMsgAnimationTick: OnAnimationTick(); break;
        case kMsgReconcile: {
            cIGZWinMgrPtr wm;
            if (wm) {
                if (wm->IsModal()) PostReconcile();
                else ReconcileEdgesWithNeighboringCities(false, true, true);
            }
            break;
        }
        case kMsgSimNewDay:
            Enter();
            if (core_ != nullptr) fc4_terrain_bodies_on_new_day(core_);  // §10.3
            Leave();
            break;
        default: break;
    }
    if (s != nullptr) FC4_RELEASE(s);
    return true;
}

void cFC4STETerrain::PostReconcile() {
    if (host_.testMode) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    auto* message = FC4MakeMessage(kMsgReconcile, 0, 0, 0);
    if (message == nullptr) return;
    ms->GeneralMessagePostToTarget(message, Target());
    message->Release();
}

void cFC4STETerrain::SetAnimationTick(bool enable) {
    if (host_.testMode || tickRegistered_ == enable) return;
    cIGZFrameWork* fw = RZGetFrameWork();
    void* service = nullptr;
    if (fw == nullptr || !fw->GetSystemService(kFC4_SID_AnimationTickManager,
                                              kFC4_IID_AnimationTickManager, &service) ||
        service == nullptr) return;
    auto* manager = static_cast<cFC4ISC4AnimationTickManager*>(service);
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(manager), "terrain animation manager");
    // SPEC-GAP: TR42: terrain's priority is unspecified; use water's 0xC0.
    if (enable) {
        manager->AddTickableObject(Target(), 0xC0);
        tickRegistered_ = true;
    }
    else {
        manager->RemoveTickableObject(Target(), 0xC0);
        tickRegistered_ = false;
    }
    FC4_RELEASE(static_cast<cIGZUnknown*>(manager));
}

void cFC4STETerrain::OnAnimationTick() {
    // §12.1: count only ticks after view creation; never wrap and retrigger.
    if (f_.view == nullptr || f_.reconcileTick >= 2) return;
    if (++f_.reconcileTick != 2) return;
    // SPEC-GAP: TR42: preference access is unspecified; use its default (1).
    PostReconcile();
}

void cFC4STETerrain::ReconcileEdgesWithNeighboringCities(bool requested, bool, bool) {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw != nullptr ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) ||
        sc4 == nullptr) return;
    FC4_ACQUIRED(sc4, "terrain reconciliation app");
    const bool enabled = sc4->GetPopupDialogsEnabled();
    FC4_RELEASE(sc4);
    if (!enabled) return;  // foundations §11.6: no completion with popups off
    // SPEC-GAP: TR43: no edge-comparison contract; report no mismatch/change.
    fc4_terrain_post post{};
    post.msg_type = 0xCA63D517;
    post.data1 = requested ? 1 : 0;
    HostPost(&host_, &post);
    Log("terrain: reconciliation complete (TR43: no edge mismatch comparison)");
}

// ---- the height queries (§4) ----------------------------------------------------------------------

uint32_t cFC4STETerrain::CellCountX() {
    return core_ != nullptr ? fc4_terrain_cell_count(core_, 0) : 0u;
}
uint32_t cFC4STETerrain::CellCountZ() {
    return core_ != nullptr ? fc4_terrain_cell_count(core_, 1) : 0u;
}

int32_t cFC4STETerrain::GetVertIndex(int32_t x, int32_t z) const {
    return core_ != nullptr ? static_cast<int32_t>(fc4_terrain_vert_index(core_, x, z)) : 0;
}

void cFC4STETerrain::GetVertex(int32_t index, int32_t& x, int32_t& z) const {
    uint32_t ux = 0, uz = 0;
    if (core_ != nullptr && fc4_terrain_vertex_of(core_, static_cast<uint32_t>(index), &ux, &uz)) {
        x = static_cast<int32_t>(ux);
        z = static_cast<int32_t>(uz);
    }
}

bool cFC4STETerrain::LocationIsInBounds(float x, float z) const {
    return core_ != nullptr ? fc4_terrain_location_is_in_bounds(core_, x, z) : false;
}

float cFC4STETerrain::GetAltitudeAtFlatIndex(int32_t index) {
    return core_ != nullptr ? fc4_terrain_altitude_at_index(core_, static_cast<uint32_t>(index)) : 0.0f;
}

float cFC4STETerrain::GetAltitudeAt(float x, float z) {
    return core_ != nullptr ? fc4_terrain_altitude(core_, x, z) : 0.0f;
}

float cFC4STETerrain::GetAltitudeAtVertex(int32_t x, int32_t z) {
    return core_ != nullptr ? fc4_terrain_altitude_at_vertex(core_, x, z) : 0.0f;
}

void cFC4STETerrain::GetAltitudes(SC4Rect<int32_t> const& r, float* out, int32_t rowStride,
                                  int32_t colStride) {
    if (core_ == nullptr || out == nullptr) return;
    // §4.2: an inclusive vertex rectangle. *[decomp only]* for the strides;
    // row-major with the caller's strides is the literal reading. [choice]
    const int32_t w = r.bottomRightX - r.topLeftX + 1;
    const int32_t h = r.bottomRightY - r.topLeftY + 1;
    if (w <= 0 || h <= 0) return;
    std::vector<float> buf(static_cast<size_t>(w) * static_cast<size_t>(h));
    fc4_terrain_rect rect{r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY};
    const size_t n = fc4_terrain_altitudes_in(core_, rect, buf.data(), buf.size());
    for (int32_t z = 0; z < h; ++z) {
        for (int32_t x = 0; x < w; ++x) {
            const size_t i = static_cast<size_t>(z) * static_cast<size_t>(w) + static_cast<size_t>(x);
            if (i >= n) return;
            out[static_cast<size_t>(z) * static_cast<size_t>(rowStride) +
                static_cast<size_t>(x) * static_cast<size_t>(colStride)] = buf[i];
        }
    }
}

void cFC4STETerrain::GetWaterDepths(SC4Rect<int32_t> const& r, float* out, int32_t rowStride,
                                    int32_t colStride) {
    if (core_ == nullptr || out == nullptr) return;
    const int32_t w = r.bottomRightX - r.topLeftX + 1;
    const int32_t h = r.bottomRightY - r.topLeftY + 1;
    if (w <= 0 || h <= 0) return;
    for (int32_t z = 0; z < h; ++z) {
        for (int32_t x = 0; x < w; ++x) {
            out[static_cast<size_t>(z) * static_cast<size_t>(rowStride) +
                static_cast<size_t>(x) * static_cast<size_t>(colStride)] =
                fc4_terrain_water_depth_at_vertex(core_, r.topLeftX + x, r.topLeftY + z);
        }
    }
}

void cFC4STETerrain::GetCellCornerAltitudes(uint32_t x, uint32_t z, float* out) {
    if (core_ == nullptr || out == nullptr) return;
    fc4_terrain_cell_corner_altitudes(core_, static_cast<int32_t>(x), static_cast<int32_t>(z), out);
}

float cFC4STETerrain::GetMaxSurfaceLevel(uint32_t x, uint32_t z) {
    return core_ != nullptr ? fc4_terrain_max_surface_level(core_, static_cast<int32_t>(x),
                                                            static_cast<int32_t>(z))
                            : 0.0f;
}

float cFC4STETerrain::GetAltitudeAtNearestGrid(float x, float z) {
    return core_ != nullptr ? fc4_terrain_altitude_at_nearest_grid(core_, x, z) : 0.0f;
}

float cFC4STETerrain::GetAltitudeCheckSum(SC4Rect<int32_t> const& r) const {
    if (core_ == nullptr) return 0.0f;
    fc4_terrain_rect rect{r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY};
    return fc4_terrain_altitude_checksum(core_, rect);
}

float cFC4STETerrain::MinHeight() {
    return core_ != nullptr ? fc4_terrain_cached_height(core_, 0) : 0.0f;
}
float cFC4STETerrain::MaxHeight() {
    return core_ != nullptr ? fc4_terrain_cached_height(core_, 1) : 0.0f;
}
float cFC4STETerrain::MinAllowedHeight() {
    return core_ != nullptr ? fc4_terrain_allowed_height(core_, 0) : 0.0f;
}
float cFC4STETerrain::MaxAllowedHeight() {
    return core_ != nullptr ? fc4_terrain_allowed_height(core_, 1) : 0.0f;
}

int32_t cFC4STETerrain::GetTerrainGridIndex(float, float) {
    // §15.1 lists the slot with no rule and §4-§10 never defines it. Refused
    // rather than guessed. See the lane report's numbered question.
    return -1;
}

void cFC4STETerrain::GetFaceNormal(float x, float z, cS3DVector3& out) {
    if (core_ == nullptr) return;  // §4.3: out is untouched out of bounds
    float v[3] = {out.fX, out.fY, out.fZ};
    if (fc4_terrain_face_normal(core_, x, z, v)) {
        out.fX = v[0];
        out.fY = v[1];
        out.fZ = v[2];
    }
}

// ---- the water queries (§5) -----------------------------------------------------------------------

float cFC4STETerrain::GetSeaLevel() const {
    return core_ != nullptr ? fc4_terrain_sea_level(core_) : 0.0f;
}
float cFC4STETerrain::GetWaterLevelAt(float x, float z) {
    return core_ != nullptr ? fc4_terrain_water_level(core_, x, z) : 0.0f;
}
float cFC4STETerrain::GetWaterLevelAtIndex(int32_t index) {
    return core_ != nullptr ? fc4_terrain_water_level_at_index(core_, static_cast<uint32_t>(index)) : 0.0f;
}
float cFC4STETerrain::GetLerpedWaterLevel(float x, float z) {
    return core_ != nullptr ? fc4_terrain_lerped_water_level(core_, x, z) : 0.0f;
}
float cFC4STETerrain::GetWaterDepthAtVertexXZ(int32_t x, int32_t z) {
    return core_ != nullptr ? fc4_terrain_water_depth_at_vertex(core_, x, z) : 0.0f;
}
bool cFC4STETerrain::CellHasWater(uint32_t x, uint32_t z) const {
    return core_ != nullptr
               ? fc4_terrain_cell_has_water(core_, static_cast<int32_t>(x), static_cast<int32_t>(z))
               : false;
}
bool cFC4STETerrain::CellHasLand(uint32_t x, uint32_t z) const {
    return core_ != nullptr ? fc4_terrain_cell_has_land(core_, static_cast<int32_t>(x), static_cast<int32_t>(z))
                            : false;
}
bool cFC4STETerrain::IsLandVertex(uint32_t x, uint32_t z) {
    return core_ != nullptr ? fc4_terrain_is_land_vertex(core_, static_cast<int32_t>(x), static_cast<int32_t>(z))
                            : false;
}
bool cFC4STETerrain::IsWaterVertex(uint32_t x, uint32_t z) {
    return core_ != nullptr ? fc4_terrain_is_water_vertex(core_, static_cast<int32_t>(x), static_cast<int32_t>(z))
                            : false;
}
bool cFC4STETerrain::IsSeaVertex(uint32_t x, uint32_t z) {
    return core_ != nullptr ? fc4_terrain_is_sea_vertex(core_, static_cast<int32_t>(x), static_cast<int32_t>(z))
                            : false;
}

bool cFC4STETerrain::AreSeaVerticesIn(SC4Rect<int32_t> const& r) const {
    if (core_ == nullptr) return false;
    // §15.1 gives the name only. The literal reading: every vertex of the
    // inclusive rectangle is a sea vertex (§5: altitude < sea level). [choice]
    for (int32_t z = r.topLeftY; z <= r.bottomRightY; ++z) {
        for (int32_t x = r.topLeftX; x <= r.bottomRightX; ++x) {
            if (!fc4_terrain_is_sea_vertex(core_, x, z)) return false;
        }
    }
    return true;
}

float cFC4STETerrain::GetSedimentValue(float x, float z) {
    return core_ != nullptr ? fc4_terrain_sediment_at(core_, x, z) : 0.0f;
}

float cFC4STETerrain::GetWaterDepthAtFlatIndex(int32_t index) {
    return core_ != nullptr ? fc4_terrain_water_depth_at_index(core_, static_cast<uint32_t>(index)) : 0.0f;
}

// ---- the setters (§6) -----------------------------------------------------------------------------

bool cFC4STETerrain::SetTerrainType(int32_t t) {
    Enter();
    const bool ok = core_ != nullptr && fc4_terrain_set_terrain_type(core_, t);
    Leave();
    return ok;
}

int32_t cFC4STETerrain::TerrainType() {
    return core_ != nullptr ? fc4_terrain_terrain_type(core_) : 0;
}

void cFC4STETerrain::CreateFlatAreas(bool) {
    // §15.1: data4 0. The two terrain generators have no capi entry point.
    Log("terrain: CreateFlatAreas is not implemented");
}

bool cFC4STETerrain::CreateShoreCliffs(bool) {
    Log("terrain: CreateShoreCliffs is not implemented");
    return false;
}

bool cFC4STETerrain::SetSeaLevel(float h) {
    Enter();
    const bool ok = core_ != nullptr && fc4_terrain_set_sea_level(core_, h);  // §6
    Leave();
    return ok;
}

bool cFC4STETerrain::ChangeSeaLevel(bool up) {
    Enter();
    bool ok = false;
    if (core_ != nullptr) {
        // §15.1 groups this with SetSeaLevel; §6 gives no step for it. The
        // literal reading of the name: move the sea by the default raise/lower
        // amount (slot 47) in the given direction. [choice]
        const float step = GetDefaultTerrainRaiseLowerAmount();
        ok = fc4_terrain_set_sea_level(core_, fc4_terrain_sea_level(core_) + (up ? step : -step));
    }
    Leave();
    return ok;
}

void cFC4STETerrain::ChangeTerrainLevel(bool up, float amount) {
    Enter();
    if (core_ != nullptr) {
        // §8.2 assigns data4 1 to this call but §6 has no rule for its shape.
        // The literal reading of the name: raise or lower every vertex by
        // `amount` (the setters' locked-vertex and clamp rules apply). [choice]
        const uint32_t cx = fc4_terrain_cell_count(core_, 0);
        const uint32_t cz = fc4_terrain_cell_count(core_, 1);
        for (uint32_t z = 0; z <= cz; ++z) {
            for (uint32_t x = 0; x <= cx; ++x) {
                const uint32_t i = fc4_terrain_vert_index(core_, static_cast<int32_t>(x), static_cast<int32_t>(z));
                if (up) {
                    fc4_terrain_raise_vertex(core_, i, amount);
                } else {
                    fc4_terrain_lower_vertex(core_, i, amount);
                }
            }
        }
        fc4_terrain_rect r{0, 0, static_cast<int32_t>(cx), static_cast<int32_t>(cz)};
        fc4_terrain_redisplay(core_, true, true, r, 1);
    }
    Leave();
}

void cFC4STETerrain::SetAltitudeRect(SC4Rect<int32_t> const& r, float h, bool redisplay, uint32_t data4) {
    Enter();
    if (core_ != nullptr) {
        fc4_terrain_rect rect{r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY};
        fc4_terrain_set_altitude_rect(core_, rect, h, redisplay, data4);  // §6
    }
    Leave();
}

void cFC4STETerrain::SetAltitudeAtVertex(int32_t index, float h) {
    Enter();
    if (core_ != nullptr) fc4_terrain_set_altitude_at_vertex(core_, static_cast<uint32_t>(index), h);
    SyncDrawVertices();
    Leave();
}

void cFC4STETerrain::SetAltitudeXZ(int32_t x, int32_t z, float h) {
    Enter();
    if (core_ != nullptr) fc4_terrain_set_altitude(core_, x, z, h);
    SyncDrawVertices();
    Leave();
}

void cFC4STETerrain::LockAltitude(SC4Rect<int32_t> const& r, bool lock) {
    Enter();
    if (core_ != nullptr) {
        fc4_terrain_rect rect{r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY};
        fc4_terrain_lock_altitude(core_, rect, lock);
    }
    Leave();
}

uint32_t cFC4STETerrain::NumberOfWaterVerticesIn(SC4Rect<int32_t> const& r) {
    if (core_ == nullptr) return 0;
    fc4_terrain_rect rect{r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY};
    return fc4_terrain_number_of_water_vertices(core_, rect);
}

void cFC4STETerrain::StartFlood() {
    // §15.1: debug, +500 m of water on vertices 20..39 x 20..39. The harness
    // never drives it; refused rather than half-done.
    Log("terrain: StartFlood (debug) is not implemented");
}

// ---- the brushes (§9) -----------------------------------------------------------------------------

bool cFC4STETerrain::ApplyBrush(uint32_t brushId, TBrushParameters& p) {
    Enter();
    bool ok = false;
    if (core_ != nullptr) {
        fc4_brush_params bp{};
        bp.from_x = p.fromX;
        bp.from_z = p.fromZ;
        bp.to_x = p.toX;
        bp.to_z = p.toZ;
        bp.level = p.level;
        bp.redisplay = p.redisplay;
        bp.strength_scale = p.strength;
        bp.radius_scale = p.radiusScaling;
        bp.data4 = p.data4;
        bp.target_height = p.targetHeight;
        // TR18: a brush may call our host callbacks. Anything that comes back
        // into this handle is refused at the boundary and counted; the core
        // counts its own refusals too. Both stay 0 for the kinds the
        // simulation drives (0, 2, 5, 7), per §15.4.
        ++host_.inside;
        ok = fc4_terrain_apply_brush(core_, brushId, &bp);
        --host_.inside;
        p.outX0 = bp.out.x0;
        p.outZ0 = bp.out.z0;
        p.outX1 = bp.out.x1;
        p.outZ1 = bp.out.z1;
    }
    Leave();
    return ok;
}

void cFC4STETerrain::BeginBrushScrub(uint32_t a, uint32_t b) {
    (void)a;
    (void)b;
    Enter();
    ++scrubbing_;
    Leave();
}

void cFC4STETerrain::EndBrushScrub(uint32_t, uint32_t) {
    Enter();
    if (core_ != nullptr) {
        if (scrubbing_ > 0) --scrubbing_;
        // §9.6: EndBrushScrub makes the surface water rebuild its bodies.
        if (scrubbing_ == 0) fc4_terrain_update_bodies_of_water(core_);
    }
    Leave();
}

// ---- RedisplayTerrain and the long operations (§8) ------------------------------------------------

void cFC4STETerrain::RedisplayTerrainRect(bool terrain, bool water, SC4Rect<int32_t> const& r,
                                          uint32_t data4) {
    Enter();
    if (core_ != nullptr) {
        fc4_terrain_rect rect{r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY};
        fc4_terrain_redisplay(core_, terrain, water, rect, data4);  // §8.1
    }
    Leave();
}

void cFC4STETerrain::RedisplayTerrainPoints(bool terrain, bool water, SC4Point<int32_t> const& a,
                                            SC4Point<int32_t> const& b, uint32_t data4) {
    Enter();
    if (core_ != nullptr) {
        // §8.1: two cell corners, sorted, x1 := max + 1 clamped to cellsX.
        fc4_terrain_redisplay_cells(core_, terrain, water, a.x, a.y, b.x, b.y, data4);
    }
    Leave();
}

void cFC4STETerrain::BeginLongTerrainOperation() {
    Enter();
    if (core_ != nullptr) fc4_terrain_begin_long_operation(core_);  // §8.3
    Leave();
}

void cFC4STETerrain::EndLongTerrainOperation() {
    Enter();
    if (core_ != nullptr) fc4_terrain_end_long_operation(core_);  // §8.3
    Leave();
}

void cFC4STETerrain::StopAccumulatingTextureUpdates() {
    f_.accumulateTexture = 0;
    Enter();
    if (core_ != nullptr) {
        // §8.2: a texture-only redisplay (data4 0). Drawing is out of scope,
        // but the post is what the simulation can see.
        const int32_t cx = static_cast<int32_t>(fc4_terrain_cell_count(core_, 0));
        const int32_t cz = static_cast<int32_t>(fc4_terrain_cell_count(core_, 1));
        fc4_terrain_rect r{0, 0, cx, cz};
        fc4_terrain_redisplay(core_, false, false, r, 0);
    }
    Leave();
}

// ---- the flags (§7) -------------------------------------------------------------------------------

bool cFC4STETerrain::IsTriangulationFlipped(uint32_t x, uint32_t z) {
    return core_ != nullptr ? fc4_terrain_is_triangulation_flipped(core_, static_cast<int32_t>(x),
                                                                  static_cast<int32_t>(z))
                            : false;
}

void cFC4STETerrain::HideTerrainCell(uint32_t x, uint32_t z, bool hide) {
    // §7: 0x20 | 0x40 hides the whole cell.
    SetFlagBit(x, z, kFlagEvenHidden, hide);
    SetFlagBit(x, z, kFlagOddHidden, hide);
}
void cFC4STETerrain::HideEvenTriangle(uint32_t x, uint32_t z, bool hide) {
    SetFlagBit(x, z, kFlagEvenHidden, hide);
}
void cFC4STETerrain::HideOddTriangle(uint32_t x, uint32_t z, bool hide) {
    SetFlagBit(x, z, kFlagOddHidden, hide);
}
bool cFC4STETerrain::IsEvenTriangleHidden(uint32_t x, uint32_t z) const {
    return (VertexFlags(x, z) & kFlagEvenHidden) != 0;
}
bool cFC4STETerrain::IsOddTriangleHidden(uint32_t x, uint32_t z) const {
    return (VertexFlags(x, z) & kFlagOddHidden) != 0;
}
bool cFC4STETerrain::IsWholeCellHidden(uint32_t x, uint32_t z) const {
    const uint16_t f = VertexFlags(x, z);
    return (f & (kFlagEvenHidden | kFlagOddHidden)) == (kFlagEvenHidden | kFlagOddHidden);
}

bool cFC4STETerrain::IsPointHidden(float x, float z) const {
    // §7 lists the hide slots together but gives no point rule. The literal
    // reading: the whole cell the point falls in. [choice]
    const int32_t cx = static_cast<int32_t>(x / 16.0f);
    const int32_t cz = static_cast<int32_t>(z / 16.0f);
    return IsWholeCellHidden(static_cast<uint32_t>(cx < 0 ? 0 : cx), static_cast<uint32_t>(cz < 0 ? 0 : cz));
}

bool cFC4STETerrain::SetTerrainCellFlags(uint32_t x, uint32_t z, uint32_t flags) {
    // §7: cell flags 1, 2, 4, 8 pack as 0x20, 0x40, 0x200, 0x2000.
    fc4_terrain* core = g_activeCore.load(std::memory_order_acquire);
    if (core == nullptr) return false;
    uint16_t f = fc4_terrain_vertex_flags(core, static_cast<int32_t>(x), static_cast<int32_t>(z));
    f = static_cast<uint16_t>(f & ~(kFlagEvenHidden | kFlagOddHidden | kFlagCell4 | kFlagFlipExternal));
    if ((flags & 1u) != 0) f = static_cast<uint16_t>(f | kFlagEvenHidden);
    if ((flags & 2u) != 0) f = static_cast<uint16_t>(f | kFlagOddHidden);
    if ((flags & 4u) != 0) f = static_cast<uint16_t>(f | kFlagCell4);
    if ((flags & 8u) != 0) f = static_cast<uint16_t>(f | kFlagFlipExternal);
    fc4_terrain_set_vertex_flags(core, static_cast<int32_t>(x), static_cast<int32_t>(z), f);
    return true;
}

uint32_t cFC4STETerrain::GetTerrainCellFlags(uint32_t x, uint32_t z) {
    const uint16_t f = VertexFlags(x, z);
    uint32_t out = 0;
    if ((f & kFlagEvenHidden) != 0) out |= 1u;
    if ((f & kFlagOddHidden) != 0) out |= 2u;
    if ((f & kFlagCell4) != 0) out |= 4u;
    if ((f & kFlagFlipExternal) != 0) out |= 8u;
    return out;
}

bool cFC4STETerrain::FlipInternalTriangulation(uint32_t x, uint32_t z, bool flip) {
    SetFlagBit(x, z, kFlagFlipInternal, flip);
    return true;
}
bool cFC4STETerrain::FlipExternalTriangulation(uint32_t x, uint32_t z, bool flip) {
    SetFlagBit(x, z, kFlagFlipExternal, flip);
    return true;
}

// ---- display state (§8.4) -------------------------------------------------------------------------

bool cFC4STETerrain::GetWaterSurfaceDisplayState() {
    return core_ != nullptr && fc4_terrain_water_display_state(core_) != 0;
}

void cFC4STETerrain::SetWaterSurfaceDisplayState(bool on) {
    Enter();
    if (core_ != nullptr) fc4_terrain_set_water_display_state(core_, on ? 1 : 0);  // §8.4
    Leave();
}

// ---- slot 91, the deleting destructor -------------------------------------------------------------

void cFC4STETerrain::DeletingDestructor(uint32_t flags) {
    // GZCOM releases through Release() (slot 2), so this should never run; if
    // the game does call its deleting-destructor slot, destroy as it expects.
    if ((flags & 1u) != 0) Release();
}

// ---- persistence (§13) ----------------------------------------------------------------------------

bool cFC4STETerrain::Load(cIGZPersistDBSegment*) {
    // §13.1: the misc, altitude and vertex property records (the edges are
    // read on demand by the region code). The plugin's stream layer does not
    // yet reach a DB segment, so the records cannot arrive here; Init builds
    // the terrain from the city grid instead and the water is rebuilt from the
    // sea level either way (§10.1; quirk 15: the sea level is not saved).
    Log("terrain: Load (records) is not wired; starting from the city grid");
    return false;
}

bool cFC4STETerrain::Save(cIGZPersistDBSegment*) {
    Log("terrain: Save (records) is not wired");
    return false;
}

}  // namespace fc4

// The factories return the primary interface's pointer: the director's
// QueryInterface takes the first reference.
cIGZUnknown* CreateFC4Terrain() {
    auto* t = new fc4::cFC4STETerrain();
    return t->Primary();
}

cIGZUnknown* CreateFC4SurfaceWater() {
    // The water is a stateless view over the one fc4_terrain, so it does not
    // matter which of the two the city constructs. A second object is fine.
    fc4::Log("terrain: creating cSTESurfaceWater (enable = terrain): a view over the shared fc4_terrain handle");
    auto* w = new fc4::cFC4STESurfaceWater();
    return w->Primary();
}

// ---- test seam op 59 ------------------------------------------------------------------------------

namespace {
// An initialized native-compatible view fixture. Calls use the real x86
// receiver/stack contract; no native code or invalid native draw is executed.
struct TerrainViewFixture {
    void** baseVtable;
    uint32_t reserved04[2]{};
    void** drawingVtable;
    uint32_t reserved10[2]{};
    uint8_t initialized = 0;
    uint8_t reserved19[3]{};
    void* city = nullptr;
    fc4::cFC4ISTETerrain* terrain = nullptr;
    uint32_t initCalls = 0, shutdownCalls = 0, releases = 0, draws = 0, errors = 0;
    uint16_t drawnFlags = 0;
    bool visible = true, failInit = false;
    uint32_t staticInitCalls = 0;
    bool staticReady = false, failStaticInit = false;
};
static_assert(offsetof(TerrainViewFixture, drawingVtable) == 0x0C, "NM20 receiver");
static_assert(offsetof(TerrainViewFixture, terrain) == 0x20, "NM20 retained primary terrain");

TerrainViewFixture* g_terrainStaticFixture = nullptr;
bool __cdecl TerrainFixtureStaticInit() {
    auto* view = g_terrainStaticFixture;
    ++view->staticInitCalls;
    if (view->failStaticInit) return false;
    view->staticReady = true;
    return true;
}

bool __fastcall TerrainFixtureQuery(TerrainViewFixture*, void*, uint32_t, void** out) {
    if (out != nullptr) *out = nullptr;
    return false;
}

uint32_t __fastcall TerrainFixtureRelease(TerrainViewFixture* view, void*) {
    ++view->releases;
    if (view->terrain != nullptr || view->initialized != 0) ++view->errors;
    return 0;
}

bool __fastcall TerrainFixtureInit(TerrainViewFixture* view, void*, fc4::cFC4ISTETerrain* terrain) {
    ++view->initCalls;
    auto* t = static_cast<fc4::cFC4STETerrain*>(terrain);
    if (!view->staticReady || t->f_.viewReady != 0 || t->f_.vertices == nullptr) ++view->errors;
    if (view->failInit) return false;
    if (reinterpret_cast<void*>(terrain) != t->Primary()) ++view->errors;
    view->terrain = terrain;
    terrain->AddRef();
    view->initialized = 1;
    return true;
}

void __fastcall TerrainFixtureShutdown(TerrainViewFixture* view, void*) {
    ++view->shutdownCalls;
    auto* t = static_cast<fc4::cFC4STETerrain*>(view->terrain);
    if (t == nullptr || t->Core() == nullptr || t->f_.vertices == nullptr || t->f_.viewReady != 0 ||
        t->GetView() != nullptr) ++view->errors;
    view->terrain->Release();
    view->terrain = nullptr;
    view->initialized = 0;
}

void __fastcall TerrainFixtureDraw(void* receiver, void*, void* context, uint32_t x, uint32_t z,
                                    float* uv, float alpha, bool force) {
    auto* view = reinterpret_cast<TerrainViewFixture*>(static_cast<uint8_t*>(receiver) - 0x0C);
    if (*reinterpret_cast<void**>(static_cast<uint8_t*>(context) + 0x4C) != receiver ||
        x != 44 || z != 3 || uv[6] != 1.0f || alpha != 1.0f) ++view->errors;
    if (!view->visible && !force) return;
    if (view->terrain == nullptr || view->initialized == 0) {
        ++view->errors;
        return;
    }
    // NM20: +0x6C and flags at vertex +0x22, stride 0x24, through the
    // retained primary terrain. This would fail on the pre-fix lifecycle.
    auto* t = static_cast<fc4::cFC4STETerrain*>(view->terrain);
    auto* vertices = *reinterpret_cast<fc4::TerrainDrawVertex**>(reinterpret_cast<uint8_t*>(view->terrain) + 0x6C);
    if (vertices != t->f_.vertices || vertices == nullptr || t->f_.viewReady != 1) ++view->errors;
    else view->drawnFlags = vertices[x + z * (t->CellCountX() + 1)].flags;
    ++view->draws;
}
}  // namespace

intptr_t TerrainGlueTest(FC4TerrainGlueTestInput* in) {
    using namespace fc4;
    if (in == nullptr || in->struct_size < sizeof(FC4TerrainGlueTestInput)) return -1;
    in->slotsChecked = 0;
    in->reentrantGuardRefused = 0;
    in->coreReentrantRefused = 0;
    in->postsSeen = 0;
    in->brushLevel = 0;
    in->mismatches = 0;

    auto* earlyWater = new cFC4STESurfaceWater();
    earlyWater->AddRef();
    auto* t = new cFC4STETerrain();
    t->AddRef();
    t->Host()->testMode = true;
    if (!t->Init()) {
        t->Release();
        earlyWater->Release();
        return -1;
    }

    fc4_terrain* core = t->Core();
    if (core != nullptr) {
        // A few slots that the consumers' list (§15.3) depends on.
        auto check = [&](bool ok) {
            ++in->slotsChecked;
            if (!ok) ++in->mismatches;
        };
        check(t->CellWidth() == 16.0f);  // abi: via cFC4ISTETerrain (float, not the SDK's uint32_t)
        check(t->GetAltitudeAt(0.0f, 0.0f) >= 0.0f);
        check(t->GetSeaLevel() > 0.0f);
        check(t->MinAllowedHeight() < t->MaxAllowedHeight());
        check(t->GetDefaultTerrainRaiseLowerAmount() == 20.0f);
        check(t->LocationIsInBounds(0.0f, 0.0f));
        check(!t->LocationIsInBounds(-1.0f, 0.0f));
        check(t->CellCountX() == t->CellCountX());

        // The queued native heights/water readers unpack data1/data2, not
        // the sender pointer. Inspect an older post after making another one.
        fc4_terrain_post post{};
        post.msg_type = 0x278E6003u;
        post.data1 = (3u << 16) | 44u;
        post.data2 = (2u << 16) | 30u;
        post.data3 = 0x00140020u;
        post.data4 = 7;
        auto* first = MakeTerrainPost(post, t->Primary());
        post.msg_type = 0x678E6E3Au;
        post.data1 = (1u << 16) | 33u;
        auto* second = MakeTerrainPost(post, t->Primary());
        check(first != nullptr && second != nullptr && first != second);
        if (first != nullptr && second != nullptr) {
            check(first->GetType() == 0x278E6003u && first->GetData1() == ((3u << 16) | 44u));
            check(first->GetData2() == post.data2 && first->GetData3() == post.data3 &&
                  first->GetData4() == post.data4);
            check(first->GetIGZUnknown() == t->Primary() && second->GetIGZUnknown() == t->Primary());
            check(second->GetType() == post.msg_type && second->GetData1() == post.data1);
        }
        if (first != nullptr) first->Release();
        if (second != nullptr) second->Release();

        // Slot 66 must preserve null and select the view sub-interface,
        // rather than handing consumers the five-slot object-base vtable.
        check(t->GetView() == nullptr);
        unsigned char viewLayout[0x10]{};
        t->f_.view = viewLayout;
        check(reinterpret_cast<void*>(t->GetView()) == viewLayout + 0x0C);
        t->f_.view = nullptr;  // borrowed test storage, never Release it

        // TR18: a host callback from inside a brush must be refused.
        t->Host()->inside = 1;
        (void)HostRandomUniform(t->Host(), 5);
        (void)HostBridgeAt(t->Host(), 0, 0, nullptr);
        (void)HostPowerPole(t->Host(), 0, 0);
        (void)HostLotAtCell(t->Host(), 0, 0);
        t->Host()->inside = 0;

        // A change: one message posted per RedisplayTerrain (§8.1 posts three,
        // of which the simulation reads 0x278E6003 / 0x678E6E3A).
        t->DoPostCityInit();
        check(t->f_.postCityInit == 1);
        // A second post must not repeat core/view initialization. A distinct
        // field value makes an unintended core default-level write visible.
        const int32_t originalLevel = fc4_terrain_default_brush_level(core);
        t->f_.defaultBrushLevel = originalLevel + 1;
        t->DoPostCityInit();
        check(fc4_terrain_default_brush_level(core) == originalLevel);
        t->f_.defaultBrushLevel = originalLevel;

        // Exercise the actual message-target/QI path. Data1 deliberately
        // disagrees with the pointed-to zoom; neither value nor pointer is
        // owned by the terrain. An unaligned payload is also legal here.
        auto* zoomMessage = cFC4Message2Standard::New();
        zoomMessage->AddRef();
        zoomMessage->SetType(kMsgViewUpdate);
        zoomMessage->SetData1(1);
        unsigned char payload[sizeof(uint32_t) + 1]{};
        zoomMessage->SetVoid2(payload + 1);
        for (uint32_t zoom : {3u, 3u, 7u, 0u}) {
            std::memcpy(payload + 1, &zoom, sizeof(zoom));
            check(t->Target()->DoMessage(zoomMessage));
            check(static_cast<uint32_t>(t->f_.defaultBrushLevel) == zoom);
            check(static_cast<uint32_t>(fc4_terrain_default_brush_level(core)) == zoom);
        }
        zoomMessage->SetVoid2(nullptr);
        zoomMessage->SetData1(4);
        check(t->Target()->DoMessage(zoomMessage));
        check(t->f_.defaultBrushLevel == 0);
        check(fc4_terrain_default_brush_level(core) == 0);
        // §12.1: view-less ticks do not start reconciliation. Once a view
        // exists, only its second tick posts; repeated PostCityInit/ticks
        // must neither reset nor wrap the once-per-object counter.
        zoomMessage->SetType(kMsgAnimationTick);
        check(t->Target()->DoMessage(zoomMessage));
        check(t->f_.reconcileTick == 0);
        t->f_.view = viewLayout;
        check(t->Target()->DoMessage(zoomMessage));
        check(t->f_.reconcileTick == 1);
        check(t->Target()->DoMessage(zoomMessage));
        check(t->f_.reconcileTick == 2);
        t->DoPostCityInit();
        for (int i = 0; i < 4; ++i) check(t->Target()->DoMessage(zoomMessage));
        check(t->f_.reconcileTick == 2);
        t->f_.view = nullptr;  // borrowed layout; never release it
        zoomMessage->Release();
        SC4Rect<int32_t> r(0, 0, 1, 1);
        t->SetAltitudeRect(r, 123.0f, true, 3);

        fc4_terrain_state st{};
        st.struct_size = sizeof(st);
        if (fc4_terrain_get_state(core, &st)) {
            in->coreReentrantRefused = st.reentrant_refused;
            in->postsSeen = st.posts_pending;
        }
        in->brushLevel = static_cast<uint32_t>(fc4_terrain_default_brush_level(core));
    }

    in->reentrantGuardRefused = t->Host()->guardRefused.load(std::memory_order_relaxed);
    // terrain-view.md §4.4.1 / lot-models.md §4: the cell model borrows the
    // drawing sub-interface from context +0x4C, including forced/offscreen draws.
    void* baseVtable[3]{reinterpret_cast<void*>(&TerrainFixtureQuery), nullptr,
                       reinterpret_cast<void*>(&TerrainFixtureRelease)};
    void* drawingVtable[16]{};
    drawingVtable[15] = reinterpret_cast<void*>(&TerrainFixtureDraw);
    TerrainViewFixture view{};
    g_terrainStaticFixture = &view;
    view.baseVtable = baseVtable;
    view.drawingVtable = drawingVtable;
    t->f_.view = &view;
    auto checkView = [&](bool ok) { ++in->slotsChecked; if (!ok) ++in->mismatches; };
    const auto initView = reinterpret_cast<cFC4STETerrain::ViewInit>(&TerrainFixtureInit);
    const auto stopView = reinterpret_cast<cFC4STETerrain::ViewShutdown>(&TerrainFixtureShutdown);
    const auto initStatics = &TerrainFixtureStaticInit;
    checkView(!t->InitView(nullptr, initView, stopView));
    checkView(view.initCalls == 0 && view.staticInitCalls == 0 && t->f_.viewReady == 0);
    view.failStaticInit = true;
    checkView(!t->InitView(initStatics, initView, stopView));
    checkView(view.initCalls == 0 && view.staticInitCalls == 1 && t->f_.viewReady == 0);
    view.failStaticInit = false;
    view.failInit = true;
    checkView(!t->InitView(initStatics, initView, stopView));
    checkView(t->f_.viewReady == 0 && view.terrain == nullptr);
    view.failInit = false;
    checkView(t->InitView(initStatics, initView, stopView));
    checkView(t->GetView() == reinterpret_cast<cISTETerrainView*>(reinterpret_cast<uint8_t*>(&view) + 0x0C));
    checkView(t->InitView(initStatics, initView, stopView) && view.initCalls == 2 && view.staticInitCalls == 3);
    checkView(t->AddRef() == 3);  // caller + initialized view + this probe
    t->Release();
    t->SetAltitudeXZ(44, 3, 275.0f);
    auto* vertices = static_cast<TerrainDrawVertex*>(t->f_.vertices);
    const uint32_t index = 44 + 3 * (t->CellCountX() + 1);
    checkView(vertices[index].altitude == 275.0f);
    t->FlipInternalTriangulation(44, 3, true);
    // A native view may write the levelling bits directly; the next sync must retain them.
    vertices[index].flags |= 0x60;
    t->SetAltitudeXZ(44, 3, 276.0f);
    checkView((vertices[index].flags & 0x60) == 0x60);
    auto* water = static_cast<uint8_t*>(t->f_.surfaceWater);
    auto* waterVertices = *reinterpret_cast<WaterDrawVertex**>(water + 0x18);
    checkView(waterVertices != nullptr && waterVertices[index].depth == t->GetWaterDepthAtFlatIndex(index));
    auto* lateWater = new cFC4STESurfaceWater();
    lateWater->AddRef();
    checkView(earlyWater->vertices_ == waterVertices && lateWater->vertices_ == waterVertices);
    uint8_t context[0x50]{};
    *reinterpret_cast<void**>(context + 0x4C) = t->GetView();
    float uv[8]{0, 0, 1, 0, 0, 1, 1, 1};
    using Draw = void(__thiscall*)(void*, void*, uint32_t, uint32_t, float*, float, bool);
    auto draw = reinterpret_cast<Draw>(drawingVtable[15]);
    draw(t->GetView(), context, 44, 3, uv, 1.0f, false);
    checkView(view.draws == 1 && (view.drawnFlags & 0x1000) != 0);
    view.visible = false;
    draw(t->GetView(), context, 44, 3, uv, 1.0f, false);
    checkView(view.draws == 1);
    draw(t->GetView(), context, 44, 3, uv, 1.0f, true);
    checkView(view.draws == 2 && (view.drawnFlags & 0x1000) != 0);
    t->Shutdown();
    checkView(view.shutdownCalls == 1 && view.releases == 1 && view.errors == 0);
    g_terrainStaticFixture = nullptr;
    checkView(t->GetView() == nullptr && t->f_.vertices == nullptr);
    checkView(earlyWater->vertices_ == nullptr && lateWater->vertices_ == nullptr);
    checkView(t->AddRef() == 2);  // view returned its retained reference
    t->Release();
    earlyWater->Release();
    lateWater->Release();
    t->Shutdown();
    checkView(view.shutdownCalls == 1 && view.releases == 1);
    t->Release();
    return static_cast<intptr_t>(in->mismatches);
}
