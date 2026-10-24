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

#include "FC4DisasterShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4DisasterHost.h"
#include "FC4Fpu.h"
#include "FC4HarnessLogic.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4Turbo.h"
#include "compat/FC4Compat.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "SC4Rect.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4City.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4Simulator.h"
#include "cISC4DisasterInstance.h"
#include "cISC4DisasterInstanceFire.h"
#include "cISC4DisasterLayer.h"
#include "cISC4DisasterManager.h"
#include "cISC4DisasterManagerFire.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISCPropertyHolder.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <algorithm>
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <deque>
#include <map>
#include <set>
#include <string>
#include <vector>

namespace fc4 {

// Defined below (outside the anonymous namespace): drives the mirror's own
// fire tick when the original refused an ignition (the resync).
void ResyncTick();
namespace {

using compat::DisasterHook;

constexpr uint32_t kMsgAnimationTick = 0x86AD10EE;  // §5.8
constexpr uint32_t kFire = 0x621CDA33;
constexpr uint32_t kRiot = 0xCA380B06;
constexpr uint32_t kIID_cISC4DisasterManagerFire = 0x49B43F57;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kTypeFlora = 0x74758926;
constexpr uint32_t kTypeBuilding = 0x278128A0;
constexpr uint32_t kTypeProp = 0xA823821E;
constexpr uint32_t kPropFlammability = 0x29244DB5;
// The original's posts (§12) the subscription counts.
constexpr uint32_t kCountedPosts[] = {0x49E3DB1D, 0x49D15B73, 0x49D15B72, 0xA9C7CAB6, 0xCA1B1E64, 0xAA1B2002};
// The messages the mirror core subscribes to (§2.5, §5.3), routed to it.
constexpr uint32_t kRoutedTypes[] = {0x99EF1143, 0xE94671EC, 0xE94671EB, 0x894671F6,
                                     0xE94671ED, 0x06CDB65B, 0xA6B79602};
constexpr int kQuietFrames = 30;
constexpr size_t kMaxDiffs = 6;

const char* HookName(DisasterHook h) {
    switch (h) {
        case DisasterHook::LayerBegun: return "layer DisasterHasBegun";
        case DisasterHook::LayerEnded: return "layer DisasterHasEnded";
        case DisasterHook::LayerRegister: return "layer RegisterDisasterLocation";
        case DisasterHook::LayerUnregisterAll: return "layer UnregisterDisasterLocations";
        case DisasterHook::LayerUnregisterByOcc: return "layer UnregisterDisasterLocationByOccupant";
        case DisasterHook::FireIgnite: return "fire Ignite";
        case DisasterHook::FireIgniteCell: return "fire IgniteCell";
        case DisasterHook::FireExtinguishCell: return "fire ExtinguishCell";
        case DisasterHook::FireQuickBurnCell: return "fire QuickBurnCell";
        case DisasterHook::FireIgniteCellRadius: return "fire IgniteCellRadius";
        case DisasterHook::FireExtinguish: return "fire Extinguish";
        case DisasterHook::FireBurnDown: return "fire BurnDown";
        case DisasterHook::FireSetCause: return "fire SetCause";
        case DisasterHook::FireMessage: return "fire DoMessage";
        case DisasterHook::FireManagerIgniteOcc: return "fire manager IgniteOccupant";
        case DisasterHook::RiotStart: return "riot AttemptDisasterStart(x, z)";
        case DisasterHook::RiotRemoveInstance: return "riot RemoveInstanceFromList";
        case DisasterHook::RiotMessage: return "riot DoMessage";
        case DisasterHook::RiotLock: return "riot LockNetworkOccupant";
        case DisasterHook::TutorialIsTutorial: return "tutorial IsTutorial";
        case DisasterHook::Count: break;
    }
    return "?";
}

struct Patch {
    void** entry = nullptr;
    uintptr_t original = 0;
    bool installed = false;
    uint64_t calls = 0;
};

struct Riot {
    uint32_t id = 0;                           // the mirror's instance
    cISC4DisasterInstance* inst = nullptr;     // the original's (one reference ours)
    std::set<uint64_t> leaders;                // the original's registered leaders
    bool removed = false;                      // RemoveInstanceFromList seen
    uint64_t lastSecond = 0, startMs = 0, updates = 0;
};

struct Counts {
    uint64_t lines = 0, clean = 0, questioned = 0;  // questioned: every difference has a question number
};

struct Shadow {
    bool enabled = false, layerReplaced = false, tried = false, installedAny = false;
    DWORD mainTid = 0;
    Patch p[static_cast<unsigned>(DisasterHook::Count)];
    cISC4City* city = nullptr;
    DisasterHostContext ctx;
    fc4_disasters_host host{};
    fc4_disasters* core = nullptr;
    bool inCore = false;
    int fireMsgDepth = 0;
    uint32_t tutCalls = 0;
    int outerMirror = 0;
    int cellDepth = 0;
    uint64_t nestedIgnites = 0;
    int managerIgniteDepth = 0;
    // The fire clock (FC4DisasterShadow.h).
    uint64_t fireClock = 0, fireNext = 0;
    bool fireNextValid = false;
    uint32_t corePhase = 0;
    uint64_t origSteps = 0, coreSteps = 0, fireStepLines = 0;
    // Fire IDs.
    std::map<uint32_t, uint32_t> idMap;
    std::deque<uint32_t> pendingIds;
    uint64_t idGuesses = 0;
    // Ignitions inside the original's fire step (a spread): mirrored after
    // the mirror's own step of that tick (a fire lit during UpdateAllFires is
    // not stepped in that pass, §5.8; the core iterates a snapshot too).
    struct Deferred {
        uint64_t o;
        bool force, focus, result, paused, emergency;
        uint32_t id;
        const char* why;
    };
    std::vector<Deferred> deferred;
    uint64_t deferredIgnites = 0;
    // Ignitions the original refused and the mirror took (Q40): the mirror's
    // fire is put out at once (it dies at the mirror's next step).
    std::set<uint64_t> resyncTargets;
    // Spreads both sides took (mirrored after the mirror's step), and whether
    // one of them outlived the original's in the mirror's current episode
    // (Q42: the original sometimes steps a fire lit during its pass).
    std::set<uint64_t> spreadTargets;
    bool lagInEpisode = false;
    uint64_t resyncs = 0, resyncSteps = 0;
    bool resyncInEpisode = false;  // a refused fire was put out in the mirror's current episode  // resyncSteps: mirror steps with no original fire tick
    // The original's fire instance.
    void* fireInternal = nullptr;            // cISC4DisasterInstanceFire* (borrowed)
    cISC4DisasterInstance* fireInst = nullptr;  // one reference ours
    uintptr_t fireCanon = 0;
    std::set<uint64_t> targets;  // every occupant an original Ignite saw (one reference ours each)
    // Riots, by the original's canonical identity.
    std::map<uintptr_t, Riot> riots;
    std::map<uintptr_t, std::deque<uint64_t>> pendingLeaders;
    std::set<uint64_t> origLocked;  // one reference ours each
    int riotStartDepth = 0;
    int riotMsgDepth = 0;
    uint32_t riotLocks = 0, riotRegisters = 0, riotUnregisters = 0;
    uint32_t riotTickLines = 0;
    cISC4DisasterInstance* riotStartInst = nullptr;  // the first riot instance registered during the start
    int64_t origBegun = 0, origEnded = 0;
    // The original's posts.
    std::map<uint32_t, uint64_t> origPosts, origD1, origD2;
    bool listening = false;
    // Quiet point.
    bool dirty = false;
    int quiet = 0;
    // Lines.
    Counts fire, riot, quietLines, calls;
    // Regression tooling only: cumulative failures cannot disappear between months.
    uint64_t differences = 0, failures = 0;
    int32_t reportMonth = 0;
    bool scripted = false, restoreTurbo = false;
    uint32_t scriptStage = 0;
    uint64_t unfollowable = 0, radiusCalls = 0, cellFallbacks = 0, extinguishCalls = 0;
};
Shadow g;

bool Ready() {
    return g.core != nullptr && !g.inCore && g.city != nullptr && GetCurrentThreadId() == g.mainTid;
}

// A value that cannot be an object (a message's void1 that is a number,
// or a misread argument): never called through.
bool NotAnObject(const void* p) { return reinterpret_cast<uintptr_t>(p) < 0x10000; }

uintptr_t Canon(cIGZUnknown* u) {
    if (u == nullptr) return 0;
    if (NotAnObject(u)) return reinterpret_cast<uintptr_t>(u);
    void* p = nullptr;
    if (!u->QueryInterface(GZIID_cIGZUnknown, &p) || p == nullptr) return reinterpret_cast<uintptr_t>(u);
    cIGZUnknown* q = static_cast<cIGZUnknown*>(p);
    FC4_ACQUIRED(q, "QueryInterface");
    FC4_RELEASE(q);
    return reinterpret_cast<uintptr_t>(q);
}

uint64_t OccToken(cIGZUnknown* u) {
    if (u == nullptr) return 0;
    if (NotAnObject(u)) return TokenOf(u);
    void* p = nullptr;
    if (!u->QueryInterface(kIID_cISC4Occupant, &p) || p == nullptr) return TokenOf(u);
    cIGZUnknown* q = static_cast<cIGZUnknown*>(p);
    FC4_ACQUIRED(q, "QueryInterface");
    FC4_RELEASE(q);
    return TokenOf(q);
}

cISC4Occupant* AsOcc(uint64_t t) {
    return (t == 0 || IsSynthetic(t)) ? nullptr : reinterpret_cast<cISC4Occupant*>(static_cast<uintptr_t>(t));
}

// Keeps one reference on an occupant the comparison reads later.
void Keep(std::set<uint64_t>& set, uint64_t t) {
    cISC4Occupant* o = AsOcc(t);
    if (o == nullptr || !set.insert(t).second) return;
    FC4_ADDREF(o);
}

void DropAll(std::set<uint64_t>& set) {
    for (uint64_t t : set) {
        if (cISC4Occupant* o = AsOcc(t)) FC4_RELEASE(o);
    }
    set.clear();
}

// Drops the kept reference of every occupant that has left the occupant
// manager (flag 1 clear, occupant-manager.md §5.2). The comparison skips them
// anyway, and holding them until the city closes would keep a burned-down
// building alive and then release the game's last reference (refaudit).
void PruneGone(std::set<uint64_t>& set) {
    for (auto it = set.begin(); it != set.end();) {
        cISC4Occupant* o = AsOcc(*it);
        if (o != nullptr && o->IsFlagSet(1)) {
            ++it;
            continue;
        }
        if (o != nullptr) FC4_RELEASE(o);
        it = set.erase(it);
    }
}

// ---- calls into the mirror core ----------------------------------------------------------------

template <class F>
struct CoreStep {
    F* f;
    static void Run(void* p) { (*static_cast<CoreStep*>(p)->f)(); }
};

// Runs `f` (which calls the core) with the fire clock or the real clock,
// under the SEH guard and the re-entrancy flag; the occupant lists the host
// took go afterwards.
template <class F>
bool CoreCall(const char* what, bool fireClock, F f) {
    g.ctx.clockMs = fireClock ? g.fireClock : 0;
    if (!fireClock) {
        LARGE_INTEGER fr{}, now{};
        QueryPerformanceFrequency(&fr);
        QueryPerformanceCounter(&now);
        g.ctx.clockMs = static_cast<uint64_t>(now.QuadPart / (fr.QuadPart / 1000));
    }
    g.inCore = true;
    CoreStep<F> s{&f};
    const bool ok = SehGuard(what, &CoreStep<F>::Run, &s);
    if (!ok) ++g.failures;
    g.inCore = false;
    DisasterHostReleaseHeld(&g.ctx);
    return ok;
}

fc4_disasters_state CoreState() {
    fc4_disasters_state s{};
    s.struct_size = sizeof(s);
    if (g.core) fc4_disasters_get_state(g.core, &s);
    return s;
}

// The core's tutorial read: counted as one mirror step (§5.8 UpdateAllFires).
bool CoreTutorial(void* u) {
    ++g.coreSteps;
    return MakeDisasterHost(static_cast<DisasterHostContext*>(u)).tutorial_no_burn_down(u);
}

// After a mirrored fire call: the phase the core entered sets the clock's
// next step (the core's `next := elapsed`, §5.9).
void TrackFirePhase() {
    const uint32_t phase = CoreState().fire_phase;
    if (phase == 1 && g.corePhase != 1) {
        g.fireNext = g.fireClock;
        g.fireNextValid = true;
    } else if (phase != 1) {
        g.fireNextValid = false;
        g.resyncInEpisode = false;
        g.lagInEpisode = false;
    }
    g.corePhase = phase;
}

uint32_t MapId(uint32_t id) {
    if (id == 0) return 0;
    auto it = g.idMap.find(id);
    if (it != g.idMap.end()) return it->second;
    uint32_t mapped = id;
    if (!g.pendingIds.empty()) {
        mapped = g.pendingIds.front();  // [choice, Q26]
        g.pendingIds.pop_front();
        ++g.idGuesses;
    }
    g.idMap[id] = mapped;
    return mapped;
}

std::set<uint32_t> CoreFireIds() {
    std::set<uint32_t> ids;
    const size_t n = fc4_disasters_fire_ids(g.core, nullptr, 0);
    std::vector<uint32_t> v(n);
    if (n) fc4_disasters_fire_ids(g.core, v.data(), n);
    ids.insert(v.begin(), v.end());
    return ids;
}

// ---- the original's state ----------------------------------------------------------------------

cISC4DisasterLayer* Layer() { return g.city ? g.city->GetDisasterLayer() : nullptr; }  // slot 72, borrowed

cISC4DisasterManager* Manager(uint32_t type) {
    cISC4DisasterLayer* l = Layer();
    return l ? l->GetDisasterManager(type) : nullptr;  // slot 7, borrowed (no AddRef, §2.2)
}

int32_t OrigFireCount() {
    if (g.fireInternal == nullptr) return 0;
    return static_cast<cISC4DisasterInstanceFire*>(g.fireInternal)->GetFireCount();  // slot 16
}

uint32_t CoreInstanceOf(cISC4DisasterInstance* inst) {
    if (inst == nullptr) return 0;
    const uintptr_t c = Canon(inst);
    if (c == g.fireCanon && c != 0) return CoreState().fire_instance;
    auto it = g.riots.find(c);
    return it != g.riots.end() ? it->second.id : 0xFFFFFFFFu;
}

// Each difference, tallied per kind (the format up to its first number)
// and per question for the city shutdown line.
std::map<std::string, uint64_t>& DiffKinds() {
    static std::map<std::string, uint64_t> m;
    return m;
}

struct Diffs {
    const char* scope = "";
    uint32_t n = 0, unexplained = 0;
    std::string text;
    void Add(const char* fmt, ...) {
        va_list ap;
        va_start(ap, fmt);
        Put(nullptr, fmt, ap);
        va_end(ap);
    }
    // A difference a numbered question of the lane report explains.
    void AddQ(const char* q, const char* fmt, ...) {
        va_list ap;
        va_start(ap, fmt);
        Put(q, fmt, ap);
        va_end(ap);
    }

private:
    void Put(const char* q, const char* fmt, va_list ap) {
        ++n;
        ++g.differences;
        if (q == nullptr) ++unexplained;
        std::string kind = std::string(scope) + " " + std::string(fmt).substr(0, std::string(fmt).find('%'));
        while (!kind.empty() && (kind.back() == ' ' || kind.back() == '(')) kind.pop_back();
        if (q != nullptr) kind += std::string(" [") + q + "]";
        ++DiffKinds()[kind];
        if (n > kMaxDiffs) return;
        char b[256];
        std::vsnprintf(b, sizeof(b), fmt, ap);
        if (!text.empty()) text += "; ";
        text += b;
        if (q != nullptr) text += std::string(" [") + q + "]";
    }
};

// The layer: ongoing count and the location list (instance, x, z).
// `countQ`: the question that explains a count difference (ongoing, the
// number of locations), or null.
void CompareLayer(Diffs& d, char* out, size_t cap, const char* countQ) {
    const fc4_disasters_state s = CoreState();
    const int64_t ongoing = g.origBegun - g.origEnded;
    if (ongoing != static_cast<int64_t>(s.ongoing)) {
        if (countQ != nullptr) {
            d.AddQ(countQ, "ongoing orig %lld ours %u", (long long)ongoing, s.ongoing);
        } else {
            d.Add("ongoing orig %lld ours %u", (long long)ongoing, s.ongoing);
        }
    }
    cISC4DisasterLayer* l = Layer();
    const int32_t n = l ? l->GetDisasterLocationCount() : 0;  // slot 16
    if (n != static_cast<int32_t>(s.location_count)) {
        if (countQ != nullptr) {
            d.AddQ(countQ, "locations orig %d ours %u", n, s.location_count);
        } else {
            d.Add("locations orig %d ours %u", n, s.location_count);
        }
    }
    for (int32_t i = 0; l != nullptr && i < n && i < static_cast<int32_t>(s.location_count); ++i) {
        cISC4DisasterInstance* inst = nullptr;
        int x = 0, z = 0;
        l->GetDisasterLocationByIndex(i, &inst, x, z);  // slot 17
        fc4_disaster_location loc{};
        fc4_disasters_get_location(g.core, static_cast<size_t>(i), &loc);
        const uint32_t mapped = CoreInstanceOf(inst);
        if (mapped != loc.instance || x != loc.x || z != loc.z) {
            // An occupant entry (a riot leader, a fire's target): the original's
            // reads (0, 0) or the cell where it was registered with focus, never
            // the occupant's cell of now (Q37).
            const bool occupantEntry = mapped == loc.instance && loc.occupant != 0;
            if (occupantEntry) {
                d.AddQ("Q37",
                       "location %d orig (inst %u, %d,%d) ours (inst %u, %d,%d occ %016llX in-manager %d)", i,
                       mapped, x, z, loc.instance, loc.x, loc.z,
                       (unsigned long long)loc.occupant,
                       loc.occupant != 0 && AsOcc(loc.occupant) != nullptr && AsOcc(loc.occupant)->IsFlagSet(1) ? 1 : 0);
                continue;
            }
            d.Add("location %d orig (inst %u, %d,%d) ours (inst %u, %d,%d)", i, mapped, x, z, loc.instance, loc.x,
                  loc.z);
        }
    }
    std::snprintf(out, cap, "ongoing %lld/%u locations %d/%u", (long long)ongoing, s.ongoing, n, s.location_count);
}

// ---- comparisons and lines ---------------------------------------------------------------------

void FireLine(const char* what) {
    Diffs d;
    PruneGone(g.targets);
    d.scope = "fire";
    const fc4_disasters_state s = CoreState();
    const int32_t origCount = OrigFireCount();
    // Burning targets: the core's fires against BURNING of every target
    // either side knows (in the occupant manager only).
    std::set<uint64_t> ours;
    for (size_t i = 0;; ++i) {
        fc4_disaster_fire f{};
        if (!fc4_disasters_get_fire(g.core, i, &f)) break;
        ours.insert(f.target);
    }
    // A fire of ours the original refused (Q40), until it dies, or the
    // episode only such a fire began in the mirror (until it ends).
    bool resync = false, onlyResync = true;
    for (uint64_t t : ours) {
        const bool r = g.resyncTargets.count(t) != 0;
        resync = resync || r;
        onlyResync = onlyResync && r;
    }
    const int32_t origPhase = g.fireInst ? g.fireInst->GetPhase() : 0;  // slot 7
    resync = resync || (g.resyncs != 0 && onlyResync && origPhase != 1 &&
                        s.fire_phase != static_cast<uint32_t>(origPhase));
    // [question Q42] A spread's fire still burning in the mirror after the
    // original's has gone: the original stepped it in the pass that lit it
    // (here and there; the mirror never does, §5.8).
    std::set<uint64_t> all = g.targets;
    all.insert(ours.begin(), ours.end());
    int32_t lags = 0;
    bool otherBurning = false;
    for (uint64_t t : all) {
        cISC4Occupant* o = AsOcc(t);
        if (o == nullptr || !o->IsFlagSet(1)) continue;
        const bool ob = DisasterOccupantBurning(o);
        const bool cb = ours.count(t) != 0;
        if (ob == cb) continue;
        if (!ob && g.spreadTargets.count(t) != 0) {
            ++lags;
        } else {
            otherBurning = true;
        }
    }
    if (lags != 0) g.lagInEpisode = true;
    const bool lagOnly = lags != 0 && !otherBurning && static_cast<int32_t>(s.fire_count) - origCount == lags;
    if (origCount != static_cast<int32_t>(s.fire_count)) {
        if (resync) {
            d.AddQ("Q40", "fires orig %d ours %u", origCount, s.fire_count);
        } else if (lagOnly) {
            d.AddQ("Q42", "fires orig %d ours %u", origCount, s.fire_count);
        } else {
            d.Add("fires orig %d ours %u", origCount, s.fire_count);
        }
    }
    uint32_t origBurning = 0, oursBurning = 0, lots = 0;
    for (uint64_t t : all) {
        cISC4Occupant* o = AsOcc(t);
        if (o == nullptr || !o->IsFlagSet(1)) continue;
        const bool ob = DisasterOccupantBurning(o);
        const bool cb = ours.count(t) != 0;
        origBurning += ob;
        oursBurning += cb;
        if (ob != cb && g.resyncTargets.count(t) != 0) {
            d.AddQ("Q40", "occupant %p burning orig %d ours %d", static_cast<void*>(o), ob, cb);
        } else if (ob != cb && !ob && g.spreadTargets.count(t) != 0) {
            d.AddQ("Q42", "occupant %p burning orig %d ours %d", static_cast<void*>(o), ob, cb);
        } else if (ob != cb) {
            d.Add("occupant %p burning orig %d ours %d", static_cast<void*>(o), ob, cb);
        }
        // The lot condition the mirror set (lot slot 35 against ours).
        if (cb) {
            cISC4LotManager* lm = g.city->GetLotManager();
            cISC4Lot* lot = lm ? AsGame(lm)->GetOccupantLot(o) : nullptr;
            auto it = g.ctx.lotConditions.find(TokenOf(lot));
            if (lot != nullptr && it != g.ctx.lotConditions.end()) {
                ++lots;
                const uint32_t live = AsGame(lot)->GetCondition();
                // A fire a spread lit is mirrored after the mirror's own step
                // of that tick (its `Ignite` ran inside the original's step),
                // so its stage - and the lot condition that follows it - can
                // be one step behind. Same family as the §5.8 walk (DS40).
                if (live != it->second && g.resyncTargets.count(t) != 0) {
                    d.AddQ("Q40", "lot %p condition orig %u ours %u", static_cast<void*>(lot), live, it->second);
                } else if (live != it->second && g.spreadTargets.count(t) != 0) {
                    d.AddQ("Q42", "lot %p condition orig %u ours %u", static_cast<void*>(lot), live, it->second);
                } else if (live != it->second) {
                    d.Add("lot %p condition orig %u ours %u", static_cast<void*>(lot), live, it->second);
                }
            }
        }
    }
    // After a refused ignition (Q40) the mirror can have begun and ended an
    // episode the original never had (phase 3 against 0).
    const bool phaseQ40 = resync || (g.resyncs != 0 && origPhase == 0 && s.fire_phase == 3);
    if (static_cast<uint32_t>(origPhase) != s.fire_phase && phaseQ40) {
        d.AddQ("Q40", "phase orig %d ours %u", origPhase, s.fire_phase);
    } else if (static_cast<uint32_t>(origPhase) != s.fire_phase) {
        d.Add("phase orig %d ours %u", origPhase, s.fire_phase);
    }
    if (g.origSteps + g.resyncSteps != g.coreSteps) {
        d.Add("steps orig %llu ours %llu (%llu of them putting out a refused fire)", (unsigned long long)g.origSteps,
              (unsigned long long)g.coreSteps, (unsigned long long)g.resyncSteps);
    }
    char layer[96];
    // Putting a refused fire out runs the mirror's KillFire, which can
    // register another fire's target as the ID's location again (§5.10).
    // A spread's fire dying a step later in the mirror (Q42) does the same.
    CompareLayer(d, layer, sizeof(layer),
                 resync || g.resyncInEpisode ? "Q40" : (g.lagInEpisode ? "Q42" : nullptr));
    ++g.fire.lines;
    if (d.n == 0) ++g.fire.clean;
    if (d.n != 0 && d.unexplained == 0) ++g.fire.questioned;
    Log("disasters shadow fire %s: orig steps=%llu fires orig=%d ours=%u burning orig=%u ours=%u phase %d/%u %s "
        "lots=%u mismatches=%u%s%s",
        what, (unsigned long long)g.origSteps, origCount, s.fire_count, origBurning, oursBurning, origPhase,
        s.fire_phase, layer, lots, d.n, d.text.empty() ? "" : "; differences: ", d.text.c_str());
}

std::set<uint64_t> LiveLocked() {
    PruneGone(g.origLocked);
    std::set<uint64_t> live;
    std::set<uint64_t> all = g.origLocked;
    all.insert(g.ctx.locked.begin(), g.ctx.locked.end());
    for (uint64_t t : all) {
        cISC4Occupant* o = AsOcc(t);
        if (o != nullptr && !o->CanRemove(2)) live.insert(t);  // slot 14 (network-manager.md §10)
    }
    return live;
}

void RiotLine(Riot& r, const char* what) {
    Diffs d;
    d.scope = "riot";
    fc4_disaster_instance info{};
    const bool have = fc4_disasters_get_instance(g.core, r.id, &info);
    const int32_t origPhase = r.inst ? r.inst->GetPhase() : -1;
    // A riot stays in our manager after phase 3 (§6.7); one that has left it
    // (its Shutdown, §3.1) reads as ended.
    const uint32_t ourPhase = have ? info.phase : 3;
    if (static_cast<uint32_t>(origPhase) != ourPhase) d.Add("phase orig %d ours %u", origPhase, ourPhase);
    std::vector<fc4_riot_leader> ls(fc4_disasters_riot_leaders(g.core, r.id, nullptr, 0));
    if (!ls.empty()) fc4_disasters_riot_leaders(g.core, r.id, ls.data(), ls.size());
    std::set<uint64_t> ours;
    for (const auto& l : ls) ours.insert(l.automaton);
    if (ours != r.leaders) {
        d.AddQ("Q39", "leaders orig %u ours %u", static_cast<unsigned>(r.leaders.size()),
               static_cast<unsigned>(ours.size()));
    }
    cISC4DisasterManager* m = Manager(kRiot);
    const int32_t origActive = m ? m->GetActiveInstanceCount() : -1;  // slot 10
    const uint32_t ourActive = fc4_disasters_active_count(g.core, kRiot);
    if (origActive != static_cast<int32_t>(ourActive)) d.Add("active orig %d ours %u", origActive, ourActive);
    const std::set<uint64_t> live = LiveLocked();
    std::set<uint64_t> ourLocked;
    const size_t nl = fc4_disasters_riot_locked(g.core, nullptr, 0);
    std::vector<uint64_t> lv(nl);
    if (nl) fc4_disasters_riot_locked(g.core, lv.data(), nl);
    ourLocked.insert(lv.begin(), lv.end());
    if (live != ourLocked) {
        uint32_t onlyOrig = 0, onlyOurs = 0;
        for (uint64_t t : live) onlyOrig += ourLocked.count(t) == 0;
        for (uint64_t t : ourLocked) onlyOurs += live.count(t) == 0;
        d.AddQ("Q38", "locked orig %u ours %u (orig only %u, ours only %u)", static_cast<unsigned>(live.size()),
              static_cast<unsigned>(ourLocked.size()), onlyOrig, onlyOurs);
    }
    std::string cells;
    for (const auto& l : ls) {
        char b[48];
        std::snprintf(b, sizeof(b), "%s%d,%d c%d s%d", cells.empty() ? "" : " ", l.x, l.z, l.counter, l.stage);
        if (cells.size() < 160) cells += b;
    }
    char layer[96];
    CompareLayer(d, layer, sizeof(layer), "Q39");  // a leader a second apart
    ++g.riot.lines;
    if (d.n == 0) ++g.riot.clean;
    if (d.n != 0 && d.unexplained == 0) ++g.riot.questioned;
    Log("disasters shadow riot %u %s: phase %d/%u leaders %u/%u [%s] active %d/%u locked %u/%u %s mismatches=%u%s%s",
        r.id, what, origPhase, ourPhase, static_cast<unsigned>(r.leaders.size()), static_cast<unsigned>(ours.size()),
        cells.c_str(), origActive, ourActive, static_cast<unsigned>(live.size()),
        static_cast<unsigned>(ourLocked.size()), layer, d.n, d.text.empty() ? "" : "; differences: ", d.text.c_str());
}

// ---- forwarders --------------------------------------------------------------------------------

template <class Fn>
Fn Orig(DisasterHook h) {
    Patch& p = g.p[static_cast<unsigned>(h)];
    ++p.calls;
    return reinterpret_cast<Fn>(p.original);
}

using Fn0 = uint32_t(__fastcall*)(void*, void*);
using Fn1 = uint32_t(__fastcall*)(void*, void*, uintptr_t);
using Fn2 = uint32_t(__fastcall*)(void*, void*, uintptr_t, uintptr_t);
using Fn3 = uint32_t(__fastcall*)(void*, void*, uintptr_t, uintptr_t, uintptr_t);
using Fn4 = uint32_t(__fastcall*)(void*, void*, uintptr_t, uintptr_t, uintptr_t, uintptr_t);
using Fn5 = uint32_t(__fastcall*)(void*, void*, uintptr_t, uintptr_t, uintptr_t, uintptr_t, uintptr_t);
using Fn6 = uint32_t(__fastcall*)(void*, void*, uintptr_t, uintptr_t, uintptr_t, uintptr_t, uintptr_t, uintptr_t);

bool B(uintptr_t v) { return (v & 0xFF) != 0; }

void SamplePause() {
    cFC4ISC4Simulator* s = g.city ? AsGame(g.city->GetSimulator()) : nullptr;
    g.ctx.pausedSample = s != nullptr && s->IsPaused();
    g.ctx.emergencySample = s != nullptr && s->IsEmergencyPaused();
}

void FirstFireInternal(void* self) {
    if (g.fireInternal != nullptr) return;
    g.fireInternal = self;
    const uintptr_t c = Canon(static_cast<cIGZUnknown*>(self));
    if (g.fireCanon == 0) g.fireCanon = c;
    Log("disasters shadow: the original fire instance: internal interface %p, identity %p%s", self,
        reinterpret_cast<void*>(c), g.fireCanon == c ? "" : " (DIFFERS from the instance interface's)");
}

// Mirrors one original Ignite (after it ran).
void MirrorIgnite(uint64_t o, bool force, uint32_t id, bool focus, bool origResult, const char* why,
                  bool compare = true) {
    const std::set<uint32_t> before = id == 0 ? CoreFireIds() : std::set<uint32_t>();
    const uint32_t mid = MapId(id);
    bool ours = false;
    CoreCall("disasters shadow Ignite (ours)", true,
             [&] { ours = fc4_disasters_ignite(g.core, o, force, mid, focus); });
    if (id == 0 && ours) {
        for (uint32_t x : CoreFireIds()) {
            if (before.count(x) == 0) g.pendingIds.push_back(x);  // the mirror's own new ID (Q26)
        }
    }
    const bool refused = !origResult && ours && !DisasterOccupantBurning(AsOcc(o));
    if (refused) {
        // [question Q40] The original refused an ignition the spec admits:
        // ours is put out (the mirror's extinguish delay is 0).
        CoreCall("disasters shadow Extinguish (resync, ours)", true, [&] { fc4_disasters_extinguish(g.core, o); });
        g.resyncTargets.insert(o);
        ++g.resyncs;
        g.resyncInEpisode = true;
        // Close the mirror's phantom episode here, not at the next framework
        // tick: a call that follows inside the same tick (two script actions
        // in one catch-up batch) must see the original's state, and must run
        // its own DisasterHasBegun so §2.7 resolves its location (DS35).
        for (int i = 0; i < 32 && CoreState().fire_phase == 1; ++i) {
            const uint64_t before = g.coreSteps;
            ResyncTick();
            if (g.coreSteps == before) break;
        }
    }
    if (!compare && origResult && ours) g.spreadTargets.insert(o);
    TrackFirePhase();
    g.dirty = true;
    // §5.4 "The refusals, exactly", logged at every call (DS38): the
    // original-side inputs the answer names, plus which of the five exits
    // those inputs admit. Exit 4 needs the D9 curve and is not in the mask;
    // the coverage and the clamped rectangle are there to read it from.
    const DisasterIgniteInputs in =
        DisasterIgniteInputsOf(g.city, AsOcc(o), force, g.ctx.pausedSample, g.ctx.emergencySample);
    if (in.coverage > 50) ++g.unfollowable;
    char exits[24] = "";
    if (refused) {
        std::string e;
        for (uint32_t i = 0; i < 5; ++i) {
            if ((in.exits & (1u << i)) == 0) continue;
            if (!e.empty()) e += ",";
            e += std::to_string(i + 1);
        }
        if (e.empty()) e = "none-of-1,2,3,5";
        std::snprintf(exits, sizeof(exits), " exits=%s", e.c_str());
    }
    char what[320];
    std::snprintf(what, sizeof(what),
                  "%s %p force=%d id=%08X->%08X focus=%d type=%08X flammability=%u rect=%d,%d..%d,%d "
                  "coverage=%u burning=%d paused=%d emergency=%d -> orig %d ours %d%s%s",
                  why, reinterpret_cast<void*>(static_cast<uintptr_t>(o)), force, id, mid, focus, in.type,
                  in.flammability, in.x0, in.z0, in.x1, in.z1, in.coverage, in.burning ? 1 : 0, g.ctx.pausedSample,
                  g.ctx.emergencySample, origResult, ours,
                  refused ? " (refused by the original: Q40; ours put out)" : "", exits);
    if (origResult != ours) ++g.calls.lines;  // counted in the line below too
    if (compare) {
        FireLine(what);
    } else {
        Log("disasters shadow fire %s (compared in the step line)", what);
    }
}

uint32_t __fastcall IgniteFwd(void* self, void* edx, uintptr_t o, uintptr_t force, uintptr_t id, uintptr_t focus) {
    const Fn4 orig = Orig<Fn4>(DisasterHook::FireIgnite);
    if (!Ready()) return orig(self, edx, o, force, id, focus);
    FirstFireInternal(self);
    if (g.cellDepth > 0 || g.managerIgniteDepth > 0) ++g.nestedIgnites;
    SamplePause();
    const uint32_t r = orig(self, edx, o, force, id, focus);
    const uint64_t t = OccToken(reinterpret_cast<cIGZUnknown*>(o));
    Keep(g.targets, t);
    if (g.outerMirror == 0 && t != 0) {
        if (g.fireMsgDepth > 0) {
            g.deferred.push_back({t, B(force), B(focus), B(r), g.ctx.pausedSample, g.ctx.emergencySample,
                                  static_cast<uint32_t>(id), "Ignite (in the original's fire step)"});
        } else {
            MirrorIgnite(t, B(force), static_cast<uint32_t>(id), B(focus), B(r), "Ignite");
        }
    }
    return r;
}

uint32_t __fastcall IgniteCellFwd(void* self, void* edx, uintptr_t x, uintptr_t z, uintptr_t p, uintptr_t t,
                                  uintptr_t id) {
    const Fn5 orig = Orig<Fn5>(DisasterHook::FireIgniteCell);
    if (!Ready() || g.outerMirror > 0) return orig(self, edx, x, z, p, t, id);
    FirstFireInternal(self);
    const int32_t cx = static_cast<int32_t>(x), cz = static_cast<int32_t>(z);
    // The cell's filtered occupants and whether they burn (the fallback below).
    std::vector<std::shared_ptr<SC4List<cISC4Occupant*>>> lists;
    std::map<uint64_t, bool> was;
    for (uint32_t type : {kTypeBuilding, kTypeFlora, kTypeProp}) {
        auto l = DisasterOccupantsIn(g.city, cx, cz, cx, cz, type, nullptr);
        if (!l) continue;
        for (cISC4Occupant* o : *l) {
            if (o != nullptr) was[TokenOf(o)] = DisasterOccupantBurning(o);
        }
        lists.push_back(l);
    }
    const uint64_t nested = g.nestedIgnites;
    ++g.cellDepth;
    SamplePause();
    const uint32_t r = orig(self, edx, x, z, p, t, id);
    --g.cellDepth;
    if (g.nestedIgnites == nested) {
        // No Ignite reached its forwarder: mirror from what the call set
        // burning (an occupant per Ignite, in the occupant manager's order).
        for (const auto& [occ, burned] : was) {
            if (burned || !DisasterOccupantBurning(AsOcc(occ))) continue;
            ++g.cellFallbacks;
            Keep(g.targets, occ);
            if (g.fireMsgDepth > 0) {
                g.deferred.push_back({occ, B(t), !B(t), true, g.ctx.pausedSample, g.ctx.emergencySample,
                                      static_cast<uint32_t>(id), "IgniteCell (no nested Ignite, in the fire step)"});
            } else {
                MirrorIgnite(occ, B(t), static_cast<uint32_t>(id), !B(t), true, "IgniteCell (no nested Ignite)");
            }
        }
    }
    return r;
}

uint32_t __fastcall ExtinguishCellFwd(void* self, void* edx, uintptr_t x, uintptr_t z, uintptr_t amount) {
    const Fn3 orig = Orig<Fn3>(DisasterHook::FireExtinguishCell);
    const uint32_t r = orig(self, edx, x, z, amount);
    if (!Ready()) return r;
    ++g.extinguishCalls;
    CoreCall("disasters shadow ExtinguishCell (ours)", true, [&] {
        fc4_disasters_extinguish_cell(g.core, static_cast<int32_t>(x), static_cast<int32_t>(z),
                                      static_cast<int32_t>(amount));
    });
    TrackFirePhase();
    g.dirty = true;
    char what[96];
    std::snprintf(what, sizeof(what), "ExtinguishCell %d,%d amount %d (the extinguish delay is drawn: Q34)",
                  static_cast<int32_t>(x), static_cast<int32_t>(z), static_cast<int32_t>(amount));
    FireLine(what);
    return r;
}

uint32_t __fastcall QuickBurnCellFwd(void* self, void* edx, uintptr_t x, uintptr_t z) {
    const Fn2 orig = Orig<Fn2>(DisasterHook::FireQuickBurnCell);
    if (!Ready()) return orig(self, edx, x, z);
    ++g.outerMirror;
    const uint32_t r = orig(self, edx, x, z);
    --g.outerMirror;
    CoreCall("disasters shadow QuickBurnCell (ours)", true,
             [&] { fc4_disasters_quick_burn_cell(g.core, static_cast<int32_t>(x), static_cast<int32_t>(z)); });
    g.dirty = true;
    FireLine("QuickBurnCell");
    return r;
}

// [question Q35] The radius arguments' type (the SDK declares floats, the
// spec cells): not mirrored as a call; its Ignite calls are mirrored one by
// one, its toast list is not.
uint32_t __fastcall IgniteCellRadiusFwd(void* self, void* edx, uintptr_t x, uintptr_t z, uintptr_t r0, uintptr_t r1) {
    const Fn4 orig = Orig<Fn4>(DisasterHook::FireIgniteCellRadius);
    if (Ready()) ++g.radiusCalls;
    return orig(self, edx, x, z, r0, r1);
}

uint32_t __fastcall ExtinguishFwd(void* self, void* edx, uintptr_t o) {
    const Fn1 orig = Orig<Fn1>(DisasterHook::FireExtinguish);
    const uint32_t r = orig(self, edx, o);
    if (!Ready()) return r;
    const uint64_t t = OccToken(reinterpret_cast<cIGZUnknown*>(o));
    CoreCall("disasters shadow Extinguish (ours)", true, [&] { fc4_disasters_extinguish(g.core, t); });
    g.dirty = true;
    FireLine("Extinguish");
    return r;
}

uint32_t __fastcall BurnDownFwd(void* self, void* edx, uintptr_t o) {
    const Fn1 orig = Orig<Fn1>(DisasterHook::FireBurnDown);
    const uint32_t r = orig(self, edx, o);
    if (!Ready()) return r;
    const uint64_t t = OccToken(reinterpret_cast<cIGZUnknown*>(o));
    CoreCall("disasters shadow BurnDown (ours)", true, [&] { fc4_disasters_burn_down(g.core, t); });
    TrackFirePhase();
    g.dirty = true;
    FireLine("BurnDown");
    return r;
}

uint32_t __fastcall SetCauseFwd(void* self, void* edx, uintptr_t c) {
    const Fn1 orig = Orig<Fn1>(DisasterHook::FireSetCause);
    const uint32_t r = orig(self, edx, c);
    if (!Ready()) return r;
    // SetCause sets the cause only while it is 0 and returns it (§5.7).
    uint32_t ours = 0;
    CoreCall("disasters shadow SetCause (ours)", true,
             [&] { ours = fc4_disasters_set_cause(g.core, static_cast<uint32_t>(c)); });
    if (ours != r) {
        ++g.failures;
        Log("disasters shadow SetCause(%u): orig returned %u ours %u", static_cast<unsigned>(c), r, ours);
    }
    return r;
}

uint32_t __fastcall ManagerIgniteOccFwd(void* self, void* edx, uintptr_t o) {
    const Fn1 orig = Orig<Fn1>(DisasterHook::FireManagerIgniteOcc);
    if (!Ready() || g.outerMirror > 0) return orig(self, edx, o);
    const uint64_t nested = g.nestedIgnites;
    ++g.managerIgniteDepth;
    SamplePause();
    const uint32_t r = orig(self, edx, o);
    --g.managerIgniteDepth;
    if (g.nestedIgnites == nested && o != 0) {
        // §5.4: IgniteOccupant(o) = Ignite(o, false, 0, true), not seen at the
        // instance's slot 6: mirror it here.
        const uint64_t t = OccToken(reinterpret_cast<cIGZUnknown*>(o));
        Keep(g.targets, t);
        MirrorIgnite(t, false, 0, true, DisasterOccupantBurning(AsOcc(t)), "IgniteOccupant (no nested Ignite)");
    }
    return r;
}

uint32_t MessageUs(cIGZMessage2* msg) {
    void* p = nullptr;
    if (!msg->QueryInterface(GZIID_cIGZMessage2Standard, &p) || p == nullptr) return 0;
    cIGZMessage2Standard* s = static_cast<cIGZMessage2Standard*>(p);
    FC4_ACQUIRED(s, "QueryInterface");
    const uint32_t us = static_cast<uint32_t>(s->GetData2());
    FC4_RELEASE(s);
    return us;
}

uint32_t __fastcall FireMessageFwd(void* self, void* edx, uintptr_t m) {
    const Fn1 orig = Orig<Fn1>(DisasterHook::FireMessage);
    cIGZMessage2* msg = reinterpret_cast<cIGZMessage2*>(m);
    if (!Ready() || msg == nullptr || msg->GetType() != kMsgAnimationTick) return orig(self, edx, m);
    const uint32_t us = MessageUs(msg);
    const int32_t origPhaseBefore = g.fireInst ? g.fireInst->GetPhase() : 0;
    const uint32_t corePhaseBefore = g.corePhase;
    ++g.fireMsgDepth;
    g.tutCalls = 0;
    const uint32_t r = orig(self, edx, m);
    --g.fireMsgDepth;
    const uint32_t steps = g.tutCalls;
    g.origSteps += steps;
    // The fire clock reaches the mirror's next step exactly when the original stepped.
    if (steps > 0 && g.fireNextValid && g.fireClock < g.fireNext) g.fireClock = g.fireNext;
    const uint64_t coreBefore = g.coreSteps;
    const uint32_t fireId = CoreState().fire_instance;
    CoreCall("disasters shadow fire tick (ours)", true, [&] { fc4_disasters_tick(g.core, fireId, us); });
    const bool stepped = g.coreSteps != coreBefore;
    if (stepped && g.fireNextValid) g.fireNext += 1000;
    TrackFirePhase();
    const int32_t origPhaseAfter = g.fireInst ? g.fireInst->GetPhase() : 0;
    std::vector<Shadow::Deferred> later;
    later.swap(g.deferred);
    for (const auto& dfr : later) {
        ++g.deferredIgnites;
        g.ctx.pausedSample = dfr.paused;
        g.ctx.emergencySample = dfr.emergency;
        MirrorIgnite(dfr.o, dfr.force, dfr.id, dfr.focus, dfr.result, dfr.why, false);
    }
    if (steps > 0 || stepped) {
        g.dirty = true;
        char what[64];
        std::snprintf(what, sizeof(what), "step %llu", (unsigned long long)++g.fireStepLines);
        FireLine(what);
    } else if (origPhaseAfter != origPhaseBefore || g.corePhase != corePhaseBefore) {
        g.dirty = true;
        char what[64];
        std::snprintf(what, sizeof(what), "phase orig %d->%d ours %u->%u", origPhaseBefore, origPhaseAfter,
                      corePhaseBefore, g.corePhase);
        FireLine(what);
    }
    return r;
}

uint32_t __fastcall TutorialFwd(void* self, void* edx) {
    const Fn0 orig = Orig<Fn0>(DisasterHook::TutorialIsTutorial);
    const uint32_t r = orig(self, edx);
    if (g.fireMsgDepth > 0 && !g.inCore) ++g.tutCalls;
    return r;
}

uint32_t __fastcall LayerBegunFwd(void* self, void* edx, uintptr_t inst) {
    const Fn1 orig = Orig<Fn1>(DisasterHook::LayerBegun);
    const uint32_t r = orig(self, edx, inst);
    if (!Ready()) return r;
    ++g.origBegun;
    g.dirty = true;
    cISC4DisasterInstance* i = reinterpret_cast<cISC4DisasterInstance*>(inst);
    if (i != nullptr && g.fireInst == nullptr && i->GetID() == kFire) {  // slot 5
        g.fireInst = i;
        FC4_ADDREF(i);
        const uintptr_t c = Canon(i);
        if (g.fireCanon != 0 && g.fireCanon != c) {
            Log("disasters shadow: the fire instance interface's identity %p DIFFERS from %p",
                reinterpret_cast<void*>(c), reinterpret_cast<void*>(g.fireCanon));
        }
        g.fireCanon = c;
    }
    return r;
}

uint32_t __fastcall LayerEndedFwd(void* self, void* edx) {
    const Fn0 orig = Orig<Fn0>(DisasterHook::LayerEnded);
    const uint32_t r = orig(self, edx);
    if (Ready()) ++g.origEnded;
    return r;
}

uint32_t __fastcall LayerRegisterFwd(void* self, void* edx, uintptr_t inst, uintptr_t x, uintptr_t z, uintptr_t occ,
                                     uintptr_t zoom, uintptr_t focus) {
    const Fn6 orig = Orig<Fn6>(DisasterHook::LayerRegister);
    const uint32_t r = orig(self, edx, inst, x, z, occ, zoom, focus);
    if (!Ready()) return r;
    cISC4DisasterInstance* i = reinterpret_cast<cISC4DisasterInstance*>(inst);
    if (i != nullptr && !NotAnObject(i)) {
        const uint32_t type = i->GetID();
        Log("disasters shadow: %s RegisterDisasterLocation(instance %p, %d, %d, occupant %p, zoom %u, focus %u)",
            type == kRiot ? "riot" : type == kFire ? "fire" : "other", static_cast<void*>(i),
            static_cast<int32_t>(x), static_cast<int32_t>(z), reinterpret_cast<void*>(occ),
            static_cast<uint32_t>(zoom), static_cast<uint32_t>(focus & 0xFF));
    }
    if (i != nullptr && !NotAnObject(i) && g.riotStartDepth > 0 && g.riotStartInst == nullptr &&
        i->GetID() == kRiot) {
        g.riotStartInst = i;
    }
    if (i != nullptr && occ != 0 && !NotAnObject(i) && !NotAnObject(reinterpret_cast<void*>(occ)) &&
        i->GetID() == kRiot) {
        const uintptr_t c = Canon(i);
        const uint64_t t = OccToken(reinterpret_cast<cIGZUnknown*>(occ));
        g.pendingLeaders[c].push_back(t);
        if (g.riotMsgDepth > 0) ++g.riotRegisters;
        auto it = g.riots.find(c);
        if (it != g.riots.end()) it->second.leaders.insert(t);
    }
    return r;
}

void ForgetLeaders(uintptr_t inst, uint64_t occ, bool all) {
    cISC4DisasterInstance* i = reinterpret_cast<cISC4DisasterInstance*>(inst);
    if (i == nullptr) return;
    auto it = g.riots.find(Canon(i));
    if (it == g.riots.end()) return;
    if (all) {
        it->second.leaders.clear();
    } else {
        it->second.leaders.erase(occ);
    }
}

uint32_t __fastcall LayerUnregisterAllFwd(void* self, void* edx, uintptr_t inst) {
    const Fn1 orig = Orig<Fn1>(DisasterHook::LayerUnregisterAll);
    const uint32_t r = orig(self, edx, inst);
    if (Ready()) ForgetLeaders(inst, 0, true);
    return r;
}

uint32_t __fastcall LayerUnregisterByOccFwd(void* self, void* edx, uintptr_t inst, uintptr_t occ) {
    const Fn2 orig = Orig<Fn2>(DisasterHook::LayerUnregisterByOcc);
    const uint32_t r = orig(self, edx, inst, occ);
    if (g.riotMsgDepth > 0) ++g.riotUnregisters;
    if (Ready()) ForgetLeaders(inst, OccToken(reinterpret_cast<cIGZUnknown*>(occ)), false);
    return r;
}

// The leaders the original registered for `canon`, handed to the mirror's
// create_riot_leader for one core call; the rest go back.
void HandLeaders(uintptr_t canon) {
    g.ctx.leaders.swap(g.pendingLeaders[canon]);
}
// Leaders the mirror did not take wait for its next call (the original can
// spawn a second before the mirror does).
void TakeBackLeaders(uintptr_t canon) {
    std::deque<uint64_t>& p = g.pendingLeaders[canon];
    p.insert(p.begin(), g.ctx.leaders.begin(), g.ctx.leaders.end());
    g.ctx.leaders.clear();
}

uint32_t __fastcall RiotStartFwd(void* self, void* edx, uintptr_t x, uintptr_t z) {
    const Fn2 orig = Orig<Fn2>(DisasterHook::RiotStart);
    if (!Ready()) return orig(self, edx, x, z);
    ++g.riotStartDepth;
    g.riotStartInst = nullptr;
    const uint32_t r = orig(self, edx, x, z);
    --g.riotStartDepth;
    // AttemptDisasterStart returns null (§6.2): the original's instance is the
    // one its first leader registered.
    cISC4DisasterInstance* inst = r != 0 ? reinterpret_cast<cISC4DisasterInstance*>(static_cast<uintptr_t>(r))
                                         : g.riotStartInst;
    if (r != 0) Log("disasters shadow riot start: the original returned %p, not null (§6.2)", static_cast<void*>(inst));
    const uintptr_t canon = inst ? Canon(inst) : 0;
    uint32_t id = 0;
    const uint64_t missingBefore = g.ctx.leadersMissing;
    const std::deque<uint64_t> registered = g.pendingLeaders[canon];
    HandLeaders(canon);
    // Ours returns null as well; a new riot is the new last entry of the
    // manager's list.
    CoreCall("disasters shadow riot start (ours)", false, [&] {
        const size_t before = fc4_disasters_manager_instances(g.core, kRiot, nullptr, 0);
        const uint32_t r0 =
            fc4_disasters_attempt_start(g.core, kRiot, static_cast<int32_t>(x), static_cast<int32_t>(z));
        const size_t n = fc4_disasters_manager_instances(g.core, kRiot, nullptr, 0);
        if (r0 != 0) {
            id = r0;
        } else if (n > before) {
            std::vector<uint32_t> ids(n);
            fc4_disasters_manager_instances(g.core, kRiot, ids.data(), ids.size());
            id = ids.back();
        }
    });
    TakeBackLeaders(canon);
    g.dirty = true;
    Log("disasters shadow riot start %d,%d: orig instance %p ours %u%s", static_cast<int32_t>(x),
        static_cast<int32_t>(z), static_cast<void*>(inst), id,
        g.ctx.leadersMissing != missingBefore ? " (ours made a leader the original did not register)" : "");
    if (inst != nullptr && id != 0) {
        Riot& rt = g.riots[canon];
        rt.id = id;
        rt.inst = inst;
        FC4_ADDREF(inst);
        rt.leaders.insert(registered.begin(), registered.end());
        LARGE_INTEGER fr{}, now{};
        QueryPerformanceFrequency(&fr);
        QueryPerformanceCounter(&now);
        rt.startMs = static_cast<uint64_t>(now.QuadPart / (fr.QuadPart / 1000));
        RiotLine(rt, "start");
    } else if ((inst != nullptr) != (id != 0)) {
        ++g.riot.lines;
        Log("disasters shadow riot start: mismatches=1; differences: started orig %d ours %d", inst != nullptr,
            id != 0);
    }
    return r;
}

uint32_t __fastcall RiotRemoveFwd(void* self, void* edx, uintptr_t inst) {
    const Fn1 orig = Orig<Fn1>(DisasterHook::RiotRemoveInstance);
    const uint32_t r = orig(self, edx, inst);
    if (!Ready() || inst == 0) return r;
    auto it = g.riots.find(Canon(reinterpret_cast<cIGZUnknown*>(inst)));
    if (it != g.riots.end()) {
        it->second.removed = true;
        RiotLine(it->second, "removed from the original's manager");
    }
    return r;
}

uint32_t __fastcall RiotMessageFwd(void* self, void* edx, uintptr_t m) {
    const Fn1 orig = Orig<Fn1>(DisasterHook::RiotMessage);
    cIGZMessage2* msg = reinterpret_cast<cIGZMessage2*>(m);
    if (!Ready() || msg == nullptr || msg->GetType() != kMsgAnimationTick) return orig(self, edx, m);
    const uintptr_t canon = Canon(static_cast<cIGZUnknown*>(self));
    const uint32_t us = MessageUs(msg);
    g.riotLocks = g.riotRegisters = g.riotUnregisters = 0;
    ++g.riotMsgDepth;
    const uint32_t r = orig(self, edx, m);
    --g.riotMsgDepth;
    auto it = g.riots.find(canon);
    if (it == g.riots.end()) return r;
    Riot& rt = it->second;
    if ((g.riotLocks | g.riotRegisters | g.riotUnregisters) != 0 && g.riotTickLines < 400) {
        ++g.riotTickLines;
        LARGE_INTEGER fr{}, now{};
        QueryPerformanceFrequency(&fr);
        QueryPerformanceCounter(&now);
        const uint64_t ms = static_cast<uint64_t>(now.QuadPart / (fr.QuadPart / 1000));
        Log("disasters shadow riot %u original tick at +%llu ms: lock calls %u, leader registrations %u, "
            "unregistrations %u",
            rt.id, (unsigned long long)(ms - rt.startMs), g.riotLocks, g.riotRegisters, g.riotUnregisters);
    }
    HandLeaders(canon);
    CoreCall("disasters shadow riot tick (ours)", false, [&] { fc4_disasters_tick(g.core, rt.id, us); });
    TakeBackLeaders(canon);
    g.dirty = true;
    const uint64_t second = (g.ctx.clockMs - rt.startMs) / 1000;
    if (second != rt.lastSecond) {
        rt.lastSecond = second;
        char what[48];
        std::snprintf(what, sizeof(what), "update %llu", (unsigned long long)++rt.updates);
        RiotLine(rt, what);
    }
    return r;
}

uint32_t __fastcall RiotLockFwd(void* self, void* edx, uintptr_t n) {
    const Fn1 orig = Orig<Fn1>(DisasterHook::RiotLock);
    const uint32_t r = orig(self, edx, n);
    if (!Ready() || n == 0 || NotAnObject(reinterpret_cast<void*>(n))) return r;
    if (g.riotMsgDepth > 0) ++g.riotLocks;
    cISC4NetworkOccupant* net = reinterpret_cast<cISC4NetworkOccupant*>(n);
    Keep(g.origLocked, TokenOf(net->AsOccupant()));  // network occupant slot 48
    return r;
}

void* ForwarderOf(DisasterHook h) {
    switch (h) {
        case DisasterHook::LayerBegun: return reinterpret_cast<void*>(&LayerBegunFwd);
        case DisasterHook::LayerEnded: return reinterpret_cast<void*>(&LayerEndedFwd);
        case DisasterHook::LayerRegister: return reinterpret_cast<void*>(&LayerRegisterFwd);
        case DisasterHook::LayerUnregisterAll: return reinterpret_cast<void*>(&LayerUnregisterAllFwd);
        case DisasterHook::LayerUnregisterByOcc: return reinterpret_cast<void*>(&LayerUnregisterByOccFwd);
        case DisasterHook::FireIgnite: return reinterpret_cast<void*>(&IgniteFwd);
        case DisasterHook::FireIgniteCell: return reinterpret_cast<void*>(&IgniteCellFwd);
        case DisasterHook::FireExtinguishCell: return reinterpret_cast<void*>(&ExtinguishCellFwd);
        case DisasterHook::FireQuickBurnCell: return reinterpret_cast<void*>(&QuickBurnCellFwd);
        case DisasterHook::FireIgniteCellRadius: return reinterpret_cast<void*>(&IgniteCellRadiusFwd);
        case DisasterHook::FireExtinguish: return reinterpret_cast<void*>(&ExtinguishFwd);
        case DisasterHook::FireBurnDown: return reinterpret_cast<void*>(&BurnDownFwd);
        case DisasterHook::FireSetCause: return reinterpret_cast<void*>(&SetCauseFwd);
        case DisasterHook::FireMessage: return reinterpret_cast<void*>(&FireMessageFwd);
        case DisasterHook::FireManagerIgniteOcc: return reinterpret_cast<void*>(&ManagerIgniteOccFwd);
        case DisasterHook::RiotStart: return reinterpret_cast<void*>(&RiotStartFwd);
        case DisasterHook::RiotRemoveInstance: return reinterpret_cast<void*>(&RiotRemoveFwd);
        case DisasterHook::RiotMessage: return reinterpret_cast<void*>(&RiotMessageFwd);
        case DisasterHook::RiotLock: return reinterpret_cast<void*>(&RiotLockFwd);
        case DisasterHook::TutorialIsTutorial: return reinterpret_cast<void*>(&TutorialFwd);
        case DisasterHook::Count: break;
    }
    return nullptr;
}

// ---- patching ----------------------------------------------------------------------------------

bool PatchEntry(DisasterHook h) {
    Patch& p = g.p[static_cast<unsigned>(h)];
    void** entry = nullptr;
    uintptr_t expected = 0;
    if (!compat::DisasterHookEntry(h, &entry, &expected)) return false;
    const uintptr_t found = reinterpret_cast<uintptr_t>(*entry);
    if (found != expected) {
        Log("disasters shadow: %s forwarder NOT installed: the entry %p holds 0x%08lX, expected 0x%08lX", HookName(h),
            static_cast<void*>(entry), static_cast<unsigned long>(found), static_cast<unsigned long>(expected));
        return false;
    }
    DWORD before = 0;
    if (!VirtualProtect(entry, sizeof(void*), PAGE_READWRITE, &before)) {
        Log("disasters shadow: %s forwarder NOT installed: VirtualProtect failed (error %lu)", HookName(h),
            GetLastError());
        return false;
    }
    p.original = found;
    *entry = ForwarderOf(h);
    DWORD ignored = 0;
    VirtualProtect(entry, sizeof(void*), before, &ignored);
    FlushInstructionCache(GetCurrentProcess(), entry, sizeof(void*));
    p.entry = entry;
    p.installed = true;
    return true;
}

void UnpatchAll(const char* why) {
    std::string done;
    for (unsigned i = 0; i < static_cast<unsigned>(DisasterHook::Count); ++i) {
        Patch& p = g.p[i];
        if (!p.installed) continue;
        DWORD before = 0;
        if (!VirtualProtect(p.entry, sizeof(void*), PAGE_READWRITE, &before)) {
            Log("disasters shadow: WARNING %s forwarder NOT removed (%s): VirtualProtect failed (error %lu)",
                HookName(static_cast<DisasterHook>(i)), why, GetLastError());
            continue;
        }
        *p.entry = reinterpret_cast<void*>(p.original);
        DWORD ignored = 0;
        VirtualProtect(p.entry, sizeof(void*), before, &ignored);
        FlushInstructionCache(GetCurrentProcess(), p.entry, sizeof(void*));
        p.installed = false;
        char b[96];
        std::snprintf(b, sizeof(b), "%s%s=%llu", done.empty() ? "" : ", ", HookName(static_cast<DisasterHook>(i)),
                      (unsigned long long)p.calls);
        done += b;
    }
    if (!done.empty()) Log("disasters shadow: forwarders removed (%s); calls: %s", why, done.c_str());
}

void InstallAll() {
    if (g.tried) return;
    g.tried = true;
    g.mainTid = GetCurrentThreadId();
    std::string ok, missing;
    for (unsigned i = 0; i < static_cast<unsigned>(DisasterHook::Count); ++i) {
        const DisasterHook h = static_cast<DisasterHook>(i);
        std::string& s = PatchEntry(h) ? ok : missing;
        if (!s.empty()) s += ", ";
        s += HookName(h);
    }
    g.installedAny = !ok.empty();
    Log("disasters shadow: forwarders installed: %s; NOT installed: %s", ok.empty() ? "none" : ok.c_str(),
        missing.empty() ? "none" : missing.c_str());
}

// ---- messages ----------------------------------------------------------------------------------

void OnMessage(cIGZMessage2* msg) {
    if (msg == nullptr || g.core == nullptr || GetCurrentThreadId() != g.mainTid) return;
    const uint32_t type = msg->GetType();
    void* p = nullptr;
    if (!msg->QueryInterface(GZIID_cIGZMessage2Standard, &p) || p == nullptr) return;
    cIGZMessage2Standard* s = static_cast<cIGZMessage2Standard*>(p);
    FC4_ACQUIRED(s, "QueryInterface");
    for (uint32_t t : kCountedPosts) {
        if (t != type) continue;
        ++g.origPosts[type];
        if (type == 0x49D15B73) {
            g.origD1[type] += static_cast<uint32_t>(s->GetData1());
            g.origD2[type] += static_cast<uint32_t>(s->GetData2());
        } else if (type == 0x49E3DB1D) {
            g.origD2[type] += static_cast<uint32_t>(s->GetData2());
        }
    }
    std::vector<std::pair<uint32_t, uint32_t>> to;
    for (const auto& [kind, id, mt] : g.ctx.subscriptions) {
        if (mt == type) to.emplace_back(kind, id);
    }
    if (!to.empty()) {
        fc4_disaster_message m{};
        m.msg_type = type;
        m.data1 = static_cast<int64_t>(s->GetData1());
        m.data2 = static_cast<int64_t>(s->GetData2());
        m.data3 = static_cast<int64_t>(s->GetData3());
        // void1 is an occupant only for the occupant-removed message (§2.5);
        // the dispatch messages' is not an object.
        m.void1 = type == 0x99EF1143 ? OccToken(static_cast<cIGZUnknown*>(s->GetVoid1())) : TokenOf(s->GetVoid1());
        for (const auto& [kind, id] : to) {
            CoreCall("disasters shadow message (ours)", false, [&] { fc4_disasters_do_message(g.core, kind, id, &m); });
        }
    }
    FC4_RELEASE(s);
}

class EventTarget final : public cIGZMessageTarget2 {
public:
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid != 0x452294AA && riid != GZIID_cIGZUnknown) return false;  // cIGZMessageTarget2
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { return refs_ > 0 ? --refs_ : 0u; }
    bool DoMessage(cIGZMessage2* msg) override {
        if (msg == nullptr) return true;
        FC4_X87_SCOPE("disasters shadow DoMessage", msg->GetType());
        OnMessage(msg);
        return true;
    }

private:
    uint32_t refs_ = 0;
};

EventTarget& Events() {
    static EventTarget t;
    return t;
}

void Listen() {
    if (g.listening) return;
    g.listening = true;
    cIGZMessageServer2Ptr ms;
    if (!ms) {
        Log("disasters shadow: no message server; the original's posts are not counted");
        return;
    }
    Events().AddRef();
    for (uint32_t t : kCountedPosts) ms->AddNotification(&Events(), t);
    for (uint32_t t : kRoutedTypes) ms->AddNotification(&Events(), t);
}

// ---- the quiet point ---------------------------------------------------------------------------

void QuietLine() {
    Diffs d;
    d.scope = "quiet";
    std::string text;
    std::set<uint32_t> types;
    for (const auto& [t, n] : g.origPosts) types.insert(t);
    for (const auto& [t, n] : g.ctx.posts) types.insert(t);
    for (uint32_t t : types) {
        const uint64_t o = g.origPosts.count(t) ? g.origPosts[t] : 0;
        const uint64_t c = g.ctx.posts.count(t) ? g.ctx.posts[t] : 0;
        char b[64];
        std::snprintf(b, sizeof(b), "%s%08X %llu/%llu", text.empty() ? "" : " ", t, (unsigned long long)o,
                      (unsigned long long)c);
        text += b;
        if (o != c && g.resyncs != 0) {  // the posts count since the city loaded
            d.AddQ("Q40", "posts %08X orig %llu ours %llu", t, (unsigned long long)o, (unsigned long long)c);
        } else if (o != c) {
            d.Add("posts %08X orig %llu ours %llu", t, (unsigned long long)o, (unsigned long long)c);
        }
    }
    const char* sumsQ = g.resyncs != 0 ? "Q40" : nullptr;  // the sums count since the city loaded
    for (uint32_t t : {0x49D15B73u, 0x49E3DB1Du}) {
        if (g.origD2[t] != g.ctx.postData2Sums[t]) {
            d.AddQ(sumsQ, "%08X data2 sum orig %llu ours %llu", t, (unsigned long long)g.origD2[t],
                   (unsigned long long)g.ctx.postData2Sums[t]);
        }
    }
    if (g.origD1[0x49D15B73] != g.ctx.postData1Sums[0x49D15B73]) {
        d.AddQ(sumsQ, "49D15B73 data1 sum orig %llu ours %llu", (unsigned long long)g.origD1[0x49D15B73],
               (unsigned long long)g.ctx.postData1Sums[0x49D15B73]);
    }
    if (g.ctx.pauses != 0 || g.ctx.saveOff != 0) d.Add("ours left pause %d save-off %d", g.ctx.pauses, g.ctx.saveOff);
    cISC4DisasterLayer* l = Layer();
    // Slot 18 (pop 0) returns a **pointer to the statistics record**, not a
    // cost (§2.6, DS39): in the original the interface pointer + 0x48. The
    // four fields the safety advisor reads (§2.6) are compared here.
    using CostFn = void*(__fastcall*)(void*, void*);
    const uint8_t* rec =
        l ? static_cast<const uint8_t*>(reinterpret_cast<CostFn>((*reinterpret_cast<void***>(l))[18])(l, nullptr))
          : nullptr;
    const fc4_disasters_state st = CoreState();
    uint32_t origBuildings = 0;
    int64_t origTotal = 0;
    float origFire = 0.0f, origPolice = 0.0f;
    if (rec != nullptr) {
        std::memcpy(&origBuildings, rec + 0x00, sizeof(origBuildings));
        std::memcpy(&origTotal, rec + 0x08, sizeof(origTotal));
        std::memcpy(&origFire, rec + 0x5C, sizeof(origFire));
        std::memcpy(&origPolice, rec + 0x60, sizeof(origPolice));
    }
    if (rec == nullptr) {
        d.Add("cost record: the original returned null");
    } else {
        if (origBuildings != st.damaged_buildings) {
            d.Add("cost record buildings orig %u ours %u", origBuildings, st.damaged_buildings);
        }
        if (origTotal != st.disaster_cost) {
            d.Add("cost record total orig %lld ours %lld", (long long)origTotal, (long long)st.disaster_cost);
        }
        if (origFire != st.fire_effectiveness) {
            d.Add("cost record fire effectiveness orig %g ours %g", static_cast<double>(origFire),
                  static_cast<double>(st.fire_effectiveness));
        }
        if (origPolice != st.police_effectiveness) {
            d.Add("cost record police effectiveness orig %g ours %g", static_cast<double>(origPolice),
                  static_cast<double>(st.police_effectiveness));
        }
    }
    char layer[96];
    CompareLayer(d, layer, sizeof(layer), nullptr);
    ++g.quietLines.lines;
    if (d.n == 0) ++g.quietLines.clean;
    if (d.n != 0 && d.unexplained == 0) ++g.quietLines.questioned;
    Log("disasters shadow quiet: posts orig/ours %s; cost record buildings %u/%u total %lld/%lld fire %g/%g "
        "police %g/%g; %s deferred=%llu refused-by-the-original=%llu unfollowable=%llu id-guesses=%llu "
        "leaders-missing=%llu cell-fallbacks=%llu radius=%llu extinguish=%llu mismatches=%u%s%s",
        text.c_str(), origBuildings, st.damaged_buildings, (long long)origTotal, (long long)st.disaster_cost,
        static_cast<double>(origFire), static_cast<double>(st.fire_effectiveness), static_cast<double>(origPolice),
        static_cast<double>(st.police_effectiveness), layer, (unsigned long long)g.deferredIgnites,
        (unsigned long long)g.resyncs, (unsigned long long)g.unfollowable,
        (unsigned long long)g.idGuesses, (unsigned long long)g.ctx.leadersMissing, (unsigned long long)g.cellFallbacks,
        (unsigned long long)g.radiusCalls, (unsigned long long)g.extinguishCalls, d.n,
        d.text.empty() ? "" : "; differences: ",
        d.text.c_str());
}

// ---- city session ------------------------------------------------------------------------------

void ReleaseRiots() {
    for (auto& [c, r] : g.riots) {
        if (r.inst != nullptr) FC4_RELEASE(r.inst);
    }
    g.riots.clear();
    g.pendingLeaders.clear();
}

void EndCity() {
    if (g.core != nullptr) {
        g.inCore = true;
        fc4_disasters_destroy(g.core);
        g.inCore = false;
        g.core = nullptr;
    }
    DisasterHostReleaseAll(&g.ctx);
    ReleaseRiots();
    DropAll(g.targets);
    DropAll(g.origLocked);
    if (g.fireInst != nullptr) FC4_RELEASE(g.fireInst);
    g.fireInst = nullptr;
    g.fireInternal = nullptr;
    g.fireCanon = 0;
    g.city = nullptr;
}

// ---- the script's action -----------------------------------------------------------------------

struct Candidate {
    int32_t x = 0, z = 0;
    uint32_t coverage = 0;
    float flammability = 0;
    cISC4Occupant* o = nullptr;
};

std::vector<Candidate> FireCandidates(cISC4City* city, size_t max) {
    std::vector<Candidate> out;
    const int32_t cx = static_cast<int32_t>(city->CellCountX()), cz = static_cast<int32_t>(city->CellCountZ());
    auto list = DisasterOccupantsIn(city, 0, 0, cx - 1, cz - 1, kTypeBuilding, nullptr);
    if (!list) return out;
    for (cISC4Occupant* o : *list) {
        if (o == nullptr || !o->IsFlagSet(1) || DisasterOccupantBurning(o)) continue;
        std::vector<float> f;
        cISCPropertyHolder* h = o->AsPropertyHolder();
        if (h == nullptr || !props::GetF32s(h, kPropFlammability, f) || f.empty() || f[0] == 0.0f) continue;
        const uint8_t cover = DisasterFireCoverage(city, o);
        if (cover != 0) continue;  // a fire the mirror can follow (FC4DisasterShadow.h)
        SC4Rect<long> r;
        o->GetBoundingCityCells(r);
        // Away from the edge, on a lot (the edge's connection pieces are not
        // what a test fire wants).
        if (r.topLeftX < 4 || r.topLeftY < 4 || r.bottomRightX >= cx - 4 || r.bottomRightY >= cz - 4) continue;
        cISC4LotManager* lm = city->GetLotManager();
        if (lm == nullptr || AsGame(lm)->GetOccupantLot(o) == nullptr) continue;
        Candidate c;
        c.x = static_cast<int32_t>(r.topLeftX);
        c.z = static_cast<int32_t>(r.topLeftY);
        c.coverage = cover;
        c.flammability = f[0];
        out.push_back(c);
    }
    std::sort(out.begin(), out.end(), [](const Candidate& a, const Candidate& b) {
        return a.z != b.z ? a.z < b.z : a.x < b.x;
    });
    if (out.size() > max) out.resize(max);
    return out;
}

std::vector<Candidate> RiotCandidates(cISC4City* city, size_t max) {
    std::vector<Candidate> out;
    const int32_t cx = static_cast<int32_t>(city->CellCountX()), cz = static_cast<int32_t>(city->CellCountZ());
    for (int32_t z = 8; z < cz - 8 && out.size() < max; ++z) {
        for (int32_t x = 8; x < cx - 8 && out.size() < max; ++x) {
            if (!DisasterIsRoadCell(city, x, z)) continue;
            const uint8_t p = DisasterPoliceCoverage(city, x, z);
            if (p > 64) continue;  // §6.4 step 2 and §6.5: a leader that rises unopposed
            Candidate c;
            c.x = x;
            c.z = z;
            c.coverage = p;
            out.push_back(c);
            x += 16;  // spread the candidates out
        }
    }
    return out;
}

}  // namespace

