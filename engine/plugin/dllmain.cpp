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

// DLL entry point: only diagnostics (which copy of the DLL was loaded, crash
// tracing for our image range, unload). Keep it minimal: it runs under the
// loader lock.

#include "FC4Checked.h"
#include "FC4Diag.h"
#include "FC4FailFast.h"
#include "FC4Fpu.h"
#include "FC4GrowthProbe.h"
#include "FC4Log.h"
#include "FC4TractShadow.h"
#include "FC4DisasterShadow.h"

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
            fc4::Log("log file: %s", fc4::LogPath());
            fc4::FpuAttach();
            fc4::DiagAttach(hinstDLL);
            fc4::FailFastAttach();
            fc4::CheckedAttach();
            fc4::FpuLog("DLL attach (after our initialisation)");
            break;
        }
        case DLL_PROCESS_DETACH:
            fc4::Log("DLL_PROCESS_DETACH (%s)", lpvReserved ? "process exit" : "FreeLibrary");
            fc4::GrowthProbeDetach();  // the forwarder lives in this DLL: put the original Grow back
            fc4::DisastersShadowDetach();  // the disasters shadow's forwarders
            fc4::TractShadowDetach();  // likewise the tract shadow's UpdatePopulation forwarder
            fc4::DiagDetach();
            fc4::LogClose();
            break;
        default:
            break;
    }
    return TRUE;
}
