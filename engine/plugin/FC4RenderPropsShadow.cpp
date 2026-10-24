// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4RenderPropsShadow.h"
#include "FC4RenderPropsFixture.h"
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
        fc4::RenderPropsRegistry(p, fields, mismatches);
        uint32_t fixtureFields = 0, fixtureMismatches = 0;
        const bool fixture = fc4::RenderPropsFixture(p, fixtureFields, fixtureMismatches);
        // RP7-7/8: late application attach, no continuous independent observer,
        // and no complete-view bridge. Never bootstrap expected state from P/S.
        fc4::Log("renderprops fixture: available=%u fields=%u mismatches=%u plan=finite-v2 setters=462 registry_samples=9 live_trace=unavailable view_snapshot=unavailable",
            fixture ? 1u : 0u, fixtureFields, fixtureMismatches);
        fields += fixtureFields;
        mismatches += fixtureMismatches + (fixture ? 0 : 1);
        FC4_RELEASE(sc4);
    } else {
        // Binding failure cannot produce a vacuous exact sample.
        fields = mismatches = 542;
    }
    fc4::Log("renderprops shadow %04d-%02d-%02d: phase=%s scope=registry-ascii+owned-fixture fields=%u live_values=unavailable(RP7) view=unavailable(RP7) mismatches=%u",
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
    Log("renderprops: ON; C-locale ASCII registry (542 fields) plus isolated native values/setters/registry stability/P-U lifetime; RP7 live trace and private view snapshot unavailable; no whole-owner credit");
    if (auto* sim = AsGame(city->GetSimulator())) julian = sim->GetSimDateNumber();
    Compare(julian, true);
}
void RenderPropsShadowMonth(int32_t julian) { Compare(julian, false); }
void RenderPropsShadowCityShutdown() {
    if (active) Log("renderprops shadow: city shutdown: %u lines, %u with mismatches=0; scope=registry-ascii+owned-fixture; live trace/view unavailable", lines, clean);
    active = false;
}
}
