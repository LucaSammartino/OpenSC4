// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>

namespace fc4::inplace {
// PUSHAD/PUSHFD storage followed by the entry wrapper's return, index and caller.
// The saved ESP word is ignored by POPAD. No addresses in this frame are logged.
struct Registers {
    uint32_t edi, esi, ebp, esp, ebx, edx, ecx, eax, flags;
    uint32_t wrapperReturn, index, callerReturn;
};
struct alignas(16) MachineState { uint8_t before[512], after[512]; };
static_assert(sizeof(Registers) == 48);
}

extern "C" void fc4_inplace_dispatch(fc4::inplace::Registers*, fc4::inplace::MachineState*);

// Private x86 bridge. In addition to the published ABI, preserve volatile native
// results: optimized callers inside the image may retain them across leaf calls.
// Observer calls never become part of the native register/FPU result.
static __declspec(naked) void fc4_inplace_enter() {
    __asm {
        pushfd
        pushad
        cld
        mov ebp, esp
        sub esp, 1040
        and esp, -16
        fxsave [esp]
        push esp
        push ebp
        call fc4_inplace_dispatch
        add esp, 8
        fxrstor [esp + 512]
        mov esp, ebp
        popad
        popfd
        ret
    }
}

// A fresh native argument frame avoids exposing observer frames as arguments.
// Restore the incoming registers, flags and FPU before entering the trampoline;
// capture ALL native result state before any observer instruction can change it.
static __declspec(naked) void fc4_inplace_invoke(
    fc4::inplace::Registers*, fc4::inplace::MachineState*, void*,
    const uint32_t*, unsigned, bool) {
    __asm {
        pushfd
        pushad
        mov ebp, esp
        sub esp, 8
        mov eax, [ebp + 40]
        mov [esp], eax
        mov eax, [ebp + 44]
        mov [esp + 4], eax
        mov esi, [ebp + 52]
        mov ecx, [ebp + 56]
        test ecx, ecx
        jz arguments_done
    arguments_loop:
        sub ecx, 4
        push [esi + ecx]
        jnz arguments_loop
    arguments_done:
        mov eax, offset returned
        cmp byte ptr [ebp + 60], 0
        jne return_selected
        cmp dword ptr [ebp + 56], 4
        je select4
        cmp dword ptr [ebp + 56], 8
        je select8
        cmp dword ptr [ebp + 56], 12
        je select12
        cmp dword ptr [ebp + 56], 16
        je select16
        jmp return_selected
    select4:
        mov eax, offset returned4
        jmp return_selected
    select8:
        mov eax, offset returned8
        jmp return_selected
    select12:
        mov eax, offset returned12
        jmp return_selected
    select16:
        mov eax, offset returned16
    return_selected:
        push eax
        push [ebp + 48]
        mov eax, [ebp + 44]
        fxrstor [eax]
        mov eax, [ebp + 40]
        push [eax + 32]
        popfd
        mov edi, [eax]
        mov esi, [eax + 4]
        mov ebp, [eax + 8]
        mov ebx, [eax + 16]
        mov edx, [eax + 20]
        mov ecx, [eax + 24]
        mov eax, [eax + 28]
        ret
    returned4:
        lea esp, [esp + 4]
        jmp returned
    returned8:
        lea esp, [esp + 8]
        jmp returned
    returned12:
        lea esp, [esp + 12]
        jmp returned
    returned16:
        lea esp, [esp + 16]
    returned:
        pushfd
        pushad
        mov esi, esp
        mov edi, [esp + 36]
        mov ecx, 9
        cld
        rep movsd
        mov eax, [esp + 40]
        fxsave [eax + 512]
        lea esp, [esp + 44]
        popad
        popfd
        ret
    }
}

#define FC4_INPLACE_ENTRY(NAME, INDEX, BYTES) \
    __declspec(naked) void NAME() { \
        __asm push INDEX \
        __asm call fc4_inplace_enter \
        __asm lea esp, [esp + 4] \
        __asm ret BYTES \
    }
