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

#include "cFC4ZoneDeveloper.h"

#include "FC4ZoneDevShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Seed.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4ZoneManager.h"
#include "compat/FC4Compat.h"

#include "SC4CellRegion.h"
#include "SC4Point.h"
#include "SC4Rect.h"
#include "cIGZMessage2Standard.h"
#include "cISC4Lot.h"
#include "cISC4LotConfiguration.h"
#include "cISC4Occupant.h"
#include "cS3DVector3.h"

#include <atomic>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <mutex>
#include <string>
#include <vector>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cIGZSerializable = 0xE4FDA3D4;

// §5.2.
constexpr uint32_t kMsgInsertOccupant = 0x99EF1142;
constexpr uint32_t kMsgViewValue = 0xA6B79602;

// The zone developer the city runs (Init to Shutdown), for the month line.
std::atomic<cFC4ZoneDeveloper*> g_live{nullptr};

// Live instances, for the shadow and the test seam (op 53).
std::mutex g_instancesLock;
std::vector<cFC4ZoneDeveloper*> g_instances;

// A few lines, then silence (any thread).
void NoteFew(volatile LONG* n, const char* text) {
    if (InterlockedIncrement(n) <= 3) fc4::Log("%s", text);
}
volatile LONG g_notePlopped = 0;
volatile LONG g_noteHighlight = 0;
volatile LONG g_noteListAllocator = 0;
volatile LONG g_noteIterator = 0;
volatile LONG g_noteNoLots = 0;

// §9 [choice] (foundations §6.3, as the tract developer's): the session seed
// S from `zonedev_seed` / FC4_ZONEDEV_SEED, else the cRZTime clock (seconds
// since 1901-01-01). The core seeds the process-wide generator with
// derive_seed(S, 0x8C09C68A) once (fc4.h).
uint32_t SessionSeed(bool* fixed) {
    const std::string s = fc4::Setting("zonedev_seed", "FC4_ZONEDEV_SEED");
    if (!s.empty()) {
        char* end = nullptr;
        const unsigned long v = std::strtoul(s.c_str(), &end, 0);
        if (end != s.c_str()) {
            *fixed = true;
            return static_cast<uint32_t>(v);
        }
    }
    *fixed = false;
    // Else the process's one session seed (FC4Seed.h, foundations §6.3).
    return fc4::SessionSeed();
}

template <class T>
T* Query(cIGZUnknown* u, uint32_t iid) {
    if (u == nullptr) return nullptr;
    void* p = nullptr;
    if (!u->QueryInterface(iid, &p) || p == nullptr) return nullptr;
    T* t = static_cast<T*>(p);
    FC4_ACQUIRED(reinterpret_cast<cIGZUnknown*>(t), "QueryInterface");
    return t;
}

template <class T>
void Drop(T* p) {
    if (p != nullptr) FC4_RELEASE(reinterpret_cast<cIGZUnknown*>(p));
}

// ---- the caller's lot lists (§8.3: B nodes, one reference per element) ---------------------------------

using LotList = fc4::GameList<cISC4Lot*>;
using LotNode = LotList::Node;

// A reference the caller's list held, given back (slot 7's clear): the test
// table's release, our own object's Release, or the game's (noted as
// acquired first: the list's references are the caller's, handed to us).
void ReleaseListEntry(const fc4::ZoneDevContext& c, cISC4Lot* lot) {
    if (lot == nullptr) return;
    if (c.test != nullptr && c.test->release != nullptr) {
        c.test->release(c.test->user, TokenOf(lot));
        return;
    }
    cIGZUnknown* o = reinterpret_cast<cIGZUnknown*>(lot);
    if (fc4::IsOurObject(o)) {
        o->Release();
        return;
    }
    FC4_ACQUIRED(o, "the caller's lot list entry (GetLotsInCellRect clears it)");
    FC4_RELEASE(o);
}

// Slot 7's clear: every element released, every node freed with B; the
// head is kept. Returns the nodes freed (0 for a list without a head).
uint32_t ClearCallerList(const fc4::ZoneDevContext& c, LotList& list) {
    LotNode* head = list.head();
    if (head == nullptr) return 0;
    uint32_t n = 0;
    for (LotNode* p = head->next; p != head && p != nullptr;) {
        LotNode* next = p->next;
        ReleaseListEntry(c, p->value);
        fc4::ZoneDevAllocators::FreeNode(p);
        p = next;
        ++n;
    }
    head->next = head;
    head->prev = head;
    return n;
}

// The lots already in a caller's list (slots 8-10 append without clearing
// and test uniqueness against them).
void ListTokens(LotList& list, std::vector<uint64_t>& out) {
    out.clear();
    LotNode* head = list.head();
    if (head == nullptr) return;
    uint32_t n = 0;
    for (LotNode* p = head->next; p != head && p != nullptr && n < (1u << 20); p = p->next, ++n) {
        out.push_back(TokenOf(p->value));
    }
}

