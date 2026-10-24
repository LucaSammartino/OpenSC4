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

#include "cFC4TrafficSimulator.h"

#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4DevelopmentInterfaces.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4GameList.h"
#include "FC4GameMap.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Seed.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4RouteDeque.h"
#include "FC4SafeLotIterator.h"
#include "FC4Streams.h"
#include "FC4TrafficCostLogic.h"
#include "cFC4Message2Standard.h"
#include "cFC4Neighbors.h"
#include "cFC4ZoneDeveloper.h"
#include "cFC4ZoneManager.h"
#include "compat/FC4Compat.h"

#include "GZServPtrs.h"
#include "SC4CellRegion.h"
#include "SC4List.h"
#include "SC4Point.h"
#include "SC4Rect.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZLanguageManager.h"
#include "cIGZLanguageUtility.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZVariant.h"
#include "cISC4App.h"
#include "cISC4BudgetSimulator.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4DemandSimulator.h"
#include "cISC4DepartmentBudget.h"
#include "cISC4GrowthDeveloper.h"
#include "cISC4HistoryWarehouse.h"
#include "cISC4LineItem.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4NeighborConnection.h"
#include "cISC4NeighborsSimulator.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4Ordinance.h"
#include "cISC4OrdinanceSimulator.h"
#include "cISC4PropOccupant.h"
#include "cISC4RegionalCity.h"
#include "cISC4ResidentialSimulator.h"
#include "cISC4Simulator.h"
#include "cISC4TrafficSource.h"
#include "cISCProperty.h"
#include "cISCPropertyHolder.h"
#include "cISTETerrain.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"

#include <algorithm>
#include <atomic>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <mutex>
#include <string>
#include <vector>

using fc4::FromToken;
using fc4::NetworkPrice;
using fc4::NetworkPriceOne;
using fc4::NetworkPriceSum;
using fc4::TokenOf;
using fc4::Sw;
namespace compat = fc4::compat;

namespace {

constexpr uint32_t kIID_cIGZUnknown = 0x00000001;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cIGZSerializable = 0xE4FDA3D4;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4NetworkOccupant = 0xA821EF94;
constexpr uint32_t kIID_cISC4NetworkTunnelOccupant = 0x8A4BD52B;  // traffic-network.md: the tunnel test

// Messages (traffic-simulator.md §3.1, §5).
constexpr uint32_t kMsgSimBegin = 0x66956810;
constexpr uint32_t kMsgDay = 0x66956814;
constexpr uint32_t kMsgMonth = 0x66956816;
constexpr uint32_t kMsgLot = 0xE8BDA2E7;
constexpr uint32_t kMsgInsertOccupant = 0x99EF1142;
constexpr uint32_t kMsgRemoveOccupant = 0x99EF1143;
constexpr uint32_t kMsgTrafficMapChanged = 0x69247DC7;
constexpr uint32_t kMsgStrikeFirst = 0x69247DC3;
constexpr uint32_t kMsgStrikeLast = 0x69247DC6;
constexpr uint32_t kMsgFerryPassDone = 0x2C06BA82;

// §4.14: the tuning exemplar.
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0xC9133286;

// traffic-network.md §8: the occupant types of the typed occupant query.
constexpr uint32_t kOccupantTypeNetwork = 0xC772BF98;
constexpr uint32_t kOccupantTypeSubway = 0x8A13918A;
constexpr uint32_t kOccupantTypeProp = 0xA823821E;

// §4.2: the trip capacity property (12 elements, u16 or u32).
constexpr uint32_t kTripCapacityProperty = 0x8A0B4100;

// The bounds of the walks over game containers (each refused above them).
constexpr uint32_t kMaxTrips = 1u << 16;          // one source's trip list
constexpr size_t kMaxRouteBytes = 1u << 20;       // one compressed route
constexpr uint32_t kMaxListNodes = 1u << 16;      // the ferry lists of slots 30 and 31
constexpr uint32_t kMaxRegisteredIterators = 4096;

fc4::TrafficContext* Ctx(void* u) { return static_cast<fc4::TrafficContext*>(u); }
cISC4City* City(void* u) { return Ctx(u)->city; }

bool Gate(Sw s, const char* what) {
    if (fc4::On(s)) return true;
    fc4::NoteSkipped(s, what);
    return false;
}

// The departments (traffic-simulator.md §1 step 5, §9.1 TG7).
constexpr uint32_t kRoadsDepartment = 0xC921CEEA;
constexpr uint32_t kMassTransitDepartment = 0xC921CEEB;
constexpr uint32_t kTrafficDepartmentGroup = 0xAA369059;
constexpr uint32_t kTrafficNameGroup = 0x6A3FF01C;
constexpr uint32_t kRoadsDepartmentName = 0x4A493B73;
constexpr uint32_t kMassTransitDepartmentName = 0xEA493B83;
// [TG23] the name of the mass transit line item 3 (`0x6A8B0CFB`, §4.9.1),
// SetName'd when UpdateMassTransitBudget creates the item itself.
constexpr uint32_t kMassTransitRailLineName = 0x6A8B0CFB;

// Test seam op 42: a callback of the test table answers instead of the game.
#define TRAFFIC_TEST(name, ...)                                                             \
    do {                                                                                    \
        const fc4_traffic_host* t_ = Ctx(u)->test;                                          \
        if (t_ != nullptr && t_->name != nullptr) return t_->name(t_->user, __VA_ARGS__);   \
    } while (0)
#define TRAFFIC_TEST0(name)                                                                 \
    do {                                                                                    \
        const fc4_traffic_host* t_ = Ctx(u)->test;                                          \
        if (t_ != nullptr && t_->name != nullptr) return t_->name(t_->user);                \
    } while (0)

// The first few notes of a kind (any thread; the counter is the cap).
void NoteFew(volatile LONG* counter, const char* text) {
    if (InterlockedIncrement(counter) <= 3) fc4::Log("%s", text);
}

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

// A reference kept until the outermost call returns (ReleaseHeld).
void Hold(void* u, cIGZUnknown* o) {
    if (o != nullptr) Ctx(u)->held.push_back(o);
}

// ---- the containers' allocator ---------------------------------------------------------------------

// The game's allocator B (allocators.md), or the test seam's.
std::atomic<void* (*)(uint32_t)> g_testAllocate{nullptr};
std::atomic<void (*)(void*)> g_testRelease{nullptr};

struct TrafficAlloc {
    static bool Available() { return g_testAllocate.load() != nullptr || fc4::StlNodeAllocator::Available(); }
    static void* Allocate(uint32_t size) {
        if (auto a = g_testAllocate.load()) return a(size);
        return fc4::StlNodeAllocator::Allocate(size);
    }
    static void Free(void* p) {
        if (p == nullptr) return;
        if (auto f = g_testRelease.load()) return f(p);
        fc4::StlNodeAllocator::Free(p);
    }
    static const void* StringVtable() { return nullptr; }
};

template <typename T>
using Vec = fc4::GameVector<T, TrafficAlloc>;

template <typename T>
Vec<T>& AsVec(fc4::GameVector<T>& v) {
    return reinterpret_cast<Vec<T>&>(v);
}

volatile LONG g_noteNoAllocator = 0;

bool AllocatorOk(const char* what) {
    if (TrafficAlloc::Available()) return true;
    static volatile LONG notes = 0;
    if (InterlockedIncrement(&notes) <= 3) {
        fc4::Log("traffic: WARNING %s left empty (the game's STL node allocator is unavailable)", what);
    }
    return false;
}

// Appends `n` bytes with the containers' allocator; false (vector unchanged
// beyond what fitted) without memory.
template <typename T>
bool Append(Vec<T>& v, const T* p, size_t n) {
    if (n == 0) return true;
    if (!v.reserve_for_resize(v.size() + n)) return false;
    for (size_t i = 0; i < n; ++i) {
        if (!v.push_back(p[i])) return false;
    }
    return true;
}

void PushBytes(void* ctx, const uint8_t* bytes, size_t len) {
    auto* v = static_cast<Vec<uint8_t>*>(ctx);
    if (v != nullptr && bytes != nullptr) Append(*v, bytes, len);
}

void PushU32(void* ctx, uint32_t value) {
    auto* v = static_cast<Vec<uint32_t>*>(ctx);
    if (v != nullptr) Append(*v, &value, 1);
}

void PushTokenVec(void* ctx, uint64_t t) { static_cast<std::vector<uint64_t>*>(ctx)->push_back(t); }

// ---- live instances, pathfinders ---------------------------------------------------------------------

std::mutex g_instancesLock;
std::vector<cFC4TrafficSimulator*> g_instances;
std::atomic<long> g_livePathFinders{0};

// The app's current city (borrowed).
cISC4City* AppCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = Query<cISC4App>(app, GZIID_cISC4App);
    if (sc4 == nullptr) return nullptr;
    cISC4City* city = sc4->GetCity();
    Drop(sc4);
    return city;
}

template <class T>
void ReleaseSlot(T*& slot) {
    if (slot != nullptr) FC4_RELEASE(slot);
    slot = nullptr;
}

// The session seed (foundations §6.3): FreeCity4.ini `traffic_seed` (env
// FC4_TRAFFIC_SEED) when set, else the wall clock as the cRZTime value for
// "now" (time() + 2177452800), as the residential and neighbours glue do.
uint32_t SessionSeed(bool* fixed) {
    const std::string s = fc4::Setting("traffic_seed", "FC4_TRAFFIC_SEED");
    if (!s.empty()) {
        char* end = nullptr;
        const unsigned long v = std::strtoul(s.c_str(), &end, 0);
        if (end != s.c_str()) {
            *fixed = true;
            return static_cast<uint32_t>(v);
        }
    }
    *fixed = false;
    // Else the process's one session seed (FC4Seed.h, foundations §6.3).
    return fc4::SessionSeed();
}

// ---- framework (§1, §5) ------------------------------------------------------------------------------

// §1 step 2: simulator slot 19 AddAgent(target, level, name, flags) on the
// city's simulator, held until Shutdown removes the agents (slot 20).
void AddAgent(void* u, uint32_t level, const uint8_t* name, size_t len, uint32_t flags) {
    TRAFFIC_TEST(add_agent, level, name, len, flags);
    fc4::TrafficContext* c = Ctx(u);
    if (c->target == nullptr || name == nullptr || !Gate(Sw::TrafficMessages, "traffic AddAgent")) return;
    cISC4Simulator* raw = c->city ? c->city->GetSimulator() : nullptr;  // city slot 50, borrowed
    if (raw == nullptr) {
        fc4::Log("traffic: no simulator; agent level %u not registered", level);
        return;
    }
    if (c->agentSim == nullptr) {
        c->agentSim = raw;
        FC4_ADDREF(c->agentSim);
    }
    const cRZBaseString n(reinterpret_cast<const char*>(name), static_cast<uint32_t>(len));
    fc4::AsGame(c->agentSim)->AddAgent(c->target, level, n, flags);
    c->agentsRegistered = true;
}

void RemoveAgents(fc4::TrafficContext* c) {
    if (c->agentSim == nullptr) return;
    if (c->agentsRegistered) fc4::AsGame(c->agentSim)->RemoveAgentAllLevels(c->target);  // slot 20
    c->agentsRegistered = false;
    ReleaseSlot(c->agentSim);
}

// §1 step 3; Shutdown removes them again (the host keeps the list).
void Subscribe(void* u, uint32_t type) {
    TRAFFIC_TEST(subscribe, type);
    fc4::TrafficContext* c = Ctx(u);
    if (!Gate(Sw::TrafficMessages, "traffic message subscription")) return;
    if (c->msgServer == nullptr || c->target == nullptr) return;
    if (std::find(c->subscribed.begin(), c->subscribed.end(), type) != c->subscribed.end()) return;
    c->msgServer->AddNotification(c->target, type);
    c->subscribed.push_back(type);
}

void Unsubscribe(fc4::TrafficContext* c) {
    if (c->msgServer != nullptr && c->target != nullptr) {
        for (uint32_t t : c->subscribed) c->msgServer->RemoveNotification(c->target, t);
    }
    c->subscribed.clear();
}

// §5: MessagePost(msg, false) of a standard message; TrafficMapChanged, the
// strike transitions and the ferry pass carry the simulator as void1 and
// IGZUnknown, the others data only.
void PostMessage(void* u, const fc4_traffic_post* m) {
    TRAFFIC_TEST(post_message, m);
    fc4::TrafficContext* c = Ctx(u);
    if (m == nullptr || !Gate(Sw::TrafficPosts, "traffic message post")) return;
    cIGZMessageServer2* ms = c->msgServer;
    if (ms == nullptr && c->postSink == nullptr) return;
    cFC4Message2Standard* msg = FC4MakeMessage(m->msg_type, static_cast<intptr_t>(m->data1),
                                               static_cast<intptr_t>(m->data2), static_cast<intptr_t>(m->data3));
    const bool self = m->msg_type == kMsgTrafficMapChanged || m->msg_type == kMsgFerryPassDone ||
                      (m->msg_type >= kMsgStrikeFirst && m->msg_type <= kMsgStrikeLast);
    if (self && c->self != nullptr) {
        msg->SetVoid1(c->self);
        msg->SetIGZUnknown(c->self);
    }
    ++c->posts;
    if (c->postSink != nullptr) c->postSink(c->postSinkContext, msg);
    if (ms != nullptr) ms->MessagePost(msg, false);
    msg->Release();
}

bool IsCityConnection(void* u, uint64_t conn);
const std::vector<uint64_t>& LotList(fc4::TrafficContext* c);

// The references the core keeps: the switch buildings (§4.7.1) and the
// source iterator's connection snapshot (§4.1, §9.1 TG19). A connection the
// core holds stays usable after it leaves the city, until the core lets go.
void AddRefToken(void* u, uint64_t t) {
    TRAFFIC_TEST(add_ref, t);
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->AddRef();
    } else {
        FC4_ADDREF(o);
    }
    fc4::TrafficContext* c = Ctx(u);
    if (++c->coreRefs[t] == 1) {
        LotList(c);
        if (c->liveLots.count(t) != 0 || c->seenLots.count(t) != 0) {
            c->heldLots.insert(t);
        } else if (IsCityConnection(u, t)) {
            c->heldConns.insert(t);
        }
    }
}

void ReleaseToken(void* u, uint64_t t) {
    TRAFFIC_TEST(release, t);
    cIGZUnknown* o = FromToken(t);
    if (o == nullptr) return;
    fc4::TrafficContext* c = Ctx(u);
    bool leftCity = false;
    auto it = c->coreRefs.find(t);
    if (it != c->coreRefs.end() && --it->second == 0) {
        c->coreRefs.erase(it);
        if (c->heldLots.erase(t) != 0) {
            LotList(c);
            leftCity = c->liveLots.count(t) == 0;
        }
        if (c->heldConns.erase(t) != 0 && !IsCityConnection(u, t)) {
            // §9.1 TG19: the snapshot was the connection's last holder once
            // the neighbour simulator let it go; the original frees it here.
            leftCity = true;
            ++c->connsLeftHeld;
        }
    }
    if (fc4::IsOurObject(o)) {
        o->Release();
    } else if (leftCity) {
        FC4_RELEASE_LAST_HOLDER(o);
    } else {
        FC4_RELEASE(o);
    }
}

// ---- city, tuning, dates ------------------------------------------------------------------------------

void CellCounts(void* u, int32_t* x, int32_t* z) {
    TRAFFIC_TEST(cell_counts, x, z);
    cISC4City* c = City(u);
    if (x != nullptr) *x = c ? static_cast<int32_t>(c->CellCountX()) : 0;  // slot 93
    if (z != nullptr) *z = c ? static_cast<int32_t>(c->CellCountZ()) : 0;  // slot 94
}

void PositionToCell(void* u, float x, float z, int32_t* cx, int32_t* cz) {
    TRAFFIC_TEST(position_to_cell, x, z, cx, cz);
    int a = 0, b = 0;
    if (cISC4City* c = City(u)) c->PositionToCell(x, z, a, b);  // slot 95
    if (cx != nullptr) *cx = a;
    if (cz != nullptr) *cz = b;
}

float CellWidth(void* u) {
    TRAFFIC_TEST0(cell_width);
    cISC4City* c = City(u);
    return c ? c->CellWidthX() : 16.0f;  // slot 91, ST0
}

// §4.14: the tuning exemplar at Init and on the reload message.
fc4_props* Tuning(void* u) {
    TRAFFIC_TEST0(tuning);
    if (!Gate(Sw::ExemplarLoads, "traffic tuning exemplar (code defaults)")) return nullptr;
    fc4::props::Exemplar ex;
    if (!ex.Load(kTuningType, kTuningGroup, kTuningInstance)) {
        fc4::Log("traffic: tuning exemplar %08X-%08X-%08X unavailable; the code defaults stay (§4.14)", kTuningType,
                 kTuningGroup, kTuningInstance);
        return nullptr;
    }
    return fc4::props::ToCoreAll(ex.Holder());
}

cFC4ISC4Simulator* Simulator(void* u) {
    cISC4City* c = City(u);
    cISC4Simulator* s = c ? c->GetSimulator() : nullptr;
    return s ? fc4::AsGame(s) : nullptr;
}

// Simulator slot 6 GetSimDate(5 pointers): the day of the month.
uint32_t SimDayOfMonth(void* u) {
    TRAFFIC_TEST0(sim_day_of_month);
    cFC4ISC4Simulator* s = Simulator(u);
    if (s == nullptr) return 0;
    uint32_t y = 0, m = 0, d = 0, doy = 0, wd = 0;
    s->GetSimDateParts(&y, &m, &d, &doy, &wd);
    return d;
}

uint32_t SimDateNumber(void* u) {
    TRAFFIC_TEST0(sim_date_number);
    cFC4ISC4Simulator* s = Simulator(u);
    return s ? static_cast<uint32_t>(s->GetSimDateNumber()) : 0;  // slot 8
}

// ---- departments (budget.md §7-§8) --------------------------------------------------------------------

cFC4ISC4BudgetSimulator* Budget(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(Sw::TrafficBudget, what)) return nullptr;
    cISC4BudgetSimulator* b = c->GetBudgetSimulator();  // city slot 52, borrowed
    return b ? fc4::AsGame(b) : nullptr;
}

cFC4ISC4DepartmentBudget* DeptOf(uint64_t t) {
    return fc4::AsGame(reinterpret_cast<cISC4DepartmentBudget*>(FromToken(t)));
}

// §1 step 5 and §9.1 TG7, only for a member still null after Read:
// CreateDepartmentBudget(id, 0xAA369059) (budget slot 25; an existing
// department comes back unchanged), one reference for the core,
// SetDepartmentName(0x6A3FF01C, name) (slot 5) and its maximum funding
// (slot 13). The line items come through CreateLineItemCb.
uint64_t CreateDepartment(void* u, uint32_t id, float maxFunding) {
    TRAFFIC_TEST(create_department, id, maxFunding);
    cFC4ISC4BudgetSimulator* b = Budget(u, "traffic department (none)");
    if (b == nullptr) return 0;
    cISC4DepartmentBudget* d = b->CreateDepartmentBudget(id, kTrafficDepartmentGroup);  // borrowed
    if (d == nullptr) {
        fc4::Log("traffic: WARNING department 0x%08X not created", id);
        return 0;
    }
    if (fc4::IsOurObject(d)) {
        d->AddRef();
    } else {
        FC4_ADDREF(d);
    }
    cFC4ISC4DepartmentBudget* g = fc4::AsGame(d);
    const uint32_t name = id == kRoadsDepartment ? kRoadsDepartmentName
                          : id == kMassTransitDepartment ? kMassTransitDepartmentName
                                                         : 0;
    if (name != 0) g->SetDepartmentName(kTrafficNameGroup, name);
    SC4Percentage max{};
    max.percentage = maxFunding;
    g->SetMaxAllowedFundingPercentage(max);
    ++Ctx(u)->departments;
    return TokenOf(d);
}

