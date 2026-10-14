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

// The zone manager's GZCOM class (docs/specs/zones.md §1; CLSID 0x298F9B2D,
// city component 6). The model lives in the Rust core behind fc4.h
// (fc4_zones); this class implements the three interfaces of §1 in the
// compiled slot order of §11.1 (FC4Interfaces.h cFC4ISC4ZoneManager: the SDK
// gets slots 7-9, 11-13 and 18 wrong) and the host callbacks over the game's
// services (§12). The zone developer / lot manager (0x498F9B01) stays the
// game's: it is reached only through cISC4ZoneDeveloper and cISC4LotManager.
//
// One object, three interfaces, one reference count: the primary interface
// (cISC4ZoneManager 0x67652433, also cIGZUnknown), cIGZMessageTarget2 and
// cIGZSerializable. Which sub-object answers an IID is the core's decision
// (fc4_zones_query_interface).
//
// The zone grid is the game's int8 grid (class 0x49B9E603) created through
// GZCOM (FC4HostGrid.h), tract size 1, instance 0, saved by reference; a
// loaded grid keeps its saved geometry (simgrid §6.4, zones.md §13.1).
//
// ZoningChange (§5.3) carries a cell region in void2 that the message must
// free (§13.1). [choice] FreeCity4 posts its own message class,
// cFC4ZoningChangeMessage, whose destructor frees the region it owns: the
// game's C allocator (0x005E55E0) has no direct-call row in docs/abi, and the
// message class works without the game (tests).
//
// Threads. The game calls the getters from a second thread while the main
// thread is inside a tick (HANDOFF §5.1), and the core is single-threaded, so
// every call into it holds this object's lock. The main thread waits for the
// lock; any other thread tries for up to kLockWaitMs and otherwise answers
// from a snapshot taken at the end of every main-thread call (GetZoneType
// reads the game grid itself) or with the neutral value, and counts a
// fallback (the status lines).
//
// The subsystem is OFF by default: FreeCity4.ini `enable = zones`; `disable`
// still wins. The zones_* switches (FC4Config.h) turn groups of game calls off
// for bisecting.
#pragma once

#include "FC4Config.h"
#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
#include "FC4TestSeam.h"
#include "cFC4Message2Standard.h"
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
class cISC4BudgetSimulator;
class cISC4City;
class cISC4OccupantManager;
class cISC4PollutionSimulator;
class cISTETerrain;

static const uint32_t kFC4_CLSID_cSC4ZoneManager = 0x298F9B2D;
static const uint32_t kFC4_IID_cISC4ZoneManager = 0x67652433;

