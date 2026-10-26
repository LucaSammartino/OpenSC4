// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

class cISC4Lot;

namespace fc4 {
// lot-developer 6.7.4 / network-manager's native Shutdown contract: SetLot initializes before it
// checks the building. Its early return leaves no owned dust effect, so that
// partial object must never enter the occupant manager's Shutdown path.
template<class Lot, class Create>
auto CreateConfiguredConstruction(Lot* lot, Create create) -> decltype(create()) {
    // SPEC-GAP: missing-building recovery is unspecified; skip the scene before Init.
    if (!lot || !lot->GetBuilding()) return nullptr;
    auto* construction = create();
    if (construction) construction->SetLot(reinterpret_cast<cISC4Lot*>(lot));
    return construction;
}
} // namespace fc4
