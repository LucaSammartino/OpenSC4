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

#include "FC4Perf.h"

#include "FC4Config.h"
#include "FC4Log.h"

#include <windows.h>

#include <algorithm>
#include <atomic>
#include <cstdio>
#include <map>
#include <string>
#include <utility>
#include <vector>

namespace fc4 {
namespace perf {
namespace {

struct Counter {
    std::atomic<uint64_t> ticks{0};
    std::atomic<uint64_t> count{0};
};

Counter g_sections[kSectionCount];

struct AgentTotal {
    std::string name;
    uint32_t level = 0;
    uint64_t ticks = 0;
    uint64_t count = 0;
};

// Deliveries happen on the main thread only (our simulator's OnTick).
std::map<std::pair<const void*, uint32_t>, AgentTotal> g_agents;
uint64_t g_tickTicks = 0;
uint64_t g_tickCount = 0;

uint64_t Frequency() {
    static const uint64_t f = [] {
        LARGE_INTEGER q;
        QueryPerformanceFrequency(&q);
        return q.QuadPart > 0 ? static_cast<uint64_t>(q.QuadPart) : 1000ull;
    }();
    return f;
}

// Milliseconds with one decimal, as text.
std::string Ms(uint64_t ticks) {
    const uint64_t tenths = ticks * 10000ull / Frequency();
    char b[32];
    std::snprintf(b, sizeof(b), "%llu.%llu", static_cast<unsigned long long>(tenths / 10),
                  static_cast<unsigned long long>(tenths % 10));
    return b;
}

const char* SectionName(uint32_t s) {
    static const char* const names[] = {
        "bds:call",         "bds:publish",      "bds:day",           "bds:month",           "bds:idle",
        "bds:other-msg",    "bds:tract-Grow",   "bds:tract-Decline", "growth:call",         "growth:publish",
        "growth:lists",     "growth:tract-UpdatePopulation",        "growth:other-msg",
    };
    static char stateNames[21][24];
    if (s < sizeof(names) / sizeof(names[0])) return names[s];
    if (s >= kGrowthState0 && s <= kGrowthStateLast) {
        char* b = stateNames[s - kGrowthState0];
        if (b[0] == 0) std::snprintf(b, sizeof(stateNames[0]), "growth:state%u", s - kGrowthState0);
        return b;
    }
    if (s == kGridGet) return "grid:get";
    if (s == kGridSet) return "grid:set";
    return "?";
}

// Appends ` text` to `line`, flushing it as a continuation line when it gets long.
void Append(std::string& line, const std::string& head, const std::string& text) {
    if (line.size() + text.size() > 900) {
        Log("%s", line.c_str());
        line = head + " (cont.):";
    }
    line += " ";
    line += text;
}

}  // namespace

bool On() {
    static const bool on = [] {
        const bool v = SettingInt("perf_profile", "FC4_PERF_PROFILE", 0) != 0;
        if (v) {
            Log("perf: ON (perf_profile = 1; tooling, no behaviour change): agent deliveries, ticks and the "
                "development sections are timed; one 'perf month' line set per turbo month line");
        }
        return v;
    }();
    return on;
}

uint64_t Now() {
    LARGE_INTEGER q;
    QueryPerformanceCounter(&q);
    return static_cast<uint64_t>(q.QuadPart);
}

void Add(Section s, uint64_t ticks) {
    if (s >= kSectionCount) return;
    g_sections[s].ticks.fetch_add(ticks, std::memory_order_relaxed);
    g_sections[s].count.fetch_add(1, std::memory_order_relaxed);
}

void Count(Section s) {
    if (s >= kSectionCount) return;
    g_sections[s].count.fetch_add(1, std::memory_order_relaxed);
}

void AddDelivery(const void* agent, uint32_t level, const char* name, uint64_t ticks) {
    AgentTotal& a = g_agents[std::make_pair(agent, level)];
    if (a.name.empty()) {
        a.name = name != nullptr ? name : "?";
        a.level = level;
    }
    a.ticks += ticks;
    ++a.count;
}

void AddTick(uint64_t ticks) {
    g_tickTicks += ticks;
    ++g_tickCount;
}

void Report(const char* date, int32_t days, uint32_t realMs) {
    if (!On()) return;
    // Agents of the same name and level (several growth developers register
    // with one name) are summed.
    std::map<std::pair<std::string, uint32_t>, std::pair<uint64_t, uint64_t>> byName;
    uint64_t agentTicks = 0;
    for (const auto& kv : g_agents) {
        auto& t = byName[std::make_pair(kv.second.name, kv.second.level)];
        t.first += kv.second.ticks;
        t.second += kv.second.count;
        agentTicks += kv.second.ticks;
    }
    std::vector<std::pair<uint64_t, std::string>> rows;
    for (const auto& kv : byName) {
        char b[160];
        std::snprintf(b, sizeof(b), "%s/%u %sms (%llu)", kv.first.first.c_str(), kv.first.second,
                      Ms(kv.second.first).c_str(), static_cast<unsigned long long>(kv.second.second));
        rows.emplace_back(kv.second.first, b);
    }
    std::sort(rows.begin(), rows.end(), [](const auto& a, const auto& b) { return a.first > b.first; });

    const uint64_t realTicks = static_cast<uint64_t>(realMs) * Frequency() / 1000ull;
    const uint64_t perDay10 = days > 0 ? static_cast<uint64_t>(realMs) * 10ull / static_cast<uint64_t>(days) : 0;
    Log("perf month %s: real=%ums days=%d ms/day=%llu.%llu tick=%sms (%llu ticks) agents=%sms; outside ticks=%sms",
        date, realMs, days, static_cast<unsigned long long>(perDay10 / 10),
        static_cast<unsigned long long>(perDay10 % 10), Ms(g_tickTicks).c_str(),
        static_cast<unsigned long long>(g_tickCount), Ms(agentTicks).c_str(),
        Ms(realTicks > g_tickTicks ? realTicks - g_tickTicks : 0).c_str());

    const std::string agentsHead = std::string("perf month ") + date + " agents";
    std::string line = agentsHead + ":";
    size_t shown = 0;
    for (const auto& r : rows) {
        if (shown == 24 || r.first == 0) break;
        Append(line, agentsHead, r.second);
        ++shown;
    }
    if (rows.size() > shown) Append(line, agentsHead, "(+" + std::to_string(rows.size() - shown) + " more)");
    Log("%s", line.c_str());

    const std::string secHead = std::string("perf month ") + date + " sections";
    line = secHead + ":";
    bool any = false;
    for (uint32_t s = 0; s < kSectionCount; ++s) {
        const uint64_t n = g_sections[s].count.load(std::memory_order_relaxed);
        if (n == 0) continue;
        const uint64_t t = g_sections[s].ticks.load(std::memory_order_relaxed);
        char b[160];
        if (s == kGridGet || s == kGridSet) {
            std::snprintf(b, sizeof(b), "%s %llu", SectionName(s), static_cast<unsigned long long>(n));
        } else {
            std::snprintf(b, sizeof(b), "%s %sms (%llu)", SectionName(s), Ms(t).c_str(),
                          static_cast<unsigned long long>(n));
        }
        Append(line, secHead, b);
        any = true;
    }
    if (!any) line += " none";
    Log("%s", line.c_str());

    for (auto& c : g_sections) {
        c.ticks.store(0, std::memory_order_relaxed);
        c.count.store(0, std::memory_order_relaxed);
    }
    for (auto& kv : g_agents) {
        kv.second.ticks = 0;
        kv.second.count = 0;
    }
    g_tickTicks = 0;
    g_tickCount = 0;
}

}  // namespace perf
}  // namespace fc4
