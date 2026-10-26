// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "cIGZUnknown.h"
#include "FC4OverlayNative.h"

// FC4-owned factory/portable identities. The native interface is at object+0,
// its draw interface at +4, canonical unknown at +8 and message target at +0x10.
// This establishes subobject offsets, not the native arenas/member layout.
constexpr uint32_t kFC4_CLSID_OverlayOwner = 0xFC401717;
constexpr uint32_t kFC4_IID_OverlayOwner = 0xFC401718;

// Explicit portable input boundary. No native Init ABI or resource layout is
// asserted by these methods. Dimensions are caller-supplied service results.
class FC4IOverlayOwner : public cIGZUnknown {
public:
    virtual bool Init(uint32_t nx, uint32_t nz, float cellWidth, bool dynamic, bool water) = 0;
    virtual void Shutdown() = 0;
    virtual uint32_t AddDecal(uint32_t textureWidth, uint32_t textureHeight,
                              float x, float z, float width, float turns) = 0;
    virtual bool Remove(uint32_t id) = 0;
    virtual bool SetAlpha(uint32_t id, float alpha) = 0;
    virtual float Alpha(uint32_t id) const = 0;
    virtual bool IsInitialized() const = 0;
    virtual bool SetShown(uint32_t id, bool shown) = 0;
    virtual bool IsShown(uint32_t id) const = 0;
    virtual bool EnableSelfLighting(uint32_t id, uint32_t on) = 0;
    virtual bool IsSelfLit(uint32_t id) const = 0;
    virtual bool SetColor(uint32_t id, float r, float g, float b) = 0;
    // Copies all three words on success; failure leaves the output untouched.
    virtual bool Color(uint32_t id, float* rgb) const = 0;
    virtual bool SetDrawState(uint32_t id, uint32_t index) = 0;
    virtual uint32_t DrawStateIndex(uint32_t id) const = 0;
    virtual bool SetPosition(uint32_t id, float x, float z) = 0;
    // Nine words: cx, cz, width, turns, stretch, repeatU, repeatV, offset, retained.
    virtual bool SetPlacement(uint32_t id, const float* placement) = 0;
    // Failure, direct IDs and the sentinel leave all output words untouched.
    virtual bool Placement(uint32_t id, float* placement) const = 0;
    // Native resource/service inputs, captured before any allocation. This is
    // still an explicit boundary; it does not intercept terrain-view Init.
    virtual bool InitServices(const FC4OverlayHost*, bool dynamic, bool water) = 0;
    virtual bool InitBindingFactory(cIGZUnknown* factory, uint32_t nx, uint32_t nz, float cellWidth,
                                    bool dynamic, bool water, int32_t quality, int32_t debug,
                                    const FC4OverlayBindingCalls* testCalls = nullptr) = 0;
    virtual bool DrawFrame(const FC4OverlayFrame*) = 0;
};

cIGZUnknown* CreateFC4OverlayOwner();

namespace fc4 {
// Invoked once after the shadow sees the live terrain view; default-off with
// the owner key. Exercises GZCOM registration, not native manager interception.
void OverlayOwnerGameCheck();
}
