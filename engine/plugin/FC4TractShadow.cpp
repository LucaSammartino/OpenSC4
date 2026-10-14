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

#include "FC4TractShadow.h"

#include "FC4Config.h"
#include "FC4DevelopmentInterfaces.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4TractInterfaces.h"
#include "FC4WriteCapture.h"
#include "cFC4TractDeveloper.h"
#include "compat/FC4Compat.h"

#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4City.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4SimGrid.h"

#include <algorithm>
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

// Tracts compared per developer and run, and the cells drawn to find them.
constexpr uint32_t kSampleTracts = 32;
constexpr uint32_t kSampleCells = 512;
// The most styles slot 9's vector is believed to hold.
constexpr size_t kMaxStyles = 4096;
// Differences listed per line.
constexpr unsigned kFirst = 6;

bool g_enabled = false;
bool g_loggedOurs = false;
bool g_loggedNone = false;

// The original's record (main thread only; reused).
WriteCapture g_capture;

// ---- the captured record as an fc4_istream, and an fc4_ostream into memory ---------------------------

struct RecordIn {
    const std::vector<uint8_t>* bytes = nullptr;
    size_t pos = 0;
    bool bad = false;
};

bool Take(void* u, void* out, size_t n) {
    RecordIn* r = static_cast<RecordIn*>(u);
    const std::vector<uint8_t>& b = *r->bytes;
    if (r->bad || n > b.size() - r->pos) {
        r->bad = true;
        return false;
    }
    if (out != nullptr && n != 0) memcpy(out, b.data() + r->pos, n);
    r->pos += n;
    return true;
}
bool InU8(void* u, uint8_t* o) { return Take(u, o, 1); }
bool InU16(void* u, uint16_t* o) { return Take(u, o, 2); }
bool InU32(void* u, uint32_t* o) { return Take(u, o, 4); }
bool InU64(void* u, uint64_t* o) { return Take(u, o, 8); }
bool InF32(void* u, float* o) { return Take(u, o, 4); }
bool InF64(void* u, double* o) { return Take(u, o, 8); }
bool InBytes(void* u, uint8_t* o, size_t n) { return Take(u, o, n); }
bool InSkip(void* u, size_t n) { return Take(u, nullptr, n); }
int32_t InError(void* u) { return static_cast<RecordIn*>(u)->bad ? 1 : 0; }
// The tract developer's record has no strings or references (§6).
bool InStr(void* u, fc4_bytes_sink, void*) {
    static_cast<RecordIn*>(u)->bad = true;
    return false;
}
bool InRef(void* u, uint32_t, fc4_ref* out) {
    *out = fc4_ref{};
    static_cast<RecordIn*>(u)->bad = true;
    return false;
}

fc4_istream MakeIn(RecordIn* in) {
    fc4_istream s{};
    s.struct_size = sizeof(s);
    s.user = in;
    s.get_u8 = &InU8;
    s.get_u16 = &InU16;
    s.get_u32 = &InU32;
    s.get_u64 = &InU64;
    s.get_f32 = &InF32;
    s.get_f64 = &InF64;
    s.get_bytes = &InBytes;
    s.get_str = &InStr;
    s.get_ref = &InRef;
    s.skip = &InSkip;
    s.error = &InError;
    return s;
}

struct RecordOut {
    std::vector<uint8_t> bytes;
    bool bad = false;
};

bool Put(void* u, const void* p, size_t n) {
    RecordOut* r = static_cast<RecordOut*>(u);
    const uint8_t* b = static_cast<const uint8_t*>(p);
    r->bytes.insert(r->bytes.end(), b, b + n);
    return true;
}
bool OutU8(void* u, uint8_t v) { return Put(u, &v, 1); }
bool OutU16(void* u, uint16_t v) { return Put(u, &v, 2); }
bool OutU32(void* u, uint32_t v) { return Put(u, &v, 4); }
bool OutU64(void* u, uint64_t v) { return Put(u, &v, 8); }
bool OutF32(void* u, float v) { return Put(u, &v, 4); }
bool OutF64(void* u, double v) { return Put(u, &v, 8); }
bool OutBytes(void* u, const uint8_t* d, size_t n) { return Put(u, d, n); }
bool OutStr(void* u, const uint8_t*, size_t) {
    static_cast<RecordOut*>(u)->bad = true;
    return false;
}
bool OutRef(void* u, const fc4_ref*) {
    static_cast<RecordOut*>(u)->bad = true;
    return false;
}
int32_t OutError(void* u) { return static_cast<RecordOut*>(u)->bad ? 1 : 0; }

