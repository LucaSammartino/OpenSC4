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

#include "FC4Log.h"

#include "GZServPtrs.h"
#include "cGZPersistResourceKey.h"
#include "cIGZPersistResourceManager.h"
#include "cIGZVariant.h"
#include "cISCProperty.h"
#include "cISCPropertyHolder.h"
#include "cISCResExemplar.h"
#include "cRZBaseVariant.h"

namespace fc4 {
namespace props {

// ---- exemplars -----------------------------------------------------------------------------------

Exemplar::~Exemplar() {
    if (res_ != nullptr) res_->Release();
}

bool Exemplar::Load(uint32_t type, uint32_t group, uint32_t instance) {
    if (res_ != nullptr) {
        res_->Release();
        res_ = nullptr;
    }
    cIGZPersistResourceManagerPtr rm;
    if (!rm) {
        Log("props: no resource manager; exemplar %08X-%08X-%08X not loaded", type, group, instance);
        return false;
    }
    cGZPersistResourceKey key(type, group, instance);
    if (!rm->GetResource(key, GZIID_cISCResExemplar, reinterpret_cast<void**>(&res_), 0, nullptr) || res_ == nullptr) {
        res_ = nullptr;
        Log("props: exemplar %08X-%08X-%08X not found", type, group, instance);
        return false;
    }
    return true;
}

const cISCPropertyHolder* Exemplar::Holder() const { return res_ ? res_->AsISCPropertyHolder() : nullptr; }

// ---- reads ------------------------------------------------------------------------------------------

namespace {

const cIGZVariant* ValueOf(const cISCPropertyHolder* holder, uint32_t id) {
    if (holder == nullptr) return nullptr;
    cISCProperty* p = holder->GetProperty(id);
    if (p == nullptr) return nullptr;
    return p->GetPropertyValue();
}

template <class T>
bool ReadNumbers(const cIGZVariant* v, std::vector<T>& out) {
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

bool Has(const cISCPropertyHolder* holder, uint32_t id) { return holder != nullptr && holder->HasProperty(id); }

uint16_t TypeOf(const cISCPropertyHolder* holder, uint32_t id) {
    const cIGZVariant* v = ValueOf(holder, id);
    return v ? v->GetType() : 0;
}

// ---- writes ----------------------------------------------------------------------------------------

namespace {

bool Put(cISCPropertyHolder* holder, uint32_t id, const cIGZVariant& value) {
    if (holder == nullptr) return false;
    // [choice] The SDK does not name AddProperty's flag; false is passed. The
    // holder copies the variant.
    return holder->AddProperty(id, &value, false);
}

}  // namespace

bool SetU32s(cISCPropertyHolder* holder, uint32_t id, const std::vector<uint32_t>& values) {
    cRZBaseVariant v;
    v.RefUint32(const_cast<uint32_t*>(values.data()), static_cast<uint32_t>(values.size()));
    return Put(holder, id, v);
}

bool SetI64s(cISCPropertyHolder* holder, uint32_t id, const std::vector<int64_t>& values) {
    cRZBaseVariant v;
    v.RefSint64(const_cast<int64_t*>(values.data()), static_cast<uint32_t>(values.size()));
    return Put(holder, id, v);
}

bool SetF32s(cISCPropertyHolder* holder, uint32_t id, const std::vector<float>& values) {
    cRZBaseVariant v;
    v.RefFloat32(const_cast<float*>(values.data()), static_cast<uint32_t>(values.size()));
    return Put(holder, id, v);
}

bool Copy(const cISCPropertyHolder* src, cISCPropertyHolder* dst, uint32_t id) {
    const cIGZVariant* v = ValueOf(src, id);
    if (v == nullptr || dst == nullptr) return false;
    cRZBaseVariant copy(*v);
    return Put(dst, id, copy);
}

}  // namespace props
}  // namespace fc4
