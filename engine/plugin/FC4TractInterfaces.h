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

// The tract developer's interfaces as the game has them
// (docs/specs/tract-developer.md §7, §8), checked by
// tests/abi_conformance_test against docs/abi/interfaces.tsv:
//
//   cFC4ISC4TractDeveloper            cISC4TractDeveloper (IID 0xA818ED1B),
//                                     slots 3-14 with the COMPILED return
//                                     channels of §7.1 (the SDK says bool for
//                                     slots 5, 6, 7 and 10, and its containers
//                                     are the service allocator's SC4List /
//                                     SC4Vector; the game's are B, §9.2).
//   cFC4ISC4LotConfigurationManager   cISC4LotConfigurationManager (28 slots):
//                                     the SDK's two AddLotConfiguration
//                                     overloads under names of their own, slot
//                                     5 with one argument (pop 4, the SDK has
//                                     two), slot 9's set the game's STLport
//                                     hash_set (B). Containers the tract
//                                     developer does not use are void*.
// The configuration filter (§7.4) is the SDK's cISC4LotConfigurationFilter
// (its 4 slots match docs/abi).
#pragma once

#include "FC4GameList.h"

#include "cIGZUnknown.h"
#include "cISC4BuildingDevelopmentSimulator.h"

#include <atomic>
#include <cstdint>

class cGZPersistResourceKey;
class cISC4LotConfiguration;
class cISC4LotConfigurationFilter;
class cISC4LotConfigurationManager;
class cISC4TractDeveloper;
class cS3DRect2D;
class cSC4LotConfigurationObject;

static const uint32_t kFC4_CLSID_cSC4TractDeveloper = 0x2990C142;
static const uint32_t kFC4_IID_cISC4TractDeveloper = 0xA818ED1B;

namespace fc4 {

// Allocator B (docs/abi/allocators.md), or the test seam's (op 45) while one
// is installed: the outcome list's nodes, the active-style vector and the
// tract developer's own game containers. Install the test allocator before a
// container takes memory and remove it only after the last one is gone.
struct TractNodeAllocator {
    using AllocateFn = void* (*)(uint32_t size);
    using FreeFn = void (*)(void* p);
    static std::atomic<AllocateFn>& TestAllocate() {
        static std::atomic<AllocateFn> f{nullptr};
        return f;
    }
    static std::atomic<FreeFn>& TestFree() {
        static std::atomic<FreeFn> f{nullptr};
        return f;
    }
    static bool Available() { return TestAllocate().load() != nullptr || StlNodeAllocator::Available(); }
    static void* Allocate(uint32_t size) {
        if (AllocateFn a = TestAllocate().load()) return a(size);
        return StlNodeAllocator::Allocate(size);
    }
    static void Free(void* p) {
        if (p == nullptr) return;
        if (FreeFn f = TestFree().load()) return f(p);
        StlNodeAllocator::Free(p);
    }
    static const void* StringVtable() { return nullptr; }
};

// std::list<cISC4TractDeveloper::Outcome> (§9.2: B, 12-byte nodes): the
// tract developer appends int32 codes and never clears it.
using TractOutcomeList = GameList<int32_t, TractNodeAllocator>;
// std::vector<uint32_t> of slots 9 and 10 (B).
using TractStyleVector = GameVector<uint32_t, TractNodeAllocator>;

}  // namespace fc4

// cISC4TractDeveloper as compiled (§7.1, vtable 0x00AB3118). Slot 15, the
// scalar deleting destructor, is not part of the interface.
class cFC4ISC4TractDeveloper : public cIGZUnknown {
public:
    using GrowthProfile = cISC4BuildingDevelopmentSimulator::GrowthProfile;

    virtual bool Init() = 0;                                                                   // 3  AL
    virtual bool Shutdown() = 0;                                                               // 4  AL = 1
    // EAX = the capacity built (int32, may be negative; callers test > 0).
    virtual int32_t Grow(GrowthProfile const& profile, fc4::TractOutcomeList& outcomes, bool& outFlag) = 0;  // 5
    virtual int32_t Decline(GrowthProfile const& profile) = 0;                                 // 6  EAX = the sum
    virtual int32_t UpdatePopulation(GrowthProfile const& profile) = 0;                        // 7  0 (original: garbage)
    virtual int32_t GetCurrentStyle() = 0;                                                     // 8
    virtual fc4::TractStyleVector& GetActiveStyles() = 0;                                      // 9  the member vector
    // EAX = the member vector's address (the original's, from the assignment).
    virtual intptr_t SetActiveStyles(fc4::TractStyleVector const& styles) = 0;                 // 10
    virtual bool IsUsingAllStylesAtOnce() = 0;                                                 // 11
    virtual cFC4ISC4TractDeveloper* UseAllStylesAtOnce(bool all) = 0;                          // 12 `this`
    virtual int32_t GetYearsBetweenStyleChanges() = 0;                                         // 13
    virtual cFC4ISC4TractDeveloper* SetYearsBetweenStyleChanges(int32_t years) = 0;            // 14 `this`
};

