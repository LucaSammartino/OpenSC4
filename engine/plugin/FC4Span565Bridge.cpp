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

void __fastcall Solid(void* self, void* edx, const uint32_t* s, uint32_t n, const fc4_span565_dest* d, uint32_t c) {
    const int r = Forced() ? -1 : fc4_span565_render_solid(s, n, d, c);
    if (Count(1, r)) reinterpret_cast<SolidFn>(kExpected[1])(self, edx, s, n, d, c);
}
void __fastcall Gradient(void* self, void* edx, const uint32_t* s, uint32_t n, const fc4_span565_dest* d,
                         const uint32_t* g) {
    const int r = Forced() ? -1 : fc4_span565_render_gradient(s, n, d, g);
    if (Count(2, r)) reinterpret_cast<GradFn>(kExpected[2])(self, edx, s, n, d, g);
}
void __fastcall Alpha(void* self, void* edx, const uint32_t* s, uint32_t n, const fc4_span565_dest* d, uint32_t c,
                      uint32_t argb) {
    const int r = Forced() ? -1 : fc4_span565_render_alpha(s, n, d, c, argb);
    if (Count(3, r)) reinterpret_cast<AlphaFn>(kExpected[3])(self, edx, s, n, d, c, argb);
}
void __fastcall AlphaGradient(void* self, void* edx, const uint32_t* s, uint32_t n, const fc4_span565_dest* d,
                              const uint32_t* g) {
    const int r = Forced() ? -1 : fc4_span565_render_alpha_gradient(s, n, d, g);
    if (Count(4, r)) reinterpret_cast<GradFn>(kExpected[4])(self, edx, s, n, d, g);
}
void __fastcall Blt(void* self, void* edx, const uint32_t* s, uint32_t n, const fc4_span565_dest* d,
                    const fc4_span565_source* src, const fc4_span565_blt* b, uint32_t kind) {
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

struct Case {
    const char* name;
    int slot;            // the renderer slot DrawPreclippedSpanList should pick (§6.1)
    uint8_t alpha;       // ctx+0x41
    uint8_t gradient;    // ctx+0x74
    uint32_t colour;     // ctx+0xC4
    uint32_t argb;       // ctx+0xB4
};

int g_traceFail = 0;

void Compare(const char* what, int slot, const std::vector<uint8_t>& a, const std::vector<uint8_t>& b, LONG oursBefore) {
    size_t diff = 0;
    for (size_t i = 0; i < a.size(); ++i) diff += a[i] != b[i] ? 1 : 0;
    const LONG ours = g_ours[1][slot] - oursBefore;
    if (diff != 0 || ours != 1) ++g_traceFail;
    Log("span565 trace: %-24s slot %d %-19s bytes=%u diff=%u ours_calls=%ld (constructed buffer)", what, slot,
        kSlotName[slot], static_cast<unsigned>(a.size()), static_cast<unsigned>(diff), static_cast<long>(ours));
}

void Trace() {
    g_traceThread = GetCurrentThreadId();
    cIGZBuffer* buf = MakeBuffer(5, 3, 5, 16);
    if (buf == nullptr) {
        Log("span565 trace: FAIL cGZBuffer create / Init(5, 3, 5, 16)");
        g_traceThread = 0;
        return;
    }
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZUnknown* ds = nullptr;
    // GZServPtrs.h cIGZDrawSystemPtr: iid 1261347763, service id 3945702347.
    if (fw == nullptr || !fw->GetSystemService(3945702347u, 1261347763u, reinterpret_cast<void**>(&ds)) || !ds) {
        Log("span565 trace: FAIL no draw system service");
        FC4_RELEASE_CREATED(buf);
        g_traceThread = 0;
        return;
    }
    FC4_ACQUIRED(ds, "GetSystemService draw system");
    void* ctx = nullptr;
    // §1.2: draw system slot 4 = 0x0082BAB0 (buffer, out), AddRefs into *out. Its
    // convention is not published: thiscall, callee pops 8, bool result (literal reading).
    using MakeCtxFn = bool(__fastcall*)(void*, void*, cIGZUnknown*, void**);
    if (VEntry(ds, 4) != 0x0082BAB0) {
        Log("span565 trace: FAIL draw system slot 4 is 0x%08lX, not 0x0082BAB0",
            static_cast<unsigned long>(VEntry(ds, 4)));
    } else {
        VSlot<MakeCtxFn>(ds, 4)(ds, nullptr, buf, &ctx);
        if (ctx != nullptr) FC4_ACQUIRED(static_cast<cIGZUnknown*>(ctx), "draw system slot 4 context");
    }
    FC4_RELEASE(ds);
    if (ctx == nullptr || *reinterpret_cast<uintptr_t*>(ctx) != 0x00AC17D0 || VEntry(ctx, 3) != 0x00830710 ||
        VEntry(ctx, 4) != 0x0082FF20 || VEntry(ctx, 22) != 0x008302E0) {
        Log("span565 trace: FAIL context %p is not a RenderToBuffer context (vtable 0x00AC17D0)", ctx);
        if (ctx != nullptr) FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(ctx));
        FC4_RELEASE_CREATED(buf);
        g_traceThread = 0;
        return;
    }
    using InitFn = uint8_t(__fastcall*)(void*, void*, void*);
    using ShutdownFn = void(__fastcall*)(void*, void*);  // §1.1 Init step 1 calls it with no argument
    using DrawFn = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t);
    const uint8_t init = VSlot<InitFn>(ctx, 3)(ctx, nullptr, buf);
    char* c = static_cast<char*>(ctx);
    void* rend = *reinterpret_cast<void**>(c + 0xB0);
    const uintptr_t rendVt = rend != nullptr ? *reinterpret_cast<uintptr_t*>(rend) : 0;
    uint8_t* px = static_cast<uint8_t*>(buf->GetColorSurfaceBits());
    const uint32_t stride = buf->GetColorSurfaceStride();
    Log("span565 trace: constructed buffer %p type 5/16 5x3 stride %u, context %p Init -> %u, renderer vtable "
        "0x%08lX (%s)", static_cast<void*>(buf), stride, ctx, init, static_cast<unsigned long>(rendVt),
        rendVt == kVtable ? "565" : "NOT 565");
    if (init == 1 && rendVt == kVtable && px != nullptr && stride == 12) {
        const size_t bytes = stride * 3;
        auto pattern = [&] {
            for (size_t i = 0; i < bytes; ++i) px[i] = static_cast<uint8_t>(i * 37 + 0x5B);
        };
        // Three spans: full row 0, x 1..3 on row 1, x 2..4 on row 2 (row 2 overlaps
        // nothing; the padding bytes 10, 11 of each row must stay).
        const uint32_t spans[6] = {0x00000000, 0x00000005, 0x00010001, 0x00010004, 0x00020002, 0x00020005};
        const uint32_t grad[12] = {0x00FF8000, 0x00408000, 0x00108000, 0x00808000, 0x00012345, 0x00054321,
                                   0x000ABCDE, 0x00080000, 0x00003001, 0x00070007, 0x00100000, 0xFFFC0000};
        const Case cases[4] = {{"solid", 1, 0, 0, 0x1234F81F, 0x80FF0000},
                               {"alpha", 3, 1, 0, 0x0000B5A6, 0x7F10C080},
                               {"gradient", 2, 0, 1, 0, 0},
                               {"alpha-gradient", 4, 1, 1, 0, 0}};
        for (const Case& k : cases) {
            c[0x41] = static_cast<char>(k.alpha);
            c[0x74] = static_cast<char>(k.gradient);
            std::memcpy(c + 0xC4, &k.colour, 4);
            std::memcpy(c + 0xB4, &k.argb, 4);
            std::memcpy(c + 0x44, grad, sizeof(grad));
            std::vector<uint8_t> native(bytes), ours(bytes);
            pattern();
            InterlockedExchange(&g_forceNative, 1);
            VSlot<DrawFn>(ctx, 22)(ctx, nullptr, spans, 3);
            InterlockedExchange(&g_forceNative, 0);
            std::memcpy(native.data(), px, bytes);
            pattern();
            const LONG before = g_ours[1][k.slot];
            VSlot<DrawFn>(ctx, 22)(ctx, nullptr, spans, 3);
            std::memcpy(ours.data(), px, bytes);
            Compare(k.name, k.slot, native, ours, before);
        }
        // RenderBlt by direct slot calls (no texture setter published, see the question file).
        cIGZBuffer* tex = MakeBuffer(4, 3, 9, 32);
        uint8_t* tp = tex != nullptr ? static_cast<uint8_t*>(tex->GetColorSurfaceBits()) : nullptr;
        if (tp != nullptr) {
            const uint32_t ts = tex->GetColorSurfaceStride();
            for (uint32_t i = 0; i < ts * 3; ++i) tp[i] = static_cast<uint8_t>(i * 91 + 0x17);
            const fc4_span565_dest d = {px, static_cast<int32_t>(stride), 5, 3, 5, 2};
            const fc4_span565_source s = {tp, static_cast<int32_t>(ts), 4, 3, {9, 4}};
            const uint32_t kinds[8] = {0, 1, 2, 3, 4, 6, 8, 11};
            for (uint32_t kind : kinds) {
                for (uint8_t keyed = 0; keyed < 2; ++keyed) {
                    fc4_span565_blt b = {0x8000, 0x4000, 0xA000, 0xC000, 4, 0x80C04020, 0, keyed, {0, 0, 0}};
                    std::memcpy(&b.key, tp + 4, 3);  // texel (1, 0), 24 bits: keyed rows skip it
                    std::vector<uint8_t> native(bytes), ours(bytes);
                    pattern();
                    InterlockedExchange(&g_forceNative, 1);
                    Blt(rend, nullptr, spans, 3, &d, &s, &b, kind);
                    InterlockedExchange(&g_forceNative, 0);
                    std::memcpy(native.data(), px, bytes);
                    pattern();
                    const LONG before = g_ours[1][5];
                    VSlot<BltFn>(rend, 5)(rend, nullptr, spans, 3, &d, &s, &b, kind);
                    std::memcpy(ours.data(), px, bytes);
                    char what[32];
                    std::snprintf(what, sizeof(what), "blt kind %u key %u", kind, keyed);
                    Compare(what, 5, native, ours, before);
                }
            }
        } else {
            ++g_traceFail;
            Log("span565 trace: FAIL texture buffer Init(4, 3, 9, 32)");
        }
        if (tex != nullptr) FC4_RELEASE_CREATED(tex);
    } else {
        ++g_traceFail;
    }
    VSlot<ShutdownFn>(ctx, 4)(ctx, nullptr);  // §1.2: undoes the destination lock
    FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(ctx));
    FC4_RELEASE_CREATED(buf);
    g_traceThread = 0;
    Log("span565 trace: %s (%d failing case%s; constructed buffer, not a shipped path: SR565-5)",
        g_traceFail == 0 ? "EXACT" : "DIFFERS", g_traceFail, g_traceFail == 1 ? "" : "s");
}

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
            "%ld/%ld/%ld/%ld/%ld (slots 1-5)", o == 0 ? "shipped    " : "constructed",
            g_ours[o][1], g_ours[o][2], g_ours[o][3], g_ours[o][4], g_ours[o][5], g_declined[o][1],
            g_declined[o][2], g_declined[o][3], g_declined[o][4], g_declined[o][5], g_native[o][1],
            g_native[o][2], g_native[o][3], g_native[o][4], g_native[o][5]);
    }
    Log("span565 bridge: removed (%d of 5 entries restored)", restored);
}

}  // namespace fc4
