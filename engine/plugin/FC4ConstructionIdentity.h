// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "cIGZUnknown.h"

class cISC4Occupant;
class cISC4Lot;

namespace fc4 {
constexpr uint32_t kConstructionIID = 0x496E7636;

// lot-records 9.3: the construction view has five slots, including Unknown.
// The SDK cISC4ConstructionOccupant declares additional slots belonging to
// other views. Never dispatch those slots through the construction view.
class ConstructionView : public cIGZUnknown {
public:
    virtual cISC4Occupant* AsOccupant() = 0; // borrowed conversion, slot 3
    virtual bool SetLot(cISC4Lot*) = 0;     // mutating operation, slot 4
};

// Separate acquisition from cleanup: a caller using SehGuard must retain the
// acquired reference outside the guarded frame, even if AsOccupant faults.
// The caller audits and releases this reference after leaving that frame.
inline bool ConstructionIdentity(cIGZUnknown* source, cISC4Occupant* expected,
                                 ConstructionView*& acquired) {
    acquired = nullptr;
    void* view = nullptr;
    if (!source || !expected || !source->QueryInterface(kConstructionIID, &view) || !view) return false;
    acquired = static_cast<ConstructionView*>(view);
    return acquired->AsOccupant() == expected;
}
} // namespace fc4
