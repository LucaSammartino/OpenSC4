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

#include "cFC4Budget.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Properties.h"
#include "FC4Streams.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "FC4GameContainers.h"
#include "SC4String.h"
#include "SC4Vector.h"
#include "cIGZApp.h"
#include "cIGZCheatCodeManager.h"
#include "cIGZFrameWork.h"
#include "cIGZIStream.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZOStream.h"
#include "cIGZString.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4Simulator.h"
#include "cISCPropertyHolder.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <string>
#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cIGZCheatCodeManager = 0xA1085722;  // as used by the SDK examples

// Messages (spec §4.3, §10).
constexpr uint32_t kMsgSimBegin = 0x66956810;
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgCityEstablished = 0x26D31EC4;
constexpr uint32_t kMsgToggleBudgetDialog = 0x29222F02;
constexpr uint32_t kMsgIdealMonthlyFundingChanged = 0x08DAC3E8;
constexpr uint32_t kMsgFundingPercentageChanged = 0x08DAC3E7;
constexpr uint32_t kMsgCheatIssued = 0x230E27AC;
constexpr uint32_t kNotifications[] = {kMsgCityEstablished, kMsgToggleBudgetDialog, kMsgIdealMonthlyFundingChanged,
                                       kMsgFundingPercentageChanged};

// Cheats (spec §4.1).
constexpr uint32_t kCheatRiskyMoney = 0xCBF31943;
constexpr uint32_t kCheatWeaknessPays = 0x4AB87766;
constexpr uint32_t kCheatMoolah = 0x493F4FE6;

// Tuning exemplar (spec §9).
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0x0BFF6824;

// Agents (spec §4.1 step 3): one-shot, month, day; flags 2 (established cities).
constexpr uint32_t kAgentLevels[] = {0, 6, 4};
constexpr uint32_t kAgentFlags = 2;
const char kAgentName[] = "cSC4BudgetSimulator";

std::string ToStd(cIGZString const& s) {
    const char* p = s.ToChar();
    return p ? std::string(p, s.Strlen()) : std::string();
}

void AppendBytes(void* ctx, const uint8_t* data, size_t len) {
    static_cast<std::string*>(ctx)->append(reinterpret_cast<const char*>(data), len);
}

void PushToken(void* ctx, uint64_t token) { static_cast<std::vector<uint64_t>*>(ctx)->push_back(token); }

fc4_department* CoreOf(cISC4DepartmentBudget* d) {
    if (d == nullptr) return nullptr;
    cIFC4CoreObject* core = nullptr;
    if (!d->QueryInterface(kFC4_IID_CoreObject, reinterpret_cast<void**>(&core)) || core == nullptr) return nullptr;
    fc4_department* h = core->CoreClass() == kFC4_CLSID_cSC4DepartmentBudget
                            ? static_cast<fc4_department*>(core->CoreHandle())
                            : nullptr;
    core->Release();
    return h;
}

cISC4DepartmentBudget* DeptOf(uint64_t token) { return reinterpret_cast<cISC4DepartmentBudget*>(FromToken(token)); }
cISC4LineItem* LineOf(uint64_t token) { return reinterpret_cast<cISC4LineItem*>(FromToken(token)); }

bool EnvFlag(const char* name) {
    char v[8] = {};
    const DWORD n = GetEnvironmentVariableA(name, v, sizeof(v));
    return n > 0 && n < sizeof(v) && v[0] == '1';
}

}  // namespace

cIGZUnknown* CreateFC4BudgetSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4BudgetSimulator*>(new cFC4BudgetSimulator()));
}
cIGZUnknown* CreateFC4DepartmentBudget() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4DepartmentBudget*>(new cFC4DepartmentBudget()));
}
cIGZUnknown* CreateFC4LineItem() {
    return static_cast<cIGZUnknown*>(static_cast<cISC4LineItem*>(new cFC4LineItem()));
}

// ===================================================================================================
// Budget simulator
// ===================================================================================================

cFC4BudgetSimulator::cFC4BudgetSimulator() {
    core_ = fc4_budget_create(fc4::BudgetHost(), Token());
    if (core_ == nullptr) fc4::Log("cFC4BudgetSimulator: core creation failed");
    FC4_LOGV("cFC4BudgetSimulator(%p) created, core %p", static_cast<void*>(this), static_cast<void*>(core_));
}

cFC4BudgetSimulator::~cFC4BudgetSimulator() {
    fc4::Log("cFC4BudgetSimulator destroyed");
    fc4_budget_destroy(core_);
    core_ = nullptr;
}

uint64_t cFC4BudgetSimulator::Token() const {
    // The identity posted as void1 / IGZUnknown and used for references
    // [choice]: the cISC4BudgetSimulator interface pointer.
    return TokenOf(static_cast<const cFC4ISC4BudgetSimulator*>(this));
}

bool cFC4BudgetSimulator::QueryInterface(uint32_t riid, void** ppvObj) { return QueryInterfaceImpl(riid, ppvObj); }

