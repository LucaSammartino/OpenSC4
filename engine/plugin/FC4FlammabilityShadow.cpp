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

#include "FC4FlammabilityShadow.h"

#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4Watchdog.h"
#include "FC4WriteCapture.h"
#include "fc4.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "SC4Point.h"
#include "SC4Rect.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4App.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4FlammabilitySimulator.h"
#include "cISC4LotManager.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4OrdinanceSimulator.h"
#include "cISC4PlumbingSimulator.h"
#include "cISC4SimGrid.h"
#include "cISC4WeatherSimulator.h"
#include "cISC4ZoneManager.h"
#include "cISCPropertyHolder.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector3.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstring>
#include <memory>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4Lot = 0xE818FCED;

// The original's six messages (flammability.md §4.1 step 3, §5).
constexpr uint32_t kMsgInsertOccupant = 0x99EF1142;
constexpr uint32_t kMsgRemoveOccupant = 0x99EF1143;
constexpr uint32_t kMsgChangeOccupant = 0x99EF1145;
constexpr uint32_t kMsgSeasonChange = 0x6716A4E5;
constexpr uint32_t kMsgOrdinanceUpdated = 0xC2C09554;
constexpr uint32_t kMsgConditionChanged = 0xE99B3D4D;
// §9: posted by EndOfCycle, void1 = IGZUnknown = the simulator.
constexpr uint32_t kMsgSimCycleComplete = 0x08F415FE;
constexpr uint32_t kMsgPostCityShutdown = 0x26D31EC3;
// The plumbing pass end (plumbing.md §6.7 step 4, after the grid swap of
// state 6): forwarded so the shadow polls the watered state (by content) at
// every pass end until the first day poll (flammability.md §6.5, FL5-FL7).
// Not one of the original's six.
constexpr uint32_t kMsgWaterMapChanged = 0x6946DC34;
// The original's day (§5: its one agent registration, level 4, by this name,
// §4.1 step 2): FlammabilityShadowDeliver hands it to the core after the
// original's delivery, as a day poll (§6.5, FL7/FL8).
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr const char* kAgentName = "cSC4FlammabilitySimulator";
constexpr uint32_t kMessages[] = {kMsgInsertOccupant,   kMsgRemoveOccupant,   kMsgChangeOccupant,
                                  kMsgSeasonChange,     kMsgOrdinanceUpdated, kMsgConditionChanged,
                                  kMsgSimCycleComplete, kMsgPostCityShutdown, kMsgWaterMapChanged};

constexpr uint16_t kRecordVersion = 2;  // §10
// The record's hotspot count is trusted up to this many (a tract each at most).
constexpr uint32_t kMaxHotspots = 1u << 20;
constexpr int32_t kMaxCount = 4096;

// The tuning exemplar (§8), as our simulator reads it.
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0xA8F41EE3;
// The three accepted occupant types (§6.1, §6.2).
constexpr uint32_t kOccupantTypes[] = {0x278128A0, 0x74758926, 0xA823821E};

constexpr long kDefaultCells = 4;
constexpr long kMaxCells = 64;

using GridU8 = cISC4SimGrid<uint8_t>;

// ---- settings and run state (main thread only) ---------------------------------------------------

bool g_enabled = false;
bool g_replaced = false;
long g_cells = kDefaultCells;
bool g_started = false;

bool g_loggedOurs = false;
bool g_loggedNone = false;
bool g_loggedNoRecord = false;

// The city the read-only host answers with (borrowed for one call into the
// core), and the game lists its occupant queries filled (held until that call
// returns: they hold one reference per occupant).
cISC4City* g_city = nullptr;
std::vector<std::shared_ptr<void>> g_held;

fc4_flammability_shadow* g_shadow = nullptr;
const void* g_shadowCity = nullptr;

WriteCapture g_capture;

