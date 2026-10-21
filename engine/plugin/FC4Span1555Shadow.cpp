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

// The cSpanRender1555 native bridge; see FC4Span1555Shadow.h.
#include "FC4Span1555Shadow.h"

#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "compat/FC4Compat.h"
#include "fc4.h"

#include "cIGZBuffer.h"
#include "cIGZCOM.h"
#include "cRZCOMDllDirector.h"

#include <windows.h>

#include <atomic>
#include <cstdint>
#include <cstring>
#include <vector>

namespace fc4 {
namespace {

#if defined(_M_IX86)

// §1.1: slot 1 RenderSolid, 2 RenderGradient, 3 RenderAlpha, 4 RenderAlphaGradient, 5 RenderBlt.
// MSVC thiscall with callee pop, emulated as __fastcall with an unused EDX argument.
using SolidFn = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t, const uint32_t*, uint32_t);
using GradFn = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t, const uint32_t*, const int32_t*);
using AlphaFn = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t, const uint32_t*, uint32_t, uint32_t);
using BltFn = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t, const uint32_t*, const uint32_t*,
                                const uint32_t*, uint32_t);

const char* const kSlotName[6] = {"dtor", "RenderSolid", "RenderGradient", "RenderAlpha", "RenderAlphaGradient",
                                  "RenderBlt"};

struct SlotStats {
    std::atomic<uint32_t> calls{0}, exact{0}, diff{0}, oursErr{0}, skipped{0}, gameCalls{0};
    std::atomic<uint64_t> changedBytes{0};
};

struct State {
    bool installed = false;
    uintptr_t original[6] = {};
    SlotStats s[6];
    std::atomic<uint32_t> detailLines{0};
    bool fixtureDone = false;
};
State g;
thread_local bool t_fixture = false;

// The destination bytes the spans cover (§1: base + y × stride + 2 × x, 32-bit products).
struct Win {
    uint8_t* lo = nullptr;
    uint32_t len = 0;
    int64_t base = 0;  // offset of pixel (0, 0) from lo
    int32_t stride = 0;
};

bool DstWindow(const uint32_t* spans, uint32_t n, const uint32_t* surface, Win* w) {
    if (n == 0 || spans == nullptr || surface == nullptr) return false;
    const int32_t stride = static_cast<int32_t>(surface[1]);
    int64_t mn = INT64_MAX, mx = INT64_MIN;
    for (uint32_t i = 0; i < n; ++i) {
        const uint32_t s = spans[2 * i], e = spans[2 * i + 1];
        if ((s >> 16) != (e >> 16) || (e & 0xFFFF) < (s & 0xFFFF)) return false;  // §1: off its row
        if (e == s) continue;
        const int64_t off = static_cast<int32_t>((s >> 16) * static_cast<uint32_t>(stride)) + 2 * int64_t(s & 0xFFFF);
        mn = off < mn ? off : mn;
        const int64_t end = off + 2 * int64_t(e - s);
        mx = end > mx ? end : mx;
    }
    if (mn == INT64_MAX) mn = mx = 0;
    if (mx - mn > (64 << 20)) return false;
    w->lo = reinterpret_cast<uint8_t*>(surface[0]) + mn;
    w->len = static_cast<uint32_t>(mx - mn);
    w->base = -mn;
    w->stride = stride;
    return true;
}

bool Readable(const void* p, MEMORY_BASIC_INFORMATION* mbi) {
    const DWORD ok = PAGE_READONLY | PAGE_READWRITE | PAGE_WRITECOPY | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE;
    return VirtualQuery(p, mbi, sizeof(*mbi)) == sizeof(*mbi) && mbi->State == MEM_COMMIT && (mbi->Protect & ok) &&
           !(mbi->Protect & PAGE_GUARD);
}

