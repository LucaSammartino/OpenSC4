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

// The lot developer's interfaces as the game compiled them, checked by
// tests/abi_conformance_test against docs/abi/interfaces.tsv and vtables.tsv:
//
//   cFC4ISC4LotDeveloper  cISC4LotDeveloper (vtable 0x00AB1B98), the 65
//       interface slots of lot-developer.md §11.1 with their compiled return
//       channels and callee pops. The SDK groups the overload pairs 16/17,
//       54/55, 56/57 and 58/59 in header order (the compiler reversed them),
//       declares slot 23 and 53 with the wrong argument bytes (pops 4 and 8),
//       slot 18 with a float rectangle (the code reads int32 cells), and bool
//       where the original returns counts (19-21), pointers (38-40, 42, 63)
//       or nothing defined (13, 14, 34, 35, 44). Every method has its own
//       name here (FC4Interfaces.h rule: no overloads, slot N is the N-th
//       virtual function).
//   cFC4ISC4NetworkManager  cISC4NetworkManager as compiled (vtable
//       0x00AA7AD0), slots 3-23: the lot developer calls slot 6
//       (DestroyNetworkOccupants), 20 (CreateConnectionNode) and 21
//       (AutoconnectLotNodes) (lot-developer.md §6.5, LD17). The SDK header
//       has an extra GetAllIntersectionPieces overload at slot 13 and so puts
//       every later method one slot late.
//   cFC4ISC4AnimationTickManager  cSC4AnimationTickManager's primary vtable
//       (service 0x86AD10EC, IID 0x86AD10ED; §6.7.8, vtables.tsv): the lot
//       developer registers its message target with slots 5 / 6.
//
// What a replacement returns where the original is undefined (§11.1): slots
// 13, 14, 34, 35 true; slot 44 the lot; slot 42 nonzero when a map entry was
// found; Release 0.
#pragma once

#include "SC4List.h"
#include "SC4Point.h"
#include "SC4Rect.h"
#include "cIGZUnknown.h"

#include <cstdint>

class cIGZMessageTarget2;
class cIGZPersistDBSegment;
class cISC4BuildingOccupant;
class cISC4BusinessUnit;
class cISC4FoundationOccupant;
class cISC4Lot;
class cISC4LotBaseTextureOccupant;
class cISC4LotConfiguration;
class cISC4LotDeveloper;
class cISC4NetworkLot;
class cISC4Occupant;
class cSC4LotConfigurationObject;

// §1.1 / §11.
static const uint32_t kFC4_CLSID_cSC4LotDeveloper = 0xA990BFE0;
static const uint32_t kFC4_IID_cISC4LotDeveloper = 0x4818E5D2;
static const uint32_t kFC4_LotDeveloperSimulatorType = 0x6818E5C9;  // slot 5
// §6.7.8: the animation-tick manager service and its interface.
static const uint32_t kFC4_SID_AnimationTickManager = 0x86AD10EC;
static const uint32_t kFC4_IID_AnimationTickManager = 0x86AD10ED;

