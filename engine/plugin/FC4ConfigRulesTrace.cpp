// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4ConfigRulesTrace.h"
#include "FC4Log.h"
#include "FC4Config.h"
#include "FC4RefAudit.h"
#include "FC4ConfigRulesSnapshot.h"
#include "cFC4ConfigRules.h"
#include "cISC4RenderProperties.h"
#include <cstring>
#include <map>
#include <memory>
#include <mutex>
namespace fc4 {
namespace {
std::mutex traceMutex;
// Borrowed only while a method of the original manager is executing: its Init
// owns the reference. We never call through these entries outside that lifetime.
std::map<uint32_t, void*> originalProperties;
struct Shadow {
    ConfigRulesSnapshot* properties;
    cFC4ISC4ConfigRules* owner;
    std::map<uint32_t, uint32_t> recommendations;
    explicit Shadow(cISC4RenderProperties& source) : properties(new ConfigRulesSnapshot(source)),
        owner(static_cast<cFC4ISC4ConfigRules*>(CreateFC4ConfigRulesShadow())) {
        FC4_ACQUIRED(properties, "cfgrules shadow snapshot created");
        owner->AddRef(); FC4_ACQUIRED(owner, "cfgrules shadow owner created");
        owner->Init(properties);
    }
    ~Shadow() { FC4_RELEASE_CREATED(owner); FC4_RELEASE_CREATED(properties); }
    void Compare(const char* phase, cISC4RenderProperties* native) {
        unsigned total = 0, different = 0, autoDifferent = 0;
        const int counts[] = {native->NumBoolProperties(), native->NumIntProperties(), native->NumFloatProperties()};
        for (int k = 0; k < 3; ++k) for (int i = 0; i < counts[k]; ++i) {
            uint32_t expected = 0, actual = 0;
            if (k == 0) { expected = native->BoolValue(i); actual = properties->BoolValue(i); }
            else if (k == 1) { expected = native->IntValue(i); actual = properties->IntValue(i); }
            else {
                const float a = native->FloatValue(i), b = properties->FloatValue(i);
                memcpy(&expected, &a, 4); memcpy(&actual, &b, 4);
            }
            ++total;
            if (expected != actual) {
                ++different;
                Log("cfgrules shadow difference phase=%s kind=%d index=%d expected=%08x actual=%08x", phase, k, i, expected, actual);
            }
        }
        for (const auto& entry : recommendations) {
            const auto actual = owner->GetOptionGroupAutoSetting(entry.first);
            if (actual != entry.second) {
                ++autoDifferent;
                Log("cfgrules shadow recommendation group=%u expected=%u actual=%u", entry.first, entry.second, actual);
            }
        }
        Log("cfgrules shadow phase=%s properties=%u property_different=%u recommendations=%u recommendation_different=%u",
            phase, total, different, static_cast<unsigned>(recommendations.size()), autoDifferent);
    }
};
std::map<uint32_t, std::unique_ptr<Shadow>> shadows;
}
void ConfigRulesTraceProperties(void* properties) {
    auto* p = static_cast<cISC4RenderProperties*>(properties);
    if (!p) return;
    for (int k = 0; k < 3; ++k) {
        const int count = k == 0 ? p->NumBoolProperties() : k == 1 ? p->NumIntProperties() : p->NumFloatProperties();
        for (int i = 0; i < count; ++i) {
            uint32_t bits = 0;
            if (k == 0) bits = p->BoolValue(i);
            else if (k == 1) bits = p->IntValue(i);
            else { const float value = p->FloatValue(i); memcpy(&bits, &value, 4); }
            Log("cfgrules property kind=%d index=%d bits=%08x", k, i, bits);
        }
    }
}
void ConfigRulesTraceOriginal(const char* name, uint32_t self, const uint32_t* args, uint32_t result) {
    if (strncmp(name, "cfgrules-", 9) != 0) return;
    std::lock_guard<std::mutex> lock(traceMutex);
    if (strcmp(name, "cfgrules-init") == 0) {
        originalProperties[self] = reinterpret_cast<void*>(args[0]);
        Log("cfgrules query Init original=1");
        if (args[0] && SettingInt("cfgrules_shadow", "FC4_CFGRULES_SHADOW", 0))
            shadows[self] = std::make_unique<Shadow>(*reinterpret_cast<cISC4RenderProperties*>(args[0]));
    } else if (strcmp(name, "cfgrules-auto") == 0) {
        Log("cfgrules query Auto group=%u result=%u original=1", args[0], result);
        const auto it = shadows.find(self);
        if (it != shadows.end()) it->second->recommendations[args[0]] = result;
    } else if (strcmp(name, "cfgrules-apply") == 0) {
        Log("cfgrules query Apply group=%u option=%u original=1", args[0], args[1]);
        ConfigRulesTraceProperties(originalProperties[self]);
        const auto it = shadows.find(self);
        if (it != shadows.end()) {
            it->second->owner->ApplyOptionGroupSetting(args[0], args[1]);
            it->second->Compare("Apply", static_cast<cISC4RenderProperties*>(originalProperties[self]));
        }
    } else if (strcmp(name, "cfgrules-evaluate") == 0) {
        Log("cfgrules query SetOptions original=1");
        ConfigRulesTraceProperties(originalProperties[self]);
        const auto it = shadows.find(self);
        if (it != shadows.end()) {
            it->second->owner->SetOptionsFromConfiguration();
            it->second->Compare("SetOptions", static_cast<cISC4RenderProperties*>(originalProperties[self]));
        }
    } else if (strcmp(name, "cfgrules-log") == 0) {
        Log("cfgrules query Log opened=%u original=1", result & 255);
    }
}
}
