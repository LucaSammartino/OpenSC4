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

#include "FC4DiagContext.h"
#include "FC4FpuLogic.h"

#include <cstdio>
#include <cstring>
#include <string>

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
    CHECK((ApplyMode(kRaised53, kFpuMode24Bit) & 0x3F) == (kRaised53 & 0x3F));
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

    // ---- the x87 register stack: tag words, the indefinite NaN, the crash
    // report's FPU line (FC4Diag.cpp) from a synthetic CONTEXT ----

    // Full tag: two bits per physical register, 11 = empty.
    CHECK(OccupiedFromFullTag(0xFFFF) == 0);
    CHECK(OccupiedFromFullTag(0x0000) == 8);
    CHECK(OccupiedFromFullTag(0xFFFC) == 1);  // register 0 valid (00)
    CHECK(OccupiedFromFullTag(0x3FFF) == 1);  // register 7 valid (00)
    CHECK(OccupiedFromFullTag(0xBFFE) == 2);  // register 7 special (10), register 0 zero (10 is special, 01 zero)
    // Abridged tag (FXSAVE): one bit per physical register, 1 = valid.
    CHECK(OccupiedFromAbridgedTag(0x00) == 0);
    CHECK(OccupiedFromAbridgedTag(0x80) == 1);
    CHECK(OccupiedFromAbridgedTag(0x81) == 2);
    CHECK(OccupiedFromAbridgedTag(0xFF) == 8);
    // Status word fields.
    CHECK(X87TopOf(0x3800) == 7 && X87TopOf(0x0000) == 0 && X87TopOf(0x3241) == 6);
    CHECK(X87StackFault(0x0041) && !X87StackFault(0x0001));
    CHECK(X87InvalidOp(0x0041) && !X87InvalidOp(0x0040));
    CHECK(X87C1(0x0200) && !X87C1(0x0041));

    // The real indefinite: sign 1, exponent 0x7FFF, significand 0xC000000000000000.
    const uint8_t indef[10] = {0, 0, 0, 0, 0, 0, 0, 0xC0, 0xFF, 0xFF};
    const uint8_t one[10] = {0, 0, 0, 0, 0, 0, 0, 0x80, 0xFF, 0x3F};      // +1.0
    const uint8_t qnan[10] = {0, 0, 0, 0, 0, 0, 0, 0xC0, 0xFF, 0x7F};     // the positive quiet NaN: not it
    const uint8_t payload[10] = {1, 0, 0, 0, 0, 0, 0, 0xC0, 0xFF, 0xFF};  // a NaN with a payload: not it
    CHECK(IsX87Indefinite(indef));
    CHECK(!IsX87Indefinite(one));
    CHECK(!IsX87Indefinite(qnan));
    CHECK(!IsX87Indefinite(payload));

    char line[256];
    FormatIndefinite(line, sizeof(line), 0);
    CHECK(std::string(line) == "none");
    FormatIndefinite(line, sizeof(line), 0x81);
    CHECK(std::string(line) == "ST0 ST7");

#if defined(_M_IX86)
    // A fault with two values on the stack: TOP = 6, so ST0 is physical
    // register 6 and ST1 physical register 7. ST0 holds the indefinite (an
    // overflowed load), ST1 holds 1.0; SF, IE and C1 are set (an overflow).
    static CONTEXT c;
    std::memset(&c, 0, sizeof(c));
    c.ContextFlags = CONTEXT_FLOATING_POINT | CONTEXT_EXTENDED_REGISTERS;
    c.FloatSave.ControlWord = 0xFFFF007F;  // only the low 16 bits are the word
    c.FloatSave.StatusWord = 0x3241;
    c.FloatSave.TagWord = 0x0FFF;  // registers 6 and 7 valid (00), the rest empty (11)
    std::memcpy(c.FloatSave.RegisterArea + 0, indef, 10);
    std::memcpy(c.FloatSave.RegisterArea + 10, one, 10);
    c.ExtendedRegisters[0] = 0x7F;  // FCW
    c.ExtendedRegisters[1] = 0x00;
    c.ExtendedRegisters[2] = 0x41;  // FSW
    c.ExtendedRegisters[3] = 0x32;
    c.ExtendedRegisters[4] = 0xC0;  // abridged tag: registers 6 and 7
    std::memcpy(c.ExtendedRegisters + 32, indef, 10);
    std::memcpy(c.ExtendedRegisters + 48, one, 10);
    FormatContextFpu(&c, line, sizeof(line));
    CHECK(std::string(line) == "fpu: CW=0x007F SW=0x3241 (TOP=6 SF=1 IE=1 C1=1) TW=0x0FFF occupied=2 "
                               "[FXSAVE FTW=0xC0 occupied=2] indefinite NaN: ST0");
    const X87State fn = DecodeFnsave(0x007F, 0x3241, 0x0FFF, c.FloatSave.RegisterArea);
    CHECK(fn.occupied == 2 && fn.indefinite == 0x01 && !fn.abridged);
    const X87State fx = DecodeFxsave(c.ExtendedRegisters);
    CHECK(fx.cw == 0x007F && fx.sw == 0x3241 && fx.tag == 0xC0 && fx.abridged && fx.occupied == 2 &&
          fx.indefinite == 0x01);

    // Without the extended registers: the FloatSave part alone.
    c.ContextFlags = CONTEXT_FLOATING_POINT;
    FormatContextFpu(&c, line, sizeof(line));
    CHECK(std::string(line) == "fpu: CW=0x007F SW=0x3241 (TOP=6 SF=1 IE=1 C1=1) TW=0x0FFF occupied=2 "
                               "indefinite NaN: ST0");

    // An empty stack, as a clean fault shows it.
    c.FloatSave.StatusWord = 0x0000;
    c.FloatSave.TagWord = 0xFFFF;
    std::memset(c.FloatSave.RegisterArea, 0, sizeof(c.FloatSave.RegisterArea));
    FormatContextFpu(&c, line, sizeof(line));
    CHECK(std::string(line) == "fpu: CW=0x007F SW=0x0000 (TOP=0 SF=0 IE=0 C1=0) TW=0xFFFF occupied=0 "
                               "indefinite NaN: none");

    // No floating-point state in the context.
    c.ContextFlags = CONTEXT_CONTROL;
    FormatContextFpu(&c, line, sizeof(line));
    CHECK(std::string(line).rfind("fpu: not in the context (flags 0x", 0) == 0);
#endif

    // The raw dwords at esp.
    const uint32_t words[8] = {1, 2, 3, 0xDEADBEEF, 5, 6, 7, 8};
    FormatEspDwords(line, sizeof(line), 0x0019FA00u, words);
    CHECK(std::string(line) == "esp dwords: 0x00000001 0x00000002 0x00000003 0xDEADBEEF 0x00000005 0x00000006 "
                               "0x00000007 0x00000008 (at esp=0x0019FA00)");
    FormatEspDwords(line, sizeof(line), 0x10u, nullptr);
    CHECK(std::string(line) == "esp dwords: not readable (esp=0x00000010 outside the thread's stack)");
    std::printf("%s\n", g_failures == 0 ? "fpu_logic_test: all checks passed" : "fpu_logic_test: FAILURES");
    return g_failures == 0 ? 0 : 1;
}
