// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4EffectsResource.h"
#include "FC4Log.h"
#include "compat/FC4Compat.h"
#include "cIGZIStream.h"
#include "cIGZOStream.h"
#include "cRZBaseString.h"

#include <array>
#include <atomic>
#include <cstring>
#include <limits>
#include <new>

namespace {
class Resource;
struct Collection {
    void** vtable;
    unsigned char fields[0x114];
    Resource* Owner();
    static bool __fastcall Query(Collection*, void*, uint32_t, void**);
    static uint32_t __fastcall Retain(Collection*, void*);
    static uint32_t __fastcall Drop(Collection*, void*);
    static void* __fastcall Destroy(Collection*, void*, uint32_t);
};
struct Key { uint32_t type = 0, group = 0, instance = 0; };
class Persist : public cIGZUnknown {
public:
    bool QueryInterface(uint32_t, void**) override;
    uint32_t AddRef() override;
    uint32_t Release() override;
    virtual Key* GetKey(Key* out) { *out = key; return out; }
    virtual bool SetKey(const Key* in) { key = *in; return true; }
    virtual uint32_t GetType() { return key.type; }
    virtual uint32_t GetGroup() { return key.group; }
    virtual uint32_t GetInstance() { return key.instance; }
    virtual bool SetType(uint32_t value) { key.type = value; return true; }
    virtual bool SetGroup(uint32_t value) { key.group = value; return true; }
    virtual bool SetInstance(uint32_t value) { key.instance = value; return true; }
    virtual uint32_t GetRefCount() { return refs.load(); }
    virtual void* Destroy(uint32_t flags);

    std::atomic<uint32_t> refs{0};
    uint32_t reserved = 0;
    Key key;
    Resource* Owner();
};
struct Name {
    const void* vtable;
    char* begin = nullptr;
    char* end = nullptr;
    char* capacity = nullptr;
    uint32_t refs = 0;
};
static_assert(sizeof(Collection) == 0x118, "resource collection extent");
static_assert(sizeof(Persist) == 0x18, "native persistence extent");
static_assert(sizeof(Name) == 0x14, "native name string object");

class Resource final : public FC4IEffectsResource {
public:
    explicit Resource(const FC4EffectsResourceBindings& bindings) : bindings_(bindings) {
        bindings_.constructCollection(&collection_);
        nativeCollectionVtable_ = collection_.vtable;
        std::memcpy(collectionVtable_.data(), collection_.vtable, sizeof(collectionVtable_));
        collectionVtable_[0] = reinterpret_cast<void*>(&Collection::Query);
        collectionVtable_[1] = reinterpret_cast<void*>(&Collection::Retain);
        collectionVtable_[2] = reinterpret_cast<void*>(&Collection::Drop);
        collectionVtable_[23] = reinterpret_cast<void*>(&Collection::Destroy);
        collection_.vtable = collectionVtable_.data();
        name_.vtable = bindings_.stringVtable;
    }
    ~Resource() {
        collection_.vtable = nativeCollectionVtable_;
        bindings_.destroyCollection(&collection_);
        if (name_.begin) bindings_.freeString(name_.begin);
    }
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out) return false;
        void* result = nullptr;
        switch (iid) {
        case kFC4_IID_cISC4EffectsResource: result = static_cast<FC4IEffectsResource*>(this); break;
        case 0xC97CF5CE: result = &collection_; break;
        case 0x456B8F1D: case 1: result = &persist_; break;
        default: return false; // Unsupported QI leaves the output cell untouched.
        }
        *out = result;
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return persist_.refs.fetch_add(1) + 1; }
    uint32_t Release() override {
        const uint32_t count = persist_.refs.fetch_sub(1) - 1;
        if (!count) persist_.Destroy(1);
        return count;
    }
    void* SetName(const char* value) override {
        const size_t size = std::strlen(value);
        if (size >= std::numeric_limits<uint32_t>::max()) return &name_;
        auto* copy = static_cast<char*>(bindings_.allocateString(static_cast<uint32_t>(size + 1)));
        // SPEC-GAP: name-allocation failure preserves the previous owned name.
        if (!copy) return &name_;
        std::memcpy(copy, value, size + 1);
        if (name_.begin) bindings_.freeString(name_.begin);
        name_.begin = copy;
        name_.end = copy + size;
        name_.capacity = copy + size + 1;
        return &name_;
    }
    const char* GetName() override { return name_.begin ? name_.begin : ""; }
    void Publish(cIGZUnknown* destination) override {
        bindings_.publishCollection(&collection_, destination);
    }
    bool Write(cIGZOStream* stream) override;
    bool Read(cIGZIStream* stream) override;
    uint32_t ClassID() override { return kFC4_CLSID_cSC4EffectsResource; }

    bool Empty() const {
        for (size_t offset : {0x4, 0x24, 0x44, 0x64, 0x84, 0xA4}) {
            uint32_t count;
            std::memcpy(&count, Bytes() + offset + 0x10, 4);
            if (count) return false;
        }
        // Logical vector ends, never spare capacity or the dictionary count.
        for (size_t offset : {0x18, 0x38, 0x58, 0x78, 0x98, 0xB8, 0xF0, 0xFC}) {
            const auto* p = reinterpret_cast<const uintptr_t*>(Bytes() + offset);
            if (p[0] != p[1]) return false;
        }
        const auto* directories = reinterpret_cast<const uintptr_t*>(Bytes() + 0xC4);
        for (auto* p = reinterpret_cast<const unsigned char*>(directories[0]);
             p != reinterpret_cast<const unsigned char*>(directories[1]); p += 0x20) {
            const auto* vector = reinterpret_cast<const uintptr_t*>(p + 0x14);
            if (vector[0] != vector[1]) return false;
            uint32_t count;
            std::memcpy(&count, p + 0x10, 4);
            if (count) return false;
        }
        return true;
    }
    unsigned char* Bytes() { return reinterpret_cast<unsigned char*>(&collection_); }
    const unsigned char* Bytes() const { return reinterpret_cast<const unsigned char*>(&collection_); }

    Collection collection_{}; // R+4; includes C's scratch string through C+117.
    Persist persist_;        // R+11C: the original factory subtracts this offset.
    Name name_{};            // R+134: SetName returns this object, not its characters.
    uint32_t reserved_ = 0;
    FC4EffectsResourceBindings bindings_;
    void** nativeCollectionVtable_ = nullptr;
    std::array<void*, 24> collectionVtable_{};
};

