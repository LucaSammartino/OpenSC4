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

// The lot manager's master lot list (the original's, or FreeCity4's mirror of
// it in the same layout), read through the safe lot
// iterator that lot manager slot 15 (GetSafeLotIterator) hands out, the way
// the original ComputeTax walks it (budget.md §5.2, BQ1 / BQ3: the tax total
// is a float32 running sum, so only this order matches it to the simoleon).
// The ABI is aura.md §6.5 and docs/abi/allocators.md table 1:
//
//   - slot 15 returns an 8-byte block from the global heap (allocator C):
//     {+0 the current list node, +4 the lot list object}, pointing at the
//     first lot and registered in the list object's iterator list;
//   - the list object's first word points to the master list's head node;
//     every node is 12 bytes {next, prev, value}, the value a cISC4Lot*;
//   - the next lot: if the current node is the head the walk is over, else
//     the lot is the node's value and current := next;
//   - the iterator list is at list object + 4 (a game list: one pointer to
//     its head node, B nodes whose values are the registered blocks);
//   - release: unregister (std::list::remove on list object + 4), then free
//     the block with C in every case (BQ4: also when no node matched or the
//     list object is null).
//
// Header-only, no game calls of its own (the caller passes the allocator B
// free), so tests/safe_lot_iterator_test.cpp drives it over fake lists. A
// walk over memory that is not such a list can still fault on a wild
// pointer: the plugin runs these under fc4::SehGuard.
#pragma once

#include <cstddef>
#include <cstdint>

