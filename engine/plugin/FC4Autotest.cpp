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

#include "FC4Autotest.h"

#include "FC4Checked.h"
#include "FC4Config.h"
#include "FC4DiagLogic.h"
#include "FC4Dialogs.h"
#include "FC4DbpfNative.h"
#include "FC4HarnessLogic.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Script.h"
#include "FC4Subsystems.h"
#include "FC4Watchdog.h"
#include "FC4WinInterfaces.h"

#include "cGZMessage.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessageTarget.h"
#include "cISC4App.h"
#include "cRZCOMDllDirector.h"

#include <cctype>
#include <cstdio>
#include <string>

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

// Rules: the verdict can be written from the vectored exception handler (any
// thread, any lock held) and from the watchdog, so writing it never allocates
// and never waits for the log lock there (LogWait 0).

namespace fc4 {
namespace {

volatile LONG g_seconds = 0;
volatile LONG g_unpause = 0;
volatile LONG g_threadStarted = 0;

volatile LONG g_cityLoaded = 0;
volatile LONG g_started = 0;
volatile DWORD g_startMs = 0;
volatile DWORD g_deadline = 0;
// This thread's id, recorded as its ThreadProc starts, so a report from
// another thread can say whose it is (FC4FailFast, foundations §19.2).
volatile DWORD g_autotestTid = 0;
int g_unpauseStage = 0;  // main thread only

volatile LONG g_verdict = 0;  // Verdict as LONG; 0 = not written
volatile LONG g_terminateArmed = 0;
volatile DWORD g_terminateAt = 0;

// autotest_end = quit: at the deadline the thread sets g_quitDue, the main
// thread's next tick asks the game to quit (g_quitRequested), and the verdict
// is written at PostSystemServiceShutdown.
volatile LONG g_endQuit = 0;
bool g_quitPost = false;  // opt-in native app command, delivered by the main-loop message pump
volatile LONG g_quitDue = 0;
volatile LONG g_quitRequested = 0;
volatile DWORD g_quitAt = 0;
volatile LONG g_quitJulian = 0;  // the last day played (Shutdown resets the published date)
constexpr DWORD kQuitGraceMs = 90000;         // [choice] the whole shutdown, city and app
// The game shut the city down by itself before the deadline (PreCityShutdown
// with no quit of ours): the run ends at its PostCityShutdown with verdict
// "ended" (quitting the game first with autotest_end = quit).
volatile LONG g_cityEnded = 0;
volatile DWORD g_endedAt = 0;
volatile LONG g_endedJulian = 0;  // the last day played before that shutdown
constexpr DWORD kExitAfterVerdictMs = 30000;  // [choice] then the process should be gone

// autotest_end_date (the harness lane; tooling): the run ends at the first
// SimNewDay on or after this julian (0 = none), as autotest_end says;
// autotest_seconds stays a timeout (0 = none) and whichever comes first ends
// the run. g_days counts the SimNewDay messages since the clock started (the
// verdict's days_per_second).
volatile LONG g_endJulian = 0;
volatile LONG g_endDateHit = 0;  // the run is ending because the end date was reached
// The end date was reached while script actions (a `save`) waited for the
// next tick (FC4Script.h): the verdict follows them, at that tick.
volatile LONG g_endAfterScript = 0;
volatile LONG g_days = 0;

// "deadline" or "end date": what ended a run that quits through the game.
const char* EndPoint() { return g_endDateHit ? "end date" : "deadline"; }

AutotestCounters Counters() {
    AutotestCounters c;
    c.foreignExceptions = ForeignExceptionCount();
    c.watchdogStalls = WatchdogStallCount();
    c.rtc = RtcErrorCount();
    c.overruns = OutOverrunCount();
    c.refWarnings = RefWarningCount();
    return c;
}

Verdict EndVerdict() { return VerdictAtEnd(Counters(), g_cityEnded != 0); }

// Writes the verdict once (the first caller wins).
// `julian` < 0: the published day. Returns true if written.
bool WriteVerdict(Verdict v, unsigned wait, const char* reason, long julian = -1) {
    if (InterlockedCompareExchange(&g_verdict, static_cast<LONG>(v), 0) != 0) return false;
    const AutotestCounters c = Counters();
    const long j = julian < 0 ? PublishedJulian() : julian;
    // The run figures (the harness lane): the date reached, the real seconds
    // since the clock started and the game days per real second, after
    // julian=. Fixed buffers: no allocation here.
    harness::RunFigures f;
    harness::FormatRunFigures(static_cast<int32_t>(j), g_started != 0, ElapsedMs(GetTickCount(), g_startMs),
                              static_cast<uint32_t>(g_days), &f);
    // close_dialogs = 1 only (FC4Dialogs.h): the dialogs it closed, after ref_warnings=.
    char dialogs[40] = "";
    if (DialogsEnabled()) std::snprintf(dialogs, sizeof(dialogs), ", dialogs_closed=%ld", DialogsClosedCount());
    LogWait(wait, "autotest: %s", reason);
    LogWait(wait,
            "AUTOTEST VERDICT: %s (foreign_exceptions=%ld, watchdog_stalls=%ld, rtc=%ld, overruns=%ld, "
            "ref_warnings=%ld%s, julian=%ld, date=%s, real_seconds=%s, days_per_second=%s, switches_off=%s, "
            "disabled=%s)",
            VerdictName(v), c.foreignExceptions, c.watchdogStalls, c.rtc, c.overruns, c.refWarnings, dialogs, j,
            f.date, f.realSeconds, f.daysPerSecond, SwitchesOffList(), DisabledSubsystemsList());
    LogFlush();
    return true;
}

[[noreturn]] void Terminate(unsigned wait, const char* from) {
    LogWait(wait, "autotest: TerminateProcess(exit code 0) from the %s", from);
    LogFlush();
    TerminateProcess(GetCurrentProcess(), 0);
    for (;;) Sleep(1000);  // TerminateProcess does not return for the current process
}

DWORD WINAPI ThreadProc(void*) {
    g_autotestTid = GetCurrentThreadId();
    for (;;) {
        Sleep(200);
        const DWORD now = GetTickCount();
        if (g_terminateArmed && static_cast<LONG>(now - g_terminateAt) >= 0) Terminate(1000, "autotest thread");
        if (!g_started || g_verdict != 0) continue;
        if (g_quitRequested) {
            if (ElapsedMs(now, g_quitAt) > kQuitGraceMs) {
                char reason[96];
                _snprintf_s(reason, sizeof(reason), _TRUNCATE, "the quit requested at the %s did not finish in 90 s",
                            EndPoint());
                WriteVerdict(Verdict::Stalled, 1000,
                             g_cityEnded ? "the quit requested after the game ended the city did not finish in 90 s"
                                         : reason);
                Terminate(1000, "autotest thread");
            }
            continue;
        }
        if (g_cityEnded) {
            if (ElapsedMs(now, g_endedAt) > kQuitGraceMs) {
                WriteVerdict(Verdict::Stalled, 1000, "the city shutdown the game started did not finish in 90 s");
                Terminate(1000, "autotest thread");
            }
            continue;
        }
        // No autotest_seconds (an autotest_end_date run): no deadline, no
        // timeout; the end date, the watchdog or an exception end the run.
        if (g_seconds <= 0) continue;
        const DWORD last = LastTickMs();
        if (g_endQuit && !g_quitDue && DeadlineVerdictDue(now, g_deadline, last)) {
            LogWait(1000,
                    "autotest: %ld s after the first tick of the loaded city: quitting at the next tick "
                    "(autotest_end = quit)",
                    static_cast<long>(g_seconds));
            InterlockedExchange(&g_quitDue, 1);
            continue;
        }
        if (!g_endQuit && DeadlineVerdictDue(now, g_deadline, last)) {
            const Verdict v = VerdictAtDeadline(Counters());
            char reason[96];
            _snprintf_s(reason, sizeof(reason), _TRUNCATE, "%ld s after the first tick of the loaded city",
                        static_cast<long>(g_seconds));
            WriteVerdict(v, 1000, reason);
            Terminate(1000, "autotest thread");
        }
        // Past the deadline with no tick for 30 s and no watchdog verdict (for
        // example FC4_NO_WATCHDOG): decide here.
        if (static_cast<LONG>(now - g_deadline) >= 0 && ElapsedMs(now, last) > 30000) {
            WriteVerdict(Verdict::Stalled, 1000, "no tick for 30 s after the deadline (watchdog silent)");
            Terminate(1000, "autotest thread");
        }
    }
}

void StartThread() {
    if (InterlockedCompareExchange(&g_threadStarted, 1, 0) != 0) return;
    // The thread pins the DLL (it never exits; the process ends with TerminateProcess).
    HMODULE self = nullptr;
    GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, reinterpret_cast<LPCWSTR>(&StartThread), &self);
    HANDLE t = CreateThread(nullptr, 64 * 1024, &ThreadProc, nullptr, 0, nullptr);
    if (t == nullptr) {
        Log("autotest: thread creation failed (error %lu)", GetLastError());
        return;
    }
    CloseHandle(t);
}

// Main thread: asks the game to quit (no dialog, no save), or ends the run as
// usual if it cannot.
void RequestQuitNow(long julian) {
    g_quitAt = GetTickCount();
    InterlockedExchange(&g_quitJulian, julian);
    InterlockedExchange(&g_quitRequested, 1);
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;  // borrowed from the framework
    if (g_quitPost) {
        cFC4IGZMessageServer* server = nullptr;
        cIGZMessageTarget* target = nullptr;
        bool posted = false;
        const char* failure = "application has no legacy message target";
        if (app && app->QueryInterface(GZIID_cIGZMessageTarget, reinterpret_cast<void**>(&target)) && target) {
            FC4_ACQUIRED(target, "QueryInterface");
            failure = "legacy message service unavailable";
            if (fw->GetSystemService(kFC4_SRVID_cIGZMessageServer, kFC4_IID_cIGZMessageServer,
                                     reinterpret_cast<void**>(&server)) && server) {
                FC4_ACQUIRED(server, "GetSystemService");
                // Native cSC4App::DoMessage at 00451C30 handles C7AA61C6 by
                // calling RequestQuit(false, false). Post, never synchronously
                // send: delivery must be outside the observer's current tick.
                const cGZMessage command(0xC7AA61C6u);
                server->GeneralMessagePostToTarget(command, target);  // void: native 00930273
                posted = true;
                FC4_RELEASE(server);
            }
            FC4_RELEASE(target);
        }
        Log("autotest: native quit command 0xC7AA61C6 queued -> %d (autotest_quit_post)", posted ? 1 : 0);
        if (posted) return;
        Log("autotest: native quit command could not be queued: %s", failure);
        WriteVerdict(Verdict::Stalled, 1000, "native quit command could not be queued");
        Terminate(1000, "observer (autotest_quit_post)");
        return;
    }
    cISC4App* sc4 = nullptr;
    if (app != nullptr && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) && sc4 != nullptr) {
        FC4_ACQUIRED(sc4, "QueryInterface");
        // [abi-ret] App slot 10 (vtables.tsv eax-or-void): no spec gives the result (question HG2). A garbage
        // true is covered by the 90 s quit grace in ThreadProc; a garbage false ends the run here instead.
        const bool ok = sc4->RequestQuit(false, false);  // slot 10: (show dialog, save first)
        FC4_RELEASE(sc4);
        Log("autotest: cISC4App::RequestQuit(false, false) -> %d; the verdict follows at PostSystemServiceShutdown",
            ok ? 1 : 0);
        if (ok) return;
    } else {
        Log("autotest: no cISC4App to quit through");
    }
    AutotestFinish(g_cityEnded      ? "the game ended the city before the deadline, and the quit could not be requested"
                   : g_endDateHit ? "end date reached, but the quit could not be requested"
                                  : "deadline reached, but the quit could not be requested",
                   "observer (autotest_end = quit)");
}

