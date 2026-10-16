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

#include "FC4ZoneDevShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4WriteCapture.h"
#include "cFC4ZoneDeveloper.h"

#include "SC4Rect.h"
#include "cISC4City.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4Occupant.h"
#include "cISC4ZoneDeveloper.h"

#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

// The interface §10.2's reader asks each cell's lot reference for.
constexpr uint32_t kIID_cISC4Lot = 0xE818FCED;
// The core refuses longer sides; a larger city is not walked.
constexpr int32_t kMaxSide = 4096;
// Differences listed per line.
constexpr unsigned kFirst = 6;
// Samples per run.
constexpr uint32_t kListRects = 8;
constexpr uint32_t kOccupantCells = 48;
constexpr uint32_t kMaxOccupants = 96;
constexpr uint32_t kCanCreateCells = 12;
// A lot list longer than this is not trusted (a walk over a broken list).
constexpr uint32_t kMaxListLots = 1u << 16;

bool g_enabled = false;
bool g_loggedOurs = false;
bool g_loggedMissing = false;

// The original's record (main thread only; reused).
WriteCapture g_capture;

// ---- the captured record as an fc4_istream ------------------------------------------------------

struct RecordIn {
    const WriteCapture* cap = nullptr;
    size_t pos = 0;
    bool bad = false;
    std::vector<uint64_t> lots;  // per reference id: the lot identity (0 = not asked yet / none)
    std::vector<uint8_t> asked;
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

bool InStr(void* u, fc4_bytes_sink sink, void* ctx) {
    uint32_t n = 0;
    if (!Take(u, &n, 4)) return false;
    std::vector<uint8_t> s(n);
    if (!Take(u, n != 0 ? s.data() : nullptr, n)) return false;
    if (sink != nullptr) sink(ctx, s.data(), s.size());
    return true;
}

// A lot reference: object number (0 = null) then class ID, resolved to the
// object's cISC4Lot identity (borrowed: the original keeps its lots alive
// across the comparison; the QueryInterface reference is dropped at once).
// Any other interface reads as null.
bool InRef(void* u, uint32_t iid, fc4_ref* out) {
    RecordIn* r = static_cast<RecordIn*>(u);
    *out = fc4_ref{};
    uint32_t id = 0;
    if (!Take(u, &id, 4)) return false;
    if (id == 0) return true;
    uint32_t cls = 0;
    if (!Take(u, &cls, 4)) return false;
    out->class_id = cls;
    if (iid != kIID_cISC4Lot) return true;
    if (id >= r->lots.size()) {
        r->lots.resize(id + 1, 0);
        r->asked.resize(id + 1, 0);
    }
    if (r->asked[id] == 0) {
        r->asked[id] = 1;
        auto* ser = static_cast<cIGZUnknown*>(const_cast<void*>(r->cap->Object(id)));
        void* q = nullptr;
        if (ser != nullptr && ser->QueryInterface(iid, &q) && q != nullptr) {
            cIGZUnknown* lot = static_cast<cIGZUnknown*>(q);
            FC4_ACQUIRED(lot, "QueryInterface");
            FC4_RELEASE(lot);
            r->lots[id] = TokenOf(lot);
        }
    }
    out->token = r->lots[id];
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
    const fc4_zonedev_host* test = nullptr;
    char date[16] = "-";
    intptr_t result = 0;
};

void Line(const Run& r, const char* what, const std::string& body) {
    std::string line;
    AppendF(line, "zonedev shadow (%s) %s %s: ", what, r.date, r.event == 0 ? "load" : "month");
    line += body;
    LogShadowLine("zonedev shadow (cont.)", reinterpret_cast<const uint8_t*>(line.data()), line.size());
}

uint32_t NextRandom(uint32_t& s) {
    s ^= s << 13;
    s ^= s >> 17;
    s ^= s << 5;
    return s;
}

void* P(uint64_t t) { return static_cast<void*>(FromToken(t)); }

// The caller's list of slots 7-10: B nodes (or the test seam's allocator),
// the layout fc4::GameList<cISC4Lot*> has.
using ShadowLotList = fc4::GameList<cISC4Lot*, ZoneDevAllocators::NodePolicy>;
static_assert(sizeof(ShadowLotList) == sizeof(fc4::GameList<cISC4Lot*>), "one head pointer either way");

fc4::GameList<cISC4Lot*>& AsCallerList(ShadowLotList& l) { return reinterpret_cast<fc4::GameList<cISC4Lot*>&>(l); }

// The original's lot list (B nodes, ONE reference per lot): its lots in
// order; each reference is noted and given back (the manager keeps its own).
void TakeList(ShadowLotList& list, std::vector<uint64_t>& out) {
    out.clear();
    if (!list.valid()) return;
    uint32_t n = 0;
    for (cISC4Lot*& lot : list) {
        if (++n > kMaxListLots) break;
        out.push_back(TokenOf(lot));
        if (lot != nullptr) {
            FC4_ACQUIRED(lot, "lot list entry");
            FC4_RELEASE(lot);
        }
        lot = nullptr;  // the list's destructor then only frees its nodes
    }
}

void Collect(void* ctx, uint64_t token) { static_cast<std::vector<uint64_t>*>(ctx)->push_back(token); }

void RunStep(void* p) {
    Run& r = *static_cast<Run*>(p);
    cISC4ZoneDeveloper* raw = r.city->GetZoneDeveloper();
    cISC4LotManager* rawLm = r.city->GetLotManager();
    if (raw == nullptr || rawLm == nullptr) {
        if (!g_loggedMissing) {
            g_loggedMissing = true;
            Log("zonedev shadow %s: the city has no zone developer or lot manager; nothing compared", r.date);
        }
        r.result = -1;
        return;
    }
    if (cFC4ZoneDeveloper::FromAny(raw) != nullptr || cFC4ZoneDeveloper::FromAny(rawLm) != nullptr) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("zonedev shadow %s: the city's zone developer is FreeCity4's (enable = zonedev); the shadow compares "
                "the ORIGINAL: nothing compared",
                r.date);
        }
        r.result = -2;
        return;
    }
    cFC4ISC4LotManager* lm = AsGame(rawLm);
    const int32_t cx = static_cast<int32_t>(r.city->CellCountX());
    const int32_t cz = static_cast<int32_t>(r.city->CellCountZ());

    // The core's host: the city's services, borrowed (the test seam: the
    // test table instead).
    ZoneDevContext ctx;
    ctx.grid = HostGridContext{"zonedev shadow", Sw::ZoneDevGridWrites};
    if (r.test != nullptr) {
        ctx.test = r.test;
    } else {
        ctx.city = r.city;
        ctx.occupants = r.city->GetOccupantManager();
        ctx.terrain = r.city->GetTerrain();
        ctx.traffic = r.city->GetTrafficSimulator();
        ctx.zones = r.city->GetZoneManager();
        ctx.bds = r.city->GetBuildingDevelopmentSimulator();
    }
    const fc4_zonedev_host host = MakeZoneDevReadOnlyHost(&ctx);

    // The core on the original's own record (§10.2), then Init (§1.2).
    bool answered = false;
    const bool wrote = g_capture.Capture(reinterpret_cast<cIGZUnknown*>(raw), &answered);
    fc4_zonedev* s = nullptr;
    const char* why = nullptr;
    size_t unread = 0;
    bool inited = false;
    if (!wrote) {
        why = answered ? "the original's Write failed" : "the original does not answer cIGZSerializable";
    } else {
        s = fc4_zonedev_create_shadow(&host, 0);
        if (s == nullptr) {
            why = "the shadow core was not created";
        } else {
            RecordIn in;
            in.cap = &g_capture;
            const fc4_istream is = MakeIn(&in);
            const bool read = fc4_zonedev_read(s, &is);
            ZoneDevReleaseHeld(&ctx);
            unread = g_capture.Bytes().size() - in.pos;
            if (!read) {
                why = "the core refused the original's record";
                fc4_zonedev_destroy(s);
                s = nullptr;
            } else {
                inited = fc4_zonedev_init(s);
                ZoneDevReleaseHeld(&ctx);
            }
        }
    }
    if (s == nullptr) {
        std::string body;
        AppendF(body, "INCOMPLETE (%s); not compared", why != nullptr ? why : "no core");
        for (const char* w : {"lots", "cells", "lists", "occupants", "CanCreateLot", "CanCreatePloppedLot"}) {
            Line(r, w, body);
        }
        r.result = -3;
        return;
    }
    const bool walkable = cx > 0 && cz > 0 && cx <= kMaxSide && cz <= kMaxSide;
    intptr_t total = 0;
    bool incomplete = false;

    // 1. Lots: slot 16 and the record.
    {
        const int32_t orig = lm->GetLotCount();
        const uint32_t ours = fc4_zonedev_lot_count(s);
        fc4_zonedev_state st{};
        st.struct_size = sizeof(st);
        fc4_zonedev_get_state(s, &st);
        uint32_t mismatches = 0;
        if (static_cast<uint32_t>(orig) != ours) ++mismatches;
        if (unread != 0) ++mismatches;
        std::string body;
        AppendF(body, "mismatches=%u count=%d/%u (orig/ours) map=%dx%d city=%dx%d init=%u record=%u bytes unread=%u",
                mismatches, orig, ours, st.map_x, st.map_z, cx, cz, inited ? 1u : 0u,
                static_cast<unsigned>(g_capture.Bytes().size()), static_cast<unsigned>(unread));
        Line(r, "lots", body);
        total += mismatches;
    }

    // 2. Cells: slot 6 on every cell, originOnly false and true.
    std::vector<uint64_t> origins;  // a few lots' origin cells, for the occupant samples
    if (!walkable) {
        std::string body;
        AppendF(body, "INCOMPLETE (city %d x %d); not compared", cx, cz);
        Line(r, "cells", body);
        incomplete = true;
    } else {
        uint32_t mismatches = 0, covered = 0, originCells = 0;
        FirstList first;
        for (int32_t x = 0; x < cx; ++x) {
            for (int32_t z = 0; z < cz; ++z) {
                const uint64_t o = TokenOf(lm->GetLot(x, z, false));
                const uint64_t u = fc4_zonedev_get_lot(s, x, z, false);
                const uint64_t oo = TokenOf(lm->GetLot(x, z, true));
                const uint64_t uo = fc4_zonedev_get_lot(s, x, z, true);
                ZoneDevReleaseHeld(&ctx);
                if (o != 0) ++covered;
                if (oo != 0) {
                    ++originCells;
                    if (origins.size() < 16) origins.push_back((static_cast<uint64_t>(x) << 32) | static_cast<uint32_t>(z));
                }
                if (o != u || oo != uo) {
                    ++mismatches;
                    first.Add("%d,%d:%p/%p,%p/%p", x, z, P(o), P(u), P(oo), P(uo));
                }
            }
        }
        std::string body;
        AppendF(body, "cells=%u covered=%u origins=%u mismatches=%u; first=%s",
                static_cast<unsigned>(cx * cz), covered, originCells, mismatches, first.Str());
        Line(r, "cells", body);
        total += mismatches;
    }

    uint32_t seed = static_cast<uint32_t>(r.julian) * 2654435761u ^ (r.event == 0 ? 0x9E3779B9u : 0x7F4A7C15u);
    if (seed == 0) seed = 1;

    // 3. Lists: slots 7 and 10 on sample rects.
    if (walkable) {
        uint32_t calls = 0, mismatches = 0, origLots = 0, oursLots = 0;
        FirstList first;
        std::string rects;
        std::vector<uint64_t> o, u;
        for (uint32_t i = 0; i < kListRects; ++i) {
            const int32_t w = 1 + static_cast<int32_t>(NextRandom(seed) % 12u);
            const int32_t h = 1 + static_cast<int32_t>(NextRandom(seed) % 12u);
            const int32_t x1 = static_cast<int32_t>(NextRandom(seed) % static_cast<uint32_t>(cx));
            const int32_t z1 = static_cast<int32_t>(NextRandom(seed) % static_cast<uint32_t>(cz));
            const int32_t x2 = x1 + w - 1, z2 = z1 + h - 1;  // may run past the city: both clamp (§8.3)
            const uint32_t sides = NextRandom(seed) % 16u;
            AppendF(rects, "%s%d,%d..%d,%d/%X", i == 0 ? "" : " ", x1, z1, x2, z2, sides);
            for (int k = 0; k < 4; ++k) {
                ShadowLotList list;
                if (!list.valid()) {
                    incomplete = true;
                    break;
                }
                uint32_t oc = 0, uc = 0;
                u.clear();
                const fc4_cell_rect rect{x1, z1, x2, z2};
                if (k < 2) {
                    const bool originOnly = k == 1;
                    oc = lm->GetLotsInCellRect(AsCallerList(list), x1, z1, x2, z2, originOnly);  // slot 7
                    uc = fc4_zonedev_lots_in_cell_rect(s, &rect, originOnly, &Collect, &u);
                } else {
                    const int32_t flags = k == 2 ? 0xF : static_cast<int32_t>(sides);
                    const SC4Rect<long> sr(x1, z1, x2, z2);
                    oc = lm->GetLotsSurroundingRect(sr, AsCallerList(list), flags);  // slot 10
                    uc = fc4_zonedev_lots_surrounding_rect(s, &rect, static_cast<uint32_t>(flags), nullptr, 0,
                                                           &Collect, &u);
                }
                ZoneDevReleaseHeld(&ctx);
                TakeList(list, o);
                ++calls;
                origLots += static_cast<uint32_t>(o.size());
                oursLots += static_cast<uint32_t>(u.size());
                if (oc != uc || o != u || oc != o.size()) {
                    ++mismatches;
                    first.Add("r%u/%d count %u/%u lots %u/%u", i, k, oc, uc, static_cast<unsigned>(o.size()),
                              static_cast<unsigned>(u.size()));
                }
            }
        }
        std::string body;
        AppendF(body, "calls=%u mismatches=%u lots=%u/%u (orig/ours) rects=%s; first=%s", calls, mismatches, origLots,
                oursLots, rects.c_str(), first.Str());
        Line(r, "lists", body);
        total += mismatches;
    } else {
        Line(r, "lists", "INCOMPLETE (no city size); not compared");
    }

    // 4. Occupants: slot 14 on the occupants of the origin cells found and of
    //    drawn cells (the read-only host's occupant lists, borrowed).
    if (walkable) {
        std::vector<uint64_t> cells(origins);
        for (uint32_t i = 0; i < kOccupantCells; ++i) {
            const uint32_t x = NextRandom(seed) % static_cast<uint32_t>(cx);
            const uint32_t z = NextRandom(seed) % static_cast<uint32_t>(cz);
            cells.push_back((static_cast<uint64_t>(x) << 32) | z);
        }
        std::vector<uint64_t> occupants;
        for (uint64_t c : cells) {
            if (occupants.size() >= kMaxOccupants || host.cell_occupants == nullptr) break;
            host.cell_occupants(host.user, static_cast<int32_t>(c >> 32), static_cast<int32_t>(c & 0xFFFFFFFFu), false,
                                0, &Collect, &occupants);
        }
        if (occupants.size() > kMaxOccupants) occupants.resize(kMaxOccupants);
        uint32_t mismatches = 0, onLots = 0;
        FirstList first;
        for (uint64_t occ : occupants) {
            const uint64_t o = TokenOf(lm->GetOccupantLot(reinterpret_cast<cISC4Occupant*>(FromToken(occ))));
            const uint64_t u = fc4_zonedev_occupant_lot(s, occ);
            if (o != 0) ++onLots;
            if (o != u) {
                ++mismatches;
                first.Add("%p:%p/%p", P(occ), P(o), P(u));
            }
        }
        ZoneDevReleaseHeld(&ctx);  // the occupant lists
        std::string body;
        AppendF(body, "occupants=%u onLots=%u mismatches=%u; first=%s", static_cast<unsigned>(occupants.size()),
                onLots, mismatches, first.Str());
        Line(r, "occupants", body);
        total += mismatches;
    } else {
        Line(r, "occupants", "INCOMPLETE (no city size); not compared");
    }

    // 5. CanCreateLot (slot 20) on sample cells and sizes, both flags, no
    //    configuration.
    if (walkable) {
        static const int32_t kSizes[4][2] = {{1, 1}, {2, 3}, {3, 3}, {4, 4}};
        uint32_t calls = 0, mismatches = 0;
        uint32_t histOrig[9] = {}, histOurs[9] = {};
        FirstList first;
        for (uint32_t i = 0; i < kCanCreateCells; ++i) {
            const int32_t x = static_cast<int32_t>(NextRandom(seed) % static_cast<uint32_t>(cx));
            const int32_t z = static_cast<int32_t>(NextRandom(seed) % static_cast<uint32_t>(cz));
            for (const auto& sz : kSizes) {
                for (int f = 0; f < 4; ++f) {
                    const bool zoneNotNeeded = (f & 1) != 0;
                    const bool waterAllowed = (f & 2) != 0;
                    const int32_t o = lm->CanCreateLot(x, z, sz[0], sz[1], zoneNotNeeded, waterAllowed, nullptr);
                    const int32_t u = fc4_zonedev_can_create_lot(s, x, z, sz[0], sz[1], zoneNotNeeded, waterAllowed, 0);
                    ZoneDevReleaseHeld(&ctx);
                    ++calls;
                    if (o >= 0 && o <= 8) ++histOrig[o];
                    if (u >= 0 && u <= 8) ++histOurs[u];
                    if (o != u) {
                        ++mismatches;
                        first.Add("%d,%d %dx%d f%d:%d/%d", x, z, sz[0], sz[1], f, o, u);
                    }
                }
            }
        }
        std::string body;
        AppendF(body, "calls=%u mismatches=%u codes 0/1/2/3/7/8 orig=%u/%u/%u/%u/%u/%u ours=%u/%u/%u/%u/%u/%u; first=%s",
                calls, mismatches, histOrig[0], histOrig[1], histOrig[2], histOrig[3], histOrig[7], histOrig[8],
                histOurs[0], histOurs[1], histOurs[2], histOurs[3], histOurs[7], histOurs[8], first.Str());
        Line(r, "CanCreateLot", body);
        total += mismatches;
    } else {
        Line(r, "CanCreateLot", "INCOMPLETE (no city size); not compared");
    }

    // 6. CanCreatePloppedLot (slot 21, §8.4.1) on sample cells, sizes and
    //    facings. The original reads the configuration without a null test,
    //    so both sides get a real one: the configuration of the first lot
    //    that has one (borrowed, never released). A city without any lot
    //    with a configuration cannot reach the code and is left alone.
    if (walkable) {
        cISC4LotConfiguration* cfg = nullptr;
        {
            ShadowLotList list{};
            lm->GetLotsInCellRect(AsCallerList(list), 0, 0, cx - 1, cz - 1, false);  // slot 7
            std::vector<uint64_t> lots;
            TakeList(list, lots);  // one reference each, given back as it goes
            for (uint64_t t : lots) {
                cISC4Lot* lot = static_cast<cISC4Lot*>(FromToken(t));
                if (lot == nullptr) continue;
                cfg = AsGame(lot)->GetLotConfiguration();  // slot 9, borrowed
                if (cfg != nullptr) break;
            }
        }
        if (cfg == nullptr) {
            Line(r, "CanCreatePloppedLot", "INCOMPLETE (no lot with a configuration); not compared");
        } else {
            static const int32_t kSizes[3][2] = {{1, 1}, {2, 3}, {3, 3}};
            uint32_t calls = 0, mismatches = 0;
            uint32_t histOrig[9] = {}, histOurs[9] = {};
            FirstList first;
            const uint64_t cfgToken = TokenOf(cfg);
            for (uint32_t i = 0; i < kCanCreateCells; ++i) {
                const int32_t x = static_cast<int32_t>(NextRandom(seed) % static_cast<uint32_t>(cx));
                const int32_t z = static_cast<int32_t>(NextRandom(seed) % static_cast<uint32_t>(cz));
                for (const auto& sz : kSizes) {
                    for (int32_t facing = 0; facing < 4; ++facing) {
                        SC4Rect<long> bounds(static_cast<long>(x), static_cast<long>(z),
                                             static_cast<long>(x + sz[0] - 1), static_cast<long>(z + sz[1] - 1));
                        fc4_cell_rect rct{x, z, x + sz[0] - 1, z + sz[1] - 1};
                        const int32_t o = lm->CanCreatePloppedLot(bounds, facing, cfg);
                        const int32_t u = fc4_zonedev_can_create_plopped_lot(s, &rct, facing, cfgToken);
                        ZoneDevReleaseHeld(&ctx);
                        ++calls;
                        if (o >= 0 && o <= 8) ++histOrig[o];
                        if (u >= 0 && u <= 8) ++histOurs[u];
                        if (o != u) {
                            ++mismatches;
                            first.Add("%d,%d %dx%d f%d:%d/%d", x, z, sz[0], sz[1], facing, o, u);
                        }
                    }
                }
            }
            std::string body;
            AppendF(body,
                    "calls=%u mismatches=%u codes 0/1/3/4/7/8 orig=%u/%u/%u/%u/%u/%u ours=%u/%u/%u/%u/%u/%u; first=%s",
                    calls, mismatches, histOrig[0], histOrig[1], histOrig[3], histOrig[4], histOrig[7], histOrig[8],
                    histOurs[0], histOurs[1], histOurs[3], histOurs[4], histOurs[7], histOurs[8], first.Str());
            Line(r, "CanCreatePloppedLot", body);
            total += mismatches;
        }
    } else {
        Line(r, "CanCreatePloppedLot", "INCOMPLETE (no city size); not compared");
    }

    fc4_zonedev_shutdown(s);
    fc4_zonedev_destroy(s);
    ZoneDevReleaseHeld(&ctx);
    r.result = incomplete ? -3 : total;
}

