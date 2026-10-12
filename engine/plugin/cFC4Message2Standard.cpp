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

#include "cFC4Message2Standard.h"

#include "cIGZSerializable.h"
#include "cIGZString.h"

#include <atomic>

namespace {
std::atomic<long> g_live{0};

template <class T>
void Assign(T*& slot, T* value) {
    if (value != nullptr) value->AddRef();
    T* old = slot;
    slot = value;
    if (old != nullptr) old->Release();
}
}  // namespace

cFC4Message2Standard::cFC4Message2Standard() { ++g_live; }

cFC4Message2Standard::~cFC4Message2Standard() {
    if (string_ != nullptr) string_->Release();
    if (unknown_ != nullptr) unknown_->Release();
    if (extra_ != nullptr) extra_->Release();
    --g_live;
}

cFC4Message2Standard* cFC4Message2Standard::New() { return new cFC4Message2Standard(); }

long cFC4Message2Standard::LiveCount() { return g_live.load(); }

cFC4Message2Standard* FC4MakeMessage(uint32_t type, intptr_t data1, intptr_t data2, intptr_t data3) {
    cFC4Message2Standard* m = cFC4Message2Standard::New();
    m->AddRef();
    m->SetType(type);
    m->SetData1(data1);
    m->SetData2(data2);
    m->SetData3(data3);
    return m;
}

// ---- cIGZUnknown -----------------------------------------------------------------

bool cFC4Message2Standard::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case GZIID_cIGZMessage2Standard:
            *ppvObj = static_cast<cIGZMessage2Standard*>(this);
            break;
        case kIID_cIGZMessage2:
            *ppvObj = static_cast<cIGZMessage2*>(this);
            break;
        case GZIID_cIGZUnknown:
            *ppvObj = static_cast<cIGZUnknown*>(this);
            break;
        default:
            *ppvObj = nullptr;
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4Message2Standard::AddRef() { return ++refCount_; }

uint32_t cFC4Message2Standard::Release() {
    // Destroyed when the count was 1 (or 0); otherwise decremented (§5.1).
    if (refCount_ <= 1) {
        delete this;
        return 0;
    }
    return --refCount_;
}

// ---- cIGZMessage2 -------------------------------------------------------------------

bool cFC4Message2Standard::Create(uint32_t dwTypeIID, void** ppUnknown) const {
    // A new standard message with the same type only (§5.1).
    cFC4Message2Standard* m = New();
    m->SetType(type_);
    if (m->QueryInterface(dwTypeIID, ppUnknown)) return true;
    delete m;
    return false;
}

uint32_t cFC4Message2Standard::GetType() const { return type_; }
void cFC4Message2Standard::SetType(uint32_t dwType) { type_ = dwType; }

bool cFC4Message2Standard::operator==(cIGZMessage2 const& o) const { return type_ == o.GetType(); }
bool cFC4Message2Standard::operator<(cIGZMessage2 const& o) const { return type_ < o.GetType(); }

// ---- data slots ------------------------------------------------------------------------

void cFC4Message2Standard::SetSlot(int i, intptr_t v) {
    data_[i] = v;
    hasData_ = static_cast<uint8_t>(hasData_ | (1u << i));
}

bool cFC4Message2Standard::GetHasData1() const { return (hasData_ & 1u) != 0; }
bool cFC4Message2Standard::GetHasData2() const { return (hasData_ & 2u) != 0; }
bool cFC4Message2Standard::GetHasData3() const { return (hasData_ & 4u) != 0; }
bool cFC4Message2Standard::GetHasData4() const { return (hasData_ & 8u) != 0; }
intptr_t cFC4Message2Standard::GetData1() const { return data_[0]; }
intptr_t cFC4Message2Standard::GetData2() const { return data_[1]; }
intptr_t cFC4Message2Standard::GetData3() const { return data_[2]; }
intptr_t cFC4Message2Standard::GetData4() const { return data_[3]; }
void* cFC4Message2Standard::GetVoid1() const { return reinterpret_cast<void*>(data_[0]); }
void* cFC4Message2Standard::GetVoid2() const { return reinterpret_cast<void*>(data_[1]); }
void* cFC4Message2Standard::GetVoid3() const { return reinterpret_cast<void*>(data_[2]); }
void* cFC4Message2Standard::GetVoid4() const { return reinterpret_cast<void*>(data_[3]); }
void cFC4Message2Standard::SetData1(intptr_t v) { SetSlot(0, v); }
void cFC4Message2Standard::SetData2(intptr_t v) { SetSlot(1, v); }
void cFC4Message2Standard::SetData3(intptr_t v) { SetSlot(2, v); }
void cFC4Message2Standard::SetData4(intptr_t v) { SetSlot(3, v); }
void cFC4Message2Standard::SetVoid1(void* p) { SetSlot(0, reinterpret_cast<intptr_t>(p)); }
void cFC4Message2Standard::SetVoid2(void* p) { SetSlot(1, reinterpret_cast<intptr_t>(p)); }
void cFC4Message2Standard::SetVoid3(void* p) { SetSlot(2, reinterpret_cast<intptr_t>(p)); }
void cFC4Message2Standard::SetVoid4(void* p) { SetSlot(3, reinterpret_cast<intptr_t>(p)); }

// ---- references --------------------------------------------------------------------------

bool cFC4Message2Standard::GetHasString() const { return string_ != nullptr; }

bool cFC4Message2Standard::GetString(uint32_t riid, void** ppStringOut) const {
    return string_ != nullptr && string_->QueryInterface(riid, ppStringOut);
}

void cFC4Message2Standard::SetString(cIGZString* pString) { Assign(string_, pString); }

bool cFC4Message2Standard::GetHasIGZUnknown() const { return unknown_ != nullptr; }
cIGZUnknown* cFC4Message2Standard::GetIGZUnknown() const { return unknown_; }
void cFC4Message2Standard::SetIGZUnknown(cIGZUnknown* pObj) { Assign(unknown_, pObj); }

bool cFC4Message2Standard::GetHasExtra() const { return extra_ != nullptr; }

uint32_t cFC4Message2Standard::GetExtra(uint32_t riid, void** ppOut) const {
    return (extra_ != nullptr && extra_->QueryInterface(riid, ppOut)) ? 1u : 0u;
}

void cFC4Message2Standard::SetExtra(cIGZSerializable* pSerializable) { Assign(extra_, pSerializable); }