void ReadSettings() {
    g_enabled = SettingInt("flammability_shadow", "FC4_FLAMMABILITY_SHADOW", 0) != 0;
    long n = SettingInt("flammability_shadow_cells", "FC4_FLAMMABILITY_SHADOW_CELLS", kDefaultCells);
    if (n <= 0) n = kDefaultCells;
    if (n > kMaxCells) n = kMaxCells;
    g_cells = n;
}

// ---- the read-only host (fc4_flammability_host, flammability.md §12) ------------------------------
//
// The reads our simulator makes, against the city's services (borrowed for
// the call; city getters do not AddRef). No grid is created through the game
// (the shadow's grids are its own), and the history, the fire, the random lot
// and the posts stay NULL. The FreeCity4.ini flammability_* switches gate our
// replacement's calls, not these.

cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cFC4ISC4Lot* LotOf(uint64_t t) { return t ? AsGame(reinterpret_cast<cISC4Lot*>(FromToken(t))) : nullptr; }

void CityCellCounts(void*, int32_t* x, int32_t* z) {
    if (x != nullptr) *x = g_city ? static_cast<int32_t>(g_city->CellCountX()) : 0;  // slots 93, 94
    if (z != nullptr) *z = g_city ? static_cast<int32_t>(g_city->CellCountZ()) : 0;
}

void PositionToCell(void*, float x, float z, int32_t* cx, int32_t* cz) {
    int a = 0, b = 0;
    if (g_city != nullptr) g_city->PositionToCell(x, z, a, b);  // slot 95
    if (cx != nullptr) *cx = a;
    if (cz != nullptr) *cz = b;
}

fc4_props* Tuning(void*) {
    props::Exemplar ex;
    if (!ex.Load(kTuningType, kTuningGroup, kTuningInstance)) return nullptr;  // the defaults
    return props::ToCoreAll(ex.Holder());
}

uint32_t OccupantType(void*, uint64_t occ) { return occ ? static_cast<uint32_t>(OccOf(occ)->GetType()) : 0; }

bool OccupantProperty(void*, uint64_t occ, uint32_t id, fc4_props* out) {
    if (occ == 0) return false;
    cISCPropertyHolder* h = OccOf(occ)->AsPropertyHolder();  // borrowed
    return h != nullptr && props::CopyToCore(h, id, out);
}

void OccupantBounds(void*, uint64_t occ, fc4_cell_rect* out) {
    if (out == nullptr || occ == 0) return;
    SC4Rect<long> r;
    OccOf(occ)->GetBoundingCityCells(r);
    out->x1 = static_cast<int32_t>(r.topLeftX);
    out->z1 = static_cast<int32_t>(r.topLeftY);
    out->x2 = static_cast<int32_t>(r.bottomRightX);
    out->z2 = static_cast<int32_t>(r.bottomRightY);
}

void OccupantPosition(void*, uint64_t occ, float* xyz) {
    if (xyz == nullptr || occ == 0) return;
    cS3DVector3 v;
    OccOf(occ)->GetPosition(&v);
    xyz[0] = v.fX;
    xyz[1] = v.fY;
    xyz[2] = v.fZ;
}

// As our simulator: the typed overload (compiled slot 41) once per accepted
// type (cFC4Flammability.h explains why not the filter overload).
void OccupantsInCells(void*, const fc4_cell_rect* r, fc4_token_callback cb, void* ctx) {
    if (r == nullptr || cb == nullptr || g_city == nullptr) return;
    cISC4OccupantManager* om = g_city->GetOccupantManager();  // borrowed
    if (om == nullptr) return;
    const SC4Point<int> mn(r->x1, r->z1);
    const SC4Point<int> mx(r->x2, r->z2);
    for (uint32_t type : kOccupantTypes) {
        auto list = std::make_shared<SC4List<cISC4Occupant*>>();
        om->GetOccupantsByStandardCityCells(*list, &mn, &mx, type, 0xFFFFFFFFu);
        for (cISC4Occupant* o : *list) {
            if (o != nullptr) cb(ctx, TokenOf(o));
        }
        g_held.push_back(std::move(list));
    }
}

