// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <windows.h>
#include <intrin.h>
#include <cstdint>
#include <unordered_map>
#include <vector>

namespace fc4 {
// PV14 section 5.3. This records genuine selector calls; it never calls a
// requester to obtain an oracle value for the shadow. Replay is a separate
// operation and must validate every argument and the complete event count.
class PropSelectorTrace {
public:
    enum class Purpose { Unknown, Kept, Creation };
    struct Event {
        uintptr_t requester;
        uint32_t depth;
        Purpose purpose;
        int32_t x, z;
        std::vector<uint32_t> candidates;
        uint32_t result;
    };
    using Select = uint32_t (__fastcall*)(void*, void*, const void*, int32_t, int32_t);

    PropSelectorTrace() = default;
    PropSelectorTrace(const PropSelectorTrace&) = delete;
    PropSelectorTrace& operator=(const PropSelectorTrace&) = delete;
    ~PropSelectorTrace() { Finish(); }

    bool Begin(const std::vector<void*>& requesters, uintptr_t imageBase) {
        if (active_ || started_) return false;
        imageBase_ = imageBase;
        valid_ = true;
        events_.clear();
        for (void* requester : requesters) {
            if (!requester) continue;
            auto** table = *static_cast<void***>(requester);
            if (installed_.find(table) != installed_.end()) continue;
            const auto original = reinterpret_cast<Select>(table[6]);
            AcquireSRWLockExclusive(&lock_);
            const auto entry = originals_.find(table);
            const bool free = entry == originals_.end() || !entry->second.installed;
            if (free) originals_[table] = Dispatch{original, true};
            ReleaseSRWLockExclusive(&lock_);
            if (!free || !Patch(table, reinterpret_cast<void*>(&Capture))) {
                valid_ = false;
                if (free) {
                    AcquireSRWLockExclusive(&lock_);
                    originals_[table].installed = false;
                    ReleaseSRWLockExclusive(&lock_);
                }
                Finish();
                return false;
            }
            installed_.emplace(table, original);
        }
        started_ = true;
        active_ = this;
        return true;
    }

    void Finish() {
        if (active_ == this) active_ = nullptr;
        for (const auto& entry : installed_) {
            if (entry.first[6] != reinterpret_cast<void*>(&Capture) ||
                !Patch(entry.first, reinterpret_cast<void*>(entry.second))) {
                valid_ = false;
                // Keep the original dispatch entry if restoration failed.
                continue;
            }
            AcquireSRWLockExclusive(&lock_);
            // A different thread may already have loaded Capture from the
            // vtable but not entered it yet. Retain its original dispatch
            // after restoration so that delayed call still forwards once.
            originals_[entry.first].installed = false;
            ReleaseSRWLockExclusive(&lock_);
        }
        installed_.clear();
        started_ = false;
    }
    bool Valid() const { return valid_; }
    const std::vector<Event>& Events() const { return events_; }

private:
    static bool Patch(void** table, void* value) {
        DWORD previous = 0;
        if (!VirtualProtect(table + 6, sizeof(void*), PAGE_READWRITE, &previous)) return false;
        InterlockedExchangePointer(table + 6, value);
        DWORD ignored = 0;
        // The pointer is already installed even if restoring protection fails.
        VirtualProtect(table + 6, sizeof(void*), previous, &ignored);
        return true;
    }
    static uint32_t __fastcall Capture(void* self, void* edx, const void* vector,
                                       int32_t x, int32_t z) {
        auto** table = *static_cast<void***>(self);
        AcquireSRWLockShared(&lock_);
        const auto found = originals_.find(table);
        const Select original = found == originals_.end() ? nullptr : found->second.original;
        ReleaseSRWLockShared(&lock_);
        if (!original) return 0; // Restoration failure is invalid, never exact.
        PropSelectorTrace* trace = active_; // Only the invoking thread records.
        if (!trace) return original(self, edx, vector, x, z);
        const uintptr_t caller = reinterpret_cast<uintptr_t>(_ReturnAddress());
        const uintptr_t relative = caller - trace->imageBase_ + 0x00400000u;
        const Purpose purpose = relative == 0x006F467Au ? Purpose::Kept
                              : relative == 0x006F1807u ? Purpose::Creation : Purpose::Unknown;
        struct Words { const uint32_t* begin; const uint32_t* end; const uint32_t* storage; };
        const auto* words = static_cast<const Words*>(vector);
        const size_t slot = trace->events_.size();
        Event event{reinterpret_cast<uintptr_t>(self), trace->depth_, purpose, x, z, {}, 0};
        if (!words || reinterpret_cast<uintptr_t>(words->end) < reinterpret_cast<uintptr_t>(words->begin) ||
            reinterpret_cast<uintptr_t>(words->storage) < reinterpret_cast<uintptr_t>(words->end) ||
            (reinterpret_cast<uintptr_t>(words->end) - reinterpret_cast<uintptr_t>(words->begin)) % 4) {
            trace->valid_ = false;
        } else if (words->begin != words->end) {
            event.candidates.assign(words->begin, words->end);
        }
        trace->events_.push_back(std::move(event));
        ++trace->depth_;
        const uint32_t result = original(self, edx, vector, x, z); // genuine call, once
        --trace->depth_;
        trace->events_[slot].result = result; // zero is preserved, not replaced
        return result;
    }

