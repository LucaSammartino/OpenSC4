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

// The crime grid shadow: a same-process differential of the crime grid
// (tooling; no spec behaviour). Opt-in, off by default:
//
//   crime_shadow = 1          (env FC4_CRIME_SHADOW)
//   crime_shadow_cells = N    differing cells listed per month
//                             (env FC4_CRIME_SHADOW_CELLS, default 8, 0..64)
//
// It needs FreeCity4's simulator and the ORIGINAL crime simulator (the
// default: `crime` is not enabled). Our simulator delivers the clock to the
// original crime simulator's agent (levels 4, 6, 10: crime.md §4.1 step 4),
// and the shadow mirrors its pipeline (§5) from those deliveries: a new
// simulator starts in state 6; SimNewMonth sets 6 -> 0; each SimNewDay or
// SimIdle runs one step and advances 0 -> 1 -> ... -> 6.
//
// The well-defined moment: the delivery that runs pipeline state 4 (§5: decay
// the crime grid, clear the incident list, commit this month's crimes,
// ProcessMonthlyCrimeStats, average crime). The shadow wraps it
// (cFC4Simulator::HostDeliver):
//   1. before: the original's crime grid (GetCrimeGrid, every tract through
//      GetTractValue) and hotspot (GetCrimeHotspot);
//   2. the original's DoMessage, unchanged;
//   3. after: the grid again, the hotspot, GetMonthlyCrimeCount,
//      GetAverageCrime, the incident list (GetCrimeIncidentArray into a vector
//      of ours), each incident's lot (lot manager GetLot at the incident's
//      cell) and its bounding rect, and the zone type at every tract corner;
//   4. FreeCity4's state-4 grid work (fc4_crime_shadow_month, the code our
//      crime simulator runs) replayed on the grid read in step 1 with the
//      original's incidents, compared cell by cell with the grid read in step 3,
//      and the hotspot, count and average next to the original's.
//
// What is compared: the deterministic part of state 4 - the decay (§6.5), the
// hotspot re-read and update, the severity stamps of the incidents that
// happened (§6.6 step 7, severities from FreeCity4's crime type list), the
// monthly count (incidents of a severity > 0) and the zoned average (§6.9).
// What is NOT compared, and why:
//   - the number of crimes to commit (§6.4): it needs ResetUnjailedCriminals,
//     which has side effects (it cannot be read without changing the police
//     simulator); the census and criminal base are not exposed;
//   - which attempts the police stopped, which type each crime got (§6.6,
//     §6.7): random draws on the original's clock-seeded generator;
//   - which lots were tried (§6.6 step 1): the lot manager's generator;
//   - the crime-effect grid (§2): no interface exposes it.
// The incidents' lot bits are checked against the type masks (§6.7) for
// information only (read after the step: an arson fire may already have
// changed the lot).
//
// Read-only: only getters are called on the original simulator, its grid, the
// lot manager, lots and the zone manager; the incident vector is ours. The
// tuning and the crime types come from the exemplars through the crime host's
// read-only table (fc4::CrimeReadOnlyHost).
//
// Log lines (always written, not the verbose channel):
//   crime shadow: ON / requested but INACTIVE ...                (game start)
//   crime shadow: tracking the original crime simulator ...      (each city)
//   crime shadow YYYY-MM-DD: state 4 compared on CxZ tracts (S=s): cells
//     mismatched=M max|orig-ours|=D [cells (x,z) orig/ours ...]; hotspot
//     orig/ours=...; count orig/ours=a/b; average orig/ours=a/b;
//     incidents=I (...); mismatches=N; months=K clean=C; not compared: ...
//   crime shadow YYYY-MM-DD: state 4 NOT OBSERVED in this delivery ...
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;
class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void CrimeShadowStartup(bool simulatorReplaced, bool crimeReplaced);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is on
// and `target` is the original crime simulator's agent, mirrors its pipeline;
// for the delivery that runs state 4 it runs before / target->DoMessage(msg) /
// after, stores the agent's result in `finished` and returns true. Otherwise
// returns false (the caller delivers).
bool CrimeShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                        int64_t data1, bool* finished);

}  // namespace fc4

// Test seam op 13 (FC4TestSeam.h): one month of the original crime simulator
// (a fake): SimNewMonth, then SimNewDay deliveries until the shadow has
// wrapped pipeline state 4 (at most 6).
struct FC4CrimeShadowTestInput {
    cISC4City* city;             // GetCrimeSimulator (55), GetLotManager (33), GetZoneManager (36)
    cIGZMessageTarget2* target;  // the deliveries' target (the original's agent)
    const uint32_t* typeIds;     // the crime type exemplars (instance = type ID) ...
    const uint32_t* severities;  // ... and their severities (property 0xAA12FCB5)
    uint32_t typeCount;
    float decay;         // tuning 0x8A0CBAAC
    uint32_t tractSize;  // tuning 0x47E2C200
    int32_t julian;      // the first SimNewDay's date
};
// Returns the month's mismatch count, -1 if the shadow took no delivery, -2 if
// it wrapped one but compared nothing.
intptr_t CrimeShadowTestRun(const FC4CrimeShadowTestInput* in);