uint64_t OccupantLot(void*, uint64_t occ) {
    cISC4LotManager* lm = (occ != 0 && g_city != nullptr) ? g_city->GetLotManager() : nullptr;  // borrowed
    return lm ? TokenOf(AsGame(lm)->GetOccupantLot(OccOf(occ))) : 0;
}

uint8_t LotCondition(void*, uint64_t lot) { return lot ? static_cast<uint8_t>(LotOf(lot)->GetCondition()) : 0; }

uint64_t LotBuilding(void*, uint64_t lot) {
    if (lot == 0) return 0;
    cISC4BuildingOccupant* b = LotOf(lot)->GetBuilding();  // not ref-counted
    return b ? TokenOf(b->AsOccupant()) : 0;
}

bool CumulativeFactor(void*, uint32_t prop, uint32_t index, float* out) {
    cISC4OrdinanceSimulator* o = g_city ? g_city->GetOrdinanceSimulator() : nullptr;  // borrowed
    if (o == nullptr) return false;
    if (out != nullptr) *out = o->GetCumulativeFactor(prop, index);
    return true;
}

bool CurrentSeason(void*, int32_t* out) {
    cISC4WeatherSimulator* w = g_city ? g_city->GetWeatherSimulator() : nullptr;  // borrowed
    if (w == nullptr) return false;
    if (out != nullptr) *out = AsGame(w)->GetCurrentSeason();
    return true;
}

int32_t IsCellWatered(void*, int32_t x, int32_t z) {
    cISC4PlumbingSimulator* p = g_city ? g_city->GetPlumbingSimulator() : nullptr;  // borrowed
    if (p == nullptr) return -1;
    return AsGame(p)->IsCellWatered(x, z) ? 1 : 0;
}

// Plumbing slot 7 GetWateredGrid (plumbing.md §10.1): the published grid,
// borrowed. The shadow polls slot 11 by content (flammability.md §6.5, FL7,
// FL9) and only counts this pointer's changes. Only the pointer is used,
// never the grid.
uint64_t WateredGrid(void*) {
    cISC4PlumbingSimulator* p = g_city ? g_city->GetPlumbingSimulator() : nullptr;  // borrowed
    return p ? TokenOf(AsGame(p)->GetWateredGrid()) : 0;
}

bool ZoneType(void*, int32_t x, int32_t z, uint32_t* out) {
    cISC4ZoneManager* zm = g_city ? g_city->GetZoneManager() : nullptr;  // borrowed
    if (zm == nullptr) return false;
    if (out != nullptr) *out = static_cast<uint32_t>(zm->GetZoneType(x, z));
    return true;
}

fc4_flammability_host ReadOnlyHost() {
    fc4_flammability_host h{};
    h.struct_size = sizeof(h);
    h.user = nullptr;
    h.city_cell_counts = &CityCellCounts;
    h.position_to_cell = &PositionToCell;
    h.tuning = &Tuning;
    h.occupant_type = &OccupantType;
    h.occupant_property = &OccupantProperty;
    h.occupant_bounds = &OccupantBounds;
    h.occupant_position = &OccupantPosition;
    h.occupants_in_cells = &OccupantsInCells;
    h.occupant_lot = &OccupantLot;
    h.lot_condition = &LotCondition;
    h.lot_building = &LotBuilding;
    h.cumulative_factor = &CumulativeFactor;
    h.current_season = &CurrentSeason;
    h.is_cell_watered = &IsCellWatered;
    h.zone_type = &ZoneType;
    h.watered_grid = &WateredGrid;
    return h;
}

// ---- the original ---------------------------------------------------------------------------------

cISC4FlammabilitySimulator* OriginalOf(cISC4City* city) {
    if (city == nullptr) return nullptr;
    cISC4FlammabilitySimulator* f = city->GetFlammabilitySimulator();  // compiled slot 58, borrowed
    if (f == nullptr) {
        if (!g_loggedNone) {
            g_loggedNone = true;
            Log("flammability shadow: the city has no flammability simulator; nothing compared");
        }
        return nullptr;
    }
    if (IsOurObject(f)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("flammability shadow: inactive: the city's flammability simulator is FreeCity4's own (the shadow "
                "compares against the ORIGINAL: do not enable flammability)");
        }
        return nullptr;
    }
    return f;
}

