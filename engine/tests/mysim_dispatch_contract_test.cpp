// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4MySimDispatchContract.h"
#include <cstdio>

namespace {
class Fixture final : public fc4::MySimDispatchPublic {
public:
    bool clearUnknown = true;
    bool acceptUnknown = false;
    bool allowDrive = false;
    bool exitResult = true;
    uint32_t refs = 1;
    bool QueryInterface(uint32_t, void** out) override {
        if (acceptUnknown) {
            *out = static_cast<fc4::MySimDispatchPublic*>(this);
            AddRef();
            return true;
        }
        if (clearUnknown) *out = nullptr;
        return false;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return --refs; }
    void* Delete(uint32_t) override { return nullptr; }
    uint32_t RemoveRef() override { return refs ? --refs : 0; }
    uint32_t GetRefCount() override { return refs; }
    bool CanUDriveIt() override { return allowDrive; }
    bool EnterUDriveIt() override { return allowDrive; }
    bool ExitUDriveIt() override { return exitResult; }
};
fc4::MySimDispatchContractResult CheckFixture(Fixture& fixture) {
    return fc4::CheckMySimDispatchConstructor(fixture, [](cIGZUnknown* owned) { owned->Release(); });
}
}

int main() {
    Fixture fixture;
    auto result = CheckFixture(fixture);
    if (result.fields != 12 || result.differences != 0 || fixture.refs != 1) return 1;
    fixture.acceptUnknown = true;
    result = CheckFixture(fixture);
    if (result.fields != 12 || result.differences != 0 || fixture.refs != 1) return 4;
    fixture.acceptUnknown = false;
    fixture.clearUnknown = false;
    result = CheckFixture(fixture);
    if (result.differences != 1 || result.failedMask != (1u << 9) || fixture.refs != 1) return 2;
    fixture.allowDrive = true;
    fixture.exitResult = false;
    result = CheckFixture(fixture);
    if (result.differences != 4 || result.failedMask != 0x2E0u || fixture.refs != 1) return 3;
    std::puts("MySim dispatch constructor ABI checker: positive and negative fixtures passed");
    return 0;
}
