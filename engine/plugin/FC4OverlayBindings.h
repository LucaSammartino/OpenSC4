// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "FC4OverlayOwner.h"
#include "FC4RefAudit.h"
#include "compat/FC4Compat.h"
#include <atomic>
#include <cstddef>

namespace fc4::overlay_bindings {
// texture-bindings §2 and overlay-manager §2.7. A binding has no vtable.
struct Binding {
    uint32_t* handles;
    uint32_t frames;
    cIGZUnknown* factory;
    uint32_t references;
};
static_assert(sizeof(Binding) == 16 && offsetof(Binding, factory) == 8, "x86 binding layout");

// overlay-manager §§2.7/2.7.1: bindings take precedence and use frame zero;
// sources are called on every submission, since obtaining a handle can render
// or reorder their cache. Neither kind of driver handle is a COM reference.
using SetTexture = void (__thiscall*)(void*, uint32_t handle, uint32_t stage);
inline void SubmitTexture(void* drawContext, SetTexture setTexture, uint32_t stage,
                          uintptr_t binding, uintptr_t source) {
    if (binding) {
        setTexture(drawContext, reinterpret_cast<const Binding*>(binding)->handles[0], stage);
    } else if (source) {
        setTexture(drawContext, reinterpret_cast<FC4IOverlaySource*>(source)->Texture(), stage);
    }
}

struct Info {
    uint32_t group, instance, levels, width, height;
    uint8_t rgba[4], bits;
};
static_assert(offsetof(Info, width) == 0x0c && offsetof(Info, bits) == 0x18, "binding-info prefix");

inline uint32_t NativeAddRef(void* binding) {
    using Fn = void (__thiscall*)(void*);
    auto fn = reinterpret_cast<Fn>(compat::DirectCall(compat::Direct::TextureBindingAddRef));
    if (fn) fn(binding);
    return 0; // the transport callback's unused result is not a native return
}
inline uint32_t NativeRelease(void* binding) {
    using Fn = uint32_t (__thiscall*)(void*);
    auto fn = reinterpret_cast<Fn>(compat::DirectCall(compat::Direct::TextureBindingRelease));
    return fn ? fn(binding) : 0;
}

class Context {
public:
    Context(cIGZUnknown* factory, FC4OverlayBindingCalls calls, int32_t quality, int32_t debug)
        : factory_(factory), calls_(calls), quality_(quality), debug_(debug) {
        FC4_ADDREF(factory_);
    }
    void Release() { if (!--refs_) delete this; }
    FC4OverlayHost Host(uint32_t nx, uint32_t nz, float width) {
        return {reinterpret_cast<uintptr_t>(this), Retain, Drop, nx, nz, width, Setting,
            Lookup, Import, ReleaseResource, KeyWord, nullptr, nullptr, nullptr, nullptr, nullptr};
    }
private:
    ~Context() { FC4_RELEASE(factory_); }
    static Context& Get(uintptr_t p) { return *reinterpret_cast<Context*>(p); }
    static void Retain(uintptr_t p) { ++Get(p).refs_; }
    static void Drop(uintptr_t p) { Get(p).Release(); }
    static int32_t Setting(uintptr_t p, uint32_t index) {
        auto& c = Get(p);
        return index == 0 ? c.quality_ : index == 1 ? c.debug_ : 1;
    }
    static void ReleaseResource(uintptr_t release, uintptr_t object, uint32_t kind) {
        if (!object) return;
        if (kind == 0) reinterpret_cast<uint32_t (*)(void*)>(release)(reinterpret_cast<void*>(object));
        else if (kind == 1) FC4_RELEASE(reinterpret_cast<FC4IOverlaySource*>(object));
    }
    void Describe(void* binding, FC4OverlayResource* out) {
        // Slot8 has no success result; unknown bindings leave output untouched.
        Info info{};
        auto** table = *reinterpret_cast<void***>(factory_);
        using GetInfo = void (__thiscall*)(void*, void*, Info*);
        reinterpret_cast<GetInfo>(table[8])(factory_, binding, &info);
        *out = {reinterpret_cast<uintptr_t>(binding), reinterpret_cast<uintptr_t>(calls_.release),
                info.width, info.height};
    }
    static uint32_t Import(uintptr_t p, uintptr_t object, uint32_t kind, FC4OverlayResource* out) {
        if (!object || !out) return 0;
        if (kind == 1) {
            auto* source = reinterpret_cast<FC4IOverlaySource*>(object);
            uint32_t size[2];
            source->Dimensions(size);
            FC4_ADDREF(source);
            *out = {object, 0, size[0], size[1]};
            return 1;
        }
        if (kind != 0) return 0;
        auto& c = Get(p);
        auto* binding = reinterpret_cast<void*>(object);
        c.Describe(binding, out);
        c.calls_.addRef(binding);
        return 1;
    }
    static uint32_t Lookup(uintptr_t p, uint32_t slot, uintptr_t key, FC4OverlayResource* out) {
        if (!out || (slot != 5 && slot != 6)) return 0;
        auto& c = Get(p);
        auto** table = *reinterpret_cast<void***>(c.factory_);
        void* binding = nullptr;
        if (slot == 5) {
            using GetID = bool (__thiscall*)(void*, uint32_t, void**);
            reinterpret_cast<GetID>(table[5])(c.factory_, static_cast<uint32_t>(key), &binding);
        } else {
            using GetKey = bool (__thiscall*)(void*, const void*, void**);
            reinterpret_cast<GetKey>(table[6])(c.factory_, reinterpret_cast<const void*>(key), &binding);
        }
        // The lookup output already owns its reference. Do not AddRef twice or
        // Release a pointer when the game did not return one.
        if (!binding) return 0;
        c.Describe(binding, out);
        return 1;
    }
    static uint32_t KeyWord(uintptr_t p) {
        auto* factory = Get(p).factory_;
        auto** table = *reinterpret_cast<void***>(factory);
        using GetWord = uint32_t (__thiscall*)(void*);
        return reinterpret_cast<GetWord>(table[11])(factory);
    }
    std::atomic<uint32_t> refs_{1};
    cIGZUnknown* factory_;
    FC4OverlayBindingCalls calls_;
    int32_t quality_, debug_;
};

inline bool Init(FC4IOverlayOwner* owner, cIGZUnknown* factory, uint32_t nx, uint32_t nz, float width,
                 bool dynamic, bool water, int32_t quality, int32_t debug, const FC4OverlayBindingCalls* test) {
    if (!owner || !factory) return false;
    FC4OverlayBindingCalls calls{NativeAddRef, NativeRelease};
    if (test) calls = *test;
    else if (!compat::DirectCall(compat::Direct::TextureBindingAddRef) ||
             !compat::DirectCall(compat::Direct::TextureBindingRelease)) return false;
    if (!calls.addRef || !calls.release) return false;
    auto* context = new Context(factory, calls, quality, debug);
    const auto host = context->Host(nx, nz, width);
    const bool result = owner->InitServices(&host, dynamic, water);
    context->Release();
    return result;
}
}
