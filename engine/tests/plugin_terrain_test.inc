// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

// The glue16 lane: the terrain shadow (plugin/FC4TerrainShadow.h, seam op 58)
// on a stand-in original made inside the DLL. Included by plugin_load_test.cpp.
#include "cFC4Terrain.h"

// A 16 x 16-cell terrain: a slope from 236 m (west) to 284 m (east) across a
// 250 m sea, with a ridge and a hollow, so the city has sea, shore and land.
std::vector<float> TerrainTestAltitudes() {
    std::vector<float> a;
    for (int z = 0; z <= 16; ++z) {
        for (int x = 0; x <= 16; ++x) {
            float h = 236.0f + 3.0f * static_cast<float>(x) + 0.25f * static_cast<float>(z % 3);
            if (x == 9) h += 6.5f;              // a ridge
            if (z == 12 && x > 3) h -= 4.75f;   // a hollow
            a.push_back(h);
        }
    }
    return a;
}

intptr_t TerrainShadowRun(FC4TestSeamFn seam, FC4TerrainShadowTestInput* in, uint32_t perturb, uint32_t noMessage,
                          const std::vector<float>& alts) {
    *in = FC4TerrainShadowTestInput{};
    in->cellsX = 16;
    in->cellsZ = 16;
    in->altitudes = alts.data();
    in->count = alts.size();
    in->seaLevel = 250.0f;
    in->perturb = perturb;
    in->withoutMessage = noMessage;
    return seam(kFC4SeamTerrainShadow, in, nullptr);
}

int TerrainShadowIniTest(const std::string& dll, const std::string& ini, const std::string& log) {
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    SetEnvironmentVariableA("FC4_TERRAIN_SHADOW", nullptr);
    WriteTextFile(ini, "[tools]\nterrain_shadow = 1\nterrain_shadow_points = 64\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (hd == nullptr) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    auto seam = reinterpret_cast<FC4TestSeamFn>(GetProcAddress(hd, "FC4TestSeam"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr && seam != nullptr);
    if (director != nullptr && seam != nullptr) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);  // game start: the configuration
        const std::vector<float> alts = TerrainTestAltitudes();
        CHECK(seam(kFC4SeamTerrainShadow, nullptr, nullptr) == -1);
        FC4TerrainShadowTestInput in{};
        // The original as the spec says (the same core): every comparison exact.
        CHECK(TerrainShadowRun(seam, &in, 0, 0, alts) == 0);
        CHECK(in.loadMismatches == 0 && in.changeMismatches == 0 && in.scriptMismatches == 0 &&
              in.monthMismatches == 0);
        CHECK(in.compared > 1000);
        CHECK(in.unfollowed == 0);
        CHECK(in.changesSeen == 5);   // the block (terrain, water), slot 50 (terrain), the script (terrain, water)
        CHECK(in.vertexChecks == 6);  // the scripted rectangle 0,0..2,1
        CHECK(in.lines == 10 && in.cleanLines == 10);  // load, 3 months, 5 changes, the script
        CHECK(in.late == 0);
        // Its messages lost: every change is found by a month's re-read, and
        // counted unfollowed at the next month.
        CHECK(TerrainShadowRun(seam, &in, 0, 1, alts) == 0);
        CHECK(in.changesSeen == 0 && in.unfollowed > 0 && in.late == 0);
        CHECK(in.vertexChecks == 6);
        // The block's messages held past the next month (the game's queued
        // posts under turbo): found ahead by the month, then followed late.
        CHECK(TerrainShadowRun(seam, &in, 0, 2, alts) == 0);
        CHECK(in.changesSeen == 5 && in.unfollowed == 0 && in.late == 6);
        CHECK(in.lines == 10 && in.cleanLines == 10);
        // An original that differs: found.
        CHECK(TerrainShadowRun(seam, &in, 1, 0, alts) > 0);
        CHECK(in.loadMismatches > 0 && in.monthMismatches > 0);
        CHECK(TerrainShadowRun(seam, &in, 2, 0, alts) > 0);
        CHECK(in.loadMismatches > 0);
        // A bad input.
        in = FC4TerrainShadowTestInput{};
        CHECK(seam(kFC4SeamTerrainShadow, &in, nullptr) == -1);
        FC4TerrainGlueTestInput glue{};
        glue.struct_size = sizeof(glue);
        CHECK(seam(kFC4SeamTerrainGlue, &glue, nullptr) == 0);
        CHECK(glue.mismatches == 0 && glue.slotsChecked >= 38);
    }
    FreeLibrary(hd);
    DeleteFileA(ini.c_str());
    const std::string text = ReadFile(log);
    const auto npos = std::string::npos;
    CHECK(text.find("terrain shadow: ON (terrain_shadow from FreeCity4.ini)") != npos);
    CHECK(text.find("sample compared (points 64;") != npos);
    CHECK(CountOf(text, "terrain shadow load 2026-01-27: cells=16x16 sea=250") == 5);
    CHECK(text.find("terrain shadow change 2026-02-06 terrain kind=4 rect=1,1..3,2: vertices=6 moved=6") != npos);
    CHECK(text.find("terrain shadow change 2026-02-07 terrain kind=4 rect=2,3..2,3: vertices=1 moved=1 wet=0 depth "
                    "rules slot50/raiselower/reset/none=1/0/0/0; message 1/1;") != npos);
    CHECK(text.find("terrain shadow script 2026-03-08 SetAltitude rect=0,0..2,1 h=262.25 kind=1: vertices 6/6;") !=
          npos);
    CHECK(text.find("terrain shadow month 2026-02-01: unfollowed=0; ahead=0;") != npos);
    CHECK(text.find("terrain shadow month 2026-03-04: unfollowed=0; ahead=6;") != npos);  // the held block
    CHECK(text.find("terrain shadow change 2026-03-05 terrain kind=4 rect=1,1..3,2: vertices=6 moved=6 wet=0 depth "
                    "rules slot50/raiselower/reset/none=6/0/0/0; message 1/1; late 6 (found ahead by a month "
                    "re-read);") != npos);
    CHECK(text.find("terrain shadow month 2026-03-04: unfollowed=7 (altitudes 7, depths only 0; rect=1,1..3,3;") ==
          npos);                                                                     // pending first: not yet
    CHECK(text.find("terrain shadow month 2026-04-01: unfollowed=7 (altitudes 7, depths only 0; rect=1,1..3,3;") !=
          npos);                                                                     // lost: the next month
    CHECK(text.find("refused=0; mismatches=0") != npos);
    CHECK(text.find("terrain shadow: city shutdown: 10 lines, 10 with mismatches=0") != npos);
    CHECK(text.find("first: altitude(") != npos);  // perturb 1 names its differences
    CHECK(text.find("first: depth(") != npos || text.find(" depth(") != npos);  // perturb 2
    if (g_failures == 0) std::printf("plugin terrain shadow test: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}
