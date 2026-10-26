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

#include "cFC4Neighbors.h"

#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Seed.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Date.h"
#include "cFC4Message2Standard.h"
#include "compat/FC4Compat.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "SC4Point.h"
#include "SC4Rect.h"
#include "cGZPersistResourceKey.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZLanguageManager.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZPersistResourceManager.h"
#include "cIGZString.h"
#include "cISC4App.h"
#include "cISC4BudgetSimulator.h"
#include "cISC4City.h"
#include "cISC4DepartmentBudget.h"
#include "cISC4LandfillDeveloper.h"
#include "cISC4LineItem.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4PlumbingSimulator.h"
#include "cISC4PollutionSimulator.h"
#include "cISC4PowerSimulator.h"
#include "cISC4PropManager.h"
#include "cISC4PropOccupant.h"
#include "cISC4Region.h"
#include "cISC4RegionalCity.h"
#include "cISC4Simulator.h"
#include "cISCPropertyHolder.h"
#include "cISTETerrain.h"
#include "cRZAutoRefCount.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"

#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <string>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cISC4NetworkOccupant = 0xA821EF94;
constexpr uint32_t kIID_cISC4PowerPoleOccupant = 0xA8C86142;
// Private: "this object is one of our deals / connections" (never answered
// by the game's objects; not the core-object IID, which these must not answer:
// the core reads every stream reference as a host object).
constexpr uint32_t kIID_FC4NeighborDeal = 0x7F4C0B21;
constexpr uint32_t kIID_FC4NeighborConnection = 0x7F4C0B22;

// Messages (§11, §5.3).
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgPreDemolishNetworkOccupant = 0xAA7A014D;
constexpr uint32_t kMsgRemoveOccupant = 0x99EF1143;
constexpr uint32_t kMsgDealStatus = 0xEA418D8F;
constexpr uint32_t kMsgConnectionCreated = 0x8A0D15DA;
constexpr uint32_t kMsgConnectionDeleted = 0xEA0D163E;
constexpr uint32_t kMsgHistoryRecord = 0x89EFA536;

// The tuning exemplar "Neighbor Deals" (§12).
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0x7A4A8458;
constexpr uint32_t kTuningInstance = 0x6A469021;

// Markers (§6.3): the prop, its occupant type (crime.md §6.10), the marker
// property.
constexpr uint32_t kMarkerProp = 0x29F10000;
constexpr uint32_t kOccupantTypeProp = 0xA823821E;
constexpr uint32_t kMarkerFlag = 0xCAA46595;
// NG25: the prop interface and the usage of a city prop (AddCityProp).
constexpr uint32_t kIID_cISC4PropOccupant = 0xE9793A65;
constexpr uint8_t kPropUsageCity = 1;

// §8.2: the departments and the waste-to-energy line item.
constexpr uint32_t kDeptPower = 0x8910BC8A;
constexpr uint32_t kDeptWater = 0xAA4014B4;
constexpr uint32_t kDeptSanitation = 0x6A42A8F5;
constexpr uint32_t kLineWasteToEnergy = 0x1F470000;

// traffic-simulator.md §4.1.4: the trip-count arrays 0xAA9EA81D + w.
constexpr uint32_t kTripTargets = 0xAA9EA81D;

// How long a thread other than the main one waits for the lock. NG6 (glue,
// neighbors.md §15.4): the original has no lock and runs every method on the
// caller's thread; the try-lock and its fallback are this glue's choice.
constexpr uint32_t kLockWaitMs = 20;

const char* const kTypeNames[13] = {"none",   "rail",   "road",  "highway",  "powerpole", "pipe",     "subway",
                                    "lightrail", "avenue", "ferry", "dirtroad", "monorail",  "groundhighway"};

const char* TypeName(uint32_t t) { return t < 13 ? kTypeNames[t] : "?"; }

// ---- the glue's lock ------------------------------------------------------------------------------
//
// One lock for every call into a neighbours core (simulator, connection or
// deal): the cores share Rc'd objects, so they are one single-threaded world.

struct GlueLock {
    CRITICAL_SECTION cs;
    GlueLock() { InitializeCriticalSection(&cs); }
    ~GlueLock() { DeleteCriticalSection(&cs); }
};

GlueLock& TheLock() {
    static GlueLock l;
    return l;
}

std::atomic<DWORD> g_mainTid{0};
std::atomic<uint32_t> g_fallbacks{0};
std::atomic<int32_t> g_countX{0}, g_countZ{0};
// Under the lock:
int g_depth = 0;
bool g_dirty = false;
cFC4NeighborsSimulator* g_current = nullptr;  // the initialised simulator deals link to (§5.4)
std::vector<std::shared_ptr<void>> g_held;     // game lists kept to the outermost exit
std::vector<cIGZUnknown*> g_heldRefs;          // regional cities AddRef'd to the outermost exit

// The registries and the park lists (any thread).
std::mutex g_objLock;
std::vector<cFC4NeighborConnection*> g_conns;
std::vector<cFC4NeighborDeal*> g_deals;
std::vector<cFC4NeighborsSimulator*> g_sims;
std::vector<cFC4NeighborConnection*> g_parkedConns;
std::vector<cFC4NeighborDeal*> g_parkedDeals;

bool IsMainThread() {
    const DWORD m = g_mainTid.load();
    return m == 0 || GetCurrentThreadId() == m;
}

bool EnterLock() {
    if (IsMainThread()) {
        EnterCriticalSection(&TheLock().cs);
        return true;
    }
    if (TryEnterCriticalSection(&TheLock().cs)) return true;
    const ULONGLONG until = GetTickCount64() + kLockWaitMs;
    while (GetTickCount64() < until) {
        Sleep(1);
        if (TryEnterCriticalSection(&TheLock().cs)) return true;
    }
    ++g_fallbacks;
    return false;
}

void OutermostExit();

// Holds the lock for one call into a core. ok() false (another thread that
// could not get it): answer from a mirror or with the neutral value.
class Section {
public:
    explicit Section(bool writes = true) : ok_(EnterLock()) {
        if (!ok_) return;
        ++g_depth;
        if (writes) g_dirty = true;
    }
    ~Section() {
        if (!ok_) return;
        if (g_depth == 1) OutermostExit();
        --g_depth;
        LeaveCriticalSection(&TheLock().cs);
    }
    Section(const Section&) = delete;
    Section& operator=(const Section&) = delete;
    bool ok() const { return ok_; }

private:
    bool ok_;
};

void Park(cFC4NeighborConnection* c) {
    std::lock_guard<std::mutex> g(g_objLock);
    g_parkedConns.push_back(c);
}

// NG5 (§15.4, the bracket stands): an original deal dies at its last Release
// and is never reused. Making the wrapper lazily and keeping it parked at zero
// references cannot be seen: no correct caller reaches an object it holds no
// reference to, and the same deal keeps the same pointer while any exists.
void Park(cFC4NeighborDeal* d) {
    std::lock_guard<std::mutex> g(g_objLock);
    g_parkedDeals.push_back(d);
}

template <class T>
void Unregister(std::vector<T*>& v, T* p) {
    std::lock_guard<std::mutex> g(g_objLock);
    v.erase(std::remove(v.begin(), v.end(), p), v.end());
}

template <class T>
void Register(std::vector<T*>& v, T* p) {
    std::lock_guard<std::mutex> g(g_objLock);
    v.push_back(p);
}

// The parked objects whose count is still 0 are deleted (main thread, the
// lock held, depth 1). Deleting may park more (a connection's holder releases
// its values): repeat until none.
template <class T>
bool SweepOnce(std::vector<T*>& parked) {
    std::vector<T*> take;
    {
        std::lock_guard<std::mutex> g(g_objLock);
        take.swap(parked);
    }
    if (take.empty()) return false;
    std::sort(take.begin(), take.end());
    take.erase(std::unique(take.begin(), take.end()), take.end());
    for (T* p : take) {
        if (p->Refs() == 0) T::Destroy(p);
    }
    return true;
}

void Sweep() {
    for (int i = 0; i < 8; ++i) {
        const bool a = SweepOnce(g_parkedDeals);
        const bool b = SweepOnce(g_parkedConns);
        if (!a && !b) break;
    }
}

void ReleaseHeld() {
    std::vector<std::shared_ptr<void>> lists;
    lists.swap(g_held);
    lists.clear();  // the game lists (and the references they hold)
    std::vector<cIGZUnknown*> refs;
    refs.swap(g_heldRefs);
    for (cIGZUnknown* r : refs) FC4_RELEASE(r);
}

void RefreshMirrors() {
    std::vector<cFC4NeighborConnection*> conns;
    std::vector<cFC4NeighborDeal*> deals;
    std::vector<cFC4NeighborsSimulator*> sims;
    {
        std::lock_guard<std::mutex> g(g_objLock);
        conns = g_conns;
        deals = g_deals;
        sims = g_sims;
    }
    // Deletion happens only under the lock (held here): all of these live.
    for (cFC4NeighborConnection* c : conns) c->RefreshMirror();
    for (cFC4NeighborDeal* d : deals) d->RefreshMirror();
    for (cFC4NeighborsSimulator* s : sims) s->RefreshSnapshot();
}

// The end of the outermost call that held the lock (depth still 1). Game
// objects are released and parked objects deleted on the main thread only;
// the mirrors are refreshed on any thread.
void OutermostExit() {
    if (IsMainThread()) {
        ReleaseHeld();
        Sweep();
    }
    if (g_dirty) {
        g_dirty = false;
        RefreshMirrors();
    }
}

// Under the lock: kept until the outermost exit. NG24 (glue, §15.4): the
// original holds regional cities only inside one call; how long the glue
// holds its own references is its choice.
void Hold(cIGZUnknown* u) {
    if (u == nullptr) return;
    if (std::find(g_heldRefs.begin(), g_heldRefs.end(), u) != g_heldRefs.end()) return;
    FC4_ADDREF(u);
    g_heldRefs.push_back(u);
}

bool g_noteDropped = false;
bool g_noteDealNotListed = false;  // NG43
void NoteDropped(const char* what) {
    if (g_noteDropped) return;
    g_noteDropped = true;
    fc4::Log("neighbours: a call from another thread could not take the lock within %u ms and was dropped (first: "
             "%s; later ones are counted as fallbacks)",
             kLockWaitMs, what);
}

// ---- small helpers --------------------------------------------------------------------------------

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

cISC4NeighborConnection* ConnOf(uint64_t t) { return reinterpret_cast<cISC4NeighborConnection*>(FromToken(t)); }
cISC4NeighborDeal* DealOf(uint64_t t) { return reinterpret_cast<cISC4NeighborDeal*>(FromToken(t)); }
cISC4RegionalCity* CityOf(uint64_t t) { return reinterpret_cast<cISC4RegionalCity*>(FromToken(t)); }
cFC4ISC4RegionalCity* RcOf(uint64_t t) { return t ? fc4::AsGame(CityOf(t)) : nullptr; }
cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }

// Our deal / connection behind an interface pointer (borrowed), or nullptr.
cFC4NeighborDeal* OurDeal(const cISC4NeighborDeal* d) {
    if (d == nullptr) return nullptr;
    void* p = nullptr;
    auto* u = const_cast<cISC4NeighborDeal*>(d);
    if (!u->QueryInterface(kIID_FC4NeighborDeal, &p) || p == nullptr) return nullptr;
    auto* ours = static_cast<cFC4NeighborDeal*>(p);
    ours->Release();  // the caller's reference keeps it alive
    return ours;
}

cFC4NeighborConnection* OurConnection(const cISC4NeighborConnection* c) {
    if (c == nullptr) return nullptr;
    void* p = nullptr;
    auto* u = const_cast<cISC4NeighborConnection*>(c);
    if (!u->QueryInterface(kIID_FC4NeighborConnection, &p) || p == nullptr) return nullptr;
    auto* ours = static_cast<cFC4NeighborConnection*>(p);
    ours->Release();
    return ours;
}

void PushToken(void* ctx, uint64_t t) { static_cast<std::vector<uint64_t>*>(ctx)->push_back(t); }

// The city's cell counts (cached for other threads).
void CityCounts(int32_t* x, int32_t* z) {
    if (cISC4City* c = fc4::NeighborAppCity()) {
        g_countX = static_cast<int32_t>(c->CellCountX());  // slot 93
        g_countZ = static_cast<int32_t>(c->CellCountZ());  // slot 94
    }
    if (x != nullptr) *x = g_countX.load();
    if (z != nullptr) *z = g_countZ.load();
}

bool Gate(fc4::Sw s, const char* what) {
    if (fc4::On(s)) return true;
    fc4::NoteSkipped(s, what);
    return false;
}

