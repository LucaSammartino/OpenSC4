// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

#include "FC4ProtectionShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Protection.h"
#include "compat/FC4Compat.h"
#include "fc4.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "cIGZAllocatorService.h"
#include "cISC4City.h"
#include "cISC4FireProtectionSimulator.h"
#include "cISC4Occupant.h"
#include "cISC4PoliceSimulator.h"
#include "cISC4SimGrid.h"

#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kClsU8 = 0x49B9E602;   // the adjusted grid's class (§2)
constexpr uint32_t kClsI16 = 0x49B9E605;  // the unadjusted grid's class (§2)
// Both original classes keep the unadjusted grid pointer at primary +0x84,
// next to the adjusted one at +0x80 (protection.md §2, PR5).
constexpr size_t kUnadjustedOffset = 0x84;
constexpr uint32_t kDispatchType[2] = {1, 0};
const char* const kTag[2] = {"fire", "police"};
constexpr size_t kFirstCells = 8;  // differing cells listed per line

bool g_enabled = false;
// The test seam's stand-ins for the original classes' primary vtables.
bool g_testVtables = false;
const void* g_testVtable[2] = {nullptr, nullptr};

struct KindState {
    std::vector<uint8_t> last;  // our grid at the previous month end
    int32_t lastX = 0, lastZ = 0;
    uint32_t stable = 0;        // consecutive month ends with the same grid of ours
    bool loggedOurs = false;
    bool loggedNoSim = false;
    bool loggedUnadjusted = false;  // the "unadjusted grid not compared" reason
};
KindState g_state[2];

// The caller's list for the original's station getter (an SC4List is its root
// pointer): a local root node, so no allocation of ours. The getter appends
// nodes of the game allocator with one reference each; both are given back
// here.
class StationList {
public:
    StationList() {
        head_.next = &head_;
        head_.prev = &head_;
        head_.value = nullptr;
        root_ = &head_;
    }
    ~StationList() {
        SC4ListNode<cISC4Occupant*>* n = head_.next;
        if (n == &head_) return;
        cIGZAllocatorServicePtr a;
        while (n != &head_) {
            SC4ListNode<cISC4Occupant*>* next = n->next;
            if (n->value != nullptr) FC4_RELEASE(n->value);
            if (a) a->Deallocate(n);
            n = next;
        }
    }
    StationList(const StationList&) = delete;
    StationList& operator=(const StationList&) = delete;

    SC4List<cISC4Occupant*>& AsList() { return *reinterpret_cast<SC4List<cISC4Occupant*>*>(&root_); }

    // The stations in list order (the references stay with this list).
    std::vector<cISC4Occupant*> Take() {
        std::vector<cISC4Occupant*> v;
        for (SC4ListNode<cISC4Occupant*>* n = head_.next; n != &head_; n = n->next) {
            if (n->value == nullptr) continue;
            FC4_ACQUIRED(n->value, "station list (one reference for the caller)");
            v.push_back(n->value);
        }
        return v;
    }

private:
    SC4ListNode<cISC4Occupant*> head_{};
    SC4ListNode<cISC4Occupant*>* root_ = nullptr;
};

// What one month's comparison reads from the original.
struct Original {
    cISC4SimGrid<uint8_t>* grid = nullptr;
    cISC4DepartmentBudget* department = nullptr;
    bool strike = false;
    float chance = 0.0f;
    int32_t totalDispatch = 0;
    float avg = 0.0f;
    float pct = 0.0f;
};

