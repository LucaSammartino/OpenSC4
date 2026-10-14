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

#include "FC4CrimeShadow.h"

#include "FC4Config.h"
#include "FC4CrimeInterfaces.h"
#include "FC4Diag.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cFC4Crime.h"
#include "cFC4Message2Standard.h"
#include "fc4.h"

#include "SC4Rect.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4App.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4CrimeSimulator.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4SimGrid.h"
#include "cISC4ZoneManager.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <climits>
#include <cstdarg>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgSimIdle = 0x6695681A;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr const char* kAgentName = "cSC4CrimeSimulator";  // §4.1 step 4
constexpr uint32_t kExemplarType = 0x6534284A;
constexpr uint32_t kCrimeTypeGroup = 0xEA12F32C;
constexpr uint32_t kPropSeverity = 0xAA12FCB5;
constexpr uint32_t kPropDecay = 0x8A0CBAAC;
constexpr uint32_t kPropTractSize = 0x47E2C200;
constexpr long kDefaultCells = 8;
constexpr long kMaxCells = 64;
// The incident vector's capacity, reserved before the call so that the
// original only fills it (§11.4 resizes; within the capacity nothing is
// reallocated). [choice] A larger month reallocates with the game's allocator
// (B), the same one this vector uses in the game.
constexpr size_t kIncidentCapacity = 4096;

// §5 pipeline states.
constexpr int kStateUnknown = -1;
constexpr int kStateCommit = 4;
constexpr int kStateIdle = 6;

const char kNotCompared[] =
    "not compared: crimes to commit (needs ResetUnjailedCriminals, which has side effects), police stops and type "
    "choice (the original's random draws), the lots tried (the lot manager's generator), the crime-effect grid "
    "(not exposed by any interface)";

// ---- settings and run state (main thread only) ----------------------------------------------------

bool g_enabled = false;
long g_cells = kDefaultCells;
int32_t g_settingsJulian = INT_MIN;

// Test seam (op 13).
bool g_testMode = false;
const FC4CrimeShadowTestInput* g_test = nullptr;

// FreeCity4's tuning and crime types (once per process: exemplars do not change).
fc4_crime_shadow* g_core = nullptr;
fc4_crime_shadow_info g_info{};

// The original simulator, its agent and the pipeline mirror.
const void* g_sim = nullptr;
cIGZMessageTarget2* g_agent = nullptr;
bool g_loggedOurs = false;
int g_state = kStateUnknown;
bool g_firstCommit = true;  // no state 4 observed yet for this simulator (§4.1 step 9: hotspot (0, 0, 0))
int32_t g_lastJulian = 0;
long g_months = 0, g_clean = 0, g_unobserved = 0;
int g_lastResult = -1;

void ReadSettings() {
    g_enabled = SettingInt("crime_shadow", "FC4_CRIME_SHADOW", 0) != 0;
    long n = SettingInt("crime_shadow_cells", "FC4_CRIME_SHADOW_CELLS", kDefaultCells);
    if (n < 0) n = 0;
    if (n > kMaxCells) n = kMaxCells;
    g_cells = n;
}

cISC4City* CurrentCity() {
    if (g_testMode) return g_test->city;
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    cISC4City* city = sc4->GetCity();
    sc4->Release();
    return city;
}

// The incident vector's allocator: the game's (B) in the game, the CRT heap
// for the test seam (no game allocator there; the fake fills within capacity).
struct ShadowAlloc {
    static bool Available() { return g_testMode || StlNodeAllocator::Available(); }
    static void* Allocate(uint32_t size) { return g_testMode ? std::malloc(size) : StlNodeAllocator::Allocate(size); }
    static void Free(void* p) {
        if (g_testMode) {
            std::free(p);
        } else {
            StlNodeAllocator::Free(p);
        }
    }
};
using IncidentVector = GameVector<FC4CrimeEvent, ShadowAlloc>;
static_assert(sizeof(IncidentVector) == sizeof(GameVector<FC4CrimeEvent>), "the same vector layout");

