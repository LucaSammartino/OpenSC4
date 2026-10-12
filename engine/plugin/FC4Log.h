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

// Plain-text log written to FreeCity4.log next to FreeCity4.dll (falls back
// to %TEMP%\FreeCity4.log if that directory is not writable).
#pragma once

namespace fc4 {

void Log(const char* format, ...);

// Full path of the log file in use (empty until the first Log call).
const char* LogPath();

}  // namespace fc4
