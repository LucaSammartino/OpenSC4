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

#include "FC4TerrainShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4HarnessLogic.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Watchdog.h"

#include "fc4.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "SC4Point.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4Simulator.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISTESurfaceWater.h"
#include "cISTETerrain.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"

#include <algorithm>
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <map>
#include <string>
#include <vector>

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

// Game calls (checked against docs/abi/interfaces.tsv by abi_conformance_test;
// every float return below is an x87 return in the table, every bool AL):
//   cISC4City 34 GetOccupantManager, 40 GetTrafficNetwork, 83 GetSurfaceWater,
//   84 GetTerrain (all borrowed);
//   cISTETerrain 7, 8 cell counts (eax), 13 GetAltitude(x, z), 14
//   GetAltitudeAtVertex, 21, 22 MinHeight / MaxHeight, 23, 24 the allowed
//   heights, 26 GetFaceNormal (void; untouched out of bounds), 29 GetSeaLevel,
//   33 GetWaterDepthAtVertex(x, z), 34 CellHasWater, 35 CellHasLand;
//   cISTESurfaceWater 6 GetWaterDepthForCell, 8 IsNavigable(x, z, r, lot),
//   12 GetWaterType;
//   the traffic network map 8 FindNetworkTile (FC4Interfaces.h); network
//   occupant 12 GetNetworkFlag, 67 GetPassageCeiling(float&) (the deck height
//   of terrain.md §10.4 step 4);
//   cISC4OccupantManager 41 GetOccupantsByStandardCityCells (typed; an A
//   list, one reference per entry);
//   cISC4PowerPoleOccupant 8 GetCell (the pole's own cell, §10.4 step 5,
//   TR20).