// Runs the original, keeps its bytes, restores the snapshot, runs ours and compares.
template <class Orig, class Ours>
void Shadow(unsigned slot, const Win& w, Orig orig, Ours ours) {
    SlotStats& st = g.s[slot];
    std::vector<uint8_t> before(w.lo, w.lo + w.len);
    orig();
    std::vector<uint8_t> o(w.lo, w.lo + w.len);
    uint32_t changed = 0;
    for (uint32_t i = 0; i < w.len; ++i) changed += o[i] != before[i];
    st.changedBytes += changed;
    if (w.len) memcpy(w.lo, before.data(), w.len);
    const int32_t rc = ours();
    if (rc != 0) {
        if (w.len) memcpy(w.lo, o.data(), w.len);  // keep the original's pixels
        ++st.oursErr;
        if (g.detailLines++ < 8) Log("span1555: %s ours refused the input (code %d); original kept", kSlotName[slot], rc);
        return;
    }
    if (w.len == 0 || memcmp(w.lo, o.data(), w.len) == 0) {
        ++st.exact;
        return;
    }
    ++st.diff;
    uint32_t first = 0, count = 0;
    for (uint32_t i = 0; i < w.len; ++i) {
        if (w.lo[i] != o[i]) {
            if (count++ == 0) first = i;
        }
    }
    if (g.detailLines++ < 8) {
        const uint32_t a = first & ~1u;
        Log("span1555: %s DIFF %u byte(s); first at window offset %u: original 0x%04X ours 0x%04X", kSlotName[slot],
            count, first, static_cast<unsigned>(o[a] | (o[a + 1 < w.len ? a + 1 : a] << 8)),
            static_cast<unsigned>(w.lo[a] | (w.lo[a + 1 < w.len ? a + 1 : a] << 8)));
    }
}

bool Enter(unsigned slot) {
    SlotStats& st = g.s[slot];
    ++st.calls;
    if (!t_fixture && st.gameCalls++ == 0) Log("span1555: game call: first %s call from a non-fixture caller", kSlotName[slot]);
    return true;
}

void __fastcall SolidFwd(void* self, void* edx, const uint32_t* spans, uint32_t n, const uint32_t* surface,
                         uint32_t colour) {
    Enter(1);
    auto orig = [&] { reinterpret_cast<SolidFn>(g.original[1])(self, edx, spans, n, surface, colour); };
    Win w;
    if (!DstWindow(spans, n, surface, &w)) {
        ++g.s[1].skipped;
        orig();
        return;
    }
    Shadow(1, w, orig, [&] { return fc4_span1555_solid(spans, n, w.lo, w.len, w.base, w.stride, colour); });
}

void __fastcall GradientFwd(void* self, void* edx, const uint32_t* spans, uint32_t n, const uint32_t* surface,
                            const int32_t* grad) {
    Enter(2);
    auto orig = [&] { reinterpret_cast<GradFn>(g.original[2])(self, edx, spans, n, surface, grad); };
    Win w;
    if (!DstWindow(spans, n, surface, &w)) {
        ++g.s[2].skipped;
        orig();
        return;
    }
    Shadow(2, w, orig, [&] { return fc4_span1555_gradient(spans, n, w.lo, w.len, w.base, w.stride, grad); });
}

void __fastcall AlphaFwd(void* self, void* edx, const uint32_t* spans, uint32_t n, const uint32_t* surface,
                         uint32_t src, uint32_t argb) {
    Enter(3);
    auto orig = [&] { reinterpret_cast<AlphaFn>(g.original[3])(self, edx, spans, n, surface, src, argb); };
    Win w;
    if (!DstWindow(spans, n, surface, &w)) {
        ++g.s[3].skipped;
        orig();
        return;
    }
    Shadow(3, w, orig, [&] { return fc4_span1555_alpha(spans, n, w.lo, w.len, w.base, w.stride, src, argb); });
}

void __fastcall AlphaGradientFwd(void* self, void* edx, const uint32_t* spans, uint32_t n, const uint32_t* surface,
                                 const int32_t* grad) {
    Enter(4);
    auto orig = [&] { reinterpret_cast<GradFn>(g.original[4])(self, edx, spans, n, surface, grad); };
    Win w;
    if (!DstWindow(spans, n, surface, &w)) {
        ++g.s[4].skipped;
        orig();
        return;
    }
    Shadow(4, w, orig, [&] { return fc4_span1555_alpha_gradient(spans, n, w.lo, w.len, w.base, w.stride, grad); });
}

