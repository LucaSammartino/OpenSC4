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
#include "cSC4LotConfigurationObject.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "SC4Vector.h"
#include "fc4.h"

#include <atomic>
#include <cstdint>
#include <map>
#include <memory>
#include <string>
#include <vector>

class cIGZMessageServer2;
class cISC4City;
class cSC4LotConfigurationObject;
static const uint32_t kFC4_CLSID_cSC4NetworkLotManager = 0xE9A6CD3B;
static const uint32_t kFC4_IID_cISC4NetworkLotManager = 0x89A5BA3F;
static const uint32_t kFC4_IID_cISC4NetworkLot = 0x89A5BA66;

// One network lot configuration of ours (§2.4): the core's copy (fc4_netlot)
// plus the game's view the callers reach: the configuration's objects as
// 0x34-byte cSC4LotConfigurationObject records (§2.3 network lot slot 5,
// prop-developer.md §4.3 step 2) and their ID lists (§8.6, object_ids). The
// records are built once, from the exemplar's lot-object properties
// (0x88EDC900 + k, lot-developer.md §17.1, prop-lotconfig.md §B3.3 with the
// network detail d = 3), and shared by every lot of the configuration. The
// record struct is the SDK's cSC4LotConfigurationObject (its layout matches
// the compiled one, lot-developer.md §17.1); the records live in one
// manager-owned SC4Vector built once per configuration (glue24: runs
// glue24-rep-6 - two storages that both hold the records by value double-
// freed each record's SC4Vector data at Shutdown, the run's heap-corruption
// crash; one owner only).
class cFC4NetworkLotConfiguration {
public:
    explicit cFC4NetworkLotConfiguration(uint64_t token) : token_(token) {}

    void SetToken(uint64_t token) { token_ = token; }
    void SetConfigId(uint32_t id) { configId_ = id; }
    void AddIds(std::vector<uint32_t> ids) { ids_.push_back(std::move(ids)); }
    // Builds one record in place: SC4Vector's push_back memmoves the element
    // bytes, so an AddObject(const T&) would share the source's `data` buffer
    // with this record and free it twice (runs glue24-rep-6 / -rep-save: the
    // heap-corruption crash at Shutdown in SC4Vector::deallocate). The caller
    // passes the fields and the data list; the data is copied value by value.
    void AddObject(uint8_t type, uint8_t flags, uint8_t rotation, bool mirrored, float x, float y, float z,
                   float rectMaxX, float rectMaxZ, float rectMinX, float rectMinZ, uint32_t foundationIID,
                   uint32_t objectID, const std::vector<uint32_t>& data) {
        records_.push_back(cSC4LotConfigurationObject{});  // default-built slot (data empty)
        cSC4LotConfigurationObject& r = *(records_.end() - 1);
        r.type = static_cast<cSC4LotConfigurationObject::Type>(type);
        r.flags = flags;
        r.rotation = static_cast<cSC4LotConfigurationObject::Rotation>(rotation);
        r.mirrored = mirrored;
        r.x = x;
        r.y = y;
        r.z = z;
        r.rectMaxX = rectMaxX;
        r.rectMaxZ = rectMaxZ;
        r.rectMinX = rectMinX;
        r.rectMinZ = rectMinZ;
        r.foundationIID = foundationIID;
        r.objectID = objectID;
        for (const uint32_t v : data) r.data.push_back(v);
    }
    uint64_t Token() const { return token_; }
    uint32_t ConfigId() const { return configId_; }
    // The raw ID list of object `index` (the core's object_ids input).
    const std::vector<uint32_t>& ObjectIds(size_t index) const;
    // The type of object `index` (0 building, 1 prop, 4 flora; 0xFF past the
    // kept records, so the expansion does nothing).
    uint8_t ObjectType(size_t index) const {
        return index < records_.size() ? static_cast<uint8_t>(records_[index].type) : 0xFF;
    }
    // The object count (the kept records).
    size_t ObjectCount() const { return records_.size(); }

    // The shared object array: the one record storage, never null (a caller
    // that takes GetLotConfigurationObjectArray() and calls size() on it would
    // dereference null, this lane's run glue24-rep-2). The pointer stays valid
    // for the manager's life; the SDK's cISC4NetworkLot slot 5 returns
    // SC4Vector<cSC4LotConfigurationObject>* whose elements are records.
    SC4Vector<cSC4LotConfigurationObject>* Objects() { return &records_; }

private:
    uint64_t token_ = 0;
    uint32_t configId_ = 0;  // the exemplar's property 0x21 (§2.4)
    // The kept records, in property order: the ONLY storage (SC4Vector's
    // push_back memmoves the element bytes, and its destructor runs each
    // element's destructor, so a second copy would free the same data).
    SC4Vector<cSC4LotConfigurationObject> records_;
    // Per kept-object index: the raw ID list of the object (object_ids).
    std::vector<std::vector<uint32_t>> ids_;
};

// A network lot of ours (§2.3: a 24-byte reference-counted object). The core
// keeps the lot as a (occupant, configuration id) pair in its record; this is
// the game's view of one, owned by the manager that hands it out.
class cFC4NetworkLot final : public cISC4NetworkLot {
public:
    cFC4NetworkLot(cISC4NetworkOccupant* occ, std::shared_ptr<cFC4NetworkLotConfiguration> config);

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    cISC4NetworkOccupant* GetNetworkOccupant() const override;
    uint32_t GetLotConfigurationObjectCount() override;
    SC4Vector<cSC4LotConfigurationObject>* GetLotConfigurationObjectArray() override;

    cFC4NetworkLotConfiguration* Config() const { return config_.get(); }

private:
    ~cFC4NetworkLot();
    std::atomic<uint32_t> refs_{1};
    cISC4NetworkOccupant* occ_ = nullptr;  // owned, independently of the core's cell entry
    std::shared_ptr<cFC4NetworkLotConfiguration> config_;
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
    // The configuration with the core's token (host identity), or null.
    cFC4NetworkLotConfiguration* ConfigByToken(uint64_t token) const;

private:
    // §3.2 step 4: every exemplar of exemplar type 0x21 (read once per
    // process), each as a core property holder plus the game's object records.
    void ReadConfigurations();

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    bool writable_ = false;
    // The host's user: the context first (the shared read callbacks read the
    // user as a NetMgrContext*), this second (the write callbacks reach the
    // manager through it, FC4NetMgrHost.h NetMgrOwner).
    fc4::NetMgrOwner owner_{};
    fc4_netlot_host host_{};
    fc4_netlot* core_ = nullptr;
    cISC4City* city_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    std::map<uint64_t, cFC4NetworkLot*> lots_;
    std::map<uint64_t, std::shared_ptr<cFC4NetworkLotConfiguration>> configs_;
    std::vector<fc4_props*> configProps_;  // alive until the adds are done
    std::atomic<uint32_t> posts_{0};
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4NetworkLotManager();

// Test seam: exercise retained lot dependencies and the manager's borrowed cache.
intptr_t NetworkLotOwnershipTest(cIGZUnknown* first, cIGZUnknown* second);
