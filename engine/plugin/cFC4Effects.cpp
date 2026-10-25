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

// The effects manager's GZCOM classes (cFC4Effects.h).

#include "cFC4Effects.h"

#include "FC4Config.h"
#include "FC4EffectsShadow.h"
#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZString.h"
#include "cISC4City.h"
#include "cISC4EffectsManager.h"
#include "cISC4VisualEffect.h"

#include <cstdio>
#include <cstring>

namespace {

// The animation tick (effects.md §12).
constexpr uint32_t kMsgAnimationTick = 0x86AD10EE;

// §2: the highest description priority allowed, from the effects detail
// option. The manager starts at 5 and the render properties may lower it.
constexpr uint32_t kDefaultMaxPriority = 5;

}  // namespace


// The effect's description name, for the shadow's twin lookup.
static bool EffectName(cFC4EffectsManager* owner, uint32_t id, char* buf, size_t cap) {
    if (owner == nullptr || owner->Core() == nullptr || buf == nullptr || cap == 0) return false;
    return fc4_effects_get_name(owner->Core(), id, buf, cap) != 0;
}

// ---- the effect component at +4 -------------------------------------------------------------------

void cFC4VisualEffectComponent::Start(uint32_t mode) {
    if (owner_ == nullptr || owner_->Core() == nullptr) return;
    fc4_effects_start(owner_->Core(), id_, mode);
    char n[128] = {};
    if (EffectName(owner_, id_, n, sizeof(n))) fc4::EffectsShadowObserveStartStop(n, true, mode);
}

void cFC4VisualEffectComponent::Stop(uint32_t mode) {
    if (owner_ == nullptr || owner_->Core() == nullptr) return;
    fc4_effects_stop(owner_->Core(), id_, mode);
    char n[128] = {};
    if (EffectName(owner_, id_, n, sizeof(n))) fc4::EffectsShadowObserveStartStop(n, false, mode);
}

bool cFC4VisualEffectComponent::IsActive() {
    // §11.2 slot 2 forwards to the visual effect's IsStarted.
    return owner_ != nullptr && owner_->Core() != nullptr && fc4_effects_is_started(owner_->Core(), id_);
}

void cFC4VisualEffectComponent::SetTransform(cS3DTransform const& local, cS3DTransform const& parent) {
    // The components' local placements are taken as the identity (the core's
    // EF7): a component's position is the effect's own.
    (void)local;
    (void)parent;
}

void cFC4VisualEffectComponent::Update(float dtSeconds) {
    // The manager's tick updates the components (§6.3). A component's own
    // Update is the core's; the manager calls it, so this is a no-op here.
    (void)dtSeconds;
}

bool cFC4VisualEffectComponent::SetParameter(uint32_t index, float const* data, uint32_t count) {
    (void)index;
    (void)data;
    (void)count;
    return true;
}

bool cFC4VisualEffectComponent::SetParameter2(uint32_t index, void const* data, uint32_t count) {
    (void)count;
    if (owner_ == nullptr || owner_->Core() == nullptr || data == nullptr) return false;
    // §11.2 slot 6: index 13 is the game records' message data.
    return fc4_effects_forward_parameter(owner_->Core(), id_, index, reinterpret_cast<uint64_t>(data));
}

uint32_t cFC4VisualEffectComponent::AddRef() {
    return owner_ != nullptr ? owner_->AddRef() : 0;
}

uint32_t cFC4VisualEffectComponent::Release() {
    return owner_ != nullptr ? owner_->Release() : 0;
}

// ---- the visual effect ----------------------------------------------------------------------------

cFC4VisualEffect::cFC4VisualEffect(cFC4EffectsManager* owner, uint32_t id)
    : owner_(owner), id_(id), component_(owner, id) {}

bool cFC4VisualEffect::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (riid == GZIID_cIGZUnknown) {
        *ppvObj = static_cast<cIGZUnknown*>(static_cast<cFC4ISC4VisualEffect*>(this));
        AddRef();
        return true;
    }
    // The effect-component interface at +4 is the same object's second
    // sub-object (§11.2); it has no separate IID of its own in docs/abi.
    *ppvObj = nullptr;
    return false;
}