// ---- public ------------------------------------------------------------------------------------

void DisastersShadowStartup(bool layerReplaced) {
    g.enabled = SettingInt("disasters_shadow", "FC4_DISASTERS_SHADOW", 0) != 0;
    g.layerReplaced = layerReplaced;
    if (!g.enabled) return;
    const char* src = "";
    Setting("disasters_shadow", "FC4_DISASTERS_SHADOW", &src);
    if (layerReplaced) {
        Log("disasters shadow: requested (disasters_shadow from %s) but INACTIVE: it compares against the ORIGINAL "
            "disaster layer, and disasters is enabled (ours)",
            src);
        g.enabled = false;
        return;
    }
    Log("disasters shadow: ON (disasters_shadow from %s): forwarders on the original layer, fire and riot (installed "
        "at the first city); a mirror core follows every ignition, fire step, riot start and riot update and is "
        "compared after each (FC4DisasterShadow.h)",
        src);
}

void DisastersShadowCityLoaded(cISC4City* city) {
    if (!g.enabled || city == nullptr) return;
    InstallAll();
    if (!g.installedAny) return;
    EndCity();
    g.city = city;
    g.ctx = DisasterHostContext{};
    g.ctx.city = city;
    g.ctx.shadow = true;
    g.host = MakeDisasterHost(&g.ctx);
    g.host.tutorial_no_burn_down = &CoreTutorial;
    LARGE_INTEGER fr{}, now{};
    QueryPerformanceFrequency(&fr);
    QueryPerformanceCounter(&now);
    g.fireClock = static_cast<uint64_t>(now.QuadPart / (fr.QuadPart / 1000));
    g.fireNextValid = false;
    g.corePhase = 0;
    g.origSteps = g.coreSteps = g.fireStepLines = 0;
    g.idMap.clear();
    g.pendingIds.clear();
    g.deferred.clear();
    g.deferredIgnites = 0;
    g.resyncTargets.clear();
    g.resyncs = g.resyncSteps = 0;
    g.resyncInEpisode = false;
    g.spreadTargets.clear();
    g.lagInEpisode = false;
    g.origBegun = g.origEnded = 0;
    g.origPosts.clear();
    g.origD1.clear();
    g.origD2.clear();
    g.fire = g.riot = g.quietLines = g.calls = Counts{};
    g.differences = g.failures = 0;
    g.reportMonth = 0;
    g.scriptStage = 0;
    g.restoreTurbo = false;
    g.scripted = SettingInt("disasters_scripted", "FC4_DISASTERS_SCRIPTED", 0) != 0;
    g.dirty = false;
    g.quiet = 0;
    g.core = fc4_disasters_create(&g.host, 0x00D15A57u);
    if (g.core == nullptr) {
        Log("disasters shadow: the mirror core could not be made; nothing compared");
        return;
    }
    const int tuning = DisasterHostLoadTuning(g.core);
    fc4_disasters_set_mirror_random(g.core);
    bool ok = false;
    uint32_t fire = 0;
    CoreCall("disasters shadow init (ours)", true, [&] {
        ok = fc4_disasters_init(g.core);
        fire = fc4_disasters_fire_instance(g.core);  // made now, on the fire clock
    });
    Listen();
    cISC4DisasterLayer* l = Layer();
    Log("disasters shadow: city loaded: mirror core init=%d fire instance %u, %d tuning exemplar(s); the original "
        "layer %p (ongoing=%d, %d location(s))",
        ok, fire, tuning, static_cast<void*>(l), l ? l->IsADisasterOngoing() : 0,
        l ? l->GetDisasterLocationCount() : 0);
}

