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

// The zone developer / lot manager's GZCOM class (docs/specs/zones.md §1,
// §1.2, §3.2, §5.2, §7-§10.2, §11.2, §11.3; CLSID 0x498F9B01, city
// components 7 and 8). The model lives in the Rust core behind fc4.h
// (fc4_zonedev); this class implements the four interfaces of §1 at the
// sub-objects the original has them (+0 cISC4ZoneDeveloper, +4
// cISC4LotManager in the compiled slot order of §11.3, FC4Interfaces.h
// cFC4ISC4LotManager, +8 cIGZMessageTarget2, +0xC cIGZSerializable) and the
// host callbacks over the game's services (§12; FC4ZoneDevHost.cpp).
//
// One object, four interfaces, one reference count. Which sub-object answers
// an IID is the core's decision (fc4_zonedev_query_interface). Lot manager
// slot 3 (Init) tail-jumps to the zone developer's Init (§1.2): both bases
// declare Init / Shutdown, so one override fills both slots.
//
// The master lot list is the core's; the host mirrors it in the game's raw
// layout (FC4SafeLotIterator.h, lot-developer.md §15.1): a list object
// {master list head node, iterator list head node}, 12-byte B nodes, so the
// safe lot iterators of slot 15 (8-byte C blocks {node, list object}) walk
// it and their owners unregister and free them as they do the original's.
//
// STAGE A: the game's lot object has no constructor row in docs/abi yet
// (lot-developer.md; re-specs11), so `new_lot` builds nothing: every lot
// creation fails cleanly (CreateLot returns 0) and logs
// "[waiting for re-specs11]". Lots read from a save are kept, queried,
// deleted and written as the spec says.
//
// Threads. As the zone manager (cFC4ZoneManager.h): every call into the core
// holds this object's lock; the main thread waits for it, any other thread
// tries for up to kLockWaitMs and otherwise answers from a snapshot (the lot
// count) or with the neutral value, and counts a fallback.
//
// The subsystem is OFF by default: FreeCity4.ini `enable = zonedev`;
// `disable` still wins. The zonedev_* switches (FC4Config.h) turn groups of
// game calls off for bisecting.
#pragma once

#include "FC4Config.h"
#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
#include "FC4SafeLotIterator.h"
#include "FC4TestSeam.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4ZoneDeveloper.h"
#include "fc4.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <atomic>
#include <cstdint>
#include <memory>
#include <unordered_map>
#include <vector>

class cIGZMessageServer2;
class cISC4BuildingDevelopmentSimulator;
class cISC4City;
class cISC4LotDeveloper;
class cISC4OccupantManager;
class cISC4TrafficSimulator;
class cISC4ZoneManager;
class cISTETerrain;
template <typename T>
class SC4List;
struct FC4ZoneDevTestState;

static const uint32_t kFC4_CLSID_cSC4ZoneDeveloper = 0x498F9B01;
static const uint32_t kFC4_IID_cISC4ZoneDeveloper = 0xE818ED2B;
static const uint32_t kFC4_IID_cISC4LotManager = 0x681905BD;

namespace fc4 {

// The game's allocators the zone developer's raw structures use: B (12-byte
// list nodes) and C (the safe lot iterator blocks), or the test seam's.
struct ZoneDevAllocators {
    static bool NodesAvailable();
    static void* AllocateNode(uint32_t size);
    static void FreeNode(void* p);
    static bool BlocksAvailable();
    static void* AllocateBlock(uint32_t size);
    static void FreeBlock(void* p);
    // The policy GameList / GameVector take (FC4GameList.h).
    struct NodePolicy {
        static bool Available() { return NodesAvailable(); }
        static void* Allocate(uint32_t size) { return AllocateNode(size); }
        static void Free(void* p) { FreeNode(p); }
        static const void* StringVtable() { return nullptr; }
    };
};

// The master lot list in the original's raw layout (zones.md §3.2, §8.1;
// FC4SafeLotIterator.h): the list object is {master head, iterator head};
// every node is {next, prev, value} from B. Main thread (under the zone
// developer's lock).
class ZoneDevMasterList {
public:
    struct ListObject {
        safelots::Node* masterHead;
        safelots::Node* iteratorHead;
    };
    static_assert(sizeof(ListObject) == 2 * sizeof(void*), "the lot list object: two head pointers");

