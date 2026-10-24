// SPDX-License-Identifier: GPL-3.0-or-later
// lighting.md §§1.1/1.4/5.6. Native owner stays running; only const getters.
#include "FC4LightingShadow.h"
#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "compat/FC4Compat.h"
#include "cISC4City.h"
#include "cISC4LightingManager.h"
#include "cS3DVector2.h"
#include "cS3DVector3.h"
#include "fc4.h"
#include <windows.h>
#include <cstring>

namespace {
constexpr uint32_t kFields = 14;
extern "C" uint32_t fc4_lighting_shadow_compare(const uint32_t*, uint32_t*);
bool enabled = false;
int32_t lastMonth = -1;
uint32_t lines = 0, clean = 0;

uint32_t Bits(float value) {
    uint32_t result;
    std::memcpy(&result, &value, sizeof(result));
    return result;
}

cISC4LightingManager* NativeOwner() {
    // §0 publishes the owner global; no guessed renderer accessor ABI (LT14).
    // Borrow only for this capture. Never AddRef/Release an unacquired pointer.
    const auto* version = fc4::compat::IdentifiedVersion();
    if (!version || version->imageBase != 0x00400000 ||
        reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr)) != 0x00400000) return nullptr;
    auto* owner = *reinterpret_cast<cISC4LightingManager**>(0x00B43DDC);
    if (!owner || *reinterpret_cast<uintptr_t*>(owner) != 0x00ABB530) return nullptr;
    return owner;
}

bool Capture(uint32_t (&fields)[kFields]) {
    auto* owner = NativeOwner();
    if (!owner) return false;
    float a = 0, b = 0, c = 0;
    cS3DVector3 color;
    owner->GetShadowParams(a, b, color);
    fields[0] = Bits(a);
    fields[1] = Bits(b);
    fields[2] = Bits(color.fX);
    fields[3] = Bits(color.fY);
    fields[4] = Bits(color.fZ);
    unsigned long texture = 0;
    cS3DVector2 pair;
    owner->GetDecalShadowParams(nullptr, texture, a, b, c, pair, color);
    fields[5] = texture;
    fields[6] = Bits(a);
    fields[7] = Bits(b);
    fields[8] = Bits(c);
    fields[9] = Bits(pair.fX);
    fields[10] = Bits(pair.fY);
    fields[11] = Bits(color.fX);
    fields[12] = Bits(color.fY);
    fields[13] = Bits(color.fZ);
    return true;
}

void Compare(int32_t julian) {
    if (!enabled) return;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    const int32_t month = date.year * 12 + date.month;
    if (month == lastMonth) return; // One dated line, including the load month.
    lastMonth = month;
    uint32_t native[kFields]{}, expected[kFields]{};
    const bool captured = Capture(native);
    const uint32_t mask = captured ? fc4_lighting_shadow_compare(native, expected) : (1u << kFields) - 1;
    uint32_t mismatches = 0;
    for (uint32_t i = 0; i < kFields; ++i) {
        if ((mask & (1u << i)) != 0) {
            ++mismatches;
            if (lines < 3) fc4::Log("lighting: field=%u native=%08X expected=%08X captured=%u",
                i, native[i], expected[i], captured ? 1u : 0u);
        }
    }
    fc4::Log("lighting shadow %04d-%02d-%02d: scope=shipped-parameters fields=%u captured=%u mismatches=%u",
        date.year, date.month, date.day_of_month, kFields, captured ? 1u : 0u, mismatches);
    ++lines;
    if (mismatches == 0) ++clean;
}
}

namespace fc4 {
void LightingShadowCityLoaded(cISC4City* city, int32_t julian) {
    enabled = city && SettingInt("lighting_shadow", nullptr, 0) != 0;
    lastMonth = -1;
    lines = clean = 0;
    if (!enabled) return;
    // LT14: literal shipped §1.4 inputs only. No native output seeds or resyncs.
    // No hook, debug register, native mutation, Init, Shutdown or lifetime probe.
    Log("lighting: ON; partial shadow: slots 14 and 16(null), 14 scalar fields; LT14 dynamic replay unavailable; no whole-owner credit");
    if (auto* sim = AsGame(city->GetSimulator())) julian = sim->GetSimDateNumber();
    Compare(julian);
}
void LightingShadowMonth(int32_t julian) { Compare(julian); }
void LightingShadowCityShutdown() {
    if (enabled) Log("lighting: shutdown: %u/%u exact; scope=shipped-parameters resyncs=0", clean, lines);
    enabled = false;
    lastMonth = -1;
}
}
