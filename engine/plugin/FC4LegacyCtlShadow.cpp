// SPDX-License-Identifier: GPL-3.0-or-later
// Published vtable boundaries only (legacy-control sections 2,3,6,7).
// Owner replay and a primitive recording widget host. Partial paths stay explicit.
#include "FC4LegacyCtlShadow.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "compat/FC4Compat.h"
#include "fc4.h"
#include <windows.h>
#include <atomic>
#include <cstring>
#include "FC4LegacyCtlRecording.h"

extern "C" int32_t fc4_legacyctl_shadow_replay(const uint32_t*, uint32_t, uint32_t, uint32_t*);

namespace fc4 {
namespace {
std::atomic<uint32_t> compared{0}, differences{0}, builds{0}, buildGetters{0};
std::atomic<uint32_t> operationCalls[7]{}, operationDifferences[7]{};
bool installed = false;
#if defined(_M_IX86)
using QueryFn = bool(__fastcall*)(void*, void*, uint32_t, void**);
using RefFn = uint32_t(__fastcall*)(void*, void*);
using OwnFn = void(__fastcall*)(void*, void*, bool);
using ControlFn = void*(__fastcall*)(void*, void*);
using BuildFn = bool(__fastcall*)(void*, void*, void*);
using ImplementFn = bool(__fastcall*)(void*, void*, uint32_t, uint32_t, uint32_t, uint32_t);

struct State { uint32_t words[5]; };
State Snapshot(const void* owner) {
    State s{};
    const auto* p = static_cast<const uint8_t*>(owner);
    std::memcpy(s.words, p + 0x0C, 16);
    s.words[4] = p[0x1C];
    return s;
}
enum Slot { Query, AddRef, Build, Own, Control, LegacyOwn, Implement,
    Parent, Force, Procedure, ID, SetID, Fragment, SlotCount };
struct Entry { uintptr_t address, expected; void* replacement; };
void* originals[SlotCount]{};

// No native post-state enters Rust: inputs are BEFORE state and method argument.
void Compare(const State& before, const void* owner, uint32_t op, uint32_t argument,
             uint32_t nativeValue, bool checkValue, const void* queryOut = nullptr, const void* rejectedBytes = nullptr) {
    uint32_t result[6]{};
    const bool ok = fc4_legacyctl_shadow_replay(before.words, op, argument, result) == 1;
    const State after = Snapshot(owner);
    uint32_t bad = ok ? 0 : 1;
    if (ok) {
        for (unsigned f = 0; f < 5; ++f) {
            if (after.words[f] != result[f]) {
                ++bad;
                if (differences.load() < 8) Log("legacyctl field difference: op=%u field=%u native=%08X core=%08X",
                    op, f, after.words[f], result[f]);
            }
        }
        if (op == 3) {
            const bool accepted = result[5] != 0;
            if ((nativeValue != 0) != accepted) ++bad;
            if (accepted) {
                const void* expected = static_cast<const uint8_t*>(owner) + result[5] - 1;
                if (!queryOut || std::memcmp(queryOut, &expected, sizeof(expected)) != 0) ++bad;
            } else if (queryOut && std::memcmp(queryOut, rejectedBytes, sizeof(void*)) != 0) {
                ++bad;
            }
        } else if (checkValue && nativeValue != result[5]) {
            ++bad;
        }
    }
    ++compared;
    ++operationCalls[op];
    operationDifferences[op] += bad;
    differences += bad;
}
bool __fastcall ImplementHook(void* self, void*, uint32_t a, uint32_t b, uint32_t c, uint32_t d) {
    const State before = Snapshot(self);
    const bool value = reinterpret_cast<ImplementFn>(originals[Implement])(self, nullptr, a, b, c, d);
    Compare(before, self, 4, 0, value, true);
    return value;
}
bool __fastcall QueryHook(void* self, void*, uint32_t iid, void** out) {
    const State before = Snapshot(self);
    // QI callers may supply an uninitialized out-pointer. Keep its character
    // representation opaque throughout; never evaluate it as a pointer/integer.
    unsigned char untouched[sizeof(void*)]{};
    if (out) std::memcpy(&untouched, out, sizeof(untouched));
    const bool value = reinterpret_cast<QueryFn>(originals[Query])(self, nullptr, iid, out);
    Compare(before, self, 3, iid, value, true, out, untouched);
    return value;
}
uint32_t __fastcall RefHook(void* self, void*) {
    const State before = Snapshot(self);
    const uint32_t value = reinterpret_cast<RefFn>(originals[AddRef])(self, nullptr);
    Compare(before, self, 0, 0, value, true);
    return value;
}
void __fastcall OwnHook(void* self, void*, bool own) {
    const State before = Snapshot(self);
    reinterpret_cast<OwnFn>(originals[Own])(self, nullptr, own);
    Compare(before, self, 1, own, 0, false);
}
void __fastcall LegacyOwnHook(void* self, void*, bool own) {
    void* owner = static_cast<uint8_t*>(self) - 8;
    const State before = Snapshot(owner);
    reinterpret_cast<OwnFn>(originals[LegacyOwn])(self, nullptr, own);
    Compare(before, owner, 1, own, 0, false);
}
void* __fastcall ControlHook(void* self, void*) {
    void* owner = static_cast<uint8_t*>(self) - 8;
    const State before = Snapshot(owner);
    void* value = reinterpret_cast<ControlFn>(originals[Control])(self, nullptr);
    Compare(before, owner, 2, 0, reinterpret_cast<uint32_t>(value), true);
    legacyrecord::Record(legacyrecord::Call(6,self,0xc),legacyrecord::H(value));
    return value;
}
template<class F> uint32_t Capture(void* owner,uint32_t op,const std::array<uint32_t,4>& args,void* props,F invoke,
    bool scalar=true,void* rectOut=nullptr) {
    using namespace legacyrecord;
    HookScope hooks;
    const State before=Snapshot(owner);
    Input input;
    Trace trace;
    trace.exhaustive=Setup();
    {
        Quiet q;
        if(props) input.Capture(props);
        ObserveWindow(reinterpret_cast<void*>(before.words[1]));
        ObserveWindow(reinterpret_cast<void*>(before.words[2]));
        ObserveUnknown(reinterpret_cast<void*>(before.words[3]));
        if(op==1 || op==2 || op==3) ObserveUnknown(reinterpret_cast<void*>(args[0]));
    }
    uint32_t native[10]{};
    { Scope scope(trace); native[5]=invoke(); }
    const State after=Snapshot(owner);
    std::memcpy(native,after.words,sizeof(after.words));
    if(rectOut) std::memcpy(native+6,rectOut,16);
    trace.exhaustive=hooks.Finish() && trace.exhaustive;
    // Build traces do not yet compare constructor defaults/final widget trees
    // or temporary metadata lifetimes. Never award full SetParameters credit.
    Replay(trace,before.words,op,args,input,native,scalar,UINT32_MAX,op!=0);
    return native[5];
}
bool __fastcall ParentHook(void* self,void*,void* parent) {
    return Capture(self,1,{reinterpret_cast<uint32_t>(parent),0,0,0},nullptr,[&]{
        return reinterpret_cast<bool(__fastcall*)(void*,void*,void*)>(originals[Parent])(self,nullptr,parent);})!=0;
}
template<unsigned S,unsigned Op> void __fastcall PointerHook(void* self,void*,void* pointer) {
    Capture(static_cast<uint8_t*>(self)-8,Op,{reinterpret_cast<uint32_t>(pointer),0,0,0},nullptr,[&]{
        reinterpret_cast<void(__fastcall*)(void*,void*,void*)>(originals[S])(self,nullptr,pointer);return 0u;},false);
}
uint32_t __fastcall IDHook(void* self,void*) {
    return Capture(self,4,{},nullptr,[&]{return reinterpret_cast<uint32_t(__fastcall*)(void*,void*)>(originals[ID])(self,nullptr);});
}
void __fastcall SetIDHook(void* self,void*,uint32_t id) {
    Capture(self,5,{id,0,0,0},nullptr,[&]{reinterpret_cast<void(__fastcall*)(void*,void*,uint32_t)>(originals[SetID])(self,nullptr,id);return 0u;},false);
}
// SPEC-GAP: §3's +4 vtable extent overlaps the primary table (LC17).
// Do not patch its area/stub slots until the table identity is resolved.
bool __fastcall FragmentHook(void* self,void*,void** out) {
    // LC8/Q9: populated native fragments cannot yet earn full method credit.
    void* owner=static_cast<uint8_t*>(self)-8;
    if(Snapshot(owner).words[1]) {
        ++legacyrecord::stats[9].calls;
        return reinterpret_cast<bool(__fastcall*)(void*,void*,void**)>(originals[Fragment])(self,nullptr,out);
    }
    unsigned char untouched[sizeof(void*)]{};
    if(out) std::memcpy(untouched,out,sizeof(untouched));
    const bool value=Capture(owner,9,{},nullptr,[&]{return reinterpret_cast<bool(__fastcall*)(void*,void*,void**)>(originals[Fragment])(self,nullptr,out);})!=0;
    if(out && std::memcmp(untouched,out,sizeof(untouched))) ++legacyrecord::stats[9].bad;
    return value;
}
bool __fastcall BuildHook(void* self, void*, void* props) {
    const bool value = Capture(self,0,{},props,[&]{return reinterpret_cast<BuildFn>(originals[Build])(self,nullptr,props);})!=0;
    ++builds;
    // GETTER probe of a native build, explicitly not a replay of props.
    // Published borrowed GetControl: never AddRef/Release its result.
    ControlHook(static_cast<uint8_t*>(self) + 8, nullptr);
    ++buildGetters;
    return value;
}
bool Install() {
    // Compatibility values exclusively from legacy-control section 3.
    Entry entries[SlotCount] = {
        {0x00AD5360, 0x0094C431, reinterpret_cast<void*>(&QueryHook)},
        {0x00AD5364, 0x0094C47C, reinterpret_cast<void*>(&RefHook)},
        {0x00AD536C, 0x00957A9B, reinterpret_cast<void*>(&BuildHook)},
        {0x00AD537C, 0x0094C314, reinterpret_cast<void*>(&OwnHook)},
        {0x00AD5330, 0x009C32AC, reinterpret_cast<void*>(&ControlHook)},
        {0x00AD5334, 0x0094C4D8, reinterpret_cast<void*>(&LegacyOwnHook)},
        {0x00AD5380, 0x00935EA0, reinterpret_cast<void*>(&ImplementHook)},
        {0x00AD5384, 0x0094C31E, reinterpret_cast<void*>(&ParentHook)},
        {0x00AD5338, 0x0094C3A1, reinterpret_cast<void*>(&PointerHook<Force,2>)},
        {0x00AD533C, 0x0094C375, reinterpret_cast<void*>(&PointerHook<Procedure,3>)},
        {0x00AD5374, 0x0094C3CD, reinterpret_cast<void*>(&IDHook)},
        {0x00AD5378, 0x0094C3E1, reinterpret_cast<void*>(&SetIDHook)},
        {0x00AD5340, 0x0096638D, reinterpret_cast<void*>(&FragmentHook)},
    };
    if (!compat::IdentifiedVersion()) return false;
    // Validate EVERY entry before changing any entry; no partial installation.
    for (unsigned i = 0; i < SlotCount; ++i) {
        MEMORY_BASIC_INFORMATION memory{};
        auto** entry = reinterpret_cast<void**>(entries[i].address);
        if (VirtualQuery(entry, &memory, sizeof(memory)) != sizeof(memory) || memory.State != MEM_COMMIT ||
            (memory.Protect & (PAGE_NOACCESS | PAGE_GUARD)) != 0 ||
            reinterpret_cast<uintptr_t>(*entry) != entries[i].expected) return false;
        originals[i] = *entry;
    }
    auto* start = reinterpret_cast<void*>(0x00AD5324);
    DWORD previous = 0;
    if (!VirtualProtect(start, 0x68, PAGE_READWRITE, &previous)) return false;
    for (const auto& entry : entries) *reinterpret_cast<void**>(entry.address) = entry.replacement;
    DWORD ignored = 0;
    VirtualProtect(start, 0x68, previous, &ignored);
    return true;
}
#endif
}  // namespace

void LegacyCtlShadowStartup() {
    if (SettingInt("legacyctl_shadow", nullptr, 0) == 0 || installed) return;
#if defined(_M_IX86)
    installed = Install();
    if(installed) Log("legacyctl: recording interpreter entries %s",legacyrecord::InstallInterpreters()?"installed":"partial/refused");
#endif
    Log("legacyctl: recording shadow %s; build/type/resource gaps retain complete=0; control-area LC17 unpatched",
        installed ? "installed" : "NOT installed");
}
void LegacyCtlShadowMonth(int32_t julian) {
    if (SettingInt("legacyctl_shadow", nullptr, 0) == 0) return;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    // Cumulative counts retain sparse panel calls. Empty/refused != exact.
    const uint32_t calls = compared.load();
    uint32_t replayBad=0;
#if defined(_M_IX86)
    for(const auto& s:legacyrecord::stats) replayBad+=s.bad.load();
#endif
    Log("%04d-%02d-%02d legacyctl shadow: compared=%u builds=%u build_getters=%u "
        "addref=%u own=%u control=%u qi=%u build_replay=partial: mismatches=%u",
        date.year, date.month, date.day_of_month, calls, builds.load(), buildGetters.load(),
        operationCalls[0].load(), operationCalls[1].load(), operationCalls[2].load(), operationCalls[3].load(),
        differences.load() + replayBad + ((!installed || calls == 0) ? 1u : 0u));
    const char* addresses[] = {"0x0094C47C", "0x0094C314", "0x009C32AC", "0x0094C431", "0x00935EA0", "0x0091559A", "0x0092B33A"};
    const char* methods[] = {"AddRef", "SetOwnControl", "GetControl", "QueryInterface", "ImplementInterface", "AreaStub", "AreaStub2"};
    for (unsigned i = 0; i < 7; ++i) {
        Log("legacyctl method %s cGZLegacyControl::%s (month) %04d-%02d-%02d: calls=%u complete=%u mismatches=%u; resyncs=0",
            addresses[i], methods[i], date.year, date.month, date.day_of_month,
            operationCalls[i].load(), operationCalls[i].load(), operationDifferences[i].load());
    }
#if defined(_M_IX86)
    const char* replayAddresses[]={"0x00957A9B","0x0094C31E","0x0094C3A1","0x0094C375","0x0094C3CD","0x0094C3E1","0x0094C3F5","0x0094C413","0x0094C77D","0x0096638D"};
    const char* replayMethods[]={"SetParameters","SetParent","ForceControl","UseWinProc","GetID","SetID","SetArea","GetArea","DoExceptions","GetScriptFragment"};
    for(unsigned i=0;i<10;++i) {
        const auto& s=legacyrecord::stats[i];
        Log("legacyctl method %s cGZLegacyControl::%s (month) %04d-%02d-%02d: calls=%u complete=%u mismatches=%u; resyncs=0",
            replayAddresses[i],replayMethods[i],date.year,date.month,date.day_of_month,s.calls.load(),s.complete.load(),s.bad.load());
    }
    for(unsigned i=0;i<std::size(legacyrecord::interpreters);++i) {
        const auto& spec=legacyrecord::interpreters[i];const auto& s=legacyrecord::stats[10+i];
        Log("legacyctl method 0x%08X cGZLegacyControl::%s (month) %04d-%02d-%02d: calls=%u complete=%u mismatches=%u; resyncs=0",
            static_cast<unsigned>(spec.entry),spec.name,date.year,date.month,date.day_of_month,s.calls.load(),s.complete.load(),s.bad.load());
    }
#endif
}
}  // namespace fc4