// §9.1 TG7: CreateLineItem(line, false) (department slot 17, replacing any
// line of that number), SetName(0x6A3FF01C, name) (line item slot 7) and,
// for an income line, SetType(Income) (slot 5). With onlyIfMissing nothing
// when GetLineItem(line) (slot 20) is not null.
void CreateLineItemCb(void* u, uint64_t dept, uint32_t line, uint32_t name, bool income, bool onlyIfMissing) {
    TRAFFIC_TEST(create_line_item, dept, line, name, income, onlyIfMissing);
    if (dept == 0 || !Gate(Sw::TrafficBudget, "traffic CreateLineItem (none)")) return;
    cFC4ISC4DepartmentBudget* g = DeptOf(dept);
    if (onlyIfMissing && g->GetLineItem(line) != nullptr) return;
    cISC4LineItem* item = g->CreateLineItem(line, false);  // borrowed
    if (item == nullptr) {
        fc4::Log("traffic: WARNING line item %u of department %p not created", line, static_cast<void*>(g));
        return;
    }
    item->SetName(kTrafficNameGroup, name);
    if (income) item->SetType(cISC4LineItem::Type::Income);
    ++Ctx(u)->lineItems;
}

void ReleaseDepartment(void* u, uint64_t dept) {
    TRAFFIC_TEST(release_department, dept);
    cIGZUnknown* o = FromToken(dept);
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->Release();
    } else {
        FC4_RELEASE(o);
    }
}

// Department slot 10 (a pointer to the funding fraction). [question] A null
// pointer: 1.0.
float DepartmentFunding(void* u, uint64_t dept) {
    TRAFFIC_TEST(department_funding, dept);
    if (dept == 0 || !Gate(Sw::TrafficBudget, "traffic GetFundingPercentage (1.0)")) return 1.0f;
    SC4Percentage* p = DeptOf(dept)->GetFundingPercentage();
    return p ? p->percentage : 1.0f;
}

// Department slot 20 GetLineItem (borrowed), line item slot 15 SetIncome (§3.3).
void SetLineItemIncome(void* u, uint64_t dept, uint32_t item, int64_t income) {
    TRAFFIC_TEST(set_line_item_income, dept, item, income);
    if (dept == 0 || !Gate(Sw::TrafficBudget, "traffic line item SetIncome")) return;
    cISC4LineItem* li = DeptOf(dept)->GetLineItem(item);
    if (li != nullptr) li->SetIncome(income);
}

// ---- network costs (TG23) ---------------------------------------------------------------------------------

// [TG23] traffic-simulator.md §4.9.1 (re-answers21): `UpdateRoadsBudget`
// (`0x0070D8A0`) and `UpdateMassTransitBudget` (`0x0070DA40`) price the
// per-network-type counts with the per-tile costs and **set** (never add to)
// the full expenses of fixed line items of the simulator's own departments,
// through `SetFullExpenses` (cISC4LineItem slot 12, `+0x30`, `0x006A9A00`;
// the int64 is pushed as two dwords). `AddToFullExpenses` (slot 13) is not
// used. Every call rewrites every line item below: the functions keep no
// "previous value" and have no early return. The pricing itself is
// FC4TrafficCostLogic.h (shared with the unit test).
//
// Writing: `department.GetLineItem(i)` (cISC4DepartmentBudget slot 20,
// `+0x50`, `0x006AA500`, borrowed pointer or null), then
// `lineItem.SetFullExpenses(v)` (cISC4LineItem slot 12, `+0x30`,
// `0x006A9A00`; the int64 `v` is pushed as two dwords). SetFullExpenses'
// own return channel is `unknown` in interfaces.tsv, so its result is not
// read.
void UpdateNetworkCosts(void* u, uint64_t department, bool massTransit, const uint32_t* cells13,
                        const float* tileCosts13) {
    TRAFFIC_TEST(update_network_costs, department, massTransit, cells13, tileCosts13);
    if (department == 0 || cells13 == nullptr || tileCosts13 == nullptr) return;
    if (!Gate(Sw::TrafficBudget, "traffic network costs (none)")) return;
    cFC4ISC4DepartmentBudget* g = DeptOf(department);
    if (g == nullptr) return;

    // [TG23] The type indices: Road 0, Rail 1, Highway 2, Street 3, Avenue 6,
    // Subway 7, LightRail 8, Monorail 9, OneWayRoad 10, GroundHighway 12.
    // Types 4, 5 and 11 (DirtRoad) are priced by neither function.
    struct Line {
        uint32_t item;
        uint64_t v;
    };
    if (!massTransit) {
        // [TG23] UpdateRoadsBudget, roads department 0xC921CEEA, in table
        // order. Items 5 (tolls, Income) and 6 are not touched. No line item
        // is null-checked: Init creates items 1–4 (§9.1 TG7), and the original
        // would fault on a missing one.
        const Line lines[4] = {
            {2, NetworkPriceSum(0, 10, cells13, tileCosts13)},  // Road + OneWayRoad
            {1, NetworkPriceOne(3, cells13, tileCosts13)},      // Street
            {3, NetworkPriceSum(2, 12, cells13, tileCosts13)},  // Highway + GroundHighway
            {4, NetworkPriceOne(6, cells13, tileCosts13)},      // Avenue
        };
        for (const Line& l : lines) {
            cISC4LineItem* li = g->GetLineItem(l.item);  // borrowed
            if (li != nullptr) li->SetFullExpenses(static_cast<int64_t>(l.v));
        }
        return;
    }

    // [TG23] UpdateMassTransitBudget, mass transit department 0xC921CEEB, in
    // table order. Item 3 alone is null-checked: when GetLineItem(3) returns
    // null it is created on the spot with CreateLineItem(3, false) (slot 17,
    // `+0x44`) and named SetName(0x6A3FF01C, 0x6A8B0CFB)
    // (`0x0070DA8A`–`0x0070DAAA`), then priced. Items 4–6 are not checked.
    // Items 1 (`0xEA5F96E7`) and 2 (the Income line `0x8A5F978C`) are not
    // touched.
    cISC4LineItem* li3 = g->GetLineItem(3);  // borrowed
    if (li3 == nullptr) {
        li3 = g->CreateLineItem(3, false);  // borrowed
        if (li3 != nullptr) {
            li3->SetName(kTrafficNameGroup, kMassTransitRailLineName);
            ++Ctx(u)->lineItems;
        }
    }
    // Rail (type 1) prices item 3, whether or not this call made it.
    const uint64_t rail = NetworkPriceOne(1, cells13, tileCosts13);
    if (li3 != nullptr) li3->SetFullExpenses(static_cast<int64_t>(rail));
    const Line lines[3] = {
        {4, NetworkPriceOne(7, cells13, tileCosts13)},  // Subway
        {5, NetworkPriceOne(8, cells13, tileCosts13)},  // LightRail
        {6, NetworkPriceOne(9, cells13, tileCosts13)},  // Monorail
    };
    for (const Line& l : lines) {
        cISC4LineItem* li = g->GetLineItem(l.item);  // borrowed
        if (li != nullptr) li->SetFullExpenses(static_cast<int64_t>(l.v));
    }
}

cISCPropertyHolder* HolderOf(uint64_t t) { return reinterpret_cast<cISCPropertyHolder*>(FromToken(t)); }
cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cISCPropertyHolder* OccHolder(uint64_t occ) { return occ ? OccOf(occ)->AsPropertyHolder() : nullptr; }  // slot 6

using BudgetItem = cFC4ISC4BudgetSimulator::BudgetItem;
constexpr uint32_t kPurposeTransit = 0xCA550301;
constexpr uint32_t kPurposeOther = 0x0C3BF549;

// §4.1.3 DoLotActivated step 3: slot 51 with the building's holder for
// 0xCA550301, else 0x0C3BF549; with the lot, slots 58 and 55; the cost of
// the 0xCA550301 item is returned (added to E). [question] The spec's
// "slot 55 with the item" is taken as AddBudgetItemToBudget(lot holder,
// item.index) after CopyBudgetItemProperties(building holder, lot holder).
int64_t ActivateBudgetItems(void* u, uint64_t building, uint64_t lot) {
    TRAFFIC_TEST(activate_budget_items, building, lot);
    cFC4ISC4BudgetSimulator* b = Budget(u, "traffic budget items (cost 0)");
    cISCPropertyHolder* bh = OccHolder(building);
    if (b == nullptr || bh == nullptr) return 0;
    BudgetItem item{};
    uint32_t purpose = kPurposeTransit;
    bool found = b->GetBudgetItemForPurpose(bh, kPurposeTransit, item);
    if (!found) {
        purpose = kPurposeOther;
        item = BudgetItem{};
        found = b->GetBudgetItemForPurpose(bh, kPurposeOther, item);
    }
    if (!found) return 0;
    ++Ctx(u)->budgetActivations;
    if (lot != 0) {
        cISCPropertyHolder* lh = fc4::AsGame(reinterpret_cast<cISC4Lot*>(FromToken(lot)))->AsPropertyHolder();
        if (lh != nullptr) {
            b->CopyBudgetItemProperties(bh, lh);                           // slot 58
            b->AddBudgetItemToBudget(lh, static_cast<uint32_t>(item.index));  // slot 55
        }
    }
    return purpose == kPurposeTransit ? item.cost : 0;
}

// §4.1.3 DoLotDeactivated step 3: 0xCA550301 removed (its cost returned),
// then, independently, 0x0C3BF549.
int64_t DeactivateBudgetItems(void* u, uint64_t holder) {
    TRAFFIC_TEST(deactivate_budget_items, holder);
    cFC4ISC4BudgetSimulator* b = Budget(u, "traffic budget items removal (cost 0)");
    cISCPropertyHolder* h = HolderOf(holder);
    if (b == nullptr || h == nullptr) return 0;
    int64_t cost = 0;
    BudgetItem item{};
    if (b->GetBudgetItemForPurpose(h, kPurposeTransit, item)) {
        b->RemoveBudgetItemFromBudget(h, static_cast<uint32_t>(item.index));  // slot 56
        cost = item.cost;
    }
    item = BudgetItem{};
    if (b->GetBudgetItemForPurpose(h, kPurposeOther, item)) {
        b->RemoveBudgetItemFromBudget(h, static_cast<uint32_t>(item.index));
    }
    return cost;
}

bool BudgetItemForPurpose(void* u, uint64_t building, uint32_t purpose, uint32_t* dept, int64_t* cost) {
    TRAFFIC_TEST(budget_item_for_purpose, building, purpose, dept, cost);
    cFC4ISC4BudgetSimulator* b = Budget(u, "traffic GetBudgetItemForPurpose (none)");
    cISCPropertyHolder* bh = OccHolder(building);
    if (b == nullptr || bh == nullptr) return false;
    BudgetItem item{};
    if (!b->GetBudgetItemForPurpose(bh, purpose, item)) return false;  // slot 51
    if (dept != nullptr) *dept = item.departmentID;
    if (cost != nullptr) *cost = item.cost;
    return true;
}

uint64_t DepartmentBudget(void* u, uint32_t id) {
    TRAFFIC_TEST(department_budget, id);
    cFC4ISC4BudgetSimulator* b = Budget(u, "traffic GetDepartmentBudget (none)");
    return b ? TokenOf(b->GetDepartmentBudget(id)) : 0;  // slot 28, borrowed
}

void RecordHistory(void* u, uint32_t id, uint32_t value) {
    TRAFFIC_TEST(record_history, id, value);
    if (!Gate(Sw::TrafficHistory, "traffic RecordHistoryData")) return;
    cISC4City* c = City(u);
    cISC4HistoryWarehouse* h = c ? c->GetHistoryWarehouse() : nullptr;
    if (h != nullptr) h->RecordHistoryData(id, value);
}

// ---- lots ------------------------------------------------------------------------------------------------

cFC4ISC4Lot* LotOf(uint64_t t) { return fc4::AsGame(reinterpret_cast<cISC4Lot*>(FromToken(t))); }

struct MasterCopy {
    fc4::safelots::Block* block;
    const void* expectedList;
    void** out;
    uint32_t cap;
    uint32_t count;
    fc4::safelots::Status status;
    bool wrongList;
};

// Plain reads of the game's list (no game call): under SehGuard.
void MasterCopyFn(void* c) {
    auto* m = static_cast<MasterCopy*>(c);
    if (m->block != nullptr && m->block->listObject != m->expectedList) {
        m->wrongList = true;
        return;
    }
    m->status = fc4::safelots::CopyLots(m->block, m->out, m->cap, &m->count);
}

struct MasterRelease {
    fc4::safelots::Block* block;
    fc4::safelots::FreeFn freeNode;
    int removed;
};

void MasterReleaseFn(void* c) {
    auto* m = static_cast<MasterRelease*>(c);
    m->removed = fc4::safelots::Unregister(m->block, m->freeNode, kMaxRegisteredIterators);
}

volatile LONG g_masterRefusals = 0;

bool MasterRefused(const char* why) {
    if (InterlockedIncrement(&g_masterRefusals) <= 3) {
        fc4::Log("traffic: the master lot list not used (%s); GetLotsInCellRect instead (lot order: §4.1 "
                 "[choice])",
                 why);
    }
    return false;
}

// §4.1: every lot of the original lot manager's master list, in list order,
// through its safe lot iterator (slot 15), as the budget glue walks it
// (FC4BudgetHost.cpp WalkMasterList): only on the identified executable, for
// the original lot manager or FreeCity4's (zonedev); the block is
// unregistered and freed with C. Borrowed: the master list holds the
// manager's reference.
bool MasterLots(cFC4ISC4LotManager* lm, std::vector<uint64_t>& out) {
    const compat::ExecutableVersion* v = compat::IdentifiedVersion();
    compat::StlNodeFreeFn freeNode = nullptr;
    const uintptr_t freeBlockVa = compat::DirectCall(compat::Direct::GlobalHeapFree);
    if (v == nullptr || freeBlockVa == 0 || !compat::StlNodeAllocator(nullptr, &freeNode)) {
        return MasterRefused("executable not identified, or allocator B / C unavailable");
    }
    // §9.1 TG10: the master list is the only order, also with FreeCity4's
    // lot manager (enable = zonedev), which mirrors it in the original's raw
    // layout and hands out the same safe lot iterators (slot 15); the list
    // object is the original's (+0x5C) or ours (SafeIteratorListObject).
    const char* why = nullptr;
    const void* expectedList = cFC4ZoneDeveloper::SafeIteratorListObject(lm, &why);
    if (expectedList == nullptr) return MasterRefused(why);
    const int32_t lotCount = lm->GetLotCount();  // slot 16
    if (lotCount < 0) return MasterRefused("negative GetLotCount");
    auto* block = reinterpret_cast<fc4::safelots::Block*>(lm->GetSafeLotIterator());  // slot 15
    if (block == nullptr) return MasterRefused("GetSafeLotIterator returned null");
    std::vector<void*> lots(static_cast<size_t>(lotCount) + 1);
    MasterCopy copy{block,
                    expectedList,
                    lots.data(),
                    static_cast<uint32_t>(lotCount),
                    0,
                    fc4::safelots::Status::NoBlock,
                    false};
    const bool copied = fc4::SehGuard("traffic master lot list walk", &MasterCopyFn, &copy);
    MasterRelease release{block, reinterpret_cast<fc4::safelots::FreeFn>(freeNode), -1};
    const bool unlinked =
        copied && !copy.wrongList && fc4::SehGuard("traffic safe lot iterator release", &MasterReleaseFn, &release);
    if (unlinked && release.removed > 0) {
        reinterpret_cast<fc4::safelots::FreeFn>(freeBlockVa)(block);  // allocator C
    } else {
        static volatile LONG leaks = 0;
        if (InterlockedIncrement(&leaks) <= 3) {
            fc4::Log("traffic: WARNING the safe lot iterator %p could not be unregistered; its block is leaked, not "
                     "freed",
                     static_cast<void*>(block));
        }
    }
    if (!copied || copy.wrongList || copy.status != fc4::safelots::Status::Ok) {
        return MasterRefused(!copied ? "the walk faulted"
                             : copy.wrongList ? "unexpected list object"
                                              : fc4::safelots::StatusName(copy.status));
    }
    out.reserve(copy.count);
    for (uint32_t i = 0; i < copy.count; ++i) {
        if (lots[i] != nullptr) out.push_back(TokenOf(lots[i]));
    }
    return true;
}

// [choice] Lot manager slot 7 over the whole city (one reference per lot,
// kept until the outermost call returns): the same lots in another order.
void RectLots(fc4::TrafficContext* c, cFC4ISC4LotManager* lm, std::vector<uint64_t>& out) {
    if (!fc4::StlNodeAllocator::Available()) {
        static volatile LONG notes = 0;
        NoteFew(&notes, "traffic: WARNING GetLotsInCellRect refused (STL node allocator unavailable): no lots");
        return;
    }
    const int32_t maxX = static_cast<int32_t>(c->city->CellCountX()) - 1;
    const int32_t maxZ = static_cast<int32_t>(c->city->CellCountZ()) - 1;
    fc4::GameList<cISC4Lot*> list;
    if (!list.valid()) return;
    lm->GetLotsInCellRect(list, 0, 0, maxX, maxZ, false);
    for (cISC4Lot*& lot : list) {
        if (lot == nullptr) continue;
        FC4_ACQUIRED(lot, "GetLotsInCellRect list entry");
        c->held.push_back(lot);  // released when the outermost call returns
        out.push_back(TokenOf(lot));
        lot = nullptr;  // the list's destructor then only frees its nodes
    }
}

// The lots of this outermost call (one walk per call).
const std::vector<uint64_t>& LotList(fc4::TrafficContext* c) {
    if (c->lotListValid) return c->lotList;
    c->lotListValid = true;
    c->lotList.clear();
    c->liveLots.clear();
    if (c->city == nullptr || !Gate(Sw::TrafficLots, "traffic lot list (no lots)")) return c->lotList;
    cISC4LotManager* raw = c->city->GetLotManager();  // borrowed
    if (raw == nullptr) return c->lotList;
    cFC4ISC4LotManager* lm = fc4::AsGame(raw);
    if (MasterLots(lm, c->lotList)) {
        ++c->lotWalksMaster;
    } else {
        c->lotList.clear();
        RectLots(c, lm, c->lotList);
        ++c->lotWalksRect;
    }
    c->liveLots.insert(c->lotList.begin(), c->lotList.end());
    return c->lotList;
}

// A lot token the core may have kept from an earlier call is used only while
// the city still has it, or when the game handed it over in this call.
bool LiveLot(fc4::TrafficContext* c, uint64_t lot) {
    if (lot == 0) return false;
    if (c->heldLots.count(lot) != 0) return true;
    if (c->seenLots.count(lot) != 0) return true;
    LotList(c);
    if (c->liveLots.count(lot) != 0) return true;
    ++c->staleTokens;
    static volatile LONG notes = 0;
    if (InterlockedIncrement(&notes) <= 3) {
        fc4::Log("traffic: lot %p is no longer in the city; the call answers the neutral value",
                 static_cast<void*>(FromToken(lot)));
    }
    return false;
}

