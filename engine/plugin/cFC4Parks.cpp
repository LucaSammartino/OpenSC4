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

#include "cFC4Parks.h"

#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Civic.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4BudgetSimulator.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4DepartmentBudget.h"
#include "cISC4LineItem.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4Occupant.h"
#include "cISC4Simulator.h"
#include "cISCPropertyHolder.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <string>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cIGZSerializable = 0xE4FDA3D4;
constexpr uint32_t kIID_cISC4BuildingOccupant = 0x87DDA3A9;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;

// Messages (§3).
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgOccupantInserted = 0x99EF1142;
constexpr uint32_t kMsgOccupantRemoved = 0x99EF1143;
constexpr uint32_t kMsgZooEscape = 0xAA667CE2;

// The tuning exemplar (§6).
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0x67CD5FA1;
constexpr uint32_t kTuningInstance = 0x8A4805BC;

fc4::ParkContext* Ctx(void* u) { return static_cast<fc4::ParkContext*>(u); }

cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cFC4ISC4Lot* LotOf(uint64_t t) { return t ? fc4::AsGame(reinterpret_cast<cISC4Lot*>(FromToken(t))) : nullptr; }
cISC4LineItem* ItemOf(uint64_t t) { return reinterpret_cast<cISC4LineItem*>(FromToken(t)); }
cFC4ISC4DepartmentBudget* DeptOf(uint64_t t) {
    return t ? fc4::AsGame(reinterpret_cast<cISC4DepartmentBudget*>(FromToken(t))) : nullptr;
}
cISCPropertyHolder* HolderOf(uint64_t occ) { return occ ? OccOf(occ)->AsPropertyHolder() : nullptr; }  // slot 6

template <class T>
T* Query(cIGZUnknown* u, uint32_t iid) {
    if (u == nullptr) return nullptr;
    void* p = nullptr;
    if (!u->QueryInterface(iid, &p) || p == nullptr) return nullptr;
    T* t = static_cast<T*>(p);
    if (fc4::IsOurObject(t)) {
        // Ours: a plain reference (released with Drop below the same way).
    } else {
        FC4_ACQUIRED(reinterpret_cast<cIGZUnknown*>(t), "QueryInterface");
    }
    return t;
}

template <class T>
void Drop(T* p) {
    if (p == nullptr) return;
    cIGZUnknown* u = reinterpret_cast<cIGZUnknown*>(p);
    if (fc4::IsOurObject(u)) {
        u->Release();
    } else {
        FC4_RELEASE(u);
    }
}

void HoldObject(cIGZUnknown* o) {
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->AddRef();
    } else {
        FC4_ADDREF(o);
    }
}

void DropObject(cIGZUnknown* o) {
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->Release();
    } else {
        FC4_RELEASE(o);
    }
}

cISC4City* AppCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    FC4_ACQUIRED(sc4, "QueryInterface");
    cISC4City* city = sc4->GetCity();
    FC4_RELEASE(sc4);
    return city;
}

// The city the manager holds (§4.1 step 2), else the application's.
cISC4City* CityOf(fc4::ParkContext* c) { return c->city ? c->city : AppCity(); }

cFC4ISC4BudgetSimulator* BudgetOf(fc4::ParkContext* c) {
    cISC4BudgetSimulator* b = c->budget;
    if (b == nullptr) {
        cISC4City* city = CityOf(c);
        b = city ? city->GetBudgetSimulator() : nullptr;  // borrowed
    }
    return b ? fc4::AsGame(b) : nullptr;
}

cFC4ISC4LotManager* LotManagerOf(fc4::ParkContext* c) {
    cISC4LotManager* lm = c->lotManager;
    if (lm == nullptr) {
        cISC4City* city = CityOf(c);
        lm = city ? city->GetLotManager() : nullptr;  // borrowed
    }
    return lm ? fc4::AsGame(lm) : nullptr;
}

// ---- host callbacks (fc4_park_host) --------------------------------------------------------------

void AddAgent(void* u, uint32_t level, const char* name, uint32_t flags) {
    fc4::ParkContext* c = Ctx(u);
    cISC4City* city = CityOf(c);
    cISC4Simulator* s = city ? city->GetSimulator() : nullptr;
    if (s == nullptr || c->target == nullptr || name == nullptr) {
        fc4::Log("parks: no simulator; the month agent is not registered");
        return;
    }
    const cRZBaseString n(name);
    fc4::AsGame(s)->AddAgent(c->target, level, n, flags);
    c->agentRegistered = true;
}

