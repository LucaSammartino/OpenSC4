// SPDX-License-Identifier: GPL-3.0-or-later
// lot-records.md 14.1 (LR12): master lots only, no Init, recursion or mutations.
#include "FC4LotRecordShadow.h"
#include "FC4LotRecordObservation.h"
#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4SafeLotIterator.h"
#include "FC4WriteCapture.h"
#include "compat/FC4Compat.h"
#include "cIGZSerializable.h"
#include "cISC4City.h"
#include <algorithm>
#include <vector>

extern "C" int32_t fc4_lotrec_verify(const uint8_t*, uint32_t, uint8_t*, uint32_t);
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
            const int32_t n = fc4_lotrec_verify(record.data(), static_cast<uint32_t>(record.size()),
                encoded.data(), static_cast<uint32_t>(encoded.size()));
            const bool equal = n >= 0 && static_cast<size_t>(n) == record.size() && encoded == record;
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
}
void LotRecordShadowObserve(cISC4City* city, int32_t julian, bool loaded) {
    static const bool enabled = SettingInt("lotrec_shadow", "FC4_LOTREC_SHADOW", 0) != 0;
    if (!enabled) return;
    Sample sample{city};
    if (!SehGuard("LR12 master lot record shadow", &Run, &sample)) ++sample.observation.errors;
    // The block is non-COM: unregister and C-free on normal and early exits.
    if (sample.block && (!SehGuard("LR12 iterator cleanup", &Sample::ReleaseIterator, &sample) || sample.removed < 0))
        ++sample.observation.errors;
    const auto& s = sample.observation;
    Log("lotrec-shadow: phase=%s julian=%d expected=%u visited=%u captures=%u exact=%u/%u mismatches=%u differences=%u errors=%u empty=%u bytes=%llu mode=record-only scope=master-lots excluded=off-list,units,occupants,unspecified-props",
        loaded ? "load" : "month", julian, s.expected, s.visited, s.captures, s.exact, s.expected,
        s.Mismatches(), s.differences, s.errors, s.expected == 0 && s.Mismatches() == 0 ? 1u : 0u, sample.bytes);
}
}
