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

#include "FC4Schedule.h"

#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4Seed.h"
#include "fc4.h"

#include "cIGZFrameWork.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <intrin.h>
#include <tlhelp32.h>

#include <cctype>
#include <cstdlib>
#include <cstring>
#include <cwchar>
#include <functional>
#include <map>
#include <queue>
#include <string>
#include <unordered_map>
#include <vector>

#pragma intrinsic(_ReturnAddress)
#pragma intrinsic(_AddressOfReturnAddress)

extern "C" IMAGE_DOS_HEADER __ImageBase;  // this DLL's load address

#ifndef FC4_SCHEDULE_CODE
#define FC4_SCHEDULE_CODE "unknown"
#endif

namespace {

// A hash of this file and FC4Schedule.h (engine/CMakeLists.txt); tools/golden.py reads it from the DLL file.
const char kScheduleCode[] = "FC4_SCHEDULE_CODE=" FC4_SCHEDULE_CODE;

// ---- addresses in the 1.1.641 executable (an interop table; checked by Supported) ----------------
constexpr uintptr_t kLimitTimerStart = 0x0068BF50;    // small-owners-b9.md §4
constexpr uintptr_t kLimitTimerExpired = 0x0068BF80;  // small-owners-b9.md §4
constexpr uintptr_t kGrowExpiredReturn = 0x0069B149;  // the call at 0x0069B144 in Grow (building-development.md §4.4)
constexpr uintptr_t kRemainingMs = 0x0070E2B0;        // traffic: a timer's remaining ms, for the path finder's budget
constexpr uintptr_t kCrtTime = 0x009EFB59;            // time(time_t*), 32-bit time_t
constexpr uintptr_t kCrtClock = 0x009F361F;           // clock()
constexpr uintptr_t kCrtSrand = 0x009EFB92;           // srand(unsigned), the calling thread's state (foundations §12)
constexpr uintptr_t kCrtGetPtd = 0x009F7604;          // _getptd(): the thread's CRT data; rand()'s state at +0x14
constexpr uintptr_t kRandomFirst = 0x00913600;        // cRZRandom / cRZRandomSuper code (foundations §6.1)
constexpr uintptr_t kRandomEnd = 0x00913C00;
constexpr uintptr_t kRandomCtor = 0x00913A5C;         // cRZRandom::cRZRandom(seed)
constexpr uintptr_t kRandomSuperCtor = 0x00913B74;    // cRZRandomSuper::cRZRandomSuper(seed)
constexpr uintptr_t kWriteRef = 0x005D4810;           // the serializer's reference writer (foundations §2.7)
constexpr uintptr_t kRegisterObject = 0x005D43F0;     // its registration (object, id, class)
constexpr uintptr_t kQpcSlot = 0x00A802F0;            // the import slot of QueryPerformanceCounter (cRZTimer, timers.md)
constexpr uintptr_t kSleepSlot = 0x00A800C8;          // Sleep
constexpr uintptr_t kTempPathASlot = 0x00A8029C;      // the import slot of GetTempPathA
constexpr uintptr_t kTempPathW = 0x00B62904;          // cRZFile's GetTempPathW, from GetProcAddress at start-up
constexpr uintptr_t kForeground = 0x005F85F0;         // "the game is the foreground window", asked by the save's thumbnail
const uint8_t kForegroundBytes[] = {0x56, 0xFF, 0x15, 0x0C, 0x05, 0xA8, 0x00};  // push esi; call [GetForegroundWindow]
constexpr uintptr_t kSleepExSlot = 0x00A802F8;        // SleepEx (cRZThread::SleepCurrentThread)
constexpr uintptr_t kTimerOverhead = 0x00B54358;      // cRZTimer's calibrated call overhead o, u64 (timers.md §1.5)
constexpr uintptr_t kMallocEntry = 0x009F066E;          // the C runtime heap (statically linked)
constexpr uintptr_t kFreeEntry = 0x009F015B;
constexpr uintptr_t kReallocEntry = 0x009F2329;
constexpr uintptr_t kCallocEntry = 0x009FE846;
constexpr uintptr_t kMsizeEntry = 0x009F6CAB;
constexpr uintptr_t kFastAllocate = 0x005B9FC0;       // the framework allocator's Allocate(n) (framework-misc2.md §2.1)
constexpr uintptr_t kProcessHeapAlloc = 0x0090CF72;   // an allocator back-end: HeapAlloc(GetProcessHeap(), 0, n)
constexpr uintptr_t kProcessHeapFree = 0x0090CF86;    // its HeapFree
constexpr uintptr_t kChunkAlloc = 0x0090D089;         // a back-end: VirtualAlloc of a 512 KB chunk
constexpr uintptr_t kChunkFree = 0x0090D0A0;          // its VirtualFree(second argument)
constexpr uintptr_t kBlockAlloc = 0x0090D8E0;         // the object pools' page source: 2^shift bytes, so aligned
constexpr uintptr_t kBlockFree = 0x0090D91C;          // its free(p, shift)

// The bytes each hooked entry must hold before we write it.
const uint8_t kStartBytes[] = {0x53, 0x56, 0x57, 0x8B, 0xF1, 0x0F, 0x31};
const uint8_t kExpiredBytes[] = {0x56, 0x0F, 0x31, 0x8B, 0x31};
const uint8_t kRemainingBytes[] = {0x83, 0xEC, 0x08, 0x56, 0x0F, 0x31};
const uint8_t kTimeBytes[] = {0x55, 0x8B, 0xEC, 0x51, 0x51};
const uint8_t kClockBytes[] = {0x55, 0x8B, 0xEC, 0x51, 0x51};
const uint8_t kWriteRefBytes[] = {0x8B, 0x41, 0x2C, 0x85, 0xC0};
// The heap entries are copied whole into trampolines (no relative operands).
const uint8_t kMallocBytes[] = {0xFF, 0x35, 0x40, 0xD9, 0xBA, 0x00};
const uint8_t kFreeBytes[] = {0x6A, 0x0C, 0x68, 0x60, 0x63, 0xAE, 0x00};
const uint8_t kReallocBytes[] = {0x6A, 0x14, 0x68, 0x80, 0x64, 0xAE, 0x00};
const uint8_t kCallocBytes[] = {0x6A, 0x10, 0x68, 0x88, 0xBA, 0xAE, 0x00};
const uint8_t kMsizeBytes[] = {0x6A, 0x10, 0x68, 0xA0, 0x68, 0xAE, 0x00};
const uint8_t kFastAllocateBytes[] = {0x56, 0x57, 0x8B, 0x7C, 0x24, 0x0C};
const uint8_t kProcessHeapBytes[] = {0xFF, 0x74, 0x24, 0x04, 0x6A, 0x00};
const uint8_t kChunkAllocBytes[] = {0x6A, 0x04, 0x68, 0x00, 0x10, 0x00, 0x00};
const uint8_t kChunkFreeBytes[] = {0x68, 0x00, 0x80, 0x00, 0x00};
const uint8_t kBlockAllocBytes[] = {0x56, 0x57, 0xBF, 0xFC, 0x25, 0xB6, 0x00};
const uint8_t kBlockFreeBytes[] = {0x55, 0x8B, 0xEC, 0x56, 0xBE, 0xFC, 0x25, 0xB6, 0x00};

// [choice] The Grow timer expires at its 16th read: every qualifying developer
// (at most 13 with the shipped exemplars) is picked first, and 16 reads keep the
// adaptive step at 1024 (5 <= n <= 20, building-development.md §4.4).
constexpr uint32_t kGrowReads = 16;
// [choice] Any other timer expires only after 2^24 reads (none should get there:
// it would be a spin like Grow's; logged).
constexpr uint32_t kOtherReads = 1u << 24;
uint32_t g_otherReads = kOtherReads;  // `schedule limit reads` (the N and 2N check)
// A timer Start set holds this in its high dword. The others are deadlines built
// inline from rdtsc + 0x7FFFFFFFFFFFFFFF (traffic DoPostCityInit): they never expire.
constexpr uint32_t kStarted = 0x5C4ED000;
// [choice] Traffic's remaining-time query; only cSC4PathFinder::FindPath's Start
// uses the result, and Start ignores it under the schedule.
constexpr uint32_t kRemaining = 1;
// [choice] rand()'s state at the city load: derive(S, K).
constexpr uint32_t kLoadRandK = 0x4C0AD000;
// [choice] The wall clock seen by the original under the schedule.
constexpr uint32_t kTime = 1767225600;  // 2026-01-01 00:00:00 UTC
constexpr int32_t kClock = 1000;        // ms since process start
// [choice] The main thread's performance counter: +1 tick per read, and at least
// kFrameMs per frame of the simulator (cFC4Simulator::OnTick). Other threads keep the real one.
constexpr uint32_t kFrameMs = 33;
// A main-thread sleep advances that counter by its length. It really sleeps as long,
// except the save's thumbnail wait (up to 10 s for the model loader in 500 ms sleeps,
// SC4WriteCityRegionViewThumbnail), which really sleeps 1 ms.
constexpr DWORD kRealSleepMs = 1;
constexpr uintptr_t kSleepCurrentThreadReturn = 0x009205B6;  // cRZThread::SleepCurrentThread's SleepEx call
constexpr uintptr_t kThumbnailWaitReturn = 0x005DE9C2;       // its call in the thumbnail wait
// [choice] The main thread's heap from the city load on: one region at a fixed address,
// so the addresses the game orders and hashes objects by (the class store, the automata
// controller map, every pointer-keyed set) repeat. The first base that can be reserved; only
// the first repeats the hash orders (a later one is a warning). It lies below this DLL's fixed
// base (CMakeLists.txt, /BASE:0x5C000000), as a randomised DLL base moved it run to run.
constexpr uintptr_t kDllBase = 0x5C000000;
constexpr uintptr_t kArenaBases[] = {0x20000000, 0x28000000, 0x30000000, 0x38000000, 0x40000000, 0x48000000};
constexpr size_t kArenaSizes[] = {0x30000000, 0x20000000, 0x18000000};
constexpr size_t kArenaCommit = 0x100000;    // committed in 1 MB steps
constexpr size_t kSmallLimit = 4096;         // blocks up to this (with the header) in 16-byte classes
constexpr size_t kChunkAlign = 0x10000;      // VirtualAlloc's granularity, for the 512 KB chunks
constexpr uint32_t kArenaMagic = 0xFC4A110C;

struct Generator {
    uintptr_t address;
    uint32_t k;
    bool super;
};

// foundations.md §6.3 (all 34 checked against the static initialisers' "clock XOR K" in this executable).
const Generator kGenerators[] = {
    {0x00B485F0, 0x0C09C0B8, false}, {0x00B49080, 0xAC09C0E8, false}, {0x00B4AFD8, 0x4C09C113, false},
    {0x00B49AF0, 0x6C09C0F4, false}, {0x00B4B398, 0x8C09C68A, false}, {0x00B22FB0, 0x00000000, false},
    {0x00B23078, 0x8C09C2D5, false}, {0x00B230C8, 0x0C09C347, false}, {0x00B21648, 0xEC09C147, false},
    {0x00B22AE8, 0x6C09C1AB, false}, {0x00B22B60, 0xEC09C1D1, false}, {0x00B22B80, 0x4C09C187, false},
    {0x00B217C0, 0xAC357AF9, false}, {0x00B22BE8, 0xAC09C1EF, false}, {0x00B22C10, 0xAC09C20E, false},
    {0x00B22C38, 0xAC09C22B, false}, {0x00B22DF8, 0xEC09C261, false}, {0x00B22E58, 0x2C09C27B, false},
    {0x00B4BC78, 0x4C09C543, false}, {0x00B4D530, 0x8C09C57A, false}, {0x00B228A0, 0x6C09BFC3, false},
    {0x00B22E28, 0xCC09C24E, false}, {0x00B4B3D0, 0x6C09C5C8, false}, {0x00B4C7C8, 0xAC09C558, false},
    {0x00B4D5D0, 0xAC09C596, false},
    {0x00B49120, 0x0C09C461, true},  {0x00B47B78, 0x6C09C3D7, true},  {0x00B48630, 0x6C09C3F6, true},
    {0x00B49BD0, 0x4C09C4D3, true},  {0x00B4A5C0, 0x4C09C51F, true},  {0x00B44870, 0x8C09C379, true},
    {0x00B43E88, 0x6C09C07B, true},  {0x00B231B0, 0xAC09C2EC, true},  {0x00B21D78, 0x4C09C170, true},
};

// ---- state (hooks run on the game's threads; the counters are diagnostics) ----------------------
bool g_wanted = false;
bool g_fixed = false;
uint32_t g_seed = 0;
uint8_t* g_clockTrampoline = nullptr;
using QpcFn = BOOL(WINAPI*)(LARGE_INTEGER*);
QpcFn g_realQpc = nullptr;
using SleepFn = void(WINAPI*)(DWORD);
using SleepExFn = DWORD(WINAPI*)(DWORD, BOOL);
SleepFn g_realSleep = nullptr;
SleepExFn g_realSleepEx = nullptr;
int64_t g_qpcPerMs = 0;
DWORD g_mainThread = 0;
int64_t g_qpc = 0;          // the main thread's virtual counter
bool g_virtual = false;     // from the city load on (the start-up screens keep real time)
int64_t g_frameTicks = 0;   // kFrameMs in counter ticks
using ClockFn = int32_t(__cdecl*)();
using SrandFn = void(__cdecl*)(unsigned);
volatile LONG g_starts = 0, g_reads = 0, g_unstartedReads = 0, g_growExpiries = 0, g_otherExpiries = 0, g_timeCalls = 0,
              g_clockVirtual = 0, g_clockReal = 0, g_refs = 0, g_saves = 0, g_srands = 0, g_frames = 0, g_qpcVirtual = 0,
              g_qpcReal = 0, g_sleeps = 0, g_sleptMs = 0;
volatile LONG g_otherLogged = 0;
int32_t g_day = 0;
uint32_t g_dayDelivery = 0;

uint32_t Derive(uint32_t k) {
    uint32_t v = g_seed ^ k;
    if (v == 0 || v == 0xFFFFFFFFu) v ^= 1u;  // foundations §6.3
    return v;
}

bool Supported() {
    const auto base = reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr));
    if (base != 0x400000) return false;
    const auto* dos = reinterpret_cast<const IMAGE_DOS_HEADER*>(base);
    if (dos->e_magic != IMAGE_DOS_SIGNATURE || dos->e_lfanew <= 0 || dos->e_lfanew > 0x1000) return false;
    const auto* nt = reinterpret_cast<const IMAGE_NT_HEADERS32*>(base + dos->e_lfanew);
    return nt->Signature == IMAGE_NT_SIGNATURE && nt->FileHeader.TimeDateStamp == 0x4C12BFB7 &&
           nt->OptionalHeader.SizeOfImage == 0x7C8000;
}