uint64_t SeenLot(fc4::TrafficContext* c, cISC4Lot* lot) {
    if (lot == nullptr) return 0;
    c->seenLots.insert(TokenOf(lot));
    return TokenOf(lot);
}

bool LotOk(void* u, uint64_t lot, const char* what) {
    return lot != 0 && Gate(Sw::TrafficLots, what) && LiveLot(Ctx(u), lot);
}

void Lots(void* u, fc4_token_callback cb, void* cbCtx) {
    TRAFFIC_TEST(lots, cb, cbCtx);
    if (cb == nullptr) return;
    for (uint64_t t : LotList(Ctx(u))) cb(cbCtx, t);
}

// QueryInterface(cISC4Lot) on a message object (§3.1: a copy of the lot).
uint64_t AsLot(void* u, uint64_t object) {
    TRAFFIC_TEST(as_lot, object);
    if (object == 0 || !Gate(Sw::TrafficLots, "traffic message lot (none)")) return 0;
    cISC4Lot* lot = Query<cISC4Lot>(FromToken(object), GZIID_cISC4Lot);
    Hold(u, lot);
    return SeenLot(Ctx(u), lot);
}

uint32_t LotZone(void* u, uint64_t lot) {
    TRAFFIC_TEST(lot_zone, lot);
    if (!LotOk(u, lot, "traffic lot GetZoneType (0)")) return 0;
    return static_cast<uint32_t>(LotOf(lot)->GetZoneType());  // slot 31
}

bool LotIsLivable(void* u, uint64_t lot) {
    TRAFFIC_TEST(lot_is_livable, lot);
    if (!LotOk(u, lot, "traffic lot IsInLivableState (false)")) return false;
    return LotOf(lot)->IsInLivableState();  // slot 37
}

bool LotHasRoadAccess(void* u, uint64_t lot) {
    TRAFFIC_TEST(lot_has_road_access, lot);
    if (!LotOk(u, lot, "traffic lot HasRoadAccess (false)")) return false;
    return LotOf(lot)->HasRoadAccess();  // slot 28
}

uint64_t LotLink(void* u, uint64_t lot) {
    TRAFFIC_TEST(lot_link, lot);
    if (!LotOk(u, lot, "traffic lot GetTravelDesignate (none)")) return 0;
    return SeenLot(Ctx(u), LotOf(lot)->GetTravelDesignate());  // slot 59, borrowed
}

uint32_t LotTravelPopulation(void* u, uint64_t lot, uint32_t dev) {
    TRAFFIC_TEST(lot_travel_population, lot, dev);
    if (!LotOk(u, lot, "traffic lot GetTravelPopulation (0)")) return 0;
    return LotOf(lot)->GetTravelPopulation(static_cast<cFC4ISC4Lot::DeveloperType>(dev));  // slot 56
}

// Lot slots 12 / 13.
void ClearLotFlag(void* u, uint64_t lot, uint32_t flag) {
    TRAFFIC_TEST(clear_lot_flag, lot, flag);
    if (!LotOk(u, lot, "traffic lot SetFlags")) return;
    cFC4ISC4Lot* l = LotOf(lot);
    const uint32_t f = l->GetFlags();
    if ((f & flag) != 0) l->SetFlags(f & ~flag);
}

uint64_t LotAt(void* u, int32_t x, int32_t z, bool anchorOnly) {
    TRAFFIC_TEST(lot_at, x, z, anchorOnly);
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(Sw::TrafficLots, "traffic GetLot (none)")) return 0;
    cISC4LotManager* lm = c->GetLotManager();
    if (lm == nullptr) return 0;
    return SeenLot(Ctx(u), fc4::AsGame(lm)->GetLot(x, z, anchorOnly));  // slot 6, borrowed
}

void LotBoundingRect(void* u, uint64_t lot, fc4_cell_rect* out) {
    TRAFFIC_TEST(lot_bounding_rect, lot, out);
    if (out == nullptr) return;
    *out = fc4_cell_rect{0, 0, -1, -1};
    if (!LotOk(u, lot, "traffic lot GetBoundingRect (empty)")) return;
    SC4Rect<int32_t> r;
    if (LotOf(lot)->GetBoundingRect(r)) *out = fc4_cell_rect{r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY};
}

void LotLocation(void* u, uint64_t lot, int32_t* x, int32_t* z) {
    TRAFFIC_TEST(lot_location, lot, x, z);
    int32_t a = 0, b = 0;
    if (LotOk(u, lot, "traffic lot GetLocation (0, 0)")) LotOf(lot)->GetLocation(a, b);  // slot 21
    if (x != nullptr) *x = a;
    if (z != nullptr) *z = b;
}

uint32_t LotFacing(void* u, uint64_t lot) {
    TRAFFIC_TEST(lot_facing, lot);
    if (!LotOk(u, lot, "traffic lot GetFacing (0)")) return 0;
    return static_cast<uint32_t>(LotOf(lot)->GetFacing());  // slot 23
}

// Lot slot 48, then the building's cISC4Occupant (building slot 3): every
// occupant token of this host is a cISC4Occupant*.
uint64_t LotBuilding(void* u, uint64_t lot) {
    TRAFFIC_TEST(lot_building, lot);
    if (!LotOk(u, lot, "traffic lot GetBuilding (none)")) return 0;
    cISC4BuildingOccupant* b = LotOf(lot)->GetBuilding();  // borrowed
    return b ? TokenOf(b->AsOccupant()) : 0;
}

// Lot slot 54: the lot's own vector of maps (FC4GameMap.h), under SehGuard.
struct PopulationRead {
    const void* vector;
    fc4::GameCapacityRead* read;
};

void PopulationReadFn(void* c) {
    auto* s = static_cast<PopulationRead*>(c);
    fc4::ReadGameMapVectorU16(s->vector, s->read);
}

void LotPopulations(void* u, uint64_t lot, fc4_traffic_population_callback cb, void* cbCtx) {
    TRAFFIC_TEST(lot_populations, lot, cb, cbCtx);
    if (cb == nullptr || !LotOk(u, lot, "traffic lot GetPopulations (none)")) return;
    const void* vector = reinterpret_cast<const void*>(LotOf(lot)->GetPopulations());
    auto r = std::make_unique<fc4::GameCapacityRead>();
    PopulationRead step{vector, r.get()};
    const bool clean = fc4::SehGuard("traffic lot GetPopulations read", &PopulationReadFn, &step);
    if (!clean || r->status != fc4::GameMapStatus::Ok) {
        static volatile LONG notes = 0;
        if (InterlockedIncrement(&notes) <= 3) {
            fc4::Log("traffic: WARNING lot %p: its population maps %s%s; none read", static_cast<void*>(FromToken(lot)),
                     clean ? "failed the checks: " : "faulted", clean ? fc4::GameMapStatusName(r->status) : "");
        }
        return;
    }
    for (uint32_t i = 0; i < r->count; ++i) {
        cb(cbCtx, r->entries[i].map, r->entries[i].devType, r->entries[i].amount);
    }
}

uint64_t LotHolder(void* u, uint64_t lot) {
    TRAFFIC_TEST(lot_holder, lot);
    if (!LotOk(u, lot, "traffic lot AsPropertyHolder (none)")) return 0;
    return TokenOf(LotOf(lot)->AsPropertyHolder());  // slot 3, borrowed
}

// ---- neighbour connections ------------------------------------------------------------------------------

cISC4NeighborConnection* ConnOf(uint64_t t) { return reinterpret_cast<cISC4NeighborConnection*>(FromToken(t)); }

cFC4ISC4NeighborsSimulator* Neighbors(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(Sw::TrafficNeighbors, what)) return nullptr;
    cISC4NeighborsSimulator* n = c->GetNeighborsSimulator();
    return n ? fc4::AsGame(n) : nullptr;
}

// The connections of this outermost call (GetAllConnections, one reference
// each, kept until the call returns).
void BuildLiveConns(void* u) {
    fc4::TrafficContext* c = Ctx(u);
    if (c->liveConnsValid) return;
    c->liveConnsValid = true;
    c->liveConns.clear();
    cFC4ISC4NeighborsSimulator* ns = Neighbors(u, "traffic GetAllConnections (none)");
    if (ns == nullptr) return;
    auto list = std::make_shared<SC4List<cISC4NeighborConnection*>>();
    ns->GetAllConnections(*list);  // slot 16
    for (cISC4NeighborConnection* conn : *list) {
        if (conn != nullptr) c->liveConns.insert(TokenOf(conn));
    }
    c->heldLists.push_back(std::move(list));
}

// Whether the neighbour simulator still lists the connection (no count).
bool IsCityConnection(void* u, uint64_t conn) {
    fc4::TrafficContext* c = Ctx(u);
    if (conn == 0) return false;
    if (c->seenConns.count(conn) != 0) return true;
    BuildLiveConns(u);
    return c->liveConns.count(conn) != 0;
}

bool LiveConn(void* u, uint64_t conn) {
    fc4::TrafficContext* c = Ctx(u);
    if (conn == 0) return false;
    // §9.1 TG19: a connection of the core's snapshot is still visited after
    // it left the city (the snapshot's reference keeps it alive).
    if (c->heldConns.count(conn) != 0) return true;
    if (IsCityConnection(u, conn)) return true;
    ++c->staleTokens;
    static volatile LONG notes = 0;
    if (InterlockedIncrement(&notes) <= 3) {
        fc4::Log("traffic: neighbour connection %p is no longer in the city; the call answers the neutral value",
                 static_cast<void*>(FromToken(conn)));
    }
    return false;
}

uint64_t SeenConn(void* u, cISC4NeighborConnection* conn) {
    if (conn == nullptr) return 0;
    Ctx(u)->seenConns.insert(TokenOf(conn));
    return TokenOf(conn);
}

bool ConnOk(void* u, uint64_t conn, const char* what) {
    return conn != 0 && Gate(Sw::TrafficNeighbors, what) && LiveConn(u, conn);
}

uint64_t ConnectionHolder(void* u, uint64_t conn) {
    TRAFFIC_TEST(connection_holder, conn);
    if (!ConnOk(u, conn, "traffic connection holder (none)")) return 0;
    return TokenOf(ConnOf(conn)->AsISCPropertyHolder());  // slot 3, borrowed
}

uint64_t FindConnection(void* u, uint32_t type, int32_t x, int32_t z) {
    TRAFFIC_TEST(find_connection, type, x, z);
    cFC4ISC4NeighborsSimulator* ns = Neighbors(u, "traffic FindConnection (none)");
    if (ns == nullptr) return 0;
    return SeenConn(u, ns->FindConnection(static_cast<cFC4ISC4NeighborsSimulator::ConnectionType>(type), x, z));
}

uint32_t ConnectionType(void* u, uint64_t conn) {
    TRAFFIC_TEST(connection_type, conn);
    if (!ConnOk(u, conn, "traffic connection GetType (0)")) return 0;
    return static_cast<uint32_t>(ConnOf(conn)->GetType());  // slot 5
}

uint64_t ConnectionOnOtherSide(void* u, uint64_t conn) {
    TRAFFIC_TEST(connection_on_other_side, conn);
    if (!ConnOk(u, conn, "traffic FindConnectionOnOtherSideOf (none)")) return 0;
    cFC4ISC4NeighborsSimulator* ns = Neighbors(u, "traffic FindConnectionOnOtherSideOf (none)");
    if (ns == nullptr) return 0;
    return SeenConn(u, ns->FindConnectionOnOtherSideOf(ConnOf(conn)));  // slot 13, borrowed
}

// Connection slot 9 (borrowed), regional city slots 43 and 5.
bool EstablishedCityPosition(void* u, uint64_t conn, int32_t* x, int32_t* z) {
    TRAFFIC_TEST(established_city_position, conn, x, z);
    if (!ConnOk(u, conn, "traffic GetCityOnOtherSideOfConnection (none)")) return false;
    cISC4RegionalCity* rc = ConnOf(conn)->GetCityOnOtherSideOfConnection();
    if (rc == nullptr) return false;
    cFC4ISC4RegionalCity* r = fc4::AsGame(rc);
    if (!r->GetEstablished()) return false;
    int32_t a = 0, b = 0;
    r->GetPosition(a, b);
    if (x != nullptr) *x = a;
    if (z != nullptr) *z = b;
    return true;
}

// traffic-network.md §2.7.4 step 2: GetConnectedCities(list, false) (slot
// 21, one reference each, released with the list), each city's position
// (slot 5) and extrapolated populations 0x2010 + 0x10 w (slot 17).
void ConnectedCities(void* u, fc4_traffic_city_callback cb, void* cbCtx) {
    TRAFFIC_TEST(connected_cities, cb, cbCtx);
    if (cb == nullptr) return;
    cFC4ISC4NeighborsSimulator* ns = Neighbors(u, "traffic GetConnectedCities (none)");
    if (ns == nullptr) return;
    SC4List<cISC4RegionalCity*> list;
    ns->GetConnectedCities(list, false);
    for (cISC4RegionalCity* rc : list) {
        if (rc == nullptr) continue;
        cFC4ISC4RegionalCity* r = fc4::AsGame(rc);
        fc4_traffic_city city{};
        r->GetPosition(city.x, city.z);
        for (uint32_t w = 0; w < 3; ++w) city.population[w] = r->GetExtrapolatedPopulation(0x2010 + 0x10 * w);
        cb(cbCtx, &city);
    }
}

// §2.7.4 step 1: GetDemand(0x2010 + 0x10 w, 0x10000).QueryDemandValue().
bool RegionalDemand(void* u, uint32_t w, float* out) {
    TRAFFIC_TEST(regional_demand, w, out);
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(Sw::TrafficNeighbors, "traffic regional demand (none)")) return false;
    cISC4DemandSimulator* ds = c->GetDemandSimulator();
    if (ds == nullptr) return false;
    cISC4Demand* d = fc4::AsGame(ds)->GetDemand(0x2010 + 0x10 * w, 0x10000);  // slot 6, borrowed
    if (d == nullptr) return false;
    const float v = fc4::AsGame(d)->QueryDemandValue();  // slot 9
    if (out != nullptr) *out = v;
    return true;
}

// Neighbour simulator slots 17 / 16: one reference each (A), kept until the
// outermost call returns.
void ConnectionList(void* u, bool all, uint32_t type, fc4_token_callback cb, void* cbCtx) {
    if (cb == nullptr) return;
    cFC4ISC4NeighborsSimulator* ns = Neighbors(u, "traffic connection list (none)");
    if (ns == nullptr) return;
    auto list = std::make_shared<SC4List<cISC4NeighborConnection*>>();
    if (all) {
        ns->GetAllConnections(*list);
    } else {
        ns->GetConnectionsOfType(static_cast<cFC4ISC4NeighborsSimulator::ConnectionType>(type), *list);
    }
    std::vector<uint64_t> tokens;
    for (cISC4NeighborConnection* conn : *list) {
        if (conn != nullptr) tokens.push_back(SeenConn(u, conn));
    }
    Ctx(u)->heldLists.push_back(std::move(list));
    for (uint64_t t : tokens) cb(cbCtx, t);
}

void ConnectionsOfType(void* u, uint32_t type, fc4_token_callback cb, void* cbCtx) {
    TRAFFIC_TEST(connections_of_type, type, cb, cbCtx);
    ConnectionList(u, false, type, cb, cbCtx);
}

void AllConnections(void* u, fc4_token_callback cb, void* cbCtx) {
    TRAFFIC_TEST(all_connections, cb, cbCtx);
    ConnectionList(u, true, 0, cb, cbCtx);
}

void ConnectionCell(void* u, uint64_t conn, int32_t* x, int32_t* z) {
    TRAFFIC_TEST(connection_cell, conn, x, z);
    int32_t a = 0, b = 0;
    if (ConnOk(u, conn, "traffic connection cell (0, 0)")) ConnOf(conn)->GetConnectionCell(a, b);  // slot 6
    if (x != nullptr) *x = a;
    if (z != nullptr) *z = b;
}

// ---- traffic sources (§4.1) --------------------------------------------------------------------------------

// The source's cISC4TrafficSource (lot slot 4 / connection slot 4,
// borrowed), or null for a token no longer in the city.
cISC4TrafficSource* SourceOf(void* u, uint32_t kind, uint64_t token, const char* what) {
    if (token == 0 || !Gate(Sw::TrafficSources, what)) return nullptr;
    if (kind == FC4_TRAFFIC_SOURCE_LOT) {
        if (!LotOk(u, token, what)) return nullptr;
        return LotOf(token)->AsTrafficSource();
    }
    if (kind == FC4_TRAFFIC_SOURCE_CONNECTION) {
        if (!ConnOk(u, token, what)) return nullptr;
        return ConnOf(token)->AsTrafficSource();
    }
    return nullptr;
}

// Slot 4 UpdateTrafficVolume run by the source itself (§4.1.1, §4.1.4): the
// lot's own code asks this simulator's slots 26 and 27 re-entrantly.
bool UpdateTrafficVolume(void* u, uint32_t kind, uint64_t source) {
    TRAFFIC_TEST(update_traffic_volume, kind, source);
    cISC4TrafficSource* s = SourceOf(u, kind, source, "traffic source UpdateTrafficVolume (not run)");
    if (s == nullptr) return false;
    s->UpdateTrafficVolume();
    ++Ctx(u)->sourceUpdates;
    return true;
}

// The source's std::list<cSC4TripData> (allocators.md: one pointer to the
// head node; 0x34-byte nodes and route buffers from B).
void* TripListObject(cISC4TrafficSource* s) { return s ? static_cast<void*>(s->GetTripDataList()) : nullptr; }

size_t RouteLength(const FC4TripVector& v, bool* ok) {
    if (v.begin == nullptr) {
        if (v.end != nullptr) *ok = false;
        return 0;
    }
    if (v.end < v.begin || v.capacityEnd < v.end || static_cast<size_t>(v.end - v.begin) > kMaxRouteBytes) {
        *ok = false;
        return 0;
    }
    return static_cast<size_t>(v.end - v.begin);
}

struct TripRead {
    const void* list;
    std::vector<fc4_traffic_trip> trips;
    std::vector<std::vector<uint8_t>> routes;  // two per trip
    bool ok;
};

// Plain reads, every link checked first: under SehGuard.
void TripReadFn(void* p) {
    auto* r = static_cast<TripRead*>(p);
    r->ok = false;
    const FC4TripNode* head = *static_cast<FC4TripNode* const*>(r->list);
    if (head == nullptr || head->next == nullptr || head->next->prev != head) return;
    uint32_t n = 0;
    for (const FC4TripNode* node = head->next; node != head; node = node->next) {
        if (node == nullptr || node->next == nullptr || node->next->prev != node || ++n > kMaxTrips) return;
        const FC4TripData& d = node->value;
        bool ok = true;
        const size_t a = RouteLength(d.route, &ok);
        const size_t b = RouteLength(d.returnRoute, &ok);
        if (!ok) return;
        r->routes.emplace_back(d.route.begin, d.route.begin + a);
        r->routes.emplace_back(d.returnRoute.begin, d.returnRoute.begin + b);
        fc4_traffic_trip t{};
        t.count = d.count;
        t.trip_type = d.type;
        t.strategy = d.strategy;
        t.flags = d.flags;
        t.status = d.status;
        t.dest_x = d.destX;
        t.dest_z = d.destZ;
        t.trip_length = d.tripLength;
        t.travel_types_used = d.travelTypesUsed;
        r->trips.push_back(t);
    }
    r->ok = true;
}

volatile LONG g_noteTripRead = 0;

