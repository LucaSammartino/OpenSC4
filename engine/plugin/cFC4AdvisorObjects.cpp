// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.
//
// cSC4Advisor / cSC4Advice fronts over the core (advisors.md §12.2, §12.3).

#include "cFC4AdvisorObjects.h"

#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cFC4AdviceSubject.h"
#include "cIGZMessage2Standard.h"
#include "fc4.h"

#include <vector>

namespace {

// A front for a new core handle (from a per-object call), not AddRef'd; the
// handle is destroyed (the front keeps its own weak one).
cFC4ISC4Advice* AdviceOf(fc4_adv_obj* h) {
    if (h == nullptr) return nullptr;
    const uint64_t t = fc4_adv_obj_token(h);
    fc4_adv_obj_destroy(h);  // the object stays listed in its advisor, so it lives on
    return FC4AdviceFront(t);
}

fc4_adv_obj* HandleOf(cIGZUnknown* p) {
    if (p == nullptr) return nullptr;
    cIFC4CoreObject* c = nullptr;
    if (!p->QueryInterface(kFC4_IID_CoreObject, reinterpret_cast<void**>(&c)) || c == nullptr) return nullptr;
    FC4_ACQUIRED(c, "QueryInterface");
    void* h = c->CoreHandle();
    FC4_RELEASE(c);
    return static_cast<fc4_adv_obj*>(h);
}

void Fill(cRZBaseString& s, const char* p, size_t n) { s.FromChar(p, static_cast<uint32_t>(n)); }

}  // namespace

cFC4ISC4Advice* FC4AdviceFront(uint64_t token) {
    cIGZUnknown* u = fc4::FromToken(token);
    if (u == nullptr) return nullptr;
    void* out = nullptr;
    if (!u->QueryInterface(kFC4_IID_cISC4Advice, &out) || out == nullptr) return nullptr;
    auto* a = static_cast<cFC4ISC4Advice*>(out);
    FC4_ACQUIRED(a, "QueryInterface");
    FC4_RELEASE(a);  // borrowed: the core's listing keeps it alive
    return a;
}

// ---- cFC4Advice (§12.3) ------------------------------------------------------------------------------

cFC4Advice::cFC4Advice(fc4_adv_obj* handle) : fc4::AdvFront(handle) { Register(Token()); }

bool cFC4Advice::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    *ppvObj = nullptr;
    if (riid == kFC4_IID_cISC4Advice || riid == 1 || riid == GetGZCLSID()) *ppvObj = static_cast<cFC4ISC4Advice*>(this);
    else if (riid == GZIID_cIGZSerializable) *ppvObj = static_cast<cIGZSerializable*>(this);
    else if (riid == kFC4_IID_CoreObject) *ppvObj = static_cast<cIFC4CoreObject*>(this);
    else return false;
    AddRef();
    return true;
}

bool cFC4Advice::State(fc4_advice_state* out) {
    *out = fc4_advice_state{};
    Pin h(*this);
    return h && fc4_adv_advice_get(h.get(), out);
}

#define FC4_ADVICE_FIELD(type, name, field)  \
    type cFC4Advice::name() {                \
        fc4_advice_state s{};                \
        return State(&s) ? static_cast<type>(s.field) : type{}; \
    }
FC4_ADVICE_FIELD(int32_t, GetMood, mood)
FC4_ADVICE_FIELD(int32_t, GetPriority, priority)
FC4_ADVICE_FIELD(uint32_t, GetGUID, guid)
FC4_ADVICE_FIELD(int32_t, GetAdvisorID, advisor_id)
FC4_ADVICE_FIELD(int32_t, GetSessionID, serial)  // the session ID is the serial (core [choice])
FC4_ADVICE_FIELD(int32_t, GetFrequencyInDays, frequency)
FC4_ADVICE_FIELD(int32_t, GetTimeOutInDays, timeout)
FC4_ADVICE_FIELD(int32_t, GetEventID, event_id)
FC4_ADVICE_FIELD(int32_t, GetCommandID, command_id)
#undef FC4_ADVICE_FIELD

