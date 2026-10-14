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

#include "FC4PropertyHolder.h"

#include "FC4RefAudit.h"
#include "FC4Streams.h"

#include "cIGZIStream.h"
#include "cIGZOStream.h"
#include "cIGZString.h"
#include "cIGZVariant.h"

#include <cstring>
#include <vector>

namespace {

constexpr uint32_t kIID_cISCProperty = 0x452160EC;

// The variant as the SDK type the game's callers declare (cFC4Variant is
// laid out in the game's compiled order, foundations §17.1).
cIGZVariant* AsSdkVariant(cFC4Variant* v) { return reinterpret_cast<cIGZVariant*>(static_cast<cFC4IGZVariant*>(v)); }
const cIGZVariant* AsSdkVariant(const cFC4Variant* v) {
    return reinterpret_cast<const cIGZVariant*>(static_cast<const cFC4IGZVariant*>(v));
}

// The bytes of a value: an array's elements, or the scalar itself. Empty for
// strings stored as scalars (none), pointers and objects.
size_t ElementSize(uint16_t base) {
    switch (base) {
        case cIGZVariant::Bool:
        case cIGZVariant::Uint8:
        case cIGZVariant::Sint8:
        case cIGZVariant::Char:
        case cIGZVariant::RZChar:
            return 1;
        case cIGZVariant::Uint16:
        case cIGZVariant::Sint16:
        case cIGZVariant::RZUnicodeChar:
            return 2;
        case cIGZVariant::Uint32:
        case cIGZVariant::Sint32:
        case cIGZVariant::Float32:
            return 4;
        case cIGZVariant::Uint64:
        case cIGZVariant::Sint64:
        case cIGZVariant::Float64:
            return 8;
        default:
            return 0;
    }
}

const void* ArrayData(const cFC4IGZVariant& v, uint16_t base) {
    switch (base) {
        case cIGZVariant::Bool:
            return v.RefBool();
        case cIGZVariant::Uint8:
            return v.RefUint8();
        case cIGZVariant::Sint8:
            return v.RefSint8();
        case cIGZVariant::Uint16:
            return v.RefUint16();
        case cIGZVariant::Sint16:
            return v.RefSint16();
        case cIGZVariant::Uint32:
            return v.RefUint32();
        case cIGZVariant::Sint32:
            return v.RefSint32();
        case cIGZVariant::Uint64:
            return v.RefUint64();
        case cIGZVariant::Sint64:
            return v.RefSint64();
        case cIGZVariant::Float32:
            return v.RefFloat32();
        case cIGZVariant::Float64:
            return v.RefFloat64();
        case cIGZVariant::Char:
            return v.RefChar();
        case cIGZVariant::RZUnicodeChar:
            return v.RefRZUnicodeChar();
        case cIGZVariant::RZChar:
            return v.RefRZChar();
        default:
            return nullptr;
    }
}

// The value's bytes into `out` (scalars: the value in its own layout).
bool ValueBytes(const cFC4IGZVariant& v, std::vector<uint8_t>& out) {
    out.clear();
    const uint16_t type = v.GetType();
    const bool isArray = (type & cIGZVariant::TypeArray) != 0;
    const uint16_t base = static_cast<uint16_t>(type & ~cIGZVariant::TypeArray);
    const size_t size = ElementSize(base);
    if (size == 0) return false;
    if (isArray) {
        const size_t n = v.GetCount();
        const void* p = ArrayData(v, base);
        if (n != 0 && p == nullptr) return false;
        out.resize(n * size);
        if (n != 0) std::memcpy(out.data(), p, n * size);
        return true;
    }
    out.resize(size);
    switch (base) {
        case cIGZVariant::Bool: {
            const bool b = v.GetValBool();
            std::memcpy(out.data(), &b, 1);
            break;
        }
        case cIGZVariant::Uint8: out[0] = v.GetValUint8(); break;
        case cIGZVariant::Sint8: out[0] = static_cast<uint8_t>(v.GetValSint8()); break;
        case cIGZVariant::Char: out[0] = static_cast<uint8_t>(v.GetValChar()); break;
        case cIGZVariant::RZChar: out[0] = static_cast<uint8_t>(v.GetValRZChar()); break;
        case cIGZVariant::Uint16: { const uint16_t x = v.GetValUint16(); std::memcpy(out.data(), &x, 2); break; }
        case cIGZVariant::Sint16: { const int16_t x = v.GetValSint16(); std::memcpy(out.data(), &x, 2); break; }
        case cIGZVariant::RZUnicodeChar: { const uint16_t x = v.GetValRZUnicodeChar(); std::memcpy(out.data(), &x, 2); break; }
        case cIGZVariant::Uint32: { const uint32_t x = v.GetValUint32(); std::memcpy(out.data(), &x, 4); break; }
        case cIGZVariant::Sint32: { const int32_t x = v.GetValSint32(); std::memcpy(out.data(), &x, 4); break; }
        case cIGZVariant::Float32: { const float x = v.GetValFloat32(); std::memcpy(out.data(), &x, 4); break; }
        case cIGZVariant::Uint64: { const uint64_t x = v.GetValUint64(); std::memcpy(out.data(), &x, 8); break; }
        case cIGZVariant::Sint64: { const int64_t x = v.GetValSint64(); std::memcpy(out.data(), &x, 8); break; }
        case cIGZVariant::Float64: { const double x = v.GetValFloat64(); std::memcpy(out.data(), &x, 8); break; }
        default: return false;
    }
    return true;
}

// The first element (or the scalar) of a numeric value as a double.
bool FirstNumber(const cFC4IGZVariant& v, double* out) {
    std::vector<uint8_t> b;
    if (!ValueBytes(v, b) || b.empty()) return false;
    const uint16_t base = static_cast<uint16_t>(v.GetType() & ~cIGZVariant::TypeArray);
    const uint8_t* p = b.data();
    double d = 0.0;
    switch (base) {
        case cIGZVariant::Bool: d = p[0] != 0 ? 1.0 : 0.0; break;
        case cIGZVariant::Uint8: d = p[0]; break;
        case cIGZVariant::Sint8: d = static_cast<int8_t>(p[0]); break;
        case cIGZVariant::Uint16: { uint16_t x; std::memcpy(&x, p, 2); d = x; break; }
        case cIGZVariant::Sint16: { int16_t x; std::memcpy(&x, p, 2); d = x; break; }
        case cIGZVariant::Uint32: { uint32_t x; std::memcpy(&x, p, 4); d = x; break; }
        case cIGZVariant::Sint32: { int32_t x; std::memcpy(&x, p, 4); d = x; break; }
        case cIGZVariant::Uint64: { uint64_t x; std::memcpy(&x, p, 8); d = static_cast<double>(x); break; }
        case cIGZVariant::Sint64: { int64_t x; std::memcpy(&x, p, 8); d = static_cast<double>(x); break; }
        case cIGZVariant::Float32: { float x; std::memcpy(&x, p, 4); d = x; break; }
        case cIGZVariant::Float64: { double x; std::memcpy(&x, p, 8); d = x; break; }
        default: return false;  // characters are not numbers
    }
    *out = d;
    return true;
}

// The first element (or the scalar) as uint32, converted as C does from the
// element's own type.
bool FirstU32(const cFC4IGZVariant& v, uint32_t* out) {
    std::vector<uint8_t> b;
    if (!ValueBytes(v, b) || b.empty()) return false;
    const uint16_t base = static_cast<uint16_t>(v.GetType() & ~cIGZVariant::TypeArray);
    const uint8_t* p = b.data();
    uint32_t r = 0;
    switch (base) {
        case cIGZVariant::Bool: r = p[0] != 0 ? 1u : 0u; break;
        case cIGZVariant::Uint8: r = p[0]; break;
        case cIGZVariant::Sint8: r = static_cast<uint32_t>(static_cast<int8_t>(p[0])); break;
        case cIGZVariant::Uint16: { uint16_t x; std::memcpy(&x, p, 2); r = x; break; }
        case cIGZVariant::Sint16: { int16_t x; std::memcpy(&x, p, 2); r = static_cast<uint32_t>(x); break; }
        case cIGZVariant::Uint32: std::memcpy(&r, p, 4); break;
        case cIGZVariant::Sint32: { int32_t x; std::memcpy(&x, p, 4); r = static_cast<uint32_t>(x); break; }
        case cIGZVariant::Uint64: { uint64_t x; std::memcpy(&x, p, 8); r = static_cast<uint32_t>(x); break; }
        case cIGZVariant::Sint64: { int64_t x; std::memcpy(&x, p, 8); r = static_cast<uint32_t>(x); break; }
        case cIGZVariant::Float32: { float x; std::memcpy(&x, p, 4); r = static_cast<uint32_t>(static_cast<int64_t>(x)); break; }
        case cIGZVariant::Float64: { double x; std::memcpy(&x, p, 8); r = static_cast<uint32_t>(static_cast<int64_t>(x)); break; }
        default: return false;
    }
    *out = r;
    return true;
}

}  // namespace

