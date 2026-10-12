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

#include "FC4Streams.h"

#include "FC4Checked.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"

#include "cIGZSerializable.h"
#include "cIGZString.h"
#include "cRZBaseString.h"

#include <cstring>
#include <string>

namespace fc4 {
namespace {

// ---- output callbacks --------------------------------------------------------------------------

cIGZOStream& O(void* u) { return static_cast<OStreamAdapter*>(u)->Stream(); }

bool PutU8(void* u, uint8_t v) { return O(u).SetUint8(v); }
bool PutU16(void* u, uint16_t v) { return O(u).SetUint16(v); }
bool PutU32(void* u, uint32_t v) { return O(u).SetUint32(v); }
bool PutU64(void* u, uint64_t v) { return O(u).SetUint64(v); }
bool PutF32(void* u, float v) { return O(u).SetFloat32(v); }
bool PutF64(void* u, double v) { return O(u).SetFloat64(v); }
bool PutBytes(void* u, const uint8_t* d, size_t n) {
    return n == 0 ? O(u).GetError() == 0 : O(u).SetVoid(d, static_cast<uint32_t>(n));
}
bool PutStr(void* u, const uint8_t* d, size_t n) {
    cRZBaseString s(reinterpret_cast<const char*>(d), n);
    return O(u).SetGZStr(s);
}
bool PutRef(void* u, const fc4_ref* r) { return static_cast<OStreamAdapter*>(u)->WriteRef(r ? r->token : 0); }
int32_t OError(void* u) { return O(u).GetError(); }

// ---- input callbacks ---------------------------------------------------------------------------

cIGZIStream& I(void* u) { return static_cast<IStreamAdapter*>(u)->Stream(); }

bool GetU8(void* u, uint8_t* v) { return I(u).GetUint8(*v); }
bool GetU16(void* u, uint16_t* v) { return I(u).GetUint16(*v); }
bool GetU32(void* u, uint32_t* v) { return I(u).GetUint32(*v); }
bool GetU64(void* u, uint64_t* v) { return I(u).GetUint64(*v); }
bool GetF32(void* u, float* v) { return I(u).GetFloat32(*v); }
bool GetF64(void* u, double* v) { return I(u).GetFloat64(*v); }
bool GetBytes(void* u, uint8_t* out, size_t n) {
    return n == 0 ? I(u).GetError() == 0 : I(u).GetVoid(out, static_cast<uint32_t>(n));
}
bool GetStr(void* u, fc4_bytes_sink sink, void* ctx) {
    cRZBaseString s;
    if (!I(u).GetGZStr(s)) return false;
    if (sink != nullptr) sink(ctx, reinterpret_cast<const uint8_t*>(s.Data()), s.Strlen());
    return true;
}
bool GetRef(void* u, uint32_t iid, fc4_ref* out) { return static_cast<IStreamAdapter*>(u)->ReadRef(iid, out); }
bool Skip(void* u, size_t n) { return I(u).Skip(static_cast<uint32_t>(n)); }
int32_t IError(void* u) { return I(u).GetError(); }

}  // namespace

// ---- OStreamAdapter ------------------------------------------------------------------------------

OStreamAdapter::OStreamAdapter(cIGZOStream& s) : s_(s) {
    if (!s.QueryInterface(kFC4_IID_cISC4DBSegmentOStream, reinterpret_cast<void**>(&seg_))) seg_ = nullptr;
    FC4_ACQUIRED(seg_, "QueryInterface");
    api_.struct_size = sizeof(api_);
    api_.user = this;
    api_.put_u8 = &PutU8;
    api_.put_u16 = &PutU16;
    api_.put_u32 = &PutU32;
    api_.put_u64 = &PutU64;
    api_.put_f32 = &PutF32;
    api_.put_f64 = &PutF64;
    api_.put_bytes = &PutBytes;
    api_.put_str = &PutStr;
    api_.put_ref = &PutRef;
    api_.error = &OError;
}

OStreamAdapter::~OStreamAdapter() {
    if (seg_ != nullptr) FC4_RELEASE(seg_);
}

bool OStreamAdapter::WriteRef(uint64_t token) {
    if (seg_ == nullptr) {
        Log("streams: WARNING reference write on a stream without the segment interface");
        return false;
    }
    if (token == 0) return seg_->WriteGZSerializable(nullptr);
    cIGZSerializable* ser = nullptr;
    if (!FromToken(token)->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&ser)) || ser == nullptr) {
        Log("streams: WARNING referenced object %p is not serializable; writing a null reference",
            static_cast<void*>(FromToken(token)));
        return seg_->WriteGZSerializable(nullptr);
    }
    FC4_ACQUIRED(ser, "QueryInterface");
    const bool ok = seg_->WriteGZSerializable(ser);
    FC4_RELEASE(ser);
    return ok;
}

