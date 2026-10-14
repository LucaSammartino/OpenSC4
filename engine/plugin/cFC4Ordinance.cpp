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

#include "cFC4Ordinance.h"

#include "FC4Checked.h"
#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "FC4GameList.h"
#include "SC4List.h"
#include "StringResourceKey.h"
#include "StringResourceManager.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessageServer2.h"
#include "cIGZString.h"
#include "cISC4App.h"
#include "cISC4BudgetSimulator.h"
#include "cISC4City.h"
#include "cISC4LandfillDeveloper.h"
#include "cISC4LineItem.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4Ordinance.h"
#include "cISC4PoliceSimulator.h"
#include "cISC4PollutionSimulator.h"
#include "cISC4PowerSimulator.h"
#include "cISC4ResidentialSimulator.h"
#include "cISC4Simulator.h"
#include "cRZAutoRefCount.h"
#include "cRZCOMDllDirector.h"

#include <cstring>
#include <string>

const uint32_t kFC4OrdinanceClasses[21] = {
    0x8BD4FC74,  // cSC4OrdinanceSimple
    0xA2BF1DDC, 0xA2BF1E43, 0x00D0723D, 0x40D07236, 0x815B4CEF, 0x22F6E80C, 0xA0D07129,
    0x62BF1DAA, 0x62BF1DB9, 0xE0D0722E, 0x22F6E81B, 0x82B9999B, 0xE0D07233, 0xA2BF1DE5,
    0xC2F6E81F, 0x62F6E7CF, 0x42BF1E18, 0xC2BF1E04, 0x02BF1DFA, 0xC2BF1DC5,
};

bool FC4IsOrdinanceClass(uint32_t clsid) {
    for (uint32_t c : kFC4OrdinanceClasses) {
        if (c == clsid) return true;
    }
    return false;
}

namespace {

// Messages (§5) and the agent (§8.1): month level, flags 2.
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kAgentLevel = 6;
constexpr uint32_t kAgentFlags = 2;
const char kAgentName[] = "cSC4OrdinanceSimulator";
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;

// Exemplars (§3.7, §8.1).
constexpr uint32_t kExemplarType = 0x6534284A;
constexpr uint32_t kOrdinanceGroup = 0xA9C2C209;

using fc4::FromToken;
using fc4::TokenOf;

cISC4City* City() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    FC4_OUT(cISC4App*, sc4, "cISC4App* (QueryInterface out pointer)");
    const bool ok = app != nullptr && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(sc4.ptr()));
    sc4.Check();
    if (!ok || sc4.get() == nullptr) return nullptr;
    FC4_ACQUIRED(sc4.get(), "QueryInterface");
    cISC4City* city = sc4.get()->GetCity();  // borrowed
    FC4_RELEASE(sc4.get());
    return city;
}

cFC4ISC4Ordinance* Ord(uint64_t token) {
    return fc4::AsGame(reinterpret_cast<cISC4Ordinance*>(static_cast<uintptr_t>(token)));
}

void Sink(const cIGZString* s, fc4_bytes_sink sink, void* ctx) {
    if (s == nullptr || sink == nullptr) return;
    const char* p = s->ToChar();
    if (p != nullptr) sink(ctx, reinterpret_cast<const uint8_t*>(p), s->Strlen());
}

void AppendBytes(void* ctx, const uint8_t* data, size_t len) {
    static_cast<std::string*>(ctx)->append(reinterpret_cast<const char*>(data), len);
}

uint32_t FloatBits(float f) {
    uint32_t u = 0;
    std::memcpy(&u, &f, sizeof(u));
    return u;
}

float BitsFloat(uint32_t u) {
    float f = 0.0f;
    std::memcpy(&f, &u, sizeof(f));
    return f;
}

// ---- host callbacks (fc4_ordinance_host) -----------------------------------------------------------

// [choice] The standard exemplar of every class we implement (§3.7: group
// 0xA9C2C209, instance = class ID). Only these classes can produce an
// ordinance (create_ordinance builds ours only), so the resource manager's key
// list is not enumerated; an exemplar of another group is not seen (open
// question 2 concerns duplicates only).
void ForEachExemplarKey(void*, fc4_key_visitor visit, void* ctx) {
    if (visit == nullptr) return;
    for (uint32_t c : kFC4OrdinanceClasses) visit(ctx, kExemplarType, kOrdinanceGroup, c);
}

fc4_props* ExemplarProps(void*, uint32_t type, uint32_t group, uint32_t instance) {
    fc4::props::Exemplar ex;
    if (!ex.Load(type, group, instance)) return nullptr;
    return fc4::props::ToCoreAll(ex.Holder());
}

