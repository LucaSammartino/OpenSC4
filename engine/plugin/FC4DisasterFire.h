// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include "cISC4FireOccupant.h"
#include "cISC4Occupant.h"
#include "cS3DVector3.h"
#include <atomic>
#include <functional>
#include <utility>

namespace fc4 {

// disasters.md 5.3/5.11: native SetTarget depends on static curves installed
// by the native fire instance. A Rust instance must own the fire interface too.
// The native object's occupant base supplies properties, groups and flags only;
// never give its fire interface a target or ask it to build stage visuals.
class DisasterFire final : public cISC4FireOccupant, public cISC4Occupant {
public:
    using TargetChange = std::function<void(cISC4Occupant*, bool)>;
    DisasterFire(cISC4FireOccupant* storage, TargetChange change, std::function<void()> release)
        : storage_(storage), change_(std::move(change)), release_(std::move(release)) {}
    ~DisasterFire() { Shutdown(); if (release_) release_(); }
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out) return false;
        *out = nullptr;
        if (iid == 1 || iid == 0x69D25C60) *out = static_cast<cISC4FireOccupant*>(this);
        else if (iid == GZIID_cISC4Occupant) *out = static_cast<cISC4Occupant*>(this);
        else return false;
        AddRef(); return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { const auto n = --refs_; if (!n) delete this; return n; }
    cISC4Occupant* AsOccupant() override { return this; }
    bool SetTarget(cISC4Occupant* target) override {
        if (target == target_.load()) return true;
        if (target) change_(target, true);
        auto* previous = target_.exchange(target);
        if (previous) change_(previous, false);
        return true;
    }
    cISC4Occupant* GetTarget() override { return target_; }
    uint32_t GetStage() override { return stage_; }
    uint32_t GetMaxStage() override { return maxStage_; }
    // TODO: bridge the core's flame visuals; never rebuild native flames
    // against static curves whose owning native instance does not exist.
    bool SetStage(uint32_t v) override { stage_ = v; return true; }
    uint32_t GetCounter() override { return counter_; }
    bool SetCounter(uint32_t v) override { counter_ = v; return true; }
    bool GetHosed() override { return hosed_ != 0; }
    bool SetHosed(int32_t delta) override { return hose ? hose(delta) : false; }
    uint32_t GetFamily() override { return family_; }
    bool SetFamily(uint32_t v) override { family_ = v; return true; }
    void Sync(uint32_t stage, uint32_t maxStage, int32_t counter, uint32_t hosed) {
        stage_ = stage; maxStage_ = maxStage; counter_ = static_cast<uint32_t>(counter); hosed_ = hosed;
    }
    std::function<bool(int32_t)> hose;
    std::function<void(cISC4Occupant*, uint32_t, uint32_t)> post;

    // occupant-manager.md 5.6 and fire-flames.md 4: keep lifecycle messages
    // on this identity, not the native storage object's occupant pointer.
    bool Init() override {
        if (!initialised_.exchange(true)) { AddOccupantGroup(0x5005); PostOccupantMessage(0x99EF1140, 0); }
        return true;
    }
    bool Shutdown() override {
        SetTarget(nullptr);
        if (initialised_.exchange(false)) PostOccupantMessage(0x99EF1141, 0);
        return true;
    }
    bool IsInitialized() override { return initialised_; }
    cISCPropertyHolder* AsPropertyHolder() override { return Base()->AsPropertyHolder(); }
    int32_t GetType() override { return 0x69D25C5F; }
    // [abi-ret] Position is an output parameter; the native result is ignored.
    bool GetPosition(cS3DVector3* v) override { if (!v) return false; Geometry()->GetPosition(v); return true; }
    // [abi-ret] SetPosition's SDK bool is not a native return contract.
    bool SetPosition(cS3DVector3 const* v) override { Base()->SetPosition(v); return true; }
    cS3DVector3* GetBoundingBox(cS3DVector3* a, cS3DVector3* b) override { return Geometry()->GetBoundingBox(a, b); }
    bool GetBoundingCityCells(SC4Rect<long>& r) override { return Geometry()->GetBoundingCityCells(r); }
    uint32_t SetRemovalFlags(uint32_t f) override { return Base()->SetRemovalFlags(f); }
    uint32_t UnsetRemovalFlags(uint32_t f) override { return Base()->UnsetRemovalFlags(f); }
    bool CanRemove(uint32_t f) override { return Base()->CanRemove(f); }
    bool PostOccupantMessage(uint32_t m, uint32_t d) override { if (post) post(this, m, d); return true; }
    uint32_t GetHighlight() override { return Base()->GetHighlight(); }
    // [abi-ret] Only the stored highlight and notification are observed.
    bool SetHighlight(uint32_t h, bool now) override { Base()->SetHighlight(h, now); return true; }
    uint8_t SetVisibility(bool v, bool send) override { return Base()->SetVisibility(v, send); }
    cISC43DPlaceableObject* GetPlaceableObject() override { return Base()->GetPlaceableObject(); }
    cISC43DPlaceableObject* GetOrCreatePlaceableObject() override { return Base()->GetOrCreatePlaceableObject(); }
    cISC43DPlaceableObject* SetPlaceableObject(cISC43DPlaceableObject* p) override { return Base()->SetPlaceableObject(p); }
    bool IsOccupantGroup(uint32_t g) override { return Base()->IsOccupantGroup(g); }
    // [abi-ret] Discard the group mutation's unspecified result.
    bool AddOccupantGroup(uint32_t g) override { Base()->AddOccupantGroup(g); return true; }
    // [abi-ret] The group set is the output; ignore the native return.
    bool GetOccupantGroups(std::set<uint32_t>& g) override { Base()->GetOccupantGroups(g); return true; }
    bool GetOccupantManagerBBox(uint8_t* b) override { return Base()->GetOccupantManagerBBox(b); }
    // [abi-ret] occupant-manager.md 5.1 ignores slot 26's result.
    bool SetOccupantManagerBBox(uint8_t* b) override { Base()->SetOccupantManagerBBox(b); return true; }
    bool GetLotTag(uint32_t& t) override { return Base()->GetLotTag(t); }
    // [abi-ret] SetLotTag's SDK bool is not a native return contract.
    bool SetLotTag(uint32_t t) override { Base()->SetLotTag(t); return true; }
    uint32_t SetFlag(uint32_t f) override { return Base()->SetFlag(f); }
    cISC4Occupant* SetAllFlags(uint32_t f) override { Base()->SetAllFlags(f); return this; }
    uint32_t ClearFlag(uint32_t f) override { return Base()->ClearFlag(f); }
    bool IsFlagSet(uint32_t f) override { return Base()->IsFlagSet(f); }
    uint32_t GetFlags() override { return Base()->GetFlags(); }
private:
    cISC4Occupant* Base() const { return storage_->AsOccupant(); }
    cISC4Occupant* Geometry() const { auto* target = target_.load(); return target ? target : Base(); }
    std::atomic<uint32_t> refs_{1}, stage_{uint32_t(-1)}, maxStage_{2}, counter_{0}, hosed_{0}, family_{0};
    cISC4FireOccupant* storage_;
    std::atomic<cISC4Occupant*> target_{nullptr};
    std::atomic<bool> initialised_{false};
    TargetChange change_;
    std::function<void()> release_;
};
} // namespace fc4
