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

#include "FC4SpanContext.h"

#include "FC4Diag.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Span1555Shadow.h"
#include "FC4Span1555Context.h"
#include "FC4SpanRender4444.h"
#include "compat/FC4Compat.h"

#include "cIGZBuffer.h"
#include "cIGZCOM.h"
#include "cIGZFrameWork.h"
#include "cRZCOMDllDirector.h"

#include <windows.h>

#include <atomic>
#include <cstring>
#include <vector>

namespace fc4 {
namespace {

enum CallerClass : unsigned { kNative = 0, kDirect, kShipped, kOther, kClasses };
const char* const kClassName[kClasses] = {"constructed_native_context", "constructed_direct", "shipped", "other"};
const char* const kRendererName[kSpanRendererCount] = {"span1555", "span4444"};
const char* const kSiteName[7] = {"solid", "gradient", "alpha", "alpha_gradient", "tex_plain", "tex_stretch",
                                  "tex_tiled"};

std::atomic<uint64_t> g_calls[kSpanRendererCount][kClasses][6];
std::atomic<uint64_t> g_sites[kSpanRendererCount][2][7];  // [native context | shipped][site]
std::atomic<uint64_t> g_refused[kSpanRendererCount];
std::atomic<uint32_t> g_activeTid{0};
bool g_fixtureDone = false;

const compat::SpanContextVA* VA() {
    static const compat::SpanContextVA* p = [] {
        static compat::SpanContextVA v;
        return compat::SpanContextAddresses(&v) ? &v : nullptr;
    }();
    return p;
}

// §1.3 counts by return address; span-render-565.md §1.1 lists the call
// instruction addresses. Literal reading: a return address 1..7 bytes after a
// listed site (the length of one CALL instruction) belongs to that site.
int SiteOf(unsigned renderer, uintptr_t ret) {
    const compat::SpanContextVA* va = VA();
    if (va == nullptr) return -1;
    if (renderer == kSpanR1555) return Span1555ContextSite(ret, va->sites);
    for (int i = 0; i < 7; ++i)
        if (ret > va->sites[i] && ret <= va->sites[i] + 7) return i;
    return -1;
}

#if defined(_M_IX86)

// ---- native calls (all thiscall; __fastcall with an EDX dummy) -------------------------------------------------
using CreateCtxFn = uint8_t(__fastcall*)(void*, void*, void*, void**);       // draw system slot 4, RET 8
using CtxInitFn = uint8_t(__fastcall*)(void*, void*, void*);                 // ctx slot 3, RET 4
using CtxVoidFn = void(__fastcall*)(void*, void*);                           // slots 4, 14
using ColorTableFn = void(__fastcall*)(void*, void*, uint32_t, uint32_t, const uint32_t*);  // slot 7, RET 0xC
using DwordFn = void(__fastcall*)(void*, void*, uint32_t);                   // slots 8, 9, RET 4
using GradientFn = void(__fastcall*)(void*, void*, const float*, float, float);  // slot 12, RET 0xC
using BltSourceFn = uint8_t(__fastcall*)(void*, void*, void*, float, float, float, float, const int32_t*);  // 13
using RectFn = void(__fastcall*)(void*, void*, const int32_t*);              // slot 15, RET 4
using DrawSpansFn = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t);  // slot 22, RET 8

template <class F>
F VSlot(void* obj, unsigned slot) {
    return reinterpret_cast<F>((*static_cast<void***>(obj))[slot]);
}
uintptr_t VtableOf(const void* obj) { return obj ? *static_cast<const uintptr_t*>(obj) : 0; }
template <class T>
T Field(const void* obj, unsigned off) {
    T v;
    std::memcpy(&v, static_cast<const uint8_t*>(obj) + off, sizeof(T));
    return v;
}

constexpr uint32_t kCLSID_cGZBuffer = 0xC470D325;  // §1.3 (1)
constexpr uint32_t kSID_DrawSystem = 0xEB2EA3CB, kIID_DrawSystem = 0x4B2EA3B3;  // §1.3 (2)
constexpr uint32_t kDstLock = 0x8040, kTexLock = 0x800;

// 29 pixels of 16 bits: stride (58 + 3) & ~3 = 60, two padding bytes per row (§1.3).
constexpr uint32_t kDW = 29, kDH = 8;
constexpr uint32_t kTW = 64, kTH = 32;  // textures: room for a 2x stretch of the 29-wide arena
// (y << 16) | x, end exclusive; full rows, both edges, single pixels, the last column.
const uint32_t kSpans[] = {0x00000000, 0x0000001D, 0x00010003, 0x00010011, 0x00020001, 0x00020002,
                           0x0003001C, 0x0003001D, 0x00040000, 0x0004001D, 0x00050000, 0x00050001,
                           0x0006000A, 0x0006001B, 0x00070000, 0x0007001D};
constexpr uint32_t kSpanCount = sizeof(kSpans) / sizeof(kSpans[0]) / 2;

struct Buf {
    cIGZBuffer* b = nullptr;
    uint8_t* bits = nullptr;
    uint32_t stride = 0, w = 0, h = 0, type = 0;
};

uint16_t LockCount(const Buf& b) { return Field<uint16_t>(b.b, 0x38); }   // §1.3 Lock / Unlock
uint16_t LockFlags(const Buf& b) { return Field<uint16_t>(b.b, 0x44); }

// Step 1: create, Init (w, h, type, bits), fill between Lock(f) and Unlock(f).
bool MakeBuf(uint32_t w, uint32_t h, uint32_t type, uint32_t bpp, uint32_t lockFlags, uint32_t seed, Buf* out) {
    cIGZCOM* com = GZCOM();
    void* p = nullptr;
    if (com == nullptr || !com->GetClassObject(kCLSID_cGZBuffer, GZIID_cIGZBuffer, &p) || p == nullptr) {
        Log("spanctx: cGZBuffer could not be created");
        return false;
    }
    out->b = static_cast<cIGZBuffer*>(p);
    FC4_ACQUIRED(out->b, "GetClassObject");  // the new buffer's one reference is ours
    out->w = w;
    out->h = h;
    out->type = type;
    if (!out->b->Init(w, h, static_cast<cGZBufferColorType::eColorType>(type), bpp)) {
        Log("spanctx: cGZBuffer Init(%u, %u, type %u, %u) returned 0", w, h, type, bpp);
        return false;
    }
    if (!out->b->Lock(lockFlags)) {
        Log("spanctx: cGZBuffer type %u Lock(0x%X) returned 0", type, lockFlags);
        return false;
    }
    out->bits = static_cast<uint8_t*>(out->b->GetColorSurfaceBits());  // slot 34
    out->stride = out->b->GetColorSurfaceStride();                     // slot 35
    const uint32_t expect = (w * (bpp >> 3) + 3) & ~3u;
    if (out->bits != nullptr && out->stride == expect) {
        uint32_t x = seed * 0x9E3779B9u + 1;
        for (uint32_t i = 0; i < out->stride * h; ++i) {
            x ^= x << 13, x ^= x >> 17, x ^= x << 5;
            out->bits[i] = static_cast<uint8_t>(x >> 24);
        }
    }
    out->b->Unlock(lockFlags);
    if (out->bits == nullptr || out->stride != expect) {
        Log("spanctx: cGZBuffer type %u: pixels %p stride %u (expected %u)", type, static_cast<void*>(out->bits),
            out->stride, expect);
        return false;
    }
    return true;
}

uint32_t FreeBuf(Buf* b) {
    if (b->b == nullptr) return 0;
    const uint32_t left = FC4_RELEASE_CREATED(b->b);
    b->b = nullptr;
    return left;
}

struct DrawCall {
    void* ctx;
};
void DrawGuarded(void* p) {
    void* ctx = static_cast<DrawCall*>(p)->ctx;
    VSlot<DrawSpansFn>(ctx, 22)(ctx, nullptr, kSpans, kSpanCount);
}

enum Kind { kSolid, kAlpha, kGradient, kAlphaGradient, kTexture };
struct Case {
    Kind kind;
    uint32_t argb;
    const float* m;
    float x0, y0;
    const Buf* src;
    float sx, sy;
    bool tiled;
    uint32_t modulate, alphaBlend;
};

// The state of the path under test (§1.3 "Set first").
void SetupCase(void* ctx, const Case& c) {
    const int32_t dstRect[4] = {0, 0, int32_t(kDW), int32_t(kDH)};  // at the origin: (x, y, w, h) = (l, t, r, b)
    const int32_t tile[4] = {0, 0, 8, 8};                          // same in both readings (question SP7)
    VSlot<ColorTableFn>(ctx, 7)(ctx, nullptr, 0, 1, &c.argb);
    if (c.kind == kTexture) {
        VSlot<GradientFn>(ctx, 12)(ctx, nullptr, nullptr, 0.0f, 0.0f);
        VSlot<BltSourceFn>(ctx, 13)(ctx, nullptr, c.src->b, 0.0f, 0.0f, c.sx, c.sy, dstRect);
        VSlot<DwordFn>(ctx, 9)(ctx, nullptr, c.modulate);
        VSlot<DwordFn>(ctx, 8)(ctx, nullptr, c.alphaBlend);
        if (c.tiled) VSlot<RectFn>(ctx, 15)(ctx, nullptr, tile);
        else VSlot<CtxVoidFn>(ctx, 14)(ctx, nullptr);
        return;
    }
    VSlot<BltSourceFn>(ctx, 13)(ctx, nullptr, nullptr, 0.0f, 0.0f, 0.0f, 0.0f, nullptr);  // no texture
    VSlot<CtxVoidFn>(ctx, 14)(ctx, nullptr);
    VSlot<DwordFn>(ctx, 8)(ctx, nullptr, (c.kind == kAlpha || c.kind == kAlphaGradient) ? 1u : 0u);
    const bool grad = c.kind == kGradient || c.kind == kAlphaGradient;
    VSlot<GradientFn>(ctx, 12)(ctx, nullptr, grad ? c.m : nullptr, grad ? c.x0 : 0.0f, grad ? c.y0 : 0.0f);
}

int ExpectedSite(const Case& c) {
    switch (c.kind) {
        case kSolid: return 0;
        case kGradient: return 1;
        case kAlpha: return 2;
        case kAlphaGradient: return 3;
        default: return c.tiled ? 6 : (c.sx != 1.0f || c.sy != 1.0f) ? 5 : 4;
    }
}

struct Totals {
    uint64_t cases = 0, exact = 0, diff = 0, refused = 0, faults = 0, routeMismatch = 0, noCall = 0, bytes = 0,
             changed = 0;
    uint64_t perSite[7] = {}, perSiteExact[7] = {};
    uint64_t sp1Cases = 0, sp1Diff = 0;  // alpha ramp outside 0..255 somewhere (question SP1, open)
};

// The alpha ramp the context built at +0x44 (record channel 3: +0x50, +0x60, +0x70; span-render-1555.md §4.1):
// false if any span pixel's 8.16 alpha lies outside 0..255.99 (span-render-4444.md §6 weights 17..31, SP1).
bool AlphaInRange(const void* ctx) {
    const int32_t c = Field<int32_t>(ctx, 0x50), dx = Field<int32_t>(ctx, 0x60), dy = Field<int32_t>(ctx, 0x70);
    for (uint32_t i = 0; i < kSpanCount; ++i) {
        const uint32_t s = kSpans[2 * i], e = kSpans[2 * i + 1];
        for (uint32_t x = s & 0xFFFF; x < (e & 0xFFFF); ++x) {
            const int32_t a = static_cast<int32_t>(uint32_t(c) + uint32_t(dx) * x + uint32_t(dy) * (s >> 16));
            if (a < 0 || a > 0xFFFFFF) return false;
        }
    }
    return true;
}

uint64_t NativeSiteCount(unsigned r, int site) { return g_sites[r][0][site].load(); }

bool UseOriginal(unsigned r, bool original) {
    return r == kSpanR1555 ? Span1555UseOriginal(original) : SpanRender4444UseOriginal(original);
}

void RunCase(unsigned r, void* ctx, Buf& dst, const std::vector<uint8_t>& init, const Case& c, Totals& t) {
    const size_t len = size_t(dst.stride) * dst.h;
    SetupCase(ctx, c);
    // SP1 is a 4444-only contract; 1555 §5 specifies the entire packed domain.
    const bool sp1 = r == kSpanR4444 && c.kind == kAlphaGradient && !AlphaInRange(ctx);
    t.sp1Cases += sp1;
    DrawCall call{ctx};
    // Original: the published functions in the renderer's vtable.
    std::memcpy(dst.bits, init.data(), len);
    UseOriginal(r, true);
    g_activeTid.store(GetCurrentThreadId());
    const bool okA = SehGuard("spanctx slot 22 (original)", &DrawGuarded, &call);
    g_activeTid.store(0);
    std::vector<uint8_t> a(dst.bits, dst.bits + len);
    // Ours: the bridge's forwarders.
    std::memcpy(dst.bits, init.data(), len);
    UseOriginal(r, false);
    uint64_t before[7];
    for (int s = 0; s < 7; ++s) before[s] = NativeSiteCount(r, s);
    const uint64_t refusedBefore = g_refused[r].load();
    g_activeTid.store(GetCurrentThreadId());
    const bool okB = SehGuard("spanctx slot 22 (ours)", &DrawGuarded, &call);
    g_activeTid.store(0);
    ++t.cases;
    t.bytes += len;
    for (size_t i = 0; i < len; ++i) t.changed += a[i] != init[i];
    int hitSite = -1, hits = 0;
    for (int s = 0; s < 7; ++s) {
        const uint64_t d = NativeSiteCount(r, s) - before[s];
        if (d) hitSite = s, hits += int(d);
    }
    const int want = ExpectedSite(c);
    if (hits == 0) ++t.noCall;
    else ++t.perSite[hitSite];
    if (!okA || !okB) {
        ++t.faults;
        return;
    }
    if (g_refused[r].load() != refusedBefore) {
        ++t.refused;
        return;
    }
    // A texture whose type selects no renderer makes no call (0x008305AA): not a route mismatch.
    if ((hits != 1 || hitSite != want) && !(hits == 0 && c.kind == kTexture)) {
        if (t.routeMismatch++ < 6)
            Log("spanctx: %s case kind %d: route %d call(s), site %s (expected one at %s)", kRendererName[r], c.kind,
                hits, hitSite >= 0 ? kSiteName[hitSite] : "none", kSiteName[want]);
    }
    size_t first = len, n = 0;
    for (size_t i = 0; i < len; ++i)
        if (a[i] != dst.bits[i]) n++, first = first == len ? i : first;
    if (sp1 && n != 0) {  // a known open difference, reported apart (never counted exact)
        if (t.sp1Diff++ < 4)
            Log("spanctx: %s SP1 (alpha ramp outside 0..255, open question) differs: %zu byte(s), first at arena "
                "offset %zu: original 0x%02X ours 0x%02X",
                kRendererName[r], n, first, a[first], dst.bits[first]);
        return;
    }
    if (n == 0) {
        ++t.exact;
        if (hits == 1) ++t.perSiteExact[hitSite];
        return;
    }
    if (t.diff++ < 8)
        Log("spanctx: %s DIFF kind %d site %s: %zu byte(s), first at arena offset %zu (row %zu col-byte %zu): "
            "original 0x%02X ours 0x%02X",
            kRendererName[r], c.kind, hitSite >= 0 ? kSiteName[hitSite] : "none", n, first, first / dst.stride,
            first % dst.stride, a[first], dst.bits[first]);
}

const float kGrad[5][12] = {
    {0.2f, 0.5f, 0.8f, 0.3f, 0.01f, -0.02f, 0.015f, 0.03f, 0.05f, 0.04f, -0.06f, 0.1f},
    {1.0f, 0.0f, 0.5f, 1.0f, -0.03f, 0.02f, 0.0f, -0.04f, 0.0f, 0.07f, 0.02f, -0.11f},
    {0.0f, 0.0f, 0.0f, 0.0f, 0.035f, 0.035f, 0.035f, 0.035f, 0.1f, 0.1f, 0.1f, 0.1f},  // leaves 0..1 (wraps)
    // Alpha inside 0..255 on every span pixel (both x0/y0 pairs): the in-range alpha-gradient cases.
    {0.1f, 0.6f, 0.9f, 0.05f, 0.02f, -0.01f, -0.02f, 0.02f, 0.03f, 0.02f, -0.05f, 0.03f},
    {0.5f, 0.5f, 0.5f, 0.95f, -0.01f, 0.01f, 0.005f, -0.025f, 0.01f, -0.02f, 0.02f, -0.02f},
};

// Steps 2-5 for one destination type.
void RunDestination(unsigned r, uint32_t type, const compat::SpanContextVA& va, std::vector<Buf>& textures) {
    const uintptr_t wantRenderer = r == kSpanR1555 ? va.renderer1555Vtable : va.renderer4444Vtable;
    Buf dst;
    if (!MakeBuf(kDW, kDH, type, 16, kDstLock, type, &dst)) {
        Log("spanctx: %s type %u: destination NOT made: route not run", kRendererName[r], type);
        FreeBuf(&dst);
        return;
    }
    const std::vector<uint8_t> init(dst.bits, dst.bits + size_t(dst.stride) * dst.h);
    // Step 2.
    cIGZFrameWork* fw = RZGetFrameWork();
    void* ds = nullptr;
    if (fw == nullptr || !fw->GetSystemService(kSID_DrawSystem, kIID_DrawSystem, &ds) || ds == nullptr) {
        Log("spanctx: draw system service 0x%08X NOT available", kSID_DrawSystem);
        FreeBuf(&dst);
        return;
    }
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(ds), "GetSystemService");
    void* ctx = nullptr;
    const uint8_t made = VSlot<CreateCtxFn>(ds, 4)(ds, nullptr, dst.b, &ctx);
    FC4_RELEASE(static_cast<cIGZUnknown*>(ds));
    if (!made || ctx == nullptr) {
        Log("spanctx: %s type %u: draw system slot 4 returned %u ctx %p", kRendererName[r], type, made, ctx);
        FreeBuf(&dst);
        return;
    }
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(ctx), "draw system slot 4 (RenderToBuffer context)");
    const uintptr_t ctxVt = VtableOf(ctx);
    const uint32_t refs = Field<uint32_t>(ctx, 4);
    if (ctxVt != va.contextVtable) {
        Log("spanctx: %s type %u: context vtable 0x%08lX, not 0x%08lX: not used", kRendererName[r], type,
            static_cast<unsigned long>(ctxVt), static_cast<unsigned long>(va.contextVtable));
        FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(ctx));
        FreeBuf(&dst);
        return;
    }
    // Step 3.
    const uint16_t lc0 = LockCount(dst), lf0 = LockFlags(dst);
    const uint8_t inited = VSlot<CtxInitFn>(ctx, 3)(ctx, nullptr, dst.b);
    const void* renderer = Field<void*>(ctx, 0xB0);
    const uintptr_t rendVt = VtableOf(renderer);
    const uint16_t lc1 = LockCount(dst), lf1 = LockFlags(dst);
    Log("spanctx: %s type %u: ctx vtable 0x%08lX refs %u; Init -> %u, renderer vtable 0x%08lX (want 0x%08lX); "
        "dst lock count %u->%u flags 0x%04X->0x%04X",
        kRendererName[r], type, static_cast<unsigned long>(ctxVt), refs, inited, static_cast<unsigned long>(rendVt),
        static_cast<unsigned long>(wantRenderer), lc0, lc1, lf0, lf1);
    Totals t;
    if (inited == 1 && rendVt == wantRenderer && dst.bits == Field<uint8_t*>(ctx, 0x78)) {
        std::vector<Case> cases;
        for (uint32_t argb : {0x80FF8040u, 0x00000000u, 0xFFFFFFFFu, 0x12345678u})
            cases.push_back({kSolid, argb, nullptr, 0, 0, nullptr, 1, 1, false, 0, 0});
        for (uint32_t argb : {0x00FF8040u, 0x40FF8040u, 0x80123456u, 0xFF0F0F0Fu, 0x7FFFFFFFu, 0xC0808080u})
            cases.push_back({kAlpha, argb, nullptr, 0, 0, nullptr, 1, 1, false, 0, 0});
        for (const auto& m : kGrad)
            for (int xy = 0; xy < 2; ++xy)
                for (Kind k : {kGradient, kAlphaGradient})
                    cases.push_back({k, 0, m, xy ? 1.5f : 0.0f, xy ? -2.25f : 0.0f, nullptr, 1, 1, false, 0, 0});
        const float geo[4][2] = {{1.0f, 1.0f}, {2.0f, 2.0f}, {0.5f, 0.75f}, {1.0f, 1.0f}};
        for (const Buf& src : textures)
            for (int gi = 0; gi < 4; ++gi)
                for (uint32_t mod : {0u, 1u})
                    for (uint32_t ab : {0u, 1u})
                        cases.push_back({kTexture, 0x80C0A040u, nullptr, 0, 0, &src, geo[gi][0], geo[gi][1], gi == 3,
                                         mod, ab});
        std::vector<uint16_t> texLocks;
        for (const Buf& src : textures) texLocks.push_back(LockCount(src));
        for (const Case& c : cases) RunCase(r, ctx, dst, init, c, t);
        VSlot<BltSourceFn>(ctx, 13)(ctx, nullptr, nullptr, 0.0f, 0.0f, 0.0f, 0.0f, nullptr);  // releases +0x24
        uint32_t texLockMoved = 0;
        for (size_t i = 0; i < textures.size(); ++i) texLockMoved += LockCount(textures[i]) != texLocks[i];
        for (int s = 0; s < 7; ++s)
            Log("spanctx: %s type %u route %s: native-context calls=%llu exact=%llu", kRendererName[r], type,
                kSiteName[s], (unsigned long long)t.perSite[s], (unsigned long long)t.perSiteExact[s]);
        Log("spanctx: %s type %u TOTAL [constructed buffer, native context, not a shipped call]: cases=%llu "
            "exact=%llu diff=%llu ours_refused=%llu faults=%llu route_mismatch=%llu no_renderer_call=%llu "
            "arena_bytes=%llu changed_by_original=%llu texture_lock_counts_moved=%u sp1_cases=%llu sp1_differing=%llu -> %s",
            kRendererName[r], type, (unsigned long long)t.cases, (unsigned long long)t.exact,
            (unsigned long long)t.diff, (unsigned long long)t.refused, (unsigned long long)t.faults,
            (unsigned long long)t.routeMismatch, (unsigned long long)t.noCall, (unsigned long long)t.bytes,
            (unsigned long long)t.changed, texLockMoved, (unsigned long long)t.sp1Cases, (unsigned long long)t.sp1Diff,
            (t.cases > 0 && t.exact + t.sp1Diff == t.cases && t.diff == 0 && t.routeMismatch == 0)
                ? (t.sp1Diff ? "EXACT outside SP1" : "EXACT")
                : "NOT exact");
    } else {
        Log("spanctx: %s type %u: Init / renderer / record check failed: route not run", kRendererName[r], type);
    }
    // Step 5.
    VSlot<CtxVoidFn>(ctx, 4)(ctx, nullptr);  // Shutdown: Unlock(0x8040) while +0x75 is set
    const uint16_t lc2 = LockCount(dst), lf2 = LockFlags(dst);
    const void* rendAfter = Field<void*>(ctx, 0xB0);
    const uint32_t refsBefore = Field<uint32_t>(ctx, 4);
    const uint32_t ctxLeft = FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(ctx));
    const uint32_t dstLeft = FreeBuf(&dst);
    Log("spanctx: %s type %u lifetime: Shutdown -> renderer %p, dst lock count %u flags 0x%04X (before Init %u "
        "0x%04X: %s); ctx refs %u, Release -> %u; dst Release -> %u",
        kRendererName[r], type, rendAfter, lc2, lf2, lc0, lf0, (lc2 == lc0 && lf2 == lf0) ? "balanced" : "NOT balanced",
        refsBefore, ctxLeft, dstLeft);
}

