// SPDX-License-Identifier: GPL-3.0-or-later
// city-tools-input 8.8 / occupant-manager 13.2.1, 13.2.6.
#include "FC4QueryToolShadow.h"
#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "compat/FC4Compat.h"
#include "cISC4City.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantFilter.h"
#include "cISCPropertyHolder.h"
#include <unordered_set>

extern "C" uint32_t fc4_querytool_shadow_pick(uint32_t, uint32_t);

namespace fc4 { namespace {
// The native filters inherit the 12-byte base record unchanged (OM 13.2.6).
// Stack records are borrowed only for slot 3; never AddRef/Release or destroy
// them through the game's deleting destructor.
struct NativeFilter { const void* primary; const void* record; uint32_t refs; };
static_assert(sizeof(NativeFilter) == 12, "native x86 filter layout");
struct Sample {
    NativeFilter hover{}, describe{};
    std::unordered_set<cISC4Occupant*> seen;
    cISC4Occupant* occupant = nullptr;
    uint32_t occupants = 0, fields = 0, mismatches = 0, hoverHits = 0, queryHits = 0;
    static void Probe(void* data) {
        auto& s = *static_cast<Sample*>(data);
        auto* o = s.occupant; // held by manager throughout callback (OM 9)
        const uint32_t type = static_cast<uint32_t>(o->GetType());
        auto* holder = o->AsPropertyHolder(); // borrowed
        const uint32_t properties = holder ?
            (holder->HasProperty(0x8A416A99) ? 1u : 0u) |
            (holder->HasProperty(0xCAA9AB92) ? 2u : 0u) : 0u;
        const bool hover = reinterpret_cast<cISC4OccupantFilter*>(&s.hover)->IsOccupantIncluded(o);
        const bool query = reinterpret_cast<cISC4OccupantFilter*>(&s.describe)->IsOccupantIncluded(o);
        const uint32_t native = (hover ? 1u : 0u) | (query ? 2u : 0u);
        const uint32_t core = fc4_querytool_shadow_pick(type, properties);
        ++s.occupants;
        s.fields += 2;
        s.hoverHits += hover ? 1u : 0u;
        s.queryHits += query ? 1u : 0u;
        if (core != native) {
            if (s.mismatches < 8) Log("querytool-shadow: DIFF type=%08x properties=%u native=%u core=%u",
                type, properties, native, core);
            s.mismatches += ((core ^ native) & 1u) + (((core ^ native) >> 1) & 1u);
        }
    }
    static bool Visit(cISC4Occupant* occupant, void* data) {
        auto& s = *static_cast<Sample*>(data);
        if (!occupant || !s.seen.insert(occupant).second) return true;
        s.occupant = occupant;
        if (!SehGuard("querytool native pick predicates", &Probe, &s)) ++s.mismatches;
        return true;
    }
};
}
void QueryToolShadowObserve(cISC4City* city, int32_t julian, bool loaded) {
    if (SettingInt("querytool_shadow", nullptr, 0) == 0) return;
    Sample s;
    const void* hover = nullptr;
    const void* describe = nullptr;
    const void* record = nullptr;
    auto* manager = city ? AsGame(city->GetOccupantManager()) : nullptr;
    // CT27: renderer-hit replay and nonmodal displayed-field capture are not
    // specified. Measure only the production pick predicates, not full queries.
    if (!manager || !compat::QueryToolFilterVtables(&hover, &describe, &record)) {
        ++s.mismatches;
    } else {
        s.hover = {hover, record, 0};
        s.describe = {describe, record, 0};
        // OM 9.2 / 10: a null filter includes all occupants; no SDK concrete class
        // implementation or owned filter reference is needed for this walk.
        if (!manager->IterateOccupantsFilter(&Sample::Visit, &s, nullptr, nullptr, nullptr)) ++s.mismatches;
    }
    if (s.occupants == 0) ++s.mismatches; // no vacuous exact observation
    Log("querytool shadow julian=%d %s: occupants=%u fields=%u hover_hits=%u query_hits=%u "
        "scope=hover,describe-predicates partial=1 excluded=renderer-hit,selected-target,display,strings,highlight "
        "mismatches=%u", julian, loaded ? "load" : "month", s.occupants, s.fields,
        s.hoverHits, s.queryHits, s.mismatches);
}
}
