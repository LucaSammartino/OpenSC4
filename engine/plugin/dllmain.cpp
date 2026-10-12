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

// DLL entry point: only diagnostics (which copy of the DLL was loaded, and
// when it is unloaded). Keep this minimal: it runs under the loader lock.

#include "FC4Log.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

BOOL WINAPI DllMain(HINSTANCE hinstDLL, DWORD fdwReason, LPVOID lpvReserved) {
    switch (fdwReason) {
        case DLL_PROCESS_ATTACH: {
            DisableThreadLibraryCalls(hinstDLL);
            char exe[MAX_PATH] = {};
            GetModuleFileNameA(nullptr, exe, MAX_PATH);
            fc4::Log("DLL_PROCESS_ATTACH: %s (base %p) loaded into %s (%s load)", fc4::ModulePath(),
                     static_cast<void*>(hinstDLL), exe, lpvReserved ? "static" : "dynamic");
            break;
        }
        case DLL_PROCESS_DETACH:
            fc4::Log("DLL_PROCESS_DETACH (%s)", lpvReserved ? "process exit" : "FreeLibrary");
            break;
        default:
            break;
    }
    return TRUE;
}
