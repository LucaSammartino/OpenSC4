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

#include "FC4Clock24Shadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4HistClock.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Watchdog.h"
#include "FC4WriteCapture.h"
#include "cFC4Clock24.h"
#include "fc4.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSystemService.h"
#include "cISC424HourClock.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4Simulator.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <deque>
#include <string>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;

// clock24.md §3.3 (0x44778230 + i) and the city's messages.
constexpr uint32_t kMsgFirst = 0x44778230;
constexpr uint32_t kMsgPause = 0x44778235;
constexpr uint32_t kMsgHiddenPause = 0x44778236;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgPostCityShutdown = 0x26D31EC3;
constexpr uint32_t kMessages[] = {kMsgFirst,     kMsgFirst + 1,   kMsgFirst + 2,       kMsgFirst + 3,
                                  kMsgFirst + 4, kMsgPause,       kMsgHiddenPause,     kMsgSimNewMonth,
                                  kMsgPostCityShutdown};
constexpr const char* kNames[] = {"HourChanged", "TimeTick",    "VisualHourChanged", "VisualTimeTick",
                                  "DayNightChanged", "PauseChanged", "HiddenPauseChanged"};

constexpr uint32_t kServiceID = 0x2B9D7C41;  // random, FreeCity4's
constexpr int32_t kServicePriority = -2100000;  // [choice] tooling; any value works
constexpr int kMaxDetails = 6;
constexpr uint64_t kStaleMs = 10000;  // a post of ours the original has not matched after 10 s

// ---- settings and run state (main thread only) ---------------------------------------------------

bool g_enabled = false;
bool g_replaced = false;
bool g_started = false;
bool g_serviceAdded = false;
bool g_loggedOurs = false;
bool g_loggedNone = false;
bool g_loggedNoRecord = false;

fc4_clock24* g_core = nullptr;
const void* g_coreOf = nullptr;
uint64_t g_now = 1;  // the shadow core's timer (microseconds)

struct Post {
    uint32_t type, d2, d3;
    uint64_t tick;  // GetTickCount64 when the shadow core posted it
};
std::deque<Post> g_ours;  // the shadow core's posts, not yet matched by the original's

// The test seam's original (FreeCity4's own clock is accepted there).
cFC4ISC424HourClock* g_testOriginal = nullptr;

// Since the last line.
struct Counts {
    uint32_t steps = 0, jumps = 0, modes = 0, pauses = 0, origPosts = 0, matched = 0, unmatched = 0, values = 0,
             valueDiffs = 0, samples = 0;
};
Counts g_counts;
uint32_t g_comparisons = 0, g_clean = 0;
int g_shown = 0;  // details shown since the last line

void ReadSettings() { g_enabled = SettingInt("clock24_shadow", "FC4_CLOCK24_SHADOW", 0) != 0; }

void Detail(const char* fmt, ...) {
    if (g_shown++ >= kMaxDetails) return;
    char buf[512];
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(buf, sizeof(buf), _TRUNCATE, fmt, ap);
    va_end(ap);
    Log("clock shadow (cont.) %s", buf);
}

const char* NameOf(uint32_t type) { return type >= kMsgFirst && type < kMsgFirst + 7 ? kNames[type - kMsgFirst] : "?"; }

// ---- the shadow core's host: read-only -----------------------------------------------------------

uint64_t HostNow(void*) { return g_now; }
void HostPost(void*, uint32_t type, uint32_t d2, uint32_t d3) { g_ours.push_back({type, d2, d3, GetTickCount64()}); }

fc4_clock24_host ShadowHost() {
    fc4_clock24_host h{};  // no framework, no cheat manager: those callbacks do nothing
    h.struct_size = sizeof(h);
    h.now_micros = &HostNow;
    h.post = &HostPost;
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

cFC4ISC424HourClock* OriginalOf(cISC4City* city) {
    if (g_testOriginal != nullptr) return g_testOriginal;
    if (city == nullptr) return nullptr;
    cISC424HourClock* c = city->Get24HourClock();  // slot 86, borrowed
    if (c == nullptr) {
        if (!g_loggedNone) {
            g_loggedNone = true;
            Log("clock shadow: the city has no 24-hour clock; nothing compared");
        }
        return nullptr;
    }
    if (IsOurObject(c)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("clock shadow: inactive: the city's 24-hour clock is FreeCity4's own (the shadow compares "
                "against the ORIGINAL: do not enable clock24)");
        }
        return nullptr;
    }
    return AsGame(c);
}

