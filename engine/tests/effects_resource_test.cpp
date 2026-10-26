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
void Construct(void* p) {
    ++constructed;
    std::memset(p, 0, 0x118);
    *static_cast<void***>(p) = table;
}
void Destroy(void*) { ++destroyed; }
void Publish(void* source, void* destination) {
    ++published;
    publicationSource = source;
    publicationDestination = destination;
}
void* Allocate(uint32_t size) { ++allocated; return std::malloc(size); }
void Free(void* p) { ++freed; std::free(p); }
FC4EffectsResourceBindings bindings{Construct, Destroy, Publish, Allocate, Free, nullptr};

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
    // A noncanonical presence byte and all seven camera words survive IO.
    r[4 + 0xD0] = 7;
    const uint32_t camera[7] = {0x80000000, 0x3F800000, 0x7FC01234, 4, 5, 6, 7};
    std::memcpy(r + 4 + 0xD4, camera, sizeof(camera));
    Stream cameraOutput;
    CHECK(resource->Write(&cameraOutput) && cameraOutput.bytes.size() == 114);
    CHECK(cameraOutput.bytes[0x49] == 7);
    std::memset(r + 4 + 0xD4, 0, sizeof(camera));
    CHECK(resource->Read(&cameraOutput));
    CHECK(r[4 + 0xD0] == 7 && std::memcmp(r + 4 + 0xD4, camera, sizeof(camera)) == 0);
    Stream absent;
    absent.bytes = golden;
    CHECK(resource->Read(&absent));
    CHECK(r[4 + 0xD0] == 0 && std::memcmp(r + 4 + 0xD4, camera, sizeof(camera)) == 0);
    CHECK(resource->Release() == 0);
    return 0;
}
}

int main() {
    CHECK(CreateFC4EffectsResource() == nullptr);
    CHECK(Identity() == 0);
    CHECK(Streams() == 0);
    CHECK(constructed == destroyed && allocated == freed);
    std::puts("effects resource: EF25 identity/lifetime/publication, W1/W3/W4/W5, R1/R2/R3/R4-empty/camera PASS; renderer payload pending EF26");
    return 0;
}
