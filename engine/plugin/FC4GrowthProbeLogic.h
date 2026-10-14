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

// The growth probe's arithmetic and lines (tooling only; building-
// development.md §12). Header-only, no game calls and no Windows calls, so
// tests/growth_probe_test.cpp checks it directly:
//
//   GrowthTrunc / GrowthQualifies   §4.3's ad(D) = trunc(active demand) and the
//                                   §4.4 test ad(D) > 5
//   CountQualifyingDevelopers       the BDS developer map (slot 34, the STLport
//                                   map of docs/abi/allocators.md, walked with
//                                   FC4GameMap.h) and that test per developer
//   FormatGrowthDayLine             one line per game day
//   FormatGrowthSummary             the summary at a city or app shutdown
//
// Times are integers from QueryPerformanceCounter ticks (microseconds,
// tenths of a microsecond per attempt), so the lines need no float formatting.
#pragma once

#include "FC4GameMap.h"

#include <algorithm>
#include <cstdarg>
#include <cstdint>
#include <cstdio>
#include <string>
#include <vector>

namespace fc4 {

// ---- the qualifying test ------------------------------------------------------------------------

// `trunc(·)` of building-development.md (the CRT _ftol): int64 truncation
// toward zero, keeping the low 32 bits; NaN and values outside the int64
// range give the int64 indefinite, whose low half is 0.
inline int32_t GrowthTrunc(float f) {
    if (!(f > -9.2233720e18f && f < 9.2233720e18f)) return 0;
    return static_cast<int32_t>(static_cast<uint32_t>(static_cast<uint64_t>(static_cast<int64_t>(f))));
}

// building-development.md §4.4: a developer whose ad(D) = trunc(active
// demand) is > 5 (signed) gets one attempt per day.
inline bool GrowthQualifies(float activeDemand) { return GrowthTrunc(activeDemand) > 5; }

// More developers than this make the map read refused (13 ship; §1.5).
constexpr uint32_t kGrowthMaxDevelopers = 256;

struct GrowthQualifyingCount {
    GameMapStatus status = GameMapStatus::Ok;  // the map walk; the counts are 0 unless Ok
    uint32_t developers = 0;                   // entries in the map
    uint32_t qualifying = 0;                   // ad(D) > 5
    uint32_t noDemand = 0;                     // `active` could not read the developer's demand
};

// Counts the developers of the BDS developer map (developer type ->
// cISC4GrowthDeveloper*, the node's 4-byte value) whose ad(D) > 5. The map
// is walked first (read-only, every link checked); `active(value, &f)` then
// reads one developer's active demand (false: not readable, not counted as
// qualifying).
template <class Active>
GrowthQualifyingCount CountQualifyingDevelopers(const GameMapObject* map, Active&& active) {
    GrowthQualifyingCount c;
    std::vector<uint32_t> values;
    values.reserve(16);
    c.status = WalkGameMap(map, kGrowthMaxDevelopers, [&](const GameMapNode& node) { values.push_back(node.value); });
    if (c.status != GameMapStatus::Ok) return c;
    c.developers = static_cast<uint32_t>(values.size());
    for (const uint32_t v : values) {
        float a = 0.0f;
        if (!active(v, &a)) {
            ++c.noDemand;
            continue;
        }
        if (GrowthQualifies(a)) ++c.qualifying;
    }
    return c;
}

// ---- integer time arithmetic --------------------------------------------------------------------

// floor(a * b / c) without overflow while b * c < 2^64 (c > 0).
inline uint64_t GrowthMulDiv(uint64_t a, uint64_t b, uint64_t c) {
    if (c == 0) return 0;
    return (a / c) * b + ((a % c) * b) / c;
}

// QPC ticks as whole microseconds (floor).
inline uint64_t GrowthMicros(uint64_t ticks, uint64_t frequency) { return GrowthMulDiv(ticks, 1000000u, frequency); }

// QPC ticks spread over `n` items, in tenths of a microsecond per item
// (the total floored to tenths, then rounded half up per item).
inline uint64_t GrowthTenthsPer(uint64_t ticks, uint64_t frequency, uint64_t n) {
    if (n == 0) return 0;
    const uint64_t tenths = GrowthMulDiv(ticks, 10000000u, frequency);
    return (tenths + n / 2) / n;
}

// -1 when the qualifying count is unknown, else attempts < qualifying.
inline int GrowthCut(uint64_t attempts, int64_t qualifying) {
    if (qualifying < 0) return -1;
    return attempts < static_cast<uint64_t>(qualifying) ? 1 : 0;
}

// ---- the day line -------------------------------------------------------------------------------

struct GrowthDayValues {
    const char* date = "";     // yyyy-mm-dd of the posted SimNewDay
    uint64_t attempts = 0;     // Grow calls since the previous line
    int64_t qualifying = -1;   // < 0: not readable
    uint64_t ticks = 0;        // QPC ticks inside those calls
    uint64_t maxTicks = 0;     // the longest of them
    uint64_t cycles = 0;       // rdtsc cycles inside them
    uint64_t frequency = 0;    // QPC ticks per second
    int64_t lots = -1;         // < 0: not readable
    int64_t population = -1;   // < 0: not readable
};

namespace growth_detail {
inline void AppendF(std::string& out, const char* fmt, ...) {
    char buf[256];
    va_list ap;
    va_start(ap, fmt);
    const int n = std::vsnprintf(buf, sizeof(buf), fmt, ap);
    va_end(ap);
    if (n > 0) out.append(buf, static_cast<size_t>(n) < sizeof(buf) ? static_cast<size_t>(n) : sizeof(buf) - 1);
}

inline void AppendSigned(std::string& out, int64_t v) {
    if (v < 0) {
        out += "-";
    } else {
        AppendF(out, "%lld", static_cast<long long>(v));
    }
}

inline void AppendTenths(std::string& out, uint64_t tenths) {
    AppendF(out, "%llu.%llu", static_cast<unsigned long long>(tenths / 10), static_cast<unsigned long long>(tenths % 10));
}
}  // namespace growth_detail

// growth probe <date>: attempts=A qualifying=Q cut=0|1 elapsed_us=E
//   per_attempt_us=P lots=L population=N max_us=M cycles=C
// (Q, cut, L, N: `-` when not readable; P: `-` without an attempt.)
inline std::string FormatGrowthDayLine(const GrowthDayValues& v) {
    using growth_detail::AppendF;
    std::string s;
    AppendF(s, "growth probe %s: attempts=%llu qualifying=", v.date, static_cast<unsigned long long>(v.attempts));
    growth_detail::AppendSigned(s, v.qualifying);
    const int cut = GrowthCut(v.attempts, v.qualifying);
    AppendF(s, " cut=%s elapsed_us=%llu per_attempt_us=", cut < 0 ? "-" : (cut ? "1" : "0"),
            static_cast<unsigned long long>(GrowthMicros(v.ticks, v.frequency)));
    if (v.attempts == 0) {
        s += "-";
    } else {
        growth_detail::AppendTenths(s, GrowthTenthsPer(v.ticks, v.frequency, v.attempts));
    }
    s += " lots=";
    growth_detail::AppendSigned(s, v.lots);
    s += " population=";
    growth_detail::AppendSigned(s, v.population);
    AppendF(s, " max_us=%llu cycles=%llu", static_cast<unsigned long long>(GrowthMicros(v.maxTicks, v.frequency)),
            static_cast<unsigned long long>(v.cycles));
    return s;
}

// ---- the summary --------------------------------------------------------------------------------

struct GrowthDayRecord {
    uint32_t attempts = 0;
    int32_t qualifying = -1;  // < 0: unknown
    uint64_t ticks = 0;
    uint64_t cycles = 0;
};

struct GrowthDistribution {
    uint32_t n = 0;
    uint32_t min = 0;
    uint32_t max = 0;
    uint64_t medianTimesTwo = 0;  // the middle value, or the sum of the two middle values
};

inline GrowthDistribution GrowthDistributionOf(std::vector<uint32_t> v) {
    GrowthDistribution d;
    d.n = static_cast<uint32_t>(v.size());
    if (v.empty()) return d;
    std::sort(v.begin(), v.end());
    d.min = v.front();
    d.max = v.back();
    const size_t m = v.size() / 2;
    d.medianTimesTwo = v.size() % 2 != 0 ? 2ull * v[m] : static_cast<uint64_t>(v[m - 1]) + v[m];
    return d;
}

// "min/median/max" (a median between two values ends in .5), or "-".
inline std::string FormatGrowthDistribution(const GrowthDistribution& d) {
    if (d.n == 0) return "-";
    std::string s;
    growth_detail::AppendF(s, "%u/%llu%s/%u", d.min, static_cast<unsigned long long>(d.medianTimesTwo / 2),
                           d.medianTimesTwo % 2 != 0 ? ".5" : "", d.max);
    return s;
}

// growth probe summary (<why>): days=D; attempts min/median/max=a/m/b (total T);
//   cut days=K, attempts on cut days min/median/max=…|-; qualifying
//   min/median/max=…|- (unknown on U day(s)); mean_us_per_attempt=X.X|-
//   (elapsed_us=E); mean_cycles_per_attempt=C|-
inline std::string FormatGrowthSummary(const char* why, const std::vector<GrowthDayRecord>& days,
                                       uint64_t frequency) {
    using growth_detail::AppendF;
    std::vector<uint32_t> attempts, cutAttempts, qualifying;
    uint64_t total = 0, ticks = 0, cycles = 0;
    uint32_t unknown = 0;
    for (const GrowthDayRecord& d : days) {
        attempts.push_back(d.attempts);
        total += d.attempts;
        ticks += d.ticks;
        cycles += d.cycles;
        if (d.qualifying < 0) {
            ++unknown;
            continue;
        }
        qualifying.push_back(static_cast<uint32_t>(d.qualifying));
        if (GrowthCut(d.attempts, d.qualifying) == 1) cutAttempts.push_back(d.attempts);
    }
    std::string s;
    AppendF(s, "growth probe summary (%s): days=%u; attempts min/median/max=%s (total %llu); ", why,
            static_cast<unsigned>(days.size()), FormatGrowthDistribution(GrowthDistributionOf(attempts)).c_str(),
            static_cast<unsigned long long>(total));
    AppendF(s, "cut days=%u, attempts on cut days min/median/max=%s; ", static_cast<unsigned>(cutAttempts.size()),
            FormatGrowthDistribution(GrowthDistributionOf(cutAttempts)).c_str());
    AppendF(s, "qualifying min/median/max=%s (unknown on %u day(s)); mean_us_per_attempt=",
            FormatGrowthDistribution(GrowthDistributionOf(qualifying)).c_str(), unknown);
    if (total == 0) {
        s += "-";
    } else {
        growth_detail::AppendTenths(s, GrowthTenthsPer(ticks, frequency, total));
    }
    AppendF(s, " (elapsed_us=%llu); mean_cycles_per_attempt=", static_cast<unsigned long long>(GrowthMicros(ticks, frequency)));
    if (total == 0) {
        s += "-";
    } else {
        AppendF(s, "%llu", static_cast<unsigned long long>((cycles + total / 2) / total));
    }
    return s;
}

}  // namespace fc4
