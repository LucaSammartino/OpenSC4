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

#include "FC4LandValueShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4Watchdog.h"
#include "FC4WriteCapture.h"
#include "cFC4LandValue.h"
#include "fc4.h"

#include "GZServPtrs.h"
#include "SC4Rect.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4App.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4City.h"
#include "cISC4GrowthDeveloper.h"
#include "cISC4LandValueSimulator.h"
#include "cISC4LotDeveloper.h"
#include "cISC4SimGrid.h"
#include "cISC4ZoneManager.h"
#include "cISTETerrain.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
// landvalue.md §9: posted at the end of state 8 (the pass end).
constexpr uint32_t kMsgLandValueMapChanged = 0x87CC75B3;
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgPostCityShutdown = 0x26D31EC3;
constexpr uint32_t kMessages[] = {kMsgLandValueMapChanged, kMsgSimNewDay, kMsgPostCityShutdown};
constexpr uint16_t kRecordVersion = 1;  // landvalue.md §10
constexpr long kDefaultCells = 4;
constexpr long kMaxCells = 64;

using GridU8 = cISC4SimGrid<uint8_t>;
using GridI16 = cISC4SimGrid<int16_t>;
using GridF32 = cISC4SimGrid<float>;

// ---- settings and run state (main thread only) ---------------------------------------------------

bool g_enabled = false;
bool g_replaced = false;
long g_cells = kDefaultCells;
bool g_started = false;

bool g_loggedOurs = false;
bool g_loggedNone = false;
bool g_loggedNoRecord = false;

// The city whose services the read-only host answers with (borrowed for one
// call into the core).
cISC4City* g_city = nullptr;

fc4_landvalue_shadow* g_shadow = nullptr;
const void* g_shadowCity = nullptr;
const void* g_shadowSim = nullptr;

WriteCapture g_capture;

void ReadSettings() {
    g_enabled = SettingInt("landvalue_shadow", "FC4_LANDVALUE_SHADOW", 0) != 0;
    long n = SettingInt("landvalue_shadow_cells", "FC4_LANDVALUE_SHADOW_CELLS", kDefaultCells);
    if (n <= 0) n = kDefaultCells;
    if (n > kMaxCells) n = kMaxCells;
    g_cells = n;
}

// ---- the read-only host (fc4_landvalue_host, landvalue.md §12) ------------------------------------
//
// Only reads: no grid is created, written, resized, shut down or released,
// nothing is recorded or posted (those callbacks stay NULL).

cISTETerrain* Terrain() { return g_city ? g_city->GetTerrain() : nullptr; }  // borrowed

cISC4BuildingDevelopmentSimulator* Bds() { return g_city ? g_city->GetBuildingDevelopmentSimulator() : nullptr; }

GridU8* U8(uint64_t t) { return reinterpret_cast<GridU8*>(FromToken(t)); }
GridI16* I16(uint64_t t) { return reinterpret_cast<GridI16*>(FromToken(t)); }

void TerrainCells(void*, int32_t* x, int32_t* z) {
    cISTETerrain* t = Terrain();
    *x = t ? static_cast<int32_t>(t->CellCountX()) : 0;
    *z = t ? static_cast<int32_t>(t->CellCountZ()) : 0;
}

bool IsLandVertex(void*, uint32_t x, uint32_t z) {
    cISTETerrain* t = Terrain();
    return t != nullptr && t->IsLandVertex(x, z);
}

float Altitude(void*, float x, float z) {
    cISTETerrain* t = Terrain();
    return t ? t->GetAltitude(x, z) : 0.0f;  // compiled slot 13 (foundations §16)
}

float MinAllowedHeight(void*) {
    cISTETerrain* t = Terrain();
    return t ? t->MinAllowedHeight() : 0.0f;
}

float MaxAllowedHeight(void*) {
    cISTETerrain* t = Terrain();
    return t ? t->MaxAllowedHeight() : 0.0f;
}

