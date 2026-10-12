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
//
// Every COM entry point is traced (FC4_LOG_LEVEL=2, the default) so in-game
// runs show exactly how the game talks to the director.

#include "FC4Log.h"
#include "cFC4Simulator.h"

#include "cIGZCOM.h"
#include "cIGZFrameWork.h"
#include "cIGZString.h"
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

// Calls to GetClassObject for other class IDs are logged up to this many times.
constexpr uint32_t kMaxOtherClassLogs = 500;

bool PassthroughRequested() {
    char value[16] = {};
    const DWORD n = GetEnvironmentVariableA("FC4_DISABLE_SIMULATOR", value, sizeof(value));
    return n > 0 && n < sizeof(value) && strcmp(value, "1") == 0;
}

// Set by the factory, so GetClassObject can tell whether it reached it.
uint32_t g_factoryCalls = 0;
// True while PostAppInit's diagnostic probe is running.
bool g_probing = false;

bool ProbeDisabled() {
    char value[16] = {};
    const DWORD n = GetEnvironmentVariableA("FC4_NO_PROBE", value, sizeof(value));
    return n > 0 && n < sizeof(value) && strcmp(value, "1") == 0;
}

struct EnumContext {
    cIGZCOMDirector::ClassObjectEnumerationCallback callback;
    void* context;
    uint32_t count;
};

void EnumThunk(uint32_t clsid, uint32_t reserved, void* pContext) {
    auto* ctx = static_cast<EnumContext*>(pContext);
    ++ctx->count;
    FC4_LOGV("EnumClassObjects: reporting clsid 0x%08X (reserved 0x%08X)", clsid, reserved);
    if (ctx->callback != nullptr) ctx->callback(clsid, reserved, ctx->context);
}

}  // namespace

class cFC4Director final : public cRZCOMDllDirector {
public:
    cFC4Director() : passthrough_(PassthroughRequested()) {
        fc4::Log("cFC4Director constructed (FreeCity4 %s, %s, dll %s, log level %d)", FC4_VERSION_STRING,
                 passthrough_ ? "passthrough" : "replacement", fc4::ModulePath(), fc4::LogLevel());
        if (!passthrough_) {
            AddCls(kFC4_CLSID_cSC4Simulator, &cFC4Director::CreateSimulator);
            FC4_LOGV("AddCls(0x%08X) registered", kFC4_CLSID_cSC4Simulator);
        }
    }

    uint32_t GetDirectorID() const override { return kFC4DirectorID; }

    bool InitializeCOM(cIGZCOM* pCOM, const cIGZString& sLibraryPath) override {
        const char* path = sLibraryPath.ToChar();
        const bool ok = cRZCOMDllDirector::InitializeCOM(pCOM, sLibraryPath);
        FC4_LOGV("InitializeCOM(com=%p, path='%s') -> %d, framework=%p", static_cast<void*>(pCOM), path ? path : "",
                 ok ? 1 : 0, static_cast<void*>(FrameWork()));
        return ok;
    }

    bool OnStart(cIGZCOM* pCOM) override {
        fc4::Log("FreeCity4 %s loaded (core ABI %u, %s mode); dll: %s; log: %s", FC4_VERSION_STRING, fc4_abi_version(),
                 passthrough_ ? "PASSTHROUGH (FC4_DISABLE_SIMULATOR=1, original simulator in use)" : "replacement",
                 fc4::ModulePath(), fc4::LogPath());
        cIGZFrameWork* fw = RZGetFrameWork();
        FC4_LOGV("OnStart(com=%p): framework=%p state=%d", static_cast<void*>(pCOM), static_cast<void*>(fw),
                 fw ? static_cast<int>(fw->GetState()) : -1);
        if (fw != nullptr) {
            if (fw->GetState() < cIGZFrameWork::kStatePreAppInit) {
                const bool hooked = fw->AddHook(static_cast<cIGZFrameWorkHooks*>(this));
                FC4_LOGV("OnStart: AddHook -> %d", hooked ? 1 : 0);
            } else {
                PostAppInit();
            }
        }
        return cRZCOMDllDirector::OnStart(pCOM);
    }

