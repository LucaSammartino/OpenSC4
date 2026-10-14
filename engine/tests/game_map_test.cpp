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

// The game's STLport map (plugin/FC4GameMap.h) against fake game maps built
// as docs/abi/allocators.md ("map and set") describes them:
//
//   - a fake B allocator that counts blocks and never really frees (a double
//     or foreign free is recorded, not a heap corruption);
//   - a fake operator[] (allocators.md, Insertion: find the position, link,
//     red-black rebalance, count + 1) and a fake subtree free, with the
//     __thiscall-through-__fastcall signatures of direct-calls.md;
//   - the GetLocalPopulationSummary worked example (allocators.md);
//   - the capacity maps of a fake 0x40-byte building profile (layouts.md);
//   - corrupt maps (a cycle, wrong counts, a deep chain, bad links and
//     colours) that the walk must refuse without hanging, and that
//     FillGameMapI32 must leave untouched;
//   - maps we build ourselves (FC4GameMapBuild.h): valid red-black trees the
//     walk accepts, every size up to 300, freed block for block.

#include "FC4GameMap.h"
#include "FC4GameMapBuild.h"

#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <map>
#include <random>
#include <vector>

using fc4::GameMapFill;
using fc4::GameMapNode;
using fc4::GameMapObject;
using fc4::GameMapStatus;

namespace {

int g_failures = 0;
#define CHECK(cond)                                                     \
    do {                                                                \
        if (!(cond)) {                                                  \
            std::printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond); \
            ++g_failures;                                               \
        }                                                               \
    } while (0)

// ---- the fake B allocator ----------------------------------------------------------------------------

struct BBlock {
    size_t size;
    bool live;
};
std::map<void*, BBlock> g_blocks;
int g_allocs = 0, g_frees = 0, g_badFrees = 0;

void* BAlloc(size_t size) {
    void* p = std::calloc(1, size);
    g_blocks[p] = BBlock{size, true};
    ++g_allocs;
    return p;
}

void BFree(void* p) {
    auto it = g_blocks.find(p);
    if (it == g_blocks.end() || !it->second.live) {
        ++g_badFrees;
        return;
    }
    it->second.live = false;
    ++g_frees;
}

bool IsLiveNodeBlock(const void* p) {
    auto it = g_blocks.find(const_cast<void*>(p));
    return it != g_blocks.end() && it->second.live && it->second.size == sizeof(GameMapNode);
}

void ReleaseAllBlocks() {
    for (auto& b : g_blocks) std::free(b.first);
    g_blocks.clear();
}

// ---- a fake game map (allocators.md) -----------------------------------------------------------------

GameMapNode* NewNode(uint32_t key, uint32_t value) {
    auto* n = static_cast<GameMapNode*>(BAlloc(sizeof(GameMapNode)));
    n->key = key;
    n->value = value;
    return n;
}

// The constructor: a B header of the node size, colour 0, parent null, left = right = header.
void InitMap(GameMapObject* m, uint32_t comparatorSentinel = 0xC0FFEE00u) {
    GameMapNode* h = NewNode(0, 0);
    h->color = fc4::kGameMapRed;
    h->parent = nullptr;
    h->left = h;
    h->right = h;
    m->header = h;
    m->count = 0;
    m->comparator = comparatorSentinel;  // +0x08: never read or written by anyone
}

