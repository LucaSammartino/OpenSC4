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

#include "FC4ZonesShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4WriteCapture.h"
#include "cFC4ZoneManager.h"

#include "cGZPersistResourceKey.h"
#include "cISC4City.h"
#include "cISC4ZoneManager.h"

#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

// The interface the §10.1 reader asks the zone grid reference for.
constexpr uint32_t kIID_cISC4SimGrid = 0xC9BD939A;
// The zone grid's class (§2, simgrid §6.4).
constexpr uint32_t kClsZoneGrid = 0x49B9E603;
constexpr int32_t kTypes = 16;
// The core refuses longer sides (fc4.h); a grid beyond it is not recounted.
constexpr int32_t kMaxSide = 4096;
// Differences listed per line.
constexpr unsigned kFirst = 6;

bool g_enabled = false;
bool g_loggedOurs = false;
bool g_loggedNoZoneManager = false;

// The original's record (main thread only; reused).
WriteCapture g_capture;

// ---- the captured record as an fc4_istream ------------------------------------------------------

struct RecordIn {
    const WriteCapture* cap = nullptr;
    size_t pos = 0;
    bool bad = false;
};

bool Take(void* u, void* out, size_t n) {
    RecordIn* r = static_cast<RecordIn*>(u);
    const std::vector<uint8_t>& b = r->cap->Bytes();
    if (r->bad || n > b.size() - r->pos) {
        r->bad = true;
        return false;
    }
    if (out != nullptr && n != 0) memcpy(out, b.data() + r->pos, n);
    r->pos += n;
    return true;
}
bool InU8(void* u, uint8_t* o) { return Take(u, o, 1); }
bool InU16(void* u, uint16_t* o) { return Take(u, o, 2); }
bool InU32(void* u, uint32_t* o) { return Take(u, o, 4); }
bool InU64(void* u, uint64_t* o) { return Take(u, o, 8); }
bool InF32(void* u, float* o) { return Take(u, o, 4); }
bool InF64(void* u, double* o) { return Take(u, o, 8); }
bool InBytes(void* u, uint8_t* o, size_t n) { return Take(u, o, n); }
bool InSkip(void* u, size_t n) { return Take(u, nullptr, n); }
int32_t InError(void* u) { return static_cast<RecordIn*>(u)->bad ? 1 : 0; }

// A string as the capture writes it: uint32 length, then the bytes.
bool InStr(void* u, fc4_bytes_sink sink, void* ctx) {
    uint32_t n = 0;
    if (!Take(u, &n, 4)) return false;
    std::vector<uint8_t> s(n);
    if (!Take(u, n != 0 ? s.data() : nullptr, n)) return false;
    if (sink != nullptr) sink(ctx, s.data(), s.size());
    return true;
}

// A reference: object number (0 = null) then class ID. [choice] Only the
// zone grid is resolved, to its cISC4SimGrid identity (borrowed: the
// original keeps its grid alive across the comparison); the five service
// references read as null: a shadow core never adopts them and reads the
// city's services through its own table.
bool InRef(void* u, uint32_t iid, fc4_ref* out) {
    RecordIn* r = static_cast<RecordIn*>(u);
    *out = fc4_ref{};
    uint32_t id = 0;
    if (!Take(u, &id, 4)) return false;
    if (id == 0) return true;
    uint32_t cls = 0;
    if (!Take(u, &cls, 4)) return false;
    out->class_id = cls;
    if (iid == kIID_cISC4SimGrid) out->token = TokenOf(r->cap->Grid(id));
    return true;
}

fc4_istream MakeIn(RecordIn* in) {
    fc4_istream s{};
    s.struct_size = sizeof(s);
    s.user = in;
    s.get_u8 = &InU8;
    s.get_u16 = &InU16;
    s.get_u32 = &InU32;
    s.get_u64 = &InU64;
    s.get_f32 = &InF32;
    s.get_f64 = &InF64;
    s.get_bytes = &InBytes;
    s.get_str = &InStr;
    s.get_ref = &InRef;
    s.skip = &InSkip;
    s.error = &InError;
    return s;
}

// ---- one run -----------------------------------------------------------------------------------------