void FormatDate(uint32_t julian, char* buf, size_t n) {
    fc4_date d{};
    fc4_date_from_julian(static_cast<int32_t>(julian), &d);
    snprintf(buf, n, "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
}

// The session seed (foundations §6.3): FreeCity4.ini `neighbors_seed` (env
// FC4_NEIGHBORS_SEED) when set, else the wall clock taken as the cRZTime
// value for "now" (time() + 2177452800), as the residential glue does. The
// proposal generator (§7.1) is the core's, seeded from it.
uint32_t SessionSeed(bool* fixed) {
    const std::string s = fc4::Setting("neighbors_seed", "FC4_NEIGHBORS_SEED");
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

// A bool property that reads true (a non-zero first element).
bool PropTrue(cISCPropertyHolder* h, uint32_t id) {
    if (h == nullptr) return false;
    cISCProperty* p = fc4::AsGame(h)->GetProperty(id);  // slot 9, borrowed
    if (p == nullptr) return false;
    const cIGZVariant* v = static_cast<const cISCProperty*>(p)->GetPropertyValue();
    return v != nullptr && fc4::VariantTrue(*fc4::AsGame(v));
}

// ---- host callbacks -------------------------------------------------------------------------------

fc4::NeighborsContext* Ctx(void* u) { return static_cast<fc4::NeighborsContext*>(u); }

cISC4City* City(void* u) {
    cISC4City* c = Ctx(u)->city;
    return c ? c : fc4::NeighborAppCity();
}

fc4_props* Tuning(void* u) {
    if (!Gate(fc4::Sw::ExemplarLoads, "neighbours tuning exemplar (code defaults)")) return nullptr;
    auto& t = Ctx(u)->tuning;
    if (!t) t = std::make_unique<fc4::props::Exemplar>();
    fc4_props* p = fc4::LoadNeighborTuning(t.get());
    if (p == nullptr) {
        fc4::Log("neighbours: tuning exemplar %08X-%08X-%08X unavailable; the code defaults stay (§12)", kTuningType,
                 kTuningGroup, kTuningInstance);
    }
    return p;
}

// §14.1 step 2: simulator slot 19 AddAgent(target, level, name, flags) on the
// city's simulator (held until RemoveAgent).
void AddAgent(void* u, uint32_t level, const char* name, size_t len, uint32_t flags) {
    fc4::NeighborsContext* c = Ctx(u);
    if (c->target == nullptr || name == nullptr) return;
    cISC4City* city = City(u);
    cISC4Simulator* raw = city ? city->GetSimulator() : nullptr;  // city slot 50, borrowed
    if (raw == nullptr) {
        fc4::Log("neighbours: no simulator; agent level %u not registered", level);
        return;
    }
    if (c->agentSim == nullptr) {
        c->agentSim = raw;
        FC4_ADDREF(c->agentSim);
    }
    const cRZBaseString n(name, static_cast<uint32_t>(len));
    fc4::AsGame(c->agentSim)->AddAgent(c->target, level, n, flags);
    c->agentsRegistered = true;
}

// §14.2: RemoveAgent(this) (every level).
void RemoveAgents(void* u) {
    fc4::NeighborsContext* c = Ctx(u);
    if (c->agentSim == nullptr) return;
    if (c->agentsRegistered) fc4::AsGame(c->agentSim)->RemoveAgentAllLevels(c->target);
    c->agentsRegistered = false;
    FC4_RELEASE(c->agentSim);
    c->agentSim = nullptr;
}

void Subscribe(void* u, uint32_t type) {
    fc4::NeighborsContext* c = Ctx(u);
    if (c->msgServer != nullptr && c->target != nullptr) c->msgServer->AddNotification(c->target, type);
}

void Unsubscribe(void* u, uint32_t type) {
    fc4::NeighborsContext* c = Ctx(u);
    if (c->msgServer != nullptr && c->target != nullptr) c->msgServer->RemoveNotification(c->target, type);
}

void CellCounts(void*, int32_t* x, int32_t* z) { CityCounts(x, z); }

uint32_t Today(void* u) { return fc4::NeighborToday(City(u)); }

void UtilityFigures(void* u, fc4_neighbors_figures* out) {
    if (out == nullptr) return;
    *out = fc4_neighbors_figures{};
    if (!Gate(fc4::Sw::NeighborsFigures, "neighbours utility figures (0, no departments)")) return;
    fc4::ReadNeighborFigures(City(u), out);
}

void LandfillUsage(void* u, uint32_t* usage, uint32_t* capacity) {
    if (usage != nullptr) *usage = 0;
    if (capacity != nullptr) *capacity = 0;
    if (!Gate(fc4::Sw::NeighborsFigures, "neighbours landfill usage (0)")) return;
    fc4::ReadNeighborLandfill(City(u), usage, capacity);
}

cIGZMessageServer2* Server(void* u, cIGZMessageServer2Ptr& fallback) {
    cIGZMessageServer2* ms = Ctx(u)->msgServer;
    return ms ? ms : static_cast<cIGZMessageServer2*>(fallback);
}

// §8.2: MessageSend of a history record 0x89EFA536 (data1 series, data2 value).
void SendHistory(void* u, uint32_t series, int32_t value) {
    if (!Gate(fc4::Sw::NeighborsHistory, "neighbours history record")) return;
    cIGZMessageServer2Ptr fallback;
    cIGZMessageServer2* ms = Server(u, fallback);
    if (ms == nullptr) return;
    cFC4Message2Standard* msg =
        FC4MakeMessage(kMsgHistoryRecord, static_cast<intptr_t>(series), static_cast<intptr_t>(value), 0);
    ms->MessageSend(msg);
    FC4_X87_CHECK("neighbours history MessageSend (after)");
    msg->Release();
    ++Ctx(u)->history;
}

// ---- region ----

uint64_t CurrentCity(void*) {
    if (!Gate(fc4::Sw::NeighborsRegionReads, "neighbours current regional city (none)")) return 0;
    cISC4RegionalCity* r = fc4::NeighborAppRegionalCity();
    Hold(r);
    return TokenOf(r);
}

// NB18 (answered, neighbors §8.2; the bracket matches): UpdateCommodityStats
// compares addresses against the game's city global, which holds the city
// object's own cISC4City interface from construction to city shutdown. This
// is the pointer cISC4App::GetCity returns (held from Init, else borrowed),
// with no QueryInterface and no wrapper, either of which would make the call
// do nothing. GetCity returns that same primary cISC4City address (NB19,
// answered, neighbors §8.2: the application's city member, which NewCity
// set from the same constructor as the global; the bracket matches).
uint64_t CurrentCityObject(void* u) { return TokenOf(City(u)); }

uint64_t CityAt(void*, int32_t rx, int32_t rz) {
    if (!Gate(fc4::Sw::NeighborsRegionReads, "neighbours region GetCity (none)")) return 0;
    cISC4RegionalCity* r = fc4::NeighborRegionCityAt(rx, rz);
    Hold(r);
    return TokenOf(r);
}

void CityPosition(void*, uint64_t city, int32_t* x, int32_t* z) {
    int32_t a = 0, b = 0;
    if (cFC4ISC4RegionalCity* r = RcOf(city)) r->GetPosition(a, b);  // slot 5
    if (x != nullptr) *x = a;
    if (z != nullptr) *z = b;
}

void CitySize(void*, uint64_t city, int32_t* x, int32_t* z) {
    int32_t a = 0, b = 0;
    if (cFC4ISC4RegionalCity* r = RcOf(city)) r->GetCitySize(a, b);  // slot 7
    if (x != nullptr) *x = a;
    if (z != nullptr) *z = b;
}

bool CityEstablished(void*, uint64_t city) {
    cFC4ISC4RegionalCity* r = RcOf(city);
    return r != nullptr && r->GetEstablished();  // slot 43
}

// Slots 54-63 (double, regional-city table of neighbors.md). NG12 (§15.4,
// matches): the getters 50-64 return a double on the x87 stack.
double CityStat(void*, uint64_t city, uint32_t stat, int32_t c) {
    cFC4ISC4RegionalCity* r = RcOf(city);
    if (r == nullptr) return 0.0;
    switch (stat) {
        case 0: return r->GetExported(c);
        case 1: return r->GetImported(c);
        case 2: return r->GetProduced(c);
        case 3: return r->GetDemanded(c);
        case 4: return r->GetCostPerUnit(c);
        default: return 0.0;
    }
}

void SetCityStat(void*, uint64_t city, uint32_t stat, int32_t c, double v) {
    cFC4ISC4RegionalCity* r = RcOf(city);
    if (r == nullptr || !Gate(fc4::Sw::NeighborsRegionWrites, "neighbours regional-city statistic write")) return;
    switch (stat) {
        case 0: r->SetExported(c, v); break;
        case 1: r->SetImported(c, v); break;
        case 2: r->SetProduced(c, v); break;
        case 3: r->SetDemanded(c, v); break;
        case 4: r->SetCostPerUnit(c, v); break;
        default: break;
    }
}

double CityBudget(void*, uint64_t city) {
    cFC4ISC4RegionalCity* r = RcOf(city);
    return r ? r->GetBudget() : 0.0;  // slot 50
}

void SetCityBudget(void*, uint64_t city, double v) {
    cFC4ISC4RegionalCity* r = RcOf(city);
    if (r != nullptr && Gate(fc4::Sw::NeighborsRegionWrites, "neighbours SetBudget")) r->SetBudget(v);  // slot 51
}

void SetCityIncome(void*, uint64_t city, double v) {
    cFC4ISC4RegionalCity* r = RcOf(city);
    if (r != nullptr && Gate(fc4::Sw::NeighborsRegionWrites, "neighbours SetIncome")) r->SetIncome(v);  // slot 53
}

// Slot 20 FindConnection(type, x, z): the regional city's connection (an
// original object, typed cISC4RegionalCity* in the SDK), borrowed.
uint64_t CityFindConnection(void*, uint64_t city, uint32_t t, int32_t x, int32_t z, bool* active) {
    if (active != nullptr) *active = false;
    cFC4ISC4RegionalCity* r = RcOf(city);
    if (r == nullptr || !Gate(fc4::Sw::NeighborsRegionReads, "neighbours regional FindConnection (none)")) return 0;
    auto* c = reinterpret_cast<cISC4NeighborConnection*>(r->FindConnection(static_cast<int32_t>(t), x, z));
    if (c == nullptr) return 0;
    if (active != nullptr) *active = c->GetIsActive();
    return TokenOf(c);
}

// Slot 21 GetAllConnections: clears first, one reference each (A,
// allocators.md); released when the list goes.
void CityConnections(void*, uint64_t city, fc4_neighbor_link_callback cb, void* ctx) {
    cFC4ISC4RegionalCity* r = RcOf(city);
    if (r == nullptr || cb == nullptr) return;
    if (!Gate(fc4::Sw::NeighborsRegionReads, "neighbours regional GetAllConnections (none)")) return;
    SC4List<cISC4NeighborConnection*> list;
    r->GetAllConnections(list);
    for (cISC4NeighborConnection* c : list) {
        if (c == nullptr) continue;
        int32_t rx = 0, rz = 0;
        c->GetRegionCellOnOtherSideOfConnection(rx, rz);
        cb(ctx, static_cast<uint32_t>(c->GetType()), rx, rz);
    }
}

void ChangeSymmetricConnection(void*, uint64_t city, uint64_t conn, bool add) {
    cFC4ISC4RegionalCity* r = RcOf(city);
    if (r == nullptr || conn == 0) return;
    if (!Gate(fc4::Sw::NeighborsSymmetric, "neighbours ChangeSymmetricConnection")) return;
    r->ChangeSymmetricConnection(ConnOf(conn), add);  // slot 22
}

// §14.1 step 5: region.ReloadCity(city) on the region's own entry for it
// (cISC4Region, SDK order: GetCity slot 11, ReloadCity slot 15). NG30
// (§15.4): the SDK order matches the game's. The original has no gate and no
// identity check; the identity check here always passes (the list was just
// built from the region's cities), and the neighbors_reload switch is a
// diagnostic that is on by default (off deviates). As the original, the
// holder is not used after the call (a failed reload frees it) and the
// result is not read: the city is looked up again.
void ReloadCity(void* u, uint64_t city) {
    cFC4ISC4RegionalCity* r = RcOf(city);
    if (r == nullptr || !Gate(fc4::Sw::NeighborsReload, "neighbours ReloadCity")) return;
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = Query<cISC4App>(app, GZIID_cISC4App);
    cISC4Region* region = sc4 ? sc4->GetRegion() : nullptr;
    Drop(sc4);
    if (region == nullptr) return;
    int32_t px = 0, pz = 0;
    r->GetPosition(px, pz);
    cRZAutoRefCount<cISC4RegionalCity>* entry = region->GetCity(static_cast<uint32_t>(px), static_cast<uint32_t>(pz));
    if (entry == nullptr || static_cast<cISC4RegionalCity*>(*entry) != CityOf(city)) return;
    cRZAutoRefCount<cISC4RegionalCity>* p = entry;
    region->ReloadCity(p);
    ++Ctx(u)->reloads;
    cRZAutoRefCount<cISC4RegionalCity>* after =
        region->GetCity(static_cast<uint32_t>(px), static_cast<uint32_t>(pz));
    cISC4RegionalCity* now = after ? static_cast<cISC4RegionalCity*>(*after) : nullptr;
    if (now == CityOf(city)) return;
    // §14.1 step 5 [compat]: the region replaced its object and dropped its
    // reference to the old one; ours (Hold) is released now, as the original
    // releases its list entry right after the reload. That frees the old
    // object: the expected last release. The core asks for the cities again
    // before steps 6-7 and does not use this token any more.
    cIGZUnknown* old = FromToken(city);
    const bool held = std::find(g_heldRefs.begin(), g_heldRefs.end(), old) != g_heldRefs.end();
    fc4::Log("neighbours: ReloadCity replaced the regional city at (%d, %d) (%p -> %p); the old object %s",
             px, pz, static_cast<void*>(old), static_cast<void*>(now),
             held ? "released (the region's reference went with the reload)" : "was not held");
    if (held) {
        g_heldRefs.erase(std::find(g_heldRefs.begin(), g_heldRefs.end(), old));
        FC4_RELEASE_CREATED(old);
    }
}

void UpdateLocalDeals(void*, uint64_t city) {
    cFC4ISC4RegionalCity* r = RcOf(city);
    if (r != nullptr && Gate(fc4::Sw::NeighborsDeals, "neighbours UpdateLocalDeals")) r->UpdateLocalDeals();  // 68
}

// Slot 70 GetLocalDeals: list assignment into an empty list (one reference
// each, allocators.md); the deals are read through their interface.
void LocalDeals(void*, uint64_t city, fc4_neighbor_deal_data_callback cb, void* ctx) {
    cFC4ISC4RegionalCity* r = RcOf(city);
    if (r == nullptr || cb == nullptr || !Gate(fc4::Sw::NeighborsDeals, "neighbours GetLocalDeals (none)")) return;
    SC4List<cISC4NeighborDeal*> list;
    r->GetLocalDeals(list);
    for (cISC4NeighborDeal* d : list) {
        fc4_neighbor_deal_data data{};
        if (fc4::ReadNeighborDeal(d, &data)) cb(ctx, &data);
    }
}

// Slot 69 SetLocalDeals: the region copies the elements (one reference each)
// and may erase nodes from the list it is given (allocators.md).
void SetLocalDeals(void* u, uint64_t city, const uint64_t* deals, size_t count, uint8_t* kept) {
    cFC4ISC4RegionalCity* r = RcOf(city);
    if (r == nullptr || !Gate(fc4::Sw::NeighborsDeals, "neighbours SetLocalDeals")) return;
    SC4List<cISC4NeighborDeal*> list;
    for (size_t i = 0; i < count; ++i) {
        cISC4NeighborDeal* d = DealOf(deals[i]);
        if (d == nullptr) continue;
        d->AddRef();  // the list's reference (ours: not audited)
        list.push_back(d);
    }
    r->SetLocalDeals(list);
    ++Ctx(u)->setLocalDeals;
    if (kept == nullptr) return;
    for (size_t i = 0; i < count; ++i) {
        cISC4NeighborDeal* d = DealOf(deals[i]);
        if (d == nullptr) continue;
        bool found = false;
        for (cISC4NeighborDeal* e : list) {
            if (e == d) {
                found = true;
                break;
            }
        }
        if (!found) {
            kept[i] = 0;
            ++Ctx(u)->regionRemoved;
        }
    }
}

void UpdateImportExport(void*, uint64_t city) {
    cFC4ISC4RegionalCity* r = RcOf(city);
    if (r != nullptr && Gate(fc4::Sw::NeighborsRegionWrites, "neighbours UpdateImportExport")) {
        r->UpdateImportExport();  // slot 71
    }
}

// ---- messages ----

// NG31 (§15.4, matches): statuses 0-7 carry the deal's object (the core makes
// it when the report is posted, and the message's reference keeps it alive
// until delivery, NG5); only status 8 has none.
void Status(void* u, uint64_t deal, uint32_t status, bool send) {
    if (!Gate(fc4::Sw::NeighborsPosts, "neighbours deal status report")) return;
    cIGZMessageServer2Ptr fallback;
    cIGZMessageServer2* ms = Server(u, fallback);
    if (ms == nullptr) return;
    cFC4Message2Standard* msg = FC4MakeMessage(kMsgDealStatus, static_cast<intptr_t>(status), 0, 0);
    if (deal != 0) msg->SetIGZUnknown(FromToken(deal));
    if (send) {
        ms->MessageSend(msg);
        FC4_X87_CHECK("neighbours status MessageSend (after)");
        ++Ctx(u)->sends;
    } else {
        ms->MessagePost(msg, false);
        ++Ctx(u)->posts;
    }
    msg->Release();
}

void PostStatus(void* u, uint64_t deal, uint32_t status) { Status(u, deal, status, false); }
void SendStatus(void* u, uint64_t deal, uint32_t status) { Status(u, deal, status, true); }

// §6.3 / §6.4: one log line per connection created or deleted, then the post.
void PostConnectionMessage(void* u, uint32_t type, uint32_t connType, int32_t x, int32_t z) {
    fc4::NeighborsContext* c = Ctx(u);
    const bool created = type == kMsgConnectionCreated;
    if (created) {
        ++c->created;
    } else if (type == kMsgConnectionDeleted) {
        ++c->deleted;
    }
    fc4::Log("neighbours connection %s: %s (type %u) at (%d, %d)", created ? "created" : "deleted", TypeName(connType),
             connType, x, z);
    if (!Gate(fc4::Sw::NeighborsPosts, "neighbours connection message")) return;
    cIGZMessageServer2Ptr fallback;
    cIGZMessageServer2* ms = Server(u, fallback);
    if (ms == nullptr) return;
    cFC4Message2Standard* msg =
        FC4MakeMessage(type, static_cast<intptr_t>(connType), static_cast<intptr_t>(x), static_cast<intptr_t>(z));
    ms->MessagePost(msg, false);
    msg->Release();
    ++c->connectionMessages;
}

// ---- markers ----

// City slot 97 (returns nothing; the outputs are the references).
void CellCenter(void* u, int32_t x, int32_t z, float* px, float* pz) {
    float a = 0.0f, b = 0.0f;
    if (cISC4City* c = City(u)) c->CellCenterToPosition(x, z, a, b);
    if (px != nullptr) *px = a;
    if (pz != nullptr) *pz = b;
}

cISTETerrain* Terrain(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(fc4::Sw::NeighborsMarkers, what)) return nullptr;
    return c->GetTerrain();
}

float MarkerAltitude(void* u, float px, float pz) {
    cISTETerrain* t = Terrain(u, "neighbours terrain GetAltitude (0)");
    return t ? t->GetAltitude(px, pz) : 0.0f;  // compiled slot 13, ST0
}

float MaxSurfaceLevel(void* u, int32_t x, int32_t z) {
    cISTETerrain* t = Terrain(u, "neighbours terrain GetMaxSurfaceLevel (0)");
    return t ? t->GetMaxSurfaceLevel(static_cast<uint32_t>(x), static_cast<uint32_t>(z)) : 0.0f;  // slot 18, ST0
}

cISC4PropManager* Props(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(fc4::Sw::NeighborsMarkers, what)) return nullptr;
    return c->GetPropManager();  // city slot 35
}

