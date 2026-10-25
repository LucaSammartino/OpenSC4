// SPDX-License-Identifier: GPL-3.0-or-later
#include "cFC4DisasterLayer.h"
#include "cISC4DisasterManager.h"
#include "cISC4DisasterManagerFire.h"
#include "cISC4DisasterInstance.h"
#include "cISC4DisasterInstanceFire.h"

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
        const uint32_t managerClasses[] = {0xA9B5AB9F, 0x69C1718E, 0x0A086BD3, 0x4A132FDF, 0x4A38206C, 0x6A7B3E9B, 0xEBFB0402};
        const uint32_t instanceClasses[] = {0x29B5AB89, 0x69C1718F, 0xAA086D86, 0x0A133012, 0x4A38206D, 0x6A7B3E9C, 0xEBFB0403};
        const uint32_t types[] = {0xC4C90997, 0x621CDA33, 0x42963812, 0x4A1332E8, 0xCA380B06, 0x6A7B4397, 0xEBFB0404};
        for (size_t n = 0; n < 7; ++n) {
            CHECK((std::find(classes.begin(), classes.end(), managerClasses[n]) != classes.end()) == enabled);
            CHECK((std::find(classes.begin(), classes.end(), instanceClasses[n]) != classes.end()) == enabled);
            void* manager = nullptr;
            CHECK(director->GetClassObject(managerClasses[n], 0x021E9CBF, &manager) == enabled);
            if (manager) {
                auto* m = static_cast<cFC4ISC4DisasterManager*>(manager);
                CHECK(m->GetID() == types[n]);
                CHECK(m->Init());
                CHECK(m->GetActiveInstanceCount() == 0);
                // Calls the real ABI slot 7 (zero arguments).
                CHECK(m->AttemptDisasterStart() == nullptr);
                CHECK(m->Shutdown());
                CHECK(m->Release() == 0);
            }
            void* instance = nullptr;
            CHECK(director->GetClassObject(instanceClasses[n], 1, &instance) == enabled);
            if (instance) {
                auto* i = static_cast<cISC4DisasterInstance*>(instance);
                CHECK(i->GetID() == types[n]);
                CHECK(i->GetPhase() == 0);
                int32_t x = 7, z = 8;
                CHECK(!i->GetLocation(x, z));
                CHECK(x == 7 && z == 8);
                CHECK(i->Shutdown());
                CHECK(i->Release() == 0);
            }
        }
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
            auto* manager = layer->GetDisasterManager(0x621CDA33); // borrowed
            CHECK(manager != nullptr);
            void* fireInterface = nullptr;
            CHECK(manager && manager->QueryInterface(0x49B43F57, &fireInterface));
            if (fireInterface) {
                auto* fire = static_cast<cISC4DisasterManagerFire*>(fireInterface);
                CHECK(fire->AsISC4DisasterManager() == manager);
                auto* instance = fire->GetFireInstance(); // borrowed, stable identity
                CHECK(instance != nullptr && instance == fire->GetFireInstance());
                CHECK(instance && instance->GetFireCount() == 0);
                CHECK(fire->GetCause() == 0);
                CHECK(fire->SetCause(2) == 2);
                CHECK(fire->SetCause(3) == 2);
                fire->Release();
            }
            // Slot 6 has two coordinates; empty test city contains no pipe.
            auto* pipe = reinterpret_cast<cFC4ISC4DisasterManager*>(layer->GetDisasterManager(0x4A1332E8));
            CHECK(pipe && pipe->AttemptDisasterStart(5, 6) == nullptr);
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