    inline static SRWLOCK lock_ = SRWLOCK_INIT;
    struct Dispatch { Select original; bool installed; };
    inline static std::unordered_map<void**, Dispatch> originals_;
    inline static thread_local PropSelectorTrace* active_ = nullptr;
    std::unordered_map<void**, Select> installed_;
    uintptr_t imageBase_ = 0;
    uint32_t depth_ = 0;
    bool started_ = false, valid_ = false;
    std::vector<Event> events_;
};

// A scoped operation transcript, never a live requester fallback. Reentrant
// selectors require nested replay support; reject them here.
class PropSelectorReplay {
public:
    explicit PropSelectorReplay(const std::vector<PropSelectorTrace::Event>& events) : events_(events) {}
    uint32_t Select(uintptr_t requester, PropSelectorTrace::Purpose purpose,
                    const uint32_t* ids, size_t count, int32_t x, int32_t z) {
        if (cursor_ == events_.size()) { Fail("extra-call"); return 0; }
        const auto& e = events_[cursor_++];
        const char* reason = e.requester != requester ? "requester" : e.depth != 0 ? "depth" :
            e.purpose != purpose ? "purpose" : e.x != x || e.z != z ? "coordinates" :
            e.candidates.size() != count ? "candidate-count" : count && !ids ? "null-candidates" : nullptr;
        if (!reason) for (size_t i=0; i<count; ++i)
            if (ids[i] != e.candidates[i]) { reason = "candidate-order"; break; }
        if (reason) {
            if (!failure_) {
                expected_ = &e;
                actual_ = {requester, 0, purpose, x, z, {}, 0};
                if (count && ids) actual_.candidates.assign(ids, ids+count);
            }
            Fail(reason); return 0;
        }
        ++matched_;
        return e.result;
    }
    bool Complete() const { return valid_ && cursor_ == events_.size(); }
    size_t Matched() const { return matched_; }
    size_t Consumed() const { return cursor_; }
    const char* Failure() const { return failure_ ? failure_ : cursor_ < events_.size() ? "missing-call" : "none"; }
    const PropSelectorTrace::Event* Expected() const {
        return expected_ ? expected_ : cursor_ < events_.size() ? &events_[cursor_] : nullptr;
    }
    const PropSelectorTrace::Event& Actual() const { return actual_; }
private:
    void Fail(const char* reason) { valid_ = false; if (!failure_) failure_ = reason; }
    const std::vector<PropSelectorTrace::Event>& events_;
    size_t cursor_ = 0, matched_ = 0;
    bool valid_ = true;
    const char* failure_ = nullptr;
    const PropSelectorTrace::Event* expected_ = nullptr;
    PropSelectorTrace::Event actual_{};
};
}