// cISC4LotConfigurationManager (vtable 0x00A8EC58, 28 slots).
class cFC4ISC4LotConfigurationManager : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                                   // 3
    virtual bool Shutdown() = 0;                                                               // 4
    virtual bool AddLotConfiguration(cISC4LotConfiguration* config) = 0;                       // 5  (SDK: + bool)
    virtual bool AddLotConfigurationByKey(cGZPersistResourceKey const& key, void* data) = 0;   // 6
    virtual bool RemoveLotConfiguration(uint32_t id) = 0;                                      // 7
    virtual cISC4LotConfiguration* GetLotConfiguration(uint32_t id) = 0;                       // 8  (no reference)
    // Inserts each matching configuration with one reference (the caller
    // releases each); does not clear; EAX = the count (§8).
    virtual uint32_t GetLotConfigurationsByFilter(fc4::GameHashSetU32<fc4::TractNodeAllocator>& configs,
                                                  cISC4LotConfigurationFilter* filter) = 0;    // 9
    virtual bool GetLotConfigurationsByFamily(void* configs, uint32_t family) = 0;             // 10
    virtual bool GetLotConfigurationsBySize(void* configs, uint32_t x, uint32_t z) = 0;        // 11
    virtual bool ClearLotConfigurationSet(void* configs) = 0;                                  // 12
    virtual bool GetLotConfigurationIDsByFilter(void* ids, cISC4LotConfigurationFilter* filter) = 0;  // 13
    virtual bool GetLotConfigurationIDsByFamily(void* ids, uint32_t family) = 0;               // 14
    virtual bool GetLotConfigurationIDsBySize(void* ids, uint32_t x, uint32_t z) = 0;          // 15
    virtual bool LotConfigurationHasBuildingFamily(cISC4LotConfiguration* config, uint32_t family,
                                                   cSC4LotConfigurationObject** object) = 0;  // 16
    virtual bool LotConfigurationHasBuildingType(cISC4LotConfiguration* config, uint32_t type,
                                                 cSC4LotConfigurationObject** object) = 0;    // 17
    virtual bool RegisterLotConfigurationFilter(cISC4LotConfigurationFilter* filter) = 0;      // 18
    virtual bool UnregisterLotConfigurationFilter(cISC4LotConfigurationFilter* filter) = 0;    // 19
    virtual intptr_t GetLotConfigurationMap() = 0;                                             // 20
    virtual intptr_t GetLotConfigurationFamilyIDSet() = 0;                                     // 21
    virtual bool CreateGenericLotConfiguration(cISC4LotConfiguration*& config, uint32_t id) = 0;  // 22
    virtual bool FixLotConfiguration(cISC4LotConfiguration* config) = 0;                       // 23
    virtual bool GetMaxFootprintForBuildingFamilies(void const* families, cS3DRect2D& rect) = 0;  // 24
    virtual bool GetMaxFootprintForFloraFamilies(void const* families, cS3DRect2D& rect) = 0;     // 25
    virtual bool GetMaxFootprintForPropFamilies(void const* families, cS3DRect2D& rect) = 0;      // 26
    virtual bool GetObjectIDArray(cSC4LotConfigurationObject* object, void* ids) = 0;          // 27
};

namespace fc4 {
// The manager travels as the SDK's pointer (city slot 37); single
// inheritance, vtable at offset 0. Call it only through this declaration.
inline cFC4ISC4LotConfigurationManager* AsGame(cISC4LotConfigurationManager* p) {
    return reinterpret_cast<cFC4ISC4LotConfigurationManager*>(p);
}
// The city's tract developer (city slot 80) seen through the compiled slots.
inline cFC4ISC4TractDeveloper* AsGame(cISC4TractDeveloper* p) { return reinterpret_cast<cFC4ISC4TractDeveloper*>(p); }
}  // namespace fc4