fc4_ostream MakeOut(RecordOut* out) {
    fc4_ostream s{};
    s.struct_size = sizeof(s);
    s.user = out;
    s.put_u8 = &OutU8;
    s.put_u16 = &OutU16;
    s.put_u32 = &OutU32;
    s.put_u64 = &OutU64;
    s.put_f32 = &OutF32;
    s.put_f64 = &OutF64;
    s.put_bytes = &OutBytes;
    s.put_str = &OutStr;
    s.put_ref = &OutRef;
    s.error = &OutError;
    return s;
}

// ---- one run -----------------------------------------------------------------------------------------

void AppendF(std::string& s, const char* fmt, ...) {
    char buf[512];
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(buf, sizeof(buf), _TRUNCATE, fmt, ap);
    va_end(ap);
    s += buf;
}

// The first kFirst differences of a line ("-" when there are none).
struct FirstList {
    std::string text;
    unsigned listed = 0;
    void Add(const char* fmt, ...) {
        if (listed >= kFirst) return;
        char buf[256];
        va_list ap;
        va_start(ap, fmt);
        _vsnprintf_s(buf, sizeof(buf), _TRUNCATE, fmt, ap);
        va_end(ap);
        if (listed != 0) text += ",";
        text += buf;
        ++listed;
    }
    const char* Str() const { return listed == 0 ? "-" : text.c_str(); }
};

struct Run {
    cISC4City* city = nullptr;
    int32_t julian = 0;
    uint32_t event = 0;  // 0 = load (PostCityInit), 1 = month
    const fc4_tract_host* test = nullptr;
    char date[16] = "-";
    intptr_t result = 0;
};

void Line(const Run& r, const char* what, const std::string& body) {
    std::string line;
    AppendF(line, "tract shadow (%s) %s %s: ", what, r.date, r.event == 0 ? "load" : "month");
    line += body;
    LogShadowLine("tract shadow (cont.)", reinterpret_cast<const uint8_t*>(line.data()), line.size());
}

std::string Hex(const std::vector<uint32_t>& v) {
    std::string s;
    for (size_t i = 0; i < v.size() && i < 16; ++i) AppendF(s, "%s0x%X", i ? "," : "", v[i]);
    if (v.size() > 16) s += ",...";
    return s.empty() ? std::string("-") : s;
}

uint32_t NextRandom(uint32_t& s) {
    s = s * 1664525u + 1013904223u;
    return s >> 8;
}

// The population comparison's callback context.
struct PopCompare {
    uint32_t lots = 0;
    uint32_t mismatches = 0;
    FirstList* first = nullptr;
    int32_t tx = 0, tz = 0;
};

void ComparePopulation(void* ctx, uint64_t lot, uint32_t devType, uint16_t population) {
    PopCompare* c = static_cast<PopCompare*>(ctx);
    ++c->lots;
    cISC4Lot* l = reinterpret_cast<cISC4Lot*>(FromToken(lot));
    if (l == nullptr) return;
    const uint16_t now =
        AsGame(l)->GetPopulation(static_cast<cFC4ISC4Lot::DeveloperType>(devType));  // lot slot 53
    if (now != population) {
        ++c->mismatches;
        c->first->Add("tract %d,%d lot %p dev 0x%X orig %u ours %u", c->tx, c->tz, static_cast<void*>(l), devType,
                      now, population);
    }
}

// ---- the UpdatePopulation forwarder (cISC4TractDeveloper slot 7) ----------------------------------------
//
// A month-end sample compares the lots' populations with what UpdatePopulation
// would set now, but the original set them at its own last call, with the
// desirability and capacities of that day: the sample drifts. The forwarder
// compares at the call itself. On the identified executable only, and only
// if the original tract developer's vtable slot 7 holds the original
// UpdatePopulation (tract-developer.md §4.18, 0x00706BC0), that one entry is
// pointed at this forwarder (same `this`, argument and EAX; callee-pop 4).
// Per call: FreeCity4's shadow core computes the populations from the same
// inputs first (reads only), then the original runs, then each lot's
// population (slot 53) is compared with ours. Tract (0, 0) is forwarded
// without a comparison (§4.18 [quirk]). The entry is put back at
// PreAppShutdown and at DLL detach.

