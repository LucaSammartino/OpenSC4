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

// fc4_grid_api over the game's own grids (simgrid.md §6.4), shared by the grid
// simulators' glue (power, plumbing, protection, flora, aura).
//
// Grids are created through GZCOM with the element type's class ID (the game
// registers 0x49B9E601-0x49B9E60B; we never register them), then Init,
// SetTractSize, SetInstanceID. A token is a cISC4SimGrid<T>* holding one
// reference. Supported element types: uint8 (0x49B9E602, also "bool"
// 0x49B9E601), int8 (0x49B9E603), uint16 (0x49B9E604), int16 (0x49B9E605)
// and float (0x49B9E60A), the ones docs/abi/interfaces.tsv checks. [choice]
// Other types are refused (create returns 0) until a subsystem needs them.
#pragma once

#include "FC4Config.h"
#include "fc4.h"

namespace fc4 {

struct HostGridContext {
    const char* tag;      // log prefix ("power", ...)
    Sw writes;            // switch for SetTractValue / SetTractValues
};

// A grid table whose `user` is `ctx` (which must outlive the table's users).
fc4_grid_api MakeGridApi(const HostGridContext* ctx);

// The two calls that own a grid reference, for glue whose core has its own
// grid callbacks (land value, fc4_landvalue_host) rather than fc4_grid_api.
// The table's create and release are these.
//
// A new game grid of class `kind` (GetClassObject, noted FC4_ACQUIRED; Init,
// SetTractSize, SetInstanceID): a token holding that one reference, or 0
// (logged with `tag`).
uint64_t CreateGameGrid(const char* tag, uint32_t kind, uint32_t instance, int32_t tractSize);
// Gives back the reference a grid token holds: the one CreateGameGrid took,
// or the one a save handed over (IStreamAdapter::ReadRef, noted FC4_ACQUIRED).
// Either is ours, so reaching 0 is an expected free (FC4_RELEASE_CREATED), not
// a warning. Returns the count the Release returned (0: it was the last).
uint32_t ReleaseGameGrid(uint64_t token);

}  // namespace fc4
