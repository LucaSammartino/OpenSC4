// SPDX-License-Identifier: GPL-3.0-or-later
// Constructed native verification, commercial-industrial sections 1,2,3,8,9.
// No forwarding implementation, native table mutation or IN3 coverage.
#include "FC4IndustrialShadow.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "compat/FC4IndustrialCompat.h"
#include "cFC4Commercial.h"
#include "cFC4Message2Standard.h"
#include "GZServPtrs.h"
#include "cIGZCOM.h"
#include "cIGZIStream.h"
#include "cIGZOStream.h"
#include "cISC4City.h"
#include "cRZCOMDllDirector.h"
#include <cstring>
#include <vector>

namespace fc4 {
namespace {
bool enabled = false;
uint32_t suites = 0, refusals = 0, totalChecks = 0, totalDiffs = 0;
constexpr uint32_t serialIID = 0xE4FDA3D4, messageIID = 0x452294AA;

// Typed public stream interfaces. Independent stream instances for each side;
// adapter callbacks do not call either owner. Unexpected operations are counted.
struct ProbeInput final : cIGZIStream {
    std::vector<uint8_t> bytes;
    size_t pos = 0;
    int32_t error = 0;
    uint32_t calls = 0, unexpected = 0;
    bool QueryInterface(uint32_t iid, void**) override {
        // The Rust adapter negotiates segment references before any owner call.
        // This record has none; refusal is stream setup, not an owner operation.
        if (iid != kFC4_IID_cISC4DBSegmentIStream) ++unexpected;
        return false;
    }
    uint32_t AddRef() override { ++unexpected; return 1; }
    uint32_t Release() override { ++unexpected; return 1; }
    bool Take(void* p, size_t n) {
        ++calls;
        if (n > bytes.size() - pos) { error = 1; return false; }
        if (p && n) std::memcpy(p, bytes.data() + pos, n);
        pos += n;
        return true;
    }
    bool Skip(uint32_t n) override { ++unexpected; return Take(nullptr, n); }
#define FC4_IN_PROBE(method, type) bool method(type& v) override { ++unexpected; return Take(&v, sizeof(v)); }
    FC4_IN_PROBE(GetSint8, int8_t)
    FC4_IN_PROBE(GetUint8, uint8_t)
    FC4_IN_PROBE(GetSint16, int16_t)
    bool GetUint16(uint16_t& v) override { return Take(&v, 2); }
    FC4_IN_PROBE(GetSint32, int32_t)
    FC4_IN_PROBE(GetUint32, uint32_t)
    FC4_IN_PROBE(GetSint64, int64_t)
    FC4_IN_PROBE(GetUint64, uint64_t)
    FC4_IN_PROBE(GetFloat32, float)
    FC4_IN_PROBE(GetFloat64, double)
#undef FC4_IN_PROBE
    bool Fail() { ++unexpected; error = 1; return false; }
    bool GetRZCharStr(char*, uint32_t) override { return Fail(); }
    bool GetGZStr(cIGZString&) override { return Fail(); }
    bool GetGZSerializable(cIGZSerializable&) override { return Fail(); }
    bool GetVoid(void* p, uint32_t n) override { ++unexpected; return Take(p, n); }
    int32_t GetError() override { return error; }
    int32_t SetUserData(cIGZVariant*) override { ++unexpected; return 0; }
    int32_t GetUserData() override { ++unexpected; return 0; }
};
struct ProbeOutput final : cIGZOStream {
    std::vector<uint8_t> bytes;
    size_t capacity = 2;
    int32_t error = 0;
    uint32_t calls = 0, unexpected = 0;
    bool QueryInterface(uint32_t iid, void**) override {
        if (iid != kFC4_IID_cISC4DBSegmentOStream) ++unexpected;
        return false;
    }
    uint32_t AddRef() override { ++unexpected; return 1; }
    uint32_t Release() override { ++unexpected; return 1; }
    void Flush() override { ++unexpected; }
    bool Put(const void* p, size_t n) {
        ++calls;
        if (n > capacity - bytes.size()) { error = 1; return false; }
        const auto* b = static_cast<const uint8_t*>(p);
        bytes.insert(bytes.end(), b, b + n);
        return true;
    }
#define FC4_OUT_PROBE(method, type) bool method(type v) override { ++unexpected; return Put(&v, sizeof(v)); }
    FC4_OUT_PROBE(SetSint8, int8_t)
    FC4_OUT_PROBE(SetUint8, uint8_t)
    FC4_OUT_PROBE(SetSint16, int16_t)
    bool SetUint16(uint16_t v) override { return Put(&v, 2); }
    FC4_OUT_PROBE(SetSint32, int32_t)
    FC4_OUT_PROBE(SetUint32, uint32_t)
    FC4_OUT_PROBE(SetSint64, int64_t)
    FC4_OUT_PROBE(SetUint64, uint64_t)
    FC4_OUT_PROBE(SetFloat32, float)
    FC4_OUT_PROBE(SetFloat64, double)
#undef FC4_OUT_PROBE
    bool Fail() { ++unexpected; error = 1; return false; }
    bool SetRZCharStr(const char*) override { return Fail(); }
    bool SetGZStr(const cIGZString&) override { return Fail(); }
    bool SetGZSerializable(const cIGZSerializable&) override { return Fail(); }
    bool SetVoid(const void* p, uint32_t n) override { ++unexpected; return Put(p, n); }
    int32_t GetError() override { return error; }
    int32_t SetUserData(cIGZVariant*) override { ++unexpected; return 0; }
    int32_t GetUserData() override { ++unexpected; return 0; }
};

struct Pair {
    cISC4IndustrialSimulator* native = nullptr;
    fc4_industrial* rust = nullptr;
    cIGZSerializable* serial = nullptr;  // held successful QI
    cIGZMessageTarget2* target = nullptr; // held successful QI
    uint32_t checks = 0, diffs = 0, reads = 0, writes = 0, messages = 0, refs = 0;
    uint32_t readFailuresNative = 0, readFailuresRust = 0, writeFailuresNative = 0, writeFailuresRust = 0;
    uint32_t readBytesNative = 0, readBytesRust = 0, writeBytesNative = 0, writeBytesRust = 0;
    void Check(bool ok, const char* what, uint32_t arg = 0) {
        ++checks;
        if (!ok && ++diffs <= 8) Log("industrial constructed diff: %s arg=%u", what, arg);
    }
    void State(uint32_t stage) {
        bool initialized = false;
        Check(compat::IndustrialInitialized(native, &initialized) &&
              initialized == fc4_industrial_is_initialized(rust), "initialized", stage);
    }
    void Read(const std::vector<uint8_t>& bytes, int32_t error, uint32_t tag) {
        ProbeInput n, r;
        n.bytes = r.bytes = bytes;
        n.error = r.error = error;
        IStreamAdapter adapter(r);
        const bool a = serial->Read(n), b = fc4_industrial_read(rust, adapter.Get());
        readFailuresNative += !a; readFailuresRust += !b;
        readBytesNative += uint32_t(n.pos); readBytesRust += uint32_t(r.pos);
        if (tag <= 2) Log("industrial record Read sample %u: return=%u/%u pos=%u/%u error=%d/%d calls=%u/%u unexpected=%u/%u",
            tag, a, b, unsigned(n.pos), unsigned(r.pos), n.error, r.error, n.calls, r.calls, n.unexpected, r.unexpected);
        Check(a == b && n.pos == r.pos && n.error == r.error && n.calls == r.calls &&
              n.unexpected == 0 && r.unexpected == 0, "Read return/position/error/calls", tag);
        ++reads;
    }
    void Write(size_t capacity, int32_t error) {
        ProbeOutput n, r;
        n.capacity = r.capacity = capacity;
        n.error = r.error = error;
        OStreamAdapter adapter(r);
        const bool a = serial->Write(n), b = fc4_industrial_write(rust, adapter.Get());
        writeFailuresNative += !a; writeFailuresRust += !b;
        writeBytesNative += uint32_t(n.bytes.size()); writeBytesRust += uint32_t(r.bytes.size());
        if (writes < 8) Log("industrial record Write sample capacity=%u input_error=%d: return=%u/%u size=%u/%u error=%d/%d calls=%u/%u unexpected=%u/%u",
            unsigned(capacity), error, a, b, unsigned(n.bytes.size()), unsigned(r.bytes.size()), n.error, r.error, n.calls, r.calls, n.unexpected, r.unexpected);
        Check(a == b && n.bytes == r.bytes && n.error == r.error && n.calls == r.calls &&
              n.unexpected == 0 && r.unexpected == 0, "Write return/bytes/error/calls", uint32_t(capacity));
        if (capacity >= 2) Check(n.bytes == std::vector<uint8_t>({1, 0}), "version bytes");
        ++writes;
    }
    bool Acquire() {
        auto* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
        void* p = nullptr;
        if (!compat::IdentifiedVersion() || !com ||
            !com->GetClassObject(kFC4_CLSID_cSC4IndustrialSimulator, kFC4_IID_cISC4IndustrialSimulator, &p) || !p) return false;
        native = static_cast<cISC4IndustrialSimulator*>(p);
        FC4_ACQUIRED(native, "industrial original factory");
        if (!compat::IndustrialOriginal(native)) return false;
        rust = fc4_industrial_create();
        if (!rust) return false;
        Check(fc4_industrial_add_ref(rust) == 1, "Rust constructed owner ref");
        const uint32_t iids[] = {1, kFC4_IID_cISC4IndustrialSimulator, messageIID, serialIID, 0, 0xFFFFFFFF};
        for (uint32_t iid : iids) {
            void* view = reinterpret_cast<void*>(uintptr_t(0x1234));
            const bool a = native->QueryInterface(iid, &view), b = fc4_industrial_query_interface(iid);
            Check(a == b, "QI result", iid);
            if (a) {
                auto* u = static_cast<cIGZUnknown*>(view);
                FC4_ACQUIRED(u, "industrial QI");
                fc4_industrial_add_ref(rust);
                const uintptr_t offset = iid == messageIID ? 4 : iid == serialIID ? 8 : 0;
                Check(reinterpret_cast<uintptr_t>(view) == reinterpret_cast<uintptr_t>(native) + offset, "QI view", iid);
                void* back = nullptr;
                Check(u->QueryInterface(1, &back) && back == native, "QI canonical identity", iid);
                if (back) {
                    FC4_ACQUIRED(static_cast<cIGZUnknown*>(back), "industrial canonical QI");
                    fc4_industrial_add_ref(rust);
                    Check(FC4_RELEASE(static_cast<cIGZUnknown*>(back)) == fc4_industrial_release(rust), "QI back release", iid);
                    ++refs;
                }
                Check(FC4_RELEASE(u) == fc4_industrial_release(rust), "QI release", iid);
                ++refs;
            } else Check(view == reinterpret_cast<void*>(uintptr_t(0x1234)), "QI failure untouched", iid);
        }
        void* s = nullptr;
        if (!native->QueryInterface(serialIID, &s) || !s) return false;
        serial = static_cast<cIGZSerializable*>(s);
        FC4_ACQUIRED(serial, "industrial serial QI");
        fc4_industrial_add_ref(rust);
        s = nullptr;
        if (!native->QueryInterface(messageIID, &s) || !s) return false;
        target = static_cast<cIGZMessageTarget2*>(s);
        FC4_ACQUIRED(target, "industrial message QI");
        fc4_industrial_add_ref(rust);
        Check(serial->GetGZCLSID() == kFC4_CLSID_cSC4IndustrialSimulator, "CLSID");
        for (unsigned i = 0; i < 4; ++i) {
            Check(FC4_ADDREF(native) == fc4_industrial_add_ref(rust), "AddRef", i);
            ++refs;
        }
        for (unsigned i = 0; i < 4; ++i) {
            Check(FC4_RELEASE(native) == fc4_industrial_release(rust), "Release", i);
            ++refs;
        }
        return true;
    }
    void Run() {
        // Exhaust every representable record version before Init (Read-before-Init).
        for (uint32_t version = 0; version < 65536; ++version)
            Read({uint8_t(version), uint8_t(version >> 8)}, 0, version);
        Read({}, 0, 65536); Read({1}, 0, 65537);
        Read({1, 0, 0xAA, 0x55}, 0, 65538); // only the defined record consumed
        Read({1, 0}, 7, 65539); Read({0, 0}, 7, 65540);
        for (uint32_t stage = 0; stage < 7; ++stage) {
            if (stage == 1 || stage == 2 || stage == 5)
                // [abi-ret] commercial-industrial section 9 publishes slot 3 as AL=1.
                Check(native->Init() == fc4_industrial_init(rust), "Init", stage);
            if (stage == 3 || stage == 4 || stage == 6)
                // [abi-ret] commercial-industrial section 9 publishes slot 4 as AL=1.
                Check(native->Shutdown() == fc4_industrial_shutdown(rust), "Shutdown", stage);
            State(stage);
            Read({1, 0}, 0, stage); Read({0xFF, 0xFF}, 0, stage); Read({}, 0, stage);
            for (size_t capacity = 0; capacity <= 3; ++capacity) {
                Write(capacity, 0); Write(capacity, 7);
            }
            const uint32_t types[] = {0, 1, 0xFFFFFFFF, 0x66956814, 0x66956816, 0x6695681A, 0x99EF1142, 0x99EF1143};
            for (uint32_t type : types) {
                auto* msg = cFC4Message2Standard::New();
                if (!msg) { Check(false, "message allocation"); continue; }
                FC4_ADDREF(msg);
                msg->SetType(type);
                Check(target->DoMessage(msg) == fc4_industrial_do_message(rust, type), "DoMessage", type);
                FC4_RELEASE_CREATED(msg);
                ++messages;
            }
            State(stage); // records/messages must not change initialization
        }
        // Final Release from initialized state exercises implicit Shutdown/destruction.
        // [abi-ret] commercial-industrial section 9 publishes Init slot 3 as AL=1.
        Check(native->Init() == fc4_industrial_init(rust), "final Init");
        State(7);
    }
    void Free() {
        if (target) { const auto n = FC4_RELEASE(target); if (rust) Check(n == fc4_industrial_release(rust), "message view Release"); target = nullptr; ++refs; }
        if (serial) { const auto n = FC4_RELEASE(serial); if (rust) Check(n == fc4_industrial_release(rust), "serial view Release"); serial = nullptr; ++refs; }
        if (native) { const auto n = FC4_RELEASE_CREATED(native); if (rust) Check(n == fc4_industrial_release(rust) && n == 0, "final Release"); native = nullptr; ++refs; }
        if (rust) { Check(!fc4_industrial_is_initialized(rust), "implicit shutdown"); fc4_industrial_destroy(rust); rust = nullptr; }
    }
};
}

void IndustrialShadowStartup(bool replaced) {
    enabled = SettingInt("industrial_shadow", "FC4_INDUSTRIAL_SHADOW", 0) != 0;
    suites = refusals = totalChecks = totalDiffs = 0;
    if (enabled && replaced) {
        enabled = false;
        refusals = 1;
        Log("industrial constructed: REFUSED replacement enabled; suites=0 refusals=1 no native oracle");
    } else if (enabled) Log("industrial constructed: ON independent original factory / Rust; IN3 excluded");
    else Log("industrial constructed: OFF suites=0 refusals=0");
}

void IndustrialShadowProbe(cISC4City* city, const char* phase) {
    if (!enabled) return;
    // Read-only city evidence at precisely this phase; never infer earlier Read/Init.
    auto* natural = city ? city->GetIndustrialSimulator() : nullptr; // borrowed, no Release
    bool initialized = false;
    const bool validated = compat::IndustrialInitialized(natural, &initialized);
    Log("industrial city snapshot %s: validated=%u initialized=%u earlier_lifecycle=unobserved", phase, validated, initialized);
    Pair pair;
    const bool acquired = pair.Acquire();
    if (acquired) { pair.Check(pair.native != natural, "independent of city"); pair.Run(); ++suites; }
    else { ++refusals; Log("industrial constructed %s: REFUSED factory/image/views/core unavailable", phase); }
    pair.Free();
    Log("industrial constructed outcomes %s: read_failures=%u/%u write_failures=%u/%u read_bytes=%u/%u write_bytes=%u/%u",
        phase, pair.readFailuresNative, pair.readFailuresRust, pair.writeFailuresNative, pair.writeFailuresRust,
        pair.readBytesNative, pair.readBytesRust, pair.writeBytesNative, pair.writeBytesRust);
    totalChecks += pair.checks; totalDiffs += pair.diffs;
    Log("industrial constructed %s: suites=%u refusals=%u checks=%u mismatches=%u reads=%u writes=%u messages=%u refs=%u total_checks=%u total_mismatches=%u natural_interception=0",
        phase, suites, refusals, pair.checks, pair.diffs, pair.reads, pair.writes, pair.messages, pair.refs, totalChecks, totalDiffs);
}
}
