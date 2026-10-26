// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <windows.h>

namespace fc4 {
// A read-only observer's forwarding slot. Never owns the observed object.
// Installation/removal happens on the simulation thread; the table is static.
class BoundarySlot {
public:
    void* original = nullptr;
    bool Install(void* object, unsigned slot, void* forwarder) {
        if (!object) return false;
        void** entry = *static_cast<void***>(object) + slot;
        if (entry_ == entry) return true;
        Remove();
        DWORD old = 0;
        if (!VirtualProtect(entry, sizeof(void*), PAGE_READWRITE, &old)) return false;
        original = *entry;
        forwarder_ = forwarder;
        entry_ = entry;
        *entry = forwarder;
        DWORD ignored = 0;
        VirtualProtect(entry, sizeof(void*), old, &ignored);
        return true;
    }
    void Remove() {
        if (!entry_) return;
        DWORD old = 0;
        if (*entry_ == forwarder_ && VirtualProtect(entry_, sizeof(void*), PAGE_READWRITE, &old)) {
            *entry_ = original;
            DWORD ignored = 0;
            VirtualProtect(entry_, sizeof(void*), old, &ignored);
        }
        entry_ = nullptr;
    }
private:
    void** entry_ = nullptr;
    void* forwarder_ = nullptr;
};
}
