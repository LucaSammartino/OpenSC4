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

#include "FC4NeighborsShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Neighbors.h"

#include "SC4List.h"
#include "cIGZMessage2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4City.h"
#include "cISC4NeighborConnection.h"
#include "cISC4NeighborDeal.h"
#include "cISC4NeighborsSimulator.h"
#include "cISC4RegionalCity.h"

#include <algorithm>
#include <climits>
#include <cmath>
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <memory>
#include <string>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr const char* kAgentName = "cSC4NeighborsSimulator";  // neighbors.md §14.1
// fc4::Log formats into a fixed buffer: longer shadow lines are split.
constexpr size_t kLogChunk = 2800;
// The differences listed in a month line.
constexpr size_t kMaxListed = 24;
// [choice] The statistics are doubles computed in a different order: equal
// within a relative 1e-6 (NaN equals NaN).
constexpr double kRelTolerance = 1e-6;

const char* const kStatNames[5] = {"exported", "imported", "produced", "demanded", "costPerUnit"};
const char* const kCommodityNames[3] = {"power", "water", "garbage"};

// ---- settings and run state (main thread only) ---------------------------------------------------

bool g_enabled = false;
int32_t g_settingsJulian = INT_MIN;

// The original simulator's agent (its cIGZMessageTarget2), per simulator.
const void* g_agentSim = nullptr;
cIGZMessageTarget2* g_agent = nullptr;
bool g_loggedOurs = false;
bool g_loggedNoCore = false;

// One month's snapshot and the lists that keep its objects alive.
struct Delivery {
    cISC4NeighborsSimulator* sim = nullptr;
    cISC4City* city = nullptr;
    cISC4RegionalCity* current = nullptr;
    uint32_t julian = 0;
    std::string incomplete;  // why nothing is compared (empty: compared)
    // Before (the snapshot) and the objects it names (one reference each).
    std::unique_ptr<SC4List<cISC4RegionalCity*>> adjacent;
    std::vector<std::unique_ptr<SC4List<cISC4NeighborConnection*>>> connLists;
    std::unique_ptr<SC4List<cISC4NeighborDeal*>> dealList;
    std::vector<fc4_neighbors_shadow_city> cities;
    std::vector<cISC4RegionalCity*> cityObjects;  // as `cities`
    std::vector<fc4_neighbors_shadow_conn> conns;
    size_t ownConns = 0;  // the first ownConns of `conns` are the original's list
    std::vector<fc4_neighbor_deal_data> deals;
    fc4_neighbors* shadow = nullptr;
    bool monthRun = false;
};
Delivery g_d;

void ReadSettings() { g_enabled = SettingInt("neighbours_shadow", "FC4_NEIGHBOURS_SHADOW", 0) != 0; }

// The original neighbours simulator if `target` is its agent: its
// cIGZMessageTarget2 interface, or failing that (logged) an agent registered
// under its name.
cISC4NeighborsSimulator* OriginalFor(cISC4City* city, cIGZMessageTarget2* target, const char* agentName) {
    if (city == nullptr || target == nullptr) return nullptr;
    cISC4NeighborsSimulator* raw = city->GetNeighborsSimulator();  // borrowed
    if (raw == nullptr) return nullptr;
    if (IsOurObject(raw)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("neighbours shadow: inactive: the city's neighbours simulator is FreeCity4's own (the shadow compares "
                "against the ORIGINAL neighbours simulator: do not enable neighbors)");
        }
        return nullptr;
    }
    if (raw != g_agentSim) {
        void* agent = nullptr;
        const bool qi = raw->QueryInterface(kIID_cIGZMessageTarget2, &agent) && agent != nullptr;
        g_agent = nullptr;
        if (qi) {
            cIGZUnknown* a = static_cast<cIGZUnknown*>(agent);
            FC4_ACQUIRED(a, "QueryInterface");
            g_agent = static_cast<cIGZMessageTarget2*>(agent);
            FC4_RELEASE(a);  // the simulator keeps it alive; only its identity is kept
        }
        g_agentSim = raw;
        Log("neighbours shadow: the original neighbours simulator %p has agent %p (%s)", static_cast<void*>(raw),
            static_cast<void*>(g_agent), qi ? "its cIGZMessageTarget2 interface" : "no interface: matched by name");
    }
    const bool same = g_agent != nullptr && target == g_agent;
    const bool byName = g_agent == nullptr && agentName != nullptr && strcmp(agentName, kAgentName) == 0;
    return (same || byName) ? raw : nullptr;
}

