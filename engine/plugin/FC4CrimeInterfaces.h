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

// The crime simulator's interfaces as the game has them (docs/specs/crime.md
// "Compiled slot tables", §11, §13), checked by tests/abi_conformance_test
// against docs/abi/interfaces.tsv:
//
//   cFC4ISC4CrimeSimulator   cISC4CrimeSimulator (IID 0x67659555), slots 3-13.
//                            The SDK order and argument sizes are right; only
//                            GetCrimeIncidentArray's container differs: the
//                            caller's vector is resized with the GAME's
//                            allocator (§14.2), so it is declared as
//                            fc4::GameVector (FC4GameList.h) instead of the
//                            SDK's SC4Vector (allocator A).
//   cFC4CrimePropRequester   cISC4PropRequester (IID 0x89C498BE, no SDK
//                            header), slots 3-6 (§11.6).
// The police simulator is called through cFC4ISC4PoliceSimulator
// (FC4Interfaces.h), whose slots 5 and 27 have the types §13 lists (crime
// calls slots 5, 27, 30 and 38 only).
#pragma once

#include "FC4GameList.h"

#include "SC4Rect.h"
#include "cIGZUnknown.h"
#include "cISC4SimGrid.h"

#include <cstdint>

class cISC4Automaton;
class cISC4PropOccupant;
class cS3DVector3;

static const uint32_t kFC4_IID_cISC4CrimeSimulator = 0x67659555;
// The prop requester IID under a name of its own: cFC4Pollution.h defines
// kFC4_IID_cISC4PropRequester (the same value) and both headers meet in
// FC4Subsystems.cpp.
static const uint32_t kFC4_IID_cISC4PropRequesterCrime = 0x89C498BE;

// tCrimeEvent (§11.4): 28 bytes. The SDK's `unknown1` is the visuals flag.
struct FC4CrimeEvent {
    uint32_t cellX;
    uint32_t cellZ;
    uint32_t typeID;
    uint8_t visualsCreated;
    uint8_t pad[3];
    uint32_t iconType;
    uint32_t iconGroup;
    uint32_t iconInstance;
};
static_assert(sizeof(FC4CrimeEvent) == 28, "tCrimeEvent is 28 bytes (crime.md §11.4)");

// A read-only view of the caller's std::vector<uint32_t> (STLport layout:
// first, last, end of storage; docs/abi/allocators.md). SelectProp only reads it.
struct FC4U32VectorView {
    const uint32_t* first;
    const uint32_t* last;
    const uint32_t* capacityEnd;
};
static_assert(sizeof(void*) != 4 || sizeof(FC4U32VectorView) == 12, "vector layout");

// cISC4CrimeSimulator, 14 interface slots (vtable 0x00AB0DC8). Slot 14 is the
// class's scalar deleting destructor and slot 15 GetSimulatorType: both
// class-private (cFC4CrimeSimulator declares them).
class cFC4ISC4CrimeSimulator : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                             // 3
    virtual bool Shutdown() = 0;                                                         // 4
    virtual cISC4SimGrid<uint8_t>* GetCrimeGrid() = 0;                                   // 5  borrowed
    virtual uint32_t GetAverageCrime() = 0;                                              // 6
    virtual uint32_t GetMonthlyCrimeCount() = 0;                                         // 7
    virtual bool GetCrimeHotspot(uint8_t* value, int32_t* x, int32_t* z) = 0;            // 8
    virtual cISC4Automaton* GetNearestCrimeSim(cS3DVector3 const& pos, float radius) = 0;      // 9  borrowed
    virtual cISC4PropOccupant* GetNearestCrimeProp(cS3DVector3 const& pos, float radius) = 0;  // 10 borrowed
    virtual bool GetCrimeIncidentArray(fc4::GameVector<FC4CrimeEvent>& out) = 0;         // 11
    virtual bool SetVisualsCreated(uint32_t index, bool value) = 0;                      // 12
    virtual bool GetVisualsCreated(uint32_t index) = 0;                                  // 13
};

// cISC4PropRequester (vtable 0x00AB0D9C): §11.6, "Compiled slot tables".
class cFC4CrimePropRequester : public cIGZUnknown {
public:
    virtual int32_t GetRequestedShare(SC4Rect<int32_t> const& rect) = 0;                  // 3
    virtual uint32_t GetRequesterSatisfactionId() = 0;                                   // 4
    virtual bool UsesCustomPropSelector() = 0;                                           // 5
    virtual uint32_t SelectProp(FC4U32VectorView const& props, int32_t cellX, int32_t cellZ) = 0;  // 6
};
