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

// The pollution and garbage simulator's GZCOM class (docs/specs/pollution.md
// §1, CLSID 0x8990C065). The simulation lives in the Rust core behind fc4.h
// (fc4_pollution); this class implements cISC4PollutionSimulator in the game's
// compiled slot order (cFC4ISC4PollutionSimulator, §8.1), cIGZMessageTarget2
// (§8.3), cISC4PropRequester (§8.4) and cIGZSerializable (§8.2), and the host
// callbacks (fc4_pollution_host) over the game's services (§9).
//
// The eight grids are the GAME's own cSC4SimGrid<int16> (§10.1 finding 2: the
// aura simulator, growth developer and map view read them as raw memory),
// created through GZCOM with class 0x49B9E605.
//
// The subsystem is OFF by default: FreeCity4.ini `enable = pollution` turns it
// on (FC4Subsystems.h); `disable` still wins. The pollution_* switches
// (FC4Config.h) turn groups of game calls off for bisecting.
//
// Threads: the game calls the getters from a second thread while the main
// thread is inside a day or a month (the core keeps the state behind a lock).
// Only the lifecycle, DoMessage, Write / Read and slot 66 count as calls into
// the core (CoreCall); the getters touch nothing on this object but the core
// handle (and slot 33 its own float).
//
// Not implemented here: the visual effects of §4.10 (the effects manager's
// CreateVisualEffect and cISC4VisualEffect / cS3DTransform are not in
// docs/abi; logged once, spec question). Everything else of §9 is called.
#pragma once

#include "FC4Config.h"
#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#include <cstdint>
#include <memory>
#include <vector>

class cIGZMessageServer2;
class cISC4City;
class cISC4LandfillDeveloper;
class cISC4LotManager;
class cISC4NeighborsSimulator;
class cISC4OrdinanceSimulator;
class cISC4PowerSimulator;
class cISC4PropDeveloper;
class cISC4ResidentialSimulator;
class cISC4TrafficSimulator;
class cISC4ZoneManager;
class cISC4HistoryWarehouse;
class cISTETerrain;

static const uint32_t kFC4_CLSID_cSC4PollutionSimulator = 0x8990C065;
static const uint32_t kFC4_IID_cISC4PollutionSimulator = 0x877181EC;
static const uint32_t kFC4_IID_cISC4PropRequester = 0x89C498BE;

// The game services the pollution host callbacks read (§9). The simulator
// fills it with references of its own (AcquireServices); the pollution shadow
// (FC4PollutionShadow.h) with the city's borrowed pointers, for one delivery.
struct FC4PollutionGame {
    cISC4City* city = nullptr;
    cISC4ZoneManager* zones = nullptr;
    cISC4TrafficSimulator* traffic = nullptr;
    cISC4OrdinanceSimulator* ordinance = nullptr;
    cISC4NeighborsSimulator* neighbors = nullptr;
    cISC4PowerSimulator* power = nullptr;
    cISC4ResidentialSimulator* residential = nullptr;
    cISC4LandfillDeveloper* landfill = nullptr;
    cISC4BudgetSimulator* budget = nullptr;
    cISTETerrain* terrain = nullptr;
    cISC4LotManager* lotManager = nullptr;
    cISC4PropDeveloper* props = nullptr;
    cISC4HistoryWarehouse* history = nullptr;
    // Gates the reads with the pollution_* switches (ours); the shadow reads
    // the original's inputs as they are.
    bool gated = true;
    // Game lists (and the references they hold) kept until the outermost call
    // into the core returns (ours) or the shadowed delivery ends (shadow).
    std::vector<std::shared_ptr<void>> held;
    void Hold(std::shared_ptr<void> list) { held.push_back(std::move(list)); }
};

namespace fc4 {
// A host table with the read callbacks only (city geometry, tuning, terrain,
// zones, traffic, ordinance, deals, power, population, landfill, occupant
// reads, department funding, budget items) over `game`, and a grid table with
// geometry and reads only (`grids` must outlive the table). Every write
// callback is NULL. For the pollution shadow.
fc4_pollution_host PollutionReadOnlyHost(FC4PollutionGame* game, const HostGridContext* grids);
}  // namespace fc4