void RemoveAgent(void* u) {
    fc4::ParkContext* c = Ctx(u);
    if (!c->agentRegistered) return;
    cISC4City* city = CityOf(c);
    cISC4Simulator* s = city ? city->GetSimulator() : nullptr;
    if (s != nullptr) fc4::AsGame(s)->RemoveAgentAllLevels(c->target);
    c->agentRegistered = false;
}

bool AcquireMessageServer(void* u) {
    fc4::ParkContext* c = Ctx(u);
    if (c->messageServer != nullptr) return true;
    cIGZMessageServer2Ptr ms;
    cIGZMessageServer2* p = ms;
    if (p == nullptr) return false;
    HoldObject(p);
    c->messageServer = p;
    return true;
}

void ReleaseMessageServer(void* u) {
    fc4::ParkContext* c = Ctx(u);
    DropObject(c->messageServer);
    c->messageServer = nullptr;
}

void Subscribe(void* u, uint32_t type) {
    fc4::ParkContext* c = Ctx(u);
    if (c->messageServer != nullptr && c->target != nullptr && c->messageServer->AddNotification(c->target, type)) {
        ++c->subscriptions;
    }
}

void Unsubscribe(void* u, uint32_t type) {
    fc4::ParkContext* c = Ctx(u);
    if (c->messageServer != nullptr && c->target != nullptr &&
        c->messageServer->RemoveNotification(c->target, type) && c->subscriptions > 0) {
        --c->subscriptions;
    }
}

bool AcquireCity(void* u) {
    fc4::ParkContext* c = Ctx(u);
    if (c->city != nullptr) return true;
    cISC4City* city = AppCity();
    if (city == nullptr) return false;
    HoldObject(city);
    c->city = city;
    return true;
}

void ReleaseCity(void* u) {
    fc4::ParkContext* c = Ctx(u);
    DropObject(c->city);
    c->city = nullptr;
}

bool AcquireBudget(void* u) {
    fc4::ParkContext* c = Ctx(u);
    if (c->budget != nullptr) return true;
    cISC4City* city = CityOf(c);
    cISC4BudgetSimulator* b = city ? city->GetBudgetSimulator() : nullptr;
    if (b == nullptr) return false;
    HoldObject(b);
    c->budget = b;
    return true;
}

void ReleaseBudget(void* u) {
    fc4::ParkContext* c = Ctx(u);
    DropObject(c->budget);
    c->budget = nullptr;
}

bool AcquireLotManager(void* u) {
    fc4::ParkContext* c = Ctx(u);
    if (c->lotManager != nullptr) return true;
    cISC4City* city = CityOf(c);
    cISC4LotManager* lm = city ? city->GetLotManager() : nullptr;
    if (lm == nullptr) return false;
    HoldObject(lm);
    c->lotManager = lm;
    return true;
}

void ReleaseLotManager(void* u) {
    fc4::ParkContext* c = Ctx(u);
    DropObject(c->lotManager);
    c->lotManager = nullptr;
}

void AddRefToken(void* u, uint64_t t) {
    if (t == 0) return;
    HoldObject(FromToken(t));
    ++Ctx(u)->holds;
}

void ReleaseToken(void* u, uint64_t t) {
    if (t == 0) return;
    DropObject(FromToken(t));
    if (Ctx(u)->holds > 0) --Ctx(u)->holds;
}

// A host object arrives from the stream with one reference (now the core's);
// one of ours arrives without one, so it is taken here (FC4Streams.h).
uint64_t ResolveRef(void* u, uint64_t t, uint32_t) {
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return 0;
    if (fc4::IsOurObject(o)) o->AddRef();
    ++Ctx(u)->holds;
    return t;
}

uint64_t DepartmentBudget(void* u, uint32_t id) {
    cFC4ISC4BudgetSimulator* b = BudgetOf(Ctx(u));
    return b ? TokenOf(b->GetDepartmentBudget(id)) : 0;  // slot 28, borrowed
}

uint64_t CreateDepartmentBudget(void* u, uint32_t id, uint32_t group) {
    cFC4ISC4BudgetSimulator* b = BudgetOf(Ctx(u));
    return b ? TokenOf(b->CreateDepartmentBudget(id, group)) : 0;  // slot 25, borrowed
}

void SetDepartmentName(void*, uint64_t dept, uint32_t group, uint32_t instance) {
    if (cFC4ISC4DepartmentBudget* d = DeptOf(dept)) d->SetDepartmentName(group, instance);  // slot 5
}

