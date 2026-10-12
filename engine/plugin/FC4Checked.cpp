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

#include "FC4Checked.h"

#include "FC4Diag.h"
#include "FC4Log.h"
#include "FC4Watchdog.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdlib>
#include <string>

#if FC4_CHECKED
#include <dbghelp.h>
#include <rtcapi.h>
#include <cstdarg>
#include <cwchar>
#endif

namespace fc4 {
namespace {

volatile LONG g_rtcErrors = 0;
volatile LONG g_overruns = 0;

void Hex(const uint8_t* p, unsigned n, char* out, size_t cap) {
    size_t len = 0;
    out[0] = 0;
    for (unsigned i = 0; i < n && len + 3 < cap; ++i) {
        const int k = _snprintf_s(out + len, cap - len, _TRUNCATE, "%02X", p[i]);
        if (k < 0) break;
        len += static_cast<size_t>(k);
    }
}

#if FC4_CHECKED

// ---- DbgHelp (loaded on the first RTC failure) -------------------------------------------------

using SymSetOptionsFn = DWORD(WINAPI*)(DWORD);
using SymInitializeFn = BOOL(WINAPI*)(HANDLE, PCSTR, BOOL);
using SymLoadModuleExFn = DWORD64(WINAPI*)(HANDLE, HANDLE, PCSTR, PCSTR, DWORD64, DWORD, PMODLOAD_DATA, DWORD);
using SymFromAddrFn = BOOL(WINAPI*)(HANDLE, DWORD64, PDWORD64, PSYMBOL_INFO);
using SymGetLineFromAddr64Fn = BOOL(WINAPI*)(HANDLE, DWORD64, PDWORD, PIMAGEHLP_LINE64);

struct Dbg {
    bool tried = false;
    bool ok = false;
    HANDLE process = nullptr;
    SymFromAddrFn fromAddr = nullptr;
    SymGetLineFromAddr64Fn line = nullptr;
};
Dbg g_dbg;
volatile LONG g_inDllMain = 0;

bool InitDbgHelp() {
    if (g_dbg.tried) return g_dbg.ok;
    g_dbg.tried = true;
    if (g_inDllMain) return false;  // never LoadLibrary under the loader lock
    HMODULE h = LoadLibraryExW(L"dbghelp.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (h == nullptr) return false;
    auto setOptions = reinterpret_cast<SymSetOptionsFn>(GetProcAddress(h, "SymSetOptions"));
    auto init = reinterpret_cast<SymInitializeFn>(GetProcAddress(h, "SymInitialize"));
    auto load = reinterpret_cast<SymLoadModuleExFn>(GetProcAddress(h, "SymLoadModuleEx"));
    g_dbg.fromAddr = reinterpret_cast<SymFromAddrFn>(GetProcAddress(h, "SymFromAddr"));
    g_dbg.line = reinterpret_cast<SymGetLineFromAddr64Fn>(GetProcAddress(h, "SymGetLineFromAddr64"));
    if (!setOptions || !init || !load || !g_dbg.fromAddr || !g_dbg.line) return false;
    // A private handle, so another DbgHelp user in the process is not disturbed.
    if (!DuplicateHandle(GetCurrentProcess(), GetCurrentProcess(), GetCurrentProcess(), &g_dbg.process, 0, FALSE,
                         DUPLICATE_SAME_ACCESS)) {
        return false;
    }
    setOptions(SYMOPT_UNDNAME | SYMOPT_DEFERRED_LOADS | SYMOPT_LOAD_LINES | SYMOPT_FAIL_CRITICAL_ERRORS);
    std::string dir = ModulePath();
    const size_t slash = dir.find_last_of("\\/");
    if (slash != std::string::npos) dir.erase(slash);
    if (!init(g_dbg.process, dir.c_str(), FALSE)) return false;
    load(g_dbg.process, nullptr, ModulePath(), nullptr, ImageBase(), static_cast<DWORD>(ImageSize()), nullptr, 0);
    g_dbg.ok = true;
    return true;
}

// "Function+0x12 (file.cpp:34)" or "" without symbols.
void Symbolize(uintptr_t addr, char* out, size_t cap) {
    out[0] = 0;
    if (!InitDbgHelp()) return;
    alignas(SYMBOL_INFO) char buf[sizeof(SYMBOL_INFO) + 256];
    SYMBOL_INFO* si = reinterpret_cast<SYMBOL_INFO*>(buf);
    memset(buf, 0, sizeof(buf));
    si->SizeOfStruct = sizeof(SYMBOL_INFO);
    si->MaxNameLen = 255;
    DWORD64 disp = 0;
    if (!g_dbg.fromAddr(g_dbg.process, addr, &disp, si)) return;
    IMAGEHLP_LINE64 line{};
    line.SizeOfStruct = sizeof(line);
    DWORD lineDisp = 0;
    if (g_dbg.line(g_dbg.process, addr, &lineDisp, &line) && line.FileName != nullptr) {
        const char* file = line.FileName;
        for (const char* p = line.FileName; *p; ++p) {
            if (*p == '\\' || *p == '/') file = p + 1;
        }
        _snprintf_s(out, cap, _TRUNCATE, "%s+0x%llX (%s:%lu)", si->Name, static_cast<unsigned long long>(disp), file,
                    static_cast<unsigned long>(line.LineNumber));
    } else {
        _snprintf_s(out, cap, _TRUNCATE, "%s+0x%llX", si->Name, static_cast<unsigned long long>(disp));
    }
}

bool IsRtcRuntime(const char* sym) {
    return strncmp(sym, "_RTC_", 5) == 0 || strncmp(sym, "failwithmessage", 15) == 0 ||
           strstr(sym, "RtcHandler") != nullptr || strncmp(sym, "RtlCaptureStackBackTrace", 24) == 0;
}

// ---- the RTC error handler -------------------------------------------------------------------

const char* RtcKind(int type) {
    switch (type) {
        case 0: return "ESP not preserved across a call (calling convention / argument count / callee-pop mismatch)";
        case 1: return "data lost in a conversion";
        case 2: return "stack corrupted around a local variable";
        case 3: return "uninitialized local variable used";
        case 4: return "stack around an _alloca corrupted";
        default: return "unknown RTC error";
    }
}

constexpr unsigned kSites = 32;
uintptr_t g_sites[kSites] = {};
LONG g_siteReports[kSites] = {};
volatile LONG g_inHandler = 0;
CRITICAL_SECTION g_rtcLock;

int __cdecl RtcHandler(int type, const wchar_t* file, int line, const wchar_t* module, const wchar_t* format, ...) {
    const LONG n = InterlockedIncrement(&g_rtcErrors);
    if (InterlockedExchange(&g_inHandler, 1) != 0) return 0;  // an RTC failure inside the report
    EnterCriticalSection(&g_rtcLock);

    wchar_t wmsg[512] = {};
    va_list ap;
    va_start(ap, format);
    if (format != nullptr) _vsnwprintf_s(wmsg, _countof(wmsg), _TRUNCATE, format, ap);
    va_end(ap);
    char msg[512] = {};
    WideCharToMultiByte(CP_UTF8, 0, wmsg, -1, msg, sizeof(msg) - 1, nullptr, nullptr);
    for (char* p = msg; *p; ++p) {
        if (*p == '\r' || *p == '\n') *p = ' ';
    }

    void* frames[32] = {};
    const USHORT count = RtlCaptureStackBackTrace(0, 32, frames, nullptr);

    // The call site: the first frame in our image outside the RTC runtime.
    char sym[512] = {};
    uintptr_t site = 0;
    char siteSym[512] = {};
    for (USHORT i = 0; i < count; ++i) {
        const uintptr_t a = reinterpret_cast<uintptr_t>(frames[i]);
        if (a < ImageBase() || a - ImageBase() >= ImageSize()) continue;
        Symbolize(a - 1, sym, sizeof(sym));
        if (sym[0] != 0 && IsRtcRuntime(sym)) continue;
        if (sym[0] == 0 && i < 3) continue;  // no symbols: skip the handler's own frames
        site = a;
        strcpy_s(siteSym, sym);
        break;
    }

    // At most 3 reports per call site, 32 sites.
    LONG reports = 0;
    bool known = false;
    for (unsigned i = 0; i < kSites; ++i) {
        if (g_sites[i] == site) {
            reports = ++g_siteReports[i];
            known = true;
            break;
        }
        if (g_sites[i] == 0) {
            g_sites[i] = site;
            reports = g_siteReports[i] = 1;
            known = true;
            break;
        }
    }
    if (known && reports <= 3) {
        char where[96];
        DescribeAddress(site, where, sizeof(where));
        char src[300] = {};
        if (file != nullptr && line > 0) {
            char f[260] = {};
            WideCharToMultiByte(CP_UTF8, 0, file, -1, f, sizeof(f) - 1, nullptr, nullptr);
            _snprintf_s(src, sizeof(src), _TRUNCATE, " [%s:%d]", f, line);
        }
        (void)module;
        (void)type;  // the report level (_CRT_ERROR), not the check
        // The check number is in the message: "Run-Time Check Failure #N - ...".
        const char* hash = strstr(msg, "Failure #");
        const int check = hash != nullptr ? atoi(hash + 9) : -1;
        Log("RTC ERROR #%ld (report %ld for this call site): %s: %s", n, reports, RtcKind(check), msg);
        Log("  call site: %s %s%s", where, siteSym[0] ? siteSym : "(no symbols: copy FreeCity4.pdb next to the DLL)",
            src);
        char activity[512];
        DescribeCurrentActivity(activity, sizeof(activity));
        Log("  activity: %s", activity);
        unsigned shown = 0;
        for (USHORT i = 0; i < count && shown < 12; ++i) {
            const uintptr_t a = reinterpret_cast<uintptr_t>(frames[i]);
            char w[96];
            DescribeAddress(a, w, sizeof(w));
            Symbolize(a - 1, sym, sizeof(sym));
            Log("    frame %2u: %s %s", static_cast<unsigned>(i), w, sym);
            ++shown;
        }
        if (n == 1) DumpCallRing("  ");
    }
    LeaveCriticalSection(&g_rtcLock);
    InterlockedExchange(&g_inHandler, 0);
    return 0;  // continue (no debugger break, no dialog)
}

// ---- self-tests -------------------------------------------------------------------------------

// The real object: its method takes no argument (callee pops nothing).
struct SelfTestNoArg {
    virtual void Method();
};
void SelfTestNoArg::Method() {}

// The wrong declaration of the same slot: one argument the callee is expected to pop.
struct SelfTestOneArg {
    virtual void Method(int) = 0;
};

__declspec(noinline) void ProvokeEspMismatch() {
    SelfTestNoArg real;
    SelfTestOneArg* wrong = reinterpret_cast<SelfTestOneArg*>(&real);
    wrong->Method(42);
}

__declspec(noinline) void WriteFourBytes(void* p) {
    const uint32_t v = 0x11223344;
    memcpy(p, &v, sizeof(v));
}

void ProvokeOutOverrun() {
    FC4_OUT(uint8_t, b, "self-test uint8 out-parameter");
    WriteFourBytes(b.ptr());
    b.Check();
}

#endif  // FC4_CHECKED

}  // namespace

void CheckedAttach() {
#if FC4_CHECKED
    InterlockedExchange(&g_inDllMain, 1);
    InitializeCriticalSection(&g_rtcLock);
    _RTC_SetErrorFuncW(&RtcHandler);
    Log("build: CHECKED (/Od /RTC1; RTC failures and out-parameter overruns are logged)");
#else
    Log("build: normal (configure with -DFC4_CHECKED=ON for the checked build)");
#endif
}

void CheckedStartup() {
    static volatile LONG done = 0;
    if (InterlockedExchange(&done, 1) != 0) return;
    const bool selfTest = GetEnvironmentVariableA("FC4_CHECKED_SELFTEST", nullptr, 0) != 0;
#if FC4_CHECKED
    InterlockedExchange(&g_inDllMain, 0);
    if (selfTest) {
        Log("checked self-test: calling a no-argument method through a one-argument declaration");
        ProvokeEspMismatch();
        Log("checked self-test: ESP check done (%ld RTC error(s) so far)", g_rtcErrors);
        ProvokeOutOverrun();
        Log("checked self-test: out-parameter check done (%ld overrun(s) so far)", g_overruns);
    }
#else
    if (selfTest) Log("FC4_CHECKED_SELFTEST ignored: this is not a checked build");
#endif
}

long RtcErrorCount() { return g_rtcErrors; }
long OutOverrunCount() { return g_overruns; }

void ReportOutOverrun(const char* what, RefSite site, unsigned size, const uint8_t* pre, const uint8_t* post,
                      unsigned canarySize) {
    const LONG n = InterlockedIncrement(&g_overruns);
    if (n > 50) return;
    char before[64], after[64];
    Hex(pre, canarySize, before, sizeof(before));
    Hex(post, canarySize, after, sizeof(after));
    Log("OUT-PARAM OVERRUN #%ld: the callee wrote outside the %u-byte %s at %s:%d (canary C5: before %s | after %s)",
        n, size, what, site.func, site.line, before, after);
    char activity[512];
    DescribeCurrentActivity(activity, sizeof(activity));
    Log("  activity: %s", activity);
}

}  // namespace fc4
