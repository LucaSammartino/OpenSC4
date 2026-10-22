// SPDX-License-Identifier: GPL-3.0-or-later
// Included by cFC4Date.cpp: opt-in independent original-interface verification.
#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4RefAudit.h"
#include "cISC4City.h"
#include "cRZBaseString.h"
#include <cstring>
#include <windows.h>

namespace fc4 {
namespace {
uint64_t dateLiveTotal = 0, dateLiveExact = 0, dateFixtureTotal = 0, dateFixtureExact = 0;
uint64_t dateRefusals = 0, dateFailures = 0;
bool dateRanFixtures = false;
class DateForeignString final : public cRZBaseString {
public:
    explicit DateForeignString(uint32_t result) : cRZBaseString("PREFIX"), residue(result) {}
    std::string captured;
    unsigned copies = 0;
    uint32_t residue;
    void Copy(cIGZString const& source) override {
        captured.assign(source.ToChar(), source.Strlen());
        ++copies; // record the borrowed input, intentionally leave output unchanged
        const uint32_t result = residue;
        __asm mov eax, result
    }
};
uint32_t DateRaw(cFC4IGZDate* date, uint32_t slot, unsigned count = 0,
                 uint32_t a = 0, uint32_t b = 0, uint32_t c = 0) {
    void* fn = (*reinterpret_cast<void***>(date))[slot];
    using F0 = uint32_t(__thiscall*)(void*);
    using F1 = uint32_t(__thiscall*)(void*, uint32_t);
    using F2 = uint32_t(__thiscall*)(void*, uint32_t, uint32_t);
    using F3 = uint32_t(__thiscall*)(void*, uint32_t, uint32_t, uint32_t);
    switch (count) {
        case 0: return reinterpret_cast<F0>(fn)(date);
        case 1: return reinterpret_cast<F1>(fn)(date, a);
        case 2: return reinterpret_cast<F2>(fn)(date, a, b);
        default: return reinterpret_cast<F3>(fn)(date, a, b, c);
    }
}
void DateCheck(const char* kind, uint32_t slot, uint32_t native, uint32_t ours, bool live = false) {
    (live ? dateLiveTotal : dateFixtureTotal)++;
    if (native == ours) (live ? dateLiveExact : dateFixtureExact)++;
    else Log("dates-probe mismatch: kind=%s slot=%u native=%u rust=%u", kind, slot, native, ours);
}
uint32_t DateCloneEax(cFC4IGZDate* source, cFC4IGZDate** out, uint32_t incoming) {
    uint32_t result;
    __asm {
        mov ecx, source
        mov edx, [ecx]
        push out
        mov eax, incoming
        call dword ptr [edx + 36]
        mov result, eax
    }
    return result;
}
void DateQueries(cFC4IGZDate* native, cFC4IGZDate* ours, bool live) {
    for (uint32_t slot : {12u,13u,14u,15u,16u,17u,18u,19u,20u,23u,25u,26u,33u,35u,42u,43u,44u,45u}) {
        unsigned argc = slot == 42 || slot == 44 ? 3 :
            slot == 16 || slot == 33 || slot == 35 || slot == 43 || slot == 45 ? 1 : 0;
        uint32_t a = argc == 3 ? 2 : slot == 43 || slot == 45 ? 2000 : 3;
        uint32_t n = DateRaw(native, slot, argc, a, 29, 2000);
        uint32_t r = DateRaw(ours, slot, argc, a, 29, 2000);
        // Only AL is published for actual bool queries. Full EAX retained for
        // every day/value, Clone and DateString, including zero success residue.
        if (slot == 19 || slot == 20 || slot == 42 || slot == 45) { n &= 255; r &= 255; }
        DateCheck(live ? "captured-live-query" : "fixture-query", slot, n, r, live);
    }
}
struct DateProbeContext { cISC4City* city; const char* phase; };
void DateProbeStep(void* raw) {
    const auto& ctx = *static_cast<DateProbeContext*>(raw);
    auto* sim = ctx.city ? AsGame(ctx.city->GetSimulator()) : nullptr;
    auto* native = sim ? sim->GetSimDate() : nullptr; // borrowed, never Release
    const uintptr_t vptr = native ? *reinterpret_cast<uintptr_t*>(native) : 0;
    // Object identity only, no executable bytes. Actual named owner, §3.
    if (vptr != 0x00ACEA38u) {
        ++dateRefusals;
        Log("dates-probe refused: phase=%s date=%p vptr=%08X expected=cGZDate/00ACEA38 (no independent original)",
            ctx.phase, static_cast<void*>(native), static_cast<unsigned>(vptr));
        return;
    }
    const uint32_t captured = native->DayNumber();
    Log("dates-probe owner: phase=%s acquisition=city.GetSimulator.GetSimDate borrowed=%p owner=cGZDate vptr=%08X captured_day=%u origin=observer-query",
        ctx.phase, static_cast<void*>(native), static_cast<unsigned>(vptr), captured);
    cFC4Date ours(static_cast<int32_t>(captured));
    DateQueries(native, &ours, true); // stateless, same captured input, no resync credit
    if (dateRanFixtures) return;
    dateRanFixtures = true;
    cFC4IGZDate* copy = reinterpret_cast<cFC4IGZDate*>(0xDEADBEEF);
    DateCheck("fixture-clone-eax", 9, native->Clone(&copy), 1);
    if (!copy || copy == reinterpret_cast<cFC4IGZDate*>(0xDEADBEEF)) { ++dateFailures; return; }
    FC4_ACQUIRED(copy, "dates-probe Clone");
    DateCheck("fixture-clone-independent", 9, copy != native, 1);
    DateCheck("fixture-clone-day", 26, copy->DayNumber(), captured);
    DateCheck("fixture-clone-ref1", 1, FC4_ADDREF(copy), 2);
    DateCheck("fixture-clone-ref1", 2, FC4_RELEASE(copy), 1);
    for (uint32_t seed : {0u, 2451546u, 0x12345678u, 0xFFFFFFFFu}) {
        // Explicit independently seeded fixtures, not natural state progression.
        DateCheck("fixture-seed", 4, copy->SetDayNumber(seed), ours.SetDayNumber(seed));
        DateQueries(copy, &ours, false);
        cFC4IGZDate* ourClone = nullptr;
        cFC4IGZDate* nativeClone = nullptr;
        DateCheck("fixture-clone-eax", 9, copy->Clone(&nativeClone), ours.Clone(&ourClone));
        if (!nativeClone || !ourClone) { ++dateFailures; continue; }
        FC4_ACQUIRED(nativeClone, "dates-probe fixture Clone");
        DateCheck("fixture-clone-day", 26, nativeClone->DayNumber(), ourClone->DayNumber());
        DateCheck("fixture-clone-ref1", 1, FC4_ADDREF(nativeClone), ourClone->AddRef());
        DateCheck("fixture-clone-ref1", 2, FC4_RELEASE(nativeClone), ourClone->Release());
        DateCheck("fixture-clone-release", 2, FC4_RELEASE_CREATED(nativeClone), ourClone->Release());
    }
    static const char names[][8] = {"Ma", "", "M\0ay", "mAyX", "May\0Jun", "Ma\xDD", "Ma\xFD", "May"};
    for (const auto& name : names) {
        DateCheck("fixture-month-eax", 3, copy->SetMonthNameDayYear(name, 1, 2000), ours.SetMonthNameDayYear(name, 1, 2000));
        DateCheck("fixture-month-day", 26, copy->DayNumber(), ours.DayNumber());
    }
    SYSTEM_INFO info{};
    GetSystemInfo(&info);
    auto* pages = static_cast<char*>(VirtualAlloc(nullptr, info.dwPageSize * 2, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE));
    if (pages) {
        DWORD old = 0;
        if (VirtualProtect(pages + info.dwPageSize, info.dwPageSize, PAGE_NOACCESS, &old)) {
            char* three = pages + info.dwPageSize - 3;
            std::memcpy(three, "May", 3);
            DateCheck("fixture-three-readable-bytes", 3, copy->SetMonthNameDayYear(three, 1, 2000), ours.SetMonthNameDayYear(three, 1, 2000));
        } else ++dateFailures;
        VirtualFree(pages, 0, MEM_RELEASE);
    } else ++dateFailures;
    copy->SetMonthDayYear(1, 2, 2000);
    ours.SetMonthDayYear(1, 2, 2000);
    for (uint32_t format : {0u,1u,2u,3u,7u,9u,15u,0xFFFFFFFFu}) {
        for (const char* initial : {"PREFIX", ""}) {
            cRZBaseString n(initial), r(initial);
            const uint32_t neax = copy->DateString(n, format);
            const uint32_t reax = ours.DateString(r, format);
            DateCheck("fixture-DateString-EAX", 41, neax, reax);
            DateCheck("fixture-DateString-owning-text", 41, std::strcmp(n.ToChar(), r.ToChar()) == 0, 1);
        }
    }
    // Owning original cRZString destinations as well as foreign destinations.
    for (const char* initial : {"PREFIX", ""}) {
        GameString n{}, r{};
        if (!GameStringInit(n, initial, std::strlen(initial)) || !GameStringInit(r, initial, std::strlen(initial))) {
            ++dateFailures;
            continue;
        }
        auto& ns = *reinterpret_cast<cIGZString*>(&n);
        auto& rs = *reinterpret_cast<cIGZString*>(&r);
        DateCheck("fixture-native-string-EAX", 41, copy->DateString(ns, 0), ours.DateString(rs, 0));
        DateCheck("fixture-native-string-owning", 41, std::strcmp(n.data(), r.data()) == 0, 1);
        DateCheck("fixture-native-string-text", 41, std::strcmp(n.data(), "1/2/2000") == 0, 1);
        GameStringDestroy(n);
        GameStringDestroy(r);
    }
    for (uint32_t sentinel : {0xA5A55A00u, 0x5A5AA501u}) {
        DateForeignString n(sentinel), r(sentinel);
        DateCheck("fixture-foreign-Copy-EAX", 41, copy->DateString(n, 0), ours.DateString(r, 0));
        DateCheck("fixture-foreign-Copy-count", 41, n.copies, r.copies);
        DateCheck("fixture-foreign-Copy-input", 41, n.captured == r.captured && n.captured == "1/2/2000", 1);
        DateCheck("fixture-foreign-Copy-no-mutation", 41,
            std::strcmp(n.ToChar(), "PREFIX") == 0 && std::strcmp(r.ToChar(), "PREFIX") == 0, 1);
    }
    copy->SetDayNumber(0x12345678u);
    ours.SetDayNumber(0x12345678u);
    for (unsigned i = 0; i < 6; ++i) DateCheck("fixture-source-refs", 1, FC4_ADDREF(copy), i + 2);
    cFC4IGZDate* independent = reinterpret_cast<cFC4IGZDate*>(0xDEADBEEF);
    cFC4IGZDate* ourIndependent = reinterpret_cast<cFC4IGZDate*>(0xDEADBEEF);
    DateCheck("fixture-incoming-eax", 9, DateCloneEax(copy, &independent, 0xA5A55A00u),
              DateCloneEax(&ours, &ourIndependent, 0xA5A55A00u));
    FC4_ACQUIRED(independent, "dates-probe refs7 Clone");
    DateCheck("fixture-source-refs-unchanged", 1, FC4_ADDREF(copy), 8);
    DateCheck("fixture-source-refs-unchanged", 2, FC4_RELEASE(copy), 7);
    for (unsigned i = 0; i < 6; ++i) DateCheck("fixture-release-source", 2, FC4_RELEASE(copy), 6 - i);
    DateCheck("fixture-release-source", 2, FC4_RELEASE_CREATED(copy), 0);
    DateCheck("fixture-after-source-release", 26, independent->DayNumber(), ourIndependent->DayNumber());
    DateCheck("fixture-independent-modify", 4, independent->SetDayNumber(0), ourIndependent->SetDayNumber(0));
    DateCheck("fixture-independent-ref1", 1, FC4_ADDREF(independent), ourIndependent->AddRef());
    DateCheck("fixture-independent-ref1", 2, FC4_RELEASE(independent), ourIndependent->Release());
    DateCheck("fixture-independent-release", 2, FC4_RELEASE_CREATED(independent), ourIndependent->Release());
    Log("dates-probe fixture coverage: null/allocation faults=not-executed outer-lock-depth=not-executed executable-locale=uncaptured(DT4) SetTime=refused(DT5); fixtures are not natural calls");
}
}
void DatesProbe(cISC4City* city, const char* phase) {
    if (SettingInt("dates_probe", "FC4_DATES_PROBE", 0) == 0) return;
    g_dateFixture = true;
    DateProbeContext context{city, phase};
    if (!SehGuard("dates-probe", DateProbeStep, &context)) ++dateFailures;
    g_dateFixture = false;
    DatesProbeSummary(phase);
}
void DatesProbeSummary(const char* phase) {
    if (SettingInt("dates_probe", "FC4_DATES_PROBE", 0) == 0) return;
    Log("dates-probe summary: phase=%s live_exact=%llu/%llu fixture_exact=%llu/%llu refusals=%llu failures=%llu fallback=0 resync=0 adapter_natural_value_calls=%llu adapter_fixture_value_calls=%llu SetTime_refusals=%llu locale_unresolved_calls=%llu owner_complete=0 DT4,DT5",
        phase, dateLiveExact, dateLiveTotal, dateFixtureExact, dateFixtureTotal, dateRefusals, dateFailures,
        g_dateNatural.load(), g_dateFixtureCalls.load(), g_dateTimeRefused.load(), g_dateLocaleUnresolved.load());
}
}