bool cFC4BudgetSimulator::QueryInterfaceImpl(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_IID_cISC4BudgetSimulator:
            *ppvObj = static_cast<cFC4ISC4BudgetSimulator*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        case kFC4_IID_CoreObject:
            *ppvObj = static_cast<cIFC4CoreObject*>(this);
            break;
        case GZIID_cIGZUnknown:
            *ppvObj = static_cast<cIGZUnknown*>(static_cast<cFC4ISC4BudgetSimulator*>(this));
            break;
        default:
            *ppvObj = nullptr;
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4BudgetSimulator::AddRef() { return ++refCount_; }

uint32_t cFC4BudgetSimulator::Release() {
    if (refCount_ > 0) --refCount_;
    if (refCount_ == 0) {
        delete this;
        return 0;
    }
    return refCount_;
}

// ---- lifecycle (spec §4.1, §4.2) ----------------------------------------------------------------

bool cFC4BudgetSimulator::Init() {
    FC4_TRACE("BudgetSimulator::Init");
    if (core_ == nullptr) return false;
    if (fc4_budget_is_initialized(core_)) return true;
    // City load: re-read FreeCity4.ini (bisection switches) and the audit settings.
    fc4::ConfigReload("city load: budget Init");
    fc4::RefAuditReload();

    // Step 5: the city (and through it the land-value and ordinance simulators).
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app != nullptr && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) && sc4 != nullptr) {
        FC4_ACQUIRED(sc4, "QueryInterface");
        city_ = sc4->GetCity();
        FC4_RELEASE(sc4);
    }
    fc4::SetBudgetCity(city_);

    // Step 3: agents; step 4: notifications; step 6: cheats.
    RegisterAgents(true);
    cIGZMessageServer2Ptr ms;
    if (ms) {
        msgServer_ = ms;
        FC4_ADDREF(msgServer_);
        for (uint32_t id : kNotifications) msgServer_->AddNotification(static_cast<cIGZMessageTarget2*>(this), id);
    }
    if (fc4::On(fc4::Sw::Cheats)) {
        RegisterCheats();
    } else {
        fc4::NoteSkipped(fc4::Sw::Cheats, "cheat manager registration");
        fc4::Log("budget: cheats not registered: switch cheats is off");
    }

    // Steps 1, 2, 7, 8 (core): tuning from the exemplar, new-city setup.
    fc4::props::Exemplar ex;
    uint64_t tuning = 0;
    if (ex.Load(kTuningType, kTuningGroup, kTuningInstance)) {
        tuning = TokenOf(ex.Holder());
    } else {
        fc4::Log("budget: tuning exemplar unavailable; using the code defaults");
    }
    // [choice] FC4_BUDGET_FIX_CS3=1 enables the intended CS$$$ weight (spec §13.1); off by default.
    const bool fixCs3 = EnvFlag("FC4_BUDGET_FIX_CS3");
    const bool ok = fc4_budget_init(core_, tuning, fixCs3);

    // Step 9: the "budget" node of the game-data registry. TODO(spec §4.1 step 9):
    // the SDK does not describe how to create registry nodes with read/write
    // values and commands, so the UI scripts' "total_funds" / "deposit_funds"
    // are not published yet.
    fc4::Log("budget: game-data registry node 'budget' not published (stub)");

    fc4_budget_state st{};
    st.struct_size = sizeof(st);
    fc4_budget_get_state(core_, &st);
    fc4::Log("budget Init: ok=%d city=%p loaded=%d funds=%lld departments=%u bonds=%u rates=%.2f/%.2f/%.2f fixCS3=%d",
             ok ? 1 : 0, static_cast<void*>(city_), st.loaded_from_save, static_cast<long long>(st.funds),
             st.department_count, st.bond_count, st.tax_rate[0], st.tax_rate[3], st.tax_rate[8], fixCs3 ? 1 : 0);
    LogDepartments();
    return ok;
}

namespace {
void CollectGroupDepartments(void* ctx, const fc4_budget_group* g, const uint8_t*, size_t, const uint32_t* ids) {
    auto* out = static_cast<std::vector<std::pair<uint32_t, uint32_t>>*>(ctx);
    for (uint32_t i = 0; i < g->department_count; ++i) out->emplace_back(g->id, ids[i]);
}
}  // namespace

void cFC4BudgetSimulator::LogDepartments() {
    // Diagnostics: every department with its line items (after a load, line IDs
    // are only known once every record has been read).
    std::vector<std::pair<uint32_t, uint32_t>> depts;
    fc4_budget_get_all_groups(core_, &CollectGroupDepartments, &depts);
    for (const auto& gd : depts) {
        cISC4DepartmentBudget* d = DeptOf(fc4_budget_get_department(core_, gd.second));
        fc4_department* h = CoreOf(d);
        if (h == nullptr) {
            fc4::Log("budget: WARNING group %u lists department 0x%08X, which does not exist", gd.first, gd.second);
            continue;
        }
        std::vector<uint64_t> lines;
        fc4_department_get_all_line_items(h, &PushToken, &lines);
        std::string ids;
        bool zero = false;
        for (uint64_t t : lines) {
            const uint32_t id = LineOf(t)->GetID();
            zero = zero || id == 0;
            char buf[16];
            _snprintf_s(buf, sizeof(buf), _TRUNCATE, " %08X", id);
            ids += buf;
        }
        fc4::Log("budget: department 0x%08X (group %u) p=%.3f pMax=%.3f ideal=%lld current=%lld lines(%u):%s%s",
                 gd.second, gd.first, fc4_department_get_funding_percentage(h),
                 fc4_department_get_max_funding_percentage(h),
                 static_cast<long long>(fc4_department_get_ideal_monthly_funding(h)),
                 static_cast<long long>(fc4_department_get_total_expenses(h)), static_cast<unsigned>(lines.size()),
                 ids.c_str(), zero ? "  WARNING: a line item has ID 0" : "");
    }
}

bool cFC4BudgetSimulator::Shutdown() {
    FC4_TRACE("BudgetSimulator::Shutdown");
    if (core_ == nullptr || !fc4_budget_is_initialized(core_)) return true;
    AddRef();
    UnregisterCheats();
    if (msgServer_ != nullptr) {
        for (uint32_t id : kNotifications) msgServer_->RemoveNotification(static_cast<cIGZMessageTarget2*>(this), id);
        FC4_RELEASE(msgServer_);
        msgServer_ = nullptr;
    }
    fc4::LogBudgetCoreRefs("before budget Shutdown");
    fc4_budget_shutdown(core_);  // shuts down and releases every department
    RegisterAgents(false);
    city_ = nullptr;
    fc4::SetBudgetCity(nullptr);
    fc4::Log("budget Shutdown");
    fc4::LogBudgetCoreRefs("after budget Shutdown");
    fc4::LogSkipCounts("budget Shutdown");
    fc4::RefSummary("budget Shutdown");
    Release();
    return true;
}

void cFC4BudgetSimulator::RegisterAgents(bool add) {
    cFC4ISC4Simulator* sim = city_ ? fc4::AsGame(city_->GetSimulator()) : nullptr;
    if (sim == nullptr) {
        if (add) fc4::Log("budget: no simulator; agents not registered");
        return;
    }
    cIGZMessageTarget2* self = static_cast<cIGZMessageTarget2*>(this);
    if (add) {
        const cRZBaseString name(kAgentName);
        for (uint32_t level : kAgentLevels) {
            sim->AddAgent(self, level, name, kAgentFlags);
        }
        agentsRegistered_ = true;
    } else if (agentsRegistered_) {
        sim->RemoveAgentAllLevels(self);
        agentsRegistered_ = false;
    }
}

void cFC4BudgetSimulator::RegisterCheats() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) return;
    FC4_ACQUIRED(sc4, "QueryInterface");
    cIGZCheatCodeManager* cm = sc4->GetCheatCodeManager();
    if (cm != nullptr && cm->QueryInterface(kIID_cIGZCheatCodeManager, reinterpret_cast<void**>(&cheatMgr_))) {
        FC4_ACQUIRED(cheatMgr_, "QueryInterface");
        cheatMgr_->RegisterCheatCode(kCheatRiskyMoney, cRZBaseString("RiskyMoney"));
        cheatMgr_->RegisterCheatCode(kCheatWeaknessPays, cRZBaseString("WeaknessPays"));
        if (sc4->GetDebugFunctionalityEnabled()) {
            cheatMgr_->RegisterCheatCode(kCheatMoolah, cRZBaseString("Moolah"));
            moolahRegistered_ = true;
        }
        cheatMgr_->AddNotification2(static_cast<cIGZMessageTarget2*>(this), 0);
    }
    FC4_RELEASE(sc4);
}

