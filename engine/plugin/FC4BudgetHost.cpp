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

// The fc4_budget_host callbacks: everything the budget core asks of the game
// (docs/specs/budget.md: city queries §4.1/§5, lots §5.2, history §5.1/§5.7,
// LTEXT names §2/§7/§8, holder properties §2, disasters §4.3, objects §1).
// One table serves the budget simulator, its departments and line items, so
// objects the game's class factory creates while loading a save work too.
//
// Bisection tooling (FC4Config.h): each behaviour that touches game objects
// has a switch; when it is off the callback is a no-op with a neutral result.
// References on game objects go through the audit (FC4RefAudit.h), and small
// buffers the game writes into are canary-guarded in checked builds
// (FC4Checked.h).

#include "FC4Checked.h"
#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Properties.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Budget.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "StringResourceKey.h"
#include "StringResourceManager.h"
#include "cGZPersistResourceKey.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessageServer2.h"
#include "cIGZString.h"
#include "cISC4App.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4DisasterLayer.h"
#include "cISC4DisasterManager.h"
#include "cISC4HistoryWarehouse.h"
#include "cISC4LandValueSimulator.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4NeighborsSimulator.h"
#include "cISC4Occupant.h"
#include "cISC4OrdinanceSimulator.h"
#include "cISC4Simulator.h"
#include "cISCPropertyHolder.h"
#include "cRZAutoRefCount.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <unordered_map>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kPropBuildingValue = 0xE91A0B5F;  // spec §5.2

cISC4City* g_city = nullptr;

// Game objects the budget core holds references on through add_ref (holder_refs).
std::unordered_map<cIGZUnknown*, long> g_coreRefs;

cISC4City* City() {
    if (g_city != nullptr) return g_city;
    // Before the budget's Init (e.g. objects created while a save loads).
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    FC4_OUT(cISC4App*, sc4, "cISC4App* (QueryInterface out pointer)");
    const bool ok = app != nullptr && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(sc4.ptr()));
    sc4.Check();
    if (!ok || sc4.get() == nullptr) return nullptr;
    FC4_ACQUIRED(sc4.get(), "QueryInterface");
    cISC4City* city = sc4.get()->GetCity();
    FC4_RELEASE(sc4.get());
    return city;
}

cISCPropertyHolder* Holder(uint64_t token) { return reinterpret_cast<cISCPropertyHolder*>(FromToken(token)); }

template <typename T>
int32_t CopyOut(bool ok, const std::vector<T>& v, T* out, size_t capacity) {
    if (!ok) return -1;
    for (size_t i = 0; i < v.size() && i < capacity; ++i) out[i] = v[i];
    return static_cast<int32_t>(v.size());
}

void Sink(const cIGZString* s, fc4_bytes_sink sink, void* ctx) {
    if (s == nullptr || sink == nullptr) return;
    const char* p = s->ToChar();
    if (p != nullptr) sink(ctx, reinterpret_cast<const uint8_t*>(p), s->Strlen());
}

// ---- callbacks -----------------------------------------------------------------------------------

void Post(void*, const fc4_budget_message* m) {
    if (m == nullptr) return;
    if (!On(Sw::MessagePosting)) {
        NoteSkipped(Sw::MessagePosting, "budget message post");
        return;
    }
    cIGZMessageServer2Ptr ms;
    if (!ms) {
        FC4_LOGV("budget post 0x%08X skipped: no message server", m->type);
        return;
    }
    cFC4Message2Standard* msg = FC4MakeMessage(m->type, static_cast<intptr_t>(m->data1),
                                               static_cast<intptr_t>(m->data2), static_cast<intptr_t>(m->data3));
    if (m->void1 != 0) msg->SetVoid1(FromToken(m->void1));
    if (m->unknown != 0) msg->SetIGZUnknown(FromToken(m->unknown));
    const bool ok = ms->MessagePost(msg, false);
    msg->Release();
    FC4_LOGV("budget post 0x%08X d1=%lld d2=%lld d3=%lld -> %s", m->type, static_cast<long long>(m->data1),
             static_cast<long long>(m->data2), static_cast<long long>(m->data3), ok ? "true" : "FALSE");
}

