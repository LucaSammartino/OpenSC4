// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4MySimDispatchShadow.h"
#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Subsystems.h"
#include "FC4WriteCapture.h"
#include "fc4.h"
#include "cIGZCOM.h"
#include "cIGZSerializable.h"
#include "cISC4Advisor.h"
#include "cISC4AdvisorSystem.h"
#include "cISC4City.h"
#include "cRZCOMDllDirector.h"
#include <cstring>
#include <set>

extern "C" uint32_t fc4_mysimdispatch_shadow_compare(const uint32_t*, uint32_t);
extern "C" uint32_t fc4_mysimdispatch_shadow_constructor(const uint8_t*, size_t);
extern "C" uint32_t fc4_mysimdispatch_shadow_auxiliary(const uint32_t*);
extern "C" uint32_t fc4_mysimdispatch_shadow_live_range(const uint32_t*);

namespace fc4 {
namespace {
constexpr uint32_t kDispatch = 0xCBC14674, kSerializable = 0xE4FDA3D4;
int32_t lastMonth = 0;
uint32_t months = 0, clean = 0;
uint32_t liveSamples = 0, rangeSamples = 0;

// Decode the writer protocol, including variable-width references. This checks
// snapshot contracts only; MS20 forbids treating snapshots as trip inputs.
bool HeaderTail(const std::vector<uint8_t>& bytes, uint32_t (&words)[18],
                uint32_t (&auxiliary)[2], uint32_t& routeCount) {
    CaptureReader in(bytes);
    uint32_t version = 0, ref = 0, scalar = 0;
    uint8_t flag = 0;
    if (!in.U32(version) || !in.U8(flag)) return false;
    for (auto& word : words) if (!in.U32(word)) return false;
    for (unsigned location = 0; location < 4; ++location) {
        if (!in.Ref(ref)) return false;
        for (unsigned i = 0; i < 6; ++i) if (!in.U32(scalar)) return false;
        if (!in.U8(flag) || !in.U8(flag) || !in.U32(scalar) || !in.U32(scalar)) return false;
    }
    if (!in.Ref(ref) || !in.U32(auxiliary[1])) return false;
    auxiliary[0] = ref == 0 ? 1u : 0u;
    uint32_t text = 0;
    if (!in.U32(text)) return false;
    const uint32_t encoding = text >> 24;
    if (encoding != 0 && encoding != 0x10) return false;
    const uint32_t length = (text & 0x00FFFFFF) * (encoding == 0x10 ? 2u : 1u);
    for (uint32_t i = 0; i < length; ++i) if (!in.U8(flag)) return false;
    return in.Ref(ref) && in.Ref(ref) && in.Ref(ref) && in.U32(routeCount);
}

// MS20 §7.2.3: compare assigned constructor fields, excluding words the
// native constructor never assigns. Native outputs never seed expected state.
bool Compare(cIGZSerializable* object, bool constructor, uint32_t& fields, uint32_t& differences, uint32_t& ranges) {
    WriteCapture capture;
    if (!capture.Capture(object) || capture.Bytes().size() < 77) return false;
    const auto& bytes = capture.Bytes();
    uint32_t words[18]{}, auxiliary[2]{}, routeCount = 0;
    if (!HeaderTail(bytes, words, auxiliary, routeCount)) return false;
    uint32_t values[7] = {object->GetGZCLSID(), 0, 0, 0, 0, 0, 0};
    std::memcpy(&values[1], bytes.data(), 4);
    // §6.2.1: auxiliary bool at byte4, 18 scalar words at byte5.
    // Current/previous state, flags, minutes, travel are words11..15.
    for (unsigned i = 0; i < 5; ++i)
        std::memcpy(&values[i + 2], bytes.data() + 5 + (11 + i) * 4, 4);
    fields += constructor ? 7 : 2;
    differences += fc4_mysimdispatch_shadow_compare(values, constructor ? 1 : 0);
    if (constructor) {
        const uint32_t result = fc4_mysimdispatch_shadow_constructor(bytes.data(), bytes.size());
        if (result == UINT32_MAX) return false;
        fields += 208; // independently predicted guaranteed prefix bytes
        differences += result;
        fields += 2; // MS22 null auxiliary reference and zero scalar
        differences += fc4_mysimdispatch_shadow_auxiliary(auxiliary);
    } else if (words[13] & 0x1000) {
        const uint32_t range[3] = {words[6], words[7], routeCount};
        fields += 3; // MS21 signed start/end and stored route bound
        differences += fc4_mysimdispatch_shadow_live_range(range);
        ++ranges;
    }
    return true;
}
}

void MySimDispatchShadowMonth(cISC4City* city, int32_t julian) {
    if (!SettingInt("mysimdispatch_shadow", "FC4_MYSIMDISPATCH_SHADOW", 0) || !city ||
        julian <= 0) return;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    const int32_t monthKey = date.year * 12 + date.month;
    if (monthKey == lastMonth) return;
    lastMonth = monthKey;
    uint32_t fields = 0, differences = 0, errors = 0, fixtures = 0, live = 0, ranges = 0;
    auto* director = RZGetCOMDllDirector();
    auto* com = director ? director->GZCOM() : nullptr;
    void* out = nullptr;
    // Never Init or Read the fixture: it must not subscribe to city messages.
    if (!FindEnabledClass(kDispatch, nullptr) && com && com->GetClassObject(kDispatch, kSerializable, &out) && out) {
        auto* fixture = static_cast<cIGZSerializable*>(out);
        FC4_ACQUIRED(fixture, "dispatch constructor fixture factory owned");
        if (Compare(fixture, true, fields, differences, ranges)) ++fixtures;
        else ++errors;
        FC4_RELEASE_CREATED(fixture);
    } else ++errors;

    // §1.3/§6.2: discover dispatches exclusively through public advisor refs.
    // Inactive MySims omit their dispatch reference; MS20 asks for a safe getter.
    std::set<const void*> seen;
    auto* advisors = city->GetAdvisorSystem(); // borrowed
    for (int id : {1, 2, 3, 4, 5, 30}) {
        auto* advisor = advisors ? advisors->GetAdvisor(id) : nullptr; // borrowed
        if (!advisor) { ++errors; continue; }
        WriteCapture owner;
        if (!owner.Capture(advisor)) { ++errors; continue; }
        for (uint32_t ref = 1; owner.Object(ref); ++ref) {
            if (owner.ClassOf(ref) != kDispatch || !seen.insert(owner.Object(ref)).second) continue;
            if (Compare(owner.SerializableAt(ref), false, fields, differences, ranges)) ++live;
            else ++errors;
        }
    }
    ++months;
    liveSamples += live;
    rangeSamples += ranges;
    if (differences == 0 && errors == 0) ++clean;
    Log("mysimdispatch shadow %04d-%02d-%02d: constructor=%u live_headers=%u live_ranges=%u fields=%u errors=%u "
        "scope=constructor-auxiliary-and-live-header-contracts live_trip=uncompared(input-boundary) transport=uncompared(input-boundary) mismatches=%u",
        date.year, date.month, date.day_of_month, fixtures, live, ranges, fields, errors, differences + errors);
}

void MySimDispatchShadowShutdown() {
    if (months) Log("mysimdispatch: measured %u/%u exact constructor/live-header samples; live_headers=%u live_ranges=%u; live trip owner remains unverified (MS20 input-boundary limitation)", clean, months, liveSamples, rangeSamples);
    lastMonth = 0;
    months = clean = 0;
    liveSamples = rangeSamples = 0;
}

void MySimDispatchShadowCityLoaded(cISC4City* city) {
    // PublishedJulian is not available on every load callback. Use the same
    // public simulator date getter as the existing MySims load shadow.
    auto* simulator = AsGame(city ? city->GetSimulator() : nullptr);
    MySimDispatchShadowMonth(city, simulator ? simulator->GetSimDateNumber() : 0);
}
}