void cFC4BudgetSimulator::UnregisterCheats() {
    if (cheatMgr_ == nullptr) return;
    cheatMgr_->UnregisterCheatCode(kCheatRiskyMoney);
    cheatMgr_->UnregisterCheatCode(kCheatWeaknessPays);
    if (moolahRegistered_) cheatMgr_->UnregisterCheatCode(kCheatMoolah);
    moolahRegistered_ = false;
    cheatMgr_->RemoveNotification2(static_cast<cIGZMessageTarget2*>(this), 0);
    FC4_RELEASE(cheatMgr_);
    cheatMgr_ = nullptr;
}

// ---- messages (spec §4.3) ----------------------------------------------------------------------------

bool cFC4BudgetSimulator::DoMessage(cIGZMessage2* pMessage) {
    FC4_TRACE("BudgetSimulator::DoMessage");
    if (pMessage == nullptr || core_ == nullptr) return true;
    const uint32_t type = pMessage->GetType();
    cIGZMessage2Standard* m = nullptr;
    if (pMessage->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m))) {
        FC4_ACQUIRED(m, "QueryInterface");
    }
    // data1 travels as a 32-bit slot; the panel IDs are unsigned (zero-extend).
    const int64_t data1 = m ? static_cast<int64_t>(static_cast<uint32_t>(m->GetData1())) : 0;
    bool result = true;

    if (type == kMsgCheatIssued) {
        const uint32_t id = static_cast<uint32_t>(data1);
        if (id == kCheatRiskyMoney || id == kCheatWeaknessPays || id == kCheatMoolah) {
            const cIGZString* text = m ? static_cast<const cIGZString*>(m->GetVoid2()) : nullptr;
            const std::string s = text ? ToStd(*text) : std::string();
            result = fc4_budget_handle_cheat(core_, id, s.data(), s.size());
            fc4::Log("budget cheat %08X '%s' -> %d (funds %lld)", id, s.c_str(), result ? 1 : 0,
                     static_cast<long long>(fc4_budget_get_total_funds(core_)));
        }
    } else if (type == kMsgSimNewMonth) {
        fc4_budget_state before{};
        before.struct_size = sizeof(before);
        fc4_budget_get_state(core_, &before);
        result = fc4_budget_handle_message(core_, type, data1);
        fc4_budget_state after{};
        after.struct_size = sizeof(after);
        fc4_budget_get_state(core_, &after);
        LogSettlement(before, after);
    } else {
        if (messagesLogged_ < 50 && type != kMsgSimNewDay) {
            ++messagesLogged_;
            FC4_LOGV("budget DoMessage(0x%08X, d1=0x%08llX)", type, static_cast<unsigned long long>(data1));
        } else if (fc4::LogLevel() >= fc4::kLogVerbose) {
            fc4::NoteSuppressed();
        }
        if (type == kMsgSimBegin || type == kMsgSimNewDay || type == kMsgCityEstablished ||
            type == kMsgToggleBudgetDialog) {
            result = fc4_budget_handle_message(core_, type, data1);
        }
        // IdealMonthlyFundingChanged / FundingPercentageChanged: subscribed, ignored (§4.1 step 4).
    }
    if (m != nullptr) FC4_RELEASE(m);
    return result;
}

void cFC4BudgetSimulator::LogSettlement(const fc4_budget_state& b, const fc4_budget_state& a) {
    fc4::Log("budget settlement: funds %lld -> %lld (income %lld, expense %lld, net %lld)%s",
             static_cast<long long>(b.funds), static_cast<long long>(a.funds),
             static_cast<long long>(a.last_month_income), static_cast<long long>(a.last_month_expense),
             static_cast<long long>(a.last_month_income - a.last_month_expense),
             b.funds != b.shadow_funds ? " [funds restored from shadow]" : "");
    char line[512];
    int n = _snprintf_s(line, sizeof(line), _TRUNCATE, "budget settlement: tax %lld =", static_cast<long long>(a.last_tax_income));
    for (int i = 0; i < 12 && n > 0 && n < static_cast<int>(sizeof(line)); ++i) {
        const int k = _snprintf_s(line + n, sizeof(line) - n, _TRUNCATE, " %lld@%.1f%%", static_cast<long long>(a.monthly_tax[i]),
                                  a.tax_rate[i]);
        if (k < 0) break;
        n += k;
    }
    fc4::Log("%s", line);
    fc4::Log("budget settlement: bonds %u (borrowed %lld, payments %lld/month, ytd %lld); departments %u; ytd income %lld "
             "expense %lld",
             a.bond_count, static_cast<long long>(a.total_borrowed),
             static_cast<long long>(fc4_budget_get_total_monthly_bond_payments(core_)),
             static_cast<long long>(a.ytd_bond_payments), a.department_count, static_cast<long long>(a.ytd_income),
             static_cast<long long>(a.ytd_expense));
}

// ---- cISC4BudgetSimulator (§6) ---------------------------------------------------------------------------

