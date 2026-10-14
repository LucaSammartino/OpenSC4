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

// The residential simulator's GZCOM class (docs/specs/residential.md and
// residential-education-health.md; CLSID 0x4990C013, city component 36). The
// model lives in the Rust core behind fc4.h (fc4_residential); this class
// implements the three interfaces of one object (residential.md §1: the
// primary cISC4ResidentialSimulator at +0, cIGZMessageTarget2 at +4,
// cIGZSerializable at +8), in the compiled slot order of §10 (FC4Interfaces.h
// fixes the SDK errors), and the host callbacks over the game's services
// (§11.1, REH §8).
//
// Grids are the GAME's own cSC4SimGrid<T>, created through GZCOM with the
// game's class IDs, tract sizes and instance IDs (FC4HostGrid.h). Every input
// is read through a game interface; a read that cannot be made (no service,
// a switch off) answers the spec's neutral value and is logged once.
//
// Threads: the core is main-thread only and publishes a snapshot and a
// detached state after every outermost main-thread call (fc4.h). The getters
// read the snapshot or run on a view of the detached state when called from
// another thread or during a call (the capi decides); a state change from
// another thread is queued by the core. No getter here reads a member that
// is not an atomic or the core's own published copy. Init, Shutdown, Read
// and the status lines run on the main thread only.
//
// AL-only returns (§10.1): slots 29, 34 and 42 return the uint8 zero-extended
// in EAX (an inline-asm epilogue, so no caller sees garbage in bits 8-31).
//
// The subsystem is OFF by default: FreeCity4.ini `enable = residential` turns
// it on (FC4Subsystems.h); `disable` still wins. The residential_* switches
// (FC4Config.h) turn groups of game calls off for bisecting.
//
// Log lines (FreeCity4.log):
//   residential Init|Read|Shutdown: ... (the state)
//   residential Write -> 0|1
//   residential month YYYY-MM-DD: pop=N (R$ a R$$ b R$$$ c) EQ=e HQ=h LE=l
//     school covered=.. buildings=.. strike=0|1 hospital covered=.. buildings=..
//     strike=0|1 state=S ... center x87=c/n (lot GetCenter calls that changed
//     the x87 stack depth / calls; c = 0 in the game) ... (one at the first
//     SimNewDay after Init, then one per SimNewMonth)
//   residential cycle N done YYYY-MM-DD: ... (one per monthly cycle that
//     reached state 10)
#pragma once

#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#include <atomic>
#include <cstdint>

class cIGZMessageServer2;
class cISC4BudgetSimulator;
class cISC4BuildingDevelopmentSimulator;
class cISC4City;
class cISC4Demand;
class cISC4GrowthDeveloper;
class cISC4OccupantManager;
class cISC4Simulator;

static const uint32_t kFC4_CLSID_cSC4ResidentialSimulator = 0x4990C013;
static const uint32_t kFC4_IID_cISC4ResidentialSimulator = 0x077AC1EE;

namespace fc4 {

// What the residential host callbacks work on. The simulator owns one; the
// shadow (FC4ResidentialShadow.h) makes a read-only one over the city it
// reads.
struct ResidentialContext {
    bool readOnly = false;  // the shadow: borrowed city, read callbacks only
    bool gated = true;      // false: the residential_* switches are ignored (the shadow reads as it is)
    // Simulator: held from acquire_services to release_services (§4.1 step 1,
    // §4.2). Shadow: the city borrowed for one delivery, the rest fetched
    // from it when used.
    cISC4City* city = nullptr;
    cISC4BuildingDevelopmentSimulator* bds = nullptr;
    cISC4OccupantManager* occupantManager = nullptr;
    cISC4GrowthDeveloper* developers[3] = {};  // 0x1010, 0x1020, 0x1030
    cISC4Demand* demandNode = nullptr;         // GetDemand(0x1000, 0): only held (§4.1)
    cIGZMessageServer2* msgServer = nullptr;   // step 2
    cISC4BudgetSimulator* budget = nullptr;    // step 3
    // step 4: the simulator is borrowed from the city each time (RG18).
    cIGZMessageTarget2* target = nullptr;      // simulator only: the agent and notification target
    bool agentsRegistered = false;
    // Observability (the status lines): calls made since construction.
    uint32_t posts = 0;        // strike messages posted (REH §5)
    uint32_t history = 0;      // history records sent (§8)
    uint32_t controllers = 0;  // automata controller calls (REH §4.9)
    uint32_t departments = 0;  // departments created (REH §4.2)
    uint32_t effects = 0;      // visual effects asked for (§5.5 step 2, RG1)
    uint32_t effectsStarted = 0;  // ... created, positioned and started
    uint32_t effectsFailed = 0;   // ... the effects manager returned none (one draw)
    uint32_t centers = 0;      // lot GetCenter calls (lot slot 15)
    uint32_t centerX87 = 0;    // ... of which changed the x87 stack depth (crime.md §13, CQ3: 0 in the game)
};

// The host table over `ctx` (the simulator's).
fc4_residential_host MakeResidentialHost(ResidentialContext* ctx, const HostGridContext* grids);
// The read-only table for the shadow: tuning, the monthly pass's inputs,
// occupant, budget, line item and ordinance reads, and the grid table's
// geometry / get (no create, set, fill, shutdown or release). Every
// callback that would change the game is NULL; the shadow core keeps those
// writes itself (fc4.h).
fc4_residential_host MakeResidentialReadOnlyHost(ResidentialContext* ctx, const HostGridContext* grids);

}  // namespace fc4

