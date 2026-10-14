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

// The game's std::set of a 4-byte key (a pointer or a uint32_t): STLport's
// red-black tree of docs/abi/allocators.md "map and set", with 0x14-byte
// nodes and the key at +0x10. The one the plugin writes is PlaceZone's
// outLots, a std::set<cRZAutoRefCount<cISC4Lot>> the caller owns (zones.md
// §6, §13.2): the zone manager inserts each lot to be demolished, uniquely,
// and AddRefs only a new insertion. Header-only, no game calls:
//
//   GameSetInsertU32   a unique insert exactly as allocators.md "Insertion"
//                      describes the game's (find the position, link, the
//                      red-black fix-up, count + 1), with a node from the
//                      caller's allocator policy (the game's B in the plugin).
//
// The set is checked first with WalkGameMap (FC4GameMap.h: the walk reads
// the colour, the links and the key only, which a 0x14-byte node has at the
// same offsets as a map node); an inconsistent set is left untouched.
#pragma once

#include "FC4GameMap.h"

#include <cstddef>
#include <cstdint>

namespace fc4 {

// A set node (0x14 bytes) and the 12-byte set object (allocators.md).
struct GameSetNode {
    uint8_t color;  // 0 = red, 1 = black; the header is always 0
    uint8_t pad[3];
    GameSetNode* parent;  // the root's parent is the header
    GameSetNode* left;
    GameSetNode* right;
    uint32_t key;  // +0x10, compared unsigned
};

struct GameSetObject {
    GameSetNode* header;  // a separate B block, never null in a constructed set
    uint32_t count;
    uint32_t comparator;  // std::less, empty; never read or written
};

#if defined(_M_IX86) || (defined(__SIZEOF_POINTER__) && __SIZEOF_POINTER__ == 4)
static_assert(sizeof(GameSetNode) == 0x14, "set node: 0x14 bytes");
static_assert(offsetof(GameSetNode, parent) == offsetof(GameMapNode, parent) &&
                  offsetof(GameSetNode, left) == offsetof(GameMapNode, left) &&
                  offsetof(GameSetNode, right) == offsetof(GameMapNode, right) &&
                  offsetof(GameSetNode, key) == offsetof(GameMapNode, key),
              "set node links and key where a map node has them");
static_assert(sizeof(GameSetObject) == 12, "set object: 12 bytes");
#endif

enum class GameSetResult : uint8_t {
    Inserted = 0,  // a new node
    Present,       // the key was there; nothing changed
    Refused,       // the set is inconsistent (see `status`) or out of memory; nothing changed
};

namespace detail {

inline void SetRotateLeft(GameSetNode* x, GameSetNode*& root) {
    GameSetNode* y = x->right;
    x->right = y->left;
    if (y->left != nullptr) y->left->parent = x;
    y->parent = x->parent;
    if (x == root) {
        root = y;
    } else if (x == x->parent->left) {
        x->parent->left = y;
    } else {
        x->parent->right = y;
    }
    y->left = x;
    x->parent = y;
}

inline void SetRotateRight(GameSetNode* x, GameSetNode*& root) {
    GameSetNode* y = x->left;
    x->left = y->right;
    if (y->right != nullptr) y->right->parent = x;
    y->parent = x->parent;
    if (x == root) {
        root = y;
    } else if (x == x->parent->right) {
        x->parent->right = y;
    } else {
        x->parent->left = y;
    }
    y->right = x;
    x->parent = y;
}

// allocators.md "Insertion" step 3 (0x0040D5E0): the red-black fix-up.
inline void SetRebalance(GameSetNode* x, GameSetNode*& root) {
    x->color = kGameMapRed;
    while (x != root && x->parent->color == kGameMapRed) {
        GameSetNode* const p = x->parent;
        GameSetNode* const g = p->parent;
        if (p == g->left) {
            GameSetNode* const uncle = g->right;
            if (uncle != nullptr && uncle->color == kGameMapRed) {
                p->color = kGameMapBlack;
                uncle->color = kGameMapBlack;
                g->color = kGameMapRed;
                x = g;
            } else {
                if (x == p->right) {
                    x = p;
                    SetRotateLeft(x, root);
                }
                x->parent->color = kGameMapBlack;
                x->parent->parent->color = kGameMapRed;
                SetRotateRight(x->parent->parent, root);
            }
        } else {
            GameSetNode* const uncle = g->left;
            if (uncle != nullptr && uncle->color == kGameMapRed) {
                p->color = kGameMapBlack;
                uncle->color = kGameMapBlack;
                g->color = kGameMapRed;
                x = g;
            } else {
                if (x == p->left) {
                    x = p;
                    SetRotateRight(x, root);
                }
                x->parent->color = kGameMapBlack;
                x->parent->parent->color = kGameMapRed;
                SetRotateLeft(x->parent->parent, root);
            }
        }
    }
    root->color = kGameMapBlack;
}

// allocators.md "Iteration": the predecessor of a node that is not the header.
inline GameSetNode* SetPredecessor(GameSetNode* n) {
    if (n->left != nullptr) {
        n = n->left;
        while (n->right != nullptr) n = n->right;
        return n;
    }
    GameSetNode* p = n->parent;
    while (n == p->left) {
        n = p;
        p = p->parent;
    }
    return p;
}

}  // namespace detail

// A unique insert of `key` into the game's set `s` (allocators.md
// "Insertion"). `maxNodes` bounds the count the consistency walk accepts.
// `status` (optional) receives the walk's verdict. A node comes from
// Alloc::Allocate(0x14); nothing is allocated unless the key is new.
template <class Alloc>
GameSetResult GameSetInsertU32(GameSetObject* s, uint32_t key, uint32_t maxNodes, GameMapStatus* status = nullptr) {
    const GameMapStatus walk =
        WalkGameMap(reinterpret_cast<const GameMapObject*>(s), maxNodes, [](const GameMapNode&) {});
    if (status != nullptr) *status = walk;
    if (walk != GameMapStatus::Ok) return GameSetResult::Refused;
    GameSetNode* const h = s->header;
    // 1. Find the position.
    GameSetNode* y = h;
    GameSetNode* x = h->parent;
    bool left = true;
    while (x != nullptr) {
        y = x;
        left = key < x->key;
        x = left ? x->left : x->right;
    }
    GameSetNode* j = y;
    if (left) {
        if (j != h->left && j != h) {
            j = detail::SetPredecessor(j);
            if (!(j->key < key)) return GameSetResult::Present;
        }
    } else if (!(j->key < key)) {
        return GameSetResult::Present;
    }
    // 2. Link.
    GameSetNode* const z = static_cast<GameSetNode*>(Alloc::Allocate(static_cast<uint32_t>(sizeof(GameSetNode))));
    if (z == nullptr) return GameSetResult::Refused;
    z->color = kGameMapRed;
    z->pad[0] = z->pad[1] = z->pad[2] = 0;
    z->key = key;
    if (y == h || key < y->key) {
        y->left = z;
        if (y == h) {
            h->parent = z;
            h->right = z;
        } else if (y == h->left) {
            h->left = z;
        }
    } else {
        y->right = z;
        if (y == h->right) h->right = z;
    }
    z->parent = y;
    z->left = nullptr;
    z->right = nullptr;
    // 3. Rebalance (the root is header.parent).
    detail::SetRebalance(z, h->parent);
    // 4. Count.
    ++s->count;
    return GameSetResult::Inserted;
}

}  // namespace fc4
