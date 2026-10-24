// SPDX-License-Identifier: GPL-3.0-or-later
// framework-gz-services §1/§3. GZ11: no registry enumeration or mutation observer.
#include "FC4GZMsgShadow.h"
#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cIGZFrameWork.h"
#include "cIGZUnknown.h"
#include "cRZCOMDllDirector.h"
#include "cISC4City.h"
#include "fc4.h"
#include <vector>

namespace {
struct Entry { uint32_t id, token, found; };
static_assert(sizeof(Entry) == 12, "Rust transport layout");
extern "C" uint32_t fc4_gzmsg_shadow_compare(const Entry*, const Entry*, uint32_t);
bool enabled = false, active = false;
uint32_t lines = 0, clean = 0;
// Numeric identities only. Never retain a game interface past Capture.
std::vector<Entry> baseline;
constexpr uint32_t service = 0x64FA8434, iid = 0x652294BC;

bool Capture(std::vector<Entry>& entries) {
    auto* fw = RZGetFrameWork(); // borrowed
    cIGZUnknown* manager = nullptr;
    const bool found = fw && fw->GetSystemService(service, iid, reinterpret_cast<void**>(&manager));
    if (!found || !manager) {
        fc4::Log("gzmsg: native manager unavailable framework=%u lookup=%u pointer=%u service=%08X iid=%08X (GZ12)",
            fw ? 1u : 0u, found ? 1u : 0u, manager ? 1u : 0u, service, iid);
        return false;
    }
    FC4_ACQUIRED(manager, "GetSystemService");
    // GZ11 diagnostic domain, not a claimed enumeration of the 32-bit ID space.
    for (uint32_t n = 0; n < 259; ++n) {
        const uint32_t id = n < 256 ? n : n == 256 ? service : n == 257 ? iid : UINT32_MAX;
        cIGZUnknown* queue = nullptr;
        // §3 slot 3: bool GetQueue(uint32_t, queue**), AddRef'd on success.
        using Lookup = bool (__thiscall*)(void*, uint32_t, cIGZUnknown**);
        auto lookup = reinterpret_cast<Lookup>((*reinterpret_cast<void***>(manager))[3]);
        const bool queued = lookup(manager, id, &queue);
        entries.push_back({ id, static_cast<uint32_t>(reinterpret_cast<uintptr_t>(queue)), queued ? 1u : 0u });
        if (queue) {
            FC4_ACQUIRED(queue, "GetQueue");
            FC4_RELEASE(queue);
        }
    }
    FC4_RELEASE(manager);
    return true;
}

void Compare(const char* phase, int32_t julian) {
    if (!enabled || !active) return;
    std::vector<Entry> sample;
    const bool captured = Capture(sample);
    uint32_t mismatches = 1;
    if (captured && !baseline.empty() && baseline.size() == sample.size())
        mismatches = fc4_gzmsg_shadow_compare(baseline.data(), sample.data(), static_cast<uint32_t>(sample.size()));
    uint32_t present = 0;
    for (const auto& entry : sample) present += entry.found != 0;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    fc4::Log("gzmsg shadow %04d-%02d-%02d %s: scope=bounded-lookup probes=%u present=%u captured=%u baseline=%u resyncs=0; mismatches=%u",
        date.year, date.month, date.day_of_month, phase, static_cast<uint32_t>(sample.size()), present,
        captured ? 1u : 0u, static_cast<uint32_t>(baseline.size()), mismatches);
    ++lines;
    if (mismatches == 0) ++clean;
}
}

namespace fc4 {
void GZMsgShadowCityLoaded(cISC4City* city, int32_t julian) {
    enabled = SettingInt("gzmsg_shadow", nullptr, 0) != 0;
    active = city != nullptr;
    lines = clean = 0;
    baseline.clear();
    if (!enabled || !active) return;
    Log("gzmsg: ON; bounded immutable lookup baseline only; GZ11 full registry/queue/posts/notifications unavailable; no whole-owner credit");
    if (auto* sim = AsGame(city->GetSimulator())) julian = sim->GetSimDateNumber();
    Capture(baseline); // One load seed, never replaced by a later native sample.
    Compare("load", julian);
}
void GZMsgShadowMonth(int32_t julian) { Compare("month", julian); }
void GZMsgShadowCityShutdown() {
    if (enabled && active)
        Log("gzmsg shadow: city shutdown: scope=bounded-lookup %u lines, %u with mismatches=0; resyncs=0", lines, clean);
    baseline.clear();
    enabled = active = false;
}
}
