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
const char* const kSlotAddress[kSlots] = {"0x00854E10", "0x00854E60", "0x00854F70", "0x00855040", "0x008551D0"};

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
    std::atomic<uint64_t> fixtureCalls[kSlots], fixtureOurs[kSlots], fixtureFallback[kSlots];
    std::atomic<uint64_t> directCalls[kSlots], directCompared[kSlots], directDiffering[kSlots];
    std::atomic<uint64_t> legacyShort{0}, legacyOverlap{0};
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

constexpr int64_t kWindowLimit = int64_t(64) << 20;

// Validate data pages only; never probe code bytes. A descriptor base may be
// pre-offset outside an allocation (§9.1); only the addresses actually read
// need to exist. VirtualQuery is a safety check, not a source of behaviour.
bool DataWindow(uint64_t address, uint64_t length, bool writable = false) {
    if (address == 0 || length == 0 || address + length > (uint64_t(1) << 32)) return false;
    const uint64_t end = address + length;
    while (address < end) {
        MEMORY_BASIC_INFORMATION m = {};
        if (!VirtualQuery(reinterpret_cast<const void*>(static_cast<uintptr_t>(address)), &m, sizeof(m)) ||
            m.State != MEM_COMMIT || (m.Protect & (PAGE_GUARD | PAGE_NOACCESS))) return false;
        const DWORD p = m.Protect & 0xFF;
        if (p != PAGE_READWRITE && p != PAGE_WRITECOPY && (writable || p != PAGE_READONLY)) return false;
        const uint64_t next = reinterpret_cast<uintptr_t>(m.BaseAddress) + uint64_t(m.RegionSize);
        if (next <= address) return false;
        address = next;
    }
    return true;
}

// §7: width/height are read ONLY by tiled kernels. In particular neither
// stride*height nor width*bpp bounds a direct blit's pre-offset source.
// Enumerate precisely the source reads defined by §9.1, without touching it.
template<class Visit>
bool SourceReads(const uint32_t* spans, uint32_t count, const uint32_t* image, const uint8_t* desc,
                 uint32_t mode, Visit visit) {
    uint32_t d[8];
    std::memcpy(d, desc, sizeof(d));
    if (d[4] > 6 || desc[28] > 1 || mode > 11) return false;
    const uint32_t bpp = d[4] == 6 ? 1 : d[4] >= 4 ? 4 : 2;
    const int64_t stride = static_cast<int32_t>(image[1]);
    if (mode >= 8 && (image[2] == 0 || image[3] == 0)) return false;
    uint64_t bytes = 0;
    for (uint32_t i = 0; i < count; ++i) {
        const uint32_t start = spans[2 * i], width = spans[2 * i + 1] - start;
        const uint32_t x = start & 0xFFFFu, y = start >> 16;
        bytes += uint64_t(width) * bpp;
        // A bridge resource limit, not a native bounds rule. Keep fallback.
        if (bytes > uint64_t(kWindowLimit)) return false;
        if (width == 0) continue;
        if (mode < 4) {
            if (!visit(int64_t(y) * stride + int64_t(x) * bpp, size_t(width) * bpp)) return false;
        } else {
            uint32_t u = mode < 8 ? d[2] * x + d[0] : (x + d[0]) % image[2];
            const uint32_t v = mode < 8 ? (d[3] * y + d[1]) >> 16 : (y + d[1]) % image[3];
            for (uint32_t j = 0; j < width; ++j) {
                const uint32_t tx = mode < 8 ? u >> 16 : u;
                if (!visit(int64_t(v) * stride + int64_t(tx) * bpp, bpp)) return false;
                if (mode < 8) u += d[2];
                else if (++u == image[2]) u = 0;
            }
        }
    }
    return true;
}

struct SourceCapture {
    std::vector<uint8_t> bytes;
    size_t origin = 0;
    int64_t first = 0, end = 0;
    bool legacyShort = false, legacyOverlap = false;
    const char* refusal = "invalid-source-geometry";
};

