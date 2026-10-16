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

// The network lot manager's GZCOM class (docs/specs/network-manager.md §1):
// city component 11, the cISC4City slot 42 getter. The lots, the lot choice,
// the road and avenue styles, the lot objects, the potholes and the record
// live in the Rust core (fc4_netlot); this class adapts the interfaces to it
// and owns the game services it asks for.
//
// Layout (§1): cFC4ISC4NetworkLotManager and cIGZUnknown at +0, the
// cIGZMessageTarget2 sub-object at +4, cIGZSerializable at +8. QueryInterface
// answers 1 and 0x89A5BA3F at +0, 0x452294AA at +4 and 0xE4FDA3D4 at +8.
//
// Threading: the core is single-threaded (RefCell); the plugin serialises
// every call on this one object (HANDOFF §5.1).
//
// OFF by default: FreeCity4.ini `enable = netlot`; `disable` still wins. The
// network manager is its own class (`enable = netmgr`, cFC4NetworkManager.h).
#pragma once

#include "FC4Interfaces.h"
#include "FC4LotDevInterfaces.h"  // cFC4ISC4NetworkLotManager
#include "FC4NetMgrHost.h"        // fc4::NetMgrContext
#include "cISC4NetworkLot.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#include <atomic>
#include <cstdint>
#include <map>
#include <string>

class cIGZMessageServer2;
class cISC4City;
static const uint32_t kFC4_CLSID_cSC4NetworkLotManager = 0xE9A6CD3B;
static const uint32_t kFC4_IID_cISC4NetworkLotManager = 0x89A5BA3F;
static const uint32_t kFC4_IID_cISC4NetworkLot = 0x89A5BA66;

// A network lot of ours (§2.3: a 24-byte reference-counted object). The core
// keeps the lot as a (occupant, configuration id) pair in its record; this is
// the game's view of one, owned by the manager that hands it out.
class cFC4NetworkLot final : public cISC4NetworkLot {
public:
    cFC4NetworkLot(cISC4NetworkOccupant* occ, uint32_t configId) : occ_(occ), configId_(configId) {}

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    cISC4NetworkOccupant* GetNetworkOccupant() const override;
    uint32_t GetLotConfigurationObjectCount() override;
    SC4Vector<cSC4LotConfigurationObject>* GetLotConfigurationObjectArray() override;

    uint32_t ConfigId() const { return configId_; }

private:
    ~cFC4NetworkLot() = default;
    uint32_t refs_ = 1;
    cISC4NetworkOccupant* occ_ = nullptr;  // borrowed: the core holds the reference
    uint32_t configId_ = 0;
};

class cFC4NetworkLotManager final : public cFC4ISC4NetworkLotManager,
                                    public cIGZMessageTarget2,
                                    public cIGZSerializable {
public:
    cFC4NetworkLotManager();
    ~cFC4NetworkLotManager();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cFC4ISC4NetworkLotManager (§11.2, the compiled order)
    bool Init() override;
    bool Shutdown() override;
    cISC4NetworkLot* GetNetworkLot(int32_t x, int32_t z) override;
    void GetNetworkLotConfigurationSet(intptr_t set) override;
    bool GetNetworkLotConfigurationById(uint32_t id, void* out) override;
    cISC4Occupant* CreateProp(cISC4NetworkLot* lot, uint32_t index, uint32_t propId) override;
    int32_t GetZoneTypeForNetwork(int32_t x, int32_t z) override;
    int32_t GetWealthForNetwork(int32_t x, int32_t z) override;
    bool IterateAllNetworkLots(LotVisitor cb, void* ctx) override;
    void RedevelopLot(uint32_t x, uint32_t z, cISC4NetworkLot* lot) override;
    void SetLotConfigurationForNextPiece(void* cfg) override;
    bool WriteLotConfiguration(void* cfg, cIGZPersistDBSegment* seg) override;

    // cIGZMessageTarget2 (§3.2: the 6 subscribed messages)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§4.2: component 11, version 2)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // For the host callbacks (cFC4NetworkLotManager.cpp).
    cIGZMessageTarget2* Target() { return static_cast<cIGZMessageTarget2*>(this); }
    cIGZUnknown* Primary() { return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4NetworkLotManager*>(this)); }
    cIGZMessageServer2* Server() { return msgServer_; }
    void SetServer(cIGZMessageServer2* ms) { msgServer_ = ms; }
    cISC4City* City() { return city_; }
    void SetCity(cISC4City* c);
    bool Writable() const { return writable_; }

    // The lot objects this class handed out, kept alive while the manager is.
    cISC4NetworkLot* KeepLot(int32_t x, int32_t z, cISC4NetworkOccupant* occ, uint32_t configId);

private:
    uint32_t refCount_ = 0;
    bool destroying_ = false;
    bool writable_ = false;
    fc4::NetMgrContext ctx_{};
    fc4_netlot_host host_{};
    fc4_netlot* core_ = nullptr;
    cISC4City* city_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    std::map<uint64_t, cFC4NetworkLot*> lots_;
    std::atomic<uint32_t> posts_{0};
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4NetworkLotManager();
