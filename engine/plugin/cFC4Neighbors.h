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

// The neighbours simulator's GZCOM classes (docs/specs/neighbors.md; CLSID
// 0xC990C0A4). The model lives in the Rust core behind fc4.h (fc4_neighbors,
// fc4_neighbor_deal, fc4_neighbor_connection); these classes implement:
//
//   cFC4NeighborsSimulator  cISC4NeighborsSimulator (0x0773FEF0, the 38
//                           compiled slots of FC4Interfaces.h), cIGZMessageTarget2
//                           and cIGZSerializable (§13), one reference count;
//   cFC4NeighborConnection  cISC4NeighborConnection (0x0773FEF2, 15 slots, SDK
//                           order) and cIGZSerializable (§4.3, CLSID 0x0A134F24),
//                           with an embedded property holder (FC4PropertyHolder.h)
//                           and traffic source (cISC4TrafficSource, the game's
//                           std::list<cSC4TripData> with allocator B);
//   cFC4NeighborDeal        cISC4NeighborDeal (0x0773FEF1, 41 slots, SDK order)
//                           and cIGZSerializable (§5.6, CLSID 0x2A134F42).
//
// §15.2: our simulator creates its own connection and deal objects, accepts
// the game's ORIGINAL connection objects in its list after a load (they reach
// the core as "foreign" connections, read and written through
// cISC4NeighborConnection only) and takes foreign deals as data. No factory is
// registered for 0x0A134F24 / 0x2A134F42: a save written by our simulator
// loads back as original connection objects through the game's own factory,
// which the game needs anyway for its regional-city records (§9.3); and the
// deal factory is unused (§15.2).
//
// Threads. The game calls getters from a second thread while the main thread
// is in a tick (HANDOFF §5.1), and the core is single-threaded, so every call
// into it - from the simulator, a connection or a deal - holds ONE lock of
// this glue. The main thread waits for it; another thread tries for up to
// kLockWaitMs and otherwise answers from a mirror (each object's state, the
// simulator's totals and counts, refreshed when the outermost call that took
// the lock returns) or with the neutral value, and counts a fallback. Nothing
// such a getter reads lives outside that lock or a mirror.
//
// Object lifetime. Connections and deals count references atomically. One
// that drops to 0 is parked, and deleted at the end of the outermost locked
// call on the main thread if its count is still 0 (a wrapper never dies
// inside a call into the core that may still hand out its token). The core
// keeps its own deals alive; a deal's object is made when the core first hands
// the deal out and lives while someone holds it.
//
// The subsystem is OFF by default: FreeCity4.ini `enable = neighbors`;
// `disable` still wins. The neighbors_* switches (FC4Config.h) turn groups of
// game calls off for bisecting.
#pragma once

#include "FC4Interfaces.h"
#include "FC4PropertyHolder.h"
#include "cIGZMessageTarget2.h"
#include "cIGZString.h"
#include "cIGZSerializable.h"
#include "cISC4NeighborConnection.h"
#include "cISC4NeighborDeal.h"
#include "cISC4TrafficSource.h"
#include "fc4.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <atomic>
#include <cstdint>
#include <memory>
#include <mutex>
#include <string>
#include <vector>

class cIGZMessageServer2;
class cISC4City;
class cISC4RegionalCity;
class cISC4Simulator;
class cSC4TripData;

static const uint32_t kFC4_CLSID_cSC4NeighborsSimulator = 0xC990C0A4;
static const uint32_t kFC4_IID_cISC4NeighborsSimulator = 0x0773FEF0;
static const uint32_t kFC4_IID_cISC4NeighborDeal = 0x0773FEF1;
static const uint32_t kFC4_IID_cISC4NeighborConnection = 0x0773FEF2;
static const uint32_t kFC4_CLSID_cSC4NeighborConnection = 0x0A134F24;
static const uint32_t kFC4_CLSID_cSC4NeighborDeal = 0x2A134F42;

