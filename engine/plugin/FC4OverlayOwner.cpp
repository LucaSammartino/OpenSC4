// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4OverlayOwner.h"
#include "FC4OverlayBindings.h"
#include "FC4OverlayRectangles.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Subsystems.h"
#include "GZServPtrs.h"
#include "cIGZCOM.h"
#include "cIGZString.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessageTarget2.h"
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
uint32_t fc4_overlay_native_init(void*, const FC4OverlayHost*, uint32_t, uint32_t);
uint32_t fc4_overlay_native_add(void*, uint32_t, uintptr_t, const float*, float, float, uint32_t);
uint32_t fc4_overlay_native_model(void*, uintptr_t, int32_t, const void*, float*, const float*, const void*);
void fc4_overlay_native_scale(void*, uint32_t, float, float);
size_t fc4_overlay_native_stats(const void*, char*, size_t);
uint32_t fc4_overlay_native_message(void*, uint32_t);
uint32_t fc4_overlay_native_draw(void*, const FC4OverlayFrame*);
}

namespace {
// The reference word occupies +0x0c; every subobject shares this lifetime.
// No native member function or deleting destructor is called on this object.
class OverlayLifetime : public cIGZUnknown {
protected:
    std::atomic<uint32_t> refs_{0};
};
static_assert(sizeof(OverlayLifetime) == 8, "x86 unknown/reference prefix");

class OverlayOwner final : public FC4INativeOverlay, public FC4IOverlayDraw,
                           public OverlayLifetime, public cIGZMessageTarget2,
                           public FC4IOverlayOwner {
public:
    OverlayOwner() : core_(fc4_overlay_owner_create()) {
        fc4::Log("overlays owner: FC4 factory constructed (portable input boundary)");
    }
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out) return false;
        *out = nullptr;
        if (iid == GZIID_cIGZUnknown)
            *out = static_cast<OverlayLifetime*>(this);
        else if (iid == kFC4_IID_OverlayOwner)
            *out = static_cast<FC4IOverlayOwner*>(this);
        else if (iid == kFC4_IID_NativeOverlay)
            *out = static_cast<FC4INativeOverlay*>(this);
        else return false;
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override {
        const uint32_t n = --refs_;
        if (!n) delete this;
        return n;
    }
    bool Draw(void*) override {
        // Native Draw stays disabled until production frame services and
        // pre-Init terrain-view routing are installed. This is not a frame cast.
        return false;
    }
    bool DoMessage(cIGZMessage2* message) override {
        return message && fc4_overlay_native_message(core_, message->GetType()) != 0;
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
    bool InitServices(const FC4OverlayHost* services, bool dynamic, bool water) override {
        return fc4_overlay_native_init(core_, services, dynamic, water) != 0;
    }
    bool DrawFrame(const FC4OverlayFrame* frame) override {
        return fc4_overlay_native_draw(core_, frame) != 0;
    }
    bool InitBindingFactory(cIGZUnknown* factory, uint32_t nx, uint32_t nz, float width,
                             bool dynamic, bool water, int32_t quality, int32_t debug,
                             const FC4OverlayBindingCalls* test) override {
        return fc4::overlay_bindings::Init(this, factory, nx, nz, width, dynamic, water, quality, debug, test);
    }
    uint32_t AddSource(const void* source, const float* xz, float width, float turns) override {
        return fc4_overlay_native_add(core_, 0, reinterpret_cast<uintptr_t>(source), xz, width, turns, 0);
    }
    uint32_t AddBinding(const void* binding, const float* xz, float width, float turns) override {
        return fc4_overlay_native_add(core_, 1, reinterpret_cast<uintptr_t>(binding), xz, width, turns, 0);
    }
    uint32_t AddKey(const void* key, const float* xz, float width, float turns) override {
        return fc4_overlay_native_add(core_, 2, reinterpret_cast<uintptr_t>(key), xz, width, turns, 0);
    }
    uint32_t AddInstance(uint32_t key, const float* xz, float width, float turns) override {
        return fc4_overlay_native_add(core_, 3, key, xz, width, turns, 0);
    }
    uint32_t AddRound(uint32_t key, const float* xz, float width, float turns) override {
        return fc4_overlay_native_add(core_, 4, key, xz, width, turns, 0);
    }
    uint32_t AddCity(uint32_t key) override {
        return fc4_overlay_native_add(core_, 5, key, nullptr, 0, 0, 0);
    }
    uint32_t AddKeyShadow(const void* key, const float* xz, float width, float turns, uint32_t flip) override {
        return fc4_overlay_native_add(core_, 6, reinterpret_cast<uintptr_t>(key), xz, width, turns, flip);
    }
    uint32_t AddGroupShadow(uint32_t group, const float* xz, float width, float turns, uint32_t flip) override {
        return fc4_overlay_native_add(core_, 7, group, xz, width, turns, flip);
    }
    uint32_t AddModel(const void* binding, int32_t n, const void* source, float* pairs,
                      const float* box, const void* descriptor) override {
        return fc4_overlay_native_model(core_, reinterpret_cast<uintptr_t>(binding), n, source, pairs, box, descriptor);
    }
    void Move(uint32_t id, const float* xz) override { if (xz) SetPosition(id, xz[0], xz[1]); }
    void Update(uint32_t id, const float* p) override { SetPlacement(id, p); }
    void ReadPlacement(uint32_t id, float* p) const override { Placement(id, p); }
    void Erase(uint32_t id) override { Remove(id); }
    void Show(uint32_t id, bool on) override { SetShown(id, on); }
    void Alpha(uint32_t id, float a) override { SetAlpha(id, a); }
    void Color(uint32_t id, const float* rgb) override { if (rgb) SetColor(id, rgb[0], rgb[1], rgb[2]); }
    void DrawMode(uint32_t id, uint32_t mode) override { SetDrawState(id, mode); }
    void SelfLighting(uint32_t id, uint32_t on) override { EnableSelfLighting(id, on); }
    void TextureScale(uint32_t id, float size, float offset) override {
        fc4_overlay_native_scale(core_, id, size, offset);
    }
    bool Shown(uint32_t id) const override { return IsShown(id); }
    float GetAlpha(uint32_t id) const override { return Alpha(id); }
    float* GetColor(float* rgb, uint32_t id) const override { Color(id, rgb); return rgb; }
    uint8_t GetDrawMode(uint32_t id) const override { return static_cast<uint8_t>(DrawStateIndex(id)); }
    bool SelfLit(uint32_t id) const override { return IsSelfLit(id); }
    void Stats(cIGZString& out) override {
        char text[80] = {};
        fc4_overlay_native_stats(core_, text, sizeof(text));
        out.FromChar(text);
    }
private:
    // §1.2: destruction and Shutdown are different protocols.
    ~OverlayOwner() { fc4_overlay_owner_destroy(core_); }
    void* core_;
};
}