Resource* Collection::Owner() { return reinterpret_cast<Resource*>(reinterpret_cast<char*>(this) - 4); }
bool __fastcall Collection::Query(Collection* self, void*, uint32_t iid, void** out) { return self->Owner()->QueryInterface(iid, out); }
uint32_t __fastcall Collection::Retain(Collection* self, void*) { return self->Owner()->AddRef(); }
uint32_t __fastcall Collection::Drop(Collection* self, void*) { return self->Owner()->Release(); }
// [abi-ret] effects.md 18.9.3: P slot 12 returns the old R pointer in EAX.
void* __fastcall Collection::Destroy(Collection* self, void*, uint32_t flags) { return self->Owner()->persist_.Destroy(flags); }
Resource* Persist::Owner() { return reinterpret_cast<Resource*>(reinterpret_cast<char*>(this) - 0x11C); }
bool Persist::QueryInterface(uint32_t iid, void** out) { return Owner()->QueryInterface(iid, out); }
uint32_t Persist::AddRef() { return Owner()->AddRef(); }
uint32_t Persist::Release() { return Owner()->Release(); }
void* Persist::Destroy(uint32_t flags) {
    Resource* resource = Owner();
    if (flags & 1) delete resource;
    else resource->~Resource();
    return resource;
}

bool Resource::Write(cIGZOStream* stream) {
    // SPEC-GAP: EF26: non-interface renderer value codecs lack complete typed
    // call declarations; do not guess them or invoke native resource Write.
    if (!Empty()) {
        fc4::Log("effectsres: nonempty Write unsupported (EF26); replacement incomplete");
        return false;
    }
    stream->SetUint16(4);
    for (uint16_t version : {2, 1, 0, 0}) {
        stream->SetUint16(version);
        stream->SetUint32(0);
    }
    for (unsigned type = 3; type <= 8; ++type) stream->SetUint32(0);
    stream->SetUint16(1); // dynamic particles, even after a root-3 Read
    stream->SetUint32(0);
    stream->SetUint16(2); // visual descriptions
    stream->SetUint32(0);
    stream->SetRZCharStr("end");
    stream->SetUint32(0xFFFFFFFF);
    const uint8_t present = Bytes()[0xD0];
    stream->SetUint8(present);
    if (present) {
        stream->SetUint16(0);
        for (unsigned offset = 0xD4; offset <= 0xEC; offset += 4) {
            float value;
            std::memcpy(&value, Bytes() + offset, 4);
            stream->SetFloat32(value);
        }
    }
    stream->SetUint32(0); // bindings
    stream->SetUint16(0);
    stream->SetUint32(0); // triggers
    return true; // Normal completion ignores all primitive Boolean results.
}

