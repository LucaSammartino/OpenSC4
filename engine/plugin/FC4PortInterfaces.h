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

// The airport and seaport developers' interfaces in the compiled slot order
// (docs/specs/port-developers.md §8; docs/abi/interfaces.tsv rows 00ab0714,
// 00ab2e10). The SDK's declarations differ in three places:
//   - slot 5 GetAvgEfficiency returns SC4Percentage by value: under MSVC a
//     hidden result pointer (pop 4, EAX = out), declared here as that pointer;
//   - slot 8 GetAllPortLots appends to the game's std::vector (allocator B,
//     docs/abi/allocators.md): fc4::GameVector here;
//   - seaport slot 11 GetNumSeaports takes NO argument (`ret` without pop);
//     the SDK declares one (§4.7, §10 item 7).
// The port sub-object at +4 (cISC4PortDeveloper, IID 0x4A2C4E69) is a
// separate declaration with the same slots 0-9, so that one class can carry
// both (its slots 10-18 are internal: §8.3, no caller outside the object).
#pragma once

#include "FC4GameList.h"
#include "SC4Percentage.h"
#include "SC4Point.h"
#include "cIGZUnknown.h"

#include <cstdint>

class cISC4Lot;

// IIDs and class IDs (§1).
static const uint32_t kFC4_IID_cISC4PortDeveloper = 0x4A2C4E69;
static const uint32_t kFC4_IID_cISC4AirportDeveloper = 0xEA20548D;
static const uint32_t kFC4_IID_cISC4SeaportDeveloper = 0xCA22FE3E;
static const uint32_t kFC4_CLSID_cSC4AirportDeveloper = 0xAA2049A1;
static const uint32_t kFC4_CLSID_cSC4SeaportDeveloper = 0xEA230A28;

// cISC4PortDeveloper slots 0-9 (§8.1 rows 0-9; the +0 interfaces' first ten).
class cFC4ISC4PortDeveloper : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                                // 3
    virtual bool Shutdown() = 0;                                                            // 4
    virtual SC4Percentage* GetAvgEfficiency(SC4Percentage* out) = 0;                        // 5 (hidden result)
    virtual uint32_t GetSimulatorType() = 0;                                                // 6
    virtual bool GetLeastEfficientPort(SC4Percentage& pct, SC4Point<int32_t>& point) = 0;  // 7
    virtual void GetAllPortLots(fc4::GameVector<cISC4Lot*>& lots) = 0;                      // 8
    virtual uint32_t GetNumPorts(int32_t sizeFlags, int32_t stageFlags) = 0;                // 9
};

// The port sub-object at +4: the same slots 0-9 under another name (the
// class below derives from both without an ambiguous base).
class cFC4ISC4PortSubObject : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                                // 3
    virtual bool Shutdown() = 0;                                                            // 4
    virtual SC4Percentage* GetAvgEfficiency(SC4Percentage* out) = 0;                        // 5
    virtual uint32_t GetSimulatorType() = 0;                                                // 6
    virtual bool GetLeastEfficientPort(SC4Percentage& pct, SC4Point<int32_t>& point) = 0;  // 7
    virtual void GetAllPortLots(fc4::GameVector<cISC4Lot*>& lots) = 0;                      // 8
    virtual uint32_t GetNumPorts(int32_t sizeFlags, int32_t stageFlags) = 0;                // 9
};

// cISC4AirportDeveloper (vtable 0x00AB0714, 15 slots).
class cFC4ISC4AirportDeveloper : public cFC4ISC4PortDeveloper {
public:
    virtual cFC4ISC4PortSubObject* AsISC4PortDeveloper() = 0;                               // 10 (+4)
    virtual uint32_t GetNumAirports(int32_t size) = 0;                                      // 11
    virtual bool GetAirportUsageValues(cISC4Lot* lot, uint32_t& usage, uint32_t& capacity) = 0;  // 12
    virtual uint32_t GetNumAirportTrips() = 0;                                              // 13
    virtual int64_t GetMaintenanceCost(cISC4Lot* lot) = 0;                                  // 14 (EDX:EAX)
};

// cISC4SeaportDeveloper (vtable 0x00AB2E10, 17 slots).
class cFC4ISC4SeaportDeveloper : public cFC4ISC4PortDeveloper {
public:
    virtual cFC4ISC4PortSubObject* AsISC4PortDeveloper() = 0;                               // 10 (+4)
    virtual uint32_t GetNumSeaports() = 0;                                                  // 11 (no argument)
    virtual bool GetSeaportUsageValues(cISC4Lot* lot, uint32_t& usage, uint32_t& capacity) = 0;  // 12
    virtual uint32_t GetNumSeaportTrips() = 0;                                              // 13
    virtual int64_t GetMaintenanceCost(cISC4Lot* lot) = 0;                                  // 14 (EDX:EAX)
    virtual bool CityHasSea() = 0;                                                          // 15
    virtual uint8_t GetCellSeaAccess(int32_t x, int32_t z) = 0;                             // 16 (AL)
};
