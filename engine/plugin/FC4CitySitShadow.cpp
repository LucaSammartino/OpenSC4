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

#include "FC4CitySitShadow.h"
#include "FC4CitySitRecord.h"

#include "FC4Config.h"
#include "FC4CoreShadow.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Streams.h"
#include "FC4WriteCapture.h"
#include "fc4.h"

#include "cISC4City.h"
#include "cISC4CitySituationManager.h"
#include "cISC4Simulator.h"

#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

bool g_enabled = false;

fc4_citysit_host g_host = {};
fc4_citysit* g_core = nullptr;
WriteCapture g_capture;

struct Tally {
    uint32_t lines = 0, clean = 0, compared = 0, mismatched = 0;
};
Tally g_tally;
std::string g_kinds;

// The record (§7): u16 version, u32 count, count x {u32 guid; u16 count},
// u8 indicatorsVisible. The GUIDs of a captured record, in record order.
void GuidsOf(const std::vector<uint8_t>& b, std::vector<uint32_t>& guids) {
    guids.clear();
    CaptureReader r(b);
    uint16_t version = 0;
    uint32_t count = 0;
    if (!r.U16(version) || !r.U32(count) || count > 4096) return;
    for (uint32_t i = 0; i < count; ++i) {
        uint32_t guid = 0;
        uint16_t n = 0;
        if (!r.U32(guid) || !r.U16(n)) return;
        guids.push_back(guid);
    }
}

fc4_citysit_host MakeReadOnlyHost() {
    fc4_citysit_host h{};
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
    const bool ok = g_core != nullptr && fc4_citysit_write(g_core, out.Get());
    if (unknownRefs != nullptr) *unknownRefs = out.UnknownRefs();
    return ok ? out.Bytes() : std::vector<uint8_t>();
}

// A float compared bit for bit (the times come back in ST0).
bool SameF32(float a, float b) {
    uint32_t x = 0, y = 0;
    memcpy(&x, &a, 4);
    memcpy(&y, &b, 4);
    return x == y;
}

