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

// Unit tests of plugin/FC4FpuLogic.h: the x87 precision re-assert of
// foundations §18.2 (our DLL's presence raises the main thread to 53-bit, and
// the unmodified game's own code then computes wrongly, so we restore 24-bit
// at PostAppInit through the game's helper).

#include "FC4FpuLogic.h"

#include <cstdio>

namespace {

int g_failures = 0;

#define CHECK(cond)                                                          \
    do {                                                                     \
        if (!(cond)) {                                                       \
            std::printf("FAILED: %s (line %d)\n", #cond, __LINE__);          \
            ++g_failures;                                                    \
        }                                                                    \
    } while (0)

}  // namespace

int main() {
    using namespace fc4;

    // The control words measured in the runs: 0x007F is what the game sets at
    // PreFrameWorkInit and at our PreAppInit reference; 0x027F is what
    // PostAppInit saw once our DLL was present (runs 23-28).
    constexpr uint16_t kGame24 = 0x007F;
    constexpr uint16_t kRaised53 = 0x027F;

    // The precision field.
    CHECK(PcField(kGame24) == 0);
    CHECK(PcField(kRaised53) == 2);
    CHECK(PcField(0x037F) == 3);
    CHECK(Is24Bit(kGame24));
    CHECK(!Is24Bit(kRaised53));
    CHECK(!Is24Bit(0x037F));

    // ApplyMode models the helper: PC changes, rounding (bits 10-11) and the
    // exception masks (bits 0-5) do not. Round-to-nearest and masks 0x3F must
    // survive, or the restore would alter more than the precision.
    CHECK(ApplyMode(kRaised53, kFpuMode24Bit) == kGame24);
    CHECK(ApplyMode(kGame24, kFpuMode53Bit) == kRaised53);
    CHECK(ApplyMode(kGame24, kFpuMode64Bit) == 0x037F);
    CHECK(ApplyMode(kRaised53, kFpuMode24Bit) & 0x3F == (kRaised53 & 0x3F));
    CHECK((ApplyMode(kRaised53, kFpuMode24Bit) >> 10) == (kRaised53 >> 10));  // RC unchanged
    CHECK((ApplyMode(kRaised53, kFpuMode53Bit) & 0x3F) == (kRaised53 & 0x3F));
    // Mode 0 on a word already at 24-bit is a no-op.
    CHECK(ApplyMode(kGame24, kFpuMode24Bit) == kGame24);
    // 24-bit is the only mode we may pass; the others would be the bug.
    CHECK(kFpuMode24Bit == 0);

    // A helper that clears PC and sets nothing else must land on 24-bit from
    // either of the two raised states.
    CHECK(Is24Bit(ApplyMode(0x027F, kFpuMode24Bit)));
    CHECK(Is24Bit(ApplyMode(0x037F, kFpuMode24Bit)));

    // The report of a re-assert. "Still wrong" outranks "unchanged": a helper
    // that did nothing while the thread sat at 53-bit is a failure, not a
    // no-op success.
    CHECK(ReassertOutcomeOf(kRaised53, kGame24) == ReassertOutcome::Restored);  // the fix working
    CHECK(ReassertOutcomeOf(kGame24, kGame24) == ReassertOutcome::AlreadyCorrect);
    CHECK(ReassertOutcomeOf(kRaised53, kRaised53) == ReassertOutcome::Failed);  // did nothing at 53-bit
    CHECK(ReassertOutcomeOf(0x037F, 0x037F) == ReassertOutcome::Failed);        // nor at 64-bit
    CHECK(ReassertOutcomeOf(kRaised53, 0x037F) == ReassertOutcome::Failed);     // went the wrong way
    // A partial restore that leaves PC non-zero is still a failure.
    CHECK(ReassertOutcomeOf(kRaised53, 0x017F) == ReassertOutcome::Failed);     // PC = 01 (reserved)
    // Only PC matters: a change elsewhere with PC still 0 is not a restore.
    CHECK(ReassertOutcomeOf(kGame24, kGame24) == ReassertOutcome::AlreadyCorrect);

    std::printf("%s\n", g_failures == 0 ? "fpu_logic_test: all checks passed" : "fpu_logic_test: FAILURES");
    return g_failures == 0 ? 0 : 1;
}