bool cFC4BudgetSimulator::SetTotalFunds(int64_t v) {
    FC4_TRACE("BudgetSimulator::SetTotalFunds");
    return fc4_budget_set_total_funds(core_, v);
}
int64_t cFC4BudgetSimulator::GetTotalFunds() {
    FC4_TRACE("BudgetSimulator::GetTotalFunds");
    return fc4_budget_get_total_funds(core_);
}
bool cFC4BudgetSimulator::DepositFunds(int64_t v) {
    FC4_TRACE("BudgetSimulator::DepositFunds");
    return fc4_budget_deposit_funds(core_, v);
}
bool cFC4BudgetSimulator::WithdrawFunds(int64_t v) {
    FC4_TRACE("BudgetSimulator::WithdrawFunds");
    return fc4_budget_withdraw_funds(core_, v);
}
int64_t cFC4BudgetSimulator::GetMinAllowableFunds() {
    FC4_TRACE("BudgetSimulator::GetMinAllowableFunds");
    return fc4_budget_get_min_allowable_funds(core_);
}
int64_t cFC4BudgetSimulator::GetYTDIncome() {
    FC4_TRACE("BudgetSimulator::GetYTDIncome");
    return fc4_budget_get_ytd_income(core_);
}
int64_t cFC4BudgetSimulator::GetEstIncome() {
    FC4_TRACE("BudgetSimulator::GetEstIncome");
    return fc4_budget_get_est_income(core_);
}
int64_t cFC4BudgetSimulator::GetYTDExpenses() {
    FC4_TRACE("BudgetSimulator::GetYTDExpenses");
    return fc4_budget_get_ytd_expenses(core_);
}
int64_t cFC4BudgetSimulator::GetEstExpenses() {
    FC4_TRACE("BudgetSimulator::GetEstExpenses");
    return fc4_budget_get_est_expenses(core_);
}
int64_t cFC4BudgetSimulator::GetTotalMonthlyExpense() {
    FC4_TRACE("BudgetSimulator::GetTotalMonthlyExpense");
    return fc4_budget_get_total_monthly_expense(core_);
}
int64_t cFC4BudgetSimulator::GetTotalYearlyExpense() {
    FC4_TRACE("BudgetSimulator::GetTotalYearlyExpense");
    return fc4_budget_get_total_yearly_expense(core_);
}
int64_t cFC4BudgetSimulator::GetTotalMonthlyIncome() {
    FC4_TRACE("BudgetSimulator::GetTotalMonthlyIncome");
    return fc4_budget_get_total_monthly_income(core_);
}
int64_t cFC4BudgetSimulator::GetTotalYearlyIncome() {
    FC4_TRACE("BudgetSimulator::GetTotalYearlyIncome");
    return fc4_budget_get_total_yearly_income(core_);
}
int64_t cFC4BudgetSimulator::GetTaxIncome() {
    FC4_TRACE("BudgetSimulator::GetTaxIncome");
    return fc4_budget_get_tax_income(core_);
}
int64_t cFC4BudgetSimulator::GetTaxIncomeOfType(int32_t t) {
    FC4_TRACE("BudgetSimulator::GetTaxIncomeOfType");
    return fc4_budget_get_tax_income_for(core_, t);
}
int32_t cFC4BudgetSimulator::SetTotalMonthlyExpense(int64_t v) {
    FC4_TRACE("BudgetSimulator::SetTotalMonthlyExpense");
    return static_cast<int32_t>(fc4_budget_set_total_monthly_expense(core_, v));  // [choice] low 32 bits (FC4Interfaces.h slots 20-23)
}
int32_t cFC4BudgetSimulator::SetTotalYearlyExpense(int64_t v) {
    FC4_TRACE("BudgetSimulator::SetTotalYearlyExpense");
    return static_cast<int32_t>(fc4_budget_set_total_yearly_expense(core_, v));  // [choice] low 32 bits (FC4Interfaces.h slots 20-23)
}
int32_t cFC4BudgetSimulator::SetTotalMonthlyIncome(int64_t v) {
    FC4_TRACE("BudgetSimulator::SetTotalMonthlyIncome");
    return static_cast<int32_t>(fc4_budget_set_total_monthly_income(core_, v));  // [choice] low 32 bits (FC4Interfaces.h slots 20-23)
}
int32_t cFC4BudgetSimulator::SetTotalYearlyIncome(int64_t v) {
    FC4_TRACE("BudgetSimulator::SetTotalYearlyIncome");
    return static_cast<int32_t>(fc4_budget_set_total_yearly_income(core_, v));  // [choice] low 32 bits (FC4Interfaces.h slots 20-23)
}
bool cFC4BudgetSimulator::ShowBudgetWindow() {
    FC4_TRACE("BudgetSimulator::ShowBudgetWindow");
    return fc4_budget_show_budget_window(core_);
}

cISC4DepartmentBudget* cFC4BudgetSimulator::CreateDepartmentBudget(uint32_t id, uint32_t group) {
    FC4_TRACE("BudgetSimulator::CreateDepartmentBudget");
    return DeptOf(fc4_budget_create_department(core_, id, group));  // borrowed
}

bool cFC4BudgetSimulator::RemoveDepartmentBudget(uint32_t id) {
    FC4_TRACE("BudgetSimulator::RemoveDepartmentBudget");
    return fc4_budget_remove_department(core_, id);
}

cISC4DepartmentBudget* cFC4BudgetSimulator::GetDepartmentBudget(uint32_t id) {
    FC4_TRACE("BudgetSimulator::GetDepartmentBudget");
    return DeptOf(fc4_budget_get_department(core_, id));
}

cISC4DepartmentBudget* cFC4BudgetSimulator::GetDepartmentBudgetByName(cIGZString const& name) {
    FC4_TRACE("BudgetSimulator::GetDepartmentBudgetByName");
    const std::string s = ToStd(name);
    return DeptOf(fc4_budget_get_department_by_name(core_, reinterpret_cast<const uint8_t*>(s.data()), s.size()));
}

namespace {
void PushGroup(void* ctx, const fc4_budget_group* g, const uint8_t* name, size_t len, const uint32_t*) {
    auto* v = static_cast<SC4Vector<cISC4BudgetSimulator::BudgetGroupInfo>*>(ctx);
    // The element owns its name's buffer; the vector (the game) must be its
    // only owner (FC4GameContainers.h). The SC4String temporary is moved
    // from, so its destructor frees nothing.
    fc4::PushOwned(*v, g->id, SC4String(reinterpret_cast<const char*>(name), static_cast<uint32_t>(len)),
                   g->total_expense, g->total_income);
}
}  // namespace

// [abi] Container crossing pending classification in docs/abi/allocators.md: SC4Vector<BudgetGroupInfo> and its SC4Strings.
bool cFC4BudgetSimulator::GetAllGroups(SC4Vector<BudgetGroupInfo>& groups) {
    FC4_TRACE("BudgetSimulator::GetAllGroups");
    groups.clear();
    fc4_budget_get_all_groups(core_, &PushGroup, &groups);
    return true;
}

bool cFC4BudgetSimulator::SetGroupName(uint32_t id, cIGZString& name) {
    FC4_TRACE("BudgetSimulator::SetGroupName");
    const std::string s = ToStd(name);
    return fc4_budget_set_group_name(core_, id, reinterpret_cast<const uint8_t*>(s.data()), s.size());
}

// [abi] Container crossing pending classification in docs/abi/allocators.md: SC4Vector filled for the game.
bool cFC4BudgetSimulator::GetDepartmentBudgetsInGroup(uint32_t id, SC4Vector<cISC4DepartmentBudget*>& out) {
    FC4_TRACE("BudgetSimulator::GetDepartmentBudgetsInGroup");
    out.clear();
    std::vector<uint64_t> tokens;
    fc4_budget_get_departments_in_group(core_, id, &PushToken, &tokens);
    for (uint64_t t : tokens) {
        cISC4DepartmentBudget* d = DeptOf(t);
        d->AddRef();  // one reference per element (spec §1)
        out.push_back(d);
    }
    return true;
}

void cFC4BudgetSimulator::NeededFundingChanged(cISC4DepartmentBudget* d) {
    FC4_TRACE("BudgetSimulator::NeededFundingChanged");
    fc4_budget_needed_funding_changed(core_, CoreOf(d));
}

