// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4DbpfRotation.h"
#include <cstdio>
#include <vector>

int main() {
    unsigned failures = 0;
    if (fc4::DbpfMonthlySubset(0, 99).count != 0) ++failures;
    // Check every rolling year, not just January-December. Small and large
    // pools exercise wraparound and the more-than-one-file monthly case.
    for (size_t files = 1; files <= 257; ++files) {
        for (uint64_t start = 0; start < 24; ++start) {
            std::vector<bool> seen(files, false);
            for (uint64_t month = start; month < start + 12; ++month) {
                const auto subset = fc4::DbpfMonthlySubset(files, month);
                size_t selected = 0;
                for (size_t i = 0; i < files; ++i) {
                    if (subset.Contains(i, files)) { seen[i] = true; ++selected; }
                }
                if (!selected || selected != subset.count || selected > (files + 11) / 12)
                    ++failures;
            }
            for (bool visited : seen) if (!visited) ++failures;
        }
    }
    // The measured house pool must visit just one whole file per month.
    for (uint64_t month = 0; month < 72; ++month) {
        const auto subset = fc4::DbpfMonthlySubset(11, month);
        if (subset.first != month % 11 || subset.count != 1) ++failures;
    }
    std::printf("dbpf rotation: %u failures\n", failures);
    return failures ? 1 : 0;
}
