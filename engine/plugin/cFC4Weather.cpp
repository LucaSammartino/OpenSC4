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

#include "cFC4Weather.h"

#include "FC4Checked.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4Seed.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4Region.h"
#include "cISC4Simulator.h"
#include "cISTETerrain.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "cS3DVector2.h"
#include "cS3DVector3.h"

#include <windows.h>

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;

// Messages (§5).
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgPostCityInit = 0x26D31EC1;
constexpr uint32_t kMsgSeasonChanged = 0x6716A4E5;
constexpr uint32_t kMsgMoistureMapUpdated = 0x09B6D58E;

// Agents (§5): month, week, day; flags 2 (established cities).
constexpr uint32_t kAgentLevels[] = {6, 5, 4};
constexpr uint32_t kAgentFlags = 2;
const char kAgentName[] = "cSC4WeatherSimulator";

// Tuning exemplar (§9) and the properties the core reads.
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0x7A4A8458;
constexpr uint32_t kTuningInstance = 0x1A2FDB6B;
using fc4::props::Kind;
const fc4::props::Spec kTuningProps[] = {
    {0xA7607D70, Kind::I64}, {0xA7607D71, Kind::I64}, {0xA7607D72, Kind::I64}, {0xA7607D73, Kind::F32},
    {0xA7607D74, Kind::F32}, {0xA7607D75, Kind::F32}, {0xA7607D76, Kind::I64}, {0xA7607D77, Kind::F32},
    {0xA7607D78, Kind::F32}, {0xA7607D79, Kind::F32}, {0xA7607D7A, Kind::F32}, {0xA7607D7B, Kind::F32},
    {0xA7607D7C, Kind::F32}, {0xA7607D7D, Kind::F32}, {0xA7607D7E, Kind::F32}, {0xA7607D7F, Kind::F32},
    {0xA7607D80, Kind::U32}, {0xA7607D81, Kind::U32}, {0xA7607D86, Kind::F32}, {0xA7607D87, Kind::F32},
    {0xA7607D88, Kind::F32}, {0xA7607D89, Kind::F32},
};

cFC4WeatherSimulator* Self(void* u) { return static_cast<cFC4WeatherSimulator*>(u); }

cISC4App* App() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app != nullptr && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) && sc4 != nullptr) {
        FC4_ACQUIRED(sc4, "QueryInterface");
        return sc4;  // the caller releases it
    }
    return nullptr;
}

// ---- host callbacks (fc4_weather_host) -------------------------------------------------------------

void CitySize(void* u, float* x, float* z) {
    cISC4City* c = Self(u)->City();
    *x = c ? c->SizeX() : 0.0f;
    *z = c ? c->SizeZ() : 0.0f;
}

void CityCellWidth(void* u, float* x, float* z) {
    cISC4City* c = Self(u)->City();
    if (c == nullptr) return;  // the core's default (16)
    *x = c->CellWidthX();
    *z = c->CellWidthZ();
}

int32_t WorldHemisphere(void* u) {
    cISC4City* c = Self(u)->City();
    return c ? static_cast<int32_t>(c->GetWorldHemisphere()) : 1;
}

int32_t SimMonth(void* u) {
    cISC4City* c = Self(u)->City();
    cFC4ISC4Simulator* s = c ? fc4::AsGame(c->GetSimulator()) : nullptr;
    if (s == nullptr) return 1;
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
    return static_cast<int32_t>(m.get());
}

bool WaterPrefs(void*, uint8_t* lo, uint8_t* hi) {
    cISC4App* app = App();
    if (app == nullptr) return false;
    cISC4Region* region = app->GetRegion();
    bool ok = false;
    if (region != nullptr) {
        uint8_t a = 60, b = 100;
        region->GetWaterPrefs(a, b);
        *lo = a;
        *hi = b;
        ok = true;
    }
    FC4_RELEASE(app);
    return ok;
}

bool TerrainAvailable(void* u) { return Self(u)->Terrain() != nullptr; }

float AltitudeAtVertex(void* u, int32_t x, int32_t z) {
    cISTETerrain* t = Self(u)->Terrain();
    return t ? t->GetAltitudeAtVertex(x, z) : 0.0f;
}

