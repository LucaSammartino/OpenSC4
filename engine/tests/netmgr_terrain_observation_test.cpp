// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4NetMgrTerrainObservation.h"
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <cassert>

int main() {
    using namespace fc4::netmgr;
    const Removal oldB{7, 10, 6, 5};
    const Insertion newB{43, 7, 11, 1, 6, 5};
    assert(ObservedReplacement(42, oldB, {newB}));
    assert(!ObservedReplacement(42, oldB, {}));
    assert(!ObservedReplacement(43, oldB, {newB})); // same old identity
    auto other = newB;
    other.x = 5;
    assert(!ObservedReplacement(42, oldB, {other})); // adjacency proves nothing
    other = newB; other.call = 8;
    assert(!ObservedReplacement(42, oldB, {other})); // later unrelated tool
    other = newB; other.sequence = 9;
    assert(!ObservedReplacement(42, oldB, {other})); // insertion preceded removal
    other = newB; other.data = 0;
    assert(!ObservedReplacement(42, oldB, {other})); // not the §10 rebuild output
    assert(!ObservedReplacement(42, {0, 10, 6, 5}, {newB})); // outside slot 50
}
