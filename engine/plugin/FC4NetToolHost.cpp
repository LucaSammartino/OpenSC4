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

// The network tool's host table: see FC4NetToolHost.h.

#include "FC4NetToolHost.h"

#include "FC4DemolitionHost.h"
#include "FC4Fpu.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"

#include "SC4List.h"
#include "SC4Point.h"
#include "cFC4Message2Standard.h"
#include "cIGZCOM.h"
#include "cRZCOMDllDirector.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4City.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantFilter.h"
#include "cISC4OccupantManager.h"
#include "cISTETerrain.h"
#include "cS3DVector3.h"

#include <cstring>
#include <utility>

namespace fc4 {

namespace {

constexpr uint32_t kIID_cISC4NetworkOccupant = 0xA821EF94;  // network-tools.md §3.3, §10
constexpr uint32_t kIID_cISC4NetworkTunnel = 0x8A4BD52B;    // §10: the paired portal
constexpr uint32_t kIID_cISC4UndergroundOccupant = 0x4A10AFC2;  // the extended underground interface (§1.2)
constexpr uint32_t kCLSID_cSC4NetworkOccupant = 0xC9C05C6E;  // network-occupants.md §1.1
constexpr uint32_t kMsgPreDemolishNetworkOccupant = 0xAA7A014D;  // §10 step 5.1

NetToolContext* Ctx(void* u) { return static_cast<NetToolContext*>(u); }
cISC4City* City(void* u) { return Ctx(u)->city; }

cISC4NetworkOccupant* NetOf(uint64_t t) { return reinterpret_cast<cISC4NetworkOccupant*>(FromToken(t)); }
cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }

cISC4OccupantManager* OMgr(void* u) {
    cISC4City* c = City(u);
    return c ? c->GetOccupantManager() : nullptr;
}

cISTETerrain* Terrain(void* u) {
    cISC4City* c = City(u);
    return c ? c->GetTerrain() : nullptr;
}

// QueryInterface with the reference for the caller (0 = it does not answer).
uint64_t QueryToken(uint64_t t, uint32_t iid) {
    if (t == 0) return 0;
    cIGZUnknown* u = FromToken(t);
    void* p = nullptr;
    if (!u->QueryInterface(iid, &p) || p == nullptr) return 0;
    cIGZUnknown* q = static_cast<cIGZUnknown*>(p);
    if (!IsOurObject(q)) FC4_ACQUIRED(q, "QueryInterface");
    return TokenOf(q);
}

void HoldToken(NetToolContext* ctx, uint64_t t) {
    if (t != 0) ctx->held.push_back(FromToken(t));
}

// The lowest network type bit shared by `flags` and the mask (0-12), or 0xFFFF_FFFF.
uint32_t TypeOfFlags(uint32_t flags) {
    uint32_t bits = flags & 0x1FFFu;
    for (uint32_t t = 0; t < 13; ++t) {
        if (bits & (1u << t)) return t;
    }
    return 0xFFFFFFFFu;
}

// ---- the drag and the pipeline (§3.2, §5) ----------------------------------------------------------

// Slot 18 DrawNetworkLine over the run: the geometry core replays the drag
// (§3.1-§3.3 of network-geometry.md) and fills dragged_count / straight_count
// / cells. false = the geometry core is absent.
bool DrawNetworkLine(void* u, fc4_nettool_run* run, bool snap) {
    NetToolContext* c = Ctx(u);
    if (c == nullptr || c->geom == nullptr || run == nullptr) return false;
    const bool ok = fc4_netgeom_draw_line(
        c->geom, run->type_id, run->start_x, run->start_z, run->end_x, run->end_z, snap);
    // Pull the geometry's counters and cells back into the run (§3.1 outputs).
    fc4_netgeom_state gs{};
    if (fc4_netgeom_get_state(c->geom, &gs)) {
        run->dragged_count = gs.dragged_count;
        run->straight_count = gs.straight_count;
        run->end_x = gs.end_x;
        run->end_z = gs.end_z;
    }
    fc4_netgeom_cell cells[FC4_NETTOOL_MAX_CELLS];
    const uint32_t n = fc4_netgeom_get_cells(c->geom, cells, FC4_NETTOOL_MAX_CELLS);
    run->cell_count = 0;
    for (uint32_t i = 0; i < n && i < FC4_NETTOOL_MAX_CELLS; ++i) {
        run->cells[run->cell_count] = fc4_nettool_cell{cells[i].x, cells[i].z};
        run->cell_count++;
    }
    ++c->draws;
    FC4_X87_CHECK("nettool DrawNetworkLine");
    return ok;
}

// §5.1 steps 4-5 through the geometry core: the validation passes (§4), the
// cell infos (§8.1), the smoothing loop (§9.4) and the terrain support test
// (§9.6). The failure code travels in fc4_netgeom_state.failure.
bool ValidateAndSolve(void* u, fc4_nettool_run* run, bool real, fc4_nettool_pass* out) {
    NetToolContext* c = Ctx(u);
    if (c == nullptr || c->geom == nullptr || run == nullptr) return false;
    (void)real;
    ++c->solves;
    const uint32_t valid = fc4_netgeom_place(c->geom);
    if (out != nullptr) {
        fc4_netgeom_state gs{};
        fc4_netgeom_get_state(c->geom, &gs);
        out->valid_count = valid;
        out->failure = gs.failure;
        // The solved cells feed §6's inserters (the run carries them).
        fc4_netgeom_cell solved[FC4_NETTOOL_MAX_CELLS];
        const uint32_t n = fc4_netgeom_get_solved(c->geom, solved, FC4_NETTOOL_MAX_CELLS);
        run->cell_count = 0;
        for (uint32_t i = 0; i < n && i < FC4_NETTOOL_MAX_CELLS; ++i) {
            run->cells[run->cell_count] = fc4_nettool_cell{solved[i].x, solved[i].z};
        run->cell_count++;
        }
        run->valid_count = valid;
    }
    return valid > 0;
}

// §3.3 step 4.1: GetNetworkOccupant(x, z) with the OR filter on `mask`
// (occupant manager slot 32 + QI 0xA821EF94): the occupant's network type,
// 0xFFFFFFFF = none. `mask` is a network FLAG mask (0x1FFF bits), so the
// manager's one-type thunk cannot express it: query the cell unfiltered and
// test the flags here.
uint32_t GetNetworkOccupant(void* u, int32_t x, int32_t z, uint32_t mask) {
    NetToolContext* c = Ctx(u);
    cISC4OccupantManager* om = OMgr(u);
    if (c == nullptr || om == nullptr) return 0xFFFFFFFFu;
    cISC4Occupant* o = nullptr;
    if (!om->GetFirstOccupantByStandardCityCell(o, x, z, static_cast<cISC4OccupantFilter*>(nullptr)) ||
        o == nullptr) {
        return 0xFFFFFFFFu;
    }
    FC4_ACQUIRED(o, "GetFirstOccupantByStandardCityCell");
    uint32_t answer = 0xFFFFFFFFu;
    cISC4NetworkOccupant* n = nullptr;
    if (o->QueryInterface(kIID_cISC4NetworkOccupant, reinterpret_cast<void**>(&n)) && n != nullptr) {
        FC4_ACQUIRED(n, "QueryInterface");
        HoldToken(c, TokenOf(n));
        if ((n->GetNetworkFlag() & mask) != 0) {
            answer = TypeOfFlags(n->GetNetworkFlag() & mask);
        }
        FC4_RELEASE(n);
    }
    FC4_RELEASE(o);
    return answer;
}

// §3.3 step 6.2: the callback tool's PlaceNetworkOccupants (one-cell join in
// the callback tool's own type).
void CallbackPlace(void* u, uint32_t callback, int32_t ax, int32_t az, int32_t bx, int32_t bz, bool real) {
    NetToolContext* c = Ctx(u);
    if (c == nullptr || callback == 0) return;
    cIGZUnknown* tool = FromToken(callback);
    if (tool == nullptr) return;
    cFC4ISC4NetworkTool* t = reinterpret_cast<cFC4ISC4NetworkTool*>(tool);
    FC4_ADDREF(tool);
    t->Init();  // slot 3
    const uint32_t r = t->ToolSlot07(
        static_cast<uint32_t>(ax), static_cast<uint32_t>(az), static_cast<uint32_t>(bx),
        static_cast<uint32_t>(bz), 0u, 0u, real ? 1u : 0u, 0xFFFFFFFFu);
    (void)r;
    t->Shutdown();  // slot 4
    FC4_RELEASE(tool);
    ++c->connections;
    FC4_X87_CHECK("nettool CallbackPlace");
}

// ---- the commit (§6) -------------------------------------------------------------------------------

// A network occupant through GZCOM (class 0xC9C05C6E, QI 0xA821EF94), one
// reference for the caller; nullptr + one log when GZCOM refuses it.
bool g_createWarned = false;

cISC4NetworkOccupant* NewNetworkOccupant() {
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* p = nullptr;
    if (com == nullptr || !com->GetClassObject(kCLSID_cSC4NetworkOccupant, kIID_cISC4NetworkOccupant, &p) ||
        p == nullptr) {
        if (!g_createWarned) {
            g_createWarned = true;
            Log("nettool: WARNING could not create a network occupant through GZCOM (class 0xC9C05C6E)");
        }
        return nullptr;
    }
    auto* n = static_cast<cISC4NetworkOccupant*>(p);
    FC4_ACQUIRED(n, "GetClassObject");
    return n;
}

// §6.2 step 3's insert through the occupant manager: position the piece on
// its cell, insert with the run's data4, then the §6.3 usable marking.
void PlaceOccupant(NetToolContext* c, cISC4OccupantManager* om, cISC4NetworkOccupant* n, int32_t x, int32_t z,
                   uint32_t type_id, uint32_t data4, bool usable) {
    n->SetNetworkFlag(1u << (type_id & 0x1F));  // slot 10: the type bit
    cISC4Occupant* occ = n->AsOccupant();       // slot 48
    if (occ == nullptr) {
        ++c->insertsRefused;
        return;
    }
    cS3DVector3 pos;
    pos.fX = static_cast<float>(x) * 16.0f + 8.0f;
    pos.fY = 0.0f;
    pos.fZ = static_cast<float>(z) * 16.0f + 8.0f;
    occ->SetPosition(&pos);  // occupant slot 9
    om->InsertOccupant(occ, data4);  // slot 20: the listeners see 0x99EF1142 with data4
    ++c->inserts;
    // §6.3 (release = true): usable at once (quirk 5). The 0x89D3E9F3 post is
    // the occupant's own SetNetworkFlag (network-manager.md §5).
    if (usable) {
        n->SetUsable(true);             // slot 20
        n->ClearNetworkFlag(0x4000u);   // slot 11
    } else {
        n->SetNetworkFlag(0x4000u);     // slot 11 (drawn translucent)
    }
    FC4_RELEASE(occ);
}

// §6.1 step 2.1 / §6.2: for each solved cell, insert the solved piece into
// the city through the occupant manager. The run carries data4 (+0x208).
// [choice] The per-cell piece choice (network-rules.md §11) is not wired
// here: every solved cell inserts one ordinary network occupant whose piece
// id stays 0 (the plain textured-cell class). The piece solver is a
// follow-up; the §3/§5/§6 state machine and the message order are the goal.
void InsertOccupantsForSolvedCells(void* u, const fc4_nettool_run* run, bool preview_only) {
    NetToolContext* c = Ctx(u);
    if (c == nullptr || run == nullptr || preview_only) return;
    cISC4OccupantManager* om = OMgr(u);
    if (om == nullptr) return;
    for (uint32_t i = 0; i < run->cell_count && i < FC4_NETTOOL_MAX_CELLS; ++i) {
        cISC4NetworkOccupant* n = NewNetworkOccupant();
        if (n == nullptr) {
            ++c->insertsRefused;
            continue;
        }
        PlaceOccupant(c, om, n, run->cells[i].x, run->cells[i].z, run->type_id, run->data4, true);
        FC4_RELEASE(n);
    }
    NetToolReleaseHeld(c);
}

// §6.3 MarkOccupantsUsable(set, release): the inserts above already marked
// the pieces usable (quirk 5); the §10 rebuild's re-walk lands here too.
void MarkOccupantsUsable(void* u, const fc4_nettool_run* run, bool release) {
    NetToolContext* c = Ctx(u);
    (void)release;
    if (c == nullptr || run == nullptr) return;
    // Nothing extra: SetUsable ran inside PlaceOccupant.
}

// §4 PlaceNetworkOccupantById: one piece at one cell, without a solve. The
// core owns the real-byte bracket; this inserts with the run's data4 (1).
void InsertNetworkOccupantById(void* u, const fc4_nettool_run* run, int32_t x, int32_t z, uint32_t piece,
                               bool usable, uint32_t type_id) {
    NetToolContext* c = Ctx(u);
    if (c == nullptr || run == nullptr) return;
    cISC4OccupantManager* om = OMgr(u);
    if (om == nullptr) {
        ++c->insertsRefused;
        return;
    }
    cISC4NetworkOccupant* n = NewNetworkOccupant();
    if (n == nullptr) {
        ++c->insertsRefused;
        return;
    }
    n->SetQuadDraw(piece, 0);  // slot 30: the requested piece id (0x100 for connection nodes)
    PlaceOccupant(c, om, n, x, z, type_id, run->data4, usable);
    FC4_RELEASE(n);
    NetToolReleaseHeld(c);
}

// ---- §10 DestroyOccupantsAndResolve ----------------------------------------------------------------

// Network occupant slot 59 GetOccupiedCell.
bool OccupiedCell(void* u, uint64_t occ, fc4_nettool_cell* out) {
    (void)u;
    cISC4NetworkOccupant* n = NetOf(occ);
    if (n == nullptr || out == nullptr) return false;
    uint32_t cx = 0, cz = 0;
    n->GetOccupiedCell(cx, cz);  // slot 59
    out->x = static_cast<int32_t>(cx);
    out->z = static_cast<int32_t>(cz);
    return true;
}

// The network occupant of a standard city cell (slot 32, unfiltered), held:
// the cell's pieces for the §10 closure walk. Returns the count written.
uint32_t CellNetworkOccupants(NetToolContext* c, cISC4OccupantManager* om, int32_t x, int32_t z, uint32_t mask,
                              uint64_t* out, uint32_t capacity, uint8_t* immovable, uint8_t* intersection) {
    cISC4Occupant* o = nullptr;
    if (!om->GetFirstOccupantByStandardCityCell(o, x, z, static_cast<cISC4OccupantFilter*>(nullptr)) ||
        o == nullptr) {
        return 0;
    }
    FC4_ACQUIRED(o, "GetFirstOccupantByStandardCityCell");
    uint32_t written = 0;
    cISC4NetworkOccupant* n = nullptr;
    if (o->QueryInterface(kIID_cISC4NetworkOccupant, reinterpret_cast<void**>(&n)) && n != nullptr) {
        FC4_ACQUIRED(n, "QueryInterface");
        const uint32_t flags = n->GetNetworkFlag();
        if ((flags & mask) != 0) {
            HoldToken(c, TokenOf(n));
            if (written < capacity && out != nullptr) {
                out[written] = TokenOf(n);
                if (immovable) immovable[written] = (flags & 0x400000u) != 0 ? 1 : 0;    // §10: flag 0x400000
                if (intersection) intersection[written] = (flags & 0x100000u) != 0 ? 1 : 0;  // §10: flag 0x100000
            }
            ++written;
        }
        FC4_RELEASE(n);
    }
    FC4_RELEASE(o);
    return written;
}

// §10 step 3: the occupant's neighbours by network type — the network
// occupants of the four cells around its own that share its type bit. Writes
// up to `capacity` neighbours, returns the total.
uint32_t NeighboursByNetworkType(void* u, uint64_t occ, fc4_nettool_neighbour* out, uint32_t capacity) {
    NetToolContext* c = Ctx(u);
    cISC4NetworkOccupant* n = NetOf(occ);
    cISC4OccupantManager* om = OMgr(u);
    if (c == nullptr || n == nullptr || om == nullptr || out == nullptr || capacity == 0) return 0;
    uint32_t cx = 0, cz = 0;
    n->GetOccupiedCell(cx, cz);  // slot 59
    const int32_t x = static_cast<int32_t>(cx);
    const int32_t z = static_cast<int32_t>(cz);
    const uint32_t mask = n->GetNetworkFlag() & 0x1FFFu;
    if (mask == 0) return 0;
    static const int32_t dx[4] = {-1, 0, 1, 0};
    static const int32_t dz[4] = {0, -1, 0, 1};
    uint32_t count = 0;
    for (int i = 0; i < 4; ++i) {
        uint64_t tokens[8];
        uint8_t imm[8] = {};
        uint8_t inter[8] = {};
        const uint32_t got = CellNetworkOccupants(c, om, x + dx[i], z + dz[i], mask, tokens, 8, imm, inter);
        for (uint32_t k = 0; k < got; ++k) {
            if (tokens[k] == occ) continue;  // never list the piece itself
            if (count < capacity) {
                out[count].occ = tokens[k];
                out[count].x = x + dx[i];
                out[count].z = z + dz[i];
                out[count].immovable = imm[k];
                out[count].intersection = inter[k];
                std::memset(out[count].reserved, 0, sizeof(out[count].reserved));
            }
            ++count;
        }
    }
    return count;
}

// Flag 0x100000: part of an intersection piece. [abi-ret] slot 55
// IsIntersection: row says eax-or-void — the bool the plugin reads is fine
// (same use as FC4NetMgrHost.cpp; the flag byte is what matters).
bool IsIntersectionPiece(void* u, uint64_t occ) {
    (void)u;
    cISC4NetworkOccupant* n = NetOf(occ);
    return n != nullptr && n->IsIntersection();  // slot 55 [abi-ret] eax-or-void: only logged
}

// Network manager slot 12 GetAllIntersectionPieces: a flood over the
// 4-neighbours from the start piece's cell collecting every 0x100000-flagged
// occupant of the same type (§7.9).
uint32_t IntersectionStructure(void* u, uint64_t occ, uint64_t* out, uint32_t capacity) {
    NetToolContext* c = Ctx(u);
    cISC4NetworkOccupant* start = NetOf(occ);
    cISC4OccupantManager* om = OMgr(u);
    if (c == nullptr || start == nullptr || om == nullptr || out == nullptr || capacity == 0) return 0;
    uint32_t sx = 0, sz = 0;
    start->GetOccupiedCell(sx, sz);
    const uint32_t mask = start->GetNetworkFlag() & 0x1FFFu;
    if (mask == 0) return 0;
    uint32_t count = 0;
    std::vector<uint8_t> seen(256 * 256, 0);
    std::vector<std::pair<int32_t, int32_t>> queue;
    queue.reserve(256);
    queue.emplace_back(static_cast<int32_t>(sx), static_cast<int32_t>(sz));
    seen[(sx & 0xFF) * 256 + (sz & 0xFF)] = 1;
    size_t head = 0;
    while (head < queue.size()) {
        const int32_t x = queue[head].first;
        const int32_t z = queue[head].second;
        ++head;
        uint64_t tokens[16];
        const uint32_t got = CellNetworkOccupants(c, om, x, z, mask, tokens, 16, nullptr, nullptr);
        for (uint32_t k = 0; k < got; ++k) {
            cISC4NetworkOccupant* m = NetOf(tokens[k]);
            if (m == nullptr || !(m->GetNetworkFlag() & 0x100000u)) continue;
            if (count < capacity) out[count] = tokens[k];
            ++count;
            uint32_t mx = 0, mz = 0;
            m->GetOccupiedCell(mx, mz);
            static const int32_t ddx[4] = {-1, 1, 0, 0};
            static const int32_t ddz[4] = {0, 0, -1, 1};
            for (int d = 0; d < 4; ++d) {
                const int32_t ax = static_cast<int32_t>(mx) + ddx[d];
                const int32_t az = static_cast<int32_t>(mz) + ddz[d];
                if (ax < 0 || az < 0 || ax > 255 || az > 255) continue;
                const size_t key = static_cast<size_t>(ax & 0xFF) * 256 + static_cast<size_t>(az & 0xFF);
                if (seen[key]) continue;
                seen[key] = 1;
                queue.emplace_back(ax, az);
            }
        }
    }
    return count;
}

// Flag 0x80000: a tunnel portal (network-geometry.md §10.3).
bool IsTunnelPortal(void* u, uint64_t occ) {
    (void)u;
    cISC4NetworkOccupant* n = NetOf(occ);
    if (n == nullptr) return false;
    return (n->GetNetworkFlag() & 0x80000u) != 0;
}

// The portal at the other end (QI 0x8A4BD52B); 0 = none.
uint64_t PairedPortal(void* u, uint64_t occ) {
    (void)u;
    cISC4NetworkOccupant* n = NetOf(occ);
    if (n == nullptr) return 0;
    return QueryToken(occ, kIID_cISC4NetworkTunnel);
}

// §10 step 5.1: MessageSend of 0xAA7A014D (synchronous), IGZUnknown = the
// occupant. Its neighbours answer by dropping their connection to it.
void SendPreDemolishSync(void* u, uint64_t occ) {
    NetToolContext* c = Ctx(u);
    cISC4Occupant* o = OccOf(occ);
    if (c == nullptr || o == nullptr) return;
    cIGZMessageServer2* ms = c->msgServer;
    cIGZMessageServer2Ptr fallback;
    if (ms == nullptr) ms = fallback;
    if (ms == nullptr) return;
    cFC4Message2Standard* msg = FC4MakeMessage(kMsgPreDemolishNetworkOccupant, 0, 0, 0);
    if (msg == nullptr) return;
    msg->SetData1(reinterpret_cast<intptr_t>(o));
    msg->SetVoid1(o);
    ms->MessageSend(msg);
    FC4_X87_CHECK("nettool SendPreDemolishSync");
    msg->Release();
}

// §10 step 5.3 (destroyed cell): occupantManager.RemoveOccupant(o, false, 0).
void RemoveOccupant(void* u, uint64_t occ, bool shutdown) {
    NetToolContext* c = Ctx(u);
    cISC4OccupantManager* om = OMgr(u);
    cISC4Occupant* o = OccOf(occ);
    if (c == nullptr || om == nullptr || o == nullptr) return;
    om->RemoveOccupant(o, shutdown, 0);  // slot 21
    ++c->removed;
}

// §10 step 5.3 (closure piece): RemoveOrReplaceOccupant — true when a plain
// piece was put back. The city's network manager owns the logic (slot 8);
// with `enable = netmgr` ours answers it, else the original's does.
bool RemoveOrReplaceOccupant(void* u, uint64_t occ) {
    NetToolContext* c = Ctx(u);
    cISC4City* city = City(u);
    cISC4Occupant* o = OccOf(occ);
    if (c == nullptr || city == nullptr || o == nullptr) return false;
    cISC4NetworkManager* nm = city->GetNetworkManager();  // slot 38, borrowed
    if (nm == nullptr) return false;
    bool replaced = false;
    reinterpret_cast<cFC4NetworkToolManager*>(nm)->RemoveOrReplaceOccupant(
        static_cast<cIGZUnknown*>(c->self), o, replaced);  // slot 8 [abi-ret] al
    if (replaced) ++c->removed;
    return replaced;
}

// §10 step 5.4: AsOccupant().Shutdown(); the reference release is the core's.
void OccupantShutdown(void* u, uint64_t occ) {
    (void)u;
    cISC4Occupant* o = OccOf(occ);
    if (o != nullptr) o->Shutdown();  // occupant slot 4
}

void OccupantRelease(void* u, uint64_t occ) {
    (void)u;
    cIGZUnknown* o = FromToken(occ);
    if (o != nullptr) FC4_RELEASE(o);
}

// §10 step 6.1: SolveNetwork(−1, true) over the re-solve set — the geometry
// core replays it; the solved cells come back for §10 step 6.2's inserts.
void SolveNetworkResolve(void* u, fc4_nettool_run* run) {
    NetToolContext* c = Ctx(u);
    if (c == nullptr || c->geom == nullptr || run == nullptr) return;
    ++c->solves;
    fc4_netgeom_place(c->geom);
    fc4_netgeom_cell solved[FC4_NETTOOL_MAX_CELLS];
    const uint32_t n = fc4_netgeom_get_solved(c->geom, solved, FC4_NETTOOL_MAX_CELLS);
    run->cell_count = 0;
    for (uint32_t i = 0; i < n && i < FC4_NETTOOL_MAX_CELLS; ++i) {
        run->cells[run->cell_count] = fc4_nettool_cell{solved[i].x, solved[i].z};
        run->cell_count++;
    }
}

// ---- §11 the underground refit ---------------------------------------------------------------------

// Terrain slot 17: the cell's current corner altitudes.
void CornerAltitudes(void* u, int32_t x, int32_t z, float out[4]) {
    cISTETerrain* t = Terrain(u);
    if (t == nullptr || out == nullptr) return;
    t->GetCellCornerAltitudes(static_cast<uint32_t>(x), static_cast<uint32_t>(z), out);  // slot 17
}

// §11.1 step 3: SmoothenNetwork(level) over the solved records — the
// geometry core's smoothing loop.
bool SmoothenUnderground(void* u, fc4_nettool_run* run, int32_t level) {
    NetToolContext* c = Ctx(u);
    if (c == nullptr || c->geom == nullptr || run == nullptr) return false;
    ++c->solves;
    return fc4_netgeom_smoothen(c->geom, level);
}

// §11.1 step 4: FindUndergroundOccupant(x, z) finding a piece: the cell's
// occupant with the smoothed corner heights read back from the geometry's
// network satisfier at the cell's four vertices.
bool FindUndergroundSolved(void* u, int32_t x, int32_t z, fc4_nettool_solved* out) {
    NetToolContext* c = Ctx(u);
    cISC4OccupantManager* om = OMgr(u);
    if (c == nullptr || om == nullptr || out == nullptr) return false;
    cISC4Occupant* o = nullptr;
    if (!om->GetFirstOccupantByStandardCityCell(o, x, z, static_cast<cISC4OccupantFilter*>(nullptr)) ||
        o == nullptr) {
        return false;
    }
    FC4_ACQUIRED(o, "GetFirstOccupantByStandardCityCell");
    bool found = false;
    cISC4NetworkOccupant* n = nullptr;
    if (o->QueryInterface(kIID_cISC4NetworkOccupant, reinterpret_cast<void**>(&n)) && n != nullptr) {
        FC4_ACQUIRED(n, "QueryInterface");
        out->occ = TokenOf(n);
        HoldToken(c, TokenOf(n));
        if (c->geom != nullptr) {
            // The cell's four vertices in the geometry's satisfier (§11.1
            // step 4.1: cSC4VertexHtConstraintSatisfier::GetHeight).
            cISC4City* city = City(u);
            const int32_t stride = static_cast<int32_t>(city ? city->CellCountX() : 64u) + 1;
            float h = 0.0f;
            for (int k = 0; k < 4; ++k) {
                static const int32_t vx[4] = {0, 0, 1, 1};
                static const int32_t vz[4] = {0, 1, 1, 0};
                const int32_t vertex = (x + vx[k]) + (z + vz[k]) * stride;
                out->corners[k] = fc4_netgeom_sat_height(c->geom, FC4_NETGEOM_SAT_NETWORK, vertex, &h) ? h : 0.0f;
            }
            found = true;
        }
        FC4_RELEASE(n);
    }
    FC4_RELEASE(o);
    return found;
}

// §11.1 step 5.4: DestroyNetworkOccupants / ReleaseOccupantList — through
// the city's network manager (slot 6) and per-reference release.
void DestroyNetworkOccupants(void* u, const uint64_t* occ, uint32_t count) {
    NetToolContext* c = Ctx(u);
    cISC4City* city = City(u);
    if (c == nullptr || city == nullptr || occ == nullptr || count == 0) return;
    cISC4NetworkManager* nm = city->GetNetworkManager();  // slot 38, borrowed
    if (nm == nullptr) return;
    SC4List<cISC4Occupant*> list;
    for (uint32_t i = 0; i < count; ++i) {
        cISC4Occupant* o = OccOf(occ[i]);
        if (o != nullptr) {
            o->AddRef();
            list.push_back(o);
        }
    }
    reinterpret_cast<cFC4NetworkToolManager*>(nm)->DestroyNetworkOccupants(list);  // slot 6 void
    for (cISC4Occupant*& o : list) {
        if (o != nullptr) o->Release();
    }
}

void ReleaseOccupantList(void* u, const uint64_t* occ, uint32_t count) {
    (void)u;
    if (occ == nullptr || count == 0) return;
    for (uint32_t i = 0; i < count; ++i) {
        cIGZUnknown* o = FromToken(occ[i]);
        if (o != nullptr) FC4_RELEASE(o);
    }
}

// §11.1 step 6: slot 86 (true), SetUsable(false), slot 50 (ReleasePathInfo),
// SetUsable(true) — in that order (NT4). The usable toggle posts 0x89D3E9F3
// data1 = 0 then 1 (the occupant's own flag posts). The reference release is
// the core's.
void UndergroundRebuild(void* u, uint64_t occ, uint32_t outcome, const float* vertex_heights,
                        const float* stored_terrain) {
    (void)u;
    cISC4NetworkOccupant* n = NetOf(occ);
    if (n == nullptr) return;
    // The vertex heights (slot 84), the model rebuild (slot 86), the path
    // drop (slot 50) and the stored terrain (slot 95) live on the extended
    // underground interface (QI 0x4A10AFC2, network-occupants.md §1.2).
    void* p = nullptr;
    if (!n->QueryInterface(kIID_cISC4UndergroundOccupant, &p) || p == nullptr) return;
    auto* ug = static_cast<cFC4UndergroundOccupant*>(p);
    FC4_ACQUIRED(ug, "QueryInterface underground");
    if (outcome == FC4_NETTOOL_U_UPDATED && vertex_heights != nullptr) {
        float vh[4];
        std::memcpy(vh, vertex_heights, sizeof(vh));
        ug->SetVertexHeights(vh);  // slot 84
    }
    ug->CreateGeometry(true);  // slot 86: rebuild the model
    n->SetUsable(false);       // shared slot 20 (posts 0x89D3E9F3 data1 = 0)
    ug->ReleasePathInfo();     // slot 50
    n->SetUsable(true);        // shared slot 20 (posts 0x89D3E9F3 data1 = 1)
    if (outcome != FC4_NETTOOL_U_DESTROY && stored_terrain != nullptr) {
        float th[4];
        std::memcpy(th, stored_terrain, sizeof(th));
        ug->SetTerrainHeights(th);  // slot 95 (also rebuilds)
    }
    FC4_RELEASE(ug);
}

}  // namespace

}  // namespace fc4

