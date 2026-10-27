// SPDX-License-Identifier: GPL-3.0-or-later
// Scoped interface observers for network-manager §7.6.1 / §8.5 NM19.
// Only vtable data is changed; original functions and arguments are forwarded.
#pragma once
#include <windows.h>
#include <map>
#include <vector>
#include <intrin.h>
#include "SC4Vector.h"
#include "cISC4NetworkTool.h"
#include "cISC4OccupantManager.h"
#include "cISC4NetworkLotConfiguration.h"
#include "FC4NetMgrTerrainObservation.h"

namespace nmtrace {
struct Patch { void** entry; void* original; void* replacement; };
inline std::vector<Patch> patches;
inline std::vector<Patch> lotPatches;
inline bool active = false, complete = false;
inline unsigned toolDepth = 0, replaceDepth = 0;
inline uint64_t toolCall = 0, nextToolCall = 0;
inline uint32_t sequence = 0;
inline std::vector<std::vector<uint64_t>> seeds;
inline fc4::netmgr::DestructionSource source = fc4::netmgr::DestructionSource::Unknown;
inline std::set<uint64_t> refitSeeds, refitRemoved;
inline std::map<uint64_t, fc4::netmgr::Removal> removals;
inline std::vector<fc4::netmgr::Insertion> insertions;

inline void* Original(void* self, size_t slot) {
    void** entry = *static_cast<void***>(self) + slot;
    for (const auto& p : patches) if (p.entry == entry) return p.original;
    for (const auto& p : lotPatches) if (p.entry == entry) return p.original;
    return nullptr;
}

inline bool Install(void* object, size_t slot, uintptr_t expected, void* replacement,
                    std::vector<Patch>& saved = patches) {
    if (!object || !fc4::compat::IdentifiedVersion()) return false;
    void** entry = *static_cast<void***>(object) + slot;
    for (const auto& p : saved) if (p.entry == entry) return *entry == replacement;
    if (reinterpret_cast<uintptr_t>(*entry) != expected) return false;
    DWORD old = 0;
    if (!VirtualProtect(entry, sizeof(void*), PAGE_READWRITE, &old)) return false;
    saved.push_back({entry, *entry, replacement});
    *entry = replacement;
    DWORD ignored = 0;
    return VirtualProtect(entry, sizeof(void*), old, &ignored) != 0;
}

inline void Stop(std::vector<Patch>& saved = patches) {
    active = false;
    for (auto p = saved.rbegin(); p != saved.rend(); ++p) {
        DWORD old = 0;
        if (*p->entry != p->replacement || !VirtualProtect(p->entry, sizeof(void*), PAGE_READWRITE, &old)) {
            complete = false;
            continue;
        }
        *p->entry = p->original;
        DWORD ignored = 0;
        if (!VirtualProtect(p->entry, sizeof(void*), old, &ignored)) complete = false;
    }
    // Retain originals if restoring an observer failed.
    bool restored = true;
    for (const auto& p : saved) if (*p.entry == p.replacement) restored = false;
    if (restored) saved.clear();
}

inline uint64_t Describe(cISC4Occupant* occupant, const char* event, uint32_t data = 0) {
    void* p = nullptr;
    if (!occupant || !occupant->QueryInterface(0xA821EF94, &p) || !p) return 0;
    auto* n = static_cast<cISC4NetworkOccupant*>(p);
    FC4_ACQUIRED(n, "QueryInterface");
    uint32_t x = 0, z = 0;
    n->GetOccupiedCell(x, z);
    const uint64_t id = fc4::TokenOf(n);
    const uint32_t serial = ++sequence;
    fc4::Log("netmgr terrain trace %s: id=%p cell=%u,%u flags=%08X tool_depth=%u replace_depth=%u call=%llu seq=%u data=%u",
             event, p, x, z, n->GetNetworkFlag(), toolDepth, replaceDepth, toolCall, serial, data);
    if (std::strcmp(event, "remove") == 0) removals.emplace(id, fc4::netmgr::Removal{toolCall, serial,
                                                     static_cast<int32_t>(x), static_cast<int32_t>(z)});
    if (std::strcmp(event, "remove") == 0 && source == fc4::netmgr::DestructionSource::UndergroundRefit &&
        refitSeeds.count(id)) refitRemoved.insert(id);
    if (std::strcmp(event, "insert") == 0) insertions.push_back({id, toolCall, serial, data,
                                                     static_cast<int32_t>(x), static_cast<int32_t>(z)});
    FC4_RELEASE(n);
    return id;
}

inline void __fastcall Destroy(void* self, void*, const SC4Vector<cISC4NetworkOccupant*>& list) {
    const auto original = reinterpret_cast<void(__thiscall*)(void*, const SC4Vector<cISC4NetworkOccupant*>&)>(Original(self, 50));
    if (active) {
        std::vector<uint64_t> ids;
        for (auto* n : list) {
            ids.push_back(fc4::TokenOf(n));
            if (source == fc4::netmgr::DestructionSource::UndergroundRefit) refitSeeds.insert(fc4::TokenOf(n));
            Describe(n->AsOccupant(), "seed");
        }
        if (source == fc4::netmgr::DestructionSource::TerrainFilter) seeds.push_back(std::move(ids));
        else if (source == fc4::netmgr::DestructionSource::Unknown) complete = false;
    }
    const uint64_t previous = toolCall;
    toolCall = ++nextToolCall;
    ++toolDepth;
    original(self, list);
    --toolDepth;
    toolCall = previous;
}

inline void __fastcall ManagerDestroy(void* self, void*, const void* list) {
    // ABI slot 6 is void, despite the SDK's bool declaration. The list
    // reference is borrowed and forwarded without copying its B-list nodes.
    const auto original = reinterpret_cast<void(__thiscall*)(void*, const void*)>(Original(self, 6));
    const auto previous = source;
    const uintptr_t caller = reinterpret_cast<uintptr_t>(_ReturnAddress());
    source = fc4::netmgr::Source(caller);
    if (active) fc4::Log("netmgr terrain trace destruction: caller=%08X source=%u", static_cast<uint32_t>(caller),
                        static_cast<unsigned>(source));
    original(self, list);
    source = previous;
}

inline cISC4NetworkTool* __fastcall GetTool(void* self, void*, int32_t type, bool unique) {
    const auto original = reinterpret_cast<cISC4NetworkTool*(__thiscall*)(void*, int32_t, bool)>(Original(self, 5));
    auto* tool = original(self, type, unique);
    if (active && !Install(tool, 50, 0x0062A5C0, reinterpret_cast<void*>(&Destroy))) complete = false;
    return tool;
}

inline bool __fastcall Replace(void* self, void*, cISC4NetworkTool* tool, cISC4Occupant* occupant, bool& replaced) {
    const auto original = reinterpret_cast<bool(__thiscall*)(void*, cISC4NetworkTool*, cISC4Occupant*, bool&)>(Original(self, 8));
    ++replaceDepth;
    if (active) Describe(occupant, "replace-entry");
    const bool result = original(self, tool, occupant, replaced);
    --replaceDepth;
    return result;
}

inline bool __fastcall Remove(void* self, void*, cISC4Occupant* occupant, bool shutdown, uint32_t data) {
    const auto original = reinterpret_cast<bool(__thiscall*)(void*, cISC4Occupant*, bool, uint32_t)>(Original(self, 21));
    if (active) Describe(occupant, "remove", data);
    const bool result = original(self, occupant, shutdown, data);
    if (active && !result) complete = false;
    return result;
}

inline bool __fastcall Insert(void* self, void*, cISC4Occupant* occupant, uint32_t data) {
    const auto original = reinterpret_cast<bool(__thiscall*)(void*, cISC4Occupant*, uint32_t)>(Original(self, 20));
    const bool result = original(self, occupant, data);
    if (active && result) Describe(occupant, "insert", data);
    return result;
}

inline void Start(cISC4City* city, uint32_t type, const uint32_t* data) {
    seeds.clear();
    removals.clear();
    insertions.clear();
    refitSeeds.clear();
    refitRemoved.clear();
    source = fc4::netmgr::DestructionSource::Unknown;
    active = true;
    toolDepth = replaceDepth = 0;
    toolCall = nextToolCall = 0;
    sequence = 0;
    complete = Install(city->GetNetworkManager(), 5, 0x006108D0, reinterpret_cast<void*>(&GetTool));
    complete = Install(city->GetNetworkManager(), 6, 0x00612110, reinterpret_cast<void*>(&ManagerDestroy)) && complete;
    complete = Install(city->GetNetworkManager(), 8, 0x006110B0, reinterpret_cast<void*>(&Replace)) && complete;
    complete = Install(city->GetOccupantManager(), 20, 0x0049EF60, reinterpret_cast<void*>(&Insert)) && complete;
    complete = Install(city->GetOccupantManager(), 21, 0x0049F240, reinterpret_cast<void*>(&Remove)) && complete;
    fc4::Log("netmgr terrain trace begin: msg=%08X data=%08X/%08X/%08X/%08X hooks=%u",
             type, data[0], data[1], data[2], data[3], complete ? 1u : 0u);
}
} // namespace nmtrace
