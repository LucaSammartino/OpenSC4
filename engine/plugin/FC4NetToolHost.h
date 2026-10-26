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

// The network tool's host table (fc4.h fc4_nettool_host, 36 callbacks;
// docs/specs/network-tools.md §3-§11) over the game's interfaces, wired the
// way cores27's host.rs names them:
//   - the drag, the passes/solve and the smoothing go through the geometry
//     core (fc4_netgeom, a bound handle in the context);
//   - the §3.3 probes, the §6 commit and the §10 destroy operations go
//     through the occupant manager and the game's network occupants;
//   - the §11 underground refit reads the terrain through the city.
//
// Tokens: an occupant token is its cISC4NetworkOccupant pointer (QI
// 0xA821EF94); a callback-tool token is its cIGZUnknown pointer. Tokens
// handed to the core are BORROWED; every query's own QI reference is
// released at its scope end (run 220: the held-list released references the
// callers had already released — a double release per query hit). Main thread.
#pragma once

#include "fc4.h"
#include "cIGZUnknown.h"
#include "SC4List.h"

#include <cstddef>
#include <cstdint>

class cIGZMessageServer2;
class cISC4Occupant;
class cISC4City;

namespace fc4 {

struct NetToolContext {
    // The city (borrowed: the tool lives inside one city's lifetime).
    cISC4City* city = nullptr;
    // The bound geometry core (fc4_netgeom) driving the §5.1 pipeline; null
    // = the pipeline answers an empty world (the tool still runs its state).
    fc4_netgeom* geom = nullptr;
    // The message server used by the §10 synchronous pre-demolish send.
    cIGZMessageServer2* msgServer = nullptr;
    // The owning cFC4NetworkTool (primary cIGZUnknown*), for the callback
    // tool of §3.3 step 6.2 (joins) and the §10 RemoveOrReplaceOccupant.
    void* self = nullptr;


