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
struct Occupant {
    void** vtbl;
    uint32_t refs = 1, acquisitions = 0, releases = 0;
};
bool __fastcall Query(Occupant*, void*, uint32_t, void** out) {
    if (out) *out = nullptr;
    return false;
}
uint32_t __fastcall AddRef(Occupant* self, void*) {
    ++self->acquisitions;
    return ++self->refs;
}
uint32_t __fastcall Release(Occupant* self, void*) {
    ++self->releases;
    return --self->refs;
}
struct Allocator {
    static bool Available() { return true; }
    static void* Allocate(uint32_t size) { return std::malloc(size); }
    static void Free(void* p) { std::free(p); }
};
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
    // A query-supplied reference is registered before release, without an
    // extra AddRef or a false over-release warning in the audit.
    occupantVt.slots[0] = reinterpret_cast<void*>(&Query);
    occupantVt.slots[1] = reinterpret_cast<void*>(&AddRef);
    occupantVt.slots[2] = reinterpret_cast<void*>(&Release);
    Occupant queried{occupantVt.slots.data()};
    {
        // SDK-compatible layout; this test allocator needs no game framework.
        // Its destructor, like SC4List's, releases non-null pointer values.
        fc4::GameList<cISC4Occupant*, Allocator> list;
        AddRef(&queried, nullptr); // one owned reference supplied by a query
        CHECK(list.push_back(reinterpret_cast<cISC4Occupant*>(&queried)));
        CHECK(list.push_back(nullptr));
        CHECK(seam(kFC4SeamDemolitionLotOccupants, &list, nullptr) == 0);
        for (auto* value : list) CHECK(value == nullptr);
    }
    CHECK(queried.refs == 1 && queried.acquisitions == 1 && queried.releases == 1);
}