uint32_t ZoneType(void*, int32_t x, int32_t z) {
    cISC4ZoneManager* zm = g_city ? g_city->GetZoneManager() : nullptr;  // borrowed
    return zm ? static_cast<uint32_t>(zm->GetZoneType(x, z)) : 0;
}

// [compat] (landvalue.md §6, LV3): GetAreaSlope (lot developer slot 29) is
// read-only in the original - terrain altitude and water level reads, locals
// only - so the shadow calls it as the original's state 5 does.
float AreaSlope(void*, int32_t x1, int32_t z1, int32_t x2, int32_t z2) {
    cISC4BuildingDevelopmentSimulator* bds = Bds();
    cISC4LotDeveloper* ld = bds ? bds->GetLotDeveloper() : nullptr;  // borrowed
    if (ld == nullptr) return 0.0f;
    const SC4Rect<int32_t> r(x1, z1, x2, z2);
    return ld->GetAreaSlope(r, true);
}

// The token is the pointer GetDeveloper(ID) returns: the original applies its
// developers in ascending unsigned order of these pointers, and so does the
// shadow's pass (landvalue.md §7.4, LV1).
uint64_t Developer(void*, uint32_t type) {
    cISC4BuildingDevelopmentSimulator* bds = Bds();
    if (bds == nullptr) return 0;
    return TokenOf(bds->GetDeveloper(static_cast<cISC4BuildingDevelopmentSimulator::DeveloperType>(type)));
}

uint64_t DesirabilityMap(void*, uint64_t dev) {
    if (dev == 0) return 0;
    return TokenOf(reinterpret_cast<cISC4GrowthDeveloper*>(FromToken(dev))->GetDesirabilityMap());
}

// Growth developer slot 10 GrowthAndDeclineListsAreValid (state < 17, EAX
// 0/1; growth-developer.md §3.2, landvalue.md §7.4 LV2): true while the
// developer's published desirability map cannot change, so the shadow
// snapshots the map only then; the last such snapshot before a change is the
// map from before the developer's state-17 copy.
bool DeveloperListsValid(void*, uint64_t dev) {
    if (dev == 0) return false;
    return reinterpret_cast<cISC4GrowthDeveloper*>(FromToken(dev))->GrowthAndDeclineListsAreValid();
}

// Slots 10-17 do not depend on the element type.
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

void GridGeometry(void*, uint64_t t, fc4_grid_geometry* out) {
    if (GridU8* g = U8(t)) {
        if (out != nullptr) GeometryOf(g, out);
    }
}

// x outer, z inner, through the interface (GetTractValue), never the memory.
template <class G, class T>
void ReadGrid(G* g, T* out, size_t n) {
    if (g == nullptr || out == nullptr) return;
    const int32_t cx = g->GetTractCountX(), cz = g->GetTractCountZ();
    size_t i = 0;
    for (int32_t x = 0; x < cx; ++x) {
        for (int32_t z = 0; z < cz && i < n; ++z) out[i++] = g->GetTractValue(x, z);
    }
}

void GridReadU8(void*, uint64_t t, uint8_t* out, size_t n) { ReadGrid(U8(t), out, n); }
void GridReadI16(void*, uint64_t t, int16_t* out, size_t n) { ReadGrid(I16(t), out, n); }

fc4_landvalue_host ReadOnlyHost() {
    fc4_landvalue_host h{};
    h.struct_size = sizeof(h);
    h.user = nullptr;
    h.terrain_cells = &TerrainCells;
    h.is_land_vertex = &IsLandVertex;
    h.altitude = &Altitude;
    h.min_allowed_height = &MinAllowedHeight;
    h.max_allowed_height = &MaxAllowedHeight;
    h.zone_type = &ZoneType;
    h.area_slope = &AreaSlope;
    h.developer = &Developer;
    h.desirability_map = &DesirabilityMap;
    h.developer_lists_valid = &DeveloperListsValid;
    h.grid_geometry = &GridGeometry;
    h.grid_read_u8 = &GridReadU8;
    h.grid_read_i16 = &GridReadI16;
    return h;
}

