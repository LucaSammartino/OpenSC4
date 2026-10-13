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

// Game containers whose memory belongs to the game's STL node allocator
// (instance B, docs/abi/allocators.md).
//
// The game has three pooled allocator instances; a block must go back to the
// instance it came from. The SDK's SC4List / SC4Vector / SC4String use the
// framework allocator service (A), so they must not cross the boundary where
// the game's side declares std::list / std::vector / cRZString (B): every
// block freed to the wrong instance seeds a foreign block into its pools
// (in-game runs 13-16).
//
// These types have the game's layouts (the same as the SDK's) and do all
// their allocation through one allocator policy, by default B:
//   GameList<T>    one pointer to a 12-byte head node; nodes {next, prev, value}
//   GameVector<T>  {begin, end, capacity_end}; elements copied bitwise
//   GameString     a cRZString {vtable, begin, end, capacity_end, refcount}
//                  with a B character buffer and the game's vtable
// The game owns the containers it passes us by reference: we fill, clear and
// grow them, never destroy them. Containers we create ourselves are destroyed
// by their destructors (elements released / destroyed, memory freed to B).
//
// An allocator policy has static Available(), Allocate(uint32_t), Free(void*)
// and StringVtable(); tests inject counting ones.
#pragma once

#include "SC4List.h"
#include "cIGZUnknown.h"
#include "compat/FC4Compat.h"

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <new>
#include <type_traits>

namespace fc4 {

// The game's STL node allocator and cRZString vtable (compat table; refused
// on an unknown executable).
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
    static const void* StringVtable() { return compat::GameStringVtable(); }
};

// ---- strings ------------------------------------------------------------------------------------

// The game's cRZString (allocators.md, Strings). Plain data: copying it
// bitwise moves the ownership of its buffer.
struct GameString {
    const void* vtable;
    char* begin;
    char* end;
    char* capacityEnd;
    uint32_t refCount;

    const char* data() const { return begin; }
    size_t size() const { return static_cast<size_t>(end - begin); }
};
static_assert(sizeof(void*) != 4 || sizeof(GameString) == 0x14, "cRZString is 0x14 bytes");

// Builds `s` (raw storage) as a game string holding `n` bytes of `text`, with
// a character buffer from Alloc. False (s untouched) without the allocator or
// the vtable.
//
// [abi] As the game's char* constructor does (allocators.md, Strings): exactly
// n + 1 bytes, end at the terminator, capacity_end = begin + n + 1 (begin plus
// the bytes allocated, terminator slot included), reference count 0 (embedded
// strings are destroyed inline, never reference-counted).
template <class Alloc = StlNodeAllocator>
bool GameStringInit(GameString& s, const char* text, size_t n) {
    const void* vt = Alloc::StringVtable();
    if (vt == nullptr || n > 0x7FFFFF00u) return false;
    char* b = static_cast<char*>(Alloc::Allocate(static_cast<uint32_t>(n + 1)));
    if (b == nullptr) return false;
    if (n != 0) std::memcpy(b, text, n);
    b[n] = 0;
    s.vtable = vt;
    s.begin = b;
    s.end = b + n;
    s.capacityEnd = b + n + 1;
    s.refCount = 0;
    return true;
}

// What the game does inline (allocators.md): free the buffer with B.
template <class Alloc = StlNodeAllocator>
void GameStringDestroy(GameString& s) {
    Alloc::Free(s.begin);
    s.begin = s.end = s.capacityEnd = nullptr;
}

// ---- element destruction ------------------------------------------------------------------------

// How a GameVector destroys an element (clear, destructor). Plain data by
// default; types holding game strings specialize it.
template <class T>
struct GameElement {
    static_assert(std::is_trivially_copyable_v<T> && std::is_trivially_destructible_v<T>,
                  "GameVector elements are plain data copied bitwise");
    template <class Alloc>
    static void Destroy(T&) {}
};

template <>
struct GameElement<GameString> {
    template <class Alloc>
    static void Destroy(GameString& s) {
        GameStringDestroy<Alloc>(s);
    }
};

