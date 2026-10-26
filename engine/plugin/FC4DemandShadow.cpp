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

#include "FC4DemandShadow.h"

#include "FC4Config.h"
#include "FC4BoundarySlot.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cFC4Message2Standard.h"

#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4Demand.h"
#include "cISC4DemandSimulator.h"
#include "cISC4HistoryWarehouse.h"
#include "cIGZDate.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <climits>
#include <cmath>
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimIdle = 0x6695681A;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr const char* kAgentName = "cSC4DemandSimulator";  // §10.1
constexpr uint32_t kExemplarType = 0x6534284A;
constexpr uint32_t kDemandGroup = 0xC7BB4816;  // §4.1
constexpr uint32_t kTotalIndex = 0x20000;      // §2
constexpr long kDefaultNodeLines = 8;
constexpr long kMaxNodeLines = 64;
constexpr long kMaxLinesPerUpdate = 48;

// The 12 R/C/I groups: the DIGEST's names and demand IDs (local node = id,
// total node = 0x20000 + id; §2, §8.1 order).
struct Group {
    uint32_t id;
    const char* name;
};
const Group kGroups[12] = {{0x1010, "R$"},   {0x1020, "R$$"},   {0x1030, "R$$$"}, {0x3110, "Cs$"},
                           {0x3120, "Cs$$"}, {0x3130, "Cs$$$"}, {0x3320, "Co$$"}, {0x3330, "Co$$$"},
                           {0x4100, "I-R"},  {0x4200, "I-D"},   {0x4300, "I-M"},  {0x4400, "I-HT"}};
// §6.1: the 24 census keys.
const uint32_t kCensusKeys[24] = {0x1011, 0x1021, 0x1031, 0x2111, 0x2112, 0x2113, 0x2114, 0x2121,
                                  0x2122, 0x2123, 0x2124, 0x2131, 0x2132, 0x2133, 0x2134, 0x3111,
                                  0x3121, 0x3131, 0x3321, 0x3331, 0x4101, 0x4201, 0x4301, 0x4401};
// §5 GetNeutralTaxRate: the census sum nodes R, C, I, passed in that order to
// fc4_demand_shadow_neutral_tax_rate, which sums f32(f32(I + C) + R) like the
// original (DQ9) through the simulator's own function.
const uint32_t kCensusSums[3] = {0x1001, 0x3001, 0x4001};

// ---- settings and run state (main thread only) ---------------------------------------------------

bool g_enabled = false;
long g_nodeLines = kDefaultNodeLines;
int32_t g_settingsJulian = INT_MIN;

// Test seam (op 11).
bool g_testMode = false;
const FC4DemandShadowTestInput* g_test = nullptr;

// FreeCity4's graph, built for one original demand simulator.
fc4_demand_shadow* g_graph = nullptr;
const void* g_graphFor = nullptr;
const void* g_graphFailedFor = nullptr;
std::vector<uint32_t> g_keys;
bool g_tuningFound = false;

// The original simulator's agent (its cIGZMessageTarget2), per simulator.
const void* g_agentSim = nullptr;
cIGZMessageTarget2* g_agent = nullptr;
bool g_loggedOurs = false;

// Updates compared since the game started.
long g_days = 0, g_clean = 0, g_inferred = 0;
long g_modChecks = 0, g_modClean = 0;
int g_lastResult = -1;

void ReadSettings() {
    g_enabled = SettingInt("demand_shadow", "FC4_DEMAND_SHADOW", 0) != 0;
    long n = SettingInt("demand_shadow_nodes", "FC4_DEMAND_SHADOW_NODES", kDefaultNodeLines);
    if (n < 0) n = 0;
    if (n > kMaxNodeLines) n = kMaxNodeLines;
    g_nodeLines = n;
}

cISC4City* CurrentCity() {
    if (g_testMode) return g_test->city;
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    cISC4City* city = sc4->GetCity();
    sc4->Release();
    return city;
}

// ---- the test seam's exemplars and getters (fc4_demand_host, user = the input) ---------------------

const FC4DemandShadowTestInput* TestOf(void* u) { return static_cast<const FC4DemandShadowTestInput*>(u); }

void TestForEachKey(void* u, fc4_key_visitor visit, void* ctx) {
    const FC4DemandShadowTestInput* t = TestOf(u);
    for (uint32_t i = 0; i < t->exemplarCount && visit != nullptr; ++i) {
        visit(ctx, kExemplarType, kDemandGroup, t->exemplarKeys[i]);
    }
}

fc4_props* TestExemplar(void* u, uint32_t type, uint32_t group, uint32_t instance) {
    const FC4DemandShadowTestInput* t = TestOf(u);
    if (type != kExemplarType || group != kDemandGroup) return nullptr;  // the tuning exemplar: missing
    fc4_props* p = fc4_props_create();
    if (p == nullptr) return nullptr;
    const uint32_t three = 3;  // ExemplarType: demand (§4.1)
    fc4_props_set_u32s(p, 0x10, &three, 1);
    for (uint32_t kind : {0x47BB3F30u, 0x47BB3F31u, 0x47BB3F32u, 0x47BB3F41u, 0x47BB3F50u}) {
        std::vector<uint32_t> v;
        for (uint32_t i = 0; i < t->linkCount; ++i) {
            const FC4DemandShadowTestLink& l = t->links[i];
            if (l.source != instance || l.kind != kind) continue;
            v.push_back(l.target);
            if (kind <= 0x47BB3F32u) v.push_back(l.percent);
        }
        if (!v.empty()) fc4_props_set_u32s(p, kind, v.data(), v.size());
    }
    return p;
}

