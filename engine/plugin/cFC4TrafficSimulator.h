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

// The traffic simulator's GZCOM class (docs/specs/traffic-simulator.md §1;
// CLSID 0x6990C1AA, a city component, simulator type 0x88027F0D), its network
// model (traffic-network.md) and the pathfinders it hands out through slot 6
// (pathfinder.md), replaced as one unit (traffic-simulator.md §9). The model
// lives in the Rust core behind fc4.h (fc4_traffic, fc4_traffic_pf); this
// file implements the 44 slots of §7 in their compiled order (FC4Interfaces.h
// cFC4ISC4TrafficSimulator: the SDK gets 11, 19, 22, 23, 26 and the container
// slots wrong), cIGZMessageTarget2 and cIGZSerializable, the cISC4PathFinder
// objects of slot 6, and the host callbacks over the game's services.
//
// One object, three interfaces, one reference count. QueryInterface answers
// 1 and 0x68027F37 (the primary interface), 0x452294AA, 0xE4FDA3D4
// (fc4_traffic_query_interface).
//
// The grids are the game's (FC4HostGrid.h: u8, u32 and f32 classes) created
// through GZCOM and saved by reference.
//
// Threads. The game calls the getters from a second thread while the main
// thread is inside a tick (HANDOFF §5.1), and the core is single-threaded, so
// every call into it holds this object's lock. The main thread waits; any
// other thread tries for up to kLockWaitMs and otherwise answers from a
// snapshot taken at the end of every outermost main-thread call, or with the
// neutral value, and counts a fallback (the status lines). Re-entrant calls
// (a lot's own UpdateTrafficVolume asking slots 26 and 27 in the middle of a
// work slice) are the core's business: it answers them from its own snapshot
// and queues re-entrant messages (fc4.h).
//
// The subsystem is OFF by default: FreeCity4.ini `enable = traffic`;
// `disable` still wins. The traffic_* switches (FC4Config.h) turn groups of
// game calls off for bisecting.
#pragma once

#include "FC4Config.h"
#include "FC4GameList.h"
#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
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
#include <map>
#include <memory>
#include <unordered_set>
#include <vector>

class cIGZMessageServer2;
class cISC4City;
class cISC4Simulator;

static const uint32_t kFC4_CLSID_cSC4TrafficSimulator = 0x6990C1AA;
static const uint32_t kFC4_IID_cISC4TrafficSimulator = 0x68027F37;
static const uint32_t kFC4_IID_cISC4PathFinder = 0xAA0DB766;

namespace fc4 {

// What the traffic simulator's host callbacks work on (main thread, or
// another thread holding the simulator's lock).
struct TrafficContext {
    // The city's services: the city held from Init (or a Read) to Shutdown,
    // the others borrowed from it at each use.
    cISC4City* city = nullptr;
    cIGZMessageServer2* msgServer = nullptr;  // held from Init to Shutdown
    cIGZMessageTarget2* target = nullptr;     // the simulator's message target
    cIGZUnknown* self = nullptr;              // void1 / IGZUnknown of the posted messages
    HostGridContext grid{"traffic", Sw::TrafficGridWrites};
    // Test seam op 42: a table whose non-NULL callbacks answer instead of the
    // game (its own `user`), and a sink every posted message is handed to.
    const fc4_traffic_host* test = nullptr;
    FC4PostSinkFn postSink = nullptr;
    void* postSinkContext = nullptr;
    // References taken for one outermost call into the core (occupant lists,
    // connections, a re-entrant message's object), released when it returns.
    std::vector<cIGZUnknown*> held;
    std::vector<std::shared_ptr<void>> heldLists;
    // The message types subscribed (Shutdown removes them) and the simulator
    // the agents were registered with (held until Shutdown removes them).
    std::vector<uint32_t> subscribed;
    cISC4Simulator* agentSim = nullptr;
    bool agentsRegistered = false;
    // Per outermost call (cleared when it returns): the lot manager's lots
    // (one walk per call), and the lots and connections known to be alive.
    // A source retained across slices (§4.1) remains usable after removal;
    // all other tokens must still belong to the city or this call.
    std::vector<uint64_t> lotList;
    bool lotListValid = false;
    std::unordered_set<uint64_t> liveLots, seenLots;
    std::unordered_set<uint64_t> liveConns, seenConns;
    bool liveConnsValid = false;
    // The references the core holds (add_ref / release): the switch
    // buildings and the source iterator's connection snapshot (§9.1 TG19,
    // a connection removed during a phase is still visited). A held
    // connection token stays usable; `heldConns` marks the tokens that were
    // connections when the core took them.
    std::unordered_map<uint64_t, uint32_t> coreRefs;
    std::unordered_set<uint64_t> heldConns;
    std::unordered_set<uint64_t> heldLots;
    uint32_t connsLeftHeld = 0;  // held connections released after they left the city
    uint32_t staleTokens = 0;  // calls refused on a token no longer in the city
    // Observability (the status lines): calls made since construction.
    uint32_t posts = 0;           // messages posted (or handed to the sink)
    uint32_t sourceUpdates = 0;   // sources' own UpdateTrafficVolume runs
    uint32_t tripReads = 0;       // trip lists read
    uint32_t tripWrites = 0;      // trip lists replaced
    uint32_t tripWriteFailures = 0;  // trip lists not replaced (no allocator, no list)
    uint32_t holderWritesInPlace = 0;  // holder properties written in place
    uint32_t holderWrites = 0;         // holder properties written through AddProperty
    uint32_t budgetActivations = 0;    // activate_budget_items calls that found an item
    uint32_t departments = 0;          // departments handed to the core
    uint32_t lineItems = 0;            // line items created (TG7)
    uint32_t lotWalksMaster = 0;       // lot lists from the master list (slot 15)
    uint32_t lotWalksRect = 0;         // lot lists from GetLotsInCellRect
};

// The host table over `ctx` (the simulator's).
fc4_traffic_host MakeTrafficHost(TrafficContext* ctx);

}  // namespace fc4

