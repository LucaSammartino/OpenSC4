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

#include "FC4DispatchShadow.h"

#include "FC4Config.h"
#include "FC4CoreShadow.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Streams.h"
#include "FC4WriteCapture.h"
#include "fc4.h"

#include "cISC4City.h"
#include "cISC4DispatchManager.h"
#include "cISC4Simulator.h"

#include <string>
#include <vector>

namespace fc4 {
namespace {

bool g_enabled = false;

fc4_dispatch_host g_host = {};
fc4_dispatch* g_core = nullptr;
WriteCapture g_capture;

// The dispatch types the sources can handle (dispatch.md §2); 8 is "no tool"
// and is what the state carries when no tool is open.
constexpr int32_t kTypes[] = {0, 1, 5, 6, 8};
constexpr size_t kTypeCount = sizeof(kTypes) / sizeof(kTypes[0]);
constexpr size_t kMaxIds = 64;

struct Tally {
    uint32_t lines = 0, clean = 0, compared = 0, mismatched = 0;
};

Tally g_tally;
std::string g_kinds;

// ---- the read-only host ---------------------------------------------------------------------------------
//
// Only `resolve_ref` is wired: the record's station, automaton and pylon
// references. Every callback that would change the game is NULL (a missing
// callback reads as 0 / false / none).

uint64_t ResolveRef(void*, uint64_t token, uint32_t) { return token; }  // the stream reader's reference

fc4_dispatch_host MakeReadOnlyHost() {
    fc4_dispatch_host h{};
    h.struct_size = sizeof(h);
    h.resolve_ref = &ResolveRef;
    return h;
}

// ---- the comparison -------------------------------------------------------------------------------------

struct Run {
    cISC4City* city = nullptr;
    int32_t julian = 0;
    uint32_t event = 0;  // 0 = load, 1 = month
    char date[16] = "-";
    intptr_t result = 0;
    uint32_t compared = 0;
};

// Our shadow core's record, with the references numbered as the capture's.
std::vector<uint8_t> WriteOurs(const WriteCapture& cap, uint32_t* unknownRefs) {
    ShadowOStream out;
    for (uint32_t i = 1;; ++i) {
        const void* o = cap.Object(i);
        if (o == nullptr) break;
        out.AddToken(TokenOf(o), i, cap.ClassOf(i));
    }
    const bool ok = g_core != nullptr && fc4_dispatch_write(g_core, out.Get());
    if (unknownRefs != nullptr) *unknownRefs = out.UnknownRefs();
    return ok ? out.Bytes() : std::vector<uint8_t>();
}

void RunStep(void* p) {
    Run& r = *static_cast<Run*>(p);
    cISC4DispatchManager* raw = r.city != nullptr ? r.city->GetDispatchManager() : nullptr;  // city slot 39
    cFC4ISC4DispatchManager* dm = raw != nullptr ? AsGame(raw) : nullptr;
    if (dm == nullptr) {
        Log("dispatch shadow %s: the city has no dispatch manager; nothing compared", r.date);
        r.result = -3;
        return;
    }

    CoreDiffs d;
    uint32_t compared = 0;

    // 1. the record round trip (§7): the original's own Write into memory,
    //    read into the shadow core, written back and compared byte for byte.
    bool answered = false;
    if (!g_capture.Capture(reinterpret_cast<cIGZUnknown*>(dm), &answered) || !answered) {
        Log("dispatch shadow %s: the original's record (its own Write) is not readable; nothing compared", r.date);
        r.result = -3;
        return;
    }
    const std::vector<uint8_t>& orig = g_capture.Bytes();
    if (g_core == nullptr) {
        g_host = MakeReadOnlyHost();
        g_core = fc4_dispatch_create(&g_host, 0);
    }
    bool readOk = false;
    if (g_core != nullptr) {
        CaptureIStream in(g_capture);
        readOk = fc4_dispatch_read(g_core, in.Get());
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

    // 2. the getters the original answers, type by type (slots 11, 21).
    for (size_t i = 0; i < kTypeCount; ++i) {
        const int32_t t = kTypes[i];
        const uint32_t oc = dm->GetDispatchCount(t);
        const uint32_t sc = g_core != nullptr ? fc4_dispatch_get_count(g_core, static_cast<uint32_t>(t)) : 0;
        ++compared;
        if (oc != sc) d.Add("count(%d) orig=%u ours=%u", t, oc, sc);

        const uint32_t oa = dm->GetAvailablePylonCount(t);
        const uint32_t sa =
            g_core != nullptr ? fc4_dispatch_get_available_pylon_count(g_core, static_cast<uint32_t>(t)) : 0;
        ++compared;
        if (oa != sa) d.Add("availPylons(%d) orig=%u ours=%u", t, oa, sa);
    }

    // 3. the ID arrays (slot 12) and the records (slot 10).
    for (size_t i = 0; i < kTypeCount; ++i) {
        const int32_t t = kTypes[i];
        uint32_t ids[kMaxIds] = {};
        uint32_t n = kMaxIds;  // the capacity; slot 12 never writes it
        const uint32_t written = dm->GetDispatchIDArray(t, ids, n);
        uint32_t oursIds[kMaxIds] = {};
        const size_t oursN =
            g_core != nullptr ? fc4_dispatch_get_id_array(g_core, static_cast<uint32_t>(t), 0, oursIds, kMaxIds) : 0;
        ++compared;
        if (written != oursN) {
            d.Add("idArray(%d) orig=%u ours=%u", t, written, static_cast<unsigned>(oursN));
        } else {
            for (uint32_t k = 0; k < written && k < kMaxIds; ++k) {
                if (ids[k] != oursIds[k]) d.Add("idArray(%d)[%u] orig=%u ours=%u", t, k, ids[k], oursIds[k]);
            }
        }
        for (uint32_t k = 0; k < written && k < kMaxIds; ++k) {
            FC4DispatchInfo info{};
            const bool okO = dm->GetDispatchInfo(ids[k], info);
            fc4_dispatch_info s{};
            const bool okS = g_core != nullptr && fc4_dispatch_get_info(g_core, ids[k], &s);
            ++compared;
            if (okO != okS) {
                d.Add("info(%u) orig=%d ours=%d", ids[k], okO ? 1 : 0, okS ? 1 : 0);
                continue;
            }
            if (!okO) continue;
            // The station, automaton and pylon travel as object identities.
            if (info.type != static_cast<int32_t>(s.dispatch_type) || info.id != s.id || info.callId != s.call_id ||
                (info.active ? 1 : 0) != s.active || info.viewHandle != s.view_handle ||
                TokenOf(info.station) != s.station || TokenOf(info.automaton) != s.automaton ||
                TokenOf(info.pylon) != s.pylon) {
                d.Add("info(%u) fields differ", ids[k]);
            }
        }
    }

    // 4. the active pylon (slot 18) as an object identity.
    {
        cISC4PylonOccupant* ap = dm->GetActivePylon();
        const uint64_t sap = g_core != nullptr ? fc4_dispatch_get_active_pylon(g_core) : 0;
        ++compared;
        if (TokenOf(ap) != sap) d.Add("activePylon orig=0x%llx ours=0x%llx", static_cast<unsigned long long>(TokenOf(ap)),
                                      static_cast<unsigned long long>(sap));
    }

    r.compared = compared;

    std::string body;
    AppendF(body, "record=%u bytes roundtrip=%s", static_cast<unsigned>(orig.size()), same ? "same" : "DIFFERENT");
    AppendF(body, "; compared=%u mismatches=%u first=%s", compared, d.n, d.Str());
    CoreShadowLine("dispatch shadow (cont.)", "dispatch", r.date, r.event != 0, body);

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
    const bool ok = SehGuard("dispatch shadow", &RunStep, &r);
    if (!ok) {
        Log("dispatch shadow %s: WARNING structured exception; comparison abandoned", r.date);
        return -3;
    }
    return r.result;
}

}  // namespace

void DispatchShadowStartup(bool dispatchReplaced) {
    g_enabled = SettingInt("dispatch_shadow", "FC4_DISPATCH_SHADOW", 0) != 0;
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("dispatch_shadow", "FC4_DISPATCH_SHADOW", &src);
    if (dispatchReplaced) {
        Log("dispatch shadow: requested (dispatch_shadow from %s) but INACTIVE: it compares the ORIGINAL dispatch "
            "manager, and `enable = dispatch` replaces it",
            src);
        g_enabled = false;
        return;
    }
    Log("dispatch shadow: ON (dispatch_shadow from %s): at PostCityInit and each SimNewMonth, the original dispatch "
        "manager's record (its own Write into memory) loaded into FreeCity4's core and written back, byte for byte, "
        "and its getters (slots 10-12, 18, 21) against the core's",
        src);
    Log("dispatch shadow: NOT readable read-only (so not compared): the automaton events (0x09D92FC3) the manager "
        "gets as the vehicle's event target, the messages it posts (0xE94671EB-ED, 0x894671F6), the tool's Begin / "
        "End (slots 16, 17 write the pylon list) and the vehicle view's tracking (slot 22 is presentation)");
}

void DispatchShadowCityLoaded(cISC4City* city) {
    if (!g_enabled || city == nullptr) return;
    cFC4ISC4Simulator* sim = AsGame(city->GetSimulator());
    const int32_t julian = sim != nullptr ? sim->GetSimDateNumber() : 0;
    RunOnce(city, julian, 0);
}

void DispatchShadowMonth(cISC4City* city, int32_t julian) {
    if (!g_enabled || city == nullptr) return;
    RunOnce(city, julian, 1);
}

void DispatchShadowCityShutdown() {
    if (!g_enabled) return;
    Log("dispatch shadow: city shutdown: %u lines, %u with mismatches=0; compared %u values, %u mismatched; "
        "differences: %s",
        g_tally.lines, g_tally.clean, g_tally.compared, g_tally.mismatched, g_kinds.empty() ? "none" : g_kinds.c_str());
    if (g_core != nullptr) {
        fc4_dispatch_destroy(g_core);
        g_core = nullptr;
    }
    g_tally = Tally{};
    g_kinds.clear();
}

intptr_t DispatchShadowTestRun(cISC4City* city, int32_t julian, uint32_t event) {
    return RunOnce(city, julian, event);
}

bool DispatchShadowEnabled() { return g_enabled; }

void DispatchShadowTally(uint32_t* lines, uint32_t* clean, uint32_t* compared, uint32_t* mismatched) {
    if (lines != nullptr) *lines = g_tally.lines;
    if (clean != nullptr) *clean = g_tally.clean;
    if (compared != nullptr) *compared = g_tally.compared;
    if (mismatched != nullptr) *mismatched = g_tally.mismatched;
}

}  // namespace fc4

intptr_t DispatchShadowTest(FC4DispatchShadowTestInput* in) {
    if (in == nullptr || in->struct_size != sizeof(FC4DispatchShadowTestInput)) return -1;
    if (in->event == 1) {
        fc4::DispatchShadowCityShutdown();
        fc4::DispatchShadowTally(&in->lines, &in->clean, &in->compared, &in->mismatched);
        return 0;
    }
    fc4::DispatchShadowTally(&in->lines, &in->clean, &in->compared, &in->mismatched);
    return fc4::DispatchShadowEnabled() ? 1 : 0;
}