std::string LowerTrim(std::string s) {
    size_t a = 0, b = s.size();
    while (a < b && std::isspace(static_cast<unsigned char>(s[a]))) ++a;
    while (b > a && std::isspace(static_cast<unsigned char>(s[b - 1]))) --b;
    s = s.substr(a, b - a);
    for (char& c : s) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    return s;
}

}  // namespace

void AutotestReload() {
    g_quitPost = SettingInt("autotest_quit_post", "FC4_AUTOTEST_QUIT_POST", 0) == 1;
    const std::string end = LowerTrim(Setting("autotest_end", "FC4_AUTOTEST_END"));
    if (end == "quit") {
        InterlockedExchange(&g_endQuit, 1);
    } else if (!end.empty() && end != "terminate") {
        Log("autotest: WARNING autotest_end='%s' is not quit or terminate; using terminate", end.c_str());
    }
    long seconds = SettingInt("autotest_seconds", "FC4_AUTOTEST_SECONDS", 0);
    long unpause = SettingInt("autotest_unpause", "FC4_AUTOTEST_UNPAUSE", 0);
    if (seconds < 0) seconds = 0;
    if (unpause < 0 || unpause > 2) {
        Log("autotest: WARNING autotest_unpause=%ld is not 0, 1 or 2; using 2", unpause);
        unpause = 2;
    }
    // autotest_end_date = YYYY-MM-DD (the harness lane; tooling).
    const std::string endDate = LowerTrim(Setting("autotest_end_date", "FC4_AUTOTEST_END_DATE"));
    harness::Date endDay{};
    long endJulian = 0;
    if (!endDate.empty()) {
        if (harness::ParseIsoDate(endDate, &endDay)) {
            endJulian = harness::JulianFromCivil(endDay.year, endDay.month, endDay.day);
        } else {
            Log("autotest: WARNING autotest_end_date='%s' is not a date YYYY-MM-DD; ignored", endDate.c_str());
        }
    }
    InterlockedExchange(&g_unpause, unpause);
    InterlockedExchange(&g_seconds, seconds);
    InterlockedExchange(&g_endJulian, endJulian);
    const char* const ending = g_endQuit
                                   ? "after quitting the game (cISC4App::RequestQuit, verdict at PostSystemServiceShutdown)"
                                   : "then TerminateProcess";
    const char* const unpauseText = unpause ? (unpause == 1 ? "to speed 1" : "to speed 2") : "off";
    if (seconds == 0 && endJulian == 0) {
        Log("autotest: off");
        return;
    }
    if (seconds > 0) {
        Log("autotest: ON - verdict %ld s after the first tick of a loaded city, %s; unpause %s", seconds, ending,
            unpauseText);
    } else {
        Log("autotest: ON - verdict at the first new day on or after %s (autotest_end_date; no timeout: "
            "autotest_seconds = 0), %s; unpause %s",
            endDate.c_str(), ending, unpauseText);
    }
    if (endJulian != 0) {
        Log("autotest: end date %s (julian %ld): the run ends at the first new day on or after it%s", endDate.c_str(),
            endJulian, seconds > 0 ? ", or at the autotest_seconds timeout, whichever comes first" : "");
    }
    StartThread();
}

