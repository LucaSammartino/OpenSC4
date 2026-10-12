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

// Calls an interface method the way the game does: through a vtable SLOT, with
// __thiscall and an explicit list of 32-bit stack arguments, and measures
// whether the callee popped exactly what was pushed (foundations §16).
// x86 (32-bit MSVC) only.
#pragma once

#include <cstdint>
#include <initializer_list>

namespace fc4test {

struct SlotCall {
    uint32_t eax = 0;
    uint32_t edx = 0;
    int32_t espDelta = 0;  // bytes left on (+) or taken off (-) the stack; 0 = balanced
    int x87Pushed = 0;     // values the callee left on the x87 stack (1 = returned a float in ST0)
};

// Pops one value the callee left on the x87 stack (a float result).
inline void PopX87() {
    __asm fstp st(0)
}

inline SlotCall CallSlot(void* self, int slot, std::initializer_list<uint32_t> args) {
    uint32_t a[16] = {};
    int n = 0;
    for (uint32_t v : args) {
        if (n < 16) a[n++] = v;
    }
    void* fn = (*reinterpret_cast<void***>(self))[slot];
    uint32_t* pa = a;
    uint32_t rEax = 0, rEdx = 0, before = 0, after = 0;
    uint16_t swBefore = 0, swAfter = 0;
    __asm {
        fnstsw swBefore
        mov before, esp
        mov ecx, n
        mov esi, pa
    push_loop:
        test ecx, ecx
        jz pushed
        dec ecx
        push dword ptr [esi + ecx * 4]
        jmp push_loop
    pushed:
        mov ecx, self
        call fn
        mov rEax, eax
        mov rEdx, edx
        mov after, esp
        mov esp, before
        fnstsw swAfter
    }
    SlotCall r;
    r.eax = rEax;
    r.edx = rEdx;
    r.espDelta = static_cast<int32_t>(before - after);
    // x87 TOP (status word bits 11-13) decreases by one per push.
    r.x87Pushed = ((swBefore >> 11) - (swAfter >> 11)) & 7;
    return r;
}

inline int64_t AsI64(const SlotCall& c) {
    return static_cast<int64_t>((static_cast<uint64_t>(c.edx) << 32) | c.eax);
}

}  // namespace fc4test
