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

// The building development simulator's GZCOM class (docs/specs/
// building-development.md; CLSID 0x8990BE7A, city component). The model is the
// Rust core behind fc4.h (fc4_development); this class implements the four
// interfaces of one object (§1.1: cISC4BuildingDevelopmentSimulator at +0,
// cIGZMessageTarget at +4, cIGZMessageTarget2 at +8, cIGZSerializable at
// +0xC) in the compiled slot order of §7.1 (FC4DevelopmentInterfaces.h), and
// the host callbacks over the game's services (§8).
//
// Replaced together with the growth developers (class 0x0990C138,
// cFC4GrowthDeveloper.h): §9 and growth-developer.md §12 make them one unit,
// so both classes are registered by the one "development" subsystem.
//
// Containers the game reads (§9.1) are kept in the game's layouts:
//   - the developer map (slot 34): an STLport map built with B
//     (FC4GameMapBuild.h), rebuilt after Init, Read and Shutdown;
//   - the special buildings (slot 44): an SC4List (A), refreshed after every
//     outermost call when the core's list changed;
//   - the catalogue (slots 11-22, 27-28, 37-40, 42-43, 45): a process-wide
//     mirror of the core's process statics, built after the first Init that
//     loaded them (§1.3 step 1); profiles (0x40 bytes, B capacity vectors and
//     maps) and names (game cRZStrings) never move once created;
//   - the building map (slot 41): the game's cISC4SimGrid<uint8> the core
//     created through GZCOM (FC4HostGrid.h).
//
// Threads: every call into the cores (this BDS, the process statics, every
// growth developer) holds one process-wide recursive lock
// (fc4::DevelopmentLock). The main thread waits for it; another thread waits
// at most 20 ms and then answers from the atomic copies published after
// every outermost main-thread call (or the slot's neutral value). The
// catalogue mirror has a lock of its own.
//
// The subsystem is OFF by default: FreeCity4.ini `enable = development`
// turns it on (FC4Subsystems.h). `development_growth_attempts_per_day = N`
// is §4.4.1's daily growth budget (0, the default, = unlimited).
//
// Log lines (FreeCity4.log):
//   development Init|Read|Shutdown: ... (the state)
//   development Write -> 0|1
//   development month YYYY-MM-DD: developers=N built=B (stages s0..s15)
//     (wealth w0/w1/w2/w3) declined=D (wealth ...) specials=S city=WxH
//     stage=.. resources=free/total ... (one per SimNewMonth)
//   development decline plan YYYY-MM-DD: N entries [day x,z stage dev ...]
//     (one per completed PlanDecline)
#pragma once

#include "FC4DevelopmentInterfaces.h"
#include "FC4HostGrid.h"
#include "cIGZMessageTarget.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#include <windows.h>

#include <atomic>
#include <cstdint>
#include <string>
#include <unordered_map>
#include <vector>

class cIGZCheatCodeManager;
class cIGZMessageServer2;
class cISC4City;
class cISC4Demand;
class cISC4GrowthDeveloper;
class cISC4Lot;

static const uint32_t kFC4_CLSID_cSC4BuildingDevelopmentSimulator = 0x8990BE7A;
static const uint32_t kFC4_IID_cISC4BuildingDevelopmentSimulator = 0x07813EAE;
static const uint32_t kFC4_CLSID_cSC4GrowthDeveloper = 0x0990C138;
static const uint32_t kFC4_IID_cISC4GrowthDeveloper = 0xE7CDD662;