void __fastcall BltFwd(void* self, void* edx, const uint32_t* spans, uint32_t n, const uint32_t* surface,
                       const uint32_t* src, const uint32_t* b, uint32_t kind) {
    Enter(5);
    auto orig = [&] { reinterpret_cast<BltFn>(g.original[5])(self, edx, spans, n, surface, src, b, kind); };
    Win w;
    MEMORY_BASIC_INFORMATION mbi{};
    // The source window: the readable pages around the record's pixel pointer (§6.1 +0), or,
    // for a pointer biased off its buffer, around the first span's first texel. The core
    // reads only inside it (a texel outside is code 4: the original's pixels are kept).
    bool ok = src != nullptr && b != nullptr && DstWindow(spans, n, surface, &w);
    if (ok && !Readable(reinterpret_cast<const void*>(src[0]), &mbi)) {
        const uint32_t s0 = spans[0];
        const uintptr_t t = src[0] + (s0 >> 16) * src[1] + (s0 & 0xFFFF) * src[5];
        ok = Readable(reinterpret_cast<const void*>(t), &mbi);
    }
    if (!ok) {
        ++g.s[5].skipped;
        orig();
        return;
    }
    const uint8_t* lo = static_cast<const uint8_t*>(mbi.BaseAddress);
    const uint32_t len = static_cast<uint32_t>(mbi.RegionSize);
    const int64_t sbase = static_cast<int64_t>(src[0]) - static_cast<int64_t>(reinterpret_cast<uintptr_t>(lo));
    Shadow(5, w, orig, [&] {
        return fc4_span1555_blt(spans, n, w.lo, w.len, w.base, w.stride, lo, len, sbase, src, b, kind);
    });
}

void* ForwarderOf(unsigned slot) {
    switch (slot) {
        case 1: return reinterpret_cast<void*>(&SolidFwd);
        case 2: return reinterpret_cast<void*>(&GradientFwd);
        case 3: return reinterpret_cast<void*>(&AlphaFwd);
        case 4: return reinterpret_cast<void*>(&AlphaGradientFwd);
        case 5: return reinterpret_cast<void*>(&BltFwd);
        default: return nullptr;
    }
}

bool Install() {
    void** entries[6] = {};
    for (unsigned i = 1; i <= 5; ++i) {
        uintptr_t expected = 0;
        if (!compat::Span1555SlotEntry(i, &entries[i], &expected)) return false;
        const uintptr_t found = reinterpret_cast<uintptr_t>(*entries[i]);
        if (found != expected) {
            Log("span1555: NOT installed: vtable entry %u holds 0x%08lX, expected 0x%08lX", i,
                static_cast<unsigned long>(found), static_cast<unsigned long>(expected));
            return false;
        }
        g.original[i] = found;
    }
    void** first = entries[1];
    DWORD before = 0;
    if (!VirtualProtect(first, 5 * sizeof(void*), PAGE_READWRITE, &before)) {
        Log("span1555: NOT installed: VirtualProtect failed (error %lu)", GetLastError());
        return false;
    }
    for (unsigned i = 1; i <= 5; ++i) *entries[i] = ForwarderOf(i);
    DWORD ignored = 0;
    VirtualProtect(first, 5 * sizeof(void*), before, &ignored);
    FlushInstructionCache(GetCurrentProcess(), first, 5 * sizeof(void*));
    return true;
}

// ---- the constructed fixture (§1.2 "What a pixel test can use") ---------------------------------

constexpr uint32_t kCLSID_cGZBuffer = 0xC470D325;  // GZCLSIDDefs.h kcGZBuffer; span-render-1555.md §1.2

struct Buf {
    cIGZBuffer* b = nullptr;
    uint8_t* bits = nullptr;
    uint32_t stride = 0, bpp = 0, type = 0, w = 0, h = 0;
};

