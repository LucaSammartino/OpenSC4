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

cIGZSerializable* WriteCapture::Object(uint32_t id) const {
    return (id >= 1 && id <= refs_.size()) ? static_cast<cIGZSerializable*>(const_cast<void*>(refs_[id - 1].object))
                                           : nullptr;
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
    size_t i = 0;
    while (i < refs_.size() && refs_[i].object != obj) ++i;
    if (i == refs_.size()) refs_.push_back(Ref{obj, Borrowed(obj, kIID_cISC4SimGrid), obj->GetGZCLSID()});
    SetUint32(static_cast<uint32_t>(i + 1));
    return SetUint32(refs_[i].classId);
}

bool WriteCapture::WriteResKey(cGZPersistResourceKey const& k) {
    SetUint32(k.type);
    SetUint32(k.group);
    return SetUint32(k.instance);
}

bool WriteCapture::WriteVariant(cIGZVariant const&) {
    good_ = false;  // not part of the records read this way
    return false;
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
