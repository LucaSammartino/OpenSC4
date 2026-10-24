// SPDX-License-Identifier: GPL-3.0-or-later
// view-utilities.md §§1,4.3,11.10: local-vector read-only differential.
#include "FC4ViewUtilShadow.h"
#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4WinInterfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4View3DWin.h"
#include "cISC43DRender.h"
#include "cRZCOMDllDirector.h"
#include "fc4.h"
#include <windows.h>
#include <array>
#include <cstring>
#include <limits>

extern "C" void fc4_viewutil_shadow_rotate(uint32_t*, int32_t);

namespace {
struct Vector { float x, y, z; };
static_assert(sizeof(Vector) == 12, "spec float32 triple");
using Words = std::array<uint32_t, 3>;
bool active = false;
int32_t lastMonth = -1;
uint32_t lines = 0, clean = 0;

bool LiveInputs(std::array<Words, 5>& vectors, int32_t& rotation) {
    auto* fw = RZGetFrameWork();
    auto* app = fw ? fw->Application() : nullptr; // borrowed
    cISC4App* sc4 = nullptr;
    if (!app || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || !sc4) return false;
    FC4_ACQUIRED(sc4, "QueryInterface");
    auto* main = reinterpret_cast<cFC4IGZWin*>(sc4->GetMainWindow()); // borrowed
    auto* parent = main ? main->GetChildWindowFromID(0x6104489A) : nullptr; // borrowed
    cISC4View3DWin* view = nullptr;
    bool available = false;
    if (parent && parent->GetChildAs(0x9A47B417, 0xFA47B3F9, reinterpret_cast<void**>(&view)) && view) {
        FC4_ACQUIRED(view, "GetChildAs");
        auto* render = view->GetRenderer(); // borrowed
        const auto* utilities = render ? reinterpret_cast<const unsigned char*>(render->GetViewUtilities()) : nullptr;
        if (utilities) {
            // §1 published cached inputs: camera position, look-at, forward,
            // right, up. Read only; never bootstrap a projection expectation
            // from native projection output. Only the local rotation is tested.
            std::memcpy(&rotation, utilities + 0x10, sizeof(rotation));
            for (size_t i = 0; i < vectors.size(); ++i)
                std::memcpy(vectors[i].data(), utilities + 0x3c + i * 12, 12);
            available = true;
        }
        FC4_RELEASE(view);
    }
    FC4_RELEASE(sc4);
    return available;
}

void Compare(int32_t julian, bool load) {
    if (!active) return;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    const int32_t month = date.year * 12 + date.month;
    if (month == lastMonth) return;
    lastMonth = month;
    uint32_t fields = 0, mismatches = 0, samples = 0;
    std::array<Words, 5> live{};
    int32_t liveRotation = 0;
    const bool available = LiveInputs(live, liveRotation);
    const auto base = reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr));
    // §4.3 explicitly documents free cdecl, in-place float32 triple and signed
    // rotation. Literal void return, §12. Addresses are for supported 1.1.641.
    using Rotate = void(__cdecl*)(Vector*, int32_t);
    auto native = reinterpret_cast<Rotate>(base + (0x007F6CC0 - 0x00400000));
    auto probe = [&](Words input, int32_t rotation) {
        Words expected = input, actual{};
        Vector v;
        std::memcpy(&v, input.data(), sizeof(v));
        native(&v, rotation);
        std::memcpy(actual.data(), &v, sizeof(v));
        fc4_viewutil_shadow_rotate(expected.data(), rotation);
        ++samples;
        for (size_t i = 0; i < actual.size(); ++i) {
            ++fields;
            if (actual[i] != expected[i]) {
                if (mismatches < 8) fc4::Log("viewutil diff: sample=%u rotation=%d word=%u native=%08x core=%08x",
                    samples, rotation, static_cast<unsigned>(i), actual[i], expected[i]);
                ++mismatches;
            }
        }
    };
    // §11.10 plus §4.3 signed remainder, infinities, quiet NaN word copies,
    // signed zero and finite extrema. Signaling NaNs require a numeric-mode
    // contract and are deliberately excluded (VU12); y is still bit-compared.
    const Words fixed[] = {
        {0x40000000, 0x40e00000, 0x41300000}, // (2,7,11)
        {0, 0x80000000, 0x80000000},
        {0x80000000, 0x7fc12345, 0},
        {0x7f800000, 0xff800000, 0xff800000},
        {0x7f7fffff, 0x3f800000, 0xff7fffff},
        {0x7fc23456, 0x80000000, 0xffc34567},
        {1, 0x3f800000, 0x80000001},
    };
    const int32_t rotations[] = {0, 1, 2, 3, -1, -2, -3, -4, 4, 5,
        std::numeric_limits<int32_t>::min(), std::numeric_limits<int32_t>::max()};
    if (base) {
        for (const auto& v : fixed) {
            for (auto r : rotations) probe(v, r);
            if (available) probe(v, liveRotation);
        }
        if (available) for (const auto& v : live) {
            for (auto r : rotations) probe(v, r);
            probe(v, liveRotation);
        }
    }
    // Missing live state or binding is a failed observation, never vacuous exact.
    ++fields;
    mismatches += !available || !base;
    fc4::Log("viewutil shadow %04d-%02d-%02d: phase=%s scope=rotation-only bodies=1/50 live=%u rotation=%d samples=%u fields=%u resyncs=0 mismatches=%u",
        date.year, date.month, date.day_of_month, load ? "load" : "month", available ? 1u : 0u,
        liveRotation, samples, fields, mismatches);
    ++lines;
    clean += mismatches == 0;
}
}

namespace fc4 {
void ViewUtilShadowCityLoaded(cISC4City* city, int32_t julian) {
    active = city && SettingInt("viewutil_shadow", nullptr, 0) != 0;
    lines = clean = 0;
    lastMonth = -1;
    if (!active) return;
    Log("viewutil: ON; partial rotation helper only (1/50 bodies); min/max and remaining owner ABI blocked VU16; no resyncs or native state writes");
    if (auto* sim = AsGame(city->GetSimulator())) julian = sim->GetSimDateNumber();
    Compare(julian, true);
}
void ViewUtilShadowMonth(int32_t julian) { Compare(julian, false); }
void ViewUtilShadowCityShutdown() {
    if (active) Log("viewutil: %u/%u exact (rotation-only), resyncs=0", clean, lines);
    active = false;
}
}
