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

// The development interfaces as the game compiled them, checked by
// tests/abi_conformance_test against docs/abi/interfaces.tsv:
//
//   cFC4ISC4BuildingDevelopmentSimulator  cISC4BuildingDevelopmentSimulator
//       (IID 0x07813EAE), the 59 slots of building-development.md §7.1. The
//       SDK groups and reverses its overloads (foundations §16) and gets
//       several returns wrong: slots 6/7/47 return the occupant pointer,
//       9 is the foundation overload, 11/13/17-19/21/27 return counts, 42/43
//       return a uint8 in AL, 45 a float in ST0, 48/49 int32 in EAX.
//   cFC4ISC4GrowthDeveloper  cISC4GrowthDeveloper (IID 0xE7CDD662), the 31
//       slots of growth-developer.md §10.1: GrowthItem is {x, z, maxStage}
//       (12 bytes), slot 24 returns a float in ST0, 26 the style map
//       (STLport map<uint32, SC4Percentage>), 29 an int16 in AX.
//   cFC4ISC4TractDeveloper  cISC4TractDeveloper, tract-developer.md §7.1:
//       Grow and Decline return int32, UpdatePopulation returns nothing.
//
// The rule of FC4Interfaces.h holds: NO overloads, so slot N is the N-th
// virtual function. Containers crossing these interfaces are the game's
// (allocators.md): vectors, lists and hash sets with the STL node allocator
// B (FC4GameList.h), maps as STLport trees (FC4GameMap.h), the special
// building list as an SDK SC4List (allocator A).
#pragma once

#include "FC4GameList.h"
#include "FC4GameMap.h"
#include "SC4List.h"
#include "cIGZUnknown.h"

#include <cstddef>
#include <cstdint>

class cGZPersistResourceKey;
class cIGZString;
class cISC4BuildingOccupant;
class cISC4BuildingTypeFilter;
class cISC4Demand;
class cISC4FoundationOccupant;
class cISC4GrowthDeveloper;
class cISC4LotDeveloper;
class cISC4LotRetainingWallOccupant;
class cISC4Occupant;
class cISC4TractDeveloper;
class cISCPropertyHolder;
template <typename T>
class cISC4SimGrid;

// cISC4BuildingOccupant::BuildingProfile (docs/abi/layouts.md, 0x40 bytes,
// 8-aligned). The capacity maps are a B vector of STLport map<uint32,
// uint16> objects (0 or 1 of them).
struct FC4BuildingProfile {
    uint32_t purpose;                   // +0x00
    uint32_t wealth;                    // +0x04
    uint16_t capacity;                  // +0x08
    uint8_t duration;                   // +0x0A  property 0x499AFA38
    uint8_t pad0;                       // +0x0B
    fc4::GameMapObject* capacityBegin;  // +0x0C
    fc4::GameMapObject* capacityEnd;    // +0x10
    fc4::GameMapObject* capacityCap;    // +0x14
    uint8_t flag27812836;               // +0x18
    uint8_t pad1[3];                    // +0x19
    uint32_t categoryFlags;             // +0x1C
    float occupantSize[3];              // +0x20
    uint32_t pad2;                      // +0x2C  never written
    int64_t plopCost;                   // +0x30
    int64_t budgetItemCost;             // +0x38
};

// cISC4BuildingDevelopmentSimulator::ConstructionProfile (BDS §4.8, 0x24 bytes).
struct FC4ConstructionProfile {
    int32_t zone;          // +0x00 building zone type
    int32_t unknown2;      // +0x04
    uint32_t wealth;       // +0x08
    uint32_t purpose;      // +0x0C
    uint8_t landValueMin;  // +0x10
    uint8_t landValueMax;  // +0x11
    uint8_t pad[2];        // +0x12
    uint32_t buildingType; // +0x14
    int32_t orientation;   // +0x18
    int32_t x;             // +0x1C
    int32_t z;             // +0x20
};

// cISC4BuildingDevelopmentSimulator::GrowthProfile (16 bytes; the tract
// developer reads the stage's low byte).
struct FC4GrowthProfile {
    cISC4GrowthDeveloper* developer;
    int32_t x;
    int32_t z;
    uint8_t stage;
    uint8_t pad[3];
};

