// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.
//
// cSC4AdviceSubject over the core subject (advisors.md §2.5, §11.4, §12.4).

#include "cFC4AdviceSubject.h"

#include "FC4Log.h"
#include "fc4.h"

#include <mutex>
#include <vector>

namespace {
std::mutex g_mu;
std::vector<cFC4AdviceSubject*> g_live;
}  // namespace

cFC4AdviceSubject::cFC4AdviceSubject(fc4_adv_obj* handle, NewSubjectFn newSubject, void* ctx)
    : handle_(handle), newSubject_(newSubject), ctx_(ctx) {
    std::lock_guard<std::mutex> lock(g_mu);
    g_live.push_back(this);
}

cFC4AdviceSubject::~cFC4AdviceSubject() {
    if (handle_ != nullptr) fc4_adv_obj_destroy(handle_);
    handle_ = nullptr;
}

size_t cFC4AdviceSubject::LiveCount() {
    std::lock_guard<std::mutex> lock(g_mu);
    return g_live.size();
}

void cFC4AdviceSubject::DestroyAll() {
    std::vector<cFC4AdviceSubject*> all;
    {
        std::lock_guard<std::mutex> lock(g_mu);
        all.swap(g_live);
    }
    if (!all.empty()) fc4::Log("advisors: %u advice subject wrapper(s) deleted at teardown", static_cast<unsigned>(all.size()));
    for (cFC4AdviceSubject* s : all) delete s;
}

bool cFC4AdviceSubject::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    *ppvObj = nullptr;
    if (riid == kFC4_IID_cISC4AdviceSubject || riid == 1 || riid == kFC4_CLSID_cSC4AdviceSubject) {
        *ppvObj = static_cast<cFC4ISC4AdviceSubject*>(this);
    } else if (riid == GZIID_cIGZSerializable) {
        *ppvObj = static_cast<cIGZSerializable*>(this);
    } else if (riid == kFC4_IID_CoreObject) {
        *ppvObj = static_cast<cIFC4CoreObject*>(this);
    } else {
        return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4AdviceSubject::AddRef() { return ++refCount_; }

// Never deletes (header: lifetime [choice]).
uint32_t cFC4AdviceSubject::Release() {
    if (refCount_ != 0) --refCount_;
    return refCount_;
}

bool cFC4AdviceSubject::Get(fc4_adv_subject_state* out) {
    *out = fc4_adv_subject_state{};
    return handle_ != nullptr && fc4_adv_subject_get(handle_, out);
}

// The fields go back with has_location cleared, so only data1, data2 and the
// object are written (fc4.h: SetLocation runs only when has_location).
bool cFC4AdviceSubject::Set(const fc4_adv_subject_state& v) {
    return handle_ != nullptr && fc4_adv_subject_set(handle_, &v);
}

bool cFC4AdviceSubject::SetLocation(float x, float z) {
    fc4_adv_subject_state st{};
    if (!Get(&st)) return false;
    st.x = x;
    st.z = z;
    st.has_location = 1;  // the core clamps to the city's cell counts (§2.5)
    Set(st);
    return true;
}

float cFC4AdviceSubject::GetLocationX() {
    fc4_adv_subject_state st{};
    return Get(&st) && st.has_location ? st.x : 0.0f;
}

float cFC4AdviceSubject::GetLocationZ() {
    fc4_adv_subject_state st{};
    return Get(&st) && st.has_location ? st.z : 0.0f;
}

bool cFC4AdviceSubject::HasLocation() {
    fc4_adv_subject_state st{};
    return Get(&st) && st.has_location != 0;
}

bool cFC4AdviceSubject::SetObject(cIGZUnknown* object) {
    fc4_adv_subject_state st{};
    if (!Get(&st)) return false;
    st.has_location = 0;
    st.object = object != nullptr ? fc4::TokenOf(object) : 0;  // the core add_refs it, releases the old one
    Set(st);
    return true;
}

cIGZUnknown* cFC4AdviceSubject::GetObjectA() {
    fc4_adv_subject_state st{};
    return Get(&st) ? fc4::FromToken(st.object) : nullptr;
}

uint32_t cFC4AdviceSubject::GetData1() {
    fc4_adv_subject_state st{};
    return Get(&st) ? st.data1 : 0;
}

bool cFC4AdviceSubject::SetData1(uint32_t v) {
    fc4_adv_subject_state st{};
    if (!Get(&st)) return true;
    st.has_location = 0;
    st.data1 = v;
    Set(st);
    return true;
}

uint32_t cFC4AdviceSubject::GetData2() {
    fc4_adv_subject_state st{};
    return Get(&st) ? st.data2 : 0;
}

bool cFC4AdviceSubject::SetData2(uint32_t v) {
    fc4_adv_subject_state st{};
    if (!Get(&st)) return true;
    st.has_location = 0;
    st.data2 = v;
    Set(st);
    return true;
}

// Slot 13: a new subject with the same fields, the object held again, AddRef'd
// for the caller.
bool cFC4AdviceSubject::Clone(cFC4ISC4AdviceSubject** out) {
    if (out == nullptr) return false;
    *out = nullptr;
    fc4_adv_subject_state st{};
    if (newSubject_ == nullptr || !Get(&st)) return false;
    fc4_adv_obj* h = newSubject_(ctx_);
    if (h == nullptr) return false;
    fc4_adv_subject_set(h, &st);  // location already clamped; the object add_ref'd
    const uint64_t token = fc4_adv_obj_token(h);
    fc4_adv_obj_destroy(h);
    if (token == 0) return false;
    auto* c = static_cast<cFC4ISC4AdviceSubject*>(fc4::FromToken(token));
    c->AddRef();
    *out = c;
    return true;
}

bool cFC4AdviceSubject::Write(cIGZOStream& out) {
    fc4::OStreamAdapter a(out);
    return handle_ != nullptr && fc4_adv_obj_write(handle_, a.Get());
}

bool cFC4AdviceSubject::Read(cIGZIStream& in) {
    fc4::IStreamAdapter a(in);
    return handle_ != nullptr && fc4_adv_obj_read(handle_, a.Get());
}
