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

// The fire and police protection simulators' GZCOM classes
// (docs/specs/protection.md §1; CLSIDs 0x2990C05A fire, 0x6990C04F police).
// The model lives in the Rust core behind fc4.h (fc4_protection); these
// classes implement the four interfaces of §1 in the compiled slot order of
// §15 (FC4Interfaces.h: the SDK declarations are wrong in many slots) and the
// host callbacks over the game's services (§14).
//
// One object, four interfaces, one reference count: the primary interface
// (fire cISC4FireProtectionSimulator 0x88F54C7B / police cISC4PoliceSimulator
// 0x67659895, also cIGZUnknown), cIGZMessageTarget2, the dispatch source
// (0xC9C00435) and cIGZSerializable. Which sub-object answers an IID is the
// core's decision (fc4_protection_query_interface).
//
// Both grids are the GAME's own cSC4SimGrid (uint8 adjusted, int16
// unadjusted, §2; FC4HostGrid.h): the map view reads the adjusted grid raw.
//
// Threads (protection.md §13.1, PR7). No protection slot is known to be
// called off the main thread: every reader of the game's fire and police
// globals is main-thread code [inferred: static reachability, no thread
// trace]; the original answers live values without locking. The game does
// call other simulators' getters from a second thread while the main thread
// is inside a tick (HANDOFF §5.1), and the core is single-threaded, so every
// call into it still holds this object's lock ([choice] kept while §13.1 is
// inferred). The main thread waits for the lock; any other thread tries for
// up to kLockWaitMs and otherwise answers from a snapshot of the cheap scalar
// values (taken at the end of every main-thread call) or with the neutral
// value, and counts a fallback (the month status line, fallbacks=). The
// fallbacks are NOT original behaviour (§13.1): a non-zero count means an
// off-main caller exists and the answers may differ from the original's.
// Nothing a getter on another thread reads lives in a RefCell outside that
// lock.
//
// The subsystem is OFF by default: FreeCity4.ini `enable = protection` turns
// both classes on together (§13.2: [choice] replace both together);
// `disable` still wins. The protection_* switches (FC4Config.h) turn groups of
// game calls off for bisecting.
#pragma once

#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <atomic>
#include <cstdint>
#include <memory>
#include <vector>

class cIGZMessageServer2;
class cISC4City;

static const uint32_t kFC4_CLSID_cSC4FireProtectionSimulator = 0x2990C05A;
static const uint32_t kFC4_CLSID_cSC4PoliceSimulator = 0x6990C04F;
static const uint32_t kFC4_IID_cISC4FireProtectionSimulator = 0x88F54C7B;
static const uint32_t kFC4_IID_cISC4PoliceSimulator = 0x67659895;
static const uint32_t kFC4_IID_cISC4DispatchSource = 0xC9C00435;

namespace fc4 {

// What the protection host callbacks work on. The simulator owns one; the
// grid shadow (FC4ProtectionShadow.h) makes a read-only one over the city it
// reads. Main thread, or another thread holding the simulator's lock.
struct ProtectionContext {
    uint32_t kind = 0;            // 0 = fire, 1 = police
    const char* tag = "fire";     // log prefix
    bool readOnly = false;        // the shadow: borrowed city, no game changes
    cISC4City* city = nullptr;    // simulator: held from acquire_services to release_services
    cIGZMessageServer2* msgServer = nullptr;  // simulator only, held likewise
    cIGZMessageTarget2* target = nullptr;     // simulator only: the agent and notification target
    cFC4ISC4DispatchSource* source = nullptr;  // simulator only: the dispatch-source sub-object
    uint64_t selfToken = 0;       // simulator only: the primary interface (void1 / IGZUnknown of §12)
    uint64_t airToken = 0;        // the air source the core holds (created here or read from a save)
    bool agentsRegistered = false;
    bool dispatchRegistered = false;
    // Objects kept alive until the outermost call into the core returns.
    std::vector<std::shared_ptr<void>> held;
    // Observability (the month status line).
    uint32_t posts = 0;           // messages posted (§12)
    uint32_t airInits = 0;        // air source Init calls made
    uint32_t controllers = 0;     // Add/RemoveControllersFromOccupantGroup calls made
};

// The host table over `ctx` (the simulator's).
fc4_protection_host MakeProtectionHost(ProtectionContext* ctx, const HostGridContext* grids);
// The read-only table for the grid shadow: every callback that would change
// the game (lifecycle, budget writes, property writes, automata, posts, air
// sources, references) is NULL; the grid API is empty (the shadow's grids
// are the core's own).
fc4_protection_host MakeProtectionReadOnlyHost(ProtectionContext* ctx);

}  // namespace fc4