bool TripList(void* u, uint32_t kind, uint64_t source, fc4_traffic_trip_callback cb, void* cbCtx) {
    TRAFFIC_TEST(trip_list, kind, source, cb, cbCtx);
    void* list = TripListObject(SourceOf(u, kind, source, "traffic GetTripDataList (no trips)"));
    if (list == nullptr) return false;
    TripRead r{list, {}, {}, false};
    const bool clean = fc4::SehGuard("traffic trip list read", &TripReadFn, &r);
    ++Ctx(u)->tripReads;
    if (!clean || !r.ok) {
        static volatile LONG notes = 0;
        if (InterlockedIncrement(&notes) <= 3) {
            fc4::Log("traffic: WARNING the trip list of source %p %s; read as empty",
                     static_cast<void*>(FromToken(source)), clean ? "failed the checks" : "faulted");
        }
        return false;
    }
    if (cb == nullptr) return true;
    for (size_t i = 0; i < r.trips.size(); ++i) {
        fc4_traffic_trip& t = r.trips[i];
        const std::vector<uint8_t>& a = r.routes[2 * i];
        const std::vector<uint8_t>& b = r.routes[2 * i + 1];
        t.route = a.data();
        t.route_len = a.size();
        t.return_route = b.data();
        t.return_len = b.size();
        cb(cbCtx, &t);
    }
    return true;
}

bool CopyRoute(FC4TripVector& v, const uint8_t* bytes, size_t len) {
    v = FC4TripVector{};
    if (len == 0 || bytes == nullptr) return true;
    if (len > kMaxRouteBytes) return false;
    auto* p = static_cast<uint8_t*>(TrafficAlloc::Allocate(static_cast<uint32_t>(len)));
    if (p == nullptr) return false;
    std::memcpy(p, bytes, len);
    v.begin = p;
    v.end = p + len;
    v.capacityEnd = p + len;
    return true;
}

void FreeRoute(FC4TripVector& v) {
    TrafficAlloc::Free(v.begin);
    v = FC4TripVector{};
}

struct TripWrite {
    void* list;
    const fc4_traffic_trip* trips;
    size_t count;
    bool ok;
};

// The ring is checked first; then every node is erased (its route buffers
// and itself freed with B) and the new trips appended (copies, B).
void TripWriteFn(void* p) {
    auto* w = static_cast<TripWrite*>(p);
    w->ok = false;
    FC4TripNode* head = *static_cast<FC4TripNode**>(w->list);
    if (head == nullptr || head->next == nullptr || head->next->prev != head) return;
    uint32_t n = 0;
    for (FC4TripNode* node = head->next; node != head; node = node->next) {
        if (node == nullptr || node->next == nullptr || node->next->prev != node || ++n > kMaxTrips) return;
    }
    for (FC4TripNode* node = head->next; node != head;) {
        FC4TripNode* next = node->next;
        FreeRoute(node->value.route);
        FreeRoute(node->value.returnRoute);
        TrafficAlloc::Free(node);
        node = next;
    }
    head->next = head;
    head->prev = head;
    for (size_t i = 0; i < w->count; ++i) {
        const fc4_traffic_trip& t = w->trips[i];
        auto* node = static_cast<FC4TripNode*>(TrafficAlloc::Allocate(static_cast<uint32_t>(sizeof(FC4TripNode))));
        if (node == nullptr) return;
        std::memset(static_cast<void*>(node), 0, sizeof(FC4TripNode));
        FC4TripData& d = node->value;
        if (!CopyRoute(d.route, t.route, t.route_len) || !CopyRoute(d.returnRoute, t.return_route, t.return_len)) {
            FreeRoute(d.route);
            TrafficAlloc::Free(node);
            return;
        }
        d.count = t.count;
        d.type = t.trip_type;
        d.strategy = t.strategy;
        d.flags = t.flags;
        d.status = t.status;
        d.destX = t.dest_x;
        d.destZ = t.dest_z;
        d.tripLength = t.trip_length;
        d.travelTypesUsed = t.travel_types_used;
        node->next = head;
        node->prev = head->prev;
        head->prev->next = node;
        head->prev = node;
    }
    w->ok = true;
}

void SetTripList(void* u, uint32_t kind, uint64_t source, const fc4_traffic_trip* trips, size_t count) {
    TRAFFIC_TEST(set_trip_list, kind, source, trips, count);
    fc4::TrafficContext* c = Ctx(u);
    void* list = TripListObject(SourceOf(u, kind, source, "traffic trip list write (not written)"));
    if (list == nullptr || (count != 0 && trips == nullptr) || !AllocatorOk("traffic trip list")) {
        if (list != nullptr) ++c->tripWriteFailures;
        return;
    }
    TripWrite w{list, trips, count, false};
    const bool clean = fc4::SehGuard("traffic trip list write", &TripWriteFn, &w);
    if (clean && w.ok) {
        ++c->tripWrites;
        return;
    }
    ++c->tripWriteFailures;
    static volatile LONG notes = 0;
    if (InterlockedIncrement(&notes) <= 3) {
        fc4::Log("traffic: WARNING the trip list of source %p was not replaced (%s)",
                 static_cast<void*>(FromToken(source)), clean ? "a broken list or no memory" : "faulted");
    }
}

// ---- property holders ------------------------------------------------------------------------------------

bool HolderProperty(void* u, uint64_t holder, uint32_t id, fc4_props* out) {
    TRAFFIC_TEST(holder_property, holder, id, out);
    if (holder == 0 || out == nullptr || !Gate(Sw::TrafficHolders, "traffic holder property (absent)")) return false;
    return fc4::props::CopyToCore(HolderOf(holder), id, out);
}

size_t ElementSize(uint16_t memType) {
    switch (memType) {
        case cIGZVariant::Bool:
        case cIGZVariant::Uint8:
        case cIGZVariant::Sint8:
        case cIGZVariant::Char: return 1;
        case cIGZVariant::Uint16:
        case cIGZVariant::Sint16: return 2;
        case cIGZVariant::Uint32:
        case cIGZVariant::Sint32:
        case cIGZVariant::Float32: return 4;
        case cIGZVariant::Uint64:
        case cIGZVariant::Sint64:
        case cIGZVariant::Float64: return 8;
        default: return 0;
    }
}

// foundations §10.2: an existing array property of the same element type
// and length is written in place through its variant's data pointer
// (variant slot 95); everything else goes through AddProperty (SetTyped).
bool WriteInPlace(cISCPropertyHolder* h, uint32_t id, uint16_t memType, const void* data, size_t count) {
    const size_t size = ElementSize(memType);
    if (size == 0 || count == 0) return false;
    cISCProperty* p = fc4::AsGame(h)->GetProperty(id);  // slot 9, borrowed
    if (p == nullptr) return false;
    cIGZVariant* raw = p->GetPropertyValue();
    if (raw == nullptr) return false;
    cFC4IGZVariant* v = fc4::AsGame(raw);
    if (v->GetType() != static_cast<uint16_t>(memType | cIGZVariant::TypeArray) || v->GetCount() != count) {
        return false;
    }
    uint8_t* dst = v->RefUint8();
    if (dst == nullptr) return false;
    std::memcpy(dst, data, size * count);
    return true;
}

void SetHolderProperty(void* u, uint64_t holder, uint32_t id, uint16_t memType, bool isArray, const void* data,
                       size_t count) {
    TRAFFIC_TEST(set_holder_property, holder, id, memType, isArray, data, count);
    if (holder == 0 || data == nullptr || !Gate(Sw::TrafficHolders, "traffic holder property write")) return;
    cISCPropertyHolder* h = HolderOf(holder);
    fc4::TrafficContext* c = Ctx(u);
    if (isArray && WriteInPlace(h, id, memType, data, count)) {
        ++c->holderWritesInPlace;
        return;
    }
    if (fc4::props::SetTyped(h, id, memType, isArray, data, count)) ++c->holderWrites;
}

bool RemoveHolderProperty(void* u, uint64_t holder, uint32_t id) {
    TRAFFIC_TEST(remove_holder_property, holder, id);
    if (holder == 0 || !Gate(Sw::TrafficHolders, "traffic RemoveProperty (false)")) return false;
    return fc4::AsGame(HolderOf(holder))->RemoveProperty(id, false);  // slot 17
}

// ---- other simulators -------------------------------------------------------------------------------------

// Residential slot 30 GetCellWorkforcePercent (SC4Percentage by value).
float WorkforcePercent(void* u, int32_t x, int32_t z) {
    TRAFFIC_TEST(workforce_percent, x, z);
    cISC4City* c = City(u);
    if (c == nullptr || x < 0 || z < 0 || !Gate(Sw::TrafficSimulators, "traffic GetCellWorkforcePercent (0)")) {
        return 0.0f;
    }
    cISC4ResidentialSimulator* r = c->GetResidentialSimulator();
    if (r == nullptr) return 0.0f;
    return fc4::AsGame(r)->GetCellWorkforcePercent(static_cast<uint32_t>(x), static_cast<uint32_t>(z)).percentage;
}

// BDS slot 32 GetDeveloper(type) (borrowed), its population map (slot 15),
// GetCellValue (grid slot 7).
int16_t DeveloperPopulation(void* u, uint32_t type, int32_t x, int32_t z) {
    TRAFFIC_TEST(developer_population, type, x, z);
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(Sw::TrafficSimulators, "traffic developer population (0)")) return 0;
    cISC4BuildingDevelopmentSimulator* bds = c->GetBuildingDevelopmentSimulator();
    cISC4GrowthDeveloper* dev = bds ? fc4::AsGame(bds)->GetDeveloper(type) : nullptr;
    cISC4SimGrid<int16_t>* map = dev ? fc4::AsGame(dev)->GetPopulationMap() : nullptr;
    if (map == nullptr) return 0;
    return map->GetCellValue(x, z);
}

cFC4ISC4OrdinanceSimulator* Ordinances(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(Sw::TrafficSimulators, what)) return nullptr;
    cISC4OrdinanceSimulator* o = c->GetOrdinanceSimulator();
    return o ? fc4::AsGame(o) : nullptr;
}

bool IsOrdinanceOn(void* u, uint32_t id) {
    TRAFFIC_TEST(is_ordinance_on, id);
    cFC4ISC4OrdinanceSimulator* o = Ordinances(u, "traffic IsOrdinanceOn (false)");
    return o != nullptr && o->IsOrdinanceOn(id);  // slot 8
}

// §4.1.2: ordinance simulator slots 14 / 15 (the IDs of the ordinances on).
void OrdinancesOn(void* u, fc4_traffic_u32_callback cb, void* cbCtx) {
    TRAFFIC_TEST(ordinances_on, cb, cbCtx);
    if (cb == nullptr) return;
    cFC4ISC4OrdinanceSimulator* o = Ordinances(u, "traffic GetOrdinanceOnArray (none)");
    if (o == nullptr) return;
    uint32_t n = o->GetOrdinanceOnCount();
    if (n == 0 || n > 4096) return;
    std::vector<uint32_t> ids(n);
    o->GetOrdinanceOnArray(ids.data(), n);
    for (uint32_t i = 0; i < n && i < ids.size(); ++i) cb(cbCtx, ids[i]);
}

// Slot 20 GetOrdinanceByID, ordinance slot 16 GetMiscProperties.
bool OrdinanceProperty(void* u, uint32_t id, uint32_t prop, fc4_props* out) {
    TRAFFIC_TEST(ordinance_property, id, prop, out);
    if (out == nullptr) return false;
    cFC4ISC4OrdinanceSimulator* o = Ordinances(u, "traffic ordinance property (absent)");
    cISC4Ordinance* ord = o ? o->GetOrdinanceByID(id) : nullptr;
    cISCPropertyHolder* h = ord ? fc4::AsGame(ord)->GetMiscProperties() : nullptr;
    return h != nullptr && fc4::props::CopyToCore(h, prop, out);
}

// ---- occupants --------------------------------------------------------------------------------------------

// QueryInterface(cISC4NetworkOccupant) as the test, then the occupant's
// cISC4Occupant identity (every occupant token of this host is one), held
// until the outermost call returns.
uint64_t AsNetworkOccupant(void* u, uint64_t object) {
    TRAFFIC_TEST(as_network_occupant, object);
    if (object == 0 || !Gate(Sw::TrafficOccupants, "traffic network occupant (none)")) return 0;
    cISC4NetworkOccupant* n = Query<cISC4NetworkOccupant>(FromToken(object), kIID_cISC4NetworkOccupant);
    if (n == nullptr) return 0;
    Drop(n);
    cISC4Occupant* o = Query<cISC4Occupant>(FromToken(object), kIID_cISC4Occupant);
    Hold(u, o);
    return TokenOf(o);
}

cISC4NetworkOccupant* NetOf(uint64_t occ) {
    return Query<cISC4NetworkOccupant>(FromToken(occ), kIID_cISC4NetworkOccupant);
}

// Network occupant slot 59 GetOccupiedCell.
void NetworkOccupantCell(void* u, uint64_t occ, int32_t* x, int32_t* z) {
    TRAFFIC_TEST(network_occupant_cell, occ, x, z);
    uint32_t a = 0, b = 0;
    if (occ != 0 && Gate(Sw::TrafficOccupants, "traffic GetOccupiedCell (0, 0)")) {
        if (cISC4NetworkOccupant* n = NetOf(occ)) {
            n->GetOccupiedCell(a, b);
            Drop(n);
        }
    }
    if (x != nullptr) *x = static_cast<int32_t>(a);
    if (z != nullptr) *z = static_cast<int32_t>(b);
}

// Occupant manager slot 41 (typed, one reference each, A), kept until the
// outermost call returns. Props are kept only if they answer cISC4PropOccupant.
void OccupantsOfType(void* u, const fc4_cell_rect* r, uint32_t type, fc4_token_callback cb, void* cbCtx) {
    if (r == nullptr || cb == nullptr) return;
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(Sw::TrafficOccupants, "traffic occupant query (no occupants)")) return;
    cISC4OccupantManager* om = c->GetOccupantManager();  // borrowed
    if (om == nullptr) return;
    const SC4Point<int> mn(r->x1, r->z1);
    const SC4Point<int> mx(r->x2, r->z2);
    auto list = std::make_shared<SC4List<cISC4Occupant*>>();
    om->GetOccupantsByStandardCityCells(*list, &mn, &mx, type, 0xFFFFFFFFu);
    std::vector<uint64_t> tokens;
    for (cISC4Occupant* o : *list) {
        if (o == nullptr) continue;
        if (type == kOccupantTypeProp) {
            cISC4PropOccupant* p = Query<cISC4PropOccupant>(o, GZIID_cISC4PropOccupant);
            if (p == nullptr) continue;
            Drop(p);
        }
        tokens.push_back(TokenOf(o));
    }
    Ctx(u)->heldLists.push_back(std::move(list));
    for (uint64_t t : tokens) cb(cbCtx, t);
}

void NetworkOccupantsInRect(void* u, const fc4_cell_rect* r, fc4_token_callback cb, void* cbCtx) {
    TRAFFIC_TEST(network_occupants_in_rect, r, cb, cbCtx);
    OccupantsOfType(u, r, kOccupantTypeNetwork, cb, cbCtx);
}

void SubwayOccupantsInRect(void* u, const fc4_cell_rect* r, fc4_token_callback cb, void* cbCtx) {
    TRAFFIC_TEST(subway_occupants_in_rect, r, cb, cbCtx);
    OccupantsOfType(u, r, kOccupantTypeSubway, cb, cbCtx);
}

void PropOccupantsInRect(void* u, const fc4_cell_rect* r, fc4_token_callback cb, void* cbCtx) {
    TRAFFIC_TEST(prop_occupants_in_rect, r, cb, cbCtx);
    OccupantsOfType(u, r, kOccupantTypeProp, cb, cbCtx);
}

void OccupantPosition(void* u, uint64_t occ, float* xyz) {
    TRAFFIC_TEST(occupant_position, occ, xyz);
    if (xyz == nullptr) return;
    xyz[0] = xyz[1] = xyz[2] = 0.0f;
    if (occ == 0 || !Gate(Sw::TrafficOccupants, "traffic occupant GetPosition")) return;
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);  // slot 8
    xyz[0] = v.fX;
    xyz[1] = v.fY;
    xyz[2] = v.fZ;
}

void OccupantBoundingCells(void* u, uint64_t occ, fc4_cell_rect* out) {
    TRAFFIC_TEST(occupant_bounding_cells, occ, out);
    if (out == nullptr) return;
    *out = fc4_cell_rect{0, 0, -1, -1};
    if (occ == 0 || !Gate(Sw::TrafficOccupants, "traffic occupant GetBoundingCityCells (empty)")) return;
    SC4Rect<long> r;
    if (OccOf(occ)->GetBoundingCityCells(r)) {  // slot 11
        *out = fc4_cell_rect{static_cast<int32_t>(r.topLeftX), static_cast<int32_t>(r.topLeftY),
                             static_cast<int32_t>(r.bottomRightX), static_cast<int32_t>(r.bottomRightY)};
    }
}

// cISC4PropOccupant slot 7 GetPropType.
uint32_t PropType(void* u, uint64_t occ) {
    TRAFFIC_TEST(prop_type, occ);
    if (occ == 0 || !Gate(Sw::TrafficOccupants, "traffic GetPropType (0)")) return 0;
    cISC4PropOccupant* p = Query<cISC4PropOccupant>(FromToken(occ), GZIID_cISC4PropOccupant);
    if (p == nullptr) return 0;
    const uint32_t t = p->GetPropType();
    Drop(p);
    return t;
}

bool OccupantProperty(void* u, uint64_t occ, uint32_t id, fc4_props* out) {
    TRAFFIC_TEST(occupant_property, occ, id, out);
    if (occ == 0 || out == nullptr || !Gate(Sw::TrafficOccupants, "traffic occupant property (absent)")) return false;
    return fc4::props::CopyToCore(OccHolder(occ), id, out);
}

bool HasNetworkFlag(void* u, uint64_t occ, uint32_t flag) {
    TRAFFIC_TEST(has_network_flag, occ, flag);
    if (occ == 0 || !Gate(Sw::TrafficOccupants, "traffic HasNetworkFlag (false)")) return false;
    cISC4NetworkOccupant* n = NetOf(occ);
    if (n == nullptr) return false;
    const bool r = n->HasNetworkFlag(flag);  // slot 8
    Drop(n);
    return r;
}

bool HasAnyNetworkFlag(void* u, uint64_t occ, uint32_t mask) {
    TRAFFIC_TEST(has_any_network_flag, occ, mask);
    if (occ == 0 || !Gate(Sw::TrafficOccupants, "traffic HasAnyNetworkFlag (false)")) return false;
    cISC4NetworkOccupant* n = NetOf(occ);
    if (n == nullptr) return false;
    const bool r = n->HasAnyNetworkFlag(mask);  // slot 9
    Drop(n);
    return r;
}

bool IsOfType(void* u, uint64_t occ, uint32_t type) {
    TRAFFIC_TEST(is_of_type, occ, type);
    if (occ == 0 || !Gate(Sw::TrafficOccupants, "traffic IsOfType (false)")) return false;
    cISC4NetworkOccupant* n = NetOf(occ);
    if (n == nullptr) return false;
    const bool r = n->IsOfType(static_cast<cISC4NetworkOccupant::eNetworkType>(type));  // slot 22
    Drop(n);
    return r;
}

