// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4ConstructionIdentity.h"
#include <cstdio>

namespace {
// Distinct interface subobjects ensure the test catches a missing QI
// adjustment and accidental acquisition/release of the borrowed conversion.
struct Occupant : cIGZUnknown {};
struct Owner : Occupant, fc4::ConstructionView {
    uint32_t refs = 1, adds = 0, releases = 0, conversions = 0, mutations = 0;
    bool reject = false, nullView = false, wrongOccupant = false, throwConversion = false;
    cISC4Occupant* occupant() { return reinterpret_cast<cISC4Occupant*>(static_cast<Occupant*>(this)); }
    bool QueryInterface(uint32_t iid, void** out) override {
        *out = nullptr;
        if (iid != fc4::kConstructionIID || reject) return false;
        if (nullView) return true;
        *out = static_cast<fc4::ConstructionView*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { ++adds; return ++refs; }
    uint32_t Release() override { ++releases; return --refs; }
    cISC4Occupant* AsOccupant() override {
        ++conversions;
        if (throwConversion) throw 1;
        return wrongOccupant ? nullptr : occupant();
    }
    bool SetLot(cISC4Lot*) override { ++mutations; return true; }
};
int failures = 0;
#define CHECK(expr) do { if (!(expr)) { ++failures; std::printf("line %d: %s\n", __LINE__, #expr); } } while (0)
}

int main() {
    Owner owner;
    fc4::ConstructionView* acquired = nullptr;
    auto* source = static_cast<Occupant*>(&owner);
    CHECK(static_cast<cIGZUnknown*>(source) != static_cast<cIGZUnknown*>(static_cast<fc4::ConstructionView*>(&owner)));
    CHECK(fc4::ConstructionIdentity(source, owner.occupant(), acquired));
    CHECK(acquired == static_cast<fc4::ConstructionView*>(&owner));
    CHECK(owner.refs == 2 && owner.adds == 1 && owner.releases == 0);
    CHECK(owner.conversions == 1 && owner.mutations == 0);
    CHECK(acquired->Release() == 1);

    owner.wrongOccupant = true;
    CHECK(!fc4::ConstructionIdentity(source, owner.occupant(), acquired));
    CHECK(acquired != nullptr); // A comparison failure still owns the QI ref.
    CHECK(acquired->Release() == 1);

    owner.reject = true;
    CHECK(!fc4::ConstructionIdentity(source, owner.occupant(), acquired));
    CHECK(acquired == nullptr && owner.refs == 1);
    owner.reject = false;
    owner.nullView = true;
    CHECK(!fc4::ConstructionIdentity(source, owner.occupant(), acquired));
    CHECK(acquired == nullptr && owner.refs == 1);
    CHECK(!fc4::ConstructionIdentity(nullptr, owner.occupant(), acquired));
    CHECK(acquired == nullptr);
    owner.nullView = false;
    owner.throwConversion = true;
    bool caught = false;
    try {
        fc4::ConstructionIdentity(source, owner.occupant(), acquired);
    } catch (int) {
        caught = true;
    }
    CHECK(caught && acquired != nullptr && owner.refs == 2);
    if (acquired) CHECK(acquired->Release() == 1);
    CHECK(owner.adds == 3 && owner.releases == 3 && owner.conversions == 3 && owner.mutations == 0);
    std::printf("construction identity: %d failures\n", failures);
    return failures ? 1 : 0;
}
