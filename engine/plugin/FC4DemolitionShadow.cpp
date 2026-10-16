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

#include "FC4DemolitionShadow.h"

#include "FC4Config.h"
#include "FC4CoreShadow.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Streams.h"
#include "FC4WriteCapture.h"
#include "fc4.h"

#include "cISC4City.h"
#include "cISC4Demolition.h"
#include "cISC4Simulator.h"

#include <string>
#include <vector>

namespace fc4 {
namespace {

bool g_enabled = false;

fc4_demolition_host g_host = {};
fc4_demolition* g_core = nullptr;
WriteCapture g_capture;

// The tuning the constructor leaves (demolition.md §6's code defaults); the
// shipped exemplar overrides them at Init, so they are only checked while the
// core has not been Inited (which a shadow never is).
constexpr uint32_t kDefaultSmallMediumModel = 64;
constexpr uint32_t kDefaultMediumLargeModel = 128;
constexpr uint32_t kDefaultSmallMediumGroup = 20;
constexpr uint32_t kDefaultMediumLargeGroup = 100;
constexpr uint32_t kDefaultDestroyedTileCost = 3;

struct Tally {
    uint32_t lines = 0, clean = 0, compared = 0, mismatched = 0;
};
Tally g_tally;
std::string g_kinds;

fc4_demolition_host MakeReadOnlyHost() {
    fc4_demolition_host h{};
    h.struct_size = sizeof(h);
    return h;
}

struct Run {
    cISC4City* city = nullptr;
    int32_t julian = 0;
    uint32_t event = 0;  // 0 = load, 1 = month
    char date[16] = "-";
    intptr_t result = 0;
    uint32_t compared = 0;
};

std::vector<uint8_t> WriteOurs(const WriteCapture& cap, uint32_t* unknownRefs) {
    ShadowOStream out;
    for (uint32_t i = 1;; ++i) {
        const void* o = cap.Object(i);
        if (o == nullptr) break;
        out.AddToken(TokenOf(o), i, cap.ClassOf(i));
    }
    const bool ok = g_core != nullptr && fc4_demolition_write(g_core, out.Get());
    if (unknownRefs != nullptr) *unknownRefs = out.UnknownRefs();
    return ok ? out.Bytes() : std::vector<uint8_t>();
}

void RunStep(void* p) {
    Run& r = *static_cast<Run*>(p);
    cISC4Demolition* raw = r.city != nullptr ? r.city->GetDemolitionUtility() : nullptr;  // city slot 31
    cFC4ISC4Demolition* dm = raw != nullptr ? AsGame(raw) : nullptr;
    if (dm == nullptr) {
        Log("demolition shadow %s: the city has no demolition utility; nothing compared", r.date);
        r.result = -3;
        return;
    }

    CoreDiffs d;
    uint32_t compared = 0;

    // 1. the record round trip (§7): a u16 version and nothing else.
    bool answered = false;
    if (!g_capture.Capture(reinterpret_cast<cIGZUnknown*>(dm), &answered) || !answered) {
        Log("demolition shadow %s: the original's record (its own Write) is not readable; nothing compared", r.date);
        r.result = -3;
        return;
    }
    const std::vector<uint8_t>& orig = g_capture.Bytes();
    if (g_core == nullptr) {
        g_host = MakeReadOnlyHost();
        g_core = fc4_demolition_create(&g_host, 0);
    }
    bool readOk = false;
    if (g_core != nullptr) {
        CaptureIStream in(g_capture);
        readOk = fc4_demolition_read(g_core, in.Get());
    }
    uint32_t unknownRefs = 0;
    const std::vector<uint8_t> ours = WriteOurs(g_capture, &unknownRefs);
    const bool same = readOk && !ours.empty() && ours == orig;
    ++compared;
    if (!readOk) {
        d.Add("record read failed");
    } else if (!same) {
        d.Add("record %u vs %u bytes%s", static_cast<unsigned>(orig.size()), static_cast<unsigned>(ours.size()),
              unknownRefs != 0 ? ", refs not in the capture" : "");
    }

    // 2. our own state stays at its constructor defaults (the shadow never
    //    Inits the core, so the §6 tuning is still the code default).
    fc4_demolition_state st{};
    st.struct_size = sizeof(st);
    const bool haveState = g_core != nullptr && fc4_demolition_get_state(g_core, &st);
    ++compared;
    if (!haveState) {
        d.Add("get_state failed");
    } else {
        if (st.initialized != 0) d.Add("state initialized=%u (a shadow never Inits)", st.initialized);
        if (st.small_medium_model != kDefaultSmallMediumModel || st.medium_large_model != kDefaultMediumLargeModel ||
            st.small_medium_group != kDefaultSmallMediumGroup || st.medium_large_group != kDefaultMediumLargeGroup ||
            st.destroyed_tile_cost != kDefaultDestroyedTileCost) {
            d.Add("tuning %u/%u/%u/%u/%u", st.small_medium_model, st.medium_large_model, st.small_medium_group,
                  st.medium_large_group, st.destroyed_tile_cost);
        }
    }

    r.compared = compared;

    std::string body;
    AppendF(body, "record=%u bytes roundtrip=%s", static_cast<unsigned>(orig.size()), same ? "same" : "DIFFERENT");
    AppendF(body, "; compared=%u mismatches=%u first=%s", compared, d.n, d.Str());
    CoreShadowLine("demolition shadow (cont.)", "demolition", r.date, r.event != 0, body);

    ++g_tally.lines;
    if (d.n == 0) ++g_tally.clean;
    g_tally.compared += compared;
    g_tally.mismatched += d.n;
    if (d.n != 0) AppendF(g_kinds, "%s%u", g_kinds.empty() ? "" : ",", d.n);
    r.result = static_cast<intptr_t>(d.n);
}

intptr_t RunOnce(cISC4City* city, int32_t julian, uint32_t event) {
    Run r;
    r.city = city;
    r.julian = julian;
    r.event = event;
    if (julian > 0) {
        fc4_date dd{};
        fc4_date_from_julian(julian, &dd);
        _snprintf_s(r.date, sizeof(r.date), _TRUNCATE, "%04d-%02d-%02d", dd.year, dd.month, dd.day_of_month);
    }
    if (city == nullptr) return -1;
    const bool ok = SehGuard("demolition shadow", &RunStep, &r);
    if (!ok) {
        Log("demolition shadow %s: WARNING structured exception; comparison abandoned", r.date);
        return -3;
    }
    return r.result;
}

}  // namespace

void DemolitionShadowStartup(bool demolitionReplaced) {
    g_enabled = SettingInt("demolition_shadow", "FC4_DEMOLITION_SHADOW", 0) != 0;
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("demolition_shadow", "FC4_DEMOLITION_SHADOW", &src);
    if (demolitionReplaced) {
        Log("demolition shadow: requested (demolition_shadow from %s) but INACTIVE: it compares the ORIGINAL "
            "demolition utility, and `enable = demolition` replaces it",
            src);
        g_enabled = false;
        return;
    }
    Log("demolition shadow: ON (demolition_shadow from %s): at PostCityInit and each SimNewMonth, the original "
        "demolition utility's record (its own Write into memory) loaded into FreeCity4's core and written back, byte "
        "for byte, and the core's own state (fc4_demolition_get_state) against the §6 code defaults",
        src);
    Log("demolition shadow: NOT compared (the record is the only thing the original exposes read-only): the "
        "DemolishRegion / DemolishLots / DemolishOccupant prices and the demolished set (a dry run needs the "
        "demolition host's 84 occupant-query callbacks, not wired in this lane), ModifyTerrainHeight (the same plus "
        "the terrain queries), the two effect methods (presentation only) and the §6 tuning (module globals, no "
        "getter). The core is covered by its conformance vectors dm00-dm12.");
}

bool DemolitionShadowEnabled() { return g_enabled; }

void DemolitionShadowCityLoaded(cISC4City* city) {
    if (!g_enabled || city == nullptr) return;
    cFC4ISC4Simulator* sim = AsGame(city->GetSimulator());
    const int32_t julian = sim != nullptr ? sim->GetSimDateNumber() : 0;
    RunOnce(city, julian, 0);
}

void DemolitionShadowMonth(cISC4City* city, int32_t julian) {
    if (!g_enabled || city == nullptr) return;
    RunOnce(city, julian, 1);
}

void DemolitionShadowCityShutdown() {
    if (!g_enabled) return;
    Log("demolition shadow: city shutdown: %u lines, %u with mismatches=0; compared %u values, %u mismatched; "
        "differences: %s",
        g_tally.lines, g_tally.clean, g_tally.compared, g_tally.mismatched, g_kinds.empty() ? "none" : g_kinds.c_str());
    if (g_core != nullptr) {
        fc4_demolition_destroy(g_core);
        g_core = nullptr;
    }
    g_tally = Tally{};
    g_kinds.clear();
}

intptr_t DemolitionShadowTestRun(cISC4City* city, int32_t julian, uint32_t event) {
    return RunOnce(city, julian, event);
}

void DemolitionShadowTally(uint32_t* lines, uint32_t* clean, uint32_t* compared, uint32_t* mismatched) {
    if (lines != nullptr) *lines = g_tally.lines;
    if (clean != nullptr) *clean = g_tally.clean;
    if (compared != nullptr) *compared = g_tally.compared;
    if (mismatched != nullptr) *mismatched = g_tally.mismatched;
}

}  // namespace fc4

intptr_t DemolitionShadowTest(FC4DemolitionShadowTestInput* in) {
    if (in == nullptr || in->struct_size != sizeof(FC4DemolitionShadowTestInput)) return -1;
    if (in->event == 1) {
        fc4::DemolitionShadowCityShutdown();
        fc4::DemolitionShadowTally(&in->lines, &in->clean, &in->compared, &in->mismatched);
        return 0;
    }
    fc4::DemolitionShadowTally(&in->lines, &in->clean, &in->compared, &in->mismatched);
    return fc4::DemolitionShadowEnabled() ? 1 : 0;
}
