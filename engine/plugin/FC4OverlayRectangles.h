// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "compat/FC4Compat.h"
#include <cstdint>
#include <type_traits>

namespace fc4::overlay_rectangles {
// overlay-manager section 4.6.1: these are already expanded VERTEX bounds.
// Expansion belongs to the owner; repeating it at this boundary adds a cell.
struct Rectangle { int32_t x0, z0, x1, z1; };
static_assert(sizeof(Rectangle) == 16 && alignof(Rectangle) == 4 &&
              std::is_trivially_copyable_v<Rectangle>, "four x86 argument words");

// ECX is the complete terrain-view base retained by Init, NOT its +0x0c
// cISTETerrainView interface. Each callee removes its own argument bytes.
using Land = void (__thiscall*)(void*, Rectangle, void*);             // pop 20
using Extension = void (__thiscall*)(void*, void*, void*, Rectangle); // pop 24
using Water = uint8_t (__thiscall*)(void*, Rectangle, void*);          // pop 20, AL
using Shadow = void (__thiscall*)(void*, const Rectangle*, void*);    // pop 8
struct Calls {
    Land land[2]{};
    Extension extension[2]{};
    Water water[2]{};
    Shadow shadow[2]{};
};

// Calls are resolved only through the executable-version gate. Merely resolving
// them neither installs terrain-view routing nor makes a portable frame native.
inline Calls Resolve() {
    using compat::Direct;
    return {{reinterpret_cast<Land>(compat::DirectCall(Direct::OverlayLandRect)),
             reinterpret_cast<Land>(compat::DirectCall(Direct::OverlayLandRectDynamic))},
            {reinterpret_cast<Extension>(compat::DirectCall(Direct::OverlayLandRectExtension)),
             reinterpret_cast<Extension>(compat::DirectCall(Direct::OverlayLandRectDynamicExtension))},
            {reinterpret_cast<Water>(compat::DirectCall(Direct::OverlayWaterRect)),
             reinterpret_cast<Water>(compat::DirectCall(Direct::OverlayWaterRectDynamic))},
            {reinterpret_cast<Shadow>(compat::DirectCall(Direct::OverlayShadowRect)),
             reinterpret_cast<Shadow>(compat::DirectCall(Direct::OverlayShadowRectDynamic))}};
}

// route is FC4OverlayFrame::rectangle's 0..7 route, selected by the Rust owner.
// All pointers are borrowed through this synchronous call. The extension has
// already been acquired by the frame producer and is released on Draw's exit.
inline bool Submit(const Calls& calls, void* viewBase, void* context, void* extension,
                   uint32_t route, const int32_t* vertices) {
    if (!viewBase || !context || !vertices || route >= 8) return false;
    const Rectangle rectangle{vertices[0], vertices[1], vertices[2], vertices[3]};
    const uint32_t dynamic = route & 1;
    switch (route / 2) {
    case 0:
        if (!calls.land[dynamic]) return false;
        calls.land[dynamic](viewBase, rectangle, context);
        break;
    case 1:
        if (!calls.extension[dynamic] || !extension) return false;
        calls.extension[dynamic](viewBase, context, extension, rectangle);
        break;
    case 2:
        if (!calls.water[dynamic]) return false;
        // Native DrawRect deliberately ignores the consumer's byte result.
        calls.water[dynamic](viewBase, rectangle, context);
        break;
    case 3:
        if (!calls.shadow[dynamic]) return false;
        // This caller-owned 16-byte local is separate from the eight argument
        // bytes. It remains alive until the synchronous consumer returns.
        calls.shadow[dynamic](viewBase, &rectangle, context);
        break;
    }
    return true;
}
}