bool Ltext(void*, uint32_t, uint32_t group, uint32_t instance, fc4_bytes_sink sink, void* ctx) {
    if (!fc4::On(fc4::Sw::Ltext)) {
        fc4::NoteSkipped(fc4::Sw::Ltext, "LTEXT lookup (not found)");
        return false;
    }
    cRZAutoRefCount<cIGZString> s;  // the SDK helper owns the reference
    if (!StringResourceManager::GetLocalizedString(StringResourceKey(group, instance), s)) return false;
    Sink(s, sink, ctx);
    return true;
}

// [choice] Our ordinances are created directly (not through GZCOM): the class
// table is ours and the object needs no registration.
fc4_ordinance* CreateOrdinance(void*, uint32_t classId, uint64_t* token) {
    if (!FC4IsOrdinanceClass(classId) || token == nullptr) return nullptr;
    cFC4Ordinance* o = new cFC4Ordinance(classId);
    if (o->Core() == nullptr) {
        delete o;
        return nullptr;
    }
    o->AddRef();  // the core's reference
    *token = o->Token();
    return o->Core();
}

void AddRefToken(void*, uint64_t t) {
    cIGZUnknown* u = FromToken(t);
    if (u == nullptr) return;
    if (fc4::IsOurObject(u)) {
        u->AddRef();
    } else {
        FC4_ADDREF(u);
    }
}

void ReleaseToken(void*, uint64_t t) {
    cIGZUnknown* u = FromToken(t);
    if (u == nullptr) return;
    if (fc4::IsOurObject(u)) {
        u->Release();
    } else {
        FC4_RELEASE(u);
    }
}

uint64_t ForeignOrdinance(void*, uint64_t t) {
    cIGZUnknown* u = FromToken(t);
    if (u == nullptr) return 0;
    FC4_OUT(cISC4Ordinance*, o, "cISC4Ordinance* (QueryInterface out pointer)");
    const bool ok = u->QueryInterface(GZIID_cISC4Ordinance, reinterpret_cast<void**>(o.ptr()));
    o.Check();
    if (!ok || o.get() == nullptr) return 0;
    if (fc4::IsOurObject(u)) {
        u->Release();
    } else {
        FC4_ACQUIRED(o.get(), "QueryInterface");
        FC4_RELEASE(u);
    }
    return TokenOf(o.get());
}

int64_t OrdinanceCall(void*, uint64_t t, uint32_t op, int64_t arg) {
    cFC4ISC4Ordinance* o = Ord(t);
    if (o == nullptr) return 0;
    const bool b = arg != 0;
    switch (op) {
        case FC4_ORD_INIT: return o->Init();
        case FC4_ORD_SHUTDOWN: return o->Shutdown();
        case FC4_ORD_CURRENT_MONTHLY_INCOME: return o->GetCurrentMonthlyIncome();
        case FC4_ORD_ID: return o->GetID();
        case FC4_ORD_YEAR_FIRST_AVAILABLE: return o->GetYearFirstAvailable();
        case FC4_ORD_CHANCE_AVAILABILITY: return o->GetChanceAvailability();
        case FC4_ORD_ENACTMENT_INCOME: return o->GetEnactmentIncome();
        case FC4_ORD_RETRACMENT_INCOME: return o->GetRetracmentIncome();
        case FC4_ORD_MONTHLY_CONSTANT_INCOME: return o->GetMonthlyConstantIncome();
        case FC4_ORD_MONTHLY_INCOME_FACTOR: return FloatBits(o->GetMonthlyIncomeFactor());
        case FC4_ORD_ADVISOR_ID: return o->GetAdvisorID();
        case FC4_ORD_IS_AVAILABLE: return o->IsAvailable();
        case FC4_ORD_IS_ON: return o->IsOn();
        case FC4_ORD_IS_ENABLED: return o->IsEnabled();
        case FC4_ORD_MONTHLY_ADJUSTED_INCOME: return o->GetMonthlyAdjustedIncome();
        case FC4_ORD_CHECK_CONDITIONS: return o->CheckConditions();
        case FC4_ORD_IS_INCOME_ORDINANCE: return o->IsIncomeOrdinance();
        case FC4_ORD_SIMULATE: return o->Simulate();
        case FC4_ORD_SET_AVAILABLE: return o->SetAvailable(b);
        case FC4_ORD_SET_ON: return o->SetOn(b);
        case FC4_ORD_SET_ENABLED: return o->SetEnabled(b);
        case FC4_ORD_FORCE_AVAILABLE: return o->ForceAvailable(b);
        case FC4_ORD_FORCE_ON: return o->ForceOn(b);
        case FC4_ORD_FORCE_MONTHLY_ADJUSTED_INCOME: return o->ForceMonthlyAdjustedIncome(arg);
        default: return 0;
    }
}

