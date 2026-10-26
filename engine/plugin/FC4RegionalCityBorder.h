// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>

namespace fc4::regionalcity {
// regional-city-records §4.6 RR3: these calls use distinct compiled slots.
template<class Map> bool TrafficPresent(Map* map, int32_t x, int32_t z) {
    return map && map->AnyNetworkAt(x, z) != 0;
}
template<class Tool> void PlaceBorderSegment(Tool* tool, int32_t x, int32_t z, int32_t ex, int32_t ez) {
    tool->ToolSlot07(x, z, ex, ez, 0, 0, 1, UINT32_MAX);
}
template<class Manager, class List, class Point>
void QueryBorderOccupants(Manager* manager, List& list, const Point& cell, uint32_t type) {
    if (manager) manager->GetOccupantsByStandardCityCellsTyped(list, &cell, &cell, type, UINT32_MAX);
}
// occupant-manager §13.2.4. The adapter supplies interface/property queries;
// the ordering here is also exercised independently of the game.
template<class Occupant> uint32_t ViewLevel(Occupant& o) {
    uint32_t level = 1;
    if (o.Interface(0x49C05B9F) || o.Interface(0xEA0A5ACF)) level = 3;
    if (o.Has(0x49512A09)) level = 3;
    if (o.Interface(0xCA16374F) || o.Interface(0xCA15ED6B)) level = 2;
    if (o.Group(0x4217) || o.Group(0x4208) || o.Group(0x421D)) level = 2;
    if (o.Has(0x8A15F1BD)) return 2;
    if (level == 1) {
        if (o.Bool(0xCAA45672)) level = 3;
        if (o.Bool(0xCAA45671)) level = 2;
        if (!o.Bool(0xCAA45670)) level = 1;
    }
    return level;
}
// regional-city §6.2.1: all eleven arguments, and ignore the result.
template<class Utility, class Region, class Filter>
void DemolishBorder(Utility* utility, const Region& region, Filter* filter) {
    if (utility) utility->DemolishRegion(true, region, 0, 0x1u, false, filter,
        nullptr, 0, nullptr, 0, 0);
}
}