uint32_t TestToday(void* u) { return static_cast<uint32_t>(TestOf(u)->julian); }
bool TestHistoryMax(void* u, uint32_t, uint32_t, uint32_t, float* max) {
    if (TestOf(u)->historyMaxRate < 0.0f || max == nullptr) return false;
    *max = TestOf(u)->historyMaxRate;
    return true;
}
float TestOrdinance(void* u, uint32_t, uint32_t) { return TestOf(u)->ordinanceFactor; }
float TestNation(void* u, uint32_t) { return TestOf(u)->nationModifier; }

struct HistoryOutput {
    uint32_t key, from, to, before, after;
    bool ok;
};
BoundarySlot g_historySlot;
std::vector<HistoryOutput> g_historyOutputs;
thread_local bool g_recordHistory = false;
bool __fastcall HistoryQuery(cISC4HistoryWarehouse* self, void*, uint32_t key,
                            const cIGZDate& from, const cIGZDate& to,
                            float& minimum, float& maximum, float& average) {
    using Fn = bool(__thiscall*)(cISC4HistoryWarehouse*, uint32_t, const cIGZDate&, const cIGZDate&,
                                float&, float&, float&);
    if (!g_recordHistory)
        return reinterpret_cast<Fn>(g_historySlot.original)(self, key, from, to, minimum, maximum, average);
    HistoryOutput output{};
    output.key = key;
    output.from = const_cast<cIGZDate&>(from).DayNumber();
    output.to = const_cast<cIGZDate&>(to).DayNumber();
    // demand.md §8.1.1: even a false return leaves consumed stack bits.
    // Copy bytes, never evaluate the indeterminate input as a C++ float.
    std::memcpy(&output.before, &maximum, sizeof(output.before));
    output.ok = reinterpret_cast<Fn>(g_historySlot.original)(self, key, from, to, minimum, maximum, average);
    std::memcpy(&output.after, &maximum, sizeof(output.after));
    g_historyOutputs.push_back(output);
    return output.ok;
}
bool CapturedHistoryMax(void* user, uint32_t key, uint32_t from, uint32_t to, float* maximum) {
    for (const auto& output : g_historyOutputs) {
        if (output.key != key || output.from != from || output.to != to) continue;
        std::memcpy(maximum, &output.after, sizeof(output.after));
        // The demand caller ignores the boolean. Supply its actual consumed
        // value, including false-return residue, to the independent formula.
        return true;
    }
    const auto host = DemandReadOnlyHost(static_cast<cISC4City*>(user));
    return host.history_max && host.history_max(host.user, key, from, to, maximum);
}

fc4_demand_host HostFor(cISC4City* city) {
    if (!g_testMode) {
        auto h = DemandReadOnlyHost(city);
        h.history_max = &CapturedHistoryMax;
        return h;
    }
    fc4_demand_host h{};
    h.struct_size = sizeof(h);
    h.user = const_cast<FC4DemandShadowTestInput*>(g_test);
    h.for_each_exemplar_key = &TestForEachKey;
    h.exemplar = &TestExemplar;
    h.today = &TestToday;
    h.history_max = &TestHistoryMax;
    h.ordinance_factor = &TestOrdinance;
    h.nation_economy_modifier = &TestNation;
    return h;
}

void PushKey(void* ctx, uint32_t key) { static_cast<std::vector<uint32_t>*>(ctx)->push_back(key); }

void DropGraph() {
    if (g_graph != nullptr) fc4_demand_shadow_destroy(g_graph);
    g_graph = nullptr;
    g_graphFor = nullptr;
    g_keys.clear();
}

// FreeCity4's graph from the demand exemplars (§4.2), once per original
// simulator (the exemplar enumeration is slow; the data does not change).
bool EnsureGraph(cISC4City* city, const void* sim) {
    if (g_graph != nullptr && g_graphFor == sim) return true;
    if (g_graphFailedFor == sim && !g_testMode) return false;
    DropGraph();
    const DWORD t0 = GetTickCount();
    const fc4_demand_host h = HostFor(city);
    g_graph = fc4_demand_shadow_create(&h);
    fc4_demand_shadow_info info{};
    info.struct_size = sizeof(info);
    if (g_graph == nullptr || !fc4_demand_shadow_get_info(g_graph, &info) || info.exemplars == 0) {
        Log("demand shadow: no demand graph (%s); not compared", g_graph == nullptr ? "creation failed"
                                                                                   : "no demand exemplars");
        DropGraph();
        g_graphFailedFor = sim;
        return false;
    }
    g_graphFor = sim;
    g_graphFailedFor = nullptr;
    fc4_demand_shadow_keys(g_graph, &PushKey, &g_keys);
    g_tuningFound = info.tuning_found != 0;
    Log("demand shadow: FreeCity4's graph for the original demand simulator %p: %u demand exemplars, %u keys, %u "
        "links, tuning %s (%lu ms)",
        sim, info.exemplars, info.keys, info.links, g_tuningFound ? "from its exemplar" : "MISSING: code defaults",
        static_cast<unsigned long>(GetTickCount() - t0));
    return true;
}