void AppendF(std::string& s, const char* fmt, ...) {
    char buf[512];
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(buf, sizeof(buf), _TRUNCATE, fmt, ap);
    va_end(ap);
    s += buf;
}

// The first kFirst differences of a line ("-" when there are none).
struct FirstList {
    std::string text;
    unsigned listed = 0;
    void Add(const char* fmt, ...) {
        if (listed >= kFirst) return;
        char buf[256];
        va_list ap;
        va_start(ap, fmt);
        _vsnprintf_s(buf, sizeof(buf), _TRUNCATE, fmt, ap);
        va_end(ap);
        if (listed != 0) text += ",";
        text += buf;
        ++listed;
    }
    const char* Str() const { return listed == 0 ? "-" : text.c_str(); }
};

struct Run {
    cISC4City* city = nullptr;
    int32_t julian = 0;
    uint32_t event = 0;  // 0 = load (PostCityInit), 1 = month
    const fc4_zones_host* test = nullptr;
    char date[16] = "-";
    intptr_t result = 0;
};

void Line(const Run& r, const char* what, const std::string& body) {
    std::string line;
    AppendF(line, "zones shadow (%s) %s %s: ", what, r.date, r.event == 0 ? "load" : "month");
    line += body;
    LogShadowLine("zones shadow (cont.)", reinterpret_cast<const uint8_t*>(line.data()), line.size());
}

// The references the core's host took for one call (the landfill occupant
// of slot 33), given back when the call returns.
void DropHeld(ZonesContext& c) {
    std::vector<cIGZUnknown*> held;
    held.swap(c.held);
    for (cIGZUnknown* o : held) FC4_RELEASE(o);
}

// The original's answers (§11.1; slots 9 and 18 are not called).
struct Original {
    uint32_t count[kTypes] = {};
    uint32_t undeveloped[kTypes] = {};
    uint32_t total = 0, abandoned = 0;
    bool texOk[kTypes] = {};
    cGZPersistResourceKey tex[kTypes];
    uint32_t drag[kTypes] = {};
    uint8_t alpha = 0, defaultAlpha = 0;
    int32_t minSize[kTypes] = {}, maxSize[kTypes] = {};
    cISC4SimGrid<int8_t>* grid = nullptr;
};

void ReadOriginal(cFC4ISC4ZoneManager* zm, Original& o) {
    o.grid = zm->GetZoneGrid();  // slot 5, no AddRef
    for (int32_t t = 0; t < kTypes; ++t) {
        o.count[t] = zm->GetZoneCount(t);                       // 6
        o.undeveloped[t] = zm->GetUndevelopedTileCountOfType(t);  // 7
        o.texOk[t] = zm->GetTextureForZone(t, o.tex[t]);         // 14
        o.drag[t] = zm->GetZoneDragColor(t);                     // 15
        o.minSize[t] = zm->GetMinZoneSize(t);                    // 19
        o.maxSize[t] = zm->GetMaxZoneSize(t);                    // 20
    }
    o.total = zm->GetUndevelopedTileCount();        // 8
    o.abandoned = zm->GetAbandonedTileCount();      // 10
    o.alpha = zm->GetZoneDisplayAlpha();            // 16
    o.defaultAlpha = zm->GetDefaultZoneDisplayAlpha();  // 17
}

// The dry-run samples: size, mask and PlaceZone flags (skipFunds, checkSlope,
// skipSizeCheck, checkLotInside). [choice] (ZG15: the original's callers in
// the spec all commit; the UI tool was not traced) These mix the user-action
// form (checkSlope, with and without the size check) with the BDS's
// DoTerrainChanged form (skipFunds, size check skipped, lot inside) and a
// call with no flag.
struct Sample {
    int32_t w, h;
    bool checker;
    bool skipFunds, checkSlope, skipSize, lotInside;
};
constexpr Sample kSamples[] = {
    {1, 1, false, false, true, false, false},
    {2, 3, false, false, true, true, true},
    {4, 4, false, true, false, true, true},
    {7, 3, true, false, false, false, false},
};
constexpr uint32_t kSampleCount = sizeof(kSamples) / sizeof(kSamples[0]);

// Positions from the date: xorshift32 (no game random number is drawn).
uint32_t NextRandom(uint32_t& s) {
    s ^= s << 13;
    s ^= s >> 17;
    s ^= s << 5;
    return s;
}

