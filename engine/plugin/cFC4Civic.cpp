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

#include "cFC4Civic.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"

#include "GZServPtrs.h"
#include "cGZPersistResourceKey.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4BudgetSimulator.h"
#include "cISC4BuildingDevelopmentSimulator.h"
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

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdlib>
#include <cstring>
#include <ctime>

using fc4::FromToken;
using fc4::TokenOf;

class cISC4AutomataControllerManager;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cIGZSerializable = 0xE4FDA3D4;
constexpr uint32_t kIID_cISC4BuildingOccupant = 0x87DDA3A9;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4Lot = 0xE818FCED;

// Messages (§3).
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgOccupantInserted = 0x99EF1142;
constexpr uint32_t kMsgOccupantRemoved = 0x99EF1143;
constexpr uint32_t kMsgLotStateChanged = 0xE8BDA2E7;
constexpr uint32_t kOccupantTypeBuilding = 0x278128A0;
// Building properties (§6).
constexpr uint32_t kPropConditional = 0xEA3209F8;
constexpr uint32_t kPropExclusion = 0xEA2E078B;
// [choice] A tooltip longer than this is cut when read from a caller (slot 7)
// or a mirror entry (a corrupt string pointer must not run away).
constexpr size_t kMaxText = 1u << 16;

// The automata controller manager (docs/abi/interfaces.tsv,
// cSC4AutomataControllerManager): slot 10 is AddControllersFromOccupantGroup
// with the group name as a cRZString (civic §4.6 step 1). Slots 5-9 are
// placeholders with the table's argument dwords (never called).
class cFC4ISC4AutomataControllerManager : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                    // 3
    virtual bool Shutdown() = 0;                                                // 4
    virtual bool Slot05(uint32_t a, uint32_t b, uint32_t c) = 0;                // 5
    virtual bool Slot06(uint32_t a, uint32_t b, uint32_t c) = 0;                // 6
    virtual bool Slot07(uint32_t a) = 0;                                        // 7
    virtual bool Slot08(uint32_t a, uint32_t b) = 0;                            // 8
    virtual bool Slot09(uint32_t a, uint32_t b, uint32_t c) = 0;                // 9
    virtual bool AddControllersFromOccupantGroup(const fc4::GameString& name, cISC4Occupant* occ,
                                                 float* unknown) = 0;           // 10
};

fc4::CivicContext* Ctx(void* u) { return static_cast<fc4::CivicContext*>(u); }

cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cFC4ISC4Lot* LotOf(uint64_t t) { return t ? fc4::AsGame(reinterpret_cast<cISC4Lot*>(FromToken(t))) : nullptr; }
cISC4LineItem* ItemOf(uint64_t t) { return reinterpret_cast<cISC4LineItem*>(FromToken(t)); }
cFC4ISC4DepartmentBudget* DeptOf(uint64_t t) {
    return t ? fc4::AsGame(reinterpret_cast<cISC4DepartmentBudget*>(FromToken(t))) : nullptr;
}
cISCPropertyHolder* HolderOf(uint64_t occ) { return occ ? OccOf(occ)->AsPropertyHolder() : nullptr; }  // slot 6

// QueryInterface for `iid`; the result (one reference) or nullptr.
template <class T>
T* Query(cIGZUnknown* u, uint32_t iid) {
    if (u == nullptr) return nullptr;
    void* p = nullptr;
    if (!u->QueryInterface(iid, &p) || p == nullptr) return nullptr;
    T* t = static_cast<T*>(p);
    FC4_ACQUIRED(reinterpret_cast<cIGZUnknown*>(t), "QueryInterface");
    return t;
}

