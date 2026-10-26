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

// The terrain replacement's view of cISTETerrain (vtable 0x00AB3D18, 98 slots)
// in the GAME's slot order (docs/specs/terrain.md §15.1, §4-§10;
// docs/abi/interfaces.tsv). The surface water's equivalent declaration is
// cFC4ISTESurfaceWater in FC4Interfaces.h (glue16: its IsNavigable pair and
// slot 11's pointer return are wrong in the SDK header).
//
// Do not implement the SDK's cISTETerrain instead: "match the slots, not the
// header". Where the SDK overloads one name MSVC groups the overloads in
// reverse declaration order, which lands them opposite the game in two places.
// Every method here therefore has its OWN name, so the vtable is exactly the
// declaration order and cannot be reordered:
//
//   SDK `GetAltitude(float,float)` then `GetAltitude(int32)` compiles to
//   slots 13 then 12 (reversed) = the game, so that pair happens to match.
//   The three SetAltitude shapes do not: the game is
//       49 SetAltitude(rect*, h, redisplay, data4)   16 bytes / 4 dwords
//       50 SetAltitudeAtVertex(index, h)              8 bytes / 2 dwords
//       51 SetAltitude(x, z, h)                      12 bytes / 3 dwords
//   and the SDK's declaration order (`(index,h)`, `(x,z,h)`-named
//   SetAltitudeAtVertex, `(rect,...)`) reverses 49/50/51 away from that.
//
// Slot 91 is the game's deleting destructor (`void __thiscall dtor(this,
// unsigned flags)`, interfaces.tsv `4 1` = one dword of arguments). A C++
// `virtual ~X()` would take TWO vtable slots on MSVC (complete-object and
// deleting) and push the tail down, so it is declared as one ordinary virtual
// with that ABI instead.
//
// Return channels (interfaces.tsv `ret`); a bool is only reliable in AL:
//   AL:      5, 34-38, 57, 83
//   x87:     6, 12-14, 18-24, 29-33, 47, 55, 61, 92
//   eax:     7, 8, 42, 54
//   void:    51, 68, 87, 88
// Anything else is `eax-or-void` / `via-call` / `edx:eax?` in the table. We
// IMPLEMENT these, we do not read them back, so ret_channel_check (which
// covers our reads of GAME returns) has nothing to say about them; the
// implementations still return exactly the declared type so the game finds
// the value in the register it expects.
#pragma once

#include "cIGZUnknown.h"
#include "SC4Point.h"
#include "SC4Rect.h"
#include "cS3DVector3.h"

#include <cstddef>
#include <cstdint>

class cIGZPersistDBSegment;
class cISTETerrainView;

namespace fc4 {

// The terrain class (city component 49, terrain.md §1) and the surface water
// it owns (terrain +0x24; city slot 83 returns it).
static const uint32_t kFC4_CLSID_cSTETerrain = 0xE98F9525;
static const uint32_t kFC4_CLSID_cSTESurfaceWater = 0x0B8DA7DB;
// The terrain 3D view (terrain.md §1, §2.3 +0xE8). Render only, but the game
// hands the pointer around (zones.md §5.2 takes it at PostCityInit from
// terrain slot 66), so the replacement creates it through the class factory.
static const uint32_t kFC4_CLSID_cSTETerrainView = 0xC9B84E10;

// The terrain's IIDs (§1): 0x00000001, 0xAB953253 and 0xC70BD37E (cISTETerrain,
// the component IID) reach the terrain; 0x452294AA (cIGZMessageTarget2) and
// 0xE4FDA3D4 (cIGZSerializable) reach its other two sub-objects.
static const uint32_t kFC4_IID_cISTETerrain = 0xC70BD37E;
// The surface water's own interface IID. terrain.md §1 does not name it and
// the SDK header has no constant; 0xAB8DA759 is what the city QIs for when
// it builds cSTESurfaceWater (observed in game, glue18).
static const uint32_t kFC4_IID_cISTESurfaceWater = 0xAB8DA759;
static const uint32_t kFC4_IID_cIGZMessageTarget2 = 0x452294AA;
static const uint32_t kFC4_IID_cIGZSerializable = 0xE4FDA3D4;

// tBrushParameters (§9.2, 0x38 bytes). The SDK's cISTETerrain::tBrushParameters
// is the same layout (its static_asserts pin strength at 0x18 and altitude at
// 0x34); restated here so the game's shape is explicit and checked.
struct TBrushParameters {
    float fromX, fromZ, toX, toZ;        // +0x00..+0x0C stroke, metres
    int32_t level;                       // +0x10  0-4; < 0 = the terrain default (+0xBC)
    uint8_t redisplay;                   // +0x14
    uint8_t _pad[3];
    float strength;                      // +0x18  strength scale
    float radiusScaling;                 // +0x1C
    uint32_t data4;                      // +0x20  RedisplayTerrain's kind (§8.2)
    int32_t outX0, outZ0, outX1, outZ1;  // +0x24..+0x30 out: touched vertex rectangle
    float targetHeight;                  // +0x34
};
static_assert(sizeof(TBrushParameters) == 0x38, "TBrushParameters layout (§9.2)");
static_assert(offsetof(TBrushParameters, strength) == 0x18, "TBrushParameters.strength");
static_assert(offsetof(TBrushParameters, data4) == 0x20, "TBrushParameters.data4");
static_assert(offsetof(TBrushParameters, outX0) == 0x24, "TBrushParameters.out");
static_assert(offsetof(TBrushParameters, targetHeight) == 0x34, "TBrushParameters.targetHeight");

// The cost of one mayor-mode brush application (slot 62, `edx:eax?`): a signed
// 64-bit amount in EDX:EAX. The tool charges it against the budget (§9.6).
using BrushCost = int64_t;

// ---- cISTETerrain, slots 0-97 (§15.1) ---------------------------------------------------------------

class cFC4ISTETerrain : public cIGZUnknown {
public:
    virtual bool Init() = 0;     // 3
    virtual bool Shutdown() = 0; // 4