uint32_t cFC4VisualEffect::AddRef() { return ++refs_; }

uint32_t cFC4VisualEffect::Release() {
    if (refs_ == 0) return 0;
    const uint32_t n = --refs_;
    if (n == 0) {
        if (owner_ != nullptr) owner_->ReleaseEffect(id_);
        delete this;
    }
    return n;
}

void cFC4VisualEffect::Start(uint32_t mode) {
    if (owner_ == nullptr || owner_->Core() == nullptr) return;
    fc4_effects_start(owner_->Core(), id_, mode);
    char n[128] = {};
    if (EffectName(owner_, id_, n, sizeof(n))) fc4::EffectsShadowObserveStartStop(n, true, mode);
}

void cFC4VisualEffect::Stop(uint32_t mode) {
    if (owner_ == nullptr || owner_->Core() == nullptr) return;
    fc4_effects_stop(owner_->Core(), id_, mode);
    char n[128] = {};
    if (EffectName(owner_, id_, n, sizeof(n))) fc4::EffectsShadowObserveStartStop(n, false, mode);
}

bool cFC4VisualEffect::IsStarted() {
    return owner_ != nullptr && owner_->Core() != nullptr && fc4_effects_is_started(owner_->Core(), id_);
}

bool cFC4VisualEffect::IsStopping() {
    return owner_ != nullptr && owner_->Core() != nullptr && fc4_effects_is_stopping(owner_->Core(), id_);
}

void cFC4VisualEffect::SetSecondaryTransform(cS3DTransform const& t) {
    // Render only (§11.1 slot 7); the core keeps the world translation only.
    (void)t;
}

void cFC4VisualEffect::SetTransform(cS3DTransform const& t) {
    // cS3DTransform is 0x38 bytes; its translation is the first three floats
    // of the object's position field (§11.1 slot 8). The core takes (x, y, z).
    const float* p = reinterpret_cast<const float*>(&t);
    transform_[0] = p[0];
    transform_[1] = p[1];
    transform_[2] = p[2];
    if (owner_ != nullptr && owner_->Core() != nullptr) {
        fc4_effects_set_position(owner_->Core(), id_, transform_[0], transform_[1], transform_[2]);
    }
}

bool cFC4VisualEffect::AttachToOccupant(cISC4Occupant* occupant) {
    // Render (§11.1 slot 9).
    (void)occupant;
    return true;
}

void cFC4VisualEffect::SetEndMessage(uint32_t type, uint32_t data1) {
    if (owner_ == nullptr || owner_->Core() == nullptr) return;
    fc4_effects_set_end_message(owner_->Core(), id_, type, data1);
    char n[128] = {};
    if (EffectName(owner_, id_, n, sizeof(n))) fc4::EffectsShadowObserveEndMessage(n, type, data1);
}

bool cFC4VisualEffect::SetParameter3(uint32_t index, float const* data, uint32_t count) {
    // §11.1 slot 11: `count * 3` floats, then slot 12.
    return SetParameter(index, data, count != 0 ? count * 3 : 0);
}

bool cFC4VisualEffect::SetParameter(uint32_t index, float const* data, uint32_t count) {
    (void)data;
    (void)count;
    if (owner_ == nullptr || owner_->Core() == nullptr) return false;
    if (index > 19) return false;  // §11.1 slot 12: index <= 19
    return true;
}

bool cFC4VisualEffect::ForwardParameter(uint32_t index, void const* data, uint32_t count) {
    (void)count;
    if (owner_ == nullptr || owner_->Core() == nullptr) return false;
    // §11.1 slot 13: forwarded to the components only. Index 13 carries the
    // effect-disaster instance (§10.2).
    return fc4_effects_forward_parameter(owner_->Core(), id_, index, reinterpret_cast<uint64_t>(data));
}

void cFC4VisualEffect::GetTransform(cS3DTransform& t) {
    float* p = reinterpret_cast<float*>(&t);
    p[0] = transform_[0];
    p[1] = transform_[1];
    p[2] = transform_[2];
}

void cFC4VisualEffect::GetSecondaryTransform(cS3DTransform& t) {
    float* p = reinterpret_cast<float*>(&t);
    p[0] = secondary_[0];
    p[1] = secondary_[1];
    p[2] = secondary_[2];
}