// ---- the test seam's tuning and types (fc4_crime_host, read-only part) -------------------------------

fc4_props* TestTuning(void*) {
    fc4_props* p = fc4_props_create();
    if (p == nullptr) return nullptr;
    const float decay = g_test->decay;
    const uint32_t size = g_test->tractSize;
    fc4_props_set_f32s(p, kPropDecay, &decay, 1);
    fc4_props_set_u32s(p, kPropTractSize, &size, 1);
    return p;
}

void TestForEachKey(void*, fc4_key_visitor visit, void* ctx) {
    for (uint32_t i = 0; i < g_test->typeCount && visit != nullptr; ++i) {
        visit(ctx, kExemplarType, kCrimeTypeGroup, g_test->typeIds[i]);
    }
}

fc4_props* TestExemplar(void*, uint32_t type, uint32_t group, uint32_t instance) {
    if (type != kExemplarType || group != kCrimeTypeGroup) return nullptr;
    for (uint32_t i = 0; i < g_test->typeCount; ++i) {
        if (g_test->typeIds[i] != instance) continue;
        fc4_props* p = fc4_props_create();
        if (p != nullptr) fc4_props_set_u32s(p, kPropSeverity, &g_test->severities[i], 1);
        return p;
    }
    return nullptr;
}

fc4_crime_host HostFor() {
    if (!g_testMode) return CrimeReadOnlyHost();
    fc4_crime_host h{};
    h.struct_size = sizeof(h);
    h.tuning = &TestTuning;
    h.for_each_type_key = &TestForEachKey;
    h.exemplar = &TestExemplar;
    return h;
}

bool EnsureCore() {
    if (g_core != nullptr) return true;
    const DWORD t0 = GetTickCount();
    const fc4_crime_host h = HostFor();
    g_core = fc4_crime_shadow_create(&h);
    g_info = fc4_crime_shadow_info{};
    g_info.struct_size = sizeof(g_info);
    if (g_core == nullptr || !fc4_crime_shadow_get_info(g_core, &g_info)) {
        Log("crime shadow: FreeCity4's crime tuning and types could not be read; not compared");
        if (g_core != nullptr) fc4_crime_shadow_destroy(g_core);
        g_core = nullptr;
        return false;
    }
    Log("crime shadow: FreeCity4's crime model: %u crime types, tuning %s (tract size %u, decay %.9g, reoffend %.9g) "
        "(%lu ms)",
        g_info.types, g_info.tuning_found ? "from its exemplar" : "MISSING: code defaults", g_info.tract_size,
        static_cast<double>(g_info.decay), static_cast<double>(g_info.reoffend),
        static_cast<unsigned long>(GetTickCount() - t0));
    return true;
}

void DropCore() {
    if (g_core != nullptr) fc4_crime_shadow_destroy(g_core);
    g_core = nullptr;
}

// The original crime simulator if `target` is its agent: its
// cIGZMessageTarget2 interface (QI, released at once: only its identity is
// kept), or failing that (logged) an agent registered under its name.
cFC4ISC4CrimeSimulator* OriginalFor(cISC4City* city, cIGZMessageTarget2* target, const char* agentName) {
    if (city == nullptr || target == nullptr) return nullptr;
    cISC4CrimeSimulator* raw = city->GetCrimeSimulator();  // city slot 55, borrowed
    if (raw == nullptr) return nullptr;
    if (IsOurObject(raw)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("crime shadow: inactive: the city's crime simulator is FreeCity4's own (the shadow compares against "
                "the ORIGINAL crime simulator: do not enable crime)");
        }
        return nullptr;
    }
    if (raw != g_sim) {
        void* agent = nullptr;
        const bool qi = raw->QueryInterface(kIID_cIGZMessageTarget2, &agent) && agent != nullptr;
        g_agent = qi ? static_cast<cIGZMessageTarget2*>(agent) : nullptr;
        if (qi) g_agent->Release();  // the simulator keeps it alive; only its identity is kept
        g_sim = raw;
        // §5: a new simulator (a new or loaded city) starts in state 6; its
        // hotspot is (0, 0, 0) until its first state 4 (§4.1 step 9).
        g_state = kStateIdle;
        g_firstCommit = true;
        g_lastJulian = 0;
        Log("crime shadow: tracking the original crime simulator %p, agent %p (%s); pipeline state 6 (a new or "
            "loaded city, §5)",
            static_cast<void*>(raw), static_cast<void*>(g_agent),
            qi ? "its cIGZMessageTarget2 interface" : "no interface: matched by name");
    }
    const bool same = g_agent != nullptr && target == g_agent;
    const bool byName = g_agent == nullptr && agentName != nullptr && strcmp(agentName, kAgentName) == 0;
    return (same || byName) ? reinterpret_cast<cFC4ISC4CrimeSimulator*>(raw) : nullptr;
}

