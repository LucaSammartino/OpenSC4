// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4WeatherShadow.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4PropCityCommon.h"
#include "FC4RefAudit.h"
#include "FC4SCResShadow.h"
#include "FC4Subsystems.h"
#include "FC4WriteCapture.h"
#include "cFC4Weather.h"
#include "cISC4City.h"
#include "cIGZMessageTarget2.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "compat/FC4Compat.h"
#include <windows.h>
#include <algorithm>
#include <cstring>

namespace fc4 {
namespace {
cFC4WeatherSimulator* g_core = nullptr;
cIGZUnknown* g_original = nullptr; // borrowed for this city's lifetime
uint64_t g_differences = 0, g_steps = 0, g_lines = 0, g_clean = 0;
bool g_recordDifferenceLogged = false;
size_t g_firstOffset = 0;
void Compare();
using MoistureFn = void (__fastcall*)(void*, void*, const SC4Rect<int32_t>&);
using AdditionFn = void (__fastcall*)(void*, void*, float, float, float, float, float);
MoistureFn g_moisture = nullptr;
AdditionFn g_addition = nullptr;
void** g_vtable = nullptr;

bool PatchSlot(unsigned slot, void* value) {
    DWORD old = 0;
    if (!VirtualProtect(g_vtable + slot, sizeof(void*), PAGE_EXECUTE_READWRITE, &old)) return false;
    InterlockedExchangePointer(g_vtable + slot, value);
    DWORD ignored = 0;
    return VirtualProtect(g_vtable + slot, sizeof(void*), old, &ignored) != 0;
}

void __fastcall UpdateMoisture(void* self, void* edx, const SC4Rect<int32_t>& rect) {
    g_moisture(self, edx, rect);
    if (self == g_original && g_core) {
        fc4_weather_update_moisture(g_core->Core(), rect.topLeftX, rect.topLeftY,
                                    rect.bottomRightX, rect.bottomRightY);
        ++g_steps;
        Compare();
    }
}

void __fastcall ManualAddition(void* self, void* edx, float x0, float z0, float x1, float z1, float value) {
    g_addition(self, edx, x0, z0, x1, z1, value);
    if (self == g_original && g_core) {
        fc4_weather_set_manual_moisture_addition(g_core->Core(), x0, z0, x1, z1, value);
        ++g_steps;
        Compare();
    }
}

// PV14 section 5.2 publishes this exact observation point: EAX is the real
// calling thread's runtime data, before rand reads +0x14. A thread-local
// hardware execution breakpoint observes that point; no game code is patched
// and no call to __getptd, rand or srand is made. Private continuable exceptions
// install/restore DR0 through the current thread's genuine exception context.
// The SCRes observer may lend its idle slot for the synchronous weekly call:
// weather 5.2 enters no resource parser and posts (does not dispatch) messages.
// Preserve its full DR0/DR6/DR7 state, including all other enabled breakpoints.
// All draws in the scoped operation must form a contiguous CRT sequence.
constexpr DWORD kConfigureObserver = 0xE042FC40;
constexpr DWORD kRandSite = 0x009EFBA4;
DWORD g_thread = 0;
DWORD g_savedDr0 = 0, g_savedDr6 = 0, g_savedDr7 = 0;
DWORD g_observedDr7 = 0;
bool g_configured = false;
uint32_t g_randBefore = 0, g_randAfter = 0, g_randCount = 0;
bool g_randValid = true;
PVOID g_handler = nullptr;

LONG CALLBACK ObserveRand(EXCEPTION_POINTERS* p) {
#if defined(_M_IX86)
    auto* c = p->ContextRecord;
    const DWORD thread = GetCurrentThreadId();
    if (thread != g_thread || g_thread == 0) return EXCEPTION_CONTINUE_SEARCH;
    if (p->ExceptionRecord->ExceptionCode == kConfigureObserver &&
        p->ExceptionRecord->NumberParameters == 1) {
        c->ContextFlags |= CONTEXT_DEBUG_REGISTERS;
        if (p->ExceptionRecord->ExceptionInformation[0] == 1) {
            g_observedDr7 = c->Dr7;
            if ((c->Dr7 & 3) != 0 &&
                !((c->Dr7 & 0xF0000u) == 0 && SCResShadowCanLendBreakpoint(c->Dr0))) {
                g_randValid = false;
                return EXCEPTION_CONTINUE_EXECUTION;
            }
            g_savedDr0 = c->Dr0;
            g_savedDr6 = c->Dr6;
            g_savedDr7 = c->Dr7;
            c->Dr0 = kRandSite;
            c->Dr6 &= ~1u;
            c->Dr7 = (c->Dr7 & ~0xF0000u) | 1u; // local, execute, one byte
            g_configured = true;
        } else {
            c->Dr0 = g_savedDr0;
            c->Dr6 = g_savedDr6;
            c->Dr7 = g_savedDr7;
            g_configured = false;
        }
        return EXCEPTION_CONTINUE_EXECUTION;
    }
    if (p->ExceptionRecord->ExceptionCode == EXCEPTION_SINGLE_STEP && g_configured &&
        c->Eip == kRandSite && (c->Dr6 & 1) != 0) {
        const uint32_t state = *reinterpret_cast<const uint32_t*>(c->Eax + 0x14);
        if (g_randCount == 0) g_randBefore = state;
        else if (state != g_randAfter) g_randValid = false;
        g_randAfter = state * 214013u + 2531011u;
        ++g_randCount;
        c->Dr6 &= ~1u;
        c->EFlags |= 0x10000; // RF: execute this instruction once, keep breakpoint armed
        return EXCEPTION_CONTINUE_EXECUTION;
    }
#else
    (void)p;
#endif
    return EXCEPTION_CONTINUE_SEARCH;
}

bool BeginRand() {
    g_randCount = 0;
    g_randValid = true;
    // Only the published, nonrelocated 1.1.641 module mapping is supported.
    const auto* version = compat::IdentifiedVersion();
    if (version == nullptr || version->imageBase != 0x00400000 ||
        reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr)) != 0x00400000) return false;
    if (g_handler == nullptr) g_handler = AddVectoredExceptionHandler(1, ObserveRand);
    if (g_handler == nullptr) return false;
    g_thread = GetCurrentThreadId();
    const ULONG_PTR operation = 1;
    RaiseException(kConfigureObserver, 0, 1, &operation);
    if (!g_configured)
        Log("weather shadow observer: configuration failed valid=%d dr7=%08X", g_randValid ? 1 : 0, g_observedDr7);
    if (!g_configured) g_thread = 0;
    return g_configured;
}

