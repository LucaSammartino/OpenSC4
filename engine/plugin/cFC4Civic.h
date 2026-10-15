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

// The civic building simulator (cSC4CivicBuildingSimulator, CLSID 0x0A413481,
// city component 39): docs/specs/civic-buildings.md. The behaviour lives in
// the Rust core behind fc4.h (fc4_civic); this class implements the three
// interfaces (§1: cISC4CivicBuildingSimulator at +0, cIGZMessageTarget2 at
// +4, cIGZSerializable at +8) in the slot order of §8 and the host callbacks
// over the game's services (budget, lots, occupants, automata).
//
// Statuses (§4.7): slot 6 hands out a ConditionalBuildingStatus* that callers
// (advisors, Lua, the ordinance) write through. The core keeps the statuses;
// this class keeps a MIRROR, one entry per key at a fixed address, in the
// game's layout ({u8 status; u16 buildingCount; cRZString itemToolTip}, the
// string a game cRZString with a buffer from the game's STL node allocator,
// allocators.md, FC4GameList.h). [choice] The hash containers of §2 do not
// cross the interface (§9 lists slots 5-8 only), so the mirror is an
// ordinary map of keys to entries; only the entries cross. Every change a
// caller made through a mirror entry is written back to the core before the
// core next reads or writes the statuses (an occupant message, a slot 7
// call, Write), and the mirror is refreshed from the core afterwards.
//
// Threads: the core is main-thread only and serialised by a lock (slot 7 may
// come from any thread). Slots 5, 6 and 8 read mirrors (exclusions, entries,
// the landmark count) under their own lock, refreshed after each core call
// that can change them.
//
// OFF by default: FreeCity4.ini `enable = civic` (FC4Subsystems.h).
//
// Log lines (FreeCity4.log):
//   civic Init|Read|Shutdown: ok=.. exclusions=.. statuses=.. centers=.. maintained=.. landmarks=..
//   civic Write -> 0|1 (..)
//   civic month YYYY-MM-DD: exclusions=.. statuses=.. centers=.. maintained=.. temporaries=..
//     occupants=../.. lots=../.. writebacks=..
#pragma once

#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4CivicBuildingSimulator.h"
#include "fc4.h"

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <mutex>
#include <string>
#include <unordered_map>
#include <unordered_set>
#include <vector>

static const uint32_t kFC4_CLSID_cSC4CivicBuildingSimulator = 0x0A413481;
static const uint32_t kFC4_IID_cISC4CivicBuildingSimulator = 0x8A41322F;

namespace fc4 {

// What the civic host callbacks work on. The simulator owns one; the civic
// shadow (FC4CivicShadow.h) makes a read-only one (no writes to the game).
struct CivicContext {
    bool readOnly = false;
    cIGZMessageTarget2* target = nullptr;  // the simulator's +4 (agents, notifications)
    bool agentRegistered = false;
    bool luaNoted = false;
    uint32_t subscriptions = 0;
    uint32_t holds = 0;  // references the core holds through add_ref / resolve_ref
};

fc4_civic_host MakeCivicHost(CivicContext* ctx);

// The session seed S (foundations §6.3): `civic_seed` / FC4_CIVIC_SEED, else
// the cRZTime clock.
uint32_t CivicSessionSeed(bool* fixed);

// city.GetAutomataControllerManager(), if any: AddControllersFromOccupantGroup(
// name, occ, 0) with the name as a game cRZString (civic §4.6 "ape_escape",
// parks §4.5 "zoomanji"); one log line prefixed with `who`.
void AddOccupantGroupControllers(const char* who, const char* name, uint64_t occ);

// One mirrored status in the game's layout (SDK ConditionalBuildingStatus).
struct CivicStatusEntry {
    uint8_t status;
    uint8_t pad0;
    uint16_t buildingCount;
    // A game cRZString {vtable, begin, end, capacity_end, refcount}.
    const void* textVtable;
    char* textBegin;
    char* textEnd;
    char* textCapacityEnd;
    uint32_t textRefCount;
};
static_assert(sizeof(void*) != 4 || sizeof(CivicStatusEntry) == 0x18, "ConditionalBuildingStatus is 0x18 bytes");
static_assert(offsetof(CivicStatusEntry, buildingCount) == 2, "buildingCount at +2");
static_assert(sizeof(void*) != 4 || offsetof(CivicStatusEntry, textVtable) == 4, "itemToolTip at +4");

}  // namespace fc4

class cFC4CivicBuildingSimulator final : public cISC4CivicBuildingSimulator,
                                         public cIGZMessageTarget2,
                                         public cIGZSerializable {
public:
    cFC4CivicBuildingSimulator();
    ~cFC4CivicBuildingSimulator();

    // cIGZUnknown (§1)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4CivicBuildingSimulator (§8)
    bool Init() override;
    bool Shutdown() override;
    bool IsBuildingExcluded(uint32_t buildingID) override;
    ConditionalBuildingStatus* GetConditionalBuildingStatus(uint32_t buildingID) const override;
    bool UpdateConditionalBuildingStatus(uint32_t buildingID, ConditionalBuildingStatus* status) override;
    uint32_t GetLandmarkCount() override;

    // cIGZMessageTarget2 (§3)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§7)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // The core (tests, the shadow's diagnostics); main thread.
    fc4_civic* Core() const { return core_; }

private:
    struct Mirror {
        fc4::CivicStatusEntry game{};
        // What the core last held for this key (to see the callers' writes).
        uint8_t syncedStatus = 0;
        uint16_t syncedCount = 0;
        std::string syncedText;
    };

    // Callers' writes through mirror entries -> the core (lock_ held).
    void FlushStatuses();
    // The core -> the mirrors (statuses, exclusions, landmark count).
    void PullStatuses();
    void PullOthers();
    Mirror* EnsureMirror(uint32_t key);
    void LogState(const char* what, bool ok);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::CivicContext ctx_;
    fc4_civic_host host_{};
    fc4_civic* core_ = nullptr;
    uint32_t seed_ = 0;
    bool seedFixed_ = false;

    std::recursive_mutex lock_;  // the core
    mutable std::mutex mirrorLock_;
    std::unordered_map<uint32_t, std::unique_ptr<Mirror>> mirrors_;
    std::unordered_set<uint32_t> exclusions_;
    std::atomic<uint32_t> landmarks_{0};
    bool mirrorRefused_ = false;
    mutable std::atomic<bool> loggedNoMirror_{false};

    uint32_t occupantMsgs_ = 0, buildingMsgs_ = 0, lotMsgs_ = 0, activations_ = 0, writebacks_ = 0;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4CivicBuildingSimulator();