void DisastersShadowCityShutdown() {
    if (g.core == nullptr) return;
    const uint64_t lines = g.fire.lines + g.riot.lines + g.quietLines.lines;
    const uint64_t clean = g.fire.clean + g.riot.clean + g.quietLines.clean;
    std::string kinds;
    for (const auto& [k, n] : DiffKinds()) {
        if (!kinds.empty()) kinds += ", ";
        kinds += k + " " + std::to_string(n);
    }
    Log("disasters shadow: city shutdown: %llu lines, %llu with mismatches=0 (fire %llu/%llu, riot %llu/%llu, quiet "
        "%llu/%llu); lines whose every difference is a numbered question: fire %llu, riot %llu, quiet %llu; original "
        "steps %llu, ours %llu; differences by kind: %s",
        (unsigned long long)lines, (unsigned long long)clean, (unsigned long long)g.fire.clean,
        (unsigned long long)g.fire.lines, (unsigned long long)g.riot.clean, (unsigned long long)g.riot.lines,
        (unsigned long long)g.quietLines.clean, (unsigned long long)g.quietLines.lines,
        (unsigned long long)g.fire.questioned, (unsigned long long)g.riot.questioned,
        (unsigned long long)g.quietLines.questioned, (unsigned long long)g.origSteps,
        (unsigned long long)g.coreSteps, kinds.empty() ? "none" : kinds.c_str());
    DiffKinds().clear();
    EndCity();
}

