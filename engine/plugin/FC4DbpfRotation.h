// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstddef>
#include <cstdint>

namespace fc4 {
struct DbpfSubset {
    size_t first, count;
    bool Contains(size_t index, size_t files) const {
        return files && (index + files - first) % files < count;
    }
};

// Shadow sampling policy, not a native format contract. Sorted paths make the
// order reproducible. For a stable file set, any twelve consecutive months
// cover every file, including every indexed record in that file.
inline DbpfSubset DbpfMonthlySubset(size_t files, uint64_t month) {
    if (!files) return {0, 0};
    const size_t count = files / 12 + (files % 12 != 0);
    return {static_cast<size_t>((month % files) * count % files), count};
}
}
