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

// The DLL's test seam: one export, FC4TestSeam, that tests (which load the
// DLL without a game framework) use to see what would otherwise only reach
// the game. The game never calls it; nothing changes unless it is called.
//
//   intptr_t __cdecl FC4TestSeam(uint32_t op, void* a, void* b);
//
//   op 1  kFC4SeamBudgetPostSink   a = FC4PostSinkFn (or null), b = context.
//         Every budget message post builds the message even without a
//         message server and hands it to the sink before posting (the sink
//         must not keep the pointer).
//   op 2  kFC4SeamObserverCity     a = cISC4City* the observer uses instead
//         of asking the application (null: back to the application).
//   op 3  kFC4SeamObserverTick     the observer's OnTick.
//   op 4  kFC4SeamObserverMessage  the observer's DoMessage with a standard
//         message of type (uint32_t)a and data1 (intptr_t)b.
//   op 5  kFC4SeamObserverApp      a = cISC4App* the observer's save autotest
//         uses instead of the framework's application (null: back to it).
//   op 6  kFC4SeamPowerBudgetPath  a = cISC4BudgetSimulator* (initialised):
//         runs the power glue's department / line-item callbacks on it,
//         including a department reference read back from a save; returns
//         0, or the first failed step (logged).
//   op 7  kFC4SeamFpuReference     a = null: the calling thread's x87 control
//         word becomes the guard's reference (as PreAppInit does in the
//         game); a != null: the per-tick check alone (FpuTick).
#pragma once

#include <cstdint>

class cIGZMessage2Standard;

enum : uint32_t {
    kFC4SeamBudgetPostSink = 1,
    kFC4SeamObserverCity = 2,
    kFC4SeamObserverTick = 3,
    kFC4SeamObserverMessage = 4,
    kFC4SeamObserverApp = 5,
    kFC4SeamPowerBudgetPath = 6,
    kFC4SeamFpuReference = 7,
};

using FC4PostSinkFn = void (*)(void* context, cIGZMessage2Standard* msg);
using FC4TestSeamFn = intptr_t(__cdecl*)(uint32_t op, void* a, void* b);

namespace fc4 {
// FC4BudgetHost.cpp
void SetBudgetPostSink(FC4PostSinkFn fn, void* context);
}  // namespace fc4
