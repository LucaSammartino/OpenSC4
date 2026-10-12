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

// Plain-text log written to FreeCity4.log next to FreeCity4.dll (or, if that
// file cannot be opened at all, %TEMP%\FreeCity4.log, which then says so in
// its first line). The path never changes afterwards. The file is opened with
// full sharing; each line is written straight to the OS, so nothing is lost
// on a crash. Failed writes are retried once and reported on the next line
// that succeeds.
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

// Closes the log handle (DLL detach). A later line reopens the same path.
void LogClose();

}  // namespace fc4

#define FC4_LOGV(...)                                                  \
    do {                                                               \
        if (::fc4::LogLevel() >= ::fc4::kLogVerbose) ::fc4::LogVerboseImpl(__VA_ARGS__); \
    } while (0)
