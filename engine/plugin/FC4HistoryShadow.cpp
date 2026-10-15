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

#include "FC4HistoryShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4HistClock.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Watchdog.h"
#include "FC4WriteCapture.h"
#include "cFC4Date.h"
#include "fc4.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <map>
#include <string>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;

// history.md §3, §4.1, §4.2 and the simulator's levels.
constexpr uint32_t kMsgRecord = 0x89EFA536;
constexpr uint32_t kMsgQueueLength = 0x89EFA537;
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgSimNewYear = 0x66956817;
constexpr uint32_t kMsgPostCityShutdown = 0x26D31EC3;
constexpr const char* kAgentName = "cSC4HistoryWarehouse";  // §3: both agent levels, one name
constexpr uint32_t kMessages[] = {kMsgRecord, kMsgQueueLength, kMsgSimNewMonth, kMsgPostCityShutdown};

constexpr uint16_t kRecordVersion = 2;  // §7
constexpr uint32_t kMaxSeries = 1u << 16;
constexpr uint32_t kMaxPoints = 1u << 20;
// A float no query writes: an output that must stay untouched starts with it.
constexpr uint32_t kSentinelBits = 0x7FC0DEADu;
constexpr int kMaxDetails = 6;
// A key no producer uses (§6): the "missing key" queries.
constexpr uint32_t kMissingKey = 0x0000F00Du;

// ---- settings and run state (main thread only) ---------------------------------------------------

bool g_enabled = false;
bool g_replaced = false;
bool g_started = false;
bool g_loggedOurs = false;
bool g_loggedNone = false;
bool g_loggedNoRecord = false;

// The shadow core and the original it follows (identity only).
fc4_history* g_core = nullptr;
const void* g_coreOf = nullptr;
uint32_t g_date = 0;  // the date the core's Store / pass read (set before each core call)

// The test seam's original (FreeCity4's own warehouse is accepted there).
cFC4ISC4HistoryWarehouse* g_testOriginal = nullptr;

// Since the last comparison line.
struct Counts {
    uint32_t records = 0, lengths = 0, days = 0, years = 0, splits = 0, stepDiffs = 0;
};
Counts g_counts;
uint32_t g_comparisons = 0, g_clean = 0;
// The original's pass step was split by its 5 ms timer (it answered false
// where the shadow core answered true): its repeats are skipped until it
// answers true.
bool g_ahead = false;

void ReadSettings() { g_enabled = SettingInt("history_shadow", "FC4_HISTORY_SHADOW", 0) != 0; }

// ---- the shadow core's host: read-only -------------------------------------------------------------

uint32_t HostDate(void*) { return g_date; }
uint32_t HostNowMs(void*) { return 0; }  // the shadow's pass timer never runs out (see Deliver)
void HostSubscribe(void*, uint32_t) {}
void HostAgent(void*, uint32_t, const char*, size_t, uint32_t) {}
void HostRemoveAgent(void*) {}
void HostSend(void*, uint32_t, uint32_t, uint32_t) {}

fc4_history_host ShadowHost() {
    fc4_history_host h{};
    h.struct_size = sizeof(h);
    h.sim_date = &HostDate;
    h.now_ms = &HostNowMs;
    h.subscribe = &HostSubscribe;
    h.unsubscribe = &HostSubscribe;
    h.add_agent = &HostAgent;
    h.remove_agent = &HostRemoveAgent;
    h.send = &HostSend;
    return h;
}

// ---- the game ------------------------------------------------------------------------------------

cISC4City* CurrentCity() {
    if (const FC4HistClockHooks* h = HistClockHooks()) return h->city ? h->city(h->ctx) : nullptr;
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    FC4_ACQUIRED(sc4, "QueryInterface");
    cISC4City* city = sc4->GetCity();  // borrowed
    FC4_RELEASE(sc4);
    return city;
}

cFC4ISC4HistoryWarehouse* OriginalOf(cISC4City* city) {
    if (g_testOriginal != nullptr) return g_testOriginal;
    if (city == nullptr) return nullptr;
    cISC4HistoryWarehouse* h = city->GetHistoryWarehouse();  // slot 32, borrowed
    if (h == nullptr) {
        if (!g_loggedNone) {
            g_loggedNone = true;
            Log("history shadow: the city has no history warehouse; nothing compared");
        }
        return nullptr;
    }
    if (IsOurObject(h)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("history shadow: inactive: the city's history warehouse is FreeCity4's own (the shadow compares "
                "against the ORIGINAL: do not enable history)");
        }
        return nullptr;
    }
    return AsGame(h);
}

