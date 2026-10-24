// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include "cIGZUnknown.h"
#include <cstdint>

namespace fc4 {

// mysims.md §7.2.3: this is the complete primary ABI, not a trip API.
// Slot 3 is a deleting destructor. It is deliberately never invoked here.
class MySimDispatchPublic : public cIGZUnknown {
public:
    virtual void* Delete(uint32_t flags) = 0;
    virtual uint32_t RemoveRef() = 0;
    virtual uint32_t GetRefCount() = 0;
    virtual bool CanUDriveIt() = 0;
    virtual bool EnterUDriveIt() = 0;
    virtual bool ExitUDriveIt() = 0;
};

struct MySimDispatchContractResult {
    uint32_t fields = 0;
    uint32_t differences = 0;
    uint32_t failedMask = 0;
};

// Only for a fresh, ownerless factory fixture, never a live dispatch. There is
// no Init/Read, subscription, direct concrete call or foreign layout access.
inline MySimDispatchContractResult CheckMySimDispatchConstructor(
    MySimDispatchPublic& p, void (*releaseOwned)(cIGZUnknown*)) {
    MySimDispatchContractResult result;
    const auto check = [&result](bool equal) {
        if (!equal) {
            ++result.differences;
            result.failedMask |= 1u << result.fields;
        }
        ++result.fields;
    };
    const uint32_t initial = p.GetRefCount();
    check(initial != 0);
    check(p.AddRef() == initial + 1);
    check(p.GetRefCount() == initial + 1);
    // RemoveRef must not destroy, even though Release normally would.
    check(p.RemoveRef() == initial);
    check(p.GetRefCount() == initial);
    // [abi-ret] mysims.md §7.2.3 dispatch slots 6-8 return bool in AL; these are not MySim slots 64-66.
    check(!p.CanUDriveIt());
    check(!p.EnterUDriveIt());
    check(p.ExitUDriveIt());
    check(p.GetRefCount() == initial);
    void* unknown = reinterpret_cast<void*>(1);
    // SPEC-GAP: no unsupported-IID rejection policy; verify ownership on either outcome.
    const bool queried = p.QueryInterface(0xFFFFFFFFu, &unknown);
    check(queried ? unknown != nullptr : unknown == nullptr);
    check(p.GetRefCount() == initial + (queried ? 1u : 0u));
    if (queried && unknown) releaseOwned(static_cast<cIGZUnknown*>(unknown));
    check(p.GetRefCount() == initial);
    return result;
}

}  // namespace fc4