// The original demand simulator if `target` is its agent: its
// cIGZMessageTarget2 interface, or failing that (logged) an agent registered
// under its name (§10.1).
cFC4ISC4DemandSimulator* OriginalFor(cISC4City* city, cIGZMessageTarget2* target, const char* agentName) {
    if (city == nullptr || target == nullptr) return nullptr;
    cISC4DemandSimulator* raw = city->GetDemandSimulator();  // borrowed
    if (raw == nullptr) return nullptr;
    if (IsOurObject(raw)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("demand shadow: inactive: the city's demand simulator is FreeCity4's own (the shadow compares "
                "against the ORIGINAL demand simulator: do not enable demand)");
        }
        return nullptr;
    }
    if (raw != g_agentSim) {
        void* agent = nullptr;
        const bool qi = raw->QueryInterface(kIID_cIGZMessageTarget2, &agent) && agent != nullptr;
        g_agent = qi ? static_cast<cIGZMessageTarget2*>(agent) : nullptr;
        if (qi) g_agent->Release();  // the simulator keeps it alive; only its identity is kept
        g_agentSim = raw;
        Log("demand shadow: the original demand simulator %p has agent %p (%s)", static_cast<void*>(raw),
            static_cast<void*>(g_agent), qi ? "its cIGZMessageTarget2 interface" : "no interface: matched by name");
    }
    const bool same = g_agent != nullptr && target == g_agent;
    const bool byName = g_agent == nullptr && agentName != nullptr && strcmp(agentName, kAgentName) == 0;
    return (same || byName) ? AsGame(raw) : nullptr;
}

// ---- reading the original ---------------------------------------------------------------------------

fc4_demand_node_state ReadNode(uint32_t key, cFC4ISC4Demand* n) {
    fc4_demand_node_state s{};
    s.key = key;
    s.region_use = n->GetRegionUse();
    s.supply = n->QuerySupplyValue();
    s.demand = n->QueryDemandValue();
    s.new_supply = n->QueryNewSupply();
    s.new_demand = n->QueryNewDemand();
    s.active_demand = n->QueryActiveDemandValue();
    s.economy_modifier = n->QueryEconomyModifier();
    s.active_max = n->QueryActiveDemandMax();
    s.active_min = n->QueryActiveDemandMin();
    s.tax_modifier = n->GetTaxModifier();
    const SC4Percentage* cap = n->GetDemandCap();
    s.demand_cap = cap != nullptr ? cap->percentage : 0.0f;
    return s;
}

cFC4ISC4Demand* NodeOf(cFC4ISC4DemandSimulator* sim, uint32_t key) {
    cISC4Demand* d = sim->GetDemand(key, 0);  // borrowed (§5: no reference added)
    return d != nullptr ? AsGame(d) : nullptr;
}

// The 12 total nodes' modifiers and the census sums (§8.1 inputs and outputs).
struct Totals {
    bool have[12];
    float tax[12], econ[12];
    bool haveSum[3];
    float sum[3];  // supply of 0x1001 / 0x3001 / 0x4001
};

void ReadTotals(cFC4ISC4DemandSimulator* sim, Totals* t) {
    for (int i = 0; i < 12; ++i) {
        cFC4ISC4Demand* n = NodeOf(sim, kTotalIndex + kGroups[i].id);
        t->have[i] = n != nullptr;
        t->tax[i] = n ? n->GetTaxModifier() : 0.0f;
        t->econ[i] = n ? n->QueryEconomyModifier() : 0.0f;
    }
    for (int i = 0; i < 3; ++i) {
        cFC4ISC4Demand* n = NodeOf(sim, kCensusSums[i]);
        t->haveSum[i] = n != nullptr;
        t->sum[i] = n ? n->QuerySupplyValue() : 0.0f;
    }
}

uint32_t Bits(float f) {
    uint32_t u = 0;
    std::memcpy(&u, &f, sizeof(u));
    return u;
}

bool Same(float a, float b) { return Bits(a) == Bits(b); }

// Distance in representable floats (sign-magnitude to a monotonic integer).
long long Ulps(float a, float b) {
    int32_t ia = 0, ib = 0;
    std::memcpy(&ia, &a, sizeof(ia));
    std::memcpy(&ib, &b, sizeof(ib));
    const long long x = ia < 0 ? static_cast<long long>(INT32_MIN) - ia : ia;
    const long long y = ib < 0 ? static_cast<long long>(INT32_MIN) - ib : ib;
    return x > y ? x - y : y - x;
}