// Other threads are suspended while an entry is rewritten, and none may be
// inside the bytes being replaced (as FC4Inplace.cpp does).
class PausedThreads {
public:
    PausedThreads(uintptr_t entry, unsigned bytes) {
        const HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if (snapshot == INVALID_HANDLE_VALUE) {
            ok_ = false;
            return;
        }
        THREADENTRY32 t{};
        t.dwSize = sizeof(t);
        for (BOOL more = Thread32First(snapshot, &t); more; more = Thread32Next(snapshot, &t)) {
            if (t.th32OwnerProcessID != GetCurrentProcessId() || t.th32ThreadID == GetCurrentThreadId()) continue;
            if (count_ == 256) {
                ok_ = false;
                break;
            }
            const HANDLE h = OpenThread(THREAD_SUSPEND_RESUME | THREAD_GET_CONTEXT, FALSE, t.th32ThreadID);
            if (h == nullptr) continue;  // a thread that has just exited
            if (SuspendThread(h) == static_cast<DWORD>(-1)) {
                CloseHandle(h);
                continue;
            }
            handles_[count_++] = h;
            CONTEXT c{};
            c.ContextFlags = CONTEXT_CONTROL;
            if (!GetThreadContext(h, &c) || (c.Eip >= entry && c.Eip < entry + bytes)) {
                ok_ = false;
                break;
            }
        }
        CloseHandle(snapshot);
    }
    ~PausedThreads() {
        for (unsigned i = count_; i > 0; --i) {
            ResumeThread(handles_[i - 1]);
            CloseHandle(handles_[i - 1]);
        }
    }
    bool ok() const { return ok_; }

private:
    HANDLE handles_[256] = {};
    unsigned count_ = 0;
    bool ok_ = true;
};

// A jump at `entry` to `target`, if the entry holds `expected`.
const char* PatchJump(uintptr_t entry, const uint8_t* expected, size_t n, const void* target) {
    auto* p = reinterpret_cast<uint8_t*>(entry);
    if (std::memcmp(p, expected, n) != 0) return "unexpected bytes";
    uint8_t jump[5] = {0xE9, 0, 0, 0, 0};
    const int32_t rel = static_cast<int32_t>(reinterpret_cast<uintptr_t>(target) - (entry + 5));
    std::memcpy(jump + 1, &rel, 4);
    PausedThreads paused(entry, 5);
    if (!paused.ok()) return "a thread is inside the entry";
    DWORD before = 0;
    if (!VirtualProtect(p, 5, PAGE_EXECUTE_READWRITE, &before)) return "VirtualProtect failed";
    std::memcpy(p, jump, 5);
    DWORD ignored = 0;
    VirtualProtect(p, 5, before, &ignored);
    FlushInstructionCache(GetCurrentProcess(), p, 5);
    return nullptr;
}

// ---- time limits --------------------------------------------------------------------------------
// LimitTimer is one u64; under the schedule it holds the reads since Start and kStarted.
void __fastcall LimitStart(uint32_t* self, void*, int32_t) {
    self[0] = 0;
    self[1] = kStarted;
    InterlockedIncrement(&g_starts);
}

uint32_t __fastcall LimitExpired(uint32_t* self) {
    if (self[1] != kStarted) {
        InterlockedIncrement(&g_unstartedReads);
        return 0;
    }
    const uint32_t reads = ++self[0];
    InterlockedIncrement(&g_reads);
    const uintptr_t site = reinterpret_cast<uintptr_t>(_ReturnAddress());
    if (site == kGrowExpiredReturn) {
        if (reads < kGrowReads) return 0;
        InterlockedIncrement(&g_growExpiries);
        return 1;
    }
    if (reads < g_otherReads) return 0;
    InterlockedIncrement(&g_otherExpiries);
    if (InterlockedIncrement(&g_otherLogged) <= 8) {
        fc4::Log("schedule: WARNING a time limit called from 0x%08X reached %u reads (a spin, or work of that "
                 "size); it expires",
                 static_cast<unsigned>(site), reads);
    }
    return 1;
}

uint32_t __fastcall RemainingMs(uint32_t*) { return kRemaining; }

// ---- timer callbacks -----------------------------------------------------------------------------
// [choice] From the city load on, the timer schedulers' callbacks (cRZCallbackTimer: the animation tick
// manager's auto-step and the effects tick it posts, tooltips, the news ticker, ...; timers.md §2.5) fire
// on the main thread at the start of each simulator frame, in each scheduler's list order, by a clock of
// kFrameMs a frame that starts at 0 with every entry due one period later. The schedulers' worker
// thread reads a clock of 0, so nothing is ever due there: no callback posts a message at a moment
// the machine chooses.
constexpr uintptr_t kTimerElapsed = 0x00890198;                  // cRZTimer::GetElapsedTime (thiscall)
constexpr uintptr_t kAddTimerNow[] = {0x0093384F, 0x0093388F};   // AddTimer's clock reads (call rel32)
constexpr uintptr_t kWorkerNow[] = {0x00933900, 0x009339AA};     // the worker loop's
constexpr uintptr_t kDefaultScheduler = 0x00B62AB0;
constexpr uint32_t kSchedulerService = 0xA76C78E7, kSchedulerServiceIid = 0x276C784B;
// A scheduler: +0x10/+0x14 entries {ID, fn, arg, one-shot (u8), period, due, fixed-rate (u8)}, +0x44 its
// lock (cRZCriticalSection: slot 1 Lock, slot 2 Unlock).
constexpr size_t kEntryWords = 7;
uint8_t* g_schedulers[2] = {};
volatile LONG g_timerOn = 0;
volatile uint32_t g_timerMs = 0;
long g_timerFires = 0;

uint32_t __fastcall SchedulerNow(void* timer, void*) {
    if (!g_timerOn) return reinterpret_cast<uint32_t(__fastcall*)(void*, void*)>(kTimerElapsed)(timer, nullptr);
    const uintptr_t site = reinterpret_cast<uintptr_t>(_ReturnAddress()) - 5;
    return site == kWorkerNow[0] || site == kWorkerNow[1] ? 0 : g_timerMs;
}

// The call at `site` to `expected` now calls `target`.
const char* PatchCall(uintptr_t site, uintptr_t expected, const void* target) {
    auto* p = reinterpret_cast<uint8_t*>(site);
    int32_t rel = 0;
    std::memcpy(&rel, p + 1, 4);
    if (p[0] != 0xE8 || site + 5 + rel != expected) return "unexpected bytes";
    rel = static_cast<int32_t>(reinterpret_cast<uintptr_t>(target) - (site + 5));
    PausedThreads paused(site, 5);
    if (!paused.ok()) return "a thread is inside the call";
    DWORD before = 0;
    if (!VirtualProtect(p, 5, PAGE_EXECUTE_READWRITE, &before)) return "VirtualProtect failed";
    std::memcpy(p + 1, &rel, 4);
    DWORD ignored = 0;
    VirtualProtect(p, 5, before, &ignored);
    FlushInstructionCache(GetCurrentProcess(), p, 5);
    return nullptr;
}

void SchedulerLock(uint8_t* scheduler, bool lock) {
    void* section = scheduler + 0x44;
    const auto* vt = *static_cast<const uintptr_t* const*>(section);
    reinterpret_cast<void(__fastcall*)(void*, void*)>(vt[lock ? 1 : 2])(section, nullptr);
}

