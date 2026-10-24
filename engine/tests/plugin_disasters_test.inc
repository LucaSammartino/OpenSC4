// SPDX-License-Identifier: GPL-3.0-or-later
#include "cFC4DisasterLayer.h"

int DisastersFactoryTest(const std::string& dll, const std::string& ini, bool enabled) {
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, enabled ? "[subsystems]\nenable = disasters\n" : "[subsystems]\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (!hd) return 1;
    auto get = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    auto* director = get ? get() : nullptr;
    CHECK(director != nullptr);
    SetGamePrecision();
    if (director) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK((std::find(classes.begin(), classes.end(), 0x61F6ABF5) != classes.end()) == enabled);
        void* obj = nullptr;
        CHECK(director->GetClassObject(0x61F6ABF5, 0x29B02A2C, &obj) == enabled);
        if (obj) {
            auto* layer = static_cast<cFC4ISC4DisasterLayer*>(obj);
            CHECK(layer->Init());
            CHECK(layer->DisastersEnabled());
            CHECK(!layer->EnableDisasters(false));
            CHECK(!layer->EnableDisasters(false));
            CHECK(!layer->EnableDisasters(true));
            CHECK(layer->EnableDisasters(true));
            CHECK(layer->GetDisasterManager(0x621CDA33) == nullptr); // no game COM
            CHECK(!layer->IsADisasterOngoing());
            CHECK(layer->RegisterDisasterLocation(nullptr, 12, 34, nullptr, 3, false));
            CHECK(layer->GetDisasterLocationCount() == 1);
            cISC4DisasterInstance* instance = reinterpret_cast<cISC4DisasterInstance*>(1);
            int32_t x = 0, z = 0;
            CHECK(layer->GetDisasterLocationByIndex(0, &instance, x, z));
            CHECK(instance == nullptr && x == 12 && z == 34);
            CHECK(!layer->GetDisasterLocationByIndex(-1, &instance, x, z));
            CHECK(layer->DisasterHasBegun(nullptr));
            CHECK(layer->IsADisasterOngoing());
            CHECK(layer->DisasterHasEnded());
            CHECK(!layer->IsADisasterOngoing());
            const void* cost = layer->GetDisasterCost();
            CHECK(cost != nullptr && *static_cast<const uint32_t*>(cost) == 0);
            CHECK(layer->UnregisterDisasterLocations(nullptr));
            CHECK(layer->GetDisasterLocationCount() == 0);
            CHECK(layer->Shutdown());
            CHECK(layer->GetDisasterCost() == cost); // stable record
            CHECK(layer->Release() == 0);
        }
    }
    FreeLibrary(hd);
    return g_failures == 0 ? 0 : 1;
}
