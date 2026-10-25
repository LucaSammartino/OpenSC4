// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
#include <cstring>

namespace fc4 {
// lot-records §3.1/§3.5: borrowed primary view; no QI, AddRef or Release.
// GetCenter: interfaces.tsv CQ3 confirms the x87 stack is empty on return;
// consume only its output vector. GetAltitude returns a float in ST0.
inline uint32_t CompareLotRecordView(void* lot, const uint32_t (&expected)[5]) {
    auto** slots = *static_cast<void***>(lot);
    using Center = void (__thiscall*)(void*, float*);
    using Flags = uint32_t (__thiscall*)(void*);
    using Altitude = float (__thiscall*)(void*);
    float center[3] = {};
    reinterpret_cast<Center>(slots[15])(lot, center);
    const uint32_t flags = reinterpret_cast<Flags>(slots[12])(lot);
    const float altitude = reinterpret_cast<Altitude>(slots[16])(lot);
    uint32_t actual[5] = {};
    std::memcpy(actual, center, sizeof(center));
    actual[3] = flags;
    std::memcpy(actual + 4, &altitude, sizeof(altitude));
    uint32_t differences = 0;
    for (unsigned i = 0; i < 5; ++i) if (actual[i] != expected[i]) differences |= 1u << i;
    return differences;
}
}
