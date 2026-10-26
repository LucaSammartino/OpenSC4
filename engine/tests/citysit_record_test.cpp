// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4CitySitRecord.h"
#include <iostream>

int main() {
    const std::vector<uint8_t> a{2,0,2,0,0,0, 3,0,0,0,4,0, 1,0,0,0,7,0, 1};
    auto b = a;
    std::swap_ranges(b.begin() + 6, b.begin() + 12, b.begin() + 12);
    int failures = 0;
    const auto check = [&](bool ok) { if (!ok) ++failures; };
    check(fc4::CitySitRecordEqual(a, b));
    b[10] ^= 1; // A changed success count must fail, regardless of order.
    check(!fc4::CitySitRecordEqual(a, b));
    b = a; b.back() = 0;
    check(!fc4::CitySitRecordEqual(a, b));
    b = a; b[2] = 3;
    check(!fc4::CitySitRecordEqual(a, b));
    b = a; b.push_back(0);
    check(!fc4::CitySitRecordEqual(a, b));
    b = a; b[0] = 1;
    check(!fc4::CitySitRecordEqual(a, b));
    if (failures) std::cerr << failures << " citysit comparison failures\n";
    return failures != 0;
}