bool cFC4Advice::Init(int32_t i) {
    Pin h(*this);
    return h && fc4_adv_advice_init(h.get(), i);
}
bool cFC4Advice::Shutdown() {
    Pin h(*this);
    return h && fc4_adv_advice_shutdown(h.get());
}
// Slot 5: the copy's front (object_created), AddRef'd for the caller before
// the temporary handle goes (it may be the copy's only strong one).
bool cFC4Advice::Clone(cFC4ISC4Advice** out) {
    if (out == nullptr) return false;
    *out = nullptr;
    Pin h(*this);
    fc4_adv_obj* c = h ? fc4_adv_advice_clone(h.get()) : nullptr;
    if (c == nullptr) return false;
    cIGZUnknown* u = fc4::FromToken(fc4_adv_obj_token(c));
    void* p = nullptr;
    const bool ok = u != nullptr && u->QueryInterface(kFC4_IID_cISC4Advice, &p) && p != nullptr;
    fc4_adv_obj_destroy(c);
    if (!ok) return false;
    *out = static_cast<cFC4ISC4Advice*>(p);  // the QueryInterface reference is the caller's
    return true;
}
bool cFC4Advice::GetTrigger() {
    Pin h(*this);
    return h && fc4_adv_advice_get_trigger(h.get());
}
cIGZString* cFC4Advice::GetTriggerString() {
    Pin h(*this);
    char buf[1024];
    const size_t n = h ? fc4_adv_advice_trigger_text(h.get(), buf, sizeof buf) : 0;
    Fill(trigger_, buf, n < sizeof buf ? n : sizeof buf - 1);
    return &trigger_;
}
cFC4ISC4Advice* cFC4Advice::SetAdvisorID(int32_t id) {
    Pin h(*this);
    if (h) fc4_adv_advice_set_advisor_id(h.get(), id);
    return this;  // §12.3: the original leaves the argument in EAX; a replacement returns this
}
bool cFC4Advice::SetFlags(uint32_t mask, bool on) {
    Pin h(*this);
    if (h) fc4_adv_advice_set_flags(h.get(), mask, on);
    return true;  // §6.9: always true
}
bool cFC4Advice::CheckFlags(uint32_t mask) {
    fc4_advice_state s{};
    return State(&s) && (s.flags & mask) == mask;  // all bits of mask
}
// [question AD12] the title / message strings and their GUIDs (slots 18-21)
// are filled by the text rebuild (§6.9, UI only); the spec does not give the
// GUIDs' source, so they read 0 until it does.
cIGZString* cFC4Advice::GetTextTitle() { return &title_; }
uint32_t cFC4Advice::GetTextTitleGUID() { return 0; }
cIGZString* cFC4Advice::GetTextMessage() { return &message_; }
uint32_t cFC4Advice::GetTextMessageGUID() { return 0; }
// [question AD13] slot 22's behaviour is not in the spec (only its row in
// §12.3); false (nothing done) until it is.
bool cFC4Advice::DoLink(const cIGZString&) { return false; }
void cFC4Advice::NotifyGlobalEffectTriggerChanged() {
    Pin h(*this);
    if (h) fc4_adv_advice_notify_global_effects(h.get());
}
bool cFC4Advice::SetEventSubject(cISC4AdviceSubject* s) {
    Pin h(*this);
    if (h) fc4_adv_advice_set_event_subject(h.get(), HandleOf(s));
    return true;  // §12.3: AL true
}
cISC4AdviceSubject* cFC4Advice::GetEventSubject() {
    Pin h(*this);
    fc4_adv_obj* s = h ? fc4_adv_advice_event_subject(h.get()) : nullptr;
    if (s == nullptr) return nullptr;
    const uint64_t t = fc4_adv_obj_token(s);
    fc4_adv_obj_destroy(s);  // the advice keeps the subject
    cIGZUnknown* u = fc4::FromToken(t);
    // not AddRef'd (the original returns its member); the token is the
    // front's cFC4ISC4AdviceSubject base (cFC4AdviceSubject::Token).
    return u != nullptr ? static_cast<cFC4ISC4AdviceSubject*>(u) : nullptr;
}
bool cFC4Advice::RebuildText() {
    Pin h(*this);
    return h && fc4_adv_advice_update_text(h.get());
}
void cFC4Advice::SetText(const char* title, size_t tn, const char* message, size_t mn) {
    Fill(title_, title, tn);
    Fill(message_, message, mn);
}
bool cFC4Advice::Write(cIGZOStream& out) {
    fc4::OStreamAdapter a(out);
    Pin h(*this);
    return h && fc4_adv_obj_write(h.get(), a.Get());
}
bool cFC4Advice::Read(cIGZIStream& in) {
    fc4::IStreamAdapter a(in);
    Pin h(*this);
    return h && fc4_adv_obj_read(h.get(), a.Get());
}
uint32_t cFC4Advice::GetGZCLSID() {
    Pin h(*this);
    return h ? fc4_adv_obj_class(h.get()) : 0x8A09F5F4;
}

