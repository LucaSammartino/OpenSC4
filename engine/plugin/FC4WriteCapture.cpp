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

#include "FC4WriteCapture.h"

#include "FC4Log.h"
#include "FC4Interfaces.h"
#include "FC4RefAudit.h"

#include "cGZPersistResourceKey.h"
#include "cIGZSerializable.h"
#include "cIGZString.h"

#include <algorithm>
#include <cstring>

namespace fc4 {
namespace {

constexpr uint32_t kIID_cISC4SimGrid = 0xC9BD939A;
// fc4::Log formats into a fixed buffer: longer shadow lines are split.
constexpr size_t kLogChunk = 2800;

// The object's identity for `iid` (nullptr if it does not answer): queried,
// and the reference dropped at once.
void* Borrowed(cIGZSerializable* obj, uint32_t iid) {
    void* q = nullptr;
    if (!obj->QueryInterface(iid, &q) || q == nullptr) return nullptr;
    cIGZUnknown* u = static_cast<cIGZUnknown*>(q);
    FC4_ACQUIRED(u, "QueryInterface");
    FC4_RELEASE(u);
    return q;
}

}  // namespace

bool WriteCapture::Capture(cIGZUnknown* obj, bool* answered) {
    bytes_.clear();
    refs_.clear();
    index_.clear();
    good_ = true;
    if (answered != nullptr) *answered = false;
    if (obj == nullptr) return false;
    void* p = nullptr;
    if (!obj->QueryInterface(GZIID_cIGZSerializable, &p) || p == nullptr) return false;
    if (answered != nullptr) *answered = true;
    cIGZSerializable* ser = static_cast<cIGZSerializable*>(p);
    FC4_ACQUIRED(ser, "QueryInterface");
    const bool ok = ser->Write(*this);
    FC4_RELEASE(ser);
    return ok && good_;
}

void* WriteCapture::Grid(uint32_t id) const { return (id >= 1 && id <= refs_.size()) ? refs_[id - 1].grid : nullptr; }

const void* WriteCapture::Object(uint32_t id) const {
    return (id >= 1 && id <= refs_.size()) ? refs_[id - 1].object : nullptr;
}

uint32_t WriteCapture::ClassOf(uint32_t id) const {
    return (id >= 1 && id <= refs_.size()) ? refs_[id - 1].classId : 0;
}

bool WriteCapture::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (riid == kFC4_IID_cISC4DBSegmentOStream || riid == GZIID_cIGZUnknown) {
        *ppvObj = static_cast<cFC4ISC4DBSegmentOStream*>(this);
        return true;
    }
    return false;
}

bool WriteCapture::SetRZCharStr(char const* s) { return PutStr(s, s ? strlen(s) : 0); }

bool WriteCapture::SetGZStr(cIGZString const& s) { return PutStr(s.Data(), s.Strlen()); }

bool WriteCapture::SetGZSerializable(cIGZSerializable const& s) {
    if (!const_cast<cIGZSerializable&>(s).Write(*this)) good_ = false;
    return good_;
}

bool WriteCapture::SetVoid(void const* p, uint32_t n) {
    if (n != 0 && p == nullptr) {
        good_ = false;
        return false;
    }
    return Put(p, n);
}

bool WriteCapture::WriteGZSerializable(cIGZSerializable const* p) {
    if (p == nullptr) return SetUint32(0);
    cIGZSerializable* obj = const_cast<cIGZSerializable*>(p);
    size_t i = refs_.size();
    const auto it = index_.find(obj);
    if (it != index_.end()) {
        i = it->second;
    } else {
        refs_.push_back(Ref{obj, Borrowed(obj, kIID_cISC4SimGrid), obj->GetGZCLSID()});
        index_.emplace(obj, i);
    }
    SetUint32(static_cast<uint32_t>(i + 1));
    return SetUint32(refs_[i].classId);
}

bool WriteCapture::WriteResKey(cGZPersistResourceKey const& k) {
    SetUint32(k.type);
    SetUint32(k.group);
    return SetUint32(k.instance);
}