void RotateLeft(GameMapNode* x, GameMapNode*& root) {
    GameMapNode* y = x->right;
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

void RotateRight(GameMapNode* x, GameMapNode*& root) {
    GameMapNode* y = x->left;
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

// allocators.md, Insertion step 3: the plain red-black insert fix-up.
void Rebalance(GameMapNode* x, GameMapNode*& root) {
    x->color = fc4::kGameMapRed;
    while (x != root && x->parent->color == fc4::kGameMapRed) {
        GameMapNode* gp = x->parent->parent;
        if (x->parent == gp->left) {
            GameMapNode* y = gp->right;
            if (y != nullptr && y->color == fc4::kGameMapRed) {
                x->parent->color = fc4::kGameMapBlack;
                y->color = fc4::kGameMapBlack;
                gp->color = fc4::kGameMapRed;
                x = gp;
            } else {
                if (x == x->parent->right) {
                    x = x->parent;
                    RotateLeft(x, root);
                }
                x->parent->color = fc4::kGameMapBlack;
                x->parent->parent->color = fc4::kGameMapRed;
                RotateRight(x->parent->parent, root);
            }
        } else {
            GameMapNode* y = gp->left;
            if (y != nullptr && y->color == fc4::kGameMapRed) {
                x->parent->color = fc4::kGameMapBlack;
                y->color = fc4::kGameMapBlack;
                gp->color = fc4::kGameMapRed;
                x = gp;
            } else {
                if (x == x->parent->left) {
                    x = x->parent;
                    RotateRight(x, root);
                }
                x->parent->color = fc4::kGameMapBlack;
                x->parent->parent->color = fc4::kGameMapRed;
                RotateLeft(x->parent->parent, root);
            }
        }
    }
    root->color = fc4::kGameMapBlack;
}

// allocators.md, Insertion steps 1, 2 and 4 (a unique insert; the key is known absent).
GameMapNode* InsertAbsent(GameMapObject* m, uint32_t key, uint32_t value) {
    GameMapNode* h = m->header;
    GameMapNode* y = h;
    GameMapNode* x = h->parent;
    while (x != nullptr) {
        y = x;
        x = key < x->key ? x->left : x->right;
    }
    GameMapNode* z = NewNode(key, value);
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
    Rebalance(z, h->parent);
    ++m->count;
    return z;
}

int g_indexCalls = 0, g_eraseCalls = 0;

// operator[] (0x00497A00 in the game): the value's address; a missing key is inserted with value 0.
void* __fastcall FakeIndex(GameMapObject* self, void*, const uint32_t* key) {
    ++g_indexCalls;
    GameMapNode* h = self->header;
    GameMapNode* y = h;  // lower bound
    for (GameMapNode* x = h->parent; x != nullptr;) {
        if (!(x->key < *key)) {
            y = x;
            x = x->left;
        } else {
            x = x->right;
        }
    }
    if (y == h || *key < y->key) y = InsertAbsent(self, *key, 0);
    return &y->value;
}

void EraseNodes(GameMapNode* x) {
    while (x != nullptr) {
        EraseNodes(x->right);
        GameMapNode* l = x->left;
        BFree(x);
        x = l;
    }
}

// The subtree free (0x004A79A0 in the game): every node under `top` with B; values are not destroyed.
void __fastcall FakeErase(GameMapObject*, void*, GameMapNode* top) {
    ++g_eraseCalls;
    EraseNodes(top);
}

void* __fastcall NullIndex(GameMapObject*, void*, const uint32_t*) {
    ++g_indexCalls;
    return nullptr;
}

// Inserts key + 1 instead of key: the map afterwards differs from what was asked.
void* __fastcall WrongKeyIndex(GameMapObject* self, void* edx, const uint32_t* key) {
    const uint32_t other = *key + 1;
    return FakeIndex(self, edx, &other);
}

void Put(GameMapObject* m, uint32_t key, uint32_t value) {
    void* slot = FakeIndex(m, nullptr, &key);
    std::memcpy(slot, &value, sizeof(value));
}

// Invariant 5: root black, no red node with a red child, equal black heights. -1 when broken.
int BlackHeight(const GameMapNode* n) {
    if (n == nullptr) return 1;
    if (n->color == fc4::kGameMapRed) {
        if ((n->left != nullptr && n->left->color == fc4::kGameMapRed) ||
            (n->right != nullptr && n->right->color == fc4::kGameMapRed))
            return -1;
    }
    if (n->left != nullptr && n->left->parent != n) return -1;
    if (n->right != nullptr && n->right->parent != n) return -1;
    const int l = BlackHeight(n->left);
    const int r = BlackHeight(n->right);
    if (l < 0 || r < 0 || l != r) return -1;
    return l + (n->color == fc4::kGameMapBlack ? 1 : 0);
}

bool ValidRedBlack(const GameMapObject* m) {
    const GameMapNode* root = m->header->parent;
    if (root == nullptr) return m->count == 0;
    return root->color == fc4::kGameMapBlack && root->parent == m->header && BlackHeight(root) > 0;
}

std::vector<std::pair<uint32_t, uint32_t>> Walk(const GameMapObject* m, GameMapStatus* status, uint32_t maxNodes = 1000) {
    std::vector<std::pair<uint32_t, uint32_t>> out;
    *status = fc4::WalkGameMap(m, maxNodes, [&](const GameMapNode& n) { out.emplace_back(n.key, n.value); });
    return out;
}

// ---- tests -------------------------------------------------------------------------------------------

void TestLayout() {
    CHECK(sizeof(GameMapNode) == 0x18);
    CHECK(sizeof(GameMapObject) == 12);
    GameMapNode n{};
    n.value = 0xBEEF0012u;  // +0x16 holds junk the game never wrote
    CHECK(fc4::GameMapU16(n) == 0x0012);
}

void TestEmptyMap() {
    GameMapObject m{};
    InitMap(&m);
    GameMapStatus s;
    uint32_t visited = 99;
    s = fc4::WalkGameMap(&m, 10, [](const GameMapNode&) {}, &visited);
    CHECK(s == GameMapStatus::Ok);
    CHECK(visited == 0);
    CHECK(fc4::WalkGameMap(nullptr, 10, [](const GameMapNode&) {}) == GameMapStatus::NullMap);
    GameMapObject noHeader{nullptr, 0, 0};
    CHECK(fc4::WalkGameMap(&noHeader, 10, [](const GameMapNode&) {}) == GameMapStatus::NullHeader);
    m.header->color = fc4::kGameMapBlack;
    CHECK(fc4::WalkGameMap(&m, 10, [](const GameMapNode&) {}) == GameMapStatus::BadHeader);
    m.header->color = fc4::kGameMapRed;
    m.header->right = nullptr;
    CHECK(fc4::WalkGameMap(&m, 10, [](const GameMapNode&) {}) == GameMapStatus::BadEmpty);
}

// Inserts in a scrambled order through the fake operator[]: the walk visits in key order, the
// tree keeps every invariant of allocators.md.
void TestWalkOrder() {
    std::mt19937 rng(12345);
    for (uint32_t size : {1u, 2u, 3u, 7u, 40u, 300u}) {
        GameMapObject m{};
        InitMap(&m);
        std::vector<uint32_t> keys;
        for (uint32_t i = 0; i < size; ++i) keys.push_back(0x1000u + i * 0x10u);
        std::shuffle(keys.begin(), keys.end(), rng);
        for (uint32_t k : keys) Put(&m, k, k ^ 0x5A5Au);
        CHECK(m.count == size);
        CHECK(ValidRedBlack(&m));
        GameMapStatus s;
        auto got = Walk(&m, &s);
        CHECK(s == GameMapStatus::Ok);
        CHECK(got.size() == size);
        bool ordered = true;
        for (uint32_t i = 0; i < got.size(); ++i) {
            if (got[i].first != 0x1000u + i * 0x10u || got[i].second != (got[i].first ^ 0x5A5Au)) ordered = false;
        }
        CHECK(ordered);
        // A repeated key changes nothing structurally.
        Put(&m, keys[0], 7);
        CHECK(m.count == size);
        // maxNodes bounds the count.
        CHECK(fc4::WalkGameMap(&m, size - 1, [](const GameMapNode&) {}) ==
              (size == 0 ? GameMapStatus::Ok : GameMapStatus::TooMany));
    }
}

// allocators.md, GetLocalPopulationSummary, worked example.
void TestWorkedExample() {
    GameMapObject m{};
    InitMap(&m, 0xC0FFEE11u);
    GameMapNode* const header = m.header;
    Put(&m, 0x1010, 999);  // the previous call's entry
    GameMapNode* const old = header->parent;
    const int freesBefore = g_frees;
    g_indexCalls = 0;
    g_eraseCalls = 0;

    const uint32_t keys[] = {0x1010, 0x1011, 0x2010};
    const int32_t values[] = {1234, 2250, -56};
    GameMapFill r;
    fc4::FillGameMapI32(&m, keys, values, 3, &FakeIndex, &FakeErase, 4096, &r);
    CHECK(r.status == GameMapStatus::Ok);
    CHECK(r.cleared == 1);
    CHECK(r.written == 3);
    CHECK(r.countAfter == 3);
    CHECK(g_eraseCalls == 1);
    CHECK(g_indexCalls == 3);
    // The old node has been freed with B, and the count is 3.
    CHECK(g_frees == freesBefore + 1);
    CHECK(!IsLiveNodeBlock(old));
    CHECK(g_badFrees == 0);
    CHECK(m.count == 3);
    // The header is kept and +0x08 never written.
    CHECK(m.header == header);
    CHECK(IsLiveNodeBlock(header));
    CHECK(header->color == fc4::kGameMapRed);
    CHECK(m.comparator == 0xC0FFEE11u);
    // Root 0x1011 black, left 0x1010 red, right 0x2010 red.
    const GameMapNode* root = header->parent;
    CHECK(root != nullptr);
    if (root != nullptr && root->left != nullptr && root->right != nullptr) {
        CHECK(root->key == 0x1011);
        CHECK(root->color == fc4::kGameMapBlack);
        CHECK(root->left->key == 0x1010 && root->left->color == fc4::kGameMapRed);
        CHECK(root->right->key == 0x2010 && root->right->color == fc4::kGameMapRed);
        CHECK(header->left == root->left);
        CHECK(header->right == root->right);
        CHECK(root->parent == header);
        CHECK(IsLiveNodeBlock(root) && IsLiveNodeBlock(root->left) && IsLiveNodeBlock(root->right));
        int32_t v = 0;
        std::memcpy(&v, &root->left->value, 4);
        CHECK(v == 1234);
        std::memcpy(&v, &root->value, 4);
        CHECK(v == 2250);
        std::memcpy(&v, &root->right->value, 4);
        CHECK(v == -56);
    } else {
        CHECK(!"worked example: root or children missing");
    }
    CHECK(ValidRedBlack(&m));

    // Again, as the next month's call: three cleared, the same three written.
    fc4::FillGameMapI32(&m, keys, values, 3, &FakeIndex, &FakeErase, 4096, &r);
    CHECK(r.status == GameMapStatus::Ok && r.cleared == 3 && r.written == 3 && m.count == 3);
    CHECK(m.header == header && g_badFrees == 0);

    // No entries: the map is emptied to the constructor's shape.
    fc4::FillGameMapI32(&m, keys, values, 0, &FakeIndex, &FakeErase, 4096, &r);
    CHECK(r.status == GameMapStatus::Ok && r.cleared == 3 && r.written == 0 && m.count == 0);
    CHECK(header->parent == nullptr && header->left == header && header->right == header);

    // A count of 0: no subtree free at all.
    g_eraseCalls = 0;
    fc4::FillGameMapI32(&m, keys, values, 2, &FakeIndex, &FakeErase, 4096, &r);
    CHECK(r.status == GameMapStatus::Ok && r.cleared == 0 && r.written == 2 && m.count == 2);
    CHECK(g_eraseCalls == 0);
}

// The 40 local nodes of the shipped data (demand.md §4.5), in key order: still a valid tree.
void TestFillForty() {
    GameMapObject m{};
    InitMap(&m);
    std::vector<uint32_t> keys;
    std::vector<int32_t> values;
    for (uint32_t i = 0; i < 40; ++i) {
        keys.push_back(0x1010u + i);
        values.push_back(static_cast<int32_t>(i) * 37 - 500);
    }
    GameMapFill r;
    fc4::FillGameMapI32(&m, keys.data(), values.data(), 40, &FakeIndex, &FakeErase, 4096, &r);
    CHECK(r.status == GameMapStatus::Ok && r.written == 40 && m.count == 40);
    CHECK(ValidRedBlack(&m));
}

// A snapshot of every byte of the map object and its nodes (the walk order), to prove "untouched".
std::vector<unsigned char> Snapshot(const GameMapObject* m, const std::vector<GameMapNode*>& nodes) {
    std::vector<unsigned char> out(reinterpret_cast<const unsigned char*>(m),
                                   reinterpret_cast<const unsigned char*>(m) + sizeof(*m));
    const auto* h = reinterpret_cast<const unsigned char*>(m->header);
    out.insert(out.end(), h, h + sizeof(GameMapNode));
    for (GameMapNode* n : nodes) {
        const auto* b = reinterpret_cast<const unsigned char*>(n);
        out.insert(out.end(), b, b + sizeof(GameMapNode));
    }
    return out;
}

struct Seven {
    GameMapObject m{};
    std::vector<GameMapNode*> nodes;  // in key order
    Seven() {
        InitMap(&m);
        for (uint32_t k : {0x40u, 0x20u, 0x60u, 0x10u, 0x30u, 0x50u, 0x70u}) Put(&m, k, k);
        fc4::WalkGameMap(&m, 100, [&](const GameMapNode& n) { nodes.push_back(const_cast<GameMapNode*>(&n)); });
    }
    GameMapNode* Key(uint32_t k) {
        for (GameMapNode* n : nodes)
            if (n->key == k) return n;
        return nullptr;
    }
};

GameMapStatus WalkStatus(const GameMapObject* m, uint32_t* visited = nullptr) {
    return fc4::WalkGameMap(m, 1000, [](const GameMapNode&) {}, visited);
}

void TestCorrupt() {
    {
        Seven t;  // the shape: 0x40 root, 0x20 / 0x60, leaves 0x10 0x30 0x50 0x70
        CHECK(t.nodes.size() == 7);
        CHECK(WalkStatus(&t.m) == GameMapStatus::Ok);
        CHECK(t.m.header->parent == t.Key(0x40));
    }
    {
        // A cycle: the largest leaf's right link points back at the root.
        Seven t;
        t.Key(0x70)->right = t.Key(0x40);
        uint32_t visited = 0;
        CHECK(WalkStatus(&t.m, &visited) != GameMapStatus::Ok);
        CHECK(visited <= 7);
    }
    {
        // A cycle that passes the child -> parent check: a leaf that is its own right child and parent,
        // reached from its real parent (whose link is then inconsistent).
        Seven t;
        GameMapNode* leaf = t.Key(0x30);
        leaf->right = leaf;
        leaf->parent = leaf;
        uint32_t visited = 0;
        CHECK(WalkStatus(&t.m, &visited) != GameMapStatus::Ok);
        CHECK(visited <= 7);
    }
    {
        // A cycle through the parent links of the smallest node: the depth cap stops the climb.
        Seven t;
        t.Key(0x10)->parent = t.Key(0x10);
        CHECK(WalkStatus(&t.m) == GameMapStatus::TooDeep);
    }
    {
        // A node that drops out of the tree (its parent forgets it; it loops on itself): the walk
        // finds 6 nodes for a count of 7.
        Seven t;
        GameMapNode* n50 = t.Key(0x50);
        n50->left = n50;
        n50->parent = n50;
        t.Key(0x60)->left = nullptr;  // 0x60 no longer reaches it; 0x40's right subtree shrinks
        uint32_t visited = 0;
        CHECK(WalkStatus(&t.m, &visited) == GameMapStatus::CountMismatch);  // 6 nodes for a count of 7
    }
    {
        // Wrong counts: one too many, one too few, 0 with nodes.
        Seven t;
        t.m.count = 8;
        uint32_t visited = 0;
        CHECK(WalkStatus(&t.m, &visited) == GameMapStatus::CountMismatch);
        CHECK(visited == 7);
        t.m.count = 6;
        CHECK(WalkStatus(&t.m, &visited) == GameMapStatus::CountMismatch);
        CHECK(visited == 6);
        t.m.count = 0;
        CHECK(WalkStatus(&t.m) == GameMapStatus::BadEmpty);
    }
    {
        // header.right / header.left not the largest / smallest node.
        Seven t;
        t.m.header->right = t.Key(0x60);
        CHECK(WalkStatus(&t.m) == GameMapStatus::BadLink);
        t.m.header->right = t.Key(0x70);
        t.m.header->left = t.Key(0x20);
        CHECK(WalkStatus(&t.m) == GameMapStatus::BadLink);
    }
    {
        // A root whose parent is not the header; a null root.
        Seven t;
        t.Key(0x40)->parent = nullptr;
        CHECK(WalkStatus(&t.m) == GameMapStatus::BadLink);
        t.Key(0x40)->parent = t.m.header;
        t.m.header->parent = nullptr;
        CHECK(WalkStatus(&t.m) == GameMapStatus::NullLink);
    }
    {
        // A colour byte other than 0 / 1; keys out of order.
        Seven t;
        t.Key(0x30)->color = 7;
        CHECK(WalkStatus(&t.m) == GameMapStatus::BadColor);
        t.Key(0x30)->color = fc4::kGameMapBlack;
        t.Key(0x30)->key = 0x05;
        CHECK(WalkStatus(&t.m) == GameMapStatus::KeyOrder);
    }
    {
        // A deep chain (right children only): refused at the depth cap; a shorter one walks.
        for (uint32_t len : {40u, 70u}) {
            GameMapObject m{};
            InitMap(&m);
            GameMapNode* h = m.header;
            GameMapNode* prev = nullptr;
            for (uint32_t i = 0; i < len; ++i) {
                GameMapNode* n = NewNode(0x100 + i, i);
                n->color = fc4::kGameMapBlack;
                if (prev == nullptr) {
                    h->parent = n;
                    h->left = n;
                    n->parent = h;
                } else {
                    prev->right = n;
                    n->parent = prev;
                }
                prev = n;
            }
            h->right = prev;
            m.count = len;
            uint32_t visited = 0;
            const GameMapStatus s = WalkStatus(&m, &visited);
            if (len <= fc4::kGameMapMaxDepth) {
                CHECK(s == GameMapStatus::Ok && visited == len);
            } else {
                CHECK(s == GameMapStatus::TooDeep);
                CHECK(visited <= fc4::kGameMapMaxDepth);
            }
        }
    }
}

// FillGameMapI32 on a map that fails the checks, or with bad input: nothing called, nothing written.
void TestFillRefuses() {
    {
        Seven t;
        t.m.count = 8;
        const auto before = Snapshot(&t.m, t.nodes);
        g_indexCalls = 0;
        g_eraseCalls = 0;
        const uint32_t keys[] = {1, 2};
        const int32_t values[] = {3, 4};
        GameMapFill r;
        fc4::FillGameMapI32(&t.m, keys, values, 2, &FakeIndex, &FakeErase, 4096, &r);
        CHECK(r.status == GameMapStatus::CountMismatch);
        CHECK(r.cleared == 0 && r.written == 0);
        CHECK(g_indexCalls == 0 && g_eraseCalls == 0);
        CHECK(Snapshot(&t.m, t.nodes) == before);
    }
    {
        Seven t;
        t.Key(0x70)->right = t.Key(0x40);  // the cycle
        const auto before = Snapshot(&t.m, t.nodes);
        g_indexCalls = 0;
        g_eraseCalls = 0;
        const uint32_t keys[] = {1};
        const int32_t values[] = {3};
        GameMapFill r;
        fc4::FillGameMapI32(&t.m, keys, values, 1, &FakeIndex, &FakeErase, 4096, &r);
        CHECK(r.status != GameMapStatus::Ok);
        CHECK(g_indexCalls == 0 && g_eraseCalls == 0);
        CHECK(Snapshot(&t.m, t.nodes) == before);
    }
    {
        // More nodes than the caller believes: TooMany, untouched.
        Seven t;
        const auto before = Snapshot(&t.m, t.nodes);
        const uint32_t keys[] = {1};
        const int32_t values[] = {3};
        GameMapFill r;
        fc4::FillGameMapI32(&t.m, keys, values, 1, &FakeIndex, &FakeErase, 6, &r);
        CHECK(r.status == GameMapStatus::TooMany);
        CHECK(Snapshot(&t.m, t.nodes) == before);
    }
    {
        // Keys not strictly increasing (a repeat): BadInput, untouched.
        Seven t;
        const auto before = Snapshot(&t.m, t.nodes);
        g_indexCalls = 0;
        g_eraseCalls = 0;
        const uint32_t keys[] = {1, 1};
        const int32_t values[] = {3, 4};
        GameMapFill r;
        fc4::FillGameMapI32(&t.m, keys, values, 2, &FakeIndex, &FakeErase, 4096, &r);
        CHECK(r.status == GameMapStatus::BadInput);
        CHECK(g_indexCalls == 0 && g_eraseCalls == 0);
        CHECK(Snapshot(&t.m, t.nodes) == before);
    }
    {
        // operator[] returning null: IndexFailed after the clear, nothing written.
        GameMapObject m{};
        InitMap(&m);
        Put(&m, 5, 5);
        const uint32_t keys[] = {1, 2};
        const int32_t values[] = {3, 4};
        GameMapFill r;
        fc4::FillGameMapI32(&m, keys, values, 2, &NullIndex, &FakeErase, 4096, &r);
        CHECK(r.status == GameMapStatus::IndexFailed);
        CHECK(r.cleared == 1 && r.written == 0 && m.count == 0);
    }
    {
        // An operator[] that inserts another key: the check afterwards says so.
        GameMapObject m{};
        InitMap(&m);
        const uint32_t keys[] = {0x10, 0x20};
        const int32_t values[] = {3, 4};
        GameMapFill r;
        fc4::FillGameMapI32(&m, keys, values, 2, &WrongKeyIndex, &FakeErase, 4096, &r);
        CHECK(r.status == GameMapStatus::BadResult);
        CHECK(r.written == 2 && m.count == 2);
    }
}

// ---- building profile capacity maps (layouts.md) ------------------------------------------------------

struct FakeProfile {
    unsigned char bytes[0x40];
    void SetVector(const void* begin, const void* end, const void* cap) {
        const void* v[3] = {begin, end, cap};
        std::memcpy(bytes + fc4::kProfileCapacityVectorOffset, v, sizeof(v));
    }
};

// A map<uint32_t, uint16_t>: the high half of +0x14 holds junk (never written by the game).
void PutU16(GameMapObject* m, uint32_t key, uint16_t amount) { Put(m, key, 0xDEAD0000u | amount); }

void TestCapacityRead() {
    std::vector<GameMapObject> maps(2);
    InitMap(&maps[0]);
    InitMap(&maps[1]);
    PutU16(&maps[0], 0x1030, 3);  // scrambled insert order
    PutU16(&maps[0], 0x1010, 12);
    PutU16(&maps[0], 0x1020, 7);
    PutU16(&maps[1], 0x3110, 40);

    FakeProfile p{};
    std::memset(p.bytes, 0xAB, sizeof(p.bytes));
    fc4::GameCapacityRead r;

    // One map: key order, the low 16 bits.
    p.SetVector(&maps[0], &maps[0] + 1, &maps[0] + 1);
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status == GameMapStatus::Ok);
    CHECK(r.maps == 1 && r.count == 3);
    CHECK(r.entries[0].devType == 0x1010 && r.entries[0].amount == 12 && r.entries[0].map == 0);
    CHECK(r.entries[1].devType == 0x1020 && r.entries[1].amount == 7);
    CHECK(r.entries[2].devType == 0x1030 && r.entries[2].amount == 3);

    // Two maps (spare capacity in the vector): map by map.
    GameMapObject* buf = maps.data();
    p.SetVector(buf, buf + 2, buf + 2);
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status == GameMapStatus::Ok && r.maps == 2 && r.count == 4);
    CHECK(r.entries[3].map == 1 && r.entries[3].devType == 0x3110 && r.entries[3].amount == 40);

    // No property: a never-pushed vector, no maps.
    p.SetVector(nullptr, nullptr, nullptr);
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status == GameMapStatus::Ok && r.maps == 0 && r.count == 0);

    // An empty map in the vector: no entries.
    GameMapObject empty{};
    InitMap(&empty);
    p.SetVector(&empty, &empty + 1, &empty + 1);
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status == GameMapStatus::Ok && r.maps == 1 && r.count == 0);

    // Not a vector of 12-byte maps.
    const auto* b = reinterpret_cast<const unsigned char*>(buf);
    p.SetVector(nullptr, buf, buf);
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status == GameMapStatus::BadVector && r.count == 0);
    p.SetVector(buf + 1, buf, buf + 1);  // end < begin
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status == GameMapStatus::BadVector);
    p.SetVector(buf, b + 13, b + 24);  // not a multiple of 12
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status == GameMapStatus::BadVector);
    p.SetVector(buf, buf + 1, buf);  // capacity below end
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status == GameMapStatus::BadVector);
    std::vector<GameMapObject> many(fc4::kMaxCapacityMaps + 1);
    for (GameMapObject& m : many) InitMap(&m);
    p.SetVector(many.data(), many.data() + many.size(), many.data() + many.size());
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status == GameMapStatus::BadVector);
    p.SetVector(many.data(), many.data() + fc4::kMaxCapacityMaps, many.data() + many.size());
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status == GameMapStatus::Ok && r.maps == fc4::kMaxCapacityMaps && r.count == 0);

    // A corrupt second map: all or nothing, and which map.
    maps[1].count = 2;
    p.SetVector(buf, buf + 2, buf + 2);
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status == GameMapStatus::CountMismatch);
    CHECK(r.badMap == 1 && r.maps == 2 && r.count == 0);
    maps[1].count = 1;

    // A cycle in the first map.
    GameMapNode* largest = maps[0].header->right;
    largest->right = maps[0].header->parent;
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status != GameMapStatus::Ok && r.badMap == 0 && r.count == 0);
    largest->right = nullptr;
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status == GameMapStatus::Ok && r.count == 4);

    // More entries than the read holds: TooMany.
    GameMapObject big{};
    InitMap(&big);
    for (uint32_t i = 0; i <= fc4::kMaxCapacityEntries; ++i) PutU16(&big, 0x1000 + i, 1);
    p.SetVector(&big, &big + 1, &big + 1);
    fc4::ReadProfileCapacities(&p, &r);
    CHECK(r.status == GameMapStatus::TooMany && r.count == 0);

    fc4::ReadProfileCapacities(nullptr, &r);
    CHECK(r.status == GameMapStatus::NullMap);
}

}  // namespace