namespace fc4 {

fc4_nettool_host MakeNetToolHost(NetToolContext* ctx) {
    fc4_nettool_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    // ---- the drag and the pipeline (§3.2, §5) ----
    h.draw_network_line = &DrawNetworkLine;
    h.validate_and_solve = &ValidateAndSolve;
    h.get_network_occupant = &GetNetworkOccupant;
    h.callback_place = &CallbackPlace;
    // ---- the commit (§6) ----
    h.insert_occupants_for_solved_cells = &InsertOccupantsForSolvedCells;
    h.mark_occupants_usable = &MarkOccupantsUsable;
    // ---- §4 / §6.4 ----
    h.insert_network_occupant_by_id = &InsertNetworkOccupantById;
    // ---- §10 DestroyOccupantsAndResolve ----
    h.occupied_cell = &OccupiedCell;
    h.neighbours_by_network_type = &NeighboursByNetworkType;
    h.is_intersection_piece = &IsIntersectionPiece;
    h.intersection_structure = &IntersectionStructure;
    h.is_tunnel_portal = &IsTunnelPortal;
    h.paired_portal = &PairedPortal;
    h.send_pre_demolish_sync = &SendPreDemolishSync;
    h.remove_occupant = &RemoveOccupant;
    h.remove_or_replace_occupant = &RemoveOrReplaceOccupant;
    h.occupant_shutdown = &OccupantShutdown;
    h.release = &OccupantRelease;
    h.solve_network_resolve = &SolveNetworkResolve;
    // ---- §11 the underground refit ----
    h.corner_altitudes = &CornerAltitudes;
    h.smoothen_underground = &SmoothenUnderground;
    h.find_underground_solved = &FindUndergroundSolved;
    h.destroy_network_occupants = &DestroyNetworkOccupants;
    h.release_occupant_list = &ReleaseOccupantList;
    h.underground_rebuild = &UndergroundRebuild;
    return h;
}

void NetToolReleaseHeld(NetToolContext* ctx) {
    if (ctx == nullptr) return;
    std::vector<cIGZUnknown*> held;
    held.swap(ctx->held);
    for (cIGZUnknown* o : held) {
        if (o != nullptr) FC4_RELEASE(o);
    }
}

}  // namespace fc4