bool EndRand() {
    const ULONG_PTR operation = 2;
    RaiseException(kConfigureObserver, 0, 1, &operation);
    g_thread = 0;
    return !g_configured && g_randValid;
}

bool CaptureOriginal(WriteCapture& cap) {
    return g_original != nullptr && cap.Capture(g_original);
}

void Compare() {
    WriteCapture original, ours;
    if (!CaptureOriginal(original) || !ours.Capture(static_cast<cFC4ISC4WeatherSimulator*>(g_core))) {
        ++g_differences;
        return;
    }
    const auto& a = original.Bytes();
    const auto& b = ours.Bytes();
    uint64_t differences = a.size() > b.size() ? a.size() - b.size() : b.size() - a.size();
    for (size_t i = 0; i < std::min(a.size(), b.size()); ++i) {
        if (a[i] != b[i]) {
            if (differences == 0) g_firstOffset = i;
            ++differences;
        }
    }
    if (differences && !g_recordDifferenceLogged) {
        g_recordDifferenceLogged = true;
        Log("weather shadow: first difference step=%llu byte=%u original/ours=%u/%u bytes=%u/%u",
            g_steps, static_cast<unsigned>(g_firstOffset),
            g_firstOffset < a.size() ? a[g_firstOffset] : 0,
            g_firstOffset < b.size() ? b[g_firstOffset] : 0,
            static_cast<unsigned>(a.size()), static_cast<unsigned>(b.size()));
    }
    g_differences += differences;
}

void Line(const char* phase, int32_t julian) {
    char date[16];
    propcity::DateText(julian, date);
    ++g_lines;
    if (g_differences == 0) ++g_clean;
    Log("weather shadow %s: phase=%s mismatches=%llu steps=%llu record=all-bytes crt=observed first-byte=%u",
        date, phase, g_differences, g_steps, static_cast<unsigned>(g_firstOffset));
}
}

void WeatherShadowCityLoaded(cISC4City* city) {
    if (!SettingInt("weather_shadow", "FC4_WEATHER_SHADOW", 0) || city == nullptr) return;
    if (FindEnabledClass(kFC4_CLSID_cSC4WeatherSimulator) != nullptr) {
        Log("weather shadow: INACTIVE (weather replaced)");
        return;
    }
    auto* original = city->GetWeatherSimulator();
    if (original == nullptr) return;
    if (compat::IdentifiedVersion() == nullptr) {
        Log("weather shadow: INACTIVE (executable identity unavailable; no native observation installed)");
        return;
    }
    g_original = reinterpret_cast<cIGZUnknown*>(original);
    // Weather section 4 slots 16/20: preserve public terrain-tool inputs as
    // well as scheduled messages. Forward each native call exactly once.
    g_vtable = *reinterpret_cast<void***>(original);
    g_moisture = reinterpret_cast<MoistureFn>(g_vtable[16]);
    g_addition = reinterpret_cast<AdditionFn>(g_vtable[20]);
    if (!PatchSlot(16, reinterpret_cast<void*>(&UpdateMoisture))) ++g_differences;
    if (!PatchSlot(20, reinterpret_cast<void*>(&ManualAddition))) ++g_differences;
    Log("weather shadow: ON (weather_shadow): bootstrap at first agent delivery after PostCityInit");
}

