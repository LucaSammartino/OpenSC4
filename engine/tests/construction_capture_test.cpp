// SPDX-License-Identifier: GPL-3.0-or-later
// LR13 T66-T73: normative native doubles, not measurements of game code.
#include "FC4TestSeam.h"
#include "FC4Streams.h"
#include "cIGZSerializable.h"
#include <windows.h>
#include <cstdio>
#include <vector>

namespace {
constexpr uint32_t kEffect = 0x2977AA49, kProp = 0xE9793A65, kCrew = 0xA9B40F05;
struct ClassView : cIGZUnknown {};
struct PropView : cIGZUnknown {};
struct SpareView : cIGZUnknown {};
struct OccupantView : cIGZUnknown {};
// Five distinct vptrs model O, O+4, O+8, O+C and O+10 on x86.
struct Effect : ClassView, PropView, SpareView, OccupantView, cIGZSerializable {
    uint32_t refs = 1, adds = 0, releases = 0, rejected = 0;
    uint32_t cls = kEffect, fieldIid = kEffect;
    bool rejectField = false, rejectSerializable = false;
    bool QueryInterface(uint32_t iid, void** out) override {
        *out = nullptr;
        if (iid == GZIID_cIGZSerializable && !rejectSerializable)
            *out = static_cast<cIGZSerializable*>(this);
        else if (iid == fieldIid && !rejectField)
            *out = static_cast<ClassView*>(this);
        else if (iid == GZIID_cIGZUnknown)
            *out = static_cast<ClassView*>(this);
        if (!*out) { ++rejected; return false; }
        AddRef();
        return true;
    }
    uint32_t AddRef() override { ++adds; return ++refs; }
    uint32_t Release() override { ++releases; return --refs; }
    bool Read(cIGZIStream&) override { return false; }
    bool Write(cIGZOStream&) override { return false; }
    uint32_t GetGZCLSID() override { return cls; }
    cIGZUnknown* Prop() { return static_cast<PropView*>(this); }
    cIGZSerializable* Serial() { return static_cast<cIGZSerializable*>(this); }
    cIGZUnknown* Class() { return static_cast<ClassView*>(this); }
};

// Native helper contract: writer QI and Serializable QI are temporary, even
// when the callback fails. The observer must pin inside the callback.
bool WriteReference(cIGZOStream& stream, cIGZUnknown* input,
                    std::vector<cIGZSerializable*>& callbacks, bool fail = false) {
    void* view = nullptr;
    if (!stream.QueryInterface(kFC4_IID_cISC4DBSegmentOStream, &view)) return false;
    auto* writer = static_cast<cFC4ISC4DBSegmentOStream*>(view);
    void* serial = nullptr;
    if (input) input->QueryInterface(GZIID_cIGZSerializable, &serial);
    auto* object = static_cast<cIGZSerializable*>(serial);
    callbacks.push_back(object);
    const bool ok = writer->WriteGZSerializable(object) && !fail;
    if (object) object->Release();
    writer->Release();
    return ok;
}

struct Construction : cIGZSerializable {
    uint32_t refs = 1;
    std::vector<cIGZUnknown*> ground, roof, sparks, crew;
    cIGZUnknown* current = nullptr;
    bool failCallback = false;
    std::vector<cIGZSerializable*> callbacks;
    bool QueryInterface(uint32_t iid, void** out) override {
        *out = nullptr;
        if (iid != GZIID_cIGZUnknown && iid != GZIID_cIGZSerializable) return false;
        *out = static_cast<cIGZSerializable*>(this); AddRef(); return true;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return --refs; }
    bool Read(cIGZIStream&) override { return false; }
    uint32_t GetGZCLSID() override { return 0xA97F909E; }
    bool Write(cIGZOStream& out) override {
        callbacks.clear();
        out.SetUint16(5);
        // Complete inherited base, including the holder (LR §2.3).
        out.SetUint16(4); out.SetUint16(0x1234);
        out.SetUint8(0); out.SetUint8(5); out.SetUint32(0x096E763C);
        for (uint8_t v : {64, 65, 66, 67}) out.SetUint8(v);
        out.SetUint16(2); out.SetUint16(2); out.SetUint32(0);
        for (int i = 0; i < 3; ++i) out.SetUint32(0);
        WriteReference(out, nullptr, callbacks); // lot
        WriteReference(out, nullptr, callbacks); // building
        out.SetUint8(7); // matrix, translation and scale, not just mask zero
        for (int i = 0; i < 13; ++i) out.SetFloat32(0.25f * (i + 1));
        for (int i = 0; i < 12; ++i) out.SetFloat32(1.5f * (i + 1)); // both boxes
        out.SetUint8(1); // live crane, explicitly seeded for shadow only
        for (int i = 0; i < 3; ++i) out.SetFloat32(10.5f + i);
        for (const auto* list : {&ground, &roof, &sparks}) {
            out.SetUint32(static_cast<uint32_t>(list->size()));
            for (auto* ref : *list) WriteReference(out, ref, callbacks);
        }
        const bool ok = WriteReference(out, current, callbacks, failCallback);
        // All 22 scalar members, in specified order, with distinct values.
        out.SetSint8(-1);
        for (int i = 0; i < 5; ++i) out.SetUint32(100 + i);
        out.SetFloat32(1.25f); out.SetFloat32(-2.5f);
        for (int i = 0; i < 5; ++i) out.SetUint32(200 + i);
        out.SetSint8(3); out.SetFloat32(0.5f); out.SetFloat32(2.75f);
        out.SetUint8(1); out.SetUint32(300); out.SetUint32(301);
        out.SetFloat32(-0.75f); out.SetFloat32(3.25f); out.SetUint8(2);
        out.SetUint32(static_cast<uint32_t>(crew.size()));
        for (auto* ref : crew) {
            out.SetUint8(1); out.SetUint8(0); WriteReference(out, ref, callbacks);
        }
        return ok;
    }
};
}

