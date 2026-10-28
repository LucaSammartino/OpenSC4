// SPDX-License-Identifier: GPL-3.0-or-later
#include "cFC4LotConfiguration.h"
#include "cFC4LotConfigManager.h"

int LotConfigurationOwnershipTest(const std::string& dll, const std::string& ini) {
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, "[subsystems]\nenable = lotcfg\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (!hd) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    auto seam = reinterpret_cast<FC4TestSeamFn>(GetProcAddress(hd, "FC4TestSeam"));
    auto* director = getDirector ? getDirector() : nullptr;
    CHECK(director && seam);
    if (director && seam) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(std::count(classes.begin(), classes.end(), kFC4_CLSID_cSC4LotConfigManager) == 1);
        // Spec 1.1: configurations are allocated by the manager, never GZCOM.
        CHECK(std::count(classes.begin(), classes.end(), 0xA99062B4u) == 0);
        void* absent = nullptr;
        CHECK(!director->GetClassObject(0xA99062B4, 1, &absent));
        CHECK(absent == nullptr);
        void* raw = nullptr;
        CHECK(director->GetClassObject(kFC4_CLSID_cSC4LotConfigManager, kFC4_IID_cISC4LotConfigManager, &raw));
        auto* config = reinterpret_cast<cFC4ISC4LotConfiguration*>(seam(kFC4SeamLotConfigurationCreate, nullptr, nullptr));
        CHECK(config != nullptr);
        if (config && raw) {
            auto* manager = static_cast<cFC4ISC4LotConfigurationManager*>(raw);
            // B5.5: the input is a vector, and even without city providers
            // each method fills exactly four floats with its minimum footprint.
            uint32_t ids[] = {1, 2, 3, 4, 5, 6};
            struct IdVector { uint32_t* begin; uint32_t* end; uint32_t* capacity; } families{ids, ids + 6, ids + 6};
            for (unsigned kind = 0; kind < 3; ++kind) {
                float out[] = {-11.0f, -22.0f, -33.0f, -44.0f, -55.0f, -66.0f};
                auto& rect = *reinterpret_cast<cS3DRect2D*>(out + 1);
                if (kind == 0) manager->GetMaxFootprintForBuildingFamilies(&families, rect);
                else if (kind == 1) manager->GetMaxFootprintForFloraFamilies(&families, rect);
                else manager->GetMaxFootprintForPropFamilies(&families, rect);
                const float minimum = kind == 0 ? 4.0f : 2.0f;
                CHECK(out[0] == -11.0f && out[5] == -66.0f);
                CHECK(out[1] == minimum && out[2] == minimum && out[3] == 0.0f && out[4] == 0.0f);
            }
            CHECK(config->GetID() == 0 && config->GetLotConfigurationObjectCount() == 0);
            uint32_t x = 99, z = 99; config->GetSize(x, z); CHECK(x == 0 && z == 0);
            CHECK(config->GetMaximumSlope() == 90.0f && config->GetMinBuildingCapacity() == 0xFFFF);
            void* qi = reinterpret_cast<void*>(1);
            CHECK(!config->QueryInterface(0xDEADBEEF, &qi) && qi == nullptr);
            CHECK(config->QueryInterface(kFC4_IID_cISC4LotConfiguration, &qi) && qi == config);
            CHECK(static_cast<cIGZUnknown*>(qi)->Release() == 1);
            CHECK(manager->AddLotConfiguration(reinterpret_cast<cISC4LotConfiguration*>(config), true));
            CHECK(config->Release() == 1); // Only the catalogue reference survives.
            CHECK(manager->GetLotConfiguration(0) == reinterpret_cast<cISC4LotConfiguration*>(config));
            CHECK(config->AddRef() == 2); // GetLotConfiguration is borrowed.
            CHECK(config->Release() == 1);
            CHECK(config->AddRef() == 2); // Hold a caller reference during replacement (B3.4).
            CHECK(manager->AddLotConfiguration(reinterpret_cast<cISC4LotConfiguration*>(config), true));
            CHECK(config->Release() == 1);
            CHECK(manager->RemoveLotConfiguration(0));
            CHECK(manager->GetLotConfiguration(0) == nullptr);
            // B3.4 deliberately erases without Release; discharge that reference.
            CHECK(config->Release() == 0);
        } else if (config) config->Release();
        if (raw) static_cast<cFC4ISC4LotConfigurationManager*>(raw)->Release();
    }
    FreeLibrary(hd);
    return g_failures == 0 ? 0 : 1;
}
