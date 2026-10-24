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
#include <atomic>
#include <mutex>

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
extern "C" uint32_t fc4_modelinst_shadow_destination(const cS3DTransform*, const cS3DTransform*);

namespace {
bool active = false;
int32_t lastMonth = -1;
uint32_t lines = 0, clean = 0;
// Portable instrumentation choice: first 4096 naturally encountered calls per
// city, not an enumeration capacity or native owner limit (spec 9.2 / MI14).
constexpr uint32_t liveCap = 4096;
std::atomic<bool> liveEnabled{false};
std::mutex liveMutex;
uint64_t encountered = 0, captured = 0, liveBad = 0;
uint32_t liveLines = 0, liveClean = 0;
void** liveEntry = nullptr;
using GetTransform = void(__thiscall*)(void*, cS3DTransform*);
GetTransform liveOriginal = nullptr;
thread_local bool fixtureCall = false;

void __fastcall ObserveTransform(void* self, void*, cS3DTransform* output) {
    bool sample = false;
    if (!fixtureCall && liveEnabled.load() && output) {
        std::lock_guard<std::mutex> lock(liveMutex);
        sample = ++encountered <= liveCap;
    }
    cS3DTransform before{};
    if (sample) std::memcpy(&before, output, sizeof(before));
    liveOriginal(self, output); // original thunk, exactly once; preserves receiver adjustment
    if (sample) {
        cS3DTransform after{};
        std::memcpy(&after, output, sizeof(after));
        const uint32_t bad = fc4_modelinst_shadow_destination(&before, &after);
        std::lock_guard<std::mutex> lock(liveMutex);
        ++captured;
        liveBad += bad;
    }
}

bool ReplaceLive(void* expected, void* value) {
    DWORD before = 0;
    if (!VirtualProtect(liveEntry, sizeof(void*), PAGE_READWRITE, &before)) return false;
    const bool replaced = InterlockedCompareExchangePointer(
        reinterpret_cast<void* volatile*>(liveEntry), value, expected) == expected;
    DWORD ignored = 0;
    VirtualProtect(liveEntry, sizeof(void*), before, &ignored);
    return replaced;
}

void LiveReport(const fc4_date& date) {
    uint64_t seen, count, bad;
    {
        std::lock_guard<std::mutex> lock(liveMutex);
        seen = encountered; count = captured; bad = liveBad;
    }
    // A city-load point can precede any render call. Do not issue a vacuous
    // exact shadow line until at least one real game call has been captured.
    if (!count) return;
    fc4::Log("modelinst-live shadow %04d-%02d-%02d: scope=static-gettransform-destination owner=cS3DModelInstance slot=9 interval=city-first-%u encountered=%llu captured=%llu dropped=%llu copy=56+56 fields=%llu source_words=unmeasured resyncs=0 mismatches=%llu",
        date.year, date.month, date.day_of_month, liveCap, seen, count,
        seen > liveCap ? seen - liveCap : 0, count * 3, bad);
    ++liveLines;
    liveClean += bad == 0;
}

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
    fixtureCall = true;
    const auto base = reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr));
    if (base && SupportedImage(base)) {
        for (uint32_t probe = 0; probe < 2; ++probe) {
            // MI14 / spec 2.2: standalone construction requires flag 1; its
            // instance interface is at +0xA0, not the container at +0. Use the
            // receiver-only nondeleting body on the allocation for cleanup.
            // These offsets apply only to this separate complete static fixture.
            alignas(4) unsigned char storage[0xA4]{};
            using Construct = void*(__thiscall*)(void*, uint32_t);
            using Lifetime = void(__thiscall*)(void*);
            reinterpret_cast<Construct>(base + 0x00402950)(storage, 1);
            auto* instance = reinterpret_cast<cIS3DModelInstance*>(storage + 0xA0);
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
    fixtureCall = false;
    fc4::Log("modelinst shadow %04d-%02d-%02d: phase=%s scope=isolated-static-transform-bounds fixtures=2 fields=%u live=separate resyncs=0 mismatches=%u",
        date.year, date.month, date.day_of_month, load ? "load" : "month", fields, mismatches);
    ++lines;
    clean += mismatches == 0;
    LiveReport(date);
}
}

namespace fc4 {
void ModelInstShadowCityLoaded(cISC4City* city, int32_t julian) {
    active = city && SettingInt("modelinst_shadow", nullptr, 0) != 0;
    lastMonth = -1;
    lines = clean = 0;
    {
        std::lock_guard<std::mutex> lock(liveMutex);
        encountered = captured = liveBad = 0;
    }
    liveLines = liveClean = 0;
    if (!active) return;
    Log("modelinst: ON; isolated finite static constructor/getters/transform/bounds; separate live destination-counter/padding observer; no whole-owner credit; MI14 binding applied");
    if (auto* sim = AsGame(city->GetSimulator())) julian = sim->GetSimDateNumber();
    Compare(julian, true);
    const auto base = reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr));
    if (base && SupportedImage(base)) {
        // Only the published static instance interface, including its thunk.
        // No guessed animated/private receivers and no added live getter calls.
        liveEntry = reinterpret_cast<void**>(base + 0x006BD508 + 9 * sizeof(void*));
        if (*liveEntry == reinterpret_cast<void*>(&ObserveTransform)) {
            // A failed prior restoration must never make our forwarder its own
            // original on a subsequent city load.
            liveEnabled.store(liveOriginal != nullptr);
        } else {
            liveOriginal = reinterpret_cast<GetTransform>(*liveEntry);
            liveEnabled.store(ReplaceLive(reinterpret_cast<void*>(liveOriginal),
                reinterpret_cast<void*>(&ObserveTransform)));
        }
        Log("modelinst live: static slot-9 observer %s; first %u calls/city; destination counter/padding only",
            liveEnabled.load() ? "ON" : "REFUSED", liveCap);
    }
}
void ModelInstShadowMonth(int32_t julian) { Compare(julian, false); }
void ModelInstShadowCityShutdown() {
    if (liveEnabled.exchange(false) && !ReplaceLive(reinterpret_cast<void*>(&ObserveTransform),
        reinterpret_cast<void*>(liveOriginal))) Log("modelinst live: slot-9 restoration refused");
    if (active) Log("modelinst shadow summary: clean=%u/%u scope=isolated-static-transform-bounds live=separate", clean, lines);
    if (active) Log("modelinst live summary: clean=%u/%u scope=static-gettransform-destination", liveClean, liveLines);
    active = false;
}
}
