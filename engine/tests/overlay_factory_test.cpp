// SPDX-License-Identifier: GPL-3.0-or-later
// The real DLL's default-off factory and portable owner lifecycle. This test
// checks the corrected native IID too; it cannot prove terrain-view interception.
#include "FC4OverlayOwner.h"
#include "cIGZCOMDirector.h"
#include <windows.h>
#include <algorithm>
#include <cstdio>
#include <fstream>
#include <string>
#include <thread>
#include <vector>

namespace {
void Collect(uint32_t id, uint32_t, void* out) { static_cast<std::vector<uint32_t>*>(out)->push_back(id); }
int failures = 0;
#define CHECK(expr) do { if (!(expr)) { std::fprintf(stderr, "line %d: %s\n", __LINE__, #expr); ++failures; } } while (0)

struct Resources {
    int refs = 0, bindings = 0, sources = 0, marks = 0, subscriptions = 0;
    int quality = 2, projectionCalls = 0, imports = 0;
    std::vector<uint32_t> slots;
    uint32_t lastKey[3] = {};
    static Resources& Of(uintptr_t p) { return *reinterpret_cast<Resources*>(p); }
    static void Retain(uintptr_t p) { ++Of(p).refs; }
    static void Release(uintptr_t p) { --Of(p).refs; }
    static int32_t Setting(uintptr_t p, uint32_t n) { return n == 0 ? Of(p).quality : 0; }
    static uint32_t Resource(uintptr_t p, uintptr_t object, uint32_t kind, FC4OverlayResource* out) {
        auto& r = Of(p);
        ++r.imports;
        if (!object) return 0;
        if (kind) ++r.sources; else ++r.bindings;
        *out = {object, p, 64, 32};
        return 1;
    }
    static void Free(uintptr_t p, uintptr_t, uint32_t kind) {
        if (kind) --Of(p).sources; else --Of(p).bindings;
    }
    static uint32_t Lookup(uintptr_t p, uint32_t slot, uintptr_t key, FC4OverlayResource* out) {
        auto& r = Of(p);
        r.slots.push_back(slot);
        if (slot == 6) std::copy_n(reinterpret_cast<const uint32_t*>(key), 3, r.lastKey);
        ++r.bindings;
        *out = {123, p, 64, 32};
        return 1;
    }
    static uint32_t KeyWord(uintptr_t) { return 0x11223344; }
    static void Redisplay(uintptr_t p, int32_t, uint32_t, const int32_t*, int32_t) { ++Of(p).marks; }
    static void Subscribe(uintptr_t p, uint32_t type, uint32_t on) {
        CHECK(type == 0xC9F775BB);
        Of(p).subscriptions += on ? 1 : -1;
    }
    static uint32_t Projection(uintptr_t p, int32_t, const void*, float*, const void*, float*) {
        ++Of(p).projectionCalls;
        return 0;
    }
    FC4OverlayHost Host() {
        return {reinterpret_cast<uintptr_t>(this), Retain, Release, 64, 64, 16, Setting,
            Lookup, Resource, Free, KeyWord, Redisplay, Subscribe, Projection, nullptr, nullptr};
    }
};

void CheckNative(FC4IOverlayOwner* owner, FC4INativeOverlay* native) {
    Resources resources;
    auto host = resources.Host();
    CHECK(owner->InitServices(&host, false, false));
    CHECK(resources.refs == 1 && resources.bindings == 1 && resources.subscriptions == 1);
    CHECK(resources.slots == std::vector<uint32_t>{5}); // Init's shadow texture
    const float xz[2] = {128, 128};
    const uint32_t key[3] = {0x7AB50E44, 123, 456};
    const auto source = native->AddSource(reinterpret_cast<void*>(1), xz, 32, 0);
    CHECK(source == 0x80000000 && resources.sources == 1);
    const auto binding = native->AddBinding(reinterpret_cast<void*>(2), xz, 32, 0);
    CHECK(binding == 0x80000001 && resources.bindings == 2);
    const auto keyed = native->AddKey(key, xz, 32, 0);
    CHECK(keyed == 0x80000002 && resources.slots.back() == 6);
    CHECK(std::equal(key, key + 3, resources.lastKey));
    CHECK(native->AddInstance(456, xz, 32, 0) == 0x80000003 && resources.slots.back() == 5);
    const auto round = native->AddRound(456, xz, 32, 0);
    CHECK(round == 0x80000004);
    const auto city = native->AddCity(456);
    CHECK(city < 0x80000000);
    float placement[9] = {};
    native->ReadPlacement(binding, placement);
    CHECK(placement[0] == 128 && placement[4] == 1 && placement[6] == 1);
    placement[0] = 160; placement[8] = 9;
    native->Update(binding, placement);
    native->Move(binding, xz);
    native->ReadPlacement(binding, placement);
    CHECK(placement[0] == 128 && placement[8] == 9);
    native->Show(round, false);
    CHECK(!native->Shown(round));
    native->Alpha(binding, 0.5f);
    CHECK(native->GetAlpha(binding) == 0.5f);
    const float rgb[3] = {2, 3, 4};
    native->Color(binding, rgb);
    float output[3] = {};
    CHECK(native->GetColor(output, binding) == output && std::equal(rgb, rgb + 3, output));
    // Verify output-first slot24, AL slot26, and byte-return slot25 by their
    // actual x86 function types rather than the SDK's incompatible signatures.
    auto** table = *reinterpret_cast<void***>(native);
    using Color = float* (__thiscall*)(void*, float*, uint32_t);
    using SelfLit = bool (__thiscall*)(void*, uint32_t);
    using Mode = uint8_t (__thiscall*)(void*, uint32_t);
    CHECK(reinterpret_cast<Color>(table[24])(native, output, UINT32_MAX) == output);
    CHECK(output[0] == 0 && output[1] == 0 && output[2] == 0);
    native->SelfLighting(binding, 3);
    native->SelfLighting(binding, 0);
    CHECK(reinterpret_cast<SelfLit>(table[26])(native, binding));
    native->DrawMode(binding, 0x123403);
    CHECK(reinterpret_cast<Mode>(table[25])(native, binding) == 3);
    native->TextureScale(binding, 16, 0);
    CHECK(resources.marks > 0);
    const auto shadow = native->AddKeyShadow(key, xz, 16, 0, 0x100);
    CHECK(shadow != UINT32_MAX);
    CHECK(native->AddGroupShadow(0x12345678, xz, 16, 0, 0x101) != UINT32_MAX);
    CHECK(resources.lastKey[0] == 0x7AB50E44 && resources.lastKey[1] == 0x12345678 &&
          resources.lastKey[2] == 0x11223344);
    const int imports = resources.imports;
    float pairs[2] = {};
    const float box[6] = {0, 0, 0, 16, 0, 16};
    CHECK(native->AddModel(reinterpret_cast<void*>(2), 1, xz, pairs, box, xz) == UINT32_MAX);
    CHECK(resources.projectionCalls == 1 && resources.imports == imports);
    native->Erase(source);
    CHECK(resources.sources == 1); // Remove retains payload until Shutdown/reuse.
    owner->Shutdown();
    CHECK(resources.sources == 0 && resources.bindings == 1 && resources.subscriptions == 0 && resources.refs == 0);
    // Quality zero must not dereference ANY add-shadow/model input pointer.
    resources.quality = 0;
    CHECK(owner->InitServices(&host, true, false));
    const auto* bad = reinterpret_cast<const float*>(1);
    CHECK(native->AddKeyShadow(bad, bad, 1, 0, 0) == UINT32_MAX);
    CHECK(native->AddGroupShadow(1, bad, 1, 0, 0) == UINT32_MAX);
    CHECK(native->AddModel(bad, 1, bad, const_cast<float*>(bad), bad, bad) == UINT32_MAX);
    CHECK(resources.projectionCalls == 1);
    owner->Shutdown();
    // Replacing the retained Init shadow binding releases the service context.
    CHECK(owner->Init(64, 64, 16, true, false));
    CHECK(resources.refs == 0 && resources.bindings == 0);
    owner->Shutdown();
}
}
int main(int argc, char** argv) {
    if (argc != 3) return 2;
    char cwd[MAX_PATH];
    if (!GetCurrentDirectoryA(MAX_PATH, cwd)) return 2;
    const std::string dir = std::string(cwd) + "/overlay-factory-" + std::to_string(GetCurrentProcessId());
    if (!CreateDirectoryA(dir.c_str(), nullptr)) return 2;
    const std::string dll = dir + "/FreeCity4.dll";
    if (!CopyFileA(argv[1], dll.c_str(), TRUE)) return 2;
    const std::string mode = argv[2];
    const bool enabled = mode == "on";
    {
        std::ofstream ini(dir + "/FreeCity4.ini");
        ini << "[tools]\ndisable = simulator,budget" << (mode == "disabled" ? ",overlays" : "")
            << "\nenable = " << (mode == "off" ? "" : "overlays") << "\n";
    }
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    HMODULE module = LoadLibraryA(dll.c_str());
    CHECK(module != nullptr);
    if (!module) return 1;
    auto get = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(module, "GZDllGetGZCOMDirector"));
    CHECK(get != nullptr);
    if (!get) return 1;
    auto* director = get();
    std::vector<uint32_t> classes;
    director->EnumClassObjects(Collect, &classes);
    CHECK(std::count(classes.begin(), classes.end(), kFC4_CLSID_OverlayOwner) == (enabled ? 1 : 0));
    FC4IOverlayOwner* owner = nullptr;
    CHECK(director->GetClassObject(kFC4_CLSID_OverlayOwner, kFC4_IID_OverlayOwner,
                                  reinterpret_cast<void**>(&owner)) == enabled);
    if (owner) {
        FC4INativeOverlay* native = nullptr;
        CHECK(owner->QueryInterface(kFC4_IID_NativeOverlay, reinterpret_cast<void**>(&native)) && native);
        cIGZUnknown* identity = nullptr;
        CHECK(native->QueryInterface(GZIID_cIGZUnknown, reinterpret_cast<void**>(&identity)));
        CHECK(identity == static_cast<cIGZUnknown*>(owner));
        if (identity) identity->Release();
        if (native) native->Release();
        FC4INativeOverlay* separate = nullptr;
        CHECK(director->GetClassObject(kFC4_CLSID_OverlayOwner, kFC4_IID_NativeOverlay,
                                      reinterpret_cast<void**>(&separate)) && separate);
        if (separate) CHECK(separate->Release() == 0);
        cIGZUnknown* unknown = nullptr;
        CHECK(owner->QueryInterface(GZIID_cIGZUnknown, reinterpret_cast<void**>(&unknown)));
        CHECK(unknown == static_cast<cIGZUnknown*>(owner));
        if (unknown) CHECK(unknown->Release() == 1);
        CHECK(!owner->QueryInterface(kFC4_IID_OverlayOwner, nullptr));
        CHECK(!owner->IsInitialized());
        CHECK(owner->AddDecal(64, 32, 128, 128, 32, 0) == UINT32_MAX);
        CHECK(owner->Init(64, 64, 16, true, false));
        const auto id = owner->AddDecal(64, 32, 128, 128, 32, 0);
        CHECK(id == 0x80000000 && owner->Alpha(id) == 1);
        CHECK(owner->IsShown(id) && !owner->IsSelfLit(id));
        CHECK(owner->SetShown(id, false) && !owner->IsShown(id));
        CHECK(owner->EnableSelfLighting(id, 2) && !owner->IsSelfLit(id));
        CHECK(owner->EnableSelfLighting(id, 3) && owner->IsSelfLit(id));
        CHECK(owner->EnableSelfLighting(id, 0) && owner->IsSelfLit(id));
        CHECK(owner->SetAlpha(id, 0.5f));
        float rgb[3] = {};
        CHECK(owner->Color(id, rgb) && rgb[0] == 1 && rgb[1] == 1 && rgb[2] == 1);
        CHECK(owner->DrawStateIndex(id) == 0);
        CHECK(owner->SetColor(id, -0.25f, 0.5f, 1.75f));
        CHECK(owner->SetDrawState(id, 0x123403));
        CHECK(owner->Color(id, rgb) && rgb[0] == -0.25f && rgb[1] == 0.5f && rgb[2] == 1.75f);
        CHECK(owner->DrawStateIndex(id) == 3);
        CHECK(!owner->Color(id, nullptr));
        float placement[9] = {};
        CHECK(owner->Placement(id, placement) && placement[0] == 128 && placement[1] == 128 &&
              placement[2] == 32 && placement[3] == 0 && placement[4] == 1 &&
              placement[5] == 1 && placement[6] == 1 && placement[7] == 0 && placement[8] == 0);
        CHECK(owner->SetPosition(id, 128.00005f, 128));
        CHECK(owner->Placement(id, placement) && placement[0] == 128);
        placement[0] = 160; placement[4] = 2; placement[5] = 3; placement[8] = 7;
        CHECK(owner->SetPlacement(id, placement));
        CHECK(owner->SetPosition(id, 192, 224));
        CHECK(owner->Placement(id, placement) && placement[0] == 192 && placement[1] == 224 &&
              placement[4] == 2 && placement[5] == 3 && placement[8] == 7);
        CHECK(!owner->SetPlacement(id, nullptr));
        CHECK(!owner->Placement(id, nullptr));
        for (const uint32_t ignored : {0u, 0x7FFFFFFFu, UINT32_MAX}) {
            CHECK(owner->SetPosition(ignored, 1, 2));
            CHECK(owner->SetPlacement(ignored, placement));
            CHECK(owner->Placement(ignored, placement) && placement[0] == 192 && placement[8] == 7);
        }
        CHECK(owner->Init(64, 64, 16, true, true));
        CHECK(owner->Alpha(id) == 0.5f); // repeated Init retains records (§1.1)
        CHECK(!owner->IsShown(id) && owner->IsSelfLit(id));
        CHECK(owner->Color(id, rgb) && rgb[0] == -0.25f && rgb[2] == 1.75f);
        CHECK(owner->DrawStateIndex(id) == 3);
        CHECK(owner->Placement(id, placement) && placement[0] == 192 && placement[8] == 7);
        std::thread reader([&] {
            for (int i = 0; i < 2000; ++i) {
                const float alpha = owner->Alpha(id);
                CHECK(alpha == 0.5f || alpha == 1);
                CHECK(owner->IsSelfLit(id));
                owner->IsShown(id);
                float color[3] = {};
                CHECK(owner->Color(id, color));
                CHECK((color[0] == -0.25f && color[1] == 0.5f && color[2] == 1.75f)
                      || (color[0] == 2 && color[1] == 3 && color[2] == 4));
                const auto state = owner->DrawStateIndex(id);
                CHECK(state == 3 || state == 255);
                float snapshot[9] = {};
                CHECK(owner->Placement(id, snapshot) && snapshot[0] == 192 &&
                      snapshot[1] == 224 && snapshot[8] == 7);
            }
        });
        for (int i = 0; i < 2000; ++i) {
            owner->SetAlpha(id, i % 2 ? 0.5f : 1);
            owner->SetShown(id, i % 2 != 0);
            owner->SetColor(id, 2, 3, 4);
            owner->SetDrawState(id, 0xFFFFFFFF);
            owner->SetPlacement(id, placement);
        }
        reader.join();
        CHECK(owner->Remove(id));
        CHECK(!owner->IsShown(id) && !owner->IsSelfLit(id));
        CHECK(!owner->SetShown(id, true));
        CHECK(!owner->EnableSelfLighting(id, 1));
        CHECK(!owner->SetColor(id, 1, 2, 3));
        CHECK(!owner->SetDrawState(id, 1));
        CHECK(!owner->SetPosition(id, 1, 2));
        CHECK(!owner->SetPlacement(id, placement));
        CHECK(!owner->Placement(id, placement) && placement[0] == 192 && placement[8] == 7);
        const float retained = rgb[0];
        CHECK(!owner->Color(id, rgb) && rgb[0] == retained);
        owner->Shutdown();
        CHECK(!owner->IsInitialized());
        CHECK(owner->Init(64, 64, 16, false, false));
        CHECK(owner->AddDecal(64, 32, 128, 128, 32, 0) == 0x80000000);
        CHECK(owner->Alpha(UINT32_MAX) == 0);
        CHECK(owner->IsShown(id) && !owner->IsSelfLit(id));
        CHECK(owner->Color(id, rgb) && rgb[0] == 1 && rgb[1] == 1 && rgb[2] == 1);
        CHECK(owner->DrawStateIndex(id) == 0);
        CHECK(!owner->IsShown(UINT32_MAX) && !owner->IsSelfLit(UINT32_MAX));
        // The no-handle sentinel succeeds without modifying the live record.
        CHECK(owner->SetShown(UINT32_MAX, false));
        CHECK(owner->EnableSelfLighting(UINT32_MAX, 1));
        CHECK(owner->SetColor(UINT32_MAX, 2, 3, 4));
        CHECK(owner->SetDrawState(UINT32_MAX, 255));
        CHECK(owner->Color(UINT32_MAX, rgb) && rgb[0] == 0 && rgb[1] == 0 && rgb[2] == 0);
        CHECK(owner->DrawStateIndex(UINT32_MAX) == 0);
        CHECK(owner->Color(id, rgb) && rgb[0] == 1 && rgb[1] == 1 && rgb[2] == 1);
        CHECK(owner->DrawStateIndex(id) == 0);
        CHECK(owner->IsShown(id) && !owner->IsSelfLit(id));
        CHECK(!owner->SetShown(UINT32_MAX - 1, true));
        CHECK(!owner->EnableSelfLighting(UINT32_MAX - 1, 1));
        owner->Shutdown();
        if (native) CheckNative(owner, native); // owner still holds the object.
        CHECK(owner->Release() == 0);
    }
    FreeLibrary(module);
    DeleteFileA(dll.c_str());
    DeleteFileA((dir + "/FreeCity4.ini").c_str());
    DeleteFileA((dir + "/FreeCity4.log").c_str());
    RemoveDirectoryA(dir.c_str());
    return failures ? 1 : 0;
}
