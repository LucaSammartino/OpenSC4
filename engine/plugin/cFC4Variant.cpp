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

#include "cFC4Variant.h"

#include "fc4.h"

#include <cstdint>
#include <cstring>
#include <vector>

bool cFC4Variant::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (riid == GZIID_cIGZVariant || riid == GZIID_cIGZUnknown) {
        *ppvObj = static_cast<cFC4IGZVariant*>(this);
        AddRef();
        return true;
    }
    return false;
}

uint32_t cFC4Variant::Release() {
    if (refs_ > 0) --refs_;
    const uint32_t left = refs_;
    if (left == 0 && heap_) delete this;  // only CreateCopy's objects
    return left;
}

bool cFC4Variant::CopyFrom(cFC4IGZVariant const& value) {
    if (&value == this) return true;
    v_.CopyFrom(*fc4::AsSdk(&value));
    return true;
}

bool cFC4Variant::CreateCopy(cFC4IGZVariant** value) const {
    if (value == nullptr) return false;
    // [choice] A heap copy with one reference for the caller.
    cFC4Variant* c = new cFC4Variant();
    c->heap_ = true;
    c->v_.CopyFrom(v_);
    c->AddRef();
    *value = c;
    return true;
}

// foundations §17.1: slot 8 takes the type and always answers true.
bool cFC4Variant::CanGetVal(uint16_t) const { return true; }

// Slot 58, the raw-bytes setter (neighbors.md §15.4 NG37, the original
// 0x00915743): the value becomes raw bytes, type 0x8000, count = `length`. A
// value that is already raw bytes of that count keeps its buffer; otherwise a
// new one of `length` bytes. The bytes are copied unless `value` is null.
// [choice] A new buffer with nothing copied holds zeros (the original's holds
// whatever its allocator gave).
void cFC4Variant::SetValVoid(void* value, uint32_t length) {
    if (v_.GetType() == cIGZVariant::VoidArray && v_.GetCount() == length) {
        if (value != nullptr && length != 0 && v_.RefVoid() != nullptr) v_.RefVoid(value, length);  // copies in place
        return;
    }
    if (value != nullptr || length == 0) {
        v_.RefVoid(value, length);  // SetType(0x8000, length) and a copy of the bytes
        return;
    }
    std::vector<uint8_t> zeros(length);
    v_.RefVoid(zeros.data(), length);
}

// Slot 57, GetValVoid (data-formats §5.2, EX17): only raw bytes answer; a
// `length` larger than the count is lowered to it, then `length` bytes are
// copied. Any other type: false, both unchanged. [choice] A null `value`
// copies nothing.
bool cFC4Variant::GetValVoid(void* value, uint32_t& length) const {
    if (v_.GetType() != cIGZVariant::VoidArray) return false;
    if (length > v_.GetCount()) length = v_.GetCount();
    const void* src = v_.RefVoid();
    if (value != nullptr && src != nullptr && length != 0) std::memcpy(value, src, length);
    return true;
}

// Slot 55, GetValString (data-formats §5.1), in the core
// (fc4_variant_get_string): the value's text replaces `value`; true only for
// a string (RZChar array), false with the round-trip text for every other
// type (Uint32 7 gives "7"). [choice] of §5.1: objects and pointers are a
// missing property (false, `value` unchanged). The scalar Char and
// RZUnicodeChar: false, unchanged (EX53, §5.1 gives no text for them).
namespace {
void SinkToString(void* ctx, const uint8_t* data, size_t len) {
    static_cast<cIGZString*>(ctx)->FromChar(reinterpret_cast<const char*>(data), static_cast<uint32_t>(len));
}
}  // namespace

bool cFC4Variant::GetValString(cIGZString& value) const {
    const uint16_t type = v_.GetType();
    const bool isArray = (type & cIGZVariant::TypeArray) != 0;
    std::vector<uint8_t> b;
    if ((type & ~cIGZVariant::TypeArray) != 0 && !fc4::VariantValueBytes(*this, b)) return false;
    const size_t n = isArray ? v_.GetCount() : 1;
    return fc4_variant_get_string(type, b.empty() ? nullptr : b.data(), (type & ~cIGZVariant::TypeArray) != 0 ? n : 0,
                                  SinkToString, &value) == 1;
}

// foundations §17.1: slot 60 is GetValUnknown(riid, void**).
// [choice] The stored object queried for `riid`.
bool cFC4Variant::GetValUnknownAs(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    *ppvObj = nullptr;
    cIGZUnknown* u = v_.GetValUnknown();
    return u != nullptr && u->QueryInterface(riid, ppvObj);
}
