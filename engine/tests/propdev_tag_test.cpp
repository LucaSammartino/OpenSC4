// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4PropDevTag.h"
#include <cstdio>

namespace {
using Trace = fc4::PropTagTrace;
using Replay = fc4::PropTagReplay;
template<class T> void Put(void* p, size_t offset, T value) {
    std::memcpy(static_cast<uint8_t*>(p) + offset, &value, sizeof(value));
}
struct Variant { void** table; uint32_t value; bool readable; };
struct Property { void** table; Variant* value; uint32_t id; };
struct Fault {
    bool absent = false, skipAdd = false, skipRelease = false, extraHolder = false;
    bool wrongKey = false, wrongIndex = false, wrongOutput = false, wrongRef = false;
    bool wrongOrder = false, addFlag = false, wrongId = false, wrongHolder = false;
    bool nested = false;
};
Fault fault;
Property* property;
size_t nested = 0;
unsigned nativeCalls = 0;
uint32_t __fastcall Hold(void* c, void*) {
    ++nativeCalls;
    auto* p = static_cast<uint8_t*>(c) - 8;
    const auto count = fc4::PropEffectSnapshot::At<uint32_t>(p, 0x14) + 1;
    Put(p, 0x14, count);
    return fault.wrongRef ? count + 1 : count;
}
uint32_t __fastcall Release(void* c, void*) {
    ++nativeCalls;
    auto* p = static_cast<uint8_t*>(c) - 8;
    const auto count = fc4::PropEffectSnapshot::At<uint32_t>(p, 0x14) - 1;
    Put(p, 0x14, count);
    return count;
}
void* __fastcall Holder(void* c, void*) {
    ++nativeCalls;
    return static_cast<uint8_t*>(c) + (fault.wrongHolder ? 12 : 8);
}
void* __fastcall Lookup(void*, void*, uint32_t) {
    ++nativeCalls;
    if (fault.nested) ++nested;
    return fault.absent ? nullptr : property;
}
bool __fastcall Add(void*, void*, void*, bool) { ++nativeCalls; return false; }
void* __fastcall Value(void* p, void*) { ++nativeCalls; return static_cast<Property*>(p)->value; }
void __fastcall Assign(void* v, void*, uint32_t x) {
    ++nativeCalls;
    static_cast<Variant*>(v)->value = fault.wrongOutput ? x + 1 : x;
}
bool Read(const void* p, uint32_t* x) {
    const auto* v = static_cast<const Variant*>(p);
    *x = v->value;
    return v->readable;
}
uint32_t Id(const void* p) { return static_cast<const Property*>(p)->id; }
size_t Count() { return nested; }

bool Run(Fault f, bool expected, bool readable = true) {
    fault = f; nested = 0; nativeCalls = 0;
    void* ptable[3]{};
    void* ctable[7]{};
    void* htable[16]{};
    void* propertyTable[6]{};
    void* variantTable[27]{};
    ctable[1] = reinterpret_cast<void*>(&Hold); ctable[2] = reinterpret_cast<void*>(&Release);
    ctable[6] = reinterpret_cast<void*>(&Holder);
    htable[9] = reinterpret_cast<void*>(&Lookup); htable[15] = reinterpret_cast<void*>(&Add);
    propertyTable[5] = reinterpret_cast<void*>(&Value); variantTable[26] = reinterpret_cast<void*>(&Assign);
    std::array<uint8_t, 0x70> prop{};
    Put(prop.data(), 0, static_cast<void*>(ptable)); Put(prop.data(), 8, static_cast<void*>(ctable));
    Put(prop.data(), 16, static_cast<void*>(htable)); Put<uint32_t>(prop.data(), 0x14, 4);
    Variant variant{variantTable, 99, readable};
    Property entry{propertyTable, &variant, f.wrongId ? 123u : 0x89A1C16Cu};
    property = &entry;
    Trace trace;
    if (!trace.Begin(ptable, ctable, &Read, &Id, &Count)) return false;
    const uint64_t token = reinterpret_cast<uintptr_t>(prop.data());
    auto* c = prop.data() + 8;
    trace.Watch(c, token);
    using Ref = uint32_t (__fastcall*)(void*,void*);
    using Get = void* (__fastcall*)(void*,void*);
    const auto held = reinterpret_cast<Ref>(ctable[1]);
    // An observer's own balanced holds must never appear as developer calls.
    {
        Trace::Recording observer(false);
        held(c,nullptr); reinterpret_cast<Ref>(ctable[2])(c,nullptr);
    }
    if (f.wrongOrder) reinterpret_cast<Get>(ctable[6])(c,nullptr);
    held(c,nullptr);
    void* h = reinterpret_cast<Get>(ctable[6])(c,nullptr);
    if (f.extraHolder) reinterpret_cast<Get>(ctable[6])(c,nullptr);
    if (!f.wrongHolder) {
        auto* p = reinterpret_cast<void* (__fastcall*)(void*,void*,uint32_t)>(htable[9])
            (h,nullptr,f.wrongKey ? 1 : 0x89A1C16C);
        if (!p) {
            p = property;
            if (!f.skipAdd) reinterpret_cast<bool (__fastcall*)(void*,void*,void*,bool)>(htable[15])
                (h,nullptr,p,f.addFlag);
        }
        auto* v = reinterpret_cast<Get>(propertyTable[5])(p,nullptr);
        reinterpret_cast<void (__fastcall*)(void*,void*,uint32_t)>(variantTable[26])(v,nullptr,f.wrongIndex ? 8 : 7);
    }
    if (!f.skipRelease) reinterpret_cast<Ref>(ctable[2])(c,nullptr);
    trace.Finish();
    const auto size = trace.Spans().front().events.size();
    const unsigned before = nativeCalls;
    held(c,nullptr); // a wrapper fetched before restoration must still forward once
    if (nativeCalls != before + 1 || trace.Spans().front().events.size() != size ||
        ctable[1] != reinterpret_cast<void*>(&Hold) || htable[9] != reinterpret_cast<void*>(&Lookup) ||
        propertyTable[5] != reinterpret_cast<void*>(&Value) || variantTable[26] != reinterpret_cast<void*>(&Assign))
        return false;
    Replay replay(trace);
    size_t cursor = 0;
    const bool matched = replay.Call(token, 7, cursor, [&](size_t end) { cursor = end; return true; });
    if ((matched && replay.Complete() && replay.Matched() == 1) != expected) {
        std::printf("tag case failed: expected=%u reason=%s events=%u\n", expected ? 1u : 0u,
            replay.Failure(), static_cast<unsigned>(size));
        return false;
    }
    if (expected) {
        Replay wrongBoundary(trace), missing(trace), extra(trace);
        if (missing.Complete()) return false;
        size_t wrong = 1;
        if (wrongBoundary.Call(token,7,wrong,[](size_t) { return true; }) || wrongBoundary.Complete()) return false;
        size_t n = 0;
        if (!extra.Call(token,7,n,[&](size_t end) { n = end; return true; }) ||
            extra.Call(token,7,n,[](size_t) { return true; }) || extra.Complete()) return false;
    }
    return true;
}
}

