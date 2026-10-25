// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstddef>
#include <cstdint>
#include <cstring>

namespace fc4::inplace {
// General x86 encoding facts only. Unknown/prefixed/control-flow instructions
// fail closed. No instruction bytes or derived targets are logged or persisted.
struct Instruction { unsigned size = 0; unsigned relative = 0; };
inline Instruction Decode(const uint8_t* p, size_t available) {
    if (available < 1) return {};
    unsigned n = 1, immediate = 0;
    bool modrm = false;
    const auto op = p[0];
    if ((op >= 0x50 && op <= 0x5f) || op == 0x90) {}
    else if (op == 0x68 || (op >= 0xb8 && op <= 0xbf) || op == 0xa1 || op == 0xa3) immediate = 4;
    else if (op == 0x6a) immediate = 1;
    else if (op == 0xe8 || op == 0xe9) return available >= 5 ? Instruction{5, 1} : Instruction{};
    else if (op == 0x8b || op == 0x89 || op == 0x8d || op == 0x85 || op == 0x84 ||
             op == 0x31 || op == 0x33 || op == 0x29 || op == 0x2b || op == 0x01 ||
             op == 0x03 || op == 0x3b || op == 0x39 || op == 0xd9 || op == 0xdd ||
             op == 0xd8 || op == 0xdc || op == 0x8a || op == 0x88) modrm = true;
    else if (op == 0x81 || op == 0xc7) { modrm = true; immediate = 4; }
    else if (op == 0x83 || op == 0x80 || op == 0xc6) { modrm = true; immediate = 1; }
    else return {};
    if (modrm) {
        if (n >= available) return {};
        const auto m = p[n++];
        const unsigned mode = m >> 6, rm = m & 7;
        if ((op == 0xc6 || op == 0xc7) && (m & 0x38)) return {};
        if (mode != 3) {
            if (rm == 4) {
                if (n >= available) return {};
                const auto sib = p[n++];
                if (mode == 0 && (sib & 7) == 5) n += 4;
            }
            if (mode == 0 && rm == 5) n += 4;
            if (mode == 1) n += 1;
            if (mode == 2) n += 4;
        }
    }
    n += immediate;
    return n <= available && n <= 15 ? Instruction{n, 0} : Instruction{};
}

// Rel32 arithmetic wraps on 32-bit x86, including jumps across the sign boundary.
inline void Jump(uint8_t* dst, uintptr_t source, uintptr_t target) {
    dst[0] = 0xe9;
    const uint32_t relative = static_cast<uint32_t>(target - (source + 5));
    std::memcpy(dst + 1, &relative, 4);
}
inline unsigned Trampoline(const uint8_t* source, size_t available, uintptr_t from,
                           uint8_t* destination, uintptr_t to) {
    unsigned n = 0;
    while (n < 5) {
        const auto i = Decode(source + n, available - n);
        if (!i.size || n + i.size > 24) return 0;
        std::memcpy(destination + n, source + n, i.size);
        if (i.relative) {
            uint32_t relative;
            std::memcpy(&relative, source + n + i.relative, 4);
            const auto target = static_cast<uint32_t>(from + n + i.size + relative);
            // No internal transfers into overwritten instruction storage.
            if (target >= from && target < from + 24) return 0;
            relative = static_cast<uint32_t>(target - (to + n + i.size));
            std::memcpy(destination + n + i.relative, &relative, 4);
        }
        n += i.size;
    }
    Jump(destination + n, to + n, from + n);
    return n;
}

inline bool Sample(uint64_t call) { return call <= 50 || (call - 50) % 20 == 0; }
inline uint32_t FloatDistance(uint32_t a, uint32_t b) {
    const auto ordered = [](uint32_t x) { return x & 0x80000000u ? ~x : x | 0x80000000u; };
    const auto x = ordered(a), y = ordered(b);
    return x > y ? x - y : y - x;
}
inline bool NearFloat(uint32_t a, uint32_t b) {
    return (a & 0x7f800000u) != 0x7f800000u && (b & 0x7f800000u) != 0x7f800000u &&
        FloatDistance(a, b) <= 4;
}
}