cIGZUnknown* CreateFC4OverlayOwner() { return static_cast<FC4IOverlayOwner*>(new OverlayOwner()); }

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
    const auto consumers = overlay_rectangles::Resolve();
    uint32_t targets = 0;
    for (uint32_t dynamic = 0; dynamic != 2; ++dynamic) {
        targets += consumers.land[dynamic] != nullptr;
        targets += consumers.extension[dynamic] != nullptr;
        targets += consumers.water[dynamic] != nullptr;
        targets += consumers.shadow[dynamic] != nullptr;
    }
    check(targets == 8);
    Log("overlays rectangle boundary: targets=%u/8 scope=version-gated-addresses native_interception=0", targets);
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
    FC4INativeOverlay* native = nullptr;
    check(owner->QueryInterface(kFC4_IID_NativeOverlay, reinterpret_cast<void**>(&native)) && native);
    if (native) {
        FC4_ACQUIRED(native, "QueryInterface");
        auto* draw = reinterpret_cast<FC4IOverlayDraw*>(reinterpret_cast<uint8_t*>(native) + 4);
        auto* target = reinterpret_cast<cIGZMessageTarget2*>(reinterpret_cast<uint8_t*>(native) + 0x10);
        cIGZUnknown* identity = nullptr;
        check(draw->QueryInterface(GZIID_cIGZUnknown, reinterpret_cast<void**>(&identity)) && identity);
        if (identity) {
            FC4_ACQUIRED(identity, "QueryInterface");
            check(reinterpret_cast<uint8_t*>(identity) == reinterpret_cast<uint8_t*>(native) + 8);
            FC4_RELEASE(identity);
        }
        FC4INativeOverlay* primary = nullptr;
        check(target->QueryInterface(kFC4_IID_NativeOverlay, reinterpret_cast<void**>(&primary)) && primary);
        if (primary) {
            FC4_ACQUIRED(primary, "QueryInterface");
            check(primary == native);
            FC4_RELEASE(primary);
        }
        check(!draw->Draw(reinterpret_cast<void*>(1)) && !target->DoMessage(nullptr));
        check(native->GetAlpha(id) == 0.51f && native->Shown(id) && native->SelfLit(id));
        check(native->GetDrawMode(id) == 3);
        float output[3] = {};
        check(native->GetColor(output, id) == output && output[0] == -0.25f &&
              output[1] == 0.5f && output[2] == 1.75f);
        native->ReadPlacement(id, placement);
        check(placement[0] == 192 && placement[1] == 224 && placement[8] == 7);
        FC4_RELEASE(native);
    }
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
    // Controlled private-manager fixture using the live factory. This does not
    // replace or draw the terrain view's managers, and its dimensions are inputs.
    auto* framework = RZGetFrameWork();
    cIGZUnknown* factory = nullptr;
    check(framework && framework->GetSystemService(0x1AC0E11A, 0xFAC0E219,
                                                  reinterpret_cast<void**>(&factory)) && factory);
    if (factory) {
        FC4_ACQUIRED(factory, "GetSystemService");
        check(owner->InitBindingFactory(factory, 64, 64, 16, true, false, 2, 0));
        native = nullptr;
        check(owner->QueryInterface(kFC4_IID_NativeOverlay, reinterpret_cast<void**>(&native)) && native);
        if (native) {
            FC4_ACQUIRED(native, "QueryInterface");
            const float xz[2] = {128, 128};
            const uint32_t key[3] = {0x7AB50E44, 0x1ABE787D, 0xAA592B77};
            check(native->AddInstance(0xAA592B77, xz, 32, 0) != UINT32_MAX);
            check(native->AddKey(key, xz, 32, 0) != UINT32_MAX);
            auto** table = *reinterpret_cast<void***>(factory);
            using Lookup = bool (__thiscall*)(void*, uint32_t, void**);
            void* binding = nullptr;
            reinterpret_cast<Lookup>(table[5])(factory, 0xAA592B77, &binding);
            check(binding != nullptr);
            if (binding) {
                check(native->AddBinding(binding, xz, 32, 0) != UINT32_MAX);
                // Bare counted binding, not COM: this factory-created reference
                // is balanced through its documented nonvirtual Release.
                fc4::overlay_bindings::NativeRelease(binding);
            }
            FC4_RELEASE(native);
        }
        owner->Shutdown();
        check(owner->Init(64, 64, 16, true, false)); // release retained shadow binding
        owner->Shutdown();
        FC4_RELEASE_CREATED(factory);
    }
    FC4_RELEASE_CREATED(owner);
    Log("overlays factory check: %u/%u mismatches=%u scope=portable-owner-native-prefix-live-bindings native_interception=0",
        checks - differences, checks, differences);
}
}
