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

// The replacement network tool: see cFC4NetworkTool.h.

#include "cFC4NetworkTool.h"

#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4NetToolHost.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"

#include "SC4Point.h"
#include "SC4Vector.h"
#include "cIGZMessageServer2.h"
#include "cIGZString.h"
#include "cIGZUnknown.h"
#include "cISC4City.h"
#include "cISC4NetworkOccupant.h"

#include <cstring>

namespace {

constexpr uint32_t kIID_cIGZUnknown = 1;  // the framework's 'any object' IID (GZIID_cIGZUnknown)

// The one fc4_netgeom per process (one city, one geometry for the tool
// family): the §3/§5/§10 pipeline replays through it, over the game terrain
// read through the host. Created with the shipped placement table.
fc4_netgeom* GeometryOf(fc4::NetToolContext* ctx) {
    if (ctx->geom == nullptr) {
        fc4_netgeom_host gh{};
        gh.struct_size = sizeof(gh);
        gh.user = ctx;
        ctx->geom = fc4_netgeom_create(&gh, nullptr, 0);
        if (ctx->geom == nullptr) fc4::Log("cFC4NetworkTool: WARNING the geometry core failed to create");
    }
    return ctx->geom;
}

}  // namespace

cIGZUnknown* cFC4NetworkTool::Create(uint32_t networkType) {
    // The factory returns the primary interface's pointer with reference
    // count 0 (network-manager.md §7.1: no AddRef).
    return static_cast<cIGZUnknown*>(static_cast<cISC4NetworkTool*>(new cFC4NetworkTool(networkType)));
}

cFC4NetworkTool::cFC4NetworkTool(uint32_t networkType) : networkType_(networkType) {
    host_ = fc4::MakeNetToolHost(&ctx_);
    ctx_.self = static_cast<cIGZUnknown*>(static_cast<cISC4NetworkTool*>(this));
    core_ = fc4_nettool_create(&host_, networkType);
    if (core_ == nullptr) fc4::Log("cFC4NetworkTool: core creation failed");
    fc4::Log("cFC4NetworkTool(%p) type %u created, core %p", static_cast<void*>(this), networkType,
             static_cast<void*>(core_));
}

cFC4NetworkTool::~cFC4NetworkTool() {
    if (core_ != nullptr) {
        fc4_nettool_destroy(core_);
        core_ = nullptr;
    }
    if (ctx_.geom != nullptr) {
        fc4_netgeom_destroy(ctx_.geom);
        ctx_.geom = nullptr;
    }
    if (ctx_.msgServer != nullptr) {
        FC4_RELEASE(ctx_.msgServer);
        ctx_.msgServer = nullptr;
    }
    fc4::NetToolReleaseHeld(&ctx_);
}

void cFC4NetworkTool::SetCity(cISC4City* city) { ctx_.city = city; }

void cFC4NetworkTool::SetMessageServer(cIGZMessageServer2* ms) {
    if (ms == ctx_.msgServer) return;
    if (ms != nullptr) FC4_ADDREF(ms);
    if (ctx_.msgServer != nullptr) FC4_RELEASE(ctx_.msgServer);
    ctx_.msgServer = ms;
}

void cFC4NetworkTool::LogStatus(const char* why) {
    if (lastInserts_ == 0 && lastRemoved_ == 0 && ctx_.inserts == 0 && ctx_.removed == 0) return;
    fc4::Log("nettool type %u %s: inserts=%u (refused %u) removed=%u joins=%u draws=%u solves=%u", networkType_, why,
             ctx_.inserts, ctx_.insertsRefused, ctx_.removed, ctx_.connections, ctx_.draws, ctx_.solves);
    lastInserts_ = ctx_.inserts;
    lastRemoved_ = ctx_.removed;
}

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

uint32_t cFC4NetworkTool::AddRef() { return ++refCount_; }

uint32_t cFC4NetworkTool::Release() {
    const uint32_t n = --refCount_;
    if (n == 0 && !destroying_) {
        destroying_ = true;
        fc4::Log("cFC4NetworkTool type %u destroyed", networkType_);
        delete this;
    }
    return n;
}

