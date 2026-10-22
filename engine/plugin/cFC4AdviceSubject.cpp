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


cFC4AdviceSubject::cFC4AdviceSubject(fc4_adv_obj* handle, NewSubjectFn newSubject, void* ctx)
    : fc4::AdvFront(handle), newSubject_(newSubject), ctx_(ctx) {
    Register(Token());
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

uint32_t cFC4AdviceSubject::AddRef() { return AddRefFront(); }

// May delete this (FC4AdvLife.h).
uint32_t cFC4AdviceSubject::Release() { return ReleaseFront(); }

bool cFC4AdviceSubject::Get(fc4_adv_subject_state* out) {
    *out = fc4_adv_subject_state{};
    Pin h(*this);
    return h && fc4_adv_subject_get(h.get(), out);
}

// The fields go back with has_location cleared, so only data1, data2 and the
// object are written (fc4.h: SetLocation runs only when has_location).
bool cFC4AdviceSubject::Set(const fc4_adv_subject_state& v) {
    Pin h(*this);
    return h && fc4_adv_subject_set(h.get(), &v);
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
    auto* c = token != 0 ? static_cast<cFC4ISC4AdviceSubject*>(fc4::FromToken(token)) : nullptr;
    if (c != nullptr) c->AddRef();  // before h goes: h may be the copy's only strong handle
    fc4_adv_obj_destroy(h);
    if (c == nullptr) return false;
    *out = c;
    return true;
}

bool cFC4AdviceSubject::Write(cIGZOStream& out) {
    fc4::OStreamAdapter a(out);
    Pin h(*this);
    return h && fc4_adv_obj_write(h.get(), a.Get());
}

bool cFC4AdviceSubject::Read(cIGZIStream& in) {
    fc4::IStreamAdapter a(in);
    Pin h(*this);
    return h && fc4_adv_obj_read(h.get(), a.Get());
}