int32_t SimDate() {
    if (const FC4HistClockHooks* h = HistClockHooks()) return h->sim_date ? static_cast<int32_t>(h->sim_date(h->ctx)) : 0;
    cISC4City* city = CurrentCity();
    cISC4Simulator* s = city ? city->GetSimulator() : nullptr;  // borrowed
    cFC4ISC4Simulator* sim = AsGame(s);
    return sim ? static_cast<int32_t>(sim->GetSimDateNumber()) : static_cast<int32_t>(PublishedJulian());
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

std::string Hms(uint32_t s) {
    char buf[16];
    _snprintf_s(buf, sizeof(buf), _TRUNCATE, "%02u:%02u:%02u", s / 3600, (s / 60) % 60, s % 60);
    return buf;
}

uint32_t Bits(float f) {
    uint32_t b = 0;
    std::memcpy(&b, &f, 4);
    return b;
}

fc4_clock24_state Ours() {
    fc4_clock24_state st{};
    st.struct_size = sizeof(st);
    if (g_core != nullptr) fc4_clock24_get_state(g_core, &st);
    return st;
}

uint32_t OrigSeconds(cFC4ISC424HourClock* o) { return (o->GetHour() * 60 + o->GetMinute()) * 60 + o->GetSeconds(); }

void DropCore() {
    fc4_clock24_destroy(g_core);
    g_core = nullptr;
    g_coreOf = nullptr;
    g_ours.clear();
}

// ---- the mirror (one idle pass) --------------------------------------------------------------------------

// One step of exactly `delta` game seconds (61-1200): the elapsed time at
// magnification 1 is delta + 0.5 s, so the two float products (§4.3) land
// between delta and delta + 1 and truncate to delta. The magnification is
// put back after (the setters post nothing, §4.6).
bool MirrorStep(uint32_t delta) {
    float mag = 0, mn = 0, mx = 0;
    fc4_clock24_get_time_magnification(g_core, &mag);
    fc4_clock24_get_time_magnification_min_max(g_core, &mn, &mx);
    if (!(mn <= 1.0f && mx >= 1.0f)) return false;
    const uint32_t before = Ours().seconds;
    fc4_clock24_set_time_magnification(g_core, 1.0f);
    g_now += static_cast<uint64_t>(delta) * 1000000ull + 500000ull;
    fc4_clock24_on_idle(g_core, 0);
    fc4_clock24_set_time_magnification(g_core, mag);
    return Ours().seconds == (before + delta) % 86400;
}

// The original's state now, mirrored: the mode and a resume first, then the
// step (or a jump), then a pause (see the header).
void Mirror(cFC4ISC424HourClock* o) {
    ++g_counts.samples;
    fc4_clock24_state st = Ours();
    const int32_t mode = o->GetVisualTimeMode();
    if (mode != st.visual_mode) {
        fc4_clock24_set_visual_time_mode(g_core, mode);
        ++g_counts.modes;
    }
    const bool paused = o->IsPaused(), hidden = o->IsHiddenPaused();
    if (!paused && st.pause_count > 0) {
        fc4_clock24_pause(g_core, false);
        ++g_counts.pauses;
    }
    if (!hidden && st.hidden_pause_count > 0) {
        fc4_clock24_hidden_pause(g_core, false);
        ++g_counts.pauses;
    }
    const uint32_t now = OrigSeconds(o);
    st = Ours();
    if (now != st.seconds) {
        const uint32_t delta = (now + 86400 - st.seconds % 86400) % 86400;
        const bool canStep = delta > 60 && delta <= 1200 && st.pause_count <= 0 && st.hidden_pause_count <= 0;
        if (canStep && MirrorStep(delta)) {
            ++g_counts.steps;
        } else {
            // A setter or a cheat (or a step the shadow could not take): the
            // same time through the setters; SetHour posts as the original's.
            fc4_clock24_set_hour(g_core, now / 3600);
            fc4_clock24_set_minute(g_core, (now / 60) % 60);
            fc4_clock24_set_seconds(g_core, now % 60);
            ++g_counts.jumps;
            if (canStep) Detail("the step of %u s to %s could not be mirrored; set instead", delta, Hms(now).c_str());
        }
    }
    st = Ours();
    if (paused && st.pause_count <= 0) {
        fc4_clock24_pause(g_core, true);
        ++g_counts.pauses;
    }
    if (hidden && st.hidden_pause_count <= 0) {
        fc4_clock24_hidden_pause(g_core, true);
        ++g_counts.pauses;
    }
}

// The values the shadow core derives itself against the original's getters
// (§4.1, §8). A magnification the original changed (Stopwatch magnify) is
// counted once, then mirrored.
uint32_t CompareValues(cFC4ISC424HourClock* o) {
    uint32_t diffs = 0;
    auto check = [&diffs](const char* what, uint32_t orig, uint32_t ours) {
        ++g_counts.values;
        if (orig != ours) {
            ++diffs;
            Detail("%s: original 0x%08X (%u), ours 0x%08X (%u)", what, orig, orig, ours, ours);
        }
    };
    const fc4_clock24_state st = Ours();
    check("game seconds", OrigSeconds(o), st.seconds);
    check("GetVisualHour", o->GetVisualHour(), fc4_clock24_get_visual_hour(g_core));
    check("GetVisualMinute", o->GetVisualMinute(), fc4_clock24_get_visual_minute(g_core));
    check("GetVisualSeconds", o->GetVisualSeconds(), fc4_clock24_get_visual_seconds(g_core));
    float fo = 0, fu = 0;
    o->GetTimePercentage(&fo);
    fc4_clock24_get_time_percentage(g_core, &fu);
    check("GetTimePercentage", Bits(fo), Bits(fu));
    fo = fu = 0;
    o->GetVisualTimePercentage(&fo);
    fc4_clock24_get_visual_time_percentage(g_core, &fu);
    check("GetVisualTimePercentage", Bits(fo), Bits(fu));
    check("IsNight", o->IsNight() ? 1u : 0u, fc4_clock24_is_night(g_core) ? 1u : 0u);
    for (uint32_t i = 0; i < 3; ++i) check("GetEventHour", o->GetEventHour(i), fc4_clock24_get_event_hour(g_core, i));
    check("GetVisualTimeMode", static_cast<uint32_t>(o->GetVisualTimeMode()), static_cast<uint32_t>(st.visual_mode));
    check("IsPaused", o->IsPaused() ? 1u : 0u, fc4_clock24_is_paused(g_core) ? 1u : 0u);
    check("IsHiddenPaused", o->IsHiddenPaused() ? 1u : 0u, fc4_clock24_is_hidden_paused(g_core) ? 1u : 0u);
    float mo = 0, mu = 0, no = 0, xo = 0, nu = 0, xu = 0;
    const uint32_t mbo = o->GetTimeMagnification(mo);  // slot 20: the bits in EAX
    const uint32_t mbu = fc4_clock24_get_time_magnification(g_core, &mu);
    const uint32_t xbo = o->GetTimeMagnificationMinMax(no, xo);  // slot 22: the bits of max
    const uint32_t xbu = fc4_clock24_get_time_magnification_min_max(g_core, &nu, &xu);
    const uint32_t before = diffs;
    check("GetTimeMagnification (EAX)", mbo, mbu);
    check("GetTimeMagnification (out)", Bits(mo), Bits(mu));
    check("GetTimeMagnificationMinMax (EAX)", xbo, xbu);
    check("GetTimeMagnificationMinMax (min)", Bits(no), Bits(nu));
    if (diffs != before) {
        fc4_clock24_set_time_magnification_min_max(g_core, no, xo);
        fc4_clock24_set_time_magnification(g_core, mo);
        Detail("the magnification and its limits mirrored from the original");
    }
    g_counts.valueDiffs += diffs;
    return diffs;
}

void SampleStep(void* ctx) {
    cFC4ISC424HourClock* o = static_cast<cFC4ISC424HourClock*>(ctx);
    const uint32_t before = Ours().seconds;
    const uint32_t steps = g_counts.steps + g_counts.jumps + g_counts.modes + g_counts.pauses;
    Mirror(o);
    // The derived values after every change (each step of the original).
    if (Ours().seconds != before || g_counts.steps + g_counts.jumps + g_counts.modes + g_counts.pauses != steps) {
        CompareValues(o);
    }
}

// One sample of the original (an idle pass, a message, a line). False: not run.
bool Sample() {
    if (g_core == nullptr) return false;
    cFC4ISC424HourClock* o = OriginalOf(CurrentCity());
    if (o == nullptr || g_coreOf != o) return false;
    if (!SehGuard("clock shadow (sample)", &SampleStep, o)) {
        Log("clock shadow: WARNING structured exception in a sample; the shadow is off for this city");
        DropCore();
        return false;
    }
    return true;
}

// ---- the messages ------------------------------------------------------------------------------------------

bool IsOriginal(cFC4ISC424HourClock* o, void* void1, cIGZUnknown* unk) {
    const void* p = static_cast<cIGZUnknown*>(o);
    return void1 == p || unk == p;
}

// One of the original's posts (§3.3), in its order: the next of the shadow
// core's for the same step.
intptr_t OnClockMessage(uint32_t type, uint32_t d2, uint32_t d3) {
    if (g_core == nullptr) return -1;
    Sample();  // the step this message came from, if the idle pass has not seen it yet
    if (g_core == nullptr) return -1;
    ++g_counts.origPosts;
    auto same = [&](const Post& p) { return p.type == type && p.d2 == d2 && p.d3 == d3; };
    // A pause and resume between two idle passes: not seen by the getters.
    if ((type == kMsgPause || type == kMsgHiddenPause) && (g_ours.empty() || !same(g_ours.front()))) {
        const fc4_clock24_state st = Ours();
        const bool on = d2 != 0;
        const bool oursOn = (type == kMsgPause ? st.pause_count : st.hidden_pause_count) > 0;
        if (on != oursOn) {
            if (type == kMsgPause) {
                fc4_clock24_pause(g_core, on);
            } else {
                fc4_clock24_hidden_pause(g_core, on);
            }
            ++g_counts.pauses;
        }
    }
    if (!g_ours.empty() && same(g_ours.front())) {
        g_ours.pop_front();
        ++g_counts.matched;
        return 0;
    }
    // Further back: ours posted something the original did not.
    for (size_t i = 1; i < g_ours.size() && i < 8; ++i) {
        if (same(g_ours[i])) {
            for (size_t k = 0; k < i; ++k) {
                Detail("ours posted %s (%u, %u), the original did not (it posted %s (%u, %u) next)",
                       NameOf(g_ours[k].type), g_ours[k].d2, g_ours[k].d3, NameOf(type), d2, d3);
            }
            g_counts.unmatched += static_cast<uint32_t>(i);
            g_ours.erase(g_ours.begin(), g_ours.begin() + static_cast<std::ptrdiff_t>(i + 1));
            ++g_counts.matched;
            return static_cast<intptr_t>(i);
        }
    }
    ++g_counts.unmatched;
    if (g_ours.empty()) {
        Detail("the original posted %s (%u, %u); ours posted nothing", NameOf(type), d2, d3);
    } else {
        Detail("the original posted %s (%u, %u); ours posted %s (%u, %u) next", NameOf(type), d2, d3,
               NameOf(g_ours.front().type), g_ours.front().d2, g_ours.front().d3);
    }
    return 1;
}

// ---- the lines ------------------------------------------------------------------------------------------------

struct Line {
    cFC4ISC424HourClock* orig = nullptr;
    int32_t julian = 0;
    intptr_t result = -1;
    uint32_t compared = 0;
};

void MonthStep(void* ctx) {
    Line& l = *static_cast<Line*>(ctx);
    Mirror(l.orig);
    CompareValues(l.orig);
    // Posts of ours the original never matched.
    const uint64_t now = GetTickCount64();
    uint32_t stale = 0;
    while (!g_ours.empty() && now - g_ours.front().tick > kStaleMs) {
        Detail("ours posted %s (%u, %u); the original did not within %u s", NameOf(g_ours.front().type),
               g_ours.front().d2, g_ours.front().d3, static_cast<unsigned>(kStaleMs / 1000));
        g_ours.pop_front();
        ++stale;
    }
    g_counts.unmatched += stale;
    const fc4_clock24_state st = Ours();
    const uint32_t mismatches = g_counts.valueDiffs + g_counts.unmatched;
    ++g_comparisons;
    if (mismatches == 0) ++g_clean;
    Log("clock shadow %s month: time=%s visual=%s mode=%d pause=%d hidden=%d; samples=%u steps=%u jumps=%u "
        "modes=%u pauses=%u; posts original=%u matched=%u unmatched=%u pending=%u; values=%u differing=%u; "
        "mismatches=%u; comparisons=%u clean=%u",
        DateText(l.julian).c_str(), Hms(st.seconds).c_str(), Hms(st.visual_seconds).c_str(), st.visual_mode,
        st.pause_count, st.hidden_pause_count, g_counts.samples, g_counts.steps, g_counts.jumps, g_counts.modes,
        g_counts.pauses, g_counts.origPosts, g_counts.matched, g_counts.unmatched,
        static_cast<unsigned>(g_ours.size()), g_counts.values, g_counts.valueDiffs, mismatches, g_comparisons,
        g_clean);
    l.compared = g_counts.values + g_counts.origPosts;
    l.result = static_cast<intptr_t>(mismatches);
    g_counts = Counts{};
    g_shown = 0;
}

intptr_t CompareNow(int32_t julian, uint32_t* compared) {
    if (g_core == nullptr) return -1;
    cFC4ISC424HourClock* o = OriginalOf(CurrentCity());
    if (o == nullptr || g_coreOf != o) return -1;
    Line l;
    l.orig = o;
    l.julian = julian > 0 ? julian : SimDate();
    if (!SehGuard("clock shadow (month)", &MonthStep, &l)) {
        Log("clock shadow %s month: WARNING structured exception; comparison abandoned", DateText(l.julian).c_str());
        return -1;
    }
    if (compared != nullptr) *compared = l.compared;
    return l.result;
}

void LoadStep(void* ctx) {
    Line& l = *static_cast<Line*>(ctx);
    WriteCapture cap;
    bool answered = false;
    if (!cap.Capture(static_cast<cIGZUnknown*>(l.orig), &answered)) {
        if (!g_loggedNoRecord) {
            g_loggedNoRecord = true;
            Log("clock shadow: the original's record (its own Write) %s; nothing compared",
                answered ? "failed" : "cannot be read (no cIGZSerializable)");
        }
        return;
    }
    const std::vector<uint8_t> orig = cap.Bytes();
    DropCore();
    const fc4_clock24_host host = ShadowHost();  // copied by the core
    g_core = fc4_clock24_create(&host);
    if (g_core == nullptr) {
        Log("clock shadow: the shadow core could not be created; nothing compared");
        return;
    }
    g_coreOf = l.orig;
    g_counts = Counts{};
    g_shown = 0;
    MemIStream in(orig);
    const bool read = fc4_clock24_read(g_core, in.Get()) && in.Remaining() == 0;
    // §4.2: Read, then Init with the tuning (as the city did with the original).
    fc4_clock24_tuning tuning{};
    const bool haveTuning = FC4Clock24ReadTuning(tuning);
    fc4_clock24_init(g_core, haveTuning ? &tuning : nullptr);
    MemOStream out;
    fc4_clock24_write(g_core, out.Get());
    // The visual time is not saved (§7): if the original has stepped since
    // the load it already follows the game time; the shadow's is moved the
    // same way (ChangeVisualTime through mode 3, then 0) and its posts for
    // it are dropped (the original's went out before the shadow existed).
    bool aligned = false;
    const fc4_clock24_state st = Ours();
    if (l.orig->GetVisualTimeMode() == 0 && st.visual_mode == 0 &&
        l.orig->GetVisualHour() * 3600 + l.orig->GetVisualMinute() * 60 + l.orig->GetVisualSeconds() == st.seconds &&
        st.visual_seconds != st.seconds) {
        fc4_clock24_set_visual_time_mode(g_core, 3);
        fc4_clock24_set_visual_time_mode(g_core, 0);
        g_ours.clear();
        aligned = true;
    }
    Mirror(l.orig);
    const uint32_t valueDiffs = CompareValues(l.orig);
    const uint32_t diffs = (read ? 0u : 1u) + (orig == out.bytes ? 0u : 1u) + valueDiffs;
    uint16_t version = 0;
    if (orig.size() >= 2) std::memcpy(&version, orig.data(), 2);
    Log("clock shadow %s load: the original's record %u bytes (version %u; time %s, pause %d, magnification %.3f "
        "max %.3f, night %u -> %u) read by the shadow core and initialised with the tuning (%s): %s, written back "
        "%s; visual %s mode %d%s; values=%u differing=%u; mismatches=%u",
        DateText(l.julian).c_str(), static_cast<unsigned>(orig.size()), version, Hms(st.seconds).c_str(),
        st.pause_count, static_cast<double>(st.magnification), static_cast<double>(st.max_magnification),
        st.night_start, st.night_end, haveTuning ? "the exemplar" : "none", read ? "ok" : "FAILED",
        orig == out.bytes ? "identical" : "DIFFERENT", Hms(Ours().visual_seconds).c_str(), Ours().visual_mode,
        aligned ? " (aligned: the original had stepped since the load)" : "", g_counts.values, valueDiffs, diffs);
    if (orig != out.bytes) {
        std::string a, b;
        char hex[4];
        for (uint8_t x : orig) {
            _snprintf_s(hex, sizeof(hex), _TRUNCATE, "%02X ", x);
            a += hex;
        }
        for (uint8_t x : out.bytes) {
            _snprintf_s(hex, sizeof(hex), _TRUNCATE, "%02X ", x);
            b += hex;
        }
        Detail("record: original %s", a.c_str());
        Detail("record: ours     %s", b.c_str());
    }
    l.compared = g_counts.values + 1;
    l.result = static_cast<intptr_t>(diffs);
    g_counts = Counts{};
    g_shown = 0;
}

intptr_t OnCityLoaded(cISC4City* city, int32_t julian, uint32_t* compared) {
    cFC4ISC424HourClock* o = OriginalOf(city);
    if (o == nullptr) return -1;
    Line l;
    l.orig = o;
    l.julian = julian > 0 ? julian : SimDate();
    if (!SehGuard("clock shadow (load)", &LoadStep, &l)) {
        Log("clock shadow %s load: WARNING structured exception; the shadow is off for this city",
            DateText(l.julian).c_str());
        DropCore();
        return -1;
    }
    if (compared != nullptr) *compared = l.compared;
    return l.result;
}

// ---- the shadow's message target and idle service (static objects) -------------------------------------------

class Shadow final : public cIGZMessageTarget2, public cIGZSystemService {
public:
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid == kIID_cIGZMessageTarget2) {
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
        } else if (riid == GZIID_cIGZSystemService) {
            *ppvObj = static_cast<cIGZSystemService*>(this);
        } else if (riid == GZIID_cIGZUnknown) {
            *ppvObj = static_cast<cIGZUnknown*>(static_cast<cIGZMessageTarget2*>(this));
        } else {
            return false;
        }
        AddRef();
        return true;
    }
    // A static object: counted, never deleted.
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
        uint32_t d1 = 0, d2 = 0, d3 = 0;
        void* v1 = nullptr;
        cIGZUnknown* unk = nullptr;
        cIGZMessage2Standard* sm = nullptr;
        if (msg->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&sm)) && sm != nullptr) {
            FC4_ACQUIRED(sm, "QueryInterface");
            d1 = static_cast<uint32_t>(sm->GetData1());
            d2 = static_cast<uint32_t>(sm->GetData2());
            d3 = static_cast<uint32_t>(sm->GetData3());
            v1 = sm->GetVoid1();
            unk = sm->GetIGZUnknown();
            FC4_RELEASE(sm);
        }
        if (type == kMsgSimNewMonth) {
            CompareNow(d1 != 0 ? static_cast<int32_t>(d1) : static_cast<int32_t>(PublishedJulian()), nullptr);
            return true;
        }
        if (g_core == nullptr) return true;
        cFC4ISC424HourClock* o = OriginalOf(CurrentCity());
        if (o == nullptr || g_coreOf != o || !IsOriginal(o, v1, unk)) return true;
        OnClockMessage(type, d2, d3);
        return true;
    }

    // cIGZSystemService: only OnIdle does something.
    uint32_t GetServiceID() override { return serviceId_; }
    cIGZSystemService* SetServiceID(uint32_t id) override {
        serviceId_ = id;
        return this;
    }
    int32_t GetServicePriority() override { return kServicePriority; }
    bool IsServiceRunning() override { return running_; }
    cIGZSystemService* SetServiceRunning(bool running) override {
        running_ = running;
        return this;
    }
    bool Init() override { return true; }
    bool Shutdown() override { return true; }
    bool OnTick(uint32_t) override { return true; }
    bool OnIdle(uint32_t) override {
        if (g_core != nullptr) Sample();
        return true;
    }
    int32_t GetServiceTickPriority() override { return kServicePriority; }