// cISC4GrowthDeveloper::GrowthItem (growth-developer.md §10.1 error 1).
struct FC4GrowthItem {
    int32_t x;
    int32_t z;
    int32_t maxStage;
};

#if defined(_M_IX86) || (defined(__SIZEOF_POINTER__) && __SIZEOF_POINTER__ == 4)
static_assert(sizeof(FC4BuildingProfile) == 0x40, "BuildingProfile: 0x40 bytes");
static_assert(offsetof(FC4BuildingProfile, capacityBegin) == 0x0C && offsetof(FC4BuildingProfile, flag27812836) == 0x18 &&
                  offsetof(FC4BuildingProfile, categoryFlags) == 0x1C &&
                  offsetof(FC4BuildingProfile, occupantSize) == 0x20 && offsetof(FC4BuildingProfile, plopCost) == 0x30 &&
                  offsetof(FC4BuildingProfile, budgetItemCost) == 0x38,
              "BuildingProfile layout (layouts.md)");
static_assert(sizeof(FC4GrowthProfile) == 16, "GrowthProfile: 16 bytes");
#endif
static_assert(sizeof(FC4ConstructionProfile) == 0x24, "ConstructionProfile: 0x24 bytes");
static_assert(offsetof(FC4ConstructionProfile, buildingType) == 0x14 && offsetof(FC4ConstructionProfile, x) == 0x1C,
              "ConstructionProfile layout (BDS §4.8)");
static_assert(sizeof(FC4GrowthItem) == 12, "GrowthItem: 12 bytes");

