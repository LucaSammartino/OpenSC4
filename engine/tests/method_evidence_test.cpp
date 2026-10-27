// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4MethodEvidence.h"
#include <cstdio>

int main() {
    fc4::MethodEvidence evidence;
    evidence.Begin(); // Native entry faults: no End, never earns completeness.
    evidence.Begin();
    evidence.End(false, 0); // Equal observed subset is still incomplete.
    evidence.Begin();
    evidence.End(true, 3); // All outputs compared, three differences persist.
    evidence.Begin();
    evidence.End(true, 0); // A later exact call cannot erase the prior failure.
    if (evidence.calls != 4 || evidence.complete != 2 || evidence.mismatches != 3) {
        std::fprintf(stderr, "method evidence lost an incomplete call or mismatch\n");
        return 1;
    }
    evidence = {}; // A new city lifetime has independent cumulative counters.
    if (evidence.calls || evidence.complete || evidence.mismatches) return 2;
    return 0;
}
