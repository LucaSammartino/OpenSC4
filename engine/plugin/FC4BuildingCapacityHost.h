// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include "FC4DevelopmentInterfaces.h"
#include "FC4RefAudit.h"
#include "cISC4City.h"

namespace fc4 {
// lot-configuration §4.1: city slot 53 lends its pointer. QI owns one
// reference; slot 39 lends catalogue storage, which must never be released.
inline uint16_t NativeBuildingCapacity(cISC4City* city, uint32_t type) {
    auto* borrowed = city ? reinterpret_cast<cIGZUnknown*>(city->GetBuildingDevelopmentSimulator()) : nullptr;
    void* acquired = nullptr;
    if (borrowed == nullptr || !borrowed->QueryInterface(0x07813EAE, &acquired) || acquired == nullptr) return 0;
    auto* bds = static_cast<cFC4ISC4BuildingDevelopmentSimulator*>(acquired);
    FC4_ACQUIRED(reinterpret_cast<cIGZUnknown*>(bds), "QueryInterface");
    const uint16_t capacity = bds->GetBuildingProfile(type).capacity; // +8, copied before Release
    FC4_RELEASE(reinterpret_cast<cIGZUnknown*>(bds));
    return capacity;
}
} // namespace fc4
