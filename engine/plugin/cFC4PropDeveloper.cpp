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

#include "cFC4PropDeveloper.h"

#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Seed.h"
#include "FC4Streams.h"

#include "cIGZMessage2.h"
#include "cIGZSerializable.h"

#include <algorithm>
#include <atomic>
#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cIGZSerializable = 0xE4FDA3D4;
constexpr uint32_t kMsgIdle = 0x6695681A;
constexpr uint32_t kMsgDay = 0x66956814;
constexpr uint32_t kMsgAnimationTick = 0x86AD10EE;

// The developer whose month line is written (the latest one made).
std::atomic<cFC4PropDeveloper*> g_live{nullptr};

// Our wealth requester behind a requester pointer, or null (the original's
// class, or another requester).
cFC4WealthRequester* OurWealth(cIGZUnknown* u) {
    if (u == nullptr || !fc4::IsOurObject(u)) return nullptr;
    return static_cast<cFC4WealthRequester*>(reinterpret_cast<cFC4ISC4PropRequester*>(u));
}

}  // namespace

// ---- the wealth requester (§1, §4.1, §8.3) ------------------------------------------------------------

cIGZUnknown* CreateFC4WealthRequester() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4PropRequester*>(new cFC4WealthRequester()));
}

// §1: 1 and 0x89C498BE → +0, 0xE4FDA3D4 → +4.
bool cFC4WealthRequester::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case GZIID_cIGZUnknown:
        case kFC4_IID_cISC4PropRequesterPD:
            *ppvObj = static_cast<cFC4ISC4PropRequester*>(this);
            break;
        case kIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        default:
            return false;
    }
    AddRef();
    return true;
}

// §1: the last Release deletes it.
uint32_t cFC4WealthRequester::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    delete this;
    return 0;
}

int32_t cFC4WealthRequester::GetRequestedShare(SC4Rect<int32_t> const&) { return 10; }  // §8.3 slot 3
uint32_t cFC4WealthRequester::GetRequesterSatisfactionId() { return 0x49A1E995; }       // slot 4
bool cFC4WealthRequester::UsesCustomPropSelector() { return true; }                     // slot 5

// Slot 6: the developer's core draws (§4.1); 0 without a developer.
uint32_t cFC4WealthRequester::SelectProp(fc4::GameVector<uint32_t> const& candidates, int32_t x, int32_t z) {
    if (owner_ == nullptr) return 0;
    auto& v = const_cast<fc4::GameVector<uint32_t>&>(candidates);  // read only (GameVector has no const begin)
    const std::vector<uint32_t> ids(v.begin(), v.end());
    return owner_->WealthSelect(ids.data(), ids.size(), x, z);
}

// §8.3: Write and Read transfer nothing and return true.
bool cFC4WealthRequester::Write(cIGZOStream&) { return true; }
bool cFC4WealthRequester::Read(cIGZIStream&) { return true; }

// ---- the developer ------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4PropDeveloper() {
    return static_cast<cIGZUnknown*>(static_cast<cISC4PropDeveloper*>(new cFC4PropDeveloper()));
}

cFC4PropDeveloper::cFC4PropDeveloper() {
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    ctx_.owner = this;
    host_ = fc4::MakePropHost(&ctx_);
    bool fixed = false;
    const char* source = nullptr;
    seed_ = fc4::SubsystemSeed("propdev_seed", "FC4_PROPDEV_SEED", &fixed, &source);
    core_ = fc4_prop_developer_create(&host_, seed_, TokenOf(static_cast<cISC4PropDeveloper*>(this)));
    if (core_ == nullptr) fc4::Log("cFC4PropDeveloper: core creation failed");
    g_live.store(this);
    fc4::Log("cFC4PropDeveloper(%p) created, core %p, session seed %u (%s)", static_cast<void*>(this),
             static_cast<void*>(core_), seed_, source ? source : (fixed ? "propdev_seed" : "session seed"));
}

