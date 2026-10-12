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

// Filling game-owned containers.
//
// SC4Vector::push_back copies the element bitwise (memmove), without running
// a copy constructor, and the container's owner (usually the game) destroys
// the element later. An element that owns memory (an SC4String, a struct
// holding one, anything with a vtable and a heap buffer) must therefore have
// exactly one owner after the push: the vector. Pushing a local and letting
// its destructor run frees the buffer the vector element still points at, and
// the owner frees it again (heap corruption, in-game run 8).
//
// fc4::PushOwned builds the element in raw storage, pushes its bytes and never
// destroys the source, which hands the ownership to the vector.
#pragma once

#include "SC4Vector.h"

#include <new>
#include <type_traits>
#include <utility>

namespace fc4 {

// Appends T{args...} to `v`; the vector (its owner) becomes the only owner.
template <typename T, typename... Args>
void PushOwned(SC4Vector<T>& v, Args&&... args) {
    alignas(T) unsigned char storage[sizeof(T)];
    T* p = new (storage) T{std::forward<Args>(args)...};
    v.push_back(*p);  // bitwise; `*p` is never destroyed
}

// For trivially destructible T a plain push_back is fine; this states it.
template <typename T>
void PushTrivial(SC4Vector<T>& v, const T& value) {
    static_assert(std::is_trivially_destructible_v<T>, "owning element: use fc4::PushOwned");
    v.push_back(value);
}

}  // namespace fc4
