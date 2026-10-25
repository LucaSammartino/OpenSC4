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
        CHECK(owner->Init(64, 64, 16, true, true));
        CHECK(owner->Alpha(id) == 0.5f); // repeated Init retains records (§1.1)
        CHECK(!owner->IsShown(id) && owner->IsSelfLit(id));
        std::thread reader([&] {
            for (int i = 0; i < 2000; ++i) {
                const float alpha = owner->Alpha(id);
                CHECK(alpha == 0.5f || alpha == 1);
                CHECK(owner->IsSelfLit(id));
                owner->IsShown(id);
            }
        });
        for (int i = 0; i < 2000; ++i) {
            owner->SetAlpha(id, i % 2 ? 0.5f : 1);
            owner->SetShown(id, i % 2 != 0);
        }
        reader.join();
        CHECK(owner->Remove(id));
        owner->Shutdown();
        CHECK(!owner->IsInitialized());
        CHECK(owner->Init(64, 64, 16, false, false));
        CHECK(owner->AddDecal(64, 32, 128, 128, 32, 0) == 0x80000000);
        CHECK(owner->Alpha(UINT32_MAX) == 0);
        CHECK(owner->IsShown(id) && !owner->IsSelfLit(id));
        CHECK(!owner->IsShown(UINT32_MAX) && !owner->IsSelfLit(UINT32_MAX));
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
