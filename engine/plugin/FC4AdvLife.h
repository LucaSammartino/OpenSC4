// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.
//
// Lifetime of a GZCOM front over a core advisor object (advisors.md §12.2-§12.4).
// The core keeps the front's token without a GZCOM reference, so a front may
// not die while the core object lives; and a front must not keep the core
// object alive on its own. So: the front holds a strong fc4_adv_obj only while
// its GZCOM count is above 0 and an fc4_adv_weak always; the core's
// object_dropped(token) marks it dropped, and a dropped front with count 0
// (and no call in flight) is deleted.

#pragma once

#include "fc4.h"

#include <cstdint>
#include <mutex>
#include <unordered_map>

namespace fc4 {

class AdvFront {
public:
    // Takes over `handle` (object_created's).
    explicit AdvFront(fc4_adv_obj* handle) : weak_(fc4_adv_obj_downgrade(handle)) { fc4_adv_obj_destroy(handle); }
    virtual ~AdvFront() {
        if (strong_ != nullptr) fc4_adv_obj_destroy(strong_);
        fc4_adv_weak_destroy(weak_);
    }
    AdvFront(const AdvFront&) = delete;
    AdvFront& operator=(const AdvFront&) = delete;

    // Keep a freshly created object alive until the director's first QI.
    // Ownership transfers to the ordinary first GZCOM reference, without
    // adding a reference the caller could never release.
    void HoldForFactory(fc4_adv_obj* handle) { strong_ = handle; }

    // A handle for one call: the strong one, or a temporary upgrade (null
    // once the core object is gone).
    class Pin {
    public:
        explicit Pin(AdvFront& f) : f_(f) {
            ++f_.pins_;
            if (f_.strong_ != nullptr) h_ = f_.strong_;
            else {
                h_ = fc4_adv_weak_upgrade(f_.weak_);
                own_ = true;
            }
        }
        ~Pin() {
            if (own_ && h_ != nullptr) fc4_adv_obj_destroy(h_);  // may run object_dropped: deferred by pins_
            --f_.pins_;
        }
        Pin(const Pin&) = delete;
        Pin& operator=(const Pin&) = delete;
        fc4_adv_obj* get() const { return h_; }
        explicit operator bool() const { return h_ != nullptr; }

    private:
        AdvFront& f_;
        fc4_adv_obj* h_ = nullptr;
        bool own_ = false;
    };

    uint32_t AddRefFront() {
        if (refs_++ == 0 && strong_ == nullptr) strong_ = fc4_adv_weak_upgrade(weak_);
        return refs_;
    }
    // Returns the new count; deletes `this` when it reaches 0 after a drop.
    uint32_t ReleaseFront() {
        if (refs_ != 0) --refs_;
        const uint32_t n = refs_;
        if (n == 0 && strong_ != nullptr) {
            fc4_adv_obj* s = strong_;
            strong_ = nullptr;
            ++pins_;
            fc4_adv_obj_destroy(s);  // the last strong reference: object_dropped runs now
            --pins_;
        }
        if (n == 0 && dropped_ && pins_ == 0) Retire(this);
        return n;
    }

    // object_dropped(token): the core object is gone.
    static void Dropped(uint64_t token) {
        AdvFront* f = nullptr;
        {
            std::lock_guard<std::mutex> lock(Mu());
            auto it = Live().find(token);
            if (it == Live().end()) return;
            f = it->second;
        }
        f->dropped_ = true;
        if (f->refs_ == 0 && f->pins_ == 0) Retire(f);
    }
    static size_t LiveCount() {
        std::lock_guard<std::mutex> lock(Mu());
        return Live().size();
    }
    // Teardown: fronts still referenced by the game (or pinned) are deleted
    // with the system, as before.
    static void DestroyAll() {
        std::unordered_map<uint64_t, AdvFront*> all;
        {
            std::lock_guard<std::mutex> lock(Mu());
            all.swap(Live());
        }
        for (auto& kv : all) delete kv.second;
    }

protected:
    // Called by the derived constructor once the token is known.
    void Register(uint64_t token) {
        token_ = token;
        std::lock_guard<std::mutex> lock(Mu());
        Live()[token] = this;
    }
    uint32_t Refs() const { return refs_; }
    fc4_adv_obj* StrongHandle() const { return strong_; }

private:
    static void Retire(AdvFront* f) {
        {
            std::lock_guard<std::mutex> lock(Mu());
            Live().erase(f->token_);
        }
        delete f;
    }
    static std::mutex& Mu() {
        static std::mutex mu;
        return mu;
    }
    static std::unordered_map<uint64_t, AdvFront*>& Live() {
        static std::unordered_map<uint64_t, AdvFront*> live;
        return live;
    }

    fc4_adv_weak* weak_ = nullptr;
    fc4_adv_obj* strong_ = nullptr;
    uint32_t refs_ = 0;
    uint32_t pins_ = 0;
    bool dropped_ = false;
    uint64_t token_ = 0;
};

}  // namespace fc4
