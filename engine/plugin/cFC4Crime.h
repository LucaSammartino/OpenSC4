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

// The crime simulator's GZCOM class (docs/specs/crime.md §1, CLSID
// 0x8990C09A). The model lives in the Rust core behind fc4.h (fc4_crime);
// this class implements the four interfaces of one object (§1:
// cISC4CrimeSimulator, cIGZMessageTarget2, cISC4PropRequester,
// cIGZSerializable) and the host callbacks over the game's services (§13).
//
// The subsystem is OFF by default: FreeCity4.ini `enable = crime` turns it on
// (FC4Subsystems.h); `disable` still wins. The crime_* switches (FC4Config.h)
// turn groups of game calls off for bisecting.
//
// Interfaces called (FC4CrimeInterfaces.h, FC4Interfaces.h): police through
// the verified cISC4PoliceSimulator slot order (whatever police simulator the
// city has: the original or a replacement); residential slots 23 / 25 with
// three out pointers each (§13); lots and the lot manager through the
// verified slot order, as power and flammability do, except lot slot 15
// GetCenter, called through a thunk that reads the out parameter (§13, CQ3:
// the x87 stack is empty on return); the string detokenizer
// (cFC4ISCStringDetokenizer slot 9) for type names (§7, CQ8).
//
// Threads (foundations §19): the getters (slots 5-8, 11-13) read the core's
// snapshot and may be called from the game's second thread; they touch no
// member of this class except atomics. GetNearestCrimeSim / Prop query the
// occupant manager like the original (from whatever thread calls them). The
// prop requester's slots run the core (main thread only); a call from another
// thread is counted, logged once and answered 0.
//
// The month status line (one per SimNewMonth, from the first month on):
//   crime month YYYY-MM: crimes=N prevented=P arson=A incidents=I grid sum/max=S/M
//     average=V hotspot=(x,z)=v props requested=R selected=K/Q jailed=J released=L
//     criminal base=B to commit=C ...
#pragma once

#include "FC4CrimeInterfaces.h"
#include "FC4HostGrid.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#include <atomic>
#include <cstdint>
#include <memory>
#include <vector>

class cIGZMessageServer2;
class cISC4City;
class cISC4LotManager;
class cISC4OccupantManager;
class cISC4OrdinanceSimulator;
class cISC4PoliceSimulator;
class cISC4Simulator;

static const uint32_t kFC4_CLSID_cSC4CrimeSimulator = 0x8990C09A;
/// The simulator type answered by class slot 15 (§1).
static const uint32_t kFC4_CrimeSimulatorType = 0x27567222;