// §6.3, NG16 (§15.4, matches): CreateProp(0x29F10000) (slot 15; the prop
// with one reference for us; on failure no marker, and the created message is
// still posted), orientation, position and the bool properties through holder
// slot 13 (AddProperty(id, 1, false), as the original), then
// AddCityProp(occupant) (slot 19); our reference is released (the prop
// manager keeps its own).
uint64_t AddMarkerProp(void* u, const float* pos, int32_t orientation, const uint32_t* bools, size_t count) {
    cISC4PropManager* pm = Props(u, "neighbours marker prop (none)");
    if (pm == nullptr || pos == nullptr) return 0;
    cISC4PropOccupant* prop = nullptr;
    if (!pm->CreateProp(kMarkerProp, prop) || prop == nullptr) {
        fc4::Log("neighbours: CreateProp(0x%08X) failed; no marker", kMarkerProp);
        return 0;
    }
    FC4_ACQUIRED(prop, "CreateProp");
    prop->SetOrientation(orientation);
    cISC4Occupant* occ = prop->AsOccupant();
    uint64_t token = 0;
    if (occ != nullptr) {
        const cS3DVector3 v(pos[0], pos[1], pos[2]);
        occ->SetPosition(&v);
        cISCPropertyHolder* h = occ->AsPropertyHolder();
        for (size_t i = 0; h != nullptr && bools != nullptr && i < count; ++i) {
            (void)fc4::AsGame(h)->AddPropertyU32(bools[i], 1, false);  // holder slot 13 (NG16)
        }
        if (pm->AddCityProp(occ)) {
            token = TokenOf(occ);
            ++Ctx(u)->markersAdded;
        } else {
            fc4::Log("neighbours: AddCityProp refused the marker prop");
        }
    }
    FC4_RELEASE_CREATED(prop);
    return token;
}

cISC4OccupantManager* Occupants(void* u, const char* what) {
    cISC4City* c = City(u);
    if (c == nullptr || !Gate(fc4::Sw::NeighborsMarkers, what)) return nullptr;
    return c->GetOccupantManager();  // city slot 34
}

// A city prop (NG25): its cISC4PropOccupant's GetPropUsage (slot 22) is 1, the
// usage AddCityProp sets.
bool IsCityProp(cISC4Occupant* o) {
    auto* p = Query<cISC4PropOccupant>(o, kIID_cISC4PropOccupant);
    if (p == nullptr) return false;
    const bool city = p->GetPropUsage() == kPropUsageCity;  // slot 22
    Drop(p);
    return city;
}

// NG25 (§6.4, §11.1, §15.4): the city props of one cell with 0xCAA46595 true.
// The original asks the prop manager's GetCityProps (slot 18), whose filter
// passes type 0xA823821E (prop) with prop usage 1 (city prop). The spec does
// not give that call's rectangle (int or float, inclusive or not), so this
// keeps the occupant manager's cell query for the type
// (GetOccupantsByStandardCityCells, slot 41; one reference each, kept to the
// outermost exit) and adds the usage test, the other half of the answer's
// [choice].
void MarkerProps(void* u, int32_t x, int32_t z, fc4_token_callback cb, void* ctx) {
    cISC4OccupantManager* om = Occupants(u, "neighbours marker lookup (none)");
    if (om == nullptr || cb == nullptr) return;
    auto list = std::make_shared<SC4List<cISC4Occupant*>>();
    const SC4Point<int> cell(x, z);
    om->GetOccupantsByStandardCityCells(*list, &cell, &cell, kOccupantTypeProp, 0xFFFFFFFFu);
    g_held.push_back(list);
    for (cISC4Occupant* o : *list) {
        if (o != nullptr && IsCityProp(o) && PropTrue(o->AsPropertyHolder(), kMarkerFlag)) cb(ctx, TokenOf(o));
    }
}

void RemoveProp(void* u, uint64_t prop) {
    cISC4PropManager* pm = Props(u, "neighbours RemovePropA");
    if (pm == nullptr || prop == 0) return;
    pm->RemovePropA(OccOf(prop));  // slot 29
    ++Ctx(u)->markersRemoved;
}

// §11.1: SetPosition (occupant slot 9), then MoveOccupant(occ, true) (slot 24).
void MoveProp(void* u, uint64_t prop, const float* pos) {
    cISC4OccupantManager* om = Occupants(u, "neighbours marker move");
    if (om == nullptr || prop == 0 || pos == nullptr) return;
    const cS3DVector3 v(pos[0], pos[1], pos[2]);
    OccOf(prop)->SetPosition(&v);
    om->MoveOccupant(OccOf(prop), true);
    ++Ctx(u)->markersMoved;
}

// ---- occupants ----

uint32_t OccupantType(void*, uint64_t occ) { return occ ? OccOf(occ)->GetType() : 0; }

// §6.5: GetNetworkFlag (slot 12) is a mask whose bit n is network type n.
// NG29 (§15.4, matches): the §6.5 numbers are network type numbers and the
// original tests flags & (1 << type); reading the flag word, the glue shifts
// and hands the core the type numbers.
void OccupantNetworks(void*, uint64_t occ, fc4_token_callback cb, void* ctx) {
    if (occ == 0 || cb == nullptr) return;
    cISC4NetworkOccupant* n = Query<cISC4NetworkOccupant>(FromToken(occ), kIID_cISC4NetworkOccupant);
    if (n == nullptr) return;
    const uint32_t mask = n->GetNetworkFlag();
    Drop(n);
    for (uint32_t b = 0; b < 32; ++b) {
        if ((mask >> b) & 1u) cb(ctx, b);
    }
}

// NG17 (§6.5, §15.4): for networks and poles alike, the occupant's position
// (cISC4Occupant::GetPosition, slot 8) converted by the city's
// PositionToCell (slot 95): 16 m cells from the city's (0, 0) corner, rounded
// down. The spec names no check on GetPosition's result [choice, spec
// question]: it is not tested, and a failed call leaves the zero vector, so
// cell (0, 0).
void OccupantCell(void* u, uint64_t occ, int32_t* x, int32_t* z) {
    int a = 0, b = 0;
    cISC4City* c = City(u);
    if (occ != 0 && c != nullptr) {
        cS3DVector3 v;  // (0, 0, 0)
        (void)OccOf(occ)->GetPosition(&v);  // slot 8
        c->PositionToCell(v.fX, v.fZ, a, b);  // slot 95
    }
    if (x != nullptr) *x = a;
    if (z != nullptr) *z = b;
}

// §11.1 RemoveOccupant: the pole interface's slot 13.
bool PowerPoleFlag(void*, uint64_t occ) {
    auto* p = Query<cFC4ISC4PowerPoleOccupant>(FromToken(occ), kIID_cISC4PowerPoleOccupant);
    if (p == nullptr) return false;
    const bool f = p->IsIgnoredByPower();
    Drop(p);
    return f;
}

// ---- objects (§15.2) ----

uint64_t ConnectionObject(void*, fc4_neighbor_connection* handle) {
    if (handle == nullptr) return 0;
    auto* c = new cFC4NeighborConnection(handle);  // count 1: the list's
    return TokenOf(c->Primary());
}

uint64_t DealObject(void*, fc4_neighbor_deal* handle) {
    if (handle == nullptr) return 0;
    auto* d = new cFC4NeighborDeal(handle);  // count 0: nobody holds it yet
    Park(d);
    return TokenOf(d->Primary());
}

bool ForeignConnection(void*, uint64_t conn, fc4_neighbor_connection_state* out) {
    return fc4::ReadNeighborConnection(ConnOf(conn), out);
}

// NG9 (§15.4, matches): type and cell are never written after a connection
// is made; on a foreign one Repair and the visited / active flags go through
// its interface.
void SetForeignConnectionFlags(void*, uint64_t conn, int32_t visited, int32_t active) {
    cISC4NeighborConnection* c = ConnOf(conn);
    if (c == nullptr) return;
    if (visited >= 0) c->SetVisitedThisCycle(visited != 0);
    if (active >= 0) c->SetIsActive(active != 0);
}

void RepairForeignConnection(void*, uint64_t conn, uint64_t city) {
    cISC4NeighborConnection* c = ConnOf(conn);
    if (c != nullptr) c->Repair(CityOf(city));
}

// The list's reference: ours, or an original read from a save (ours to
// free, as the land value grids: the save handed it over).
void ReleaseConnection(void*, uint64_t conn) {
    cIGZUnknown* o = FromToken(conn);
    if (o == nullptr) return;
    if (fc4::IsOurObject(o)) {
        o->Release();
    } else {
        FC4_RELEASE_CREATED(o);
    }
}

// ---- foreign deals (§15.4 NG4) --------------------------------------------------------------------
// A caller's deal that is not ours, kept by AddDeal as the listed deal: the
// core reads it and drives it through its cISC4NeighborDeal, so its own code
// runs (and may call back into the simulator: the lock is recursive).

bool ForeignDeal(void*, uint64_t deal, fc4_neighbor_deal_data* out) {
    return fc4::ReadNeighborDeal(DealOf(deal), out);
}

