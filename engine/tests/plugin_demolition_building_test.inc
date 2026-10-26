// SPDX-License-Identifier: GPL-3.0-or-later
// Regression for the Darris, Indereast and West Weburg demolition exits.
// Lot slot 48 returns cISC4BuildingOccupant*, whose occupant is a distinct
// interface (SDK cISC4BuildingOccupant::AsOccupant, slot 3).
namespace demolition_building_test {
struct Interface {
    void** vtbl;
    void* result;
    unsigned calls = 0;
};
void* __fastcall GetResult(Interface* self, void*) {
    ++self->calls;
    return self->result;
}
} // namespace demolition_building_test

void DemolitionBuildingInterfaceChecks(FC4TestSeamFn seam) {
    using namespace demolition_building_test;
    FakeVtable lotVt, buildingVt, occupantVt;
    lotVt.slots[48] = reinterpret_cast<void*>(&GetResult);
    buildingVt.slots[3] = reinterpret_cast<void*>(&GetResult);
    FakeObject occupant{occupantVt.slots.data()};
    Interface building{buildingVt.slots.data(), &occupant};
    Interface lot{lotVt.slots.data(), &building};
    const auto token = seam(kFC4SeamDemolitionLotBuilding, &lot, nullptr);
    CHECK(token == reinterpret_cast<intptr_t>(&occupant));
    CHECK(token != reinterpret_cast<intptr_t>(&building));
    CHECK(lot.calls == 1 && building.calls == 1);
    // Empty lots and a missing lot stay null, without calling a building.
    lot.result = nullptr;
    CHECK(seam(kFC4SeamDemolitionLotBuilding, &lot, nullptr) == 0);
    CHECK(seam(kFC4SeamDemolitionLotBuilding, nullptr, nullptr) == 0);
    CHECK(lot.calls == 2 && building.calls == 1);
    // Conversion may itself return null; it is still a borrowed interface.
    lot.result = &building;
    building.result = nullptr;
    CHECK(seam(kFC4SeamDemolitionLotBuilding, &lot, nullptr) == 0);
    CHECK(lot.calls == 3 && building.calls == 2);
}