// ---- maps we build (FC4GameMapBuild.h) -----------------------------------------------------------------

struct FakeB {
    static void* Allocate(uint32_t size) { return BAlloc(size); }
    static void Free(void* p) { BFree(p); }
};

void TestBuild() {
    for (size_t n = 0; n <= 300; ++n) {
        std::vector<uint32_t> keys(n), values(n);
        for (size_t i = 0; i < n; ++i) {
            keys[i] = static_cast<uint32_t>(i * 7 + 3);
            values[i] = static_cast<uint32_t>(0xA0000000u + i);
        }
        const int allocs = g_allocs, frees = g_frees;
        GameMapObject m{nullptr, 0, 0x5EED5EEDu};
        CHECK(fc4::BuildGameMap<FakeB>(m, keys.data(), values.data(), n));
        CHECK(g_allocs - allocs == static_cast<int>(n) + 1);
        CHECK(m.count == n);
        CHECK(m.comparator == 0x5EED5EEDu);
        CHECK(m.header != nullptr && m.header->color == fc4::kGameMapRed);
        CHECK(ValidRedBlack(&m));
        GameMapStatus s = GameMapStatus::Ok;
        const auto got = Walk(&m, &s, 1000);
        CHECK(s == GameMapStatus::Ok);
        CHECK(got.size() == n);
        for (size_t i = 0; i < got.size() && i < n; ++i) CHECK(got[i].first == keys[i] && got[i].second == values[i]);
        if (n == 0) CHECK(m.header->parent == nullptr && m.header->left == m.header && m.header->right == m.header);
        fc4::FreeGameMap<FakeB>(m);
        CHECK(m.header == nullptr && m.count == 0);
        CHECK(g_frees - frees == static_cast<int>(n) + 1);
    }
    // Keys out of order or repeated: refused, nothing allocated, the map untouched.
    const uint32_t bad[3] = {5, 5, 9};
    const uint32_t vals[3] = {1, 2, 3};
    GameMapObject m{nullptr, 77, 0};
    const int allocs = g_allocs;
    CHECK(!fc4::BuildGameMap<FakeB>(m, bad, vals, 3));
    CHECK(m.header == nullptr && m.count == 77 && g_allocs == allocs);
}

int main() {
    TestLayout();
    TestEmptyMap();
    TestWalkOrder();
    TestWorkedExample();
    TestFillForty();
    TestCorrupt();
    TestFillRefuses();
    TestCapacityRead();
    TestBuild();
    CHECK(g_badFrees == 0);
    ReleaseAllBlocks();
    if (g_failures != 0) {
        std::printf("game_map_test: %d failure(s)\n", g_failures);
        return 1;
    }
    std::printf("game_map_test: all checks passed\n");
    return 0;
}