private:
    volatile LONG refs_ = 0;
    uint32_t serviceId_ = kServiceID;
    bool running_ = false;
};

Shadow& TheShadow() {
    static Shadow s;
    return s;
}

void AddService() {
    if (g_serviceAdded || HistClockHooks() != nullptr) return;
    cIGZFrameWork* fw = RZGetFrameWork();
    if (fw == nullptr) return;
    cIGZSystemService* s = static_cast<cIGZSystemService*>(&TheShadow());
    const bool a = fw->AddSystemService(s);
    const bool b = fw->AddToOnIdle(s);
    g_serviceAdded = true;
    Log("clock shadow: idle service added (service %d, on-idle %d)", a ? 1 : 0, b ? 1 : 0);
}

void RemoveService() {
    if (!g_serviceAdded) return;
    g_serviceAdded = false;
    if (cIGZFrameWork* fw = RZGetFrameWork()) {
        cIGZSystemService* s = static_cast<cIGZSystemService*>(&TheShadow());
        fw->RemoveFromOnIdle(s);
        fw->RemoveSystemService(s);
    }
}

}  // namespace

void Clock24ShadowStartup(bool clockReplaced) {
    ReadSettings();
    g_replaced = clockReplaced;
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("clock24_shadow", "FC4_CLOCK24_SHADOW", &src);
    if (clockReplaced) {
        Log("clock shadow: requested (clock24_shadow from %s) but INACTIVE: it compares against the ORIGINAL "
            "24-hour clock, and clock24 is enabled (ours)",
            src);
        return;
    }
    Log("clock shadow: ON (clock24_shadow from %s): a shadow core (FreeCity4's clock) is read from the ORIGINAL "
        "clock's record at each city load (then Init with the tuning) and compared byte for byte; at every idle "
        "pass each step of the original (its game time moved by 61-1200 s) is taken by the shadow core too "
        "(a synthetic elapsed time: the original's timer has no read path), the mode and pause flags are "
        "mirrored, and the visual time, both percentages, IsNight, the event hours and the magnification are "
        "compared; the original's seven messages (0x44778230-36) must equal the shadow core's, in order",
        src);
}

