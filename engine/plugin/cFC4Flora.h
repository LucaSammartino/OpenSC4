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

// The flora simulator's GZCOM class (docs/specs/flora.md §1, CLSID
// 0xC990C034). The model lives in the Rust core behind fc4.h (fc4_flora over
// the process-wide fc4_flora_registry); this class implements
// cISC4FloraSimulator (§11.1: the SDK declaration is correct, its overload
// pairs compile reversed exactly as the game's vtable has them),
// cIGZMessageTarget2 (§11.2) and cIGZSerializable (§11.3), and the host
// callbacks over the game's services (§12).
//
// Flora is occupant-based: every tree is the GAME's cSC4FloraOccupant, created
// through GZCOM (class 0xA9C05C85, §13.2) and handled only through its
// interfaces (cISC4Occupant, cISC4FloraOccupant §11.4) and the occupant
// manager's.
//
// The static flora records are the GAME's layout (docs/abi/layouts.md "Static
// flora record", §13.2): GetStaticFloraData hands out a raw 0x178-byte block
// that the flora occupant, the flora tool and the terrain brush read and
// reference-count directly. The plugin builds one block per record of the
// process-wide registry and keeps one reference on it for the life of the
// process [choice, §13.2, FQ8], so no game Release ever frees one (the blocks
// and the exemplars they hold are never freed).
//
// The subsystem is OFF by default: FreeCity4.ini `enable = flora` turns it on
// (FC4Subsystems.h); `disable` still wins. The flora_* switches (FC4Config.h)
// turn groups of game calls off for bisecting.
//
// Threads: the registry getters (slots 7-14) read the registry's snapshot and
// take no lock (any thread). Every other call into the core (lifecycle,
// DoMessage, Write / Read, slots 5, 6 and 15-24) runs under one process-wide
// re-entrant lock (CoreCall), the same order as the core's own lock, so a
// second thread waits for a month in progress instead of racing it.
//
// The obliterated city's trees (§10.3, FQ3): SaveObliterated and Init's
// restore go through the segment's cISC4DBSegment and its record streams (the
// flora_obliteration switch). [choice] The application shutdown callback
// (§4.1 step 2) is not registered: it has no observable original behaviour
// (FQ2, FQ8), and the records live for the whole process.
#pragma once

#include "FC4Interfaces.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4FloraSimulator.h"
#include "fc4.h"

#include <cstddef>
#include <cstdint>
#include <memory>
#include <mutex>
#include <vector>

class cIGZCheatCodeManager;
class cIGZMessageServer2;
class cISC4City;
class cISC4LotManager;
class cISC4OccupantManager;
class cISC4Simulator;
class cISC4WeatherSimulator;
class cISCResExemplar;
class cISTETerrain;

static const uint32_t kFC4_CLSID_cSC4FloraSimulator = 0xC990C034;
static const uint32_t kFC4_IID_cISC4FloraSimulator = 0x06C66177;

// The static flora record (docs/abi/layouts.md, 0x178 bytes; flora.md §3.3).
// No vtable; +0x170 is a plain reference count that holders change directly.
struct FC4FloraBlock {
    uint32_t type;              // +0x000
    uint32_t next;              // +0x004 next cluster type
    uint32_t seed;              // +0x008 seed type (§6.11)
    uint8_t water;              // +0x00C
    uint8_t pad0[3];            // +0x00D
    float floats[14];           // +0x010 property 0x083DC35B (+0x38 half-width, +0x44 height)
    float clusterRadius;        // +0x048
    float clusterHeight;        // +0x04C
    uint8_t table[256];         // +0x050 [elevation index][moisture type]
    uint32_t unnamed;           // +0x150 property 0x083DE16F
    uint32_t* familyBegin;      // +0x154 std::vector<uint32_t> (B), or null
    uint32_t* familyEnd;        // +0x158
    uint32_t* familyCapacity;   // +0x15C
    int64_t plopCost;           // +0x160
    cISCResExemplar* exemplar;  // +0x168 a counted reference, or null
    uint8_t flags;              // +0x16C
    uint8_t pad1[3];            // +0x16D
    int32_t refCount;           // +0x170
    uint32_t tail;              // +0x174 padding
};
#if defined(_M_IX86) || defined(__i386__)
static_assert(sizeof(FC4FloraBlock) == 0x178, "static flora record: 0x178 bytes");
static_assert(offsetof(FC4FloraBlock, floats) == 0x10 && offsetof(FC4FloraBlock, clusterRadius) == 0x48,
              "static flora record layout");
