// SPDX-License-Identifier: GPL-3.0-or-later
// lot-records.md 14.1/14.2: scoped records only, no Init, recursion or mutations.
#include "FC4LotRecordShadow.h"
#include "FC4LotRecordObservation.h"
#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4SafeLotIterator.h"
#include "FC4WriteCapture.h"
#include "FC4Interfaces.h"
#include "compat/FC4Compat.h"
#include "cIGZSerializable.h"
#include "cISC4City.h"
#include "cIGZAllocatorService.h"
#include "GZServPtrs.h"
#include "fc4.h"
#include <algorithm>
#include <vector>

extern "C" int32_t fc4_lotrec_verify(const uint8_t*, uint32_t, uint8_t*, uint32_t, uint32_t*);
extern "C" int32_t fc4_lotowner_verify(uint32_t, const uint8_t*, uint32_t, uint8_t*, uint32_t);
namespace fc4 { namespace {
template<class R, class... A> R Call(void* p, unsigned slot, A... args) {
    using F = R (__thiscall*)(void*, A...);
    return reinterpret_cast<F>((*static_cast<void***>(p))[slot])(p, args...);
}
struct Sample {
    cISC4City* city;
    LotRecordObservation observation;
    safelots::Block* block = nullptr;
    safelots::FreeFn freeNode = nullptr, freeBlock = nullptr;
    int removed = -1;
    // LR12 uses the native encoder shared with automata, independent of Rust.
    WriteCapture capture{false};
    cIGZUnknown* lot = nullptr;
    cIGZSerializable* serializable = nullptr;
    bool captured = false;
    uint64_t bytes = 0;
    uint64_t trips = 0, exactTrips = 0;
    static void Capture(void* p) {
        auto& s = *static_cast<Sample*>(p);
        void* view = nullptr;
        if (!s.lot->QueryInterface(0xE4FDA3D4, &view) || !view) return;
        s.serializable = static_cast<cIGZSerializable*>(view);
        FC4_ACQUIRED(s.serializable, "QueryInterface");
        s.capture.BeginRecord();
        // bool consumes AL only, including false with a nonzero upper EAX.
        s.captured = s.serializable->Write(s.capture) && s.capture.GetError() == 0;
    }
    static void ReleaseIterator(void* p) {
        auto& s = *static_cast<Sample*>(p);
        s.removed = safelots::Release(s.block, s.freeNode, s.freeBlock, 65536);
    }
    void Run() {
        if (!city || !compat::IdentifiedVersion()) { ++observation.errors; return; }
        const uintptr_t freeVA = compat::DirectCall(compat::Direct::GlobalHeapFree);
        compat::StlNodeFreeFn nodeFree = nullptr;
        if (!freeVA || !compat::StlNodeAllocator(nullptr, &nodeFree)) { ++observation.errors; return; }
        freeNode = reinterpret_cast<safelots::FreeFn>(nodeFree);
        freeBlock = reinterpret_cast<safelots::FreeFn>(freeVA);
        void* manager = Call<void*>(city, 33);
        if (!manager) { ++observation.errors; return; }
        observation.expected = Call<uint32_t>(manager, 16);
        // A bound rejects corrupt counts explicitly; no failing lot is dropped.
        if (observation.expected > 1000000) { ++observation.errors; return; }
        std::vector<void*> lots(observation.expected);
        block = Call<safelots::Block*>(manager, 15);
        uint32_t count = 0;
        const auto status = safelots::CopyLots(block, lots.data(), observation.expected, &count);
        if (status != safelots::Status::Ok) {
            ++observation.errors;
            Log("lotrec-shadow: error reason=iterator status=%s", safelots::StatusName(status));
            return;
        }
        for (uint32_t i = 0; i < count; ++i) {
            if (!observation.Visit(lots[i])) continue;
            lot = static_cast<cIGZUnknown*>(lots[i]);
            serializable = nullptr;
            captured = false;
            const bool safe = SehGuard("LR12 lot Write", &Capture, this);
            if (serializable) { FC4_RELEASE(serializable); serializable = nullptr; }
            if (!safe || !captured) {
                observation.Record(false, false, false);
                Log("lotrec-shadow: error index=%u reason=capture", i);
                continue;
            }
            const auto& record = capture.Bytes();
            bytes += record.size();
            std::vector<uint8_t> encoded(record.size());
            uint32_t tripCount = 0;
            const int32_t n = fc4_lotrec_verify(record.data(), static_cast<uint32_t>(record.size()),
                encoded.data(), static_cast<uint32_t>(encoded.size()), &tripCount);
            const bool equal = n >= 0 && static_cast<size_t>(n) == record.size() && encoded == record;
            trips += tripCount;
            // Conservative evidence: credit trips only when their entire containing
            // lot is byte-exact. A rejected lot remains an inventory error below.
            if (equal) exactTrips += tripCount;
            observation.Record(true, n >= 0, equal);
            if (!equal && observation.errors + observation.differences <= 8) {
                size_t offset = 0;
                while (offset < record.size() && offset < encoded.size() && record[offset] == encoded[offset]) ++offset;
                Log("lotrec-shadow: difference index=%u bytes=%u encoded=%d first=%u native=%u core=%u",
                    i, static_cast<uint32_t>(record.size()), n, static_cast<uint32_t>(offset),
                    offset < record.size() ? record[offset] : 0, offset < encoded.size() ? encoded[offset] : 0);
            }
        }
        observation.Finish(status == safelots::Status::Ok && count == observation.expected,
            Call<uint32_t>(manager, 16));
    }
};
void Run(void* p) { static_cast<Sample*>(p)->Run(); }

// LR15: the native list holder is one pointer; allocator A owns its sentinel.
// Keep it outside the guarded query so early exits still reach slot-48 cleanup.
struct OwnerSample {
    struct Node { Node* next; Node* prev; cIGZUnknown* value; };
    struct List { Node* root = nullptr; } list;
    cISC4City* city;
    const char* label;
    uint32_t classId, type;
    LotRecordObservation observation, masters;
    WriteCapture capture{false};
    uint64_t bytes = 0;
    void* manager = nullptr;
    safelots::Block* block = nullptr;
    safelots::FreeFn freeNode = nullptr, freeBlock = nullptr;
    cIGZUnknown* owner = nullptr;
    cIGZSerializable* serializable = nullptr;
    bool captured = false, complete = false;
    int removed = -1;

