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

// The terrain shadow: a same-process, read-only differential of the
// terrain core (fc4_terrain, docs/specs/terrain.md) against the ORIGINAL
// cSTETerrain / cSTESurfaceWater (tooling; no spec behaviour of its own).
// Opt-in, off by default:
//
//   terrain_shadow = 1          (env FC4_TERRAIN_SHADOW)
//   terrain_shadow_points = N   sample points per month (env
//                               FC4_TERRAIN_SHADOW_POINTS, default 256, 16..4096)
//   terrain_shadow_diffs = N    differences named per line (env
//                               FC4_TERRAIN_SHADOW_DIFFS, default 8, 1..64)
//   terrain_shadow_lines = N    change lines per month (env
//                               FC4_TERRAIN_SHADOW_LINES, default 8, 0..1000;
//                               the rest are counted in the month line)
//
// There is no terrain replacement: the city's terrain is always the
// original's, and the shadow never writes to it. Our simulator is not needed
// (the shadow has its own message subscription).
//   - At PostCityInit a FreeCity4 terrain is built from the original's state
//     read through its interfaces: the cell counts (terrain slots 7, 8), every
//     vertex altitude (slot 14) and the sea level (slot 29). Its water is
//     rebuilt from the sea level (fc4_terrain_create_from_altitudes, §10.1)
//     and compared vertex by vertex with the original's depths (slot 33);
//     then the original's depths are copied (lakes and AnimateWater are not
//     modelled at load), and its cached minimum and maximum (slots 21, 22;
//     §4.6: the saved, possibly stale values) are taken. The allowed heights
//     (slots 23, 24) are compared with the shipped tuning. Every cell is then
//     compared (CellHasWater, CellHasLand, GetWaterDepthForCell, GetWaterType,
//     IsNavigable r = 0) and a sample of points (GetAltitude, GetFaceNormal).
//   - On every terrain-changed message 0x278E6003 and water-changed message
//     0x678E6E3A (§8.1; data1/data2 pack the inclusive vertex rectangle,
//     data4 the kind of §8.2) the rectangle is re-read: the message does not
//     say which setter ran, so each moved vertex is replayed through our
//     SetAltitudeAtVertex (slot 50 rule, §6) and the depth it gives is
//     classified against the original's (slot 50 rule, the Raise/Lower rule
//     `sea - alt`, the SetAltitude(rect) reset rule, or none) per kind; then
//     the original's depth is copied. Our RedisplayTerrain is called with the
//     message's flags, rectangle and kind: its post must carry the same four
//     data words (the packing), and with the terrain flag our recomputed
//     minimum and maximum must equal the original's. The queries are then
//     compared on every cell of the rectangle grown by one (capped).
//   - The script action `terrain` (FC4Script.h) calls the original's
//     SetAltitude(rect, h, true, kind) (slot 49): the shadow mirrors that
//     exact call with fc4_terrain_set_altitude_rect and compares every vertex
//     of the rectangle (altitude and depth) and the cached heights at once.
//   - At each SimNewMonth: every vertex is re-read. The terrain messages are
//     queued posts, delivered after the change (under turbo, weeks of game
//     time later), so a vertex that differs from ours is usually ahead of its
//     message: it is copied and kept pending ("ahead"); the change message
//     that covers it puts ours back first and follows it ("late"). A pending
//     vertex no message covered by the next month was changed without one
//     ("unfollowed", not a core difference). Then the cached heights, the sea
//     level and a deterministic sample (seeded by the date) of the queries are
//     compared: GetAltitude and GetFaceNormal at points inside, on cell edges
//     and diagonals, on and just outside the city edge; CellHasWater,
//     CellHasLand on cells including one ring outside; GetWaterDepthForCell
//     and GetWaterType on cells inside (the original does not bounds-check
//     them); IsNavigable(x, z, r, null) with r = 0, 1, 2, half of the cells
//     drawn from the cells that have water.
//
// Floats are compared bit for bit. The navigability host (bridges: the
// traffic network map's FindNetworkTile(x, z, 0x1F4F, 1), network flag
// 0x40000, the passage ceiling of tile slot 67; power poles: occupant manager
// slot 41 with type 0x2890D4DE on the cell) reads the game only and never
// calls back into the terrain handle (TR18: the core refuses that; the
// refused count is in every line).
//
// Log lines (always written, not the verbose channel):
//   terrain shadow: ON (...) | requested but INACTIVE (...)                       game start
//   terrain shadow load yyyy-mm-dd: cells=XxZ sea=..; depths rebuilt from the sea level e/t; cache orig a/b,
//     fresh c/d (kept: orig); allowed heights e/t; queries <name> e/t ...; mismatches=N[; first: ...]
//   terrain shadow change yyyy-mm-dd terrain|water kind=K rect=x0,z0..x1,z1: vertices=V moved=M wet=W depth
//     rules slot50/raiselower/reset/none=a/b/c/d; message e/t; [late L (...); ][moved elsewhere E; cache ..; ]
//     queries ...; mismatches=N        (the date: the simulator's at delivery, not the change's)
//   terrain shadow script yyyy-mm-dd SetAltitude rect=..: h=.. kind=K: vertices e/t; cache e/t; mismatches=N
//   terrain shadow month yyyy-mm-dd: unfollowed=U[ (altitudes, depths only, rect, first)]; ahead=A; cache e/t;
//     sea e/t; queries ...; changes terrain=T
//     water=W kinds=k0/../k6 (lines logged L); refused=R; mismatches=N[; first: ...]
//   terrain shadow: city shutdown: N lines, M with mismatches=0; totals <name> e/t ...
#pragma once