bool ModifiersChanged(const Totals& a, const Totals& b) {
    for (int i = 0; i < 12; ++i) {
        if (a.have[i] != b.have[i]) return true;
        if (a.have[i] && (!Same(a.tax[i], b.tax[i]) || !Same(a.econ[i], b.econ[i]))) return true;
    }
    return false;
}

// ---- one update ------------------------------------------------------------------------------------

struct Update {
    cISC4City* city;
    cFC4ISC4DemandSimulator* sim;
    fc4_date date;
    bool day;  // NewDay (else SimIdle)
    std::vector<uint32_t> keys;
    std::vector<cFC4ISC4Demand*> nodes;
    std::vector<fc4_demand_node_state> pre, post;
    Totals t0, t1;
    float neutralOrig;
    bool neutralRead;
};

Update g_u;

void BeforeStep(void*) {
    Update& u = g_u;
    u.keys.clear();
    u.nodes.clear();
    u.pre.clear();
    u.post.clear();
    ReadTotals(u.sim, &u.t0);
    if (!u.day) return;
    for (uint32_t key : g_keys) {
        cFC4ISC4Demand* n = NodeOf(u.sim, key);
        if (n == nullptr) continue;
        u.keys.push_back(key);
        u.nodes.push_back(n);
        u.pre.push_back(ReadNode(key, n));
    }
}

void AfterStep(void*) {
    Update& u = g_u;
    ReadTotals(u.sim, &u.t1);
    u.neutralRead = false;
    if (!u.day) return;
    for (size_t i = 0; i < u.nodes.size(); ++i) u.post.push_back(ReadNode(u.keys[i], u.nodes[i]));
    // §5: the original crashes without one of the census sum nodes; only ask
    // when all three exist.
    if (u.t1.haveSum[0] && u.t1.haveSum[1] && u.t1.haveSum[2]) {
        u.neutralOrig = u.sim->GetNeutralTaxRate();
        u.neutralRead = true;
    }
}

// ---- the report -------------------------------------------------------------------------------------

void AppendF(char* out, size_t cap, const char* fmt, ...) {
    const size_t len = strlen(out);
    if (len + 1 >= cap) return;
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(out + len, cap - len, _TRUNCATE, fmt, ap);
    va_end(ap);
}

// "a/b", "!" when the bits differ; "-" when absent.
void AppendPair(char* out, size_t cap, const char* sep, bool have, float orig, float ours) {
    if (!have) {
        AppendF(out, cap, "%s-", sep);
        return;
    }
    AppendF(out, cap, "%s%.9g/%.9g%s", sep, static_cast<double>(orig), static_cast<double>(ours),
            Same(orig, ours) ? "" : "!");
}

// "name orig/ours=a/b" plus "!(orig-ours=d, u ulp)" when the bits differ.
void AppendField(char* out, size_t cap, const char* name, float orig, float ours) {
    AppendF(out, cap, "%s orig/ours=%.9g/%.9g", name, static_cast<double>(orig), static_cast<double>(ours));
    if (!Same(orig, ours)) {
        AppendF(out, cap, "!(orig-ours=%.9g, %lld ulp)", static_cast<double>(orig) - static_cast<double>(ours),
                Ulps(orig, ours));
    }
}

void Prefix(char* out, size_t cap) {
    _snprintf_s(out, cap, _TRUNCATE, "demand shadow %04d-%02d-%02d", g_u.date.year, g_u.date.month,
                g_u.date.day_of_month);
}

int IndexOf(uint32_t key) {
    for (size_t i = 0; i < g_u.keys.size(); ++i) {
        if (g_u.keys[i] == key) return static_cast<int>(i);
    }
    return -1;
}

bool IsCensusKey(uint32_t key) {
    for (uint32_t k : kCensusKeys) {
        if (k == key) return true;
    }
    return false;
}