bool CaptureSource(const uint32_t* spans, uint32_t count, const Region& dst, int32_t dstStride,
                   const uint32_t* image, const uint8_t* desc, uint32_t mode, SourceCapture* out) {
    int64_t lo = 0, hi = 0;
    bool any = false;
    const uint64_t dstBase = reinterpret_cast<uintptr_t>(dst.lo) + dst.origin;
    const bool valid = SourceReads(spans, count, image, desc, mode, [&](int64_t off, size_t n) {
        const int64_t address = int64_t(image[0]) + off;
        if (address <= 0 || !DataWindow(uint64_t(address), n)) {
            out->refusal = "unreadable-source-data";
            return false;
        }
        // SPEC-GAP: SR8888-1 leaves true alias load/store order unspecified; retain native rendering.
        // Check actual writes, not the old oversized stride*height envelope.
        for (uint32_t i = 0; i < count; ++i) {
            const uint32_t s = spans[2 * i], w = spans[2 * i + 1] - s;
            if (w == 0) continue;
            const int64_t a = int64_t(dstBase) + int64_t(s >> 16) * dstStride + int64_t(s & 0xFFFFu) * 4;
            if (address < a + int64_t(w) * 4 && a < address + int64_t(n)) {
                out->refusal = "actual-source-destination-alias-SR8888-1";
                return false;
            }
        }
        if (!any || off < out->first) out->first = off;
        if (!any || off + int64_t(n) > out->end) out->end = off + int64_t(n);
        any = true;
        if (off < lo) lo = off;
        if (off + int64_t(n) > hi) hi = off + int64_t(n);
        return true;
    });
    if (!valid) return false;
    if (hi - lo > kWindowLimit) {
        out->refusal = "source-snapshot-over-64MiB";
        return false;
    }
    out->origin = static_cast<size_t>(-lo);
    out->bytes.resize(hi > lo ? static_cast<size_t>(hi - lo) : 1);
    // Sparse capture: unused bytes before/among touched rows are never read
    // from native memory. Rust sees the same texels, with a representable base.
    SourceReads(spans, count, image, desc, mode, [&](int64_t off, size_t n) {
        std::memcpy(out->bytes.data() + static_cast<size_t>(off - lo),
                    reinterpret_cast<const void*>(static_cast<uintptr_t>(int64_t(image[0]) + off)), n);
        return true;
    });
    const int64_t legacyLen = int64_t(static_cast<int32_t>(image[1])) * image[3];
    out->legacyShort = any && (out->first < 0 || out->end > legacyLen || legacyLen <= 0);
    out->legacyOverlap = legacyLen > 0 && uint64_t(image[0]) < reinterpret_cast<uintptr_t>(dst.lo) + dst.len &&
                         reinterpret_cast<uintptr_t>(dst.lo) < uint64_t(image[0]) + uint64_t(legacyLen);
    return true;
}

