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

// Reports for the failures that end the process with a fail-fast
// (0xC0000409) and bypass the exception handlers (in-game run 23), every
// build:
//   - a Rust panic in the core (release builds abort): the panic message,
//     location, thread and backtrace (fc4_set_panic_log);
//   - an invalid parameter to our CRT (statically linked: our module only);
//   - a pure virtual call in our code.
// Each writes its lines and the last FreeCity4 interface calls to the log,
// flushes, and then lets the failure go on as before (abort / fail-fast).
#pragma once

namespace fc4 {

// DllMain (process attach), after DiagAttach.
void FailFastAttach();

// The panic sink's text as the log receives it: one log line per text line.
void FailFastLogText(const char* text, unsigned long len);

}  // namespace fc4
