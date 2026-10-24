// SPDX-License-Identifier: GPL-3.0-or-later
// Exercise the real probe and Rust transport, including the two SDK signature
// corrections. No native class ID/factory route is specified for this owner.
#include "../plugin/FC4OverlaysShadow.cpp"
#include <cstdio>
#include <cstdlib>

namespace {
long enabled = 0;
unsigned calls = 0, observations = 0, summaries = 0;
bool valid = true;
void __fastcall Unexpected(void*, void*) { std::abort(); }
bool __fastcall Enabled(void*, void*, uint32_t id) {
    valid &= id == UINT32_MAX; ++calls; return false;
}
float __fastcall Alpha(void*, void*, uint32_t id) {
    valid &= id == UINT32_MAX; ++calls; return 0.0f;
}
float* __fastcall Color(void*, void*, float* out, uint32_t id) {
    valid &= id == UINT32_MAX; ++calls;
    out[0] = out[1] = out[2] = 0.0f;
    return out;
}
uint8_t __fastcall Mode(void*, void*, uint32_t id) {
    valid &= id == UINT32_MAX; ++calls; return 0;
}
bool __fastcall SelfLit(void*, void*, uint32_t id) {
    valid &= id == UINT32_MAX; ++calls; return false;
}
void __fastcall Placement(void*, void*, uint32_t id, void*) {
    valid &= id == UINT32_MAX || id == 0; ++calls;
    // Both sentinel and direct-ID placement outputs remain untouched (§2.3).
}
}

namespace fc4 {
long SettingInt(const char*, const char*, long) { return enabled; }
long PublishedJulian() { return 0; }
void Log(const char*, ...) {}
void OverlaysObservationReset() { observations = summaries = 0; }
void OverlaysObservationSample(cISTEOverlayManager*, uint32_t, const char*, int32_t, bool) { ++observations; }
void OverlaysObservationSummary() { ++summaries; }
}

int main() {
    using namespace fc4;
    static_assert(sizeof(void*) == 4, "x86 thiscall contract");
    std::array<void*, 28> table;
    table.fill(reinterpret_cast<void*>(&Unexpected));
    table[14] = reinterpret_cast<void*>(&Placement);
    table[22] = reinterpret_cast<void*>(&Enabled);
    table[23] = reinterpret_cast<void*>(&Alpha);
    table[24] = reinterpret_cast<void*>(&Color);
    table[25] = reinterpret_cast<void*>(&Mode);
    table[26] = reinterpret_cast<void*>(&SelfLit);
    struct Object { void** table; } object{table.data()};
    // Every other slot aborts, including AddRef/Release and all mutations.
    const auto actual = ReadContract(reinterpret_cast<cISTEOverlayManager*>(&object));
    std::array<uint32_t, 25> expected{};
    if (!fc4_overlays_verify_contract(expected.data(), 25) || actual != expected ||
        !valid || calls != 7) return 1;

    // Default-off must never dereference even an invalid borrowed city pointer.
    auto* poison = reinterpret_cast<cISC4City*>(1);
    OverlaysShadowCityLoaded(poison);
    OverlaysShadowOnTick(poison);
    OverlaysShadowMonth(poison, 0);
    OverlaysShadowCityShutdown();
    if (g_lines || g_fields || observations || summaries) return 2;

    // A city which never exposes a view must fail closed on shutdown. A prior
    // unavailable monthly sample does not cancel the pending load requirement.
    enabled = 1;
    OverlaysShadowCityLoaded(nullptr);
    OverlaysShadowOnTick(nullptr);
    if (!g_loadPending || g_lines) return 3;
    OverlaysShadowMonth(nullptr, 0);
    OverlaysShadowCityShutdown();
    if (g_loadPending || g_enabled || g_lines != 2 || g_clean ||
        g_fields != 200 || g_differences != 200 || observations != 8 || summaries != 1) return 4;
    // Shutdown is idempotent; loading another disabled city clears stale totals.
    OverlaysShadowCityShutdown();
    if (g_lines != 2 || summaries != 1) return 5;
    enabled = 0;
    OverlaysShadowCityLoaded(nullptr);
    if (g_lines || g_fields || g_differences || observations || summaries) return 6;
    std::puts("overlays contract: PASS (x86 slots, core parity, borrowed references, opt-in, unavailable load)");
    return 0;
}
