// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4ModelInstShadow.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Interfaces.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4RenderProperties.h"
#include "cIS3DModelInstance.h"
#include "cRZCOMDllDirector.h"
#include "fc4.h"
#include <windows.h>
#include <cstring>

// Layout: model-instances 1.2 and view-utilities 7. These definitions complete
// the SDK's opaque records; no private object state is inspected.
class cS3DTransform {
public:
    uint8_t counter, kind, reserved[2];
    float words[13];
};
class cS3DBoundingBox {
public:
    float values[6];
};
static_assert(sizeof(cS3DTransform) == 0x38, "transform ABI");
extern "C" bool fc4_modelinst_shadow_fixture(uint32_t, cS3DTransform*, cS3DTransform*, float*);

namespace {
bool active = false;
int32_t lastMonth = -1;
uint32_t lines = 0, clean = 0;

bool SupportedImage(uintptr_t base) {
    auto* fw = RZGetFrameWork();
    auto* app = fw ? fw->Application() : nullptr; // borrowed
    cISC4App* sc4 = nullptr;
    if (!app || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || !sc4) return false;
    FC4_ACQUIRED(sc4, "QueryInterface");
    auto* p = sc4->GetRenderProperties(); // borrowed, render-properties 1.4
    // Same published image guard as RenderPropsFixture. Never probe code bytes.
    const bool supported = p && reinterpret_cast<uintptr_t>(*reinterpret_cast<void***>(p)) == base + 0x006BB6C0;
    FC4_RELEASE(sc4);
    return supported;
}

void Compare(int32_t julian, bool load) {
    if (!active) return;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    const int32_t month = date.year * 12 + date.month;
    if (lastMonth == month) return;
    lastMonth = month;
    uint32_t fields = 0, mismatches = 0;
    const auto base = reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr));
    if (base && SupportedImage(base)) {
        for (uint32_t probe = 0; probe < 2; ++probe) {
            // MI14: live enumeration/ownership and missing getter contracts are
            // unavailable. This finite fixture is separate from all city objects.
            // Literal constructor/body-destructor interpretation of spec 2:
            // receiver-only thiscall; use the nondeleting body, never Release
            // embedded storage or invoke the pool delete on it.
            alignas(4) unsigned char storage[0xA4]{};
            using Lifetime = void(__thiscall*)(void*);
            reinterpret_cast<Lifetime>(base + 0x00402950)(storage);
            auto* instance = reinterpret_cast<cIS3DModelInstance*>(storage);
            const bool bound = reinterpret_cast<uintptr_t>(*reinterpret_cast<void***>(instance)) == base + 0x006BD508;
            ++fields;
            mismatches += !bound;
            if (bound) {
                cS3DTransform supplied{}, expected{}, actual{};
                cS3DBoundingBox world{}, local{};
                float expectedWorld[6]{};
                const bool transport = fc4_modelinst_shadow_fixture(probe, &supplied, &expected, expectedWorld);
                ++fields;
                mismatches += !transport;
                if (transport) {
                    if (probe != 0) instance->SetTransform(supplied); // isolated fixture only
                    actual = supplied;
                    actual.counter = 255;
                    instance->GetTransform(actual);
                    // Check every defined byte; +2/+3 are caller-owned padding.
                    fields += 54;
                    const auto* a = reinterpret_cast<const uint8_t*>(&actual);
                    const auto* e = reinterpret_cast<const uint8_t*>(&expected);
                    for (uint32_t i = 0; i < sizeof(actual); ++i) {
                        if (i != 2 && i != 3) mismatches += a[i] != e[i];
                    }
                    instance->GetBounds(local);
                    instance->GetWorldBounds(world);
                    constexpr float empty[6] = {1, 1, 1, 0, 0, 0};
                    for (uint32_t i = 0; i < 6; ++i) {
                        fields += 2;
                        mismatches += std::memcmp(&local.values[i], &empty[i], sizeof(float)) != 0;
                        mismatches += std::memcmp(&world.values[i], &expectedWorld[i], sizeof(float)) != 0;
                    }
                    fields += 4;
                    mismatches += instance->GetProperties() != 2;
                    mismatches += instance->GetOwner() != nullptr;
                    mismatches += instance->GetCustomData() != nullptr;
                    mismatches += instance->GetResModel() != nullptr;
                }
            }
            reinterpret_cast<Lifetime>(base + 0x00402620)(storage);
        }
    } else {
        ++mismatches; // no vacuous exact line on an unsupported image
    }
    fc4::Log("modelinst shadow %04d-%02d-%02d: phase=%s scope=isolated-static-transform-bounds fixtures=2 fields=%u live=unavailable(MI14) resyncs=0 mismatches=%u",
        date.year, date.month, date.day_of_month, load ? "load" : "month", fields, mismatches);
    ++lines;
    clean += mismatches == 0;
}
}

namespace fc4 {
void ModelInstShadowCityLoaded(cISC4City* city, int32_t julian) {
    active = city && SettingInt("modelinst_shadow", nullptr, 0) != 0;
    lastMonth = -1;
    lines = clean = 0;
    if (!active) return;
    Log("modelinst: ON; isolated finite static constructor/getters/transform/bounds only; no live-instance or whole-owner credit (MI14)");
    if (auto* sim = AsGame(city->GetSimulator())) julian = sim->GetSimDateNumber();
    Compare(julian, true);
}
void ModelInstShadowMonth(int32_t julian) { Compare(julian, false); }
void ModelInstShadowCityShutdown() {
    if (active) Log("modelinst shadow summary: clean=%u/%u scope=isolated-static-transform-bounds live=unavailable(MI14)", clean, lines);
    active = false;
}
}
