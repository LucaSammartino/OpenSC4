// SPDX-License-Identifier: GPL-3.0-or-later
// S3: opt-in observational entry detours. Contracts: view-utilities.md §§1,12,13.
#include "FC4Inplace.h"
#include "FC4InplaceDetour.h"
#include "FC4InplaceMachine.h"
#include "FC4CompareData.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4ConfigRulesTrace.h"
#include <windows.h>
#include <tlhelp32.h>
#include <intrin.h>
#include <array>
#include <algorithm>
#include <string>
#include <vector>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <cmath>

extern "C" bool fc4_compare_adapter(const char*,const uint32_t*,fc4::compare::Buffer*,size_t,fc4::compare::Result*);
extern "C" bool fc4_compare_fixture(const char*,unsigned,uint32_t*,fc4::compare::Buffer*,size_t);

namespace {
#include "FC4CompareState.h"
bool Supported() {
    imageBase = reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr));
    if (imageBase != 0x400000) return false;
    const auto* dos = reinterpret_cast<const IMAGE_DOS_HEADER*>(imageBase);
    if (dos->e_magic != IMAGE_DOS_SIGNATURE || dos->e_lfanew <= 0 || dos->e_lfanew > 0x1000) return false;
    const auto* nt = reinterpret_cast<const IMAGE_NT_HEADERS32*>(imageBase + dos->e_lfanew);
    return nt->Signature == IMAGE_NT_SIGNATURE && nt->FileHeader.Machine == 0x14c &&
        nt->FileHeader.TimeDateStamp == 0x4c12bfb7 && nt->OptionalHeader.Magic == 0x10b &&
        nt->OptionalHeader.ImageBase == 0x400000 && nt->OptionalHeader.SizeOfImage == 0x7c8000;
}

// Installation only: suspend other process threads before changing executable
// storage. No heap allocation/logging while suspended; reject an occupied entry.
struct PausedThreads {
    HANDLE handles[256]{}; unsigned count = 0; bool ok = true;
    PausedThreads(uintptr_t entry, unsigned bytes) {
        const auto snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if (snapshot == INVALID_HANDLE_VALUE) { ok = false; return; }
        THREADENTRY32 thread{}; thread.dwSize = sizeof(thread);
        BOOL next = Thread32First(snapshot, &thread);
        while (next) {
            if (thread.th32OwnerProcessID == GetCurrentProcessId() && thread.th32ThreadID != GetCurrentThreadId()) {
                if (count == 256) { ok = false; break; }
                HANDLE handle = OpenThread(THREAD_SUSPEND_RESUME | THREAD_GET_CONTEXT, FALSE, thread.th32ThreadID);
                if (!handle) { ok = false; break; }
                if (SuspendThread(handle) == DWORD(-1)) { CloseHandle(handle); ok = false; break; }
                handles[count++] = handle;
                CONTEXT context{}; context.ContextFlags = CONTEXT_CONTROL;
                if (!GetThreadContext(handle, &context) ||
                    (context.Eip >= entry && context.Eip < entry + bytes)) { ok = false; break; }
            }
            next = Thread32Next(snapshot, &thread);
        }
        CloseHandle(snapshot);
    }
    ~PausedThreads() { for (unsigned i = count; i; --i) { ResumeThread(handles[i-1]); CloseHandle(handles[i-1]); } }
};
bool Install(Function& fn, void* forwarder) {
    auto* entry = reinterpret_cast<uint8_t*>(fn.va);
    MEMORY_BASIC_INFORMATION memory{};
    if (!VirtualQuery(entry, &memory, sizeof(memory)) || memory.Type != MEM_IMAGE ||
        (memory.Protect & 0xff) != PAGE_EXECUTE_READ ||
        reinterpret_cast<uintptr_t>(memory.BaseAddress) + memory.RegionSize - fn.va < 24) return false;
    auto* trampoline = static_cast<uint8_t*>(VirtualAlloc(nullptr, 64, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE));
    if (!trampoline) return false;
    const auto bytes = fc4::inplace::Trampoline(entry, 24, fn.va, trampoline, reinterpret_cast<uintptr_t>(trampoline));
    DWORD previous = 0;
    if (!bytes || !VirtualProtect(trampoline, 64, PAGE_EXECUTE_READ, &previous)) {
        VirtualFree(trampoline, 0, MEM_RELEASE); return false;
    }
    FlushInstructionCache(GetCurrentProcess(), trampoline, 64);
    bool installed = false;
    {
        PausedThreads threads(fn.va, bytes);
        if (threads.ok && VirtualProtect(entry, 5, PAGE_EXECUTE_READWRITE, &previous)) {
            fn.original = trampoline;
            fc4::inplace::Jump(entry, fn.va, reinterpret_cast<uintptr_t>(forwarder));
            DWORD ignored = 0; VirtualProtect(entry, 5, previous, &ignored);
            FlushInstructionCache(GetCurrentProcess(), entry, 5);
            installed = true;
        }
    }
    if (!installed) VirtualFree(trampoline, 0, MEM_RELEASE);
    return installed;
}