// ---- IStreamAdapter -------------------------------------------------------------------------------

IStreamAdapter::IStreamAdapter(cIGZIStream& s) : s_(s) {
    if (!s.QueryInterface(kFC4_IID_cISC4DBSegmentIStream, reinterpret_cast<void**>(&seg_))) seg_ = nullptr;
    FC4_ACQUIRED(seg_, "QueryInterface");
    api_.struct_size = sizeof(api_);
    api_.user = this;
    api_.get_u8 = &GetU8;
    api_.get_u16 = &GetU16;
    api_.get_u32 = &GetU32;
    api_.get_u64 = &GetU64;
    api_.get_f32 = &GetF32;
    api_.get_f64 = &GetF64;
    api_.get_bytes = &GetBytes;
    api_.get_str = &GetStr;
    api_.get_ref = &GetRef;
    api_.skip = &Skip;
    api_.error = &IError;
}

IStreamAdapter::~IStreamAdapter() {
    if (seg_ != nullptr) FC4_RELEASE(seg_);
}

bool IStreamAdapter::ReadRef(uint32_t iid, fc4_ref* out) {
    if (out == nullptr) return false;
    *out = fc4_ref{};
    if (seg_ == nullptr) {
        Log("streams: WARNING reference read on a stream without the segment interface");
        return false;
    }
    FC4_OUT(cIGZSerializable*, read, "cIGZSerializable* (ReadGZSerializable out pointer)");
    const bool readOk = seg_->ReadGZSerializable(read.ptr(), false);
    read.Check();
    if (!readOk) return false;
    cIGZSerializable* obj = read.get();
    if (obj == nullptr) return true;  // null reference
    FC4_ACQUIRED(obj, "ReadGZSerializable");

    // [inferred] The object comes back with a reference owned by us.
    out->class_id = obj->GetGZCLSID();
    cIFC4CoreObject* core = nullptr;
    if (obj->QueryInterface(kFC4_IID_CoreObject, reinterpret_cast<void**>(&core)) && core != nullptr) {
        // One of ours: the core keeps the object alive through its handle.
        out->token = TokenOf(static_cast<cIGZUnknown*>(obj));
        out->core_handle = core->CoreHandle();
        core->Release();
        FC4_RELEASE(obj);  // ours: not audited
        return true;
    }
    if (iid == 0 || iid == GZIID_cIGZSerializable) {
        out->token = TokenOf(static_cast<cIGZUnknown*>(obj));  // the caller owns this reference
        return true;
    }
    void* typed = nullptr;
    const bool has = obj->QueryInterface(iid, &typed) && typed != nullptr;
    if (has) FC4_ACQUIRED(static_cast<cIGZUnknown*>(typed), "QueryInterface");
    FC4_RELEASE(obj);
    if (!has) {
        Log("streams: referenced object of class 0x%08X lacks interface 0x%08X; read as null", out->class_id, iid);
        out->class_id = 0;
        return true;  // the stream itself stays good (foundations §2.7)
    }
    out->token = TokenOf(typed);  // the caller owns this reference
    return true;
}

}  // namespace fc4
