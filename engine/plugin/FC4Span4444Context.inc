// SPDX-License-Identifier: GPL-3.0-or-later
// 4444-only native conformance for span-render-4444.md §1.3.1/§1.3.2.
// Included inside FC4SpanContext's private namespace to reuse its native helpers.
// These are constructed calls, never evidence of a shipped type-7 producer (SP4).

class Refusing4444Source final : public cIGZUnknown {
public:
    bool QueryInterface(uint32_t, void** out) override { *out = nullptr; return false; }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
};

void Run4444Context(void* ctx, Buf& dst, const std::vector<uint8_t>& init, const compat::SpanContextVA& va) {
    uint64_t cases = 0, exact = 0, bytes = 0, changed = 0, refused = 0, failures = 0;
    unsigned classifierChecks = 0;
    for (uintptr_t site : va.sites)
        for (unsigned delta = 0; delta <= 7; ++delta) {
            ++classifierChecks;
            if ((SiteOf(site + delta, kSpanR4444) >= 0) != (delta == 3)) ++failures;
        }
    auto compare = [&](const uint32_t* spans, uint32_t count) {
        DrawCall call{ctx, spans, count};
        const size_t len = init.size();
        std::memcpy(dst.bits, init.data(), len);
        UseOriginal(kSpanR4444, true);
        g_activeTid.store(GetCurrentThreadId());
        const bool aok = SehGuard("span4444 SP6 original context", &DrawGuarded, &call);
        g_activeTid.store(0);
        const std::vector<uint8_t> expected(dst.bits, dst.bits + len);
        std::memcpy(dst.bits, init.data(), len);
        UseOriginal(kSpanR4444, false);
        const uint64_t before = g_refused[kSpanR4444].load();
        g_activeTid.store(GetCurrentThreadId());
        const bool bok = SehGuard("span4444 SP6 ours context", &DrawGuarded, &call);
        g_activeTid.store(0);
        ++cases;
        bytes += len;
        refused += g_refused[kSpanR4444].load() - before;
        size_t diff = 0;
        for (size_t i = 0; i < len; ++i) {
            changed += expected[i] != init[i];
            diff += expected[i] != dst.bits[i];
        }
        if (aok && bok && diff == 0 && g_refused[kSpanR4444].load() == before) ++exact;
        else Log("span4444 SP6 case=%llu differing_bytes=%zu original_ok=%u ours_ok=%u", cases, diff, aok, bok);
    };
    // Published 8x6 examples: scales, reciprocal steps, placement, output edges,
    // and first sampled texel. All spans remain within the real allocations.
    struct Placement {
        float x, y, sx, sy;
        uint32_t fx, fy, du, dv, ox, oy;
        int32_t rect[4];
        uint32_t px, py;
    };
    const Placement placements[] = {
        {3,5,1,1, 0x10000,0x10000,0x10000,0x10000,3,5, {3,5,11,11},3,5},
        {3,5,2,.5f, 0x20000,0x8000,0x8000,0x20000,0x18000,0xA0000, {3,5,19,8},3,5},
        {3,5,.5f,2, 0x8000,0x20000,0x20000,0x8000,0x60000,0x28000, {3,5,7,17},3,5},
        {3,5,.75f,.75f, 0xC000,0xC000,0x15555,0x15555,0x3FFFF,0x6AAA9, {3,5,9,9},3,5},
        {3.5f,5.5f,1,1, 0x10000,0x10000,0x10000,0x10000,4,6, {4,6,12,12},4,6},
        {3.5f,5.5f,2,2, 0x20000,0x20000,0x8000,0x8000,0x1C000,0x2C000, {3,5,19,17},3,5},
    };
    for (uint32_t type : {2u,4u,5u,6u,7u,8u,9u}) {
        Buf src;
        if (!MakeBuf(8, 6, type, type == 2 ? 8 : type >= 8 ? 32 : 16, kTexLock, 700 + type, &src)) {
            ++failures;
            FreeBuf(&src);
            continue;
        }
        const uint16_t lock0 = LockCount(src), flags0 = LockFlags(src);
        const uint32_t refs0 = Field<uint32_t>(src.b, 4);
        for (const auto& p : placements) {
            int32_t rect[4] = {111,222,333,444};
            const uint8_t accepted = VSlot<BltSourceFn>(ctx, 13)(ctx, nullptr, src.b, p.x,p.y,p.sx,p.sy,rect);
            const uint32_t expected[] = {p.ox,p.oy,p.fx,p.fy,p.du,p.dv};
            bool state = accepted == 1 && Field<void*>(ctx, 0x24) == src.b;
            for (unsigned i = 0; i < 6; ++i) state &= Field<uint32_t>(ctx, 0x28 + 4*i) == expected[i];
            state &= std::memcmp(rect, p.rect, sizeof(rect)) == 0;
            state &= Field<uint32_t>(ctx, 0xA8) == uint32_t(rect[2]-rect[0]);
            state &= Field<uint32_t>(ctx, 0xAC) == uint32_t(rect[3]-rect[1]);
            state &= Field<uint8_t>(ctx, 0x40) == (p.fx != 0x10000 || p.fy != 0x10000);
            if (!state) ++failures;
            const uint32_t spans[] = {(p.py<<16)|p.px, (p.py<<16)|(p.px+1)};
            const uint32_t argb = 0x80C0A040;
            VSlot<ColorTableFn>(ctx, 7)(ctx, nullptr, 0,1,&argb);
            VSlot<GradientFn>(ctx, 12)(ctx, nullptr, nullptr,0,0);
            VSlot<CtxVoidFn>(ctx, 14)(ctx, nullptr);
            uint32_t key = 0;
            std::memcpy(&key, src.bits, type == 2 ? 1 : type >= 8 ? 4 : 2);
            // Native key setter/getter, including source alpha bits. The draw
            // consumer masks the key according to the published format table.
            for (unsigned keyed = 0; keyed < 2; ++keyed) {
                if (keyed) src.b->SetTransparency(key);
                else src.b->RemoveTransparency();
                for (uint32_t op = 0; op < 4; ++op) {
                    VSlot<DwordFn>(ctx, 8)(ctx, nullptr, op & 1);
                    VSlot<DwordFn>(ctx, 9)(ctx, nullptr, op >> 1);
                    compare(spans, 1);
                    if (type == 7 && op == 0 && !keyed) {
                        const unsigned index = unsigned(&p - placements);
                        const unsigned tx = index == 2 ? 1 : 0, ty = index == 1 ? 1 : 0;
                        uint16_t sample, actual;
                        std::memcpy(&sample, src.bits + ty*src.stride + 2*tx, 2);
                        std::memcpy(&actual, dst.bits + p.py*dst.stride + 2*p.px, 2);
                        if (sample != actual) ++failures;
                    }
                }
            }
            Log("span4444 SP6 source_type=%u placement=%.2f,%.2f scale=%.2f,%.2f state=%s rect=%d,%d,%d,%d key=%08X",
                type,p.x,p.y,p.sx,p.sy,state ? "exact" : "DIFF",rect[0],rect[1],rect[2],rect[3],key);
        }
        // Nonorigin tile, source edge rectangle, tiling overrides stretching.
        int32_t rect[4] = {};
        VSlot<BltSourceFn>(ctx, 13)(ctx,nullptr,src.b,3,5,1,1,rect);
        const int32_t tile[] = {2,1,6,4};
        VSlot<RectFn>(ctx,15)(ctx,nullptr,tile);
        const uint32_t tileSpans[] = {0x50003,0x50005,0x50007,0x50008};
        src.b->RemoveTransparency();
        VSlot<DwordFn>(ctx,8)(ctx,nullptr,0);
        VSlot<DwordFn>(ctx,9)(ctx,nullptr,0);
        compare(tileSpans,2);
        if (type == 7) {
            const unsigned dx[] = {3,4,7}, tx[] = {2,3,2};
            for (unsigned i = 0; i < 3; ++i) {
                uint16_t sample, actual;
                std::memcpy(&sample, src.bits + src.stride + 2*tx[i], 2);
                std::memcpy(&actual, dst.bits + 5*dst.stride + 2*dx[i], 2);
                if (sample != actual) ++failures;
            }
        }
        if (std::memcmp(tile, static_cast<uint8_t*>(ctx)+0x94, sizeof(tile)) != 0) ++failures;
        VSlot<BltSourceFn>(ctx,13)(ctx,nullptr,src.b,3,5,2,.5f,rect);
        compare(tileSpans,2);
        VSlot<CtxVoidFn>(ctx,14)(ctx,nullptr);
        // Null and failed-QI paths leave output/state untouched except +0x24.
        int32_t sentinel[] = {111,222,333,444};
        const int32_t saved[] = {111,222,333,444};
        uint8_t savedState[0x19];
        std::memcpy(savedState, static_cast<uint8_t*>(ctx)+0x28, sizeof(savedState));
        if (VSlot<BltSourceFn>(ctx,13)(ctx,nullptr,nullptr,0,0,1,1,sentinel) != 0) ++failures;
        Refusing4444Source bad;
        if (VSlot<BltSourceFn>(ctx,13)(ctx,nullptr,&bad,0,0,1,1,sentinel) != 1) ++failures;
        if (std::memcmp(sentinel,saved,sizeof(saved)) || Field<void*>(ctx,0x24)) ++failures;
        if (std::memcmp(savedState,static_cast<uint8_t*>(ctx)+0x28,sizeof(savedState))) ++failures;
        const bool balanced = LockCount(src) == lock0 && LockFlags(src) == flags0 && Field<uint32_t>(src.b,4) == refs0;
        if (!balanced) ++failures;
        const uint32_t left = FreeBuf(&src);
        if (left != 0) ++failures;
        Log("span4444 SP6 source_type=%u locks_refs=%s Release=%u",type,balanced ? "balanced" : "DIFF",left);
    }
    Log("span4444 SP6 TOTAL constructed_native_context cases=%llu exact=%llu compared_bytes=%llu changed_bytes=%llu refused=%llu contract_failures=%llu classifier_checks=%u -> %s",
        cases,exact,bytes,changed,refused,failures,classifierChecks,
        cases && cases == exact && !refused && !failures ? "EXACT" : "NOT exact");
}