void VertexNormal(void* u, int32_t x, int32_t z, float* xyz) {
    cISTETerrain* t = Self(u)->Terrain();
    if (t == nullptr) return;
    cS3DVector3 n(0.0f, 1.0f, 0.0f);
    t->GetVertexNormal(static_cast<uint32_t>(x), static_cast<uint32_t>(z), n);
    xyz[0] = n.fX;
    xyz[1] = n.fY;
    xyz[2] = n.fZ;
}

bool CellHasWater(void* u, int32_t x, int32_t z) {
    cISTETerrain* t = Self(u)->Terrain();
    return t != nullptr && t->CellHasWater(static_cast<uint32_t>(x), static_cast<uint32_t>(z));
}

float Altitude(void* u, float x, float z) {
    cISTETerrain* t = Self(u)->Terrain();
    return t ? t->GetAltitude(x, z) : 0.0f;  // compiled slot 13 (foundations §16)
}

float SeaLevel(void* u) {
    cISTETerrain* t = Self(u)->Terrain();
    return t ? t->GetSeaLevel() : 0.0f;
}

// void1 = the weather simulator as cIGZUnknown, data2 (§5).
void Post(void* u, uint32_t type, uint32_t data2) {
    if (Self(u)->RecordOnly()) return;
    if (type == kMsgMoistureMapUpdated) Self(u)->MarkMoistureChanged();
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* msg = cFC4Message2Standard::New();
    msg->AddRef();
    msg->SetType(type);
    msg->SetVoid1(static_cast<cIGZUnknown*>(static_cast<cFC4ISC4WeatherSimulator*>(Self(u))));
    if (type == kMsgSeasonChanged) msg->SetData2(static_cast<intptr_t>(data2));
    ms->MessagePost(msg, false);
    msg->Release();
}

fc4_weather_host MakeHost(cFC4WeatherSimulator* self) {
    fc4_weather_host h{};
    h.struct_size = sizeof(h);
    h.user = self;
    h.city_size = &CitySize;
    h.city_cell_width = &CityCellWidth;
    h.world_hemisphere = &WorldHemisphere;
    h.sim_month = &SimMonth;
    h.water_prefs = &WaterPrefs;
    h.terrain_available = &TerrainAvailable;
    h.altitude_at_vertex = &AltitudeAtVertex;
    h.vertex_normal = &VertexNormal;
    h.cell_has_water = &CellHasWater;
    h.altitude = &Altitude;
    h.sea_level = &SeaLevel;
    h.post = &Post;
    return h;
}

}  // namespace

cIGZUnknown* CreateFC4WeatherSimulator() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4WeatherSimulator*>(new cFC4WeatherSimulator()));
}

cFC4WeatherSimulator::cFC4WeatherSimulator(bool recordOnly) : recordOnly_(recordOnly) {
    host_ = MakeHost(this);
    // Seed(0xFFFFFFFF) in the original (§3); ours takes derive(S, GZCLSID) of
    // the session seed (foundations §6.3, FC4Seed.h).
    core_ = fc4_weather_create(&host_, fc4::SessionSeed());
    if (core_ == nullptr) fc4::Log("cFC4WeatherSimulator: core creation failed");
    fc4::Log("cFC4WeatherSimulator(%p) created, core %p", static_cast<void*>(this), static_cast<void*>(core_));
}

cFC4WeatherSimulator::~cFC4WeatherSimulator() {
    fc4_weather_destroy(core_);
    core_ = nullptr;
    fc4::Log("cFC4WeatherSimulator destroyed");
}

bool cFC4WeatherSimulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kFC4_IID_cISC4WeatherSimulator:
            *ppvObj = static_cast<cFC4ISC4WeatherSimulator*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case kFC4_IID_cIGZMessageTarget:
            *ppvObj = static_cast<cIGZMessageTarget*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        case GZIID_cIGZUnknown:
            *ppvObj = static_cast<cIGZUnknown*>(static_cast<cFC4ISC4WeatherSimulator*>(this));
            break;
        default:
            *ppvObj = nullptr;
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4WeatherSimulator::AddRef() { return ++refCount_; }

