// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include "cIGZUnknown.h"
#include <cstdint>

class cISC4DisasterInstance;
class cISC4DisasterManager;
class cISC4Occupant;

// disasters.md §2.8, docs/abi/interfaces.tsv: the SDK omits slots 7–10.
class cFC4ISC4DisasterLayer : public cIGZUnknown {
public:
    virtual bool Init() = 0;
    virtual bool Shutdown() = 0;
    virtual bool EnableDisasters(bool enable) = 0;
    virtual bool DisastersEnabled() = 0;
    virtual cISC4DisasterManager* GetDisasterManager(uint32_t type) = 0;
    virtual bool DisasterHasBegun(cISC4DisasterInstance* instance) = 0;
    virtual bool DisasterHasEnded() = 0;
    virtual bool IsADisasterOngoing() = 0;
    virtual bool GoToNextDisaster() = 0;
    virtual bool GetMostRecentDisasterLocation(cISC4DisasterInstance*, int32_t&, int32_t&) = 0;
    virtual bool RegisterDisasterLocation(cISC4DisasterInstance*, int32_t, int32_t, cISC4Occupant*, int32_t, bool) = 0;
    virtual bool UnregisterDisasterLocations(cISC4DisasterInstance*) = 0;
    virtual bool UnregisterDisasterLocationByOccupant(cISC4DisasterInstance*, cISC4Occupant*) = 0;
    virtual int32_t GetDisasterLocationCount() = 0;
    virtual bool GetDisasterLocationByIndex(int32_t, cISC4DisasterInstance**, int32_t&, int32_t&) = 0;
    virtual const void* GetDisasterCost() = 0;
};

cIGZUnknown* CreateFC4DisasterLayer();

// disasters.md §3.1 and §7: slot 6 takes coordinates, slot 7 takes none.
// The SDK declares these overloads in the opposite order.
template<class T> class SC4List;
class cFC4ISC4DisasterManager : public cIGZUnknown {
public:
    virtual bool Init() = 0;
    virtual bool Shutdown() = 0;
    virtual uint32_t GetID() = 0;
    virtual cISC4DisasterInstance* AttemptDisasterStart(int32_t, int32_t) = 0;
    virtual cISC4DisasterInstance* AttemptDisasterStart() = 0;
    virtual bool AddInstanceToList(cISC4DisasterInstance*) = 0;
    virtual bool RemoveInstanceFromList(cISC4DisasterInstance*) = 0;
    virtual int32_t GetActiveInstanceCount() = 0;
    virtual const SC4List<cISC4DisasterInstance*>* GetInstances() = 0;
};

#define FC4_DISASTER_FACTORIES(Name) \
    cIGZUnknown* CreateFC4DisasterManager##Name(); \
    cIGZUnknown* CreateFC4DisasterInstance##Name();
FC4_DISASTER_FACTORIES(Volcano)
FC4_DISASTER_FACTORIES(Fire)
FC4_DISASTER_FACTORIES(Earthquake)
FC4_DISASTER_FACTORIES(PipeBurst)
FC4_DISASTER_FACTORIES(Riot)
FC4_DISASTER_FACTORIES(Effect)
FC4_DISASTER_FACTORIES(Toxic)
#undef FC4_DISASTER_FACTORIES