bool ForeignDealCall(void*, uint64_t deal, uint32_t op, int64_t a, int64_t b) {
    cISC4NeighborDeal* d = DealOf(deal);
    if (d == nullptr) return false;
    const int32_t ia = static_cast<int32_t>(a);
    const int32_t ib = static_cast<int32_t>(b);
    // The date setters take a date set to Julian day `a`.
    cFC4Date date(static_cast<uint32_t>(a));
    switch (op) {
        case 0: d->SetType(ia); break;
        case 1: d->SetRegionCellOnOtherSideOfConnection(ia, ib); break;
        case 2: d->SetRegionCellOnCitySideOfConnection(ia, ib); break;
        case 3: d->SetLastAmount(ia); break;
        case 4: d->SetLastIncAmount(ia); break;
        case 5: d->SetIncAmount(ia); break;
        case 6: d->SetIncAmount2(ia); break;
        case 7: d->SetAmountUsed(ia); break;
        case 8: d->SetStartDate(*fc4::AsSdk(&date)); break;
        case 9: d->SetExpirationDate(*fc4::AsSdk(&date)); break;
        case 10: d->SetCurrentDate(*fc4::AsSdk(&date)); break;
        case 11: d->SetSuspensionDate(*fc4::AsSdk(&date)); break;
        case FC4_NEIGHBOR_DEAL_CALL_SET_AMOUNT: d->SetAmount(ia); break;
        case FC4_NEIGHBOR_DEAL_CALL_UPDATE_COST: d->UpdateCost(); break;
        case FC4_NEIGHBOR_DEAL_CALL_UPDATE_STATE: d->UpdateState(); break;
        case FC4_NEIGHBOR_DEAL_CALL_CANCEL: d->Cancel(); break;
        case FC4_NEIGHBOR_DEAL_CALL_ACTIVATE: return d->Activate();
        case FC4_NEIGHBOR_DEAL_CALL_SET_FLAGS: d->SetFlags(ia, b != 0); break;  // void at the interface
        default: return false;
    }
    return true;
}

// The reference AddDeal took for the core. The caller usually lets go of
// its own after AddDeal, so this one may be the last: reaching 0 is the
// expected free (the audited AddDeal acquired it).
void ReleaseDeal(void*, uint64_t deal) {
    cIGZUnknown* o = FromToken(deal);
    if (o != nullptr) FC4_RELEASE_CREATED(o);
}

}  // namespace

// ---- shared reads -----------------------------------------------------------------------------------

namespace fc4 {

uint32_t NeighborsFallbacks() { return g_fallbacks.load(); }

cISC4City* NeighborAppCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = Query<cISC4App>(app, GZIID_cISC4App);
    if (sc4 == nullptr) return nullptr;
    cISC4City* city = sc4->GetCity();  // borrowed
    Drop(sc4);
    return city;
}

cISC4RegionalCity* NeighborAppRegionalCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = Query<cISC4App>(app, GZIID_cISC4App);
    if (sc4 == nullptr) return nullptr;
    cISC4RegionalCity* r = sc4->GetRegionalCity();  // borrowed
    Drop(sc4);
    return r;
}

// NG15 (§15.4, matches): the cell is passed as the region's uint32
// arguments as it is; a negative cell has the same bits and finds no entry,
// as in the original (the map holds only cells covered by a city).
cISC4RegionalCity* NeighborRegionCityAt(int32_t rx, int32_t rz) {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = Query<cISC4App>(app, GZIID_cISC4App);
    if (sc4 == nullptr) return nullptr;
    cISC4Region* region = sc4->GetRegion();  // borrowed
    Drop(sc4);
    if (region == nullptr) return nullptr;
    cRZAutoRefCount<cISC4RegionalCity>* e = region->GetCity(static_cast<uint32_t>(rx), static_cast<uint32_t>(rz));
    return e ? static_cast<cISC4RegionalCity*>(*e) : nullptr;  // region-owned
}

bool ReadNeighborConnection(cISC4NeighborConnection* c, fc4_neighbor_connection_state* out) {
    if (out == nullptr) return false;
    *out = fc4_neighbor_connection_state{};
    if (c == nullptr) return false;
    out->conn_type = static_cast<uint32_t>(c->GetType());
    c->GetConnectionCell(out->x, out->z);
    c->GetRegionCellOnOtherSideOfConnection(out->other_x, out->other_z);
    out->visited = c->GetVisitedThisCycle() ? 1 : 0;
    out->active = c->GetIsActive() ? 1 : 0;
    return true;
}

bool ReadNeighborDeal(cISC4NeighborDeal* d, fc4_neighbor_deal_data* out) {
    if (out == nullptr) return false;
    *out = fc4_neighbor_deal_data{};
    if (d == nullptr) return false;
    if (cFC4NeighborDeal* ours = OurDeal(d)) {
        *out = ours->Data();
        return true;
    }
    out->deal_type = static_cast<uint32_t>(d->GetType());
    d->GetRegionCellOnOtherSideOfConnection(out->other_x, out->other_z);
    d->GetRegionCellOnCitySideOfConnection(out->city_x, out->city_z);
    out->amount = d->GetAmount();
    out->inc_amount = d->GetIncAmount();
    out->inc_amount2 = d->GetIncAmount2();
    out->last_amount = d->GetLastAmount();
    out->last_inc_amount = d->GetLastIncAmount();
    out->amount_used = d->GetAmountUsed();
    out->flags = static_cast<uint32_t>(d->GetFlags());
    out->cost = d->GetCost();  // EDX:EAX
    // The getters fill a date through its Set(dayNumber) (slot 4).
    cFC4Date date(0);
    d->GetStartDate(*AsSdk(&date));
    out->start_date = date.DayNumber();
    d->GetExpirationDate(*AsSdk(&date));
    out->expiration_date = date.DayNumber();
    d->GetCurrentDate(*AsSdk(&date));
    out->current_date = date.DayNumber();
    d->GetSuspensionDate(*AsSdk(&date));
    out->suspension_date = date.DayNumber();
    return true;
}

bool ReadNeighborCity(cISC4RegionalCity* c, fc4_neighbors_shadow_city* out) {
    if (out == nullptr) return false;
    *out = fc4_neighbors_shadow_city{};
    if (c == nullptr) return false;
    cFC4ISC4RegionalCity* r = AsGame(c);
    out->token = TokenOf(c);
    r->GetPosition(out->pos_x, out->pos_z);
    r->GetCitySize(out->size_x, out->size_z);
    out->established = r->GetEstablished() ? 1 : 0;
    out->budget = r->GetBudget();
    out->income = r->GetIncome();
    for (int32_t k = 0; k < 3; ++k) {
        out->stats[0][k] = r->GetExported(k + 1);
        out->stats[1][k] = r->GetImported(k + 1);
        out->stats[2][k] = r->GetProduced(k + 1);
        out->stats[3][k] = r->GetDemanded(k + 1);
        out->stats[4][k] = r->GetCostPerUnit(k + 1);
    }
    return true;
}

void ReadNeighborFigures(cISC4City* city, fc4_neighbors_figures* out) {
    if (out == nullptr) return;
    *out = fc4_neighbors_figures{};
    if (city == nullptr) return;
    if (cISC4BudgetSimulator* raw = city->GetBudgetSimulator()) {  // city slot 52
        cFC4ISC4BudgetSimulator* b = AsGame(raw);
        out->total_funds = static_cast<double>(b->GetTotalFunds());             // slot 6
        out->monthly_income = static_cast<double>(b->GetTotalMonthlyIncome());  // slot 16
        if (cISC4DepartmentBudget* d = b->GetDepartmentBudget(kDeptPower)) {    // slot 28, borrowed
            out->has_power_department = 1;
            out->power_department_expenses = static_cast<double>(AsGame(d)->GetTotalExpenses());  // slot 15
            if (cISC4LineItem* li = AsGame(d)->GetLineItem(kLineWasteToEnergy)) {               // slot 20
                out->waste_to_energy_expenses = static_cast<double>(li->GetFullExpenses());
            }
        }
        if (cISC4DepartmentBudget* d = b->GetDepartmentBudget(kDeptWater)) {
            out->has_water_department = 1;
            out->water_department_expenses = static_cast<double>(AsGame(d)->GetTotalExpenses());
        }
        if (cISC4DepartmentBudget* d = b->GetDepartmentBudget(kDeptSanitation)) {
            out->has_sanitation_department = 1;
            out->sanitation_department_expenses = static_cast<double>(AsGame(d)->GetTotalExpenses());
        }
    }
    if (cISC4PowerSimulator* raw = city->GetPowerSimulator()) {
        cFC4ISC4PowerSimulator* p = AsGame(raw);
        out->power_production = static_cast<double>(p->GetMonthlyProduction());    // slot 12
        out->power_requirement = static_cast<double>(p->GetMonthlyRequirement());  // slot 11
    }
    if (cISC4PlumbingSimulator* raw = city->GetPlumbingSimulator()) {
        cFC4ISC4PlumbingSimulator* p = AsGame(raw);
        out->water_capacity = static_cast<double>(p->GetWaterProductionCapacity());  // slot 14
        out->water_demand = static_cast<double>(p->GetWaterDemand());               // slot 23
    }
    if (cISC4PollutionSimulator* raw = city->GetPollutionSimulator()) {
        cFC4ISC4PollutionSimulator* p = AsGame(raw);
        out->garbage_recycled = static_cast<double>(p->GetGarbageRecycledLastMonth());               // 38
        out->waste_to_energy_capacity = static_cast<double>(p->GetMonthlyWasteToEnergyCapacity());  // 32
        out->incinerator_capacity = static_cast<double>(p->GetMonthlyIncineratorCapacity());        // 31
        // Slots 34/35 return 64 bits in edx:eax, as cFC4ISC4PollutionSimulator declares (the SDK: 32).
        out->garbage_abducted = static_cast<double>(p->GetGarbageAbductedPrevMonth());  // 34 (abi: via cFC4I...)
        out->garbage_produced = static_cast<double>(p->GetGarbageProducedLastMonth());  // 35 (abi: via cFC4I...)
    }
}

void ReadNeighborLandfill(cISC4City* city, uint32_t* usage, uint32_t* capacity) {
    uint32_t u = 0, c = 0;
    cISC4LandfillDeveloper* dev = city ? city->GetLandfillDeveloper() : nullptr;
    if (dev != nullptr) AsGame(dev)->GetLandfillUsage(u, c);  // slot 10
    if (usage != nullptr) *usage = u;
    if (capacity != nullptr) *capacity = c;
}

uint32_t NeighborToday(cISC4City* city) {
    cISC4Simulator* s = city ? city->GetSimulator() : nullptr;
    cFC4IGZDate* d = s ? AsGame(s)->GetSimDate() : nullptr;  // slot 7
    return d ? d->DayNumber() : 0;                           // date slot 26
}

fc4_props* LoadNeighborTuning(props::Exemplar* keep) {
    if (keep == nullptr || !keep->Load(kTuningType, kTuningGroup, kTuningInstance)) return nullptr;
    return props::ToCoreAll(keep->Holder());
}

fc4_neighbors_host MakeNeighborsHost(NeighborsContext* ctx) {
    fc4_neighbors_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.tuning = &Tuning;
    h.add_agent = &AddAgent;
    h.remove_agents = &RemoveAgents;
    h.subscribe = &Subscribe;
    h.unsubscribe = &Unsubscribe;
    h.cell_counts = &CellCounts;
    h.today = &Today;
    h.utility_figures = &UtilityFigures;
    h.landfill_usage = &LandfillUsage;
    h.send_history = &SendHistory;
    h.current_city = &CurrentCity;
    h.current_city_object = &CurrentCityObject;
    h.city_at = &CityAt;
    h.city_position = &CityPosition;
    h.city_size = &CitySize;
    h.city_established = &CityEstablished;
    h.city_stat = &CityStat;
    h.set_city_stat = &SetCityStat;
    h.city_budget = &CityBudget;
    h.set_city_budget = &SetCityBudget;
    h.set_city_income = &SetCityIncome;
    h.city_find_connection = &CityFindConnection;
    h.city_connections = &CityConnections;
    h.change_symmetric_connection = &ChangeSymmetricConnection;
    h.reload_city = &ReloadCity;
    h.update_local_deals = &UpdateLocalDeals;
    h.local_deals = &LocalDeals;
    h.set_local_deals = &SetLocalDeals;
    h.update_import_export = &UpdateImportExport;
    h.post_status = &PostStatus;
    h.send_status = &SendStatus;
    h.post_connection_message = &PostConnectionMessage;
    h.cell_center = &CellCenter;
    h.marker_altitude = &MarkerAltitude;
    h.max_surface_level = &MaxSurfaceLevel;
    h.add_marker_prop = &AddMarkerProp;
    h.marker_props = &MarkerProps;
    h.remove_prop = &RemoveProp;
    h.move_prop = &MoveProp;
    h.occupant_type = &OccupantType;
    h.occupant_networks = &OccupantNetworks;
    h.occupant_cell = &OccupantCell;
    h.power_pole_flag = &PowerPoleFlag;
    h.connection_object = &ConnectionObject;
    h.deal_object = &DealObject;
    h.foreign_connection = &ForeignConnection;
    h.set_foreign_connection_flags = &SetForeignConnectionFlags;
    h.repair_foreign_connection = &RepairForeignConnection;
    h.release_connection = &ReleaseConnection;
    h.foreign_deal = &ForeignDeal;  // NG4 (ABI 45)
    h.foreign_deal_call = &ForeignDealCall;
    h.release_deal = &ReleaseDeal;
    return h;
}

}  // namespace fc4

// ---- trip list ------------------------------------------------------------------------------------

namespace {

void FreeVector(FC4TripVector& v) {
    if (v.begin != nullptr) fc4::StlNodeAllocator::Free(v.begin);
    v = FC4TripVector{};
}

bool g_noteNoTripList = false;

}  // namespace

FC4TripList::FC4TripList() : head_(nullptr) {
    if (!fc4::StlNodeAllocator::Available()) return;
    auto* h = static_cast<FC4TripNode*>(fc4::StlNodeAllocator::Allocate(static_cast<uint32_t>(sizeof(FC4TripNode))));
    if (h == nullptr) return;
    std::memset(static_cast<void*>(h), 0, sizeof(FC4TripNode));
    h->next = h;
    h->prev = h;
    head_ = h;
}

