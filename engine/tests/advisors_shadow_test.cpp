// SPDX-License-Identifier: GPL-3.0-or-later
// Synthetic section-11 graph through the actual DLL's capture/core/output
// path, including an opaque external object and failed record versions.
#include "FC4TestSeam.h"
#include "FC4Streams.h"
#include "cFC4AdvisorObjects.h"
#include "cIGZSerializable.h"
#include <windows.h>
#include <cstdio>
#include <initializer_list>

namespace {
struct Record : cIGZSerializable {
    uint32_t cls, iid, refs = 1, reads = 0, writes = 0;
    uint16_t version;
    Record* child = nullptr;
    cIGZSerializable* advice[2] = {};
    uint32_t flags = 0x12;
    bool extraByte = false;
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
        } else if (cls == 0xEAC937B4 || cls == 0x4BE372CD) {
            out.SetSint32(7);  // table index
            out.SetRZCharStr("Environment");
            out.SetRZCharStr("A. Test");
            out.SetSint32(4);  // month counter
            out.SetSint32(12); // current day
            out.SetSint32(25); // advisor ID
            out.SetSint32(-1); // cursor
            out.SetUint32(advice[0] == nullptr ? 0 : 2);
            if (advice[0] != nullptr) {
                for (auto* a : advice) {
                    segment.WriteGZSerializable(a);
                    out.SetSint32(40);  // expiry
                    out.SetSint32(3);   // last month
                    out.SetSint32(15);  // next day
                    out.SetUint8(1);    // active
                    out.SetSint32(0);   // event ID
                }
            }
            out.SetUint32(1);  // event queue
            out.SetSint32(0x12345678);
            segment.WriteGZSerializable(child);
        } else if (cls == 0x8BE3753B || cls == 0x8A09F5F4) {
            out.SetSint32(7);
            out.SetUint32(this == child ? 0xABC : 0xDEF);
            out.SetUint32(flags);
            out.SetSint32(25);
            out.SetSint32(17);  // priority
            out.SetSint32(35);  // frequency
            out.SetSint32(45);  // timeout
            out.SetSint32(-1);  // mood
            out.SetUint32(2);
            out.SetUint32(0x123);
            out.SetUint32(0x456);
            segment.WriteGZSerializable(child == this ? nullptr : child);
        } else if (cls == 0x6A26D049) {
            out.SetUint8(1);
            out.SetFloat32(12.5f);
            out.SetFloat32(-7.25f);
            out.SetUint32(0xFF0055AA);
            out.SetUint32(42);
            segment.WriteGZSerializable(child);
        }
        if (extraByte) out.SetUint8(0xA5);
        return out.GetError() == 0;
    }
};

// Only QI and GetEventID may be called on the advice interface. All other
// slots are inert: the shadow must neither Init nor mutate the native advice.
struct Advice final : Record, cFC4ISC4Advice {
    Advice() : Record(0x8BE3753B, kFC4_IID_cISC4Advice, 4) {}
    bool QueryInterface(uint32_t id, void** out) override {
        if (out != nullptr && id == kFC4_IID_cISC4Advice) {
            *out = static_cast<cFC4ISC4Advice*>(this);
            AddRef();
            return true;
        }
        return Record::QueryInterface(id, out);
    }
    uint32_t AddRef() override { return Record::AddRef(); }
    uint32_t Release() override { return Record::Release(); }
    bool Init(int32_t) override { return false; }
    bool Shutdown() override { return false; }
    bool Clone(cFC4ISC4Advice**) override { return false; }
    int32_t GetMood() override { return -1; }
    int32_t GetPriority() override { return 17; }
    bool GetTrigger() override { return false; }
    cIGZString* GetTriggerString() override { return nullptr; }
    uint32_t GetGUID() override { return 0; }
    int32_t GetAdvisorID() override { return 25; }
    cFC4ISC4Advice* SetAdvisorID(int32_t) override { return this; }
    int32_t GetSessionID() override { return 0; }
    bool SetFlags(uint32_t, bool) override { return false; }
    bool CheckFlags(uint32_t mask) override { return (flags & mask) == mask; }
    int32_t GetFrequencyInDays() override { return 35; }
    int32_t GetTimeOutInDays() override { return 45; }
    cIGZString* GetTextTitle() override { return nullptr; }
    uint32_t GetTextTitleGUID() override { return 0; }
    cIGZString* GetTextMessage() override { return nullptr; }
    uint32_t GetTextMessageGUID() override { return 0; }
    bool DoLink(const cIGZString&) override { return false; }
    void NotifyGlobalEffectTriggerChanged() override {}
    bool SetEventSubject(cISC4AdviceSubject*) override { return false; }
    cISC4AdviceSubject* GetEventSubject() override { return nullptr; }
    int32_t GetEventID() override { return 0; }
    int32_t GetCommandID() override { return 0; }
    bool UpdateText() override { return false; }
    bool RebuildText() override { return false; }
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
    Advice first, second;
    first.child = &subject;
    second.child = &second;  // distinct GUID, null subject
    advisor.cls = 0x4BE372CD;
    advisor.advice[0] = &first;
    advisor.advice[1] = &second;
    check("AD17 city advice base records and shared subject", 0, &root);
    first.flags = 2;
    check("AD19 clean city advice expected Read fixup", 0, &root);
    if (first.flags != 2) ++failures;  // native captured flags stay clean
    first.cls = 0x8A09F5F4;
    check("AD19 clean plain advice expected Read fixup", 0, &root);
    if (first.flags != 2) ++failures;
    first.flags = 0x80000002;  // advisor entry's active byte agrees with bit 2
    check("AD19 high flag bits preserved with Read fixup", 0, &root);
    if (first.flags != 0x80000002) ++failures;
    first.extraByte = true;
    check("AD19 never ignores other bytes with a clean flag", 1, static_cast<Record*>(&first));
    first.extraByte = false;
    first.version = 3;
    check("AD19 never normalizes a rejected advice version", 1, static_cast<Record*>(&first));
    first.version = 4;
    first.flags = 0x12;
    check("AD18 already dirty advice exact", 0, &root);
    root.version = 2;
    check("bad system version", 1, &root);
    root.version = 1;
    subject.version = 3;
    check("bad subject version", 1, &root);
    check("missing system", 1, nullptr);
    for (Record* r : {&root, &advisor, &subject, &external, static_cast<Record*>(&first), static_cast<Record*>(&second)}) {
        if (r->refs != 1 || r->reads != 0) {
            std::printf("class=%08X refs=%u native_reads=%u\n", r->cls, r->refs, r->reads);
            ++failures;
        }
    }
    FreeLibrary(dll);
    std::printf("advisor record graph: %d failures\n", failures);
    return failures == 0 ? 0 : 4;
}
