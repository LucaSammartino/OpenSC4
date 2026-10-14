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

#include "FC4Script.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4HarnessLogic.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Observer.h"
#include "FC4Turbo.h"
#include "cFC4ZoneManager.h"

#include "cISC4City.h"

#include <cctype>
#include <cstdio>
#include <string>
#include <utility>
#include <vector>

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

// Game calls (checked against docs/abi/interfaces.tsv; none returns in ST0,
// so the x87 rule holds with the declarations of FC4Interfaces.h):
//   cISC4City 36 GetZoneManager (borrowed), 50 GetSimulator (borrowed);
//   cISC4ZoneManager 11 PlaceZone (pop 40, AL);
//   cISC4Simulator 9 Pause, 12 Resume, 15 IsPaused, 27 GetSimSpeed,
//   28 SetSimSpeed (pop 4) - all eax-or-void.

namespace fc4 {
namespace {

harness::ScriptSchedule g_schedule;  // main thread only (game start, SimNewDay)
std::string g_path;                  // the loaded script, "" = none

constexpr unsigned long long kMaxScriptBytes = 4ull << 20;  // [choice] a text file of actions, not a save

std::string Trimmed(std::string s) {
    size_t a = 0, b = s.size();
    while (a < b && std::isspace(static_cast<unsigned char>(s[a]))) ++a;
    while (b > a && std::isspace(static_cast<unsigned char>(s[b - 1]))) --b;
    return s.substr(a, b - a);
}

// [choice] A relative path is relative to the folder of FreeCity4.ini (the
// DLL's folder), not to the game's working directory.
std::string ResolvePath(const std::string& p) {
    const bool absolute = (p.size() >= 2 && p[1] == ':') || (!p.empty() && (p[0] == '\\' || p[0] == '/'));
    if (absolute) return p;
    const std::string ini = IniPath();
    const size_t slash = ini.find_last_of("\\/");
    return slash == std::string::npos ? p : ini.substr(0, slash + 1) + p;
}

bool ReadWhole(const std::string& path, std::string* out, unsigned long* error) {
    HANDLE f = CreateFileA(path.c_str(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                           nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (f == INVALID_HANDLE_VALUE) {
        *error = GetLastError();
        return false;
    }
    LARGE_INTEGER size{};
    if (!GetFileSizeEx(f, &size) || size.QuadPart < 0 ||
        static_cast<unsigned long long>(size.QuadPart) > kMaxScriptBytes) {
        *error = size.QuadPart > 0 ? static_cast<unsigned long>(ERROR_FILE_TOO_LARGE) : GetLastError();
        CloseHandle(f);
        return false;
    }
    out->assign(static_cast<size_t>(size.QuadPart), '\0');
    DWORD got = 0;
    const bool ok = out->empty() ||
                    (ReadFile(f, &(*out)[0], static_cast<DWORD>(out->size()), &got, nullptr) && got == out->size());
    if (!ok) *error = GetLastError();
    CloseHandle(f);
    return ok;
}

// ---- zone, pricezone, dezone: PlaceZone (zones.md §6) -----------------------------------------------

struct ZoneCall {
    cFC4ISC4ZoneManager* zm;
    SC4CellRegion<long>* region;
    int32_t type;
    bool commit;
    bool ok;
    int64_t cost;
    int32_t error;
};

// The argument list of zones.md §6, as a user's zoning ([choice] the flags):
// skipFunds = false (the normal funds rules), checkSlope = true (§6 notes:
// it "behaves as 'this is a user action'" [inferred]), skipSizeCheck =
// false, checkLotInside = true, outLots = null (a set would need the game's
// allocator B for its nodes, §13.2; nothing here needs the lots).
void ZoneStep(void* p) {
    auto* c = static_cast<ZoneCall*>(p);
    c->ok = c->zm->PlaceZone(*c->region, c->type, c->commit, false, true, false, true, &c->cost, &c->error,
                             nullptr);  // slot 11
}

const char* ZoneErrorName(int32_t e) {
    switch (e) {  // zones.md §6
        case 0: return "none";
        case 1: return "insufficient funds";
        case 2: return "landfill on water";
        case 3: return "zone too small";
        case 4: return "zone too large";
        case 5: return "blocked";
        default: return "unknown";
    }
}

std::string RunZone(const harness::ScriptAction& a, cISC4City* city) {
    if (city == nullptr) return "skipped: no city";
    cFC4ISC4ZoneManager* zm = AsGame(city->GetZoneManager());  // city slot 36, borrowed
    if (zm == nullptr) return "skipped: the city has no zone manager";
    // The region in the §11.4 layout, every cell set, in our own memory: the
    // game only clears the bits of rejected cells and never reallocates or
    // frees it (zones.md §13.2), so allocator C is not needed.
    ZoneRegionBuffer rb;
    if (!rb.BuildFull(a.x0, a.z0, a.x1, a.z1)) return "error: the rectangle is larger than a zone region can be";
    ZoneCall c{zm, &rb.Game(), a.type, a.op != harness::ScriptOp::PriceZone, false, 0, 0};
    const bool clean = SehGuard("script PlaceZone", &ZoneStep, &c);
    const uint32_t cells = static_cast<uint32_t>(a.x1 - a.x0 + 1) * static_cast<uint32_t>(a.z1 - a.z0 + 1);
    uint32_t accepted = 0;
    for (int32_t x = a.x0; x <= a.x1; ++x) {
        for (int32_t z = a.z0; z <= a.z1; ++z) accepted += rb.Bit(x, z) ? 1u : 0u;
    }
    char b[224];
    std::snprintf(b, sizeof(b), "%sok=%d error=%d (%s) cost=%lld accepted=%u of %u cell(s)%s",
                  c.commit ? "" : "dry run: ", c.ok ? 1 : 0, c.error, ZoneErrorName(c.error),
                  static_cast<long long>(c.cost), accepted, cells,
                  clean ? "" : " [structured exception in PlaceZone]");
    return b;
}

// ---- speed, pause, resume: the city's simulator ------------------------------------------------------

std::string RunSimulator(const harness::ScriptAction& a, cISC4City* city) {
    cFC4ISC4Simulator* sim = city ? AsGame(city->GetSimulator()) : nullptr;  // city slot 50, borrowed
    if (sim == nullptr) return "skipped: no city simulator";
    char b[160];
    if (a.op == harness::ScriptOp::Speed) {
        const int32_t before = sim->GetSimSpeed();
        const bool r = sim->SetSimSpeed(a.speed);
        std::snprintf(b, sizeof(b), "SetSimSpeed(%d) = %d; speed %d -> %d", a.speed, r ? 1 : 0, before,
                      sim->GetSimSpeed());
    } else if (a.op == harness::ScriptOp::Pause) {
        const bool r = sim->Pause();
        std::snprintf(b, sizeof(b), "Pause() = %d; paused=%d", r ? 1 : 0, sim->IsPaused() ? 1 : 0);
    } else {
        const bool r = sim->Resume();
        std::snprintf(b, sizeof(b), "Resume() = %d; paused=%d", r ? 1 : 0, sim->IsPaused() ? 1 : 0);
    }
    return b;
}

void RunAction(const harness::ScriptAction& a, int32_t julian, cISC4City* city) {
    using harness::ScriptOp;
    std::string result;
    bool digest = false;
    switch (a.op) {
        case ScriptOp::Zone:
        case ScriptOp::PriceZone:
        case ScriptOp::Dezone: result = RunZone(a, city); break;
        case ScriptOp::Speed:
        case ScriptOp::Pause:
        case ScriptOp::Resume: result = RunSimulator(a, city); break;
        case ScriptOp::Turbo:
            result = TurboScriptSet(a.on) ? (a.on ? "turbo on" : "turbo off")
                                          : "no effect: our simulator is not running (disable = simulator?)";
            break;
        case ScriptOp::Digest:
            result = "the DIGEST line follows";
            digest = true;
            break;
        case ScriptOp::Mark: result = "marked"; break;
    }
    std::string late;
    if (julian != a.julian) late = " (run on " + harness::FormatDate(harness::CivilFromJulian(julian)) + ")";
    Log("script: line %d %s %s%s%s -> %s%s", a.line, harness::FormatDate(a.date).c_str(), harness::ScriptOpName(a.op),
        a.text.empty() ? "" : " ", a.text.c_str(), result.c_str(), late.c_str());
    if (digest) ObserverWriteDigest(julian);
}

}  // namespace

int ScriptLoad(const char* rawPath, const char* why) {
    g_schedule.Load({});
    g_path.clear();
    const std::string path = ResolvePath(rawPath ? Trimmed(rawPath) : std::string());
    std::string text;
    unsigned long error = 0;
    if (path.empty() || !ReadWhole(path, &text, &error)) {
        Log("script: ERROR cannot read '%s' (%s; error %lu); no action runs", path.c_str(), why, error);
        return -1;
    }
    harness::ParsedScript parsed = harness::ParseScript(text);
    if (!parsed.errors.empty()) {
        for (const harness::ScriptError& e : parsed.errors) Log("script: ERROR line %d: %s", e.line, e.message.c_str());
        Log("script: REFUSED '%s' (%s): %u parse error(s); no action of it runs", path.c_str(), why,
            static_cast<unsigned>(parsed.errors.size()));
        return -2;
    }
    g_schedule.Load(std::move(parsed.actions));
    g_path = path;
    if (g_schedule.Size() == 0) {
        Log("script: '%s' (%s): no actions", path.c_str(), why);
        return 0;
    }
    Log("script: loaded '%s' (%s; tooling): %u action(s), %s to %s; each runs once at the first new day on or after "
        "its date",
        path.c_str(), why, static_cast<unsigned>(g_schedule.Size()),
        harness::FormatDate(g_schedule.First()->date).c_str(), harness::FormatDate(g_schedule.Last()->date).c_str());
    return static_cast<int>(g_schedule.Size());
}

void ScriptReload() {
    const std::string v = Trimmed(Setting("script", "FC4_SCRIPT"));
    if (v.empty()) {  // off: nothing read, nothing logged
        g_schedule.Load({});
        g_path.clear();
        return;
    }
    ScriptLoad(v.c_str(), "script setting");
}

void ScriptOnNewDay(int32_t julian) {
    if (g_schedule.Remaining() == 0) return;
    const std::vector<const harness::ScriptAction*> due = g_schedule.Due(julian);
    if (due.empty()) return;
    // [choice] Run inside the SimNewDay message, not at the next tick.
    cISC4City* city = ObserverCity();
    for (const harness::ScriptAction* a : due) RunAction(*a, julian, city);
    if (g_schedule.Remaining() == 0) {
        Log("script: all %u action(s) of '%s' have run", static_cast<unsigned>(g_schedule.Size()), g_path.c_str());
    }
}

}  // namespace fc4