bool OrdinanceText(void*, uint64_t t, uint32_t which, fc4_bytes_sink sink, void* ctx) {
    cFC4ISC4Ordinance* o = Ord(t);
    if (o == nullptr) return false;
    cIGZString* s = which == 0 ? o->GetName() : o->GetDescription();  // borrowed
    if (s == nullptr) return false;
    Sink(s, sink, ctx);
    return true;
}

bool OrdinanceMiscProperty(void*, uint64_t t, uint32_t prop, fc4_props* out) {
    cFC4ISC4Ordinance* o = Ord(t);
    if (o == nullptr || out == nullptr) return false;
    return fc4::props::CopyToCore(o->GetMiscProperties(), prop, out);
}

// void1 = the simulator's IGZUnknown, data2, data3 (§7.1).
void Post(void* u, uint32_t type, uint32_t data2, uint32_t data3) {
    auto* sim = static_cast<cFC4OrdinanceSimulator*>(u);
    if (sim == nullptr) return;
    if (!fc4::On(fc4::Sw::MessagePosting)) {
        fc4::NoteSkipped(fc4::Sw::MessagePosting, "ordinance message post");
        return;
    }
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* msg = cFC4Message2Standard::New();
    msg->AddRef();
    msg->SetType(type);
    msg->SetVoid1(sim->AsUnknown());
    msg->SetData2(static_cast<intptr_t>(data2));
    msg->SetData3(static_cast<intptr_t>(data3));
    ms->MessagePost(msg, false);
    msg->Release();
    FC4_LOGV("ordinance post 0x%08X d2=0x%08X d3=%u", type, data2, data3);
}

bool SimYear(void*, uint32_t* year) {
    cISC4City* c = City();
    cFC4ISC4Simulator* s = c ? fc4::AsGame(c->GetSimulator()) : nullptr;
    if (s == nullptr || year == nullptr) return false;
    FC4_OUT(uint32_t, y, "uint32 year (GetSimDateParts)");
    FC4_OUT(uint32_t, m, "uint32 month (GetSimDateParts)");
    FC4_OUT(uint32_t, d, "uint32 day (GetSimDateParts)");
    FC4_OUT(uint32_t, doy, "uint32 day of year (GetSimDateParts)");
    FC4_OUT(uint32_t, wd, "uint32 weekday (GetSimDateParts)");
    s->GetSimDateParts(y.ptr(), m.ptr(), d.ptr(), doy.ptr(), wd.ptr());
    y.Check();
    m.Check();
    d.Check();
    doy.Check();
    wd.Check();
    *year = y.get();
    return true;
}

int32_t ResidentialPopulation(void*) {
    cISC4City* c = City();
    cISC4ResidentialSimulator* r = c ? c->GetResidentialSimulator() : nullptr;
    return r ? r->GetPopulation() : 0;
}

bool HasBudgetSimulator(void*) {
    cISC4City* c = City();
    return c != nullptr && c->GetBudgetSimulator() != nullptr;
}

// Borrowed pointers throughout (budget: GetDepartmentBudget and GetLineItem add no reference).
cFC4ISC4DepartmentBudget* Department(uint32_t dept) {
    cISC4City* c = City();
    cFC4ISC4BudgetSimulator* b = c ? fc4::AsGame(c->GetBudgetSimulator()) : nullptr;
    cISC4DepartmentBudget* d = b ? b->GetDepartmentBudget(dept) : nullptr;
    return d ? fc4::AsGame(d) : nullptr;
}

bool DepartmentTotalExpenses(void*, uint32_t dept, int64_t* out) {
    cFC4ISC4DepartmentBudget* d = Department(dept);
    if (d == nullptr || out == nullptr) return false;
    *out = d->GetTotalExpenses();
    return true;
}

bool LineItemFullExpenses(void*, uint32_t dept, uint32_t line, int64_t* out) {
    cFC4ISC4DepartmentBudget* d = Department(dept);
    cISC4LineItem* li = d ? d->GetLineItem(line) : nullptr;
    if (li == nullptr || out == nullptr) return false;
    *out = li->GetFullExpenses();
    return true;
}

// §3.6 LandfillGasRecovery: the second output of GetLandfillUsage.
uint32_t LandfillCapacity(void*) {
    cISC4City* c = City();
    cISC4LandfillDeveloper* l = c ? c->GetLandfillDeveloper() : nullptr;
    if (l == nullptr) return 0;
    FC4_OUT(uint32_t, usage, "uint32 usage (GetLandfillUsage)");
    FC4_OUT(uint32_t, capacity, "uint32 capacity (GetLandfillUsage)");
    l->GetLandfillUsage(usage.get(), capacity.get());
    usage.Check();
    capacity.Check();
    return capacity.get();
}