void Clock24ShadowStart() {
    if (g_started || !g_enabled || g_replaced) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) {
        Log("clock shadow: no message server; not started");
        return;
    }
    TheShadow().AddRef();  // our own reference (a static object)
    bool ok = true;
    for (uint32_t m : kMessages) {
        ok = ms->AddNotification(static_cast<cIGZMessageTarget2*>(&TheShadow()), m) && ok;
    }
    g_started = true;
    Log("clock shadow: listening (the seven clock messages, SimNewMonth, PostCityShutdown)%s",
        ok ? "" : " [WARNING a notification was refused]");
}

void Clock24ShadowStop() {
    if (!g_started) return;
    g_started = false;
    RemoveService();
    cIGZMessageServer2Ptr ms;
    if (ms) {
        for (uint32_t m : kMessages) ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(&TheShadow()), m);
    }
    TheShadow().Release();
    DropCore();
}

void Clock24ShadowCityLoaded(cISC4City* city) {
    if (!g_started) return;
    if (OnCityLoaded(city, SimDate(), nullptr) >= 0) AddService();
}

}  // namespace fc4

intptr_t Clock24ShadowTest(FC4Clock24ShadowTestInput* in) {
    if (in == nullptr || in->struct_size < sizeof(FC4Clock24ShadowTestInput) || in->original == nullptr) return -1;
    fc4::g_testOriginal = reinterpret_cast<cFC4ISC424HourClock*>(in->original);
    intptr_t r = -1;
    in->compared = 0;
    switch (in->event) {
        case 0:
            r = fc4::OnCityLoaded(nullptr, in->julian, &in->compared);
            break;
        case 1:
            r = fc4::Sample() ? 0 : -1;
            break;
        case 2:
            r = fc4::OnClockMessage(in->type, in->data2, in->data3);
            break;
        case 3:
            r = fc4::CompareNow(in->julian, &in->compared);
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