// The lock flags are the ones the published callers use (§1.1: Init locks its target with
// 0x8040, the textured path its texture with 0x800); unlocking with the same flags is the
// literal reading (the spec gives no Unlock argument).
bool MakeBuffer(uint32_t w, uint32_t h, uint32_t type, uint32_t bits, uint32_t lockFlags, Buf* out) {
    cIGZCOM* com = GZCOM();
    void* p = nullptr;
    if (com == nullptr || !com->GetClassObject(kCLSID_cGZBuffer, GZIID_cIGZBuffer, &p) || p == nullptr) {
        Log("span1555: fixture: cGZBuffer 0x%08X could not be created", kCLSID_cGZBuffer);
        return false;
    }
    out->b = static_cast<cIGZBuffer*>(p);
    FC4_ACQUIRED(out->b, "GetClassObject");  // the new buffer's one reference is ours
    if (!out->b->Init(w, h, static_cast<cGZBufferColorType::eColorType>(type), bits) || !out->b->Lock(lockFlags)) {
        Log("span1555: fixture: cGZBuffer Init/Lock (type %u, %u bpp) failed", type, bits);
        FC4_RELEASE_CREATED(out->b);
        out->b = nullptr;
        return false;
    }
    out->bits = static_cast<uint8_t*>(out->b->GetColorSurfaceBits());  // vfunc +0x88 (§1.1)
    out->stride = out->b->GetColorSurfaceStride();                     // vfunc +0x8C
    out->bpp = out->b->GetBytesPerPixel();                             // vfunc +0x44
    out->type = type;
    out->w = w;
    out->h = h;
    if (out->bits == nullptr || out->bpp * w > out->stride) {
        Log("span1555: fixture: cGZBuffer type %u gave no usable surface", type);
        return false;
    }
    for (uint32_t y = 0; y < h; ++y) {
        for (uint32_t x = 0; x < w * out->bpp; ++x) {
            out->bits[y * out->stride + x] = static_cast<uint8_t>((x * 0x9D + y * 0x3B + type * 0x51) ^ (x >> 3));
        }
    }
    return true;
}

void FreeBuffer(Buf* b, uint32_t lockFlags) {
    if (b->b == nullptr) return;
    if (b->bits != nullptr) b->b->Unlock(lockFlags);
    FC4_RELEASE_CREATED(b->b);
    b->b = nullptr;
}

uint32_t Fnv(const uint8_t* p, uint32_t n) {
    uint32_t h = 2166136261u;
    for (uint32_t i = 0; i < n; ++i) h = (h ^ p[i]) * 16777619u;
    return h;
}

// §6.1: source-format index by texture colour type, and the colour-key mask by index.
uint32_t FormatIndex(uint32_t type) { return type == 7 ? 0 : type == 6 ? 1 : type == 9 ? 4 : 0; }
uint32_t KeyMask(uint32_t idx) { return idx == 0 ? 0x0FFF : idx == 4 ? 0xFFFFFF : 0x7FFF; }

