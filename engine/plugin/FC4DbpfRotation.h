// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstddef>
#include <cstdint>

namespace fc4 {
// The harness keeps delivering ticks while a dated shadow queue drains.
// Retain the final day's sample, but never extend that queue beyond its target.
inline bool DbpfSampleInWindow(int32_t julian, int32_t endJulian) {
    return endJulian <= 0 || julian <= endJulian;
}

// data-formats 8.3.2: a tick in a modal save is not a return; a completed
// save at a later date cannot fill the earlier live-file obligation either.
inline bool DbpfReplayAtSameCityDate(bool returned, unsigned activeSaves,
    const void* promisedCity, int32_t promisedDate, const void* city, int32_t date) {
    return returned && !activeSaves && promisedCity && city == promisedCity && date == promisedDate;
}

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
