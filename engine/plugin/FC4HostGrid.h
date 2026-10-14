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
// simulators' glue (power, plumbing, protection, flora).
//
// Grids are created through GZCOM with the element type's class ID (the game
// registers 0x49B9E601-0x49B9E60B; we never register them), then Init,
// SetTractSize, SetInstanceID. A token is a cISC4SimGrid<T>* holding one
// reference. Supported element types: uint8 (0x49B9E602, also "bool"
// 0x49B9E601), int16 (0x49B9E605) and float (0x49B9E60A), the ones
// docs/abi/interfaces.tsv checks. [choice] Other types are refused (create
// returns 0) until a subsystem needs them. uint16 grids (0x49B9E604) the game
// hands out can be read (get) but are never created or written.
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

}  // namespace fc4
