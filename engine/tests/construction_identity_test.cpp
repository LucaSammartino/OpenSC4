// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4ConstructionIdentity.h"
#include "FC4ConstructionSetup.h"
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

// An Init-only native construction subscribes to callbacks but has no dust
// effect. Exercise the creation boundary, including a rejected lot, without
// ever publishing that partial native state to the manager's shutdown drain.
struct SetupLot {
    void* building = nullptr;
    void* GetBuilding() const { return building; }
};
struct SetupConstruction {
    uint32_t setups = 0, dustRefs = 0, dustReleases = 0, shutdowns = 0;
    void SetLot(cISC4Lot* lot) {
        if (!reinterpret_cast<SetupLot*>(lot)->GetBuilding()) return;
        ++setups;
        ++dustRefs; // native SetLot creates and retains one dust effect
    }
    void Shutdown() {
        CHECK(setups == 1 && dustRefs == 1);
        ++shutdowns;
        --dustRefs;
        ++dustReleases;
    }
};
void SetupBoundary() {
    SetupLot lot;
    SetupConstruction construction;
    uint32_t creates = 0;
    auto create = [&]() { ++creates; return &construction; };
    CHECK(fc4::CreateConfiguredConstruction(static_cast<SetupLot*>(nullptr), create) == nullptr);
    CHECK(fc4::CreateConfiguredConstruction(&lot, create) == nullptr);
    CHECK(creates == 0 && construction.setups == 0 && construction.shutdowns == 0);
    lot.building = &lot;
    CHECK(fc4::CreateConfiguredConstruction(&lot, []() -> SetupConstruction* { return nullptr; }) == nullptr);
    CHECK(fc4::CreateConfiguredConstruction(&lot, create) == &construction);
    CHECK(creates == 1 && construction.setups == 1 && construction.dustRefs == 1);
    // Bulk teardown clears the optional prop-manager cache; that does not
    // release the construction-owned dust reference ahead of native Shutdown.
    construction.Shutdown();
    CHECK(construction.shutdowns == 1 && construction.dustRefs == 0 && construction.dustReleases == 1);
}
}

int main() {
    SetupBoundary();
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
    // A broken conversion and a missing expected identity must not compare
    // equal merely because both pointers are null. No QI is needed here.
    CHECK(!fc4::ConstructionIdentity(source, nullptr, acquired));
    CHECK(acquired == nullptr && owner.refs == 1 && owner.adds == 1 && owner.conversions == 1);
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
    CHECK(!fc4::ConstructionIdentity(source, nullptr, acquired));
    CHECK(acquired == nullptr && owner.refs == 1);
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
