// SPDX-License-Identifier: GPL-3.0-or-later
// sc-resources §§5-6 adapter reads only. SCR8 blocks the remaining owners.
// Every forwarder calls the ORIGINAL exactly once, returning its result intact.
#include "FC4SCResShadow.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4Interfaces.h"
#include "compat/FC4Compat.h"
#include "cIGZPersistDBSerialRecord.h"
#include "cISC4City.h"
#include "fc4.h"
#include <windows.h>
#include <atomic>
#include <cinttypes>
#include <cstring>
#include <vector>

namespace {
struct Observation {
    uint32_t text, slot, counter, total, afterCounter, status;
    uint8_t before[12], after[12];
    const uint8_t* bytes;
    size_t length;
};
static_assert(offsetof(Observation, bytes) == 48, "Rust repr(C) observation alignment");
extern "C" uint32_t fc4_scres_shadow_compare(const Observation*);
struct Patch { void** entry = nullptr; void* original = nullptr; void* forward = nullptr; };
Patch g_patches[2][14];
std::atomic<uint64_t> g_calls{0}, g_bytes{0}, g_bad{0}, g_refused{0};
std::atomic<uint64_t> g_binary{0}, g_text{0};
bool g_enabled = false;
uint64_t g_lines = 0, g_clean = 0;
thread_local bool g_inside = false;

uint32_t Word(const void* object, size_t offset) {
    uint32_t value;
    std::memcpy(&value, static_cast<const uint8_t*>(object) + offset, sizeof(value));
    return value;
}
constexpr size_t Width(unsigned slot) {
    return slot == 2 ? 12 : slot == 1 || slot == 4 || slot == 6 || slot == 7 ? 8 :
        slot >= 12 ? 1 : slot >= 10 ? 2 : 4;
}

template<unsigned Mode, unsigned Slot>
bool __fastcall Read(void* self, void*, void* output) {
    using Original = bool (__thiscall*)(void*, void*);
    const auto original = reinterpret_cast<Original>(g_patches[Mode][Slot].original);
    if (g_inside || !output) return original(self, output);
    g_inside = true;
    struct Exit { ~Exit() { g_inside = false; } } exit;
    Observation o{};
    o.text = Mode;
    o.slot = Slot;
    o.counter = Word(self, 8);
    o.total = Word(self, 12);
    std::memcpy(o.before, output, Width(Slot));
    // §6.1's base-record pointer is borrowed for this call; no reference is
    // acquired or released. No native owner layout beyond this published state.
    cIGZPersistDBSerialRecord* record = nullptr;
    std::memcpy(&record, static_cast<uint8_t*>(self) + 4, sizeof(record));
    const uint32_t start = record->GetPosition();
    const bool status = original(self, output);
    o.status = status ? 1u : 0u;
    o.afterCounter = Word(self, 8);
    std::memcpy(o.after, output, Width(Slot));
    const uint32_t end = record->GetPosition();
    // Literal SDK read/seek capture: only the bytes actually consumed by the
    // original are copied, after it returns. Restore its post-call cursor even
    // when capture fails. SCR8 requests a non-mutating raw-byte accessor.
    bool captured = end >= start && end - start <= 1024 * 1024;
    std::vector<uint8_t> bytes;
    if (captured && end != start) {
        bytes.resize(end - start);
        const bool sought = record->SeekAbsolute(start);
        const bool read = sought && record->GetFieldVoid(bytes.data(), end - start);
        const bool restored = record->SeekAbsolute(end);
        captured = sought && read && restored;
    }
    if (!captured) {
        ++g_refused;
        return status;
    }
    o.bytes = bytes.data();
    o.length = bytes.size();
    const uint32_t difference = fc4_scres_shadow_compare(&o);
    ++g_calls;
    if (Mode) ++g_text; else ++g_binary;
    g_bytes += bytes.size();
    if (difference && ++g_bad <= 8) fc4::Log("scres difference: mode=%u slot=%u status=%u counter=%u/%u total=%u bytes=%u channels=%u",
        Mode, Slot, o.status, o.counter, o.afterCounter, o.total, end - start, difference);
    return status;
}

template<unsigned Mode>
void* Forward(unsigned slot) {
    switch (slot) {
        case 1: return reinterpret_cast<void*>(&Read<Mode, 1>);
        case 2: return reinterpret_cast<void*>(&Read<Mode, 2>);
        case 4: return reinterpret_cast<void*>(&Read<Mode, 4>);
        case 5: return reinterpret_cast<void*>(&Read<Mode, 5>);
        case 6: return reinterpret_cast<void*>(&Read<Mode, 6>);
        case 7: return reinterpret_cast<void*>(&Read<Mode, 7>);
        case 8: return reinterpret_cast<void*>(&Read<Mode, 8>);
        case 9: return reinterpret_cast<void*>(&Read<Mode, 9>);
        case 10: return reinterpret_cast<void*>(&Read<Mode, 10>);
        case 11: return reinterpret_cast<void*>(&Read<Mode, 11>);
        case 12: return reinterpret_cast<void*>(&Read<Mode, 12>);
        case 13: return reinterpret_cast<void*>(&Read<Mode, 13>);
        default: return nullptr;
    }
}
// sc-resources §§5.2/6.1 and docs/abi/vtables.tsv (binary attribution is
// misleading in that table; the behavior spec explicitly settles the views).
const uintptr_t kTables[2] = {0x00AD9978, 0x00AD99E8};
const uintptr_t kFunctions[2][14] = {
    {0, 0x0097EBA0, 0x0097ED17, 0, 0x0097ECC5, 0x0097ECAC, 0x0097EC93,
     0x0097EC7A, 0x0097EC61, 0x0097EC48, 0x0097EC2F, 0x0097EC16, 0x0097EBFE, 0x0097EBE6},
    {0, 0x00980CCE, 0x0097EE9C, 0, 0x00980C59, 0x00980BEA, 0x00980B80,
     0x0097EC7A, 0x00980DC7, 0x00980DB9, 0x00980D9C, 0x00980D9C, 0x00980D81, 0x00980D81}
};
bool Replace(Patch& patch, void* value) {
    DWORD before = 0;
    if (!VirtualProtect(patch.entry, sizeof(void*), PAGE_READWRITE, &before)) return false;
    InterlockedExchangePointer(reinterpret_cast<void* volatile*>(patch.entry), value);
    DWORD ignored = 0;
    VirtualProtect(patch.entry, sizeof(void*), before, &ignored);
    return true;
}
}