int32_t PowerPlantCount(void*, int32_t type) {
    cISC4City* c = City();
    cISC4PowerSimulator* p = c ? c->GetPowerSimulator() : nullptr;
    return p ? p->GetPowerPlantCount(type) : 0;
}

uint32_t PoliceStationCount(void*) {
    cISC4City* c = City();
    cISC4PoliceSimulator* p = c ? c->GetPoliceSimulator() : nullptr;
    // Slot 24, through the verified declaration (protection.md §15.2).
    return p ? fc4::AsGame(p)->GetPoliceStationCount() : 0;
}

// §3.6 TrashPresort: the getter returns a POINTER to an f32 fraction in EAX
// (the SDK says int32; the plugin is 32-bit, so the value is the pointer).
static_assert(sizeof(void*) == 4, "GetRecyclingCenterEffect returns a 32-bit pointer");
float RecyclingCenterEffect(void*) {
    cISC4City* c = City();
    cISC4PollutionSimulator* p = c ? c->GetPollutionSimulator() : nullptr;
    if (p == nullptr) return 0.0f;
    const auto* f = reinterpret_cast<const float*>(static_cast<intptr_t>(p->GetRecyclingCenterEffect()));
    return f ? *f : 0.0f;
}

// §3.6 LegalizeGambling SetOn(false), step 1: the lots of a building type.
// [choice] The tokens are borrowed (the lot manager keeps the lots); the
// list's references are released before returning.
void LotsWithBuildingType(void*, uint32_t type, fc4_token_callback cb, void* ctx) {
    if (!fc4::On(fc4::Sw::LotScan)) {
        fc4::NoteSkipped(fc4::Sw::LotScan, "ordinance lot scan (no lots)");
        return;
    }
    cISC4City* c = City();
    cFC4ISC4LotManager* lm = c ? fc4::AsGame(c->GetLotManager()) : nullptr;
    if (lm == nullptr || cb == nullptr) return;
    const int32_t maxX = static_cast<int32_t>(c->CellCountX()) - 1;
    const int32_t maxZ = static_cast<int32_t>(c->CellCountZ()) - 1;
    // [abi] The game allocates the list's nodes from its STL node allocator
    // (docs/abi/allocators.md): a GameList, never the SDK SC4List.
    if (!fc4::StlNodeAllocator::Available()) {
        fc4::Log("ordinance: WARNING lot scan refused (STL node allocator unavailable); no lots");
        return;
    }
    fc4::GameList<cISC4Lot*> lots;
    if (!lots.valid()) return;
    lm->GetLotsInCellRect(lots, 0, 0, maxX, maxZ, false);  // slot 7
    for (cISC4Lot*& sdkLot : lots) {
        if (sdkLot == nullptr) continue;
        FC4_ACQUIRED(sdkLot, "GetLotsInCellRect list entry");
        if (fc4::AsGame(sdkLot)->GetBuildingType(false) == type) cb(ctx, TokenOf(sdkLot));  // slot 46
    }
    for (cISC4Lot*& sdkLot : lots) {
        if (sdkLot == nullptr) continue;
        FC4_RELEASE(sdkLot);
        sdkLot = nullptr;
    }
}

// Steps 1-3 of LegalizeGambling SetOn(false) beyond the lot scan are not
// implemented [spec question]: the demolition utility's arguments, the civic
// building simulator's status record and the 3D view's input-control calls
// are not specified. They are logged.
void DemolishLots(void*, const uint64_t*, size_t count) {
    fc4::Log("ordinance: LegalizeGambling off: %u casino lot(s) to demolish; demolition is not implemented",
             static_cast<unsigned>(count));
}

void HideConditionalBuilding(void*, uint32_t type) {
    fc4::Log("ordinance: LegalizeGambling off: conditional building 0x%08X should be hidden (not implemented)", type);
}

void RemoveViewInputControl(void*, uint32_t id) {
    FC4_LOGV("ordinance: LegalizeGambling off: input control 0x%08X not removed (not implemented)", id);
}

}  // namespace

fc4_ordinance_host FC4MakeOrdinanceHost(cFC4OrdinanceSimulator* sim) {
    fc4_ordinance_host h{};
    h.struct_size = sizeof(h);
    h.user = sim;
    h.for_each_exemplar_key = &ForEachExemplarKey;
    h.exemplar = &ExemplarProps;
    h.ltext = &Ltext;
    h.create_ordinance = &CreateOrdinance;
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.foreign_ordinance = &ForeignOrdinance;
    h.ordinance_call = &OrdinanceCall;
    h.ordinance_text = &OrdinanceText;
    h.ordinance_misc_property = &OrdinanceMiscProperty;
    h.post = &Post;
    h.sim_year = &SimYear;
    h.residential_population = &ResidentialPopulation;
    h.has_budget_simulator = &HasBudgetSimulator;
    h.department_total_expenses = &DepartmentTotalExpenses;
    h.line_item_full_expenses = &LineItemFullExpenses;
    h.landfill_capacity = &LandfillCapacity;
    h.power_plant_count = &PowerPlantCount;
    h.police_station_count = &PoliceStationCount;
    h.recycling_center_effect = &RecyclingCenterEffect;
    h.lots_with_building_type = &LotsWithBuildingType;
    h.demolish_lots = &DemolishLots;
    h.hide_conditional_building = &HideConditionalBuilding;
    h.remove_view_input_control = &RemoveViewInputControl;
    return h;
}

