// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4ProcessHashState.h"
#include <cstdio>

namespace {
unsigned destroyedNodes = 0;
struct Value {
    unsigned refs = 2;
    void Release() { --refs; }
};
struct Node {
    Node* next;
    Value* value;
    ~Node() { ++destroyedNodes; }
};
struct Table {
    void** begin;
    void** end;
    void** capacityEnd;
    uint32_t count;
};
using State = fc4::ProcessHashState<Table, Node>;
State& ProcessState() {
    static State state;
    return state;
}
void Populate(State& state, Value* value) {
    state.buckets = new void*[2]{};
    state.used = 2;
    state.nodeCount = 2;
    state.table = {state.buckets, state.buckets + 2, state.buckets + 2, 2};
    state.buckets[1] = new Node{new Node{nullptr, value}, value};
}
bool Empty(const State& state) {
    return state.buckets == nullptr && state.used == 0 && state.nodeCount == 0 &&
           state.table.begin == nullptr && state.table.end == nullptr &&
           state.table.capacityEnd == nullptr && state.table.count == 0;
}
}

int main() {
    Value value;
    State* stable = &ProcessState();
    stable->ownsValues = true;
    Populate(*stable, &value);
    void** buffer = stable->buckets;
    stable->Clear();  // Rebuild releases every map reference, retaining buckets.
    if (value.refs != 0 || destroyedNodes != 2 || stable->buckets != buffer ||
        stable->used != 2 || stable->nodeCount != 0 || stable->table.count != 0) return 1;
    stable->Reset();  // App shutdown must keep the static state itself alive.
    stable->Reset();
    if (&ProcessState() != stable || !Empty(*stable) || destroyedNodes != 2) return 2;
    Value borrowed;
    {
        State family;
        Populate(family, &borrowed);
        // Family entries owe no value references; destructor must free storage.
    }
    if (borrowed.refs != 2 || destroyedNodes != 4) return 3;
    Value second;
    Populate(*stable, &second);
    stable->Reset();
    if (second.refs != 0 || destroyedNodes != 6 || !Empty(*stable)) return 4;
    std::puts("process hash state: static lifetime, rebuild, repeated reset, owned and borrowed values passed");
    return 0;
}
