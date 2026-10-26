// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <windows.h>
#include <cstdint>
#include <map>
#include <mutex>

namespace fc4::legacyrecord {
struct SavedHook { void* original; void* replacement; bool scoped; };
inline std::mutex tableMutex;
inline std::map<std::pair<void**, uint32_t>, SavedHook> originals;
inline unsigned captures = 0;

// Recording must leave shared game interfaces native between captures. In
// particular, DBPF validates every COM vtable entry against the game image.
struct HookScope {
    bool open = true;
    HookScope() {
        std::lock_guard<std::mutex> lock(tableMutex);
        ++captures;
    }
    HookScope(const HookScope&) = delete;
    HookScope& operator=(const HookScope&) = delete;
    bool Finish() {
        std::lock_guard<std::mutex> lock(tableMutex);
        if (!open) return true;
        open = false;
        if (--captures) return true;
        bool ok = true;
        for (const auto& entry : originals) {
            if (!entry.second.scoped) continue;
            auto** slot = entry.first.first + entry.first.second / 4;
            if (*slot == entry.second.original) continue;
            // Preserve another observer's replacement rather than overwrite it.
            if (*slot != entry.second.replacement) { ok = false; continue; }
            DWORD previous = 0;
            if (!VirtualProtect(slot, sizeof(void*), PAGE_READWRITE, &previous)) { ok = false; continue; }
            *slot = entry.second.original;
            DWORD ignored = 0;
            if (!VirtualProtect(slot, sizeof(void*), previous, &ignored)) ok = false;
        }
        return ok;
    }
    ~HookScope() { Finish(); }
};

inline bool Patch(void* object, uint32_t offset, void* hook, bool scoped = false) {
    if (!object) return true;
    auto** table = *static_cast<void***>(object);
    std::lock_guard<std::mutex> lock(tableMutex);
    if (!captures) return false;
    const auto key = std::make_pair(table, offset);
    const auto found = originals.find(key);
    if (found != originals.end()) {
        if (found->second.replacement != hook || found->second.scoped != scoped) return false;
        if (table[offset / 4] == hook) return true;
        if (table[offset / 4] != found->second.original) return false;
    }
    DWORD previous = 0;
    if (!VirtualProtect(table + offset / 4, sizeof(void*), PAGE_READWRITE, &previous)) return false;
    if (found == originals.end()) originals.emplace(key, SavedHook{table[offset / 4], hook, scoped});
    table[offset / 4] = hook;
    DWORD ignored = 0;
    return VirtualProtect(table + offset / 4, sizeof(void*), previous, &ignored) != 0;
}
inline void* Original(void* self, uint32_t offset) {
    std::lock_guard<std::mutex> lock(tableMutex);
    // Keep the original available to a hook already entered on another thread.
    return originals.at({*static_cast<void***>(self), offset}).original;
}
} // namespace fc4::legacyrecord
