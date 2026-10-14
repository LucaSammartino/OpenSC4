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

#include "FC4BudgetShadow.h"
#include "FC4DemandShadow.h"
#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4Observer.h"
#include "FC4PollutionShadow.h"
#include "FC4Popups.h"
#include "FC4RefAudit.h"
#include "cFC4Demand.h"
#include "cFC4Flammability.h"
#include "cFC4Power.h"
#include "fc4.h"

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
        case kFC4SeamObserverApp:
            fc4::ObserverTestSetApp(static_cast<cISC4App*>(a));
            fc4::PopupDialogsTestSetApp(static_cast<cISC4App*>(a));
            return 1;
        case kFC4SeamPowerBudgetPath:
            return PowerTestBudgetPath(static_cast<cISC4BudgetSimulator*>(a));
        case kFC4SeamFlammabilityFire:
            return FlammabilityTestFirePath(static_cast<cISC4City*>(a));
        case kFC4SeamBudgetShadow:
            return BudgetShadowTestMonth(static_cast<const FC4ShadowTestInput*>(a));
        case kFC4SeamDemandShadow:
            return DemandShadowTestDeliver(static_cast<const FC4DemandShadowTestInput*>(a));
        case kFC4SeamRefAuditRelease: {
            cIGZUnknown* o = static_cast<cIGZUnknown*>(a);
            const intptr_t mode = reinterpret_cast<intptr_t>(b);
            const long before = fc4::RefWarningCount();
            if ((mode & 1) != 0) FC4_ACQUIRED(o, "test seam");
            if ((mode & 2) != 0) {
                FC4_RELEASE_CREATED(o);
            } else {
                FC4_RELEASE(o);
            }
            return fc4::RefWarningCount() - before;
        }
        case kFC4SeamPollutionShadow:
            return PollutionShadowTestDeliver(static_cast<const FC4PollutionShadowTestInput*>(a));
        case kFC4SeamDemandCapacity: {
            const auto* in = static_cast<const FC4DemandCapacityTestInput*>(a);
            if (in == nullptr) return -1;
            return DemandReportCapacities(nullptr, in->buildingType, in->profile, in->visit, in->ctx);
        }
        case kFC4SeamRustPanic:
            fc4_debug_panic();
            return 1;  // only if the build unwinds
        case kFC4SeamFpuReference:
            if (a == nullptr) {
                fc4::FpuSetReference("test seam");
            } else {
                fc4::FpuTick();  // the observer's per-tick check, without the rest of OnTick
            }
            return 1;
        default:
            return 0;
    }
}