bool AutotestEnabled() { return g_seconds > 0 || g_endJulian != 0; }

uint32_t AutotestThreadId() { return g_autotestTid; }

void AutotestOnCityLoaded() {
    if (!AutotestEnabled()) return;
    if (InterlockedExchange(&g_cityLoaded, 1) == 0) Log("autotest: city loaded (PostCityInit); clock starts at the next tick");
}

namespace {

// The verdict's reason at the end date.
void EndDateReason(char* out, size_t size) {
    const harness::Date e = harness::CivilFromJulian(static_cast<int32_t>(g_endJulian));
    _snprintf_s(out, size, _TRUNCATE, "end date %04d-%02d-%02d reached (autotest_end_date)", e.year, e.month, e.day);
}

}  // namespace

int AutotestOnTick() {
    if (!AutotestEnabled() || !g_cityLoaded) return 0;
    // UI commands can pump nested ticks and span many framework ticks. Drain
    // dated actions before either end-date exit path, including graceful quit.
    // Keep the wall deadline effective if an action never finishes.
    if ((g_quitDue || g_endAfterScript) && g_endDateHit && ScriptWaiting() &&
        (g_seconds <= 0 || ElapsedMs(GetTickCount(), g_startMs) < static_cast<DWORD>(g_seconds) * 1000u)) return 0;
    if (g_quitDue && !g_quitRequested && g_verdict == 0) {
        // The opt-in DBPF shadow yields between records. Finish the last dated
        // sample on framework ticks before requesting quit; the wall deadline,
        // watchdog and external timeout remain active. Shutdown still reports
        // non-exact samples if it interrupts the reader for any other reason.
        if (DbpfShadowPending()) return 0;
        RequestQuitNow(PublishedJulian());
        return 0;
    }
    if (g_endAfterScript && g_verdict == 0) {
        // The observer ran the script's waiting actions just before this call
        // (FC4Observer.cpp, OnTick); the verdict's day is the day played now.
        char reason[96];
        EndDateReason(reason, sizeof(reason));
        WriteVerdict(VerdictAtDeadline(Counters()), 1000, reason);
        Terminate(1000, "observer (autotest_end_date)");
    }
    const DWORD now = GetTickCount();
    if (!g_started) {
        g_startMs = now;
        g_deadline = now + static_cast<DWORD>(g_seconds) * 1000u;
        InterlockedExchange(&g_started, 1);
        if (g_endJulian == 0) {
            Log("autotest: clock started; verdict in %ld s", static_cast<long>(g_seconds));
        } else {
            const harness::Date e = harness::CivilFromJulian(static_cast<int32_t>(g_endJulian));
            if (g_seconds > 0) {
                Log("autotest: clock started; verdict in %ld s or at %04d-%02d-%02d (autotest_end_date), whichever "
                    "comes first",
                    static_cast<long>(g_seconds), e.year, e.month, e.day);
            } else {
                Log("autotest: clock started; verdict at %04d-%02d-%02d (autotest_end_date), no timeout", e.year,
                    e.month, e.day);
            }
        }
    }
    if (g_unpause == 0) return 0;
    // [choice] Unpause at the first tick, and again after 3 s and 10 s in case
    // the game paused the city after loading it.
    const DWORD at[3] = {0, 3000, 10000};
    if (g_unpauseStage < 3 && ElapsedMs(now, g_startMs) >= at[g_unpauseStage]) {
        ++g_unpauseStage;
        return static_cast<int>(g_unpause);
    }
    return 0;
}