// cISC4BuildingDevelopmentSimulator, 59 slots (BDS §7.1; slot in the comment).
class cFC4ISC4BuildingDevelopmentSimulator : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                                     // 3
    virtual bool Shutdown() = 0;                                                                 // 4
    virtual bool LocationIsOKForNewBuilding(uint32_t type, float x, float z) = 0;                // 5
    virtual cISC4BuildingOccupant* AddNewBuildingOccupantAtCell(uint32_t type, int32_t cellX, int32_t cellZ,
                                                                int32_t orientation) = 0;        // 6
    virtual cISC4BuildingOccupant* AddNewBuildingOccupantAt(uint32_t type, float* position,
                                                            int32_t orientation) = 0;            // 7
    virtual bool AddNewBuildingOccupant(cISC4BuildingOccupant* occupant) = 0;                    // 8
    virtual int64_t GetCostOfFoundationDemolition(cISC4FoundationOccupant* occupant, uint32_t flags) = 0;  // 9
    virtual int64_t GetCostOfOccupantDemolition(cISC4BuildingOccupant* occupant, uint32_t flags) = 0;      // 10
    virtual uint32_t GetAllBuildingTypes(fc4::GameVector<uint32_t>& types) = 0;                  // 11
    virtual uint32_t GetAllBuildingTypesArray(uint32_t* types, uint32_t& count) = 0;             // 12
    virtual uint32_t GetAllBuildingFamilyTypes(fc4::GameVector<uint32_t>& families) = 0;         // 13
    virtual bool GetBuildingKeyFromType(uint32_t type, cGZPersistResourceKey& key) = 0;          // 14
    virtual bool IsBuildingTypeValid(uint32_t type) = 0;                                         // 15
    virtual bool IsBuildingFamilyTypeValid(uint32_t family) = 0;                                 // 16
    virtual uint32_t GetBuildingTypesByFilter(fc4::GameHashSetU32<>& types, cISC4BuildingTypeFilter* filter) = 0;  // 17
    virtual uint32_t GetBuildingTypesByFamily(fc4::GameVector<uint32_t>& types, uint32_t family) = 0;     // 18
    virtual uint32_t GetBuildingTypeSetByFamily(fc4::GameHashSetU32<>& types, uint32_t family) = 0;       // 19
    virtual bool IsBuildingInFamily(uint32_t type, uint32_t family) = 0;                         // 20
    virtual uint32_t GetAllFoundationTypes(fc4::GameVector<uint32_t>& types) = 0;                // 21
    virtual bool GetFoundationKeyFromType(uint32_t type, cGZPersistResourceKey& key) = 0;        // 22
    virtual bool AddNewFoundationOccupantAt(uint32_t type, int32_t a, float* position, float b, float c, int32_t d,
                                            float e) = 0;                                        // 23
    virtual bool AddNewFoundationOccupant(cISC4FoundationOccupant* occupant) = 0;                // 24
    virtual bool DemolishFoundationOccupant(cISC4FoundationOccupant* occupant, uint32_t flags) = 0;  // 25
    virtual bool LoadFoundationProperties(cISC4FoundationOccupant* occupant, uint32_t type) = 0;     // 26
    virtual uint32_t GetAllLotRetainingWallTypes(fc4::GameVector<uint32_t>& types) = 0;          // 27
    virtual bool GetLotRetainingWallKeyFromType(uint32_t type, cGZPersistResourceKey& key) = 0;  // 28
    virtual bool LoadLotRetainingWallProperties(cISC4LotRetainingWallOccupant* occupant, uint32_t type) = 0;  // 29
    virtual uint32_t GetDeveloperType(uint32_t purpose, uint32_t wealth) = 0;                    // 30
    virtual uint32_t GetWealth(uint32_t developerType) = 0;                                      // 31
    virtual cISC4GrowthDeveloper* GetDeveloper(uint32_t developerType) const = 0;                // 32
    virtual cISC4GrowthDeveloper* GetDeveloperOf(uint32_t purpose, uint32_t wealth) const = 0;   // 33
    virtual fc4::GameMapObject* GetDevelopers() = 0;                                             // 34
    virtual cISC4TractDeveloper* GetTractDeveloper() = 0;                                        // 35
    virtual cISC4LotDeveloper* GetLotDeveloper() = 0;                                            // 36
    virtual cIGZString* GetExemplarName(uint32_t type) = 0;                                      // 37
    virtual cIGZString* GetBuildingName(uint32_t type) = 0;                                      // 38
    virtual FC4BuildingProfile& GetBuildingProfile(uint32_t type) const = 0;                     // 39
    virtual FC4BuildingProfile* GetBuildingProfilePtr(uint32_t type) const = 0;                  // 40
    virtual cISC4SimGrid<uint8_t>* GetBuildingMap() = 0;                                         // 41
    virtual uint8_t GetBuildingMapKeyOf(uint32_t purpose, uint32_t wealth) = 0;                  // 42 (AL)
    virtual uint8_t GetBuildingMapKey(uint32_t type) = 0;                                        // 43 (AL)
    virtual SC4List<cISC4Occupant*>* GetSpecialBuildings() = 0;                                  // 44
    virtual float GetMaxCapacity(int32_t zone, uint32_t purpose, uint32_t wealth) = 0;           // 45 (ST0)
    virtual bool IsSuitableForConstruction(FC4ConstructionProfile const& profile, int32_t& reason) = 0;  // 46
    virtual cISC4BuildingOccupant* PlopBuilding(FC4ConstructionProfile const& profile) = 0;      // 47
    virtual int32_t GetFreeConstructionResources() = 0;                                          // 48
    virtual int32_t GetTotalConstructionResources() = 0;                                         // 49
    virtual bool ClaimConstructionResources(uint16_t amount) = 0;                                // 50
    virtual bool ReleaseConstructionResources(uint16_t amount) = 0;                              // 51
    virtual int32_t GetDayOfLastSuccessfulBuild(uint32_t developerType) = 0;                     // 52
    virtual float GetStagePopulationMultiplier() = 0;                                            // 53 (ST0)
    virtual bool CheatIgnorePlumbing() = 0;                                                      // 54
    virtual bool CheatIgnorePower() = 0;                                                         // 55
    virtual bool CheatIgnoreCaps() = 0;                                                          // 56
    virtual bool CheatDebugGrowth() = 0;                                                         // 57
    virtual bool CheatCoffeeBar() = 0;                                                           // 58
};

