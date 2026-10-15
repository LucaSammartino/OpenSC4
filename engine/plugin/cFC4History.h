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

// The history warehouse's GZCOM class (docs/specs/history.md §1): city
// component 1, the cISC4City slot 32 getter. The series, the yearly thinning
// pass, the queries and the record live in the Rust core (fc4_history); this
// class adapts the interfaces to it and owns the game services it uses.
//
// Layout (§1): cISC4HistoryWarehouse and cIGZUnknown at +0, the
// cIGZMessageTarget2 sub-object at +4 (its subscriptions and agents),
// cIGZSerializable at +8. QueryInterface answers 0x69F4A803, 1 and
// 0xE4FDA3D4 only, NOT the message target's 0x452294AA [quirk].
//
// Threading: one CoreLock (FC4HistClock.h). RecordHistoryData (slot 12) runs
// outside it: it only sends the record message, which comes back through the
// message server to DoMessage. Queries take the caller's x87 precision.
//
// OFF by default: FreeCity4.ini `enable = history`; `disable` still wins.
#pragma once

#include "FC4HistClock.h"
#include "FC4Interfaces.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#include <atomic>
#include <cstdint>

class cIGZMessageServer2;
class cISC4City;
class cISC4Simulator;

static const uint32_t kFC4_CLSID_cSC4HistoryWarehouse = 0x89EFA536;
static const uint32_t kFC4_IID_cISC4HistoryWarehouse = 0x69F4A803;

class cFC4HistoryWarehouse final : public cFC4ISC4HistoryWarehouse,
                                   public cIGZMessageTarget2,
                                   public cIGZSerializable {
public:
    cFC4HistoryWarehouse();
    ~cFC4HistoryWarehouse();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4HistoryWarehouse (§8, the compiled order)
    bool Init() override;
    bool Shutdown() override;
    bool GetCount(uint32_t key, cFC4IGZDate const& from, cFC4IGZDate const& to, float& n) override;
    bool GetTrend(uint32_t key, cFC4IGZDate const& from, cFC4IGZDate const& to, float& slope,
                  float& intercept) override;
    bool GetMinMaxAverage(uint32_t key, cFC4IGZDate const& from, cFC4IGZDate const& to, float& min, float& max,
                          float& avg) override;
    bool GetPoint(uint32_t key, cFC4IGZDate const& date, float& v) override;
    bool Coalesce(uint32_t key, cFC4IGZDate const& from, cFC4IGZDate const& to, int32_t granularity) override;
    bool DisposeAll(uint32_t key) override;
    bool DisposeRange(uint32_t key, cFC4IGZDate const& from, cFC4IGZDate const& to) override;
    bool RecordHistoryData(uint32_t key, int32_t value) override;
    bool SetHistoryDataQueueLength(uint32_t key, uint32_t n) override;

    // cIGZMessageTarget2 (§3)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§7)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // For the host callbacks (cFC4History.cpp).
    cIGZMessageTarget2* Target() { return static_cast<cIGZMessageTarget2*>(this); }
    cIGZUnknown* Primary() { return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4HistoryWarehouse*>(this)); }
    fc4::CoreLock& Lock() { return lock_; }
    uint32_t CachedDate() const { return date_; }
    void Subscribe(uint32_t type, bool add);
    void Agent(uint32_t level, const char* name, uint32_t flags, bool add);
    void Send(uint32_t type, uint32_t data1, uint32_t data2);

private:
    uint32_t SimDate();
    fc4_history_state State();  // under the lock

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::CoreLock lock_;
    fc4_history_host host_{};
    fc4_history* core_ = nullptr;
    cISC4City* city_ = nullptr;
    cISC4Simulator* simulator_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    uint32_t date_ = 0;  // the simulation date read before the core call that may store a point
    std::atomic<uint32_t> records_{0};
    std::atomic<uint32_t> queries_{0};
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4HistoryWarehouse();
