// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include <cstdint>

namespace fc4 {

// Owned progress values from the published L/ScanningIterator layouts. This
// decoder neither reads native memory nor installs an interceptor. LV8 blocks
// the latter; callers must not label pass-end samples as Step observations.
struct LandValueIteratorProgress {
    uint8_t state = 0;
    int32_t x = 0, z = 0, step = 1;
    int32_t x0 = 0, z0 = 0, x1 = 0, z1 = 0;
};

struct LandValueStepProgress {
    int32_t state = 9;
    bool ignoreZoning = false;
    LandValueIteratorProgress tract, slope;
    uint32_t developerCursor = 0, developerHeader = 0;
};

struct LandValueTractInterval {
    bool valid = false;
    // Counting restarts, but first-scan extrema survive: keep both intervals.
    bool zoningFallback = false;
    // Half-open ordinal interval in z-outer/x-inner order, relative to the
    // BEFORE iterator's bounds. Never use an exhausted/reset after coordinate.
    uint64_t first = 0, end = 0;
};

inline bool LandValueIteratorExtent(const LandValueIteratorProgress& p, uint64_t& width, uint64_t& count) {
    // Only the published constructor's unit step is supported. Reject other
    // strides rather than guessing a contract for a native field mutation.
    if (p.state > 2 || p.step != 1 || p.x1 < p.x0 || p.z1 < p.z0) return false;
    width = static_cast<uint64_t>(int64_t{p.x1} - p.x0 + 1);
    const uint64_t height = static_cast<uint64_t>(int64_t{p.z1} - p.z0 + 1);
    // Reject the sole possible overflowing product of two int32 extents.
    if (width == (uint64_t{1} << 32) && height == (uint64_t{1} << 32)) return false;
    count = width * height;
    return true;
}

inline bool LandValueYieldedCount(const LandValueIteratorProgress& p, uint64_t width,
                                 uint64_t count, uint64_t& yielded) {
    if (p.state == 0) { yielded = 0; return true; }
    if (p.state == 2) { yielded = count; return true; }
    if (p.x < p.x0 || p.x > p.x1 || p.z < p.z0 || p.z > p.z1) return false;
    yielded = static_cast<uint64_t>(int64_t{p.z} - p.z0) * width +
              static_cast<uint64_t>(int64_t{p.x} - p.x0) + 1;
    return true;
}

// landvalue.md sections 6.1.1/6.1.3: decode only the OLD state's work.
// The curve-presence input is needed for state 4's explicitly specified skip.
// No resync, simulation clock, native pointer retention or output-based retry.
inline LandValueTractInterval LandValueDecodeTractInterval(const LandValueStepProgress& before,
                                                          const LandValueStepProgress& after,
                                                          bool altitudeCurvePresent) {
    LandValueTractInterval r;
    if (before.state < 0 || before.state > 9 || after.state < 0 || after.state > 9) return r;
    if (before.state != 1 && before.state != 3 && before.state != 4 &&
        before.state != 5 && before.state != 8) {
        r.valid = true;
        return r;
    }
    const auto& p = before.state == 5 ? before.slope : before.tract;
    const auto& q = before.state == 5 ? after.slope : after.tract;
    uint64_t width = 0, count = 0;
    if (!LandValueIteratorExtent(p, width, count) ||
        !LandValueYieldedCount(p, width, count, r.first)) return r;
    r.end = r.first;
    if (before.state == 4 && !altitudeCurvePresent) {
        r.valid = after.state == 6;
        return r;
    }
    r.zoningFallback = before.state == 3 && !before.ignoreZoning && after.ignoreZoning;
    const int32_t next = before.state == 1 ? 2 : before.state == 3 ? 5 :
                         before.state == 5 ? 4 : before.state == 4 ? 6 : 9;
    if (r.zoningFallback) {
        // Reset belongs to the second scan. Keep the first scan's inputs:
        // min/max survive even though sum/count/histogram restart (§6.1.6).
        if (after.state != 3 || q.state != 0) return r;
        r.end = count;
    } else if (after.state == next) {
        r.end = count;
    } else if (after.state == before.state) {
        if (p.x0 != q.x0 || p.z0 != q.z0 || p.x1 != q.x1 || p.z1 != q.z1 || p.step != q.step ||
            q.state > 2 || !LandValueYieldedCount(q, width, count, r.end) || r.end < r.first) return r;
    } else {
        return r;
    }
    r.valid = true;
    return r;
}

}  // namespace fc4