// The core's callback: one lot with ONE reference for the caller, appended
// to the caller's list with a B node.
struct Appender {
    const fc4::ZoneDevContext* ctx;
    LotNode* head;
    uint32_t appended = 0;
    uint32_t dropped = 0;
};

void AppendLot(void* p, uint64_t token) {
    Appender* a = static_cast<Appender*>(p);
    cISC4Lot* lot = reinterpret_cast<cISC4Lot*>(FromToken(token));
    LotNode* n = a->head != nullptr
                     ? static_cast<LotNode*>(fc4::ZoneDevAllocators::AllocateNode(static_cast<uint32_t>(sizeof(LotNode))))
                     : nullptr;
    if (n == nullptr) {
        // No list or no node: the caller's reference goes back at once.
        ++a->dropped;
        NoteFew(&g_noteListAllocator, "zonedev: a lot list could not take a lot (no list or no STL node); dropped");
        ReleaseListEntry(*a->ctx, lot);
        return;
    }
    n->value = lot;
    n->next = a->head;
    n->prev = a->head->prev;
    a->head->prev->next = n;
    a->head->prev = n;
    ++a->appended;
}

fc4_cell_rect RectOf(const SC4Rect<long>& r) {
    return fc4_cell_rect{static_cast<int32_t>(r.topLeftX), static_cast<int32_t>(r.topLeftY),
                         static_cast<int32_t>(r.bottomRightX), static_cast<int32_t>(r.bottomRightY)};
}

void PutRect(SC4Rect<long>& out, const fc4_cell_rect& r) {
    out.topLeftX = r.x1;
    out.topLeftY = r.z1;
    out.bottomRightX = r.x2;
    out.bottomRightY = r.z2;
}

const char* Plural(uint32_t n) { return n == 1 ? "" : "s"; }

}  // namespace

namespace fc4 {

void ZoneDevMonthLine(int32_t julian) {
    cFC4ZoneDeveloper* z = g_live.load();
    if (z != nullptr) z->LogMonth(julian);
}

}  // namespace fc4

// ---- object -------------------------------------------------------------------------------------------------

cIGZUnknown* CreateFC4ZoneDeveloper() {
    return static_cast<cIGZUnknown*>(static_cast<cISC4ZoneDeveloper*>(new cFC4ZoneDeveloper()));
}

cFC4ZoneDeveloper::cFC4ZoneDeveloper() {
    InitializeCriticalSection(&lock_);
    mainTid_ = GetCurrentThreadId();  // the class store creates city components on the main thread
    ctx_.target = static_cast<cIGZMessageTarget2*>(this);
    ctx_.master = &master_;
    seed_ = SessionSeed(&seedFixed_);
    CreateCore();
    {
        std::lock_guard<std::mutex> g(g_instancesLock);
        g_instances.push_back(this);
    }
    RefreshSnapshot();
    fc4::Log("cFC4ZoneDeveloper(%p) created, core %p, session seed 0x%08X (%s); lots from the game's lot pool "
             "(§8.5.1)",
             static_cast<void*>(Primary()), static_cast<void*>(core_), seed_, seedFixed_ ? "zonedev_seed" : "session seed");
}

// The core over this object's table (the test seam's callbacks answer first,
// through the context). The first core of the process seeds the shared
// generator (fc4.h).
void cFC4ZoneDeveloper::CreateCore() {
    host_ = fc4::MakeZoneDevHost(&ctx_);
    core_ = fc4_zonedev_create(&host_, seed_);
    if (core_ == nullptr) fc4::Log("cFC4ZoneDeveloper: core creation failed");
}

cFC4ZoneDeveloper::~cFC4ZoneDeveloper() {
    {
        std::lock_guard<std::mutex> g(g_instancesLock);
        for (size_t i = 0; i < g_instances.size(); ++i) {
            if (g_instances[i] == this) {
                g_instances.erase(g_instances.begin() + static_cast<std::ptrdiff_t>(i));
                break;
            }
        }
    }
    cFC4ZoneDeveloper* self = this;
    g_live.compare_exchange_strong(self, nullptr);
    fc4_zonedev_destroy(core_);  // releases what a never-initialised object still holds (a Read's lots)
    core_ = nullptr;
    fc4::ZoneDevReleaseHeld(&ctx_);
    fc4::ZoneDevReleaseServices(&ctx_);
    const uint32_t orphans = master_.OrphanIterators();
    fc4::Log("cFC4ZoneDeveloper destroyed (%u fallbacks, %u safe lot iterator%s orphaned)", fallbacks_.load(), orphans,
             Plural(orphans));
    DeleteCriticalSection(&lock_);
}

// §1: which sub-object answers is the core's decision; anything else is
// false and the out pointer is left untouched.
bool cFC4ZoneDeveloper::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr || !fc4_zonedev_query_interface(riid)) return false;
    if (riid == kFC4_IID_cISC4LotManager) {
        *ppvObj = static_cast<cFC4ISC4LotManager*>(this);
    } else if (riid == kIID_cIGZMessageTarget2) {
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
    } else if (riid == kIID_cIGZSerializable) {
        *ppvObj = static_cast<cIGZSerializable*>(this);
    } else {
        *ppvObj = Primary();  // 0x00000001 and 0xE818ED2B
    }
    ++refCount_;
    return true;
}

