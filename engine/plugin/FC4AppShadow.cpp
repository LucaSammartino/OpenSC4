// SPDX-License-Identifier: GPL-3.0-or-later
// application.md sections 1, 2, 6.2, 9. AP17: bounded scope, no whole-owner credit.
#include "FC4AppShadow.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cISC4App.h"
#include "cRZCOMDllDirector.h"
#include "cRZBaseString.h"
#include "fc4.h"
#include <cstring>

extern "C" {
void* fc4_app_shadow_new();
void fc4_app_shadow_free(void*);
uint32_t fc4_app_shadow_input(void*, uint32_t, const char*, uint32_t);
void fc4_app_shadow_message(void*, uint32_t);
uint32_t fc4_app_shadow_mode(const void*, uint32_t);
uint32_t fc4_app_shadow_compare(void*, uint32_t, const char*, uint32_t);
uint32_t fc4_app_shadow_expected(void*, uint32_t, char*, uint32_t);
}

namespace {
void* model = nullptr;
bool seeded = false;
bool pathCacheDiagnostic = false;
uint32_t lines = 0, clean = 0, startupLines = 0, startupClean = 0, events = 0;
uint32_t cacheLines = 0, cacheClean = 0;
// AP3: exact slots, never the incompatible SDK directory names/signatures.
constexpr uint32_t offsets[] = {0x7C, 0x90, 0xA4, 0xB8, 0xCC, 0xE0, 0x108, 0x11C};

cISC4App* AcquireApp() {
    auto* fw = RZGetFrameWork();
    auto* app = fw ? fw->Application() : nullptr; // borrowed
    cISC4App* sc4 = nullptr;
    if (app && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) && sc4) {
        FC4_ACQUIRED(sc4, "QueryInterface");
        return sc4;
    }
    return nullptr;
}

void Report(int32_t julian, bool load) {
    if (!model) return;
    auto* app = AcquireApp();
    uint32_t differences = 0, fields = 0;
    uint32_t cacheDifferences = 0, cacheFields = 0;
    if (!app) {
        differences = 1; // Unavailable samples must not be counted as exact.
    } else {
        auto** table = *reinterpret_cast<void***>(app);
        if (!seeded) {
            // Spec section 9 explicitly identifies these as stored char pointers.
            // Take independent inputs once, not getter outputs on every sample.
            const auto* object = reinterpret_cast<const char*>(app) - 0x0C;
            for (uint32_t i = 0; i < 8; ++i) {
                const auto* path = *reinterpret_cast<const char* const*>(object + offsets[i]);
                if (!path) path = "";
                differences += fc4_app_shadow_input(model, i, path, static_cast<uint32_t>(std::strlen(path)));
                fc4::Log("app: input=%u stored=%s", i, path);
            }
            cRZBaseString name;
            app->GetAppName(name);
            differences += fc4_app_shadow_input(model, 8, name.ToChar(), name.Strlen());
            seeded = true;
        }
        for (uint32_t i = 0; i < 18; ++i) {
            // AP18: native prewarmed cache/base and Scripts suffix disagree with
            // section 9. Keep these out of the supported label, never resync.
            const bool unresolvedCache = i == 14 || i == 15;
            if (unresolvedCache && !pathCacheDiagnostic) continue;
            cRZBaseString out("app-shadow-sentinel");
            const uint32_t slot = i < 16 ? 51 + i : i == 16 ? 28 : 30;
            if (slot == 53 || slot == 54) {
                using Getter = void (__thiscall*)(void*, cIGZString&, int32_t);
                reinterpret_cast<Getter>(table[slot])(app, out, -1);
            } else {
                using Getter = void (__thiscall*)(void*, cIGZString&);
                reinterpret_cast<Getter>(table[slot])(app, out);
            }
            const uint32_t mismatch = fc4_app_shadow_compare(model, i, out.ToChar(), out.Strlen());
            if (unresolvedCache) { cacheDifferences += mismatch; ++cacheFields; }
            else { differences += mismatch; ++fields; }
            if (mismatch) {
                char expected[32769]{};
                fc4_app_shadow_expected(model, i, expected, sizeof(expected));
                fc4::Log("app: field=%u slot=%u differs (AP3/AP7/AP18); native=%s expected=%s", i, slot, out.ToChar(), expected);
            }
        }
        // Compare after dispatch has completed (next observer tick for load,
        // monthly quiet point thereafter), so notification order is immaterial.
        using Mode = uint32_t (__thiscall*)(void*);
        const uint32_t nativeMode = reinterpret_cast<Mode>(table[25])(app);
        differences += fc4_app_shadow_mode(model, nativeMode);
        ++fields;
        FC4_RELEASE(app);
    }
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    fc4::Log("%s shadow %04d-%02d-%02d: scope=paths-mode phase=%s fields=%u mode_events=%u resyncs=0; mismatches=%u",
        load ? "app-startup" : "app", date.year, date.month, date.day_of_month,
        load ? "first-city-quiet-point" : "month", fields, events, differences);
    if (load) { ++startupLines; startupClean += differences == 0; }
    else { ++lines; clean += differences == 0; }
    if (pathCacheDiagnostic) {
        if (cacheFields != 2) ++cacheDifferences;
        fc4::Log("app-path-cache shadow %04d-%02d-%02d: phase=%s fields=%u AP18 resyncs=0; mismatches=%u",
            date.year, date.month, date.day_of_month, load ? "first-city-quiet-point" : "month",
            cacheFields, cacheDifferences);
        ++cacheLines; cacheClean += cacheDifferences == 0;
    }
}
bool loadDue = false;
}

namespace fc4 {
void AppShadowStart() {
    if (model || SettingInt("app_shadow", nullptr, 0) == 0) return;
    model = fc4_app_shadow_new();
    seeded = loadDue = false;
    lines = clean = startupLines = startupClean = events = 0;
    cacheLines = cacheClean = 0;
    pathCacheDiagnostic = SettingInt("app_shadow_path_cache", nullptr, 0) != 0;
    Log("app: ON; 17 supported path/filename/mode fields; AP18 excludes slots65/66 (app_shadow_path_cache diagnostic=%u); AP17 excludes startup replay, pause/speed, frame/tick counters, idle/pump decisions; no whole-owner credit", pathCacheDiagnostic ? 1u : 0u);
}
void AppShadowMessage(uint32_t kind) {
    if (!model) return;
    if (kind == 0x26D31EC2 || kind == 0x8BB5BB46 || kind == 0xCBB5BB45 || kind == 0xEA8AE29A) {
        fc4_app_shadow_message(model, kind);
        ++events;
    }
}
void AppShadowObserve(int32_t julian, bool load) {
    if (!model) return;
    if (load) { loadDue = true; return; }
    Report(julian, false);
}
// A tick is an observation scheduling point, not a native frame/tick comparison.
void AppShadowTick(int32_t julian) {
    // Published date is unavailable during PostCityInit; wait for a dated quiet tick.
    if (model && loadDue && julian > 0) { loadDue = false; Report(julian, true); }
}
void AppShadowStop() {
    if (!model) return;
    Log("app shadow: shutdown: scope=paths-mode %u lines, %u with mismatches=0; resyncs=0", lines, clean);
    Log("app-startup shadow: shutdown: scope=first-city-paths-mode %u lines, %u with mismatches=0; resyncs=0", startupLines, startupClean);
    if (pathCacheDiagnostic) Log("app-path-cache shadow: shutdown: AP18 %u lines, %u with mismatches=0; resyncs=0", cacheLines, cacheClean);
    fc4_app_shadow_free(model);
    model = nullptr;
}
}
