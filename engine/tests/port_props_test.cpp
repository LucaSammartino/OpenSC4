// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4PortProps.h"

#include <cstdio>
#include <cstdlib>
#include <limits>
#include <string>
#include <vector>

static void Check(bool ok) {
    if (!ok) { std::fputs("PD12 boundary check failed\n", stderr); std::abort(); }
}

struct Alloc {
    static inline int blocks = 0;
    static bool Available() { return true; }
    static void* Allocate(uint32_t n) { ++blocks; return std::malloc(n); }
    static void Free(void* p) { if (p) { --blocks; std::free(p); } }
};

struct Occupant {
    char name;
    uint32_t type = 0xA823821E;
    float x = 40, z = 56;
    bool holder = true, instance = true, exemplar = false, bbox = true;
    bool output = true, queryResult = true, tag = false, member = true;
    int owner = 0, state = 0, occupantRefs = 0, propRefs = 0, transitions = 0;
    int typeReads = 0, positionReads = 0, propertyReads = 0, queries = 0;
    int retains = 0, releases = 0, unavailable = 0;
    uint32_t Type() { ++typeReads; return type; }
    void Position(float& px, float& pz) { ++positionReads; px = x; pz = z; }
    bool HasProperty(uint32_t id) {
        Check(id == 0xC98204B8);
        ++propertyReads;
        if (!holder) { ++unavailable; return false; } // explicitly safe-host fallback
        return instance || exemplar; // values deliberately never read
    }
};

struct Provider {
    using Occupant = ::Occupant;
    using Prop = ::Occupant;
    std::vector<Occupant*> candidates;
    std::vector<Occupant*> finished;
    std::string trace;
    bool available = true, enumerationResult = true, initialized = true;
    bool injected = false, gridDedup = false, filterReleased = false;
    int unavailable = 0, listReleases = 0;
    Occupant* removeOnSet = nullptr;
    uint32_t property = 0xC98204B8;
    bool Available() { return available; }
    void Unavailable() { ++unavailable; }
    bool Enumerate(fc4::GameList<Occupant*, Alloc>& list) {
        const fc4::PortPropBounds bounds{32, 48, 64, 80};
        if (!initialized) return false;
        for (auto* o : candidates) {
            // Emulate the provider's bbox test and grid dedup. The host must
            // neither replace these tests nor repeat them after enumeration.
            bool seen = false;
            for (auto* previous : finished) if (previous == o) seen = true;
            if (gridDedup && seen && !o->tag) continue;
            if (!injected && (!o->bbox || !fc4::PortPropMatches(*o, bounds, property))) continue;
            ++o->occupantRefs;
            Check(list.push_back(o));
            finished.push_back(o);
        }
        return enumerationResult;
    }
    void ReleaseFilter() { filterReleased = true; }
    bool QueryProp(Occupant* o, Prop*& prop) {
        Check(filterReleased && listReleases == 0 && o->occupantRefs > 0);
        ++o->queries;
        // QueryInterface's output is independent of its Boolean.
        Check(prop == nullptr);
        if (o->output) { ++o->propRefs; prop = o; }
        return o->queryResult;
    }
    void AddRefProp(Occupant* o) { Check(o->propRefs == 1); ++o->propRefs; ++o->retains; }
    void ReleaseProp(Occupant* o) { Check(o->propRefs > 0); --o->propRefs; ++o->releases; }
    void SetState(Occupant* o, uint32_t state) {
        Check(o->propRefs == 1 && o->occupantRefs > 0 && listReleases == 0);
        trace += o->name;
        if (o->state != static_cast<int>(state)) ++o->transitions;
        o->state = state;
        if (removeOnSet) removeOnSet->member = false;
    }
    void ReleaseList(fc4::GameList<Occupant*, Alloc>& list) {
        Check(filterReleased);
        ++listReleases;
        auto* root = list.head();
        auto* node = root->next;
        while (node != root) {
            auto* next = node->next;
            Check(node->value->occupantRefs > 0 && node->value->propRefs == 0);
            --node->value->occupantRefs;
            Alloc::Free(node);
            node = next;
        }
        root->next = root->prev = root;
    }
};

static void Run(Provider& p, uint32_t expected, const char* trace) {
    Check(fc4::SwitchPortAccessProps<Provider, Alloc>(p, 1) == expected);
    Check(p.trace == trace && Alloc::blocks == 0);
    for (auto* o : p.candidates) Check(o->occupantRefs == 0 && o->propRefs == 0);
    if (p.available) Check(p.listReleases == 1 && p.filterReleased);
}

