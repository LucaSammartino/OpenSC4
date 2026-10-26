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

#include "FC4Watchdog.h"

#include "FC4Autotest.h"
#include "FC4Diag.h"
#include "FC4DiagLogic.h"
#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4Script.h"
#include "fc4.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <psapi.h>

#include <cstdio>
#include <cstdlib>
#include <cstring>

// Rules for this file:
// - The main thread only writes plain words and fixed buffers here (no locks,
//   no allocation, no logging), so marking activity cannot block it.
// - While the main thread is suspended, the watchdog calls only
//   GetThreadContext and ReadProcessMemory (system calls: no heap, no loader
//   lock, no log lock); everything else happens after ResumeThread.
// - The watchdog logs with LogWait: it never waits more than 1 s for the log
//   lock, then writes without it.

namespace fc4 {
namespace {

constexpr int kDepth = 16;
constexpr unsigned kMaxSamples = 10;
constexpr unsigned kStackDwords = 1024;  // 4 KiB of the main thread's stack per sample
constexpr unsigned kMaxReturnAddresses = 64;

// ---- shared state (main thread writes, watchdog reads) -------------------------------------

volatile DWORD g_mainTid = 0;
HANDLE g_mainThread = nullptr;
uintptr_t g_mainStackBase = 0;
uintptr_t g_mainStackLimit = 0;
// This watchdog thread's id, recorded as its ThreadProc starts, so a report
// from another thread can say whose it is (FC4FailFast, foundations §19.2).
volatile DWORD g_watchdogTid = 0;

const char* volatile g_actLabel[kDepth] = {};
volatile DWORD g_actSince[kDepth] = {};
volatile LONG g_actDepth = 0;  // may exceed kDepth (only the first kDepth are recorded)

volatile LONG g_delActive = 0;
volatile DWORD g_delSince = 0;
volatile uint32_t g_delLevel = 0;
volatile uint32_t g_delType = 0;
const void* volatile g_delAgent = nullptr;
char g_delName[64] = {};

volatile LONG g_ticks = 0;
volatile DWORD g_lastTickMs = 0;
volatile LONG g_deliveries = 0;
volatile LONG g_ifaceCalls = 0;
volatile LONG g_suppressed = 0;
volatile LONG g_foreign = 0;
volatile LONG g_stallEpisodes = 0;

volatile LONG g_julian = 0;
volatile LONG g_speed = 0;
volatile LONG g_pause = 0;
volatile LONG g_hidden = 0;
volatile LONG g_emergency = 0;
volatile LONG g_cityRunning = 0;

// ---- module snapshot (watchdog writes, anyone reads) ---------------------------------------

struct Module {
    uintptr_t base;
    uintptr_t size;
    char name[40];
};
constexpr unsigned kModules = 256;
Module g_mods[2][kModules];
volatile LONG g_modCount[2] = {0, 0};
volatile LONG g_modActive = 0;
uintptr_t g_exeBase = 0;
uintptr_t g_exeSize = 0;

// ---- the watchdog thread ----------------------------------------------------------------------

HANDLE g_thread = nullptr;
HANDLE g_stop = nullptr;
volatile LONG g_started = 0;

DWORD EnvMs(const char* name, DWORD fallback) {
    char v[16] = {};
    const DWORD n = GetEnvironmentVariableA(name, v, sizeof(v));
    const long x = (n > 0 && n < sizeof(v)) ? strtol(v, nullptr, 10) : 0;
    return x > 0 ? static_cast<DWORD>(x) : fallback;
}

uintptr_t ImageSizeAt(uintptr_t base) {
    const auto* dos = reinterpret_cast<const IMAGE_DOS_HEADER*>(base);
    if (dos == nullptr || dos->e_magic != IMAGE_DOS_SIGNATURE) return 0;
    const auto* nt = reinterpret_cast<const IMAGE_NT_HEADERS*>(base + dos->e_lfanew);
    return nt->Signature == IMAGE_NT_SIGNATURE ? nt->OptionalHeader.SizeOfImage : 0;
}

// Reads the loaded-module list without the loader lock (psapi reads the PEB).
void RefreshModules() {
    HMODULE mods[kModules];
    DWORD needed = 0;
    if (!K32EnumProcessModules(GetCurrentProcess(), mods, sizeof(mods), &needed)) return;
    const LONG target = 1 - g_modActive;
    unsigned n = 0;
    const unsigned total = needed / sizeof(HMODULE) < kModules ? needed / sizeof(HMODULE) : kModules;
    for (unsigned i = 0; i < total; ++i) {
        MODULEINFO mi{};
        if (!K32GetModuleInformation(GetCurrentProcess(), mods[i], &mi, sizeof(mi))) continue;
        Module& m = g_mods[target][n];
        m.base = reinterpret_cast<uintptr_t>(mi.lpBaseOfDll);
        m.size = mi.SizeOfImage;
        if (K32GetModuleBaseNameA(GetCurrentProcess(), mods[i], m.name, sizeof(m.name)) == 0) {
            strcpy_s(m.name, "?");
        }
        ++n;
    }
    g_modCount[target] = static_cast<LONG>(n);
    InterlockedExchange(&g_modActive, target);
}

struct CpuTimes {
    unsigned long long processMs = 0;
    unsigned long long mainMs = 0;
};

unsigned long long FileTimeMs(const FILETIME& a, const FILETIME& b) {
    const unsigned long long x = (static_cast<unsigned long long>(a.dwHighDateTime) << 32) | a.dwLowDateTime;
    const unsigned long long y = (static_cast<unsigned long long>(b.dwHighDateTime) << 32) | b.dwLowDateTime;
    return (x + y) / 10000ull;
}

CpuTimes Cpu() {
    CpuTimes t;
    FILETIME c, e, k, u;
    if (GetProcessTimes(GetCurrentProcess(), &c, &e, &k, &u)) t.processMs = FileTimeMs(k, u);
    if (g_mainThread != nullptr && GetThreadTimes(g_mainThread, &c, &e, &k, &u)) t.mainMs = FileTimeMs(k, u);
    return t;
}

// The process's memory: address space used and the largest free block (a
// 32-bit game fails large allocations once its address space fragments),
// and the private bytes. Walks the address space (a few thousand
// VirtualQuery calls): heartbeats only.
void DescribeMemory(char* out, size_t cap) {
    MEMORYSTATUSEX ms{};
    ms.dwLength = sizeof(ms);
    unsigned long long used = 0, total = 0, largest = 0;
    if (GlobalMemoryStatusEx(&ms)) {
        total = ms.ullTotalVirtual;
        used = ms.ullTotalVirtual - ms.ullAvailVirtual;
    }
    MEMORY_BASIC_INFORMATION mbi;
    for (uintptr_t a = 0x10000; VirtualQuery(reinterpret_cast<const void*>(a), &mbi, sizeof(mbi)) == sizeof(mbi);) {
        if (mbi.State == MEM_FREE && mbi.RegionSize > largest) largest = mbi.RegionSize;
        const uintptr_t next = reinterpret_cast<uintptr_t>(mbi.BaseAddress) + mbi.RegionSize;
        if (next <= a) break;
        a = next;
    }
    PROCESS_MEMORY_COUNTERS_EX pmc{};
    pmc.cb = sizeof(pmc);
    unsigned long long priv = 0;
    if (GetProcessMemoryInfo(GetCurrentProcess(), reinterpret_cast<PROCESS_MEMORY_COUNTERS*>(&pmc), sizeof(pmc))) {
        priv = pmc.PrivateUsage;
    }
    _snprintf_s(out, cap, _TRUNCATE,
                "memory: address space used %llu of %llu MiB, largest free block %llu MiB, private %llu MiB", used >> 20,
                total >> 20, largest >> 20, priv >> 20);
}

// "what the main thread is doing" into `out`.
void DescribeActivity(DWORD now, char* out, size_t cap) {
    out[0] = 0;
    const LONG depth = g_actDepth;
    size_t len = 0;
    if (depth <= 0) {
        const DWORD last = g_lastTickMs;
        _snprintf_s(out, cap, _TRUNCATE, "outside FreeCity4 (last OnTick %lu ms ago)",
                    last ? static_cast<unsigned long>(ElapsedMs(now, last)) : 0ul);
        return;
    }
    for (LONG i = 0; i < depth && i < kDepth; ++i) {
        const char* l = g_actLabel[i];
        const int k = _snprintf_s(out + len, cap - len, _TRUNCATE, "%s%s (%lu ms)", i ? " > " : "",
                                  l ? l : "?", static_cast<unsigned long>(ElapsedMs(now, g_actSince[i])));
        if (k < 0) return;
        len += static_cast<size_t>(k);
    }
    if (depth > kDepth) {
        const int k = _snprintf_s(out + len, cap - len, _TRUNCATE, " > ... (depth %ld)", depth);
        if (k > 0) len += static_cast<size_t>(k);
    }
    if (g_delActive) {
        char name[sizeof(g_delName)];
        memcpy(name, g_delName, sizeof(name));
        name[sizeof(name) - 1] = 0;
        _snprintf_s(out + len, cap - len, _TRUNCATE,
                    " | delivering msg 0x%08X (level %u) to agent %p '%s' for %lu ms", g_delType, g_delLevel,
                    g_delAgent, name, static_cast<unsigned long>(ElapsedMs(now, g_delSince)));
    }
}

void Heartbeat(const char* why, bool deltas) {
    static LONG lastTicks = 0, lastDeliveries = 0, lastCalls = 0, lastSuppressed = 0, lastForeign = 0;
    const LONG ticks = g_ticks, deliveries = g_deliveries, calls = g_ifaceCalls, suppressed = g_suppressed,
               foreign = g_foreign;
    fc4_date d{};
    fc4_date_from_julian(g_julian, &d);
    char activity[512];
    DescribeActivity(GetTickCount(), activity, sizeof(activity));
    const CpuTimes cpu = Cpu();
    char memory[160];
    DescribeMemory(memory, sizeof(memory));
    // The x87 leftover guard's running total (FC4Fpu.h), with its top sites.
    char x87[192];
    X87DescribeLeftovers(x87, sizeof(x87));
    if (deltas) {
        LogWait(1000,
                "heartbeat (%s): %04d-%02d-%02d (julian %ld) speed %ld pause %ld hidden %ld emergency %ld city %s | "
                "+%ld OnTick, +%ld deliveries, +%ld interface calls, +%ld verbose lines suppressed, +%ld foreign "
                "exceptions | main: %s | cpu: process %llu ms, main thread %llu ms | log lines written unlocked %lu | %s "
                "| %s",
                why, d.year, d.month, d.day_of_month, g_julian, g_speed, g_pause, g_hidden, g_emergency,
                g_cityRunning ? "running" : "not running", ticks - lastTicks, deliveries - lastDeliveries,
                calls - lastCalls, suppressed - lastSuppressed, foreign - lastForeign, activity, cpu.processMs,
                cpu.mainMs, LogUnlockedLines(), memory, x87);
        lastTicks = ticks;
        lastDeliveries = deliveries;
        lastCalls = calls;
        lastSuppressed = suppressed;
        lastForeign = foreign;
    } else {
        Log("heartbeat (%s): %04d-%02d-%02d (julian %ld) speed %ld pause %ld hidden %ld emergency %ld | totals: %ld "
            "OnTick, %ld deliveries, %ld interface calls, %ld verbose lines suppressed, %ld foreign exceptions | cpu: "
            "process %llu ms, main thread %llu ms | %s | %s",
            why, d.year, d.month, d.day_of_month, g_julian, g_speed, g_pause, g_hidden, g_emergency, ticks,
            deliveries, calls, suppressed, foreign, cpu.processMs, cpu.mainMs, memory, x87);
    }
}

bool InRange(uintptr_t a, uintptr_t base, uintptr_t size) { return base != 0 && a >= base && a - base < size; }

bool ReadSelf(uintptr_t addr, void* out, size_t len) {
    SIZE_T got = 0;
    return ReadProcessMemory(GetCurrentProcess(), reinterpret_cast<const void*>(addr), out, len, &got) && got == len;
}

// A value that points just after a call instruction (E8 rel32, or FF /2).
bool LooksLikeReturnAddress(uintptr_t v) {
    unsigned char b[7];
    if (v < 7 || !ReadSelf(v - 7, b, sizeof(b))) return false;
    // b[6] is the byte before v.
    if (b[2] == 0xE8) return true;                                         // call rel32
    auto isCall = [](unsigned char modrm) { return ((modrm >> 3) & 7) == 2; };
    if (b[5] == 0xFF && isCall(b[6]) && (b[6] >> 6) == 3) return true;      // call reg
    if (b[5] == 0xFF && isCall(b[6]) && (b[6] >> 6) == 0 && (b[6] & 7) != 4 && (b[6] & 7) != 5) return true;  // call [reg]
    if (b[4] == 0xFF && isCall(b[5]) && (b[5] >> 6) == 1) return true;      // call [reg+disp8]
    if (b[3] == 0xFF && isCall(b[4]) && (b[4] >> 6) == 1 && (b[4] & 7) == 4) return true;  // call [sib+disp8]
    if (b[1] == 0xFF && isCall(b[2]) && ((b[2] >> 6) == 2 || ((b[2] >> 6) == 0 && (b[2] & 7) == 5))) return true;  // disp32
    if (b[0] == 0xFF && isCall(b[1]) && (b[1] >> 6) == 2 && (b[1] & 7) == 4) return true;  // call [sib+disp32]
    return false;
}

// One sample of the main thread. Nothing between SuspendThread and
// ResumeThread may allocate or take a lock.
void Sample(unsigned index, const char* kind, DWORD stalledMs, bool full) {
    static DWORD stack[kStackDwords];
    static CONTEXT ctx;
    memset(&ctx, 0, sizeof(ctx));
    ctx.ContextFlags = CONTEXT_CONTROL | CONTEXT_INTEGER;
    SIZE_T got = 0;
    bool haveCtx = false;
    DWORD suspendError = 0;
    const DWORD t0 = GetTickCount();
    if (SuspendThread(g_mainThread) != static_cast<DWORD>(-1)) {
        haveCtx = GetThreadContext(g_mainThread, &ctx) != FALSE;
        if (haveCtx) {
            uintptr_t want = sizeof(stack);
            if (ctx.Esp >= g_mainStackLimit && ctx.Esp < g_mainStackBase && g_mainStackBase - ctx.Esp < want) {
                want = g_mainStackBase - ctx.Esp;
            }
            if (!ReadProcessMemory(GetCurrentProcess(), reinterpret_cast<const void*>(ctx.Esp), stack, want, &got)) {
                got = 0;
            }
        }
        ResumeThread(g_mainThread);
    } else {
        suspendError = GetLastError();
    }
    const DWORD suspendedMs = GetTickCount() - t0;

    // ---- formatting (the main thread runs again) ----
    const DWORD now = GetTickCount();
    char activity[768];
    DescribeActivity(now, activity, sizeof(activity));
    const CpuTimes cpu = Cpu();
    static CpuTimes prev;
    if (index == 0) prev = cpu;
    LogWait(1000, "WATCHDOG: main thread (tid %lu) stalled - %s for %lu ms [sample %u/%u]", static_cast<unsigned long>(g_mainTid),
            kind, static_cast<unsigned long>(stalledMs), index + 1, kMaxSamples);
    LogWait(1000, "  activity: %s", activity);
    LogWait(1000, "  cpu: process %llu ms (+%llu since the previous sample), main thread %llu ms (+%llu)",
            cpu.processMs, cpu.processMs - prev.processMs, cpu.mainMs, cpu.mainMs - prev.mainMs);
    prev = cpu;
    if (!haveCtx) {
        LogWait(1000, "  context: unavailable (suspend error %lu)", static_cast<unsigned long>(suspendError));
    } else {
        char eip[96];
        DescribeAddress(ctx.Eip, eip, sizeof(eip));
        LogWait(1000, "  context: eip=0x%08lX (%s) esp=0x%08lX ebp=0x%08lX eax=0x%08lX ecx=0x%08lX edx=0x%08lX "
                      "(suspended %lu ms, %lu stack bytes copied)",
                static_cast<unsigned long>(ctx.Eip), eip, static_cast<unsigned long>(ctx.Esp),
                static_cast<unsigned long>(ctx.Ebp), static_cast<unsigned long>(ctx.Eax),
                static_cast<unsigned long>(ctx.Ecx), static_cast<unsigned long>(ctx.Edx),
                static_cast<unsigned long>(suspendedMs), static_cast<unsigned long>(got));
        unsigned found = 0;
        const unsigned words = static_cast<unsigned>(got / sizeof(DWORD));
        for (unsigned i = 0; i < words && found < kMaxReturnAddresses; ++i) {
            const uintptr_t v = stack[i];
            const bool candidate = InRange(v, g_exeBase, g_exeSize) || InRange(v, ImageBase(), ImageSize());
            if (!candidate || !LooksLikeReturnAddress(v)) continue;
            char where[96];
            DescribeAddress(v, where, sizeof(where));
            LogWait(1000, "    [esp+0x%04X] %s", i * 4, where);
            ++found;
        }
        if (found == 0) LogWait(1000, "    no return address into the executable or FreeCity4.dll in the copied stack");
    }
    if (full) DumpCallRing("  ");
}

DWORD WINAPI ThreadProc(void*) {
    g_watchdogTid = GetCurrentThreadId();
    const DWORD stallMs = EnvMs("FC4_WATCHDOG_STALL_MS", 5000);
    const DWORD noTickMs = EnvMs("FC4_WATCHDOG_NOTICK_MS", 15000);
    const DWORD heartbeatMs = EnvMs("FC4_HEARTBEAT_MS", 10000);
    const DWORD sampleMs = EnvMs("FC4_WATCHDOG_SAMPLE_MS", 5000);
    DWORD lastHeartbeat = GetTickCount();
    DWORD lastRefresh = lastHeartbeat;
    DWORD episodeKey = 0;
    bool inEpisode = false;
    unsigned samples = 0;
    DWORD nextSample = 0;
    DWORD episodeStart = 0;
    LogWait(1000, "watchdog: running (stall %lu ms, no-OnTick %lu ms, heartbeat %lu ms, main tid %lu, watchdog tid %lu)",
            static_cast<unsigned long>(stallMs), static_cast<unsigned long>(noTickMs),
            static_cast<unsigned long>(heartbeatMs), static_cast<unsigned long>(g_mainTid),
            static_cast<unsigned long>(g_watchdogTid));
    while (WaitForSingleObject(g_stop, 1000) == WAIT_TIMEOUT) {
        const DWORD now = GetTickCount();
        if (now - lastRefresh >= 10000) {
            RefreshModules();
            lastRefresh = now;
        }
        // Stall rules (FC4DiagLogic.h). Timestamps the main thread wrote after
        // `now` was read count as 0 ms, not as an unsigned underflow.
        // Acquire the save allowance before reading progress: its transition
        // to zero publishes the framework tick resumed after the save.
        const DWORD saveGrace = ScriptSaveGraceMs();
        StallInput in;
        in.now = now;
        in.depth = g_actDepth;
        in.actSince0 = g_actSince[0];
        in.cityRunning = g_cityRunning != 0;
        in.hiddenPaused = g_hidden != 0;
        in.lastTickMs = g_lastTickMs;
        in.stallMs = stallMs;
        // Under autotest, a stall outside FreeCity4 is sampled after stallMs
        // too, so the evidence is in the log before the 10 s verdict.
        in.noTickMs = AutotestEnabled() && noTickMs > stallMs ? stallMs : noTickMs;
        // Full saves may take longer than autotest's normal ten-second stall
        // limit. This explicit, bounded script-only allowance defaults to zero.
        if (saveGrace > in.stallMs) in.stallMs = saveGrace;
        if (saveGrace > in.noTickMs) in.noTickMs = saveGrace;
        const StallResult st = EvaluateStall(in);
        if (st.kind != StallKind::None) {
            const DWORD key = st.kind == StallKind::InsideFC4 ? (st.since | 1u) : (st.since & ~1u);
            if (!inEpisode || key != episodeKey) {
                inEpisode = true;
                episodeKey = key;
                episodeStart = st.since;
                samples = 0;
                nextSample = now;
                InterlockedIncrement(&g_stallEpisodes);
            }
            if (samples < kMaxSamples && static_cast<LONG>(now - nextSample) >= 0) {
                Sample(samples, StallKindName(st.kind), ElapsedMs(now, episodeStart), samples == 0);
                ++samples;
                nextSample = now + sampleMs;
            }
            if (st.stalledMs > kAutotestStallMs) AutotestOnStall(StallKindName(st.kind), st.stalledMs);
        } else if (inEpisode) {
            LogWait(1000, "WATCHDOG: stall over after %lu ms (%u sample(s))",
                    static_cast<unsigned long>(ElapsedMs(now, episodeStart)), samples);
            inEpisode = false;
        }
        if (now - lastHeartbeat >= heartbeatMs) {
            Heartbeat("periodic", true);
            lastHeartbeat = now;
        }
    }
    LogWait(1000, "watchdog: stopped");
    HMODULE self = nullptr;
    GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                       reinterpret_cast<LPCWSTR>(&ThreadProc), &self);
    FreeLibraryAndExitThread(self, 0);  // drops the reference taken in WatchdogStart
}

}  // namespace