constexpr uintptr_t kOriginalUpdatePopulation = 0x00706BC0;  // §4.18
constexpr int kSlotUpdatePopulation = 7;

using UpdatePopulationFn = int32_t(__fastcall*)(void* self, void* edx, const FC4GrowthProfile& profile);

struct Expect {
    cISC4Lot* lot;
    uint32_t devType;
    uint16_t population;
};

struct Hook {
    bool tried = false;
    bool installed = false;
    void** entry = nullptr;
    uintptr_t original = 0;
    DWORD mainTid = 0;
    // The shadow core the forwarder computes with (main thread), over a
    // read-only table on the city's borrowed services.
    TractContext ctx;
    fc4_tract_host host{};
    fc4_tract* core = nullptr;
    bool inside = false;
    std::vector<Expect> expect;
    // Since the last line.
    uint64_t calls = 0, compared = 0, origin = 0, lots = 0, mismatches = 0, failures = 0;
    FirstList first;
    uint64_t totalCalls = 0, totalCompared = 0, totalMismatches = 0;
};
Hook g_hook;

void CollectExpected(void* ctx, uint64_t lot, uint32_t devType, uint16_t population) {
    static_cast<std::vector<Expect>*>(ctx)->push_back(
        Expect{reinterpret_cast<cISC4Lot*>(FromToken(lot)), devType, population});
}

struct HookStep {
    const FC4GrowthProfile* profile = nullptr;
    int32_t result = -1;
    uint32_t mismatches = 0;
};

void OursStep(void* p) {
    HookStep& s = *static_cast<HookStep*>(p);
    fc4_tract_profile prof{};
    prof.developer = TokenOf(s.profile->developer);
    prof.x = s.profile->x;
    prof.z = s.profile->z;
    s.result = fc4_tract_shadow_update_population(g_hook.core, &prof, &CollectExpected, &g_hook.expect);
}

void CompareStep(void* p) {
    HookStep& s = *static_cast<HookStep*>(p);
    for (const Expect& e : g_hook.expect) {
        if (e.lot == nullptr) continue;
        const uint16_t now = AsGame(e.lot)->GetPopulation(static_cast<cFC4ISC4Lot::DeveloperType>(e.devType));
        if (now != e.population) {
            ++s.mismatches;
            g_hook.first.Add("tract %d,%d lot %p dev 0x%X orig %u ours %u", s.profile->x, s.profile->z,
                             static_cast<void*>(e.lot), e.devType, now, e.population);
        }
    }
}

int32_t __fastcall UpdatePopulationForwarder(void* self, void* edx, const FC4GrowthProfile& profile) {
    const UpdatePopulationFn original = reinterpret_cast<UpdatePopulationFn>(g_hook.original);
    ++g_hook.calls;
    if (g_hook.core == nullptr || g_hook.inside || GetCurrentThreadId() != g_hook.mainTid) {
        return original(self, edx, profile);
    }
    if (profile.x == 0 && profile.z == 0) {
        ++g_hook.origin;  // §4.18 [quirk]: seeded from the clock in the original
        return original(self, edx, profile);
    }
    g_hook.inside = true;
    g_hook.expect.clear();
    HookStep s;
    s.profile = &profile;
    const bool ours = SehGuard("tract shadow UpdatePopulation (ours)", &OursStep, &s);
    const int32_t r = original(self, edx, profile);
    if (ours && s.result >= 0 && SehGuard("tract shadow UpdatePopulation (compare)", &CompareStep, &s)) {
        ++g_hook.compared;
        g_hook.lots += g_hook.expect.size();
        g_hook.mismatches += s.mismatches;
    } else {
        ++g_hook.failures;
    }
    TractReleaseHeld(&g_hook.ctx);  // the lots the shadow core's list holds
    g_hook.expect.clear();
    g_hook.inside = false;
    return r;
}