namespace fc4 {
namespace {

constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgTerrainChanged = 0x278E6003;  // §8.1
constexpr uint32_t kMsgWaterChanged = 0x678E6E3A;    // §8.1
constexpr uint32_t kMsgSeaLevelChanged = 0x89171496;  // §8.4
constexpr uint32_t kMsgLongOpBegin = 0xEA3B0AF1;     // §8.3
constexpr uint32_t kMsgLongOpEnd = 0x6A3B0AFD;
constexpr uint32_t kMessages[] = {kMsgSimNewMonth, kMsgTerrainChanged, kMsgWaterChanged, kMsgSeaLevelChanged,
                                  kMsgLongOpBegin, kMsgLongOpEnd};
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kNetworkTileMask = 0x1F4F;      // §10.4 step 4
constexpr uint32_t kNetworkFlagBridge = 0x40000;   // §10.4 step 4
constexpr uint32_t kOccupantTypePowerPole = 0x2890D4DE;  // §10.4 step 5
constexpr uint32_t kIID_cISC4PowerPoleOccupant = 0xA8C86142;  // §10.4 step 5 (TR20)

constexpr long kDefaultPoints = 256;
constexpr long kDefaultDiffs = 8;
constexpr long kDefaultLines = 8;
constexpr uint32_t kMaxRectCells = 4096;  // [choice] per change, a stride beyond
constexpr size_t kLogChunk = 2800;        // fc4::Log has a fixed buffer
constexpr uint32_t kSentinelBits = 0x7FC0DEADu;  // an output the query must leave untouched

// ---- the original's answers ---------------------------------------------------------------------------
//
// A function table, so the test seam can stand a second fc4_terrain in for
// the original (in game: the city's cISTETerrain and cISTESurfaceWater).
struct Original {
    void* u = nullptr;
    uint32_t cellsX = 0, cellsZ = 0;
    float (*altitudeAtVertex)(void*, int32_t, int32_t) = nullptr;
    float (*depthAtVertex)(void*, int32_t, int32_t) = nullptr;
    float (*seaLevel)(void*) = nullptr;
    float (*cachedHeight)(void*, int) = nullptr;   // 0 min, 1 max
    float (*allowedHeight)(void*, int) = nullptr;  // 0 min, 1 max
    float (*altitude)(void*, float, float) = nullptr;
    void (*faceNormal)(void*, float, float, float*) = nullptr;
    bool (*cellHasWater)(void*, int32_t, int32_t) = nullptr;
    bool (*cellHasLand)(void*, int32_t, int32_t) = nullptr;
    float (*depthForCell)(void*, int32_t, int32_t) = nullptr;
    int32_t (*waterType)(void*, int32_t, int32_t) = nullptr;
    bool (*isNavigable)(void*, int32_t, int32_t, int32_t) = nullptr;
};

// ---- in game -------------------------------------------------------------------------------------------

struct GameCtx {
    cISC4City* city = nullptr;
    cISTETerrain* t = nullptr;
    cFC4ISTESurfaceWater* w = nullptr;  // the compiled declaration (slots 7 and 8 differ from the SDK)
};
GameCtx g_game;  // borrowed for one event (main thread)

GameCtx* G(void* u) { return static_cast<GameCtx*>(u); }
float GAltV(void* u, int32_t x, int32_t z) { return G(u)->t->GetAltitudeAtVertex(x, z); }  // 14
float GDepthV(void* u, int32_t x, int32_t z) { return G(u)->t->GetWaterDepthAtVertex(x, z); }  // 33
float GSea(void* u) { return G(u)->t->GetSeaLevel(); }  // 29
float GCached(void* u, int which) { return which == 0 ? G(u)->t->MinHeight() : G(u)->t->MaxHeight(); }  // 21, 22
float GAllowed(void* u, int which) {
    return which == 0 ? G(u)->t->MinAllowedHeight() : G(u)->t->MaxAllowedHeight();  // 23, 24
}
float GAlt(void* u, float x, float z) { return G(u)->t->GetAltitude(x, z); }  // 13
void GNormal(void* u, float x, float z, float* out) {
    cS3DVector3 v(out[0], out[1], out[2]);  // the sentinel: slot 26 leaves it untouched out of bounds
    G(u)->t->GetFaceNormal(x, z, v);
    out[0] = v.fX;
    out[1] = v.fY;
    out[2] = v.fZ;
}
bool GWater(void* u, int32_t x, int32_t z) {
    return G(u)->t->CellHasWater(static_cast<uint32_t>(x), static_cast<uint32_t>(z));  // 34
}
bool GLand(void* u, int32_t x, int32_t z) {
    return G(u)->t->CellHasLand(static_cast<uint32_t>(x), static_cast<uint32_t>(z));  // 35
}
float GDepthCell(void* u, int32_t x, int32_t z) { return G(u)->w->GetWaterDepthForCell(x, z); }  // water 6
int32_t GType(void* u, int32_t x, int32_t z) { return G(u)->w->GetWaterType(x, z); }  // water 12
bool GNav(void* u, int32_t x, int32_t z, int32_t r) { return G(u)->w->IsNavigable(x, z, r, nullptr); }  // water 8

Original GameOriginal() {
    Original o;
    o.u = &g_game;
    o.cellsX = g_game.t->CellCountX();  // 7
    o.cellsZ = g_game.t->CellCountZ();  // 8
    o.altitudeAtVertex = &GAltV;
    o.depthAtVertex = &GDepthV;
    o.seaLevel = &GSea;
    o.cachedHeight = &GCached;
    o.allowedHeight = &GAllowed;
    o.altitude = &GAlt;
    o.faceNormal = &GNormal;
    o.cellHasWater = &GWater;
    o.cellHasLand = &GLand;
    o.depthForCell = &GDepthCell;
    o.waterType = &GType;
    o.isNavigable = &GNav;
    return o;
}

// The navigability host of our core (§10.4 steps 4 and 5), read-only. It
// reads the game only and never calls back into fc4_terrain (TR18).
int32_t HostBridgeAt(void* u, int32_t x, int32_t z, float* deck) {
    cISC4City* c = G(u)->city;
    if (c == nullptr) return FC4_TERRAIN_BRIDGE_NONE;
    auto* map = reinterpret_cast<cFC4ISC4TrafficNetworkMap*>(c->GetTrafficNetwork());  // city slot 40
    if (map == nullptr) return FC4_TERRAIN_BRIDGE_NONE;
    void* tile = map->FindNetworkTile(x, z, kNetworkTileMask, true);  // slot 8, a pointer into the map
    if (tile == nullptr) return FC4_TERRAIN_BRIDGE_NONE;
    cISC4NetworkOccupant* n = static_cast<cISC4NetworkOccupant*>(tile);
    if ((n->GetNetworkFlag() & kNetworkFlagBridge) == 0) return FC4_TERRAIN_BRIDGE_NONE;  // slot 12
    // [inferred] "the tile's deck height (tile slot 67) is known": slot 67 is
    // the SDK's GetPassageCeiling(float&), true with the height (TG-1).
    float h = 0.0f;
    if (!n->GetPassageCeiling(h)) return FC4_TERRAIN_BRIDGE_UNKNOWN_DECK;
    if (deck != nullptr) *deck = h;
    return FC4_TERRAIN_BRIDGE_DECK;
}

// [TR20] §10.4 step 5: the occupant manager's typed cell query (min = max =
// the visited cell) lists every power pole whose *bounds* touch that cell,
// so a pole whose own cell is a neighbour is listed too. The cell is blocked
// only when the pole's own cell (`GetCell`, pole slot 8) **equals** the
// visited cell. An occupant that does not answer `cISC4PowerPoleOccupant`
// (`0xA8C86142`), or a pole whose own cell is another one, does not block.
// Every occupant is visited even after a match, and each is released (the
// list carries one reference per entry; QueryInterface a second).
bool HostPowerPole(void* u, int32_t x, int32_t z) {
    cISC4City* c = G(u)->city;
    cISC4OccupantManager* om = c ? c->GetOccupantManager() : nullptr;  // city slot 34, borrowed
    if (om == nullptr) return false;
    SC4List<cISC4Occupant*> list;
    const SC4Point<int> cell(x, z);
    om->GetOccupantsByStandardCityCells(list, &cell, &cell, kOccupantTypePowerPole, 0xFFFFFFFFu);  // slot 41
    bool any = false;
    for (cISC4Occupant*& o : list) {
        if (o == nullptr) continue;
        FC4_ACQUIRED(o, "GetOccupantsByStandardCityCells list entry");
        cFC4ISC4PowerPoleOccupant* p = nullptr;
        if (o->QueryInterface(kIID_cISC4PowerPoleOccupant, reinterpret_cast<void**>(&p)) && p != nullptr) {
            FC4_ACQUIRED(p, "QueryInterface");
            int32_t px = 0, pz = 0;
            p->GetCell(px, pz);  // pole slot 8; its return channel is not read
            if (px == x && pz == z) any = true;
            FC4_RELEASE(p);
            p = nullptr;
        }
        FC4_RELEASE(o);
        o = nullptr;  // the list's destructor then only frees its nodes
    }
    return any;
}

fc4_terrain_host GameHost() {
    fc4_terrain_host h{};
    h.struct_size = sizeof(h);
    h.user = &g_game;
    h.bridge_at = &HostBridgeAt;
    h.has_power_pole = &HostPowerPole;
    return h;  // no post (ours are taken and compared), no lot (lot = null), no lava, no brushes
}

// ---- settings and state (main thread) ------------------------------------------------------------------

bool g_enabled = false;
bool g_replaced = false;  // glue18: `enable = terrain`, so there is no original to compare
long g_points = kDefaultPoints;
long g_diffs = kDefaultDiffs;
long g_lines = kDefaultLines;
bool g_started = false;

enum Query { kQAltitude, kQNormal, kQWater, kQLand, kQDepth, kQType, kQNavigable, kQCount };
const char* const kQueryNames[kQCount] = {"altitude", "normal", "water", "land", "depth", "type", "navigable"};

struct Tally {
    uint32_t exact[kQCount] = {};
    uint32_t total[kQCount] = {};
    uint32_t Mismatches() const {
        uint32_t n = 0;
        for (int q = 0; q < kQCount; ++q) n += total[q] - exact[q];
        return n;
    }
    uint32_t Total() const {
        uint32_t n = 0;
        for (int q = 0; q < kQCount; ++q) n += total[q];
        return n;
    }
    void Add(const Tally& o) {
        for (int q = 0; q < kQCount; ++q) {
            exact[q] += o.exact[q];
            total[q] += o.total[q];
        }
    }
};

// One output line in the making: the comparisons, the extra mismatches and
// the first differences.
struct Line {
    Tally q;
    uint32_t other = 0;  // mismatches outside the queries (packing, cache, setters, load depths)
    std::vector<std::string> firsts;
    void Diff(const char* fmt, ...) {
        if (static_cast<long>(firsts.size()) >= g_diffs) return;
        char b[256];
        va_list ap;
        va_start(ap, fmt);
        std::vsnprintf(b, sizeof(b), fmt, ap);
        va_end(ap);
        firsts.emplace_back(b);
    }
    uint32_t Mismatches() const { return q.Mismatches() + other; }
};

struct Shadow {
    fc4_terrain* ours = nullptr;
    Original src;
    fc4_terrain_host host{};
    bool standIn = false;  // the test seam's original
    uint32_t vertsX = 0, vertsZ = 0;
    std::vector<int32_t> waterCells;  // x + z * cellsX of the cells with water (navigability sample)
    // This month.
    uint32_t terrainChanges = 0, waterChanges = 0, seaChanges = 0, longOps = 0, changeLines = 0;
    uint32_t kinds[8] = {};
    // Over the city.
    uint32_t lines = 0, cleanLines = 0;
    Tally totals;
    uint32_t otherTotal = 0, otherChecks = 0;
    uint32_t unfollowedTotal = 0;
    // Vertices a month re-read found changed ahead of their message (the
    // terrain messages are queued posts, delivered after the change, and late
    // under turbo): copied, with ours before and the original's then. A
    // change message covering one restores ours first and follows it; one no
    // message covered by the next month is unfollowed.
    struct Pending {
        float alt0, depth0, alt1, depth1;
    };
    std::map<uint32_t, Pending> pending;
    uint32_t lateTotal = 0;
    uint32_t rules[8][4] = {};  // per kind (7 = other): slot50, raise/lower, reset, none
    // Test seam counters.
    uint32_t loadMismatches = 0, changeMismatches = 0, scriptMismatches = 0, monthMismatches = 0;
    uint32_t vertexChecks = 0, changesSeen = 0;
};
Shadow g_s;

// ---- helpers ------------------------------------------------------------------------------------------

// Removes every queued message of `t` (ours: compared where needed, never sent).
void Drain(fc4_terrain* t) {
    fc4_terrain_post buf[16];
    while (fc4_terrain_take_posts(t, buf, 16) == 16) {
    }
}

bool SameBits(float a, float b) { return std::memcmp(&a, &b, sizeof(float)) == 0; }

float Sentinel() {
    float f;
    std::memcpy(&f, &kSentinelBits, sizeof(f));
    return f;
}

std::string DateText(int32_t julian) {
    if (julian <= 0) return "-";
    return harness::FormatDate(harness::CivilFromJulian(julian));
}

// fc4::Log has a fixed buffer: a long line goes out in pieces.
void Emit(const std::string& s) {
    const char* p = s.c_str();
    const size_t len = s.size();
    size_t off = 0;
    bool first = true;
    while (first || off < len) {
        size_t n = std::min(kLogChunk, len - off);
        if (off + n < len) {
            size_t cut = n;
            while (cut > kLogChunk / 2 && p[off + cut - 1] != ' ') --cut;
            if (cut > kLogChunk / 2) n = cut;
        }
        if (first) {
            Log("%.*s", static_cast<int>(n), p + off);
        } else {
            Log("terrain shadow (cont.) %.*s", static_cast<int>(n), p + off);
        }
        off += n;
        first = false;
    }
}

void Append(std::string& s, const char* fmt, ...) {
    char b[512];
    va_list ap;
    va_start(ap, fmt);
    std::vsnprintf(b, sizeof(b), fmt, ap);
    va_end(ap);
    s += b;
}

void AppendQueries(std::string& s, const Tally& t) {
    s += "queries";
    for (int q = 0; q < kQCount; ++q) {
        if (t.total[q] == 0) continue;
        Append(s, " %s %u/%u", kQueryNames[q], t.exact[q], t.total[q]);
    }
    if (t.Total() == 0) s += " none";
}

void Finish(std::string& s, Line& L) {
    Append(s, "; refused=%u", [] {
        fc4_terrain_state st{};
        st.struct_size = sizeof(st);
        return fc4_terrain_get_state(g_s.ours, &st) ? st.reentrant_refused : 0u;
    }());
    Append(s, "; mismatches=%u", L.Mismatches());
    if (!L.firsts.empty()) {
        s += "; first:";
        for (const std::string& f : L.firsts) s += " " + f;
    }
    ++g_s.lines;
    if (L.Mismatches() == 0) ++g_s.cleanLines;
    g_s.totals.Add(L.q);
    g_s.otherTotal += L.other;
}

// ---- the comparisons ------------------------------------------------------------------------------------

void CompareAltitude(Line& L, float x, float z) {
    const float o = g_s.src.altitude(g_s.src.u, x, z);
    const float m = fc4_terrain_altitude(g_s.ours, x, z);
    ++L.q.total[kQAltitude];
    if (SameBits(o, m)) {
        ++L.q.exact[kQAltitude];
    } else {
        L.Diff("altitude(%.9g,%.9g) orig=%.9g ours=%.9g", x, z, o, m);
    }
}

void CompareNormal(Line& L, float x, float z) {
    const float s = Sentinel();
    float o[3] = {s, s, s};
    float m[3] = {s, s, s};
    g_s.src.faceNormal(g_s.src.u, x, z, o);
    fc4_terrain_face_normal(g_s.ours, x, z, m);
    ++L.q.total[kQNormal];
    if (SameBits(o[0], m[0]) && SameBits(o[1], m[1]) && SameBits(o[2], m[2])) {
        ++L.q.exact[kQNormal];
    } else {
        L.Diff("normal(%.9g,%.9g) orig=(%.9g,%.9g,%.9g) ours=(%.9g,%.9g,%.9g)", x, z, o[0], o[1], o[2], m[0], m[1],
               m[2]);
    }
}

void CompareBool(Line& L, Query q, int32_t x, int32_t z, bool o, bool m) {
    ++L.q.total[q];
    if (o == m) {
        ++L.q.exact[q];
    } else {
        L.Diff("%s(%d,%d) orig=%d ours=%d", kQueryNames[q], x, z, o ? 1 : 0, m ? 1 : 0);
    }
}

// CellHasWater and CellHasLand: any cell (both bounds-check, §5).
void CompareCellFlags(Line& L, int32_t x, int32_t z) {
    CompareBool(L, kQWater, x, z, g_s.src.cellHasWater(g_s.src.u, x, z), fc4_terrain_cell_has_water(g_s.ours, x, z));
    CompareBool(L, kQLand, x, z, g_s.src.cellHasLand(g_s.src.u, x, z), fc4_terrain_cell_has_land(g_s.ours, x, z));
}

// GetWaterDepthForCell (cells inside only: the original reads the four corners
// without a bounds check, §10.2).
void CompareDepth(Line& L, int32_t x, int32_t z) {
    if (x < 0 || z < 0 || static_cast<uint32_t>(x) >= g_s.src.cellsX || static_cast<uint32_t>(z) >= g_s.src.cellsZ) {
        return;
    }
    const float o = g_s.src.depthForCell(g_s.src.u, x, z);
    const float m = fc4_terrain_water_depth_for_cell(g_s.ours, x, z);
    ++L.q.total[kQDepth];
    if (SameBits(o, m)) {
        ++L.q.exact[kQDepth];
    } else {
        L.Diff("depth(%d,%d) orig=%.9g ours=%.9g", x, z, o, m);
    }
}

// GetWaterType (a vertex inside only: no bounds check either, §10.2).
void CompareType(Line& L, int32_t x, int32_t z) {
    if (x < 0 || z < 0 || static_cast<uint32_t>(x) >= g_s.vertsX || static_cast<uint32_t>(z) >= g_s.vertsZ) return;
    const int32_t o = g_s.src.waterType(g_s.src.u, x, z);
    const int32_t m = static_cast<int32_t>(fc4_terrain_water_type(g_s.ours, x, z));
    ++L.q.total[kQType];
    if (o == m) {
        ++L.q.exact[kQType];
    } else {
        L.Diff("type(%d,%d) orig=%d ours=%d", x, z, o, m);
    }
}

void CompareNavigable(Line& L, int32_t x, int32_t z, int32_t r) {
    const bool o = g_s.src.isNavigable(g_s.src.u, x, z, r);
    const bool m = fc4_terrain_is_navigable(g_s.ours, x, z, r, 0);
    ++L.q.total[kQNavigable];
    if (o == m) {
        ++L.q.exact[kQNavigable];
    } else {
        L.Diff("navigable(%d,%d,r=%d) orig=%d ours=%d", x, z, r, o ? 1 : 0, m ? 1 : 0);
    }
}

// Every query of one cell (the change rectangles and the load pass).
void CompareCell(Line& L, int32_t cx, int32_t cz, bool withPoints) {
    if (withPoints) {
        const float x = 16.0f * static_cast<float>(cx), z = 16.0f * static_cast<float>(cz);
        CompareAltitude(L, x + 8.0f, z + 8.0f);
        CompareAltitude(L, x + 4.0f, z + 12.0f);  // fx <= fz (§4.1)
        CompareAltitude(L, x + 12.0f, z + 4.0f);  // fx > fz
        CompareNormal(L, x + 12.0f, z + 4.0f);
        CompareNormal(L, x + 4.0f, z + 12.0f);
    }
    CompareCellFlags(L, cx, cz);
    CompareDepth(L, cx, cz);
    CompareType(L, cx, cz);
    CompareNavigable(L, cx, cz, 0);
}

// Cached minimum and maximum (§4.6).
void CompareCache(Line& L, const char* when) {
    for (int w = 0; w < 2; ++w) {
        const float o = g_s.src.cachedHeight(g_s.src.u, w);
        const float m = fc4_terrain_cached_height(g_s.ours, static_cast<uint32_t>(w));
        ++g_s.otherChecks;
        if (!SameBits(o, m)) {
            ++L.other;
            L.Diff("cache %s %s orig=%.9g ours=%.9g", w == 0 ? "min" : "max", when, o, m);
        }
    }
}

// ---- the sample ---------------------------------------------------------------------------------------

struct Rng {
    uint32_t s;
    uint32_t Next() {
        s += 0x9E3779B9u;
        uint32_t z = s;
        z = (z ^ (z >> 16)) * 0x85EBCA6Bu;
        z = (z ^ (z >> 13)) * 0xC2B2AE35u;
        return z ^ (z >> 16);
    }
    uint32_t Below(uint32_t n) { return n == 0 ? 0 : Next() % n; }
    float Unit() { return static_cast<float>(Next() >> 8) * (1.0f / 16777216.0f); }
};

void RefreshWaterCells() {
    g_s.waterCells.clear();
    for (uint32_t z = 0; z < g_s.src.cellsZ; ++z) {
        for (uint32_t x = 0; x < g_s.src.cellsX; ++x) {
            if (fc4_terrain_cell_has_water(g_s.ours, static_cast<int32_t>(x), static_cast<int32_t>(z))) {
                g_s.waterCells.push_back(static_cast<int32_t>(x + z * g_s.src.cellsX));
            }
        }
    }
}

// A deterministic sample seeded by `seed` (the date): points for GetAltitude
// and GetFaceNormal, cells for the rest.
void CompareSample(Line& L, uint32_t seed, long points) {
    Rng r{seed * 2654435761u + 0x5EEDu};
    const uint32_t cx = g_s.src.cellsX, cz = g_s.src.cellsZ;
    const float sizeX = 16.0f * static_cast<float>(cx), sizeZ = 16.0f * static_cast<float>(cz);
    const float edges[] = {0.0f, -0.001f, -0.5f, 0.5f};
    for (long i = 0; i < points; ++i) {
        float x = r.Unit() * sizeX, z = r.Unit() * sizeZ;
        switch (i % 8) {
            case 0:  // a vertex (§4.4)
                x = 16.0f * static_cast<float>(r.Below(cx + 1));
                z = 16.0f * static_cast<float>(r.Below(cz + 1));
                break;
            case 1:  // a cell boundary in x (§18 test 2)
                x = 16.0f * static_cast<float>(r.Below(cx + 1));
                break;
            case 2: {  // the diagonal fx = fz
                const float f = r.Unit() * 16.0f;
                x = 16.0f * static_cast<float>(r.Below(cx)) + f;
                z = 16.0f * static_cast<float>(r.Below(cz)) + f;
                break;
            }
            case 3: {  // the city edge and just outside (§3, §4.5)
                const float e = edges[r.Below(4)];
                if (r.Below(2) == 0) {
                    x = r.Below(2) == 0 ? e : sizeX + e;
                } else {
                    z = r.Below(2) == 0 ? e : sizeZ + e;
                }
                break;
            }
            case 4:  // quarter metres
                x = static_cast<float>(r.Below(cx * 64 + 1)) * 0.25f;
                z = static_cast<float>(r.Below(cz * 64 + 1)) * 0.25f;
                break;
            default: break;  // anywhere
        }
        CompareAltitude(L, x, z);
        CompareNormal(L, x, z);
        // Cells: one ring outside for the bounds-checked queries.
        const int32_t bx = static_cast<int32_t>(r.Below(cx + 2)) - 1;
        const int32_t bz = static_cast<int32_t>(r.Below(cz + 2)) - 1;
        CompareCellFlags(L, bx, bz);
        int32_t wx = static_cast<int32_t>(r.Below(cx)), wz = static_cast<int32_t>(r.Below(cz));
        if ((i & 1) == 0 && !g_s.waterCells.empty()) {
            const int32_t c = g_s.waterCells[r.Below(static_cast<uint32_t>(g_s.waterCells.size()))];
            wx = c % static_cast<int32_t>(cx);
            wz = c / static_cast<int32_t>(cx);
        }
        CompareDepth(L, wx, wz);
        CompareType(L, static_cast<int32_t>(r.Below(cx + 1)), static_cast<int32_t>(r.Below(cz + 1)));
        const int32_t nr = static_cast<int32_t>(i % 3);
        if ((i & 1) == 0) {
            CompareNavigable(L, wx, wz, nr);
        } else {
            CompareNavigable(L, bx, bz, nr);
        }
    }
}

// ---- following the original ------------------------------------------------------------------------------

// Copies vertex (x, z) of the original into ours: the altitude through slot
// 50's setter, then the depth raw.
void CopyVertex(int32_t x, int32_t z, float alt, float depth) {
    const uint32_t i = static_cast<uint32_t>(x) + static_cast<uint32_t>(z) * g_s.vertsX;
    fc4_terrain_set_altitude_at_vertex(g_s.ours, i, alt);
    fc4_terrain_set_water_depth(g_s.ours, x, z, depth);
}

// The month re-read. First the vertices left pending since the last month
// (no message covered them: unfollowed; summarised for the month line). Then
// every vertex is re-read: the ones that differ from ours changed ahead of
// their message (or without one); they are copied and become pending.
struct Unfollowed {
    uint32_t n = 0, alt = 0;
    int32_t x0 = 0, z0 = 0, x1 = -1, z1 = -1;
    int32_t fx = 0, fz = 0;
    float fa0 = 0.0f, fa1 = 0.0f, fd0 = 0.0f, fd1 = 0.0f;
    void Add(int32_t x, int32_t z, const Shadow::Pending& p) {
        if (n == 0) {
            x0 = x1 = fx = x;
            z0 = z1 = fz = z;
            fa0 = p.alt0;
            fa1 = p.alt1;
            fd0 = p.depth0;
            fd1 = p.depth1;
        } else {
            x0 = std::min(x0, x);
            x1 = std::max(x1, x);
            z0 = std::min(z0, z);
            z1 = std::max(z1, z);
        }
        ++n;
        if (!SameBits(p.alt0, p.alt1)) ++alt;
    }
};
Unfollowed Resync(uint32_t* ahead) {
    Unfollowed u;
    for (const auto& e : g_s.pending) {
        u.Add(static_cast<int32_t>(e.first % g_s.vertsX), static_cast<int32_t>(e.first / g_s.vertsX), e.second);
    }
    g_s.pending.clear();
    uint32_t n = 0;
    for (uint32_t z = 0; z < g_s.vertsZ; ++z) {
        for (uint32_t x = 0; x < g_s.vertsX; ++x) {
            const int32_t ix = static_cast<int32_t>(x), iz = static_cast<int32_t>(z);
            const float a = g_s.src.altitudeAtVertex(g_s.src.u, ix, iz);
            const float d = g_s.src.depthAtVertex(g_s.src.u, ix, iz);
            const float ma = fc4_terrain_altitude_at_vertex(g_s.ours, ix, iz);
            const float md = fc4_terrain_water_depth_at_vertex(g_s.ours, ix, iz);
            if (SameBits(a, ma) && SameBits(d, md)) continue;
            ++n;
            g_s.pending[x + z * g_s.vertsX] = Shadow::Pending{ma, md, a, d};
            CopyVertex(ix, iz, a, d);
        }
    }
    if (ahead != nullptr) *ahead = n;
    return u;
}

int32_t ClampI(int32_t v, int32_t lo, int32_t hi) { return v < lo ? lo : (v > hi ? hi : v); }

// Vertices outside the rectangle x0..x1 x z0..z1 whose altitude or depth
// differs from ours (not copied): changes this message does not cover (their
// own message is still queued, or there is none). Marked in `mask` (by vertex
// index) so the cells they touch are left out of this message's comparisons.
Unfollowed Elsewhere(int32_t x0, int32_t z0, int32_t x1, int32_t z1, std::vector<uint8_t>* mask) {
    Unfollowed u;
    for (uint32_t z = 0; z < g_s.vertsZ; ++z) {
        for (uint32_t x = 0; x < g_s.vertsX; ++x) {
            const int32_t ix = static_cast<int32_t>(x), iz = static_cast<int32_t>(z);
            if (ix >= x0 && ix <= x1 && iz >= z0 && iz <= z1) continue;
            const float a = g_s.src.altitudeAtVertex(g_s.src.u, ix, iz);
            const float ma = fc4_terrain_altitude_at_vertex(g_s.ours, ix, iz);
            const float d = g_s.src.depthAtVertex(g_s.src.u, ix, iz);
            const float md = fc4_terrain_water_depth_at_vertex(g_s.ours, ix, iz);
            if (SameBits(a, ma) && SameBits(d, md)) continue;
            if (mask != nullptr) {
                mask->resize(static_cast<size_t>(g_s.vertsX) * g_s.vertsZ);
                (*mask)[x + z * g_s.vertsX] = 1;
            }
            if (u.n == 0) {
                u.x0 = u.x1 = ix;
                u.z0 = u.z1 = iz;
            } else {
                u.x0 = std::min(u.x0, ix);
                u.x1 = std::max(u.x1, ix);
                u.z0 = std::min(u.z0, iz);
                u.z1 = std::max(u.z1, iz);
            }
            ++u.n;
        }
    }
    return u;
}

void DropShadow() {
    fc4_terrain_destroy(g_s.ours);
    g_s = Shadow{};
}

// Builds ours from the original (PostCityInit) and compares the load state.
void Load(int32_t julian) {
    // The original queries compute on demand. New-city setup can leave Lua's
    // precision active here; compare under simulation precision (§18.2).
    FpuSimulationEntry("terrain shadow load queries");
    const uint32_t cx = g_s.src.cellsX, cz = g_s.src.cellsZ;
    g_s.vertsX = cx + 1;
    g_s.vertsZ = cz + 1;
    std::vector<float> alts(static_cast<size_t>(g_s.vertsX) * g_s.vertsZ);
    for (uint32_t z = 0; z < g_s.vertsZ; ++z) {
        for (uint32_t x = 0; x < g_s.vertsX; ++x) {
            alts[x + z * g_s.vertsX] =
                g_s.src.altitudeAtVertex(g_s.src.u, static_cast<int32_t>(x), static_cast<int32_t>(z));
        }
    }
    const float sea = g_s.src.seaLevel(g_s.src.u);
    g_s.ours = fc4_terrain_create_from_altitudes(&g_s.host, nullptr, cx, cz, alts.data(), alts.size(), sea);
    if (g_s.ours == nullptr) {
        Log("terrain shadow load %s: INCOMPLETE (our terrain could not be made for %ux%u cells); not compared",
            DateText(julian).c_str(), cx, cz);
        return;
    }
    Line L;
    // §10.1: the water is rebuilt from the sea level on load.
    uint32_t depthSame = 0, depthTotal = 0;
    for (uint32_t z = 0; z < g_s.vertsZ; ++z) {
        for (uint32_t x = 0; x < g_s.vertsX; ++x) {
            const int32_t ix = static_cast<int32_t>(x), iz = static_cast<int32_t>(z);
            const float o = g_s.src.depthAtVertex(g_s.src.u, ix, iz);
            const float m = fc4_terrain_water_depth_at_vertex(g_s.ours, ix, iz);
            ++depthTotal;
            ++g_s.otherChecks;
            if (SameBits(o, m)) {
                ++depthSame;
            } else {
                ++L.other;
                L.Diff("load depth(%d,%d) orig=%.9g ours=%.9g", ix, iz, o, m);
                fc4_terrain_set_water_depth(g_s.ours, ix, iz, o);
            }
        }
    }
    // §4.6: the original's cache is the saved one (possibly stale); ours is fresh.
    const float freshMin = fc4_terrain_cached_height(g_s.ours, 0), freshMax = fc4_terrain_cached_height(g_s.ours, 1);
    const float origMin = g_s.src.cachedHeight(g_s.src.u, 0), origMax = g_s.src.cachedHeight(g_s.src.u, 1);
    fc4_terrain_set_cached_heights(g_s.ours, origMin, origMax);
    // §2.4: the shipped tuning against the original's allowed heights.
    uint32_t allowedSame = 0;
    for (int w = 0; w < 2; ++w) {
        const float o = g_s.src.allowedHeight(g_s.src.u, w);
        const float m = fc4_terrain_allowed_height(g_s.ours, static_cast<uint32_t>(w));
        ++g_s.otherChecks;
        if (SameBits(o, m)) {
            ++allowedSame;
        } else {
            ++L.other;
            L.Diff("allowed %s orig=%.9g ours=%.9g", w == 0 ? "min" : "max", o, m);
        }
    }
    fc4_terrain_post_city_init(g_s.ours);  // view ready: RedisplayTerrain posts from now on (§8.1)
    Drain(g_s.ours);
    RefreshWaterCells();
    // Every cell, and a sample of points.
    for (uint32_t z = 0; z < cz; ++z) {
        for (uint32_t x = 0; x < cx; ++x) CompareCell(L, static_cast<int32_t>(x), static_cast<int32_t>(z), false);
    }
    CompareSample(L, static_cast<uint32_t>(julian), g_points * 4);
    std::string s;
    Append(s, "terrain shadow load %s: cells=%ux%u sea=%.9g water cells=%u; depths rebuilt from the sea level %u/%u; "
              "cache orig %.9g/%.9g, fresh %.9g/%.9g (kept: orig, §4.6); allowed heights %u/2; ",
           DateText(julian).c_str(), cx, cz, sea, static_cast<unsigned>(g_s.waterCells.size()), depthSame, depthTotal,
           origMin, origMax, freshMin, freshMax, allowedSame);
    AppendQueries(s, L.q);
    Finish(s, L);
    g_s.loadMismatches += L.Mismatches();
    Emit(s);
}

int KindSlot(uint32_t kind) { return kind < 7 ? static_cast<int>(kind) : 7; }

// A terrain- or water-changed message (§8.1, §8.2).
void Change(uint32_t type, uint32_t d1, uint32_t d2, uint32_t d3, uint32_t d4, int32_t julian) {
    const bool terrain = type == kMsgTerrainChanged;
    ++g_s.changesSeen;
    if (terrain) {
        ++g_s.terrainChanges;
    } else {
        ++g_s.waterChanges;
    }
    const int ks = KindSlot(d4);
    ++g_s.kinds[ks];
    Line L;
    // The rectangle, unpacked as the consumers do (§8.1), clamped to the vertices.
    const int32_t rx0 = static_cast<int32_t>(d1 >> 16), rx1 = static_cast<int32_t>(d1 & 0xFFFF);
    const int32_t rz0 = static_cast<int32_t>(d2 >> 16), rz1 = static_cast<int32_t>(d2 & 0xFFFF);
    const int32_t vx = static_cast<int32_t>(g_s.vertsX) - 1, vz = static_cast<int32_t>(g_s.vertsZ) - 1;
    const int32_t x0 = ClampI(std::min(rx0, rx1), 0, vx), x1 = ClampI(std::max(rx0, rx1), 0, vx);
    const int32_t z0 = ClampI(std::min(rz0, rz1), 0, vz), z1 = ClampI(std::max(rz0, rz1), 0, vz);
    // Vertices a month re-read already copied ahead of this message: ours
    // goes back to its state before, so the change is followed here.
    uint32_t late = 0;
    if (!g_s.pending.empty()) {
        for (int32_t z = z0; z <= z1; ++z) {
            for (int32_t x = x0; x <= x1; ++x) {
                const uint32_t i = static_cast<uint32_t>(x) + static_cast<uint32_t>(z) * g_s.vertsX;
                auto it = g_s.pending.find(i);
                if (it == g_s.pending.end()) continue;
                fc4_terrain_set_altitude_at_vertex(g_s.ours, i, it->second.alt0);
                fc4_terrain_set_water_depth(g_s.ours, x, z, it->second.depth0);
                g_s.pending.erase(it);
                ++late;
            }
        }
        Drain(g_s.ours);
        g_s.lateTotal += late;
    }
    std::vector<uint8_t> aheadMask;
    const Unfollowed away = Elsewhere(x0, z0, x1, z1, &aheadMask);
    // Re-read: replay moved vertices through slot 50 and classify the depth.
    const float sea = fc4_terrain_sea_level(g_s.ours);
    uint32_t vertices = 0, moved = 0, wet = 0;
    uint32_t rules[4] = {};
    for (int32_t z = z0; z <= z1; ++z) {
        for (int32_t x = x0; x <= x1; ++x) {
            ++vertices;
            const float a = g_s.src.altitudeAtVertex(g_s.src.u, x, z);
            const float d = g_s.src.depthAtVertex(g_s.src.u, x, z);
            const float ma = fc4_terrain_altitude_at_vertex(g_s.ours, x, z);
            const float md = fc4_terrain_water_depth_at_vertex(g_s.ours, x, z);
            if (SameBits(a, ma)) {
                if (!SameBits(d, md)) {
                    ++wet;
                    fc4_terrain_set_water_depth(g_s.ours, x, z, d);
                }
                continue;
            }
            ++moved;
            const uint32_t i = static_cast<uint32_t>(x) + static_cast<uint32_t>(z) * g_s.vertsX;
            fc4_terrain_set_altitude_at_vertex(g_s.ours, i, a);  // slot 50: alt := h, the depth by its rule (§6)
            const float after = fc4_terrain_altitude_at_vertex(g_s.ours, x, z);
            ++g_s.otherChecks;
            if (!SameBits(after, a)) {
                ++L.other;
                L.Diff("slot50 alt(%d,%d) set %.9g got %.9g", x, z, a, after);
            }
            const float rule50 = fc4_terrain_water_depth_at_vertex(g_s.ours, x, z);
            volatile float vs = sea, va = a;  // one float subtraction each (r24 on the SSE path)
            const float ruleRL = vs - va;
            const float ruleReset = a < sea ? ruleRL : 0.0f;
            int k = 3;
            if (SameBits(d, rule50)) {
                k = 0;
            } else if (SameBits(d, ruleRL)) {
                k = 1;
            } else if (SameBits(d, ruleReset)) {
                k = 2;
            }
            ++rules[k];
            ++g_s.rules[ks][k];
            fc4_terrain_set_water_depth(g_s.ours, x, z, d);
        }
    }
    // Our RedisplayTerrain with the message's flags, rectangle and kind:
    // the same four words (§8.1 packing), and the recomputed cache.
    Drain(g_s.ours);
    fc4_terrain_rect rect{rx0, rz0, rx1, rz1};
    fc4_terrain_redisplay(g_s.ours, terrain, !terrain, rect, d4);
    fc4_terrain_post posts[4] = {};
    const size_t np = fc4_terrain_take_posts(g_s.ours, posts, 4);
    Drain(g_s.ours);
    bool packed = false;
    for (size_t p = 0; p < np && p < 4; ++p) {
        if (posts[p].msg_type != type) continue;
        packed = posts[p].data1 == d1 && posts[p].data2 == d2 && posts[p].data3 == d3 && posts[p].data4 == d4;
        if (!packed) {
            L.Diff("message orig %08X/%08X/%u/%u ours %08X/%08X/%u/%u", d1, d2, d3, d4, posts[p].data1, posts[p].data2,
                   posts[p].data3, posts[p].data4);
        }
        break;
    }
    ++g_s.otherChecks;
    if (!packed) {
        ++L.other;
        if (L.firsts.empty()) L.Diff("message: ours posted no %08X (%u posts)", type, static_cast<unsigned>(np));
    }
    if (terrain) CompareCache(L, "after redisplay");
    // The queries on the cells the rectangle touches (x0 - 1 .. x1, §8.1), capped.
    const int32_t cx0 = ClampI(x0 - 1, 0, static_cast<int32_t>(g_s.src.cellsX) - 1);
    const int32_t cx1 = ClampI(x1, 0, static_cast<int32_t>(g_s.src.cellsX) - 1);
    const int32_t cz0 = ClampI(z0 - 1, 0, static_cast<int32_t>(g_s.src.cellsZ) - 1);
    const int32_t cz1 = ClampI(z1, 0, static_cast<int32_t>(g_s.src.cellsZ) - 1);
    const uint32_t cells = static_cast<uint32_t>(cx1 - cx0 + 1) * static_cast<uint32_t>(cz1 - cz0 + 1);
    const uint32_t stride = cells > kMaxRectCells ? (cells + kMaxRectCells - 1) / kMaxRectCells : 1;
    uint32_t n = 0, skipped = 0;
    auto ahead = [&](int32_t x, int32_t z) {
        return !aheadMask.empty() && aheadMask[static_cast<uint32_t>(x) + static_cast<uint32_t>(z) * g_s.vertsX] != 0;
    };
    for (int32_t z = cz0; z <= cz1; ++z) {
        for (int32_t x = cx0; x <= cx1; ++x) {
            if ((n++ % stride) != 0) continue;
            if (ahead(x, z) || ahead(x + 1, z) || ahead(x, z + 1) || ahead(x + 1, z + 1)) {
                ++skipped;  // a corner changed ahead of its message: compared when that message comes
                continue;
            }
            CompareCell(L, x, z, true);
        }
    }
    if (moved > 0 || wet > 0) RefreshWaterCells();
    std::string s;
    Append(s, "terrain shadow change %s %s kind=%u rect=%d,%d..%d,%d: vertices=%u moved=%u wet=%u depth rules "
              "slot50/raiselower/reset/none=%u/%u/%u/%u; message %u/1; ",
           DateText(julian).c_str(), terrain ? "terrain" : "water", d4, rx0, rz0, rx1, rz1, vertices, moved, wet,
           rules[0], rules[1], rules[2], rules[3], packed ? 1u : 0u);
    if (late > 0) Append(s, "late %u (found ahead by a month re-read); ", late);
    Append(s, "changed elsewhere %u", away.n);
    if (away.n > 0) {
        Append(s, " (rect=%d,%d..%d,%d; cells touching them skipped %u)", away.x0, away.z0, away.x1, away.z1,
               skipped);
    }
    Append(s, "; ");
    if (terrain) {
        Append(s, "cache %.9g/%.9g; ", fc4_terrain_cached_height(g_s.ours, 0), fc4_terrain_cached_height(g_s.ours, 1));
    }
    if (stride > 1) Append(s, "cells %u (every %u-th) ", cells, stride);
    AppendQueries(s, L.q);
    const uint32_t mm = L.Mismatches();
    g_s.changeMismatches += mm;
    if (static_cast<long>(g_s.changeLines) < g_lines || mm != 0) {
        ++g_s.changeLines;
        Finish(s, L);
        Emit(s);
    } else {
        // Not logged: counted in the totals only.
        g_s.totals.Add(L.q);
        g_s.otherTotal += L.other;
    }
}

void Month(int32_t julian) {
    Line L;
    uint32_t ahead = 0;
    const Unfollowed uf = Resync(&ahead);
    const uint32_t unfollowed = uf.n;
    g_s.unfollowedTotal += unfollowed;
    if (ahead > 0) RefreshWaterCells();
    CompareCache(L, "month");
    const float os = g_s.src.seaLevel(g_s.src.u), ms = fc4_terrain_sea_level(g_s.ours);
    ++g_s.otherChecks;
    const bool seaSame = SameBits(os, ms);
    if (!seaSame) {
        ++L.other;
        L.Diff("sea orig=%.9g ours=%.9g", os, ms);
    }
    CompareSample(L, static_cast<uint32_t>(julian), g_points);
    std::string s;
    Append(s, "terrain shadow month %s: unfollowed=%u", DateText(julian).c_str(), unfollowed);
    if (unfollowed > 0) {
        Append(s, " (altitudes %u, depths only %u; rect=%d,%d..%d,%d; first (%d,%d) alt %.9g->%.9g depth %.9g->%.9g)",
               uf.alt, uf.n - uf.alt, uf.x0, uf.z0, uf.x1, uf.z1, uf.fx, uf.fz, uf.fa0, uf.fa1, uf.fd0, uf.fd1);
    }
    Append(s, "; ahead=%u", ahead);
    Append(s, "; cache %.9g/%.9g; sea %u/1; ", fc4_terrain_cached_height(g_s.ours, 0),
           fc4_terrain_cached_height(g_s.ours, 1), seaSame ? 1u : 0u);
    AppendQueries(s, L.q);
    Append(s, "; changes terrain=%u water=%u sea=%u long=%u kinds=%u/%u/%u/%u/%u/%u/%u/%u (lines %u)",
           g_s.terrainChanges, g_s.waterChanges, g_s.seaChanges, g_s.longOps, g_s.kinds[0], g_s.kinds[1],
           g_s.kinds[2], g_s.kinds[3], g_s.kinds[4], g_s.kinds[5], g_s.kinds[6], g_s.kinds[7], g_s.changeLines);
    Finish(s, L);
    g_s.monthMismatches += L.Mismatches();
    Emit(s);
    g_s.terrainChanges = g_s.waterChanges = g_s.seaChanges = g_s.longOps = g_s.changeLines = 0;
    std::memset(g_s.kinds, 0, sizeof(g_s.kinds));
}

// SetSeaLevel posted 0x89171496 (§6): ours takes the same sea level with the
// same reset, then every depth is compared.
void SeaLevel(int32_t julian) {
    ++g_s.seaChanges;
    Line L;
    const float sea = g_s.src.seaLevel(g_s.src.u);
    const bool accepted = fc4_terrain_set_sea_level(g_s.ours, sea);
    Drain(g_s.ours);
    uint32_t same = 0, total = 0;
    for (uint32_t z = 0; z < g_s.vertsZ; ++z) {
        for (uint32_t x = 0; x < g_s.vertsX; ++x) {
            const int32_t ix = static_cast<int32_t>(x), iz = static_cast<int32_t>(z);
            const float o = g_s.src.depthAtVertex(g_s.src.u, ix, iz);
            ++total;
            ++g_s.otherChecks;
            if (SameBits(o, fc4_terrain_water_depth_at_vertex(g_s.ours, ix, iz))) {
                ++same;
            } else {
                ++L.other;
                L.Diff("sea depth(%d,%d) orig=%.9g", ix, iz, o);
                fc4_terrain_set_water_depth(g_s.ours, ix, iz, o);
            }
        }
    }
    RefreshWaterCells();
    std::string s;
    Append(s, "terrain shadow sea %s: sea=%.9g accepted=%d depths %u/%u", DateText(julian).c_str(), sea,
           accepted ? 1 : 0, same, total);
    Finish(s, L);
    g_s.monthMismatches += L.Mismatches();
    Emit(s);
}

void Scripted(int32_t x0, int32_t z0, int32_t x1, int32_t z1, float h, uint32_t kind, int32_t julian) {
    Line L;
    fc4_terrain_rect rect{x0, z0, x1, z1};
    Drain(g_s.ours);
    fc4_terrain_set_altitude_rect(g_s.ours, rect, h, true, kind);  // slot 49, as the original was called
    Drain(g_s.ours);
    uint32_t same = 0, total = 0;
    for (int32_t z = std::max(z0, 0); z <= std::min(z1, static_cast<int32_t>(g_s.vertsZ) - 1); ++z) {
        for (int32_t x = std::max(x0, 0); x <= std::min(x1, static_cast<int32_t>(g_s.vertsX) - 1); ++x) {
            const float a = g_s.src.altitudeAtVertex(g_s.src.u, x, z);
            const float d = g_s.src.depthAtVertex(g_s.src.u, x, z);
            const float ma = fc4_terrain_altitude_at_vertex(g_s.ours, x, z);
            const float md = fc4_terrain_water_depth_at_vertex(g_s.ours, x, z);
            ++total;
            ++g_s.otherChecks;
            ++g_s.vertexChecks;
            if (SameBits(a, ma) && SameBits(d, md)) {
                ++same;
                continue;
            }
            ++L.other;
            L.Diff("vertex(%d,%d) orig=%.9g/%.9g ours=%.9g/%.9g", x, z, a, d, ma, md);
            CopyVertex(x, z, a, d);
        }
    }
    CompareCache(L, "after SetAltitude");
    RefreshWaterCells();
    std::string s;
    Append(s, "terrain shadow script %s SetAltitude rect=%d,%d..%d,%d h=%.9g kind=%u: vertices %u/%u; cache "
              "%.9g/%.9g",
           DateText(julian).c_str(), x0, z0, x1, z1, h, kind, same, total, fc4_terrain_cached_height(g_s.ours, 0),
           fc4_terrain_cached_height(g_s.ours, 1));
    Finish(s, L);
    g_s.scriptMismatches += L.Mismatches();
    Emit(s);
}

void ShutdownLine() {
    if (g_s.ours == nullptr) return;
    std::string s;
    Append(s, "terrain shadow: city shutdown: %u lines, %u with mismatches=0; totals ", g_s.lines, g_s.cleanLines);
    AppendQueries(s, g_s.totals);
    Append(s, "; other checks %u, mismatched %u; unfollowed vertices %u, followed late %u, pending at shutdown %u; "
              "depth rules by kind (slot50/raiselower/reset/none)",
           g_s.otherChecks, g_s.otherTotal, g_s.unfollowedTotal, g_s.lateTotal,
           static_cast<unsigned>(g_s.pending.size()));
    for (int k = 0; k < 8; ++k) {
        const uint32_t* r = g_s.rules[k];
        if (r[0] + r[1] + r[2] + r[3] == 0) continue;
        Append(s, " %s%d:%u/%u/%u/%u", k == 7 ? ">" : "", k == 7 ? 6 : k, r[0], r[1], r[2], r[3]);
    }
    Emit(s);
}

// ---- in-game events (guarded) -----------------------------------------------------------------------------

struct Event {
    uint32_t type = 0;
    uint32_t d[4] = {};
    int32_t julian = 0;
    int32_t rect[4] = {};
    float h = 0.0f;
};

bool BindGame(cISC4City* city) {
    g_game = GameCtx{};
    if (city == nullptr) return false;
    g_game.city = city;
    g_game.t = city->GetTerrain();       // slot 84, borrowed
    g_game.w = fc4::AsGame(city->GetSurfaceWater());  // slot 83, borrowed
    return g_game.t != nullptr && g_game.w != nullptr;
}

void LoadStep(void* p) {
    const Event* e = static_cast<const Event*>(p);
    g_s.src = GameOriginal();
    g_s.host = GameHost();
    Load(e->julian);
}

void EventStep(void* p) {
    const Event* e = static_cast<const Event*>(p);
    switch (e->type) {
        case kMsgSimNewMonth: Month(e->julian); break;
        case kMsgTerrainChanged:
        case kMsgWaterChanged: Change(e->type, e->d[0], e->d[1], e->d[2], e->d[3], e->julian); break;
        case kMsgSeaLevelChanged: SeaLevel(e->julian); break;
        default: break;
    }
}

void ScriptStep(void* p) {
    const Event* e = static_cast<const Event*>(p);
    Scripted(e->rect[0], e->rect[1], e->rect[2], e->rect[3], e->h, e->d[3], e->julian);
}

cISC4City* CurrentCity();  // below
int32_t SimJulian(cISC4City* city);  // below

void OnMessage(cIGZMessage2* msg) {
    const uint32_t type = msg->GetType();
    if (type == kMsgLongOpBegin || type == kMsgLongOpEnd) {
        ++g_s.longOps;
        return;
    }
    if (g_s.ours == nullptr || g_s.standIn) return;
    Event e;
    e.type = type;
    cIGZMessage2Standard* sm = nullptr;
    if (msg->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&sm)) && sm != nullptr) {
        FC4_ACQUIRED(sm, "QueryInterface");
        e.d[0] = static_cast<uint32_t>(sm->GetData1());
        e.d[1] = static_cast<uint32_t>(sm->GetData2());
        e.d[2] = static_cast<uint32_t>(sm->GetData3());
        e.d[3] = static_cast<uint32_t>(sm->GetData4());
        FC4_RELEASE(sm);
    }
    cISC4City* const city = CurrentCity();
    e.julian = type == kMsgSimNewMonth && e.d[0] != 0 ? static_cast<int32_t>(e.d[0]) : SimJulian(city);
    if (!BindGame(city)) return;
    if (!SehGuard("terrain shadow (event)", &EventStep, &e)) {
        Log("terrain shadow: a fault while following message %08X; the shadow stops for this city (its terrain "
            "leaks)",
            type);
        g_s.ours = nullptr;
    }
    g_game = GameCtx{};
}

