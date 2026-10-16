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

// Unit tests of plugin/FC4DiagLogic.h: the watchdog's elapsed-time and stall
// rules (including the "no OnTick for 4294967249 ms" underflow seen in run 6)
// and the autotest verdict; and of plugin/FC4AutoloadLogic.h, the autoload's
// wall-clock wait for the region's city list.

#include "FC4AutoloadLogic.h"
#include "FC4DiagLogic.h"

#include <cstdio>

namespace {

int g_failures = 0;

#define CHECK(cond)                                                          \
    do {                                                                     \
        if (!(cond)) {                                                       \
            std::printf("FAILED: %s (line %d)\n", #cond, __LINE__);          \
            ++g_failures;                                                    \
        }                                                                    \
    } while (0)

}  // namespace

int main() {
    using namespace fc4;

    // ElapsedMs: ordinary, wrap-around, and a timestamp newer than `now`.
    CHECK(ElapsedMs(1000, 400) == 600);
    CHECK(ElapsedMs(5, 0xFFFFFFF0u) == 21);  // GetTickCount wrapped
    CHECK(ElapsedMs(1000, 1047) == 0);       // was 4294967249
    CHECK(ElapsedMs(7, 7) == 0);

    // The run-6 case: the main thread wrote lastTickMs after the watchdog read `now`.
    {
        StallInput in;
        in.now = 100000;
        in.cityRunning = true;
        in.lastTickMs = 100047;
        const StallResult r = EvaluateStall(in);
        CHECK(r.kind == StallKind::None);
        CHECK(r.stalledMs == 0);
    }
    // Same for an activity that started after `now` was read.
    {
        StallInput in;
        in.now = 100000;
        in.depth = 1;
        in.actSince0 = 100010;
        CHECK(EvaluateStall(in).kind == StallKind::None);
    }
    // Real stalls.
    {
        StallInput in;
        in.now = 200000;
        in.cityRunning = true;
        in.lastTickMs = 200000 - 16000;
        const StallResult r = EvaluateStall(in);
        CHECK(r.kind == StallKind::NoOnTick && r.stalledMs == 16000 && r.since == 184000);
        in.hiddenPaused = true;
        CHECK(EvaluateStall(in).kind == StallKind::None);
        in.hiddenPaused = false;
        in.cityRunning = false;
        CHECK(EvaluateStall(in).kind == StallKind::None);
        in.cityRunning = true;
        in.lastTickMs = 0;  // no tick seen yet
        CHECK(EvaluateStall(in).kind == StallKind::None);
    }
    {
        StallInput in;
        in.now = 300000;
        in.depth = 2;
        in.actSince0 = 300000 - 6000;
        const StallResult r = EvaluateStall(in);
        CHECK(r.kind == StallKind::InsideFC4 && r.stalledMs == 6000);
        in.actSince0 = 300000 - 4000;
        CHECK(EvaluateStall(in).kind == StallKind::None);
    }

    // Autotest verdicts.
    {
        AutotestCounters c;
        CHECK(VerdictAtDeadline(c) == Verdict::Clean);
        c.refWarnings = 3;
        c.watchdogStalls = 1;
        CHECK(VerdictAtDeadline(c) == Verdict::Clean);
        c.foreignExceptions = 1;
        CHECK(VerdictAtDeadline(c) == Verdict::Corrupt);
        c.foreignExceptions = 0;
        c.rtc = 1;
        CHECK(VerdictAtDeadline(c) == Verdict::Corrupt);
        c.rtc = 0;
        c.overruns = 2;
        CHECK(VerdictAtDeadline(c) == Verdict::Corrupt);
        CHECK(VerdictName(Verdict::Stalled)[0] == 's');
        CHECK(VerdictAtEnd(c, true) == Verdict::Corrupt);  // corruption decides first
        c.overruns = 0;
        CHECK(VerdictAtEnd(c, false) == Verdict::Clean);
        CHECK(VerdictAtEnd(c, true) == Verdict::Ended);
        CHECK(VerdictName(Verdict::Ended)[0] == 'e');
    }
    CHECK(!DeadlineVerdictDue(9999, 10000, 9999));
    CHECK(DeadlineVerdictDue(10000, 10000, 9500));
    CHECK(!DeadlineVerdictDue(20000, 10000, 15000));  // main thread silent for 5 s: wait
    CHECK(DeadlineVerdictDue(20000, 10000, 20003));   // tick newer than `now`
    CHECK(!DeadlineVerdictDue(20000, 10000, 0));

    // The autoload's wall-clock wait for the region's city list
    // (FC4AutoloadLogic.h): ticks about 1 ms apart at start-up used to spend
    // the old 10-check budget within ~130 ms.
    {
        AutoloadEmptyWait w;
        CHECK(AutoloadCheckDue(w, 5000));
        CHECK(AutoloadWaitedMs(w, 5000) == 0);
        CHECK(AutoloadRecordEmpty(w, 5000));
        CHECK(w.checks == 1 && w.firstMs == 5000 && w.lastMs == 5000);
        int due = 0;
        for (uint32_t t = 5001; t < 6000; ++t) due += AutoloadCheckDue(w, t) ? 1 : 0;
        CHECK(due == 0);  // no second read within the second
        CHECK(AutoloadCheckDue(w, 6000));
        uint32_t now = 6000;
        while (AutoloadRecordEmpty(w, now)) now += kAutoloadRetryMs;
        CHECK(now == 5000 + kAutoloadGiveUpMs);  // gives up 120 s after the first empty read
        CHECK(w.checks == 121);
        CHECK(AutoloadWaitedMs(w, now) == 120000);
    }
    // GetTickCount wraps during the wait.
    {
        AutoloadEmptyWait w;
        CHECK(AutoloadRecordEmpty(w, 0xFFFFFF00u));
        CHECK(!AutoloadCheckDue(w, 0x100u));  // 512 ms later
        CHECK(AutoloadCheckDue(w, 0x300u));   // 1024 ms later
        CHECK(AutoloadRecordEmpty(w, 0x300u));
        CHECK(AutoloadWaitedMs(w, 0x300u) == 1024);
        CHECK(AutoloadRecordEmpty(w, 0xFFFFFF00u + kAutoloadGiveUpMs - 1));
        CHECK(!AutoloadRecordEmpty(w, 0xFFFFFF00u + kAutoloadGiveUpMs));
    }

    if (g_failures == 0) std::printf("diag logic test: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}
