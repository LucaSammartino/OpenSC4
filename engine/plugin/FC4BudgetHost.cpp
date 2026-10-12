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

#include "FC4Interfaces.h"
#include "FC4Properties.h"
#include "FC4Log.h"
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

#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kPropBuildingValue = 0xE91A0B5F;  // spec §5.2

cISC4City* g_city = nullptr;

cISC4City* City() {
    if (g_city != nullptr) return g_city;
    // Before the budget's Init (e.g. objects created while a save loads).
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
    uint32_t y = 0, m = 0, d = 0, doy = 0, wd = 0;
    s->GetSimDateParts(&y, &m, &d, &doy, &wd);
    *year = static_cast<int32_t>(y);
    *month = static_cast<int32_t>(m);
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
    cISC4NeighborsSimulator* n = c ? c->GetNeighborsSimulator() : nullptr;
    return n ? n->GetTotalMonthlyNeighborDealIncome() : 0;
}

int64_t NeighborExpense(void*) {
    cISC4City* c = City();
    cISC4NeighborsSimulator* n = c ? c->GetNeighborsSimulator() : nullptr;
    return n ? n->GetTotalMonthlyNeighborDealExpense() : 0;
}

void ForEachLot(void*, fc4_lot_visitor visit, void* ctx) {
    cISC4City* c = City();
    cFC4ISC4LotManager* lm = c ? AsGame(c->GetLotManager()) : nullptr;
    if (lm == nullptr || visit == nullptr) return;
    cISC4LandValueSimulator* lv = c->GetLandValueSimulator();
    const int32_t maxX = static_cast<int32_t>(c->CellCountX()) - 1;
    const int32_t maxZ = static_cast<int32_t>(c->CellCountZ()) - 1;
    // [choice] Every lot of the city through one rectangle query; the list
    // holds a reference per lot and releases them when it goes away.
    SC4List<cISC4Lot*> lots;
    lm->GetLotsInCellRect(lots, 0, 0, maxX, maxZ, false);  // slot 7; budget Errata round 2, item 5
    for (cISC4Lot* sdkLot : lots) {
        if (sdkLot == nullptr) continue;
        cFC4ISC4Lot* lot = AsGame(sdkLot);
        fc4_lot_info info{};
        cISC4BuildingOccupant* b = lot->GetBuilding();
        info.has_building = b != nullptr;
        info.livable = lot->IsInLivableState();
        info.lot_wealth = lot->GetOccupantWealth();
        uint8_t sx = 1, sz = 1;
        lot->GetSize(sx, sz);
        info.size_x = sx;
        info.size_z = sz;
        int32_t x = 0, z = 0;
        if (lv != nullptr && lot->GetLocation(x, z)) info.land_value = lv->GetLandValue(x, z);
        if (b != nullptr) {
            const cISC4BuildingOccupant::BuildingProfile& p = b->GetBuildingProfile();
            info.purpose = static_cast<uint32_t>(p.purpose);
            info.building_wealth = static_cast<uint32_t>(p.wealth);
            cISCPropertyHolder* h = nullptr;
            if (b->QueryInterface(GZIID_cISCPropertyHolder, reinterpret_cast<void**>(&h)) && h != nullptr) {
                std::vector<int64_t> v;
                if (props::GetI64s(h, kPropBuildingValue, v) && !v.empty()) info.building_value = v[0];
                h->Release();
            }
        }
        visit(ctx, &info);
    }
}

void RecordHistory(void*, uint32_t series, int32_t value) {
    cISC4City* c = City();
    if (cISC4HistoryWarehouse* h = c ? c->GetHistoryWarehouse() : nullptr) h->RecordHistoryData(series, value);
}

void SetHistoryQueueLength(void*, uint32_t series, uint32_t length) {
    cISC4City* c = City();
    if (cISC4HistoryWarehouse* h = c ? c->GetHistoryWarehouse() : nullptr) h->SetHistoryDataQueueLength(series, length);
}

bool TryStartDisaster(void*, uint32_t id, float x, float z) {
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
    cRZAutoRefCount<cIGZString> s;
    if (!StringResourceManager::GetLocalizedString(StringResourceKey(group, instance), s)) return false;
    Sink(s, sink, ctx);
    return true;
}

bool BuildingTypeName(void*, uint32_t type, fc4_bytes_sink sink, void* ctx) {
    // Budget Errata round 2, item 2: the building-development simulator's
    // user-visible name of the type (compiled slot 38). The string belongs to
    // the simulator: copied, not released. Never-loaded types give "".
    cISC4City* c = City();
    cISC4BuildingDevelopmentSimulator* bd = c ? c->GetBuildingDevelopmentSimulator() : nullptr;
    if (bd == nullptr) return false;
    Sink(bd->GetBuildingName(type), sink, ctx);
    return true;
}

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
    void* occ = nullptr;
    if (u == nullptr || !u->QueryInterface(GZIID_cISC4Occupant, &occ) || occ == nullptr) return 0;
    cISC4Occupant* o = static_cast<cISC4Occupant*>(occ);
    const uint64_t t = TokenOf(static_cast<cIGZUnknown*>(o));
    o->Release();  // borrowed: the holder keeps the occupant alive
    return t;
}

uint32_t ClassOf(void*, uint64_t token) {
    cIGZUnknown* u = FromToken(token);
    cIGZSerializable* s = nullptr;
    if (u == nullptr || !u->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&s)) || s == nullptr) {
        return 0;
    }
    const uint32_t c = s->GetGZCLSID();
    s->Release();
    return c;
}

void AddRefToken(void*, uint64_t t) {
    if (cIGZUnknown* u = FromToken(t)) u->AddRef();
}

void ReleaseToken(void*, uint64_t t) {
    if (cIGZUnknown* u = FromToken(t)) u->Release();
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

}  // namespace fc4
