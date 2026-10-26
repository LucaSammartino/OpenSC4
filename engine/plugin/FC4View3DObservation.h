// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <array>
#include <cstdint>

namespace fc4::view3dshadow {
// No tolerances, sampling, native post-state inputs, or persistent resyncs.
struct Evidence {
    bool state = true, result = true, out = true, calls = true;
    bool bound = false;
    bool Exact() const { return state && result && out && calls; }
    bool Complete() const { return bound; }
};
struct Totals {
    uint64_t calls = 0, complete = 0, mismatches = 0;
    void Add(const Evidence& evidence) {
        ++calls;
        if (evidence.Complete()) ++complete;
        // A detected difference remains a difference even on partial paths.
        if (!evidence.Exact()) ++mismatches;
    }
};
constexpr size_t OwnerBytes = 0x448;
using State = std::array<uint8_t, OwnerBytes>;
using Packet = std::array<uint32_t, 22>;
}
