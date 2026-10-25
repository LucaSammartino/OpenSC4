// SPDX-License-Identifier: GPL-3.0-or-later
// lot-records 9/9.1.1: core record storage and literal field-reference ownership.
#include "cFC4ConstructionRecord.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4WriteCapture.h"
#include "cIGZSerializable.h"
#include <atomic>
#include <mutex>
#include <new>
#include <vector>

extern "C" void* fc4_construction_record_create();
extern "C" void fc4_construction_record_destroy(void*);
extern "C" bool fc4_construction_record_read(void*, const fc4_istream*);
extern "C" bool fc4_construction_record_write(const void*, const fc4_ostream*);

namespace {
void ReleaseFields(std::vector<cIGZUnknown*>& fields) {
    for (auto*& field : fields) FC4_RELEASE_AT_DESTRUCTION(field);
    fields.clear();
}

// A read acquires one reference per nonnull field, including duplicates. Never
// deduplicate by object identity: each acquisition has its own release.
struct TrackedInput {
    const fc4_istream& source;
    fc4_istream api{};
    std::vector<cIGZUnknown*> fields;
    explicit TrackedInput(const fc4_istream& s) : source(s) {
        api.struct_size = sizeof(api);
        api.user = this;
#define FC4_CONSTRUCTION_GET(name, type) \
        api.name = [](void* p, type* value) { \
            auto& t = *static_cast<TrackedInput*>(p); \
            return t.source.name && t.source.name(t.source.user, value); \
        }
        FC4_CONSTRUCTION_GET(get_u8, uint8_t);
        FC4_CONSTRUCTION_GET(get_u16, uint16_t);
        FC4_CONSTRUCTION_GET(get_u32, uint32_t);
        FC4_CONSTRUCTION_GET(get_u64, uint64_t);
        FC4_CONSTRUCTION_GET(get_f32, float);
        FC4_CONSTRUCTION_GET(get_f64, double);
#undef FC4_CONSTRUCTION_GET
        api.get_bytes = [](void* p, uint8_t* bytes, size_t count) {
            auto& t = *static_cast<TrackedInput*>(p);
            return t.source.get_bytes && t.source.get_bytes(t.source.user, bytes, count);
        };
        api.get_str = [](void* p, fc4_bytes_sink sink, void* context) {
            auto& t = *static_cast<TrackedInput*>(p);
            return t.source.get_str && t.source.get_str(t.source.user, sink, context);
        };
        api.get_ref = [](void* p, uint32_t iid, fc4_ref* out) {
            auto& t = *static_cast<TrackedInput*>(p);
            *out = {};
            const bool ok = t.source.get_ref && t.source.get_ref(t.source.user, iid, out);
            if (ok && out->token) {
                auto* field = fc4::FromToken(out->token);
                // Existing stream adapters return our objects as borrowed
                // tokens, expecting a core resolver to retain them. This
                // record deliberately keeps host tokens, so take that ref here.
                if (fc4::IsOurObject(field)) FC4_ADDREF(field);
                t.fields.push_back(field);
            }
            return ok;
        };
        api.skip = [](void* p, size_t count) {
            auto& t = *static_cast<TrackedInput*>(p);
            return t.source.skip && t.source.skip(t.source.user, count);
        };
        api.error = [](void* p) {
            auto& t = *static_cast<TrackedInput*>(p);
            return t.source.error ? t.source.error(t.source.user) : 0;
        };
    }
    ~TrackedInput() { ReleaseFields(fields); }
};

class ConstructionRecord final : public cIGZSerializable {
public:
    ConstructionRecord() : core_(fc4_construction_record_create()) {}
    ~ConstructionRecord() {
        // Drop the core's opaque tokens before releasing the matching fields.
        fc4_construction_record_destroy(core_);
        ReleaseFields(fields_);
    }
    bool Valid() const { return core_ != nullptr; }
    bool QueryInterface(uint32_t iid, void** output) override {
        if (!output) return false;
        *output = nullptr;
        if (iid != GZIID_cIGZUnknown && iid != GZIID_cIGZSerializable) return false;
        *output = static_cast<cIGZSerializable*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override {
        const auto n = --refs_;
        if (!n) delete this;
        return n;
    }
    uint32_t GetGZCLSID() override { return 0xA97F909E; }
    bool Write(cIGZOStream& stream) override {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        fc4::OStreamAdapter output(stream);
        return fc4_construction_record_write(core_, output.Get());
    }
    bool Read(cIGZIStream& stream) override {
        fc4::IStreamAdapter input(stream);
        return ReadCore(*input.Get());
    }
    bool ReadCore(const fc4_istream& input) {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        if (input.struct_size < sizeof(fc4_istream)) return false;
        TrackedInput tracked(input);
        if (!fc4_construction_record_read(core_, &tracked.api)) return false;
        // Acquire every new field before dropping old fields, including when
        // a second Read resolves to the same native objects as the first.
        fields_.swap(tracked.fields);
        return true;
    }
private:
    std::atomic<uint32_t> refs_{0};
    std::recursive_mutex mutex_;
    void* core_;
    std::vector<cIGZUnknown*> fields_;
};
}

cIGZUnknown* CreateFC4ConstructionRecord() {
    auto* record = new (std::nothrow) ConstructionRecord;
    if (record && !record->Valid()) { delete record; return nullptr; }
    return record;
}

namespace fc4 {
intptr_t ConstructionRecordFactoryTest(cIGZUnknown* source, void* output) {
    if (!output) return -1;
    auto** result = static_cast<cIGZSerializable**>(output);
    *result = nullptr;
    auto* unknown = CreateFC4ConstructionRecord();
    if (!unknown) return -1;
    auto* record = static_cast<ConstructionRecord*>(static_cast<cIGZSerializable*>(unknown));
    record->AddRef();
    if (source) {
        WriteCapture capture(true, true);
        if (!capture.Capture(source)) { record->Release(); return -1; }
        CaptureIStream input(capture);
        if (!record->ReadCore(*input.Get()) || !input.good() || !input.atEnd()) {
            record->Release();
            return 0;
        }
    }
    *result = record;
    return 1;
}
}
