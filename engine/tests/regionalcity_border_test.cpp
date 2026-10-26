// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4RegionalCityBorder.h"
#include "FC4RegionalCityState.h"
#include <array>
#include <cstdio>
#include <map>
#include <set>
#include <vector>

#define CHECK(x) do { if (!(x)) { std::fprintf(stderr, "line %d: %s\n", __LINE__, #x); return 1; } } while (0)
namespace {
struct Queries {
    std::set<uint32_t> interfaces, groups;
    std::map<uint32_t, bool> properties;
    bool Interface(uint32_t n) { return interfaces.count(n) != 0; }
    bool Group(uint32_t n) { return groups.count(n) != 0; }
    bool Has(uint32_t n) { return properties.count(n) != 0; }
    bool Bool(uint32_t n) { const auto p = properties.find(n); return p != properties.end() && p->second; }
};
struct Utility {
    bool exact = false;
    const void* expected;
    bool DemolishRegion(bool doIt, const int& region, int privilege, uint32_t flags,
        bool dezone, Queries* filter, int64_t* cost, intptr_t demolished, void* clicked, long x, long y) {
        exact = doIt && &region == expected && privilege == 0 && flags == 1 && !dezone && filter
            && !cost && !demolished && !clicked && !x && !y;
        return false; // a false demolition result is deliberately ignored
    }
};
struct Run {
    FC4RegionalCityOwner* owner;
    unsigned create = 0, remove = 0, segment = 0, posts = 0;
    uint32_t layer = 0, word = 0;
    bool reentered = false;
    std::vector<std::array<int32_t, 2>> cells;
};
void Ref(void*) {}
uint32_t Type(void*) { return 2; }
void Cell(void*, int32_t* x, int32_t* z) { *x = 64; *z = 10; }
void Repair(void*, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t) {}
const FC4RegionalCityConnectionCalls calls{Ref, Ref, Type, Cell, Repair};
bool Other(void*, void*) { return false; }
void Adjacent(void*, void*, void (*)(void*, int32_t, int32_t, int32_t, int32_t, void*)) {}
bool Network(void*, uint32_t n) { return n == 0; }
bool Find(void*, uint32_t, int32_t, int32_t) { return false; }
void Change(void* p, uint32_t t, int32_t x, int32_t z, bool add, bool dir) {
    auto& r = *static_cast<Run*>(p);
    if (t == 2 && x == -1 && z == 10 && !dir) { if (add) ++r.create; else ++r.remove; }
}
bool Present(void*, int32_t x, int32_t z, uint32_t kind) { return kind == 0 && x == 1 && z == 10; }
void Cells(void* p, uint32_t action, const int32_t (*cells)[2], uint32_t n, uint32_t layer) {
    auto& r = *static_cast<Run*>(p);
    if (action != 0) return;
    r.layer = layer;
    for (uint32_t i = 0; i < n; ++i) r.cells.push_back({cells[i][0], cells[i][1]});
    double population = 0;
    r.reentered = fc4_regionalcity_owner_get(r.owner, 4, 0, &population);
}
void Segment(void* p, uint32_t n, int32_t x, int32_t z, int32_t ex, int32_t ez) {
    if (n == 0 && x == 0 && z == 10 && ex == 2 && ez == 10) ++static_cast<Run*>(p)->segment;
}
void Occupants(void*, uint32_t n, int32_t x, int32_t z, void* sink, void (*emit)(void*, uint64_t)) {
    if (n == 0 && x == 0 && z == 10) emit(sink, 99);
}
uint32_t Word(void* p, uint64_t o) { return o == 99 ? static_cast<Run*>(p)->word : 0; }
void SetWord(void* p, uint64_t o, uint32_t w) { if (o == 99) static_cast<Run*>(p)->word = w; }
void Post(void* p, uint32_t m) { if (m == 0x6AC284F3) ++static_cast<Run*>(p)->posts; }
}
int main() {
    using fc4::regionalcity::ViewLevel;
    Queries q;
    CHECK(ViewLevel(q) == 1);
    q.properties[0xCAA45672] = true;
    CHECK(ViewLevel(q) == 1);
    q.properties[0xCAA45670] = true;
    CHECK(ViewLevel(q) == 3);
    q.properties[0xCAA45671] = true;
    CHECK(ViewLevel(q) == 2);
    q.interfaces.insert(0x49C05B9F);
    CHECK(ViewLevel(q) == 3); // prop flags cannot override a classified pipe
    q.interfaces.insert(0xCA16374F);
    CHECK(ViewLevel(q) == 2);
    q.interfaces.clear(); q.properties.clear();
    q.properties[0x49512A09] = false;
    CHECK(ViewLevel(q) == 3); // presence, regardless of property value
    q.groups.insert(0x4217);
    CHECK(ViewLevel(q) == 2);
    q.groups.clear(); q.properties[0x8A15F1BD] = false;
    CHECK(ViewLevel(q) == 2);
    int region = 7;
    Utility utility{false, &region};
    fc4::regionalcity::DemolishBorder(&utility, region, &q);
    CHECK(utility.exact);
    auto* owner = fc4_regionalcity_owner_create(2);
    CHECK(owner);
    Run run{owner};
    const FC4RegionalCityConnectionHost host{&run, calls, true, Other, Adjacent, Other, Network,
        Find, Change, Present, Cells, Segment, Occupants, Word, SetWord, Post};
    CHECK(fc4_regionalcity_owner_change_connection(owner, &run, &calls, -1, 0, 1, 1, true));
    CHECK(fc4_regionalcity_owner_reconcile(owner, true, false, &host));
    CHECK(run.create == 1 && run.remove == 0 && run.segment == 2 && run.posts == 1);
    CHECK(run.layer == 1 && run.reentered && run.word == 2);
    CHECK((run.cells == std::vector<std::array<int32_t, 2>>{{0, 10}, {2, 10}}));
    CHECK(fc4_regionalcity_owner_reconcile(owner, true, false, &host));
    CHECK(run.create == 1 && run.posts == 2); // pending queue drained, still posts
    fc4_regionalcity_owner_destroy(owner);
    return 0;
}
