// SPDX-License-Identifier: GPL-3.0-or-later
// Clean-room source: span-render-8 §§1–7, 9, 10.1–10.7; public setters in span-render-565 §1.3.
#include "FC4SpanRender8.h"
#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Subsystems.h"
#include "compat/FC4Compat.h"
#include "fc4.h"
#include "cIGZBuffer.h"
#include "cIGZCOM.h"
#include "cIGZFrameWork.h"
#include "cRZCOMDllDirector.h"
#include <windows.h>
#include <intrin.h>
#include <atomic>
#include <cstring>
#include <vector>

namespace fc4 {
namespace {
// Exact thiscall stack: the EDX dummy occupies a register, never a stack word.
using Solid = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t, const uint32_t*, uint32_t);
using Grad = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t, const uint32_t*, const uint32_t*);
using Alpha = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t, const uint32_t*, uint32_t, uint32_t);
using Blt = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t, const uint32_t*, const uint32_t*, const uint32_t*, uint32_t);
struct Count { std::atomic<uint64_t> calls{0}, ours{0}, exact{0}, diff{0}, refusal{0}, fallback{0}; };
Count counts[4][6]; // constructed direct, constructed native context, shipped context, other
std::atomic<uint64_t> kernels[4][13][12];
Count kernelResults[4][13][12];
thread_local Count* fixtureKernel = nullptr;
thread_local unsigned fixtureClass = 3;
thread_local bool bypass = false;
thread_local uint64_t fixtureRefusals = 0, fixtureCalls = 0;
void** table = nullptr;
void* originals[5] = {};
compat::SpanContextVA va = {};
bool shadow = false, fixtureDone = false;
uintptr_t delta = 0;

bool Window(const void* p, size_t n, bool write = false) {
    uintptr_t a = reinterpret_cast<uintptr_t>(p), end = a + n;
    if (!a || !n || n > 64u * 1024u * 1024u || end < a) return false;
    while (a < end) {
        MEMORY_BASIC_INFORMATION m = {};
        if (!VirtualQuery(reinterpret_cast<void*>(a), &m, sizeof(m)) || m.State != MEM_COMMIT ||
            (m.Protect & (PAGE_GUARD | PAGE_NOACCESS))) return false;
        DWORD prot = m.Protect & 255;
        if (prot != PAGE_READWRITE && prot != PAGE_WRITECOPY && (write || prot != PAGE_READONLY)) return false;
        uintptr_t next = reinterpret_cast<uintptr_t>(m.BaseAddress) + m.RegionSize;
        if (next <= a) return false;
        a = next;
    }
    return true;
}

// Owned policy: bound to the complete described arena, including row padding.
// Prebiased/invalid native records are refused; their original call is retained.
bool Image(const uint32_t* p, FC4Span8Image& out, bool write) {
    if (!Window(p, 24)) return false;
    int64_t stride = static_cast<int32_t>(p[1]);
    if (!p[2] || !p[3] || stride <= 0 || stride > 0x100000 || p[3] > 0x10000) return false;
    uint64_t len = uint64_t(stride) * p[3];
    if (len > 64u * 1024u * 1024u) return false;
    out = {reinterpret_cast<uint8_t*>(p[0]), size_t(len), 0, int32_t(stride), p[2], p[3]};
    return Window(out.bytes, out.len, write);
}

