// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include <cstdint>
#include <atomic>

namespace fc4 {
// span-render-1555 §1.3 shares these context calls with 4444 §1.3.2 (SP6b).
// Deliberately renderer-specific until the sibling lanes reconcile their contracts.
constexpr int Span1555ContextSite(uintptr_t ret, const uintptr_t* sites) {
    for (int i = 0; i < 7; ++i)
        if (ret == sites[i] + 3) return i;
    return -1;
}

constexpr bool Span1555ContextSiteConformance() {
    constexpr uintptr_t sites[] = {0x00830647, 0x008305FC, 0x00830627, 0x008305E7,
                                   0x00830591, 0x0083053B, 0x008304D1};
    for (int i = 0; i < 7; ++i)
        for (int delta = 0; delta <= 7; ++delta)
            if (Span1555ContextSite(sites[i] + delta, sites) != (delta == 3 ? i : -1)) return false;
    return true;
}
static_assert(Span1555ContextSiteConformance(), "1555 context accepts only exact CALL returns");

inline std::atomic<uint64_t> gSpan1555TextureRows[2][14][12]{};  // native fixture / other; row / kind
inline std::atomic<uint64_t> gSpan1555TextureInvalid{0};
inline void Span1555NoteTexture(const uint32_t* src, const uint32_t* b, uint32_t kind, bool native) {
    constexpr uint32_t formats[] = {6, 99, 2, 3, 1, 0, 5, 4};  // source types 2..9 (§6.1)
    constexpr uint32_t masks[] = {0xFFF, 0x7FFF, 0x7FFF, 0xFFFF, 0xFFFFFF, 0xFFFFFF, 0};
    const uint32_t keyFlag = b ? uint8_t(b[7]) : 0;
    if (!src || !b || src[4] < 2 || src[4] > 9 || b[4] > 6 || kind > 11 || keyFlag > 1 ||
        formats[src[4] - 2] != b[4] || (keyFlag && (b[6] & ~masks[b[4]])) || (b[4] == 6 && keyFlag)) {
        ++gSpan1555TextureInvalid;
        return;
    }
    ++gSpan1555TextureRows[native ? 0 : 1][2 * b[4] + keyFlag][kind];
}
}  // namespace fc4
