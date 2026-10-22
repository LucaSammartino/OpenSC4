// SPDX-License-Identifier: GPL-3.0-or-later
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

#include "FC4Span565Bridge.h"

#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "fc4.h"

#include "cIGZBuffer.h"
#include "cIGZCOM.h"
#include "cIGZFrameWork.h"
#include "cRZCOMDllDirector.h"

#include <cstdint>
#include <cstring>
#include <vector>
#include <intrin.h>

#include <windows.h>

namespace fc4 {
namespace {

// span-render-565.md §1.1: vtable 0x00AC1844, slot 0 (deleting destructor) and
// the five methods. All six are checked; only 1-5 are written.
constexpr uintptr_t kVtable = 0x00AC1844;
constexpr uintptr_t kExpected[6] = {0x0082FF00, 0x0083DE50, 0x0083DEA0, 0x0083DFB0, 0x0083E070, 0x0083E210};
const char* const kSlotName[6] = {"dtor", "RenderSolid", "RenderGradient", "RenderAlpha", "RenderAlphaGradient",
                                  "RenderBlt"};

bool g_enabled = false;
bool g_installed = false;
// The trace: its thread (calls on it are "constructed") and the forced-native switch.
volatile DWORD g_traceThread = 0;
volatile LONG g_forceNative = 0;
// [origin 0 shipped / 1 constructed][slot 1..5]
volatile LONG g_ours[2][6] = {};
volatile LONG g_declined[2][6] = {};
volatile LONG g_native[2][6] = {};

// §1.1: MSVC thiscall, ECX never read, callee pops; called as __fastcall with EDX passed through.
using SolidFn = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t, const fc4_span565_dest*, uint32_t);
using GradFn = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t, const fc4_span565_dest*, const uint32_t*);
using AlphaFn = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t, const fc4_span565_dest*, uint32_t,
                                  uint32_t);
using BltFn = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t, const fc4_span565_dest*,
                                const fc4_span565_source*, const fc4_span565_blt*, uint32_t);

int Origin() { return g_traceThread != 0 && GetCurrentThreadId() == g_traceThread ? 1 : 0; }

// true: draw with the native method (trace forcing it, or ours declined).
bool Count(int slot, int status) {
    const int o = Origin();
    if (o == 1 && g_forceNative != 0) {
        InterlockedIncrement(&g_native[o][slot]);
        return true;
    }
    InterlockedIncrement(status == FC4_SPAN565_OK ? &g_ours[o][slot] : &g_declined[o][slot]);
    return status != FC4_SPAN565_OK;
}

bool Forced() { return Origin() == 1 && g_forceNative != 0; }

#include "FC4Span565Measure.inc"

