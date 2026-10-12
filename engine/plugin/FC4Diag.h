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

}  // namespace fc4
