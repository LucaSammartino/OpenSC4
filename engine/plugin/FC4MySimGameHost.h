// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

// The read-only city services of fc4_mysim_host (mysims.md §4.5-§4.10): the
// cells, disasters, buildings, lots, residential values, grids, coverage,
// utilities and stations the MySims read. Shared by the MySims shadow
// (FC4MySimsShadow.cpp) and the MySim replacement (cFC4MySim.cpp,
// `enable = mysims`). Everything is read through the city's interfaces of
// the moment (ours or the original's); nothing is written.
#pragma once

#include "fc4.h"

namespace fc4 {

// Sets the city/building/grid callbacks of `h` (the Lua, dispatch, message
// and reference callbacks are the caller's).
void FillMySimGameServices(fc4_mysim_host* h);

}  // namespace fc4