// [choice] As the other simulators: at a count of 1 (or below) Shutdown,
// destroy, return 0 (the weather spec does not describe Release).
uint32_t cFC4WeatherSimulator::Release() {
    if (refCount_ > 1) return --refCount_;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    refCount_ = 0;
    Shutdown();
    delete this;
    return 0;
}

// ---- lifecycle (§8) ---------------------------------------------------------------------------------

bool cFC4WeatherSimulator::Init() {
    FC4_TRACE("WeatherSimulator::Init");
    if (initialized_) return true;
    // Step 2: PostCityInit, the city.
    if (!recordOnly_ && msgServer_ == nullptr) {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            msgServer_ = ms;
            FC4_ADDREF(msgServer_);
            msgServer_->AddNotification(static_cast<cIGZMessageTarget2*>(this), kMsgPostCityInit);
        }
    }
    if (city_ == nullptr) {
        if (cISC4App* app = App()) {
            city_ = app->GetCity();
            FC4_ADDREF(city_);
            FC4_RELEASE(app);
        }
    }
    // Step 4 (terrain first: the core's moisture variation reads no terrain,
    // but the water scan of the first month does).
    if (city_ != nullptr && terrain_ == nullptr) {
        terrain_ = city_->GetTerrain();
        FC4_ADDREF(terrain_);
    }
    // Steps 1, 3, 5-8 (core): tuning, grids, iterator, ambient values.
    fc4::props::Exemplar ex;
    const bool haveTuning = ex.Load(kTuningType, kTuningGroup, kTuningInstance);
    if (!haveTuning) fc4::Log("weather: tuning exemplar unavailable; the current values stay");
    fc4::props::CoreProps tuning(
        haveTuning ? fc4::props::ToCore(ex.Holder(), kTuningProps, sizeof(kTuningProps) / sizeof(kTuningProps[0]))
                   : nullptr);
    const bool ok = fc4_weather_init(core_, tuning.Get());
    // Step 4: on the first Init of this object, the simulator and the agents.
    if (!firstInitDone_) {
        firstInitDone_ = true;
        RegisterAgents(true);
    }
    initialized_ = true;
    moistureDirty_ = true;
    SyncMoisture();

    fc4_weather_state st{};
    st.struct_size = sizeof(st);
    fc4_weather_get_state(core_, &st);
    lastSeason_ = st.last_season;
    fc4::Log("weather Init: ok=%d city=%p terrain=%p cells %dx%d, %d per week, loaded=%d, season %d, ambient "
             "temp %.2f moisture %.3f",
             ok ? 1 : 0, static_cast<void*>(city_), static_cast<void*>(terrain_), st.count_x, st.count_z,
             st.cells_per_week, st.loaded_from_save, st.last_season, st.ambient_temp, st.ambient_moisture);
    return true;
}

bool cFC4WeatherSimulator::Shutdown() {
    FC4_TRACE("WeatherSimulator::Shutdown");
    if (!initialized_) return true;
    initialized_ = false;
    if (!destroying_) AddRef();  // keep alive while the simulator and the server let go
    FC4_RELEASE(terrain_);
    terrain_ = nullptr;
    RegisterAgents(false);
    FC4_RELEASE(city_);
    city_ = nullptr;
    if (msgServer_ != nullptr) {
        msgServer_->RemoveNotification(static_cast<cIGZMessageTarget2*>(this), kMsgPostCityInit);
        FC4_RELEASE(msgServer_);
        msgServer_ = nullptr;
    }
    fc4_weather_shutdown(core_);
    fc4::Log("weather Shutdown");
    if (!destroying_) Release();
    return true;
}

void cFC4WeatherSimulator::RegisterAgents(bool add) {
    if (recordOnly_) return;
    cIGZMessageTarget2* self = static_cast<cIGZMessageTarget2*>(this);
    if (add) {
        if (simulator_ == nullptr && city_ != nullptr) {
            simulator_ = city_->GetSimulator();
            FC4_ADDREF(simulator_);
        }
        cFC4ISC4Simulator* sim = fc4::AsGame(simulator_);
        if (sim == nullptr) {
            fc4::Log("weather: no simulator; agents not registered");
            return;
        }
        const cRZBaseString name(kAgentName);
        for (uint32_t level : kAgentLevels) sim->AddAgent(self, level, name, kAgentFlags);
        agentsRegistered_ = true;
    } else {
        if (agentsRegistered_ && simulator_ != nullptr) fc4::AsGame(simulator_)->RemoveAgentAllLevels(self);
        agentsRegistered_ = false;
        FC4_RELEASE(simulator_);
        simulator_ = nullptr;
    }
}