// FreeCity4's §8.1 next to the original's new modifiers; returns the
// mismatch count (fields), -1 if ours could not be computed.
int CompareModifiers(const char* prefix, const char* when, long* lines) {
    const Update& u = g_u;
    if (!EnsureGraph(u.city, u.sim)) return -1;
    fc4_demand_modifiers m{};
    m.struct_size = sizeof(m);
    const fc4_demand_host h = HostFor(u.city);
    if (!fc4_demand_shadow_modifiers(g_graph, &h, u.t0.sum[0], u.t0.sum[1], u.t0.sum[2], 0.0f, 0.0f, &m)) {
        Log("%s modifiers (%s): INCOMPLETE (our computation failed); not compared", prefix, when);
        return -1;
    }
    char line[2048] = "";
    _snprintf_s(line, sizeof(line), _TRUNCATE, "%s modifiers (%s): tax orig/ours=", prefix, when);
    int mis = 0;
    bool misAt[12] = {};
    for (int i = 0; i < 12; ++i) {
        AppendPair(line, sizeof(line), i ? "," : "", u.t1.have[i], u.t1.tax[i], m.node[i].tax_modifier);
        misAt[i] = u.t1.have[i] && !Same(u.t1.tax[i], m.node[i].tax_modifier);
        if (misAt[i]) ++mis;
    }
    AppendF(line, sizeof(line), "; econ orig/ours=");
    for (int i = 0; i < 12; ++i) {
        AppendPair(line, sizeof(line), i ? "," : "", u.t1.have[i], u.t1.econ[i], m.node[i].economy_modifier);
        if (u.t1.have[i] && !Same(u.t1.econ[i], m.node[i].economy_modifier)) {
            ++mis;
            misAt[i] = true;
        }
    }
    int noHistory = 0;
    for (int i = 0; i < 12; ++i) noHistory += (u.t1.have[i] && m.node[i].from_history == 0) ? 1 : 0;
    ++g_modChecks;
    if (mis == 0) ++g_modClean;
    AppendF(line, sizeof(line), "; neutral=%.9g (raw %.9g; census supply R/C/I=%.9g/%.9g/%.9g)",
            static_cast<double>(m.neutral), static_cast<double>(m.neutral_raw), static_cast<double>(u.t0.sum[0]),
            static_cast<double>(u.t0.sum[1]), static_cast<double>(u.t0.sum[2]));
    if (noHistory != 0) {
        AppendF(line, sizeof(line), "; no tax history in the window for %d type(s): max rate = neutral*10 [choice]",
                noHistory);
    }
    AppendF(line, sizeof(line), "; mismatches=%d; checks=%ld clean=%ld", mis, g_modChecks, g_modClean);
    if (!g_tuningFound) AppendF(line, sizeof(line), "; tuning: code defaults");
    Log("%s", line);
    for (int i = 0; i < 12 && *lines > 0; ++i) {
        if (!misAt[i]) continue;
        const fc4_demand_modifier& t = m.node[i];
        char l[1024] = "";
        _snprintf_s(l, sizeof(l), _TRUNCATE, "%s modifiers %s (0x%05X): ", prefix, kGroups[i].name,
                    kTotalIndex + kGroups[i].id);
        AppendField(l, sizeof(l), "tax", u.t1.tax[i], t.tax_modifier);
        AppendF(l, sizeof(l), "; ");
        AppendField(l, sizeof(l), "econ", u.t1.econ[i], t.economy_modifier);
        AppendF(l, sizeof(l),
                "; before orig tax/econ=%.9g/%.9g; ours: max rate=%.9g (%s) ratio=%.9g curve=%.9g ordinance=%.9g "
                "nation=%.9g today=%u",
                static_cast<double>(u.t0.tax[i]), static_cast<double>(u.t0.econ[i]),
                static_cast<double>(t.max_rate), t.from_history ? "history" : "no data: neutral*10",
                static_cast<double>(t.ratio), static_cast<double>(t.tax_curve),
                static_cast<double>(t.ordinance_factor), static_cast<double>(t.nation), m.today);
        Log("%s", l);
        --*lines;
    }
    return mis;
}

void LogNode(const char* prefix, size_t i, const fc4_demand_node_state& ours, bool inferred) {
    const fc4_demand_node_state& a = g_u.post[i];
    const fc4_demand_node_state& b = g_u.pre[i];
    char l[2048] = "";
    _snprintf_s(l, sizeof(l), _TRUNCATE, "%s node 0x%05X (ru %u): ", prefix, a.key, a.region_use);
    AppendField(l, sizeof(l), "supply", a.supply, ours.supply);
    AppendF(l, sizeof(l), "; ");
    AppendField(l, sizeof(l), "demand", a.demand, ours.demand);
    AppendF(l, sizeof(l), "; ");
    AppendField(l, sizeof(l), "pending supply", a.new_supply, ours.new_supply);
    AppendF(l, sizeof(l), "; ");
    AppendField(l, sizeof(l), "pending demand", a.new_demand, ours.new_demand);
    AppendF(l, sizeof(l), "; ");
    AppendField(l, sizeof(l), "active", a.active_demand, ours.active_demand);
    AppendF(l, sizeof(l), "; ");
    AppendField(l, sizeof(l), "cap", a.demand_cap, ours.demand_cap);
    AppendF(l, sizeof(l),
            " | before: supply=%.9g demand=%.9g pending=%.9g/%.9g active=%.9g cap=%.9g econ=%.9g tax=%.9g "
            "min/max=%.9g/%.9g%s",
            static_cast<double>(b.supply), static_cast<double>(b.demand), static_cast<double>(b.new_supply),
            static_cast<double>(b.new_demand), static_cast<double>(b.active_demand), static_cast<double>(b.demand_cap),
            static_cast<double>(a.economy_modifier), static_cast<double>(a.tax_modifier),
            static_cast<double>(b.active_min), static_cast<double>(b.active_max),
            inferred ? " [census node: pending inferred]" : "");
    Log("%s", l);
}

bool StateDiffers(const fc4_demand_node_state& a, const fc4_demand_node_state& b) {
    return !Same(a.supply, b.supply) || !Same(a.demand, b.demand) || !Same(a.new_supply, b.new_supply) ||
           !Same(a.new_demand, b.new_demand) || !Same(a.active_demand, b.active_demand) ||
           !Same(a.demand_cap, b.demand_cap);
}

