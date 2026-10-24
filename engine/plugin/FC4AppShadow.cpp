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
}

namespace {
void* model = nullptr;
bool seeded = false;
uint32_t lines = 0, clean = 0, startupLines = 0, startupClean = 0, events = 0;
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
            }
            cRZBaseString name;
            app->GetAppName(name);
            differences += fc4_app_shadow_input(model, 8, name.ToChar(), name.Strlen());
            seeded = true;
        }
        for (uint32_t i = 0; i < 18; ++i) {
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
            differences += mismatch;
            ++fields;
            if (mismatch) fc4::Log("app: field=%u slot=%u differs (AP3/AP7); native=%s", i, slot, out.ToChar());
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
}
bool loadDue = false;
int32_t loadJulian = 0;
}

namespace fc4 {
void AppShadowStart() {
    if (model || SettingInt("app_shadow", nullptr, 0) == 0) return;
    model = fc4_app_shadow_new();
    seeded = loadDue = false;
    lines = clean = startupLines = startupClean = events = 0;
    Log("app: ON; paths and lifecycle mode only; AP17 excludes startup replay, pause/speed, frame/tick counters, idle/pump decisions; no whole-owner credit");
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
    if (load) { loadDue = true; loadJulian = julian; return; }
    Report(julian, false);
}
// A tick is an observation scheduling point, not a native frame/tick comparison.
void AppShadowTick() {
    if (model && loadDue) { loadDue = false; Report(loadJulian, true); }
}
void AppShadowStop() {
    if (!model) return;
    Log("app shadow: shutdown: scope=paths-mode %u lines, %u with mismatches=0; resyncs=0", lines, clean);
    Log("app-startup shadow: shutdown: scope=first-city-paths-mode %u lines, %u with mismatches=0; resyncs=0", startupLines, startupClean);
    fc4_app_shadow_free(model);
    model = nullptr;
}
}
