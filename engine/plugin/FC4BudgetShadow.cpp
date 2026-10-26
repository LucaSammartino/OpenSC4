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

#include "FC4BudgetShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"
#include "fc4.h"
#include "fc4_trace.h"

#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4DepartmentBudget.h"
#include "cISC4OrdinanceSimulator.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <climits>
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr int kTypes = 12;
// Tuning exemplar (budget spec §9), the one our budget's Init reads.
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0x0BFF6824;
constexpr long kDefaultLotLines = 6;
constexpr long kMaxLotLines = 64;
constexpr long kMaxLinesPerMonth = 48;  // type and lot lines together

// ---- settings and run state (main thread only) ---------------------------------------------------

bool g_enabled = false;
long g_lotLines = kDefaultLotLines;
int32_t g_settingsJulian = INT_MIN;
bool g_loggedOurBudget = false;

// Test seam (op 10).
bool g_testMode = false;
cISC4City* g_testCity = nullptr;
const FC4ShadowTestLot* g_testLots = nullptr;
uint32_t g_testLotCount = 0;

// Months compared since the game started; the last month's mismatch count.
long g_months = 0;
long g_clean = 0;
int g_lastMismatches = -1;

void ReadSettings() {
    g_enabled = SettingInt("budget_shadow", "FC4_BUDGET_SHADOW", 0) != 0;
    long n = SettingInt("budget_shadow_lots", "FC4_BUDGET_SHADOW_LOTS", kDefaultLotLines);
    if (n < 0) n = 0;
    if (n > kMaxLotLines) n = kMaxLotLines;
    g_lotLines = n;
}