// ---- main-thread markers ------------------------------------------------------------------------

bool OnMainThread() { return g_mainTid != 0 && GetCurrentThreadId() == g_mainTid; }
uint32_t MainThreadId() { return g_mainTid; }
uint32_t WatchdogThreadId() { return g_watchdogTid; }

void NoteOffMainThreadCall(const char* what) {
    // Only meaningful once the main thread is known; before that every call
    // looks off-thread. One line per distinct method, then a count.
    if (g_mainTid == 0 || OnMainThread()) return;
    static volatile LONG g_logged = 0;
    static const int kMaxLines = 40;
    if (InterlockedIncrement(&g_logged) <= kMaxLines) {
        LogWait(1000, "OFF-MAIN-THREAD core read: %s on tid %lu (main tid %lu) - the game calls the clock "
                      "cross-thread; core state must stay Sync (foundations §19.2)",
                what, static_cast<unsigned long>(GetCurrentThreadId()), static_cast<unsigned long>(g_mainTid));
    }
}
bool IsLikelyReturnAddress(uintptr_t addr) { return LooksLikeReturnAddress(addr); }

namespace {
// The executable's range, computed on first use (GetModuleHandleW(nullptr)
// reads the PEB; no loader lock).
void EnsureExeRange() {
    if (g_exeBase != 0) return;
    const uintptr_t base = reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr));
    g_exeSize = ImageSizeAt(base);
    g_exeBase = base;
}
}  // namespace

