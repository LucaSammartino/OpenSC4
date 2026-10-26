// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "FC4PropDevEffects.h"
#include <utility>

namespace fc4 {
// prop-developer 4.4/PV18: C is the service-return occupant, P the separately
// established primary prop. Observe only the developer's post-service span;
// observer holds and delegated holder internals do not enter this transcript.
class PropTagTrace {
public:
    enum class Kind { Hold, Holder, Lookup, Add, Value, Assign, Release };
    struct Event {
        Kind kind;
        uintptr_t receiver = 0, result = 0;
        uint32_t argument = 0, flag = 0;
        uint32_t refsBefore = 0, refsAfter = 0;
        uint32_t value = 0;
        bool readable = true;
        size_t nestedBegin = 0, nestedEnd = 0;
    };
    struct Span {
        uintptr_t occupant = 0, prop = 0;
        bool ordinary = false;
        std::vector<Event> events;
    };
    using ReadValue = bool (*)(const void*, uint32_t*);
    using PropertyId = uint32_t (*)(const void*);
    using NestedCount = size_t (*)();
    class Recording {
        PropTagTrace* trace_;
        bool previous_;
    public:
        explicit Recording(bool on) : trace_(active_), previous_(trace_ && trace_->recording_) {
            if (trace_) trace_->recording_ = on;
        }
        ~Recording() { if (trace_) trace_->recording_ = previous_; }
        Recording(const Recording&) = delete;
        Recording& operator=(const Recording&) = delete;
    };
    ~PropTagTrace() { Finish(); }
    bool Begin(void** ordinaryProp, void** ordinaryOccupant, ReadValue read, PropertyId id, NestedCount count) {
        if (active_ || !installed_.empty() || !ordinaryProp || !ordinaryOccupant || !read || !id || !count)
            return false;
        propTable_ = ordinaryProp; occupantTable_ = ordinaryOccupant;
        read_ = read; id_ = id; count_ = count; valid_ = true; depth_ = 0; recording_ = true;
        spans_.clear(); owners_.clear(); active_ = this;
        return true;
    }
    void Watch(void* occupant, uint64_t prop) {
        if (active_ != this) { valid_ = false; return; }
        Span s;
        s.occupant = reinterpret_cast<uintptr_t>(occupant); s.prop = static_cast<uintptr_t>(prop);
        s.ordinary = s.prop && s.occupant == s.prop + 8 &&
            *reinterpret_cast<void***>(s.prop) == propTable_ &&
            *reinterpret_cast<void***>(s.occupant) == occupantTable_;
        const size_t index = spans_.size();
        spans_.push_back(s);
        if (!s.ordinary) { valid_ = false; return; }
        owners_[s.occupant] = index;
        Install(occupant, 1, reinterpret_cast<void*>(&Hold));
        Install(occupant, 2, reinterpret_cast<void*>(&Release));
        Install(occupant, 6, reinterpret_cast<void*>(&Holder));
    }
    void Finish() {
        if (active_ == this) active_ = nullptr;
        for (const auto& entry : installed_)
            if (!Patch(entry.first.first, entry.first.second, entry.second)) valid_ = false;
        installed_.clear();
    }
    bool Valid() const { return valid_; }
    const std::vector<Span>& Spans() const { return spans_; }
private:
    using Key = std::pair<void**, size_t>;
    static void* Original(void* self, size_t slot) {
        AcquireSRWLockShared(&lock_);
        const auto it = originals_.find({*static_cast<void***>(self), slot});
        void* result = it == originals_.end() ? nullptr : it->second;
        ReleaseSRWLockShared(&lock_);
        return result;
    }
    static bool Patch(void** table, size_t slot, void* value) {
        DWORD old = 0;
        if (!VirtualProtect(table + slot, sizeof(void*), PAGE_READWRITE, &old)) return false;
        InterlockedExchangePointer(table + slot, value);
        DWORD ignored = 0;
        VirtualProtect(table + slot, sizeof(void*), old, &ignored);
        return true;
    }
    void Install(void* self, size_t slot, void* callback) {
        if (!self) { valid_ = false; return; }
        const Key key{*static_cast<void***>(self), slot};
        if (installed_.count(key)) return;
        void* original = key.first[slot];
        AcquireSRWLockExclusive(&lock_);
        originals_[key] = original;
        ReleaseSRWLockExclusive(&lock_);
        if (!Patch(key.first, slot, callback)) { valid_ = false; return; }
        installed_[key] = original;
    }
    template<class Invoke> static uintptr_t Call(void* self, Kind kind, uint32_t arg, uint32_t flag,
                                                Invoke invoke) {
        auto* t = active_;
        if (!t || !t->recording_ || t->depth_) return invoke();
        const auto owner = t->owners_.find(reinterpret_cast<uintptr_t>(self));
        if (owner == t->owners_.end()) return invoke();
        const size_t span = owner->second;
        const size_t index = t->spans_[span].events.size();
        Event event;
        event.kind = kind; event.receiver = reinterpret_cast<uintptr_t>(self);
        event.argument = arg; event.flag = flag; event.nestedBegin = t->count_();
        const bool ref = kind == Kind::Hold || kind == Kind::Release;
        const auto* prop = reinterpret_cast<const void*>(t->spans_[span].prop);
        if (ref) event.refsBefore = PropEffectSnapshot::At<uint32_t>(prop, 0x14);
        t->spans_[span].events.push_back(event);
        ++t->depth_;
        const uintptr_t result = invoke(); // genuine call, once, including its this adjustment
        --t->depth_;
        auto& out = t->spans_[span].events[index];
        out.result = result; out.nestedEnd = t->count_();
        if (ref) out.refsAfter = PropEffectSnapshot::At<uint32_t>(prop, 0x14);
        if (kind == Kind::Assign) out.readable = t->read_(self, &out.value);
        if (kind == Kind::Add) out.value = t->id_(reinterpret_cast<void*>(static_cast<uintptr_t>(arg)));
        // Ordinary AsPropertyHolder is the declared P+0x10 subobject. A
        // custom result needs a separate contract before its table is patched.
        if (kind == Kind::Holder && result != t->spans_[span].prop + 0x10) {
            t->valid_ = false;
            return result;
        }
        if (result && (kind == Kind::Holder || kind == Kind::Lookup || kind == Kind::Value)) {
            void* child = reinterpret_cast<void*>(result);
            t->owners_[result] = span;
            if (kind == Kind::Holder) {
                t->Install(child, 9, reinterpret_cast<void*>(&Lookup));
                t->Install(child, 15, reinterpret_cast<void*>(&Add));
            } else if (kind == Kind::Lookup) t->Install(child, 5, reinterpret_cast<void*>(&Value));
            else t->Install(child, 26, reinterpret_cast<void*>(&Assign));
        }
        if (kind == Kind::Add && arg) {
            auto* property = reinterpret_cast<void*>(static_cast<uintptr_t>(arg));
            t->owners_[arg] = span;
            t->Install(property, 5, reinterpret_cast<void*>(&Value));
        }
        if (kind == Kind::Release) {
            for (auto it = t->owners_.begin(); it != t->owners_.end();) {
                if (it->second == span) it = t->owners_.erase(it); else ++it;
            }
        }
        return result;
    }
    static uint32_t __fastcall Hold(void* p, void* edx) {
        auto f = reinterpret_cast<uint32_t (__fastcall*)(void*,void*)>(Original(p, 1));
        return static_cast<uint32_t>(Call(p, Kind::Hold, 0, 0, [&] { return f(p,edx); }));
    }
    static uint32_t __fastcall Release(void* p, void* edx) {
        auto f = reinterpret_cast<uint32_t (__fastcall*)(void*,void*)>(Original(p, 2));
        return static_cast<uint32_t>(Call(p, Kind::Release, 0, 0, [&] { return f(p,edx); }));
    }
    static void* __fastcall Holder(void* p, void* edx) {
        auto f = reinterpret_cast<void* (__fastcall*)(void*,void*)>(Original(p, 6));
        return reinterpret_cast<void*>(Call(p, Kind::Holder, 0, 0, [&] { return reinterpret_cast<uintptr_t>(f(p,edx)); }));
    }
    static void* __fastcall Lookup(void* p, void* edx, uint32_t id) {
        auto f = reinterpret_cast<void* (__fastcall*)(void*,void*,uint32_t)>(Original(p, 9));
        return reinterpret_cast<void*>(Call(p, Kind::Lookup, id, 0, [&] { return reinterpret_cast<uintptr_t>(f(p,edx,id)); }));
    }
    static bool __fastcall Add(void* p, void* edx, void* property, bool flag) {
        auto f = reinterpret_cast<bool (__fastcall*)(void*,void*,void*,bool)>(Original(p, 15));
        return Call(p, Kind::Add, static_cast<uint32_t>(reinterpret_cast<uintptr_t>(property)), flag ? 1u : 0u,
            [&] { return f(p,edx,property,flag) ? 1u : 0u; }) != 0;
    }
    static void* __fastcall Value(void* p, void* edx) {
        auto f = reinterpret_cast<void* (__fastcall*)(void*,void*)>(Original(p, 5));
        return reinterpret_cast<void*>(Call(p, Kind::Value, 0, 0, [&] { return reinterpret_cast<uintptr_t>(f(p,edx)); }));
    }
    static void __fastcall Assign(void* p, void* edx, uint32_t value) {
        auto f = reinterpret_cast<void (__fastcall*)(void*,void*,uint32_t)>(Original(p, 26));
        Call(p, Kind::Assign, value, 0, [&] { f(p,edx,value); return uintptr_t{0}; });
    }
    inline static SRWLOCK lock_ = SRWLOCK_INIT;
    inline static std::map<Key, void*> originals_;
    inline static thread_local PropTagTrace* active_ = nullptr;
    std::map<Key, void*> installed_;
    std::map<uintptr_t, size_t> owners_;
    std::vector<Span> spans_;
    void** propTable_ = nullptr;
    void** occupantTable_ = nullptr;
    ReadValue read_ = nullptr;
    PropertyId id_ = nullptr;
    NestedCount count_ = nullptr;
    uint32_t depth_ = 0;
    bool valid_ = false, recording_ = true;
};

// The expected sequence comes from 4.4, not a replay of opaque effects.
// Lookup/property/holder identities are delegated inputs, but their uses,
// arguments, ordering, reference deltas and the assigned output are checked.
class PropTagReplay {
public:
    explicit PropTagReplay(const PropTagTrace& trace) : trace_(trace) {}
    template<class Replay> bool Call(uint64_t prop, uint32_t requester, size_t& nested, Replay replay) {
        if (cursor_ == trace_.Spans().size()) return Fail("extra-tag");
        const auto& s = trace_.Spans()[cursor_++];
        if (!s.ordinary || s.prop != prop) return Fail("tag-receiver");
        size_t i = 0;
        auto next = [&](PropTagTrace::Kind kind, uintptr_t receiver) -> const PropTagTrace::Event* {
            if (i == s.events.size()) { Fail("missing-tag-call"); return nullptr; }
            const auto& e = s.events[i++];
            if (e.kind != kind || e.receiver != receiver) { Fail("tag-call-order"); return nullptr; }
            if (e.nestedBegin != nested || e.nestedEnd < e.nestedBegin || !replay(e.nestedEnd) || nested != e.nestedEnd) {
                Fail("tag-nested-boundary"); return nullptr;
            }
            return &e;
        };
        using K = PropTagTrace::Kind;
        const auto* hold = next(K::Hold, s.occupant);
        if (!hold || hold->refsAfter != hold->refsBefore + 1 || hold->result != hold->refsAfter)
            return Fail("tag-hold");
        const auto* holder = next(K::Holder, s.occupant);
        if (!holder || !holder->result) return Fail("tag-holder");
        const auto* lookup = next(K::Lookup, holder->result);
        if (!lookup || lookup->argument != 0x89A1C16C) return Fail("tag-key");
        uintptr_t property = lookup->result;
        if (!property) {
            const auto* add = next(K::Add, holder->result);
            if (!add || !add->argument || add->flag || add->value != 0x89A1C16C) return Fail("tag-add");
            property = add->argument; // Add's status is expressly ignored, including false.
        }
        const auto* value = next(K::Value, property);
        if (!value || !value->result) return Fail("tag-variant");
        const auto* assign = next(K::Assign, value->result);
        if (!assign || assign->argument != requester || !assign->readable || assign->value != requester)
            return Fail("tag-assignment");
        const auto* release = next(K::Release, s.occupant);
        if (!release || !release->refsBefore || release->refsAfter != release->refsBefore - 1 ||
            release->result != release->refsAfter) return Fail("tag-release");
        if (i != s.events.size()) return Fail("extra-tag-call");
        ++matched_;
        return true;
    }
    bool Complete() const { return valid_ && trace_.Valid() && cursor_ == trace_.Spans().size(); }
    size_t Matched() const { return matched_; }
    const char* Failure() const { return failure_; }
private:
    bool Fail(const char* reason) { if (valid_) failure_ = reason; valid_ = false; return false; }
    const PropTagTrace& trace_;
    size_t cursor_ = 0, matched_ = 0;
    bool valid_ = true;
    const char* failure_ = "none";
};
} // namespace fc4