// A fire the original refused (Q40) while it has no episode: its tick does
// not run, so the mirror's is driven here until the mirror's episode ends.
void ResyncTick() {
    if (g.resyncs == 0 || g.inCore) return;
    const fc4_disasters_state s = CoreState();
    const int32_t origPhase = g.fireInst ? g.fireInst->GetPhase() : 0;
    if (s.fire_phase != 1 || origPhase == 1) return;
    for (size_t i = 0;; ++i) {
        fc4_disaster_fire f{};
        if (!fc4_disasters_get_fire(g.core, i, &f)) break;
        if (g.resyncTargets.count(f.target) == 0) return;
    }
    if (s.fire_count != 0 && g.fireNextValid && g.fireClock < g.fireNext) g.fireClock = g.fireNext;
    const uint64_t before = g.coreSteps;
    CoreCall("disasters shadow fire tick (resync, ours)", true,
             [&] { fc4_disasters_tick(g.core, s.fire_instance, 0); });
    const bool stepped = g.coreSteps != before;
    g.resyncSteps += g.coreSteps - before;
    if (stepped && g.fireNextValid) g.fireNext += 1000;
    const uint32_t phaseBefore = g.corePhase;
    TrackFirePhase();
    if (stepped || g.corePhase != phaseBefore) {
        g.dirty = true;
        FireLine("resync tick (a fire the original refused, Q40; the original has no fire tick)");
    }
}

