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

// The weather simulator's GZCOM class (docs/specs/weather.md §1). The model
// lives in the Rust core behind fc4.h; this class adapts the SDK interfaces to
// it and owns the game services it uses.
//
// The map view reads the moisture grid's memory directly through
// GetMoistureValueMapPtr (§11.2), so this class keeps an
// SC4Vector<SC4Vector<uint8_t>>-compatible mirror of the core's grid that stays
// at the same address while the city is loaded.
//
// The subsystem is OFF by default: FreeCity4.ini `enable = weather` turns it
// on (FC4Subsystems.h); `disable` still wins.
#pragma once

#include "FC4Interfaces.h"
#include "cIGZMessageTarget.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#include <cstdint>
#include <vector>

class cIGZMessageServer2;
class cISC4City;
class cISTETerrain;
class cISC4Simulator;

static const uint32_t kFC4_CLSID_cSC4WeatherSimulator = 0x2990C1BC;
static const uint32_t kFC4_IID_cISC4WeatherSimulator = 0xC6C66245;
static const uint32_t kFC4_IID_cIGZMessageTarget = 0xE98B2F57;

// The memory layout of an SC4Vector: begin, end, end of storage.
struct FC4ByteRow {
    uint8_t* begin;
    uint8_t* end;
    uint8_t* capacity;
};
struct FC4ByteGridVector {
    FC4ByteRow* begin;
    FC4ByteRow* end;
    FC4ByteRow* capacity;
};
static_assert(sizeof(void*) != 4 || sizeof(FC4ByteGridVector) == 12, "SC4Vector layout: three pointers");

class cFC4WeatherSimulator final : public cFC4ISC4WeatherSimulator,
                                   public cIGZMessageTarget2,
                                   public cIGZMessageTarget,
                                   public cIGZSerializable {
public:
    explicit cFC4WeatherSimulator(bool recordOnly = false);
    ~cFC4WeatherSimulator();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4WeatherSimulator (§4)
    bool Init() override;
    bool Shutdown() override;
    bool DoPostCityInit() override;
    float CellWidthX() override;
    float CellWidthZ() override;
    uint32_t CellCountX() override;
    uint32_t CellCountZ() override;
    int32_t PositionToCell(float x, float z, int32_t& cx, int32_t& cz) override;
    void CellToPosition(int32_t cx, int32_t cz, float& x, float& z) override;
    void CellToStandardCityCell(int32_t cx, int32_t cz, int32_t& sx, int32_t& sz) override;
    int64_t StandardCityCellToCell(int32_t sx, int32_t sz, int32_t& cx, int32_t& cz) override;
    int32_t GetCurrentSeason() override;
    int32_t GetCurrentSeasonMonth() override;
    void UpdateMoisture(SC4Rect<int32_t> const& rect) override;
    uint8_t GetMoistureValue(float x, float z) override;
    void GetMoistureValueMapPtr(FC4ByteGridVector*& out) override;
    uint32_t GetMoistureType(float x, float z) override;  // a clean EAX (flora.md §6.2, FQ5)
    void SetManualMoistureAddition(float x0, float z0, float x1, float z1, float v) override;
    float GetHumidity(float x, float z) override;
    float GetAmbientWind(cS3DVector2& wind) override;
    float GetWindAtCell(int32_t x, int32_t z, cS3DVector2& wind) override;

    // cIGZMessageTarget2 (§5)
    bool DoMessage(cIGZMessage2* pMessage) override;
    // cIGZMessageTarget (§1: returns true, does nothing)
    bool DoMessage(cGZMessage& message) override;

    // cIGZSerializable (§10)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // For the host callbacks (cFC4Weather.cpp).
    cISC4City* City() const { return city_; }
    bool RecordOnly() const { return recordOnly_; }
    fc4_weather* Core() const { return core_; }
    cISTETerrain* Terrain() const { return terrain_; }
    void MarkMoistureChanged() { moistureDirty_ = true; }

private:
    void RegisterAgents(bool add);
    void SyncMoisture();
    void LogSeasonChange();

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4_weather_host host_{};
    fc4_weather* core_ = nullptr;
    bool initialized_ = false;
    bool firstInitDone_ = false;
    cISC4City* city_ = nullptr;
    cISTETerrain* terrain_ = nullptr;
    cISC4Simulator* simulator_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    bool agentsRegistered_ = false;
    int32_t lastSeason_ = -1;

    // The moisture grid mirror (§11.2): rows indexed by x.
    bool recordOnly_ = false;
    bool moistureDirty_ = true;
    std::vector<uint8_t> cells_;
    std::vector<FC4ByteRow> rows_;
    FC4ByteGridVector grid_{};
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4WeatherSimulator();