class Target final : public cIGZMessageTarget2 {
public:
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid != kIID_cIGZMessageTarget2 && riid != GZIID_cIGZUnknown) return false;
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return static_cast<uint32_t>(InterlockedIncrement(&refs_)); }
    uint32_t Release() override {
        const LONG r = InterlockedDecrement(&refs_);
        return r > 0 ? static_cast<uint32_t>(r) : 0u;
    }
    bool DoMessage(cIGZMessage2* msg) override {
        if (msg == nullptr) return true;
        FC4_X87_SCOPE("terrain shadow DoMessage", msg->GetType());  // x87 leftover guard (FC4Fpu.h)
        OnMessage(msg);
        return true;
    }

private:
    volatile LONG refs_ = 0;
};

Target& TheTarget() {
    static Target t;
    return t;
}

// The city simulator's date now (slot 8; the published one without it): the
// terrain messages arrive late, so this is the delivery's date.
int32_t SimJulian(cISC4City* city) {
    cFC4ISC4Simulator* sim = city ? fc4::AsGame(city->GetSimulator()) : nullptr;  // borrowed
    const int32_t j = sim ? static_cast<int32_t>(sim->GetSimDateNumber()) : 0;
    return j > 0 ? j : static_cast<int32_t>(PublishedJulian());
}

