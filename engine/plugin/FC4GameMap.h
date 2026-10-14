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

// The game's std::map of a uint32_t key to a 4-byte value: STLport's
// red-black tree, NOT a map compiled with a current MSVC (docs/abi/
// allocators.md, "map and set"). Header-only, no game calls of its own:
//
//   WalkGameMap            a read-only in-order walk (header.left, in-order
//                          successor, until the header) with bounds and
//                          sanity checks: the node count must equal the map's
//                          count, the depth is capped, every link it follows
//                          must be non-null and point back (child -> parent),
//                          keys must increase, and a step limit stops a cycle.
//   ReadGameMapVectorU16   a std::vector<std::map<uint32_t, uint16_t>>
//                          object {begin, end, capacity_end} of 12-byte
//                          maps, through WalkGameMap; all or nothing.
//   ReadProfileCapacities  the capacity maps of a building profile
//                          (docs/abi/layouts.md: such a vector at +0x0C).
//                          A lot's populations (lot slot 54 returns the
//                          vector at lot + 0x54, aura.md §6.5.1) are read
//                          with ReadGameMapVectorU16.
//   FillGameMapI32         empties and fills a map<uint32_t, int32_t> the game
//                          owns with the game's own subtree free and
//                          operator[] (docs/abi/direct-calls.md), whose
//                          addresses the caller passes (only on the
//                          identified executable; tests pass fakes).
//
// A walk over memory that is not a map at all can still fault on a wild
// pointer: callers in the plugin run these under fc4::SehGuard.
#pragma once

#include <cstddef>
#include <cstdint>
#include <cstring>