bool cFC4VisualEffect::AddComponent(void* descRec, cFC4ISC4EffectComponent* component) {
    // §11.1 slot 16: internal, used while the manager builds components.
    (void)descRec;
    (void)component;
    return true;
}

bool cFC4VisualEffect::TestFlag(uint32_t bit) {
    // §11.1 slot 17: the description's effect flags (§11.1 table), bit <= 8.
    if (bit > 8 || owner_ == nullptr || owner_->Core() == nullptr) return false;
    fc4_effect_info info = {};
    if (!fc4_effects_get_effect(owner_->Core(), id_, &info)) return false;
    return (info.flags & (1u << bit)) != 0;
}


// ---- the manager ----------------------------------------------------------------------------------

cFC4EffectsManager* cFC4EffectsCollection::Owner() {
    return reinterpret_cast<cFC4EffectsManager*>(reinterpret_cast<unsigned char*>(this) - 0x94);
}

bool cFC4EffectsCollection::QueryInterface(uint32_t iid, void** out) {
    return Owner()->QueryInterface(iid, out);
}

uint32_t cFC4EffectsCollection::AddRef() { return Owner()->AddRef(); }
uint32_t cFC4EffectsCollection::Release() { return Owner()->Release(); }

void cFC4EffectsCollection::BeginCollection() {
    // Section 18.6: preserve descriptions and camera values; clear only the
    // publication vectors and camera-presence byte. No nesting counter.
    std::memset(fields_ + 0xF0 - 4, 0, 0x18);
    fields_[0xD0 - 4] = 0;
}

void cFC4EffectsCollection::EndCollection() {
    // Empty publication has no bindings or notification indices to resolve.
}

cFC4EffectsManager::cFC4EffectsManager() : refs_(0) {
    static_assert(sizeof(cFC4EffectsCollection) == 0x120, "native collection extent");
    // The tests exercise these receiver offsets in the compiled x86 DLL.
    uint32_t priority = kDefaultMaxPriority;
    std::memcpy(reinterpret_cast<unsigned char*>(this) + 0xB90, &priority, sizeof(priority));
    const int32_t lod = -1;
    for (size_t offset = 0xC84; offset <= 0xC8C; offset += 4)
        std::memcpy(reinterpret_cast<unsigned char*>(this) + offset, &lod, sizeof(lod));
    ctx_.shadow = false;
    host_ = fc4::MakeEffectsHost(&ctx_);
}

cFC4EffectsManager::~cFC4EffectsManager() {
    if (core_ != nullptr) {
        fc4_effects_destroy(core_);
        core_ = nullptr;
    }
    if (dir_ != nullptr) {
        fc4_effects_directory_destroy(dir_);
        dir_ = nullptr;
    }
}

bool cFC4EffectsManager::QueryInterface(uint32_t riid, void** ppvObj) {

    if (ppvObj == nullptr) return false;
    if (riid == GZIID_cIGZUnknown || riid == kFC4_IID_cISC4EffectsManager) {
        *ppvObj = static_cast<cIGZUnknown*>(static_cast<cFC4ISC4EffectsManager*>(this));
        AddRef();
        return true;
    }
    // effects.md section 18.2: callers need the message sub-object at +8,
    // not the primary manager pointer.
    if (riid == 0x452294AAu) {
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
        AddRef();
        return true;
    }
    if (riid == 0xC97CF5CEu) {
        *ppvObj = &collection_;
        AddRef();
        return true;
    }
    // effects.md section 18.2: an unsupported IID leaves the output untouched.
    return false;
}

uint32_t cFC4EffectsManager::AddRef() { return ++refs_; }

uint32_t cFC4EffectsManager::Release() {

    if (refs_ == 0) return 0;
    const uint32_t n = --refs_;
    if (n == 0) delete this;
    return n;
}