// ---- helpers ----------------------------------------------------------------------------------------

bool fc4::VariantU32(const cFC4IGZVariant& v, uint32_t* out) {
    uint32_t r = 0;
    if (!FirstU32(v, &r)) return false;
    if (out != nullptr) *out = r;
    return true;
}

bool fc4::VariantTrue(const cFC4IGZVariant& v) {
    double d = 0.0;
    return FirstNumber(v, &d) && d != 0.0;
}

bool fc4::VariantToCore(const cFC4IGZVariant& v, uint32_t id, fc4_props* out) {
    if (out == nullptr) return false;
    const uint16_t type = v.GetType();
    const bool isArray = (type & cIGZVariant::TypeArray) != 0;
    const uint16_t base = static_cast<uint16_t>(type & ~cIGZVariant::TypeArray);
    if (ElementSize(base) == 0) return false;  // empty, pointers, objects: not kept by the core
    std::vector<uint8_t> b;
    if (!ValueBytes(v, b)) return false;
    const size_t n = isArray ? v.GetCount() : 1;
    return fc4_props_set_typed(out, id, base, isArray, b.empty() ? nullptr : b.data(), n);
}

// ---- cFC4Property -------------------------------------------------------------------------------------

bool cFC4Property::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (riid != kIID_cISCProperty && riid != GZIID_cIGZUnknown) return false;
    *ppvObj = static_cast<cISCProperty*>(this);
    AddRef();
    return true;
}