class cFC4ResidentialSimulator final : public cFC4ISC4ResidentialSimulator,
                                       public cIGZMessageTarget2,
                                       public cIGZSerializable {
public:
    cFC4ResidentialSimulator();
    // Class slots of the primary vtable after the 60 interface slots, in this
    // order (§10.1): 60 the scalar deleting destructor, 61
    // GetHealthAffectingBuildings, 62 TrackTracts, 63 OutputTractInfo. No
    // base declares a virtual destructor, so all four are new primary slots.
    virtual ~cFC4ResidentialSimulator();
    virtual void GetHealthAffectingBuildings(SC4List<cISC4Occupant*>& list);  // 61
    virtual void TrackTracts();                                               // 62: no-op
    virtual void OutputTractInfo(int32_t a, int32_t b);                       // 63: no-op

    // cIGZUnknown (§1: one object, one reference count)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4ResidentialSimulator (§10.1, REH §7)
    bool Init() override;
    bool Shutdown() override;
    cISC4SimGrid<uint8_t>* GetProximityMap(WealthType wealth) override;
    bool SchoolIsOnStrike() override;
    bool HealthIsOnStrike() override;
    bool EndSchoolStrike() override;
    bool EndHealthStrike() override;
    SC4Percentage ChanceOfSchoolStrike() override;
    SC4Percentage ChanceOfHealthStrike() override;
    SC4Percentage GetSchoolSystemRating() override;
    SC4Percentage GetHealthSystemRating() override;
    uint32_t GetSchoolSystemTotals(FC4ResidentialQueryData* totals, SC4Vector<uint32_t> const* groups) override;
    uint32_t GetHospitalSystemTotals(FC4ResidentialQueryData* totals, SC4Vector<uint32_t> const* groups) override;
    int32_t GetPopulation() override;
    int32_t GetTotalCityEducationUpkeepCost() override;
    int32_t GetTotalCityHealthUpkeepCost() override;
    bool SetOccupantFundingPercentages(cISC4Occupant* occupant, SC4Percentage const& staff,
                                       SC4Percentage const& vehicle, bool isSchool) override;
    bool GetOccupantFundingPercentages(cISC4Occupant* occupant, SC4Percentage& staff, SC4Percentage& vehicle,
                                       bool isSchool) override;
    bool GetAverageEQGrid(cISC4SimGrid<float>*& grid, float* min, float* max) override;
    bool GetAverageHQGrid(cISC4SimGrid<float>*& grid, float* min, float* max) override;
    bool GetEQGrids(cISC4SimGrid<float>*& g0, cISC4SimGrid<float>*& g1, cISC4SimGrid<float>*& g2) override;
    bool GetHQGrids(cISC4SimGrid<float>*& g0, cISC4SimGrid<float>*& g1, cISC4SimGrid<float>*& g2) override;
    bool GetPopulationGrids(cISC4SimGrid<uint16_t>*& g0, cISC4SimGrid<uint16_t>*& g1,
                            cISC4SimGrid<uint16_t>*& g2) override;
    bool GetSchoolQueryData(cISC4Occupant* occupant, FC4ResidentialQueryData* data) override;
    bool GetHospitalQueryData(cISC4Occupant* occupant, FC4ResidentialQueryData* data) override;
    bool EstimateCurrentOccupantCapacity(cISC4Occupant* occupant, uint32_t& capacity, uint32_t& staff) override;
    uint8_t GetCellLifeExpectancy(uint32_t cellX, uint32_t cellZ) override;
    SC4Percentage GetCellWorkforcePercent(uint32_t cellX, uint32_t cellZ) override;
    SC4Percentage GetGlobalWorkforcePercent() override;
    float GetGlobalEQ() override;
    float GetGlobalHQ() override;
    uint8_t GetGlobalLE() override;
    float GetCellEQ(uint32_t cellX, uint32_t cellZ) override;
    float GetCellHQ(uint32_t cellX, uint32_t cellZ) override;
    float GetCellEQByWealth(uint32_t cellX, uint32_t cellZ, WealthType wealth) override;
    float GetCellHQByWealth(uint32_t cellX, uint32_t cellZ, WealthType wealth) override;
    cISC4SimGrid<float>* GetSchoolAverageGradeMap() override;
    cISC4SimGrid<float>* GetHospitalAverageGradeMap() override;
    cISC4SimGrid<uint8_t>* GetAverageAgeMap() override;
    uint8_t GetAverageNewAgeByWealth(WealthType wealth) override;
    bool GetEQMinAndMaxCellCoords(uint32_t& minX, uint32_t& minZ, uint32_t& maxX, uint32_t& maxZ, float& min,
                                  float& max) override;
    bool GetHQMinAndMaxCellCoords(uint32_t& minX, uint32_t& minZ, uint32_t& maxX, uint32_t& maxZ, float& min,
                                  float& max) override;
    bool GetOccupantCoverage(cISC4Occupant* occupant, SC4Percentage const& level, float& distance,
                             float& radius) override;
    int32_t GetSchoolBuildingCount() override;
    bool GetSchoolBuildings(SC4List<cISC4Occupant*>& buildings, SC4Vector<uint32_t> const* groups) override;
    int32_t GetHospitalBuildingCount() override;
    bool GetHospitalBuildings(SC4List<cISC4Occupant*>& buildings, SC4Vector<uint32_t> const* groups) override;
    int32_t GetMaxEQ() override;
    int32_t GetMaxHQ() override;
    bool GetGlobalAutoBudgetForSchools() override;
    bool SetGlobalAutoBudgetForSchools(bool enable) override;
    bool GetGlobalAutoBudgetForHospitals() override;
    bool SetGlobalAutoBudgetForHospitals(bool enable) override;
    bool GetAutoBudget(cISC4Occupant* occupant) override;
    bool SetAutoBudget(cISC4Occupant* occupant, bool enable) override;
    bool EstimateIdealFunding(cISC4Occupant* occupant, SC4Percentage& funding) override;
    void ToggleTractTracking(int32_t a, int32_t b) override;

    // cIGZMessageTarget2 (§4.3)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§9)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // The token the core knows this object by (its primary interface, the
    // identity QueryInterface hands out for 0x00000001 and 0x077AC1EE).
    uint64_t SelfToken() const {
        return static_cast<uint64_t>(
            reinterpret_cast<uintptr_t>(static_cast<const cFC4ISC4ResidentialSimulator*>(this)));
    }

private:
    fc4_residential_snapshot Snapshot() const;
    uint32_t Buildings(uint32_t which, SC4List<cISC4Occupant*>& list, SC4Vector<uint32_t> const* groups);
    void LogState(const char* what);
    void LogMonth();
    void LogCycle();

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::ResidentialContext ctx_;
    fc4::HostGridContext gridCtx_{"residential", fc4::Sw::ResidentialGridWrites};
    fc4_residential_host host_{};
    fc4_residential* core_ = nullptr;
    // The thread the class store created this object on (the main thread;
    // the core's main thread too). Messages from any other thread are queued
    // by the core and only counted here.
    uint32_t mainTid_ = 0;
    // Status lines (main thread only).
    int32_t lastJulian_ = 0;      // the last SimNewDay's date
    uint32_t lastState_ = 0xFFFFFFFFu;  // the state after the last SimNewDay
    uint32_t days_ = 0, months_ = 0, cycles_ = 0;
    std::atomic<uint32_t> offMainMessages_{0};
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4ResidentialSimulator();
