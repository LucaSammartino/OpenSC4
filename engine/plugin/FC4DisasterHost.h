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

// The disaster layer's host table (fc4_disasters_host, fc4.h; disasters.md)
// on the city's managers: the occupant manager, the occupants and their
// properties, the lot manager, the fire protection, police and flammability
// grids, the traffic network map, the simulator's pause, the tutorial, the
// weather, the message server and the wall clock.
//
// Two modes:
//   - live (a layer replacement): reads and writes go to the game. The fire
//     occupant class 0x69D25C5F (§5.11, DS26) and the riot leader automata
//     (§6.4, automata.md §8.1, DS27) are created through GZCOM and the
//     pedestrian manager; the writes the specs leave open are counted, logged
//     once and not made (terrain brushes, pipes, the volcano, effects and the
//     "RiotN" effect names; questions Q29-Q31 of the lane report);
//   - shadow (FC4DisasterShadow.h): reads go to the game; every write is
//     recorded in the context instead (nothing in the game changes); the
//     BURNING property (0x69DBEBE1) of an occupant is answered from the
//     mirror's own fire occupants; fire occupants are synthetic tokens; the
//     riot leaders are the original's automata the shadow hands over; now_ms
//     is the shadow's clock.
//
// Tokens: an occupant is its cISC4Occupant* (TokenOf); a synthetic token has
// bit 63 set and is never called through.
#pragma once

#include "fc4.h"

#include "SC4List.h"

#include <cstdint>
#include <deque>
#include <functional>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <tuple>
#include <vector>

class cISC4Automaton;
class cISC4City;
class cISC4FireOccupant;
class cISC4Occupant;

namespace fc4 {

constexpr uint64_t kSyntheticToken = 1ull << 63;

inline bool IsSynthetic(uint64_t t) { return (t & kSyntheticToken) != 0; }

struct DisasterHostContext {
    cISC4City* city = nullptr;  // borrowed for the city session
    bool shadow = false;
    // A replacement routes period posts through its public layer, after the
    // manager core returns. Other host users retain the normal message path.
    std::function<void(const fc4_disaster_post&)> postOverride;

    // ---- shadow mode ----
    // The clock now_ms answers (the shadow sets it before each core call).
    uint64_t clockMs = 0;
    // The pause state sampled before the original's call that is mirrored.
    bool pausedSample = false, emergencySample = false;
    // The mirror's fire occupants: target -> fire token, and back.
    std::map<uint64_t, uint64_t> burning;
    uint64_t nextSynthetic = 1;
    // Leaders handed to create_riot_leader (the original's automata, in the
    // order the original registered them), per riot context.
    std::deque<uint64_t> leaders;
    uint64_t leadersMissing = 0;  // create_riot_leader with none handed over
    // Recorded writes.
    std::map<uint32_t, uint64_t> posts;          // per message type
    std::map<uint32_t, uint64_t> postData1Sums;  // 0x49D15B73: buildings
    std::map<uint32_t, uint64_t> postData2Sums;  // 0x49D15B73: flora; 0x49E3DB1D: started
    std::map<uint64_t, uint32_t> lotConditions;  // lot token -> the last condition set
    std::set<uint64_t> locked;                   // unset_removal_flags(2) minus set_removal_flags(2)
    std::map<std::string, uint64_t> writes;      // per callback name
    std::map<uint32_t, int64_t> history;         // series -> sum
    int32_t pauses = 0, saveOff = 0;
    // The core's subscriptions (target kind, id, message type), for the
    // shadow's router.
    std::set<std::tuple<uint32_t, uint32_t, uint32_t>> subscriptions;
    std::set<uint32_t> ticks;  // instances registered for the animation tick
    // Real references the core holds (shadow and live): token -> count.
    std::map<uint64_t, int64_t> refs;

    // ---- live mode ----
    std::set<std::string> notWired;  // callbacks whose write is not made (logged once each)
    // The game objects the live creates made, one reference each, keyed by
    // their token: released by the matching remove / end and by
    // DisasterHostReleaseAll (a fire occupant's token is its cISC4Occupant*,
    // a leader's its cISC4Automaton*).
    std::map<uint64_t, cISC4FireOccupant*> fireOccupants;
    std::map<uint64_t, cISC4Automaton*> riotLeaders;

    // Lists from occupant queries, held until the outermost core call returns.
    std::vector<std::shared_ptr<void>> held;
};

// The host table over `ctx` (which must outlive the core).
fc4_disasters_host MakeDisasterHost(DisasterHostContext* ctx);

// Releases every reference the core left (city shutdown, after the core is
// destroyed) and the held lists.
void DisasterHostReleaseAll(DisasterHostContext* ctx);
void DisasterHostReleaseHeld(DisasterHostContext* ctx);

// The tuning exemplars (fire "Flame" 0xC9B6B73D, riot 0x4A38206D; type
// 0x6534284A, group 0x29B6C670; disasters.md §5.1, §6.1) handed to the core.
// Returns how many were found.
int DisasterHostLoadTuning(fc4_disasters* core);

// disasters.md §5.4 "The refusals, exactly": one `Ignite`'s original-side
// inputs, for the shadow's log at a refusal (DS38). `rect` is the clamped
// cell rectangle of step 4.1 and `coverage` its average. `exits` is a bitmask
// of the exits those inputs admit (bit 0 = exit 1, ... bit 4 = exit 5); exit 4
// is not in the mask (it needs the D9 curve), so read it from `coverage`.
struct DisasterIgniteInputs {
    uint32_t type = 0;
    uint32_t flammability = 0;  // 0x29244DB5, read as an unsigned integer
    int32_t x0 = 0, z0 = 0, x1 = 0, z1 = 0;
    uint8_t coverage = 0;
    bool burning = false;  // 0x69DBEBE1, the live game value
    uint32_t exits = 0;
};
DisasterIgniteInputs DisasterIgniteInputsOf(cISC4City* city, cISC4Occupant* o, bool force, bool paused,
                                            bool emergencyPaused);

// Helpers the shadow shares.
uint8_t DisasterFireCoverage(cISC4City* city, cISC4Occupant* o);  // §5.4 step 4.1
uint8_t DisasterPoliceCoverage(cISC4City* city, int32_t x, int32_t z);
bool DisasterIsRoadCell(cISC4City* city, int32_t x, int32_t z);
uint64_t DisasterNetworkOccupantAt(cISC4City* city, int32_t x, int32_t z);
bool DisasterOccupantCell(cISC4City* city, cISC4Occupant* o, int32_t* x, int32_t* z);
bool DisasterOccupantBurning(cISC4Occupant* o);  // the live 0x69DBEBE1
// Every occupant of `type` in the cell rectangle (occupant manager slot 41):
// the list holds one reference each until it goes (`holdIn`: kept in the
// context until DisasterHostReleaseHeld); nullptr without a city.
std::shared_ptr<SC4List<cISC4Occupant*>> DisasterOccupantsIn(cISC4City* city, int32_t x0, int32_t z0, int32_t x1,
                                                             int32_t z1, uint32_t type, DisasterHostContext* holdIn);

}  // namespace fc4