// traffic-network.md §7.2: slot 53 is PathConnection(pathType, entry, exit)
// (pop 12, AL); the SDK has slots 53 and 54 the other way round, so it is
// called through the vtable.
bool PathConnection(void* u, uint64_t occ, uint32_t pathType, uint32_t entry, uint32_t exit) {
    TRAFFIC_TEST(path_connection, occ, pathType, entry, exit);
    if (occ == 0 || !Gate(Sw::TrafficOccupants, "traffic PathConnection (false)")) return false;
    cISC4NetworkOccupant* n = NetOf(occ);
    if (n == nullptr) return false;
    using Fn = uint8_t(__fastcall*)(void* self, void* edx, uint32_t pathType, uint32_t entry, uint32_t exit);
    const Fn f = reinterpret_cast<Fn>((*reinterpret_cast<void***>(n))[53]);
    const bool r = f(n, nullptr, pathType, entry, exit) != 0;
    Drop(n);
    return r;
}

// traffic-network.md §4.2 step 3, §7.2: network occupant slot 49
// GetPathInfo (pop 0, EAX), through the vtable (the SDK declares no path
// info). [TG24 MATCH] Borrowed: slot 49 builds the path info on the first
// call and caches it in the occupant (`+0x140`), and returns the cached
// pointer **without** AddRef, so the caller borrows it and must not release
// it (traffic-network.md §7.2). It may be null: the pre-built-model override
// returns null when it has no model. The occupant keeps its cached path info
// while it is in the city, so the token stays usable for the rest of the
// rebuild pass; nothing releases it.
uint64_t PathInfo(void* u, uint64_t occ) {
    TRAFFIC_TEST(path_info, occ);
    if (occ == 0 || !Gate(Sw::TrafficOccupants, "traffic GetPathInfo (none)")) return 0;
    cISC4NetworkOccupant* n = NetOf(occ);
    if (n == nullptr) return 0;
    using Fn = void*(__fastcall*)(void* self, void* edx);
    void* pi = reinterpret_cast<Fn>((*reinterpret_cast<void***>(n))[49])(n, nullptr);
    Drop(n);
    return TokenOf(pi);
}

// traffic-network.md §4.2 step 3: the turn-around test, path info slot 4
// (`cSC4PathInfo::GetPath`, pop 20, EAX) with (pathType, entry, 0xFF, 0, 0)
// as the original passes them; a non-zero result allows a turn-around at
// that entry. [TG24 MATCH] The whole EAX is tested: slot 4 returns a path
// pointer or null, not a bool (`0x0071ABF9`).
bool PathInfoTurnAround(void* u, uint64_t pi, uint32_t pathType, uint32_t entry) {
    TRAFFIC_TEST(path_info_turn_around, pi, pathType, entry);
    if (pi == 0 || !Gate(Sw::TrafficOccupants, "traffic path info turn-around (false)")) return false;
    void* p = reinterpret_cast<void*>(static_cast<uintptr_t>(pi));
    using Fn = uintptr_t(__fastcall*)(void* self, void* edx, uint32_t pathType, uint32_t entry, uint32_t exit,
                                      uint32_t a4, uint32_t a5);
    return reinterpret_cast<Fn>((*reinterpret_cast<void***>(p))[4])(p, nullptr, pathType, entry, 0xFF, 0, 0) != 0;
}

// ---- path info: lane continuity (TG24) -------------------------------------------------------------------

// [TG24] The path key of traffic-network.md §7.3: the exit side in bits 0–7
// (0–3; `0xFF` names the turn-around path of an entry), the entry side in
// bits 8–15, a lane index in bits 16–23 and the path type in bits 24–27.
// Bits 28–31 are 0 for the paths these queries return. The "exit byte" of a
// path is not a field of the record: it is bits 0–7 of its key.
uint8_t KeyExit(uint32_t key) { return static_cast<uint8_t>(key & 0xFFu); }
uint8_t KeyEntry(uint32_t key) { return static_cast<uint8_t>((key >> 8) & 0xFFu); }

// [TG24] A path record (what slots 3 and 4 point at, §7.3): `+0` the start and
// `+4` the end pointer of its point array (12-byte points, three f32 world
// coordinates x, y, z), `+0xC` its length (f32). The **first vertex** is the
// point at `+0`'s address; the end point used by slot 7 is the last point
// (`end − 12`). Only the two pointers are read here.
void* PathPointAt(void* record, size_t index) {
    if (record == nullptr) return nullptr;
    char* base = static_cast<char*>(record);
    void* begin = nullptr;
    void* end = nullptr;
    std::memcpy(&begin, base, sizeof(begin));              // +0
    std::memcpy(&end, base + sizeof(void*), sizeof(end));  // +4
    if (begin == nullptr || end == nullptr || end < begin) return nullptr;
    const size_t bytes = static_cast<size_t>(static_cast<char*>(end) - static_cast<char*>(begin));
    if (bytes < 12 || index >= bytes / 12) return nullptr;
    return static_cast<char*>(begin) + 12 * index;
}

// [TG24] §4.4 step 1 (`LaneContinuity`, `RemoveBadElevatedPaths` at
// `0x00715940`): `keys := GetPathsFromEntryEdge(1, e)` (path info slot 14:
// every car path whose entry is e, in the path table's hash order). For each
// key in that order, `path := GetPath(key, null)` (slot 3); when it is not
// null the path's **first vertex** goes to the caller with the key's exit
// byte, and the core keeps the ones whose exit byte is the connection's exit.
// The vertex is a borrowed token into the path info (valid while the path info
// is unchanged); neither the path record nor the path info is released.
void PathInfoExitsFromEntry(void* u, uint64_t pi, uint32_t pathType, uint32_t entry,
                            fc4_traffic_exit_callback cb, void* ctx) {
    TRAFFIC_TEST(path_info_exits_from_entry, pi, pathType, entry, cb, ctx);
    if (pi == 0 || cb == nullptr) return;
    if (!Gate(Sw::TrafficOccupants, "traffic path info exits (none)")) return;
    void* p = reinterpret_cast<void*>(static_cast<uintptr_t>(pi));
    if (!AllocatorOk("traffic path-info key list")) return;
    // Slot 14 empties `keys` first (keeps the capacity) and appends; the
    // vector is the caller's and goes back to allocator B.
    Vec<uint32_t> keys;
    using FillFn = uint32_t(__fastcall*)(void* self, void* edx, uint32_t pathType, uint32_t entry, void* out);
    const FillFn fill = reinterpret_cast<FillFn>((*reinterpret_cast<void***>(p))[14]);
    fill(p, nullptr, pathType, entry, &keys);  // pop 12; EAX = the count appended
    using GetFn = void*(__fastcall*)(void* self, void* edx, uint32_t key, void* lengthOut);
    const GetFn get = reinterpret_cast<GetFn>((*reinterpret_cast<void***>(p))[3]);
    for (size_t i = 0; i < keys.size(); ++i) {
        const uint32_t key = keys[i];
        void* rec = get(p, nullptr, key, nullptr);  // pop 8; borrowed, null when absent
        void* first = PathPointAt(rec, 0);
        if (first == nullptr) continue;
        cb(ctx, KeyExit(key), TokenOf(first));
    }
}

// [TG24] §4.4 step 2:
// `near := GetPathsNearEndPoint(1, &v, 0xFF, (e + 2) mod 4, 4.0)` (path info
// slot 7): the keys of the car paths through the given exit whose **last**
// vertex lies within `radius` of the vertex, nearest first. For each key the
// caller ORs `1 << entry(k)` into its mask, so the entry sides come back here.
// The result is a `vector<u32>` returned **by value** through a hidden first
// argument: the callee constructs it and the caller frees it (allocator B).
void PathInfoEntriesNear(void* u, uint64_t pi, uint32_t pathType, uint64_t vertex, uint32_t exit, float radius,
                         fc4_traffic_u32_callback cb, void* ctx) {
    TRAFFIC_TEST(path_info_entries_near, pi, pathType, vertex, exit, radius, cb, ctx);
    if (pi == 0 || cb == nullptr) return;
    if (!Gate(Sw::TrafficOccupants, "traffic path info entries (none)")) return;
    void* p = reinterpret_cast<void*>(static_cast<uintptr_t>(pi));
    const float* point = reinterpret_cast<const float*>(static_cast<uintptr_t>(vertex));
    if (point == nullptr) return;
    if (!AllocatorOk("traffic path-info near list")) return;
    // Uninitialised storage: slot 7 constructs the vector in it.
    using U32Vec = Vec<uint32_t>;
    alignas(U32Vec) unsigned char storage[sizeof(U32Vec)];
    U32Vec* keys = reinterpret_cast<U32Vec*>(storage);
    using NearFn = void*(__fastcall*)(void* self, void* edx, void* hidden, uint32_t pathType, const float* point,
                                      uint32_t entry, uint32_t side, float radius);
    const NearFn nearFn = reinterpret_cast<NearFn>((*reinterpret_cast<void***>(p))[7]);
    // pop 24 (the hidden pointer included: 6 dwords); EAX = the hidden pointer.
    // `entry = 0xFF` is the wildcard here (the "any entry" of §4.4 step 2);
    // it is not a wildcard in slot 14 (§7.3).
    nearFn(p, nullptr, keys, pathType, point, 0xFF, exit, radius);
    for (size_t i = 0; i < keys->size(); ++i) {
        cb(ctx, KeyEntry((*keys)[i]));
    }
    // The caller frees what the callee built (allocator B).
    keys->~U32Vec();
}

bool IsTunnel(void* u, uint64_t occ) {
    TRAFFIC_TEST(is_tunnel, occ);
    if (occ == 0 || !Gate(Sw::TrafficOccupants, "traffic tunnel test (false)")) return false;
    cIGZUnknown* t = Query<cIGZUnknown>(FromToken(occ), kIID_cISC4NetworkTunnelOccupant);
    if (t == nullptr) return false;
    Drop(t);
    return true;
}

// ---- the paired portal (TG25) ----------------------------------------------------------------------------

// [TG25] traffic-network.md §4.8 step 1: the paired portal is the pointer
// member at **`+0x1D0`** of the concrete `cSC4NetworkTunnelOccupant`, read by
// a one-instruction getter (`0x00647570`) at `0x0071414B`. The offset is from
// the class's primary base, which is what QueryInterface(0x8A4BD52B) and
// QueryInterface(0xA821EF94) both return. (In `Write` and `Read` the same
// member is at `+0x1C8`, because those run on the `+8` serializable
// sub-object.)
//
// [TG25] The member is a **counted** reference to the other portal: the only
// setter (`0x00647530`) AddRefs the new pointer and Releases the old one, and
// the occupant's `Shutdown` (`0x00647500`) releases it and nulls it, so this
// portal owns a reference on the other (a reference cycle `Shutdown` breaks).
// Reading it is therefore a **borrow**: this returns the member with no AddRef
// and no Release, so neither portal's reference count moves (§11 test 16).
// Never Release a pointer you did not AddRef. The original wraps its own use
// in a temporary AddRef / Release (`0x0071415E` / `0x0071419A`), which is
// balanced and leaves the counts the same.
//
// [TG25] The returned token is this host's usual occupant identity, a
// `cISC4Occupant*`: the original takes the pair's `AsOccupant` (network
// occupant slot 48, `this + 4`) before its position (occupant slot 8), and
// the core reads the position of what comes back here.
uint64_t TunnelPairedPortal(void* u, uint64_t occ) {
    TRAFFIC_TEST(tunnel_paired_portal, occ);
    if (occ == 0 || !Gate(Sw::TrafficOccupants, "traffic tunnel pair (none)")) return 0;
    cIGZUnknown* t = Query<cIGZUnknown>(FromToken(occ), kIID_cISC4NetworkTunnelOccupant);
    if (t == nullptr) return 0;
    void* pair = nullptr;
    std::memcpy(&pair, static_cast<char*>(static_cast<void*>(t)) + 0x1D0, sizeof(pair));  // the counted member
    Drop(t);  // the one reference QueryInterface gave us, balanced
    if (pair == nullptr) return 0;  // never paired, or after its Shutdown (TN11)
    using AsFn = void*(__fastcall*)(void* self, void* edx);
    cISC4NetworkOccupant* np = reinterpret_cast<cISC4NetworkOccupant*>(pair);
    void* o = reinterpret_cast<AsFn>((*reinterpret_cast<void***>(np))[48])(np, nullptr);  // pop 0, EAX
    return TokenOf(o);
}

// ---- terrain and water ----------------------------------------------------------------------------------

bool CellUnderWater(void* u, int32_t x, int32_t z) {
    TRAFFIC_TEST(cell_under_water, x, z);
    cISC4City* c = City(u);
    if (c == nullptr || x < 0 || z < 0 || !Gate(Sw::TrafficTerrain, "traffic CellHasWater (dry)")) return false;
    cISTETerrain* t = c->GetTerrain();
    return t != nullptr && t->CellHasWater(static_cast<uint32_t>(x), static_cast<uint32_t>(z));  // slot 34
}

// City slot 83 GetSurfaceWater (borrowed): cISTESurfaceWater has no
// declaration matching the game (traffic-network.md §8: its IsNavigable
// overloads are in the other order), so slots 6, 8 and 9 are called through
// the vtable.
void* SurfaceWater(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(Sw::TrafficTerrain, what)) return nullptr;
    return static_cast<void*>(c->GetSurfaceWater());
}

void* VtableSlot(void* obj, int slot) { return (*reinterpret_cast<void***>(obj))[slot]; }

bool IsNavigable(void* u, int32_t x, int32_t z) {
    TRAFFIC_TEST(is_navigable, x, z);
    void* sw = SurfaceWater(u, "traffic IsNavigable (false)");
    if (sw == nullptr) return false;
    using Fn = uint8_t(__fastcall*)(void* self, void* edx, int32_t x, int32_t z, int32_t radius, void* lot);
    return reinterpret_cast<Fn>(VtableSlot(sw, 8))(sw, nullptr, x, z, 0, nullptr) != 0;  // pop 16, AL
}

float WaterDepth(void* u, int32_t x, int32_t z) {
    TRAFFIC_TEST(water_depth, x, z);
    void* sw = SurfaceWater(u, "traffic GetWaterDepthForCell (0)");
    if (sw == nullptr) return 0.0f;
    using Fn = float(__fastcall*)(void* self, void* edx, int32_t x, int32_t z);
    return reinterpret_cast<Fn>(VtableSlot(sw, 6))(sw, nullptr, x, z);  // pop 8, ST0
}

bool BodyOfWater(void* u, int32_t x, int32_t z, uint16_t* out) {
    TRAFFIC_TEST(body_of_water, x, z, out);
    cISC4City* c = City(u);
    if (c == nullptr || x < 0 || z < 0 || static_cast<uint32_t>(x) >= c->CellCountX() ||
        static_cast<uint32_t>(z) >= c->CellCountZ()) {
        return false;
    }
    void* sw = SurfaceWater(u, "traffic GetBodyOfWaterMap (none)");
    if (sw == nullptr) return false;
    using Fn = cISC4SimGrid<uint16_t>*(__fastcall*)(void* self, void* edx);
    cISC4SimGrid<uint16_t>* g = reinterpret_cast<Fn>(VtableSlot(sw, 9))(sw, nullptr);
    if (g == nullptr) return false;
    if (out != nullptr) *out = g->GetCellValue(x, z);  // grid slot 7
    return true;
}

// ---- language -------------------------------------------------------------------------------------------

// GetLanguageUtility(0) (borrowed), GetLanguageRoadDrivingSide: 1 = right
// (traffic-simulator.md §4.5). Without the service: right.
bool DrivesOnRight(void* u) {
    TRAFFIC_TEST0(drives_on_right);
    cIGZLanguageManagerPtr lm;
    if (!lm) return true;
    cIGZLanguageUtility* lu = lm->GetLanguageUtility(0);
    return lu == nullptr || lu->GetLanguageRoadDrivingSide() == 1;
}

}  // namespace

// ---- the host table ---------------------------------------------------------------------------------------

namespace fc4 {

// Callbacks left NULL answer the core's neutral value. By traffic-simulator
// §9.1 (TG2, re-answers10):
// - not needed for the simulation: sim_clock_ms (the 4 ms deadline is the
//   unit budget), view_cell_rect, cell_has_network_occupant and
//   create_accident (the accident check: wall-clock gated, visible cells
//   only; it costs only the visible crashes), occupants_in_cells and
//   occupant_filter_includes (pathfinder filter 2 is unreachable);
// - matched another way: lot_strategy_byte and take_connection_volume_dirty
//   (update_traffic_volume runs the source's own UpdateTrafficVolume, which
//   reads its strategy byte and tests and clears its own volume-dirty flag);
//   the occupant query over a rectangle is network_occupants_in_rect (slot
//   41, wired);
// - wired since re-answers21: update_network_costs (TG23, §4.9.1: the roads
//   items 2, 1, 3, 4 and the mass transit items 3, 4, 5, 6, priced with the
//   §4.9.1 rounding and set with SetFullExpenses from the five call sites),
//   path_info_exits_from_entry and path_info_entries_near (TG24, path info
//   slots 14 then 3, and slot 7: the keys, the borrowed first vertex and the
//   result containers of §7.3; these feed lane continuity, §4.4) and
//   tunnel_paired_portal (TG25, the counted pointer at +0x1D0 of
//   cSC4NetworkTunnelOccupant, borrowed). Nothing is left NULL [question].
fc4_traffic_host MakeTrafficHost(TrafficContext* ctx) {
    fc4_traffic_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.grids = MakeGridApi(&ctx->grid);
    h.add_agent = &AddAgent;
    h.subscribe = &Subscribe;
    h.post_message = &PostMessage;
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.cell_counts = &CellCounts;
    h.position_to_cell = &PositionToCell;
    h.cell_width = &CellWidth;
    h.tuning = &Tuning;
    h.sim_day_of_month = &SimDayOfMonth;
    h.sim_date_number = &SimDateNumber;
    h.create_department = &CreateDepartment;
    h.create_line_item = &CreateLineItemCb;
    h.release_department = &ReleaseDepartment;
    h.department_funding = &DepartmentFunding;
    h.set_line_item_income = &SetLineItemIncome;
    h.update_network_costs = &UpdateNetworkCosts;  // [TG23]
    h.activate_budget_items = &ActivateBudgetItems;
    h.deactivate_budget_items = &DeactivateBudgetItems;
    h.budget_item_for_purpose = &BudgetItemForPurpose;
    h.department_budget = &DepartmentBudget;
    h.record_history = &RecordHistory;
    h.lots = &Lots;
    h.as_lot = &AsLot;
    h.lot_zone = &LotZone;
    h.lot_is_livable = &LotIsLivable;
    h.lot_has_road_access = &LotHasRoadAccess;
    h.lot_link = &LotLink;
    h.lot_travel_population = &LotTravelPopulation;
    h.clear_lot_flag = &ClearLotFlag;
    h.lot_at = &LotAt;
    h.lot_bounding_rect = &LotBoundingRect;
    h.lot_location = &LotLocation;
    h.lot_facing = &LotFacing;
    h.lot_building = &LotBuilding;
    h.lot_populations = &LotPopulations;
    h.lot_holder = &LotHolder;
    h.update_traffic_volume = &UpdateTrafficVolume;
    h.trip_list = &TripList;
    h.set_trip_list = &SetTripList;
    h.holder_property = &HolderProperty;
    h.set_holder_property = &SetHolderProperty;
    h.remove_holder_property = &RemoveHolderProperty;
    h.connection_holder = &ConnectionHolder;
    h.find_connection = &FindConnection;
    h.connection_type = &ConnectionType;
    h.connection_on_other_side = &ConnectionOnOtherSide;
    h.established_city_position = &EstablishedCityPosition;
    h.connected_cities = &ConnectedCities;
    h.regional_demand = &RegionalDemand;
    h.connections_of_type = &ConnectionsOfType;
    h.all_connections = &AllConnections;
    h.connection_cell = &ConnectionCell;
    h.workforce_percent = &WorkforcePercent;
    h.developer_population = &DeveloperPopulation;
    h.is_ordinance_on = &IsOrdinanceOn;
    h.ordinances_on = &OrdinancesOn;
    h.ordinance_property = &OrdinanceProperty;
    h.as_network_occupant = &AsNetworkOccupant;
    h.network_occupant_cell = &NetworkOccupantCell;
    h.network_occupants_in_rect = &NetworkOccupantsInRect;
    h.subway_occupants_in_rect = &SubwayOccupantsInRect;
    h.prop_occupants_in_rect = &PropOccupantsInRect;
    h.occupant_position = &OccupantPosition;
    h.occupant_bounding_cells = &OccupantBoundingCells;
    h.prop_type = &PropType;
    h.occupant_property = &OccupantProperty;
    h.has_network_flag = &HasNetworkFlag;
    h.has_any_network_flag = &HasAnyNetworkFlag;
    h.is_of_type = &IsOfType;
    h.path_connection = &PathConnection;
    h.path_info = &PathInfo;
    h.path_info_turn_around = &PathInfoTurnAround;
    h.path_info_exits_from_entry = &PathInfoExitsFromEntry;  // [TG24]
    h.path_info_entries_near = &PathInfoEntriesNear;        // [TG24]
    h.is_tunnel = &IsTunnel;
    h.tunnel_paired_portal = &TunnelPairedPortal;  // [TG25]
    h.cell_under_water = &CellUnderWater;
    h.is_navigable = &IsNavigable;
    h.water_depth = &WaterDepth;
    h.body_of_water = &BodyOfWater;
    h.drives_on_right = &DrivesOnRight;
    return h;
}

}  // namespace fc4