    virtual bool IsInitialized() = 0;                         // 5   AL
    virtual float CellWidth() = 0;                            // 6   x87, 16.0
    virtual uint32_t CellCountX() = 0;                        // 7   eax
    virtual uint32_t CellCountZ() = 0;                        // 8   eax
    virtual int32_t GetVertIndex(int32_t x, int32_t z) const = 0;            // 9
    virtual void GetVertex(int32_t index, int32_t& x, int32_t& z) const = 0; // 10
    virtual bool LocationIsInBounds(float x, float z) const = 0;             // 11  §3

    virtual float GetAltitudeAtFlatIndex(int32_t index) = 0;  // 12  §4.2
    virtual float GetAltitudeAt(float x, float z) = 0;        // 13  §4.1
    virtual float GetAltitudeAtVertex(int32_t x, int32_t z) = 0;  // 14  §4.2
    virtual void GetAltitudes(SC4Rect<int32_t> const& r, float* out, int32_t rowStride,
                              int32_t colStride) = 0;         // 15  §4.2
    virtual void GetWaterDepths(SC4Rect<int32_t> const& r, float* out, int32_t rowStride,
                                int32_t colStride) = 0;       // 16  §5
    virtual void GetCellCornerAltitudes(uint32_t x, uint32_t z, float* out) = 0;  // 17  §4.2
    virtual float GetMaxSurfaceLevel(uint32_t x, uint32_t z) = 0;  // 18  §5
    virtual float GetAltitudeAtNearestGrid(float x, float z) = 0;  // 19  §4.2
    virtual float GetAltitudeCheckSum(SC4Rect<int32_t> const& r) const = 0;  // 20  §4.2
    virtual float MinHeight() = 0;                            // 21  the cached value, §4.6
    virtual float MaxHeight() = 0;                            // 22
    virtual float MinAllowedHeight() = 0;                     // 23  the tuning, §2.4
    virtual float MaxAllowedHeight() = 0;                     // 24
    virtual int32_t GetTerrainGridIndex(float x, float z) = 0;  // 25
    virtual void GetFaceNormal(float x, float z, cS3DVector3& out) = 0;   // 26  §4.3
    virtual void GetLerpedNormal(float x, float z, cS3DVector3& out) = 0; // 27  render
    virtual void GetVertexNormal(uint32_t x, uint32_t z, cS3DVector3& out) = 0;  // 28  render

