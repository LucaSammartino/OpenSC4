// SPDX-License-Identifier: GPL-3.0-or-later
// Synthetic section-11 graph through the actual DLL's capture/core/output
// path, including an opaque external object and failed record versions.
#include "FC4TestSeam.h"
#include "FC4Streams.h"
#include "cIGZSerializable.h"
#include <windows.h>
#include <cstdio>
#include <initializer_list>

namespace {
struct Record final : cIGZSerializable {
    uint32_t cls, iid, refs = 1, reads = 0, writes = 0;
    uint16_t version;
    Record* child = nullptr;
    Record(uint32_t c, uint32_t i, uint16_t v) : cls(c), iid(i), version(v) {}
    bool QueryInterface(uint32_t id, void** out) override {
        if (out == nullptr) return false;
        *out = nullptr;
        if (id != GZIID_cIGZUnknown && id != GZIID_cIGZSerializable && id != iid) return false;
        *out = static_cast<cIGZSerializable*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return --refs; }
    bool Read(cIGZIStream&) override { ++reads; return false; }
    uint32_t GetGZCLSID() override { return cls; }
    bool Write(cIGZOStream& out) override {
        ++writes;
        auto& segment = static_cast<cFC4ISC4DBSegmentOStream&>(out);
        out.SetUint16(version);
        if (cls == 0x4A0DD616) {
            out.SetUint32(1);
            segment.WriteGZSerializable(child);
            out.SetSint32(25);
        } else if (cls == 0xEAC937B4) {
            out.SetSint32(7);  // table index
            out.SetRZCharStr("Environment");
            out.SetRZCharStr("A. Test");
            out.SetSint32(4);  // month counter
            out.SetSint32(12); // current day
            out.SetSint32(25); // advisor ID
            out.SetSint32(-1); // cursor
            out.SetUint32(0);  // advice count
            out.SetUint32(1);  // event queue
            out.SetSint32(0x12345678);
            segment.WriteGZSerializable(child);
        } else if (cls == 0x6A26D049) {
            out.SetUint8(1);
            out.SetFloat32(12.5f);
            out.SetFloat32(-7.25f);
            out.SetUint32(0xFF0055AA);
            out.SetUint32(42);
            segment.WriteGZSerializable(child);
        }
        return out.GetError() == 0;
    }
};
}

int main(int argc, char** argv) {
    if (argc != 2) return 1;
    HMODULE dll = LoadLibraryA(argv[1]);
    if (dll == nullptr) return 2;
    auto seam = reinterpret_cast<FC4TestSeamFn>(GetProcAddress(dll, "FC4TestSeam"));
    if (seam == nullptr) return 3;
    Record root(0x4A0DD616, 0x4A0DD617, 1);
    Record advisor(0xEAC937B4, 0xCA09F544, 3);
    Record subject(0x6A26D049, 0x6A26D050, 4);
    Record external(0x11112222, 0x99999999, 0);
    root.child = &advisor;
    advisor.child = &subject;
    subject.child = &external;
    int failures = 0;
    auto check = [&](const char* name, intptr_t expected, void* object) {
        const intptr_t actual = seam(kFC4SeamAdvisorsShadow, object, nullptr);
        if (actual != expected) {
            std::printf("%s: expected %ld mismatches, got %ld\n", name, static_cast<long>(expected), static_cast<long>(actual));
            ++failures;
        }
    };
    check("valid graph", 0, &root);
    if (root.writes != 1 || advisor.writes != 1 || subject.writes != 1 || external.writes != 0) ++failures;
    root.version = 2;
    check("bad system version", 1, &root);
    root.version = 1;
    subject.version = 3;
    check("bad subject version", 1, &root);
    check("missing system", 1, nullptr);
    for (Record* r : {&root, &advisor, &subject, &external}) {
        if (r->refs != 1 || r->reads != 0) {
            std::printf("class=%08X refs=%u native_reads=%u\n", r->cls, r->refs, r->reads);
            ++failures;
        }
    }
    FreeLibrary(dll);
    std::printf("advisor record graph: %d failures\n", failures);
    return failures == 0 ? 0 : 4;
}