// ---- reading the original -------------------------------------------------------------------------

using GridU8 = cISC4SimGrid<uint8_t>;

void ReadGeometry(GridU8* g, fc4_grid_geometry* out) {
    out->tract_size = g->GetTractSize();
    out->shift = g->GetTractShift();
    out->count_x = g->GetTractCountX();
    out->count_z = g->GetTractCountZ();
    out->width_x = g->GetTractWidthX();
    out->width_z = g->GetTractWidthZ();
    out->inv_width_x = g->GetOneOverTractWidthX();
    out->inv_width_z = g->GetOneOverTractWidthZ();
}

size_t TractCount(const fc4_grid_geometry& g) {
    return (g.count_x > 0 && g.count_z > 0) ? static_cast<size_t>(g.count_x) * static_cast<size_t>(g.count_z) : 0;
}

// count_x * count_z values, x outer (fc4_crime_shadow_month_in).
void ReadValues(GridU8* g, const fc4_grid_geometry& geo, std::vector<uint8_t>* out) {
    out->assign(TractCount(geo), 0);
    size_t i = 0;
    for (int32_t x = 0; x < geo.count_x; ++x) {
        for (int32_t z = 0; z < geo.count_z; ++z) (*out)[i++] = g->GetTractValue(x, z);
    }
}

struct Hotspot {
    bool set = false;  // GetCrimeHotspot returned true
    uint8_t value = 0;
    int32_t x = 0, z = 0;  // city cells (S * tract)
};

Hotspot ReadHotspot(cFC4ISC4CrimeSimulator* sim) {
    Hotspot h;
    h.set = sim->GetCrimeHotspot(&h.value, &h.x, &h.z);  // slot 8
    if (!h.set) h = Hotspot{};
    return h;
}

struct Incident {
    FC4CrimeEvent event;
    bool hasLot = false;
    fc4_cell_rect rect{};
    int32_t candidate = -1;  // fc4_crime_shadow_candidate (1, 0, -1 unknown type / unread)
};

struct Month {
    cISC4City* city = nullptr;
    cFC4ISC4CrimeSimulator* sim = nullptr;
    fc4_date date{};
    GridU8* gridBefore = nullptr;
    GridU8* gridAfter = nullptr;
    fc4_grid_geometry geo{};
    std::vector<uint8_t> pre, post;
    Hotspot hotBefore, hotAfter;
    uint32_t count = 0, average = 0;
    bool arrayRead = false;
    uint32_t arrayLength = 0;
    std::vector<Incident> incidents;
    std::vector<uint32_t> zones;
    bool zonesRead = false;
};

Month g_m;

void BeforeStep(void*) {
    Month& m = g_m;
    m.gridBefore = m.sim->GetCrimeGrid();  // slot 5, borrowed
    m.pre.clear();
    m.geo = fc4_grid_geometry{};
    if (m.gridBefore != nullptr) {
        ReadGeometry(m.gridBefore, &m.geo);
        ReadValues(m.gridBefore, m.geo, &m.pre);
    }
    m.hotBefore = ReadHotspot(m.sim);
}

