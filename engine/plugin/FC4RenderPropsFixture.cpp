// SPDX-License-Identifier: GPL-3.0-or-later
// Native isolated fixture: render-properties §§1.1, 1.3, 2, 7.1 and 7.4.
#include "FC4RenderPropsFixture.h"
#include "FC4RefAudit.h"
#include "cISC4RenderProperties.h"
#include <windows.h>
#include <cstring>

extern "C" void* fc4_renderprops_fixture_new();
extern "C" void fc4_renderprops_fixture_drop(void*);
extern "C" uint32_t fc4_renderprops_fixture_value(const void*, uint32_t, int32_t);
extern "C" bool fc4_renderprops_fixture_set(void*, uint32_t, int32_t, uint32_t);
extern "C" uint32_t fc4_renderprops_fixture_ref(void*, uint32_t, uint32_t, uint32_t);

namespace fc4 {
namespace {
void Equal(uint32_t actual, uint32_t expected, uint32_t& fields, uint32_t& mismatches) {
    ++fields;
    mismatches += actual != expected;
}

uint32_t Bits(cISC4RenderProperties* p, uint32_t kind, int32_t index) {
    if (kind == 0) return p->BoolValue(index) ? 1 : 0;
    if (kind == 1) return static_cast<uint32_t>(p->IntValue(index));
    const float value = p->FloatValue(index);
    uint32_t bits;
    std::memcpy(&bits, &value, sizeof(bits));
    return bits;
}

void Values(cISC4RenderProperties* p, const void* expected, uint32_t& fields, uint32_t& mismatches) {
    constexpr int32_t counts[] = {38, 24, 15};
    for (uint32_t kind = 0; kind < 3; ++kind) {
        for (int32_t index = 0; index < counts[kind]; ++index) {
            Equal(Bits(p, kind, index), fc4_renderprops_fixture_value(expected, kind, index), fields, mismatches);
        }
    }
}

uint32_t Count(cISC4RenderProperties* p) {
    uint32_t count;
    std::memcpy(&count, reinterpret_cast<const char*>(p) + 8, sizeof(count));
    return count;
}

// §§1.1/1.3 specify a receiver-only constructor and 0x30-byte complete
// storage. Literal ABI interpretation (RP8): __thiscall, no stack inputs.
// Non-freeing U destructor flags 0/2 reclaim the game-created tables while
// the outer storage remains ours. Never let Release free stack/embedded storage.
struct NativeOwner {
    alignas(4) unsigned char storage[0x30]{};
    cISC4RenderProperties* p() { return reinterpret_cast<cISC4RenderProperties*>(storage); }
    cIGZUnknown* u() { return reinterpret_cast<cIGZUnknown*>(storage + 4); }
    void* expected = fc4_renderprops_fixture_new();
    explicit NativeOwner(uintptr_t base) {
        using Construct = void(__thiscall*)(void*);
        reinterpret_cast<Construct>(base + 0x003E3650)(storage);
    }
    NativeOwner(const NativeOwner&) = delete;
    NativeOwner& operator=(const NativeOwner&) = delete;
    void* Destroy(uint32_t flags) {
        using DestroyFn = void*(__thiscall*)(void*, uint32_t);
        auto** vtable = *reinterpret_cast<void***>(u());
        return reinterpret_cast<DestroyFn>(vtable[3])(u(), flags);
    }
    ~NativeOwner() { fc4_renderprops_fixture_drop(expected); }
};
}

bool RenderPropsFixture(cISC4RenderProperties* application, uint32_t& fields, uint32_t& mismatches) {
    // Spec addresses refer to the supported 1.1.641 image. Validate the original
    // object and its documented getter slots before calling any private entry.
    const auto base = reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr));
    if (!application || !base) return false;
    const auto* vtable = *reinterpret_cast<const uintptr_t* const*>(application);
    if (reinterpret_cast<uintptr_t>(vtable) != base + 0x006BB6C0 ||
        vtable[6] != base + 0x003DDEC0 || vtable[7] != base + 0x003DDEE0 ||
        vtable[8] != base + 0x003DDF00) return false;

    NativeOwner a(base), b(base);
    Equal(Count(a.p()), fc4_renderprops_fixture_ref(a.expected, 0, 0, 0), fields, mismatches);
    Equal(Count(b.p()), fc4_renderprops_fixture_ref(b.expected, 0, 0, 0), fields, mismatches);
    Values(a.p(), a.expected, fields, mismatches);
    Values(b.p(), b.expected, fields, mismatches);
    constexpr int32_t counts[] = {38, 24, 15};
    // Three ordered passes cover every valid slot/index, duplicate/no-change
    // writes, signed integer extremes, unclamped values, and finite binary32
    // including negative zero. No exceptional x87/NaN claim is made.
    for (uint32_t pass = 0; pass < 3; ++pass) {
        for (uint32_t kind = 0; kind < 3; ++kind) {
            for (int32_t index = 0; index < counts[kind]; ++index) {
                uint32_t bits = pass == 1 ? 1 : 0;
                if (kind == 1) bits = pass == 0 ? static_cast<uint32_t>(-17) :
                    pass == 1 ? 0x80000000u + index : 0x7fffffffu - index;
                if (kind == 2) bits = pass == 0 ? 0x80000000u : pass == 1 ? 0x3fc00000u : 0xc18a0000u;
                for (uint32_t duplicate = 0; duplicate < 2; ++duplicate) {
                    // Inputs are generated independently of every native result.
                    // Apply to Rust only after the native void setter completes.
                    if (kind == 0) a.p()->SetBoolValue(index, bits != 0);
                    else if (kind == 1) a.p()->SetIntValue(index, static_cast<int32_t>(bits));
                    else {
                        float value;
                        std::memcpy(&value, &bits, sizeof(value));
                        a.p()->SetFloatValue(index, value);
                    }
                    Equal(fc4_renderprops_fixture_set(a.expected, kind, index, bits), 1, fields, mismatches);
                    // All 77 outputs after EVERY write: also catch wrong-index/type
                    // corruption that a later write might otherwise hide.
                    Values(a.p(), a.expected, fields, mismatches);
                }
            }
        }
        Values(b.p(), b.expected, fields, mismatches);
    }

    // Both views accept only IID 1 and return U, sharing one count. All reference
    // operations here concern owners we constructed; borrowed live P/S are untouched.
    for (uint32_t view = 0; view < 2; ++view) {
        cIGZUnknown* receiver = view == 0 ? static_cast<cIGZUnknown*>(a.p()) : a.u();
        void* output = a.p(); // failure must preserve this non-null sentinel
        const uint32_t rejected = fc4_renderprops_fixture_ref(a.expected, view, 3, 0);
        Equal(receiver->QueryInterface(0, &output), rejected != UINT32_MAX, fields, mismatches);
        Equal(output == a.p(), 1, fields, mismatches);
        Equal(receiver->QueryInterface(0xffffffffu, nullptr), 0, fields, mismatches);
        output = nullptr;
        const uint32_t offset = fc4_renderprops_fixture_ref(a.expected, view, 3, 1);
        Equal(receiver->QueryInterface(1, &output), offset != UINT32_MAX, fields, mismatches);
        Equal(output == a.storage + offset, 1, fields, mismatches);
        FC4_ACQUIRED(static_cast<cIGZUnknown*>(output), "owned render fixture QueryInterface");
        Equal(Count(a.p()), fc4_renderprops_fixture_ref(a.expected, view, 0, 0), fields, mismatches);
    }
    Equal(FC4_ADDREF(a.p()), fc4_renderprops_fixture_ref(a.expected, 0, 1, 0), fields, mismatches);
    Equal(FC4_ADDREF(a.u()), fc4_renderprops_fixture_ref(a.expected, 1, 1, 0), fields, mismatches);
    Equal(FC4_RELEASE(a.p()), fc4_renderprops_fixture_ref(a.expected, 0, 2, 0), fields, mismatches);
    Equal(FC4_RELEASE(a.u()), fc4_renderprops_fixture_ref(a.expected, 1, 2, 0), fields, mismatches);
    Equal(FC4_RELEASE(a.u()), fc4_renderprops_fixture_ref(a.expected, 1, 2, 0), fields, mismatches);
    Equal(Count(a.p()), fc4_renderprops_fixture_ref(a.expected, 1, 0, 0), fields, mismatches);
    // One acquired U reference remains and is consumed by our explicit lifetime
    // termination. Record it in the audit, without issuing a freeing Release.
    const auto owned = RefBefore(a.u());
    const uint32_t offset = fc4_renderprops_fixture_ref(a.expected, 1, 4, 2);
    Equal(a.Destroy(2) == a.storage + offset, 1, fields, mismatches);
    RefNote(owned, -1, 0, "owned fixture nonfree destruction", FC4_SITE, true, true);
    Values(b.p(), b.expected, fields, mismatches);
    const uint32_t otherOffset = fc4_renderprops_fixture_ref(b.expected, 1, 4, 0);
    Equal(b.Destroy(0) == b.storage + otherOffset, 1, fields, mismatches);
    // No access to either terminated native object's contents beyond here.
    return true;
}
}
