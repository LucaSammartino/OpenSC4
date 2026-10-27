// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4View3DObservation.h"
#include <cassert>
int main() {
    using namespace fc4::view3dshadow;
    Totals totals;
    Evidence e;
    totals.Add(e);
    assert(totals.calls == 1 && totals.complete == 0 && totals.mismatches == 0);
    e.bound = true;
    totals.Add(e);
    assert(totals.calls == 2 && totals.complete == 1 && totals.mismatches == 0);
    for (auto field : {&Evidence::state,&Evidence::result,&Evidence::out,&Evidence::calls}) {
        e.*field = false;
        assert(!e.Exact());
        totals.Add(e);
        e.*field = true;
    }
    e.bound = false; e.calls = false;
    totals.Add(e);
    assert(totals.calls == 7 && totals.complete == 5 && totals.mismatches == 5);
}
