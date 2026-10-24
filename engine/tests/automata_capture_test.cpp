// SPDX-License-Identifier: GPL-3.0-or-later
// Exercise actual DLL capture/replay with an adjusted effect interface and a
// temporary released by its writer, then malformed but in-range/unknown IDs.
#include "FC4TestSeam.h"
#include "FC4Streams.h"
#include "cIGZSerializable.h"
#include <windows.h>
#include <cstdio>

namespace {
constexpr uint32_t kClass = 0x2977AA49;
struct Object : cIGZSerializable {
    uint32_t refs = 1, zeroes = 0, queries = 0;
    bool QueryInterface(uint32_t iid, void** out) override {
        *out = nullptr;
        if (refs == 0) return false;  // detect a stale capture without invoking UB
        if (iid != GZIID_cIGZUnknown && iid != GZIID_cIGZSerializable && iid != kClass) return false;
        ++queries;
        *out = static_cast<cIGZSerializable*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { if (--refs == 0) ++zeroes; return refs; }
    bool Read(cIGZIStream&) override { return false; }
    bool Write(cIGZOStream&) override { return false; }
    uint32_t GetGZCLSID() override { return kClass; }
};
// Models the non-serializable effect interface accepted by native Write. Only
// its IUnknown prefix can be used until QI supplies the serializable interface.
struct EffectInterface : Object {
    Object* actual;
    uint32_t wrongClassCalls = 0;
    explicit EffectInterface(Object* o) : actual(o) {}
    bool QueryInterface(uint32_t iid, void** out) override { return actual->QueryInterface(iid, out); }
    uint32_t AddRef() override { return actual->AddRef(); }
    uint32_t Release() override { return actual->Release(); }
    uint32_t GetGZCLSID() override { ++wrongClassCalls; return 0xBAD; }
};
struct Writer : Object {
    Object* child;
    cIGZSerializable* supplied;
    uint32_t badId = 0, badClass = kClass;
    bool drop = false;
    Writer(Object* c, cIGZSerializable* s) : child(c), supplied(s) {}
    bool Write(cIGZOStream& out) override {
        auto& segment = static_cast<cFC4ISC4DBSegmentOStream&>(out);
        if (!segment.WriteGZSerializable(supplied)) return false;
        if (badId) {
            out.SetUint32(badId);
            out.SetUint32(badClass);
        } else {
            // Duplicate incoming interface keeps the original ID and takes
            // only one capture-owned reference.
            if (!segment.WriteGZSerializable(supplied)) return false;
            segment.WriteGZSerializable(nullptr);
        }
        if (drop) child->Release();
        return true;
    }
};
}

int main(int argc, char** argv) {
    if (argc != 2) return 1;
    HMODULE dll = LoadLibraryA(argv[1]);
    if (!dll) return 2;
    auto seam = reinterpret_cast<FC4TestSeamFn>(GetProcAddress(dll, "FC4TestSeam"));
    if (!seam) return 3;
    auto replay = [&](Writer& writer) {
        return seam(kFC4SeamAutomataCapture, &writer,
                    reinterpret_cast<void*>(static_cast<uintptr_t>(kClass)));
    };
    Object child;
    EffectInterface effect(&child);
    Writer temporary(&child, &effect);
    temporary.drop = true;
    if (replay(temporary) != 1 || child.refs != 0 || child.zeroes != 1 ||
        child.queries < 3 || effect.wrongClassCalls != 0 || temporary.refs != 1) return 4;
    Object other;
    Writer invalid(&other, &other);
    invalid.badId = 2;  // no such capture ID
    if (replay(invalid) != 0 || other.refs != 1) return 5;
    invalid.badId = 1;  // an existing ID belonging to another class
    invalid.badClass ^= 1;
    if (replay(invalid) != 0 || other.refs != 1) return 6;
    invalid.badId = 0;
    if (replay(invalid) != 1 || other.refs != 1) return 7;
    FreeLibrary(dll);
    std::puts("automata capture: adjusted interface, released temporary, duplicate/null, unknown ID and class collision PASS");
    return 0;
}