uint32_t cFC4Property::Release() {
    const uint32_t r = --refs_;
    if (r == 0) delete this;
    return r;
}

cIGZVariant* cFC4Property::GetPropertyValue() { return AsSdkVariant(&value_); }

const cIGZVariant* cFC4Property::GetPropertyValue() const { return AsSdkVariant(&value_); }

bool cFC4Property::SetPropertyValue(const cIGZVariant& value) {
    value_.CopyFromGame(*fc4::AsGame(&value));
    return true;
}

bool cFC4Property::Write(cIGZOStream& stream) const {
    void* p = nullptr;
    if (!stream.QueryInterface(kFC4_IID_cISC4DBSegmentOStream, &p) || p == nullptr) return false;
    auto* seg = static_cast<cFC4ISC4DBSegmentOStream*>(p);
    seg->SetUint32(id_);
    seg->WriteVariant(*AsSdkVariant(&value_));
    const bool ok = seg->GetError() == 0;
    seg->Release();
    return ok;
}

bool cFC4Property::Read(cIGZIStream& stream) {
    void* p = nullptr;
    if (!stream.QueryInterface(kFC4_IID_cISC4DBSegmentIStream, &p) || p == nullptr) return false;
    auto* seg = static_cast<cFC4ISC4DBSegmentIStream*>(p);
    uint32_t id = 0;
    if (seg->GetUint32(id)) id_ = id;
    seg->ReadVariant(*AsSdkVariant(&value_));
    const bool ok = seg->GetError() == 0;
    seg->Release();
    return ok;
}

// ---- cFC4PropertyHolder -------------------------------------------------------------------------------

cFC4PropertyHolder::~cFC4PropertyHolder() { Clear(); }

bool cFC4PropertyHolder::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (riid == kFC4_IID_cISCPropertyHolder) {
        *ppvObj = static_cast<cFC4ISCPropertyHolder*>(this);
        AddRef();
        return true;
    }
    return owner_ != nullptr && owner_->QueryInterface(riid, ppvObj);
}

uint32_t cFC4PropertyHolder::AddRef() { return owner_ ? owner_->AddRef() : 1; }

uint32_t cFC4PropertyHolder::Release() { return owner_ ? owner_->Release() : 1; }

cISCProperty* cFC4PropertyHolder::Slot(uint32_t id) {
    auto it = props_.find(id);
    if (it != props_.end()) return it->second;
    auto* p = new cFC4Property(id);
    p->AddRef();  // the map's reference
    props_.emplace(id, p);
    return p;
}

cISCProperty* cFC4PropertyHolder::Find(uint32_t id) const {
    auto it = props_.find(id);
    return it == props_.end() ? nullptr : it->second;
}

