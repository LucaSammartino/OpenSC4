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

// The growth developer's GZCOM class (docs/specs/growth-developer.md; CLSID
// 0x0990C138). The model is the Rust core behind fc4.h (fc4_growth); this
// class implements the three interfaces of one object (§1: cISC4GrowthDeveloper
// at +0, cIGZMessageTarget2 at +4, cIGZSerializable at +8) in the compiled
// slot order of §10.1 (FC4DevelopmentInterfaces.h), and the host callbacks
// over the game's services (§11).
//
// Replaced together with the building development simulator (§12,
// cFC4Development.h): the BDS creates its developers through GZCOM, so with
// `enable = development` every developer of a city is this class.
//
// Containers the game reads (§12) are kept in the game's layouts:
//   - the growth and decline lists (slots 11 / 12): B vectors of 12-byte
//     GrowthItems, refilled from the core after every outermost call in which
//     the update state changed (the lists change only in states 17 and 20);
//   - the style percentages (slot 26): an STLport map<uint32, float> built
//     with B (FC4GameMapBuild.h) after every Init.
//
// Threads: the process-wide development lock (fc4::DevelopmentLock) around
// every core call; another thread that cannot take it within 20 ms gets the
// values published after the last outermost main-thread call.
//
// Log lines (FreeCity4.log):
//   growth 0x<type> Init|Read|Shutdown: ... (the state)
//   growth 0x<type> Write -> 0|1
//   growth 0x<type> update YYYY-MM-DD: growth=G decline=D capacity=C
//     phantom=P size=S ... (one per finished monthly update, state 20 -> 0)
#pragma once

#include "FC4DevelopmentInterfaces.h"
#include "FC4GameList.h"
#include "FC4GameMap.h"
#include "FC4HostGrid.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#include <windows.h>

#include <atomic>
#include <cstdint>
#include <vector>

class cIGZMessageServer2;
class cISC4City;

namespace fc4 {

// What the growth host callbacks work on. Each developer owns one; the
// development shadow (FC4DevelopmentShadow.h) makes read-only ones over the
// city it reads.
struct GrowthContext {
    bool readOnly = false;                 // the shadow: borrowed services, read callbacks only, no switches
    cIGZMessageTarget2* target = nullptr;  // the agent and notification target (the developer only)
    // §1.2 step 2, held from acquire_service to release_service: 0 city,
    // 1 BDS, 2 occupant manager, 3 zone manager, 4 land value, 5 traffic.
    cIGZUnknown* services[6] = {};
    cIGZMessageServer2* msgServer = nullptr;
    cIGZUnknown* simulator = nullptr;  // cISC4Simulator, held while the agents are registered
    bool agentsRegistered = false;
    // References kept until the outermost call returns (the lots of
    // StateChanged messages a running call queued).
    std::vector<cIGZUnknown*> held;
    // The state-18 slicing deadline (performance counter).
    LONGLONG deadline = 0;
    // Observability.
    uint32_t sourceMaps = 0, missingSourceMaps = 0, populationUpdates = 0;
};

// The host table over `ctx`.
fc4_growth_host MakeGrowthHost(GrowthContext* ctx, const HostGridContext* grids);
// Releases what `ctx` still holds (agents, notifications, services, held references).
void ReleaseGrowthContext(GrowthContext* ctx);
// Releases the references kept for the outermost call.
void DropGrowthHeld(GrowthContext* ctx);

}  // namespace fc4

class cFC4GrowthDeveloper final : public cFC4ISC4GrowthDeveloper, public cIGZMessageTarget2, public cIGZSerializable {
public:
    cFC4GrowthDeveloper();
    // Class slot 31: the scalar deleting destructor (§10.1).
    virtual ~cFC4GrowthDeveloper();

    // cIGZUnknown (§1: one object, one reference count)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4GrowthDeveloper (§10.1)
    bool Init(cISCPropertyHolder* holder) override;
    bool Shutdown() override;
    int32_t GetType() override;
    uint32_t GetSubType() override;
    uint32_t GetPurpose() override;
    uint32_t GetWealth() override;
    void GetConstructionInfo(FC4ConstructionProfile& profile) override;
    bool GrowthAndDeclineListsAreValid() override;
    fc4::GameVector<FC4GrowthItem>& GetGrowthList() override;
    fc4::GameVector<FC4GrowthItem>& GetDeclineList() override;
    cISC4SimGrid<int16_t>* GetDesirabilityMap() override;
    cISC4SimGrid<uint16_t>* GetCapacityMap() override;
    cISC4SimGrid<int16_t>* GetPopulationMap() override;
    cISC4SimGrid<uint16_t>* GetMaxCapacityMap() override;
    int32_t GetTotalCapacity() override;
    cISC4Demand* GetSourceDemand() override;
    uint8_t GetPowerSupplyCap() override;
    uint8_t GetWaterSupplyCap() override;
    uint8_t GetFireCoverageCap() override;
    uint8_t GetParkCap() override;
    int32_t GetDesirabilityThreshold() override;
    float GetAbandonmentThreshold(int32_t densityType) override;
    int32_t GetMonopolyUse() override;
    fc4::GameMapObject* GetStylePercentages() override;
    void StartMapUpdate() override;
    bool IsMapUpdateFinished() override;
    int16_t EstimateDesirability(int32_t cellX, int32_t cellZ) override;
    void GetSubjectiveThresholds(int32_t index, int32_t& min, int32_t& max) override;

    // cIGZMessageTarget2 (§3.1)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§9)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    uint64_t SelfToken() const {
        return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(static_cast<const cFC4ISC4GrowthDeveloper*>(this)));
    }

private:
    class Call;  // the process lock and depth counting around core calls
    friend class Call;
    void Publish();
    void RefreshLists();
    void RebuildStyles();
    void LogState(const char* what);
    void LogUpdate(const fc4_growth_state& st);
    uint64_t GridToken(uint32_t which);
    uint8_t CapAt(uint32_t i);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    uint32_t depth_ = 0;
    fc4::GrowthContext ctx_;
    fc4::HostGridContext gridCtx_;
    fc4_growth_host host_{};
    fc4_growth* core_ = nullptr;
    // Slots 11 / 12 (B buffers) and slot 26 (B header and nodes).
    fc4::GameVector<FC4GrowthItem> growth_;
    fc4::GameVector<FC4GrowthItem> decline_;
    fc4::GameMapObject styles_{nullptr, 0, 0};
    bool listsDirty_ = true;
    bool stylesDirty_ = true;
    uint8_t lastState_ = 0;
    // Atomic copies for other threads (Publish).
    std::atomic<int32_t> type_{0}, totalCapacity_{0}, threshold_{0}, monopoly_{1};
    std::atomic<uint32_t> subType_{0}, purpose_{0}, wealth_{0}, caps_{0};
    std::atomic<uint64_t> grids_[4] = {};
    std::atomic<uint64_t> sourceDemand_{0};
    std::atomic<uint8_t> updateState_{0};
    std::atomic<uint32_t> offMainCalls_{0};
    // Status lines (main thread only).
    uint32_t updates_ = 0;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4GrowthDeveloper();