// The shared implementation: cIGZMessageTarget2 (§5), cIGZSerializable (§11),
// the dispatch source (§8, §15.4), the lock, and the bodies of the primary
// slots both classes share. Not a class of its own (§1: there is no
// registered class for the base alone); the two final classes below put
// their primary interface first.
class cFC4ProtectionBase : public cIGZMessageTarget2, public cIGZSerializable, public cFC4ISC4DispatchSource {
public:
    // cIGZMessageTarget2 (§5)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§11)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // The dispatch source (§8, §15.4)
    uint32_t CanHandleDispatchType(uint32_t type) override;
    bool CanDispatch(float x, float z, bool* allBusy) override;
    int32_t GetMaxPossibleDispatchCount() override;
    uint32_t GetStations(SC4List<cISC4Occupant*>& list) override;
    bool GetDispatchesAtStation(cISC4Occupant* station, uint32_t* flags, uint32_t* base, uint32_t* count) override;
    cISC4Occupant* DispatchFromNearestStation(float x, float z, uint32_t* id) override;
    bool DispatchFromStation(cISC4Occupant* station, uint32_t* id) override;
    bool RevokeDispatchFromStation(cISC4Occupant* station, uint32_t id) override;
    bool GetDispatchAutomataProperties(uint32_t a, uint32_t* b, float* c, uint32_t* d, uint32_t e) override;

    // How long a thread other than the main one waits for the lock.
    static constexpr uint32_t kLockWaitMs = 20;

    // Observability: calls answered from the snapshot / neutral value (not
    // original behaviour, §13.1 / PR7: expected to stay 0).
    uint32_t Fallbacks() const { return fallbacks_.load(); }

protected:
    explicit cFC4ProtectionBase(uint32_t kind);
    ~cFC4ProtectionBase();
    // Second construction step, from the final class: the primary interface
    // (its identity for QueryInterface, the core and §12) and the core.
    void Construct(cIGZUnknown* primary);

    // Holds the lock for one call into the core (see the file comment). On a
    // thread other than the main one ok() may be false: answer from the
    // snapshot or with the neutral value.
    class Section {
    public:
        explicit Section(cFC4ProtectionBase* s);
        ~Section();
        Section(const Section&) = delete;
        Section& operator=(const Section&) = delete;
        bool ok() const { return ok_; }

    private:
        cFC4ProtectionBase* s_;
        bool ok_;
    };

    // The scalar values a getter on another thread falls back to.
    struct Snapshot {
        std::atomic<uint64_t> grid{0}, department{0}, jailDepartment{0};
        std::atomic<uint32_t> avgBits{0}, pctBits{0}, jailedPctBits{0};
        std::atomic<uint32_t> stations{0}, onStrike{0}, strikeDate{0};
        std::atomic<uint32_t> jailCapacity{0}, jailPopulation{0}, criminals{0}, arrests{0}, jails{0};
    };

    // QueryInterface (§1): the core says which sub-object answers; AddRef on
    // success; the out pointer is left untouched otherwise.
    bool QueryImpl(uint32_t riid, void** ppvObj);
    uint32_t AddRefImpl() { return ++refCount_; }
    // §1: true when this Release is the last one (count <= 1): the caller
    // then shuts down and deletes the object and returns 0.
    bool ReleaseIsFinal(uint32_t* remaining);