namespace fc4 {

// What the zone manager's host callbacks work on. The zone manager owns one;
// the zones shadow (FC4ZonesShadow.h) fills one with a city's borrowed
// services. Main thread, or another thread holding the zone manager's lock.
struct ZonesContext {
    bool readOnly = false;  // the shadow: borrowed services, read callbacks only
    bool gated = true;      // false: the zones_* switches are ignored (the shadow reads as it is)
    // The services of §1.1 step 2 and §10.1 field 13: held from Init (or a
    // Read) to Shutdown by the zone manager, borrowed by the shadow.
    cISC4City* city = nullptr;
    cISC4OccupantManager* occupants = nullptr;
    cISTETerrain* terrain = nullptr;
    cISC4PollutionSimulator* pollution = nullptr;
    cISC4BudgetSimulator* budget = nullptr;
    cIGZMessageServer2* msgServer = nullptr;  // zone manager only
    cIGZMessageTarget2* target = nullptr;     // zone manager only: the notification target
    cIGZUnknown* self = nullptr;              // zone manager only: ZoningChange's void1 / IGZUnknown
    HostGridContext grid{"zones", Sw::ZonesGridWrites};
    // Test seam op 38: a table whose non-NULL callbacks answer instead of the
    // game (its own `user`), and a sink every ZoningChange is handed to.
    const fc4_zones_host* test = nullptr;
    FC4PostSinkFn postSink = nullptr;
    void* postSinkContext = nullptr;
    // References taken for one call into the core (slot 33's occupant),
    // released when the outermost call returns.
    std::vector<cIGZUnknown*> held;
    // Observability (the status lines): calls made since construction.
    uint32_t posts = 0;            // ZoningChange messages posted (or handed to the sink)
    uint32_t parcellizations = 0;  // DoParcellization calls
    uint32_t pricings = 0;         // DemolishLots(false, ...) calls
    uint32_t demolitions = 0;      // DemolishLots(true, ...) calls
    uint32_t outLotsInserted = 0;  // new outLots elements
    uint32_t outLotsRefused = 0;   // outLots insertions refused (an inconsistent set, no allocator)
    uint32_t withdrawals = 0;      // WithdrawFunds calls
    uint32_t textures = 0;         // SetEmptyLotBaseTexture calls
};

// The host table over `ctx` (the zone manager's).
fc4_zones_host MakeZonesHost(ZonesContext* ctx);
// The read-only table for the shadow: every callback that would change the
// game (services, subscriptions, ZoningChange, parcellization, funds, the
// commit's demolition, lot textures, grid writes) is NULL; the shadow core
// swallows those writes before they reach the table anyway.
fc4_zones_host MakeZonesReadOnlyHost(ZonesContext* ctx);

// ---- the game's SC4CellRegion<long> (zones.md §11.4) ------------------------------------------------

// Reads a game region into the core's form over `mask` (row-major by z:
// index (z - z0) * width + (x - x0)). False (logged once with `tag`) when the
// region is not in the documented layout, or larger than the core accepts.
// An empty rectangle reads as an empty region.
bool ReadZoneRegion(const void* game, std::vector<uint8_t>& mask, fc4_zone_region* out, const char* tag);
// Clears, in the game region, the bits of the cells the core cleared in
// `r` (PlaceZone's rejected cells: the game only clears bits). Returns the
// bits cleared.
uint32_t ClearRejectedBits(void* game, const fc4_zone_region& r);

// A region in the game's layout whose memory is ours: for DoParcellization,
// GetZoningCost and the dry runs of the shadow. Not copyable.
class ZoneRegionBuffer {
public:
    ZoneRegionBuffer() = default;
    ZoneRegionBuffer(const ZoneRegionBuffer&) = delete;
    ZoneRegionBuffer& operator=(const ZoneRegionBuffer&) = delete;
    // False when the region is larger than the core accepts. An empty
    // rectangle builds an empty map.
    bool Build(const fc4_zone_region& in);
    // A rectangle with every cell set.
    bool BuildFull(int32_t x1, int32_t z1, int32_t x2, int32_t z2);
    SC4CellRegion<long>& Game() { return *reinterpret_cast<SC4CellRegion<long>*>(&layout_); }
    const void* Raw() const { return &layout_; }
    void* Raw() { return &layout_; }
    bool Bit(int32_t x, int32_t z) const;

private:
    // The cRZCellMap<bool> layout with a vtable of our own (slot 0, the
    // destructor, is its only virtual; nothing the game does with a region it
    // is handed destroys it).
    class CellMap {
    public:
        virtual ~CellMap() = default;
        uint32_t width = 0;
        uint32_t height = 0;
        uint32_t words = 0;
        uint32_t** rows = nullptr;
    };
    struct Layout {
        int32_t x1 = 0, z1 = 0, x2 = -1, z2 = -1;
        CellMap map;
    };
    static_assert(sizeof(void*) != 4 || sizeof(Layout) == 0x24, "SC4CellRegion<long> layout");
    Layout layout_;
    std::vector<uint32_t> bits_;
    std::vector<uint32_t*> rows_;
};

// The zone manager the city runs, if it is FreeCity4's (null otherwise);
// main thread. The observer's month hook writes its month line.
void ZonesMonthLine(int32_t julian);

}  // namespace fc4

// ZoningChange 0x07699455 (zones.md §5.3): a standard message that owns the
// changed-cell region its void2 points at and frees it when destroyed
// (§13.1: "post its own message class whose destructor frees it").
class cFC4ZoningChangeMessage final : public cFC4Message2Standard {
public:
    // A new message (reference count 0) owning a copy of `region`: void1 =
    // IGZUnknown = `zoneManager`, void2 = the region, data3 = the zone type,
    // data4 = checkSlope. Null if the region is too large.
    static cFC4ZoningChangeMessage* New(const fc4_zone_region& region, int32_t zoneType, bool checkSlope,
                                        cIGZUnknown* zoneManager);
    static long LiveRegions();  // for tests

private:
    cFC4ZoningChangeMessage() = default;
    ~cFC4ZoningChangeMessage() override;
    fc4::ZoneRegionBuffer* region_ = nullptr;
};

class cFC4ZoneManager final : public cFC4ISC4ZoneManager, public cIGZMessageTarget2, public cIGZSerializable {
public:
    cFC4ZoneManager();
    ~cFC4ZoneManager();