bool WriteCapture::WriteVariant(cIGZVariant const& v) {
    // foundations §§9.1–9.2, 17.1: capture native variant values through the
    // verified interface, independently of the Rust variant writer being
    // checked by the automaton/construction record shadow.
    const cFC4IGZVariant* value = AsGame(&v);
    const uint16_t type = value->GetType();
    if (type == 0x10) {
        SetUint32(0xA9BEE9AC);
        void* serial = nullptr;
        if (!const_cast<cFC4IGZVariant*>(value)->GetValUnknownAs(GZIID_cIGZSerializable, &serial) ||
            serial == nullptr) return WriteGZSerializable(nullptr);
        auto* object = static_cast<cIGZSerializable*>(serial);
        FC4_ACQUIRED(object, "variant GetValUnknownAs");
        const bool ok = WriteGZSerializable(object);
        FC4_RELEASE(object);
        return ok;
    }
    SetUint32(0);
    const uint16_t base = type & 0x7FFF;
    const bool array = (type & 0x8000) != 0;
    static const uint32_t tags[] = {0, 0xB, 1, 5, 2, 6, 3, 7, 4, 8, 9, 0xA, 0x11, 0x10, 0xC, 0x12};
    if (base >= sizeof(tags) / sizeof(tags[0])) {
        good_ = false;
        return false;
    }
    SetUint32(tags[base] | (array ? 0x8000u : 0u));
    if (base == 0xF) {  // unsupported saved pointer values, foundations §9.2
        good_ = false;
        return false;
    }
    if (array) {
        static const size_t widths[] = {1, 1, 1, 1, 2, 2, 4, 4, 8, 8, 4, 8, 1, 2, 1};
        const uint32_t count = value->GetCount();
        const uint64_t bytes = static_cast<uint64_t>(count) * widths[base];
        // RefUint8 exposes the array's raw storage (FC4Properties.h). No
        // conversion or terminator is written; strings have the same header.
        const uint8_t* data = value->RefUint8();
        if (bytes > UINT32_MAX || (count != 0 && data == nullptr)) {
            good_ = false;
            return false;
        }
        SetUint32(count);
        return count == 0 || Put(data, static_cast<size_t>(bytes));
    }
    switch (base) {
        case 0: return good_;
        case 1: return SetUint8(value->GetValBool() ? 1 : 0);
        case 2: return SetUint8(value->GetValUint8());
        case 3: return SetSint8(value->GetValSint8());
        case 4: return SetUint16(value->GetValUint16());
        case 5: return SetSint16(value->GetValSint16());
        case 6: return SetUint32(value->GetValUint32());
        case 7: return SetSint32(value->GetValSint32());
        case 8: return SetUint64(value->GetValUint64());
        case 9: return SetSint64(value->GetValSint64());
        case 0xA: return SetFloat32(value->GetValFloat32());
        case 0xB: return SetFloat64(value->GetValFloat64());
        case 0xC: return SetSint8(value->GetValChar());
        case 0xD: return SetUint16(value->GetValRZUnicodeChar());
        case 0xE: return SetSint8(value->GetValRZChar());
        default: good_ = false; return false;
    }
}

bool WriteCapture::Put(const void* p, size_t n) {
    const uint8_t* b = static_cast<const uint8_t*>(p);
    bytes_.insert(bytes_.end(), b, b + n);  // x86: already little-endian
    return good_;
}

bool WriteCapture::PutStr(const char* s, size_t n) {
    if (n > 0x00FFFFFF || (n != 0 && s == nullptr)) {
        good_ = false;
        return false;
    }
    SetUint32(static_cast<uint32_t>(n));
    return Put(s, n);
}

bool CaptureReader::Get(void* out, size_t n) {
    if (pos_ + n > b_.size()) {
        pos_ = b_.size();
        return false;
    }
    memcpy(out, b_.data() + pos_, n);
    pos_ += n;
    return true;
}

bool CaptureReader::Ref(uint32_t& id) {
    if (!U32(id)) return false;
    uint32_t cls = 0;
    return id == 0 || U32(cls);
}

// ---- CaptureIStream -------------------------------------------------------------------------------