void cFC4BudgetSimulator::FundingPercentageChanged(cISC4DepartmentBudget* d) {
    FC4_TRACE("BudgetSimulator::FundingPercentageChanged");
    fc4_budget_funding_percentage_changed(core_, CoreOf(d));
}

int64_t cFC4BudgetSimulator::GetFundingById(uint32_t) {
    FC4_TRACE("BudgetSimulator::GetFundingById");
    return fc4_budget_get_funding(core_);
}
int64_t cFC4BudgetSimulator::GetFundingOf(cISC4DepartmentBudget*) {
    FC4_TRACE("BudgetSimulator::GetFundingOf");
    return fc4_budget_get_funding(core_);
}
float cFC4BudgetSimulator::GetTaxRate(uint32_t t) {
    FC4_TRACE("BudgetSimulator::GetTaxRate");
    return fc4_budget_get_tax_rate(core_, t);
}
bool cFC4BudgetSimulator::SetTaxRate(uint32_t t, float r) {
    FC4_TRACE("BudgetSimulator::SetTaxRate");
    return fc4_budget_set_tax_rate(core_, t, r);
}
int64_t cFC4BudgetSimulator::GetWeightedAssessedTaxValue(uint32_t t) {
    FC4_TRACE("BudgetSimulator::GetWeightedAssessedTaxValue");
    return fc4_budget_get_weighted_assessed_tax_value(core_, t);
}
int64_t cFC4BudgetSimulator::GetBondIncrement() {
    FC4_TRACE("BudgetSimulator::GetBondIncrement");
    return fc4_budget_get_bond_increment(core_);
}
bool cFC4BudgetSimulator::IssueBond(uint32_t amount) {
    FC4_TRACE("BudgetSimulator::IssueBond");
    const bool ok = fc4_budget_issue_bond(core_, amount);
    fc4::Log("budget IssueBond(%u) -> %d (borrowed %lld)", amount, ok ? 1 : 0,
             static_cast<long long>(fc4_budget_get_total_borrowed(core_)));
    return ok;
}
int64_t cFC4BudgetSimulator::GetTotalBorrowed() {
    FC4_TRACE("BudgetSimulator::GetTotalBorrowed");
    return fc4_budget_get_total_borrowed(core_);
}
int64_t cFC4BudgetSimulator::GetCurrentBorrowingLimit() {
    FC4_TRACE("BudgetSimulator::GetCurrentBorrowingLimit");
    return fc4_budget_get_current_borrowing_limit(core_);
}
int64_t cFC4BudgetSimulator::GetCurrentBondLimit() {
    FC4_TRACE("BudgetSimulator::GetCurrentBondLimit");
    return fc4_budget_get_current_bond_limit(core_);
}
int64_t cFC4BudgetSimulator::GetCurrentMaxOutstandingBondsLimit() {
    FC4_TRACE("BudgetSimulator::GetCurrentMaxOutstandingBondsLimit");
    return fc4_budget_get_current_max_outstanding_bonds_limit(core_);
}
int64_t cFC4BudgetSimulator::GetTotalMonthlyBondPayments() {
    FC4_TRACE("BudgetSimulator::GetTotalMonthlyBondPayments");
    return fc4_budget_get_total_monthly_bond_payments(core_);
}

namespace {
void PushLoan(void* ctx, const fc4_loan_info* l) {
    cISC4BudgetSimulator::LoanInfo info;
    info.monthsRemaining = l->months_remaining;
    info.yearsRemaining = l->years_remaining;
    info.totalLoanAmount = l->total_loan_amount;
    info.remainingPayment = l->remaining_payment;
    info.monthlyPayment = l->monthly_payment;
    fc4::PushTrivial(*static_cast<SC4Vector<cISC4BudgetSimulator::LoanInfo>*>(ctx), info);
}

cISC4BudgetSimulator::BudgetItem FromC(const fc4_budget_item& i) {
    cISC4BudgetSimulator::BudgetItem b;
    b.departmentID = i.department_id;
    b.lineItem = i.line_item;
    b.purpose = i.purpose;
    b.cost = i.cost;
    b.index = i.index;
    b.localFundingPercentage = i.local_funding;
    return b;
}

void PushItem(void* ctx, const fc4_budget_item* i) {
    static_cast<std::vector<fc4_budget_item>*>(ctx)->push_back(*i);
}
}  // namespace

// [abi] Container crossing pending classification in docs/abi/allocators.md: SC4Vector filled for the game.
bool cFC4BudgetSimulator::GetAllLoans(SC4Vector<LoanInfo>& loans) {
    FC4_TRACE("BudgetSimulator::GetAllLoans");
    loans.clear();
    fc4_budget_get_all_loans(core_, &PushLoan, &loans);
    return true;
}

int32_t cFC4BudgetSimulator::GetLoanTimeInMonths() {
    FC4_TRACE("BudgetSimulator::GetLoanTimeInMonths");
    return fc4_budget_get_loan_time_in_months(core_);
}
int64_t cFC4BudgetSimulator::GetMonthlyPaymentForLoan(int64_t l) {
    FC4_TRACE("BudgetSimulator::GetMonthlyPaymentForLoan");
    return fc4_budget_get_monthly_payment_for_loan(core_, l);
}
int64_t cFC4BudgetSimulator::GetFullCostOfLoan(int64_t l) {
    FC4_TRACE("BudgetSimulator::GetFullCostOfLoan");
    return fc4_budget_get_full_cost_of_loan(core_, l);
}

// [abi] Container crossing pending classification in docs/abi/allocators.md: SC4Vector filled for the game.
bool cFC4BudgetSimulator::GetBudgetItemInfo(cISCPropertyHolder* holder, SC4Vector<BudgetItem>& out) {
    FC4_TRACE("BudgetSimulator::GetBudgetItemInfo");
    std::vector<fc4_budget_item> items;
    if (!fc4_budget_get_budget_item_info(core_, TokenOf(holder), &PushItem, &items)) return false;  // untouched
    out.clear();
    for (const fc4_budget_item& i : items) fc4::PushTrivial(out, FromC(i));
    return true;
}

bool cFC4BudgetSimulator::GetBudgetItemForPurpose(cISCPropertyHolder* holder, uint32_t purpose, BudgetItem& item) {
    FC4_TRACE("BudgetSimulator::GetBudgetItemForPurpose");
    fc4_budget_item i{};
    if (!fc4_budget_get_budget_item_for_purpose(core_, TokenOf(holder), purpose, &i)) return false;
    item = FromC(i);
    return true;
}

bool cFC4BudgetSimulator::ChangeBudgetItemLine(cISCPropertyHolder* holder, uint32_t index, uint32_t line) {
    FC4_TRACE("BudgetSimulator::ChangeBudgetItemLine");
    return fc4_budget_change_budget_item_line(core_, TokenOf(holder), index, line);
}

