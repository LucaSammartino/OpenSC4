// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4RenderPropsShadow.h"
#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4RenderProperties.h"
#include "cRZCOMDllDirector.h"
#include "fc4.h"
#include <cstring>

extern "C" int32_t fc4_renderprops_shadow_count(uint32_t);
extern "C" int32_t fc4_renderprops_shadow_query(uint32_t, uint32_t, uint32_t, char*, uint32_t);

namespace {
bool active = false;
int32_t lastMonth = -1;
uint32_t lines = 0, clean = 0;

void Compare(int32_t julian, bool load) {
    if (!active) return;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    const int32_t month = date.year * 12 + date.month;
    if (lastMonth == month) return; // One dated summary even if load is month start.
    lastMonth = month;
    uint32_t fields = 0, mismatches = 0;
    auto* fw = RZGetFrameWork();
    auto* app = fw ? fw->Application() : nullptr; // borrowed
    cISC4App* sc4 = nullptr;
    if (app && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) && sc4) {
        FC4_ACQUIRED(sc4, "QueryInterface");
        // Spec 1.4: keep acquired application live throughout borrowed use.
        auto* p = sc4->GetRenderProperties();
        for (uint32_t kind = 0; kind < 3; ++kind) {
            const int32_t count = fc4_renderprops_shadow_count(kind);
            const int32_t nativeCount = !p ? -1 : kind == 0 ? p->NumBoolProperties() :
                kind == 1 ? p->NumIntProperties() : p->NumFloatProperties();
            ++fields;
            mismatches += nativeCount != count;
            for (int32_t i = 0; i < count; ++i) {
                const bool valid = p && i < nativeCount;
                for (uint32_t probe = 0; probe < 6; ++probe) {
                    char query[128]{};
                    const int32_t expected = fc4_renderprops_shadow_query(kind, i, probe, query, sizeof(query));
                    if (probe == 0) {
                        const char* name = !valid ? nullptr : kind == 0 ? p->NameFromBoolPropertyID(i) :
                            kind == 1 ? p->NameFromIntPropertyID(i) : p->NameFromFloatPropertyID(i);
                        ++fields;
                        mismatches += !name || std::strcmp(name, query) != 0;
                    }
                    ++fields;
                    const int32_t actual = !p ? INT32_MIN : kind == 0 ? p->BoolPropertyIDFromName(query) :
                        kind == 1 ? p->IntPropertyIDFromName(query) : p->FloatPropertyIDFromName(query);
                    mismatches += actual != expected;
                }
            }
        }
        FC4_RELEASE(sc4);
    } else {
        // Binding failure cannot produce a vacuous exact sample.
        fields = mismatches = 542;
    }
    fc4::Log("renderprops shadow %04d-%02d-%02d: phase=%s scope=registry-ascii fields=%u values=uncompared(RP7) mismatches=%u",
        date.year, date.month, date.day_of_month, load ? "load" : "month", fields, mismatches);
    ++lines;
    clean += mismatches == 0;
}
}

namespace fc4 {
void RenderPropsShadowCityLoaded(cISC4City* city, int32_t julian) {
    active = city && SettingInt("renderprops_shadow", nullptr, 0) != 0;
    lines = clean = 0;
    lastMonth = -1;
    if (!active) return;
    Log("renderprops: ON; explicit C-locale ASCII registry profile; RP7 live setter inputs unavailable; values/setters/lifetime/view copy uncompared; no whole-owner credit");
    if (auto* sim = AsGame(city->GetSimulator())) julian = sim->GetSimDateNumber();
    Compare(julian, true);
}
void RenderPropsShadowMonth(int32_t julian) { Compare(julian, false); }
void RenderPropsShadowCityShutdown() {
    if (active) Log("renderprops shadow: city shutdown: %u lines, %u with mismatches=0; scope=registry-ascii", lines, clean);
    active = false;
}
}
