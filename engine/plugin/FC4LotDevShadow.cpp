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

#include "FC4LotDevShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4LotDevInterfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4TractInterfaces.h"
#include "FC4WriteCapture.h"
#include "cFC4LotDeveloper.h"

#include "SC4Rect.h"
#include "SC4Vector.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4Demolition.h"
#include "cISC4FloraSimulator.h"
#include "cISC4FoundationOccupant.h"
#include "cISC4Lot.h"
#include "cISC4LotConfiguration.h"
#include "cISC4LotDeveloper.h"
#include "cISC4LotManager.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4PlumbingSimulator.h"
#include "cISC4PowerSimulator.h"
#include "cISC4PropManager.h"
#include "cISC4TrafficSimulator.h"
#include "cISTETerrain.h"
#include "cS3DVector3.h"
#include "cSC4LotConfigurationObject.h"

#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

// Lots compared per run, and the configuration objects per lot (slot 39).
constexpr uint32_t kSampleLots = 64;
constexpr uint32_t kSampleObjects = 8;
// Differences listed per line.
constexpr unsigned kFirst = 6;
constexpr uint32_t kIID_cIGZUnknown = 0x00000001;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
// The night deque's buildings in the shadow core: object number | this (never
// a pointer of the 32-bit process; the read-only table never dereferences them).
constexpr uint64_t kNightTokenBase = 1ull << 40;

bool g_enabled = false;
bool g_loggedOurs = false;
bool g_loggedNone = false;

// The original's record (main thread only; reused).
WriteCapture g_capture;

// ---- the captured record as an fc4_istream, and an fc4_ostream into memory ---------------------------

struct RecordIn {
    const std::vector<uint8_t>* bytes = nullptr;
    size_t pos = 0;
    bool bad = false;
};