    ZoneDevMasterList() = default;
    ZoneDevMasterList(const ZoneDevMasterList&) = delete;
    ZoneDevMasterList& operator=(const ZoneDevMasterList&) = delete;
    ~ZoneDevMasterList();

    // Allocates the two head nodes (false without allocator B; logged once).
    bool Ensure();
    bool Valid() const { return obj_.masterHead != nullptr && obj_.iteratorHead != nullptr; }
    // master_insert: before `before` (0 or not listed: appended).
    bool Insert(void* lot, void* before);
    // master_remove: unlinks the lot's node; every registered iterator on it
    // moves to the next node first (§8.6 step 4).
    bool Remove(void* lot);
    // master_reset: the list becomes `lots` in order; registered iterators
    // move to the list's end (the head).
    void Reset(const uint64_t* lots, size_t n);
    // Slot 15: a new C block {first node, list object} registered with a B
    // node at the end of the iterator list; null without the allocators.
    safelots::Block* NewIterator();
    // The registered iterators still in the iterator list.
    uint32_t RegisteredIterators() const;
    // [choice] (zones.md §14.3 Q3) At Shutdown / destruction every block
    // still registered is unlinked (its B node freed) and its list object
    // set to null, so its owner's release skips the unregister and frees the
    // block with C (budget.md §5.2 BQ4) instead of touching freed memory.
    uint32_t OrphanIterators();
    // The traffic simulator's iterator (§8.5 step 4): the node's lot (0 at
    // the end) when `block` is one of this list's, false otherwise.
    bool Owns(const safelots::Block* block) const { return block != nullptr && block->listObject == &obj_; }
    uint64_t LotAt(const safelots::Node* node) const;
    // Moves `block` onto the node of `lot` (0: the head).
    void MoveTo(safelots::Block* block, uint64_t lot);
    const ListObject* Object() const { return &obj_; }
    size_t Size() const { return nodes_.size(); }
    // The lots in list order (tests, diagnostics).
    void Lots(std::vector<void*>& out) const;

private:
    void FreeMaster();
    ListObject obj_{nullptr, nullptr};
    std::unordered_map<void*, safelots::Node*> nodes_;
};

// What the zone developer's host callbacks work on. The zone developer owns
// one; the shadow (FC4ZoneDevShadow.h) fills one with a city's borrowed
// services. Main thread, or another thread holding the zone developer's lock.
struct ZoneDevContext {
    bool readOnly = false;  // the shadow: borrowed services, read callbacks only
    bool gated = true;      // false: the zonedev_* switches are ignored (the shadow reads as it is)
    // §1.2 step 2 (without the 3D view): held from Init to Shutdown by the
    // zone developer, borrowed by the shadow.
    cISC4LotDeveloper* lotDeveloper = nullptr;
    cISC4BuildingDevelopmentSimulator* bds = nullptr;
    cISC4City* city = nullptr;
    cISC4OccupantManager* occupants = nullptr;
    cISTETerrain* terrain = nullptr;
    cISC4TrafficSimulator* traffic = nullptr;
    cISC4ZoneManager* zones = nullptr;
    cIGZMessageServer2* msgServer = nullptr;  // zone developer only
    cIGZMessageTarget2* target = nullptr;     // zone developer only: the notification target
    // §5.2 PostCityInit: the street (3), dirt-road (11) and road (0) network
    // tools, each with a reference (zone developer only).
    cIGZUnknown* tools[3] = {nullptr, nullptr, nullptr};
    HostGridContext grid{"zonedev", Sw::ZoneDevGridWrites};
    // Test seam op 53: a table whose non-NULL callbacks answer instead of the
    // game (its own `user`).
    const fc4_zonedev_host* test = nullptr;
    // The master lot list mirror (zone developer only).
    ZoneDevMasterList* master = nullptr;
    // The traffic simulator's iterator of the CreateLot in progress (§8.5 step 4).
    safelots::Block* insertion = nullptr;
    // References and lists taken for one call into the core, released when
    // the outermost call returns.
    std::vector<cIGZUnknown*> held;
    std::vector<cIGZUnknown*> heldConfigs;  // the configurations' references (not audited one by one)
    std::vector<std::shared_ptr<SC4List<cISC4Occupant*>>> heldLists;
    // References handed on to a caller (slots 7-10, 22, 23): AddRef'd for the caller.
    bool forCaller = false;
    // The lot new_lot just built (reference count 0) until the core has
    // AddRef'd it twice (§8.5 step 2; AddRefToken), and those AddRefs so far.
    void* freshLot = nullptr;
    uint32_t freshAdds = 0;
    // Observability (the status lines): since construction.
    uint32_t newLotRefusals = 0;  // new_lot calls that built no lot (switch off, executable not identified)
    uint32_t canCreateRefusals = 0;  // slot 20 answers of 0 because lots cannot be built (ZG34)
    uint32_t lotsBuilt = 0;       // lots built from the game's lot pool (§8.5.1)
    uint32_t joins = 0;           // street joins (§5.2.1, §5.2.2)
    uint32_t dezoneCalls = 0;     // UpdateZoneType's PlaceZone dezones (§7.2.2)
    uint32_t zoneUpdates = 0;     // lot UpdateZoneType calls
    uint32_t posts = 0;           // DeferOccupantOperations messages posted
    uint32_t streetsPlaced = 0;   // street Place calls
    uint32_t demolitions = 0;     // DemolishRegion calls
    uint32_t levellings = 0;      // AttemptAreaLevelling calls
    uint32_t removedOccupants = 0;
    uint32_t iteratorsIssued = 0;  // slot 15 blocks handed out
    uint32_t configRefs = 0;       // configuration references taken (and given back)
};

// The host table over `ctx` (the zone developer's).
fc4_zonedev_host MakeZoneDevHost(ZoneDevContext* ctx);
// The read-only table for the shadow: every callback that would change the
// game (services, subscriptions, tools, posts, demolition, streets, lot
// changes, levelling, the lot developer's writes, occupant removal, the
// mirror, grid writes) is NULL; the shadow core swallows those before they
// reach the table anyway.
fc4_zonedev_host MakeZoneDevReadOnlyHost(ZoneDevContext* ctx);
// Gives back what the context holds for one call (held references, lists,
// the configurations' references).
void ZoneDevReleaseHeld(ZoneDevContext* ctx);
// Gives back the services and network tools (zone developer only).
void ZoneDevReleaseServices(ZoneDevContext* ctx);
// Test seam op 53: the raw structures' allocators (null: the game's).
void ZoneDevSetTestAllocators(void* (*allocate)(uint32_t), void (*free)(void*));
// Whether `ctx`'s host can build new lots (ZG34): the test table's new_lot
// when one is installed, else zonedev_lot_changes on and the lot pool and
// constructor available (the executable identified). No side effects.
bool ZoneDevLotsConstructible(const ZoneDevContext* ctx);

// The zone developer the city runs, if it is FreeCity4's (null otherwise);
// main thread. The observer's month hook writes its month line.
void ZoneDevMonthLine(int32_t julian);

}  // namespace fc4