void AutotestOnNewDay(int32_t julian) {
    if (!AutotestEnabled() || !g_cityLoaded || !g_started) return;
    InterlockedIncrement(&g_days);
    if (g_verdict != 0 || g_quitDue || g_quitRequested || g_cityEnded || g_endAfterScript) return;
    if (!harness::EndDateDue(static_cast<int32_t>(g_endJulian), julian, true)) return;
    // [choice] The day is checked inside the SimNewDay message (main thread),
    // so a terminate run ends on exactly that day even when turbo plays many
    // days per tick; a quit run asks the game to quit at the next tick, as at
    // the deadline. [choice] Script actions waiting for the next tick (a
    // `save`, FC4Script.h) run first: the verdict then follows at that tick.
    InterlockedExchange(&g_endDateHit, 1);
    const bool afterScript = !g_endQuit && ScriptWaiting();
    const harness::Date e = harness::CivilFromJulian(static_cast<int32_t>(g_endJulian));
    const harness::Date d = harness::CivilFromJulian(julian);
    const DWORD ms = ElapsedMs(GetTickCount(), g_startMs);
    Log("autotest: end date %04d-%02d-%02d reached at the new day %04d-%02d-%02d (julian %ld), %lu.%03lu s after the "
        "clock started; %s",
        e.year, e.month, e.day, d.year, d.month, d.day, static_cast<long>(julian), static_cast<unsigned long>(ms / 1000),
        static_cast<unsigned long>(ms % 1000),
        g_endQuit     ? "quitting at the next tick (autotest_end = quit)"
        : afterScript ? "the script's waiting action(s) first, then the verdict and TerminateProcess at the next tick"
                      : "verdict, then TerminateProcess");
    if (g_endQuit) {
        InterlockedExchange(&g_quitDue, 1);
        return;
    }
    if (afterScript) {
        InterlockedExchange(&g_endAfterScript, 1);
        return;
    }
    char reason[96];
    EndDateReason(reason, sizeof(reason));
    WriteVerdict(VerdictAtDeadline(Counters()), 1000, reason, julian);
    Terminate(1000, "observer (autotest_end_date)");
}

