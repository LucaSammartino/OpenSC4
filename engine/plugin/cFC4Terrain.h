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

// The terrain replacement: cSTETerrain and cSTESurfaceWater over fc4_terrain
// (docs/specs/terrain.md). Off by default, `enable = terrain` in FreeCity4.ini
// (FC4Subsystems.cpp: the registry CLSID takes precedence over the game's, so
// the city builds these from the start and the original classes never run).
//
// The two classes share ONE fc4_terrain handle: the original's surface water is
// owned by the terrain (terrain +0x24) and the core holds the depths and the
// bodies-of-water map together with the height field.
//
// TR18 (terrain.md §15.4) is enforced at THIS boundary as well as in the core.
// The host callbacks the core calls while it holds the handle (a brush's
// random_uniform, the lava's fire and flammability calls, the navigability
// host, and the message posts) read or drive the GAME only. They never call
// back into fc4_terrain for this handle: a call that tries is refused, counted
// in `reentrant_guard_refused`, and logged. The core's own re-entrant lock
// refuses and counts too (`fc4_terrain_state.reentrant_refused`); both counts
// are expected to stay 0 on every simulation path (§15.4: the lava and the
// kinds 0/2/5/7 brushes never re-enter). God-mode brush kinds 1 and 8-10 do
// call the flora and the surface water by design; the glue never drives them,
// and if a future caller does, that work must run outside the borrow.
//
// TR9/TR23 (§2.3, quirk 13): the default brush level is the view's zoom index.
// The view-update message 0xA6B79602 carries it at *GetVoid2(). The initial
// copy the original makes in DoPostCityInit from the 3D camera (global
// 0x00B43DD8, +0xC) is NOT reproduced: the plugin has no camera interface (see
// the numbered question in the lane report). Until the first view message the
// level stays 0, which is also what a harness with no view sees.
#pragma once

#include "FC4TerrainInterfaces.h"

#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"

#include <cstdint>

namespace fc4 {
class CoreLock;
}

// Test seam op 59 (FC4TestSeam.h): the replacement on a stand-in world, through
// the same code as in game. `a` is the input, `b` receives the counters.
struct FC4TerrainGlueTestInput {
    uint32_t struct_size;
    uint32_t cellsX, cellsZ;
    const float* altitudes;  // (cellsX + 1) x (cellsZ + 1), index order, or null
    size_t count;
    float seaLevel;
    uint32_t op;  // 0 = the whole scenario; see cFC4Terrain.cpp
    // Out.
    uint32_t slotsChecked;        // vtable slots that answered as expected
    uint32_t reentrantGuardRefused;  // TR18: a host callback tried the handle
    uint32_t coreReentrantRefused;   // the core's own count
    uint32_t postsSeen;           // queued messages handed to the host
    uint32_t brushLevel;          // the default brush level after the scenario
    uint32_t mismatches;
};
intptr_t TerrainGlueTest(FC4TerrainGlueTestInput* in);

// The replacement classes (cFC4Terrain.cpp).
cIGZUnknown* CreateFC4Terrain();
cIGZUnknown* CreateFC4SurfaceWater();
