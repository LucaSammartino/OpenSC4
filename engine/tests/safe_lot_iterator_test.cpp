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

// The master lot list through the safe lot iterator (plugin/FC4SafeLotIterator.h,
// budget.md §5.2 BQ1 / BQ3) against fake lists built as aura.md §6.5 and
// docs/abi/allocators.md describe them: a lot list object whose first word
// points to the master list's head and whose second word points to the
// iterator list's head, 12-byte nodes {next, prev, value}, and an 8-byte
// iterator block {current node, list object} registered in the iterator list.
//
//   - the walk returns the lots in list order and leaves the cursor at the head;
//   - an empty list, a cursor not at the first lot, a broken link and a cycle
//     that avoids the head (bounded by GetLotCount) are refused;
//   - unregistering unlinks exactly the block's node(s) and frees them with
//     the given B free; a broken iterator ring is refused untouched.

#include "FC4SafeLotIterator.h"

#include <cstdio>
#include <vector>

namespace {

int g_failures = 0;
#define CHECK(cond)                                                     \
    do {                                                                \
        if (!(cond)) {                                                  \
            std::printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond); \
            ++g_failures;                                               \
        }                                                               \
    } while (0)

using fc4::safelots::Block;
using fc4::safelots::Node;
using fc4::safelots::Status;

std::vector<void*> g_freed;
void __cdecl FakeFree(void* p) { g_freed.push_back(p); }

// A ring of `values` behind a head node (values in list order).
struct Ring {
    Node head{};
    std::vector<Node> nodes;
    explicit Ring(const std::vector<void*>& values) : nodes(values.size()) {
        head.next = head.prev = &head;
        for (size_t i = 0; i < values.size(); ++i) {
            Node* n = &nodes[i];
            n->value = values[i];
            n->prev = head.prev;
            n->next = &head;
            head.prev->next = n;
            head.prev = n;
        }
    }
    size_t Length() const {
        size_t k = 0;
        for (const Node* n = head.next; n != &head; n = n->next) ++k;
        return k;
    }
};

// The lot list object: {master head, iterator head}.
struct ListObject {
    Node* masterHead;
    Node* iteratorHead;
};

void* Lot(uintptr_t i) { return reinterpret_cast<void*>(0x1000 + 0x10 * i); }

void TestWalkInListOrder() {
    Ring master({Lot(3), Lot(1), Lot(2)});
    Block block{};
    Block other{};
    Ring iters({&other, &block});
    ListObject list{&master.head, &iters.head};
    block.current = master.head.next;
    block.listObject = &list;
    void* out[4] = {};
    uint32_t n = 99;
    CHECK(fc4::safelots::CopyLots(&block, out, 3, &n) == Status::Ok);
    CHECK(n == 3 && out[0] == Lot(3) && out[1] == Lot(1) && out[2] == Lot(2));
    CHECK(block.current == &master.head);  // advanced as the original does
    // Unregister: only the block's node goes, freed with B.
    g_freed.clear();
    Node* const mine = &iters.nodes[1];
    CHECK(fc4::safelots::Unregister(&block, &FakeFree, 16) == 1);
    CHECK(g_freed.size() == 1 && g_freed[0] == mine);
    CHECK(iters.Length() == 1 && iters.head.next->value == &other);
    CHECK(iters.head.next->next == &iters.head && iters.head.prev == iters.head.next);
    // A second release finds nothing: the caller must not free the block.
    g_freed.clear();
    CHECK(fc4::safelots::Unregister(&block, &FakeFree, 16) == 0);
    CHECK(g_freed.empty());
}

void TestEmptyAndBounds() {
    Ring master({});
    Block block{};
    Ring iters({&block});
    ListObject list{&master.head, &iters.head};
    block.current = master.head.next;  // the head itself
    block.listObject = &list;
    void* out[1] = {};
    uint32_t n = 99;
    CHECK(fc4::safelots::CopyLots(&block, out, 0, &n) == Status::Ok && n == 0);

    // More nodes than GetLotCount.
    Ring three({Lot(1), Lot(2), Lot(3)});
    ListObject list3{&three.head, &iters.head};
    Block b3{three.head.next, &list3};
    void* out3[3] = {};
    CHECK(fc4::safelots::CopyLots(&b3, out3, 2, &n) == Status::TooMany);
    // Exactly GetLotCount nodes is fine.
    b3.current = three.head.next;
    CHECK(fc4::safelots::CopyLots(&b3, out3, 3, &n) == Status::Ok && n == 3);
}

