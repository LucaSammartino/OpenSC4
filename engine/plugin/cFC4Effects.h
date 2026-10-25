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

// The effects manager's GZCOM classes (docs/specs/effects.md §1, §11):
// cSC4EffectsManager (CLSID 0x49822F75, city component, city +0x1FC, city
// slot 85), its visual effects and their component sub-objects, backed by the
// Rust core (fc4_effects). The manager is NOT a saved component (§13).
//
// Finishing (§7, EF2). The original finishes effects only in the 3D view's
// draw pass, and a session that never draws never ends one. This
// implementation finishes them OUTSIDE the draw pass: the manager runs the
// same §7 sweep from the animation tick's finish hook (fc4_effects_finish_hook,
// FinishMode::Hook). It does not synthesise an end message. The original's
// behaviour is still available as the compat mode `effects_finish = draw_pass`
// (fc4_effects_draw_pass), so a run can measure the difference.
//
// Animations are stubbed (FC4EffectsHost.h): particles, decals, sequences,
// sounds, cameras and attractors exist as core components but nothing
// animates them, so `render_component_active` is always false and a render
// component never holds an effect. The finish test is therefore NOT the
// original's — see docs/lanes/glue22.md question 2.
//
// The effect directory (§3) is the core's synthetic volcano / meteor-crater
// directory by default: it holds the records the disasters use. Parsing the
// shipped 1154-effect resource out of SimCity_1.dat is core-side (and covered
// by the ef17_ vectors); feeding it from the game's resource manager is
// question 1.
//
// Layout (§18): cISC4EffectsManager and cIGZUnknown at +0, the draw-pass
// sub-object at +4, and cIGZMessageTarget2 at +8 (the animation tick).
// Native field storage and the collection identity are retained through +0xE00.
// Unspecified internal-base dispatch is left unexposed.
// A visual effect has cISC4VisualEffect at +0 and
// the effect component at +4 (vtable 0x00AA1148) — two sub-objects, so the
// overlapping method names live on different classes.
//
// OFF by default: FreeCity4.ini `enable = effects`.
#pragma once

#include "FC4EffectsHost.h"
#include "FC4Interfaces.h"

#include "cIGZMessageTarget2.h"
#include "fc4.h"

#include <cstdint>
#include <map>

class cIGZString;
class cISC4EffectMaps;
class cISC4Occupant;
class cISC4VisualEffect;
class cS3DTransform;

static const uint32_t kFC4_CLSID_cSC4EffectsManager = 0x49822F75;
static const uint32_t kFC4_IID_cISC4EffectsManager = 0xC97CF5CD;
static const uint32_t kFC4_IID_cISC4DisasterManagerEffect = 0xCA7B3F3C;

class cFC4EffectsManager;

// The effect component at +4 of a visual effect (§11.2). Its Start, Stop and
// IsActive forward to the owner's visual-effect slots 3, 4 and 5; Update is
// the core's per-component update (a nested meta component's child is updated
// by the manager's tick).
class cFC4VisualEffectComponent final : public cFC4ISC4EffectComponent {
public:
    explicit cFC4VisualEffectComponent(cFC4EffectsManager* owner, uint32_t id) : owner_(owner), id_(id) {}

    void Start(uint32_t mode) override;
    void Stop(uint32_t mode) override;
    bool IsActive() override;
    void SetTransform(cS3DTransform const& local, cS3DTransform const& parent) override;
    void Update(float dtSeconds) override;
    bool SetParameter(uint32_t index, float const* data, uint32_t count) override;
    bool SetParameter2(uint32_t index, void const* data, uint32_t count) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

private:
    cFC4EffectsManager* owner_;
    uint32_t id_;
};

// A visual effect: one fc4_effects id behind cISC4VisualEffect, with the
// component sub-object at +4 that lets an effect act as a component of
// another (§11.2).
class cFC4VisualEffect final : public cFC4ISC4VisualEffect {
public:
    cFC4VisualEffect(cFC4EffectsManager* owner, uint32_t id);
    ~cFC4VisualEffect() = default;

    // cIGZUnknown (the +0 sub-object)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4VisualEffect (§11.1)
    void Start(uint32_t mode) override;
    void Stop(uint32_t mode) override;
    bool IsStarted() override;
    bool IsStopping() override;
    void SetSecondaryTransform(cS3DTransform const& t) override;
    void SetTransform(cS3DTransform const& t) override;
    bool AttachToOccupant(cISC4Occupant* occupant) override;
    void SetEndMessage(uint32_t type, uint32_t data1) override;
    bool SetParameter3(uint32_t index, float const* data, uint32_t count) override;
    bool SetParameter(uint32_t index, float const* data, uint32_t count) override;
    bool ForwardParameter(uint32_t index, void const* data, uint32_t count) override;
    void GetTransform(cS3DTransform& t) override;
    void GetSecondaryTransform(cS3DTransform& t) override;
    bool AddComponent(void* descRec, cFC4ISC4EffectComponent* component) override;
    bool TestFlag(uint32_t bit) override;

