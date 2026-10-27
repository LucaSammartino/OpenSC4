// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "FC4OverlayNative.h"
#include "compat/FC4Compat.h"
#include "cIGZGDriver.h"

namespace fc4::overlay_device {
// Native context methods from draw-context sections 3, 5 and 7, with stack
// cleanup/receivers from functions.tsv. Transparency and environment colour
// have no receiver. In particular, RGB and matrix arguments are pointers,
// not floats passed by value. No portable frame is a native context.
using Word = void (__thiscall*)(void*, uint32_t);
using Record = void (__thiscall*)(void*, const void*);
using Scalar = void (__stdcall*)(float);
using Plain = void (__stdcall*)();
using Color = void (__stdcall*)(const float*);
using Reset = void (__thiscall*)(void*);
using StageRecord = void (__thiscall*)(void*, const void*, uint32_t);
using StageWords = void (__thiscall*)(void*, uint32_t, uint32_t, uint32_t);
using Pair = void (__thiscall*)(void*, uint32_t, uint32_t);
using Alpha = void (__thiscall*)(void*, uint32_t, float);
struct Calls {
    Word depthOffset{};
    Record drawState{};
    Scalar transparency{};
    Record shade{}, selfLitShade{};
    Plain resetTransparency{};
    Reset resetShade{};
    StageRecord transform{};
    Word clearTransform{};
    StageWords filtering{}, wrap{};
    StageRecord combiner{};
    Pair textureEnabled{};
    Color environment{};
    Word alphaEnabled{};
    Alpha alphaFunction{};

    uint32_t Count() const {
        return (depthOffset != nullptr) + (drawState != nullptr) + (transparency != nullptr) +
            (shade != nullptr) + (selfLitShade != nullptr) + (resetTransparency != nullptr) +
            (resetShade != nullptr) + (transform != nullptr) + (clearTransform != nullptr) +
            (filtering != nullptr) + (wrap != nullptr) + (combiner != nullptr) +
            (textureEnabled != nullptr) + (environment != nullptr) + (alphaEnabled != nullptr) +
            (alphaFunction != nullptr);
    }
};

inline Calls Resolve() {
    using compat::Direct;
    return {reinterpret_cast<Word>(compat::DirectCall(Direct::OverlayDepthOffset)),
        reinterpret_cast<Record>(compat::DirectCall(Direct::OverlayDrawState)),
        reinterpret_cast<Scalar>(compat::DirectCall(Direct::OverlayTransparency)),
        reinterpret_cast<Record>(compat::DirectCall(Direct::OverlayShade)),
        reinterpret_cast<Record>(compat::DirectCall(Direct::OverlaySelfLitShade)),
        reinterpret_cast<Plain>(compat::DirectCall(Direct::OverlayResetTransparency)),
        reinterpret_cast<Reset>(compat::DirectCall(Direct::OverlayResetShade)),
        reinterpret_cast<StageRecord>(compat::DirectCall(Direct::OverlayTransform)),
        reinterpret_cast<Word>(compat::DirectCall(Direct::OverlayClearTransform)),
        reinterpret_cast<StageWords>(compat::DirectCall(Direct::OverlayFiltering)),
        reinterpret_cast<StageWords>(compat::DirectCall(Direct::OverlayWrap)),
        reinterpret_cast<StageRecord>(compat::DirectCall(Direct::OverlayCombiner)),
        reinterpret_cast<Pair>(compat::DirectCall(Direct::OverlayTextureEnabled)),
        reinterpret_cast<Color>(compat::DirectCall(Direct::OverlayEnvironment)),
        reinterpret_cast<Word>(compat::DirectCall(Direct::OverlayAlphaEnabled)),
        reinterpret_cast<Alpha>(compat::DirectCall(Direct::OverlayAlphaFunction))};
}

// All pointers are borrowed through Submit. State records must be native
// packed states, never a Rust DrawState or an ordinal cast to a pointer.
struct Inputs {
    void* context{};
    cIGZGDriver* driver{};
    const void* states[5]{};
    // SPEC-GAP: the three manager combiner byte records are not specified;
    // require explicit records, and refuse submission when one is missing.
    const void* combiners[3]{};
};

template<class Fn, class... Args>
inline bool Invoke(Fn fn, Args... args) {
    if (!fn) return false;
    fn(args...);
    return true;
}

// This is the device-command boundary only. It does not acquire frame services,
// install routing, or claim successful native Draw. Missing targets/records
// fail before the affected command makes any device call.
inline bool Submit(const Calls& calls, const Inputs& in, const FC4OverlayCommand& c) {
    if (!in.context) return false;
    switch (c.kind) {
    case FC4OverlayCommandKind::DepthOffset:
        return Invoke(calls.depthOffset, in.context, c.a);
    case FC4OverlayCommandKind::DrawState:
        return c.a < 5 && in.states[c.a] && Invoke(calls.drawState, in.context, in.states[c.a]);
    case FC4OverlayCommandKind::Transparency:
        return Invoke(calls.transparency, c.data[0]);
    case FC4OverlayCommandKind::Shade:
        return Invoke(c.a ? calls.selfLitShade : calls.shade, in.context, c.data);
    case FC4OverlayCommandKind::VertexColors:
        if (!in.driver) return false;
        in.driver->EnableVertexColors(c.a != 0, c.b != 0);
        return true;
    case FC4OverlayCommandKind::ResetTransparency:
        return Invoke(calls.resetTransparency);
    case FC4OverlayCommandKind::ResetShade:
        return Invoke(calls.resetShade, in.context);
    case FC4OverlayCommandKind::Transform:
        return Invoke(calls.transform, in.context, c.data, c.a);
    case FC4OverlayCommandKind::ClearTransform:
        return Invoke(calls.clearTransform, in.context, c.a);
    case FC4OverlayCommandKind::Filtering:
        return Invoke(calls.filtering, in.context, c.b, c.c, c.a);
    case FC4OverlayCommandKind::Wrap:
        return Invoke(calls.wrap, in.context, c.b, c.c, c.a);
    case FC4OverlayCommandKind::ShadowCombiner:
        return c.b < 3 && in.combiners[c.b] &&
            Invoke(calls.combiner, in.context, in.combiners[c.b], c.a);
    case FC4OverlayCommandKind::TextureEnabled:
        return Invoke(calls.textureEnabled, in.context, c.b, c.a);
    case FC4OverlayCommandKind::Environment:
        return Invoke(calls.environment, c.data);
    case FC4OverlayCommandKind::AlphaTest:
        if (!calls.alphaEnabled || (c.a && !calls.alphaFunction)) return false;
        calls.alphaEnabled(in.context, c.a);
        if (c.a) calls.alphaFunction(in.context, 6, c.data[0]); // greater-or-equal, section 4.6
        return true;
    case FC4OverlayCommandKind::DeviceFlag7:
        if (!in.driver) return false;
        if (c.a) in.driver->Enable(7);
        else in.driver->Disable(7);
        return true;
    }
    return false;
}
}
