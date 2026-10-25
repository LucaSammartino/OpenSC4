// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "cIGZUnknown.h"

// FC4-owned identities, NOT native cSTEOverlayManager identities. The native
// interface IID is 0x895D6544 and must never be returned for this interface.
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
};

cIGZUnknown* CreateFC4OverlayOwner();
