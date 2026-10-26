// SPDX-License-Identifier: GPL-3.0-or-later
// Interface boundaries from ui-windows §3 and view3d-window §§1,6,6f.
#include "FC4View3DShadow.h"
#include "FC4View3DObservation.h"
#include "FC4Config.h"
#include "FC4HarnessLogic.h"
#include "FC4InplaceDetour.h"
#include "FC4Log.h"
#include <windows.h>
#include <atomic>
#include <cstring>
#include <exception>

extern "C" bool fc4_view3d_shadow_replay(uint32_t, const uint32_t*, uint32_t*, uint32_t*, uint32_t*);

namespace fc4 {
namespace {
using namespace view3dshadow;
struct Method {
    unsigned group, slot;
    uint32_t address;
    const char* name;
    bool replay;
    uintptr_t original = 0;
    std::atomic<uint64_t> calls{0}, complete{0}, mismatches{0};
};
// Partial methods deliberately remain visible in the same monthly ledger.
Method methods[] = {
    {0,3,0x005F8400,"AsIGZWin",true},
    {0,4,0x0043FD00,"GetRenderer",true},
    {0,5,0x007F0FE0,"GetModelMaker",true},
    {0,7,0x007F1C10,"ProcessCommand",false},
    {0,8,0x007E60E0,"SetZoom",false},
    {0,9,0x007E62C0,"ZoomIn",false},
    {0,10,0x007E6320,"ZoomOut",false},
    {0,11,0x007E6150,"SetZoomLimits",false},
    {0,12,0x007E5780,"CanZoomIn",true},
    {0,13,0x007E57A0,"CanZoomOut",true},
    {0,14,0x007E61D0,"SetRotation",false},
    {0,15,0x007E6220,"RotateLeft",false},
    {0,16,0x007E6270,"RotateRight",false},
    {0,17,0x007E6380,"SetZoomAndRotation",false},
    {0,20,0x007E57E0,"GetCurrentViewLevelInfo",true},
    {0,22,0x007E57F0,"GetCurrentViewLevel",true},
    {0,31,0x007E56F0,"GetKeyAccelerator",true},
    {0,33,0x007F64F0,"SetScrolling",false},
    {0,34,0x007E5740,"ScrollStop",true},
    {0,39,0x007E5900,"GetTerrainQueryEnabled",true},
    {0,51,0x007F1060,"GetLastSelectedBuilding",true},
    {0,52,0x007F1080,"GetLastSelectedLot",true},
    {0,53,0x007F1020,"SetLastSelectedBuilding",true},
    {0,54,0x007F1040,"SetLastSelectedLot",true},
    {1,3,0x007F53C0,"DoMessage",false},
    {2,3,0x007F3E50,"DoMessage",false},
    {2,88,0x007F6200,"GZPaint",false},
    {2,143,0x007E5BA0,"GZOnCommand",true},
    {3,3,0x007F58E0,"FilterMessage",false},
};
bool installed = false, zoomBound = false;
uintptr_t zoomOriginal = 0;
struct Capture {
    Capture* previous = nullptr;
    void* receiver = nullptr;
    uint32_t zoom = 0, reads = 0;
};
thread_local Capture* capture = nullptr;

uintptr_t Address(uint32_t va) {
    return reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr)) + (va - 0x00400000);
}
template<class T> T Load(const void* p, size_t offset) {
    T value; std::memcpy(&value, static_cast<const uint8_t*>(p) + offset, sizeof value); return value;
}
template<class T> void Store(State& state, size_t offset, T value) {
    std::memcpy(state.data() + offset, &value, sizeof value);
}
Packet Inputs(const void* owner, uint32_t a, uint32_t b) {
    return {{static_cast<uint32_t>(reinterpret_cast<uintptr_t>(owner)),
        Load<uint32_t>(owner,0xF4), Load<uint32_t>(owner,0xF8), Load<uint32_t>(owner,0x120),
        Load<uint32_t>(owner,0x324), Load<uint8_t>(owner,0x126),
        Load<uint32_t>(owner,0x43C), Load<uint8_t>(owner,0x444),
        Load<uint32_t>(owner,0x440), Load<uint8_t>(owner,0x445),
        Load<uint8_t>(owner,0x330), Load<uint32_t>(owner,0x32C), Load<uint32_t>(owner,0x334),
        Load<uint8_t>(owner,0x338), Load<uint8_t>(owner,0x339), Load<uint32_t>(owner,0x33C),
        Load<uint32_t>(owner,0x364), Load<uint32_t>(owner,0x368), 0,
        Load<uint32_t>(owner,0xF0), a, b}};
}
uint32_t __fastcall VirtualZoom(void* self, void*) {
    using Fn = uint32_t(__fastcall*)(void*, void*);
    const auto result = reinterpret_cast<Fn>(zoomOriginal)(self, nullptr);
    if (capture) {
        ++capture->reads;
        capture->receiver = self;
        capture->zoom = result;
    }
    return result;
}
void* Detour(uintptr_t site, void* hook) {
    auto* trampoline = static_cast<uint8_t*>(VirtualAlloc(nullptr,64,MEM_COMMIT|MEM_RESERVE,PAGE_EXECUTE_READWRITE));
    if (!trampoline) return nullptr;
    auto* entry = reinterpret_cast<uint8_t*>(site);
    const auto size = inplace::Trampoline(entry,24,site,trampoline,reinterpret_cast<uintptr_t>(trampoline));
    if (!size) { VirtualFree(trampoline,0,MEM_RELEASE); return nullptr; }
    DWORD old;
    if (!VirtualProtect(entry,size,PAGE_EXECUTE_READWRITE,&old)) { VirtualFree(trampoline,0,MEM_RELEASE); return nullptr; }
    inplace::Jump(entry,site,reinterpret_cast<uintptr_t>(hook));
    for (unsigned i=5;i<size;++i) entry[i]=0x90;
    DWORD ignored; VirtualProtect(entry,size,old,&ignored);
    FlushInstructionCache(GetCurrentProcess(),entry,size);
    return trampoline; // Retained with the pinned DLL until process exit.
}
Method& Find(unsigned group, unsigned slot) {
    for (auto& method : methods) if (method.group == group && method.slot == slot) return method;
    std::terminate();
}
Evidence Compare(const Method& method, const State& before, const void* owner, Packet input,
                 uint32_t nativeResult, const Capture& observed) {
    Evidence evidence;
    const bool zoom = method.slot == 12 || method.slot == 13;
    if (zoom && (!zoomBound || observed.reads == 0)) return evidence;
    input[18] = observed.zoom;
    Packet output{}; uint32_t value = 0, reads = 0;
    if (!fc4_view3d_shadow_replay(method.slot,input.data(),output.data(),&value,&reads)) return evidence;
    State expected = before;
    if (method.slot == 34) {
        Store(expected,0x339,static_cast<uint8_t>(output[14])); Store(expected,0x33C,output[15]);
    } else if (method.slot == 53) {
        Store(expected,0x43C,output[6]); Store(expected,0x444,static_cast<uint8_t>(output[7]));
    } else if (method.slot == 54) {
        Store(expected,0x440,output[8]); Store(expected,0x445,static_cast<uint8_t>(output[9]));
    }
    evidence.state = std::memcmp(owner,expected.data(),expected.size()) == 0;
    evidence.calls = observed.reads == reads && (!zoom || reinterpret_cast<uintptr_t>(observed.receiver) == input[19]);
    if (method.slot == 20) evidence.out = input[20] != 0 && Load<uint32_t>(reinterpret_cast<void*>(input[20]),0) == value;
    else if (method.slot != 34 && method.slot != 53 && method.slot != 54) {
        // GZOnCommand returns AL; the other bound getters have full EAX contracts.
        evidence.result = (method.slot == 143 ? (nativeResult & 0xff) : nativeResult) == value;
    }
    // SPEC-GAP: these four slots have unresolved eax-or-void returns (§3.3);
    // compare their observable writes, but never award complete method credit.
    evidence.bound = method.slot != 20 && method.slot != 34 && method.slot != 53 && method.slot != 54;
    return evidence;
}