#include <cstddef>
#include <cstdint>

class cISC4City;

namespace fc4 {

// Game start (FC4Subsystems.cpp): reads the settings and logs the state.
void TerrainShadowStartup();

// PostAppInit (cFC4Director): reads the settings, logs the state, and (when
// on) subscribes to SimNewMonth and the terrain messages. PreAppShutdown:
// unsubscribes.
void TerrainShadowStart();
void TerrainShadowStop();

// The observer's PostCityInit / PreCityShutdown (main thread).
void TerrainShadowCityLoaded(cISC4City* city);
void TerrainShadowCityShutdown();

// The script action `terrain` (FC4Script.cpp), after it called the
// original's SetAltitude(rect, h, true, kind) (slot 49): mirror the same call
// on ours and compare the rectangle. No effect when the shadow is off.
void TerrainShadowScriptedSetAltitude(int32_t x0, int32_t z0, int32_t x1, int32_t z1, float h, uint32_t kind,
                                      int32_t julian);

}  // namespace fc4

// Test seam op 58 (FC4TestSeam.h): the shadow on a stand-in original built
// inside the DLL from `altitudes` (a second fc4_terrain), through the same
// code as in game: load, a month, a change of the original (followed by its
// messages: `withoutMessage` 0; lost: 1; the first change's held past the
// next month, as the game's queued posts under turbo: 2), a month, a
// scripted SetAltitude, a month, shutdown. `perturb` makes the stand-in
// differ from the spec on purpose (1: its GetAltitude adds 1 m on the east
// half; 2: its GetWaterDepthForCell returns 0 on every cell).
struct FC4TerrainShadowTestInput {
    uint32_t cellsX, cellsZ;
    const float* altitudes;  // (cellsX + 1) x (cellsZ + 1), index order
    size_t count;
    float seaLevel;
    uint32_t perturb;
    uint32_t withoutMessage;
    // Out.
    uint32_t loadMismatches;
    uint32_t changeMismatches;
    uint32_t scriptMismatches;
    uint32_t monthMismatches;
    uint32_t unfollowed;     // over the months
    uint32_t compared;       // query comparisons over the scenario
    uint32_t changesSeen;    // terrain / water messages followed
    uint32_t vertexChecks;   // the scripted mirror's vertex comparisons
    uint32_t lines;          // lines written
    uint32_t cleanLines;     // lines with mismatches=0
    uint32_t late;           // vertices a month re-read found ahead of their message, then followed
};
// Returns the scenario's total mismatch count, -1 for a bad input.
intptr_t TerrainShadowTest(FC4TerrainShadowTestInput* in);