// Slot 5 of the stored object (both GetPropertyValue overloads give the same
// variant); the variant in the game's slot order (foundations §17.1).
cFC4IGZVariant* cFC4PropertyHolder::ValueOf(cISCProperty* p) {
    return p != nullptr ? fc4::AsGame(p->GetPropertyValue()) : nullptr;
}

// The holder may hold the last reference (the caller released its own after
// AddProperty, foundations §10.2): freeing the object then is expected.
void cFC4PropertyHolder::Drop(cISCProperty* p) const {
    if (p != nullptr) FC4_RELEASE_CREATED(p);
}

bool cFC4PropertyHolder::HasProperty(uint32_t id) const {
    std::lock_guard<std::recursive_mutex> g(lock_);
    return Find(id) != nullptr;
}

// [choice, spec question] No cIGZUnknownList implementation: not supported.
bool cFC4PropertyHolder::GetPropertyList(cIGZUnknownList** list) const {
    if (list != nullptr) *list = nullptr;
    return false;
}

// [choice, spec question] The value's raw bytes (an array's elements, a
// scalar's value). A null buffer asks for the size; a short buffer gets
// nothing (false) and the size.
bool cFC4PropertyHolder::GetPropertyData(uint32_t id, void* buffer, uint32_t& length) const {
    std::vector<uint8_t> b;
    {
        std::lock_guard<std::recursive_mutex> g(lock_);
        const cFC4IGZVariant* v = ValueOf(Find(id));
        if (v == nullptr || !ValueBytes(*v, b)) return false;
    }
    const uint32_t size = static_cast<uint32_t>(b.size());
    if (buffer == nullptr) {
        length = size;
        return true;
    }
    if (length < size) {
        length = size;
        return false;
    }
    if (size != 0) std::memcpy(buffer, b.data(), size);
    length = size;
    return true;
}

bool cFC4PropertyHolder::GetPropertyInterface(uint32_t id, uint32_t riid, void** ppv) const {
    cIGZUnknown* obj = nullptr;
    {
        std::lock_guard<std::recursive_mutex> g(lock_);
        cFC4IGZVariant* v = ValueOf(Find(id));
        if (v == nullptr || v->GetType() != cIGZVariant::IGZUnknown) return false;
        obj = v->GetValUnknown();  // slot 59, borrowed
        if (obj != nullptr) obj->AddRef();
    }
    if (obj == nullptr) return false;
    const bool ok = obj->QueryInterface(riid, ppv);
    obj->Release();
    return ok;
}

bool cFC4PropertyHolder::GetPropertyString(uint32_t id, cIGZString& value) const {
    std::lock_guard<std::recursive_mutex> g(lock_);
    const cFC4IGZVariant* v = ValueOf(Find(id));
    if (v == nullptr) return false;
    const uint16_t type = v->GetType();
    if (type == (cIGZVariant::Char | cIGZVariant::TypeArray)) {
        value.FromChar(v->RefChar() ? v->RefChar() : "", v->GetCount());
        return true;
    }
    if (type == (cIGZVariant::RZChar | cIGZVariant::TypeArray)) {
        value.FromChar(v->RefRZChar() ? v->RefRZChar() : "", v->GetCount());
        return true;
    }
    return false;
}

bool cFC4PropertyHolder::GetPropertyU32(uint32_t id, uint32_t& value) const {
    std::lock_guard<std::recursive_mutex> g(lock_);
    const cFC4IGZVariant* v = ValueOf(Find(id));
    uint32_t r = 0;
    if (v == nullptr || !FirstU32(*v, &r)) return false;
    value = r;
    return true;
}

// Slot 9: the stored object itself, borrowed (no AddRef; foundations §10.2).
cISCProperty* cFC4PropertyHolder::GetProperty(uint32_t id) const {
    std::lock_guard<std::recursive_mutex> g(lock_);
    return Find(id);
}

bool cFC4PropertyHolder::AddPropertyData(uint32_t id, void* data, uint32_t length, bool) {
    std::lock_guard<std::recursive_mutex> g(lock_);
    if (cFC4IGZVariant* v = ValueOf(Slot(id))) v->SetRefUint8(static_cast<uint8_t*>(data), data ? length : 0);
    return true;
}

bool cFC4PropertyHolder::AddPropertyObject(uint32_t id, cIGZUnknown* value, bool) {
    std::lock_guard<std::recursive_mutex> g(lock_);
    if (cFC4IGZVariant* v = ValueOf(Slot(id))) v->SetValUnknown(value);
    return true;
}