// The date the original's Store and pass read (simulator slot 8).
uint32_t SimDate(cISC4City* city) {
    if (const FC4HistClockHooks* h = HistClockHooks()) return h->sim_date ? h->sim_date(h->ctx) : 0;
    cISC4Simulator* s = city ? city->GetSimulator() : nullptr;  // borrowed
    cFC4ISC4Simulator* sim = AsGame(s);
    return sim ? static_cast<uint32_t>(sim->GetSimDateNumber()) : 0;
}

std::string DateText(int32_t julian) {
    char buf[16] = "?";
    if (julian > 0) {
        fc4_date d{};
        fc4_date_from_julian(julian, &d);
        _snprintf_s(buf, sizeof(buf), _TRUNCATE, "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
    }
    return buf;
}

cIGZUnknown* Unknown(cFC4ISC4HistoryWarehouse* o) { return static_cast<cIGZUnknown*>(o); }

// ---- records ------------------------------------------------------------------------------------------

struct Series {
    uint32_t maxCount = 0;
    std::vector<fc4_history_point> points;
};
using SeriesMap = std::map<uint32_t, Series>;

// §7, version 2 (what Write writes): false on a short or malformed record.
bool Parse(const std::vector<uint8_t>& b, SeriesMap& out) {
    size_t pos = 0;
    auto take = [&](void* p, size_t n) {
        if (pos + n > b.size()) return false;
        std::memcpy(p, b.data() + pos, n);
        pos += n;
        return true;
    };
    uint16_t version = 0;
    uint32_t count = 0;
    if (!take(&version, 2) || version != kRecordVersion || !take(&count, 4) || count > kMaxSeries) return false;
    for (uint32_t i = 0; i < count; ++i) {
        uint32_t key = 0, n = 0;
        Series s;
        if (!take(&key, 4) || !take(&s.maxCount, 4) || !take(&n, 4) || n > kMaxPoints) return false;
        s.points.resize(n);
        for (uint32_t j = 0; j < n; ++j) {
            if (!take(&s.points[j].date, 4) || !take(&s.points[j].value, 4)) return false;
        }
        out[key] = std::move(s);
    }
    return pos == b.size();
}

bool CaptureOriginal(cFC4ISC4HistoryWarehouse* o, std::vector<uint8_t>& out) {
    WriteCapture cap;
    bool answered = false;
    const bool ok = cap.Capture(Unknown(o), &answered);
    out = cap.Bytes();
    if (!ok && !g_loggedNoRecord) {
        g_loggedNoRecord = true;
        Log("history shadow: the original's record (its own Write) %s; nothing compared",
            answered ? "failed" : "cannot be read (no cIGZSerializable)");
    }
    return ok;
}

std::vector<uint8_t> WriteOurs() {
    MemOStream o;
    if (g_core != nullptr) fc4_history_write(g_core, o.Get());
    return o.bytes;
}

void DropCore() {
    fc4_history_destroy(g_core);
    g_core = nullptr;
    g_coreOf = nullptr;
    g_ahead = false;
}

// A new shadow core from the original's record bytes.
bool BuildCore(cFC4ISC4HistoryWarehouse* o, const std::vector<uint8_t>& bytes) {
    DropCore();
    const fc4_history_host host = ShadowHost();  // copied by the core
    g_core = fc4_history_create(&host);
    if (g_core == nullptr) {
        Log("history shadow: the shadow core could not be created; nothing compared");
        return false;
    }
    g_coreOf = o;
    MemIStream in(bytes);
    return fc4_history_read(g_core, in.Get()) && in.Remaining() == 0;
}

uint32_t PassState() {
    fc4_history_state st{};
    st.struct_size = sizeof(st);
    return (g_core != nullptr && fc4_history_get_state(g_core, &st)) ? st.pass_state : 0;
}

void Detail(int& shown, const char* fmt, ...) {
    if (shown++ >= kMaxDetails) return;
    char buf[640];
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(buf, sizeof(buf), _TRUNCATE, fmt, ap);
    va_end(ap);
    Log("history shadow (cont.) %s", buf);
}

// The series of the two records, key by key: the number that differ.
uint32_t CompareSeries(const SeriesMap& orig, const SeriesMap& ours, int& shown) {
    uint32_t diffs = 0;
    for (const auto& kv : orig) {
        const auto it = ours.find(kv.first);
        if (it == ours.end()) {
            ++diffs;
            Detail(shown, "series 0x%08X: only in the original (%u points, maxCount %u)", kv.first,
                   static_cast<unsigned>(kv.second.points.size()), kv.second.maxCount);
            continue;
        }
        const Series& a = kv.second;
        const Series& b = it->second;
        if (a.maxCount != b.maxCount) {
            ++diffs;
            Detail(shown, "series 0x%08X: maxCount %u, ours %u (a queue length set through slot 13?)", kv.first,
                   a.maxCount, b.maxCount);
            continue;
        }
        if (a.points.size() != b.points.size() ||
            (!a.points.empty() &&
             std::memcmp(a.points.data(), b.points.data(), a.points.size() * sizeof(fc4_history_point)) != 0)) {
            ++diffs;
            size_t i = 0;
            while (i < a.points.size() && i < b.points.size() && a.points[i].date == b.points[i].date &&
                   a.points[i].value == b.points[i].value) {
                ++i;
            }
            const std::string da = i < a.points.size() ? DateText(static_cast<int32_t>(a.points[i].date)) : "-";
            const std::string db = i < b.points.size() ? DateText(static_cast<int32_t>(b.points[i].date)) : "-";
            Detail(shown, "series 0x%08X: %u points, ours %u; first difference at %u: (%s, %d) vs (%s, %d)", kv.first,
                   static_cast<unsigned>(a.points.size()), static_cast<unsigned>(b.points.size()),
                   static_cast<unsigned>(i), da.c_str(), i < a.points.size() ? a.points[i].value : 0, db.c_str(),
                   i < b.points.size() ? b.points[i].value : 0);
        }
    }
    for (const auto& kv : ours) {
        if (orig.find(kv.first) == orig.end()) {
            ++diffs;
            Detail(shown, "series 0x%08X: only in the shadow (%u points)", kv.first,
                   static_cast<unsigned>(kv.second.points.size()));
        }
    }
    return diffs;
}

// ---- the queries (slots 5-8) -----------------------------------------------------------------------------

uint32_t Bits(float f) {
    uint32_t b = 0;
    std::memcpy(&b, &f, 4);
    return b;
}

float Sentinel() {
    float f = 0;
    std::memcpy(&f, &kSentinelBits, 4);
    return f;
}

struct QueryRun {
    cFC4ISC4HistoryWarehouse* orig = nullptr;
    uint32_t now = 0;
    std::vector<uint32_t> keys;
    uint32_t compared = 0;
    uint32_t diffs = 0;
    int shown = 0;
    bool done = false;
};

// One query on both sides: the result and every output's bits.
void Check(QueryRun& r, const char* what, uint32_t key, uint32_t from, uint32_t to, uint32_t bits, bool okO,
           const float* o, bool okS, const float* s, int n) {
    ++r.compared;
    bool same = okO == okS;
    for (int i = 0; i < n && same; ++i) same = Bits(o[i]) == Bits(s[i]);
    if (same) return;
    ++r.diffs;
    char vals[192] = "";
    size_t len = 0;
    for (int i = 0; i < n; ++i) {
        len += static_cast<size_t>(_snprintf_s(vals + len, sizeof(vals) - len, _TRUNCATE, " %08X/%08X", Bits(o[i]),
                                               Bits(s[i])));
        if (len >= sizeof(vals)) break;
    }
    Detail(r.shown, "%s key 0x%08X [%s, %s] %u-bit: original %d, ours %d; outputs original/ours%s", what, key,
           DateText(static_cast<int32_t>(from)).c_str(), DateText(static_cast<int32_t>(to)).c_str(), bits, okO ? 1 : 0,
           okS ? 1 : 0, vals);
}

void QueryKey(QueryRun& r, uint32_t key) {
    // The series' ends (the shadow's: the records were compared first).
    uint32_t maxCount = 0;
    size_t count = 0;
    fc4_history_get_series(g_core, key, &maxCount, nullptr, 0, &count);
    std::vector<fc4_history_point> pts(count);
    if (count > 0) fc4_history_get_series(g_core, key, &maxCount, pts.data(), count, &count);
    const uint32_t now = r.now;
    const uint32_t first = pts.empty() ? now : pts.front().date;
    const uint32_t last = pts.empty() ? now : pts.back().date;
    const uint32_t ranges[][2] = {{now - 30, now},   {now - 365, now},       {now + 1, now},  // Lua trend_count (§4.6)
                                  {first, last},     {first - 10, first - 1}, {now - 3650, now - 1826}};
    const uint32_t points[] = {now, now - 15, now - 400, first};
    cFC4ISC4HistoryWarehouse* o = r.orig;
    for (const auto& rg : ranges) {
        cFC4Date df(static_cast<int32_t>(rg[0])), dt(static_cast<int32_t>(rg[1]));
        {
            float a[1] = {Sentinel()}, b[1] = {Sentinel()};
            const bool okO = o->GetCount(key, df, dt, a[0]);
            const bool okS = fc4_history_get_count(g_core, key, rg[0], rg[1], &b[0]);
            Check(r, "GetCount", key, rg[0], rg[1], 0, okO, a, okS, b, 1);
        }
        for (uint32_t bits : {24u, 53u}) {
            float a[3] = {Sentinel(), Sentinel(), Sentinel()}, b[3] = {Sentinel(), Sentinel(), Sentinel()};
            bool okO;
            {
                PrecisionScope p(bits);
                okO = o->GetTrend(key, df, dt, a[0], a[1]);
            }
            bool okS = fc4_history_get_trend(g_core, key, rg[0], rg[1], bits, &b[0], &b[1]);
            Check(r, "GetTrend", key, rg[0], rg[1], bits, okO, a, okS, b, 2);
            a[0] = a[1] = a[2] = b[0] = b[1] = b[2] = Sentinel();
            {
                PrecisionScope p(bits);
                okO = o->GetMinMaxAverage(key, df, dt, a[0], a[1], a[2]);
            }
            okS = fc4_history_get_min_max_average(g_core, key, rg[0], rg[1], bits, &b[0], &b[1], &b[2]);
            Check(r, "GetMinMaxAverage", key, rg[0], rg[1], bits, okO, a, okS, b, 3);
        }
    }
    for (uint32_t d : points) {
        cFC4Date dd(static_cast<int32_t>(d));
        for (uint32_t bits : {24u, 53u}) {
            float a[1] = {Sentinel()}, b[1] = {Sentinel()};
            bool okO;
            {
                PrecisionScope p(bits);
                okO = o->GetPoint(key, dd, a[0]);
            }
            const bool okS = fc4_history_get_point(g_core, key, d, bits, &b[0]);
            Check(r, "GetPoint", key, d, d, bits, okO, a, okS, b, 1);
        }
    }
}

void RunQueries(void* ctx) {
    QueryRun& r = *static_cast<QueryRun*>(ctx);
    for (uint32_t key : r.keys) QueryKey(r, key);
    QueryKey(r, kMissingKey);  // false on both sides, outputs untouched (§4.7)
    r.done = true;
}

// ---- the comparisons --------------------------------------------------------------------------------------

struct Compare {
    cFC4ISC4HistoryWarehouse* orig = nullptr;
    const char* what = "";
    int32_t julian = 0;
    intptr_t result = -1;
    uint32_t compared = 0;
};

void CompareStep(void* ctx) {
    Compare& c = *static_cast<Compare*>(ctx);
    std::vector<uint8_t> orig;
    if (!CaptureOriginal(c.orig, orig)) return;
    const std::vector<uint8_t> ours = WriteOurs();
    uint32_t seriesDiffs = 0;
    int shown = 0;
    SeriesMap mo, ms;
    const bool parsed = Parse(orig, mo) && Parse(ours, ms);
    if (orig != ours) {
        seriesDiffs = parsed ? CompareSeries(mo, ms, shown) : 1;
        if (seriesDiffs == 0) seriesDiffs = 1;  // the bytes differ: count it even when no series does
        if (!parsed) Detail(shown, "a record does not parse (%u / %u bytes)", static_cast<unsigned>(orig.size()),
                            static_cast<unsigned>(ours.size()));
    }
    QueryRun q;
    q.orig = c.orig;
    q.now = g_date;
    q.shown = shown;
    for (const auto& kv : ms) q.keys.push_back(kv.first);
    if (!SehGuard("history shadow (queries)", &RunQueries, &q) || !q.done) {
        Log("history shadow %s %s: WARNING structured exception in the queries; abandoned",
            DateText(c.julian).c_str(), c.what);
    }
    uint32_t points = 0;
    for (const auto& kv : mo) points += static_cast<uint32_t>(kv.second.points.size());
    const uint32_t mismatches = seriesDiffs + q.diffs + g_counts.stepDiffs;
    ++g_comparisons;
    if (mismatches == 0) ++g_clean;
    Log("history shadow %s %s: series=%u points=%u record=%u bytes %s; queries=%u (24/53-bit) differing=%u; "
        "records=%u lengths=%u days=%u years=%u split_steps=%u step_results_differing=%u pass=%u; mismatches=%u; "
        "comparisons=%u clean=%u",
        DateText(c.julian).c_str(), c.what, static_cast<unsigned>(mo.size()), points,
        static_cast<unsigned>(orig.size()), orig == ours ? "identical" : "DIFFERENT", q.compared, q.diffs,
        g_counts.records, g_counts.lengths, g_counts.days, g_counts.years, g_counts.splits, g_counts.stepDiffs,
        PassState(), mismatches, g_comparisons, g_clean);
    g_counts = Counts{};
    c.compared = static_cast<uint32_t>(mo.size()) + q.compared;
    c.result = static_cast<intptr_t>(mismatches);
    // After a record mismatch: rebuilt from the original while both passes
    // are idle (the shadow's pass state, and no split step pending).
    if (seriesDiffs > 0 && !g_ahead && PassState() == 0 && parsed) {
        if (BuildCore(c.orig, orig)) Log("history shadow (cont.) rebuilt from the original's record");
    }
}

intptr_t CompareNow(const char* what, int32_t julian, uint32_t* compared) {
    cFC4ISC4HistoryWarehouse* o = OriginalOf(CurrentCity());
    if (o == nullptr || g_core == nullptr || g_coreOf != o) return -1;
    if (g_ahead) {
        Log("history shadow %s %s: deferred (the original's pass step is split by its timer)",
            DateText(julian).c_str(), what);
        return -1;
    }
    g_date = SimDate(CurrentCity());
    Compare c;
    c.orig = o;
    c.what = what;
    c.julian = julian > 0 ? julian : static_cast<int32_t>(g_date);
    if (!SehGuard("history shadow (compare)", &CompareStep, &c)) {
        Log("history shadow %s %s: WARNING structured exception; comparison abandoned", DateText(c.julian).c_str(),
            what);
        return -1;
    }
    if (compared != nullptr) *compared = c.compared;
    return c.result;
}

struct Load {
    cFC4ISC4HistoryWarehouse* orig = nullptr;
    int32_t julian = 0;
    intptr_t result = -1;
    uint32_t compared = 0;
};

void LoadStep(void* ctx) {
    Load& l = *static_cast<Load*>(ctx);
    std::vector<uint8_t> orig;
    if (!CaptureOriginal(l.orig, orig)) return;
    const bool read = BuildCore(l.orig, orig);
    const std::vector<uint8_t> ours = WriteOurs();
    SeriesMap mo;
    const bool parsed = Parse(orig, mo);
    uint32_t points = 0;
    for (const auto& kv : mo) points += static_cast<uint32_t>(kv.second.points.size());
    const uint32_t diffs = (read ? 0u : 1u) + (orig == ours ? 0u : 1u) + (parsed ? 0u : 1u);
    Log("history shadow %s load: the original's record %u bytes (%u series, %u points) read by the shadow core: %s, "
        "written back %s; mismatches=%u",
        DateText(l.julian).c_str(), static_cast<unsigned>(orig.size()), static_cast<unsigned>(mo.size()), points,
        read ? "ok" : "FAILED", orig == ours ? "identical" : "DIFFERENT", diffs);
    g_counts = Counts{};
    l.compared = static_cast<uint32_t>(mo.size());
    l.result = static_cast<intptr_t>(diffs);
}

intptr_t OnCityLoaded(cISC4City* city, int32_t julian, uint32_t* compared) {
    cFC4ISC4HistoryWarehouse* o = OriginalOf(city);
    if (o == nullptr) return -1;
    g_date = SimDate(city);
    Load l;
    l.orig = o;
    l.julian = julian > 0 ? julian : static_cast<int32_t>(g_date);
    if (!SehGuard("history shadow (load)", &LoadStep, &l)) {
        Log("history shadow %s load: WARNING structured exception; the shadow is off for this city",
            DateText(l.julian).c_str());
        DropCore();
        return -1;
    }
    if (compared != nullptr) *compared = l.compared;
    // The queries at load, against the record just read.
    if (l.result >= 0 && g_core != nullptr) {
        uint32_t c = 0;
        const intptr_t q = CompareNow("load queries", l.julian, &c);
        if (compared != nullptr) *compared += c;
        if (q > 0) l.result += q;
    }
    return l.result;
}

// A record or queue-length message (§3): to the shadow core with the date the
// original's Store reads.
intptr_t OnMessage(uint32_t type, uint32_t data1, uint32_t data2) {
    if (g_core == nullptr) return -1;
    if (type != kMsgRecord && type != kMsgQueueLength) return 0;
    g_date = SimDate(CurrentCity());
    fc4_history_do_message(g_core, type, data1, data2);
    if (type == kMsgRecord) {
        ++g_counts.records;
    } else {
        ++g_counts.lengths;
    }
    return 1;
}

// The original's agent delivery has run (`finished` its result): the shadow
// core's step (§4.3).
intptr_t AfterOriginalStep(uint32_t type, bool finished, int32_t julian) {
    if (g_core == nullptr) return 0;
    g_date = SimDate(CurrentCity());
    if (type == kMsgSimNewYear) {
        ++g_counts.years;
        if (!fc4_history_do_message(g_core, type, 0, 0) || !finished) ++g_counts.stepDiffs;
        return 1;
    }
    ++g_counts.days;
    if (g_ahead) {
        // The shadow already did this step's work in one call.
        ++g_counts.splits;
        if (finished) g_ahead = false;
        return 1;
    }
    const uint32_t before = PassState();
    const bool ours = fc4_history_do_message(g_core, type, 0, 0);
    if (ours != finished) {
        if (ours && !finished) {
            g_ahead = true;  // the original's 5 ms timer stopped its step early
            ++g_counts.splits;
        } else {
            ++g_counts.stepDiffs;
            Log("history shadow (cont.) %s: SimNewDay step: original %d, ours %d", DateText(julian).c_str(),
                finished ? 1 : 0, ours ? 1 : 0);
        }
    }
    if (before != 0 && PassState() == 0 && !g_ahead) CompareNow("pass end", julian, nullptr);
    return 1;
}

class Target final : public cIGZMessageTarget2 {
public:
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid != kIID_cIGZMessageTarget2 && riid != GZIID_cIGZUnknown) return false;
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return static_cast<uint32_t>(InterlockedIncrement(&refs_)); }
    uint32_t Release() override {
        const LONG r = InterlockedDecrement(&refs_);
        return r > 0 ? static_cast<uint32_t>(r) : 0u;
    }
    bool DoMessage(cIGZMessage2* msg) override {
        if (msg == nullptr) return true;
        const uint32_t type = msg->GetType();
        if (type == kMsgPostCityShutdown) {
            DropCore();
            return true;
        }
        uint32_t d1 = 0, d2 = 0;
        cIGZMessage2Standard* sm = nullptr;
        if (msg->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&sm)) && sm != nullptr) {
            FC4_ACQUIRED(sm, "QueryInterface");
            d1 = static_cast<uint32_t>(sm->GetData1());
            d2 = static_cast<uint32_t>(sm->GetData2());
            FC4_RELEASE(sm);
        }
        if (type == kMsgSimNewMonth) {
            CompareNow("month", d1 != 0 ? static_cast<int32_t>(d1) : static_cast<int32_t>(PublishedJulian()), nullptr);
        } else {
            OnMessage(type, d1, d2);
        }
        return true;
    }

