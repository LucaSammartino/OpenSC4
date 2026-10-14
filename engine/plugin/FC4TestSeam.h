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
//         The popup-dialogs switch (FC4Popups.h) uses it too (borrowed).
//   op 6  kFC4SeamPowerBudgetPath  a = cISC4BudgetSimulator* (initialised):
//         runs the power glue's department / line-item callbacks on it,
//         including a department reference read back from a save; returns
//         0, or the first failed step (logged).
//   op 7  kFC4SeamFpuReference     a = null: the calling thread's x87 control
//         word becomes the guard's reference (as PreAppInit does in the
//         game); a != null: the per-tick check alone (FpuTick).
//   op 8  kFC4SeamRustPanic        a deliberate panic in the core
//         (fc4_debug_panic): release builds log the panic report and
//         abort with a fail-fast.
//   op 9  kFC4SeamFlammabilityFire a = cISC4City*: the flammability glue's
//         ignition path (IgniteCell, then SetCause(5)) on the city's
//         disaster layer; returns 0, or 1 (no city) / 2 (no ignition).
//   op 10 kFC4SeamBudgetShadow     a = const FC4ShadowTestInput*
//         (FC4BudgetShadow.h): one budget-shadow month on a fake city whose
//         budget is the "original"; returns the month's mismatch count, -1
//         (not taken) or -2 (delivered, not compared).
//   op 11 kFC4SeamDemandShadow     a = const FC4DemandShadowTestInput*
//         (FC4DemandShadow.h): one demand-shadow delivery (NewDay or SimIdle)
//         to a fake "original" demand simulator; returns the update's
//         mismatch count, -1 (not taken) or -2 (delivered, nothing compared).
//   op 12 kFC4SeamRefAuditRelease  a = cIGZUnknown* (a game object whose
//         Release returns 0), b = mode: bit 0 = note an acquired reference
//         first (FC4_ACQUIRED), bit 1 = release with FC4_RELEASE_CREATED
//         (else FC4_RELEASE); returns the reference warnings it caused.
//   op 13 kFC4SeamCrimeShadow      a = const FC4CrimeShadowTestInput*
//         (FC4CrimeShadow.h): one crime-shadow month (SimNewMonth, then
//         SimNewDay up to pipeline state 4) to a fake "original" crime
//         simulator; returns the month's mismatch count, -1 (not taken) or
//         -2 (delivered, nothing compared).
//   op 14 kFC4SeamAuraShadow       a = const FC4AuraShadowTestInput*
//         (FC4AuraShadow.h): the aura shadow on a fake city whose aura
//         simulator is the "original": the city-load baseline or one
//         cycle-completed comparison; returns the cycle's difference count
//         (a baseline: 1 taken, 0 not), -1 (not run) or -2 (ran, nothing
//         compared).
//   op 16 kFC4SeamPollutionShadow  a = const FC4PollutionShadowTestInput*
//         (FC4PollutionShadow.h): one pollution-shadow delivery (SimNewDay or
//         SimNewMonth) to a fake "original" pollution simulator; returns the
//         delivery's difference count, -1 (not taken) or -2 (delivered,
//         nothing compared). (15 is left to the parallel glue lanes.)
//   op 17 kFC4SeamFloraShadow      a = const FC4FloraShadowTestInput*,
//         b = intptr_t* (or null) for the registry check's difference count
//         (FC4FloraShadow.h): one flora-shadow SimNewMonth to a fake
//         "original" flora simulator; returns the month's mismatch count, -1
//         (not taken) or -2 (delivered, nothing compared).
//   op 18 kFC4SeamLandfillShadow   a = const FC4LandfillShadowTestInput*
//         (FC4LandfillShadow.h): one landfill-shadow SimNewMonth delivery to
//         a fake "original" landfill developer; returns the month's
//         difference count, -1 (not taken) or -2 (delivered, nothing
//         compared).
//   op 20 kFC4SeamDemandCapacity   a = const FC4DemandCapacityTestInput*
//         (cFC4Demand.h): the demand glue's capacity-map read of a building
//         profile (DemandReportCapacities, no simulator): the visitor gets
//         every entry; returns the entries reported, 0 (switch off) or -1
//         (refused, logged).
//   op 21 kFC4SeamPowerShadow      a = const FC4PowerShadowTestInput*
//         (FC4PowerShadow.h): one power-shadow delivery (SimNewDay, SimIdle
//         or SimNewMonth) to a fake "original" power simulator; returns the
//         delivery's difference count, -1 (not taken), -2 (delivered,
//         nothing compared) or -3 (passed through: the original idle).
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
    kFC4SeamRustPanic = 8,
    kFC4SeamFlammabilityFire = 9,
    kFC4SeamBudgetShadow = 10,
    kFC4SeamDemandShadow = 11,
    kFC4SeamRefAuditRelease = 12,
    kFC4SeamCrimeShadow = 13,
    kFC4SeamAuraShadow = 14,
    kFC4SeamPollutionShadow = 16,
    kFC4SeamFloraShadow = 17,
    kFC4SeamLandfillShadow = 18,
    kFC4SeamDemandCapacity = 20,
    kFC4SeamPowerShadow = 21,
};

using FC4PostSinkFn = void (*)(void* context, cIGZMessage2Standard* msg);
using FC4TestSeamFn = intptr_t(__cdecl*)(uint32_t op, void* a, void* b);

namespace fc4 {
// FC4BudgetHost.cpp
void SetBudgetPostSink(FC4PostSinkFn fn, void* context);
}  // namespace fc4
