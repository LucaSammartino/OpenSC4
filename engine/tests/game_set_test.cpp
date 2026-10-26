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

// The game's STLport set (plugin/FC4GameSet.h): PlaceZone's outLots insert
// (zones.md §6, §13.2) against sets built as docs/abi/allocators.md ("map and
// set") describes them:
//
//   - a fake B allocator that counts blocks;
//   - the empty set (header: red, parent null, left = right = header);
//   - unique inserts in random orders, checked against std::set and the
//     red-black invariants (black root, no red-red, equal black heights,
//     consistent parent links, header.left / header.right the extremes);
//   - a present key allocates nothing; an inconsistent set and a failed
//     allocation leave the set untouched.

#include "FC4GameSet.h"
#include "FC4NetworkConfigurationSet.h"

#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <map>
#include <random>
#include <set>
#include <vector>

using fc4::GameSetInsertU32;
using fc4::GameSetNode;
using fc4::GameSetObject;
using fc4::GameSetResult;

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

std::map<void*, size_t> g_blocks;
int g_allocs = 0;
bool g_failNext = false;

struct FakeB {
    static void* Allocate(uint32_t size) {
        if (g_failNext) {
            g_failNext = false;
            return nullptr;
        }
        void* p = std::calloc(1, size);
        g_blocks[p] = size;
        ++g_allocs;
        return p;
    }
    static void Free(void* p) {
        g_blocks.erase(p);
        std::free(p);
    }
};

// A constructed empty set: the header is a separate B block.
GameSetObject MakeSet() {
    GameSetObject s{};
    GameSetNode* h = static_cast<GameSetNode*>(FakeB::Allocate(sizeof(GameSetNode)));
    h->color = fc4::kGameMapRed;
    h->parent = nullptr;
    h->left = h;
    h->right = h;
    s.header = h;
    s.count = 0;
    return s;
}

void FreeSubtree(GameSetNode* n) {
    if (n == nullptr) return;
    FreeSubtree(n->left);
    FreeSubtree(n->right);
    FakeB::Free(n);
}

void DestroySet(GameSetObject& s) {
    FreeSubtree(s.header->parent);
    FakeB::Free(s.header);
    s.header = nullptr;
    s.count = 0;
}

// Black height of the subtree, or -1 when a rule is broken.
int CheckNode(const GameSetNode* n, const GameSetNode* parent, std::vector<uint32_t>& keys) {
    if (n == nullptr) return 1;
    if (n->parent != parent) return -1;
    if (n->color != fc4::kGameMapRed && n->color != fc4::kGameMapBlack) return -1;
    if (n->color == fc4::kGameMapRed) {
        if ((n->left && n->left->color == fc4::kGameMapRed) || (n->right && n->right->color == fc4::kGameMapRed)) {
            return -1;
        }
    }
    const int l = CheckNode(n->left, n, keys);
    keys.push_back(n->key);
    const int r = CheckNode(n->right, n, keys);
    if (l < 0 || r < 0 || l != r) return -1;
    return l + (n->color == fc4::kGameMapBlack ? 1 : 0);
}

// The set holds exactly `expected`, as a valid STLport red-black tree.
bool SetMatches(const GameSetObject& s, const std::set<uint32_t>& expected) {
    const GameSetNode* h = s.header;
    if (h->color != fc4::kGameMapRed || s.count != expected.size()) return false;
    if (expected.empty()) return h->parent == nullptr && h->left == h && h->right == h;
    const GameSetNode* root = h->parent;
    if (root == nullptr || root->color != fc4::kGameMapBlack) return false;
    std::vector<uint32_t> keys;
    if (CheckNode(root, h, keys) < 0) return false;
    if (!std::equal(keys.begin(), keys.end(), expected.begin(), expected.end())) return false;
    const GameSetNode* mn = root;
    while (mn->left) mn = mn->left;
    const GameSetNode* mx = root;
    while (mx->right) mx = mx->right;
    return h->left == mn && h->right == mx;
}

void TestEmptyAndSingle() {
    GameSetObject s = MakeSet();
    std::set<uint32_t> ref;
    CHECK(SetMatches(s, ref));
    CHECK(GameSetInsertU32<FakeB>(&s, 0x1234, 100) == GameSetResult::Inserted);
    ref.insert(0x1234);
    CHECK(SetMatches(s, ref));
    const int before = g_allocs;
    CHECK(GameSetInsertU32<FakeB>(&s, 0x1234, 100) == GameSetResult::Present);
    CHECK(g_allocs == before);  // nothing allocated for a present key
    CHECK(SetMatches(s, ref));
    DestroySet(s);
}