// =====================================================================================================
// The ordinance simulator
// =====================================================================================================

cIGZUnknown* CreateFC4OrdinanceSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cISC4OrdinanceSimulator*>(new cFC4OrdinanceSimulator()));
}

cFC4OrdinanceSimulator::cFC4OrdinanceSimulator() {
    host_ = FC4MakeOrdinanceHost(this);
    core_ = fc4_ordinance_sim_create(&host_, TokenOf(AsUnknown()));
    if (core_ == nullptr) fc4::Log("cFC4OrdinanceSimulator: core creation failed");
    fc4::Log("cFC4OrdinanceSimulator(%p) created, core %p", static_cast<void*>(this), static_cast<void*>(core_));
}

cFC4OrdinanceSimulator::~cFC4OrdinanceSimulator() {
    fc4_ordinance_sim_destroy(core_);  // releases every ordinance (§8.2)
    core_ = nullptr;
    fc4::Log("cFC4OrdinanceSimulator destroyed");
}

bool cFC4OrdinanceSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_IID_cISC4OrdinanceSimulator:
            *ppvObj = static_cast<cISC4OrdinanceSimulator*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        case GZIID_cIGZUnknown:
            *ppvObj = AsUnknown();
            break;
        default:
            *ppvObj = nullptr;
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4OrdinanceSimulator::AddRef() { return ++refCount_; }

// [choice] As the other simulators: at a count of 1 (or below) Shutdown,
// destroy, return 0 (the ordinance spec does not describe Release).
uint32_t cFC4OrdinanceSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

void cFC4OrdinanceSimulator::LogState(const char* what) {
    fc4_ordinance_sim_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_ordinance_sim_get_state(core_, &st)) return;
    fc4::Log("ordinance %s: %u ordinances (%u available, %u on), loaded=%d, expense %lld, income %lld, pending %lld",
             what, st.count, fc4_ordinance_sim_count(core_, 0), fc4_ordinance_sim_count(core_, 1),
             st.loaded_from_save, static_cast<long long>(st.monthly_expense),
             static_cast<long long>(st.monthly_income), static_cast<long long>(st.pending));
}

// ---- lifecycle (§8) ---------------------------------------------------------------------------------

bool cFC4OrdinanceSimulator::Init() {
    FC4_TRACE("OrdinanceSimulator::Init");
    fc4_ordinance_sim_state st{};
    st.struct_size = sizeof(st);
    const bool wasInitialized = fc4_ordinance_sim_get_state(core_, &st) && st.initialized != 0;
    if (wasInitialized) return true;
    RegisterAgent(true);                         // step 2
    const bool ok = fc4_ordinance_sim_init(core_);  // steps 1, 3, 4
    fc4::Log("ordinance Init: ok=%d simulator=%p agent=%d", ok ? 1 : 0, static_cast<void*>(simulator_),
             agentRegistered_ ? 1 : 0);
    LogState("after Init");
    return true;
}

bool cFC4OrdinanceSimulator::Shutdown() {
    FC4_TRACE("OrdinanceSimulator::Shutdown");
    fc4_ordinance_sim_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_ordinance_sim_get_state(core_, &st) || st.initialized == 0) return true;
    if (!destroying_) AddRef();  // keep alive while the simulator lets go
    RegisterAgent(false);
    fc4_ordinance_sim_shutdown(core_);  // the map is kept (§8.2)
    fc4::Log("ordinance Shutdown");
    if (!destroying_) Release();
    return true;
}

void cFC4OrdinanceSimulator::RegisterAgent(bool add) {
    cIGZMessageTarget2* self = static_cast<cIGZMessageTarget2*>(this);
    if (add) {
        if (simulator_ == nullptr) {
            cISC4City* c = City();
            simulator_ = c ? c->GetSimulator() : nullptr;
            FC4_ADDREF(simulator_);
        }
        cFC4ISC4Simulator* sim = fc4::AsGame(simulator_);
        if (sim == nullptr) {
            fc4::Log("ordinance: no simulator; agent not registered");
            return;
        }
        const cRZBaseString name(kAgentName);
        agentRegistered_ = sim->AddAgent(self, kAgentLevel, name, kAgentFlags);
    } else {
        if (agentRegistered_ && simulator_ != nullptr) fc4::AsGame(simulator_)->RemoveAgentAllLevels(self);
        agentRegistered_ = false;
        FC4_RELEASE(simulator_);
        simulator_ = nullptr;
    }
}