void InstallHook() {
    if (g_hook.tried) return;
    g_hook.tried = true;
    void** vt = nullptr;
    uintptr_t grow = 0;
    if (!compat::TractDeveloperGrow(&vt, &grow)) {
        Log("tract shadow: UpdatePopulation forwarder NOT installed: the original tract developer vtable is not "
            "available (see the compat lines); the population line samples tracts at each month instead");
        return;
    }
    void** entry = vt + kSlotUpdatePopulation;
    const uintptr_t found = reinterpret_cast<uintptr_t>(*entry);
    if (found != kOriginalUpdatePopulation) {
        Log("tract shadow: UpdatePopulation forwarder NOT installed: vtable %p slot 7 holds 0x%08lX, expected "
            "0x%08lX; the population line samples tracts at each month instead",
            static_cast<void*>(vt), static_cast<unsigned long>(found),
            static_cast<unsigned long>(kOriginalUpdatePopulation));
        return;
    }
    DWORD before = 0;
    if (!VirtualProtect(entry, sizeof(void*), PAGE_READWRITE, &before)) {
        Log("tract shadow: UpdatePopulation forwarder NOT installed: VirtualProtect failed (error %lu)",
            GetLastError());
        return;
    }
    g_hook.original = found;
    g_hook.mainTid = GetCurrentThreadId();
    *entry = reinterpret_cast<void*>(&UpdatePopulationForwarder);
    DWORD ignored = 0;
    const bool restored = VirtualProtect(entry, sizeof(void*), before, &ignored) != 0;
    FlushInstructionCache(GetCurrentProcess(), entry, sizeof(void*));
    g_hook.entry = entry;
    g_hook.installed = true;
    Log("tract shadow: UpdatePopulation forwarder installed at %p (vtable %p slot 7): 0x%08lX -> %p (FreeCity4.dll); "
        "page protection 0x%lX %s; every call is compared lot by lot (tract (0, 0) forwarded only)",
        static_cast<void*>(entry), static_cast<void*>(vt), static_cast<unsigned long>(found),
        reinterpret_cast<void*>(&UpdatePopulationForwarder), before, restored ? "restored" : "NOT restored");
}

void RemoveHook(const char* why) {
    if (!g_hook.installed) return;
    DWORD before = 0;
    if (!VirtualProtect(g_hook.entry, sizeof(void*), PAGE_READWRITE, &before)) {
        Log("tract shadow: WARNING UpdatePopulation forwarder NOT removed (%s): VirtualProtect failed (error %lu)", why,
            GetLastError());
        return;
    }
    *g_hook.entry = reinterpret_cast<void*>(g_hook.original);
    DWORD ignored = 0;
    VirtualProtect(g_hook.entry, sizeof(void*), before, &ignored);
    FlushInstructionCache(GetCurrentProcess(), g_hook.entry, sizeof(void*));
    g_hook.installed = false;
    Log("tract shadow: UpdatePopulation forwarder removed (%s): slot 7 holds 0x%08lX again; %llu call(s), %llu "
        "compared, %llu mismatch(es) in all",
        why, static_cast<unsigned long>(g_hook.original),
        static_cast<unsigned long long>(g_hook.totalCalls + g_hook.calls),
        static_cast<unsigned long long>(g_hook.totalCompared + g_hook.compared),
        static_cast<unsigned long long>(g_hook.totalMismatches + g_hook.mismatches));
}

// The forwarder's core: made anew for each city (at its load), with the
// tuning the original's Init read (§4.21); UpdatePopulation reads nothing
// else of the core's state.
void HookCoreFor(cISC4City* city) {
    if (g_hook.core != nullptr) {
        fc4_tract_destroy(g_hook.core);
        g_hook.core = nullptr;
    }
    TractReleaseHeld(&g_hook.ctx);
    g_hook.ctx.city = city;
    if (city == nullptr || !g_hook.installed) return;
    g_hook.ctx.readOnly = true;
    g_hook.ctx.gated = false;
    g_hook.host = MakeTractReadOnlyHost(&g_hook.ctx);
    g_hook.core = fc4_tract_create_shadow(&g_hook.host);
    if (g_hook.core != nullptr) fc4_tract_read_tuning(g_hook.core);
}