private:
    volatile LONG refs_ = 0;
};

Target& TheTarget() {
    static Target t;
    return t;
}

}  // namespace

void HistoryShadowStartup(bool historyReplaced) {
    ReadSettings();
    g_replaced = historyReplaced;
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("history_shadow", "FC4_HISTORY_SHADOW", &src);
    if (historyReplaced) {
        Log("history shadow: requested (history_shadow from %s) but INACTIVE: it compares against the ORIGINAL "
            "history warehouse, and history is enabled (ours)",
            src);
        return;
    }
    Log("history shadow: ON (history_shadow from %s): a shadow core (FreeCity4's warehouse) is read from the "
        "ORIGINAL warehouse's record at each city load and follows it read-only: the record (0x89EFA536) and "
        "queue-length (0x89EFA537) messages with the simulation date, and its SimNewYear / SimNewDay agent steps "
        "(the yearly pass; its 5 ms timer splits are followed); at each SimNewMonth and pass end the two records "
        "(the original's own Write) are compared byte for byte and series by series, and the queries (slots 5-8) "
        "on six ranges and four dates per series at 24-bit and 53-bit precision, bit for bit; queue lengths set "
        "through slot 13 outside a message are not seen",
        src);
}

void HistoryShadowStart() {
    if (g_started || !g_enabled || g_replaced) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) {
        Log("history shadow: no message server; not started");
        return;
    }
    TheTarget().AddRef();  // our own reference (a static object)
    bool ok = true;
    for (uint32_t m : kMessages) ok = ms->AddNotification(static_cast<cIGZMessageTarget2*>(&TheTarget()), m) && ok;
    g_started = true;
    Log("history shadow: listening (record, queue length, SimNewMonth, PostCityShutdown)%s",
        ok ? "" : " [WARNING a notification was refused]");
}

