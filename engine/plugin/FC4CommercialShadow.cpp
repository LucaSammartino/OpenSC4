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

#include "FC4CommercialShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4HostGrid.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4Watchdog.h"
#include "cFC4Commercial.h"
#include "compat/FC4Compat.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4CommercialSimulator.h"
#include "cISC4SimGrid.h"
#include "cRZCOMDllDirector.h"

#include <cstdio>
#include <cstring>
#include <string>

namespace fc4 {
namespace {

constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimIdle = 0x6695681A;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr const char* kAgentName = "cSC4CommercialSimulator";  // §3
constexpr uint32_t kDeveloperCO2 = 0x3320;
constexpr uint32_t kDeveloperCO3 = 0x3330;
constexpr long kDefaultCells = 4;
constexpr long kMaxCells = 64;

// §2.1 offsets from the primary interface pointer (build 1.1.641).
constexpr size_t kOffDirty = 0x20;
constexpr size_t kOffState = 0x24;
constexpr size_t kOffCO2 = 0x34;
constexpr size_t kOffCO3 = 0x38;

using GridU8 = cISC4SimGrid<uint8_t>;

bool g_enabled = false;
bool g_simulatorReplaced = false;
bool g_commercialReplaced = false;
long g_cells = kDefaultCells;

const void* g_sim = nullptr;  // the original being followed
cIGZMessageTarget2* g_agent = nullptr;
const void* g_vtable = nullptr;
bool g_followed = false;
bool g_loggedOurs = false;

// The shadow core and its context.
const HostGridContext g_gridCtx{"commercial shadow", Sw::CommercialGridWrites};
CommercialContext g_ctx;
fc4_commercial_host g_host{};
fc4_commercial* g_core = nullptr;
const void* g_coreCity = nullptr;
bool g_coreFailed = false;

uint32_t g_rebuilds = 0;
uint32_t g_idleSteps = 0, g_idleMismatches = 0;
uint32_t g_lines = 0, g_cleanLines = 0;

void ReadSettings() {
    g_enabled = SettingInt("commercial_shadow", "FC4_COMMERCIAL_SHADOW", 0) != 0;
    long n = SettingInt("commercial_shadow_cells", "FC4_COMMERCIAL_SHADOW_CELLS", kDefaultCells);
    if (n <= 0) n = kDefaultCells;
    if (n > kMaxCells) n = kMaxCells;
    g_cells = n;
}

bool Active() { return g_enabled && g_simulatorReplaced && !g_commercialReplaced; }

cISC4City* CurrentCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    FC4_ACQUIRED(sc4, "QueryInterface");
    cISC4City* city = sc4->GetCity();
    FC4_RELEASE(sc4);
    return city;
}

cISC4CommercialSimulator* OriginalOf(cISC4City* city) {
    if (city == nullptr) return nullptr;
    cISC4CommercialSimulator* raw = city->GetCommercialSimulator();  // slot 54, borrowed
    if (raw == nullptr) return nullptr;
    if (IsOurObject(raw)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("commercial shadow: inactive: the city's commercial simulator is FreeCity4's own (the shadow compares "
                "against the ORIGINAL: do not enable commercial)");
        }
        return nullptr;
    }
    return raw;
}

// ---- raw reads of the original (§2.1) ---------------------------------------------------------------

struct RawRead {
    const void* obj = nullptr;
    const void* vtable = nullptr;
    bool vtableOk = false;
    uint8_t dirty = 0;
    int32_t state = -1;
    const void* co2 = nullptr;
    const void* co3 = nullptr;
};

template <class T>
T At(const void* obj, size_t off) {
    return *reinterpret_cast<const T*>(static_cast<const uint8_t*>(obj) + off);
}

void ReadRaw(void* ctx) {
    RawRead* r = static_cast<RawRead*>(ctx);
    r->vtableOk = r->vtable != nullptr && At<const void*>(r->obj, 0) == r->vtable;
    if (!r->vtableOk) return;
    r->dirty = At<uint8_t>(r->obj, kOffDirty);
    r->state = At<int32_t>(r->obj, kOffState);
    r->co2 = At<const void*>(r->obj, kOffCO2);
    r->co3 = At<const void*>(r->obj, kOffCO3);
}

bool ReadOriginal(const void* sim, RawRead* out) {
    *out = RawRead{};
    out->obj = sim;
    out->vtable = g_vtable;
    return SehGuard("commercial shadow (reading the original's state)", &ReadRaw, out) && out->vtableOk;
}