bool cFC4PropertyHolder::AddPropertyString(uint32_t id, cIGZString const& value, bool) {
    std::lock_guard<std::recursive_mutex> g(lock_);
    const char* s = value.ToChar();
    if (cFC4IGZVariant* v = ValueOf(Slot(id))) v->SetRefChar(const_cast<char*>(s ? s : ""), s ? value.Strlen() : 0);
    return true;
}

bool cFC4PropertyHolder::AddPropertyU32(uint32_t id, uint32_t value, bool) {
    std::lock_guard<std::recursive_mutex> g(lock_);
    if (cFC4IGZVariant* v = ValueOf(Slot(id))) v->SetValUint32(value);
    return true;
}

bool cFC4PropertyHolder::AddPropertyVariant(uint32_t id, cFC4IGZVariant const* value, bool) {
    std::lock_guard<std::recursive_mutex> g(lock_);
    cFC4IGZVariant* v = ValueOf(Slot(id));
    if (v == nullptr) return true;
    if (value == nullptr) {
        v->Erase();
    } else {
        v->CopyFrom(*value);
    }
    return true;
}

// Slot 15, AddProperty(cISCProperty*, bool) (foundations §10.2 "Ownership"):
// the caller's object itself is stored, with a reference of ours, under its
// own ID; an entry of that ID is replaced and released. [choice] A null
// property adds nothing (false).
bool cFC4PropertyHolder::AddPropertyRecord(cISCProperty* property, bool) {
    if (property == nullptr) return false;
    const uint32_t id = property->GetPropertyID();  // slot 3
    cISCProperty* old = nullptr;
    {
        std::lock_guard<std::recursive_mutex> g(lock_);
        auto it = props_.find(id);
        if (it != props_.end() && it->second == property) return true;  // already the entry
        FC4_ADDREF(property);  // the map's reference
        if (it != props_.end()) {
            old = it->second;
            it->second = property;
        } else {
            props_.emplace(id, property);
        }
    }
    Drop(old);
    return true;
}

// Slot 16 [choice]: the caller's value is copied into the entry of its ID
// (the caller's object is not kept).
bool cFC4PropertyHolder::CopyAddProperty(cISCProperty* property, bool) {
    if (property == nullptr) return false;
    const uint32_t id = property->GetPropertyID();  // slot 3
    const cFC4IGZVariant* from = ValueOf(property);
    std::lock_guard<std::recursive_mutex> g(lock_);
    cISCProperty* entry = Slot(id);
    if (entry == property) return true;
    cFC4IGZVariant* v = ValueOf(entry);
    if (v == nullptr) return true;
    if (from == nullptr) {
        v->Erase();
    } else {
        v->CopyFrom(*from);
    }
    return true;
}

bool cFC4PropertyHolder::RemoveProperty(uint32_t id, bool) {
    cISCProperty* p = nullptr;
    {
        std::lock_guard<std::recursive_mutex> g(lock_);
        auto it = props_.find(id);
        if (it == props_.end()) return false;
        p = it->second;
        props_.erase(it);
    }
    Drop(p);
    return true;
}

bool cFC4PropertyHolder::RemoveAllProperties(bool) {
    Clear();
    return true;
}

bool cFC4PropertyHolder::EnumPropertiesFiltered(FilterFn filter, EnumFn fn, void* data) const {
    std::vector<cISCProperty*> snapshot;
    {
        std::lock_guard<std::recursive_mutex> g(lock_);
        snapshot.reserve(props_.size());
        for (const auto& e : props_) {
            FC4_ADDREF(e.second);
            snapshot.push_back(e.second);
        }
    }
    for (cISCProperty* p : snapshot) {
        if (fn != nullptr && (filter == nullptr || filter(p, data))) fn(p, data);
        Drop(p);
    }
    return true;
}

bool cFC4PropertyHolder::EnumProperties(EnumFn fn, void* data) const {
    return EnumPropertiesFiltered(nullptr, fn, data);
}

bool cFC4PropertyHolder::CompactProperties() { return true; }

// The current values, read from the stored objects now (a write the game made
// in place through a variant's data pointer is included).
fc4_props* cFC4PropertyHolder::ToCore() const {
    fc4_props* out = fc4_props_create();
    if (out == nullptr) return nullptr;
    std::lock_guard<std::recursive_mutex> g(lock_);
    for (const auto& e : props_) {
        if (const cFC4IGZVariant* v = ValueOf(e.second)) fc4::VariantToCore(*v, e.first, out);
    }
    return out;
}