namespace fc4 {

// The process-wide recursive lock of fc4.h ("one process-wide lock" for the
// BDS, its process statics and every growth developer).
class DevelopmentLock {
public:
    // Main thread: waits. Other threads: waits at most `otherThreadMs`;
    // held() says whether the lock was taken.
    explicit DevelopmentLock(uint32_t otherThreadMs = 20);
    ~DevelopmentLock();
    DevelopmentLock(const DevelopmentLock&) = delete;
    DevelopmentLock& operator=(const DevelopmentLock&) = delete;
    bool held() const { return held_; }

private:
    bool held_ = false;
};

// The thread the first development object was created on (the main thread).
bool OnDevelopmentMainThread();

// The process statics (created at first use; never destroyed: they live
// until process exit like the original's, §1.4 and growth §7). Call with the
// lock held.
fc4_development_process* DevelopmentProcess();
fc4_growth_statics* GrowthStatics();

// The session seed S (foundations §6.3): FreeCity4.ini `development_seed`
// (env FC4_DEVELOPMENT_SEED) when set, else the wall clock.
uint32_t DevelopmentSessionSeed(bool* fixed);

// The growth developer token of a developer type in the live BDS's map (0 =
// none; borrowed). Used by the growth developers' host.
uint64_t DevelopmentDeveloperOf(uint32_t developerType);

// What the BDS host callbacks work on. The simulator owns one; the shadow
// (FC4DevelopmentShadow.h) makes a read-only one over the city it reads.
struct DevelopmentContext {
    bool readOnly = false;  // the shadow: borrowed city, read callbacks only, no switches
    cIGZMessageTarget2* target = nullptr;  // the agent and notification target (simulator only)
    // Held from acquire_services to release_services (§1.3 step 5, §1.4).
    cISC4City* city = nullptr;
    cIGZUnknown* occupantManager = nullptr;  // cISC4OccupantManager
    cIGZUnknown* zoneManager = nullptr;      // cISC4ZoneManager
    cIGZUnknown* lotManager = nullptr;       // cISC4LotManager
    cIGZUnknown* landValue = nullptr;        // cISC4LandValueSimulator
    cIGZUnknown* power = nullptr;            // cISC4PowerSimulator
    cIGZUnknown* terrain = nullptr;          // cISTETerrain
    int32_t cellCountX = 0, cellCountZ = 0;  // cached at acquire_services (§1.3 step 5)
    // Framework registrations.
    cIGZMessageServer2* msgServer = nullptr;
    cIGZUnknown* simulator = nullptr;  // cISC4Simulator, held while agents are registered
    bool agentsRegistered = false;
    cIGZCheatCodeManager* cheatMgr = nullptr;
    std::vector<uint32_t> cheats;
    uint32_t commandId = 0;
    // Borrowed tokens (lots of GetLotsInCellRect, message lots and occupants
    // queued during a call) kept alive until the outermost call returns; one
    // reference each.
    std::vector<cIGZUnknown*> held;
    // Objects created through GZCOM for the core (occupants, developers, the
    // construction node) and the references the core holds on them: the
    // last one is an expected free (FC4_RELEASE_CREATED).
    std::unordered_map<cIGZUnknown*, uint32_t> created;
    // Observability.
    uint32_t occupantsCreated = 0, loadsFailed = 0, inserts = 0, removals = 0, grows = 0, declines = 0;
    uint32_t advisorEvents = 0, demolitions = 0, dezones = 0, lotElementUpdates = 0;
    // Foundations and retaining walls (§4.9.2, DG6): slot 23 calls and
    // successes, slot 26 / 29 calls and successes, foundations created.
    uint32_t foundationCalls = 0, foundationsAdded = 0, foundationsCreated = 0;
    uint32_t foundationLoads = 0, foundationLoadsOk = 0, wallLoads = 0, wallLoadsOk = 0;
    // The wall of the slot 29 call in progress (cISC4LotRetainingWallOccupant,
    // borrowed) and its occupant token (retaining_wall_type).
    cIGZUnknown* currentWall = nullptr;
    uint64_t currentWallToken = 0;
};

// The host table over `ctx` (the simulator's).
fc4_development_host MakeDevelopmentHost(DevelopmentContext* ctx, const HostGridContext* grids);
// Releases everything `ctx` still holds (services, registrations, held tokens).
void ReleaseDevelopmentContext(DevelopmentContext* ctx);
// Releases the tokens kept for the outermost call.
void DropHeldTokens(DevelopmentContext* ctx);

// Tooling (log lines only; no game write): what a month of development did to
// the city's lots, for comparing a run of the original development unit (the
// shadow's "development lots ... (the original's)") with one of ours
// ("development lots ... (ours)").
//
// The StateChanged flow: lot state changes (lot-developer.md, HabitationState
// 0..6; 7 = any other value) since the last Take().
struct LotFlow {
    uint32_t changes[8][8] = {};
    void Note(uint32_t from, uint32_t to);
    // "from>to:n ..." for the nonzero pairs ("-" for none); then cleared.
    std::string Take();
};

// The city's lots now, through lot manager slot 6 GetLot(x, z, true) for
// every cell (borrowed, each lot once): states, the livable ones, vacancy
// reasons, and the livable buildings whose wealth exceeds their lot's
// occupant wealth (budget.md 5.2's waning-wealth multiplier).
std::string LotCensus(cISC4City* city);

}  // namespace fc4

