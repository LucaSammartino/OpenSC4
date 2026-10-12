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

// Pure decision logic of the diagnostics (watchdog stall rule, autotest
// verdict), header-only and free of Windows calls so tests/diag_logic_test.cpp
// can check it directly. Tooling only; no spec behaviour.
#pragma once

#include <cstdint>

namespace fc4 {

// Milliseconds from `since` to `now` on the 32-bit GetTickCount clock
// (wrap-around safe). A timestamp written by another thread after `now` was
// read is "in the future": that is 0 ms, never ~4294967295.
inline uint32_t ElapsedMs(uint32_t now, uint32_t since) {
    const uint32_t d = now - since;
    return static_cast<int32_t>(d) < 0 ? 0u : d;
}

enum class StallKind { None, InsideFC4, NoOnTick };

struct StallInput {
    uint32_t now = 0;
    long depth = 0;            // main thread's activity depth inside FreeCity4
    uint32_t actSince0 = 0;    // when the outermost activity started
    bool cityRunning = false;
    bool hiddenPaused = false;
    uint32_t lastTickMs = 0;   // 0 = no OnTick seen yet
    uint32_t stallMs = 5000;   // threshold inside FreeCity4
    uint32_t noTickMs = 15000; // threshold without OnTick
};

struct StallResult {
    StallKind kind = StallKind::None;
    uint32_t since = 0;      // start of the stall (episode key)
    uint32_t stalledMs = 0;  // how long so far
};

// The watchdog's stall rule: inside FreeCity4 for more than stallMs, or (city
// running, not hidden-paused, outside FreeCity4) no OnTick for more than noTickMs.
inline StallResult EvaluateStall(const StallInput& in) {
    StallResult r;
    if (in.depth > 0) {
        const uint32_t ms = ElapsedMs(in.now, in.actSince0);
        if (ms > in.stallMs) {
            r.kind = StallKind::InsideFC4;
            r.since = in.actSince0;
            r.stalledMs = ms;
        }
    } else if (in.cityRunning && !in.hiddenPaused && in.lastTickMs != 0) {
        const uint32_t ms = ElapsedMs(in.now, in.lastTickMs);
        if (ms > in.noTickMs) {
            r.kind = StallKind::NoOnTick;
            r.since = in.lastTickMs;
            r.stalledMs = ms;
        }
    }
    return r;
}

inline const char* StallKindName(StallKind k) {
    switch (k) {
        case StallKind::InsideFC4: return "inside FreeCity4";
        case StallKind::NoOnTick: return "no OnTick";
        default: return "none";
    }
}

// ---- autotest ----------------------------------------------------------------------------------

enum class Verdict { None, Clean, Corrupt, Stalled };

inline const char* VerdictName(Verdict v) {
    switch (v) {
        case Verdict::Clean: return "clean";
        case Verdict::Corrupt: return "corrupt";
        case Verdict::Stalled: return "stalled";
        default: return "none";
    }
}

struct AutotestCounters {
    long foreignExceptions = 0;
    long watchdogStalls = 0;
    long rtc = 0;
    long overruns = 0;
    long refWarnings = 0;
};

// The verdict when the autotest time is up. [choice] RTC errors and out-param
// overruns (checked build) count as corruption like a foreign exception;
// reference warnings and finished stalls are reported but do not decide.
inline Verdict VerdictAtDeadline(const AutotestCounters& c) {
    return (c.foreignExceptions > 0 || c.rtc > 0 || c.overruns > 0) ? Verdict::Corrupt : Verdict::Clean;
}

// Autotest stall rule: a stall reported by the watchdog becomes the verdict
// once it has lasted longer than this.
constexpr uint32_t kAutotestStallMs = 10000;
// After a "corrupt" verdict, the process lives this long so the watchdog can
// still record a stall that follows.
constexpr uint32_t kAutotestCorruptGraceMs = 5000;

// What the autotest clock does at `now` (all times GetTickCount ms):
//   deadline reached and the main thread ticked recently -> write the verdict;
//   deadline reached but the last OnTick is older than 2 s -> keep waiting (a
//   stall in progress becomes "stalled" through the watchdog instead).
inline bool DeadlineVerdictDue(uint32_t now, uint32_t deadline, uint32_t lastTickMs) {
    if (static_cast<int32_t>(now - deadline) < 0) return false;
    return lastTickMs != 0 && ElapsedMs(now, lastTickMs) <= 2000;
}

}  // namespace fc4
