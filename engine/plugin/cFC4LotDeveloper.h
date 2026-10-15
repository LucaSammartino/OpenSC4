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

// The lot developer's GZCOM class (docs/specs/lot-developer.md §1, §11; CLSID
// 0xA990BFE0, city component 16). The model lives in the Rust core behind
// fc4.h (fc4_lotdev); this class implements the three interfaces of §11 in
// the compiled slot order and return channels of §11.1
// (FC4LotDevInterfaces.h cFC4ISC4LotDeveloper) and the host callbacks over
// the game's services (§14, FC4LotDevHost.cpp). Slots 38, 41, 45, 62, 63 and
// 64 are not the core's (fc4.h): 38, 41, 45 and 62 answer through the host
// callbacks (first_building, base_texture_occupant,
// update_lot_base_texture_occupant, update_lot_retaining_walls), 63 and 64
// are made here (§15.5).
//
// One object, three interfaces, one reference count: the primary interface
// (cISC4LotDeveloper 0x4818E5D2, also cIGZUnknown), cIGZMessageTarget2 (+4:
// the agent, the subscriptions and the animation tick, §3) and
// cIGZSerializable (+8, §13). Which sub-object answers an IID is the core's
// decision (fc4_lotdev_query_interface).
//
// Threads. Everything the lot developer does runs on the main thread (the
// simulator's agents, the message server, the animation tick, the BDS and
// the tract developer). The core is single-threaded, so every call into it
// holds this object's lock (a recursive critical section: the BDS or the
// tract developer may call back in while the core runs; the core queues
// messages and refuses other calls then, fc4.h). [choice] A thread other
// than the main one waits for the lock as well: no slot of this class is a
// getter the UI polls.
//
// The subsystem is OFF by default: FreeCity4.ini `enable = lotdev`; `disable`
// still wins. The lotdev_* switches (FC4Config.h) turn groups of game calls
// off for bisecting.
#pragma once

#include "FC4Config.h"
#include "FC4LotDevInterfaces.h"
#include "FC4TestSeam.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <atomic>
#include <cstdint>
#include <vector>

class cIGZMessageServer2;
class cISC4BuildingDevelopmentSimulator;
class cISC4City;
class cISC4Demolition;
class cISC4FloraSimulator;
class cISC4LotConfigurationManager;
class cISC4LotManager;
class cISC4OccupantManager;
class cISC4PlumbingSimulator;
class cISC4PowerSimulator;
class cISC4PropManager;
class cISC4TrafficSimulator;
class cISTETerrain;

