// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "cISC4RenderProperties.h"
#include <atomic>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
// An owned renderer snapshot for read-only owner comparisons. No setters or
// reference-count operations are forwarded to the game's renderer.
class ConfigRulesSnapshot final : public cISC4RenderProperties {
    std::atomic<uint32_t> refs_{1};
    std::vector<std::string> names_[3];
    std::vector<uint32_t> values_[3];
    int32_t Find(unsigned kind, const char* name) {
        for (size_t i = 0; i < names_[kind].size(); ++i)
            if (names_[kind][i] == name) return static_cast<int32_t>(i);
        return -1;
    }
    uint32_t Get(unsigned kind, int32_t index) const {
        return index >= 0 && size_t(index) < values_[kind].size() ? values_[kind][index] : 0;
    }
    void Set(unsigned kind, int32_t index, uint32_t value) {
        if (index >= 0 && size_t(index) < values_[kind].size()) values_[kind][index] = value;
    }
    const char* Name(unsigned kind, int32_t index) {
        return index >= 0 && size_t(index) < names_[kind].size() ? names_[kind][index].c_str() : "";
    }
public:
    explicit ConfigRulesSnapshot(cISC4RenderProperties& source) {
        const int32_t counts[] = {source.NumBoolProperties(), source.NumIntProperties(), source.NumFloatProperties()};
        for (unsigned kind = 0; kind < 3; ++kind) {
            for (int32_t i = 0; i < counts[kind]; ++i) {
                const char* name = kind == 0 ? source.NameFromBoolPropertyID(i) : kind == 1
                    ? source.NameFromIntPropertyID(i) : source.NameFromFloatPropertyID(i);
                names_[kind].emplace_back(name ? name : "");
                uint32_t bits;
                if (kind == 0) bits = source.BoolValue(i);
                else if (kind == 1) bits = source.IntValue(i);
                else { const float value = source.FloatValue(i); memcpy(&bits, &value, 4); }
                values_[kind].push_back(bits);
            }
        }
    }
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out) return false;
        *out = nullptr;
        if (iid != 1) return false;
        *out = static_cast<cISC4RenderProperties*>(this); AddRef(); return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { const auto n = --refs_; if (!n) delete this; return n; }
    int32_t NumBoolProperties() override { return static_cast<int32_t>(values_[0].size()); }
    int32_t NumIntProperties() override { return static_cast<int32_t>(values_[1].size()); }
    int32_t NumFloatProperties() override { return static_cast<int32_t>(values_[2].size()); }
    bool BoolValue(int32_t i) override { return Get(0, i) != 0; }
    int32_t IntValue(int32_t i) override { return static_cast<int32_t>(Get(1, i)); }
    float FloatValue(int32_t i) override { const auto bits = Get(2, i); float v; memcpy(&v, &bits, 4); return v; }
    void SetBoolValue(int32_t i, bool v) override { Set(0, i, v); }
    void SetIntValue(int32_t i, int32_t v) override { Set(1, i, static_cast<uint32_t>(v)); }
    void SetFloatValue(int32_t i, float v) override { uint32_t bits; memcpy(&bits, &v, 4); Set(2, i, bits); }
    int32_t BoolPropertyIDFromName(const char* n) override { return Find(0, n); }
    int32_t IntPropertyIDFromName(const char* n) override { return Find(1, n); }
    int32_t FloatPropertyIDFromName(const char* n) override { return Find(2, n); }
    const char* NameFromBoolPropertyID(int32_t i) override { return Name(0, i); }
    const char* NameFromIntPropertyID(int32_t i) override { return Name(1, i); }
    const char* NameFromFloatPropertyID(int32_t i) override { return Name(2, i); }
};
}