void RunStep(void* p) {
    Run& r = *static_cast<Run*>(p);
    cISC4TractDeveloper* raw = r.city->GetTractDeveloper();  // city slot 80, borrowed
    if (raw == nullptr) {
        if (!g_loggedNone) {
            g_loggedNone = true;
            Log("tract shadow %s: the city has no tract developer; nothing compared", r.date);
        }
        r.result = -1;
        return;
    }
    if (cFC4TractDeveloper::FromPrimary(raw) != nullptr) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("tract shadow %s: the city's tract developer is FreeCity4's (enable = tract); the shadow compares "
                "the ORIGINAL: nothing compared",
                r.date);
        }
        r.result = -2;
        return;
    }
    cFC4ISC4TractDeveloper* td = AsGame(raw);
    bool incomplete = false;
    intptr_t total = 0;

    // 1. The original's style state (slots 8, 9, 11, 13).
    const uint32_t oCurrent = static_cast<uint32_t>(td->GetCurrentStyle());
    std::vector<uint32_t> oStyles;
    bool stylesOk = true;
    {
        fc4::GameVector<uint32_t>& v = td->GetActiveStyles();  // B, the original's member
        const size_t n = v.size();
        if (n > kMaxStyles) {
            stylesOk = false;
        } else {
            for (size_t i = 0; i < n; ++i) oStyles.push_back(v[i]);
        }
    }
    const bool oAll = td->IsUsingAllStylesAtOnce();
    const int32_t oYears = td->GetYearsBetweenStyleChanges();

    // 2. The core in shadow mode on the original's record.
    TractContext ctx;
    if (r.test != nullptr) {
        ctx.test = r.test;
    } else {
        ctx.city = r.city;
    }
    const fc4_tract_host host = MakeTractReadOnlyHost(&ctx);
    fc4_tract* s = fc4_tract_create_shadow(&host);
    bool answered = false;
    const bool captured = g_capture.Capture(reinterpret_cast<cIGZUnknown*>(raw), &answered);
    std::string body;
    if (s == nullptr || !captured || !stylesOk) {
        AppendF(body, "INCOMPLETE (%s)",
                s == nullptr      ? "no shadow core"
                : !answered       ? "no cIGZSerializable"
                : !captured       ? "Write failed"
                                  : "slot 9's vector is not a vector");
        Line(r, "styles", body);
        incomplete = true;
    } else {
        const std::vector<uint8_t>& rec = g_capture.Bytes();
        RecordIn in;
        in.bytes = &rec;
        const fc4_istream is = MakeIn(&in);
        const bool readOk = fc4_tract_read(s, &is);
        fc4_tract_read_tuning(s);  // §4.21: what the original's Init did after its Read
        RecordOut out;
        const fc4_ostream os = MakeOut(&out);
        const bool writeOk = fc4_tract_write(s, &os);
        const bool same = readOk && writeOk && in.pos == rec.size() && out.bytes == rec;
        FirstList first;
        uint32_t mismatches = 0;
        if (!same) {
            ++mismatches;
            first.Add("record %zu/%zu bytes read=%d write=%d", out.bytes.size(), rec.size(), readOk ? 1 : 0,
                      writeOk ? 1 : 0);
        }
        const uint32_t sCurrent = fc4_tract_current_style(s);
        const size_t sn = fc4_tract_active_styles(s, nullptr, 0);
        std::vector<uint32_t> sStyles(sn);
        if (sn != 0) fc4_tract_active_styles(s, sStyles.data(), sn);
        const bool sAll = fc4_tract_is_using_all_styles(s);
        const int32_t sYears = fc4_tract_years_between_style_changes(s);
        if (sCurrent != oCurrent) {
            ++mismatches;
            first.Add("slot8 orig 0x%X ours 0x%X", oCurrent, sCurrent);
        }
        if (sStyles != oStyles) {
            ++mismatches;
            first.Add("slot9 orig %s ours %s", Hex(oStyles).c_str(), Hex(sStyles).c_str());
        }
        if (sAll != oAll) {
            ++mismatches;
            first.Add("slot11 orig %d ours %d", oAll ? 1 : 0, sAll ? 1 : 0);
        }
        if (sYears != oYears) {
            ++mismatches;
            first.Add("slot13 orig %d ours %d", oYears, sYears);
        }
        fc4_tract_state st{};
        st.struct_size = sizeof(st);
        fc4_tract_get_state(s, &st);
        AppendF(body, "record=%zu bytes roundtrip=%s current=0x%X index=%d styles=%s all=%d years=%d/%d "
                      "mismatches=%u first=%s",
                rec.size(), same ? "same" : "DIFFERENT", oCurrent, st.current_style_index, Hex(oStyles).c_str(),
                oAll ? 1 : 0, st.years_since_style_change, oYears, mismatches, first.Str());
        Line(r, "styles", body);
        total += mismatches;
    }

    // 3. UpdatePopulation (§4.18) on sample tracts, unless the forwarder
    // compares every call (the "calls" line).
    if (s != nullptr && (r.test != nullptr || !g_hook.installed)) {
        body.clear();
        cISC4BuildingDevelopmentSimulator* bdsRaw = r.city->GetBuildingDevelopmentSimulator();
        cISC4LotManager* lmRaw = r.city->GetLotManager();
        if (bdsRaw == nullptr || lmRaw == nullptr) {
            Line(r, "population", "INCOMPLETE (no BDS or lot manager)");
            incomplete = true;
        } else {
            cFC4ISC4BuildingDevelopmentSimulator* bds = AsGame(bdsRaw);
            cFC4ISC4LotManager* lm = AsGame(lmRaw);
            std::vector<cISC4GrowthDeveloper*> devs;
            for (uint32_t purpose = 1; purpose <= 8; ++purpose) {
                for (uint32_t wealth = 1; wealth <= 3; ++wealth) {
                    cISC4GrowthDeveloper* d = bds->GetDeveloperOf(purpose, wealth);  // slot 33, no reference
                    if (d != nullptr && std::find(devs.begin(), devs.end(), d) == devs.end()) devs.push_back(d);
                }
            }
            const int32_t cellsX = static_cast<int32_t>(r.city->CellCountX());
            const int32_t cellsZ = static_cast<int32_t>(r.city->CellCountZ());
            FirstList first;
            PopCompare cmp;
            cmp.first = &first;
            uint32_t tracts = 0;
            uint32_t seed = static_cast<uint32_t>(r.julian) * 2654435761u + r.event;
            for (cISC4GrowthDeveloper* d : devs) {
                cISC4SimGrid<int16_t>* g = AsGame(d)->GetDesirabilityMap();  // slot 13
                if (g == nullptr || cellsX <= 0 || cellsZ <= 0) continue;
                const int32_t shift = g->GetTractShift();
                if (shift < 0 || shift > 8) continue;
                std::vector<std::pair<int32_t, int32_t>> picked;
                for (uint32_t i = 0; i < kSampleCells && picked.size() < kSampleTracts; ++i) {
                    const int32_t x = static_cast<int32_t>(NextRandom(seed) % static_cast<uint32_t>(cellsX));
                    const int32_t z = static_cast<int32_t>(NextRandom(seed) % static_cast<uint32_t>(cellsZ));
                    if (lm->GetLot(x, z, false) == nullptr) continue;  // slot 6, borrowed
                    const std::pair<int32_t, int32_t> t(x >> shift, z >> shift);
                    if (t.first == 0 && t.second == 0) continue;  // §4.18 [quirk]: clock-seeded in the original
                    if (std::find(picked.begin(), picked.end(), t) != picked.end()) continue;
                    picked.push_back(t);
                }
                for (const auto& t : picked) {
                    fc4_tract_profile prof{};
                    prof.developer = TokenOf(d);
                    prof.x = t.first;
                    prof.z = t.second;
                    cmp.tx = t.first;
                    cmp.tz = t.second;
                    if (fc4_tract_shadow_update_population(s, &prof, &ComparePopulation, &cmp) < 0) {
                        incomplete = true;
                    }
                    TractReleaseHeld(&ctx);
                    ++tracts;
                }
            }
            AppendF(body, "developers=%zu tracts=%u lots=%u mismatches=%u first=%s", devs.size(), tracts, cmp.lots,
                    cmp.mismatches, first.Str());
            Line(r, "population", body);
            total += cmp.mismatches;
        }
    }

    fc4_tract_destroy(s);
    TractReleaseHeld(&ctx);
    r.result = incomplete ? -3 : total;
}

