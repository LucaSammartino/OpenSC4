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

// The prop developer (cSC4PropDeveloper, CLSID 0x89C48F47, city component
// 10) and its wealth requester (cSC4WealthRequester, class 0x89C49986):
// docs/specs/prop-developer.md. The behaviour lives in the Rust core behind
// fc4.h (fc4_prop_developer); these classes implement the interfaces (§1,
// §8: cISC4PropDeveloper at +0, cIGZMessageTarget2 at +4, cIGZSerializable
// at +8; the requester's cISC4PropRequester at +0 and cIGZSerializable at
// +4) and the host callbacks over the game's services (FC4PropDevHost.cpp).
//
// References (fc4.h): the core holds requesters and list props through the
// host's hold / release; it does NOT release the three managers its Read
// takes, the sign list's references, nor any wealth requester token. So the
// host releases the managers and the signs right after Read (the managers
// are re-acquired at Init, §7 [choice]; the sign list is not held, §2), and
// owns every wealth requester object it creates or its Read returns,
// releasing them when the developer goes.
//
// The member generator (§5) is seeded from the session seed S
// (`propdev_seed` / FC4_PROPDEV_SEED, else session_seed / the clock,
// FC4Seed.h): the core derives seeding::prop_developer_seed(S); the wealth
// requester's file generator is seeding::keys::WEALTH_REQUESTER of S.
//
// OFF by default: FreeCity4.ini `enable = propdev` (FC4Subsystems.h).
//
// Log lines (FreeCity4.log):
//   propdev Init|Read|Shutdown: ok=.. scan=.. scanner=(..) requesters=.. night=.. tod=.. dated=.. power=.. ...
//   propdev Write -> 0|1 (..)
//   propdev month YYYY-MM-DD: blocks=.. (idle .., day ..) removed=.. created=.. ...
#pragma once

#include "FC4Interfaces.h"
#include "FC4PropCityCommon.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4PropDeveloper.h"
#include "fc4.h"

#include <cstdint>
#include <vector>

class cIGZMessage2;
class cIGZMessageServer2;
class cISC4City;

static const uint32_t kFC4_CLSID_cSC4PropDeveloper = 0x89C48F47;
static const uint32_t kFC4_IID_cISC4PropDeveloper = 0xE9A1F4B6;
static const uint32_t kFC4_CLSID_cSC4WealthRequester = 0x89C49986;
static const uint32_t kFC4_IID_cISC4PropRequesterPD = 0x89C498BE;  // cISC4PropRequester (also in cFC4Pollution.h)
static const uint32_t kFC4_IID_cISC4PropOccupant = 0xE9793A65;

class cFC4PropDeveloper;

