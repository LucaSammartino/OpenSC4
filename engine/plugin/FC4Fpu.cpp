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

#include "FC4Fpu.h"

#include "FC4FpuLogic.h"
#include "FC4Log.h"
#include "compat/FC4Compat.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <xmmintrin.h>

#include <cstdio>

namespace fc4 {
namespace {

volatile DWORD g_mainTid = 0;
volatile LONG g_haveRef = 0;
uint16_t g_ref = 0;
uint16_t g_lastTickCw = 0;
bool g_haveLastTick = false;
unsigned long g_ticks = 0;
volatile LONG g_warnedDiff = 0;
volatile LONG g_warnedPc = 0;
long g_changeLines = 0;
constexpr long kMaxChangeLines = 50;

const char* PcName(uint16_t cw) {
    switch ((cw >> 8) & 3) {
        case 0: return "24-bit";
        case 2: return "53-bit";
        case 3: return "64-bit";
        default: return "reserved";
    }
}

const char* RcName(uint16_t cw) {
    switch ((cw >> 10) & 3) {
        case 0: return "nearest";
        case 1: return "down";
        case 2: return "up";
        default: return "chop";
    }
}

bool OnMain() { return g_mainTid != 0 && GetCurrentThreadId() == g_mainTid; }

void Describe(char* out, size_t cap, uint16_t cw) {
    _snprintf_s(out, cap, _TRUNCATE, "x87 CW=0x%04X (PC %s, RC %s, masks 0x%02X) MXCSR=0x%08X", cw, PcName(cw),
                RcName(cw), cw & 0x3F, SseMxcsr());
}

}  // namespace

uint16_t X87ControlWord() {
#if defined(_M_IX86)
    uint16_t cw = 0;
    __asm fnstcw cw
    return cw;
#else
    return 0;
#endif
}

uint32_t SseMxcsr() { return _mm_getcsr(); }

void FpuCheck(const char* where) {
    if (!OnMain() || g_haveRef == 0) return;
    const uint16_t cw = X87ControlWord();
    if (cw != g_ref && InterlockedExchange(&g_warnedDiff, 1) == 0) {
        char now[128], ref[128];
        Describe(now, sizeof(now), cw);
        Describe(ref, sizeof(ref), g_ref);
        Log("fpu: WARNING the main thread's control word changed (%s): now %s; the game set %s. Original "
            "subsystems now compute at the wrong precision (foundations §18.2). Logged once.",
            where, now, ref);
    }
    if (((cw >> 8) & 3) != 0 && InterlockedExchange(&g_warnedPc, 1) == 0) {
        Log("fpu: WARNING the main thread is not at 24-bit x87 precision (%s): CW=0x%04X (PC %s). Logged once.",
            where, cw, PcName(cw));
    }
}

void FpuLog(const char* where, bool hook) {
    if (hook && g_mainTid == 0) g_mainTid = GetCurrentThreadId();
    char d[128];
    Describe(d, sizeof(d), X87ControlWord());
    Log("fpu %s: %s [thread %lu%s]", where, d, GetCurrentThreadId(), OnMain() ? ", main" : "");
    FpuCheck(where);
}

void FpuAttach() {
    // The loader thread is normally the game's main thread, but not its
    // identity source: only framework hooks mark the main thread.
    char d[128];
    Describe(d, sizeof(d), X87ControlWord());
    Log("fpu DLL attach: %s [thread %lu]", d, GetCurrentThreadId());
}

void FpuSetReference(const char* where) {
    if (g_mainTid == 0) g_mainTid = GetCurrentThreadId();
    g_ref = X87ControlWord();
    InterlockedExchange(&g_haveRef, 1);
    char d[128];
    Describe(d, sizeof(d), g_ref);
    Log("fpu %s: %s [thread %lu, main] = the reference for the guard", where, d, GetCurrentThreadId());
    FpuCheck(where);
}

void FpuReassert24Bit(const char* where) {
    if (g_mainTid == 0) g_mainTid = GetCurrentThreadId();
    if (!OnMain()) return;  // the simulation runs on the main thread only (§18.2)
    const uint16_t before = X87ControlWord();
    compat::FpuSetPrecisionFn setPrecision = nullptr;
    if (!compat::FpuSetPrecision(&setPrecision)) return;  // refused and logged there
    setPrecision(kFpuMode24Bit);
    const uint16_t after = X87ControlWord();
    const char* outcome = "?";
    switch (ReassertOutcomeOf(before, after)) {
        case ReassertOutcome::AlreadyCorrect: outcome = "already 24-bit; unchanged"; break;
        case ReassertOutcome::Restored: outcome = "RESTORED 24-bit precision"; break;
        case ReassertOutcome::Failed: outcome = "FAILED (still not 24-bit)"; break;
    }
    char b[128], a[128];
    Describe(b, sizeof(b), before);
    Describe(a, sizeof(a), after);
    Log("fpu re-assert 24-bit (%s, the game's helper 0x008929E0 mode 0): %s -> %s; %s [thread %lu, main]", where, b, a,
        outcome, GetCurrentThreadId());
    // Re-arm the guard: a change from here on is new information, not a repeat
    // of the one already reported, and the reference stays the 24-bit word.
    InterlockedExchange(&g_warnedDiff, 0);
    InterlockedExchange(&g_warnedPc, 0);
    g_lastTickCw = after;
    g_haveLastTick = true;
    FpuCheck(where);
}

void FpuTick() {
    if (g_mainTid == 0) g_mainTid = GetCurrentThreadId();  // ticks run on the main thread
    if (!OnMain()) return;
    ++g_ticks;
    const uint16_t cw = X87ControlWord();
    if (!g_haveLastTick || cw != g_lastTickCw) {
        if (g_changeLines < kMaxChangeLines) {
            ++g_changeLines;
            char d[128];
            Describe(d, sizeof(d), cw);
            if (g_haveLastTick) {
                Log("fpu tick %lu: control word changed from 0x%04X: %s", g_ticks, g_lastTickCw, d);
            } else {
                Log("fpu tick %lu (first): %s", g_ticks, d);
            }
        }
        g_lastTickCw = cw;
        g_haveLastTick = true;
    }
    char where[48];
    _snprintf_s(where, sizeof(where), _TRUNCATE, "tick %lu", g_ticks);
    FpuCheck(where);
}

}  // namespace fc4
