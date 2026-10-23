// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4NetOccShadow.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4WriteCapture.h"
#include "cISC4City.h"
#include "cISC4OccupantManager.h"
#include "cIGZSerializable.h"
#include "fc4.h"
#include <cinttypes>
#include <cstdio>

// Private static-link verification transport, absent from the DLL export table.
struct NetOccResult {
    uint32_t status, fixups, first, expectedLen, actualLen, expected, actual;
};
extern "C" bool fc4_netocc_shadow_compare(uint32_t, const uint8_t*, size_t, NetOccResult*);

namespace fc4 {
namespace {
cISC4City* g_city = nullptr; // borrowed only while the city is up
bool g_enabled = false;
uint64_t g_lines = 0, g_clean = 0;
const uint32_t kClasses[] = {0xC9C05C6E, 0x49C1A034, 0x49CC1BCD, 0x8A4BD52B,
    0x4A10AFC2, 0x49C05B9F, 0xCA16374F, 0x8A139184, 0xEA0A5ACF, 0xCA15ED6B,
    0x09C05C6A, 0xC9C05C5D, 0x49B6D69B};

void Compare(const char* event, int32_t julian) {
    if (!g_enabled || !g_city) return;
    uint32_t records = 0, mismatches = 0, fixups = 0;
    uint64_t bytes = 0;
    uint32_t kinds[13]{};
    uint32_t badKinds[13]{};
    // Occupant-manager §13 writes every indexed occupant reference. Reuse its
    // capture's deduplicated object table; no private layout or guessed iterator.
    WriteCapture manager;
    if (!manager.Capture(g_city->GetOccupantManager())) {
        ++mismatches;
    } else {
        for (uint32_t id = 1; manager.Object(id) != nullptr; ++id) {
            const uint32_t cls = manager.ClassOf(id);
            size_t kind = 0;
            while (kind < 13 && kClasses[kind] != cls) ++kind;
            if (kind == 13) continue;
            ++records;
            ++kinds[kind];
            WriteCapture record(true);
            auto* object = manager.SerializableAt(id); // borrowed during capture
            NetOccResult result{};
            const bool captured = record.Capture(object);
            const auto& b = record.Bytes();
            const bool compared = captured && fc4_netocc_shadow_compare(cls, b.data(), b.size(), &result);
            fixups += result.fixups;
            bytes += b.size();
            if (!compared || result.status != 0) {
                ++mismatches;
                if (++badKinds[kind] <= 2) Log("netocc difference: class=%08X object=%u capture=%u status=%u at=%u expected=%02X actual=%02X lengths=%u/%u",
                    cls, id, captured ? 1u : 0u, compared ? result.status : 4u, result.first,
                    result.expected, result.actual, result.expectedLen, result.actualLen);
            }
        }
    }
    // An empty sample must never be credited as an exact shadow.
    if (records == 0) ++mismatches;
    fc4_date date{};
    char dateText[16] = "?";
    if (julian > 0) {
        fc4_date_from_julian(julian, &date);
        _snprintf_s(dateText, sizeof(dateText), _TRUNCATE, "%04d-%02d-%02d", date.year, date.month, date.day_of_month);
    }
    Log("netocc shadow %s %s: mismatches=%u fixups=%u records=%u bytes=%" PRIu64
        " network=%u prebuilt=%u bridge=%u tunnel=%u underground=%u pipe=%u subway=%u connection=%u pipe_connection=%u subway_connection=%u pole=%u line=%u pylon=%u",
        event, dateText, mismatches, fixups, records, bytes,
        kinds[0], kinds[1], kinds[2], kinds[3], kinds[4], kinds[5], kinds[6], kinds[7], kinds[8], kinds[9], kinds[10], kinds[11], kinds[12]);
    ++g_lines;
    if (mismatches == 0) ++g_clean;
}
}
void NetOccShadowCityLoaded(cISC4City* city, int32_t julian) {
    g_enabled = SettingInt("netocc_shadow", nullptr, 0) != 0;
    // An absent/off ini key keeps no city pointer and performs no capture.
    g_city = nullptr;
    if (!g_enabled) return;
    g_city = city;
    g_lines = g_clean = 0;
    Compare("load", julian);
}
void NetOccShadowMonth(int32_t julian) { Compare("month", julian); }
void NetOccShadowCityShutdown() {
    if (g_enabled) Log("netocc shadow summary: clean=%" PRIu64 "/%" PRIu64, g_clean, g_lines);
    g_city = nullptr;
    g_enabled = false;
}
}