bool RunOurs(const std::vector<fc4_demand_node_state>& seed, std::vector<fc4_demand_node_state>* ours,
             fc4_demand_shadow_stats* st) {
    *ours = seed;
    st->struct_size = sizeof(*st);
    return fc4_demand_shadow_propagate(g_graph, seed.data(), static_cast<uint32_t>(seed.size()), ours->data(), st);
}

// The NewDay comparison; returns the mismatch count, -1 if not compared.
int ReportDay(bool beforeClean, bool afterClean) {
    // SPEC-GAP: Unordered bonus/cap min/max selection for captured NaN tax inputs needs DM-GC29-2.
    Update& u = g_u;
    char prefix[48];
    Prefix(prefix, sizeof(prefix));
    long lines = kMaxLinesPerUpdate;
    if (!beforeClean || !afterClean || g_graph == nullptr || u.post.size() != u.pre.size()) {
        Log("%s: INCOMPLETE (%s); not compared", prefix,
            !beforeClean  ? "structured exception before the original's NewDay"
            : !afterClean ? "structured exception after the original's NewDay"
                          : "no graph");
        return -1;
    }
    const size_t n = u.pre.size();
    // Inputs the NewDay may set before its propagation: the modifiers
    // (census state 0, §8.1). Ours uses the original's new values.
    std::vector<fc4_demand_node_state> seed = u.pre;
    int modsSet = 0, otherInputs = 0;
    for (size_t i = 0; i < n; ++i) {
        const fc4_demand_node_state& a = u.pre[i];
        const fc4_demand_node_state& b = u.post[i];
        if (!Same(a.tax_modifier, b.tax_modifier) || !Same(a.economy_modifier, b.economy_modifier)) {
            ++modsSet;
            seed[i].tax_modifier = b.tax_modifier;
            seed[i].economy_modifier = b.economy_modifier;
        }
        if (!Same(a.active_min, b.active_min) || !Same(a.active_max, b.active_max) || a.region_use != b.region_use) {
            ++otherInputs;
        }
    }
    std::vector<fc4_demand_node_state> ours;
    fc4_demand_shadow_stats st{};
    if (!RunOurs(seed, &ours, &st)) {
        Log("%s: INCOMPLETE (our propagation failed); not compared", prefix);
        return -1;
    }
    // A census finishing inside the DoIdle(5) slice (§6.4): a census node's
    // totals differ although it only folds its own pending values.
    std::vector<uint32_t> moved;
    std::vector<bool> inferredAt(n, false);
    bool inferable = true;
    for (size_t i = 0; i < n; ++i) {
        if (!IsCensusKey(u.keys[i])) continue;
        if (Same(ours[i].supply, u.post[i].supply) && Same(ours[i].demand, u.post[i].demand)) continue;
        moved.push_back(u.keys[i]);
        const uint32_t ru = u.pre[i].region_use;
        if ((ru != 1 && ru != 2) || fc4_demand_shadow_is_fed(g_graph, u.keys[i])) inferable = false;
    }
    bool inferred = false;
    if (!moved.empty() && inferable) {
        for (uint32_t k : moved) {
            const int i = IndexOf(k);
            if (u.pre[i].region_use == 1) {
                seed[i].new_supply = u.post[i].supply - u.pre[i].supply;
            } else {
                seed[i].new_demand = u.post[i].demand - u.pre[i].demand;
            }
            inferredAt[i] = true;
        }
        if (!RunOurs(seed, &ours, &st)) {
            Log("%s: INCOMPLETE (our propagation failed); not compared", prefix);
            return -1;
        }
        inferred = true;
    }

    // Field by field, bitwise.
    int mismatches = 0;
    double maxAbs = 0.0;
    long long maxUlps = 0;
    std::vector<size_t> bad;
    for (size_t i = 0; i < n; ++i) {
        const fc4_demand_node_state& a = u.post[i];
        const fc4_demand_node_state& b = ours[i];
        const float pa[6] = {a.supply, a.demand, a.new_supply, a.new_demand, a.active_demand, a.demand_cap};
        const float pb[6] = {b.supply, b.demand, b.new_supply, b.new_demand, b.active_demand, b.demand_cap};
        bool any = false;
        for (int f = 0; f < 6; ++f) {
            if (Same(pa[f], pb[f])) continue;
            ++mismatches;
            any = true;
            const double d = std::fabs(static_cast<double>(pa[f]) - static_cast<double>(pb[f]));
            if (d > maxAbs) maxAbs = d;
            const long long ul = Ulps(pa[f], pb[f]);
            if (ul > maxUlps) maxUlps = ul;
        }
        if (any) bad.push_back(i);
    }
    // GetNeutralTaxRate (§5) from the original's census sums after the update.
    float neutralOurs = 0.0f;
    bool neutralMis = false;
    if (u.neutralRead) {
        neutralOurs = fc4_demand_shadow_neutral_tax_rate(g_graph, u.t1.sum[0], u.t1.sum[1], u.t1.sum[2]);
        neutralMis = !Same(u.neutralOrig, neutralOurs);
        if (neutralMis) ++mismatches;
    }

    char line[4096] = "";
    _snprintf_s(line, sizeof(line), _TRUNCATE, "%s: demand orig/ours=", prefix);
    for (int g = 0; g < 12; ++g) {
        const int i = IndexOf(kGroups[g].id);
        AppendPair(line, sizeof(line), g ? "," : "", i >= 0, i >= 0 ? u.post[i].demand : 0.0f,
                   i >= 0 ? ours[i].demand : 0.0f);
    }
    AppendF(line, sizeof(line), "; active orig/ours=");
    for (int g = 0; g < 12; ++g) {
        const int i = IndexOf(kTotalIndex + kGroups[g].id);
        AppendPair(line, sizeof(line), g ? "," : "", i >= 0, i >= 0 ? u.post[i].active_demand : 0.0f,
                   i >= 0 ? ours[i].active_demand : 0.0f);
    }
    AppendF(line, sizeof(line), "; neutral orig/ours=");
    AppendPair(line, sizeof(line), "", u.neutralRead, u.neutralOrig, neutralOurs);
    ++g_days;
    if (mismatches == 0) ++g_clean;
    if (inferred) ++g_inferred;
    AppendF(line, sizeof(line), "; nodes=%u; mismatches=%d", static_cast<unsigned>(n), mismatches);
    if (mismatches != 0) {
        AppendF(line, sizeof(line), " [");
        const size_t shown = bad.size() < 8 ? bad.size() : 8;
        for (size_t j = 0; j < shown; ++j) AppendF(line, sizeof(line), "%s0x%05X", j ? "," : "", u.keys[bad[j]]);
        if (bad.size() > shown) AppendF(line, sizeof(line), " +%u more", static_cast<unsigned>(bad.size() - shown));
        if (neutralMis) AppendF(line, sizeof(line), "%sneutral", bad.empty() ? "" : " ");
        AppendF(line, sizeof(line), "] max|orig-ours|=%.9g maxulps=%lld", maxAbs, maxUlps);
    }
    AppendF(line, sizeof(line), "; passes=%u transfers=%u", st.passes, st.transfers);
    if (!st.settled) AppendF(line, sizeof(line), " (NOT SETTLED)");
    if (st.missing_targets != 0) AppendF(line, sizeof(line), " missing-targets=%u", st.missing_targets);
    AppendF(line, sizeof(line), "; updates=%ld clean=%ld", g_days, g_clean);
    if (g_inferred != 0) AppendF(line, sizeof(line), " inferred=%ld", g_inferred);
    if (modsSet != 0) {
        AppendF(line, sizeof(line), "; modifiers set in this delivery on %d node(s): ours used the new values",
                modsSet);
    }
    if (inferred) {
        AppendF(line, sizeof(line),
                "; census applied in this delivery (%u census nodes): their pending values inferred from the "
                "result (an ulp difference here is not conclusive)",
                static_cast<unsigned>(moved.size()));
    } else if (!moved.empty()) {
        AppendF(line, sizeof(line), "; census nodes moved in this delivery but cannot be inferred (%u): compared as is",
                static_cast<unsigned>(moved.size()));
    }
    if (otherInputs != 0) {
        AppendF(line, sizeof(line), "; WARNING min/max/region use changed during the delivery on %d node(s)",
                otherInputs);
    }
    if (!g_tuningFound) AppendF(line, sizeof(line), "; tuning: code defaults");
    const char* off = SwitchesOffList();
    if (strcmp(off, "-") != 0) AppendF(line, sizeof(line), "; switches_off=%s", off);
    Log("%s", line);
    --lines;

    long nodeLines = g_nodeLines;
    for (size_t j = 0; j < bad.size() && nodeLines > 0 && lines > 0; ++j, --nodeLines, --lines) {
        LogNode(prefix, bad[j], ours[bad[j]], inferredAt[bad[j]]);
    }
    if (neutralMis && lines > 0) {
        Log("%s neutral: GetNeutralTaxRate orig/ours=%.9g/%.9g (orig-ours=%.9g, %lld ulp) from census supply "
            "R/C/I=%.9g/%.9g/%.9g",
            prefix, static_cast<double>(u.neutralOrig), static_cast<double>(neutralOurs),
            static_cast<double>(u.neutralOrig) - static_cast<double>(neutralOurs), Ulps(u.neutralOrig, neutralOurs),
            static_cast<double>(u.t1.sum[0]), static_cast<double>(u.t1.sum[1]), static_cast<double>(u.t1.sum[2]));
        --lines;
    }
    if (ModifiersChanged(u.t0, u.t1)) {
        const int mm = CompareModifiers(prefix, "NewDay", &lines);
        if (mm > 0) mismatches += mm;
    }
    return mismatches;
}

