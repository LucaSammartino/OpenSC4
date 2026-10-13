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

// Version-specific interop with the host executable. This is the ONLY place
// that knows fixed addresses inside a particular SimCity 4 build; every entry
// comes from docs/specs or docs/abi (never from inspecting the executable). Anything here
// is used only after the running executable has been positively identified.
#pragma once

#include <cstdint>

namespace fc4 {
namespace compat {

// Non-virtual game functions called by address (docs/abi/direct-calls.md).
enum class Direct : unsigned {
    PoleRemoveConnection,  // cSC4PowerPoleOccupant::RemoveConnection: thiscall (other base, bool), pop 8
    PipeSetWateredState,   // cSC4PipeOccupant::SetWateredState: thiscall (bool), pop 4
    AirDispatchInit,       // air dispatch source Init: thiscall (), pop 0, AL
    AirDispatchShutdown,   // air dispatch source Shutdown: thiscall (), pop 0, AL
    Count
};

// One supported executable and its interop addresses (spec §5.5).
struct ExecutableVersion {
    const char* name;
    // Identification (main module PE headers, as loaded in memory).
    uint32_t timeDateStamp;
    uint32_t sizeOfImage;
    uint32_t imageBase;
    // CPU-cycle limit-timer coefficients (32-bit floats, virtual addresses).
    uint32_t cyclesToMillisecsVA;
    uint32_t millisecsToCyclesVA;
    // The STL node allocator instance (docs/abi/allocators.md): cdecl
    // `void* (uint32_t size)` / `void (void*)`.
    uint32_t stlNodeAllocateVA;
    uint32_t stlNodeFreeVA;
    // The game's cRZString vtable (docs/abi/allocators.md, Strings).
    uint32_t rzStringVtableVA;
    // The RZ framework's "set FPU precision" helper (foundations §18.1-18.2):
    // cdecl, one int mode, no return value, the caller pops.
    uint32_t fpuSetPrecisionVA;
    // Direct-call targets, indexed by Direct (docs/abi/direct-calls.md).
    uint32_t directVA[static_cast<unsigned>(Direct::Count)];
};

// The running executable if it is a table entry loaded at its preferred base
// (once per process, logged), else nullptr.
const ExecutableVersion* IdentifiedVersion();

using StlNodeAllocateFn = void*(__cdecl*)(uint32_t size);
using StlNodeFreeFn = void(__cdecl*)(void* block);

// The RZ framework's "set FPU precision" helper (foundations §18.1-18.2):
// clears the PC field of the control word, then sets bit 1 for mode 1
// (53-bit) or bits 0-1 for mode 2 (64-bit), so mode 0 means 24-bit. Cdecl,
// one int argument, no return value, and the callee's plain `ret` leaves the
// caller to pop - every game call site is `push 1; call; add esp,4`. A
// `__stdcall`/`__thiscall` declaration would corrupt the stack.
using FpuSetPrecisionFn = void(__cdecl*)(int mode);

// The game's set-FPU-precision helper, or false (logged once) if the
// executable is not identified or the address is not executable code inside
// its image. Never guessed. `mode` comes from FC4FpuLogic.h (0 = 24-bit).
bool FpuSetPrecision(FpuSetPrecisionFn* setPrecision);

// The game's STL node allocator entry points (docs/abi/allocators.md), or
// false (logged once) if the executable is not identified or the addresses
// are not executable code inside its image. Never guessed.
bool StlNodeAllocator(StlNodeAllocateFn* allocate, StlNodeFreeFn* release);

// The game's cRZString vtable, or nullptr (logged once) if the executable is
// not identified or the address is not read-only data inside its image whose
// first entry is code inside the image.
const void* GameStringVtable();

// The address of a direct-call target, or 0 (logged once per target) if the
// executable is not identified or the address is not executable code inside
// its image. Never guessed.
uintptr_t DirectCall(Direct which);

// The compatibility table.
const ExecutableVersion* SupportedVersions(int* count);

// Identifies the running executable and validates the limit-timer globals
// (once per process; every check and its outcome is logged). Returns true if
// writes are allowed.
bool LimitTimersAvailable();

// Writes CyclesToMillisecs / MillisecsToCycles if LimitTimersAvailable() and
// the values currently stored there are still plausible. Returns true if the
// values were written. Never writes otherwise (logs a warning once).
bool PublishLimitTimers(float cyclesToMs, float msToCycles);

// Number of successful publishes (diagnostics).
uint64_t PublishCount();

}  // namespace compat
}  // namespace fc4