template<unsigned Group, unsigned Slot, class... Args>
uint32_t __fastcall Hook(void* self, void*, Args... args) {
    auto& method = Find(Group,Slot);
    using Fn = uint32_t(__fastcall*)(void*,void*,Args...);
    State before{}; Packet input{};
    auto* owner = static_cast<uint8_t*>(self) - (Group == 1 ? 4 : Group == 2 ? 12 : Group == 3 ? 8 : 0);
    const uint32_t arguments[] = {static_cast<uint32_t>(args)...,0,0};
    if (method.replay) {
        std::memcpy(before.data(),owner,before.size());
        input = Inputs(owner,arguments[0],arguments[1]);
    }
    Capture observed{capture};
    capture = &observed;
    const auto result = reinterpret_cast<Fn>(method.original)(self,nullptr,args...);
    capture = observed.previous;
    const auto call = ++method.calls;
    if (method.replay) {
        const auto evidence = Compare(method,before,owner,input,result,observed);
        if (evidence.Complete()) ++method.complete;
        if (!evidence.Exact()) {
            ++method.mismatches;
            if (call <= 10) Log("view3d mismatch 0x%08X cSC4View3DWin::%s call=%llu state=%u return=%u out=%u calls=%u",
                method.address,method.name,call,evidence.state,evidence.result,evidence.out,evidence.calls);
        }
    }
    return result;
}
template<unsigned Group, unsigned Slot, class... Args>
bool Bind(void** table) {
    auto& method = Find(Group,Slot);
    if (reinterpret_cast<uintptr_t>(table[Slot]) != Address(method.address)) return false;
    method.original = reinterpret_cast<uintptr_t>(table[Slot]);
    DWORD old;
    if (!VirtualProtect(table+Slot,sizeof(void*),PAGE_READWRITE,&old)) return false;
    InterlockedExchangePointer(table+Slot,reinterpret_cast<void*>(&Hook<Group,Slot,Args...>));
    DWORD ignored; VirtualProtect(table+Slot,sizeof(void*),old,&ignored);
    return true;
}
} // namespace