// The schedule is harness policy, not disaster behaviour. Wait for a new month
// after load, then run one fire and one riot serially. Animation ticks use wall
// time even during the disaster's emergency pause. Never force a phase or resume
// the simulator behind the layer's back. Other disaster types remain off (DS28).
void ScriptedDisasters(const fc4_date& date) {
    if (!g.scripted || g.scriptStage == 3) return;
    cISC4DisasterLayer* layer = Layer();
    if (!layer || layer->IsADisasterOngoing() || g.dirty) return;
    if (g.scriptStage == 0) {
        if (date.year < 2026 || (date.year == 2026 && date.month < 2)) return;
        g.restoreTurbo = TurboConfig().on;
        TurboScriptSet(false);
        Log("disasters scripted: fire: %s", DisasterScriptAction(g.city, 0, -1, -1).c_str());
        if (OrigFireCount() <= 0) ++g.failures;
        g.scriptStage = 1;
    } else if (g.scriptStage == 1) {
        Log("disasters scripted: riot: %s", DisasterScriptAction(g.city, 1, -1, -1).c_str());
        if (g.riots.empty()) ++g.failures;
        g.scriptStage = 2;
    } else {
        g.scriptStage = 3;
        if (g.restoreTurbo) TurboScriptSet(true);
        Log("disasters scripted: complete: fire=%llu riot=%llu resyncs=%llu",
            (unsigned long long)g.fire.lines, (unsigned long long)g.riot.lines,
            (unsigned long long)g.resyncs);
    }
}

