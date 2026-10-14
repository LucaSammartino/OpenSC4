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

// The landfill developer's GZCOM class (docs/specs/landfill.md §1; CLSID
// 0xEA09D4AE). The model lives in the Rust core behind fc4.h (fc4_landfill);
// this class implements the three interfaces of §1 in the compiled slot order
// of §7 (FC4Interfaces.h cFC4ISC4LandfillDeveloper: the SDK gets slots 5 and
// 9 wrong) and the host callbacks over the game's services (§8).
//
// One object, three interfaces, one reference count: the primary interface
// (cISC4LandfillDeveloper 0x8A09D502, also cIGZUnknown), cIGZMessageTarget2
// and cIGZSerializable. Which sub-object answers an IID is the core's
// decision (fc4_landfill_query_interface).
//
// Landfill tiles are kept in the C ABI layer, not in the game (fc4.h): the
// game's cSC4LandfillOccupant takes its tile exemplar in a way neither the
// spec nor docs/abi documents, so no tile occupant is created. The cells keep
// pollution's active-landfill flag (§4.8.2), which is what the simulation
// reads; no tile model is drawn (logged once). Two more parts are left out
// and logged once: SetEmptyCellTexture (the base-texture occupant's factory
// is not documented) and the zone effects (§4.10, rendering only).
//
// Threads. The game calls the getters from a second thread while the main
// thread is inside a tick (HANDOFF §5.1), and the core is single-threaded, so
// every call into it holds this object's lock. The main thread waits for the
// lock; any other thread tries for up to kLockWaitMs and otherwise answers
// from a snapshot of the scalar values (taken at the end of every main-thread
// call) or with the neutral value, and counts a fallback (the month status
// line). Nothing a getter on another thread reads lives outside that lock.
//
// The subsystem is OFF by default: FreeCity4.ini `enable = landfill`;
// `disable` still wins. The landfill_* switches (FC4Config.h) turn groups of
// game calls off for bisecting.
#pragma once

#include "FC4Interfaces.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <atomic>
#include <cstdint>
#include <memory>
#include <vector>

class cIGZMessageServer2;
class cISC4City;

static const uint32_t kFC4_CLSID_cSC4LandfillDeveloper = 0xEA09D4AE;
static const uint32_t kFC4_IID_cISC4LandfillDeveloper = 0x8A09D502;

namespace fc4 {

// What the landfill host callbacks work on. The developer owns one; the
// landfill shadow (FC4LandfillShadow.h) fills one with a city's borrowed
// services for one delivery. Main thread, or another thread holding the
// developer's lock.
struct LandfillContext {
    bool readOnly = false;      // the shadow: borrowed city, read callbacks only
    bool gated = true;          // false: the landfill_* switches are ignored (the shadow reads as it is)
    cISC4City* city = nullptr;  // developer: held from acquire_services to release_services
    cIGZMessageServer2* msgServer = nullptr;  // developer only, held likewise
    cIGZMessageTarget2* target = nullptr;     // developer only: the agent and notification target
    bool agentsRegistered = false;
    // Game lists (and the references they hold) kept until the outermost call
    // into the core returns.
    std::vector<std::shared_ptr<void>> held;
    // Observability (the month status line): calls made since construction.
    uint32_t placeZones = 0;      // PlaceZone (type 0, §4.4.8)
    uint32_t zotsPlaced = 0;      // AddLotProp that returned a prop
    uint32_t zotsFailed = 0;      // ... that returned null
    uint32_t zotsRemoved = 0;     // RemovePropA
    uint32_t occupantsRemoved = 0;  // RemoveOccupant (game tiles, flora, base textures)
    uint32_t levellings = 0;      // AttemptAreaLevelling
    uint32_t historySends = 0;    // history records sent
};

// The host table over `ctx` (the developer's).
fc4_landfill_host MakeLandfillHost(LandfillContext* ctx);
// The read-only table for the shadow: every callback that would change the
// game (lifecycle, references, pollution and budget writes, zoning,
// occupant changes, props, levelling, terrain writes, effects, history) is
// NULL. The shadow core records those writes itself and never calls them.
fc4_landfill_host MakeLandfillReadOnlyHost(LandfillContext* ctx);

}  // namespace fc4

class cFC4LandfillDeveloper final : public cFC4ISC4LandfillDeveloper,
                                    public cIGZMessageTarget2,
                                    public cIGZSerializable {
public:
    cFC4LandfillDeveloper();
    ~cFC4LandfillDeveloper();

    // cIGZUnknown (all three interfaces)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4LandfillDeveloper (§7.1)
    bool Init() override;
    bool Shutdown() override;
    SC4Percentage* GetEfficiency(SC4Percentage* result) override;
    uint32_t GetSimulatorType() override;
    int64_t GetMaintenanceCosts() override;
    int64_t GetCurrentBudget() override;
    int64_t GetZoningCost(SC4CellRegion<int32_t> const* region) override;
    void GetLandfillUsage(uint32_t& usage, uint32_t& capacity) override;
    int32_t GetLandfillTileCount() const override;
    intptr_t GetLandfillZoneFromCell(int32_t x, int32_t z) override;

    // cIGZMessageTarget2 (§3.2)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§6.1)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // How long a thread other than the main one waits for the lock.
    static constexpr uint32_t kLockWaitMs = 20;

    // Observability: calls answered from the snapshot / neutral value.
    uint32_t Fallbacks() const { return fallbacks_.load(); }

private:
    // Holds the lock for one call into the core (see the file comment). On a
    // thread other than the main one ok() may be false: answer from the
    // snapshot or with the neutral value.
    class Section {
    public:
        explicit Section(const cFC4LandfillDeveloper* s);
        ~Section();
        Section(const Section&) = delete;
        Section& operator=(const Section&) = delete;
        bool ok() const { return ok_; }

    private:
        cFC4LandfillDeveloper* s_;
        bool ok_;
    };

    // The scalar values a getter on another thread falls back to.
    struct Snapshot {
        std::atomic<uint32_t> efficiencyBits{0x3F800000u};  // 1.0f (§4.1 default efficiency)
        std::atomic<int64_t> maintenance{0}, budget{0};
        std::atomic<uint32_t> usage{0}, capacity{0};
        std::atomic<int32_t> tiles{0};
    };

    bool Enter();
    void Leave();
    void RefreshSnapshot();
    void LogState(const char* what);

    cIGZUnknown* Primary() { return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4LandfillDeveloper*>(this)); }

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::LandfillContext ctx_;
    fc4_landfill_host host_{};
    fc4_landfill* core_ = nullptr;

    mutable CRITICAL_SECTION lock_;
    DWORD mainTid_ = 0;
    int depth_ = 0;  // nesting of sections held (by the lock holder)
    mutable std::atomic<uint32_t> fallbacks_{0};
    Snapshot snap_;
    uint32_t months_ = 0;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4LandfillDeveloper();
