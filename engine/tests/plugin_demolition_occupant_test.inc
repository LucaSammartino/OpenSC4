// SPDX-License-Identifier: GPL-3.0-or-later
// Lot slot 48 returns a building view; demolition's callbacks use occupants.
namespace demolition_occupant_test {
struct Occupant : cIGZUnknown {
    uint32_t refs = 1;
    bool QueryInterface(uint32_t, void** out) override { *out = nullptr; return false; }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return --refs; }
};
struct Building : cIGZUnknown {
    Occupant occupant;
    bool available = true;
    uint32_t queries = 0;
    bool QueryInterface(uint32_t iid, void** out) override {
        ++queries;
        *out = nullptr;
        if (!available || iid != 0x26BB767Eu) return false;
        *out = &occupant;
        occupant.AddRef();
        return true;
    }
    uint32_t AddRef() override { return 2; }
    uint32_t Release() override { return 1; }
};
struct Lot {
    void** vtable;
    Building* building;
};
cIGZUnknown* __fastcall GetBuilding(Lot* self, void*) { return self->building; }
}  // namespace demolition_occupant_test

void DemolitionLotBuildingTest(FC4TestSeamFn seam) {
    using namespace demolition_occupant_test;
    CHECK(seam != nullptr);
    if (!seam) return;
    void* slots[49]{};
    slots[48] = reinterpret_cast<void*>(&GetBuilding);
    Building building;
    Lot lot{slots, &building};
    CHECK(static_cast<void*>(&building) != static_cast<void*>(&building.occupant));
    CHECK(seam(kFC4SeamDemolitionLotBuilding, &lot, nullptr) == reinterpret_cast<intptr_t>(&building.occupant));
    CHECK(building.queries == 1 && building.occupant.refs == 1);
    building.available = false;
    CHECK(seam(kFC4SeamDemolitionLotBuilding, &lot, nullptr) == 0);
    CHECK(building.occupant.refs == 1);
    lot.building = nullptr;
    CHECK(seam(kFC4SeamDemolitionLotBuilding, &lot, nullptr) == 0);
    CHECK(seam(kFC4SeamDemolitionLotBuilding, nullptr, nullptr) == 0);
}
