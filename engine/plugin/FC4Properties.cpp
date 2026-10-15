// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

#include "FC4Properties.h"

#include "FC4Checked.h"
#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cFC4Variant.h"
#include "fc4.h"

#include "GZServPtrs.h"
#include "cGZPersistResourceKey.h"
#include "cIGZPersistResourceManager.h"
#include "cIGZVariant.h"
#include "cISCProperty.h"
#include "cISCPropertyHolder.h"
#include "cISCResExemplar.h"
#include "cISCResExemplarCohort.h"

#include <algorithm>
#include <cstring>
#include <unordered_set>

namespace fc4 {
namespace props {

// ---- exemplars -----------------------------------------------------------------------------------

Exemplar::~Exemplar() {
    if (res_ != nullptr) FC4_RELEASE(res_);
}

bool Exemplar::Load(uint32_t type, uint32_t group, uint32_t instance) {
    if (res_ != nullptr) {
        FC4_RELEASE(res_);
        res_ = nullptr;
    }
    if (!On(Sw::ExemplarLoads)) {
        NoteSkipped(Sw::ExemplarLoads, "exemplar load (not found)");
        Log("props: exemplar %08X-%08X-%08X not loaded: switch exemplar_loads is off", type, group, instance);
        return false;
    }
    cIGZPersistResourceManagerPtr rm;
    if (!rm) {
        Log("props: no resource manager; exemplar %08X-%08X-%08X not loaded", type, group, instance);
        return false;
    }
    cGZPersistResourceKey key(type, group, instance);
    FC4_OUT(cISCResExemplar*, out, "cISCResExemplar* (GetResource out pointer)");
    const bool ok = rm->GetResource(key, GZIID_cISCResExemplar, reinterpret_cast<void**>(out.ptr()), 0, nullptr);
    out.Check();
    if (!ok || out.get() == nullptr) {
        Log("props: exemplar %08X-%08X-%08X not found", type, group, instance);
        return false;
    }
    res_ = out.get();
    FC4_ACQUIRED(res_, "GetResource");
    return true;
}

const cISCPropertyHolder* Exemplar::Holder() const { return res_ ? res_->AsISCPropertyHolder() : nullptr; }

// ---- reads ------------------------------------------------------------------------------------------

namespace {

// Game variants are read through the compiled slot order (foundations §17.1).
const cFC4IGZVariant* ValueOf(const cISCPropertyHolder* holder, uint32_t id) {
    if (holder == nullptr) return nullptr;
    if (!On(Sw::PropertyReads)) {
        NoteSkipped(Sw::PropertyReads, "GetProperty (absent)");
        return nullptr;
    }
    cISCProperty* p = AsGame(holder)->GetProperty(id);  // slot 9
    if (p == nullptr) return nullptr;
    return AsGame(p->GetPropertyValue());
}

template <class T>
bool ReadNumbers(const cFC4IGZVariant* v, std::vector<T>& out) {
    out.clear();
    if (v == nullptr) return false;
    const uint16_t type = v->GetType();
    const bool isArray = (type & cIGZVariant::TypeArray) != 0;
    const uint16_t base = static_cast<uint16_t>(type & ~cIGZVariant::TypeArray);
    const uint32_t n = isArray ? v->GetCount() : 1;
    out.reserve(n);
    for (uint32_t i = 0; i < n; ++i) {
        T value{};
        switch (base) {
            case cIGZVariant::Bool: value = static_cast<T>(isArray ? v->RefBool()[i] : v->GetValBool()); break;
            case cIGZVariant::Uint8: value = static_cast<T>(isArray ? v->RefUint8()[i] : v->GetValUint8()); break;
            case cIGZVariant::Sint8: value = static_cast<T>(isArray ? v->RefSint8()[i] : v->GetValSint8()); break;
            case cIGZVariant::Uint16: value = static_cast<T>(isArray ? v->RefUint16()[i] : v->GetValUint16()); break;
            case cIGZVariant::Sint16: value = static_cast<T>(isArray ? v->RefSint16()[i] : v->GetValSint16()); break;
            case cIGZVariant::Uint32: value = static_cast<T>(isArray ? v->RefUint32()[i] : v->GetValUint32()); break;
            case cIGZVariant::Sint32: value = static_cast<T>(isArray ? v->RefSint32()[i] : v->GetValSint32()); break;
            case cIGZVariant::Uint64: value = static_cast<T>(isArray ? v->RefUint64()[i] : v->GetValUint64()); break;
            case cIGZVariant::Sint64: value = static_cast<T>(isArray ? v->RefSint64()[i] : v->GetValSint64()); break;
            case cIGZVariant::Float32: value = static_cast<T>(isArray ? v->RefFloat32()[i] : v->GetValFloat32()); break;
            case cIGZVariant::Float64: value = static_cast<T>(isArray ? v->RefFloat64()[i] : v->GetValFloat64()); break;
            default:
                out.clear();
                return false;  // not numeric
        }
        out.push_back(value);
    }
    return true;
}

}  // namespace

bool GetU32s(const cISCPropertyHolder* holder, uint32_t id, std::vector<uint32_t>& out) {
    return ReadNumbers(ValueOf(holder, id), out);
}

bool GetI64s(const cISCPropertyHolder* holder, uint32_t id, std::vector<int64_t>& out) {
    return ReadNumbers(ValueOf(holder, id), out);
}

bool GetF32s(const cISCPropertyHolder* holder, uint32_t id, std::vector<float>& out) {
    return ReadNumbers(ValueOf(holder, id), out);
}

bool Has(const cISCPropertyHolder* holder, uint32_t id) {
    if (holder == nullptr) return false;
    if (!On(Sw::PropertyReads)) {
        NoteSkipped(Sw::PropertyReads, "HasProperty (absent)");
        return false;
    }
    return AsGame(holder)->HasProperty(id);
}

uint16_t TypeOf(const cISCPropertyHolder* holder, uint32_t id) {
    const cFC4IGZVariant* v = ValueOf(holder, id);
    return v ? v->GetType() : 0;
}

bool GetRawArrayBytes(const cISCPropertyHolder* holder, uint32_t id, std::vector<uint8_t>& out) {
    out.clear();
    const cFC4IGZVariant* v = ValueOf(holder, id);
    if (v == nullptr) return false;
    const uint16_t type = v->GetType();
    if ((type & cIGZVariant::TypeArray) == 0) return false;  // a scalar: its value is no pointer of ours
    size_t size = 0;
    switch (static_cast<uint16_t>(type & ~cIGZVariant::TypeArray)) {
        case 0:  // the void array: raw bytes
        case cIGZVariant::Bool:
        case cIGZVariant::Uint8:
        case cIGZVariant::Sint8:
        case cIGZVariant::Char:
        case cIGZVariant::RZChar: size = 1; break;
        case cIGZVariant::Uint16:
        case cIGZVariant::Sint16:
        case cIGZVariant::RZUnicodeChar: size = 2; break;
        case cIGZVariant::Uint32:
        case cIGZVariant::Sint32:
        case cIGZVariant::Float32:
        case cIGZVariant::VoidPtr: size = 4; break;
        case cIGZVariant::Uint64:
        case cIGZVariant::Sint64:
        case cIGZVariant::Float64: size = 8; break;
        default: return false;  // object arrays and unknown types
    }
    const size_t n = static_cast<size_t>(v->GetCount()) * size;
    if (n == 0) return true;
    const uint8_t* data = v->RefUint8();  // slot 95: the data pointer whatever the type
    if (data == nullptr) return false;
    out.assign(data, data + n);
    return true;
}

// ---- writes ----------------------------------------------------------------------------------------

namespace {

bool Put(cISCPropertyHolder* holder, uint32_t id, const cFC4IGZVariant& value) {
    if (holder == nullptr) return false;
    if (!On(Sw::PropertyWrites)) {
        // [choice] Reported as written, so the core's logic runs unchanged.
        NoteSkipped(Sw::PropertyWrites, "AddPropertyVariant (reported as written)");
        return true;
    }
    // [choice] The SDK does not name AddProperty's flag; false is passed. The
    // holder copies the variant.
    return AsGame(holder)->AddPropertyVariant(id, &value, false);  // slot 14
}

}  // namespace

bool SetU32s(cISCPropertyHolder* holder, uint32_t id, const std::vector<uint32_t>& values) {
    cFC4Variant v;  // never the SDK variant: its slots 8 and 60 differ from the game's
    v.SetRefUint32(const_cast<uint32_t*>(values.data()), static_cast<uint32_t>(values.size()));
    return Put(holder, id, v);
}

bool SetI64s(cISCPropertyHolder* holder, uint32_t id, const std::vector<int64_t>& values) {
    cFC4Variant v;  // never the SDK variant: its slots 8 and 60 differ from the game's
    v.SetRefSint64(const_cast<int64_t*>(values.data()), static_cast<uint32_t>(values.size()));
    return Put(holder, id, v);
}

bool SetF32s(cISCPropertyHolder* holder, uint32_t id, const std::vector<float>& values) {
    cFC4Variant v;  // never the SDK variant: its slots 8 and 60 differ from the game's
    v.SetRefFloat32(const_cast<float*>(values.data()), static_cast<uint32_t>(values.size()));
    return Put(holder, id, v);
}

bool SetTyped(cISCPropertyHolder* holder, uint32_t id, uint16_t memType, bool isArray, const void* data, size_t count) {
    if (holder == nullptr || (data == nullptr && count != 0) || (!isArray && count != 1) || count > 0x7FFFFFFFu) {
        return false;
    }
    const uint32_t n = static_cast<uint32_t>(count);
    // The array setters reference the caller's elements; Put copies them into the holder.
    void* p = const_cast<void*>(data);
    cFC4Variant v;  // never the SDK variant: its slots 8 and 60 differ from the game's
    auto scalar = [&](auto t) {
        decltype(t) value;
        std::memcpy(&value, data, sizeof(value));
        return value;
    };
    switch (memType) {
        case 0x01: isArray ? v.SetRefBool(static_cast<bool*>(p), n) : v.SetValBool(scalar(uint8_t{}) != 0); break;
        case 0x02: isArray ? v.SetRefUint8(static_cast<uint8_t*>(p), n) : v.SetValUint8(scalar(uint8_t{})); break;
        case 0x03: isArray ? v.SetRefSint8(static_cast<int8_t*>(p), n) : v.SetValSint8(scalar(int8_t{})); break;
        case 0x04: isArray ? v.SetRefUint16(static_cast<uint16_t*>(p), n) : v.SetValUint16(scalar(uint16_t{})); break;
        case 0x05: isArray ? v.SetRefSint16(static_cast<int16_t*>(p), n) : v.SetValSint16(scalar(int16_t{})); break;
        case 0x06: isArray ? v.SetRefUint32(static_cast<uint32_t*>(p), n) : v.SetValUint32(scalar(uint32_t{})); break;
        case 0x07: isArray ? v.SetRefSint32(static_cast<int32_t*>(p), n) : v.SetValSint32(scalar(int32_t{})); break;
        case 0x08: isArray ? v.SetRefUint64(static_cast<uint64_t*>(p), n) : v.SetValUint64(scalar(uint64_t{})); break;
        case 0x09: isArray ? v.SetRefSint64(static_cast<int64_t*>(p), n) : v.SetValSint64(scalar(int64_t{})); break;
        case 0x0A: isArray ? v.SetRefFloat32(static_cast<float*>(p), n) : v.SetValFloat32(scalar(float{})); break;
        case 0x0B:
            if (isArray) {
                v.SetRefFloat64(static_cast<double*>(p), n);
            } else {
                v.SetValFloat64(scalar(double{}));
            }
            break;
        case 0x0C: isArray ? v.SetRefChar(static_cast<char*>(p), n) : v.SetValChar(scalar(char{})); break;
        case 0x0D:
            isArray ? v.SetRefRZUnicodeChar(static_cast<uint16_t*>(p), n) : v.SetValRZUnicodeChar(scalar(uint16_t{}));
            break;
        case 0x0E: isArray ? v.SetRefRZChar(static_cast<char*>(p), n) : v.SetValRZChar(scalar(char{})); break;
        default: return false;
    }
    return Put(holder, id, v);
}

bool Copy(const cISCPropertyHolder* src, cISCPropertyHolder* dst, uint32_t id) {
    const cFC4IGZVariant* v = ValueOf(src, id);
    if (v == nullptr || dst == nullptr) return false;
    cFC4Variant copy;
    copy.CopyFromGame(*v);
    return Put(dst, id, copy);
}

// ---- copies into the core ----------------------------------------------------------------------

namespace {

void CollectId(cISCProperty* p, void* ctx) {
    if (p != nullptr) static_cast<std::vector<uint32_t>*>(ctx)->push_back(p->GetPropertyID());  // slot 3
}

// One element type: an array through its Ref pointer, a scalar through GetVal.
template <class T>
bool Typed(fc4_props* out, uint32_t id, uint16_t base, bool isArray, uint32_t n, const T* ref, T scalar) {
    if (isArray) return fc4_props_set_typed(out, id, base, true, n > 0 ? ref : nullptr, n);
    return fc4_props_set_typed(out, id, base, false, &scalar, 1);
}

}  // namespace

std::vector<uint32_t> PropertyIds(const cISCPropertyHolder* holder) {
    std::vector<uint32_t> ids;
    if (holder == nullptr) return ids;
    if (!On(Sw::PropertyReads)) {
        NoteSkipped(Sw::PropertyReads, "EnumProperties (none)");
        return ids;
    }
    AsGame(holder)->EnumProperties(&CollectId, &ids);  // slot 20
    return ids;
}

namespace {

// `o` as a cohort (QueryInterface 0x452160FB, one reference), or nullptr.
cISCResExemplarCohort* AsCohort(cIGZUnknown* o) {
    if (o == nullptr) return nullptr;
    void* p = nullptr;
    if (!o->QueryInterface(GZIID_cISCResExemplarCohort, &p) || p == nullptr) return nullptr;
    auto* c = static_cast<cISCResExemplarCohort*>(p);
    FC4_ACQUIRED(c, "QueryInterface");
    return c;
}

}  // namespace

std::vector<EffectiveProperty> EffectiveProperties(const cISCPropertyHolder* holder, cISCResExemplar* res) {
    std::vector<EffectiveProperty> out;
    if (holder == nullptr) return out;
    std::unordered_set<uint32_t> seen;
    for (uint32_t id : PropertyIds(holder)) {
        if (seen.insert(id).second) out.push_back(EffectiveProperty{id, holder, 0});
    }
    if (!On(Sw::PropertyReads)) return out;  // PropertyIds noted the skip
    cISCResExemplar* asked = nullptr;
    if (res == nullptr) {
        void* p = nullptr;
        auto* u = const_cast<cIGZUnknown*>(static_cast<const cIGZUnknown*>(holder));
        if (!u->QueryInterface(GZIID_cISCResExemplar, &p) || p == nullptr) return out;  // not an exemplar
        asked = static_cast<cISCResExemplar*>(p);
        FC4_ACQUIRED(asked, "QueryInterface");
        res = asked;
    }
    cISCResExemplarCohort* cohort = AsCohort(static_cast<cIGZUnknown*>(res->GetCohort()));  // slot 9, borrowed
    std::vector<const cISCResExemplarCohort*> visited;
    for (uint32_t depth = 1; cohort != nullptr && depth <= 32; ++depth) {
        if (std::find(visited.begin(), visited.end(), cohort) != visited.end()) break;  // a cycle
        visited.push_back(cohort);
        const cISCPropertyHolder* h = cohort->AsISCPropertyHolder();  // slot 8 (slot 9 is the same thunk)
        for (uint32_t id : PropertyIds(h)) {
            if (seen.insert(id).second) out.push_back(EffectiveProperty{id, h, depth});
        }
        cISCResExemplarCohort* next = AsCohort(static_cast<cIGZUnknown*>(cohort->GetParent()));  // slot 6
        FC4_RELEASE(cohort);
        cohort = next;
    }
    if (cohort != nullptr) FC4_RELEASE(cohort);
    if (asked != nullptr) FC4_RELEASE(asked);
    return out;
}

bool CopyToCore(const cISCPropertyHolder* holder, uint32_t id, fc4_props* out) {
    return CopyVariantToCore(ValueOf(holder, id), id, out);
}

bool CopyVariantToCore(const cFC4IGZVariant* v, uint32_t id, fc4_props* out) {
    if (v == nullptr || out == nullptr) return false;
    const uint16_t type = v->GetType();
    const bool a = (type & cIGZVariant::TypeArray) != 0;
    const uint16_t base = static_cast<uint16_t>(type & ~cIGZVariant::TypeArray);
    const uint32_t n = a ? v->GetCount() : 1;
    switch (base) {
        case cIGZVariant::Bool: {
            // bool is one byte (MSVC), as fc4_props_set_typed expects.
            static_assert(sizeof(bool) == 1, "bool layout");
            return Typed<bool>(out, id, base, a, n, a ? v->RefBool() : nullptr, a ? false : v->GetValBool());
        }
        case cIGZVariant::Uint8:
            return Typed<uint8_t>(out, id, base, a, n, a ? v->RefUint8() : nullptr, a ? 0 : v->GetValUint8());
        case cIGZVariant::Sint8:
            return Typed<int8_t>(out, id, base, a, n, a ? v->RefSint8() : nullptr, a ? 0 : v->GetValSint8());
        case cIGZVariant::Uint16:
            return Typed<uint16_t>(out, id, base, a, n, a ? v->RefUint16() : nullptr, a ? 0 : v->GetValUint16());
        case cIGZVariant::Sint16:
            return Typed<int16_t>(out, id, base, a, n, a ? v->RefSint16() : nullptr, a ? 0 : v->GetValSint16());
        case cIGZVariant::Uint32:
            return Typed<uint32_t>(out, id, base, a, n, a ? v->RefUint32() : nullptr, a ? 0 : v->GetValUint32());
        case cIGZVariant::Sint32:
            return Typed<int32_t>(out, id, base, a, n, a ? v->RefSint32() : nullptr, a ? 0 : v->GetValSint32());
        case cIGZVariant::Uint64:
            return Typed<uint64_t>(out, id, base, a, n, a ? v->RefUint64() : nullptr, a ? 0 : v->GetValUint64());
        case cIGZVariant::Sint64:
            return Typed<int64_t>(out, id, base, a, n, a ? v->RefSint64() : nullptr, a ? 0 : v->GetValSint64());
        case cIGZVariant::Float32:
            return Typed<float>(out, id, base, a, n, a ? v->RefFloat32() : nullptr, a ? 0.0f : v->GetValFloat32());
        case cIGZVariant::Float64:
            return Typed<double>(out, id, base, a, n, a ? v->RefFloat64() : nullptr, a ? 0.0 : v->GetValFloat64());
        case cIGZVariant::Char:
            return Typed<char>(out, id, base, a, n, a ? v->RefChar() : nullptr, a ? '\0' : v->GetValChar());
        case cIGZVariant::RZUnicodeChar:
            return Typed<uint16_t>(out, id, base, a, n, a ? v->RefRZUnicodeChar() : nullptr,
                                   a ? 0 : v->GetValRZUnicodeChar());
        case cIGZVariant::RZChar:
            return Typed<char>(out, id, base, a, n, a ? v->RefRZChar() : nullptr, a ? '\0' : v->GetValRZChar());
        default:
            return false;  // empty, pointers, objects: not kept by the core
    }
}

}  // namespace props
}  // namespace fc4