cISC4City* CurrentCity() {
    if (g_testMode) return g_testCity;
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

// ---- one month ---------------------------------------------------------------------------------------

struct LotRec {
    fc4_lot_info info;
    fc4_tax_lot r;
    int32_t x, z;
    bool located;
};

struct Month {
    cISC4City* city;
    cFC4ISC4BudgetSimulator* budget;
    fc4_date date;
    uint32_t difficulty;
    // Before the original's settlement.
    int64_t funds0, ytdInc0, ytdExp0;
    float rates[kTypes];
    int64_t ordInc, ordExp, nbInc, nbExp, bonds;
    int64_t deptInc;
    bool deptIncKnown;
    bool tuningLoaded;
    fc4_tax_scan scan;
    bool scanned;
    const char* lotWalk;  // how ForEachLot walked the lots (BudgetLastLotWalk)
    std::vector<LotRec> lots;
    int32_t curX, curZ;
    bool curLocated;
    // After it.
    int64_t funds1, ytdInc1, ytdExp1;
    int64_t wa[kTypes];
    int64_t tax[kTypes];
    int64_t total;
    int64_t deptExp;
    bool deptExpKnown;
};

Month g_m;

// Every department of every group, each once: Σ GetTotalIncome() and Σ
// GetTotalExpenses() (the settlement's sums, budget §5.1). [abi] The game
// fills B vectors (allocators.md table (2)); without its allocator (unknown
// executable, tests) nothing is read and the sums are unknown.
bool SumDepartments(cFC4ISC4BudgetSimulator* budget, int64_t* income, int64_t* expense) {
    *income = 0;
    *expense = 0;
    if (budget == nullptr || !StlNodeAllocator::Available()) return false;
    GameVector<GameBudgetGroupInfo> groups;
    // [abi-ret] Slots 29 and 31 (interfaces.tsv eax-or-void) return no meaningful value (budget.md §6, the
    // closing rule): the SDK's bool is a leftover AL. Judge by the vectors only.
    budget->GetAllGroups(groups);
    std::vector<uint32_t> seen;
    for (GameBudgetGroupInfo& g : groups) {
        GameVector<cISC4DepartmentBudget*> depts;
        budget->GetDepartmentBudgetsInGroup(g.groupID, depts);  // [abi-ret] slot 31: no result (above)
        for (cISC4DepartmentBudget* raw : depts) {
            if (raw == nullptr) continue;
            cFC4ISC4DepartmentBudget* d = AsGame(raw);
            const uint32_t id = d->GetDepartmentID();
            bool dup = false;
            for (uint32_t s : seen) dup = dup || s == id;
            if (!dup) {
                seen.push_back(id);
                *income += d->GetTotalIncome();
                *expense += d->GetTotalExpenses();
            }
            raw->Release();  // one reference per element (budget spec §1)
        }
    }
    return true;
}

// The live lot scan: the location of the lot about to be visited.
void Tap(void*, cFC4ISC4Lot* lot) {
    int32_t x = -1, z = -1;
    g_m.curLocated = lot != nullptr && lot->GetLocation(x, z);
    g_m.curX = x;
    g_m.curZ = z;
}

// The test seam's lot source.
void TestForEachLot(void*, fc4_lot_visitor visit, void* ctx) {
    for (uint32_t i = 0; i < g_testLotCount && visit != nullptr; ++i) {
        g_m.curX = g_testLots[i].x;
        g_m.curZ = g_testLots[i].z;
        g_m.curLocated = true;
        visit(ctx, g_testLots[i].info);
    }
}

void OnTaxLot(void*, uint32_t, const fc4_lot_info* lot, const fc4_tax_lot* r) {
    if (lot == nullptr || r == nullptr) return;
    g_m.lots.push_back(LotRec{*lot, *r, g_m.curX, g_m.curZ, g_m.curLocated});
}

// Step 1, just before the original budget's DoMessage(SimNewMonth).
void BeforeStep(void*) {
    Month& m = g_m;
    cFC4ISC4BudgetSimulator* b = m.budget;
    m.funds0 = b->GetTotalFunds();
    m.ytdInc0 = b->GetYTDIncome();
    m.ytdExp0 = b->GetYTDExpenses();
    for (int t = 0; t < kTypes; ++t) m.rates[t] = b->GetTaxRate(static_cast<uint32_t>(t));
    m.bonds = b->GetTotalMonthlyBondPayments();
    cISC4OrdinanceSimulator* o = m.city->GetOrdinanceSimulator();
    m.ordInc = o ? o->GetOrdinanceMonthlyIncome() : 0;
    m.ordExp = o ? o->GetOrdinanceMonthlyExpense() : 0;
    cFC4ISC4NeighborsSimulator* n = AsGame(m.city->GetNeighborsSimulator());  // int64, edx:eax (NG11)
    m.nbInc = n ? n->GetTotalMonthlyNeighborDealIncome() : 0;
    m.nbExp = n ? n->GetTotalMonthlyNeighborDealExpense() : 0;
    int64_t unused = 0;
    m.deptIncKnown = SumDepartments(b, &m.deptInc, &unused);
    m.difficulty = static_cast<uint32_t>(m.city->GetDifficultyLevel());

    // Our ComputeTax on the same lots (read-only host table).
    props::Exemplar ex;
    uint64_t tuning = 0;
    if (!g_testMode && ex.Load(kTuningType, kTuningGroup, kTuningInstance)) tuning = TokenOf(ex.Holder());
    m.tuningLoaded = tuning != 0;
    fc4_budget_host host = *BudgetReadOnlyHost();
    if (g_testMode) {
        host.for_each_lot = &TestForEachLot;
    } else {
        SetBudgetLotTap(&Tap, nullptr);
    }
    m.lots.clear();
    m.scan = fc4_tax_scan{};
    m.scan.struct_size = sizeof(m.scan);
    m.curX = m.curZ = -1;
    m.curLocated = false;
    // [compat] the original's CS$$$ weight bug (§13.1) is what it computes.
    m.scanned = fc4_budget_shadow_tax(&host, tuning, false, m.difficulty, m.rates, &OnTaxLot, nullptr, &m.scan);
    SetBudgetLotTap(nullptr, nullptr);
    m.lotWalk = g_testMode ? "test lots" : BudgetLastLotWalk();
}

// Step 3, just after it.
void AfterStep(void*) {
    Month& m = g_m;
    cFC4ISC4BudgetSimulator* b = m.budget;
    m.funds1 = b->GetTotalFunds();
    m.ytdInc1 = b->GetYTDIncome();
    m.ytdExp1 = b->GetYTDExpenses();
    for (int t = 0; t < kTypes; ++t) {
        m.wa[t] = b->GetWeightedAssessedTaxValue(static_cast<uint32_t>(t));
        m.tax[t] = b->GetTaxIncomeOfType(t);
    }
    m.total = b->GetTaxIncome();
    int64_t unused = 0;
    m.deptExpKnown = SumDepartments(b, &unused, &m.deptExp);
}

// ---- the report --------------------------------------------------------------------------------------

void AppendF(char* out, size_t cap, const char* fmt, ...) {
    const size_t len = strlen(out);
    if (len + 1 >= cap) return;
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(out + len, cap - len, _TRUNCATE, fmt, ap);
    va_end(ap);
}

void AppendPair(char* out, size_t cap, const char* key, bool haveOrig, int64_t orig, bool haveOurs, int64_t ours) {
    AppendF(out, cap, "; %s orig/ours=", key);
    if (haveOrig) {
        AppendF(out, cap, "%lld", static_cast<long long>(orig));
    } else {
        AppendF(out, cap, "-");
    }
    if (haveOurs) {
        AppendF(out, cap, "/%lld", static_cast<long long>(ours));
    } else {
        AppendF(out, cap, "/-");
    }
    if (haveOrig && haveOurs && orig != ours) AppendF(out, cap, "!");
}

const char* StatusName(uint32_t s) {
    switch (s) {
        case FC4_TAX_LOT_COUNTED: return "counted";
        case FC4_TAX_LOT_NO_BUILDING: return "no-building";
        case FC4_TAX_LOT_NOT_LIVABLE: return "not-livable";
        case FC4_TAX_LOT_UNTAXED: return "untaxed";
        default: return "?";
    }
}

void LogLot(const char* prefix, int t, const LotRec& l, int64_t diff) {
    char where[32];
    if (l.located) {
        _snprintf_s(where, sizeof(where), _TRUNCATE, "(%d,%d)", l.x, l.z);
    } else {
        _snprintf_s(where, sizeof(where), _TRUNCATE, "(?)");
    }
    const int64_t mag = diff < 0 ? -diff : diff;
    Log("%s type %d lot %s %ux%u: %s purpose=%u wealth b%u/l%u value=%lld lv=%u assessed=%lld tax=%.3f%s", prefix, t,
        where, l.info.size_x, l.info.size_z, StatusName(l.r.status), l.info.purpose, l.info.building_wealth,
        l.info.lot_wealth, static_cast<long long>(l.info.building_value), static_cast<unsigned>(l.info.land_value),
        static_cast<long long>(l.r.assessed), static_cast<double>(l.r.tax),
        (mag != 0 && l.r.assessed == mag) ? " [assessed = |orig-ours|]" : "");
}

// The lots of group `t` that best explain a difference of `diff` (orig -
// ours) in its assessed sum, capped: first those whose assessed value is the
// difference, then the not-livable ones (the original might count them),
// then counted ones whose building and lot wealth differ, then the rest.
void LogTypeLots(const char* prefix, int t, int64_t diff, long* budgetLines) {
    const int64_t mag = diff < 0 ? -diff : diff;
    std::vector<bool> shown(g_m.lots.size(), false);
    long n = 0;
    for (int pass = 0; pass < 4; ++pass) {
        for (size_t i = 0; i < g_m.lots.size(); ++i) {
            if (n >= g_lotLines || *budgetLines <= 0) return;
            const LotRec& l = g_m.lots[i];
            if (shown[i] || l.r.group != t) continue;
            bool pick = false;
            switch (pass) {
                case 0: pick = mag != 0 && l.r.assessed == mag; break;
                case 1: pick = l.r.status == FC4_TAX_LOT_NOT_LIVABLE; break;
                case 2: pick = l.r.status == FC4_TAX_LOT_COUNTED && l.info.building_wealth != l.info.lot_wealth; break;
                default: pick = l.r.status == FC4_TAX_LOT_COUNTED; break;
            }
            if (!pick) continue;
            shown[i] = true;
            LogLot(prefix, t, l, diff);
            ++n;
            --*budgetLines;
        }
    }
}

// Writes the month's lines; returns the mismatch count.
int Report(bool beforeClean, bool afterClean) {
    const Month& m = g_m;
    char prefix[48];
    _snprintf_s(prefix, sizeof(prefix), _TRUNCATE, "budget shadow %04d-%02d-%02d", m.date.year, m.date.month,
                m.date.day_of_month);
    if (!beforeClean || !afterClean || !m.scanned) {
        Log("%s: INCOMPLETE (%s); not compared", prefix,
            !beforeClean ? "structured exception before the settlement"
                         : (!afterClean ? "structured exception after the settlement" : "our tax scan failed"));
        return -1;
    }
    const fc4_tax_scan& s = m.scan;
    bool mis[kTypes] = {};
    int typeMis = 0;
    char line[2048] = "";
    _snprintf_s(line, sizeof(line), _TRUNCATE, "%s: tax orig/ours=", prefix);
    for (int t = 0; t < kTypes; ++t) {
        mis[t] = m.wa[t] != s.weighted_assessed[t] || m.tax[t] != s.tax_income[t];
        if (mis[t]) ++typeMis;
        AppendF(line, sizeof(line), "%s%lld/%lld%s", t ? "," : "", static_cast<long long>(m.tax[t]),
                static_cast<long long>(s.tax_income[t]), mis[t] ? "!" : "");
    }
    // Year end (the settlement on January 1st): the year-to-date values are
    // zeroed after it (budget §5.1), so the month's income and expense are not
    // observable as deltas; the net (funds) still is.
    const bool yearEnd = m.date.month == 1;
    const int64_t origInc = m.ytdInc1 - m.ytdInc0;
    const int64_t origExp = m.ytdExp1 - m.ytdExp0;
    const int64_t oursInc = s.total_tax + m.ordInc + m.nbInc + m.deptInc;
    const int64_t oursExp = m.ordExp + m.nbExp + m.deptExp + m.bonds;
    const bool totalMis = m.total != s.total_tax;
    const bool incMis = !yearEnd && m.deptIncKnown && origInc != oursInc;
    const bool expMis = !yearEnd && m.deptExpKnown && origExp != oursExp;
    AppendPair(line, sizeof(line), "total", true, m.total, true, s.total_tax);
    AppendPair(line, sizeof(line), "income", !yearEnd, origInc, m.deptIncKnown, oursInc);
    AppendPair(line, sizeof(line), "expense", !yearEnd, origExp, m.deptExpKnown, oursExp);
    AppendPair(line, sizeof(line), "net", true, m.funds1 - m.funds0, m.deptIncKnown && m.deptExpKnown,
               oursInc - oursExp);
    const int mismatches = typeMis + (totalMis ? 1 : 0) + (incMis ? 1 : 0) + (expMis ? 1 : 0);
    fc4_trace_budget_expected(m.date.year, m.date.month, m.date.day_of_month,
                              m.wa, m.tax, m.total, origInc - m.ordInc - m.nbInc - m.deptInc,
                              !yearEnd && m.deptIncKnown, expMis, mismatches);
    ++g_months;
    if (mismatches == 0) ++g_clean;
    AppendF(line, sizeof(line), "; lots=%u; mismatches=%d", s.lots, mismatches);
    if (mismatches != 0) {
        // e.g. " [tax 3,4,6 total income]"
        AppendF(line, sizeof(line), " [");
        const char* sep = "";
        if (typeMis != 0) {
            AppendF(line, sizeof(line), "tax");
            const char* comma = " ";
            for (int t = 0; t < kTypes; ++t) {
                if (!mis[t]) continue;
                AppendF(line, sizeof(line), "%s%d", comma, t);
                comma = ",";
            }
            sep = " ";
        }
        if (totalMis) {
            AppendF(line, sizeof(line), "%stotal", sep);
            sep = " ";
        }
        if (incMis) {
            AppendF(line, sizeof(line), "%sincome", sep);
            sep = " ";
        }
        if (expMis) AppendF(line, sizeof(line), "%sexpense", sep);
        AppendF(line, sizeof(line), "]");
    }
    AppendF(line, sizeof(line), "; months=%ld clean=%ld", g_months, g_clean);
    if (yearEnd) AppendF(line, sizeof(line), "; year end: income/expense not observable");
    if (!m.deptIncKnown || !m.deptExpKnown) AppendF(line, sizeof(line), "; departments unreadable");
    if (!m.tuningLoaded) AppendF(line, sizeof(line), "; tuning: code defaults");
    const char* off = SwitchesOffList();
    if (strcmp(off, "-") != 0) AppendF(line, sizeof(line), "; switches_off=%s", off);
    Log("%s", line);

    long budgetLines = kMaxLinesPerMonth;
    for (int t = 0; t < kTypes && budgetLines > 0; ++t) {
        if (!mis[t]) continue;
        const int64_t diff = m.wa[t] - s.weighted_assessed[t];
        uint32_t notLivable = 0, gap = 0;
        int64_t notLivableSum = 0;
        for (const LotRec& l : m.lots) {
            if (l.r.group != t) continue;
            if (l.r.status == FC4_TAX_LOT_NOT_LIVABLE) {
                ++notLivable;
                notLivableSum += l.r.assessed;
            } else if (l.r.status == FC4_TAX_LOT_COUNTED && l.info.building_wealth != l.info.lot_wealth) {
                ++gap;
            }
        }
        Log("%s type %d: assessed orig/ours=%lld/%lld (orig-ours=%lld); tax orig/ours=%lld/%lld; rate=%.2f mult=%.3f; "
            "ours: counted=%u (building wealth != lot wealth: %u), not-livable=%u (assessed %lld)",
            prefix, t, static_cast<long long>(m.wa[t]), static_cast<long long>(s.weighted_assessed[t]),
            static_cast<long long>(diff), static_cast<long long>(m.tax[t]), static_cast<long long>(s.tax_income[t]),
            static_cast<double>(m.rates[t]), static_cast<double>(s.multiplier), s.counted[t], gap, notLivable,
            static_cast<long long>(notLivableSum));
        --budgetLines;
        LogTypeLots(prefix, t, diff, &budgetLines);
    }
    if (totalMis && typeMis == 0) {
        // BQ3: the total is a float32 running sum in lot order, never the sum of
        // the lines; over the master list it is the original's to the simoleon.
        if (m.lotWalk != nullptr && strcmp(m.lotWalk, "master list") == 0) {
            Log("%s: total differs with every type equal, over the master list (the original's lot order, BQ3: "
                "the total should be exact): a lot tax differs within its type's truncation",
                prefix);
        } else {
            Log("%s: total differs with every type equal: the lot order (lots by %s, not the master list: budget "
                "§5.2 [choice], the float32 running total; ±1 per 1000 lots allowed)",
                prefix, m.lotWalk != nullptr ? m.lotWalk : "?");
        }
    }
    return mismatches;
}

// True if `target` is the month agent of the city's original budget: its
// cIGZMessageTarget2 interface, or failing that (logged once) an agent that
// registered under the budget's agent name (budget spec §4.1 step 3).
bool IsOriginalBudgetAgent(cISC4City* city, cIGZMessageTarget2* target, const char* agentName,
                           cFC4ISC4BudgetSimulator** budgetOut) {
    *budgetOut = nullptr;
    if (city == nullptr || target == nullptr) return false;
    cISC4BudgetSimulator* raw = city->GetBudgetSimulator();  // borrowed
    if (raw == nullptr) return false;
    if (IsOurObject(raw)) {
        if (!g_loggedOurBudget) {
            g_loggedOurBudget = true;
            Log("budget shadow: inactive: the city's budget simulator is FreeCity4's own (the shadow compares "
                "against the ORIGINAL budget: disable = budget)");
        }
        return false;
    }
    void* agent = nullptr;
    const bool qi = raw->QueryInterface(kIID_cIGZMessageTarget2, &agent) && agent != nullptr;
    const bool same = qi && static_cast<cIGZMessageTarget2*>(agent) == target;
    if (qi) static_cast<cIGZMessageTarget2*>(agent)->Release();
    const bool byName = !same && agentName != nullptr && strcmp(agentName, "cSC4BudgetSimulator") == 0;
    if (!same && !byName) return false;
    static bool logged = false;
    if (!logged) {
        logged = true;
        Log("budget shadow: the original budget %p has month agent %p (matched by %s)", static_cast<void*>(raw),
            static_cast<void*>(target), same ? "its cIGZMessageTarget2 interface" : "agent name only");
    }
    *budgetOut = AsGame(raw);
    return true;
}

}  // namespace