// cISC4BudgetSimulator::BudgetGroupInfo as the game's callers build and
// destroy it (allocators.md table (2): 0x28 bytes, the name's buffer is B).
struct GameBudgetGroupInfo {
    uint32_t groupID;
    GameString groupName;
    int64_t totalExpense;
    int64_t totalIncome;
};
static_assert(sizeof(void*) != 4 || (sizeof(GameBudgetGroupInfo) == 0x28 && offsetof(GameBudgetGroupInfo, groupName) == 4 &&
                                     offsetof(GameBudgetGroupInfo, totalExpense) == 0x18 &&
                                     offsetof(GameBudgetGroupInfo, totalIncome) == 0x20),
              "BudgetGroupInfo layout (allocators.md)");

template <>
struct GameElement<GameBudgetGroupInfo> {
    template <class Alloc>
    static void Destroy(GameBudgetGroupInfo& g) {
        GameStringDestroy<Alloc>(g.groupName);
    }
};

// ---- vectors ------------------------------------------------------------------------------------

template <typename T, typename Alloc = StlNodeAllocator>
class GameVector {
public:
    GameVector() = default;
    GameVector(const GameVector&) = delete;
    GameVector& operator=(const GameVector&) = delete;
    // Only for vectors we create; the game destroys the ones it passes us.
    ~GameVector() {
        clear();
        Alloc::Free(begin_);
        begin_ = end_ = capacityEnd_ = nullptr;
    }

    size_t size() const { return static_cast<size_t>(end_ - begin_); }
    size_t capacity() const { return static_cast<size_t>(capacityEnd_ - begin_); }
    bool empty() const { return begin_ == end_; }
    T* begin() { return begin_; }
    T* end() { return end_; }
    T& operator[](size_t i) { return begin_[i]; }

    // Destroys the elements; keeps the buffer (as the game's clear does).
    void clear() {
        for (T* p = begin_; p != end_; ++p) GameElement<T>::template Destroy<Alloc>(*p);
        end_ = begin_;
    }

    // Grows the buffer to at least `n` elements with Alloc, freeing the old
    // buffer (even one the game allocated: it is B too) with Alloc.
    bool reserve(size_t n) {
        if (n <= capacity()) return true;
        if (n > 0x7FFFFFFFu / sizeof(T)) return false;
        T* fresh = static_cast<T*>(Alloc::Allocate(static_cast<uint32_t>(n * sizeof(T))));
        if (fresh == nullptr) return false;
        const size_t count = size();
        if (count != 0) std::memcpy(static_cast<void*>(fresh), static_cast<const void*>(begin_), count * sizeof(T));
        Alloc::Free(begin_);
        begin_ = fresh;
        end_ = fresh + count;
        capacityEnd_ = fresh + n;
        return true;
    }

    // Appends `value` bitwise: the vector takes over whatever it owns (a
    // string buffer, a reference).
    bool push_back(const T& value) {
        if (end_ == capacityEnd_) {
            const size_t c = capacity();
            if (!reserve(c < 4 ? 4 : c * 2)) return false;
        }
        std::memcpy(static_cast<void*>(end_), static_cast<const void*>(&value), sizeof(T));
        ++end_;
        return true;
    }

private:
    T* begin_ = nullptr;
    T* end_ = nullptr;
    T* capacityEnd_ = nullptr;
};

static_assert(sizeof(GameVector<uint32_t>) == 3 * sizeof(void*), "vector layout: begin, end, capacity_end");

// ---- lists --------------------------------------------------------------------------------------

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

    // Only for lists we create; the game destroys the ones it passes us.
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

    // Appends a node from Alloc holding `value` (the list takes over whatever
    // the value owns, e.g. a reference). False if out of memory.
    bool push_back(const T& value) {
        Node* n = static_cast<Node*>(Alloc::Allocate(static_cast<uint32_t>(sizeof(Node))));
        if (n == nullptr) return false;
        new (&n->value) T(value);
        n->next = root_;
        n->prev = root_->prev;
        root_->prev->next = n;
        root_->prev = n;
        return true;
    }

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

    Node* root_;  // the only member: the game's list layout
};

static_assert(sizeof(GameList<cIGZUnknown*>) == sizeof(void*), "list layout: the head pointer");

}  // namespace fc4
