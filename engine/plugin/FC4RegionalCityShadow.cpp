// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4RegionalCityShadow.h"
#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4WriteCapture.h"
#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cISC4App.h"
#include "cISC4Region.h"
#include "cISC4RegionalCity.h"
#include "cISC4NeighborConnection.h"
#include "cISC4NeighborDeal.h"
#include "cRZAutoRefCount.h"
#include "cRZCOMDllDirector.h"
#include "cRZString.h"
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
    cRZString text;
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
        case 9: actual = c->GetDifficultyLevel(); name = "difficulty"; break;
        case 10: actual = c->GetCitySerialNumber(); name = "serial"; break;
        case 11: actual = c->GetBirthDate(); name = "birthDate"; break;
        case 12: actual = c->GetEstablished(); name = "established"; break;
        case 13: case 14: { float a = 0, b = 0; c->GetWorldPosition(a, b); actual = field == 13 ? a : b; name = "worldPosition"; break; }
        case 15: actual = c->GetWorldBaseElevation(); name = "elevation"; break;
        case 16: actual = c->GetWorldHemisphere(); name = "hemisphere"; break;
        case 17: actual = c->GetBudget(); name = "budget"; break;
        case 18: actual = c->GetIncome(); name = "income"; break;
        case 19: actual = c->GetTutorialGUID(); name = "tutorialGUID"; break;
        case 20: actual = c->IsTutorial(); name = "isTutorial"; break;
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
            if (!c->GetLocalDeals(list)) return false;
            for (auto* entry : list) { (void)entry; ++actual; }
            name = "localDealCount"; break;
        }
        case 30: c->GetCityName(text); name = "cityName"; break;
        case 31: c->GetMayorName(text); name = "mayorName"; break;
        case 32: c->GetUtilityAdvisorName(text); name = "utilityAdvisorName"; break;
        case 33: c->GetCityDescription(text); name = "description"; break;
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
    else if (field != 100) equal = actual == expected || (std::isnan(actual) && std::isnan(expected));
    if (!equal && ctx.listed++ < 8) {
        char detail[192];
        snprintf(detail, sizeof(detail), " %s[%u] native=%.17g core=%.17g;", name, arg, actual, expected);
        ctx.differences += detail;
    }
    return equal;
}

struct Observation { int32_t julian; bool load; };
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
    const bool enumerated = region && region->GetAllCities(cities);
    std::set<cISC4RegionalCity*> records;
    if (current) records.insert(current);
    for (const auto& holder : cities) if (auto* c = static_cast<cISC4RegionalCity*>(holder)) records.insert(c);
    uint32_t fields = 0, mismatches = enumerated && current ? 0 : 1, decoded = 0;
    std::string differences;
    for (auto* record : records) {
        fc4::WriteCapture capture;
        if (!capture.Capture(record)) { ++mismatches; differences += " capture-failed;"; continue; }
        Context ctx{fc4::AsGame(record), &capture.Bytes()};
        const uint64_t result = fc4_regionalcity_shadow_compare(capture.Bytes().data(), static_cast<uint32_t>(capture.Bytes().size()), &ProbeField, &ctx);
        if (result == UINT64_MAX) { ++mismatches; differences += " decode-failed;"; continue; }
        ++decoded;
        fields += static_cast<uint32_t>(result >> 32);
        mismatches += static_cast<uint32_t>(result);
        if (!ctx.differences.empty() && differences.size() < 1500) {
            differences += " serial=" + std::to_string(ctx.city->GetCitySerialNumber()) + ctx.differences;
        }
    }
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
    if (load) Log("regionalcity shadow: ON; native live record -> core decode; exact numeric/string getters and record roundtrip; no cache refresh/save/mutations");
    Observation obs{julian, load};
    SehGuard("regionalcity shadow", &Observe, &obs);
}
}