int main(int argc, char** argv) {
    if (argc != 2) return 1;
    HMODULE dll = LoadLibraryA(argv[1]);
    if (!dll) return 2;
    auto seam = reinterpret_cast<FC4TestSeamFn>(GetProcAddress(dll, "FC4TestSeam"));
    if (!seam) return 3;
    int failures = 0;
    auto check = [&](const char* name, Construction& record, intptr_t expected) {
        const auto got = seam(kFC4SeamConstructionCapture, &record, nullptr);
        if (got != expected || record.refs != 1) {
            std::printf("%s got=%zd expected=%zd refs=%u\n", name, got, expected, record.refs);
            ++failures;
        }
    };
    Effect effect;
    auto offset = [&](const void* p) { return static_cast<const char*>(p) - reinterpret_cast<const char*>(&effect); };
    if (offset(effect.Prop()) != 4 || offset(effect.Serial()) != 0x10) return 4;
    Construction c;
    c.sparks = {effect.Prop()}; c.current = effect.Serial();
    // RefAudit performs one balanced Serializable QI per previously unseen
    // vtable. Warm its class cache before counting helper acquisitions.
    check("T66 audit-cache warmup", c, 1);
    effect.adds = effect.releases = 0;
    check("T66 derived identity through both views", c, 1);
    if (c.callbacks[2] != effect.Serial() || c.callbacks[3] != effect.Serial()) ++failures;
    // Per occurrence: writer helper QI, observer QI, reader temporary and
    // successful field QI. Duplicate identity still has independent lifetimes.
    if (effect.adds != 8 || effect.releases != 8) {
        std::printf("T66 helper acquisitions %u/%u expected 8/8\n", effect.adds, effect.releases);
        ++failures;
    }
    Effect ground, roof, crew;
    ground.fieldIid = roof.fieldIid = kProp;
    ground.cls = roof.cls = 0x2977AA47;
    crew.fieldIid = kCrew; crew.cls = 0x896E75AF;
    c.ground = {ground.Class()}; c.roof = {roof.Class()}; c.crew = {crew.Class()};
    check("T67 prop and crew literal IIDs, order and bools", c, 1);
    c.ground.clear(); c.roof.clear(); c.crew.clear();
    c.sparks = {nullptr}; c.current = nullptr;
    check("T68 present null", c, 1);
    c.sparks.clear(); check("T68 empty vector", c, 1);
    Effect bare; bare.cls = 0xA9BC9AB6; bare.rejectField = true;
    c.current = bare.Serial();
    check("T69 rejected bare occupant stays mismatched", c, 0);
    if (bare.adds != 3 || bare.releases != 3) ++failures; // no field acquisition
    c.current = nullptr; c.sparks = {bare.Prop()}; c.crew = {bare.Class()};
    check("T70 rejected list entries remain present", c, 0);
    if (bare.adds != 9 || bare.releases != 9) ++failures;
    c.sparks.clear(); c.crew.clear(); c.current = effect.Serial();
    c.failCallback = true; check("T71 failed callback balances pins and QIs", c, -1);
    c.failCallback = false; check("T71 successful callback balances pins and QIs", c, 1);
    for (auto* input : {effect.Prop(), static_cast<cIGZUnknown*>(effect.Serial())}) {
        void* serial = nullptr; void* field = nullptr;
        if (!input->QueryInterface(GZIID_cIGZSerializable, &serial) || serial != effect.Serial()) ++failures;
        auto* acquired = static_cast<cIGZSerializable*>(serial);
        if (!acquired->QueryInterface(kEffect, &field) || field != effect.Class()) ++failures;
        if (field) static_cast<cIGZUnknown*>(field)->Release();
        if (acquired) acquired->Release();
    }
    if (effect.refs != 1) ++failures; // T72: owner remains alive
    c.sparks = {effect.Prop()};
    const auto membership = c.sparks;
    check("T73 first unchanged capture", c, 1);
    const auto identities = c.callbacks;
    check("T73 second unchanged capture", c, 1);
    if (c.sparks != membership || c.callbacks != identities || c.current != effect.Serial()) ++failures;
    // The live-name record portion of T73 is checked by the Rust codec test.
    for (auto* object : {&effect, &ground, &roof, &crew, &bare}) {
        if (object->refs != 1 || object->adds != object->releases) ++failures;
    }
    if (bare.rejected < 3) ++failures;
    FreeLibrary(dll);
    std::printf("LR13 T66-T73 construction capture: %d failures\n", failures);
    return failures == 0 ? 0 : 5;
}