// ---- cFC4Advisor (§12.2) -----------------------------------------------------------------------------

cFC4Advisor::cFC4Advisor(fc4_adv_obj* handle) : fc4::AdvFront(handle) { Register(Token()); }

bool cFC4Advisor::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    *ppvObj = nullptr;
    if (riid == kFC4_IID_cISC4Advisor || riid == 1 || riid == GetGZCLSID()) *ppvObj = static_cast<cFC4ISC4Advisor*>(this);
    else if (riid == GZIID_cIGZSerializable) *ppvObj = static_cast<cIGZSerializable*>(this);
    else if (riid == kFC4_IID_cIGZMessageTarget2) *ppvObj = static_cast<cIGZMessageTarget2*>(this);
    else if (riid == kFC4_IID_CoreObject) *ppvObj = static_cast<cIFC4CoreObject*>(this);
    else return false;  // [question AD14] cISC4MySim 0x0A1DA42F is the MySim spec's (not answered here)
    AddRef();
    return true;
}

bool cFC4Advisor::Init(int32_t i) {
    Pin h(*this);
    return h && fc4_adv_advisor_init(h.get(), i);
}
bool cFC4Advisor::Shutdown() {
    active_.Clear();
    Pin h(*this);
    return h && fc4_adv_advisor_shutdown(h.get());
}
int32_t cFC4Advisor::GetID() {
    Pin h(*this);
    return h ? fc4_adv_advisor_id(h.get()) : 0;
}
static cIGZString* TextOf(fc4::AdvFront::Pin& h, uint32_t which, cRZBaseString& s) {
    char buf[512];
    const size_t n = h ? fc4_adv_advisor_text(h.get(), which, buf, sizeof buf) : 0;
    s.FromChar(buf, static_cast<uint32_t>(n < sizeof buf ? n : sizeof buf - 1));
    return &s;
}
cIGZString* cFC4Advisor::GetName() {
    Pin h(*this);
    return TextOf(h, 0, name_);
}
cIGZString* cFC4Advisor::GetPersonalName() {
    Pin h(*this);
    return TextOf(h, 1, personal_);
}
bool cFC4Advisor::SetName(const cIGZString& s) {
    Pin h(*this);
    return h && fc4_adv_advisor_set_text(h.get(), 0, reinterpret_cast<const uint8_t*>(s.ToChar()), s.Strlen());
}
bool cFC4Advisor::SetPersonalName(const cIGZString& s) {
    Pin h(*this);
    return h && fc4_adv_advisor_set_text(h.get(), 1, reinterpret_cast<const uint8_t*>(s.ToChar()), s.Strlen());
}
// Slots 10/11: the active map rebuilt in the game's layout (FC4AdvisorMaps.h),
// keyed by the advice fronts. [choice] rebuilt on each call (the four
// consumers read it right after the call, on the simulation thread).
void* cFC4Advisor::GetCurrentAdvices() {
    std::vector<fc4::AdvisorMapEntry> e;
    {
        Pin h(*this);
        for (uint32_t i = 0; h; ++i) {
            int32_t value = 0;
            fc4_adv_obj* a = fc4_adv_advisor_current_advice(h.get(), i, &value);
            if (a == nullptr) break;
            cFC4ISC4Advice* f = AdviceOf(a);
            if (f == nullptr) continue;
            e.push_back({static_cast<uint32_t>(reinterpret_cast<uintptr_t>(f)), static_cast<uint32_t>(value), f});
        }
    }
    if (!active_.Rebuild(std::move(e))) fc4::Log("advisors: active map of advisor %d not rebuilt", GetID());
    return active_.Object();
}
cFC4ISC4Advice* cFC4Advisor::GetAdviceByGUID(uint32_t guid) {
    Pin h(*this);
    return h ? AdviceOf(fc4_adv_advisor_advice_by_guid(h.get(), guid)) : nullptr;  // not AddRef'd
}
cFC4ISC4Advice* cFC4Advisor::GetAdviceBySessionID(int32_t id) {
    Pin h(*this);
    return h ? AdviceOf(fc4_adv_advisor_advice_by_session(h.get(), id)) : nullptr;
}
bool cFC4Advisor::Simulate(int32_t kind) {
    Pin h(*this);
    if (h) fc4_adv_advisor_simulate(h.get(), kind);
    return true;  // §12.2: AL true
}
// [question AD15] slots 15-17 (head animations) are UI (§3.2) with no
// behaviour in the spec: no animations (0 / false) until it gives one.
int32_t cFC4Advisor::GetHeadAnimCount(bool) { return 0; }
bool cFC4Advisor::SetHeadAnim(bool, int32_t) { return false; }
int32_t cFC4Advisor::GetHeadAnimID(int32_t) { return 0; }
bool cFC4Advisor::PushEvent(int32_t event, cISC4AdviceSubject* s) {
    Pin h(*this);
    return h && fc4_adv_advisor_push_event(h.get(), static_cast<uint32_t>(event), HandleOf(s));
}
bool cFC4Advisor::CommandWouldTrigger(uint32_t command) {
    Pin h(*this);
    return h && fc4_adv_advisor_command_would_trigger(h.get(), command);
}
bool cFC4Advisor::RetireAdvice(cFC4ISC4Advice* advice) {
    Pin h(*this);
    return h && fc4_adv_advisor_retire_advice(h.get(), HandleOf(advice));
}
bool cFC4Advisor::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr) return true;
    fc4_adv_message m{};
    m.msg_type = pMessage->GetType();
    cIGZMessage2Standard* std = nullptr;
    if (pMessage->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&std)) && std != nullptr) {
        FC4_ACQUIRED(std, "QueryInterface");
        m.data1 = static_cast<uint32_t>(std->GetData1());
        m.data2 = static_cast<uint32_t>(std->GetData2());
        m.data3 = static_cast<uint32_t>(std->GetData3());
        m.data4 = static_cast<uint32_t>(std->GetData4());
        m.void1 = fc4::TokenOf(std->GetVoid1());
        FC4_RELEASE(std);
    }
    Pin h(*this);
    return h ? fc4_adv_advisor_do_message(h.get(), &m) : true;
}
bool cFC4Advisor::Write(cIGZOStream& out) {
    fc4::OStreamAdapter a(out);
    Pin h(*this);
    return h && fc4_adv_obj_write(h.get(), a.Get());
}
bool cFC4Advisor::Read(cIGZIStream& in) {
    fc4::IStreamAdapter a(in);
    Pin h(*this);
    return h && fc4_adv_obj_read(h.get(), a.Get());
}
uint32_t cFC4Advisor::GetGZCLSID() {
    Pin h(*this);
    return h ? fc4_adv_obj_class(h.get()) : 0xEAC937B4;
}
