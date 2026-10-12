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

#include "FC4TestSeam.h"

#include "FC4Log.h"
#include "FC4Observer.h"

extern "C" __declspec(dllexport) intptr_t __cdecl FC4TestSeam(uint32_t op, void* a, void* b) {
    switch (op) {
        case kFC4SeamBudgetPostSink:
            fc4::Log("test seam: budget post sink %s", a ? "installed" : "removed");
            fc4::SetBudgetPostSink(reinterpret_cast<FC4PostSinkFn>(a), b);
            return 1;
        case kFC4SeamObserverCity:
            fc4::ObserverTestSetCity(static_cast<cISC4City*>(a));
            return 1;
        case kFC4SeamObserverTick:
            fc4::ObserverTestTick();
            return 1;
        case kFC4SeamObserverMessage:
            fc4::ObserverTestMessage(static_cast<uint32_t>(reinterpret_cast<uintptr_t>(a)), reinterpret_cast<intptr_t>(b));
            return 1;
        default:
            return 0;
    }
}