// [choice] The object's lists and registrations go with it: Shutdown's
// release (a no-op after the city's Shutdown), then the host's references.
cFC4PropDeveloper::~cFC4PropDeveloper() {
    cFC4PropDeveloper* self = this;
    g_live.compare_exchange_strong(self, nullptr);
    if (core_ != nullptr) fc4_prop_developer_shutdown(core_);
    for (cIGZUnknown* w : ctx_.wealth) {
        if (cFC4WealthRequester* r = OurWealth(w)) r->SetOwner(nullptr);
    }
    fc4::ReleasePropContext(&ctx_);
    fc4_prop_developer_destroy(core_);
    core_ = nullptr;
    fc4::Log("cFC4PropDeveloper destroyed (%u idle, %u day, %u ticks, %u messages)", idle_, day_, ticks_, messages_);
}

// §1: 0xE9A1F4B6 and 1 → +0; 0x452294AA → +4; 0xE4FDA3D4 → +8.
bool cFC4PropDeveloper::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case GZIID_cIGZUnknown:
        case kFC4_IID_cISC4PropDeveloper:
            *ppvObj = static_cast<cISC4PropDeveloper*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case kIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        default:
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4PropDeveloper::AddRef() { return core_ ? fc4_prop_developer_add_ref(core_) : 1; }

// The last Release runs the deleting destructor.
uint32_t cFC4PropDeveloper::Release() {
    if (destroying_) return 0;
    const uint32_t left = core_ ? fc4_prop_developer_release(core_) : 0;
    if (left != 0) return left;
    destroying_ = true;
    delete this;
    return 0;
}

void cFC4PropDeveloper::LogState(const char* what, bool ok) {
    fc4_prop_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_prop_developer_get_state(core_, &st)) {
        fc4::Log("propdev %s: ok=%d (no state)", what, ok ? 1 : 0);
        return;
    }
    fc4::Log("propdev %s: ok=%d scan=%d scanner=(%u %d,%d step %d clip %d,%d-%d,%d) wealthAdded=%u requesters=%u "
             "night=%u (cursor %u mode %u batch %u acc %u isNight %u) triggers=%u tod=%u dated=%u power=%u "
             "signs=%u visible=%u wealth-objects=%u agent=%d tick=%d subscriptions=%u",
             what, ok ? 1 : 0, st.scan_flag, st.scanner_state, st.scanner_x, st.scanner_z, st.scanner_step,
             st.clip_x0, st.clip_z0, st.clip_x1, st.clip_z1, st.wealth_added, st.requester_count, st.night_count,
             st.night_cursor, st.night_mode, st.night_batch, st.night_acc, st.is_night, st.trigger_count,
             st.time_of_day_count, st.dated_count, st.power_count, st.sign_count, st.signs_visible,
             static_cast<unsigned>(ctx_.wealth.size()), ctx_.agentRegistered ? 1 : 0, ctx_.tickRegistered ? 1 : 0,
             static_cast<unsigned>(ctx_.subscriptions.size()));
}

// Our wealth requesters answer through this developer.
void cFC4PropDeveloper::AdoptWealthOwners() {
    for (cIGZUnknown* w : ctx_.wealth) {
        if (cFC4WealthRequester* r = OurWealth(w)) r->SetOwner(this);
    }
}

// ---- lifecycle (§3.1, §3.2) ---------------------------------------------------------------------------

bool cFC4PropDeveloper::Init() {
    FC4_TRACE("PropDeveloper::Init");
    if (core_ != nullptr) fc4_prop_developer_init(core_);
    ctx_.pool.Drain();
    AdoptWealthOwners();
    LogState("Init", true);
    return true;  // §3.1
}