void RunFixture() {
    uint32_t obj = 0;  // a 4-byte cSpanRender1555 (§1.1: only the vtable pointer; `this` is never read)
    {
        void** entry = nullptr;
        if (!compat::Span1555SlotEntry(0, &entry, nullptr)) return;
        obj = static_cast<uint32_t>(reinterpret_cast<uintptr_t>(entry));
    }
    void** vt = reinterpret_cast<void**>(obj);
    t_fixture = true;
    uint32_t casesRun = 0;
    const uint32_t kDstTypes[2] = {4, 6};
    const uint32_t kSrcTypes[3] = {7, 6, 9};
    for (uint32_t dt : kDstTypes) {
        Buf d;
        if (!MakeBuffer(32, 8, dt, 16, 0x8040, &d)) {
            FreeBuffer(&d, 0x8040);
            continue;
        }
        const uint32_t initial = Fnv(d.bits, d.stride * d.h);
        const uint32_t surface[2] = {static_cast<uint32_t>(reinterpret_cast<uintptr_t>(d.bits)), d.stride};
        // Spans: one per row, varying start and length, all inside 32 x 8 (and x < 16 for plain blits).
        uint32_t wide[16], narrow[16];
        for (uint32_t y = 0; y < 8; ++y) {
            wide[2 * y] = (y << 16) | y;
            wide[2 * y + 1] = (y << 16) | (32 - y);
            narrow[2 * y] = (y << 16) | (y & 3);
            narrow[2 * y + 1] = (y << 16) | (16 - (y & 1));
        }
        reinterpret_cast<SolidFn>(vt[1])(&obj, nullptr, wide, 8, surface, 0x12347C1F);
        reinterpret_cast<SolidFn>(vt[1])(&obj, nullptr, narrow, 4, surface, 0x0000801F);
        const uint32_t alphas[3][2] = {{0x7FFF, 0x80000000}, {0x1234, 0xFE000000}, {0x4321, 0x05FFFFFF}};
        for (const auto& a : alphas) reinterpret_cast<AlphaFn>(vt[3])(&obj, nullptr, wide, 8, surface, a[0], a[1]);
        const int32_t grad[12] = {0x40000, 0x80000, 0xC0000, 0x100000, 0x8000, -0x4000, 0x2000, 0x1000,
                                  0x10000, 0x8000,  -0x8000, 0x4000};
        reinterpret_cast<GradFn>(vt[2])(&obj, nullptr, wide, 8, surface, grad);
        reinterpret_cast<GradFn>(vt[4])(&obj, nullptr, wide, 8, surface, grad);
        casesRun += 7;
        for (uint32_t st : kSrcTypes) {
            Buf s;
            if (!MakeBuffer(16, 16, st, st == 9 ? 32 : 16, 0x800, &s)) {
                FreeBuffer(&s, 0x800);
                continue;
            }
            const uint32_t idx = FormatIndex(st);
            uint32_t texel0 = 0;
            memcpy(&texel0, s.bits, s.bpp);
            const uint32_t sbits = static_cast<uint32_t>(reinterpret_cast<uintptr_t>(s.bits));
            for (uint32_t key = 0; key < 2; ++key) {
                for (uint32_t kind = 0; kind < 12; ++kind) {
                    uint32_t rec[6] = {sbits, s.stride, 16, 16, st, s.bpp};
                    uint32_t b[8] = {0, 0, 0, 0, idx, 0x80C08040, key ? (texel0 & KeyMask(idx)) : 0, key};
                    const uint32_t* sp = narrow;
                    if (kind >= 8) {  // tiled (§7.6): an 8 x 8 tile at (2, 3), u0 = 3, v0 = 5
                        rec[0] = sbits + 2 * s.bpp + 3 * s.stride;
                        rec[2] = rec[3] = 8;
                        b[0] = 3;
                        b[1] = 5;
                        sp = wide;
                    } else if (kind >= 4) {  // stretch (§7.5): half scale, source column x / 2 < 16
                        b[2] = b[3] = 0x8000;
                        sp = wide;
                    }
                    reinterpret_cast<BltFn>(vt[5])(&obj, nullptr, sp, 8, surface, rec, b, kind);
                    ++casesRun;
                }
            }
            FreeBuffer(&s, 0x800);
        }
        Log("span1555: fixture (constructed buffer, not a game path): destination type %u 32x8 stride %u: "
            "pixels fnv 0x%08X -> 0x%08X",
            dt, d.stride, initial, Fnv(d.bits, d.stride * d.h));
        FreeBuffer(&d, 0x8040);
    }
    t_fixture = false;
    uint32_t exact = 0, diff = 0, err = 0;
    for (unsigned i = 1; i <= 5; ++i) {
        const SlotStats& s = g.s[i];
        exact += s.exact;
        diff += s.diff;
        err += s.oursErr;
        Log("span1555: %s calls=%u exact=%u diff=%u ours_refused=%u skipped=%u game_calls=%u changed_bytes=%llu",
            kSlotName[i], s.calls.load(), s.exact.load(), s.diff.load(), s.oursErr.load(), s.skipped.load(),
            s.gameCalls.load(), static_cast<unsigned long long>(s.changedBytes.load()));
    }
    Log("span1555: fixture total (constructed buffer): cases=%u exact=%u diff=%u ours_refused=%u -> %s", casesRun,
        exact, diff, err, (diff == 0 && err == 0 && exact > 0) ? "EXACT" : "MISMATCH");
}

#endif  // _M_IX86

}  // namespace

void Span1555Startup(bool enabled) {
    if (!enabled) return;
#if defined(_M_IX86)
    g.installed = Install();
    if (g.installed) {
        Log("span1555: ON (enable=span1555): vtable 0x00AC182C slots 1-5 forward to ours, compared with the "
            "original on every call");
    }
#else
    Log("span1555: not installed (x86 only)");
#endif
}

void Span1555CityLoaded() {
#if defined(_M_IX86)
    if (!g.installed || g.fixtureDone) return;
    g.fixtureDone = true;
    RunFixture();
#endif
}

}  // namespace fc4