void ShowBudgetWindow(void*, uint32_t panel) {
    // [choice] The UI is outside the simulation (spec §6).
    Log("budget: ShowBudgetWindow(panel %u) requested; the budget UI is not replaced", panel);
}

uint32_t Difficulty(void*) {
    cISC4City* c = City();
    return c ? static_cast<uint32_t>(c->GetDifficultyLevel()) : 0;
}

void SimDate(void*, int32_t* year, int32_t* month) {
    cISC4City* c = City();
    cFC4ISC4Simulator* s = c ? fc4::AsGame(c->GetSimulator()) : nullptr;
    if (s == nullptr) return;
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
    *year = static_cast<int32_t>(y.get());
    *month = static_cast<int32_t>(m.get());
}

bool Established(void*) {
    cISC4City* c = City();
    return c != nullptr && c->GetEstablished();
}

void CitySize(void*, float* x, float* z) {
    if (cISC4City* c = City()) {
        *x = c->SizeX();
        *z = c->SizeZ();
    }
}

int64_t OrdinanceIncome(void*) {
    cISC4City* c = City();
    cISC4OrdinanceSimulator* o = c ? c->GetOrdinanceSimulator() : nullptr;
    return o ? o->GetOrdinanceMonthlyIncome() : 0;
}

int64_t OrdinanceExpense(void*) {
    cISC4City* c = City();
    cISC4OrdinanceSimulator* o = c ? c->GetOrdinanceSimulator() : nullptr;
    return o ? o->GetOrdinanceMonthlyExpense() : 0;
}

int64_t NeighborIncome(void*) {
    cISC4City* c = City();
    cFC4ISC4NeighborsSimulator* n = c ? fc4::AsGame(c->GetNeighborsSimulator()) : nullptr;  // 32-bit returns (docs/abi)
    return n ? n->GetTotalMonthlyNeighborDealIncome() : 0;
}

int64_t NeighborExpense(void*) {
    cISC4City* c = City();
    cFC4ISC4NeighborsSimulator* n = c ? fc4::AsGame(c->GetNeighborsSimulator()) : nullptr;  // 32-bit returns (docs/abi)
    return n ? n->GetTotalMonthlyNeighborDealExpense() : 0;
}

