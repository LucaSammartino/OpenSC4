// SPDX-License-Identifier: GPL-3.0-or-later
// Bounded native variant experiment; foundations sections 9, 10 and 17.1.
// Included only by FC4FormatsShadow.cpp to reuse its city-load lifecycle.
#pragma once

#include "FC4Interfaces.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "compat/FC4Compat.h"
#include "cISCProperty.h"
#include "cFC4Variant.h"

namespace fc4 {
namespace variant_shadow {

struct Totals {
    unsigned cases = 0, checks = 0, exact = 0, failures = 0, refusals = 0, clones = 0;
    unsigned scalarShapeFailures = 0;
    unsigned coreCountCalls = 0, coreSetCalls = 0, coreWriteCalls = 0;
    unsigned productionClones = 0, productionCloneRefusals = 0;
    uint32_t Count(const cFC4IGZVariant& owned) {
        ++coreCountCalls;
        return owned.GetCount();  // actual production virtual -> CAPI -> core
    }
    void Check(bool ok, const char* operation, uint16_t type) {
        ++checks;
        if (ok) ++exact;
        else {
            ++failures;
            if (std::strcmp(operation, "scalar shape") == 0) ++scalarShapeFailures;
            Log("variant shadow: FAIL %s type=0x%04X", operation, type);
        }
    }
};

class ProductionCopy {
public:
    ProductionCopy(const cFC4IGZVariant& source, Totals& t) {
        (void)source.CreateCopy(&p_);
        if (p_ && p_ != &source) {
            FC4_ACQUIRED(p_, "variant production CreateCopy owned result");
            ++t.productionClones;
        } else {
            p_ = nullptr;
            ++t.productionCloneRefusals;
            ++t.refusals;
            Log("variant shadow: REFUSED production CreateCopy output");
        }
    }
    ~ProductionCopy() { if (p_) FC4_RELEASE_CREATED(p_); }
    cFC4IGZVariant* Get() const { return p_; }
private:
    cFC4IGZVariant* p_ = nullptr;
};

// No unpublished layout: only the interface pointer and its published 127
// entries. Borrowed source identity is published in foundations 17.1. Owned
// SDK CreateCopy results may use a different native COM implementation table;
// validate the complete published interface in the supported image, without
// hardcoding an unpublished clone table or reading implementation data.
inline bool Native(const cFC4IGZVariant* p, bool requireSourceIdentity = false) {
    if (!p || !compat::IdentifiedVersion()) return false;
    const auto table = *reinterpret_cast<const uintptr_t* const*>(p);
    if (requireSourceIdentity && reinterpret_cast<uintptr_t>(table) != 0x00AA6788u) return false;
    MEMORY_BASIC_INFORMATION region{};
    if (!VirtualQuery(table, &region, sizeof(region)) || region.State != MEM_COMMIT ||
        region.AllocationBase != GetModuleHandleW(nullptr) ||
        (region.Protect & (PAGE_GUARD | PAGE_NOACCESS)) ||
        reinterpret_cast<uintptr_t>(table + 127) >
            reinterpret_cast<uintptr_t>(region.BaseAddress) + region.RegionSize) return false;
    for (unsigned i = 0; i < 127; ++i) {
        MEMORY_BASIC_INFORMATION code{};
        if (!VirtualQuery(reinterpret_cast<const void*>(table[i]), &code, sizeof(code)) ||
            code.State != MEM_COMMIT || code.AllocationBase != GetModuleHandleW(nullptr) ||
            !(code.Protect & (PAGE_EXECUTE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE |
                              PAGE_EXECUTE_WRITECOPY)) || (code.Protect & PAGE_GUARD)) return false;
    }
    return true;
}

class Owned {
public:
    explicit Owned(const cFC4IGZVariant& source, Totals& t) {
        // Literal SDK ownership reading: successful CreateCopy hands us one
        // owned reference. No borrowed property or property value is released.
        if (!Native(&source)) { ++t.refusals; return; }
        // [abi-ret] Return channel is eax-or-void: the owned output pointer, not
        // the nominal SDK bool, establishes success (HG6).
        (void)source.CreateCopy(&p_);
        if (p_) FC4_ACQUIRED(p_, "variant CreateCopy owned result");
        if (!p_ || p_ == &source || !Native(p_)) {
            ++t.refusals;
            Log("variant shadow: clone refused result=%s alias=%u native=%u",
                p_ ? "non-null" : "null", p_ == &source ? 1u : 0u, Native(p_) ? 1u : 0u);
            // Never release an aliased borrowed pointer on a contract violation.
            if (p_ == &source) p_ = nullptr;
            return;
        }
        ++t.clones;
        valid_ = true;
    }
    ~Owned() { if (p_) FC4_RELEASE_CREATED(p_); }
    Owned(const Owned&) = delete;
    Owned& operator=(const Owned&) = delete;
    cFC4IGZVariant* Get() const { return valid_ ? p_ : nullptr; }
private:
    cFC4IGZVariant* p_ = nullptr;
    bool valid_ = false;
};

inline void Sink(void* context, const uint8_t* data, size_t count) {
    static_cast<std::vector<uint8_t>*>(context)->assign(data, data + count);
}

inline uint32_t U32(const std::vector<uint8_t>& b, size_t at) {
    uint32_t value = 0;
    if (at + sizeof(value) <= b.size()) std::memcpy(&value, b.data() + at, sizeof(value));
    return value;
}

// The Rust side consumes literal input only, before ANY native getter. The
// serialized payload exposes Rust Variant storage without adding a C export.
// This does NOT verify the native segment serializer (HG4); no native result
// is serialized by Rust and called a native serialization oracle.
template<class T>
std::vector<uint8_t> Core(uint16_t type, bool array, const T* input, size_t count, Totals& t) {
    fc4_props* core = fc4_props_create();
    std::vector<uint8_t> bytes;
    bool ok = false;
    if (core) {
        ++t.coreSetCalls;
        if (fc4_props_set_typed(core, 1, type, array, input, count)) {
            ++t.coreWriteCalls;
            ok = fc4_props_write_variant(core, 1, Sink, &bytes);
        }
    }
    t.Check(ok, "Rust literal storage", type);
    fc4_props_destroy(core);
    return bytes;
}

template<class T>
bool Payload(const std::vector<uint8_t>& bytes, bool array, const T* actual, size_t count) {
    const size_t offset = array ? 12 : 8;
    return bytes.size() == offset + sizeof(T) * count &&
           (!array || U32(bytes, 8) == count) &&
           (!count || (actual && std::memcmp(actual, bytes.data() + offset, sizeof(T) * count) == 0));
}

template<class T>
void Cases(cFC4IGZVariant& native, uint16_t type, uint32_t diskTag, const T (&input)[3],
           void (cFC4IGZVariant::*set)(T), T (cFC4IGZVariant::*get)() const,
           bool (cFC4IGZVariant::*getTo)(T&) const,
           void (cFC4IGZVariant::*setArray)(T*, uint32_t), T* (cFC4IGZVariant::*array)() const,
           Totals& t) {
    // Two distinct scalar writes and 0/1/3 element arrays are bounded cases,
    // not evidence for cross-type conversion or all 127 methods (HG5).
    const unsigned beforeChecks = t.checks, beforeExact = t.exact, beforeFailures = t.failures;
    cFC4Variant production;
    cFC4IGZVariant& own = production;
    for (unsigned i = 0; i < 2; ++i) {
        ++t.cases;
        const auto expected = Core(type, false, input + i, 1, t);
        (native.*set)(input[i]);
        (own.*set)(input[i]);
        const auto ownCount = t.Count(own);
        if (i == 0) Log("variant shadow: scalar metadata type=0x%04X native_type=0x%04X native_count=%u production_core_count=%u",
                        type, native.GetType(), native.GetCount(), ownCount);
        t.Check(native.GetType() == type && native.GetType() == own.GetType() &&
                    native.GetCount() == ownCount && U32(expected, 4) == diskTag,
                "scalar shape", type);
        const T value = (native.*get)();
        const T ownValue = (own.*get)();
        T out = input[2];
        T ownOut = input[2];
        const bool ok = (native.*getTo)(out);
        const bool ownOk = (own.*getTo)(ownOut);
        t.Check(Payload(expected, false, &value, 1) && Payload(expected, false, &ownValue, 1), "scalar value getter", type);
        t.Check(ok && ownOk && Payload(expected, false, &out, 1) && Payload(expected, false, &ownOut, 1), "scalar out getter", type);
        Owned copy(native, t);
        ProductionCopy ownCopy(own, t);
        if (auto* p = copy.Get()) {
            // Change source after clone. The clone must retain the original
            // independently captured literal, with no Rust resynchronization.
            native.SetValUint32(0x13579BDF);
            own.SetValUint32(0x13579BDF);
            const T copied = (p->*get)();
            auto* q = ownCopy.Get();
            const T ownCopied = q ? (q->*get)() : input[2];
            t.Check(q && p->GetType() == q->GetType() && Payload(expected, false, &copied, 1) &&
                        Payload(expected, false, &ownCopied, 1), "scalar clone", type);
            (void)native.CopyFrom(*p);  // [abi-ret] HG6: check state, not undefined EAX.
            if (q) (void)own.CopyFrom(*q);
            const T restored = (native.*get)();
            const T ownRestored = (own.*get)();
            t.Check(q && native.GetType() == own.GetType() && Payload(expected, false, &restored, 1) &&
                        Payload(expected, false, &ownRestored, 1), "CopyFrom state", type);
        }
    }
    for (unsigned count : {0u, 1u, 3u}) {
        ++t.cases;
        const auto expected = Core(type, true, input, count, t);
        // A separate mutable copy is needed by the public copying-setter ABI.
        // It lives until Erase; the literal input and Rust state never change.
        T supplied[3] = {input[0], input[1], input[2]};
        T ownSupplied[3] = {input[0], input[1], input[2]};
        (native.*setArray)(supplied, count);
        (own.*setArray)(ownSupplied, count);
        supplied[0] = input[2];
        ownSupplied[0] = input[2];
        const auto ownCount = t.Count(own);
        t.Check(native.GetType() == (0x8000 | type) && native.GetType() == own.GetType() &&
                    native.GetCount() == ownCount && ownCount == count &&
                    U32(expected, 4) == (0x8000 | diskTag), "array shape", type);
        t.Check(Payload(expected, true, (native.*array)(), count) &&
                    Payload(expected, true, (own.*array)(), count), "array copying setter/data getter", type);
        Owned copy(native, t);
        ProductionCopy ownCopy(own, t);
        if (auto* p = copy.Get()) {
            if (count) (native.*array)()[0] = input[2];
            if (count) (own.*array)()[0] = input[2];
            auto* q = ownCopy.Get();
            const auto cloneCount = q ? t.Count(*q) : 0;
            t.Check(q && p->GetType() == q->GetType() && p->GetCount() == cloneCount &&
                        Payload(expected, true, (p->*array)(), count) &&
                        Payload(expected, true, (q->*array)(), count), "array clone isolation", type);
            native.SetValUint32(0x2468ACE0);
            own.SetValUint32(0x2468ACE0);
            (void)native.CopyFrom(*p);  // [abi-ret] HG6: check state, not undefined EAX.
            if (q) (void)own.CopyFrom(*q);
            const auto restoredCount = t.Count(own);
            t.Check(q && native.GetCount() == restoredCount && Payload(expected, true, (native.*array)(), count) &&
                        Payload(expected, true, (own.*array)(), count),
                    "array CopyFrom state", type);
        }
        (void)native.Erase();  // [abi-ret] HG6: check state, not undefined EAX.
        (void)own.Erase();
        const auto erasedCount = t.Count(own);
        t.Check(native.GetType() == own.GetType() && native.GetCount() == erasedCount &&
                    own.GetType() == 0, "Erase state", type);
    }
    Log("variant shadow: type=0x%04X cases=5 checks=%u exact=%u failures=%u",
        type, t.checks - beforeChecks, t.exact - beforeExact, t.failures - beforeFailures);
}

inline void Run() {
    if (!SettingInt("variant_shadow", "FC4_VARIANT_SHADOW", 0)) return;
    Totals t;
    if (gVariantGetCount != fc4_variant_get_count) {
        Log("variant shadow: REFUSED production Rust GetCount hook absent; fallback=1 resync=0");
        return;
    }
    Log("variant shadow: bounded literal native experiment; no 127-method owner or native serialization claim");
    if (!compat::IdentifiedVersion() || ExemplarFactoryActive()) {
        Log("variant shadow: refused unsupported image or replacement exemplar factory; cases=0 refusals=1");
        return;
    }
    props::Exemplar exemplar;
    // Existing public tuning resource; its properties supply ONLY a clone
    // source, never any expected value or mutable live game state.
    if (!exemplar.Load(0x6534284A, 0xE7E2C2DB, 0xA8F41EE3) || !exemplar.Holder()) {
        Log("variant shadow: refused missing SDK exemplar; cases=0 refusals=1");
        return;
    }
    const cFC4IGZVariant* borrowed = nullptr;
    for (uint32_t id : props::PropertyIds(exemplar.Holder())) {
        const cISCProperty* property = exemplar.Holder()->GetProperty(id);
        if (!property) continue;
        const auto* value = AsGame(property->GetPropertyValue());
        if (Native(value, true) && (value->GetType() & 0x7FFF) >= 1 && (value->GetType() & 0x7FFF) <= 14) {
            borrowed = value;
            break;
        }
    }
    if (!borrowed) {
        Log("variant shadow: refused no native numeric clone source; cases=0 refusals=1");
        return;
    }
    {
        Owned owned(*borrowed, t);
        if (auto* p = owned.Get()) {
#define FC4_VARIANT_CASE(T, Name, Type, Tag, A, B, C) \
            { const T input[3] = {A, B, C}; \
              Cases(*p, Type, Tag, input, &cFC4IGZVariant::SetVal##Name, \
                    &cFC4IGZVariant::GetVal##Name, &cFC4IGZVariant::GetVal##Name##To, \
                    &cFC4IGZVariant::SetRef##Name, &cFC4IGZVariant::Ref##Name, t); }
            FC4_VARIANT_CASE(bool, Bool, 1, 11, true, false, false)
            FC4_VARIANT_CASE(uint8_t, Uint8, 2, 1, 255, 17, 0)
            FC4_VARIANT_CASE(int8_t, Sint8, 3, 5, -128, 127, 0)
            FC4_VARIANT_CASE(uint16_t, Uint16, 4, 2, 65535, 0x1234, 0)
            FC4_VARIANT_CASE(int16_t, Sint16, 5, 6, -32768, 32767, 0)
            FC4_VARIANT_CASE(uint32_t, Uint32, 6, 3, 0xFFFFFFFFu, 0x12345678u, 0u)
            FC4_VARIANT_CASE(int32_t, Sint32, 7, 7, (-2147483647 - 1), 2147483647, 0)
            FC4_VARIANT_CASE(uint64_t, Uint64, 8, 4, UINT64_C(0xFFFFFFFFFFFFFFFF), UINT64_C(0x123456789ABCDEF0), 0)
            FC4_VARIANT_CASE(int64_t, Sint64, 9, 8, (-INT64_C(9223372036854775807) - 1), INT64_C(9223372036854775807), 0)
            FC4_VARIANT_CASE(float, Float32, 10, 9, -123.5f, 0.25f, 0.0f)
            FC4_VARIANT_CASE(double, Float64, 11, 10, -123.5, 0.25, 0.0)
            FC4_VARIANT_CASE(char, Char, 12, 17, 'A', 'z', '\0')
            FC4_VARIANT_CASE(uint16_t, RZUnicodeChar, 13, 16, 0x263A, 0x0041, 0)
            FC4_VARIANT_CASE(char, RZChar, 14, 12, '4', '2', '\0')
#undef FC4_VARIANT_CASE
        }
    }
    Log("variant shadow: total cases=%u checks=%u exact=%u failures=%u refusals=%u clones=%u "
        "scalar_shape_HG7=%u unexplained=%u; "
        "excluded=conversions,string-operations,objects,pointers,reference-alias,serialization (HG4/HG5)",
        t.cases, t.checks, t.exact, t.failures, t.refusals, t.clones,
        t.scalarShapeFailures, t.failures - t.scalarShapeFailures);
    Log("variant shadow: production core_native_count_calls=%u core_set_typed_calls=%u core_write_variant_calls=%u "
        "production_clones=%u production_clone_refusals=%u fallback=0 resync=0; typed getters/copies use production SDK storage",
        t.coreCountCalls, t.coreSetCalls, t.coreWriteCalls, t.productionClones, t.productionCloneRefusals);
}

}  // namespace variant_shadow
}  // namespace fc4
