// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

#include "FC4Pacing.h"

#include "FC4Config.h"
#include "FC4Log.h"
#include "fc4.h"

#include "cIGZMessage2.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cctype>
#include <cstring>
#include <string>
#include <vector>

namespace {

// The conformance shadows (FC4Config.h; 26 keys); any of them on makes `original` the
// default when `pacing` is unset.
const char* const kShadowKeys[] = {
    "budget_shadow",      "demand_shadow",     "pollution_shadow",   "protection_shadow", "crime_shadow",
    "aura_shadow",        "flora_shadow",      "landfill_shadow",    "power_shadow",      "plumbing_shadow",
    "landvalue_shadow",   "flammability_shadow", "residential_shadow", "zones_shadow",    "neighbours_shadow",
    "development_shadow", "tract_shadow",      "commercial_shadow",  "civic_shadow",      "parks_shadow",
    "zonedev_shadow",     "history_shadow",    "clock24_shadow",     "lotdev_shadow",     "traffic_shadow",
    "disasters_shadow",
};

struct PacingValue {
    fc4::PacingMode mode = fc4::PacingMode::Deterministic;
    std::string setting;  // the `pacing` text, when set
    const char* source = "";
    const char* shadow = nullptr;  // the shadow that chose `original`, if any
    bool invalid = false;
};

std::string Lower(std::string s) {
    size_t b = s.find_first_not_of(" \t");
    size_t e = s.find_last_not_of(" \t");
    if (b == std::string::npos) return std::string();
    s = s.substr(b, e - b + 1);
    for (char& c : s) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    return s;
}

PacingValue ReadPacing() {
    PacingValue r;
    const char* source = "";
    r.setting = fc4::Setting("pacing", "FC4_PACING", &source);
    r.source = source;
    for (const char* key : kShadowKeys) {
        if (fc4::SettingInt(key, nullptr, 0) != 0) {
            r.shadow = key;
            break;
        }
    }
    const std::string v = Lower(r.setting);
    if (v == "deterministic") {
        r.mode = fc4::PacingMode::Deterministic;
    } else if (v == "original") {
        r.mode = fc4::PacingMode::Original;
    } else {
        r.invalid = !v.empty();
        r.mode = r.shadow != nullptr ? fc4::PacingMode::Original : fc4::PacingMode::Deterministic;
    }
    return r;
}

const PacingValue& ThePacing() {
    static const PacingValue p = ReadPacing();  // thread-safe local static
    return p;
}

// This DLL's image, for PacingIsOurs.
struct Image {
    uintptr_t base = 0;
    uintptr_t end = 0;
};

const Image& OurImage() {
    static const Image img = [] {
        Image i;
        HMODULE m = nullptr;
        if (GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                               reinterpret_cast<LPCWSTR>(&OurImage), &m) &&
            m != nullptr) {
            const auto* dos = reinterpret_cast<const IMAGE_DOS_HEADER*>(m);
            const auto* nt = reinterpret_cast<const IMAGE_NT_HEADERS*>(reinterpret_cast<const uint8_t*>(m) + dos->e_lfanew);
            i.base = reinterpret_cast<uintptr_t>(m);
            i.end = i.base + nt->OptionalHeader.SizeOfImage;
        }
        return i;
    }();
    return img;
}

// Main thread only (the simulator's tick and the glue's posts).
bool g_dayOpen = false;
int32_t g_openDay = 0;
std::vector<cIGZMessage2*> g_queue;
fc4::PacingStats g_stats;

constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewYear = 0x66956817;
constexpr uint32_t kMsgLotStateChanged = 0xE8BDA2E7;

bool Deferrable(uint32_t type) {
    return (type >= kMsgSimNewDay && type <= kMsgSimNewYear) || type == kMsgLotStateChanged;
}

// [choice] The most messages one settle sends, counting those deferred while
// sending (a handler that posts again); the rest are released unsent.
constexpr size_t kMaxFlush = 1u << 20;

// [choice] The most message-server ticks one settle runs to empty the post
// queue (simulator.md §5.7: posted messages are delivered before the next
// day's agents). The ticks also stop when one leaves the queue no shorter (a
// message that is posted again at every tick stays for the frame's tick).
constexpr uint32_t kMaxPumps = 64;

}  // namespace