void HistoryShadowStop() {
    if (!g_started) return;
    g_started = false;
    cIGZMessageServer2Ptr ms;
    if (ms) {
        for (uint32_t m : kMessages) ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(&TheTarget()), m);
    }
    TheTarget().Release();
    DropCore();
}

void HistoryShadowCityLoaded(cISC4City* city) {
    if (!g_started) return;
    OnCityLoaded(city, static_cast<int32_t>(SimDate(city)), nullptr);
}

bool HistoryShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                          int64_t data1, bool* finished) {
    if ((type != kMsgSimNewDay && type != kMsgSimNewYear) || target == nullptr || msg == nullptr ||
        finished == nullptr) {
        return false;
    }
    if (!g_started || g_core == nullptr) return false;
    // Cheap rejection for every other agent: the original registers by name (§3).
    if (agentName == nullptr || strcmp(agentName, kAgentName) != 0) return false;
    cFC4ISC4HistoryWarehouse* o = OriginalOf(CurrentCity());
    // The +4 sub-object (§1; QueryInterface does not answer it, so the
    // identity is checked by its address).
    if (o == nullptr || g_coreOf != o ||
        reinterpret_cast<char*>(target) != reinterpret_cast<char*>(static_cast<cIGZUnknown*>(o)) + 4) {
        return false;
    }
    *finished = target->DoMessage(msg);  // the original's step, unchanged
    AfterOriginalStep(type, *finished, static_cast<int32_t>(data1 != 0 ? data1 : PublishedJulian()));
    return true;
}

}  // namespace fc4

