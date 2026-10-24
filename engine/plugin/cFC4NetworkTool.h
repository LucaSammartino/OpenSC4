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

// The replacement network tool (glue29): the cSC4NetworkTool the player's
// drag and the simulation's scripted placements reach, delegating to the
// fc4_nettool_* C ABI (cores27; docs/specs/network-tools.md). The tool state
// (§2), PlaceNetworkOccupants (§3), PlaceNetworkOccupantById (§4), the
// PlaceNetwork pipeline (§5), the commit (§6), DestroyOccupantsAndResolve
// (§10) and the underground refit (§11) are the Rust core's; this class
// adapts the SDK interface to it and owns the game services through the host
// table of FC4NetToolHost.h (the geometry core fc4_netgeom drives the
// pipeline, the occupant manager carries the inserts).
//
// The class inherits FC4Interfaces.h's cFC4ISC4NetworkToolFull — the compiled
// interface order (interfaces.tsv 0x00AA8748, slots 0-50) with the SDK's
// signature errors fixed: slot 10 UpdateOnZoneChange takes a rectangle by
// value and a uint32 (pops 20; the SDK declares none — the game's power
// simulator calls it on the shared type-5 tool, power.md §5), slot 33 returns
// a float in ST0, slot 35 takes (index, string). The class-method slots past
// the SDK (51-80, network-geometry.md §1.1) are not declared: nothing outside
// the tool's own class calls them in the specs, and a replacement that keeps
// the original's layout is [choice].
//
// Threading: the core is single-threaded (RefCell); the plugin serialises
// every call on this one object. Getters run from a second thread while
// OnTick runs (HANDOFF §5.1), so nothing here lives in a RefCell.
//
// OFF by default: FreeCity4.ini `enable = nettool`; `disable` still wins.
// The tools are created by the network manager's factory (§7.1) — with the
// original manager running, `enable = nettool` swaps the manager's tool
// classes through the nettool registry below.
#pragma once

#include "cISC4NetworkTool.h"

#include "FC4Interfaces.h"  // cFC4ISC4NetworkToolFull (the compiled interface)
#include "FC4NetToolHost.h"  // fc4::NetToolContext

#include <atomic>
#include <cstdint>

class cIGZMessageServer2;
class cISC4City;

// The replacement tool's synthetic class ID (no GZCOM ID exists for the
// game's tool classes, question NM11) and the factory entry for the
// subsystem table.
static const uint32_t kFC4_CLSID_cFC4NetworkTool = 0x4E543030;  // "NT00"
cIGZUnknown* CreateFC4NetworkToolEntry();