namespace {
void Bootstrap() {
    g_core = new cFC4WeatherSimulator(true);
    g_core->AddRef();
    // Bootstrap once at city load. Init establishes host/tuning; Read then
    // restores the complete original record, including RNG and iterator.
    // No month/step ever copies the original's model state back into ours.
    g_core->Init();
    WriteCapture capture;
    if (!CaptureOriginal(capture)) { ++g_differences; return; }
    CaptureIStream input(capture);
    if (!fc4_weather_read(g_core->Core(), input.Get()) || !input.atEnd()) ++g_differences;
    Compare();
    Log("weather shadow: ON (weather_shadow): load bootstrap; independent day/week/month stepping, full record comparison");
}
}

bool WeatherShadowDeliver(cIGZMessageTarget2* target, const char* name, cIGZMessage2* msg,
                          uint32_t type, int64_t data1, bool* finished) {
    if (g_original == nullptr || name == nullptr || std::strcmp(name, "cSC4WeatherSimulator") != 0 ||
        target == nullptr || msg == nullptr || finished == nullptr) return false;
    if (g_core == nullptr) { Bootstrap(); Line("load", static_cast<int32_t>(data1)); }
    const int32_t nativeDate = AsGame(g_core->City()->GetSimulator())->GetSimDateNumber();
    const bool weekly = type == 0x66956815;
    const bool observing = weekly && BeginRand();
    if (g_steps < 12)
        Log("weather shadow delivery: type=%08X delivered=%lld native=%d weekly=%d observing=%d",
            type, data1, nativeDate, weekly ? 1 : 0, observing ? 1 : 0);
    if (weekly && !observing) ++g_differences;
    *finished = target->DoMessage(msg); // exactly one genuine original call
    if (observing && !EndRand()) ++g_differences;
    if (observing && g_randCount != 0) fc4_weather_shadow_crt(g_core->Core(), &g_randBefore, true);
    if (observing && g_randCount == 0) fc4_weather_shadow_crt(g_core->Core(), &g_randAfter, false);
    // The delivered month data3 is the current calendar month (weather 5.3).
    cIGZMessage2Standard* standard = nullptr;
    uint32_t month = 0;
    if (msg->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&standard)) && standard) {
        FC4_ACQUIRED(standard, "QueryInterface");
        month = static_cast<uint32_t>(standard->GetData3());
        FC4_RELEASE(standard);
    }
    fc4_weather_handle_message(g_core->Core(), type, static_cast<uint32_t>(data1), month);
    if (observing) {
        uint32_t state = 0;
        if (!fc4_weather_shadow_crt(g_core->Core(), &state, false) || state != g_randAfter) ++g_differences;
        if (g_steps < 48)
            Log("weather replay: type=%08X delivered=%lld native=%d draws=%u valid=%d crt=%08X/%08X/%08X",
                type, data1, nativeDate, g_randCount, g_randValid ? 1 : 0,
                g_randBefore, g_randAfter, state);
    }
    ++g_steps;
    Compare();
    return true;
}

void WeatherShadowMonth(cISC4City*, int32_t julian) {
    if (g_core) { Compare(); Line("month", julian); }
}

void WeatherShadowCityShutdown() {
    if (!g_original) return;
    if (g_vtable[16] == reinterpret_cast<void*>(&UpdateMoisture))
        PatchSlot(16, reinterpret_cast<void*>(g_moisture));
    if (g_vtable[20] == reinterpret_cast<void*>(&ManualAddition))
        PatchSlot(20, reinterpret_cast<void*>(g_addition));
    Log("weather shadow: city shutdown: %llu lines, %llu with mismatches=0", g_lines, g_clean);
    if (g_core) { g_core->Shutdown(); g_core->Release(); }
    g_core = nullptr;
    g_original = nullptr;
    g_differences = g_steps = g_lines = g_clean = 0;
    g_recordDifferenceLogged = false;
    if (g_handler) { RemoveVectoredExceptionHandler(g_handler); g_handler = nullptr; }
}
}
