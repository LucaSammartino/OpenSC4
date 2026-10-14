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

#include "FC4AuraShadow.h"
#include "FC4BudgetShadow.h"
#include "FC4CrimeShadow.h"
#include "FC4DemandShadow.h"
#include "FC4FlammabilityShadow.h"
#include "FC4FloraShadow.h"
#include "FC4GrowthProbe.h"
#include "FC4LandValueShadow.h"
#include "FC4LandfillShadow.h"
#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4Observer.h"
#include "FC4PlumbingShadow.h"
#include "FC4PollutionShadow.h"
#include "FC4PowerShadow.h"
#include "FC4ProtectionShadow.h"
#include "FC4ResidentialShadow.h"
#include "FC4Popups.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "cFC4Crime.h"
#include "cFC4Demand.h"
#include "cFC4Flammability.h"
#include "cFC4Landfill.h"
#include "cFC4Plumbing.h"
#include "cFC4Power.h"
#include "cFC4ZoneManager.h"
#include "cFC4Protection.h"
#include "cFC4Variant.h"
#include "fc4.h"

#include <cstring>
#include <vector>

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
        case kFC4SeamCrimeShadow:
            return CrimeShadowTestRun(static_cast<const FC4CrimeShadowTestInput*>(a));
        case kFC4SeamAuraShadow:
            return AuraShadowTestCityLoad(static_cast<const FC4AuraShadowDeliverInput*>(a));
        case kFC4SeamAuraShadowDeliver:
            return AuraShadowTestDeliver(static_cast<const FC4AuraShadowDeliverInput*>(a));
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
        case kFC4SeamFloraShadow:
            return FloraShadowTestDeliver(static_cast<const FC4FloraShadowTestInput*>(a), static_cast<intptr_t*>(b));
        case kFC4SeamLandfillShadow:
            return LandfillShadowTestDeliver(static_cast<const FC4LandfillShadowTestInput*>(a));
        case kFC4SeamPowerShadow:
            return PowerShadowTestDeliver(static_cast<const FC4PowerShadowTestInput*>(a));
        case kFC4SeamPlumbingShadow:
            return PlumbingShadowTestRun(static_cast<const FC4PlumbingShadowTestInput*>(a));
        case kFC4SeamDemandCapacity: {
            const auto* in = static_cast<const FC4DemandCapacityTestInput*>(a);
            if (in == nullptr) return -1;
            return DemandReportCapacities(nullptr, in->buildingType, in->profile, in->visit, in->ctx);
        }
        case kFC4SeamLandValueShadow:
            return LandValueShadowTest(static_cast<const FC4LandValueShadowTestInput*>(a));
        case kFC4SeamFlammabilityShadow:
            return FlammabilityShadowTest(static_cast<const FC4FlammabilityShadowTestInput*>(a));
        case kFC4SeamResidentialShadow:
            return ResidentialShadowTestDeliver(static_cast<const FC4ResidentialShadowTestInput*>(a));
        case kFC4SeamRawArrayBytes: {
            auto* io = static_cast<FC4RawArrayBytesTestIo*>(b);
            if (a == nullptr || io == nullptr) return -1;
            std::vector<uint8_t> bytes;
            if (!fc4::props::GetRawArrayBytes(static_cast<const cISCPropertyHolder*>(a), io->id, bytes)) return 0;
            io->len = static_cast<uint32_t>(bytes.size());
            if (!bytes.empty())
                std::memcpy(io->bytes, bytes.data(), bytes.size() < sizeof(io->bytes) ? bytes.size() : sizeof(io->bytes));
            return 1;
        }
        case kFC4SeamLotCenter:
            return fc4::CrimeTestLotCenter(a, static_cast<float*>(b));
        case kFC4SeamCellCenter: {
            auto* io = static_cast<FC4CellCenterTestIo*>(b);
            if (a == nullptr || io == nullptr) return -1;
            fc4::ProtectionContext pc;
            pc.readOnly = true;
            pc.city = static_cast<cISC4City*>(a);
            const fc4_protection_host ph = fc4::MakeProtectionReadOnlyHost(&pc);
            ph.cell_center_to_position(ph.user, io->cx, io->cz, &io->protectionX, &io->protectionZ);
            fc4::LandfillContext lc;
            lc.readOnly = true;
            lc.city = static_cast<cISC4City*>(a);
            const fc4_landfill_host lh = fc4::MakeLandfillReadOnlyHost(&lc);
            lh.cell_center_to_position(lh.user, io->cx, io->cz, &io->landfillX, &io->landfillZ);
            return 1;
        }
        case kFC4SeamPipeBurst: {
            const int32_t* xz = static_cast<const int32_t*>(b);
            if (a == nullptr || xz == nullptr) return -1;
            return fc4::PlumbingTestPipeBurst(static_cast<cISC4City*>(a), xz[0], xz[1]);
        }
        case kFC4SeamVariant: {
            if (a == nullptr) return -1;
            const cFC4Variant proto;
            cFC4IGZVariant* copy = nullptr;
            if (!proto.CreateCopy(&copy)) return -1;
            *static_cast<cFC4IGZVariant**>(a) = copy;
            return 1;
        }
        case kFC4SeamX87Check: {
            static fc4::X87Site site = {"test seam", 0, 0, 0, nullptr};
            return fc4::X87Check(site);
        }
        case kFC4SeamRustPanic:
            fc4_debug_panic();
            return 1;  // only if the build unwinds
        case kFC4SeamProtectionShadowVtables:
            fc4::ProtectionShadowTestOriginalVtables(a, b);
            return 1;
        case kFC4SeamZones:
            return a == nullptr ? -1 : ZonesTestSeam(static_cast<const FC4ZonesTestInput*>(a));
        case kFC4SeamGrowthProbe:
            return GrowthProbeTest(static_cast<const FC4GrowthProbeTestInput*>(a));
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
