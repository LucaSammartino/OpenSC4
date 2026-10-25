// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4InplaceDetour.h"
#include "FC4InplaceMachine.h"
#include <intrin.h>
#include <windows.h>
#include <crtdbg.h>
#include <cstdio>

namespace {
unsigned testBytes=4;
bool testCallee=true;
void* testNative=nullptr;
unsigned dispatches=0;
bool nestedCase=false;
LONG WINAPI Crash(EXCEPTION_POINTERS* e) {
    std::fprintf(stderr,"machine test exception code=%08lx test_rva=%08lx\n",e->ExceptionRecord->ExceptionCode,
        e->ContextRecord->Eip-static_cast<uint32_t>(reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr))));
    std::fflush(stderr);return EXCEPTION_EXECUTE_HANDLER;
}
// Authored test code: leave meaningful volatile register, flags and x87 results.
#define TEST_NATIVE(NAME, POP) \
    __declspec(naked) void NAME() { \
        __asm mov eax, [esp + 4] \
        __asm add [eax], ecx \
        __asm mov eax, 11223344h \
        __asm mov ecx, 55667788h \
        __asm mov edx, 99aabbcch \
        __asm fld1 \
        __asm stc \
        __asm ret POP \
    }
TEST_NATIVE(Native0,0)
TEST_NATIVE(Native4,4)
TEST_NATIVE(Native8,8)
TEST_NATIVE(Native12,12)
TEST_NATIVE(Native16,16)
FC4_INPLACE_ENTRY(Hook0,0,0)
FC4_INPLACE_ENTRY(Hook4,0,4)
FC4_INPLACE_ENTRY(Hook8,0,8)
FC4_INPLACE_ENTRY(Hook12,0,12)
FC4_INPLACE_ENTRY(Hook16,0,16)
FC4_INPLACE_ENTRY(NestedHook,1,4)
__declspec(naked) void NativeNested() {
    __asm push [esp + 4]
    __asm call Hook4
    __asm ret 4
}

struct Result {
    uint32_t eax=0, ecx=0, edx=0, flags=0, output=0;
    alignas(16) uint8_t fx[512]{};
};
Result Run(void* target, unsigned bytes, bool callee) {
    Result r;
    auto* output=&r.output;
    auto* result=&r;
    const unsigned cleanup=callee?0:bytes;
    __asm {
        fninit
        mov ecx, bytes
        sub ecx, 4
        jz last_argument
    next_argument:
        push 0
        sub ecx, 4
        jnz next_argument
    last_argument:
        push output
        mov eax, 11
        mov ecx, 7
        mov edx, 9
        clc
        call target
        pushfd
        push eax
        mov eax, result
        pop dword ptr [eax]
        mov [eax + 4], ecx
        mov [eax + 8], edx
        pop dword ptr [eax + 12]
        add esp, cleanup
    }
    _fxsave(r.fx);
    return r;
}
bool MachineTests() {
    alignas(16) uint8_t previous[512];_fxsave(previous);
    void* native[]={reinterpret_cast<void*>(Native4),reinterpret_cast<void*>(Native8),
        reinterpret_cast<void*>(Native12),reinterpret_cast<void*>(Native16),reinterpret_cast<void*>(Native0),reinterpret_cast<void*>(Native0)};
    void* hooks[]={reinterpret_cast<void*>(Hook4),reinterpret_cast<void*>(Hook8),
        reinterpret_cast<void*>(Hook12),reinterpret_cast<void*>(Hook16),reinterpret_cast<void*>(Hook0),reinterpret_cast<void*>(Hook0)};
    const unsigned sizes[]={4,8,12,16,8,12};
    bool exact=true;
    for (unsigned i=0;i<6;++i) {
        testNative=native[i];testBytes=sizes[i];testCallee=i<4;
        const auto a=Run(testNative,testBytes,testCallee);
        const auto b=Run(hooks[i],testBytes,testCallee);
        // Ignore architectural instruction/data pointers and reserved FX bytes.
        exact &= a.eax==b.eax && a.ecx==b.ecx && a.edx==b.edx && a.output==7 && b.output==7;
        exact &= a.eax==0x11223344 && a.ecx==0x55667788 && a.edx==0x99aabbcc && (a.flags&1)!=0;
        exact &= ((a.flags^b.flags)&0xcd5)==0;
        exact &= std::memcmp(a.fx,b.fx,8)==0 && std::memcmp(a.fx+24,b.fx+24,8)==0;
        exact &= std::memcmp(a.fx+32,b.fx+32,128)==0;
    }
    nestedCase=true;testNative=reinterpret_cast<void*>(NativeNested);testBytes=4;testCallee=true;
    const auto nested=Run(reinterpret_cast<void*>(NestedHook),4,true);
    std::printf("nested results eax=%08x ecx=%08x edx=%08x output=%u carry=%u\n",
        nested.eax,nested.ecx,nested.edx,nested.output,nested.flags&1);
    exact &= nested.eax==0x11223344 && nested.ecx==0x55667788 && nested.edx==0x99aabbcc;
    exact &= nested.output==7 && (nested.flags&1)!=0;
    _fxrstor(previous);
    std::printf("inplace machine bridge: cases=7 dispatches=%u exact=%u\n",dispatches,exact?1:0);
    return exact && dispatches==8;
}
}

extern "C" void fc4_inplace_dispatch(fc4::inplace::Registers* r, fc4::inplace::MachineState* m) {
    ++dispatches;
    __asm fninit
    const auto native=nestedCase && r->index==0?reinterpret_cast<void*>(Native4):testNative;
    fc4_inplace_invoke(r,m,native,reinterpret_cast<uint32_t*>(r+1),testBytes,testCallee);
    // Deliberately destroy observer FPU and volatile state after native execution.
    __asm fninit
    __asm xor eax, eax
    __asm xor ecx, ecx
    __asm xor edx, edx
}

int main() {
    SetErrorMode(SEM_FAILCRITICALERRORS|SEM_NOGPFAULTERRORBOX);
    SetUnhandledExceptionFilter(Crash);
    _CrtSetReportMode(_CRT_ERROR,_CRTDBG_MODE_FILE);
    _CrtSetReportFile(_CRT_ERROR,_CRTDBG_FILE_STDERR);
    _CrtSetReportMode(_CRT_ASSERT,_CRTDBG_MODE_FILE);
    _CrtSetReportFile(_CRT_ASSERT,_CRTDBG_FILE_STDERR);
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
    const uint8_t decrement[]={0x8b,0x44,0x24,0x08,0x48};
    check(Trampoline(decrement,sizeof(decrement),0x1000,out,0x2000)==5);
    for (size_t i = 0; i < sizeof(standard) - 1; ++i)
        check(Trampoline(standard, i, 0x1000, out, 0x2000) == 0);
    unsigned selected = 0;
    for (uint64_t i = 1; i <= 1000; ++i) selected += Sample(i);
    check(selected == 97 && Sample(50) && !Sample(51) && Sample(70));
    check(NearFloat(0x3f800000, 0x3f800001));
    check(!NearFloat(0x3f800000, 0x40000000));
    check(!NearFloat(0x7f800000, 0x7f800000));
    check(MachineTests());
    std::printf("inplace mechanics: failures=%d\n", failures);
    return failures ? 1 : 0;
}