FC4TripList::~FC4TripList() {
    if (head_ == nullptr) return;
    FC4TripNode* n = head_->next;
    while (n != head_) n = Erase(n);
    fc4::StlNodeAllocator::Free(head_);
    head_ = nullptr;
}

bool FC4TripList::PushBack(const FC4TripData& t) {
    if (head_ == nullptr) return false;
    auto* n = static_cast<FC4TripNode*>(fc4::StlNodeAllocator::Allocate(static_cast<uint32_t>(sizeof(FC4TripNode))));
    if (n == nullptr) return false;
    n->value = t;
    n->next = head_;
    n->prev = head_->prev;
    head_->prev->next = n;
    head_->prev = n;
    return true;
}

FC4TripNode* FC4TripList::Erase(FC4TripNode* n) {
    FC4TripNode* next = n->next;
    n->prev->next = n->next;
    n->next->prev = n->prev;
    FreeVector(n->value.route);
    FreeVector(n->value.returnRoute);
    fc4::StlNodeAllocator::Free(n);
    return next;
}

// ---- traffic source -------------------------------------------------------------------------------

bool cFC4NeighborTrafficSource::QueryInterface(uint32_t riid, void** ppvObj) {
    return owner_->QueryInterface(riid, ppvObj);
}

uint32_t cFC4NeighborTrafficSource::AddRef() { return owner_->AddRef(); }

uint32_t cFC4NeighborTrafficSource::Release() { return owner_->Release(); }

SC4List<cSC4TripData>* cFC4NeighborTrafficSource::GetTripDataList() {
    if (!trips_.valid()) {
        if (!g_noteNoTripList) {
            g_noteNoTripList = true;
            fc4::Log("neighbours: no trip list (the game's node allocator is not available on this executable); "
                     "GetTripDataList returns null");
        }
        return nullptr;
    }
    return reinterpret_cast<SC4List<cSC4TripData>*>(&trips_);
}

// traffic-simulator.md §4.1.4. The volume-dirty flag is cleared first, before
// the other side is looked up, so each connection object syncs once (NG8,
// neighbors.md §15.4). Without a matching connection on the other side (no
// city there, or no match) the whole trip list is cleared, every type
// included (NG19). The result is true: the one caller that reaches a
// connection's slot 4 ignores it (NG21).
bool cFC4NeighborTrafficSource::UpdateTrafficVolume() {
    if (!owner_->TakeVolumeDirty() || !trips_.valid()) return true;
    uint32_t target[3][9] = {};
    cISC4City* city = fc4::NeighborAppCity();
    cISC4NeighborsSimulator* ns = city ? city->GetNeighborsSimulator() : nullptr;
    cISC4NeighborConnection* other = ns ? fc4::AsGame(ns)->FindConnectionOnOtherSideOf(owner_->Primary()) : nullptr;
    if (other == nullptr) {
        FC4TripNode* const head = trips_.head();
        for (FC4TripNode* n = head->next; n != head;) n = trips_.Erase(n);
        return true;
    }
    const cISCPropertyHolder* holder = other->AsISCPropertyHolder();
    for (uint32_t w = 0; w < 3 && holder != nullptr; ++w) {
        std::vector<uint32_t> v;
        if (!fc4::props::GetU32s(holder, kTripTargets + w, v)) continue;
        for (size_t s = 0; s < 9 && s < v.size(); ++s) target[w][s] = v[s];
    }
    FC4TripNode* const head = trips_.head();
    for (uint8_t w = 0; w < 3; ++w) {
        for (uint8_t s = 0; s < 9; ++s) {
            const uint32_t want = target[w][s];
            uint32_t have = 0;
            for (FC4TripNode* n = head->next; n != head; n = n->next) {
                if (n->value.type == w && n->value.strategy == s) have += n->value.count;
            }
            if (have < want) {
                const uint32_t d = (want - have) & 0xFFFFu;  // [quirk: 16-bit]
                FC4TripNode* hit = nullptr;
                for (FC4TripNode* n = head->next; n != head && hit == nullptr; n = n->next) {
                    if (n->value.type == w && n->value.strategy == s && n->value.status == 5) hit = n;
                }
                if (hit != nullptr) {
                    hit->value.count += d;
                } else {
                    FC4TripData t{};
                    t.count = d;
                    t.type = w;
                    t.strategy = s;
                    t.flags = 4;  // forced start type
                    t.status = 0;
                    trips_.PushBack(t);
                }
            } else if (have > want) {
                uint32_t excess = have - want;
                for (int pass = 0; pass < 2 && excess != 0; ++pass) {
                    FC4TripNode* n = head->next;
                    while (n != head && excess != 0) {
                        FC4TripData& t = n->value;
                        if (t.type == w && t.strategy == s && (pass == 1 || t.status == 5)) {
                            const uint32_t take = std::min(excess, t.count);
                            t.count -= take;
                            excess -= take;
                            if (t.count == 0) {
                                n = trips_.Erase(n);
                                continue;
                            }
                        }
                        n = n->next;
                    }
                }
            }
        }
    }
    return true;
}

// NG20 (neighbors.md §15.4): the connection cell itself (outside the city,
// e.g. (-1, 5)), twice: (x, z, x, z); the unsigned fields take the signed
// cell as it is.
bool cFC4NeighborTrafficSource::GetSourceCells(SC4Rect<uint32_t>& rect) {
    const FC4ConnectionMirror m = owner_->State();
    rect.topLeftX = static_cast<uint32_t>(m.state.x);
    rect.topLeftY = static_cast<uint32_t>(m.state.z);
    rect.bottomRightX = static_cast<uint32_t>(m.state.x);
    rect.bottomRightY = static_cast<uint32_t>(m.state.z);
    return true;
}

// ---- connection -----------------------------------------------------------------------------------

cFC4NeighborConnection::cFC4NeighborConnection(fc4_neighbor_connection* handle)
    : h_(handle), holder_(static_cast<cISC4NeighborConnection*>(this)), traffic_(this) {
    Register(g_conns, this);
    RefreshMirror();
}

cFC4NeighborConnection::~cFC4NeighborConnection() {
    Unregister(g_conns, this);
    {
        Section sec;
        fc4_neighbor_connection_destroy(h_);
        h_ = nullptr;
    }
    holder_.Clear();
}

// §1: cISC4NeighborConnection and cIGZUnknown give the primary, plus
// cIGZSerializable; not the property holder (reached by AsISCPropertyHolder)
// nor the traffic source, whose QueryInterface forwards here (NG26, §15.4:
// matches); the private IID gives this object.
bool cFC4NeighborConnection::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (riid == kFC4_IID_cISC4NeighborConnection || riid == GZIID_cIGZUnknown) {
        *ppvObj = Primary();
    } else if (riid == GZIID_cIGZSerializable) {
        *ppvObj = static_cast<cIGZSerializable*>(this);
    } else if (riid == kIID_FC4NeighborConnection) {
        *ppvObj = this;
    } else {
        return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4NeighborConnection::AddRef() { return ++refs_; }

uint32_t cFC4NeighborConnection::Release() {
    uint32_t cur = refs_.load();
    while (cur != 0 && !refs_.compare_exchange_weak(cur, cur - 1)) {
    }
    if (cur == 0) return 0;  // over-release: ignored
    if (cur == 1) Park(this);
    return cur - 1;
}

FC4ConnectionMirror cFC4NeighborConnection::State() const {
    Section sec(false);
    if (!sec.ok() || h_ == nullptr) return mirror_.Get();
    FC4ConnectionMirror m{};
    fc4_neighbor_connection_get(h_, &m.state);
    int32_t cx = 0, cz = 0;
    CityCounts(&cx, &cz);
    m.innerOk = fc4_neighbor_connection_inner_cell(h_, cx, cz, &m.innerX, &m.innerZ) ? 1 : 0;
    return m;
}

void cFC4NeighborConnection::RefreshMirror() {
    if (h_ == nullptr) return;
    FC4ConnectionMirror m{};
    fc4_neighbor_connection_get(h_, &m.state);
    m.innerOk = fc4_neighbor_connection_inner_cell(h_, g_countX.load(), g_countZ.load(), &m.innerX, &m.innerZ) ? 1 : 0;
    mirror_.Set(m);
}

cISCPropertyHolder* cFC4NeighborConnection::AsISCPropertyHolder() { return holder_.AsSdk(); }

cISC4TrafficSource* cFC4NeighborConnection::AsTrafficSource() { return &traffic_; }

cISC4NeighborsSimulator::ConnectionType cFC4NeighborConnection::GetType() const {
    return static_cast<cISC4NeighborsSimulator::ConnectionType>(State().state.conn_type);
}

void cFC4NeighborConnection::GetConnectionCell(int32_t& cellX, int32_t& cellZ) const {
    const FC4ConnectionMirror m = State();
    cellX = m.state.x;
    cellZ = m.state.z;
}

bool cFC4NeighborConnection::GetConnectionInnerCell(int32_t& cellX, int32_t& cellZ) const {
    const FC4ConnectionMirror m = State();
    if (m.innerOk == 0) return false;
    cellX = m.innerX;
    cellZ = m.innerZ;
    return true;
}

void cFC4NeighborConnection::GetRegionCellOnOtherSideOfConnection(int32_t& cellX, int32_t& cellZ) const {
    const FC4ConnectionMirror m = State();
    cellX = m.state.other_x;
    cellZ = m.state.other_z;
}

cISC4RegionalCity* cFC4NeighborConnection::GetCityOnOtherSideOfConnection() const {
    const FC4ConnectionMirror m = State();
    return fc4::NeighborRegionCityAt(m.state.other_x, m.state.other_z);
}

// §4.2: otherRegionCell from the cell with R's position and size.
// [choice] A null R changes nothing.
void cFC4NeighborConnection::Repair(cISC4RegionalCity* pCity) {
    if (pCity == nullptr) return;
    int32_t px = 0, pz = 0, sx = 0, sz = 0;
    fc4::AsGame(pCity)->GetPosition(px, pz);
    fc4::AsGame(pCity)->GetCitySize(sx, sz);
    Section sec;
    if (!sec.ok()) {
        NoteDropped("connection Repair");
        return;
    }
    int32_t cx = 0, cz = 0;
    CityCounts(&cx, &cz);
    fc4_neighbor_connection_repair(h_, cx, cz, px, pz, sx, sz);
}

bool cFC4NeighborConnection::GetVisitedThisCycle() const { return State().state.visited != 0; }

void cFC4NeighborConnection::SetVisitedThisCycle(bool visited) {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("connection SetVisitedThisCycle");
        return;
    }
    fc4_neighbor_connection_set_field(h_, 3, visited ? 1 : 0, 0);
}

bool cFC4NeighborConnection::GetIsActive() const { return State().state.active != 0; }

void cFC4NeighborConnection::SetIsActive(bool active) {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("connection SetIsActive");
        return;
    }
    fc4_neighbor_connection_set_field(h_, 4, active ? 1 : 0, 0);
}

// §4.3: the holder goes to the core (a copy) and the core writes the record.
bool cFC4NeighborConnection::Write(cIGZOStream& gzOut) {
    Section sec;
    if (!sec.ok()) return false;
    fc4_props* p = holder_.ToCore();
    fc4_neighbor_connection_set_props(h_, p);
    fc4_props_destroy(p);
    fc4::OStreamAdapter a(gzOut);
    return fc4_neighbor_connection_write(h_, a.Get());
}

// §4.3 (version 1 only); the holder takes the properties read.
bool cFC4NeighborConnection::Read(cIGZIStream& gzIn) {
    Section sec;
    if (!sec.ok()) return false;
    fc4::IStreamAdapter a(gzIn);
    const bool ok = fc4_neighbor_connection_read(h_, a.Get());
    holder_.Clear();
    fc4_neighbor_connection_props(h_, &cFC4PropertyHolder::FromCoreProperty, &holder_);
    return ok;
}

uint32_t cFC4NeighborConnection::GetGZCLSID() { return kFC4_CLSID_cSC4NeighborConnection; }

// ---- deal -----------------------------------------------------------------------------------------

cFC4NeighborDeal::cFC4NeighborDeal(fc4_neighbor_deal* handle) : h_(handle) {
    fc4_neighbor_deal_set_token(h_, TokenOf(Primary()));
    Register(g_deals, this);
    RefreshMirror();
}

cFC4NeighborDeal::~cFC4NeighborDeal() {
    Unregister(g_deals, this);
    Section sec;
    fc4_neighbor_deal_destroy(h_);  // the core forgets this object's token
    h_ = nullptr;
}

// §1: cISC4NeighborDeal and cIGZUnknown give the primary, plus
// cIGZSerializable; the private IID gives this object. NG18 (§15.4): the
// original deal's holder is unreachable too; the core keeps the properties of
// a record it read and writes them back (§5.6).
bool cFC4NeighborDeal::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (riid == kFC4_IID_cISC4NeighborDeal || riid == GZIID_cIGZUnknown) {
        *ppvObj = Primary();
    } else if (riid == GZIID_cIGZSerializable) {
        *ppvObj = static_cast<cIGZSerializable*>(this);
    } else if (riid == kIID_FC4NeighborDeal) {
        *ppvObj = this;
    } else {
        return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4NeighborDeal::AddRef() { return ++refs_; }

uint32_t cFC4NeighborDeal::Release() {
    uint32_t cur = refs_.load();
    while (cur != 0 && !refs_.compare_exchange_weak(cur, cur - 1)) {
    }
    if (cur == 0) return 0;  // over-release: ignored
    if (cur == 1) Park(this);
    return cur - 1;
}

fc4_neighbor_deal_data cFC4NeighborDeal::Data() const {
    Section sec(false);
    if (!sec.ok() || h_ == nullptr) return mirror_.Get();
    fc4_neighbor_deal_data d{};
    fc4_neighbor_deal_get(h_, &d);
    return d;
}

void cFC4NeighborDeal::RefreshMirror() {
    if (h_ == nullptr) return;
    fc4_neighbor_deal_data d{};
    fc4_neighbor_deal_get(h_, &d);
    mirror_.Set(d);
}

void cFC4NeighborDeal::Claim() const {
    if (g_current != nullptr && g_current->Core() != nullptr) fc4_neighbors_claim_deal(g_current->Core(), h_);
}

void cFC4NeighborDeal::SetField(uint32_t field, int32_t a, int32_t b) {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("deal setter");
        return;
    }
    fc4_neighbor_deal_set_field(h_, field, a, b);
}

