// SPDX-License-Identifier: GPL-3.0-or-later
// Included inside FC4SpanContext.cpp's native fixture namespace; only 1555 calls this.
// span-render-1555 §§1.3,6.1 explicitly share the setters specified by 4444 SP6.
uint32_t Span1555SourceReferences(cIGZBuffer* b) {
    // Buffer reference storage has no published offset. Observe only the public
    // interface, balancing our own probe reference before returning its count.
    FC4_ADDREF(b);
    return FC4_RELEASE(b);
}

void Run1555SourceCases(void* ctx, Buf& dst, const std::vector<uint8_t>& init, Totals& totals) {
    struct Placement {
        float x, y, sx, sy;
        int32_t ox, oy, du, dv;
        int32_t rect[4];
    };
    const Placement placements[] = {
        {3, 5, 1, 1, 3, 5, 0x10000, 0x10000, {3, 5, 11, 11}},
        {3, 5, 2, .5f, 0x18000, 0xA0000, 0x8000, 0x20000, {3, 5, 19, 8}},
        {3, 5, .5f, 2, 0x60000, 0x28000, 0x20000, 0x8000, {3, 5, 7, 17}},
        {3, 5, .75f, .75f, 0x3FFFF, 0x6AAA9, 0x15555, 0x15555, {3, 5, 9, 9}},
        {3.5f, 5.5f, 1, 1, 4, 6, 0x10000, 0x10000, {4, 6, 12, 12}},
        {3.5f, 5.5f, 2, 2, 0x1C000, 0x2C000, 0x8000, 0x8000, {3, 5, 19, 17}},
    };
    uint32_t checks = 0, failures = 0;
    for (uint32_t type : {2u, 4u, 5u, 6u, 7u, 8u, 9u}) {
        Buf src;
        if (!MakeBuf(8, 6, type, type == 2 ? 8 : type >= 8 ? 32 : 16, kTexLock, 500 + type, &src)) {
            ++failures;
            FreeBuf(&src);
            continue;
        }
        const auto refs = Span1555SourceReferences(src.b);
        const auto locks = LockCount(src), flags = LockFlags(src);
        for (unsigned keyed = 0; keyed < (type == 2 ? 1u : 2u); ++keyed) {
            uint32_t raw = 0;
            std::memcpy(&raw, src.bits, type >= 8 ? 4 : type == 2 ? 1 : 2);
            if (keyed) src.b->SetTransparency(raw);
            else src.b->RemoveTransparency();
            for (const auto& p : placements) {
                int32_t rect[4] = {-111, 222, -333, 444};
                const uint32_t spans[] = {uint32_t(p.rect[1]) << 16 | uint32_t(p.rect[0]),
                                          uint32_t(p.rect[1]) << 16 | uint32_t(p.rect[2])};
                Case c{kTexture, 0x80C0A040, nullptr, 0, 0, &src, p.sx, p.sy, false, 0, 0};
                c.x = p.x; c.y = p.y; c.outRect = rect; c.spans = spans; c.count = 1;
                for (unsigned mode = 0; mode < 4; ++mode) {
                    c.alphaBlend = mode & 1; c.modulate = mode >> 1;
                    RunCase(kSpanR1555, ctx, dst, init, c, totals);
                    bool ok = std::memcmp(rect, p.rect, sizeof(rect)) == 0;
                    ok = ok && Field<int32_t>(ctx, 0x28) == p.ox && Field<int32_t>(ctx, 0x2C) == p.oy;
                    ok = ok && Field<int32_t>(ctx, 0x38) == p.du && Field<int32_t>(ctx, 0x3C) == p.dv;
                    ok = ok && Field<int32_t>(ctx, 0x30) == int32_t(p.sx * 65536) &&
                        Field<int32_t>(ctx, 0x34) == int32_t(p.sy * 65536);
                    ok = ok && Field<uint8_t>(ctx, 0x40) == (p.sx != 1 || p.sy != 1);
                    ok = ok && Field<int32_t>(ctx, 0xA8) == rect[2] - rect[0];
                    ok = ok && Field<int32_t>(ctx, 0xAC) == rect[3] - rect[1];
                    const auto heldRefs = Span1555SourceReferences(src.b);
                    ok = ok && Field<void*>(ctx, 0x24) == src.b && heldRefs == refs + 1;
                    ok = ok && LockCount(src) == locks && LockFlags(src) == flags;
                    if (!ok && keyed == 0 && type == 2 && mode == 0)
                        Log("spanctx: span1555 SP6 FAIL placement (%g,%g,%g,%g): rect=%d,%d,%d,%d "
                            "ox/oy=%08X,%08X du/dv=%08X,%08X forward=%08X,%08X mode=%u refs=%u base=%u",
                            p.x, p.y, p.sx, p.sy, rect[0], rect[1], rect[2], rect[3],
                            Field<uint32_t>(ctx, 0x28), Field<uint32_t>(ctx, 0x2C),
                            Field<uint32_t>(ctx, 0x38), Field<uint32_t>(ctx, 0x3C),
                            Field<uint32_t>(ctx, 0x30), Field<uint32_t>(ctx, 0x34),
                            Field<uint8_t>(ctx, 0x40), heldRefs, refs);
                    ++checks; failures += !ok;
                }
            }
            int32_t tile[] = {2, 1, 6, 4};
            const uint32_t spans[] = {0x00050003, 0x00050008};
            Case c{kTexture, 0x80C0A040, nullptr, 0, 0, &src, 1, 1, true, 0, 0};
            c.x = 3; c.y = 5; c.tileRect = tile; c.spans = spans; c.count = 1;
            for (unsigned mode = 0; mode < 4; ++mode) {
                c.alphaBlend = mode & 1; c.modulate = mode >> 1;
                RunCase(kSpanR1555, ctx, dst, init, c, totals);
                ++checks;
                failures += tile[0] != 2 || tile[1] != 1 || tile[2] != 6 || tile[3] != 4;
            }
        }
        int32_t sentinel[] = {-111, 222, -333, 444};
        const uint8_t cleared = VSlot<BltSourceFn>(ctx, 13)(ctx, nullptr, nullptr, 0, 0, 1, 1, sentinel);
        const bool balanced = cleared == 0 && Field<void*>(ctx, 0x24) == nullptr &&
            Span1555SourceReferences(src.b) == refs && LockCount(src) == locks && LockFlags(src) == flags &&
            sentinel[0] == -111 && sentinel[1] == 222 && sentinel[2] == -333 && sentinel[3] == 444;
        ++checks; failures += !balanced;
        Log("spanctx: span1555 source type %u keyed_variants=%u setter/lifetime=%s Release -> %u",
            type, type == 2 ? 1u : 2u, balanced ? "balanced" : "FAILED", FreeBuf(&src));
    }
    Log("spanctx: span1555 SP6 source contracts: checks=%u failures=%u (nonorigin/scales/rounding/tile/null/ref/lock)",
        checks, failures);
    totals.routeMismatch += failures;
}