namespace fc4 {
namespace props {
class Exemplar;
}

// ---- reads of the game shared with the neighbours shadow ---------------------------------------------
//
// Through the game's interfaces only; borrowed objects; main thread. Each
// returns false / leaves the output zeroed when the object is missing.

// A connection's state through cISC4NeighborConnection (ours or an original).
bool ReadNeighborConnection(cISC4NeighborConnection* c, fc4_neighbor_connection_state* out);
// A deal's state through cISC4NeighborDeal (dates through a cIGZDate of ours).
bool ReadNeighborDeal(cISC4NeighborDeal* d, fc4_neighbor_deal_data* out);
// A regional city's position, size, established flag, budget, income and the
// five statistics of commodities 1-3 (token = the city).
bool ReadNeighborCity(cISC4RegionalCity* c, fc4_neighbors_shadow_city* out);
// What UpdateCommodityStats reads from the city's simulators (§8.2).
void ReadNeighborFigures(cISC4City* city, fc4_neighbors_figures* out);
// The landfill developer's GetLandfillUsage (0, 0 without one).
void ReadNeighborLandfill(cISC4City* city, uint32_t* usage, uint32_t* capacity);
// The simulator date as a Julian day (0 without one).
uint32_t NeighborToday(cISC4City* city);
// The tuning exemplar "Neighbor Deals" (§12): a NEW fc4_props, or NULL.
fc4_props* LoadNeighborTuning(props::Exemplar* keep);
// The app's current city, regional city and region (borrowed; NULL if none).
cISC4City* NeighborAppCity();
cISC4RegionalCity* NeighborAppRegionalCity();
// region.GetCity(rx, rz) (§2.2): the regional city there (borrowed) or NULL.
cISC4RegionalCity* NeighborRegionCityAt(int32_t rx, int32_t rz);

}  // namespace fc4

// ---- trip data (the connection's traffic source) -----------------------------------------------------

// cSC4TripData (traffic-simulator.md §2, 0x2C bytes): the route vectors are
// the game's std::vector<uint8_t> (allocator B, allocators.md).
struct FC4TripVector {
    uint8_t* begin;
    uint8_t* end;
    uint8_t* capacityEnd;
};
struct FC4TripData {
    FC4TripVector route;        // 0x00
    FC4TripVector returnRoute;  // 0x0C
    uint32_t count;             // 0x18
    uint8_t type;               // 0x1C
    uint8_t strategy;           // 0x1D
    uint8_t flags;              // 0x1E
    uint8_t status;             // 0x1F
    int16_t destX;              // 0x20
    int16_t destZ;              // 0x22
    float tripLength;           // 0x24
    uint32_t travelTypesUsed;   // 0x28
};
struct FC4TripNode {
    FC4TripNode* next;
    FC4TripNode* prev;
    FC4TripData value;
};
static_assert(sizeof(void*) != 4 || sizeof(FC4TripData) == 0x2C, "cSC4TripData is 0x2C bytes");
static_assert(sizeof(void*) != 4 || sizeof(FC4TripNode) == 0x34, "the list's nodes are 0x34 bytes");

// The game's std::list<cSC4TripData> (allocators.md, GetTripDataList): one
// pointer to a full-size head node; nodes and route buffers from allocator B.
class FC4TripList {
public:
    FC4TripList();
    ~FC4TripList();
    FC4TripList(const FC4TripList&) = delete;
    FC4TripList& operator=(const FC4TripList&) = delete;

    bool valid() const { return head_ != nullptr; }
    FC4TripNode* head() const { return head_; }
    // Appends a copy of `t` (it must own no route buffers). False without memory.
    bool PushBack(const FC4TripData& t);
    // Unlinks and frees `n` and its route buffers; returns the next node.
    FC4TripNode* Erase(FC4TripNode* n);

private:
    FC4TripNode* head_;  // the only member: the game's list layout
};
static_assert(sizeof(FC4TripList) == sizeof(void*), "list layout: the head pointer");

