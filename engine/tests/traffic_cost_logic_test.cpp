// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

// Unit tests of plugin/FC4TrafficCostLogic.h: the network cost pricing of
// traffic-simulator.md §4.9.1 (TG23, re-answers21). The worked example and
// the tie cases are the spec's own numbers.

#include "FC4TrafficCostLogic.h"

#include <cstdio>
#include <cstring>

namespace {

int g_failures = 0;

#define CHECK(cond)                                                          \
    do {                                                                     \
        if (!(cond)) {                                                       \
            std::printf("FAILED: %s (line %d)\n", #cond, __LINE__);          \
            ++g_failures;                                                    \
        }                                                                    \
    } while (0)

#define CHECK_EQ(a, b)                                                                          \
    do {                                                                                        \
        const auto a_ = (a);                                                                    \
        const auto b_ = (b);                                                                    \
        if (!(a_ == b_)) {                                                                      \
            std::printf("FAILED: %s == %s (line %d): %llu vs %llu\n", #a, #b, __LINE__,          \
                        static_cast<unsigned long long>(a_), static_cast<unsigned long long>(b_)); \
            ++g_failures;                                                                       \
        }                                                                                       \
    } while (0)

// [TG23] The shipped tile costs (§4.9.1), 13 floats indexed by network type:
// Road 0, Rail 1, Highway 2, Street 3, DirtRoad 4, (unused) 5, Avenue 6,
// Subway 7, LightRail 8, Monorail 9, OneWayRoad 10, DirtRoad 11,
// GroundHighway 12.
const float kShippedCosts[13] = {0.1f, 0.03f, 0.5f, 0.05f, 0.0f, 0.0f, 0.15f,
                                 0.3f, 0.2f,  0.3f, 0.1f,  0.0f, 0.5f};

}  // namespace

int main() {
    using fc4::NetworkPrice;
    using fc4::NetworkPriceOne;
    using fc4::NetworkPriceSum;

    // [TG23] §4.9.1 worked example: counts Road 538, OneWayRoad 10, Street
    // 458, Highway 100, GroundHighway 21, Avenue 37, Rail 143, Subway 60,
    // LightRail 7, Monorail 3.
    uint32_t cells[13] = {0};
    cells[0] = 538;   // Road
    cells[1] = 143;   // Rail
    cells[2] = 100;   // Highway
    cells[3] = 458;   // Street
    cells[6] = 37;    // Avenue
    cells[7] = 60;    // Subway
    cells[8] = 7;     // LightRail
    cells[9] = 3;     // Monorail
    cells[10] = 10;   // OneWayRoad
    cells[12] = 21;   // GroundHighway

    CHECK_EQ(NetworkPrice(538, kShippedCosts[0]), 54u);    // 53.799999 -> 54
    CHECK_EQ(NetworkPrice(10, kShippedCosts[10]), 1u);     // 1.0 -> 1
    CHECK_EQ(NetworkPrice(458, kShippedCosts[3]), 23u);    // 22.9 -> 23
    CHECK_EQ(NetworkPrice(100, kShippedCosts[2]), 50u);    // 50.0 -> 50
    CHECK_EQ(NetworkPrice(21, kShippedCosts[12]), 10u);    // 10.5 -> 10, tie to even
    CHECK_EQ(NetworkPrice(37, kShippedCosts[6]), 6u);      // 5.55 -> 6
    CHECK_EQ(NetworkPrice(143, kShippedCosts[1]), 4u);     // 4.29 -> 4
    CHECK_EQ(NetworkPrice(60, kShippedCosts[7]), 18u);     // 18.0 -> 18
    CHECK_EQ(NetworkPrice(7, kShippedCosts[8]), 1u);       // 1.4 -> 1
    CHECK_EQ(NetworkPrice(3, kShippedCosts[9]), 1u);       // 0.90000004 -> 1

    // [TG23] The line items, in the order the functions write them.
    // Roads: item 2 := price(Road) + price(OneWayRoad), item 1 := price(Street),
    // item 3 := price(Highway) + price(GroundHighway), item 4 := price(Avenue).
    CHECK_EQ(NetworkPriceSum(0, 10, cells, kShippedCosts), 55u);
    CHECK_EQ(NetworkPriceOne(3, cells, kShippedCosts), 23u);
    CHECK_EQ(NetworkPriceSum(2, 12, cells, kShippedCosts), 60u);
    CHECK_EQ(NetworkPriceOne(6, cells, kShippedCosts), 6u);
    // Mass transit: items 3–6 := price(Rail), price(Subway), price(LightRail),
    // price(Monorail).
    CHECK_EQ(NetworkPriceOne(1, cells, kShippedCosts), 4u);
    CHECK_EQ(NetworkPriceOne(7, cells, kShippedCosts), 18u);
    CHECK_EQ(NetworkPriceOne(8, cells, kShippedCosts), 1u);
    CHECK_EQ(NetworkPriceOne(9, cells, kShippedCosts), 1u);

    // [TG23] Ties (§11 test 16): Street counts 10, 30, 50 give 0, 2, 2 and
    // Street 15 and 25 give 1 and 1. Street's cost is 0.05, so the products
    // are 0.5, 1.5, 2.5 (ties to even) and 0.75, 1.25.
    CHECK_EQ(NetworkPrice(10, kShippedCosts[3]), 0u);
    CHECK_EQ(NetworkPrice(30, kShippedCosts[3]), 2u);
    CHECK_EQ(NetworkPrice(50, kShippedCosts[3]), 2u);
    CHECK_EQ(NetworkPrice(15, kShippedCosts[3]), 1u);
    CHECK_EQ(NetworkPrice(25, kShippedCosts[3]), 1u);

    // [TG23] Set, not added: the same counts price to the same values twice.
    CHECK_EQ(NetworkPrice(458, kShippedCosts[3]), NetworkPrice(458, kShippedCosts[3]));

    // [TG23] "Combining two types": an unsigned 64-bit sum of the two u32
    // values, so the carry goes into the high word instead of wrapping. Two
    // prices of 0xFFFFFFFD (the negative-cost quirk below) sum to
    // 0x1FFFFFFFA.
    uint32_t wrap[13] = {0};
    float neg[13] = {0};
    neg[0] = -1.0f;
    neg[1] = -1.0f;
    wrap[0] = 3;
    wrap[1] = 3;
    CHECK_EQ(NetworkPrice(3, -1.0f), 0xFFFFFFFDu);
    CHECK_EQ(NetworkPriceSum(0, 1, wrap, neg), static_cast<uint64_t>(0x1FFFFFFFAull));

    // [TG23] A line item fed by one type gets that u32 zero-extended to 64
    // bits.
    CHECK_EQ(NetworkPriceOne(0, wrap, neg), static_cast<uint64_t>(0xFFFFFFFDull));

    // [TG23] Outside 0 ≤ p < 2^22 the bit pattern gives something else, which
    // a replacement keeps *[compat]*: a negative tuned cost gives a negative
    // integer read as a u32 (−3 → 0xFFFFFFFD).
    CHECK_EQ(NetworkPrice(3, -1.0f), static_cast<uint32_t>(-3));

    if (g_failures != 0) {
        std::printf("%d failure(s)\n", g_failures);
        return 1;
    }
    std::printf("traffic_cost_logic_test: all checks passed\n");
    return 0;
}