namespace fc4 {

// One safe lot iterator handed to the core (fc4.h safe_lot_iterator): the
// game's block (lot manager slot 15, walked raw as the original does, §15.1)
// on the identified executable with the original lot manager, otherwise a
// list of the lots (GetLotsInCellRect over the city, one reference each)
// taken at creation and at every rewind.
struct LotDevIterator {
    void* block = nullptr;            // the game's {current node, list object}, or null
    std::vector<cIGZUnknown*> lots;   // the fallback's lots (one reference each)
    size_t next = 0;                  // the fallback's cursor
    bool faulted = false;             // a raw step faulted: the iterator answers "end" from then on
};

// What the lot developer's host callbacks work on. The lot developer owns
// one; the lot developer shadow (FC4LotDevShadow.h) fills one with a city's
// borrowed services. Main thread, holding the lot developer's lock.
struct LotDevContext {
    bool readOnly = false;  // the shadow: borrowed services, read callbacks only
    bool gated = true;      // false: the lotdev_* switches are ignored (the shadow reads as it is)
    // §1.3 step 3: held from Init to Shutdown by the lot developer (one
    // reference each), borrowed by the shadow.
    cISC4City* city = nullptr;
    cISC4LotManager* lots = nullptr;
    cISC4OccupantManager* occupants = nullptr;
    cISC4PropManager* props = nullptr;
    cISC4BuildingDevelopmentSimulator* bds = nullptr;
    cISC4PowerSimulator* power = nullptr;
    cISC4PlumbingSimulator* plumbing = nullptr;
    cISC4FloraSimulator* flora = nullptr;
    cISTETerrain* terrain = nullptr;
    cISC4TrafficSimulator* traffic = nullptr;
    cISC4Demolition* demolition = nullptr;
    cIGZMessageServer2* msgServer = nullptr;  // lot developer only (§1.3 step 9)
    cIGZMessageTarget2* target = nullptr;     // lot developer only: this + 4
    uint32_t agentLevels = 0;                 // AddAgent calls made since the last RemoveAgent
    bool tickRegistered = false;              // animation-tick manager slot 5 made (§1.3 step 10)
    // Test seam op 52: a table whose non-NULL callbacks answer instead of the
    // game (its own `user`).
    const fc4_lotdev_host* test = nullptr;
    // References taken for one top-level call into the core (the occupants
    // and lots the game's lists gave, created objects, message objects),
    // released when it returns. `created`: an object made for the call whose
    // last reference may be this one (its free is expected).
    struct Held {
        cIGZUnknown* p;
        bool created;
    };
    std::vector<Held> held;
    // The identities (cIGZUnknown 0x00000001) of the objects removed,
    // demolished or deleted during the call: their held references may be
    // the last ones. `expireAll`: a bulk removal (a lot's props, a building's
    // props, DemolishLots, flora in an area) ran; every held reference may be.
    std::vector<const void*> expired;
    bool expireAll = false;
    // The iterators handed to the core and not yet freed.
    std::vector<LotDevIterator*> iterators;
    // lotdev_lot_log: the lot the daily pass is updating and its state,
    // condition and zot before (one line per lot that changed).
    bool lotLog = false;
    uint64_t logLot = 0;
    uint32_t logState = 0, logCondition = 0, logZot = 0;
    // Observability (the status lines): game calls made since construction.
    uint32_t buildingsAdded = 0, buildingsFailed = 0, constructionOccupants = 0, foundationsAdded = 0;
    uint32_t propsAdded = 0, floraAdded = 0, texturesMade = 0, texturesSet = 0, occupantsRemoved = 0;
    uint32_t demolished = 0, lotsDeleted = 0, terrainWrites = 0, unitsMade = 0, lightsSet = 0;
    uint32_t listsRefused = 0, iteratorFallbacks = 0, iteratorFaults = 0, posts = 0;
    // Lot content calls (§6.5): ID arrays asked / empty, flora types valid / not, family sets asked / empty,
    // flora and prop adds asked / answered null.
    uint32_t idArrays = 0, idArraysEmpty = 0, floraValid = 0, floraInvalid = 0, floraFamilies = 0,
             floraFamiliesEmpty = 0, floraAdds = 0, floraAddsNull = 0, propAdds = 0, propAddsNull = 0;
};

// The host table over `ctx` (the lot developer's).
fc4_lotdev_host MakeLotDevHost(LotDevContext* ctx);
// The read-only table for the shadow: every callback that would change the
// game is a no-op answering the neutral value (the shadow core reaches only
// Init's reads and the geometry slots), has_simulator answers false (Init
// stops at §1.3 step 8: no agent, no subscription, no texture walk).
fc4_lotdev_host MakeLotDevReadOnlyHost(LotDevContext* ctx);
// Keeps one reference of `p` (already taken) until the outermost call returns.
void LotDevHold(LotDevContext* ctx, cIGZUnknown* p, bool created = false);
// Notes that `p` (any interface) was removed, demolished or deleted.
void LotDevExpire(LotDevContext* ctx, cIGZUnknown* p);
// Releases the references `ctx` keeps for one top-level call.
void LotDevReleaseHeld(LotDevContext* ctx);
// The services `ctx` holds (the Shutdown path, or a destructor after a
// failed Init); a read-only context holds none. Also frees iterators left.
void LotDevReleaseServices(LotDevContext* ctx);
// The lotdev_lot_log line of the lot the daily pass updated last, if any.
void LotDevFlushLotLog(LotDevContext* ctx);
// Slots 38 and 41 (host-side helpers the class calls directly).
cISC4BuildingOccupant* LotDevFirstBuilding(LotDevContext* ctx, cISC4Lot* lot);
cISC4Occupant* LotDevBaseTextureOccupant(LotDevContext* ctx, cISC4Lot* lot, bool create);
// A configuration object as fc4.h has it (`ids` holds its ID array; the
// result points into it), and the object at `index` of a configuration's
// array (slot 23), or null.
fc4_lotdev_cfg_object LotDevObjectOf(cSC4LotConfigurationObject* o, std::vector<uint32_t>& ids);
cSC4LotConfigurationObject* LotDevObjectAt(cISC4LotConfiguration* cfg, uint32_t index);

// The lot developer the city runs, if it is FreeCity4's (null otherwise);
// main thread. The observer's month hook writes its month line.
void LotDevMonthLine(int32_t julian);

// The process's class statics (§10.1; fc4.h: one per process), created on
// first use with the session seed (lotdev_seed / FC4_LOTDEV_SEED, else the
// cRZTime clock). `fixed` (optional): whether the seed came from the setting.
const fc4_lotdev_statics* LotDevStatics(uint32_t* seed = nullptr, bool* fixed = nullptr);

}  // namespace fc4