bool cFC4EffectsManager::LoadDirectory() {
    // Sections 3 and 6.2: decode the packed resource into core-owned records.
    // Never publish native allocator-owned objects into Rust.
    std::vector<uint8_t> bytes;
    if (fc4::EffectsReadDirectory(bytes))
        dir_ = fc4_effects_directory_parse(bytes.data(), bytes.size());
    // SPEC-GAP: without a resource service retain the standalone directory.
    if (dir_ == nullptr) dir_ = fc4_effects_directory_volcano();
    if (dir_ == nullptr) return false;
    fc4_effects_directory_counts counts = {};
    if (fc4_effects_directory_get_counts(dir_, &counts)) {
        fc4::Log("effects: directory loaded (brushes %u, attractors %u, games %u, effects %u, names %u)",
            static_cast<unsigned>(counts.brushes), static_cast<unsigned>(counts.attractors),
            static_cast<unsigned>(counts.games), static_cast<unsigned>(counts.effects),
            static_cast<unsigned>(counts.names));
    }
    return true;
}

bool cFC4EffectsManager::Init() {

    if (inited_) return true;
    const uint32_t sessionSeed = static_cast<uint32_t>(fc4::SettingInt("session_seed", "FC4_SESSION_SEED", 0));
    if (core_ == nullptr) {
        core_ = fc4_effects_create(&host_, sessionSeed);
        if (core_ == nullptr) {
            fc4::Log("effects: WARNING the core refused the host table");
            return false;
        }
    }
    if (dir_ == nullptr && !LoadDirectory()) return false;
    if (!fc4_effects_set_directory(core_, dir_)) return false;
    fc4_effects_set_max_priority(core_, kDefaultMaxPriority);
    // EF2: finish outside the draw pass unless the run asks for the original's
    // draw-pass compat mode.
    const long finish = fc4::SettingInt("effects_finish", "FC4_EFFECTS_FINISH", 0);
    fc4_effects_set_finish_mode(core_, finish != 0 ? FC4_EFFECTS_FINISH_DRAW_PASS : FC4_EFFECTS_FINISH_HOOK);
    // §6: the animation tick drives the tick and the finish hook.
    cIGZMessageServer2Ptr ms;
    if (ms) {

        ms->AddNotification(static_cast<cIGZMessageTarget2*>(this), kMsgAnimationTick);

    } else {
        fc4::Log("effects: WARNING no message server; the animation tick will not arrive");
    }
    inited_ = true;
    fc4::Log("effects: Init (finish mode %s, session seed %u)", finish != 0 ? "draw_pass" : "hook",
        static_cast<unsigned>(sessionSeed));
    return true;
}

bool cFC4EffectsManager::Shutdown() {

    cIGZMessageServer2Ptr ms;
    if (ms) ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(this), kMsgAnimationTick);
    if (core_ != nullptr) fc4_effects_shutdown(core_);
    for (auto& kv : wrappers_) {
        if (kv.second != nullptr) kv.second->DetachOwner();
    }
    wrappers_.clear();
    inited_ = false;
    return true;
}

bool cFC4EffectsManager::HasVisualEffect(char const* name) {

    if (core_ == nullptr || name == nullptr) return false;
    return fc4_effects_has(core_, name);
}

// The name form (§4.1): failure leaves the caller's output untouched.
bool cFC4EffectsManager::CreateByName(char const* name, cISC4VisualEffect** out) {
    if (name == nullptr || out == nullptr) return false;  // §4.1 step 1
    if (core_ == nullptr && !Init()) return false;
    uint32_t id = 0;
    if (!fc4_effects_create_effect(core_, name, &id)) return false;  // §4.1 step 2
    cFC4VisualEffect* e = Wrap(id);
    if (e == nullptr) return false;
    *out = fc4::AsSdk(static_cast<cFC4ISC4VisualEffect*>(e));
    fc4::EffectsShadowObserveCreate(name);
    return true;  // §4.1 step 7: an index of -1 still succeeds
}

// The ID-pair form (§4.2). The description is found by an ID pair; the core's directory lookup is
// by name, so this form resolves through the description index the pair denotes (question 7).
bool cFC4EffectsManager::CreateById(uint32_t id0, uint32_t id1, cISC4VisualEffect** out) {
    (void)out;
    fc4::Log("effects: CreateVisualEffect(id pair 0x%08X, 0x%08X) is not resolved yet; question 7",
             static_cast<unsigned>(id0), static_cast<unsigned>(id1));
    return false;
}