// ---- the original ---------------------------------------------------------------------------------

// The original land value simulator of `city` (borrowed), or nullptr (logged
// once when it is missing or ours).
cISC4LandValueSimulator* OriginalOf(cISC4City* city) {
    if (city == nullptr) return nullptr;
    cISC4LandValueSimulator* lv = city->GetLandValueSimulator();  // compiled slot 61, borrowed
    if (lv == nullptr) {
        if (!g_loggedNone) {
            g_loggedNone = true;
            Log("landvalue shadow: the city has no land value simulator; nothing compared");
        }
        return nullptr;
    }
    if (IsOurObject(lv)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("landvalue shadow: inactive: the city's land value simulator is FreeCity4's own (the shadow compares "
                "against the ORIGINAL: do not enable landvalue)");
        }
        return nullptr;
    }
    return lv;
}

void DropShadow() {
    fc4_landvalue_shadow_destroy(g_shadow);
    g_shadow = nullptr;
    g_shadowCity = nullptr;
    g_shadowSim = nullptr;
}

struct CreateCtx {
    const fc4_landvalue_host* host;
    bool tuning;
};

void CreateWithTuning(void* ctx, const fc4_landvalue_tuning* t) {
    CreateCtx* c = static_cast<CreateCtx*>(ctx);
    c->tuning = t != nullptr;
    g_shadow = fc4_landvalue_shadow_create(c->host, t, static_cast<uint32_t>(g_cells));
}

// A shadow core per city and original simulator, made with the tuning
// exemplar our Init would read.
bool EnsureShadow(cISC4City* city, const void* sim) {
    if (g_shadow != nullptr && g_shadowCity == city && g_shadowSim == sim) return true;
    DropShadow();
    const fc4_landvalue_host host = ReadOnlyHost();  // copied by the core
    CreateCtx ctx{&host, false};
    FC4LandValueReadTuning(&CreateWithTuning, &ctx);
    if (g_shadow == nullptr) {
        Log("landvalue shadow: the shadow core could not be created; nothing compared");
        return false;
    }
    g_shadowCity = city;
    g_shadowSim = sim;
    Log("landvalue shadow: comparing with the original land value simulator %p (city %p); tuning %s; differing "
        "tracts listed per grid: %ld",
        sim, static_cast<const void*>(city), ctx.tuning ? "from the exemplar" : "the code defaults (no exemplar)",
        g_cells);
    return true;
}

// What was read from the original, with the arrays the ABI struct points at.
struct OrigRead {
    std::vector<uint8_t> intrinsic, total, types;
    std::vector<float> slope;
    fc4_landvalue_original o{};
};

template <class G, class T>
void ReadAll(G* g, fc4_grid_geometry* geom, std::vector<T>& out) {
    *geom = fc4_grid_geometry{};
    out.clear();
    if (g == nullptr) return;
    GeometryOf(reinterpret_cast<GridU8*>(g), geom);
    if (geom->count_x <= 0 || geom->count_z <= 0 || geom->count_x > 4096 || geom->count_z > 4096) return;
    out.resize(static_cast<size_t>(geom->count_x) * static_cast<size_t>(geom->count_z));
    ReadGrid(g, out.data(), out.size());
}