// cISC4GrowthDeveloper, 31 slots (growth-developer.md §10.1).
class cFC4ISC4GrowthDeveloper : public cIGZUnknown {
public:
    virtual bool Init(cISCPropertyHolder* holder) = 0;                                           // 3
    virtual bool Shutdown() = 0;                                                                 // 4
    virtual int32_t GetType() = 0;                                                               // 5
    virtual uint32_t GetSubType() = 0;                                                           // 6
    virtual uint32_t GetPurpose() = 0;                                                           // 7
    virtual uint32_t GetWealth() = 0;                                                            // 8
    virtual void GetConstructionInfo(FC4ConstructionProfile& profile) = 0;                       // 9
    virtual bool GrowthAndDeclineListsAreValid() = 0;                                            // 10
    virtual fc4::GameVector<FC4GrowthItem>& GetGrowthList() = 0;                                 // 11
    virtual fc4::GameVector<FC4GrowthItem>& GetDeclineList() = 0;                                // 12
    virtual cISC4SimGrid<int16_t>* GetDesirabilityMap() = 0;                                     // 13
    virtual cISC4SimGrid<uint16_t>* GetCapacityMap() = 0;                                        // 14
    virtual cISC4SimGrid<int16_t>* GetPopulationMap() = 0;                                       // 15
    virtual cISC4SimGrid<uint16_t>* GetMaxCapacityMap() = 0;                                     // 16
    virtual int32_t GetTotalCapacity() = 0;                                                      // 17
    virtual cISC4Demand* GetSourceDemand() = 0;                                                  // 18
    virtual uint8_t GetPowerSupplyCap() = 0;                                                     // 19 (AL)
    virtual uint8_t GetWaterSupplyCap() = 0;                                                     // 20 (AL)
    virtual uint8_t GetFireCoverageCap() = 0;                                                    // 21 (AL)
    virtual uint8_t GetParkCap() = 0;                                                            // 22 (AL)
    virtual int32_t GetDesirabilityThreshold() = 0;                                              // 23
    virtual float GetAbandonmentThreshold(int32_t densityType) = 0;                              // 24 (ST0)
    virtual int32_t GetMonopolyUse() = 0;                                                        // 25
    virtual fc4::GameMapObject* GetStylePercentages() = 0;                                       // 26
    virtual void StartMapUpdate() = 0;                                                           // 27
    virtual bool IsMapUpdateFinished() = 0;                                                      // 28
    virtual int16_t EstimateDesirability(int32_t cellX, int32_t cellZ) = 0;                      // 29 (AX)
    virtual void GetSubjectiveThresholds(int32_t index, int32_t& min, int32_t& max) = 0;         // 30
};

// cISC4TractDeveloper, 15 slots (tract-developer.md §7.1). Called only.
class cFC4ISC4TractDeveloper : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                                     // 3
    virtual bool Shutdown() = 0;                                                                 // 4
    virtual int32_t Grow(FC4GrowthProfile const& profile, fc4::GameList<int32_t>& outcomes, bool& flag) = 0;  // 5
    virtual int32_t Decline(FC4GrowthProfile const& profile) = 0;                                // 6
    virtual void UpdatePopulation(FC4GrowthProfile const& profile) = 0;                          // 7
    virtual int32_t GetCurrentStyle() const = 0;                                                 // 8
    virtual fc4::GameVector<uint32_t>& GetActiveStyles() = 0;                                    // 9
    virtual intptr_t SetActiveStyles(fc4::GameVector<uint32_t> const& styles) = 0;               // 10
    virtual bool IsUsingAllStylesAtOnce() const = 0;                                             // 11
    virtual bool UseAllStylesAtOnce(bool flag) = 0;                                              // 12 (AL)
    virtual int32_t GetYearsBetweenStyleChanges() const = 0;                                     // 13
    virtual int32_t SetYearsBetweenStyleChanges(int32_t years) = 0;                              // 14
};

namespace fc4 {
// The game's objects travel as SDK pointers (single inheritance, vtable at
// offset 0); call them only through these declarations.
inline cFC4ISC4GrowthDeveloper* AsGame(cISC4GrowthDeveloper* p) { return reinterpret_cast<cFC4ISC4GrowthDeveloper*>(p); }
inline cISC4GrowthDeveloper* AsSdk(cFC4ISC4GrowthDeveloper* p) { return reinterpret_cast<cISC4GrowthDeveloper*>(p); }
inline cFC4ISC4TractDeveloper* AsGame(cISC4TractDeveloper* p) { return reinterpret_cast<cFC4ISC4TractDeveloper*>(p); }
}  // namespace fc4
