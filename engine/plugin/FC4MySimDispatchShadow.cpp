// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4MySimDispatchShadow.h"
#include "FC4Config.h"
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

namespace fc4 {
namespace {
constexpr uint32_t kDispatch = 0xCBC14674, kSerializable = 0xE4FDA3D4;
int32_t lastMonth = 0;
uint32_t months = 0, clean = 0;

// Only settled constructor state fields and writer identity are compared.
// MS20: opaque/tuning/location constructor defaults are not inferred here.
bool Compare(cIGZSerializable* object, bool constructor, uint32_t& fields, uint32_t& differences) {
    WriteCapture capture;
    if (!capture.Capture(object) || capture.Bytes().size() < 77) return false;
    const auto& bytes = capture.Bytes();
    uint32_t values[7] = {object->GetGZCLSID(), 0, 0, 0, 0, 0, 0};
    std::memcpy(&values[1], bytes.data(), 4);
    // §6.2.1: auxiliary bool at byte4, 18 scalar words at byte5.
    // Current/previous state, flags, minutes, travel are words11..15.
    for (unsigned i = 0; i < 5; ++i)
        std::memcpy(&values[i + 2], bytes.data() + 5 + (11 + i) * 4, 4);
    fields += constructor ? 7 : 2;
    differences += fc4_mysimdispatch_shadow_compare(values, constructor ? 1 : 0);
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
    uint32_t fields = 0, differences = 0, errors = 0, fixtures = 0, live = 0;
    auto* director = RZGetCOMDllDirector();
    auto* com = director ? director->GZCOM() : nullptr;
    void* out = nullptr;
    // Never Init or Read the fixture: it must not subscribe to city messages.
    if (!FindEnabledClass(kDispatch, nullptr) && com && com->GetClassObject(kDispatch, kSerializable, &out) && out) {
        auto* fixture = static_cast<cIGZSerializable*>(out);
        if (Compare(fixture, true, fields, differences)) ++fixtures;
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
            if (Compare(owner.SerializableAt(ref), false, fields, differences)) ++live;
            else ++errors;
        }
    }
    ++months;
    if (differences == 0 && errors == 0) ++clean;
    Log("mysimdispatch shadow %04d-%02d-%02d: constructor=%u live_headers=%u fields=%u errors=%u "
        "scope=constructor-and-writer-identity-only live_trip=uncompared(MS20) transport=uncompared(MS18) mismatches=%u",
        date.year, date.month, date.day_of_month, fixtures, live, fields, errors, differences + errors);
}

void MySimDispatchShadowShutdown() {
    if (months) Log("mysimdispatch: measured %u/%u exact constructor/header samples; live trip owner remains unverified (MS20/MS18)", clean, months);
    lastMonth = 0;
    months = clean = 0;
}
}
