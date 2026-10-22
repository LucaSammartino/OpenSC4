// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "FC4Compat.h"
#include <windows.h>
#include <cstdint>

namespace fc4 { namespace compat {
// Published commercial-industrial sections 1, 2.2 and 9, GOG 1.1.641.
// No scanning, executable-file reads, table writes or private entrypoints.
inline bool IndustrialImageRange(uintptr_t p, size_t n, bool code) {
    const auto* v = IdentifiedVersion();
    MEMORY_BASIC_INFORMATION m{};
    if (!v || v->timeDateStamp != 0x4C12BFB7 || v->sizeOfImage != 0x7C8000 ||
        v->imageBase != 0x400000 || p < v->imageBase ||
        p + n < p || p + n > uintptr_t(v->imageBase) + v->sizeOfImage ||
        VirtualQuery(reinterpret_cast<void*>(p), &m, sizeof(m)) != sizeof(m) ||
        m.State != MEM_COMMIT || m.Type != MEM_IMAGE ||
        reinterpret_cast<uintptr_t>(m.AllocationBase) != v->imageBase ||
        (m.Protect & (PAGE_GUARD | PAGE_NOACCESS)) ||
        p + n > reinterpret_cast<uintptr_t>(m.BaseAddress) + m.RegionSize) return false;
    return !code || (m.Protect & (PAGE_EXECUTE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY));
}

inline bool IndustrialOriginal(const void* primary) {
    if (!primary || !IdentifiedVersion()) return false;
    __try {
        const auto* views = static_cast<const uintptr_t*>(primary);
        const uintptr_t tables[] = {0xAB1530, 0xAB1520, 0xAB1508};
        const unsigned slots[] = {5, 4, 6};
        for (unsigned i = 0; i < 3; ++i) {
            if (views[i] != tables[i] || !IndustrialImageRange(tables[i], slots[i] * 4, false)) return false;
            const auto* t = reinterpret_cast<const uintptr_t*>(tables[i]);
            for (unsigned j = 0; j < slots[i]; ++j)
                if (!IndustrialImageRange(t[j], 1, true)) return false;
        }
        const auto* t = reinterpret_cast<const uintptr_t*>(tables[0]);
        const auto* m = reinterpret_cast<const uintptr_t*>(tables[1]);
        const auto* s = reinterpret_cast<const uintptr_t*>(tables[2]);
        return t[0] == 0x6B8970 && t[1] == 0x4F7BA0 && t[2] == 0x6D5140 &&
            t[3] == 0x6B89F0 && t[4] == 0x6B8A10 && m[3] == 0x6B8AA0 &&
            s[3] == 0x42E440 && s[4] == 0x6B8A70 && s[5] == 0x6B8960;
    } __except (EXCEPTION_EXECUTE_HANDLER) { return false; }
}

inline bool IndustrialInitialized(const void* p, bool* initialized) {
    if (!IndustrialOriginal(p)) return false;
    __try {
        *initialized = *(static_cast<const uint8_t*>(p) + 0x0C) != 0;
        return true;
    } __except (EXCEPTION_EXECUTE_HANDLER) { return false; }
}
}}
