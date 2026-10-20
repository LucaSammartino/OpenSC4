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

// The replacement network manager's tool stand-in (glue24).
//
// §13 of network-manager.md: "The tools, models, remap tables and retrofit are
// presentation or legacy" - a replacement does not have to reimplement the
// tool subsystem (network-tools.md, network-geometry.md). But the original's
// Init holds 13 real tools (§3.1 step 5) and other components take them from
// the manager's slot 5 without a null test, so the replacement's slot 5 must
// answer a usable object: the game's zone developer dereferences the null at
// PostCityInit (runs glue24-rep-1..3 crash at SimCity 4.exe+0x33293C).
//
// The game's own tool classes are constructed by the original manager's own
// code (network-manager.md §7.1: the factory 0x00610907-0x00610910 calls the
// constructors directly); no GZCOM class ID for them is stated in the specs,
// so a replacement cannot create the game's tools without a new RE fact
// (question NM11). This stand-in answers the SDK's cISC4NetworkTool interface
// in the compiled slot order (the SDK's declaration matches, interfaces.tsv
// 0x00AA8748) and does nothing: Init/Shutdown true, everything else
// null/0/false. That is enough for the runs: at PostCityInit the zone
// developer only takes three tools, AddRefs them and Inits them (zones.md
// §5.2); placement, the destroy pipeline and the underground refit are only
// reached on a player or scripted drag, a demolition or a terrain edit, and
// the scripted `network` action refuses a stand-in (FC4Script.cpp checks the
// class before it places; the log line says so).
#pragma once

#include "cISC4NetworkTool.h"

#include <cstdint>

// The stand-in tool. QueryInterface answers cIGZUnknown and cISC4NetworkTool
// (0x8A21EF94? no: the SDK names no IID constant; the interface the game
// queries the tool for is its own - the tool travels as the pointer the
// manager hands out, and the only QueryInterfaces the game does on it are
// cIGZUnknown's (slot 0) and nothing else in the specs).
class cFC4NetworkTool final : public cISC4NetworkTool {
public:
    cFC4NetworkTool() = default;

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4NetworkTool (the SDK's declaration order = the compiled order).
    bool Init() override;
    bool Shutdown() override;
    void Reset() override;
    uint32_t GetUITool(cISC4NetworkToolUI** outUi) override;
    uint32_t PlaceNetworkOccupants(uint32_t startX, uint32_t startZ, uint32_t endX, uint32_t endZ,
                                   unsigned long networkMask, cISC4NetworkTool* toolCallback,
                                   bool bUseWorldCache,
                                   cISC4NetworkOccupant::eNetworkType networkType) override;
    void PlaceNetworkOccupantById(int x, int z, unsigned long occupantId, bool bSelect,
                                  cISC4NetworkOccupant::eNetworkType networkType) override;
    uint32_t OccupantExists(uint32_t x, uint32_t z, unsigned long networkMask,
                            unsigned long& outOccupantId) override;
    void UpdateOnZoneChange() override;
    void ForceReload() override;
    void SetTraceFinal(bool enable) override;
    void ClearFailureReason() override;
    uint32_t GetFailureState() const override;
    void GetFailureReasonString(cIGZString& outStr) const override;
    void ClearOccupantHighlights() override;
    void ClearCellPreviews() override;
    uint32_t DrawNetworkLine(const SC4Point<uint32_t>& start, const SC4Point<uint32_t>& end, bool useWorldCache,
                             cISC4NetworkOccupant::eNetworkType networkType) override;
    void PlaceAllSegments(bool commit, bool previewOnly, bool allowDemolition,
                          cISC4NetworkOccupant::eNetworkType networkType) override;
    int GetCostOfSolution() const override;
    void EnablePreviewUpdate(bool enable) override;
    void SetAnchorPoints(const SC4Vector<SC4Point<uint32_t>>& points) override;
    void GetDraggedCells(SC4Vector<SC4Point<uint32_t>>& outCells) const override;
    void SetValidStepCount(uint32_t count) override;
    uint32_t GetValidStepCount() const override;
    uint32_t GetStraightStepCount() const override;
    uint32_t GetTotalStepCount() const override;
    bool IsRegularStep(uint32_t stepIndex) const override;
    void GetLastValidPoint(SC4Point<uint32_t>& outPoint) const override;
    bool GetPossibleBridges(SC4Vector<int32_t>& outBridgeInstanceIds) const override;
    int GetPreferredBridge() const override;
    void SetPreferredBridge(int bridgeId) override;
    void GetBridgeAltitudeOverride() override;
    void SetBridgeAltitudeOverride(float altitude) override;
    void GetBridgeName(cIGZString& outStr, int bridgeInstanceId) const override;
    int GetMajorBridge() const override;
    bool GetMajorBridgeAltitude(float& outAltitude) const override;
    bool GetMajorBridgeReferenceAltitude(float& outAltitude) const override;
    bool IsMajorBridgeLandBridge() const override;
    bool GetBridgeBitmapKey(long bridgeId, unsigned long& outGroup, unsigned long& outInstance) const override;
    bool GetPossibleNeighborConnection(NeighborConnectionInfo& outInfo) const override;
    uint32_t ViewInputControlId() const override;
    uint32_t CursorId() const override;
    uint32_t DirectionalCursorBase() const override;
    void PenWidth(int& outWidthX, int& outWidthZ) const override;
    uint32_t StopPickAtWater() const override;
    uint32_t GetToolViewId() const override;
    void GetPlacementParams(tPlacementParams& outParams) const override;
    void QueryUserForPotentialIntersections() override;
    void DestroyOccupantsAndResolve(const SC4Vector<cISC4NetworkOccupant*>& occupants) override;

private:
    ~cFC4NetworkTool() = default;
    uint32_t refs_ = 0;
};
