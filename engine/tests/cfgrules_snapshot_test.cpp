// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4ConfigRulesSnapshot.h"
#include <cassert>
#include <cstring>

struct Live final : cISC4RenderProperties {
    unsigned refs = 0, writes = 0;
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return --refs; }
    int32_t NumBoolProperties() override { return 1; }
    int32_t NumIntProperties() override { return 1; }
    int32_t NumFloatProperties() override { return 1; }
    bool BoolValue(int32_t) override { return true; }
    int32_t IntValue(int32_t) override { return -123; }
    float FloatValue(int32_t) override { uint32_t bits = 0x80000000; float v; memcpy(&v, &bits, 4); return v; }
    void SetBoolValue(int32_t, bool) override { ++writes; }
    void SetIntValue(int32_t, int32_t) override { ++writes; }
    void SetFloatValue(int32_t, float) override { ++writes; }
    int32_t BoolPropertyIDFromName(const char* name) override {
        return strcmp(name, "Flag") == 0 || strcmp(name, "FLAG") == 0 || strcmp(name, "f") == 0 ? 0 : -1;
    }
    int32_t IntPropertyIDFromName(const char*) override { return 0; }
    int32_t FloatPropertyIDFromName(const char*) override { return 0; }
    const char* NameFromBoolPropertyID(int32_t) override { return "Flag"; }
    const char* NameFromIntPropertyID(int32_t) override { return "Signed"; }
    const char* NameFromFloatPropertyID(int32_t) override { return "Scale"; }
};
int main() {
    Live live;
    auto* copy = new fc4::ConfigRulesSnapshot(live);
    assert(copy->BoolValue(0) && copy->IntValue(0) == -123);
    const float value = copy->FloatValue(0);
    uint32_t bits; memcpy(&bits, &value, 4); assert(bits == 0x80000000);
    assert(copy->BoolPropertyIDFromName("Flag") == 0 && copy->BoolPropertyIDFromName("missing") == -1);
    assert(copy->BoolPropertyIDFromName("FLAG") == 0 && copy->BoolPropertyIDFromName("f") == 0);
    copy->SetBoolValue(0, false); copy->SetIntValue(0, 9); copy->SetFloatValue(0, 1.25f);
    assert(!copy->BoolValue(0) && copy->IntValue(0) == 9 && copy->FloatValue(0) == 1.25f);
    copy->SetIntValue(-1, 7); copy->SetIntValue(1, 7);
    assert(copy->IntValue(0) == 9 && live.writes == 0 && live.refs == 0);
    assert(copy->Release() == 0);
}