int main() {
    { // PD12.1 initialized zero output
        Provider p; Run(p, 0, "");
    }
    { // PD12.2 multiple, provider order, repeat setter on state 1
        Occupant a{'A'}, b{'B'}, c{'C'}; a.state = 1;
        Provider p{{&b, &a, &c}}; Run(p, 3, "BAC");
        Check(a.transitions == 0 && b.transitions == 1 && c.transitions == 1);
        Check(a.retains == 1 && a.releases == 2);
    }
    { // PD12.3 instance zero, inherited false, absent, empty-array presence
        Occupant a{'A'}, b{'B'}, c{'C'}, d{'D'};
        b.instance = false; b.exemplar = true; c.instance = false;
        Provider p{{&a, &b, &c, &d}}; Run(p, 3, "ABD");
        Check(c.queries == 0 && c.propertyReads == 1);
    }
    { // PD12.4 type/spatial order, inclusive max, NaN fails
        Occupant a{'A'}, b{'B'}, c{'C'}, d{'D'}, e{'E'}, f{'F'};
        a.type = 0; b.x = 31; c.x = 64; d.x = 65; e.z = 80;
        f.x = std::numeric_limits<float>::quiet_NaN();
        Provider p{{&a, &b, &c, &d, &e, &f}}; Run(p, 2, "CE");
        Check(a.positionReads == 0 && b.propertyReads == 0 && f.propertyReads == 0);
    }
    { // PD12.5 ownership does not replace spatial tests
        Occupant a{'A'}, b{'B'}; a.owner = 99; b.x = 100;
        Provider p{{&a, &b}}; Run(p, 1, "A");
    }
    { // PD12.6 false/null, false/nonnull, true/nonnull; true/null too
        Occupant a{'A'}, b{'B'}, c{'C'};
        a.output = false; a.queryResult = false; b.queryResult = false;
        Provider p{{&a, &b, &c}}; Run(p, 2, "BC");
        Check(a.queries == 1 && a.retains == 0);
        a.queryResult = true;
        Provider second{{&a}}; Run(second, 0, "");
    }
    { // PD12.7 duplicates in a finished provider list are preserved
        Occupant a{'A'}, b{'B'};
        Provider p{{&a, &a, &b}}; Run(p, 3, "AAB");
        Check(a.retains == 2 && a.releases == 4 && a.transitions == 1);
    }
    { // PD12.8 native provider dedup/tag policy, no second host dedup
        Occupant a{'A'};
        Provider p{{&a, &a}}; p.gridDedup = true; Run(p, 1, "A");
        a.tag = true;
        Provider tagged{{&a, &a}}; tagged.gridDedup = true; Run(tagged, 2, "AA");
    }
    { // PD12.9 ignore enumeration result, process any finished output
        Provider empty; empty.initialized = false; Run(empty, 0, "");
        Occupant a{'A'}, b{'B'};
        Provider p{{&b, &a}}; p.injected = true; p.enumerationResult = false; Run(p, 2, "BA");
    }
    { // PD12.10 missing manager is labeled fallback, not native conformance
        Provider p; p.available = false; Run(p, 0, "");
        Check(p.unavailable == 1 && !p.filterReleased && p.listReleases == 0);
    }
    { // PD12.11 missing holder is labeled fallback; absent property is normal
        Occupant a{'A'}, b{'B'}; a.holder = false; b.instance = false;
        Provider p{{&a, &b}}; Run(p, 0, "");
        Check(a.unavailable == 1 && b.unavailable == 0 && a.queries == 0);
    }
    { // PD12.12 A's setter removes B, but finished list holds B until cleanup
        Occupant a{'A'}, b{'B'};
        Provider p{{&a, &b}}; p.removeOnSet = &b; Run(p, 2, "AB");
        Check(!b.member && b.positionReads == 1 && b.propertyReads == 1 && b.queries == 1);
    }
    { // Manager bbox rejection precedes filter; zero property bypasses holder
        Occupant a{'A'}, b{'B'}; a.bbox = false; b.holder = false;
        Provider p{{&a, &b}}; p.property = 0; Run(p, 1, "B");
        Check(a.typeReads == 0 && b.propertyReads == 0);
    }
    std::puts("PD12.1-PD12.12 PASS (native-grid policy emulated; missing providers labeled safe-host fallbacks)");
}
