// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4OverlayOwner.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Subsystems.h"
#include "GZServPtrs.h"
#include "cIGZCOM.h"
#include <atomic>

// Static-library transport only; no additions to fc4.h or DLL exports.
extern "C" {
void* fc4_overlay_owner_create();
void fc4_overlay_owner_destroy(void*);
uint32_t fc4_overlay_owner_init(void*, uint32_t, uint32_t, float, uint32_t, uint32_t);
void fc4_overlay_owner_shutdown(void*);
uint32_t fc4_overlay_owner_add(void*, uint32_t, uint32_t, float, float, float, float);
uint32_t fc4_overlay_owner_remove(void*, uint32_t);
uint32_t fc4_overlay_owner_set_alpha(void*, uint32_t, float);
float fc4_overlay_owner_alpha(const void*, uint32_t);
uint32_t fc4_overlay_owner_initialized(const void*);
uint32_t fc4_overlay_owner_set_shown(void*, uint32_t, uint32_t);
uint32_t fc4_overlay_owner_shown(const void*, uint32_t);
uint32_t fc4_overlay_owner_enable_self_lighting(void*, uint32_t, uint32_t);
uint32_t fc4_overlay_owner_self_lit(const void*, uint32_t);
}

namespace {
class OverlayOwner final : public FC4IOverlayOwner {
public:
    OverlayOwner() : core_(fc4_overlay_owner_create()) {
        fc4::Log("overlays owner: FC4 factory constructed (portable input boundary)");
    }
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out) return false;
        *out = nullptr;
        if (iid != GZIID_cIGZUnknown && iid != kFC4_IID_OverlayOwner) return false;
        *out = static_cast<FC4IOverlayOwner*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override {
        const uint32_t n = --refs_;
        if (!n) delete this;
        return n;
    }
    bool Init(uint32_t nx, uint32_t nz, float width, bool dynamic, bool water) override {
        return fc4_overlay_owner_init(core_, nx, nz, width, dynamic, water) != 0;
    }
    void Shutdown() override { fc4_overlay_owner_shutdown(core_); }
    uint32_t AddDecal(uint32_t tw, uint32_t th, float x, float z, float width, float turns) override {
        return fc4_overlay_owner_add(core_, tw, th, x, z, width, turns);
    }
    bool Remove(uint32_t id) override { return fc4_overlay_owner_remove(core_, id) != 0; }
    bool SetAlpha(uint32_t id, float alpha) override { return fc4_overlay_owner_set_alpha(core_, id, alpha) != 0; }
    float Alpha(uint32_t id) const override { return fc4_overlay_owner_alpha(core_, id); }
    bool IsInitialized() const override { return fc4_overlay_owner_initialized(core_) != 0; }
    bool SetShown(uint32_t id, bool shown) override {
        return fc4_overlay_owner_set_shown(core_, id, shown) != 0;
    }
    bool IsShown(uint32_t id) const override { return fc4_overlay_owner_shown(core_, id) != 0; }
    bool EnableSelfLighting(uint32_t id, uint32_t on) override {
        return fc4_overlay_owner_enable_self_lighting(core_, id, on) != 0;
    }
    bool IsSelfLit(uint32_t id) const override { return fc4_overlay_owner_self_lit(core_, id) != 0; }
private:
    // §1.2: destruction and Shutdown are different protocols.
    ~OverlayOwner() { fc4_overlay_owner_destroy(core_); }
    std::atomic<uint32_t> refs_{0};
    void* core_;
};
}

cIGZUnknown* CreateFC4OverlayOwner() { return new OverlayOwner(); }

namespace fc4 {
void OverlayOwnerGameCheck() {
    if (!FindEnabledClass(kFC4_CLSID_OverlayOwner)) return;
    auto* com = GZCOM();
    FC4IOverlayOwner* owner = nullptr;
    if (!com || !com->GetClassObject(kFC4_CLSID_OverlayOwner, kFC4_IID_OverlayOwner,
                                     reinterpret_cast<void**>(&owner)) || !owner) {
        Log("overlays factory check: 0/1 mismatches=1 scope=portable-owner native_interception=0");
        return;
    }
    FC4_ACQUIRED(owner, "GetClassObject");
    uint32_t checks = 0, differences = 0;
    auto check = [&](bool exact) { ++checks; if (!exact) ++differences; };
    check(!owner->IsInitialized());
    check(owner->Init(64, 64, 16, true, false));
    const uint32_t id = owner->AddDecal(64, 32, 128, 128, 32, 0);
    check(id == 0x80000000);
    check(owner->Alpha(id) == 1);
    check(owner->IsShown(id));
    check(owner->SetShown(id, false) && !owner->IsShown(id));
    check(owner->SetShown(id, true) && owner->IsShown(id));
    check(!owner->IsSelfLit(id));
    check(owner->EnableSelfLighting(id, 2) && !owner->IsSelfLit(id));
    check(owner->EnableSelfLighting(id, 3) && owner->IsSelfLit(id));
    check(owner->EnableSelfLighting(id, 0) && owner->IsSelfLit(id));
    check(owner->SetAlpha(id, 0.51f));
    check(owner->Init(64, 64, 16, false, true));
    check(owner->Alpha(id) == 0.51f);
    check(owner->SetAlpha(id, 0.52f));
    check(owner->Alpha(id) == 0.51f);
    check(owner->Remove(id));
    owner->Shutdown();
    check(!owner->IsInitialized());
    check(owner->Init(64, 64, 16, false, false));
    check(owner->AddDecal(64, 32, 128, 128, 32, 0) == id);
    owner->Shutdown();
    FC4_RELEASE_CREATED(owner);
    Log("overlays factory check: %u/%u mismatches=%u scope=portable-owner native_interception=0",
        checks - differences, checks, differences);
}
}