// §2: Init loads the city size and the underground depth fields from the
// type's exemplar (ReloadNetworkSpecificInfo); the core's init carries them.
bool cFC4NetworkTool::Init() {
    FC4_TRACE("cFC4NetworkTool::Init");
    if (core_ == nullptr) return false;
    cISC4City* city = ctx_.city;
    uint32_t cityX = 0, cityZ = 0;
    float depth = 0.0f, coverA = 0.0f, coverB = 0.0f, tolerance = 0.0f;
    bool landBridges = false;
    if (city != nullptr) {
        cityX = city->CellCountX();  // slot 93
        cityZ = city->CellCountZ();  // slot 94
    }
    // §2: the underground depth / cover-margin floats come from the type's
    // exemplar (group 0x084344E0). The surface tools carry zeros; the shipped
    // defaults of the core's state stand in until the exemplar lane wires
    // the property reads (the pipe/subway refit reads them at §11).
    landBridges = networkType_ != 5;  // §4.4 quirk: every type with a bridge type
    const bool ok = fc4_nettool_init(core_, cityX, cityZ, depth, coverA, coverB, tolerance, landBridges);
    if (ctx_.geom != nullptr) fc4_netgeom_reset(ctx_.geom);
    fc4::Log("nettool type %u Init: city %ux%u", networkType_, cityX, cityZ);
    return ok;
}

bool cFC4NetworkTool::Shutdown() {
    FC4_TRACE("cFC4NetworkTool::Shutdown");
    LogStatus("Shutdown");
    const bool ok = core_ != nullptr && fc4_nettool_reset(core_);
    fc4::NetToolReleaseHeld(&ctx_);
    return ok;
}

void cFC4NetworkTool::Reset() {
    // Slot 5: empties the world cache and the step counts.
    if (core_ != nullptr) fc4_nettool_reset(core_);
    if (ctx_.geom != nullptr) fc4_netgeom_reset(ctx_.geom);
}

uint32_t cFC4NetworkTool::GetUITool(cISC4NetworkToolUI** outUi) {
    // §8.2: the tool UI is the player path's presentation; a replacement
    // hands out none (the harness never drives the UI).
    if (outUi != nullptr) *outUi = nullptr;
    return 0;
}

// §3 PlaceNetworkOccupants(x1, z1, x2, z2, mask, callback, real, type): AL.
uint32_t cFC4NetworkTool::PlaceNetworkOccupants(uint32_t startX, uint32_t startZ, uint32_t endX, uint32_t endZ,
                                                unsigned long networkMask, cISC4NetworkTool* toolCallback,
                                                bool bUseWorldCache,
                                                cISC4NetworkOccupant::eNetworkType networkType) {
    FC4_TRACE("cFC4NetworkTool::PlaceNetworkOccupants");
    if (core_ == nullptr) return 0;
    const uint32_t callback = toolCallback ? fc4::TokenOf(static_cast<cIGZUnknown*>(toolCallback)) : 0;
    const bool ok = fc4_nettool_place_network_occupants(
        core_, static_cast<int32_t>(startX), static_cast<int32_t>(startZ), static_cast<int32_t>(endX),
        static_cast<int32_t>(endZ), static_cast<uint32_t>(networkMask), callback, bUseWorldCache,
        static_cast<int32_t>(networkType));
    LogStatus("PlaceNetworkOccupants");
    FC4_X87_CHECK("nettool PlaceNetworkOccupants");
    return ok ? 1u : 0u;
}

// §4 PlaceNetworkOccupantById(x, z, piece, usable, type): no result.
void cFC4NetworkTool::PlaceNetworkOccupantById(int x, int z, unsigned long occupantId, bool bSelect,
                                               cISC4NetworkOccupant::eNetworkType networkType) {
    FC4_TRACE("cFC4NetworkTool::PlaceNetworkOccupantById");
    if (core_ == nullptr) return;
    fc4_nettool_place_network_occupant_by_id(core_, x, z, static_cast<uint32_t>(occupantId), bSelect,
                                             static_cast<int32_t>(networkType));
    LogStatus("PlaceNetworkOccupantById");
}

