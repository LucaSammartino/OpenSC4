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
    b = a; b[11] ^= 1; // Compare the full u16 count, including its high byte.
    check(!fc4::CitySitRecordEqual(a, b));
    b = a; b[9] ^= 1; // GUIDs are persistent state too.
    check(!fc4::CitySitRecordEqual(a, b));
    b = a; b[6] = 1; // Replacing an entry with a duplicate is not a permutation.
    check(!fc4::CitySitRecordEqual(a, b));
    b = a; b.back() = 0;
    check(!fc4::CitySitRecordEqual(a, b));
    b = a; b[2] = 3;
    check(!fc4::CitySitRecordEqual(a, b));
    b = a; b.push_back(0);
    check(!fc4::CitySitRecordEqual(a, b));
    b = a; b[0] = 1;
    check(!fc4::CitySitRecordEqual(a, b));
    check(!fc4::CitySitRecordEqual(b, b)); // Unsupported shape cannot self-validate.
    check(!fc4::CitySitRecordEqual({}, {}));
    b = a; b.pop_back();
    check(!fc4::CitySitRecordEqual(b, b)); // Both readers must provide a complete record.
    const std::vector<uint8_t> emptyMap{2,0,0,0,0,0,1};
    check(fc4::CitySitRecordEqual(emptyMap, emptyMap));
    if (failures) std::cerr << failures << " citysit comparison failures\n";
    return failures != 0;
}