intptr_t RunOnce(cISC4City* city, int32_t julian, uint32_t event, const fc4_tract_host* test) {
    Run r;
    r.city = city;
    r.julian = julian;
    r.event = event;
    r.test = test;
    if (julian > 0) {
        fc4_date d{};
        fc4_date_from_julian(julian, &d);
        _snprintf_s(r.date, sizeof(r.date), _TRUNCATE, "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
    }
    if (city == nullptr) return -1;
    if (!SehGuard("tract shadow", &RunStep, &r)) {
        Log("tract shadow %s: WARNING structured exception; comparison abandoned", r.date);
        return -3;
    }
    return r.result;
}

}  // namespace

void TractShadowStartup(bool tractReplaced) {
    g_enabled = SettingInt("tract_shadow", "FC4_TRACT_SHADOW", 0) != 0;
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("tract_shadow", "FC4_TRACT_SHADOW", &src);
    if (tractReplaced) {
        Log("tract shadow: requested (tract_shadow from %s) but INACTIVE: it compares the ORIGINAL tract developer, "
            "and `enable = tract` replaces it",
            src);
        g_enabled = false;
        return;
    }
    Log("tract shadow: ON (tract_shadow from %s): at PostCityInit and each SimNewMonth, the original tract "
        "developer's style state (slots 8, 9, 11, 13) against FreeCity4's core loaded from the original's own record "
        "(its Write into memory, then ReadTuning; the core's Write must give the same bytes), and UpdatePopulation "
        "(at every call through a slot 7 forwarder on the identified executable, lot by lot; otherwise on up to %u "
        "sample tracts per growth developer at each month)",
        src, static_cast<unsigned>(kSampleTracts));
    Log("tract shadow: NOT compared: Grow and Decline (they change the city and draw from the generators), the "
        "histogram, the filter map; tract (0, 0) (seeded from the clock in the original)");
}