class cFC4LotDeveloper final : public cFC4ISC4LotDeveloper, public cIGZMessageTarget2, public cIGZSerializable {
public:
    cFC4LotDeveloper();
    ~cFC4LotDeveloper();

    // cIGZUnknown (all three interfaces)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4LotDeveloper as compiled (§11.1)
    bool Init() override;
    bool Shutdown() override;
    uint32_t GetSimulatorType() override;
    bool StartConstruction(cISC4Lot* lot, cISC4LotConfiguration* cfg, uint32_t type, bool animate) override;
    bool ConstructDefaultLot(cISC4Lot* lot, cISC4LotConfiguration* cfg, uint32_t type, bool animate) override;
    bool AbandonBuilding(cISC4Lot* lot, int32_t reason) override;
    bool ReoccupyBuilding(cISC4Lot* lot, int32_t devType) override;
    bool ChangeBuilding(cISC4Lot* lot, uint32_t type) override;
    bool RemoveBuildingZotProp(cISC4Lot* lot) override;
    bool StartDemolishLot(cISC4Lot* lot) override;
    bool EndDemolishLot(cISC4Lot* lot) override;
    bool ExplodeLot(cISC4Lot* lot, bool burnt) override;
    float GetTerrainAltitudeByVertex(int32_t x, int32_t z, bool withWater) override;
    float GetAreaAltitudeF(SC4Rect<float> const& rect, bool lowest, bool withWater) override;
    float GetAreaAltitudeI(SC4Rect<int32_t> const& rect, bool lowest, bool withWater) override;
    float GetAverageAltitude(SC4Rect<int32_t> const& rect, bool withWater) override;
    int32_t LevelInternalVertices(SC4Rect<int32_t> const& rect, float target, bool skipWater, bool force,
                                  cISC4Lot* lot) override;
    int32_t LevelBorderVertices(SC4Rect<int32_t> const& rect, float h, bool unused, cISC4Lot* lot) override;
    int32_t AttemptLevelling(SC4Rect<int32_t> const& rect, float unused, bool withWater, bool perVertex,
                             float maxAreaSlope, cISC4Lot* lot) override;
    bool GetOccupantCellCoverage(cISC4Occupant* occupant, SC4Rect<int32_t>& rect) override;
    bool IsAreaDevelopedStub(SC4Rect<int32_t> const& rect) override;
    bool IsLotDeveloped(cISC4Lot* lot) override;
    bool IsAreaFlat(SC4Rect<int32_t> const& rect, bool withWater) override;
    bool IsLotFlat(cISC4Lot* lot, bool withWater) override;
    float GetAreaAltitudeRange(SC4Rect<int32_t> const& rect, bool withWater) override;
    float GetLotAltitudeRange(cISC4Lot* lot, bool withWater) override;
    float GetAreaSlope(SC4Rect<int32_t> const& rect, bool withWater) override;
    float GetLotSlope(cISC4Lot* lot, bool withWater) override;
    float GetOccupantSlope(cISC4Occupant* occupant, bool withWater) override;
    int64_t GetLotDevelopmentCost(SC4Rect<int32_t> const& rect) override;
    bool RemoveLotFromItsBusinessUnit(cISC4Lot* lot) override;
    bool UpdateTravelDesignates(cISC4BusinessUnit*& unit) override;
    bool UpdateTravelPopulations(cISC4BusinessUnit* unit) override;
    bool NeedsAnchorBuilding(SC4Rect<int32_t> const& rect, int32_t purpose) override;
    bool UpdateLotElements(cISC4Lot* lot, uint32_t kind) override;
    cISC4BuildingOccupant* GetFirstBuilding(cISC4Lot* lot) override;
    int32_t* GetObjectPosition(cISC4Lot* lot, cISC4LotConfiguration* cfg, cSC4LotConfigurationObject* obj, float* pos,
                               int32_t& orient) override;
    int32_t* GetBuildingPosition(cISC4Lot* lot, float* pos, int32_t& orient) override;
    bool GetLotBaseTextureOccupant(cISC4Lot* lot, cISC4LotBaseTextureOccupant*& occ, bool create) override;
    intptr_t GetBoundaryTexture(int32_t* sides, uint32_t& texture, uint8_t& rotation, bool& flip) override;
    void SetEmptyLotBaseTexture(cISC4Lot* lot) override;
    cISC4Lot* SetNormalLotBaseTexture(cISC4Lot* lot, cISC4LotConfiguration* cfg, bool flag) override;
    bool UpdateLotBaseTextureOccupant(cISC4Lot* lot, bool hadFoundation) override;
    bool GetForceEmptyLotBaseTextures() override;
    void SetForceEmptyLotBaseTextures(bool force) override;
    cISC4FoundationOccupant* GetFoundationByOccupant(cISC4Occupant* occupant) override;
    cISC4FoundationOccupant* GetFoundationByCell(int32_t kind, int32_t x, int32_t z) override;
    cISC4FoundationOccupant* GetFoundationByPosition(int32_t kind, float x, float z) override;
    cISC4FoundationOccupant* GetLotFoundation(cISC4Lot* lot) override;
    bool RemoveFoundation(int32_t kind, float x, float z) override;
    bool RemoveLotFoundationAt(int32_t x, int32_t z) override;
    bool AddLotFoundationCfg(cISC4Lot* lot, cISC4LotConfiguration* cfg, bool useLotAltitude, bool level,
                             bool walls) override;
    bool AddLotFoundationType(cISC4Lot* lot, uint32_t type, bool useLotAltitude, bool level, bool walls) override;
    bool IsOccupantFoundationRequired(cISC4Occupant* occupant) override;
    bool IsBoxFoundationRequired(float* min, float* max) override;
    bool AddBuildingFoundationCfg(cISC4BuildingOccupant* building, cISC4LotConfiguration* cfg) override;
    bool AddBuildingFoundationType(cISC4BuildingOccupant* building, uint32_t type) override;
    bool RemoveBuildingFoundation(cISC4Occupant* occupant) override;
    bool UpdateLotFoundation(cISC4Lot* lot) override;
    bool UpdateLotRetainingWalls(cISC4Lot* lot) override;
    cISC4Occupant* CreateLotProp(cISC4Lot* lot, uint32_t objectIndex, uint32_t propID) override;
    bool GetLotConfigurationObjectOccupant(cISC4Lot* lot, uint32_t objectIndex, cISC4Occupant*& out) override;