void Track(cISC4CommercialSimulator* sim) {
    g_sim = sim;
    g_followed = false;
    void* agent = nullptr;
    const bool qi = sim->QueryInterface(kIID_cIGZMessageTarget2, &agent) && agent != nullptr;
    g_agent = nullptr;
    if (qi) {
        cIGZUnknown* a = static_cast<cIGZUnknown*>(agent);
        FC4_ACQUIRED(a, "QueryInterface");
        g_agent = static_cast<cIGZMessageTarget2*>(agent);
        FC4_RELEASE(a);  // the simulator keeps it alive; only its identity is kept
    }
    g_vtable = compat::CommercialSimulatorVtable();
    const char* why = nullptr;
    RawRead r;
    if (g_agent == nullptr) {
        why = "it does not answer cIGZMessageTarget2 (its agent cannot be recognised)";
    } else if (g_vtable == nullptr) {
        why = "the original's vtable is not known for this executable (compat table)";
    } else if (!ReadOriginal(sim, &r)) {
        why = r.vtableOk ? "its state could not be read" : "its vtable is not the original's (compat table)";
    } else if (r.state < 0 || r.state > 3 || r.dirty > 1) {
        why = "+0x20 / +0x24 are not a dirty byte and a state 0..3";
    } else if (r.co2 != static_cast<const void*>(sim->GetProximityMap(
                           static_cast<cISC4BuildingDevelopmentSimulator::DeveloperType>(kDeveloperCO2))) ||
               r.co3 != static_cast<const void*>(sim->GetProximityMap(
                           static_cast<cISC4BuildingDevelopmentSimulator::DeveloperType>(kDeveloperCO3)))) {
        why = "+0x34 / +0x38 are not its proximity grids (slot 5)";
    }
    if (why != nullptr) {
        Log("commercial shadow: the original commercial simulator %p is NOT followed: %s; nothing compared",
            static_cast<const void*>(sim), why);
        return;
    }
    g_followed = true;
    Log("commercial shadow: following the original commercial simulator %p (agent %p, vtable %p): dirty=%u state=%d "
        "grids CO$$=%p CO$$$=%p",
        static_cast<const void*>(sim), static_cast<void*>(g_agent), g_vtable, r.dirty, static_cast<int>(r.state),
        r.co2, r.co3);
}

// ---- the shadow core ------------------------------------------------------------------------------

// §4.1 step 5 for the shadow: Init needs a demand simulator and both nodes;
// the shadow hands it dummy tokens (never read, §2.1) so that its grids are
// created.
bool ShadowHasDemand(void*) { return true; }
uint64_t ShadowDemandNode(void*, uint32_t id) { return 0x5D000000ull | id; }
void ShadowReleaseNode(void*, uint64_t) {}

void DropCore() {
    if (g_core != nullptr) {
        fc4_commercial_shutdown(g_core);  // releases its two grids
        fc4_commercial_destroy(g_core);
    }
    g_core = nullptr;
    g_coreCity = nullptr;
}

bool EnsureCore(cISC4City* city) {
    if (g_core != nullptr && g_coreCity == city) return true;
    DropCore();
    g_ctx = CommercialContext{};
    g_ctx.readOnly = true;
    g_ctx.city = city;  // borrowed: BDS slots 41 / 42 through the city's BDS
    g_host = MakeCommercialHost(&g_ctx, &g_gridCtx);
    g_host.has_demand_simulator = &ShadowHasDemand;
    g_host.demand_node = &ShadowDemandNode;
    g_host.release_demand_node = &ShadowReleaseNode;
    g_core = fc4_commercial_create(&g_host);
    bool ok = g_core != nullptr && fc4_commercial_init(g_core);
    if (ok) {
        ok = fc4_commercial_proximity_map(g_core, kDeveloperCO2) != 0 &&
             fc4_commercial_proximity_map(g_core, kDeveloperCO3) != 0;
    }
    if (!ok) {
        if (!g_coreFailed) {
            g_coreFailed = true;
            Log("commercial shadow: the shadow core could not be created or its grids made; nothing compared");
        }
        DropCore();
        return false;
    }
    g_coreCity = city;
    Log("commercial shadow: shadow core ready (city %p; its own CO$$ / CO$$$ grids)", static_cast<void*>(city));
    return true;
}

GridU8* CoreGrid(uint32_t type) { return reinterpret_cast<GridU8*>(FromToken(fc4_commercial_proximity_map(g_core, type))); }

GridU8* OrigGrid(cISC4CommercialSimulator* sim, uint32_t type) {
    return sim->GetProximityMap(static_cast<cISC4BuildingDevelopmentSimulator::DeveloperType>(type));  // slot 5
}

// Copies the original's grid into ours (same size required).
bool CopyGrid(GridU8* from, GridU8* to) {
    if (from == nullptr || to == nullptr) return false;
    const int32_t cx = from->GetTractCountX(), cz = from->GetTractCountZ();
    if (cx != to->GetTractCountX() || cz != to->GetTractCountZ()) return false;
    for (int32_t x = 0; x < cx; ++x) {
        for (int32_t z = 0; z < cz; ++z) to->SetTractValue(x, z, from->GetTractValue(x, z));
    }
    return true;
}

