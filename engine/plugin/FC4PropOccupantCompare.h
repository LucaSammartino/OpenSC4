// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "FC4PropOccupantFixtures.h"
#include <cstddef>

namespace fc4 { namespace prop_verify {
struct MethodCount {
    unsigned attempted = 0, dispatched = 0, compared = 0, refused = 0, mismatches = 0, checks = 0;
};
struct Counts {
    MethodCount methods[24]{};
    unsigned snapshots = 0, mutations = 0;
    // AL normalization applies only to the three published byte result slots.
    static uint32_t Normalize(unsigned slot, uint32_t value) {
        return slot == 10 || slot == 18 || slot == 22 ? value & 0xffu : value;
    }
    bool Compare(unsigned slot, uint32_t native, uint32_t rust, uint32_t expected) {
        auto& m = methods[slot];
        ++m.compared;
        m.checks += 2;
        native = Normalize(slot, native);
        rust = Normalize(slot, rust);
        bool exact = native == expected && rust == expected;
        if (!exact) ++m.mismatches;
        return exact;
    }
    template<class Endpoint> bool Enter(Endpoint& native, unsigned slot) {
        auto& m = methods[slot];
        ++m.attempted;
        if (!native.Authority(slot)) { ++m.refused; return false; }
        ++m.dispatched;
        return true;
    }
};
struct Lifetime {
    unsigned factory = 0, qi = 0, releaseQI = 0, releaseFactory = 0;
    unsigned rustCreated = 0, rustDestroyed = 0, foreignCleanup = 0;
    bool Balanced() const {
        return factory == releaseFactory && qi == releaseQI && rustCreated == rustDestroyed;
    }
};

// Production cleanup boundary, also exercised with fake owned references.
// Failure to prove native authority never dispatches the release callback.
template<class Endpoint, class Release>
bool ReleaseOwned(Counts& c, Lifetime& life, Endpoint& n, bool factory, Release release) {
    if (!c.Enter(n, 2)) return false;
    const uint32_t refs = release();
    if (factory) ++life.releaseFactory; else ++life.releaseQI;
    const uint32_t expected = factory ? 0u : 1u;
    return c.Compare(2, refs, expected, expected);
}

// Both endpoints own independent state. This runner never writes native results
// into Rust or the literal oracle, and stops at the first authority refusal.
template<class Native, class Rust, class Report>
bool Snapshot(Counts& c, Native& n, Rust& r, unsigned object, Scalars e, Report report) {
    const unsigned slots[] = {9, 12, 18, 19, 22};
    const uint32_t values[] = {uint32_t(e.orientation), e.state, e.chance, e.timer, e.usage};
    for (unsigned j = 0; j < 5; ++j) {
        const unsigned slot = slots[j];
        if (!c.Enter(n, slot)) return false;
        const uint32_t a = n.Read(slot, 0), b = r.Read(slot, 0);
        if (!c.Compare(slot, a, b, values[j])) report(object, slot, 0, a, b, values[j]);
    }
    for (uint8_t mask : timerMasks) {
        if (!c.Enter(n, 20)) return false;
        const uint32_t a = n.Read(20, mask), b = r.Read(20, mask);
        const uint32_t expected = (e.timer & mask) != 0 ? 1u : 0u;
        if (!c.Compare(20, a, b, expected)) report(object, 20, mask, a, b, expected);
    }
    ++c.snapshots;
    return true;
}

template<class Native, class Rust, class Report>
bool Exercise(Counts& c, Native (&n)[2], Rust (&r)[2], Report report) {
    Scalars expected[2] = {defaults, defaults};
    for (unsigned i = 0; i < 2; ++i)
        if (!Snapshot(c, n[i], r[i], i, expected[i], report)) return false;
    for (const Fixture& f : fixtures) {
        if (!c.Enter(n[f.object], f.slot)) return false;
        uint32_t a = n[f.object].Write(f), b = r[f.object].Write(f);
        if (f.slot == 10 && !c.Compare(10, a, b, 1)) report(f.object, 10, uint32_t(f.arg), a, b, 1);
        // Void setters have no return comparison: all logical postconditions
        // are measured by the following two full independent snapshots.
        expected[f.object] = f.expected;
        ++c.mutations;
        for (unsigned i = 0; i < 2; ++i)
            if (!Snapshot(c, n[i], r[i], i, expected[i], report)) return false;
    }
    return true;
}
} }