// cISC4LotDeveloper as compiled (§11.1; the slot in the comment, then the
// callee pop where it is not the arguments' plain size).
class cFC4ISC4LotDeveloper : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                                           // 3
    virtual bool Shutdown() = 0;                                                                       // 4
    virtual uint32_t GetSimulatorType() = 0;                                                           // 5
    virtual bool StartConstruction(cISC4Lot* lot, cISC4LotConfiguration* cfg, uint32_t type, bool animate) = 0;  // 6
    virtual bool ConstructDefaultLot(cISC4Lot* lot, cISC4LotConfiguration* cfg, uint32_t type, bool animate) = 0;  // 7
    virtual bool AbandonBuilding(cISC4Lot* lot, int32_t reason) = 0;                                  // 8
    virtual bool ReoccupyBuilding(cISC4Lot* lot, int32_t devType) = 0;                                 // 9
    virtual bool ChangeBuilding(cISC4Lot* lot, uint32_t type) = 0;                                     // 10
    virtual bool RemoveBuildingZotProp(cISC4Lot* lot) = 0;                                             // 11
    virtual bool StartDemolishLot(cISC4Lot* lot) = 0;                                                  // 12
    virtual bool EndDemolishLot(cISC4Lot* lot) = 0;                                                    // 13 (undefined: true)
    virtual bool ExplodeLot(cISC4Lot* lot, bool burnt) = 0;                                            // 14 (undefined: true)
    virtual float GetTerrainAltitudeByVertex(int32_t x, int32_t z, bool withWater) = 0;                // 15 ST0
    virtual float GetAreaAltitudeF(SC4Rect<float> const& rect, bool lowest, bool withWater) = 0;       // 16 ST0
    virtual float GetAreaAltitudeI(SC4Rect<int32_t> const& rect, bool lowest, bool withWater) = 0;     // 17 ST0
    virtual float GetAverageAltitude(SC4Rect<int32_t> const& rect, bool withWater) = 0;                // 18 ST0
    virtual int32_t LevelInternalVertices(SC4Rect<int32_t> const& rect, float target, bool skipWater, bool force,
                                          cISC4Lot* lot) = 0;                                          // 19 EAX count
    virtual int32_t LevelBorderVertices(SC4Rect<int32_t> const& rect, float h, bool unused,
                                        cISC4Lot* lot) = 0;                                            // 20 EAX count
    virtual int32_t AttemptLevelling(SC4Rect<int32_t> const& rect, float unused, bool withWater, bool perVertex,
                                     float maxAreaSlope, cISC4Lot* lot) = 0;                           // 21 EAX count
    virtual bool GetOccupantCellCoverage(cISC4Occupant* occupant, SC4Rect<int32_t>& rect) = 0;         // 22
    virtual bool IsAreaDevelopedStub(SC4Rect<int32_t> const& rect) = 0;                                // 23 pop 4, AL 0
    virtual bool IsLotDeveloped(cISC4Lot* lot) = 0;                                                    // 24
    virtual bool IsAreaFlat(SC4Rect<int32_t> const& rect, bool withWater) = 0;                         // 25
    virtual bool IsLotFlat(cISC4Lot* lot, bool withWater) = 0;                                         // 26
    virtual float GetAreaAltitudeRange(SC4Rect<int32_t> const& rect, bool withWater) = 0;              // 27 ST0
    virtual float GetLotAltitudeRange(cISC4Lot* lot, bool withWater) = 0;                              // 28 ST0
    virtual float GetAreaSlope(SC4Rect<int32_t> const& rect, bool withWater) = 0;                      // 29 ST0
    virtual float GetLotSlope(cISC4Lot* lot, bool withWater) = 0;                                      // 30 ST0
    virtual float GetOccupantSlope(cISC4Occupant* occupant, bool withWater) = 0;                       // 31 ST0
    virtual int64_t GetLotDevelopmentCost(SC4Rect<int32_t> const& rect) = 0;                           // 32 EDX:EAX
    virtual bool RemoveLotFromItsBusinessUnit(cISC4Lot* lot) = 0;                                      // 33
    virtual bool UpdateTravelDesignates(cISC4BusinessUnit*& unit) = 0;                                 // 34 (undefined: true)
    virtual bool UpdateTravelPopulations(cISC4BusinessUnit* unit) = 0;                                 // 35 (undefined: true)
    virtual bool NeedsAnchorBuilding(SC4Rect<int32_t> const& rect, int32_t purpose) = 0;               // 36
    virtual bool UpdateLotElements(cISC4Lot* lot, uint32_t kind) = 0;                                  // 37
    virtual cISC4BuildingOccupant* GetFirstBuilding(cISC4Lot* lot) = 0;                                // 38 no reference
    virtual int32_t* GetObjectPosition(cISC4Lot* lot, cISC4LotConfiguration* cfg, cSC4LotConfigurationObject* obj,
                                       float* pos, int32_t& orient) = 0;                               // 39 EAX &orient
    virtual int32_t* GetBuildingPosition(cISC4Lot* lot, float* pos, int32_t& orient) = 0;             // 40 EAX &orient / 0
    virtual bool GetLotBaseTextureOccupant(cISC4Lot* lot, cISC4LotBaseTextureOccupant*& occ, bool create) = 0;  // 41
    virtual intptr_t GetBoundaryTexture(int32_t* sides, uint32_t& texture, uint8_t& rotation,
                                        bool& flip) = 0;                                               // 42 EAX nonzero: found
    virtual void SetEmptyLotBaseTexture(cISC4Lot* lot) = 0;                                            // 43
    virtual cISC4Lot* SetNormalLotBaseTexture(cISC4Lot* lot, cISC4LotConfiguration* cfg, bool flag) = 0;  // 44 (the lot)
    virtual bool UpdateLotBaseTextureOccupant(cISC4Lot* lot, bool hadFoundation) = 0;                 // 45
    virtual bool GetForceEmptyLotBaseTextures() = 0;                                                   // 46
    virtual void SetForceEmptyLotBaseTextures(bool force) = 0;                                         // 47
    virtual cISC4FoundationOccupant* GetFoundationByOccupant(cISC4Occupant* occupant) = 0;             // 48
    virtual cISC4FoundationOccupant* GetFoundationByCell(int32_t kind, int32_t x, int32_t z) = 0;      // 49
    virtual cISC4FoundationOccupant* GetFoundationByPosition(int32_t kind, float x, float z) = 0;      // 50
    virtual cISC4FoundationOccupant* GetLotFoundation(cISC4Lot* lot) = 0;                              // 51
    virtual bool RemoveFoundation(int32_t kind, float x, float z) = 0;                                 // 52
    virtual bool RemoveLotFoundationAt(int32_t x, int32_t z) = 0;                                      // 53 pop 8
    virtual bool AddLotFoundationCfg(cISC4Lot* lot, cISC4LotConfiguration* cfg, bool useLotAltitude, bool level,
                                     bool walls) = 0;                                                  // 54
    virtual bool AddLotFoundationType(cISC4Lot* lot, uint32_t type, bool useLotAltitude, bool level,
                                      bool walls) = 0;                                                 // 55
    virtual bool IsOccupantFoundationRequired(cISC4Occupant* occupant) = 0;                            // 56
    virtual bool IsBoxFoundationRequired(float* min, float* max) = 0;                                  // 57
    virtual bool AddBuildingFoundationCfg(cISC4BuildingOccupant* building, cISC4LotConfiguration* cfg) = 0;  // 58
    virtual bool AddBuildingFoundationType(cISC4BuildingOccupant* building, uint32_t type) = 0;        // 59
    virtual bool RemoveBuildingFoundation(cISC4Occupant* occupant) = 0;                                // 60
    virtual bool UpdateLotFoundation(cISC4Lot* lot) = 0;                                               // 61
    virtual bool UpdateLotRetainingWalls(cISC4Lot* lot) = 0;                                           // 62
    virtual cISC4Occupant* CreateLotProp(cISC4Lot* lot, uint32_t objectIndex, uint32_t propID) = 0;   // 63 EAX occ
    virtual bool GetLotConfigurationObjectOccupant(cISC4Lot* lot, uint32_t objectIndex,
                                                   cISC4Occupant*& out) = 0;                           // 64
};

