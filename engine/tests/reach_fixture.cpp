// SPDX-License-Identifier: GPL-3.0-or-later
#include <windows.h>

// A real, independently loaded DLL. Body counters are independent of the VEH.
extern "C" {
__declspec(dllexport) volatile LONG reach_fixture_counts[4] = {};

__declspec(dllexport) __declspec(naked) int __cdecl ReachFixtureA(int) {
    __asm {
        nop
        lock inc dword ptr [reach_fixture_counts]
        mov eax, [esp + 4]
        add eax, 7
        ret
    }
}
__declspec(dllexport) __declspec(naked) int __cdecl ReachFixtureB(int) {
    __asm {
        push ebp
        mov ebp, esp
        lock inc dword ptr [reach_fixture_counts + 4]
        mov eax, [ebp + 8]
        imul eax, 3
        pop ebp
        ret
    }
}
__declspec(dllexport) __declspec(naked) float __cdecl ReachFixtureFloat(float) {
    __asm {
        fld dword ptr [esp + 4]
        lock inc dword ptr [reach_fixture_counts + 8]
        ret
    }
}
__declspec(dllexport) __declspec(naked) unsigned __cdecl ReachFixtureFlags() {
    __asm {
        mov eax, 0
        adc eax, 0
        lock inc dword ptr [reach_fixture_counts + 12]
        ret
    }
}
__declspec(dllexport) __declspec(naked) void __cdecl ReachFixtureRet() { __asm ret }
__declspec(dllexport) __declspec(naked) void __cdecl ReachFixtureJump() { __asm jmp ReachFixtureRet }
__declspec(dllexport) __declspec(naked) int __cdecl ReachFixtureHook(int) {
    __asm {
        mov eax, [esp + 4]
        ret
    }
}
__declspec(dllexport) __declspec(naked) int __fastcall ReachFixtureFault(const int*) {
    __asm {
        mov eax, [ecx]
        ret
    }
}
__declspec(dllexport) __declspec(naked) unsigned __cdecl ReachFixturePushFlags() {
    __asm {
        pushfd
        pop eax
        ret
    }
}
}