void AutotestFinish(const char* reason, const char* from) {
    WriteVerdict(EndVerdict(), 1000, reason, g_cityEnded ? g_endedJulian : -1);
    Terminate(1000, from);
}

void AutotestOnShutdownComplete(const char* hook) {
    if (!g_quitRequested || g_verdict != 0) return;
    char reason[128];
    if (g_cityEnded) {
        _snprintf_s(reason, sizeof(reason), _TRUNCATE,
                    "the game ended the city before the deadline; the quit after it reached %s", hook);
    } else {
        _snprintf_s(reason, sizeof(reason), _TRUNCATE, "the game quit at the %s and reached %s", EndPoint(), hook);
    }
    if (!WriteVerdict(EndVerdict(), 1000, reason, g_quitJulian)) return;
    // The game exits by itself from here; if it does not, the thread ends it.
    g_terminateAt = GetTickCount() + kExitAfterVerdictMs;
    InterlockedExchange(&g_terminateArmed, 1);
}

void AutotestOnCityShutdown(bool post) {
    if (!AutotestEnabled() || !g_started || g_quitDue || g_quitRequested || g_verdict != 0) return;
    if (!post) {
        if (InterlockedExchange(&g_cityEnded, 1) != 0) return;
        g_endedAt = GetTickCount();
        InterlockedExchange(&g_endedJulian, PublishedJulian());
        if (g_seconds > 0) {
            Log("autotest: WARNING the game is shutting the city down by itself %lu s after the clock started, before "
                "the %ld s deadline (julian %ld); the run ends at its PostCityShutdown with verdict \"ended\"",
                static_cast<unsigned long>(ElapsedMs(g_endedAt, g_startMs) / 1000), static_cast<long>(g_seconds),
                static_cast<long>(g_endedJulian));
        } else {
            Log("autotest: WARNING the game is shutting the city down by itself %lu s after the clock started, before "
                "the end date (julian %ld); the run ends at its PostCityShutdown with verdict \"ended\"",
                static_cast<unsigned long>(ElapsedMs(g_endedAt, g_startMs) / 1000), static_cast<long>(g_endedJulian));
        }
        return;
    }
    if (!g_cityEnded) return;
    if (g_endQuit) {
        Log("autotest: the city the game ended has shut down; quitting (autotest_end = quit)");
        RequestQuitNow(g_endedJulian);
        return;
    }
    AutotestFinish("the game ended the city before the deadline (PostCityShutdown)", "observer (city ended)");
}

void AutotestOnForeignException() {
    if (!AutotestEnabled()) return;
    WriteVerdict(Verdict::Corrupt, 0, "first-chance exception outside FreeCity4.dll (see the report above)");
    if (InterlockedCompareExchange(&g_terminateArmed, 1, 0) == 0) {
        g_terminateAt = GetTickCount() + kAutotestCorruptGraceMs;
        LogWait(0, "autotest: terminating in %lu ms (time for stall evidence)",
                static_cast<unsigned long>(kAutotestCorruptGraceMs));
        StartThread();  // normally running already
    }
}

void AutotestOnStall(const char* kind, uint32_t stalledMs) {
    if (!AutotestEnabled()) return;
    char reason[128];
    _snprintf_s(reason, sizeof(reason), _TRUNCATE, "main thread stalled (%s) for %lu ms", kind,
                static_cast<unsigned long>(stalledMs));
    WriteVerdict(Verdict::Stalled, 1000, reason);
    Terminate(1000, "watchdog thread");
}

}  // namespace fc4