// The SimIdle comparison (modifiers only); -2 when nothing changed.
int ReportIdle(bool beforeClean, bool afterClean) {
    if (!beforeClean || !afterClean) {
        char prefix[48];
        Prefix(prefix, sizeof(prefix));
        Log("%s modifiers (SimIdle): INCOMPLETE (structured exception %s the original's SimIdle); not compared",
            prefix, beforeClean ? "after" : "before");
        return -1;
    }
    if (!ModifiersChanged(g_u.t0, g_u.t1)) return -2;
    char prefix[48];
    Prefix(prefix, sizeof(prefix));
    long lines = kMaxLinesPerUpdate;
    return CompareModifiers(prefix, "SimIdle", &lines);
}

}  // namespace

void DemandShadowStartup(bool simulatorReplaced, bool demandReplaced) {
    ReadSettings();
    if (!g_enabled) return;  // off by default: not a word in the log
    const std::string srcName = [] {
        const char* s = "";
        Setting("demand_shadow", "FC4_DEMAND_SHADOW", &s);
        return std::string(s);
    }();
    const char* src = srcName.c_str();
    if (simulatorReplaced && !demandReplaced) {
        Log("demand shadow: ON (demand_shadow from %s): at each NewDay, FreeCity4's propagation over the original "
            "demand simulator's node states read just before it, compared with its states just after; whenever the "
            "original's tax/economy modifiers change, FreeCity4's modifiers next to them (CO/I economy bonuses taken "
            "as 0: not readable); node lines per update: %ld",
            src, g_nodeLines);
    } else {
        Log("demand shadow: requested (demand_shadow from %s) but INACTIVE: it needs FreeCity4's simulator and the "
            "original demand simulator (do not enable demand); here the simulator is %s and demand is %s",
            src, simulatorReplaced ? "ours" : "the original", demandReplaced ? "ours" : "the original");
    }
}