class cFC4BuildingDevelopmentSimulator final : public cFC4ISC4BuildingDevelopmentSimulator,
                                               public cIGZMessageTarget,
                                               public cIGZMessageTarget2,
                                               public cIGZSerializable {
public:
    cFC4BuildingDevelopmentSimulator();
    // Slot 59: the scalar deleting destructor (§7.1).
    virtual ~cFC4BuildingDevelopmentSimulator();

    // cIGZUnknown (§1.1: one object, one reference count)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4BuildingDevelopmentSimulator (§7.1)
    bool Init() override;
    bool Shutdown() override;
    bool LocationIsOKForNewBuilding(uint32_t type, float x, float z) override;
    cISC4BuildingOccupant* AddNewBuildingOccupantAtCell(uint32_t type, int32_t cellX, int32_t cellZ,
                                                        int32_t orientation) override;
    cISC4BuildingOccupant* AddNewBuildingOccupantAt(uint32_t type, float* position, int32_t orientation) override;
    bool AddNewBuildingOccupant(cISC4BuildingOccupant* occupant) override;
    int64_t GetCostOfFoundationDemolition(cISC4FoundationOccupant* occupant, uint32_t flags) override;
    int64_t GetCostOfOccupantDemolition(cISC4BuildingOccupant* occupant, uint32_t flags) override;
    uint32_t GetAllBuildingTypes(fc4::GameVector<uint32_t>& types) override;
    uint32_t GetAllBuildingTypesArray(uint32_t* types, uint32_t& count) override;
    uint32_t GetAllBuildingFamilyTypes(fc4::GameVector<uint32_t>& families) override;
    bool GetBuildingKeyFromType(uint32_t type, cGZPersistResourceKey& key) override;
    bool IsBuildingTypeValid(uint32_t type) override;
    bool IsBuildingFamilyTypeValid(uint32_t family) override;
    uint32_t GetBuildingTypesByFilter(fc4::GameHashSetU32<>& types, cISC4BuildingTypeFilter* filter) override;
    uint32_t GetBuildingTypesByFamily(fc4::GameVector<uint32_t>& types, uint32_t family) override;
    uint32_t GetBuildingTypeSetByFamily(fc4::GameHashSetU32<>& types, uint32_t family) override;
    bool IsBuildingInFamily(uint32_t type, uint32_t family) override;
    uint32_t GetAllFoundationTypes(fc4::GameVector<uint32_t>& types) override;
    bool GetFoundationKeyFromType(uint32_t type, cGZPersistResourceKey& key) override;
    bool AddNewFoundationOccupantAt(uint32_t type, int32_t a, float* position, float b, float c, int32_t d,
                                    float e) override;
    bool AddNewFoundationOccupant(cISC4FoundationOccupant* occupant) override;
    bool DemolishFoundationOccupant(cISC4FoundationOccupant* occupant, uint32_t flags) override;
    bool LoadFoundationProperties(cISC4FoundationOccupant* occupant, uint32_t type) override;
    uint32_t GetAllLotRetainingWallTypes(fc4::GameVector<uint32_t>& types) override;
    bool GetLotRetainingWallKeyFromType(uint32_t type, cGZPersistResourceKey& key) override;
    bool LoadLotRetainingWallProperties(cISC4LotRetainingWallOccupant* occupant, uint32_t type) override;
    uint32_t GetDeveloperType(uint32_t purpose, uint32_t wealth) override;
    uint32_t GetWealth(uint32_t developerType) override;
    cISC4GrowthDeveloper* GetDeveloper(uint32_t developerType) const override;
    cISC4GrowthDeveloper* GetDeveloperOf(uint32_t purpose, uint32_t wealth) const override;
    fc4::GameMapObject* GetDevelopers() override;
    cISC4TractDeveloper* GetTractDeveloper() override;
    cISC4LotDeveloper* GetLotDeveloper() override;
    cIGZString* GetExemplarName(uint32_t type) override;
    cIGZString* GetBuildingName(uint32_t type) override;
    FC4BuildingProfile& GetBuildingProfile(uint32_t type) const override;
    FC4BuildingProfile* GetBuildingProfilePtr(uint32_t type) const override;
    cISC4SimGrid<uint8_t>* GetBuildingMap() override;
    uint8_t GetBuildingMapKeyOf(uint32_t purpose, uint32_t wealth) override;
    uint8_t GetBuildingMapKey(uint32_t type) override;
    SC4List<cISC4Occupant*>* GetSpecialBuildings() override;
    float GetMaxCapacity(int32_t zone, uint32_t purpose, uint32_t wealth) override;
    bool IsSuitableForConstruction(FC4ConstructionProfile const& profile, int32_t& reason) override;
    cISC4BuildingOccupant* PlopBuilding(FC4ConstructionProfile const& profile) override;
    int32_t GetFreeConstructionResources() override;
    int32_t GetTotalConstructionResources() override;
    bool ClaimConstructionResources(uint16_t amount) override;
    bool ReleaseConstructionResources(uint16_t amount) override;
    int32_t GetDayOfLastSuccessfulBuild(uint32_t developerType) override;
    float GetStagePopulationMultiplier() override;
    bool CheatIgnorePlumbing() override;
    bool CheatIgnorePower() override;
    bool CheatIgnoreCaps() override;
    bool CheatDebugGrowth() override;
    bool CheatCoffeeBar() override;

    // cIGZMessageTarget (§7.3): true, nothing done.
    bool DoMessage(cGZMessage& message) override;
    // cIGZMessageTarget2 (§3)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§6)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    uint64_t SelfToken() const {
        return static_cast<uint64_t>(
            reinterpret_cast<uintptr_t>(static_cast<const cFC4ISC4BuildingDevelopmentSimulator*>(this)));
    }

    // Called after every outermost core call (main thread, lock held).
    void Publish();