namespace fc4 {

// ---- layouts (allocators.md, "map and set") -----------------------------------------------------

// A node, and the header (same size; its value bytes are never used).
struct GameMapNode {
    uint8_t color;  // 0 = red, 1 = black; the header is always 0
    uint8_t pad[3];
    GameMapNode* parent;  // the root's parent is the header
    GameMapNode* left;
    GameMapNode* right;
    uint32_t key;  // +0x10, compared unsigned
    uint32_t value;  // +0x14: int32_t / pointer / float bits; a uint16_t uses the low half
};

// The 12-byte map object.
struct GameMapObject {
    GameMapNode* header;  // a separate B block, never null in a constructed map
    uint32_t count;
    uint32_t comparator;  // std::less, empty; never read or written
};

#if defined(_M_IX86) || (defined(__SIZEOF_POINTER__) && __SIZEOF_POINTER__ == 4)
static_assert(sizeof(GameMapNode) == 0x18, "map node: 0x18 bytes");
static_assert(offsetof(GameMapNode, parent) == 0x04 && offsetof(GameMapNode, left) == 0x08 &&
                  offsetof(GameMapNode, right) == 0x0C && offsetof(GameMapNode, key) == 0x10 &&
                  offsetof(GameMapNode, value) == 0x14,
              "map node layout (allocators.md)");
static_assert(sizeof(GameMapObject) == 12, "map object: 12 bytes");
#endif

constexpr uint8_t kGameMapRed = 0;
constexpr uint8_t kGameMapBlack = 1;

// A map<uint32_t, uint16_t> node's value: the low 16 bits at +0x14 (+0x16 is
// padding the game never writes, so it is not read).
inline uint16_t GameMapU16(const GameMapNode& n) {
    uint16_t v = 0;
    std::memcpy(&v, reinterpret_cast<const unsigned char*>(&n) + offsetof(GameMapNode, value), sizeof(v));
    return v;
}

// ---- the read-only walk ---------------------------------------------------------------------------

enum class GameMapStatus : uint8_t {
    Ok = 0,
    NullMap,        // no map object
    NullHeader,     // header pointer null
    BadHeader,      // header colour not 0
    BadEmpty,       // count 0, but the header does not describe an empty tree
    TooMany,        // count above the caller's limit
    NullLink,       // a link the walk must follow is null
    BadLink,        // a child whose parent is not the node it hangs from, a climb past the
                    // header, or header.left / header.right not the smallest / largest node
    BadColor,       // a node colour other than 0 or 1
    TooDeep,        // deeper than kGameMapMaxDepth
    StepLimit,      // more steps than a tree of `count` nodes needs (a cycle)
    CountMismatch,  // a different number of nodes than the count
    KeyOrder,       // keys not strictly increasing (a revisited node: a cycle)
    BadVector,      // a profile's vector of maps is not a vector of 12-byte maps
    BadInput,       // FillGameMapI32: keys not strictly increasing
    IndexFailed,    // FillGameMapI32: operator[] returned null
    BadResult,      // FillGameMapI32: the map afterwards is not exactly the entries written
};

inline const char* GameMapStatusName(GameMapStatus s) {
    switch (s) {
        case GameMapStatus::Ok: return "ok";
        case GameMapStatus::NullMap: return "no map object";
        case GameMapStatus::NullHeader: return "null header";
        case GameMapStatus::BadHeader: return "header colour not 0";
        case GameMapStatus::BadEmpty: return "count 0 but the tree is not empty";
        case GameMapStatus::TooMany: return "count above the limit";
        case GameMapStatus::NullLink: return "null link";
        case GameMapStatus::BadLink: return "inconsistent link";
        case GameMapStatus::BadColor: return "node colour not 0/1";
        case GameMapStatus::TooDeep: return "deeper than the depth cap";
        case GameMapStatus::StepLimit: return "step limit (cycle)";
        case GameMapStatus::CountMismatch: return "node count differs from the map's count";
        case GameMapStatus::KeyOrder: return "keys not increasing (cycle)";
        case GameMapStatus::BadVector: return "not a vector of 12-byte maps";
        case GameMapStatus::BadInput: return "entries not in increasing key order";
        case GameMapStatus::IndexFailed: return "operator[] returned null";
        case GameMapStatus::BadResult: return "the map afterwards differs from the entries written";
    }
    return "?";
}

// A red-black tree is at most 2·log2(n + 1) deep: 64 covers any count a
// 32-bit process can hold.
constexpr uint32_t kGameMapMaxDepth = 64;

// Walks `m` in key order: visit(const GameMapNode&) for each node, from
// header.left by in-order successor (allocators.md, Iteration) until the
// header. Never writes. Stops at the first inconsistency and returns why
// (nodes already visited stay visited: collect, and use them only on Ok).
// `maxNodes` bounds the count accepted. `visited` (optional) receives the
// number of nodes visited.
template <class Visit>
GameMapStatus WalkGameMap(const GameMapObject* m, uint32_t maxNodes, Visit&& visit, uint32_t* visited = nullptr) {
    uint32_t seen = 0;
    auto done = [&](GameMapStatus s) {
        if (visited != nullptr) *visited = seen;
        return s;
    };
    if (m == nullptr) return done(GameMapStatus::NullMap);
    const GameMapNode* const h = m->header;
    if (h == nullptr) return done(GameMapStatus::NullHeader);
    // DQ6 (demand.md §7.2): every map constructor, the copy constructor
    // included, writes header colour 0 and nothing changes it later; colour 1
    // never occurs in the game, so refusing it is our choice.
    if (h->color != kGameMapRed) return done(GameMapStatus::BadHeader);
    const uint32_t count = m->count;
    if (count == 0) {
        // The empty map: parent null, left = right = header (the constructors and the clear).
        // DQ4 (demand.md §5): count 0 with a non-empty tree is unreachable in the
        // game (every writer keeps the count equal to the nodes); the original
        // would reuse the stale tree, a corrupt map, so it is refused (our choice).
        if (h->parent != nullptr || h->left != h || h->right != h) return done(GameMapStatus::BadEmpty);
        return done(GameMapStatus::Ok);
    }
    if (count > maxNodes) return done(GameMapStatus::TooMany);
    const GameMapNode* const root = h->parent;
    if (root == nullptr || h->left == nullptr || h->right == nullptr) return done(GameMapStatus::NullLink);
    if (root == h || h->left == h || h->right == h || root->parent != h) return done(GameMapStatus::BadLink);

    // The depth of the first node: its parent chain up to the header.
    const GameMapNode* n = h->left;
    if (n->left != nullptr) return done(GameMapStatus::BadLink);  // not the smallest node
    int32_t depth = 0;
    for (const GameMapNode* p = n; p != h; p = p->parent) {
        if (p == nullptr) return done(GameMapStatus::NullLink);
        if (++depth > static_cast<int32_t>(kGameMapMaxDepth)) return done(GameMapStatus::TooDeep);
    }

    // A full in-order walk follows each edge once down and once up.
    const uint64_t stepLimit = 4ull * count + 4ull * kGameMapMaxDepth + 16;
    uint64_t steps = 0;
    uint32_t prevKey = 0;
    const GameMapNode* last = nullptr;
    auto step = [&](int32_t delta) -> GameMapStatus {
        depth += delta;
        if (++steps > stepLimit) return GameMapStatus::StepLimit;
        if (depth > static_cast<int32_t>(kGameMapMaxDepth)) return GameMapStatus::TooDeep;
        if (depth < 0) return GameMapStatus::BadLink;  // climbed past the header
        return GameMapStatus::Ok;
    };
    while (n != h) {
        if (n->color > kGameMapBlack) return done(GameMapStatus::BadColor);
        if (seen == count) return done(GameMapStatus::CountMismatch);  // more nodes than the count
        if (last != nullptr && !(prevKey < n->key)) return done(GameMapStatus::KeyOrder);
        visit(*n);
        ++seen;
        prevKey = n->key;
        last = n;
        // The successor (allocators.md, Iteration).
        if (n->right != nullptr) {
            const GameMapNode* c = n->right;
            if (c->parent != n) return done(GameMapStatus::BadLink);
            n = c;
            if (GameMapStatus s = step(+1); s != GameMapStatus::Ok) return done(s);
            while (n->left != nullptr) {
                c = n->left;
                if (c->parent != n) return done(GameMapStatus::BadLink);
                n = c;
                if (GameMapStatus s = step(+1); s != GameMapStatus::Ok) return done(s);
            }
        } else {
            const GameMapNode* y = n->parent;
            if (y == nullptr) return done(GameMapStatus::NullLink);
            while (n == y->right) {
                n = y;
                if (GameMapStatus s = step(-1); s != GameMapStatus::Ok) return done(s);
                y = y->parent;
                if (y == nullptr) return done(GameMapStatus::NullLink);
            }
            // Up to the parent, except at the root/header pair (the largest node's successor is the header).
            if (n->right != y) {
                // n is y's left child, or the root under the header.
                if (y->left != n && !(y == h && h->parent == n)) return done(GameMapStatus::BadLink);
                n = y;
                if (GameMapStatus s = step(-1); s != GameMapStatus::Ok) return done(s);
            }
        }
    }
    if (depth != 0) return done(GameMapStatus::BadLink);
    if (seen != count) return done(GameMapStatus::CountMismatch);
    if (last != h->right) return done(GameMapStatus::BadLink);  // header.right is not the largest node
    return done(GameMapStatus::Ok);
}

// ---- a building profile's capacity maps (layouts.md, "The capacity maps") ------------------------

constexpr size_t kProfileCapacityVectorOffset = 0x0C;  // begin, end, capacity_end of 12-byte maps
// DQ7 (demand.md §7.2): the original sets no limit; a profile holds at most 1
// map by construction and every shipped capacity property 1 entry, so only
// plug-in data could exceed these (refused above them: our choice).
constexpr uint32_t kMaxCapacityMaps = 16;              // the shipped game has 0 or 1
constexpr uint32_t kMaxCapacityEntries = 128;          // over every map; keys are developer types

struct GameCapacityEntry {
    uint32_t map;  // index in the profile's vector
    uint32_t devType;
    uint16_t amount;
};

struct GameCapacityRead {
    GameMapStatus status = GameMapStatus::Ok;
    uint32_t maps = 0;     // maps in the vector
    uint32_t badMap = 0;   // the map that failed (status != Ok, not BadVector)
    uint32_t count = 0;    // entries (0 unless status == Ok)
    GameCapacityEntry entries[kMaxCapacityEntries];
};

// Reads every (key, uint16 value) of every map of the STLport vector object
// at `vector` ({begin, end, capacity_end} of 12-byte map<uint32_t, uint16_t>),
// map by map, each in key order; entries whose value is 0 are kept. All or
// nothing: on any inconsistency `status` says why and `count` is 0. Never
// writes. A never-pushed vector (begin = end = capacity_end = null) has no
// maps.
inline void ReadGameMapVectorU16(const void* vector, GameCapacityRead* out) {
    out->status = GameMapStatus::Ok;
    out->maps = 0;
    out->badMap = 0;
    out->count = 0;
    if (vector == nullptr) {
        out->status = GameMapStatus::NullMap;
        return;
    }
    const GameMapObject* v[3] = {};
    std::memcpy(v, vector, sizeof(v));
    const uintptr_t begin = reinterpret_cast<uintptr_t>(v[0]);
    const uintptr_t end = reinterpret_cast<uintptr_t>(v[1]);
    const uintptr_t cap = reinterpret_cast<uintptr_t>(v[2]);
    if (begin == 0) {
        // Never pushed to (begin = end = null): a profile without property
        // 0x27812834, or a lot with no population maps.
        if (end != 0 || cap != 0) out->status = GameMapStatus::BadVector;
        return;
    }
    if (end < begin || cap < end || (end - begin) % sizeof(GameMapObject) != 0 ||
        (cap - begin) % sizeof(GameMapObject) != 0) {
        out->status = GameMapStatus::BadVector;
        return;
    }
    const size_t maps = (end - begin) / sizeof(GameMapObject);
    if (maps > kMaxCapacityMaps) {
        out->status = GameMapStatus::BadVector;
        return;
    }
    out->maps = static_cast<uint32_t>(maps);
    uint32_t n = 0;
    for (uint32_t i = 0; i < out->maps; ++i) {
        const GameMapStatus s = WalkGameMap(v[0] + i, kMaxCapacityEntries - n, [&](const GameMapNode& node) {
            if (n < kMaxCapacityEntries) out->entries[n++] = GameCapacityEntry{i, node.key, GameMapU16(node)};
        });
        if (s != GameMapStatus::Ok) {
            out->status = s;
            out->badMap = i;
            return;  // count stays 0
        }
    }
    out->count = n;
}

// Reads every (developer type, amount) of every capacity map of the 0x40-byte
// profile at `profile` (its vector at +0x0C), as ReadGameMapVectorU16. Never
// writes the profile.
inline void ReadProfileCapacities(const void* profile, GameCapacityRead* out) {
    if (profile == nullptr) {
        ReadGameMapVectorU16(nullptr, out);
        return;
    }
    ReadGameMapVectorU16(static_cast<const unsigned char*>(profile) + kProfileCapacityVectorOffset, out);
}

// ---- filling a map the game owns (direct-calls.md) ------------------------------------------------

// The game's functions, __thiscall (ECX = the map), called through __fastcall
// with EDX unused: operator[] pops 4 (a pointer to the key) and returns the
// value's address in EAX; the subtree free pops 4 (the subtree's top).
using GameMapIndexFn = void*(__fastcall*)(GameMapObject* self, void* edx, const uint32_t* key);
using GameMapEraseFn = void(__fastcall*)(GameMapObject* self, void* edx, GameMapNode* top);

struct GameMapFill {
    GameMapStatus status = GameMapStatus::Ok;
    uint32_t cleared = 0;     // entries removed by the clear (the count before)
    uint32_t written = 0;     // operator[] calls made
    uint32_t countAfter = 0;  // the map's count afterwards
};

// What GetLocalPopulationSummary does to the caller's map (allocators.md,
// GetLocalPopulationSummary):
//   0. checks the map with WalkGameMap (`maxNodes`); an inconsistent map, or
//      keys not strictly increasing, is left untouched;
//   1. when the count is not 0: erase(header.parent), then header.left :=
//      header, header.parent := null, header.right := header, count := 0
//      (the header stays; it is never freed or replaced, +0x08 never written);
//   2. map[keys[i]] := values[i] through operator[], in the given order;
//   3. walks the result again: it must hold exactly the entries written.
// `r` is updated as it goes, so it tells how far a call that faulted got.
inline void FillGameMapI32(GameMapObject* map, const uint32_t* keys, const int32_t* values, uint32_t n,
                           GameMapIndexFn index, GameMapEraseFn erase, uint32_t maxNodes, GameMapFill* r) {
    *r = GameMapFill{};
    for (uint32_t i = 1; i < n; ++i) {
        if (!(keys[i - 1] < keys[i])) {
            r->status = GameMapStatus::BadInput;
            return;
        }
    }
    r->status = WalkGameMap(map, maxNodes, [](const GameMapNode&) {});
    if (map != nullptr) r->countAfter = map->count;
    if (r->status != GameMapStatus::Ok) return;
    GameMapNode* const h = map->header;
    if (map->count != 0) {
        r->cleared = map->count;
        erase(map, nullptr, h->parent);
        h->left = h;
        h->parent = nullptr;
        h->right = h;
        map->count = 0;
        r->countAfter = 0;
    }
    for (uint32_t i = 0; i < n; ++i) {
        const uint32_t key = keys[i];
        void* slot = index(map, nullptr, &key);
        r->countAfter = map->count;
        if (slot == nullptr) {
            r->status = GameMapStatus::IndexFailed;
            return;
        }
        std::memcpy(slot, &values[i], sizeof(int32_t));
        ++r->written;
    }
    uint32_t at = 0;
    bool same = true;
    const GameMapStatus post = WalkGameMap(map, n, [&](const GameMapNode& node) {
        int32_t v = 0;
        std::memcpy(&v, &node.value, sizeof(v));
        if (at >= n || node.key != keys[at] || v != values[at]) same = false;
        ++at;
    });
    if (post != GameMapStatus::Ok) {
        r->status = post;
    } else if (!same || at != n) {
        r->status = GameMapStatus::BadResult;
    }
}

}  // namespace fc4
