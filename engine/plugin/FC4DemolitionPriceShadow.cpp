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

// The demolition pricing differential: see FC4DemolitionPriceShadow.h.

#include "FC4DemolitionPriceShadow.h"

#include "FC4Config.h"
#include "FC4DemolitionHost.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Streams.h"

#include "SC4List.h"
#include "cISC4City.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"

#include <cinttypes>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

struct PriceShadow {
    bool enabled = false;
    bool active = false;
    bool warned = false;
    cISC4City* city = nullptr;
    DemolitionHostContext ctx{};
    fc4_demolition_host host{};
    fc4_demolition* core = nullptr;
    uint32_t lines = 0, clean = 0, compared = 0, mismatched = 0;
    std::string first;
};

PriceShadow& S() {
    static PriceShadow s;
    return s;
}

// The original's utility (city slot 31), borrowed.
cFC4ISC4Demolition* Original(cISC4City* city) {
    cISC4Demolition* d = city ? city->GetDemolitionUtility() : nullptr;
    return d ? AsGame(d) : nullptr;
}

void Note(uint32_t julian, const char* event, uint32_t priced, uint32_t mismatches, const std::string& first) {
    PriceShadow& s = S();
    ++s.lines;
    if (mismatches == 0) ++s.clean;
    s.compared += priced;
    s.mismatched += mismatches;
    if (mismatches != 0 && s.first.empty()) s.first = first;
    Log("demolition price shadow %d load|%s: priced=%u mismatches=%u first=%s", julian, event, priced,
        mismatches, first.empty() ? "-" : first.c_str());
}

}  // namespace

bool DemolitionPriceShadowEnabled() { return S().enabled && S().active; }

void DemolitionPriceShadowStartup(bool demolitionReplaced) {
    PriceShadow& s = S();
    s.enabled = SettingInt("demolition_price_shadow", "FC4_DEMOLITION_PRICE_SHADOW", 0) != 0;
    if (!s.enabled) {
        Log("demolition price shadow: off (demolition_price_shadow = 0)");
        return;
    }
    const char* srcName = "ini";
    Setting("demolition_price_shadow", "FC4_DEMOLITION_PRICE_SHADOW", &srcName);
    const std::string src = srcName != nullptr ? srcName : "ini";
    if (demolitionReplaced) {
        s.active = false;
        Log("demolition price shadow: requested (demolition_price_shadow = 1) but INACTIVE: "
            "enable = demolition replaced the original, so there is nothing to price against");
        return;
    }
    s.active = true;
    Log("demolition price shadow: ON (demolition_price_shadow from %s): every DemolishOccupant price and one "
        "DemolishRegion dry run, ours against the original's, on the live city at PostCityInit and every "
        "SimNewMonth. Not priced here: ModifyTerrainHeight's vertex counts (a dry run still posts the terrain "
        "message, §4.7 [quirk]) and the effects (§4.8, presentation only).");
}

void DemolitionPriceShadowCityLoaded(cISC4City* city) {
    PriceShadow& s = S();
    if (!s.enabled) return;
    if (!s.active) {
        if (!s.warned) {
            Log("demolition price shadow: INACTIVE (the original's utility is not the one running)");
            s.warned = true;
        }
        return;
    }
    s.city = city;
    s.ctx = DemolitionHostContext{};
    s.ctx.city = city;
    s.host = MakeDemolitionHost(&s.ctx);
    if (s.core != nullptr) fc4_demolition_destroy(s.core);
    s.core = fc4_demolition_create(&s.host, 0);
    if (s.core == nullptr || !fc4_demolition_init(s.core)) {
        Log("demolition price shadow: WARNING our core did not initialise (load not compared)");
        return;
    }
    const uint32_t cellsX = city ? city->CellCountX() : 0;
    const uint32_t cellsZ = city ? city->CellCountZ() : 0;
    std::string first;
    uint32_t priced = 0, mism = 0;
    cFC4ISC4Demolition* orig = Original(city);
    if (orig == nullptr) {
        first = "no original utility";
        mism = 1;
    } else {
        fc4_cell_rect r{0, 0, static_cast<int32_t>(cellsX), static_cast<int32_t>(cellsZ)};
        auto list = DemolitionOccupantsInRect(city, &r, &s.ctx);
        for (cISC4Occupant* o : *static_cast<SC4List<cISC4Occupant*>*>(list.get())) {
            if (o == nullptr) continue;
            ++priced;
            int64_t a = 0, b = 0;
            const bool ra = orig->DemolishOccupant(false, o, 0, 0, &a, true, nullptr);  // slot 8
            const bool rb = fc4_demolition_demolish_occupant(s.core, false, TokenOf(o), 0, 0, true, 0, &b);
            if (ra != rb || a != b) {
                ++mism;
                if (first.empty()) {
                    char buf[128];
                    std::snprintf(buf, sizeof(buf), "occupant %p type 0x%08X: original ok=%d cost=%" PRId64
                                                    ", ours ok=%d cost=%" PRId64,
                                  static_cast<void*>(o), o->GetType(), ra ? 1 : 0, a, rb ? 1 : 0, b);
                    first = buf;
                }
            }
        }
    }
    DemolitionHostReleaseHeld(&s.ctx);
    Note(0, "load", priced, mism, first);
    Log("demolition price shadow: load city %u x %u cells, our core %p over the %d-callback host", cellsX, cellsZ,
        static_cast<void*>(s.core), 86);
}