bool Take(void* u, void* out, size_t n) {
    RecordIn* r = static_cast<RecordIn*>(u);
    const std::vector<uint8_t>& b = *r->bytes;
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
// The lot developer's record has no strings (§13).
bool InStr(void* u, fc4_bytes_sink, void*) {
    static_cast<RecordIn*>(u)->bad = true;
    return false;
}
// A reference as WriteCapture holds it: the object number (0 = null), then
// its class ID. The core gets the number as a token (kNightTokenBase | n).
bool InRef(void* u, uint32_t, fc4_ref* out) {
    *out = fc4_ref{};
    uint32_t id = 0;
    if (!InU32(u, &id)) return false;
    if (id == 0) return true;
    uint32_t cls = 0;
    if (!InU32(u, &cls)) return false;
    out->token = kNightTokenBase | id;
    out->class_id = cls;
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

struct RecordOut {
    std::vector<uint8_t> bytes;
    bool bad = false;
};

bool Put(void* u, const void* p, size_t n) {
    RecordOut* r = static_cast<RecordOut*>(u);
    const uint8_t* b = static_cast<const uint8_t*>(p);
    r->bytes.insert(r->bytes.end(), b, b + n);
    return true;
}
bool OutU8(void* u, uint8_t v) { return Put(u, &v, 1); }
bool OutU16(void* u, uint16_t v) { return Put(u, &v, 2); }
bool OutU32(void* u, uint32_t v) { return Put(u, &v, 4); }
bool OutU64(void* u, uint64_t v) { return Put(u, &v, 8); }
bool OutF32(void* u, float v) { return Put(u, &v, 4); }
bool OutF64(void* u, double v) { return Put(u, &v, 8); }
bool OutBytes(void* u, const uint8_t* d, size_t n) { return Put(u, d, n); }
bool OutStr(void* u, const uint8_t*, size_t) {
    static_cast<RecordOut*>(u)->bad = true;
    return false;
}
// The token back to its object number, then the class ID the original wrote.
bool OutRef(void* u, const fc4_ref* r) {
    if (r == nullptr || r->token == 0) return OutU32(u, 0);
    if ((r->token & ~0xFFFFFFFFull) != kNightTokenBase) {
        static_cast<RecordOut*>(u)->bad = true;
        return false;
    }
    const uint32_t id = static_cast<uint32_t>(r->token & 0xFFFFFFFFull);
    OutU32(u, id);
    return OutU32(u, g_capture.ClassOf(id));
}
int32_t OutError(void* u) { return static_cast<RecordOut*>(u)->bad ? 1 : 0; }

fc4_ostream MakeOut(RecordOut* out) {
    fc4_ostream s{};
    s.struct_size = sizeof(s);
    s.user = out;
    s.put_u8 = &OutU8;
    s.put_u16 = &OutU16;
    s.put_u32 = &OutU32;
    s.put_u64 = &OutU64;
    s.put_f32 = &OutF32;
    s.put_f64 = &OutF64;
    s.put_bytes = &OutBytes;
    s.put_str = &OutStr;
    s.put_ref = &OutRef;
    s.error = &OutError;
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
    const fc4_lotdev_host* test = nullptr;
    char date[16] = "-";
    intptr_t result = 0;
};

void Line(const Run& r, const std::string& body) {
    std::string line;
    AppendF(line, "lotdev shadow %s %s: ", r.date, r.event == 0 ? "load" : "month");
    line += body;
    LogShadowLine("lotdev shadow (cont.)", reinterpret_cast<const uint8_t*>(line.data()), line.size());
}

// The object's identity (cIGZUnknown), for comparing two interfaces of it.
const void* Identity(const void* p) {
    if (p == nullptr) return nullptr;
    cIGZUnknown* u = static_cast<cIGZUnknown*>(const_cast<void*>(p));
    void* q = nullptr;
    if (!u->QueryInterface(kIID_cIGZUnknown, &q) || q == nullptr) return p;
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(q), "QueryInterface");
    FC4_RELEASE(static_cast<cIGZUnknown*>(q));
    return q;
}

uint32_t Bits(float f) {
    uint32_t b = 0;
    memcpy(&b, &f, sizeof(b));
    return b;
}

// One lot's comparisons.
struct Compare {
    cFC4ISC4LotDeveloper* orig = nullptr;
    fc4_lotdev* core = nullptr;
    LotDevContext* ctx = nullptr;
    float cellWidth = 16.0f;
    uint32_t compared = 0, mismatches = 0;
    uint32_t rawTruth = 0;  // slot 22 AL values other than 0 / 1 (compared as true)
    FirstList first;

    void Float(const char* what, int32_t x, int32_t z, float o, float c) {
        ++compared;
        if (Bits(o) == Bits(c)) return;
        ++mismatches;
        first.Add("%s@%d,%d orig %.9g ours %.9g", what, x, z, static_cast<double>(o), static_cast<double>(c));
    }
    void Int(const char* what, int32_t x, int32_t z, int64_t o, int64_t c) {
        ++compared;
        if (o == c) return;
        ++mismatches;
        first.Add("%s@%d,%d orig %lld ours %lld", what, x, z, static_cast<long long>(o), static_cast<long long>(c));
    }
    void Ptr(const char* what, int32_t x, int32_t z, const void* o, const void* c) {
        ++compared;
        if (o == c) return;
        ++mismatches;
        first.Add("%s@%d,%d orig %p ours %p", what, x, z, o, c);
    }
};

void CompareLot(Compare& k, cISC4Lot* lot) {
    cFC4ISC4Lot* l = AsGame(lot);
    cFC4ISC4LotDeveloper* o = k.orig;
    fc4_lotdev* c = k.core;
    const uint64_t lt = TokenOf(lot);
    int32_t x = 0, z = 0;
    l->GetLocation(x, z);  // slot 21
    SC4Rect<int32_t> r;
    l->GetBoundingRect(r);  // slot 14
    const fc4_cell_rect cr{r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY};
    const float w = k.cellWidth;
    const SC4Rect<float> rf(r.topLeftX * w, r.topLeftY * w, (r.bottomRightX + 1) * w, (r.bottomRightY + 1) * w);
    const float rf4[4] = {rf.topLeftX, rf.topLeftY, rf.bottomRightX, rf.bottomRightY};

    for (int wi = 0; wi < 2; ++wi) {
        const bool water = wi != 0;
        k.Float(water ? "s15w" : "s15", x, z, o->GetTerrainAltitudeByVertex(r.topLeftX, r.topLeftY, water),
                fc4_lotdev_terrain_altitude_by_vertex(c, r.topLeftX, r.topLeftY, water));
        for (int li = 0; li < 2; ++li) {
            const bool lowest = li != 0;
            k.Float(water ? (lowest ? "s16lw" : "s16w") : (lowest ? "s16l" : "s16"), x, z,
                    o->GetAreaAltitudeF(rf, lowest, water), fc4_lotdev_area_altitude_f(c, rf4, lowest, water));
            k.Float(water ? (lowest ? "s17lw" : "s17w") : (lowest ? "s17l" : "s17"), x, z,
                    o->GetAreaAltitudeI(r, lowest, water), fc4_lotdev_area_altitude(c, &cr, lowest, water));
        }
        k.Float(water ? "s18w" : "s18", x, z, o->GetAverageAltitude(r, water),
                fc4_lotdev_average_area_altitude(c, &cr, water));
        {
            const bool of = o->IsAreaFlat(r, water);
            const bool cf = fc4_lotdev_is_area_flat(c, &cr, water);
            const uint32_t before = k.mismatches;
            k.Int(water ? "s25w" : "s25", x, z, of, cf);
            if (k.mismatches != before) {
                // The ranges behind a flatness difference (slot 27 both sides, the rectangle).
                k.first.Add("s25r@%d,%d rect %d,%d,%d,%d orig %.9g ours %.9g", x, z, r.topLeftX, r.topLeftY,
                            r.bottomRightX, r.bottomRightY, static_cast<double>(o->GetAreaAltitudeRange(r, water)),
                            static_cast<double>(fc4_lotdev_area_altitude_range(c, &cr, water)));
            }
        }
        k.Int(water ? "s26w" : "s26", x, z, o->IsLotFlat(lot, water), fc4_lotdev_is_lot_flat(c, lt, water));
        k.Float(water ? "s27w" : "s27", x, z, o->GetAreaAltitudeRange(r, water),
                fc4_lotdev_area_altitude_range(c, &cr, water));
        k.Float(water ? "s28w" : "s28", x, z, o->GetLotAltitudeRange(lot, water),
                fc4_lotdev_lot_altitude_range(c, lt, water));
        k.Float(water ? "s29w" : "s29", x, z, o->GetAreaSlope(r, water), fc4_lotdev_area_slope(c, &cr, water));
        k.Float(water ? "s30w" : "s30", x, z, o->GetLotSlope(lot, water), fc4_lotdev_lot_slope(c, lt, water));
    }
    k.Int("s32", x, z, o->GetLotDevelopmentCost(r), fc4_lotdev_lot_development_cost(c, &cr));

    // Slot 40.
    {
        float op[3] = {0, 0, 0}, cp[3] = {0, 0, 0};
        int32_t oo = 0, co = 0;
        const bool oHas = o->GetBuildingPosition(lot, op, oo) != nullptr;
        const bool cHas = fc4_lotdev_position_for_building(c, lt, cp, &co);
        k.Int("s40", x, z, oHas, cHas);
        if (oHas && cHas) {
            k.Float("s40x", x, z, op[0], cp[0]);
            k.Float("s40y", x, z, op[1], cp[1]);
            k.Float("s40z", x, z, op[2], cp[2]);
            k.Int("s40o", x, z, oo, co);
        }
    }

    // Slot 39 on the first objects of the lot's configuration.
    if (cISC4LotConfiguration* cfg = l->GetLotConfiguration()) {  // slot 9, no reference
        std::vector<uint32_t> ids;
        for (uint32_t i = 0; i < kSampleObjects; ++i) {
            cSC4LotConfigurationObject* obj = LotDevObjectAt(cfg, i);
            if (obj == nullptr) break;
            const fc4_lotdev_cfg_object co = LotDevObjectOf(obj, ids);
            float op[3] = {0, 0, 0}, cp[3] = {0, 0, 0};
            int32_t oo = 0;
            o->GetObjectPosition(lot, cfg, obj, op, oo);
            const int32_t cor = fc4_lotdev_object_position(c, lt, TokenOf(cfg), &co, cp);
            k.Float("s39x", x, z, op[0], cp[0]);
            k.Float("s39y", x, z, op[1], cp[1]);
            k.Float("s39z", x, z, op[2], cp[2]);
            k.Int("s39o", x, z, oo, cor);
        }
    }

    // Slot 51.
    k.Ptr("s51", x, z, Identity(o->GetLotFoundation(lot)),
          Identity(FromToken(fc4_lotdev_lot_foundation(c, lt))));

    // The lot's building: slots 22, 31, 48, 56, 57.
    cISC4BuildingOccupant* b = l->GetBuilding();  // slot 48, borrowed
    void* p = nullptr;
    if (b == nullptr || !b->QueryInterface(kIID_cISC4Occupant, &p) || p == nullptr) return;
    cISC4Occupant* occ = static_cast<cISC4Occupant*>(p);
    FC4_ACQUIRED(occ, "QueryInterface");
    const uint64_t ot = TokenOf(occ);
    {
        SC4Rect<int32_t> orr;
        fc4_cell_rect crr{0, 0, 0, 0};
        // The original's AL is the x87 status flags its second PositionToCell leaves (section 17.2,
        // LD18 / LD27), not a result: only the rectangle is compared. rawAL still counts the bytes
        // other than 0 / 1 (information only).
        using CoverageFn = uint8_t(__thiscall*)(void*, cISC4Occupant*, SC4Rect<int32_t>&);
        const CoverageFn coverage = (*reinterpret_cast<CoverageFn* const*>(o))[22];
        const uint8_t oRaw = coverage(o, occ, orr);
        fc4_lotdev_occupant_cell_coverage(c, ot, &crr);
        if (oRaw > 1) ++k.rawTruth;
        k.Int("s22x1", x, z, orr.topLeftX, crr.x1);
        k.Int("s22z1", x, z, orr.topLeftY, crr.z1);
        k.Int("s22x2", x, z, orr.bottomRightX, crr.x2);
        k.Int("s22z2", x, z, orr.bottomRightY, crr.z2);
    }
    k.Float("s31", x, z, o->GetOccupantSlope(occ, false), fc4_lotdev_occupant_slope(c, ot, false));
    k.Float("s31w", x, z, o->GetOccupantSlope(occ, true), fc4_lotdev_occupant_slope(c, ot, true));
    k.Ptr("s48", x, z, Identity(o->GetFoundationByOccupant(occ)),
          Identity(FromToken(fc4_lotdev_foundation_by_occupant(c, ot))));
    k.Int("s56", x, z, o->IsOccupantFoundationRequired(occ), fc4_lotdev_is_occupant_foundation_required(c, ot));
    {
        cS3DVector3 mn(0.0f, 0.0f, 0.0f), mx(0.0f, 0.0f, 0.0f);
        occ->GetBoundingBox(&mn, &mx);  // slot 10
        float a[3] = {mn.fX, mn.fY, mn.fZ}, bb[3] = {mx.fX, mx.fY, mx.fZ};
        float a2[3] = {mn.fX, mn.fY, mn.fZ}, bb2[3] = {mx.fX, mx.fY, mx.fZ};
        k.Int("s57", x, z, o->IsBoxFoundationRequired(a, bb), fc4_lotdev_is_box_foundation_required(c, a2, bb2));
    }
    FC4_RELEASE(occ);
}

using LotList = GameList<cISC4Lot*, TractNodeAllocator>;

void RunStep(void* p) {
    Run& r = *static_cast<Run*>(p);
    cISC4LotDeveloper* raw = r.city->GetLotDeveloper();  // borrowed
    if (raw == nullptr) {
        if (!g_loggedNone) {
            g_loggedNone = true;
            Log("lotdev shadow %s: the city has no lot developer; nothing compared", r.date);
        }
        r.result = -1;
        return;
    }
    if (cFC4LotDeveloper::FromPrimary(raw) != nullptr) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("lotdev shadow %s: the city's lot developer is FreeCity4's (enable = lotdev); the shadow compares the "
                "ORIGINAL: nothing compared",
                r.date);
        }
        r.result = -2;
        return;
    }
    cFC4ISC4LotDeveloper* orig = AsGame(raw);

    // The shadow core over the city's borrowed services.
    LotDevContext ctx;
    ctx.test = r.test;
    ctx.city = r.city;
    ctx.lots = r.city->GetLotManager();
    ctx.occupants = r.city->GetOccupantManager();
    ctx.props = r.city->GetPropManager();
    ctx.bds = r.city->GetBuildingDevelopmentSimulator();
    ctx.power = r.city->GetPowerSimulator();
    ctx.plumbing = r.city->GetPlumbingSimulator();
    ctx.flora = r.city->GetFloraSimulator();
    ctx.terrain = r.city->GetTerrain();
    ctx.traffic = r.city->GetTrafficSimulator();
    ctx.demolition = r.city->GetDemolitionUtility();
    const fc4_lotdev_host host = MakeLotDevReadOnlyHost(&ctx);
    fc4_lotdev* core = fc4_lotdev_create(&host, LotDevStatics());

    bool answered = false;
    const bool captured = g_capture.Capture(reinterpret_cast<cIGZUnknown*>(raw), &answered);
    std::string body;
    if (core == nullptr || !captured) {
        AppendF(body, "INCOMPLETE (%s)",
                core == nullptr ? "no shadow core"
                : !answered     ? "no cIGZSerializable"
                                : "Write failed");
        Line(r, body);
        if (core != nullptr) fc4_lotdev_destroy(core);
        LotDevReleaseHeld(&ctx);
        LotDevReleaseServices(&ctx);
        r.result = -3;
        return;
    }

    // 1. Read (the original's record), then Init (§1.2), then Write.
    const std::vector<uint8_t>& rec = g_capture.Bytes();
    RecordIn in;
    in.bytes = &rec;
    const fc4_istream is = MakeIn(&in);
    const bool readOk = fc4_lotdev_read(core, &is);
    LotDevReleaseHeld(&ctx);
    fc4_lotdev_init(core);  // false at §1.3 step 8 (no simulator in the read-only table)
    LotDevReleaseHeld(&ctx);
    RecordOut out;
    const fc4_ostream os = MakeOut(&out);
    const bool writeOk = fc4_lotdev_write(core, &os);
    const bool same = readOk && writeOk && !out.bad && in.pos == rec.size() && out.bytes == rec;
    Compare k;
    k.orig = orig;
    k.core = core;
    k.ctx = &ctx;
    k.cellWidth = r.city->CellWidthX();
    ++k.compared;
    if (!same) {
        ++k.mismatches;
        size_t at = 0;
        while (at < rec.size() && at < out.bytes.size() && rec[at] == out.bytes[at]) ++at;
        k.first.Add("record read=%d write=%d used=%zu/%zu ours=%zu first-diff@%zu", readOk ? 1 : 0, writeOk ? 1 : 0,
                    in.pos, rec.size(), out.bytes.size(), at);
    }
    // The record's night state against the core's.
    fc4_lotdev_state st{};
    st.struct_size = sizeof(st);
    fc4_lotdev_get_state(core, &st);
    CaptureReader cr(rec);
    uint16_t version = 0;
    int32_t vertsX = 0;
    float extentX = 0.0f, extentZ = 0.0f;
    uint32_t n = 0, cursor = 0;
    uint8_t mode = 0, lit = 0;
    bool parsed = cr.U16(version) && cr.I32(vertsX) && cr.F32(extentX) && cr.F32(extentZ) && cr.U32(n);
    for (uint32_t i = 0; parsed && i < n; ++i) {
        uint32_t id = 0;
        parsed = cr.Ref(id);
    }
    parsed = parsed && cr.U32(cursor) && cr.U8(mode) && cr.U8(lit);
    if (!parsed) {
        ++k.mismatches;
        k.first.Add("record not parsed (%zu bytes)", rec.size());
    } else {
        k.Int("vertsX", 0, 0, vertsX, st.verts_x);
        k.Float("extentX", 0, 0, extentX, st.extent_x);
        k.Float("extentZ", 0, 0, extentZ, st.extent_z);
        k.Int("night", 0, 0, n, st.night_len);
        k.Int("cursor", 0, 0, cursor, st.night_cursor);
        k.Int("mode", 0, 0, mode, st.night_mode);
        k.Int("lit", 0, 0, lit != 0, st.lit_flag != 0);
    }
    k.Int("s46", 0, 0, orig->GetForceEmptyLotBaseTextures(), fc4_lotdev_get_force_empty(core));

    // 2. The sample of lots (one reference each, from the whole city's list).
    uint32_t lots = 0, sampled = 0;
    cISC4LotManager* lmRaw = r.city->GetLotManager();
    if (lmRaw != nullptr && TractNodeAllocator::Available()) {
        LotList list;
        if (list.valid()) {
            AsGame(lmRaw)->GetLotsInCellRect(reinterpret_cast<GameList<cISC4Lot*>&>(list), 0, 0,
                                             static_cast<int32_t>(r.city->CellCountX()) - 1,
                                             static_cast<int32_t>(r.city->CellCountZ()) - 1, false);
            std::vector<cISC4Lot*> all;
            for (cISC4Lot*& lot : list) {
                if (lot == nullptr) continue;
                FC4_ACQUIRED(lot, "GetLotsInCellRect list entry");
                all.push_back(lot);
                lot = nullptr;
            }
            lots = static_cast<uint32_t>(all.size());
            const size_t stride = all.size() > kSampleLots ? all.size() / kSampleLots : 1;
            for (size_t i = 0; i < all.size() && sampled < kSampleLots; i += stride, ++sampled) {
                CompareLot(k, all[i]);
                LotDevReleaseHeld(&ctx);
            }
            for (cISC4Lot* lot : all) FC4_RELEASE(lot);
        }
    }

    AppendF(body, "record=%zu bytes roundtrip=%s night=%u cursor=%u mode=%u lit=%u lots=%u sampled=%u compared=%u "
                  "mismatches=%u rawAL=%u cw=0x%04X first=%s",
            rec.size(), same ? "same" : "DIFFERENT", n, cursor, static_cast<unsigned>(mode),
            static_cast<unsigned>(lit), lots, sampled, k.compared, k.mismatches, k.rawTruth, X87ControlWord(), k.first.Str());
    Line(r, body);

    fc4_lotdev_destroy(core);
    LotDevReleaseHeld(&ctx);
    LotDevReleaseServices(&ctx);
    r.result = k.mismatches;
}