    bool GetClassObject(uint32_t rclsid, uint32_t riid, void** ppvObj) override {
        const uint32_t before = g_factoryCalls;
        const bool ok = cRZCOMDllDirector::GetClassObject(rclsid, riid, ppvObj);
        const bool ours = rclsid == kFC4_CLSID_cSC4Simulator;
        if (ours || otherClassLogs_ < kMaxOtherClassLogs) {
            if (!ours) ++otherClassLogs_;
            FC4_LOGV("GetClassObject(clsid=0x%08X, iid=0x%08X) -> %d, obj=%p, factory %s", rclsid, riid, ok ? 1 : 0,
                     (ok && ppvObj) ? *ppvObj : nullptr, g_factoryCalls != before ? "CALLED" : "not called");
        }
        return ok;
    }

    void EnumClassObjects(ClassObjectEnumerationCallback pCallback, void* pContext) override {
        FC4_LOGV("EnumClassObjects(callback=%p, context=%p) begin", reinterpret_cast<void*>(pCallback), pContext);
        EnumContext ctx{pCallback, pContext, 0};
        cRZCOMDllDirector::EnumClassObjects(&EnumThunk, &ctx);
        FC4_LOGV("EnumClassObjects end: %u class(es) reported", ctx.count);
    }

    bool CanUnloadNow() override {
        const bool r = cRZCOMDllDirector::CanUnloadNow();
        FC4_LOGV("CanUnloadNow -> %d (refcount %u)", r ? 1 : 0, RefCount());
        return r;
    }

    bool PreFrameWorkInit() override {
        FC4_LOGV("PreFrameWorkInit");
        return true;
    }

    bool PreAppInit() override {
        FC4_LOGV("PreAppInit");
        return true;
    }

    bool PostAppInit() override {
        fc4::Log("PostAppInit (%s)", passthrough_ ? "passthrough: simulator class not registered"
                                                  : "simulator class 0x2990C1E5 provided by FreeCity4");
        // Ask GZCOM itself whether it now resolves the simulator class, and
        // through which director. This creates (and then releases) one object.
        // Set FC4_NO_PROBE=1 to skip it.
        if (!passthrough_ && fc4::LogLevel() >= fc4::kLogVerbose && !ProbeDisabled()) {
            cIGZCOM* com = GZCOM();
            g_probing = true;
            const uint32_t before = g_factoryCalls;
            cIGZUnknown* probe = nullptr;
            const bool ok = com != nullptr &&
                            com->GetClassObject(kFC4_CLSID_cSC4Simulator, GZIID_cIGZUnknown,
                                                reinterpret_cast<void**>(&probe));
            g_probing = false;
            FC4_LOGV("PostAppInit probe: GZCOM()->GetClassObject(0x%08X, IUnknown) -> %d, obj=%p, our factory %s",
                     kFC4_CLSID_cSC4Simulator, ok ? 1 : 0, static_cast<void*>(probe),
                     g_factoryCalls != before ? "CALLED" : "NOT called (another factory answered)");
            if (ok && probe != nullptr) probe->Release();
        }
        return true;
    }

    bool PreAppShutdown() override {
        FC4_LOGV("PreAppShutdown");
        return true;
    }

    bool PostAppShutdown() override {
        fc4::Log("PostAppShutdown");
        return true;
    }

private:
    static cIGZUnknown* CreateSimulator() {
        ++g_factoryCalls;
        cFC4Simulator* sim = new cFC4Simulator();
        fc4::Log("cFC4Simulator created (%p), factory call #%u%s", static_cast<void*>(sim), g_factoryCalls,
                 g_probing ? " [PostAppInit diagnostic probe; released immediately]" : "");
        // The director QueryInterfaces the object, which takes the first reference.
        return static_cast<cIGZUnknown*>(static_cast<cISC4Simulator*>(sim));
    }

    bool passthrough_;
    uint32_t otherClassLogs_ = 0;
};

cRZCOMDllDirector* RZGetCOMDllDirector() {
    static cFC4Director sDirector;
    return &sDirector;
}