cISC4City* CurrentCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    FC4_ACQUIRED(sc4, "QueryInterface");
    cISC4City* city = sc4->GetCity();  // borrowed
    FC4_RELEASE(sc4);
    return city;
}

void ReadSettings() {
    g_enabled = SettingInt("terrain_shadow", "FC4_TERRAIN_SHADOW", 0) != 0;
    g_points = std::min(4096L, std::max(16L, SettingInt("terrain_shadow_points", "FC4_TERRAIN_SHADOW_POINTS",
                                                        kDefaultPoints)));
    long d = SettingInt("terrain_shadow_diffs", "FC4_TERRAIN_SHADOW_DIFFS", kDefaultDiffs);
    g_diffs = d <= 0 ? kDefaultDiffs : std::min(64L, d);
    g_lines = std::min(1000L, std::max(0L, SettingInt("terrain_shadow_lines", "FC4_TERRAIN_SHADOW_LINES",
                                                      kDefaultLines)));
}

// ---- the test seam's stand-in original --------------------------------------------------------------------

struct StandIn {
    fc4_terrain* t = nullptr;
    uint32_t perturb = 0;
};
StandIn* S(void* u) { return static_cast<StandIn*>(u); }
float SAltV(void* u, int32_t x, int32_t z) { return fc4_terrain_altitude_at_vertex(S(u)->t, x, z); }
float SDepthV(void* u, int32_t x, int32_t z) { return fc4_terrain_water_depth_at_vertex(S(u)->t, x, z); }
float SSea(void* u) { return fc4_terrain_sea_level(S(u)->t); }
float SCached(void* u, int w) { return fc4_terrain_cached_height(S(u)->t, static_cast<uint32_t>(w)); }
float SAllowed(void* u, int w) { return fc4_terrain_allowed_height(S(u)->t, static_cast<uint32_t>(w)); }
float SAlt(void* u, float x, float z) {
    const float a = fc4_terrain_altitude(S(u)->t, x, z);
    const float half = 8.0f * static_cast<float>(fc4_terrain_cell_count(S(u)->t, 0));
    return (S(u)->perturb & 1) != 0 && x > half ? a + 1.0f : a;
}
void SNormal(void* u, float x, float z, float* out) { fc4_terrain_face_normal(S(u)->t, x, z, out); }
bool SWater(void* u, int32_t x, int32_t z) { return fc4_terrain_cell_has_water(S(u)->t, x, z); }
bool SLand(void* u, int32_t x, int32_t z) { return fc4_terrain_cell_has_land(S(u)->t, x, z); }
float SDepthCell(void* u, int32_t x, int32_t z) {
    return (S(u)->perturb & 2) != 0 ? 0.0f : fc4_terrain_water_depth_for_cell(S(u)->t, x, z);
}
int32_t SType(void* u, int32_t x, int32_t z) { return static_cast<int32_t>(fc4_terrain_water_type(S(u)->t, x, z)); }
bool SNav(void* u, int32_t x, int32_t z, int32_t r) { return fc4_terrain_is_navigable(S(u)->t, x, z, r, 0); }