// The incident's lot bits (§6.7), read only: lot.GetLocation -> zone type,
// lot.GetBuilding -> profile, lot.GetCondition.
int32_t Candidate(cFC4ISC4Lot* lot, cISC4ZoneManager* zm, uint32_t typeId) {
    int32_t lx = 0, lz = 0;
    const uint32_t zone = (lot->GetLocation(lx, lz) && zm != nullptr) ? static_cast<uint32_t>(zm->GetZoneType(lx, lz)) : 0;
    cISC4BuildingOccupant* b = lot->GetBuilding();
    uint32_t purpose = 0, wealth = 0;
    if (b != nullptr) {
        const cISC4BuildingOccupant::BuildingProfile& p = b->GetBuildingProfile();
        purpose = static_cast<uint32_t>(p.purpose);
        wealth = static_cast<uint32_t>(p.wealth);
    }
    const uint8_t condition = static_cast<uint8_t>(lot->GetCondition());
    return fc4_crime_shadow_candidate(g_core, typeId, zone, b != nullptr, purpose, wealth, condition);
}

void AfterStep(void*) {
    Month& m = g_m;
    m.gridAfter = m.sim->GetCrimeGrid();
    m.post.clear();
    if (m.gridAfter != nullptr) {
        fc4_grid_geometry geo{};
        ReadGeometry(m.gridAfter, &geo);
        if (m.gridBefore == nullptr) m.geo = geo;
        ReadValues(m.gridAfter, geo, &m.post);
    }
    m.hotAfter = ReadHotspot(m.sim);
    m.count = m.sim->GetMonthlyCrimeCount();  // slot 7
    m.average = m.sim->GetAverageCrime();      // slot 6
    // The incident list (slot 11) into our vector, reserved first.
    m.incidents.clear();
    m.arrayRead = false;
    m.arrayLength = 0;
    IncidentVector v;
    if (v.reserve(kIncidentCapacity)) {
        m.sim->GetCrimeIncidentArray(reinterpret_cast<GameVector<FC4CrimeEvent>&>(v));
        m.arrayRead = true;
        m.arrayLength = static_cast<uint32_t>(v.size());
        cISC4LotManager* lmRaw = m.city->GetLotManager();  // city slot 33, borrowed
        cFC4ISC4LotManager* lm = lmRaw ? AsGame(lmRaw) : nullptr;
        cISC4ZoneManager* zm = m.city->GetZoneManager();  // city slot 36, borrowed
        for (size_t i = 0; i < v.size(); ++i) {
            Incident inc;
            inc.event = v[i];
            // The incident's lot: the lot at its cell (the cell of the lot's
            // centre, §11.4), read only (GetLot(x, z, false), borrowed).
            cISC4Lot* lot = lm ? lm->GetLot(static_cast<int32_t>(inc.event.cellX),
                                            static_cast<int32_t>(inc.event.cellZ), false)
                               : nullptr;
            if (lot != nullptr) {
                cFC4ISC4Lot* l = AsGame(lot);
                SC4Rect<int32_t> r{};
                l->GetBoundingRect(r);
                inc.hasLot = true;
                inc.rect.x1 = r.topLeftX;
                inc.rect.z1 = r.topLeftY;
                inc.rect.x2 = r.bottomRightX;
                inc.rect.z2 = r.bottomRightY;
                if (g_core != nullptr) inc.candidate = Candidate(l, zm, inc.event.typeID);
            }
            m.incidents.push_back(inc);
        }
    }
    // §6.9: the zone type at every tract corner (x * S, z * S), S the grid's.
    m.zones.clear();
    m.zonesRead = false;
    if (cISC4ZoneManager* zm = m.city->GetZoneManager()) {
        const int32_t s = m.geo.tract_size;
        m.zones.reserve(TractCount(m.geo));
        for (int32_t x = 0; x < m.geo.count_x; ++x) {
            for (int32_t z = 0; z < m.geo.count_z; ++z) {
                m.zones.push_back(static_cast<uint32_t>(zm->GetZoneType(x * s, z * s)));
            }
        }
        m.zonesRead = true;
    }
}

