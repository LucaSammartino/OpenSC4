// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "fc4.h"
#include <windows.h>
#include <array>
#include <cstdint>
#include <cstring>
#include <map>
#include <vector>

namespace fc4 {
// PV18: read only semantic fields at completed-call boundaries. The caller
// must identify 1.1.641, relocate the declared ordinary vtable, and protect
// the prop's lifetime. Never apply this layout to an undeclared custom prop.
struct PropEffectSnapshot {
    template<class T> static T At(const void* p, size_t offset) {
        T result;
        std::memcpy(&result, static_cast<const uint8_t*>(p) + offset, sizeof(result));
        return result;
    }
    static bool Capture(const void* prop, const void* ordinary, fc4_prop_effect_state& out) {
        if (!prop || At<const void*>(prop, 0) != ordinary) return false;
        fc4_prop_effect_state s{};
        const auto* range = At<const uint8_t*>(prop, 0x54); // primary P, not C=P+8
        if (range) {
            s.present = 1;
            s.interval = At<uint32_t>(range, 0);
            s.duration = At<uint32_t>(range, 4);
            s.start_day = At<uint32_t>(range, 0x0c);
            s.end_day = At<uint32_t>(range, 0x1c);
        }
        s.timer = At<uint8_t>(prop, 0x60);
        s.chance = At<uint8_t>(prop, 0x62);
        s.state = At<uint8_t>(prop, 0x63);
        out = s;
        return true;
    }
};

struct PropEffectRandom {
    std::array<uint32_t, 624> words{};
    uint32_t index = 0;
    bool Capture(const void* generator) {
        if (!generator) return false;
        const uintptr_t base = reinterpret_cast<uintptr_t>(generator);
        const uintptr_t next = PropEffectSnapshot::At<uint32_t>(generator, 0x9c0);
        const int32_t remaining = PropEffectSnapshot::At<int32_t>(generator, 0x9c4);
        if (next < base || next > base + 0x9c0 || (next - base) % 4 || remaining < 0 || remaining > 624)
            return false;
        const uint32_t k = static_cast<uint32_t>((next - base) / 4);
        if (k + static_cast<uint32_t>(remaining) != 624) return false;
        std::memcpy(words.data(), generator, sizeof(uint32_t) * words.size());
        index = k;
        return true;
    }
    bool operator==(const PropEffectRandom& b) const { return words == b.words && index == b.index; }
};

// Scoped same-thread observation of genuine outgoing prop calls. Child
// setters belong to their parent span, not extra developer host calls.
// Service implementation calls are paused; insertion handlers resume at
// their actual boundary. All intercepted calls forward exactly once.
class PropEffectTrace {
public:
    using Date = uint32_t (*)(const void*);
    struct Event {
        uint64_t prop = 0;
        uint32_t slot = 0, date = 0;
        std::array<uint32_t, 4> args{};
        fc4_prop_effect_state before{}, after{};
        PropEffectRandom randomBefore, randomAfter;
        bool available = false;
        const char* Difference(fc4_prop_effect_result& effects, fc4_prop_effect_state* output = nullptr) const {
            if (!available) return "unavailable";
            auto expected = before;
            auto random = randomBefore;
            if (!fc4_prop_developer_shadow_effect(&expected, slot, args.data(), date,
                random.words.data(), &random.index, &effects)) return "semantic-input";
            if (output) *output = expected;
            if (std::memcmp(&expected, &after, sizeof(expected)) != 0) return "semantic-output";
            return random == randomAfter ? nullptr : "random-output";
        }
        bool Check(fc4_prop_effect_result& effects) const { return Difference(effects) == nullptr; }
    };
    class Recording {
        PropEffectTrace* trace_;
        bool previous_;
    public:
        explicit Recording(bool on) : trace_(active_), previous_(trace_ && trace_->recording_) {
            if (trace_) trace_->recording_ = on;
        }
        ~Recording() { if (trace_) trace_->recording_ = previous_; }
        Recording(const Recording&) = delete;
        Recording& operator=(const Recording&) = delete;
    };
    ~PropEffectTrace() { Finish(); }
    PropEffectTrace() = default;
    PropEffectTrace(const PropEffectTrace&) = delete;
    PropEffectTrace& operator=(const PropEffectTrace&) = delete;
    bool Begin(void** ordinary, const void* random, Date now, Date tick) {
        if (active_ || table_ || !ordinary || !random || !now || !tick) return false;
        table_ = ordinary; random_ = random; now_ = now; tick_ = tick;
        valid_ = true; recording_ = true; depth_ = 0; events_.clear();
        const std::pair<size_t, void*> callbacks[] = {
            {11, reinterpret_cast<void*>(&State)}, {15, reinterpret_cast<void*>(&Range)},
            {16, reinterpret_cast<void*>(&Tick)}, {17, reinterpret_cast<void*>(&Chance)},
            {21, reinterpret_cast<void*>(&Timer)}
        };
        for (const auto& callback : callbacks) {
            const Key key{table_, callback.first};
            AcquireSRWLockExclusive(&lock_);
            const auto found = originals_.find(key);
            const bool free = found == originals_.end() || !found->second.installed;
            if (free) originals_[key] = {table_[callback.first], true};
            ReleaseSRWLockExclusive(&lock_);
            if (!free || !Patch(table_, callback.first, callback.second)) {
                if (free) {
                    AcquireSRWLockExclusive(&lock_);
                    originals_[key].installed = false;
                    ReleaseSRWLockExclusive(&lock_);
                }
                valid_ = false; Finish(); return false;
            }
            installed_[callback.first] = callback.second;
        }
        active_ = this;
        return true;
    }
    void Finish() {
        if (active_ == this) active_ = nullptr;
        for (const auto& entry : installed_) {
            const auto original = Original(table_, entry.first);
            if (table_[entry.first] != entry.second || !Patch(table_, entry.first, original)) {
                valid_ = false; continue;
            }
            AcquireSRWLockExclusive(&lock_);
            originals_[{table_, entry.first}].installed = false;
            ReleaseSRWLockExclusive(&lock_);
        }
        installed_.clear(); table_ = nullptr;
    }
    bool Valid() const { return valid_; }
    const std::vector<Event>& Events() const { return events_; }
private:
    using Key = std::pair<void**, size_t>;
    struct Dispatch { void* original; bool installed; };
    static void* Original(void** table, size_t slot) {
        AcquireSRWLockShared(&lock_);
        const auto it = originals_.find({table, slot});
        void* original = it == originals_.end() ? nullptr : it->second.original;
        ReleaseSRWLockShared(&lock_);
        return original;
    }
    static bool Patch(void** table, size_t slot, void* value) {
        DWORD old = 0;
        if (!VirtualProtect(table + slot, sizeof(void*), PAGE_READWRITE, &old)) return false;
        InterlockedExchangePointer(table + slot, value);
        DWORD ignored = 0;
        VirtualProtect(table + slot, sizeof(void*), old, &ignored);
        return true;
    }
    template<class Invoke> static void Call(void* self, uint32_t slot, std::array<uint32_t,4> args,
                                           const void* date, Invoke invoke) {
        auto** table = *static_cast<void***>(self);
        const auto original = Original(table, slot);
        auto* trace = active_;
        if (!trace || !trace->recording_ || trace->depth_) { invoke(original); return; }
        const size_t i = trace->events_.size();
        Event e;
        e.prop = reinterpret_cast<uintptr_t>(self); e.slot = slot; e.args = args;
        e.date = slot == 16 ? trace->tick_(date) : slot == 15 ? trace->now_(nullptr) : 0;
        e.available = PropEffectSnapshot::Capture(self, trace->table_, e.before) && e.randomBefore.Capture(trace->random_);
        trace->events_.push_back(e);
        ++trace->depth_;
        invoke(original);
        --trace->depth_;
        auto& result = trace->events_[i];
        result.available = PropEffectSnapshot::Capture(self, trace->table_, result.after) &&
            result.randomAfter.Capture(trace->random_) && result.available;
    }
    static void __fastcall State(void* p, void* edx, uint32_t state) {
        Call(p, 11, {state,0,0,0}, nullptr, [&](void* f) {
            reinterpret_cast<void (__fastcall*)(void*,void*,uint32_t)>(f)(p,edx,state);
        });
    }
    static void __fastcall Chance(void* p, void* edx, uint8_t chance) {
        Call(p, 17, {chance,0,0,0}, nullptr, [&](void* f) {
            reinterpret_cast<void (__fastcall*)(void*,void*,uint8_t)>(f)(p,edx,chance);
        });
    }
    static void __fastcall Timer(void* p, void* edx, uint32_t mask, bool on) {
        Call(p, 21, {mask,on?1u:0u,0,0}, nullptr, [&](void* f) {
            reinterpret_cast<void (__fastcall*)(void*,void*,uint32_t,bool)>(f)(p,edx,mask,on);
        });
    }
    static void __fastcall Range(void* p, void* edx, uint32_t i, uint32_t l, uint8_t m, uint8_t d) {
        Call(p, 15, {i,l,m,d}, nullptr, [&](void* f) {
            reinterpret_cast<void (__fastcall*)(void*,void*,uint32_t,uint32_t,uint8_t,uint8_t)>(f)(p,edx,i,l,m,d);
        });
    }
    static void __fastcall Tick(void* p, void* edx, const void* date) {
        Call(p, 16, {}, date, [&](void* f) {
            reinterpret_cast<void (__fastcall*)(void*,void*,const void*)>(f)(p,edx,date);
        });
    }
    inline static SRWLOCK lock_ = SRWLOCK_INIT;
    inline static std::map<Key, Dispatch> originals_;
    inline static thread_local PropEffectTrace* active_ = nullptr;
    void** table_ = nullptr;
    const void* random_ = nullptr;
    Date now_ = nullptr, tick_ = nullptr;
    std::map<size_t, void*> installed_;
    std::vector<Event> events_;
    uint32_t depth_ = 0;
    bool valid_ = false, recording_ = true;
};

class PropEffectReplay {
public:
    explicit PropEffectReplay(const PropEffectTrace& trace) : trace_(trace) {}
    void Call(uint64_t prop, uint32_t slot, std::array<uint32_t,4> args = {}, uint32_t date = 0) {
        if (cursor_ == trace_.Events().size()) { Fail("extra-call", nullptr); return; }
        const auto& e = trace_.Events()[cursor_++];
        fc4_prop_effect_result effects{};
        const char* reason = prop != e.prop ? "receiver" : slot != e.slot ? "slot" :
            args != e.args ? "arguments" : date != e.date ? "date" : e.Difference(effects);
        if (reason) Fail(reason, &e);
        else { ++matched_; draws_ += effects.random_drawn; changes_ += effects.state_changed; }
    }
    bool Complete() const { return valid_ && trace_.Valid() && cursor_ == trace_.Events().size(); }
    size_t Consumed() const { return cursor_; }
    size_t Matched() const { return matched_; }
    size_t Draws() const { return draws_; }
    size_t Changes() const { return changes_; }
    const char* Failure() const { return failure_ ? failure_ : cursor_ < trace_.Events().size() ? "missing-call" : "capture"; }
    const PropEffectTrace::Event* Expected() const {
        return expected_ ? expected_ : cursor_ < trace_.Events().size() ? &trace_.Events()[cursor_] : nullptr;
    }
private:
    void Fail(const char* reason, const PropEffectTrace::Event* e) {
        valid_ = false;
        if (!failure_) { failure_ = reason; expected_ = e; }
    }
    const PropEffectTrace& trace_;
    const char* failure_ = nullptr;
    const PropEffectTrace::Event* expected_ = nullptr;
    size_t cursor_ = 0, matched_ = 0, draws_ = 0, changes_ = 0;
    bool valid_ = true;
};
}
