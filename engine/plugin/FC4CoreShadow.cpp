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

#include "FC4CoreShadow.h"

#include "FC4Log.h"

#include "cGZPersistResourceKey.h"
#include "cIGZSerializable.h"
#include "cIGZString.h"

#include <algorithm>
#include <cstdarg>
#include <cstring>

namespace fc4 {
namespace {

// fc4::Log formats into a fixed buffer: longer shadow lines are split.
constexpr size_t kLogChunk = 2800;

bool PutU8(void* u, uint8_t v) { return static_cast<ShadowOStream*>(u)->SetUint8(v); }
bool PutU16(void* u, uint16_t v) { return static_cast<ShadowOStream*>(u)->SetUint16(v); }
bool PutU32(void* u, uint32_t v) { return static_cast<ShadowOStream*>(u)->SetUint32(v); }
bool PutU64(void* u, uint64_t v) { return static_cast<ShadowOStream*>(u)->SetUint64(v); }
bool PutF32(void* u, float v) { return static_cast<ShadowOStream*>(u)->SetFloat32(v); }
bool PutF64(void* u, double v) { return static_cast<ShadowOStream*>(u)->SetFloat64(v); }
bool PutBytes(void* u, const uint8_t* d, size_t n) { return static_cast<ShadowOStream*>(u)->SetVoid(d, n); }
bool PutStrCb(void* u, const uint8_t* d, size_t n) {
    return static_cast<ShadowOStream*>(u)->SetVoid(d, n);  // the length is written by the core first
}
bool PutRefCb(void* u, const fc4_ref* r) { return static_cast<ShadowOStream*>(u)->PutRefPublic(r); }
int32_t OutError(void* u) { return static_cast<ShadowOStream*>(u)->GetError(); }

}  // namespace

void ShadowOStream::AddToken(uint64_t token, uint32_t objectId, uint32_t classId) {
    if (token == 0) return;
    objects_.emplace(token, Obj{objectId, classId});
}

void ShadowOStream::Bind() {
    if (api_.user != nullptr) return;
    api_.user = this;
    api_.put_u8 = &PutU8;
    api_.put_u16 = &PutU16;
    api_.put_u32 = &PutU32;
    api_.put_u64 = &PutU64;
    api_.put_f32 = &PutF32;
    api_.put_f64 = &PutF64;
    api_.put_bytes = &PutBytes;
    api_.put_str = &PutStrCb;
    api_.put_ref = &PutRefCb;
    api_.error = &OutError;
}

bool ShadowOStream::Put(const void* p, size_t n) {
    if (n != 0 && p == nullptr) {
        good_ = false;
        return false;
    }
    const uint8_t* b = static_cast<const uint8_t*>(p);
    bytes_.insert(bytes_.end(), b, b + n);
    return true;
}

bool ShadowOStream::PutRef(const fc4_ref* r) {
    const uint64_t token = r != nullptr ? r->token : 0;
    if (token == 0) return SetUint32(0);
    const auto it = objects_.find(token);
    if (it == objects_.end()) {
        ++unknownRefs_;  // a token the capture never saw: written as null
        return SetUint32(0);
    }
    SetUint32(it->second.id);
    return SetUint32(it->second.classId);
}

bool ShadowOStream::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (riid == kFC4_IID_cISC4DBSegmentOStream || riid == GZIID_cIGZUnknown) {
        *ppvObj = static_cast<cFC4ISC4DBSegmentOStream*>(this);
        return true;
    }
    return false;
}

bool ShadowOStream::SetRZCharStr(char const* s) { return Put(s, s != nullptr ? strlen(s) : 0); }

bool ShadowOStream::SetGZStr(cIGZString const& s) { return Put(s.Data(), s.Strlen()); }

bool ShadowOStream::SetGZSerializable(cIGZSerializable const&) {
    // The core writes references through put_ref; a nested object stream is
    // not used by these records.
    good_ = false;
    return false;
}

bool ShadowOStream::SetVoid(void const* p, uint32_t n) {
    if (n != 0 && p == nullptr) {
        good_ = false;
        return false;
    }
    return Put(p, n);
}

bool ShadowOStream::WriteGZSerializable(cIGZSerializable const* p) {
    if (p == nullptr) return SetUint32(0);
    fc4_ref r{};
    r.token = reinterpret_cast<uint64_t>(const_cast<cIGZSerializable*>(p));
    r.class_id = const_cast<cIGZSerializable*>(p)->GetGZCLSID();
    return PutRef(&r);
}

bool ShadowOStream::WriteResKey(cGZPersistResourceKey const& k) {
    SetUint32(k.type);
    SetUint32(k.group);
    return SetUint32(k.instance);
}

bool ShadowOStream::WriteVariant(cIGZVariant const&) {
    good_ = false;
    return false;
}

// ---- the difference list -------------------------------------------------------------------------------

void CoreDiffs::Add(const char* fmt, ...) {
    ++n;
    ++unexplained;
    if (listed >= kMax) return;
    char buf[256];
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(buf, sizeof(buf), _TRUNCATE, fmt, ap);
    va_end(ap);
    if (listed != 0) text += ",";
    text += buf;
    ++listed;
}

void CoreDiffs::AddQ(const char* question, const char* fmt, ...) {
    ++n;
    if (listed >= kMax) return;
    char body[224];
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(body, sizeof(body), _TRUNCATE, fmt, ap);
    va_end(ap);
    char buf[256];
    _snprintf_s(buf, sizeof(buf), _TRUNCATE, "%s [%s]", body, question);
    if (listed != 0) text += ",";
    text += buf;
    ++listed;
}

void AppendF(std::string& s, const char* fmt, ...) {
    char buf[512];
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(buf, sizeof(buf), _TRUNCATE, fmt, ap);
    va_end(ap);
    s += buf;
}

void CoreShadowLine(const char* cont, const char* name, const char* date, bool month, const std::string& body) {
    std::string line;
    AppendF(line, "%s shadow %s %s: ", name, date, month ? "month" : "load");
    line += body;
    LogShadowLine(cont, reinterpret_cast<const uint8_t*>(line.data()), line.size());
}

}  // namespace fc4