struct GridDiff {
    uint32_t cells = 0, edge1 = 0, zeros = 0, mismatches = 0;
    std::string listed;
};

GridDiff Compare(GridU8* orig, GridU8* ours) {
    GridDiff d;
    if (orig == nullptr || ours == nullptr) {
        d.mismatches = 1;
        d.listed = orig == nullptr ? " (no original grid)" : " (no shadow grid)";
        return d;
    }
    const int32_t cx = orig->GetTractCountX(), cz = orig->GetTractCountZ();
    if (cx != ours->GetTractCountX() || cz != ours->GetTractCountZ()) {
        d.mismatches = 1;
        d.listed = " (sizes differ)";
        return d;
    }
    long listed = 0;
    for (int32_t x = 0; x < cx; ++x) {
        for (int32_t z = 0; z < cz; ++z) {
            const uint8_t a = orig->GetTractValue(x, z), b = ours->GetTractValue(x, z);
            ++d.cells;
            if (a == 0) ++d.zeros;
            if (z == 0 && a == 1) ++d.edge1;
            if (a != b) {
                ++d.mismatches;
                if (listed < g_cells) {
                    char t[48];
                    snprintf(t, sizeof(t), " (%d,%d) %u/%u", x, z, a, b);
                    d.listed += t;
                    ++listed;
                }
            }
        }
    }
    return d;
}

std::string DateText(int32_t julian) {
    if (julian <= 0) return "0000-00-00";
    fc4_date d{};
    fc4_date_from_julian(julian, &d);
    char b[24];
    snprintf(b, sizeof(b), "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
    return b;
}

struct Step {
    cISC4CommercialSimulator* sim = nullptr;
    cIGZMessageTarget2* target = nullptr;
    cIGZMessage2* msg = nullptr;
    RawRead before;
    bool copied = false;
};

// Before the original's step: the original's grids into ours (a rebuild
// step only).
void CallCopy(void* ctx) {
    Step* s = static_cast<Step*>(ctx);
    s->copied = CopyGrid(OrigGrid(s->sim, kDeveloperCO2), CoreGrid(kDeveloperCO2)) &&
                CopyGrid(OrigGrid(s->sim, kDeveloperCO3), CoreGrid(kDeveloperCO3));
}

// After it: our step from the original's before-state, then the comparison.
struct After {
    Step* step = nullptr;
    RawRead after;
    int32_t julian = 0;
    fc4_commercial_state ours{};
};

void CallOurStep(void* ctx) {
    After* a = static_cast<After*>(ctx);
    fc4_commercial_set_machine(g_core, a->step->before.dirty != 0, a->step->before.state);
    fc4_commercial_do_idle(g_core);
    a->ours.struct_size = sizeof(a->ours);
    fc4_commercial_get_state(g_core, &a->ours);
}

void CallCompare(void* ctx) {
    After* a = static_cast<After*>(ctx);
    const Step* s = a->step;
    const int32_t from = s->before.state;
    // State 0 continues with state 1 in the same call (§4.3): CO$$; state 2:
    // CO$$$.
    const bool co3 = from == 2;
    const uint32_t type = co3 ? kDeveloperCO3 : kDeveloperCO2;
    GridDiff d = Compare(OrigGrid(s->sim, type), CoreGrid(type));
    // The other grid must be unchanged on both sides (it was copied).
    GridDiff other = Compare(OrigGrid(s->sim, co3 ? kDeveloperCO2 : kDeveloperCO3),
                             CoreGrid(co3 ? kDeveloperCO2 : kDeveloperCO3));
    const bool machineOk = a->after.state == a->ours.state && (a->after.dirty != 0) == (a->ours.dirty != 0);
    const uint32_t mismatches = d.mismatches + other.mismatches + (machineOk ? 0 : 1) + (s->copied ? 0 : 1);
    ++g_rebuilds;
    ++g_lines;
    if (mismatches == 0 && g_idleMismatches == 0) ++g_cleanLines;
    Log("commercial shadow %s rebuild %u %s (state %d->%d orig, %d->%d ours; dirty %u/%u): cells=%u edge1=%u "
        "zeros=%u; other grid unchanged mismatches=%u; idle steps since the last line=%u (machine mismatches=%u); "
        "mismatches=%u%s%s%s",
        DateText(a->julian).c_str(), g_rebuilds, co3 ? "CO$$$" : "CO$$", static_cast<int>(from),
        static_cast<int>(a->after.state), static_cast<int>(from), static_cast<int>(a->ours.state), a->after.dirty,
        a->ours.dirty, d.cells, d.edge1, d.zeros, other.mismatches, g_idleSteps, g_idleMismatches,
        mismatches + g_idleMismatches, d.listed.empty() ? "" : "; first differing (x,z) orig/ours:", d.listed.c_str(),
        s->copied ? "" : "; the original's grids could not be copied before the step");
    g_idleSteps = 0;
    g_idleMismatches = 0;
}

bool Deliver(cISC4City* city, cISC4CommercialSimulator* sim, cIGZMessageTarget2* target, cIGZMessage2* msg,
             uint32_t type, int64_t data1, bool* finished) {
    Step s;
    s.sim = sim;
    s.target = target;
    s.msg = msg;
    if (!ReadOriginal(sim, &s.before)) {
        g_followed = false;
        Log("commercial shadow: the original commercial simulator %p can no longer be read; no longer followed",
            static_cast<const void*>(sim));
        return false;
    }
    const bool core = EnsureCore(city);
    const bool rebuild = s.before.state >= 0 && s.before.state <= 2;
    if (core && rebuild) SehGuard("commercial shadow (grid copy)", &CallCopy, &s);
    *finished = target->DoMessage(msg);
    if (!core) return true;
    After a;
    a.step = &s;
    a.julian = type == kMsgSimNewDay ? static_cast<int32_t>(data1) : static_cast<int32_t>(PublishedJulian());
    if (!ReadOriginal(sim, &a.after)) {
        g_followed = false;
        Log("commercial shadow: the original commercial simulator %p could not be read after a step; no longer "
            "followed",
            static_cast<const void*>(sim));
        return true;
    }
    if (!SehGuard("commercial shadow (our step)", &CallOurStep, &a)) return true;
    if (rebuild) {
        SehGuard("commercial shadow (comparison)", &CallCompare, &a);
    } else {
        ++g_idleSteps;
        if (a.after.state != a.ours.state || (a.after.dirty != 0) != (a.ours.dirty != 0)) {
            ++g_idleMismatches;
            Log("commercial shadow %s: idle step differs: orig dirty %u->%u state %d->%d, ours dirty ->%u state ->%d",
                DateText(a.julian).c_str(), s.before.dirty, a.after.dirty, static_cast<int>(s.before.state),
                static_cast<int>(a.after.state), a.ours.dirty, static_cast<int>(a.ours.state));
        }
    }
    return true;
}

}  // namespace