// ---- object ---------------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4TrafficSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4TrafficSimulator*>(new cFC4TrafficSimulator()));
}

cFC4TrafficSimulator::cFC4TrafficSimulator() {
    InitializeCriticalSection(&lock_);
    mainTid_ = GetCurrentThreadId();  // the class store creates city components on the main thread
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    ctx_.self = Primary();
    seed_ = SessionSeed(&seedFixed_);
    host_ = fc4::MakeTrafficHost(&ctx_);
    core_ = fc4_traffic_create(&host_, seed_);
    if (core_ == nullptr) fc4::Log("cFC4TrafficSimulator: core creation failed");
    {
        std::lock_guard<std::mutex> g(g_instancesLock);
        g_instances.push_back(this);
    }
    RefreshSnapshot();
    fc4::Log("cFC4TrafficSimulator(%p) created, core %p, session seed %u (%s)", static_cast<void*>(Primary()),
             static_cast<void*>(core_), seed_, seedFixed_ ? "traffic_seed" : "session seed");
}

cFC4TrafficSimulator::~cFC4TrafficSimulator() {
    {
        std::lock_guard<std::mutex> g(g_instancesLock);
        g_instances.erase(std::remove(g_instances.begin(), g_instances.end(), this), g_instances.end());
    }
    EnterCriticalSection(&lock_);
    // The pathfinders still alive lose their core with this simulator.
    for (cFC4PathFinder* p : pathfinders_) p->Orphan();
    pathfinders_.clear();
    fc4_traffic_destroy(core_);  // releases what a never-initialised or loaded object still holds
    core_ = nullptr;
    ClearFerryVectors();
    ReleaseHeld();
    LeaveCriticalSection(&lock_);
    Unsubscribe(&ctx_);
    RemoveAgents(&ctx_);
    ReleaseSlot(ctx_.msgServer);
    ReleaseSlot(ctx_.city);
    fc4::Log("cFC4TrafficSimulator destroyed (%u fallbacks, %u external pathfinders)", fallbacks_.load(),
             externalPathFinders_);
    DeleteCriticalSection(&lock_);
}

void cFC4TrafficSimulator::ClearFerryVectors() {
    for (auto& kv : ferryVectors_) delete static_cast<Vec<uint8_t>*>(kv.second);
    ferryVectors_.clear();
}

// §1: 1 and 0x68027F37 → the primary interface; 0x452294AA, 0xE4FDA3D4.
bool cFC4TrafficSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr || !fc4_traffic_query_interface(riid)) return false;
    if (riid == kIID_cIGZMessageTarget2) {
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
    } else if (riid == kIID_cIGZSerializable) {
        *ppvObj = static_cast<cIGZSerializable*>(this);
    } else {
        *ppvObj = Primary();
    }
    ++refCount_;
    return true;
}

uint32_t cFC4TrafficSimulator::AddRef() { return ++refCount_; }

// Above 1: decrement; otherwise Shutdown, then destroy.
uint32_t cFC4TrafficSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    Shutdown();
    delete this;
    return 0;
}

cFC4TrafficSimulator* cFC4TrafficSimulator::FromPrimary(const void* primary) {
    std::lock_guard<std::mutex> g(g_instancesLock);
    for (cFC4TrafficSimulator* t : g_instances) {
        if (static_cast<const void*>(t->Primary()) == primary ||
            static_cast<const void*>(static_cast<cIGZMessageTarget2*>(t)) == primary ||
            static_cast<const void*>(static_cast<cIGZSerializable*>(t)) == primary) {
            return t;
        }
    }
    return nullptr;
}

void cFC4TrafficSimulator::SetTestHooks(const fc4_traffic_host* test, FC4PostSinkFn sink, void* sinkContext) {
    Section sec(this);
    ctx_.test = test;
    ctx_.postSink = sink;
    ctx_.postSinkContext = sinkContext;
    for (cFC4PathFinder* p : pathfinders_) p->Orphan();
    pathfinders_.clear();
    fc4_traffic_destroy(core_);
    host_ = fc4::MakeTrafficHost(&ctx_);
    if (test != nullptr && test->grids.create != nullptr) host_.grids = test->grids;
    core_ = fc4_traffic_create(&host_, seed_);
    lastCycleState_ = 0xFFFFFFFFu;
    fc4::Log("traffic: test hooks %s (core %p)", test ? "installed" : "removed", static_cast<void*>(core_));
}

long cFC4TrafficSimulator::LivePathFinders() { return g_livePathFinders.load(); }

void cFC4TrafficSimulator::Forget(cFC4PathFinder* pf) {
    pathfinders_.erase(std::remove(pathfinders_.begin(), pathfinders_.end(), pf), pathfinders_.end());
}

bool cFC4TrafficSimulator::Owns(const cFC4ISC4PathFinder* pf) {
    for (cFC4PathFinder* p : pathfinders_) {
        if (static_cast<const cFC4ISC4PathFinder*>(p) == pf) return true;
    }
    return false;
}

// ---- the lock ---------------------------------------------------------------------------------------------

bool cFC4TrafficSimulator::Enter() {
    const DWORD me = GetCurrentThreadId();
    if (mainTid_ == 0 || me == mainTid_) {
        EnterCriticalSection(&lock_);
        return true;
    }
    if (TryEnterCriticalSection(&lock_)) return true;
    const ULONGLONG until = GetTickCount64() + kLockWaitMs;
    while (GetTickCount64() < until) {
        Sleep(1);
        if (TryEnterCriticalSection(&lock_)) return true;
    }
    ++fallbacks_;
    return false;
}

void cFC4TrafficSimulator::Leave() { LeaveCriticalSection(&lock_); }

cFC4TrafficSimulator::Section::Section(const cFC4TrafficSimulator* s)
    : s_(const_cast<cFC4TrafficSimulator*>(s)), ok_(s_->Enter()) {
    if (ok_) ++s_->depth_;
}

cFC4TrafficSimulator::Section::~Section() {
    if (!ok_) return;
    if (--s_->depth_ == 0) {
        // The outermost call: the references and caches kept for it go, and
        // the values other threads fall back to are refreshed.
        s_->ReleaseHeld();
        s_->RefreshSnapshot();
    }
    s_->Leave();
}

void cFC4TrafficSimulator::ReleaseHeld() {
    std::vector<cIGZUnknown*> held;
    held.swap(ctx_.held);
    std::vector<std::shared_ptr<void>> lists;
    lists.swap(ctx_.heldLists);
    ctx_.lotList.clear();
    ctx_.lotListValid = false;
    ctx_.liveLots.clear();
    ctx_.seenLots.clear();
    ctx_.liveConns.clear();
    ctx_.seenConns.clear();
    ctx_.liveConnsValid = false;
    for (cIGZUnknown* o : held) FC4_RELEASE(o);
    lists.clear();  // each list releases its elements
}

// Only the core's own state (no host calls): any thread holding the lock.
void cFC4TrafficSimulator::RefreshSnapshot() {
    if (core_ == nullptr) return;
    fc4_traffic_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_traffic_get_state(core_, &st)) return;
    snap_.maps[FC4_TRAFFIC_MAP_AIR] = reinterpret_cast<cISC4SimGrid<uint8_t>*>(FromToken(st.air));
    snap_.maps[FC4_TRAFFIC_MAP_COMMERCIAL] = reinterpret_cast<cISC4SimGrid<uint8_t>*>(FromToken(st.commercial));
    snap_.maps[FC4_TRAFFIC_MAP_CONGESTION] = reinterpret_cast<cISC4SimGrid<uint8_t>*>(FromToken(st.congestion));
    snap_.maps[FC4_TRAFFIC_MAP_TRIP_LENGTH] = reinterpret_cast<cISC4SimGrid<uint8_t>*>(FromToken(st.trip_length));
    const float scalars[4] = {st.trip_scale, st.trip_scale_for_display, st.last_monthly_income, st.freight_scaling};
    for (int i = 0; i < 4; ++i) {
        uint32_t bits = 0;
        std::memcpy(&bits, &scalars[i], sizeof(bits));
        snap_.scalarBits[i] = bits;
    }
    for (int w = 0; w < 4; ++w) {
        for (int s = 0; s < 3; ++s) {
            uint32_t bits = 0;
            std::memcpy(&bits, &st.pct[w][s], sizeof(bits));
            snap_.pctBits[w][s] = bits;
        }
    }
    snap_.cycleState = st.cycle_state;
    snap_.day = st.day_index;
}

namespace {

const char* StateName(uint32_t s) {
    switch (s) {
        case 0: return "StartRecalc";
        case 1: return "lot sweep";
        case 2: return "CreateMaps";
        case 3: return "ReduceTransitSwitches";
        case 4: return "CreateFinalMap";
        case 5: return "ClearTripDestinations";
        case 6: return "ReduceEdgeDensities";
        case 7: return "DumpEdgeDensities";
        case 8: return "FinalCalculations";
        case 10: return "RefreshCommuteMap";
        case 11: return "UpdateNeighborConnections";
        case 12: return "UpdateSubnetworks";
        case 13: return "UpdateSubnetworkInfo";
        case 15: return "FinishTripMap";
        case 16: return "ConfirmTrips";
        case 17: return "ReRouteTrips";
        case 18: return "FullTrips";
        case 19: return "FerryRoutes";
        default: return "-";
    }
}

// The game date ("-" without a simulator).
void DateText(cISC4City* city, char* out, size_t cap) {
    cISC4Simulator* s = city ? city->GetSimulator() : nullptr;
    if (s == nullptr) {
        _snprintf_s(out, cap, _TRUNCATE, "-");
        return;
    }
    fc4_date d{};
    fc4_date_from_julian(fc4::AsGame(s)->GetSimDateNumber(), &d);
    _snprintf_s(out, cap, _TRUNCATE, "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
}

}  // namespace

// The status lines (Init, Read, Write, Shutdown). Inside a section.
void cFC4TrafficSimulator::LogState(const char* what) {
    fc4_traffic_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_traffic_get_state(core_, &st)) {
        fc4::Log("traffic %s: no core", what);
        return;
    }
    lastCycleState_ = st.cycle_state;  // the next cycle line starts from the state logged here
    fc4::Log("traffic %s: init=%u loaded=%u state=%u (%s) strike=%u day=%u/%u total=%d processed=%d passTotal=%d "
             "reroute=%d search=%d units=%u scale=%.4f/%.4f income=%.0f freight=%.4f attempted=%u,%u,%u "
             "routed=%u,%u,%u free=%u,%u,%u,%u switches=%u ferries=%u potholes=%u damaged=%u/%u edges=%u "
             "maps=%p,%p,%p,%p depts=%p,%p posts=%u/%u native=%u trips r%u/w%u (failed %u) holders=%u in place, %u "
             "added budget=%u depts=%u lines=%u lots master=%u rect=%u stale=%u "
             "held conns=%u (left %u) reentrant=%u queued=%u pathfinders=%u (deques filled %u failed %u) "
             "fallbacks=%u (messages dropped %u, slot 19 direct %u)",
             what, st.initialized, st.loaded, st.cycle_state, StateName(st.cycle_state), st.strike_state,
             st.day_index, st.days, st.total, st.processed, st.pass_total, st.to_reroute, st.to_search,
             st.units_per_slice, st.trip_scale, st.trip_scale_for_display, st.last_monthly_income, st.freight_scaling,
             st.attempted[0], st.attempted[1], st.attempted[2], st.routed[0], st.routed[1], st.routed[2],
             st.free_dest[0], st.free_dest[1], st.free_dest[2], st.free_dest[3], st.switches, st.ferry_terminals,
             st.potholes, st.road_damaged, st.rail_damaged, st.edge_records, static_cast<void*>(FromToken(st.air)),
             static_cast<void*>(FromToken(st.commercial)), static_cast<void*>(FromToken(st.congestion)),
             static_cast<void*>(FromToken(st.trip_length)), static_cast<void*>(FromToken(st.roads)),
             static_cast<void*>(FromToken(st.mass_transit)), st.posted, ctx_.posts, ctx_.sourceUpdates,
             ctx_.tripReads, ctx_.tripWrites, ctx_.tripWriteFailures, ctx_.holderWritesInPlace, ctx_.holderWrites,
             ctx_.budgetActivations, ctx_.departments, ctx_.lineItems, ctx_.lotWalksMaster, ctx_.lotWalksRect,
             ctx_.staleTokens, static_cast<unsigned>(ctx_.heldConns.size()), ctx_.connsLeftHeld,
             st.reentrant_calls, st.queued_messages, externalPathFinders_, dequeFills_, dequeFailures_,
             fallbacks_.load(), droppedMessages_.load(), directCapacityWrites_.load());
}

// After every message (inside its section): the work-cycle line when the
// state changed, the month line after the month agent.
void cFC4TrafficSimulator::AfterMessage(uint32_t type) {
    if (core_ == nullptr) return;
    fc4_traffic_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_traffic_get_state(core_, &st)) return;
    if (type == kMsgDay) ++days_;
    if (st.cycle_state != lastCycleState_) {
        char date[24];
        DateText(ctx_.city, date, sizeof(date));
        if (lastCycleState_ == 0xFFFFFFFFu) {
            fc4::Log("traffic cycle %s: state %u (%s), processed %d of %d, day %u of %u", date, st.cycle_state,
                     StateName(st.cycle_state), st.processed, st.total, st.day_index, st.days);
        } else {
            fc4::Log("traffic cycle %s: state %u (%s) -> %u (%s), processed %d of %d, day %u of %u, %u game days so "
                     "far",
                     date, lastCycleState_, StateName(lastCycleState_), st.cycle_state, StateName(st.cycle_state),
                     st.processed, st.total, st.day_index, st.days, days_);
        }
        lastCycleState_ = st.cycle_state;
    }
    if (type != kMsgMonth) return;
    ++months_;
    char date[24];
    DateText(ctx_.city, date, sizeof(date));
    char types[160];
    size_t used = 0;
    uint32_t routes = 0;
    types[0] = 0;
    for (int t = 0; t < 9; ++t) {
        routes += st.type_counts[t];
        const int w = _snprintf_s(types + used, sizeof(types) - used, _TRUNCATE, "%s%u", t ? "," : "", st.type_counts[t]);
        if (w < 0) break;
        used += static_cast<size_t>(w);
    }
    fc4::Log("traffic month %s (#%u): trips attempted=%u,%u,%u routed=%u,%u,%u; sources updated=%u, trip lists "
             "read=%u written=%u; routes applied=%u by travel type %s; income=%.0f (transit %.0f, tolls %.0f); "
             "car commute %.2f; accidents=%u (prone cells %u); map averages air=%.3f commercial=%.3f congestion=%.3f "
             "trip length=%.3f; state %u (%s)",
             date, months_, st.attempted[0], st.attempted[1], st.attempted[2], st.routed[0], st.routed[1],
             st.routed[2], st.native_updates, st.trip_reads, st.trip_writes, routes, types, st.last_monthly_income,
             st.transit_income, st.toll_income, st.avg_car_commute, st.accidents_posted, st.accident_cells,
             fc4_traffic_map_average(core_, FC4_TRAFFIC_MAP_AIR), fc4_traffic_map_average(core_, FC4_TRAFFIC_MAP_COMMERCIAL),
             fc4_traffic_map_average(core_, FC4_TRAFFIC_MAP_CONGESTION),
             fc4_traffic_map_average(core_, FC4_TRAFFIC_MAP_TRIP_LENGTH), st.cycle_state, StateName(st.cycle_state));
}

// ---- lifecycle (§1) -------------------------------------------------------------------------------------

bool cFC4TrafficSimulator::Init() {
    FC4_TRACE("TrafficSimulator::Init");
    mainTid_ = GetCurrentThreadId();
    Section sec(this);
    if (ctx_.city == nullptr) {
        ctx_.city = AppCity();
        if (ctx_.city != nullptr) {
            FC4_ADDREF(ctx_.city);
        } else {
            fc4::Log("traffic: no city at Init");
        }
    }
    if (ctx_.msgServer == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            ctx_.msgServer = ms;
            FC4_ADDREF(ctx_.msgServer);
        }
    }
    const char* source = nullptr;
    const std::string text = fc4::Setting("traffic_units_per_slice", "FC4_TRAFFIC_UNITS_PER_SLICE", &source);
    const long units = fc4::SettingInt("traffic_units_per_slice", "FC4_TRAFFIC_UNITS_PER_SLICE", 0);
    const uint32_t n = units > 0 ? static_cast<uint32_t>(units) : 0;
    const bool ok = core_ != nullptr && fc4_traffic_init(core_, n);
    if (n != 0) {
        fc4::Log("traffic Init: units per slice %u (%s; the core's default is %u, traffic-simulator.md §3.4)", n,
                 source && *source ? source : "setting", fc4_traffic_default_units_per_slice());
    } else {
        fc4::Log("traffic Init: units per slice %u (the core's default, traffic-simulator.md §3.4 [choice]%s%s)",
                 fc4_traffic_default_units_per_slice(), text.empty() ? "" : "; ignored setting ",
                 text.empty() ? "" : text.c_str());
    }
    LogState("Init");
    return ok;
}

bool cFC4TrafficSimulator::Shutdown() {
    FC4_TRACE("TrafficSimulator::Shutdown");
    // Keep this object alive while the message server drops its reference
    // (unless this already is the final Release).
    if (!destroying_) Primary()->AddRef();
    {
        Section sec(this);
        if (core_ != nullptr) fc4_traffic_shutdown(core_);
        Unsubscribe(&ctx_);
        RemoveAgents(&ctx_);
        LogState("Shutdown");
        ReleaseSlot(ctx_.msgServer);
        ReleaseSlot(ctx_.city);
        lastCycleState_ = 0xFFFFFFFFu;
    }
    if (!destroying_) Primary()->Release();
    return true;
}