    // Bodies of the primary slots both classes have (§15.1, §15.2).
    bool InitImpl();
    bool ShutdownImpl();
    cISC4SimGrid<uint8_t>* GridImpl();
    cISC4DepartmentBudget* DepartmentImpl();
    bool IsStationImpl(cISC4Occupant* occupant);
    bool GetStationFundingImpl(cISC4Occupant* occupant, int32_t* cost, SC4Percentage* funding);
    bool SetStationFundingImpl(cISC4Occupant* occupant, SC4Percentage const& funding);
    SC4Percentage* StationEfficiencyImpl(SC4Percentage* result, cISC4Occupant* occupant);
    bool IsOnStrikeImpl();
    SC4Percentage* ChanceOfStrikeImpl(SC4Percentage* result);
    bool BeginStrikeImpl(cISC4Occupant* station);
    bool EndStrikeImpl();
    uint32_t StrikeStartDateImpl();
    uint32_t StrikeLengthImpl();
    int32_t StrikingStationsImpl(SC4List<cISC4Occupant*>& list);
    bool IsStationOnStrikeImpl(cISC4Occupant* station);
    float AverageCoverageImpl();
    SC4Percentage const* PercentCoverageImpl();
    bool OccupantCoverageImpl(cISC4Occupant* station, SC4Percentage const& p, float& coverRadius,
                              float& stationRadius);
    int32_t StationsImpl(SC4List<cISC4Occupant*>& list);
    uint32_t StationCountImpl();
    float StationDispatchRadiusImpl(cISC4Occupant* station);
    int32_t TotalDispatchCountImpl();
    // Police only (§9, §15.2).
    cISC4DepartmentBudget* JailDepartmentImpl();
    int32_t ResetUnjailedCriminalsImpl();
    uint32_t JailCapacityOfImpl(cISC4Occupant* jail);
    uint32_t JailCapacityImpl();
    uint32_t JailPopulationImpl();
    uint32_t CriminalCountImpl();
    int32_t JailsImpl(SC4List<cISC4Occupant*>& list);
    bool IsJailImpl(cISC4Occupant* occupant);
    uint32_t ArrestCountImpl();
    uint32_t JailCountImpl();
    SC4Percentage* JailedPercentageImpl(SC4Percentage* result);
    int64_t JailUpkeepCostImpl(cISC4Occupant* jail);
    bool ProcessMonthlyCrimeStatsImpl();
    bool IsInJailbreakImpl(cISC4Occupant* jail);
    bool BeginJailbreakImpl(cISC4Occupant* jail);
    bool EndJailbreakImpl(cISC4Occupant* jail);
    int32_t JailbreakBuildingsImpl(SC4List<cISC4Occupant*>& list);
    uint32_t StationArrestCountImpl(cISC4Occupant* station);
    uint32_t StationCrimeCountImpl(cISC4Occupant* station);

    bool destroying_ = false;

private:
    bool Enter();
    void Leave();
    void RefreshSnapshot();
    void LogState(const char* what);
    void Hold(cIGZUnknown* object);

    uint32_t kind_;
    cIGZUnknown* primary_ = nullptr;
    uint32_t refCount_ = 0;
    fc4::ProtectionContext ctx_;
    fc4::HostGridContext gridCtx_;
    fc4_protection_host host_{};
    fc4_protection* core_ = nullptr;

    CRITICAL_SECTION lock_;
    DWORD mainTid_ = 0;
    int depth_ = 0;  // nesting of sections held (by the lock holder)
    std::atomic<uint32_t> fallbacks_{0};
    Snapshot snap_;
    SC4Percentage pct_{0.0f};  // what GetPercentCoverage points at (stable)
    uint32_t months_ = 0;
    uint32_t days_ = 0;
    // The month line's WARNING about a 0 x 0 coverage grid, once per object.
    bool emptyGridWarned_ = false;
};