class cFC4CrimeSimulator final : public cFC4ISC4CrimeSimulator,
                                 public cIGZMessageTarget2,
                                 public cIGZSerializable,
                                 public cFC4CrimePropRequester {
public:
    cFC4CrimeSimulator();
    // Class slots of the primary vtable, in this order (§1, "Compiled slot
    // tables"): 14 the scalar deleting destructor, 15 GetSimulatorType. No
    // base declares a virtual destructor, so both are new primary slots.
    virtual ~cFC4CrimeSimulator();
    virtual uint32_t GetSimulatorType();

    // cIGZUnknown (§1: one object, one reference count)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4CrimeSimulator (§11)
    bool Init() override;
    bool Shutdown() override;
    cISC4SimGrid<uint8_t>* GetCrimeGrid() override;
    uint32_t GetAverageCrime() override;
    uint32_t GetMonthlyCrimeCount() override;
    bool GetCrimeHotspot(uint8_t* value, int32_t* x, int32_t* z) override;
    cISC4Automaton* GetNearestCrimeSim(cS3DVector3 const& pos, float radius) override;
    cISC4PropOccupant* GetNearestCrimeProp(cS3DVector3 const& pos, float radius) override;
    bool GetCrimeIncidentArray(fc4::GameVector<FC4CrimeEvent>& out) override;
    bool SetVisualsCreated(uint32_t index, bool value) override;
    bool GetVisualsCreated(uint32_t index) override;

    // cIGZMessageTarget2 (§5)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§12)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // cISC4PropRequester (§11.6)
    int32_t GetRequestedShare(SC4Rect<int32_t> const& rect) override;
    uint32_t GetRequesterSatisfactionId() override;
    bool UsesCustomPropSelector() override;
    uint32_t SelectProp(FC4U32VectorView const& props, int32_t cellX, int32_t cellZ) override;

    // For the host callbacks (cFC4Crime.cpp).
    cISC4City* City() const { return city_; }
    cISC4OccupantManager* OccupantManager() const { return occupantManager_; }
    cISC4LotManager* LotManager() const { return lotManager_; }
    cISC4OrdinanceSimulator* Ordinance() const { return ordinance_; }
    cISC4PoliceSimulator* Police() const { return police_; }
    cIGZMessageTarget2* Target() { return static_cast<cIGZMessageTarget2*>(this); }
    cFC4CrimePropRequester* Requester() { return static_cast<cFC4CrimePropRequester*>(this); }
    // The token the core knows this object by (its primary interface, the
    // identity QueryInterface hands out for 0x67659555).
    uint64_t SelfToken() const {
        return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(static_cast<const cFC4ISC4CrimeSimulator*>(this)));
    }
    void AcquireServices();
    void ReleaseServices();
    void Subscribe(const uint32_t* ids, size_t count, bool add);
    void AddAgent(const char* name, const uint32_t* levels, uint32_t count, uint32_t flags);
    void RemoveAgent();
    // Keeps a game list or object (and the references it holds) until the
    // outermost call into the core returns.
    void Hold(std::shared_ptr<void> p) { held_.push_back(std::move(p)); }

    // Observability only (the month line): host-callback counts since the
    // last month line. Main thread, like every call into the core.
    struct Probes {
        uint32_t lotsFound = 0;    // GetRandomLot answers (attempts with a lot, §6.6 step 1)
        uint32_t noLot = 0;        // GetRandomLot returned null
        uint32_t lotBits = 0;      // GetLotBits (attempts the police did not stop, §6.7)
        uint32_t igniteCalls = 0;  // arson IgniteOccupant asks (§6.8)
        uint32_t ignited = 0;      // ... that returned true
        uint32_t posts = 0;        // 0x0A159D95 posts
        uint32_t mapPosts = 0;     // 0x88EA4DFF posts
        uint32_t jailed = 0;       // GetJailPopulation (last answer)
        int32_t released = 0;      // ResetUnjailedCriminals (last answer)
        uint32_t policeGrid = 0;   // 1: the last month had a police grid
        // GetCenter's x87 stack depth after the call: unchanged per §13 (CQ3);
        // a change is counted and logged once, never compensated.
        uint32_t centerEmptySt0 = 0, centerSt0 = 0;
    };
    Probes& Probe() { return probes_; }
    // Test seam only: a borrowed city, no AddRef.
    void TestSetCity(cISC4City* city) { city_ = city; }

    class CoreCall {
    public:
        explicit CoreCall(cFC4CrimeSimulator* s) : s_(s) { ++s_->depth_; }
        ~CoreCall() {
            if (--s_->depth_ == 0) s_->held_.clear();
        }
        CoreCall(const CoreCall&) = delete;
        CoreCall& operator=(const CoreCall&) = delete;

    private:
        cFC4CrimeSimulator* s_;
    };

private:
    void LogMonth(int32_t julian);
    void LogState(const char* what);
    bool OffMain(const char* what);
    cIGZUnknown* Nearest(cS3DVector3 const& pos, float radius, uint32_t kind);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::HostGridContext gridCtx_{"crime", fc4::Sw::CrimeGridWrites};
    fc4_crime_host host_{};
    fc4_crime* core_ = nullptr;
    cISC4City* city_ = nullptr;
    cISC4OccupantManager* occupantManager_ = nullptr;
    cISC4LotManager* lotManager_ = nullptr;
    cISC4OrdinanceSimulator* ordinance_ = nullptr;
    cISC4PoliceSimulator* police_ = nullptr;
    cISC4Simulator* agentSim_ = nullptr;  // AddRef'd while the agent is registered
    cIGZMessageServer2* msgServer_ = nullptr;
    std::vector<std::shared_ptr<void>> held_;
    int depth_ = 0;
    uint32_t days_ = 0, months_ = 0;
    int32_t lastJulian_ = 0;  // the last SimNewDay's date (the month line's label)
    Probes probes_;
    // Prop requester slots (prop developer timing; atomics: any thread).
    std::atomic<uint32_t> shareCalls_{0}, selectCalls_{0}, selected_{0}, offMainCalls_{0};
    std::atomic<bool> offMainLogged_{false};
    uint32_t incidentArrayCalls_ = 0;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4CrimeSimulator();

namespace fc4 {
// The crime host table for the shadow (FC4CrimeShadow.cpp): tuning,
// for_each_type_key, exemplar and ltext only (the resource and language
// managers, read only, and the game's string detokenizer on the name copies,
// crime.md §7 step 3); every other callback NULL.
fc4_crime_host CrimeReadOnlyHost();
// Test seam only (op 27, kFC4SeamLotCenter): the lot slot 15 thunk on `lot` (a
// cISC4Lot*). Returns the values the call left on the x87 stack (0 = the
// original; 1 = one pushed, which the caller must pop; 7 = one popped).
int CrimeTestLotCenter(void* lot, float* out3);
}  // namespace fc4