    // cIGZMessageTarget2 (§3)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§13)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // The month status line (the observer's SimNewMonth; main thread).
    void LogMonth(int32_t julian);
    // Test seam op 52 (FC4TestSeam.h): the instance one of whose interfaces
    // is `p`, or null.
    static cFC4LotDeveloper* FromPrimary(const void* p);
    void SetTestHooks(const fc4_lotdev_host* test);
    // The core's state (for the test seam); false without a core.
    bool State(fc4_lotdev_state* out) const;
    const fc4::LotDevContext& Context() const { return ctx_; }

private:
    // Holds the lock for one call into the core. At the outermost section
    // the references kept for the call are released.
    class Section {
    public:
        explicit Section(const cFC4LotDeveloper* s);
        ~Section();
        Section(const Section&) = delete;
        Section& operator=(const Section&) = delete;

    private:
        cFC4LotDeveloper* s_;
    };

    // The month line's counters at the previous line.
    struct MonthBase {
        uint32_t visited = 0, abandoned = 0, restored = 0, conditions = 0, zots = 0;
        uint32_t started = 0, finished = 0, demolitions = 0, nightBatches = 0, refused = 0, queued = 0;
        uint32_t buildingsAdded = 0, propsAdded = 0, floraAdded = 0, texturesSet = 0, removed = 0, lights = 0;
        uint32_t days = 0, dayRedeliveries = 0;
    };

