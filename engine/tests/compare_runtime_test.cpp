// SPDX-License-Identifier: GPL-3.0-or-later
// Authored native code exercises dispatch modes; no original image is loaded.
#include "../plugin/FC4Inplace.cpp"
#include <cassert>
namespace fc4 {
void Log(const char*,...) {}
std::string Setting(const char*,const char*,const char**) {return {};}
long SettingInt(const char*,const char*,long fallback) {return fallback;}
}
unsigned nativeCalls=0,adapterCalls=0;
bool enterNested=false;
FC4_INPLACE_ENTRY(AuthoredInner,1,0)
void SaveFx(uint8_t* state) {
    __asm {
        mov eax,state
        fxsave [eax]
    }
}
extern "C" uint32_t __cdecl AuthoredNative(uint32_t* value) {
    ++nativeCalls;
    if (enterNested) {
        enterNested=false;
        reinterpret_cast<uint32_t(__cdecl*)(uint32_t*)>(AuthoredInner)(value);
    }
    return ++*value;
}
extern "C" bool fc4_compare_adapter(const char*,const uint32_t*,fc4::compare::Buffer* buffers,size_t count,fc4::compare::Result* result) {
    ++adapterCalls;if (count!=1||buffers[0].size!=4) return false;
    uint32_t value;std::memcpy(&value,buffers[0].data,4);++value;
    std::memcpy(buffers[0].data,&value,4);result->eax=value;return true;
}
extern "C" bool fc4_compare_fixture(const char*,unsigned,uint32_t*,fc4::compare::Buffer*,size_t) {return false;}
int main() {
    auto& fn=functions[0];fn.name="authored-unit";fn.ret="eax";fn.mode="compare";
    fn.receiver=false;fn.argCount=1;fn.pop=0;fn.fingerprint=123;
    fn.original=reinterpret_cast<void*>(AuthoredNative);
    fn.specs=fc4::compare::Parse("arg0+0/4/rw/0:u32!:1",1);
    auto* value=static_cast<uint32_t*>(VirtualAlloc(nullptr,4096,MEM_RESERVE|MEM_COMMIT,PAGE_READWRITE));assert(value);
    struct Frame {fc4::inplace::Registers regs;uint32_t args[16];} frame{};
    frame.args[0]=uint32_t(reinterpret_cast<uintptr_t>(value));
    fc4::inplace::MachineState machine;
    const auto run=[&](const char* mode) {
        modeOverride=mode;*value=41;SaveFx(machine.before);
        Dispatch(&frame.regs,&machine);
        assert(*value==42&&frame.regs.eax==42);
    };
    run("compare");assert(nativeCalls==1&&adapterCalls==1&&fn.counts[0].exact==1);
    run("record");assert(nativeCalls==2&&adapterCalls==1);
    run("replace");assert(nativeCalls==2&&adapterCalls==2&&fn.counts[0].replaced==1);
    Input input;input.windows.emplace_back(4);input.windows[0][0]=41;
    modeOverride="compare";source=2;Replay(fn,input);
    assert(fn.counts[2].exact==1&&input.windows[0][0]==41);
    persist=true;source=0;recordLimit=1000;
    assert(Record(fn,input));const auto corpus=LoadCorpus(fn);
    assert(!corpus.empty()&&corpus.back().canonical&&corpus.back().windows[0][0]==41);
    // Unsampled outer calls must release the lock before a sampled inner call.
    functions[1]=fn;functions[1].counts={};functions[1].name="authored-inner";
    fn.counts[0].calls=50;*value=41;enterNested=true;SaveFx(machine.before);
    Dispatch(&frame.regs,&machine);
    assert(*value==43&&functions[1].counts[0].exact==1);
    VirtualFree(value,0,MEM_RELEASE);
}
