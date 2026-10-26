// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4ConfigRulesTrace.h"
#include "FC4Log.h"
#include "cISC4RenderProperties.h"
#include <cstring>
#include <map>
#include <mutex>
namespace fc4 {
namespace {
std::mutex traceMutex;
// Borrowed only while a method of the original manager is executing: its Init
// owns the reference. We never call through these entries outside that lifetime.
std::map<uint32_t, void*> originalProperties;
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
    } else if (strcmp(name, "cfgrules-auto") == 0) {
        Log("cfgrules query Auto group=%u result=%u original=1", args[0], result);
    } else if (strcmp(name, "cfgrules-apply") == 0) {
        Log("cfgrules query Apply group=%u option=%u original=1", args[0], args[1]);
        ConfigRulesTraceProperties(originalProperties[self]);
    } else if (strcmp(name, "cfgrules-evaluate") == 0) {
        Log("cfgrules query SetOptions original=1");
        ConfigRulesTraceProperties(originalProperties[self]);
    } else if (strcmp(name, "cfgrules-log") == 0) {
        Log("cfgrules query Log opened=%u original=1", result & 255);
    }
}
}