bool cFC4PropDeveloper::Shutdown() {
    FC4_TRACE("PropDeveloper::Shutdown");
    if (core_ != nullptr) fc4_prop_developer_shutdown(core_);
    ctx_.pool.Drain();
    LogState("Shutdown", true);
    // The managers and the message server; the wealth requesters once the
    // core has let them go (§3.2: wealthAdded cleared).
    fc4_prop_state st{};
    st.struct_size = sizeof(st);
    const bool keepWealth = core_ != nullptr && fc4_prop_developer_get_state(core_, &st) && st.wealth_added != 0;
    std::vector<cIGZUnknown*> wealth;
    if (keepWealth) wealth.swap(ctx_.wealth);
    for (cIGZUnknown* w : ctx_.wealth) {
        if (cFC4WealthRequester* r = OurWealth(w)) r->SetOwner(nullptr);
    }
    fc4::ReleasePropContext(&ctx_);
    if (keepWealth) ctx_.wealth.swap(wealth);
    return true;  // §3.2
}

// ---- requesters (§4.1) ----------------------------------------------------------------------------------

void cFC4PropDeveloper::AddPropRequester(cISC4PropRequester* r) {
    if (core_ == nullptr) return;
    cIGZUnknown* u = reinterpret_cast<cIGZUnknown*>(r);
    uint32_t cls = 0;
    if (u != nullptr) {
        auto* s = static_cast<cIGZSerializable*>(fc4::propcity::QueryInto(ctx_.pool, u, kIID_cIGZSerializable));
        if (s != nullptr) cls = s->GetGZCLSID();
    }
    if (u != nullptr && cls == kFC4_CLSID_cSC4WealthRequester) {
        // The core keeps no reference to a wealth requester: the host does.
        fc4::propcity::Hold(u);
        ctx_.wealth.push_back(u);
        if (cFC4WealthRequester* w = OurWealth(u)) w->SetOwner(this);
    }
    fc4_prop_developer_add_prop_requester(core_, TokenOf(u), cls);
    ctx_.pool.Drain();
    fc4::Log("propdev AddPropRequester(%p, class 0x%08X)", static_cast<void*>(u), cls);
}

void cFC4PropDeveloper::RemovePropRequester(cISC4PropRequester* r) {
    if (core_ == nullptr) return;
    const uint64_t token = TokenOf(r);
    // The class of the first entry with this token (a wealth requester's
    // reference is the host's).
    const size_t n = fc4_prop_developer_list(core_, FC4_PROP_LIST_REQUESTERS, nullptr, 0);
    std::vector<fc4_prop_ref> reqs(n);
    if (n != 0) fc4_prop_developer_list(core_, FC4_PROP_LIST_REQUESTERS, reqs.data(), n);
    uint32_t cls = 0;
    for (const fc4_prop_ref& q : reqs) {
        if (q.token == token) {
            cls = q.class_id;
            break;
        }
    }
    const bool removed = fc4_prop_developer_remove_prop_requester(core_, token);
    if (removed && cls == kFC4_CLSID_cSC4WealthRequester) {
        auto it = std::find(ctx_.wealth.begin(), ctx_.wealth.end(), FromToken(token));
        if (it != ctx_.wealth.end()) {
            cIGZUnknown* w = *it;
            ctx_.wealth.erase(it);
            if (std::find(ctx_.wealth.begin(), ctx_.wealth.end(), w) == ctx_.wealth.end()) {
                if (cFC4WealthRequester* o = OurWealth(w)) o->SetOwner(nullptr);
            }
            fc4::propcity::Drop(w);
        }
    }
    ctx_.pool.Drain();
    fc4::Log("propdev RemovePropRequester(%p) -> %d", static_cast<void*>(FromToken(token)), removed ? 1 : 0);
}

void cFC4PropDeveloper::UpdateAllOptionalProps() {
    if (core_ != nullptr) fc4_prop_developer_update_all(core_);
    ctx_.pool.Drain();
}

void cFC4PropDeveloper::ShowSignsAndLabels(bool show) {
    if (core_ != nullptr) fc4_prop_developer_show_signs(core_, show);
    ctx_.pool.Drain();
}

void cFC4PropDeveloper::ToggleSignsAndLabels() {
    if (core_ != nullptr) fc4_prop_developer_toggle_signs(core_);
    ctx_.pool.Drain();
}

bool cFC4PropDeveloper::AreSignsAndLabelsVisible() { return core_ != nullptr && fc4_prop_developer_signs_visible(core_); }

