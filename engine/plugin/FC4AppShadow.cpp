// SPDX-License-Identifier: GPL-3.0-or-later
// application.md sections 1, 2, 6.2, 9. AP17: bounded scope, no whole-owner credit.
#include "FC4AppShadow.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4DbpfNative.h"
#include "compat/FC4Compat.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cISC4App.h"
#include "cRZCOMDllDirector.h"
#include "cRZBaseString.h"
#include "fc4.h"
#include <cstring>
#include <windows.h>
#include <intrin.h>

extern "C" {
void* fc4_app_shadow_new();
void fc4_app_shadow_free(void*);
uint32_t fc4_app_shadow_input(void*, uint32_t, const char*, uint32_t);
void fc4_app_shadow_message(void*, uint32_t);
uint32_t fc4_app_shadow_mode(void*, uint32_t, uint32_t);
uint32_t fc4_app_shadow_compare(void*, uint32_t, const char*, uint32_t);
uint32_t fc4_app_shadow_expected(void*, uint32_t, char*, uint32_t);
uint32_t fc4_app_shadow_method_stats(const void*, uint32_t, uint64_t*);
uint32_t fc4_app_shadow_delivery(void*, uint32_t, uint32_t, uint32_t);
uint32_t fc4_app_shadow_scalar(void*, uint32_t, uint32_t, uint32_t, uint32_t, uint32_t);
}