void ReadOriginal(uint32_t kind, cIGZUnknown* sim, StationList& list, Original& o) {
    SC4Percentage r{0.0f};
    if (kind == 0) {
        cFC4ISC4FireProtectionSimulator* f = AsGame(reinterpret_cast<cISC4FireProtectionSimulator*>(sim));
        o.grid = f->GetFireProtectionGrid();         // 5
        o.department = f->GetDepartmentBudget();     // 6
        o.strike = f->IsOnStrike();                  // 11
        f->GetChanceOfStrike(&r);                    // 12
        o.chance = r.percentage;
        o.avg = f->GetAverageCoverage();             // 19
        const SC4Percentage* p = f->GetPercentCoverage();  // 20
        o.pct = p ? p->percentage : 0.0f;
        o.totalDispatch = f->GetTotalDispatchCount();  // 25
        f->GetFireStations(list.AsList());             // 22
    } else {
        cFC4ISC4PoliceSimulator* p = AsGame(reinterpret_cast<cISC4PoliceSimulator*>(sim));
        o.grid = p->GetPolicePowerGrid();            // 5
        o.department = p->GetDepartmentBudget();     // 6
        o.strike = p->IsOnStrike();                  // 10
        p->GetChanceOfStrike(&r);                    // 11
        o.chance = r.percentage;
        o.avg = p->GetAverageCoverage();             // 18
        const SC4Percentage* pc = p->GetPercentCoverage();  // 19
        o.pct = pc ? pc->percentage : 0.0f;
        o.totalDispatch = p->GetTotalDispatchCount();  // 26
        p->GetPoliceStations(list.AsList());           // 21
    }
}

float OriginalEfficiency(uint32_t kind, cIGZUnknown* sim, cISC4Occupant* s) {
    SC4Percentage r{0.0f};
    if (kind == 0) {
        AsGame(reinterpret_cast<cISC4FireProtectionSimulator*>(sim))->GetStationEfficiency(&r, s);  // 10
    } else {
        AsGame(reinterpret_cast<cISC4PoliceSimulator*>(sim))->GetStationEfficiency(&r, s);  // 25
    }
    return r.percentage;
}

float OriginalRadius(uint32_t kind, cIGZUnknown* sim, cISC4Occupant* s) {
    if (kind == 0) return AsGame(reinterpret_cast<cISC4FireProtectionSimulator*>(sim))->GetStationDispatchRadius(s);
    return AsGame(reinterpret_cast<cISC4PoliceSimulator*>(sim))->GetStationDispatchRadius(s);  // 23
}

// PR5 (protection.md §2): the original's unadjusted int16 grid, borrowed
// from its primary +0x84 (no slot returns it). Read only from an object
// whose primary vtable is the original class's (the identified executable's,
// compat table; or the test seam's), so another plug-in's replacement is
// never read raw. nullptr, and `why` set, when it cannot be read. PR16
// confirms the recognition (only the class's own constructor and destructor
// write the primary vtable; +0x84 is null outside Init..Shutdown).
cISC4SimGrid<int16_t>* OriginalUnadjusted(uint32_t kind, cIGZUnknown* sim, const char** why) {
    const void* vt = g_testVtables ? g_testVtable[kind] : compat::ProtectionPrimaryVtable(kind);
    if (vt == nullptr) {
        *why = "the original class cannot be recognised: executable not identified";
        return nullptr;
    }
    const void* actual = nullptr;
    std::memcpy(&actual, sim, sizeof(actual));
    if (actual != vt) {
        *why = "the simulator's vtable is not the original class's";
        return nullptr;
    }
    cISC4SimGrid<int16_t>* g = nullptr;
    std::memcpy(&g, reinterpret_cast<const uint8_t*>(sim) + kUnadjustedOffset, sizeof(g));
    if (g == nullptr) *why = "the original has no unadjusted grid";
    return g;
}

uint32_t Bits(float f) {
    uint32_t b = 0;
    std::memcpy(&b, &f, sizeof(b));
    return b;
}

struct MonthCall {
    cISC4City* city;
    int32_t julian;
    uint32_t kind;
    char date[16];
};

void Line(const MonthCall* c, const char* fmt, ...) {
    char body[1600];
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(body, sizeof(body), _TRUNCATE, fmt, ap);
    va_end(ap);
    Log("protection shadow (%s) %s: %s", kTag[c->kind], c->date, body);
}