    virtual float GetSeaLevel() const = 0;                    // 29  §5
    virtual float GetWaterLevelAt(float x, float z) = 0;      // 30  §5
    virtual float GetWaterLevelAtIndex(int32_t index) = 0;    // 31  §5
    virtual float GetLerpedWaterLevel(float x, float z) = 0;  // 32  §5.1
    virtual float GetWaterDepthAtVertexXZ(int32_t x, int32_t z) = 0;  // 33  §5
    virtual bool CellHasWater(uint32_t x, uint32_t z) const = 0;  // 34  AL, §5
    virtual bool CellHasLand(uint32_t x, uint32_t z) const = 0;   // 35  AL, §5
    virtual bool IsLandVertex(uint32_t x, uint32_t z) = 0;    // 36  AL, §5
    virtual bool IsWaterVertex(uint32_t x, uint32_t z) = 0;   // 37  AL, §5
    virtual bool IsSeaVertex(uint32_t x, uint32_t z) = 0;     // 38  AL, §5
    virtual bool AreSeaVerticesIn(SC4Rect<int32_t> const& r) const = 0;  // 39  §5

    virtual void DoPostCityInit() = 0;                        // 40  §12.1
    virtual bool SetTerrainType(int32_t t) = 0;               // 41  §6
    virtual int32_t TerrainType() = 0;                        // 42  eax
    virtual void CreateFlatAreas(bool b) = 0;                 // 43  data4 0
    virtual bool CreateShoreCliffs(bool b) = 0;               // 44  data4 0
    virtual bool SetSeaLevel(float h) = 0;                    // 45  §6
    virtual bool ChangeSeaLevel(bool up) = 0;                 // 46
    virtual float GetDefaultTerrainRaiseLowerAmount() const = 0;  // 47  x87, 20.0
    virtual void ChangeTerrainLevel(bool up, float amount) = 0;  // 48  data4 1

    virtual void SetAltitudeRect(SC4Rect<int32_t> const& r, float h, bool redisplay,
                                 uint32_t data4) = 0;         // 49  §6
    virtual void SetAltitudeAtVertex(int32_t index, float h) = 0;  // 50  §6
    virtual void SetAltitudeXZ(int32_t x, int32_t z, float h) = 0; // 51  §6
    virtual void LockAltitude(SC4Rect<int32_t> const& r, bool lock) = 0;  // 52  §6

    virtual uint32_t NumberOfWaterVerticesIn(SC4Rect<int32_t> const& r) = 0;  // 53  §5
    virtual cISTETerrainView* GetRenderObject() = 0;          // 54  eax, +0xE4 (render)
    virtual float GetSedimentValue(float x, float z) = 0;     // 55  §5
    virtual void StartFlood() = 0;                            // 56  debug
    virtual bool ApplyBrush(uint32_t brushId, TBrushParameters& p) = 0;  // 57  AL, §9.2

    virtual bool GetBrushExemplarID(int32_t type, uint32_t& out) = 0;  // 58  §9.6
    // §9.6. The game pops 8 bytes (2 dwords) on both brush-scrub slots
    // (interfaces.tsv 59 and 60); the SDK declares BeginBrushScrub with one.
    virtual void BeginBrushScrub(uint32_t a, uint32_t b) = 0;  // 59  §9.6
    virtual void EndBrushScrub(uint32_t a, uint32_t b) = 0;   // 60  §9.6
    virtual float BrushRadius(uint32_t brushId) const = 0;    // 61  x87
    virtual BrushCost GetCostPerBrushApplication(uint32_t brushId) = 0;  // 62  EDX:EAX
    virtual bool GetUIElementsForBrush(uint32_t brushId, uint32_t* out) const = 0;  // 63
    virtual void UpdateWaveProducers() = 0;                   // 64  render
    virtual bool IsGoodWaveProducerLocation(float x, float y, float z) const = 0;  // 65  render
    virtual cISTETerrainView* GetView() = 0;                  // 66  eax (render)
    virtual void DoLighting(SC4Rect<int32_t> const& r) = 0;   // 67  render

    virtual void RedisplayTerrainRect(bool terrain, bool water, SC4Rect<int32_t> const& r,
                                      uint32_t data4) = 0;    // 68  void, §8.1
    virtual void RedisplayTerrainPoints(bool terrain, bool water, SC4Point<int32_t> const& a,
                                        SC4Point<int32_t> const& b, uint32_t data4) = 0;  // 69  §8.1

