// SPDX-License-Identifier: GPL-3.0-or-later
// lighting.md §§1.1/1.4/5.6. Native owner stays running; only const getters.
#include "FC4LightingShadow.h"
#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4PropertyHolder.h"
#include "compat/FC4Compat.h"
#include "cISC4City.h"
#include "cISC4LightingManager.h"
#include "cIGZVariant.h"
#include "cS3DVector2.h"
#include "cS3DVector3.h"
#include "fc4.h"
#include <windows.h>
#include <cstring>
#include <vector>

namespace {
constexpr uint32_t kFields = 14;
extern "C" uint32_t fc4_lighting_shadow_compare(const uint32_t*, const uint32_t*, uint32_t*);
extern "C" uint32_t fc4_lighting_shadow_decal_compare(const uint32_t*, const uint32_t*, const uint32_t*, uint32_t*);
extern "C" uint32_t fc4_lighting_shadow_empty_compare(uint32_t, const uint32_t*, uint32_t*);
bool enabled = false;
bool inputsReady = false;
uint32_t inputs[10]{};
int32_t lastMonth = -1;
uint32_t lines = 0, clean = 0;
uint32_t queryLines = 0, queryClean = 0;

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

uint32_t CompareQueries(int32_t julian) {
    auto* owner = NativeOwner();
    if (!owner || !inputsReady) {
        fc4::Log("lighting queries: unavailable mismatches=1");
        ++queryLines;
        return 1; // Missing observations cannot count as an exact capture.
    }
    uint32_t mismatches = 0, cases = 0;
    // §1.6: these are independently supplied query arguments, not inferred
    // current owner state. Test all presence combinations, including an empty
    // nonnull holder and full overrides, then repeat the original null query.
    const uint32_t ids[6] = {0x085489C8, 0x085489CC, 0x085489CD, 0x085489CA, 0x085489C9, 0x085489CB};
    const uint32_t offsets[6] = {1, 2, 3, 4, 5, 7};
    const uint32_t counts[6] = {1, 1, 1, 1, 2, 3};
    uint32_t overrides[10] = {0, 0x12345678, Bits(0.25f), Bits(0.75f), Bits(20.0f),
        Bits(-2.0f), Bits(3.0f), Bits(0.5f), Bits(0.25f), Bits(0.75f)};
    for (uint32_t mask = 0; mask < 64; ++mask) {
        cFC4PropertyHolder holder(nullptr); // Owned local input; never retained by native getter.
        bool ready = true;
        for (uint32_t i = 0; i < 6; ++i) {
            if (!(mask & (1u << i))) continue;
            if (i == 0) {
                // §1.6 includes property type/count in the independent input.
                // The expected Inputs record models scalar u32/f32 values:
                // a one-element array is not an interchangeable native input.
                ready = fc4::props::SetTyped(holder.AsSdk(), ids[i], cIGZVariant::Uint32,
                    false, &overrides[1], 1) && ready;
                ready = fc4::props::TypeOf(holder.AsSdk(), ids[i]) == cIGZVariant::Uint32 && ready;
            } else {
                std::vector<float> values(counts[i]);
                for (uint32_t j = 0; j < counts[i]; ++j)
                    std::memcpy(&values[j], &overrides[offsets[i] + j], sizeof(float));
                ready = fc4::props::SetTyped(holder.AsSdk(), ids[i], cIGZVariant::Float32,
                    counts[i] > 1, values.data(), counts[i]) && ready;
                const auto type = counts[i] == 1 ? cIGZVariant::Float32 : cIGZVariant::Float32Array;
                ready = fc4::props::TypeOf(holder.AsSdk(), ids[i]) == type && ready;
            }
        }
        uint32_t native[9]{}, expected[9]{};
        unsigned long texture = 0;
        float a = 0, b = 0, c = 0;
        cS3DVector2 pair;
        cS3DVector3 color;
        owner->GetDecalShadowParams(holder.AsSdk(), texture, a, b, c, pair, color);
        const uint32_t values[9] = {texture, Bits(a), Bits(b), Bits(c), Bits(pair.fX), Bits(pair.fY),
            Bits(color.fX), Bits(color.fY), Bits(color.fZ)};
        std::memcpy(native, values, sizeof(native));
        overrides[0] = mask;
        const uint32_t diff = ready ? fc4_lighting_shadow_decal_compare(inputs, overrides, native, expected) : 511;
        for (uint32_t i = 0; i < 9; ++i) if (diff & (1u << i)) {
            if (queryLines < 3 && mismatches < 3) fc4::Log("lighting query: slot=16 presence=%02X field=%u native=%08X expected=%08X ready=%u",
                mask, i, native[i], expected[i], ready ? 1u : 0u);
            ++mismatches;
        }
        ++cases;
    }
    // §5.8 publishes this byte layout; SDK forward-declares these types.
    // Count zero never reads point/UV arrays and must leave output untouched.
    struct TransformInput { uint32_t header; float linear[9], translation[3], scale; };
    static_assert(sizeof(TransformInput) == 56, "lighting transform input layout");
    for (uint32_t flags : {0u, 7u}) {
        TransformInput transform = {flags << 8, {1, 0, 0, 0, 1, 0, 0, 0, 1}, {2, 3, 4}, 2};
        uint32_t native[17]{}, expected[17]{};
        for (uint32_t i = 0; i < 16; ++i) native[i + 1] = 0x3E800001 + i;
        native[0] = owner->CalcShadowProjection(0, nullptr, nullptr,
            reinterpret_cast<const cS3DTransform&>(transform), reinterpret_cast<cS3DMatrix4&>(native[1])) ? 1u : 0u;
        const uint32_t diff = fc4_lighting_shadow_empty_compare(flags, native, expected);
        for (uint32_t i = 0; i < 17; ++i) if (diff & (1u << i)) {
            if (queryLines < 3 && mismatches < 3) fc4::Log("lighting query: slot=17 count=0 flags=%u field=%u native=%08X expected=%08X",
                flags, i, native[i], expected[i]);
            ++mismatches;
        }
        ++cases;
    }
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    fc4::Log("lighting queries %04d-%02d-%02d: cases=%u fields=610 mismatches=%u; slot16_masks=64 slot17_empty=2 whole_owner=unavailable resyncs=0",
        date.year, date.month, date.day_of_month, cases, mismatches);
    ++queryLines;
    if (mismatches == 0) ++queryClean;
    return mismatches;
}

void Compare(int32_t julian) {
    if (!enabled) return;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    const int32_t month = date.year * 12 + date.month;
    if (month == lastMonth) return; // One dated line, including the load month.
    lastMonth = month;
    const uint32_t queryMismatches = CompareQueries(julian);
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
    mismatches += queryMismatches;
    fc4::Log("lighting shadow %04d-%02d-%02d: scope=exemplar-and-queries fields=%u captured=%u mismatches=%u",
        date.year, date.month, date.day_of_month, kFields + 610, captured ? 1u : 0u, mismatches);
    ++lines;
    if (mismatches == 0) ++clean;
}
}

