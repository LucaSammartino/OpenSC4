// SPDX-License-Identifier: GPL-3.0-or-later
// Bounded read-only city geometry, not whole-owner or persistence verification.
#include "FC4CityShadow.h"
#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "cISC4City.h"
#include "fc4.h"

namespace {
struct Sample { float x, z; int32_t cell_x, cell_z; };
static_assert(sizeof(Sample) == 16, "Rust transport layout");
extern "C" uint32_t fc4_city_shadow_compare(const Sample*, uint32_t, float, float);
}

namespace fc4 {
void CityShadowObserve(cISC4City* city, int32_t julian, bool load) {
    if (SettingInt("city_shadow", nullptr, 0) == 0 || !city) return;
    // HG8: no scalar city record contract. Do not call Write (it also writes
    // other records), mutate the city, or seed expected values from its getters.
    // Only fixed caller-owned coordinate inputs enter the independent Rust core.
    Sample samples[] = {{0, 0, 0, 0}, {0.25f, 15.75f, 0, 0},
        {16, 16, 0, 0}, {31.75f, 32.25f, 0, 0}, {-0.25f, -0.25f, 0, 0},
        {-16, -16.25f, 0, 0}, {1024, 2048, 0, 0}, {4096, 4096, 0, 0}};
    for (auto& s : samples) city->PositionToCell(s.x, s.z, s.cell_x, s.cell_z);
    const uint32_t mismatches = fc4_city_shadow_compare(samples, 8, city->CellWidthX(), city->CellWidthZ());
    if (load) {
        if (auto* sim = AsGame(city->GetSimulator())) julian = sim->GetSimDateNumber();
        Log("city: HG8 bounded geometry only; compared PositionToCell and CellWidthX/Z; "
            "all other owner methods and serialized fields uncompared; no whole-owner credit");
    }
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    Log("city shadow %04d-%02d-%02d: phase=%s scope=geometry probes=8 fields=18 mismatches=%u",
        date.year, date.month, date.day_of_month, load ? "load" : "month", mismatches);
}
}