Original StandInOriginal(StandIn* si) {
    Original o;
    o.u = si;
    o.cellsX = fc4_terrain_cell_count(si->t, 0);
    o.cellsZ = fc4_terrain_cell_count(si->t, 1);
    o.altitudeAtVertex = &SAltV;
    o.depthAtVertex = &SDepthV;
    o.seaLevel = &SSea;
    o.cachedHeight = &SCached;
    o.allowedHeight = &SAllowed;
    o.altitude = &SAlt;
    o.faceNormal = &SNormal;
    o.cellHasWater = &SWater;
    o.cellHasLand = &SLand;
    o.depthForCell = &SDepthCell;
    o.waterType = &SType;
    o.isNavigable = &SNav;
    return o;
}

// The stand-in's queued messages, followed as the game would deliver them.
std::vector<fc4_terrain_post> TakeStandInPosts(StandIn* si) {
    fc4_terrain_post posts[16] = {};
    const size_t n = fc4_terrain_take_posts(si->t, posts, 16);
    Drain(si->t);
    return std::vector<fc4_terrain_post>(posts, posts + std::min<size_t>(n, 16));
}

void DeliverPosts(const std::vector<fc4_terrain_post>& posts, int32_t julian) {
    for (const fc4_terrain_post& p : posts) {
        if (p.msg_type == kMsgTerrainChanged || p.msg_type == kMsgWaterChanged) {
            Change(p.msg_type, p.data1, p.data2, p.data3, p.data4, julian);
        }
    }
}