void MonthlyLine(const fc4_date& date) {
    Diffs d;
    d.scope = "month";
    char layer[96];
    CompareLayer(d, layer, sizeof(layer), nullptr);
    const uint64_t mismatches = g.differences + g.failures + g.calls.lines + g.resyncs +
                                g.unfollowable + g.ctx.leadersMissing;
    Log("disasters shadow %04d-%02d-%02d: fire=%llu/%llu riot=%llu/%llu quiet=%llu/%llu "
        "%s script=%u resyncs=%llu unfollowable=%llu failures=%llu mismatches=%llu",
        date.year, date.month, date.day_of_month,
        (unsigned long long)g.fire.clean, (unsigned long long)g.fire.lines,
        (unsigned long long)g.riot.clean, (unsigned long long)g.riot.lines,
        (unsigned long long)g.quietLines.clean, (unsigned long long)g.quietLines.lines,
        layer, g.scriptStage, (unsigned long long)g.resyncs, (unsigned long long)g.unfollowable,
        (unsigned long long)g.failures, (unsigned long long)mismatches);
}

void DisastersShadowOnTick() {
    if (g.core == nullptr || GetCurrentThreadId() != g.mainTid) return;
    ResyncTick();
    auto* sim = g.city ? AsGame(g.city->GetSimulator()) : nullptr;
    fc4_date date{};
    if (sim) {
        fc4_date_from_julian(sim->GetSimDateNumber(), &date);
        ScriptedDisasters(date);
        const int32_t month = date.year * 12 + date.month;
        if (month != g.reportMonth) {
            // Include the load sample; thereafter one line per observed month.
            g.reportMonth = month;
            MonthlyLine(date);
        }
    }
    if (!g.dirty) return;
    const int64_t ongoing = g.origBegun - g.origEnded;
    const fc4_disasters_state s = CoreState();
    if (ongoing != 0 || s.ongoing != 0) {
        g.quiet = 0;
        return;
    }
    if (++g.quiet < kQuietFrames) return;
    g.quiet = 0;
    g.dirty = false;
    QuietLine();
}