void __fastcall Solid(void* self, void* edx, const uint32_t* s, uint32_t n, const fc4_span565_dest* d, uint32_t c) {
    NoteCaller(1, reinterpret_cast<uintptr_t>(_ReturnAddress()));
    if (Forced()) {
        Count(1, -1);
        Measure565(1, s, n, d, nullptr, nullptr, 0,
            [&] { reinterpret_cast<SolidFn>(kExpected[1])(self, edx, s, n, d, c); },
            [&](const fc4_span565_dest* copy, const uint32_t* ss) { return fc4_span565_render_solid(ss, n, copy, c); });
        return;
    }
    const int r = Forced() ? -1 : fc4_span565_render_solid(s, n, d, c);
    if (Count(1, r)) reinterpret_cast<SolidFn>(kExpected[1])(self, edx, s, n, d, c);
}
void __fastcall Gradient(void* self, void* edx, const uint32_t* s, uint32_t n, const fc4_span565_dest* d,
                         const uint32_t* g) {
    NoteCaller(2, reinterpret_cast<uintptr_t>(_ReturnAddress()));
    if (Forced()) {
        Count(2, -1);
        uint32_t record[12];
        std::memcpy(record,g,sizeof(record));
        Measure565(2, s, n, d, nullptr, nullptr, 0,
            [&] { reinterpret_cast<GradFn>(kExpected[2])(self, edx, s, n, d, g); },
            [&](const fc4_span565_dest* copy, const uint32_t* ss) { return fc4_span565_render_gradient(ss, n, copy, record); });
        if (std::memcmp(record,g,sizeof(record))) ++g_contract565;
        return;
    }
    const int r = Forced() ? -1 : fc4_span565_render_gradient(s, n, d, g);
    if (Count(2, r)) reinterpret_cast<GradFn>(kExpected[2])(self, edx, s, n, d, g);
}
void __fastcall Alpha(void* self, void* edx, const uint32_t* s, uint32_t n, const fc4_span565_dest* d, uint32_t c,
                      uint32_t argb) {
    NoteCaller(3, reinterpret_cast<uintptr_t>(_ReturnAddress()));
    if (Forced()) {
        Count(3, -1);
        Measure565(3, s, n, d, nullptr, nullptr, 0,
            [&] { reinterpret_cast<AlphaFn>(kExpected[3])(self, edx, s, n, d, c, argb); },
            [&](const fc4_span565_dest* copy, const uint32_t* ss) { return fc4_span565_render_alpha(ss, n, copy, c, argb); });
        return;
    }
    const int r = Forced() ? -1 : fc4_span565_render_alpha(s, n, d, c, argb);
    if (Count(3, r)) reinterpret_cast<AlphaFn>(kExpected[3])(self, edx, s, n, d, c, argb);
}
void __fastcall AlphaGradient(void* self, void* edx, const uint32_t* s, uint32_t n, const fc4_span565_dest* d,
                              const uint32_t* g) {
    NoteCaller(4, reinterpret_cast<uintptr_t>(_ReturnAddress()));
    if (Forced()) {
        Count(4, -1);
        uint32_t record[12];
        std::memcpy(record,g,sizeof(record));
        Measure565(4, s, n, d, nullptr, nullptr, 0,
            [&] { reinterpret_cast<GradFn>(kExpected[4])(self, edx, s, n, d, g); },
            [&](const fc4_span565_dest* copy, const uint32_t* ss) { return fc4_span565_render_alpha_gradient(ss, n, copy, record); });
        if (std::memcmp(record,g,sizeof(record))) ++g_contract565;
        return;
    }
    const int r = Forced() ? -1 : fc4_span565_render_alpha_gradient(s, n, d, g);
    if (Count(4, r)) reinterpret_cast<GradFn>(kExpected[4])(self, edx, s, n, d, g);
}
void __fastcall Blt(void* self, void* edx, const uint32_t* s, uint32_t n, const fc4_span565_dest* d,
                    const fc4_span565_source* src, const fc4_span565_blt* b, uint32_t kind) {
    NoteCaller(5, reinterpret_cast<uintptr_t>(_ReturnAddress()));
    if (Forced()) {
        Count(5, -1);
        Measure565(5, s, n, d, src, b, kind,
            [&] { reinterpret_cast<BltFn>(kExpected[5])(self, edx, s, n, d, src, b, kind); },
            [&](const fc4_span565_dest* copy, const uint32_t* ss) { return fc4_span565_render_blt(ss, n, copy, src, b, kind); });
        return;
    }
    const int r = Forced() ? -1 : fc4_span565_render_blt(s, n, d, src, b, kind);
    if (Count(5, r)) reinterpret_cast<BltFn>(kExpected[5])(self, edx, s, n, d, src, b, kind);
}

uintptr_t Forwarder(int slot) {
    switch (slot) {
        case 1: return reinterpret_cast<uintptr_t>(&Solid);
        case 2: return reinterpret_cast<uintptr_t>(&Gradient);
        case 3: return reinterpret_cast<uintptr_t>(&Alpha);
        case 4: return reinterpret_cast<uintptr_t>(&AlphaGradient);
        default: return reinterpret_cast<uintptr_t>(&Blt);
    }
}

bool Readable(const void* p, size_t len) {
    MEMORY_BASIC_INFORMATION mbi = {};
    if (VirtualQuery(p, &mbi, sizeof(mbi)) != sizeof(mbi) || mbi.State != MEM_COMMIT) return false;
    if ((mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD)) != 0) return false;
    return static_cast<const char*>(p) + len <= static_cast<const char*>(mbi.BaseAddress) + mbi.RegionSize;
}