namespace {
void* model = nullptr;
bool seeded = false;
bool layoutSupported = false;
bool pathCacheDiagnostic = false;
uint32_t lines = 0, clean = 0, startupLines = 0, startupClean = 0, events = 0;
uint32_t cacheLines = 0, cacheClean = 0;
using MessageFn = bool (__fastcall*)(void*, void*, cIGZMessage2*);
using SaveFn = bool (__fastcall*)(void*, void*, const cIGZString&, bool);
MessageFn nativeMessage = nullptr;
SaveFn nativeSave = nullptr;
void** messageEntry = nullptr;
void** saveEntry = nullptr;
void* appTarget = nullptr;
void* saveApp = nullptr;
uint32_t delivery = 0;
uint32_t deliveryMismatches = 0;
thread_local uint32_t deliveryDepth = 0;

bool ModeInput(uint32_t type) {
    return type == 0x26D31EC2 || type == 0x8BB5BB46 || type == 0xCBB5BB45 || type == 0xEA8AE29A;
}

bool Exchange(void** entry, void* value) {
    DWORD old = 0;
    if (!VirtualProtect(entry, sizeof(void*), PAGE_READWRITE, &old)) return false;
    InterlockedExchangePointer(entry, value);
    DWORD ignored = 0;
    VirtualProtect(entry, sizeof(void*), old, &ignored);
    return true;
}

// AP19: observe the app's actual target, not our later notification. Forward
// the identical message once, without pumping or changing subscriber order.
bool __fastcall MessageForward(void* self, void* edx, cIGZMessage2* msg) {
    const auto original = nativeMessage;
    const uint32_t type = msg ? msg->GetType() : 0;
    const bool observe = self == appTarget && ModeInput(type);
    const uint32_t id = observe ? ++delivery : 0;
    if (observe) {
        ++deliveryDepth;
        fc4::Log("app input entry: id=%u thread=%lu depth=%u target=%p message=%p type=0x%08X",
            id, GetCurrentThreadId(), deliveryDepth, self, msg, type);
    }
    const bool result = original(self, edx, msg);
    if (observe) {
        fc4::AppShadowMessage(type);
        fc4::Log("app input return: id=%u thread=%lu depth=%u target=%p type=0x%08X result=%u",
            id, GetCurrentThreadId(), deliveryDepth, self, type, result ? 1u : 0u);
        --deliveryDepth;
    }
    if (self == appTarget && model) {
        // Section 1.2: mode at whole-object +0x44 (message target is +4).
        // Do not use this output as a model input. All deliveries count, even
        // when their timer/collaborator effects cannot yet be compared.
        const uint32_t mode = *reinterpret_cast<const uint32_t*>(static_cast<const char*>(self) + 0x40);
        if (fc4_app_shadow_delivery(model, type, result ? 1u : 0u, mode)) {
            ++deliveryMismatches;
            fc4::Log("app: delivery differs; type=0x%08X result=%u mode=%u", type, result ? 1u : 0u, mode);
        }
    }
    return result;
}

// EX58: all public slot-15 callers, including independent native callers.
// Execution here after original() is the actual caller continuation, not a
// PostSave/next-tick guess. Untraced PreSave still invalidates without replay.
bool __fastcall SaveForward(void* self, void* edx, const cIGZString& path, bool fast) {
    const auto original = nativeSave;
    const bool observe = self == saveApp;
    if (observe) fc4::DbpfShadowSaveEnter(self, _ReturnAddress());
    const bool result = original(self, edx, path, fast);
    if (observe) fc4::DbpfShadowSaveReturned(self, result);
    return result;
}
// AP3: exact slots, never the incompatible SDK directory names/signatures.
constexpr uint32_t offsets[] = {0x7C, 0x90, 0xA4, 0xB8, 0xCC, 0xE0, 0x108, 0x11C};

// Only spec-published addresses; unresolved identities retain interface slots.
// Counts cover our explicit same-input getter calls, not total native entries.
constexpr const char* methodIds[][2] = {
    {"0x00449DE0", "cSC4App::GetDataDirectory"},
    {"0x00449E00", "cSC4App::GetCDDataDirectory"},
    {"0x0044BF90", "cSC4App::GetPluginDirectory"},
    {"0x0044C010", "cSC4App::GetCDPluginDirectory"},
    {"0x00449E20", "cSC4App::GetSkuSpecificDirectory"},
    {"0x00449E40", "cSC4App::GetUserDataDirectory"},
    {"0x00449E60", "cSC4App::GetDirectorySlot57"},
    {"0x00449E80", "cSC4App::GetRegionsDirectory"}, // AP3: slot58 is the stored user base.
    {"-", "cISC4App::slot59"}, {"-", "cISC4App::slot60"},
    {"-", "cISC4App::slot61"}, {"-", "cISC4App::slot62"},
    {"-", "cISC4App::slot63"}, {"-", "cISC4App::slot64"},
    {"-", "cISC4App::slot65"}, {"-", "cISC4App::slot66"},
    {"-", "cISC4App::slot28"}, {"-", "cISC4App::slot30"},
    {"0x0080BC00", "cSC4App::GetMode"},
    {"0x00451ED0", "cSC4App::DoMessage"},
    {"-", "cISC4App::slot21"}, {"-", "cISC4App::slot23"},
    {"0x0068D1B0", "cSC4App::GetStored4C"},
    {"0x006EFA20", "cSC4App::GetFeatureManager"},
    {"0x00698CF0", "cSC4App::GetRegionOwner"},
    {"-", "cISC4App::slot36"},
    {"0x0081F910", "cSC4App::GetCity"},
    {"-", "cISC4App::slot40"}, {"-", "cISC4App::slot41"},
    {"-", "cISC4App::slot43"}, {"-", "cISC4App::slot45"},
    {"-", "cISC4App::slot46"}, {"-", "cISC4App::slot47"},
    {"-", "cISC4App::slot48"}, {"-", "cISC4App::slot49"},
    {"-", "cISC4App::slot50"},
};

void ReportMethods(const fc4_date& date) {
    for (uint32_t field = 0; field < sizeof(methodIds) / sizeof(methodIds[0]); ++field) {
        if ((field == 14 || field == 15) && !pathCacheDiagnostic) continue;
        uint64_t stats[3]{};
        if (fc4_app_shadow_method_stats(model, field, stats) != 0) continue;
        fc4::Log("app method %s %s (month) %04d-%02d-%02d: calls=%llu complete=%llu mismatches=%llu; resyncs=0",
            methodIds[field][0], methodIds[field][1], date.year, date.month, date.day_of_month,
            static_cast<unsigned long long>(stats[0]), static_cast<unsigned long long>(stats[1]),
            static_cast<unsigned long long>(stats[2]));
    }
}

// Section 2: independent pre-call owner fields, exact scalar returns and the
// post-call field. Embedded getters depend only on the owner identity. No
// QueryInterface/AddRef/Release is performed on these borrowed return values.
void CompareScalars(cISC4App* app, uint32_t& fields, uint32_t& differences) {
    struct Getter { uint32_t slot, offset; bool byte; };
    constexpr Getter getters[] = {
        {21, 0x650, true}, {23, 0x651, true}, {26, 0x4C, false},
        {32, 0x314, false}, {34, 0x54, false}, {36, 0x58, false},
        {37, 0x5C, false}, {40, 0x5BC, false}, {41, 0x5C0, false},
        {43, 0x5C8, false}, {45, 0, false}, {46, 0x660, false},
        {47, 0x664, false}, {48, 0, false}, {49, 0, false}, {50, 0x668, true},
    };
    const auto* object = reinterpret_cast<const char*>(app) - 0x0C;
    auto** table = *reinterpret_cast<void***>(app);
    for (uint32_t i = 0; i < sizeof(getters) / sizeof(getters[0]); ++i) {
        const auto& getter = getters[i];
        const auto read = [&]() -> uint32_t {
            if (!getter.offset) return 0;
            const uint32_t stored = getter.byte ? *reinterpret_cast<const uint8_t*>(object + getter.offset) :
                *reinterpret_cast<const uint32_t*>(object + getter.offset);
            // Section 11: +0x58 holds a reference slot, whose first word is
            // the regional-city token used by the portable application model.
            return getter.slot == 36 && stored ?
                *reinterpret_cast<const uint32_t*>(static_cast<uintptr_t>(stored)) : stored;
        };
        const uint32_t before = read();
        uint32_t result = 0;
        if (getter.byte) {
            using ByteGetter = uint8_t (__thiscall*)(void*);
            result = reinterpret_cast<ByteGetter>(table[getter.slot])(app);
        } else {
            using WordGetter = uint32_t (__thiscall*)(void*);
            result = reinterpret_cast<WordGetter>(table[getter.slot])(app);
        }
        const uint32_t after = read();
        const uint32_t mismatch = fc4_app_shadow_scalar(model, i, before, result, after,
            static_cast<uint32_t>(reinterpret_cast<uintptr_t>(object)));
        differences += mismatch;
        ++fields;
        if (mismatch) fc4::Log("app: scalar differs; slot=%u before=0x%08X result=0x%08X after=0x%08X",
            getter.slot, before, result, after);
    }
}

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
    uint32_t differences = deliveryMismatches, fields = 0;
    uint32_t cacheDifferences = 0, cacheFields = 0;
    if (!app || !layoutSupported) {
        ++differences; // Unavailable samples must not be counted as exact.
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
        // Inputs were captured at the native app target's normal returns.
        using Mode = uint32_t (__thiscall*)(void*);
        const uint32_t nativeMode = reinterpret_cast<Mode>(table[25])(app);
        const uint32_t modeMismatch = fc4_app_shadow_mode(model, nativeMode, messageEntry ? 1u : 0u);
        differences += modeMismatch;
        if (!messageEntry) ++differences; // missing app-input coverage is never exact
        if (modeMismatch) fc4::Log("app: mode differs; native=%u observed_events=%u", nativeMode, events);
        ++fields;
        CompareScalars(app, fields, differences);
    }
    if (app) FC4_RELEASE(app);
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    fc4::Log("%s shadow %04d-%02d-%02d: scope=paths-mode phase=%s fields=%u mode_events=%u resyncs=0; mismatches=%u",
        load ? "app-startup" : "app", date.year, date.month, date.day_of_month,
        load ? "first-city-quiet-point" : "month", fields, events, differences);
    if (load) { ++startupLines; startupClean += differences == 0; }
    else { ++lines; clean += differences == 0; }
    if (!load) ReportMethods(date);
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
    const bool appEnabled = SettingInt("app_shadow", nullptr, 0) != 0;
    const bool dbpfEnabled = SettingInt("dbpf_shadow", nullptr, 0) != 0 ||
        SettingInt("dbpf_bounds_shadow", nullptr, 0) != 0;
    if ((!appEnabled && !dbpfEnabled) || model || messageEntry || saveEntry) return;
    if (appEnabled) model = fc4_app_shadow_new();
    seeded = loadDue = layoutSupported = false;
    lines = clean = startupLines = startupClean = events = 0;
    cacheLines = cacheClean = 0;
    deliveryMismatches = 0;
    pathCacheDiagnostic = SettingInt("app_shadow_path_cache", nullptr, 0) != 0;
    // application.md 1.2 and 6.2.1: install before the first city transition.
    // Validate the identified image's published interface tables, never code bytes.
    auto* app = AcquireApp();
    const auto* version = compat::IdentifiedVersion();
    if (app && version && version->imageBase == 0x00400000) {
        auto** table = *reinterpret_cast<void***>(app);
        layoutSupported = reinterpret_cast<uintptr_t>(table) == 0x00A86AF0;
        auto* target = reinterpret_cast<char*>(app) - 8; // app object +4 (section 1.2)
        auto** messages = *reinterpret_cast<void***>(target);
        if (model && reinterpret_cast<uintptr_t>(messages) == 0x00A86C1C) {
            nativeMessage = reinterpret_cast<MessageFn>(messages[3]);
            appTarget = target;
            if (Exchange(messages + 3, reinterpret_cast<void*>(&MessageForward))) messageEntry = messages + 3;
        }
        if (dbpfEnabled && reinterpret_cast<uintptr_t>(table) == 0x00A86AF0) {
            nativeSave = reinterpret_cast<SaveFn>(table[15]);
            saveApp = app;
            if (Exchange(table + 15, reinterpret_cast<void*>(&SaveForward))) saveEntry = table + 15;
        }
        Log("app observation: target=%p message_forward=%u save_app=%p save_forward=%u before-city-transitions=1",
            appTarget, messageEntry ? 1u : 0u, saveApp, saveEntry ? 1u : 0u);
    }
    if (app) FC4_RELEASE(app);
    if (model) Log("app: ON; 33 supported path/filename/mode/scalar fields; AP18 excludes slots65/66 (app_shadow_path_cache diagnostic=%u); AP17 excludes startup replay, pause/speed, frame/tick counters, idle/pump decisions; no whole-owner credit", pathCacheDiagnostic ? 1u : 0u);
    // SPEC-GAP: AP6/AP17 lack pre-startup inputs, timer accessors at OnIdle,
    // and shutdown helper destruction order; none earns complete method credit.
    if (model) Log("app: method coverage bounded to getter calls; string getters complete=0 (AP21 return unobserved; lazy cache writes/base calls also unobserved); startup/OnIdle/shutdown unobserved (AP6/AP17)");
    if (model) Log("app: app-target method captures every delivery return and mode-branch owner output; complete=0 without GetType/collaborator trace");
}
void AppShadowMessage(uint32_t kind) {
    if (!model) return;
    if (ModeInput(kind)) {
        fc4_app_shadow_message(model, kind);
        ++events;
        Log("app: mode message=0x%08X observed_events=%u", kind, events);
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
    if (messageEntry) Exchange(messageEntry, reinterpret_cast<void*>(nativeMessage));
    if (saveEntry) Exchange(saveEntry, reinterpret_cast<void*>(nativeSave));
    messageEntry = saveEntry = nullptr;
    appTarget = saveApp = nullptr;
    if (!model) return;
    Log("app shadow: shutdown: scope=paths-mode %u lines, %u with mismatches=0; resyncs=0", lines, clean);
    Log("app-startup shadow: shutdown: scope=first-city-paths-mode %u lines, %u with mismatches=0; resyncs=0", startupLines, startupClean);
    if (pathCacheDiagnostic) Log("app-path-cache shadow: shutdown: AP18 %u lines, %u with mismatches=0; resyncs=0", cacheLines, cacheClean);
    fc4_app_shadow_free(model);
    model = nullptr;
}
}
