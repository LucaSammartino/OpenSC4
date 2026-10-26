// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4ConfigRulesAcquisition.h"
#include <cassert>
#include <cstdio>
#include <string>
#include <vector>
using namespace fc4::cfgrules;
struct Fake {
    bool available = true, copyAvailable = true, buffer = true;
    unsigned begins = 0, clears = 0, flushes = 0, arrays = 0, draws = 0, copies = 0, ends = 0, pixels = 0;
    unsigned lit = 2;
    std::vector<unsigned> disabled;
    std::string order;
    bool Begin() { ++begins; order += 'B'; return available; }
    void ClearColor() { order += 'Z'; }
    void Clear() { ++clears; order += 'C'; }
    void Disable(unsigned c) { disabled.push_back(c); }
    void Arrays(const Vertex* v) {
        ++arrays;
        assert(v[0].x == 3.9375f && v[0].y == 3.9375f);
        assert(v[1].x == 3.9375f && v[1].y == 4.0625f);
        assert(v[2].x == 4.0625f && v[2].y == 4.0625f);
        assert(v[3].x == 4.0625f && v[3].y == 3.9375f);
        assert(v[252].x == 60.8125f && v[254].y == 60.9375f);
        for (unsigned i = 0; i < 256; ++i) assert(v[i].z == 0.f && v[i].color == UINT32_MAX);
        order += 'A';
    }
    void Draw() { ++draws; order += 'D'; }
    bool Copy() { ++copies; assert(draws == 20 && flushes == 19); order += 'R'; return copyAvailable; }
    void Flush() { ++flushes; order += 'F'; }
    void End() { ++ends; order += 'E'; }
    bool HasBuffer() { return buffer; }
    uint32_t Pixel(int x, int y) {
        assert(ends == 1 && clears == 21 && flushes == 21);
        assert(x == 4 + static_cast<int>(pixels % 8) * 8 && y == 4 + static_cast<int>(pixels / 8) * 8);
        return pixels++ < lit ? 0x00010000 : 0xff000000;
    }
};
struct Store {
    bool present, enabled;
    unsigned gets = 0;
    std::vector<bool> sets;
    bool Get() { ++gets; return enabled; }
    void Set(bool b) { sets.push_back(b); enabled = present && b; }
};
int main() {
    bool result = true;
    Fake software;
    assert(Probe(software, false, result) && !result && software.begins == 0);
    for (unsigned lit : {0u, 1u, 2u, 64u}) {
        Fake f; f.lit = lit;
        assert(Probe(f, true, result) && result == (lit > 1));
        assert(f.begins == 1 && f.arrays == 20 && f.draws == 20 && f.copies == 1 && f.pixels == 64);
        assert(f.disabled.size() == 100 && f.order.substr(f.order.size() - 5) == "RFCFE");
        for (unsigned i = 0; i < 100; ++i) assert(f.disabled[i] == std::vector<unsigned>({4,3,0,1,5})[i % 5]);
    }
    Fake noBuffer; noBuffer.buffer = false;
    assert(Probe(noBuffer, true, result) && !result && noBuffer.draws == 20 && noBuffer.pixels == 0);
    Fake noInterface; noInterface.copyAvailable = false;
    assert(!Probe(noInterface, true, result) && noInterface.ends == 1 && noInterface.clears == 21);
    Fake noService; noService.available = false;
    assert(!Probe(noService, true, result) && noService.draws == 0);
    for (bool present : {false, true}) for (bool enabled : {false, true}) {
        if (!present && enabled) continue;
        Store s{present, enabled};
        assert(BackingStore(s) == present && s.enabled == enabled && s.gets == 2);
        assert(s.sets == std::vector<bool>({true, enabled}));
    }
    puts("cfgrules acquisition: probe geometry/order, unavailable paths and backing-store restoration passed");
}