void DeliverStandInPosts(StandIn* si, int32_t julian, bool deliver) {
    const std::vector<fc4_terrain_post> posts = TakeStandInPosts(si);
    if (deliver) DeliverPosts(posts, julian);
}

}  // namespace

void TerrainShadowStartup(bool terrainReplaced) {
    ReadSettings();
    g_replaced = terrainReplaced;
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("terrain_shadow", "FC4_TERRAIN_SHADOW", &src);
    if (terrainReplaced) {
        Log("terrain shadow: requested (terrain_shadow from %s) but INACTIVE: it compares against the ORIGINAL "
            "cSTETerrain, and terrain is enabled (ours)",
            src);
        return;
    }
    Log("terrain shadow: ON (terrain_shadow from %s): at PostCityInit a FreeCity4 terrain is built from the ORIGINAL "
        "cSTETerrain's altitudes and sea level (read-only), its rebuilt water compared with the original's depths "
        "(then copied) and every cell compared; each terrain / water-changed message (0x278E6003 / 0x678E6E3A) "
        "re-reads its rectangle (moved vertices replayed through slot 50), checks the packing and the cache and "
        "compares the rectangle's cells; each month every vertex is re-read (changes ahead of their queued message kept pending, unfollowed ones counted) and a "
        "sample compared (points %ld; differences named %ld; change lines %ld): GetAltitude, GetFaceNormal, "
        "CellHasWater, CellHasLand, GetWaterDepthForCell, GetWaterType, IsNavigable(r = 0..2, no lot)",
        src, g_points, g_diffs, g_lines);
}