uint32_t cFC4ZoneDeveloper::AddRef() { return ++refCount_; }

// As the zone manager's (§1): count > 1 → decrement; otherwise Shutdown, then destroy.
uint32_t cFC4ZoneDeveloper::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    if (destroying_) return 0;  // a release from inside the final Shutdown
    destroying_ = true;
    Shutdown();
    delete this;
    return 0;
}

cFC4ZoneDeveloper* cFC4ZoneDeveloper::FromAny(const void* p) {
    if (p == nullptr) return nullptr;
    std::lock_guard<std::mutex> g(g_instancesLock);
    for (cFC4ZoneDeveloper* z : g_instances) {
        if (static_cast<const void*>(z->Primary()) == p ||
            static_cast<const void*>(static_cast<cFC4ISC4LotManager*>(z)) == p ||
            static_cast<const void*>(static_cast<cIGZMessageTarget2*>(z)) == p ||
            static_cast<const void*>(static_cast<cIGZSerializable*>(z)) == p) {
            return z;
        }
    }
    return nullptr;
}

const void* cFC4ZoneDeveloper::SafeIteratorListObject(const void* lotManager, const char** why) {
    const char* reason = nullptr;
    const void* list = nullptr;
    const fc4::compat::ExecutableVersion* v = fc4::compat::IdentifiedVersion();
    if (lotManager == nullptr) {
        reason = "no lot manager";
    } else if (v == nullptr) {
        reason = "executable not identified";
    } else {
        const cFC4ZoneDeveloper* ours = FromAny(lotManager);
        list = fc4::safelots::ExpectedListObject(lotManager, v->imageBase, v->sizeOfImage,
                                                 ours != nullptr ? ours->MasterListObject() : nullptr);
        if (list == nullptr) reason = "the lot manager is neither the original nor FreeCity4's";
    }
    if (why != nullptr) *why = reason;
    return list;
}

void cFC4ZoneDeveloper::SetTestHooks(const fc4_zonedev_host* test) {
    Section sec(this);
    ctx_.test = test;
    fc4_zonedev_destroy(core_);
    host_ = fc4::MakeZoneDevHost(&ctx_);
    if (test != nullptr && test->grids.create != nullptr) host_.grids = test->grids;
    core_ = fc4_zonedev_create(&host_, seed_);
    fc4::Log("zonedev: test hooks %s (core %p)", test ? "installed" : "removed", static_cast<void*>(core_));
}

// ---- the lock (cFC4ZoneManager's) ------------------------------------------------------------------------

bool cFC4ZoneDeveloper::Enter() {
    const DWORD me = GetCurrentThreadId();
    if (mainTid_ == 0 || me == mainTid_) {
        EnterCriticalSection(&lock_);
        return true;
    }
    if (TryEnterCriticalSection(&lock_)) return true;
    const ULONGLONG until = GetTickCount64() + kLockWaitMs;
    while (GetTickCount64() < until) {
        Sleep(1);
        if (TryEnterCriticalSection(&lock_)) return true;
    }
    ++fallbacks_;
    return false;
}

void cFC4ZoneDeveloper::Leave() { LeaveCriticalSection(&lock_); }

cFC4ZoneDeveloper::Section::Section(const cFC4ZoneDeveloper* s)
    : s_(const_cast<cFC4ZoneDeveloper*>(s)), ok_(s_->Enter()) {
    if (ok_) ++s_->depth_;
}

cFC4ZoneDeveloper::Section::~Section() {
    if (!ok_) return;
    if (--s_->depth_ == 0) {
        // The outermost call: the references and lists kept for it go, and
        // the values other threads fall back to are refreshed.
        s_->ctx_.forCaller = false;
        fc4::ZoneDevReleaseHeld(&s_->ctx_);
        s_->RefreshSnapshot();
    }
    s_->Leave();
}

// Only the core's own state (no host calls): any thread holding the lock.
void cFC4ZoneDeveloper::RefreshSnapshot() {
    if (core_ == nullptr) return;
    fc4_zonedev_state st{};
    st.struct_size = sizeof(st);
    if (fc4_zonedev_get_state(core_, &st)) {
        snapLotCount_ = static_cast<int32_t>(st.lot_count);
        snapGrid_ = reinterpret_cast<cISC4SimGrid<uint16_t>*>(FromToken(st.max_capacity_grid));
    }
}