    // cIGZUnknown (all three interfaces)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4ZoneManager as compiled (§11.1)
    bool Init() override;
    bool Shutdown() override;
    cISC4SimGrid<int8_t>* GetZoneGrid() override;
    uint32_t GetZoneCount(int32_t type) override;
    uint32_t GetUndevelopedTileCountOfType(int32_t type) override;
    uint32_t GetUndevelopedTileCount() override;
    uint32_t GetDevelopmentFailureCount(int32_t zone, int32_t result) override;
    uint32_t GetAbandonedTileCount() override;
    bool PlaceZone(SC4CellRegion<long>& region, int32_t type, bool commit, bool skipFunds, bool checkSlope,
                   bool skipSizeCheck, bool checkLotInside, int64_t* outCost, int32_t* outError,
                   void* outLots) override;
    int32_t GetZoneType(int32_t x, int32_t z) override;
    bool IsRCIZone(int32_t type) override;
    bool GetTextureForZone(int32_t type, cGZPersistResourceKey& key) override;
    uint32_t GetZoneDragColor(int32_t type) override;
    uint8_t GetZoneDisplayAlpha() override;
    uint8_t GetDefaultZoneDisplayAlpha() override;
    void SetZoneDisplayAlpha(uint8_t alpha) override;
    int32_t GetMinZoneSize(int32_t type) override;
    int32_t GetMaxZoneSize(int32_t type) override;

    // cIGZMessageTarget2 (§5.1)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§10.1)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // How long a thread other than the main one waits for the lock.
    static constexpr uint32_t kLockWaitMs = 20;

    // Observability: calls answered from the snapshot / neutral value.
    uint32_t Fallbacks() const { return fallbacks_.load(); }
    // The month status line (the observer's SimNewMonth; main thread).
    void LogMonth(int32_t julian);
    // Test seam op 38 (FC4TestSeam.h): the instance whose primary interface
    // is `primary`, or null.
    static cFC4ZoneManager* FromPrimary(const void* primary);
    void SetTestHooks(const fc4_zones_host* test, FC4PostSinkFn sink, void* sinkContext);

private:
    // Holds the lock for one call into the core (see the file comment). On a
    // thread other than the main one ok() may be false: answer from the
    // snapshot or with the neutral value.
    class Section {
    public:
        explicit Section(const cFC4ZoneManager* s);
        ~Section();
        Section(const Section&) = delete;
        Section& operator=(const Section&) = delete;
        bool ok() const { return ok_; }

    private:
        cFC4ZoneManager* s_;
        bool ok_;
    };

    // The values a getter on another thread falls back to.
    struct Snapshot {
        std::atomic<uint32_t> zoneCount[16] = {};
        std::atomic<uint32_t> undeveloped[16] = {};
        std::atomic<uint32_t> undevelopedTotal{0}, abandoned{0};
        std::atomic<int32_t> minSize[16] = {}, maxSize[16] = {};
        std::atomic<uint32_t> texture[16] = {}, dragColor[16] = {};
        std::atomic<uint32_t> alpha{0xFF}, defaultAlpha{0xFF};
        std::atomic<cISC4SimGrid<int8_t>*> grid{nullptr};
        std::atomic<int32_t> gridX{0}, gridZ{0};
    };

    bool Enter();
    void Leave();
    void RefreshSnapshot();
    void LogState(const char* what);
    void ReleaseHeld();

    cIGZUnknown* Primary() { return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4ZoneManager*>(this)); }

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::ZonesContext ctx_;
    fc4_zones_host host_{};
    fc4_zones* core_ = nullptr;

    mutable CRITICAL_SECTION lock_;
    DWORD mainTid_ = 0;
    int depth_ = 0;  // nesting of sections held (by the lock holder)
    mutable std::atomic<uint32_t> fallbacks_{0};
    Snapshot snap_;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4ZoneManager();

// Test seam op 38 (FC4TestSeam.h, kFC4SeamZones): `a` is one of these.
struct FC4ZonesTestInput {
    uint32_t op;  // kFC4ZonesTestHooks or kFC4ZonesTestShadow
    // kFC4ZonesTestHooks: on `zoneManager` (FreeCity4's, any of its interfaces),
    // every non-NULL callback of `host` answers instead of the game (with
    // host->user); a `host` whose grids.create is set replaces the game grids;
    // every ZoningChange is handed to `postSink` before it is posted; the outLots
    // set's nodes come from `setAllocate` / `setFree` instead of the game's
    // allocator. The core is created anew (call it before Init / Read).
    cIGZUnknown* zoneManager;
    const fc4_zones_host* host;
    FC4PostSinkFn postSink;
    void* postSinkContext;
    void* (*setAllocate)(uint32_t size);
    void (*setFree)(void* p);
    // kFC4ZonesTestShadow: one shadow comparison (FC4ZonesShadow.h) on `city`,
    // whose zone manager is the "original": event 0 = PostCityInit, 1 = a month.
    cISC4City* city;
    int32_t julian;
    uint32_t event;
};
enum : uint32_t { kFC4ZonesTestHooks = 1, kFC4ZonesTestShadow = 2 };
// Op 38's body: kFC4ZonesTestHooks returns 1 (installed) or -1 (not a
// FreeCity4 zone manager); kFC4ZonesTestShadow returns what
// ZonesShadowTestRun returns (FC4ZonesShadow.h).
intptr_t ZonesTestSeam(const FC4ZonesTestInput* in);
