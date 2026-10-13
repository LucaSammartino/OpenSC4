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
#include "FC4BudgetShadow.h"
#include "FC4DemandShadow.h"
#include "FC4Log.h"
#include "FC4Observer.h"
#include "FC4Popups.h"
#include "FC4RefAudit.h"
#include "cFC4Demand.h"
#include "cFC4Ordinance.h"
#include "cFC4Power.h"
#include "cFC4Plumbing.h"
#include "cFC4Flammability.h"
#include "FC4PollutionShadow.h"
#include "cFC4Pollution.h"

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
cIGZUnknown* CreateFC4WeatherSimulator();

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

// docs/specs/weather.md §1. Off by default (M2 glue trial): `enable = weather`.
const ClassEntry kWeatherClasses[] = {
    {0x2990C1BC, "cSC4WeatherSimulator", &CreateFC4WeatherSimulator},
};

// docs/specs/ordinance.md §1, §3.6. Off by default (M2 glue trial):
// `enable = ordinance`. The simulator and the ordinance classes go together
// (§10.2).
const ClassEntry kOrdinanceClasses[] = {
    {0xA990C08E, "cSC4OrdinanceSimulator", &CreateFC4OrdinanceSimulator},
    {0x8BD4FC74, "cSC4OrdinanceSimple", &CreateFC4Ordinance<0x8BD4FC74>},
    {0xA2BF1DDC, "cSC4OrdinanceCarpoolIncentive", &CreateFC4Ordinance<0xA2BF1DDC>},
    {0xA2BF1E43, "cSC4OrdinanceCleanAir", &CreateFC4Ordinance<0xA2BF1E43>},
    {0x00D0723D, "cSC4OrdinanceCommunityCPRTraining", &CreateFC4Ordinance<0x00D0723D>},
    {0x40D07236, "cSC4OrdinanceFreeClinics", &CreateFC4Ordinance<0x40D07236>},
    {0x815B4CEF, "cSC4OrdinanceJuniorSportsLeague", &CreateFC4Ordinance<0x815B4CEF>},
    {0x22F6E80C, "cSC4OrdinanceLandfillGasRecovery", &CreateFC4Ordinance<0x22F6E80C>},
    {0xA0D07129, "cSC4OrdinanceLegalizeGambling", &CreateFC4Ordinance<0xA0D07129>},
    {0x62BF1DAA, "cSC4OrdinanceMandatorySmokeDetectors", &CreateFC4Ordinance<0x62BF1DAA>},
    {0x62BF1DB9, "cSC4OrdinanceNeighborhoodWatch", &CreateFC4Ordinance<0x62BF1DB9>},
    {0xE0D0722E, "cSC4OrdinanceNuclearFreeZone", &CreateFC4Ordinance<0xE0D0722E>},
    {0x22F6E81B, "cSC4OrdinancePaperReductionAct", &CreateFC4Ordinance<0x22F6E81B>},
    {0x82B9999B, "cSC4OrdinancePowerConservation", &CreateFC4Ordinance<0x82B9999B>},
    {0xE0D07233, "cSC4OrdinanceProReadingCampaign", &CreateFC4Ordinance<0xE0D07233>},
    {0xA2BF1DE5, "cSC4OrdinanceShuttleService", &CreateFC4Ordinance<0xA2BF1DE5>},
    {0xC2F6E81F, "cSC4OrdinanceTireRecycling", &CreateFC4Ordinance<0xC2F6E81F>},
    {0x62F6E7CF, "cSC4OrdinanceTouristPromotion", &CreateFC4Ordinance<0x62F6E7CF>},
    {0x42BF1E18, "cSC4OrdinanceTrashPresort", &CreateFC4Ordinance<0x42BF1E18>},
    {0xC2BF1E04, "cSC4OrdinanceVehicleEmissionStandard", &CreateFC4Ordinance<0xC2BF1E04>},
    {0x02BF1DFA, "cSC4OrdinanceWaterConservation", &CreateFC4Ordinance<0x02BF1DFA>},
    {0xC2BF1DC5, "cSC4OrdinanceYouthCurfew", &CreateFC4Ordinance<0xC2BF1DC5>},
};