// The status lines (Init, Read, Write, Shutdown, month). Inside a section.
void cFC4ZoneDeveloper::LogState(const char* what) {
    fc4_zonedev_state st{};
    st.struct_size = sizeof(st);
    if (core_ == nullptr || !fc4_zonedev_get_state(core_, &st)) return;
    fc4::Log("zonedev %s: version=1 init=%u loaded=%u real=%u lots=%u map=%dx%d created=%u deleted=%u "
             "iterators=%u issued=%u parcellizations=%u dezones=%u reentrant=%u refused=%u built=%u streets=%u "
             "joins=%u demolitions=%u levellings=%u removed=%u posts=%u options=0x%X grid=%p view=0x%X "
             "fallbacks=%u can_create_refused=%u",
             what, st.initialized, st.loaded, st.real_mode, st.lot_count, st.map_x, st.map_z, st.lots_created,
             st.lots_deleted, master_.RegisteredIterators(), ctx_.iteratorsIssued, st.parcellizations, st.dezones,
             st.reentrant_calls, ctx_.newLotRefusals, ctx_.lotsBuilt, ctx_.streetsPlaced, ctx_.joins,
             ctx_.demolitions, ctx_.levellings,
             ctx_.removedOccupants, ctx_.posts, st.options, static_cast<void*>(FromToken(st.max_capacity_grid)),
             st.view_value, fallbacks_.load(), ctx_.canCreateRefusals);
}

void cFC4ZoneDeveloper::LogMonth(int32_t julian) {
    Section sec(this);
    if (!sec.ok()) return;
    fc4_date d{};
    fc4_date_from_julian(julian, &d);
    char what[48];
    _snprintf_s(what, sizeof(what), _TRUNCATE, "month %04d-%02d-%02d", d.year, d.month, d.day_of_month);
    LogState(what);
}

// ---- lifecycle (§1.2) -----------------------------------------------------------------------------------------

// Zone developer slot 3; lot manager slot 3 tail-jumps here (one override
// fills both). §1.2 without the 3D view (§13): the core drives every step.
// [question] (§1.2 step 7) The cheats are not registered: the host has no
// cheat-manager call and their bodies are not specified (§5.2).
bool cFC4ZoneDeveloper::Init() {
    FC4_TRACE("ZoneDeveloper::Init");
    mainTid_ = GetCurrentThreadId();
    Section sec(this);
    const bool ok = fc4_zonedev_init(core_);
    if (!master_.Ensure()) fc4::Log("zonedev: the raw master lot list could not be made (no STL node allocator)");
    g_live = this;
    LogState(ok ? "Init" : "Init FAILED");
    return ok;
}

bool cFC4ZoneDeveloper::Shutdown() {
    FC4_TRACE("ZoneDeveloper::Shutdown");
    // Keep this object alive while the message server drops its reference
    // (unless this already is the final Release).
    if (!destroying_) Primary()->AddRef();
    {
        Section sec(this);
        fc4_zonedev_shutdown(core_);
        LogState("Shutdown");
    }
    cFC4ZoneDeveloper* self = this;
    g_live.compare_exchange_strong(self, nullptr);
    if (!destroying_) Primary()->Release();
    return true;  // AL = 1
}

// ---- cISC4ZoneDeveloper (§7.1, §8.8, §11.2) ------------------------------------------------------------------

// Slot 5 (UI preview). [question] (§11.2, §13.2: *out receives a new C
// region the caller frees; the caller is not traced) No region is handed
// out: *out := null, a value the original also stores (§13.2).
void cFC4ZoneDeveloper::HighlightParcels(SC4CellRegion<long> const& region, cISC4ZoneManager::ZoneType zoneType,
                                         SC4Point<long>* point, SC4CellRegion<long>** out) {
    Section sec(this);
    if (out != nullptr) *out = nullptr;
    if (!sec.ok() || core_ == nullptr) return;
    std::vector<uint8_t> mask;
    fc4_zone_region r{};
    if (!fc4::ReadZoneRegion(&region, mask, &r, "zonedev")) return;
    int32_t pt[2] = {0, 0};
    if (point != nullptr) {
        pt[0] = static_cast<int32_t>(point->x);
        pt[1] = static_cast<int32_t>(point->y);
    }
    bool written = false;
    fc4_cell_rect outRect{0, 0, -1, -1};
    fc4_zonedev_parcel_result res{};
    res.struct_size = sizeof(res);
    fc4_zonedev_highlight_parcels(core_, &r, static_cast<int32_t>(zoneType), point != nullptr ? pt : nullptr,
                                  out != nullptr, &written, &outRect, &res);
    if (written) {
        NoteFew(&g_noteHighlight, "zonedev: HighlightParcels' region is not handed out (*out = null; the caller "
                                  "and its allocator are not traced, zones.md §13.2; question)");
    }
}