template <class T>
void Drop(T* p) {
    if (p != nullptr) FC4_RELEASE(reinterpret_cast<cIGZUnknown*>(p));
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

cFC4ISC4BudgetSimulator* Budget() {
    cISC4City* city = AppCity();
    cISC4BudgetSimulator* b = city ? city->GetBudgetSimulator() : nullptr;  // borrowed
    return b ? fc4::AsGame(b) : nullptr;
}

// The canonical token of an occupant: its cISC4Occupant interface (one token
// stands for a building and its occupant). The reference the query took is
// given back at once: the caller's object keeps it alive.
uint64_t CanonicalOccupant(cIGZUnknown* u) {
    cISC4Occupant* o = Query<cISC4Occupant>(u, kIID_cISC4Occupant);
    if (o == nullptr) return 0;
    const uint64_t t = TokenOf(o);
    Drop(o);
    return t;
}

// ---- host callbacks (fc4_civic_host) -------------------------------------------------------------

void AddAgent(void* u, uint32_t level, const char* name, uint32_t flags) {
    fc4::CivicContext* c = Ctx(u);
    cISC4City* city = AppCity();
    cISC4Simulator* s = city ? city->GetSimulator() : nullptr;
    if (s == nullptr || c->target == nullptr || name == nullptr) {
        fc4::Log("civic: no simulator; the month agent is not registered");
        return;
    }
    const cRZBaseString n(name);
    fc4::AsGame(s)->AddAgent(c->target, level, n, flags);
    c->agentRegistered = true;
}

void RemoveAgent(void* u) {
    fc4::CivicContext* c = Ctx(u);
    if (!c->agentRegistered) return;
    cISC4City* city = AppCity();
    cISC4Simulator* s = city ? city->GetSimulator() : nullptr;
    if (s != nullptr) fc4::AsGame(s)->RemoveAgentAllLevels(c->target);
    c->agentRegistered = false;
}

void Subscribe(void* u, uint32_t type) {
    fc4::CivicContext* c = Ctx(u);
    cIGZMessageServer2Ptr ms;
    if (ms && c->target != nullptr && ms->AddNotification(c->target, type)) ++c->subscriptions;
}

void Unsubscribe(void* u, uint32_t type) {
    fc4::CivicContext* c = Ctx(u);
    cIGZMessageServer2Ptr ms;
    if (ms && c->target != nullptr && ms->RemoveNotification(c->target, type) && c->subscriptions > 0) {
        --c->subscriptions;
    }
}

// §4.1 step 6. [choice] (question CIV-1) The `civic` Lua table is not
// published: the game-data registry call entries it would need
// (lua-host.md §9.4, cIGZCommandParameterSet) have no ABI rows yet.
void RegisterLua(void* u) {
    fc4::CivicContext* c = Ctx(u);
    if (!c->luaNoted) {
        c->luaNoted = true;
        fc4::Log("civic: Lua civic.get_landmark_count() NOT published (no registry call entry yet); "
                 "GetLandmarkCount (slot 8) answers");
    }
}

void UnregisterLua(void*) {}

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
// one of ours (our department, when the budget subsystem is on) arrives
// without one, so it is taken here (FC4Streams.h, OurObjectRef).
uint64_t ResolveRef(void* u, uint64_t t, uint32_t) {
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return 0;
    if (fc4::IsOurObject(o)) o->AddRef();
    ++Ctx(u)->holds;
    return t;
}

uint64_t DepartmentBudget(void*, uint32_t id) {
    cFC4ISC4BudgetSimulator* b = Budget();
    return b ? TokenOf(b->GetDepartmentBudget(id)) : 0;  // slot 28, borrowed
}

uint64_t CreateDepartmentBudget(void*, uint32_t id, uint32_t group) {
    cFC4ISC4BudgetSimulator* b = Budget();
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

bool BudgetItem(void*, uint64_t occ, uint32_t purpose, fc4_budget_item* out) {
    cFC4ISC4BudgetSimulator* b = Budget();
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

void ChangeBudgetItemLine(void*, uint64_t occ, int32_t index, uint32_t line) {
    cFC4ISC4BudgetSimulator* b = Budget();
    cISCPropertyHolder* h = HolderOf(occ);
    if (b != nullptr && h != nullptr) b->ChangeBudgetItemLine(h, static_cast<uint32_t>(index), line);  // slot 52
}

void RemoveBudgetItem(void*, uint64_t occ, int32_t index) {
    cFC4ISC4BudgetSimulator* b = Budget();
    cISCPropertyHolder* h = HolderOf(occ);
    if (b != nullptr && h != nullptr) b->RemoveBudgetItemFromBudget(h, static_cast<uint32_t>(index));  // slot 56
}

uint64_t LineItem(void*, uint64_t dept, uint32_t line) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    return d ? TokenOf(d->GetLineItem(line)) : 0;  // slot 20, borrowed
}

uint64_t CreateLineItem(void*, uint64_t dept, uint32_t line, bool local) {
    cFC4ISC4DepartmentBudget* d = DeptOf(dept);
    return d ? TokenOf(d->CreateLineItemForBuildingType(line, local)) : 0;  // slot 18, borrowed
}

void RemoveLineItem(void*, uint64_t dept, uint32_t line) {
    if (cFC4ISC4DepartmentBudget* d = DeptOf(dept)) d->RemoveLineItem(line);  // slot 19
}

void AddToFullExpenses(void*, uint64_t line, int64_t amount) {
    if (line != 0) ItemOf(line)->AddToFullExpenses(amount);  // slot 13
}

void AddToIncome(void*, uint64_t line, int64_t amount) {
    if (line != 0) ItemOf(line)->AddToIncome(amount);  // slot 16
}

int64_t LineIncome(void*, uint64_t line) { return line ? ItemOf(line)->GetIncome() : 0; }  // slot 14, EDX:EAX

void SetLineIncome(void*, uint64_t line, int64_t income) {
    if (line != 0) ItemOf(line)->SetIncome(income);  // slot 15
}

void SetLineType(void*, uint64_t line, uint32_t type) {
    if (line != 0) ItemOf(line)->SetType(static_cast<cISC4LineItem::Type>(type));  // slot 5
}

uint32_t OccupantType(void*, uint64_t occ) { return occ ? static_cast<uint32_t>(OccOf(occ)->GetType()) : 0; }

bool OccupantU32(uint64_t occ, uint32_t id, uint32_t* out) {
    cISCPropertyHolder* h = HolderOf(occ);
    if (h == nullptr) return false;
    std::vector<uint32_t> v;
    if (!fc4::props::GetU32s(h, id, v) || v.empty()) return false;
    if (out != nullptr) *out = v[0];
    return true;
}

bool OccupantBoolProperty(void*, uint64_t occ, uint32_t id, bool* out) {
    uint32_t v = 0;
    if (!OccupantU32(occ, id, &v)) return false;
    if (out != nullptr) *out = v != 0;
    return true;
}

bool OccupantU32Property(void*, uint64_t occ, uint32_t id, uint32_t* out) { return OccupantU32(occ, id, out); }

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

bool BuildingKeyInstance(void* u, uint64_t occ, uint32_t* instance) {
    cISC4City* city = AppCity();
    cISC4BuildingDevelopmentSimulator* bds = city ? city->GetBuildingDevelopmentSimulator() : nullptr;
    if (bds == nullptr) return false;
    cGZPersistResourceKey key;
    if (!bds->GetBuildingKeyFromType(BuildingType(u, occ), key)) return false;  // slot 14
    if (instance != nullptr) *instance = key.instance;
    return true;
}

bool IsOccupantGroup(void*, uint64_t occ, uint32_t group) { return occ != 0 && OccOf(occ)->IsOccupantGroup(group); }

// §3: the lot-state message's IGZUnknown as a cISC4Lot (one reference, the
// core's; it releases it through `release`).
uint64_t MessageLot(void* u, uint64_t unknown) {
    cISC4Lot* lot = Query<cISC4Lot>(FromToken(unknown), kIID_cISC4Lot);
    if (lot != nullptr) ++Ctx(u)->holds;
    return TokenOf(lot);
}

// Lot slot 48, as the building's canonical occupant token.
uint64_t LotBuilding(void*, uint64_t lot) {
    cFC4ISC4Lot* l = LotOf(lot);
    cISC4BuildingOccupant* b = l ? l->GetBuilding() : nullptr;  // borrowed
    return b ? CanonicalOccupant(reinterpret_cast<cIGZUnknown*>(b)) : 0;
}

uint64_t OccupantLot(void*, uint64_t occ) {
    if (occ == 0) return 0;
    cISC4City* city = AppCity();
    cISC4LotManager* lm = city ? city->GetLotManager() : nullptr;
    return lm ? TokenOf(fc4::AsGame(lm)->GetOccupantLot(OccOf(occ))) : 0;  // slot 14, borrowed
}

uint32_t LotCondition(void*, uint64_t lot) {
    cFC4ISC4Lot* l = LotOf(lot);
    return l ? l->GetCondition() : 0;  // slot 35
}

void SetLotCondition(void*, uint64_t lot, uint32_t condition) {
    if (cFC4ISC4Lot* l = LotOf(lot)) l->SetCondition(condition);  // slot 36
}

// §4.6 step 1: AddControllersFromOccupantGroup("ape_escape", occ, 0), the name
// as a game cRZString.
void AddControllers(void*, const char* name, uint64_t occ) {
    cISC4City* city = AppCity();
    cISC4AutomataControllerManager* m = city ? city->GetAutomataControllerManager() : nullptr;  // slot 47
    if (m == nullptr || name == nullptr) return;
    fc4::GameString s{};
    if (!fc4::GameStringInit(s, name, std::strlen(name))) {
        fc4::Log("civic: ape escape: no game string allocator; no controllers added");
        return;
    }
    const bool ok = reinterpret_cast<cFC4ISC4AutomataControllerManager*>(m)->AddControllersFromOccupantGroup(
        s, OccOf(occ), nullptr);
    fc4::GameStringDestroy(s);
    fc4::Log("civic: ape escape at occupant %p: AddControllersFromOccupantGroup(\"%s\") -> %d",
             static_cast<void*>(OccOf(occ)), name, ok ? 1 : 0);
}

}  // namespace

fc4_civic_host fc4::MakeCivicHost(CivicContext* ctx) {
    fc4_civic_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    // Reads (both tables).
    h.department_budget = &DepartmentBudget;
    h.department_funding = &DepartmentFunding;
    h.budget_item = &BudgetItem;
    h.line_item = &LineItem;
    h.line_income = &LineIncome;
    h.occupant_type = &OccupantType;
    h.occupant_bool_property = &OccupantBoolProperty;
    h.occupant_u32_property = &OccupantU32Property;
    h.is_building_occupant = &IsBuildingOccupant;
    h.building_type = &BuildingType;
    h.building_key_instance = &BuildingKeyInstance;
    h.is_occupant_group = &IsOccupantGroup;
    h.message_lot = &MessageLot;
    h.lot_building = &LotBuilding;
    h.occupant_lot = &OccupantLot;
    h.lot_condition = &LotCondition;
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.resolve_ref = &ResolveRef;
    if (ctx->readOnly) return h;
    h.add_agent = &AddAgent;
    h.remove_agent = &RemoveAgent;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.register_lua = &RegisterLua;
    h.unregister_lua = &UnregisterLua;
    h.create_department_budget = &CreateDepartmentBudget;
    h.set_department_name = &SetDepartmentName;
    h.set_max_funding = &SetMaxFunding;
    h.change_budget_item_line = &ChangeBudgetItemLine;
    h.remove_budget_item = &RemoveBudgetItem;
    h.create_line_item = &CreateLineItem;
    h.remove_line_item = &RemoveLineItem;
    h.add_to_full_expenses = &AddToFullExpenses;
    h.add_to_income = &AddToIncome;
    h.set_line_income = &SetLineIncome;
    h.set_line_type = &SetLineType;
    h.set_lot_condition = &SetLotCondition;
    h.add_controllers = &AddControllers;
    return h;
}

// §5 [choice]: the session seed S, as crime's (foundations §6.3).
uint32_t fc4::CivicSessionSeed(bool* fixed) {
    const std::string s = fc4::Setting("civic_seed", "FC4_CIVIC_SEED");
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

// ---- the simulator ------------------------------------------------------------------------------

namespace {

std::string EntryText(const fc4::CivicStatusEntry& e) {
    if (e.textBegin == nullptr || e.textEnd < e.textBegin) return std::string();
    size_t n = static_cast<size_t>(e.textEnd - e.textBegin);
    if (n > kMaxText) n = kMaxText;
    return std::string(e.textBegin, n);
}

fc4::GameString& StringOf(fc4::CivicStatusEntry& e) {
    return *reinterpret_cast<fc4::GameString*>(&e.textVtable);
}

// The core's status `key`: false when it has none.
bool CoreStatus(fc4_civic* core, uint32_t key, uint8_t* status, uint16_t* count, std::string* text) {
    char buf[512];
    size_t len = 0;
    if (!fc4_civic_get_status(core, key, status, count, buf, sizeof(buf), &len)) return false;
    if (len < sizeof(buf)) {
        text->assign(buf, len);
        return true;
    }
    std::vector<char> big(len + 1);
    fc4_civic_get_status(core, key, status, count, big.data(), big.size(), &len);
    text->assign(big.data(), len);
    return true;
}

}  // namespace

cIGZUnknown* CreateFC4CivicBuildingSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cISC4CivicBuildingSimulator*>(new cFC4CivicBuildingSimulator()));
}