bool Resource::Read(cIGZIStream* stream) {
    uint16_t root = 0;
    stream->GetUint16(root);
    if (root != 3 && root != 4) return false;
    // Empty directories are a conformance path, not a substitute for the
    // shipped renderer payload. Until EF26 is settled, refuse that payload.
    if (!Empty()) return false;
    auto emptySection = [stream](bool versioned) {
        uint16_t version = 0;
        if (versioned) stream->GetUint16(version);
        uint32_t count = 0xFFFFFFFF;
        stream->GetUint32(count);
        if (count) fc4::Log("effectsres: nonempty/incomplete Read unsupported (EF26), count=%u; replacement incomplete", count);
        return count == 0;
    };
    for (unsigned family = 0; family < 4; ++family) if (!emptySection(true)) return false;
    for (unsigned type = 3; type <= 8; ++type) if (!emptySection(false)) return false;
    if (root == 4 && !emptySection(true)) return false;
    if (!emptySection(true)) return false;
    // Read the exact counted spelling (including the index); no standalone
    // FFFFFFFF string-length terminator is admitted.
    cRZBaseString spelling;
    uint32_t index = 0;
    stream->GetGZStr(spelling);
    stream->GetUint32(index);
    if (std::strcmp(spelling.ToChar(), "end") || index != 0xFFFFFFFF) return false;
    stream->GetUint8(Bytes()[0xD0]);
    if (Bytes()[0xD0]) {
        uint16_t version = 0;
        stream->GetUint16(version);
        for (unsigned offset = 0xD4; offset <= 0xEC; offset += 4) {
            auto& value = *reinterpret_cast<float*>(Bytes() + offset);
            stream->GetFloat32(value);
        }
    }
    if (!emptySection(false)) return false;
    return emptySection(true);
}
}

cIGZUnknown* CreateFC4EffectsResourceWithBindings(const FC4EffectsResourceBindings& bindings) {
    if (!bindings.constructCollection || !bindings.destroyCollection || !bindings.publishCollection ||
        !bindings.allocateString || !bindings.freeString) return nullptr;
    auto* result = new (std::nothrow) Resource(bindings);
    if (result) {
        // The physical identities are part of the original resource-factory ABI.
        if (reinterpret_cast<char*>(&result->collection_) - reinterpret_cast<char*>(result) != 4 ||
            reinterpret_cast<char*>(&result->persist_) - reinterpret_cast<char*>(result) != 0x11C ||
            reinterpret_cast<char*>(&result->name_) - reinterpret_cast<char*>(result) != 0x134) {
            delete result;
            return nullptr;
        }
    }
    return result;
}

cIGZUnknown* CreateFC4EffectsResource() {
    using namespace fc4::compat;
    const auto construct = DirectCall(Direct::EffectsCollectionConstruct);
    const auto publish = DirectCall(Direct::EffectsCollectionPublish);
    StlNodeAllocateFn allocate = nullptr;
    StlNodeFreeFn release = nullptr;
    if (!construct || !publish || !StlNodeAllocator(&allocate, &release)) return nullptr;
    FC4EffectsResourceBindings bindings{};
    bindings.constructCollection = [](void* receiver) {
        reinterpret_cast<void*(__thiscall*)(void*)>(DirectCall(Direct::EffectsCollectionConstruct))(receiver);
    };
    bindings.destroyCollection = [](void* receiver) {
        // The resource restores the base collection table from its constructor.
        // Invoke its deleting destructor with flags zero, never a manager's
        // adjusting destructor or a complete-resource destructor.
        auto** table = *static_cast<void***>(receiver);
        reinterpret_cast<void*(__thiscall*)(void*, uint32_t)>(table[23])(receiver, 0);
    };
    bindings.publishCollection = [](void* source, void* destination) {
        reinterpret_cast<void(__thiscall*)(void*, void*)>(DirectCall(Direct::EffectsCollectionPublish))(source, destination);
    };
    bindings.allocateString = allocate;
    bindings.freeString = release;
    bindings.stringVtable = GameStringVtable();
    if (!bindings.stringVtable) return nullptr;
    return CreateFC4EffectsResourceWithBindings(bindings);
}