uint32_t cFC4TrafficSimulator::GetSimulatorType() { return fc4_traffic_simulator_type(); }

// ---- messages (§3.1) --------------------------------------------------------------------------------------

bool cFC4TrafficSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr) return true;
    FC4_X87_SCOPE("traffic DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    if (core_ == nullptr) return true;
    Section sec(this);
    if (!sec.ok()) {
        // §9.1 TG21: the original handles every message on the delivering
        // thread and drops none. A message from another thread that cannot
        // get the lock is dropped, counted and logged: it must not happen in
        // normal play.
        const uint32_t n = ++droppedMessages_;
        if (n <= 3) {
            fc4::Log("traffic: WARNING message 0x%08X from another thread dropped (the lock is busy; %u so far)",
                     pMessage->GetType(), n);
        }
        return true;
    }
    const uint32_t type = pMessage->GetType();
    uint32_t d1 = 0, d2 = 0, d3 = 0;
    uint64_t object = 0;
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    if (s != nullptr) {
        d1 = static_cast<uint32_t>(s->GetData1());
        d2 = static_cast<uint32_t>(s->GetData2());
        d3 = static_cast<uint32_t>(s->GetData3());
        // §3.1: only two messages carry an object: the lot message (its
        // IGZUnknown) and the occupant messages (void1, the occupant). The
        // others are data only (their void1 is not read).
        cIGZUnknown* o = nullptr;
        if (type == kMsgLot) {
            o = s->GetIGZUnknown();
        } else if (type == kMsgInsertOccupant || type == kMsgRemoveOccupant) {
            o = static_cast<cIGZUnknown*>(s->GetVoid1());
        }
        if (o != nullptr) {
            // A message delivered during another call is queued by the core
            // and run when that call ends: its object stays alive until then.
            if (depth_ > 1) {
                FC4_ADDREF(o);
                ctx_.held.push_back(o);
            }
            object = TokenOf(o);
        }
    }
    const bool r = fc4_traffic_do_message(core_, type, d1, d2, d3, object);
    Drop(s);
    AfterMessage(type);
    return r;
}

// ---- persistence (§6) -------------------------------------------------------------------------------------

bool cFC4TrafficSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("TrafficSimulator::Write");
    Section sec(this);
    fc4::OStreamAdapter a(s);
    const bool ok = core_ != nullptr && fc4_traffic_write(core_, a.Get());
    fc4::Log("traffic Write -> %d", ok ? 1 : 0);
    LogState("Write");
    return ok;
}

// Runs before Init: the core sizes nothing from the city here.
bool cFC4TrafficSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("TrafficSimulator::Read");
    Section sec(this);
    fc4::IStreamAdapter a(s);
    const bool ok = core_ != nullptr && fc4_traffic_read(core_, a.Get());
    fc4::Log("traffic Read -> %d", ok ? 1 : 0);
    LogState("Read");
    return ok;
}

uint32_t cFC4TrafficSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4TrafficSimulator; }

// ---- cISC4TrafficSimulator (§7) -----------------------------------------------------------------------------

// Slot 6 (pathfinder.md §1): a new, initialised pathfinder with one
// reference for the caller. [question] A null out pointer: false.
bool cFC4TrafficSimulator::CreatePathFinder(cFC4ISC4PathFinder** out) {
    if (out == nullptr) return false;
    Section sec(this);
    if (!sec.ok()) {
        *out = nullptr;  // [choice] another thread that cannot get the lock: none
        return false;
    }
    auto* p = new cFC4PathFinder(this);
    pathfinders_.push_back(p);
    p->AddRef();
    *out = p;
    ++externalPathFinders_;
    FC4_LOGV("traffic CreatePathFinder: %p (core %p), %u handed out, %ld alive", static_cast<void*>(p),
             static_cast<void*>(p->Core()), externalPathFinders_, g_livePathFinders.load());
    return true;
}

// Slot 7. [question] A pathfinder that is not one of this simulator's: false.
bool cFC4TrafficSimulator::SetupPathFinderForLot(cFC4ISC4PathFinder* pf, cISC4Lot* lot) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr || pf == nullptr || !Owns(pf)) return false;
    auto* ours = static_cast<cFC4PathFinder*>(pf);
    if (ours->Core() == nullptr) return false;
    return fc4_traffic_setup_path_finder_for_lot(core_, ours->Core(), SeenLot(&ctx_, lot));
}

cISC4SimGrid<uint8_t>* cFC4TrafficSimulator::GetAirPollutingTrafficMap() {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return snap_.maps[FC4_TRAFFIC_MAP_AIR].load();
    return reinterpret_cast<cISC4SimGrid<uint8_t>*>(FromToken(fc4_traffic_map(core_, FC4_TRAFFIC_MAP_AIR)));
}

cISC4SimGrid<uint8_t>* cFC4TrafficSimulator::GetCommercialTrafficMap() {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return snap_.maps[FC4_TRAFFIC_MAP_COMMERCIAL].load();
    return reinterpret_cast<cISC4SimGrid<uint8_t>*>(FromToken(fc4_traffic_map(core_, FC4_TRAFFIC_MAP_COMMERCIAL)));
}

cISC4SimGrid<uint8_t>* cFC4TrafficSimulator::GetCongestionMap() {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return snap_.maps[FC4_TRAFFIC_MAP_CONGESTION].load();
    return reinterpret_cast<cISC4SimGrid<uint8_t>*>(FromToken(fc4_traffic_map(core_, FC4_TRAFFIC_MAP_CONGESTION)));
}

cISC4SimGrid<uint8_t>* cFC4TrafficSimulator::GetTripLengthMap() {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return snap_.maps[FC4_TRAFFIC_MAP_TRIP_LENGTH].load();
    return reinterpret_cast<cISC4SimGrid<uint8_t>*>(FromToken(fc4_traffic_map(core_, FC4_TRAFFIC_MAP_TRIP_LENGTH)));
}

// Slot 11 (ST0). [choice] Another thread that cannot get the lock: 0.
float cFC4TrafficSimulator::GetBackgroundTraffic(int32_t x, int32_t z) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return 0.0f;
    return fc4_traffic_background_traffic(core_, x, z);
}

namespace {
FC4TrafficEdgeDensity g_zeroEdge{};  // the shared zero record (never written)
}

// Slot 12: a record from a ring of kEdgeRing stable buffers, or the shared
// zero record. [question] How long a caller keeps the pointer is not
// specified; the ring keeps each record for the next 63 calls.
FC4TrafficEdgeDensity* cFC4TrafficSimulator::GetTrafficEdgeDensity(uint8_t x, uint8_t z, uint32_t travelType,
                                                                   bool evening) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return &g_zeroEdge;
    uint16_t v[33] = {};
    if (!fc4_traffic_edge_density(core_, x, z, travelType, evening, v)) return &g_zeroEdge;
    FC4TrafficEdgeDensity* rec = &edges_[edgeNext_ % kEdgeRing];
    ++edgeNext_;
    static_assert(sizeof(v) == sizeof(FC4TrafficEdgeDensity), "33 u16");
    std::memcpy(rec, v, sizeof(v));
    return rec;
}

float cFC4TrafficSimulator::Scalar(uint32_t which) {
    Section sec(this);
    if (sec.ok() && core_ != nullptr) return fc4_traffic_scalar(core_, which);
    const uint32_t bits = which < 4 ? snap_.scalarBits[which].load() : 0;
    float f = 0.0f;
    std::memcpy(&f, &bits, sizeof(f));
    return f;
}

float cFC4TrafficSimulator::GetTripScale() { return Scalar(0); }
float cFC4TrafficSimulator::GetTripScaleForDisplay() { return Scalar(1); }
float cFC4TrafficSimulator::GetLastMonthlyIncome() { return Scalar(2); }
float cFC4TrafficSimulator::GetFreightScalingFactor() { return Scalar(3); }

bool cFC4TrafficSimulator::IsRoadDamaged(int32_t x, int32_t z) {
    Section sec(this);
    return sec.ok() && core_ != nullptr && fc4_traffic_is_road_damaged(core_, x, z);
}

bool cFC4TrafficSimulator::CheckRailAccident(int32_t x, int32_t z) {
    Section sec(this);
    return sec.ok() && core_ != nullptr && fc4_traffic_check_rail_accident(core_, x, z);
}

namespace {
uint32_t TripCapacityElement(cISCPropertyHolder* holder, uint32_t i);
bool WriteTripCapacityElement(cISCPropertyHolder* holder, uint32_t i, uint32_t value);
}  // namespace

// Slot 19 (void). §9.1 TG15 / TG21: the original uses no simulator state and
// no lock, only the holder's property 0x8A0B4100, so another thread that
// cannot get the lock writes the holder directly (the same rule as the core:
// a u16 array unless a value needs u32).
void cFC4TrafficSimulator::SetMaxTripCapacity(cISCPropertyHolder* holder, uint32_t k, uint32_t value) {
    Section sec(this);
    if (holder == nullptr) return;
    if (!sec.ok()) {
        if (WriteTripCapacityElement(holder, k, value)) ++directCapacityWrites_;
        return;
    }
    if (core_ == nullptr) return;
    fc4_traffic_set_max_trip_capacity(core_, TokenOf(holder), k, value);
}

namespace {

// Slots 20 / 21 on a thread that cannot get the lock (§4.2): element `i` of
// the holder's 0x8A0B4100, a 12-element u16 or u32 array; else 0.
// [question] the original reads the holder directly on any thread.
uint32_t TripCapacityElement(cISCPropertyHolder* holder, uint32_t i) {
    if (holder == nullptr || i >= 12) return 0;
    cISCProperty* p = fc4::AsGame(holder)->GetProperty(kTripCapacityProperty);
    if (p == nullptr) return 0;
    cIGZVariant* raw = p->GetPropertyValue();
    if (raw == nullptr) return 0;
    cFC4IGZVariant* v = fc4::AsGame(raw);
    if (v->GetCount() != 12) return 0;
    const uint8_t* data = v->RefUint8();
    if (data == nullptr) return 0;
    switch (v->GetType()) {
        case cIGZVariant::Uint16Array: {
            uint16_t e = 0;
            std::memcpy(&e, data + 2 * i, sizeof(e));
            return e;
        }
        case cIGZVariant::Uint32Array: {
            uint32_t e = 0;
            std::memcpy(&e, data + 4 * i, sizeof(e));
            return e;
        }
        default: return 0;
    }
}

// Slot 19 without the lock (TG15): element `i` of the holder's 0x8A0B4100
// set, the others kept; a u32 array when the property is one or `value` is
// 0xFFFF or more, else a u16 array (traffic-simulator §4.2, as the core's
// set_capacity_element).
bool WriteTripCapacityElement(cISCPropertyHolder* holder, uint32_t i, uint32_t value) {
    if (holder == nullptr || i >= 12) return false;
    uint32_t vals[12] = {};
    bool wide = false;
    if (cISCProperty* p = fc4::AsGame(holder)->GetProperty(kTripCapacityProperty)) {
        cIGZVariant* raw = p->GetPropertyValue();
        cFC4IGZVariant* v = raw ? fc4::AsGame(raw) : nullptr;
        if (v != nullptr && v->GetCount() == 12) {
            const uint16_t t = v->GetType();
            if (t == cIGZVariant::Uint32Array) wide = true;
            if (t == cIGZVariant::Uint16Array || t == cIGZVariant::Uint32Array) {
                for (uint32_t j = 0; j < 12; ++j) vals[j] = TripCapacityElement(holder, j);
            }
        }
    }
    if (value >= 0xFFFF) wide = true;
    vals[i] = value;
    if (wide) {
        if (WriteInPlace(holder, kTripCapacityProperty, cIGZVariant::Uint32, vals, 12)) return true;
        return fc4::props::SetTyped(holder, kTripCapacityProperty, cIGZVariant::Uint32, true, vals, 12);
    }
    uint16_t narrow[12];
    for (uint32_t j = 0; j < 12; ++j) narrow[j] = static_cast<uint16_t>(vals[j]);
    if (WriteInPlace(holder, kTripCapacityProperty, cIGZVariant::Uint16, narrow, 12)) return true;
    return fc4::props::SetTyped(holder, kTripCapacityProperty, cIGZVariant::Uint16, true, narrow, 12);
}

}  // namespace

uint32_t cFC4TrafficSimulator::GetMaxTripCapacity(cISCPropertyHolder* holder, uint32_t k) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return k < 4 ? TripCapacityElement(holder, k) : 0;
    return fc4_traffic_max_trip_capacity(core_, TokenOf(holder), k);
}

uint32_t cFC4TrafficSimulator::GetTrafficArrived(cISCPropertyHolder* holder, uint32_t k) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return k < 4 ? TripCapacityElement(holder, 8 + k) : 0;
    return fc4_traffic_traffic_arrived(core_, TokenOf(holder), k);
}

// Slot 22. [question] The original hands out the lot manager's safe lot
// iterator during the lot sweep (state 1), which the cycle never enters
// (§3.2): FreeCity4 holds no iterator and answers null.
void* cFC4TrafficSimulator::GetDesiredLotInsertionPoint() {
    Section sec(this);
    if (sec.ok() && core_ != nullptr && fc4_traffic_desired_lot_insertion_point(core_)) {
        static volatile LONG notes = 0;
        NoteFew(&notes, "traffic: GetDesiredLotInsertionPoint asked in the lot sweep; no iterator held, null");
    }
    return nullptr;
}

float cFC4TrafficSimulator::GetCapacity(uint32_t transitNetwork, int32_t x, int32_t z) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return 0.0f;
    return fc4_traffic_capacity(core_, transitNetwork, x, z);
}

float cFC4TrafficSimulator::GetTravelTimeRatio(int32_t x, int32_t z, uint32_t travelType) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return 0.0f;
    return fc4_traffic_travel_time_ratio(core_, x, z, travelType);
}

uint32_t cFC4TrafficSimulator::GetConnectionCount(uint32_t transitNetwork, int32_t x, int32_t z) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return 0;
    return fc4_traffic_connection_count(core_, transitNetwork, x, z);
}

// Slot 26 (void, pop 8; §4.1.2): `*out` resized to 3 with B and filled.
void cFC4TrafficSimulator::GetTravelStrategyPercentages(uint32_t wealth, fc4::GameVector<float>* out) {
    float v[3] = {0.0f, 0.0f, 0.0f};
    {
        Section sec(this);
        if (sec.ok() && core_ != nullptr) {
            fc4_traffic_travel_strategy_percentages(core_, wealth, v);
        } else if (wealth < 4) {
            for (int s = 0; s < 3; ++s) {
                const uint32_t bits = snap_.pctBits[wealth][s].load();
                std::memcpy(&v[s], &bits, sizeof(float));
            }
        }
    }
    if (out == nullptr || !AllocatorOk("GetTravelStrategyPercentages' vector")) return;
    Vec<float>& o = AsVec(*out);
    o.clear();
    Append(o, v, 3);
}

// Slot 28 (allocators.md): the caller's ilist (allocator A) is cleared (each
// element released, its node freed with A), then one reference per switch
// building is appended.
bool cFC4TrafficSimulator::GetTransitSwitches(SC4List<cISC4Occupant*>& list) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return false;
    while (!list.empty()) {
        auto it = list.begin();
        cISC4Occupant*& o = *it;
        if (o != nullptr) {
            FC4_ACQUIRED(o, "the caller's GetTransitSwitches list entry");
            FC4_RELEASE(o);
            o = nullptr;
        }
        list.erase(it);
    }
    std::vector<uint64_t> tokens;
    fc4_traffic_transit_switches(core_, &PushTokenVec, &tokens);
    for (uint64_t t : tokens) {
        cISC4Occupant* o = OccOf(t);
        if (o == nullptr) continue;
        FC4_ADDREF_FOR_CALLER(o);
        list.push_back(o);
    }
    return true;
}

// Slot 29: a vector the simulator owns (one per tile pair, replaced at the
// next call for the same pair), or null. [question] how long the caller may
// keep it is not specified.
fc4::GameVector<uint8_t>* cFC4TrafficSimulator::GetFerryRouteBetweenTiles(int32_t x1, int32_t z1, int32_t x2,
                                                                          int32_t z2) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr || !AllocatorOk("GetFerryRouteBetweenTiles' vector")) return nullptr;
    auto* v = new Vec<uint8_t>();
    if (!fc4_traffic_ferry_route_between_tiles(core_, x1, z1, x2, z2, &PushBytes, v)) {
        delete v;
        return nullptr;
    }
    const uint64_t key = (static_cast<uint64_t>(static_cast<uint16_t>(x1)) << 48) |
                         (static_cast<uint64_t>(static_cast<uint16_t>(z1)) << 32) |
                         (static_cast<uint64_t>(static_cast<uint16_t>(x2)) << 16) | static_cast<uint16_t>(z2);
    auto it = ferryVectors_.find(key);
    if (it != ferryVectors_.end()) {
        delete static_cast<Vec<uint8_t>*>(it->second);
        it->second = v;
    } else {
        ferryVectors_[key] = v;
    }
    return reinterpret_cast<fc4::GameVector<uint8_t>*>(v);
}

namespace {

// The ferry lists' nodes (allocators.md): {next, prev, value}.
struct RouteNode {
    RouteNode* next;
    RouteNode* prev;
    FC4TripVector route;  // vector<uint8_t>: begin, end, capacity end
};
static_assert(sizeof(void*) != 4 || sizeof(RouteNode) == 0x14, "0x14-byte nodes");

struct FerryInfo {
    cISC4Lot* lotA;              // +0x00 (one reference)
    cISC4Lot* lotB;              // +0x04 (one reference, may be null)
    uint32_t type;               // +0x08
    uint16_t counter1[2];        // +0x0C
    uint16_t counter2[2];        // +0x10
    uint8_t neighbour;           // +0x14
    uint8_t pad;                 // +0x15
    int16_t neighbourX;          // +0x16
    int16_t neighbourZ;          // +0x18
    uint16_t pad2;               // +0x1A
    FC4TripVector routeAB;       // +0x1C
    FC4TripVector routeBA;       // +0x28
};
static_assert(sizeof(void*) != 4 || sizeof(FerryInfo) == 0x34, "FerryRouteInfo is 0x34 bytes");

struct FerryNode {
    FerryNode* next;
    FerryNode* prev;
    FerryInfo info;
};
static_assert(sizeof(void*) != 4 || sizeof(FerryNode) == 0x3C, "0x3C-byte nodes");

// Checks the ring (at most kMaxListNodes nodes, each link pointing back).
template <class Node>
bool RingOk(Node* head) {
    if (head == nullptr || head->next == nullptr || head->next->prev != head) return false;
    uint32_t n = 0;
    for (Node* node = head->next; node != head; node = node->next) {
        if (node == nullptr || node->next == nullptr || node->next->prev != node || ++n > kMaxListNodes) return false;
    }
    return true;
}

template <class Node>
void Link(Node* head, Node* node) {
    node->next = head;
    node->prev = head->prev;
    head->prev->next = node;
    head->prev = node;
}

void ReleaseLot(cISC4Lot* lot) {
    if (lot == nullptr) return;
    FC4_ACQUIRED(lot, "the caller's FerryRouteInfo lot");
    FC4_RELEASE(lot);
}

struct RouteSink {
    RouteNode* head;
    bool ok;
};

void AppendRoute(void* ctx, const uint8_t* bytes, size_t len) {
    auto* s = static_cast<RouteSink*>(ctx);
    if (!s->ok) return;
    auto* node = static_cast<RouteNode*>(TrafficAlloc::Allocate(static_cast<uint32_t>(sizeof(RouteNode))));
    if (node == nullptr || !CopyRoute(node->route, bytes, len)) {
        TrafficAlloc::Free(node);
        s->ok = false;
        return;
    }
    Link(s->head, node);
}

struct FerrySink {
    FerryNode* head;
    bool ok;
};

void AppendFerry(void* ctx, const fc4_traffic_ferry_route* info, const uint8_t* ab, size_t lenAB, const uint8_t* ba,
                 size_t lenBA) {
    auto* s = static_cast<FerrySink*>(ctx);
    if (!s->ok || info == nullptr) return;
    auto* node = static_cast<FerryNode*>(TrafficAlloc::Allocate(static_cast<uint32_t>(sizeof(FerryNode))));
    if (node == nullptr) {
        s->ok = false;
        return;
    }
    std::memset(static_cast<void*>(node), 0, sizeof(FerryNode));
    FerryInfo& f = node->info;
    if (!CopyRoute(f.routeAB, ab, lenAB) || !CopyRoute(f.routeBA, ba, lenBA)) {
        FreeRoute(f.routeAB);
        TrafficAlloc::Free(node);
        s->ok = false;
        return;
    }
    f.lotA = reinterpret_cast<cISC4Lot*>(FromToken(info->lot_a));
    f.lotB = reinterpret_cast<cISC4Lot*>(FromToken(info->lot_b));
    if (f.lotA != nullptr) FC4_ADDREF_FOR_CALLER(f.lotA);
    if (f.lotB != nullptr) FC4_ADDREF_FOR_CALLER(f.lotB);
    f.type = info->ferry_type;
    f.counter1[0] = info->counter1[0];
    f.counter1[1] = info->counter1[1];
    f.counter2[0] = info->counter2[0];
    f.counter2[1] = info->counter2[1];
    f.neighbour = info->neighbour;
    f.neighbourX = info->neighbour_x;
    f.neighbourZ = info->neighbour_z;
    Link(s->head, node);
}

}  // namespace

