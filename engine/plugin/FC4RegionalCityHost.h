// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "FC4RefAudit.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cISC4App.h"
#include "cRZCOMDllDirector.h"

namespace fc4::regionalcity {
struct App {
    cISC4App* p = nullptr;
    App() {
        auto* fw = RZGetFrameWork();
        auto* a = fw ? fw->Application() : nullptr;
        if (a && a->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&p)) && p)
            FC4_ACQUIRED(p, "regionalcity application");
    }
    ~App() { if (p) FC4_RELEASE(p); }
    App(const App&) = delete;
    App& operator=(const App&) = delete;
};
}
