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

// The data-formats shadow (glue-formats4; data-formats.md §2-§5, §8): with
// `formats_shadow = 1` (a shadow key: there is no "formats" subsystem to
// enable, nothing is replaced), at the first city load every
// `*.dat` file of the install folder (§3.1 step 1, directory-listing order) is
// opened by the core's DBPF reader (fc4_dbpf_*, engine/formats), and each
// exemplar (type 0x6534284A) is parsed by the core's exemplar reader. Its own
// properties are compared with those of the exemplar the game's resource
// manager loaded for the same key (cISCPropertyHolder EnumProperties, own list
// only; FC4Properties.h PropertyIds / CopyToCore): the same IDs, and for each
// the same variant type, shape and bytes (fc4_props_write_variant). The game
// keeps its own exemplar factory and segments (§8: plug-in mode uses the
// originals); nothing is replaced and nothing in the game is changed.
//
// One summary line per file and one in total; up to formats_shadow_lines
// (default 8) detail lines for differences. `formats_shadow_max` (default 0 =
// all) caps the exemplars compared per file.
#pragma once

namespace fc4 {
// FC4Subsystems.cpp, at plug-in startup: reads the switches, logs ON or nothing.
void FormatsShadowStartup();
// FC4Observer.cpp, after each city load: runs the comparison once per session.
void FormatsShadowCityLoaded();
}  // namespace fc4
