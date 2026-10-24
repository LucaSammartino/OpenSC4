// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4PropDevTranscript.h"
#include <cstdio>

namespace {
struct Requester { void** vtable; uint32_t calls; uint32_t result; };
struct Words { const uint32_t* begin; const uint32_t* end; const uint32_t* storage; };
uint32_t __fastcall Select(void* self, void*, const void*, int32_t, int32_t) {
    auto* requester = static_cast<Requester*>(self);
    ++requester->calls;
    return requester->result;
}
}
int main() {
    void* table[7]{};
    table[6] = reinterpret_cast<void*>(&Select);
    Requester requester{table, 0, 0};
    const uint32_t ids[]{17, 23, 17};
    const Words words{ids, ids + 3, ids + 3};
    fc4::PropSelectorTrace trace;
    if (!trace.Begin({&requester}, 0x00400000)) return 1;
    using Function = fc4::PropSelectorTrace::Select;
    const auto callback = reinterpret_cast<Function>(table[6]);
    if (callback(&requester, nullptr, &words, -7, 259) != 0) return 2;
    requester.result = 23;
    if (callback(&requester, nullptr, &words, 2, 3) != 23) return 3;
    trace.Finish();
    if (!trace.Valid() || requester.calls != 2 || table[6] != reinterpret_cast<void*>(&Select)) return 4;
    const auto& events = trace.Events();
    if (events.size() != 2 || events[0].candidates != std::vector<uint32_t>({17, 23, 17}) ||
        events[0].result != 0 || events[1].result != 23 || events[0].x != -7 || events[0].z != 259 ||
        events[0].requester != reinterpret_cast<uintptr_t>(&requester) || events[0].depth != 0) return 5;
    // A caller that loaded the forwarder before restoration must still
    // execute the original once; a second scope may reuse the same vtable.
    if (callback(&requester, nullptr, &words, 0, 0) != 23 || requester.calls != 3) return 6;
    if (!trace.Begin({&requester}, 0x00400000)) return 7;
    reinterpret_cast<Function>(table[6])(&requester, nullptr, &words, 0, 0);
    trace.Finish();
    if (!trace.Valid() || requester.calls != 4 || trace.Events().size() != 1) return 8;
    std::puts("propdev transcript: duplicate order, coordinates, zero return, once-only call and restoration PASS");
    return 0;
}