void TractShadowCityLoaded(cISC4City* city) {
    if (!g_enabled || city == nullptr) return;
    cFC4ISC4Simulator* sim = AsGame(city->GetSimulator());
    const int32_t julian = sim != nullptr ? sim->GetSimDateNumber() : 0;
    // The forwarder only for the original (RunOnce says when it is ours).
    cISC4TractDeveloper* raw = city->GetTractDeveloper();
    if (raw != nullptr && cFC4TractDeveloper::FromPrimary(raw) == nullptr) InstallHook();
    HookCoreFor(city);
    RunOnce(city, julian, 0, nullptr);
}

void TractShadowMonth(cISC4City* city, int32_t julian) {
    if (!g_enabled || city == nullptr) return;
    RunOnce(city, julian, 1, nullptr);
    if (!g_hook.installed) return;
    fc4_date d{};
    fc4_date_from_julian(julian, &d);
    std::string line;
    AppendF(line,
            "tract shadow (calls) %04d-%02d-%02d month: calls=%llu compared=%llu origin=%llu failures=%llu lots=%llu "
            "mismatches=%llu first=%s",
            d.year, d.month, d.day_of_month, static_cast<unsigned long long>(g_hook.calls),
            static_cast<unsigned long long>(g_hook.compared), static_cast<unsigned long long>(g_hook.origin),
            static_cast<unsigned long long>(g_hook.failures), static_cast<unsigned long long>(g_hook.lots),
            static_cast<unsigned long long>(g_hook.mismatches), g_hook.first.Str());
    LogShadowLine("tract shadow (cont.)", reinterpret_cast<const uint8_t*>(line.data()), line.size());
    g_hook.totalCalls += g_hook.calls;
    g_hook.totalCompared += g_hook.compared;
    g_hook.totalMismatches += g_hook.mismatches;
    g_hook.calls = g_hook.compared = g_hook.origin = g_hook.failures = g_hook.lots = g_hook.mismatches = 0;
    g_hook.first = FirstList{};
}

void TractShadowCityShutdown() {
    if (g_hook.core == nullptr && g_hook.ctx.city == nullptr) return;
    HookCoreFor(nullptr);
}

void TractShadowStop() {
    HookCoreFor(nullptr);
    RemoveHook("PreAppShutdown");
}

void TractShadowDetach() { RemoveHook("DLL detach"); }

intptr_t TractShadowTestRun(cISC4City* city, int32_t julian, uint32_t event, const fc4_tract_host* testHost) {
    return RunOnce(city, julian, event, testHost);
}

}  // namespace fc4