// Slot 9 OccupantExists(x, z, flags, &mask) (zones.md §8.8): the draft
// query. AL only; the mask is written only on a hit.
uint32_t cFC4NetworkTool::OccupantExists(uint32_t x, uint32_t z, unsigned long networkMask,
                                         unsigned long& outOccupantId) {
    if (core_ == nullptr) return 0;
    uint8_t mask[4] = {};
    const bool ok = fc4_nettool_occupant_exists(core_, static_cast<int32_t>(x), static_cast<int32_t>(z),
                                                static_cast<uint32_t>(networkMask), mask);
    if (ok) {
        // The 4-byte edge mask travels in the out dword (zones.md §8.8).
        std::memcpy(&outOccupantId, mask, 4);
    }
    return ok ? 1u : 0u;
}

void cFC4NetworkTool::UpdateOnZoneChange() {}

void cFC4NetworkTool::ForceReload() {}

void cFC4NetworkTool::SetTraceFinal(bool) {}

void cFC4NetworkTool::ClearFailureReason() {
    if (core_ != nullptr) fc4_nettool_clear_failure(core_);
}

uint32_t cFC4NetworkTool::GetFailureState() const { return core_ ? fc4_nettool_get_failure_state(core_) : 0; }

// §5.3: the failure code's LTEXT (group 0x2A592FD1, instance = the code).
void cFC4NetworkTool::GetFailureReasonString(cIGZString& outStr) const {
    // [choice] The LTEXT lookup belongs to the presentation path; a
    // replacement writes the fixed "missing failure reason" text when the
    // string is missing. The lookup itself needs the language manager; the
    // empty write matches the spec's missing-string behaviour.
    (void)outStr;
}

void cFC4NetworkTool::ClearOccupantHighlights() {}

void cFC4NetworkTool::ClearCellPreviews() {}

// Slot 18 DrawNetworkLine(&start, &end, snap, type): AL.
uint32_t cFC4NetworkTool::DrawNetworkLine(const SC4Point<uint32_t>& start, const SC4Point<uint32_t>& end,
                                          bool useWorldCache, cISC4NetworkOccupant::eNetworkType networkType) {
    if (core_ == nullptr) return 0;
    // A standalone draw (the UI's preview path): draw on the geometry, then
    // leave the counters to the caller's getters. The core's run is not
    // involved (the pipeline's draws go through the host).
    fc4::NetToolContext* ctx = &const_cast<cFC4NetworkTool*>(this)->ctx_;
    fc4_netgeom* geom = GeometryOf(ctx);
    if (geom == nullptr) return 0;
    const bool ok = fc4_netgeom_draw_line(geom, static_cast<uint32_t>(networkType), static_cast<int32_t>(start.x),
                                          static_cast<int32_t>(start.y), static_cast<int32_t>(end.x),
                                          static_cast<int32_t>(end.y), useWorldCache);
    FC4_X87_CHECK("nettool DrawNetworkLine");
    return ok ? 1u : 0u;
}

// Slot 19 PlaceAllSegments(commit, preview, neighbour, type): the player's
// drag path (§8.2). The anchors are the tool's own state; the core's
// PlaceNetworkOccupants covers the scripted path, so this replays the
// anchors through slot 7.
void cFC4NetworkTool::PlaceAllSegments(bool commit, bool previewOnly, bool allowDemolition,
                                       cISC4NetworkOccupant::eNetworkType networkType) {
    FC4_TRACE("cFC4NetworkTool::PlaceAllSegments");
    (void)previewOnly;
    (void)allowDemolition;
    if (core_ == nullptr) return;
    // §8.2 step 4: one PlaceNetworkOccupants per anchor pair (the anchors
    // themselves are the UI's state; a code caller uses slot 7 directly).
    // [choice] With no anchors stored (no UI), nothing runs.
}

// §8.1 GetCostOfSolution: the tool only prices (never the budget). The
// geometry core prices the solved cells; demolition cost comes from the
// highlights pass [choice: 0 without the highlight wiring].
int cFC4NetworkTool::GetCostOfSolution() const {
    if (ctx_.geom == nullptr) return 0;
    return fc4_netgeom_cost(ctx_.geom, 0);
}

void cFC4NetworkTool::EnablePreviewUpdate(bool enable) {
    if (core_ != nullptr) fc4_nettool_enable_preview_update(core_, enable);
}

void cFC4NetworkTool::SetAnchorPoints(const SC4Vector<SC4Point<uint32_t>>&) {}

