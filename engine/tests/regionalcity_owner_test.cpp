// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4RegionalCityState.h"
#include <cstdio>
#include <cstring>

#define CHECK(expr) do { if (!(expr)) { std::fprintf(stderr, "line %d: %s\n", __LINE__, #expr); return 1; } } while (0)

namespace {
struct Object { int refs = 0, adds = 0, drops = 0, x = 0, z = 0, repairs = 0; uint32_t flags = 0; };
void Retain(void* p) { auto& o = *static_cast<Object*>(p); ++o.refs; ++o.adds; }
void Release(void* p) { auto& o = *static_cast<Object*>(p); --o.refs; ++o.drops; }
uint32_t Type(void*) { return 2; }
void Pair(void* p, int32_t* x, int32_t* z) { auto& o = *static_cast<Object*>(p); *x = o.x; *z = o.z; }
void SetPair(void* p, int32_t x, int32_t z) { auto& o = *static_cast<Object*>(p); o.x = x; o.z = z; }
void Repair(void* p, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t) { ++static_cast<Object*>(p)->repairs; }
int32_t Amount(void*) { return 25; }
uint32_t Flags(void* p) { return static_cast<Object*>(p)->flags; }
bool SetFlags(void* p, uint32_t mask, bool on) {
    auto& flags = static_cast<Object*>(p)->flags;
    flags = on ? flags | mask : flags & ~mask;
    return true;
}
const FC4RegionalCityConnectionCalls connections = {Retain, Release, Type, Pair, SetPair, Repair};
const FC4RegionalCityDealCalls deals = {Retain, Release, Pair, Pair, SetPair, SetPair, Type, Amount, Flags, SetFlags};
}

int main() {
    auto* city = fc4_regionalcity_owner_create(2);
    CHECK(city);
    double value = -1;
    CHECK(fc4_regionalcity_owner_get(city, 9, 0, &value) && value == 2);
    CHECK(fc4_regionalcity_owner_set(city, 10, 0, 4294967295.0));
    CHECK(fc4_regionalcity_owner_get(city, 10, 0, &value) && value == 4294967295.0);
    CHECK(fc4_regionalcity_owner_set(city, 19, 0, -1));
    CHECK(fc4_regionalcity_owner_get(city, 19, 0, &value) && value == -1);
    CHECK(fc4_regionalcity_owner_set(city, 50, 99, -5));
    CHECK(fc4_regionalcity_owner_get(city, 50, 99, &value) && value == 0);
    CHECK(fc4_regionalcity_owner_size(city, 2, 4));
    CHECK(fc4_regionalcity_owner_get(city, 3, 0, &value) && value == 4);
    const uint8_t name[] = {'A', 0, 'B'};
    CHECK(fc4_regionalcity_owner_set_text(city, 30, name, sizeof(name)));
    CHECK(fc4_regionalcity_owner_text(city, 30, nullptr, 0) == sizeof(name));
    uint8_t out[4] = {99, 99, 99, 99};
    CHECK(fc4_regionalcity_owner_text(city, 30, out, 2) == sizeof(name) && out[0] == 99);
    CHECK(fc4_regionalcity_owner_text(city, 30, out, 4) == sizeof(name));
    CHECK(std::memcmp(out, name, sizeof(name)) == 0 && out[3] == 99);
    Object connection, deal;
    void* cp = &connection;
    void* dp = &deal;
    CHECK(fc4_regionalcity_owner_connections(city, &cp, 1, &connections));
    CHECK(fc4_regionalcity_owner_deals(city, false, &dp, 1, &deals));
    CHECK(fc4_regionalcity_owner_deals(city, true, &dp, 1, &deals));
    CHECK(connection.refs == 1 && deal.refs == 2);
    CHECK(fc4_regionalcity_owner_get(city, 25, 0, &value) && value == 1);
    void* invalid[] = {cp, nullptr};
    CHECK(!fc4_regionalcity_owner_connections(city, invalid, 2, &connections));
    CHECK(connection.refs == 1);
    CHECK(fc4_regionalcity_owner_position(city, -2, 5, true));
    CHECK(connection.repairs == 1);
    CHECK(fc4_regionalcity_owner_connections(city, nullptr, 0, &connections));
    CHECK(connection.refs == 0);
    fc4_regionalcity_owner_destroy(city);
    CHECK(deal.refs == 0);
    CHECK(connection.adds == connection.drops && deal.adds == deal.drops);
    fc4_regionalcity_owner_destroy(nullptr);
    return 0;
}
