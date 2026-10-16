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

// The automata layer's host tables (FC4AutomataHost.h; docs/specs/automata.md).
#include "FC4AutomataHost.h"

#include "FC4AutomataInterfaces.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"  // TokenOf

#include "cGZPersistResourceKey.h"
#include "cIGZUnknown.h"
#include "cISC4City.h"
#include "cS3DVector3.h"
#include "SC4String.h"

#include <cinttypes>
#include <cstdio>

namespace fc4 {
namespace {

// The building occupant type of 8.2 (a fire's target gets the controllers).
constexpr uint32_t kOccupantTypeBuilding = 0x278128A0;
// The riot leader's automata group (8.1 step 3).
constexpr uint32_t kRiotAutomataGroup = 0x4107;
// The traffic network types that count as a road (disasters.md 6.4 step 1.3):
// road 0 -> 0x1, street 3 -> 0x8, avenue 6 -> 0x40.
constexpr uint32_t kRoadTypeBits[3] = {0x1u, 0x8u, 0x40u};
// The spiral search of disasters.md 6.4 step 1: Chebyshev distance <= 7.
constexpr int32_t kSpiralRings = 7;
// The maximum leaders of disasters.md 6.1 (the shipped tuning is 10).
constexpr uint32_t kDefaultMaxLeaders = 10;

// ---- borrowed game services ----------------------------------------------

cISC4City* City(void* user) { return static_cast<AutomataHostContext*>(user)->city; }

cFC4ISC4AutomataManager* Pedestrian(cISC4City* city) {
    return city != nullptr ? AsGame(city->GetPedestrianManager()) : nullptr;  // slot 44
}

cFC4ISC4AutomataControllerManager* Controllers(cISC4City* city) {
    return city != nullptr ? AsGame(city->GetAutomataControllerManager()) : nullptr;  // slot 47
}

cFC4ISC4DisasterLayer* Layer(cISC4City* city) {
    return city != nullptr ? reinterpret_cast<cFC4ISC4DisasterLayer*>(city->GetDisasterLayer()) : nullptr;
}

cFC4ISC4TrafficNetworkMap* Map(cISC4City* city) {
    return city != nullptr
               ? reinterpret_cast<cFC4ISC4TrafficNetworkMap*>(
                     reinterpret_cast<void*>(city->GetTrafficNetwork()))
               : nullptr;
}

cFC4ISC4Automaton* AutomatonOf(uint64_t token) {
    return reinterpret_cast<cFC4ISC4Automaton*>(token);
}

cISC4Occupant* OccupantOf(uint64_t token) { return reinterpret_cast<cISC4Occupant*>(token); }

// ---- riot (8.1) -----------------------------------------------------------

// Step 1: the nearest road tile's position, or none. The clockwise spiral of
// disasters.md 6.4 step 1, over the traffic network map's tile lookup: the
// centre first, then each ring starting at (x + r, z - r + 1) and running +z
// first, cells outside the city skipped. A cell passes when its tile holds a
// network occupant of type 0, 3 or 6 (road, street, avenue).
bool NearestRoadTile(void* user, int32_t x, int32_t z, float* position) {
    cISC4City* city = City(user);
    cFC4ISC4TrafficNetworkMap* map = Map(city);
    if (map == nullptr || position == nullptr) return false;
    for (int32_t r = 0; r <= kSpiralRings; ++r) {
        if (r == 0) {
            void* tile = nullptr;
            for (uint32_t bit : kRoadTypeBits) {
                tile = map->FindNetworkTile(x, z, bit, true);
                if (tile != nullptr) break;
            }
            if (tile != nullptr) {
                position[0] = static_cast<float>(x) * 16.0f;
                position[1] = 0.0f;
                position[2] = static_cast<float>(z) * 16.0f;
                return true;
            }
            continue;
        }
        for (int32_t i = 0; i < 2 * r; ++i) {
            const int32_t cx = x + r;
            const int32_t cz = z - r + 1 + i;
            for (int32_t side = 0; side < 4; ++side) {
                int32_t px = cx, pz = cz;
                if (side == 1) { px = x + r - i; pz = z + r; }
                if (side == 2) { px = x - r; pz = z + r - i; }
                if (side == 3) { px = x - r + i; pz = z - r; }
                void* tile = nullptr;
                for (uint32_t bit : kRoadTypeBits) {
                    tile = map->FindNetworkTile(px, pz, bit, true);
                    if (tile != nullptr) break;
                }
                if (tile != nullptr) {
                    position[0] = static_cast<float>(px) * 16.0f;
                    position[1] = 0.0f;
                    position[2] = static_cast<float>(pz) * 16.0f;
                    return true;
                }
            }
        }
    }
    return false;
}

// Step 2: the leader count has reached its cap (6.1). The police-coverage
// half of the original's check is the disaster layer's (disasters.md 6.4
// step 2) and is not repeated here; the core's contract is the count.
bool LeaderCapReached(void* user, uint32_t leaders) {
    AutomataHostContext* ctx = static_cast<AutomataHostContext*>(user);
    const uint32_t maxLeaders = ctx->maxLeaders != 0 ? ctx->maxLeaders : kDefaultMaxLeaders;
    return leaders >= maxLeaders;
}

// Step 3: CreateAutomaton(0x4107, position, 2, zero key, null, 0xFF, null)
// on the pedestrian manager (7.2's caller table).
uint64_t CreateAutomaton(void* user, const fc4_automata_create_args* args) {
    AutomataHostContext* ctx = static_cast<AutomataHostContext*>(user);
    cFC4ISC4AutomataManager* m = Pedestrian(City(user));
    if (m == nullptr || args == nullptr) return 0;
    cS3DVector3 position(args->position[0], args->position[1], args->position[2]);
    cGZPersistResourceKey key(args->model_key[0], args->model_key[1], args->model_key[2]);
    cS3DVector3 orientation(args->orientation[0], args->orientation[1], args->orientation[2]);
    cS3DVector3* orient = args->has_orientation != 0 ? &orientation : nullptr;
    cISC4Automaton* a = m->CreateAutomaton(args->group, position, args->flags, key,
                                           reinterpret_cast<cIGZMessageTarget2*>(args->callback),
                                           args->priority, orient);
    if (a == nullptr) {
        ++ctx->refused;
        return 0;
    }
    ++ctx->created;
    return TokenOf(a);
}

void AddRef(void* /*user*/, uint64_t automaton) {
    cIGZUnknown* a = reinterpret_cast<cIGZUnknown*>(automaton);
    if (a != nullptr) FC4_ADDREF(a);
}

void Release(void* /*user*/, uint64_t automaton) {
    cIGZUnknown* a = reinterpret_cast<cIGZUnknown*>(automaton);
    if (a != nullptr) FC4_RELEASE(a);
}

// Step 5: SetState(8, 0, -1.0, 1, 0, 0, 0) for the start; the end's
// SetState(1, ...) is made the same way and is a no-op against a `final`
// leader (8.1 "Corrects"): the call happens, its result is not read.
void SetState(void* user, uint64_t automaton, uint32_t stateType, uint32_t a, float b, uint32_t c,
              uint32_t d, uint32_t e, uint32_t f) {
    AutomataHostContext* ctx = static_cast<AutomataHostContext*>(user);
    cFC4ISC4Automaton* p = AutomatonOf(automaton);
    if (p == nullptr) return;
    ++ctx->stateCalls;
    p->SetState(stateType, a, b, c, d, e, f);
}

void SetVisible(void* /*user*/, uint64_t automaton, bool visible) {
    cFC4ISC4Automaton* p = AutomatonOf(automaton);
    if (p != nullptr) p->SetVisible(visible);
}

uint64_t AsOccupant(void* /*user*/, uint64_t automaton) {
    cFC4ISC4Automaton* p = AutomatonOf(automaton);
    return p != nullptr ? TokenOf(p->AsOccupant()) : 0;
}

// Step 7: register the leader's location with the disaster layer (city slot
// 72, slot 13).
void RegisterLocation(void* user, uint64_t automaton, const float* position) {
    cFC4ISC4DisasterLayer* layer = Layer(City(user));
    cFC4ISC4Automaton* p = AutomatonOf(automaton);
    if (layer == nullptr || p == nullptr) return;
    const int32_t x = position != nullptr ? static_cast<int32_t>(position[0] / 16.0f) : 0;
    const int32_t z = position != nullptr ? static_cast<int32_t>(position[2] / 16.0f) : 0;
    layer->RegisterDisasterLocation(nullptr, x, z, p->AsOccupant(), 0, false);
}

// Stage rise (8.1): AddControllerFromTemplateName(leader as occupant,
// "riot_leader_<stage>", 0) on the controller manager (city slot 47, slot 5).
bool AddControllerFromTemplateName(void* user, uint64_t occupant, const char* name, uint32_t extra) {
    AutomataHostContext* ctx = static_cast<AutomataHostContext*>(user);
    cFC4ISC4AutomataControllerManager* cm = Controllers(City(user));
    if (cm == nullptr || name == nullptr) return false;
    ++ctx->stageCalls;
    SC4String s(name);
    return cm->AddControllerFromTemplateName(OccupantOf(occupant), s,
                                             reinterpret_cast<float const*>(static_cast<uintptr_t>(extra))) !=
           nullptr;
}

// The end (8.1 step 2) and the fire occupant's Shutdown (8.2): controller
// manager slot 14, RemoveControllers(occupant).
void RemoveControllers(void* user, uint64_t occupant) {
    cFC4ISC4AutomataControllerManager* cm = Controllers(City(user));
    if (cm != nullptr) cm->RemoveControllers(OccupantOf(occupant));
}

// The end (8.1 step 3): disaster layer slot 15, by occupant.
void UnregisterLocation(void* user, uint64_t occupant) {
    cFC4ISC4DisasterLayer* layer = Layer(City(user));
    if (layer != nullptr) layer->UnregisterDisasterLocationByOccupant(OccupantOf(occupant));
}

// ---- fire (8.2) -----------------------------------------------------------

uint32_t OccupantType(void* /*user*/, uint64_t occupant) {
    cISC4Occupant* o = OccupantOf(occupant);
    cFC4ISC4Occupant* p = reinterpret_cast<cFC4ISC4Occupant*>(o);
    return p != nullptr ? p->GetType() : 0;
}

// SetTarget: a building (0x278128A0) gets the "fire_occupant" controllers
// (controller manager slot 10, the by-name AddControllersFromOccupantGroup).
bool AddControllersFromOccupantGroupName(void* user, const char* name, uint64_t occupant, uint32_t extra) {
    AutomataHostContext* ctx = static_cast<AutomataHostContext*>(user);
    cFC4ISC4AutomataControllerManager* cm = Controllers(City(user));
    if (cm == nullptr || name == nullptr) return false;
    ++ctx->fireCalls;
    SC4String s(name);
    return cm->AddControllersFromOccupantGroupName(s, OccupantOf(occupant),
                                                   reinterpret_cast<float*>(static_cast<uintptr_t>(extra)));
}

}  // namespace

fc4_automata_riot_host MakeAutomataRiotHost(AutomataHostContext* ctx) {
    fc4_automata_riot_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.nearest_road_tile = &NearestRoadTile;
    h.leader_cap_reached = &LeaderCapReached;
    h.create_automaton = &CreateAutomaton;
    h.add_ref = &AddRef;
    h.set_state = &SetState;
    h.set_visible = &SetVisible;
    h.as_occupant = &AsOccupant;
    h.register_location = &RegisterLocation;
    h.add_controller_from_template_name = &AddControllerFromTemplateName;
    h.remove_controllers = &RemoveControllers;
    h.unregister_location = &UnregisterLocation;
    h.release = &Release;
    return h;
}

fc4_automata_fire_host MakeAutomataFireHost(AutomataHostContext* ctx) {
    fc4_automata_fire_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.occupant_type = &OccupantType;
    h.add_controllers_from_occupant_group_name = &AddControllersFromOccupantGroupName;
    h.remove_controllers = &RemoveControllers;
    return h;
}

void AutomataPauseAll(cISC4City* city) {
    cISC4AutomataManager* all[4] = {city != nullptr ? city->GetVehicleManager() : nullptr,
                                    city != nullptr ? city->GetPedestrianManager() : nullptr,
                                    city != nullptr ? city->GetAircraftManager() : nullptr,
                                    city != nullptr ? city->GetWatercraftManager() : nullptr};
    for (cISC4AutomataManager* m : all) {
        if (m != nullptr) AsGame(m)->Pause();
    }
}

void AutomataResumeAll(cISC4City* city) {
    cISC4AutomataManager* all[4] = {city != nullptr ? city->GetVehicleManager() : nullptr,
                                    city != nullptr ? city->GetPedestrianManager() : nullptr,
                                    city != nullptr ? city->GetAircraftManager() : nullptr,
                                    city != nullptr ? city->GetWatercraftManager() : nullptr};
    for (cISC4AutomataManager* m : all) {
        if (m != nullptr) AsGame(m)->Resume();
    }
}

}  // namespace fc4