intptr_t RunOnce(cISC4City* city, int32_t julian, uint32_t event, const fc4_lotdev_host* test) {
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
    if (!SehGuard("lotdev shadow", &RunStep, &r)) {
        Log("lotdev shadow %s: WARNING structured exception; comparison abandoned", r.date);
        return -3;
    }
    return r.result;
}

}  // namespace

void LotDevShadowStartup(bool lotdevReplaced) {
    g_enabled = SettingInt("lotdev_shadow", "FC4_LOTDEV_SHADOW", 0) != 0;
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("lotdev_shadow", "FC4_LOTDEV_SHADOW", &src);
    if (lotdevReplaced) {
        Log("lotdev shadow: requested (lotdev_shadow from %s) but INACTIVE: it compares the ORIGINAL lot developer, "
            "and `enable = lotdev` replaces it",
            src);
        g_enabled = false;
        return;
    }
    Log("lotdev shadow: ON (lotdev_shadow from %s): at PostCityInit and each SimNewMonth, FreeCity4's core reads the "
        "original lot developer's own record (its Write into memory), runs Init over a read-only table (it stops at "
        "step 8: no simulator) and must write the same bytes back with the same night state; then, on up to %u lots, "
        "the original's slots 15-18, 22, 25-32, 39 (first %u objects), 40, 46, 48, 51, 56 and 57 against the core's, "
        "bit for bit",
        src, static_cast<unsigned>(kSampleLots), static_cast<unsigned>(kSampleObjects));
    Log("lotdev shadow: NOT compared: the daily pass, construction, demolition and explosion, the night lights (they "
        "change the city and draw from the generators); levelling (slots 19-21) and the foundation add / remove "
        "slots (52-55, 58-61) (they write the terrain or the city); slots 42 / 43 (the boundary map is filled at Init "
        "step 11, past the read-only Init); the section 5 tests and the tuning (no getter); the night deque's "
        "buildings (references: compared as object numbers only)");
}

void LotDevShadowCityLoaded(cISC4City* city) {
    if (!g_enabled || city == nullptr) return;
    cFC4ISC4Simulator* sim = AsGame(city->GetSimulator());
    const int32_t julian = sim != nullptr ? sim->GetSimDateNumber() : 0;
    RunOnce(city, julian, 0, nullptr);
}

void LotDevShadowMonth(cISC4City* city, int32_t julian) {
    if (!g_enabled || city == nullptr) return;
    RunOnce(city, julian, 1, nullptr);
}

void LotDevShadowCityShutdown() {}

intptr_t LotDevShadowTestRun(cISC4City* city, int32_t julian, uint32_t event, const fc4_lotdev_host* testHost) {
    return RunOnce(city, julian, event, testHost);
}

}  // namespace fc4
