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

// The cSpanRender8888 native bridge; see FC4SpanRender8888.h.
// Behaviour source: docs/specs/span-render-8888.md §1.1 (boundary), §2-§9.

#include "FC4SpanRender8888.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Log.h"
#include "FC4SpanContext.h"
#include "FC4RefAudit.h"
#include "FC4Subsystems.h"
#include "compat/FC4Compat.h"
#include "fc4.h"


#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <intrin.h>

#include <atomic>
#include <cstdint>
#include <cstring>
#include <vector>

namespace fc4 {
namespace {

// §1.1: MSVC thiscall, `this` in ECX, stack arguments popped by the callee,
// no return value. __fastcall with a dummy EDX argument has the same stack
// contract (ECX, EDX, then the stack arguments, callee pops them).
using SolidFn = void(__fastcall*)(void* self, void* edx, const uint32_t* spans, uint32_t count,
                                  const uint32_t* buffer, uint32_t colour);  // pop 16
using GradFn = void(__fastcall*)(void* self, void* edx, const uint32_t* spans, uint32_t count,
                                 const uint32_t* buffer, const int32_t* grad);  // pop 16
using AlphaFn = void(__fastcall*)(void* self, void* edx, const uint32_t* spans, uint32_t count,
                                  const uint32_t* buffer, uint32_t colour, uint32_t alpha);  // pop 20
using BltFn = void(__fastcall*)(void* self, void* edx, const uint32_t* spans, uint32_t count,
                                const uint32_t* rectA, const uint32_t* rectB, const uint8_t* src,
                                uint32_t mode);  // pop 24

enum Slot : unsigned { kSolid = 0, kGradient, kAlpha, kAlphaGradient, kBlt, kSlots };  // vtable slots 1-5
const char* const kSlotName[kSlots] = {"RenderSolid", "RenderGradient", "RenderAlpha", "RenderAlphaGradient",
                                       "RenderBlt"};

struct State {
    bool installed = false;
    bool shadow = false;
    void** vtable = nullptr;  // entry 0
    uintptr_t original[kSlots] = {};
    std::atomic<uint64_t> calls[kSlots];
    std::atomic<uint64_t> ours[kSlots];       // rendered by the core
    std::atomic<uint64_t> fallback[kSlots];   // core rejected: the original rendered
    std::atomic<uint64_t> compared[kSlots];   // shadow: compared calls
    std::atomic<uint64_t> differing[kSlots];  // shadow: calls with a differing byte
    std::atomic<uint64_t> diffPixels[kSlots];
};
State g;

// The bytes a span list touches on a {base, stride} target (§2): from pixel
// (0, 0) or the lowest span byte, whichever is lower, to the highest span
// byte. False if a span is wider than one 16-bit row or the region exceeds
// 64 MB (then the call is not bridged: the original runs).
struct Region {
    uint8_t* lo = nullptr;
    size_t len = 0;
    size_t origin = 0;  // byte offset of pixel (0, 0) in the region
};

bool RegionOf(const uint32_t* spans, uint32_t count, const uint32_t* desc, Region* r) {
    if (spans == nullptr || desc == nullptr || count == 0) return false;
    const int64_t stride = static_cast<int32_t>(desc[1]);
    int64_t lo = 0, hi = 0;
    for (uint32_t i = 0; i < count; ++i) {
        const uint32_t start = spans[2 * i], width = spans[2 * i + 1] - start;
        if (width > 0x10000u) return false;
        const int64_t off = static_cast<int64_t>(start >> 16) * stride + 4 * static_cast<int64_t>(start & 0xFFFFu);
        if (off < lo) lo = off;
        if (off + 4 * static_cast<int64_t>(width) > hi) hi = off + 4 * static_cast<int64_t>(width);
    }
    if (hi - lo > (int64_t(64) << 20) || hi <= lo) return false;
    r->lo = reinterpret_cast<uint8_t*>(desc[0]) + lo;
    r->len = static_cast<size_t>(hi - lo);
    r->origin = static_cast<size_t>(-lo);
    return true;
}

// The source image of RenderBlt (`rectB`, §7): rows 0..height-1 from base
// with a positive stride. False otherwise (the original runs).
bool SourceOf(const uint32_t* rectB, const uint8_t** p, size_t* len) {
    const int32_t stride = static_cast<int32_t>(rectB[1]);
    const uint32_t height = rectB[3];
    if (rectB[0] == 0 || stride <= 0 || height == 0 || uint64_t(stride) * height > (uint64_t(64) << 20)) return false;
    *p = reinterpret_cast<const uint8_t*>(rectB[0]);
    *len = static_cast<size_t>(stride) * height;
    return true;
}

// Ours on `r` (a region of the real target, or a copy of it).
int32_t Ours(Slot s, const uint32_t* spans, uint32_t count, uint8_t* dst, const Region& r, int32_t stride,
             uint32_t a, uint32_t b, const uint32_t* rectB, const uint8_t* src, uint32_t mode) {
    switch (s) {
        case kSolid: return fc4_span8888_solid(spans, count, dst, r.len, r.origin, stride, a);
        case kAlpha: return fc4_span8888_alpha(spans, count, dst, r.len, r.origin, stride, a, b);
        case kGradient:
        case kAlphaGradient:
            return fc4_span8888_gradient(spans, count, dst, r.len, r.origin, stride, reinterpret_cast<const uint32_t*>(a),
                                         s == kAlphaGradient);
        case kBlt: {
            const uint8_t* sp = nullptr;
            size_t sl = 0;
            if (!SourceOf(rectB, &sp, &sl)) return 1;
            return fc4_span8888_blt(spans, count, dst, r.len, r.origin, stride, sp, sl, 0,
                                    static_cast<int32_t>(rectB[1]), rectB[2], rectB[3], src, mode);
        }
        default: return 1;
    }
}

void Original(Slot s, void* self, const uint32_t* spans, uint32_t count, const uint32_t* buffer, uint32_t a,
              uint32_t b, const uint32_t* rectB, const uint8_t* src, uint32_t mode) {
    const uintptr_t f = g.original[s];
    switch (s) {
        case kSolid: reinterpret_cast<SolidFn>(f)(self, nullptr, spans, count, buffer, a); break;
        case kAlpha: reinterpret_cast<AlphaFn>(f)(self, nullptr, spans, count, buffer, a, b); break;
        case kGradient:
        case kAlphaGradient:
            reinterpret_cast<GradFn>(f)(self, nullptr, spans, count, buffer, reinterpret_cast<const int32_t*>(a));
            break;
        case kBlt: reinterpret_cast<BltFn>(f)(self, nullptr, spans, count, buffer, rectB, src, mode); break;
        default: break;
    }
}

// One forwarded call. `buffer` is the target descriptor (slots 1-4) or
// rectA (slot 5); both start {base, stride} (§2, §7).
void Bridge(Slot s, uintptr_t ret, void* self, const uint32_t* spans, uint32_t count, const uint32_t* buffer,
            uint32_t a, uint32_t b, const uint32_t* rectB, const uint8_t* src, uint32_t mode) {
    g.calls[s].fetch_add(1, std::memory_order_relaxed);
    SpanNoteCall(kSpanR8888, s + 1, ret, false);  // this module's fixture never calls through the vtable
    Region r;
    const bool bridged = RegionOf(spans, count, buffer, &r) && (s != kBlt || (rectB != nullptr && src != nullptr));
    const int32_t stride = bridged ? static_cast<int32_t>(buffer[1]) : 0;
    if (!bridged) {
        g.fallback[s].fetch_add(1, std::memory_order_relaxed);
        SpanNoteRefused(kSpanR8888);
        Original(s, self, spans, count, buffer, a, b, rectB, src, mode);
        return;
    }
    // The context fixture (FC4SpanContext.cpp) needs ours' pixels even under span8888_shadow.
    if (!g.shadow || SpanContextFixtureThread()) {
        if (Ours(s, spans, count, r.lo, r, stride, a, b, rectB, src, mode) == 0) {
            g.ours[s].fetch_add(1, std::memory_order_relaxed);
        } else {
            g.fallback[s].fetch_add(1, std::memory_order_relaxed);
            SpanNoteRefused(kSpanR8888);
            Original(s, self, spans, count, buffer, a, b, rectB, src, mode);
        }
        return;
    }
    std::vector<uint8_t> copy(r.lo, r.lo + r.len);
    Original(s, self, spans, count, buffer, a, b, rectB, src, mode);
    if (Ours(s, spans, count, copy.data(), r, stride, a, b, rectB, src, mode) != 0) {
        g.fallback[s].fetch_add(1, std::memory_order_relaxed);
        return;
    }
    g.compared[s].fetch_add(1, std::memory_order_relaxed);
    uint64_t px = 0;
    for (size_t i = 0; i + 3 < r.len; i += 4) px += std::memcmp(&copy[i], r.lo + i, 4) != 0;
    if (px != 0) {
        g.diffPixels[s].fetch_add(px, std::memory_order_relaxed);
        if (g.differing[s].fetch_add(1, std::memory_order_relaxed) < 8)
            Log("span8888: DIFF %s count=%u: %llu pixels differ (original vs ours)", kSlotName[s], count,
                (unsigned long long)px);
    }
}

void __fastcall FwdSolid(void* self, void*, const uint32_t* spans, uint32_t count, const uint32_t* buffer,
                         uint32_t colour) {
    Bridge(kSolid, reinterpret_cast<uintptr_t>(_ReturnAddress()), self, spans, count, buffer, colour, 0, nullptr, nullptr, 0);
}
void __fastcall FwdGradient(void* self, void*, const uint32_t* spans, uint32_t count, const uint32_t* buffer,
                            const int32_t* grad) {
    Bridge(kGradient, reinterpret_cast<uintptr_t>(_ReturnAddress()), self, spans, count, buffer, static_cast<uint32_t>(reinterpret_cast<uintptr_t>(grad)), 0,
           nullptr, nullptr, 0);
}
void __fastcall FwdAlpha(void* self, void*, const uint32_t* spans, uint32_t count, const uint32_t* buffer,
                         uint32_t colour, uint32_t alpha) {
    Bridge(kAlpha, reinterpret_cast<uintptr_t>(_ReturnAddress()), self, spans, count, buffer, colour, alpha, nullptr, nullptr, 0);
}
void __fastcall FwdAlphaGradient(void* self, void*, const uint32_t* spans, uint32_t count, const uint32_t* buffer,
                                 const int32_t* grad) {
    Bridge(kAlphaGradient, reinterpret_cast<uintptr_t>(_ReturnAddress()), self, spans, count, buffer, static_cast<uint32_t>(reinterpret_cast<uintptr_t>(grad)), 0,
           nullptr, nullptr, 0);
}
void __fastcall FwdBlt(void* self, void*, const uint32_t* spans, uint32_t count, const uint32_t* rectA,
                       const uint32_t* rectB, const uint8_t* src, uint32_t mode) {
    Bridge(kBlt, reinterpret_cast<uintptr_t>(_ReturnAddress()), self, spans, count, rectA, 0, 0, rectB, src, mode);
}

void* Forwarder(unsigned s) {
    switch (s) {
        case kSolid: return reinterpret_cast<void*>(&FwdSolid);
        case kGradient: return reinterpret_cast<void*>(&FwdGradient);
        case kAlpha: return reinterpret_cast<void*>(&FwdAlpha);
        case kAlphaGradient: return reinterpret_cast<void*>(&FwdAlphaGradient);
        default: return reinterpret_cast<void*>(&FwdBlt);
    }
}

bool Enabled() {
    size_t n = 0;
    const Subsystem* s = Subsystems(&n);
    for (size_t i = 0; i < n; ++i)
        if (std::strcmp(s[i].key, "span8888") == 0) return SubsystemEnabled(s[i]);
    return false;
}

bool WriteEntries(void** first, void* const* values) {
    DWORD before = 0;
    if (!VirtualProtect(first, kSlots * sizeof(void*), PAGE_READWRITE, &before)) return false;
    for (unsigned i = 0; i < kSlots; ++i) first[i] = values[i];
    DWORD ignored = 0;
    VirtualProtect(first, kSlots * sizeof(void*), before, &ignored);
    FlushInstructionCache(GetCurrentProcess(), first, kSlots * sizeof(void*));
    return true;
}

// ---- the constructed fixture (NOT a game path) ---------------------------------------------------

constexpr int kW = 32, kH = 8, kStride = 4 * kW + 12;  // a padded stride (3 pad pixels)
constexpr int kSrcW = 64, kSrcH = 64, kSrcStride = 4 * kSrcW;
const uint32_t kSpans[] = {
    // (y << 16) | x, end = start + width (§2); unaligned starts exercise the fill helper (§3)
    0x00000000, 0x00000020, 0x00010001, 0x00010008, 0x00020003, 0x00020004,
    0x00030002, 0x00030016, 0x00050005, 0x00050012, 0x00070000, 0x0007001F,
};
constexpr uint32_t kSpanCount = 6;

struct Fixture {
    uint32_t seed = 0x13572468u;
    uint32_t Next() {
        seed = seed * 1664525u + 1013904223u;
        return seed;
    }
    uint8_t dstInit[kH * kStride];
    uint8_t src[kSrcH * kSrcStride];
    uint64_t cases = 0, pixels = 0, diffs = 0, rejected = 0, faults = 0;
    uint64_t slotCases[kSlots] = {}, slotDiffs[kSlots] = {};
};

struct Call {
    Slot s;
    uint32_t a, b, mode;
    const uint32_t* rectB;
    const uint8_t* src;
    uint32_t* desc;
};

void CallOriginalGuarded(void* ctx) {
    Call* c = static_cast<Call*>(ctx);
    Original(c->s, nullptr, kSpans, kSpanCount, c->desc, c->a, c->b, c->rectB, c->src, c->mode);
}

void RunCase(Fixture& f, Slot s, uint32_t a, uint32_t b, const uint32_t* rectB, const uint8_t* src, uint32_t mode) {
    uint8_t native[sizeof(f.dstInit)], ours[sizeof(f.dstInit)];
    std::memcpy(native, f.dstInit, sizeof(native));
    std::memcpy(ours, f.dstInit, sizeof(ours));
    uint32_t desc[6] = {static_cast<uint32_t>(reinterpret_cast<uintptr_t>(native)), uint32_t(kStride), kW, kH, 0, 0};
    ++f.cases;
    ++f.slotCases[s];
    Region r;
    r.lo = ours;
    r.len = sizeof(ours);
    r.origin = 0;
    if (Ours(s, kSpans, kSpanCount, ours, r, kStride, a, b, rectB, src, mode) != 0) {
        ++f.rejected;
        return;
    }
    Call c{s, a, b, mode, rectB, src, desc};
    if (!SehGuard("span8888 fixture original", &CallOriginalGuarded, &c)) {
        ++f.faults;
        return;
    }
    uint64_t px = 0;
    int first = -1;
    for (int i = 0; i < int(sizeof(native)); i += 4) {
        if (std::memcmp(native + i, ours + i, 4) != 0) {
            ++px;
            if (first < 0) first = i;
        }
    }
    f.pixels += sizeof(native) / 4;
    if (px != 0) {
        ++f.diffs;
        ++f.slotDiffs[s];
        if (f.diffs <= 12) {
            uint32_t n = 0, o = 0;
            std::memcpy(&n, native + first, 4);
            std::memcpy(&o, ours + first, 4);
            Log("span8888 fixture [constructed direct]: DIFF %s a=0x%08X b=0x%08X mode=%u: %llu pixels, first at byte %d "
                "(x=%d y=%d) original 0x%08X ours 0x%08X",
                kSlotName[s], a, b, mode, (unsigned long long)px, first, (first % kStride) / 4, first / kStride, n, o);
        }
    }
}

void RunFixture() {
    static Fixture f;  // ~17 KB
    for (auto& v : f.dstInit) v = uint8_t(f.Next() >> 24);
    for (auto& v : f.src) v = uint8_t(f.Next() >> 24);
    // Slot 1 (§3): 8888 colours, alpha nibble included, high word ignored.
    for (uint32_t c : {0x1234ABCDu, 0x0000F00Fu, 0xFFFF0000u}) RunCase(f, kSolid, c, 0, nullptr, nullptr, 0);
    // Slot 3 (§4): alpha byte in bits 24-31, weights 0..16.
    for (uint32_t al : {0x00000000u, 0x07FFFFFFu, 0x08000000u, 0x80000000u, 0x3A000000u, 0xF8000000u, 0xFFFFFFFFu})
        for (uint32_t c : {0x0F84u, 0x7C3Eu, 0xFFFFu}) RunCase(f, kAlpha, c, al, nullptr, nullptr, 0);
    // Slots 2 and 4 (§5, §6): 8.16 ramps B, G, R, A; in range, and one where R leaves 0..255 (U2).
    static const int32_t grads[][12] = {
        {0x00100000, 0x00800000, 0x00200000, 0x00000000, 0x00040000, -0x00030000, 0x00060000, 0x00080000,
         0x00011000, 0x00020000, 0x00030000, 0x00100000},
        {0x00FF8000, 0x00000000, 0x00F00000, 0x00FF0000, -0x00071234, 0x00051234, 0x00012345, -0x00081000,
         0x00000000, 0x00010000, -0x00020000, -0x00010000},
        {0x00000000, 0x00000000, 0x00F80000, 0x00400000, 0x00008000, 0x00010000, 0x00010000, 0x00020000, 0, 0, 0,
         0},
    };
    for (const auto& gr : grads) {
        const uint32_t p = static_cast<uint32_t>(reinterpret_cast<uintptr_t>(gr));
        RunCase(f, kGradient, p, 0, nullptr, nullptr, 0);
        RunCase(f, kAlphaGradient, p, 0, nullptr, nullptr, 0);
    }
    // Slot 5 (§7, §8): every row (format 0..6 x key 0/1) and column 0..11.
    uint32_t rectB[6] = {static_cast<uint32_t>(reinterpret_cast<uintptr_t>(f.src)), uint32_t(kSrcStride), 16, 16, 0, 0};
    for (uint32_t format = 0; format < 7; ++format) {
        for (uint32_t keyed = 0; keyed < 2; ++keyed) {
            uint8_t desc[32] = {};
            const uint32_t d[7] = {0x00028000, 0x00010000, 0x00008000, 0x0000C000, format, 0x80C0A040u, 0};
            std::memcpy(desc, d, sizeof(d));
            // The key: the texel at (2, 1), 4 bytes read, so 16-bit and 8-bit formats match it too.
            std::memcpy(desc + 0x18, f.src + 1 * kSrcStride + 2 * 4, 4);
            desc[0x1C] = uint8_t(keyed);
            for (uint32_t mode = 0; mode < 12; ++mode) RunCase(f, kBlt, 0, 0, rectB, desc, mode);
        }
    }
    for (unsigned s = 0; s < kSlots; ++s)
        Log("span8888 fixture [constructed direct, not a game path]: slot %u %s cases=%llu differing=%llu", s + 1,
            kSlotName[s], (unsigned long long)f.slotCases[s], (unsigned long long)f.slotDiffs[s]);
    Log("span8888 fixture [constructed direct, not a game path]: TOTAL cases=%llu pixels=%llu differing_cases=%llu "
        "rejected_by_core=%llu faults=%llu -> %s",
        (unsigned long long)f.cases, (unsigned long long)f.pixels, (unsigned long long)f.diffs,
        (unsigned long long)f.rejected, (unsigned long long)f.faults,
        f.diffs == 0 && f.faults == 0 && f.rejected == 0 ? "EXACT" : "NOT exact");
}

}  // namespace

void SpanRender8888Start() {
    if (!Enabled()) return;
    void** vt = nullptr;
    uintptr_t fn[kSlots] = {};
    if (!compat::SpanRender8888Entries(&vt, fn)) {
        Log("span8888: bridge NOT installed (vtable unavailable)");
        return;
    }
    for (unsigned i = 0; i < kSlots; ++i) {
        const uintptr_t found = reinterpret_cast<uintptr_t>(vt[1 + i]);
        if (found != fn[i]) {
            Log("span8888: bridge NOT installed: entry %u holds 0x%08lX, expected 0x%08lX", i + 1,
                static_cast<unsigned long>(found), static_cast<unsigned long>(fn[i]));
            return;
        }
        g.original[i] = found;
    }
    g.vtable = vt;
    g.shadow = SettingInt("span8888_shadow", "FC4_SPAN8888_SHADOW", 0) != 0;
    RunFixture();
    void* fwd[kSlots];
    for (unsigned i = 0; i < kSlots; ++i) fwd[i] = Forwarder(i);
    if (!WriteEntries(vt + 1, fwd)) {
        Log("span8888: bridge NOT installed: VirtualProtect failed (error %lu)", GetLastError());
        return;
    }
    g.installed = true;
    Log("span8888: bridge installed on vtable %p slots 1-5 (%s)", static_cast<void*>(vt),
        g.shadow ? "shadow: the original renders, ours compared" : "ours renders; the original on core rejection");
}

bool SpanRender8888UseOriginal(bool original) {
    if (!g.installed) return false;
    void* v[kSlots];
    for (unsigned i = 0; i < kSlots; ++i) v[i] = original ? reinterpret_cast<void*>(g.original[i]) : Forwarder(i);
    return WriteEntries(g.vtable + 1, v);
}

void SpanRender8888Stop() {
    if (!g.installed) return;
    g.installed = false;
    void* orig[kSlots];
    for (unsigned i = 0; i < kSlots; ++i) {
        if (g.vtable[1 + i] != Forwarder(i))
            Log("span8888: WARNING entry %u no longer holds our forwarder; restoring the original anyway", i + 1);
        orig[i] = reinterpret_cast<void*>(g.original[i]);
    }
    const bool ok = WriteEntries(g.vtable + 1, orig);
    for (unsigned s = 0; s < kSlots; ++s)
        Log("span8888: %s calls=%llu ours=%llu original_fallback=%llu shadow_compared=%llu shadow_differing=%llu "
            "diff_pixels=%llu",
            kSlotName[s], (unsigned long long)g.calls[s].load(), (unsigned long long)g.ours[s].load(),
            (unsigned long long)g.fallback[s].load(), (unsigned long long)g.compared[s].load(),
            (unsigned long long)g.differing[s].load(), (unsigned long long)g.diffPixels[s].load());
    Log("span8888: bridge %s", ok ? "removed (original entries back)" : "NOT removed: VirtualProtect failed");
}

}  // namespace fc4
