// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4EffectsResource.h"
#include "FC4Log.h"
#include "compat/FC4Compat.h"
#include "cIGZIStream.h"
#include "cIGZOStream.h"
#include "cRZBaseString.h"

#include <array>
#include <algorithm>
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
// effects.md 18.7.2; allocator B, not SDK containers or this DLL's CRT.
struct Vector {
    unsigned char* begin = nullptr;
    unsigned char* end = nullptr;
    unsigned char* capacity = nullptr;
    uint32_t Count(uint32_t stride) const {
        return begin == end ? 0 : static_cast<uint32_t>((end - begin) / stride);
    }
};
template<class T> T& At(void* p, size_t offset) {
    return *reinterpret_cast<T*>(static_cast<unsigned char*>(p) + offset);
}
const char* Text(const void* p) {
    const auto* s = static_cast<const Vector*>(p);
    return s->begin ? reinterpret_cast<const char*>(s->begin) : "";
}
bool PointerFamily(uint32_t family) { return family == 0 || family == 16 || (family >= 3 && family <= 8); }
uint32_t Stride(uint32_t family) {
    switch (family) {
    case 1: return 0x64;
    case 2: return 0x38;
    case 20: return 0x28; // shake
    case 21: return 0x1C; // light
    case 22: return 0x14; // ID binding
    case 23: return 0x10; // message trigger
    default: return 4;
    }
}
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

    Vector& Values(uint32_t family);
    void FreeVector(Vector& v) {
        if (v.begin) bindings_.freeString(v.begin);
        v = {};
    }
    template<class Initialize, class Destroy>
    void Resize(Vector& v, uint32_t count, uint32_t stride, Initialize initialize, Destroy destroy) {
        if (count > std::numeric_limits<uint32_t>::max() / stride) throw std::bad_alloc();
        while (v.Count(stride) > count) { v.end -= stride; destroy(v.end); }
        const uint32_t old = v.Count(stride);
        if (v.Count(1) < count * stride && (!v.begin || static_cast<size_t>(v.capacity - v.begin) < count * stride)) {
            auto* fresh = static_cast<unsigned char*>(bindings_.allocateString(count * stride));
            if (!fresh) throw std::bad_alloc();
            if (old) std::memcpy(fresh, v.begin, old * stride);
            if (v.begin) bindings_.freeString(v.begin);
            v = {fresh, fresh + old * stride, fresh + count * stride};
        }
        while (v.Count(stride) < count) {
            initialize(v.end);
            v.end += stride;
        }
    }
    void Initialize(uint32_t family, void* value);
    void DestroyValue(uint32_t family, void* value);
    void ResizeValues(uint32_t family, uint32_t count);
    void ReadString(cIGZIStream*, Vector&);
    void ReadList(cIGZIStream*, Vector&, uint32_t width);
    void WriteList(cIGZOStream*, const Vector&, uint32_t width);
    void ReadValue(uint32_t, uint16_t, cIGZIStream*, void*);
    void WriteValue(uint32_t, cIGZOStream*, const void*);
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