void BudgetShadowStartup(bool simulatorReplaced, bool budgetReplaced) {
    ReadSettings();
    if (!g_enabled) return;  // off by default: not a word in the log
    const std::string src = [] {
        const char* s = "";
        Setting("budget_shadow", "FC4_BUDGET_SHADOW", &s);
        return std::string(s);
    }();
    if (simulatorReplaced && !budgetReplaced) {
        Log("budget shadow: ON (budget_shadow from %s): at each month-end, FreeCity4's tax scan next to the original "
            "budget's settlement, same lots, same moment; lot lines per mismatched type: %ld",
            src.c_str(), g_lotLines);
    } else {
        Log("budget shadow: requested (budget_shadow from %s) but INACTIVE: it needs FreeCity4's simulator and the "
            "original budget (disable = budget); here the simulator is %s and the budget is %s",
            src.c_str(), simulatorReplaced ? "ours" : "the original", budgetReplaced ? "ours" : "the original");
    }
}

bool BudgetShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                         int64_t data1, bool* finished) {
    if (type != kMsgSimNewMonth || target == nullptr || msg == nullptr || finished == nullptr) return false;
    const int32_t julian = static_cast<int32_t>(data1);
    if (!g_testMode && julian != g_settingsJulian) {
        g_settingsJulian = julian;
        ReadSettings();  // re-read once per month (FreeCity4.ini may change between city loads)
    }
    if (!g_enabled && !g_testMode) return false;
    cISC4City* city = CurrentCity();
    cFC4ISC4BudgetSimulator* budget = nullptr;
    if (!IsOriginalBudgetAgent(city, target, agentName, &budget)) return false;

    g_m.city = city;
    g_m.budget = budget;
    fc4_date_from_julian(julian, &g_m.date);
    const bool beforeClean = SehGuard("budget shadow (before the settlement)", &BeforeStep, nullptr);
    SetBudgetLotTap(nullptr, nullptr);  // also after a structured exception
    *finished = target->DoMessage(msg);  // the original budget's settlement
    const bool afterClean = SehGuard("budget shadow (after the settlement)", &AfterStep, nullptr);
    g_lastMismatches = Report(beforeClean, afterClean);
    g_m.lots.clear();
    g_m.lots.shrink_to_fit();
    g_m.city = nullptr;
    g_m.budget = nullptr;
    return true;  // delivered (compared or not)
}

}  // namespace fc4

intptr_t BudgetShadowTestMonth(const FC4ShadowTestInput* in) {
    if (in == nullptr) return -1;
    fc4::g_testMode = true;
    fc4::g_testCity = in->city;
    fc4::g_testLots = in->lots;
    fc4::g_testLotCount = in->lotCount;
    fc4::g_lastMismatches = -1;
    cFC4Message2Standard* msg = FC4MakeMessage(fc4::kMsgSimNewMonth, static_cast<intptr_t>(in->julian), 0, 0);
    bool finished = false;
    const bool taken = fc4::BudgetShadowDeliver(in->target, nullptr, msg, fc4::kMsgSimNewMonth, in->julian, &finished);
    msg->Release();
    fc4::g_testMode = false;
    fc4::g_testCity = nullptr;
    fc4::g_testLots = nullptr;
    fc4::g_testLotCount = 0;
    if (!taken) return -1;
    return fc4::g_lastMismatches < 0 ? -2 : fc4::g_lastMismatches;  // -2: delivered, not compared
}