void View3DShadowStartup() {
    if (installed || SettingInt("view3d_shadow",nullptr,0) != 1) return;
    HMODULE pinned = nullptr;
    if (!GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_PIN,
        reinterpret_cast<LPCSTR>(&View3DShadowStartup),&pinned)) return;
    installed = true;
    zoomOriginal = reinterpret_cast<uintptr_t>(Detour(Address(0x007CBC40),reinterpret_cast<void*>(&VirtualZoom)));
    zoomBound = zoomOriginal != 0;
    auto** primary = reinterpret_cast<void**>(Address(0x00ABCE40));
    auto** target = reinterpret_cast<void**>(Address(0x00ABCE2C));
    auto** window = reinterpret_cast<void**>(Address(0x00ABCBC0));
    auto** filter = reinterpret_cast<void**>(Address(0x00ABCE1C));
    unsigned bound = 0;
    bound += Bind<0,3>(primary); bound += Bind<0,4>(primary); bound += Bind<0,5>(primary);
    bound += Bind<0,7,uint32_t,uint32_t,uint32_t>(primary);
    bound += Bind<0,8,uint32_t>(primary); bound += Bind<0,9>(primary); bound += Bind<0,10>(primary);
    bound += Bind<0,11,uint32_t,uint32_t>(primary); bound += Bind<0,12>(primary); bound += Bind<0,13>(primary);
    bound += Bind<0,14,uint32_t>(primary); bound += Bind<0,15>(primary); bound += Bind<0,16>(primary);
    bound += Bind<0,17,uint32_t,uint32_t>(primary); bound += Bind<0,20,uint32_t>(primary); bound += Bind<0,22>(primary);
    bound += Bind<0,31>(primary); bound += Bind<0,33,uint32_t,uint32_t,uint32_t>(primary); bound += Bind<0,34>(primary);
    bound += Bind<0,39>(primary); bound += Bind<0,51>(primary); bound += Bind<0,52>(primary);
    bound += Bind<0,53,uint32_t>(primary); bound += Bind<0,54,uint32_t>(primary);
    bound += Bind<1,3,uint32_t>(target); bound += Bind<2,3,uint32_t>(window); bound += Bind<2,88>(window);
    bound += Bind<2,143,uint32_t,uint32_t>(window); bound += Bind<3,3,uint32_t,uint32_t>(filter);
    Log("view3d shadow installed boundaries=%u/29 virtual_zoom=%u; invocation inputs only; resyncs=0",bound,zoomBound);
    Log("view3d incomplete: messages need tutorial/base/UI/dispatcher traces; camera changes need renderer, timer, hidden-pause and invalidation traces; paint needs drawing/deferred/timer traces; complete=0");
    Log("view3d incomplete: slots 20/34/53/54 compare writes; unresolved return contracts retain complete=0");
}

void View3DShadowMonth(int32_t julian) {
    if (!installed) return;
    const auto date = harness::CivilFromJulian(julian);
    for (const auto& method : methods)
        Log("view3d method 0x%08X cSC4View3DWin::%s (month) %04d-%02d-%02d: calls=%llu complete=%llu mismatches=%llu; resyncs=0",
            method.address,method.name,date.year,date.month,date.day,method.calls.load(),method.complete.load(),method.mismatches.load());
}
} // namespace fc4
