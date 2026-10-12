// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

#include "FC4Diag.h"

#include "FC4Log.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdio>
#include <cstring>

namespace fc4 {
namespace {

uintptr_t g_base = 0;
uintptr_t g_size = 0;
void* g_vehHandle = nullptr;
volatile LONG g_vehLogged = 0;
constexpr LONG kMaxVehLogs = 200;
volatile LONG g_foreignLogged = 0;
constexpr LONG kMaxForeignLogs = 8;

// Ring of the last interface-method entries (single simulation thread).
constexpr unsigned kRing = 64;
const char* g_ring[kRing] = {};
DWORD g_ringTick[kRing] = {};
unsigned g_ringNext = 0;
unsigned long long g_ringTotal = 0;

bool InImage(uintptr_t addr) { return g_base != 0 && addr >= g_base && addr - g_base < g_size; }

const char* CodeName(DWORD code) {
    switch (code) {
        case EXCEPTION_ACCESS_VIOLATION: return "ACCESS_VIOLATION";
        case EXCEPTION_ILLEGAL_INSTRUCTION: return "ILLEGAL_INSTRUCTION";
        case EXCEPTION_INT_DIVIDE_BY_ZERO: return "INT_DIVIDE_BY_ZERO";
        case EXCEPTION_STACK_OVERFLOW: return "STACK_OVERFLOW";
        case EXCEPTION_PRIV_INSTRUCTION: return "PRIV_INSTRUCTION";
        case EXCEPTION_BREAKPOINT: return "BREAKPOINT";
        case 0xE06D7363: return "C++ exception";
        case 0xC0000409: return "STACK_BUFFER_OVERRUN / fail-fast";
        default: return "other";
    }
}

void LogException(const char* prefix, const EXCEPTION_RECORD* rec, const CONTEXT* ctx) {
    const uintptr_t addr = reinterpret_cast<uintptr_t>(rec->ExceptionAddress);
    const bool ours = InImage(addr);
    char detail[96] = "";
    if (rec->ExceptionCode == EXCEPTION_ACCESS_VIOLATION && rec->NumberParameters >= 2) {
        const ULONG_PTR kind = rec->ExceptionInformation[0];
        _snprintf_s(detail, sizeof(detail), _TRUNCATE, " (%s address 0x%08lX)",
                    kind == 0 ? "reading" : (kind == 1 ? "writing" : "executing"),
                    static_cast<unsigned long>(rec->ExceptionInformation[1]));
    }
    unsigned long eip = 0, esp = 0;
#if defined(_M_IX86)
    if (ctx != nullptr) {
        eip = ctx->Eip;
        esp = ctx->Esp;
    }
#else
    (void)ctx;
#endif
    Log("%s: code 0x%08lX %s%s at 0x%08lX (%s FreeCity4.dll base 0x%08lX%s0x%lX) eip=0x%08lX esp=0x%08lX", prefix,
        static_cast<unsigned long>(rec->ExceptionCode), CodeName(rec->ExceptionCode), detail,
        static_cast<unsigned long>(addr), ours ? "inside" : "OUTSIDE", static_cast<unsigned long>(g_base),
        ours ? " + " : ", image size ", static_cast<unsigned long>(ours ? addr - g_base : g_size), eip, esp);
}

void ModuleOf(uintptr_t addr, char* name, size_t cap, uintptr_t* base) {
    name[0] = 0;
    *base = 0;
    HMODULE m = nullptr;
    if (GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                           reinterpret_cast<LPCSTR>(addr), &m) &&
        m != nullptr) {
        *base = reinterpret_cast<uintptr_t>(m);
        char path[MAX_PATH] = {};
        GetModuleFileNameA(m, path, MAX_PATH);
        const char* slash = strrchr(path, '\\');
        strncpy_s(name, cap, slash ? slash + 1 : path, _TRUNCATE);
    } else {
        strncpy_s(name, cap, "(no module)", _TRUNCATE);
    }
}

void LogForeign(EXCEPTION_POINTERS* ep) {
    char mod[64];
    uintptr_t base = 0;
    const uintptr_t addr = reinterpret_cast<uintptr_t>(ep->ExceptionRecord->ExceptionAddress);
    ModuleOf(addr, mod, sizeof(mod), &base);
    char prefix[160];
    _snprintf_s(prefix, sizeof(prefix), _TRUNCATE, "first-chance exception OUTSIDE FreeCity4.dll in %s+0x%lX", mod,
                static_cast<unsigned long>(addr - base));
    LogException(prefix, ep->ExceptionRecord, ep->ContextRecord);
#if defined(_M_IX86)
    // Return addresses into our image on the faulting stack (who called the game).
    if (ep->ContextRecord != nullptr) {
        const uintptr_t* sp = reinterpret_cast<const uintptr_t*>(ep->ContextRecord->Esp);
        int found = 0;
        for (int i = 0; i < 256 && found < 8; ++i) {
            if (IsBadReadPtr(sp + i, sizeof(uintptr_t))) break;
            if (InImage(sp[i])) {
                Log("  stack[esp+0x%03X] = FreeCity4.dll+0x%lX", i * 4, static_cast<unsigned long>(sp[i] - g_base));
                ++found;
            }
        }
        if (found == 0) Log("  no return address into FreeCity4.dll in the top 1 KiB of the stack");
    }
#endif
    const unsigned n = g_ringTotal < kRing ? static_cast<unsigned>(g_ringTotal) : kRing;
    Log("  last %u of %llu FreeCity4 interface calls (oldest first):", n, g_ringTotal);
    for (unsigned k = 0; k < n; ++k) {
        const unsigned i = (g_ringNext + kRing - n + k) % kRing;
        Log("    [%lu ms] %s", static_cast<unsigned long>(g_ringTick[i]), g_ring[i] ? g_ring[i] : "?");
    }
}

LONG CALLBACK VectoredHandler(EXCEPTION_POINTERS* ep) {
    if (ep == nullptr || ep->ExceptionRecord == nullptr) return EXCEPTION_CONTINUE_SEARCH;
    const uintptr_t addr = reinterpret_cast<uintptr_t>(ep->ExceptionRecord->ExceptionAddress);
    // 0x406D1388 = thread naming; 0x40010006/0x4001000A = OutputDebugString.
    const DWORD code = ep->ExceptionRecord->ExceptionCode;
    if (InImage(addr) && code != 0x406D1388 && code != 0x40010006 && code != 0x4001000A) {
        if (InterlockedIncrement(&g_vehLogged) <= kMaxVehLogs) {
            LogException("first-chance exception in FreeCity4.dll", ep->ExceptionRecord, ep->ContextRecord);
        }
    } else if (!InImage(addr) && (code == EXCEPTION_ACCESS_VIOLATION || code == EXCEPTION_ILLEGAL_INSTRUCTION ||
                                  code == EXCEPTION_PRIV_INSTRUCTION || code == EXCEPTION_STACK_OVERFLOW ||
                                  code == EXCEPTION_INT_DIVIDE_BY_ZERO || code == 0xC0000409)) {
        if (InterlockedIncrement(&g_foreignLogged) <= kMaxForeignLogs) LogForeign(ep);
    }
    return EXCEPTION_CONTINUE_SEARCH;  // never swallow
}

struct Captured {
    EXCEPTION_RECORD record;
    CONTEXT context;
    bool valid;
};

int Filter(EXCEPTION_POINTERS* ep, Captured* out) {
    if (ep != nullptr && ep->ExceptionRecord != nullptr) {
        out->record = *ep->ExceptionRecord;
        if (ep->ContextRecord != nullptr) out->context = *ep->ContextRecord;
        out->valid = true;
    }
    return EXCEPTION_EXECUTE_HANDLER;
}

// No C++ objects with destructors in this function (required for __try).
bool RunGuarded(void (*fn)(void*), void* ctx, Captured* captured) {
    __try {
        fn(ctx);
        return true;
    } __except (Filter(GetExceptionInformation(), captured)) {
        return false;
    }
}

}  // namespace

