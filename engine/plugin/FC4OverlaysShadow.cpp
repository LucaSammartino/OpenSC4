// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4OverlaysShadow.h"
#include "FC4OverlaysObservation.h"
#include "FC4OverlayOwner.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4Watchdog.h"
#include "fc4.h"
#include "cISC4City.h"
#include "cISTETerrain.h"
#include "cISTETerrainView.h"
#include "cISTEOverlayManager.h"

#include <array>
#include <cstring>

// Private static-library transport; deliberately absent from fc4.h / DLL exports.
extern "C" uint32_t fc4_overlays_verify_contract(uint32_t* out, uint32_t count);

namespace fc4 {
namespace {
bool g_enabled = false;
bool g_loadPending = false;
uint32_t g_lines = 0, g_clean = 0;
uint64_t g_fields = 0, g_differences = 0;

uint32_t Bits(float value) {
    uint32_t bits;
    std::memcpy(&bits, &value, sizeof(bits));
    return bits;
}

std::array<uint32_t, 25> ReadContract(cISTEOverlayManager* manager) {
    std::array<uint32_t, 25> words{};
    const uint32_t none = UINT32_MAX;
    words[0] = manager->OverlayIsEnabled(none) ? 1 : 0;
    words[1] = Bits(manager->OverlayAlpha(none));
    // overlay-manager.md §2.3: SDK slot24 has the wrong argument order and
    // return type; use the documented output-first thiscall, not OverlayColor.
    auto** vtable = *reinterpret_cast<void***>(manager);
    using Color = float* (__thiscall*)(void*, float*, uint32_t);
    float color[3] = {0.75f, 0.75f, 0.75f};
    reinterpret_cast<Color>(vtable[24])(manager, color, none);
    for (size_t i = 0; i < 3; ++i) words[2 + i] = Bits(color[i]);
    words[5] = manager->OverlayDrawMode(none);
    // Slot26 returns a boolean (§2.3), not the SDK's uint32_t flags.
    using SelfLit = bool (__thiscall*)(void*, uint32_t);
    words[6] = reinterpret_cast<SelfLit>(vtable[26])(manager, none) ? 1 : 0;
    static_assert(sizeof(cISTEOverlayManager::cDecalInfo) == 9 * sizeof(uint32_t), "placement ABI");
    for (uint32_t probe = 0; probe < 2; ++probe) {
        uint32_t input[9];
        for (uint32_t i = 0; i < 9; ++i) input[i] = 0x3e800000 + probe * 16 + i;
        cISTEOverlayManager::cDecalInfo placement;
        std::memcpy(&placement, input, sizeof(input));
        // Both -1 and direct IDs leave output untouched, independent of arenas.
        manager->DecalInfo(probe == 0 ? none : 0, &placement);
        std::memcpy(words.data() + 7 + probe * 9, &placement, sizeof(input));
    }
    return words;
}

void Sample(cISC4City* city, const char* phase, int32_t julian) {
    if (!g_enabled) return;
    std::array<uint32_t, 25> expected{};
    const bool coreOK = fc4_overlays_verify_contract(expected.data(), 25) != 0;
    // All pointers are borrowed for this synchronous event. No AddRef/Release,
    // writes, render callbacks or native overlay allocation are performed.
    auto* terrain = city ? city->GetTerrain() : nullptr;
    auto* view = terrain ? terrain->GetView() : nullptr;
    uint32_t differences = 0, managers = 0;
    for (uint32_t type = 0; type < 4; ++type) {
        auto* manager = view ? view->GetOverlayManager(static_cast<cISTETerrainView::tOverlayManagerType>(type)) : nullptr;
        OverlaysObservationSample(manager, type, phase, julian, SettingInt("overlays_fixture", nullptr, 0) != 0);
        if (!manager || !coreOK) {
            differences += 25;  // Unavailable comparisons never count as exact.
            continue;
        }
        ++managers;
        const auto actual = ReadContract(manager);
        for (size_t field = 0; field < actual.size(); ++field) {
            if (actual[field] == expected[field]) continue;
            ++differences;
            Log("overlays contract diff: type=%u field=%u original=%08x core=%08x",
                type, static_cast<unsigned>(field), actual[field], expected[field]);
        }
    }
    ++g_lines;
    if (differences == 0) ++g_clean;
    g_fields += 100;
    g_differences += differences;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    Log("overlays shadow %04d-%02d-%02d contract phase=%s: %u/100 mismatches=%u managers=%u julian=%d scope=sentinels",
        date.year, date.month, date.day_of_month, phase, 100 - differences, differences, managers, julian);
}
}

void OverlaysShadowCityLoaded(cISC4City* city) {
    g_enabled = SettingInt("overlays_shadow", "FC4_OVERLAYS_SHADOW", 0) != 0;
    g_lines = g_clean = 0;
    g_fields = g_differences = 0;
    OverlaysObservationReset();
    // terrain.md §15.5.6: GetView can be null before view initialization.
    // Measured PostCityInit ordering exposes that state; defer the load probe
    // until the first framework tick with all four managers, outside city Init.
    g_loadPending = g_enabled;
    (void)city;
}
void OverlaysShadowOnTick(cISC4City* city) {
    if (!g_enabled || !g_loadPending) return;
    auto* terrain = city ? city->GetTerrain() : nullptr;
    auto* view = terrain ? terrain->GetView() : nullptr;
    if (!view) return;
    for (uint32_t type = 0; type < 4; ++type) {
        if (!view->GetOverlayManager(static_cast<cISTETerrainView::tOverlayManagerType>(type))) return;
    }
    g_loadPending = false;
    OverlayOwnerGameCheck();
    Sample(city, "load", static_cast<int32_t>(PublishedJulian()));
}
void OverlaysShadowMonth(cISC4City* city, int32_t julian) {
    OverlaysShadowOnTick(city);
    Sample(city, "month", julian);
}
void OverlaysShadowCityShutdown() {
    if (g_enabled) {
        // A city that never exposes its managers must not yield a vacuous pass.
        if (g_loadPending) Sample(nullptr, "load-unavailable", static_cast<int32_t>(PublishedJulian()));
        OverlaysObservationSummary();
        Log("overlays contract summary: %u/%u lines exact mismatches=%llu fields=%llu scope=sentinels",
            g_clean, g_lines, static_cast<unsigned long long>(g_differences), static_cast<unsigned long long>(g_fields));
    }
    g_enabled = false;
    g_loadPending = false;
}
}
