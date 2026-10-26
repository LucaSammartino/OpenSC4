// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4EffectsResource.h"
#include "compat/FC4Compat.h"
#include "cIGZIStream.h"
#include "cIGZOStream.h"
#include "cIGZString.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <stdexcept>
#include <thread>
#include <vector>

namespace fc4 { void Log(const char*, ...) {} }

#define CHECK(expr) do { if (!(expr)) { std::fprintf(stderr, "line %d: %s\n", __LINE__, #expr); return 1; } } while (0)

// Production compatibility refusal is also exercised without a game image.
namespace fc4::compat {
uintptr_t DirectCall(Direct) { return 0; }
bool StlNodeAllocator(StlNodeAllocateFn*, StlNodeFreeFn*) { return false; }
const void* GameStringVtable() { return nullptr; }
}

namespace {
unsigned constructed = 0, destroyed = 0, allocated = 0, freed = 0, published = 0;
void* publicationSource = nullptr;
void* publicationDestination = nullptr;
void* table[24]{};
void* Allocate(uint32_t size);
void Construct(void* p) {
    ++constructed;
    std::memset(p, 0, 0x118);
    *static_cast<void***>(p) = table;
    // Native collections construct all thirteen directory slots (18.7.2).
    auto* directories = static_cast<uint8_t*>(Allocate(13 * 0x20));
    std::memset(directories, 0, 13 * 0x20);
    auto** vector = reinterpret_cast<uint8_t**>(static_cast<uint8_t*>(p) + 0xC4);
    vector[0] = directories;
    vector[1] = vector[2] = directories + 13 * 0x20;
}
void DestroyContents(void*);
void Destroy(void* p) { ++destroyed; DestroyContents(p); }
void Publish(void* source, void* destination) {
    ++published;
    publicationSource = source;
    publicationDestination = destination;
}
void* Allocate(uint32_t size) { ++allocated; return std::malloc(size); }
void Free(void* p) { ++freed; std::free(p); }
FC4EffectsResourceBindings bindings{Construct, Destroy, Publish, Allocate, Free, nullptr};

struct Vector { uint8_t* begin; uint8_t* end; uint8_t* capacity; };
template<class T> T& Field(void* p, size_t offset) {
    return *reinterpret_cast<T*>(static_cast<uint8_t*>(p) + offset);
}
void FreeVector(Vector& v) { if (v.begin) Free(v.begin); v = {}; }
void DestroyDecal(void* p) {
    for (size_t offset : {0x10, 0x1C, 0x28, 0x34, 0x40}) FreeVector(Field<Vector>(p, offset));
}
void DestroyContents(void* p) {
    auto& decals = Field<Vector>(p, 0x78);
    for (auto* d = decals.begin; d != decals.end; d += 0x64) DestroyDecal(d);
    FreeVector(decals);
    for (size_t offset : {0x98, 0xB8, 0xF0, 0xFC}) {
        auto& values = Field<Vector>(p, offset);
        const size_t stride = offset == 0x98 ? 0x28 : offset == 0xB8 ? 0x1C : offset == 0xF0 ? 0x14 : 0x10;
        for (auto* v = values.begin; v != values.end; v += stride) {
            if (offset == 0x98 || offset == 0xB8) {
                FreeVector(Field<Vector>(v, offset == 0x98 ? 8 : 0));
                FreeVector(Field<Vector>(v, offset == 0x98 ? 0x14 : 0xC));
            } else FreeVector(Field<Vector>(v, offset == 0xF0 ? 0 : 4));
        }
        FreeVector(values);
    }
    FreeVector(Field<Vector>(p, 0xD4));
    FreeVector(Field<Vector>(p, 0xC4));
}
Vector CopyVector(const Vector& source) {
    if (source.begin == source.end) return {};
    const auto n = static_cast<uint32_t>(source.end - source.begin);
    auto* copy = static_cast<uint8_t*>(Allocate(n));
    std::memcpy(copy, source.begin, n);
    return {copy, copy + n, copy + n};
}
void CopyDecals(void* source, void* destination) {
    const auto& input = Field<Vector>(source, 0x78);
    auto& output = Field<Vector>(destination, 0x78);
    output = CopyVector(input);
    for (auto* p = output.begin; p != output.end; p += 0x64) {
        const auto* original = input.begin + (p - output.begin);
        for (size_t offset : {0x10, 0x1C, 0x28, 0x34, 0x40})
            Field<Vector>(p, offset) = CopyVector(*reinterpret_cast<const Vector*>(original + offset));
    }
}

template<class Result, class... Args>
Result Slot(void* receiver, size_t slot, Args... args) {
    using Function = Result(__thiscall*)(void*, Args...);
    return reinterpret_cast<Function>((*static_cast<void***>(receiver))[slot])(receiver, args...);
}

class Stream final : public cIGZIStream, public cIGZOStream {
public:
    std::vector<uint8_t> bytes;
    size_t cursor = 0;
    unsigned calls = 0, throwAt = 0, strings = 0, retains = 0, releases = 0, queries = 0, errors = 0;
    bool status = true, leaveRoot = false;
    bool QueryInterface(uint32_t, void**) override { ++queries; return false; }
    uint32_t AddRef() override { ++retains; return 1; }
    uint32_t Release() override { ++releases; return 1; }
    void Before() { if (++calls == throwAt) throw std::runtime_error("interrupted stream"); }
    template<class T> bool Put(T value) {
        Before();
        const auto* p = reinterpret_cast<const uint8_t*>(&value);
        bytes.insert(bytes.end(), p, p + sizeof(value));
        return status;
    }
    template<class T> bool Get(T& value) {
        Before();
        if (cursor + sizeof(value) > bytes.size()) throw std::runtime_error("truncated fixture");
        if (!(leaveRoot && cursor == 0)) std::memcpy(&value, bytes.data() + cursor, sizeof(value));
        cursor += sizeof(value);
        return status;
    }
    bool Skip(uint32_t n) override { cursor += n; return status; }
    bool GetSint8(int8_t& v) override { return Get(v); }
    bool GetUint8(uint8_t& v) override { return Get(v); }
    bool GetSint16(int16_t& v) override { return Get(v); }
    bool GetUint16(uint16_t& v) override { return Get(v); }
    bool GetSint32(int32_t& v) override { return Get(v); }
    bool GetUint32(uint32_t& v) override { return Get(v); }
    bool GetSint64(int64_t& v) override { return Get(v); }
    bool GetUint64(uint64_t& v) override { return Get(v); }
    bool GetFloat32(float& v) override { return Get(v); }
    bool GetFloat64(double& v) override { return Get(v); }
    bool GetRZCharStr(char*, uint32_t) override { throw std::runtime_error("wrong string entry"); }
    bool GetGZStr(cIGZString& value) override {
        ++strings;
        uint32_t length = 0;
        Get(length);
        if (cursor + length > bytes.size()) throw std::runtime_error("truncated string");
        value.FromChar(reinterpret_cast<const char*>(bytes.data() + cursor), length);
        cursor += length;
        return status;
    }
    bool GetGZSerializable(cIGZSerializable&) override { throw std::runtime_error("unexpected serializable"); }
    bool GetVoid(void*, uint32_t) override { throw std::runtime_error("unexpected raw read"); }
    int32_t GetError() override { ++errors; return 0; }
    int32_t SetUserData(cIGZVariant*) override { return 0; }
    int32_t GetUserData() override { return 0; }
    void Flush() override { throw std::runtime_error("unexpected flush"); }
    bool SetSint8(int8_t v) override { return Put(v); }
    bool SetUint8(uint8_t v) override { return Put(v); }
    bool SetSint16(int16_t v) override { return Put(v); }
    bool SetUint16(uint16_t v) override { return Put(v); }
    bool SetSint32(int32_t v) override { return Put(v); }
    bool SetUint32(uint32_t v) override { return Put(v); }
    bool SetSint64(int64_t v) override { return Put(v); }
    bool SetUint64(uint64_t v) override { return Put(v); }
    bool SetFloat32(float v) override { return Put(v); }
    bool SetFloat64(double v) override { return Put(v); }
    bool SetRZCharStr(const char* value) override {
        ++strings;
        const auto length = static_cast<uint32_t>(std::strlen(value));
        Put(length);
        bytes.insert(bytes.end(), value, value + length);
        return status;
    }
    bool SetGZStr(const cIGZString&) override { throw std::runtime_error("wrong output string entry"); }
    bool SetGZSerializable(const cIGZSerializable&) override { throw std::runtime_error("unexpected serializable"); }
    bool SetVoid(const void*, uint32_t) override { throw std::runtime_error("unexpected raw write"); }
};

unsigned decalReads = 0, decalWrites = 0;
void* decodedDecal = nullptr;
void ConstructDescription(uint32_t family, void* p) {
    if (family != 1) throw std::runtime_error("unexpected constructed family");
    std::memset(p, 0, 0x64); // valid owned empty containers for the injected dependency
}
void ReadDescription(uint32_t family, uint16_t version, cIGZIStream* stream, void* p) {
    if (family != 1 || version != 0xFFFF || static_cast<Stream*>(stream)->cursor != 0xE)
        throw std::runtime_error("wrong typed reader arguments or resource cursor");
    ++decalReads;
    decodedDecal = p;
    stream->GetUint32(Field<uint32_t>(p, 0));
    Field<uint32_t>(p, 0) &= 0x7F;
    stream->GetUint32(Field<uint32_t>(p, 4));
    stream->GetUint8(Field<uint8_t>(p, 8)); stream->GetUint8(Field<uint8_t>(p, 9));
    stream->GetFloat32(Field<float>(p, 0xC));
    for (size_t offset : {0x10, 0x1C, 0x28, 0x34, 0x40}) {
        uint32_t count = 0;
        stream->GetUint32(count);
        const uint32_t bytes = count * (offset == 0x34 ? 12 : 4);
        auto& v = Field<Vector>(p, offset);
        FreeVector(v);
        auto* buffer = static_cast<uint8_t*>(Allocate(bytes));
        v = {buffer, buffer + bytes, buffer + bytes};
        for (auto* q = v.begin; q != v.end; q += 4) stream->GetFloat32(*reinterpret_cast<float*>(q));
    }
    for (size_t offset = 0x4C; offset <= 0x60; offset += 4) stream->GetFloat32(Field<float>(p, offset));
    if (Field<uint8_t>(p, 9) == 0) { Field<uint8_t>(p, 9) = 2; Field<uint32_t>(p, 0) |= 0x40; }
}
void WriteDescription(uint32_t family, cIGZOStream* stream, const void* value) {
    if (family != 1 || static_cast<Stream*>(stream)->bytes.size() != 0xE)
        throw std::runtime_error("wrong typed writer arguments or resource cursor");
    ++decalWrites;
    auto* p = const_cast<void*>(value);
    stream->SetUint32(Field<uint32_t>(p, 0)); stream->SetUint32(Field<uint32_t>(p, 4));
    stream->SetUint8(Field<uint8_t>(p, 8)); stream->SetUint8(Field<uint8_t>(p, 9));
    stream->SetFloat32(Field<float>(p, 0xC));
    for (size_t offset : {0x10, 0x1C, 0x28, 0x34, 0x40}) {
        const auto& v = Field<Vector>(p, offset);
        stream->SetUint32(static_cast<uint32_t>((v.end - v.begin) / (offset == 0x34 ? 12 : 4)));
        for (auto* q = v.begin; q != v.end; q += 4) stream->SetFloat32(*reinterpret_cast<float*>(q));
    }
    for (size_t offset = 0x4C; offset <= 0x60; offset += 4) stream->SetFloat32(Field<float>(p, offset));
}

int DecalWorkedCase() {
    auto dependencies = bindings;
    dependencies.constructDescription = ConstructDescription;
    dependencies.readDescription = ReadDescription;
    dependencies.writeDescription = WriteDescription;
    dependencies.publishCollection = CopyDecals;
    auto* resource = static_cast<FC4IEffectsResource*>(CreateFC4EffectsResourceWithBindings(dependencies));
    CHECK(resource && resource->AddRef() == 1);
    Stream empty;
    CHECK(resource->Write(&empty));
    Stream payload;
    payload.Put<uint32_t>(0xFFFFFFA5); payload.Put<uint32_t>(0x12345678);
    payload.Put<uint8_t>(7); payload.Put<uint8_t>(0); payload.Put<float>(6);
    for (float f : {2.f, 3.f, 4.f}) { payload.Put<uint32_t>(1); payload.Put<float>(f); }
    payload.Put<uint32_t>(1);
    for (float f : {0.25f, 0.5f, 1.f}) payload.Put<float>(f);
    payload.Put<uint32_t>(1); payload.Put<float>(0.75f);
    for (float f : {1.25f, 2.5f, -3.f, 1.5f, -2.f, 8.f}) payload.Put<float>(f);
    CHECK(payload.bytes.size() == 86);
    Stream input;
    input.bytes = empty.bytes;
    input.bytes[8] = input.bytes[9] = 0xFF; // decal version is consumed, not interpreted
    input.bytes[10] = 1;
    input.bytes.insert(input.bytes.begin() + 0xE, payload.bytes.begin(), payload.bytes.end());
    input.status = false;
    CHECK(resource->Read(&input) && input.cursor == input.bytes.size());
    auto* c = reinterpret_cast<uint8_t*>(resource) + 4;
    auto& values = Field<Vector>(c, 0x78);
    CHECK(values.end - values.begin == 0x64 && values.begin == decodedDecal && decalReads == 1);
    CHECK(Field<uint32_t>(decodedDecal, 0) == 0x65 && Field<uint32_t>(decodedDecal, 4) == 0x12345678);
    CHECK(Field<uint8_t>(decodedDecal, 8) == 7 && Field<uint8_t>(decodedDecal, 9) == 2);
    CHECK(Field<float>(decodedDecal, 0xC) == 6);
    CHECK(Field<Vector>(decodedDecal, 0x34).end - Field<Vector>(decodedDecal, 0x34).begin == 12);
    alignas(4) uint8_t destination[0x118]{};
    resource->Publish(reinterpret_cast<cIGZUnknown*>(destination));
    CHECK(Field<Vector>(destination, 0x78).begin != values.begin);
    Stream interrupted;
    interrupted.bytes = empty.bytes;
    interrupted.throwAt = 6; // decal shrink completed; shake version not supplied
    bool threw = false;
    try { resource->Read(&interrupted); } catch (const std::runtime_error&) { threw = true; }
    CHECK(threw && values.begin == values.end); // no rollback of the completed shrink
    CHECK(resource->Release() == 0); // the copied payload must outlive its source
    auto* writer = static_cast<FC4IEffectsResource*>(CreateFC4EffectsResourceWithBindings(dependencies));
    CHECK(writer && writer->AddRef() == 1);
    Field<Vector>(reinterpret_cast<uint8_t*>(writer) + 4, 0x78) = Field<Vector>(destination, 0x78);
    Field<Vector>(destination, 0x78) = {};
    Stream output;
    CHECK(writer->Write(&output) && decalWrites == 1);
    input.bytes[8] = 1; input.bytes[9] = 0; // normalized section version
    const uint32_t flags = 0x65;
    std::memcpy(input.bytes.data() + 0xE, &flags, 4);
    input.bytes[0xE + 9] = 2;
    CHECK(output.bytes == input.bytes);
    CHECK(writer->Release() == 0);
    return 0;
}

int AuxiliaryRecords() {
    auto* resource = static_cast<FC4IEffectsResource*>(CreateFC4EffectsResourceWithBindings(bindings));
    CHECK(resource && resource->AddRef() == 1);
    Stream input;
    input.Put<uint16_t>(4);
    for (uint16_t version : {2, 1}) { input.Put<uint16_t>(version); input.Put<uint32_t>(0); }
    input.Put<uint16_t>(0); input.Put<uint32_t>(1); // shake
    input.Put<float>(5); input.Put<float>(1);
    input.Put<uint32_t>(1); input.Put<float>(2);
    input.Put<uint32_t>(1); input.Put<float>(60);
    input.Put<float>(0.5f); input.Put<uint8_t>(0xA5);
    input.Put<uint16_t>(0); input.Put<uint32_t>(1); // light
    input.Put<uint32_t>(1);
    for (float f : {0.25f, 0.5f, 1.f}) input.Put<float>(f);
    input.Put<uint32_t>(1); input.Put<float>(0.75f); input.Put<float>(2);
    for (unsigned i = 0; i < 6; ++i) input.Put<uint32_t>(0);
    input.Put<uint16_t>(1); input.Put<uint32_t>(0); // dynamic
    input.Put<uint16_t>(2); input.Put<uint32_t>(0); // visual
    input.SetRZCharStr("end"); input.Put<uint32_t>(0xFFFFFFFF); input.Put<uint8_t>(0);
    input.Put<uint32_t>(2);
    for (unsigned i = 0; i < 2; ++i) { input.SetRZCharStr("same"); input.Put<uint32_t>(9); input.Put<uint32_t>(7); }
    input.Put<uint16_t>(0); input.Put<uint32_t>(2);
    for (unsigned i = 0; i < 2; ++i) { input.Put<uint32_t>(0x12345678); input.SetRZCharStr("trigger"); }
    input.status = false;
    CHECK(resource->Read(&input) && input.cursor == input.bytes.size());
    auto* c = reinterpret_cast<uint8_t*>(resource) + 4;
    CHECK(Field<Vector>(c, 0xF0).end - Field<Vector>(c, 0xF0).begin == 2 * 0x14);
    CHECK(Field<Vector>(c, 0xFC).end - Field<Vector>(c, 0xFC).begin == 2 * 0x10);
    CHECK(Field<uint32_t>(Field<Vector>(c, 0x98).begin, 0x24) == 0xA5);
    Stream output;
    CHECK(resource->Write(&output) && output.bytes == input.bytes);
    CHECK(resource->Read(&output) && output.cursor == output.bytes.size()); // replaces owned strings/lists
    CHECK(resource->Release() == 0);
    return 0;
}

int Identity() {
    auto* raw = CreateFC4EffectsResourceWithBindings(bindings);
    CHECK(raw);
    auto* resource = static_cast<FC4IEffectsResource*>(raw);
    auto* r = reinterpret_cast<unsigned char*>(resource);
    CHECK(raw->AddRef() == 1);
    CHECK(resource->ClassID() == 0x2A5118AD);
    CHECK(std::strcmp(resource->GetName(), "") == 0);
    void* c = nullptr;
    void* p = nullptr;
    CHECK(raw->QueryInterface(0xC97CF5CE, &c));
    CHECK(c == r + 4);
    CHECK(Slot<bool>(c, 0, uint32_t{1}, &p));
    CHECK(p == r + 0x11C);
    CHECK(Slot<uint32_t>(p, 11) == 3);
    void* r2 = nullptr;
    CHECK(Slot<bool>(p, 0, uint32_t{0xCA51189B}, &r2));
    CHECK(r2 == raw);
    CHECK(static_cast<cIGZUnknown*>(r2)->Release() == 3);
    void* p2 = nullptr;
    CHECK(Slot<bool>(p, 0, uint32_t{0x456B8F1D}, &p2));
    CHECK(p2 == p);
    CHECK(Slot<uint32_t>(p2, 2) == 3);
    for (uint32_t iid : {0xE4FDA3D4u, 0x2A5118ADu, 0xEA5118B0u, 0xFFFFFFFFu}) {
        void* sentinel = reinterpret_cast<void*>(0x12345678);
        CHECK(!raw->QueryInterface(iid, &sentinel));
        CHECK(sentinel == reinterpret_cast<void*>(0x12345678));
        CHECK(Slot<uint32_t>(p, 11) == 3);
    }
    uint32_t key[3] = {9, 9, 9};
    CHECK(Slot<void*>(p, 3, static_cast<void*>(key)) == key);
    CHECK(key[0] == 0 && key[1] == 0 && key[2] == 0);
    const uint32_t assigned[3] = {0xEA5118B0, 0xEA5118B1, 1};
    CHECK(Slot<bool>(p, 4, static_cast<const void*>(assigned)));
    for (unsigned i = 0; i < 3; ++i) CHECK(Slot<uint32_t>(p, 5 + i) == assigned[i]);
    for (unsigned i = 0; i < 3; ++i) {
        CHECK(Slot<bool>(p, 8 + i, uint32_t{70 + i}));
        CHECK(Slot<uint32_t>(p, 5 + i) == 70 + i);
    }
    char input[] = "smoke\0ignored";
    CHECK(resource->SetName(input) == r + 0x134);
    std::memcpy(input, "other", 5);
    CHECK(std::strcmp(resource->GetName(), "smoke") == 0);
    CHECK(Slot<uint32_t>(p, 5) == 70);
    CHECK(resource->ClassID() == 0x2A5118AD);
    unsigned char destination[0x118]{};
    resource->Publish(reinterpret_cast<cIGZUnknown*>(destination));
    CHECK(published == 1 && publicationSource == c && publicationDestination == destination);
    CHECK(Slot<uint32_t>(p, 11) == 3);
    std::vector<std::thread> readers;
    std::atomic<bool> good{true};
    for (unsigned i = 0; i < 8; ++i) readers.emplace_back([&] {
        for (unsigned n = 0; n < 1000; ++n) {
            void* held = nullptr;
            if (!raw->QueryInterface(1, &held) || held != p) good = false;
            else static_cast<cIGZUnknown*>(held)->Release();
        }
    });
    for (auto& reader : readers) reader.join();
    CHECK(good && Slot<uint32_t>(p, 11) == 3);
    const unsigned oldDestroyed = destroyed;
    CHECK(Slot<uint32_t>(c, 2) == 2);
    CHECK(Slot<uint32_t>(p, 2) == 1);
    CHECK(destroyed == oldDestroyed);
    CHECK(raw->Release() == 0);
    CHECK(destroyed == oldDestroyed + 1);
    return 0;
}

int Streams() {
    auto* resource = static_cast<FC4IEffectsResource*>(CreateFC4EffectsResourceWithBindings(bindings));
    CHECK(resource && resource->AddRef() == 1);
    Stream output;
    CHECK(resource->Write(&output));
    const auto golden = output.bytes;
    CHECK(golden.size() == 84 && golden[0] == 4 && golden[2] == 2);
    CHECK(golden[8] == 1 && golden[0x32] == 1 && golden[0x38] == 2);
    CHECK(golden[0x3E] == 3 && golden[0x42] == 'e' && golden[0x43] == 'n' && golden[0x44] == 'd');
    CHECK(std::all_of(golden.begin() + 0x45, golden.begin() + 0x49, [](uint8_t b) { return b == 0xFF; }));
    CHECK(output.strings == 1 && output.retains == 0 && output.releases == 0 && output.queries == 0 && output.errors == 0);
    Stream falseOutput;
    falseOutput.status = false;
    CHECK(resource->Write(&falseOutput) && falseOutput.bytes == golden);
    Stream interrupted;
    interrupted.throwAt = 3;
    bool threw = false;
    try { resource->Write(&interrupted); } catch (const std::runtime_error&) { threw = true; }
    CHECK(threw && interrupted.bytes == std::vector<uint8_t>({4, 0, 2, 0}));
    auto* r = reinterpret_cast<unsigned char*>(resource);
    for (uint16_t root : {0, 1, 2, 5, 0x400, 0xFFFF}) {
        Stream input;
        input.bytes = golden;
        std::memcpy(input.bytes.data(), &root, 2);
        std::array<unsigned char, 0x118> before;
        std::memcpy(before.data(), r + 4, before.size());
        CHECK(!resource->Read(&input) && input.cursor == 2);
        CHECK(std::memcmp(before.data(), r + 4, before.size()) == 0);
    }
    for (uint16_t root : {3, 4}) {
        Stream input;
        input.bytes = golden;
        if (root == 3) input.bytes.erase(input.bytes.begin() + 0x32, input.bytes.begin() + 0x38);
        std::memcpy(input.bytes.data(), &root, 2);
        input.status = false; // status itself is ignored when destinations are supplied
        CHECK(resource->Read(&input) && input.cursor == input.bytes.size());
        CHECK(input.strings == 1 && input.retains == 0 && input.releases == 0 && input.queries == 0 && input.errors == 0);
        Stream rewritten;
        CHECK(resource->Write(&rewritten) && rewritten.bytes == golden);
    }
    Stream noHeader;
    noHeader.bytes = golden;
    noHeader.status = false;
    noHeader.leaveRoot = true;
    CHECK(!resource->Read(&noHeader) && noHeader.cursor == 2);
    // EF26-C6: counted camera curve, not a raw seven-word memory image.
    r[4 + 0xD0] = 7;
    auto& curve = Field<Vector>(r + 4, 0xD4);
    curve.begin = static_cast<uint8_t*>(Allocate(4));
    curve.end = curve.capacity = curve.begin + 4;
    *reinterpret_cast<float*>(curve.begin) = 7;
    const float camera[4] = {1, 2, 3, 4};
    std::memcpy(r + 4 + 0xE0, camera, sizeof(camera));
    Stream cameraOutput;
    CHECK(resource->Write(&cameraOutput) && cameraOutput.bytes.size() == 110);
    CHECK(cameraOutput.bytes[0x49] == 7);
    cameraOutput.bytes[0x4A] = cameraOutput.bytes[0x4B] = 0xFF;
    std::memset(r + 4 + 0xE0, 0, sizeof(camera));
    CHECK(resource->Read(&cameraOutput));
    CHECK(r[4 + 0xD0] == 7 && std::memcmp(r + 4 + 0xE0, camera, sizeof(camera)) == 0);
    CHECK(curve.end - curve.begin == 4 && *reinterpret_cast<float*>(curve.begin) == 7);
    Stream normalized;
    CHECK(resource->Write(&normalized) && normalized.bytes[0x4A] == 0 && normalized.bytes[0x4B] == 0);
    Stream absent;
    absent.bytes = golden;
    CHECK(resource->Read(&absent));
    CHECK(r[4 + 0xD0] == 0 && std::memcmp(r + 4 + 0xE0, camera, sizeof(camera)) == 0);
    CHECK(curve.end - curve.begin == 4 && *reinterpret_cast<float*>(curve.begin) == 7);
    CHECK(resource->Release() == 0);
    return 0;
}
}

int main() {
    CHECK(CreateFC4EffectsResource() == nullptr);
    CHECK(Identity() == 0);
    CHECK(Streams() == 0);
    CHECK(DecalWorkedCase() == 0);
    CHECK(AuxiliaryRecords() == 0);
    CHECK(constructed == destroyed && allocated == freed);
    std::puts("effects resource: EF25 identities/streams; EF26-C1 decal typed IO/publication/source destruction and EF26-C6 camera PASS");
    return 0;
}