bool cFC4WeatherSimulator::DoPostCityInit() {
    const bool r = fc4_weather_do_post_city_init(core_);
    SyncMoisture();
    LogSeasonChange();
    return r;
}

// ---- the moisture grid mirror (§11.2) ---------------------------------------------------------------

void cFC4WeatherSimulator::SyncMoisture() {
    if (!moistureDirty_ || core_ == nullptr) return;
    moistureDirty_ = false;
    int32_t nx = 0, nz = 0;
    const size_t n = fc4_weather_moisture_grid(core_, nullptr, 0, &nx, &nz);
    if (n != cells_.size() || static_cast<size_t>(nx) != rows_.size()) {
        // A new shape (Init of a new city, or a load): the only time the
        // mirror moves.
        cells_.assign(n, 0);
        rows_.assign(static_cast<size_t>(nx), FC4ByteRow{});
        for (int32_t x = 0; x < nx; ++x) {
            uint8_t* row = cells_.data() + static_cast<size_t>(x) * static_cast<size_t>(nz);
            rows_[static_cast<size_t>(x)] = FC4ByteRow{row, row + nz, row + nz};
        }
        grid_ = FC4ByteGridVector{rows_.data(), rows_.data() + rows_.size(), rows_.data() + rows_.size()};
        fc4::Log("weather: moisture grid mirror %dx%d at %p", nx, nz, static_cast<void*>(&grid_));
    }
    if (n > 0) fc4_weather_moisture_grid(core_, cells_.data(), cells_.size(), &nx, &nz);
}

void cFC4WeatherSimulator::LogSeasonChange() {
    fc4_weather_state st{};
    st.struct_size = sizeof(st);
    if (!fc4_weather_get_state(core_, &st)) return;
    if (st.last_season != lastSeason_) {
        static const char* const kNames[] = {"winter", "spring", "summer", "fall"};
        const char* name = (st.last_season >= 0 && st.last_season < 4) ? kNames[st.last_season] : "?";
        fc4::Log("weather: season %s (ambient temp %.2f, moisture %.3f, wind %.2f)", name, st.ambient_temp,
                 st.ambient_moisture, st.wind_speed);
        lastSeason_ = st.last_season;
    }
}

// ---- cISC4WeatherSimulator (§4) -----------------------------------------------------------------------

float cFC4WeatherSimulator::CellWidthX() { return fc4_weather_cell_width(core_, 0); }
float cFC4WeatherSimulator::CellWidthZ() { return fc4_weather_cell_width(core_, 1); }
uint32_t cFC4WeatherSimulator::CellCountX() { return fc4_weather_cell_count(core_, 0); }
uint32_t cFC4WeatherSimulator::CellCountZ() { return fc4_weather_cell_count(core_, 1); }

int32_t cFC4WeatherSimulator::PositionToCell(float x, float z, int32_t& cx, int32_t& cz) {
    return fc4_weather_position_to_cell(core_, x, z, &cx, &cz);
}

// §4 (corrected 2026-10-14): returns nothing and leaves nothing on the x87
// stack; the results are the out parameters only.
void cFC4WeatherSimulator::CellToPosition(int32_t cx, int32_t cz, float& x, float& z) {
    fc4_weather_cell_to_position(core_, cx, cz, &x, &z);
}

void cFC4WeatherSimulator::CellToStandardCityCell(int32_t cx, int32_t cz, int32_t& sx, int32_t& sz) {
    fc4_weather_cell_to_standard_city_cell(core_, cx, cz, &sx, &sz);
}