class cFC4TrafficSimulator;

// A pathfinder of slot 6 (pathfinder.md §1): it forwards its slots to
// fc4_traffic_pf_* with its simulator's core, under the simulator's lock. It
// belongs to the simulator that created it; when that simulator is destroyed
// first, the pathfinder is orphaned and answers every slot with the neutral
// value. Release (§1): above 1 decrement; otherwise Shutdown if initialised,
// then destroy, return 0.
class cFC4PathFinder final : public cFC4ISC4PathFinder {
public:
    explicit cFC4PathFinder(cFC4TrafficSimulator* owner);
    // Slot 33: the scalar deleting destructor lands here (pathfinder.md §7).
    virtual ~cFC4PathFinder();

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    bool Init() override;
    bool Shutdown() override;
    void SetFlag(uint32_t flags) override;
    void ClearFlag(uint32_t flags) override;
    void ClearAllFlags() override;
    void SetRouteInfo(void* routeDeque) override;
    void SetTripDestinationFilter(uint32_t filter) override;
    void SetDestinationFlag(uint8_t bits) override;
    void ClearDestinationFlag(uint8_t bits) override;
    void ClearAllDestinationFlags() override;
    void SetTravelTypeFlag(uint16_t mask, bool alsoStarting) override;
    void ClearTravelTypeFlag(uint16_t mask) override;
    void ClearAllTravelTypeFlags() override;
    void SetStartingCostArray(float* costs9) override;
    void SetMaxTripLength(float length) override;
    void SetTrafficDensity(uint32_t trips) override;
    void SetStartRect(SC4Rect<int32_t> rect) override;
    void SetEndRect(SC4Rect<int32_t> rect) override;
    void SetMaxWorkTime(uint32_t ms) override;
    void Reset() override;
    uint32_t FindPath() override;
    uint32_t GetTrafficArrived() override;
    float GetTripLength() override;
    float GetTripLengthScaledForDisplay() override;
    void GetEndCell(int32_t& x, int32_t& z) const override;
    uint8_t GetLastTravelType() const override;
    void ApplyTraffic(uint32_t style) override;
    bool WasTripReversible() override;
    void ReleasePathNodes() override;
    void GetCompressedRoute(fc4::GameVector<uint8_t>& out, uint32_t& mask) override;

    fc4_traffic_pf* Core() const { return pf_; }
    // The simulator is going away: the core pathfinder goes with it.
    void Orphan();

private:
    bool Set(uint32_t slot, uint32_t a, uint32_t b);
    // §4.13 step 5 / §7 (TG1): the caller's deque, after a recorded trip.
    void FillRouteDeque();

    uint32_t refCount_ = 0;
    cFC4TrafficSimulator* owner_;
    fc4_traffic_pf* pf_ = nullptr;
    void* routeDeque_ = nullptr;  // the caller's deque<RouteData>* (slot 8), filled by FindPath (TG1)
};

