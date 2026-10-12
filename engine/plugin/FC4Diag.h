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

// Crash diagnostics for in-game testing.
#pragma once

#include <cstdint>

namespace fc4 {

// Records this DLL's image range (from DllMain) and installs a first-chance
// vectored exception handler that logs every exception raised at an address
// inside the image (code, address, offset from the base). It never handles
// the exception: the search continues normally.
void DiagAttach(void* moduleBase);
void DiagDetach();

uintptr_t ImageBase();
uintptr_t ImageSize();

// Runs fn(ctx) under a Win32 structured-exception guard. If a structured
// exception escapes, logs `what`, the exception code, address and offset from
// our image base, and returns false; otherwise returns true. C++ objects in
// frames between here and the fault are not unwound, so this is a
// diagnostic last resort.
bool SehGuard(const char* what, void (*fn)(void* ctx), void* ctx);

// Records an entry into one of our interface methods in a ring buffer of the
// last 64 calls, and (verbose log level) logs the first 20 calls of each call
// site. The ring is written to the log when an exception is raised OUTSIDE
// our image (the first 8 such exceptions: the game may handle some itself),
// together with the faulting module and offset and any return addresses into
// our image found on the stack. `counter` is the call site's own counter.
void CallTrace(const char* what, unsigned* counter);

}  // namespace fc4

#define FC4_TRACE(NAME)                     \
    do {                                    \
        static unsigned fc4TraceCount_ = 0; \
        ::fc4::CallTrace(NAME, &fc4TraceCount_); \
    } while (0)

namespace fc4 {

}  // namespace fc4