void RunStep(void* p) {
    Run& r = *static_cast<Run*>(p);
    cISC4ZoneManager* raw = r.city->GetZoneManager();  // city slot 36, borrowed
    if (raw == nullptr) {
        if (!g_loggedNoZoneManager) {
            g_loggedNoZoneManager = true;
            Log("zones shadow %s: the city has no zone manager; nothing compared", r.date);
        }
        r.result = -1;
        return;
    }
    if (cFC4ZoneManager::FromPrimary(raw) != nullptr) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("zones shadow %s: the city's zone manager is FreeCity4's (enable = zones); the shadow compares the "
                "ORIGINAL: nothing compared",
                r.date);
        }
        r.result = -2;
        return;
    }
    cFC4ISC4ZoneManager* zm = AsGame(raw);
    Original o;
    ReadOriginal(zm, o);

    // The core's host: the city's services, borrowed (the test seam: the
    // test table instead, and no city).
    ZonesContext ctx;
    ctx.grid = HostGridContext{"zones shadow", Sw::ZonesGridWrites};
    if (r.test != nullptr) {
        ctx.test = r.test;
    } else {
        ctx.city = r.city;
        ctx.occupants = r.city->GetOccupantManager();
        ctx.terrain = r.city->GetTerrain();
        ctx.pollution = r.city->GetPollutionSimulator();
        ctx.budget = r.city->GetBudgetSimulator();
    }
    const fc4_zones_host host = MakeZonesReadOnlyHost(&ctx);
    const fc4_grid_api& api = host.grids;

    // The grid and its recount.
    const uint64_t gridToken = TokenOf(o.grid);
    fc4_grid_geometry geom{};
    uint32_t gridClass = 0;
    if (gridToken != 0) {
        gridClass = api.class_of(api.user, gridToken);
        api.geometry(api.user, gridToken, &geom);
    }
    const bool gridOk = gridToken != 0 && gridClass == kClsZoneGrid && geom.count_x > 0 && geom.count_z > 0 &&
                        geom.count_x <= kMaxSide && geom.count_z <= kMaxSide;
    uint32_t recount[kTypes] = {};
    uint32_t other = 0;
    if (gridOk) {
        for (int32_t x = 0; x < geom.count_x; ++x) {
            for (int32_t z = 0; z < geom.count_z; ++z) {
                const int v = static_cast<int>(api.get(api.user, gridToken, kClsZoneGrid, x, z));
                if (v >= 0 && v < kTypes) {
                    ++recount[v];
                } else {
                    ++other;
                }
            }
        }
    }

    // The core on the original's own record (§10.1), then Init (§1.1, §4).
    bool answered = false;
    const bool wrote = g_capture.Capture(reinterpret_cast<cIGZUnknown*>(raw), &answered);
    fc4_zones* s = nullptr;
    const char* why = nullptr;
    size_t unread = 0;
    if (!wrote) {
        why = answered ? "the original's Write failed" : "the original does not answer cIGZSerializable";
    } else {
        s = fc4_zones_create_shadow(&host);
        if (s == nullptr) {
            why = "the shadow core was not created";
        } else {
            RecordIn in;
            in.cap = &g_capture;
            const fc4_istream is = MakeIn(&in);
            const bool read = fc4_zones_read(s, &is);
            DropHeld(ctx);
            unread = g_capture.Bytes().size() - in.pos;
            if (!read) {
                why = "the core refused the original's record";
                fc4_zones_destroy(s);
                s = nullptr;
            } else {
                fc4_zones_init(s);
                DropHeld(ctx);
            }
        }
    }
    bool incomplete = false;
    intptr_t total = 0;

    // 1. Counts: the original's slot 6, the recount and the core's.
    if (!gridOk) {
        std::string body;
        AppendF(body, "INCOMPLETE (zone grid %p: class 0x%08X, %d x %d); not compared", static_cast<void*>(o.grid),
                gridClass, geom.count_x, geom.count_z);
        Line(r, "counts", body);
        incomplete = true;
    } else {
        uint32_t mismatches = 0;
        FirstList first;
        std::string counts;
        for (int32_t t = 0; t < kTypes; ++t) {
            const uint32_t ours = s != nullptr ? fc4_zones_zone_count(s, t) : o.count[t];
            // zones.md 10.1: loaded counts survive Init; they are not rebuilt
            // from the grid. The native recount is diagnostic, not slot 6.
            const bool differ = ours != o.count[t];
            if (differ) {
                ++mismatches;
                if (s != nullptr) {
                    first.Add("%d:%u/%u/%u", t, o.count[t], recount[t], ours);
                } else {
                    first.Add("%d:%u/%u/-", t, o.count[t], recount[t]);
                }
            }
            if (o.count[t] != 0 || recount[t] != 0 || ours != 0) {
                if (s != nullptr) {
                    AppendF(counts, " %d:%u/%u/%u", t, o.count[t], recount[t], ours);
                } else {
                    AppendF(counts, " %d:%u/%u/-", t, o.count[t], recount[t]);
                }
            }
        }
        const char* gridRef = "-";
        if (s != nullptr) {
            const bool same = fc4_zones_grid(s) == gridToken;
            gridRef = same ? "same" : "DIFFERENT";
            if (!same) ++mismatches;
        }
        std::string body;
        AppendF(body, "grid=%dx%d cells=%u other=%u gridRef=%s mismatches=%u orig/recount/ours:",
                geom.count_x, geom.count_z, static_cast<unsigned>(geom.count_x * geom.count_z), other, gridRef,
                mismatches);
        body += counts.empty() ? " -" : counts;
        AppendF(body, "; first=%s", first.Str());
        if (s == nullptr) AppendF(body, "; ours not compared (%s)", why);
        Line(r, "counts", body);
        total += mismatches;
        if (s == nullptr) incomplete = true;
    }

    if (s == nullptr) {
        std::string body;
        AppendF(body, "INCOMPLETE (%s); not compared", why != nullptr ? why : "no core");
        Line(r, "bookkeeping", body);
        Line(r, "tuning", body);
        Line(r, "PlaceZone", body);
        r.result = -3;
        return;
    }

    // 2. Bookkeeping: slots 7, 8 and 10 (never updated after Init, §2) and
    //    the record read to its end.
    {
        uint32_t mismatches = 0;
        FirstList first;
        for (int32_t t = 0; t < kTypes; ++t) {
            const uint32_t ours = fc4_zones_undeveloped_tile_count(s, t);
            if (ours != o.undeveloped[t]) {
                ++mismatches;
                first.Add("undeveloped[%d]=%u/%u", t, o.undeveloped[t], ours);
            }
        }
        const uint32_t oursTotal = fc4_zones_undeveloped_total(s);
        const uint32_t oursAbandoned = fc4_zones_abandoned_tile_count(s);
        if (oursTotal != o.total) ++mismatches;
        if (oursAbandoned != o.abandoned) ++mismatches;
        if (unread != 0) ++mismatches;
        std::string body;
        AppendF(body,
                "mismatches=%u undeveloped[0]=%u/%u total=%u/%u abandoned=%u/%u (orig/ours) record=%u bytes "
                "unread=%u; first=%s",
                mismatches, o.undeveloped[0], fc4_zones_undeveloped_tile_count(s, 0), o.total, oursTotal, o.abandoned,
                oursAbandoned, static_cast<unsigned>(g_capture.Bytes().size()), static_cast<unsigned>(unread),
                first.Str());
        Line(r, "bookkeeping", body);
        total += mismatches;
    }

    // 3. Tuning: slots 14-17, 19 and 20 after Init's exemplar read (§4).
    {
        uint32_t mismatches = 0;
        FirstList first;
        for (int32_t t = 0; t < kTypes; ++t) {
            uint32_t key[3] = {};
            const bool ok = fc4_zones_texture_for_zone(s, t, key);
            if (ok != o.texOk[t] || (ok && (key[0] != o.tex[t].type || key[1] != o.tex[t].group ||
                                            key[2] != o.tex[t].instance))) {
                ++mismatches;
                first.Add("texture[%d]=%u:0x%08X/%u:0x%08X", t, o.texOk[t] ? 1u : 0u, o.tex[t].instance, ok ? 1u : 0u,
                          key[2]);
            }
            const uint32_t drag = fc4_zones_drag_color(s, t);
            if (drag != o.drag[t]) {
                ++mismatches;
                first.Add("drag[%d]=0x%08X/0x%08X", t, o.drag[t], drag);
            }
            const int32_t mn = fc4_zones_min_size(s, t);
            if (mn != o.minSize[t]) {
                ++mismatches;
                first.Add("min[%d]=%d/%d", t, o.minSize[t], mn);
            }
            const int32_t mx = fc4_zones_max_size(s, t);
            if (mx != o.maxSize[t]) {
                ++mismatches;
                first.Add("max[%d]=%d/%d", t, o.maxSize[t], mx);
            }
        }
        const uint8_t alpha = fc4_zones_display_alpha(s);
        const uint8_t defaultAlpha = fc4_zones_default_display_alpha(s);
        if (alpha != o.alpha) ++mismatches;
        if (defaultAlpha != o.defaultAlpha) ++mismatches;
        std::string body;
        AppendF(body, "values=%d mismatches=%u alpha=%u/%u default=%u/%u (orig/ours); first=%s", kTypes * 4 + 2,
                mismatches, o.alpha, alpha, o.defaultAlpha, defaultAlpha, first.Str());
        Line(r, "tuning", body);
        total += mismatches;
    }

    // 4. PlaceZone dry runs (commit = false, outLots null).
    {
        uint32_t seed = static_cast<uint32_t>(r.julian) * 2654435761u ^ (r.event == 0 ? 0x9E3779B9u : 0x7F4A7C15u);
        if (seed == 0) seed = 1;
        uint32_t calls = 0, mismatches = 0, okOrig = 0, okOurs = 0, clearedOrig = 0, clearedOurs = 0;
        int64_t costOrig = 0, costOurs = 0;
        FirstList first;
        std::string regions;
        for (uint32_t i = 0; i < kSampleCount; ++i) {
            const Sample& sm = kSamples[i];
            const int32_t w = sm.w < geom.count_x ? sm.w : geom.count_x;
            const int32_t h = sm.h < geom.count_z ? sm.h : geom.count_z;
            const int32_t x1 = static_cast<int32_t>(NextRandom(seed) % static_cast<uint32_t>(geom.count_x - w + 1));
            const int32_t z1 = static_cast<int32_t>(NextRandom(seed) % static_cast<uint32_t>(geom.count_z - h + 1));
            const int32_t x2 = x1 + w - 1, z2 = z1 + h - 1;
            AppendF(regions, "%s%d,%d..%d,%d%s", i == 0 ? "" : " ", x1, z1, x2, z2, sm.checker ? "(checker)" : "");
            for (int32_t t = 0; t < kTypes; ++t) {
                // The same cells for both: row-major by z for the core, the
                // §11.4 layout (our memory) for the original.
                std::vector<uint8_t> mask(static_cast<size_t>(w) * static_cast<size_t>(h));
                uint32_t set = 0;
                for (int32_t dz = 0; dz < h; ++dz) {
                    for (int32_t dx = 0; dx < w; ++dx) {
                        const bool on = !sm.checker || ((dx + dz) % 2) == 0;
                        mask[static_cast<size_t>(dz) * w + dx] = on ? 1 : 0;
                        set += on ? 1u : 0u;
                    }
                }
                fc4_zone_region cr{x1, z1, x2, z2, mask.data(), mask.size()};
                ZoneRegionBuffer rb;
                if (!rb.Build(cr)) continue;
                int64_t oc = 0;
                int32_t oe = 0;
                const bool ook = zm->PlaceZone(rb.Game(), t, false, sm.skipFunds, sm.checkSlope, sm.skipSize,
                                               sm.lotInside, &oc, &oe, nullptr);  // slot 11
                fc4_zones_place_result pr{};
                pr.struct_size = sizeof(pr);
                fc4_zones_place_zone(s, &cr, t, false, sm.skipFunds, sm.checkSlope, sm.skipSize, sm.lotInside,
                                     nullptr, nullptr, &pr);
                DropHeld(ctx);
                ++calls;
                uint32_t bitDiff = 0, origSet = 0, oursSet = 0;
                for (int32_t dz = 0; dz < h; ++dz) {
                    for (int32_t dx = 0; dx < w; ++dx) {
                        const bool ob = rb.Bit(x1 + dx, z1 + dz);
                        const bool ub = mask[static_cast<size_t>(dz) * w + dx] != 0;
                        origSet += ob ? 1u : 0u;
                        oursSet += ub ? 1u : 0u;
                        if (ob != ub) ++bitDiff;
                    }
                }
                okOrig += ook ? 1u : 0u;
                okOurs += pr.ok != 0 ? 1u : 0u;
                costOrig += oc;
                costOurs += pr.cost;
                clearedOrig += set - origSet;
                clearedOurs += set - oursSet;
                if (ook != (pr.ok != 0) || oe != pr.error || oc != pr.cost || bitDiff != 0) {
                    ++mismatches;
                    first.Add("r%u t%d ok %d/%d err %d/%d cost %lld/%lld bits %u", i, t, ook ? 1 : 0, pr.ok, oe,
                              pr.error, static_cast<long long>(oc), static_cast<long long>(pr.cost), bitDiff);
                }
            }
        }
        std::string body;
        AppendF(body,
                "calls=%u mismatches=%u ok=%u/%u cost=%lld/%lld cleared=%u/%u (orig/ours) regions=%s; first=%s",
                calls, mismatches, okOrig, okOurs, static_cast<long long>(costOrig), static_cast<long long>(costOurs),
                clearedOrig, clearedOurs, regions.c_str(), first.Str());
        Line(r, "PlaceZone", body);
        total += mismatches;
    }

    fc4_zones_shutdown(s);
    fc4_zones_destroy(s);
    DropHeld(ctx);
    r.result = incomplete ? -3 : total;
}