class cFC4NeighborConnection;

// cISC4TrafficSource of a connection (traffic-simulator.md §4.1, vtable of 6
// slots): QueryInterface / AddRef / Release go to the connection.
class cFC4NeighborTrafficSource final : public cISC4TrafficSource {
public:
    explicit cFC4NeighborTrafficSource(cFC4NeighborConnection* owner) : owner_(owner) {}

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // Slot 3: the list itself (the traffic simulator edits it in place).
    SC4List<cSC4TripData>* GetTripDataList() override;
    // Slot 4 (traffic-simulator.md §4.1.4).
    bool UpdateTrafficVolume() override;
    // Slot 5 (pop 4): the connection's cell.
    bool GetSourceCells(SC4Rect<uint32_t>& rect) override;

    FC4TripList& Trips() { return trips_; }

private:
    cFC4NeighborConnection* owner_;
    FC4TripList trips_;
};

// ---- mirrors (what a getter on another thread answers from) --------------------------------------

template <class T>
class FC4Mirror {
public:
    void Set(const T& v) {
        std::lock_guard<std::mutex> g(m_);
        v_ = v;
    }
    T Get() const {
        std::lock_guard<std::mutex> g(m_);
        return v_;
    }

private:
    mutable std::mutex m_;
    T v_{};
};

struct FC4ConnectionMirror {
    fc4_neighbor_connection_state state;
    int32_t innerOk, innerX, innerZ;
};

// ---- the connection ---------------------------------------------------------------------------------

class cFC4NeighborConnection final : public cISC4NeighborConnection, public cIGZSerializable {
public:
    // Takes the NEW handle (destroyed with the object); count 1, the
    // simulator list's reference (fc4.h connection_object).
    explicit cFC4NeighborConnection(fc4_neighbor_connection* handle);
    cFC4NeighborConnection(const cFC4NeighborConnection&) = delete;
    cFC4NeighborConnection& operator=(const cFC4NeighborConnection&) = delete;

    // cIGZUnknown (both interfaces)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4NeighborConnection (§4.2)
    cISCPropertyHolder* AsISCPropertyHolder() override;
    cISC4TrafficSource* AsTrafficSource() override;
    cISC4NeighborsSimulator::ConnectionType GetType() const override;
    void GetConnectionCell(int32_t& cellX, int32_t& cellZ) const override;
    bool GetConnectionInnerCell(int32_t& cellX, int32_t& cellZ) const override;
    void GetRegionCellOnOtherSideOfConnection(int32_t& cellX, int32_t& cellZ) const override;
    cISC4RegionalCity* GetCityOnOtherSideOfConnection() const override;
    void Repair(cISC4RegionalCity* pCity) override;
    bool GetVisitedThisCycle() const override;
    void SetVisitedThisCycle(bool visited) override;
    bool GetIsActive() const override;
    void SetIsActive(bool active) override;

    // cIGZSerializable (§4.3)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    cISC4NeighborConnection* Primary() { return static_cast<cISC4NeighborConnection*>(this); }
    fc4_neighbor_connection* Handle() const { return h_; }
    uint32_t Refs() const { return refs_.load(); }
    // The core state (under the lock) or the mirror.
    FC4ConnectionMirror State() const;
    // Refreshes the mirror (the lock is held).
    void RefreshMirror();
    // §4.1 needsTrafficSync: true once (then false) after creation (NG8,
    // §15.4: set only by the constructor, so each object syncs once).
    bool TakeVolumeDirty() { return volumeDirty_.exchange(false); }
    cFC4PropertyHolder& Holder() { return holder_; }

    // Deletion (the main thread, under the lock, count 0): the park sweep.
    static void Destroy(cFC4NeighborConnection* c) { delete c; }

private:
    ~cFC4NeighborConnection();

