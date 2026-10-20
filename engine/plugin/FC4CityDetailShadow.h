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

// The city detail shadow: a same-process differential of the city detail
// manager (city-detail.md; tooling, no spec behaviour). Opt-in, off by
// default:
//
//   citydetail_shadow = 1        (env FC4_CITYDETAIL_SHADOW)
//
// It needs the ORIGINAL class (`citydetail` not enabled). The city's own
// instance cannot be reached (§6: nothing outside the city holds it), so at
// each city load (PostCityInit) the shadow makes a fresh ORIGINAL object
// (GZCOM class 0xEAD4BA36) and a fresh shadow core (read-only host) and puts
// both through the same inputs, every one of which leaves the city alone
// (§3.1: no pass when the stored level is none or the current setting):
//   - fresh: Init; the level (slot 5) and the record (§4, Write) compared;
//   - Read of `00000000 <setting>` then Init; Read's result, the level and
//     the record compared;
//   - Read of `00000000 FFFFFFFF` then Init (none): the same;
//   - Read of `01000000 <setting>` (version 1): both must refuse it.
// The other levels would run a pass on the original (it would change the
// city): only ours runs them, with a recording host, and the line gives what
// its pass would do (not compared).
//
// Log lines (always written, not the verbose channel):
//   citydetail shadow: ON (...) | requested but INACTIVE (...)        game start
//   citydetail shadow load YYYY-MM-DD: setting=s fresh=a/b read-same=.. read-none=.. read-v1=..;
//     dry runs: level 1 -> pass ..; mismatches=N[; differences: ...]
#pragma once

class cISC4City;

namespace fc4 {

void CityDetailShadowStartup(bool cityDetailReplaced);

// FC4Observer.cpp, PostCityInit.
void CityDetailShadowCityLoaded(cISC4City* city);

}  // namespace fc4
