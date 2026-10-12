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

// Lists the game fills from its STL node allocator (docs/abi/allocators.md).
//
// The game has three pooled allocator instances; a block must go back to the
// instance it came from. The SDK's SC4List allocates and frees its nodes
// through the framework allocator service, so it must not be handed to a game
// method whose list nodes come from the STL node allocator
// (cISC4LotManager::GetLotsInCellRect): every node it frees there seeds a
// foreign block into the service's pools (in-game runs 13-16).
//
// fc4::GameList<T> has the SC4List layout (one pointer: the head node; nodes
// {next, prev, value}) and allocates and frees every node, head included,
// with one allocator, by default the game's STL node allocator. Destroying it
// releases each remaining non-null cIGZUnknown value once and frees every
// node with that allocator.
#pragma once

#include "SC4List.h"
#include "cIGZUnknown.h"
#include "compat/FC4Compat.h"

#include <cstdint>
#include <new>
#include <type_traits>

namespace fc4 {

// The game's STL node allocator (compat table; refused on an unknown exe).
struct StlNodeAllocator {
    static bool Available() { return compat::StlNodeAllocator(nullptr, nullptr); }
    static void* Allocate(uint32_t size) {
        compat::StlNodeAllocateFn a = nullptr;
        return compat::StlNodeAllocator(&a, nullptr) ? a(size) : nullptr;
    }
    static void Free(void* p) {
        compat::StlNodeFreeFn f = nullptr;
        if (p != nullptr && compat::StlNodeAllocator(nullptr, &f)) f(p);
    }
};

template <typename T, typename Alloc = StlNodeAllocator>
class GameList {
public:
    using Node = SC4ListNode<T>;
    using iterator = SC4ListIterator<T>;

    // Allocates the head node; check valid() (false if the allocator is
    // unavailable or out of memory).
    GameList() : root_(nullptr) {
        if (!Alloc::Available()) return;
        Node* head = static_cast<Node*>(Alloc::Allocate(static_cast<uint32_t>(sizeof(Node))));
        if (head == nullptr) return;
        head->next = head;
        head->prev = head;
        if constexpr (std::is_pointer_v<T>) {
            head->value = nullptr;
        } else {
            new (&head->value) T();
        }
        root_ = head;
    }
    GameList(const GameList&) = delete;
    GameList& operator=(const GameList&) = delete;

    ~GameList() {
        if (root_ == nullptr) return;
        Node* n = root_->next;
        while (n != root_) {
            Node* next = n->next;
            DestroyValue(n->value);
            Alloc::Free(n);
            n = next;
        }
        if constexpr (!std::is_pointer_v<T>) root_->value.~T();
        Alloc::Free(root_);
        root_ = nullptr;
    }

    bool valid() const { return root_ != nullptr; }

    // What the game's methods take (same layout: the head pointer).
    SC4List<T>& Sdk() { return *reinterpret_cast<SC4List<T>*>(this); }

    Node* head() const { return root_; }
    iterator begin() { return iterator(root_->next); }
    iterator end() { return iterator(root_); }

private:
    static void DestroyValue(T& v) {
        if constexpr (std::is_pointer_v<T>) {
            if constexpr (std::is_base_of_v<cIGZUnknown, std::remove_pointer_t<T>>) {
                if (v != nullptr) v->Release();  // the list's reference
            }
            v = nullptr;
        } else {
            v.~T();
        }
    }

    Node* root_;  // the only member: SC4List layout
};

static_assert(sizeof(GameList<cIGZUnknown*>) == sizeof(SC4List<cIGZUnknown*>), "GameList must have the SC4List layout");

}  // namespace fc4
