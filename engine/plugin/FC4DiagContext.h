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

// The crash report's lines that come from a CONTEXT (FC4Diag.cpp LogForeign):
// the x87 state and the raw dwords at esp. Header-only, no allocation, so the
// vectored handler can use it and tests/fpu_logic_test.cpp can check it with a
// synthetic context. x86 only.
#pragma once

#include "FC4FpuLogic.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdint>
#include <cstdio>

namespace fc4 {

#if defined(_M_IX86)
// "fpu: CW=... SW=... (TOP=.. SF=. IE=. C1=.) TW=0x.... occupied=N [FXSAVE FTW=0x.. occupied=N] indefinite NaN: ..."
// from FloatSave (FNSAVE layout; the tag word is decoded as the full tag)
// and, with CONTEXT_EXTENDED_REGISTERS, the FXSAVE image (abridged tag).
inline int FormatContextFpu(const CONTEXT* c, char* out, size_t cap) {
    if (c == nullptr || (c->ContextFlags & CONTEXT_FLOATING_POINT) != CONTEXT_FLOATING_POINT) {
        return std::snprintf(out, cap, "fpu: not in the context (flags 0x%08lX)",
                             c ? static_cast<unsigned long>(c->ContextFlags) : 0ul);
    }
    const FLOATING_SAVE_AREA& f = c->FloatSave;
    const X87State fn = DecodeFnsave(static_cast<uint16_t>(f.ControlWord), static_cast<uint16_t>(f.StatusWord),
                                     static_cast<uint16_t>(f.TagWord), f.RegisterArea);
    if ((c->ContextFlags & CONTEXT_EXTENDED_REGISTERS) == CONTEXT_EXTENDED_REGISTERS) {
        const X87State fx = DecodeFxsave(c->ExtendedRegisters);
        return FormatCrashFpu(out, cap, fn, &fx);
    }
    return FormatCrashFpu(out, cap, fn, nullptr);
}
#endif

// "esp dwords: 0x........ x8 (at esp=0x........)" from the 8 dwords read at
// esp, or "esp dwords: not readable (esp=0x........ outside the thread's
// stack)" when `words` is null.
inline int FormatEspDwords(char* out, size_t cap, uint32_t esp, const uint32_t* words) {
    if (words == nullptr) {
        return std::snprintf(out, cap, "esp dwords: not readable (esp=0x%08X outside the thread's stack)", esp);
    }
    return std::snprintf(out, cap,
                         "esp dwords: 0x%08X 0x%08X 0x%08X 0x%08X 0x%08X 0x%08X 0x%08X 0x%08X (at esp=0x%08X)",
                         words[0], words[1], words[2], words[3], words[4], words[5], words[6], words[7], esp);
}

}  // namespace fc4