class cFC4TrafficSimulator final : public cFC4ISC4TrafficSimulator,
                                   public cIGZMessageTarget2,
                                   public cIGZSerializable {
public:
    cFC4TrafficSimulator();
    // Slot 43 of the primary vtable: the scalar deleting destructor lands
    // there, as in the original (§7).
    virtual ~cFC4TrafficSimulator();

    // cIGZUnknown (all three interfaces)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4TrafficSimulator as compiled (§7)
    bool Init() override;
    bool Shutdown() override;
    uint32_t GetSimulatorType() override;
    bool CreatePathFinder(cFC4ISC4PathFinder** out) override;
    bool SetupPathFinderForLot(cFC4ISC4PathFinder* pf, cISC4Lot* lot) override;
    cISC4SimGrid<uint8_t>* GetAirPollutingTrafficMap() override;
    cISC4SimGrid<uint8_t>* GetCommercialTrafficMap() override;
    cISC4SimGrid<uint8_t>* GetCongestionMap() override;
    float GetBackgroundTraffic(int32_t x, int32_t z) override;
    FC4TrafficEdgeDensity* GetTrafficEdgeDensity(uint8_t x, uint8_t z, uint32_t travelType, bool evening) override;
    cISC4SimGrid<uint8_t>* GetTripLengthMap() override;
    float GetTripScale() override;
    float GetTripScaleForDisplay() override;
    bool IsRoadDamaged(int32_t x, int32_t z) override;
    bool CheckRailAccident(int32_t x, int32_t z) override;
    float GetLastMonthlyIncome() override;
    void SetMaxTripCapacity(cISCPropertyHolder* holder, uint32_t k, uint32_t value) override;
    uint32_t GetMaxTripCapacity(cISCPropertyHolder* holder, uint32_t k) override;
    uint32_t GetTrafficArrived(cISCPropertyHolder* holder, uint32_t k) override;
    void* GetDesiredLotInsertionPoint() override;
    float GetCapacity(uint32_t transitNetwork, int32_t x, int32_t z) override;
    float GetTravelTimeRatio(int32_t x, int32_t z, uint32_t travelType) override;
    uint32_t GetConnectionCount(uint32_t transitNetwork, int32_t x, int32_t z) override;
    void GetTravelStrategyPercentages(uint32_t wealth, fc4::GameVector<float>* out) override;
    float GetFreightScalingFactor() override;
    bool GetTransitSwitches(SC4List<cISC4Occupant*>& list) override;
    fc4::GameVector<uint8_t>* GetFerryRouteBetweenTiles(int32_t x1, int32_t z1, int32_t x2, int32_t z2) override;
    bool GetAllFerryRoutes(void* list) override;
    bool GetFerryRoutesInUse(void* list) override;
    uint32_t GetFerryTerminalCount(uint32_t type) override;
    bool GetWaterRoute(int32_t x1, int32_t z1, int32_t x2, int32_t z2, fc4::GameVector<uint8_t>& out) override;
    const uint8_t* GetTrafficStats() override;
    bool GetTransitSwitchQueryData(uint32_t x, uint32_t z, FC4TransitSwitchQueryData& out) override;
    bool AreLotsConnected(cISC4Lot* a, cISC4Lot* b) override;
    uint32_t GetConnectedOccupantCount(cISC4Lot* lot, uint32_t developerType) override;
    uint32_t GetConnectedDestinationCount(cISC4Lot* lot, int32_t k) override;
    bool GetSubnetworksForLot(cISC4Lot* lot, fc4::GameVector<uint32_t>& out) override;
    bool GetSubnetworksInRectangle(SC4Rect<int32_t> const& rect, fc4::GameVector<uint32_t>& out) override;
    bool GetSubnetworksInRegion(SC4CellRegion<long> const& region, fc4::GameVector<uint32_t>& out) override;
    bool GetOccupantCountForAllSubnetworks(uint32_t developerType, fc4::GameVector<uint32_t>& out) override;

    // cIGZMessageTarget2 (§3.1)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§6)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // How long a thread other than the main one waits for the lock.
    static constexpr uint32_t kLockWaitMs = 20;
    // Slot 12's ring of stable records (a caller keeps the pointer for a while).
    static constexpr size_t kEdgeRing = 64;

    uint32_t Fallbacks() const { return fallbacks_.load(); }
    // Test seam op 42 (FC4TestSeam.h): the instance whose interface is
    // `primary`, or null.
    static cFC4TrafficSimulator* FromPrimary(const void* primary);
    void SetTestHooks(const fc4_traffic_host* test, FC4PostSinkFn sink, void* sinkContext);
    // Live pathfinders handed out through slot 6 (every simulator).
    static long LivePathFinders();

    // For the pathfinders: the core, under the simulator's lock.
    fc4_traffic* Core() const { return core_; }
    void Forget(cFC4PathFinder* pf);
    bool Owns(const cFC4ISC4PathFinder* pf);

    // Holds the lock for one call into the core (see the file comment). On a
    // thread other than the main one ok() may be false: answer from the
    // snapshot or with the neutral value.
    class Section {
    public:
        explicit Section(const cFC4TrafficSimulator* s);
        ~Section();
        Section(const Section&) = delete;
        Section& operator=(const Section&) = delete;
        bool ok() const { return ok_; }

    private:
        cFC4TrafficSimulator* s_;
        bool ok_;
    };

private:
    // The values a getter on another thread falls back to.
    struct Snapshot {
        std::atomic<cISC4SimGrid<uint8_t>*> maps[4] = {};
        std::atomic<uint32_t> scalarBits[4] = {};  // slots 14, 15, 18, 27 (float bits)
        std::atomic<uint32_t> pctBits[4][3] = {};  // slot 26
        std::atomic<uint32_t> cycleState{0}, day{0};
    };

    bool Enter();
    void Leave();
    void RefreshSnapshot();
    void ReleaseHeld();
    void LogState(const char* what);
    // The work-cycle line on a state change, the month line (stage 3).
    void AfterMessage(uint32_t type);
    float Scalar(uint32_t which);
    void ClearFerryVectors();

    cIGZUnknown* Primary() { return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4TrafficSimulator*>(this)); }

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::TrafficContext ctx_;
    fc4_traffic_host host_{};
    fc4_traffic* core_ = nullptr;
    uint32_t seed_ = 0;
    bool seedFixed_ = false;

    mutable CRITICAL_SECTION lock_;
    DWORD mainTid_ = 0;
    int depth_ = 0;  // nesting of sections held (by the lock holder)
    mutable std::atomic<uint32_t> fallbacks_{0};
    // §9.1 TG21: messages another thread delivered while the lock was busy
    // (the original drops none); TG15: slot 19 writes made directly on the
    // holder, without the lock (the original's own path).
    std::atomic<uint32_t> droppedMessages_{0};
    std::atomic<uint32_t> directCapacityWrites_{0};
    Snapshot snap_;

    // The pathfinders this simulator handed out and still lives with.
    std::vector<cFC4PathFinder*> pathfinders_;
    uint32_t externalPathFinders_ = 0;
    uint32_t dequeFills_ = 0;     // route deques filled (TG1)
    uint32_t dequeFailures_ = 0;  // fills that faulted, found no map or ran out of memory
    // Slot 12: a ring of stable records; slot 34: the statistics block.
    FC4TrafficEdgeDensity edges_[kEdgeRing] = {};
    size_t edgeNext_ = 0;
    uint8_t stats_[FC4_TRAFFIC_STATS_BLOCK] = {};
    // Slot 29: one vector per tile pair (a fc4::GameVector<uint8_t> filled
    // with the containers' allocator, cFC4TrafficSimulator.cpp), kept until
    // the next call for the same pair or the destructor.
    std::map<uint64_t, void*> ferryVectors_;
    // The last work-cycle state and day logged.
    uint32_t lastCycleState_ = 0xFFFFFFFFu;
    uint32_t months_ = 0;
    uint32_t days_ = 0;

    friend class cFC4PathFinder;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4TrafficSimulator();

