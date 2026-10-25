// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4LotRecordView.h"
#include <array>
#include <cstdio>

namespace {
struct Lot {
    void** vtable;
    uint32_t words[5];
    unsigned calls[3] = {};
};
void __fastcall Center(Lot* lot, void*, float* output) {
    ++lot->calls[0];
    std::memcpy(output, lot->words, 12);
}
uint32_t __fastcall Flags(Lot* lot, void*) { ++lot->calls[1]; return lot->words[3]; }
float __fastcall Altitude(Lot* lot, void*) {
    ++lot->calls[2];
    float value;
    std::memcpy(&value, lot->words + 4, 4);
    return value;
}
}
int main() {
    std::array<void*, 17> slots{};
    slots[12] = reinterpret_cast<void*>(&Flags);
    slots[15] = reinterpret_cast<void*>(&Center);
    slots[16] = reinterpret_cast<void*>(&Altitude);
    // Exact bits matter: +0/-0 and two NaN payloads must not compare equal.
    uint32_t expected[5] = {0x80000000u, 0x7fc01234u, 0x43a80000u, 0xdbu, 0x40000000u};
    Lot lot{slots.data(), {expected[0], expected[1], expected[2], expected[3], expected[4]}};
    if (fc4::CompareLotRecordView(&lot, expected) != 0) return 1;
    for (unsigned i = 0; i < 5; ++i) {
        lot.words[i] ^= 1u;
        if (fc4::CompareLotRecordView(&lot, expected) != (1u << i)) return 2;
        lot.words[i] ^= 1u;
    }
    for (unsigned count : lot.calls) if (count != 6) return 3;
    // Floating equality would hide signed-zero and NaN-payload differences.
    expected[0] = 0;
    expected[4] = 0;
    lot.words[0] = 0x80000000u;
    lot.words[4] = 0x80000000u;
    if (fc4::CompareLotRecordView(&lot, expected) != 0x11u) return 4;
    lot.words[0] = 0;
    lot.words[4] = 0;
    lot.words[1] = 0x7fc05678u;
    if (fc4::CompareLotRecordView(&lot, expected) != 0x02u) return 5;
    std::puts("lot record view: exact bits, slot ABI, all five mismatches detected");
    return 0;
}
