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

// The flammability simulator's GZCOM class (docs/specs/flammability.md §1,
// CLSID 0xE990C045). The model lives in the Rust core behind fc4.h
// (fc4_flammability); this class implements the nine-slot interface (the SDK
// declaration matches docs/abi exactly) plus cIGZMessageTarget2 and
// cIGZSerializable, and the host callbacks over the game's services (§12).
//
// Both grids are the GAME's own cSC4SimGrid<uint8> (§13.2: the map view walks
// the adjusted grid's memory through GetGridData, so a replacement must hand
// out a real one created through GZCOM, class 0x49B9E602, tract size 1).
//
// The agent registration (§4.1 step 2) and the six notifications (step 3) are
// this class's job: the core asks for neither, exactly as in land value.
//
// The subsystem is OFF by default: FreeCity4.ini `enable = flammability` turns
// it on (FC4Subsystems.h); `disable` still wins. The flammability_* switches
// (FC4Config.h) turn groups of game calls off for bisecting.
//
// Containers (docs/abi): the occupant query fills SC4Lists (A, one reference
// each), kept until the outermost call into the core returns. [choice] §6.2
// asks for the occupant manager's FILTER overload (slot 40), but
// cISC4OccupantFilter is not in docs/abi/interfaces.tsv, so its vtable is not
// verified; this class calls the verified TYPED overload (slot 41) once per
// accepted type (building, flora, prop) and merges the results. An occupant has
// exactly one type, so the three sets are disjoint and the union is what the
// filter would have returned.
#pragma once

#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4FlammabilitySimulator.h"
#include "fc4.h"

#include <cstdint>
#include <memory>
#include <vector>

class cIGZMessageServer2;
class cISC4City;
class cISC4LotManager;
class cISC4OccupantManager;
class cISC4OrdinanceSimulator;
class cISC4PlumbingSimulator;
class cISC4WeatherSimulator;
class cISTETerrain;

static const uint32_t kFC4_CLSID_cSC4FlammabilitySimulator = 0xE990C045;
static const uint32_t kFC4_IID_cISC4FlammabilitySimulator = 0x48F41617;
/// `cIGZMessageTarget` (0xE98B2F57) is NOT answered (§1).
static const uint32_t kFC4_IID_cIGZMessageTarget = 0xE98B2F57;

class cFC4FlammabilitySimulator final : public cISC4FlammabilitySimulator,
                                        public cIGZMessageTarget2,
                                        public cIGZSerializable {
public:
    cFC4FlammabilitySimulator();
    ~cFC4FlammabilitySimulator();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4FlammabilitySimulator (§11.1; the SDK declaration matches the game)
    bool Init() override;
    bool Shutdown() override;
    cISC4SimGrid<uint8_t>* GetFlammabilityGrid() override;
    bool IsCellFlammable(int x, int z) override;
    bool GetFlammabilityHotspot(uint8_t* value, int* x, int* z) override;
    uint8_t GetAverageFlammability() override;

    // cIGZMessageTarget2 (§5)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§10)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // For the host callbacks (cFC4Flammability.cpp).
    cISC4City* City() const { return city_; }
    cISC4OccupantManager* OccupantManager() const { return occupantManager_; }
    cISC4LotManager* LotManager() const { return lotManager_; }
    cISC4OrdinanceSimulator* Ordinance() const { return ordinance_; }
    cISC4PlumbingSimulator* Plumbing() const { return plumbing_; }
    cISC4WeatherSimulator* Weather() const { return weather_; }
    cIGZMessageTarget2* Target() { return static_cast<cIGZMessageTarget2*>(this); }
    // The token the core knows this object by (its cISC4FlammabilitySimulator*,
    // the same identity the primary QueryInterface hands out).
    uint64_t SelfToken() const {
        return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(static_cast<const cISC4FlammabilitySimulator*>(this)));
    }
    void AcquireServices();
    void ReleaseServices();
    void Subscribe(const uint32_t* ids, size_t count, bool add);
    // Keeps a game list (and the references it holds) until the outermost call
    // into the core returns.
    void Hold(std::shared_ptr<void> list) { held_.push_back(std::move(list)); }

    // Counts the calls into the core; the outermost one drops the held lists.
    class CoreCall {
    public:
        explicit CoreCall(cFC4FlammabilitySimulator* s) : s_(s) { ++s_->depth_; }
        ~CoreCall() {
            if (--s_->depth_ == 0) s_->held_.clear();
        }
        CoreCall(const CoreCall&) = delete;
        CoreCall& operator=(const CoreCall&) = delete;

    private:
        cFC4FlammabilitySimulator* s_;
    };

private:
    uint64_t Occ(cISC4Occupant* occupant) const;
    void RegisterAgents(bool add);
    void LogState(const char* what);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::HostGridContext gridCtx_{"flammability", fc4::Sw::FlammabilityGridWrites};
    fc4_flammability_host host_{};
    fc4_flammability* core_ = nullptr;
    cISC4City* city_ = nullptr;
    cISC4OccupantManager* occupantManager_ = nullptr;
    cISC4LotManager* lotManager_ = nullptr;
    cISC4OrdinanceSimulator* ordinance_ = nullptr;
    cISC4PlumbingSimulator* plumbing_ = nullptr;
    cISC4WeatherSimulator* weather_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    std::vector<std::shared_ptr<void>> held_;
    int depth_ = 0;
    bool agentsRegistered_ = false;
    uint32_t cycles_ = 0;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4FlammabilitySimulator();