class cFC4FireProtectionSimulator final : public cFC4ISC4FireProtectionSimulator, public cFC4ProtectionBase {
public:
    cFC4FireProtectionSimulator();
    ~cFC4FireProtectionSimulator() = default;

    // cIGZUnknown (all four interfaces)
    bool QueryInterface(uint32_t riid, void** ppvObj) override { return QueryImpl(riid, ppvObj); }
    uint32_t AddRef() override { return AddRefImpl(); }
    uint32_t Release() override;

    // cISC4FireProtectionSimulator (§15.1)
    bool Init() override { return InitImpl(); }
    bool Shutdown() override { return ShutdownImpl(); }
    cISC4SimGrid<uint8_t>* GetFireProtectionGrid() override { return GridImpl(); }
    cISC4DepartmentBudget* GetDepartmentBudget() override { return DepartmentImpl(); }
    bool IsFireStation(cISC4Occupant* o) override { return IsStationImpl(o); }
    bool GetStationFunding(cISC4Occupant* o, int32_t* cost, SC4Percentage* f) override {
        return GetStationFundingImpl(o, cost, f);
    }
    bool SetStationFunding(cISC4Occupant* o, SC4Percentage const& f) override { return SetStationFundingImpl(o, f); }
    SC4Percentage* GetStationEfficiency(SC4Percentage* r, cISC4Occupant* o) override {
        return StationEfficiencyImpl(r, o);
    }
    bool IsOnStrike() override { return IsOnStrikeImpl(); }
    SC4Percentage* GetChanceOfStrike(SC4Percentage* r) override { return ChanceOfStrikeImpl(r); }
    bool BeginStrike(cISC4Occupant* s) override { return BeginStrikeImpl(s); }
    bool EndStrike() override { return EndStrikeImpl(); }
    uint32_t GetStrikeStartDate() override { return StrikeStartDateImpl(); }
    uint32_t GetStrikeLength() override { return StrikeLengthImpl(); }
    int32_t GetStrikingStations(SC4List<cISC4Occupant*>& l) override { return StrikingStationsImpl(l); }
    bool IsStationOnStrike(cISC4Occupant* s) override { return IsStationOnStrikeImpl(s); }
    float GetAverageCoverage() override { return AverageCoverageImpl(); }
    SC4Percentage const* GetPercentCoverage() override { return PercentCoverageImpl(); }
    bool GetOccupantCoverage(cISC4Occupant* s, SC4Percentage const& p, float& cover, float& station) override {
        return OccupantCoverageImpl(s, p, cover, station);
    }
    int32_t GetFireStations(SC4List<cISC4Occupant*>& l) override { return StationsImpl(l); }
    uint32_t GetFireStationCount() override { return StationCountImpl(); }
    float GetStationDispatchRadius(cISC4Occupant* s) override { return StationDispatchRadiusImpl(s); }
    int32_t GetTotalDispatchCount() override { return TotalDispatchCountImpl(); }
};

class cFC4PoliceSimulator final : public cFC4ISC4PoliceSimulator, public cFC4ProtectionBase {
public:
    cFC4PoliceSimulator();
    ~cFC4PoliceSimulator() = default;

    // cIGZUnknown (all four interfaces)
    bool QueryInterface(uint32_t riid, void** ppvObj) override { return QueryImpl(riid, ppvObj); }
    uint32_t AddRef() override { return AddRefImpl(); }
    uint32_t Release() override;