// Ours on `r` (a region of the real target, or a copy of it).
int32_t Ours(Slot s, const uint32_t* spans, uint32_t count, uint8_t* dst, const Region& r, int32_t stride,
             uint32_t a, uint32_t b, const uint32_t* rectB, const uint8_t* src, uint32_t mode,
             const SourceCapture* captured = nullptr) {
    switch (s) {
        case kSolid: return fc4_span8888_solid(spans, count, dst, r.len, r.origin, stride, a);
        case kAlpha: return fc4_span8888_alpha(spans, count, dst, r.len, r.origin, stride, a, b);
        case kGradient:
        case kAlphaGradient:
            return fc4_span8888_gradient(spans, count, dst, r.len, r.origin, stride, reinterpret_cast<const uint32_t*>(a),
                                         s == kAlphaGradient);
        case kBlt: {
            SourceCapture local;
            if (!captured) {
                if (!CaptureSource(spans, count, r, stride, rectB, src, mode, &local)) return 1;
                captured = &local;
            }
            return fc4_span8888_blt(spans, count, dst, r.len, r.origin, stride,
                                    captured->bytes.data(), captured->bytes.size(), captured->origin,
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
    const bool fixture = SpanContextFixtureThread();
    if (fixture) g.fixtureCalls[s].fetch_add(1, std::memory_order_relaxed);
    SpanNoteCall(kSpanR8888, s + 1, ret, false);  // this module's fixture never calls through the vtable
    Region r;
    bool bridged = RegionOf(spans, count, buffer, &r) && (s != kBlt || (rectB != nullptr && src != nullptr));
    const int32_t stride = bridged ? static_cast<int32_t>(buffer[1]) : 0;
    const char* refusal = "unsupported-destination-window";
    if (bridged) bridged = DataWindow(reinterpret_cast<uintptr_t>(r.lo), r.len, true);
    SourceCapture capture;
    if (bridged && s == kBlt) {
        bridged = CaptureSource(spans, count, r, stride, rectB, src, mode, &capture);
        refusal = capture.refusal;
        if (bridged && (capture.legacyShort || capture.legacyOverlap)) {
            const uint64_t shorter = capture.legacyShort ? g.legacyShort.fetch_add(1) : g.legacyShort.load();
            const uint64_t overlap = capture.legacyOverlap ? g.legacyOverlap.fetch_add(1) : g.legacyOverlap.load();
            if (shorter + overlap < 16)
                Log("span8888: captured %s count=%u mode=%u source=%08X stride=%d width=%u height=%u "
                    "format=%u keyed=%u touched=[%lld,%lld) legacy_short=%u legacy_envelope_overlap=%u",
                    fixture ? "constructed-native" : "runtime", count, mode, rectB[0], static_cast<int32_t>(rectB[1]),
                    rectB[2], rectB[3], reinterpret_cast<const uint32_t*>(src)[4], unsigned(src[28]),
                    (long long)capture.first, (long long)capture.end, unsigned(capture.legacyShort), unsigned(capture.legacyOverlap));
        }
    }
    if (!bridged) {
        if (g.fallback[s].fetch_add(1, std::memory_order_relaxed) < 8)
            Log("span8888: refused %s reason=%s count=%u mode=%u", kSlotName[s], refusal, count, mode);
        if (fixture) g.fixtureFallback[s].fetch_add(1);
        SpanNoteRefused(kSpanR8888);
        Original(s, self, spans, count, buffer, a, b, rectB, src, mode);
        return;
    }
    // The context fixture (FC4SpanContext.cpp) needs ours' pixels even under span8888_shadow.
    if (!g.shadow || fixture) {
        const int32_t status = Ours(s, spans, count, r.lo, r, stride, a, b, rectB, src, mode, &capture);
        if (status == 0) {
            g.ours[s].fetch_add(1, std::memory_order_relaxed);
            if (fixture) g.fixtureOurs[s].fetch_add(1);
        } else {
            if (g.fallback[s].fetch_add(1, std::memory_order_relaxed) < 8)
                Log("span8888: refused %s core status=%d count=%u mode=%u", kSlotName[s], status, count, mode);
            if (fixture) g.fixtureFallback[s].fetch_add(1);
            SpanNoteRefused(kSpanR8888);
            Original(s, self, spans, count, buffer, a, b, rectB, src, mode);
        }
        return;
    }
    // Freeze all non-pixel inputs before native rendering too. The source was
    // captured before Original; true source/destination alias was refused.
    const std::vector<uint32_t> savedSpans(spans, spans + size_t(count) * 2);
    uint32_t savedImage[6] = {}, savedDesc[8] = {}, savedGradient[12] = {};
    if (s == kBlt) {
        std::memcpy(savedImage, rectB, sizeof(savedImage));
        std::memcpy(savedDesc, src, sizeof(savedDesc));
    }
    if (s == kGradient || s == kAlphaGradient) {
        std::memcpy(savedGradient, reinterpret_cast<const void*>(a), sizeof(savedGradient));
        a = static_cast<uint32_t>(reinterpret_cast<uintptr_t>(savedGradient));
    }
    std::vector<uint8_t> copy(r.lo, r.lo + r.len);
    const auto* savedSource = reinterpret_cast<const uint8_t*>(savedDesc);
    Original(s, self, savedSpans.data(), count, buffer, a, b, savedImage, savedSource, mode);
    const int32_t status = Ours(s, savedSpans.data(), count, copy.data(), r, stride, a, b, savedImage, savedSource, mode, &capture);
    if (status != 0) {
        if (g.fallback[s].fetch_add(1, std::memory_order_relaxed) < 8)
            Log("span8888: shadow refused %s core status=%d count=%u mode=%u", kSlotName[s], status, count, mode);
        SpanNoteRefused(kSpanR8888);
        return;
    }
    uint64_t px = 0;
    // Compare every byte, including a trailing partial group for odd strides.
    for (size_t i = 0; i < r.len; i += 4) {
        const size_t n = r.len - i < 4 ? r.len - i : 4;
        px += std::memcmp(&copy[i], r.lo + i, n) != 0;
    }
    if (px != 0) {
        g.diffPixels[s].fetch_add(px, std::memory_order_relaxed);
        if (g.differing[s].fetch_add(1, std::memory_order_relaxed) < 8)
            Log("span8888: DIFF %s count=%u: %llu pixels differ (original vs ours)", kSlotName[s], count,
                (unsigned long long)px);
    }
    g.compared[s].fetch_add(1, std::memory_order_relaxed);
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
    uint64_t cases = 0, pixels = 0, diffs = 0, rejected = 0, faults = 0, guardFailures = 0;
    uint64_t slotCases[kSlots] = {}, slotDiffs[kSlots] = {};
};

struct Call {
    Slot s;
    uint32_t a, b, mode;
    const uint32_t* rectB;
    const uint8_t* src;
    uint32_t* desc;
    const uint32_t* spans;
    uint32_t count;
};

void CallOriginalGuarded(void* ctx) {
    Call* c = static_cast<Call*>(ctx);
    Original(c->s, nullptr, c->spans, c->count, c->desc, c->a, c->b, c->rectB, c->src, c->mode);
}

void RunCase(Fixture& f, Slot s, uint32_t a, uint32_t b, const uint32_t* rectB, const uint8_t* src, uint32_t mode,
             const uint32_t* spans = kSpans, uint32_t count = kSpanCount) {
    constexpr size_t guard = 32;
    uint8_t native[sizeof(f.dstInit) + 2 * guard], ours[sizeof(native)];
    std::memset(native, 0xA5, sizeof(native));
    std::memcpy(native + guard, f.dstInit, sizeof(f.dstInit));
    std::memcpy(ours, native, sizeof(ours));
    uint32_t desc[6] = {static_cast<uint32_t>(reinterpret_cast<uintptr_t>(native + guard)), uint32_t(kStride), kW, kH, 0, 0};
    ++f.cases;
    ++f.slotCases[s];
    g.directCalls[s].fetch_add(1, std::memory_order_relaxed);
    Region r;
    r.lo = ours;
    r.len = sizeof(ours);
    r.origin = guard;
    if (Ours(s, spans, count, ours, r, kStride, a, b, rectB, src, mode) != 0) {
        ++f.rejected;
        return;
    }
    Call c{s, a, b, mode, rectB, src, desc, spans, count};
    if (!SehGuard("span8888 fixture original", &CallOriginalGuarded, &c)) {
        ++f.faults;
        return;
    }
    uint64_t px = 0;
    int first = -1;
    for (size_t i = 0; i < sizeof(native); ++i) {
        bool touched = false;
        for (uint32_t j = 0; j < count; ++j) {
            const uint32_t start = spans[2 * j];
            const size_t off = guard + (start >> 16) * kStride + (start & 0xFFFFu) * 4;
            touched |= i >= off && i < off + (spans[2 * j + 1] - start) * 4;
        }
        const uint8_t initial = i >= guard && i < guard + sizeof(f.dstInit) ? f.dstInit[i - guard] : 0xA5;
        if (!touched && (native[i] != initial || ours[i] != initial)) ++f.guardFailures;
    }
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
        g.directDiffering[s].fetch_add(1, std::memory_order_relaxed);
        if (f.diffs <= 12) {
            uint32_t n = 0, o = 0;
            std::memcpy(&n, native + first, 4);
            std::memcpy(&o, ours + first, 4);
            Log("span8888 fixture [constructed direct]: DIFF %s a=0x%08X b=0x%08X mode=%u: %llu pixels, first at byte %d "
                "(x=%d y=%d) original 0x%08X ours 0x%08X",
                kSlotName[s], a, b, mode, (unsigned long long)px, first, (first % kStride) / 4, first / kStride, n, o);
        }
    }
    g.directCompared[s].fetch_add(1, std::memory_order_relaxed);
}

void RunFixture() {
    static Fixture f;  // ~17 KB
    for (auto& v : f.dstInit) v = uint8_t(f.Next() >> 24);
    for (auto& v : f.src) v = uint8_t(f.Next() >> 24);
    // Slot 1 (§3): all 32 colour bits, including alpha, are copied.
    for (uint32_t c : {0x1234ABCDu, 0x0000F00Fu, 0xFFFF0000u}) RunCase(f, kSolid, c, 0, nullptr, nullptr, 0);
    // Slot 3 (§4): alpha byte in bits 24-31, weight A + (A >> 7).
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
            // Match a sampled texel for each format, with the caller's published key mask.
            const unsigned bpp = format < 4 ? 2 : format < 6 ? 4 : 1;
            const uint32_t masks[] = {0xFFFu, 0x7FFFu, 0x7FFFu, 0xFFFFu, 0xFFFFFFu, 0xFFFFFFu, 0u};
            uint32_t key = 0;
            std::memcpy(&key, f.src + kSrcStride + 2 * bpp, bpp);
            key &= masks[format];
            std::memcpy(desc + 0x18, &key, sizeof(key));
            desc[0x1C] = uint8_t(keyed);
            for (uint32_t mode = 0; mode < 12; ++mode) RunCase(f, kBlt, 0, 0, rectB, desc, mode);
        }
    }
    // Independent native comparisons for the corrected §7 bounds contract:
    // direct/stretch ignore these zero dimensions, for all seven formats.
    const uint32_t ignoredSize[6] = {
        static_cast<uint32_t>(reinterpret_cast<uintptr_t>(f.src)), kSrcStride, 0, 0, 0, 0};
    for (uint32_t format = 0; format < 7; ++format) {
        const uint32_t desc[8] = {0, 0, 0x10000, 0x10000, format, 0xFFFFFFFF, 0, 0};
        for (uint32_t mode : {0u, 4u})
            RunCase(f, kBlt, 0, 0, ignoredSize, reinterpret_cast<const uint8_t*>(desc), mode);
    }
    // A pre-offset base in a RESERVED (unreadable) page, with only the
    // actually sampled rows committed. Never read that unused prefix.
    SYSTEM_INFO sys = {};
    GetSystemInfo(&sys);
    const size_t page = sys.dwPageSize;
    auto* arena = static_cast<uint8_t*>(VirtualAlloc(nullptr, 3 * page, MEM_RESERVE, PAGE_NOACCESS));
    auto* pixels = arena ? static_cast<uint8_t*>(VirtualAlloc(arena + page, 2 * page, MEM_COMMIT, PAGE_READWRITE)) : nullptr;
    if (!pixels) {
        ++f.faults;
    } else {
        for (size_t i = 0; i < 2 * page; ++i) pixels[i] = uint8_t(f.Next() >> 24);
        const uint32_t spans[] = {0x00010002, 0x00010008, 0x00020001, 0x00020009};
        const uint32_t image[6] = {static_cast<uint32_t>(reinterpret_cast<uintptr_t>(arena)),
                                  static_cast<uint32_t>(page), 0, 0, 0, 0};
        const uint32_t desc[8] = {0, 0, 0x10000, 0x10000, 4, 0xFFFFFFFF, 0, 0};
        for (uint32_t mode : {0u, 4u})
            RunCase(f, kBlt, 0, 0, image, reinterpret_cast<const uint8_t*>(desc), mode, spans, 2);
    }
    if (arena) VirtualFree(arena, 0, MEM_RELEASE);
    // Safety controls are refusals, not native comparison successes.
    const uint32_t oneSpan[] = {0, 2};
    uint32_t alias[4] = {0x12345678, 0x9ABCDEF0, 0x11111111, 0x22222222};
    const uint32_t image[6] = {static_cast<uint32_t>(reinterpret_cast<uintptr_t>(alias)), 16, 0, 0, 0, 0};
    const uint32_t desc[8] = {0, 0, 0, 0, 4, 0xFFFFFFFF, 0, 0};
    Region ar;
    ar.lo = reinterpret_cast<uint8_t*>(alias);
    ar.len = sizeof(alias);
    SourceCapture refused;
    const bool safe = !CaptureSource(oneSpan, 1, ar, 16, image, reinterpret_cast<const uint8_t*>(desc), 0, &refused) &&
                      std::strcmp(refused.refusal, "actual-source-destination-alias-SR8888-1") == 0 &&
                      alias[0] == 0x12345678 && alias[1] == 0x9ABCDEF0;
    if (!safe) ++f.guardFailures;
    Log("span8888 fixture safety [not a comparison]: true_alias_refused=%u unchanged=%u", unsigned(safe), unsigned(safe));
    for (unsigned s = 0; s < kSlots; ++s)
        Log("span8888 fixture [constructed direct, not a game path]: slot %u %s cases=%llu differing=%llu", s + 1,
            kSlotName[s], (unsigned long long)f.slotCases[s], (unsigned long long)f.slotDiffs[s]);
    Log("span8888 fixture [constructed direct, not a game path]: TOTAL cases=%llu pixels=%llu differing_cases=%llu "
        "rejected_by_core=%llu faults=%llu untouched_corruptions=%llu -> %s",
        (unsigned long long)f.cases, (unsigned long long)f.pixels, (unsigned long long)f.diffs,
        (unsigned long long)f.rejected, (unsigned long long)f.faults, (unsigned long long)f.guardFailures,
        f.diffs == 0 && f.faults == 0 && f.rejected == 0 && f.guardFailures == 0 ? "EXACT" : "NOT exact");
}

}  // namespace

