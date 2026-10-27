// SPDX-License-Identifier: GPL-3.0-or-later
#include "cISC4MySim.h"
#include "SC4Vector.h"

namespace mysims_ownership_test {
struct Owner;
struct View { void** vtable; Owner* owner; };
struct Owner {
    View sim, advisor;
    uint32_t refs = 1, acquisitions = 0, releases = 0;
    int32_t id;
    bool answers = true;
};
uint32_t __fastcall AddRef(View* self, void*) {
    ++self->owner->acquisitions;
    return ++self->owner->refs;
}
uint32_t __fastcall Release(View* self, void*) {
    ++self->owner->releases;
    return --self->owner->refs;
}
bool __fastcall Query(View* self, void*, uint32_t iid, void** out) {
    *out = nullptr;
    if (!self->owner->answers || iid != 0xCA09F544u) return false;
    *out = &self->owner->advisor;
    AddRef(&self->owner->advisor, nullptr);
    return true;
}
int32_t __fastcall GetID(View* self, void*) { return self->owner->id; }
struct Allocator {
    static bool Available() { return true; }
    static void* Allocate(uint32_t size) { return std::malloc(size); }
    static void Free(void* p) { std::free(p); }
};
struct Modal { void** vtable; bool blocked = false; unsigned calls = 0; };
bool __fastcall IsModal(Modal* self, void*) {
    ++self->calls;
    return self->blocked;
}

struct SavedOwner;
struct SavedView { void** vtable; SavedOwner* owner; };
struct SavedOwner {
    SavedView serializable, building, occupant, dispatch;
    uint32_t refs = 1, acquisitions = 0, releases = 0;
    bool hasBuilding = true, hasOccupant = true;
};
uint32_t __fastcall SavedAddRef(SavedView* self, void*) {
    ++self->owner->acquisitions;
    return ++self->owner->refs;
}
uint32_t __fastcall SavedRelease(SavedView* self, void*) {
    ++self->owner->releases;
    return --self->owner->refs;
}
bool __fastcall SavedQuery(SavedView* self, void*, uint32_t iid, void** out) {
    *out = nullptr;
    SavedOwner& owner = *self->owner;
    if (iid == 0x87DDA3A9u && owner.hasBuilding) *out = &owner.building;
    if (iid == 0x26BB767Eu && owner.hasOccupant) *out = &owner.occupant;
    if (iid == 0xEB8B6F9Au) *out = &owner.dispatch;
    if (*out == nullptr) return false;
    SavedAddRef(static_cast<SavedView*>(*out), nullptr);
    return true;
}
uint32_t __fastcall SavedClass(SavedView*, void*) { return 0xA9BD882Du; }

void SavedReferenceChecks(FC4TestSeamFn seam) {
    FakeVtable vt;
    vt.slots[0] = reinterpret_cast<void*>(&SavedQuery);
    vt.slots[1] = reinterpret_cast<void*>(&SavedAddRef);
    vt.slots[2] = reinterpret_cast<void*>(&SavedRelease);
    vt.slots[5] = reinterpret_cast<void*>(&SavedClass);
    SavedOwner owner{{vt.slots.data(), nullptr}, {vt.slots.data(), nullptr},
                     {vt.slots.data(), nullptr}, {vt.slots.data(), nullptr}};
    owner.serializable.owner = owner.building.owner = owner.occupant.owner = owner.dispatch.owner = &owner;
    uint32_t iid = 0x87DDA3A9u;
    // The real InjectStream must normalize the saved Serializable/Building
    // views to the Occupant identity used by building-removal messages.
    CHECK(seam(kFC4SeamMySimsSavedReference, &owner.serializable, &iid) ==
          reinterpret_cast<intptr_t>(&owner.occupant));
    CHECK(owner.refs == 1 && owner.acquisitions == 2 && owner.releases == 2);
    owner.hasBuilding = false;
    CHECK(seam(kFC4SeamMySimsSavedReference, &owner.serializable, &iid) == 0);
    CHECK(owner.refs == 1 && owner.acquisitions == 2 && owner.releases == 2);
    owner.hasBuilding = true;
    owner.hasOccupant = false;
    CHECK(seam(kFC4SeamMySimsSavedReference, &owner.serializable, &iid) == 0);
    CHECK(owner.refs == 1 && owner.acquisitions == 3 && owner.releases == 3);
    iid = 0xEB8B6F9Au;
    CHECK(seam(kFC4SeamMySimsSavedReference, &owner.serializable, &iid) ==
          reinterpret_cast<intptr_t>(&owner.dispatch));
    CHECK(owner.refs == 1 && owner.acquisitions == 4 && owner.releases == 4);
    iid = 0xFFFFFFFFu;
    CHECK(seam(kFC4SeamMySimsSavedReference, &owner.serializable, &iid) == 0);
    CHECK(seam(kFC4SeamMySimsSavedReference, nullptr, &iid) == 0);
    CHECK(owner.refs == 1 && owner.acquisitions == 4 && owner.releases == 4);
}
} // namespace mysims_ownership_test