// [choice] Returns cz (the original leaves a 64-bit value in EDX:EAX; no
// caller reads it).
int64_t cFC4WeatherSimulator::StandardCityCellToCell(int32_t sx, int32_t sz, int32_t& cx, int32_t& cz) {
    fc4_weather_standard_city_cell_to_cell(core_, sx, sz, &cx, &cz);
    return cz;
}

int32_t cFC4WeatherSimulator::GetCurrentSeason() { return fc4_weather_season(core_); }
int32_t cFC4WeatherSimulator::GetCurrentSeasonMonth() { return fc4_weather_season_month(core_); }

void cFC4WeatherSimulator::UpdateMoisture(SC4Rect<int32_t> const& r) {
    fc4_weather_update_moisture(core_, r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY);
    SyncMoisture();
}

uint8_t cFC4WeatherSimulator::GetMoistureValue(float x, float z) { return fc4_weather_moisture_value(core_, x, z); }

// §11.2 [compat]: the address of the grid object goes into the caller's variable.
void cFC4WeatherSimulator::GetMoistureValueMapPtr(FC4ByteGridVector*& out) {
    SyncMoisture();
    out = &grid_;
}

// The 0..15 value is returned zero-extended: the original flora adds all of
// EAX to a table offset (flora.md §6.2, FQ5).
uint32_t cFC4WeatherSimulator::GetMoistureType(float x, float z) {
    return static_cast<uint32_t>(fc4_weather_moisture_type(core_, x, z));
}

void cFC4WeatherSimulator::SetManualMoistureAddition(float x0, float z0, float x1, float z1, float v) {
    fc4_weather_set_manual_moisture_addition(core_, x0, z0, x1, z1, v);
}

float cFC4WeatherSimulator::GetHumidity(float x, float z) { return fc4_weather_humidity(core_, x, z); }

float cFC4WeatherSimulator::GetAmbientWind(cS3DVector2& wind) {
    float v[2] = {0.0f, 0.0f};
    const float speed = fc4_weather_ambient_wind(core_, v);
    wind.fX = v[0];
    wind.fY = v[1];
    return speed;
}

float cFC4WeatherSimulator::GetWindAtCell(int32_t, int32_t, cS3DVector2& wind) { return GetAmbientWind(wind); }

// ---- messages (§5) --------------------------------------------------------------------------------------

bool cFC4WeatherSimulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("weather DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    const uint32_t type = pMessage->GetType();
    uint32_t data1 = 0, data3 = 0;
    cIGZMessage2Standard* m = nullptr;
    if (pMessage->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) && m != nullptr) {
        FC4_ACQUIRED(m, "QueryInterface");
        data1 = static_cast<uint32_t>(m->GetData1());
        data3 = static_cast<uint32_t>(m->GetData3());
        FC4_RELEASE(m);
    }
    if (type == kMsgPostCityInit) {
        DoPostCityInit();  // §5.4
        return true;
    }
    fc4_weather_handle_message(core_, type, data1, data3);
    SyncMoisture();
    if (type != kMsgSimNewDay) LogSeasonChange();
    return true;
}

bool cFC4WeatherSimulator::DoMessage(cGZMessage&) { return true; }

// ---- persistence (§10) ------------------------------------------------------------------------------------

bool cFC4WeatherSimulator::Write(cIGZOStream& s) {
    FC4_TRACE("WeatherSimulator::Write");
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_weather_write(core_, a.Get());
    if (!recordOnly_) fc4::Log("weather Write -> %d", ok ? 1 : 0);
    return ok;
}

bool cFC4WeatherSimulator::Read(cIGZIStream& s) {
    FC4_TRACE("WeatherSimulator::Read");
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_weather_read(core_, a.Get());
    moistureDirty_ = true;
    SyncMoisture();
    fc4_weather_state st{};
    st.struct_size = sizeof(st);
    fc4_weather_get_state(core_, &st);
    lastSeason_ = st.last_season;
    fc4::Log("weather Read -> %d (cells %dx%d, season %d, wind month %d)", ok ? 1 : 0, st.count_x, st.count_z,
             st.last_season, st.wind_month);
    return ok;
}

uint32_t cFC4WeatherSimulator::GetGZCLSID() { return kFC4_CLSID_cSC4WeatherSimulator; }