uint32_t*& EntriesBegin(uint8_t* scheduler) { return *reinterpret_cast<uint32_t**>(scheduler + 0x10); }
uint32_t*& EntriesEnd(uint8_t* scheduler) { return *reinterpret_cast<uint32_t**>(scheduler + 0x14); }

// At the city load: the framework's scheduler and the default one, every entry due one period from 0.
bool TimersStart() {
    cIGZFrameWork* fw = RZGetFrameWork();
    void* service = nullptr;
    if (fw == nullptr || !fw->GetSystemService(kSchedulerService, kSchedulerServiceIid, &service) || service == nullptr) {
        return false;
    }
    const auto* vt = *static_cast<const uintptr_t* const*>(service);
    g_schedulers[0] = reinterpret_cast<uint8_t*(__fastcall*)(void*, void*)>(vt[3])(service, nullptr);
    reinterpret_cast<uint32_t(__fastcall*)(void*, void*)>(vt[2])(service, nullptr);  // Release
    if (g_schedulers[0] == nullptr) return false;
    if (g_schedulers[0] != reinterpret_cast<uint8_t*>(kDefaultScheduler)) {
        g_schedulers[1] = reinterpret_cast<uint8_t*>(kDefaultScheduler);
    }
    for (uint8_t* sch : g_schedulers) {
        if (sch != nullptr) SchedulerLock(sch, true);
    }
    g_timerMs = 0;
    for (uint8_t* sch : g_schedulers) {
        if (sch == nullptr) continue;
        for (uint32_t* e = EntriesBegin(sch); e != EntriesEnd(sch); e += kEntryWords) e[5] = e[4];
    }
    InterlockedExchange(&g_timerOn, 1);
    for (int i = 1; i >= 0; --i) {
        if (g_schedulers[i] != nullptr) SchedulerLock(g_schedulers[i], false);
    }
    return true;
}

// One frame: the worker loop's step (timers.md §2.5, steps 1-2) on the main thread by the frame clock.
void TimersFrame() {
    if (!g_timerOn) return;
    g_timerMs += kFrameMs;
    const uint32_t now = g_timerMs;
    for (uint8_t* sch : g_schedulers) {
        if (sch == nullptr) continue;
        struct Fire {
            uint32_t fn, arg;
        } fire[64];
        unsigned n = 0;
        SchedulerLock(sch, true);
        for (uint32_t* e = EntriesBegin(sch); e != EntriesEnd(sch);) {
            if (now < e[5]) {
                e += kEntryWords;
                continue;
            }
            if (n < 64) fire[n++] = {e[1], e[2]};
            if ((e[3] & 0xFF) != 0) {  // one-shot: erased
                uint32_t*& end = EntriesEnd(sch);
                std::memmove(e, e + kEntryWords, (end - (e + kEntryWords)) * sizeof(uint32_t));
                end -= kEntryWords;
                continue;
            }
            e[5] = (e[6] & 0xFF) != 0 ? e[5] + e[4] : now + e[4];
            if (e[5] < now) e[5] = 0xFFFFFFFFu;
            e += kEntryWords;
        }
        SchedulerLock(sch, false);
        for (unsigned i = 0; i < n; ++i) reinterpret_cast<void(__cdecl*)(uint32_t)>(fire[i].fn)(fire[i].arg);
        g_timerFires += static_cast<long>(n);
    }
}

// ---- stopwatches started before the load ---------------------------------------------------------
// [choice] A cRZTimer (timers.md §1) still running from the start-up screens reads, on the main thread,
// as started at the city load with nothing accumulated: its first read or stop there moves its start
// stamp (a real counter value, below the virtual counter's start) to the virtual counter's start.
// Otherwise the app's idle, autosave and cache timers and the animation tick's first step carry the
// start-up's real length into the run.
constexpr uintptr_t kStopwatchReads[] = {0x00890198, 0x00890279, 0x00890377, 0x008905F0};  // GetElapsedTime x3, Stop
const uint8_t kElapsedBytes[] = {0x55, 0x8B, 0xEC, 0x83, 0xEC, 0x14};
const uint8_t kStopBytes[] = {0x55, 0x8B, 0xEC, 0x51, 0x51};
int64_t g_loadQpc = 0;  // the virtual counter's start
uint8_t* g_stopwatchReal[4] = {};
long g_stopwatchesRebased = 0;

void __fastcall StopwatchRebase(uint8_t* timer) {
    if (!g_virtual || GetCurrentThreadId() != g_mainThread || *reinterpret_cast<int32_t*>(timer) == 0) return;
    auto* start = reinterpret_cast<int64_t*>(timer + 0x08);
    if (*start == 0 || *start >= g_loadQpc) return;
    *start = g_loadQpc;
    *reinterpret_cast<int64_t*>(timer + 0x10) = 0;
    ++g_stopwatchesRebased;
}

// The hooked entries (thiscall): rebase `this`, then the original.
#define FC4_STOPWATCH_THUNK(i)                     \
    __declspec(naked) void StopwatchThunk##i() {   \
        __asm push ecx                             \
        __asm call StopwatchRebase                 \
        __asm pop ecx                              \
        __asm jmp dword ptr [g_stopwatchReal + 4 * i] \
    }
FC4_STOPWATCH_THUNK(0)
FC4_STOPWATCH_THUNK(1)
FC4_STOPWATCH_THUNK(2)
FC4_STOPWATCH_THUNK(3)
#undef FC4_STOPWATCH_THUNK
void (*const kStopwatchThunks[])() = {StopwatchThunk0, StopwatchThunk1, StopwatchThunk2, StopwatchThunk3};

// ---- uninitialised values ------------------------------------------------------------------------
// [choice] A transit switch inserted for an unknown ID is all zero (traffic-network.md §4.7.3), as in
// our game: the original copies a stack temporary whose fields are whatever the stack held (pointers
// into memory outside the fixed heap among them), into its pathing costs and the save.
constexpr uintptr_t kSwitchInsert = 0x006D9131;  // the miss path of the switch table's insert-if-missing (0x006D90F0)
const uint8_t kSwitchInsertBytes[] = {0x89, 0x4C, 0x24, 0x44, 0x8D, 0x4C, 0x24, 0x10};  // mov [esp+44h],ecx; lea ecx,[esp+10h]
long g_switchesZeroed = 0;

__declspec(naked) void SwitchInsertThunk() {
    __asm {
        push eax
        push ecx
        push edi
        lea edi, [esp + 0x1C]  // the 0x34-byte temporary at [esp+10h] before the three pushes
        mov ecx, 0x0D
        xor eax, eax
        rep stosd
        pop edi
        pop ecx
        pop eax
        lock inc g_switchesZeroed
        mov dword ptr [esp + 0x44], ecx
        lea ecx, [esp + 0x10]
        push 0x006D9139  // kSwitchInsertResume
        ret
    }
}