cFC4CivicBuildingSimulator::cFC4CivicBuildingSimulator() {
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    host_ = fc4::MakeCivicHost(&ctx_);
    seed_ = fc4::CivicSessionSeed(&seedFixed_);
    core_ = fc4_civic_create(&host_, seed_);
    if (core_ == nullptr) fc4::Log("cFC4CivicBuildingSimulator: core creation failed");
    fc4::Log("cFC4CivicBuildingSimulator(%p) created, core %p, session seed %u (%s)", static_cast<void*>(this),
             static_cast<void*>(core_), seed_, seedFixed_ ? "civic_seed" : "cRZTime");
}

cFC4CivicBuildingSimulator::~cFC4CivicBuildingSimulator() {
    fc4_civic_destroy(core_);  // the references a loaded, never-initialised object still holds
    core_ = nullptr;
    RemoveAgent(&ctx_);
    std::lock_guard<std::mutex> g(mirrorLock_);
    for (auto& kv : mirrors_) fc4::GameStringDestroy(StringOf(kv.second->game));
    mirrors_.clear();
    fc4::Log("cFC4CivicBuildingSimulator destroyed (%u occupant messages, %u lot messages, %u writebacks)",
             occupantMsgs_, lotMsgs_, writebacks_);
}

// §1: 1 and 0x8A41322F give the primary interface, 0x452294AA the message
// target, 0xE4FDA3D4 the serializable; anything else is false.
bool cFC4CivicBuildingSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case GZIID_cIGZUnknown:
        case kFC4_IID_cISC4CivicBuildingSimulator:
            *ppvObj = static_cast<cISC4CivicBuildingSimulator*>(this);
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

