// SPDX-License-Identifier: GPL-3.0-or-later
// lot-records 9.2/9.3, T79: known-live getters only; never Write stale effects.
#include "FC4ConstructionShadow.h"
#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cIGZSerializable.h"
#include "cISC4City.h"
#include "cISC4Occupant.h"
#include "cS3DVector3.h"
#include "fc4.h"
#include <cmath>
#include <cstring>
#include <limits>
#include <unordered_set>

extern "C" bool fc4_construction_shadow_position(const float*, float*);

namespace fc4 { namespace {
struct Sample {
    std::unordered_set<cISC4Occupant*> seen;
    uint32_t visited = 0, fields = 0, mismatches = 0, uninitialized = 0;
    cISC4Occupant* occupant = nullptr; // Manager holds it throughout callback (OM 9).
    cIGZSerializable* serializable = nullptr;

    static void Probe(void* data) {
        auto& s = *static_cast<Sample*>(data);
        // A retained post-Shutdown instance is outside LR14/T79 live scope.
        if (!s.occupant->IsInitialized()) { ++s.uninitialized; return; }
        ++s.visited;
        const float nan = std::numeric_limits<float>::quiet_NaN();
        cS3DVector3 lo(nan), hi(nan), position(nan);
        s.occupant->GetBoundingBox(&lo, &hi);
        s.occupant->GetPosition(&position); // Output contract only; do not infer AL.
        const float bounds[] = {lo.fX, lo.fY, lo.fZ, hi.fX, hi.fY, hi.fZ};
        const float native[] = {position.fX, position.fY, position.fZ};
        float expected[3] = {};
        const bool compared = fc4_construction_shadow_position(bounds, expected);
        for (unsigned i = 0; i < 3; ++i) {
            ++s.fields;
            if (!compared || !std::isfinite(bounds[i]) || !std::isfinite(bounds[i + 3]) ||
                !std::isfinite(native[i]) || std::memcmp(&native[i], &expected[i], sizeof(float)) != 0) {
                if (s.mismatches < 8) Log("construction-shadow: DIFF axis=%u min=%.9g max=%.9g native=%.9g core=%.9g",
                    i, bounds[i], bounds[i + 3], native[i], expected[i]);
                ++s.mismatches;
            }
        }
        ++s.fields;
        s.mismatches += s.occupant->GetType() != 0x096E763C;
        void* view = nullptr;
        ++s.fields;
        if (s.occupant->QueryInterface(0xE4FDA3D4, &view) && view) {
            s.serializable = static_cast<cIGZSerializable*>(view);
            FC4_ACQUIRED(s.serializable, "construction getter QI");
            s.mismatches += s.serializable->GetGZCLSID() != 0xA97F909E;
        } else {
            ++s.mismatches;
        }
    }
    static bool Visit(cISC4Occupant* occupant, void* data) {
        auto& s = *static_cast<Sample*>(data);
        if (!occupant) { ++s.mismatches; return true; }
        if (!s.seen.insert(occupant).second) return true;
        s.occupant = occupant;
        if (!SehGuard("construction live getters", &Probe, &s)) ++s.mismatches;
        if (s.serializable) { FC4_RELEASE(s.serializable); s.serializable = nullptr; }
        return true;
    }
};
}

void ConstructionShadowObserve(cISC4City* city, int32_t julian, bool loaded) {
    static const bool enabled = SettingInt("construction_shadow", "FC4_CONSTRUCTION_SHADOW", 0) != 0;
    if (!enabled) return;
    Sample s;
    auto* manager = city ? AsGame(city->GetOccupantManager()) : nullptr; // borrowed
    // OM 9.2/9.4: typed slot 45, null limits select the city; no guessed ABI.
    if (!manager || !manager->IterateOccupantsTyped(&Sample::Visit, &s, nullptr, nullptr, 0x096E763C))
        ++s.mismatches;
    if (loaded && city) {
        auto* sim = AsGame(city->GetSimulator());
        if (sim) julian = sim->GetSimDateNumber();
    }
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    Log("construction shadow %04d-%02d-%02d %s: occupants=%u fields=%u uninitialized=%u empty=%u scope=live-position,class,type partial=1 excluded=record,lifecycle,schedule,render; mismatches=%u",
        date.year, date.month, date.day_of_month, loaded ? "load" : "month",
        s.visited, s.fields, s.uninitialized, s.visited == 0 ? 1u : 0u, s.mismatches);
}
}