void TerrainShadowStart() {
    if (g_started || !g_enabled || g_replaced) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) {
        Log("terrain shadow: no message server; not started");
        return;
    }
    TheTarget().AddRef();  // our own reference (a static object)
    bool ok = true;
    for (uint32_t m : kMessages) ok = ms->AddNotification(static_cast<cIGZMessageTarget2*>(&TheTarget()), m) && ok;
    g_started = true;
    Log("terrain shadow: listening (SimNewMonth, terrain / water / sea level changed, long operations)%s",
        ok ? "" : " [WARNING a notification was refused]");
}

void TerrainShadowStop() {
    if (!g_started) return;
    g_started = false;
    cIGZMessageServer2Ptr ms;
    if (ms) {
        for (uint32_t m : kMessages) ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(&TheTarget()), m);
    }
    TheTarget().Release();
}

void TerrainShadowCityLoaded(cISC4City* city) {
    if (!g_started) return;
    DropShadow();
    if (!BindGame(city)) {
        Log("terrain shadow load: INCOMPLETE (no city, terrain or surface water); not compared");
        g_game = GameCtx{};
        return;
    }
    Event e;
    e.julian = SimJulian(city);
    if (!SehGuard("terrain shadow (load)", &LoadStep, &e)) {
        Log("terrain shadow: a fault while reading the original at load; not compared for this city");
        g_s.ours = nullptr;
    }
    g_game = GameCtx{};
}