// The replacement tool over the fc4_nettool core. Created through GZCOM as
// class `nettool` (FC4Subsystems.cpp), directly by the glue's tool registry,
// or by the netmgr replacement's create_tool callback (glue24); it starts at
// reference count 0 like the original's factory (network-manager.md §7.1: the
// factory returns it with no AddRef).
//
// Live and stand-in in one class: `live` (the nettool key on) delegates to
// the fc4_nettool core; `!live` (glue24's manager running without the key)
// keeps the do-nothing stand-in semantics glue24's runs were verified with —
// Init/Shutdown true, every placement a no-op that reports failure (§3.4).
// The manager's create_tool passes the key's verdict (cFC4NetworkManager.cpp
// CreateTool), so `enable = netmgr,nettool` puts the real tool in the
// manager's own cache.
class cFC4NetworkTool final : public cFC4ISC4NetworkToolFull {
public:
    cFC4NetworkTool(uint32_t networkType, bool live);
    ~cFC4NetworkTool();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cFC4ISC4NetworkToolFull (the compiled interface order; the methods the
    // table names or the plugin calls).
    bool Init() override;
    bool Shutdown() override;
    void Reset() override;
    uint32_t GetUITool(cISC4NetworkToolUI** outUi) override;
    uint32_t PlaceNetworkOccupants(uint32_t startX, uint32_t startZ, uint32_t endX, uint32_t endZ,
                                   unsigned long networkMask, cFC4ISC4NetworkToolFull* toolCallback,
                                   bool bUseWorldCache,
                                   cISC4NetworkOccupant::eNetworkType networkType) override;
    void PlaceNetworkOccupantById(int x, int z, unsigned long occupantId, bool bSelect,
                                  cISC4NetworkOccupant::eNetworkType networkType) override;
    uint32_t OccupantExists(uint32_t x, uint32_t z, unsigned long networkMask,
                            unsigned long& outOccupantId) override;
    void UpdateOnZoneChange(FC4Int32Quad rect, uint32_t data3) override;
    void ForceReload() override;
    void SetTraceFinal(bool enable) override;
    void ClearFailureReason() override;
    uint32_t GetFailureState() const override;
    void GetFailureReasonString(cIGZString& outStr) override;
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
    float GetBridgeAltitudeOverride() override;
    void SetBridgeAltitudeOverride(float altitude) override;
    void GetBridgeName(int bridgeIndex, cIGZString& outStr) override;
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

    // The city this tool serves (borrowed; set by the subsystem glue at the
    // city load, before Init — the host's queries need it).
    void SetCity(cISC4City* city);
    cISC4City* City() const { return ctx_.city; }
    // The message server for the §10 synchronous pre-demolish send (held).
    void SetMessageServer(cIGZMessageServer2* ms);
    // The core handle (for the host's context and diagnostics; null when
    // the tool runs as the stand-in).
    fc4_nettool* Core() const { return core_; }
    // Whether this tool runs the fc4_nettool core (the nettool key on) or
    // the stand-in no-ops (glue24's verified default).
    bool Live() const { return live_; }
    // The tool's network type (§2: +0x35C).
    uint32_t NetworkType() const { return networkType_; }
    // One status line of what the host did since the last call (the runs).
    void LogStatus(const char* why);

    // The class factory (reference count 0; FC4Subsystems.cpp nettool key).
    static cIGZUnknown* Create(uint32_t networkType);

private:
    // Mutation-thread publication: public count getters must not enter the
    // geometry's RefCell when the game queries them concurrently (§5.1).
    void PublishStepCounts(bool clear = false);
    std::atomic<uint32_t> publishedValid_{0};
    std::atomic<uint32_t> publishedStraight_{0};
    std::atomic<uint32_t> publishedTotal_{0};
    std::atomic<uint32_t> refCount_{0};
    bool destroying_ = false;
    bool live_ = false;
    uint32_t networkType_ = 0;
    fc4::NetToolContext ctx_{};
    fc4_nettool_host host_{};
    fc4_nettool* core_ = nullptr;
    // The last run's counters for the status line.
    uint32_t lastInserts_ = 0;
    uint32_t lastRemoved_ = 0;
};

// The tool registry (the glue's inject point). With `enable = nettool` the
// callers inside our plugin (the scripted `network` action, the zone
// developer's street tools, the power line's shared tool) take their tools
// here instead of the network manager's factory, so the placements run
// through the fc4_nettool core. The tools are cached per type for the city's
// lifetime (the shared-tool semantics of network-manager.md §7.1); they are
// released at the city shutdown (NetToolRegistryCityShutdown).
namespace NetToolRegistry {
// Whether the nettool key is on in this run (read once, logged).
bool Enabled();
// FC4Subsystems.cpp: the enable key's verdict (called once at game start).
void SetEnabled(bool on);
// The shared tool of `type` (0-12), created on first use; null outside the
// registry. The pointer stays valid until the city shutdown (borrowed).
cFC4NetworkTool* Take(int32_t type, cISC4City* city, cIGZMessageServer2* ms);
// Releases every cached tool (PreCityShutdown).
void CityShutdown();
// The registry's status counters for the log line.
void LogSummary(const char* why);
}  // namespace NetToolRegistry