void ForEachLot(void*, fc4_lot_visitor visit, void* ctx) {
    if (!On(Sw::LotScan)) {
        NoteSkipped(Sw::LotScan, "ForEachLot (no lots: property tax 0)");
        return;
    }
    cISC4City* c = City();
    cFC4ISC4LotManager* lm = c ? AsGame(c->GetLotManager()) : nullptr;
    if (lm == nullptr || visit == nullptr) return;
    cISC4LandValueSimulator* lv = c->GetLandValueSimulator();
    const int32_t maxX = static_cast<int32_t>(c->CellCountX()) - 1;
    const int32_t maxZ = static_cast<int32_t>(c->CellCountZ()) - 1;
    // [choice] Every lot of the city through one rectangle query; the game
    // appends one reference per lot (budget Errata round 2, item 5), which we
    // release below (audited) before the list frees its nodes.
    FC4_OUT(SC4List<cISC4Lot*>, lots, "SC4List<cISC4Lot*> (GetLotsInCellRect)");
    lm->GetLotsInCellRect(lots.get(), 0, 0, maxX, maxZ, false);  // slot 7
    lots.Check();
    for (cISC4Lot*& sdkLot : lots.get()) {
        if (sdkLot == nullptr) continue;
        FC4_ACQUIRED(sdkLot, "GetLotsInCellRect list entry");
        cFC4ISC4Lot* lot = AsGame(sdkLot);
        fc4_lot_info info{};
        cISC4BuildingOccupant* b = lot->GetBuilding();
        info.has_building = b != nullptr;
        info.livable = lot->IsInLivableState();
        info.lot_wealth = lot->GetOccupantWealth();
        {
            FC4_OUT(uint8_t, sx, "uint8 sizeX (cISC4Lot::GetSize)");
            FC4_OUT(uint8_t, sz, "uint8 sizeZ (cISC4Lot::GetSize)");
            sx.get() = 1;
            sz.get() = 1;
            lot->GetSize(sx.get(), sz.get());
            sx.Check();
            sz.Check();
            info.size_x = sx.get();
            info.size_z = sz.get();
        }
        if (lv != nullptr) {
            FC4_OUT(int32_t, x, "int32 x (cISC4Lot::GetLocation)");
            FC4_OUT(int32_t, z, "int32 z (cISC4Lot::GetLocation)");
            const bool located = lot->GetLocation(x.get(), z.get());
            x.Check();
            z.Check();
            if (located) info.land_value = lv->GetLandValue(x.get(), z.get());
        }
        if (b != nullptr) {
            const cISC4BuildingOccupant::BuildingProfile& p = b->GetBuildingProfile();
            info.purpose = static_cast<uint32_t>(p.purpose);
            info.building_wealth = static_cast<uint32_t>(p.wealth);
            FC4_OUT(cISCPropertyHolder*, h, "cISCPropertyHolder* (QueryInterface out pointer)");
            const bool has = b->QueryInterface(GZIID_cISCPropertyHolder, reinterpret_cast<void**>(h.ptr()));
            h.Check();
            if (has && h.get() != nullptr) {
                FC4_ACQUIRED(h.get(), "QueryInterface");
                std::vector<int64_t> v;
                if (props::GetI64s(h.get(), kPropBuildingValue, v) && !v.empty()) info.building_value = v[0];
                FC4_RELEASE(h.get());
            }
        }
        visit(ctx, &info);
    }
    for (cISC4Lot*& sdkLot : lots.get()) {
        if (sdkLot == nullptr) continue;
        FC4_RELEASE(sdkLot);
        sdkLot = nullptr;  // the list destructor then only frees its nodes
    }
}

void RecordHistory(void*, uint32_t series, int32_t value) {
    if (!On(Sw::History)) {
        NoteSkipped(Sw::History, "RecordHistoryData");
        return;
    }
    cISC4City* c = City();
    if (cISC4HistoryWarehouse* h = c ? c->GetHistoryWarehouse() : nullptr) h->RecordHistoryData(series, value);
}

void SetHistoryQueueLength(void*, uint32_t series, uint32_t length) {
    if (!On(Sw::History)) {
        NoteSkipped(Sw::History, "SetHistoryDataQueueLength");
        return;
    }
    cISC4City* c = City();
    if (cISC4HistoryWarehouse* h = c ? c->GetHistoryWarehouse() : nullptr) h->SetHistoryDataQueueLength(series, length);
}

bool TryStartDisaster(void*, uint32_t id, float x, float z) {
    if (!On(Sw::Disaster)) {
        NoteSkipped(Sw::Disaster, "AttemptDisasterStart (not started)");
        return false;
    }
    cISC4City* c = City();
    cISC4DisasterLayer* layer = c ? c->GetDisasterLayer() : nullptr;
    cISC4DisasterManager* dm = layer ? layer->GetDisasterManager(id) : nullptr;
    if (dm == nullptr) return false;
    // Budget §4.3: integer WORLD coordinates, truncated toward zero (the
    // earthquake converts to cells itself); compiled slot 6, foundations §17.8.
    const bool started = dm->AttemptDisasterStart(static_cast<int32_t>(x), static_cast<int32_t>(z)) != nullptr;
    Log("budget: disaster 0x%08X at (%.0f, %.0f): %s", id, x, z, started ? "started" : "not started");
    return started;
}

uint32_t RngSeed(void*) { return GetTickCount(); }  // foundations §6.2: wall clock

