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

// The commercial simulator (cSC4CommercialSimulator, CLSID 0x8990C080, city
// component 22) and the industrial stub (cSC4IndustrialSimulator, CLSID
// 0xA990C022, city component 28): docs/specs/commercial-industrial.md. The
// behaviour lives in the Rust core behind fc4.h (fc4_commercial /
// fc4_industrial); these classes implement the three interfaces of each object
// (§1: the primary interface at +0, cIGZMessageTarget2 at +4,
// cIGZSerializable at +8) in the slot order of §9, and the commercial host
// callbacks over the game's services.
//
// The CO$$ / CO$$$ proximity grids (§4.6) are the GAME's own
// cSC4SimGrid<uint8> objects, created through GZCOM (FC4HostGrid.h) or handed
// over by the save (§8.1, by reference), because the growth developers and the
// query tool read their memory directly (§10, simgrid.md §6.3).
// GetProximityMap returns the interface pointer behind the core's grid token,
// without an AddRef.
//
// Threads: the core is main-thread only. GetProximityMap may be called from
// any thread: it reads the two grid pointers this class publishes (atomics)
// after every main-thread call that can change them (Init, Shutdown, Read).
//
// Both subsystems are OFF by default: FreeCity4.ini `enable = commercial` /
// `enable = industrial` turns one on (FC4Subsystems.h); `disable` still wins.
//
// Log lines (FreeCity4.log):
//   commercial Init|Read|Shutdown: ok=.. dirty=.. state=.. grids CO$$=.. CO$$$=.. demand=../..
//   commercial Write -> 0|1
//   commercial rebuild N YYYY-MM-DD: CO$$ edge=../.. zeros=.. CO$$$ edge=.. zeros=..
//     (one per finished pair of map rebuilds, §4.3)
//   commercial month YYYY-MM-DD: dirty=.. state=.. rebuilds=.. occupants=..
//     (inserted/removed messages seen, of which office buildings) days=.. idles=..
//   industrial Init|Read|Write|Shutdown: ...
#pragma once

#include "FC4HostGrid.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4CommercialSimulator.h"
#include "cISC4IndustrialSimulator.h"
#include "fc4.h"

#include <atomic>
#include <cstdint>

class cIGZMessageServer2;
class cISC4BuildingDevelopmentSimulator;
class cISC4City;

static const uint32_t kFC4_CLSID_cSC4CommercialSimulator = 0x8990C080;
static const uint32_t kFC4_IID_cISC4CommercialSimulator = 0x077ABEB2;
static const uint32_t kFC4_CLSID_cSC4IndustrialSimulator = 0xA990C022;
static const uint32_t kFC4_IID_cISC4IndustrialSimulator = 0x477A9E7D;

namespace fc4 {

// What the commercial host callbacks work on. The simulator owns one; the
// commercial shadow (FC4CommercialShadow.h) makes a read-only one over the
// city it reads (only building_map, building_map_key and building_purpose).
struct CommercialContext {
    bool readOnly = false;
    cISC4City* city = nullptr;                          // simulator: held (§4.1 step 4); shadow: borrowed
    cISC4BuildingDevelopmentSimulator* bds = nullptr;   // simulator: held; shadow: the city's, borrowed
    cIGZMessageServer2* msgServer = nullptr;            // §4.1 step 3
    cIGZMessageTarget2* target = nullptr;               // simulator only: agents and notifications
    bool agentsRegistered = false;
    uint32_t demandNodes = 0;  // held demand nodes (§2.1, never read)
    uint32_t occupantMessages = 0;  // insert / remove messages handed to the core
    uint32_t officeMessages = 0;    // ... whose building's purpose was 3 (§3)
};

// The host table over `ctx`.
fc4_commercial_host MakeCommercialHost(CommercialContext* ctx, const HostGridContext* grids);

}  // namespace fc4

class cFC4CommercialSimulator final : public cISC4CommercialSimulator,
                                      public cIGZMessageTarget2,
                                      public cIGZSerializable {
public:
    cFC4CommercialSimulator();
    ~cFC4CommercialSimulator();

    // cIGZUnknown (§1, §2.3)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4CommercialSimulator (§9)
    bool Init() override;
    bool Shutdown() override;
    cISC4SimGrid<uint8_t>* GetProximityMap(cISC4BuildingDevelopmentSimulator::DeveloperType type) override;

    // cIGZMessageTarget2 (§3)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§8.1)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

private:
    void Publish();
    void LogState(const char* what, bool ok);
    void NoteStep(uint32_t type, int32_t before);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4::HostGridContext gridCtx_;
    fc4::CommercialContext ctx_;
    fc4_commercial_host host_{};
    fc4_commercial* core_ = nullptr;
    std::atomic<cISC4SimGrid<uint8_t>*> co2_{nullptr};
    std::atomic<cISC4SimGrid<uint8_t>*> co3_{nullptr};
    int32_t lastJulian_ = 0;
    uint32_t rebuilds_ = 0;  // finished CO$$ + CO$$$ pairs (state 2 -> 3)
    uint32_t days_ = 0, idles_ = 0;
};

class cFC4IndustrialSimulator final : public cISC4IndustrialSimulator,
                                      public cIGZMessageTarget2,
                                      public cIGZSerializable {
public:
    cFC4IndustrialSimulator();
    ~cFC4IndustrialSimulator();

    // cIGZUnknown (§1, §2.3)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4IndustrialSimulator (§9)
    bool Init() override;
    bool Shutdown() override;

    // cIGZMessageTarget2 (§3: reads the type, returns true)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§8.2)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

private:
    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4_industrial* core_ = nullptr;
};

// Factories for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4CommercialSimulator();
cIGZUnknown* CreateFC4IndustrialSimulator();
