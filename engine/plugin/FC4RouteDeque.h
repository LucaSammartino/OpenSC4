// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

// The caller-owned route deque of cISC4PathFinder::SetRouteInfo (slot 8),
// filled in place after a recorded trip (pathfinder.md §4.13 step 5 and §7,
// "The route deque's layout"; TG1). The callers construct, walk and free it
// with their own inlined STLport code, so the fill writes the ten words
// exactly as the rules of §7 say: clear, then push_front per node from the
// result back to the root, regrowing the map at the front when needed. Node
// buffers (0x7E bytes, 42 three-byte RouteData) and the map come from
// allocator B (`Alloc`: Allocate(uint32_t) / Free(void*)).
#pragma once

#include <cstddef>
#include <cstdint>
#include <cstring>

namespace fc4 {

struct RouteDequeIter {
    uint8_t* cur;    // current element
    uint8_t* first;  // its node buffer
    uint8_t* last;   // first + 0x7E
    uint8_t** node;  // its map slot
};

struct RouteDeque {
    RouteDequeIter start;   // +0x00
    RouteDequeIter finish;  // +0x10 (cur = one past the last element)
    uint8_t** map;          // +0x20
    uint32_t mapSize;       // +0x24, in pointers
};

#if defined(_M_IX86) || defined(__i386__)
static_assert(sizeof(RouteDeque) == 0x28, "route deque: ten 32-bit words (pathfinder.md §7)");
#endif

constexpr uint32_t kRouteNodeBytes = 0x7E;  // 42 RouteData
constexpr uint32_t kRouteDataBytes = 3;

enum class RouteFillStatus { Ok, NoMap, OutOfMemory };

namespace route_deque_detail {

inline void SetNode(RouteDequeIter& it, uint8_t** node) {
    it.node = node;
    it.first = *node;
    it.last = it.first + kRouteNodeBytes;
}

// §7 step 3: the map regrowth for one node at the front.
template <class Alloc>
bool ReserveFront(RouteDeque& d) {
    const uint32_t used = static_cast<uint32_t>(d.finish.node - d.start.node) + 1;
    const uint32_t need = used + 1;
    uint8_t** newStart;
    if (d.mapSize > 2 * need) {
        newStart = d.map + (d.mapSize - need) / 2 + 1;
        std::memmove(newStart, d.start.node, used * sizeof(uint8_t*));
    } else {
        const uint32_t newSize = d.mapSize + (d.mapSize > 1 ? d.mapSize : 1) + 2;
        auto** newMap = static_cast<uint8_t**>(Alloc::Allocate(newSize * static_cast<uint32_t>(sizeof(uint8_t*))));
        if (newMap == nullptr) return false;
        newStart = newMap + (newSize - need) / 2 + 1;
        std::memcpy(newStart, d.start.node, used * sizeof(uint8_t*));
        Alloc::Free(d.map);
        d.map = newMap;
        d.mapSize = newSize;
    }
    SetNode(d.start, newStart);
    SetNode(d.finish, newStart + used - 1);
    return true;
}

}  // namespace route_deque_detail

// §7 step 1: clear. The start buffer and the start iterator are kept.
template <class Alloc>
void RouteDequeClear(RouteDeque& d) {
    for (uint8_t** n = d.start.node + 1; n < d.finish.node; ++n) Alloc::Free(*n);
    if (d.finish.node != d.start.node) Alloc::Free(d.finish.first);
    d.finish = d.start;
}

// §7 step 2: push_front of one RouteData.
template <class Alloc>
bool RouteDequePushFront(RouteDeque& d, const uint8_t* rd) {
    if (d.start.cur != d.start.first) {
        d.start.cur -= kRouteDataBytes;
        std::memcpy(d.start.cur, rd, kRouteDataBytes);
        return true;
    }
    if (d.start.node - d.map < 1 && !route_deque_detail::ReserveFront<Alloc>(d)) return false;
    auto* buf = static_cast<uint8_t*>(Alloc::Allocate(kRouteNodeBytes));
    if (buf == nullptr) return false;
    *(d.start.node - 1) = buf;
    route_deque_detail::SetNode(d.start, d.start.node - 1);
    d.start.cur = d.start.last - kRouteDataBytes;
    std::memcpy(d.start.cur, rd, kRouteDataBytes);
    return true;
}

// §4.13 step 5: clear, then push_front `count` RouteData from `route`
// (root -> result, 3 bytes each) starting at the result, so the deque runs
// root -> result.
template <class Alloc>
RouteFillStatus RouteDequeFill(RouteDeque& d, const uint8_t* route, size_t count) {
    if (d.map == nullptr || d.start.node == nullptr) return RouteFillStatus::NoMap;
    RouteDequeClear<Alloc>(d);
    for (size_t i = count; i-- > 0;) {
        if (!RouteDequePushFront<Alloc>(d, route + i * kRouteDataBytes)) return RouteFillStatus::OutOfMemory;
    }
    return RouteFillStatus::Ok;
}

}  // namespace fc4