// ---- the report -------------------------------------------------------------------------------------

void AppendF(char* out, size_t cap, const char* fmt, ...) {
    const size_t len = strlen(out);
    if (len + 1 >= cap) return;
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(out + len, cap - len, _TRUNCATE, fmt, ap);
    va_end(ap);
}

void AppendHotspot(char* out, size_t cap, bool set, uint8_t value, int32_t x, int32_t z) {
    if (set) {
        AppendF(out, cap, "(%d,%d)=%u", x, z, value);
    } else {
        AppendF(out, cap, "none");
    }
}

uint32_t TruncAverage(uint32_t bits) {
    float f = 0.0f;
    std::memcpy(&f, &bits, sizeof(f));
    if (!(f > -9.2233720e18f && f < 9.2233720e18f)) return 0;  // _ftol: NaN / out of range -> 0 (§6.9)
    return static_cast<uint32_t>(static_cast<int64_t>(f));
}

// The state-4 comparison; returns the mismatch count, -1 if not compared.
int Report(bool beforeClean, bool afterClean) {
    Month& m = g_m;
    char prefix[48];
    _snprintf_s(prefix, sizeof(prefix), _TRUNCATE, "crime shadow %04d-%02d-%02d", m.date.year, m.date.month,
                m.date.day_of_month);
    if (!beforeClean || !afterClean) {
        Log("%s: INCOMPLETE (structured exception %s the original's state 4); not compared", prefix,
            beforeClean ? "after" : "before");
        return -1;
    }
    if (g_core == nullptr) {
        Log("%s: INCOMPLETE (no FreeCity4 crime model); not compared", prefix);
        return -1;
    }
    if (m.gridBefore == nullptr || m.gridAfter == nullptr || m.gridBefore != m.gridAfter || m.pre.empty() ||
        m.pre.size() != m.post.size()) {
        Log("%s: INCOMPLETE (crime grid before/after %p/%p, %u/%u tracts); not compared", prefix,
            static_cast<void*>(m.gridBefore), static_cast<void*>(m.gridAfter), static_cast<unsigned>(m.pre.size()),
            static_cast<unsigned>(m.post.size()));
        return -1;
    }
    const int32_t s = m.geo.tract_size > 0 ? m.geo.tract_size : 1;
    // The hotspot before, in tracts. GetCrimeHotspot answers only for a value
    // above 0 (§11.2). Otherwise: before the simulator's first state 4 it is
    // (0, 0) (§4.1 step 9); later its value 0 is its tract's value (decay and
    // stamps keep hotspotValue = grid[hx][hz], §6.5, §6.6), so any tract of
    // value 0 replays the same (the first one is used).
    const char* hotNote = "read";
    int32_t hx = 0, hz = 0;
    if (m.hotBefore.set) {
        hx = m.hotBefore.x / s;
        hz = m.hotBefore.z / s;
    } else if (g_firstCommit) {
        hotNote = "(0,0): the simulator's first state 4 [assumed]";
    } else {
        hotNote = "a tract of value 0";
        for (size_t i = 0; i < m.pre.size(); ++i) {
            if (m.pre[i] == 0) {
                hx = static_cast<int32_t>(i / static_cast<size_t>(m.geo.count_z));
                hz = static_cast<int32_t>(i % static_cast<size_t>(m.geo.count_z));
                break;
            }
            if (i + 1 == m.pre.size()) hotNote = "UNKNOWN (no tract of value 0): (0,0) used";
        }
    }
    std::vector<fc4_crime_shadow_incident> inc;
    inc.reserve(m.incidents.size());
    uint32_t noLot = 0, notCandidates = 0, unknownTypes = 0;
    for (const Incident& i : m.incidents) {
        fc4_crime_shadow_incident x{};
        x.type_id = i.event.typeID;
        x.has_rect = i.hasLot ? 1 : 0;
        x.rect = i.rect;
        if (!i.hasLot) ++noLot;
        if (i.candidate == 0) ++notCandidates;
        if (i.hasLot && i.candidate < 0) ++unknownTypes;
        inc.push_back(x);
    }
    fc4_crime_shadow_month_in in{};
    in.struct_size = sizeof(in);
    in.geometry = m.geo;
    in.pre = m.pre.data();
    in.hotspot_x = hx;
    in.hotspot_z = hz;
    in.hotspot_value = m.hotBefore.set ? m.hotBefore.value : 0;
    in.incidents = inc.data();
    in.incident_count = static_cast<uint32_t>(inc.size());
    in.zones = m.zonesRead ? m.zones.data() : nullptr;
    std::vector<uint8_t> ours(m.pre.size(), 0);
    fc4_crime_shadow_month_out out{};
    out.struct_size = sizeof(out);
    if (!fc4_crime_shadow_month(g_core, &in, ours.data(), &out)) {
        Log("%s: INCOMPLETE (our replay failed); not compared", prefix);
        return -1;
    }

    // State 4 not in this delivery (the mirror is out of step): the grid did
    // not move although ours did, and no incident appeared.
    const bool origMoved = m.pre != m.post;
    const bool oursMoved = m.pre != ours;
    if (!origMoved && oursMoved && m.arrayLength == 0) {
        ++g_unobserved;
        g_state = kStateUnknown;  // resynchronised at the next month
        Log("%s: state 4 NOT OBSERVED in this delivery (the original's grid and incidents did not change; ours "
            "would have); pipeline mirror out of step, resynchronising at the next SimNewMonth; not compared "
            "(unobserved=%ld)",
            prefix, g_unobserved);
        return -1;
    }

    // Cell by cell.
    int cellMis = 0, maxDiff = 0;
    char cells[1024] = "";
    long listed = 0;
    for (size_t i = 0; i < m.post.size(); ++i) {
        if (m.post[i] == ours[i]) continue;
        ++cellMis;
        const int d = static_cast<int>(m.post[i]) - static_cast<int>(ours[i]);
        if ((d < 0 ? -d : d) > maxDiff) maxDiff = d < 0 ? -d : d;
        if (listed < g_cells) {
            const int32_t x = static_cast<int32_t>(i / static_cast<size_t>(m.geo.count_z));
            const int32_t z = static_cast<int32_t>(i % static_cast<size_t>(m.geo.count_z));
            AppendF(cells, sizeof(cells), "%s(%d,%d) %u/%u pre %u", listed ? ", " : "", x, z, m.post[i], ours[i],
                    m.pre[i]);
            ++listed;
        }
    }
    // Hotspot (§11.2: cells of the tract corner, value low byte, only when > 0).
    const bool ourHotSet = out.hotspot_value > 0;
    const uint8_t ourHotValue = static_cast<uint8_t>(out.hotspot_value);
    const int32_t ourHx = s * out.hotspot_x, ourHz = s * out.hotspot_z;
    const bool hotMis = m.hotAfter.set != ourHotSet ||
                        (ourHotSet && (m.hotAfter.value != ourHotValue || m.hotAfter.x != ourHx || m.hotAfter.z != ourHz));
    const bool countMis = m.count != out.count;
    const uint32_t ourAverage = TruncAverage(out.average_bits);
    const bool averageMis = out.average_computed != 0 && m.average != ourAverage;
    const int mismatches = cellMis + (hotMis ? 1 : 0) + (countMis ? 1 : 0) + (averageMis ? 1 : 0);
    ++g_months;
    if (mismatches == 0) ++g_clean;

    char line[4096] = "";
    _snprintf_s(line, sizeof(line), _TRUNCATE,
                "%s: state 4 compared on %dx%d tracts (S=%d; decay, hotspot, stamps of the original's incidents, "
                "count, average): cells mismatched=%d max|orig-ours|=%d",
                prefix, m.geo.count_x, m.geo.count_z, s, cellMis, maxDiff);
    if (cellMis != 0) {
        AppendF(line, sizeof(line), " [cells (x,z) orig/ours: %s%s]", cells,
                cellMis > listed ? " ..." : "");
    }
    AppendF(line, sizeof(line), "; hotspot orig/ours=");
    AppendHotspot(line, sizeof(line), m.hotAfter.set, m.hotAfter.value, m.hotAfter.x, m.hotAfter.z);
    AppendF(line, sizeof(line), "/");
    AppendHotspot(line, sizeof(line), ourHotSet, ourHotValue, ourHx, ourHz);
    AppendF(line, sizeof(line), "%s (before: ", hotMis ? "!" : "");
    AppendHotspot(line, sizeof(line), m.hotBefore.set, m.hotBefore.value, m.hotBefore.x, m.hotBefore.z);
    AppendF(line, sizeof(line), ", replayed from %s)", hotNote);
    AppendF(line, sizeof(line), "; count orig/ours=%u/%u%s", m.count, out.count, countMis ? "!" : "");
    if (out.average_computed != 0) {
        AppendF(line, sizeof(line), "; average orig/ours=%u/%u%s", m.average, ourAverage, averageMis ? "!" : "");
    } else {
        AppendF(line, sizeof(line), "; average orig=%u (not compared: no zone manager)", m.average);
    }
    AppendF(line, sizeof(line),
            "; incidents=%u (severity sum %u, no lot at the cell %u, unknown types %u, lots not matching the type's "
            "masks %u [information: read after the step])",
            m.arrayLength, out.severity_sum, noLot, out.unknown_types, notCandidates);
    if (!m.arrayRead) AppendF(line, sizeof(line), " [incident list NOT READ: no allocator]");
    AppendF(line, sizeof(line), "; mismatches=%d; months=%ld clean=%ld", mismatches, g_months, g_clean);
    if (g_unobserved != 0) AppendF(line, sizeof(line), " unobserved=%ld", g_unobserved);
    if (g_info.tuning_found == 0) AppendF(line, sizeof(line), "; tuning: code defaults");
    AppendF(line, sizeof(line), "; %s", kNotCompared);
    const char* off = SwitchesOffList();
    if (strcmp(off, "-") != 0) AppendF(line, sizeof(line), "; switches_off=%s", off);
    Log("%s", line);
    g_firstCommit = false;
    return mismatches;
}