// Slot 30 (allocators.md): the caller's list of vector<u8> (B) is cleared,
// then a copy of every route appended.
bool cFC4TrafficSimulator::GetAllFerryRoutes(void* list) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr || list == nullptr || !AllocatorOk("GetAllFerryRoutes' list")) return false;
    RouteNode* head = *static_cast<RouteNode**>(list);
    if (!RingOk(head)) {
        static volatile LONG notes = 0;
        NoteFew(&notes, "traffic: WARNING GetAllFerryRoutes: the caller's list failed the checks; left alone");
        return false;
    }
    for (RouteNode* n = head->next; n != head;) {
        RouteNode* next = n->next;
        FreeRoute(n->route);
        TrafficAlloc::Free(n);
        n = next;
    }
    head->next = head->prev = head;
    RouteSink sink{head, true};
    fc4_traffic_all_ferry_routes(core_, &AppendRoute, &sink);
    return true;
}

// Slot 31 (allocators.md): the caller's list of FerryRouteInfo (B) is
// cleared (both lots released), then a copy of every route in use appended
// with one reference on each lot.
bool cFC4TrafficSimulator::GetFerryRoutesInUse(void* list) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr || list == nullptr || !AllocatorOk("GetFerryRoutesInUse' list")) return false;
    FerryNode* head = *static_cast<FerryNode**>(list);
    if (!RingOk(head)) {
        static volatile LONG notes = 0;
        NoteFew(&notes, "traffic: WARNING GetFerryRoutesInUse: the caller's list failed the checks; left alone");
        return false;
    }
    for (FerryNode* n = head->next; n != head;) {
        FerryNode* next = n->next;
        ReleaseLot(n->info.lotA);
        ReleaseLot(n->info.lotB);
        FreeRoute(n->info.routeAB);
        FreeRoute(n->info.routeBA);
        TrafficAlloc::Free(n);
        n = next;
    }
    head->next = head->prev = head;
    FerrySink sink{head, true};
    fc4_traffic_ferry_routes_in_use(core_, &AppendFerry, &sink);
    return true;
}

uint32_t cFC4TrafficSimulator::GetFerryTerminalCount(uint32_t type) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return 0;
    return fc4_traffic_ferry_terminal_count(core_, type);
}

// Slot 33: the caller's vector<u8> (B) is cleared, then filled.
bool cFC4TrafficSimulator::GetWaterRoute(int32_t x1, int32_t z1, int32_t x2, int32_t z2,
                                         fc4::GameVector<uint8_t>& out) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return false;
    Vec<uint8_t>& o = AsVec(out);
    o.clear();
    if (!AllocatorOk("GetWaterRoute's vector")) return false;
    return fc4_traffic_water_route(core_, x1, z1, x2, z2, &PushBytes, &o);
}

// Slot 34: the statistics block (FC4_TRAFFIC_STATS_BLOCK bytes), a member
// of this object refreshed at each call.
const uint8_t* cFC4TrafficSimulator::GetTrafficStats() {
    Section sec(this);
    if (sec.ok() && core_ != nullptr) fc4_traffic_stats_block(core_, stats_, sizeof(stats_));
    return stats_;
}

bool cFC4TrafficSimulator::GetTransitSwitchQueryData(uint32_t x, uint32_t z, FC4TransitSwitchQueryData& out) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return false;
    fc4_traffic_switch_query q{};
    if (!fc4_traffic_transit_switch_query_data(core_, static_cast<int32_t>(x), static_cast<int32_t>(z), &q)) {
        return false;
    }
    out.available = q.available;
    out.maxCapacity = q.max_capacity;
    out.loadFraction = q.load_fraction;
    out.usage = q.usage;
    out.cost = q.cost;
    return true;
}

bool cFC4TrafficSimulator::AreLotsConnected(cISC4Lot* a, cISC4Lot* b) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return false;
    return fc4_traffic_are_lots_connected(core_, SeenLot(&ctx_, a), SeenLot(&ctx_, b));
}

uint32_t cFC4TrafficSimulator::GetConnectedOccupantCount(cISC4Lot* lot, uint32_t developerType) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return 0;
    return fc4_traffic_connected_occupant_count(core_, SeenLot(&ctx_, lot), developerType);
}

uint32_t cFC4TrafficSimulator::GetConnectedDestinationCount(cISC4Lot* lot, int32_t k) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return 0;
    return fc4_traffic_connected_destination_count(core_, SeenLot(&ctx_, lot), k);
}

// Slots 39-42 (allocators.md): the caller's vector<u32> (B) is cleared, then
// filled. A replacement returns true where the original returns garbage
// (traffic-network.md §7.1 [choice]).
bool cFC4TrafficSimulator::GetSubnetworksForLot(cISC4Lot* lot, fc4::GameVector<uint32_t>& out) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return false;
    Vec<uint32_t>& o = AsVec(out);
    o.clear();
    if (!AllocatorOk("GetSubnetworksForLot's vector")) return false;
    bool ok = false;
    fc4_traffic_subnetworks_for_lot(core_, SeenLot(&ctx_, lot), &PushU32, &o, &ok);
    return ok;
}

bool cFC4TrafficSimulator::GetSubnetworksInRectangle(SC4Rect<int32_t> const& rect, fc4::GameVector<uint32_t>& out) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return false;
    Vec<uint32_t>& o = AsVec(out);
    o.clear();
    if (!AllocatorOk("GetSubnetworksInRectangle's vector")) return true;
    const fc4_cell_rect r{rect.topLeftX, rect.topLeftY, rect.bottomRightX, rect.bottomRightY};
    fc4_traffic_subnetworks_in_rectangle(core_, &r, &PushU32, &o);
    return true;
}

bool cFC4TrafficSimulator::GetSubnetworksInRegion(SC4CellRegion<long> const& region, fc4::GameVector<uint32_t>& out) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return false;
    Vec<uint32_t>& o = AsVec(out);
    o.clear();
    if (!AllocatorOk("GetSubnetworksInRegion's vector")) return true;
    std::vector<uint8_t> mask;
    fc4_zone_region r{};
    if (!fc4::ReadZoneRegion(&region, mask, &r, "traffic")) return true;  // [choice] an unreadable region: none
    fc4_traffic_subnetworks_in_region(core_, &r, &PushU32, &o);
    return true;
}

bool cFC4TrafficSimulator::GetOccupantCountForAllSubnetworks(uint32_t developerType, fc4::GameVector<uint32_t>& out) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return false;
    Vec<uint32_t>& o = AsVec(out);
    o.clear();
    if (!AllocatorOk("GetOccupantCountForAllSubnetworks' vector")) return true;
    fc4_traffic_occupant_count_for_all_subnetworks(core_, developerType, &PushU32, &o);
    return true;
}

// ---- the pathfinders of slot 6 (pathfinder.md) ------------------------------------------------------------

cFC4PathFinder::cFC4PathFinder(cFC4TrafficSimulator* owner) : owner_(owner) {
    // Under the owner's lock (CreatePathFinder).
    pf_ = owner_ != nullptr && owner_->Core() != nullptr ? fc4_traffic_pf_create(owner_->Core()) : nullptr;
    ++g_livePathFinders;
}

cFC4PathFinder::~cFC4PathFinder() {
    fc4_traffic_pf_destroy(pf_);  // null once released or orphaned
    --g_livePathFinders;
}

void cFC4PathFinder::Orphan() {
    fc4_traffic_pf_destroy(pf_);
    pf_ = nullptr;
    owner_ = nullptr;
}

bool cFC4PathFinder::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr || (riid != kFC4_IID_cISC4PathFinder && riid != kIID_cIGZUnknown)) return false;
    *ppvObj = static_cast<cFC4ISC4PathFinder*>(this);
    ++refCount_;
    return true;
}

uint32_t cFC4PathFinder::AddRef() { return ++refCount_; }

// pathfinder.md §1: above 1 decrement; otherwise Shutdown if initialised,
// then destroy, return 0.
uint32_t cFC4PathFinder::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    if (cFC4TrafficSimulator* owner = owner_) {
        // Any thread waits for the lock here: the owner's list and its core
        // change (a pathfinder released from another thread is rare).
        EnterCriticalSection(&owner->lock_);
        if (pf_ != nullptr && fc4_traffic_pf_is_initialised(pf_)) fc4_traffic_pf_shutdown(pf_);
        owner->Forget(this);
        fc4_traffic_pf_destroy(pf_);
        pf_ = nullptr;
        owner_ = nullptr;
        LeaveCriticalSection(&owner->lock_);
    }
    delete this;
    return 0;
}

// Every slot holds the owner's lock; an orphan (or another thread that
// cannot get the lock) answers the neutral value.
#define PF_ENTER(neutral)                                    \
    if (owner_ == nullptr || pf_ == nullptr) return neutral; \
    cFC4TrafficSimulator::Section sec(owner_);               \
    if (!sec.ok() || owner_->Core() == nullptr) return neutral
#define PF_ENTER_VOID()                              \
    if (owner_ == nullptr || pf_ == nullptr) return; \
    cFC4TrafficSimulator::Section sec(owner_);       \
    if (!sec.ok() || owner_->Core() == nullptr) return

bool cFC4PathFinder::Set(uint32_t slot, uint32_t a, uint32_t b) {
    PF_ENTER(false);
    return fc4_traffic_pf_set(pf_, slot, a, b);
}

bool cFC4PathFinder::Init() {
    PF_ENTER(false);
    return fc4_traffic_pf_init(owner_->Core(), pf_);
}

bool cFC4PathFinder::Shutdown() {
    PF_ENTER(false);
    return fc4_traffic_pf_shutdown(pf_);
}

void cFC4PathFinder::SetFlag(uint32_t flags) { Set(5, flags, 0); }
void cFC4PathFinder::ClearFlag(uint32_t flags) { Set(6, flags, 0); }
void cFC4PathFinder::ClearAllFlags() { Set(7, 0, 0); }

// Slot 8: the caller's deque is remembered (FindPath below).
void cFC4PathFinder::SetRouteInfo(void* routeDeque) {
    PF_ENTER_VOID();
    routeDeque_ = routeDeque;
    fc4_traffic_pf_set_route_info(pf_, routeDeque != nullptr);
}

void cFC4PathFinder::SetTripDestinationFilter(uint32_t filter) { Set(9, filter, 0); }
void cFC4PathFinder::SetDestinationFlag(uint8_t bits) { Set(10, bits, 0); }
void cFC4PathFinder::ClearDestinationFlag(uint8_t bits) { Set(11, bits, 0); }
void cFC4PathFinder::ClearAllDestinationFlags() { Set(12, 0, 0); }
void cFC4PathFinder::SetTravelTypeFlag(uint16_t mask, bool alsoStarting) { Set(13, mask, alsoStarting ? 1u : 0u); }
void cFC4PathFinder::ClearTravelTypeFlag(uint16_t mask) { Set(14, mask, 0); }
void cFC4PathFinder::ClearAllTravelTypeFlags() { Set(15, 0, 0); }

void cFC4PathFinder::SetStartingCostArray(float* costs9) {
    PF_ENTER_VOID();
    fc4_traffic_pf_set_starting_cost_array(pf_, costs9);
}

void cFC4PathFinder::SetMaxTripLength(float length) {
    PF_ENTER_VOID();
    fc4_traffic_pf_set_max_trip_length(pf_, length);
}

void cFC4PathFinder::SetTrafficDensity(uint32_t trips) { Set(18, trips, 0); }

void cFC4PathFinder::SetStartRect(SC4Rect<int32_t> rect) {
    PF_ENTER_VOID();
    const fc4_cell_rect r{rect.topLeftX, rect.topLeftY, rect.bottomRightX, rect.bottomRightY};
    fc4_traffic_pf_set_rect(pf_, false, &r);
}

void cFC4PathFinder::SetEndRect(SC4Rect<int32_t> rect) {
    PF_ENTER_VOID();
    const fc4_cell_rect r{rect.topLeftX, rect.topLeftY, rect.bottomRightX, rect.bottomRightY};
    fc4_traffic_pf_set_rect(pf_, true, &r);
}

void cFC4PathFinder::SetMaxWorkTime(uint32_t ms) { Set(21, ms, 0); }
void cFC4PathFinder::Reset() { Set(22, 0, 0); }

// Slot 23. After a recorded trip (1, also the flag-2 fallback of §4.12) the
// caller's route deque is cleared and filled root -> result in place
// (pathfinder.md §4.13 step 5 and §7; TG1), with allocator B.
namespace {

struct RouteBytes {
    std::vector<uint8_t> bytes;
};

void CollectRoute(void* ctx, const uint8_t* b, size_t n) {
    auto* r = static_cast<RouteBytes*>(ctx);
    r->bytes.insert(r->bytes.end(), b, b + n);
}

struct RouteFill {
    fc4::RouteDeque* deque;
    const uint8_t* route;
    size_t count;
    fc4::RouteFillStatus status;
};

void RouteFillFn(void* c) {
    auto* f = static_cast<RouteFill*>(c);
    f->status = fc4::RouteDequeFill<TrafficAlloc>(*f->deque, f->route, f->count);
}

}  // namespace

uint32_t cFC4PathFinder::FindPath() {
    PF_ENTER(0u);
    const uint32_t r = fc4_traffic_pf_find_path(owner_->Core(), pf_);
    if (r == 1 && routeDeque_ != nullptr) FillRouteDeque();
    return r;
}

void cFC4PathFinder::FillRouteDeque() {
    if (!AllocatorOk("FindPath route deque")) return;
    RouteBytes rb;
    const size_t n = fc4_traffic_pf_route(pf_, &CollectRoute, &rb);
    if (rb.bytes.size() != n * fc4::kRouteDataBytes) return;
    RouteFill f{static_cast<fc4::RouteDeque*>(routeDeque_), rb.bytes.data(), n, fc4::RouteFillStatus::Ok};
    const bool clean = fc4::SehGuard("traffic route deque fill", &RouteFillFn, &f);
    if (clean && f.status == fc4::RouteFillStatus::Ok) {
        ++owner_->dequeFills_;
        return;
    }
    ++owner_->dequeFailures_;
    static volatile LONG notes = 0;
    NoteFew(&notes, !clean ? "traffic: WARNING FindPath route deque fill faulted (deque left as written so far)"
                    : f.status == fc4::RouteFillStatus::NoMap
                        ? "traffic: WARNING FindPath route deque has no map; left untouched"
                        : "traffic: WARNING FindPath route deque fill out of memory (partial route)");
}

uint32_t cFC4PathFinder::GetTrafficArrived() {
    PF_ENTER(0u);
    return fc4_traffic_pf_traffic_arrived(pf_);
}

float cFC4PathFinder::GetTripLength() {
    PF_ENTER(0.0f);
    return fc4_traffic_pf_trip_length(pf_);
}

float cFC4PathFinder::GetTripLengthScaledForDisplay() {
    PF_ENTER(0.0f);
    return fc4_traffic_pf_trip_length_for_display(owner_->Core(), pf_);
}

void cFC4PathFinder::GetEndCell(int32_t& x, int32_t& z) const {
    x = 0;
    z = 0;
    if (owner_ == nullptr || pf_ == nullptr) return;
    cFC4TrafficSimulator::Section sec(owner_);
    if (!sec.ok()) return;
    fc4_traffic_pf_end_cell(pf_, &x, &z);
}

uint8_t cFC4PathFinder::GetLastTravelType() const {
    if (owner_ == nullptr || pf_ == nullptr) return 0;
    cFC4TrafficSimulator::Section sec(owner_);
    if (!sec.ok()) return 0;
    return fc4_traffic_pf_last_travel_type(pf_);
}

void cFC4PathFinder::ApplyTraffic(uint32_t style) {
    PF_ENTER_VOID();
    fc4_traffic_pf_apply_traffic(owner_->Core(), pf_, style);
}

bool cFC4PathFinder::WasTripReversible() {
    PF_ENTER(false);
    return fc4_traffic_pf_was_trip_reversible(owner_->Core(), pf_);
}

void cFC4PathFinder::ReleasePathNodes() {
    PF_ENTER_VOID();
    fc4_traffic_pf_release_path_nodes(pf_);
}

// Slot 32 (allocators.md): the caller's vector<u8> (B) is cleared (its
// buffer kept), then filled; `mask` is the travel types on the route.
void cFC4PathFinder::GetCompressedRoute(fc4::GameVector<uint8_t>& out, uint32_t& mask) {
    Vec<uint8_t>& o = AsVec(out);
    o.clear();
    mask = 0;
    PF_ENTER_VOID();
    if (!AllocatorOk("GetCompressedRoute's vector")) return;
    mask = fc4_traffic_pf_compressed_route(owner_->Core(), pf_, &PushBytes, &o);
}

#undef PF_ENTER
#undef PF_ENTER_VOID

// ---- test seam op 42 --------------------------------------------------------------------------------------

intptr_t TrafficTestSeam(const FC4TrafficTestInput* in) {
    if (in == nullptr) return -1;
    if (in->op == kFC4TrafficTestHooks) {
        g_testAllocate = in->allocate;
        g_testRelease = in->release;
        if (in->traffic == nullptr) return 0;
        cFC4TrafficSimulator* t = cFC4TrafficSimulator::FromPrimary(in->traffic);
        if (t == nullptr) return -1;
        t->SetTestHooks(in->host, in->postSink, in->postSinkContext);
        return 1;
    }
    if (in->op == kFC4TrafficTestLivePathFinders) return static_cast<intptr_t>(cFC4TrafficSimulator::LivePathFinders());
    return -1;
}