void MonthStep(void* p) {
    MonthCall* c = static_cast<MonthCall*>(p);
    const uint32_t kind = c->kind;
    KindState& k = g_state[kind];
    // City slot 57 GetFireProtectionSimulator returns city +0x174, the fire
    // class 0x2990C05A (protection.md §1, PR4 confirmed); slot 65 the police.
    cIGZUnknown* sim = kind == 0 ? static_cast<cIGZUnknown*>(c->city->GetFireProtectionSimulator())  // city slot 57
                                 : static_cast<cIGZUnknown*>(c->city->GetPoliceSimulator());       // city slot 65
    if (sim == nullptr) {
        if (!k.loggedNoSim) Line(c, "skipped (the city has no %s simulator)", kTag[kind]);
        k.loggedNoSim = true;
        return;
    }
    if (IsOurObject(sim)) {
        if (!k.loggedOurs) {
            Line(c, "skipped (the city's simulator is FreeCity4's: enable = protection; the shadow compares the "
                    "ORIGINAL simulators only)");
        }
        k.loggedOurs = true;
        return;
    }

    // 1. The original's inputs and outputs (getters only).
    StationList list;
    Original o;
    ReadOriginal(kind, sim, list, o);
    const std::vector<cISC4Occupant*> stations = list.Take();
    if (o.grid == nullptr) {
        Line(c, "skipped (the original has no coverage grid yet)");
        return;
    }
    HostGridContext gridCtx{"protection shadow", Sw::ProtectionGridWrites};
    const fc4_grid_api api = MakeGridApi(&gridCtx);
    const uint64_t gridToken = TokenOf(o.grid);
    fc4_grid_geometry geom{};
    api.geometry(api.user, gridToken, &geom);  // slots 10-17
    if (geom.count_x <= 0 || geom.count_z <= 0 || geom.count_x > 4096 || geom.count_z > 4096) {
        Line(c, "skipped (grid geometry %dx%d)", geom.count_x, geom.count_z);
        return;
    }

    // 2. Ours on the same inputs.
    ProtectionContext ctx;
    ctx.kind = kind;
    ctx.tag = kind == 0 ? "fire (shadow)" : "police (shadow)";
    ctx.readOnly = true;
    ctx.city = c->city;  // borrowed for this call
    const fc4_protection_host host = MakeProtectionReadOnlyHost(&ctx);
    fc4_protection* core = fc4_protection_shadow_create(&host, kind);
    if (core == nullptr) {
        Line(c, "skipped (shadow core creation failed)");
        return;
    }
    std::vector<uint64_t> tokens;
    tokens.reserve(stations.size());
    for (cISC4Occupant* s : stations) tokens.push_back(TokenOf(s));
    const bool ran = fc4_protection_shadow_run(core, &geom, tokens.data(), tokens.size(), TokenOf(o.department),
                                               o.strike);
    const size_t n = static_cast<size_t>(geom.count_x) * static_cast<size_t>(geom.count_z);
    std::vector<uint8_t> ours(n, 0);
    const size_t got = ran ? fc4_protection_shadow_grid(core, ours.data(), ours.size()) : 0;
    if (!ran || got != n) {
        fc4_protection_destroy(core);
        Line(c, "skipped (shadow run %d, %u of %u tracts)", ran ? 1 : 0, static_cast<unsigned>(got),
             static_cast<unsigned>(n));
        return;
    }

    // 3. Compare. The grid: x outer (index x * count_z + z), as the core copies it.
    uint32_t mismatches = 0;
    int maxDiff = 0;
    char first[400] = "-";
    size_t listed = 0;
    for (int32_t x = 0; x < geom.count_x; ++x) {
        for (int32_t z = 0; z < geom.count_z; ++z) {
            const int orig = static_cast<int>(api.get(api.user, gridToken, kClsU8, x, z));
            const int mine = ours[static_cast<size_t>(x) * static_cast<size_t>(geom.count_z) + z];
            if (orig == mine) continue;
            ++mismatches;
            const int d = orig > mine ? orig - mine : mine - orig;
            if (d > maxDiff) maxDiff = d;
            if (listed < kFirstCells) {
                char cell[48];
                _snprintf_s(cell, sizeof(cell), _TRUNCATE, "%s(%d,%d):%d/%d", listed == 0 ? "" : ",", x, z, orig,
                            mine);
                if (listed == 0) first[0] = 0;
                strncat_s(first, cell, _TRUNCATE);
                ++listed;
            }
        }
    }
    const bool same = k.lastX == geom.count_x && k.lastZ == geom.count_z && k.last == ours;
    k.stable = same ? k.stable + 1 : 0;
    k.last = std::move(ours);
    k.lastX = geom.count_x;
    k.lastZ = geom.count_z;
    const uint32_t dispatches = host.dispatch_count(host.user, kDispatchType[kind]);
    const bool expectExact = k.stable >= 2 && dispatches == 0;

    uint32_t effMismatch = 0, radMismatch = 0;
    char effFirst[160] = "";
    for (cISC4Occupant* s : stations) {
        const float eo = OriginalEfficiency(kind, sim, s);
        const float em = fc4_protection_station_efficiency(core, TokenOf(s));
        const float ro = OriginalRadius(kind, sim, s);
        const float rm = fc4_protection_station_dispatch_radius(core, TokenOf(s));
        if (Bits(eo) != Bits(em)) {
            if (effMismatch == 0) {
                _snprintf_s(effFirst, sizeof(effFirst), _TRUNCATE, " (first %p: %.6g/%.6g)", static_cast<void*>(s),
                            eo, em);
            }
            ++effMismatch;
        }
        if (Bits(ro) != Bits(rm)) ++radMismatch;
    }
    const float chanceOurs = fc4_protection_chance_of_strike(core);
    const int32_t totalOurs = fc4_protection_total_dispatch_count(core);

    // 4. The unadjusted grids (PR5), when the original's can be read.
    const char* why = nullptr;
    char unadjusted[600] = "";
    if (cISC4SimGrid<int16_t>* ug = OriginalUnadjusted(kind, sim, &why)) {
        const uint64_t ut = TokenOf(ug);
        fc4_grid_geometry ugeom{};
        api.geometry(api.user, ut, &ugeom);  // slots 10-17
        std::vector<int16_t> mineU(n, 0);
        const size_t gotU = fc4_protection_shadow_unadjusted_grid(core, mineU.data(), mineU.size());
        if (ugeom.count_x != geom.count_x || ugeom.count_z != geom.count_z || ugeom.tract_size != geom.tract_size) {
            why = "its geometry differs from the adjusted grid's";
        } else if (gotU != n) {
            why = "our unadjusted grid has another size";
        } else {
            uint32_t um = 0;
            int umax = 0;
            char ufirst[400] = "-";
            size_t ulisted = 0;
            for (int32_t x = 0; x < geom.count_x; ++x) {
                for (int32_t z = 0; z < geom.count_z; ++z) {
                    const int orig = static_cast<int>(api.get(api.user, ut, kClsI16, x, z));
                    const int mine = mineU[static_cast<size_t>(x) * static_cast<size_t>(geom.count_z) + z];
                    if (orig == mine) continue;
                    ++um;
                    const int d = orig > mine ? orig - mine : mine - orig;
                    if (d > umax) umax = d;
                    if (ulisted < kFirstCells) {
                        char cell[48];
                        _snprintf_s(cell, sizeof(cell), _TRUNCATE, "%s(%d,%d):%d/%d", ulisted == 0 ? "" : ",", x, z,
                                    orig, mine);
                        if (ulisted == 0) ufirst[0] = 0;
                        strncat_s(ufirst, cell, _TRUNCATE);
                        ++ulisted;
                    }
                }
            }
            _snprintf_s(unadjusted, sizeof(unadjusted), _TRUNCATE, "unadjusted cells=%u mismatches=%u max|diff|=%d first=%s",
                        static_cast<unsigned>(n), um, umax, ufirst);
        }
    }
    fc4_protection_destroy(core);

    Line(c,
         "cells=%u mismatches=%u max|diff|=%d stations=%u dispatches=%u stable=%u expectExact=%d first=%s; "
         "efficiency mismatches=%u/%u%s; radius mismatches=%u/%u; strike=%d; chance orig/ours=%.6g/%.6g%s; "
         "totalDispatch orig/ours=%d/%d; avg=%.4g pct=%.4g (original)",
         static_cast<unsigned>(n), mismatches, maxDiff, static_cast<unsigned>(stations.size()), dispatches, k.stable,
         expectExact ? 1 : 0, first, effMismatch, static_cast<unsigned>(stations.size()), effFirst, radMismatch,
         static_cast<unsigned>(stations.size()), o.strike ? 1 : 0, o.chance, chanceOurs,
         Bits(o.chance) == Bits(chanceOurs) ? "" : " MISMATCH", o.totalDispatch, totalOurs, o.avg, o.pct);
    if (expectExact && mismatches != 0) {
        Line(c, "WARNING %u cells differ although the grid should be exact (stable for %u month ends, no dispatch)",
             mismatches, k.stable);
    }
    if (unadjusted[0] != 0) {
        Line(c, "%s", unadjusted);
    } else if (why != nullptr && !k.loggedUnadjusted) {
        k.loggedUnadjusted = true;
        Line(c, "unadjusted grid not compared (%s)", why);
    }
}

}  // namespace