Vector& Resource::Values(uint32_t family) {
    if (family >= 3 && family <= 8) {
        auto& directories = At<Vector>(Bytes(), 0xC4);
        return At<Vector>(directories.begin + (family - 3) * 0x20, 0x14);
    }
    size_t offset = 0;
    switch (family) {
    case 0: offset = 0x38; break;
    case 1: offset = 0x78; break;
    case 2: offset = 0x18; break;
    case 16: offset = 0x58; break;
    case 20: offset = 0x98; break;
    case 21: offset = 0xB8; break;
    case 22: offset = 0xF0; break;
    case 23: offset = 0xFC; break;
    }
    return At<Vector>(Bytes(), offset);
}
void Resource::ReadString(cIGZIStream* stream, Vector& value) {
    cRZBaseString text;
    stream->GetGZStr(text);
    const uint32_t size = text.Strlen();
    if (size == std::numeric_limits<uint32_t>::max()) throw std::bad_alloc();
    auto* copy = static_cast<unsigned char*>(bindings_.allocateString(size + 1));
    if (!copy) throw std::bad_alloc();
    std::memcpy(copy, text.ToChar(), size);
    copy[size] = 0;
    FreeVector(value);
    value = {copy, copy + size, copy + size + 1};
}
void Resource::ReadList(cIGZIStream* stream, Vector& value, uint32_t width) {
    uint32_t count = 0;
    stream->GetUint32(count);
    if (width == 1 && count >= 1000000) return; // EF26 counted-f32 gate
    Resize(value, count, 4 * width, [](void* p) { (void)p; }, [](void* p) { (void)p; });
    for (auto* p = value.begin; p != value.end; p += 4) stream->GetFloat32(*reinterpret_cast<float*>(p));
}
void Resource::WriteList(cIGZOStream* stream, const Vector& value, uint32_t width) {
    stream->SetUint32(value.Count(4 * width));
    for (auto* p = value.begin; p != value.end; p += 4) stream->SetFloat32(*reinterpret_cast<const float*>(p));
}
void Resource::Initialize(uint32_t family, void* value) {
    std::memset(value, 0, Stride(family));
    if (family == 1) {
        bindings_.constructDescription(family, value);
    } else if (family == 2) {
        // SPEC-GAP: new old-format visual records have unspecified trailing defaults; use zero.
    } else if (family == 20 || family == 21) {
        // SPEC-GAP: fresh shake/light constructor defaults are unspecified; use zeroed valid containers.
    } else if (PointerFamily(family)) {
        uint32_t extent = 0;
        switch (family) {
        case 0: extent = 0x208; break;
        case 16: extent = 0x40; break;
        case 3: extent = 0x34; break;
        case 5: extent = 0x50; break;
        case 8: extent = 0x18; break;
        default: extent = 0x1C; break;
        }
        void* object = bindings_.allocateObject(extent);
        if (!object) throw std::bad_alloc();
        bindings_.constructDescription(family, object);
        // A newly installed reference belongs to this collection (18.9.6).
        ++At<uint32_t>(object, family == 0 ? 0x204 : family == 16 ? 4 : 8);
        *static_cast<void**>(value) = object;
    }
}
void Resource::DestroyValue(uint32_t family, void* value) {
    if (PointerFamily(family)) {
        if (void* object = *static_cast<void**>(value)) bindings_.releaseDescription(family, object);
    } else {
        if (family == 1) for (size_t offset : {0x10, 0x1C, 0x28, 0x34, 0x40}) FreeVector(At<Vector>(value, offset));
        if (family == 20) for (size_t offset : {8, 0x14}) FreeVector(At<Vector>(value, offset));
        if (family == 21) for (size_t offset : {0, 0xC}) FreeVector(At<Vector>(value, offset));
        if (family == 2) {
            for (size_t offset : {8, 0x14}) {
                auto& list = At<Vector>(value, offset);
                const uint32_t stride = offset == 8 ? 0x6C : 0x18;
                for (auto* p = list.begin; p != list.end; p += stride) FreeVector(At<Vector>(p, offset == 8 ? 0 : 4));
                FreeVector(list);
            }
            FreeVector(At<Vector>(value, 0x2C));
        }
        if (family == 22 || family == 23) FreeVector(At<Vector>(value, family == 22 ? 0 : 4));
    }
}
void Resource::ResizeValues(uint32_t family, uint32_t count) {
    Resize(Values(family), count, Stride(family),
        [&](void* p) {
            if (PointerFamily(family)) *static_cast<void**>(p) = nullptr;
            else Initialize(family, p);
        }, [&](void* p) { DestroyValue(family, p); });
}
void Resource::ReadValue(uint32_t family, uint16_t version, cIGZIStream* stream, void* value) {
    if (family == 20) {
        stream->GetFloat32(At<float>(value, 0));
        stream->GetFloat32(At<float>(value, 4));
        ReadList(stream, At<Vector>(value, 8), 1);
        ReadList(stream, At<Vector>(value, 0x14), 1);
        stream->GetFloat32(At<float>(value, 0x20));
        uint8_t table = 0;
        stream->GetUint8(table);
        At<uint32_t>(value, 0x24) = table;
    } else if (family == 21) {
        ReadList(stream, At<Vector>(value, 0), 3);
        ReadList(stream, At<Vector>(value, 0xC), 1);
        stream->GetFloat32(At<float>(value, 0x18));
    } else if (family == 22) {
        ReadString(stream, At<Vector>(value, 0));
        stream->GetUint32(At<uint32_t>(value, 0xC));
        stream->GetUint32(At<uint32_t>(value, 0x10));
    } else if (family == 23) {
        stream->GetUint32(At<uint32_t>(value, 0));
        ReadString(stream, At<Vector>(value, 4));
    } else {
        bindings_.readDescription(family, version, stream, PointerFamily(family) ? *static_cast<void**>(value) : value);
    }
}
void Resource::WriteValue(uint32_t family, cIGZOStream* stream, const void* value) {
    auto* p = const_cast<void*>(value);
    if (family == 20) {
        stream->SetFloat32(At<float>(p, 0));
        stream->SetFloat32(At<float>(p, 4));
        WriteList(stream, At<Vector>(p, 8), 1);
        WriteList(stream, At<Vector>(p, 0x14), 1);
        stream->SetFloat32(At<float>(p, 0x20));
        stream->SetUint8(static_cast<uint8_t>(At<uint32_t>(p, 0x24)));
    } else if (family == 21) {
        WriteList(stream, At<Vector>(p, 0), 3);
        WriteList(stream, At<Vector>(p, 0xC), 1);
        stream->SetFloat32(At<float>(p, 0x18));
    } else if (family == 22) {
        stream->SetRZCharStr(Text(p));
        stream->SetUint32(At<uint32_t>(p, 0xC));
        stream->SetUint32(At<uint32_t>(p, 0x10));
    } else if (family == 23) {
        stream->SetUint32(At<uint32_t>(p, 0));
        stream->SetRZCharStr(Text(static_cast<unsigned char*>(p) + 4));
    } else {
        bindings_.writeDescription(family, stream, PointerFamily(family) ? *static_cast<void* const*>(value) : value);
    }
}
bool Resource::Write(cIGZOStream* stream) {
    auto section = [&](uint32_t family, int version) {
        if (version >= 0) stream->SetUint16(static_cast<uint16_t>(version));
        const auto& values = Values(family);
        stream->SetUint32(values.Count(Stride(family)));
        for (auto* p = values.begin; p != values.end; p += Stride(family)) WriteValue(family, stream, p);
    };
    stream->SetUint16(4);
    section(0, 2); section(1, 1); section(20, 0); section(21, 0);
    for (uint32_t family = 3; family <= 8; ++family) section(family, -1);
    section(16, 1); section(2, 2);
    const auto& buckets = At<Vector>(Bytes(), 8);
    for (auto* b = buckets.begin; b != buckets.end; b += 4) {
        for (void* node = *reinterpret_cast<void**>(b); node; node = *static_cast<void**>(node)) {
            stream->SetRZCharStr(Text(static_cast<unsigned char*>(node) + 4));
            stream->SetUint32(At<uint32_t>(node, 0x10));
        }
    }
    stream->SetRZCharStr("end"); stream->SetUint32(0xFFFFFFFF);
    stream->SetUint8(Bytes()[0xD0]);
    if (Bytes()[0xD0]) {
        stream->SetUint16(0);
        WriteList(stream, At<Vector>(Bytes(), 0xD4), 1);
        for (unsigned offset = 0xE0; offset <= 0xEC; offset += 4) stream->SetFloat32(At<float>(Bytes(), offset));
    }
    section(22, -1); section(23, 0);
    return true; // Primitive and dependency Boolean statuses are not a result.
}
bool Resource::Read(cIGZIStream* stream) {
    uint16_t root = 0;
    stream->GetUint16(root);
    if (root != 3 && root != 4) return false;
    auto section = [&](uint32_t family, bool versioned) {
        uint16_t version = 0;
        // SPEC-GAP: an unwritten section count has no specified value; diagnose via an oversized sentinel.
        uint32_t count = 0xFFFFFFFF;
        if (versioned) stream->GetUint16(version);
        stream->GetUint32(count);
        ResizeValues(family, count);
        auto& values = Values(family);
        for (auto* p = values.begin; p != values.end; p += Stride(family)) {
            // Pointer owners are replaced before their typed Read (18.9.6).
            if (PointerFamily(family)) { DestroyValue(family, p); Initialize(family, p); }
            ReadValue(family, version, stream, p);
        }
    };
    section(0, true); section(1, true); section(20, true); section(21, true);
    for (uint32_t family = 3; family <= 8; ++family) section(family, false);
    if (root == 4) section(16, true);
    section(2, true);
    for (;;) {
        cRZBaseString spelling;
        uint32_t index = 0;
        stream->GetGZStr(spelling); stream->GetUint32(index);
        if (spelling.Strlen() == 3 && !std::memcmp(spelling.ToChar(), "end", 3) && index == 0xFFFFFFFF) break;
        if (!bindings_.insertVisualName) {
            fc4::Log("effectsres: visual-map insertion unavailable; replacement incomplete (EF27)");
            return false;
        }
        bindings_.insertVisualName(Bytes() + 4, spelling.ToChar(), index);
    }
    stream->GetUint8(Bytes()[0xD0]);
    if (Bytes()[0xD0]) {
        uint16_t version = 0;
        stream->GetUint16(version);
        ReadList(stream, At<Vector>(Bytes(), 0xD4), 1);
        for (unsigned offset = 0xE0; offset <= 0xEC; offset += 4) stream->GetFloat32(At<float>(Bytes(), offset));
    }
    section(22, false); section(23, true);
    return true;
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
    if (!DirectCall(Direct::GlobalHeapAllocate)) return nullptr;
    for (unsigned n = static_cast<unsigned>(Direct::EffectsParticleConstruct);
         n <= static_cast<unsigned>(Direct::EffectsDynamicDestroy); ++n) {
        if (!DirectCall(static_cast<Direct>(n))) return nullptr;
    }
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
    bindings.allocateObject = [](uint32_t size) {
        return reinterpret_cast<void*(__cdecl*)(uint32_t)>(DirectCall(Direct::GlobalHeapAllocate))(size);
    };
    bindings.constructDescription = [](uint32_t family, void* value) {
        Direct target = Direct::EffectsDecalConstruct;
        switch (family) {
        case 0: target = Direct::EffectsParticleConstruct; break;
        case 16: target = Direct::EffectsDynamicConstruct; break;
        case 3: target = Direct::EffectsBrushConstruct; break;
        case 4: target = Direct::EffectsAttractorConstruct; break;
        case 5: target = Direct::EffectsScrubberConstruct; break;
        case 6: target = Direct::EffectsSequenceConstruct; break;
        case 7: target = Direct::EffectsSoundConstruct; break;
        case 8: target = Direct::EffectsCameraConstruct; break;
        }
        reinterpret_cast<void*(__thiscall*)(void*)>(DirectCall(target))(value);
    };
    bindings.releaseDescription = [](uint32_t family, void* value) {
        if (family == 0) {
            // [abi-ret] effects.md 18.8.3: 005B1C80 is intrusive release, not assignment.
            reinterpret_cast<uintptr_t(__thiscall*)(void*)>(DirectCall(Direct::EffectsParticleDrop))(value);
        } else if (family == 16) {
            if (--At<uint32_t>(value, 4) == 0) {
                reinterpret_cast<void*(__thiscall*)(void*, uint32_t)>(DirectCall(Direct::EffectsDynamicDestroy))(value, 1);
            }
        } else {
            auto** table = *static_cast<void***>(value);
            // effects.md 18.7.1: component descriptions release through slot 5.
            reinterpret_cast<uint32_t(__thiscall*)(void*)>(table[5])(value);
        }
    };
    bindings.readDescription = [](uint32_t family, uint16_t version, cIGZIStream* stream, void* value) {
        // effects.md 18.10.1: borrowed typed values; no helper AL success test.
        if (family == 0 && version == 1) {
            reinterpret_cast<void(__thiscall*)(void*, cIGZIStream*)>(DirectCall(Direct::EffectsParticleReadOld))(value, stream);
        } else if (family == 2 && version == 1) {
            reinterpret_cast<cIGZIStream*(__thiscall*)(void*, cIGZIStream*)>(DirectCall(Direct::EffectsVisualReadOld))(value, stream);
        } else if (family >= 3 && family <= 8) {
            auto** table = *static_cast<void***>(value);
            reinterpret_cast<bool(__thiscall*)(void*, cIGZIStream*)>(table[2])(value, stream);
        } else {
            const Direct target = family == 0 ? Direct::EffectsParticleRead : family == 1 ? Direct::EffectsDecalRead :
                family == 16 ? Direct::EffectsDynamicRead : Direct::EffectsVisualRead;
            reinterpret_cast<void(__cdecl*)(cIGZIStream*, void*)>(DirectCall(target))(stream, value);
        }
    };
    bindings.writeDescription = [](uint32_t family, cIGZOStream* stream, const void* value) {
        if (family >= 3 && family <= 8) {
            auto* table = *static_cast<void* const* const*>(value);
            reinterpret_cast<bool(__thiscall*)(const void*, cIGZOStream*)>(table[3])(value, stream);
        } else if (family == 1) {
            reinterpret_cast<cIGZOStream*(__cdecl*)(cIGZOStream*, const void*)>(DirectCall(Direct::EffectsDecalWrite))(stream, value);
        } else {
            const Direct target = family == 0 ? Direct::EffectsParticleWrite :
                family == 16 ? Direct::EffectsDynamicWrite : Direct::EffectsVisualWrite;
            reinterpret_cast<void(__cdecl*)(cIGZOStream*, const void*)>(DirectCall(target))(stream, value);
        }
    };
    if (!bindings.stringVtable) return nullptr;
    return CreateFC4EffectsResourceWithBindings(bindings);
}