// ---- cISC4OrdinanceSimulator (§4.2) -------------------------------------------------------------------

namespace {
// The core handle of one of our ordinance objects, NULL for a foreign one.
const fc4_ordinance* CoreOf(cISC4Ordinance* o) {
    if (!fc4::IsOurObject(o)) return nullptr;
    cIFC4CoreObject* co = nullptr;
    if (!o->QueryInterface(kFC4_IID_CoreObject, reinterpret_cast<void**>(&co)) || co == nullptr) return nullptr;
    const fc4_ordinance* h =
        FC4IsOrdinanceClass(co->CoreClass()) ? static_cast<const fc4_ordinance*>(co->CoreHandle()) : nullptr;
    co->Release();
    return h;
}
}  // namespace

bool cFC4OrdinanceSimulator::AddOrdinance(cISC4Ordinance& ordinance) {
    FC4_TRACE("OrdinanceSimulator::AddOrdinance");
    cISC4Ordinance* o = &ordinance;  // §4.2: null -> false (callers pass *nullptr)
    if (o == nullptr) return false;
    return fc4_ordinance_sim_add(core_, TokenOf(o), CoreOf(o));
}

bool cFC4OrdinanceSimulator::RemoveOrdinance(cISC4Ordinance& ordinance) {
    FC4_TRACE("OrdinanceSimulator::RemoveOrdinance");
    cISC4Ordinance* o = &ordinance;
    if (o == nullptr) return false;
    return fc4_ordinance_sim_remove(core_, TokenOf(o), CoreOf(o));
}

bool cFC4OrdinanceSimulator::IsOrdinanceAvailable(uint32_t id) { return fc4_ordinance_sim_query(core_, 0, id); }
bool cFC4OrdinanceSimulator::IsOrdinanceOn(uint32_t id) { return fc4_ordinance_sim_query(core_, 1, id); }
bool cFC4OrdinanceSimulator::IsOrdinanceAvailableButOff(uint32_t id) { return fc4_ordinance_sim_query(core_, 2, id); }
bool cFC4OrdinanceSimulator::IsOrdinanceEnabled(uint32_t id) { return fc4_ordinance_sim_query(core_, 3, id); }

bool cFC4OrdinanceSimulator::SetOrdinanceAvailable(uint32_t id, bool available) {
    FC4_TRACE("OrdinanceSimulator::SetOrdinanceAvailable");
    return fc4_ordinance_sim_set(core_, 0, id, available);
}

bool cFC4OrdinanceSimulator::SetOrdinanceOn(uint32_t id, bool on) {
    FC4_TRACE("OrdinanceSimulator::SetOrdinanceOn");
    const bool r = fc4_ordinance_sim_set(core_, 1, id, on);
    fc4::Log("ordinance 0x%08X: SetOrdinanceOn(%d) -> %d (on=%d)", id, on ? 1 : 0, r ? 1 : 0,
             fc4_ordinance_sim_query(core_, 1, id) ? 1 : 0);
    return r;
}

bool cFC4OrdinanceSimulator::SetOrdinanceEnabled(uint32_t id, bool enabled) {
    return fc4_ordinance_sim_set(core_, 2, id, enabled);
}

uint32_t cFC4OrdinanceSimulator::GetOrdinanceAvailableCount() { return fc4_ordinance_sim_count(core_, 0); }
uint32_t cFC4OrdinanceSimulator::GetOrdinanceOnCount() { return fc4_ordinance_sim_count(core_, 1); }

uint32_t cFC4OrdinanceSimulator::GetOrdinanceAvailableArray(uint32_t* out, uint32_t& count) {
    return fc4_ordinance_sim_array(core_, 0, out, &count);
}
uint32_t cFC4OrdinanceSimulator::GetOrdinanceOnArray(uint32_t* out, uint32_t& count) {
    return fc4_ordinance_sim_array(core_, 1, out, &count);
}
uint32_t cFC4OrdinanceSimulator::GetOrdinanceIDArray(uint32_t* out, uint32_t& count) {
    return fc4_ordinance_sim_array(core_, 2, out, &count);
}

uint32_t cFC4OrdinanceSimulator::GetOrdinanceIDByName(cIGZString const& name) {
    const char* p = name.ToChar();
    return fc4_ordinance_sim_id_by_name(core_, reinterpret_cast<const uint8_t*>(p ? p : ""), p ? name.Strlen() : 0);
}

