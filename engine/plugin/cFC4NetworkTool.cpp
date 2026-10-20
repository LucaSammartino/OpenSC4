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

#include "cFC4NetworkTool.h"

#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"

// The SDK's cISC4NetworkTool declarations match the compiled order (the SDK
// lists from Init, interfaces.tsv 0x00AA8748 slot 3 = SDK index 0), so
// inheriting from it gives the compiled vtable. The class-method slots past
// the SDK (51-81, network-geometry.md §1.1) are not in the interface; nothing
// outside the tool's own class calls them in the specs.
namespace {

constexpr uint32_t kIID_cIGZUnknown = 1;  // the framework's 'any object' IID (GZIID_cIGZUnknown)

}  // namespace

bool cFC4NetworkTool::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (riid != kIID_cIGZUnknown) {
        *ppvObj = nullptr;
        return false;
    }
    *ppvObj = static_cast<cIGZUnknown*>(this);
    AddRef();
    return true;
}

uint32_t cFC4NetworkTool::AddRef() { return ++refs_; }

uint32_t cFC4NetworkTool::Release() {
    if (refs_ > 1) return --refs_;
    delete this;
    return 0;
}

bool cFC4NetworkTool::Init() {
    // The original's Init reads the type's placement exemplars (§2) and
    // builds the occupant filter; this stand-in has no state to load.
    return true;
}

bool cFC4NetworkTool::Shutdown() { return true; }

void cFC4NetworkTool::Reset() {}

uint32_t cFC4NetworkTool::GetUITool(cISC4NetworkToolUI**) { return 0; }

uint32_t cFC4NetworkTool::PlaceNetworkOccupants(uint32_t, uint32_t, uint32_t, uint32_t, unsigned long,
                                                cISC4NetworkTool*, bool, cISC4NetworkOccupant::eNetworkType) {
    // §3: the placement pipeline is network-tools.md §5 and network-geometry.md
    // (not implemented); a stand-in places nothing and reports the failure
    // (§3.4: a failed validation places nothing and returns false).
    return 0;
}

void cFC4NetworkTool::PlaceNetworkOccupantById(int, int, unsigned long, bool,
                                               cISC4NetworkOccupant::eNetworkType) {}

uint32_t cFC4NetworkTool::OccupantExists(uint32_t, uint32_t, unsigned long, unsigned long&) { return 0; }

void cFC4NetworkTool::UpdateOnZoneChange() {}

void cFC4NetworkTool::ForceReload() {}

void cFC4NetworkTool::SetTraceFinal(bool) {}

void cFC4NetworkTool::ClearFailureReason() {}

uint32_t cFC4NetworkTool::GetFailureState() const { return 0; }

void cFC4NetworkTool::GetFailureReasonString(cIGZString&) const {}

void cFC4NetworkTool::ClearOccupantHighlights() {}

void cFC4NetworkTool::ClearCellPreviews() {}

uint32_t cFC4NetworkTool::DrawNetworkLine(const SC4Point<uint32_t>&, const SC4Point<uint32_t>&, bool,
                                          cISC4NetworkOccupant::eNetworkType) {
    return 0;
}

void cFC4NetworkTool::PlaceAllSegments(bool, bool, bool, cISC4NetworkOccupant::eNetworkType) {}

int cFC4NetworkTool::GetCostOfSolution() const { return 0; }

void cFC4NetworkTool::EnablePreviewUpdate(bool) {}

void cFC4NetworkTool::SetAnchorPoints(const SC4Vector<SC4Point<uint32_t>>&) {}

void cFC4NetworkTool::GetDraggedCells(SC4Vector<SC4Point<uint32_t>>&) const {}

void cFC4NetworkTool::SetValidStepCount(uint32_t) {}

uint32_t cFC4NetworkTool::GetValidStepCount() const { return 0; }

uint32_t cFC4NetworkTool::GetStraightStepCount() const { return 0; }

uint32_t cFC4NetworkTool::GetTotalStepCount() const { return 0; }

bool cFC4NetworkTool::IsRegularStep(uint32_t) const { return false; }

void cFC4NetworkTool::GetLastValidPoint(SC4Point<uint32_t>&) const {}

bool cFC4NetworkTool::GetPossibleBridges(SC4Vector<int32_t>&) const { return false; }

int cFC4NetworkTool::GetPreferredBridge() const { return -1; }

void cFC4NetworkTool::SetPreferredBridge(int) {}

void cFC4NetworkTool::GetBridgeAltitudeOverride() {}

void cFC4NetworkTool::SetBridgeAltitudeOverride(float) {}

void cFC4NetworkTool::GetBridgeName(cIGZString&, int) const {}

int cFC4NetworkTool::GetMajorBridge() const { return -1; }

bool cFC4NetworkTool::GetMajorBridgeAltitude(float&) const { return false; }

bool cFC4NetworkTool::GetMajorBridgeReferenceAltitude(float&) const { return false; }

bool cFC4NetworkTool::IsMajorBridgeLandBridge() const { return false; }

bool cFC4NetworkTool::GetBridgeBitmapKey(long, unsigned long&, unsigned long&) const { return false; }

bool cFC4NetworkTool::GetPossibleNeighborConnection(NeighborConnectionInfo&) const { return false; }

uint32_t cFC4NetworkTool::ViewInputControlId() const { return 0; }

uint32_t cFC4NetworkTool::CursorId() const { return 0; }

uint32_t cFC4NetworkTool::DirectionalCursorBase() const { return 0; }

void cFC4NetworkTool::PenWidth(int& outWidthX, int& outWidthZ) const {
    outWidthX = 0;
    outWidthZ = 0;
}

uint32_t cFC4NetworkTool::StopPickAtWater() const { return 0; }

uint32_t cFC4NetworkTool::GetToolViewId() const { return 0; }

void cFC4NetworkTool::GetPlacementParams(tPlacementParams&) const {}

void cFC4NetworkTool::QueryUserForPotentialIntersections() {}

void cFC4NetworkTool::DestroyOccupantsAndResolve(const SC4Vector<cISC4NetworkOccupant*>&) {
    // §10: the collect / close / remove / rebuild pipeline (the rebuild is
    // network-geometry.md §9, not implemented). A stand-in destroys nothing;
    // the core's destroy groups (§7.2) already release what they hold, so a
    // no-op here leaves the occupants in place, which the log counts.
}
