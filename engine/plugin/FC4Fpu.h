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

// The main thread's floating-point control state (tooling; foundations
// §18.2). The game sets 24-bit x87 precision at the start of PreFrameWorkInit
// and the whole simulation depends on it, so FreeCity4 never changes the main
// thread's x87 control word, and logs it:
//   fpu <where>: x87 CW=0x007F (PC 24-bit, RC nearest) MXCSR=0x00001F80 [thread …]
// at DLL attach, at every framework hook, at city load, at each observe month
// line, inside our budget settlement, and on any tick where it changed.
// Always on: the first time the main thread's control word differs from the
// reference (the value seen at PreAppInit, after the game's PreFrameWorkInit),
// or is not at 24-bit precision once the reference is taken, one WARNING line.
#pragma once

#include <cstdint>

namespace fc4 {

uint16_t X87ControlWord();  // fnstcw (0 on non-x86 builds)
uint32_t SseMxcsr();        // _mm_getcsr

// Logs the state at `where` and runs the guard. `hook` marks the calling
// thread as the main thread (framework hooks run there).
void FpuLog(const char* where, bool hook = false);
// DllMain (process attach): the control word as the loader left it.
void FpuAttach();
// PreAppInit: the reference for the guard (logged).
void FpuSetReference(const char* where);
// Every framework tick (observer): logs a change of the control word once,
// with the tick number, and runs the guard.
void FpuTick();
// The guard alone (no line unless it fires).
void FpuCheck(const char* where);

}  // namespace fc4
