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

#include "FC4GrowthProbeHook.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <intrin.h>

namespace fc4 {
namespace {

// The game's __thiscall Grow, called as __fastcall with EDX passed through:
// ECX = this, three stack arguments popped by the callee (12 bytes), the
// result in EAX (tract-developer.md §7.1; docs/abi/README.md: eax-or-void,
// an int32 here, never ST0). The same trick as FC4GameMap.h's direct calls.
using GrowFn = int32_t(__fastcall*)(void* self, void* edx, const void* profile, void* outcomes, void* flag);

GrowFn g_original = nullptr;
void** g_vtable = nullptr;
bool g_installed = false;
GrowthHookExitFn g_exitCheck = nullptr;

uint64_t g_calls = 0;
uint64_t g_qpcTicks = 0;
uint64_t g_tscCycles = 0;
uint64_t g_maxTicks = 0;
int32_t g_lastResult = 0;

// The forwarder: a __thiscall callee seen from the game (ECX = this, EDX
// ignored and passed on, 12 bytes popped, EAX = the original's result).
// Integer work only.
int32_t __fastcall GrowForwarder(void* self, void* edx, const void* profile, void* outcomes, void* flag) {
    LARGE_INTEGER q0;
    QueryPerformanceCounter(&q0);
    const uint64_t c0 = __rdtsc();
    const int32_t built = g_original(self, edx, profile, outcomes, flag);
    const uint64_t c1 = __rdtsc();
    LARGE_INTEGER q1;
    QueryPerformanceCounter(&q1);
    const uint64_t ticks = static_cast<uint64_t>(q1.QuadPart - q0.QuadPart);
    g_calls += 1;
    g_qpcTicks += ticks;
    g_tscCycles += c1 - c0;
    if (ticks > g_maxTicks) g_maxTicks = ticks;
    g_lastResult = built;
    const GrowthHookExitFn exitCheck = g_exitCheck;
    if (exitCheck != nullptr) exitCheck();
    return built;
}

bool Executable(DWORD protect) {
    const DWORD base = protect & 0xFF;
    return base == PAGE_EXECUTE || base == PAGE_EXECUTE_READ || base == PAGE_EXECUTE_READWRITE ||
           base == PAGE_EXECUTE_WRITECOPY;
}

// Whether `entry` is committed memory that can be read; its protection.
bool Readable(void** entry, DWORD* protect) {
    MEMORY_BASIC_INFORMATION mbi{};
    if (VirtualQuery(entry, &mbi, sizeof(mbi)) != sizeof(mbi)) return false;
    *protect = mbi.Protect;
    return mbi.State == MEM_COMMIT && mbi.Protect != 0 && (mbi.Protect & (PAGE_GUARD | PAGE_NOACCESS)) == 0;
}

// Writes `value` into the one entry: writable for the store only (the
// execute bit kept, so code sharing the page keeps running), then the page's
// protection back.
bool WriteEntry(void** entry, uintptr_t value, GrowthHookReport* r) {
    DWORD now = 0;
    if (!Readable(entry, &now)) {
        r->error = ERROR_INVALID_ADDRESS;
        return false;
    }
    const DWORD writable = Executable(now) ? PAGE_EXECUTE_READWRITE : PAGE_READWRITE;
    DWORD before = 0;
    if (!VirtualProtect(entry, sizeof(void*), writable, &before)) {
        r->error = GetLastError();
        return false;
    }
    r->protectBefore = before;
    InterlockedExchange(reinterpret_cast<volatile LONG*>(entry), static_cast<LONG>(value));
    r->written = value;
    DWORD ignored = 0;
    r->protectRestored = VirtualProtect(entry, sizeof(void*), before, &ignored) != 0;
    if (!r->protectRestored) r->error = GetLastError();
    return true;
}

}  // namespace

const char* GrowthHookStatusName(GrowthHookStatus s) {
    switch (s) {
        case GrowthHookStatus::Installed: return "installed";
        case GrowthHookStatus::Removed: return "removed";
        case GrowthHookStatus::AlreadyInstalled: return "already installed";
        case GrowthHookStatus::NotInstalled: return "not installed";
        case GrowthHookStatus::NullVtable: return "no vtable";
        case GrowthHookStatus::Unreadable: return "the entry is not readable, aligned memory";
        case GrowthHookStatus::UnexpectedEntry: return "the entry does not hold the expected original";
        case GrowthHookStatus::EntryChanged: return "the entry no longer holds the forwarder";
        case GrowthHookStatus::ProtectFailed: return "VirtualProtect failed";
    }
    return "?";
}

GrowthHookStatus InstallGrowthHook(void** vtable, uintptr_t expectedOriginal, GrowthHookReport* report) {
    GrowthHookReport local;
    GrowthHookReport* r = report != nullptr ? report : &local;
    *r = GrowthHookReport{};
    if (g_installed) return GrowthHookStatus::AlreadyInstalled;
    if (vtable == nullptr) return GrowthHookStatus::NullVtable;
    void** entry = vtable + kTractGrowSlot;
    r->entryAddress = reinterpret_cast<uintptr_t>(entry);
    DWORD protect = 0;
    if (r->entryAddress % sizeof(void*) != 0 || !Readable(entry, &protect)) return GrowthHookStatus::Unreadable;
    r->protectBefore = protect;
    const uintptr_t current = reinterpret_cast<uintptr_t>(*static_cast<void* volatile*>(entry));
    r->found = current;
    if (current != expectedOriginal || current == 0) return GrowthHookStatus::UnexpectedEntry;
    g_original = reinterpret_cast<GrowFn>(current);
    if (!WriteEntry(entry, reinterpret_cast<uintptr_t>(&GrowForwarder), r)) return GrowthHookStatus::ProtectFailed;
    g_vtable = vtable;
    g_installed = true;
    return GrowthHookStatus::Installed;
}

GrowthHookStatus RemoveGrowthHook(GrowthHookReport* report) {
    GrowthHookReport local;
    GrowthHookReport* r = report != nullptr ? report : &local;
    *r = GrowthHookReport{};
    if (!g_installed) return GrowthHookStatus::NotInstalled;
    void** entry = g_vtable + kTractGrowSlot;
    r->entryAddress = reinterpret_cast<uintptr_t>(entry);
    const uintptr_t current = reinterpret_cast<uintptr_t>(*static_cast<void* volatile*>(entry));
    r->found = current;
    // Someone patched over the forwarder: writing the original would drop
    // their hook, so the entry is left alone (and g_original kept, so a chain
    // that ends in the forwarder still reaches the game while we are loaded).
    if (current != reinterpret_cast<uintptr_t>(&GrowForwarder)) return GrowthHookStatus::EntryChanged;
    if (!WriteEntry(entry, reinterpret_cast<uintptr_t>(g_original), r)) return GrowthHookStatus::ProtectFailed;
    g_installed = false;
    g_vtable = nullptr;
    // g_original stays: the forwarder still forwards if anyone calls it late.
    return GrowthHookStatus::Removed;
}

bool GrowthHookInstalled() { return g_installed; }
void** GrowthHookVtable() { return g_installed ? g_vtable : nullptr; }
uintptr_t GrowthHookOriginal() { return g_installed ? reinterpret_cast<uintptr_t>(g_original) : 0; }
uintptr_t GrowthHookAddress() { return reinterpret_cast<uintptr_t>(&GrowForwarder); }

GrowthHookTotals GrowthHookRead() {
    GrowthHookTotals t;
    t.calls = g_calls;
    t.qpcTicks = g_qpcTicks;
    t.tscCycles = g_tscCycles;
    t.lastResult = g_lastResult;
    return t;
}

uint64_t GrowthHookTakeMaxTicks() {
    const uint64_t m = g_maxTicks;
    g_maxTicks = 0;
    return m;
}

void SetGrowthHookExitCheck(GrowthHookExitFn fn) { g_exitCheck = fn; }

}  // namespace fc4
