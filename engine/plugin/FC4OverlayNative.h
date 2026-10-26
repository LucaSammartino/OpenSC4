// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "cIGZUnknown.h"
#include <cstddef>

class cIGZString;
constexpr uint32_t kFC4_IID_NativeOverlay = 0x895D6544;

// overlay-manager section 2.7.1 (OV16): slot3 writes exactly two uint32_t
// words through one stack pointer, with no return value. Slot4 takes no
// arguments; its opaque driver handle is not an acquired reference.
class FC4IOverlaySource : public cIGZUnknown {
public:
    virtual void Dimensions(uint32_t* widthHeight) = 0;
    virtual uint32_t Texture() = 0;
};

// overlay-manager.md §3. Do not inherit the SDK overlay declaration: its
// overloaded slots 3..6 and output-first slot 24 describe a different ABI.
class FC4INativeOverlay : public cIGZUnknown {
public:
    virtual uint32_t AddSource(const void*, const float*, float, float) = 0;
    virtual uint32_t AddBinding(const void*, const float*, float, float) = 0;
    virtual uint32_t AddKey(const void*, const float*, float, float) = 0;
    virtual uint32_t AddInstance(uint32_t, const float*, float, float) = 0;
    virtual uint32_t AddRound(uint32_t, const float*, float, float) = 0;
    virtual uint32_t AddCity(uint32_t) = 0;
    virtual uint32_t AddKeyShadow(const void*, const float*, float, float, uint32_t) = 0;
    virtual uint32_t AddGroupShadow(uint32_t, const float*, float, float, uint32_t) = 0;
    virtual uint32_t AddModel(const void*, int32_t, const void*, float*, const float*, const void*) = 0;
    virtual void Move(uint32_t, const float*) = 0;
    virtual void Update(uint32_t, const float*) = 0;
    virtual void ReadPlacement(uint32_t, float*) const = 0;
    virtual void Erase(uint32_t) = 0;
    virtual void Show(uint32_t, bool) = 0;
    virtual void Alpha(uint32_t, float) = 0;
    virtual void Color(uint32_t, const float*) = 0;
    virtual void DrawMode(uint32_t, uint32_t) = 0;
    virtual void SelfLighting(uint32_t, uint32_t) = 0;
    virtual void TextureScale(uint32_t, float, float) = 0;
    virtual bool Shown(uint32_t) const = 0;
    virtual float GetAlpha(uint32_t) const = 0;
    virtual float* GetColor(float*, uint32_t) const = 0;
    virtual uint8_t GetDrawMode(uint32_t) const = 0;
    virtual bool SelfLit(uint32_t) const = 0;
    virtual void Stats(cIGZString&) = 0;
};

// overlay-manager §1 and functions.tsv 0x00739490: the draw registry receives
// manager+4, slot3 takes one context pointer and returns AL.
class FC4IOverlayDraw : public cIGZUnknown {
public:
    virtual bool Draw(void* context) = 0;
};

// Private static-library service transport, not a public C ABI export.
// Returned resources each transfer ONE acquired reference. kind: binding=0,
// source=1. The resource callback acquires direct inputs; lookup owns its result.
struct FC4OverlayResource {
    uintptr_t object;
    // Independent of the service context; valid through releaseResource even
    // after Shutdown has released view/terrain/factory service references.
    uintptr_t context;
    uint32_t width, height;
};
// Callback table is immutable for a context's lifetime. Resource contexts own
// their separate lifetimes and must not retain the manager's view/terrain.
struct FC4OverlayHost {
    uintptr_t context;
    void (*retain)(uintptr_t);
    void (*release)(uintptr_t);
    uint32_t nx, nz;
    float cellWidth;
    int32_t (*setting)(uintptr_t, uint32_t);
    uint32_t (*lookup)(uintptr_t, uint32_t, uintptr_t, FC4OverlayResource*);
    uint32_t (*resource)(uintptr_t, uintptr_t, uint32_t, FC4OverlayResource*);
    void (*releaseResource)(uintptr_t, uintptr_t, uint32_t);
    uint32_t (*keyWord)(uintptr_t);
    void (*redisplay)(uintptr_t, int32_t, uint32_t, const int32_t*, int32_t);
    void (*subscription)(uintptr_t, uint32_t, uint32_t);
    uint32_t (*projection)(uintptr_t, int32_t, const void*, float*, const void*, float*);
    uint32_t (*terrainRect)(uintptr_t, int32_t, const void*, const float*, int32_t*);
    void (*direction)(uintptr_t, float*);
};

// Test overrides are private to the FC4 factory. Production uses the counted
// binding entry points from texture-bindings §2 behind the compat version gate.
struct FC4OverlayBindingCalls {
    uint32_t (*addRef)(void*);
    uint32_t (*release)(void*);
};

// Private portable draw transport, NOT the game's draw-context layout. The
// producer supplies raw inputs; Rust selects records and composes transforms.
enum class FC4OverlayCommandKind : uint32_t {
    DepthOffset, DrawState, Transparency, Shade, VertexColors, ResetTransparency,
    ResetShade, Transform, ClearTransform, Filtering, Wrap, ShadowCombiner,
    TextureEnabled, Environment, AlphaTest, DeviceFlag7
};
struct FC4OverlayCommand {
    FC4OverlayCommandKind kind;
    uint32_t a, b, c;
    float data[16];
};
struct FC4OverlayRows {
    int32_t begin, end;
    uint32_t count;
    const int32_t* starts;
    const int32_t* ends;
};
struct FC4OverlayFrame {
    uintptr_t context;
    // snapshot=1, decals=2, shadows=4, useSecondStage=8.
    uint32_t flags;
    void (*resetModel)(uintptr_t);
    void (*modelMatrix)(uintptr_t, float*);
    // depthOffset=0, deviceFlag7=1, locationStrengthEnabled=2.
    uint32_t (*value)(uintptr_t, uint32_t);
    // acquire=1 transfers one extension reference if the result is nonzero;
    // acquire=0 releases exactly that reference on the common exit.
    uint32_t (*vertexExtension)(uintptr_t, uint32_t acquire);
    void (*rows)(uintptr_t, FC4OverlayRows*);
    void (*command)(uintptr_t, const FC4OverlayCommand*);
    // Borrowed resources, valid for the call. Null means no resource in that arm.
    void (*texture)(uintptr_t, uint32_t stage, uintptr_t binding, uintptr_t source);
    // route: land static/dynamic/extension static/extension dynamic=0..3,
    // water static/dynamic=4..5, land shadow static/dynamic=6..7.
    void (*rectangle)(uintptr_t, uint32_t route, const int32_t* vertices);
    int32_t (*water)(uintptr_t, int32_t* cells);
    void (*shadowParameters)(uintptr_t, float* q_s_rgb);
    float (*strength)(uintptr_t, const float* worldXZ);
};