// Slot 6: the caller's region is neither freed nor kept (§13.2).
void cFC4ZoneDeveloper::DoParcellization(SC4CellRegion<long> const& region, cISC4ZoneManager::ZoneType zoneType,
                                         bool flag) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return;  // [choice] another thread that cannot get the lock: not done
    std::vector<uint8_t> mask;
    fc4_zone_region r{};
    if (!fc4::ReadZoneRegion(&region, mask, &r, "zonedev")) return;
    uint32_t cells = 0;
    for (uint8_t b : mask) cells += b != 0 ? 1u : 0u;
    const uint32_t refusedBefore = ctx_.newLotRefusals;
    fc4_zonedev_parcel_result res{};
    res.struct_size = sizeof(res);
    fc4_zonedev_do_parcellization(core_, &r, static_cast<int32_t>(zoneType), flag, &res);
    FC4_X87_CHECK("zonedev DoParcellization");
    const int32_t t = static_cast<int32_t>(zoneType);
    fc4::Log("zonedev %s: type=%d rect=%d,%d..%d,%d cells=%u flag=%u lots=%u failed=%u deleted=%u streets=%u/%u "
             "demolish=%u%s",
             (t >= 1 && t <= 9) ? "parcellize" : "dezone", t, r.x0, r.z0, r.x1, r.z1, cells, flag ? 1u : 0u,
             res.lots_created, res.lot_failures, res.lots_deleted, res.streets_placed, res.streets_failed,
             res.demolish_regions,
             ctx_.newLotRefusals != refusedBefore ? " [lots refused, see CreateLot]" : "");
}

void cFC4ZoneDeveloper::SetOptions(bool alternateOrientation, bool placeStreets, bool customZoneSize) {
    Section sec(this);
    if (!sec.ok()) return;
    fc4_zonedev_set_options(core_, alternateOrientation, placeStreets, customZoneSize);
}

bool cFC4ZoneDeveloper::ExistsNetworkOfType(int32_t x, int32_t z, uint32_t networkFlags) {
    Section sec(this);
    if (!sec.ok()) return false;
    return fc4_zonedev_exists_network_of_type(core_, x, z, networkFlags);
}

bool cFC4ZoneDeveloper::IsCellWater(int32_t x, int32_t z) {
    Section sec(this);
    if (!sec.ok()) return false;
    return fc4_zonedev_is_cell_water(core_, x, z);
}

// ---- cISC4LotManager (§8, §11.3) ------------------------------------------------------------------------------
//
// [choice] A thread other than the main one that cannot get the lock gets
// the neutral answer (null, 0, false), except the lot count and the grid
// (snapshots); it is counted as a fallback.

cISC4Lot* cFC4ZoneDeveloper::GetLotAtPoint(cS3DVector3 const& point) {
    Section sec(this);
    if (!sec.ok()) return nullptr;
    const float pos[3] = {point.fX, point.fY, point.fZ};
    return reinterpret_cast<cISC4Lot*>(FromToken(fc4_zonedev_get_lot_at_position(core_, pos)));
}

cISC4Lot* cFC4ZoneDeveloper::GetLot(int32_t cellX, int32_t cellZ, bool originOnly) {
    Section sec(this);
    if (!sec.ok()) return nullptr;
    return reinterpret_cast<cISC4Lot*>(FromToken(fc4_zonedev_get_lot(core_, cellX, cellZ, originOnly)));
}

// Slot 7: clears the list (each element released, each node freed with B),
// then appends with B nodes, one reference each; EAX = the number appended.
uint32_t cFC4ZoneDeveloper::GetLotsInCellRect(fc4::GameList<cISC4Lot*>& lots, int32_t x1, int32_t z1, int32_t x2,
                                              int32_t z2, bool originOnly) {
    Section sec(this);
    if (!sec.ok()) return 0;
    ClearCallerList(ctx_, lots);
    const fc4_cell_rect rect{x1, z1, x2, z2};
    Appender a{&ctx_, lots.head()};
    ctx_.forCaller = true;
    fc4_zonedev_lots_in_cell_rect(core_, &rect, originOnly, &AppendLot, &a);
    ctx_.forCaller = false;
    return a.appended;
}

uint32_t cFC4ZoneDeveloper::GetLotsSurroundingCell(int32_t cellX, int32_t cellZ, fc4::GameList<cISC4Lot*>& lots,
                                                   int32_t flags) {
    Section sec(this);
    if (!sec.ok()) return 0;
    std::vector<uint64_t> existing;
    ListTokens(lots, existing);
    Appender a{&ctx_, lots.head()};
    ctx_.forCaller = true;
    fc4_zonedev_lots_surrounding_cell(core_, cellX, cellZ, static_cast<uint32_t>(flags), existing.data(),
                                      existing.size(), &AppendLot, &a);
    ctx_.forCaller = false;
    return a.appended;
}

uint32_t cFC4ZoneDeveloper::GetLotsSurroundingLot(cISC4Lot const* lot, fc4::GameList<cISC4Lot*>& lots,
                                                  int32_t flags) {
    Section sec(this);
    if (!sec.ok()) return 0;
    std::vector<uint64_t> existing;
    ListTokens(lots, existing);
    Appender a{&ctx_, lots.head()};
    ctx_.forCaller = true;
    fc4_zonedev_lots_surrounding_lot(core_, TokenOf(lot), static_cast<uint32_t>(flags), existing.data(),
                                     existing.size(), &AppendLot, &a);
    ctx_.forCaller = false;
    return a.appended;
}