    std::atomic<uint32_t> refs_{1};
    fc4_neighbor_connection* h_;
    cFC4PropertyHolder holder_;
    cFC4NeighborTrafficSource traffic_;
    FC4Mirror<FC4ConnectionMirror> mirror_;
    std::atomic<bool> volumeDirty_{true};
};

// ---- the deal ---------------------------------------------------------------------------------------

class cFC4NeighborDeal final : public cISC4NeighborDeal, public cIGZSerializable {
public:
    // Takes the NEW handle (destroyed with the object) and gives the core this
    // object's token; count 0 (parked until someone takes a reference).
    explicit cFC4NeighborDeal(fc4_neighbor_deal* handle);
    cFC4NeighborDeal(const cFC4NeighborDeal&) = delete;
    cFC4NeighborDeal& operator=(const cFC4NeighborDeal&) = delete;

    // cIGZUnknown (both interfaces)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4NeighborDeal (§5.4, SDK order)
    void SetRegionCellOnCitySideOfConnection(int32_t x, int32_t z) override;
    void GetRegionCellOnCitySideOfConnection(int32_t& x, int32_t& z) const override;
    void SetRegionCellOnOtherSideOfConnection(int32_t x, int32_t z) override;
    void GetRegionCellOnOtherSideOfConnection(int32_t& x, int32_t& z) const override;
    cISC4RegionalCity* GetCityOnOtherSideOfConnection() const override;
    int32_t GetType() const override;
    void SetType(int32_t type) override;
    int32_t GetAmount() const override;
    void SetAmount(int32_t amount) override;
    int32_t GetLastAmount() const override;
    void SetLastAmount(int32_t v) override;
    int32_t GetLastIncAmount() const override;
    void SetLastIncAmount(int32_t v) override;
    int32_t GetIncAmount() const override;
    void SetIncAmount(int32_t v) override;
    int32_t GetIncAmount2() const override;
    void SetIncAmount2(int32_t v) override;
    int32_t GetAmountUsed() const override;
    void SetAmountUsed(int32_t v) override;
    int64_t GetCost() const override;
    void UpdateCost() override;
    void GetStartDate(cIGZDate& date) const override;
    void SetStartDate(cIGZDate const& date) override;
    void GetExpirationDate(cIGZDate& date) const override;
    void SetExpirationDate(cIGZDate const& date) override;
    void GetCurrentDate(cIGZDate& date) override;
    void SetCurrentDate(cIGZDate const& date) override;
    void GetSuspensionDate(cIGZDate& date) override;
    void SetSuspensionDate(cIGZDate const& date) override;
    void Copy(cISC4NeighborDeal const& copyFrom) override;
    void UpdateState() override;
    void Cancel() override;
    bool Activate() override;
    int32_t GetFlags() const override;
    void SetFlags(int32_t flags, bool value) override;
    bool CheckFlagsAnd(int32_t flags) const override;
    bool CheckFlagsOr(int32_t flags) const override;
    bool Clone(cISC4NeighborDeal** outCopy, bool silent) const override;

    // cIGZSerializable (§5.6)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    cISC4NeighborDeal* Primary() { return static_cast<cISC4NeighborDeal*>(this); }
    fc4_neighbor_deal* Handle() const { return h_; }
    uint32_t Refs() const { return refs_.load(); }
    // The core state (under the lock) or the mirror.
    fc4_neighbor_deal_data Data() const;
    void RefreshMirror();

    static void Destroy(cFC4NeighborDeal* d) { delete d; }

private:
    ~cFC4NeighborDeal();
    // A plain setter (fc4_neighbor_deal_set_field).
    void SetField(uint32_t field, int32_t a, int32_t b);
    // Links the deal to the current simulator ("the current city's neighbours
    // simulator", §5.4) before a call that needs it (the lock is held).
    void Claim() const;

    std::atomic<uint32_t> refs_{0};
    fc4_neighbor_deal* h_;
    FC4Mirror<fc4_neighbor_deal_data> mirror_;
};

// ---- the simulator ----------------------------------------------------------------------------------