namespace fc4 {

PacingMode Pacing() { return ThePacing().mode; }

const char* PacingName(PacingMode m) { return m == PacingMode::Original ? "original" : "deterministic"; }

void PacingStartup() {
    const PacingValue& p = ThePacing();
    const bool ok = fc4_pacing_set_process(static_cast<uint32_t>(p.mode));
    if (p.invalid) {
        Log("pacing: WARNING pacing = '%s' (%s) is neither deterministic nor original; using the default",
            p.setting.c_str(), p.source);
    }
    const bool set = !p.setting.empty() && !p.invalid;
    const char* why = set ? "set" : p.shadow != nullptr ? "the default with a shadow on" : "the default";
    if (set && p.mode == PacingMode::Deterministic && p.shadow != nullptr) {
        Log("pacing: WARNING pacing = deterministic with %s on: the shadows compare under the original's pacing "
            "(their verdicts are not meaningful here)",
            p.shadow);
    }
    if (p.mode == PacingMode::Deterministic) {
        Log("pacing: deterministic (%s%s%s; simulator.md section 5.7 choice: our idle cycles complete within their "
            "day at its settle point, item counts instead of ms budgets, posted day/month/StateChanged messages sent "
            "at the settle point; pacing = original keeps the clock-bound pacing)%s",
            why, p.setting.empty() || p.invalid ? "" : ", ", p.setting.empty() || p.invalid ? "" : p.source,
            ok ? "" : " WARNING: the core refused the mode");
    } else {
        Log("pacing: original (%s%s%s%s; the original's frame- and clock-bound pacing, simulator.md section 5.7 "
            "items 1-4, for the conformance shadows; pacing = deterministic forces the default)%s",
            why, !set && p.shadow != nullptr ? ": " : "", !set && p.shadow != nullptr ? p.shadow : "",
            p.setting.empty() || p.invalid ? "" : (std::string(", ") + p.source).c_str(),
            ok ? "" : " WARNING: the core refused the mode");
    }
}

bool PacingIsOurs(const cIGZMessageTarget2* target) {
    if (target == nullptr) return false;
    const Image& img = OurImage();
    if (img.base == 0) return false;
    const uintptr_t vtbl = *reinterpret_cast<const uintptr_t*>(target);
    return vtbl >= img.base && vtbl < img.end;
}

void PacingDayOpened(int32_t julian) {
    if (!PacingDeterministic()) return;
    g_dayOpen = true;
    g_openDay = julian;
}

bool PacingDefer(cIGZMessage2* msg) {
    if (msg == nullptr || !g_dayOpen || !PacingDeterministic()) return false;
    if (!Deferrable(msg->GetType())) return false;
    msg->AddRef();  // the queue's reference (foundations §5.1: the post queue holds one too)
    g_queue.push_back(msg);
    ++g_stats.deferred;
    return true;
}

uint32_t PacingFlush(cIGZMessageServer2* ms, int32_t julian) {
    uint32_t sent = 0;
    size_t done = 0;
    // A handler may defer more while we send (the day is still open): they go
    // out in this settle too, after the earlier ones.
    while (done < g_queue.size()) {
        cIGZMessage2* m = g_queue[done];
        g_queue[done] = nullptr;
        ++done;
        if (ms != nullptr && done <= kMaxFlush) {
            ms->MessageSend(m);
            ++sent;
        } else {
            ++g_stats.dropped;
        }
        m->Release();
    }
    g_queue.clear();
    g_dayOpen = false;
    // Everything else posted during the day (the original's lot StateChanged
    // among it, and our cores' own posts): the message server's tick, until
    // its queue is empty.
    if (ms != nullptr) {
        uint32_t pumps = 0;
        uint32_t size = ms->GetMessageQueueSize();
        while (pumps < kMaxPumps && size > 0) {
            ms->OnTick(0);
            ++pumps;
            const uint32_t after = ms->GetMessageQueueSize();
            if (after >= size) {
                size = after;
                break;
            }
            size = after;
        }
        g_stats.pumps += pumps;
        if (size > 0) {
            ++g_stats.leftovers;
            if (g_stats.leftovers <= 3) {
                Log("pacing: the post queue still holds %u message(s) after %u message-server tick(s) at the settle "
                    "point of julian %d",
                    size, pumps, julian);
            }
        }
    }
    ++g_stats.settles;
    g_stats.sent += sent;
    return sent;
}

void PacingDrop(const char* why) {
    size_t n = 0;
    for (cIGZMessage2* m : g_queue) {
        if (m != nullptr) {
            m->Release();
            ++n;
        }
    }
    g_queue.clear();
    g_dayOpen = false;
    g_stats.dropped += n;
    if (n > 0) Log("pacing: %u deferred message(s) released unsent (%s)", static_cast<unsigned>(n), why);
}

PacingStats PacingCounters() { return g_stats; }

uint32_t PacingFixedIdleSteps(const char* name) {
    if (!PacingDeterministic() || name == nullptr) return 0;
    if (std::strcmp(name, "cSC4PropDeveloper") == 0) return 20;
    return 0;
}

}  // namespace fc4