void RunFixture() {
    const compat::SpanContextVA* va = VA();
    if (va == nullptr) {
        Log("spanctx: addresses unavailable: fixture not run");
        return;
    }
    const bool on1555 = Span1555UseOriginal(false), on4444 = SpanRender4444UseOriginal(false);
    if (!on1555 && !on4444) return;
    // Texture sources: every buffer type a published renderer table names (16-bit 4..7, 32-bit 8, 9).
    std::vector<Buf> textures;
    for (uint32_t type : {4u, 5u, 6u, 7u, 8u, 9u}) {
        Buf b;
        if (MakeBuf(kTW, kTH, type, type >= 8 ? 32 : 16, kTexLock, 100 + type, &b)) {
            textures.push_back(b);
        } else {
            Log("spanctx: texture type %u NOT made (skipped)", type);
            FreeBuf(&b);
        }
    }
    if (on1555) {
        RunDestination(kSpanR1555, 4, *va, textures);
        RunDestination(kSpanR1555, 6, *va, textures);
    }
    if (on4444) RunDestination(kSpanR4444, 7, *va, textures);
    for (Buf& b : textures) {
        const uint32_t type = b.type;
        const uint32_t left = FreeBuf(&b);
        Log("spanctx: texture type %u Release -> %u (%s)", type, left, left == 0 ? "balanced" : "NOT balanced");
    }
    if (on1555) Span1555UseOriginal(false);
    if (on4444) SpanRender4444UseOriginal(false);
}

