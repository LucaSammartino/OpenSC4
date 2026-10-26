// SPDX-License-Identifier: GPL-3.0-or-later
// The real DLL's default-off factory and portable owner lifecycle. This test
// deliberately rejects native IID 0x895D6544; it cannot prove native interception.
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
        void* native = reinterpret_cast<void*>(1);
        CHECK(!owner->QueryInterface(0x895D6544, &native) && native == nullptr);
        native = reinterpret_cast<void*>(1);
        CHECK(!director->GetClassObject(kFC4_CLSID_OverlayOwner, 0x895D6544, &native) && native == nullptr);
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
        CHECK(owner->Release() == 0);
    }
    FreeLibrary(module);
    DeleteFileA(dll.c_str());
    DeleteFileA((dir + "/FreeCity4.ini").c_str());
    DeleteFileA((dir + "/FreeCity4.log").c_str());
    RemoveDirectoryA(dir.c_str());
    return failures ? 1 : 0;
}
