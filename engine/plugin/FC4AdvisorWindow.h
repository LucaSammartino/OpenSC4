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

// The advisor system's window slots 23-28 (advisors.md §12.1.1) for a replacement system that keeps the
// original advisor window: the system member "+0x5C" (one owned reference to the 0x70-byte win proc block,
// null until slot 24 runs) and the five bodies, run through injected operations so the logic is testable.
// In the plugin the operations are the game's code (GameAdvisorWinProcOps, FC4AdvisorWindowGame.cpp:
// the global heap allocate and the five direct calls of the AD4 ABI, AddRef / Release through the block
// base's cIGZUnknown slots 1 / 2).
#pragma once

#include <cstdint>

namespace fc4 {

struct AdvisorWinProcOps {
    void* (*allocate)(uint32_t size);  // the game heap (0x5E55E0)
    void* (*construct)(void* block);   // 0x41DB50: EAX = this, refcount 0
    void (*init)(void* winProc);       // 0x41E320
    void (*show)(void* winProc);       // 0x41D650
    void (*hide)(void* winProc);       // 0x41D710
    void (*teardown)(void* winProc);   // 0x41D830
    void (*addRef)(void* winProc);     // block base slot 1
    void (*release)(void* winProc);    // block base slot 2
};

// The win proc's visible byte (§12.1.1 slot 25/26: "+0x56").
constexpr uint32_t kAdvisorWinProcVisibleOffset = 0x56;
constexpr uint32_t kAdvisorWinProcSize = 0x70;

class AdvisorWindowSlot {
public:
    AdvisorWindowSlot() = default;
    explicit AdvisorWindowSlot(const AdvisorWinProcOps& ops) : ops_(ops), haveOps_(true) {}
    AdvisorWindowSlot(const AdvisorWindowSlot&) = delete;
    AdvisorWindowSlot& operator=(const AdvisorWindowSlot&) = delete;

    void SetOps(const AdvisorWinProcOps& ops) {
        ops_ = ops;
        haveOps_ = true;
    }
    bool HasOps() const { return haveOps_; }

    // Slot 23 GetAdvisorWinProc: the member, no AddRef.
    void* Get() const { return winProc_; }

    // Slot 24 StartAdvisorWindow. Returns false only when the operations are missing (nothing done).
    bool Start() {
        if (winProc_ != nullptr) return true;
        if (!haveOps_) return false;
        // §12.1.1: allocate 0x70 bytes, construct directly; no null test (an allocation failure crashes in
        // the original, quirk reproduced literally).
        void* created = ops_.construct(ops_.allocate(kAdvisorWinProcSize));
        if (created != winProc_) {
            ops_.addRef(created);
            void* old = winProc_;
            winProc_ = created;
            if (old != nullptr) ops_.release(old);
        }
        ops_.init(winProc_);
        return true;
    }

    // Slot 25 (show), slot 26 (hide): only when the window exists.
    void Show() {
        if (winProc_ != nullptr && haveOps_) ops_.show(winProc_);
    }
    void Hide() {
        if (winProc_ != nullptr && haveOps_) ops_.hide(winProc_);
    }

    // Slot 27: the win proc's byte +0x56, else false.
    bool IsVisible() const {
        if (winProc_ == nullptr) return false;
        return static_cast<const uint8_t*>(winProc_)[kAdvisorWinProcVisibleOffset] != 0;
    }

    // Slot 28 RemoveWinProcs: teardown, then null the member and Release it.
    void Remove() {
        if (winProc_ == nullptr || !haveOps_) return;
        ops_.teardown(winProc_);
        void* p = winProc_;
        if (p != nullptr) {
            winProc_ = nullptr;
            ops_.release(p);
        }
    }

private:
    AdvisorWinProcOps ops_{};
    bool haveOps_ = false;
    void* winProc_ = nullptr;
};

// The private global (advisors.md §14.2): the original constructor stores the system's primary
// cISC4AdvisorSystem pointer there and both destructors store null; the original Lua extensions a
// replacement registers (§8.3.1 rows 5-9, §8.3.2) read it with no null test. `slot` is the compat address
// (compat::AdvisorSystemGlobal()), null when the executable is not identified.
class AdvisorSystemGlobalSlot {
public:
    explicit AdvisorSystemGlobalSlot(void** slot) : slot_(slot) {}
    bool Available() const { return slot_ != nullptr; }
    bool Publish(void* system) {
        if (slot_ == nullptr) return false;
        *slot_ = system;
        return true;
    }
    // Literal reading of §14.2 ("store null"): unconditional, as the original destructors do.
    void Withdraw() {
        if (slot_ != nullptr) *slot_ = nullptr;
    }
    void* Current() const { return slot_ != nullptr ? *slot_ : nullptr; }

private:
    void** slot_;
};

// The game's operations (plugin only): false when any target is unavailable (compat refused it).
bool GameAdvisorWinProcOps(AdvisorWinProcOps* out);

}  // namespace fc4