struct Call {
    void* self;
    uint32_t slot;
    const uint32_t* spans;
    uint32_t n;
    const uint32_t* dst;
    uint32_t colour = 0, argb = 0;
    const uint32_t* grad = nullptr;
    const uint32_t* src = nullptr;
    const uint32_t* params = nullptr;
    uint32_t kind = 0;
};
void Original(const Call& c) {
    const bool before = bypass;
    bypass = true; // §3/§5 redispatch: retain one logical call, use original sibling slot.
    switch (c.slot) {
        case 1: reinterpret_cast<Solid>(originals[0])(c.self, nullptr, c.spans, c.n, c.dst, c.colour); break;
        case 2: case 4: reinterpret_cast<Grad>(originals[c.slot - 1])(c.self, nullptr, c.spans, c.n, c.dst, c.grad); break;
        case 3: reinterpret_cast<Alpha>(originals[2])(c.self, nullptr, c.spans, c.n, c.dst, c.colour, c.argb); break;
        default: reinterpret_cast<Blt>(originals[4])(c.self, nullptr, c.spans, c.n, c.dst, c.src, c.params, c.kind); break;
    }
    bypass = before;
}
bool Job(const Call& c, FC4Span8Job& j) {
    if (!c.n || c.n > 65536 || !Window(c.spans, size_t(c.n) * 8) || !Image(c.dst, j.dst, true)) return false;
    j.slot = c.slot; j.count = c.n; j.spans = c.spans;
    j.colour = c.colour; j.argb = c.argb; j.gradient = c.grad; j.params = c.params; j.kind = c.kind;
    if ((c.slot == 2 || c.slot == 4) && !Window(c.grad, 48)) return false;
    if (c.slot != 5) return true;
    if (!Window(c.params, 32) || c.params[4] > 6 || (c.params[7] & 255) > 1 || c.kind > 11) return false;
    // Row13 is outside this bridge even when its core model could return a result (SR8-3).
    if (2 * c.params[4] + (c.params[7] & 255) == 13 || !Image(c.src, j.src, false)) return false;
    uintptr_t a = reinterpret_cast<uintptr_t>(j.dst.bytes), b = reinterpret_cast<uintptr_t>(j.src.bytes);
    return a + j.dst.len <= b || b + j.src.len <= a;
}
void Dispatch(const Call& c, uintptr_t ret) {
    if (bypass) { Original(c); return; }
    unsigned cls = fixtureClass;
    if (cls == 3 && ret >= delta + 0x008302E0 && ret <= delta + 0x0083064F) cls = 2;
    auto& t = counts[cls][c.slot];
    ++t.calls;
    if (cls < 2) ++fixtureCalls;
    FC4Span8Job j = {};
    bool ready = Job(c, j);
    Count* kt = nullptr;
    if (c.slot == 5 && Window(c.params,32) && c.params[4]<7 && (c.params[7]&255)<2 && c.kind<12) {
        unsigned row=2*c.params[4]+(c.params[7]&255);
        if(row<13) { kt=&kernelResults[cls][row][c.kind]; ++kt->calls; }
    }
    if (cls<2) fixtureKernel=kt;
    if (c.slot == 5 && ready) ++kernels[cls][2 * c.params[4] + (c.params[7] & 255)][c.kind];
    if (!ready) {
        ++t.refusal; ++t.fallback;
        if (kt) { ++kt->refusal; ++kt->fallback; }
        if (cls < 2) ++fixtureRefusals;
        Original(c);
        return;
    }
    if (shadow && cls >= 2) {
        // Independent initial destination, original retained; no resynchronisation writes.
        std::vector<uint8_t> scratch(j.dst.bytes, j.dst.bytes + j.dst.len);
        auto* actual = j.dst.bytes;
        j.dst.bytes = scratch.data();
        int status = fc4_span8_render(&j);
        Original(c);
        if (status) { ++t.refusal; ++t.fallback; if(kt) { ++kt->refusal; ++kt->fallback; } }
        else if (!std::memcmp(actual, scratch.data(), scratch.size())) { ++t.exact; if(kt) ++kt->exact; }
        else { ++t.diff; if(kt) ++kt->diff; }
    } else if (fc4_span8_render(&j) == 0) { ++t.ours; if(kt) ++kt->ours; }
    else {
        ++t.refusal; ++t.fallback;
        if (kt) { ++kt->refusal; ++kt->fallback; }
        if (cls < 2) ++fixtureRefusals;
        Original(c);
    }
}
void __fastcall F1(void* s, void*, const uint32_t* p, uint32_t n, const uint32_t* d, uint32_t c) {
    Dispatch({s, 1, p, n, d, c}, reinterpret_cast<uintptr_t>(_ReturnAddress()));
}
void __fastcall F2(void* s, void*, const uint32_t* p, uint32_t n, const uint32_t* d, const uint32_t* g) {
    Call c{s, 2, p, n, d}; c.grad = g; Dispatch(c, reinterpret_cast<uintptr_t>(_ReturnAddress()));
}
void __fastcall F3(void* s, void*, const uint32_t* p, uint32_t n, const uint32_t* d, uint32_t c, uint32_t a) {
    Dispatch({s, 3, p, n, d, c, a}, reinterpret_cast<uintptr_t>(_ReturnAddress()));
}
void __fastcall F4(void* s, void*, const uint32_t* p, uint32_t n, const uint32_t* d, const uint32_t* g) {
    Call c{s, 4, p, n, d}; c.grad = g; Dispatch(c, reinterpret_cast<uintptr_t>(_ReturnAddress()));
}
void __fastcall F5(void* s, void*, const uint32_t* p, uint32_t n, const uint32_t* d, const uint32_t* src,
                  const uint32_t* b, uint32_t k) {
    Call c{s, 5, p, n, d}; c.src = src; c.params = b; c.kind = k;
    Dispatch(c, reinterpret_cast<uintptr_t>(_ReturnAddress()));
}
bool Entries(bool original) {
    void* f[5] = {reinterpret_cast<void*>(&F1), reinterpret_cast<void*>(&F2), reinterpret_cast<void*>(&F3),
        reinterpret_cast<void*>(&F4), reinterpret_cast<void*>(&F5)};
    DWORD old = 0, unused = 0;
    if (!VirtualProtect(table + 1, 5 * sizeof(void*), PAGE_READWRITE, &old)) return false;
    for (unsigned i = 0; i < 5; ++i) table[i + 1] = original ? originals[i] : f[i];
    return VirtualProtect(table + 1, 5 * sizeof(void*), old, &unused) != 0;
}
template<class F> F Slot(void* p, unsigned i) { return reinterpret_cast<F>((*static_cast<void***>(p))[i]); }
template<class T> T Field(void* p, unsigned off) { T t; std::memcpy(&t, static_cast<uint8_t*>(p) + off, sizeof(t)); return t; }

