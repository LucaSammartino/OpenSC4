// SPDX-License-Identifier: GPL-3.0-or-later
// persistence-manager sections 3, 8.1, 8.2; persistence section 5.
// Read-only projection, NOT a reconstruction of factory/cache history (PR9).
#include "FC4PersistShadow.h"
#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "GZServPtrs.h"
#include "cIGZPersistResourceManager.h"
#include "cIGZPersistResourceKeyList.h"
#include "cGZPersistResourceKey.h"
#include "fc4.h"
#include <algorithm>
#include <vector>

namespace {
struct Key { uint32_t type, group, instance; };
static_assert(sizeof(Key) == 12, "private snapshot transport");
}
extern "C" uint32_t fc4_pm_filter_snapshot(const Key*, size_t, uint32_t, Key*, size_t);

namespace fc4 {
namespace {
bool enabled = false;
uint32_t lines = 0, clean = 0;
constexpr uint32_t Limit = 1000000;

bool Capture(cIGZPersistResourceKeyList* list, uint32_t reported, std::vector<Key>& keys) {
    if (!list) return false;
    FC4_ACQUIRED(list, "persist snapshot key list");
    const uint32_t size = list->Size();
    const bool valid = size <= Limit && size == reported;
    if (valid) {
        keys.reserve(size);
        for (uint32_t i = 0; i < size; ++i) {
            const auto& k = list->GetKey(i);
            keys.push_back({k.type, k.group, k.instance});
        }
    }
    // A newly created list is ours to destroy, including an empty list.
    FC4_RELEASE_CREATED(list);
    return valid;
}

void Compare(const char* phase, int32_t julian) {
    if (!enabled) return;
    uint32_t differences = 0, queries = 0, compared = 0, factories = 0;
    std::vector<Key> input;
    {
        // SDK service helper balances its own acquired reference, within sample.
        cIGZPersistResourceManagerPtr manager;
        if (!manager) {
            ++differences;
        } else {
            factories = manager->GetFactoryCount(); // diagnostic only, no replay claim
            cIGZPersistResourceKeyList* list = nullptr;
            const uint32_t reported = manager->GetResourceList(&list, nullptr);
            if (!Capture(list, reported, input)) {
                ++differences;
            } else {
                // Every observed type, plus an independent normally absent type.
                // No cap silently omits types or keys. Preserve duplicate keys.
                std::vector<uint32_t> types{0xF4C4E008};
                for (const auto& k : input) types.push_back(k.type);
                std::sort(types.begin(), types.end());
                types.erase(std::unique(types.begin(), types.end()), types.end());
                std::vector<Key> expected(input.size());
                for (uint32_t type : types) {
                    const uint32_t n = fc4_pm_filter_snapshot(input.data(), input.size(), type,
                        expected.data(), expected.size());
                    std::vector<Key> native;
                    list = nullptr;
                    const uint32_t count = manager->GetResourceListForType(&list, type);
                    ++queries;
                    if (!Capture(list, count, native) || n > input.size() || n != native.size()) {
                        ++differences;
                        continue;
                    }
                    for (uint32_t i = 0; i < n; ++i) {
                        const auto& a = native[i]; const auto& b = expected[i];
                        differences += a.type != b.type || a.group != b.group || a.instance != b.instance;
                        ++compared;
                    }
                }
            }
        }
    } // No game reference is retained between samples or through DLL unload.
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    Log("persist shadow %04d-%02d-%02d %s: scope=cache-type-projection keys=%u queries=%u compared=%u factories_observed=%u resyncs=0; mismatches=%u",
        date.year, date.month, date.day_of_month, phase, static_cast<unsigned>(input.size()),
        queries, compared, factories, differences);
    ++lines;
    clean += differences == 0;
}
}
void PersistShadowCityLoaded(cISC4City* city, int32_t julian) {
    enabled = SettingInt("persist_shadow", nullptr, 0) != 0;
    lines = clean = 0;
    if (enabled) Log("persist: ON; read-only cache type projection only; PR9 registry/refcount replay unavailable; SCR11 buffer replay unavailable; no whole-owner coverage credit");
    // PublishedJulian is not initialized yet during the city-load message.
    // Match the house shadows: this getter is borrowed and read-only.
    if (enabled && city) {
        auto* sim = AsGame(city->GetSimulator());
        if (sim) julian = sim->GetSimDateNumber();
    }
    Compare("load", julian);
}
void PersistShadowMonth(int32_t julian) { Compare("month", julian); }
void PersistShadowCityShutdown() {
    if (enabled) Log("persist shadow: city shutdown: %u lines, %u with mismatches=0; resyncs=0", lines, clean);
    enabled = false;
}
}
