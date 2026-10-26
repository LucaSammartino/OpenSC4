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
uint32_t fc4_overlay_owner_set_color(void*, uint32_t, float, float, float);
uint32_t fc4_overlay_owner_color(const void*, uint32_t, float*);
uint32_t fc4_overlay_owner_set_draw_state(void*, uint32_t, uint32_t);
uint32_t fc4_overlay_owner_draw_state(const void*, uint32_t);
uint32_t fc4_overlay_owner_set_position(void*, uint32_t, float, float);
uint32_t fc4_overlay_owner_set_placement(void*, uint32_t, const float*);
uint32_t fc4_overlay_owner_placement(const void*, uint32_t, float*);
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
    bool SetColor(uint32_t id, float r, float g, float b) override {
        return fc4_overlay_owner_set_color(core_, id, r, g, b) != 0;
    }
    bool Color(uint32_t id, float* rgb) const override {
        return fc4_overlay_owner_color(core_, id, rgb) != 0;
    }
    bool SetDrawState(uint32_t id, uint32_t index) override {
        return fc4_overlay_owner_set_draw_state(core_, id, index) != 0;
    }
    uint32_t DrawStateIndex(uint32_t id) const override { return fc4_overlay_owner_draw_state(core_, id); }
    bool SetPosition(uint32_t id, float x, float z) override {
        return fc4_overlay_owner_set_position(core_, id, x, z) != 0;
    }
    bool SetPlacement(uint32_t id, const float* placement) override {
        return fc4_overlay_owner_set_placement(core_, id, placement) != 0;
    }
    bool Placement(uint32_t id, float* placement) const override {
        return fc4_overlay_owner_placement(core_, id, placement) != 0;
    }
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
    float rgb[3] = {};
    check(owner->Color(id, rgb) && rgb[0] == 1 && rgb[1] == 1 && rgb[2] == 1);
    check(owner->DrawStateIndex(id) == 0);
    check(owner->SetColor(id, -0.25f, 0.5f, 1.75f));
    check(owner->SetDrawState(id, 0x123403));
    check(owner->Color(id, rgb) && rgb[0] == -0.25f && rgb[1] == 0.5f && rgb[2] == 1.75f);
    check(owner->DrawStateIndex(id) == 3);
    check(owner->IsSelfLit(id) && owner->IsShown(id));
    float placement[9] = {};
    check(owner->Placement(id, placement) && placement[0] == 128 && placement[1] == 128 &&
          placement[2] == 32 && placement[4] == 1 && placement[5] == 1 && placement[6] == 1);
    check(owner->SetPosition(id, 128.00005f, 128));
    check(owner->Placement(id, placement) && placement[0] == 128);
    placement[0] = 160; placement[4] = 2; placement[5] = 3; placement[8] = 7;
    check(owner->SetPlacement(id, placement));
    check(owner->SetPosition(id, 192, 224));
    check(owner->Placement(id, placement) && placement[0] == 192 && placement[1] == 224 &&
          placement[4] == 2 && placement[5] == 3 && placement[8] == 7);
    check(owner->Placement(UINT32_MAX, placement) && placement[0] == 192 && placement[8] == 7);
    check(owner->SetPosition(0, 1, 2));
    check(owner->Placement(0, placement) && placement[0] == 192);
    check(owner->Init(64, 64, 16, false, true));
    check(owner->Alpha(id) == 0.51f);
    check(owner->Color(id, rgb) && rgb[0] == -0.25f && rgb[1] == 0.5f && rgb[2] == 1.75f);
    check(owner->DrawStateIndex(id) == 3);
    check(owner->SetAlpha(id, 0.52f));
    check(owner->Placement(id, placement) && placement[0] == 192 && placement[8] == 7);
    check(owner->Alpha(id) == 0.51f);
    check(owner->IsShown(id) && owner->IsSelfLit(id));
    // The no-handle sentinel is a successful no-op, not an invalid arena index.
    check(owner->SetShown(UINT32_MAX, true));
    check(owner->EnableSelfLighting(UINT32_MAX, 1));
    check(owner->SetColor(UINT32_MAX, 2, 3, 4));
    check(owner->SetDrawState(UINT32_MAX, 255));
    check(owner->Color(UINT32_MAX, rgb) && rgb[0] == 0 && rgb[1] == 0 && rgb[2] == 0);
    check(owner->DrawStateIndex(UINT32_MAX) == 0);
    check(!owner->SetShown(UINT32_MAX - 1, true));
    check(!owner->EnableSelfLighting(UINT32_MAX - 1, 1));
    check(owner->Remove(id));
    check(!owner->IsShown(id) && !owner->IsSelfLit(id));
    owner->Shutdown();
    check(!owner->IsInitialized());
    check(owner->Init(64, 64, 16, false, false));
    check(owner->AddDecal(64, 32, 128, 128, 32, 0) == id);
    check(owner->IsShown(id) && !owner->IsSelfLit(id));
    check(owner->Color(id, rgb) && rgb[0] == 1 && rgb[1] == 1 && rgb[2] == 1);
    check(owner->DrawStateIndex(id) == 0);
    owner->Shutdown();
    FC4_RELEASE_CREATED(owner);
    Log("overlays factory check: %u/%u mismatches=%u scope=portable-owner native_interception=0",
        checks - differences, checks, differences);
}
}