void CommercialShadowStartup(bool simulatorReplaced, bool commercialReplaced) {
    g_simulatorReplaced = simulatorReplaced;
    g_commercialReplaced = commercialReplaced;
    ReadSettings();
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("commercial_shadow", "FC4_COMMERCIAL_SHADOW", &src);
    if (commercialReplaced) {
        Log("commercial shadow: requested (commercial_shadow from %s) but INACTIVE: it compares against the ORIGINAL "
            "commercial simulator, and commercial is enabled (ours)",
            src);
    } else if (!simulatorReplaced) {
        Log("commercial shadow: requested (commercial_shadow from %s) but INACTIVE: it follows the original's "
            "deliveries, which pass through FreeCity4's simulator, and simulator is not enabled",
            src);
    } else {
        Log("commercial shadow: ON (commercial_shadow from %s): before each SimIdle / SimNewDay step of the ORIGINAL "
            "commercial simulator its dirty byte and state (+0x20 / +0x24, read-only, vtable checked) are read; "
            "before a rebuild step (state 0..2) its CO$$ / CO$$$ grids are copied into the shadow core's own; after "
            "the step ours runs the same step on the live building map and the machine and both grids are compared "
            "cell by cell (commercial-industrial.md §4.3, §4.4); one line per map rebuild; differing cells listed "
            "per grid: %ld. Nothing is written to the game",
            src, g_cells);
    }
}

void CommercialShadowCityShutdown() {
    if (g_core == nullptr && g_sim == nullptr) return;
    if (g_lines > 0 || g_core != nullptr) {
        Log("commercial shadow: city shutdown: %u rebuild lines, %u with mismatches=0", g_lines, g_cleanLines);
    }
    DropCore();
    g_sim = nullptr;
    g_agent = nullptr;
    g_followed = false;
}

bool CommercialShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                             int64_t data1, bool* finished) {
    if ((type != kMsgSimIdle && type != kMsgSimNewDay) || target == nullptr || msg == nullptr ||
        finished == nullptr || !Active()) {
        return false;
    }
    // Cheap rejection for every other agent.
    if (target != g_agent && (agentName == nullptr || strcmp(agentName, kAgentName) != 0)) return false;
    cISC4City* city = CurrentCity();
    cISC4CommercialSimulator* sim = OriginalOf(city);
    if (sim == nullptr) return false;
    if (sim != g_sim) Track(sim);
    if (!g_followed || target != g_agent) return false;
    return Deliver(city, sim, target, msg, type, data1, finished);
}

}  // namespace fc4
