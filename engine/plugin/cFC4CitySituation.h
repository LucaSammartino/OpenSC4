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

// The city situation manager's GZCOM class (docs/specs/city-situation.md §1):
// city component 52, the cISC4City slot 49 getter. The mission clock, the
// status block and the save record live in the Rust core (fc4_citysit); this
// class adapts the interfaces to it and owns the game services it uses.
//
// Layout (§1): cISC4CitySituationManager and cIGZUnknown at +0, the
// cIGZMessageTarget2 sub-object at +4 (its subscriptions and the animation
// tick), cIGZMessageTarget at +8, and cIGZUnknown / cIGZSerializable at
// +0xC [quirk]. QueryInterface answers the sub-object offset the core gives:
// 0x6BB1437A at +0, 0x452294AA at +4, 0xE98B2F57 at +8, and 1 / 0xE4FDA3D4 at
// +0xC. A sub-object at +0x10 (the window procedure) is not reachable.
//
// This is the §8 **minimal replacement** [choice] the spec sanctions: it keeps
// the record byte for byte and answers the interface with "no situation". The
// mission gameplay itself (targets, pursuit, coverage, the player-drive
// camera, the windows and effects) is deliberately out of scope — §0 calls a
// full one "a gameplay feature of several thousand lines". The host table
// therefore wires the message server and its subscriptions, the indicator
// view's ShowAllIndicators, the preferences' reward list, the aura / budget
// outcome and the messages it posts, and leaves the mission pieces NULL (a
// missing callback reads as 0 / false / none). Unattended play never starts a
// mission (§0: "its popup never opens by itself"), so the difference is only
// visible to a player who starts one.
//
// Threading: the core is single-threaded (RefCell); the plugin serialises
// every call on this one object. Getters run from a second thread while the
// animation tick runs (HANDOFF §5.1), so nothing here lives in a RefCell.
//
// OFF by default: FreeCity4.ini `enable = citysit`; `disable` still wins.
#pragma once

#include "FC4Interfaces.h"
#include "cIGZMessageTarget.h"
#include "cGZMessage.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#include <atomic>
#include <cstdint>
#include <mutex>

class cIGZMessageServer2;
class cISC4City;

static const uint32_t kFC4_CLSID_cSC4CitySituationManager = 0x0BB14381;
static const uint32_t kFC4_IID_cISC4CitySituationManager = 0x6BB1437A;

class cFC4CitySituationManager final : public cFC4ISC4CitySituationManager,
                                       public cIGZMessageTarget2,
                                       public cIGZMessageTarget,
                                       public cIGZSerializable {
public:
    cFC4CitySituationManager();
    ~cFC4CitySituationManager();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4CitySituationManager (§8, the compiled order)
    bool Init() override;
    bool Shutdown() override;
    uint16_t GetMissionSuccessCount(uint32_t guid) override;
    cISC4CitySituationAdvice* GetActiveSituation() override;
    uint32_t GetCurrentStatus() override;
    cISC4Automaton* GetActiveAuto() override;
    cISC4Occupant* GetCurrentTarget() override;
    uint32_t GetCurrentTargetIndex() override;
    uint32_t GetNumTargets() override;
    uint32_t GetNumTargetsRemaining() override;
    float GetCurrentTimeLimitSeconds() override;
    float GetCurrentSituationTime() override;
    float GetCurrentTimeRemainingSeconds() override;
    void GetCurrentTargetName(cIGZString& out) override;
    bool GetSituationAura(float& radius, float& magnitude) override;
    bool GetSituationMoney(int64_t& money) override;
    bool IsSituationAvailable(cISC4Automaton* automaton) override;
    bool IsSituationAvailable(cISC4Occupant* occupant) override;
    bool InitiateSituationByGuid(uint32_t guid, bool showBriefing, uint32_t group) override;
    bool InitiateSituationForOccupant(cISC4Occupant* occ, cISC4CitySituationAdvice* a) override;
    bool InitiateSituationByAutomataGroup(uint32_t group, bool showBriefing, bool search) override;
    void StartJoyride(cISC4Automaton* automaton, bool b, cIGZMessageTarget2* target) override;
    void TreatAutomatonAsGroup(cISC4Automaton* automaton, uint32_t group) override;
    void SetMaxCSI(uint8_t n) override;
    uint8_t GetMaxCSI() override;
    int32_t GetTrackedAutomataCount() override;
    void SetCSIVisible(bool visible) override;
    bool GetCSIVisible() override;
    bool HandleTuningParametersChanged() override;
    uint32_t CountdownIsInProgress() override;
    void ResetTutorials() override;
    void InitiatePlayerDriveMode(bool on) override;
    bool InitiateSituation(cISC4CitySituationAdvice* advice, cISC4Automaton* automaton, bool showBriefing) override;
    void ActivateSituation() override;
    void DeactivateSituation() override;
    void EndSituation() override;

    // cIGZMessageTarget2 (§3) and the legacy cIGZMessageTarget (§1)
    bool DoMessage(cIGZMessage2* pMessage) override;
    bool DoMessage(cGZMessage& sMessage) override;

    // cIGZSerializable (§7)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // For the host callbacks (cFC4CitySituation.cpp).
    cIGZMessageTarget2* Target() { return static_cast<cIGZMessageTarget2*>(this); }
    cIGZUnknown* Primary() { return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4CitySituationManager*>(this)); }

    bool msgServerHeld = false;
    cIGZMessageServer2* Server();
    void SetServer(cIGZMessageServer2* ms);
    cISC4City* City();
    void SetCity(cISC4City* c);
    void Send(uint32_t type, uint32_t data1, uint32_t data2, uint64_t object);
    void Post(uint32_t type, uint64_t object, uint32_t data1, uint32_t data2);

private:
    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4_citysit_host host_{};
    fc4_citysit* core_ = nullptr;
    cISC4City* city_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    std::atomic<uint32_t> sends_{0};

public:
    // glue-citysit4: every call into the core runs under this lock. The core
    // is single-threaded (RefCell) while the game reads slots 19/27/28 from
    // the simulation thread (§9) and the animation tick arrives on the main
    // thread (§3). Recursive: a host callback may re-enter through a
    // synchronous MessageSend.
    std::recursive_mutex mu;
    // The trace (glue-citysit4): calls per §8 slot (0-39), the animation
    // ticks and the other messages delivered, logged at the first call of a
    // slot and summed at Shutdown.
    std::atomic<uint32_t> slotCalls[40] = {};
    std::atomic<uint32_t> ticks{0};
    std::atomic<uint32_t> messages{0};
    bool tickRegistered = false;
    void Note(int slot, const char* name);
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4CitySituationManager();