void RunStep(void* p) {
    Run& r = *static_cast<Run*>(p);
    cISC4CitySituationManager* raw =
        r.city != nullptr ? r.city->GetCitySituationManager() : nullptr;  // city slot 49
    cFC4ISC4CitySituationManager* cm = raw != nullptr ? AsGame(raw) : nullptr;
    if (cm == nullptr) {
        Log("citysit shadow %s: the city has no city situation manager; nothing compared", r.date);
        r.result = -3;
        return;
    }

    CoreDiffs d;
    uint32_t compared = 0;

    bool answered = false;
    if (!g_capture.Capture(reinterpret_cast<cIGZUnknown*>(cm), &answered) || !answered) {
        Log("citysit shadow %s: the original's record (its own Write) is not readable; nothing compared", r.date);
        r.result = -3;
        return;
    }
    const std::vector<uint8_t>& orig = g_capture.Bytes();
    if (g_core == nullptr) {
        g_host = MakeReadOnlyHost();
        g_core = fc4_citysit_create(&g_host, 0, 0);
    }
    bool readOk = false;
    if (g_core != nullptr) {
        CaptureIStream in(g_capture);
        readOk = fc4_citysit_read(g_core, in.Get());
    }
    uint32_t unknownRefs = 0;
    const std::vector<uint8_t> ours = WriteOurs(g_capture, &unknownRefs);
    const bool same = readOk && CitySitRecordEqual(ours, orig);
    if (same && ours != orig)
        Log("citysit record: all fields equal; success-map iteration order differs (%u entries)",
            static_cast<unsigned>((orig.size() - 7) / 6));
    ++compared;
    if (!readOk) {
        d.Add("record read failed");
    } else if (!same) {
        d.Add("record %u vs %u bytes%s", static_cast<unsigned>(orig.size()), static_cast<unsigned>(ours.size()),
              unknownRefs != 0 ? ", refs not in the capture" : "");
    }

    // the success counts the record carries (slot 5 against the core's)
    std::vector<uint32_t> guids;
    GuidsOf(orig, guids);
    for (uint32_t guid : guids) {
        const uint16_t oc = cm->GetMissionSuccessCount(guid);
        const uint16_t sc = g_core != nullptr ? fc4_citysit_get_success_count(g_core, guid) : 0;
        ++compared;
        if (oc != sc) d.Add("successCount(0x%08x) orig=%u ours=%u", guid, oc, sc);
    }

    // the indicator flag (slot 30) and the status block (slots 6-12, 27, 28, 32)
    {
        const bool ov = cm->GetCSIVisible();
        const bool sv = g_core != nullptr && fc4_citysit_csi_visible(g_core);
        ++compared;
        if (ov != sv) d.Add("csiVisible orig=%d ours=%d", ov ? 1 : 0, sv ? 1 : 0);
    }
    {
        const uint32_t os = cm->GetCurrentStatus();
        const uint32_t ss = g_core != nullptr ? fc4_citysit_current_status(g_core) : 0;
        ++compared;
        if (os != ss) d.Add("status orig=%u ours=%u", os, ss);
    }
    {
        const void* oa = cm->GetActiveSituation();
        const uint64_t sa = g_core != nullptr ? fc4_citysit_active_situation(g_core) : 0;
        ++compared;
        if (TokenOf(oa) != sa) d.Add("activeSituation orig=0x%llx ours=0x%llx",
                                     static_cast<unsigned long long>(TokenOf(oa)), static_cast<unsigned long long>(sa));
    }
    {
        const void* oa = cm->GetActiveAuto();
        const uint64_t sa = g_core != nullptr ? fc4_citysit_active_auto(g_core) : 0;
        ++compared;
        if (TokenOf(oa) != sa) {
            d.Add("activeAuto orig=0x%llx ours=0x%llx", static_cast<unsigned long long>(TokenOf(oa)),
                  static_cast<unsigned long long>(sa));
        }
    }
    {
        const void* oa = cm->GetCurrentTarget();
        const uint64_t sa = g_core != nullptr ? fc4_citysit_current_target(g_core) : 0;
        ++compared;
        if (TokenOf(oa) != sa) {
            d.Add("currentTarget orig=0x%llx ours=0x%llx", static_cast<unsigned long long>(TokenOf(oa)),
                  static_cast<unsigned long long>(sa));
        }
    }
    {
        const uint32_t oi = cm->GetCurrentTargetIndex(), on = cm->GetNumTargets(), orr = cm->GetNumTargetsRemaining();
        const uint32_t si = g_core != nullptr ? fc4_citysit_current_target_index(g_core) : 0;
        const uint32_t sn = g_core != nullptr ? fc4_citysit_num_targets(g_core) : 0;
        const uint32_t sr = g_core != nullptr ? fc4_citysit_num_targets_remaining(g_core) : 0;
        compared += 3;
        if (oi != si) d.Add("targetIndex orig=%u ours=%u", oi, si);
        if (on != sn) d.Add("numTargets orig=%u ours=%u", on, sn);
        if (orr != sr) d.Add("targetsRemaining orig=%u ours=%u", orr, sr);
    }
    // NOT GetTrackedAutomataCount (slot 28) or GetMaxCSI (slot 27): the first
    // is the size of the indicator tracking (§4.2), which is built from the
    // AdviceAdded / AdviceRemoved messages this shadow does not follow; the
    // second is a process-wide static the automata manager loads from its own
    // tuning (§2 "a static byte (0x00B21D34), not a member"), not this
    // manager's state. Both are named in the game-start line.
    {
        const uint32_t ok = cm->CountdownIsInProgress();
        const bool sk = g_core != nullptr && fc4_citysit_countdown_in_progress(g_core);
        ++compared;
        if ((ok != 0) != sk) d.Add("countdown orig=%u ours=%d", ok, sk ? 1 : 0);
    }

    // the three float times (slots 13-15, ST0), bit for bit
    {
        const float a = cm->GetCurrentTimeLimitSeconds();
        const float b = g_core != nullptr ? fc4_citysit_time_limit_seconds(g_core) : 0.0f;
        const float c = cm->GetCurrentSituationTime();
        const float e = g_core != nullptr ? fc4_citysit_situation_time(g_core) : 0.0f;
        const float f = cm->GetCurrentTimeRemainingSeconds();
        const float h = g_core != nullptr ? fc4_citysit_time_remaining_seconds(g_core) : 0.0f;
        compared += 3;
        if (!SameF32(a, b)) d.Add("timeLimit orig=%g ours=%g", a, b);
        if (!SameF32(c, e)) d.Add("situationTime orig=%g ours=%g", c, e);
        if (!SameF32(f, h)) d.Add("timeRemaining orig=%g ours=%g", f, h);
    }

    // the situation's aura and money (slots 17, 18): both false with no mission
    {
        float orad = 0, omag = 0, srad = 0, smag = 0;
        const bool oa = cm->GetSituationAura(orad, omag);
        const bool sa = g_core != nullptr && fc4_citysit_situation_aura(g_core, &srad, &smag);
        ++compared;
        if (oa != sa || (oa && (!SameF32(orad, srad) || !SameF32(omag, smag)))) {
            d.Add("aura orig=%d/%g/%g ours=%d/%g/%g", oa ? 1 : 0, orad, omag, sa ? 1 : 0, srad, smag);
        }
        int64_t om = 0, sm = 0;
        const bool omo = cm->GetSituationMoney(om);
        const bool smo = g_core != nullptr && fc4_citysit_situation_money(g_core, &sm);
        ++compared;
        if (omo != smo || om != sm) d.Add("money orig=%d/%lld ours=%d/%lld", omo ? 1 : 0, static_cast<long long>(om),
                                          smo ? 1 : 0, static_cast<long long>(sm));
    }

    r.compared = compared;

    std::string body;
    AppendF(body, "record=%u bytes roundtrip=%s", static_cast<unsigned>(orig.size()), same ? "same" : "DIFFERENT");
    AppendF(body, "; compared=%u mismatches=%u first=%s", compared, d.n, d.Str());
    CoreShadowLine("citysit shadow (cont.)", "citysit", r.date, r.event != 0, body);

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
    const bool ok = SehGuard("citysit shadow", &RunStep, &r);
    if (!ok) {
        Log("citysit shadow %s: WARNING structured exception; comparison abandoned", r.date);
        return -3;
    }
    return r.result;
}

}  // namespace