bool Ltext(void*, uint32_t group, uint32_t instance, fc4_bytes_sink sink, void* ctx) {
    if (!On(Sw::Ltext)) {
        NoteSkipped(Sw::Ltext, "LTEXT lookup (not found)");
        return false;
    }
    // The SDK helper owns the reference (cRZAutoRefCount; not audited).
    cRZAutoRefCount<cIGZString> s;
    if (!StringResourceManager::GetLocalizedString(StringResourceKey(group, instance), s)) return false;
    Sink(s, sink, ctx);
    return true;
}

bool BuildingTypeName(void*, uint32_t type, fc4_bytes_sink sink, void* ctx) {
    if (!On(Sw::BuildingName)) {
        NoteSkipped(Sw::BuildingName, "GetBuildingName (empty name)");
        return true;
    }
    // Budget Errata round 2, item 2: the building-development simulator's
    // user-visible name of the type (compiled slot 38). The string belongs to
    // the simulator: copied, not released. Never-loaded types give "".
    cISC4City* c = City();
    cISC4BuildingDevelopmentSimulator* bd = c ? c->GetBuildingDevelopmentSimulator() : nullptr;
    if (bd == nullptr) return false;
    Sink(bd->GetBuildingName(type), sink, ctx);
    return true;
}

// Property reads and writes: the switches live in FC4Properties.cpp.
int32_t GetU32s(void*, uint64_t h, uint32_t p, uint32_t* out, size_t cap) {
    std::vector<uint32_t> v;
    return CopyOut(props::GetU32s(Holder(h), p, v), v, out, cap);
}

int32_t GetI64s(void*, uint64_t h, uint32_t p, int64_t* out, size_t cap) {
    std::vector<int64_t> v;
    return CopyOut(props::GetI64s(Holder(h), p, v), v, out, cap);
}

int32_t GetF32s(void*, uint64_t h, uint32_t p, float* out, size_t cap) {
    std::vector<float> v;
    return CopyOut(props::GetF32s(Holder(h), p, v), v, out, cap);
}

bool SetU32s(void*, uint64_t h, uint32_t p, const uint32_t* v, size_t n) {
    return props::SetU32s(Holder(h), p, std::vector<uint32_t>(v, v + n));
}

bool SetI64s(void*, uint64_t h, uint32_t p, const int64_t* v, size_t n) {
    return props::SetI64s(Holder(h), p, std::vector<int64_t>(v, v + n));
}

bool SetF32s(void*, uint64_t h, uint32_t p, const float* v, size_t n) {
    return props::SetF32s(Holder(h), p, std::vector<float>(v, v + n));
}

bool CopyProperty(void*, uint64_t src, uint64_t dst, uint32_t p) { return props::Copy(Holder(src), Holder(dst), p); }

uint64_t OccupantOf(void*, uint64_t h) {
    cIGZUnknown* u = FromToken(h);
    if (u == nullptr) return 0;
    FC4_OUT(void*, occ, "cISC4Occupant* (QueryInterface out pointer)");
    const bool ok = u->QueryInterface(GZIID_cISC4Occupant, occ.ptr());
    occ.Check();
    if (!ok || occ.get() == nullptr) return 0;
    cISC4Occupant* o = static_cast<cISC4Occupant*>(occ.get());
    FC4_ACQUIRED(o, "QueryInterface");
    const uint64_t t = TokenOf(static_cast<cIGZUnknown*>(o));
    FC4_RELEASE(o);  // borrowed: the holder keeps the occupant alive
    return t;
}

uint32_t ClassOf(void*, uint64_t token) {
    cIGZUnknown* u = FromToken(token);
    if (u == nullptr) return 0;
    FC4_OUT(cIGZSerializable*, s, "cIGZSerializable* (QueryInterface out pointer)");
    const bool ok = u->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(s.ptr()));
    s.Check();
    if (!ok || s.get() == nullptr) return 0;
    FC4_ACQUIRED(s.get(), "QueryInterface");
    const uint32_t c = s.get()->GetGZCLSID();
    FC4_RELEASE(s.get());
    return c;
}

