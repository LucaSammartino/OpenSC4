// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>

namespace fc4 {
// Begin before entering native code: an exception or missing observation must
// leave a counted, incomplete call. Completeness is independent of equality.
struct MethodEvidence {
    uint64_t calls = 0, complete = 0, mismatches = 0;
    void Begin() { ++calls; }
    void End(bool allOutputsCompared, uint32_t differences) {
        complete += allOutputsCompared ? 1u : 0u;
        mismatches += differences;
    }
};
}