ActivityScope::ActivityScope(const char* label) : tracked_(OnMainThread()) {
    if (!tracked_) return;
    const LONG d = g_actDepth;
    if (d >= 0 && d < kDepth) {
        g_actLabel[d] = label;
        g_actSince[d] = GetTickCount();
    }
    InterlockedIncrement(&g_actDepth);
}

ActivityScope::~ActivityScope() {
    if (tracked_) InterlockedDecrement(&g_actDepth);
}

TickScope::TickScope() : scope_("OnTick") {
    InterlockedIncrement(&g_ticks);
    g_lastTickMs = GetTickCount();
}

TickScope::~TickScope() { g_lastTickMs = GetTickCount(); }

void DeliveryBegin(uint32_t level, uint32_t type, const void* agent, const char* name) {
    InterlockedIncrement(&g_deliveries);
    if (!OnMainThread()) return;
    g_delLevel = level;
    g_delType = type;
    g_delAgent = agent;
    strncpy_s(g_delName, name ? name : "?", _TRUNCATE);
    g_delSince = GetTickCount();
    InterlockedExchange(&g_delActive, 1);
}

void DeliveryEnd() {
    if (OnMainThread()) InterlockedExchange(&g_delActive, 0);
}

void PublishSimState(int32_t julian, int32_t speed, int32_t pause, int32_t hidden, int32_t emergency) {
    g_julian = julian;
    g_speed = speed;
    g_pause = pause;
    g_hidden = hidden;
    g_emergency = emergency;
}

