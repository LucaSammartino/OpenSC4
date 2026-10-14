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

// The growth probe's counting hook (tooling only; building-development.md
// §12). It replaces ONE vtable entry, slot 5 (+0x14) of a cISC4TractDeveloper
// vtable, `int32 Grow(GrowthProfile const&, std::list<Outcome>&, bool&)`
// (tract-developer.md §7.1: __thiscall, three dword arguments, callee-pop 12,
// the built capacity in EAX), with a forwarder that
//   - reads QueryPerformanceCounter and rdtsc before and after the call,
//   - calls the saved original with the same this, EDX and arguments,
//   - counts the call and adds the two elapsed times to running totals,
//   - runs the optional exit check (the probe sets the x87 leftover check
//     there in verbose mode), and
//   - returns the original's EAX unchanged.
// No floating-point work in the forwarder (integer counters only; the x87
// state is never touched), no game call, no allocation, no log line.
//
// Install and remove take the vtable address as a parameter, so a test can
// patch a fake vtable in its own process. Win32 only; no other dependency
// (tests/growth_probe_test.cpp compiles this file on its own).
//
// Threading: the original calls Grow only from the BDS's day agent, on the
// main thread (building-development.md §4.4); install, remove and the totals'
// readers run there too. The totals are plain integers (a 64-bit atomic on
// x86 may be compiled to x87 loads and stores).
#pragma once

#include <cstdint>

namespace fc4 {

// cISC4TractDeveloper::Grow (tract-developer.md §7.1).
constexpr unsigned kTractGrowSlot = 5;

enum class GrowthHookStatus : int {
    Installed = 0,     // install: the entry now holds the forwarder
    Removed,           // remove: the entry holds the original again
    AlreadyInstalled,  // install: a hook is in place (one at a time)
    NotInstalled,      // remove: nothing to remove
    NullVtable,        // install: no vtable
    Unreadable,        // install: the entry is not committed, readable memory
    UnexpectedEntry,   // install: the entry does not hold the expected original (nothing written)
    EntryChanged,      // remove: the entry no longer holds the forwarder (someone patched over it; left alone)
    ProtectFailed,     // VirtualProtect refused (error = GetLastError); nothing written
};

const char* GrowthHookStatusName(GrowthHookStatus s);

// Running totals since the process started (never reset; readers take
// differences).
struct GrowthHookTotals {
    uint64_t calls = 0;
    uint64_t qpcTicks = 0;   // sum of QPC(after) - QPC(before) over the calls
    uint64_t tscCycles = 0;  // sum of rdtsc(after) - rdtsc(before)
    int32_t lastResult = 0;  // the EAX of the last call
};

// What install / remove saw and did.
struct GrowthHookReport {
    uintptr_t entryAddress = 0;  // &vtable[kTractGrowSlot]
    uintptr_t found = 0;         // the entry's value before the call
    uintptr_t written = 0;       // the value written (0 = nothing written)
    unsigned long protectBefore = 0;  // the page protection found (and restored)
    unsigned long error = 0;          // GetLastError of a failed VirtualProtect
    bool protectRestored = false;     // the second VirtualProtect (back to protectBefore) succeeded
};

// Patches vtable[kTractGrowSlot] if it holds `expectedOriginal`: makes the
// one entry writable (keeping the execute bit if the page has it), saves the
// original, writes the forwarder, restores the page protection.
GrowthHookStatus InstallGrowthHook(void** vtable, uintptr_t expectedOriginal, GrowthHookReport* report);
// Writes the saved original back (same protection dance) if the entry still
// holds the forwarder. The totals are kept.
GrowthHookStatus RemoveGrowthHook(GrowthHookReport* report);

bool GrowthHookInstalled();
void** GrowthHookVtable();      // the patched vtable, or null
uintptr_t GrowthHookOriginal();  // the saved original (0 when not installed)
uintptr_t GrowthHookAddress();   // the forwarder

GrowthHookTotals GrowthHookRead();
// The longest single forwarded call since the last take, in QPC ticks; resets it.
uint64_t GrowthHookTakeMaxTicks();

// Runs after every forwarded call, before the return (null = none). Must not
// change the x87 state or EAX's meaning (the forwarder keeps the result).
using GrowthHookExitFn = void (*)();
void SetGrowthHookExitCheck(GrowthHookExitFn fn);

}  // namespace fc4