// One delivery to the original's agent: the pipeline mirror (§5), and the
// wrapped state-4 delivery. Returns true if it delivered.
bool Deliver(cISC4City* city, cFC4ISC4CrimeSimulator* sim, cIGZMessageTarget2* target, cIGZMessage2* msg,
             uint32_t type, int64_t data1, bool* finished) {
    if (type == kMsgSimNewMonth) {
        if (g_state == kStateIdle || g_state == kStateUnknown) g_state = 0;
        return false;
    }
    if (type == kMsgSimNewDay) {
        const int32_t julian = static_cast<int32_t>(data1);
        // A date that does not follow the last one: another city (or a load).
        if (g_lastJulian != 0 && julian != g_lastJulian && julian != g_lastJulian + 1) {
            Log("crime shadow: date jump (%d -> %d): a new or loaded city; pipeline state 6", g_lastJulian, julian);
            g_state = kStateIdle;
            g_firstCommit = true;
        }
        g_lastJulian = julian;
    }
    if (g_state < 0 || g_state >= kStateIdle) return false;
    const int step = g_state++;
    if (step != kStateCommit) return false;
    g_m = Month{};
    g_m.city = city;
    g_m.sim = sim;
    fc4_date_from_julian(g_lastJulian, &g_m.date);
    EnsureCore();
    const bool beforeClean = SehGuard("crime shadow (before the original's state 4)", &BeforeStep, nullptr);
    *finished = target->DoMessage(msg);  // the original crime simulator, unchanged
    const bool afterClean = SehGuard("crime shadow (after the original's state 4)", &AfterStep, nullptr);
    g_lastResult = Report(beforeClean, afterClean);
    g_m = Month{};
    return true;
}

}  // namespace