    static void Capture(void* p) {
        auto& s = *static_cast<OwnerSample*>(p);
        void* view = nullptr;
        if (!s.owner->QueryInterface(0xE4FDA3D4, &view) || !view) return;
        s.serializable = static_cast<cIGZSerializable*>(view);
        FC4_ACQUIRED(s.serializable, "QueryInterface");
        if (s.serializable->GetGZCLSID() != s.classId) return;
        s.capture.BeginRecord();
        // Unit Write returns AL; these two occupant writers return EAX=0/1.
        const bool ok = s.type == 0 ? s.serializable->Write(s.capture) :
            Call<uint32_t>(s.serializable, 3, static_cast<cIGZOStream*>(&s.capture)) != 0;
        s.captured = ok && s.capture.GetError() == 0;
    }
    void Compare(cIGZUnknown* value) {
        owner = value; serializable = nullptr; captured = false;
        const bool safe = SehGuard("LR15 owner Write", &Capture, this);
        if (serializable) { FC4_RELEASE(serializable); serializable = nullptr; }
        if (!safe || !captured) { observation.Record(false, false, false); return; }
        const auto& record = capture.Bytes();
        bytes += record.size();
        std::vector<uint8_t> encoded(record.size());
        const int32_t n = fc4_lotowner_verify(classId, record.data(), static_cast<uint32_t>(record.size()),
            encoded.data(), static_cast<uint32_t>(encoded.size()));
        const bool equal = n >= 0 && static_cast<size_t>(n) == record.size() && encoded == record;
        observation.Record(true, n >= 0, equal);
        if (!equal && observation.Mismatches() <= 8) {
            size_t offset = 0;
            while (offset < record.size() && record[offset] == encoded[offset]) ++offset;
            Log("%s-shadow: difference index=%u bytes=%u encoded=%d first=%u native=%u core=%u",
                label, observation.visited - 1, static_cast<uint32_t>(record.size()), n,
                static_cast<uint32_t>(offset), offset < record.size() ? record[offset] : 0,
                offset < encoded.size() ? encoded[offset] : 0);
        }
    }
    void Units() {
        const uintptr_t freeVA = compat::DirectCall(compat::Direct::GlobalHeapFree);
        compat::StlNodeFreeFn nodeFree = nullptr;
        if (!freeVA || !compat::StlNodeAllocator(nullptr, &nodeFree)) { ++observation.errors; return; }
        freeNode = reinterpret_cast<safelots::FreeFn>(nodeFree);
        freeBlock = reinterpret_cast<safelots::FreeFn>(freeVA);
        manager = Call<void*>(city, 33); // borrowed
        if (!manager) { ++observation.errors; return; }
        masters.expected = Call<uint32_t>(manager, 16);
        if (masters.expected > 1000000) { ++observation.errors; return; }
        std::vector<void*> lots(masters.expected);
        block = Call<safelots::Block*>(manager, 15);
        uint32_t count = 0;
        const auto status = safelots::CopyLots(block, lots.data(), masters.expected, &count);
        if (status != safelots::Status::Ok) { ++observation.errors; return; }
        for (uint32_t i = 0; i < count; ++i) {
            if (!masters.Visit(lots[i])) continue;
            auto* unit = Call<cIGZUnknown*>(lots[i], 25); // borrowed, never Release
            // Shared GetUnit edges are expected, unlike duplicate master lots.
            if (!unit || observation.seen.count(unit)) continue;
            ++observation.expected;
            if (observation.Visit(unit)) Compare(unit);
        }
        masters.Finish(count == masters.expected, Call<uint32_t>(manager, 16));
        observation.errors += masters.Mismatches();
        complete = masters.Mismatches() == 0;
    }
    void Occupants() {
        manager = Call<void*>(city, 34); // borrowed
        if (!manager) { ++observation.errors; return; }
        cIGZAllocatorServicePtr allocator;
        list.root = static_cast<Node*>(allocator->Allocate(sizeof(Node)));
        if (!list.root) { ++observation.errors; return; }
        list.root->next = list.root->prev = list.root;
        list.root->value = nullptr;
        const int32_t min[2] = {0, 0}, max[2] = {191, 191};
        // Explicit full world grid, inclusive, unlimited; true/empty is success.
        const bool queried = Call<bool>(manager, 39, &list, min, max, type, int32_t(-1));
        complete = ObserveRegisteredList(list.root, queried, observation,
            [this](cIGZUnknown* value) { Compare(value); });
    }
    static void Run(void* p) {
        auto& s = *static_cast<OwnerSample*>(p);
        if (!s.city || !compat::IdentifiedVersion()) { ++s.observation.errors; return; }
        if (s.type == 0) s.Units(); else s.Occupants();
    }
    static void Cleanup(void* p) {
        auto& s = *static_cast<OwnerSample*>(p);
        if (s.block) {
            s.removed = safelots::Release(s.block, s.freeNode, s.freeBlock, 65536);
            if (s.removed < 0) ++s.observation.errors;
        }
        if (s.list.root) {
            Call<void>(s.manager, 48, &s.list); // releases element references and nodes
            cIGZAllocatorServicePtr allocator;
            allocator->Deallocate(s.list.root);
            s.list.root = nullptr;
        }
    }
    void Observe(const fc4_date& date, bool loaded) {
        if (!SehGuard("LR15 scoped owner inventory", &Run, this)) ++observation.errors;
        if (!SehGuard("LR15 owner inventory cleanup", &Cleanup, this)) ++observation.errors;
        const auto& s = observation;
        Log("%s shadow %04d-%02d-%02d %s: complete=%u expected=%u visited=%u captures=%u exact=%u/%u differences=%u errors=%u empty=%u bytes=%llu masters=%u/%u mode=record-only scope=%s; mismatches=%u",
            label, date.year, date.month, date.day_of_month, loaded ? "load" : "month",
            complete ? 1u : 0u, s.expected, s.visited, s.captures, s.exact, s.expected,
            s.differences, s.errors, complete && s.expected == 0 && s.Mismatches() == 0 ? 1u : 0u,
            bytes, masters.visited, masters.expected,
            type == 0 ? "master-rooted-units" : "full-grid-registered-occupants", s.Mismatches());
    }
};
}
void LotRecordShadowObserve(cISC4City* city, int32_t julian, bool loaded) {
    static const bool enabled = SettingInt("lotrec_shadow", "FC4_LOTREC_SHADOW", 0) != 0;
    static const bool tripEnabled = SettingInt("lottrip_shadow", "FC4_LOTTRIP_SHADOW", 0) != 0;
    static const bool unitEnabled = SettingInt("lotunit_shadow", "FC4_LOTUNIT_SHADOW", 0) != 0;
    static const bool textureEnabled = SettingInt("lotbasetexture_shadow", "FC4_LOTBASETEXTURE_SHADOW", 0) != 0;
    static const bool wallEnabled = SettingInt("lotwall_shadow", "FC4_LOTWALL_SHADOW", 0) != 0;
    if (!enabled && !tripEnabled && !unitEnabled && !textureEnabled && !wallEnabled) return;
    Sample sample{city};
    if ((enabled || tripEnabled) && !SehGuard("LR12 master lot record shadow", &Run, &sample)) ++sample.observation.errors;
    // The block is non-COM: unregister and C-free on normal and early exits.
    if (sample.block && (!SehGuard("LR12 iterator cleanup", &Sample::ReleaseIterator, &sample) || sample.removed < 0))
        ++sample.observation.errors;
    const auto& s = sample.observation;
    if (loaded && city) {
        auto* sim = AsGame(city->GetSimulator());  // borrowed
        if (sim) julian = sim->GetSimDateNumber();
    }
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    if (unitEnabled) { OwnerSample s{{}, city, "lotunit", 0x4A232DA8, 0}; s.Observe(date, loaded); }
    if (textureEnabled) { OwnerSample s{{}, city, "lotbasetexture", 0xC97F987C, 0x497F6D9D}; s.Observe(date, loaded); }
    if (wallEnabled) { OwnerSample s{{}, city, "lotwall", 0x49C05C9F, 0x895D1169}; s.Observe(date, loaded); }
    if (enabled) Log("lotrec shadow %04d-%02d-%02d %s: julian=%d expected=%u visited=%u captures=%u exact=%u/%u differences=%u errors=%u empty=%u bytes=%llu mode=record-only scope=master-lots excluded=off-list,units,occupants,unspecified-props; mismatches=%u",
        date.year, date.month, date.day_of_month, loaded ? "load" : "month", julian,
        s.expected, s.visited, s.captures, s.exact, s.expected,
        s.differences, s.errors, s.expected == 0 && s.Mismatches() == 0 ? 1u : 0u, sample.bytes, s.Mismatches());
    if (tripEnabled) Log("lottrip shadow %04d-%02d-%02d %s: owner=cSC4TripData lots=%u/%u exact=%llu/%llu empty=%u mode=record-only scope=master-lot-inline-trips excluded=off-list-trips; mismatches=%u",
        date.year, date.month, date.day_of_month, loaded ? "load" : "month",
        s.exact, s.expected, sample.exactTrips, sample.trips,
        sample.trips == 0 && s.Mismatches() == 0 ? 1u : 0u, s.Mismatches());
}
}