uint32_t cFC4ZoneDeveloper::GetLotsSurroundingRect(SC4Rect<long> const& rect, fc4::GameList<cISC4Lot*>& lots,
                                                   int32_t flags) {
    Section sec(this);
    if (!sec.ok()) return 0;
    std::vector<uint64_t> existing;
    ListTokens(lots, existing);
    const fc4_cell_rect r = RectOf(rect);
    Appender a{&ctx_, lots.head()};
    ctx_.forCaller = true;
    fc4_zonedev_lots_surrounding_rect(core_, &r, static_cast<uint32_t>(flags), existing.data(), existing.size(),
                                      &AppendLot, &a);
    ctx_.forCaller = false;
    return a.appended;
}

// Slot 11: the original leaves EAX undefined; true (§11.3 [choice]).
bool cFC4ZoneDeveloper::GetVerticesAtEdgeOfLot(SC4Rect<long> const& cells, int32_t flags, SC4Rect<long>& vertices) {
    Section sec(this);
    if (!sec.ok()) return true;
    const fc4_cell_rect r = RectOf(cells);
    fc4_cell_rect out = RectOf(vertices);
    if (fc4_zonedev_vertices_at_edge_of_lot(core_, &r, static_cast<uint32_t>(flags), &out)) PutRect(vertices, out);
    return true;
}

bool cFC4ZoneDeveloper::GetNeighborCellsAtEdgeOfLot(SC4Rect<long> const& cells, int32_t flags, SC4Rect<long>& out) {
    Section sec(this);
    if (!sec.ok()) return false;
    const fc4_cell_rect r = RectOf(cells);
    fc4_cell_rect o{0, 0, -1, -1};
    const bool al = fc4_zonedev_neighbor_cells_at_edge_of_lot(core_, &r, static_cast<uint32_t>(flags), &o);
    PutRect(out, o);
    return al;
}

cISC4Lot* cFC4ZoneDeveloper::GetRandomLot() {
    Section sec(this);
    if (!sec.ok()) return nullptr;
    return reinterpret_cast<cISC4Lot*>(FromToken(fc4_zonedev_random_lot(core_)));
}

cISC4Lot* cFC4ZoneDeveloper::GetOccupantLot(cISC4Occupant* occupant) {
    Section sec(this);
    if (!sec.ok()) return nullptr;
    return reinterpret_cast<cISC4Lot*>(FromToken(fc4_zonedev_occupant_lot(core_, TokenOf(occupant))));
}

// Slot 15: a new C block {first node, the list object}, registered in the
// raw iterator list; its owner unregisters and frees it (FC4SafeLotIterator.h).
intptr_t cFC4ZoneDeveloper::GetSafeLotIterator() {
    Section sec(this);
    if (!sec.ok()) return 0;
    fc4::safelots::Block* b = master_.NewIterator();
    if (b == nullptr) {
        NoteFew(&g_noteIterator, "zonedev: GetSafeLotIterator returned null (no game allocator B / C)");
        return 0;
    }
    ++ctx_.iteratorsIssued;
    return reinterpret_cast<intptr_t>(b);
}

int32_t cFC4ZoneDeveloper::GetLotCount() {
    Section sec(this);
    if (!sec.ok()) return snapLotCount_.load();
    return static_cast<int32_t>(fc4_zonedev_lot_count(core_));
}

float cFC4ZoneDeveloper::GetLotSavvyTerrainVertexAltitude(int32_t x, int32_t z, cISC4Lot* exclude, bool* ok) {
    Section sec(this);
    if (!sec.ok()) {
        if (ok != nullptr) *ok = false;
        return 0.0f;
    }
    bool k = false;
    const float v = fc4_zonedev_lot_savvy_terrain_vertex_altitude(core_, x, z, TokenOf(exclude), &k);
    if (ok != nullptr) *ok = k;
    return v;
}

float cFC4ZoneDeveloper::GetLotSavvyTerrainAltitude(float x, float z, bool* onLot) {
    Section sec(this);
    if (!sec.ok()) {
        if (onLot != nullptr) *onLot = false;
        return 0.0f;
    }
    return fc4_zonedev_lot_savvy_terrain_altitude(core_, x, z, onLot);  // `ok` may be null (§8.7)
}

// Slot 19. [question] (§8.7) The spec does not say that *ok is written: it
// is left untouched.
float cFC4ZoneDeveloper::GetLotSavvyCellAltitude(int32_t cellX, int32_t cellZ, bool* ok) {
    (void)ok;
    Section sec(this);
    if (!sec.ok()) return 0.0f;
    return fc4_zonedev_lot_savvy_cell_altitude(core_, cellX, cellZ);
}

