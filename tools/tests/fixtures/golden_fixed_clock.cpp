#include "FC4HarnessLogic.h"
#include <cassert>
#include <cstdio>

int main() {
    fc4::harness::FixedStepClock c;
    c.BeginFrame();
    const auto first = c.Now();
    for (unsigned i = 0; i < 64; ++i) {
        assert(c.Now() == first + i);
        c.Delivered();
    }
    assert(c.Now() == first + 64);
    c.Delivered();
    assert(c.Now() == first + 64);
    c.BeginFrame();
    const auto empty = c.Now();
    for (unsigned i = 0; i < 4096; ++i) c.Now();
    assert(c.Now() == empty + 64);
    c.BeginFrame();
    const auto day = c.Now();
    c.FinishFrame();
    assert(c.Now() == day + 64);
    puts("fixed scheduler clock: 64-delivery cap, empty-frame bound, day barrier PASS");
}