    // Observability: the counters the log line reports.
    uint32_t draws = 0;            // draw_network_line replays on the geom core
    uint32_t solves = 0;           // validate_and_solve calls
    uint32_t inserts = 0;          // real InsertOccupant calls (data4 = run.data4)
    uint32_t insertsRefused = 0;   // null occupant / manager
    uint32_t removed = 0;          // §10 removals
    uint32_t connections = 0;      // §3.3 one-cell joins
};

// NT32/NT33: bounded read-only selection on fresh straight powerline cells.
// False means unavailable; no placement/model host or native totals are used.
bool ReadNetToolPowerlineSelection(NetToolContext* ctx, fc4_powerline_selection* out);

// Layout pin: cFC4NetworkTool embeds this struct and FC4NetToolHost.cpp reads
// it, so every translation unit that includes this header checks the same
// size and offsets (a change here must rebuild both, see glue-netrules-impl4).
static_assert(sizeof(NetToolContext) == 4 * sizeof(void*) + 6 * sizeof(uint32_t), "NetToolContext layout changed");
static_assert(offsetof(NetToolContext, self) == 3 * sizeof(void*), "NetToolContext layout changed");
static_assert(offsetof(NetToolContext, connections) == 4 * sizeof(void*) + 5 * sizeof(uint32_t),
              "NetToolContext layout changed");

// The host table over `ctx` (which must outlive the core). Fills every
// callback the core can reach on the simulation paths; the presentation-only
// ones (highlights, previews, draft) stay NULL like glue24's empty defaults.
fc4_nettool_host MakeNetToolHost(NetToolContext* ctx);

// The netgeom host table (the geometry core's own view: terrain via the
// city's cISTETerrain, world via the occupant manager).
fc4_netgeom_host MakeNetGeomHost(NetToolContext* ctx);

// Read-only resource boundary diagnostics for the opt-in NT31 replay.
void LogNetToolReplayResources();

// ---- verified-order game interfaces (docs/abi/interfaces.tsv) ------------------------------------

// cISC4NetworkManager, slots 3-8 (the §10 path calls slots 5, 6 and 8). Own
// shape: FC4DemolitionHost.h's cFC4NetworkManager stops at slot 7, and we
// must not edit that shared file for slot 8.
class cFC4NetworkToolManager : public cIGZUnknown {
public:
    virtual bool NmInit() = 0;                                        // 3  [abi-ret] eax-or-void
    virtual bool NmShutdown() = 0;                                    // 4  [abi-ret] eax-or-void
    virtual void* GetNetworkTool(int32_t type, bool unique) = 0;      // 5  eax
    virtual void DestroyNetworkOccupants(SC4List<cISC4Occupant*>& list) = 0;  // 6 void
    virtual void QueueRemovals(bool on) = 0;                          // 7  void
    virtual bool RemoveOrReplaceOccupant(void* tool, cISC4Occupant* occ, bool& replaced) = 0;  // 8 al
};

// cISC4NetworkOccupant's extended underground view (vtable 0x00AAAD40, 106
// slots: the base 81 plus the underground tail). Only the slots the §11
// refit calls are named (network-occupants.md §3.1, network-tools.md §11);
// the rest are numbered placeholders so the vtable keeps the game's order.
class cFC4UndergroundOccupant : public cIGZUnknown {
public:
    // 0..49 are the shared cISC4NetworkOccupant slots (not called here).
    virtual void UGSlot00() = 0;
    virtual void UGSlot01() = 0;
    virtual void UGSlot02() = 0;
    virtual void UGSlot03() = 0;
    virtual void UGSlot04() = 0;
    virtual void UGSlot05() = 0;
    virtual void UGSlot06() = 0;
    virtual void UGSlot07() = 0;
    virtual void UGSlot08() = 0;
    virtual void UGSlot09() = 0;
    virtual void UGSlot10() = 0;
    virtual void UGSlot11() = 0;
    virtual void UGSlot12() = 0;
    virtual void UGSlot13() = 0;
    virtual void UGSlot14() = 0;
    virtual void UGSlot15() = 0;
    virtual void UGSlot16() = 0;
    virtual void UGSlot17() = 0;
    virtual void UGSlot18() = 0;
    virtual void UGSlot19() = 0;
    virtual void UGSlot20() = 0;
    virtual void UGSlot21() = 0;
    virtual void UGSlot22() = 0;
    virtual void UGSlot23() = 0;
    virtual void UGSlot24() = 0;
    virtual void UGSlot25() = 0;
    virtual void UGSlot26() = 0;
    virtual void UGSlot27() = 0;
    virtual void UGSlot28() = 0;
    virtual void UGSlot29() = 0;
    virtual void UGSlot30() = 0;
    virtual void UGSlot31() = 0;
    virtual void UGSlot32() = 0;
    virtual void UGSlot33() = 0;
    virtual void UGSlot34() = 0;
    virtual void UGSlot35() = 0;
    virtual void UGSlot36() = 0;
    virtual void UGSlot37() = 0;
    virtual void UGSlot38() = 0;
    virtual void UGSlot39() = 0;
    virtual void UGSlot40() = 0;
    virtual void UGSlot41() = 0;
    virtual void UGSlot42() = 0;
    virtual void UGSlot43() = 0;
    virtual void UGSlot44() = 0;
    virtual void UGSlot45() = 0;
    virtual void UGSlot46() = 0;
    virtual void UGSlot47() = 0;
    virtual void UGSlot48() = 0;
    virtual void UGSlot49() = 0;
    virtual void ReleasePathInfo() = 0;              // 50
    // 51..83 are not called here.
    virtual void UGSlot51() = 0;
    virtual void UGSlot52() = 0;
    virtual void UGSlot53() = 0;
    virtual void UGSlot54() = 0;
    virtual void UGSlot55() = 0;
    virtual void UGSlot56() = 0;
    virtual void UGSlot57() = 0;
    virtual void UGSlot58() = 0;
    virtual void UGSlot59() = 0;
    virtual void UGSlot60() = 0;
    virtual void UGSlot61() = 0;
    virtual void UGSlot62() = 0;
    virtual void UGSlot63() = 0;
    virtual void UGSlot64() = 0;
    virtual void UGSlot65() = 0;
    virtual void UGSlot66() = 0;
    virtual void UGSlot67() = 0;
    virtual void UGSlot68() = 0;
    virtual void UGSlot69() = 0;
    virtual void UGSlot70() = 0;
    virtual void UGSlot71() = 0;
    virtual void UGSlot72() = 0;
    virtual void UGSlot73() = 0;
    virtual void UGSlot74() = 0;
    virtual void UGSlot75() = 0;
    virtual void UGSlot76() = 0;
    virtual void UGSlot77() = 0;
    virtual void UGSlot78() = 0;
    virtual void UGSlot79() = 0;
    virtual void UGSlot80() = 0;
    virtual void UGSlot81() = 0;
    virtual void UGSlot82() = 0;
    virtual void UGSlot83() = 0;
    virtual void SetVertexHeights(const float heights[4]) = 0;   // 84
    virtual void UGSlot85() = 0;
    virtual void CreateGeometry(bool rebuild) = 0;               // 86
    virtual void UGSlot87() = 0;
    virtual void UGSlot88() = 0;
    virtual void UGSlot89() = 0;
    virtual void UGSlot90() = 0;
    virtual void UGSlot91() = 0;
    virtual void UGSlot92() = 0;
    virtual void UGSlot93() = 0;
    virtual void GetTerrainHeights(float out[4]) = 0;            // 94
    virtual void SetTerrainHeights(const float heights[4]) = 0;  // 95
};

intptr_t NetToolOccupantViewsTest(cIGZUnknown* network, cIGZMessageServer2* server);

}  // namespace fc4