class cFC4ZoneDeveloper final : public cISC4ZoneDeveloper,
                                public cFC4ISC4LotManager,
                                public cIGZMessageTarget2,
                                public cIGZSerializable {
public:
    cFC4ZoneDeveloper();
    ~cFC4ZoneDeveloper();

    // cIGZUnknown (all four interfaces)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4ZoneDeveloper slots 3-9 (§11.2) and cISC4LotManager slots 3-4
    bool Init() override;
    bool Shutdown() override;
    void HighlightParcels(SC4CellRegion<long> const& region, cISC4ZoneManager::ZoneType zoneType,
                          SC4Point<long>* point, SC4CellRegion<long>** out) override;
    void DoParcellization(SC4CellRegion<long> const& region, cISC4ZoneManager::ZoneType zoneType,
                          bool flag) override;
    void SetOptions(bool alternateOrientation, bool placeStreets, bool customZoneSize) override;
    bool ExistsNetworkOfType(int32_t x, int32_t z, uint32_t networkFlags) override;
    bool IsCellWater(int32_t x, int32_t z) override;

    // cISC4LotManager slots 5-27 as compiled (§11.3)
    cISC4Lot* GetLotAtPoint(cS3DVector3 const& point) override;
    cISC4Lot* GetLot(int32_t cellX, int32_t cellZ, bool originOnly) override;
    uint32_t GetLotsInCellRect(fc4::GameList<cISC4Lot*>& lots, int32_t x1, int32_t z1, int32_t x2, int32_t z2,
                               bool originOnly) override;
    uint32_t GetLotsSurroundingCell(int32_t cellX, int32_t cellZ, fc4::GameList<cISC4Lot*>& lots,
                                    int32_t flags) override;
    uint32_t GetLotsSurroundingLot(cISC4Lot const* lot, fc4::GameList<cISC4Lot*>& lots, int32_t flags) override;
    uint32_t GetLotsSurroundingRect(SC4Rect<long> const& rect, fc4::GameList<cISC4Lot*>& lots,
                                    int32_t flags) override;
    bool GetVerticesAtEdgeOfLot(SC4Rect<long> const& cells, int32_t flags, SC4Rect<long>& vertices) override;
    bool GetNeighborCellsAtEdgeOfLot(SC4Rect<long> const& cells, int32_t flags, SC4Rect<long>& out) override;
    cISC4Lot* GetRandomLot() override;
    cISC4Lot* GetOccupantLot(cISC4Occupant* occupant) override;
    intptr_t GetSafeLotIterator() override;
    int32_t GetLotCount() override;
    float GetLotSavvyTerrainVertexAltitude(int32_t x, int32_t z, cISC4Lot* exclude, bool* ok) override;
    float GetLotSavvyTerrainAltitude(float x, float z, bool* onLot) override;
    float GetLotSavvyCellAltitude(int32_t cellX, int32_t cellZ, bool* ok) override;
    int32_t CanCreateLot(int32_t x, int32_t z, int32_t w, int32_t h, bool zoneNotNeeded, bool waterAllowed,
                         cISC4LotConfiguration* config) override;
    int32_t CanCreatePloppedLot(SC4Rect<long> bounds, int32_t n, cISC4LotConfiguration* config) override;
    bool CreateLotIfPossible(int32_t x, int32_t z, int32_t w, int32_t h, int32_t facing, cISC4Lot*& lot,
                             bool zoneNotNeeded, bool waterAllowed, cISC4LotConfiguration* config) override;
    bool CreateLot(int32_t x, int32_t z, int32_t w, int32_t h, int32_t facing, cISC4Lot*& lot) override;
    bool DeleteLot(cISC4Lot* lot) override;
    bool DeleteLots(int32_t x1, int32_t z1, int32_t x2, int32_t z2) override;
    bool GetLotVertexAltitudeAdjustmentRange(int32_t x, int32_t z, float& a, float& b) override;
    cISC4SimGrid<uint16_t>* GetMaximumCapacityGrid() override;

    // cIGZMessageTarget2 (§5.2)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§10.2)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // How long a thread other than the main one waits for the lock.
    static constexpr uint32_t kLockWaitMs = 20;

    uint32_t Fallbacks() const { return fallbacks_.load(); }
    // The month status line (the observer's SimNewMonth; main thread).
    void LogMonth(int32_t julian);
    // The instance any of whose four interfaces is `p`, or null (the shadow,
    // test seam op 53).
    static cFC4ZoneDeveloper* FromAny(const void* p);
    // The raw master list object the safe lot iterators of slot 15 point to
    // (the original's lot manager + 0x5C): the traffic simulator's lot walk
    // (traffic-simulator §4.1, §9.1 TG10) checks its blocks against it.
    const void* MasterListObject() const { return master_.Object(); }
    // The list object the safe lot iterators of `lotManager`'s slot 15 point
    // to, when a glue may walk them raw (the traffic simulator's lot walk,
    // the budget's ComputeTax walk, the lot developer's daily pass): on the
    // identified executable only, the original lot manager's (+0x5C) or
    // FreeCity4's (MasterListObject()); null otherwise, with *why (if not
    // null) saying why (safelots::ExpectedListObject).
    static const void* SafeIteratorListObject(const void* lotManager, const char** why);
    void SetTestHooks(const fc4_zonedev_host* test);
    void TestState(FC4ZoneDevTestState& out);

private:
    class Section {
    public:
        explicit Section(const cFC4ZoneDeveloper* s);
        ~Section();
        Section(const Section&) = delete;
        Section& operator=(const Section&) = delete;
        bool ok() const { return ok_; }

    private:
        cFC4ZoneDeveloper* s_;
        bool ok_;
    };

    bool Enter();
    void Leave();
    void RefreshSnapshot();
    void LogState(const char* what);
    void CreateCore();

    cIGZUnknown* Primary() { return static_cast<cIGZUnknown*>(static_cast<cISC4ZoneDeveloper*>(this)); }

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    uint32_t seed_ = 0;
    bool seedFixed_ = false;
    fc4::ZoneDevContext ctx_;
    fc4::ZoneDevMasterList master_;
    fc4_zonedev_host host_{};
    fc4_zonedev* core_ = nullptr;

    mutable CRITICAL_SECTION lock_;
    DWORD mainTid_ = 0;
    int depth_ = 0;
    mutable std::atomic<uint32_t> fallbacks_{0};
    std::atomic<int32_t> snapLotCount_{0};
    std::atomic<cISC4SimGrid<uint16_t>*> snapGrid_{nullptr};
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4ZoneDeveloper();

// Test seam op 53 (FC4TestSeam.h, kFC4SeamZoneDev): `a` is one of these.
struct FC4ZoneDevTestInput {
    uint32_t op;  // kFC4ZoneDevTestHooks, kFC4ZoneDevTestShadow or kFC4ZoneDevTestState
    // kFC4ZoneDevTestHooks: on `zoneDeveloper` (FreeCity4's, any of its
    // interfaces), every non-NULL callback of `host` answers instead of the
    // game (with host->user); the core is created anew (call it before Init /
    // Read). The raw structures' allocators (B nodes, C blocks) become
    // `allocate` / `free` (null: the game's again); a null `zoneDeveloper`
    // sets the allocators only.
    cIGZUnknown* zoneDeveloper;
    const fc4_zonedev_host* host;
    void* (*allocate)(uint32_t size);
    void (*free)(void* p);
    // kFC4ZoneDevTestShadow: one shadow comparison (FC4ZoneDevShadow.h) on
    // `city`, whose lot manager and zone developer are the "original": event
    // 0 = PostCityInit, 1 = a month; `host` (may be null) answers the shadow
    // core's host calls instead of the game.
    cISC4City* city;
    int32_t julian;
    uint32_t event;
};
enum : uint32_t { kFC4ZoneDevTestHooks = 1, kFC4ZoneDevTestShadow = 2, kFC4ZoneDevTestState = 3 };
// kFC4ZoneDevTestState's out block (`b`): what the zone developer holds.
struct FC4ZoneDevTestState {
    uint32_t masterSize;     // nodes in the raw master list
    uint32_t iterators;      // registered safe lot iterators
    uint32_t newLotRefusals;  // new_lot calls that built no lot
    uint32_t heldConfigs;    // configuration references not given back yet (0 between calls)
    uint32_t lotsBuilt;      // lots built from the game's lot pool
    uint32_t canCreateRefusals;  // slot 20 answers of 0 because lots cannot be built (ZG34)
};
// Op 53's body: kFC4ZoneDevTestHooks returns 1 (installed), 0 (allocators
// only) or -1 (not a FreeCity4 zone developer); kFC4ZoneDevTestShadow returns
// what ZoneDevShadowTestRun returns; kFC4ZoneDevTestState fills `b` and
// returns 1 (-1: not ours or no `b`).
intptr_t ZoneDevTestSeam(const FC4ZoneDevTestInput* in, void* b);
