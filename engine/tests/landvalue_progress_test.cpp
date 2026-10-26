// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4LandValueProgress.h"

#include <cstdlib>
#include <iostream>
#include <limits>

namespace {
void Check(bool ok) { if (!ok) std::abort(); }
void Interval(const fc4::LandValueStepProgress& a, const fc4::LandValueStepProgress& b,
              uint64_t first, uint64_t end, bool fallback = false, bool curve = true) {
    const auto r = fc4::LandValueDecodeTractInterval(a, b, curve);
    Check(r.valid && r.first == first && r.end == end && r.zoningFallback == fallback);
}
}

int main() {
    fc4::LandValueStepProgress a, b;
    a.state = 1;
    a.tract = {0, -2, 3, 1, -2, 3, 0, 4}; // six signed/inclusive coordinates
    b = a;
    Interval(a, b, 0, 0); // fresh unchanged: no invented first yield
    b.tract.state = 1;
    Interval(a, b, 0, 1);
    a = b;
    Interval(a, b, 1, 1); // yielding coordinate was already consumed
    b.tract.x = -1; b.tract.z = 4;
    Interval(a, b, 1, 5); // wrap x first, then advance z
    a = b;
    b.tract.state = 2; b.tract.z = 5;
    Interval(a, b, 5, 6); // exhausted z is not a tract
    b.state = 2; b.tract = {};
    Interval(a, b, 5, 6); // transition resets do not supply new work
    b.state = 4;
    Check(!fc4::LandValueDecodeTractInterval(a, b, true).valid);
    a.state = 3; a.ignoreZoning = false;
    b = a; b.ignoreZoning = true; b.tract.state = 0;
    Interval(a, b, 5, 6, true);
    const auto firstScanTail = fc4::LandValueDecodeTractInterval(a, b, true);
    a = b; b.tract.state = 1; b.tract.x = -2; b.tract.z = 3;
    Interval(a, b, 0, 1); // retry restarts counting, retaining first-scan extrema
    b.state = 5;
    const auto secondScan = fc4::LandValueDecodeTractInterval(a, b, true);
    Check(firstScanTail.valid && firstScanTail.zoningFallback &&
          firstScanTail.first == 5 && firstScanTail.end == 6);
    Check(secondScan.valid && !secondScan.zoningFallback &&
          secondScan.first == 0 && secondScan.end == 6); // both scans remain capture inputs
    a.state = 4; b.state = 6;
    Interval(a, b, 0, 0, false, false); // absent curve skips intrinsic writes
    a.state = 5; a.slope = {1, 0, 0, 1, 0, 0, 1, 1};
    b = a; b.state = 4; b.slope = {};
    Interval(a, b, 1, 4); // use slope iterator, not tract iterator
    a.state = 8; a.tract = {1, 0, 4, 1, -2, 3, 0, 4};
    b = a; b.state = 9;
    Interval(a, b, 6, 6); // no remaining tracts at final publication
    for (int state : {0, 2, 6, 7, 9}) {
        a.state = state; b.state = state;
        Interval(a, b, 0, 0); // includes developer/header calls: no tract scan
    }
    a.state = 1; b = a; b.tract.x0 = -3;
    Check(!fc4::LandValueDecodeTractInterval(a, b, true).valid);
    b = a; b.tract.state = 0;
    Check(!fc4::LandValueDecodeTractInterval(a, b, true).valid); // unannounced rewind
    b = a; b.tract.state = 1; b.tract.x = 99;
    Check(!fc4::LandValueDecodeTractInterval(a, b, true).valid);
    a.tract.step = 2; b = a;
    Check(!fc4::LandValueDecodeTractInterval(a, b, true).valid);
    a.tract = {0, 0, 0, 1, std::numeric_limits<int32_t>::min(), std::numeric_limits<int32_t>::min(),
               std::numeric_limits<int32_t>::max(), std::numeric_limits<int32_t>::max()};
    b = a;
    Check(!fc4::LandValueDecodeTractInterval(a, b, true).valid); // count overflow
    std::cout << "landvalue progress intervals passed\n";
}