// [choice] A new intersection's four sides and flag start at zero: the original's add-intersection
// (0x004B4300) sets only the temporary's vector and list before copying it into the network map's
// intersection table, and the sides that stay unused carry stack contents into the save.
constexpr uintptr_t kIntersectionInsert = 0x004B434D;  // push 0Ch; mov dword ptr [esp+8Ch], 0
const uint8_t kIntersectionInsertBytes[] = {0x6A, 0x0C, 0xC7, 0x84, 0x24, 0x8C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
long g_intersectionsZeroed = 0;

__declspec(naked) void IntersectionInsertThunk() {
    __asm {
        push eax
        push ecx
        push edi
        lea edi, [esp + 0x40]  // the temporary at [esp+34h] before the three pushes: sides and flag, 54h bytes
        mov ecx, 0x15
        xor eax, eax
        rep stosd
        pop edi
        pop ecx
        pop eax
        lock inc g_intersectionsZeroed
        push 0x0C
        mov dword ptr [esp + 0x8C], 0
        push 0x004B435A  // after the two replaced instructions
        ret
    }
}

// ---- wall clock ---------------------------------------------------------------------------------
uint32_t __cdecl CrtTime(uint32_t* out) {
    InterlockedIncrement(&g_timeCalls);
    if (out != nullptr) *out = kTime;
    return kTime;
}

int32_t __cdecl CrtClock() {
    const uintptr_t site = reinterpret_cast<uintptr_t>(_ReturnAddress());
    if (site >= kRandomFirst && site < kRandomEnd) {
        InterlockedIncrement(&g_clockVirtual);
        return kClock;
    }
    // [choice] Window fades (cGZWin) and the other users keep the real clock.
    InterlockedIncrement(&g_clockReal);
    return reinterpret_cast<ClockFn>(g_clockTrampoline)();
}

BOOL WINAPI VirtualQpc(LARGE_INTEGER* out) {
    if (!g_virtual || GetCurrentThreadId() != g_mainThread) {
        InterlockedIncrement(&g_qpcReal);
        return g_realQpc(out);
    }
    InterlockedIncrement(&g_qpcVirtual);
    out->QuadPart = ++g_qpc;
    return TRUE;
}

bool MainSleep(DWORD ms) {
    if (!g_virtual || GetCurrentThreadId() != g_mainThread || ms == INFINITE) return false;
    InterlockedIncrement(&g_sleeps);
    InterlockedExchangeAdd(&g_sleptMs, static_cast<LONG>(ms));
    g_qpc += static_cast<int64_t>(ms) * g_qpcPerMs;
    return true;
}

void WINAPI VirtualSleep(DWORD ms) {
    MainSleep(ms);
    g_realSleep(ms);
}

DWORD WINAPI VirtualSleepEx(DWORD ms, BOOL alertable) {
    // [ret][ms][alertable][SleepCurrentThread's return]: the thumbnail wait sleeps 1 ms.
    const auto* frame = static_cast<const uintptr_t*>(_AddressOfReturnAddress());
    const bool thumbnail = frame[0] == kSleepCurrentThreadReturn && frame[3] == kThumbnailWaitReturn;
    return g_realSleepEx(MainSleep(ms) && thumbnail && ms > kRealSleepMs ? kRealSleepMs : ms, alertable);
}

// ---- temporary files ----------------------------------------------------------------------------
// [original only] cRZFile::MakeTempPathName (0x0091A497) names a file from the temp directory and time(),
// counting down while the name exists. With time() fixed, games running side by side race for the same names
// in one directory, and the retries reach the main thread's heap. Each process has a directory of its own,
// emptied at start-up (a process ID can be reused).
wchar_t g_tempW[MAX_PATH] = {};
char g_tempA[MAX_PATH] = {};
void* g_realTempPathA = nullptr;

DWORD WINAPI TempPathA(DWORD n, LPSTR out) {
    const DWORD len = static_cast<DWORD>(std::strlen(g_tempA));
    if (out == nullptr || n <= len) return len + 1;
    std::memcpy(out, g_tempA, len + 1);
    return len;
}

DWORD WINAPI TempPathW(DWORD n, LPWSTR out) {
    const DWORD len = static_cast<DWORD>(wcslen(g_tempW));
    if (out == nullptr || n <= len) return len + 1;
    std::memcpy(out, g_tempW, (len + 1) * sizeof(wchar_t));
    return len;
}

const char* TempStart() {
    wchar_t base[MAX_PATH];
    const DWORD n = GetTempPathW(MAX_PATH, base);
    if (n == 0 || n > MAX_PATH - 16) return "no temp directory";
    swprintf(g_tempW, MAX_PATH, L"%sFC4-%08X\\", base, static_cast<unsigned>(GetCurrentProcessId()));
    if (!CreateDirectoryW(g_tempW, nullptr) && GetLastError() != ERROR_ALREADY_EXISTS) return "cannot create its directory";
    WIN32_FIND_DATAW found{};
    const HANDLE h = FindFirstFileW((std::wstring(g_tempW) + L"*").c_str(), &found);
    if (h != INVALID_HANDLE_VALUE) {
        do {
            if ((found.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) == 0) DeleteFileW((std::wstring(g_tempW) + found.cFileName).c_str());
        } while (FindNextFileW(h, &found));
        FindClose(h);
    }
    if (WideCharToMultiByte(CP_ACP, 0, g_tempW, -1, g_tempA, MAX_PATH, nullptr, nullptr) == 0) return "path not convertible";
    void** wide = reinterpret_cast<void**>(kTempPathW);
    if (*wide == nullptr) return "cRZFile's GetTempPathW is not resolved yet";
    *wide = reinterpret_cast<void*>(&TempPathW);
    return nullptr;
}

// ---- the window -----------------------------------------------------------------------------------
// [original only] A save renders the region view's thumbnail only while the game is the foreground window
// (0x005DDEC0), which with games side by side is chance; the rendering reaches the main thread's heap. The game
// is never in the foreground: every save writes the default thumbnail, after its wait (virtual sleeps).
long g_backgrounds = 0;

bool __cdecl NeverForeground() {
    InterlockedIncrement(&g_backgrounds);
    return false;
}

// [original only] A shown window is painted when other windows uncover it (games side by side minimizing at
// real-time moments), and the paint creates the screen's draw context on the main thread's heap. The game's
// windows are minimized at the city load, before that heap starts; one shown again (the game restores itself at the
// end of a load) is minimized again at the next frame and counted.
HWND g_windows[8] = {};
long g_windowCount = 0, g_minimized = 0, g_reminimized = 0;

BOOL CALLBACK MinimizeOwn(HWND h, LPARAM) {
    DWORD pid = 0;
    GetWindowThreadProcessId(h, &pid);
    if (pid == GetCurrentProcessId() && IsWindowVisible(h) && g_windowCount < 8) g_windows[g_windowCount++] = h;
    return TRUE;
}

void MinimizeWindows() {
    if (g_windowCount == 0) EnumWindows(&MinimizeOwn, 0);
    for (long k = 0; k < g_windowCount; ++k) {
        const HWND h = g_windows[k];
        if (IsWindow(h) && IsWindowVisible(h) && !IsIconic(h)) {
            ShowWindow(h, SW_MINIMIZE);
            ++(g_frames == 0 ? g_minimized : g_reminimized);
        }
    }
}

// ---- generators ---------------------------------------------------------------------------------
// The calling thread's rand() state (foundations §12.1); our cores draw from it too (fc4_crt_set_host).
uint32_t* CrtState() {
    using GetPtdFn = uint8_t*(__cdecl*)();
    return reinterpret_cast<uint32_t*>(reinterpret_cast<GetPtdFn>(kCrtGetPtd)() + 0x14);
}

void ReseedGenerators() {
    using CtorFn = void*(__fastcall*)(void* self, void*, uint32_t seed);
    for (const Generator& g : kGenerators) {
        const auto ctor = reinterpret_cast<CtorFn>(g.super ? kRandomSuperCtor : kRandomCtor);
        ctor(reinterpret_cast<void*>(g.address), nullptr, Derive(g.k));
    }
}

// The generators the original seeds from the clock (cRZRandom::Seed or cRZRandomSuper::Seed of 0xFFFFFFFF, reading
// time() and clock()) get derive(K) of foundations §6.3, as our cores seed theirs (engine/core/src/seeding.rs).
// The owner is the first return site in the table among the 16 dwords above Seed's return address (a generator's
// constructor calls Seed; the protection simulators' common constructor is called by the fire and police ones).
enum SeedKind : uint8_t {
    kClockSeed,    // K, for a seed of 0xFFFFFFFF
    kCountedSeed,  // K XOR n, n the earlier seeds at this site since the load, for a seed of 0xFFFFFFFF
    kAnySeed,      // K, whatever the seed (the traffic simulator's Init seeds from a cRZTime)
    kTimes28Seed,  // derive(K) x 28, whatever the seed (the prop developer: cRZTime x 28)
};

struct SeedSite {
    uintptr_t site;
    uint32_t k;
    SeedKind kind;
};

const SeedSite kSeedSites[] = {
    {0x0041AA02, 0x4A0DD616, kClockSeed},    // advisor system
    {0x00429A68, 0x4A1DBBBF, kCountedSeed},  // each MySim (n = its creation index)
    {0x0045DA58, 0xEAD4BA36, kCountedSeed},  // city detail manager, increase pass
    {0x0052A02F, 0x0BB14381, kClockSeed},    // city situation manager
    {0x0057ADD9, 0x69C1718F, kClockSeed},    // disaster fire instance
    {0x0057D072, 0x4A132FDF, kCountedSeed},  // pipe burst, AttemptDisasterStart
    {0x0068D2B8, 0x0990BDD8, kClockSeed},    // aura simulator
    {0x0069E02E, 0x0A413481, kCountedSeed},  // civic building simulator, monthly
    {0x006ADA65, 0x2990C05A, kClockSeed},    // fire protection simulator
    {0x006E1F95, 0x6990C04F, kClockSeed},    // police simulator
    {0x006B193A, 0xC990C034, kClockSeed},    // flora simulator
    {0x006D0628, 0xC990C0A4, kClockSeed},    // neighbours' deal propositions (process-wide)
    {0x006D585A, 0xAA3FE6BC, kClockSeed},    // park manager
    {0x006DF11B, 0x0990C075, kClockSeed},    // plumbing simulator (Init)
    {0x006F34B6, 0x89C48F47, kTimes28Seed},  // prop developer
    {0x007005D6, 0x4990C013, kClockSeed},    // residential simulator
    {0x0071F1A5, 0x6990C1AA, kAnySeed},      // traffic simulator (Init)
    {0x00728E58, 0x2990C1BC, kClockSeed},    // weather simulator
};
constexpr size_t kSeedSiteCount = sizeof(kSeedSites) / sizeof(kSeedSites[0]);
uint32_t g_seedCounts[kSeedSiteCount] = {};  // seeds at each site since the load
long g_seedsDerived = 0, g_seedsClock = 0;
uintptr_t g_clockSeedSites[16] = {};          // the sites of clock seeds not in the table, logged once each

uint32_t __cdecl SeedFor(const uint32_t* stack) {
    const uint32_t seed = stack[1];
    for (int i = 0; i < 16; ++i) {
        for (size_t j = 0; j < kSeedSiteCount; ++j) {
            const SeedSite& e = kSeedSites[j];
            if (stack[i] != e.site) continue;
            if (seed != 0xFFFFFFFFu && e.kind != kAnySeed && e.kind != kTimes28Seed) return seed;
            InterlockedIncrement(&g_seedsDerived);
            const uint32_t n = g_seedCounts[j]++;
            if (e.kind == kCountedSeed) return Derive(e.k ^ n);
            if (e.kind == kTimes28Seed) return Derive(e.k) * 28u;
            return Derive(e.k);
        }
    }
    if (seed == 0xFFFFFFFFu) {
        InterlockedIncrement(&g_seedsClock);
        uintptr_t site = 0;  // the first return site outside the generators' code
        for (int i = 0; i < 16 && site == 0; ++i) {
            if (stack[i] >= 0x00401000 && stack[i] < 0x00A70000 && (stack[i] < kRandomFirst || stack[i] >= kRandomEnd)) site = stack[i];
        }
        for (uintptr_t& logged : g_clockSeedSites) {
            if (logged == site) break;
            if (logged == 0) {
                logged = site;
                fc4::Log("schedule: a clock seed at 0x%08X keeps time() and clock() (not in the seed table)", static_cast<unsigned>(site));
                break;
            }
        }
    }
    return seed;
}

constexpr uintptr_t kRandomSeed = 0x00913667;  // cRZRandom::Seed
const uint8_t kRandomSeedBytes[] = {0x56, 0x8B, 0xF1, 0x8B, 0x4C, 0x24, 0x08};  // push esi; mov esi, ecx; mov ecx, [esp+8]
constexpr uintptr_t kRandomSuperSeed = 0x00913A72;  // cRZRandomSuper::Seed
const uint8_t kRandomSuperSeedBytes[] = {0x8B, 0x44, 0x24, 0x04, 0x83, 0xF8, 0xFF};  // mov eax, [esp+4]; cmp eax, -1

__declspec(naked) void RandomSeedThunk() {
    __asm {
        push ecx
        lea eax, [esp + 4]  // Seed's return address, then its seed
        push eax
        call SeedFor
        add esp, 4
        pop ecx
        mov [esp + 4], eax
        push esi
        mov esi, ecx
        mov ecx, [esp + 8]
        push 0x0091366E  // after the three replaced instructions
        ret
    }
}

__declspec(naked) void RandomSuperSeedThunk() {
    __asm {
        push ecx
        lea eax, [esp + 4]
        push eax
        call SeedFor
        add esp, 4
        pop ecx
        mov [esp + 4], eax
        mov eax, [esp + 4]
        cmp eax, -1
        push 0x00913A79  // after the two replaced instructions (push and ret keep the flags)
        ret
    }
}

// ---- object IDs ---------------------------------------------------------------------------------
struct SaveIds {
    std::unordered_map<uintptr_t, uint32_t> ids;
    uint32_t next = 1;
};
std::unordered_map<uintptr_t, SaveIds> g_saveIds;  // by serializer; main thread (saves)

uint32_t __fastcall WriteRef(uint8_t* iface, void*, void* stream, void* object) {
    using U32Fn = uint32_t(__fastcall*)(void*, void*);
    using SetU32Fn = uint8_t(__fastcall*)(void*, void*, uint32_t);
    using ErrorFn = uint8_t(__fastcall*)(void*, void*);
    using RegisterFn = uint8_t(__fastcall*)(void*, void*, void* object, uint32_t id, uint32_t clsid);
    // As the original (0x005D4810): refuse when the serializer is not writing.
    if (*reinterpret_cast<uint32_t*>(iface + 0x2C) == 0) return 0;
    uint8_t* base = iface - 8;
    if (*reinterpret_cast<uint32_t*>(base + 0x28) == 0) return 0;
    void** svt = *reinterpret_cast<void***>(stream);
    uint32_t clsid = 0, id = 0;
    if (object != nullptr) {
        void** ovt = *reinterpret_cast<void***>(object);
        clsid = reinterpret_cast<U32Fn>(ovt[5])(object, nullptr);  // GetGZCLSID
        // A serializer that has registered nothing yet starts a new numbering
        // (its +0x2C counts the objects it registered; a reused address is reset).
        SaveIds& s = g_saveIds[reinterpret_cast<uintptr_t>(base)];
        if (*reinterpret_cast<uint32_t*>(base + 0x2C) == 0 && s.next != 1) {
            s.ids.clear();
            s.next = 1;
        }
        if (s.next == 1 && s.ids.empty()) InterlockedIncrement(&g_saves);
        const auto it = s.ids.find(reinterpret_cast<uintptr_t>(object));
        if (it != s.ids.end()) {
            id = it->second;
        } else {
            id = s.next++;
            s.ids.emplace(reinterpret_cast<uintptr_t>(object), id);
        }
        InterlockedIncrement(&g_refs);
    }
    reinterpret_cast<SetU32Fn>(svt[9])(stream, nullptr, id);  // SetUint32
    if (object != nullptr) {
        reinterpret_cast<SetU32Fn>(svt[9])(stream, nullptr, clsid);
        if (!reinterpret_cast<RegisterFn>(kRegisterObject)(base, nullptr, object, id, clsid)) return 0;
    }
    if (reinterpret_cast<ErrorFn>(svt[18])(stream, nullptr)) return 0;  // GetError
    return 1;
}

// ---- heap ---------------------------------------------------------------------------------------
// From the city load on, every allocation the main thread makes through the C runtime,
// the framework allocator or its chunk back-ends comes from one region at a fixed base,
// with free lists that only the main thread touches; so the same sequence of requests
// gets the same addresses. A free block is reused lowest address first, so the order
// in which a batch of blocks is freed (a container walked in hash order) does not
// matter. Other threads keep the original heaps. A region block freed on another
// thread is never reused (its timing is not the main thread's).
struct BlockHeader {   // 16 bytes before each block (the chunks have none)
    uint32_t size;     // requested
    uint32_t span;     // header + block, rounded
    uint32_t magic;
    uint32_t reserved;
};

struct Arena {
    uint8_t* base = nullptr;
    size_t size = 0;
    size_t top = 0;
    size_t committed = 0;
    using Free = std::priority_queue<uint32_t, std::vector<uint32_t>, std::greater<uint32_t>>;  // offsets, lowest first
    Free small[kSmallLimit / 16];
    std::map<size_t, Free> large;    // by span
    Free chunks;                     // 512 KB, 64 KB aligned
    Free blocks[24];                 // the pools' pages, by shift
    std::map<uint8_t*, size_t> chunkSizes;
    uint32_t hash = 2166136261u;
    uint32_t ops = 0;
};
Arena g_arena;
bool g_heapReady = false;  // the region is reserved and every heap entry hooked
bool g_heap = false;       // from the city load on
volatile LONG g_heapLeaks = 0, g_heapFallbacks = 0, g_heapForeign = 0;

struct HeapTrampolines {
    uint8_t* malloc = nullptr;
    uint8_t* free = nullptr;
    uint8_t* realloc = nullptr;
    uint8_t* calloc = nullptr;
    uint8_t* msize = nullptr;
    uint8_t* fastAllocate = nullptr;
    uint8_t* processHeapAlloc = nullptr;
    uint8_t* processHeapFree = nullptr;
    uint8_t* chunkAlloc = nullptr;
    uint8_t* chunkFree = nullptr;
    uint8_t* blockAlloc = nullptr;
    uint8_t* blockFree = nullptr;
} g_heapReal;

using MallocFn = void*(__cdecl*)(size_t);
using FreeFn = void(__cdecl*)(void*);
using ReallocFn = void*(__cdecl*)(void*, size_t);
using CallocFn = void*(__cdecl*)(size_t, size_t);
using MsizeFn = size_t(__cdecl*)(void*);
using FastAllocateFn = void*(__fastcall*)(void*, void*, uint32_t);
using HeapFreeFn = BOOL(__cdecl*)(void*);
using ChunkAllocFn = void*(__stdcall*)(uint32_t);
using ChunkFreeFn = BOOL(__stdcall*)(uint32_t, void*);
using BlockAllocFn = void*(__cdecl*)(uint32_t);
using BlockFreeFn = void(__cdecl*)(void*, uint32_t);

bool InArena(const void* p) {
    const auto* b = static_cast<const uint8_t*>(p);
    return g_arena.base != nullptr && b >= g_arena.base && b < g_arena.base + g_arena.size;
}

bool OnArena() { return g_heap && GetCurrentThreadId() == g_mainThread; }

// Diagnostics: `heap trace = <directory>` writes every region operation of the main
// thread (op, size, offset, three callers) to heap-<pid>.bin, up to `heap trace max` operations
// (environment SC4_HEAP_TRACE / SC4_HEAP_TRACE_MAX: the golden harness drops FC4_*).
HANDLE g_traceFile = INVALID_HANDLE_VALUE;
uint32_t g_traceMax = 0;
uint32_t g_dumpAt = 0;        // `heap dump at = N`: the region's committed bytes at operation N
std::string g_traceDir;
constexpr uint32_t kTraceWords = 6;  // op, size, offset, three return addresses in the executable
uint32_t g_traceBuffer[kTraceWords * 16384];
uint32_t g_traceUsed = 0;
const uintptr_t* g_frame = nullptr;  // the hooked entry's return-address slot (main thread)

bool ReturnSite(uintptr_t v) {
    if (v < 0x00401005 || v >= 0x00A80000) return false;
    const auto* b = reinterpret_cast<const uint8_t*>(v);
    return b[-5] == 0xE8 || (b[-6] == 0xFF && ((b[-5] >> 3) & 7) == 2) || (b[-2] == 0xFF && ((b[-1] >> 3) & 7) == 2) ||
           (b[-3] == 0xFF && ((b[-2] >> 3) & 7) == 2) || (b[-7] == 0xFF && ((b[-6] >> 3) & 7) == 2);
}

void TraceFlush() {
    if (g_traceFile == INVALID_HANDLE_VALUE || g_traceUsed == 0) return;
    DWORD written = 0;
    WriteFile(g_traceFile, g_traceBuffer, g_traceUsed * 4, &written, nullptr);
    g_traceUsed = 0;
}

void ArenaDump() {
    const std::string path = g_traceDir + "/heap-" + std::to_string(GetCurrentProcessId()) + "-" +
                             std::to_string(g_arena.ops) + ".dmp";
    const HANDLE f = CreateFileA(path.c_str(), GENERIC_WRITE, FILE_SHARE_READ, nullptr, CREATE_ALWAYS, 0, nullptr);
    if (f == INVALID_HANDLE_VALUE) return;
    for (size_t at = 0; at < g_arena.top;) {
        const DWORD n = static_cast<DWORD>(g_arena.top - at < 0x1000000 ? g_arena.top - at : 0x1000000);
        DWORD written = 0;
        WriteFile(f, g_arena.base + at, n, &written, nullptr);
        at += n;
    }
    CloseHandle(f);
}

void ArenaNote(uint32_t op, uint32_t a, uint32_t b) {
    if (g_dumpAt != 0 && g_arena.ops == g_dumpAt) ArenaDump();
    if (g_traceFile != INVALID_HANDLE_VALUE && g_arena.ops < g_traceMax) {
        uint32_t* r = g_traceBuffer + g_traceUsed;
        r[0] = op;
        r[1] = a;
        r[2] = b;
        r[3] = r[4] = r[5] = 0;
        for (int i = 0, k = 3; g_frame != nullptr && i < 48 && k < 6; ++i) {
            if (ReturnSite(g_frame[i])) r[k++] = static_cast<uint32_t>(g_frame[i]);
        }
        if ((g_traceUsed += kTraceWords) == sizeof(g_traceBuffer) / 4) TraceFlush();
    }
    if (op == 'F' || op == 'c' || op == 'd') {  // frees change no address by themselves (lowest first)
        ++g_arena.ops;
        return;
    }
    uint32_t h = g_arena.hash;
    for (const uint32_t v : {op, a, b}) {
        for (int i = 0; i < 4; ++i) h = (h ^ ((v >> (8 * i)) & 0xFF)) * 16777619u;
    }
    g_arena.hash = h;
    ++g_arena.ops;
}

uint8_t* ArenaBump(size_t span, size_t align) {
    const size_t at = (g_arena.top + align - 1) & ~(align - 1);
    if (at + span > g_arena.size) return nullptr;
    if (at + span > g_arena.committed) {
        const size_t want = (at + span - g_arena.committed + kArenaCommit - 1) & ~(kArenaCommit - 1);
        const size_t grow = want < g_arena.size - g_arena.committed ? want : g_arena.size - g_arena.committed;
        if (VirtualAlloc(g_arena.base + g_arena.committed, grow, MEM_COMMIT, PAGE_READWRITE) == nullptr) return nullptr;
        g_arena.committed += grow;
    }
    g_arena.top = at + span;
    return g_arena.base + at;
}

void* ArenaAlloc(size_t n) {
    if (n > g_arena.size) return nullptr;
    size_t span = (n + sizeof(BlockHeader) + 15) & ~size_t{15};
    uint8_t* block = nullptr;
    Arena::Free* reuse = nullptr;
    if (span <= kSmallLimit) {
        reuse = &g_arena.small[span / 16 - 1];
    } else {
        span = (span + 4095) & ~size_t{4095};
        const auto it = g_arena.large.find(span);
        if (it != g_arena.large.end()) reuse = &it->second;
    }
    if (reuse != nullptr && !reuse->empty()) {
        block = g_arena.base + reuse->top();
        reuse->pop();
    }
    if (block == nullptr && (block = ArenaBump(span, 16)) == nullptr) {
        InterlockedIncrement(&g_heapFallbacks);
        return nullptr;
    }
    auto* h = reinterpret_cast<BlockHeader*>(block);
    h->size = static_cast<uint32_t>(n);
    h->span = static_cast<uint32_t>(span);
    h->magic = kArenaMagic;
    h->reserved = 0;
    ArenaNote('A', static_cast<uint32_t>(n), static_cast<uint32_t>(block - g_arena.base));
    return block + sizeof(BlockHeader);
}

BlockHeader* ArenaHeader(void* p) {
    auto* h = reinterpret_cast<BlockHeader*>(static_cast<uint8_t*>(p) - sizeof(BlockHeader));
    return h->magic == kArenaMagic ? h : nullptr;
}

// Main thread only.
void ArenaFree(void* p) {
    BlockHeader* h = ArenaHeader(p);
    if (h == nullptr) {
        InterlockedIncrement(&g_heapForeign);  // not a block start (a chunk is freed by ChunkFree)
        return;
    }
    auto* block = reinterpret_cast<uint8_t*>(h);
    ArenaNote('F', h->size, static_cast<uint32_t>(block - g_arena.base));
    const auto offset = static_cast<uint32_t>(block - g_arena.base);
    (h->span <= kSmallLimit ? g_arena.small[h->span / 16 - 1] : g_arena.large[h->span]).push(offset);
}

void* __cdecl CrtMalloc(size_t n) {
    if (OnArena()) {
        g_frame = static_cast<const uintptr_t*>(_AddressOfReturnAddress());
        if (void* p = ArenaAlloc(n)) return p;
    }
    return reinterpret_cast<MallocFn>(g_heapReal.malloc)(n);
}

void __cdecl CrtFree(void* p) {
    if (p == nullptr) return;
    if (OnArena()) g_frame = static_cast<const uintptr_t*>(_AddressOfReturnAddress());
    if (!InArena(p)) {
        reinterpret_cast<FreeFn>(g_heapReal.free)(p);
    } else if (GetCurrentThreadId() == g_mainThread) {
        ArenaFree(p);
    } else {
        InterlockedIncrement(&g_heapLeaks);
    }
}

size_t __cdecl CrtMsize(void* p) {
    if (p != nullptr && InArena(p)) {
        const BlockHeader* h = ArenaHeader(p);
        return h != nullptr ? h->size : 0;
    }
    return reinterpret_cast<MsizeFn>(g_heapReal.msize)(p);
}

void* __cdecl CrtRealloc(void* p, size_t n) {
    if (p == nullptr) return CrtMalloc(n);
    if (n == 0) {  // the C runtime frees and returns null
        CrtFree(p);
        return nullptr;
    }
    const bool main = OnArena();
    if (main) g_frame = static_cast<const uintptr_t*>(_AddressOfReturnAddress());
    if (!InArena(p) && !main) return reinterpret_cast<ReallocFn>(g_heapReal.realloc)(p, n);
    const size_t old = CrtMsize(p);
    if (InArena(p) && main) {
        BlockHeader* h = ArenaHeader(p);
        if (h != nullptr && n + sizeof(BlockHeader) <= h->span && n + sizeof(BlockHeader) + 16 > h->span) {
            ArenaNote('R', static_cast<uint32_t>(n), static_cast<uint32_t>(reinterpret_cast<uint8_t*>(h) - g_arena.base));
            h->size = static_cast<uint32_t>(n);
            return p;
        }
    }
    void* q = main ? ArenaAlloc(n) : nullptr;
    if (q == nullptr) q = reinterpret_cast<MallocFn>(g_heapReal.malloc)(n);
    if (q == nullptr) return nullptr;
    std::memcpy(q, p, old < n ? old : n);
    CrtFree(p);
    return q;
}

void* __cdecl CrtCalloc(size_t count, size_t n) {
    if (OnArena() && (count == 0 || n <= 0xFFFFFFE0u / count)) {
        g_frame = static_cast<const uintptr_t*>(_AddressOfReturnAddress());
        if (void* p = ArenaAlloc(count * n)) {
            std::memset(p, 0, count * n);
            return p;
        }
    }
    return reinterpret_cast<CallocFn>(g_heapReal.calloc)(count, n);
}

// The framework allocator: a region block carries the header of the allocator's own
// large blocks, (n + 4) | 0x80000000, so its Deallocate and Reallocate send it to free().
void* __fastcall FastAllocate(void* self, void*, uint32_t n) {
    if (OnArena()) {
        g_frame = static_cast<const uintptr_t*>(_AddressOfReturnAddress());
        if (auto* q = static_cast<uint32_t*>(ArenaAlloc(n + 4))) {
            q[0] = (n + 4) | 0x80000000u;
            return q + 1;
        }
    }
    return reinterpret_cast<FastAllocateFn>(g_heapReal.fastAllocate)(self, nullptr, n);
}

void* __cdecl ProcessHeapAlloc(size_t n) {
    if (OnArena()) {
        g_frame = static_cast<const uintptr_t*>(_AddressOfReturnAddress());
        if (void* p = ArenaAlloc(n)) return p;
    }
    return reinterpret_cast<MallocFn>(g_heapReal.processHeapAlloc)(n);
}

BOOL __cdecl ProcessHeapFree(void* p) {
    if (p == nullptr || !InArena(p)) return reinterpret_cast<HeapFreeFn>(g_heapReal.processHeapFree)(p);
    CrtFree(p);
    return TRUE;
}

constexpr size_t kChunkSize = 0x80000;

void* __stdcall ChunkAlloc(uint32_t unused) {
    if (OnArena()) {
        g_frame = static_cast<const uintptr_t*>(_AddressOfReturnAddress());
        uint8_t* chunk = nullptr;
        if (!g_arena.chunks.empty()) {
            chunk = g_arena.base + g_arena.chunks.top();
            g_arena.chunks.pop();
            std::memset(chunk, 0, kChunkSize);  // as fresh VirtualAlloc memory
        } else {
            chunk = ArenaBump(kChunkSize, kChunkAlign);
        }
        if (chunk != nullptr) {
            g_arena.chunkSizes[chunk] = kChunkSize;
            ArenaNote('C', kChunkSize, static_cast<uint32_t>(chunk - g_arena.base));
            return chunk;
        }
        InterlockedIncrement(&g_heapFallbacks);
    }
    return reinterpret_cast<ChunkAllocFn>(g_heapReal.chunkAlloc)(unused);
}

BOOL __stdcall ChunkFree(uint32_t first, void* p) {
    if (p == nullptr || !InArena(p)) return reinterpret_cast<ChunkFreeFn>(g_heapReal.chunkFree)(first, p);
    if (GetCurrentThreadId() != g_mainThread) {
        InterlockedIncrement(&g_heapLeaks);
        return TRUE;
    }
    auto* chunk = static_cast<uint8_t*>(p);
    const auto it = g_arena.chunkSizes.find(chunk);
    if (it == g_arena.chunkSizes.end()) {
        InterlockedIncrement(&g_heapForeign);
        return TRUE;
    }
    ArenaNote('c', static_cast<uint32_t>(it->second), static_cast<uint32_t>(chunk - g_arena.base));
    g_arena.chunks.push(static_cast<uint32_t>(chunk - g_arena.base));
    g_arena.chunkSizes.erase(it);
    return TRUE;
}

// The object pools' pages (2^shift bytes; a pool finds an object's page by that alignment).
void* __cdecl BlockAlloc(uint32_t shift) {
    if (OnArena() && shift < 24) {
        g_frame = static_cast<const uintptr_t*>(_AddressOfReturnAddress());
        const size_t n = size_t{1} << shift;
        Arena::Free& reuse = g_arena.blocks[shift];
        uint8_t* block = nullptr;
        if (!reuse.empty()) {
            block = g_arena.base + reuse.top();
            reuse.pop();
        } else {
            block = ArenaBump(n, n);
        }
        if (block != nullptr) {
            ArenaNote('D', static_cast<uint32_t>(n), static_cast<uint32_t>(block - g_arena.base));
            return block;
        }
        InterlockedIncrement(&g_heapFallbacks);
    }
    return reinterpret_cast<BlockAllocFn>(g_heapReal.blockAlloc)(shift);
}

void __cdecl BlockFree(void* p, uint32_t shift) {
    if (p == nullptr || !InArena(p)) {
        reinterpret_cast<BlockFreeFn>(g_heapReal.blockFree)(p, shift);
    } else if (GetCurrentThreadId() != g_mainThread || shift >= 24) {
        InterlockedIncrement(&g_heapLeaks);
    } else {
        g_frame = static_cast<const uintptr_t*>(_AddressOfReturnAddress());
        const auto offset = static_cast<uint32_t>(static_cast<uint8_t*>(p) - g_arena.base);
        ArenaNote('d', 1u << shift, offset);
        g_arena.blocks[shift].push(offset);
    }
}

// The static object pools (an instance {vtable, current page, full pages}; slot 2 makes a page;
// a page is {next, prev, free list, count}, count + 0x40000000 while full). At the city load
// every page made before it is retired (count + kRetired: never current again, never moved
// between the rings, never released), and each pool starts from one new page of the region,
// so the city's objects come from the region whatever the start-up screens left in the pools.
struct PoolSite {
    uintptr_t pool;
    uintptr_t vtable;  // 0: set by its constructor at run time (checked against the range only)
};
constexpr PoolSite kPools[] = {
    {0x00B07F1C, 0x00A87370}, {0x00B07FC8, 0x00A88644}, {0x00B09428, 0x00A8CE88}, {0x00B09588, 0x00A8FE7C},
    {0x00B098A8, 0x00A942D8}, {0x00B098B4, 0x00A942E8}, {0x00B098D0, 0x00A943E8}, {0x00B09FEC, 0x00A99E24},
    {0x00B0A018, 0x00A9A428}, {0x00B0A09C, 0x00A9C7F4}, {0x00B0A0F8, 0x00A9D194}, {0x00B0A808, 0x00AA20A0},
    {0x00B0A824, 0x00AA2870}, {0x00B0A830, 0x00AA2A08}, {0x00B0A86C, 0x00AA2CDC}, {0x00B0A884, 0x00AA2CEC},
    {0x00B0A898, 0x00AA2E94}, {0x00B0A8AC, 0x00AA30B4}, {0x00B0A8C8, 0x00AA4458}, {0x00B0A8E4, 0x00AA4960},
    {0x00B0A8F0, 0x00AA4970}, {0x00B0A8FC, 0x00AA4980}, {0x00B0A908, 0x00AA4990}, {0x00B0A914, 0x00AA49A0},
    {0x00B0A96C, 0x00AA5288}, {0x00B0A9B8, 0x00AA54E0}, {0x00B0A9E8, 0x00AA6CB4}, {0x00B0A9F4, 0x00AA6CC4},
    {0x00B0B2D0, 0x00AA7180}, {0x00B0B394, 0x00AA7F00}, {0x00B0B3A8, 0x00AA8224}, {0x00B0B570, 0x00AA9458},
    {0x00B0BEC0, 0x00AA9978}, {0x00B0C0C8, 0x00AA9D04}, {0x00B0C258, 0x00AA9D14}, {0x00B0CDB8, 0x00AAA590},
    {0x00B0D008, 0x00AAABC0}, {0x00B0D3AC, 0x00AB184C}, {0x00B0D484, 0x00AB3380}, {0x00B0DC30, 0x00000000},
    {0x00B0E330, 0x00ABD28C}, {0x00B0E33C, 0x00ABD29C}, {0x00B0E348, 0x00ABD2AC}, {0x00B0E368, 0x00ABD4D0},
    {0x00B0E388, 0x00ABD5F8},
};
constexpr uint32_t kRetired = 0x20000000;
using PageMakeFn = uint32_t*(__fastcall*)(void*);
uint32_t g_poolsRetired = 0, g_pagesRetired = 0;

// Main thread, with the region on (the new pages come from BlockAlloc).
void PoolsRetire() {
    for (const PoolSite& site : kPools) {
        auto* pool = reinterpret_cast<uint32_t*>(site.pool);
        const uint32_t vtable = pool[0];
        if ((site.vtable != 0 && vtable != site.vtable) || vtable < 0x00A80000 || vtable >= 0x00B00000 || pool[1] == 0) {
            fc4::Log("schedule: WARNING object pool at 0x%08X not restarted (vtable 0x%08X)",
                     static_cast<unsigned>(site.pool), vtable);
            continue;
        }
        for (const uint32_t head : {pool[1], pool[2]}) {
            auto* page = reinterpret_cast<uint32_t*>(head);
            while (page != nullptr) {
                page[3] += kRetired;
                ++g_pagesRetired;
                page = reinterpret_cast<uint32_t*>(page[0]);
                if (page == reinterpret_cast<uint32_t*>(head)) break;
            }
        }
        uint32_t* page = reinterpret_cast<PageMakeFn>(reinterpret_cast<const uint32_t*>(vtable)[2])(pool);
        page[0] = page[1] = static_cast<uint32_t>(reinterpret_cast<uintptr_t>(page));
        pool[1] = static_cast<uint32_t>(reinterpret_cast<uintptr_t>(page));
        pool[2] = 0;
        ++g_poolsRetired;
    }
}

// A trampoline: the entry's first n bytes (whole instructions, no relative operands), then a jump back.
uint8_t* Trampoline(uint8_t*& page, size_t& used, uintptr_t entry, size_t n) {
    if (page == nullptr) {
        page = static_cast<uint8_t*>(VirtualAlloc(nullptr, 4096, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE));
        if (page == nullptr) return nullptr;
    }
    if (used + n + 5 > 4096) return nullptr;
    uint8_t* t = page + used;
    std::memcpy(t, reinterpret_cast<const void*>(entry), n);
    t[n] = 0xE9;
    const int32_t rel = static_cast<int32_t>((entry + n) - (reinterpret_cast<uintptr_t>(t) + n + 5));
    std::memcpy(t + n + 1, &rel, 4);
    used += (n + 5 + 15) & ~size_t{15};
    return t;
}

// Reserves the region and hooks the heap entries; false (and the original heaps) if any step fails.
bool HeapInstall(const char** failure) {
    for (const size_t size : kArenaSizes) {
        for (const uintptr_t base : kArenaBases) {
            void* p = VirtualAlloc(reinterpret_cast<void*>(base), size, MEM_RESERVE, PAGE_NOACCESS);
            if (p == reinterpret_cast<void*>(base)) {
                g_arena.base = static_cast<uint8_t*>(p);
                g_arena.size = size;
                break;
            }
            if (p != nullptr) VirtualFree(p, 0, MEM_RELEASE);
        }
        if (g_arena.base != nullptr) break;
    }
    if (g_arena.base == nullptr) {
        *failure = "no fixed base could be reserved";
        return false;
    }
    struct Entry {
        uintptr_t entry;
        const uint8_t* bytes;
        size_t n;
        const void* hook;
        uint8_t** real;
    };
    const Entry entries[] = {
        {kMallocEntry, kMallocBytes, sizeof(kMallocBytes), reinterpret_cast<const void*>(&CrtMalloc), &g_heapReal.malloc},
        {kFreeEntry, kFreeBytes, sizeof(kFreeBytes), reinterpret_cast<const void*>(&CrtFree), &g_heapReal.free},
        {kReallocEntry, kReallocBytes, sizeof(kReallocBytes), reinterpret_cast<const void*>(&CrtRealloc), &g_heapReal.realloc},
        {kCallocEntry, kCallocBytes, sizeof(kCallocBytes), reinterpret_cast<const void*>(&CrtCalloc), &g_heapReal.calloc},
        {kMsizeEntry, kMsizeBytes, sizeof(kMsizeBytes), reinterpret_cast<const void*>(&CrtMsize), &g_heapReal.msize},
        {kFastAllocate, kFastAllocateBytes, sizeof(kFastAllocateBytes), reinterpret_cast<const void*>(&FastAllocate),
         &g_heapReal.fastAllocate},
        {kProcessHeapAlloc, kProcessHeapBytes, sizeof(kProcessHeapBytes), reinterpret_cast<const void*>(&ProcessHeapAlloc),
         &g_heapReal.processHeapAlloc},
        {kProcessHeapFree, kProcessHeapBytes, sizeof(kProcessHeapBytes), reinterpret_cast<const void*>(&ProcessHeapFree),
         &g_heapReal.processHeapFree},
        {kChunkAlloc, kChunkAllocBytes, sizeof(kChunkAllocBytes), reinterpret_cast<const void*>(&ChunkAlloc),
         &g_heapReal.chunkAlloc},
        {kChunkFree, kChunkFreeBytes, sizeof(kChunkFreeBytes), reinterpret_cast<const void*>(&ChunkFree), &g_heapReal.chunkFree},
        {kBlockAlloc, kBlockAllocBytes, sizeof(kBlockAllocBytes), reinterpret_cast<const void*>(&BlockAlloc),
         &g_heapReal.blockAlloc},
        {kBlockFree, kBlockFreeBytes, sizeof(kBlockFreeBytes), reinterpret_cast<const void*>(&BlockFree), &g_heapReal.blockFree},
    };
    // Every entry is checked and every trampoline built before the first jump is written.
    uint8_t* page = nullptr;
    size_t used = 0;
    for (const Entry& e : entries) {
        if (std::memcmp(reinterpret_cast<const void*>(e.entry), e.bytes, e.n) != 0) {
            *failure = "unexpected entry bytes";
            return false;
        }
        if ((*e.real = Trampoline(page, used, e.entry, e.n)) == nullptr) {
            *failure = "no trampoline page";
            return false;
        }
    }
    FlushInstructionCache(GetCurrentProcess(), page, 4096);
    for (const Entry& e : entries) {
        if (const char* err = PatchJump(e.entry, e.bytes, e.n, e.hook)) {
            *failure = err;  // the entries patched so far still call the original through their trampolines
            return false;
        }
    }
    return true;
}

std::string Lower(std::string s) {
    for (char& c : s) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    const size_t b = s.find_first_not_of(" \t"), e = s.find_last_not_of(" \t");
    return b == std::string::npos ? std::string() : s.substr(b, e - b + 1);
}

}  // namespace