void DisastersShadowStop() {
    UnpatchAll("PreAppShutdown");
}

void DisastersShadowDetach() {
    UnpatchAll("DLL detach");
}

std::string DisasterScriptAction(cISC4City* city, int32_t kind, int32_t x, int32_t z) {
    if (city == nullptr) return "skipped: no city";
    char b[512];
    if (kind == harness::kDisasterSurvey) {
        std::string text = "fire candidates (uncovered, flammable):";
        for (const Candidate& c : FireCandidates(city, 6)) {
            std::snprintf(b, sizeof(b), " %d,%d (flammability %.0f)", c.x, c.z, c.flammability);
            text += b;
        }
        text += "; riot candidates (road, police coverage <= 64):";
        for (const Candidate& c : RiotCandidates(city, 6)) {
            std::snprintf(b, sizeof(b), " %d,%d (police %u)", c.x, c.z, c.coverage);
            text += b;
        }
        return text;
    }
    const bool fire = kind == 0;
    cISC4DisasterLayer* layer = city->GetDisasterLayer();  // slot 72, borrowed
    cISC4DisasterManager* m = layer ? layer->GetDisasterManager(fire ? kFire : kRiot) : nullptr;  // borrowed
    if (m == nullptr) return "no disaster manager";
    cISC4DisasterInstanceFire* fireInst = nullptr;  // borrowed
    void* p = nullptr;
    if (fire && m->QueryInterface(kIID_cISC4DisasterManagerFire, &p) && p != nullptr) {
        cISC4DisasterManagerFire* f = static_cast<cISC4DisasterManagerFire*>(p);
        FC4_ACQUIRED(f, "QueryInterface");
        fireInst = f->GetFireInstance();  // slot 11, borrowed
        FC4_RELEASE(f);
    }
    std::vector<Candidate> cells;
    const bool autoPick = x < 0 || z < 0;
    if (autoPick) {
        // auto: the survey's candidates in order until the original starts one
        // (a fire: until its fire count rises).
        cells = fire ? FireCandidates(city, 6) : RiotCandidates(city, 1);
        if (cells.empty()) return fire ? "no fire candidate (see disaster survey)" : "no riot candidate";
    } else {
        Candidate c;
        c.x = x;
        c.z = z;
        cells.push_back(c);
    }
    std::string text;
    for (const Candidate& c : cells) {
        const int32_t before = fireInst ? fireInst->GetFireCount() : -1;
        // Manager slot 6, AttemptDisasterStart(x, z) (foundations §17: the
        // overloads compile in reverse order, so the SDK's (x, z) is slot 6).
        cISC4DisasterInstance* inst = m->AttemptDisasterStart(c.x, c.z);
        const int32_t after = fireInst ? fireInst->GetFireCount() : -1;
        std::snprintf(b, sizeof(b),
                      "%s%s%s AttemptDisasterStart(%d, %d) -> instance %p; fires %d->%d active=%d ongoing=%d",
                      text.empty() ? "" : "; ", autoPick ? "auto " : "", fire ? "fire" : "riot", c.x, c.z,
                      static_cast<void*>(inst), before, after, m->GetActiveInstanceCount(),
                      layer->IsADisasterOngoing());
        text += b;
        if (!fire || after > before) break;
    }
    return text;
}

}  // namespace fc4