void SetCityRunning(bool running) {
    if (running) g_lastTickMs = GetTickCount();
    InterlockedExchange(&g_cityRunning, running ? 1 : 0);
}

void NoteFrameworkTick() { g_lastTickMs = GetTickCount(); }

void PublishJulian(int32_t julian) {
    if (julian != 0) g_julian = julian;
}

long PublishedJulian() { return g_julian; }
long ForeignExceptionCount() { return g_foreign; }
long WatchdogStallCount() { return g_stallEpisodes; }
uint32_t LastTickMs() { return g_lastTickMs; }

void NoteSuppressed() { InterlockedIncrement(&g_suppressed); }
void NoteInterfaceCall() { InterlockedIncrement(&g_ifaceCalls); }
void NoteForeignException() { InterlockedIncrement(&g_foreign); }

void HeartbeatNow(const char* why) { Heartbeat(why, false); }

void DescribeCurrentActivity(char* out, unsigned cap) { DescribeActivity(GetTickCount(), out, cap); }

bool InMainExe(uintptr_t addr) {
    EnsureExeRange();
    return InRange(addr, g_exeBase, g_exeSize);
}

bool DescribeAddress(uintptr_t addr, char* out, unsigned cap) {
    EnsureExeRange();
    const char* name = nullptr;
    uintptr_t base = 0;
    if (InRange(addr, ImageBase(), ImageSize())) {
        name = "FreeCity4.dll";
        base = ImageBase();
    } else {
        const LONG a = g_modActive;
        const LONG n = g_modCount[a];
        for (LONG i = 0; i < n; ++i) {
            const Module& m = g_mods[a][i];
            if (InRange(addr, m.base, m.size)) {
                name = m.name;
                base = m.base;
                break;
            }
        }
        if (name == nullptr && InRange(addr, g_exeBase, g_exeSize)) {
            name = "(executable)";
            base = g_exeBase;
        }
    }
    if (name == nullptr) {
        _snprintf_s(out, cap, _TRUNCATE, "0x%08lX (no module)", static_cast<unsigned long>(addr));
        return false;
    }
    _snprintf_s(out, cap, _TRUNCATE, "%s+0x%lX", name, static_cast<unsigned long>(addr - base));
    return true;
}