namespace {

CaptureIStream& CIS(void* u) { return *static_cast<CaptureIStream*>(u); }
bool CisU8(void* u, uint8_t* o) { return CIS(u).Take(o, 1); }
bool CisU16(void* u, uint16_t* o) { return CIS(u).Take(o, 2); }
bool CisU32(void* u, uint32_t* o) { return CIS(u).Take(o, 4); }
bool CisU64(void* u, uint64_t* o) { return CIS(u).Take(o, 8); }
bool CisF32(void* u, float* o) { return CIS(u).Take(o, 4); }
bool CisF64(void* u, double* o) { return CIS(u).Take(o, 8); }
bool CisBytes(void* u, uint8_t* o, size_t n) { return CIS(u).Take(o, n); }
bool CisSkip(void* u, size_t n) { return CIS(u).Take(nullptr, n); }
bool CisStr(void* u, fc4_bytes_sink sink, void* ctx) { return CIS(u).Str(sink, ctx); }
bool CisRef(void* u, uint32_t iid, fc4_ref* out) { return CIS(u).Ref(iid, out); }
int32_t CisError(void* u) { return CIS(u).good() ? 0 : 1; }

}  // namespace

CaptureIStream::CaptureIStream(const WriteCapture& cap, const std::vector<uint8_t>* bytes)
    : cap_(cap), bytes_(bytes) {
    api_.struct_size = sizeof(api_);
    api_.user = this;
    api_.get_u8 = &CisU8;
    api_.get_u16 = &CisU16;
    api_.get_u32 = &CisU32;
    api_.get_u64 = &CisU64;
    api_.get_f32 = &CisF32;
    api_.get_f64 = &CisF64;
    api_.get_bytes = &CisBytes;
    api_.get_str = &CisStr;
    api_.get_ref = &CisRef;
    api_.skip = &CisSkip;
    api_.error = &CisError;
}

bool CaptureIStream::Take(void* out, size_t n) {
    const std::vector<uint8_t>& b = B();
    if (!good_ || n > b.size() - pos_) {
        good_ = false;
        return false;
    }
    if (out != nullptr && n != 0) memcpy(out, b.data() + pos_, n);
    pos_ += n;
    return true;
}

bool CaptureIStream::Str(fc4_bytes_sink sink, void* ctx) {
    uint32_t n = 0;
    if (!Take(&n, 4)) return false;
    const std::vector<uint8_t>& b = B();
    if (n > b.size() - pos_) {
        good_ = false;
        return false;
    }
    if (sink != nullptr) sink(ctx, b.data() + pos_, n);
    pos_ += n;
    return true;
}

bool CaptureIStream::Ref(uint32_t iid, fc4_ref* out) {
    uint32_t id = 0, cls = 0;
    if (!Take(&id, 4)) return false;
    if (id != 0 && !Take(&cls, 4)) return false;
    fc4_ref r{};
    r.class_id = cls;
    cIGZSerializable* obj = id != 0 ? cap_.SerializableAt(id) : nullptr;
    if (obj != nullptr) {
        void* p = nullptr;
        if (obj->QueryInterface(iid == 0 ? GZIID_cIGZUnknown : iid, &p) && p != nullptr) {
            cIGZUnknown* q = static_cast<cIGZUnknown*>(p);
            if (IsOurObject(q)) {
                q->Release();  // ours: no reference travels (resolve_ref takes one)
            } else {
                FC4_ACQUIRED(q, "QueryInterface");  // the reader's reference
            }
            r.token = TokenOf(q);
        }
    }
    if (out != nullptr) *out = r;
    return true;
}

void LogShadowLine(const char* cont, const uint8_t* data, size_t len) {
    const char* p = reinterpret_cast<const char*>(data);
    size_t off = 0;
    bool first = true;
    while (first || off < len) {
        size_t n = std::min(kLogChunk, len - off);
        if (off + n < len) {
            size_t cut = n;
            while (cut > kLogChunk / 2 && p[off + cut - 1] != ' ') --cut;
            if (cut > kLogChunk / 2) n = cut;
        }
        if (first) {
            Log("%.*s", static_cast<int>(n), p + off);
        } else {
            Log("%s %.*s", cont, static_cast<int>(n), p + off);
        }
        off += n;
        first = false;
    }
}

}  // namespace fc4