void SetMaxFunding(void*, uint64_t dept, float max) {
    if (cFC4ISC4DepartmentBudget* d = DeptOf(dept)) {
        SC4Percentage p;
        p.percentage = max;
        d->SetMaxAllowedFundingPercentage(p);  // slot 13
    }
}

bool DepartmentFunding(void*, uint64_t dept, float* out) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    SC4Percentage* p = d ? d->GetFundingPercentage() : nullptr;  // slot 10, a pointer
    if (p == nullptr) return false;
    if (out != nullptr) *out = p->percentage;
    return true;
}

void ToItem(const fc4_budget_item& in, cFC4ISC4BudgetSimulator::BudgetItem* out) {
    out->departmentID = in.department_id;
    out->lineItem = in.line_item;
    out->purpose = in.purpose;
    out->index = in.index;
    out->cost = in.cost;
    out->localFundingPercentage = in.local_funding;
}

bool BudgetItemOf(void* u, uint64_t occ, uint32_t purpose, fc4_budget_item* out) {
    cFC4ISC4BudgetSimulator* b = BudgetOf(Ctx(u));
    cISCPropertyHolder* h = HolderOf(occ);
    if (b == nullptr || h == nullptr) return false;
    cFC4ISC4BudgetSimulator::BudgetItem item{};
    if (!b->GetBudgetItemForPurpose(h, purpose, item)) return false;  // slot 51
    if (out != nullptr) {
        out->department_id = item.departmentID;
        out->line_item = item.lineItem;
        out->purpose = item.purpose;
        out->index = item.index;
        out->cost = item.cost;
        out->local_funding = item.localFundingPercentage;
        out->reserved = 0;
    }
    return true;
}

void ChangeBudgetItemLine(void* u, uint64_t occ, int32_t index, uint32_t line) {
    cFC4ISC4BudgetSimulator* b = BudgetOf(Ctx(u));
    cISCPropertyHolder* h = HolderOf(occ);
    if (b != nullptr && h != nullptr) b->ChangeBudgetItemLine(h, static_cast<uint32_t>(index), line);  // slot 52
}

uint64_t LineItemFromBudgetItem(void* u, const fc4_budget_item* in) {
    cFC4ISC4BudgetSimulator* b = BudgetOf(Ctx(u));
    if (b == nullptr || in == nullptr) return 0;
    cFC4ISC4BudgetSimulator::BudgetItem item{};
    ToItem(*in, &item);
    return TokenOf(b->GetLineItemFromBudgetItem(item));  // slot 57, borrowed
}

uint64_t LineItem(void*, uint64_t dept, uint32_t line) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    return d ? TokenOf(d->GetLineItem(line)) : 0;  // slot 20, borrowed
}

uint64_t CreateLineItem(void*, uint64_t dept, uint32_t line, bool local) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    return d ? TokenOf(d->CreateLineItemForBuildingType(line, local)) : 0;  // slot 18, borrowed
}

void SetLineDisplayFlag(void*, uint64_t line, uint32_t flag, bool on) {
    if (line != 0) ItemOf(line)->SetDisplayFlag(static_cast<cISC4LineItem::DisplayFlag>(flag), on);
}

void AddToFullExpenses(void*, uint64_t line, int64_t amount) {
    if (line != 0) ItemOf(line)->AddToFullExpenses(amount);  // slot 13
}

int64_t LineSecondary(void*, uint64_t line) { return line ? ItemOf(line)->GetSecondaryInfoField() : 0; }

void SetLineSecondary(void*, uint64_t line, int64_t value) {
    if (line != 0) ItemOf(line)->SetSecondaryInfoField(value);
}

uint32_t OccupantType(void*, uint64_t occ) { return occ ? static_cast<uint32_t>(OccOf(occ)->GetType()) : 0; }

bool OccupantHasProperty(void*, uint64_t occ, uint32_t id) {
    cISCPropertyHolder* h = HolderOf(occ);
    return h != nullptr && fc4::props::Has(h, id);
}

bool IsOccupantGroup(void*, uint64_t occ, uint32_t group) { return occ != 0 && OccOf(occ)->IsOccupantGroup(group); }

bool IsBuildingOccupant(void*, uint64_t occ) {
    cISC4BuildingOccupant* b = Query<cISC4BuildingOccupant>(FromToken(occ), kIID_cISC4BuildingOccupant);
    Drop(b);
    return b != nullptr;
}

