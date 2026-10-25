// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>

// Private static-link transport. Owner is opaque and destroyed exactly once;
// destroy must not overlap any call. Getters/setters serialize state access.
struct FC4RegionalCityOwner;
struct fc4_istream;
struct fc4_ostream;
// Callbacks must remain callable for each retained reference's lifetime and
// allow serialized calls from the game's getter thread. They must not re-enter
// this owner. Each retain/release pair owns one interface reference.
struct FC4RegionalCityConnectionCalls {
    void (*retain)(void*);
    void (*release)(void*);
    uint32_t (*type)(void*);
    void (*cell)(void*, int32_t*, int32_t*);
    void (*repair)(void*, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);
};
struct FC4RegionalCityDealCalls {
    void (*retain)(void*);
    void (*release)(void*);
    void (*citySide)(void*, int32_t*, int32_t*);
    void (*otherSide)(void*, int32_t*, int32_t*);
    void (*setCitySide)(void*, int32_t, int32_t);
    void (*setOtherSide)(void*, int32_t, int32_t);
    uint32_t (*type)(void*);
    int32_t (*amount)(void*);
    uint32_t (*flags)(void*);
    bool (*setFlags)(void*, uint32_t, bool);
};
// read consumes the shared stream and transfers one owned interface reference;
// null means failure. write returns -1 for no serializable IID, 0 failure, 1 ok.
// The record's u16 helper version is handled by the caller, not this transport.
struct FC4RegionalCityRecordCalls {
    void* context;
    void* (*read)(void*, uint32_t kind, const fc4_istream*);
    int32_t (*write)(void*, uint32_t kind, void*, const fc4_ostream*);
    FC4RegionalCityConnectionCalls connections;
    FC4RegionalCityDealCalls deals;
};
struct FC4RegionalCityRegionCalls {
    void* context;
    uint64_t thisCity, currentCity;
    uint64_t (*cityAt)(void*, int32_t, int32_t);
};
struct FC4RegionalCityPopulation { uint32_t type; int32_t count; float jobs[4]; };
struct FC4RegionalCityPoint { uint32_t type; float position[3]; bool livable; };
struct FC4RegionalCityCache {
    uint8_t difficulty;
    uint32_t serial;
    bool established;
    int8_t mayorRating;
    float tax[12];
    int32_t tutorial;
    bool residential;
    int32_t population;
    float workforce;
    const FC4RegionalCityPopulation* populations;
    uint32_t populationCount;
    const FC4RegionalCityPoint* points;
    uint32_t pointCount;
};
// Scalar fields: 0/1 position, 2/3 size, 4 population, 5/6 jobs,
// 7 workforce, 8 rating, 9 difficulty, 10 serial, 11 birth, 12 established,
// 13/14/15 world, 16 hemisphere, 17 budget, 18 income, 19 tutorial ID,
// 20 tutorial flag, 21/22 original locale, 23/24 last locale,
// 25 connections count, 26 local deals count; 40 tax, 41 population(type),
// 42 extrapolated (set adds growth), 43 allowable extrapolation;
// 50 produced, 51 demanded, 52 exported, 53 imported, 54 cost, 55 balance.
// Only fields 10-15, 17-19, 42 and 50-54 accept scalar setters.
// Text fields: 30 city, 31 mayor, 32 advisor, 33 description, 34 path.
extern "C" {
FC4RegionalCityOwner* fc4_regionalcity_owner_create(uint8_t difficulty);
void fc4_regionalcity_owner_destroy(FC4RegionalCityOwner*);
bool fc4_regionalcity_owner_get(const FC4RegionalCityOwner*, uint32_t field, uint32_t arg, double* out);
bool fc4_regionalcity_owner_set(FC4RegionalCityOwner*, uint32_t field, uint32_t arg, double value);
bool fc4_regionalcity_owner_position(FC4RegionalCityOwner*, int32_t x, int32_t z, bool moveConnections);
bool fc4_regionalcity_owner_size(FC4RegionalCityOwner*, int32_t x, int32_t z);
// Returns required byte count; copies only if capacity suffices. No terminator.
// UINT32_MAX is failure. A null output is a length query.
uint32_t fc4_regionalcity_owner_text(const FC4RegionalCityOwner*, uint32_t field, uint8_t* out, uint32_t capacity);
bool fc4_regionalcity_owner_set_text(FC4RegionalCityOwner*, uint32_t field, const uint8_t* bytes, uint32_t len);
// Lists copy/acquire borrowed references atomically. Null elements reject the
// entire list. The deal list transport stores O/L; it does not run SetLocalDeals.
bool fc4_regionalcity_owner_connections(FC4RegionalCityOwner*, void* const* objects, uint32_t count,
    const FC4RegionalCityConnectionCalls*);
bool fc4_regionalcity_owner_deals(FC4RegionalCityOwner*, bool local, void* const* objects, uint32_t count,
    const FC4RegionalCityDealCalls*);
// Wire decode only: no localization, Init, or deal claiming. Failed reads leave
// the owner's state intact and release all successfully constructed elements.
bool fc4_regionalcity_owner_decode(FC4RegionalCityOwner*, const fc4_istream*, uint8_t difficulty,
    const FC4RegionalCityRecordCalls*);
bool fc4_regionalcity_owner_write(const FC4RegionalCityOwner*, const fc4_ostream*,
    const FC4RegionalCityRecordCalls*);
// kind 0: connections; 1: local deals. Emitted pointers are borrowed.
bool fc4_regionalcity_owner_list(const FC4RegionalCityOwner*, uint32_t kind, void*, void (*)(void*, void*));
void* fc4_regionalcity_owner_find(const FC4RegionalCityOwner*, uint32_t type, int32_t x, int32_t z);
// action 0: UpdateLocalDeals; 1: SetLocalDeals; 2: UpdateImportExport.
bool fc4_regionalcity_owner_deal_action(FC4RegionalCityOwner*, uint32_t action,
    const FC4RegionalCityRegionCalls*, void* const*, uint32_t, const FC4RegionalCityDealCalls*);
// event 0: return L to O; 1: reset growth; 2: release all three lists.
bool fc4_regionalcity_owner_event(FC4RegionalCityOwner*, uint32_t event);
bool fc4_regionalcity_owner_change_connection(FC4RegionalCityOwner*, void*, const FC4RegionalCityConnectionCalls*,
    int32_t sourceX, int32_t sourceZ, int32_t sourceSizeX, int32_t sourceSizeZ, bool add);
bool fc4_regionalcity_owner_cache(FC4RegionalCityOwner*, const FC4RegionalCityCache*);
bool fc4_regionalcity_owner_preferences(FC4RegionalCityOwner*, const uint8_t*, uint32_t,
    const uint8_t*, uint32_t, uint8_t difficulty);
bool fc4_regionalcity_owner_points(const FC4RegionalCityOwner*, uint32_t type, void*,
    void (*)(void*, const float (*)[3], uint32_t));
}