namespace fc4 {
namespace safelots {

struct Node {
    Node* next;
    Node* prev;
    void* value;
};
static_assert(sizeof(Node) == 3 * sizeof(void*), "list node: next, prev, value");

struct Block {
    Node* current;     // +0
    void* listObject;  // +4
};
static_assert(sizeof(Block) == 2 * sizeof(void*), "iterator block: current node, list object");

enum class Status : int {
    Ok = 0,
    NoBlock,      // slot 15 returned null
    NoList,       // the block's list object is null
    NoHead,       // the list object's head pointer is null
    NotAtFirst,   // the block does not point at the first lot (head->next)
    BrokenLink,   // a node whose next is null or whose next->prev is not the node
    TooMany,      // more nodes than the bound (GetLotCount)
};

inline const char* StatusName(Status s) {
    switch (s) {
        case Status::Ok: return "ok";
        case Status::NoBlock: return "no iterator";
        case Status::NoList: return "null list object";
        case Status::NoHead: return "null head";
        case Status::NotAtFirst: return "iterator not at the first lot";
        case Status::BrokenLink: return "broken link";
        case Status::TooMany: return "more nodes than GetLotCount";
    }
    return "?";
}

// [abi] The original lot manager's lot list object is at lot manager + 0x5C
// (aura.md §6.5): its slot 15 blocks point there.
constexpr uintptr_t kOriginalListObjectOffset = 0x5C;

// The list object the blocks of `lotManager`'s slot 15 must point to for a
// raw walk, or null when its blocks are not walked raw:
//   - the original lot manager (its vtable, the object's first word, inside
//     the identified executable's image [imageBase, imageBase + sizeOfImage)):
//     lotManager + 0x5C;
//   - otherwise `replacementList`: FreeCity4's lot manager (enable = zonedev)
//     mirrors the master list in the original's raw layout and hands out the
//     same blocks (cFC4ZoneDeveloper.h); the caller passes its zone
//     developer's MasterListObject(), or null when the lot manager is not
//     FreeCity4's (then: null, never walked).
// A zero sizeOfImage (executable not identified) matches no vtable.
inline const void* ExpectedListObject(const void* lotManager, uintptr_t imageBase, uint32_t sizeOfImage,
                                      const void* replacementList) {
    if (lotManager == nullptr) return nullptr;
    const uintptr_t vtable = *static_cast<const uintptr_t*>(lotManager);
    if (vtable >= imageBase && vtable - imageBase < sizeOfImage) {
        return static_cast<const char*>(lotManager) + kOriginalListObjectOffset;
    }
    return replacementList;
}

// The master list's head node, or null.
inline Node* MasterHead(const Block* b) {
    if (b == nullptr || b->listObject == nullptr) return nullptr;
    return *static_cast<Node* const*>(b->listObject);
}

// Copies the lots of the master list into out[0..cap) in list order, taking
// each lot as the original does (the block's cursor advances to the head).
// Every node is checked before it is used (non-null, next non-null,
// next->prev == node); `cap` (GetLotCount) bounds the walk, so a cycle that
// avoids the head ends with TooMany. On anything but Ok, *count holds the
// lots copied so far and the caller must not use them.
inline Status CopyLots(Block* b, void** out, uint32_t cap, uint32_t* count) {
    *count = 0;
    if (b == nullptr) return Status::NoBlock;
    if (b->listObject == nullptr) return Status::NoList;
    Node* const head = MasterHead(b);
    if (head == nullptr) return Status::NoHead;
    if (head->next == nullptr || head->next->prev != head) return Status::BrokenLink;
    if (b->current != head->next) return Status::NotAtFirst;
    for (Node* n = b->current; n != head;) {
        if (*count >= cap) return Status::TooMany;
        if (n == nullptr || n->next == nullptr || n->next->prev != n) return Status::BrokenLink;
        out[(*count)++] = n->value;
        b->current = n->next;
        n = n->next;
    }
    return Status::Ok;
}

using FreeFn = void(__cdecl*)(void* block);

// Unregisters `b` from its list object's iterator list (list object + 4):
// every node whose value is `b` is unlinked and freed with `freeNode`
// (allocator B), which is what std::list::remove (`0x00809580`) does on that
// STLport list (a head pointer, no size field) *[abi: an equivalent, not the
// game's own function]*; the scan runs to the end, so duplicates go too
// (budget.md §5.2, BQ4). A null list object has nothing to unlink: 0, as the
// original skips the remove when the block's owner is null (BQ4). The whole
// ring is checked first (at most `bound` nodes, each link pointing back);
// nothing is changed unless it passes. Returns the number of nodes removed
// (0 when none matched), or -1 when the ring failed the checks or `b` /
// `freeNode` is null.
inline int Unregister(Block* b, FreeFn freeNode, uint32_t bound) {
    if (b == nullptr || freeNode == nullptr) return -1;
    if (b->listObject == nullptr) return 0;
    Node* const head = *reinterpret_cast<Node* const*>(static_cast<char*>(b->listObject) + sizeof(void*));
    if (head == nullptr || head->next == nullptr || head->prev == nullptr) return -1;
    uint32_t seen = 0;
    for (Node* n = head;;) {
        if (n->next == nullptr || n->next->prev != n) return -1;
        n = n->next;
        if (n == head) break;
        if (++seen > bound) return -1;
    }
    int removed = 0;
    for (Node* n = head->next; n != head;) {
        Node* const next = n->next;
        if (n->value == b) {
            n->prev->next = next;
            next->prev = n->prev;
            freeNode(n);
            ++removed;
        }
        n = next;
    }
    return removed;
}

// The release of ComputeTax's iterator (budget.md §5.2, BQ4): Unregister,
// then the 8-byte block is freed with `freeBlock` (allocator C) in every case
// the original reaches: after a remove that matched one node or more, one
// that matched none, and with a null list object (the original frees the
// block unconditionally at `0x006920AF`). [choice] When the ring fails
// Unregister's checks (-1) the block stays registered, so it is leaked, never
// freed: freeing it would leave a dangling node in the lot manager's ring (the
// original would walk that ring and fault). Returns Unregister's result; the
// block was freed exactly when it is 0 or more.
inline int Release(Block* b, FreeFn freeNode, FreeFn freeBlock, uint32_t bound) {
    if (freeBlock == nullptr) return -1;
    const int removed = Unregister(b, freeNode, bound);
    if (removed >= 0) freeBlock(b);
    return removed;
}

}  // namespace safelots
}  // namespace fc4