private:
    class Call;  // the process lock and depth counting around core calls
    friend class Call;
    void LogState(const char* what);
    void LogMonth();
    void LogDeclinePlan();
    void RebuildDevelopers();
    void RefreshSpecials();
    uint64_t PublishedDeveloper(uint32_t developerType);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    uint32_t depth_ = 0;
    fc4::DevelopmentContext ctx_;
    fc4::HostGridContext gridCtx_;
    fc4_development_host host_{};
    fc4_development* core_ = nullptr;
    // The developer map handed out by slot 34 (B header and nodes), and the
    // (type, token) list it was built from.
    fc4::GameMapObject developers_{nullptr, 0, 0};
    std::vector<std::pair<uint32_t, uint64_t>> developerList_;
    // The special buildings handed out by slot 44 (one reference each), and
    // the tokens it was built from.
    SC4List<cISC4Occupant*>* specials_ = nullptr;
    std::vector<uint64_t> specialTokens_;
    // Publish's scratch lists (kept, so a Publish that finds nothing changed allocates nothing: the game calls
    // the BDS about 2,000 times a day and each outermost call publishes; devperf).
    std::vector<std::pair<uint32_t, uint64_t>> developerScratch_;
    std::vector<uint64_t> specialScratch_;
    // Atomic copies for other threads (Publish).
    std::atomic<uint64_t> buildingMap_{0};
    std::atomic<int32_t> freeResources_{0}, totalResources_{0};
    std::atomic<uint32_t> cheats_{0};
    std::atomic<float> stageMultiplier_{1.0f};
    std::atomic<uint32_t> offMainCalls_{0};
    // Guards developerList_ for readers on other threads (slot 32).
    SRWLOCK pubLock_ = SRWLOCK_INIT;
    // Status lines (main thread only).
    fc4_development_state lastMonth_{};
    bool haveLastMonth_ = false;
    uint8_t lastPlanState_ = 0;
    uint32_t days_ = 0, months_ = 0, plans_ = 0;
    uint32_t lastNewDay_ = 0;  // data1 of the last SimNewDay (the date without a city)
    fc4::LotFlow lotFlow_;     // StateChanged flow since the last month line
    uint32_t StatusDay();
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4BuildingDevelopmentSimulator();