uint32_t BuildingType(void*, uint64_t occ) {
    cISC4BuildingOccupant* b = Query<cISC4BuildingOccupant>(FromToken(occ), kIID_cISC4BuildingOccupant);
    if (b == nullptr) return 0;
    const uint32_t t = b->GetBuildingType();  // slot 4
    Drop(b);
    return t;
}

// §4.4 step 4: the lot at the cell city.PositionToCell gives for the
// occupant's position (lot manager slot 6, GetLot(x, z, false)).
uint64_t OccupantPositionLot(void* u, uint64_t occ) {
    fc4::ParkContext* c = Ctx(u);
    cISC4City* city = CityOf(c);
    cFC4ISC4LotManager* lm = LotManagerOf(c);
    if (occ == 0 || city == nullptr || lm == nullptr) return 0;
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);  // slot 8
    int cx = 0, cz = 0;
    city->PositionToCell(v.fX, v.fZ, cx, cz);  // slot 95
    return TokenOf(lm->GetLot(cx, cz, false));  // borrowed
}

void SetLotCondition(void* u, uint64_t lot, uint32_t condition) {
    if (cFC4ISC4Lot* l = LotOf(lot)) {
        l->SetCondition(condition);  // slot 36
        ++Ctx(u)->lotWrites;
    }
}

// §4.5 step 2: a standard message, void1 = the manager, IGZUnknown = the zoo.
void PostZooEscape(void* u, uint64_t manager, uint64_t zoo) {
    fc4::ParkContext* c = Ctx(u);
    ++c->escapes;
    cIGZMessageServer2* ms = c->messageServer;
    cIGZMessageServer2Ptr fallback;
    if (ms == nullptr) ms = fallback;
    if (ms == nullptr) {
        fc4::Log("parks: zoo escape at %p: no message server; ZooEscape not posted",
                 static_cast<void*>(FromToken(zoo)));
        return;
    }
    cFC4Message2Standard* msg = cFC4Message2Standard::New();
    msg->AddRef();
    msg->SetType(kMsgZooEscape);
    msg->SetVoid1(FromToken(manager));
    msg->SetIGZUnknown(FromToken(zoo));
    const bool ok = ms->MessagePost(msg, false);
    msg->Release();
    if (ok) ++c->posts;
    fc4::Log("parks: zoo escape at %p: ZooEscape 0x%08X posted -> %d", static_cast<void*>(FromToken(zoo)),
             kMsgZooEscape, ok ? 1 : 0);
}

void AddControllers(void*, const char* name, uint64_t occ) { fc4::AddOccupantGroupControllers("parks", name, occ); }

std::string DateText(uint32_t julian) {
    if (julian == 0) return "0000-00-00";
    fc4_date d{};
    fc4_date_from_julian(static_cast<int32_t>(julian), &d);
    char b[24];
    snprintf(b, sizeof(b), "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
    return b;
}

// The canonical token of an occupant: its cISC4Occupant interface. The
// reference the query took is given back at once.
uint64_t CanonicalOccupant(cIGZUnknown* u) {
    cISC4Occupant* o = Query<cISC4Occupant>(u, kIID_cISC4Occupant);
    if (o == nullptr) return 0;
    const uint64_t t = TokenOf(o);
    Drop(o);
    return t;
}

}  // namespace

fc4_park_host fc4::MakeParkHost(ParkContext* ctx) {
    fc4_park_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    // Reads and holds (both tables).
    h.acquire_city = &AcquireCity;
    h.release_city = &ReleaseCity;
    h.acquire_budget = &AcquireBudget;
    h.release_budget = &ReleaseBudget;
    h.acquire_lot_manager = &AcquireLotManager;
    h.release_lot_manager = &ReleaseLotManager;
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.resolve_ref = &ResolveRef;
    h.department_budget = &DepartmentBudget;
    h.department_funding = &DepartmentFunding;
    h.budget_item = &BudgetItemOf;
    h.line_item_from_budget_item = &LineItemFromBudgetItem;
    h.line_item = &LineItem;
    h.line_secondary = &LineSecondary;
    h.occupant_type = &OccupantType;
    h.occupant_has_property = &OccupantHasProperty;
    h.is_occupant_group = &IsOccupantGroup;
    h.is_building_occupant = &IsBuildingOccupant;
    h.building_type = &BuildingType;
    h.occupant_position_lot = &OccupantPositionLot;
    if (ctx->readOnly) return h;
    h.add_agent = &AddAgent;
    h.remove_agent = &RemoveAgent;
    h.acquire_message_server = &AcquireMessageServer;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.release_message_server = &ReleaseMessageServer;
    h.create_department_budget = &CreateDepartmentBudget;
    h.set_department_name = &SetDepartmentName;
    h.set_max_funding = &SetMaxFunding;
    h.change_budget_item_line = &ChangeBudgetItemLine;
    h.create_line_item = &CreateLineItem;
    h.set_line_display_flag = &SetLineDisplayFlag;
    h.add_to_full_expenses = &AddToFullExpenses;
    h.set_line_secondary = &SetLineSecondary;
    h.set_lot_condition = &SetLotCondition;
    h.post_zoo_escape = &PostZooEscape;
    h.add_controllers = &AddControllers;
    return h;
}

