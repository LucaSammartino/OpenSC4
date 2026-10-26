// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4LegacyCtlHooks.h"
#include <cassert>
#include <cstdio>
#include <thread>

namespace {
void Native() {}
void Hook() {}
void ForeignHook() {}
DWORD Protection(void* memory) {
    MEMORY_BASIC_INFORMATION info{};
    assert(VirtualQuery(memory, &info, sizeof(info)) == sizeof(info));
    return info.Protect;
}
}

int main() {
    using namespace fc4::legacyrecord;
    auto** table = static_cast<void**>(VirtualAlloc(nullptr, 4096, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE));
    assert(table);
    auto* native = reinterpret_cast<void*>(&Native);
    auto* hook = reinterpret_cast<void*>(&Hook);
    for (unsigned i = 0; i < 4; ++i) table[i] = native;
    DWORD previous = 0;
    assert(VirtualProtect(table, 4096, PAGE_READONLY, &previous));
    struct Object { void** table; } object{table};
    assert(!Patch(&object, 12, hook));

    // Repeated builds must preserve the complete native COM table when idle,
    // including its page protection, while nested interpreters keep recording.
    for (unsigned build = 0; build < 3; ++build) {
        HookScope outer;
        assert(Patch(&object, 12, hook));
        assert(Original(&object, 12) == native);
        assert(table[3] == hook && Protection(table) == PAGE_READONLY);
        {
            HookScope inner;
            assert(Patch(&object, 12, hook));
            assert(Patch(&object, 0, hook));
        }
        assert(table[0] == hook && table[3] == hook);
        // A second thread leaving a capture must not tear down the first one.
        std::thread nested([&] {
            HookScope concurrent;
            assert(Patch(&object, 4, hook));
        });
        nested.join();
        assert(table[1] == hook && table[3] == hook);
        assert(outer.Finish());
        for (unsigned i = 0; i < 4; ++i) assert(table[i] == native);
        assert(Protection(table) == PAGE_READONLY);
        // A hook entered before restoration still has its saved original.
        assert(Original(&object, 12) == native);
    }
    {
        HookScope scope;
        assert(Patch(&object, 12, hook));
        assert(VirtualProtect(table, 4096, PAGE_READWRITE, &previous));
        table[3] = reinterpret_cast<void*>(&ForeignHook);
        assert(VirtualProtect(table, 4096, previous, &previous));
        assert(!scope.Finish());
        assert(table[3] == reinterpret_cast<void*>(&ForeignHook));
    }
    assert(VirtualFree(table, 0, MEM_RELEASE));
    std::puts("legacyctl recording hooks: native tables restored, nested/concurrent captures preserved");
    return 0;
}
