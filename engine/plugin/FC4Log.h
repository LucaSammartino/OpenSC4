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

// Info-level line that never blocks for long on the log lock: it waits at
// most `waitMs` (0 = not at all), then appends through a second handle
// without the lock. No heap allocation and no other lock, so it is safe in
// the vectored exception handler and in the watchdog. Lines before the log
// was first opened are dropped (counted).
void LogWait(unsigned waitMs, const char* format, ...);
unsigned long LogUnlockedLines();
unsigned long LogDroppedLines();

// Full path of the log file in use (empty until the first line is written).
const char* LogPath();

// Full path of this DLL.
const char* ModulePath();

// Closes the log handle (DLL detach). A later line reopens the same path.
void LogClose();
// FlushFileBuffers on the log handles (no lock; any thread, also before TerminateProcess).
void LogFlush();

}  // namespace fc4

#define FC4_LOGV(...)                                                  \
    do {                                                               \
        if (::fc4::LogLevel() >= ::fc4::kLogVerbose) ::fc4::LogVerboseImpl(__VA_ARGS__); \
    } while (0)
