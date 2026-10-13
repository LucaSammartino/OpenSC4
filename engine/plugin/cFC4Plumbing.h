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

// The plumbing (water) simulator's GZCOM class (docs/specs/plumbing.md §1,
// CLSID 0x0990C075). The simulation lives in the Rust core behind fc4.h
// (fc4_plumbing); this class implements cISC4PlumbingSimulator in the game's
// compiled slot order (cFC4ISC4PlumbingSimulator, §10.1) and the host
// callbacks (fc4_plumbing_host) over the game's services (§11).
//
// Both grids are the GAME's own cSC4SimGrid<uint8> (§12.2 item 1: the map view
// reads the published grid's memory through GetGridData, so a replacement must
// hand out a real one created through GZCOM, class 0x49B9E602).
//
// The subsystem is OFF by default: FreeCity4.ini `enable = plumbing` turns it
// on (FC4Subsystems.h); `disable` still wins. The plumbing_* switches
// (FC4Config.h) turn groups of game calls off for bisecting.
//
// Containers and direct calls (docs/abi): slots 28 appends to the caller's
// ilist (A: SC4List); the occupant query (occupant manager compiled slot 41)
// fills an SC4List (A, one reference each) kept until the outermost call into
// the core returns; FindDeals and the connection queries fill SC4Lists the
// same way. The pipe's non-virtual SetWateredState is called by address on the
// identified executable only (docs/abi/direct-calls.md; §12.2 item 3).
#pragma once

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
class cISC4NeighborsSimulator;
class cISC4OccupantManager;
class cISC4PollutionSimulator;
class cISC4OrdinanceSimulator;
class cISC4LotManager;
class cISTETerrain;

static const uint32_t kFC4_CLSID_cSC4PlumbingSimulator = 0x0990C075;
static const uint32_t kFC4_IID_cISC4PlumbingSimulator = 0x47782CCC;

class cFC4PlumbingSimulator final : public cFC4ISC4PlumbingSimulator,
                                    public cIGZMessageTarget2,
                                    public cIGZSerializable {
public:
    cFC4PlumbingSimulator();
    ~cFC4PlumbingSimulator();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4PlumbingSimulator (§10.1, compiled order)
    bool Init() override;
    bool Shutdown() override;
    uint8_t GetSupplyValue(int32_t x, int32_t z) override;
    bool GetSupplyValueMapPtr(cISC4SimGrid<uint8_t>*& grid) override;
    cISC4SimGrid<uint8_t>* GetWateredGrid() override;
    bool AreWateredBuildingsInCell(int32_t x, int32_t z) override;
    bool IsBuildingWatered(cISC4Occupant* occupant) override;
    uint32_t GetWaterProduction(cISC4Occupant* occupant) override;
    bool IsCellWatered(int32_t x, int32_t z) override;
    uint32_t GetNumWaterSources() override;
    uint32_t GetNumPollutedWaterSources() override;
    int32_t GetWaterProductionCapacity() override;
    int32_t GetWaterConsumed() override;
    int32_t GetWaterImported() override;
    int32_t GetWaterExported() override;
    uint32_t GetNumPipes() override;
    uint32_t GetNumDistressedPipes() override;
    uint32_t GetNumBurstPipes() override;
    uint32_t GetNumWateredBuildings() override;
    uint32_t GetNumUnwateredBuildings() override;
    int32_t GetWaterDemand() override;
    bool GetWaterProducerInfo(cISC4Occupant* producer, uint32_t* currentCapacity, uint32_t* maxCapacity,
                              SC4Percentage* efficiency, bool* shutDownByPollution, uint32_t* waterUsed) override;
    bool GetWaterProducerConditionPercent(cISC4Occupant* producer, SC4Percentage& conditionPercent) override;
    bool GetOccupantMonthlyUpkeepCost(cISC4Occupant* occupant, int64_t& monthlyUpkeepCost) override;
    bool FindWaterBuildings(cISC4Occupant** lowestFunded, SC4Percentage& funding, cISC4Occupant** worstCondition,
                            long& conditionPercent) override;
    bool GetWaterProducers(SC4List<cISC4Occupant*>& list) override;

    // cIGZMessageTarget2 (§5.1)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§9)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // For the host callbacks (cFC4Plumbing.cpp).
    cISC4City* City() const { return city_; }
    cISC4OccupantManager* OccupantManager() const { return occupantManager_; }
    cISC4LotManager* LotManager() const { return lotManager_; }
    cISC4PollutionSimulator* Pollution() const { return pollution_; }
    cISC4OrdinanceSimulator* Ordinance() const { return ordinance_; }
    cISC4NeighborsSimulator* Neighbors() const { return neighbors_; }
    cISC4BudgetSimulator* Budget() const { return budget_; }
    cISTETerrain* Terrain() const { return terrain_; }
    cIGZMessageTarget2* Target() { return static_cast<cIGZMessageTarget2*>(this); }
    // The token the core knows this object by (its cISC4PlumbingSimulator*).
    uint64_t SelfToken() const { return TokenOf(static_cast<const cFC4ISC4PlumbingSimulator*>(this)); }
    void AcquireServices();
    void ReleaseServices();
    void Subscribe(const uint32_t* ids, size_t count, bool add);
    // Keeps a game list (and the references it holds) until the outermost
    // call into the core returns.
    void Hold(std::shared_ptr<void> list) { held_.push_back(std::move(list)); }

    // Counts the calls into the core; the outermost one drops the held lists.
    class CoreCall {
    public:
        explicit CoreCall(cFC4PlumbingSimulator* s) : s_(s) { ++s_->depth_; }
        ~CoreCall() {
            if (--s_->depth_ == 0) s_->held_.clear();
        }
        CoreCall(const CoreCall&) = delete;
        CoreCall& operator=(const CoreCall&) = delete;

    private:
        cFC4PlumbingSimulator* s_;
    };

private:
    uint64_t Occ(cISC4Occupant* occupant) const;
    void LogState(const char* what);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::HostGridContext gridCtx_{"plumbing", fc4::Sw::PlumbingGridWrites};
    fc4_plumbing_host host_{};
    fc4_plumbing* core_ = nullptr;
    cISC4City* city_ = nullptr;
    cISC4OccupantManager* occupantManager_ = nullptr;
    cISC4LotManager* lotManager_ = nullptr;
    cISC4PollutionSimulator* pollution_ = nullptr;
    cISC4OrdinanceSimulator* ordinance_ = nullptr;
    cISC4NeighborsSimulator* neighbors_ = nullptr;
    cISC4BudgetSimulator* budget_ = nullptr;
    cISTETerrain* terrain_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    std::vector<std::shared_ptr<void>> held_;
    int depth_ = 0;
    uint32_t lastState_ = 9;
    uint32_t passes_ = 0;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4PlumbingSimulator();