void cFC4NeighborDeal::SetRegionCellOnCitySideOfConnection(int32_t x, int32_t z) { SetField(2, x, z); }

void cFC4NeighborDeal::GetRegionCellOnCitySideOfConnection(int32_t& x, int32_t& z) const {
    const fc4_neighbor_deal_data d = Data();
    x = d.city_x;
    z = d.city_z;
}

void cFC4NeighborDeal::SetRegionCellOnOtherSideOfConnection(int32_t x, int32_t z) { SetField(1, x, z); }

void cFC4NeighborDeal::GetRegionCellOnOtherSideOfConnection(int32_t& x, int32_t& z) const {
    const fc4_neighbor_deal_data d = Data();
    x = d.other_x;
    z = d.other_z;
}

cISC4RegionalCity* cFC4NeighborDeal::GetCityOnOtherSideOfConnection() const {
    const fc4_neighbor_deal_data d = Data();
    return fc4::NeighborRegionCityAt(d.other_x, d.other_z);
}

int32_t cFC4NeighborDeal::GetType() const { return static_cast<int32_t>(Data().deal_type); }
void cFC4NeighborDeal::SetType(int32_t type) { SetField(0, type, 0); }
int32_t cFC4NeighborDeal::GetAmount() const { return Data().amount; }

// §5.4: through the current simulator (the core checks Grace / Probation).
void cFC4NeighborDeal::SetAmount(int32_t amount) {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("deal SetAmount");
        return;
    }
    Claim();
    fc4_neighbor_deal_set_amount(h_, amount);
}

int32_t cFC4NeighborDeal::GetLastAmount() const { return Data().last_amount; }
void cFC4NeighborDeal::SetLastAmount(int32_t v) { SetField(3, v, 0); }
int32_t cFC4NeighborDeal::GetLastIncAmount() const { return Data().last_inc_amount; }
void cFC4NeighborDeal::SetLastIncAmount(int32_t v) { SetField(4, v, 0); }
int32_t cFC4NeighborDeal::GetIncAmount() const { return Data().inc_amount; }
void cFC4NeighborDeal::SetIncAmount(int32_t v) { SetField(5, v, 0); }
int32_t cFC4NeighborDeal::GetIncAmount2() const { return Data().inc_amount2; }
void cFC4NeighborDeal::SetIncAmount2(int32_t v) { SetField(6, v, 0); }
int32_t cFC4NeighborDeal::GetAmountUsed() const { return Data().amount_used; }
void cFC4NeighborDeal::SetAmountUsed(int32_t v) { SetField(7, v, 0); }
int64_t cFC4NeighborDeal::GetCost() const { return Data().cost; }

void cFC4NeighborDeal::UpdateCost() {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("deal UpdateCost");
        return;
    }
    Claim();
    fc4_neighbor_deal_update_cost(h_);
}

// The date getters fill the caller's date through Set(dayNumber) (slot 4);
// the setters read its DayNumber (slot 26).
void cFC4NeighborDeal::GetStartDate(cIGZDate& date) const { fc4::AsGame(&date)->SetDayNumber(Data().start_date); }
void cFC4NeighborDeal::SetStartDate(cIGZDate const& date) {
    SetField(8, static_cast<int32_t>(const_cast<cFC4IGZDate*>(fc4::AsGame(&date))->DayNumber()), 0);
}
void cFC4NeighborDeal::GetExpirationDate(cIGZDate& date) const {
    fc4::AsGame(&date)->SetDayNumber(Data().expiration_date);
}
void cFC4NeighborDeal::SetExpirationDate(cIGZDate const& date) {
    SetField(9, static_cast<int32_t>(const_cast<cFC4IGZDate*>(fc4::AsGame(&date))->DayNumber()), 0);
}
void cFC4NeighborDeal::GetCurrentDate(cIGZDate& date) { fc4::AsGame(&date)->SetDayNumber(Data().current_date); }
void cFC4NeighborDeal::SetCurrentDate(cIGZDate const& date) {
    SetField(10, static_cast<int32_t>(const_cast<cFC4IGZDate*>(fc4::AsGame(&date))->DayNumber()), 0);
}
void cFC4NeighborDeal::GetSuspensionDate(cIGZDate& date) {
    fc4::AsGame(&date)->SetDayNumber(Data().suspension_date);
}
void cFC4NeighborDeal::SetSuspensionDate(cIGZDate const& date) {
    SetField(11, static_cast<int32_t>(const_cast<cFC4IGZDate*>(fc4::AsGame(&date))->DayNumber()), 0);
}

// §5.4: the source through its interface (ours or an original). NG13
// (§15.4): void matches (the original leaves no meaningful value).
void cFC4NeighborDeal::Copy(cISC4NeighborDeal const& copyFrom) {
    fc4_neighbor_deal_data d{};
    if (!fc4::ReadNeighborDeal(const_cast<cISC4NeighborDeal*>(&copyFrom), &d)) return;
    Section sec;
    if (!sec.ok()) {
        NoteDropped("deal Copy");
        return;
    }
    fc4_neighbor_deal_copy(h_, &d);
}

void cFC4NeighborDeal::UpdateState() {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("deal UpdateState");
        return;
    }
    Claim();
    fc4_neighbor_deal_update_state(h_);
}

void cFC4NeighborDeal::Cancel() {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("deal Cancel");
        return;
    }
    Claim();
    fc4_neighbor_deal_cancel(h_);
}

bool cFC4NeighborDeal::Activate() {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("deal Activate");
        return false;
    }
    Claim();
    return fc4_neighbor_deal_activate(h_);
}

int32_t cFC4NeighborDeal::GetFlags() const { return static_cast<int32_t>(Data().flags); }

void cFC4NeighborDeal::SetFlags(int32_t flags, bool value) {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("deal SetFlags");
        return;
    }
    Claim();
    fc4_neighbor_deal_set_flags(h_, static_cast<uint32_t>(flags), value);
}

bool cFC4NeighborDeal::CheckFlagsAnd(int32_t flags) const {
    const uint32_t f = static_cast<uint32_t>(flags);
    return (Data().flags & f) == f;
}

bool cFC4NeighborDeal::CheckFlagsOr(int32_t flags) const { return (Data().flags & static_cast<uint32_t>(flags)) != 0; }

// §5.5: a new deal of ours; `out` gets it with one reference.
bool cFC4NeighborDeal::Clone(cISC4NeighborDeal** outCopy, bool silent) const {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("deal Clone");
        return false;
    }
    Claim();
    fc4_neighbor_deal* nh = fc4_neighbor_deal_clone(h_, silent);
    if (nh == nullptr) return false;
    auto* c = new cFC4NeighborDeal(nh);
    c->AddRef();  // the caller's
    if (outCopy != nullptr) {
        *outCopy = c->Primary();
    } else {
        c->Release();
    }
    return true;
}

bool cFC4NeighborDeal::Write(cIGZOStream& gzOut) {
    Section sec;
    if (!sec.ok()) return false;
    fc4::OStreamAdapter a(gzOut);
    return fc4_neighbor_deal_write(h_, a.Get());
}

bool cFC4NeighborDeal::Read(cIGZIStream& gzIn) {
    Section sec;
    if (!sec.ok()) return false;
    fc4::IStreamAdapter a(gzIn);
    return fc4_neighbor_deal_read(h_, a.Get());
}

uint32_t cFC4NeighborDeal::GetGZCLSID() { return kFC4_CLSID_cSC4NeighborDeal; }

// ---- simulator ------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4NeighborsSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4NeighborsSimulator*>(new cFC4NeighborsSimulator()));
}

cFC4NeighborsSimulator::cFC4NeighborsSimulator() {
    if (g_mainTid.load() == 0) g_mainTid = GetCurrentThreadId();  // the class store runs on the main thread
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    host_ = fc4::MakeNeighborsHost(&ctx_);
    bool fixed = false;
    const uint32_t seed = SessionSeed(&fixed);
    core_ = fc4_neighbors_create(&host_, TokenOf(Primary()), seed);
    twoCurves_ = fc4::SettingInt("neighbors_two_curves", "FC4_NEIGHBORS_TWO_CURVES", 0) != 0;
    if (core_ != nullptr && twoCurves_) fc4_neighbors_set_two_curves(core_, true);
    Register(g_sims, this);
    fc4::Log("cFC4NeighborsSimulator(%p) created, core %p, session seed 0x%08X (%s)%s", static_cast<void*>(Primary()),
             static_cast<void*>(core_), seed, fixed ? "neighbors_seed" : "session seed",
             twoCurves_ ? ", two price curves (neighbors_two_curves)" : "");
}

cFC4NeighborsSimulator::~cFC4NeighborsSimulator() {
    {
        Section sec;
        Unregister(g_sims, this);
        if (g_current == this) g_current = nullptr;
        fc4_neighbors_destroy(core_);  // releases what a loaded, never initialised object still holds
        core_ = nullptr;
    }
    fc4::Log("cFC4NeighborsSimulator destroyed (%u months, %u fallbacks)", months_, g_fallbacks.load());
}

// §1: the primary for its IID and cIGZUnknown, the message target, the
// serializable; anything else is false with the out pointer untouched.
bool cFC4NeighborsSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (riid == kFC4_IID_cISC4NeighborsSimulator || riid == GZIID_cIGZUnknown) {
        *ppvObj = Primary();
    } else if (riid == kIID_cIGZMessageTarget2) {
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
    } else if (riid == GZIID_cIGZSerializable) {
        *ppvObj = static_cast<cIGZSerializable*>(this);
    } else {
        return false;
    }
    ++refCount_;
    return true;
}

uint32_t cFC4NeighborsSimulator::AddRef() { return ++refCount_; }

// As the other subsystems: at a count of 1 (or below) Shutdown, destroy,
// return 0.
uint32_t cFC4NeighborsSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    if (destroying_) return 0;
    destroying_ = true;
    Shutdown();
    delete this;
    return 0;
}

void cFC4NeighborsSimulator::RefreshSnapshot() {
    if (core_ == nullptr) return;
    fc4_neighbors_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_neighbors_get_state(core_, &st)) return;
    snap_.income = st.income;
    snap_.expense = st.expense;
    snap_.connections = st.connections;
    snap_.deals = st.deals;
}

void cFC4NeighborsSimulator::AcquireServices() {
    if (ctx_.city == nullptr) {
        ctx_.city = fc4::NeighborAppCity();
        if (ctx_.city != nullptr) FC4_ADDREF(ctx_.city);
    }
    if (ctx_.city == nullptr) fc4::Log("neighbours: no city; services not acquired");
    if (ctx_.msgServer == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            ctx_.msgServer = ms;
            FC4_ADDREF(ctx_.msgServer);
        }
    }
}

void cFC4NeighborsSimulator::ReleaseServices() {
    if (ctx_.msgServer != nullptr) FC4_RELEASE(ctx_.msgServer);
    ctx_.msgServer = nullptr;
    if (ctx_.city != nullptr) FC4_RELEASE(ctx_.city);
    ctx_.city = nullptr;
}

// Init, Read, Write and Shutdown lines. Inside a section.
void cFC4NeighborsSimulator::LogState(const char* what) {
    fc4_neighbors_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_neighbors_get_state(core_, &st)) {
        fc4::Log("neighbours %s: no core", what);
        return;
    }
    fc4::Log("neighbours %s: init=%u ready=%u connections=%u (foreign %u) deals=%u dealsDirty=%u statsDirty=%u "
             "uiDisabled=%u busy=%u income=%lld expense=%lld tuning=%s current=%u fallbacks=%u",
             what, st.initialized, st.ready, st.connections, st.foreign_connections, st.deals, st.deals_dirty,
             st.stats_dirty, st.ui_disabled, st.busy, static_cast<long long>(st.income),
             static_cast<long long>(st.expense), (ctx_.tuning && ctx_.tuning->Resource()) ? "yes" : "no",
             g_current == this ? 1u : 0u, g_fallbacks.load());
}