// Slot 20 (§8.4). ZG34 (tract-developer.md §4.15.3): a failed
// CreateLotIfPossible after PerformAggregation's DeleteLots is not safe (the
// original Grow then reaches later candidates' borrowed, freed lots). While
// this host cannot build lots (zonedev_lot_changes off, or the lot pool /
// constructor unavailable), the slot answers 0 for every rectangle *[choice,
// ZG34]*: IsLotSuitable maps 0 to 0 (tract-developer.md §4.8 check 5), so no
// lot is suitable and Grow never reaches aggregation, subdivision or
// redevelopment. The core's own callers (CreateLotIfPossible, parcellization)
// are unchanged: they fail cleanly at CreateLot, after nothing was deleted.
int32_t cFC4ZoneDeveloper::CanCreateLot(int32_t x, int32_t z, int32_t w, int32_t h, bool zoneNotNeeded,
                                        bool waterAllowed, cISC4LotConfiguration* config) {
    Section sec(this);
    if (!sec.ok()) return 0;
    if (!fc4::ZoneDevLotsConstructible(&ctx_)) {
        ++ctx_.canCreateRefusals;
        NoteFew(&g_noteNoLots, "zonedev: WARNING lots cannot be built (zonedev_lot_changes off, or the lot pool / "
                               "constructor not available): CanCreateLot answers 0, so growth leaves existing lots "
                               "alone (ZG34)");
        return 0;
    }
    return fc4_zonedev_can_create_lot(core_, x, z, w, h, zoneNotNeeded, waterAllowed, TokenOf(config));
}

// Slot 21 (the plop path, §8.4.1): the facing `n` and the configuration's
// objects; reads only. The first few answers are logged.
int32_t cFC4ZoneDeveloper::CanCreatePloppedLot(SC4Rect<long> bounds, int32_t n, cISC4LotConfiguration* config) {
    Section sec(this);
    if (!sec.ok() || core_ == nullptr) return 0;
    const fc4_cell_rect r{static_cast<int32_t>(bounds.topLeftX), static_cast<int32_t>(bounds.topLeftY),
                          static_cast<int32_t>(bounds.bottomRightX), static_cast<int32_t>(bounds.bottomRightY)};
    const int32_t code = fc4_zonedev_can_create_plopped_lot(core_, &r, n, TokenOf(config));
    FC4_X87_CHECK("zonedev CanCreatePloppedLot");
    char line[200];
    _snprintf_s(line, sizeof(line), _TRUNCATE, "zonedev: CanCreatePloppedLot(%d,%d..%d,%d, facing %d) = %d", r.x1,
                r.z1, r.x2, r.z2, n, code);
    NoteFew(&g_notePlopped, line);
    return code;
}

// Slot 22 (cfg ignored, §8.5): *out := the lot with ONE reference, or null.
bool cFC4ZoneDeveloper::CreateLotIfPossible(int32_t x, int32_t z, int32_t w, int32_t h, int32_t facing, cISC4Lot*& lot,
                                            bool zoneNotNeeded, bool waterAllowed, cISC4LotConfiguration* config) {
    (void)config;
    lot = nullptr;
    Section sec(this);
    if (!sec.ok()) return false;
    uint64_t out = 0;
    ctx_.forCaller = true;
    const bool r = fc4_zonedev_create_lot_if_possible(core_, x, z, w, h, facing, zoneNotNeeded, waterAllowed, &out);
    ctx_.forCaller = false;
    lot = reinterpret_cast<cISC4Lot*>(FromToken(out));
    return r;
}

// Slot 23 (§8.5): *out := the lot with the caller's reference, or null (no
// lot built: FC4ZoneDevHost.cpp NewLot).
bool cFC4ZoneDeveloper::CreateLot(int32_t x, int32_t z, int32_t w, int32_t h, int32_t facing, cISC4Lot*& lot) {
    lot = nullptr;
    Section sec(this);
    if (!sec.ok()) return false;
    uint64_t out = 0;
    ctx_.forCaller = true;
    const bool r = fc4_zonedev_create_lot(core_, x, z, w, h, facing, &out);
    ctx_.forCaller = false;
    lot = reinterpret_cast<cISC4Lot*>(FromToken(out));
    return r;
}

// Slots 24-26 leave EAX undefined in the original: true (§11.3 [choice]).
bool cFC4ZoneDeveloper::DeleteLot(cISC4Lot* lot) {
    Section sec(this);
    if (!sec.ok()) return true;
    fc4_zonedev_delete_lot(core_, TokenOf(lot));
    return true;
}

bool cFC4ZoneDeveloper::DeleteLots(int32_t x1, int32_t z1, int32_t x2, int32_t z2) {
    Section sec(this);
    if (!sec.ok()) return true;
    fc4_zonedev_delete_lots(core_, x1, z1, x2, z2);
    return true;
}

bool cFC4ZoneDeveloper::GetLotVertexAltitudeAdjustmentRange(int32_t x, int32_t z, float& a, float& b) {
    Section sec(this);
    if (!sec.ok()) return true;
    fc4_zonedev_lot_vertex_altitude_adjustment_range(core_, x, z, &a, &b);
    return true;
}