void CitySitShadowStartup(bool citysitReplaced) {
    g_enabled = SettingInt("citysit_shadow", "FC4_CITYSIT_SHADOW", 0) != 0;
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("citysit_shadow", "FC4_CITYSIT_SHADOW", &src);
    if (citysitReplaced) {
        Log("citysit shadow: requested (citysit_shadow from %s) but INACTIVE: it compares the ORIGINAL city "
            "situation manager, and `enable = citysit` replaces it",
            src);
        g_enabled = false;
        return;
    }
    Log("citysit shadow: ON (citysit_shadow from %s): at PostCityInit and each SimNewMonth, the original city "
        "situation manager's record (its own Write into memory) loaded into FreeCity4's core and written back, byte "
        "for byte, and its getters (slots 5-15, 17, 18, 30, 32) against the core's. This is the only in-game "
        "check of this core.",
        src);
    Log("citysit shadow: NOT readable read-only (so not compared): the indicator tracking and its count (slots 19, "
        "20, 22-25 mark vehicles and slot 28 counts them: it is built from the AdviceAdded / AdviceRemoved messages "
        "this shadow does not follow), GetMaxCSI (slot 27: a process-wide static the automata manager loads from its "
        "own tuning, city-situation.md §2, not this manager's state), the mission itself (targets, pursuit, "
        "coverage, the player-drive camera: §0 leaves those to the host), the Lua table, the popup, the reward list "
        "it writes into the preferences, and the messages it sends");
}

bool CitySitShadowEnabled() { return g_enabled; }

void CitySitShadowCityLoaded(cISC4City* city) {
    if (!g_enabled || city == nullptr) return;
    cFC4ISC4Simulator* sim = AsGame(city->GetSimulator());
    const int32_t julian = sim != nullptr ? sim->GetSimDateNumber() : 0;
    RunOnce(city, julian, 0);
}

void CitySitShadowMonth(cISC4City* city, int32_t julian) {
    if (!g_enabled || city == nullptr) return;
    RunOnce(city, julian, 1);
}

void CitySitShadowCityShutdown() {
    if (!g_enabled) return;
    Log("citysit shadow: city shutdown: %u lines, %u with mismatches=0; compared %u values, %u mismatched; "
        "differences: %s",
        g_tally.lines, g_tally.clean, g_tally.compared, g_tally.mismatched, g_kinds.empty() ? "none" : g_kinds.c_str());
    if (g_core != nullptr) {
        fc4_citysit_destroy(g_core);
        g_core = nullptr;
    }
    g_tally = Tally{};
    g_kinds.clear();
}

intptr_t CitySitShadowTestRun(cISC4City* city, int32_t julian, uint32_t event) { return RunOnce(city, julian, event); }

void CitySitShadowTally(uint32_t* lines, uint32_t* clean, uint32_t* compared, uint32_t* mismatched) {
    if (lines != nullptr) *lines = g_tally.lines;
    if (clean != nullptr) *clean = g_tally.clean;
    if (compared != nullptr) *compared = g_tally.compared;
    if (mismatched != nullptr) *mismatched = g_tally.mismatched;
}

}  // namespace fc4

intptr_t CitySitShadowTest(FC4CitySitShadowTestInput* in) {
    if (in == nullptr || in->struct_size != sizeof(FC4CitySitShadowTestInput)) return -1;
    if (in->event == 1) {
        fc4::CitySitShadowCityShutdown();
        fc4::CitySitShadowTally(&in->lines, &in->clean, &in->compared, &in->mismatched);
        return 0;
    }
    fc4::CitySitShadowTally(&in->lines, &in->clean, &in->compared, &in->mismatched);
    return fc4::CitySitShadowEnabled() ? 1 : 0;
}
