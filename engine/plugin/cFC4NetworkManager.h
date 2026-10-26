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

// The network manager's GZCOM class (docs/specs/network-manager.md §1): city
// component 53, the cISC4City slot 38 getter. The managed list, the flags
// table, the terrain rule, removal queueing, the crews, the retrofit list and
// the record live in the Rust core (fc4_netmgr); this class adapts the
// interfaces to it and owns the game services it asks for.
//
// Layout (§1): cFC4ISC4NetworkManager and cIGZUnknown at +0, the
// cIGZMessageTarget2 sub-object at +4, cIGZSerializable at +8. QueryInterface
// answers 1 and 0xA9764C8C at +0, 0x452294AA at +4 and 0xE4FDA3D4 at +8
// (fc4_netmgr_query_interface accepts exactly those).
//
// Threading: the core is single-threaded (RefCell); the plugin serialises
// every call on this one object. Getters run from a second thread while OnTick
// runs (HANDOFF §5.1), so nothing here lives in a RefCell.
//
// OFF by default: FreeCity4.ini `enable = netmgr`; `disable` still wins. The
// network lot manager is its own class (`enable = netlot`, cFC4NetworkLotManager.h).
#pragma once

#include "FC4Interfaces.h"
#include "FC4LotDevInterfaces.h"  // cFC4ISC4NetworkManager
#include "FC4NetMgrHost.h"        // fc4::NetMgrContext
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#include <atomic>
#include <cstdint>

class cIGZMessageServer2;
class cISC4City;

static const uint32_t kFC4_CLSID_cSC4NetworkManager = 0xC990BD46;
static const uint32_t kFC4_IID_cISC4NetworkManager = 0xA9764C8C;

class cFC4NetworkManager final : public cFC4ISC4NetworkManager,
                                 public cIGZMessageTarget2,
                                 public cIGZSerializable {
public:
    cFC4NetworkManager();
    ~cFC4NetworkManager();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4NetworkManager (§11.1, the compiled order)
    bool Init() override;
    bool Shutdown() override;
    void* GetNetworkTool(int32_t type, bool unique) override;
    void DestroyNetworkOccupants(SC4List<cISC4Occupant*> const& list) override;
    void QueueRemovals(bool queue) override;
    bool RemoveOrReplaceOccupant(void* tool, cISC4Occupant* occ, bool& replaced) override;
    bool IsOccupantSubwayStation(cISC4Occupant* occ) override;
    intptr_t FindPipeOccupant(intptr_t a, int32_t b, intptr_t c) override;
    intptr_t FindPipeConnectionOccupant(intptr_t a, int32_t b, intptr_t c) override;
    intptr_t GetAllIntersectionPieces(intptr_t a, intptr_t b, intptr_t c, intptr_t d) override;
    uint32_t MapPreviewTexture(uint32_t a, uint8_t& b) override;
    uint32_t MapShadowTexture(uint32_t a, uint8_t& b, uint32_t c, uint32_t d) override;
    uint32_t MapPath(uint32_t a) override;
    void RegisterConstructionCrew(intptr_t crew) override;
    intptr_t UnregisterConstructionCrew(intptr_t crew) override;
    void* GetWaterOccupantFilter() override;
    void* GetSubwayOccupantFilter() override;
    bool CreateConnectionNode(cISC4Occupant*& out, int32_t type, SC4Point<int> const& cell, uint32_t edgesA,
                              uint32_t edgesB, uint32_t pathOverride, uint8_t c) override;
    void AutoconnectLotNodes(cISC4Lot* lot, SC4List<cISC4Occupant*> const& nodes) override;
    const char* GetEnglishNetworkName(int32_t type) override;
    void AddToRetrofitList(intptr_t networkOccupant) override;

    // cIGZMessageTarget2 (§3: the 11 subscribed messages)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§4.1: component 53, version 1)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // For the host callbacks (cFC4NetworkManager.cpp).
    cIGZMessageTarget2* Target() { return static_cast<cIGZMessageTarget2*>(this); }
    cIGZUnknown* Primary() { return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4NetworkManager*>(this)); }
    cIGZMessageServer2* Server() { return msgServer_; }
    void SetServer(cIGZMessageServer2* ms) { msgServer_ = ms; }
    cISC4City* City() { return city_; }
    void SetCity(cISC4City* c);
    bool Writable() const { return writable_; }
    void SetWritable(bool w) { writable_ = w; }

private:
    uint32_t refCount_ = 0;
    bool destroying_ = false;
    bool writable_ = false;
    // The host's user: the context first (the shared read callbacks read the
    // user as a NetMgrContext*), this second (the write callbacks reach the
    // manager through it, FC4NetMgrHost.h NetMgrOwner).
    fc4::NetMgrOwner owner_{};
    fc4_netmgr_host host_{};
    fc4_netmgr* core_ = nullptr;
    cISC4City* city_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    std::atomic<uint32_t> posts_{0};
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4NetworkManager();

// DLL-only probe of the specified crew shutdown slot and reentrant ownership.
intptr_t NetworkCrewOwnershipTest();