// cSC4AnimationTickManager (vtables.tsv cSC4AnimationTickManager::vftable,
// 17 slots; §6.7.8). Only slots 5 and 6 are called. Their first argument is
// the tickable target (the lot developer's message target, this + 4), the
// second the priority (0xC0: every frame).
class cFC4ISC4AnimationTickManager : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                         // 3
    virtual bool Shutdown() = 0;                                                     // 4
    virtual bool AddTickableObject(cIGZMessageTarget2* target, uint32_t priority) = 0;     // 5
    virtual bool RemoveTickableObject(cIGZMessageTarget2* target, uint32_t priority) = 0;  // 6
    virtual bool FindTickableObject(cIGZMessageTarget2* target, uint32_t* priority) = 0;   // 7
    virtual bool SetNewPriority(cIGZMessageTarget2* target, uint32_t priority) = 0;        // 8
    virtual bool DoAnimationTick() = 0;                                              // 9
    virtual intptr_t GetSlot10() = 0;                                                // 10
    virtual bool SetMinAndMaxTickTimeDeltas(uint32_t minUs, uint32_t maxUs) = 0;     // 11
    virtual bool PauseAnimationSystem(bool pause) = 0;                               // 12
    virtual bool IsAnimationSystemPaused() = 0;                                      // 13
    virtual bool EnableAutoStep(bool enable, uint32_t stepUs) = 0;                   // 14
    virtual bool SetStepTickTimeDelta(uint32_t stepUs) = 0;                          // 15
    virtual bool StepAnimationSystem() = 0;                                          // 16
};