void DropShadow() {
    fc4_flammability_shadow_destroy(g_shadow);
    g_shadow = nullptr;
    g_shadowCity = nullptr;
}

// One shadow core per city (a new original simulator restarts its baseline).
bool EnsureShadow(cISC4City* city) {
    if (g_shadow != nullptr && g_shadowCity == city) return true;
    DropShadow();
    const fc4_flammability_host host = ReadOnlyHost();  // copied by the core
    g_shadow = fc4_flammability_shadow_create(&host, static_cast<uint32_t>(g_cells));
    if (g_shadow == nullptr) {
        Log("flammability shadow: the shadow core could not be created; nothing compared");
        return false;
    }
    g_shadowCity = city;
    Log("flammability shadow: following the original flammability simulator of city %p; differing tracts listed per "
        "grid: %ld",
        static_cast<const void*>(city), g_cells);
    return true;
}

void GeometryOf(GridU8* g, fc4_grid_geometry* out) {
    out->tract_size = g->GetTractSize();
    out->shift = g->GetTractShift();
    out->count_x = g->GetTractCountX();
    out->count_z = g->GetTractCountZ();
    out->width_x = g->GetTractWidthX();
    out->width_z = g->GetTractWidthZ();
    out->inv_width_x = g->GetOneOverTractWidthX();
    out->inv_width_z = g->GetOneOverTractWidthZ();
}

// Every tract, x outer, through GetTractValue (never the memory).
void ReadAll(GridU8* g, fc4_grid_geometry* geom, std::vector<uint8_t>& out) {
    *geom = fc4_grid_geometry{};
    out.clear();
    if (g == nullptr) return;
    GeometryOf(g, geom);
    if (geom->count_x <= 0 || geom->count_z <= 0 || geom->count_x > kMaxCount || geom->count_z > kMaxCount) return;
    out.reserve(static_cast<size_t>(geom->count_x) * static_cast<size_t>(geom->count_z));
    for (int32_t x = 0; x < geom->count_x; ++x) {
        for (int32_t z = 0; z < geom->count_z; ++z) out.push_back(g->GetTractValue(x, z));
    }
}

struct OrigRead {
    std::vector<uint8_t> adjusted, raw, flammable;
    std::vector<fc4_flammability_shadow_hotspot> hotspots;
    fc4_flammability_original o{};
};

// The record (§10): u16 version 2, the adjusted and raw grid references, f32
// multiplier, u8 average, u32 count, count x (s32 x, s32 z, s32 weight).
bool ParseRecord(OrigRead& r) {
    CaptureReader in(g_capture.Bytes());
    uint16_t version = 0;
    uint32_t adjusted = 0, raw = 0, count = 0;
    float g = 0.0f;
    uint8_t avg = 0;
    if (!in.U16(version) || version != kRecordVersion || !in.Ref(adjusted) || !in.Ref(raw) || !in.F32(g) ||
        !in.U8(avg) || !in.U32(count) || count > kMaxHotspots) {
        return false;
    }
    for (uint32_t i = 0; i < count; ++i) {
        fc4_flammability_shadow_hotspot h{};
        if (!in.I32(h.x) || !in.I32(h.z) || !in.I32(h.weight)) return false;
        r.hotspots.push_back(h);
    }
    // The references' cISC4SimGrid identities (borrowed: the original keeps
    // its grids while this message is delivered).
    ReadAll(static_cast<GridU8*>(g_capture.Grid(adjusted)), &r.o.geometry, r.adjusted);
    ReadAll(static_cast<GridU8*>(g_capture.Grid(raw)), &r.o.raw_geometry, r.raw);
    r.o.global_multiplier = g;
    r.o.average = avg;
    return true;
}

