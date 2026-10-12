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

#include "FC4Subsystems.h"

#include "FC4Autotest.h"
#include "FC4Checked.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cctype>
#include <cstring>
#include <string>

// Factories (defined next to each class).
cIGZUnknown* CreateFC4Simulator();
cIGZUnknown* CreateFC4BudgetSimulator();
cIGZUnknown* CreateFC4DepartmentBudget();
cIGZUnknown* CreateFC4LineItem();
cIGZUnknown* CreateFC4LandValueSimulator();

namespace fc4 {
namespace {

// ---- the table ------------------------------------------------------------------------------------

const ClassEntry kSimulatorClasses[] = {
    {0x2990C1E5, "cSC4Simulator", &CreateFC4Simulator},
};

// docs/specs/budget.md §1: the budget simulator, departments and line items.
const ClassEntry kBudgetClasses[] = {
    {0xE990BE01, "cSC4BudgetSimulator", &CreateFC4BudgetSimulator},
    {0xE990BFFC, "cSC4DepartmentBudget", &CreateFC4DepartmentBudget},
    {0xAA313C9F, "cSC4LineItem", &CreateFC4LineItem},
};

// docs/specs/landvalue.md §1. Off by default (M3 trial): `enable = landvalue`.
// The game's own grid classes (0x49B9E602..0x49B9E60B) are never registered
// (simgrid.md §6.3).
const ClassEntry kLandValueClasses[] = {
    {0xE990C18E, "cSC4LandValueSimulator", &CreateFC4LandValueSimulator},
};

const Subsystem kSubsystems[] = {
    {"simulator", "FC4_DISABLE_SIMULATOR", kSimulatorClasses, sizeof(kSimulatorClasses) / sizeof(kSimulatorClasses[0]),
     true},
    {"budget", nullptr, kBudgetClasses, sizeof(kBudgetClasses) / sizeof(kBudgetClasses[0]), true},
    {"landvalue", nullptr, kLandValueClasses, sizeof(kLandValueClasses) / sizeof(kLandValueClasses[0]), false},
};

constexpr size_t kSubsystemCount = sizeof(kSubsystems) / sizeof(kSubsystems[0]);

// ---- switches --------------------------------------------------------------------------------------

std::string Env(const char* name) {
    char value[512] = {};
    const DWORD n = GetEnvironmentVariableA(name, value, sizeof(value));
    return (n > 0 && n < sizeof(value)) ? std::string(value, n) : std::string();
}

std::string Lower(std::string s) {
    for (char& c : s) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    return s;
}

bool ListContains(const std::string& list, const std::string& key) {
    size_t i = 0;
    while (i < list.size()) {
        while (i < list.size() && (list[i] == ',' || list[i] == ' ' || list[i] == ';')) ++i;
        size_t j = i;
        while (j < list.size() && list[j] != ',' && list[j] != ' ' && list[j] != ';') ++j;
        if (j > i) {
            const std::string item = list.substr(i, j - i);
            if (item == key || item == "all") return true;
        }
        i = j;
    }
    return false;
}

bool g_resolved = false;
bool g_enabled[kSubsystemCount] = {};
char g_disabledList[128] = "-";

void Resolve() {
    if (g_resolved) return;
    g_resolved = true;
    // First call outside DllMain: configuration, audit settings, checked-build self-tests.
    ConfigReload("game start");
    RefAuditReload();
    CheckedStartup();
    AutotestReload();
    // FreeCity4.ini `disable = ...` wins over FC4_DISABLE.
    const char* source = "";
    const std::string disable = Lower(Setting("disable", "FC4_DISABLE", &source));
    const char* enableSource = "";
    const std::string enable = Lower(Setting("enable", "FC4_ENABLE", &enableSource));
    for (size_t i = 0; i < kSubsystemCount; ++i) {
        const Subsystem& s = kSubsystems[i];
        if (!s.defaultOn && !ListContains(enable, s.key)) {
            // Off by default and not enabled: not "disabled" (the lists and the
            // autotest verdict stay as without this subsystem).
            g_enabled[i] = false;
            Log("subsystem '%s': off by default (enable = %s in FreeCity4.ini to use it)", s.key, s.key);
            continue;
        }
        bool off = ListContains(disable, s.key);
        const char* why = off ? (strcmp(source, "FreeCity4.ini") == 0 ? "FreeCity4.ini (disable=)" : "FC4_DISABLE") : "";
        if (!off && s.legacyDisableEnv != nullptr && Env(s.legacyDisableEnv) == "1") {
            off = true;
            why = s.legacyDisableEnv;
        }
        g_enabled[i] = !off;
        if (off) {
            if (strcmp(g_disabledList, "-") == 0) g_disabledList[0] = 0;
            if (g_disabledList[0] != 0) strncat_s(g_disabledList, ",", _TRUNCATE);
            strncat_s(g_disabledList, s.key, _TRUNCATE);
        }
        if (off) {
            Log("subsystem '%s': DISABLED by %s; the original classes are used", s.key, why);
        } else {
            Log("subsystem '%s': enabled (%u class%s replaced)%s", s.key, static_cast<unsigned>(s.classCount),
                s.classCount == 1 ? "" : "es",
                s.defaultOn ? ""
                            : (strcmp(enableSource, "FreeCity4.ini") == 0 ? " by FreeCity4.ini (enable=)"
                                                                         : " by FC4_ENABLE"));
        }
    }
}

}  // namespace

const char* DisabledSubsystemsList() { return g_disabledList; }

bool SimulatorReplaced() {
    const Subsystem* owner = nullptr;
    return FindEnabledClass(0x2990C1E5, &owner) != nullptr;
}

const Subsystem* Subsystems(size_t* count) {
    if (count != nullptr) *count = kSubsystemCount;
    return kSubsystems;
}

bool SubsystemEnabled(const Subsystem& s) {
    Resolve();
    for (size_t i = 0; i < kSubsystemCount; ++i) {
        if (&kSubsystems[i] == &s) return g_enabled[i];
    }
    return false;
}

const ClassEntry* FindEnabledClass(uint32_t clsid, const Subsystem** owner) {
    Resolve();
    for (size_t i = 0; i < kSubsystemCount; ++i) {
        if (!g_enabled[i]) continue;
        const Subsystem& s = kSubsystems[i];
        for (size_t k = 0; k < s.classCount; ++k) {
            if (s.classes[k].clsid == clsid) {
                if (owner != nullptr) *owner = &s;
                return &s.classes[k];
            }
        }
    }
    return nullptr;
}

}  // namespace fc4