bool WriteEntry(uintptr_t* entry, uintptr_t value) {
    DWORD before = 0;
    if (!VirtualProtect(entry, sizeof(*entry), PAGE_READWRITE, &before)) return false;
    *entry = value;
    DWORD ignored = 0;
    VirtualProtect(entry, sizeof(*entry), before, &ignored);
    FlushInstructionCache(GetCurrentProcess(), entry, sizeof(*entry));
    return true;
}

// ---- span565_trace: the constructed-buffer fixture (§1.2) ------------------------------------------

template <class T>
T VSlot(void* obj, int index) {
    return reinterpret_cast<T>((*reinterpret_cast<uintptr_t**>(obj))[index]);
}
uintptr_t VEntry(void* obj, int index) { return (*reinterpret_cast<uintptr_t**>(obj))[index]; }

cIGZBuffer* MakeBuffer(uint32_t w, uint32_t h, uint32_t type, uint32_t bits) {
    cIGZBuffer* b = nullptr;
    if (!GZCOM()->GetClassObject(0xC470D325, GZIID_cIGZBuffer, reinterpret_cast<void**>(&b)) || b == nullptr) {
        return nullptr;
    }
    FC4_ACQUIRED(b, "GetClassObject cGZBuffer");
    if (!b->Init(w, h, static_cast<cGZBufferColorType::eColorType>(type), bits)) {
        FC4_RELEASE_CREATED(b);
        return nullptr;
    }
    return b;
}

#include "FC4Span565Fixture.inc"

}  // namespace

void Span565SetEnabled(bool on) { g_enabled = on; }

void Span565Install() {
    if (!g_enabled || g_installed) return;
    uintptr_t* vt = reinterpret_cast<uintptr_t*>(kVtable);
    if (!Readable(vt, sizeof(uintptr_t) * 6)) {
        Log("span565 bridge: NOT installed (vtable 0x%08lX not readable)", static_cast<unsigned long>(kVtable));
        return;
    }
    for (int i = 0; i < 6; ++i) {
        if (vt[i] != kExpected[i]) {
            Log("span565 bridge: NOT installed (slot %d holds 0x%08lX, expected 0x%08lX)", i,
                static_cast<unsigned long>(vt[i]), static_cast<unsigned long>(kExpected[i]));
            return;
        }
    }
    for (int i = 1; i <= 5; ++i) {
        if (!WriteEntry(&vt[i], Forwarder(i))) {
            Log("span565 bridge: VirtualProtect failed at slot %d; restoring", i);
            for (int j = 1; j < i; ++j) WriteEntry(&vt[j], kExpected[j]);
            return;
        }
    }
    g_installed = true;
    Log("span565 bridge: installed (vtable 0x00AC1844 slots 1-5 -> fc4_span565_*; declined calls go native)");
    if (SettingInt("span565_trace", "FC4_SPAN565_TRACE", 0) != 0) Trace();
}

void Span565Restore() {
    if (!g_installed) return;
    uintptr_t* vt = reinterpret_cast<uintptr_t*>(kVtable);
    int restored = 0;
    for (int i = 1; i <= 5; ++i) {
        if (vt[i] == Forwarder(i) && WriteEntry(&vt[i], kExpected[i])) ++restored;
    }
    g_installed = false;
    for (int o = 0; o < 2; ++o) {
        Log("span565 bridge: %s calls ours %ld/%ld/%ld/%ld/%ld declined %ld/%ld/%ld/%ld/%ld forced-native "
            "%ld/%ld/%ld/%ld/%ld (slots 1-5)", o == 0 ? "nonfixture " : "constructed",
            g_ours[o][1], g_ours[o][2], g_ours[o][3], g_ours[o][4], g_ours[o][5], g_declined[o][1],
            g_declined[o][2], g_declined[o][3], g_declined[o][4], g_declined[o][5], g_native[o][1],
            g_native[o][2], g_native[o][3], g_native[o][4], g_native[o][5]);
    }
    const char* const origins[] = {"constructed_native_context","constructed_direct","natural_native_context","other"};
    for (unsigned i = 0; i < 4; ++i)
        Log("span565 origins: %s %ld/%ld/%ld/%ld/%ld (slots 1-5)",origins[i],
            g_callers565[i][1],g_callers565[i][2],g_callers565[i][3],g_callers565[i][4],g_callers565[i][5]);
    Log("span565 bridge: removed (%d of 5 entries restored)", restored);
}

}  // namespace fc4