void TestRandomOrders() {
    std::mt19937 rng(20261014);
    for (int round = 0; round < 50; ++round) {
        GameSetObject s = MakeSet();
        std::set<uint32_t> ref;
        const int n = 1 + static_cast<int>(rng() % 300);
        bool ok = true;
        for (int i = 0; i < n; ++i) {
            // Pointer-like keys with repeats.
            const uint32_t key = (rng() % 200) * 0x10 + 0x01000000u;
            const GameSetResult r = GameSetInsertU32<FakeB>(&s, key, 1000);
            const bool fresh = ref.insert(key).second;
            if (r != (fresh ? GameSetResult::Inserted : GameSetResult::Present)) ok = false;
            if (!SetMatches(s, ref)) ok = false;
        }
        CHECK(ok);
        // Ascending and descending runs (the rotations on one side).
        for (uint32_t k = 1; k <= 64; ++k) {
            GameSetInsertU32<FakeB>(&s, k, 2000);
            ref.insert(k);
        }
        for (uint32_t k = 0xFFFFFFFFu; k > 0xFFFFFFFFu - 64; --k) {
            GameSetInsertU32<FakeB>(&s, k, 2000);
            ref.insert(k);
        }
        CHECK(SetMatches(s, ref));
        DestroySet(s);
    }
    CHECK(g_blocks.empty());
}

void TestRefusals() {
    GameSetObject s = MakeSet();
    std::set<uint32_t> ref;
    for (uint32_t k : {5u, 3u, 8u}) {
        GameSetInsertU32<FakeB>(&s, k, 10);
        ref.insert(k);
    }
    CHECK(SetMatches(s, ref));
    // A wrong count: refused, nothing allocated.
    s.count = 7;
    fc4::GameMapStatus st = fc4::GameMapStatus::Ok;
    const int before = g_allocs;
    CHECK(GameSetInsertU32<FakeB>(&s, 9, 10, &st) == GameSetResult::Refused);
    CHECK(st != fc4::GameMapStatus::Ok);
    CHECK(g_allocs == before);
    s.count = 3;
    // More nodes than maxNodes: refused.
    CHECK(GameSetInsertU32<FakeB>(&s, 9, 2) == GameSetResult::Refused);
    // A failed allocation: refused, the set unchanged.
    g_failNext = true;
    CHECK(GameSetInsertU32<FakeB>(&s, 9, 10) == GameSetResult::Refused);
    CHECK(SetMatches(s, ref));
    // Then it works.
    CHECK(GameSetInsertU32<FakeB>(&s, 9, 10) == GameSetResult::Inserted);
    ref.insert(9);
    CHECK(SetMatches(s, ref));
    DestroySet(s);
    CHECK(g_blocks.empty());
}

void TestNetworkConfigurationFillRefill() {
    auto storage = MakeSet();
    struct Boundary {
        fc4::NetworkConfigurationSet set;
        uint32_t after = 0xDEADBEEF;
    } boundary{{storage.header, 0}};
    auto* const header = storage.header;
    const std::vector<uint32_t> catalogue{0x10002000, 0x10003000, 0x10001000};
    std::map<uint32_t, int> refs{{0x10001000, 1}, {0x10002000, 1}, {0x10003000, 1}};
    const auto fill = [&](const std::vector<uint32_t>& values) {
        CHECK(fc4::ReplaceNetworkConfigurationSet<FakeB>(&boundary.set, values,
            [&](uint32_t pointer) { ++refs[pointer]; }));
        CHECK(boundary.after == 0xDEADBEEF && boundary.set.header == header);
        storage.count = boundary.set.count;
    };
    fill(catalogue);
    CHECK(SetMatches(storage, {0x10001000, 0x10002000, 0x10003000}));
    CHECK(g_blocks.size() == 4);
    for (auto& item : refs) { CHECK(item.second == 2); --item.second; }  // caller Releases
    fill(catalogue);
    CHECK(SetMatches(storage, {0x10001000, 0x10002000, 0x10003000}));
    CHECK(g_blocks.size() == 4);  // old entries freed with B; header preserved
    for (auto& item : refs) { CHECK(item.second == 2); --item.second; }
    fill({0x10002000, 0x10002000});
    CHECK(storage.count == 1 && refs[0x10002000] == 3);  // AddRef even on duplicate
    refs[0x10002000] -= 2;
    fill({});
    CHECK(SetMatches(storage, {}) && g_blocks.size() == 1);
    for (auto& item : refs) CHECK(item.second == 1);  // clearing never Releases a value
    DestroySet(storage);
    CHECK(g_blocks.empty());
}

}  // namespace

int main() {
    TestEmptyAndSingle();
    TestRandomOrders();
    TestRefusals();
    TestNetworkConfigurationFillRefill();
    if (g_failures != 0) {
        std::printf("game_set_test: %d failure(s)\n", g_failures);
        return 1;
    }
    std::printf("game_set_test: all passed\n");
    return 0;
}