// The core's own references. Our objects always pass through. Game objects
// (locally funded holders, objects read from the save) obey holder_refs; a
// reference taken while the switch was on is always given back.
void AddRefToken(void*, uint64_t t) {
    cIGZUnknown* u = FromToken(t);
    if (u == nullptr) return;
    if (IsOurObject(u)) {
        u->AddRef();
        return;
    }
    if (!On(Sw::HolderRefs)) {
        NoteSkipped(Sw::HolderRefs, "AddRef of a game object for the budget core");
        return;
    }
    ++g_coreRefs[u];
    FC4_ADDREF(u);
}

void ReleaseToken(void*, uint64_t t) {
    cIGZUnknown* u = FromToken(t);
    if (u == nullptr) return;
    if (IsOurObject(u)) {
        u->Release();
        return;
    }
    auto it = g_coreRefs.find(u);
    if (it != g_coreRefs.end()) {
        if (--it->second == 0) g_coreRefs.erase(it);
    } else if (!On(Sw::HolderRefs)) {
        NoteSkipped(Sw::HolderRefs, "Release of a game object for the budget core");
        return;
    }
    FC4_RELEASE(u);
}

void* CreateObject(void*, uint32_t classId) {
    // [choice] Our own classes directly (the class registry would route these
    // CLSIDs to us anyway while the budget subsystem is enabled).
    if (classId == kFC4_CLSID_cSC4DepartmentBudget) {
        cFC4DepartmentBudget* d = new cFC4DepartmentBudget();
        d->AddRef();  // owned by the core
        return d->CoreHandle();
    }
    if (classId == kFC4_CLSID_cSC4LineItem) {
        cFC4LineItem* l = new cFC4LineItem();
        l->AddRef();
        return l->CoreHandle();
    }
    Log("budget: create_object(0x%08X) refused", classId);
    return nullptr;
}

fc4_budget_host MakeHost() {
    fc4_budget_host h{};
    h.struct_size = sizeof(h);
    h.user = nullptr;
    h.post = &Post;
    h.show_budget_window = &ShowBudgetWindow;
    h.difficulty = &Difficulty;
    h.sim_date = &SimDate;
    h.city_established = &Established;
    h.city_size = &CitySize;
    h.ordinance_income = &OrdinanceIncome;
    h.ordinance_expense = &OrdinanceExpense;
    h.neighbor_income = &NeighborIncome;
    h.neighbor_expense = &NeighborExpense;
    h.for_each_lot = &ForEachLot;
    h.record_history = &RecordHistory;
    h.set_history_queue_length = &SetHistoryQueueLength;
    h.try_start_disaster = &TryStartDisaster;
    h.rng_seed = &RngSeed;
    h.ltext = &Ltext;
    h.building_type_name = &BuildingTypeName;
    h.get_u32s = &GetU32s;
    h.get_i64s = &GetI64s;
    h.get_f32s = &GetF32s;
    h.set_u32s = &SetU32s;
    h.set_i64s = &SetI64s;
    h.set_f32s = &SetF32s;
    h.copy_property = &CopyProperty;
    h.occupant_of = &OccupantOf;
    h.class_of = &ClassOf;
    h.add_ref = &AddRefToken;
    h.release = &ReleaseToken;
    h.create_object = &CreateObject;
    return h;
}

}  // namespace

const fc4_budget_host* BudgetHost() {
    static const fc4_budget_host host = MakeHost();
    return &host;
}

void SetBudgetCity(cISC4City* city) { g_city = city; }

void LogBudgetCoreRefs(const char* why) {
    long total = 0;
    for (const auto& kv : g_coreRefs) total += kv.second;
    Log("budget core (%s): holds %ld reference(s) on %u game object(s) taken through add_ref", why, total,
        static_cast<unsigned>(g_coreRefs.size()));
}

}  // namespace fc4