bool DemandShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                         int64_t data1, bool* finished) {
    if ((type != kMsgSimNewDay && type != kMsgSimIdle) || target == nullptr || msg == nullptr || finished == nullptr) {
        return false;
    }
    if (!g_testMode && !g_enabled) {
        if (type != kMsgSimNewDay) return false;
        const int32_t julian = static_cast<int32_t>(data1);
        if (julian != g_settingsJulian) {
            g_settingsJulian = julian;
            ReadSettings();  // once per day (FreeCity4.ini may change between city loads)
        }
        if (!g_enabled) return false;
    }
    // Cheap rejection for every other agent: ours registers by name (AddAgent).
    if (!g_testMode && target != g_agent && (agentName == nullptr || strcmp(agentName, kAgentName) != 0)) return false;
    cISC4City* city = CurrentCity();
    cFC4ISC4DemandSimulator* sim = OriginalFor(city, target, agentName);
    if (sim == nullptr) return false;

    g_u.city = city;
    g_u.sim = sim;
    g_u.day = type == kMsgSimNewDay;
    fc4_date_from_julian(static_cast<int32_t>(data1), &g_u.date);
    if (g_u.day) {
        const int32_t julian = static_cast<int32_t>(data1);
        if (!g_testMode && julian != g_settingsJulian) {
            g_settingsJulian = julian;
            ReadSettings();
        }
        EnsureGraph(city, sim);
    }
    const bool beforeClean = SehGuard("demand shadow (before the original's update)", &BeforeStep, nullptr);
    g_historyOutputs.clear();
    if (!g_testMode) {
        const bool hooked = g_historySlot.Install(city->GetHistoryWarehouse(), 7,
                                                   reinterpret_cast<void*>(&HistoryQuery));
        if (!hooked) Log("demand boundary: WARNING history output capture unavailable");
    }
    g_recordHistory = true;
    *finished = target->DoMessage(msg);  // the original demand simulator, unchanged
    g_recordHistory = false;
    g_historySlot.Remove();
    for (const auto& output : g_historyOutputs) {
        if (!output.ok) Log("demand boundary: key=%08X window=%u..%u history=false max bits=%08X -> %08X",
                            output.key, output.from, output.to, output.before, output.after);
    }
    const bool afterClean = SehGuard("demand shadow (after the original's update)", &AfterStep, nullptr);
    g_lastResult = g_u.day ? ReportDay(beforeClean, afterClean) : ReportIdle(beforeClean, afterClean);
    g_u.city = nullptr;
    g_u.sim = nullptr;
    return true;  // delivered (compared or not)
}

}  // namespace fc4

intptr_t DemandShadowTestDeliver(const FC4DemandShadowTestInput* in) {
    if (in == nullptr) return -1;
    fc4::g_testMode = true;
    fc4::g_test = in;
    fc4::g_agentSim = nullptr;  // each call is a fresh "game"
    fc4::g_agent = nullptr;
    fc4::g_graphFailedFor = nullptr;
    fc4::DropGraph();
    fc4::g_lastResult = -1;
    const intptr_t data3 = in->msgType == fc4::kMsgSimIdle ? 5 : 0;
    cFC4Message2Standard* msg = FC4MakeMessage(in->msgType, static_cast<intptr_t>(in->julian), 0, data3);
    bool finished = false;
    const bool taken = fc4::DemandShadowDeliver(in->target, nullptr, msg, in->msgType, in->julian, &finished);
    msg->Release();
    fc4::DropGraph();
    fc4::g_agentSim = nullptr;
    fc4::g_agent = nullptr;
    fc4::g_testMode = false;
    fc4::g_test = nullptr;
    if (!taken) return -1;
    return fc4::g_lastResult == -1 ? -2 : fc4::g_lastResult;
}