// cISC4NetworkManager as compiled (interfaces.tsv cISC4NetworkManager, 25
// slots; slot 24 is the class's deleting destructor). The slot in the comment,
// then the callee pop.
class cFC4ISC4NetworkManager : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                                   // 3
    virtual bool Shutdown() = 0;                                                               // 4
    virtual void* GetNetworkTool(int32_t type, bool unique) = 0;                               // 5
    virtual void DestroyNetworkOccupants(SC4List<cISC4Occupant*> const& list) = 0;             // 6, pop 4
    virtual void QueueRemovals(bool queue) = 0;                                                // 7
    virtual bool RemoveOrReplaceOccupant(void* tool, cISC4Occupant* occ, bool& replaced) = 0;  // 8
    virtual bool IsOccupantSubwayStation(cISC4Occupant* occ) = 0;                              // 9
    virtual intptr_t FindPipeOccupant(intptr_t a, int32_t b, intptr_t c) = 0;                  // 10
    virtual intptr_t FindPipeConnectionOccupant(intptr_t a, int32_t b, intptr_t c) = 0;        // 11
    virtual intptr_t GetAllIntersectionPieces(intptr_t a, intptr_t b, intptr_t c, intptr_t d) = 0;  // 12
    virtual uint32_t MapPreviewTexture(uint32_t a, uint8_t& b) = 0;                            // 13
    virtual uint32_t MapShadowTexture(uint32_t a, uint8_t& b, uint32_t c, uint32_t d) = 0;     // 14
    virtual uint32_t MapPath(uint32_t a) = 0;                                                  // 15
    virtual void RegisterConstructionCrew(intptr_t crew) = 0;                                  // 16
    virtual intptr_t UnregisterConstructionCrew(intptr_t crew) = 0;                            // 17
    virtual void* GetWaterOccupantFilter() = 0;                                                // 18
    virtual void* GetSubwayOccupantFilter() = 0;                                               // 19
    // 20, pop 28, AL; `out` gets the node's occupant with one reference for the caller
    virtual bool CreateConnectionNode(cISC4Occupant*& out, int32_t type, SC4Point<int> const& cell, uint32_t edgesA,
                                      uint32_t edgesB, uint32_t pathOverride, uint8_t c) = 0;
    virtual void AutoconnectLotNodes(cISC4Lot* lot, SC4List<cISC4Occupant*> const& nodes) = 0;  // 21, pop 8
    virtual const char* GetEnglishNetworkName(int32_t type) = 0;                               // 22
    virtual void AddToRetrofitList(intptr_t networkOccupant) = 0;                              // 23
};

// cISC4NetworkLotManager as compiled (vtable 0x00AA7958, interfaces.tsv
// cISC4NetworkLotManager, 16 slots; slot 15 is the class's deleting
// destructor). The SDK's declaration matches the compiled order here, so this
// is the SDK's list with every method named for its slot. Declared next to
// cFC4ISC4NetworkManager for the network managers' replacement classes
// (glue24: `enable = netlot`).
class cFC4ISC4NetworkLotManager : public cIGZUnknown {
public:
    typedef bool (*LotVisitor)(uint32_t x, uint32_t z, cISC4NetworkLot* lot, void* ctx);

    virtual bool Init() = 0;                                                    // 3
    virtual bool Shutdown() = 0;                                                // 4
    virtual cISC4NetworkLot* GetNetworkLot(int32_t x, int32_t z) = 0;           // 5, borrowed
    // 6, pop 4: the SDK's `intptr_t` is a set the caller owns (allocators.md).
    virtual void GetNetworkLotConfigurationSet(intptr_t set) = 0;
    // 7: true when a loaded configuration has `id`; `out` is never written
    // (network-manager.md 8.6).
    virtual bool GetNetworkLotConfigurationById(uint32_t id, void* out) = 0;    // 7
    virtual cISC4Occupant* CreateProp(cISC4NetworkLot* lot, uint32_t index, uint32_t propId) = 0;  // 8, borrowed
    virtual int32_t GetZoneTypeForNetwork(int32_t x, int32_t z) = 0;            // 9
    virtual int32_t GetWealthForNetwork(int32_t x, int32_t z) = 0;              // 10
    virtual bool IterateAllNetworkLots(LotVisitor cb, void* ctx) = 0;           // 11
    virtual void RedevelopLot(uint32_t x, uint32_t z, cISC4NetworkLot* lot) = 0;  // 12
    virtual void SetLotConfigurationForNextPiece(void* cfg) = 0;                // 13
    virtual bool WriteLotConfiguration(void* cfg, cIGZPersistDBSegment* seg) = 0;  // 14
};

namespace fc4 {
// The game's lot developer travels as the SDK's pointer (BDS slot 36, city
// slot 79); single inheritance, vtable at offset 0.
inline cFC4ISC4LotDeveloper* AsGame(cISC4LotDeveloper* p) { return reinterpret_cast<cFC4ISC4LotDeveloper*>(p); }
inline cISC4LotDeveloper* AsSdk(cFC4ISC4LotDeveloper* p) { return reinterpret_cast<cISC4LotDeveloper*>(p); }
}  // namespace fc4
