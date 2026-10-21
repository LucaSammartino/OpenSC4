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

// The game's advisor win proc operations (advisors.md §12.1.1 "Direct-call ABI", AD4).

#include "FC4AdvisorWindow.h"

#include "compat/FC4Compat.h"
#include "cIGZUnknown.h"

namespace fc4 {
namespace {

// thiscall with no stack arguments, called through __fastcall: ECX = the block base, EDX unused, pop 0.
using ThisFn = void*(__fastcall*)(void* self, void* unusedEdx);
using AllocateFn = void*(__cdecl*)(uint32_t size);

uintptr_t g_va[6] = {};

void* Allocate(uint32_t size) { return reinterpret_cast<AllocateFn>(g_va[0])(size); }
void* Construct(void* block) { return reinterpret_cast<ThisFn>(g_va[1])(block, nullptr); }
void Init(void* p) { reinterpret_cast<ThisFn>(g_va[2])(p, nullptr); }
void Show(void* p) { reinterpret_cast<ThisFn>(g_va[3])(p, nullptr); }
void Hide(void* p) { reinterpret_cast<ThisFn>(g_va[4])(p, nullptr); }
void Teardown(void* p) { reinterpret_cast<ThisFn>(g_va[5])(p, nullptr); }
// The block base answers QueryInterface for cIGZUnknown (§12.1.1 "Creation route"): slots 1 / 2.
void AddRefBlock(void* p) { static_cast<cIGZUnknown*>(p)->AddRef(); }
void ReleaseBlock(void* p) { static_cast<cIGZUnknown*>(p)->Release(); }

}  // namespace

bool GameAdvisorWinProcOps(AdvisorWinProcOps* out) {
    using compat::Direct;
    const Direct which[6] = {Direct::GlobalHeapAllocate,  Direct::AdvisorWinProcConstruct,
                             Direct::AdvisorWinProcInit,  Direct::AdvisorWinProcShow,
                             Direct::AdvisorWinProcHide,  Direct::AdvisorWinProcTeardown};
    for (int i = 0; i < 6; ++i) {
        g_va[i] = compat::DirectCall(which[i]);
        if (g_va[i] == 0) return false;
    }
    out->allocate = &Allocate;
    out->construct = &Construct;
    out->init = &Init;
    out->show = &Show;
    out->hide = &Hide;
    out->teardown = &Teardown;
    out->addRef = &AddRefBlock;
    out->release = &ReleaseBlock;
    return true;
}

}  // namespace fc4
