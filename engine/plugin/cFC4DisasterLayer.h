// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.
//
// The disaster layer replacement (`enable = disasters`; docs/specs/disasters.md
// §2, contract §2.8). Layer-only: the seven managers are the game's own,
// created through GZCOM CreateInstance(class, 0x021E9CBF) and returned by
// slot 7 unchanged (§2.8 DS44: their instances cast that pointer to the
// concrete native manager). The layer state lives in the Rust core
// (fc4_dlayer_*, engine/capi/src/disaster_layer.rs).
//
// References (§2.8, conformance test 23): the factory returns the object
// with no reference; the director's QueryInterface adds the city's one.
// The effect and toxic instances AddRef it in their Init and Release it in
// their Shutdown, which runs inside our Shutdown. We hold no city reference.
#pragma once

#include "cIGZMessageTarget2.h"
#include "cIGZUnknown.h"
#include "fc4.h"

#include <atomic>
#include <cstdint>
#include <map>

class cISC4City;
class cISC4DisasterInstance;
class cISC4DisasterManager;
class cISC4Occupant;
class cIGZMessage2;

static const uint32_t kFC4_CLSID_cSC4DisasterLayer = 0x61F6ABF5;
static const uint32_t kFC4_IID_cISC4DisasterLayer = 0x29B02A2C;
static const uint32_t kFC4_IID_cISC4DisasterManager = 0x021E9CBF;

// cISC4DisasterLayer as the game calls it (docs/abi/interfaces.tsv, vtable
// 0x00A9E390, 19 slots). The SDK header gives slot 9 an argument; the ABI
// row pops 0, so it takes none here.
class cFC4ISC4DisasterLayer : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                       // 3
    virtual bool Shutdown() = 0;                                                   // 4
    virtual bool EnableDisasters(bool enable) = 0;                                 // 5
    virtual bool DisastersEnabled() = 0;                                           // 6
    virtual cISC4DisasterManager* GetDisasterManager(uint32_t type) = 0;           // 7
    virtual bool DisasterHasBegun(cISC4DisasterInstance* instance) = 0;            // 8
    virtual bool DisasterHasEnded() = 0;                                           // 9, pop 0
    virtual bool IsADisasterOngoing() = 0;                                         // 10
    virtual bool GoToNextDisaster() = 0;                                           // 11
    virtual bool GetMostRecentDisasterLocation(cISC4DisasterInstance* instance, int& x, int& z) = 0;  // 12
    virtual bool RegisterDisasterLocation(cISC4DisasterInstance* instance, int x, int z, cISC4Occupant* occupant,
                                          int32_t zoom, bool focus) = 0;           // 13
    virtual bool UnregisterDisasterLocations(cISC4DisasterInstance* instance) = 0;  // 14
    virtual bool UnregisterDisasterLocationByOccupant(cISC4DisasterInstance* instance,
                                                      cISC4Occupant* occupant) = 0;  // 15
    virtual int32_t GetDisasterLocationCount() = 0;                                // 16
    virtual bool GetDisasterLocationByIndex(int32_t index, cISC4DisasterInstance** instance, int& x,
                                            int& z) = 0;                           // 17
    virtual const void* GetDisasterCost() = 0;                                     // 18: the §2.6 record
};

// The §2.6 statistics record, in record offsets (static_asserts in the .cpp).
// +0x50 is the special-building vector; its element layout is not given
// (only the safety advisor reads the record, at +0x00/+0x08/+0x5C/+0x60), so
// it stays an empty vector (three null pointers) [question DS46].
struct FC4DisasterCostRecord {
    uint32_t buildingCount;   // +0x00
    uint32_t pad04;
    int64_t totalCost;        // +0x08
    struct Kind {
        uint32_t count;
        uint32_t pad;
        int64_t cost;
    } kinds[4];               // +0x10 .. +0x48
    void* special[3];         // +0x50 (x86)
    float fireEffectiveness;  // +0x5C
    float policeEffectiveness;  // +0x60
};

cIGZUnknown* CreateFC4DisasterLayer();

class cFC4DisasterLayer final : public cFC4ISC4DisasterLayer, public cIGZMessageTarget2 {
public:
    cFC4DisasterLayer();
    ~cFC4DisasterLayer();

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    bool Init() override;
    bool Shutdown() override;
    bool EnableDisasters(bool enable) override;
    bool DisastersEnabled() override;
    cISC4DisasterManager* GetDisasterManager(uint32_t type) override;
    bool DisasterHasBegun(cISC4DisasterInstance* instance) override;
    bool DisasterHasEnded() override;
    bool IsADisasterOngoing() override;
    bool GoToNextDisaster() override;
    bool GetMostRecentDisasterLocation(cISC4DisasterInstance* instance, int& x, int& z) override;
    bool RegisterDisasterLocation(cISC4DisasterInstance* instance, int x, int z, cISC4Occupant* occupant, int32_t zoom,
                                  bool focus) override;
    bool UnregisterDisasterLocations(cISC4DisasterInstance* instance) override;
    bool UnregisterDisasterLocationByOccupant(cISC4DisasterInstance* instance, cISC4Occupant* occupant) override;
    int32_t GetDisasterLocationCount() override;
    bool GetDisasterLocationByIndex(int32_t index, cISC4DisasterInstance** instance, int& x, int& z) override;
    const void* GetDisasterCost() override;

    bool DoMessage(cIGZMessage2* msg) override;

    uint32_t RefCount() const { return refCount_.load(); }  // conformance test 23
    cISC4City* City() const;  // borrowed, from the app at each use

private:
    void RefreshRecord();

    std::atomic<uint32_t> refCount_{0};
    fc4_dlayer_host host_{};
    fc4_dlayer* core_ = nullptr;
    std::map<uint32_t, cISC4DisasterManager*> managers_;  // type -> native manager, one reference each
    FC4DisasterCostRecord record_{};
};