#endif  // _M_IX86

void LogCounts(const char* when) {
    for (unsigned r = 0; r < kSpanRendererCount; ++r) {
        for (unsigned c = 0; c < kClasses; ++c) {
            uint64_t sum = 0;
            for (unsigned s = 1; s <= 5; ++s) sum += g_calls[r][c][s].load();
            Log("spanctx: %s %s %s calls: total=%llu solid=%llu gradient=%llu alpha=%llu alpha_gradient=%llu "
                "blt=%llu%s",
                when, kRendererName[r], kClassName[c], (unsigned long long)sum,
                (unsigned long long)g_calls[r][c][1].load(), (unsigned long long)g_calls[r][c][2].load(),
                (unsigned long long)g_calls[r][c][3].load(), (unsigned long long)g_calls[r][c][4].load(),
                (unsigned long long)g_calls[r][c][5].load(),
                c == kShipped && sum == 0 ? " (no natural call observed in this session)" : "");
        }
        uint64_t shippedSites[7];
        for (int s = 0; s < 7; ++s) shippedSites[s] = g_sites[r][1][s].load();
        Log("spanctx: %s %s shipped per site: solid=%llu gradient=%llu alpha=%llu alpha_gradient=%llu tex_plain=%llu "
            "tex_stretch=%llu tex_tiled=%llu",
            when, kRendererName[r], (unsigned long long)shippedSites[0], (unsigned long long)shippedSites[1],
            (unsigned long long)shippedSites[2], (unsigned long long)shippedSites[3],
            (unsigned long long)shippedSites[4], (unsigned long long)shippedSites[5],
            (unsigned long long)shippedSites[6]);
    }
}

}  // namespace