// Slot 27: no AddRef (§8.7).
cISC4SimGrid<uint16_t>* cFC4ZoneDeveloper::GetMaximumCapacityGrid() {
    Section sec(this);
    if (!sec.ok()) return snapGrid_.load();
    return reinterpret_cast<cISC4SimGrid<uint16_t>*>(FromToken(fc4_zonedev_max_capacity_grid(core_)));
}

// ---- messages (§5.2) ------------------------------------------------------------------------------------------

bool cFC4ZoneDeveloper::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("zonedev DoMessage", pMessage->GetType());  // x87 leftover guard (FC4Fpu.h)
    Section sec(this);
    if (!sec.ok()) return true;  // [choice] a message from another thread that cannot get the lock is dropped
    const uint32_t type = pMessage->GetType();
    uint64_t occupant = 0;
    uint32_t d1 = 0, d3 = 0, d4 = 0, value = 0;
    cIGZMessage2Standard* s = Query<cIGZMessage2Standard>(pMessage, GZIID_cIGZMessage2Standard);
    if (s != nullptr) {
        occupant = TokenOf(s->GetVoid1());
        d1 = static_cast<uint32_t>(s->GetData1());
        d3 = static_cast<uint32_t>(s->GetData3());
        d4 = static_cast<uint32_t>(s->GetData4());
        if (type == kMsgViewValue && s->GetVoid2() != nullptr) value = *static_cast<const uint32_t*>(s->GetVoid2());
    }
    fc4_zonedev_parcel_result out{};
    out.struct_size = sizeof(out);
    fc4_zonedev_do_message(core_, type, occupant, d1, d3, d4, value, &out);
    Drop(s);
    if (type == kMsgInsertOccupant && (out.lots_deleted != 0 || out.lots_created != 0 || out.lot_failures != 0)) {
        fc4::Log("zonedev InsertOccupant: occupant %p data4=%u deleted=%u remnants=%u failed=%u%s",
                 static_cast<void*>(FromToken(occupant)), d4, out.lots_deleted, out.lots_created, out.lot_failures,
                 out.lot_failures != 0 ? " [lots refused, see CreateLot]" : "");
    }
    return true;  // §5: always
}

// ---- persistence (§10.2) --------------------------------------------------------------------------------------

bool cFC4ZoneDeveloper::Write(cIGZOStream& s) {
    FC4_TRACE("ZoneDeveloper::Write");
    Section sec(this);
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_zonedev_write(core_, a.Get());
    fc4::Log("zonedev Write -> %d", ok ? 1 : 0);
    LogState("Write");
    return ok;
}

// Runs before Init: the core sizes nothing from the city here; the master
// list is rebuilt in first-cell order (the mirror follows, master_reset).
bool cFC4ZoneDeveloper::Read(cIGZIStream& s) {
    FC4_TRACE("ZoneDeveloper::Read");
    Section sec(this);
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_zonedev_read(core_, a.Get());
    fc4::Log("zonedev Read -> %d (raw master list %u lots)", ok ? 1 : 0, static_cast<unsigned>(master_.Size()));
    LogState("Read");
    return ok;
}

uint32_t cFC4ZoneDeveloper::GetGZCLSID() { return kFC4_CLSID_cSC4ZoneDeveloper; }

// ---- test seam op 53 ------------------------------------------------------------------------------------------

intptr_t ZoneDevTestSeam(const FC4ZoneDevTestInput* in, void* b) {
    if (in == nullptr) return -1;
    if (in->op == kFC4ZoneDevTestHooks) {
        fc4::ZoneDevSetTestAllocators(in->allocate, in->free);
        if (in->zoneDeveloper == nullptr) return 0;
        cFC4ZoneDeveloper* z = cFC4ZoneDeveloper::FromAny(in->zoneDeveloper);
        if (z == nullptr) return -1;
        z->SetTestHooks(in->host);
        return 1;
    }
    if (in->op == kFC4ZoneDevTestShadow) return fc4::ZoneDevShadowTestRun(in->city, in->julian, in->event, in->host);
    if (in->op == kFC4ZoneDevTestState) {
        cFC4ZoneDeveloper* z = cFC4ZoneDeveloper::FromAny(in->zoneDeveloper);
        auto* out = static_cast<FC4ZoneDevTestState*>(b);
        if (z == nullptr || out == nullptr) return -1;
        z->TestState(*out);
        return 1;
    }
    return -1;
}

void cFC4ZoneDeveloper::TestState(FC4ZoneDevTestState& out) {
    Section sec(this);
    out.masterSize = static_cast<uint32_t>(master_.Size());
    out.iterators = master_.RegisteredIterators();
    out.newLotRefusals = ctx_.newLotRefusals;
    out.heldConfigs = static_cast<uint32_t>(ctx_.heldConfigs.size());
    out.lotsBuilt = ctx_.lotsBuilt;
    out.canCreateRefusals = ctx_.canCreateRefusals;
}