void CrimeShadowStartup(bool simulatorReplaced, bool crimeReplaced) {
    ReadSettings();
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    const std::string unused = Setting("crime_shadow", "FC4_CRIME_SHADOW", &src);
    (void)unused;
    if (simulatorReplaced && !crimeReplaced) {
        Log("crime shadow: ON (crime_shadow from %s): the ORIGINAL crime simulator stays active; at the delivery "
            "that runs its pipeline state 4 (tracked from its SimNewMonth / SimNewDay / SimIdle deliveries), its "
            "crime grid, hotspot, monthly count and average are read before and after, and FreeCity4's state-4 grid "
            "work (decay, hotspot, severity stamps of the original's incidents on their lots' rects, zoned average) "
            "is replayed on the grid read before and compared cell by cell; %s; differing cells listed per month: %ld",
            src, kNotCompared, g_cells);
    } else {
        Log("crime shadow: requested (crime_shadow from %s) but INACTIVE: it needs FreeCity4's simulator and the "
            "original crime simulator (do not enable crime); here the simulator is %s and crime is %s",
            src, simulatorReplaced ? "ours" : "the original", crimeReplaced ? "ours" : "the original");
    }
}

bool CrimeShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                        int64_t data1, bool* finished) {
    if ((type != kMsgSimNewDay && type != kMsgSimNewMonth && type != kMsgSimIdle) || target == nullptr ||
        msg == nullptr || finished == nullptr) {
        return false;
    }
    if (!g_testMode && !g_enabled) {
        if (type != kMsgSimNewDay) return false;
        const int32_t julian = static_cast<int32_t>(data1);
        if (julian != g_settingsJulian) {
            g_settingsJulian = julian;
            ReadSettings();  // once per day (FreeCity4.ini may change between city loads)
        }
        if (!g_enabled) return false;
    }
    // Cheap rejection for every other agent.
    if (!g_testMode && target != g_agent && (agentName == nullptr || strcmp(agentName, kAgentName) != 0)) return false;
    cISC4City* city = CurrentCity();
    cFC4ISC4CrimeSimulator* sim = OriginalFor(city, target, agentName);
    if (sim == nullptr) return false;
    return Deliver(city, sim, target, msg, type, data1, finished);
}

}  // namespace fc4