// [choice] The session seed S, as civic's (foundations §6.3).
uint32_t fc4::ParkSessionSeed(bool* fixed) {
    const std::string s = fc4::Setting("parks_seed", "FC4_PARKS_SEED");
    if (!s.empty()) {
        char* end = nullptr;
        const unsigned long v = std::strtoul(s.c_str(), &end, 0);
        if (end != s.c_str()) {
            *fixed = true;
            return static_cast<uint32_t>(v);
        }
    }
    *fixed = false;
    constexpr int64_t kRzTimeUnixOffset = 2177452800;  // 1901-01-01 to 1970-01-01
    return static_cast<uint32_t>(static_cast<int64_t>(std::time(nullptr)) + kRzTimeUnixOffset);
}

fc4_props* fc4::ParkTuning() {
    fc4::props::Exemplar ex;
    if (!ex.Load(kTuningType, kTuningGroup, kTuningInstance)) return nullptr;
    return fc4::props::ToCoreAll(ex.Holder());
}

// ---- the manager ---------------------------------------------------------------------------------

cIGZUnknown* CreateFC4ParkManager() {
    return static_cast<cIGZUnknown*>(static_cast<cISC4ParkManager*>(new cFC4ParkManager()));
}

cFC4ParkManager::cFC4ParkManager() {
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    host_ = fc4::MakeParkHost(&ctx_);
    seed_ = fc4::ParkSessionSeed(&seedFixed_);
    core_ = fc4_park_create(&host_, seed_, TokenOf(static_cast<cISC4ParkManager*>(this)));
    if (core_ == nullptr) fc4::Log("cFC4ParkManager: core creation failed");
    fc4::Log("cFC4ParkManager(%p) created, core %p, session seed %u (%s)", static_cast<void*>(this),
             static_cast<void*>(core_), seed_, seedFixed_ ? "parks_seed" : "cRZTime");
}

cFC4ParkManager::~cFC4ParkManager() {
    // The core's lists and holds are released by Shutdown (Release runs it);
    // a loaded object that was never initialised keeps its list references
    // (question PKG-2).
    fc4_park_destroy(core_);
    core_ = nullptr;
    RemoveAgent(&ctx_);
    fc4::Log("cFC4ParkManager destroyed (%u occupant messages, %u lot writes, %u escapes)", occupantMsgs_,
             ctx_.lotWrites, ctx_.escapes);
}

// §1: 1 and 0xAA3FE6BC give the primary interface, 0x452294AA the message
// target, 0xE4FDA3D4 the serializable; anything else is false.
bool cFC4ParkManager::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case GZIID_cIGZUnknown:
        case kFC4_IID_cISC4ParkManager:
            *ppvObj = static_cast<cISC4ParkManager*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case kIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        default:
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4ParkManager::AddRef() { return ++refCount_; }

// §8: above 1 decrement; at 1 or less Shutdown, destroy, return 0 (without
// decrementing first).
uint32_t cFC4ParkManager::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

void cFC4ParkManager::LogState(const char* what, bool ok) {
    fc4_park_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_park_get_state(core_, &st)) {
        fc4::Log("parks %s: ok=%d (no state)", what, ok ? 1 : 0);
        return;
    }
    fc4::Log("parks %s: ok=%d init=%u parks=%u zoos=%u threshold=%g curve=%u dept=%p latch=%u held=%u%u%u%u "
             "subscriptions=%u agent=%d holds=%u",
             what, ok ? 1 : 0, st.initialized, st.parks, st.zoos, static_cast<double>(st.distress_threshold),
             st.curve_points, static_cast<void*>(FromToken(st.department)), st.underfunded, st.city_held,
             st.budget_held, st.lot_manager_held, st.message_server_held, ctx_.subscriptions,
             ctx_.agentRegistered ? 1 : 0, ctx_.holds);
}

