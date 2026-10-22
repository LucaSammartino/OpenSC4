// SPDX-License-Identifier: GPL-3.0-or-later
// Included in the 565 bridge's private namespace. No context/buffer fields are read.
// Original mutates the native target once. Rust receives independent, guarded
// copies of the complete padded preimage and source allocation, never its output.
constexpr uintptr_t kContextReturns565[] = {
    0x0083064A, 0x008305FF, 0x0083062A, 0x008305EA,
    0x00830594, 0x0083053E, 0x008304D4};  // 4444 §1.3.2, shared 2D context
volatile LONG g_callers565[4][6] = {};  // constructed native / direct / natural native / other
volatile LONG g_sites565[7] = {};
int g_expectedSite565 = -1;
uint64_t g_cases565 = 0, g_exact565 = 0, g_diff565 = 0, g_refused565 = 0;
uint64_t g_bytes565 = 0, g_changed565 = 0, g_noop565 = 0, g_contract565 = 0;
uint64_t g_unchanged565 = 0, g_unaccounted565 = 0;
uint64_t g_rows565[14][12] = {};
uint8_t* g_dst565 = nullptr;
size_t g_dstBytes565 = 0;
const uint8_t* g_src565 = nullptr;
size_t g_srcBytes565 = 0;
fc4_span565_source g_expectedSource565{};
fc4_span565_blt g_expectedBlt565{};
bool g_expectTexture565 = false;
bool g_allowNoChange565 = false;
int g_expectedSlot565 = 0;
uint32_t g_expectedKind565 = 0;

void NoteCaller(int slot, uintptr_t ret) {
    int site = -1;
    for (int i = 0; i < 7; ++i) if (ret == kContextReturns565[i]) site = i;
    const bool fixture = Origin() == 1;
    const int category = fixture ? (site >= 0 ? 0 : 1) : (site >= 0 ? 2 : 3);
    InterlockedIncrement(&g_callers565[category][slot]);
    if (fixture && site >= 0) InterlockedIncrement(&g_sites565[site]);
    if (fixture && (site != g_expectedSite565 || slot != g_expectedSlot565)) ++g_contract565;
}