intptr_t RunOnce(cISC4City* city, int32_t julian, uint32_t event, const fc4_zones_host* test) {
    Run r;
    r.city = city;
    r.julian = julian;
    r.event = event;
    r.test = test;
    if (julian > 0) {
        fc4_date d{};
        fc4_date_from_julian(julian, &d);
        _snprintf_s(r.date, sizeof(r.date), _TRUNCATE, "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
    }
    if (city == nullptr) return -1;
    if (!SehGuard("zones shadow", &RunStep, &r)) {
        Log("zones shadow %s: WARNING structured exception; comparison abandoned", r.date);
        return -3;
    }
    return r.result;
}

}  // namespace

void ZonesShadowStartup(bool zonesReplaced) {
    g_enabled = SettingInt("zones_shadow", "FC4_ZONES_SHADOW", 0) != 0;
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("zones_shadow", "FC4_ZONES_SHADOW", &src);
    if (zonesReplaced) {
        Log("zones shadow: requested (zones_shadow from %s) but INACTIVE: it compares the ORIGINAL zone manager, and "
            "`enable = zones` replaces it",
            src);
        g_enabled = false;
        return;
    }
    Log("zones shadow: ON (zones_shadow from %s): at PostCityInit and each SimNewMonth, the original zone manager's "
        "16 zone counts (slot 6) against a recount of its zone grid (slot 5) and against FreeCity4's core loaded "
        "from the original's own record (its Write into memory, the grid by reference); slots 7, 8, 10 and the "
        "tuning (slots 14-17, 19, 20) against the core after Init; and %u sample regions x 16 zone types of "
        "PlaceZone dry runs (commit = false: ok, error, cost, cleared bits)",
        src, static_cast<unsigned>(kSampleCount));
    Log("zones shadow: NOT readable read-only (so not compared): the development-failure maps (slot 9 inserts "
        "entries), the creation and destruction costs and max slopes (only through PlaceZone's cost and "
        "rejections)");
}

void ZonesShadowCityLoaded(cISC4City* city) {
    if (!g_enabled || city == nullptr) return;
    cFC4ISC4Simulator* sim = AsGame(city->GetSimulator());
    const int32_t julian = sim != nullptr ? sim->GetSimDateNumber() : 0;
    RunOnce(city, julian, 0, nullptr);
}

void ZonesShadowMonth(cISC4City* city, int32_t julian) {
    if (!g_enabled || city == nullptr) return;
    RunOnce(city, julian, 1, nullptr);
}

intptr_t ZonesShadowTestRun(cISC4City* city, int32_t julian, uint32_t event, const fc4_zones_host* testHost) {
    return RunOnce(city, julian, event, testHost);
}

}  // namespace fc4
