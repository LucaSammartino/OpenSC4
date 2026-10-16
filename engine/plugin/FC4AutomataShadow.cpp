// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR
// PURPOSE. See the GNU General Public License for more details. You should have
// received a copy of the GNU General Public License along with this program; if
// not, see <https://www.gnu.org/licenses/>.

// The automata layer shadow (FC4AutomataShadow.h; docs/specs/automata.md).
#include "FC4AutomataShadow.h"

#include "FC4Config.h"
#include "FC4HistClock.h"  // MemOStream
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"  // TokenOf
#include "FC4WriteCapture.h"

#include "cIGZUnknown.h"
#include "cISC4City.h"

#include <cinttypes>
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#include "fc4.h"

namespace fc4 {
namespace {

bool g_enabled = false;
bool g_loggedNoRecord = false;
cISC4City* g_city = nullptr;  // borrowed while the city is up

// Per-component tallies, summed for the shutdown line.
uint64_t g_lines = 0;
uint64_t g_cleanLines = 0;
uint64_t g_mismatches = 0;
uint64_t g_componentsCompared = 0;
uint64_t g_bytesCompared = 0;

// A line's detail, appended (the shadows' shared shape).
void AppendF(std::string& out, const char* fmt, ...) {
    char buf[512];
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(buf, sizeof(buf), _TRUNCATE, fmt, ap);
    va_end(ap);
    out += buf;
}

std::string DateText(int32_t julian) {
    char buf[16] = "?";
    if (julian > 0) {
        fc4_date d{};
        fc4_date_from_julian(julian, &d);
        _snprintf_s(buf, sizeof(buf), _TRUNCATE, "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
    }
    return buf;
}

// ---- the shadow cores' host: the references a captured record hands us ------

void HostAddRef(void* /*user*/, uint64_t token) {
    if (token != 0) FC4_ADDREF(static_cast<cIGZUnknown*>(reinterpret_cast<void*>(token)));
}

void HostRelease(void* /*user*/, uint64_t token) {
    if (token != 0) FC4_RELEASE(static_cast<cIGZUnknown*>(reinterpret_cast<void*>(token)));
}

// The controller manager's host also logs and reads a controller's flags
// (§3.2: flag 8 keeps a controller out of the record). The shadow never has a
// live controller to ask: a captured record carries no flags of its own, and
// a missing value reads as 0 (never transient), which is what the original
// writes for the controllers the corpus holds.
uint32_t HostControllerFlags(void* /*user*/, uint64_t /*controller*/) { return 0; }

void HostLog(void* /*user*/, const char* text) { Log("automata shadow: %s", text); }

fc4_automata_ref_host RefHost() {
    fc4_automata_ref_host h{};
    h.struct_size = sizeof(h);
    h.user = nullptr;
    h.add_ref = &HostAddRef;
    h.release = &HostRelease;
    return h;
}

fc4_automata_cm_host CmHost() {
    fc4_automata_cm_host h{};
    h.struct_size = sizeof(h);
    h.user = nullptr;
    h.log = &HostLog;
    h.add_ref = &HostAddRef;
    h.release = &HostRelease;
    h.controller_flags = &HostControllerFlags;
    return h;
}

// ---- one component: the original's record against the shadow core's --------

// The three shapes the six components come in (§2, §3, §4-§5).
enum class Kind { Scripts, Cm, Manager };

struct Part {
    const char* name;   // the log's short name
    uint32_t slot;      // the city getter (§0's table)
    uint32_t classId;   // the class ID of §0
    Kind kind;
    void* core = nullptr;
};

// §0's table: 40 the script system, 41 the controller manager, 43-46 the
// managers. The city getters are slots 48, 47, 43, 44, 45, 46.
Part g_parts[] = {
    {"script", 48, 0x4A1B664E, Kind::Scripts, nullptr},   // 40
    {"cm", 47, 0xCA14CA03, Kind::Cm, nullptr},            // 41
    {"vehicle", 43, 0xA98F9D3B, Kind::Manager, nullptr},  // 43
    {"pedestrian", 44, 0xC98F9D17, Kind::Manager, nullptr},  // 44
    {"aircraft", 45, 0xC98F9D4A, Kind::Manager, nullptr},    // 45
    {"watercraft", 46, 0xA98F9D28, Kind::Manager, nullptr},  // 46
};

constexpr size_t kParts = sizeof(g_parts) / sizeof(g_parts[0]);

cIGZUnknown* UnknownOf(cISC4City* city, uint32_t slot) {
    if (city == nullptr) return nullptr;
    switch (slot) {
        case 43:
            return reinterpret_cast<cIGZUnknown*>(city->GetVehicleManager());
        case 44:
            return reinterpret_cast<cIGZUnknown*>(city->GetPedestrianManager());
        case 45:
            return reinterpret_cast<cIGZUnknown*>(city->GetAircraftManager());
        case 46:
            return reinterpret_cast<cIGZUnknown*>(city->GetWatercraftManager());
        case 47:
            return reinterpret_cast<cIGZUnknown*>(city->GetAutomataControllerManager());
        case 48:
            return reinterpret_cast<cIGZUnknown*>(city->GetAutomataScriptSystem());
        default:
            return nullptr;
    }
}

void DropCore(Part& p) {
    if (p.core == nullptr) return;
    switch (p.kind) {
        case Kind::Scripts:
            fc4_automata_scripts_destroy(static_cast<fc4_automata_scripts*>(p.core));
            break;
        case Kind::Cm:
            fc4_automata_cm_destroy(static_cast<fc4_automata_cm*>(p.core));
            break;
        case Kind::Manager:
            fc4_automata_manager_destroy(static_cast<fc4_automata_manager*>(p.core));
            break;
    }
    p.core = nullptr;
}

bool MakeCore(Part& p) {
    DropCore(p);
    switch (p.kind) {
        case Kind::Scripts:
            p.core = fc4_automata_scripts_create();
            break;
        case Kind::Cm: {
            const fc4_automata_cm_host h = CmHost();
            p.core = fc4_automata_cm_create(&h);
            break;
        }
        case Kind::Manager: {
            const fc4_automata_ref_host h = RefHost();
            p.core = fc4_automata_manager_create(p.classId, &h);
            break;
        }
    }
    return p.core != nullptr;
}

bool WriteCore(Part& p, MemOStream& out) {
    switch (p.kind) {
        case Kind::Scripts:
            return fc4_automata_scripts_write(static_cast<fc4_automata_scripts*>(p.core), out.Get());
        case Kind::Cm:
            return fc4_automata_cm_write(static_cast<fc4_automata_cm*>(p.core), out.Get());
        case Kind::Manager:
            return fc4_automata_manager_write(static_cast<fc4_automata_manager*>(p.core), out.Get());
    }
    return false;
}

bool ReadCore(Part& p, CaptureIStream& in) {
    switch (p.kind) {
        case Kind::Scripts:
            return fc4_automata_scripts_read(static_cast<fc4_automata_scripts*>(p.core), in.Get());
        case Kind::Cm:
            return fc4_automata_cm_read(static_cast<fc4_automata_cm*>(p.core), in.Get());
        case Kind::Manager:
            return fc4_automata_manager_read(static_cast<fc4_automata_manager*>(p.core), in.Get());
    }
    return false;
}

// A short form of the two byte sequences' first difference.
void FirstDifference(const std::vector<uint8_t>& a, const std::vector<uint8_t>& b, char* out, size_t cap) {
    const size_t n = a.size() < b.size() ? a.size() : b.size();
    for (size_t i = 0; i < n; ++i) {
        if (a[i] != b[i]) {
            _snprintf_s(out, cap, _TRUNCATE, "first difference at byte %u (ours 0x%02X, original 0x%02X)",
                        static_cast<unsigned>(i), a[i], b[i]);
            return;
        }
    }
    _snprintf_s(out, cap, _TRUNCATE, "lengths differ (ours %u, original %u)", static_cast<unsigned>(a.size()),
                static_cast<unsigned>(b.size()));
}

// One component: the original's own Write against the shadow core's.
// Returns the mismatch count (0 = the records are identical).
uint32_t ComparePart(Part& p, std::string& out) {
    cIGZUnknown* obj = UnknownOf(g_city, p.slot);
    if (obj == nullptr) {
        AppendF(out, " %s: no component", p.name);
        return 0;
    }
    WriteCapture cap;
    bool answered = false;
    const bool captured = cap.Capture(obj, &answered);
    if (!captured) {
        AppendF(out, " %s: the original's record %s", p.name, answered ? "failed" : "cannot be read");
        if (!g_loggedNoRecord) {
            g_loggedNoRecord = true;
            Log("automata shadow: the original's record (its own Write) %s; nothing compared",
                answered ? "failed" : "cannot be read (no cIGZSerializable)");
        }
        return 1;
    }
    const std::vector<uint8_t> original = cap.Bytes();
    if (!MakeCore(p)) {
        AppendF(out, " %s: the shadow core could not be created", p.name);
        return 1;
    }
    CaptureIStream in(cap);
    const bool read = ReadCore(p, in);
    MemOStream again;
    const bool written = read && in.atEnd() && WriteCore(p, again);
    uint32_t mismatches = 0;
    if (!read) {
        AppendF(out, " %s: %u bytes, read FAILED", p.name, static_cast<unsigned>(original.size()));
        mismatches = 1;
    } else if (!in.atEnd()) {
        AppendF(out, " %s: %u bytes, read did not consume the record", p.name,
                static_cast<unsigned>(original.size()));
        mismatches = 1;
    } else if (!written) {
        AppendF(out, " %s: %u bytes, written back FAILED", p.name, static_cast<unsigned>(original.size()));
        mismatches = 1;
    } else if (again.bytes == original) {
        AppendF(out, " %s: %u bytes identical", p.name, static_cast<unsigned>(original.size()));
    } else {
        char diff[128] = "";
        FirstDifference(again.bytes, original, diff, sizeof(diff));
        AppendF(out, " %s: %u bytes DIFFERENT (%s)", p.name, static_cast<unsigned>(original.size()), diff);
        mismatches = 1;
    }
    g_bytesCompared += original.size();
    return mismatches;
}

void RunCompare(const char* what, int32_t julian) {
    if (!g_enabled || g_city == nullptr) return;
    std::string line;
    uint32_t mismatches = 0;
    for (size_t i = 0; i < kParts; ++i) {
        mismatches += ComparePart(g_parts[i], line);
    }
    AppendF(line, "; mismatches=%u", mismatches);
    char buf[64] = "";
    const std::string d = DateText(julian);
    _snprintf_s(buf, sizeof(buf), _TRUNCATE, "automata shadow %s %s:", d.c_str(), what);
    Log("%s%s", buf, line.c_str());
    ++g_lines;
    if (mismatches == 0) {
        ++g_cleanLines;
    } else {
        g_mismatches += mismatches;
    }
    g_componentsCompared += kParts;
}

}  // namespace

void AutomataShadowStartup(bool automataReplaced) {
    const char* src = "ini";
    g_enabled = SettingInt("automata_shadow", "FC4_AUTOMATA_SHADOW", 0) != 0;
    if (g_enabled) Setting("automata_shadow", "FC4_AUTOMATA_SHADOW", &src);
    if (!g_enabled) return;
    if (automataReplaced) {
        Log("automata shadow: requested (automata_shadow from %s) but INACTIVE: it compares against the ORIGINAL "
            "automata components, and `automata` is enabled (ours)",
            src);
        g_enabled = false;
        return;
    }
    Log("automata shadow: ON (automata_shadow from %s): the original's six component records (40 the script system, "
        "41 the controller manager, 43-46 the four managers) and every object their records reference are read by a "
        "shadow core (FreeCity4's automata layer) and written back, and the two byte sequences are compared at each "
        "city load and each month. Success is mismatches=0 on every line.",
        src);
}

void AutomataShadowStart() {}

void AutomataShadowStop() {
    g_enabled = false;
    g_city = nullptr;
}

void AutomataShadowCityLoaded(cISC4City* city) {
    g_city = city;
    if (!g_enabled || city == nullptr) return;
    g_loggedNoRecord = false;
    RunCompare("load", 0);
}

void AutomataShadowCityShutdown() {
    if (!g_enabled && g_lines == 0) {
        g_city = nullptr;
        return;
    }
    Log("automata shadow: city shutdown: %llu lines, %llu with mismatches=0; components compared %llu, bytes "
        "compared %llu, mismatches %llu",
        static_cast<unsigned long long>(g_lines), static_cast<unsigned long long>(g_cleanLines),
        static_cast<unsigned long long>(g_componentsCompared), static_cast<unsigned long long>(g_bytesCompared),
        static_cast<unsigned long long>(g_mismatches));
    for (size_t i = 0; i < kParts; ++i) DropCore(g_parts[i]);
    g_city = nullptr;
    g_lines = 0;
    g_cleanLines = 0;
    g_mismatches = 0;
    g_componentsCompared = 0;
    g_bytesCompared = 0;
}

void AutomataShadowMonth(int32_t julian) { RunCompare("month", julian); }

}  // namespace fc4

void AutomataShadowStartup(bool automataReplaced) { fc4::AutomataShadowStartup(automataReplaced); }
void AutomataShadowStart() { fc4::AutomataShadowStart(); }
void AutomataShadowStop() { fc4::AutomataShadowStop(); }
void AutomataShadowCityLoaded(cISC4City* city) { fc4::AutomataShadowCityLoaded(city); }
void AutomataShadowCityShutdown() { fc4::AutomataShadowCityShutdown(); }
void AutomataShadowMonth(int32_t julian) { fc4::AutomataShadowMonth(julian); }
