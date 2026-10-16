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

// The network cost pricing of traffic-simulator.md §4.9.1 (TG23,
// re-answers21): `UpdateRoadsBudget` (`0x0070D8A0`) and
// `UpdateMassTransitBudget` (`0x0070DA40`) price the per-network-type counts
// `cells[t]` with the per-tile costs `tileCost[t]` (property `0x6A84493E`, 13
// floats indexed by network type) before they write the departments' line
// items. Pure logic, so the plugin and the unit tests share it.

#pragma once

#include <cstdint>
#include <cstring>

namespace fc4 {

// [TG23] One single-precision (24-bit mantissa) rounding step, the original's
// x87 at 24-bit precision. The store through a volatile float forces the
// rounding whatever the calling thread's x87 control word is (foundations §18:
// our DLL's presence raises the main thread to 53-bit).
inline float TrafficR24(float v) {
    volatile float t = v;
    return t;
}

// [TG23] §4.9.1 "Pricing one type": `price(t)` is a u32 computed with
// single-precision x87 arithmetic.
//   n := cells[t] loaded as an unsigned 32-bit value (a value at or above
//        2^31 is corrected by adding 2^32);
//   p := r24(n × tileCost[t]);
//   s := r24(p + 12582912.0) (the f32 constant 0x4B400000, 1.5 × 2^23),
//        stored as an f32;
//   price(t) := bits(s) − 0x4B400000, as a u32 (wrapping).
//
// For 0 ≤ p < 2^22 this is p rounded to the nearest integer, **ties to even**
// (0.5 → 0, 1.5 → 2, 2.5 → 2, 10.5 → 10); the f32 store is IEEE
// round-to-nearest-even, which is what gives the tie rule. Outside that range
// the bit pattern gives something else, which a replacement keeps
// *[compat]*: a negative tuned cost gives a negative integer read as a u32
// (−3 → 0xFFFFFFFD), and a product of 2^22 or more no longer lands on an
// integer grid of 1. With the shipped costs a product of 2^22 would need tens
// of millions of records, so normal play never leaves the exact range.
//
// [choice] `n` is kept exact and the product is rounded once, as the formula
// `r24(n × tileCost[t])` is written and as x87 does it (`fild` loads the
// integer exactly into the 80-bit register, then the multiply runs at 24-bit
// precision). Reading `n` as an f32 first would round it again for counts
// above 2^24.
inline uint32_t NetworkPrice(uint32_t cells, float tileCost) {
    // The unsigned load: the original's "a value at or above 2^31 is
    // corrected by adding 2^32" lands on this same value.
    const double n = (cells & 0x80000000u) != 0 ? static_cast<double>(static_cast<int32_t>(cells)) + 4294967296.0
                                                : static_cast<double>(cells);
    const float p = TrafficR24(static_cast<float>(n * static_cast<double>(tileCost)));
    const float s = TrafficR24(p + 12582912.0f);
    uint32_t bits = 0;
    std::memcpy(&bits, &s, sizeof(bits));
    return bits - 0x4B400000u;
}

// [TG23] §4.9.1 "Combining two types": a line item fed by two types gets
// `price(a) + price(b)` computed as an unsigned 64-bit sum of the two u32
// values (the carry goes into the high word, `0x0070D91C`/`0x0070D91E`).
inline uint64_t NetworkPriceSum(uint32_t a, uint32_t b, const uint32_t* cells, const float* costs) {
    return static_cast<uint64_t>(NetworkPrice(cells[a], costs[a])) + static_cast<uint64_t>(NetworkPrice(cells[b], costs[b]));
}

// [TG23] A line item fed by one type gets that u32 zero-extended to 64 bits.
inline uint64_t NetworkPriceOne(uint32_t t, const uint32_t* cells, const float* costs) {
    return static_cast<uint64_t>(NetworkPrice(cells[t], costs[t]));
}

}  // namespace fc4
