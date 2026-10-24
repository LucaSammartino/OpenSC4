// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstddef>
#include <cstdint>
#include <cstring>

namespace fc4::netrules {
// docs/abi/network-rules-observation.md, NR17. No wildcard/relocation bytes.
inline constexpr uintptr_t kBase = 0x00400000;
inline constexpr uintptr_t kSites[] = {0x006220D0, 0x006253D0, 0x006232F0};
inline constexpr size_t kLengths[] = {5, 6, 5};
inline constexpr uint8_t kBytes[3][6] = {
    {0x53, 0x8B, 0x5C, 0x24, 0x0C, 0},
    {0x81, 0xEC, 0x80, 0, 0, 0},
    {0xA0, 0xA8, 0x52, 0xB4, 0, 0},
};
inline bool Reference(uintptr_t base, const uint8_t spans[3][6]) {
    if (base != kBase) return false;
    for (size_t i = 0; i < 3; ++i)
        if (std::memcmp(spans[i], kBytes[i], kLengths[i])) return false;
    return true;
}
// Readiness is independent of retained table storage and process flag value.
struct Epoch {
    uint32_t number = 0;
    bool ready = false;
    void Invalidate() { ++number; ready = false; }
    void Complete() { ready = true; }
};
}  // namespace fc4::netrules
