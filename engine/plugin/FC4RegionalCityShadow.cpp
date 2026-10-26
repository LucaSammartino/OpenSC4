// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4RegionalCityShadow.h"
#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4WriteCapture.h"
#include "FC4RegionalCityRecordReplay.h"
#include "FC4RegionalCityElements.h"
#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZCOM.h"
#include "cIGZFrameWork.h"
#include "cISC4App.h"
#include "cISC4Region.h"
#include "cISC4RegionalCity.h"
#include "cISC4NeighborConnection.h"
#include "cISC4NeighborDeal.h"
#include "cRZAutoRefCount.h"
#include "cRZCOMDllDirector.h"
#include "cRZBaseString.h"
#include "SC4List.h"
#include "fc4.h"
#include <cmath>
#include <cstdio>
#include <cstring>
#include <set>
#include <string>

namespace {
using Probe = bool (*)(void*, uint32_t, uint32_t, double, const uint8_t*, uint32_t);
extern "C" uint64_t fc4_regionalcity_shadow_compare(const uint8_t*, uint32_t, Probe, void*);
extern "C" uint64_t fc4_regionalcity_shadow_mutation_compare(const uint8_t*, uint32_t, uint32_t, Probe, void*);
struct Context {
    cFC4ISC4RegionalCity* city;
    const std::vector<uint8_t>* wire;
    uint32_t listed = 0;
    std::string differences;
};

bool ProbeField(void* raw, uint32_t field, uint32_t arg, double expected, const uint8_t* bytes, uint32_t len) {
    auto& ctx = *static_cast<Context*>(raw);
    auto* c = ctx.city;
    double actual = 0;
    const char* name = "unknown";
    bool equal = false;
    cRZBaseString text;
    int32_t x = 0, z = 0;
    switch (field) {
        case 0: case 1: c->GetPosition(x, z); actual = field == 0 ? x : z; name = "position"; break;
        case 2: case 3: c->GetCitySize(x, z); actual = field == 2 ? x : z; name = "size"; break;
        case 4: actual = c->GetPopulation(); name = "population"; break;
        case 5: actual = c->GetCommercialJobs(); name = "commercialJobs"; break;
        case 6: actual = c->GetIndustrialJobs(); name = "industrialJobs"; break;
        case 7: {
            // regional-city-records §5.4 / RY5: EAX is an address, not float bits.
            const auto* value = reinterpret_cast<const float*>(static_cast<uintptr_t>(c->GetWorkforcePercentage()));
            actual = value ? *value : NAN; name = "workforce"; break;
        }
        case 8: actual = c->GetMayorRating(); name = "mayorRating"; break;
        case 9: actual = static_cast<uint32_t>(c->GetDifficultyLevel()); name = "difficulty"; break;
        case 10: actual = c->GetCitySerialNumber(); name = "serial"; break;
        case 11: actual = c->GetBirthDate(); name = "birthDate"; break;
        case 12: actual = c->GetEstablished(); name = "established"; break;
        case 13: case 14: { float a = 0, b = 0; c->GetWorldPosition(a, b); actual = field == 13 ? a : b; name = "worldPosition"; break; }
        case 15: actual = c->GetWorldBaseElevation(); name = "elevation"; break;
        case 16: actual = static_cast<uint32_t>(c->GetWorldHemisphere()); name = "hemisphere"; break;
        case 17: actual = c->GetBudget(); name = "budget"; break;
        case 18: actual = c->GetIncome(); name = "income"; break;
        case 19: actual = c->GetTutorialGUID(); name = "tutorialGUID"; break;
        // [abi-ret] regional-city §3: IsTutorial returns GetTutorialGUID() != -1.
        case 20: actual = c->IsTutorial(); name = "isTutorial"; break;
        // [abi-ret] regional-city §3 explicitly says both locale getters return true.
        case 21: case 22: if (!c->GetOriginalLanguageAndCountry(x, z)) return false; actual = field == 21 ? x : z; name = "originalLocale"; break;
        case 23: case 24: if (!c->GetLastLanguageAndCountry(x, z)) return false; actual = field == 23 ? x : z; name = "lastLocale"; break;
        case 25: {
            SC4List<cISC4NeighborConnection*> list;
            c->GetAllConnections(list);
            // The SDK list destructor owns the copied references.
            for (auto* entry : list) { (void)entry; ++actual; }
            name = "connectionCount"; break;
        }
        case 26: {
            SC4List<cISC4NeighborDeal*> list;
            // [abi-ret] regional-city §3 / foundations §11.3: compare the copied L list;
            // the result is unspecified and is not an input to this observer.
            c->GetLocalDeals(list);
            for (auto* entry : list) { (void)entry; ++actual; }
            name = "localDealCount"; break;
        }
        case 30: c->GetCityName(text); name = "cityName"; break;
        case 31: c->GetMayorName(text); name = "mayorName"; break;
        case 32: c->GetUtilityAdvisorName(text); name = "utilityAdvisorName"; break;
        case 33: c->GetCityDescription(text); name = "description"; break;
        case 60: {
            fc4::GameVector<FC4PointOfInterest> points;
            const FC4PointOfInterest sentinel{{0x3f800000, 0xc0000000, 0x40400000}};
            if (!points.push_back(sentinel)) return false;
            c->GetPointsOfInterest(arg, points);
            name = "pointsOfInterest";
            actual = static_cast<double>(points.size());
            expected = static_cast<double>(len / sizeof(FC4PointOfInterest));
            equal = len == points.size() * sizeof(FC4PointOfInterest) &&
                (len == 0 || std::memcmp(bytes, points.begin(), len) == 0);
            if (!equal) {
                char detail[160];
                size_t offset = 0;
                const auto* actualBytes = reinterpret_cast<const uint8_t*>(points.begin());
                const size_t actualLen = points.size() * sizeof(FC4PointOfInterest);
                while (offset < len && offset < actualLen && bytes[offset] == actualBytes[offset]) ++offset;
                snprintf(detail, sizeof(detail), " poi[%u] nativeBytes=%zu coreBytes=%u firstDifference=%zu;",
                    arg, actualLen, len, offset);
                ctx.differences += detail;
            }
            break;
        }
        case 40: actual = c->GetTaxRate(arg); name = "tax"; break;
        case 41: actual = c->GetPopulationOfType(arg); name = "populationByType"; break;
        case 42: actual = c->GetExtrapolatedPopulation(arg); name = "extrapolated"; break;
        case 43: actual = c->GetAllowableExtrapolation(arg); name = "allowable"; break;
        case 50: actual = c->GetProduced(static_cast<int32_t>(arg)); name = "produced"; break;
        case 51: actual = c->GetDemanded(static_cast<int32_t>(arg)); name = "demanded"; break;
        case 52: actual = c->GetExported(static_cast<int32_t>(arg)); name = "exported"; break;
        case 53: actual = c->GetImported(static_cast<int32_t>(arg)); name = "imported"; break;
        case 54: actual = c->GetCostPerUnit(static_cast<int32_t>(arg)); name = "costPerUnit"; break;
        case 55: actual = c->GetCommodityBalance(static_cast<int32_t>(arg)); name = "balance"; break;
        case 100: {
            name = "recordRoundtrip";
            size_t offset = 0;
            while (offset < len && offset < ctx.wire->size() && bytes[offset] == (*ctx.wire)[offset]) ++offset;
            equal = len == ctx.wire->size() && offset == len;
            if (!equal) {
                char detail[128];
                snprintf(detail, sizeof(detail), " recordBytes native=%zu core=%u firstDifference=%zu;", ctx.wire->size(), len, offset);
                ctx.differences += detail;
            }
            break;
        }
        default: return false;
    }
    if (field >= 30 && field <= 33) equal = text.Strlen() == len && (len == 0 || memcmp(text.Data(), bytes, len) == 0);
    else if (field != 100 && field != 60) equal = actual == expected || (std::isnan(actual) && std::isnan(expected));
    if (!equal && ctx.listed++ < 8) {
        char detail[192];
        snprintf(detail, sizeof(detail), " %s[%u] native=%.17g core=%.17g;", name, arg, actual, expected);
        ctx.differences += detail;
    }
    return equal;
}

struct Observation { int32_t julian; bool load; };

// A separate, never-Init object exercises setters without changing any city,
// subscriptions, connections or save files belonging to the running game.
// SPEC-GAP: regional-city §1 identifies direct construction sites but supplies no interception contract; this is a shadow fixture, not a replacement.
void MutateFixture(cFC4ISC4RegionalCity* c, uint32_t step) {
    if (step == 0) {
        c->SetPosition(-3, 7, false);
        c->SetCitySize(2, 4);
        c->SetCitySerialNumber(0x81234567);
        c->SetBirthDate(0xf1234567);
        c->SetEstablished(true);
        c->SetWorldPosition(12.5f, -45.25f);
        c->SetWorldBaseElevation(37.75f);
        c->SetBudget(-123456.125);
        c->SetIncome(98765.0625);
        c->SetTutorialGUID(0x12345678);
        cRZBaseString city("FC4 isolated city"), mayor("FC4 isolated mayor");
        cRZBaseString utility("FC4 isolated utility"), description("FC4 isolated description");
        c->SetCityName(city);
        c->SetMayorName(mayor);
        c->SetUtilityAdvisorName(utility);
        c->SetCityDescription(description);
    } else if (step == 1 || step == 2) {
        for (int32_t commodity : {1, 2, 3, 97}) {
            const double value = step == 1 ? commodity + 0.125 : -1.0;
            c->SetProduced(commodity, value);
            c->SetDemanded(commodity, value * 2.0);
            c->SetExported(commodity, value * 3.0);
            c->SetImported(commodity, value * 4.0);
            c->SetCostPerUnit(commodity, value * 5.0);
        }
    } else {
        c->SetEstablished(false);
        c->SetTutorialGUID(0xffffffff);
        cRZBaseString empty;
        c->SetCityName(empty);
        c->SetMayorName(empty);
        c->SetUtilityAdvisorName(empty);
        c->SetCityDescription(empty);
        c->ExtrapolateGrowth(0x1010, 17.75f);
        c->ExtrapolateGrowth(0x1010, -2.5f);
        c->GetAllowableExtrapolation(0x1010);
    }
}

void CompareMutations(uint32_t& fields, uint32_t& mismatches, std::string& differences) {
    cISC4RegionalCity* fixture = nullptr;
    auto* com = GZCOM();
    if (!com || !com->GetClassObject(0x6a037c24, 0x69d2be1d, reinterpret_cast<void**>(&fixture)) || !fixture) {
        ++mismatches;
        differences += " fixture-create-failed;";
        return;
    }
    FC4_ACQUIRED(fixture, "regionalcity isolated factory fixture");
    fc4::WriteCapture baseline;
    if (!baseline.Capture(fixture)) {
        ++mismatches;
        differences += " fixture-capture-failed;";
    } else {
        for (uint32_t step = 0; step != 4; ++step) {
            auto* city = fc4::AsGame(fixture);
            MutateFixture(city, step);
            fc4::WriteCapture after;
            if (!after.Capture(fixture)) {
                ++mismatches;
                differences += " fixture-write-failed;";
                break;
            }
            Context ctx{city, &after.Bytes()};
            const uint64_t result = fc4_regionalcity_shadow_mutation_compare(baseline.Bytes().data(),
                static_cast<uint32_t>(baseline.Bytes().size()), step, &ProbeField, &ctx);
            if (result == UINT64_MAX) {
                ++mismatches;
                differences += " fixture-decode-failed;";
                break;
            }
            fields += static_cast<uint32_t>(result >> 32);
            mismatches += static_cast<uint32_t>(result);
            if (!ctx.differences.empty() && differences.size() < 1500)
                differences += " fixture-step=" + std::to_string(step) + ctx.differences;
        }
    }
    FC4_RELEASE_CREATED(fixture);
}

void Observe(void* raw) {
    const auto& obs = *static_cast<Observation*>(raw);
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (!app || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || !sc4) {
        fc4::Log("regionalcity shadow: unavailable application; no comparison"); return;
    }
    FC4_ACQUIRED(sc4, "regionalcity shadow app");
    cISC4Region* region = sc4->GetRegion(); // borrowed
    cISC4RegionalCity* current = sc4->GetRegionalCity(); // borrowed; often distinct from region's copy
    SC4List<cRZAutoRefCount<cISC4RegionalCity>> cities;
    const bool enumerated = region != nullptr;
    // [abi-ret] region §5 specifies appended references; ignore the unspecified result.
    if (region) region->GetAllCities(cities);
    std::set<cISC4RegionalCity*> records;
    if (current) records.insert(current);
    for (const auto& holder : cities) if (auto* c = static_cast<cISC4RegionalCity*>(holder)) records.insert(c);
    uint32_t fields = 0, mismatches = enumerated && current ? 0 : 1, decoded = 0;
    std::string differences;
    for (auto* record : records) {
        fc4::WriteCapture capture;
        if (!capture.Capture(record)) { ++mismatches; differences += " capture-failed;"; continue; }
        ++fields;
        if (!fc4::regionalcity_replay::Roundtrip(capture)) {
            ++mismatches;
            differences += " owner-record-roundtrip-failed;";
        }
        if (!fc4::regionalcity::CheckLiveRecord(capture, fc4::AsGame(record))) {
            ++mismatches;
            differences += " live-record-roundtrip-failed;";
        }
        Context ctx{fc4::AsGame(record), &capture.Bytes()};
        const uint64_t result = fc4_regionalcity_shadow_compare(capture.Bytes().data(), static_cast<uint32_t>(capture.Bytes().size()), &ProbeField, &ctx);
        if (result == UINT64_MAX) { ++mismatches; differences += " decode-failed;"; continue; }
        ++decoded;
        fields += static_cast<uint32_t>(result >> 32);
        mismatches += static_cast<uint32_t>(result);
        // The differential must not change a native record. In particular,
        // allowable-extrapolation and commodity getters can insert map entries.
        fc4::WriteCapture after;
        ++fields;
        if (!after.Capture(record) || after.Bytes() != capture.Bytes()) {
            ++mismatches;
            ctx.differences += " observation-changed-record;";
        }
        if (!ctx.differences.empty() && differences.size() < 1500) {
            differences += " serial=" + std::to_string(ctx.city->GetCitySerialNumber()) + ctx.differences;
        }
    }
    CompareMutations(fields, mismatches, differences);
    fc4_date date{};
    fc4_date_from_julian(obs.julian, &date);
    fc4::Log("regionalcity shadow %04d-%02d-%02d: phase=%s records=%u decoded=%u fields=%u mismatches=%u%s",
        date.year, date.month, date.day_of_month, obs.load ? "load" : "month",
        static_cast<uint32_t>(records.size()), decoded, fields, mismatches, differences.c_str());
    FC4_RELEASE(sc4);
}
}

namespace fc4 {
void RegionalCityShadowObserve(int32_t julian, bool load) {
    if (SettingInt("regionalcity_shadow", nullptr, 0) == 0) return;
    if (load) Log("regionalcity shadow: ON; native live record -> core decode; exact numeric/string/point getters and record roundtrip; host-owned element record replay; isolated setter fixtures; observation preserves native record");
    Observation obs{julian, load};
    SehGuard("regionalcity shadow", &Observe, &obs);
}
}