bool cFC4EffectsManager::CreateVisualEffectById(uint32_t id0, uint32_t id1, cISC4VisualEffect** out) {
    return CreateById(id0, id1, out);
}

bool cFC4EffectsManager::CreateVisualEffectByName(char const* name, cISC4VisualEffect** out) {
    return CreateByName(name, out);
}

void cFC4EffectsManager::GetStatsString(cIGZString& stats) {

    fc4_effects_state st = {};
    char buf[256] = {};
    if (core_ != nullptr && fc4_effects_get_state(core_, &st)) {
        std::snprintf(buf, sizeof(buf),
                      "effects: created %llu, recycled %llu, end posts %llu, select draws %llu, ticks %llu, "
                      "sweeps %llu, live %u, active %u",
                      static_cast<unsigned long long>(st.created), static_cast<unsigned long long>(st.recycled),
                      static_cast<unsigned long long>(st.end_posts), static_cast<unsigned long long>(st.select_draws),
                      static_cast<unsigned long long>(st.ticks), static_cast<unsigned long long>(st.sweeps),
                      static_cast<unsigned>(st.effect_count), static_cast<unsigned>(st.active_count));
    } else {
        std::snprintf(buf, sizeof(buf), "effects: no core");
    }
    stats.FromChar(buf, static_cast<uint32_t>(std::strlen(buf)));
}

cISC4EffectMaps* cFC4EffectsManager::GetEffectMaps() {

    // Render (§2); out of scope.
    return nullptr;
}

// [abi-ret] effects.md §11.1 / docs/abi: GetTallestBuildingAltitude returns in
// st(0) (x87), not EAX. A replacement answers 0.0 without a render layer.
float cFC4EffectsManager::GetTallestBuildingAltitude() {
 return 0.0f; }

// The draw-pass override (§7, EF2). The original finishes effects here and nowhere else.
// Our sweep runs after each animation tick instead, so this does nothing in the hook mode;
// in the compat mode it is the finishing pass.
void cFC4EffectsManager::Update(void* drawContext) {
    (void)drawContext;

    if (core_ == nullptr) return;
    const long finish = fc4::SettingInt("effects_finish", "FC4_EFFECTS_FINISH", 0);
    if (finish != 0) fc4_effects_draw_pass(core_);
}

bool cFC4EffectsManager::DoMessage(cIGZMessage2* pMessage) {

    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("effects DoMessage", pMessage->GetType());
    const uint32_t type = pMessage->GetType();
    if (type == kMsgAnimationTick) {
        // §6: the animation tick, dilated by the simulator's slot 32. The
        // tick runs while the game is paused (quirk 3). data2 is the elapsed
        // microseconds.
        uint32_t us = 0;
        cIGZMessage2Standard* m = nullptr;
        if (pMessage->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) && m != nullptr) {
            FC4_ACQUIRED(m, "QueryInterface");
            us = static_cast<uint32_t>(m->GetData2());
            FC4_RELEASE(m);
        }
        fc4_effects_tick(core_, us);
        // EF2: our finish point — the same §7 sweep, outside the draw pass.
        fc4_effects_finish_hook(core_);
        // The same-process mirror (effects_shadow = 1) sees the same step.
        fc4::EffectsShadowOnTick(us);
    }
    return true;
}

void cFC4EffectsManager::ReleaseEffect(uint32_t id) {
    if (core_ != nullptr) fc4_effects_release(core_, id);
    wrappers_.erase(id);
}

cFC4VisualEffect* cFC4EffectsManager::Wrap(uint32_t id) {
    auto it = wrappers_.find(id);
    if (it != wrappers_.end() && it->second != nullptr) {
        it->second->AddRef();
        return it->second;
    }
    cFC4VisualEffect* e = new (std::nothrow) cFC4VisualEffect(this, id);
    if (e == nullptr) return nullptr;
    wrappers_[id] = e;
    return e;
}

cIGZUnknown* CreateFC4EffectsManager() {
    cFC4EffectsManager* m = new (std::nothrow) cFC4EffectsManager();
    return m != nullptr ? static_cast<cIGZUnknown*>(static_cast<cFC4ISC4EffectsManager*>(m)) : nullptr;
}
