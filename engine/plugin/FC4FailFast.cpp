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

#include "FC4FailFast.h"

#include "FC4Autotest.h"
#include "FC4Diag.h"
#include "FC4Log.h"
#include "FC4Watchdog.h"
#include "fc4.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdint>
#include <cstdlib>
#include <stdlib.h>

namespace fc4 {
namespace {

// The log lock may be held by the failing thread itself: never wait long.
constexpr unsigned kWaitMs = 200;

void PanicSink(void*, const uint8_t* text, size_t len) {
    FailFastLogText(reinterpret_cast<const char*>(text), static_cast<unsigned long>(len));
    // Which thread panicked. Rust prints "unnamed" for a Win32 thread, so name
    // it here: our watchdog and autotest threads read core state only through
    // globals the main thread publishes, never through fc4_sim_* (foundations
    // §19.2), so a panic on an unlisted tid means the GAME called the master
    // clock cross-thread - which the original tolerates only because its own
    // getters read plain members.
    const DWORD tid = GetCurrentThreadId();
    const char* who = "not ours: a GAME thread called into the core";
    if (tid == MainThreadId()) {
        who = "the MAIN thread";
    } else if (tid != 0 && tid == WatchdogThreadId()) {
        who = "our WATCHDOG thread";
    } else if (tid != 0 && tid == AutotestThreadId()) {
        who = "our AUTOTEST thread";
    }
    Log("fail-fast: the panicking thread was tid %lu (%s); main tid %lu, watchdog tid %lu, autotest tid %lu",
        static_cast<unsigned long>(tid), who, static_cast<unsigned long>(MainThreadId()),
        static_cast<unsigned long>(WatchdogThreadId()), static_cast<unsigned long>(AutotestThreadId()));
    DumpCallRing("  ");
    LogFlush();
}

void __cdecl InvalidParameter(const wchar_t* expression, const wchar_t* function, const wchar_t* file,
                              unsigned line, uintptr_t) {
    // Release CRTs pass nulls; debug ones name the call.
    LogWait(kWaitMs, "CRT invalid parameter in FreeCity4.dll: %ls in %ls (%ls:%u); fail-fast follows",
            expression ? expression : L"?", function ? function : L"?", file ? file : L"?", line);
    DumpCallRing("  ");
    LogFlush();
    __fastfail(5);  // FAST_FAIL_INVALID_ARG, as the CRT's default handler ends it
}

void __cdecl PureCall() {
    LogWait(kWaitMs, "pure virtual function call in FreeCity4.dll; abort follows");
    DumpCallRing("  ");
    LogFlush();
    std::abort();
}

}  // namespace

void FailFastLogText(const char* text, unsigned long len) {
    unsigned long at = 0;
    while (at < len) {
        unsigned long end = at;
        while (end < len && text[end] != '\n') ++end;
        LogWait(kWaitMs, "%.*s", static_cast<int>(end - at), text + at);
        at = end + 1;
    }
}

void FailFastAttach() {
    fc4_set_panic_log(&PanicSink, nullptr);
    _set_invalid_parameter_handler(&InvalidParameter);
    _set_purecall_handler(&PureCall);
    Log("fail-fast reports on: a Rust panic, a CRT invalid parameter or a pure call is logged (with the last "
        "interface calls) before the process ends");
}

}  // namespace fc4