uint32_t cFC4CivicBuildingSimulator::AddRef() { return ++refCount_; }

// As the commercial simulator's (§2.3 there): above 1 decrement; at 1 or less
// Shutdown, destroy, return 0.
uint32_t cFC4CivicBuildingSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

cFC4CivicBuildingSimulator::Mirror* cFC4CivicBuildingSimulator::EnsureMirror(uint32_t key) {
    std::lock_guard<std::mutex> g(mirrorLock_);
    auto it = mirrors_.find(key);
    if (it != mirrors_.end()) return it->second.get();
    if (mirrorRefused_) return nullptr;
    auto m = std::make_unique<Mirror>();
    fc4::GameString s{};
    if (!fc4::GameStringInit(s, "", 0)) {
        mirrorRefused_ = true;
        fc4::Log("civic: the game's string allocator is not available (compat table): GetConditionalBuildingStatus "
                 "answers null");
        return nullptr;
    }
    std::memcpy(static_cast<void*>(&m->game.textVtable), &s, sizeof(s));
    Mirror* p = m.get();
    mirrors_.emplace(key, std::move(m));
    return p;
}

void cFC4CivicBuildingSimulator::FlushStatuses() {
    if (core_ == nullptr) return;
    std::vector<std::pair<uint32_t, Mirror*>> all;
    {
        std::lock_guard<std::mutex> g(mirrorLock_);
        all.reserve(mirrors_.size());
        for (auto& kv : mirrors_) all.emplace_back(kv.first, kv.second.get());
    }
    for (auto& [key, m] : all) {
        const std::string text = EntryText(m->game);
        if (m->game.status == m->syncedStatus && m->game.buildingCount == m->syncedCount && text == m->syncedText) {
            continue;
        }
        fc4_civic_update_status(core_, key, m->game.status, m->game.buildingCount, text.data(), text.size());
        m->syncedStatus = m->game.status;
        m->syncedCount = m->game.buildingCount;
        m->syncedText = text;
        ++writebacks_;
    }
}