    virtual bool IsTriangulationFlipped(uint32_t x, uint32_t z) = 0;  // 70  §7
    virtual void HideTerrainCell(uint32_t x, uint32_t z, bool hide) = 0;    // 71  §7
    virtual void HideEvenTriangle(uint32_t x, uint32_t z, bool hide) = 0;  // 72  §7
    virtual void HideOddTriangle(uint32_t x, uint32_t z, bool hide) = 0;   // 73  §7
    virtual bool IsEvenTriangleHidden(uint32_t x, uint32_t z) const = 0;    // 74  §7
    virtual bool IsOddTriangleHidden(uint32_t x, uint32_t z) const = 0;     // 75  §7
    virtual bool IsWholeCellHidden(uint32_t x, uint32_t z) const = 0;       // 76  §7
    virtual bool IsPointHidden(float x, float z) const = 0;                 // 77  §7
    virtual void StartAccumulatingTextureUpdates() = 0;       // 78  render
    virtual void StopAccumulatingTextureUpdates() = 0;        // 79  render
    virtual bool IsAccumulatingTextureUpdates() = 0;          // 80  render
    virtual bool SetTerrainCellFlags(uint32_t x, uint32_t z, uint32_t flags) = 0;  // 81  §7
    virtual uint32_t GetTerrainCellFlags(uint32_t x, uint32_t z) = 0;              // 82  §7

    virtual bool GetWaterSurfaceDisplayState() = 0;           // 83  AL, +0xDD
    virtual void SetWaterSurfaceDisplayState(bool on) = 0;    // 84  §8.4
    virtual bool GetPlumbingInfoDisplayState() = 0;           // 85  a global byte
    virtual void SetPlumbingInfoDisplayState(bool on) = 0;    // 86
    virtual void BeginLongTerrainOperation() = 0;             // 87  void, §8.3
    virtual void EndLongTerrainOperation() = 0;               // 88  void, §8.3