intptr_t RunOnce(cISC4City* city, int32_t julian, uint32_t event, const fc4_zonedev_host* test) {
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
    if (!SehGuard("zonedev shadow", &RunStep, &r)) {
        Log("zonedev shadow %s: WARNING structured exception; comparison abandoned", r.date);
        return -3;
    }
    return r.result;
}

}  // namespace

void ZoneDevShadowStartup(bool zonedevReplaced) {
    g_enabled = SettingInt("zonedev_shadow", "FC4_ZONEDEV_SHADOW", 0) != 0;
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("zonedev_shadow", "FC4_ZONEDEV_SHADOW", &src);
    if (zonedevReplaced) {
        Log("zonedev shadow: requested (zonedev_shadow from %s) but INACTIVE: it compares the ORIGINAL zone "
            "developer, and `enable = zonedev` replaces it",
            src);
        g_enabled = false;
        return;
    }
    Log("zonedev shadow: ON (zonedev_shadow from %s): at PostCityInit and each SimNewMonth, FreeCity4's core loaded "
        "from the original zone developer's own record (its Write into memory, each cell's lot by reference) against "
        "the original lot manager: the lot count (slot 16), every cell (slot 6, originOnly 0 and 1), %u sample rects "
        "of GetLotsInCellRect (slot 7) and GetLotsSurroundingRect (slot 10) in order, GetOccupantLot (slot 14) on "
        "sample occupants, CanCreateLot (slot 20) on %u sample cells x 4 sizes x 4 flag pairs and CanCreatePloppedLot "
        "(slot 21) on %u sample cells x 3 sizes x 4 facings with the configuration of the first lot that has one",
        src, static_cast<unsigned>(kListRects), static_cast<unsigned>(kCanCreateCells),
        static_cast<unsigned>(kCanCreateCells));
    Log("zonedev shadow: NOT compared: parcellization (it changes the city and draws random numbers, zones.md §7, §9; "
        "no dry run exists: HighlightParcels draws, posts and places street drafts, §7.1), lot creation and "
        "deletion, the safe lot iterator, the master list order (a Read rebuilds it in first-cell order, §10.2)");
}

void ZoneDevShadowCityLoaded(cISC4City* city) {
    if (!g_enabled || city == nullptr) return;
    cFC4ISC4Simulator* sim = AsGame(city->GetSimulator());
    const int32_t julian = sim != nullptr ? sim->GetSimDateNumber() : 0;
    RunOnce(city, julian, 0, nullptr);
}

void ZoneDevShadowMonth(cISC4City* city, int32_t julian) {
    if (!g_enabled || city == nullptr) return;
    RunOnce(city, julian, 1, nullptr);
}

intptr_t ZoneDevShadowTestRun(cISC4City* city, int32_t julian, uint32_t event, const fc4_zonedev_host* testHost) {
    return RunOnce(city, julian, event, testHost);
}

}  // namespace fc4
