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
