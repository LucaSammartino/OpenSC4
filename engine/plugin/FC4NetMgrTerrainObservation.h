// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
#include <vector>

namespace fc4::netmgr {
enum class DestructionSource { Unknown, TerrainFilter, UndergroundRefit };

// Published function extents from docs/abi/functions.tsv; classify the
// observed caller, without reading instructions or inferring from cells.
inline DestructionSource Source(uintptr_t caller) {
    if (caller >= 0x00613950 && caller < 0x00613950 + 1964) return DestructionSource::TerrainFilter;
    if (caller >= 0x006588D0 && caller < 0x006588D0 + 2393) return DestructionSource::UndergroundRefit;
    return DestructionSource::Unknown;
}

struct Removal {
    uint64_t call;
    uint32_t sequence;
    int32_t x, z;
};
struct Insertion {
    uint64_t id, call;
    uint32_t sequence, data;
    int32_t x, z;
};

// §7.6.1 / network-tools §10: old-object absence is distinct from cell
// vacancy. Require the observed ordered remove/insert pair from one tool
// call, including the neighbour-rebuild insertion's data4=1.
inline bool ObservedReplacement(uint64_t old, const Removal& removal, const std::vector<Insertion>& insertions) {
    if (!removal.call) return false;
    for (const auto& next : insertions) {
        if (next.call == removal.call && next.sequence > removal.sequence && next.id != old && next.data == 1 &&
            next.x == removal.x && next.z == removal.z) return true;
    }
    return false;
}
} // namespace fc4::netmgr