void cFC4CivicBuildingSimulator::PullStatuses() {
    if (core_ == nullptr) return;
    const size_t n = fc4_civic_status_keys(core_, nullptr, 0);
    std::vector<uint32_t> keys(n);
    if (n != 0) fc4_civic_status_keys(core_, keys.data(), keys.size());
    std::string text;
    for (uint32_t key : keys) {
        uint8_t status = 0;
        uint16_t count = 0;
        if (!CoreStatus(core_, key, &status, &count, &text)) continue;
        Mirror* m = EnsureMirror(key);
        if (m == nullptr) return;
        if (m->syncedStatus == status && m->syncedCount == count && m->syncedText == text &&
            EntryText(m->game) == text) {
            continue;
        }
        m->game.status = status;
        m->game.buildingCount = count;
        if (EntryText(m->game) != text) {
            fc4::GameString s{};
            if (fc4::GameStringInit(s, text.data(), text.size())) {
                fc4::GameStringDestroy(StringOf(m->game));
                std::memcpy(static_cast<void*>(&m->game.textVtable), &s, sizeof(s));
            }
        }
        m->syncedStatus = status;
        m->syncedCount = count;
        m->syncedText = text;
    }
}

void cFC4CivicBuildingSimulator::PullOthers() {
    if (core_ == nullptr) return;
    const size_t n = fc4_civic_exclusions(core_, nullptr, 0);
    std::vector<uint32_t> ids(n);
    if (n != 0) fc4_civic_exclusions(core_, ids.data(), ids.size());
    {
        std::lock_guard<std::mutex> g(mirrorLock_);
        exclusions_.clear();
        exclusions_.insert(ids.begin(), ids.end());
    }
    landmarks_.store(fc4_civic_landmark_count(core_));
}