// ---- lifetime ------------------------------------------------------------------------------------

void WatchdogStart() {
    if (InterlockedCompareExchange(&g_started, 1, 0) != 0) return;
    if (g_mainThread != nullptr) CloseHandle(g_mainThread);
    g_mainThread = OpenThread(THREAD_SUSPEND_RESUME | THREAD_GET_CONTEXT | THREAD_QUERY_INFORMATION, FALSE,
                              GetCurrentThreadId());
    const NT_TIB* tib = reinterpret_cast<const NT_TIB*>(NtCurrentTeb());
    g_mainStackBase = reinterpret_cast<uintptr_t>(tib->StackBase);
    g_mainStackLimit = reinterpret_cast<uintptr_t>(tib->StackLimit);
    g_mainTid = GetCurrentThreadId();
    EnsureExeRange();
    RefreshModules();
    if (GetEnvironmentVariableA("FC4_NO_WATCHDOG", nullptr, 0) != 0) {
        Log("watchdog: disabled by FC4_NO_WATCHDOG (markers still recorded)");
        return;
    }
    // The thread holds its own reference to this DLL, so the DLL cannot be
    // unloaded under it; it drops it with FreeLibraryAndExitThread.
    HMODULE self = nullptr;
    if (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, reinterpret_cast<LPCWSTR>(&WatchdogStart), &self)) {
        Log("watchdog: could not pin the module (error %lu); not started", GetLastError());
        return;
    }
    g_stop = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    g_thread = g_stop ? CreateThread(nullptr, 64 * 1024, &ThreadProc, nullptr, 0, nullptr) : nullptr;
    if (g_thread == nullptr) {
        Log("watchdog: thread creation failed (error %lu)", GetLastError());
        FreeLibrary(self);
        return;
    }
    Log("watchdog: started for main thread %lu (stack 0x%08lX..0x%08lX, executable 0x%08lX+0x%lX)",
        static_cast<unsigned long>(g_mainTid), static_cast<unsigned long>(g_mainStackLimit),
        static_cast<unsigned long>(g_mainStackBase), static_cast<unsigned long>(g_exeBase),
        static_cast<unsigned long>(g_exeSize));
}

void WatchdogStop() {
    if (g_thread == nullptr) return;
    SetEvent(g_stop);
    if (WaitForSingleObject(g_thread, 3000) != WAIT_OBJECT_0) Log("watchdog: did not stop within 3 s");
    CloseHandle(g_thread);
    g_thread = nullptr;
    CloseHandle(g_stop);
    g_stop = nullptr;
    InterlockedExchange(&g_started, 0);
}

}  // namespace fc4