void SpanRender8888Start() {
    if (!Enabled()) return;
    void** vt = nullptr;
    uintptr_t fn[kSlots] = {};
    for (unsigned i = 0; i < kSlots; ++i) {
        void** entry = nullptr;
        if (!compat::Span8888SlotEntry(i + 1, &entry, &fn[i])) {
            Log("span8888: bridge NOT installed (vtable unavailable)");
            return;
        }
        if (i == 0) vt = entry - 1;
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

void SpanRender8888Month(int32_t julian) {
    if (!g.installed || !g.shadow) return;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    // Sections 1.1 and 7: void methods, stateless owner, read-only arguments;
    // all observable outputs are destination bytes, including alpha. No calls
    // on other objects. Native-context forwards render only ours here, so their
    // outer fixture comparisons are reported separately by FC4SpanContext.
    for (unsigned s = 0; s < kSlots; ++s) {
        Log("span8888 method %s nGZDrawSoftRender::cSpanRender8888::%s (month) %04d-%02d-%02d: "
            "calls=%llu complete=%llu mismatches=%llu; resyncs=0",
            kSlotAddress[s], kSlotName[s], date.year, date.month, date.day_of_month,
            (unsigned long long)(g.calls[s].load() - g.fixtureCalls[s].load()),
            (unsigned long long)g.compared[s].load(), (unsigned long long)g.differing[s].load());
        Log("span8888_direct method %s nGZDrawSoftRender::cSpanRender8888::%s (month) %04d-%02d-%02d: "
            "calls=%llu complete=%llu mismatches=%llu; resyncs=0",
            kSlotAddress[s], kSlotName[s], date.year, date.month, date.day_of_month,
            (unsigned long long)g.directCalls[s].load(), (unsigned long long)g.directCompared[s].load(),
            (unsigned long long)g.directDiffering[s].load());
    }
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
    for (unsigned s = 0; s < kSlots; ++s) {
        Log("span8888: %s calls=%llu ours=%llu original_fallback=%llu shadow_compared=%llu shadow_differing=%llu "
            "diff_pixels=%llu",
            kSlotName[s], (unsigned long long)g.calls[s].load(), (unsigned long long)g.ours[s].load(),
            (unsigned long long)g.fallback[s].load(), (unsigned long long)g.compared[s].load(),
            (unsigned long long)g.differing[s].load(), (unsigned long long)g.diffPixels[s].load());
        const uint64_t fixture = g.fixtureCalls[s].load();
        Log("span8888: runtime %s calls=%llu ours=%llu original_fallback=%llu shadow_compared=%llu "
            "constructed_native_calls=%llu constructed_native_ours=%llu constructed_native_fallback=%llu unaccounted=%llu",
            kSlotName[s], (unsigned long long)(g.calls[s].load() - fixture),
            (unsigned long long)(g.ours[s].load() - g.fixtureOurs[s].load()),
            (unsigned long long)(g.fallback[s].load() - g.fixtureFallback[s].load()),
            (unsigned long long)g.compared[s].load(), (unsigned long long)fixture,
            (unsigned long long)g.fixtureOurs[s].load(), (unsigned long long)g.fixtureFallback[s].load(),
            (unsigned long long)(g.calls[s].load() - g.ours[s].load() - g.fallback[s].load() - g.compared[s].load()));
    }
    Log("span8888: corrected_source_windows legacy_short=%llu legacy_envelope_overlap=%llu resync=0",
        (unsigned long long)g.legacyShort.load(), (unsigned long long)g.legacyOverlap.load());
    Log("span8888: bridge %s", ok ? "removed (original entries back)" : "NOT removed: VirtualProtect failed");
}

}  // namespace fc4
