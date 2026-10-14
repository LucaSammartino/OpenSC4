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

// The aura (mayor rating) simulator's GZCOM class (docs/specs/aura.md §1,
// CLSID 0x0990BDD8). The pipeline lives in the Rust core behind fc4.h
// (fc4_aura); this class implements cISC4AuraSimulator (§12.1: the SDK
// declaration is correct for MSVC, whose overload reversal puts the
// (x, z, type) AddTransientEffect at slot 6 and the 4-float one at slot 7),
// cIGZMessageTarget2 (§12.3) and cIGZSerializable (§12.2), and the host
// callbacks (fc4_aura_host) over the game's services (§13).
//
// The aura's grids are the GAME's own cSC4SimGrid<int8> / <int16> (§14.2: the
// map view reads the aura grid and the park map as raw memory), created
// through GZCOM (FC4HostGrid.h).
//
// The subsystem is OFF by default: FreeCity4.ini `enable = aura` turns it on
// (FC4Subsystems.h); `disable` still wins. The aura_* switches (FC4Config.h)
// turn groups of game calls off for bisecting.
//
// Threads: the game calls the getters (slots 5, 8-10, 12-15) from a second
// thread while the main thread is inside a day or an idle call. They read
// only the snapshot the core publishes behind a lock after every main-thread
// call (fc4_aura_ratings / fc4_aura_grid), never the core's state; slot 5
// then reads the game's aura grid as the original does.
//
// Where the glue departs from the original's mechanism (the spec's choices):
//   - the lot manager's safe lot iterator (§6.5, AU5): [choice] the lots are
//     found by GetLot(x, z, false) over every cell, each lot once, when state
//     4 prepares the walk (the stamp is a maximum: the order does not matter);
//   - GetPopulations (§6.5.1, AU3): lot slot 54's vector of
//     map<uint32_t, uint16_t> is read in place (FC4GameMap.h), every entry,
//     counts of 0 included;
//   - the game-data registry node "aura" (§4.3, AU6): not published (as
//     budget; no shipped file calls its commands);
//   - the process-wide base-aura table (§10, AU11): not written (only the
//     aura reads it; the core keeps the values in its object).
#pragma once

#include "FC4Config.h"
#include "FC4HostGrid.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4AuraSimulator.h"
#include "fc4.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdint>
#include <vector>

class cIGZMessageServer2;
class cISC4City;

static const uint32_t kFC4_CLSID_cSC4AuraSimulator = 0x0990BDD8;
static const uint32_t kFC4_IID_cISC4AuraSimulator = 0xC773E880;

// What the aura host callbacks work on. The simulator fills it with the city
// it holds (AddRef'd at Init); the aura shadow (FC4AuraShadow.h) with the
// city's borrowed pointer for one comparison.
struct FC4AuraGame {
    cISC4City* city = nullptr;
    // Gates the reads with the aura_* switches (ours); the shadow reads the
    // original's inputs as they are.
    bool gated = true;
    const char* tag = "aura";
    // The residential population grids (§6.15): one GetPopulationGrids per
    // outermost call, each grid AddRef'd for us, released by DropHeld.
    cISC4SimGrid<uint16_t>* pop[3] = {nullptr, nullptr, nullptr};
    bool popFetched = false;
    // References kept until the outermost call into the core returns.
    std::vector<cIGZUnknown*> held;
    void DropHeld();
    // The CPU-cycle limit timer (§6.5): a QueryPerformanceCounter deadline.
    LONGLONG deadline = 0;
    // Inputs found missing (bit = FC4_AURA_INPUT_*): logged once each, and
    // listed on the cycle line.
    uint32_t missingLogged = 0;
    uint32_t cycleMissing = 0;
    bool loggedPopulations = false;
    bool loggedRegistry = false;
    bool loggedBaseAura = false;
    // Lots stamped and lot walks since the last cycle line.
    uint32_t lotsWalked = 0;
    uint32_t lotWalks = 0;
};

namespace fc4 {
// A host table with the read callbacks only (city, simulator date, tuning,
// lots, land value, occupants, funding, tax rates, ordinances, input grids)
// over `game`, and a grid table with geometry and reads only (`grids` must
// outlive the table). Every write callback is NULL. For the aura shadow.
fc4_aura_host AuraReadOnlyHost(FC4AuraGame* game, const HostGridContext* grids);
}  // namespace fc4

class cFC4AuraSimulator final : public cISC4AuraSimulator,
                                public cIGZMessageTarget2,
                                public cIGZSerializable,
                                public FC4AuraGame {
public:
    cFC4AuraSimulator();
    ~cFC4AuraSimulator();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4AuraSimulator (§12.1; slots 6 / 7 by MSVC's overload reversal)
    bool Init() override;
    bool Shutdown() override;
    int8_t GetAuraValue(int32_t x, int32_t z) override;
    void AddTransientEffect(float x, float z, float radius, float magnitude) override;
    void AddTransientEffect(float x, float z, uint32_t type) override;
    int8_t GetMayorRating() const override;
    int8_t GetMaxMayorRating(SC4Point<long>& location) const override;
    int8_t GetMinMayorRating(SC4Point<long>& location) const override;
    void BustStrike(eStrikeBuster type) override;
    cISC4SimGrid<int8_t>* GetAuraGrid() override;
    cISC4SimGrid<int8_t>* GetTransientAuraGrid() override;
    cISC4SimGrid<int16_t>* GetParkMap() const override;
    cISC4SimGrid<int16_t>* GetLandmarkMap() const override;

    // cIGZMessageTarget2 (§5)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§11)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // For the host callbacks (cFC4Aura.cpp).
    cIGZMessageTarget2* Target() { return static_cast<cIGZMessageTarget2*>(this); }
    // The token the core knows this object by: the primary interface (§11.1's
    // void1 / IGZUnknown of the cycle-completed message).
    uint64_t SelfToken() const {
        return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(static_cast<const cISC4AuraSimulator*>(this)));
    }
    void Subscribe(const uint32_t* ids, size_t count, bool add);
    void NotePost() { ++posts_; }

    // Counts the calls into the core; the outermost one drops the held
    // references and logs a completed cycle.
    class CoreCall {
    public:
        explicit CoreCall(cFC4AuraSimulator* s) : s_(s) { ++s_->depth_; }
        ~CoreCall();
        CoreCall(const CoreCall&) = delete;
        CoreCall& operator=(const CoreCall&) = delete;

    private:
        cFC4AuraSimulator* s_;
    };

private:
    void AcquireServices();
    void ReleaseServices();
    void LogState(const char* what);
    void LogCycle();

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::HostGridContext gridCtx_{"aura", fc4::Sw::AuraGridWrites};
    fc4_aura_host host_{};
    fc4_aura* core_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    int depth_ = 0;
    uint32_t days_ = 0;
    uint32_t months_ = 0;
    uint32_t posts_ = 0;        // cycle-completed messages (§11.1), as the core asked
    uint32_t cyclesLogged_ = 0;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4AuraSimulator();