void TerrainShadowCityShutdown() {
    if (g_s.ours == nullptr || g_s.standIn) return;
    ShutdownLine();
    DropShadow();
}

void TerrainShadowScriptedSetAltitude(int32_t x0, int32_t z0, int32_t x1, int32_t z1, float h, uint32_t kind,
                                      int32_t julian) {
    if (g_s.ours == nullptr || g_s.standIn) return;
    Event e;
    e.rect[0] = x0;
    e.rect[1] = z0;
    e.rect[2] = x1;
    e.rect[3] = z1;
    e.h = h;
    e.d[3] = kind;
    e.julian = julian;
    if (!BindGame(CurrentCity())) return;
    if (!SehGuard("terrain shadow (script)", &ScriptStep, &e)) {
        Log("terrain shadow: a fault while mirroring the scripted SetAltitude; the shadow stops for this city");
        g_s.ours = nullptr;
    }
    g_game = GameCtx{};
}

}  // namespace fc4

intptr_t TerrainShadowTest(FC4TerrainShadowTestInput* in) {
    using namespace fc4;
    if (in == nullptr || in->altitudes == nullptr || in->cellsX < 4 || in->cellsZ < 4) return -1;
    fc4_terrain_host none{};
    none.struct_size = sizeof(none);
    StandIn si;
    si.perturb = in->perturb;
    si.t = fc4_terrain_create_from_altitudes(&none, nullptr, in->cellsX, in->cellsZ, in->altitudes, in->count,
                                             in->seaLevel);
    if (si.t == nullptr) return -1;
    fc4_terrain_post_city_init(si.t);
    Drain(si.t);
    const long points = g_points, diffs = g_diffs, lines = g_lines;
    g_points = 64;
    g_diffs = 4;
    g_lines = 8;
    DropShadow();
    g_s.standIn = true;
    g_s.src = StandInOriginal(&si);
    g_s.host = none;
    const int32_t j0 = 2461068;  // 2026-01-27 (Konradshohe's first day)
    Load(j0);
    if (g_s.ours == nullptr) {
        fc4_terrain_destroy(si.t);
        g_s = Shadow{};
        g_points = points;
        g_diffs = diffs;
        g_lines = lines;
        return -1;
    }
    Month(j0 + 5);
    // A change of the original: a raised block, its messages followed (or not).
    fc4_terrain_rect r{1, 1, 3, 2};
    fc4_terrain_set_altitude_rect(si.t, r, fc4_terrain_altitude_at_vertex(si.t, 1, 1) + 7.5f, true, 4);
    // 2: its messages held past the next month (queued posts delivered late).
    const std::vector<fc4_terrain_post> held = TakeStandInPosts(&si);
    if (in->withoutMessage == 0) DeliverPosts(held, j0 + 10);
    // A lowered vertex through slot 50 (the network grading's setter, §8.2 kind 4).
    const uint32_t vi = fc4_terrain_vert_index(si.t, 2, 3);
    fc4_terrain_set_altitude_at_vertex(si.t, vi, fc4_terrain_sea_level(si.t) - 3.0f);
    fc4_terrain_redisplay(si.t, true, false, fc4_terrain_rect{2, 3, 2, 3}, 4);
    DeliverStandInPosts(&si, j0 + 11, in->withoutMessage != 1);
    Month(j0 + 36);
    if (in->withoutMessage == 2) DeliverPosts(held, j0 + 37);
    // The script action's call, mirrored.
    fc4_terrain_rect sr{0, 0, 2, 1};
    const float sh = fc4_terrain_sea_level(si.t) + 12.25f;
    fc4_terrain_set_altitude_rect(si.t, sr, sh, true, 1);
    Scripted(sr.x0, sr.z0, sr.x1, sr.z1, sh, 1, j0 + 40);
    DeliverStandInPosts(&si, j0 + 40, in->withoutMessage != 1);
    Month(j0 + 64);
    ShutdownLine();
    in->loadMismatches = g_s.loadMismatches;
    in->changeMismatches = g_s.changeMismatches;
    in->scriptMismatches = g_s.scriptMismatches;
    in->monthMismatches = g_s.monthMismatches;
    in->unfollowed = g_s.unfollowedTotal;
    in->late = g_s.lateTotal;
    in->compared = g_s.totals.Total();
    in->changesSeen = g_s.changesSeen;
    in->vertexChecks = g_s.vertexChecks;
    in->lines = g_s.lines;
    in->cleanLines = g_s.cleanLines;
    const intptr_t total =
        static_cast<intptr_t>(g_s.loadMismatches + g_s.changeMismatches + g_s.scriptMismatches + g_s.monthMismatches);
    DropShadow();
    fc4_terrain_destroy(si.t);
    g_points = points;
    g_diffs = diffs;
    g_lines = lines;
    return total;
}