namespace fc4 {
void SCResShadowStart() {
    if (g_enabled || SettingInt("scres_shadow", nullptr, 0) == 0) return;
    g_enabled = true;
    if (!compat::IdentifiedVersion()) {
        ++g_refused;
        Log("scres: refused: executable not identified");
        return;
    }
    for (unsigned mode = 0; mode != 2; ++mode) {
        for (unsigned slot = 1; slot != 14; ++slot) {
            if (slot == 3) continue; // SCR8: native string argument view unsettled.
            auto& patch = g_patches[mode][slot];
            patch.entry = reinterpret_cast<void**>(kTables[mode] + 4 * slot);
            patch.original = reinterpret_cast<void*>(kFunctions[mode][slot]);
            patch.forward = mode ? Forward<1>(slot) : Forward<0>(slot);
            if (*patch.entry != patch.original || !Replace(patch, patch.forward)) {
                ++g_refused;
                Log("scres: refused mode=%u slot=%u (entry mismatch or protection failure)", mode, slot);
                patch.entry = nullptr;
            }
        }
    }
    Log("scres: ON: record-only binary/text numeric, descriptor and key reads; PARTIAL coverage; strings/parsers/images/writers blocked by SCR8");
}
void SCResShadowLine(const char* event, int32_t julian, cISC4City* city) {
    if (!g_enabled) return;
    // The observer's published date is not initialized in the city-load notice.
    // Use the city's borrowed simulator, as the clock/history shadows do.
    if (julian <= 0) {
        auto* sim = city ? AsGame(city->GetSimulator()) : nullptr;
        if (sim) julian = sim->GetSimDateNumber();
    }
    if (julian <= 0) {
        ++g_refused;
        Log("scres: %s date unavailable; observation refused", event);
        return;
    }
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    // Never credit an empty observation set as an exact differential.
    const uint64_t mismatches = g_bad.load() + g_refused.load() + (g_calls.load() == 0 ? 1u : 0u);
    Log("scres shadow %04d-%02d-%02d %s: partial=adapter_reads calls=%" PRIu64
        " binary=%" PRIu64 " text=%" PRIu64 " bytes=%" PRIu64 " refused=%" PRIu64 "; mismatches=%" PRIu64,
        date.year, date.month, date.day_of_month, event, g_calls.load(), g_binary.load(), g_text.load(),
        g_bytes.load(), g_refused.load(), mismatches);
    ++g_lines;
    if (!mismatches) ++g_clean;
}
void SCResShadowStop() {
    if (!g_enabled) return;
    for (auto& mode : g_patches) for (auto& patch : mode) {
        if (patch.entry && *patch.entry == patch.forward && !Replace(patch, patch.original))
            Log("scres: failed to restore adapter entry");
    }
    Log("scres shadow summary: clean=%" PRIu64 "/%" PRIu64 " partial=adapter_reads", g_clean, g_lines);
    g_enabled = false;
}
}
