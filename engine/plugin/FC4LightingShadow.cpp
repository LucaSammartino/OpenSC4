// SPDX-License-Identifier: GPL-3.0-or-later
// lighting.md §§1.1/1.4/5.6. Native owner stays running; only const getters.
#include "FC4LightingShadow.h"
#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "compat/FC4Compat.h"
#include "cISC4City.h"
#include "cISC4LightingManager.h"
#include "cS3DVector2.h"
#include "cS3DVector3.h"
#include "fc4.h"
#include <windows.h>
#include <cstring>
#include <vector>

namespace {
constexpr uint32_t kFields = 14;
extern "C" uint32_t fc4_lighting_shadow_compare(const uint32_t*, const uint32_t*, uint32_t*);
bool enabled = false;
bool inputsReady = false;
uint32_t inputs[10]{};
int32_t lastMonth = -1;
uint32_t lines = 0, clean = 0;

uint32_t Bits(float value) {
    uint32_t result;
    std::memcpy(&result, &value, sizeof(result));
    return result;
}

bool CaptureInputs() {
    // §1.3 publishes this resource as the owner's input, separate from outputs.
    fc4::props::Exemplar exemplar;
    if (!exemplar.Load(0x6534284A, 0xA9189CF0, 1)) return false;
    const auto* holder = exemplar.Holder();
    if (!holder) return false;
    std::memset(inputs, 0, sizeof(inputs));
    const uint32_t ids[6] = {0x085489C8, 0x085489CC, 0x085489CD, 0x085489CA, 0x085489C9, 0x085489CB};
    const uint32_t offsets[6] = {1, 2, 3, 4, 5, 7};
    const uint32_t counts[6] = {1, 1, 1, 1, 2, 3};
    for (uint32_t i = 0; i < 6; ++i) {
        if (!fc4::props::Has(holder, ids[i])) continue; // §1.4 initial value if absent.
        if (i == 0) {
            std::vector<uint32_t> values;
            if (!fc4::props::GetU32s(holder, ids[i], values) || values.size() != 1) return false;
            inputs[1] = values[0];
        } else {
            std::vector<float> values;
            if (!fc4::props::GetF32s(holder, ids[i], values) || values.size() != counts[i]) return false;
            for (uint32_t j = 0; j < counts[i]; ++j) inputs[offsets[i] + j] = Bits(values[j]);
        }
        inputs[0] |= 1u << i;
    }
    fc4::Log("lighting: input resource=6534284A/A9189CF0/00000001 presence=%02X texture=%08X; immutable load capture",
        inputs[0], inputs[1]);
    return true;
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
    const bool captured = inputsReady && Capture(native);
    const uint32_t mask = captured ? fc4_lighting_shadow_compare(inputs, native, expected) : (1u << kFields) - 1;
    uint32_t mismatches = 0;
    for (uint32_t i = 0; i < kFields; ++i) {
        if ((mask & (1u << i)) != 0) {
            ++mismatches;
            if (lines < 3) fc4::Log("lighting: field=%u native=%08X expected=%08X captured=%u",
                i, native[i], expected[i], captured ? 1u : 0u);
        }
    }
    fc4::Log("lighting shadow %04d-%02d-%02d: scope=exemplar-parameters fields=%u captured=%u mismatches=%u",
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
    inputsReady = false;
    if (!enabled) return;
    // LT14: §1.4 exemplar inputs only. No native output seeds or resyncs.
    // No hook, debug register, native mutation, Init, Shutdown or lifetime probe.
    Log("lighting: ON; partial shadow: slots 14 and 16(null), 14 scalar fields; LT14 dynamic replay unavailable; no whole-owner credit");
    inputsReady = CaptureInputs();
    if (auto* sim = AsGame(city->GetSimulator())) julian = sim->GetSimDateNumber();
    Compare(julian);
}
void LightingShadowMonth(int32_t julian) { Compare(julian); }
void LightingShadowCityShutdown() {
    if (enabled) Log("lighting: shutdown: %u/%u exact; scope=exemplar-parameters resyncs=0", clean, lines);
    enabled = false;
    inputsReady = false;
    lastMonth = -1;
}
}