// Test seam op 42 (FC4TestSeam.h, kFC4SeamTraffic): `a` is one of these.
struct FC4TrafficTestInput {
    uint32_t op;  // kFC4TrafficTestHooks or kFC4TrafficTestLivePathFinders
    // kFC4TrafficTestHooks: on `traffic` (FreeCity4's, any of its interfaces),
    // every non-NULL callback of `host` answers instead of the game (with
    // host->user); a `host` whose grids.create is set replaces the game grids;
    // every posted message is handed to `postSink` instead of the message
    // server; the containers of slots 26, 28-33 and 39-42 and the pathfinder's
    // slot 32 are filled with `allocate` / `release` instead of the game's
    // allocator B. The core is created anew (call it before Init / Read). A
    // null `traffic` sets the allocator only (null: the game's again).
    cIGZUnknown* traffic;
    const fc4_traffic_host* host;
    FC4PostSinkFn postSink;
    void* postSinkContext;
    void* (*allocate)(uint32_t size);
    void (*release)(void* p);
};
enum : uint32_t { kFC4TrafficTestHooks = 1, kFC4TrafficTestLivePathFinders = 2 };
// Op 42's body: kFC4TrafficTestHooks returns 1 (installed), 0 (the allocator
// only) or -1 (not FreeCity4's traffic simulator); kFC4TrafficTestLivePathFinders
// returns the live slot-6 pathfinders.
intptr_t TrafficTestSeam(const FC4TrafficTestInput* in);
