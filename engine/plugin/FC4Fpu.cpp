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
#include "FC4Watchdog.h"
#include "compat/FC4Compat.h"
#include "fc4.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <immintrin.h>
#include <xmmintrin.h>

#include <cstdio>
#include <cstring>

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

// ---- the x87 leftover guard ----

// Sites with findings, newest first (pushed on the main thread, read by the
// watchdog's heartbeat without a lock: a site is fully set before it is
// linked, and never unlinked).
X87Site* volatile g_x87Sites = nullptr;
volatile LONG g_x87Total = 0;
volatile LONG g_x87SfLogged = 0;
// The previous check (main thread only): its site and what it found.
const X87Site* g_x87PrevSite = nullptr;
int g_x87PrevOccupied = 0;

bool GuardThread() { return g_mainTid == 0 || OnMain(); }

void X87Register(X87Site& site) {
    if (InterlockedCompareExchange(&site.registered, 1, 0) != 0) return;
    X87Site* head = nullptr;
    do {
        head = g_x87Sites;
        site.next = head;
    } while (InterlockedCompareExchangePointer(reinterpret_cast<PVOID volatile*>(&g_x87Sites), &site, head) != head);
}

// The game month of the published julian day (year * 12 + month), or 1 when
// no day was published yet; never 0 (0 = "never logged").
long X87MonthKey(int* year, int* month) {
    const long j = PublishedJulian();
    *year = 0;
    *month = 0;
    if (j <= 0) return 1;
    fc4_date d{};
    fc4_date_from_julian(static_cast<int32_t>(j), &d);
    *year = d.year;
    *month = d.month;
    const long key = static_cast<long>(d.year) * 12 + d.month;
    return key > 1 ? key : 2;
}

// FXSAVE of the calling thread's x87/SSE state into `fx` (512 bytes, 16-aligned).
void X87Save(uint8_t* fx) {
#if defined(_M_IX86)
    _fxsave(fx);
#else
    std::memset(fx, 0, 512);
#endif
}

}  // namespace

int X87Occupied() {
    alignas(16) uint8_t fx[512];
    X87Save(fx);
    return OccupiedFromAbridgedTag(fx[4]);
}

long X87LeftoverTotal() { return g_x87Total; }

int X87Check(X87Site& site, uint32_t msg, const char* who) {
    if (!GuardThread()) return 0;
    alignas(16) uint8_t fx[512];
    X87Save(fx);
    const X87State st = DecodeFxsave(fx);
    const X87Site* prev = g_x87PrevSite;
    const int prevOccupied = g_x87PrevOccupied;
    g_x87PrevSite = &site;
    g_x87PrevOccupied = st.occupied;
    if (X87StackFault(st.sw) && InterlockedExchange(&g_x87SfLogged, 1) == 0) {
        Log("x87: the stack-fault flag is set at '%s' (FSW=0x%04X: SF=1 IE=%d C1=%d, a stack %s since the flags "
            "were last cleared; sticky, so it may be older than this check). Logged once.",
            site.name, st.sw, X87InvalidOp(st.sw) ? 1 : 0, X87C1(st.sw) ? 1 : 0,
            X87C1(st.sw) ? "overflow" : "underflow");
    }
    if (st.occupied == 0) return 0;
    X87Register(site);
    const long atSite = InterlockedIncrement(&site.count);
    const long total = InterlockedIncrement(&g_x87Total);
    int year = 0, month = 0;
    const long key = X87MonthKey(&year, &month);
    if (site.loggedMonth != key) {
        site.loggedMonth = key;
        char nan[48];
        FormatIndefinite(nan, sizeof(nan), st.indefinite);
        char detail[112] = "";
        if (msg != 0 && who != nullptr) {
            _snprintf_s(detail, sizeof(detail), _TRUNCATE, " (msg 0x%08X, '%s')", msg, who);
        } else if (msg != 0) {
            _snprintf_s(detail, sizeof(detail), _TRUNCATE, " (msg 0x%08X)", msg);
        } else if (who != nullptr) {
            _snprintf_s(detail, sizeof(detail), _TRUNCATE, " ('%s')", who);
        }
        char when[24] = "none yet";
        if (year != 0) _snprintf_s(when, sizeof(when), _TRUNCATE, "%04d-%02d", year, month);
        Log("x87 leftovers at '%s'%s: %d register(s) occupied (FSW=0x%04X TOP=%u, FTW=0x%02X), indefinite NaN: %s; "
            "previous check '%s' had %d; game month %s, %ld at this site, %ld in all",
            site.name, detail, st.occupied, st.sw, X87TopOf(st.sw), st.tag & 0xFFu, nan,
            prev ? prev->name : "(none)", prevOccupied, when, atSite, total);
    }
    return st.occupied;
}

void X87DescribeLeftovers(char* out, size_t cap) {
    if (cap == 0) return;
    const long total = g_x87Total;
    int len = _snprintf_s(out, cap, _TRUNCATE, "x87 leftovers: %ld", total);
    if (len < 0 || total == 0) return;
    // The three sites with the most findings (a few sites at most: a scan
    // per place is cheap, and nothing here allocates).
    const X87Site* shown[3] = {nullptr, nullptr, nullptr};
    for (int k = 0; k < 3; ++k) {
        const X87Site* best = nullptr;
        for (const X87Site* s = g_x87Sites; s != nullptr; s = s->next) {
            if (s == shown[0] || s == shown[1] || s->count == 0) continue;
            if (best == nullptr || s->count > best->count) best = s;
        }
        if (best == nullptr) break;
        shown[k] = best;
        const int n = _snprintf_s(out + len, cap - static_cast<size_t>(len), _TRUNCATE, "%s%s %ld", k ? ", " : " (",
                                  best->name, static_cast<long>(best->count));
        if (n < 0) return;
        len += n;
    }
    if (shown[0] != nullptr) _snprintf_s(out + len, cap - static_cast<size_t>(len), _TRUNCATE, ")");
}

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
    // The leftover guard at every FpuLog (framework hooks, city load, the
    // monthly observation, our budget settlement); `where` names the call.
    static X87Site site = {"fpu observation", 0, 0, 0, nullptr};
    X87Check(site, 0, where);
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

void FpuSimulationEntry(const char* where) {
    if (OnMain() && !Is24Bit(X87ControlWord())) FpuReassert24Bit(where);
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
    // The leftover guard once per framework tick: what the game's frame (and
    // every service ticked before the observer) left behind.
    static X87Site site = {"framework tick (observer)", 0, 0, 0, nullptr};
    X87Check(site);
}

}  // namespace fc4