// docs/specs/demand.md §1. Off by default (M2 glue trial): `enable = demand`.
// The node class is registered because saves store nodes by class (§1).
const ClassEntry kDemandClasses[] = {
    {0x2990BFF2, "cSC4DemandSimulator", &CreateFC4DemandSimulator},
    {0x8990C12D, "cSC4Demand", &CreateFC4Demand},
};

// docs/specs/power.md §1. Off by default (M3 glue trial): `enable = power`.
const ClassEntry kPowerClasses[] = {
    {0x0990C005, "cSC4PowerSimulator", &CreateFC4PowerSimulator},
};

// docs/specs/plumbing.md §1. Off by default (M3 glue trial): `enable = plumbing`.
const ClassEntry kPlumbingClasses[] = {
    {0x0990C075, "cSC4PlumbingSimulator", &CreateFC4PlumbingSimulator},
};

// docs/specs/flammability.md §1. Off by default (M3 glue trial):
// `enable = flammability`.
const ClassEntry kFlammabilityClasses[] = {
    {0xE990C045, "cSC4FlammabilitySimulator", &CreateFC4FlammabilitySimulator},
};

// docs/specs/pollution.md §1. Off by default (M3 glue trial): `enable = pollution`.
const ClassEntry kPollutionClasses[] = {
    {0x8990C065, "cSC4PollutionSimulator", &CreateFC4PollutionSimulator},
};

const Subsystem kSubsystems[] = {
    {"simulator", "FC4_DISABLE_SIMULATOR", kSimulatorClasses, sizeof(kSimulatorClasses) / sizeof(kSimulatorClasses[0]),
     true},
    {"budget", nullptr, kBudgetClasses, sizeof(kBudgetClasses) / sizeof(kBudgetClasses[0]), true},
    {"landvalue", nullptr, kLandValueClasses, sizeof(kLandValueClasses) / sizeof(kLandValueClasses[0]), false},
    {"weather", nullptr, kWeatherClasses, sizeof(kWeatherClasses) / sizeof(kWeatherClasses[0]), false},
    {"ordinance", nullptr, kOrdinanceClasses, sizeof(kOrdinanceClasses) / sizeof(kOrdinanceClasses[0]), false},
    {"demand", nullptr, kDemandClasses, sizeof(kDemandClasses) / sizeof(kDemandClasses[0]), false},
    {"power", nullptr, kPowerClasses, sizeof(kPowerClasses) / sizeof(kPowerClasses[0]), false},
    {"plumbing", nullptr, kPlumbingClasses, sizeof(kPlumbingClasses) / sizeof(kPlumbingClasses[0]), false},
    {"flammability", nullptr, kFlammabilityClasses, sizeof(kFlammabilityClasses) / sizeof(kFlammabilityClasses[0]),
     false},
    {"pollution", nullptr, kPollutionClasses, sizeof(kPollutionClasses) / sizeof(kPollutionClasses[0]), false},
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
    ObserverReload();
    PopupDialogsReload();  // popup_dialogs (FC4Popups.h); the call itself is made at PostAppInit
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
    // kSubsystems[0] is the simulator, [1] the budget, [5] demand.
    BudgetShadowStartup(g_enabled[0], g_enabled[1]);
    DemandShadowStartup(g_enabled[0], g_enabled[5]);
    // The pollution shadow (FC4PollutionShadow.h), by key.
    for (size_t i = 0; i < kSubsystemCount; ++i) {
        if (strcmp(kSubsystems[i].key, "pollution") == 0) PollutionShadowStartup(g_enabled[0], g_enabled[i]);
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