intptr_t CrimeShadowTestRun(const FC4CrimeShadowTestInput* in) {
    if (in == nullptr) return -1;
    fc4::g_testMode = true;
    fc4::g_test = in;
    fc4::g_sim = nullptr;  // each run is a fresh "game"
    fc4::g_agent = nullptr;
    fc4::g_state = fc4::kStateUnknown;
    fc4::DropCore();
    fc4::g_lastResult = -1;
    bool taken = false;
    // SimNewMonth, then SimNewDay until state 4 has been wrapped (states 0-4).
    const uint32_t types[7] = {fc4::kMsgSimNewMonth, fc4::kMsgSimNewDay, fc4::kMsgSimNewDay, fc4::kMsgSimNewDay,
                               fc4::kMsgSimNewDay,   fc4::kMsgSimNewDay, fc4::kMsgSimNewDay};
    int32_t julian = in->julian;
    for (uint32_t t : types) {
        const int64_t data1 = t == fc4::kMsgSimNewDay ? julian++ : in->julian;
        cFC4Message2Standard* msg = FC4MakeMessage(t, static_cast<intptr_t>(data1), 0, 0);
        bool finished = false;
        if (fc4::CrimeShadowDeliver(in->target, nullptr, msg, t, data1, &finished)) {
            taken = true;
        } else {
            in->target->DoMessage(msg);
        }
        msg->Release();
        if (taken) break;
    }
    fc4::DropCore();
    fc4::g_sim = nullptr;
    fc4::g_agent = nullptr;
    fc4::g_testMode = false;
    fc4::g_test = nullptr;
    if (!taken) return -1;
    return fc4::g_lastResult == -1 ? -2 : fc4::g_lastResult;
}