static_assert(offsetof(FC4FloraBlock, table) == 0x50 && offsetof(FC4FloraBlock, unnamed) == 0x150,
              "static flora record layout");
static_assert(offsetof(FC4FloraBlock, familyBegin) == 0x154 && offsetof(FC4FloraBlock, plopCost) == 0x160,
              "static flora record layout");
static_assert(offsetof(FC4FloraBlock, exemplar) == 0x168 && offsetof(FC4FloraBlock, flags) == 0x16C,
              "static flora record layout");
static_assert(offsetof(FC4FloraBlock, refCount) == 0x170, "static flora record layout");
#endif

namespace fc4 {

// The process-wide flora registry (§3.2), created on first use and never
// destroyed. Its blocks are FC4FloraBlocks built by the plugin.
fc4_flora_registry* FloraRegistry();

// The process-wide re-entrant lock every call into the flora core runs under.
std::recursive_mutex& FloraLock();

// Each flora exemplar (resource type 0x6534284A, property 0x10 = 0x0F) in the
// resource manager's order, as the host's flora_exemplars (§4.1 step 2):
// `visit` gets NEW props (taken) and, with `keepRefs`, one exemplar reference
// (taken), else 0. `gated`: obey the flora_exemplars / exemplar_loads
// switches (ours); the shadow reads the original's inputs as they are.
void FloraForEachExemplar(bool keepRefs, bool gated, fc4_flora_exemplar_visitor visit, void* ctx);

// The values of a game-layout record (ours or the original's). The family IDs
// are borrowed from the block's vector. False for a null block.
bool FloraBlockValues(const void* block, fc4_flora_record* out);

}  // namespace fc4

class cFC4FloraSimulator final : public cISC4FloraSimulator, public cIGZMessageTarget2, public cIGZSerializable {
public:
    cFC4FloraSimulator();
    ~cFC4FloraSimulator();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4FloraSimulator (§11.1, the SDK declaration)
    bool Init() override;
    bool Shutdown() override;
    bool AddStaticFloraData(cSC4StaticFloraData* record) override;                         // 6
    bool AddStaticFloraData(cGZPersistResourceKey* key, cISCResExemplar* exemplar) override;  // 5
    cSC4StaticFloraData* GetStaticFloraData(uint32_t type) override;                        // 7
    uint32_t GetAllFloraTypes(uint32_t* buffer, uint32_t& count) override;                  // 9
    void GetAllFloraTypes(SC4Vector<uint32_t>& vector) override;                            // 8
    void GetAllFloraFamilyTypes(SC4Vector<uint32_t>& vector) override;                      // 10
    bool IsFloraTypeValid(uint32_t type) override;                                          // 11
    bool IsFloraFamilyTypeValid(uint32_t type) override;                                    // 12
    uint32_t GetFloraTypesByFilter(intptr_t& hashSet, cISC4FloraTypeFilter* filter) override;  // 13
    uint32_t GetFloraTypesByFamily(intptr_t& hashSet, uint32_t family) override;               // 14
    bool LocationIsOKForNewFlora(uint32_t type, float x, float z, cISC4Occupant* exclude) override;  // 15
    uint32_t GetBestFloraTypeForLocation(float x, float z, float& score) override;                  // 16
    bool AddNewFloraOccupant(cISC4Occupant* occupant) override;                                     // 18
    cISC4Occupant* AddNewFloraOccupant(uint32_t type, float* pos, bool force) override;            // 17
    bool DemolishFloraOccupant(cISC4Occupant* occupant, int32_t flags) override;                    // 19
    bool DemolishFloraInCellArea(SC4Rect<int32_t> const& rect, int32_t flags) override;             // 20
    bool DemolishFloraInBoundingBox(float const* min, float const* max, int32_t flags) override;    // 21
    int64_t GetCostOfOccupantDemolition(cISC4Occupant* occupant, int32_t flags) override;           // 22
    bool KillFloraOccupant(cISC4Occupant* occupant) override;                                       // 23
    bool SaveObliterated(cIGZPersistDBSegment* segment) override;                                   // 24