void cFC4CivicBuildingSimulator::LogState(const char* what, bool ok) {
    fc4_civic_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_civic_get_state(core_, &st)) {
        fc4::Log("civic %s: ok=%d (no state)", what, ok ? 1 : 0);
        return;
    }
    fc4::Log("civic %s: ok=%d init=%u exclusions=%u statuses=%u centers=%u maintained=%u temporaries=%u "
             "landmarks=%p subscriptions=%u agent=%d holds=%u mirrors=%u",
             what, ok ? 1 : 0, st.initialized, st.exclusions, st.statuses, st.disease_centers, st.maintained,
             st.temporaries, static_cast<void*>(FromToken(st.landmarks)), ctx_.subscriptions,
             ctx_.agentRegistered ? 1 : 0, ctx_.holds, static_cast<unsigned>(mirrors_.size()));
}

// ---- lifecycle (§4.1, §4.2) ----------------------------------------------------------------------

bool cFC4CivicBuildingSimulator::Init() {
    FC4_TRACE("CivicBuildingSimulator::Init");
    std::lock_guard<std::recursive_mutex> g(lock_);
    const bool ok = core_ != nullptr && fc4_civic_init(core_);
    PullStatuses();
    PullOthers();
    LogState("Init", ok);
    return true;  // §8: AL (1)
}

bool cFC4CivicBuildingSimulator::Shutdown() {
    FC4_TRACE("CivicBuildingSimulator::Shutdown");
    if (!destroying_) AddRef();
    {
        std::lock_guard<std::recursive_mutex> g(lock_);
        FlushStatuses();
        if (core_ != nullptr) fc4_civic_shutdown(core_);
        PullOthers();
        LogState("Shutdown", true);
    }
    if (!destroying_) Release();
    return true;
}

// ---- the interface (§4.7) ------------------------------------------------------------------------

bool cFC4CivicBuildingSimulator::IsBuildingExcluded(uint32_t id) {
    std::lock_guard<std::mutex> g(mirrorLock_);
    return exclusions_.count(id) != 0;
}

cISC4CivicBuildingSimulator::ConditionalBuildingStatus* cFC4CivicBuildingSimulator::GetConditionalBuildingStatus(
    uint32_t key) const {
    std::lock_guard<std::mutex> g(mirrorLock_);
    auto it = mirrors_.find(key);
    if (it == mirrors_.end()) return nullptr;
    return reinterpret_cast<ConditionalBuildingStatus*>(&it->second->game);
}

// Slot 7: find or insert, copy status, count and text. [choice] (question
// CIV-3) The caller's tooltip is read as a game cRZString (its begin and end
// pointers), the layout the SDK's SC4String shares. Returns true.
bool cFC4CivicBuildingSimulator::UpdateConditionalBuildingStatus(uint32_t key, ConditionalBuildingStatus* status) {
    if (status == nullptr || core_ == nullptr) return true;
    const auto* in = reinterpret_cast<const fc4::CivicStatusEntry*>(status);
    const uint8_t st = in->status;
    const uint16_t count = in->buildingCount;
    const std::string text = EntryText(*in);
    std::lock_guard<std::recursive_mutex> g(lock_);
    FlushStatuses();
    fc4_civic_update_status(core_, key, st, count, text.data(), text.size());
    PullStatuses();
    return true;
}

