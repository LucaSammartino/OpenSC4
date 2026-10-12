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

namespace fc4 {
namespace {

uintptr_t g_base = 0;
uintptr_t g_size = 0;
void* g_vehHandle = nullptr;
volatile LONG g_vehLogged = 0;
constexpr LONG kMaxVehLogs = 200;

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

LONG CALLBACK VectoredHandler(EXCEPTION_POINTERS* ep) {
    if (ep == nullptr || ep->ExceptionRecord == nullptr) return EXCEPTION_CONTINUE_SEARCH;
    const uintptr_t addr = reinterpret_cast<uintptr_t>(ep->ExceptionRecord->ExceptionAddress);
    // 0x406D1388 = thread naming; 0x40010006/0x4001000A = OutputDebugString.
    const DWORD code = ep->ExceptionRecord->ExceptionCode;
    if (InImage(addr) && code != 0x406D1388 && code != 0x40010006 && code != 0x4001000A) {
        if (InterlockedIncrement(&g_vehLogged) <= kMaxVehLogs) {
            LogException("first-chance exception in FreeCity4.dll", ep->ExceptionRecord, ep->ContextRecord);
        }
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