    void LogState(const char* what);
    void CreateCore();
    // fc4_cell_rect from the game's rectangle {x1, z1, x2, z2}.
    static fc4_cell_rect Cells(SC4Rect<int32_t> const& r);

    cIGZUnknown* Primary() { return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4LotDeveloper*>(this)); }

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::LotDevContext ctx_;
    fc4_lotdev_host host_{};
    fc4_lotdev* core_ = nullptr;
    uint32_t seed_ = 0;
    bool seedFixed_ = false;
    // Day messages delivered, and those answered false (re-delivered).
    uint32_t days_ = 0, dayRedeliveries_ = 0;

    mutable CRITICAL_SECTION lock_;
    int depth_ = 0;  // nesting of sections held
    MonthBase month_;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4LotDeveloper();

// Test seam op 52 (FC4TestSeam.h, kFC4SeamLotDev): `a` is one of these.
struct FC4LotDevTestInput {
    uint32_t op;  // kFC4LotDevTestHooks, kFC4LotDevTestShadow or kFC4LotDevTestState
    // kFC4LotDevTestHooks: on `lotDeveloper` (FreeCity4's, any of its
    // interfaces), every non-NULL callback of `host` answers instead of the
    // game (with host->user); the game containers (B: object ID arrays, hash
    // sets, lists) come from `allocate` / `free`. The core is created anew
    // (call it before Init / Read). A null `lotDeveloper` sets the allocator
    // only (null: B again).
    cIGZUnknown* lotDeveloper;
    const fc4_lotdev_host* host;
    void* (*allocate)(uint32_t size);
    void (*free)(void* p);
    // kFC4LotDevTestShadow: one shadow comparison (FC4LotDevShadow.h) on
    // `city`, whose lot developer is the "original": event 0 = PostCityInit,
    // 1 = a month; `host` (may be null) answers the shadow core's host calls.
    cISC4City* city;
    int32_t julian;
    uint32_t event;
};
enum : uint32_t { kFC4LotDevTestHooks = 1, kFC4LotDevTestShadow = 2, kFC4LotDevTestState = 3 };
// kFC4LotDevTestState's answer: the core's state and the host's counters.
struct FC4LotDevTestState {
    fc4_lotdev_state core;  // struct_size set by the seam
    uint32_t agentLevels, tickRegistered, held, iterators;
    uint32_t buildingsAdded, propsAdded, texturesSet, occupantsRemoved, unitsMade, lightsSet, posts;
};
// Op 52's body: kFC4LotDevTestHooks returns 1 (installed), 0 (the allocator
// only) or -1 (not a FreeCity4 lot developer); kFC4LotDevTestShadow returns
// what LotDevShadowTestRun returns (FC4LotDevShadow.h); kFC4LotDevTestState
// fills `*out` (b) and returns 1 (-1 without `out` or a lot developer).
intptr_t LotDevTestSeam(const FC4LotDevTestInput* in, void* out);
