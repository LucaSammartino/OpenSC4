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

// The park manager (cSC4ParkManager, CLSID 0xAA3FE6BC, city component 15):
// docs/specs/parks.md. The behaviour lives in the Rust core behind fc4.h
// (fc4_park); this class implements the three interfaces (§1:
// cISC4ParkManager at +0, cIGZMessageTarget2 at +4, cIGZSerializable at +8)
// in the slot order of §8 and the host callbacks over the game's services
// (budget, lots, occupants, the message server, automata).
//
// The tuning (§6, exemplar "Parks Manager" 0x6534284A / 0x67CD5FA1 /
// 0x8A4805BC) is handed to the core at Init (it reads it at the object's
// first Init only). The ZooEscape message (§4.5) is a standard message with
// void1 = this object's cISC4ParkManager interface and IGZUnknown = the zoo,
// MessagePost(msg, false).
//
// The member generator (§5) is seeded from the session seed S [choice]:
// `parks_seed` / FC4_PARKS_SEED, else the cRZTime clock; the core derives
// derive(0xAA3FE6BC) from it.
//
// OFF by default: FreeCity4.ini `enable = parks` (FC4Subsystems.h).
//
// Log lines (FreeCity4.log):
//   parks Init|Read|Shutdown: ok=.. init=.. parks=.. zoos=.. threshold=.. curve=.. dept=.. latch=.. held=....
//   parks Write -> 0|1 (parks=.. zoos=..)
//   parks month YYYY-MM-DD: parks=.. zoos=.. funding=.. latch=.. lot-writes=.. escapes=.. occupants=..
#pragma once

#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4ParkManager.h"
#include "fc4.h"

#include <cstdint>

class cIGZMessageServer2;
class cISC4BudgetSimulator;
class cISC4City;
class cISC4LotManager;

static const uint32_t kFC4_CLSID_cSC4ParkManager = 0xAA3FE6BC;
static const uint32_t kFC4_IID_cISC4ParkManager = 0xAA3FE6BC;  // §1: the same value as the CLSID

namespace fc4 {

// What the park host callbacks work on. The manager owns one; the parks
// shadow (FC4ParksShadow.h) makes a read-only one (no writes to the game).
struct ParkContext {
    bool readOnly = false;
    cIGZMessageTarget2* target = nullptr;  // the manager's +4 (agents, notifications)
    bool agentRegistered = false;
    uint32_t subscriptions = 0;
    uint32_t holds = 0;  // references the core holds through add_ref / resolve_ref
    // Held through acquire_* (§4.1 steps 2, 4, 5).
    cIGZMessageServer2* messageServer = nullptr;
    cISC4City* city = nullptr;
    cISC4BudgetSimulator* budget = nullptr;
    cISC4LotManager* lotManager = nullptr;
    // Counters (log lines).
    uint32_t lotWrites = 0, escapes = 0, posts = 0;
};

fc4_park_host MakeParkHost(ParkContext* ctx);

// The session seed S: `parks_seed` / FC4_PARKS_SEED, else the cRZTime clock.
uint32_t ParkSessionSeed(bool* fixed);

// The "Parks Manager" exemplar's properties (§6) as a new fc4_props (the
// caller destroys it); nullptr when the exemplar is missing.
fc4_props* ParkTuning();

}  // namespace fc4

class cFC4ParkManager final : public cISC4ParkManager, public cIGZMessageTarget2, public cIGZSerializable {
public:
    cFC4ParkManager();
    ~cFC4ParkManager();

    // cIGZUnknown (§1)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4ParkManager (§8)
    bool Init() override;
    bool Shutdown() override;

    // cIGZMessageTarget2 (§3)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§7)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // The core (tests, diagnostics); main thread.
    fc4_park* Core() const { return core_; }

private:
    void LogState(const char* what, bool ok);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::ParkContext ctx_;
    fc4_park_host host_{};
    fc4_park* core_ = nullptr;
    uint32_t seed_ = 0;
    bool seedFixed_ = false;
    uint32_t occupantMsgs_ = 0;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4ParkManager();

// Test seam op 57 (FC4TestSeam.h kFC4SeamParks): the host's ZooEscape post.
struct FC4ParksTestInput {
    cIGZMessageServer2* server;  // borrowed; receives MessagePost
    cIGZUnknown* manager;        // void1
    cIGZUnknown* zoo;            // IGZUnknown
};

namespace fc4 {
intptr_t ParksTestZooEscape(const FC4ParksTestInput* in);
}  // namespace fc4
