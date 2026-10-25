// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4InplaceDetour.h"
#include <cstdio>

int main() {
    using namespace fc4::inplace;
    int failures = 0;
    const auto check = [&](bool ok) { if (!ok) ++failures; };
    uint8_t out[40]{};
    const uint8_t standard[] = {0x55,0x8b,0xec,0x83,0xec,0x20};
    check(Trampoline(standard, sizeof(standard), 0x1000, out, 0x2000) == 6);
    check(std::memcmp(standard, out, 6) == 0 && out[6] == 0xe9);
    const uint8_t sib[] = {0x8b,0x44,0x24,0x04,0x50};
    check(Trampoline(sib, sizeof(sib), 0x1000, out, 0x2000) == 5);
    const uint8_t relative[] = {0xe8,0xfb,0x0f,0x00,0x00};
    check(Trampoline(relative, sizeof(relative), 0x1000, out, 0x3000) == 5);
    uint32_t rel; std::memcpy(&rel, out + 1, 4);
    check(static_cast<uint32_t>(0x3005 + rel) == 0x2000);
    const uint8_t internal[] = {0xe9,0,0,0,0};
    check(Trampoline(internal, sizeof(internal), 0x1000, out, 0x2000) == 0);
    const uint8_t shortJump[] = {0xeb,0x03,0x90,0x90,0x90};
    check(Trampoline(shortJump, sizeof(shortJump), 0x1000, out, 0x2000) == 0);
    for (size_t i = 0; i < sizeof(standard) - 1; ++i)
        check(Trampoline(standard, i, 0x1000, out, 0x2000) == 0);
    unsigned selected = 0;
    for (uint64_t i = 1; i <= 1000; ++i) selected += Sample(i);
    check(selected == 97 && Sample(50) && !Sample(51) && Sample(70));
    check(NearFloat(0x3f800000, 0x3f800001));
    check(!NearFloat(0x3f800000, 0x40000000));
    check(!NearFloat(0x7f800000, 0x7f800000));
    std::printf("inplace mechanics: failures=%d\n", failures);
    return failures ? 1 : 0;
}
