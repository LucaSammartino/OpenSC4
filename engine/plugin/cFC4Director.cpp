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

// The FreeCity4.dll COM director. It registers the class factory for the
// simulator (CLSID 0x2990C1E5) so the game builds our cFC4Simulator instead
// of its own. Setting FC4_DISABLE_SIMULATOR=1 in the environment selects
// "passthrough" mode: the DLL loads and logs, but registers nothing, so the
// original simulator runs (for A/B comparisons).

#include "FC4Log.h"
#include "cFC4Simulator.h"

#include "cIGZFrameWork.h"
#include "cRZCOMDllDirector.h"
#include "fc4.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstring>

#ifndef FC4_VERSION_STRING
#define FC4_VERSION_STRING "0.1.0"
#endif

namespace {

// Random, unique director ID for FreeCity4.dll.
constexpr uint32_t kFC4DirectorID = 0x5C7A43E1;

bool PassthroughRequested() {
    char value[16] = {};
    const DWORD n = GetEnvironmentVariableA("FC4_DISABLE_SIMULATOR", value, sizeof(value));
    return n > 0 && n < sizeof(value) && strcmp(value, "1") == 0;
}

}  // namespace

class cFC4Director final : public cRZCOMDllDirector {
public:
    cFC4Director() : passthrough_(PassthroughRequested()) {
        if (!passthrough_) AddCls(kFC4_CLSID_cSC4Simulator, &cFC4Director::CreateSimulator);
    }

    uint32_t GetDirectorID() const override { return kFC4DirectorID; }

    bool OnStart(cIGZCOM* pCOM) override {
        fc4::Log("FreeCity4 %s loaded (core ABI %u, %s mode); log: %s", FC4_VERSION_STRING, fc4_abi_version(),
                 passthrough_ ? "PASSTHROUGH (FC4_DISABLE_SIMULATOR=1, original simulator in use)" : "replacement",
                 fc4::LogPath());
        cIGZFrameWork* fw = RZGetFrameWork();
        if (fw != nullptr) {
            if (fw->GetState() < cIGZFrameWork::kStatePreAppInit) {
                fw->AddHook(static_cast<cIGZFrameWorkHooks*>(this));
            } else {
                PostAppInit();
            }
        }
        return cRZCOMDllDirector::OnStart(pCOM);
    }

    bool PostAppInit() override {
        fc4::Log("PostAppInit (%s)", passthrough_ ? "passthrough: simulator class not registered"
                                                  : "simulator class 0x2990C1E5 provided by FreeCity4");
        return true;
    }

    bool PostAppShutdown() override {
        fc4::Log("PostAppShutdown");
        return true;
    }

private:
    static cIGZUnknown* CreateSimulator() {
        cFC4Simulator* sim = new cFC4Simulator();
        fc4::Log("cFC4Simulator created (%p)", static_cast<void*>(sim));
        // The director QueryInterfaces the object, which takes the first reference.
        return static_cast<cIGZUnknown*>(static_cast<cISC4Simulator*>(sim));
    }

    bool passthrough_;
};

cRZCOMDllDirector* RZGetCOMDllDirector() {
    static cFC4Director sDirector;
    return &sDirector;
}