void TestRefusals() {
    void* out[8] = {};
    uint32_t n = 0;
    CHECK(fc4::safelots::CopyLots(nullptr, out, 8, &n) == Status::NoBlock);
    Block noList{};
    CHECK(fc4::safelots::CopyLots(&noList, out, 8, &n) == Status::NoList);
    ListObject nullHead{nullptr, nullptr};
    Block b{nullptr, &nullHead};
    CHECK(fc4::safelots::CopyLots(&b, out, 8, &n) == Status::NoHead);

    // Not at the first lot.
    Ring master({Lot(1), Lot(2)});
    ListObject list{&master.head, nullptr};
    Block second{&master.nodes[1], &list};  // the second lot
    CHECK(fc4::safelots::CopyLots(&second, out, 8, &n) == Status::NotAtFirst);

    // A broken back link.
    Ring broken({Lot(1), Lot(2), Lot(3)});
    ListObject lb{&broken.head, nullptr};
    broken.nodes[2].prev = &broken.nodes[0];
    Block bb{broken.head.next, &lb};
    CHECK(fc4::safelots::CopyLots(&bb, out, 8, &n) == Status::BrokenLink);

    // A cycle that never returns to the head: bounded by the lot count.
    Ring cyc({Lot(1), Lot(2), Lot(3)});
    cyc.nodes[2].next = &cyc.nodes[1];
    cyc.nodes[1].prev = &cyc.nodes[2];
    ListObject lc{&cyc.head, nullptr};
    Block bc{cyc.head.next, &lc};
    // nodes[0].next is nodes[1], whose prev is now nodes[2]: refused at once.
    CHECK(fc4::safelots::CopyLots(&bc, out, 8, &n) == Status::BrokenLink);
    // A consistent two-node cycle detached from the head.
    Ring det({Lot(1), Lot(2)});
    Node a{}, c{};
    a.next = &c;
    a.prev = &c;
    c.next = &a;
    c.prev = &a;
    a.value = Lot(7);
    c.value = Lot(8);
    det.head.next = &a;
    ListObject ld{&det.head, nullptr};
    Block bd{&a, &ld};
    // head->next->prev is not the head: refused before the walk.
    CHECK(fc4::safelots::CopyLots(&bd, out, 8, &n) == Status::BrokenLink);
    a.prev = &det.head;  // now the head link checks out, the cycle does not
    c.next = &a;
    CHECK(fc4::safelots::CopyLots(&bd, out, 8, &n) == Status::BrokenLink);
}

void TestUnregisterRefusesBrokenRing() {
    Block block{};
    Ring iters({&block, nullptr, &block});
    Ring master({});
    ListObject list{&master.head, &iters.head};
    block.listObject = &list;
    iters.nodes[1].prev = &iters.head;  // broken back link
    g_freed.clear();
    CHECK(fc4::safelots::Unregister(&block, &FakeFree, 16) == -1);
    CHECK(g_freed.empty() && iters.head.next == &iters.nodes[0]);
    iters.nodes[1].prev = &iters.nodes[0];
    // Too many registered iterators for the bound.
    CHECK(fc4::safelots::Unregister(&block, &FakeFree, 2) == -1);
    CHECK(g_freed.empty());
    // Both of the block's nodes go (std::list::remove removes every match).
    CHECK(fc4::safelots::Unregister(&block, &FakeFree, 16) == 2);
    CHECK(g_freed.size() == 2 && iters.Length() == 1 && iters.head.next->value == nullptr);
    CHECK(fc4::safelots::Unregister(nullptr, &FakeFree, 16) == -1);
    CHECK(fc4::safelots::Unregister(&block, nullptr, 16) == -1);
}

}  // namespace

int main() {
    TestWalkInListOrder();
    TestEmptyAndBounds();
    TestRefusals();
    TestUnregisterRefusesBrokenRing();
    if (g_failures != 0) {
        std::printf("safe_lot_iterator_test: %d failure(s)\n", g_failures);
        return 1;
    }
    std::printf("safe_lot_iterator_test: all checks passed\n");
    return 0;
}