namespace fc4 {

bool ScheduleFixed() { return g_fixed; }

void ScheduleStartup() {
    const char* source = "";
    const std::string v = Lower(Setting("schedule", "FC4_SCHEDULE", &source));
    g_wanted = v == "fixed";
    if (!g_wanted) {
        if (!v.empty() && v != "original") Log("schedule: WARNING schedule = '%s' is neither fixed nor original", v.c_str());
        return;
    }
    if (!Supported()) {
        Log("schedule: WARNING schedule = fixed needs the 1.1.641 executable; nothing is changed");
        return;
    }
    g_seed = SessionSeed(nullptr);
    const std::string reads = Setting("schedule limit reads", "SC4_SCHEDULE_LIMIT_READS");
    if (!reads.empty()) g_otherReads = static_cast<uint32_t>(std::strtoul(reads.c_str(), nullptr, 10));
    // The clock trampoline: clock()'s first five bytes (no relative operands), then back.
    g_clockTrampoline = static_cast<uint8_t*>(VirtualAlloc(nullptr, 16, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE));
    if (g_clockTrampoline != nullptr) {
        std::memcpy(g_clockTrampoline, kClockBytes, 5);
        g_clockTrampoline[5] = 0xE9;
        const int32_t rel = static_cast<int32_t>((kCrtClock + 5) - (reinterpret_cast<uintptr_t>(g_clockTrampoline) + 10));
        std::memcpy(g_clockTrampoline + 6, &rel, 4);
        DWORD before = 0;
        VirtualProtect(g_clockTrampoline, 16, PAGE_EXECUTE_READ, &before);
        FlushInstructionCache(GetCurrentProcess(), g_clockTrampoline, 16);
    }
    struct Hook {
        const char* name;
        uintptr_t entry;
        const uint8_t* bytes;
        size_t n;
        const void* target;
    };
    const Hook hooks[] = {
        {"LimitTimer::Start", kLimitTimerStart, kStartBytes, sizeof(kStartBytes), reinterpret_cast<const void*>(&LimitStart)},
        {"LimitTimer::Expired", kLimitTimerExpired, kExpiredBytes, sizeof(kExpiredBytes),
         reinterpret_cast<const void*>(&LimitExpired)},
        {"traffic remaining ms", kRemainingMs, kRemainingBytes, sizeof(kRemainingBytes),
         reinterpret_cast<const void*>(&RemainingMs)},
        {"time()", kCrtTime, kTimeBytes, sizeof(kTimeBytes), reinterpret_cast<const void*>(&CrtTime)},
        {"clock()", kCrtClock, kClockBytes, sizeof(kClockBytes), reinterpret_cast<const void*>(&CrtClock)},
        {"reference writer", kWriteRef, kWriteRefBytes, sizeof(kWriteRefBytes), reinterpret_cast<const void*>(&WriteRef)},
    };
    bool all = g_clockTrampoline != nullptr;
    for (const Hook& h : hooks) {
        const char* err = PatchJump(h.entry, h.bytes, h.n, h.target);
        if (err != nullptr) {
            all = false;
            Log("schedule: WARNING %s at 0x%08X not hooked: %s", h.name, static_cast<unsigned>(h.entry), err);
        }
    }
    // The performance counter: the import slot, so every cRZTimer reader on the
    // main thread sees the virtual counter; o = 0 (it was measured on the real one).
    g_mainThread = GetCurrentThreadId();
    LARGE_INTEGER f{};
    QueryPerformanceFrequency(&f);
    g_frameTicks = f.QuadPart * kFrameMs / 1000;
    g_qpcPerMs = f.QuadPart / 1000;
    const struct {
        const char* name;
        uintptr_t slot;
        void* hook;
        void** real;
    } slots[] = {
        {"QueryPerformanceCounter", kQpcSlot, reinterpret_cast<void*>(&VirtualQpc), reinterpret_cast<void**>(&g_realQpc)},
        {"Sleep", kSleepSlot, reinterpret_cast<void*>(&VirtualSleep), reinterpret_cast<void**>(&g_realSleep)},
        {"SleepEx", kSleepExSlot, reinterpret_cast<void*>(&VirtualSleepEx), reinterpret_cast<void**>(&g_realSleepEx)},
        {"GetTempPathA", kTempPathASlot, reinterpret_cast<void*>(&TempPathA), &g_realTempPathA},
    };
    if (const char* err = TempStart()) {
        all = false;
        Log("schedule: WARNING no temp directory of its own: %s", err);
    }
    for (const auto& e : slots) {
        auto* slot = reinterpret_cast<void**>(e.slot);
        DWORD before = 0;
        if (VirtualProtect(slot, 4, PAGE_READWRITE, &before)) {
            *e.real = *slot;
            *slot = e.hook;
            DWORD ignored = 0;
            VirtualProtect(slot, 4, before, &ignored);
        } else {
            all = false;
            Log("schedule: WARNING the %s import slot is not writable", e.name);
        }
    }
    *reinterpret_cast<uint64_t*>(kTimerOverhead) = 0;
    const char* heapFailure = nullptr;
    g_heapReady = HeapInstall(&heapFailure);
    if (!g_heapReady) {
        all = false;
        Log("schedule: WARNING the heap is not fixed: %s", heapFailure);
    } else if (g_arena.base != reinterpret_cast<uint8_t*>(kArenaBases[0]) || g_arena.size != kArenaSizes[0]) {
        all = false;
        Log("schedule: WARNING the heap is at 0x%08X (%u MB), not 0x%08X (%u MB): its hash orders are not the other runs'",
            static_cast<unsigned>(reinterpret_cast<uintptr_t>(g_arena.base)), static_cast<unsigned>(g_arena.size >> 20),
            static_cast<unsigned>(kArenaBases[0]), static_cast<unsigned>(kArenaSizes[0] >> 20));
    }
    if (reinterpret_cast<uintptr_t>(&__ImageBase) != kDllBase) {
        all = false;
        Log("schedule: WARNING FreeCity4.dll is at 0x%08X, not its fixed base 0x%08X: the game's pointer-keyed sets "
            "holding its objects order differently", static_cast<unsigned>(reinterpret_cast<uintptr_t>(&__ImageBase)),
            static_cast<unsigned>(kDllBase));
    }
    for (const uintptr_t site : {kAddTimerNow[0], kAddTimerNow[1], kWorkerNow[0], kWorkerNow[1]}) {
        if (const char* err = PatchCall(site, kTimerElapsed, reinterpret_cast<const void*>(&SchedulerNow))) {
            all = false;
            Log("schedule: WARNING the timer scheduler's clock read at 0x%08X not hooked: %s", static_cast<unsigned>(site), err);
        }
    }
    uint8_t* page = nullptr;
    size_t used = 0;
    for (int i = 0; i < 4; ++i) {
        const uint8_t* bytes = i < 3 ? kElapsedBytes : kStopBytes;
        const size_t n = i < 3 ? sizeof(kElapsedBytes) : sizeof(kStopBytes);
        const char* err = std::memcmp(reinterpret_cast<const void*>(kStopwatchReads[i]), bytes, n) != 0 ? "unexpected bytes"
                          : (g_stopwatchReal[i] = Trampoline(page, used, kStopwatchReads[i], n)) == nullptr ? "no trampoline"
                                                                                                         : nullptr;
        if (err == nullptr) err = PatchJump(kStopwatchReads[i], bytes, n, reinterpret_cast<const void*>(kStopwatchThunks[i]));
        if (err != nullptr) {
            all = false;
            Log("schedule: WARNING the stopwatch read at 0x%08X not hooked: %s", static_cast<unsigned>(kStopwatchReads[i]), err);
        }
    }
    if (page != nullptr) FlushInstructionCache(GetCurrentProcess(), page, 4096);
    if (const char* err = PatchJump(kSwitchInsert, kSwitchInsertBytes, sizeof(kSwitchInsertBytes),
                                    reinterpret_cast<const void*>(&SwitchInsertThunk))) {
        all = false;
        Log("schedule: WARNING the transit switch insert at 0x%08X not hooked: %s", static_cast<unsigned>(kSwitchInsert), err);
    }
    if (const char* err = PatchJump(kIntersectionInsert, kIntersectionInsertBytes, sizeof(kIntersectionInsertBytes),
                                    reinterpret_cast<const void*>(&IntersectionInsertThunk))) {
        all = false;
        Log("schedule: WARNING the intersection insert at 0x%08X not hooked: %s", static_cast<unsigned>(kIntersectionInsert), err);
    }
    if (const char* err = PatchJump(kForeground, kForegroundBytes, sizeof(kForegroundBytes),
                                    reinterpret_cast<const void*>(&NeverForeground))) {
        all = false;
        Log("schedule: WARNING the foreground test at 0x%08X not hooked: %s", static_cast<unsigned>(kForeground), err);
    }
    if (const char* err = PatchJump(kRandomSeed, kRandomSeedBytes, sizeof(kRandomSeedBytes),
                                    reinterpret_cast<const void*>(&RandomSeedThunk))) {
        all = false;
        Log("schedule: WARNING cRZRandom::Seed at 0x%08X not hooked: %s", static_cast<unsigned>(kRandomSeed), err);
    }
    if (const char* err = PatchJump(kRandomSuperSeed, kRandomSuperSeedBytes, sizeof(kRandomSuperSeedBytes),
                                    reinterpret_cast<const void*>(&RandomSuperSeedThunk))) {
        all = false;
        Log("schedule: WARNING cRZRandomSuper::Seed at 0x%08X not hooked: %s", static_cast<unsigned>(kRandomSuperSeed), err);
    }
    ReseedGenerators();
    fc4_crt_set_host(&CrtState);
    g_fixed = true;
    Log("schedule: fixed (code %s, %s; docs/specs/simulator.md 5.8): time limits count reads (Grow expires at read %u, others "
        "at %u), time() = %u, the generators' clock() = %d, %u file-local generators re-seeded with derive(S = "
        "0x%08X, K), rand() set before every agent delivery and shared with our cores, save object IDs 1, 2, 3, ..., the main thread's "
        "performance counter virtual (+%u ms a frame), temp directory %s, the main thread's heap at 0x%08X (%u MB) from "
        "the city load%s",
        kScheduleCode + 18, source, kGrowReads, g_otherReads, kTime, kClock, static_cast<unsigned>(sizeof(kGenerators) / sizeof(kGenerators[0])),
        g_seed, kFrameMs, g_tempA, static_cast<unsigned>(reinterpret_cast<uintptr_t>(g_arena.base)),
        static_cast<unsigned>(g_arena.size >> 20), all ? "" : " WARNING: some hooks failed (above)");
}

void ScheduleBeforeCityLoad() {
    if (!g_fixed) return;
    MinimizeWindows();
    std::memset(g_seedCounts, 0, sizeof(g_seedCounts));
    ReseedGenerators();
    // rand() as the load (subsystem Init shuffles: flora, weather) finds it.
    reinterpret_cast<SrandFn>(kCrtSrand)(Derive(kLoadRandK));
    // The main thread's virtual counter starts here, at the real value, so intervals
    // that began on the start-up screens stay positive.
    LARGE_INTEGER now{};
    g_realQpc(&now);
    g_qpc = g_loadQpc = now.QuadPart;
    g_virtual = true;
    g_heap = g_heapReady;
    if (g_heap) PoolsRetire();
    if (!TimersStart()) Log("schedule: WARNING the timer scheduler was not found: its callbacks stay on the worker thread");
    const std::string trace = Setting("heap trace", "SC4_HEAP_TRACE");
    if (g_heap && !trace.empty()) {
        const std::string path = trace + "/heap-" + std::to_string(GetCurrentProcessId()) + ".bin";
        g_traceFile = CreateFileA(path.c_str(), GENERIC_WRITE, FILE_SHARE_READ, nullptr, CREATE_ALWAYS, 0, nullptr);
        const std::string max = Setting("heap trace max", "SC4_HEAP_TRACE_MAX");
        g_traceMax = max.empty() ? 8000000u : static_cast<uint32_t>(std::strtoul(max.c_str(), nullptr, 10));
        g_traceDir = trace;
        g_dumpAt = static_cast<uint32_t>(std::strtoul(Setting("heap dump at", "SC4_HEAP_DUMP_AT").c_str(), nullptr, 10));
        Log("schedule: heap trace to %s (%s, up to %u operations)", path.c_str(),
            g_traceFile != INVALID_HANDLE_VALUE ? "open" : "FAILED", g_traceMax);
    }
    Log("schedule: file-local generators re-seeded, rand() set, the virtual performance counter and the fixed heap "
        "(%s; %u object pools restarted, %u earlier pages retired) started before the city load", g_heap ? "on" : "off",
        g_poolsRetired, g_pagesRetired);
}

void ScheduleDayOpened(int32_t julian) {
    g_day = julian;
    g_dayDelivery = 0;
}

void ScheduleFrame() {
    if (!g_fixed || !g_virtual) return;
    InterlockedIncrement(&g_frames);
    g_qpc += g_frameTicks;
    MinimizeWindows();
    TimersFrame();
}

uint32_t ScheduleClockMs() {
    if (!g_fixed || !g_virtual || g_qpcPerMs == 0) return 0;
    return static_cast<uint32_t>((g_qpc - g_loadQpc) / g_qpcPerMs);
}

void ScheduleBeforeDelivery(const void* target, uint32_t msgType) {
    if (!g_fixed) return;
    {
        // FNV-1a over every generator's state (cRZRandom 12 bytes, cRZRandomSuper 0x9C8).
        uint32_t h = 2166136261u;
        for (const Generator& g : kGenerators) {
            const auto* b = reinterpret_cast<const uint8_t*>(g.address);
            for (size_t i = 0, n = g.super ? 0x9C8 : 12; i < n; ++i) h = (h ^ b[i]) * 16777619u;
        }
        const uint32_t randState = *CrtState();
        Log("schedule: digest %d.%u rand 0x%08X generators 0x%08X heap %u 0x%08X target vt 0x%08X msg 0x%08X", g_day,
            g_dayDelivery, randState, h, g_arena.ops, g_arena.hash, *reinterpret_cast<const uint32_t*>(target), msgType);
    }
    // [choice] rand() state for the n-th delivery of day d: S XOR (d * 0x9E3779B1) XOR (n * 0x85EBCA77).
    const uint32_t state = g_seed ^ (static_cast<uint32_t>(g_day) * 0x9E3779B1u) ^ (g_dayDelivery * 0x85EBCA77u);
    ++g_dayDelivery;
    reinterpret_cast<SrandFn>(kCrtSrand)(state);
    InterlockedIncrement(&g_srands);
}

void ScheduleMonthLine(int32_t julian) {
    if (!g_fixed) return;
    TraceFlush();
    Log("schedule: month at julian %d: time limits started %ld, read %ld (+%ld unstarted, never expire), Grow "
        "expiries %ld, other expiries %ld; "
        "time() calls %ld; clock() virtual %ld, real %ld; performance counter virtual %ld, real (other threads) %ld; "
        "frames %ld, sleeps %ld (%ld ms); timer callbacks %ld on the main thread; stopwatches rebased %ld; transit switches zeroed %ld; intersections zeroed %ld; foreground tests %ld (all false); windows minimized %ld, again %ld; clock seeds derived %ld, kept %ld; rand() set %ld times; save references %ld in %ld save(s); heap %u operations "
        "(0x%08X), %u MB used, %u MB committed, %ld blocks freed on other threads, %ld fallbacks, %ld foreign frees",
        julian, g_starts, g_reads, g_unstartedReads, g_growExpiries, g_otherExpiries, g_timeCalls, g_clockVirtual, g_clockReal, g_qpcVirtual, g_qpcReal, g_frames, g_sleeps, g_sleptMs, g_timerFires, g_stopwatchesRebased, g_switchesZeroed, g_intersectionsZeroed, g_backgrounds, g_minimized, g_reminimized, g_seedsDerived, g_seedsClock, g_srands,
        g_refs, g_saves, g_arena.ops, g_arena.hash, static_cast<unsigned>(g_arena.top >> 20),
        static_cast<unsigned>(g_arena.committed >> 20), g_heapLeaks, g_heapFallbacks, g_heapForeign);
}

}  // namespace fc4
