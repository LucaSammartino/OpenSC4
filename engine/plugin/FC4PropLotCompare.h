// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "fc4.h"
#include <cstring>
#include <vector>

namespace fc4 {
namespace proplot {
struct Comparison {
    bool rawEqual;
    bool definedEqual;
    unsigned fixups;
};

// PL23: preserve raw evidence; only absent/three-value size permits this mask.
inline Comparison Profile(const unsigned char* a, const unsigned char* b, bool ordinarySize) {
    Comparison r{true, true, 0};
    for (unsigned i = 0; i < 0x1C; ++i) {
        if (a[i] == b[i]) continue;
        r.rawEqual = false;
        if (!(ordinarySize && i >= 0x15 && i <= 0x17)) r.definedEqual = false;
    }
    r.fixups = ordinarySize && std::memcmp(a + 0x15, b + 0x15, 3) != 0 ? 1u : 0u;
    return r;
}

// Caller proves this is the final appended record from the foundation property.
// Ordinary property-derived type-3 records do not qualify. All other fields,
// including the foundation type and both empty ID lists, remain checked.
inline Comparison Object(const fc4_lotdev_cfg_object& a, const std::vector<uint32_t>& ai,
                         const fc4_lotdev_cfg_object& b, const std::vector<uint32_t>& bi,
                         bool appended) {
    const bool fields = a.object_type == b.object_type && a.flags == b.flags &&
        a.orientation == b.orientation && a.byte3 == b.byte3 &&
        std::memcmp(a.offset, b.offset, sizeof(a.offset)) == 0 &&
        std::memcmp(a.footprint_max, b.footprint_max, sizeof(a.footprint_max)) == 0 &&
        std::memcmp(a.footprint_min, b.footprint_min, sizeof(a.footprint_min)) == 0 &&
        a.foundation == b.foundation && ai == bi;
    const bool unspecified = appended && a.object_type == 3 && b.object_type == 3 && ai.empty() && bi.empty();
    return {fields && a.tag == b.tag, fields && (unspecified || a.tag == b.tag),
            unspecified && a.tag != b.tag ? 1u : 0u};
}
} // namespace proplot
} // namespace fc4
