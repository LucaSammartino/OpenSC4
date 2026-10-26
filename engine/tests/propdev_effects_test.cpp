// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4PropDevEffects.h"
#include <cstdio>

namespace {
template<class T> void Put(void* p, size_t offset, T value) {
    std::memcpy(static_cast<uint8_t*>(p) + offset, &value, sizeof(value));
}
using Snapshot = fc4::PropEffectSnapshot;
std::array<uint8_t,0x9c8> generator{};
unsigned calls = 0;
void __fastcall State(void* p, void*, uint32_t state) {
    ++calls; Put<uint8_t>(p,0x63,static_cast<uint8_t>(state));
}
void __fastcall Chance(void* p, void*, uint8_t chance) { ++calls; Put(p,0x62,chance); }
void __fastcall Timer(void* p, void*, uint32_t mask, bool on) {
    ++calls;
    const uint8_t before = Snapshot::At<uint8_t>(p,0x60);
    const uint8_t after = static_cast<uint8_t>(on ? before | mask : before & ~mask);
    Put(p,0x60,after);
    if ((before&15)!=15 && (after&15)==15) {
        // Fixture's next raw word is zero; U(100)=0, accepted at chance 100.
        const uint32_t next = Snapshot::At<uint32_t>(generator.data(),0x9c0);
        Put(generator.data(),0x9c0,next+4);
        Put<int32_t>(generator.data(),0x9c4,0);
        Put<uint8_t>(p,0x63,0);
    }
}
void __fastcall Tick(void* p, void*, const void*) {
    ++calls;
    auto* range = Snapshot::At<uint8_t*>(p,0x54);
    if (range) {
        Put<uint8_t>(p,0x60,11);
        Put<uint32_t>(range,0x0c,110);
    }
}
void __fastcall Range(void* p, void*, uint32_t, uint32_t, uint8_t, uint8_t) {
    ++calls; Put<void*>(p,0x54,nullptr);
}
uint32_t Now(const void*) { return 100; }
uint32_t Date(const void* p) { return *static_cast<const uint32_t*>(p); }
}

int main() {
    static_assert(sizeof(void*)==4,"PV18 native fixture is x86");
    void* table[22]{};
    table[11]=reinterpret_cast<void*>(&State); table[15]=reinterpret_cast<void*>(&Range);
    table[16]=reinterpret_cast<void*>(&Tick); table[17]=reinterpret_cast<void*>(&Chance);
    table[21]=reinterpret_cast<void*>(&Timer);
    std::array<uint8_t,0x70> prop{};
    std::array<uint8_t,0x28> range{};
    Put(prop.data(),0,static_cast<void*>(table));
    Put(prop.data(),0x54,range.data());
    Put<uint8_t>(prop.data(),0x60,9); Put<uint8_t>(prop.data(),0x62,100); Put<uint8_t>(prop.data(),0x63,1);
    Put<uint32_t>(range.data(),0,10); Put<uint32_t>(range.data(),4,2);
    Put<uint32_t>(range.data(),0x0c,100); Put<uint32_t>(range.data(),0x1c,102);
    Put<uint32_t>(range.data(),0x10,0x12345678); Put<uint32_t>(range.data(),0x20,0x87654321);
    const uint32_t base = reinterpret_cast<uintptr_t>(generator.data());
    Put(generator.data(),0x9c0,base+623*4); Put<int32_t>(generator.data(),0x9c4,1);

    fc4_prop_effect_state snapshot{};
    if (!Snapshot::Capture(prop.data(),table,snapshot) || snapshot.start_day!=100 || snapshot.end_day!=102) return 1;
    const auto initial=snapshot;
    if (Snapshot::Capture(prop.data(),range.data(),snapshot) || std::memcmp(&snapshot,&initial,sizeof(initial))) return 2;
    fc4::PropEffectRandom random;
    if (!random.Capture(generator.data()) || random.index!=623) return 3;
    Put<int32_t>(generator.data(),0x9c4,2);
    if (random.Capture(generator.data())) return 4;
    Put<int32_t>(generator.data(),0x9c4,1);

    fc4::PropEffectTrace trace, busy;
    if (!trace.Begin(table,generator.data(),&Now,&Date) || busy.Begin(table,generator.data(),&Now,&Date)) return 5;
    // Calls from a delegated service are not developer outputs. Insertion
    // callbacks resume at their real nested boundary, then restore the pause.
    {
        fc4::PropEffectTrace::Recording delegated(false);
        reinterpret_cast<void (__fastcall*)(void*,void*,uint8_t)>(table[17])(prop.data(),nullptr,100);
        {
            fc4::PropEffectTrace::Recording insertion(true);
            const uint32_t date=100;
            reinterpret_cast<void (__fastcall*)(void*,void*,const void*)>(table[16])(prop.data(),nullptr,&date);
        }
        reinterpret_cast<void (__fastcall*)(void*,void*,uint8_t)>(table[17])(prop.data(),nullptr,100);
    }
    reinterpret_cast<void (__fastcall*)(void*,void*,uint32_t,bool)>(table[21])(prop.data(),nullptr,4,true);
    reinterpret_cast<void (__fastcall*)(void*,void*,uint32_t)>(table[11])(prop.data(),nullptr,256);
    reinterpret_cast<void (__fastcall*)(void*,void*,uint32_t,uint32_t,uint8_t,uint8_t)>(table[15])
        (prop.data(),nullptr,0,0,0,0);
    trace.Finish();
    if (!trace.Valid() || calls!=6 || trace.Events().size()!=4 || table[16]!=reinterpret_cast<void*>(&Tick)) return 6;
    fc4::PropEffectReplay replay(trace);
    const uint64_t token=reinterpret_cast<uintptr_t>(prop.data());
    replay.Call(token,16,{},100);
    replay.Call(token,21,{4,1,0,0});
    replay.Call(token,11,{256,0,0,0});
    replay.Call(token,15,{},100);
    if (!replay.Complete() || replay.Matched()!=4 || replay.Draws()!=1 || replay.Changes()!=2) return 7;
    if (trace.Events()[3].after.present || trace.Events()[3].after.timer!=15) return 8;
    auto changed=trace.Events()[1];
    changed.randomAfter.index=623;
    fc4_prop_effect_result effect{};
    if (changed.Check(effect)) return 9;
    fc4::PropEffectReplay missing(trace), wrong(trace);
    if (missing.Complete()) return 10;
    wrong.Call(token,16,{},99);
    if (wrong.Matched() || wrong.Complete()) return 11;
    // k=624/r=0 is valid and must not dereference the one-past word.
    if (!random.Capture(generator.data()) || random.index!=624) return 12;
    Put(prop.data(),0x54,static_cast<void*>(nullptr));
    if (!Snapshot::Capture(prop.data(),table,snapshot) || snapshot.present || snapshot.start_day) return 13;
    std::puts("propdev PV18 semantic snapshots, scoped calls and RNG comparison passed");
    return 0;
}
