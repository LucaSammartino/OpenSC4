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

// Tooling (`simgrids_probe = 1`, default off): record digests of the grids a
// save hands to our glue, logged at PostCityInit (FC4SimGridProbe.cpp).
#pragma once

#include <cstdint>

class cIGZUnknown;

namespace fc4 {

// From IStreamAdapter::ReadRef: `obj` (borrowed) was read with class `clsid`.
void SimGridProbeNoteRead(cIGZUnknown* obj, uint32_t clsid);
// From the observer's PostCityInit: log each held grid's record digest, then
// release them.
void SimGridProbeAtCityLoaded();

}  // namespace fc4
