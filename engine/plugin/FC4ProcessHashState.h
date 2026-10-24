// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include <cstdint>

namespace fc4 {

// Process tables keep stable object addresses. Only their nodes and bucket
// arrays are heap allocations; application shutdown must never delete a table
// state obtained from function-local static storage.
template <class Table, class Node>
struct ProcessHashState {
    Table table{};
    void** buckets = nullptr;
    uint32_t used = 0;
    uint32_t nodeCount = 0;
    bool ownsValues = false;

    ProcessHashState() = default;
    ProcessHashState(const ProcessHashState&) = delete;
    ProcessHashState& operator=(const ProcessHashState&) = delete;

    void Clear() {
        if (buckets != nullptr) {
            for (uint32_t i = 0; i < used; ++i) {
                for (Node* n = static_cast<Node*>(buckets[i]); n != nullptr;) {
                    Node* next = n->next;
                    if (ownsValues && n->value != nullptr) n->value->Release();
                    delete n;
                    n = next;
                }
                buckets[i] = nullptr;
            }
        }
        nodeCount = 0;
        table.count = 0;
    }

    void Reset() {
        Clear();
        delete[] buckets;
        buckets = nullptr;
        used = 0;
        table = {};
    }

    ~ProcessHashState() { Reset(); }
};

}  // namespace fc4
