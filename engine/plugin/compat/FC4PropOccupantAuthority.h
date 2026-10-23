// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "FC4Compat.h"
#include <windows.h>
#include <cstdint>

namespace fc4 { namespace compat {
struct PropAuthority {
    uintptr_t table = 0, target = 0;
    HMODULE module = nullptr;
    const char* reason = "unexamined";
};
// Only published interface identity and dispatch-pointer/module provenance.
// No object fields, instructions, constructors, pools, or private calls.
inline bool OriginalPlainProp(const void* object, unsigned slot, PropAuthority& out) {
    out = {};
    out.reason = "unsupported-version";
    const auto* v = IdentifiedVersion();
    if (!v || v->timeDateStamp != 0x4c12bfb7 || v->sizeOfImage != 0x7c8000 || v->imageBase != 0x400000)
        return false;
    out.reason = "null-or-slot";
    if (!object || slot >= 24) return false;
    __try {
        out.table = *static_cast<const uintptr_t*>(object);
        out.reason = "primary-table-not-plain-original";
        // prop-occupant 1/4.1: exact primary table, behind the existing gate.
        if (out.table != 0x00aa4900) return false;
        out.target = reinterpret_cast<const uintptr_t*>(out.table)[slot];
        out.reason = "dispatch-module-not-game-image";
        MEMORY_BASIC_INFORMATION m{};
        if (!GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                reinterpret_cast<LPCSTR>(out.target), &out.module) || out.module != GetModuleHandleW(nullptr) ||
            reinterpret_cast<uintptr_t>(out.module) != v->imageBase ||
            VirtualQuery(reinterpret_cast<void*>(out.target), &m, sizeof(m)) != sizeof(m) ||
            m.Type != MEM_IMAGE || m.State != MEM_COMMIT || m.AllocationBase != out.module ||
            (m.Protect & (PAGE_GUARD | PAGE_NOACCESS)) ||
            !(m.Protect & (PAGE_EXECUTE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY))) return false;
        out.reason = "original-plain-primary-game-module";
        return true;
    } __except (EXCEPTION_EXECUTE_HANDLER) {
        out.reason = "unreadable-interface-identity";
        return false;
    }
}
} }