void DiagAttach(void* moduleBase) {
    g_base = reinterpret_cast<uintptr_t>(moduleBase);
    const auto* dos = static_cast<const IMAGE_DOS_HEADER*>(moduleBase);
    if (dos != nullptr && dos->e_magic == IMAGE_DOS_SIGNATURE) {
        const auto* nt = reinterpret_cast<const IMAGE_NT_HEADERS*>(static_cast<const char*>(moduleBase) + dos->e_lfanew);
        if (nt->Signature == IMAGE_NT_SIGNATURE) g_size = nt->OptionalHeader.SizeOfImage;
    }
    g_vehHandle = AddVectoredExceptionHandler(1, &VectoredHandler);
    Log("diagnostics: image 0x%08lX..0x%08lX, vectored handler %s", static_cast<unsigned long>(g_base),
        static_cast<unsigned long>(g_base + g_size), g_vehHandle ? "installed" : "NOT installed");
}

void DiagDetach() {
    if (g_vehHandle != nullptr) {
        RemoveVectoredExceptionHandler(g_vehHandle);
        g_vehHandle = nullptr;
    }
}

void CallTrace(const char* what, unsigned* counter) {
    g_ring[g_ringNext] = what;
    g_ringTick[g_ringNext] = GetTickCount();
    g_ringNext = (g_ringNext + 1) % kRing;
    ++g_ringTotal;
    if (counter != nullptr && *counter < 20) {
        ++*counter;
        if (LogLevel() >= kLogVerbose) LogVerboseImpl("call %s (#%u)", what, *counter);
    }
}

uintptr_t ImageBase() { return g_base; }
uintptr_t ImageSize() { return g_size; }

bool SehGuard(const char* what, void (*fn)(void* ctx), void* ctx) {
    Captured captured{};
    if (RunGuarded(fn, ctx, &captured)) return true;
    char prefix[160];
    _snprintf_s(prefix, sizeof(prefix), _TRUNCATE, "STRUCTURED EXCEPTION caught in %s", what);
    if (captured.valid) {
        LogException(prefix, &captured.record, &captured.context);
    } else {
        Log("%s (no exception record)", prefix);
    }
    return false;
}

}  // namespace fc4
