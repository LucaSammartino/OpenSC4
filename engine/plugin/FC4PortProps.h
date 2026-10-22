// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include "FC4GameList.h"

namespace fc4 {

struct PortPropBounds {
    float minX, minZ, maxX, maxZ;
};

// PD12: type, reported position (inclusive, NaN rejected), then presence on
// the occupant's holder. Never fetch the property's value or owning lot.
template <class Candidate>
bool PortPropMatches(Candidate& candidate, const PortPropBounds& bounds, uint32_t property) {
    if (candidate.Type() != 0xA823821Eu) return false;
    float x = 0, z = 0;
    candidate.Position(x, z);
    if (!(x >= bounds.minX && x <= bounds.maxX && z >= bounds.minZ && z <= bounds.maxZ)) return false;
    return property == 0 || candidate.HasProperty(property);
}

// PD12: retain the finished occupant snapshot across every setter. The
// provider owns list references and releases them through manager slot 48;
// the GameList destructor then frees only the empty sentinel. Enumeration's
// Boolean is deliberately ignored, as is QueryInterface's Boolean inside
// QueryProp. The returned count is diagnostic, never an advisor-event gate.
template <class Provider, class Alloc = StlNodeAllocator>
uint32_t SwitchPortAccessProps(Provider& provider, uint32_t state) {
    if (!provider.Available()) {
        provider.Unavailable();
        return 0;
    }
    GameList<typename Provider::Occupant*, Alloc> list;
    if (!list.valid()) {
        provider.Unavailable();
        return 0;
    }
    provider.Enumerate(list);
    provider.ReleaseFilter();
    uint32_t count = 0;
    for (auto* occupant : list) {
        typename Provider::Prop* prop = nullptr;
        provider.QueryProp(occupant, prop);  // output nullness, not the Boolean
        if (prop == nullptr) continue;
        provider.AddRefProp(prop);
        provider.ReleaseProp(prop);  // QueryInterface's reference
        provider.SetState(prop, state);
        provider.ReleaseProp(prop);  // retained across SetState
        ++count;
    }
    provider.ReleaseList(list);
    return count;
}

}  // namespace fc4