bool cFC4BudgetSimulator::ChangeBudgetItemCost(cISCPropertyHolder* holder, uint32_t index, int64_t cost) {
    FC4_TRACE("BudgetSimulator::ChangeBudgetItemCost");
    return fc4_budget_change_budget_item_cost(core_, TokenOf(holder), index, cost);
}

bool cFC4BudgetSimulator::ChangeBudgetItemLocalFunding(cISCPropertyHolder* holder, uint32_t index,
                                                       SC4Percentage const& funding) {
    FC4_TRACE("BudgetSimulator::ChangeBudgetItemLocalFunding");
    return fc4_budget_change_budget_item_local_funding(core_, TokenOf(holder), index, funding.percentage);
}

bool cFC4BudgetSimulator::AddBudgetItemToBudget(cISCPropertyHolder* holder, uint32_t index) {
    FC4_TRACE("BudgetSimulator::AddBudgetItemToBudget");
    return fc4_budget_add_budget_item_to_budget(core_, TokenOf(holder), index);
}

bool cFC4BudgetSimulator::RemoveBudgetItemFromBudget(cISCPropertyHolder* holder, uint32_t index) {
    FC4_TRACE("BudgetSimulator::RemoveBudgetItemFromBudget");
    return fc4_budget_remove_budget_item_from_budget(core_, TokenOf(holder), index);
}

cISC4LineItem* cFC4BudgetSimulator::GetLineItemFromBudgetItem(BudgetItem& item) {
    FC4_TRACE("BudgetSimulator::GetLineItemFromBudgetItem");
    fc4_budget_item i{};
    i.department_id = item.departmentID;
    i.line_item = item.lineItem;
    i.purpose = item.purpose;
    i.cost = item.cost;
    i.index = item.index;
    i.local_funding = item.localFundingPercentage;
    return LineOf(fc4_budget_get_line_item_from_budget_item(core_, &i));
}

bool cFC4BudgetSimulator::CopyBudgetItemProperties(cISCPropertyHolder* src, cISCPropertyHolder* dst) {
    FC4_TRACE("BudgetSimulator::CopyBudgetItemProperties");
    return fc4_budget_copy_budget_item_properties(core_, TokenOf(src), TokenOf(dst));
}

// ---- persistence (§11.1) ---------------------------------------------------------------------------------

bool cFC4BudgetSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("BudgetSimulator::Write");
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_budget_write(core_, a.Get());
    fc4::Log("budget Write -> %d", ok ? 1 : 0);
    return ok;
}

bool cFC4BudgetSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("BudgetSimulator::Read");
    // A saved city loads (Read comes before Init): re-read FreeCity4.ini.
    fc4::ConfigReload("city load: budget Read");
    fc4::RefAuditReload();
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_budget_read(core_, a.Get());
    fc4_budget_state st{};
    st.struct_size = sizeof(st);
    fc4_budget_get_state(core_, &st);
    fc4::Log("budget Read -> %d (funds %lld, departments %u, bonds %u)", ok ? 1 : 0, static_cast<long long>(st.funds),
             st.department_count, st.bond_count);
    return ok;
}

uint32_t cFC4BudgetSimulator::GetGZCLSID() {
    FC4_TRACE("BudgetSimulator::GetGZCLSID");
    return kFC4_CLSID_cSC4BudgetSimulator;
}

// ===================================================================================================
// Department budget (§7)
// ===================================================================================================

cFC4DepartmentBudget::cFC4DepartmentBudget() {
    core_ = fc4_department_create(fc4::BudgetHost(), TokenOf(static_cast<cFC4ISC4DepartmentBudget*>(this)));
}

cFC4DepartmentBudget::~cFC4DepartmentBudget() { fc4_department_destroy(core_); }

bool cFC4DepartmentBudget::QueryInterface(uint32_t riid, void** ppvObj) { return QueryInterfaceImpl(riid, ppvObj); }