void MySimsActiveOwnershipChecks(FC4TestSeamFn seam) {
    using namespace mysims_ownership_test;
    FakeVtable simVt, advisorVt;
    simVt.slots[0] = advisorVt.slots[0] = reinterpret_cast<void*>(&Query);
    simVt.slots[1] = advisorVt.slots[1] = reinterpret_cast<void*>(&AddRef);
    simVt.slots[2] = advisorVt.slots[2] = reinterpret_cast<void*>(&Release);
    advisorVt.slots[5] = reinterpret_cast<void*>(&GetID);
    Owner first{{simVt.slots.data(), nullptr}, {advisorVt.slots.data(), nullptr}, 1, 0, 0, 3};
    Owner refused{{simVt.slots.data(), nullptr}, {advisorVt.slots.data(), nullptr}, 1, 0, 0, 4, false};
    first.sim.owner = first.advisor.owner = &first;
    refused.sim.owner = refused.advisor.owner = &refused;
    {
        fc4::GameVector<cISC4MySim*, Allocator> values;
        // Each entry supplies a query-owned reference, even repeated pointers.
        AddRef(&first.sim, nullptr);
        CHECK(values.push_back(reinterpret_cast<cISC4MySim*>(&first.sim)));
        CHECK(values.push_back(nullptr));
        AddRef(&first.sim, nullptr);
        CHECK(values.push_back(reinterpret_cast<cISC4MySim*>(&first.sim)));
        AddRef(&refused.sim, nullptr);
        CHECK(values.push_back(reinterpret_cast<cISC4MySim*>(&refused.sim)));
        int32_t ids[5] = {-1, -1, -1, -1, -1};
        CHECK(seam(kFC4SeamMySimsActiveIds, &values, ids) == 0);
        CHECK(ids[0] == 3 && ids[1] == 0 && ids[2] == 3 && ids[3] == 0 && ids[4] == -1);
        for (auto* value : values) CHECK(value == nullptr);
        CHECK(first.refs == 1 && first.acquisitions == 4 && first.releases == 4);
        CHECK(refused.refs == 1 && refused.acquisitions == 1 && refused.releases == 1);
        // Run the actual SDK's element cleanup over the ABI-compatible buffer.
        // Before the fix this released the query references a second time.
        reinterpret_cast<SC4Vector<cISC4MySim*>*>(&values)->clear();
        CHECK(first.refs == 1 && first.releases == 4 && refused.refs == 1 && refused.releases == 1);
        CHECK(seam(kFC4SeamMySimsActiveIds, &values, ids) == 0); // empty vector
    }
    CHECK(first.refs == 1 && refused.refs == 1);
    FakeVtable managerVt;
    managerVt.slots[42] = reinterpret_cast<void*>(&IsModal);
    Modal manager{managerVt.slots.data()};
    CHECK(seam(kFC4SeamMySimsUiBlocked, &manager, nullptr) == 0);
    manager.blocked = true;
    CHECK(seam(kFC4SeamMySimsUiBlocked, &manager, nullptr) == 1);
    CHECK(manager.calls == 2);
    CHECK(seam(kFC4SeamMySimsUiBlocked, nullptr, nullptr) == 0);
    SavedReferenceChecks(seam);
}
