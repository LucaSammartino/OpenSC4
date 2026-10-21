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

// Factories of the layout-compatible grid family (docs/specs/simgrid.md §2;
// FC4SimGridLogic.h). Each returns a count-0 object (§6.1), or nullptr
// without the game's STL node allocator.
#pragma once

#include "cIGZUnknown.h"

cIGZUnknown* CreateFC4SimGridBool();  // 0x49B9E601 (a uint8 grid)
cIGZUnknown* CreateFC4SimGridU8();    // 0x49B9E602
cIGZUnknown* CreateFC4SimGridS8();    // 0x49B9E603
cIGZUnknown* CreateFC4SimGridU16();   // 0x49B9E604
cIGZUnknown* CreateFC4SimGridS16();   // 0x49B9E605
cIGZUnknown* CreateFC4SimGridU32();   // 0x49B9E606
cIGZUnknown* CreateFC4SimGridS32();   // 0x49B9E607
cIGZUnknown* CreateFC4SimGridU64();   // 0x49B9E608
cIGZUnknown* CreateFC4SimGridS64();   // 0x49B9E609
cIGZUnknown* CreateFC4SimGridF32();   // 0x49B9E60A
cIGZUnknown* CreateFC4SimGridF64();   // 0x49B9E60B