void ProtectionShadowStartup(bool protectionReplaced) {
    g_enabled = SettingInt("protection_shadow", "FC4_PROTECTION_SHADOW", 0) != 0;
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("protection_shadow", "FC4_PROTECTION_SHADOW", &src);
    if (protectionReplaced) {
        Log("protection shadow: requested (protection_shadow from %s) but INACTIVE: it compares the ORIGINAL fire and "
            "police simulators, and `enable = protection` replaces them",
            src);
        g_enabled = false;
        return;
    }
    Log("protection shadow: ON (protection_shadow from %s): at each SimNewMonth, per simulator, FreeCity4's coverage "
        "grid rebuilt from the original's stations, department and strike flag, compared cell by cell with the "
        "original's adjusted grid, and the unadjusted int16 grid with the original's (primary +0x84, read only from "
        "the original class); plus per-station efficiency and dispatch radius, chance of strike and total "
        "dispatch count",
        src);
    Log("protection shadow: NOT readable read-only (so not compared): the original's cycle accumulators "
        "(zoned/sum/nonZero) and iterator position (avg and pct are logged as the original's)");
}

void ProtectionShadowTestOriginalVtables(const void* fire, const void* police) {
    g_testVtables = fire != nullptr || police != nullptr;
    g_testVtable[0] = fire;
    g_testVtable[1] = police;
}

void ProtectionShadowMonth(cISC4City* city, int32_t julian) {
    if (!g_enabled || city == nullptr) return;
    for (uint32_t kind = 0; kind < 2; ++kind) {
        MonthCall c{city, julian, kind, {}};
        fc4_date d{};
        fc4_date_from_julian(julian, &d);
        _snprintf_s(c.date, sizeof(c.date), _TRUNCATE, "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
        if (!SehGuard(kind == 0 ? "protection shadow (fire)" : "protection shadow (police)", &MonthStep, &c)) {
            Log("protection shadow (%s) %s: WARNING structured exception; comparison abandoned", kTag[kind], c.date);
        }
    }
}

}  // namespace fc4