// ---- lifecycle (§4.1) ----------------------------------------------------------------------------

bool cFC4ParkManager::Init() {
    FC4_TRACE("ParkManager::Init");
    bool ok = false;
    if (core_ != nullptr) {
        fc4_park_state st{};
        st.struct_size = sizeof(st);
        fc4_park_get_state(core_, &st);
        // The tuning is read at the object's first Init only (§4.1 step 1).
        fc4::props::CoreProps tuning(st.tuning_read ? nullptr : fc4::ParkTuning());
        ok = fc4_park_init(core_, tuning.Get());
    }
    LogState("Init", ok);
    return true;  // §8: AL (1)
}

bool cFC4ParkManager::Shutdown() {
    FC4_TRACE("ParkManager::Shutdown");
    if (!destroying_) AddRef();
    if (core_ != nullptr) fc4_park_shutdown(core_);
    LogState("Shutdown", true);
    if (!destroying_) Release();
    return true;
}

// ---- messages (§3) -------------------------------------------------------------------------------

bool cFC4ParkManager::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("parks DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    const uint32_t type = pMessage->GetType();
    if (type != kMsgSimNewMonth && type != kMsgOccupantInserted && type != kMsgOccupantRemoved) return true;
    uint32_t d[3] = {};
    cIGZUnknown* void1 = nullptr;
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    if (s != nullptr) {
        d[0] = static_cast<uint32_t>(s->GetData1());
        d[1] = static_cast<uint32_t>(s->GetData2());
        d[2] = static_cast<uint32_t>(s->GetData3());
        void1 = static_cast<cIGZUnknown*>(s->GetVoid1());
    }
    if (type == kMsgSimNewMonth) {
        const uint32_t lotsBefore = ctx_.lotWrites, escapesBefore = ctx_.escapes;
        fc4_park_do_message(core_, type, d[0], d[1], d[2], 0);
        fc4_park_state st{};
        st.struct_size = sizeof(st);
        fc4_park_get_state(core_, &st);
        float f = -1.0f;
        if (st.department != 0) DepartmentFunding(&ctx_, st.department, &f);
        fc4::Log("parks month %s: parks=%u zoos=%u funding=%g latch=%u lot-writes=%u escapes=%u occupants=%u",
                 DateText(d[0]).c_str(), st.parks, st.zoos, static_cast<double>(f), st.underfunded,
                 ctx_.lotWrites - lotsBefore, ctx_.escapes - escapesBefore, occupantMsgs_);
    } else {
        ++occupantMsgs_;
        fc4_park_do_message(core_, type, d[0], d[1], d[2], CanonicalOccupant(void1));
    }
    Drop(s);
    return true;  // §3
}

// ---- persistence (§7) ----------------------------------------------------------------------------

bool cFC4ParkManager::Write(cIGZOStream& s) {
    FC4_TRACE("ParkManager::Write");
    fc4::OStreamAdapter a(s);
    const bool ok = core_ != nullptr && fc4_park_write(core_, a.Get());
    fc4_park_state st{};
    st.struct_size = sizeof(st);
    if (core_ != nullptr) fc4_park_get_state(core_, &st);
    fc4::Log("parks Write -> %d (parks=%u zoos=%u)", ok ? 1 : 0, st.parks, st.zoos);
    return ok;
}

bool cFC4ParkManager::Read(cIGZIStream& s) {
    FC4_TRACE("ParkManager::Read");
    fc4::IStreamAdapter a(s);
    const bool ok = core_ != nullptr && fc4_park_read(core_, a.Get());
    LogState("Read", ok);
    return ok;
}

uint32_t cFC4ParkManager::GetGZCLSID() { return kFC4_CLSID_cSC4ParkManager; }

// ---- test seam op 57 -----------------------------------------------------------------------------

intptr_t fc4::ParksTestZooEscape(const FC4ParksTestInput* in) {
    if (in == nullptr) return -1;
    fc4::ParkContext ctx;
    ctx.messageServer = in->server;  // borrowed for the call
    PostZooEscape(&ctx, TokenOf(in->manager), TokenOf(in->zoo));
    ctx.messageServer = nullptr;
    return static_cast<intptr_t>(ctx.posts);
}
