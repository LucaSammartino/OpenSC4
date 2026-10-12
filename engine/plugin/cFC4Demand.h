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

// The demand simulator and the demand node (docs/specs/demand.md §1). The
// graph, census, modifiers, region export and persistence live in the Rust
// core behind fc4.h; these classes adapt the game interfaces to it.
//
// Replaceability (§1): the game's building development simulator makes its
// own demand nodes; the simulator only keeps its own nodes in its map and
// reads foreign ones through cISC4Demand.
//
// The subsystem is OFF by default: FreeCity4.ini `enable = demand` turns it on
// (FC4Subsystems.h); `disable` still wins.
#pragma once

#include "FC4Interfaces.h"
#include "FC4Streams.h"
#include "SC4Percentage.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#include <cstdint>
#include <map>

class cIGZMessageServer2;
class cISC4City;

static const uint32_t kFC4_CLSID_cSC4DemandSimulator = 0x2990BFF2;
static const uint32_t kFC4_IID_cISC4DemandSimulator = 0x47967E16;
static const uint32_t kFC4_CLSID_cSC4Demand = 0x8990C12D;
static const uint32_t kFC4_IID_cISC4Demand = 0x0797A4E8;

class cFC4DemandSimulator final : public cFC4ISC4DemandSimulator,
                                  public cIGZMessageTarget2,
                                  public cIGZSerializable {
public:
    cFC4DemandSimulator();
    ~cFC4DemandSimulator();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4DemandSimulator (§5)
    bool Init() override;
    bool Shutdown() override;
    uint32_t GetSimulatorType() override;
    cISC4Demand* GetDemand(uint32_t demandID, uint32_t demandIndex) override;
    void UpdateOccupantEffects(uint32_t buildingType, SC4Percentage const& oldPct,
                               SC4Percentage const& newPct) override;
    float CalculateJobsPerUnitOfDemand(float* jobs, uint32_t demandID, uint32_t demandIndex) override;
    uint32_t GetJobsBySensus(uint32_t type) override;
    float GetNeutralTaxRate() override;
    void GetLocalPopulationSummary(std::map<uint32_t, int32_t>& summary) const override;

    // cIGZMessageTarget2 (§10.2)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§12.1)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // For the host callbacks (cFC4Demand.cpp): the city held since Init.
    cISC4City* City() const { return city_; }
    cIGZUnknown* AsUnknown() { return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4DemandSimulator*>(this)); }

private:
    bool Initialized() const;
    void RegisterAgents(bool add);
    void Subscribe(bool add);
    void LogState(const char* what);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4_demand_host host_{};
    fc4_demand_sim* core_ = nullptr;
    cISC4City* city_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    bool agentsRegistered_ = false;
    mutable bool summaryNoted_ = false;
};

// A demand node (§3) of our own.
class cFC4Demand final : public cFC4ISC4Demand, public cIGZSerializable, public cIFC4CoreObject {
public:
    cFC4Demand();
    ~cFC4Demand();

    // The token the core uses (the cISC4Demand*) and the core object.
    uint64_t Token() { return fc4::TokenOf(static_cast<cFC4ISC4Demand*>(this)); }
    fc4_demand_node* Core() const { return core_; }

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4Demand (§3.2)
    bool Init() override;
    bool Shutdown() override;
    bool SimulationBegin() override;
    uint32_t GetId() const override;
    bool SetId(uint32_t id) override;
    float QuerySupplyValue() const override;
    float QueryDemandValue() const override;
    float QueryNewSupply() const override;
    float QueryNewDemand() const override;
    float QueryActiveDemandValue() const override;
    float QueryEconomyModifier() const override;
    float QueryActiveDemandMax() const override;
    float QueryActiveDemandMin() const override;
    float AddToSupplyValue(float value) override;
    float AddToDemandValue(float value) override;
    bool SetSupplyValue(float value) override;
    bool SetDemandValue(float value) override;
    bool SetActiveDemandMax(float value) override;
    bool SetActiveDemandMin(float value) override;
    float GetEconomyModifier() const override;
    bool SetEconomyModifier(float value) override;
    float GetTaxModifier() const override;
    bool SetTaxModifier(float value) override;
    SC4Percentage* GetDemandCap() const override;
    bool SetDemandCap(SC4Percentage const& demandCap) override;
    uint32_t GetRegionUse() override;
    bool SetRegionUse(uint32_t regionUse) override;
    bool EndOfCycle() override;
    void DebugLockValue(float value) override;
    void DebugUnlockValue() override;
    bool DebugIsValueLocked() override;

    // cIGZSerializable (§12.2)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // cIFC4CoreObject
    void* CoreHandle() override { return core_; }
    uint32_t CoreClass() override { return kFC4_CLSID_cSC4Demand; }

private:
    float Get(uint32_t which) const { return fc4_demand_node_get(core_, which); }

    uint32_t refCount_ = 0;
    fc4_demand_node* core_ = nullptr;
    // GetDemandCap returns a pointer (§3.2): a copy of the core's value,
    // refreshed by every GetDemandCap call.
    mutable SC4Percentage cap_{};
};

// Factories for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4DemandSimulator();
cIGZUnknown* CreateFC4Demand();
