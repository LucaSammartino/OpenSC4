// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "FC4InplaceDetour.h"
#include <windows.h>
#include <tlhelp32.h>

namespace fc4 {
// Process-lifetime forwarding observer. Uses the existing x86 relocation
// utility, fails closed, and never persists or logs instruction storage.
class NativeBoundary {
    struct Paused {
        HANDLE handles[256]{};
        unsigned count = 0;
        bool ok = true;
        Paused(uintptr_t entry, unsigned bytes) {
            HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if (snapshot == INVALID_HANDLE_VALUE) { ok = false; return; }
            THREADENTRY32 thread{};
            thread.dwSize = sizeof(thread);
            for (BOOL next = Thread32First(snapshot, &thread); next; next = Thread32Next(snapshot, &thread)) {
                if (thread.th32OwnerProcessID != GetCurrentProcessId() || thread.th32ThreadID == GetCurrentThreadId()) continue;
                if (count == 256) { ok = false; break; }
                HANDLE handle = OpenThread(THREAD_SUSPEND_RESUME | THREAD_GET_CONTEXT, FALSE, thread.th32ThreadID);
                if (!handle) {
                    // The snapshot can contain a thread that has since exited.
                    if (GetLastError() == ERROR_INVALID_PARAMETER) continue;
                    ok = false; break;
                }
                if (SuspendThread(handle) == DWORD(-1)) { CloseHandle(handle); ok = false; break; }
                handles[count++] = handle;
                CONTEXT context{};
                context.ContextFlags = CONTEXT_CONTROL;
                if (!GetThreadContext(handle, &context) || (context.Eip >= entry && context.Eip < entry + bytes)) {
                    ok = false; break;
                }
            }
            CloseHandle(snapshot);
        }
        ~Paused() { while (count) { HANDLE h = handles[--count]; ResumeThread(h); CloseHandle(h); } }
    };
public:
    void* original = nullptr;
    const char* failure = "none";
    bool Install(uintptr_t address, void* forwarder) {
        if (original) return true;
        auto* entry = reinterpret_cast<uint8_t*>(address);
        MEMORY_BASIC_INFORMATION memory{};
        failure = "image protection";
        if (!VirtualQuery(entry, &memory, sizeof(memory)) || memory.Type != MEM_IMAGE ||
            (memory.Protect & 0xff) != PAGE_EXECUTE_READ ||
            reinterpret_cast<uintptr_t>(memory.BaseAddress) + memory.RegionSize < address + 24) return false;
        auto* trampoline = static_cast<uint8_t*>(VirtualAlloc(nullptr, 64, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE));
        failure = "allocation";
        if (!trampoline) return false;
        const unsigned bytes = inplace::Trampoline(entry, 24, address, trampoline, reinterpret_cast<uintptr_t>(trampoline));
        DWORD old = 0;
        failure = bytes ? "trampoline protection" : "unsupported prologue";
        if (!bytes || !VirtualProtect(trampoline, 64, PAGE_EXECUTE_READ, &old)) {
            VirtualFree(trampoline, 0, MEM_RELEASE); return false;
        }
        FlushInstructionCache(GetCurrentProcess(), trampoline, 64);
        bool installed = false;
        {
            Paused paused(address, bytes);
            failure = paused.ok ? "entry protection" : "thread boundary";
            if (paused.ok && VirtualProtect(entry, 5, PAGE_EXECUTE_READWRITE, &old)) {
                original = trampoline;
                inplace::Jump(entry, address, reinterpret_cast<uintptr_t>(forwarder));
                DWORD ignored = 0;
                VirtualProtect(entry, 5, old, &ignored);
                FlushInstructionCache(GetCurrentProcess(), entry, 5);
                installed = true;
                failure = "none";
            }
        }
        if (!installed) VirtualFree(trampoline, 0, MEM_RELEASE);
        return installed;
    }
};
}