// No AddRef (§4.2 [compat]).
cISC4Ordinance* cFC4OrdinanceSimulator::GetOrdinanceByID(uint32_t id) {
    return reinterpret_cast<cISC4Ordinance*>(static_cast<uintptr_t>(fc4_ordinance_sim_by_id(core_, id)));
}

cISC4Ordinance* cFC4OrdinanceSimulator::GetOrdinanceByName(cIGZString const& name) {
    const char* p = name.ToChar();
    const uint64_t t =
        fc4_ordinance_sim_by_name(core_, reinterpret_cast<const uint8_t*>(p ? p : ""), p ? name.Strlen() : 0);
    return reinterpret_cast<cISC4Ordinance*>(static_cast<uintptr_t>(t));
}

int64_t cFC4OrdinanceSimulator::GetOrdinanceMonthlyExpense() { return fc4_ordinance_sim_money(core_, 0); }
int64_t cFC4OrdinanceSimulator::GetOrdinanceMonthlyIncome() { return fc4_ordinance_sim_money(core_, 1); }

// §6.1 [compat]: a 32-bit sum; returned sign-extended in EDX:EAX, so a caller
// reading EAX (the original's channel) sees the same value.
int64_t cFC4OrdinanceSimulator::GetCumulativeOffset(uint32_t prop, uint32_t index) {
    return fc4_ordinance_sim_cumulative_offset(core_, prop, index);
}

float cFC4OrdinanceSimulator::GetCumulativeFactor(uint32_t prop, uint32_t index) {
    return fc4_ordinance_sim_cumulative_factor(core_, prop, index);
}

// ---- messages (§5) --------------------------------------------------------------------------------------

bool cFC4OrdinanceSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("ordinance DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    const uint32_t type = pMessage->GetType();
    fc4_ordinance_sim_handle_message(core_, type);
    if (type == kMsgSimNewMonth) LogState("month");
    return true;
}

// ---- persistence (§9.1) ------------------------------------------------------------------------------------

bool cFC4OrdinanceSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("OrdinanceSimulator::Write");
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_ordinance_sim_write(core_, a.Get());
    fc4::Log("ordinance Write -> %d", ok ? 1 : 0);
    return ok;
}

bool cFC4OrdinanceSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("OrdinanceSimulator::Read");
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_ordinance_sim_read(core_, a.Get());
    fc4::Log("ordinance Read -> %d", ok ? 1 : 0);
    LogState("after Read");
    return ok;
}

uint32_t cFC4OrdinanceSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4OrdinanceSimulator; }

// =====================================================================================================
// Standard ordinances
// =====================================================================================================

namespace {
const fc4_ordinance_host* OrdinanceHost() {
    static const fc4_ordinance_host h = FC4MakeOrdinanceHost(nullptr);
    return &h;
}
}  // namespace

cFC4Ordinance::cFC4Ordinance(uint32_t classId) : classId_(classId) {
    core_ = fc4_ordinance_create(OrdinanceHost(), classId, Token());
    if (core_ == nullptr) fc4::Log("cFC4Ordinance(0x%08X): core creation failed", classId);
    FC4_LOGV("cFC4Ordinance(0x%08X) %p created", classId, static_cast<void*>(this));
}

cFC4Ordinance::~cFC4Ordinance() {
    fc4_ordinance_destroy(core_);
    core_ = nullptr;
}

// [choice] Not the original's private IID 0x8BD4FC74 (§10.2: only the
// original simulator used it) nor the misc holder's interfaces (callers use
// GetMiscProperties).
bool cFC4Ordinance::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case GZIID_cISC4Ordinance:
            *ppvObj = static_cast<cFC4ISC4Ordinance*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        case kFC4_IID_CoreObject:
            *ppvObj = static_cast<cIFC4CoreObject*>(this);
            break;
        case GZIID_cIGZUnknown:
            *ppvObj = static_cast<cIGZUnknown*>(static_cast<cFC4ISC4Ordinance*>(this));
            break;
        default:
            *ppvObj = nullptr;
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4Ordinance::AddRef() { return ++refCount_; }

uint32_t cFC4Ordinance::Release() {
    if (refCount_ > 0) --refCount_;
    if (refCount_ == 0) {
        delete this;
        return 0;
    }
    return refCount_;
}

int64_t cFC4Ordinance::Call(uint32_t op, int64_t arg) const { return fc4_ordinance_call(core_, op, arg); }

bool cFC4Ordinance::Init() { return Call(FC4_ORD_INIT) != 0; }
bool cFC4Ordinance::Shutdown() { return Call(FC4_ORD_SHUTDOWN) != 0; }
int64_t cFC4Ordinance::GetCurrentMonthlyIncome() { return Call(FC4_ORD_CURRENT_MONTHLY_INCOME); }
uint32_t cFC4Ordinance::GetID() const { return static_cast<uint32_t>(Call(FC4_ORD_ID)); }