void ReadOriginal(cISC4LandValueSimulator* lv, OrigRead& r) {
    r.o.struct_size = sizeof(r.o);
    // The record (§10): u16 version, three references, u8 dirty-or-running,
    // u8 average, f32 mean altitude.
    bool answered = false;
    if (g_capture.Capture(lv, &answered)) {
        CaptureReader in(g_capture.Bytes());
        uint16_t version = 0;
        uint32_t ref = 0;
        uint8_t flag = 0, avg = 0;
        float mean = 0.0f;
        if (in.U16(version) && version == kRecordVersion && in.Ref(ref) && in.Ref(ref) && in.Ref(ref) &&
            in.U8(flag) && in.U8(avg) && in.F32(mean)) {
            r.o.has_record = 1;
            r.o.record_flag = flag;
            r.o.record_average = avg;
            r.o.mean_altitude = mean;
        }
    }
    if (r.o.has_record == 0 && !g_loggedNoRecord) {
        g_loggedNoRecord = true;
        Log("landvalue shadow: the original's record (its own Write) %s; the mean altitude and the settled flag are "
            "not compared",
            answered ? "did not parse" : "cannot be read (no cIGZSerializable)");
    }
    GridU8* intrinsic = lv->GetIntrinsicLandValueMap();  // slot 7, borrowed
    GridU8* total = lv->GetTotalLandValueMap();          // slot 8
    GridF32* slope = lv->GetSlopeMap();                  // slot 9
    ReadAll(intrinsic, &r.o.intrinsic_geometry, r.intrinsic);
    ReadAll(total, &r.o.total_geometry, r.total);
    ReadAll(slope, &r.o.slope_geometry, r.slope);
    // GetLandValueType (slot 5) takes cell coordinates: each total tract's first
    // cell. [compat] (landvalue.md §7.6, LV4): it reads the total grid's
    // GetCellValue, which shifts the cell to its tract, so the type is uniform
    // over a tract and the first cell gives it exactly.
    if (!r.total.empty()) {
        const int32_t s = r.o.total_geometry.tract_size;
        r.types.reserve(r.total.size());
        for (int32_t x = 0; x < r.o.total_geometry.count_x; ++x) {
            for (int32_t z = 0; z < r.o.total_geometry.count_z; ++z) {
                r.types.push_back(static_cast<uint8_t>(lv->GetLandValueType(x * s, z * s)));
            }
        }
    }
    r.o.average = lv->GetAverageLandValue();    // slot 10
    r.o.mode_altitude = lv->GetModeAltitude();  // slot 11
    r.o.intrinsic = r.intrinsic.empty() ? nullptr : r.intrinsic.data();
    r.o.total = r.total.empty() ? nullptr : r.total.data();
    r.o.slope = r.slope.empty() ? nullptr : r.slope.data();
    r.o.types = r.types.empty() ? nullptr : r.types.data();
}

void Sink(void*, const uint8_t* data, size_t len) { LogShadowLine("landvalue shadow (cont.)", data, len); }

// ---- one event (SehGuard's context) --------------------------------------------------------------

struct Run {
    cISC4LandValueSimulator* sim = nullptr;
    int32_t julian = 0;
    bool day = false;
    bool done = false;
    intptr_t result = -1;
};
Run g_run;

void Step(void*) {
    if (g_run.day) {
        g_run.result = static_cast<intptr_t>(fc4_landvalue_shadow_note_day(g_shadow));
    } else {
        OrigRead r;
        ReadOriginal(g_run.sim, r);
        g_run.result = fc4_landvalue_shadow_compare(g_shadow, &r.o, g_run.julian, &Sink, nullptr);
    }
    g_run.done = true;
}