class cFC4PollutionSimulator final : public cFC4ISC4PollutionSimulator,
                                     public cIGZMessageTarget2,
                                     public cFC4ISC4PropRequester,
                                     public cIGZSerializable,
                                     public FC4PollutionGame {
public:
    cFC4PollutionSimulator();
    ~cFC4PollutionSimulator();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4PollutionSimulator (§8.1, compiled order)
    bool Init() override;
    bool Shutdown() override;
    cISC4SimGrid<int16_t>* GetPollutionGrid(uint32_t type) override;
    uint16_t GetPollutionValue(uint32_t type, uint32_t cellX, uint32_t cellZ) override;
    bool GetAirValue(uint32_t cellX, uint32_t cellZ, int32_t& value) override;
    bool GetWaterValue(uint32_t cellX, uint32_t cellZ, int32_t& value) override;
    bool GetGarbageValue(uint32_t cellX, uint32_t cellZ, int32_t& value) override;
    int32_t GetAverageAirValue() override;
    int32_t GetAverageWaterValue() override;
    int32_t GetAverageGarbageValue() override;
    bool IsAirPolluted(uint32_t cellX, uint32_t cellZ) override;
    bool IsWaterPolluted(uint32_t cellX, uint32_t cellZ) override;
    bool IsGarbageAccumulated(uint32_t cellX, uint32_t cellZ) override;
    bool IsRadioactive(uint32_t cellX, uint32_t cellZ) override;
    bool GetWorstAirTract(uint32_t& cellX, uint32_t& cellZ, int32_t& value) override;
    bool GetWorstWaterTract(uint32_t& cellX, uint32_t& cellZ, int32_t& value) override;
    bool GetWorstGarbageTract(uint32_t& cellX, uint32_t& cellZ, int32_t& value) override;
    uint32_t GetGarbageSupply() override;
    int32_t GetLandfillTileCapacity() override;
    int32_t GetGarbageCapacity() override;
    bool IsGarbageUncollected() override;
    int32_t GetTotalGarbageUncollected() override;
    bool SetIsRadioactive(uint32_t cellX, uint32_t cellZ, bool radioactive) override;
    bool SetAirValue(uint32_t cellX, uint32_t cellZ, int16_t value) override;
    bool SetWaterValue(uint32_t cellX, uint32_t cellZ, int16_t value) override;
    bool SetGarbageValue(uint32_t cellX, uint32_t cellZ, int16_t value) override;
    int32_t AddToGarbageSupply(int32_t amount) override;
    int32_t SetGarbageCapacity(int32_t capacity) override;
    int32_t GetMonthlyIncineratorCapacity() override;
    int32_t GetMonthlyWasteToEnergyCapacity() override;
    SC4Percentage* GetRecyclingCenterEffect() override;
    int64_t GetGarbageAbductedPrevMonth() override;
    int64_t GetGarbageProducedLastMonth() override;
    int64_t GetGarbageImportedLastMonth() override;
    int64_t GetGarbageExportedLastMonth() override;
    int64_t GetGarbageRecycledLastMonth() override;
    int64_t GetGarbageIncineratedLastMonth() override;
    int64_t GetGarbageConvertedToEnergyLastMonth() override;
    int64_t GetGarbageLandfilledLastMonth() override;
    uint32_t GetIncineratorCount() override;
    uint32_t GetWasteToEnergyBuildingCount() override;
    uint32_t GetRecyclingCenterCount() override;
    uint32_t GetWaterTreatmentPlantCount() override;
    bool GetGarbageBuildings(SC4List<cISC4Occupant*>& list) override;
    uint32_t GetBuildingGarbageCapacity(cISC4Occupant* occupant) override;
    float GetGarbageScalingFactor() override;
    bool IsActiveLandfill(uint32_t cellX, uint32_t cellZ) override;
    bool SetIsActiveLandfill(uint32_t cellX, uint32_t cellZ, bool active) override;
    int64_t GarbagePutInLandfill(int32_t amount) override;
    bool GetMaxCapacity(uint32_t property, cISC4Occupant* occupant, uint32_t& result) override;
    bool GetCurrentCapacity(uint32_t property, cISC4Occupant* occupant, uint32_t& result) override;
    bool GetRecyclingCenterMaxCapacity(cISC4Occupant* occupant, SC4Percentage& capacity) override;
    bool GetRecyclingCenterCurrentCapacity(cISC4Occupant* occupant, SC4Percentage& capacity) override;
    bool GetCurrentOccupantCondition(cISC4Occupant* occupant, SC4Percentage& condition) override;
    bool GetCurrentOccupantEfficiency(cISC4Occupant* occupant, SC4Percentage& efficiency) override;
    bool GetIncineratorUsedCapacity(cISC4Occupant* occupant, uint32_t& capacity) override;
    bool GetWasteToEnergyUsedCapacity(cISC4Occupant* occupant, uint32_t& capacity) override;
    bool GetIncineratorCurrentCapacity(cISC4Occupant* occupant, uint32_t& capacity) override;
    bool GetWasteToEnergyCurrentCapacity(cISC4Occupant* occupant, uint32_t& capacity) override;
    bool GetWaterTreatmentMaxCapacity(cISC4Occupant* occupant, uint32_t& capacity) override;
    bool GetWaterTreatmentCurrentCapacity(cISC4Occupant* occupant, uint32_t& capacity) override;
    bool GetWaterTreatmentUsedCapacity(cISC4Occupant* occupant, uint32_t& capacity) override;
    bool CalcOccupantMaintenanceCost(cISC4Occupant* occupant, int64_t& cost) override;
    void AddRadiationToCity(cS3DVector3 const& position, float strength, float radius) override;
    int32_t GetMaxAirPollutionLevelForUI() override;
    int32_t GetMaxWaterPollutionLevelForUI() override;
    int32_t GetMaxGarbagePollutionLevelForUI() override;
    int32_t GetMaxRadiationPollutionLevelForUI() override;

    // cIGZMessageTarget2 (§8.3)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cISC4PropRequester (§8.4, §4.11)
    int32_t GetRequestedShare(SC4Rect<int32_t> const& cells) override;
    uint32_t GetRequesterSatisfactionId() override;
    bool UsesCustomPropSelector() override;
    uint32_t SelectProp(fc4::GameVector<uint32_t> const& candidates, int32_t a, int32_t b) override;

    // cIGZSerializable (§6, §8.2)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // For the host callbacks (cFC4Pollution.cpp).
    cIGZMessageTarget2* Target() { return static_cast<cIGZMessageTarget2*>(this); }
    cFC4ISC4PropRequester* Requester() { return static_cast<cFC4ISC4PropRequester*>(this); }
    // The token the core knows this object by (its cISC4PollutionSimulator*,
    // the identity the primary QueryInterface hands out).
    uint64_t SelfToken() const {
        return static_cast<uint64_t>(
            reinterpret_cast<uintptr_t>(static_cast<const cFC4ISC4PollutionSimulator*>(this)));
    }
    void Subscribe(const uint32_t* ids, size_t count, bool add);

    // Counts the calls into the core; the outermost one drops the held lists.
    class CoreCall {
    public:
        explicit CoreCall(cFC4PollutionSimulator* s) : s_(s) { ++s_->depth_; }
        ~CoreCall() {
            if (--s_->depth_ == 0) s_->held.clear();
        }
        CoreCall(const CoreCall&) = delete;
        CoreCall& operator=(const CoreCall&) = delete;

    private:
        cFC4PollutionSimulator* s_;
    };

private:
    // §4.1 step 4 (and every day, for a service the city created later): each
    // service only while its member is still null.
    void AcquireServices();
    void ReleaseServices();
    void LogState(const char* what);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::HostGridContext gridCtx_{"pollution", fc4::Sw::PollutionGridWrites};
    fc4_pollution_host host_{};
    fc4_pollution* core_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    SC4Percentage recyclingEffect_{};  // slot 33 points here
    int depth_ = 0;
    uint32_t days_ = 0;
    uint32_t months_ = 0;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4PollutionSimulator();
