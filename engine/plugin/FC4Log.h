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
// to %TEMP%\FreeCity4.log if that directory is not writable). Every line is
// flushed and the file closed immediately, so nothing is lost on a crash.
//
// Verbosity comes from the environment variable FC4_LOG_LEVEL, read once:
//   0 = off, 1 = info (lifecycle lines), 2 = verbose (call tracing; default).
#pragma once

namespace fc4 {

constexpr int kLogOff = 0;
constexpr int kLogInfo = 1;
constexpr int kLogVerbose = 2;

int LogLevel();

// Info-level line.
void Log(const char* format, ...);

// Verbose line; use the FC4_LOGV macro so arguments are not evaluated when off.
void LogVerboseImpl(const char* format, ...);

// Full path of the log file in use (empty until the first line is written).
const char* LogPath();

// Full path of this DLL.
const char* ModulePath();

}  // namespace fc4

#define FC4_LOGV(...)                                                  \
    do {                                                               \
        if (::fc4::LogLevel() >= ::fc4::kLogVerbose) ::fc4::LogVerboseImpl(__VA_ARGS__); \
    } while (0)
