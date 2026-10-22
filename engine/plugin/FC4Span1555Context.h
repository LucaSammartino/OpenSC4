// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include <cstdint>

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
}  // namespace fc4