void SpanNoteCall(unsigned renderer, unsigned slot, uintptr_t ret, bool fixtureDirect) {
    if (renderer >= kSpanRendererCount || slot < 1 || slot > 5) return;
    const int site = SiteOf(renderer, ret);
    const unsigned c = site >= 0 ? (SpanContextFixtureThread() ? kNative : kShipped) : (fixtureDirect ? kDirect : kOther);
    g_calls[renderer][c][slot].fetch_add(1, std::memory_order_relaxed);
    if (site >= 0) g_sites[renderer][c == kNative ? 0 : 1][site].fetch_add(1, std::memory_order_relaxed);
}

void SpanNoteRefused(unsigned renderer) {
    if (renderer < kSpanRendererCount) g_refused[renderer].fetch_add(1, std::memory_order_relaxed);
}

bool SpanContextFixtureThread() {
    const uint32_t t = g_activeTid.load(std::memory_order_relaxed);
    return t != 0 && t == GetCurrentThreadId();
}

void SpanContextCityLoaded() {
    if (g_fixtureDone) return;
    g_fixtureDone = true;
#if defined(_M_IX86)
    RunFixture();
    LogCounts("after fixture");
#endif
}

void SpanContextShutdown() {
    if (g_fixtureDone) LogCounts("at shutdown");
}

}  // namespace fc4
