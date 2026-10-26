// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include "FC4GameSet.h"

#include <vector>

namespace fc4 {

// network-manager.md §8.6 (NM15): unlike the zone outLots set, this boundary
// has only a header and count. Never access a comparator word after the count.
struct NetworkConfigurationSet {
    GameSetNode* header;
    uint32_t count;
};

#if defined(_M_IX86) || (defined(__SIZEOF_POINTER__) && __SIZEOF_POINTER__ == 4)
static_assert(sizeof(NetworkConfigurationSet) == 8, "network configuration set: 8 bytes");
#endif

template <class Alloc, class AddRef>
bool ReplaceNetworkConfigurationSet(NetworkConfigurationSet* out, const std::vector<uint32_t>& values,
                                    AddRef&& addRef) {
    if (out == nullptr) return false;
    // Adapt only the two specified words to the existing tree algorithm.
    GameSetObject tree{out->header, out->count, 0};
    const GameMapObject map{reinterpret_cast<GameMapNode*>(tree.header), tree.count, 0};
    std::vector<void*> old;
    if (WalkGameMap(&map, UINT32_MAX, [&](const GameMapNode& n) {
            old.push_back(const_cast<GameMapNode*>(&n));
        }) != GameMapStatus::Ok) return false;
    // Clearing frees nodes, never the caller's header or pointed-to objects.
    for (void* node : old) Alloc::Free(node);
    tree.header->parent = nullptr;
    tree.header->left = tree.header->right = tree.header;
    tree.count = out->count = 0;
    for (uint32_t value : values) {
        if (GameSetInsertU32<Alloc>(&tree, value, UINT32_MAX) == GameSetResult::Refused) return false;
        out->count = tree.count;
        addRef(value);  // NM15: unconditional, including a duplicate pointer.
    }
    return true;
}

}  // namespace fc4