intptr_t OnEvent(cISC4City* city, int32_t julian, bool day) {
    cISC4LandValueSimulator* sim = OriginalOf(city);
    if (sim == nullptr) return -1;
    g_city = city;
    if (!EnsureShadow(city, sim)) {
        g_city = nullptr;
        return -1;
    }
    g_run = Run{};
    g_run.sim = sim;
    g_run.julian = julian;
    g_run.day = day;
    SehGuard(day ? "landvalue shadow (day)" : "landvalue shadow (pass end)", &Step, nullptr);
    g_city = nullptr;
    intptr_t result = g_run.done ? g_run.result : -2;
    if (!day && result < 0) result = -2;
    g_run = Run{};
    return result;
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

intptr_t Dispatch(uint32_t type, cISC4City* city, int32_t julian) {
    switch (type) {
        case kMsgLandValueMapChanged:
            return OnEvent(city, julian, false);
        case kMsgSimNewDay:
            return OnEvent(city, julian, true);
        case kMsgPostCityShutdown:
            DropShadow();
            return 0;
        default:
            return -1;
    }
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
        Dispatch(type, type == kMsgPostCityShutdown ? nullptr : CurrentCity(),
                 static_cast<int32_t>(PublishedJulian()));
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

void LandValueShadowStartup(bool landValueReplaced) {
    ReadSettings();
    g_replaced = landValueReplaced;
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("landvalue_shadow", "FC4_LANDVALUE_SHADOW", &src);
    if (landValueReplaced) {
        Log("landvalue shadow: requested (landvalue_shadow from %s) but INACTIVE: it compares against the ORIGINAL "
            "land value simulator, and landvalue is enabled (ours)",
            src);
        return;
    }
    Log("landvalue shadow: ON (landvalue_shadow from %s): at the end of each pass of the ORIGINAL land value "
        "simulator (LandValueMapChanged 0x87CC75B3, posted at the end of state 8) FreeCity4's pass (states 0-8) runs "
        "on its live inputs, read-only, on local grids, and is compared tract by tract (intrinsic, total, slope, "
        "GetLandValueType) with the average, mode altitude and mean altitude; one line per pass; differing tracts "
        "listed per grid: %ld; several growth developers apply in ascending pointer order, as in the original (LV1); "
        "a desirability map republished after the original's pass read it is retried with the map from before the "
        "developer's last state-17 copy (snapshotted while its GrowthAndDeclineListsAreValid is true, LV2)",
        src, g_cells);
    Log("landvalue shadow: NOT readable read-only (left out of the comparison): the scratch grid, the histogram, the "
        "minimum / maximum altitude, pivot and ranges (no getter, not saved: they show only through the intrinsic "
        "grid), the dirty flag apart (the record holds dirty OR running: logged as settled), the "
        "inputs as each state read them (input_capture=pass-end-sample; native_step_capture=0; "
        "settled=0 means dirty OR running, not proof of an input change). LV1 section 6.1 describes native "
        "progress fields; their docs/abi layout record is published (LV6 closed), but safe native interceptor "
        "installation/exclusion and lifetime remain unproved. LV8 (section 6.1.4) confirms lifecycle ordering "
        "is not dispatch exclusion; no host exclusion mechanism is installed. A clean sampled "
        "comparison is not independent per-Step input evidence");
}

void LandValueShadowStart() {
    if (g_started || !g_enabled || g_replaced) return;
    cIGZMessageServer2Ptr ms;
    if (!ms) {
        Log("landvalue shadow: no message server; not started");
        return;
    }
    TheTarget().AddRef();  // our own reference (a static object)
    bool ok = true;
    for (uint32_t m : kMessages) ok = ms->AddNotification(static_cast<cIGZMessageTarget2*>(&TheTarget()), m) && ok;
    g_started = true;
    Log("landvalue shadow: listening (LandValueMapChanged, SimNewDay, PostCityShutdown)%s",
        ok ? "" : " [WARNING a notification was refused]");
}

void LandValueShadowStop() {
    if (!g_started) return;
    g_started = false;
    cIGZMessageServer2Ptr ms;
    if (ms) {
        for (uint32_t m : kMessages) ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(&TheTarget()), m);
    }
    TheTarget().Release();
    DropShadow();
}

}  // namespace fc4

intptr_t LandValueShadowTest(const FC4LandValueShadowTestInput* in) {
    if (in == nullptr) return -1;
    static const uint32_t kEvents[] = {fc4::kMsgLandValueMapChanged, fc4::kMsgSimNewDay, fc4::kMsgPostCityShutdown};
    const intptr_t r = in->event < 3 ? fc4::Dispatch(kEvents[in->event], in->city, in->julian) : -1;
    return r;
}