namespace fc4 {
void LightingShadowCityLoaded(cISC4City* city, int32_t julian) {
    enabled = city && SettingInt("lighting_shadow", nullptr, 0) != 0;
    lastMonth = -1;
    lines = clean = 0;
    queryLines = queryClean = 0;
    inputsReady = false;
    if (!enabled) return;
    // LT14: §1.4 exemplar inputs only. No native output seeds or resyncs.
    // No hook, debug register, native mutation, Init, Shutdown or lifetime probe.
    Log("lighting: ON; slots14/16(null) original 14 fields; slot16(nonnull) 64 masks; slot17 count-zero 2 flags; whole_owner=unavailable (LT14 section1.6)");
    Log("lighting coverage: slots0-13,15,18-29 and slot17 nonempty unavailable: no construction-to-query ordered tint/settings/view/terrain transcript; cImageARGB8 unavailable");
    inputsReady = CaptureInputs();
    if (auto* sim = AsGame(city->GetSimulator())) julian = sim->GetSimDateNumber();
    Compare(julian);
}
void LightingShadowMonth(int32_t julian) { Compare(julian); }
void LightingShadowCityShutdown() {
    if (enabled) Log("lighting: shutdown: %u/%u exact; scope=exemplar-and-queries resyncs=0", clean, lines);
    if (enabled) Log("lighting: query shutdown: %u/%u exact; fields_per_capture=610 whole_owner=unavailable resyncs=0", queryClean, queryLines);
    enabled = false;
    inputsReady = false;
    lastMonth = -1;
}
}
