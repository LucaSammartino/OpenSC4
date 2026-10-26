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
#include "FC4Properties.h"

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
#include "FC4TerrainInterfaces.h"  // cFC4ISTETerrain: the compiled terrain slots
#include "cS3DVector3.h"
#include "GZServPtrs.h"
#include "cGZPersistResourceKey.h"
#include "cIGZPersistDBRecord.h"
#include "cIGZPersistResourceManager.h"
#include "compat/FC4Compat.h"

#include <cstdio>
#include <cstring>
#include <sstream>
#include <cstdlib>
#include <unordered_map>
#include <utility>
#include <vector>

namespace fc4 {

// On-demand geometry: declared in cFC4NetworkTool.cpp (the shared core for
// the tool family); the host callbacks create it on the first core call.
fc4_netgeom* GeometryOf(NetToolContext* ctx);

namespace {

constexpr uint32_t kIID_cISC4NetworkOccupant = 0xA821EF94;  // network-tools.md §3.3, §10
constexpr uint32_t kIID_cISC4NetworkTunnel = 0x8A4BD52B;    // §10: the paired portal
constexpr uint32_t kIID_cISC4UndergroundOccupant = 0x4A10AFC2;  // the extended underground interface (§1.2)
constexpr uint32_t kCLSID_cSC4NetworkOccupant = 0xC9C05C6E;  // network-occupants.md §1.1
constexpr uint32_t kMsgPreDemolishNetworkOccupant = 0xAA7A014D;  // §10 step 5.1

NetToolContext* Ctx(void* u) { return static_cast<NetToolContext*>(u); }
cISC4City* City(void* u) { return Ctx(u)->city; }

cISC4NetworkOccupant* NetOf(uint64_t t) { return reinterpret_cast<cISC4NetworkOccupant*>(FromToken(t)); }
cISC4Occupant* OccOf(uint64_t t) {
    // network-occupants.md section 1: the occupant interface is distinct from
    // the network interface. Slot 48 returns its borrowed occupant view.
    cISC4NetworkOccupant* n = NetOf(t);
    return n != nullptr ? n->AsOccupant() : nullptr;
}

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

// The lowest network type bit shared by `flags` and the mask (0-12), or 0xFFFF_FFFF.
uint32_t TypeOfFlags(uint32_t flags) {
    uint32_t bits = flags & 0x1FFFu;
    for (uint32_t t = 0; t < 13; ++t) {
        if (bits & (1u << t)) return t;
    }
    return 0xFFFFFFFFu;
}

// ---- the piece solver (network-rules.md §2, §6, §11) -----------------------------------------------

// The rule database (§2.1), process-wide like the original's tables, loaded
// on the first solve (main thread: tool calls only). [choice] Loaded once per
// process; the original reloads after the application's static-data cleanup
// (§2.4.2), which this plugin does not observe.
fc4_network_rules* g_rules = nullptr;
bool g_rulesTried = false;
uint32_t g_solveLogs = 0;
uint32_t g_insertLogs = 0;

// The solved records of the last ValidateAndSolve, read by the
// InsertOccupantsForSolvedCells of the same run (both inside one
// PlaceNetworkOccupants on the main thread). Kept here, not in
// NetToolContext, so the context's layout (shared with cFC4NetworkTool)
// does not change.
struct SolvedPiece {
    int32_t x, z;
    uint32_t id, orientation;
};
constexpr uint32_t kMaxSolvedPieces = FC4_NETTOOL_MAX_CELLS;
SolvedPiece g_pieces[kMaxSolvedPieces];
uint32_t g_pieceCount = 0;
const NetToolContext* g_piecesOwner = nullptr;  // the context whose run solved them
uint32_t g_ruleSolves = 0;

// One raw resource through the resource manager's DB record (SDK
// cIGZPersistResourceManager::OpenDBRecord / CloseDBRecord, cIGZPersistDBRecord
// GetSize / GetFieldVoid). [choice] The third OpenDBRecord argument is unnamed
// in the SDK; false (read access) is the literal reading. The record is closed
// through CloseDBRecord, never Released here (we did not AddRef it).
bool ReadRuleResource(cIGZPersistResourceManager* rm, uint32_t type, uint32_t group, uint32_t instance,
                      std::vector<uint8_t>& out) {
    out.clear();
    cGZPersistResourceKey key(type, group, instance);
    cIGZPersistDBRecord* rec = nullptr;
    // [abi-ret] slot 37 OpenDBRecord: row says eax-or-void, so its bool is not
    // trusted; the out-pointer alone decides whether a record was opened.
    (void)rm->OpenDBRecord(key, &rec, false);
    if (rec == nullptr) return false;
    const uint32_t size = rec->GetSize();
    out.resize(size);
    const bool ok = size == 0 || rec->GetFieldVoid(out.data(), size);
    rm->CloseDBRecord(key, &rec);
    if (!ok) out.clear();
    return ok;
}

// §2.1's load order: per type its tile files (§2.3, inside step 1), the INI
// (tile verifiers, §9.2), then 0x10000001, 0x10000002, 0x10000000.
fc4_network_rules* Rules() {
    if (g_rulesTried) return g_rules;
    g_rulesTried = true;
    cIGZPersistResourceManagerPtr rm;
    if (!rm) {
        Log("netrules: WARNING no resource manager; the piece solver stays off");
        return nullptr;
    }
    g_rules = fc4_network_rules_create();
    if (g_rules == nullptr) return nullptr;
    constexpr uint32_t kType = 0x0A5BCF4Bu, kGroup = 0xAA5BCF57u;
    std::vector<uint8_t> bytes;
    uint32_t tiles = 0, missing = 0, rules = 0;
    for (uint32_t t = 0; t < 13; ++t) {
        uint32_t inst[4] = {};
        const uint32_t n = fc4_network_rules_tile_instances(t, inst, 4);
        for (uint32_t i = 0; i < n && i < 4; ++i) {
            const bool found = ReadRuleResource(rm, kType, kGroup, inst[i], bytes);
            fc4_network_rules_report rep{};
            rep.struct_size = sizeof(rep);
            // A missing file still appends an evaluator that never matches (§2.3).
            fc4_network_rules_load(g_rules, inst[i], found ? bytes.data() : nullptr, found ? bytes.size() : 0, &rep);
            ++tiles;
            if (!found) ++missing;
            rules += rep.rules;
        }
    }
    // The network INI: type 0, group 0x8A5971C5, instance 0x8A5993B9 (§2.1 step 1).
    const bool ini = ReadRuleResource(rm, 0u, 0x8A5971C5u, FC4_NR_NETWORK_INI, bytes);
    if (ini) fc4_network_rules_load(g_rules, FC4_NR_NETWORK_INI, bytes.data(), bytes.size(), nullptr);
    const uint32_t rul[3] = {0x10000001u, 0x10000002u, 0x10000000u};
    uint32_t rulFound = 0;
    for (uint32_t r : rul) {
        if (ReadRuleResource(rm, kType, kGroup, r, bytes)) {
            fc4_network_rules_load(g_rules, r, bytes.data(), bytes.size(), nullptr);
            ++rulFound;
        }
    }
    Log("netrules: rules loaded tile_files=%u missing=%u tile_rules=%u ini=%d rul=%u/3 intersection_rules=%u",
        tiles, missing, rules, ini ? 1 : 0, rulFound, fc4_network_rules_intersection_count(g_rules));
    // §2.4.3 point 2: the original cIntRule map is filled only by the original
    // tools' Init; this database is ours and is not published to the original
    // puzzle-piece control, whose catalogue command this plugin does not own.
    Log("netrules: note the original puzzle-piece control reads only the original cIntRule map "
        "(not filled by this plugin; network-rules.md 2.4.3 point 2)");
    return g_rules;
}

void ToRulesCell(const fc4_netgeom_cell_info& gi, fc4_network_rules_cell& rc) {
    std::memset(&rc, 0, sizeof(rc));
    rc.struct_size = sizeof(rc);
    rc.x = gi.x;
    rc.z = gi.z;
    rc.flags = gi.flags;
    std::memcpy(rc.edges, gi.edges, sizeof(rc.edges));
    rc.in_run = gi.in_run;
    rc.intersection = gi.intersection;
    rc.fixed = gi.fixed;
    rc.node = gi.connection_node;
    // [choice] fc4_netgeom_cell_info carries no one-way byte and no occupant
    // fields (piece, orientation, node edges, anchor): they stay 0 / absent.
    rc.step = gi.step;
    rc.solved = -1;
}

// §6.1 over the geometry's set of cells to be solved (S) and the cell infos
// in their pad-2 square (§6.2). The solved records go to the context for
// InsertOccupantsForSolvedCells (§11). [choice] The solver's result (§6.1
// step 7) is logged next to the geometry's valid count but not adopted.
void SolvePieces(NetToolContext* c, const fc4_nettool_run* run, uint32_t geomValid) {
    fc4_network_rules* r = Rules();
    if (r == nullptr || c->geom == nullptr || c->city == nullptr) return;
    static fc4_netgeom_cell s[FC4_NETTOOL_MAX_CELLS];  // main thread only
    const uint32_t n = fc4_netgeom_get_solved(c->geom, s, FC4_NETTOOL_MAX_CELLS);
    if (n == 0) return;
    fc4_network_rules_grid* g = fc4_network_rules_grid_create(c->city->CellCountX(), c->city->CellCountZ());
    if (g == nullptr) return;
    uint32_t infos = 0;
    for (uint32_t i = 0; i < n && i < FC4_NETTOOL_MAX_CELLS; ++i) {
        for (int32_t dz = -2; dz <= 2; ++dz) {
            for (int32_t dx = -2; dx <= 2; ++dx) {
                fc4_netgeom_cell_info gi{};
                gi.struct_size = sizeof(gi);
                fc4_network_rules_cell have{};
                have.struct_size = sizeof(have);
                const int32_t x = s[i].x + dx, z = s[i].z + dz;
                if (fc4_network_rules_grid_get_cell(g, x, z, &have)) continue;
                if (!fc4_netgeom_get_cell_info(c->geom, x, z, &gi)) continue;
                fc4_network_rules_cell rc;
                ToRulesCell(gi, rc);
                if (fc4_network_rules_grid_set_cell(g, &rc)) ++infos;
            }
        }
    }
    for (uint32_t i = 0; i < n && i < FC4_NETTOOL_MAX_CELLS; ++i) fc4_network_rules_grid_add_to_solve(g, s[i].x, s[i].z);
    int32_t result = 0;
    const bool ok = fc4_network_rules_solve(r, g, run->type_id, false, &result);
    static fc4_network_rules_solved recs[kMaxSolvedPieces];
    const uint32_t total = fc4_network_rules_grid_solved(g, recs, kMaxSolvedPieces);
    const uint32_t kept = total < kMaxSolvedPieces ? total : kMaxSolvedPieces;
    for (uint32_t i = 0; i < kept; ++i) {
        g_pieces[i] = SolvedPiece{recs[i].x, recs[i].z, recs[i].id, recs[i].orientation};
    }
    g_pieceCount = kept;
    g_piecesOwner = c;
    const uint32_t unsolved = fc4_network_rules_grid_unsolved(g, nullptr, 0);
    fc4_network_rules_grid_destroy(g);
    ++g_ruleSolves;
    if (g_solveLogs < 32 || (g_solveLogs % 64) == 0) {
        char buf[512];
        int len = 0;
        for (uint32_t i = 0; i < kept && i < 8 && len < 400; ++i) {
            len += std::snprintf(buf + len, sizeof(buf) - len, " (%d,%d)=%08X/%02X", recs[i].x, recs[i].z,
                                 recs[i].id, recs[i].orientation & 0xFF);
        }
        buf[len < 0 ? 0 : len] = '\0';
        Log("netrules solve #%u type=%u ok=%d S=%u infos=%u solved=%u unsolved=%u result=%d geom_valid=%u pieces:%s",
            g_ruleSolves, run->type_id, ok ? 1 : 0, n, infos, total, unsolved, result, geomValid, buf);
    }
    ++g_solveLogs;
}

// ---- the drag and the pipeline (§3.2, §5) ----------------------------------------------------------

// Slot 18 DrawNetworkLine over the run: the geometry core replays the drag
// (§3.1-§3.3 of network-geometry.md) and fills dragged_count / straight_count
// / cells. false = the geometry core is absent.
bool DrawNetworkLine(void* u, fc4_nettool_run* run, bool snap) {
    NetToolContext* c = Ctx(u);
    if (c == nullptr || run == nullptr) return false;
    if (GeometryOf(c) == nullptr) return false;  // on-demand (run 216: o geom nunca existia aqui)
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
    if (c == nullptr || run == nullptr) return false;
    if (GeometryOf(c) == nullptr) return false;  // on-demand
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
    // The piece solver (network-rules.md §6, a component of this tool per
    // §2.4.3 point 1): the rule stages over the cell infos the geometry built.
    g_pieceCount = 0;
    g_piecesOwner = nullptr;
    if (valid > 0) SolvePieces(c, run, valid);
    // Callback success means the pass result was filled, including refusal.
    // Returning false discards out->failure in the C ABI adapter (NT28).
    return true;
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
    // NO release of `occ` here: AsOccupant (slot 3) is folded — returns this + 8
    // (network-occupants.md §4.2), the same object WITHOUT an AddRef. The single
    // GetClassObject reference is released by the caller on `n`. Releasing the
    // folded view too double-releases the object (run 217: net -16 on the
    // cISC4Occupant view, over-release warnings at this line, and the AV in the
    // network manager's shutdown path right after the tool's Shutdown).
}

// §6.1 step 2.1 / §6.2: for each solved cell, insert the solved piece into
// the city through the occupant manager. The run carries data4 (+0x208).
// The per-cell piece choice (network-rules.md §11) comes from SolvePieces'
// solved records (our rule solver). The occupant is created through GZCOM
// class 0xC9C05C6E; its piece id and orientation are written by slot 30
// SetPiece (§11.2, NR14; compat::NetworkOccupantSetPiece, which refuses an
// occupant whose slot 30 is not the specified method) BEFORE the insert, and
// read back through slots 23 / 26 before and after. Cells without a record
// insert as before.
void InsertOccupantsForSolvedCells(void* u, const fc4_nettool_run* run, bool preview_only) {
    NetToolContext* c = Ctx(u);
    if (c == nullptr || run == nullptr || preview_only) return;
    cISC4OccupantManager* om = OMgr(u);
    if (om == nullptr) return;
    std::unordered_map<uint32_t, uint32_t> byKey;  // (z << 16) + x -> solved index
    const uint32_t have = g_piecesOwner == c ? g_pieceCount : 0;  // only this context's run
    byKey.reserve(have * 2 + 1);
    for (uint32_t i = 0; i < have; ++i) {
        const auto& p = g_pieces[i];
        byKey.emplace((static_cast<uint32_t>(p.z) << 16) + static_cast<uint32_t>(p.x), i);
    }
    // [choice] Pipe (4) and Subway (7) are underground networks: §11.2 names no
    // contract for their occupants' slot 30, so their pieces are not applied
    // (counted as unsupported; question NR15).
    const bool supportedType = run->type_id != 4 && run->type_id != 7;
    uint32_t chosen = 0, applied = 0, unsupported = 0, freshZero = 0, preInsertSame = 0, sameAsOccupant = 0;
    char buf[640];
    int len = 0;
    buf[0] = '\0';
    for (uint32_t i = 0; i < run->cell_count && i < FC4_NETTOOL_MAX_CELLS; ++i) {
        cISC4NetworkOccupant* n = NewNetworkOccupant();
        if (n == nullptr) {
            ++c->insertsRefused;
            continue;
        }
        const int32_t x = run->cells[i].x, z = run->cells[i].z;
        const auto it = byKey.find((static_cast<uint32_t>(z) << 16) + static_cast<uint32_t>(x));
        const SolvedPiece* p = it != byKey.end() ? &g_pieces[it->second] : nullptr;
        uint32_t prePiece = 0, preOrient = 0, setPiece = 0, setOrient = 0;
        bool didApply = false;
        if (p != nullptr) {
            ++chosen;
            prePiece = n->PieceId();               // slot 23: a fresh occupant reads 0/0 (§11.2)
            preOrient = n->GetRotationAndFlip();   // slot 26
            if (prePiece == 0 && preOrient == 0) ++freshZero;
            // §11.2: slot 30 on the new occupant before it enters the occupant
            // manager, so the 0x99EF1142 listeners see the final piece. A
            // connection node (flag 0x200000) is never new here.
            if (supportedType && !n->HasNetworkFlag(0x200000u) &&
                compat::NetworkOccupantSetPiece(n, p->id, static_cast<uint8_t>(p->orientation & 0xFF))) {
                didApply = true;
                ++applied;
                setPiece = n->PieceId();
                setOrient = n->GetRotationAndFlip();
                if (setPiece == p->id && setOrient == (p->orientation & 0xFF)) ++preInsertSame;
            } else {
                ++unsupported;
            }
        }
        PlaceOccupant(c, om, n, x, z, run->type_id, run->data4, true);
        if (p != nullptr) {
            const uint32_t occPiece = n->PieceId();               // slot 23 after the insert
            const uint32_t occOrient = n->GetRotationAndFlip();   // slot 26 (+0x104)
            if (occPiece == p->id && occOrient == (p->orientation & 0xFF)) ++sameAsOccupant;
            if (len < 520 && chosen <= 6) {
                len += std::snprintf(buf + len, sizeof(buf) - len,
                                     " (%d,%d) ours=%08X/%02X pre=%08X/%02X set=%08X/%02X occ=%08X/%02X%s", x, z,
                                     p->id, p->orientation & 0xFF, prePiece, preOrient, setPiece, setOrient,
                                     occPiece, occOrient, didApply ? "" : " UNAPPLIED");
            }
        }
        FC4_RELEASE(n);
    }
    if (g_insertLogs < 32 || (g_insertLogs % 64) == 0) {
        Log("netrules insert type=%u cells=%u chosen=%u fresh_zero=%u applied=%u unsupported=%u "
            "set_before_insert=%u same_as_occupant=%u (piece write: slot30 SetPiece before InsertOccupant, NR14):%s",
            run->type_id, run->cell_count, chosen, freshZero, applied, unsupported, preInsertSame, sameAsOccupant,
            buf);
    }
    ++g_insertLogs;
    // Per-run lifetime: the records belong to the one ValidateAndSolve that
    // preceded this insert. Drop them once consumed so a later context (even
    // one reusing this address) never reads another run's pieces.
    if (g_piecesOwner == c) {
        g_pieceCount = 0;
        g_piecesOwner = nullptr;
    }
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
    msg->SetIGZUnknown(o);  // network-tools.md section 10: retain the occupant during delivery
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
    if (c == nullptr || run == nullptr) return;
    if (GeometryOf(c) == nullptr) return;  // on-demand
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

// ---- the fc4_netgeom host (the geometry core's own view of the world) ------
// The geometry reads the terrain through its own host table (or a bound core
// terrain). Without these callbacks its world is empty (cells 0x0) and every
// drag fails — the bug of the first game run (glue29-run213: ok=0 everywhere,
// netgeom logs silent). These adapters answer from the game's cISTETerrain and
// the occupant manager, the same sources the tool's own host uses.

// The city's terrain as compiled (FC4TerrainInterfaces.h): single
// inheritance, vtable at offset 0.
cFC4ISTETerrain* GameTerrain(void* u) {
    return reinterpret_cast<cFC4ISTETerrain*>(Terrain(u));
}

// Terrain cells (vertex grid): (cellsX + 1) * (cellsZ + 1) vertices.
void GeomCells(void* u, uint32_t* cellsX, uint32_t* cellsZ) {
    cFC4ISTETerrain* t = GameTerrain(u);
    if (t == nullptr || cellsX == nullptr || cellsZ == nullptr) return;
    *cellsX = t->CellCountX();  // slot 7
    *cellsZ = t->CellCountZ();  // slot 8
}

// Slot 12: the altitude at a flat vertex index.
float GeomVertexAltitude(void* u, int32_t vertex) {
    cFC4ISTETerrain* t = GameTerrain(u);
    if (t == nullptr) return 0.0f;
    return t->GetAltitudeAtFlatIndex(vertex);  // slot 12
}

// Slot 34: the cell has water.
bool GeomCellHasWater(void* u, int32_t x, int32_t z) {
    cFC4ISTETerrain* t = GameTerrain(u);
    if (t == nullptr) return false;
    return t->CellHasWater(static_cast<uint32_t>(x), static_cast<uint32_t>(z));  // slot 34
}

// Slot 33: the surface water level at a vertex (NaN = none).
float GeomSurfaceWaterLevel(void* u, int32_t vertex) {
    cFC4ISTETerrain* t = GameTerrain(u);
    if (t == nullptr) return 0.0f;
    return t->GetWaterDepthAtFlatIndex(vertex);  // slot 92 (depth; NaN when 0 below)
}

// Slot 24: the maximum allowed vertex height.
float GeomMaxAllowedHeight(void* u) {
    cFC4ISTETerrain* t = GameTerrain(u);
    if (t == nullptr) return 0.0f;
    return t->MaxAllowedHeight();  // slot 24
}

// Slot 50: writes the vertex altitude (the grading, §9.7).
void GeomSetAltitudeAtVertex(void* u, int32_t vertex, float h) {
    cFC4ISTETerrain* t = GameTerrain(u);
    if (t == nullptr) return;
    t->SetAltitudeAtVertex(vertex, h);  // slot 50
}

// Slots 87/88: the long terrain operation bracket.
void GeomBeginLongTerrainOperation(void* u) {
    cFC4ISTETerrain* t = GameTerrain(u);
    if (t == nullptr) return;
    t->BeginLongTerrainOperation();  // slot 87
}

void GeomEndLongTerrainOperation(void* u) {
    cFC4ISTETerrain* t = GameTerrain(u);
    if (t == nullptr) return;
    t->EndLongTerrainOperation();  // slot 88
}

// Slot 68: the terrain redisplay (data4 kinds per terrain.md §8).
void GeomRedisplayTerrain(void* u, const fc4_terrain_rect* rect, uint32_t kind) {
    cFC4ISTETerrain* t = GameTerrain(u);
    if (t == nullptr || rect == nullptr) return;
    SC4Rect<int32_t> r(rect->x0, rect->z0, rect->x1, rect->z1);
    t->RedisplayTerrainRect(true, true, r, kind);  // slot 68
}

// A connection node in the cell (network-manager.md §7.7: flag 0x200000).
bool GeomIsConnectionNode(void* u, int32_t x, int32_t z) {
    NetToolContext* c = Ctx(u);
    if (c == nullptr) return false;
    cISC4OccupantManager* om = OMgr(u);
    if (om == nullptr) return false;
    uint64_t tokens[8] = {};
    const uint32_t n = CellNetworkOccupants(c, om, x, z, 0x1FFFu, tokens, 8, nullptr, nullptr);
    for (uint32_t i = 0; i < n; ++i) {
        cISC4NetworkOccupant* net = NetOf(tokens[i]);
        if (net != nullptr && (net->GetNetworkFlag() & 0x200000u) != 0) return true;
    }
    return false;
}

// The cell's occupants in the geometry's record (kind + type + immovable).
uint32_t GeomCellOccupants(void* u, int32_t x, int32_t z, fc4_netgeom_occupant* out, uint32_t capacity) {
    NetToolContext* c = Ctx(u);
    if (c == nullptr || out == nullptr || capacity == 0) return 0;
    cISC4OccupantManager* om = OMgr(u);
    if (om == nullptr) return 0;
    uint64_t tokens[16] = {};
    const uint32_t n = CellNetworkOccupants(c, om, x, z, 0x1FFFu, tokens, 16, nullptr, nullptr);
    uint32_t written = 0;
    for (uint32_t i = 0; i < n && written < capacity; ++i) {
        cISC4NetworkOccupant* net = NetOf(tokens[i]);
        if (net == nullptr) continue;
        const uint32_t flags = net->GetNetworkFlag();
        fc4_netgeom_occupant o{};
        o.kind = FC4_NETGEOM_OCC_NETWORK;
        o.immovable = (flags & 0x400000u) != 0 ? 1 : 0;
        o.connection_node = (flags & 0x200000u) != 0 ? 1 : 0;
        o.type_id = TypeOfFlags(flags);
        out[written] = o;
        ++written;
    }
    return written;
}

// Slot 59 over the proposed cells: the count the tool's own host uses.
uint32_t GeomUpdateOccupantHighlights(void* u, uint32_t type_id, const fc4_netgeom_cell* cells, uint32_t count,
                                      uint32_t valid) {
    (void)u;
    (void)type_id;
    (void)cells;
    (void)count;
    return valid;  // [choice] keep the valid count (the highlight pass wiring is later work)
}

uint32_t GeomBridgeTypeCount(void* u, uint32_t type_id) {
    (void)u;
    (void)type_id;
    return 0;  // no bridge types wired yet (the bridge pass passes with 0)
}

// ---- §11 the underground refit ---------------------------------------------------------------------

// Terrain slot 17: the cell's current corner altitudes.
void CornerAltitudes(void* u, int32_t x, int32_t z, float out[4]) {
    cFC4ISTETerrain* t = GameTerrain(u);
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
        // Section 11.1's refit destroy list already contains AsOccupant views.
        cISC4Occupant* o = reinterpret_cast<cISC4Occupant*>(FromToken(occ[i]));
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

intptr_t NetToolOccupantViewsTest(cIGZUnknown* network, cIGZMessageServer2* server) {
    if (network == nullptr || server == nullptr) return -1;
    NetToolContext ctx{};
    ctx.msgServer = server;
    SendPreDemolishSync(&ctx, TokenOf(network));
    OccupantShutdown(&ctx, TokenOf(network));
    OccupantShutdown(&ctx, 0);
    return 1;
}

}  // namespace fc4

namespace fc4 {

bool ReadNetToolPowerlineSelection(NetToolContext* ctx, fc4_powerline_selection* out) {
#if defined(_WIN32) && defined(_M_IX86)
    if (!ctx || !ctx->city || !ctx->geom || !out) return false;
    auto* terrain = reinterpret_cast<cFC4ISTETerrain*>(ctx->city->GetTerrain());
    auto* zones = reinterpret_cast<cFC4ISC4ZoneManager*>(ctx->city->GetZoneManager());
    auto* lots = reinterpret_cast<cFC4ISC4LotManager*>(ctx->city->GetLotManager());
    auto* occupants = ctx->city->GetOccupantManager();
    // NT33: active view service, not a terrain subobject. Own the QI only
    // during this synchronous replay. Nothing survives to PreAppShutdown.
    auto* service = *reinterpret_cast<cIGZUnknown**>(0x00B43DD0);
    cIGZUnknown* renderer = nullptr;
    if (!terrain || !zones || !lots || !occupants || !service ||
        !service->QueryInterface(0xE9C622D8, reinterpret_cast<void**>(&renderer)) || !renderer) return false;
    FC4_ACQUIRED(renderer, "QueryInterface");
    struct RendererScope {
        cIGZUnknown* p;
        ~RendererScope() { FC4_RELEASE(p); }
    } rendererScope{renderer};
    struct Reads {
        cFC4ISTETerrain* terrain;
        cFC4ISC4ZoneManager* zones;
        cFC4ISC4LotManager* lots;
        cISC4OccupantManager* occupants;
        cIGZUnknown* renderer;
        uint32_t rayCalls = 0;
        uint32_t lotCalls = 0;
    } reads{terrain, zones, lots, occupants, renderer};
    fc4_netgeom_state state{};
    if (!fc4_netgeom_get_state(ctx->geom, &state) || state.straight_count != state.dragged_count) return false;
    std::vector<fc4_netgeom_cell> cells(fc4_netgeom_get_cells(ctx->geom, nullptr, 0));
    std::vector<fc4_netgeom_step> steps(fc4_netgeom_get_steps(ctx->geom, nullptr, 0));
    fc4_netgeom_get_cells(ctx->geom, cells.data(), static_cast<uint32_t>(cells.size()));
    fc4_netgeom_get_steps(ctx->geom, steps.data(), static_cast<uint32_t>(steps.size()));
    fc4_powerline_selection_host h{};
    h.struct_size = sizeof(h);
    h.user = &reads;
    h.cell = [](void* u, int32_t x, int32_t z, int32_t* zone, bool* blocked) {
        auto& r = *static_cast<Reads*>(u);
        cISC4Occupant* o = nullptr;
        if (r.occupants->GetFirstOccupantByStandardCityCell(o, x, z, static_cast<cISC4OccupantFilter*>(nullptr)) && o) {
            FC4_ACQUIRED(o, "GetFirstOccupantByStandardCityCell");
            FC4_RELEASE(o);
            return false;  // Bounded ABI has no reuse/occupant classification host.
        }
        *zone = r.zones->GetZoneType(x, z);
        *blocked = r.lots->GetLot(x, z, false) != nullptr;  // borrowed; NT33
        ++r.lotCalls;
        return true;
    };
    h.width = [](void* u) {
        // Terrain slot 6 returns ST0. Use an explicit float call boundary:
        // the SDK's same-named uint32 method has an unused-method ABI waiver.
        auto* t = static_cast<Reads*>(u)->terrain;
        using ReadWidth = float (__thiscall*)(void*);
        return reinterpret_cast<ReadWidth>((*reinterpret_cast<void***>(t))[6])(t);
    };
    h.height = [](void* u, float x, float z) { return static_cast<Reads*>(u)->terrain->GetWaterLevelAt(x, z); };
    h.special = [](void* u, int32_t x, int32_t z) {
        return static_cast<Reads*>(u)->terrain->CellHasWater(static_cast<uint32_t>(x), static_cast<uint32_t>(z));
    };
    h.corners = [](void* u, int32_t x, int32_t z, float* outCorners) {
        static_cast<Reads*>(u)->terrain->GetCellCornerAltitudes(static_cast<uint32_t>(x), static_cast<uint32_t>(z), outCorners);
    };
    h.ray = [](void* u, const float* origin, const float* direction, float* hit) {
        auto& reads = *static_cast<Reads*>(u);
        ++reads.rayCalls;
        auto* r = reads.renderer;
        using Pick = bool (__thiscall*)(void*, const float*, const float*, float*, uint32_t, uint32_t);
        auto** table = *reinterpret_cast<void***>(r);
        return reinterpret_cast<Pick>(table[61])(r, origin, direction, hit, 0, 0);
    };
    props::Exemplar tuning;
    if (!tuning.Load(0x6534284A, 0xE7E2C2DB, 0xC911E35B)) return false;
    std::vector<uint32_t> spacing, poleCost;
    std::vector<float> clearance, wireCost;
    props::GetU32s(tuning.Holder(), 0x098B25C8, spacing);
    props::GetU32s(tuning.Holder(), 0x4A5F5953, poleCost);
    props::GetF32s(tuning.Holder(), 0x098B27D0, clearance);
    props::GetF32s(tuning.Holder(), 0x0A5F596B, wireCost);
    if (spacing.empty() || poleCost.empty() || clearance.empty() || wireCost.empty()) return false;
    std::vector<fc4_powerline_selected_pole> poles(cells.size());
    const bool complete = fc4_powerline_select(&h, cells.data(), static_cast<uint32_t>(cells.size()),
        steps.data(), static_cast<uint32_t>(steps.size()), state.straight_count,
        spacing[0], clearance[0], poleCost[0], wireCost[0], poles.data(), static_cast<uint32_t>(poles.size()), out);
    Log("nettool powerline observations complete=%d prefix=%u poles=%u wires=%u cost=%d rays=%u lots=%u scope=selection-only contract=NT32-NT33",
        complete ? 1 : 0, state.straight_count, out->poles, out->wires, out->cost, reads.rayCalls, reads.lotCalls);
    return complete;
#else
    (void)ctx;
    (void)out;
    return false;
#endif
}

void LogNetToolReplayResources() {
    static bool tried = false;
    if (tried) return;
    tried = true;  // NT31: a missing first resource load is not retried.
    props::Exemplar tuning;
    const bool loaded = tuning.Load(0x6534284A, 0xE7E2C2DB, 0xC911E35B);
    std::vector<uint32_t> spacing, poleCost;
    std::vector<float> clearance, wireCost;
    props::GetU32s(tuning.Holder(), 0x098B25C8, spacing);
    props::GetU32s(tuning.Holder(), 0x4A5F5953, poleCost);
    props::GetF32s(tuning.Holder(), 0x098B27D0, clearance);
    props::GetF32s(tuning.Holder(), 0x0A5F596B, wireCost);
    Log("nettool resources powerline loaded=%d spacing=%u clearance=%.9g pole_cost=%u wire_cost=%.9g "
        "scope=resource-read placement=uncovered clearance_ray=unavailable contract=NT31",
        loaded ? 1 : 0, spacing.empty() ? 20 : spacing[0], clearance.empty() ? 7.0f : clearance[0],
        poleCost.empty() ? 0 : poleCost[0], wireCost.empty() ? 0.0f : wireCost[0]);
    cIGZPersistResourceManagerPtr rm;
    std::vector<uint8_t> bytes;
    if (!rm || !ReadRuleResource(rm, 0, 0x8A5971C5, 0x8A5993B9, bytes)) {
        Log("nettool resources network_ini=unavailable scope=resource-read");
        return;
    }
    const auto trim = [](const std::string& s) {
        const size_t first = s.find_first_not_of(" \t\r\n");
        return first == std::string::npos ? std::string() : s.substr(first, s.find_last_not_of(" \t\r\n") - first + 1);
    };
    std::istringstream input(std::string(bytes.begin(), bytes.end()));
    std::string line, section;
    uint32_t poles = 0, bridges = 0, loadedBridges = 0;
    while (std::getline(input, line)) {
        line = trim(line.substr(0, line.find_first_of(";#")));
        if (line.empty()) continue;
        if (line.front() == '[' && line.back() == ']') {
            section = line.substr(1, line.size() - 2);
            continue;
        }
        if (section != "PowerPoles" && section != "RoadBridgeExemplars") continue;
        const size_t equal = line.find('=');
        if (equal == std::string::npos) continue;
        const std::string key = trim(line.substr(0, equal)), value = trim(line.substr(equal + 1));
        if (section == "PowerPoles") {
            // Raw unsigned INI mapping, not invented pole instances. No model
            // attachment/orientation coverage is inferred from this inventory.
            Log("nettool resources PowerPoles key=%s instance=%s scope=resource-read", key.c_str(), value.c_str());
            ++poles;
            continue;
        }
        ++bridges;
        std::istringstream tokens(value);
        std::vector<std::string> parts;
        std::string token;
        while (std::getline(tokens, token, ',')) parts.push_back(trim(token));
        if (parts.size() != 4) {
            Log("nettool resources road_bridge key=%s malformed_tokens=%u", key.c_str(), static_cast<uint32_t>(parts.size()));
            continue;
        }
        uint32_t tgi[3]{};
        bool parsed = true;
        for (size_t i = 0; i < 3; ++i) {
            char* end = nullptr;
            const unsigned long parsedValue = std::strtoul(parts[i].c_str(), &end, 16);
            parsed &= end != parts[i].c_str() && *end == '\0';
            tgi[i] = static_cast<uint32_t>(parsedValue);
        }
        props::Exemplar bridge;
        const bool found = parsed && bridge.Load(tgi[0], tgi[1], tgi[2]);
        if (found) ++loadedBridges;
        std::vector<uint32_t> minSpan, maxSpan, cost, deck;
        props::GetU32s(bridge.Holder(), 0x482CB458, minSpan);
        props::GetU32s(bridge.Holder(), 0x482CB467, maxSpan);
        props::GetU32s(bridge.Holder(), 0x482CB470, cost);
        props::GetU32s(bridge.Holder(), 0x482CB445, deck);
        Log("nettool resources road_bridge key=%s tgi=%08X/%08X/%08X loaded=%d min_span_present=%d "
            "min_span=%u max_span=%u cost=%u/%u deck=%08X scope=resource-read model_host=uncovered",
            key.c_str(), tgi[0], tgi[1], tgi[2], found ? 1 : 0, minSpan.empty() ? 0 : 1,
            minSpan.empty() ? 0 : minSpan[0], maxSpan.empty() ? 100000 : maxSpan[0],
            cost.empty() ? 0 : cost[0], cost.size() < 2 ? 0 : cost[1], deck.empty() ? 0 : deck[0]);
    }
    Log("nettool resources network_ini=loaded pole_mappings=%u road_entries=%u road_loaded=%u "
        "scope=resource-read selection_host=uncovered model_host=uncovered", poles, bridges, loadedBridges);
}

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

// The netgeom host over the ctx (terrain via the city, world via the occupant
// manager). The presentation-only callbacks stay NULL (glue24's pattern).
fc4_netgeom_host MakeNetGeomHost(NetToolContext* ctx) {
    fc4_netgeom_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.cells = &GeomCells;
    h.vertex_altitude = &GeomVertexAltitude;
    h.cell_has_water = &GeomCellHasWater;
    h.surface_water_level = &GeomSurfaceWaterLevel;
    h.max_allowed_height = &GeomMaxAllowedHeight;
    h.set_altitude_at_vertex = &GeomSetAltitudeAtVertex;
    h.begin_long_terrain_operation = &GeomBeginLongTerrainOperation;
    h.redisplay_terrain = &GeomRedisplayTerrain;
    h.end_long_terrain_operation = &GeomEndLongTerrainOperation;
    h.is_connection_node = &GeomIsConnectionNode;
    h.cell_occupants = &GeomCellOccupants;
    h.update_occupant_highlights = &GeomUpdateOccupantHighlights;
    h.bridge_type_count = &GeomBridgeTypeCount;
    return h;
}

}  // namespace fc4