    // cIGZMessageTarget2 (§5)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§10.1)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // For the host callbacks (cFC4Flora.cpp).
    cIGZMessageTarget2* Target() { return static_cast<cIGZMessageTarget2*>(this); }
    cISC4FloraSimulator* Primary() { return static_cast<cISC4FloraSimulator*>(this); }
    // The token the core knows this object by (its cISC4FloraSimulator*, the
    // identity the primary QueryInterface hands out).
    uint64_t SelfToken() const {
        return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(static_cast<const cISC4FloraSimulator*>(this)));
    }
    cISC4City* City() const { return city_; }
    cISC4OccupantManager* OccupantManager() const { return occupantManager_; }
    cISC4LotManager* LotManager() const { return lotManager_; }
    cISTETerrain* Terrain() const { return terrain_; }
    cISC4WeatherSimulator* Weather() const { return weather_; }
    cISC4Simulator* Simulator() const { return simulator_; }
    // The city (one reference of ours), fetched once from the application.
    cISC4City* EnsureCity();
    void AcquireServices();
    void ReleaseServices();
    void Subscribe(const uint32_t* ids, size_t count, bool add);
    void AddAgent(uint32_t level, const char* name, uint32_t flags);
    void RemoveAgent();
    void RegisterCheat(uint32_t id, const char* name);
    void UnregisterCheat(uint32_t id);
    void RegisterCommand(uint32_t id, const char* name);
    void UnregisterCommand(uint32_t id);
    // Keeps a game list (and the references it holds) until the outermost call
    // into the core returns.
    void Hold(std::shared_ptr<void> list) { held_.push_back(std::move(list)); }

    // Counts the calls into the core under the process-wide lock; the
    // outermost one drops the held lists.
    class CoreCall {
    public:
        explicit CoreCall(cFC4FloraSimulator* s);
        ~CoreCall();
        CoreCall(const CoreCall&) = delete;
        CoreCall& operator=(const CoreCall&) = delete;

    private:
        cFC4FloraSimulator* s_;
        std::unique_lock<std::recursive_mutex> lock_;
    };

    // Observability only (status lines).
    struct Probes {
        uint32_t offMainThread = 0;  // slots 5, 6, 15-24 called off the main thread
        uint32_t lockWaits = 0;      // CoreCalls that found the lock taken
        uint32_t messages = 0;       // DoMessage calls
        uint32_t days = 0, months = 0;
    };
    Probes& Probe() { return probes_; }

private:
    void LogState(const char* what, int32_t julian);
    void NoteThread(const char* what);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4_flora_host host_{};
    fc4_flora* core_ = nullptr;
    cISC4City* city_ = nullptr;
    cISC4OccupantManager* occupantManager_ = nullptr;
    cISC4LotManager* lotManager_ = nullptr;
    cISTETerrain* terrain_ = nullptr;
    cISC4WeatherSimulator* weather_ = nullptr;
    cISC4Simulator* simulator_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    cIGZCheatCodeManager* cheatMgr_ = nullptr;
    uint32_t cheatId_ = 0;
    uint32_t commandId_ = 0;  // registered with the command server (0 = none)
    bool agentsRegistered_ = false;
    std::vector<std::shared_ptr<void>> held_;
    int depth_ = 0;
    fc4_flora_state last_{};  // the counters at the previous month line
    Probes probes_;
};

// Factory for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4FloraSimulator();