int main() {
    static_assert(sizeof(void*) == 4, "PV18 tag fixture is x86");
    if (!Run({},true)) return 1;
    Fault f;
    f.absent = true; if (!Run(f,true)) return 2; // false Add result is ignored, not retried
    f = {}; f.nested = true; if (!Run(f,true)) return 3;
    f = {}; f.wrongKey = true; if (!Run(f,false)) return 4;
    f = {}; f.wrongIndex = true; if (!Run(f,false)) return 5;
    f = {}; f.wrongOutput = true; if (!Run(f,false)) return 6;
    f = {}; f.wrongRef = true; if (!Run(f,false)) return 7;
    f = {}; f.wrongOrder = true; if (!Run(f,false)) return 8;
    f = {}; f.extraHolder = true; if (!Run(f,false)) return 9;
    f = {}; f.skipRelease = true; if (!Run(f,false)) return 10;
    f = {}; f.absent = true; f.skipAdd = true; if (!Run(f,false)) return 11;
    f = {}; f.absent = true; f.addFlag = true; if (!Run(f,false)) return 12;
    f = {}; f.absent = true; f.wrongId = true; if (!Run(f,false)) return 13;
    f = {}; f.wrongHolder = true; if (!Run(f,false)) return 14;
    if (!Run({},false,false)) return 15;
    // Undeclared primary objects must not get ordinary offsets or hooks.
    void* declared[3]{};
    void* other[3]{};
    void* ctable[7]{};
    ctable[1] = reinterpret_cast<void*>(&Hold);
    std::array<uint8_t,0x20> custom{};
    Put(custom.data(),0,static_cast<void*>(other)); Put(custom.data(),8,static_cast<void*>(ctable));
    Trace trace;
    if (!trace.Begin(declared,ctable,&Read,&Id,&Count)) return 16;
    trace.Watch(custom.data()+8,reinterpret_cast<uintptr_t>(custom.data()));
    trace.Finish();
    if (trace.Valid() || ctable[1] != reinterpret_cast<void*>(&Hold)) return 17;
    std::puts("propdev PV18 creation holds, concrete holder/tag writes and nested boundaries passed");
    return 0;
}