void DemolitionPriceShadowMonth(cISC4City* city, int32_t julian) {
    PriceShadow& s = S();
    if (!s.enabled || !s.active || s.core == nullptr || city == nullptr) return;
    cFC4ISC4Demolition* orig = Original(city);
    std::string first;
    uint32_t priced = 0, mism = 0;
    if (orig == nullptr) {
        first = "no original utility";
        mism = 1;
    } else {
        // Every occupant price (§4.3), then one whole-city DemolishRegion
        // dry run (§4.4).
        const uint32_t cellsX = city->CellCountX(), cellsZ = city->CellCountZ();
        fc4_cell_rect r{0, 0, static_cast<int32_t>(cellsX), static_cast<int32_t>(cellsZ)};
        auto list = DemolitionOccupantsInRect(city, &r, &s.ctx);
        for (cISC4Occupant* o : *static_cast<SC4List<cISC4Occupant*>*>(list.get())) {
            if (o == nullptr) continue;
            ++priced;
            int64_t a = 0, b = 0;
            const bool ra = orig->DemolishOccupant(false, o, 0, 0, &a, true, nullptr);  // slot 8
            const bool rb = fc4_demolition_demolish_occupant(s.core, false, TokenOf(o), 0, 0, true, 0, &b);
            if (ra != rb || a != b) {
                ++mism;
                if (first.empty()) {
                    char buf[128];
                    std::snprintf(buf, sizeof(buf), "occupant %p type 0x%08X: original ok=%d cost=%" PRId64
                                                    ", ours ok=%d cost=%" PRId64,
                                  static_cast<void*>(o), o->GetType(), ra ? 1 : 0, a, rb ? 1 : 0, b);
                    first = buf;
                }
            }
        }
        DemolitionHostReleaseHeld(&s.ctx);

        // The region dry run: the same cell rectangle on both sides.
        fc4_cell_rect whole{0, 0, static_cast<int32_t>(cellsX) - 1, static_cast<int32_t>(cellsZ) - 1};
        std::vector<uint8_t> mask(static_cast<size_t>(cellsX) * cellsZ, 0xFF);
        fc4_zone_region reg{};
        reg.x0 = 0;
        reg.z0 = 0;
        reg.x1 = static_cast<int32_t>(cellsX) - 1;
        reg.z1 = static_cast<int32_t>(cellsZ) - 1;
        reg.mask = mask.data();
        reg.len = mask.size();
        ++priced;
        int64_t a = 0, b = 0;
        // The original takes the game's SC4CellRegion; the whole-city rect is
        // the same one the host builds, so both sides see the same cells.
        const bool ra = orig->DemolishRegion(false, *reinterpret_cast<SC4CellRegion<int32_t> const*>(&whole), 0, 0,
                                             false, nullptr, &a, 0, nullptr, 0, 0);  // slot 6
        const bool rb = fc4_demolition_demolish_region(s.core, false, &reg, 0, 0, false, 0, 0, 0, 0, 0, &b);
        if (ra != rb || a != b) {
            ++mism;
            if (first.empty()) {
                char buf[128];
                std::snprintf(buf, sizeof(buf),
                              "DemolishRegion whole city: original ok=%d cost=%" PRId64
                              ", ours ok=%d cost=%" PRId64,
                              ra ? 1 : 0, a, rb ? 1 : 0, b);
                first = buf;
            }
        }
    }
    Note(julian, "month", priced, mism, first);
}

void DemolitionPriceShadowCityShutdown() {
    PriceShadow& s = S();
    if (!s.enabled) return;
    if (s.core != nullptr) {
        fc4_demolition_destroy(s.core);
        s.core = nullptr;
    }
    DemolitionHostReleaseAll(&s.ctx);
    Log("demolition price shadow: city shutdown: %u lines, %u with mismatches=0; compared %u prices, %u mismatched; "
        "differences: %s",
        s.lines, s.clean, s.compared, s.mismatched, s.first.empty() ? "none" : s.first.c_str());
    s.city = nullptr;
    s.lines = s.clean = s.compared = s.mismatched = 0;
    s.first.clear();
}

}  // namespace fc4
