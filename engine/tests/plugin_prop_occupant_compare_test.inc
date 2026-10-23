// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4PropOccupantCompare.h"

// Test doubles exercise the production comparator and refusal boundary, not a
// second owner. They play the frozen literal transcript with intentional local
// faults. Neither fixtures nor production owner expectations are changed.
namespace prop_compare_test {
using namespace fc4::prop_verify;
struct Double {
    Scalars s = defaults;
    unsigned calls = 0, deny = 24, fault = 0;
    Double* shared = nullptr;
    bool Authority(unsigned slot) { return slot != deny; }
    uint32_t Read(unsigned slot, uint8_t mask) {
        ++calls;
        const auto e = shared ? shared->s : s;
        switch (slot) {
            case 9: return uint32_t(e.orientation);
            case 12: return e.state | (fault == 2 ? 0x100u : 0u);
            case 18: return fault == 3 ? uint32_t(e.chance != 0) : e.chance;
            case 19: return fault == 4 ? e.timer & 0xf : e.timer;
            case 20: return (e.timer & mask) != 0 ? 1u : 0u;
            case 22: return fault == 5 ? uint32_t(e.usage != 0) : e.usage;
        }
        return 0;
    }
    uint32_t Write(const Fixture& f) {
        ++calls;
        s = f.expected;
        if (fault == 1 && f.slot == 10 && f.arg == 4) s.orientation = 4;
        if (fault == 6 && f.slot == 17) s.state = 1; // adjacent-field contamination
        if (fault == 7 && f.slot == 21 && !f.on) s.timer |= uint32_t(f.arg); // broken clear
        return 1;
    }
};
unsigned Mismatches(const Counts& c) {
    unsigned result = 0;
    for (const auto& m : c.methods) result += m.mismatches;
    return result;
}
void Check() {
    auto discard = [](unsigned, unsigned, uint32_t, uint32_t, uint32_t, uint32_t) {};
    Counts exact;
    Double n[2], r[2];
    CHECK(Exercise(exact, n, r, discard));
    CHECK(exact.snapshots == 92 && exact.mutations == 45 && Mismatches(exact) == 0);
    unsigned dispatched = 0, compared = 0;
    for (const auto& m : exact.methods) { dispatched += m.dispatched; compared += m.compared; }
    CHECK(dispatched == 1057 && compared == 1022);
    CHECK(exact.methods[20].dispatched == 552 && exact.methods[10].compared == 10);
    CHECK(exact.methods[11].compared == 0 && exact.methods[21].compared == 0); // void
    for (unsigned fault = 1; fault <= 7; ++fault) {
        Counts c;
        Double bad[2], core[2];
        bad[0].fault = fault;
        CHECK(Exercise(c, bad, core, discard));
        CHECK(Mismatches(c) > 0);
    }
    {
        Counts c;
        Double bad[2], core[2];
        bad[1].shared = &bad[0];
        CHECK(Exercise(c, bad, core, discard));
        CHECK(Mismatches(c) > 0); // per-object independence
    }
    {
        Counts c;
        Double denied[2], core[2];
        denied[0].deny = 19;
        CHECK(!Exercise(c, denied, core, discard));
        CHECK(c.methods[19].attempted == 1 && c.methods[19].refused == 1);
        CHECK(c.methods[19].dispatched == 0 && c.methods[19].compared == 0);
        CHECK(denied[0].calls == 3 && core[0].calls == 3 && denied[1].calls == 0);
        CHECK(c.mutations == 0 && c.snapshots == 0 && Mismatches(c) == 0);
    }
    Counts channels;
    CHECK(channels.Compare(18, 0x123400ff, 255, 255)); // AL upper bits undefined
    CHECK(channels.Compare(22, 0xabcd0004, 4, 4));
    CHECK(!channels.Compare(18, 255, 1, 255)); // byte != boolean
    CHECK(!channels.Compare(12, 0x100, 0, 0)); // clean full EAX required
    CHECK(!channels.Compare(19, 0x100, 0, 0));
    CHECK(!channels.Compare(20, 0x101, 1, 1));
    Lifetime life;
    CHECK(life.Balanced());
    life.factory = 2; life.qi = 2; life.rustCreated = 2;
    CHECK(!life.Balanced());
    life.releaseQI = 2; life.releaseFactory = 2; life.rustDestroyed = 2;
    CHECK(life.Balanced());
    ++life.releaseQI;
    CHECK(!life.Balanced()); // over-release also fails
    {
        Counts c;
        Lifetime owned;
        Double endpoint;
        unsigned refs = 2, releases = 0;
        owned.factory = 1; owned.qi = 1;
        auto release = [&]() { ++releases; return --refs; };
        CHECK(ReleaseOwned(c, owned, endpoint, false, release));
        CHECK(!owned.Balanced() && refs == 1 && releases == 1);
        endpoint.deny = 2;
        CHECK(!ReleaseOwned(c, owned, endpoint, true, release));
        CHECK(refs == 1 && releases == 1 && owned.releaseFactory == 0);
        endpoint.deny = 24;
        CHECK(ReleaseOwned(c, owned, endpoint, true, release));
        CHECK(owned.Balanced() && refs == 0 && releases == 2);
        CHECK(!ReleaseOwned(c, owned, endpoint, true, release));
        CHECK(!owned.Balanced() && c.methods[2].mismatches == 1);
    }
    // The entire frozen timer sequence is safe independently of either owner.
    for (const auto& f : fixtures) CHECK((f.expected.timer & 0xf) != 0xf);
}
}
