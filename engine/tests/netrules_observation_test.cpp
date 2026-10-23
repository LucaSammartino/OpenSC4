// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4NetRulesObservation.h"
#include <cassert>
#include <cstring>
#include <initializer_list>
int main() {
    using namespace fc4::netrules;
    uint8_t spans[3][6];
    std::memcpy(spans, kBytes, sizeof(spans));
    // NR17 vectors 45–48: all bytes, jump conflicts, absolute operand, base.
    assert(Reference(kBase, spans));
    assert(kSites[0] + kLengths[0] == 0x006220D5);
    assert(kSites[1] + kLengths[1] == 0x006253D6);
    assert(kSites[2] + kLengths[2] == 0x006232F5);
    for (size_t i = 0; i < 3; ++i) {
        for (size_t j = 0; j < kLengths[i]; ++j) {
            spans[i][j] ^= 1; assert(!Reference(kBase, spans)); spans[i][j] ^= 1;
        }
        const auto saved = spans[i][0]; spans[i][0] = 0xE9;
        assert(!Reference(kBase, spans)); spans[i][0] = saved;
    }
    uint8_t trampoline[16]{}; std::memcpy(trampoline, spans[2], 5);
    uint32_t operand = 0; std::memcpy(&operand, trampoline + 1, 4);
    assert(operand == 0x00B452A8); // must not relocate this absolute operand
    assert(!Reference(kBase + 0x10000, spans));
    // NR17 vector 49: zero flag and retained tables cannot revalidate an epoch.
    for (uint8_t flag : {0, 1, 0x7F}) {
        Epoch e; e.Complete();
        e.Invalidate(); // cleanup entry, independent of incoming flag
        assert(!e.ready && e.number == 1);
        flag = 0; (void)flag;
        assert(!e.ready);
        e.Invalidate(); e.Complete(); // next observed reload return
        assert(e.ready && e.number == 2);
    }
}