#include "FC4Span8Fixture.inc"
} // namespace

void Span8Start() {
    size_t n = 0; const Subsystem* subs = Subsystems(&n);
    bool enabled = false;
    for (size_t i = 0; i < n; ++i) if (!std::strcmp(subs[i].key, "span8")) enabled = SubsystemEnabled(subs[i]);
    if (!enabled || !compat::SpanContextAddresses(&va)) return;
    delta = va.contextVtable - 0x00AC17D0;
    table = reinterpret_cast<void**>(delta + 0x00AC188C);
    const uintptr_t expected[6] = {0x0082FF00, 0x0085E640, 0x0085E6A0, 0x0085E720, 0x0085E740, 0x0085E750};
    for (unsigned i = 0; i < 6; ++i) if (reinterpret_cast<uintptr_t>(table[i]) != delta + expected[i]) {
        Log("span8: refused vtable slot=%u (SR9 exact key)", i); table = nullptr; return;
    }
    for (unsigned i = 0; i < 5; ++i) originals[i] = table[i + 1];
    shadow = SettingInt("span8_shadow", "FC4_SPAN8_SHADOW", 0) != 0;
    if (!Entries(false)) { table = nullptr; Log("span8: entry install failed"); return; }
    Log("span8: installed slots1-5 slot0_original=1 shadow=%u; SR8-2 reachability open, SR8-3 row13 refused, SR8-4 bypass gap", unsigned(shadow));
}
void Span8CityLoaded() {
    if (!table || fixtureDone) return;
    fixtureDone = true;
    Fixture();
}
void Span8Stop() {
    if (!table) return;
    const bool restored = Entries(true);
    for (unsigned c = 0; c < 4; ++c) {
        for (unsigned s = 1; s <= 5; ++s) {
            auto& t = counts[c][s];
            Log("span8: class=%u slot=%u calls=%llu ours=%llu shadow_exact=%llu shadow_diff=%llu refusals=%llu fallback=%llu resync=0",
                c,s,t.calls.load(),t.ours.load(),t.exact.load(),t.diff.load(),t.refusal.load(),t.fallback.load());
        }
        for (unsigned r = 0; r < 13; ++r) {
            Log("span8: class=%u format=%u key=%u kind_counts=%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu,%llu",
                c,r/2,r%2,kernels[c][r][0].load(),kernels[c][r][1].load(),kernels[c][r][2].load(),kernels[c][r][3].load(),
                kernels[c][r][4].load(),kernels[c][r][5].load(),kernels[c][r][6].load(),kernels[c][r][7].load(),
                kernels[c][r][8].load(),kernels[c][r][9].load(),kernels[c][r][10].load(),kernels[c][r][11].load());
            for(unsigned k=0;k<12;++k) {
                auto& t=kernelResults[c][r][k];
                Log("span8 kernel: class=%u format=%u key=%u kind=%u calls=%llu ours=%llu exact=%llu diff=%llu refusals=%llu fallback=%llu resync=0",
                    c,r/2,r%2,k,t.calls.load(),t.ours.load(),t.exact.load(),t.diff.load(),t.refusal.load(),t.fallback.load());
            }
        }
    }
    Log("span8: restored=%u slot0_original=1; classes0=constructed_direct 1=constructed_native 2=shipped_context 3=other; zero counts earn no credit", unsigned(restored));
    table = nullptr;
}
} // namespace fc4