void ReadOriginal(cISC4City* city, cISC4FlammabilitySimulator* sim, OrigRead& r) {
    r.o.struct_size = sizeof(r.o);
    r.o.simulator = TokenOf(sim);
    bool answered = false;
    if (g_capture.Capture(sim, &answered) && ParseRecord(r)) {
        r.o.has_record = 1;
    } else if (!g_loggedNoRecord) {
        g_loggedNoRecord = true;
        Log("flammability shadow: the original's record (its own Write) %s; its grids, multiplier, average and "
            "hotspots cannot be read: nothing is compared",
            answered ? "did not parse" : "cannot be read (no cIGZSerializable)");
    }
    r.o.adjusted = r.adjusted.empty() ? nullptr : r.adjusted.data();
    r.o.raw = r.raw.empty() ? nullptr : r.raw.data();
    r.o.hotspots = r.hotspots.empty() ? nullptr : r.hotspots.data();
    r.o.hotspot_count = static_cast<uint32_t>(r.hotspots.size());
    // The getters (§11.1): slot 6 per city cell, slot 7, slot 8.
    const int32_t cx = static_cast<int32_t>(city->CellCountX());
    const int32_t cz = static_cast<int32_t>(city->CellCountZ());
    if (cx > 0 && cz > 0 && cx <= kMaxCount && cz <= kMaxCount) {
        r.flammable.reserve(static_cast<size_t>(cx) * static_cast<size_t>(cz));
        for (int32_t x = 0; x < cx; ++x) {
            for (int32_t z = 0; z < cz; ++z) r.flammable.push_back(sim->IsCellFlammable(x, z) ? 1 : 0);
        }
        r.o.flammable = r.flammable.data();
        r.o.cells_x = cx;
        r.o.cells_z = cz;
    }
    uint8_t value = 0;
    int x = 0, z = 0;
    r.o.hotspot_found = sim->GetFlammabilityHotspot(&value, &x, &z) ? 1 : 0;
    r.o.hotspot_value = value;
    r.o.hotspot_x = x;
    r.o.hotspot_z = z;
    r.o.average_getter = sim->GetAverageFlammability();
}

void Sink(void*, const uint8_t* data, size_t len) { LogShadowLine("flammability shadow (cont.)", data, len); }

// ---- one event (SehGuard's context) --------------------------------------------------------------

struct Run {
    cISC4City* city = nullptr;
    cISC4FlammabilitySimulator* sim = nullptr;
    int32_t julian = 0;
    const fc4_flammability_message* msg = nullptr;  // null: a cycle end
    bool done = false;
    intptr_t result = -1;
};
Run g_run;

void Step(void*) {
    if (g_run.msg != nullptr) {
        g_run.result = fc4_flammability_shadow_message(g_shadow, g_run.msg) ? 1 : 0;
    } else {
        OrigRead r;
        ReadOriginal(g_run.city, g_run.sim, r);
        g_run.result = fc4_flammability_shadow_cycle(g_shadow, &r.o, g_run.julian, &Sink, nullptr);
    }
    g_run.done = true;
}

// QueryInterface for `iid` (one reference, FC4_ACQUIRED), or nullptr.
cIGZUnknown* Query(cIGZUnknown* u, uint32_t iid) {
    void* p = nullptr;
    if (u == nullptr || !u->QueryInterface(iid, &p) || p == nullptr) return nullptr;
    cIGZUnknown* q = static_cast<cIGZUnknown*>(p);
    FC4_ACQUIRED(q, "QueryInterface");
    return q;
}

void Drop(cIGZUnknown* p) {
    if (p != nullptr) FC4_RELEASE(p);
}

intptr_t OnEvent(cISC4City* city, int32_t julian, const fc4_flammability_message* msg) {
    cISC4FlammabilitySimulator* sim = OriginalOf(city);
    if (sim == nullptr) return -1;
    g_city = city;
    if (!EnsureShadow(city)) {
        g_city = nullptr;
        return -1;
    }
    g_run = Run{};
    g_run.city = city;
    g_run.sim = sim;
    g_run.julian = julian;
    g_run.msg = msg;
    SehGuard(msg ? "flammability shadow (message)" : "flammability shadow (cycle end)", &Step, nullptr);
    g_held.clear();  // the occupant lists of this call
    g_city = nullptr;
    intptr_t result = g_run.done ? g_run.result : -2;
    if (msg == nullptr && result < 0) result = -2;
    if (msg != nullptr && !g_run.done) result = 0;
    g_run = Run{};
    return result;
}