    uint32_t EffectId() const { return id_; }
    cFC4EffectsManager* Owner() { return owner_; }
    // The +4 sub-object, as the effect-component interface.
    cFC4ISC4EffectComponent* ComponentOf() { return &component_; }
    void DetachOwner() { owner_ = nullptr; }

private:
    cFC4EffectsManager* owner_;
    uint32_t id_;
    uint32_t refs_ = 1;
    cFC4VisualEffectComponent component_;
    float transform_[3] = {0, 0, 0};
    float secondary_[3] = {0, 0, 0};
};

// Section 18.2: this identity belongs to the manager, not to an independently
// reference-counted object. Its native collection fields start at receiver +4.
class cFC4EffectsCollection final : public cIGZUnknown {
public:
    bool QueryInterface(uint32_t iid, void** out) override;
    uint32_t AddRef() override;
    uint32_t Release() override;
    virtual void BeginCollection();
    virtual void EndCollection();

private:
    // SPEC-GAP: native publication calls beyond Begin/End are not exposed until
    // description value construction/copy contracts have an adapter.
    unsigned char fields_[0x11C] = {};
    cFC4EffectsManager* Owner();
};

class cFC4EffectsManager final : public cFC4ISC4EffectsManager,
                              public cFC4ISC4EffectsDrawPass,
                              public cIGZMessageTarget2 {
public:
    cFC4EffectsManager();
    ~cFC4EffectsManager();

    // cIGZUnknown (both sub-objects)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4EffectsManager (§1)
    bool Init() override;
    bool Shutdown() override;
    bool HasVisualEffect(char const* name) override;
    bool CreateVisualEffect(uint32_t id0, uint32_t id1, cISC4VisualEffect** out) override;
    bool CreateVisualEffect(char const* name, cISC4VisualEffect** out) override;
    void GetStatsString(cIGZString& stats) override;
    cISC4EffectMaps* GetEffectMaps() override;
    float GetTallestBuildingAltitude() override;

    // cIGZMessageTarget2: kSC4MessageAnimationTick (0x86AD10EE) drives §6, and
    // the finish hook runs after each tick (EF2).
    bool DoMessage(cIGZMessage2* pMessage) override;

    // The +4 sub-object: the 3D view's draw pass (§7). In the compat finish mode this is
    // the only thing that finishes effects; in ours it runs nothing (EF2).
    void Update(void* drawContext) override;

    // For the visual effects and the host callbacks.
    fc4_effects* Core() { return core_; }
    cISC4City* City() { return ctx_.city; }
    fc4::EffectsHostContext& Ctx() { return ctx_; }
    // Drops the manager's reference on an effect (a wrapper's last Release).
    void ReleaseEffect(uint32_t id);
    cFC4EffectsCollection* Collection() { return &collection_; }

private:
    // Section 18.4: direct native fields must not alias core_, ctx_ or host_.
    // SPEC-GAP: section 18.5 does not specify callable internal-base contracts.
    unsigned char nativeBases_[0x94 - 12] = {};
    cFC4EffectsCollection collection_;
    unsigned char nativeState_[0xE00 - 0x1B4] = {};

    // The two overload bodies (§4.1, §4.2). The vtable slots classify their first argument and
    // call one of these, because the SDK's declaration order and the measured one disagree
    // (question 8).
    bool CreateByName(char const* name, cISC4VisualEffect** out);
    bool CreateById(uint32_t id0, uint32_t id1, cISC4VisualEffect** out);
    bool LoadDirectory();
    cFC4VisualEffect* Wrap(uint32_t id);

    fc4_effects* core_ = nullptr;
    fc4_effects_directory* dir_ = nullptr;
    fc4::EffectsHostContext ctx_;
    fc4_effects_host host_ = {};
    // Director factories return refcount zero; QueryInterface owns the first reference.
    uint32_t refs_;
    bool inited_ = false;
    std::map<uint32_t, cFC4VisualEffect*> wrappers_;
};

// The factory FC4Subsystems.cpp registers for CLSID 0x49822F75.
cIGZUnknown* CreateFC4EffectsManager();