// The month line: connections by type, deals by state, the totals, the
// current regional city's statistics, the calls made. Inside a section.
void cFC4NeighborsSimulator::LogMonth(uint32_t julian) {
    fc4_neighbors_state st{};
    st.struct_size = sizeof(st);
    fc4_neighbors_get_state(core_, &st);
    std::vector<uint64_t> conns;
    fc4_neighbors_connections(core_, 0, 0, 0, &PushToken, &conns);
    uint32_t byType[13] = {};
    for (uint64_t t : conns) {
        const uint32_t type = static_cast<uint32_t>(ConnOf(t)->GetType());
        if (type < 13) ++byType[type];
    }
    std::string types;
    for (uint32_t i = 0; i < 13; ++i) {
        if (byType[i] == 0) continue;
        char b[48];
        snprintf(b, sizeof(b), "%s%s=%u", types.empty() ? "" : " ", TypeName(i), byType[i]);
        types += b;
    }
    const size_t n = fc4_neighbors_deal_data(core_, nullptr, 0);
    std::vector<fc4_neighbor_deal_data> deals(n);
    if (n != 0) fc4_neighbors_deal_data(core_, deals.data(), n);
    uint32_t proposed = 0, active = 0, cancelled = 0, failing = 0, expired = 0, silent = 0, grace = 0, probation = 0;
    for (const fc4_neighbor_deal_data& d : deals) {
        proposed += (d.flags & 0x1u) ? 1 : 0;
        active += (d.flags & 0x2u) ? 1 : 0;
        cancelled += (d.flags & 0x4u) ? 1 : 0;
        failing += (d.flags & 0x38u) ? 1 : 0;
        expired += (d.flags & 0x40u) ? 1 : 0;
        silent += (d.flags & 0x80u) ? 1 : 0;
        grace += (d.flags & 0x100u) ? 1 : 0;
        probation += (d.flags & 0x200u) ? 1 : 0;
    }
    std::string stats = "no regional city";
    fc4_neighbors_shadow_city rc{};
    if (fc4::On(fc4::Sw::NeighborsRegionReads) && fc4::ReadNeighborCity(fc4::NeighborAppRegionalCity(), &rc)) {
        char b[640];
        snprintf(b, sizeof(b),
                 "budget=%.0f income=%.0f power exp/imp/prod/dem/cpu=%.0f/%.0f/%.0f/%.0f/%.6g water=%.0f/%.0f/%.0f/"
                 "%.0f/%.6g garbage=%.0f/%.0f/%.0f/%.0f/%.6g",
                 rc.budget, rc.income, rc.stats[0][0], rc.stats[1][0], rc.stats[2][0], rc.stats[3][0], rc.stats[4][0],
                 rc.stats[0][1], rc.stats[1][1], rc.stats[2][1], rc.stats[3][1], rc.stats[4][1], rc.stats[0][2],
                 rc.stats[1][2], rc.stats[2][2], rc.stats[3][2], rc.stats[4][2]);
        stats = b;
    }
    char date[16];
    FormatDate(julian, date, sizeof(date));
    fc4::Log("neighbours month to %s: connections=%u (%s) foreign=%u deals=%u (proposed %u active %u cancelled %u "
             "failing %u expired %u silent %u grace %u probation %u) income=%lld expense=%lld ready=%u; %s; posts=%u "
             "sends=%u connectionMessages=%u created=%u deleted=%u setLocalDeals=%u removedByRegion=%u markers "
             "+%u -%u ~%u history=%u reloads=%u fallbacks=%u months=%u",
             date, static_cast<uint32_t>(conns.size()), types.empty() ? "-" : types.c_str(), st.foreign_connections,
             static_cast<uint32_t>(n), proposed, active, cancelled, failing, expired, silent, grace, probation,
             static_cast<long long>(st.income), static_cast<long long>(st.expense), st.ready, stats.c_str(),
             ctx_.posts, ctx_.sends, ctx_.connectionMessages, ctx_.created, ctx_.deleted, ctx_.setLocalDeals,
             ctx_.regionRemoved, ctx_.markersAdded, ctx_.markersRemoved, ctx_.markersMoved, ctx_.history,
             ctx_.reloads, g_fallbacks.load(), months_);
}

template <class T>
void cFC4NeighborsSimulator::Fill(SC4List<T*>& list, const std::vector<uint64_t>& tokens, bool clearFirst) {
    if (clearFirst) {
        while (!list.empty()) list.erase(list.begin());  // releases each element
    }
    for (uint64_t t : tokens) {
        T* p = reinterpret_cast<T*>(FromToken(t));
        if (p == nullptr) continue;
        FC4_ADDREF_FOR_CALLER(p);
        list.push_back(p);
    }
}

// ---- lifecycle (§14) ------------------------------------------------------------------------------

bool cFC4NeighborsSimulator::Init() {
    FC4_TRACE("NeighborsSimulator::Init");
    g_mainTid = GetCurrentThreadId();
    Section sec;
    AcquireServices();
    g_current = this;
    CityCounts(nullptr, nullptr);
    fc4_neighbors_init(core_);  // §14.1, all of it
    LogState("Init");
    return true;  // §14.1
}

bool cFC4NeighborsSimulator::Shutdown() {
    FC4_TRACE("NeighborsSimulator::Shutdown");
    // Keep this object alive while the simulator and the message server drop
    // their references (unless this already is the final Release).
    if (!destroying_) Primary()->AddRef();
    {
        Section sec;
        fc4_neighbors_shutdown(core_);  // §14.2
        ctx_.tuning.reset();            // "release the tuning exemplar"
        ReleaseServices();
        if (g_current == this) g_current = nullptr;
    }
    fc4::Log("neighbours Shutdown: %u months, %u fallbacks", months_, g_fallbacks.load());
    if (!destroying_) Primary()->Release();
    return true;  // §14.2
}

// ---- connections (slots 5-23) ---------------------------------------------------------------------

namespace {
bool g_noteNoDialog = false;
bool g_noteDeclined = false;
bool g_noteAsked = false;
cISC4App* g_testApp = nullptr;  // FC4TestSeam op 48 (borrowed)
fc4::NeighborsConfirmFn g_testConfirm = nullptr;  // FC4TestSeam op 66

// SC4GetPopupModalDialogsEnabled (neighbors.md §15.4 NG36, the original
// 0x005F9090): cISC4App::GetPopupDialogsEnabled (slot 23, advisors.md
// §6.8.1), or false with no app (NG36: the original treats a missing app as
// popups off, so the request is refused). The test's app is borrowed and
// called directly; the framework's is queried and released.
bool PopupDialogsEnabled() {
    if (g_testApp != nullptr) return g_testApp->GetPopupDialogsEnabled();
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = Query<cISC4App>(app, GZIID_cISC4App);
    if (sc4 == nullptr) return false;
    const bool on = sc4->GetPopupDialogsEnabled();  // slot 23
    Drop(sc4);
    return on;
}

// One LTEXT through SC4LoadString's lookup (data-formats §6.2): (0x2026960B,
// group + current language ID, instance), then, when that is missing and the
// language ID is not 0, (0x2026960B, group, instance); U+2022 becomes U+200B
// (§6.1). [spec question NG45] neighbors.md §6.2 names the keys but not the
// loader; SC4LoadString's rule is the literal reading. The resource manager's
// GetPrivateResource (slot 4) asked for cIGZString, as the crime glue does.
// False (`out` untouched) when neither key loads.
bool LoadLtext(uint32_t group, uint32_t instance, std::string& out) {
    cIGZPersistResourceManagerPtr rm;
    if (!rm) return false;
    cIGZLanguageManagerPtr lm;
    const uint32_t language = lm ? lm->GetCurrentLanguage() : 0;
    auto load = [&](uint32_t g) -> bool {
        const cGZPersistResourceKey key(0x2026960B, g, instance);
        cIGZString* s = nullptr;
        const bool ok = rm->GetPrivateResource(key, GZIID_cIGZString, reinterpret_cast<void**>(&s), 0, nullptr);
        if (s == nullptr) return false;
        FC4_ACQUIRED(s, "GetPrivateResource");
        if (ok) out.assign(s->ToChar() != nullptr ? s->ToChar() : "", s->ToChar() != nullptr ? s->Strlen() : 0);
        FC4_RELEASE_CREATED(s);
        return ok;
    };
    if (!load(group + language) && !(language != 0 && load(group))) return false;
    for (size_t at = out.find("\xE2\x80\xA2"); at != std::string::npos; at = out.find("\xE2\x80\xA2", at + 3))
        out[at + 2] = '\x8B';  // U+2022 -> U+200B
    return true;
}
}  // namespace

namespace fc4 {
void NeighborsTestSetApp(cISC4App* app) { g_testApp = app; }
void NeighborsTestSetConfirm(NeighborsConfirmFn fn) { g_testConfirm = fn; }

// The confirmation text (neighbors.md §6.2): `sprintf(text, name)`.
// [choice] Only the first `%s` takes the name and `%%` gives `%`; any other
// `%` sequence is copied as it is (the shipped template is not specified
// beyond its one string argument; spec question NG45).
std::string NeighborsFormatConfirmText(const std::string& format, const std::string& name) {
    std::string r;
    bool used = false;
    for (size_t i = 0; i < format.size(); ++i) {
        if (format[i] == '%' && i + 1 < format.size()) {
            if (format[i + 1] == '%') {
                r += '%';
                ++i;
                continue;
            }
            if (format[i + 1] == 's' && !used) {
                r += name;
                used = true;
                ++i;
                continue;
            }
        }
        r += format[i];
    }
    return r;
}

bool NeighborsTestCopyDeal(cISC4NeighborDeal* src, cISC4NeighborDeal** out) {
    if (out == nullptr) return false;
    *out = nullptr;
    fc4_neighbor_deal_data data{};
    if (!ReadNeighborDeal(src, &data)) return false;
    Section sec;
    if (!sec.ok()) return false;
    fc4_neighbor_deal* h = fc4_neighbor_deal_create();
    if (h == nullptr) return false;
    fc4_neighbor_deal_copy(h, &data);
    auto* d = new cFC4NeighborDeal(h);
    d->AddRef();  // the caller's
    *out = d->Primary();
    return true;
}
}  // namespace fc4

// Slot 5 (§6.2; NG14, NG35, NG36, §15.4): the core answers 0 (exists: false)
// or 1 (ask the player). The original then calls
// nSC4UI::GenericConfirmationDialog (0x0078DFF0, cdecl (title, text,
// yesLabel, noLabel), labels null for Yes / No) and returns its answer: true
// only for Yes. With popups disabled, or no app (NG36), that dialog returns
// false without asking, so the request is refused. NG35: with popups on, the
// dialog is called at its build-gated address (compat::Direct) with the §6.2
// title and text. [choice] An executable the compat table does not identify
// has no dialog: the request is then confirmed without asking (logged once;
// the temporary deviation NG14 allows). The core's lock is not held across
// the modal dialog (the game's second thread calls our getters meanwhile).
bool cFC4NeighborsSimulator::RequestCreateConnection(ConnectionType type, int32_t x, int32_t z, bool) {
    uint64_t neighbour = 0;
    {
        Section sec;
        if (!sec.ok()) return false;
        const int32_t r =
            fc4_neighbors_request_create_connection(core_, static_cast<uint32_t>(type), x, z, &neighbour);
        if (r == 0) return false;
    }
    if (!PopupDialogsEnabled()) {
        if (!g_noteDeclined) {
            g_noteDeclined = true;
            fc4::Log("neighbours: RequestCreateConnection refused: popup dialogs are disabled (or no app), so the "
                     "confirmation dialog answers no (NG14, NG36; first: %s at (%d, %d))",
                     TypeName(static_cast<uint32_t>(type)), x, z);
        }
        return false;
    }
    fc4::NeighborsConfirmFn confirm = g_testConfirm;
    if (confirm == nullptr) {
        confirm = reinterpret_cast<fc4::NeighborsConfirmFn>(
            fc4::compat::DirectCall(fc4::compat::Direct::GenericConfirmationDialog));
    }
    if (confirm == nullptr) {
        if (!g_noteNoDialog) {
            g_noteNoDialog = true;
            fc4::Log("neighbours: RequestCreateConnection confirms without the yes/no dialog: "
                     "GenericConfirmationDialog is not available (first: %s at (%d, %d), neighbour %p)",
                     TypeName(static_cast<uint32_t>(type)), x, z, static_cast<void*>(FromToken(neighbour)));
        }
        return true;
    }
    // §6.2: title LTEXT 0x6A231EAA/0xAA8E151B; text sprintf(LTEXT 0x0A8E152D,
    // name), the name the established neighbour's (regional city slot 33) or
    // LTEXT 0x0A8E15F9. [spec question NG45] The two instance-only keys are
    // taken in the title's group 0x6A231EAA (literal reading). A missing
    // LTEXT gives the empty text [choice].
    constexpr uint32_t kGroup = 0x6A231EAA;
    std::string title, format, name;
    LoadLtext(kGroup, 0xAA8E151B, title);
    LoadLtext(kGroup, 0x0A8E152D, format);
    cFC4ISC4RegionalCity* rc = RcOf(neighbour);
    if (rc != nullptr) {
        cRZBaseString n;
        rc->GetCityName(n);  // slot 33
        if (n.ToChar() != nullptr) name.assign(n.ToChar(), n.Strlen());
    } else {
        LoadLtext(kGroup, 0x0A8E15F9, name);
    }
    cRZBaseString titleStr(title.c_str(), static_cast<uint32_t>(title.size()));
    const std::string text = fc4::NeighborsFormatConfirmText(format, name);
    cRZBaseString textStr(text.c_str(), static_cast<uint32_t>(text.size()));
    const bool yes = confirm(&titleStr, &textStr, nullptr, nullptr);
    if (!g_noteAsked) {
        g_noteAsked = true;
        fc4::Log("neighbours: RequestCreateConnection asked the player (NG35; first: %s at (%d, %d), answer %s)",
                 TypeName(static_cast<uint32_t>(type)), x, z, yes ? "yes" : "no");
    }
    return yes;
}

// Slot 6 [choice]: the new connection (borrowed; the list holds it) or null.
// NG10 (§15.4): the original's value is not meaningful and every caller
// ignores it, so any value works.
cISC4NeighborConnection* cFC4NeighborsSimulator::CreateConnection(ConnectionType type, int32_t x, int32_t z,
                                                                  bool flag) {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("CreateConnection");
        return nullptr;
    }
    return ConnOf(fc4_neighbors_create_connection(core_, static_cast<uint32_t>(type), x, z, flag));
}

void cFC4NeighborsSimulator::DeleteConnection(ConnectionType type, int32_t x, int32_t z) {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("DeleteConnection");
        return;
    }
    fc4_neighbors_delete_connection(core_, static_cast<uint32_t>(type), x, z);
}

void cFC4NeighborsSimulator::DeleteOccupantConnection(cISC4Occupant* occupant) {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("DeleteOccupantConnection");
        return;
    }
    fc4_neighbors_delete_occupant_connection(core_, TokenOf(occupant));
}

void cFC4NeighborsSimulator::DeleteAllConnectionsOfType(ConnectionType type) {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("DeleteAllConnectionsOfType");
        return;
    }
    fc4_neighbors_delete_all_connections_of_type(core_, static_cast<uint32_t>(type));
}

bool cFC4NeighborsSimulator::HasConnection(ConnectionType type, int32_t x, int32_t z) const {
    Section sec(false);
    return sec.ok() && fc4_neighbors_has_connection(core_, static_cast<uint32_t>(type), x, z);
}

