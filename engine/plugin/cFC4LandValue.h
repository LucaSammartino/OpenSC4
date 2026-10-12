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

// The land value simulator's GZCOM class (docs/specs/landvalue.md §1). The
// pipeline lives in the Rust core behind fc4.h; this class adapts the SDK
// interfaces to it and owns the game services it uses.
//
// Grid ownership (simgrid.md §6.3): other subsystems read the land value grids'
// memory directly, so the grids are the GAME's own cSC4SimGrid objects, created
// through GZCOM (CLSIDs 0x49B9E602 / 0x49B9E60A, registered by the game). We
// never register those class IDs.
//
// The subsystem is OFF by default: FreeCity4.ini `enable = landvalue` turns it
// on (FC4Subsystems.h); `disable` still wins.
#pragma once

#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4LandValueSimulator.h"
#include "fc4.h"

#include <cstdint>

class cIGZMessageServer2;
class cISC4City;
class cISC4ZoneManager;
class cISTETerrain;

static const uint32_t kFC4_CLSID_cSC4LandValueSimulator = 0xE990C18E;
static const uint32_t kFC4_IID_cISC4LandValueSimulator = 0xE7781D8C;
static const uint32_t kFC4_IID_cISC4SimGrid = 0xC9BD939A;
static const uint32_t kFC4_CLSID_SimGridUint8 = 0x49B9E602;
static const uint32_t kFC4_CLSID_SimGridFloat = 0x49B9E60A;

class cFC4LandValueSimulator final : public cISC4LandValueSimulator,
                                     public cIGZMessageTarget2,
                                     public cIGZSerializable {
public:
    cFC4LandValueSimulator();
    ~cFC4LandValueSimulator();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4LandValueSimulator (§11.1; the SDK declaration matches the game)
    bool Init() override;
    bool Shutdown() override;
    LandValueType GetLandValueType(int32_t x, int32_t z) override;
    uint8_t GetLandValue(int32_t x, int32_t z) override;
    cISC4SimGrid<uint8_t>* GetIntrinsicLandValueMap() override;
    cISC4SimGrid<uint8_t>* GetTotalLandValueMap() override;
    cISC4SimGrid<float>* GetSlopeMap() override;
    uint8_t GetAverageLandValue() override;
    float GetModeAltitude() override;

    // cIGZMessageTarget2 (§5)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§10)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // For the host callbacks (cFC4LandValue.cpp).
    cISC4City* City() const { return city_; }
    cISTETerrain* Terrain() const { return terrain_; }
    cISC4ZoneManager* ZoneManager() const { return zoneManager_; }

private:
    void RegisterAgents(bool add);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4_landvalue_host host_{};
    fc4_landvalue* core_ = nullptr;
    bool initialized_ = false;
    cISC4City* city_ = nullptr;
    cISTETerrain* terrain_ = nullptr;
    cISC4ZoneManager* zoneManager_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    bool agentsRegistered_ = false;
    uint32_t lastState_ = 9;
    uint32_t passes_ = 0;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4LandValueSimulator();