    virtual void ReconcileEdgesWithNeighboringCities(bool a, bool b, bool c) = 0;  // 89
    virtual bool SaveObliterated(cIGZPersistDBSegment* seg) = 0;  // 90
    // 91: the game's deleting destructor, `void __thiscall dtor(this, unsigned
    // flags)`. One vtable slot, not a C++ virtual destructor (which would take
    // two). `flags & 1` means "destroy, then operator delete".
    virtual void DeletingDestructor(uint32_t flags) = 0;
    virtual float GetWaterDepthAtFlatIndex(int32_t index) = 0;  // 92  x87, §5
    virtual bool FlipInternalTriangulation(uint32_t x, uint32_t z, bool flip) = 0;  // 93  §7
    virtual bool FlipExternalTriangulation(uint32_t x, uint32_t z, bool flip) = 0;  // 94  §7
    virtual bool LoadObliterated(cIGZPersistDBSegment* seg) = 0;  // 95
    virtual bool Load(cIGZPersistDBSegment* seg) = 0;         // 96  §13.1
    virtual bool Save(cIGZPersistDBSegment* seg) = 0;         // 97  §13.1
};

// ---- the object's own fields (§2.3) ---------------------------------------------------------------
// The native view reads these records directly (terrain.md §2.2,
// terrain-view.md §4.4.1), through the PRIMARY terrain at +0x6C.
struct TerrainDrawVertex {
    float altitude;
    float normal[3];
    uint8_t reserved[6];
    uint16_t layers[6];
    uint16_t flags;
};
struct WaterDrawVertex {
    float depth;
    uint8_t reserved[12];
    float velocity[3];
    float sediment;
    uint32_t tail;
};
static_assert(sizeof(TerrainDrawVertex) == 0x24, "native terrain vertex stride");
static_assert(offsetof(TerrainDrawVertex, flags) == 0x22, "native diagonal flags");
static_assert(sizeof(WaterDrawVertex) == 0x24, "native water vertex stride");
static_assert(offsetof(WaterDrawVertex, sediment) == 0x1C, "native sediment");
//
// The game reads these directly, not only through the slots: §1 says
// QueryInterface returns `this`, `this + 4` and `this + 8`, which is three
// vptrs followed by the fields at +0x0C - exactly what MSVC lays out for
// `class X : public cFC4ISTETerrain, public cIGZMessageTarget2,
// public cIGZSerializable`. cFC4STETerrain keeps its members in THIS order so
// the offsets match the original's. In-game evidence that they matter:
// cSC4City::Init dereferences the pointer at terrain +0x24 (the surface water).
//
// Every offset below is the struct-relative one; the object offset is +0x0C.
struct TerrainFields {
    uint8_t initialized;                 // 0x00  (object +0x0C)  slot 5
    uint8_t _p01[7];                     // 0x01..0x07
    uint8_t postCityInit;                // 0x08  (+0x14)  once-only guard
    uint8_t _p09[3];                     // 0x09..0x0B
    int32_t terrainType;                 // 0x0C  (+0x18)  slots 41/42
    void* city;                          // 0x10  (+0x1C)
    uint32_t _p14;                       // 0x14  (+0x20)
    void* surfaceWater;                  // 0x18  (+0x24)  the surface water object
    uint32_t cellsX;                     // 0x1C  (+0x28)  slot 7
    uint32_t cellsZ;                     // 0x20  (+0x2C)  slot 8
    uint32_t cellsXm1;                   // 0x24  (+0x30)
    uint32_t cellsZm1;                   // 0x28  (+0x34)
    uint32_t vertsX;                     // 0x2C  (+0x38)
    uint32_t vertsZ;                     // 0x30  (+0x3C)
    int32_t cityRect[4];                 // 0x34  (+0x40..+0x4C)  (0, 0, cellsX, cellsZ)
    float sizeX;                         // 0x44  (+0x50)
    float sizeZ;                         // 0x48  (+0x54)
    int32_t citySizeType;                // 0x4C  (+0x58)
    uint32_t vertexCount;                // 0x50  (+0x5C)
    uint32_t _p54[3];                    // 0x54  (+0x60..+0x68)
    void* vertices;                      // 0x60  (+0x6C)  the vertex array
    float cachedMin;                     // 0x64  (+0x70)  slot 21, §4.6
    float cachedMax;                     // 0x68  (+0x74)  slot 22
    float saved78;                       // 0x6C  (+0x78)  tuning 0x48CD7B20 (308)
    float seaLevel;                      // 0x70  (+0x7C)  slot 29
    uint32_t _p74;                       // 0x74  (+0x80)
    int32_t neighbourOffset[8];          // 0x78  (+0x84..+0xA3)
    uint32_t _p98[6];                    // 0x98  (+0xA4..+0xBB)
    int32_t defaultBrushLevel;           // 0xB0  (+0xBC)  §2.3, TR9
    uint8_t accumulateTexture;           // 0xB4  (+0xC0)
    uint8_t _pB5[0x17];                  // 0xB5..0xCB
    uint32_t reconcileTick;              // 0xCC  (+0xD8)
    uint8_t _pD0;                        // 0xD0  (+0xDC)
    uint8_t waterDisplayState;           // 0xD1  (+0xDD)  slot 83
    uint8_t _pD2[0x06];                  // 0xD2..0xD7
    void* renderObject;                  // 0xD8  (+0xE4)  slot 54 returns it
    void* view;                          // 0xDC  (+0xE8)  the terrain 3D view
    uint8_t viewReady;                   // 0xE0  (+0xEC)  §8.1 gate
    uint8_t _pE1[3];
};
static_assert(offsetof(TerrainFields, initialized) == 0x00, "+0x0C");
static_assert(offsetof(TerrainFields, postCityInit) == 0x08, "+0x14");
static_assert(offsetof(TerrainFields, terrainType) == 0x0C, "+0x18");
static_assert(offsetof(TerrainFields, city) == 0x10, "+0x1C");
static_assert(offsetof(TerrainFields, surfaceWater) == 0x18, "+0x24");
static_assert(offsetof(TerrainFields, cellsX) == 0x1C, "+0x28");
static_assert(offsetof(TerrainFields, cellsZ) == 0x20, "+0x2C");
static_assert(offsetof(TerrainFields, vertsX) == 0x2C, "+0x38");
static_assert(offsetof(TerrainFields, sizeX) == 0x44, "+0x50");
static_assert(offsetof(TerrainFields, vertexCount) == 0x50, "+0x5C");
static_assert(offsetof(TerrainFields, vertices) == 0x60, "+0x6C");
static_assert(offsetof(TerrainFields, cachedMin) == 0x64, "+0x70");
static_assert(offsetof(TerrainFields, seaLevel) == 0x70, "+0x7C");
static_assert(offsetof(TerrainFields, defaultBrushLevel) == 0xB0, "+0xBC");
static_assert(offsetof(TerrainFields, reconcileTick) == 0xCC, "+0xD8");
static_assert(offsetof(TerrainFields, waterDisplayState) == 0xD1, "+0xDD");
static_assert(offsetof(TerrainFields, renderObject) == 0xD8, "+0xE4");
static_assert(offsetof(TerrainFields, viewReady) == 0xE0, "+0xEC");
static_assert(sizeof(TerrainFields) == 0xE4, "TerrainFields covers +0x0C..+0xEF");

}  // namespace fc4