// One of the six messages, with its objects as the original takes them
// (§5): void1 queried for cISC4Occupant, IGZUnknown for cISC4Lot; both
// references are released after the call. [compat] (§5, FL4): one send
// delivers in subscription order, so the original (subscribed in its Init)
// handles each message just before this later target, on the same state.
// The plumbing pass end comes here too, without objects: the core polls the
// watered state (until its first day poll) and mirrors nothing. So does the
// original's SimNewDay (FlammabilityShadowDeliver): a day poll.
intptr_t OnMessage(cISC4City* city, uint32_t type, cIGZUnknown* void1, cIGZUnknown* unknown) {
    if (OriginalOf(city) == nullptr) return -1;
    fc4_flammability_message m{};
    m.msg_type = type;
    cIGZUnknown* occ = nullptr;
    cIGZUnknown* lot = nullptr;
    if (type == kMsgInsertOccupant || type == kMsgRemoveOccupant || type == kMsgChangeOccupant) {
        occ = Query(void1, kIID_cISC4Occupant);
        m.occupant = TokenOf(occ);
    } else if (type == kMsgConditionChanged) {
        lot = Query(unknown, kIID_cISC4Lot);
        m.lot = TokenOf(lot);
    }
    const intptr_t r = OnEvent(city, 0, &m);
    Drop(lot);
    Drop(occ);
    return r;
}

cISC4City* CurrentCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    FC4_ACQUIRED(sc4, "QueryInterface");
    cISC4City* city = sc4->GetCity();  // borrowed
    FC4_RELEASE(sc4);
    return city;
}

// The shadow's message target: a static object, counted, never deleted.
class Target final : public cIGZMessageTarget2 {
public:
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid != kIID_cIGZMessageTarget2 && riid != GZIID_cIGZUnknown) return false;
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return static_cast<uint32_t>(InterlockedIncrement(&refs_)); }
    uint32_t Release() override {
        const LONG r = InterlockedDecrement(&refs_);
        return r > 0 ? static_cast<uint32_t>(r) : 0u;
    }
    bool DoMessage(cIGZMessage2* msg) override {
        if (msg == nullptr) return true;
        const uint32_t type = msg->GetType();
        if (type == kMsgPostCityShutdown) {
            DropShadow();
        } else if (type == kMsgSimCycleComplete) {
            OnEvent(CurrentCity(), static_cast<int32_t>(PublishedJulian()), nullptr);
        } else {
            cIGZUnknown* s = Query(msg, GZIID_cIGZMessage2Standard);
            cIGZMessage2Standard* sm = static_cast<cIGZMessage2Standard*>(static_cast<void*>(s));
            OnMessage(CurrentCity(), type, sm ? static_cast<cIGZUnknown*>(sm->GetVoid1()) : nullptr,
                      sm ? sm->GetIGZUnknown() : nullptr);
            Drop(s);
        }
        return true;
    }

private:
    volatile LONG refs_ = 0;
};

Target& TheTarget() {
    static Target t;
    return t;
}

}  // namespace

