// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
#include <unordered_set>
namespace fc4 {
// LR12 inventory accounting, independent of the live process for T58-T65.
struct LotRecordObservation {
    uint32_t expected = 0, visited = 0, captures = 0, exact = 0;
    uint32_t differences = 0, errors = 0;
    std::unordered_set<const void*> seen;
    bool Visit(const void* lot) {
        if (!lot || !seen.insert(lot).second) { ++errors; return false; }
        ++visited;
        return true;
    }
    void Record(bool captured, bool read, bool equal) {
        if (!captured) { ++errors; return; }
        ++captures;
        if (!read) ++errors;
        else if (!equal) ++differences;
        else ++exact;
    }
    void Finish(bool normalEnd, uint32_t countAfter) {
        if (!normalEnd || countAfter != expected || visited != expected) ++errors;
    }
    uint32_t Mismatches() const { return errors + differences; }
};
}