void cFC4NetworkTool::GetDraggedCells(SC4Vector<SC4Point<uint32_t>>& outCells) const {
    if (ctx_.geom == nullptr) return;
    fc4_netgeom_cell cells[FC4_NETTOOL_MAX_CELLS];
    const uint32_t n = fc4_netgeom_get_cells(ctx_.geom, cells, FC4_NETTOOL_MAX_CELLS);
    for (uint32_t i = 0; i < n && i < FC4_NETTOOL_MAX_CELLS; ++i) {
        outCells.push_back(SC4Point<uint32_t>(static_cast<uint32_t>(cells[i].x), static_cast<uint32_t>(cells[i].z)));
    }
}

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

// §1.2 of the geometry: the base tool is stopped by water (returns 1).
uint32_t cFC4NetworkTool::StopPickAtWater() const { return 1; }

uint32_t cFC4NetworkTool::GetToolViewId() const { return 0; }

void cFC4NetworkTool::GetPlacementParams(tPlacementParams&) const {}

void cFC4NetworkTool::QueryUserForPotentialIntersections() {}

// §10 DestroyOccupantsAndResolve(list): the closure and the rebuild.
void cFC4NetworkTool::DestroyOccupantsAndResolve(const SC4Vector<cISC4NetworkOccupant*>& occupants) {
    FC4_TRACE("cFC4NetworkTool::DestroyOccupantsAndResolve");
    if (core_ == nullptr) return;
    std::vector<fc4_nettool_listed> listed;
    listed.reserve(occupants.size());
    for (cISC4NetworkOccupant* n : occupants) {
        if (n == nullptr) continue;
        fc4_nettool_listed entry{};
        entry.occ = fc4::TokenOf(static_cast<cIGZUnknown*>(n));
        uint32_t cx = 0, cz = 0;
        n->GetOccupiedCell(cx, cz);  // slot 59
        entry.x = static_cast<int32_t>(cx);
        entry.z = static_cast<int32_t>(cz);
        listed.push_back(entry);
        // The list's entries are AddRef'd (§7.2 step 2.1); the core releases
        // them through the host's `release`.
        FC4_ADDREF(static_cast<cIGZUnknown*>(n));
    }
    fc4_nettool_destroy_occupants_and_resolve(core_, listed.data(), static_cast<uint32_t>(listed.size()));
    LogStatus("DestroyOccupantsAndResolve");
    FC4_X87_CHECK("nettool DestroyOccupantsAndResolve");
}

cIGZUnknown* CreateFC4NetworkToolEntry() {
    // The subsystem table's factory creates the generic tool (type 0) with
    // reference count 0; the class itself is not the simulation path — the
    // registry's tools are (cFC4NetworkTool.h).
    return cFC4NetworkTool::Create(0);
}

// ---- the tool registry -----------------------------------------------------------------------------

namespace NetToolRegistry {

namespace {

// One shared tool per network type (the manager's shared-tool cache).
cFC4NetworkTool* g_tools[13] = {};
bool g_enabled = false;

}  // namespace

bool Enabled() { return g_enabled; }

void SetEnabled(bool on) { g_enabled = on; }

cFC4NetworkTool* Take(int32_t type, cISC4City* city, cIGZMessageServer2* ms) {
    if (!g_enabled || type < 0 || type >= 13) return nullptr;
    cFC4NetworkTool*& slot = g_tools[type];
    if (slot == nullptr) {
        slot = new cFC4NetworkTool(static_cast<uint32_t>(type));
        slot->AddRef();  // the registry's reference
        fc4::Log("nettool: shared tool of type %d created", type);
    }
    if (city != nullptr) slot->SetCity(city);
    if (ms != nullptr) slot->SetMessageServer(ms);
    return slot;
}

void CityShutdown() {
    for (int t = 0; t < 13; ++t) {
        if (g_tools[t] == nullptr) continue;
        g_tools[t]->Shutdown();  // tool slot 4
        g_tools[t]->Release();   // the registry's reference
        g_tools[t] = nullptr;
    }
}

void LogSummary(const char* why) {
    uint32_t live = 0;
    for (int t = 0; t < 13; ++t) {
        if (g_tools[t] != nullptr) ++live;
    }
    if (live > 0 || g_enabled) {
        fc4::Log("nettool %s: %u live tool(s)", why, live);
    }
}

}  // namespace NetToolRegistry