struct Window { uint8_t* pointer; size_t size; int output; bool floating; };
struct Region { uint8_t* pointer = nullptr; size_t size = 0; uint8_t* before = nullptr; uint8_t* after = nullptr; };
struct Capture {
    std::array<Window, 16> windows{}; unsigned windowCount = 0;
    std::array<Region, 16> regions{}; unsigned regionCount = 0;
    size_t total = 0; bool ok = true; const char* reason = "invalid-window";
    bool Refuse(const char* why) { reason=why; return false; }
    ~Capture() { for (auto& r : regions) if (r.before) VirtualFree(r.before, 0, MEM_RELEASE); }
    void Add(void* pointer, size_t size, int output = -1, bool floating = true) {
        if (!pointer || windowCount == windows.size()) { ok = false; return; }
        windows[windowCount++] = {static_cast<uint8_t*>(pointer), size, output, floating};
    }
    bool Prepare(uintptr_t stackFloor) {
        if (!ok) return false;
        // SPEC-GAP: output aliases have no replay contract; refuse them.
        for (unsigned i=0; i<windowCount; ++i) for (unsigned j=0; j<i; ++j) {
            const auto& a=windows[i]; const auto& b=windows[j];
            if ((a.output >= 0 || b.output >= 0) &&
                a.pointer < b.pointer+b.size && b.pointer < a.pointer+a.size) return Refuse("output-alias");
        }
        for (unsigned i=0; i<windowCount; ++i) {
            auto& w=windows[i]; MEMORY_BASIC_INFORMATION m{};
            if (!VirtualQuery(w.pointer,&m,sizeof(m)) || m.State != MEM_COMMIT || m.Type != MEM_PRIVATE ||
                (m.Protect != PAGE_READWRITE && m.Protect != PAGE_WRITECOPY) ||
                reinterpret_cast<uintptr_t>(w.pointer)+w.size > reinterpret_cast<uintptr_t>(m.BaseAddress)+m.RegionSize) return Refuse("not-private-writable-region");
            auto* begin=static_cast<uint8_t*>(m.BaseAddress);
            auto* end=begin+m.RegionSize;
            // All memory below the native caller's arguments is active observer/original
            // stack, not persistent call output. TLS identifies this thread's stack.
            const auto* tib = reinterpret_cast<const NT_TIB*>(NtCurrentTeb());
            if (begin >= tib->StackLimit && begin < tib->StackBase) begin = reinterpret_cast<uint8_t*>(stackFloor);
            if (w.pointer < begin || w.pointer+w.size > end) return Refuse("active-stack-or-cross-region");
            bool found=false;
            for (unsigned j=0; j<regionCount; ++j) if (regions[j].pointer==begin) found=true;
            if (found) continue;
            const auto size=static_cast<size_t>(end-begin);
            if (size > 64u*1024*1024 || total+size > 64u*1024*1024) return Refuse("region-cap");
            total += size;
            auto* storage=static_cast<uint8_t*>(VirtualAlloc(nullptr,size*2,MEM_RESERVE|MEM_COMMIT,PAGE_READWRITE));
            if (!storage) return Refuse("snapshot-allocation");
            regions[regionCount++]={begin,size,storage,storage+size};
        }
        return true;
    }
    void Snapshot(bool after) { for (unsigned i=0; i<regionCount; ++i) {
        auto& r=regions[i]; std::memcpy(after?r.after:r.before,r.pointer,r.size);
    } }
    const uint8_t* Saved(const Window& w, bool after) const {
        for (unsigned i=0; i<regionCount; ++i) { const auto& r=regions[i];
            if (w.pointer>=r.pointer && w.pointer+w.size<=r.pointer+r.size)
                return (after?r.after:r.before)+(w.pointer-r.pointer);
        }
        return nullptr;
    }
    void Restore(bool after) { for (unsigned i=0; i<windowCount; ++i) {
        const auto& w=windows[i];
        // Never write back read-only receiver/input windows: a concurrent camera
        // update must not be overwritten by the observer.
        if (w.output>=0) std::memcpy(w.pointer,Saved(w,after),w.size);
    } }
    bool Outside(unsigned& region, size_t& offset) const {
        for (unsigned j=0; j<regionCount; ++j) { const auto& r=regions[j];
            // Fast equal-region path avoids per-byte work for read-only functions.
            if (std::memcmp(r.before,r.after,r.size)==0) continue;
            for (size_t n=0; n<r.size; ++n) if (r.before[n]!=r.after[n]) {
                bool captured=false;
                for (unsigned k=0; k<windowCount; ++k) {
                    const auto& w=windows[k];
                    if (r.pointer+n>=w.pointer && r.pointer+n<w.pointer+w.size) captured=true;
                }
                if (!captured) { region=j; offset=n; return true; }
            }
        }
        return false;
    }
};

#include "FC4CompareRuntime.h"