uint32_t cFC4CivicBuildingSimulator::GetLandmarkCount() { return landmarks_.load(); }

// ---- messages (§3) -------------------------------------------------------------------------------

bool cFC4CivicBuildingSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("civic DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    const uint32_t type = pMessage->GetType();
    if (type != kMsgSimNewMonth && type != kMsgOccupantInserted && type != kMsgOccupantRemoved &&
        type != kMsgLotStateChanged) {
        return true;
    }
    uint32_t d[3] = {};
    cIGZUnknown* void1 = nullptr;
    cIGZUnknown* unknown = nullptr;
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    if (s != nullptr) {
        d[0] = static_cast<uint32_t>(s->GetData1());
        d[1] = static_cast<uint32_t>(s->GetData2());
        d[2] = static_cast<uint32_t>(s->GetData3());
        void1 = static_cast<cIGZUnknown*>(s->GetVoid1());
        unknown = s->GetIGZUnknown();
    }
    std::lock_guard<std::recursive_mutex> g(lock_);
    if (type == kMsgOccupantInserted || type == kMsgOccupantRemoved) {
        ++occupantMsgs_;
        const uint64_t occ = CanonicalOccupant(void1);
        bool building = false, cond = false, excl = false;
        if (occ != 0 && OccOf(occ)->GetType() == kOccupantTypeBuilding) {
            building = true;
            cISCPropertyHolder* h = HolderOf(occ);
            cond = fc4::props::Has(h, kPropConditional);
            excl = fc4::props::Has(h, kPropExclusion);
            ++buildingMsgs_;
        }
        if (cond) FlushStatuses();
        fc4_civic_do_message(core_, type, d[0], d[1], d[2], occ, 0);
        if (cond) PullStatuses();
        if (excl || building) PullOthers();
    } else if (type == kMsgLotStateChanged) {
        ++lotMsgs_;
        const uint32_t before = landmarks_.load();
        fc4_civic_do_message(core_, type, d[0], d[1], d[2], 0, TokenOf(unknown));
        PullOthers();
        if (landmarks_.load() != before) ++activations_;
    } else {
        fc4_civic_do_message(core_, type, d[0], d[1], d[2], 0, 0);
        PullOthers();
        fc4_civic_state st{};
        st.struct_size = sizeof(st);
        fc4_civic_get_state(core_, &st);
        fc4_date dt{};
        if (d[0] > 0) fc4_date_from_julian(static_cast<int32_t>(d[0]), &dt);
        fc4::Log("civic month %04d-%02d-%02d: exclusions=%u statuses=%u centers=%u maintained=%u temporaries=%u "
                 "occupants=%u/%u lots=%u/%u writebacks=%u holds=%u",
                 dt.year, dt.month, dt.day_of_month, st.exclusions, st.statuses, st.disease_centers, st.maintained,
                 st.temporaries, buildingMsgs_, occupantMsgs_, activations_, lotMsgs_, writebacks_, ctx_.holds);
    }
    Drop(s);
    return true;  // §3
}

// ---- persistence (§7) ----------------------------------------------------------------------------

bool cFC4CivicBuildingSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("CivicBuildingSimulator::Write");
    std::lock_guard<std::recursive_mutex> g(lock_);
    FlushStatuses();
    fc4::OStreamAdapter a(s);
    const bool ok = core_ != nullptr && fc4_civic_write(core_, a.Get());
    fc4_civic_state st{};
    st.struct_size = sizeof(st);
    if (core_ != nullptr) fc4_civic_get_state(core_, &st);
    fc4::Log("civic Write -> %d (exclusions=%u statuses=%u centers=%u maintained=%u)", ok ? 1 : 0, st.exclusions,
             st.statuses, st.disease_centers, st.maintained);
    return ok;
}

bool cFC4CivicBuildingSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("CivicBuildingSimulator::Read");
    std::lock_guard<std::recursive_mutex> g(lock_);
    FlushStatuses();
    fc4::IStreamAdapter a(s);
    const bool ok = core_ != nullptr && fc4_civic_read(core_, a.Get());
    PullStatuses();
    PullOthers();
    LogState("Read", ok);
    return ok;
}

uint32_t cFC4CivicBuildingSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4CivicBuildingSimulator; }
