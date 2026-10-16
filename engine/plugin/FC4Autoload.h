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

// Autoload: open a city save at startup, so unattended in-game runs need no
// click (tooling only; no spec behaviour).
//
// FreeCity4.ini `autoload_city = <full path of a .sc4 save>` (else
// FC4_AUTOLOAD_CITY). Once the region view is up (the app has a region with
// cities) and `autoload_delay_ticks` framework ticks (default 60) have passed,
// the observer's tick finds the region's city whose save path is that file
// and calls cISC4App::LoadCity(path, <that city's entry in the region>)
// (compiled slot 13). The save must belong to the region the game opens at
// start (the last region used). While the region lists no cities, the list is
// read again about once a second (wall clock), and the autoload gives up only
// 120 s after the first empty read (FC4AutoloadLogic.h), logged as
//   autoload: the region lists no cities yet (<n>); retrying in 1000 ms (<ms> ms since the first check)
//   autoload: the region lists no cities; giving up after <ms> ms (<n> checks)
// One attempt per process; logged as
//   autoload: LoadCity(<path>) -> <result>
#pragma once

namespace fc4 {

// Every framework tick (observer, main thread).
void AutoloadOnTick();
// PostCityInit seen (a city is loaded: nothing left to do).
void AutoloadOnCityLoaded();

}  // namespace fc4