void FlammabilityShadowStartup(bool flammabilityReplaced) {
    ReadSettings();
    g_replaced = flammabilityReplaced;
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("flammability_shadow", "FC4_FLAMMABILITY_SHADOW", &src);
    if (flammabilityReplaced) {
        Log("flammability shadow: requested (flammability_shadow from %s) but INACTIVE: it compares against the "
            "ORIGINAL flammability simulator, and flammability is enabled (ours)",
            src);
        return;
    }
    Log("flammability shadow: ON (flammability_shadow from %s): a mirror simulator (FreeCity4's, on its own grids) "
        "follows the ORIGINAL flammability simulator's six messages on its live inputs, read-only; at each of the "
        "original's cycle ends (SimCycleComplete 0x08F415FE) the original's record (its own Write: adjusted and raw "
        "grids, multiplier, average, hotspots) and getters (slots 6-8) are compared with it; the first cycle seen is "
        "the baseline; one line per cycle; differing tracts listed per grid: %ld; a tract whose watered state changed "
        "in the cycle (plumbing slot 11 compared by content right after every SimNewDay the original gets, or at "
        "every event before the first one, and at the cycle end) may hold its value under either state, as the day "
        "visit applies the no-water factor (FL5-FL7); the changes seen after the wrap day (by the count of days since "
        "the original's Init) are carried into the next cycle (FL8)",
        src, g_cells);
    Log("flammability shadow: NOT readable read-only (left out of the comparison): the iterator start and the "
        "hotspot shuffle (C rand() draws), the fire roll (a clock-seeded generator) and the random lot, the "
        "candidate list and the zoned sum and count (no getter, not saved), and the iterator position (so each "
        "adjusted tract is checked against the values the cycle could have written, and the hotspots against every "
        "rotation of the visiting order)");
}

void FlammabilityShadowStart() {
    if (g_started || !g_enabled || g_replaced) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) {
        Log("flammability shadow: no message server; not started");
        return;
    }
    TheTarget().AddRef();  // our own reference (a static object)
    bool ok = true;
    for (uint32_t m : kMessages) ok = ms->AddNotification(static_cast<cIGZMessageTarget2*>(&TheTarget()), m) && ok;
    g_started = true;
    Log("flammability shadow: listening (the six messages, SimCycleComplete, PostCityShutdown, the plumbing pass "
        "end)%s",
        ok ? "" : " [WARNING a notification was refused]");
}

void FlammabilityShadowStop() {
    if (!g_started) return;
    g_started = false;
    cIGZMessageServer2Ptr ms;
    if (ms) {
        for (uint32_t m : kMessages) ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(&TheTarget()), m);
    }
    TheTarget().Release();
    DropShadow();
}

bool FlammabilityShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                               int64_t data1, bool* finished) {
    (void)data1;  // §5: read but not used by the original either
    if (type != kMsgSimNewDay || target == nullptr || msg == nullptr || finished == nullptr) return false;
    if (!g_started || g_replaced) return false;
    // Cheap rejection for every other agent: the original registers by name (§4.1 step 2).
    if (agentName == nullptr || strcmp(agentName, kAgentName) != 0) return false;
    cISC4City* city = CurrentCity();
    cISC4FlammabilitySimulator* sim = OriginalOf(city);
    if (sim == nullptr) return false;
    // The original's own agent: its cIGZMessageTarget2 identity (only the
    // identity is kept; the simulator keeps it alive).
    void* agent = nullptr;
    if (sim->QueryInterface(kIID_cIGZMessageTarget2, &agent) && agent != nullptr) {
        cIGZUnknown* a = static_cast<cIGZUnknown*>(agent);
        FC4_ACQUIRED(a, "QueryInterface");
        const bool same = static_cast<cIGZMessageTarget2*>(agent) == target;
        FC4_RELEASE(a);
        if (!same) return false;
    }
    *finished = target->DoMessage(msg);  // the original flammability simulator's day, unchanged
    // [choice] (flammability.md §6.5, FL7/FL8): right after the original's
    // visits, the core counts the day and polls the watered state.
    OnMessage(city, kMsgSimNewDay, nullptr, nullptr);
    return true;
}

}  // namespace fc4

intptr_t FlammabilityShadowTest(const FC4FlammabilityShadowTestInput* in) {
    if (in == nullptr) return -1;
    switch (in->event) {
        case 0:
            return fc4::OnMessage(in->city, in->msg_type, in->void1, in->igz_unknown);
        case 1:
            return fc4::OnEvent(in->city, in->julian, nullptr);
        case 2:
            fc4::DropShadow();
            return 0;
        default:
            return -1;
    }
}