bool cFC4DepartmentBudget::QueryInterfaceImpl(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_IID_cISC4DepartmentBudget:
        case kFC4_CLSID_cSC4DepartmentBudget:  // the CLSID is also accepted as an IID (§1)
            *ppvObj = static_cast<cFC4ISC4DepartmentBudget*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        case kFC4_IID_CoreObject:
            *ppvObj = static_cast<cIFC4CoreObject*>(this);
            break;
        case GZIID_cIGZUnknown:
            *ppvObj = static_cast<cIGZUnknown*>(static_cast<cFC4ISC4DepartmentBudget*>(this));
            break;
        default:
            *ppvObj = nullptr;
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4DepartmentBudget::AddRef() { return ++refCount_; }

uint32_t cFC4DepartmentBudget::Release() {
    if (refCount_ > 0) --refCount_;
    if (refCount_ == 0) {
        delete this;
        return 0;
    }
    return refCount_;
}

uint32_t cFC4DepartmentBudget::GetDepartmentID() const {
    FC4_TRACE("DepartmentBudget::GetDepartmentID");
    return fc4_department_get_id(core_);
}

bool cFC4DepartmentBudget::GetDepartmentName(cIGZString& name) {
    FC4_TRACE("DepartmentBudget::GetDepartmentName");
    std::string s;
    if (!fc4_department_get_name(core_, &AppendBytes, &s)) return false;
    name.FromChar(s.data(), static_cast<uint32_t>(s.size()));
    return true;
}

bool cFC4DepartmentBudget::SetDepartmentName(uint32_t g, uint32_t i) {
    FC4_TRACE("DepartmentBudget::SetDepartmentName");
    return fc4_department_set_name(core_, g, i);
}
uint32_t cFC4DepartmentBudget::GetBudgetGroup() const {
    FC4_TRACE("DepartmentBudget::GetBudgetGroup");
    return fc4_department_get_budget_group(core_);
}
bool cFC4DepartmentBudget::SetBudgetGroup(uint32_t g) {
    FC4_TRACE("DepartmentBudget::SetBudgetGroup");
    return fc4_department_set_budget_group(core_, g);
}
bool cFC4DepartmentBudget::GetIsFixedFunding() const {
    FC4_TRACE("DepartmentBudget::GetIsFixedFunding");
    return fc4_department_is_fixed_funding(core_);
}
bool cFC4DepartmentBudget::SetFixedFunding(bool v) {
    FC4_TRACE("DepartmentBudget::SetFixedFunding");
    return fc4_department_set_fixed_funding(core_, v);
}

SC4Percentage* cFC4DepartmentBudget::GetFundingPercentage() const {
    FC4_TRACE("DepartmentBudget::GetFundingPercentage");
    pct_.percentage = fc4_department_get_funding_percentage(core_);
    return &pct_;
}

bool cFC4DepartmentBudget::SetFundingPercentage(SC4Percentage const& p, uint32_t arg) {
    FC4_TRACE("DepartmentBudget::SetFundingPercentage");
    return fc4_department_set_funding_percentage(core_, p.percentage, arg);
}

SC4Percentage* cFC4DepartmentBudget::GetMaxAllowedFundingPercentage() const {
    FC4_TRACE("DepartmentBudget::GetMaxAllowedFundingPercentage");
    maxPct_.percentage = fc4_department_get_max_funding_percentage(core_);
    return &maxPct_;
}

bool cFC4DepartmentBudget::SetMaxAllowedFundingPercentage(SC4Percentage const& p) {
    FC4_TRACE("DepartmentBudget::SetMaxAllowedFundingPercentage");
    return fc4_department_set_max_funding_percentage(core_, p.percentage);
}

int64_t cFC4DepartmentBudget::GetIdealMonthlyFunding() const {
    FC4_TRACE("DepartmentBudget::GetIdealMonthlyFunding");
    return fc4_department_get_ideal_monthly_funding(core_);
}
int64_t cFC4DepartmentBudget::GetTotalExpenses() const {
    FC4_TRACE("DepartmentBudget::GetTotalExpenses");
    return fc4_department_get_total_expenses(core_);
}
int64_t cFC4DepartmentBudget::GetTotalIncome() const {
    FC4_TRACE("DepartmentBudget::GetTotalIncome");
    return fc4_department_get_total_income(core_);
}

cISC4LineItem* cFC4DepartmentBudget::CreateLineItem(uint32_t id, bool isLocal) {
    FC4_TRACE("DepartmentBudget::CreateLineItem");
    return LineOf(fc4_department_create_line_item(core_, id, isLocal));
}

cISC4LineItem* cFC4DepartmentBudget::CreateLineItemForBuildingType(uint32_t type, bool isLocal) {
    FC4_TRACE("DepartmentBudget::CreateLineItemForBuildingType");
    return LineOf(fc4_department_create_line_item_for_building_type(core_, type, isLocal));
}

bool cFC4DepartmentBudget::RemoveLineItem(uint32_t id) {
    FC4_TRACE("DepartmentBudget::RemoveLineItem");
    return fc4_department_remove_line_item(core_, id);
}

cISC4LineItem* cFC4DepartmentBudget::GetLineItem(uint32_t id) {
    FC4_TRACE("DepartmentBudget::GetLineItem");
    return LineOf(fc4_department_get_line_item(core_, id));
}

// [abi] Container crossing pending classification in docs/abi/allocators.md: SC4Vector filled for the game.
bool cFC4DepartmentBudget::GetAllLineItems(SC4Vector<cISC4LineItem*>& out) {
    FC4_TRACE("DepartmentBudget::GetAllLineItems");
    out.clear();
    std::vector<uint64_t> tokens;
    fc4_department_get_all_line_items(core_, &PushToken, &tokens);
    for (uint64_t t : tokens) {
        cISC4LineItem* l = LineOf(t);
        l->AddRef();
        out.push_back(l);
    }
    return true;
}

bool cFC4DepartmentBudget::AddLocallyFundedObject(cISCPropertyHolder* h, uint32_t purpose) {
    FC4_TRACE("DepartmentBudget::AddLocallyFundedObject");
    return h != nullptr && fc4_department_add_locally_funded_object(core_, TokenOf(h), purpose);
}

bool cFC4DepartmentBudget::RemoveLocallyFundedObject(cISCPropertyHolder* h, uint32_t purpose) {
    FC4_TRACE("DepartmentBudget::RemoveLocallyFundedObject");
    return h != nullptr && fc4_department_remove_locally_funded_object(core_, TokenOf(h), purpose);
}

bool cFC4DepartmentBudget::SetLocalFundingPercent(cISCPropertyHolder* h, SC4Percentage* p, uint32_t purpose) {
    FC4_TRACE("DepartmentBudget::SetLocalFundingPercent");
    return h != nullptr && p != nullptr &&
           fc4_department_set_local_funding_percent(core_, TokenOf(h), p->percentage, purpose);
}

bool cFC4DepartmentBudget::SetLocalFullFunding(cISCPropertyHolder* h, int64_t cost, uint32_t purpose) {
    FC4_TRACE("DepartmentBudget::SetLocalFullFunding");
    return h != nullptr && fc4_department_set_local_full_funding(core_, TokenOf(h), cost, purpose);
}

SC4Percentage const* cFC4DepartmentBudget::GetLocalFundingPercentPtr(cISCPropertyHolder* h, uint32_t purpose) {
    FC4_TRACE("DepartmentBudget::GetLocalFundingPercent");
    // Budget erratum [compat]: the game dereferences EAX at once.
    static const SC4Percentage kNotFound{0.0f};  // never written
    float q = 0.0f;
    if (h == nullptr || !fc4_department_find_local_funding_percent(core_, TokenOf(h), purpose, &q)) return &kNotFound;
    localPct_.percentage = q;  // [choice] valid until the next call on this object
    return &localPct_;
}

int64_t cFC4DepartmentBudget::GetLocalFullFunding(cISCPropertyHolder* h, uint32_t purpose) {
    FC4_TRACE("DepartmentBudget::GetLocalFullFunding");
    return fc4_department_get_local_full_funding(core_, TokenOf(h), purpose);
}

bool cFC4DepartmentBudget::IsLocallyFundedObjectInDepartment(cISCPropertyHolder* h, uint32_t purpose) {
    FC4_TRACE("DepartmentBudget::IsLocallyFundedObjectInDepartment");
    return fc4_department_is_locally_funded_object(core_, TokenOf(h), purpose);
}

bool cFC4DepartmentBudget::RecalculateAllLineItemCurrentExpenses() {
    FC4_TRACE("DepartmentBudget::RecalculateAllLineItemCurrentExpenses");
    return fc4_department_recalculate(core_);
}
int64_t cFC4DepartmentBudget::GetTotalSpending() const {
    FC4_TRACE("DepartmentBudget::GetTotalSpending");
    return fc4_department_get_total_spending(core_);
}
bool cFC4DepartmentBudget::SetTotalSpending(int64_t v) {
    FC4_TRACE("DepartmentBudget::SetTotalSpending");
    return fc4_department_set_total_spending(core_, v);
}

// [abi] Container crossing pending classification in docs/abi/allocators.md: SC4List nodes appended for the game.
bool cFC4DepartmentBudget::GetLocallyFundedItemsByPurpose(uint32_t purpose, SC4List<cISCPropertyHolder*>& list) {
    FC4_TRACE("DepartmentBudget::GetLocallyFundedItemsByPurpose");
    std::vector<uint64_t> tokens;
    fc4_department_get_locally_funded_items_by_purpose(core_, purpose, &PushToken, &tokens);
    for (uint64_t t : tokens) {  // appended, no clear (§7)
        cISCPropertyHolder* h = reinterpret_cast<cISCPropertyHolder*>(FromToken(t));
        FC4_ADDREF_FOR_CALLER(h);  // the caller's list owns it
        list.push_back(h);
    }
    return true;
}

bool cFC4DepartmentBudget::HasLocallyFundedItemsByPurpose(uint32_t purpose) {
    FC4_TRACE("DepartmentBudget::HasLocallyFundedItemsByPurpose");
    return fc4_department_has_locally_funded_items_by_purpose(core_, purpose);
}

bool cFC4DepartmentBudget::Write(cIGZOStream& s) {
    FC4_TRACE("DepartmentBudget::Write");
    fc4::OStreamAdapter a(s);
    return fc4_department_write(core_, a.Get());
}

bool cFC4DepartmentBudget::Read(cIGZIStream& s) {
    FC4_TRACE("DepartmentBudget::Read");
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_department_read(core_, a.Get());
    FC4_LOGV("department 0x%08X Read -> %d", fc4_department_get_id(core_), ok ? 1 : 0);
    return ok;
}

uint32_t cFC4DepartmentBudget::GetGZCLSID() {
    FC4_TRACE("DepartmentBudget::GetGZCLSID");
    return kFC4_CLSID_cSC4DepartmentBudget;
}

// ===================================================================================================
// Line item (§8)
// ===================================================================================================

cFC4LineItem::cFC4LineItem() {
    core_ = fc4_line_item_create(fc4::BudgetHost(), TokenOf(static_cast<cISC4LineItem*>(this)));
}

cFC4LineItem::~cFC4LineItem() { fc4_line_item_destroy(core_); }

bool cFC4LineItem::QueryInterface(uint32_t riid, void** ppvObj) { return QueryInterfaceImpl(riid, ppvObj); }

bool cFC4LineItem::QueryInterfaceImpl(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_CLSID_cSC4LineItem:  // the CLSID doubles as the IID (§1)
            *ppvObj = static_cast<cISC4LineItem*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        case kFC4_IID_CoreObject:
            *ppvObj = static_cast<cIFC4CoreObject*>(this);
            break;
        case GZIID_cIGZUnknown:
            *ppvObj = static_cast<cIGZUnknown*>(static_cast<cISC4LineItem*>(this));
            break;
        default:
            *ppvObj = nullptr;
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4LineItem::AddRef() { return ++refCount_; }

uint32_t cFC4LineItem::Release() {
    if (refCount_ > 0) --refCount_;
    if (refCount_ == 0) {
        delete this;
        return 0;
    }
    return refCount_;
}

uint32_t cFC4LineItem::GetID() const {
    FC4_TRACE("LineItem::GetID");
    return fc4_line_item_get_id(core_);
}
cISC4LineItem::Type cFC4LineItem::GetType() const {
    FC4_TRACE("LineItem::GetType");
    return static_cast<Type>(fc4_line_item_get_type(core_));
}
bool cFC4LineItem::SetType(Type t) {
    FC4_TRACE("LineItem::SetType");
    return fc4_line_item_set_type(core_, static_cast<uint32_t>(t));
}

bool cFC4LineItem::GetName(intptr_t& name) {
    FC4_TRACE("LineItem::GetName");
    // [compat] The argument is the game's string object (a cIGZString), not an
    // integer, despite the SDK's declaration.
    std::string s;
    if (!fc4_line_item_get_name(core_, &AppendBytes, &s)) return false;
    reinterpret_cast<cIGZString*>(&name)->FromChar(s.data(), static_cast<uint32_t>(s.size()));
    return true;
}

bool cFC4LineItem::SetName(uint32_t g, uint32_t i) {
    FC4_TRACE("LineItem::SetName");
    return fc4_line_item_set_name(core_, g, i);
}
int64_t cFC4LineItem::GetSecondaryInfoField() const {
    FC4_TRACE("LineItem::GetSecondaryInfoField");
    return fc4_line_item_get_secondary_info(core_);
}
bool cFC4LineItem::SetSecondaryInfoField(int64_t v) {
    FC4_TRACE("LineItem::SetSecondaryInfoField");
    return fc4_line_item_set_secondary_info(core_, v);
}
int64_t cFC4LineItem::GetFullExpenses() const {
    FC4_TRACE("LineItem::GetFullExpenses");
    return fc4_line_item_get_full_expenses(core_);
}
int64_t cFC4LineItem::GetCurrentExpenses() const {
    FC4_TRACE("LineItem::GetCurrentExpenses");
    return fc4_line_item_get_current_expenses(core_);
}
bool cFC4LineItem::SetFullExpenses(int64_t v) {
    FC4_TRACE("LineItem::SetFullExpenses");
    return fc4_line_item_set_full_expenses(core_, v);
}
bool cFC4LineItem::AddToFullExpenses(int64_t v) {
    FC4_TRACE("LineItem::AddToFullExpenses");
    return fc4_line_item_add_to_full_expenses(core_, v);
}
int64_t cFC4LineItem::GetIncome() const {
    FC4_TRACE("LineItem::GetIncome");
    return fc4_line_item_get_income(core_);
}
bool cFC4LineItem::SetIncome(int64_t v) {
    FC4_TRACE("LineItem::SetIncome");
    return fc4_line_item_set_income(core_, v);
}
bool cFC4LineItem::AddToIncome(int64_t v) {
    FC4_TRACE("LineItem::AddToIncome");
    return fc4_line_item_add_to_income(core_, v);
}
cISC4LineItem::DisplayFlag cFC4LineItem::GetDisplayFlags() {
    FC4_TRACE("LineItem::GetDisplayFlags");
    return static_cast<DisplayFlag>(fc4_line_item_get_display_flags(core_));
}
bool cFC4LineItem::SetDisplayFlag(DisplayFlag f, bool on) {
    FC4_TRACE("LineItem::SetDisplayFlag");
    return fc4_line_item_set_display_flag(core_, static_cast<uint32_t>(f), on);
}
bool cFC4LineItem::IsLocalFundingItem() const {
    FC4_TRACE("LineItem::IsLocalFundingItem");
    return fc4_line_item_is_local(core_);
}

bool cFC4LineItem::Write(cIGZOStream& s) {
    FC4_TRACE("LineItem::Write");
    fc4::OStreamAdapter a(s);
    return fc4_line_item_write(core_, a.Get());
}

bool cFC4LineItem::Read(cIGZIStream& s) {
    FC4_TRACE("LineItem::Read");
    fc4::IStreamAdapter a(s);
    return fc4_line_item_read(core_, a.Get());
}

uint32_t cFC4LineItem::GetGZCLSID() {
    FC4_TRACE("LineItem::GetGZCLSID");
    return kFC4_CLSID_cSC4LineItem;
}