namespace fc4 {

// What the prop host callbacks work on. The developer owns one; the prop
// developer shadow (FC4PropDevShadow.h) makes a read-only one (no writes to
// the game) whose recording hooks are set.
struct PropDevContext {
    bool readOnly = false;
    // PV15: an unavailable manager is not an observed daytime input.
    bool lightingAvailable = false;
    cIGZMessageTarget2* target = nullptr;  // the developer's +4 (agents, notifications, ticks)
    cFC4PropDeveloper* owner = nullptr;    // the wealth requesters' developer (null in the shadow)
    bool agentRegistered = false;
    bool tickRegistered = false;
    std::vector<uint32_t> subscriptions;
    cIGZMessageServer2* messageServer = nullptr;  // held while subscribed
    // The managers' tokens handed to the core at Init (one reference each,
    // released after the core's Shutdown).
    cIGZUnknown* managers[3] = {nullptr, nullptr, nullptr};
    // Wealth requester objects the host owns (one reference each).
    std::vector<cIGZUnknown*> wealth;
    // List entries and query results kept for the current core call.
    propcity::CallPool pool;
    uint32_t holds = 0;
    // Counters (log lines).
    uint32_t removes = 0, creates = 0, posts = 0, stateWrites = 0, timerWrites = 0;
    // Shadow hooks (null in the developer): recorded instead of done.
    void (*onRemove)(void* hookCtx, uint64_t prop) = nullptr;
    uint64_t (*onCreate)(void* hookCtx, uint64_t lotOrNetLot, bool network, uint32_t index, uint32_t id) = nullptr;
    uint32_t (*onSelect)(void* hookCtx, uint64_t requester, const uint32_t* ids, size_t count, int32_t x,
                         int32_t z) = nullptr;
    void* hookCtx = nullptr;
};

fc4_prop_host MakePropHost(PropDevContext* ctx);

// The canonical token of a prop: its cISC4PropOccupant interface (IID
// 0xE9793A65, as the record's references are read), the reference kept in
// `pool`; 0 when it does not answer.
uint64_t PropToken(propcity::CallPool& pool, cIGZUnknown* u);

// After a core's Read through `refs` (FC4PropCityCommon.h RecordingIStream):
// the three manager references and the sign list's (one each) are
// released, and the wealth requester references (class 0x89C49986) are
// handed to `ctx->wealth`. On a failed Read every reference is released.
void AfterPropRead(PropDevContext* ctx, fc4_prop_developer* core, bool ok,
                   const std::vector<propcity::RecordingIStream::Taken>& refs);

// Releases what the context holds (managers, message server, wealth
// requesters) after the core's Shutdown.
void ReleasePropContext(PropDevContext* ctx);

// The core's message for a game message (fc4_prop_message), the object
// canonicalised (§3.3: a prop as its cISC4PropOccupant, a network occupant
// as its cISC4Occupant, the lot-state lot as the message's IGZUnknown); the
// references go to `pool`. False for a message the developer ignores.
bool PropMessageOf(cIGZMessage2* msg, propcity::CallPool& pool, fc4_prop_message* out);

// The 13 subscriptions of §3.1 step 5.
extern const uint32_t kPropDevSubscriptions[13];

}  // namespace fc4

// The wealth requester (§1, §4.1, §8.3). Its SelectProp is answered by the
// developer's core (fc4_prop_developer_wealth_select); the core never calls
// this object (it answers wealth requesters itself).
class cFC4WealthRequester final : public cFC4ISC4PropRequester, public cIGZSerializable {
public:
    cFC4WealthRequester() = default;

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override { return ++refCount_; }
    uint32_t Release() override;

    int32_t GetRequestedShare(SC4Rect<int32_t> const& cells) override;
    uint32_t GetRequesterSatisfactionId() override;
    bool UsesCustomPropSelector() override;
    uint32_t SelectProp(fc4::GameVector<uint32_t> const& candidates, int32_t x, int32_t z) override;

    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override { return kFC4_CLSID_cSC4WealthRequester; }

    void SetOwner(cFC4PropDeveloper* d) { owner_ = d; }

private:
    uint32_t refCount_ = 0;
    cFC4PropDeveloper* owner_ = nullptr;
};

class cFC4PropDeveloper final : public cISC4PropDeveloper, public cIGZMessageTarget2, public cIGZSerializable {
public:
    cFC4PropDeveloper();
    virtual ~cFC4PropDeveloper();  // slot 11 (§8.1): the deleting destructor

    // cIGZUnknown (§1)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4PropDeveloper (§8.1)
    bool Init() override;
    bool Shutdown() override;
    void AddPropRequester(cISC4PropRequester* r) override;
    void RemovePropRequester(cISC4PropRequester* r) override;
    void UpdateAllOptionalProps() override;
    void ShowSignsAndLabels(bool show) override;
    void ToggleSignsAndLabels() override;
    bool AreSignsAndLabelsVisible() override;

    // cIGZMessageTarget2 (§3.3)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§7)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // The wealth requester's SelectProp (§4.1).
    uint32_t WealthSelect(const uint32_t* ids, size_t count, int32_t x, int32_t z);

    fc4_prop_developer* Core() const { return core_; }

    // The month status line (FC4Observer.cpp; enable = propdev only).
    static void MonthLine(int32_t julian);

private:
    void LogState(const char* what, bool ok);
    void AdoptWealthOwners();

    bool destroying_ = false;
    fc4::PropDevContext ctx_;
    fc4_prop_host host_{};
    fc4_prop_developer* core_ = nullptr;
    uint32_t seed_ = 0;
    // Counters for the month line.
    uint32_t idle_ = 0, day_ = 0, ticks_ = 0, messages_ = 0;
};

// Factories for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4PropDeveloper();
cIGZUnknown* CreateFC4WealthRequester();
