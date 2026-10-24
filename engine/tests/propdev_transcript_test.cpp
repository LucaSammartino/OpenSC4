// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4PropDevTranscript.h"
#include "FC4PropDevCreation.h"
#include <cstdio>
#include <cstring>

namespace {
struct Requester { void** vtable; uint32_t calls; uint32_t result; };
struct Words { const uint32_t* begin; const uint32_t* end; const uint32_t* storage; };
uint32_t __fastcall Select(void* self, void*, const void*, int32_t, int32_t) {
    auto* requester = static_cast<Requester*>(self);
    ++requester->calls;
    return requester->result;
}
struct Creator { void** vtable; unsigned calls; void* result; };
size_t nested=0;
unsigned retains=0;
void* __fastcall Create(void* self,void*,void*,uint32_t,uint32_t) {
    auto* c=static_cast<Creator*>(self); ++c->calls; ++nested; return c->result;
}
uint64_t Retain(void*,void* result) { ++retains; return reinterpret_cast<uintptr_t>(result); }
size_t Nested() { return nested; }
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
    using Purpose = fc4::PropSelectorTrace::Purpose;
    auto recorded = events;
    recorded[0].purpose = Purpose::Kept;
    fc4::PropSelectorReplay replay(recorded);
    // Current trace has one genuine call after reuse, including duplicates.
    if (replay.Complete() || std::strcmp(replay.Failure(), "missing-call")) return 9;
    if (replay.Select(reinterpret_cast<uintptr_t>(&requester), Purpose::Kept, ids, 3, 0, 0) != 23 ||
        !replay.Complete() || replay.Matched() != 1 || requester.calls != 4) return 10;
    recorded[0].result = 0;
    fc4::PropSelectorReplay zero(recorded);
    if (zero.Select(reinterpret_cast<uintptr_t>(&requester), Purpose::Kept, ids, 3, 0, 0) != 0 ||
        !zero.Complete()) return 11;
    recorded[0].result = 0xFFFFFFFF;
    fc4::PropSelectorReplay outside(recorded);
    if (outside.Select(reinterpret_cast<uintptr_t>(&requester), Purpose::Kept, ids, 3, 0, 0) != 0xFFFFFFFF ||
        !outside.Complete()) return 12;
    for (unsigned mismatch=0; mismatch<7; ++mismatch) {
        auto changed = recorded;
        if (mismatch==0) ++changed[0].requester;
        if (mismatch==1) changed[0].purpose=Purpose::Creation;
        if (mismatch==2) ++changed[0].x;
        if (mismatch==3) ++changed[0].z;
        if (mismatch==4) ++changed[0].depth;
        if (mismatch==5) changed[0].candidates.pop_back();
        if (mismatch==6) std::swap(changed[0].candidates[0],changed[0].candidates[1]);
        fc4::PropSelectorReplay invalid(changed);
        if (invalid.Select(reinterpret_cast<uintptr_t>(&requester),Purpose::Kept,ids,3,0,0) != 0 ||
            invalid.Complete()) return 13;
        const char* reasons[]{"requester", "purpose", "coordinates", "coordinates", "depth",
                              "candidate-count", "candidate-order"};
        if (std::strcmp(invalid.Failure(), reasons[mismatch]) || !invalid.Expected() ||
            invalid.Actual().candidates != std::vector<uint32_t>({17,23,17})) return 15;
    }
    if (outside.Select(reinterpret_cast<uintptr_t>(&requester),Purpose::Kept,ids,3,0,0) != 0 ||
        outside.Complete()) return 14;
    if (std::strcmp(outside.Failure(), "extra-call")) return 16;
    void* lotTable[64]{}; lotTable[63]=reinterpret_cast<void*>(&Create);
    void* netTable[9]{}; netTable[8]=reinterpret_cast<void*>(&Create);
    Creator lotCreator{lotTable,0,nullptr}, netCreator{netTable,0,&requester};
    fc4::PropCreationTrace creations;
    if (!creations.Begin(&lotCreator,&netCreator,&Retain,nullptr,&Nested)) return 17;
    const auto lotCall=reinterpret_cast<fc4::PropCreationTrace::Create>(lotTable[63]);
    const auto netCall=reinterpret_cast<fc4::PropCreationTrace::Create>(netTable[8]);
    if (lotCall(&lotCreator,nullptr,&lotCreator,4,17)!=nullptr ||
        netCall(&netCreator,nullptr,&netCreator,8,23)!=&requester) return 18;
    creations.Finish();
    const auto& attempts=creations.Events();
    if (!creations.Valid() || attempts.size()!=2 || retains!=1 || lotCreator.calls!=1 || netCreator.calls!=1 ||
        attempts[0].lot!=reinterpret_cast<uintptr_t>(&lotCreator) || attempts[0].network ||
        attempts[0].index!=4 || attempts[0].id!=17 || attempts[0].result || attempts[0].nestedBegin!=0 ||
        attempts[0].nestedEnd!=1 || !attempts[1].network || attempts[1].index!=8 || attempts[1].id!=23 ||
        attempts[1].result!=reinterpret_cast<uintptr_t>(&requester) || attempts[1].nestedBegin!=1 ||
        attempts[1].nestedEnd!=2 || attempts[1].depth) return 19;
    if (lotTable[63]!=reinterpret_cast<void*>(&Create) || netTable[8]!=reinterpret_cast<void*>(&Create)) return 20;
    lotCall(&lotCreator,nullptr,&lotCreator,4,17); // loaded forwarding pointer remains valid after restoration
    if (lotCreator.calls!=2 || creations.Events().size()!=2 || retains!=1) return 21;
    if (!creations.Begin(&lotCreator,&netCreator,&Retain,nullptr,&Nested)) return 22;
    creations.Finish();
    std::puts("propdev transcript: duplicate order, coordinates, zero return, once-only call and restoration PASS");
    return 0;
}