namespace fc4 {

// What the neighbours host callbacks work on (main thread, or another thread
// holding the glue's lock).
struct NeighborsContext {
    cISC4City* city = nullptr;                // held from Init to Shutdown
    cIGZMessageServer2* msgServer = nullptr;  // held likewise
    cIGZMessageTarget2* target = nullptr;     // the agent and notification target
    cISC4Simulator* agentSim = nullptr;       // held from add_agent to remove_agents
    bool agentsRegistered = false;
    std::unique_ptr<props::Exemplar> tuning;  // GetTuningExemplar (§12)
    // Observability (the month line), since construction.
    uint32_t posts = 0, sends = 0, connectionMessages = 0, created = 0, deleted = 0;
    uint32_t markersAdded = 0, markersRemoved = 0, markersMoved = 0, history = 0;
    uint32_t setLocalDeals = 0, regionRemoved = 0, reloads = 0;
};

fc4_neighbors_host MakeNeighborsHost(NeighborsContext* ctx);

}  // namespace fc4

class cFC4NeighborsSimulator final : public cFC4ISC4NeighborsSimulator,
                                     public cIGZMessageTarget2,
                                     public cIGZSerializable {
public:
    cFC4NeighborsSimulator();
    ~cFC4NeighborsSimulator();
    cFC4NeighborsSimulator(const cFC4NeighborsSimulator&) = delete;
    cFC4NeighborsSimulator& operator=(const cFC4NeighborsSimulator&) = delete;

    // cIGZUnknown (all three interfaces)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4NeighborsSimulator (§6.2, compiled slots 3-37)
    bool Init() override;
    bool Shutdown() override;
    bool RequestCreateConnection(ConnectionType type, int32_t x, int32_t z, bool flag) override;
    cISC4NeighborConnection* CreateConnection(ConnectionType type, int32_t x, int32_t z, bool flag) override;
    void DeleteConnection(ConnectionType type, int32_t x, int32_t z) override;
    void DeleteOccupantConnection(cISC4Occupant* occupant) override;
    void DeleteAllConnectionsOfType(ConnectionType type) override;
    bool HasConnection(ConnectionType type, int32_t x, int32_t z) const override;
    cISC4NeighborConnection* FindConnection(ConnectionType type, int32_t x, int32_t z) override;
    cISC4RegionalCity* FindCityOnOtherSideOfConnection(int32_t x, int32_t z) override;
    cISC4NeighborConnection* FindConnectionOnOtherSideOf(cISC4NeighborConnection* connection) override;
    cISC4NeighborConnection* FindConnectionOnOtherSideAt(ConnectionType type, int32_t x, int32_t z) override;
    cISC4NeighborConnection* FindUnvisitedConnection(ConnectionType type, cISC4RegionalCity const* city) override;
    void GetAllConnections(SC4List<cISC4NeighborConnection*>& list) const override;
    void GetConnectionsOfType(ConnectionType type, SC4List<cISC4NeighborConnection*>& list) const override;
    void GetConnectionsOfTypeToCity(ConnectionType type, cISC4RegionalCity const* city,
                                    SC4List<cISC4NeighborConnection*>& list) const override;
    void GetConnectionsToCity(cISC4RegionalCity const* city, SC4List<cISC4NeighborConnection*>& list) override;
    void GetAdjacentCities(SC4List<cISC4RegionalCity*>& list, bool includeUnestablished) override;
    void GetConnectedCities(SC4List<cISC4RegionalCity*>& list, bool includeUnestablished) override;
    void GetAllConnectedCities(SC4List<cISC4RegionalCity*>& list, bool includeUnestablished) override;
    cISC4NeighborConnection* FindConnectionSuitableForDeal(int32_t dealType, cISC4RegionalCity const* city) override;
    bool ProposeDeal(int32_t dealType, cISC4RegionalCity* otherCity, cISC4NeighborDeal** outDeal) override;
    bool ProposeDeals(SC4List<cISC4NeighborDeal*>& deals, bool flag) override;
    bool AddDeal(cISC4NeighborDeal* deal) override;
    void SetUIDisableFlag(bool flag) override;
    void FindDealsAll(SC4List<cISC4NeighborDeal*>& deals) override;
    void FindDealsWithCity(int32_t dealType, cISC4RegionalCity* city, SC4List<cISC4NeighborDeal*>& deals) override;
    void FindDealsOfType(int32_t dealType, SC4List<cISC4NeighborDeal*>& deals) override;
    void MarkDealsDirty() override;
    int64_t GetTotalMonthlyNeighborDealIncome() override;   // NG11: edx:eax
    int64_t GetTotalMonthlyNeighborDealExpense() override;  // NG11: edx:eax
    void UpdateCommodityStats(cISC4City* city) override;
    double GetDealCost(int32_t dealType, double amount, cISC4RegionalCity const* city) override;
    double GetDealMaxAmount(int32_t dealType, cISC4RegionalCity const* city) override;
    cISCResExemplar* GetTuningExemplar() override;

    // cIGZMessageTarget2 (§11.1)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§13)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    fc4_neighbors* Core() const { return core_; }
    // Refreshes the values other threads fall back to (the lock is held).
    void RefreshSnapshot();

private:
    struct Snapshot {
        std::atomic<int64_t> income{0}, expense{0};
        std::atomic<uint32_t> connections{0}, deals{0};
    };

    cIGZUnknown* Primary() { return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4NeighborsSimulator*>(this)); }
    void AcquireServices();
    void ReleaseServices();
    void LogState(const char* what);
    void LogMonth(uint32_t julian);
    // The list slots: tokens from the core, appended with one reference each.
    template <class T>
    static void Fill(SC4List<T*>& list, const std::vector<uint64_t>& tokens, bool clearFirst);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::NeighborsContext ctx_;
    fc4_neighbors_host host_{};
    fc4_neighbors* core_ = nullptr;
    Snapshot snap_;
    uint32_t months_ = 0;
    bool twoCurves_ = false;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4NeighborsSimulator();

