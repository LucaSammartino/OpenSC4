// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <algorithm>
#include <array>
#include <cstdint>
#include <vector>

namespace fc4 {
// city-situation.md section 7: hash-map iteration order is not persistent
// state. Compare every byte, canonicalizing only the six-byte map entries.
inline bool CitySitRecordEqual(std::vector<uint8_t> a, std::vector<uint8_t> b) {
    const auto canonicalize = [](std::vector<uint8_t>& bytes) {
        if (bytes.size() < 7 || bytes[0] != 2 || bytes[1] != 0) return false;
        const uint32_t n = uint32_t(bytes[2]) | uint32_t(bytes[3]) << 8 |
            uint32_t(bytes[4]) << 16 | uint32_t(bytes[5]) << 24;
        if (n != (bytes.size() - 7) / 6 || (bytes.size() - 7) % 6 != 0) return false;
        std::vector<std::array<uint8_t, 6>> entries(n);
        for (size_t i = 0; i < n; ++i)
            std::copy_n(bytes.begin() + 6 + 6 * i, 6, entries[i].begin());
        std::sort(entries.begin(), entries.end());
        for (size_t i = 0; i < n; ++i)
            std::copy(entries[i].begin(), entries[i].end(), bytes.begin() + 6 + 6 * i);
        return true;
    };
    return canonicalize(a) && canonicalize(b) && a == b;
}
}