// The strings belong to the object (no reference for the caller).
cIGZString* cFC4Ordinance::GetName() {
    std::string s;
    fc4_ordinance_text(core_, 0, &AppendBytes, &s);
    name_.FromChar(s.data(), static_cast<uint32_t>(s.size()));
    return &name_;
}

cIGZString* cFC4Ordinance::GetDescription() {
    std::string s;
    fc4_ordinance_text(core_, 1, &AppendBytes, &s);
    description_.FromChar(s.data(), static_cast<uint32_t>(s.size()));
    return &description_;
}

uint32_t cFC4Ordinance::GetYearFirstAvailable() { return static_cast<uint32_t>(Call(FC4_ORD_YEAR_FIRST_AVAILABLE)); }
uint32_t cFC4Ordinance::GetChanceAvailability() { return static_cast<uint32_t>(Call(FC4_ORD_CHANCE_AVAILABILITY)); }
int64_t cFC4Ordinance::GetEnactmentIncome() { return Call(FC4_ORD_ENACTMENT_INCOME); }
int64_t cFC4Ordinance::GetRetracmentIncome() { return Call(FC4_ORD_RETRACMENT_INCOME); }
int64_t cFC4Ordinance::GetMonthlyConstantIncome() { return Call(FC4_ORD_MONTHLY_CONSTANT_INCOME); }
float cFC4Ordinance::GetMonthlyIncomeFactor() {
    return BitsFloat(static_cast<uint32_t>(Call(FC4_ORD_MONTHLY_INCOME_FACTOR)));
}
uint32_t cFC4Ordinance::GetAdvisorID() { return static_cast<uint32_t>(Call(FC4_ORD_ADVISOR_ID)); }

// [choice] The parent exemplar's holder (the ordinance's own saved properties
// are not visible through it). Loaded once per key; nullptr without one.
cISCPropertyHolder* cFC4Ordinance::GetMiscProperties() {
    uint32_t key[3] = {0, 0, 0};
    if (!fc4_ordinance_get_key(core_, &key[0], &key[1], &key[2])) return nullptr;
    if (!miscLoaded_ || std::memcmp(key, miscKey_, sizeof(key)) != 0) {
        miscLoaded_ = true;
        std::memcpy(miscKey_, key, sizeof(key));
        misc_.Load(key[0], key[1], key[2]);
    }
    return const_cast<cISCPropertyHolder*>(misc_.Holder());
}

bool cFC4Ordinance::IsAvailable() { return Call(FC4_ORD_IS_AVAILABLE) != 0; }
bool cFC4Ordinance::IsOn() { return Call(FC4_ORD_IS_ON) != 0; }
bool cFC4Ordinance::IsEnabled() { return Call(FC4_ORD_IS_ENABLED) != 0; }
int64_t cFC4Ordinance::GetMonthlyAdjustedIncome() { return Call(FC4_ORD_MONTHLY_ADJUSTED_INCOME); }
bool cFC4Ordinance::CheckConditions() { return Call(FC4_ORD_CHECK_CONDITIONS) != 0; }
bool cFC4Ordinance::IsIncomeOrdinance() { return Call(FC4_ORD_IS_INCOME_ORDINANCE) != 0; }
bool cFC4Ordinance::Simulate() { return Call(FC4_ORD_SIMULATE) != 0; }
bool cFC4Ordinance::SetAvailable(bool v) { return Call(FC4_ORD_SET_AVAILABLE, v) != 0; }
bool cFC4Ordinance::SetOn(bool v) { return Call(FC4_ORD_SET_ON, v) != 0; }
bool cFC4Ordinance::SetEnabled(bool v) { return Call(FC4_ORD_SET_ENABLED, v) != 0; }
bool cFC4Ordinance::ForceAvailable(bool v) { return Call(FC4_ORD_FORCE_AVAILABLE, v) != 0; }
bool cFC4Ordinance::ForceOn(bool v) { return Call(FC4_ORD_FORCE_ON, v) != 0; }
bool cFC4Ordinance::ForceMonthlyAdjustedIncome(int64_t v) { return Call(FC4_ORD_FORCE_MONTHLY_ADJUSTED_INCOME, v) != 0; }

bool cFC4Ordinance::Write(cIGZOStream& s) {
    fc4::OStreamAdapter a(s);
    return fc4_ordinance_write(core_, a.Get());
}

bool cFC4Ordinance::Read(cIGZIStream& s) {
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_ordinance_read(core_, a.Get());
    FC4_LOGV("ordinance 0x%08X Read -> %d (id 0x%08X)", classId_, ok ? 1 : 0, GetID());
    return ok;
}

uint32_t cFC4Ordinance::GetGZCLSID() { return classId_; }