class cISC4App;

namespace fc4 {
// Calls answered from a mirror or with the neutral value (any thread).
uint32_t NeighborsFallbacks();
// Test seam (FC4TestSeam.h op 48): an app whose GetPopupDialogsEnabled
// (slot 23) RequestCreateConnection reads instead of the framework's
// (borrowed: never queried or released; null: back to the framework's).
void NeighborsTestSetApp(cISC4App* app);
// nSC4UI::GenericConfirmationDialog (neighbors.md §15.4 NG35, the original
// 0x0078DFF0): cdecl (title, text, yesLabel, noLabel), the labels null for the
// default Yes and No; true only when closed with Yes.
using NeighborsConfirmFn = bool(__cdecl*)(cIGZString* title, cIGZString* text, cIGZString* yesLabel,
                                          cIGZString* noLabel);
// FC4TestSeam op 66: a fake dialog for RequestCreateConnection (null: the
// game's, through compat::Direct::GenericConfirmationDialog).
void NeighborsTestSetConfirm(NeighborsConfirmFn fn);
// §6.2's sprintf(text, name) (NG45 [choice]: the first %s, and %%).
std::string NeighborsFormatConfirmText(const std::string& format, const std::string& name);
// Test seam (op 48 with `b`): a new deal of ours with `src`'s state (Copy,
// §5.4), one reference for the caller in `*out`. AddDeal now keeps a
// foreign deal itself (NG4), so the tests make their deal of ours here.
bool NeighborsTestCopyDeal(cISC4NeighborDeal* src, cISC4NeighborDeal** out);
// Main-thread owners outside the simulator (e.g. regional-city record replay)
// enter the normal locked exit so zero-reference elements are collected even
// when the neighbors replacement itself is disabled.
bool NeighborsCollectReleasedElements();
}  // namespace fc4