uint32_t cFC4PropDeveloper::WealthSelect(const uint32_t* ids, size_t count, int32_t x, int32_t z) {
    if (core_ == nullptr) return 0;
    const uint32_t id = fc4_prop_developer_wealth_select(core_, ids, count, x, z);
    ctx_.pool.Drain();
    return id;
}

// ---- messages (§3.3) ----------------------------------------------------------------------------------

bool cFC4PropDeveloper::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    const uint32_t type = pMessage->GetType();
    FC4_X87_SCOPE("propdev DoMessage", type);  // x87 leftover guard (FC4Fpu.h)
    fc4_prop_message m{};
    if (!fc4::PropMessageOf(pMessage, ctx_.pool, &m)) {
        ctx_.pool.Drain();
        return true;
    }
    if (type == kMsgIdle) {
        ++idle_;
    } else if (type == kMsgDay) {
        ++day_;
    } else if (type == kMsgAnimationTick) {
        ++ticks_;
    } else {
        ++messages_;
    }
    const bool r = fc4_prop_developer_do_message(core_, &m);
    ctx_.pool.Drain();
    return r;
}

// ---- persistence (§7) ---------------------------------------------------------------------------------

bool cFC4PropDeveloper::Write(cIGZOStream& s) {
    FC4_TRACE("PropDeveloper::Write");
    fc4::OStreamAdapter a(s);
    const bool ok = core_ != nullptr && fc4_prop_developer_write(core_, a.Get());
    LogState(ok ? "Write -> 1" : "Write -> 0", ok);
    return ok;
}

bool cFC4PropDeveloper::Read(cIGZIStream& s) {
    FC4_TRACE("PropDeveloper::Read");
    fc4::IStreamAdapter a(s);
    fc4::propcity::RecordingIStream rec(a.Get(), false);
    const bool ok = core_ != nullptr && fc4_prop_developer_read(core_, rec.Get());
    fc4::AfterPropRead(&ctx_, core_, ok, rec.Refs());
    AdoptWealthOwners();
    LogState("Read", ok);
    return ok;
}

uint32_t cFC4PropDeveloper::GetGZCLSID() { return kFC4_CLSID_cSC4PropDeveloper; }

// ---- the month line -----------------------------------------------------------------------------------

void cFC4PropDeveloper::MonthLine(int32_t julian) {
    cFC4PropDeveloper* d = g_live.load();
    if (d == nullptr || d->core_ == nullptr) return;
    static uint32_t lastIdle = 0, lastDay = 0, lastRemoves = 0, lastCreates = 0, lastPosts = 0, lastTicks = 0;
    static const cFC4PropDeveloper* lastOwner = nullptr;
    if (lastOwner != d) {  // a new city's developer: its counters start at 0
        lastOwner = d;
        lastIdle = lastDay = lastRemoves = lastCreates = lastPosts = lastTicks = 0;
    }
    fc4_prop_state st{};
    st.struct_size = sizeof(st);
    fc4_prop_developer_get_state(d->core_, &st);
    char date[16];
    fc4::Log("propdev month %s: blocks=%u (idle %u, day %u) removed=%u created=%u posts=%u scanner=(%d,%d) "
             "requesters=%u night=%u tod=%u dated=%u power=%u signs=%u isNight=%u ticks=%u",
             fc4::propcity::DateText(julian, date), (d->idle_ - lastIdle) + (d->day_ - lastDay), d->idle_ - lastIdle,
             d->day_ - lastDay, d->ctx_.removes - lastRemoves, d->ctx_.creates - lastCreates,
             d->ctx_.posts - lastPosts, st.scanner_x, st.scanner_z, st.requester_count, st.night_count,
             st.time_of_day_count, st.dated_count, st.power_count, st.sign_count, st.is_night, d->ticks_ - lastTicks);
    lastIdle = d->idle_;
    lastDay = d->day_;
    lastRemoves = d->ctx_.removes;
    lastCreates = d->ctx_.creates;
    lastPosts = d->ctx_.posts;
    lastTicks = d->ticks_;
}