intptr_t HistoryShadowTest(FC4HistoryShadowTestInput* in) {
    if (in == nullptr || in->struct_size < sizeof(FC4HistoryShadowTestInput) || in->original == nullptr) return -1;
    fc4::g_testOriginal = reinterpret_cast<cFC4ISC4HistoryWarehouse*>(in->original);
    intptr_t r = -1;
    in->compared = 0;
    switch (in->event) {
        case 0:
            r = fc4::OnCityLoaded(nullptr, in->julian, &in->compared);
            break;
        case 1:
            r = fc4::OnMessage(in->type, in->data1, in->data2);
            break;
        case 2: {
            // The original's +4 target, as the simulator's delivery would see it.
            auto* target = reinterpret_cast<cIGZMessageTarget2*>(reinterpret_cast<char*>(in->original) + 4);
            if (fc4::g_core == nullptr || fc4::g_coreOf != fc4::g_testOriginal) break;
            struct Msg final : public cIGZMessage2 {
                uint32_t t;
                explicit Msg(uint32_t type) : t(type) {}
                bool QueryInterface(uint32_t, void**) override { return false; }
                uint32_t AddRef() override { return 1; }
                uint32_t Release() override { return 1; }
                bool Create(uint32_t, void**) const override { return false; }
                uint32_t GetType() const override { return t; }
                void SetType(uint32_t type) override { t = type; }
                bool operator==(cIGZMessage2 const&) const override { return false; }
                bool operator<(cIGZMessage2 const&) const override { return false; }
            } m(in->type);
            const bool fin = target->DoMessage(&m);
            in->finished = fin ? 1 : 0;
            r = fc4::AfterOriginalStep(in->type, fin, in->julian);
            break;
        }
        case 3:
            r = fc4::CompareNow("month", in->julian, &in->compared);
            break;
        case 4:
            fc4::DropCore();
            r = 0;
            break;
        default:
            break;
    }
    fc4::g_testOriginal = nullptr;
    return r;
}