    // cISC4PoliceSimulator (§15.2)
    bool Init() override { return InitImpl(); }
    bool Shutdown() override { return ShutdownImpl(); }
    cISC4SimGrid<uint8_t>* GetPolicePowerGrid() override { return GridImpl(); }
    cISC4DepartmentBudget* GetDepartmentBudget() override { return DepartmentImpl(); }
    bool GetStationFunding(cISC4Occupant* o, int32_t* cost, SC4Percentage* f) override {
        return GetStationFundingImpl(o, cost, f);
    }
    bool SetStationFunding(cISC4Occupant* o, SC4Percentage const& f) override { return SetStationFundingImpl(o, f); }
    cISC4DepartmentBudget* GetJailDepartmentBudget() override { return JailDepartmentImpl(); }
    bool IsOnStrike() override { return IsOnStrikeImpl(); }
    SC4Percentage* GetChanceOfStrike(SC4Percentage* r) override { return ChanceOfStrikeImpl(r); }
    bool BeginStrike(cISC4Occupant* s) override { return BeginStrikeImpl(s); }
    bool EndStrike() override { return EndStrikeImpl(); }
    uint32_t GetStrikeStartDate() override { return StrikeStartDateImpl(); }
    uint32_t GetStrikeLength() override { return StrikeLengthImpl(); }
    int32_t GetStrikingStations(SC4List<cISC4Occupant*>& l) override { return StrikingStationsImpl(l); }
    bool IsStationOnStrike(cISC4Occupant* s) override { return IsStationOnStrikeImpl(s); }
    float GetAverageCoverage() override { return AverageCoverageImpl(); }
    SC4Percentage const* GetPercentCoverage() override { return PercentCoverageImpl(); }
    bool GetOccupantCoverage(cISC4Occupant* s, SC4Percentage const& p, float& cover, float& station) override {
        return OccupantCoverageImpl(s, p, cover, station);
    }
    int32_t GetPoliceStations(SC4List<cISC4Occupant*>& l) override { return StationsImpl(l); }
    bool IsPoliceStation(cISC4Occupant* o) override { return IsStationImpl(o); }
    float GetStationDispatchRadius(cISC4Occupant* s) override { return StationDispatchRadiusImpl(s); }
    uint32_t GetPoliceStationCount() override { return StationCountImpl(); }
    SC4Percentage* GetStationEfficiency(SC4Percentage* r, cISC4Occupant* o) override {
        return StationEfficiencyImpl(r, o);
    }
    int32_t GetTotalDispatchCount() override { return TotalDispatchCountImpl(); }
    int32_t ResetUnjailedCriminals() override { return ResetUnjailedCriminalsImpl(); }
    uint32_t GetJailCapacityOf(cISC4Occupant* j) override { return JailCapacityOfImpl(j); }
    uint32_t GetJailCapacity() override { return JailCapacityImpl(); }
    uint32_t GetJailPopulation() override { return JailPopulationImpl(); }
    uint32_t GetCriminalCount() override { return CriminalCountImpl(); }
    int32_t GetJails(SC4List<cISC4Occupant*>& l) override { return JailsImpl(l); }
    bool IsJail(cISC4Occupant* o) override { return IsJailImpl(o); }
    uint32_t GetArrestCount() override { return ArrestCountImpl(); }
    uint32_t GetJailCount() override { return JailCountImpl(); }
    SC4Percentage* GetJailedPercentage(SC4Percentage* r) override { return JailedPercentageImpl(r); }
    int64_t GetJailCurrentUpkeepCost(cISC4Occupant* j) override { return JailUpkeepCostImpl(j); }
    bool ProcessMonthlyCrimeStats() override { return ProcessMonthlyCrimeStatsImpl(); }
    bool IsInJailbreak(cISC4Occupant* j) override { return IsInJailbreakImpl(j); }
    bool BeginJailbreak(cISC4Occupant* j) override { return BeginJailbreakImpl(j); }
    bool EndJailbreak(cISC4Occupant* j) override { return EndJailbreakImpl(j); }
    int32_t GetJailbreakBuildings(SC4List<cISC4Occupant*>& l) override { return JailbreakBuildingsImpl(l); }
    uint32_t GetPoliceStationArrestCount(cISC4Occupant* s) override { return StationArrestCountImpl(s); }
    uint32_t GetPoliceStationCrimeCount(cISC4Occupant* s) override { return StationCrimeCountImpl(s); }
};

// Factories for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4FireProtectionSimulator();
cIGZUnknown* CreateFC4PoliceSimulator();