// fc4::Log has a fixed buffer: a long line goes out in pieces, split at a
// space where there is one.
void LogLong(const std::string& line) {
    const char* p = line.c_str();
    const size_t len = line.size();
    size_t off = 0;
    bool first = true;
    while (first || off < len) {
        size_t n = std::min(kLogChunk, len - off);
        if (off + n < len) {
            size_t cut = n;
            while (cut > kLogChunk / 2 && p[off + cut - 1] != ' ') --cut;
            if (cut > kLogChunk / 2) n = cut;
        }
        if (first) {
            Log("%.*s", static_cast<int>(n), p + off);
        } else {
            Log("neighbours shadow (cont.) %.*s", static_cast<int>(n), p + off);
        }
        off += n;
        first = false;
    }
}

std::string DateText(uint32_t julian) {
    fc4_date d{};
    fc4_date_from_julian(static_cast<int32_t>(julian), &d);
    char b[24];
    snprintf(b, sizeof(b), "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
    return b;
}

// The connections of a list, appended to `out` with `city` as their holder.
void AddConns(SC4List<cISC4NeighborConnection*>& list, uint64_t city, std::vector<fc4_neighbors_shadow_conn>* out) {
    for (cISC4NeighborConnection* c : list) {
        if (c == nullptr) continue;
        fc4_neighbors_shadow_conn sc{};
        sc.city = city;
        sc.token = TokenOf(c);
        ReadNeighborConnection(c, &sc.state);
        out->push_back(sc);
    }
}

void BeforeStep(void*) {
    Delivery& d = g_d;
    cFC4ISC4NeighborsSimulator* orig = AsGame(d.sim);
    if (d.current == nullptr) {
        d.incomplete = "no current regional city";
        return;
    }
    // The cities: the current one, then the original's adjacent cities (§9.1).
    d.cities.emplace_back();
    ReadNeighborCity(d.current, &d.cities.back());
    d.cityObjects.push_back(d.current);
    d.adjacent = std::make_unique<SC4List<cISC4RegionalCity*>>();
    orig->GetAdjacentCities(*d.adjacent, true);  // slot 20: one reference each
    for (cISC4RegionalCity* c : *d.adjacent) {
        if (c == nullptr || c == d.current) continue;
        d.cities.emplace_back();
        ReadNeighborCity(c, &d.cities.back());
        d.cityObjects.push_back(c);
    }
    // The connections: the original's list (in order), then each other
    // city's (regional-city slot 21).
    d.connLists.push_back(std::make_unique<SC4List<cISC4NeighborConnection*>>());
    orig->GetAllConnections(*d.connLists.back());  // slot 16: one reference each
    AddConns(*d.connLists.back(), TokenOf(d.current), &d.conns);
    d.ownConns = d.conns.size();
    for (size_t i = 1; i < d.cityObjects.size(); ++i) {
        d.connLists.push_back(std::make_unique<SC4List<cISC4NeighborConnection*>>());
        AsGame(d.cityObjects[i])->GetAllConnections(*d.connLists.back());
        AddConns(*d.connLists.back(), TokenOf(d.cityObjects[i]), &d.conns);
    }
    // The deals (slot 28), each through cISC4NeighborDeal.
    d.dealList = std::make_unique<SC4List<cISC4NeighborDeal*>>();
    orig->FindDealsAll(*d.dealList);
    for (cISC4NeighborDeal* deal : *d.dealList) {
        d.deals.emplace_back();
        ReadNeighborDeal(deal, &d.deals.back());
    }

    fc4_neighbors_shadow_input in{};
    in.struct_size = sizeof(in);
    in.today = NeighborToday(d.city);
    if (in.today == 0) in.today = d.julian;
    d.julian = in.today;
    in.count_x = static_cast<int32_t>(d.city->CellCountX());
    in.count_z = static_cast<int32_t>(d.city->CellCountZ());
    in.current_city = TokenOf(d.current);
    in.city_object = TokenOf(d.city);
    ReadNeighborLandfill(d.city, &in.landfill_usage, &in.landfill_capacity);
    // [choice, spec question] Not readable: the original's ConnectionsReady
    // and UI-disable flags; taken as seen / clear (the state of a loaded
    // city in play).
    in.ready = 1;
    in.ui_disabled = 0;
    in.two_curves = SettingInt("neighbors_two_curves", "FC4_NEIGHBORS_TWO_CURVES", 0) != 0 ? 1 : 0;
    ReadNeighborFigures(d.city, &in.figures);
    props::Exemplar keep;
    fc4_props* tuning = LoadNeighborTuning(&keep);
    in.tuning = tuning;
    in.cities = d.cities.data();
    in.city_count = d.cities.size();
    in.conns = d.conns.empty() ? nullptr : d.conns.data();
    in.conn_count = d.conns.size();
    in.deals = d.deals.empty() ? nullptr : d.deals.data();
    in.deal_count = d.deals.size();
    d.shadow = fc4_neighbors_shadow_create(&in);
    if (tuning != nullptr) fc4_props_destroy(tuning);
    if (d.shadow == nullptr) {
        d.incomplete = "the shadow core could not be created";
        if (!g_loggedNoCore) {
            g_loggedNoCore = true;
            Log("neighbours shadow: fc4_neighbors_shadow_create failed; months pass through uncompared");
        }
        return;
    }
    // FreeCity4's month on the snapshot (it reads nothing of the game).
    fc4_neighbors_message m{};
    m.msg_type = kMsgSimNewMonth;
    m.data1 = d.julian;
    fc4_neighbors_do_message(d.shadow, &m);
    d.monthRun = true;
}

// ---- the comparison ------------------------------------------------------------------------------

struct Diffs {
    uint32_t count = 0;
    std::string listed;
    size_t shown = 0;

    void Add(const char* fmt, ...) {
        ++count;
        if (shown >= kMaxListed) return;
        ++shown;
        char b[256];
        va_list ap;
        va_start(ap, fmt);
        vsnprintf(b, sizeof(b), fmt, ap);
        va_end(ap);
        if (!listed.empty()) listed += ", ";
        listed += b;
    }
};

bool SameDouble(double a, double b) {
    if (std::isnan(a) || std::isnan(b)) return std::isnan(a) && std::isnan(b);
    const double scale = std::max(1.0, std::max(std::fabs(a), std::fabs(b)));
    return std::fabs(a - b) <= kRelTolerance * scale;
}

void CompareDeal(Diffs& df, size_t i, const fc4_neighbor_deal_data& o, const fc4_neighbor_deal_data& s) {
#define FC4_DEAL_FIELD(f, fmt)                                                                             \
    if (o.f != s.f) df.Add("deal %u " #f " %" fmt "/%" fmt, static_cast<unsigned>(i), o.f, s.f)
    FC4_DEAL_FIELD(deal_type, "u");
    if (o.other_x != s.other_x || o.other_z != s.other_z) {
        df.Add("deal %u otherCell (%d,%d)/(%d,%d)", static_cast<unsigned>(i), o.other_x, o.other_z, s.other_x,
               s.other_z);
    }
    if (o.city_x != s.city_x || o.city_z != s.city_z) {
        df.Add("deal %u cityCell (%d,%d)/(%d,%d)", static_cast<unsigned>(i), o.city_x, o.city_z, s.city_x, s.city_z);
    }
    if (o.flags != s.flags) df.Add("deal %u flags 0x%X/0x%X", static_cast<unsigned>(i), o.flags, s.flags);
    if (o.cost != s.cost) {
        df.Add("deal %u cost %lld/%lld", static_cast<unsigned>(i), static_cast<long long>(o.cost),
               static_cast<long long>(s.cost));
    }
    FC4_DEAL_FIELD(amount, "d");
    FC4_DEAL_FIELD(amount_used, "d");
    FC4_DEAL_FIELD(inc_amount, "d");
    FC4_DEAL_FIELD(inc_amount2, "d");
    FC4_DEAL_FIELD(last_amount, "d");
    FC4_DEAL_FIELD(last_inc_amount, "d");
    FC4_DEAL_FIELD(start_date, "u");
    FC4_DEAL_FIELD(expiration_date, "u");
    FC4_DEAL_FIELD(current_date, "u");
    FC4_DEAL_FIELD(suspension_date, "u");
#undef FC4_DEAL_FIELD
}

void CompareCity(Diffs& df, const fc4_neighbors_shadow_city& o, const fc4_neighbors_shadow_city& s, bool current) {
    const int32_t px = o.pos_x, pz = o.pos_z;
    const char* tag = current ? "R" : "city";
    if (!SameDouble(o.budget, s.budget)) df.Add("%s(%d,%d) budget %.6g/%.6g", tag, px, pz, o.budget, s.budget);
    if (!SameDouble(o.income, s.income)) df.Add("%s(%d,%d) income %.6g/%.6g", tag, px, pz, o.income, s.income);
    for (int st = 0; st < 5; ++st) {
        for (int k = 0; k < 3; ++k) {
            if (!SameDouble(o.stats[st][k], s.stats[st][k])) {
                df.Add("%s(%d,%d) %s %s %.6g/%.6g", tag, px, pz, kCommodityNames[k], kStatNames[st], o.stats[st][k],
                       s.stats[st][k]);
            }
        }
    }
}

void PushToken(void* ctx, uint64_t t) { static_cast<std::vector<uint64_t>*>(ctx)->push_back(t); }

bool SameState(const fc4_neighbor_connection_state& a, const fc4_neighbor_connection_state& b) {
    return a.conn_type == b.conn_type && a.x == b.x && a.z == b.z && a.other_x == b.other_x &&
           a.other_z == b.other_z && a.visited == b.visited && a.active == b.active;
}

const char* kTypeNames[13] = {"none",      "rail",   "road",  "highway",  "powerpole", "pipe",         "subway",
                              "lightrail", "avenue", "ferry", "dirtroad", "monorail",  "groundhighway"};
const char* TypeName(uint32_t t) { return t < 13 ? kTypeNames[t] : "?"; }

void AfterStep(void*) {
    Delivery& d = g_d;
    if (!d.incomplete.empty() || !d.monthRun) {
        if (d.incomplete.empty()) d.incomplete = "the shadow's month did not run";
        LogLong("neighbours shadow month to " + DateText(d.julian) + ": INCOMPLETE (" + d.incomplete +
                "); not compared");
        return;
    }
    cFC4ISC4NeighborsSimulator* orig = AsGame(d.sim);
    Diffs df;

    // Connections: the original's list after its month, against ours (the
    // same objects in order) and the state our month started from.
    SC4List<cISC4NeighborConnection*> after;
    orig->GetAllConnections(after);
    std::vector<fc4_neighbors_shadow_conn> afterConns;
    AddConns(after, TokenOf(d.current), &afterConns);
    std::vector<uint64_t> ours;
    fc4_neighbors_connections(d.shadow, 0, 0, 0, &PushToken, &ours);
    if (afterConns.size() != ours.size()) {
        df.Add("connections %u/%u", static_cast<unsigned>(afterConns.size()), static_cast<unsigned>(ours.size()));
    }
    uint32_t byType[13] = {};
    for (size_t i = 0; i < afterConns.size(); ++i) {
        const fc4_neighbors_shadow_conn& a = afterConns[i];
        if (a.state.conn_type < 13) ++byType[a.state.conn_type];
        if (i < ours.size() && ours[i] != a.token) {
            df.Add("connection %u object %p/%p", static_cast<unsigned>(i), reinterpret_cast<void*>(a.token),
                   reinterpret_cast<void*>(ours[i]));
            continue;
        }
        const fc4_neighbors_shadow_conn* before = nullptr;
        for (size_t j = 0; j < d.ownConns; ++j) {
            if (d.conns[j].token == a.token) {
                before = &d.conns[j];
                break;
            }
        }
        if (before == nullptr) {
            df.Add("connection %u %s (%d,%d) new in the original's month", static_cast<unsigned>(i),
                   TypeName(a.state.conn_type), a.state.x, a.state.z);
        } else if (!SameState(before->state, a.state)) {
            df.Add("connection %u %s (%d,%d) type/cell/other/flags %u (%d,%d) (%d,%d) v%u a%u -> %u (%d,%d) (%d,%d) "
                   "v%u a%u",
                   static_cast<unsigned>(i), TypeName(a.state.conn_type), a.state.x, a.state.z,
                   before->state.conn_type, before->state.x, before->state.z, before->state.other_x,
                   before->state.other_z, before->state.visited, before->state.active, a.state.conn_type, a.state.x,
                   a.state.z, a.state.other_x, a.state.other_z, a.state.visited, a.state.active);
        }
    }

    // Deals, index by index.
    SC4List<cISC4NeighborDeal*> afterDeals;
    orig->FindDealsAll(afterDeals);
    std::vector<fc4_neighbor_deal_data> od;
    for (cISC4NeighborDeal* deal : afterDeals) {
        od.emplace_back();
        ReadNeighborDeal(deal, &od.back());
    }
    const size_t sn = fc4_neighbors_deal_data(d.shadow, nullptr, 0);
    std::vector<fc4_neighbor_deal_data> sd(sn);
    if (sn != 0) fc4_neighbors_deal_data(d.shadow, sd.data(), sn);
    if (od.size() != sd.size()) {
        df.Add("deals %u/%u", static_cast<unsigned>(od.size()), static_cast<unsigned>(sd.size()));
    }
    uint32_t active = 0, proposed = 0, failing = 0;
    for (size_t i = 0; i < od.size(); ++i) {
        active += (od[i].flags & 0x2u) ? 1 : 0;
        proposed += (od[i].flags & 0x1u) ? 1 : 0;
        failing += (od[i].flags & 0x38u) ? 1 : 0;
        if (i < sd.size()) CompareDeal(df, i, od[i], sd[i]);
    }

    // Totals (slots 32 / 33: the full int64 in EDX:EAX, NG11).
    const int64_t oi = orig->GetTotalMonthlyNeighborDealIncome();
    const int64_t oe = orig->GetTotalMonthlyNeighborDealExpense();
    const int64_t si = fc4_neighbors_total_income(d.shadow);
    const int64_t se = fc4_neighbors_total_expense(d.shadow);
    if (oi != si) df.Add("income %lld/%lld", static_cast<long long>(oi), static_cast<long long>(si));
    if (oe != se) df.Add("expense %lld/%lld", static_cast<long long>(oe), static_cast<long long>(se));

    // The statistics of every city of the snapshot.
    fc4_neighbors_shadow_city rAfter{};
    for (size_t i = 0; i < d.cityObjects.size(); ++i) {
        fc4_neighbors_shadow_city o{};
        fc4_neighbors_shadow_city s{};
        ReadNeighborCity(d.cityObjects[i], &o);
        if (i == 0) rAfter = o;
        if (!fc4_neighbors_shadow_city_get(d.shadow, TokenOf(d.cityObjects[i]), &s)) {
            df.Add("city (%d,%d) missing in the shadow", o.pos_x, o.pos_z);
            continue;
        }
        CompareCity(df, o, s, i == 0);
    }

    fc4_neighbors_shadow_counters k{};
    fc4_neighbors_shadow_get_counters(d.shadow, &k);
    std::string types;
    for (uint32_t t = 0; t < 13; ++t) {
        if (byType[t] == 0) continue;
        char b[40];
        snprintf(b, sizeof(b), "%s%s=%u", types.empty() ? "" : " ", TypeName(t), byType[t]);
        types += b;
    }
    char head[1024];
    snprintf(head, sizeof(head),
             "neighbours shadow month to %s: connections=%u (%s) deals=%u (active %u proposed %u failing %u) "
             "income=%lld/%lld expense=%lld/%lld cities=%u R budget=%.0f income=%.0f power exp/imp=%.0f/%.0f water "
             "exp/imp=%.0f/%.0f garbage exp/imp=%.0f/%.0f; ours: posts=%u sends=%u retries=%u connectionMessages=%u "
             "setLocalDeals=%u updateImportExport=%u history=%u; mismatches=%u",
             DateText(d.julian).c_str(), static_cast<unsigned>(afterConns.size()), types.empty() ? "-" : types.c_str(),
             static_cast<unsigned>(od.size()), active, proposed, failing, static_cast<long long>(oi),
             static_cast<long long>(si), static_cast<long long>(oe), static_cast<long long>(se),
             static_cast<unsigned>(d.cityObjects.size()), rAfter.budget, rAfter.income, rAfter.stats[0][0],
             rAfter.stats[1][0], rAfter.stats[0][1], rAfter.stats[1][1], rAfter.stats[0][2], rAfter.stats[1][2],
             k.posts, k.sends, k.retries, k.connection_messages, k.set_local_deals, k.update_import_export, k.history,
             df.count);
    std::string line = head;
    if (df.count != 0) {
        line += "; differences (original/ours): " + df.listed;
        if (df.count > df.shown) line += ", ...";
    }
    LogLong(line);
}

void DropDelivery() {
    if (g_d.shadow != nullptr) fc4_neighbors_destroy(g_d.shadow);
    g_d = Delivery{};  // the lists release their references
}

}  // namespace

void NeighborsShadowStartup(bool simulatorReplaced, bool neighborsReplaced) {
    ReadSettings();
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("neighbours_shadow", "FC4_NEIGHBOURS_SHADOW", &src);
    if (simulatorReplaced && !neighborsReplaced) {
        Log("neighbours shadow: ON (neighbours_shadow from %s): at each SimNewMonth of the ORIGINAL neighbours "
            "simulator, FreeCity4's month on its connections and deals (read through their interfaces), the current "
            "regional city and its adjacent cities with their connections and statistics, the city's budget, "
            "utility and landfill figures, the date and the tuning, read just before; compared just after with its "
            "connection list (objects, type, cell, other-side cell, flags), each deal (state, cost, amount, the "
            "other amounts, the dates), its income and expense totals (slots 32/33) and every city's budget, income "
            "and commodity statistics (relative 1e-6); one line per month",
            src);
        Log("neighbours shadow: not readable read-only, hence assumed or not compared: the original's "
            "ConnectionsReady and UI-disable flags (taken as seen / clear), its dirty and busy flags (taken as "
            "clear), its price-curve state (rebuilt from the tuning by our Init), the regions' O lists (taken as "
            "what the stored import/export values hold beyond the original's last push), the status reports and "
            "connection messages it posted (counted on our side only), the history records it sent, the deal "
            "property holders, the marker props");
    } else {
        Log("neighbours shadow: requested (neighbours_shadow from %s) but INACTIVE: it needs FreeCity4's simulator "
            "and the original neighbours simulator (do not enable neighbors); here the simulator is %s and "
            "neighbours is %s",
            src, simulatorReplaced ? "ours" : "the original", neighborsReplaced ? "ours" : "the original");
    }
}

bool NeighborsShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                            int64_t data1, bool* finished) {
    if (type == kMsgSimNewDay) {
        const int32_t day = static_cast<int32_t>(data1);
        if (day != g_settingsJulian) {
            g_settingsJulian = day;
            ReadSettings();  // once per day (FreeCity4.ini may change between city loads)
        }
    }
    if (type != kMsgSimNewMonth || target == nullptr || msg == nullptr || finished == nullptr) return false;
    if (!g_enabled) return false;
    // Cheap rejection for every other agent: the original registers by name (§14.1).
    if (target != g_agent && (agentName == nullptr || strcmp(agentName, kAgentName) != 0)) return false;
    cISC4City* city = NeighborAppCity();
    cISC4NeighborsSimulator* sim = OriginalFor(city, target, agentName);
    if (sim == nullptr) return false;

    DropDelivery();
    g_d.sim = sim;
    g_d.city = city;
    g_d.current = NeighborAppRegionalCity();
    g_d.julian = static_cast<uint32_t>(data1);
    if (!SehGuard("neighbours shadow (before the original's month)", &BeforeStep, nullptr) && g_d.incomplete.empty()) {
        g_d.incomplete = "a fault while reading the original's inputs";
    }
    *finished = target->DoMessage(msg);  // the original neighbours simulator, unchanged
    if (!SehGuard("neighbours shadow (after the original's month)", &AfterStep, nullptr)) {
        Log("neighbours shadow month to %s: INCOMPLETE (a fault while comparing); not compared",
            DateText(g_d.julian).c_str());
    }
    DropDelivery();
    return true;  // delivered (compared or not)
}

}  // namespace fc4