template<class Native, class Rust>
void Measure565(int slot, const uint32_t* spans, uint32_t n, const fc4_span565_dest* dst,
                const fc4_span565_source* src, const fc4_span565_blt* blt, uint32_t kind,
                Native native, Rust rust) {
    ++g_cases565;
    if (!dst || dst->pixels != g_dst565 || dst->stride != 36 || dst->width != 17 ||
        dst->height != 12 || dst->color_type != 5 || dst->bytes_per_pixel != 2 ||
        !spans || !n || n > 8 || bool(src) != g_expectTexture565) {
        ++g_contract565;
        ++g_unaccounted565;
        native();
        return;
    }
    if (src) {
        // All six source dwords are defined. Plain geometry words, tiled
        // steps, an inactive key and padding are not consumed by the route;
        // capture them verbatim for Rust without imposing invented values.
        const bool geometry = blt && (g_expectedSite565 == 4 ||
            (blt->u0 == g_expectedBlt565.u0 && blt->v0 == g_expectedBlt565.v0 &&
             (g_expectedSite565 == 6 || (blt->du == g_expectedBlt565.du && blt->dv == g_expectedBlt565.dv))));
        if (std::memcmp(src, &g_expectedSource565, sizeof(*src)) || !blt || !geometry ||
            blt->format != g_expectedBlt565.format || blt->argb != g_expectedBlt565.argb ||
            (blt->key_flag && blt->key != g_expectedBlt565.key) ||
            blt->key_flag != g_expectedBlt565.key_flag || kind != g_expectedKind565) {
            if (g_contract565 < 8 && blt) Log("span565 producer: case=%llu site=%d source=%p/%d/%u/%u/%u/%u geometry=%08X/%08X/%08X/%08X kind=%u key=%08X/%u expected_key=%08X/%u",
                g_cases565,g_expectedSite565,src->texels,src->stride,src->width,src->height,src->unread[0],src->unread[1],
                blt->u0,blt->v0,blt->du,blt->dv,kind,blt->key,blt->key_flag,g_expectedBlt565.key,g_expectedBlt565.key_flag);
            ++g_contract565;
        }
        if (!blt || blt->format > 6 || kind > 11 || blt->key_flag > 1) {
            ++g_unaccounted565;
            native();
            return;
        }
        ++g_rows565[2 * blt->format + blt->key_flag][kind];
    }
    std::vector<uint8_t> before(g_dst565, g_dst565 + g_dstBytes565);
    std::vector<uint8_t> copy(g_dstBytes565 + 64, 0xD3);
    std::memcpy(copy.data() + 32, before.data(), before.size());
    fc4_span565_dest rd = *dst;
    rd.pixels = copy.data() + 32;
    const fc4_span565_dest savedDst = *dst;
    const std::vector<uint32_t> savedSpans(spans, spans + 2 * n);
    std::vector<uint8_t> source;
    std::vector<uint8_t> sourceBefore;
    fc4_span565_source rs{};
    fc4_span565_blt savedBlt{};
    if (src) {
        source.assign(g_srcBytes565 + 64, 0xE7);
        std::memcpy(source.data() + 32, g_src565, g_srcBytes565);
        sourceBefore.assign(g_src565, g_src565 + g_srcBytes565);
        rs = *src;
        savedBlt = *blt;
    }
    native();  // original authoritative; no second host draw, restore, or resync
    int status;
    if (src) {
        const uintptr_t bias = reinterpret_cast<uintptr_t>(rs.texels) - reinterpret_cast<uintptr_t>(g_src565);
        rs.texels = reinterpret_cast<const uint8_t*>(reinterpret_cast<uintptr_t>(source.data() + 32) + bias);
        // Rust uses the captured source, including plain pointer bias and padding.
        status = fc4_span565_render_blt(savedSpans.data(), n, &rd, &rs, &savedBlt, kind);
        if (std::memcmp(source.data() + 32, sourceBefore.data(), g_srcBytes565) ||
            std::memcmp(g_src565, sourceBefore.data(), g_srcBytes565) ||
            std::memcmp(src, &g_expectedSource565, sizeof(*src)) ||
            std::memcmp(blt, &savedBlt, sizeof(savedBlt))) ++g_contract565;
        for (size_t i = 0; i < 32; ++i)
            if (source[i] != 0xE7 || source[source.size()-1-i] != 0xE7) ++g_contract565;
    } else {
        status = rust(&rd,savedSpans.data());
    }
    if (status != FC4_SPAN565_OK) ++g_refused565;  // no fallback permitted in measured branch
    if (std::memcmp(dst, &savedDst, sizeof(*dst)) ||
        std::memcmp(spans, savedSpans.data(), savedSpans.size()*4)) ++g_contract565;
    std::vector<uint8_t> writable(g_dstBytes565, 0);
    for (uint32_t i = 0; i < n; ++i) {
        const uint32_t y = spans[2*i] >> 16, x0 = spans[2*i] & 0xFFFF, x1 = spans[2*i+1] & 0xFFFF;
        if (y >= 12 || (spans[2*i+1] >> 16) != y || x0 > x1 || x1 > 17) { ++g_contract565; continue; }
        for (uint32_t x = 2*x0; x < 2*x1; ++x) writable[y*36+x] = 1;
    }
    size_t diff = 0, changed = 0;
    for (size_t i = 0; i < before.size(); ++i) {
        diff += g_dst565[i] != copy[32+i];
        changed += g_dst565[i] != before[i];
        if (!writable[i] && (g_dst565[i] != before[i] || copy[32+i] != before[i])) ++g_contract565;
    }
    for (size_t i = 0; i < 32; ++i)
        if (copy[i] != 0xD3 || copy[copy.size()-1-i] != 0xD3) ++g_contract565;
    g_bytes565 += before.size();
    g_changed565 += changed;
    if (diff) ++g_diff565;
    else if (status == FC4_SPAN565_OK) ++g_exact565;
    // Explicit zero-alpha/empty-span controls are separate from changed cases.
    const bool noop = g_allowNoChange565;
    if (noop) { ++g_noop565; if (changed) ++g_contract565; }
    else if (!changed) ++g_unchanged565;
    if (diff || status != FC4_SPAN565_OK)
        Log("span565 compare: DIFF case=%llu slot=%d kind=%u row=%u bytes=%u different=%u status=%d",
            g_cases565, slot, kind, blt ? 2*blt->format+blt->key_flag : 0,
            unsigned(before.size()), unsigned(diff), status);
}