void cFC4PropertyHolder::Clear() {
    std::map<uint32_t, cISCProperty*> old;
    {
        std::lock_guard<std::recursive_mutex> g(lock_);
        old.swap(props_);
    }
    for (auto& e : old) Drop(e.second);
}

void cFC4PropertyHolder::FromCoreProperty(void* ctx, uint32_t id, uint16_t memType, bool isArray, const void* data,
                                          size_t count) {
    auto* h = static_cast<cFC4PropertyHolder*>(ctx);
    if (h == nullptr) return;
    std::lock_guard<std::recursive_mutex> g(h->lock_);
    cFC4IGZVariant* pv = ValueOf(h->Slot(id));
    if (pv == nullptr) return;
    cFC4IGZVariant& v = *pv;
    const uint32_t n = static_cast<uint32_t>(count);
    const bool a = isArray;
    switch (memType) {
        case cIGZVariant::Bool: {
            const bool* p = static_cast<const bool*>(data);
            if (a) v.SetRefBool(const_cast<bool*>(p), n); else v.SetValBool(p != nullptr && *p);
            break;
        }
        case cIGZVariant::Uint8: {
            const auto* p = static_cast<const uint8_t*>(data);
            if (a) v.SetRefUint8(const_cast<uint8_t*>(p), n); else v.SetValUint8(p ? *p : 0);
            break;
        }
        case cIGZVariant::Sint8: {
            const auto* p = static_cast<const int8_t*>(data);
            if (a) v.SetRefSint8(const_cast<int8_t*>(p), n); else v.SetValSint8(p ? *p : 0);
            break;
        }
        case cIGZVariant::Uint16: {
            const auto* p = static_cast<const uint16_t*>(data);
            if (a) v.SetRefUint16(const_cast<uint16_t*>(p), n); else v.SetValUint16(p ? *p : 0);
            break;
        }
        case cIGZVariant::Sint16: {
            const auto* p = static_cast<const int16_t*>(data);
            if (a) v.SetRefSint16(const_cast<int16_t*>(p), n); else v.SetValSint16(p ? *p : 0);
            break;
        }
        case cIGZVariant::Uint32: {
            const auto* p = static_cast<const uint32_t*>(data);
            if (a) v.SetRefUint32(const_cast<uint32_t*>(p), n); else v.SetValUint32(p ? *p : 0);
            break;
        }
        case cIGZVariant::Sint32: {
            const auto* p = static_cast<const int32_t*>(data);
            if (a) v.SetRefSint32(const_cast<int32_t*>(p), n); else v.SetValSint32(p ? *p : 0);
            break;
        }
        case cIGZVariant::Uint64: {
            const auto* p = static_cast<const uint64_t*>(data);
            if (a) v.SetRefUint64(const_cast<uint64_t*>(p), n); else v.SetValUint64(p ? *p : 0);
            break;
        }
        case cIGZVariant::Sint64: {
            const auto* p = static_cast<const int64_t*>(data);
            if (a) v.SetRefSint64(const_cast<int64_t*>(p), n); else v.SetValSint64(p ? *p : 0);
            break;
        }
        case cIGZVariant::Float32: {
            const auto* p = static_cast<const float*>(data);
            if (a) v.SetRefFloat32(const_cast<float*>(p), n); else v.SetValFloat32(p ? *p : 0.0f);
            break;
        }
        case cIGZVariant::Float64: {
            const auto* p = static_cast<const double*>(data);
            if (a) v.SetRefFloat64(const_cast<double*>(p), n); else v.SetValFloat64(p ? *p : 0.0);
            break;
        }
        case cIGZVariant::Char: {
            const auto* p = static_cast<const char*>(data);
            if (a) v.SetRefChar(const_cast<char*>(p), n); else v.SetValChar(p ? *p : '\0');
            break;
        }
        case cIGZVariant::RZUnicodeChar: {
            const auto* p = static_cast<const uint16_t*>(data);
            if (a) v.SetRefRZUnicodeChar(const_cast<uint16_t*>(p), n); else v.SetValRZUnicodeChar(p ? *p : 0);
            break;
        }
        case cIGZVariant::RZChar: {
            const auto* p = static_cast<const char*>(data);
            if (a) v.SetRefRZChar(const_cast<char*>(p), n); else v.SetValRZChar(p ? *p : '\0');
            break;
        }
        default:
            v.Erase();
            break;
    }
}

size_t cFC4PropertyHolder::Count() const {
    std::lock_guard<std::recursive_mutex> g(lock_);
    return props_.size();
}
