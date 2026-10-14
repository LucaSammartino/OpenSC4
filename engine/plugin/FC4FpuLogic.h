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

// Pure decision logic of the x87 precision re-assert (foundations §18.2),
// header-only and free of Windows calls so tests/fpu_logic_test.cpp can check
// it directly. No game address is known here; FC4Fpu.cpp gets the helper
// itself from compat (the only place allowed to).
#pragma once

#include <cstddef>
#include <cstdint>
#include <cstdio>

namespace fc4 {

// The precision-control field of an x87 control word (bits 8-9): 0 = 24-bit,
// 2 = 53-bit, 3 = 64-bit, 1 = reserved.
inline unsigned PcField(uint16_t cw) { return (cw >> 8) & 3u; }

inline bool Is24Bit(uint16_t cw) { return PcField(cw) == 0u; }

// The mode the game's helper takes. It clears the PC field, then sets bit 1
// for mode 1 (53-bit) or bits 0-1 for mode 2 (64-bit), so mode 0 is 24-bit -
// the only mode FreeCity4 may pass (§18.2: the unmodified game runs the whole
// session at 24-bit).
constexpr int kFpuMode24Bit = 0;
constexpr int kFpuMode53Bit = 1;
constexpr int kFpuMode64Bit = 2;

// The control word the helper leaves behind for `mode`: PC changes, rounding
// and the exception masks do not.
inline uint16_t ApplyMode(uint16_t cw, int mode) {
    uint16_t out = static_cast<uint16_t>(cw & ~uint16_t(0x0300));
    if (mode == kFpuMode53Bit) {
        out = static_cast<uint16_t>(out | uint16_t(0x0200));  // PC = 10
    } else if (mode == kFpuMode64Bit) {
        out = static_cast<uint16_t>(out | uint16_t(0x0300));  // PC = 11
    }
    return out;
}

enum class ReassertOutcome {
    AlreadyCorrect,  // it was 24-bit and stayed so
    Restored,        // it was above 24-bit and the helper brought it back
    Failed,          // it is still above 24-bit: report this, do not trust the run
};

// How to report a re-assert from the control word before and after it.
// "Still wrong" outranks "unchanged": a helper that did nothing at 53-bit is a
// failure, not a no-op success.
inline ReassertOutcome ReassertOutcomeOf(uint16_t before, uint16_t after) {
    if (!Is24Bit(after)) return ReassertOutcome::Failed;
    return after == before ? ReassertOutcome::AlreadyCorrect : ReassertOutcome::Restored;
}

}  // namespace fc4