cISC4NeighborConnection* cFC4NeighborsSimulator::FindConnection(ConnectionType type, int32_t x, int32_t z) {
    Section sec(false);
    return sec.ok() ? ConnOf(fc4_neighbors_find_connection(core_, static_cast<uint32_t>(type), x, z)) : nullptr;
}

cISC4RegionalCity* cFC4NeighborsSimulator::FindCityOnOtherSideOfConnection(int32_t x, int32_t z) {
    Section sec(false);
    return sec.ok() ? CityOf(fc4_neighbors_find_city_on_other_side_of_connection(core_, x, z)) : nullptr;
}

// Slot 13: the argument's type and cell (through its interface).
cISC4NeighborConnection* cFC4NeighborsSimulator::FindConnectionOnOtherSideOf(cISC4NeighborConnection* connection) {
    if (connection == nullptr) return nullptr;
    fc4_neighbor_connection_state s{};
    fc4::ReadNeighborConnection(connection, &s);
    Section sec(false);
    if (!sec.ok()) return nullptr;
    return ConnOf(fc4_neighbors_find_connection_on_other_side(core_, s.conn_type, s.x, s.z, nullptr));
}

cISC4NeighborConnection* cFC4NeighborsSimulator::FindConnectionOnOtherSideAt(ConnectionType type, int32_t x,
                                                                             int32_t z) {
    Section sec(false);
    if (!sec.ok()) return nullptr;
    return ConnOf(fc4_neighbors_find_connection_on_other_side(core_, static_cast<uint32_t>(type), x, z, nullptr));
}

cISC4NeighborConnection* cFC4NeighborsSimulator::FindUnvisitedConnection(ConnectionType type,
                                                                         cISC4RegionalCity const* city) {
    Section sec(false);
    if (!sec.ok()) return nullptr;
    return ConnOf(fc4_neighbors_find_unvisited_connection(core_, static_cast<uint32_t>(type), TokenOf(city)));
}

// Slots 16-19: clear first, one reference each (allocators.md).
void cFC4NeighborsSimulator::GetAllConnections(SC4List<cISC4NeighborConnection*>& list) const {
    Section sec(false);
    std::vector<uint64_t> t;
    if (sec.ok()) fc4_neighbors_connections(core_, 0, 0, 0, &PushToken, &t);
    Fill(list, t, true);
}

void cFC4NeighborsSimulator::GetConnectionsOfType(ConnectionType type, SC4List<cISC4NeighborConnection*>& list) const {
    Section sec(false);
    std::vector<uint64_t> t;
    if (sec.ok()) fc4_neighbors_connections(core_, 1, static_cast<uint32_t>(type), 0, &PushToken, &t);
    Fill(list, t, true);
}

void cFC4NeighborsSimulator::GetConnectionsOfTypeToCity(ConnectionType type, cISC4RegionalCity const* city,
                                                        SC4List<cISC4NeighborConnection*>& list) const {
    Section sec(false);
    std::vector<uint64_t> t;
    if (sec.ok()) fc4_neighbors_connections(core_, 2, static_cast<uint32_t>(type), TokenOf(city), &PushToken, &t);
    Fill(list, t, true);
}

void cFC4NeighborsSimulator::GetConnectionsToCity(cISC4RegionalCity const* city,
                                                  SC4List<cISC4NeighborConnection*>& list) {
    Section sec(false);
    std::vector<uint64_t> t;
    if (sec.ok()) fc4_neighbors_connections(core_, 3, 0, TokenOf(city), &PushToken, &t);
    Fill(list, t, true);
}

// Slots 20-22: clear first, one reference each.
void cFC4NeighborsSimulator::GetAdjacentCities(SC4List<cISC4RegionalCity*>& list, bool includeUnestablished) {
    Section sec(false);
    std::vector<uint64_t> t;
    if (sec.ok()) fc4_neighbors_cities(core_, 0, includeUnestablished, &PushToken, &t);
    Fill(list, t, true);
}

void cFC4NeighborsSimulator::GetConnectedCities(SC4List<cISC4RegionalCity*>& list, bool includeUnestablished) {
    Section sec(false);
    std::vector<uint64_t> t;
    if (sec.ok()) fc4_neighbors_cities(core_, 1, includeUnestablished, &PushToken, &t);
    Fill(list, t, true);
}

void cFC4NeighborsSimulator::GetAllConnectedCities(SC4List<cISC4RegionalCity*>& list, bool includeUnestablished) {
    Section sec(false);
    std::vector<uint64_t> t;
    if (sec.ok()) fc4_neighbors_cities(core_, 2, includeUnestablished, &PushToken, &t);
    Fill(list, t, true);
}

cISC4NeighborConnection* cFC4NeighborsSimulator::FindConnectionSuitableForDeal(int32_t dealType,
                                                                               cISC4RegionalCity const* city) {
    Section sec(false);
    if (!sec.ok()) return nullptr;
    return ConnOf(fc4_neighbors_find_connection_suitable_for_deal(core_, static_cast<uint32_t>(dealType),
                                                                  TokenOf(city)));
}

// ---- deals (slots 24-37) --------------------------------------------------------------------------

// Slot 24 (§7.1): `out` gets the deal with one reference when a suitable
// connection was found.
bool cFC4NeighborsSimulator::ProposeDeal(int32_t dealType, cISC4RegionalCity* otherCity, cISC4NeighborDeal** outDeal) {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("ProposeDeal");
        return false;
    }
    const uint64_t t = fc4_neighbors_propose_deal(core_, static_cast<uint32_t>(dealType), TokenOf(otherCity));
    cISC4NeighborDeal* d = DealOf(t);
    if (d == nullptr) return false;
    if (outDeal != nullptr) {
        FC4_ADDREF_FOR_CALLER(d);
        *outDeal = d;
    }
    return true;
}

// Slot 25: clear first, one reference each; true.
bool cFC4NeighborsSimulator::ProposeDeals(SC4List<cISC4NeighborDeal*>& deals, bool flag) {
    Section sec;
    std::vector<uint64_t> t;
    if (sec.ok()) {
        fc4_neighbors_propose_deals(core_, flag, &PushToken, &t);
    } else {
        NoteDropped("ProposeDeals");
    }
    Fill(deals, t, true);
    return true;
}

// Slot 26 (§6.2): Silent off, append, push both; true. A deal of ours is
// added itself. A foreign deal (a caller's object, not ours) is kept too
// (§15.4 NG4, the spec's recommendation): the simulator takes one reference
// to that object, clears its Silent flag through its interface and simulates
// it, so the caller's later calls act on the listed deal; the reference is
// released when the core lets the deal go (ReleaseDeal).
bool cFC4NeighborsSimulator::AddDeal(cISC4NeighborDeal* deal) {
    if (deal == nullptr) return false;
    cFC4NeighborDeal* ours = OurDeal(deal);
    Section sec;
    if (!sec.ok()) {
        NoteDropped("AddDeal");
        return false;
    }
    if (ours != nullptr) {
        fc4_neighbors_add_deal(core_, ours->Handle());
        return true;
    }
    FC4_ADDREF(deal);  // the core's reference (NG4)
    if (!fc4_neighbors_add_foreign_deal(core_, TokenOf(deal))) {
        // Not listed (no core): the reference is still ours to give back.
        // NG43 (neighbors.md §15.4): the original always lists the deal and
        // returns true; not listing it is an accepted, logged deviation
        // (mod-only: only another plugin's deal reaches this).
        FC4_RELEASE(deal);
        if (!g_noteDealNotListed) {
            g_noteDealNotListed = true;
            fc4::Log("neighbours: AddDeal could not list a deal that is not ours (no simulator core); it returns "
                     "true and keeps no reference (NG43; logged once)");
        }
    }
    return true;
}

void cFC4NeighborsSimulator::SetUIDisableFlag(bool flag) {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("SetUIDisableFlag");
        return;
    }
    fc4_neighbors_set_ui_disable_flag(core_, flag);
}

// Slot 28: list assignment (allocators.md): existing nodes take the new
// elements, surplus nodes go, missing ones are appended; one reference each.
void cFC4NeighborsSimulator::FindDealsAll(SC4List<cISC4NeighborDeal*>& deals) {
    Section sec(false);
    std::vector<uint64_t> tokens;
    if (sec.ok()) fc4_neighbors_find_deals(core_, 0, 0, 0, &PushToken, &tokens);
    size_t i = 0;
    auto it = deals.begin();
    for (; it != deals.end() && i < tokens.size(); ++it, ++i) {
        cISC4NeighborDeal* p = DealOf(tokens[i]);
        if (p != nullptr) FC4_ADDREF_FOR_CALLER(p);
        cISC4NeighborDeal* old = *it;
        *it = p;
        if (old != nullptr) {
            // The node's reference passes to us with the node (as the list's
            // own assignment releases the element it replaces).
            FC4_ACQUIRED(old, "the caller list's replaced element");
            FC4_RELEASE(old);
        }
    }
    while (it != deals.end()) it = deals.erase(it);
    for (; i < tokens.size(); ++i) {
        cISC4NeighborDeal* p = DealOf(tokens[i]);
        if (p == nullptr) continue;
        FC4_ADDREF_FOR_CALLER(p);
        deals.push_back(p);
    }
}

void cFC4NeighborsSimulator::FindDealsWithCity(int32_t dealType, cISC4RegionalCity* city,
                                               SC4List<cISC4NeighborDeal*>& deals) {
    Section sec(false);
    std::vector<uint64_t> t;
    if (sec.ok()) fc4_neighbors_find_deals(core_, 2, static_cast<uint32_t>(dealType), TokenOf(city), &PushToken, &t);
    Fill(deals, t, true);
}

void cFC4NeighborsSimulator::FindDealsOfType(int32_t dealType, SC4List<cISC4NeighborDeal*>& deals) {
    Section sec(false);
    std::vector<uint64_t> t;
    if (sec.ok()) fc4_neighbors_find_deals(core_, 1, static_cast<uint32_t>(dealType), 0, &PushToken, &t);
    Fill(deals, t, true);
}

void cFC4NeighborsSimulator::MarkDealsDirty() {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("MarkDealsDirty");
        return;
    }
    fc4_neighbors_mark_deals_dirty(core_);
}

// Slots 32/33 (§10.1; NG11, §15.4): the full signed 64-bit sum in EDX:EAX.
// Every game caller (finance advisor, budget simulator, budget windows) reads
// both halves; a 32-bit return would leave garbage in EDX for them.
int64_t cFC4NeighborsSimulator::GetTotalMonthlyNeighborDealIncome() {
    Section sec(false);
    return sec.ok() ? fc4_neighbors_total_income(core_) : snap_.income.load();
}

int64_t cFC4NeighborsSimulator::GetTotalMonthlyNeighborDealExpense() {
    Section sec(false);
    return sec.ok() ? fc4_neighbors_total_expense(core_) : snap_.expense.load();
}

// Slot 34 (§8.2): keyed on the city object.
void cFC4NeighborsSimulator::UpdateCommodityStats(cISC4City* city) {
    Section sec;
    if (!sec.ok()) {
        NoteDropped("UpdateCommodityStats");
        return;
    }
    fc4_neighbors_update_commodity_stats(core_, TokenOf(city));
}

double cFC4NeighborsSimulator::GetDealCost(int32_t dealType, double amount, cISC4RegionalCity const* city) {
    Section sec(false);
    return sec.ok() ? fc4_neighbors_deal_cost(core_, static_cast<uint32_t>(dealType), amount, TokenOf(city)) : 0.0;
}

double cFC4NeighborsSimulator::GetDealMaxAmount(int32_t dealType, cISC4RegionalCity const* city) {
    Section sec(false);
    return sec.ok() ? fc4_neighbors_deal_max_amount(core_, static_cast<uint32_t>(dealType), TokenOf(city)) : 0.0;
}

// Slot 37: the exemplar Init loaded (no AddRef), null before Init or without it.
cISCResExemplar* cFC4NeighborsSimulator::GetTuningExemplar() {
    return ctx_.tuning ? ctx_.tuning->Resource() : nullptr;
}

// ---- messages (§11.1) -----------------------------------------------------------------------------

bool cFC4NeighborsSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("neighbours DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    Section sec;
    if (!sec.ok()) {
        NoteDropped("DoMessage");
        // [choice] (NG33, glue) a message from another thread that cannot get
        // the lock is dropped; the original never drops one and returns true.
        return true;
    }
    const uint32_t type = pMessage->GetType();
    fc4_neighbors_message m{};
    m.msg_type = type;
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    if (s != nullptr) {
        m.data1 = static_cast<uint32_t>(s->GetData1());
        m.data2 = static_cast<uint32_t>(s->GetData2());
        m.data3 = static_cast<uint32_t>(s->GetData3());
        if (type == kMsgPreDemolishNetworkOccupant || type == kMsgRemoveOccupant) {
            m.occupant = TokenOf(static_cast<cISC4Occupant*>(s->GetVoid1()));
        }
    }
    fc4_neighbors_do_message(core_, &m);
    Drop(s);
    if (type == kMsgSimNewMonth) {
        ++months_;
        if (g_depth == 1) {
            uint32_t today = fc4::NeighborToday(ctx_.city ? ctx_.city : fc4::NeighborAppCity());
            if (today == 0) today = m.data1;
            LogMonth(today);
        }
    }
    return true;  // always
}

// ---- persistence (§13) ----------------------------------------------------------------------------

bool cFC4NeighborsSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("NeighborsSimulator::Write");
    Section sec;
    if (!sec.ok()) return false;
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_neighbors_write(core_, a.Get());
    fc4::Log("neighbours Write -> %d", ok ? 1 : 0);
    return ok;
}

// Runs before Init: the core reads the references only (version 2).
bool cFC4NeighborsSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("NeighborsSimulator::Read");
    Section sec;
    if (!sec.ok()) return false;
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_neighbors_read(core_, a.Get());
    fc4::Log("neighbours Read -> %d", ok ? 1 : 0);
    LogState("Read");
    return ok;
}

uint32_t cFC4NeighborsSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4NeighborsSimulator; }

bool fc4::NeighborsCollectReleasedElements() {
    Section sec;
    return sec.ok() && IsMainThread();
}
